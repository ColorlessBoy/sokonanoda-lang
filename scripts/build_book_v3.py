#!/usr/bin/env python3
"""Book v3 — build the whole multi-page book from the repository.

Two layers, both from the source tree only (nothing hardcoded by unit count):

  1. `scripts/build_book_proto.py` (v2) renders one self-contained HTML page per
     course: the course's units, exercises, hints, solutions and solutions
     appendix. Its output is a build artifact (`dist/book/book-v2*/`).
  2. `scripts/book_v3/build_site.py` splits that page into fragments and
     re-assembles them as one page per unit, plus chapter/cover/search/print
     pages — the published multi-page reading experience.

Volumes (the book layer, not the course layer):

  卷 I   集合论        courses/set-theory    112 units / 13 chapters
  卷 II  数系与实数    courses/analysis      171 units / 16 chapters (II.1–III.9)

Usage (repository root; needs Python 3 only — no browser, no LaTeX):

    python3 scripts/build_book_v3.py                 # build every volume
    python3 scripts/build_book_v3.py --check         # validate the built tree
    python3 scripts/build_book_v3.py --volume i      # one volume only

Output (gitignored): `dist/book/book-v3/`
  index.html                       the library landing page (two volumes)
  volume-0/index.html              formal-logic introduction
  volume-i/{index,chapters,units,print,search,sitemap,404}.html
  volume-ii/…
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
BOOK_V3 = ROOT / "scripts" / "book_v3"
STATIC = BOOK_V3 / "static"
OUT_DEFAULT = ROOT / "dist" / "book" / "book-v3"

VOLUMES = [
    # (out dir, course dir, label, title)
    #
    # 卷 I（集合论）暂未开启：`courses/set-theory` 的单元文件还过不了 v2 渲染契约
    # （4 处未解析行、2 处 Markdown 标记、I.10–I.13 单元号不连续），需要一轮内容
    # 修订后再加回来。这里先只出版分析课（卷 II/III，171 单元）。
    # ("volume-i", "courses/set-theory", "卷 I", "集合论"),
    ("volume-ii", "courses/analysis", "卷 II", "数系与实数"),
]


def run(cmd: list[str], *, cwd: Path = ROOT) -> int:
    print("$", " ".join(str(c) for c in cmd))
    proc = subprocess.run(cmd, cwd=cwd)
    return proc.returncode


def stage_course(course_dir: Path, stage: Path) -> Path:
    """A throw-away repo-shaped root whose `courses/analysis` is `course_dir`.

    The v2 builder resolves its inputs relative to its own location, so giving it
    a staged root is the least invasive way to render a second course without
    teaching the builder a flag it only needs for one caller.
    """
    (stage / "courses").mkdir(parents=True, exist_ok=True)
    link = stage / "courses" / "analysis"
    if link.exists() or link.is_symlink():
        link.unlink() if link.is_symlink() else shutil.rmtree(link)
    # A real copy, not a symlink: the v2 builder calls `relative_to(ROOT)` on the
    # unit paths it discovers, which fails through a symlinked course root.
    shutil.copytree(course_dir, link)
    for name in ("book", "content_style"):
        dst = stage / "scripts" / name
        dst.parent.mkdir(parents=True, exist_ok=True)
        if dst.exists() or dst.is_symlink():
            dst.unlink() if dst.is_symlink() else shutil.rmtree(dst)
        dst.symlink_to(ROOT / "scripts" / name, target_is_directory=True)
    shutil.copyfile(ROOT / "scripts" / "build_book_proto.py", stage / "scripts" / "build_book_proto.py")
    return stage


def build_v2_preview(stage: Path, out_dir: Path) -> Path:
    rc = run([sys.executable, str(stage / "scripts" / "build_book_proto.py"), "--out", str(out_dir)])
    if rc != 0:
        raise SystemExit("v2 build failed (rc=%d): %s" % (rc, stage))
    return out_dir / "index.html"


def build_volume(out_root: Path, out_name: str, course_dir: Path, preview_html: Path, work: Path,
                 label: str = "", title: str = "") -> None:
    out = out_root / out_name
    rc = run([
        sys.executable, str(BOOK_V3 / "build_site.py"),
        "--preview", str(preview_html),
        "--course-json", str(course_dir / "course.json"),
        "--style-css", str(ROOT / "scripts" / "book" / "style.css"),
        "--out", str(out),
        "--volume-dir", out_name,
        "--volume-label", label or "卷 II",
        "--volume-title", title or "数系与实数",
        "--base-url", "https://colorlessboy.github.io/sokonanoda-lang/book/",
    ])
    if rc != 0:
        raise SystemExit("v3 build failed for %s (rc=%d)" % (out_name, rc))


def copy_static(out_root: Path) -> None:
    assets = out_root / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(BOOK_V3 / "assets" / "v3.css", assets / "v3.css")
    shutil.copyfile(BOOK_V3 / "assets" / "v3.js", assets / "v3.js")
    shutil.copyfile(STATIC / "library.html", out_root / "index.html")
    v0 = out_root / "volume-0"
    (v0 / "assets").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(STATIC / "volume-0.html", v0 / "index.html")
    for name in ("book-components.css", "v3.css", "v3.js"):
        shutil.copyfile(STATIC / "volume-0-assets" / name, v0 / "assets" / name)


def count_units(course_json: Path) -> tuple[int, int]:
    manifest = json.loads(course_json.read_text(encoding="utf-8"))
    units = sum(len(ch.get("units", [])) for vol in manifest.get("volumes", []) for ch in vol.get("chapters", []))
    chapters = sum(len(vol.get("chapters", [])) for vol in manifest.get("volumes", []))
    return units, chapters


def verify(out_root: Path) -> int:
    """Link-and-count contract: no broken relative link, every unit page present."""
    errors: list[str] = []
    pages = 0
    for html in sorted(out_root.rglob("*.html")):
        pages += 1
        text = html.read_text(encoding="utf-8", errors="replace")
        import re

        for m in re.finditer(r'(?:href|src)="([^"#]+)"', text):
            target = m.group(1)
            if target.startswith(("http://", "https://", "mailto:", "data:", "//")):
                continue
            resolved = (html.parent / target).resolve()
            if not resolved.exists():
                errors.append("%s: missing target %s" % (html.relative_to(out_root).as_posix(), target))
    for out_name, course_dir, *_ in VOLUMES:
        units, chapters = count_units(ROOT / course_dir / "course.json")
        volume = out_root / out_name
        got_units = len(list((volume / "units").glob("unit*.html")))
        got_chapters = len(list((volume / "chapters").glob("*.html")))
        if got_units != units:
            errors.append("%s: %d unit pages, expected %d" % (out_name, got_units, units))
        if got_chapters != chapters:
            errors.append("%s: %d chapter pages, expected %d" % (out_name, got_chapters, chapters))
    if errors:
        print("book v3 verify: FAIL", file=sys.stderr)
        for e in errors[:40]:
            print("  -", e, file=sys.stderr)
        return 1
    print("book v3 verify: OK (%d pages, every relative link resolves)" % pages)
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Build the multi-page book (volumes I–II)")
    ap.add_argument("--out", default=str(OUT_DEFAULT))
    ap.add_argument("--check", action="store_true", help="verify an existing build instead of writing")
    ap.add_argument("--volume", default="", help="build only one volume dir (volume-i / volume-ii)")
    args = ap.parse_args(argv)

    out_root = Path(args.out).resolve()
    if args.check:
        return verify(out_root)

    if out_root.exists():
        shutil.rmtree(out_root)
    out_root.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="soko-book-") as tmp:
        tmp_path = Path(tmp)
        for out_name, course_dir, label, title in VOLUMES:
            if args.volume and args.volume != out_name:
                continue
            course = (ROOT / course_dir).resolve()
            work = tmp_path / out_name
            work.mkdir(parents=True, exist_ok=True)
            stage = stage_course(course, work / "stage")
            preview = build_v2_preview(stage, work / "v2")
            build_volume(out_root, out_name, course, preview, work, label, title)
            units, chapters = count_units(course / "course.json")
            print("  %s: %d units / %d chapters" % (out_name, units, chapters))

    copy_static(out_root)
    return verify(out_root)


if __name__ == "__main__":
    sys.exit(main())
