#!/usr/bin/env python3
"""Build `site/data/search.json` — the static search index for `search.html`.

Why a generated index instead of a search service: the site is zero-build static
on GitHub Pages. A JSON index plus client-side filtering is the only option that
adds no backend, no third-party request and no runtime dependency — and the whole
site is under 30 pages, so a linear scan in the browser is instant.

What gets indexed: every page's title and description, plus one entry per
heading section (`h2`/`h3`) carrying that section's opening text. Section-level
entries are what make search useful on long pages — a hit that says "diagnostics
.html §4" beats a hit that says "diagnostics.html".

Python stdlib only. Deterministic output. Run after adding or editing pages:

    python3 scripts/gen-site-search.py
    python3 scripts/gen-site-search.py --check   # fail if the index is stale

**`--check` 比的是"索引有没有反映页面文字"，不是"记的提交号是不是 HEAD"**：
载荷里带 `source_commit`（出处，有用），但**一旦把它一起比，这个判据永远不可能通过**
——生成索引 → 提交 → HEAD 变了 → 重建出的载荷与文件里的不一致。这是"给自己拍一张
带自己哈希的照片"。实测踩过：索引在 `702e444` 生成、随 `4afe42a` 提交，CI 在
`4afe42a` 上重算得到 `4afe42a` ⇒ `--check` 必红 ⇒ 部署挂掉。
所以比较用的规范形式把 `source_commit` 抹掉（`_payload(..., provenance=False)`），
写盘时仍然带上。

Exit codes: 0 ok · 1 stale (with --check) · 2 usage.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import html.parser
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
OUT = SITE / "data" / "search.json"

# 每个小节摘多少字。太短检索不到，太长索引就白大了——索引**整份**随 search.html
# 一起下载，所以它是唯一一个体积随页数线性增长的产物。
# 中文在 UTF-8 里是**每字 3 字节**，所以这份索引的大小基本就是摘要字数决定的：
#   260 字 → 14 页 61.9 KB（推 28 页约 120 KB，比任何单页都大）
#   150 字 → 50.5 KB
#    90 字 → 约 35 KB
# 命中主要靠标题（它们本身写得很具体）与页面描述，摘要只用来判断相关性，
# 90 字（中文约一行半）够用。
SECTION_CHARS = 90
# 整页没有小节标题时，用开头这段做条目。
LEAD_CHARS = 200

SKIP_IDS = {"main"}


class SectionScanner(html.parser.HTMLParser):
    """Split one page into (heading, id, text) sections.

    Line-oriented extraction would be simpler but wrong: the templates put
    `<span class="sec-num">` inside headings and wrap text across lines, so a
    real parser is the only way to get the heading text and the section body
    without dragging markup into the index.
    """

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.title = ""
        self.description = ""
        self.h1 = ""
        self.sections: list[dict[str, str]] = []
        self._heading_stack: list[str] = []
        self._current: dict[str, str] | None = None
        self._buffer: list[str] = []
        self._capture: str | None = None
        self._skip_depth = 0
        self._in_title = False

    # -- helpers -----------------------------------------------------------
    def _flush(self) -> None:
        if self._current is None:
            return
        text = re.sub(r"\s+", " ", html.unescape("".join(self._buffer))).strip()
        self._current["text"] = text[:SECTION_CHARS]
        if self._current["text"] or self._current["heading"]:
            self.sections.append(self._current)
        self._current = None
        self._buffer = []

    # -- parser hooks ------------------------------------------------------
    def handle_starttag(self, tag: str, attrs) -> None:
        a = dict(attrs)
        if tag in {"script", "style", "nav", "footer", "header"}:
            self._skip_depth += 1
            return
        if tag == "title":
            self._in_title = True
        elif tag == "meta" and a.get("name") == "description":
            self.description = a.get("content", "")
        elif tag in {"h2", "h3"}:
            self._flush()
            self._capture = tag
            self._buffer = []
            self._current = {"heading": "", "id": a.get("id", ""), "level": tag, "text": ""}
        elif tag == "h1":
            self._capture = "h1"
            self._buffer = []

    def handle_endtag(self, tag: str) -> None:
        if tag in {"script", "style", "nav", "footer", "header"}:
            self._skip_depth = max(0, self._skip_depth - 1)
            return
        if tag == "title":
            self._in_title = False
        elif tag in {"h2", "h3"} and self._capture == tag:
            text = re.sub(r"\s+", " ", "".join(self._buffer)).strip()
            if self._current is not None:
                self._current["heading"] = text
            self._capture = None
            self._buffer = []
        elif tag == "h1" and self._capture == "h1":
            self.h1 = re.sub(r"\s+", " ", "".join(self._buffer)).strip()
            self._capture = None
            self._buffer = []

    def handle_data(self, data: str) -> None:
        if self._in_title:
            self.title += data
        if self._skip_depth:
            return
        if self._capture:
            self._buffer.append(data)
        elif self._current is not None:
            self._buffer.append(data)

    def close(self) -> None:  # noqa: D102
        super().close()
        self._flush()


def scan_page(path: Path) -> dict:
    scanner = SectionScanner()
    scanner.feed(path.read_text(encoding="utf-8"))
    scanner.close()

    rel = path.relative_to(SITE).as_posix()
    entries: list[dict[str, str]] = []

    # 页面自身的条目：标题 + 描述，这样搜"内核判卷"能命中页面而不是只命中小节。
    entries.append(
        {
            "page": rel,
            "anchor": "",
            "heading": scanner.h1 or scanner.title,
            "text": re.sub(r"\s+", " ", scanner.description).strip(),
        }
    )
    for section in scanner.sections:
        heading = section["heading"].strip()
        if not heading:
            continue
        entries.append(
            {
                "page": rel,
                "anchor": section["id"],
                "heading": heading,
                "text": section["text"],
            }
        )

    return {
        "page": rel,
        "title": re.sub(r"\s+", " ", scanner.title).strip(),
        "description": re.sub(r"\s+", " ", scanner.description).strip(),
        "entries": entries,
    }


def build() -> dict:
    pages = sorted(p for p in SITE.rglob("*.html") if "_partials" not in p.parts)
    records = [scan_page(p) for p in pages]

    commit = ""
    try:
        commit = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=ROOT, capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        commit = "unknown"

    version = ""
    data = SITE / "data" / "site.json"
    if data.is_file():
        try:
            version = json.loads(data.read_text(encoding="utf-8")).get("version", "")
        except json.JSONDecodeError:
            version = ""

    return {
        "version": version,
        "source_commit": commit,
        "page_count": len(records),
        "entry_count": sum(len(r["entries"]) for r in records),
        "pages": records,
    }


def _payload(index: dict, *, provenance: bool) -> str:
    """Canonical JSON for the index.

    紧凑输出：这份索引**整份**随 search.html 下载，缩进纯属浪费。
    实测 indent=1 → 53.8 KB，紧凑 → 约 40 KB（同一份内容）。

    `provenance=False` 抹掉 `source_commit`，**只给比较用**：那个字段是"索引是哪次
    提交生成的"，而提交动作本身会改 HEAD ⇒ 带着它比较，判据永远不可能通过
    （详见模块 docstring 的实测记录）。写盘时照旧带上。
    """
    data = dict(index)
    if not provenance:
        data["source_commit"] = ""
    return json.dumps(data, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the index is stale")
    args = parser.parse_args()

    index = build()
    payload = _payload(index, provenance=True)
    digest = hashlib.sha256(payload.encode("utf-8")).hexdigest()[:12]

    if args.check:
        if not OUT.is_file():
            print("site-search: index missing — run python3 scripts/gen-site-search.py")
            return 1
        try:
            existing = json.loads(OUT.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            print("site-search: index is not JSON — run python3 scripts/gen-site-search.py")
            return 1
        if _payload(existing, provenance=False) != _payload(index, provenance=False):
            print("site-search: index is stale — run python3 scripts/gen-site-search.py")
            return 1
        print(f"site-search: ok ({index['page_count']} pages, {index['entry_count']} entries)")
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(payload, encoding="utf-8")
    size = len(payload.encode("utf-8"))
    print(
        f"site-search: wrote {OUT.relative_to(ROOT)} — "
        f"{index['page_count']} pages, {index['entry_count']} entries, "
        f"{size / 1024:.1f} KB, sha256:{digest}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
