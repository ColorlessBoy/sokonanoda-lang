#!/usr/bin/env python3
"""知识卡渲染器：data/cards.json → out/<id>.png（Chrome 无头模式截图，2x）。

内容（文本）是唯一资产，本文件只是**渲染器之一**：同一份 JSON 以后还可以渲染成
聊天卡片、右栏面板、PDF。用法：

    python3 render.py --list
    python3 render.py --id is-strict-order
    python3 render.py --all
"""
import argparse
import html
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data" / "cards.json"
OUT = ROOT / "out"
CHROME_CANDIDATES = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
]
WIDTH = 1080
SCALE = 2
PAGE_BG = "#f2f4f8"

CSS = """
*{box-sizing:border-box;margin:0;padding:0}
:root{
  --ink:#11151c;--muted:#5b6472;--line:#e6e9ef;--accent:#2f6bff;--accent-soft:#eaf0ff;
  --card:#ffffff;--code-bg:#14171d;--code-fg:#e9eef7;--inline-bg:#eef1f7;
}
html,body{background:PAGE_BG_PLACEHOLDER;width:1080px}
body{font-family:-apple-system,BlinkMacSystemFont,"PingFang SC","Hiragino Sans GB","Helvetica Neue",sans-serif;color:var(--ink)}
.card{width:1000px;margin:40px 40px;background:var(--card);border-radius:28px;overflow:hidden;box-shadow:0 18px 48px rgba(15,23,42,.10)}
.head{padding:56px 56px 26px;background:linear-gradient(135deg,#f5f8ff 0%,#ffffff 62%)}
.kicker{display:inline-flex;align-items:center;gap:10px;font-size:22px;font-weight:600;color:var(--accent);background:var(--accent-soft);border-radius:999px;padding:8px 20px}
h1{font-size:58px;line-height:1.16;letter-spacing:.005em;margin:24px 0 20px}
.one{font-size:28px;line-height:1.55;color:#1b2430;border-left:6px solid var(--accent);background:#f6f8ff;border-radius:0 14px 14px 0;padding:18px 26px}
.body{padding:0 56px 8px}
.def{margin:28px 0 6px;background:var(--code-bg);border-radius:18px;padding:26px 30px 30px;color:var(--code-fg);font:24px/1.6 "Menlo","SF Mono",monospace;white-space:pre-wrap;overflow-wrap:anywhere}
.def .lang{display:block;font:700 17px/1 -apple-system,sans-serif;letter-spacing:.12em;text-transform:uppercase;color:#8ea3c9;margin-bottom:14px}
section{margin-top:36px}
h2{font-size:30px;line-height:1.2;display:flex;align-items:center;gap:12px;margin-bottom:16px}
h2::before{content:"";width:9px;height:28px;border-radius:4px;background:var(--accent)}
table{width:100%;border-collapse:separate;border-spacing:0;border:1px solid var(--line);border-radius:16px;overflow:hidden;font-size:25px}
th{background:#f6f8fb;text-align:left;padding:15px 18px;font-weight:600;color:#28303c}
td{padding:15px 18px;border-top:1px solid var(--line);color:#1d2632;vertical-align:top;line-height:1.5}
tr:nth-child(even) td{background:#fafbfd}
ul{list-style:none}
li{position:relative;padding-left:38px;margin:13px 0;font-size:26px;line-height:1.6;color:#1d2632}
li::before{content:"›";position:absolute;left:8px;top:-2px;color:var(--accent);font-weight:800;font-size:30px}
code{font-family:"Menlo","SF Mono",monospace;font-size:.9em;background:var(--inline-bg);border-radius:6px;padding:2px 8px}
.def code{background:transparent;padding:0;font-size:1em}
.foot{margin:44px 56px 0;padding:22px 0 42px;border-top:1px solid var(--line);display:flex;justify-content:space-between;gap:24px;color:var(--muted);font-size:20px}
.foot .brand{font-weight:600;color:#39445a;white-space:nowrap}
""".replace("PAGE_BG_PLACEHOLDER", PAGE_BG)

def rich(text: str) -> str:
    """把 `code` 片段转成 <code>，其余文本转义。"""
    parts = re.split(r"(`[^`]*`)", str(text))
    out = []
    for part in parts:
        if len(part) >= 2 and part.startswith("`") and part.endswith("`"):
            out.append("<code>" + html.escape(part[1:-1]) + "</code>")
        else:
            out.append(html.escape(part))
    return "".join(out)

def section_html(section: dict) -> str:
    kind = section.get("kind")
    heading = html.escape(str(section.get("heading", "")))
    if kind == "table":
        columns = section.get("columns", [])
        head = "".join("<th>" + rich(col) + "</th>" for col in columns)
        rows = []
        for row in section.get("rows", []):
            rows.append("<tr>" + "".join("<td>" + rich(cell) + "</td>" for cell in row) + "</tr>")
        return ("<section><h2>" + heading + "</h2><table><thead><tr>" + head + "</tr></thead><tbody>"
                + "".join(rows) + "</tbody></table></section>")
    if kind == "list":
        items = "".join("<li>" + rich(item) + "</li>" for item in section.get("items", []))
        return "<section><h2>" + heading + "</h2><ul>" + items + "</ul></section>"
    return "<section><h2>" + heading + "</h2><p>" + rich(section.get("text", "")) + "</p></section>"

def card_html(card: dict) -> str:
    definition = card.get("definition") or {}
    lang = html.escape(str(definition.get("lang", "text")))
    code = html.escape(str(definition.get("code", "")))
    sections = "".join(section_html(section) for section in card.get("sections", []))
    return f"""<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><style>{CSS}</style></head>
<body>
  <div class="card">
    <div class="head">
      <span class="kicker">{html.escape(str(card.get("kicker", "")))}</span>
      <h1>{html.escape(str(card.get("title", "")))}</h1>
      <p class="one">{rich(card.get("oneLiner", ""))}</p>
    </div>
    <div class="body">
      <div class="def"><span class="lang">{lang}</span>{code}</div>
      {sections}
    </div>
    <div class="foot">
      <span>出处：{rich(card.get("source", ""))}</span>
      <span class="brand">卷 I 集合论 · 知识卡</span>
    </div>
  </div>
  <script>document.fonts.ready.then(function () {{ document.title = "H:" + document.documentElement.scrollHeight; }});</script>
</body></html>"""

def chrome_path() -> str:
    for candidate in CHROME_CANDIDATES:
        if Path(candidate).exists():
            return candidate
    raise SystemExit("找不到 Chrome/Chromium：需要它来做无头截图")

def chrome_profile() -> str:
    profile = OUT / ".chrome-profile"
    profile.mkdir(parents=True, exist_ok=True)
    return str(profile)

def measure_height(chrome: str, page: Path) -> int:
    result = subprocess.run(
        [chrome, "--headless", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
         "--no-default-browser-check", "--no-sandbox", "--user-data-dir=" + chrome_profile(),
         "--virtual-time-budget=6000", "--dump-dom", page.as_uri()],
        capture_output=True, text=True, timeout=90)
    match = re.search(r"<title>H:(\d+)</title>", result.stdout)
    if not match:
        tail = (result.stderr or result.stdout or "")[-400:]
        raise SystemExit("高度测量失败：" + tail)
    return max(200, int(match.group(1)))

def render(card: dict, chrome: str) -> Path:
    card_id = card["id"]
    build = OUT / ".build"
    build.mkdir(parents=True, exist_ok=True)
    page = build / (card_id + ".html")
    page.write_text(card_html(card), encoding="utf-8")
    height = measure_height(chrome, page)
    target = OUT / (card_id + ".png")
    subprocess.run(
        [chrome, "--headless", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
         "--no-default-browser-check", "--no-sandbox", "--user-data-dir=" + chrome_profile(),
         "--force-device-scale-factor=" + str(SCALE),
         "--window-size=" + str(WIDTH) + "," + str(height),
         "--virtual-time-budget=6000", "--screenshot=" + str(target), page.as_uri()],
        capture_output=True, text=True, timeout=120)
    if not target.exists() or target.stat().st_size == 0:
        raise SystemExit("截图失败：" + str(target))
    return target

def main() -> int:
    parser = argparse.ArgumentParser(description="渲染知识卡 PNG")
    parser.add_argument("--id", help="卡片 id")
    parser.add_argument("--all", action="store_true", help="渲染全部卡片")
    parser.add_argument("--list", action="store_true", help="列出卡片 id")
    args = parser.parse_args()
    data = json.loads(DATA.read_text(encoding="utf-8"))
    cards = data["cards"]
    if args.list:
        for card in cards:
            print(card["id"], "·", card["title"])
        return 0
    if args.all:
        selected = cards
    elif args.id:
        selected = [card for card in cards if card["id"] == args.id]
        if not selected:
            raise SystemExit("no such card: " + args.id)
    else:
        parser.print_help()
        return 2
    chrome = chrome_path()
    OUT.mkdir(parents=True, exist_ok=True)
    for card in selected:
        path = render(card, chrome)
        print("PNG " + str(path.relative_to(ROOT)) + "  " + str(path.stat().st_size // 1024) + " KB")
    return 0

if __name__ == "__main__":
    sys.exit(main())
