#!/usr/bin/env python3
"""站点总验收（单页站点）：**一条命令，exit 0 才算过**。

设计：`docs/design/site-single-page.md`。只用 python3 标准库。

这个脚本取代了重构期的五个工具（`check-site.py` / `site-verify.py` /
`site-audit.py` / `site-functest.py` / `site-shot.py`，合计约 2800 行）——
站点只剩一个页面之后，那些检查里的绝大部分（跨页导航、搜索索引、
导航单源注入、多页 sitemap、逐页元数据）**没有对象可查**了。

查什么（每项一个名字，失败会指名）：

| 项 | 断言 |
|---|---|
| `pages`     | `site/` 下恰好一个 HTML 页（`index.html`），没有 `_partials/` 之类的内部目录 |
| `sitemap`   | `sitemap.xml` 与真实页面集合**双向相等**（不悬空、不漏页） |
| `links`     | 每个 `href`/`src` 都解析得到：相对路径文件存在，页内锚点有对应 `id` |
| `css-urls`  | CSS 里每个 `url(...)` 指向的文件存在（自托管字体） |
| `version`   | 页面/脚本里**没有写死的版本号**（`0.x.y` 字面量），且版本引用走 `data-site-version` |
| `meta`      | head 里 `lang`/`title`/`description`/`canonical`/`viewport`/`favicon`/`og:*` 齐全 |
| `markup`    | 标签配对（`html.parser` 走一遍）；没有内联 `style=` 属性 |
| `assets`    | 没有位图；html+css+js 的原始字节在预算内 |
| `data`      | `site/data/site.json` 与最新**已发布 tag** 一致（`gen-site-data.py --check`） |
| `render`    | （`--browser`）真 Chrome 跑一遍：资源零 404，且版本号被 JS 回填进 DOM |

用法：

```bash
python3 scripts/check-site.py              # 默认 9 项（不需要浏览器）
python3 scripts/check-site.py --browser    # 额外跑渲染实跑（要 Chrome）
python3 scripts/check-site.py --json       # 机器可读
```

退出码：**0** 全绿 / **1** 有断言判红 / **3** 无法判定（缺 tag、缺文件）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from html.parser import HTMLParser

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SITE = os.path.join(REPO_ROOT, "site")
PAGE = os.path.join(SITE, "index.html")
DATA = os.path.join(SITE, "data", "site.json")

# 体积预算（原始字节，不含字体）。单页站点没有理由接近这个数：
# 它是"再加一个页面/一个库之前，先想想"的闸门，不是性能指标。
SIZE_BUDGET = 120 * 1024
BITMAP_EXT = (".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".bmp", ".ico")

# 写死的版本号：拒 IP（127.0.0.1），抓 `v0.62.0` 与 `0.62.0`。
VERSION_LITERAL = re.compile(r"(?<![\d.])0\.\d+\.\d+(?![\d.])")
# 版本引用必须走这些钩子，值由 site.js 从 data/site.json 回填。
VERSION_HOOKS = ("data-site-version", "data-version-text", "data-version-href")

TEXT_EXT = (".html", ".css", ".js", ".txt", ".xml", ".json", ".svg")


class Failure(Exception):
    pass


def rel(path: str) -> str:
    return os.path.relpath(path, REPO_ROOT)


def walk_site() -> list[str]:
    out: list[str] = []
    for root, dirs, files in os.walk(SITE):
        dirs[:] = [d for d in dirs if d not in (".git",)]
        for name in files:
            out.append(os.path.join(root, name))
    return sorted(out)


def read(path: str) -> str:
    with open(path, encoding="utf-8") as handle:
        return handle.read()


# ── HTML 解析：标签配对 + 引用收集 ─────────────────────────────────────

VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link",
        "meta", "param", "source", "track", "wbr"}


class PageParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.stack: list[tuple[str, int]] = []
        self.errors: list[str] = []
        self.refs: list[tuple[str, str, int]] = []   # (attr, value, line)
        self.ids: set[str] = set()
        self.inline_styles: list[int] = []
        self.meta: dict[str, str] = {}

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        line = self.getpos()[0]
        if tag not in VOID:
            self.stack.append((tag, line))
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if "style" in attrs:
            self.inline_styles.append(line)
        for attr in ("href", "src"):
            if attr in attrs:
                self.refs.append((attr, attrs[attr], line))
        if tag == "html" and "lang" in attrs:
            self.meta["lang"] = attrs["lang"]
        if tag == "meta":
            key = attrs.get("name") or attrs.get("property")
            if key:
                self.meta[key] = attrs.get("content", "")
        if tag == "link" and "rel" in attrs:
            self.meta[f"link:{attrs['rel']}"] = attrs.get("href", "")
        if tag == "title":
            self.meta["_in_title"] = "1"
        if tag == "script" and "src" not in attrs:
            self.meta.setdefault("inline_scripts", "0")
            self.meta["inline_scripts"] = str(int(self.meta["inline_scripts"]) + 1)

    def handle_endtag(self, tag):
        if tag in VOID:
            return
        if not self.stack:
            self.errors.append(f"第 {self.getpos()[0]} 行：多余的 </{tag}>")
            return
        open_tag, line = self.stack.pop()
        if open_tag != tag:
            self.errors.append(f"第 {line} 行 <{open_tag}> 与第 {self.getpos()[0]} 行 </{tag}> 不配对")
        if tag == "title" and self.stack:
            self.meta.pop("_in_title", None)

    def handle_data(self, data):
        if self.meta.pop("_in_title", None):
            self.meta["title"] = data.strip()

    def close(self):
        super().close()
        for tag, line in self.stack:
            self.errors.append(f"第 {line} 行 <{tag}> 没有闭合")


# ── 各项断言 ───────────────────────────────────────────────────────────

def check_pages(files: list[str]) -> str:
    pages = [f for f in files if f.endswith(".html")]
    if len(pages) != 1 or os.path.abspath(pages[0]) != os.path.abspath(PAGE):
        raise Failure(f"site/ 下应当恰好一个页面 index.html，实际：{[rel(p) for p in pages]}")
    internal = [rel(f) for f in files if os.sep + "_" in f]
    if internal:
        raise Failure(f"发布树里不该有内部目录（_partials 之类）：{internal}")
    return f"1 页（{rel(PAGE)}）"


def check_sitemap(files: list[str], parser: PageParser) -> str:
    path = os.path.join(SITE, "sitemap.xml")
    if not os.path.exists(path):
        raise Failure("缺 site/sitemap.xml")
    locs = re.findall(r"<loc>([^<]+)</loc>", read(path))
    if not locs:
        raise Failure("sitemap.xml 里一条 <loc> 都没有")
    if len(locs) != len(set(locs)):
        raise Failure("sitemap.xml 里有重复 URL")
    # 双向：页面集合（除 404）必须与 loc 集合一一对应。
    pages = {os.path.basename(f) for f in files if f.endswith(".html")}
    listed = {u.rstrip("/").rsplit("/", 1)[-1] or "index.html" for u in locs}
    listed = {name if name.endswith(".html") else "index.html" for name in listed}
    if pages != listed:
        raise Failure(f"sitemap 与真实页面不一致：页面 {sorted(pages)} vs sitemap {sorted(listed)}")
    return f"{len(locs)} 条，与页面集合双向相等"


def check_links(parser: PageParser) -> str:
    checked = 0
    for attr, value, line in parser.refs:
        if value.startswith(("http://", "https://", "mailto:", "data:")):
            continue
        if value.startswith("#"):
            anchor = value[1:]
            if anchor and anchor not in parser.ids:
                raise Failure(f"第 {line} 行：锚点 #{anchor} 在页面里没有对应 id")
            checked += 1
            continue
        target = value.split("#", 1)[0].split("?", 1)[0]
        if not target:
            continue
        resolved = os.path.normpath(os.path.join(SITE, target))
        if not os.path.exists(resolved):
            raise Failure(f"第 {line} 行：{attr}=\"{value}\" 指向不存在的 {target}")
        checked += 1
    return f"{checked} 条站内引用全部可解析"


def check_css_urls(files: list[str]) -> str:
    checked = 0
    for path in files:
        if not path.endswith(".css"):
            continue
        base = os.path.dirname(path)
        for url in re.findall(r"url\(\s*[\"']?([^\"')]+)[\"']?\s*\)", read(path)):
            if url.startswith(("http://", "https://", "data:")):
                continue
            if not os.path.exists(os.path.normpath(os.path.join(base, url))):
                raise Failure(f"{rel(path)}：url({url}) 指向不存在的文件")
            checked += 1
    return f"{checked} 个资源引用存在"


def check_version(files: list[str], html: str) -> str:
    if not any(hook in html for hook in VERSION_HOOKS):
        raise Failure("页面里没有任何版本引用钩子（data-site-version 等）——版本号只能由数据回填")
    offenders: list[str] = []
    for path in files:
        if not path.endswith(TEXT_EXT):
            continue
        if os.path.abspath(path) == os.path.abspath(DATA):
            continue          # 这一份**就是**版本号的来源
        for num, line in enumerate(read(path).splitlines(), 1):
            if VERSION_LITERAL.search(line):
                offenders.append(f"{rel(path)}:{line}")
    if offenders:
        raise Failure("写死的版本号："
                      + "、".join(offenders)
                      + "（版本号只能由 site/data/site.json 回填）")
    return f"{len(VERSION_HOOKS)} 类钩子，0 处写死"


def check_meta(parser: PageParser) -> str:
    required = ["lang", "title", "description", "viewport", "og:title", "og:description"]
    missing = [k for k in required if not parser.meta.get(k)]
    if not parser.meta.get("link:canonical"):
        missing.append("canonical")
    if not parser.meta.get("link:icon"):
        missing.append("favicon")
    if missing:
        raise Failure(f"head 缺元数据：{missing}")
    if parser.meta.get("inline_scripts") != "1":
        raise Failure("内联 <script> 只允许 head 里那一段配色 bootstrap（防首帧闪烁）")
    return f"{len(required) + 2} 项齐全（lang={parser.meta['lang']}）"


def check_markup(parser: PageParser) -> str:
    if parser.errors:
        raise Failure("标签不配对：" + "；".join(parser.errors[:4]))
    if parser.inline_styles:
        raise Failure(f"内联 style= 出现在第 {parser.inline_styles} 行（样式只能进 site.css）")
    return "标签配对，无内联样式"


def check_assets(files: list[str]) -> str:
    bitmaps = [rel(f) for f in files if f.lower().endswith(BITMAP_EXT)]
    if bitmaps:
        raise Failure(f"站点里不该有位图（用 CSS/SVG）：{bitmaps}")
    total = sum(os.path.getsize(f) for f in files
                if f.endswith((".html", ".css", ".js")))
    if total > SIZE_BUDGET:
        raise Failure(f"html+css+js 共 {total} 字节，超过预算 {SIZE_BUDGET}")
    return f"零位图；html+css+js {total} 字节（预算 {SIZE_BUDGET}）"


def check_data() -> str:
    proc = subprocess.run([sys.executable, os.path.join(REPO_ROOT, "scripts", "gen-site-data.py"),
                           "--check"], cwd=REPO_ROOT, capture_output=True, text=True)
    if proc.returncode == 3:
        raise Failure("解析不出已发布 tag（浅克隆缺 tag？）—— 无法判定，不判绿")
    if proc.returncode != 0:
        raise Failure((proc.stderr or proc.stdout).strip())
    return (proc.stdout or "").strip().removeprefix("site data: ")


# ── 渲染实跑（可选）───────────────────────────────────────────────────

def find_chrome() -> str | None:
    for candidate in (
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "google-chrome", "google-chrome-stable", "chromium", "chromium-browser",
    ):
        if os.path.sep in candidate:
            if os.path.exists(candidate):
                return candidate
        else:
            from shutil import which
            found = which(candidate)
            if found:
                return found
    return None


def check_render() -> str:
    """真 Chrome 跑一遍：资源零 404，且版本号被 JS 回填进 DOM。"""
    import http.server

    import threading
    import shutil
    import tempfile
    import time

    chrome = find_chrome()
    if not chrome:
        raise Failure("找不到 Chrome（--browser 需要它）")

    missing: list[str] = []

    class Handler(http.server.SimpleHTTPRequestHandler):
        # **不要**改成 HTTP/1.1：keep-alive 会让 Chrome 认为还有未完成的网络活动，
        # `--virtual-time-budget` 于是永远走不完 ⇒ 挂死。默认 HTTP/1.0（响应完即关
        # 连接）才能让虚拟时间正常推进。这是实测出来的，别"顺手升级协议"。
        def __init__(self, *a, **kw):
            super().__init__(*a, directory=SITE, **kw)

        def log_message(self, fmt, *a):        # 静音
            pass

        def send_error(self, code, message=None, explain=None):
            missing.append(self.path)
            super().send_error(code, message, explain)

    # **必须多线程**：Chrome 会开一条不说话的预连接，单线程的 `TCPServer` 一旦
    # 轮询到它就会卡在 `readline()` 上，把后面真正的请求全挡住 ⇒ Chrome 永远等
    # 不到页面、`--virtual-time-budget` 也永远走不完（实测：同一条命令时红时绿）。
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler) as httpd:
        httpd.daemon_threads = True
        port = httpd.server_address[1]
        thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        thread.start()
        profile = tempfile.mkdtemp(prefix="soko-chrome-")
        try:
            # 用**旧版** headless：macOS 上 `--headless=new` + `--dump-dom` 会挂住
            # 不返回（实测 120s 超时）。旧版能出 DOM。
            #
            # `--virtual-time-budget` 而不是 `--timeout`：后者会在
            # `fetch("data/site.json")` 落定**之前**就把 DOM 倒出来，于是"版本号已
            # 回填"时红时绿（实测同样命令 4 次命中 vs 0 次）。
            #
            # **不等 Chrome 退出**：macOS 上它把完整 DOM 打到 stdout 之后**进程不退出**
            # （实测 stdout 已有 15 KB 完整 DOM，进程挂到被杀为止）。所以判据是
            # "DOM 到齐了没有"，不是"进程结束了没有"——一见到 `</html>` 就杀掉它。
            proc = subprocess.Popen([
                chrome, "--headless", "--disable-gpu", "--no-sandbox",
                "--no-first-run", "--no-default-browser-check",
                "--disable-extensions", "--disable-background-networking",
                "--disable-sync", "--hide-scrollbars",
                f"--user-data-dir={profile}", "--virtual-time-budget=5000",
                "--dump-dom", f"http://127.0.0.1:{port}/index.html",
            ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)

            chunks: list[str] = []

            def pump() -> None:
                for line in proc.stdout or ():
                    chunks.append(line)

            reader = threading.Thread(target=pump, daemon=True)
            reader.start()
            deadline = time.time() + 30
            while time.time() < deadline:
                if "</html>" in "".join(chunks):
                    break
                if proc.poll() is not None:
                    break
                time.sleep(0.1)
            dom = "".join(chunks)
            if proc.poll() is None:
                proc.kill()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                pass
        finally:
            shutil.rmtree(profile, ignore_errors=True)
            httpd.shutdown()

    if missing:
        raise Failure(f"渲染时有 404：{sorted(set(missing))[:5]}")
    if not dom.strip():
        raise Failure("Chrome 没有输出 DOM（进程可能起不来）")
    try:
        version = json.loads(read(DATA)).get("version", "")
    except (OSError, json.JSONDecodeError):
        version = ""
    if not version:
        raise Failure("site/data/site.json 里没有 version，无法验证回填")
    if f">{version}<" not in dom.replace(" ", ""):
        raise Failure(f"DOM 里没找到回填后的版本号 {version}（site.js 没跑或被缓存）")
    return f"Chrome 渲染通过，版本 {version} 已回填，资源零 404"


# ── 主流程 ─────────────────────────────────────────────────────────────

CHECKS = [
    ("pages", lambda files, parser, html: check_pages(files)),
    ("sitemap", lambda files, parser, html: check_sitemap(files, parser)),
    ("links", lambda files, parser, html: check_links(parser)),
    ("css-urls", lambda files, parser, html: check_css_urls(files)),
    ("version", lambda files, parser, html: check_version(files, html)),
    ("meta", lambda files, parser, html: check_meta(parser)),
    ("markup", lambda files, parser, html: check_markup(parser)),
    ("assets", lambda files, parser, html: check_assets(files)),
    ("data", lambda files, parser, html: check_data()),
]


def main(argv: list[str] | None = None) -> int:
    parser_args = argparse.ArgumentParser(description="站点总验收（单页站点）")
    parser_args.add_argument("--browser", action="store_true", help="额外跑真 Chrome 渲染检查")
    parser_args.add_argument("--json", dest="as_json", action="store_true", help="机器可读输出")
    args = parser_args.parse_args(argv)

    if not os.path.exists(PAGE):
        print(f"site: 找不到 {rel(PAGE)}", file=sys.stderr)
        return 3

    files = walk_site()
    page_parser = PageParser()
    page_parser.feed(read(PAGE))
    page_parser.close()
    html = read(PAGE)

    results: list[dict] = []
    failed = False
    for name, fn in CHECKS:
        try:
            detail = fn(files, page_parser, html)
            results.append({"check": name, "ok": True, "detail": detail})
        except Failure as error:
            results.append({"check": name, "ok": False, "detail": str(error)})
            failed = True

    if args.browser:
        try:
            results.append({"check": "render", "ok": True, "detail": check_render()})
        except Failure as error:
            results.append({"check": "render", "ok": False, "detail": str(error)})
            failed = True

    if args.as_json:
        print(json.dumps({"ok": not failed, "checks": results}, ensure_ascii=False, indent=2))
    else:
        for row in results:
            print(f"  {'✓' if row['ok'] else '✗'} {row['check']:<9} {row['detail']}")
        total = len(results)
        green = sum(1 for row in results if row["ok"])
        print(f"site: {'ok' if not failed else 'FAILED'}（{green}/{total}）")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
