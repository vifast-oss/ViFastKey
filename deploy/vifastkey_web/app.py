"""ViFastKey web: landing page + API Python.

Trang tĩnh trong ``static/`` là landing page/playground (WASM). FastAPI bổ sung
API chạy chính engine Python (oracle) và các tiện ích công khai:

* ``POST /api/type``      — chuỗi phím thô -> chữ tiếng Việt
* ``POST /api/charset``   — chuyển bảng mã
* ``GET  /api/meta``      — phiên bản, kiểu gõ, bảng mã, giới hạn
* ``GET  /api/stats``     — thống kê tổng hợp, không lưu nội dung người dùng
* ``POST /api/download``  — đếm lượt bấm nút tải theo nền tảng
* ``POST /api/visit``     — đếm lượt xem ẩn danh
* ``POST /api/feedback``  — góp ý/báo lỗi, chỉ admin xem
* ``GET  /admin``         — bảng điều khiển (cần đăng nhập ở /login)

Chỉ dùng thư viện chuẩn của ViFastKey + FastAPI/uvicorn. Không gọi mạng.
"""

from __future__ import annotations

import csv
import hashlib
import html
import io
import logging
import os
import re
import secrets
import sqlite3
import threading
import time
from collections import defaultdict, deque
from contextlib import asynccontextmanager
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Literal

from adapters import (
    FakeTextField,
    MemoryConfigStore,
    StaticAppContext,
    WordListDictionary,
)
from fastapi import FastAPI, HTTPException, Request, Response
from fastapi.middleware.gzip import GZipMiddleware
from fastapi.middleware.trustedhost import TrustedHostMiddleware
from fastapi.responses import (
    FileResponse,
    HTMLResponse,
    JSONResponse,
    PlainTextResponse,
    RedirectResponse,
)
from fastapi.staticfiles import StaticFiles

from admin_portal import Portal
from pydantic import BaseModel, ConfigDict, Field
from vfk_core.charset import (
    DISPLAY_NAMES as CHARSET_NAMES,
)
from vfk_core.charset import (
    SINGLE_BYTE,
    normalize_charset,
)
from vfk_core.charset import (
    convert as convert_charset,
)
from vfk_engine import (
    Commit,
    Config,
    ImeSession,
    KeyEvent,
    OutputMode,
    PassThrough,
    Ports,
    Preedit,
    Replace,
)
from vfk_ffi import VERSION
from vi_syllable import ToneStyle

ROOT = Path(__file__).resolve().parent
STATIC = Path(os.environ.get("STATIC_DIR", ROOT / "static")).resolve()
DATA_DIR = Path(os.environ.get("DATA_DIR", ROOT / "data"))
DOWNLOADS_DIR = Path(os.environ.get("DOWNLOADS_DIR", ROOT / "downloads")).resolve()
DB_PATH = Path(os.environ.get("STATS_DB", DATA_DIR / "stats.sqlite3"))
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "").strip()
SITE_URL = os.environ.get("SITE_URL", "https://key.vifast.org").rstrip("/")
MAX_TYPE_CHARS = 500
MAX_CHARSET_CHARS = 2000
MAX_FEEDBACK_CHARS = 1000
MAX_CONTACT_CHARS = 120
RATE_WINDOW = 60.0
TYPE_RATE_LIMIT = 60
CHARSET_RATE_LIMIT = 60
FEEDBACK_RATE_LIMIT = 5
VN_TZ = timezone(timedelta(hours=7))

DEFAULT_MACROS = {"ko": "không", "vn": "Việt Nam", "dc": "được"}
METHODS = {
    "telex": "Telex",
    "simple_telex": "Simple Telex",
    "vni": "VNI",
    "viqr": "VIQR",
}
FEEDBACK_CATEGORIES = {
    "gop_y": "Góp ý chung",
    "cai_tien": "Đề xuất cải tiến",
    "bao_loi": "Báo lỗi",
}
FEEDBACK_STATUS = {"new": "Chờ xem", "read": "Đã đọc", "resolved": "Đã xử lý"}


# Tên file trong GitHub Release → nền tảng; server tự map để client không gửi
# chuỗi tùy ý vào thống kê.
DOWNLOAD_PLATFORMS = {
    "vifastkey-windows-hook.zip": "windows",
    "vifastkey-windows-hook-0.1.0.zip": "windows",
    "vifastkey-windows-hook-0.1.2.zip": "windows",
    "vifastkey-windows-hook-0.1.1.zip": "windows",
    "vifastkey-windows-tsf.zip": "windows",
    "vifastkey-cli-windows-x64.zip": "windows",
    "vifastkey-macos-imk-universal.zip": "macos",
    "vifastkey-cli-macos.tar.gz": "macos",
    "vifastkey-cli-linux-x86_64.tar.gz": "linux",
    "vifastkey-fcitx5-linux-x86_64.tar.gz": "linux",
    "vifastkey-ibus-linux.tar.gz": "linux",
    "vifastkey-settings-linux-x86_64.tar.gz": "linux",
    "vifastkey-wayland-linux-x86_64.tar.gz": "linux",
    "vifastkey-settings-windows-x64.zip": "windows",
    "vifastkey-android-debug.apk": "android",
    "vifastkey-android-release.apk": "android",
    "vifastkey-android-release.aab": "android",
    "vifastkey-ios-simulator.zip": "ios",
    "vifastkey-uefi-x86_64.zip": "uefi",
    "vifastkey-wasm.zip": "web",
    "vifastkey-chrome-extension.zip": "extension",
}
DOWNLOAD_LABELS = {
    "windows": "Windows",
    "macos": "macOS",
    "linux": "Linux",
    "android": "Android",
    "ios": "iOS Simulator",
    "uefi": "UEFI x86_64",
    "extension": "Chrome/Edge",
    "web": "WASM",
}
DOWNLOAD_RATE_LIMIT = 60

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("vifastkey-web")

# Từ điển dùng chung, chỉ đọc sau khi khởi tạo nên an toàn với nhiều request.
DICTIONARY = WordListDictionary()
_LOCK = threading.Lock()
_RATE_LOCK = threading.Lock()
_RATE_BUCKETS: dict[str, deque[float]] = defaultdict(deque)
_STATS_LOCK = threading.Lock()
_SALT: str | None = None
_STARTED = time.time()


# ---------------------------------------------------------------- cơ sở dữ liệu
def _connect() -> sqlite3.Connection:
    connection = sqlite3.connect(str(DB_PATH), timeout=10)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA busy_timeout=3000")
    return connection


def _init_db() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    with _STATS_LOCK:
        connection = _connect()
        try:
            connection.executescript(
                """
                PRAGMA journal_mode=WAL;
                CREATE TABLE IF NOT EXISTS meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS visitors (
                    visitor TEXT PRIMARY KEY,
                    first_seen REAL NOT NULL,
                    last_seen REAL NOT NULL,
                    page_views INTEGER NOT NULL DEFAULT 0,
                    demo_count INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE IF NOT EXISTS demos (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts REAL NOT NULL,
                    visitor TEXT NOT NULL,
                    chars INTEGER NOT NULL,
                    method TEXT NOT NULL,
                    tone TEXT NOT NULL,
                    elapsed_ms REAL NOT NULL
                );
                CREATE INDEX IF NOT EXISTS demos_ts ON demos(ts);
                CREATE TABLE IF NOT EXISTS feedback (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts REAL NOT NULL,
                    visitor TEXT NOT NULL,
                    category TEXT NOT NULL,
                    message TEXT NOT NULL,
                    contact TEXT NOT NULL DEFAULT '',
                    status TEXT NOT NULL DEFAULT 'new',
                    updated REAL NOT NULL
                );
                CREATE INDEX IF NOT EXISTS feedback_status ON feedback(status);
                CREATE TABLE IF NOT EXISTS download_clicks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts REAL NOT NULL,
                    platform TEXT NOT NULL,
                    asset TEXT NOT NULL,
                    source TEXT NOT NULL DEFAULT 'web'
                );
                CREATE INDEX IF NOT EXISTS download_ts ON download_clicks(ts);
                """
            )
            connection.commit()
        finally:
            connection.close()


def _get_salt() -> str:
    global _SALT
    if _SALT:
        return _SALT
    configured = os.environ.get("STATS_SALT", "").strip()
    if configured:
        _SALT = configured
        return _SALT
    with _STATS_LOCK:
        connection = _connect()
        try:
            row = connection.execute(
                "SELECT value FROM meta WHERE key='salt'"
            ).fetchone()
            if row:
                _SALT = str(row["value"])
            else:
                _SALT = secrets.token_hex(16)
                connection.execute(
                    "INSERT INTO meta(key, value) VALUES('salt', ?)", (_SALT,)
                )
                connection.commit()
        finally:
            connection.close()
    return _SALT


def _client_ip(request: Request) -> str:
    forwarded = request.headers.get("x-forwarded-for", "")
    return forwarded.split(",")[0].strip() or (
        request.client.host if request.client else "unknown"
    )


def _ip_hash(request: Request) -> str:
    return hashlib.sha256(
        f"{_get_salt()}:ip:{_client_ip(request)}".encode()
    ).hexdigest()[:24]


def _visitor_hash(request: Request, visitor_id: str | None = None) -> str:
    if visitor_id and re.fullmatch(r"[A-Za-z0-9_-]{8,80}", visitor_id):
        material = f"id:{visitor_id}"
    else:
        material = f"ip:{_client_ip(request)}"
    return hashlib.sha256(f"{_get_salt()}:{material}".encode()).hexdigest()[:24]


def _record_page(visitor: str) -> None:
    now = time.time()
    with _STATS_LOCK:
        connection = _connect()
        try:
            connection.execute(
                "INSERT INTO visitors(visitor, first_seen, last_seen, page_views) VALUES(?,?,?,1) "
                "ON CONFLICT(visitor) DO UPDATE SET last_seen=excluded.last_seen, "
                "page_views=visitors.page_views+1",
                (visitor, now, now),
            )
            connection.commit()
        finally:
            connection.close()


def _record_demo(
    visitor: str, chars: int, method: str, tone: str, elapsed_ms: float
) -> None:
    now = time.time()
    with _STATS_LOCK:
        connection = _connect()
        try:
            connection.execute(
                "INSERT INTO demos(ts, visitor, chars, method, tone, elapsed_ms) VALUES(?,?,?,?,?,?)",
                (now, visitor, chars, method, tone, elapsed_ms),
            )
            connection.execute(
                "INSERT INTO visitors(visitor, first_seen, last_seen, demo_count) VALUES(?,?,?,1) "
                "ON CONFLICT(visitor) DO UPDATE SET last_seen=excluded.last_seen, "
                "demo_count=visitors.demo_count+1",
                (visitor, now, now),
            )
            connection.commit()
        finally:
            connection.close()


def _record_download(platform: str, asset: str, source: str) -> None:
    """Ghi một lượt bấm tải; chỉ platform/asset/source, không lưu IP/text."""
    now = time.time()
    with _STATS_LOCK:
        connection = _connect()
        try:
            connection.execute(
                "INSERT INTO download_clicks(ts, platform, asset, source) VALUES(?,?,?,?)",
                (now, platform, asset, source),
            )
            connection.commit()
        finally:
            connection.close()


def _record_feedback(visitor: str, category: str, message: str, contact: str) -> int:
    now = time.time()
    with _STATS_LOCK:
        connection = _connect()
        try:
            cursor = connection.execute(
                "INSERT INTO feedback(ts, visitor, category, message, contact, status, updated) "
                "VALUES(?,?,?,?,?,'new',?)",
                (now, visitor, category, message, contact, now),
            )
            connection.commit()
            return int(cursor.lastrowid)
        finally:
            connection.close()


def _feedback_rows(limit: int = 100) -> list[dict]:
    with _STATS_LOCK:
        connection = _connect()
        try:
            rows = connection.execute(
                "SELECT id, ts, visitor, category, message, contact, status "
                "FROM feedback ORDER BY id DESC LIMIT ?",
                (limit,),
            ).fetchall()
        finally:
            connection.close()
    return [
        {
            **dict(row),
            "time": datetime.fromtimestamp(float(row["ts"]), VN_TZ).strftime(
                "%d/%m %H:%M"
            ),
        }
        for row in rows
    ]


def _stats_payload() -> dict:
    now = time.time()
    local_now = datetime.fromtimestamp(now, VN_TZ)
    day_start = local_now.replace(hour=0, minute=0, second=0, microsecond=0).timestamp()
    with _STATS_LOCK:
        connection = _connect()
        try:
            users = connection.execute("SELECT COUNT(*) AS n FROM visitors").fetchone()[
                "n"
            ]
            pages = connection.execute(
                "SELECT COALESCE(SUM(page_views),0) AS n FROM visitors"
            ).fetchone()["n"]
            demos = connection.execute("SELECT COUNT(*) AS n FROM demos").fetchone()[
                "n"
            ]
            demos_today = connection.execute(
                "SELECT COUNT(*) AS n FROM demos WHERE ts >= ?", (day_start,)
            ).fetchone()["n"]
            users_today = connection.execute(
                "SELECT COUNT(*) AS n FROM visitors WHERE last_seen >= ?", (day_start,)
            ).fetchone()["n"]
            totals = connection.execute(
                "SELECT COALESCE(SUM(chars),0) AS chars, COALESCE(AVG(chars),0) AS avg_chars, "
                "COALESCE(AVG(elapsed_ms),0) AS avg_ms FROM demos"
            ).fetchone()
            feedback_total = connection.execute(
                "SELECT COUNT(*) AS n FROM feedback"
            ).fetchone()["n"]
            feedback_new = connection.execute(
                "SELECT COUNT(*) AS n FROM feedback WHERE status='new'"
            ).fetchone()["n"]
            downloads_total = connection.execute(
                "SELECT COUNT(*) AS n FROM download_clicks"
            ).fetchone()["n"]
            downloads_today = connection.execute(
                "SELECT COUNT(*) AS n FROM download_clicks WHERE ts >= ?", (day_start,)
            ).fetchone()["n"]
            downloads_by_platform = {
                str(row["platform"]): int(row["n"])
                for row in connection.execute(
                    "SELECT platform, COUNT(*) AS n FROM download_clicks GROUP BY platform"
                ).fetchall()
            }
            recent = [
                dict(row)
                for row in connection.execute(
                    "SELECT ts, chars, method, tone, elapsed_ms FROM demos ORDER BY id DESC LIMIT 12"
                ).fetchall()
            ]
        finally:
            connection.close()
    return {
        "generated_at": datetime.fromtimestamp(now, VN_TZ).strftime(
            "%Y-%m-%d %H:%M:%S"
        ),
        "timezone": "UTC+07:00",
        "page_views": int(pages),
        "visitors": int(users),
        "visitors_today": int(users_today),
        "demos": int(demos),
        "demos_today": int(demos_today),
        "chars_total": int(totals["chars"]),
        "avg_chars": round(float(totals["avg_chars"]), 1),
        "avg_ms": round(float(totals["avg_ms"]), 3),
        "feedback_total": int(feedback_total),
        "feedback_new": int(feedback_new),
        "downloads_total": int(downloads_total),
        "downloads_today": int(downloads_today),
        "downloads_by_platform": downloads_by_platform,
        "recent": [
            {
                **row,
                "time": datetime.fromtimestamp(float(row["ts"]), VN_TZ).strftime(
                    "%d/%m %H:%M:%S"
                ),
                "chars": int(row["chars"]),
                "elapsed_ms": round(float(row["elapsed_ms"]), 3),
            }
            for row in recent
        ],
    }


# ---------------------------------------------------------------- rate limit
def _rate_ok(bucket: str, limit: int) -> bool:
    now = time.monotonic()
    with _RATE_LOCK:
        if len(_RATE_BUCKETS) > 4096:
            stale = []
            for key, queue in _RATE_BUCKETS.items():
                while queue and now - queue[0] > RATE_WINDOW:
                    queue.popleft()
                if not queue:
                    stale.append(key)
            for key in stale:
                _RATE_BUCKETS.pop(key, None)
        queue = _RATE_BUCKETS[bucket]
        while queue and now - queue[0] > RATE_WINDOW:
            queue.popleft()
        if len(queue) >= limit:
            return False
        queue.append(now)
        return True


# ---------------------------------------------------------------- engine
def _clean_macros(raw: dict[str, str] | None) -> dict[str, str]:
    if not raw:
        return {}
    if len(raw) > 50:
        raise HTTPException(status_code=422, detail="Tối đa 50 macro")
    clean: dict[str, str] = {}
    for key, value in raw.items():
        key, value = str(key).strip().lower(), str(value).strip()
        if not key or not value:
            continue
        if len(key) > 40 or len(value) > 120:
            raise HTTPException(status_code=422, detail="Macro sai độ dài")
        if any(ord(ch) < 32 for ch in key + value):
            raise HTTPException(status_code=422, detail="Macro chứa ký tự điều khiển")
        clean[key] = value
    return clean


def _summary(action) -> str:
    if isinstance(action, PassThrough):
        return "pass"
    if isinstance(action, Preedit):
        return f"preedit {len(action.text)}"
    if isinstance(action, Replace):
        return f"replace −{action.delete_before} +{len(action.insert)}"
    if isinstance(action, Commit):
        return f"commit {len(action.text)}"
    return "action"


def _type_text(
    text: str,
    method: str,
    tone: str,
    mode: str,
    spell_check: bool,
    allow_coda_k: bool,
    macro_digits: bool,
    unicode_form: str,
    macros: dict[str, str],
    trace: bool = False,
) -> tuple[str, list[dict], float]:
    """Chạy engine Python trên một chuỗi phím thô.

    ``<`` = Backspace, ``~`` = Escape, ``|`` = Enter (giống CLI của dự án).
    Trả về (text, trace, elapsed_ms).
    """
    started = time.perf_counter()
    config = Config(
        input_method=method,
        tone_style=ToneStyle(tone),
        spell_check=spell_check,
        allow_coda_k=allow_coda_k,
        macro_digits=macro_digits,
        unicode_form=unicode_form,
        macros={**DEFAULT_MACROS, **macros},
    )
    field = FakeTextField()
    session = ImeSession(
        Ports(MemoryConfigStore(config), StaticAppContext("web"), field, DICTIONARY),
        OutputMode(mode),
    )
    steps: list[dict] = []
    special = {"<": "Backspace", "~": "Escape", "|": "Enter"}
    for char in text:
        key = special.get(char, char)
        event = KeyEvent(key)
        action = session.process_key(event)
        field.apply(action, event)
        if trace:
            steps.append({"key": key, "action": _summary(action), "text": field.text})
    final_action = session.process_key(KeyEvent("Enter"))
    field.apply(final_action)
    if trace:
        steps.append(
            {"key": "Enter", "action": _summary(final_action), "text": field.text}
        )
    return field.text, steps, (time.perf_counter() - started) * 1000


# ---------------------------------------------------------------- admin
def _admin_authorized(request: Request) -> bool:
    """Phiên đăng nhập bằng form (cookie ký HMAC); không dùng HTTP Basic."""
    return portal.auth_ok(request)


def _admin_denied() -> Response:
    return RedirectResponse("/login?next=/admin", status_code=303)


def _csv_cell(value: object) -> str:
    text = str(value)
    return "'" + text if text[:1] in ("=", "+", "-", "@") else text


def _admin_html(stats: dict) -> str:
    feedback = _feedback_rows()
    rows = []
    for item in feedback:
        message = html.escape(str(item["message"])).replace("\n", "<br>")
        contact = html.escape(str(item["contact"])) or "—"
        category = html.escape(
            FEEDBACK_CATEGORIES.get(str(item["category"]), str(item["category"]))
        )
        status = FEEDBACK_STATUS.get(str(item["status"]), str(item["status"]))
        status_key = str(item["status"])
        actions = []
        if status_key != "read":
            actions.append(
                f"<form method='post' action='/admin/feedback/{item['id']}/read' class='vf-form-inline'>"
                f"<button class='vf-tiny'>Đã đọc</button></form>"
            )
        if status_key != "resolved":
            actions.append(
                f"<form method='post' action='/admin/feedback/{item['id']}/resolved' class='vf-form-inline'>"
                f"<button class='vf-tiny vf-tiny-ok'>Đã xử lý</button></form>"
            )
        if not actions:
            actions.append("<span class='vf-muted'>Đã xử lý</span>")
        pill = {"new": "vf-pill vf-pill-new"}.get(status_key, "vf-pill vf-pill-ok")
        rows.append(
            f"<tr><td class='vf-mono'>{item['time']}</td>"
            f"<td><span class='vf-pill'>{category}</span></td>"
            f"<td class='vf-msg'>{message}</td><td>{contact}</td>"
            f"<td class='vf-mono'>{html.escape(str(item['visitor']))[:10]}</td>"
            f"<td><span class='{pill}'>{status}</span></td><td>{' '.join(actions)}</td></tr>"
        )
    feedback_rows = "".join(rows) or '<tr><td colspan="7" class="vf-muted">Chưa có góp ý nào.</td></tr>'
    recent_rows = (
        "".join(
            f"<tr><td class='vf-mono'>{item['time']}</td><td class='vf-num'>{item['chars']}</td>"
            f"<td>{html.escape(str(item['method']))}</td>"
            f"<td>{html.escape(str(item['tone']))}</td><td class='vf-num'>{item['elapsed_ms']:.3f}s</td></tr>"
            for item in stats["recent"]
        )
        or '<tr><td colspan="5" class="vf-muted">Chưa có lượt chạy API nào.</td></tr>'
    )
    download_summary = " · ".join(
        f"{label} {int(stats['downloads_by_platform'].get(key, 0))}"
        for key, label in DOWNLOAD_LABELS.items()
    )
    auth_note = (
        "Đã bật đăng nhập bằng mật khẩu."
        if ADMIN_PASSWORD
        else "Đang mở công khai, chưa đặt ADMIN_PASSWORD."
    )
    cards = "".join(
        f'<div class="vf-card"><span class="vf-card-label">{label}</span>'
        f'<span class="vf-card-value">{value}</span></div>'
        for label, value in (
            ("Lượt xem trang", stats["page_views"]),
            ("Khách ẩn danh", stats["visitors"]),
            ("Lượt chạy API", stats["demos"]),
            ("Lượt chạy hôm nay", stats["demos_today"]),
            ("Khách hôm nay", stats["visitors_today"]),
            ("Ký tự đã chuyển", stats["chars_total"]),
            ("Ký tự / lượt", stats["avg_chars"]),
            ("Xử lý trung bình", f'{stats["avg_ms"]:.2f} ms'),
            ("Góp ý / báo lỗi", stats["feedback_total"]),
            ("Góp ý mới", stats["feedback_new"]),
            ("Lượt bấm tải", stats["downloads_total"]),
            ("Tải hôm nay", stats["downloads_today"]),
        )
    )

    content = f"""
{portal.panel_html()}
<h2 class="vf-h2">Tổng quan</h2>
<div class="vf-cards">{cards}</div>

<h2 class="vf-h2">Lượt bấm tải theo nền tảng</h2>
<p class="vf-muted">{download_summary}</p>

<h2 class="vf-h2">Góp ý &amp; báo lỗi</h2>
<p class="vf-muted">{auth_note} · <a class="vf-link" href="/admin/feedback.csv">Tải CSV</a></p>
<div class="vf-scroll">
  <table class="vf-table">
    <thead><tr><th>Thời điểm</th><th>Loại</th><th>Nội dung</th><th>Liên hệ</th><th>Ẩn danh</th><th>Trạng thái</th><th>Cập nhật</th></tr></thead>
    <tbody>{feedback_rows}</tbody>
  </table>
</div>

<h2 class="vf-h2">Lượt chạy API gần đây</h2>
<div class="vf-scroll">
  <table class="vf-table">
    <thead><tr><th>Thời điểm</th><th>Ký tự</th><th>Kiểu gõ</th><th>Dấu</th><th>Xử lý</th></tr></thead>
    <tbody>{recent_rows}</tbody>
  </table>
</div>

<p class="vf-note">Chỉ lưu hash visitor/IP, số ký tự và thời gian xử lý; không lưu nội dung bạn gõ.
Nội dung góp ý chỉ người đã đăng nhập mới xem được. {auth_note}
Lượt bấm tải chỉ đếm click trên trang chủ, không xác nhận file đã tải xong.</p>
"""
    return portal.shell(content)


# ---------------------------------------------------------------- app
@asynccontextmanager
async def lifespan(_: FastAPI):
    _init_db()
    _get_salt()
    logger.info("ViFastKey web ready, version=%s", VERSION)
    yield


app = FastAPI(
    title="ViFastKey web",
    version=VERSION,
    docs_url=None,
    redoc_url=None,
    openapi_url=None,
    lifespan=lifespan,
)
app.add_middleware(GZipMiddleware, minimum_size=700)
hosts = [
    h.strip()
    for h in os.environ.get(
        "ALLOWED_HOSTS",
        "key.vifast.org,localhost,127.0.0.1,testserver",
    ).split(",")
    if h.strip()
]
app.add_middleware(TrustedHostMiddleware, allowed_hosts=hosts or ["*"])

# Đăng nhập quản trị bằng form + thống kê truy cập (dùng chung admin_portal.py).
portal = Portal(site="key.vifast.org", title="ViFastKey", db_path=DB_PATH,
                accent="#C6281E", log=logger,
                favicon="/brand/favicon.ico", admin_key="key")
portal.install(app)


@app.middleware("http")
async def security_headers(request: Request, call_next):
    response = await call_next(request)
    response.headers.setdefault("X-Content-Type-Options", "nosniff")
    response.headers.setdefault("X-Frame-Options", "SAMEORIGIN")
    response.headers.setdefault("Referrer-Policy", "strict-origin-when-cross-origin")
    response.headers.setdefault("Strict-Transport-Security", "max-age=31536000")
    response.headers.setdefault(
        "Permissions-Policy", "camera=(), microphone=(), geolocation=()"
    )
    response.headers.setdefault(
        "Content-Security-Policy",
        "default-src 'self'; img-src 'self' data:; font-src 'self'; "
        "style-src 'self' 'unsafe-inline'; script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval'; "
        "connect-src 'self'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'",
    )
    path = request.url.path
    suffix = Path(path).suffix.lower()
    if suffix in {".css", ".js"}:
        response.headers.setdefault("Cache-Control", "no-cache")
    elif path.startswith(("/brand/", "/fonts/", "/pkg/")) or suffix in {
        ".wasm",
        ".png",
        ".ico",
        ".ttf",
        ".webmanifest",
        ".svg",
    }:
        response.headers.setdefault("Cache-Control", "public, max-age=604800")
    elif path.startswith(("/api/", "/admin")) or path == "/healthz":
        response.headers.setdefault("Cache-Control", "no-store")
    elif path == "/" or path.endswith(".html"):
        response.headers.setdefault("Cache-Control", "no-cache")
    portal.track(request, response.status_code)
    return response


# ---------------------------------------------------------------- models
class TypeRequest(BaseModel):
    text: str = Field(..., max_length=MAX_TYPE_CHARS)
    method: Literal["telex", "simple_telex", "vni", "viqr"] = "telex"
    tone: Literal["old", "new"] = "new"
    mode: Literal["replace", "preedit"] = "replace"
    spell_check: bool = True
    allow_coda_k: bool = False
    macro_digits: bool = False
    unicode_form: Literal["NFC", "NFD"] = "NFC"
    macros: dict[str, str] | None = None
    visitor_id: str | None = None
    trace: bool = False


class CharsetRequest(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    text: str = Field(..., max_length=MAX_CHARSET_CHARS)
    from_charset: str = Field("unicode", alias="from")
    to_charset: str = Field("tcvn3", alias="to")


class VisitRequest(BaseModel):
    visitor_id: str | None = None


class DownloadRequest(BaseModel):
    asset: str
    visitor_id: str | None = None
    source: Literal["web", "api"] = "web"


class FeedbackRequest(BaseModel):
    category: str = "gop_y"
    message: str
    contact: str | None = None
    visitor_id: str | None = None
    website: str | None = None


# ---------------------------------------------------------------- routes
@app.get("/", response_class=HTMLResponse)
def index() -> Response:
    return FileResponse(STATIC / "index.html", headers={"Cache-Control": "no-cache"})


@app.get("/healthz")
def healthz() -> dict:
    db_ok = True
    try:
        with _STATS_LOCK:
            connection = _connect()
            try:
                connection.execute("SELECT 1").fetchone()
            finally:
                connection.close()
    except Exception:
        logger.exception("health db check failed")
        db_ok = False
    return {
        "ok": db_ok,
        "version": VERSION,
        "engine": "python",
        "db": db_ok,
        "uptime_seconds": round(time.time() - _STARTED, 1),
    }


@app.get("/api/meta")
def api_meta() -> dict:
    charsets = [
        {"key": key, "name": name, "single_byte": key in SINGLE_BYTE}
        for key, name in CHARSET_NAMES.items()
    ]
    return {
        "version": VERSION,
        "engine": "python-oracle",
        "max_type_chars": MAX_TYPE_CHARS,
        "max_charset_chars": MAX_CHARSET_CHARS,
        "methods": METHODS,
        "tones": {"old": "Dấu cũ", "new": "Dấu mới"},
        "charsets": charsets,
        "default_macros": DEFAULT_MACROS,
        "special_keys": {"<": "Backspace", "~": "Escape", "|": "Enter"},
        "site": SITE_URL,
    }


@app.post("/api/type")
def api_type(payload: TypeRequest, request: Request) -> dict:
    text = payload.text.replace("\x00", " ")
    if not text.strip():
        raise HTTPException(status_code=422, detail="Nhập chuỗi phím cần chuyển")
    visitor = _visitor_hash(request, payload.visitor_id)
    if not _rate_ok(f"type:{_ip_hash(request)}", TYPE_RATE_LIMIT):
        raise HTTPException(
            status_code=429,
            detail="Chạy quá nhiều; thử lại sau 1 phút",
            headers={"Retry-After": "60"},
        )
    macros = _clean_macros(payload.macros)
    try:
        if not _LOCK.acquire(timeout=5):
            raise HTTPException(status_code=503, detail="Máy đang bận; thử lại sau")
        try:
            output, steps, elapsed_ms = _type_text(
                text,
                payload.method,
                payload.tone,
                payload.mode,
                payload.spell_check,
                payload.allow_coda_k,
                payload.macro_digits,
                payload.unicode_form,
                macros,
                payload.trace,
            )
        finally:
            _LOCK.release()
    except HTTPException:
        raise
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    except Exception as exc:
        logger.exception("type failed")
        raise HTTPException(
            status_code=500, detail="Lỗi chuyển chữ; thử chuỗi khác"
        ) from exc
    try:
        _record_demo(visitor, len(text), payload.method, payload.tone, elapsed_ms)
    except Exception:
        logger.exception("record demo failed")
    result = {
        "output": output,
        "input": text,
        "method": payload.method,
        "tone": payload.tone,
        "mode": payload.mode,
        "engine": "python",
        "version": VERSION,
        "elapsed_ms": round(elapsed_ms, 3),
    }
    if payload.trace:
        result["trace"] = steps
    return result


@app.post("/api/charset")
def api_charset(payload: CharsetRequest, request: Request) -> dict:
    if not payload.text:
        raise HTTPException(status_code=422, detail="Nhập chuỗi cần chuyển bảng mã")
    if not _rate_ok(f"charset:{_ip_hash(request)}", CHARSET_RATE_LIMIT):
        raise HTTPException(
            status_code=429,
            detail="Chạy quá nhiều; thử lại sau 1 phút",
            headers={"Retry-After": "60"},
        )
    try:
        source = normalize_charset(payload.from_charset)
        target = normalize_charset(payload.to_charset)
        output = convert_charset(payload.text, source, target)
        hex_text = None
        if target in SINGLE_BYTE:
            try:
                hex_text = " ".join(f"{byte:02x}" for byte in output.encode("latin-1"))
            except UnicodeEncodeError:
                hex_text = None
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    return {
        "output": output,
        "hex": hex_text,
        "from": source,
        "to": target,
        "engine": "python",
        "version": VERSION,
    }


@app.get("/api/stats")
def api_stats() -> dict:
    stats = _stats_payload()
    return {
        "generated_at": stats["generated_at"],
        "page_views": stats["page_views"],
        "visitors": stats["visitors"],
        "demos": stats["demos"],
        "demos_today": stats["demos_today"],
        "chars_total": stats["chars_total"],
        "feedback_total": stats["feedback_total"],
        "downloads_total": stats["downloads_total"],
        "downloads_today": stats["downloads_today"],
        "downloads_by_platform": [
            {
                "platform": key,
                "label": label,
                "count": int(stats["downloads_by_platform"].get(key, 0)),
            }
            for key, label in DOWNLOAD_LABELS.items()
        ],
    }


@app.post("/api/download")
def api_download(payload: DownloadRequest, request: Request) -> dict:
    """Đếm lượt bấm nút tải; không xác nhận file đã tải xong."""
    asset = Path(str(payload.asset)).name
    platform = DOWNLOAD_PLATFORMS.get(asset)
    if not platform:
        raise HTTPException(status_code=422, detail="File tải không hợp lệ")
    visitor = _visitor_hash(request, payload.visitor_id)
    if not _rate_ok(f"download:{visitor}", DOWNLOAD_RATE_LIMIT):
        raise HTTPException(status_code=429, detail="Ghi nhận quá nhanh; thử lại sau")
    try:
        _record_download(platform, asset, payload.source)
    except Exception:
        logger.exception("download stats failed")
        raise HTTPException(status_code=500, detail="Không ghi được thống kê") from None
    return {"ok": True, "platform": platform, "asset": asset}


@app.post("/api/visit")
def api_visit(payload: VisitRequest, request: Request) -> dict:
    try:
        visitor = _visitor_hash(request, payload.visitor_id)
        if not _rate_ok(f"visit:{_ip_hash(request)}", 30):
            return {"ok": True}
        _record_page(visitor)
    except Exception:
        logger.exception("visit failed")
        return {"ok": False}
    return {"ok": True}


@app.post("/api/feedback")
def api_feedback(payload: FeedbackRequest, request: Request) -> dict:
    if (payload.website or "").strip():
        return {"ok": True, "queued": 0}
    category = payload.category if payload.category in FEEDBACK_CATEGORIES else "gop_y"
    message = re.sub(
        r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]", "", payload.message or ""
    ).strip()
    if len(message) < 5:
        raise HTTPException(status_code=422, detail="Nội dung cần ít nhất 5 ký tự")
    if len(message) > MAX_FEEDBACK_CHARS:
        raise HTTPException(
            status_code=422, detail=f"Nội dung tối đa {MAX_FEEDBACK_CHARS} ký tự"
        )
    contact = re.sub(r"[\x00-\x1f\x7f]", "", payload.contact or "").strip()
    if len(contact) > MAX_CONTACT_CHARS:
        raise HTTPException(
            status_code=422, detail=f"Liên hệ tối đa {MAX_CONTACT_CHARS} ký tự"
        )
    visitor = _visitor_hash(request, payload.visitor_id)
    if not _rate_ok(f"feedback:{_ip_hash(request)}", FEEDBACK_RATE_LIMIT):
        raise HTTPException(
            status_code=429,
            detail="Gửi quá nhiều; thử lại sau 1 phút",
            headers={"Retry-After": "60"},
        )
    feedback_id = _record_feedback(visitor, category, message, contact)
    logger.info("feedback stored id=%d category=%s", feedback_id, category)
    return {"ok": True, "queued": 1}


@app.get("/admin", response_class=HTMLResponse)
def admin(request: Request) -> Response:
    if not _admin_authorized(request):
        return _admin_denied()
    return HTMLResponse(
        _admin_html(_stats_payload()), headers={"Cache-Control": "no-store"}
    )


@app.get("/admin.json")
def admin_json(request: Request) -> Response:
    if not _admin_authorized(request):
        return JSONResponse({"error": "unauthorized"}, status_code=401)
    return JSONResponse(_stats_payload(), headers={"Cache-Control": "no-store"})


@app.get("/admin/feedback.csv")
def admin_feedback_csv(request: Request) -> Response:
    if not _admin_authorized(request):
        return _admin_denied()
    buffer = io.StringIO()
    buffer.write("\ufeff")
    writer = csv.writer(buffer)
    writer.writerow(
        ["id", "thoi_gian", "loai", "noi_dung", "lien_he", "an_danh", "trang_thai"]
    )
    for item in _feedback_rows(limit=10000):
        writer.writerow(
            [
                item["id"],
                item["time"],
                FEEDBACK_CATEGORIES.get(item["category"], item["category"]),
                _csv_cell(item["message"]),
                _csv_cell(item["contact"]),
                item["visitor"],
                FEEDBACK_STATUS.get(item["status"], item["status"]),
            ]
        )
    return Response(
        content=buffer.getvalue(),
        media_type="text/csv; charset=utf-8",
        headers={
            "Content-Disposition": 'attachment; filename="vifastkey-feedback.csv"',
            "Cache-Control": "no-store",
        },
    )


@app.post("/admin/feedback/{feedback_id}/{status}")
def admin_feedback_status(feedback_id: int, status: str, request: Request) -> Response:
    if not _admin_authorized(request):
        return _admin_denied()
    if status not in FEEDBACK_STATUS:
        raise HTTPException(status_code=400, detail="Trạng thái không hợp lệ")
    with _STATS_LOCK:
        connection = _connect()
        try:
            connection.execute(
                "UPDATE feedback SET status=?, updated=? WHERE id=?",
                (status, time.time(), feedback_id),
            )
            connection.commit()
        finally:
            connection.close()
    return Response(status_code=303, headers={"Location": "/admin"})


@app.get("/downloads/{version}/{filename}")
def download_file(version: str, filename: str) -> Response:
    """Mirror tải trực tiếp khi GitHub Release tạm không dùng được."""
    if not re.fullmatch(r"0\.\d+\.\d+", version) or Path(filename).name != filename:
        raise HTTPException(status_code=404, detail="Không tìm thấy file")
    if filename not in DOWNLOAD_PLATFORMS and filename != "sha256sums.txt":
        raise HTTPException(status_code=404, detail="Không tìm thấy file")
    path = DOWNLOADS_DIR / version / filename
    if not path.is_file():
        raise HTTPException(status_code=404, detail="Chưa có file mirror")
    return FileResponse(
        path,
        filename=filename,
        headers={"Cache-Control": "public, max-age=3600"},
    )


@app.get("/favicon.ico", include_in_schema=False)
def favicon() -> Response:
    return FileResponse(
        STATIC / "brand" / "favicon.ico",
        media_type="image/x-icon",
        headers={"Cache-Control": "public, max-age=604800"},
    )


@app.get("/robots.txt", include_in_schema=False)
def robots() -> PlainTextResponse:
    return PlainTextResponse(
        f"User-agent: *\nAllow: /\nSitemap: {SITE_URL}/sitemap.xml\n",
        headers={"Cache-Control": "public, max-age=86400"},
    )


@app.get("/sitemap.xml", include_in_schema=False)
def sitemap() -> Response:
    body = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        f"  <url><loc>{html.escape(SITE_URL)}/</loc><changefreq>weekly</changefreq><priority>1.0</priority></url>\n"
        "</urlset>\n"
    )
    return Response(
        content=body,
        media_type="application/xml",
        headers={"Cache-Control": "public, max-age=86400"},
    )


@app.exception_handler(404)
async def not_found(request: Request, _exc):
    if request.url.path.startswith("/api/") or request.url.path in {
        "/healthz",
        "/admin.json",
    }:
        return JSONResponse({"detail": "Not found"}, status_code=404)
    if "text/html" not in request.headers.get("accept", ""):
        return PlainTextResponse("Not found", status_code=404)
    return HTMLResponse(
        """<!doctype html><html lang="vi"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>Không tìm thấy · ViFastKey</title>
<style>body{font-family:system-ui,Arial,sans-serif;background:#FBF6EE;color:#1F2430;display:grid;
place-items:center;min-height:100vh;margin:0}main{text-align:center;padding:32px}h1{font-size:42px;margin:0 0 8px}
a{color:#D42A22;font-weight:700}</style></head><body><main><h1>404</h1>
<p>Trang này không tồn tại.</p><p><a href="/">Về ViFastKey</a></p></main></body></html>""",
        status_code=404,
    )


# Mount cuối cùng: mọi route API/admin ở trên được ưu tiên.

# ── Bản phát hành công khai (proxy GitHub Releases, cache 10 phút) ──────────
RELEASES_REPO = "vifast-oss/ViFastKey"
_RELEASES_CACHE: dict = {"at": 0.0, "data": []}


@app.get("/api/releases")
def api_releases():
    import json as _json
    import time as _time
    from urllib.request import Request as _Req, urlopen as _urlopen

    now = _time.time()
    at = _RELEASES_CACHE.get("at") or 0.0
    if items := _RELEASES_CACHE.get("data"):
        if (now - float(at)) < 600:
            return JSONResponse({"repo": RELEASES_REPO, "releases": items, "cached": True})
    items: list = []
    try:
        req = _Req(
            f"https://api.github.com/repos/{RELEASES_REPO}/releases?per_page=5",
            headers={"Accept": "application/vnd.github+json", "User-Agent": "vifast-web"},
        )
        with _urlopen(req, timeout=6) as resp:
            raw = _json.loads(resp.read().decode("utf-8"))
        for r in raw[:5]:
            body = [
                ln.strip()
                for ln in (r.get("body") or "").splitlines()
                if ln.strip() and not ln.lstrip().startswith("#")
            ]
            items.append({
                "tag": r.get("tag_name"),
                "name": r.get("name") or r.get("tag_name"),
                "published_at": r.get("published_at"),
                "url": r.get("html_url"),
                "prerelease": bool(r.get("prerelease")),
                "summary": (body[0][:200] if body else ""),
            })
        _RELEASES_CACHE.update({"at": now, "data": items})
    except Exception as exc:  # mạng lỗi / rate-limit: trả danh sách rỗng, không 5xx
        logging.getLogger(__name__).info("releases fetch failed: %s", exc)
    return JSONResponse({"repo": RELEASES_REPO, "releases": items, "cached": False})


app.mount("/", StaticFiles(directory=STATIC, html=True), name="static")
