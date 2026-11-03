#!/usr/bin/env python3
"""exception-census: **per-file census of the strict dialect's "written exceptions"**.

Why this exists: `nofallback_kept_<reason>` / `fallback_shape` are the only places where strict
still goes back to the old elaborator, and each one is a deviation from `REQUIREMENTS §9.39`
(`strict` must never do more work than `loose`).  A total over the whole corpus hides which file
carries which exception — and the acceptance rule for the D4 revisit is "one line per remaining
file: exception type / count / why it cannot be removed yet", which needs exactly this table.

⚠ **The census is not the invariant.** Measured 2026-11-03 (binary `06f3eb09`, HEAD 4b5bb690):
8 files carry a `nofallback_kept_*`, but only 4 of them violate `strict <= loose`, and 3 of the 7
violating files carry **no** kept exception at all (`unit02` / `unit06` / `G30`). Use
`scripts/strict-score.py --corpus` for the invariant itself.

Usage:
    python3 scripts/strict/exception-census.py <binary> [--only SUBSTR]... [--json]
    python3 scripts/strict/exception-census.py <binary> --write /tmp/exc.json

Each run is cold (`SOKONANODA_CACHE_DIR` = fresh temp dir, module/project stores off,
`SOKO_STRICT_AT` unset) and gets `SOKO_JUDGE_STATS=1` so the `KERNEL_APP ...` line is printed at
exit.  Wall time: ~1.5 min for all 357 tracked files at `-P 6`.
"""
from __future__ import annotations

import argparse
import concurrent.futures as cf
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
# `KERNEL_APP ...` keys that mean "strict tried and then went back to the old path".
KEPT = re.compile(r"nofallback_kept_(\w+)=(\d+)")
# Counters that behave like an exception but live in their own field (pre-attempt gates).
ALSO = ("fallback_shape", "nofallback_dead")
STAT = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)=(\S*)")


def files() -> list[str]:
    out = subprocess.run(["git", "ls-files", "*.sokonanoda"], cwd=REPO,
                         capture_output=True, text=True, timeout=120)
    if out.returncode != 0:
        sys.exit(f"git ls-files failed: {out.stderr[-200:]}")
    return sorted(x for x in out.stdout.splitlines() if x.strip())


def one(binp: str, rel: str) -> dict:
    cache = tempfile.mkdtemp(prefix="exc-cache.")
    env = dict(os.environ)
    env.pop("SOKO_STRICT_AT", None)
    env.update({
        "SOKONANODA_NO_PROJECT_ARTIFACTS": "1",
        "SOKONANODA_NO_MODULE_ARTIFACTS": "1",
        "SOKONANODA_CACHE_DIR": cache,
        "SOKO_JUDGE_STATS": "1",
    })
    try:
        p = subprocess.run([binp, "grade", "--dialect", "strict", rel, "--json"], cwd=REPO,
                           env=env, capture_output=True, text=True, timeout=900)
        rc, out, err = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        rc, out, err = "timeout", "", ""
    finally:
        shutil.rmtree(cache, ignore_errors=True)
    diags = 0
    for line in out.splitlines():
        if line.startswith("{"):
            try:
                if json.loads(line).get("type") == "diagnostic":
                    diags += 1
            except json.JSONDecodeError:
                pass
    kept, also = {}, {}
    for line in err.splitlines():
        if line.startswith("KERNEL_APP "):
            for k, v in KEPT.findall(line):
                kept[k] = kept.get(k, 0) + int(v)
            pairs = dict(STAT.findall(line))
            for k in ALSO:
                if k in pairs:
                    also[k] = also.get(k, 0) + int(pairs[k])
    return {"file": rel, "rc": rc, "diags": diags, "kept": kept, "also": also}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("bin")
    ap.add_argument("--only", action="append", default=[], metavar="SUBSTR",
                    help="restrict to paths containing SUBSTR (repeatable)")
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--json", action="store_true", help="print the whole table as JSON")
    ap.add_argument("--write", metavar="FILE", help="also write the JSON table to FILE")
    args = ap.parse_args()
    if not Path(args.bin).is_file():
        sys.exit(f"no such binary: {args.bin}")
    rels = [f for f in files() if not args.only or any(s in f for s in args.only)]
    if not rels:
        sys.exit("no files matched")
    with cf.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        rows = list(pool.map(lambda r: one(args.bin, r), rels))
    interesting = [r for r in rows if r["kept"] or any(r["also"].values())]
    table = {"bin": str(Path(args.bin).resolve()), "files": len(rows), "rows": rows}
    if args.json or args.write:
        text = json.dumps(table, ensure_ascii=False, indent=1)
        if args.write:
            Path(args.write).write_text(text + "\n", encoding="utf-8")
        if args.json:
            print(text)
    print(f"exception-census: {len(rows)} files, "
          f"{len(interesting)} with an exception", file=sys.stderr)
    for r in interesting:
        parts = [f"{k}={v}" for k, v in sorted(r["kept"].items())]
        parts += [f"{k}={v}" for k, v in sorted(r["also"].items()) if v]
        print(f"  rc={r['rc']!s:<2} diags={r['diags']:<2} {r['file']}\n"
              f"      {' '.join(parts)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
