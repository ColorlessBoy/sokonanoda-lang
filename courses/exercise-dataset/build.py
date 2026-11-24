#!/usr/bin/env python3
"""Generate the course exercise dataset consumed by the language-service plugin.

Usage (repository root):

    python3 courses/exercise-dataset/build.py            # write data/*.json
    python3 courses/exercise-dataset/build.py --check    # committed data must be current
    python3 courses/exercise-dataset/build.py --json     # machine-readable counts

The dataset is derived only from course sources (`course.json` + canvases +
solution keys); it never calls the kernel and never edits a course file.  Output
is deterministic: the same sources produce byte-identical JSON.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import dataset  # noqa: E402


def _write_atomic(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmpdir", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(text)
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def _read_committed(root: Path, rel: str) -> str:
    path = root / rel
    return path.read_text(encoding="utf-8") if path.is_file() else ""


def main(argv: list = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=HERE, help="dataset directory (default: this one)")
    parser.add_argument("--check", action="store_true", help="fail if committed data differs from a fresh build")
    parser.add_argument("--json", action="store_true", help="print counts and per-course output bytes")
    args = parser.parse_args(argv)

    payload = dataset.build_all()
    files = dataset.dataset_files(payload)
    out = args.out.resolve()
    stale = []
    for rel, text in files.items():
        committed = _read_committed(out, rel)
        if committed != text:
            stale.append(rel)

    if args.check:
        if stale:
            print("exercise dataset: FAIL (generated data is stale)", file=sys.stderr)
            print("  stale: " + ", ".join(stale), file=sys.stderr)
            print("  rebuild: python3 courses/exercise-dataset/build.py", file=sys.stderr)
            return 1
        print(f"exercise dataset: clean ({len(files)} files)")
        return 0

    for rel, text in files.items():
        _write_atomic(out / rel, text)

    if args.json:
        report = {
            "schema": payload["manifest"]["schema"],
            "totals": payload["manifest"]["totals"],
            "courses": [
                {**course, "bytes": len(files[course["file"]].encode("utf-8"))}
                for course in payload["manifest"]["courses"]
            ],
            "files": {rel: len(text.encode("utf-8")) for rel, text in files.items()},
        }
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        total = payload["manifest"]["totals"]
        print(
            "exercise dataset: "
            f"{total['exercises']} exercises · {total['units']} units · "
            f"{total['solutions']} solutions · {total['hints']} hints"
        )
        for course in payload["manifest"]["courses"]:
            print(
                f"  {course['id']}: {course['exercises']} exercises / "
                f"{course['units']} units ({course['file']})"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
