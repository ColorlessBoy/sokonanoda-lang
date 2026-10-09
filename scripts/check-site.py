#!/usr/bin/env python3
"""站点总验收：**一条命令，exit 0 才算过**。

设计：`docs/design/site-single-page.md`。只用 python3 标准库。

站点现在是**一页**：`index.html`。更新日志**不再有子页**（2026-10-09 用户拍板：
「changelog 不要单独子页面了，链接到 github 对应的地方吧」⇒ 页脚直链
`editor/vscode/CHANGELOG.md` 的 GitHub 页面，站点少一份生成物与一份逐字节判据 ✓）。这里查什么
（每项一个名字，失败会指名）：

| 项 | 断言 |
|---|---|
| `pages`     | `site/` 下恰好这一页，没有 `_partials/` 之类的内部目录 |
| `sitemap`   | `sitemap.xml` 与真实页面集合**双向相等**（不悬空、不漏页） |
| `links`     | 每页的 `href`/`src` 都解析得到：相对路径文件存在；锚点（含跨页锚点）有对应 `id` |
| `css-urls`  | CSS 里每个 `url(...)` 指向的文件存在（自托管字体） |
| `version`   | 每页都有版本回填钩子；除生成物外**没有写死的版本号**（`0.x.y` 字面量） |
| `meta`      | 每页 head 里 `lang`/`title`/`description`/`canonical`/`viewport`/`favicon`/`og:*` 齐全，且**可执行**内联 script 只有 head 那一段（文案字典是 `application/json`，不算） |\n| `i18n`      | 每页引用 `assets/i18n.js`（内含 `navigator.languages` 的 zh 判定）；英文文案字典是合法 JSON，且**与页面 `data-i18n*` 键逐个对齐**（漏译/多译都判红） |
| `markup`    | 标签配对（`html.parser` 走一遍）；没有内联 `style=` 属性 |
| `assets`    | 位图**只允许**头图 `assets/hero-vscode.png`（且有体积预算、宽高比与 `<img>` 属性一致）；html+css+js 在预算内 |
| `data`      | `site/data/site.json` 与最新**已发布 tag** 一致（`gen-site-data.py --check`） |
| `render`    | （`--browser`）真 Chrome 跑这一页 × 两种浏览器语言（`--accept-lang`）：资源零 404、版本号已回填、**中文读者看中文 / 其它语言看英文** |\n| `layout`    | （`--browser`）真 Chrome 量横向溢出：4 个宽度，`scrollWidth` 不许超过视口 |

用法：

```bash
python3 scripts/check-site.py              # 默认 11 项（不需要浏览器）
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
import struct
import subprocess
import sys
from html.parser import HTMLParser

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SITE = os.path.join(REPO_ROOT, "site")
INDEX = os.path.join(SITE, "index.html")
DATA = os.path.join(SITE, "data", "site.json")

# 页面集合：**恰好这一页**（2026-10-09 起单页）。加页面的时候，sitemap 与这里要一起改
# （check_sitemap 会双向对账）。
PAGES = ["index.html"]

# 体积预算（原始字节，不含字体）。它是"再加一个页面/一个库之前，先想想"的闸门。
SIZE_BUDGET = 120 * 1024
# 头图是**真截图**（不是装饰位图），所以有位图预算：一张、有上限。
HERO = "assets/hero-vscode.png"
# 两版配色（用户 2026-10-09）：站点跟随系统配色，暗色页上贴亮色截图会刺眼 ⇒ 头图也两版。
# **两张必须是同一裁切几何**（同一靶子、同一 VSIX，只换 workbench.colorTheme）——
# 尺寸不一致就说明其中一张是别的什么截的，判红（见 check_assets）。
HERO_DARK = "assets/hero-vscode-dark.png"
HERO_BUDGET = 400 * 1024
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
        self.images: list[dict] = []
        self.i18n_keys: set[str] = set()          # data-i18n / -alt / -aria 用到的键
        self.lang_choices = 0                     # topbar 的语言项个数（中/英）
        self.theme_choices = 0                    # topbar 的配色项个数（自动/深色/浅色）
        self.in_header = False                    # 是否在 <header> 里（topbar 判据要用）
        self.github_in_header = 0                 # topbar 的 GitHub 链接个数
        self.json_blocks: dict[str, str] = {}     # <script type="application/json" id=…> 的内容
        self.scripts_src: list[str] = []          # 外链 <script src>
        self.inline_head = 0                      # **可执行**内联脚本（head / body 分开数）
        self.inline_body = 0
        self.in_head = True
        self._json_id: str | None = None
        self._json_text: list[str] = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        line = self.getpos()[0]
        if tag not in VOID:
            self.stack.append((tag, line))
        if tag == "head":
            self.in_head = True
        if tag == "body":
            self.in_head = False
        if tag == "header":
            self.in_header = True
        if tag == "a" and "github.com" in (attrs.get("href") or "") and self.in_header:
            self.github_in_header += 1
        for key in ("data-i18n", "data-i18n-alt", "data-i18n-aria"):
            if key in attrs:
                self.i18n_keys.add(attrs[key])
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if "style" in attrs:
            self.inline_styles.append(line)
        for attr in ("href", "src"):
            if attr in attrs:
                self.refs.append((attr, attrs[attr], line))
        if tag == "img":
            self.images.append({"src": attrs.get("src", ""), "width": attrs.get("width"),
                                "height": attrs.get("height"), "alt": attrs.get("alt"), "line": line,
                                "hero_light": attrs.get("data-hero-light"),
                                "hero_dark": attrs.get("data-hero-dark")})
        if "data-lang-choice" in attrs:
            self.lang_choices += 1
        if "data-theme-choice" in attrs:
            self.theme_choices += 1
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
        if tag == "script":
            kind = (attrs.get("type") or "text/javascript").lower()
            if attrs.get("src"):
                self.scripts_src.append(attrs["src"])
            elif kind == "application/json" and attrs.get("id"):
                self._json_id = attrs["id"]        # 文案字典：**纯数据**，不算可执行脚本
                self._json_text = []
            elif kind in ("text/javascript", "module"):
                if self.in_head:
                    self.inline_head += 1
                else:
                    self.inline_body += 1

    def handle_endtag(self, tag):
        if tag == "header":
            self.in_header = False
        if tag == "script" and self._json_id:
            self.json_blocks[self._json_id] = "".join(self._json_text)
            self._json_id = None
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
        if self._json_id:
            self._json_text.append(data)

    def close(self):
        super().close()
        for tag, line in self.stack:
            self.errors.append(f"第 {line} 行 <{tag}> 没有闭合")


def parse_pages() -> dict[str, PageParser]:
    parsed: dict[str, PageParser] = {}
    for name in PAGES:
        path = os.path.join(SITE, name)
        if not os.path.exists(path):
            raise Failure(f"缺页面 {name}（站点是单页：site/index.html 必须在）")
        parser = PageParser()
        parser.feed(read(path))
        parser.close()
        parsed[name] = parser
    return parsed


# ── 各项断言 ───────────────────────────────────────────────────────────

def check_pages(files: list[str]) -> str:
    pages = sorted(os.path.basename(f) for f in files if f.endswith(".html"))
    if pages != sorted(PAGES):
        raise Failure(f"site/ 下应当恰好 {sorted(PAGES)}，实际：{pages}")
    internal = [rel(f) for f in files if os.sep + "_" in f]
    if internal:
        raise Failure(f"发布树里不该有内部目录（_partials 之类）：{internal}")
    return f"{len(PAGES)} 页（{', '.join(PAGES)}）"


def check_sitemap(files: list[str]) -> str:
    path = os.path.join(SITE, "sitemap.xml")
    if not os.path.exists(path):
        raise Failure("缺 site/sitemap.xml")
    locs = re.findall(r"<loc>([^<]+)</loc>", read(path))
    if not locs:
        raise Failure("sitemap.xml 里一条 <loc> 都没有")
    if len(locs) != len(set(locs)):
        raise Failure("sitemap.xml 里有重复 URL")
    # 双向：页面集合必须与 loc 集合一一对应。
    pages = {os.path.basename(f) for f in files if f.endswith(".html")}
    listed = {u.rstrip("/").rsplit("/", 1)[-1] or "index.html" for u in locs}
    listed = {name if name.endswith(".html") else "index.html" for name in listed}
    if pages != listed:
        raise Failure(f"sitemap 与真实页面不一致：页面 {sorted(pages)} vs sitemap {sorted(listed)}")
    return f"{len(locs)} 条，与页面集合双向相等"


def check_links(pages: dict[str, PageParser]) -> str:
    checked = 0
    for name, parser in pages.items():
        for attr, value, line in parser.refs:
            if value.startswith(("http://", "https://", "mailto:", "data:")):
                continue
            if value.startswith("#"):
                anchor = value[1:]
                if anchor and anchor not in parser.ids:
                    raise Failure(f"{name}:{line}：锚点 #{anchor} 在本页没有对应 id")
                checked += 1
                continue
            target = value.split("#", 1)[0].split("?", 1)[0]
            anchor = value.split("#", 1)[1] if "#" in value else ""
            if not target:
                continue
            resolved = os.path.normpath(os.path.join(SITE, target))
            if not os.path.exists(resolved):
                raise Failure(f"{name}:{line}：{attr}=\"{value}\" 指向不存在的 {target}")
            if anchor and os.path.basename(resolved) in pages:
                if anchor not in pages[os.path.basename(resolved)].ids:
                    raise Failure(f"{name}:{line}：{target}#{anchor} 的目标页里没有 id={anchor}")
            checked += 1
    return f"{checked} 条站内引用全部可解析（含跨页锚点）"


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


def check_version(files: list[str], pages: dict[str, PageParser]) -> str:
    for name, parser in pages.items():
        if not any(hook in read(os.path.join(SITE, name)) for hook in VERSION_HOOKS):
            raise Failure(f"{name} 里没有任何版本引用钩子（data-site-version 等）——版本号只能由数据回填")
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
    return f"{len(VERSION_HOOKS)} 类钩子 × {len(PAGES)} 页，0 处写死"


def check_meta(pages: dict[str, PageParser]) -> str:
    required = ["lang", "title", "description", "viewport", "og:title", "og:description"]
    for name, parser in pages.items():
        missing = [k for k in required if not parser.meta.get(k)]
        if not parser.meta.get("link:canonical"):
            missing.append("canonical")
        if not parser.meta.get("link:icon"):
            missing.append("favicon")
        if missing:
            raise Failure(f"{name} 的 head 缺元数据：{missing}")
        if parser.inline_head != 1 or parser.inline_body != 0:
            raise Failure(
                f"{name}：内联**可执行** <script> 只允许 head 里那一段配色 bootstrap"
                f"（实测 head={parser.inline_head} · body={parser.inline_body}）；"
                f"语言脚本走 assets/i18n.js、文案字典走 <script type=\"application/json\"> ✓")
    return f"{len(required) + 2} 项 × {len(PAGES)} 页齐全；内联可执行脚本各 1 段"


I18N_SCRIPT = "assets/i18n.js"
# 页面级键：不在 DOM 上（`<title>` 与 meta description 由脚本改）
I18N_PAGE_KEYS = {"title", "description"}


def check_i18n(pages: dict[str, PageParser]) -> str:
    """中英文自动识别（2026-10-09 追加需求）：字典与页面文案**逐个键对齐**。

    判据三条（都咬得住"漏译/多译/机制被删"）：
      ① 每页都引用 `assets/i18n.js`，且那个文件里**真的有**语言判定（`navigator.languages` + `zh`）；
      ② 每页的 `<script type="application/json" id="i18n-en">` 是**合法 JSON**，且带 `title`/`description`；
      ③ **DOM 里用到的 `data-i18n*` 键集合 == 字典键集合 − {title, description}** ——
         加了一句中文却忘了加英文（或多加了没人用的键）立刻判红。
    """
    script_path = os.path.join(SITE, I18N_SCRIPT)
    if not os.path.exists(script_path):
        raise Failure(f"缺 {I18N_SCRIPT}（浏览器语言检测没有落点）")
    script = read(script_path)
    for marker in ("navigator.languages", "zh", "data-i18n", "SOKO_I18N", "data-lang-choice", "soko-lang"):
        if marker not in script:
            raise Failure(f"{I18N_SCRIPT} 里找不到语言判定标记 {marker!r}（机制被删了？）")

    detail: list[str] = []
    for name, parser in pages.items():
        if I18N_SCRIPT not in parser.scripts_src:
            raise Failure(f"{name} 没有引用 {I18N_SCRIPT}（浏览器语言检测不会跑）")
        if parser.lang_choices != 2 or parser.theme_choices != 3:
            raise Failure(f"{name} 的 topbar 偏好项实测 语言 {parser.lang_choices} 个 / 配色 {parser.theme_choices} 个"
                          f"（要 2 个「中/英」+ 3 个「自动/深色/浅色」——用户 2026-10-09："
                          f"「都不要方框和图标了……和它是什么 怎么开始 教材 用一摸一样的样式」）")
        block = parser.json_blocks.get("i18n-en")
        if not block:
            raise Failure(f'{name} 缺 <script type="application/json" id="i18n-en">（英文文案字典）')
        try:
            dictionary = json.loads(block)
        except json.JSONDecodeError as error:
            raise Failure(f"{name} 的英文文案字典不是合法 JSON：{error}")
        if not dictionary.get("title") or not dictionary.get("description"):
            raise Failure(f"{name} 的字典缺 title/description（<title> 与 meta description 换不了）")
        missing = sorted(parser.i18n_keys - set(dictionary))
        extra = sorted(set(dictionary) - parser.i18n_keys - I18N_PAGE_KEYS)
        if missing or extra:
            raise Failure(f"{name} 的英文文案与页面文案对不上：缺 {missing} · 多 {extra}")
        detail.append(f"{name} {len(parser.i18n_keys)} 键")
    return "字典与 DOM 键逐个对齐（" + " · ".join(detail) + f"）；{I18N_SCRIPT} 的 zh 判定在"


def check_markup(pages: dict[str, PageParser]) -> str:
    """标签配对 + 无内联样式 + **topbar 有 GitHub 仓库链接**（用户 2026-10-09 点名）。"""
    for name, parser in pages.items():
        if parser.errors:
            raise Failure(f"{name} 标签不配对：" + "；".join(parser.errors[:4]))
        if parser.inline_styles:
            raise Failure(f"{name}：内联 style= 出现在第 {parser.inline_styles} 行（样式只能进 site.css）")
        # topbar 必须有**恰好一个** GitHub 仓库链接（用户 2026-10-09："topbar 没有 github 链接呢？"）。
        # 静态判据（不需要浏览器）；浏览器那一半在 `render` 里量同一个事实。
        if parser.github_in_header != 1:
            raise Failure(f"{name} 的 topbar 里 GitHub 链接实测 {parser.github_in_header} 个"
                          f"（要恰好 1 个：header 内一个指向 github.com 的 <a>）")
    return f"每页标签配对，无内联样式；topbar 有 1 个 GitHub 链接"


def png_size(path: str) -> tuple[int, int]:
    with open(path, "rb") as handle:
        head = handle.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        raise Failure(f"{rel(path)} 不是 PNG")
    return struct.unpack(">II", head[16:24])


def check_assets(files: list[str], pages: dict[str, PageParser]) -> str:
    bitmaps = {os.path.relpath(f, SITE): f for f in files if f.lower().endswith(BITMAP_EXT)}
    allowed = {HERO, HERO_DARK}
    unexpected = [k for k in bitmaps if k not in allowed]
    if unexpected:
        raise Failure(f"位图只允许两版头图 {HERO} / {HERO_DARK}（都必须由 scripts/site-screenshot.mjs 生成）：{unexpected}")
    missing = sorted(allowed - set(bitmaps))
    if missing:
        raise Failure(f"缺头图 {missing}（两版配色都要有：`node scripts/site-screenshot.mjs [--theme dark]`）")
    sizes = {}
    for key in (HERO, HERO_DARK):
        size = os.path.getsize(bitmaps[key])
        if size > HERO_BUDGET:
            raise Failure(f"{key} 有 {size} 字节，超过头图预算 {HERO_BUDGET}")
        sizes[key] = (size, png_size(bitmaps[key]))
    if sizes[HERO][1] != sizes[HERO_DARK][1]:
        raise Failure(f"两版头图尺寸不一致：{HERO} {sizes[HERO][1]} vs {HERO_DARK} {sizes[HERO_DARK][1]}"
                      f"（同一靶子/同一裁切几何才对，见 site-screenshot.mjs）")
    width, height = sizes[HERO][1]
    # 宽高比必须与首页 <img> 上写的 width/height 一致：否则图会被压扁，而这一点
    # 光看页面是看不出来的（"数据对了 ≠ 用户看见对了"）。
    img = next((i for i in pages["index.html"].images if (i.get("hero_light") or i["src"]).endswith("hero-vscode.png")), None)
    if img is None:
        raise Failure(f"{HERO} 在 index.html 里没有被引用")
    if not img["width"] or not img["height"]:
        raise Failure("头图 <img> 必须写 width/height（否则首屏会跳版）")
    want = float(img["width"]) / float(img["height"])
    got = width / height
    if abs(want - got) / got > 0.01:
        raise Failure(f"头图宽高比 {got:.3f}（{width}×{height}）与 <img width/height> 的 {want:.3f} 不一致")
    if not (img["alt"] or "").strip():
        raise Failure("头图必须有 alt")
    # 两版都要被页面**指到**：`site.js` 靠这两个属性换 src，漏一个那版就是死图。
    if os.path.relpath(SITE + "/" + (img.get("hero_light") or ""), SITE) not in allowed:
        raise Failure(f"<img> 的 data-hero-light 指向 {img.get('hero_light')!r}，不是 {HERO}")
    if img.get("hero_dark") != HERO_DARK:
        raise Failure(f"<img> 的 data-hero-dark 必须指向 {HERO_DARK}（site.js 靠它换暗色图），实测 {img.get('hero_dark')!r}")
    # 暗色头图必须**预加载**（用户 2026-10-09：「转深色模式的时候，图片变化好慢」）：
    # 少了它，切主题要等 210 KB 下载完才换图 ✗。⚠ 这条是**补的** —— 上一次改动里
    # 那句 `str.replace` 锚点缩进不匹配 ⇒ **静默没落地**，而当时没有任何判据咬它 ✗。
    index_html = read(os.path.join(SITE, "index.html"))
    if not re.search(r'<link[^>]*rel="preload"[^>]*as="image"[^>]*href="%s"' % re.escape(HERO_DARK), index_html):
        raise Failure(f"index.html 没有预加载 {HERO_DARK}（要一条 `<link rel=preload as=image ...>`）"
                      f"—— 切深色会等下载完才换图 ✗")
    light_kb = sizes[HERO][0] // 1024
    dark_kb = sizes[HERO_DARK][0] // 1024
    detail = (f"头图两版 {light_kb}+{dark_kb} KB（各 ≤{HERO_BUDGET // 1024} KB，{width}×{height}，"
              f"尺寸一致）")
    total = sum(os.path.getsize(f) for f in files if f.endswith((".html", ".css", ".js")))
    if total > SIZE_BUDGET:
        raise Failure(f"html+css+js 共 {total} 字节，超过预算 {SIZE_BUDGET}")
    return f"{detail}；html+css+js {total} 字节（预算 {SIZE_BUDGET}）"


def run_script(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run([sys.executable] + args, cwd=REPO_ROOT, capture_output=True, text=True)


def check_data() -> str:
    proc = run_script([os.path.join(REPO_ROOT, "scripts", "gen-site-data.py"), "--check"])
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


def _dump_dom(chrome: str, url: str, budget: int = 5000, timeout: int = 30, accept_lang: str | None = None) -> str:
    """跑一次**旧版** headless Chrome，返回 DOM。

    为什么用旧版 headless：macOS 上 `--headless=new` + `--dump-dom` 会挂住不返回（实测 120s）。
    为什么用 `--virtual-time-budget` 而不是 `--timeout`：后者会在 `fetch("data/site.json")`
    落定**之前**就把 DOM 倒出来，于是"版本号已回填"时红时绿（实测 4 次命中 vs 0 次）。
    为什么不等进程退出：macOS 上 Chrome 打完完整 DOM 之后**进程不退出** ⇒ 判据是"DOM 到齐
    了没有"（见到 `</html>` 就杀），不是"进程结束了没有"。
    """
    import shutil
    import tempfile
    import threading
    import time

    profile = tempfile.mkdtemp(prefix="soko-chrome-")
    try:
        proc = subprocess.Popen([
            chrome, "--headless", "--disable-gpu", "--no-sandbox",
            "--no-first-run", "--no-default-browser-check",
            "--disable-extensions", "--disable-background-networking",
            "--disable-sync", "--hide-scrollbars",
            # `--accept-lang` 会改 `navigator.languages`（实测：en-US ⇒ ["en-US"]）——
            # 于是"中文读者看中文 / 其它语言看英文"这条判据能在真浏览器里判 ✓。
            *([f"--accept-lang={accept_lang}"] if accept_lang else []),
            f"--user-data-dir={profile}", f"--virtual-time-budget={budget}",
            "--dump-dom", url,
        ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        chunks: list[str] = []

        def pump():
            for line in proc.stdout or ():
                chunks.append(line)

        threading.Thread(target=pump, daemon=True).start()
        deadline = time.time() + timeout
        while time.time() < deadline:
            if "</html>" in "".join(chunks) or proc.poll() is not None:
                break
            time.sleep(0.1)
        dom = "".join(chunks)
        if proc.poll() is None:
            proc.kill()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass
        return dom
    finally:
        shutil.rmtree(profile, ignore_errors=True)


# 布局探针：在**真浏览器**里把每页按几个宽度装进 iframe，量 `scrollWidth` 与视口宽度。
# 为什么非要真浏览器：横向溢出的成因在排版（`100vw` 计算、不可断行的长标识符、负 margin
# 悬挑），静态检查看不见 —— 实测就是这么抓到两个（CHANGELOG 里一条 836px 的测试路径把
# 整页撑宽、`.wide` 的 margin-inline 被组件自己的 margin 简写覆盖）。
LAYOUT_WIDTHS = [360, 768, 1024, 1440]


def _layout_probe(widths: list[int]) -> str:
    return f"""<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>layout</title></head>
<body><pre id="out">pending</pre>
<script>
(async () => {{
  const pages = {json.dumps(PAGES)}, widths = {json.dumps(widths)}, out = [];
  for (const page of pages) for (const w of widths) {{
    const f = document.createElement('iframe');
    f.style.cssText = 'width:' + w + 'px;height:900px;border:0';
    f.src = page;
    document.body.appendChild(f);
    await new Promise((r) => {{ f.onload = r; }});
    // ⚠ **等字体落定再量**：站点是自托管字体，回退字体与正文字体的度量不同 ⇒
    // 不等就可能量到"还没换字体"的宽度（实测 360px 上差 6px，判据会时红时绿 ✗）。
    try {{ await f.contentWindow.document.fonts.ready; }} catch (e) {{}}
    await new Promise((r) => setTimeout(r, 80));
    const doc = f.contentDocument, win = f.contentWindow;
    const h1 = doc.querySelector('h1');
    const cs = h1 ? win.getComputedStyle(h1) : null;
    const lh = cs ? (parseFloat(cs.lineHeight) || parseFloat(cs.fontSize) * 1.2) : 0;
    out.push({{ page: page, width: w, inner: win.innerWidth,
               scrollW: doc.documentElement.scrollWidth,
               lang: doc.documentElement.lang,
               h1Lines: h1 && lh ? Math.max(1, Math.round(h1.getBoundingClientRect().height / lh)) : null }});
    f.remove();
  }}
  document.getElementById('out').textContent = JSON.stringify(out);
}})();
</script></body></html>"""


def _toggle_probe() -> str:
    """语言切换按钮的**用户动作**判据：真 Chrome 里真点一下（用户 2026-10-09 点名要按钮）。

    判据必须是"点下去 ⇒ 可见结果"（AGENTS.md 验证纪律 0(a)）：`<html lang>`、
    nav 文案、按钮自己的标签、以及 localStorage 里记住的选择，四样都要跟着变；
    再点一下必须**回得去**（切回中文靠 `i18n.js` 抓下来的原文，抓漏了就回不去 ✗）。
    """
    return """<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>toggle</title>
<link rel="icon" href="data:,"><!-- 探针页不请求 /favicon.ico（否则会被记成渲染期 404 ✗）-->
</head>
<body><pre id="out">pending</pre>
<script>
(async () => {
  const out = {};
  try {
    const f = document.createElement('iframe');
    f.style.cssText = 'width:1200px;height:900px;border:0';
    f.src = 'index.html';
    document.body.appendChild(f);
    await new Promise((r) => { f.onload = r; });
    await new Promise((r) => setTimeout(r, 250));
    const d = f.contentDocument, win = f.contentWindow;
    const langBtns = Array.from(d.querySelectorAll('[data-lang-choice]'));
    const themeBtns = Array.from(d.querySelectorAll('[data-theme-choice]'));
    const nav = () => ((d.querySelector('nav a[data-i18n="navWhat"]') || {}).textContent || '').trim();
    const pressed = (arr, attr) => arr.filter((b) => b.getAttribute('aria-pressed') === 'true')
      .map((b) => b.getAttribute(attr));
    const snap = () => ({ lang: d.documentElement.lang, nav: nav(),
                          theme: d.documentElement.getAttribute('data-theme'),
                          langPressed: pressed(langBtns, 'data-lang-choice'),
                          themePressed: pressed(themeBtns, 'data-theme-choice'),
                          prefsVisible: Array.from(d.querySelectorAll('.prefs')).every((u) => u.getBoundingClientRect().height > 0),
                          stored: (() => { try { return win.localStorage.getItem('soko-lang'); } catch (e) { return 'ERR'; } })() });
    out.before = snap();
    if (langBtns.length !== 2 || themeBtns.length !== 3) {
      throw new Error('偏好项个数不对：语言 ' + langBtns.length + '（要 2：中/英）· 配色 ' + themeBtns.length + '（要 3：自动/深色/浅色）');
    }
    const clickChoice = (attr, value) => {
      const b = d.querySelector('[' + attr + '="' + value + '"]');
      if (!b) throw new Error('找不到 ' + attr + '=' + value);
      b.click();
    };
    clickChoice('data-lang-choice', 'en');
    await new Promise((r) => setTimeout(r, 150));
    out.after = snap();
    // ⚠ 用户报的是**英文**那版折行（"英文版的 topbar 都不能在一行放下"）⇒ 切到英文后必须再量一次。
    const rowsOf = () => {
      const hw = d.querySelector('.site-header .wrap');
      if (!hw) return null;
      const rs = Array.from(hw.children).filter((k) => k.getBoundingClientRect().height > 0)
        .map((k) => k.getBoundingClientRect());
      const overlap = Math.min(...rs.map((r) => r.bottom)) - Math.max(...rs.map((r) => r.top));
      return { rows: overlap > 0 ? 1 : 2, w: Math.round(hw.getBoundingClientRect().width),
               detail: rs.map((r) => `${Math.round(r.top)}-${Math.round(r.bottom)}`).join(' ') };
    };
    out.headerEn = rowsOf();
    clickChoice('data-lang-choice', 'zh');
    await new Promise((r) => setTimeout(r, 150));
    out.back = snap();
    // 主题按钮：点到暗色 ⇒ **头图必须换成暗色那版**（用户 2026-10-09 的"黑白两版"）。
    const img = d.querySelector('img[data-hero-light]');
    out.hero = { light: img && img.getAttribute('src'), theme: null, dark: null,
                 expectLight: img && img.getAttribute('data-hero-light'),
                 expectDark: img && img.getAttribute('data-hero-dark') };
    // 控件必须**在一条水平线上**（用户 2026-10-09：「两个方框都不在一条水平线上」）——
    // 量的是**顶边**：差 >1px 就是错位（图标按钮没有文字基线，靠 .header-controls 兜住）。
    // 三个控件要**等高 + 同中心线**（用户 2026-10-09：「按钮高度搞统一，按钮中横线对齐」）。
    // ⚠ 只量顶边是不够的：pill 比方框高 1.8px 时顶边只差 0.89px ⇒ 旧判据放它过去 ✗。
    const geom = (sel) => {
      const e = d.querySelector(sel);
      if (!e) return null;
      const r = e.getBoundingClientRect();
      return { top: +r.top.toFixed(2), h: +r.height.toFixed(2), center: +((r.top + r.bottom) / 2).toFixed(2) };
    };
    // ⚠ **文字也要量**（用户 2026-10-09：「标题等文字，和三个方框感觉不在一个水平线上」）——
    // 只量三个方框会漏掉"品牌/导航按文字基线落位"这条（实测中心线差 5.75px ✗）。
    // ── 通用几何审计：**别再靠人眼/手算**（用户 2026-10-09：「要手动计算吗？这么不智能」）──
    // 把 header/footer 里每一项的顶边/高度/中心线/宽度全量出来，判据在 Python 侧逐条判，
    // 失败时**直接把数字打出来** ⇒ 以后任何一项错位都由判据说话，不需要人去量。
    const rowGeom = (sel) => {
      const e = d.querySelector(sel);
      if (!e) return null;
      return Array.from(e.children).filter((k) => k.getBoundingClientRect().height > 0).map((k) => {
        const r = k.getBoundingClientRect();
        return { what: (k.className && String(k.className).split(/[ ]+/)[0]) || k.tagName.toLowerCase(),
                 top: +r.top.toFixed(2), h: +r.height.toFixed(2), w: +r.width.toFixed(2),
                 bottom: +r.bottom.toFixed(2), center: +((r.top + r.bottom) / 2).toFixed(2) };
      });
    };
    out.rows = { header: rowGeom('.site-header .wrap'), footer: rowGeom('.site-footer .wrap') };
    // ── 文字基线（用户 2026-10-09：「top bar 左右两边的文字中心线也不在同一横线」）──
    // ⚠ 判"文字是否在同一条横线"**不能用盒子的 center**：字号不同时盒子中心对齐 ≠ 文字对齐 ✗。
    // 插一个零宽 inline-block 探针，它的**底边**就落在该行的基线上 ✓ —— 这是唯一稳的量法。
    // ⚠ 量法：用 Range 取该元素里**第一段文字**的行盒底边。
    // 不要往元素里插零宽探针 —— `nav ul` / `.prefs` 是 flex 容器，插进去它就变成
    // **flex item**、按 flex 规则落位（实测量出 18/35/66.8 三条"基线"，全是假的 ✗）。
    // Range 走的是文字自身的行盒，与 flex 布局无关 ✓；不同字号的下伸部略有差别（~0.5px），
    // 所以判据容差取 2px ✓。
    const baselineOf = (el) => {
      const walker = d.createTreeWalker(el, NodeFilter.SHOW_TEXT);
      let node = walker.nextNode();
      while (node && !node.textContent.trim()) node = walker.nextNode();   // 跳过缩进空白
      if (!node) return null;
      const r = d.createRange();
      r.setStart(node, 0);
      r.setEnd(node, node.textContent.length);
      const rects = r.getClientRects();
      return rects.length ? +rects[0].bottom.toFixed(2) : null;
    };
    const rowBaselines = (sel) => {
      const e = d.querySelector(sel);
      if (!e) return null;
      return Array.from(e.children).filter((k) => k.getBoundingClientRect().height > 0).map((k) => ({
        what: (k.className && String(k.className).split(/[ ]+/)[0]) || k.tagName.toLowerCase(),
        baseline: baselineOf(k), h: +k.getBoundingClientRect().height.toFixed(2) }));
    };
    out.baselines = { header: rowBaselines('.site-header .wrap'), footer: rowBaselines('.site-footer .wrap') };
    // 偏好项**必须可见**（用户 2026-10-09 报「中英文和主题切换的没掉了」——那是 `hidden` 起步的锅）
    out.prefsVisible = Array.from(d.querySelectorAll('.prefs')).map((u) => u.getBoundingClientRect().height > 0);
    // 可点目标的命中尺寸（WCAG 2.2 AA 2.5.8 要求 ≥ 24×24 CSS px）
    out.targets = Array.from(d.querySelectorAll('.site-header a, .site-header button, .site-footer a, .site-footer button'))
      .map((e) => { const r = e.getBoundingClientRect();
        return { t: (e.textContent || '').trim().slice(0, 14) || (e.getAttribute('aria-label') || '?'),
                 w: +r.width.toFixed(1), h: +r.height.toFixed(1) }; });
    // topbar 必须**放得下一行**（用户 2026-10-09：「英文版的 topbar 都不能在一行放下」）：
    // 量 header 里三个子项（品牌 / 导航 / 控件组）的顶边有几行 —— >1 就是折行了 ✗。
    const hw = d.querySelector('.site-header .wrap');
    out.headerRows = (() => {
      if (!hw) return null;
      const rs = Array.from(hw.children).filter((k) => k.getBoundingClientRect().height > 0)
        .map((k) => k.getBoundingClientRect());
      if (rs.length < 2) return { rows: 1, detail: 'n/a' };
      // ⚠ **一行 = 竖直区间互相重叠**，不是"顶边相同"：头部是 `align-items: baseline`，
      // 同一行里品牌/导航/控件的顶边本来就不同 ⇒ 用顶边会把一行误判成三行（踩过 ✗）。
      const overlap = Math.min(...rs.map((r) => r.bottom)) - Math.max(...rs.map((r) => r.top));
      return { rows: overlap > 0 ? 1 : 2,
               detail: rs.map((r) => `${Math.round(r.top)}-${Math.round(r.bottom)}`).join(' ') };
    })();
    // GitHub 是**导航项**（用户 2026-10-09：「就在『教材』后面加一项」）⇒ 在 header 里数它。
    out.headerGithub = (() => { const h = d.querySelector('header'); return h ? h.querySelectorAll('a[href*="github.com"]').length : null; })();
    clickChoice('data-theme-choice', 'dark');
    await new Promise((r) => setTimeout(r, 150));
    out.hero.theme = d.documentElement.getAttribute('data-theme');
    out.hero.dark = img && img.getAttribute('src');
    out.themePressedAfter = pressed(themeBtns, 'data-theme-choice');
  } catch (e) { out.error = String((e && e.message) || e); }
  document.getElementById('out').textContent = JSON.stringify(out);
})();
</script></body></html>"""


class _SiteServer:
    """只读站点服务器（HTTP/1.0 + 多线程）+ 一条 `/__layout__` 探针路由。

    ⚠ **不要**改成 HTTP/1.1：keep-alive 会让 Chrome 认为还有未完成的网络活动，
    `--virtual-time-budget` 于是永远走不完 ⇒ 挂死。默认 HTTP/1.0（响应完即关连接）
    才能让虚拟时间正常推进。**必须多线程**：Chrome 会开一条不说话的预连接，单线程
    服务器一旦轮询到它就会卡在 `readline()` 上，把后面真正的请求全挡住。
    """

    def __init__(self) -> None:
        import http.server
        import threading

        self.missing: list[str] = []
        outer = self

        class Handler(http.server.SimpleHTTPRequestHandler):
            def __init__(self, *a, **kw):
                super().__init__(*a, directory=SITE, **kw)

            def log_message(self, fmt, *a):
                pass

            def send_error(self, code, message=None, explain=None):
                outer.missing.append(self.path)
                super().send_error(code, message, explain)

            def do_GET(self):
                if self.path.startswith("/__toggle__"):
                    body = _toggle_probe().encode("utf-8")
                    self.send_response(200)
                    self.send_header("content-type", "text/html; charset=utf-8")
                    self.send_header("content-length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                    return
                if self.path.startswith("/__layout__"):
                    body = _layout_probe(LAYOUT_WIDTHS).encode("utf-8")
                    self.send_response(200)
                    self.send_header("content-type", "text/html; charset=utf-8")
                    self.send_header("content-length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                    return
                super().do_GET()

        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.httpd.daemon_threads = True
        self.port = self.httpd.server_address[1]
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()

    def url(self, path: str) -> str:
        return f"http://127.0.0.1:{self.port}/{path}"

    def shutdown(self) -> None:
        self.httpd.shutdown()


def check_render() -> str:
    """真 Chrome 跑**两页**：资源零 404，且版本号被 JS 回填进 DOM。"""
    chrome = find_chrome()
    if not chrome:
        raise Failure("找不到 Chrome（--browser 需要它）")
    try:
        version = json.loads(read(DATA)).get("version", "")
    except (OSError, json.JSONDecodeError):
        version = ""
    if not version:
        raise Failure("site/data/site.json 里没有 version，无法验证回填")

    server = _SiteServer()
    try:
        for name in PAGES:
            # 第 5 个元素 = **运行期**标签（site.js 从 `window.SOKO_I18N` 取，不是静态文案）——
            # 这条判据钉的是 i18n.js ↔ site.js 的**接缝**：静态文案换了、按钮没换 = 半页英文 ✗。
            # 复制按钮只在首页（changelog 页没有命令块）⇒ 运行期标签按页取。
            copy_zh = (">复制<",) if name == "index.html" else ()
            copy_en = (">Copy<",) if name == "index.html" else ()
            for lang, shown, hidden, html_lang, runtime in (
                # 运行期由脚本画的字（配色三项的标签 + 复制按钮）也必须跟上语言。
                ("zh-CN", ">它是什么<", ">What it is<", 'lang="zh-CN"',
                 (">自动<", ">深色<", ">浅色<") + copy_zh),
                ("en-US", ">What it is<", ">它是什么<", 'lang="en"',
                 (">Auto<", ">Dark<", ">Light<") + copy_en),
            ):
                dom = _dump_dom(chrome, server.url(name), accept_lang=lang)
                if not dom.strip():
                    raise Failure(f"{name}（{lang}）：Chrome 没有输出 DOM（进程可能起不来）")
                if f">{version}<" not in dom.replace(" ", ""):
                    raise Failure(f"{name}（{lang}）：DOM 里没找到回填后的版本号 {version}（site.js 没跑或被缓存）")
                if html_lang not in dom:
                    raise Failure(f"{name}（{lang}）：<html> 的 lang 没切到 {html_lang}（i18n.js 没跑？）")
                # 判据必须是**换过之后的元素文本**（`>…<`），不能只搜字符串：英文字典本身
                # 就在 DOM 里（application/json 块），搜裸字符串会永远为真 ⇒ 守卫空转 ✗。
                if shown not in dom:
                    raise Failure(f"{name}（{lang}）：页面文案没切到该语言（找不到 {shown}）")
                if hidden in dom:
                    raise Failure(f"{name}（{lang}）：另一种语言的原文案还在（不该出现 {hidden}）")
                # topbar 语言按钮：标签写**当前语言**（中文页「中」、英文页「EN」），
                # 而且必须**已被 i18n.js 放出来**（HTML 里 `hidden` 起步 ⇒ 没跑就是隐藏的 ✗）。
                if re.search(r'<ul class="prefs"[^>]*hidden', dom):
                    raise Failure(f"{name}（{lang}）：偏好项还是 hidden —— 脚本没把它放出来？")
                want = "zh" if lang == "zh-CN" else "en"
                got_btn = re.search(r'<button[^>]*data-lang-choice="%s"[^>]*>' % want, dom)
                if not got_btn or 'aria-pressed="true"' not in got_btn.group(0):
                    raise Failure(f"{name}（{lang}）：当前语言那一项（{want}）没被标为当前档："
                                  f"{got_btn.group(0) if got_btn else '找不到该项'}")
                other = "en" if want == "zh" else "zh"
                other_btn = re.search(r'<button[^>]*data-lang-choice="%s"[^>]*>' % other, dom)
                if other_btn and 'aria-pressed="true"' in other_btn.group(0):
                    raise Failure(f"{name}（{lang}）：非当前语言那一项（{other}）也被标成了当前档 ✗")
                for marker in runtime:
                    if marker not in dom:
                        raise Failure(f"{name}（{lang}）：运行期标签没跟上语言（找不到 {marker}）")
        # ── topbar 语言按钮：**真点一下**（点下去 ⇒ 可见结果；再点回去也要成立）──
        import html as html_mod
        dom = _dump_dom(chrome, server.url("__toggle__"), budget=15000, timeout=60, accept_lang="zh-CN")
        found = re.search(r'<pre id="out">(.*?)</pre>', dom, re.S)
        if not found or found.group(1).strip() in ("", "pending"):
            raise Failure("语言按钮探针没跑完（DOM 里没有结果）—— 探针坏了，不判绿")
        got = json.loads(html_mod.unescape(found.group(1)))
        if got.get("error"):
            raise Failure(f"语言按钮探针报错：{got['error']}")
        before, after, back = got["before"], got["after"], got["back"]
        if (before["lang"] != "zh-CN" or before["nav"] != "它是什么" or before["langPressed"] != ["zh"]
                or not before["prefsVisible"] or before["stored"] is not None):
            raise Failure(f"中文读者的初态不对：{before}（应为 lang=zh-CN · 导航中文 ·「中」是当前档 · "
                          f"偏好项已放出 · 还没记住过选择）")
        if after["lang"] != "en" or after["nav"] != "What it is" or after["langPressed"] != ["en"] or after["stored"] != "en":
            raise Failure(f"点「英」没切到英文（可见结果不对）：{after}")
        if back["lang"] != "zh-CN" or back["nav"] != "它是什么" or back["langPressed"] != ["zh"] or back["stored"] != "zh":
            raise Failure(f"点「中」没切回中文（原文没抓全？）：{back}")
        hero = got.get("hero") or {}
        if hero.get("light") != HERO:
            raise Failure(f"亮色下头图应是 {HERO}，实测 {hero.get('light')!r}")
        if hero.get("theme") != "dark" or hero.get("dark") != HERO_DARK:
            raise Failure(f"主题点到暗色后头图没换（要 {HERO_DARK}）：{hero}")
        for label, rows in (("中文", got.get("headerRows")), ("英文", got.get("headerEn"))):
            rows = rows or {}
            if rows.get("rows") != 1:
                raise Failure(f"{label} topbar 折行了（{rows}）—— 英文自然宽 ~750px / 中文 653px，"
                              f"头部必须比正文栏宽一档（`.site-header .wrap` 的 max-width）✗")
        # ── 通用几何审计（每一行 × 每一项，失败时把数字全打出来）──
        bl = got.get("baselines") or {}
        for where, items in (("topbar", bl.get("header")), ("页脚", bl.get("footer"))):
            if not items:
                raise Failure(f"{where} 量不到任何项（选择器坏了？）—— 判据不判绿")
            bs = [it["baseline"] for it in items]
            spread = max(bs) - min(bs)
            if spread > 2.0:
                detail = " · ".join(f'{it["what"]}(基线 {it["baseline"]})' for it in items)
                raise Failure(f"{where} 各项的**文字基线**不在同一条横线上（最大差 {spread:.2f}px）：{detail} —— "
                              f"纯文字行必须 `align-items: baseline`；用 `center` 对齐的是**盒子**，"
                              f"字号不同（16px 品牌 vs 14px 导航）就会错开 ✗")
        if not all(got.get("prefsVisible") or []):
            raise Failure(f"偏好项（配色/语言）不可见 —— 用户 2026-10-09 报过「中英文和主题切换的没掉了」"
                          f"（实测 {got.get('prefsVisible')}）")
        # 可点目标 ≥ 24×24（WCAG 2.2 AA 2.5.8 Target Size (Minimum)）
        small = [t for t in (got.get("targets") or []) if t["w"] < 24 or t["h"] < 24]
        if small:
            raise Failure("这些可点目标小于 24×24 CSS px（WCAG 2.2 AA 2.5.8）："
                          + " · ".join(f'{t["t"]} {t["w"]}×{t["h"]}' for t in small))
        if server.missing:
            raise Failure(f"渲染时有 404：{sorted(set(server.missing))[:5]}")
    finally:
        server.shutdown()
    return (f"Chrome 渲染 {len(PAGES)} 页 × 2 种语言通过（中文读者看中文 · 其它语言看英文），"
            f"语言项真点过（中→英→中，含 localStorage 记忆）· 配色项真点过（点「深色」⇒ 头图换暗色那版 + 当前档标对）· "
            f"topbar 中英各一行 · **topbar/页脚逐项几何审计通过**（文字同一条基线 · 偏好项可见 · 命中尺寸 ≥24px），"
            f"版本 {version} 已回填，资源零 404")


def check_layout() -> str:
    """真 Chrome 量**横向溢出** + **中文 hero 标题不折行**：每页 × 几个宽度。

    ⚠ 跑**两种浏览器语言**：溢出两种都要判（英文更长的词更容易撑破），而"标题单行"
    只对**中文**判 —— 英文标题 `sokonanoda · a formal proof teaching language` 本来就长，
    折两行是正常的（用户 2026-10-09 报的是中文页："标题都是两行"）。
    """
    import html as html_mod

    chrome = find_chrome()
    if not chrome:
        raise Failure("找不到 Chrome（--browser 需要它）")
    server = _SiteServer()
    try:
        rows_by_lang = {}
        for lang in ("zh-CN", "en-US"):
            dom = _dump_dom(chrome, server.url("__layout__"), budget=20000, timeout=60, accept_lang=lang)
            found = re.search(r'<pre id="out">(.*?)</pre>', dom, re.S)
            if not found or found.group(1).strip() in ("", "pending"):
                raise Failure(f"布局探针没跑完（{lang} 的 DOM 里没有结果）—— 探针本身坏了，不判绿")
            rows_by_lang[lang] = json.loads(html_mod.unescape(found.group(1)))
    finally:
        server.shutdown()
    bad = [r for rows in rows_by_lang.values() for r in rows if r["scrollW"] > r["inner"] + 1]
    if bad:
        raise Failure("这些宽度下页面横向溢出："
                      + "、".join(f'{r["page"]}@{r["width"]}（内容 {r["scrollW"]}px > 视口 {r["inner"]}px）' for r in bad))
    # 中文 hero 标题**必须单行**（2026-10-09 用户实测："正文太窄，标题都是两行" ⇒
    # `--measure` 40rem → 46rem）。只判 1024 以上：360/768 是窄屏，折行是对的 ✓。
    wrapped = [r for r in rows_by_lang["zh-CN"]
               if r["page"] == "index.html" and r["width"] >= 1024 and r.get("h1Lines") != 1]
    if wrapped:
        raise Failure("中文 hero 标题在宽屏上折行了（用户实测过的那条）："
                      + "、".join(f'{r["page"]}@{r["width"]} = {r["h1Lines"]} 行' for r in wrapped))
    return (f"{len(PAGES)} 页 × {len(LAYOUT_WIDTHS)} 个宽度（{LAYOUT_WIDTHS}）× 2 种语言零横向溢出；"
            f"中文 hero 标题在 ≥1024 单行")


# ── 主流程 ─────────────────────────────────────────────────────────────

CHECKS = [
    ("pages", lambda files, pages: check_pages(files)),
    ("sitemap", lambda files, pages: check_sitemap(files)),
    ("links", lambda files, pages: check_links(pages)),
    ("css-urls", lambda files, pages: check_css_urls(files)),
    ("version", lambda files, pages: check_version(files, pages)),
    ("meta", lambda files, pages: check_meta(pages)),
    ("i18n", lambda files, pages: check_i18n(pages)),
    ("markup", lambda files, pages: check_markup(pages)),
    ("assets", lambda files, pages: check_assets(files, pages)),
    ("data", lambda files, pages: check_data()),
]


def main(argv: list[str] | None = None) -> int:
    parser_args = argparse.ArgumentParser(description="站点总验收（单页站点）")
    parser_args.add_argument("--browser", action="store_true", help="额外跑真 Chrome 渲染检查")
    parser_args.add_argument("--json", dest="as_json", action="store_true", help="机器可读输出")
    args = parser_args.parse_args(argv)

    if not os.path.exists(INDEX):
        print(f"site: 找不到 {rel(INDEX)}", file=sys.stderr)
        return 3

    files = walk_site()
    try:
        pages = parse_pages()
    except Failure as error:
        print(f"site: {error}", file=sys.stderr)
        return 1

    results: list[dict] = []
    failed = False
    for name, fn in CHECKS:
        try:
            results.append({"check": name, "ok": True, "detail": fn(files, pages)})
        except Failure as error:
            results.append({"check": name, "ok": False, "detail": str(error)})
            failed = True

    if args.browser:
        for name, fn in (("render", check_render), ("layout", check_layout)):
            try:
                results.append({"check": name, "ok": True, "detail": fn()})
            except Failure as error:
                results.append({"check": name, "ok": False, "detail": str(error)})
                failed = True

    if args.as_json:
        print(json.dumps({"ok": not failed, "checks": results}, ensure_ascii=False, indent=2))
    else:
        for row in results:
            print(f"  {'✓' if row['ok'] else '✗'} {row['check']:<10} {row['detail']}")
        total = len(results)
        green = sum(1 for row in results if row["ok"])
        print(f"site: {'ok' if not failed else 'FAILED'}（{green}/{total}）")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
