"""Port ra (driven ports) của hexagon. Mỗi nền tảng cài đặt các Protocol này.

Rust: mỗi Protocol là 1 trait; `Ports` gom các trait object (Box<dyn ...>) hoặc generic.
"""
from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field, fields
from pathlib import Path
from typing import Protocol

from vi_syllable import ToneStyle

CONFIG_SCHEMA_VERSION = 1


@dataclass
class AppProfile:
    """Gợi ý cho ADAPTER theo từng app (engine không dùng trực tiếp).
    Rút ra từ lỗi thực tế: vnkey (terminal, Electron), fcitx5-lotus (ack Chrome, độ trễ theo độ dài)."""
    output_mode: str | None = None       # "preedit" | "replace" | None (mặc định adapter)
    backspace_delay_ms: int = 0          # trễ cơ sở giữa các backspace giả
    delay_per_char_ms: int = 0           # cộng thêm theo số ký tự bị xoá
    wait_ack: bool = False               # chờ app xử lý xong backspace rồi mới chèn
    edit_previous_word: bool | None = None
    restore_after_space: bool | None = None  # ghi đè config; terminal Linux = false
    vietnamese: bool | None = None       # ép bật/tắt (game, vim normal mode...)
    # Ghi đè theo app (P1 từ nghiên cứu VnKey/XKey/Bamboo):
    input_method: str | None = None      # telex | simple_telex | vni
    tone_style: ToneStyle | None = None  # None = dùng Config
    spell_check: bool | None = None
    charset: str | None = None           # gợi ý bảng mã cho adapter


# Dữ liệu profile/alias nằm trong một file chung để Python và Rust cùng đọc.
# Rust include file này lúc compile; Python đọc lúc import.
_APP_PROFILES_PATH = Path(__file__).with_name("app_profiles.json")
_APP_PROFILE_DATA = json.loads(_APP_PROFILES_PATH.read_text(encoding="utf-8"))

APP_ALIASES: dict[str, str] = dict(_APP_PROFILE_DATA["aliases"])


def _make_profile(value: dict) -> AppProfile:
    value = dict(value)
    if isinstance(value.get("tone_style"), str):
        value["tone_style"] = ToneStyle(value["tone_style"])
    return AppProfile(**value)


DEFAULT_APP_PROFILES: dict[str, AppProfile] = {
    name: _make_profile(value)
    for name, value in _APP_PROFILE_DATA["profiles"].items()
}
del _APP_PROFILE_DATA


@dataclass
class Config:
    # --- kiểu gõ
    input_method: str = "telex"             # telex | simple_telex | vni
    tone_style: ToneStyle = ToneStyle.NEW  # hoà | hòa (mặc định kiểu mới)
    spell_check: bool = True
    w_as_u: bool = False                    # w đứng riêng mặc định là chữ w; bật để w -> ư
    bracket_keys: bool = False              # [ -> ơ, ] -> ư
    quick_telex: bool = False               # cc=ch, gg=gi, ...
    allow_coda_k: bool = False              # âm cuối k cho tên riêng (Đắk Lắk); chỉ sắc/nặng
    # --- sau khi kết thúc từ
    macro_enabled: bool = True
    macro_digits: bool = False              # chữ số thuộc từ (cần macro_enabled) cho mp3, k8s
    macro_undo: bool = True                 # Backspace ngay sau macro: khôi phục phím gõ
    macro_in_off_mode: bool = False         # vẫn mở macro khi đang tắt tiếng Việt
    macros: dict[str, str] = field(default_factory=dict)
    auto_capitalize: bool = False           # viết hoa đầu câu
    # --- ngữ cảnh
    excluded_apps: list[str] = field(default_factory=list)
    app_modes: dict[str, bool] = field(default_factory=dict)  # nhớ Việt/Anh theo app
    default_vietnamese: bool = True
    edit_previous_word: bool = True         # dùng port SurroundingText
    app_profiles: dict[str, AppProfile] = field(default_factory=lambda: dict(DEFAULT_APP_PROFILES))
    restore_after_space: bool = True        # Backspace ngay sau dấu cách: nạp lại trạng thái từ trước (soft reset)
    smart_caps: bool = False                # 2 chữ hoa đầu từ (trừ DD) -> gõ thẳng, bảo vệ HTTP_PORT, JSON...
    # --- đầu ra
    unicode_form: str = "NFC"               # NFC | NFD
    # --- phiên bản tài liệu; bản cũ thiếu trường này được hiểu là 1
    schema_version: int = CONFIG_SCHEMA_VERSION


class ConfigStore(Protocol):
    def load(self) -> Config: ...
    def save(self, config: Config) -> None: ...


class AppContext(Protocol):
    def current_app(self) -> str | None: ...


class SurroundingText(Protocol):
    def before_cursor(self, n: int) -> str | None: ...


class Dictionary(Protocol):
    def is_english(self, word: str) -> bool: ...      # phím thô, chữ thường: 'data'
    def is_vietnamese(self, word: str) -> bool: ...   # chữ tiếng Việt, thường, NFC: 'nhưng'


@dataclass
class Ports:
    config: ConfigStore
    app: AppContext
    surrounding: SurroundingText
    dictionary: Dictionary


def config_to_dict(c: Config) -> dict:
    """Config -> dict JSON (tone_style thành chuỗi "old"/"new")."""
    d = asdict(c)
    d["tone_style"] = c.tone_style.value
    return d


def config_from_dict(d: dict) -> Config:
    """Dict JSON -> Config. Bỏ qua khoá lạ; nhận tone_style/app_profiles dạng thô.
    Dùng chung cho file config, FFI và WASM để mọi nơi chịu được config của bản mới hơn."""
    names = {f.name for f in fields(Config)}
    d = {k: v for k, v in d.items() if k in names}
    if "input_method" in d and not isinstance(d["input_method"], str):
        raise TypeError("input_method phải là chuỗi")
    if "tone_style" in d and not isinstance(d["tone_style"], (str, ToneStyle)):
        raise TypeError("tone_style phải là 'old'/'new'")
    if isinstance(d.get("tone_style"), str):
        d["tone_style"] = ToneStyle(d["tone_style"])
    if isinstance(d.get("app_profiles"), dict):
        # File người dùng chỉ cần chứa app ghi đè; preset còn lại vẫn giữ.
        profiles = dict(DEFAULT_APP_PROFILES)
        for key, value in d["app_profiles"].items():
            profiles[key] = value if isinstance(value, AppProfile) else AppProfile(**value)
        d["app_profiles"] = profiles
    return Config(**d)


def normalize_app_id(app_id: str | None) -> str:
    """Chuẩn hoá app id: bỏ path/.exe/.app/.desktop, lowercase, lấy segment cuối của bundle id."""
    if not app_id:
        return ""
    key = app_id.strip().lower().replace("\\", "/").rsplit("/", 1)[-1]
    for suffix in (".exe", ".app", ".desktop"):
        if key.endswith(suffix):
            key = key[: -len(suffix)]
    if ":" in key:                                    # WM_CLASS "instance:class"
        key = key.split(":", 1)[0]
    if "." in key and key.split(".")[0] in {"com", "org", "io", "net", "app"}:
        key = key.rsplit(".", 1)[-1]
    return APP_ALIASES.get(key, key)


def resolve_profile(config: Config, app_id: str | None) -> AppProfile:
    """So khớp app id đã chuẩn hoá; fallback theo segment cuối của bundle id."""
    key = normalize_app_id(app_id)
    if not key:
        return AppProfile()
    return config.app_profiles.get(key, AppProfile())
