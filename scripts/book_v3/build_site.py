#!/usr/bin/env python3
"""Book v3 — static multi-page site builder (prototype).

Source of truth for CONTENT is the v2 single-page render:
  dist/book/book-v2/index.html   (built by scripts/build_book_proto.py)
  courses/analysis/course.json
We split the rendered unit/appendix fragments out of that page and re-assemble
them into one page per unit, plus chapter/cover/search/sitemap/print pages.

Why fragments first: v2 already owns the hard part (course parsing, math/code
rendering, hint/solution markup). v3 is an information-architecture change, not
a content-pipeline rewrite. The moment the content model is exposed (see v2
build_book_proto.py parse_* functions), the same templates take model objects;
only `load_fragments()` changes.

Generalises to all 72 units: every list here comes from course.json / the
preview's unit sections — no unit number is hardcoded.

Usage (from repo root, after `python3 scripts/build_book_proto.py --out dist/book/book-v2`):
  python3 scripts/book_v3/build_site.py --out dist/book/book-v3
  python3 scripts/book_v3/build_site.py --out dist/book/book-v3 --check
Options:
  --demo 1,35,72        only emit these unit pages (default: all units)
  --print 1,35,72       clean print pages for these units (default: same as demo)
  --print-chapters II.1,II.4,II.7   chapter print bundles (default: none)
  --base-url URL        canonical/sitemap base (default https://example.invalid/book-v3)
"""
from __future__ import annotations

import argparse
import html as html_mod
import json
import re
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TEMPLATES = HERE / "templates"
ASSETS = HERE / "assets"
SITE_CSS_FALLBACK_NAME = "book-components.css"


# ────────────────────────── generic helpers ──────────────────────────
def read_text(p: Path) -> str:
    return p.read_text(encoding="utf-8", errors="replace")


def strip_tags(s: str) -> str:
    s = re.sub(r"<[^>]+>", "", s)
    return html_mod.unescape(re.sub(r"\s+", " ", s)).strip()


def balanced_section(html: str, start: int) -> str:
    """Return html[start:] up to the matching </section> (handles nesting)."""
    depth = 0
    i = start
    pat = re.compile(r"<section\b|</section>")
    while True:
        m = pat.search(html, i)
        if not m:
            raise ValueError("unbalanced <section> at %d" % start)
        if m.group(0) == "</section>":
            depth -= 1
        else:
            depth += 1
        i = m.end()
        if depth == 0:
            return html[start:i]


def pad(n: int | str) -> str:
    n = str(int(n))
    return n if len(n) >= 2 else "0" + n


def rel(root: str, path: str) -> str:
    """'units/unit01.html' -> './units/unit01.html' for root='.' pages."""
    return path if root == "." else root.rstrip("/") + "/" + path


def render(template_name: str, **tokens: str) -> str:
    s = read_text(TEMPLATES / template_name)
    for k, v in tokens.items():
        s = s.replace("{{%s}}" % k, v)
    left = re.findall(r"\{\{[A-Z_]+\}\}", s)
    if left:
        raise ValueError("unreplaced tokens in %s: %s" % (template_name, left[:5]))
    return s


# ────────────────────────── content loading ──────────────────────────
class Book:
    def __init__(self, manifest_path: Path, preview_path: Path, style_css: Path):
        self.source = manifest_path.parent.parent.parent  # repo root (for relative links only)
        self.style_css = style_css
        self.manifest = json.loads(read_text(manifest_path))
        self.preview = read_text(preview_path)
        self.cover = self._extract_cover()
        # Sibling volumes that this build will actually emit pages for; empty
        # means a single-volume book, and then no cross-volume link is written
        # (a link to a volume that was not built is a 404 by construction).
        self.siblings: list[tuple[str, str, str]] = []  # (out dir, label, title)
        self.units: list[dict] = []
        self.chapters: list[dict] = []
        self.by_id: dict[str, dict] = {}
        self._build_manifest()
        self.unit_html: dict[int, str] = {}
        self.appendix_html: dict[int, str] = {}
        self.unit_meta: dict[int, dict] = {}
        self.unit_intro: dict[int, str] = {}
        self.ids: dict[int, set[str]] = {}
        self.index = self._extract_index()
        self.anchor_unit = {e["a"]: e["u"] for e in self.index if isinstance(e.get("u"), int)}
        self.unit_title: dict[int, str] = {}
        self.unit_en: dict[int, str] = {}
        self._extract_fragments()

    # -- manifest -----------------------------------------------------
    def _build_manifest(self):
        chapters = []
        total = 0
        display_index = 0
        for vi, vol in enumerate(self.manifest.get("volumes", []), 1):
            for ci, ch in enumerate(vol.get("chapters", []), 1):
                c = {
                    "id": ch["id"],                 # II.1
                    "slug": ch["id"].split(".")[0].lower() + "-" + ch["id"].split(".")[-1],
                    "title": ch.get("title", ""),
                    "volume_title": vol.get("title", ""),
                    "volume_id": vol.get("id", ""),
                    "index": ci,
                    "prereqs": ch.get("prereqs", []),
                    "tags": ch.get("tags", []),
                    "units": [],
                    "global_index": len(chapters) + 1,
                }
                for u in ch.get("units", []):
                    total += 1
                    display_index += 1
                    n = int(u.get("unit") or total)
                    unit = {
                        "n": n,
                        "display_n": display_index,
                        "file": u["file"],
                        "title": u.get("title", ""),
                        "title_en": u.get("title_en", ""),
                        "chapter": c,
                    }
                    c["units"].append(unit)
                    self.units.append(unit)
                chapters.append(c)
        self.chapters = chapters
        for i, u in enumerate(self.units):
            u["prev"] = self.units[i - 1] if i else None
            u["next"] = self.units[i + 1] if i + 1 < len(self.units) else None
            self.by_id[str(u["n"])] = u
        for i, c in enumerate(chapters):
            c["prev"] = chapters[i - 1] if i else None
            c["next"] = chapters[i + 1] if i + 1 < len(chapters) else None
            first, last = c["units"][0], c["units"][-1]
            c["first_unit"], c["last_unit"] = first, last

    # -- preview fragments -------------------------------------------
    def _extract_cover(self) -> str:
        m = re.search(r'<header class="cover" id="top">.*?</header>', self.preview, re.S)
        if not m:
            raise ValueError("cover header not found in preview")
        return m.group(0).replace('<header class="cover" id="top">', '<header class="cover">', 1)

    def _extract_index(self) -> list[dict]:
        m = re.search(r'<script type="application/json" id="book-search-index">(.*?)</script>',
                      self.preview, re.S)
        if not m:
            raise ValueError("book-search-index not found")
        return json.loads(m.group(1))

    def _extract_fragments(self):
        unit_starts = [(m.start(), int(m.group(1))) for m in re.finditer(
            r'<section class="unit[^"]*" id="unit-(\d+)">', self.preview)]
        unit_starts.sort()
        for k, (pos, n) in enumerate(unit_starts):
            end = unit_starts[k + 1][0] if k + 1 < len(unit_starts) else None
            block = balanced_section(self.preview, pos)
            self.unit_html[n] = block
            self.ids[n] = set(re.findall(r'id="([^"]+)"', block))
            m = re.search(r'<h2><span class="unit-number">\s*([^<]*)\s*</span><span>(.*?)</span></h2>', block, re.S)
            if m:
                self.unit_title[n] = strip_tags(m.group(2))
            m2 = re.search(r'<p class="unit-title-en">(.*?)</p>', block, re.S)
            if m2:
                self.unit_en[n] = strip_tags(m2.group(1))
            mm = re.search(r'<p class="unit-meta">(.*?)</p>', block, re.S)
            meta = {"text": "", "defs": 0, "thms": 0, "ex": 0, "hints": 0}
            if mm:
                raw = re.sub(r'<a\b.*?</a>', '', mm.group(1), flags=re.S)
                meta["text"] = strip_tags(raw)
                for key, pat in (("defs", r"(\d+)\s*个定义"), ("thms", r"(\d+)\s*个定理"),
                                 ("ex", r"(\d+)\s*道练习"), ("hints", r"(\d+)\s*条提示")):
                    g = re.search(pat, meta["text"])
                    meta[key] = int(g.group(1)) if g else 0
            self.unit_meta[n] = meta
            m3 = re.search(r'<section class="unit-intro"', block)
            if m3:
                intro = balanced_section(block, m3.start())
                mp = re.search(r"<p>(.*?)</p>", intro, re.S)
                if mp:
                    self.unit_intro[n] = strip_tags(mp.group(1))
        for m in re.finditer(r'<section class="appendix-unit" id="appendix-unit-(\d+)">', self.preview):
            n = int(m.group(1))
            self.appendix_html[n] = balanced_section(self.preview, m.start())

    # -- lookups ------------------------------------------------------
    def chapter_of(self, unit_n: int) -> dict:
        return self.by_id[str(unit_n)]["chapter"]

    def chapter_units(self, chapter: dict) -> list[dict]:
        return chapter["units"]

    def unit_label(self, u: dict) -> str:
        return "单元 %s %s" % (self.display_n(u), u["title"])

    def display_n(self, u: dict) -> str:
        return pad(u.get("display_n", u["n"]))

    def display_title(self, u: dict) -> str:
        t = re.sub(r"^单元[①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳㉑-㉟㊱-㊿\d]+\s*", "", u["title"]).strip()
        t = t.replace("`", "").replace("**", "").strip()
        return t or u["title"]


# ────────────────────────── link rewriting ──────────────────────────
LINK_RE = re.compile(r'href="#([^"]+)"')


def resolve_anchor(a: str, cur_unit: int | None, cur_chapter: dict | None) -> str | None:
    """Return site-root-relative target (path or path#anchor) / '#'-local, or None."""
    m = re.match(r"unit-(\d+)$", a)
    if m:
        n = int(m.group(1))
        if cur_unit and n == cur_unit:
            return "#top"
        return "units/unit%s.html" % pad(n)
    m = re.match(r"chapter-([ivxlcdm]+)-(\d+)$", a)
    if m:
        return "chapters/%s-%s.html" % (m.group(1).lower(), m.group(2))
    m = re.match(r"appendix-unit-(\d+)$", a)
    if m:
        n = int(m.group(1))
        if cur_unit and n == cur_unit:
            return "#%s" % a
        base = "units/unit%s.html" % pad(n)
        return "%s#appendix-unit-%d" % (base, n)
    if a == "appendix-solutions":
        return "sitemap.html"
    m = re.match(r"(?:app-sol|sol)-(\d+)-\d+$", a)
    if m:
        n = int(m.group(1))
        if cur_unit and n == cur_unit:
            return "#%s" % a
        return "units/unit%s.html#%s" % (pad(n), a)
    m = re.match(r"(?:decl|ex|section)-(\d+)-\d+$", a)
    if m:
        n = int(m.group(1))
        if cur_unit and n == cur_unit:
            return "#%s" % a
        return "units/unit%s.html#%s" % (pad(n), a)
    if a == "toc":
        if cur_chapter:
            return "chapters/%s.html#units" % cur_chapter["slug"]
        return "#path"
    if a in ("top", "main", "path", "learning-path"):
        return "#" + a
    if a in ANCHOR_MAPS:
        return ANCHOR_MAPS[a]
    return "LOOKUP:" + a   # resolved against anchor_unit later


ANCHOR_MAPS: dict[str, str] = {}


def rewrite_links(html: str, book: Book, root: str, cur_unit: int | None,
                  cur_chapter: dict | None) -> tuple[str, list[str]]:
    unresolved: list[str] = []

    def repl(m: re.Match) -> str:
        a = m.group(1)
        target = resolve_anchor(a, cur_unit, cur_chapter)
        if target is None:
            return m.group(0)
        if target.startswith("LOOKUP:"):
            anchor = target.split(":", 1)[1]
            u = book.anchor_unit.get(anchor)
            if u is None:
                unresolved.append(anchor)
                return m.group(0)
            if cur_unit and int(u) == cur_unit:
                return "#" + anchor
            return rel(root, "units/unit%s.html#%s" % (pad(u), anchor))
        if target.startswith("#"):
            return 'href="%s"' % target
        return 'href="%s"' % rel(root, target)

    return LINK_RE.sub(repl, html), unresolved


# ────────────────────────── page pieces ──────────────────────────
def breadcrumb(items: list[tuple[str | None, str]], root: str) -> str:
    lis = []
    for href, label in items:
        if href:
            lis.append('<li><a href="%s">%s</a></li>' % (rel(root, href), label))
        else:
            lis.append('<li aria-current="page">%s</li>' % label)
    return ('<nav class="breadcrumb" aria-label="面包屑"><ol>%s</ol></nav>'
            % "".join(lis))


def sidebar(book: Book, root: str, active_chapter: str | None = None,
            active_unit: int | None = None) -> str:
    # One nested tree: chapter -> units.  The active chapter is expanded; the
    # others are collapsed.  No second “chapter units” block below the list.
    blocks = []
    for ch in book.chapters:
        current = active_chapter == ch["id"]
        unit_items = []
        for u in ch["units"]:
            unit_items.append(
                '<li%s><a href="%s">%02d %s</a></li>' % (
                    ' class="is-current"' if active_unit == u["n"] else '',
                    rel(root, "units/unit%s.html" % pad(u["n"])), u.get("display_n", u["n"]),
                    html_mod.escape(book.display_title(u))))
        blocks.append(
            '<details class="side-chapter"%s><summary><a href="%s">%s %s</a></summary>'
            '<ol>%s</ol></details>' % (
                ' open' if current else '', rel(root, chapter_url(ch)),
                ch["id"], html_mod.escape(ch["title"]), "".join(unit_items)))
    return ('<aside class="site-side no-print" id="site-side" aria-label="目录">'
            '<div class="side-head"><strong>目录</strong>'
            '<button type="button" class="side-close" data-side-toggle aria-label="关闭目录">×</button></div>'
            '<nav class="side-nav">%s</nav></aside>' % "".join(blocks))


def unit_toc(book: Book, n: int) -> str:
    items = [("#unit-intro", "导语")]
    for e in book.index:
        if e.get("u") == n and e.get("k") == "section":
            items.append(("#" + e["a"], e["t"]))
    if not any(e.get("u") == n and e.get("k") == "section" for e in book.index):
        ids = book.ids.get(n, set())
        decls = sorted((i for i in ids if i.startswith("decl-")), key=lambda x: int(x.rsplit("-", 1)[-1]))
        exs = sorted((i for i in ids if i.startswith("ex-")), key=lambda x: int(x.rsplit("-", 1)[-1]))
        if decls:
            items.append(("#" + decls[0], "定义与定理"))
        if exs:
            items.append(("#" + exs[0], "练习"))
    if n in book.appendix_html:
        items.append(("#appendix-unit-%d" % n, "练习与解答"))
    return "\n".join('<li><a href="%s">%s</a></li>' % (h, html_mod.escape(t)) for h, t in items)


def pager(prev_item: dict | None, next_item: dict | None, root: str,
          label_prev: str, label_next: str) -> str:
    left = ('<a href="%s">← %s</a>' % (rel(root, label_prev), html_mod.escape(prev_item["label"]))) if prev_item else '<span class="empty">本章到头了</span>'
    right = ('<a class="next" href="%s">%s →</a>' % (rel(root, label_next), html_mod.escape(next_item["label"]))) if next_item else '<span class="empty">没有下一页</span>'
    return left + right


def footer(book: Book) -> str:
    return ("从集合论到实分析 · 卷 II · v3 多页原型 · 内容取自 <code>agent/book-prototype</code> "
            "@ <code>c19346da</code> · %d 单元 / %d 章 · 无外部请求" %
            (len(book.units), len(book.chapters)))


# The volume currently being built — set once in main() from --volume.
CURRENT_VOLUME = ["volume-ii"]
CURRENT_VOLUME_META: dict = {}


def brand_of(book: Book) -> str:
    meta = CURRENT_VOLUME_META or {}
    label = meta.get("label") or "卷 II"
    title = meta.get("title") or "数系与实数"
    return "%s %s" % (label, title)


def book_volume_cards(book: Book) -> str:
    current = CURRENT_VOLUME[0]
    meta = CURRENT_VOLUME_META or {}
    cards = []
    for vol_dir, label, title in book.siblings:
        units = meta.get("units", "") if vol_dir == current else ""
        count = meta.get("count", "") if vol_dir == current else ""
        cls = "volume-card is-current" if vol_dir == current else "volume-card"
        href = "index.html" if vol_dir == current else "../%s/index.html" % vol_dir
        cur = ' aria-current="page"' if vol_dir == current else ""
        cards.append(
            '<a class="%s" href="%s"%s><span class="volume-num">%s</span>'
            '<strong>%s</strong><span>%s</span></a>' % (cls, href, cur, label, title, count or units))
    return "".join(cards)


def page(book: Book, out: Path, relpath: str, *, kind: str, title: str,
         description: str, body: str, breadcrumb_html: str = "", page_class: str = "",
         base_url: str = "", active_chapter: str | None = None,
         active_unit: int | None = None, sidebar_html: str | None = None) -> None:
    root = "." if "/" not in relpath else ".."
    total_units = len(book.units)
    if sidebar_html is None:
        sidebar_html = sidebar(book, root, active_chapter, active_unit)
    shell_class = "" if sidebar_html else " no-side"
    home_url = "../index.html" if root == "." else "../../index.html"
    brand_title, brand_sub = brand_of(book), "从集合论到实分析 · 交互读本 v3"
    if kind == "print":
        book_switch = ""
    else:
        back = "" if root == "." else "../"
        current_vol = CURRENT_VOLUME[0]
        parts = []
        for vol_dir, label, _title in book.siblings:
            if vol_dir == current_vol:
                parts.append('<a href="%sindex.html" aria-current="true">%s</a>' % (back, label))
            else:
                parts.append('<a href="%s../%s/index.html">%s</a>' % (back, vol_dir, label))
        book_switch = "".join(parts)
    doc = render("base.html",
                 TITLE=html_mod.escape(title),
                 DESCRIPTION=html_mod.escape(description),
                 CANONICAL=(base_url.rstrip("/") + "/" + relpath) if base_url else relpath,
                 ROOT=root, PAGE_KIND=kind, PAGE_CLASS=page_class,
                 BREADCRUMB=breadcrumb_html, BODY=body,
                 SIDEBAR=sidebar_html, SHELL_CLASS=shell_class, BOOK_SWITCH=book_switch,
                 HOME_URL=home_url, BRAND_TITLE=brand_title, BRAND_SUB=brand_sub,
                 TOTAL_UNITS=str(total_units), FOOTER_NOTE=footer(book))
    target = out / relpath
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(doc, encoding="utf-8")


def chapter_url(ch: dict) -> str:
    return "chapters/%s.html" % ch["slug"]


FRACTION_RE = re.compile(r"\(([^()]+)\)\s*/\s*\(([^()]+)\)")


def readable_math(html: str) -> str:
    """Make simple inline fractions readable: (a-b)/(m+1) -> stacked fraction."""
    def span(m):
        inner = m.group(2)
        def frac(fm):
            num = fm.group(1).replace("-", "−")
            den = fm.group(2).replace("-", "−")
            return ('<span class="frac-inline"><span class="num">%s</span>'
                    '<span class="den">%s</span></span>' % (num, den))
        return m.group(1) + FRACTION_RE.sub(frac, inner) + m.group(3)
    return re.sub(r'(<span class="math">)(.*?)(</span>)', span, html)


# ────────────────────────── builders ──────────────────────────
def build_unit(book: Book, out: Path, u: dict, base_url: str) -> None:
    n = u["n"]
    dn = book.display_n(u)
    ch = u["chapter"]
    root = ".."
    block, u1 = rewrite_links(book.unit_html[n], book, root, n, ch)
    block = readable_math(block)
    block = block.replace('<section class="unit-intro"', '<section class="unit-intro" id="unit-intro"', 1)
    # h2 -> h1 (page needs exactly one h1; keep the unit number + title markup)
    block = re.sub(r'(<span class="unit-number">)(\d+)(</span>)', r'\g<1>%s\g<3>' % dn, block, count=1)
    block = block.replace("<h2><span class=\"unit-number\">", "<h1><span class=\"unit-number\">", 1)
    block = re.sub(r"</h2>", "</h1>", block, count=1)
    block = re.sub(r'(<h1><span class="unit-number">[^<]*</span>)(<span>)', r'\1 \2', block, count=1)
    block = re.sub(r'全书第\s*\d+\s*/\s*\d+\s*单元', '全书第 %s / %s 单元' % (dn, len(book.units)), block, count=1)
    # print entry; the old "↑ 目录" jumped to the chapter top, which is not
    # what the right rail is for now — remove it.
    block = block.replace('<a class="back-to-toc"',
                          '<p class="unit-print-entry no-print"><a href="%s">打印 / PDF 本单元</a></p>'
                          '<a class="back-to-toc" data-remove="1"' % rel(root, "print/unit%s.html" % pad(n)), 1)
    block = re.sub(r'<a class="back-to-toc"[^>]*>.*?</a>', '', block, flags=re.S)
    block = block.replace('href="#toc"', 'href="%s#units"' % rel(root, chapter_url(ch)))
    chapter_outline = "\n".join(
        '<li%s><a href="%s" aria-current="%s"><span>%s</span> %s</a></li>' % (
            ' class="is-current"' if u2["n"] == n else '',
            rel(root, "units/unit%s.html" % pad(u2["n"])),
            'page' if u2["n"] == n else 'false',
            book.display_n(u2), html_mod.escape(book.display_title(u2)))
        for u2 in ch["units"])
    appendix = ""
    if n in book.appendix_html:
        appendix, u2 = rewrite_links(book.appendix_html[n], book, root, n, ch)
        appendix = readable_math(appendix)
        u1 += u2
        appendix = '<section class="unit-appendix" aria-label="练习与解答">%s</section>' % appendix
    prev_u, next_u = u.get("prev"), u.get("next")
    prev_item = {"label": ("%s %s" % (book.display_n(prev_u), book.display_title(prev_u))) if prev_u else ""}
    next_item = {"label": ("%s %s" % (book.display_n(next_u), book.display_title(next_u))) if next_u else ""}
    body = render("unit.html",
                  ROOT=root, UNIT_TOC=unit_toc(book, n),
                  UNIT_BODY=block, APPENDIX_BODY=appendix,
                  CHAPTER_URL=rel(root, chapter_url(ch)),
                  PRINT_URL=rel(root, "print/unit%s.html" % pad(n)),
                  CHAPTER_OUTLINE=chapter_outline,
                  PAGER=pager(prev_u and prev_item, next_u and next_item, root,
                                  "units/unit%s.html" % pad(prev_u["n"]) if prev_u else "",
                                  "units/unit%s.html" % pad(next_u["n"]) if next_u else ""))
    bc = breadcrumb([("index.html", "首页"), (chapter_url(ch), "%s %s" % (ch["id"], ch["title"])),
                     (None, "单元 %s %s" % (dn, book.display_title(u)))], root)
    page(book, out, "units/unit%s.html" % pad(n), kind="unit",
         title="%s %s · 从集合论到实分析" % (dn, book.display_title(u)),
         description="%s：导语、定义与定理、练习与解答。%s" % (u["title"], u["title_en"]),
         body=body, breadcrumb_html=bc, base_url=base_url,
         active_chapter=ch["id"], active_unit=n)


def unit_card(book: Book, u: dict) -> str:
    n = u["n"]
    pn = book.display_n(u)
    return ('<li><a class="unit-jump" href="#unit-%s"><span class="num">%s</span>'
            '<span>%s</span></a></li>' % (n, pn, html_mod.escape(book.display_title(u))))


def chapter_meta(book: Book, ch: dict) -> tuple[str, dict]:
    units = ch["units"]
    unit_ns = {u["n"] for u in units}
    decls = sum(1 for e in book.index if e.get("u") in unit_ns
                and e.get("k") in ("def", "theorem", "inductive", "axiom", "exercise"))
    ex = sum(book.unit_meta.get(u["n"], {}).get("ex", 0) for u in units)
    hints = sum(book.unit_meta.get(u["n"], {}).get("hints", 0) for u in units)
    text = "%d 单元 · %d 道练习 · %d 个声明 · %d 条提示" % (len(units), ex, decls, hints)
    return text, {"units": len(units), "ex": ex, "decls": decls, "hints": hints}


def chapter_unit_html(book: Book, u: dict, ch: dict, root: str) -> str:
    n = u["n"]
    block, _ = rewrite_links(book.unit_html[n], book, root, n, ch)
    block = readable_math(block)
    # Same-page chapter: only this chapter's units become in-page anchors;
    # links to units in other chapters stay normal page links.
    chapter_ns = {x["n"] for x in ch["units"]}
    def _unit_link(m):
        target = int(m.group(1))
        if target in chapter_ns:
            return 'href="#unit-%d"' % target
        return 'href="../units/unit%s.html"' % pad(target)
    block = re.sub(r'href="(?:../)?units/unit(\d+)\.html"', _unit_link, block)
    block = re.sub(r'<nav class="unit-nav.*?</nav>', '', block, flags=re.S)
    block = block.replace('href="#top"', 'href="#unit-%d"' % n)
    appendix = ""
    if n in book.appendix_html:
        appendix, _ = rewrite_links(book.appendix_html[n], book, root, n, ch)
        appendix = readable_math(appendix)
        appendix = '<section class="unit-appendix" aria-label="练习与解答">%s</section>' % appendix
    return ('<div class="chapter-unit" data-unit="%s">%s%s</div>' % (n, block, appendix))


def build_chapter(book: Book, out: Path, ch: dict, base_url: str) -> None:
    root = ".."
    meta_text, counts = chapter_meta(book, ch)
    lede = ("本章共 %d 单元，从「%s」到「%s」。本章所有单元在同一页，按顺序向下读；"
            "点上方单元可以直接滑到对应位置。" % (len(ch["units"]), book.display_title(ch["first_unit"]),
                             book.display_title(ch["last_unit"])))
    cards = "\n".join(unit_card(book, u) for u in ch["units"])
    chapter_body = "\n".join(chapter_unit_html(book, u, ch, root) for u in ch["units"])
    outline_items = "\n".join(
        '<li><a href="#unit-%s" data-outline-unit="%s"><span>%s</span> %s</a></li>' % (
            u["n"], u["n"], book.display_n(u), html_mod.escape(book.display_title(u)))
        for u in ch["units"])
    prev_item = {"label": ("%s %s" % (ch["prev"]["id"], ch["prev"]["title"])) if ch.get("prev") else ""}
    next_item = {"label": ("%s %s" % (ch["next"]["id"], ch["next"]["title"])) if ch.get("next") else ""}
    body = render("chapter.html",
                  VOLUME_TITLE=html_mod.escape(ch["volume_title"]),
                  CHAPTER_INDEX=str(ch["index"]), CHAPTER_ID=ch["id"],
                  CHAPTER_TITLE=html_mod.escape(ch["title"]),
                  CHAPTER_META=html_mod.escape(meta_text),
                  CHAPTER_LEDE=html_mod.escape(lede),
                  CHAPTER_UNITS=str(len(ch["units"])),
                  UNIT_CARDS=cards, CHAPTER_BODY=chapter_body, OUTLINE_ITEMS=outline_items,
                  PAGER=pager(ch.get("prev") and prev_item, ch.get("next") and next_item,
                                     root, chapter_url(ch["prev"]) if ch.get("prev") else "",
                                     chapter_url(ch["next"]) if ch.get("next") else ""))
    bc = breadcrumb([("index.html", "首页"), (None, "%s %s" % (ch["id"], ch["title"]))], root)
    page(book, out, chapter_url(ch), kind="chapter",
         title="%s %s · 从集合论到实分析" % (ch["id"], ch["title"]),
         description="%s：%d 单元。" % (ch["title"], len(ch["units"])),
         body=body, breadcrumb_html=bc, base_url=base_url, active_chapter=ch["id"])


def build_cover(book: Book, out: Path, base_url: str) -> None:
    root = "."
    cover, unresolved = rewrite_links(book.cover, book, root, None, None)
    cover = cover.replace("#unit-1", rel(root, "units/unit01.html"))
    cover = cover.replace("#toc", "#path")
    path_items = []
    for ch in book.chapters:
        meta_text, counts = chapter_meta(book, ch)
        path_items.append(
            '<li><a href="%s"><span class="path-num">%s</span>%s</a>'
            '<span class="path-meta">%s · 从「%s」到「%s」</span></li>' % (
                rel(root, chapter_url(ch)), ch["id"], html_mod.escape(ch["title"]),
                html_mod.escape(meta_text),
                html_mod.escape(book.display_title(ch["first_unit"])),
                html_mod.escape(book.display_title(ch["last_unit"]))))
    volume_links = book_volume_cards(book)
    body = render("cover.html", COVER_HEADER=cover,
                  PATH_ITEMS="\n".join(path_items),
                  SEARCH_COUNT=str(len(book.index)))
    page(book, out, "index.html", kind="cover",
         title="从集合论到实分析 · 交互读本 v3",
         description="卷 II 数系与实数：7 章 72 单元，多页读本。",
         body=body, base_url=base_url)


def search_index_for_site(book: Book) -> list[dict]:
    out = []
    chapter_by_slug = {c["id"]: c for c in book.chapters}
    for e in book.index:
        k, a = e.get("k"), e.get("a", "")
        u = e.get("u")
        item = {"t": e.get("t", ""), "k": k, "a": a}
        if k == "chapter" and a.startswith("chapter-ii-"):
            item["url"] = "chapters/ii-%s.html" % a.rsplit("-", 1)[-1]
            item["ctx"] = "章"
        elif k == "unit" and isinstance(u, int):
            item["url"] = "units/unit%s.html" % pad(u)
            item["ctx"] = "单元 %s · %s" % (pad(u), book.by_id[str(u)]["chapter"]["id"])
        elif isinstance(u, int):
            item["url"] = "units/unit%s.html#%s" % (pad(u), a)
            item["ctx"] = "单元 %s %s%s" % (pad(u), book.display_title(book.by_id[str(u)]),
                                           (" · " + e["h"]) if e.get("h") else "")
        else:
            continue
        out.append(item)
    return out


def build_search(book: Book, out: Path, base_url: str) -> None:
    idx = search_index_for_site(book)
    payload = json.dumps(idx, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
    body = render("search.html", SEARCH_COUNT=str(len(idx)), SEARCH_INDEX_JSON=payload)
    page(book, out, "search.html", kind="search", title="搜索 · 从集合论到实分析",
         description="全书 %d 条索引：章、单元、声明、练习、正文小标题。" % len(idx),
         body=body, base_url=base_url)


def build_sitemap(book: Book, out: Path, base_url: str, prints: list[int],
                  print_chapters: list[str]) -> None:
    chapter_links = "\n".join(
        '<li><a href="%s">%s %s</a><span class="note"> · %d 单元</span></li>' % (
            chapter_url(ch), ch["id"], html_mod.escape(ch["title"]), len(ch["units"]))
        for ch in book.chapters)
    units_links = []
    for ch in book.chapters:
        lis = "".join('<li><a href="units/unit%s.html">%s %s</a></li>' % (
            pad(u["n"]), book.display_n(u), html_mod.escape(book.display_title(u))) for u in ch["units"])
        units_links.append('<section><h3><a href="%s">%s %s</a></h3><ul class="sitemap-units">%s</ul></section>'
                           % (chapter_url(ch), ch["id"], html_mod.escape(ch["title"]), lis))
    print_links = []
    for n in prints:
        print_links.append('<li><a href="print/unit%s.html">单元 %s 打印页</a></li>' % (pad(n), pad(n)))
    for cid in print_chapters:
        slug = "ii-" + cid.split(".")[-1]
        print_links.append('<li><a href="print/chapter-%s.html">%s 章打印页</a></li>' % (slug, cid))
    body = render("sitemap.html", CHAPTER_LINKS=chapter_links,
                  UNIT_LINKS="\n".join(units_links), PRINT_LINKS="\n".join(print_links))
    page(book, out, "sitemap.html", kind="sitemap", title="站点地图 · 从集合论到实分析",
         description="全部页面：封面、7 章、72 单元、搜索、打印。",
         body=body, base_url=base_url)
    # sitemap.xml
    urls = ["index.html", "search.html", "sitemap.html"]
    urls += [chapter_url(ch) for ch in book.chapters]
    urls += ["units/unit%s.html" % pad(u["n"]) for u in book.units]
    urls += ["print/unit%s.html" % pad(n) for n in prints]
    urls += ["print/chapter-ii-%s.html" % cid.split(".")[-1] for cid in print_chapters]
    base = (base_url or "").rstrip("/")
    xml = ['<?xml version="1.0" encoding="UTF-8"?>',
           '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">']
    for u in urls:
        loc = (base + "/" + u) if base else u
        xml.append("  <url><loc>%s</loc></url>" % html_mod.escape(loc))
    xml.append("</urlset>")
    (out / "sitemap.xml").write_text("\n".join(xml), encoding="utf-8")


def build_print_unit(book: Book, out: Path, n: int, base_url: str) -> None:
    u = book.by_id[str(n)]
    ch = u["chapter"]
    root = ".."
    block, _ = rewrite_links(book.unit_html[n], book, root, n, ch)
    block = re.sub(r'<div class="unit-actions no-print">.*?</div>', '', block, flags=re.S)
    block = re.sub(r'<a class="back-to-toc"[^>]*>.*?</a>', '', block, flags=re.S)
    block = block.replace("<h2><span class=\"unit-number\">", "<h2><span class=\"unit-number\">", 1)
    appendix = ""
    if n in book.appendix_html:
        appendix, _ = rewrite_links(book.appendix_html[n], book, root, n, ch)
    body_html = block + appendix
    body_html = re.sub(r"<details(\s|>)", r"<details open\1", body_html)
    header = ('<header class="print-head"><p class="kicker">%s %s · 单元 %s</p>'
              '<h1>%s</h1><p class="note">%s</p></header>' % (
                  ch["id"], html_mod.escape(ch["title"]), pad(n),
                  html_mod.escape(book.display_title(u)), html_mod.escape(u["title_en"])))
    body = render("print.html", PRINT_TITLE=html_mod.escape("单元 %s %s" % (pad(n), book.display_title(u))),
                  BACK_URL=rel(root, "units/unit%s.html" % pad(n)), ROOT=root,
                  PRINT_BODY=header + body_html)
    page(book, out, "print/unit%s.html" % pad(n), kind="print",
         title="打印 · 单元 %s %s" % (pad(n), book.display_title(u)), description="干净打印页（A4）。",
         body=body, base_url=base_url)


def build_print_chapter(book: Book, out: Path, ch: dict, base_url: str) -> None:
    root = ".."
    parts = []
    for u in ch["units"]:
        n = u["n"]
        block, _ = rewrite_links(book.unit_html[n], book, root, n, ch)
        block = re.sub(r'<div class="unit-actions no-print">.*?</div>', '', block, flags=re.S)
        block = re.sub(r'<a class="back-to-toc"[^>]*>.*?</a>', '', block, flags=re.S)
        parts.append(block)
        if n in book.appendix_html:
            appendix, _ = rewrite_links(book.appendix_html[n], book, root, n, ch)
            parts.append(appendix)
    html = "\n".join(parts)
    html = re.sub(r"<details(\s|>)", r"<details open\1", html)
    header = ('<header class="print-head"><p class="kicker">%s · 卷 II</p><h1>%s %s</h1>'
              '<p class="note">%d 单元 · 干净打印页（A4）</p></header>' % (
                  ch["id"], ch["id"], html_mod.escape(ch["title"]), len(ch["units"])))
    body = render("print.html", PRINT_TITLE=html_mod.escape("%s %s" % (ch["id"], ch["title"])),
                  BACK_URL=rel(root, chapter_url(ch)), ROOT=root, PRINT_BODY=header + html)
    page(book, out, "print/chapter-ii-%s.html" % ch["id"].split(".")[-1], kind="print",
         title="打印 · %s %s" % (ch["id"], ch["title"]), description="章打印页（A4）。",
         body=body, base_url=base_url)


def build_404(book: Book, out: Path, base_url: str) -> None:
    body = render("404.html", ROOT=".")
    page(book, out, "404.html", kind="404", title="404 · 从集合论到实分析",
         description="页面不存在。", body=body, base_url=base_url)


# ────────────────────────── assets / check ──────────────────────────
def copy_assets(book: Book, out: Path) -> None:
    adir = out / "assets"
    adir.mkdir(parents=True, exist_ok=True)
    for name in ("v3.css", "v3.js"):
        shutil.copyfile(ASSETS / name, adir / name)
    src_css = book.style_css
    if not src_css.exists():
        raise FileNotFoundError(src_css)
    shutil.copyfile(src_css, adir / SITE_CSS_FALLBACK_NAME)
    (out / "robots.txt").write_text("User-agent: *\nAllow: /\n", encoding="utf-8")
    (out / "favicon.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">'
        '<rect width="32" height="32" fill="#1f5e46"/>'
        '<text x="16" y="22" font-size="16" text-anchor="middle" fill="#fff" font-family="serif">∀</text></svg>',
        encoding="utf-8")


def check_site(book: Book, out: Path, prints: list[int], print_chapters: list[str]) -> int:
    errors: list[str] = []
    pages = list(out.glob("**/*.html"))
    if not (out / "index.html").exists():
        errors.append("missing index.html")
    expected = 1 + len(book.chapters) + len(book.units) + 3 + len(prints) + len(print_chapters)
    if len(pages) < expected - 2:
        errors.append("page count %d < expected ~%d" % (len(pages), expected))
    href_re = re.compile(r'href="([^"]+)"')
    checked = 0
    for p in pages:
        text = read_text(p)
        ids = set(re.findall(r'id="([^"]+)"', text))
        for href in href_re.findall(text):
            if href.startswith(("http://", "https://", "mailto:", "javascript:", "data:")):
                continue
            if href.startswith("#"):
                if href[1:] and href[1:] not in ids:
                    errors.append("%s: missing same-page id %s" % (p.name, href))
                continue
            path_part, _, frag = href.partition("#")
            target = (p.parent / path_part).resolve()
            checked += 1
            if not target.exists():
                errors.append("%s: missing target %s" % (p.relative_to(out), href))
                continue
            if frag:
                tids = set(re.findall(r'id="([^"]+)"', read_text(target)))
                if frag not in tids:
                    errors.append("%s: missing anchor %s in %s" % (p.relative_to(out), frag, target.name))
    # search index urls
    idx = search_index_for_site(book)
    for e in idx:
        target = (out / e["url"].partition("#")[0])
        if not target.exists():
            errors.append("search index: missing %s" % e["url"])
    print("pages: %d (cover 1, chapters %d, units %d, search/sitemap/404 3, print %d+%d)" % (
        len(pages), len(book.chapters), len(book.units), len(prints), len(print_chapters)))
    print("internal hrefs checked: %d" % checked)
    if errors:
        print("CHECK FAILED: %d issue(s)" % len(errors))
        for e in errors[:40]:
            print("  -", e)
        return 1
    print("CHECK OK")
    return 0


# ────────────────────────── main ──────────────────────────
def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Build the v3 multi-page book prototype")
    repo_root = HERE.parent.parent
    ap.add_argument("--preview", default=str(repo_root / "dist/book/book-v2/index.html"),
                    help="v2 single-page preview to split (default: dist/book/book-v2/index.html)")
    ap.add_argument("--course-json", default=str(repo_root / "courses/analysis/course.json"))
    ap.add_argument("--style-css", default=str(repo_root / "scripts/book/style.css"))
    ap.add_argument("--out", default=str(repo_root / "dist/book/book-v3"))
    ap.add_argument("--demo", default="", help="comma unit numbers to emit (default: all)")
    ap.add_argument("--print", dest="print_units", default="", help="comma unit numbers for print pages")
    ap.add_argument("--print-chapters", default="", help="comma chapter ids (II.1,...) for print bundles")
    ap.add_argument("--base-url", default="https://example.invalid/book-v3")
    ap.add_argument("--volume-dir", default="volume-ii", help="output sub-directory for this volume")
    ap.add_argument("--volume-label", default="卷 II")
    ap.add_argument("--volume-title", default="数系与实数")
    ap.add_argument("--siblings", default="", help="comma list of other volume dirs that will also be built")
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args(argv)

    preview = Path(args.preview).resolve()
    if not preview.is_file():
        raise SystemExit("preview not found: %s (run scripts/build_book_proto.py first)" % preview)
    out = Path(args.out).resolve()
    book = Book(Path(args.course_json).resolve(), preview, Path(args.style_css).resolve())
    CURRENT_VOLUME[0] = args.volume_dir
    CURRENT_VOLUME_META.clear()
    CURRENT_VOLUME_META.update({"label": args.volume_label, "title": args.volume_title})
    book.siblings = [(args.volume_dir, args.volume_label, args.volume_title)]
    for other in [x.strip() for x in args.siblings.split(",") if x.strip()]:
        book.siblings.append((other, other.replace("volume-", "卷 ").upper().replace("VOLUME-", ""), ""))
    demo = [int(x) for x in args.demo.split(",") if x.strip()] or [u["n"] for u in book.units]
    prints = [int(x) for x in args.print_units.split(",") if x.strip()] or [u["n"] for u in book.units]
    pch = [x.strip() for x in args.print_chapters.split(",") if x.strip()]
    chapter_by_id = {c["id"]: c for c in book.chapters}

    if args.check:
        return check_site(book, out, prints, pch)

    # clean generated html (keep dist dir itself)
    for p in out.glob("**/*.html"):
        p.unlink()
    if (out / "sitemap.xml").exists():
        (out / "sitemap.xml").unlink()

    copy_assets(book, out)
    build_cover(book, out, args.base_url)
    for ch in book.chapters:
        build_chapter(book, out, ch, args.base_url)
    for n in demo:
        build_unit(book, out, book.by_id[str(n)], args.base_url)
    build_search(book, out, args.base_url)
    build_sitemap(book, out, args.base_url, prints, pch)
    for n in prints:
        build_print_unit(book, out, n, args.base_url)
    for cid in pch:
        if cid not in chapter_by_id:
            raise SystemExit("unknown chapter id: %s" % cid)
        build_print_chapter(book, out, chapter_by_id[cid], args.base_url)
    build_404(book, out, args.base_url)

    print("built: %s" % out)
    print("  cover index.html; chapters: %d; units emitted: %d/%d; print pages: %d + %d chapter bundles" % (
        len(book.chapters), len(demo), len(book.units), len(prints), len(pch)))
    print("  search entries: %d" % len(search_index_for_site(book)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
