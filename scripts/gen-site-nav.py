#!/usr/bin/env python3
"""Navigation / footer single-source generator and drift checker (D2 §3.2).

`site/_partials/header.html` and `site/_partials/footer.html` are the **only**
source for the two blocks that appear on every page. Each page carries them
between markers::

    <!--#nav-->   … header block …   <!--/#nav-->
    <!--#footer--> … footer block …  <!--/#footer-->

    python3 scripts/gen-site-nav.py --check   # report drift, write nothing, exit 1
    python3 scripts/gen-site-nav.py --write   # rewrite every page's blocks

The active page is marked **without JavaScript**: `--write` puts
`aria-current="page"` on the nav link whose `data-nav` equals the page's
`<body data-page="…">`, and strips it from every other link. Running `--write`
twice is a no-op the second time (idempotent by construction: the block is
rendered deterministically and compared before writing).

Page ids come from the path (`kernel.html` → `kernel`, `en/index.html` → `en`),
so there is no second hand-maintained map to drift. A page with no `data-page`
is a loud error — except during bootstrap, when the page has no markers yet and
`--write` can infer the id from its path (it prints what it inferred, and
`--check` never infers). A `data-page` with no matching `data-nav` is always a
loud error naming the file.

Python stdlib only. Exit codes: 0 ok, 1 drift/error, 2 usage.
"""

from __future__ import annotations

import argparse
import difflib
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_SITE = REPO / "site"

# kind -> (start marker, end marker, partial file name)
BLOCKS: dict[str, tuple[str, str, str]] = {
    "header": ("<!--#nav-->", "<!--/#nav-->", "header.html"),
    "footer": ("<!--#footer-->", "<!--/#footer-->", "footer.html"),
}
PARTIALS_DIR = "_partials"

# The one active-page normalisation rule (check-site.py reuses these).
ARIA_CURRENT_RE = re.compile(r'\s+aria-current="page"')
NAV_ATTR_RE = re.compile(r'\bdata-nav="([^"]*)"')
PAGE_ATTR_RE = re.compile(r'\bdata-page="([^"]*)"')
TAG_A_RE = re.compile(r"<a\b[^>]*>")
BODY_TAG_RE = re.compile(r"<body\b[^>]*>")
URL_ATTR_RE = re.compile(r'\b(href|src)="([^"]*)"')
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")

# Whole elements used to bootstrap pages that predate the markers.
HEADER_REGION_RE = re.compile(
    r'(?:[ \t]*<a\b[^>]*class="[^"]*\bskip-link\b[^"]*"[^>]*>.*?</a>[ \t]*\r?\n)?'
    r"[ \t]*<header\b.*?</header>",
    re.S,
)
FOOTER_REGION_RE = re.compile(r"[ \t]*<footer\b.*?</footer>", re.S)


class NavError(Exception):
    """Loud, file-naming failure — never a silent skip."""


# --------------------------------------------------------------------------
# pages and partials
# --------------------------------------------------------------------------


def site_pages(site: Path) -> list[Path]:
    """Every published page, sorted. `_partials/` is the source, not a page."""
    return sorted(
        path
        for path in site.rglob("*.html")
        if PARTIALS_DIR not in path.relative_to(site).parts
    )


def rel_posix(site: Path, path: Path) -> str:
    return path.relative_to(site).as_posix()


def depth_of(rel: str) -> int:
    """Directory depth of a page relative to the site root (`en/index.html` → 1)."""
    return rel.count("/")


def derive_page_id(rel: str) -> str:
    """Page id from its path: `kernel.html` → `kernel`, `en/index.html` → `en`."""
    parts = rel.split("/")
    if parts[-1] == "index.html":
        return parts[-2] if len(parts) > 1 else "index"
    return parts[-1][: -len(".html")]


def load_partials(site: Path) -> dict[str, str]:
    """Read the two partials; the block content excludes the final newline."""
    partials: dict[str, str] = {}
    for kind, (_, _, name) in BLOCKS.items():
        path = site / PARTIALS_DIR / name
        if not path.is_file():
            raise NavError(
                f"{path}: missing partial — it is the single source for the {kind} block"
            )
        text = path.read_text(encoding="utf-8")
        if not text.endswith("\n") or text.endswith("\n\n"):
            raise NavError(
                f"{path}: partial must end with exactly one newline "
                "(the generated block is compared byte-for-byte)"
            )
        # A marker literal inside the partial (even in a comment) would make every
        # extraction stop early and silently duplicate the rest of the block.
        for marker in BLOCKS[kind][:2]:
            if marker in text:
                raise NavError(
                    f"{path}: partial must not contain the marker literal {marker!r} "
                    "— it would truncate every generated block"
                )
        partials[kind] = text[:-1]
    return partials


def nav_ids(partials: dict[str, str]) -> set[str]:
    """Every `data-nav` value the partials define."""
    return {
        match.group(1)
        for text in partials.values()
        for match in NAV_ATTR_RE.finditer(text)
    }


# --------------------------------------------------------------------------
# rendering (the single implementation of the block + marking rule)
# --------------------------------------------------------------------------


def rewrite_paths(block: str, depth: int, root_absolute: bool = False) -> str:
    """Prefix site-root-relative `href`/`src` values for a page at `depth`.

    `root_absolute` is for pages served at an **arbitrary** depth: only 404.html
    qualifies (GitHub Pages serves it for any unknown URL, so a relative
    `assets/…` resolves against whatever the visitor typed and 404s again).
    Those pages declare `<body data-root="/">` and get `/`-prefixed links
    instead of depth-relative ones.
    """
    if root_absolute:
        def fix_abs(match: re.Match[str]) -> str:
            attr, value = match.group(1), match.group(2)
            if value.startswith(("#", "/")) or SCHEME_RE.match(value):
                return match.group(0)
            return f'{attr}="/{value}"'

        return URL_ATTR_RE.sub(fix_abs, block)

    if depth == 0:
        return block

    def fix(match: re.Match[str]) -> str:
        attr, value = match.group(1), match.group(2)
        if value.startswith("#") or value.startswith("/") or SCHEME_RE.match(value):
            return match.group(0)
        return f'{attr}="{"../" * depth}{value}"'

    return URL_ATTR_RE.sub(fix, block)


def mark_current(block: str, page_id: str) -> str:
    """Put `aria-current="page"` on the link whose `data-nav` is `page_id` only."""

    def fix(match: re.Match[str]) -> str:
        tag = ARIA_CURRENT_RE.sub("", match.group(0))
        nav = NAV_ATTR_RE.search(tag)
        if nav and page_id and nav.group(1) == page_id:
            tag = tag[: nav.end()] + ' aria-current="page"' + tag[nav.end() :]
        return tag

    return TAG_A_RE.sub(fix, block)


def is_root_absolute(source: str) -> bool:
    """`<body data-root="/">` — the page is served at an arbitrary depth."""
    body = BODY_TAG_RE.search(source)
    if body is None:
        return False
    match = re.search(r'\bdata-root="([^"]*)"', body.group(0))
    return bool(match and match.group(1) == "/")


def render_block(partial: str, rel: str, page_id: str, root_absolute: bool = False) -> str:
    """The exact block content a page must carry, for its own depth and page id."""
    return mark_current(rewrite_paths(partial, depth_of(rel), root_absolute), page_id)


def normalize_block(text: str) -> str:
    """Drop active-page marking — the one comparison rule, shared with check-site.py."""
    return ARIA_CURRENT_RE.sub("", text)


def extract_block(source: str, start: str, end: str) -> str | None:
    """Raw text between the markers (exclusive), or None when a marker is absent."""
    begin = source.find(start)
    if begin < 0:
        return None
    finish = source.find(end, begin + len(start))
    if finish < 0:
        return None
    return source[begin + len(start) : finish]


def describe_diff(actual: str, expected: str, limit: int = 4) -> str:
    """A short, actionable summary of how a block drifted."""
    lines = [
        line
        for line in difflib.unified_diff(
            normalize_block(actual).splitlines(),
            normalize_block(expected).splitlines(),
            lineterm="",
            n=0,
        )
        if not line.startswith(("---", "+++", "@@"))
    ]
    head = "; ".join(line.strip() for line in lines[:limit])
    tail = "" if len(lines) <= limit else f"; …(+{len(lines) - limit} more diff line(s))"
    return f"first differences: {head}{tail}"


# --------------------------------------------------------------------------
# assertions shared with scripts/check-site.py
# --------------------------------------------------------------------------


def block_problems(
    site: Path, rel: str, source: str, partials: dict[str, str]
) -> list[str]:
    """Marker presence + byte equality with the partials (aria-current normalised)."""
    problems: list[str] = []
    for kind, (start, end, name) in BLOCKS.items():
        expected = f"\n{render_block(partials[kind], rel, '', is_root_absolute(source))}\n"
        actual = extract_block(source, start, end)
        if actual is None:
            problems.append(
                f"{rel}: missing {start} … {end} markers around the {kind} block "
                "(run: python3 scripts/gen-site-nav.py --write)"
            )
        elif normalize_block(actual) != normalize_block(expected):
            problems.append(
                f"{rel}: {kind} block drifted from site/{PARTIALS_DIR}/{name} — "
                + describe_diff(actual, expected)
            )
    return problems


def declared_page_id(source: str) -> str | None:
    """The `data-page` value on `<body>`, or None."""
    body = BODY_TAG_RE.search(source)
    if body is None:
        return None
    match = PAGE_ATTR_RE.search(body.group(0))
    return match.group(1) if match else None



def nav_marked_tags(source: str) -> list[str]:
    """`<a>` tags carrying aria-current **inside the managed nav/footer blocks**.

    Scoped deliberately. `aria-current="page"` is also the correct WAI-ARIA
    markup for the last item of a breadcrumb, and pages legitimately use it
    there — flagging those was a false positive (found on `glossary.html`).
    What this checker owns is the *navigation*: exactly one nav link, the one
    matching `data-page`, must be marked, with no JavaScript involved.
    """
    tags: list[str] = []
    for kind, (start, end, _name) in BLOCKS.items():
        block = extract_block(source, start, end)
        if block is None:
            continue
        tags += [tag for tag in TAG_A_RE.findall(block) if ARIA_CURRENT_RE.search(tag)]
    return tags


def page_id_problems(
    rel: str, source: str, partials: dict[str, str]
) -> list[str]:
    """`data-page` exists, is a real nav target, and owns the aria-current marking."""
    problems: list[str] = []
    declared = declared_page_id(source)
    if not declared:
        return [
            f'{rel}: <body> has no data-page (expected data-page="{derive_page_id(rel)}"; '
            "run: python3 scripts/gen-site-nav.py --write)"
        ]

    # `data-page="none"` 是显式的退出标记（只有 404.html 用）：它带着导航块，
    # 但**不该**把任何一项标成当前页——它的 URL 是任意的，标谁都是假话。
    # 退出必须是写下来的，缺属性仍然报错（那与"忘了"无法区分）。
    if declared == "none":
        marked = [tag for tag in TAG_A_RE.findall(source) if ARIA_CURRENT_RE.search(tag)]
        if marked:
            problems.append(
                f'{rel}: data-page="none" but the page marks a nav link as current — '
                "an opt-out page must not claim any item is the current one"
            )
        return problems

    if declared not in nav_ids(partials):
        problems.append(
            f'{rel}: data-page="{declared}" matches no nav link with '
            f'data-nav="{declared}" in site/{PARTIALS_DIR}/ (add one, or fix data-page)'
        )

    marked = nav_marked_tags(source)
    if not marked:
        problems.append(
            f'{rel}: no aria-current="page" in the nav/footer blocks — the current nav link '
            "must be marked in the HTML (no JavaScript)"
        )
    for tag in marked:
        nav = NAV_ATTR_RE.search(tag)
        if nav is None:
            problems.append(
                f'{rel}: aria-current="page" on a nav link without data-nav: {tag}'
            )
        elif nav.group(1) != declared:
            problems.append(
                f'{rel}: aria-current="page" sits on data-nav="{nav.group(1)}" '
                f'but <body data-page="{declared}">'
            )
    return problems


# --------------------------------------------------------------------------
# check / write
# --------------------------------------------------------------------------


def check(site: Path) -> int:
    try:
        partials = load_partials(site)
    except NavError as exc:
        print(f"site nav: {exc}")
        return 1

    pages = site_pages(site)
    if not pages:
        print(f"site nav: no pages under {site}")
        return 1

    problems: list[str] = []
    for path in pages:
        rel = rel_posix(site, path)
        source = path.read_text(encoding="utf-8")
        problems += block_problems(site, rel, source, partials)
        problems += page_id_problems(rel, source, partials)

    if problems:
        print(f"site nav: {len(problems)} problem(s) in {len(pages)} page(s)")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print(
        f"site nav: ok ({len(pages)} pages, header+footer match "
        f"site/{PARTIALS_DIR}/, aria-current consistent)"
    )
    return 0


def plan_page(
    site: Path, path: Path, partials: dict[str, str]
) -> tuple[str | None, list[str], list[str]]:
    """Return (new source, change list, error list) for one page."""
    rel = rel_posix(site, path)
    source = path.read_text(encoding="utf-8")
    errors: list[str] = []
    changes: list[str] = []

    managed = all(extract_block(source, *BLOCKS[k][:2]) is not None for k in BLOCKS)
    declared = declared_page_id(source)
    if declared is None:
        # `data-page="none"` is the **explicit** opt-out: the page carries the
        # nav/footer blocks but no item may be marked current. Only 404.html
        # uses it (its URL is arbitrary, so marking anything would be a lie).
        # It must be written down — a *missing* attribute is still an error,
        # because that is indistinguishable from an oversight.
        if managed and not is_root_absolute(source):
            errors.append(
                f"{rel}: <body> has no data-page — refusing to guess for a page that "
                "already carries the markers (never a silent skip). Write "
                'data-page="none" to opt out explicitly, or name the nav item.'
            )
            return None, [], errors
        if managed:
            declared = "none"
        else:
            declared = derive_page_id(rel)
            changes.append(f'bootstrap data-page="{declared}"')

    if declared != "none" and declared not in nav_ids(partials):
        errors.append(
            f'{rel}: data-page="{declared}" matches no nav link with '
            f'data-nav="{declared}" in site/{PARTIALS_DIR}/ — add the link or fix data-page'
        )
        return None, [], errors

    body = BODY_TAG_RE.search(source)
    if body is None:
        errors.append(f"{rel}: no <body> element to carry data-page")
        return None, [], errors
    if PAGE_ATTR_RE.search(body.group(0)) is None:
        source = (
            source[: body.end() - 1]
            + f' data-page="{declared}">'
            + source[body.end() :]
        )

    for kind, (start, end, _) in BLOCKS.items():
        rendered = render_block(partials[kind], rel, declared, is_root_absolute(source))
        wanted = f"{start}\n{rendered}\n{end}"
        begin = source.find(start)
        if begin >= 0:
            finish = source.find(end, begin + len(start))
            if finish < 0:
                errors.append(f"{rel}: {start} without {end} (half-written markers)")
                return None, [], errors
            finish += len(end)
            if source[begin:finish] == wanted:
                continue
            source = source[:begin] + wanted + source[finish:]
        else:
            region_re = HEADER_REGION_RE if kind == "header" else FOOTER_REGION_RE
            region = region_re.search(source)
            if region is None:
                errors.append(
                    f"{rel}: no {start} … {end} markers and no <{kind}> element to "
                    "replace — author the block with markers (D2 §3.1)"
                )
                return None, [], errors
            source = source[: region.start()] + wanted + source[region.end() :]
        changes.append(kind)

    return source, changes, errors


def write(site: Path) -> int:
    try:
        partials = load_partials(site)
    except NavError as exc:
        print(f"site nav: {exc}")
        return 1

    pages = site_pages(site)
    if not pages:
        print(f"site nav: no pages under {site}")
        return 1

    planned: list[tuple[Path, str, list[str]]] = []
    errors: list[str] = []
    for path in pages:
        new_source, changes, page_errors = plan_page(site, path, partials)
        errors += page_errors
        if new_source is not None and changes:
            planned.append((path, new_source, changes))

    if errors:
        print(f"site nav: {len(errors)} error(s) — nothing written")
        for error in errors:
            print(f"  - {error}")
        return 1

    for path, new_source, changes in planned:
        path.write_text(new_source, encoding="utf-8")
        print(f"  wrote {rel_posix(site, path)}: {', '.join(changes)}")

    unchanged = len(pages) - len(planned)
    if not planned:
        print(f"site nav: ok ({len(pages)} pages, already in sync)")
    else:
        print(f"site nav: {len(planned)} page(s) written, {unchanged} unchanged")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="gen-site-nav.py",
        description="Single-source navigation/footer generator and drift checker.",
    )
    parser.add_argument(
        "--site",
        type=Path,
        default=DEFAULT_SITE,
        help="site root (default: <repo>/site)",
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="report drift, write nothing")
    mode.add_argument("--write", action="store_true", help="rewrite every page's blocks")
    args = parser.parse_args(argv)

    if not (args.check or args.write):
        parser.print_usage(sys.stderr)
        print("gen-site-nav.py: pick one mode: --check or --write", file=sys.stderr)
        return 2
    if not args.site.is_dir():
        print(f"gen-site-nav.py: --site {args.site} is not a directory", file=sys.stderr)
        return 2

    return check(args.site) if args.check else write(args.site)


if __name__ == "__main__":
    sys.exit(main())
