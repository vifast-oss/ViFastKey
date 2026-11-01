#!/usr/bin/env python3
"""Build static HTML docs at /docs/ from docs/ime-survey/*.md.

Usage: python3 docs/ime-survey/build_site.py
Requires: node/npx (marked@15).
"""
import html
import pathlib
import re
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
SRC = pathlib.Path(__file__).resolve().parent
OUT = ROOT / "web/docs"
TMP = pathlib.Path("/private/tmp/vfk-docs-build")

DOCS = [
    {
        "slug": "areca-ime",
        "file": "areca-ime.md",
        "title": "Areca IME — sổ tay kỹ thuật",
        "desc": "Kiến trúc, scheduler, 6 backend rewrite, trust probe, timing, config, build, debug và test của Areca IME.",
        "tag": "Sổ tay",
        "index": True,
    },
    {
        "slug": "survey-all-imes",
        "file": "survey-all-imes.md",
        "title": "Khảo sát 31 bộ gõ tiếng Việt",
        "desc": "Phân tích từng repo theo nhóm Fcitx5, IBus, Wayland và native; ma trận tính năng và bài học chung.",
        "tag": "Khảo sát",
        "index": True,
    },
    {
        "slug": "comparison-vifastkey",
        "file": "comparison-vifastkey.md",
        "title": "ViFastKey so với 31 bộ gõ",
        "desc": "So sánh theo 8 trục, scorecard, điểm mạnh/yếu và roadmap đề xuất cho ViFastKey.",
        "tag": "So sánh",
        "index": True,
    },
]

CSS = r"""
@font-face {
  font-family: "Be Vietnam Pro";
  src: url("/fonts/BeVietnamPro-Regular.ttf") format("truetype");
  font-weight: 400; font-style: normal; font-display: swap;
}
:root {
  --bg: #fbf6ee; --surface: #fff; --cream: #fff8ec; --ink: #1f2430;
  --body: #3f4652; --muted: #5f6673; --accent: #d42a22; --accent-dark: #b3140f;
  --gold: #ffb800; --border: #e8ddd0; --radius: 14px;
  --font: "Be Vietnam Pro", system-ui, Arial, sans-serif;
  --mono: "SFMono-Regular", ui-monospace, Menlo, Consolas, monospace;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; scroll-padding-top: 84px; }
body {
  margin: 0; color: var(--ink); background: var(--bg);
  font-family: var(--font); font-size: 16px; line-height: 1.65;
  -webkit-font-smoothing: antialiased; text-rendering: optimizeLegibility;
}
a { color: var(--accent-dark); text-decoration-thickness: 1px; text-underline-offset: 2px; }
a:hover { color: var(--accent); }
.doc-header {
  position: sticky; top: 0; z-index: 10;
  background: rgba(251, 246, 238, 0.92); backdrop-filter: blur(10px);
  border-bottom: 1px solid var(--border);
}
.doc-header .inner {
  max-width: 1040px; margin: 0 auto; padding: 12px 24px;
  display: flex; align-items: center; gap: 18px; justify-content: space-between;
}
.brand { display: inline-flex; align-items: center; gap: 10px; font-weight: 600; color: var(--ink); text-decoration: none; }
.brand img { width: 30px; height: 30px; }
.brand .red { color: var(--accent); }
.brand em { font-style: normal; }
.doc-nav { display: flex; gap: 16px; font-size: 14px; }
.doc-nav a { color: var(--muted); text-decoration: none; }
.doc-nav a:hover { color: var(--ink); }
.wrap { max-width: 1040px; margin: 0 auto; padding: 0 24px; }
.crumbs { font-size: 13px; color: var(--muted); padding: 22px 0 0; }
.crumbs a { color: var(--muted); text-decoration: none; }
.crumbs a:hover { color: var(--ink); }
article { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius);
  padding: 40px 48px; margin: 18px 0 48px; box-shadow: 0 18px 50px -38px rgba(90,45,25,.35); }
article h1 { font-size: 34px; line-height: 1.2; margin: 0 0 8px; }
article h2 { font-size: 25px; line-height: 1.3; margin: 44px 0 14px; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
article h3 { font-size: 19px; margin: 30px 0 10px; }
article h4 { font-size: 17px; margin: 24px 0 8px; }
article p, article li { color: var(--body); }
article { overflow-wrap: break-word; }
article strong { color: var(--ink); }
article blockquote { margin: 18px 0; padding: 14px 18px; background: var(--cream);
  border-left: 4px solid var(--gold); border-radius: 0 8px 8px 0; }
article blockquote p { margin: 6px 0; }
article code { font-family: var(--mono); font-size: 0.9em; background: #f4eee5;
  padding: 2px 5px; border-radius: 5px; }
article pre { background: #1f2430; color: #f6f1e8; padding: 18px 20px; border-radius: 10px;
  overflow-x: auto; line-height: 1.5; }
article pre code { background: none; padding: 0; color: inherit; font-size: 13.5px; }
.table-wrap { margin: 18px 0; overflow-x: auto; -webkit-overflow-scrolling: touch; border-radius: 10px; }
article table { display: table; width: 100%; min-width: 100%; border-collapse: collapse; margin: 0; font-size: 14px; }
article table thead { background: var(--cream); }
article th, article td { border: 1px solid var(--border); padding: 8px 10px; text-align: left; vertical-align: top; }
article th { font-weight: 600; color: var(--ink); white-space: nowrap; }
article tbody tr:nth-child(even) { background: #fdfaf5; }
article hr { border: 0; border-top: 1px solid var(--border); margin: 34px 0; }
article img { max-width: 100%; height: auto; }
article ul, article ol { padding-left: 26px; }
article li { margin: 5px 0; }
.doc-footer { border-top: 1px solid var(--border); padding: 24px 0 48px; color: var(--muted); font-size: 13px; }
.cards { display: grid; gap: 16px; margin: 26px 0 48px; }
.card { display: block; background: var(--surface); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 22px 24px; text-decoration: none; color: inherit;
  transition: transform .15s ease, box-shadow .15s ease; }
.card:hover { transform: translateY(-2px); box-shadow: 0 18px 40px -28px rgba(90,45,25,.45); }
.card .tag { display: inline-block; font-size: 12px; font-weight: 600; letter-spacing: .04em;
  color: var(--accent-dark); background: var(--cream); border: 1px solid var(--border);
  padding: 2px 8px; border-radius: 999px; margin-bottom: 10px; }
.card h2 { margin: 0 0 6px; font-size: 20px; border-bottom: 0; padding-bottom: 0; }
.card p { margin: 0; color: var(--muted); font-size: 14px; }
.lede { color: var(--muted); margin: 0 0 8px; }
@media (max-width: 720px) {
  .wrap { padding: 0 12px; }
  .doc-header .inner { padding: 10px 12px; }
  article { padding: 20px 16px; margin: 12px 0 32px; border-radius: 12px; }
  article h1 { font-size: 26px; }
  article h2 { font-size: 21px; margin-top: 34px; }
  article h3 { font-size: 18px; }
  article blockquote { padding: 12px 14px; }
  article pre { padding: 14px 14px; border-radius: 8px; }
  article pre code { font-size: 12.5px; }
  article table { font-size: 13px; }
  article th, article td { padding: 7px 8px; }
  article th { white-space: normal; }
  .table-wrap { margin: 14px 0; }
  .cards { gap: 12px; }
  .card { padding: 18px 16px; }
  .doc-nav { display: none; }
}
@media (max-width: 380px) {
  .doc-suffix { display: none; }
}
"""

TEMPLATE = """<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · ViFastKey</title>
<meta name="description" content="{desc}">
<link rel="canonical" href="https://vifastkey.hoctuthien.com/docs/{slug}.html">
<meta name="theme-color" content="#FFF8EC">
<link rel="icon" href="/brand/favicon.ico" sizes="any">
<link rel="icon" type="image/png" sizes="32x32" href="/brand/favicon-32.png">
<link rel="stylesheet" href="/docs/docs.css">
{robots}
</head>
<body>
<header class="doc-header">
  <div class="inner">
    <a class="brand" href="/" aria-label="ViFastKey — về trang chủ">
      <img src="/brand/vifastkey-icon-192.png" alt="">
      <span><span class="red">Vi</span><em>Fast</em><span class="red">Key</span> <span class="doc-suffix" style="font-weight:400;color:var(--muted)">· Tài liệu</span></span>
    </a>
    <nav class="doc-nav" aria-label="Tài liệu">
      <a href="/">Trang chủ</a>
      <a href="/docs/">Tài liệu</a>
      <a href="/docs/areca-ime.html">Areca</a>
    </nav>
  </div>
</header>
<div class="wrap">
  <div class="crumbs"><a href="/">ViFastKey</a> / <a href="/docs/">Tài liệu</a> / {crumb}</div>
  <article>
{body}
  </article>
</div>
<footer class="doc-footer"><div class="wrap">ViFastKey · tài liệu kỹ thuật · cập nhật 2026-11-01</div></footer>
</body>
</html>
"""

INDEX_TEMPLATE = """<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Tài liệu kỹ thuật · ViFastKey</title>
<meta name="description" content="Khảo sát 31 bộ gõ tiếng Việt, sổ tay kỹ thuật Areca IME và so sánh với ViFastKey.">
<link rel="canonical" href="https://vifastkey.hoctuthien.com/docs/">
<meta name="theme-color" content="#FFF8EC">
<link rel="icon" href="/brand/favicon.ico" sizes="any">
<link rel="stylesheet" href="/docs/docs.css">
</head>
<body>
<header class="doc-header">
  <div class="inner">
    <a class="brand" href="/"><img src="/brand/vifastkey-icon-192.png" alt="">
      <span><span class="red">Vi</span><em>Fast</em><span class="red">Key</span> <span class="doc-suffix" style="font-weight:400;color:var(--muted)">· Tài liệu</span></span>
    </a>
    <nav class="doc-nav"><a href="/">Trang chủ</a><a href="/docs/">Tài liệu</a></nav>
  </div>
</header>
<div class="wrap">
  <div class="crumbs"><a href="/">ViFastKey</a> / Tài liệu</div>
  <article>
    <h1>Tài liệu kỹ thuật</h1>
    <p class="lede">Khảo sát bộ gõ tiếng Việt và ghi chú kiến trúc. Snapshot 2026-11-01.</p>
    <div class="cards">
{cards}
    </div>
    <h2>Dữ liệu</h2>
    <p><a href="/docs/data/imes.csv">imes.csv</a> — bảng tổng hợp 31 repository (một dòng/repo).</p>
  </article>
</div>
<footer class="doc-footer"><div class="wrap">ViFastKey · tài liệu kỹ thuật · cập nhật 2026-11-01</div></footer>
</body>
</html>
"""


def convert(md_path: pathlib.Path, slug: str) -> str:
    frag = TMP / f"{slug}.frag.html"
    subprocess.run(
        ["npx", "--yes", "marked@15", "-i", str(md_path), "-o", str(frag)],
        check=True,
    )
    body = frag.read_text(encoding="utf-8")
    # .md links -> .html
    body = re.sub(r'href="([^"]+)\.md(#[^"]*)?"', lambda m: f'href="{m.group(1)}.html{m.group(2) or ""}"', body)
    # Bọc bảng trong div cuộn ngang để không đẩy document rộng trên mobile.
    body = body.replace("<table>", '<div class="table-wrap"><table>').replace("</table>", "</table></div>")
    return body


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "data").mkdir(exist_ok=True)
    TMP.mkdir(parents=True, exist_ok=True)
    (OUT / "docs.css").write_text(CSS.strip() + "\n", encoding="utf-8")

    for doc in DOCS:
        body = convert(SRC / doc["file"], doc["slug"])
        robots = '<meta name="robots" content="noindex">' if doc["slug"] != "areca-ime" else ""
        page = TEMPLATE.format(
            title=html.escape(doc["title"]),
            desc=html.escape(doc["desc"]),
            slug=doc["slug"],
            crumb=html.escape(doc["title"]),
            body=body,
            robots=robots,
        )
        (OUT / f"{doc['slug']}.html").write_text(page, encoding="utf-8")

    cards = "\n".join(
        f'      <a class="card" href="/docs/{d["slug"]}.html">'
        f'<span class="tag">{html.escape(d["tag"])}</span>'
        f'<h2>{html.escape(d["title"])}</h2><p>{html.escape(d["desc"])}</p></a>'
        for d in DOCS if d["index"]
    )
    (OUT / "index.html").write_text(INDEX_TEMPLATE.format(cards=cards), encoding="utf-8")

    shutil.copy2(SRC / "data/imes.csv", OUT / "data/imes.csv")
    for p in OUT.rglob("*"):
        p.chmod(0o755 if p.is_dir() else 0o644)
    print("built", OUT)
    for p in sorted(OUT.rglob("*")):
        if p.is_file():
            print(f"  {p.relative_to(OUT)}  {p.stat().st_size/1024:.1f} KB")


if __name__ == "__main__":
    main()
