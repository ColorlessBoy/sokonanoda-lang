#!/usr/bin/env python3
"""Total acceptance for `site/` — completeness **and** correctness, in one command.

The per-page tools (`check-site.py`, `site-audit.py`, `gen-site-nav.py`) each guard
one axis and are run by the authors as they work. This script is the *release*
check: it answers the two questions a reviewer actually asks, and it answers them
about the whole site rather than about one page.

  **Completeness** — is anything missing?
    C1  every page on the D2 site map exists (and no page exists that is not on it)
    C2  `sitemap.xml` and the real page set agree, both directions
    C3  no orphans: every page is reachable from the shared footer sitemap
    C4  every page carries the nav/footer markers and a `data-page`
    C5  every page carries the required head metadata

  **Correctness** — is what is there true and sound?
    K1  no hardcoded version numbers anywhere
    K2  no bare hex in markup; every `var(--x)` in CSS is defined
    K3  no horizontal overflow at 1440 / 900 / 390 (real viewports)
    K4  no semantic colour that failed to resolve (a token typo renders silently transparent)
    K13 dark mode ("夜读") actually resolves — measured by setting data-theme="dark"
        on the live page, which is the path the theme toggle takes
    K15 the interactive features actually work — every other check here is
        static (read source, measure layout); none of them clicks anything
    K5  every in-site link resolves
    K6  no banned AI-tell patterns (centred hero, eyebrow labels, arrow-suffixed
        links, emoji icons, uniform radius, inline styles, bitmaps)
    K7  every page's tags balance
    K8  accessibility basics: exactly one h1, no skipped heading levels, labelled
        inputs, `lang` on `<html>`, a skip link
    K9  budgets: per-page HTML, CSS, JS, fonts

Exit codes: 0 = everything passed · 1 = at least one check failed · 2 = usage error.

Usage:
  python3 scripts/site-verify.py                 # everything
  python3 scripts/site-verify.py --quick         # skip the browser checks (K3/K4)
  python3 scripts/site-verify.py --json          # machine-readable summary
"""

from __future__ import annotations

import argparse
import html
import html.parser
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
D2 = ROOT / "docs" / "design" / "site-rebuild" / "spec" / "D2-information-architecture.md"

# 站点地图（D2 §1 与 §1.1 的表格）就是"该有哪些页"的权威。从文档里读，
# 不在这里再抄一份——抄一份就多一个漂移源。
# 行首的编号列可能是 `A1`/`G1`，也可能是 `—`（样板页与站点文件那几行），
# 所以这里不限定字母数字。
PLAN_ROW_RE = re.compile(r"^\|\s*[A-Z0-9—]+\s*\|\s*`([a-z0-9/-]+\.html)`", re.M)
PLAN_G_RE = re.compile(r"^\|\s*G\d+\s*\|\s*`([a-z0-9/-]+\.html)`", re.M)

# 站点文件（不是 HTML 页面，但要存在）
EXTRA_FILES = ("favicon.svg", "robots.txt", "sitemap.xml", "llms.txt")

# ── K6：被点名的 AI 长相。每条都必须**可机械判定**，否则就是口号。 ──────────
BANNED_PATTERNS: list[tuple[str, str, str]] = [
    ("centred-hero", r'\.hero[^{}]*\{[^}]*text-align:\s*center', "居中 hero"),
    ("eyebrow", r'class="[^"]*\beyebrow\b', "标题上方的 eyebrow 标签"),
    ("arrow-link", r'<a\b[^>]*>[^<]*→\s*</a>', "链接文本以 → 结尾"),
    ("emoji-icon", r"[\U0001F300-\U0001FAFF\u2600-\u27BF]", "emoji 当图标"),
    ("inline-style", r'\sstyle="', "内联 style"),
    ("bitmap", r'<img\b|\.(?:png|jpe?g|gif|webp)\b', "位图"),
    ("letter-spacing-upper", r'text-transform:\s*uppercase', "全大写 + 字距的标签"),
]

# 允许出现的例外：每条都要写理由，空字典是目标。
BANNED_ALLOW: dict[str, set[str]] = {
    # `→` 在正文里作为**数学记法**出现是合法的（本语言真的用 → 表示箭头类型），
    # 被禁的是把它当装饰尾巴挂在链接文本后面。正则只抓后者。
    "arrow-link": set(),
}

# ── K8：可访问性基础 ────────────────────────────────────────────────────────
REQUIRED_HEAD = (
    ('<meta name="description"', "meta description"),
    ('rel="canonical"', "canonical"),
    ('name="theme-color"', "theme-color"),
    ('property="og:title"', "og:title"),
    ('property="og:description"', "og:description"),
    ('property="og:type"', "og:type"),
)


class TagBalance(html.parser.HTMLParser):
    """Unclosed / mismatched tags, and a heading outline."""

    VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input",
            "link", "meta", "param", "source", "track", "wbr"}

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.stack: list[str] = []
        self.errors: list[str] = []
        self.headings: list[tuple[int, str]] = []
        self._capture: int | None = None
        self._buf: list[str] = []
        self.inputs: list[dict] = []
        self._in_label = 0

    def handle_starttag(self, tag: str, attrs) -> None:
        a = dict(attrs)
        if tag in {"input", "select", "textarea"}:
            self.inputs.append(
                {
                    "type": a.get("type", "text"),
                    "id": a.get("id", ""),
                    "aria-label": a.get("aria-label", ""),
                    "labelled": self._in_label > 0,
                }
            )
        if tag == "label":
            self._in_label += 1
        if tag in self.VOID:
            return
        if tag in {"h1", "h2", "h3", "h4"}:
            self._capture = int(tag[1])
            self._buf = []
        self.stack.append(tag)

    def handle_endtag(self, tag: str) -> None:
        if tag == "label":
            self._in_label = max(0, self._in_label - 1)
        if tag in {"h1", "h2", "h3", "h4"} and self._capture == int(tag[1]):
            text = re.sub(r"\s+", " ", "".join(self._buf)).strip()
            self.headings.append((self._capture, text))
            self._capture = None
        if tag in self.VOID:
            return
        if not self.stack:
            self.errors.append(f"stray </{tag}>")
            return
        if self.stack[-1] == tag:
            self.stack.pop()
        else:
            self.errors.append(f"</{tag}> closes <{self.stack[-1]}>")

    def handle_data(self, data: str) -> None:
        if self._capture:
            self._buf.append(data)


def pages() -> list[Path]:
    return sorted(p for p in SITE.rglob("*.html") if "_partials" not in p.parts)


def rel(path: Path) -> str:
    return path.relative_to(SITE).as_posix()


def planned_pages() -> set[str]:
    text = D2.read_text(encoding="utf-8")
    return set(PLAN_ROW_RE.findall(text)) | set(PLAN_G_RE.findall(text))


# ---------------------------------------------------------------------------
# checks
# ---------------------------------------------------------------------------


def check_completeness() -> tuple[list[str], str]:
    problems: list[str] = []
    built = {rel(p) for p in pages()}
    plan = planned_pages()
    if not plan:
        return ["could not read the site map from D2 (the plan is the authority)"], "no plan"

    for missing in sorted(plan - built):
        problems.append(f"planned page `{missing}` does not exist")
    for extra in sorted(built - plan - {"en/index.html"}):
        problems.append(f"page `{extra}` exists but is not on the D2 site map")
    for name in EXTRA_FILES:
        if not (SITE / name).is_file():
            problems.append(f"site file `{name}` is missing")

    # 孤儿检查：每个页面都要能从共享页脚（或页头）到达。页脚承担完整站点地图，
    # 所以一条从页脚出发的链接就够。
    footer = (SITE / "_partials" / "footer.html")
    reachable: set[str] = set()
    if footer.is_file():
        for href in re.findall(r'href="([^"]+)"', footer.read_text(encoding="utf-8")):
            if href.startswith(("http", "#", "mailto:")):
                continue
            reachable.add(href)
    # index 由页头品牌链接到达；404 是错误页，**故意**不进页脚站点地图
    # （它没有可索引的 URL，进站点地图反而是错的）。
    for page in sorted(built):
        if page in reachable or page in {"index.html", "404.html"}:
            continue
        problems.append(f"orphan: `{page}` is not linked from the footer sitemap")

    return problems, f"{len(built)} built / {len(plan)} planned, {len(reachable)} footer links"


def check_markers() -> tuple[list[str], str]:
    problems: list[str] = []
    for path in pages():
        src = path.read_text(encoding="utf-8")
        r = rel(path)
        for marker in ("<!--#nav-->", "<!--/#nav-->", "<!--#footer-->", "<!--/#footer-->"):
            if marker not in src:
                problems.append(f"{r}: missing marker {marker}")
        if "data-page=" not in src:
            problems.append(f"{r}: <body> has no data-page")
    return problems, f"{len(pages())} page(s)"


def check_head() -> tuple[list[str], str]:
    problems: list[str] = []
    for path in pages():
        src = path.read_text(encoding="utf-8")
        r = rel(path)
        if "<title>" not in src:
            problems.append(f"{r}: no <title>")
        for needle, label in REQUIRED_HEAD:
            if needle not in src:
                problems.append(f"{r}: missing {label}")
    return problems, f"{len(pages())} page(s) × {len(REQUIRED_HEAD) + 1} items"


# 允许出现版本字面量的页面，每条都要写理由。规则的本意是**站点绝不宣称一个
# 过期的"当前版本"**（当前版本一律由 data-site-version 在浏览时填）。
# 引用历史版本作为**数据**不是宣称：缺口台账原文就记着每条缺口关账于哪个版本，
# 把它换成占位符反而会让页面谎报台账写了什么。
VERSION_ALLOW: dict[str, str] = {
    "engineering.html": "引用缺口台账原文与一条真实台账记录（关账版本是数据，不是当前版本）",
}


def check_no_hardcoded_version() -> tuple[list[str], str]:
    pattern = re.compile(r"(?<![\d.])0\.\d+\.\d+(?![\d.])")
    problems: list[str] = []
    allowed = 0
    for path in pages():
        r = rel(path)
        if r in VERSION_ALLOW:
            allowed += 1
            continue
        for match in pattern.finditer(path.read_text(encoding="utf-8")):
            problems.append(f"{r}: hardcoded version `{match.group(0)}`")
    note = f"{len(pages())} page(s) scanned"
    if allowed:
        note += f", {allowed} allowlisted with a reason"
    return problems, note


def check_css_tokens() -> tuple[list[str], str]:
    """Every `var(--x)` used in a stylesheet must be defined in a stylesheet.

    This is the check that would have caught the `--mark*` bug: CSS custom
    properties fail **silently**, so a typo yields `transparent` and no error.
    """
    assets = SITE / "assets"
    defined: set[str] = set()
    sheets = sorted(assets.glob("*.css"))
    for sheet in sheets:
        src = re.sub(r"/\*.*?\*/", "", sheet.read_text(encoding="utf-8"), flags=re.S)
        defined |= set(re.findall(r"(--[a-z0-9-]+)\s*:", src))

    problems: list[str] = []
    for sheet in sheets:
        src = re.sub(r"/\*.*?\*/", "", sheet.read_text(encoding="utf-8"), flags=re.S)
        for name in sorted(set(re.findall(r"var\((--[a-z0-9-]+)", src))):
            if name not in defined:
                problems.append(f"{sheet.name}: var({name}) is never defined")

    # 裸 hex 只允许出现在令牌文件与 SVG 里。
    for sheet in sheets:
        if sheet.name in {"tokens.css", "fonts.css", "styleguide.css"}:
            continue
        src = re.sub(r"/\*.*?\*/", "", sheet.read_text(encoding="utf-8"), flags=re.S)
        for match in re.finditer(r"#[0-9a-fA-F]{3,8}\b", src):
            problems.append(f"{sheet.name}: bare hex `{match.group(0)}` outside tokens")
    return problems, f"{len(sheets)} stylesheet(s), {len(defined)} token(s) defined"


def check_banned_patterns() -> tuple[list[str], str]:
    problems: list[str] = []
    hits = 0
    for path in pages():
        src = path.read_text(encoding="utf-8")
        r = rel(path)
        for name, pattern, label in BANNED_PATTERNS:
            if r in BANNED_ALLOW.get(name, set()):
                continue
            found = re.findall(pattern, src)
            if found:
                hits += len(found)
                sample = found[0] if isinstance(found[0], str) else found[0][0]
                problems.append(f"{r}: {label} ({name}) ×{len(found)} — e.g. {sample[:40]!r}")
    return problems, f"{len(pages())} page(s) × {len(BANNED_PATTERNS)} pattern(s), {hits} hit(s)"


def check_html_and_a11y() -> tuple[list[str], str]:
    problems: list[str] = []
    for path in pages():
        src = path.read_text(encoding="utf-8")
        r = rel(path)
        parser = TagBalance()
        try:
            parser.feed(src)
            parser.close()
        except Exception as exc:
            problems.append(f"{r}: parse error {exc!r}")
            continue
        for err in parser.errors[:3]:
            problems.append(f"{r}: {err}")
        if parser.stack:
            problems.append(f"{r}: unclosed <{', '.join(parser.stack[:4])}>")

        h1s = [h for h in parser.headings if h[0] == 1]
        if len(h1s) != 1:
            problems.append(f"{r}: {len(h1s)} <h1> element(s), expected exactly 1")
        levels = [h[0] for h in parser.headings]
        for prev, cur in zip(levels, levels[1:]):
            if cur > prev + 1:
                problems.append(f"{r}: heading level jumps h{prev} → h{cur}")
                break
        if 'class="skip-link"' not in src:
            problems.append(f"{r}: no skip link")
        if not re.search(r'<html[^>]+lang="', src):
            problems.append(f"{r}: <html> has no lang")
        for inp in parser.inputs:
            if inp["type"] in {"hidden", "submit", "button"}:
                continue
            has_label = inp["labelled"] or inp["aria-label"] or (
                inp["id"] and f'for="{inp["id"]}"' in src
            )
            if not has_label:
                problems.append(f"{r}: <input type={inp['type']}> has no label")
    return problems, f"{len(pages())} page(s)"


def check_data_fidelity() -> tuple[list[str], str]:
    """**零伪造**：页面渲染的每一条录制数据，都必须真的在生成它的 JSON 里。

    这是本站最重要的一条正确性检查。站点上所有"内核输出"都是预计算的，
    所以它们要么逐字来自 `site/data/*.json`，要么就是编的——没有第三种可能。
    光靠人读查不出来（编的也长得很像），必须机器比对。

    做法：从页面里抽出被当作"真实数据"的片段，回对应数据文件里找。
    找不到 = 页面写了一个数据文件里没有的值 = 伪造或数据已过期。
    """
    problems: list[str] = []

    CHECKS: list[tuple[str, str, list[tuple[str, str]], str]] = [
        ("walkthrough.html", "walkthrough.json",
         [(r'<pre class="goal-ty">(.*?)</pre>', "目标文本")],
         "走查页的每个目标"),
        # 选择器按**真实标记**写，不是按我以为的标记写——第一版三个里错了两个，
        # 报的是"抽不到"，而不是页面有问题。
        ("diagnostics.html", "diagnostics.json",
         [(r'<span class="dc">([a-z][a-z0-9-]+)</span>', "诊断码")],
         "诊断字典的每个码"),
        ("kernel.html", "events.json",
         [(r'<span class="event-type[^"]*">(.*?)</span>', "事件类型")],
         "内核页的每种事件"),
    ]

    def strip_tags(text: str) -> str:
        return html.unescape(re.sub(r"<[^>]+>", "", text)).strip()

    checked = 0
    for page_name, data_name, extractors, label in CHECKS:
        page = SITE / page_name
        data = SITE / "data" / data_name
        if not page.is_file() or not data.is_file():
            problems.append(f"{label}: 缺页面或缺数据文件（{page_name} / {data_name}）")
            continue
        blob = data.read_text(encoding="utf-8")
        src = page.read_text(encoding="utf-8")
        for pattern, what in extractors:
            found = re.findall(pattern, src, re.S)
            if not found:
                problems.append(f"{page_name}: 抽不到任何{what}（选择器变了？）")
                continue
            for raw in found:
                value = strip_tags(raw).strip()
                if not value:
                    continue
                checked += 1
                # 数据文件里是 JSON 转义过的，两种形态都试。
                if value in blob or json.dumps(value, ensure_ascii=False)[1:-1] in blob:
                    continue
                problems.append(
                    f"{page_name}: {what} `{value[:60]}` 不在 {data_name} 里 —— 伪造或数据已过期"
                )

    # 第二类：**散文里引用的数据**。首页的编辑器插画把 playground 的实测计数当数据
    # 引用（`decl_checked: 30`、`exercise_open: 4`），而 K16 量的是 `evidence.json` 里的
    # `decl.checked`——名字一个下划线、一个点，所以上面那张"逐字复制"的表盖不住它，
    # 而这两个数**每次动 playground 都会变**。
    #
    # 只做这一处，不做通用的"页面引用数据"检查：`infix: 50 " ∈ " => Set.mem` 这类记法
    # 语法、文件路径、URL 都会撞上同一个正则，通用版只能靠一张手工豁免表活着——那种
    # 检查会烂掉，而烂掉的检查比没有检查更坏。真正的通用做法是把这类值挪进数据文件、
    # 由 JS 回填（像版本号那样），那是页面改版的事。
    #
    # 顺带记一次真实的教训：`counts_source: previous-run` 曾以同样的形状写在
    # `index.html` 上，而生成器在有已发布二进制的机器上会实测成 `gate` —— 页面于是
    # 成了一句谎话，**没有任何检查会响**。那处已改成描述机制、不再引用取值。
    index_page = SITE / "index.html"
    evidence = SITE / "data" / "evidence.json"
    quotes = re.findall(
        r"<code>(decl_checked|example_checked|exercise_open|warning): (\d+)</code>",
        index_page.read_text(encoding="utf-8") if index_page.is_file() else "",
    )
    if not quotes:
        problems.append("index.html: 抽不到 playground 计数（选择器变了？）")
    else:
        try:
            events = json.loads(evidence.read_text(encoding="utf-8")) \
                ["measured"]["playground_events"]["value"]
        except (OSError, json.JSONDecodeError, KeyError, TypeError):
            events = None
        if not isinstance(events, dict):
            problems.append("evidence.json: 读不到 measured.playground_events.value")
        else:
            for key, number in quotes:
                checked += 1
                name = key.replace("_", ".")
                if events.get(name) != int(number):
                    problems.append(
                        f"index.html: playground 计数 {key}: {number} 与 evidence.json 的 "
                        f"{name}: {events.get(name)} 不一致 —— 伪造或数据已过期"
                    )
    return problems, f"{checked} 条录制值回查数据文件"


def check_citation_density() -> tuple[list[str], str]:
    """展示内核输出的页面，必须说清那份输出是**哪条命令**产生的。

    这是"每个主张都能点开看到真实证据"的可判定部分。
    """
    # 判据是"页面里出现了产生这份输出的命令"，而不是某一种固定写法——
    # 站点同时用 `scripts/soko …` 与 `soko query …` 两种拼法，都算数。
    NEEDLES = ("scripts/soko", "soko grade", "soko query", "query check", "query state")
    DATA_PAGES = (
        "walkthrough.html", "kernel.html", "diagnostics.html",
        "index.html", "agents.html", "get-started.html",
    )
    problems: list[str] = []
    for name in DATA_PAGES:
        path = SITE / name
        if not path.is_file():
            continue
        src = path.read_text(encoding="utf-8")
        if not any(n in src for n in NEEDLES):
            problems.append(
                f"{name}: 展示了内核输出，却没有出现产生它的命令"
                f"（应有 {' / '.join(NEEDLES)} 之一）"
            )
    return problems, f"{len(DATA_PAGES)} 个数据页"


def release_ref(version: str) -> str:
    """The git ref the site's facts must be reproducible from: the release tag.

    `HEAD` is the wrong anchor. The site describes a **released** version, and
    between releases HEAD carries work the released binary cannot even parse
    (measured: the 0.61.0 binary reports zero declarations on the post-0.61.0
    canvas). Anchoring on the tag makes the judgement mean what it says:
    "can these numbers be reproduced from the tree as it was released?"
    Returns "" when there is no such tag, so callers can skip honestly.
    """
    for ref in (f"v{version}", version):
        probe = subprocess.run(["git", "rev-parse", "--verify", f"{ref}^{{commit}}"],
                               cwd=ROOT, capture_output=True, text=True, check=False)
        if probe.returncode == 0:
            return ref
    return ""


def load_generator():
    """Import `scripts/gen-site-data.py` — the one definition of "released version".

    The file name has a dash, so it cannot be a normal import; loading it by path
    means K17 judges the artifact against **the same function that produced it**
    instead of a second regex that can drift from it (the reason `check-site.py`
    imports `gen-site-nav.py` rather than re-deriving the nav rule).
    """
    path = ROOT / "scripts" / "gen-site-data.py"
    spec = importlib.util.spec_from_file_location("gen_site_data", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules["gen_site_data"] = module
    previous = sys.dont_write_bytecode
    sys.dont_write_bytecode = True  # never drop __pycache__ into scripts/
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = previous
    return module


def check_version_is_released() -> tuple[list[str], str]:
    """`site.json` 的 `version` 必须是**一个已发布 tag** —— 且不能是"下一个版本"。

    为什么非有不可：站点的 28 个页脚、`about` 的统计卡、每条下载指令都从这个数
    拼出来，而 `compare.html` 明说它是「已发布的版本，不是工作树」。生成器已经按
    tag 取数（`gen-site-data.py::release_version`），但**只有这条判据能挡住回归**：

    版本若写成 `Cargo.toml` 的那个（未发布），K12/K16 会**安静跳过**——它们按
    `site.json` 的版本去找已发布二进制和 tag，找不到就报"跳过：不算通过"。跳过的
    措辞和平时一模一样，于是站点可以带着一个**下载不到的版本**上线，而报告全绿。
    2026-09-21 实测过这条路：语言线把 `Cargo.toml` 推到 `0.62.0`（无 tag、无产物）
    后，照原样重跑生成器写出的正是 `version: 0.62.0`。

    **"谎话"与"落后"要分开**（2026-09-21，合入 main 之前收紧过一次）：

    * **谎话（判红）**：站点写的那个版本**根本没有 tag** —— 读者按页脚的版本去下载
      会 404。这是上面那条实测出来的病，必须拦住。
    * **落后（只记一笔）**：站点写的版本**有 tag**，只是比最新的旧（发布刚落地、
      站点数据还没重跑）。这不是假话——那个版本真的能下载——而且此刻 `pages.yml`
      正在部署，判红会让**整条部署挂掉**、把一个"该重跑生成器"的待办伪装成故障。
      宁可绿着部署一份落后一个版本的站点，也不要红着不部署：前者是旧的真话，
      后者是新的空白。

    找不到任何 tag 时**跳过并说明**（浅克隆、非 git 检出），不假装通过。
    """
    data = SITE / "data" / "site.json"
    if not data.is_file():
        return ["site/data/site.json 缺失"], "no data"
    try:
        published = json.loads(data.read_text(encoding="utf-8")).get("version", "")
    except json.JSONDecodeError as exc:
        return [f"site/data/site.json 不是 JSON：{exc!r}"], "unreadable"

    try:
        generator = load_generator()
        released, tag = generator.release_version()
    except Exception as exc:  # a crashed import is a failure, never a skip
        return [f"scripts/gen-site-data.py 导不进来：{exc!r}"], "import failed"
    if not released:
        return [], "跳过：仓库里找不到 vX.Y.Z 发布 tag（不算通过）"

    if published == released:
        return [], f"v{published}"

    # 站点写的版本自己有 tag 吗？有 ⇒ 它可下载，只是落后。
    if release_ref(published):
        return [], (f"v{published} 落后于最新 tag {tag} —— 老的真话，不是假话；"
                    f"发布后按 STATE §12.2 重跑生成器")
    hint = ""
    next_version = generator.get_version()
    if next_version and published == next_version:
        hint = (f"。这正是把它写成 Cargo.toml 的后果：那是**下一个**版本，"
                f"发行窗口里一直领先于最后一个 tag")
    return [f"站点写的版本是 {published!r}，而它**不是任何发布 tag** —— 读者按它下载会 404"
            f"（最新发布 tag 是 {tag}）{hint}"], f"not released（{published}）"


def check_course_counts() -> tuple[list[str], str]:
    """站点发布的课程计数，必须能在**干净检出 + 已发布二进制**上复现。

    为什么值得单独一条：这是本站唯一一类"数字来自一次测量、而不是来自仓库里的
    某个文件"的内容。工作树脏的时候（本项目长期有并行开发），现场重跑会得到
    不同的数——`courses/` 被改过就会。所以判据不是"现场跑一遍对不对"，而是
    **发布 tag 的课程 + 发布的二进制**能不能跑出页面写的数。

    做法：把 tag 的课程解到临时目录，用钉住的二进制跑课程门禁，与
    `site/data/site.json` 的 `set_theory.totals` 逐项比对。找不到钉住的二进制、
    或找不到对应的发布 tag 时**跳过并说明**，不假装通过。
    """
    data = SITE / "data" / "site.json"
    if not data.is_file():
        return ["site/data/site.json 缺失"], "no data"
    try:
        published = json.loads(data.read_text(encoding="utf-8"))["set_theory"]["totals"]
    except (json.JSONDecodeError, KeyError) as exc:
        return [f"site.json 里读不到 set_theory.totals：{exc!r}"], "unreadable"

    # 已发布二进制：从 VS Code 扩展目录里找与仓库版本一致的那个。
    version = json.loads(data.read_text(encoding="utf-8")).get("version", "")
    home = Path.home() / ".vscode" / "extensions"
    candidates = sorted(home.glob(f"sokonanoda-lang.sokonanoda-{version}-*/bin/*/sokonanoda"))
    if not candidates:
        return [], f"跳过：找不到 {version} 的已发布二进制（不算通过）"

    ref = release_ref(version)
    if not ref:
        return [], f"跳过：找不到 {version} 的发布 tag（不算通过）"

    work = ROOT / ".cache" / "verify-clean"
    subprocess.run(["rm", "-rf", str(work)], check=False)
    work.mkdir(parents=True, exist_ok=True)
    # 只解课程是不够的：门禁要沿目录向上找 `scripts/soko` 才认「本检出」，
    # 再从该目录的 `Cargo.toml` 读版本钉，与二进制 `--version` 比对，不一致就
    # **exit 2 拒绝判卷**（它是对的——那双眼睛分不出"已发布二进制"和"走错门的
    # 工作树"）。少解这两样，临时目录会向上撞到本仓库根、拿 HEAD 的版本钉去卡
    # 已发布二进制，把一次本来正确的复现判成红。三样一起解出来，临时目录才是
    # 一个自洽的**已发布检出**：钉 = 该 tag 的版本，判据才真的在说"能不能复现"。
    members = ["courses/set-theory", "scripts/soko", "Cargo.toml"]
    tar = subprocess.run(["git", "archive", ref, *members],
                         cwd=ROOT, capture_output=True, check=False)
    if tar.returncode != 0:
        return [f"git archive {ref} {' '.join(members)} 失败"], "git failed"
    subprocess.run(["tar", "-x", "-C", str(work)], input=tar.stdout, check=False)

    gate = work / "courses" / "set-theory" / "tools" / "check.py"
    if not gate.is_file():
        return [f"{ref} 的课程门禁不在预期路径"], "no gate"
    env = dict(**__import__("os").environ, SOKONANODA_BIN=str(candidates[0]))
    proc = subprocess.run([sys.executable, str(gate), "--json"],
                          capture_output=True, text=True, timeout=600, check=False, env=env)

    try:
        summary = json.loads(proc.stdout)["summary"]
    except (json.JSONDecodeError, KeyError):
        tail = (proc.stderr or proc.stdout or "").strip().splitlines()
        why = f"：{tail[-1]}" if tail else ""
        return [f"课程门禁没有输出可解析的 summary（exit {proc.returncode}{why}）"], "no summary"

    problems: list[str] = []
    for key, published_key in (("targets", "targets"), ("checked", "checked"),
                               ("open", "open"), ("rejected", "failed")):
        got = summary.get(key)
        want = published.get(published_key)
        if got != want:
            problems.append(
                f"课程计数 {published_key}: 站点写 {want}，{ref}+发布二进制实测 {got}"
            )
    return problems, (f"{ref} + {version} 实测 "
                      f"{summary.get('targets')}/{summary.get('checked')}/"
                      f"{summary.get('open')}/{summary.get('rejected')}")


# 语义色只允许出现在**有真实判定背书**的元素上。这条规则写在 tokens.css 的文件头、
# 也写在 D1 与站点自己的验收清单里，但在此之前**没有机器判据**——于是它被违反了
# 整整一轮（全站每个链接都是绿的，悬停变朱）。一个写在文档里却没人检查的规则
# 等于没有规则，所以这里把它变成断言。
SEMANTIC_FORBIDDEN = (
    # (正则, 说明) —— 命中的选择器块里不许出现 var(--acc*) / var(--verm*)
    (r"^a\s*\{[^}]*\}", "链接"),
    (r"^a:hover[^{]*\{[^}]*\}", "链接悬停"),
    (r"^a:visited\s*\{[^}]*\}", "已访问链接"),
    (r"^\.btn--primary\s*\{[^}]*\}", "主按钮"),
    (r"^\.btn--quiet\s*\{[^}]*\}", "静默按钮"),
    (r"^\.nav-group a\[aria-current[^\]]*\]\s*\{[^}]*\}", "导航当前页"),
    (r"^\.toc a\[aria-current[^\]]*\]\s*\{[^}]*\}", "目录当前节"),
    (r"^\.tabs__tab\[aria-selected[^\]]*\]\s*\{[^}]*\}", "标签页选中"),
    (r"^\.footer-nav a:hover\s*\{[^}]*\}", "页脚链接悬停"),
    (r"^::selection\s*\{[^}]*\}", "选区"),
    (r"^em\s*\{[^}]*\}", "着重号"),
    (r"^\.search-hit mark\s*\{[^}]*\}", "搜索命中高亮"),
)


# 某些令牌**本身**就不该由语义色定义。焦点环是典型：它是可访问性 affordance，
# 与"内核说通过"无关。这条是被 a11y 探针抓出来的——渲染后焦点环是绿的
# （`--ring-color: var(--acc)`），而所有静态检查都看不见它。
TOKEN_MUST_BE_NEUTRAL = {
    "--ring-color": "焦点环是可访问性 affordance，不是内核判定",
    "--shadow-scroll": "表格横滚提示是版式手法，不是判定",
}


def check_semantic_colors() -> tuple[list[str], str]:
    """`--acc*` / `--verm*` 只允许出现在判定与诊断上。"""
    problems: list[str] = []
    tokens = SITE / "assets" / "tokens.css"
    if tokens.is_file():
        src = re.sub(r"/\*.*?\*/", "", tokens.read_text(encoding="utf-8"), flags=re.S)
        for name, why in TOKEN_MUST_BE_NEUTRAL.items():
            m = re.search(re.escape(name) + r"\s*:\s*([^;]+);", src)
            if m and ("var(--acc" in m.group(1) or "var(--verm" in m.group(1)):
                problems.append(f"tokens.css: {name} 由语义色定义 —— {why}")
    checked = 0
    for sheet in sorted((SITE / "assets").glob("*.css")):
        if sheet.name in {"tokens.css", "styleguide.css"}:
            continue  # 令牌定义处与色卡处当然会出现
        src = re.sub(r"/\*.*?\*/", "", sheet.read_text(encoding="utf-8"), flags=re.S)
        for pattern, label in SEMANTIC_FORBIDDEN:
            for match in re.finditer(pattern, src, re.M):
                checked += 1
                block = match.group(0)
                if "var(--acc" in block or "var(--verm" in block:
                    problems.append(
                        f"{sheet.name}: {label} 用了语义色 —— "
                        "绿只给内核判定、朱只给诊断与限制，两者都不适用于它"
                    )
    return problems, f"{checked} 个非判定元素的选择器已检查"


def check_playground_counts() -> tuple[list[str], str]:
    """站点发布的 playground 计数，必须能在**发布 tag 的画布 + 已发布二进制**上复现。

    与 K12（课程计数）同一个道理，但踩的是另一条腿：playground 是仓库里**会随
    语言一起改**的文件。开发中的下一批改动把它从「自建 7 条 And/Or 公理」改成了
    内建记法，计数因此从 30/2/4/2 变成 23/2/4/1——只有用**发布的那把二进制**量
    **该 tag 的那份画布**，才得到站点该写的数。

    这也解释了为什么不能拿工作树现场跑：工作树的画布可能用的是未发布语法，
    发布版根本解析不了它（实测：0.61.0 在当前工作树的 playground 上只出 1 条
    diagnostic，因为那份画布已经是下一代的）。
    """
    data = SITE / "data" / "evidence.json"
    if not data.is_file():
        return ["site/data/evidence.json 缺失"], "no data"
    try:
        published = json.loads(data.read_text(encoding="utf-8"))["measured"]["playground_events"]["value"]
    except (json.JSONDecodeError, KeyError) as exc:
        return [f"evidence.json 里读不到 playground_events：{exc!r}"], "unreadable"

    site_json = SITE / "data" / "site.json"
    version = ""
    if site_json.is_file():
        version = json.loads(site_json.read_text(encoding="utf-8")).get("version", "")
    home = Path.home() / ".vscode" / "extensions"
    candidates = sorted(home.glob(f"sokonanoda-lang.sokonanoda-{version}-*/bin/*/sokonanoda"))
    if not candidates:
        return [], f"跳过：找不到 {version} 的已发布二进制（不算通过）"

    ref = release_ref(version)
    if not ref:
        return [], f"跳过：找不到 {version} 的发布 tag（不算通过）"

    work = ROOT / ".cache" / "verify-playground"
    subprocess.run(["rm", "-rf", str(work)], check=False)
    work.mkdir(parents=True, exist_ok=True)
    blob = subprocess.run(["git", "show", f"{ref}:playground.sokonanoda"],
                          cwd=ROOT, capture_output=True, check=False)
    if blob.returncode != 0:
        return [f"git show {ref}:playground.sokonanoda 失败"], "git failed"
    canvas = work / "playground.sokonanoda"
    canvas.write_bytes(blob.stdout)

    proc = subprocess.run([str(candidates[0]), str(canvas), "--json"],
                          capture_output=True, text=True, timeout=180, check=False)
    counts: dict[str, int] = {}
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line).get("type")
        except json.JSONDecodeError:
            continue
        if ev:
            counts[ev] = counts.get(ev, 0) + 1

    problems: list[str] = []
    for key in ("decl.checked", "example.checked", "exercise.open", "warning"):
        got, want = counts.get(key, 0), published.get(key)
        if want != got:
            problems.append(
                f"playground 计数 {key}: 站点写 {want}，{ref} 画布 + {version} 实测 {got}"
            )
    detail = " / ".join(f"{k} {counts.get(k, 0)}" for k in
                        ("decl.checked", "example.checked", "exercise.open", "warning"))
    return problems, f"{ref} + {version}: {detail}"


def run_tool(args: list[str]) -> tuple[bool, str]:
    try:
        proc = subprocess.run(
            [sys.executable, *args], cwd=ROOT, capture_output=True, text=True,
            timeout=600, check=False,
        )
    except subprocess.TimeoutExpired:
        return False, "timed out"
    tail = [ln for ln in (proc.stdout or "").strip().splitlines() if ln.strip()]
    return proc.returncode == 0, (tail[-1][:110] if tail else f"exit {proc.returncode}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--quick", action="store_true", help="skip browser checks")
    parser.add_argument("--json", action="store_true", help="machine-readable summary")
    args = parser.parse_args()

    results: list[tuple[str, str, list[str], str]] = []

    def add(group: str, name: str, fn) -> None:
        problems, note = fn()
        results.append((group, name, problems, note))

    add("完整性", "C1 页面清单", check_completeness)
    # C2（sitemap 双向）与 K5 是同一个断言，由 check-site.py 报告，不重复算。
    # C3（无孤儿）已并入 C1：C1 会逐个页面检查它能否从页脚站点地图到达。
    add("完整性", "C4 导航标记", check_markers)
    add("完整性", "C5 头部元数据", check_head)

    add("正确性", "K1 无写死版本", check_no_hardcoded_version)
    add("正确性", "K2 CSS 令牌", check_css_tokens)
    add("正确性", "K6 AI 味模式", check_banned_patterns)
    add("正确性", "K7 标签配平", check_html_and_a11y)
    add("正确性", "K10 零伪造（数据保真）", check_data_fidelity)
    add("正确性", "K11 出处可查", check_citation_density)
    add("正确性", "K12 课程计数可复现", check_course_counts)
    add("正确性", "K14 语义色纪律", check_semantic_colors)
    add("正确性", "K16 playground 计数可复现", check_playground_counts)
    add("正确性", "K17 版本是发布 tag", check_version_is_released)

    # 子工具：它们各自已是完整的检查，这里只汇总结果。
    for label, cmd in (
        ("K5 链接与预算", ["scripts/check-site.py"]),
        ("C4 导航一致性", ["scripts/gen-site-nav.py", "--check"]),
        ("搜索索引新鲜度", ["scripts/gen-site-search.py", "--check"]),
    ):
        ok, note = run_tool(cmd)
        results.append(("正确性", label, [] if ok else [f"{cmd[0]} reported problems"], note))

    if not args.quick:
        ok, note = run_tool(["scripts/site-audit.py"])
        results.append(("正确性", "K3/K4/K13 渲染审计（含暗色）",
                        [] if ok else ["overflow, unresolved colour, or broken dark mode"], note))
        # K15 是唯一**会点击**的检查：其余全部是静态的（读源码、量布局）。
        # 没有它，站点上每一行 JavaScript 都可以是坏的而全绿通过——搜索功能
        # 就是这么发布出去的：审计过溢出、审计过配色，却从未被执行过一次。
        ok, note = run_tool(["scripts/site-functest.py"])
        results.append(("正确性", "K15 交互功能实跑",
                        [] if ok else ["a feature did not behave as expected"], note))

    # C1 已经算了页面清单，这里把 sitemap 双向一致性交给 check-site.py（K5 行），
    # 所以 C2/C3 只作说明，不重复报错。

    if args.json:
        print(json.dumps(
            [{"group": g, "check": n, "ok": not p, "problems": p, "note": note}
             for g, n, p, note in results],
            ensure_ascii=False, indent=1))
    else:
        total_fail = 0
        group_now = None
        for group, name, problems, note in results:
            if group != group_now:
                print(f"\n── {group} ──")
                group_now = group
            mark = "FAIL" if problems else "ok  "
            if problems:
                total_fail += 1
            print(f"  {mark} {name:<20} {note}")
            for problem in problems[:12]:
                print(f"        - {problem}")
            if len(problems) > 12:
                print(f"        … 另有 {len(problems) - 12} 条")

        checks = len(results)
        print(f"\n{'=' * 62}")
        if total_fail:
            print(f"总验收：{checks - total_fail}/{checks} 项通过，{total_fail} 项未通过")
        else:
            print(f"总验收：{checks}/{checks} 项全部通过")

    failed = any(p for _, _, p, _ in results)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
