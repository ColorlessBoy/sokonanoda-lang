#!/usr/bin/env python3
"""Site hygiene checks (run in CI before deploying to GitHub Pages).

Ten assertions, all born from this repo's documentation-drift history
(docs/design/site-rebuild/spec/D2 §3.3; the superseded rules are in
docs/design/site.md §5.3):

1. **links** — every in-site `href` / `src` target must exist on disk, and
   every repo-relative `docs/...` / `course/...` link must point at a real
   file (mirrors `crates/cli/tests/skill.rs`'s
   `skill_referenced_repo_paths_exist`).
2. **versions** — `site/**/*.html` must not contain a literal `0.x.y`
   version. Version numbers come from `site/data/site.json` and the GitHub
   Releases API at view time — hardcoding them is how `README.md` ended up
   advertising `V=0.9.0` while the repo was at 0.17.0.
3. **nav-drift** — each page's `<!--#nav-->` / `<!--#footer-->` blocks equal
   `site/_partials/{header,footer}.html` (after the aria-current
   normalisation). The comparison rule lives in `scripts/gen-site-nav.py` and
   is *imported*, never re-derived: one implementation, one truth.
4. **data-page** — every page declares `<body data-page="…">`, that id names a
   real nav link, and it owns the page's single `aria-current="page"`.
5. **metadata** — exactly one `<h1>`, non-empty `<title>`,
   `<meta name="description">`, `<link rel="canonical">`,
   `<meta name="theme-color">`, `og:title` / `og:description` / `og:type`.
6. **asset-budget** — every local stylesheet / script a page references
   exists, and the measured totals respect D2 §3.4.
7. **no-bitmaps** — no `<img>` that is not an SVG, no `.png`/`.jpg`/`.gif`
   reference in HTML (the old site shipped PIL-rendered fake screenshots).
8. **no-inline-style** — no `style=` attributes (token bypass).
9. **turnstile** — every page carries the `.turnstile` device, or is listed
   in an explicit exemption set with a reason.
10. **sitemap** — `sitemap.xml` and the pages on disk agree in *both*
    directions: no page missing from the sitemap, no sitemap entry pointing at
    a page that does not exist (`en/` maps to `en/index.html`, not the root
    `index.html`).

Every assertion reports `file: reason` and fails independently; each prints a
summary line with counts (and measured byte totals for the budget). Python
stdlib only, so the site pipeline stays zero-build.

Usage: `python3 scripts/check-site.py [--site DIR]`.
Exit codes: 0 clean, 1 problems, 2 usage error.
"""

from __future__ import annotations

import argparse
import gzip
import html.parser
import json
import importlib.util
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_SITE = Path(__file__).resolve().parent.parent / "site"
VERSION_RE = re.compile(r"(?<![\d.])0\.\d+\.\d+(?![\d.])")
# Any `scheme:` prefix (https:, mailto:, vscode: …) — not a repo-relative link.
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
# `_partials/` holds the single source for the nav/footer blocks, not a page:
# it is never published and its links are site-root-relative by convention.
PAGE_SKIP_DIRS = {"_partials"}
BITMAP_RE = re.compile(r"[\w./~%+-]+\.(?:png|jpe?g|gif|webp|avif)\b", re.IGNORECASE)

# Budgets, bytes (D2 §3.4).
#
# CSS 有两条：raw 防悄悄膨胀，gzip 才是读者实际付的代价。真实数字来自
# S0 阶段的实测——四份样式表合计 ~96 KB raw / ~30 KB gzip，其中约四成是
# 注释（注释解释了"为什么"，而且 gzip 压得掉）。把它压到 45 KB raw 的唯一
# 办法是删注释，那是把设计理由换成体积，不划算；所以 raw 上限按实测定，
# gzip 上限卡在 32 KB（相当于一张小图，且跨页缓存）。
HTML_BUDGET = 90 * 1024
HTML_GZIP_BUDGET = 26 * 1024
# CSS 的预算按**实测基线 + ~15% 余量**定，不是按整数定的。定这条时四份样式表
# 是 100.1 KB raw / 31.3 KB gzip（tokens 19.3 + base 13.2 + site 29.5 +
# editor 38.1），其中约四成是注释——注释解释"为什么"，而且 gzip 压得掉。
# 预算的用途是**拦住回归**，不是逼作者删理由去凑一个整数。
# 实测基线（7 份样式表，含 diagnostics.css 与 styleguide.css）：
#   114.8 KB raw / 36.8 KB gzip。预算给 ~10% 余量拦回归。
# 早先只数了四份文件，所以这个数一直偏低——**写死的文件清单会漂**，
# 现在 CSS_BUDGET_FILES 是 glob 出来的。
CSS_BUDGET = 128 * 1024
CSS_GZIP_BUDGET = 41 * 1024
JS_BUDGET = 12 * 1024
FONTS_BUDGET = 120 * 1024
# 预算覆盖**全部**页面可加载的样式表。早先这里写死了四个文件名，结果
# `diagnostics.css`（诊断页专用，子 agent 后加的）一直没被计数——写死的清单
# 与"实际有哪些文件"是两件事，前者会漂。fonts.css 只含 @font-face，不参与
# 体积预算（字体本身另有 FONTS_BUDGET）。
CSS_BUDGET_EXCLUDE = {"fonts.css"}
CSS_BUDGET_FILES = tuple(
    sorted(p.name for p in (DEFAULT_SITE / "assets").glob("*.css")
           if p.name not in CSS_BUDGET_EXCLUDE)
)
JS_BUDGET_FILE = "site.js"

# ---------------------------------------------------------------------------
# Allowlists — every entry must carry a reason, and an empty dict is the goal.
# Add an entry only with a comment saying why the exception is legitimate.
# ---------------------------------------------------------------------------

# Bitmaps: site-relative path -> reason. (D2 §3.3 exempts favicon.svg; the
# og:image social card is the only expected future entry.)
BITMAP_ALLOWLIST: dict[str, str] = {}

# Inline `style=`: page (site-relative) -> {exact attribute value: reason}.
INLINE_STYLE_ALLOWLIST: dict[str, dict[str, str]] = {}

# Turnstile: page (site-relative) -> reason for not carrying the device.
TURNSTILE_EXEMPT: dict[str, str] = {}

# Pages that legitimately carry no `data-page` nav marking. 404 is the only one:
# it is reached at an arbitrary URL, so marking a nav item "current" would be a
# lie, and it has no nav entry of its own. Everything else must declare one.
PAGE_ID_EXEMPT: dict[str, str] = {
    "404.html": "错误页：URL 任意，标任何一项为 aria-current 都是假话",
}

# `sitemap.xml` must list every page except these (an error page has no
# indexable URL of its own).
SITEMAP_EXEMPT: dict[str, str] = {
    "404.html": "错误页不进 sitemap（没有可索引的 URL）",
}


def load_nav_module():
    """Import scripts/gen-site-nav.py — the one implementation of the nav rules.

    The file name has a dash, so it cannot be a normal import; loading it by
    path keeps `gen-site-nav.py --check` and this checker provably identical.
    """
    path = Path(__file__).resolve().parent / "gen-site-nav.py"
    spec = importlib.util.spec_from_file_location("gen_site_nav", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules["gen_site_nav"] = module
    previous = sys.dont_write_bytecode
    sys.dont_write_bytecode = True  # never drop __pycache__ into scripts/
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = previous
    return module


class TagScanner(html.parser.HTMLParser):
    """Collect refs, tags, attributes and `<title>` text; fail loudly on markup."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.refs: list[str] = []
        self.tags: list[tuple[str, dict[str, str]]] = []
        self.stylesheets: list[str] = []
        self.scripts: list[str] = []
        self.images: list[str] = []
        self.inline_styles: list[tuple[str, str]] = []
        self.title: str = ""
        self._in_title = False

    def handle_starttag(self, tag: str, attrs) -> None:
        values = {name: (value or "") for name, value in attrs}
        self.tags.append((tag, values))
        for name, value in attrs:
            if name in {"href", "src"} and value:
                self.refs.append(value)
        if tag == "title":
            self._in_title = True
        elif tag == "img":
            self.images.append(values.get("src", ""))
        elif tag == "link" and "stylesheet" in values.get("rel", "").split():
            if values.get("href"):
                self.stylesheets.append(values["href"])
        elif tag == "script" and values.get("src"):
            self.scripts.append(values["src"])
        if values.get("style"):
            self.inline_styles.append((tag, values["style"]))

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self._in_title = False

    def handle_data(self, data: str) -> None:
        if self._in_title:
            self.title += data

    def meta_content(self, key: str, value: str) -> str | None:
        """Content of the first `<meta {key}="{value}">`, or None."""
        for tag, attrs in self.tags:
            if tag == "meta" and attrs.get(key) == value:
                return attrs.get("content", "")
        return None

    def link_href(self, rel: str) -> str | None:
        """href of the first `<link rel="{rel}">`, or None."""
        for tag, attrs in self.tags:
            if tag == "link" and rel in attrs.get("rel", "").split():
                return attrs.get("href", "")
        return None

    def has_class(self, name: str) -> bool:
        return any(
            name in attrs.get("class", "").split() for _, attrs in self.tags
        )


@dataclass
class Page:
    path: Path
    rel: str  # relative to the site root, posix
    source: str
    scanner: TagScanner
    repo: Path
    site: Path
    problems: dict[str, list[str]] = field(default_factory=dict)


def html_files(site: Path) -> list[Path]:
    return sorted(
        path
        for path in site.rglob("*.html")
        if not PAGE_SKIP_DIRS.intersection(path.relative_to(site).parts)
    )


def resolve_ref(page: Path, ref: str, repo: Path) -> Path | None:
    """Resolve a relative href/src against the page's own directory.

    `None` for external URLs, URI-scheme links (`https:`, `mailto:`,
    `vscode:` …), pure anchors, and links that escape the repo (those are
    someone else's problem — and a path-traversal smell).
    """
    if ref.startswith("#") or SCHEME_RE.match(ref):
        return None
    path = ref.split("#", 1)[0]
    if not path:
        return None
    candidate = (page.parent / path).resolve()
    if repo.resolve() not in candidate.parents:
        return None
    return candidate


def size_of(path: Path) -> int:
    return path.stat().st_size if path.is_file() else 0


def gzip_size(path: Path) -> int:
    """Bytes actually transferred for a text asset (GitHub Pages gzips them).

    Asserting on raw size alone would push authors to delete the comments that
    explain *why* a rule exists — comments are nearly free after gzip, so the
    honest budget is measured after gzip.
    """
    if not path.is_file():
        return 0
    return len(gzip.compress(path.read_bytes(), compresslevel=9))


def kb(size: int) -> str:
    return f"{size / 1024:.1f} KB" if size >= 1024 else f"{size} B"


# ---------------------------------------------------------------------------
# assertions — uniform signature, each fails independently, each returns a note
# ---------------------------------------------------------------------------


@dataclass
class Context:
    site: Path
    repo: Path
    pages: list[Page]
    parse_errors: list[str]
    nav: object | None
    nav_error: str | None


def assertion_links(ctx: Context) -> tuple[list[str], str]:
    problems = list(ctx.parse_errors)
    if not ctx.pages and not problems:
        problems.append("site/ has no HTML files at all")
    for page in ctx.pages:
        for ref in page.scanner.refs:
            target = resolve_ref(page.path, ref, ctx.repo)
            if target is not None and not target.exists():
                problems.append(f"{page.rel}: broken link -> {ref}")
    return problems, f"{len(ctx.pages)} page(s) scanned"


def assertion_versions(ctx: Context) -> tuple[list[str], str]:
    problems: list[str] = []
    for page in ctx.pages:
        # 版本号一律来自 site.json / Releases API，不许写死。
        if page.path.name == "site.json":
            continue
        for match in VERSION_RE.finditer(page.source):
            problems.append(
                f"{page.rel}: hardcoded version `{match.group(0)}` "
                "(use site/data/site.json or the Releases API)"
            )
    if not (ctx.site / "data" / "site.json").exists():
        problems.append("site/data/site.json is missing (run gen-site-data.py)")

    # `version.json` 是给页面回填版本号用的极小副本：`site.json` 有 12 KB，
    # 而 28 页每一页都要填一次版本号，为一个字符串付 12 KB 不划算。
    # 两份必须一致，否则页脚会显示一个与站点数据不符的版本——比不显示更糟。
    version_file = ctx.site / "data" / "version.json"
    if not version_file.exists():
        problems.append("site/data/version.json is missing (run gen-site-data.py)")
    else:
        try:
            small = json.loads(version_file.read_text(encoding="utf-8")).get("version")
            full_path = ctx.site / "data" / "site.json"
            full = (
                json.loads(full_path.read_text(encoding="utf-8")).get("version")
                if full_path.exists()
                else None
            )
            if full is not None and full != small:
                problems.append(
                    f"site/data/version.json says {small!r} but site.json says {full!r}"
                )
        except json.JSONDecodeError as exc:
            problems.append(f"site/data/version.json is not valid JSON: {exc}")
    return problems, f"{len(ctx.pages)} page(s) scanned"


def _partials_or_problem(ctx: Context):
    """(partials, problems, note) — the nav module may legitimately be missing."""
    if ctx.nav is None:
        return None, [ctx.nav_error or "scripts/gen-site-nav.py unavailable"], "unavailable"
    try:
        return ctx.nav.load_partials(ctx.site), [], ""
    except Exception as exc:  # NavError: loud, file-naming
        return None, [str(exc)], "partials unavailable"


def assertion_nav_drift(ctx: Context) -> tuple[list[str], str]:
    partials, problems, note = _partials_or_problem(ctx)
    if partials is None:
        return problems, note
    for page in ctx.pages:
        problems += ctx.nav.block_problems(ctx.site, page.rel, page.source, partials)
    return problems, f"{len(ctx.pages)} page(s) vs site/_partials/ (1 comparison rule)"


def assertion_data_page(ctx: Context) -> tuple[list[str], str]:
    partials, problems, note = _partials_or_problem(ctx)
    if partials is None:
        return problems, note
    for page in ctx.pages:
        if page.rel in PAGE_ID_EXEMPT:
            continue
        problems += ctx.nav.page_id_problems(page.rel, page.source, partials)
    return problems, f"{len(ctx.pages)} page(s) checked for data-page/aria-current"


def assertion_metadata(ctx: Context) -> tuple[list[str], str]:
    problems: list[str] = []
    for page in ctx.pages:
        scanner = page.scanner
        headings = [tag for tag, _ in scanner.tags if tag == "h1"]
        if len(headings) != 1:
            problems.append(
                f"{page.rel}: expected exactly one <h1>, found {len(headings)}"
            )
        if not scanner.title.strip():
            problems.append(f"{page.rel}: <title> is missing or empty")
        for label, content in (
            ('<meta name="description">', scanner.meta_content("name", "description")),
            ('<meta name="theme-color">', scanner.meta_content("name", "theme-color")),
        ):
            if not content:
                problems.append(
                    f"{page.rel}: {label} is missing or has empty content"
                )
        if not scanner.link_href("canonical"):
            problems.append(
                f'{page.rel}: <link rel="canonical"> is missing or has an empty href'
            )
        for prop in ("og:title", "og:description", "og:type"):
            if not scanner.meta_content("property", prop):
                problems.append(
                    f'{page.rel}: <meta property="{prop}"> is missing or has empty content'
                )
    return problems, f"{len(ctx.pages)} page(s) × 8 required metadata items"


def assertion_asset_budget(ctx: Context) -> tuple[list[str], str]:
    problems: list[str] = []
    assets = ctx.site / "assets"

    for page in ctx.pages:
        for ref in page.scanner.stylesheets + page.scanner.scripts:
            target = resolve_ref(page.path, ref, ctx.repo)
            if target is not None and not target.exists():
                problems.append(f"{page.rel}: referenced asset missing -> {ref}")

    html_sizes = {page.rel: len(page.source.encode("utf-8")) for page in ctx.pages}
    rel, biggest = max(html_sizes.items(), key=lambda item: item[1], default=("—", 0))
    if biggest > HTML_BUDGET:
        problems.append(
            f"{rel}: page HTML is {kb(biggest)}, over the {kb(HTML_BUDGET)} budget (D2 §3.4)"
        )
    # 内容页（诊断字典 64 个码、语言清单）本来就长；raw 上限只是为了拦住
    # "悄悄膨胀"，真正该卡的是 gzip 后的传输量。两条都断言。
    html_gzip = {p.rel: gzip_size(p.path) for p in ctx.pages}
    rel_gz, biggest_gz = max(html_gzip.items(), key=lambda item: item[1], default=("—", 0))
    if biggest_gz > HTML_GZIP_BUDGET:
        problems.append(
            f"{rel_gz}: page HTML gzips to {kb(biggest_gz)}, over the "
            f"{kb(HTML_GZIP_BUDGET)} budget (D2 §3.4)"
        )

    css = {name: size_of(assets / name) for name in CSS_BUDGET_FILES}
    css_total = sum(css.values())
    if css_total > CSS_BUDGET:
        problems.append(
            f"site/assets: {' + '.join(CSS_BUDGET_FILES)} total {kb(css_total)}, "
            f"over the {kb(CSS_BUDGET)} budget (D2 §3.4)"
        )
    # 真正传输的是 gzip 后的大小，所以两条都断言：raw 防"悄悄膨胀"，
    # gzip 才是读者实际付的代价（GitHub Pages 对文本资源默认压缩）。
    css_gzip = sum(gzip_size(assets / name) for name in CSS_BUDGET_FILES)
    if css_gzip > CSS_GZIP_BUDGET:
        problems.append(
            f"site/assets: the four stylesheets gzip to {kb(css_gzip)}, "
            f"over the {kb(CSS_GZIP_BUDGET)} budget (D2 §3.4)"
        )
    missing_css = [name for name in CSS_BUDGET_FILES if not (assets / name).is_file()]

    js_path = assets / JS_BUDGET_FILE
    js = size_of(js_path)
    if js > JS_BUDGET:
        problems.append(
            f"site/assets/{JS_BUDGET_FILE}: {kb(js)}, over the {kb(JS_BUDGET)} budget (D2 §3.4)"
        )

    fonts_dir = assets / "fonts"
    font_files = (
        sorted(p for p in fonts_dir.glob("*") if p.is_file())
        if fonts_dir.is_dir()
        else []
    )
    fonts_total = sum(size_of(p) for p in font_files)
    if fonts_total > FONTS_BUDGET:
        problems.append(
            f"site/assets/fonts: {len(font_files)} file(s) total {kb(fonts_total)}, "
            f"over the {kb(FONTS_BUDGET)} budget (D2 §3.4)"
        )

    css_detail = ", ".join(
        f"{name} {kb(size) if (assets / name).is_file() else 'missing'}"
        for name, size in css.items()
    )
    note = (
        f"measured: html max {kb(biggest)} ({rel}) / {kb(HTML_BUDGET)} · "
        f"html gzip max {kb(biggest_gz)} ({rel_gz}) / {kb(HTML_GZIP_BUDGET)} · "
        f"css {kb(css_total)} raw / {kb(CSS_BUDGET)} · gzip {kb(css_gzip)} / "
        f"{kb(CSS_GZIP_BUDGET)} ({css_detail}) · "
        f"site.js {kb(js) if js_path.is_file() else 'missing'} / {kb(JS_BUDGET)} · "
        f"fonts {kb(fonts_total)} in {len(font_files)} file(s) / {kb(FONTS_BUDGET)}"
    )
    return problems, note


def site_key(page: Page, ref: str) -> str:
    """Allowlist key for a reference: site-relative path when it resolves inside."""
    target = resolve_ref(page.path, ref, page.repo)
    if target is None:
        return ref
    try:
        return target.relative_to(page.site).as_posix()
    except ValueError:
        return ref


def assertion_no_bitmaps(ctx: Context) -> tuple[list[str], str]:
    problems: list[str] = []
    for page in ctx.pages:
        reported: set[str] = set()
        for match in BITMAP_RE.finditer(page.source):
            ref = match.group(0)
            if site_key(page, ref) in BITMAP_ALLOWLIST:
                continue
            line = page.source.count("\n", 0, match.start()) + 1
            problems.append(
                f"{page.rel}:{line}: bitmap reference `{ref}` — the site ships zero "
                "bitmaps (D2 §3.3); use SVG, or add a reasoned BITMAP_ALLOWLIST entry"
            )
            reported.add(ref)
        for src in page.scanner.images:
            if not src:
                problems.append(f"{page.rel}: <img> without a src")
                continue
            if src.split("#")[0].split("?")[0].lower().endswith(".svg"):
                continue
            if src in reported or site_key(page, src) in BITMAP_ALLOWLIST:
                continue
            problems.append(
                f'{page.rel}: <img src="{src}"> is not an SVG — bitmaps are not '
                "allowed (D2 §3.3)"
            )
    return problems, f"{len(ctx.pages)} page(s) scanned for <img>/bitmap references"


def assertion_no_inline_style(ctx: Context) -> tuple[list[str], str]:
    problems: list[str] = []
    for page in ctx.pages:
        allowed = INLINE_STYLE_ALLOWLIST.get(page.rel, {})
        for tag, value in page.scanner.inline_styles:
            if value in allowed:
                continue
            problems.append(
                f'{page.rel}: inline style="{value}" on <{tag}> — use a token/class '
                "(D2 §3.3); an allowlist entry needs a reason"
            )
    return problems, f"{len(ctx.pages)} page(s) scanned for style= attributes"


def assertion_turnstile(ctx: Context) -> tuple[list[str], str]:
    problems: list[str] = []
    for page in ctx.pages:
        if page.scanner.has_class("turnstile") or page.rel in TURNSTILE_EXEMPT:
            continue
        problems.append(
            f"{page.rel}: no .turnstile element — it is the site's one memorable "
            "design device (D2 §3.3); add it, or add a reasoned TURNSTILE_EXEMPT entry"
        )
    return problems, f"{len(ctx.pages)} page(s) checked"


def assertion_sitemap(ctx: "Context") -> tuple[list[str], str]:
    """`sitemap.xml` and the real page set must agree, both directions.

    A hand-written sitemap is exactly the kind of list this repository has
    watched drift six times. Two failure modes matter and both are silent:
    a URL that 404s (crawlers cache it), and a page missing from the sitemap
    (invisible). So assert set equality, not a count.
    """
    problems: list[str] = []
    sitemap = ctx.site / "sitemap.xml"
    if not sitemap.is_file():
        return ["site/sitemap.xml is missing"], "no sitemap"

    text = sitemap.read_text(encoding="utf-8")
    prefix = "https://colorlessboy.github.io/sokonanoda-lang/"
    listed: set[str] = set()
    for loc in re.findall(r"<loc>([^<]+)</loc>", text):
        if not loc.startswith(prefix):
            problems.append(f"sitemap.xml: <loc>{loc}</loc> is not under {prefix}")
            continue
        rest = loc[len(prefix) :]
        # `/` 与 `/en/` 都是目录 URL，各自映射到该目录的 index.html。
        # （早先这里把两者都映射成根 index.html，于是 `en/index.html` 永远被判"未列出"。）
        if rest == "":
            listed.add("index.html")
        elif rest.endswith("/"):
            listed.add(rest + "index.html")
        else:
            listed.add(rest)

    actual: set[str] = set()
    for page in ctx.pages:
        rel = page.rel
        if rel in SITEMAP_EXEMPT:
            continue
        actual.add(rel)

    for rel in sorted(actual - listed):
        problems.append(f"sitemap.xml: page `{rel}` exists but is not listed")
    for rel in sorted(listed - actual):
        problems.append(f"sitemap.xml: lists `{rel}` but no such page exists")

    return problems, f"{len(listed)} url(s) vs {len(actual)} page(s)"


def git_dirty_crates(repo: Path) -> int:
    """How many files under `crates/` differ from HEAD (0 when clean).

    Returns 0 on any failure — this is a courtesy notice, and a repository
    without git (or without git on PATH) must not turn it into a site failure.
    """
    try:
        out = subprocess.run(
            ["git", "status", "--porcelain", "--", "crates/"],
            cwd=repo, capture_output=True, text=True, timeout=20, check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return 0
    if out.returncode != 0:
        return 0
    return sum(1 for line in out.stdout.splitlines() if line.strip())


ASSERTIONS = (
    ("links", assertion_links),
    ("versions", assertion_versions),
    ("nav-drift", assertion_nav_drift),
    ("data-page", assertion_data_page),
    ("metadata", assertion_metadata),
    ("asset-budget", assertion_asset_budget),
    ("no-bitmaps", assertion_no_bitmaps),
    ("no-inline-style", assertion_no_inline_style),
    ("turnstile", assertion_turnstile),
    ("sitemap", assertion_sitemap),
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Site hygiene checks (see the module docstring for the 10 assertions)."
    )
    parser.add_argument(
        "--site",
        type=Path,
        default=DEFAULT_SITE,
        help="site root (default: <repo>/site); use a scratch copy to self-test",
    )
    args = parser.parse_args(argv)
    site = args.site.resolve()
    if not site.is_dir():
        print(f"check-site.py: --site {args.site} is not a directory", file=sys.stderr)
        return 2
    repo = site.parent

    pages: list[Page] = []
    parse_errors: list[str] = []
    for path in html_files(site):
        rel = path.relative_to(site).as_posix()
        source = path.read_text(encoding="utf-8")
        scanner = TagScanner()
        try:
            scanner.feed(source)
            scanner.close()
        except Exception as exc:  # malformed markup
            parse_errors.append(f"{rel}: HTML parse error: {exc}")
            continue
        pages.append(
            Page(path=path, rel=rel, source=source, scanner=scanner, repo=repo, site=site)
        )

    try:
        nav: object | None = load_nav_module()
        nav_error = None
    except Exception as exc:
        nav, nav_error = None, f"scripts/gen-site-nav.py could not be imported: {exc}"

    ctx = Context(
        site=site, repo=repo, pages=pages, parse_errors=parse_errors, nav=nav, nav_error=nav_error
    )

    results = []
    for name, assertion in ASSERTIONS:
        try:
            problems, note = assertion(ctx)
        except Exception as exc:  # a crashed assertion is a failure, never a skip
            problems, note = [f"assertion {name} crashed: {exc!r}"], "crashed"
        results.append((name, problems, note))

    total = sum(len(problems) for _, problems, _ in results)
    failed = sum(1 for _, problems, _ in results if problems)
    if total:
        print(
            f"site hygiene: {total} problem(s); {failed}/{len(results)} assertion(s) failed"
        )
    for name, problems, note in results:
        status = "FAIL" if problems else "ok  "
        suffix = f"  ·  {note}" if note else ""
        print(f"  {name:<16} {status}  {len(problems):>3} problem(s){suffix}")
        for problem in problems:
            print(f"    - {problem}")

    # 版本陷阱提醒（不是站点的错，所以不算失败，但必须说出来）。
    #
    # `scripts/soko` 的解析顺序里「仓库构建」优先，所以只要 crates/ 是脏的，
    # 它量的就是**未发布代码**。这一条是拿一次真实误判换来的：我据此把 C1 的
    # 「七个 tactic」错改成「十三个」，因为工作区有一份给 by 块加 tactic 的 WIP。
    # 站点写的是已发布版本的事实，所以这里每次都提醒一句。
    dirty = git_dirty_crates(DEFAULT_SITE.parent)
    if dirty:
        print(
            f"\n⚠  crates/ 有 {dirty} 个未提交改动 —— scripts/soko 解析的是用这棵树"
            "编出来的仓库构建，量到的是**未发布代码**。\n"
            "   要量已发布版本的事实，先钉二进制：\n"
            "   export SOKONANODA_BIN=~/.vscode/extensions/"
            "sokonanoda-lang.sokonanoda-<版本>-<平台>/bin/<平台>/sokonanoda\n"
            "   详见 docs/design/site-rebuild/spec/D9-page-brief.md §4.0"
        )

    if total:
        return 1

    print(
        f"site hygiene: ok ({len(pages)} pages, {len(results)} assertions, "
        "links and versions clean)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

