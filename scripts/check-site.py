#!/usr/bin/env python3
"""Site hygiene checks (run in CI before deploying to GitHub Pages).

Two guards, both born from this repo's documentation-drift history
(docs/design/site.md §5.3):

1. **Links**: every in-site `href` / `src` target must exist on disk, and
   every repo-relative `docs/...` / `course/...` link must point at a real
   file (mirrors `crates/cli/tests/skill.rs`'s
   `skill_referenced_repo_paths_exist`).
2. **No hardcoded versions**: `site/**/*.html` must not contain a literal
   `0.x.y` version. Version numbers come from `site/data/site.json` and the
   GitHub Releases API at view time — hardcoding them is how `README.md`
   ended up advertising `V=0.9.0` while the repo was at 0.17.0.

Python stdlib only, so the site pipeline stays zero-build.
"""

from __future__ import annotations

import html.parser
import re
import sys
from pathlib import Path

SITE = Path(__file__).resolve().parent.parent / "site"
VERSION_RE = re.compile(r"\b0\.\d+\.\d+\b")
REPO = SITE.parent


class TagScanner(html.parser.HTMLParser):
    """Collect href/src values and fail loudly on malformed markup."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.refs: list[str] = []

    def handle_starttag(self, tag: str, attrs) -> None:
        for name, value in attrs:
            if name in {"href", "src"} and value:
                self.refs.append(value)


def html_files() -> list[Path]:
    return sorted(SITE.rglob("*.html"))


def resolve_ref(page: Path, ref: str) -> Path | None:
    """Resolve a relative href/src against the page's own directory.

    `None` for external URLs, pure anchors, and links that escape the repo
    (those are someone else's problem — and a path-traversal smell).
    """
    if ref.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path = ref.split("#", 1)[0]
    if not path:
        return None
    candidate = (page.parent / path).resolve()
    if REPO.resolve() not in candidate.parents:
        return None
    return candidate


def main() -> int:
    failures: list[str] = []
    files = html_files()
    if not files:
        failures.append("site/ has no HTML files at all")

    for path in files:
        rel = path.relative_to(REPO)
        source = path.read_text(encoding="utf-8")

        scanner = TagScanner()
        try:
            scanner.feed(source)
            scanner.close()
        except Exception as exc:  # malformed markup
            failures.append(f"{rel}: HTML parse error: {exc}")
            continue

        for ref in scanner.refs:
            target = resolve_ref(path, ref)
            if target is not None and not target.exists():
                failures.append(f"{rel}: broken link -> {ref}")

        # 版本号一律来自 site.json / Releases API，不许写死。
        if path.name != "site.json":
            for match in VERSION_RE.finditer(source):
                failures.append(
                    f"{rel}: hardcoded version `{match.group(0)}` "
                    "(use site/data/site.json or the Releases API)"
                )

    data = SITE / "data" / "site.json"
    if not data.exists():
        failures.append("site/data/site.json is missing (run gen-site-data.py)")

    if failures:
        print(f"site hygiene: {len(failures)} problem(s)")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print(f"site hygiene: ok ({len(files)} pages, links and versions clean)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
