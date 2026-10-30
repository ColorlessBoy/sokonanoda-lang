#!/usr/bin/env python3
"""strict-score: the single end-to-end scoreboard for the strict-dialect refactor.

REQUIREMENTS.md §9.39 / reports/2026-10-29-strict-replan.md §3–§4 (P0):
strict is only an argument-binding policy, and it must never do more work than loose.
This script measures the product, not internal proxies:

  lib     every courses/set-theory/lib/*.sokonanoda x {loose, strict} via `grade --json`
          gate lib_strict: every strict run exits 0 (lib_loose reported, also a gate)
  counts  three fixed files x {loose, strict} with SOKO_JUDGE_STATS=1
          gate work_le_loose: strict JUDGE_INFER_SPLIT.misses <= loose AND
                              strict JUDGE_STATS.prefix_bytes <= loose (structural counts;
                              a miss = whole-prefix recompile, a hit is ~free;
                              wall seconds are reported, never gated)
  course  (--course) `course <course.json> --json` x {loose, strict}
          gate course_strict: strict failed == 0 and same unit count as loose

Every run is cold and deterministic:
  * SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_NO_MODULE_ARTIFACTS=1 (module-root stores off);
  * SOKONANODA_CACHE_DIR=<fresh temp dir per run>: the GLOBAL compile cache
    (crates/front/src/compile/cache.rs::root, default ~/Library/Caches/sokonanoda/compiled)
    is still read/written with the two switches above, and a hit replays the report
    without judging (a warm run prints no JUDGE_* lines at all). A fresh dir per run
    neutralises it without disabling the prelude-source file that also lives there;
  * SOKO_STRICT_AT is removed from the inherited environment; the dialect is chosen with
    `--dialect loose|strict`, which main.rs parses globally and passes to grade AND course
    (highest source in crates/cli/src/dialect.rs; overrides SOKO_STRICT_AT and directives).

--env K=V applies to strict runs only. Exit: 0 all gates green, 1 a gate red, 2 usage/infra.
Stdlib only.
"""
from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LIB_DIR = REPO / "courses/set-theory/lib"
COURSE = REPO / "courses/set-theory/course.json"
COUNT_FILES = [
    "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    "courses/set-theory/units/I.4/unit12-synthesis.sokonanoda",
    "courses/set-theory/lib/Equiv.sokonanoda",
]
DIALECTS = ("loose", "strict")
# `TAG k=v k=v ...` with an UPPERCASE tag (e.g. `JUDGE_INFER calls=51 total_ms=177 ...`).
STAT_LINE = re.compile(r"^([A-Z][A-Z0-9_]*)((?:\s+[A-Za-z_][A-Za-z0-9_]*=\S*)+)\s*$")
STAT_PAIR = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)=(\S*)")


class Infra(Exception):
    """Usage / infrastructure failure ⇒ exit 2."""


def default_bin() -> str:
    try:
        out = subprocess.run(
            [str(REPO / "scripts/soko"), "version", "--json"],
            cwd=REPO, capture_output=True, text=True, timeout=120,
        )
        path = json.loads(out.stdout)["cli"]["path"]
    except Exception as exc:  # noqa: BLE001 — any failure here is infra
        raise Infra(f"cannot resolve default binary via `scripts/soko version --json`: {exc}")
    return path


def bin_identity(path: str) -> dict:
    p = Path(path)
    if not p.is_file() or not os.access(p, os.X_OK):
        raise Infra(f"binary not found or not executable: {path}")
    sha = hashlib.sha256(p.read_bytes()).hexdigest()
    try:
        out = subprocess.run([path, "version", "--json"], capture_output=True, text=True, timeout=60)
        version = json.loads(out.stdout)
    except Exception as exc:  # noqa: BLE001
        version = {"error": f"`version --json` unparsable: {exc}"}
    return {"path": str(p.resolve()), "sha256": sha[:16], "version": version}


def run_cold(argv: list[str], extra_env: dict, timeout: float) -> dict:
    """One cold process: fresh global-cache dir, module-root stores off."""
    cache = tempfile.mkdtemp(prefix="strict-score-cache.")
    env = dict(os.environ)
    env.pop("SOKO_STRICT_AT", None)
    env.update({
        "SOKONANODA_NO_PROJECT_ARTIFACTS": "1",
        "SOKONANODA_NO_MODULE_ARTIFACTS": "1",
        "SOKONANODA_CACHE_DIR": cache,
    })
    env.update(extra_env)
    t0 = time.monotonic()
    try:
        proc = subprocess.run(argv, cwd=REPO, env=env, capture_output=True, text=True,
                              timeout=timeout)
        res = {"exit": proc.returncode, "stdout": proc.stdout, "stderr": proc.stderr,
               "timeout": False}
    except subprocess.TimeoutExpired:
        res = {"exit": "timeout", "stdout": "", "stderr": "", "timeout": True}
    res["wall_s"] = round(time.monotonic() - t0, 3)
    shutil.rmtree(cache, ignore_errors=True)
    return res


def parse_events(stdout: str, what: str) -> list[dict]:
    events = []
    for n, line in enumerate(stdout.splitlines(), 1):
        if not line.strip():
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError as exc:
            raise Infra(f"{what}: stdout line {n} is not JSON ({exc}): {line[:120]!r}")
    return events


def parse_stats(stderr: str) -> dict:
    """Every `TAG k=v ...` line. Repeated tags: numeric values are summed, `lines` counts them."""
    stats: dict[str, dict] = {}
    for line in stderr.splitlines():
        m = STAT_LINE.match(line.strip())
        if not m:
            continue
        slot = stats.setdefault(m.group(1), {"lines": 0})
        slot["lines"] += 1
        for k, v in STAT_PAIR.findall(m.group(2)):
            try:
                num = int(v)
            except ValueError:
                try:
                    num = float(v)
                except ValueError:
                    slot[k] = v
                    continue
            prev = slot.get(k, 0)
            slot[k] = (prev if isinstance(prev, (int, float)) else 0) + num
    return stats


def env_for(dialect: str, strict_env: dict) -> dict:
    return dict(strict_env) if dialect == "strict" else {}


# ---------------------------------------------------------------- sections

def lib_job(binp: str, f: Path, dialect: str, strict_env: dict, timeout: float) -> dict:
    rel = str(f.relative_to(REPO))
    r = run_cold([binp, "grade", "--dialect", dialect, rel, "--json"], env_for(dialect, strict_env),
                 timeout)
    out = {"file": rel, "dialect": dialect, "exit": r["exit"], "wall_s": r["wall_s"]}
    if r["timeout"]:
        out.update(diagnostics=None, decl_checked=None, first_diagnostic=None)
        return out
    events = parse_events(r["stdout"], f"grade {rel} ({dialect})")
    diags = [e for e in events if e.get("type") == "diagnostic"]
    out["diagnostics"] = len(diags)
    out["decl_checked"] = sum(1 for e in events if e.get("type") == "decl.checked")
    if diags:
        d = diags[0]
        line = ((d.get("span") or {}).get("start") or {}).get("line")
        out["first_diagnostic"] = {
            "code": d.get("code"),
            "message": (d.get("message") or "")[:160],
            "at": f"{d.get('file') or rel}:{line}",
        }
    else:
        out["first_diagnostic"] = None
    if r["exit"] not in (0, 1):
        out["stderr_tail"] = r["stderr"][-400:]
    return out


def counts_job(binp: str, rel: str, dialect: str, strict_env: dict, timeout: float) -> dict:
    env = env_for(dialect, strict_env)
    env["SOKO_JUDGE_STATS"] = "1"
    r = run_cold([binp, "grade", "--dialect", dialect, rel, "--json"], env, timeout)
    out = {"file": rel, "dialect": dialect, "exit": r["exit"], "wall_s": r["wall_s"],
           "timeout": r["timeout"]}
    if not r["timeout"]:
        out["stats"] = parse_stats(r["stderr"])
    return out


def course_job(binp: str, dialect: str, strict_env: dict, timeout: float) -> dict:
    r = run_cold([binp, "--dialect", dialect, "course", str(COURSE), "--json"],
                 env_for(dialect, strict_env), timeout)
    out = {"dialect": dialect, "exit": r["exit"], "wall_s": r["wall_s"], "timeout": r["timeout"]}
    if r["timeout"]:
        return out
    events = parse_events(r["stdout"], f"course ({dialect})")
    units = [e for e in events if e.get("type") == "course.unit"]
    summary = next((e for e in events if e.get("type") == "course.summary"), None)
    if summary is None:
        raise Infra(f"course ({dialect}): no course.summary event (exit {r['exit']}): "
                    f"{r['stderr'][-300:]!r}")
    failing = [u["file"] for u in units if (u.get("failed") or 0) > 0 or u.get("error")]
    out.update(targets=len(units), checked=summary.get("checked"), failed=summary.get("failed"),
               open=summary.get("open"), failing_units=failing[:20])
    return out


# ---------------------------------------------------------------- gates

def gate_lib(rows: list[dict], dialect: str) -> dict:
    mine = [r for r in rows if r["dialect"] == dialect]
    passed = sum(1 for r in mine if r["exit"] == 0)
    return {"green": passed == len(mine) and mine != [], "passed": passed, "total": len(mine),
            "failing": [r["file"] for r in mine if r["exit"] != 0]}


def stat(row: dict, tag: str, key: str):
    return ((row.get("stats") or {}).get(tag) or {}).get(key)


def gate_work(rows: list[dict]) -> dict:
    per_file = []
    green = True
    for rel in COUNT_FILES:
        lo = next(r for r in rows if r["file"] == rel and r["dialect"] == "loose")
        st = next(r for r in rows if r["file"] == rel and r["dialect"] == "strict")
        li, si = stat(lo, "JUDGE_INFER", "calls"), stat(st, "JUDGE_INFER", "calls")
        # Gate on cache MISSES, not calls: a hit costs ~0.1 ms, a miss recompiles the whole
        # prefix (~0.15-0.5 s). Measured 2026-10-30 on unit12: calls 587 vs 67, misses 30 vs 9.
        lm, sm = stat(lo, "JUDGE_INFER_SPLIT", "misses"), stat(st, "JUDGE_INFER_SPLIT", "misses")
        lp, sp = stat(lo, "JUDGE_STATS", "prefix_bytes"), stat(st, "JUDGE_STATS", "prefix_bytes")
        reason = None
        if lo["timeout"] or st["timeout"]:
            ok, reason = False, "timeout"
        elif None in (lm, sm, lp, sp):
            # A missing counter means the probe did not fire: never treat that as "0 work".
            ok, reason = False, "missing JUDGE_INFER_SPLIT.misses / JUDGE_STATS.prefix_bytes"
        else:
            ok = sm <= lm and sp <= lp
        green &= ok
        per_file.append({"file": rel, "green": ok, "infer_misses": {"loose": lm, "strict": sm},
                         "infer_calls": {"loose": li, "strict": si},
                         "prefix_bytes": {"loose": lp, "strict": sp},
                         "wall_s": {"loose": lo["wall_s"], "strict": st["wall_s"]},
                         **({"reason": reason} if reason else {})})
    return {"green": green, "files": per_file}


def gate_course(rows: list[dict]) -> dict:
    lo = next(r for r in rows if r["dialect"] == "loose")
    st = next(r for r in rows if r["dialect"] == "strict")
    if lo["timeout"] or st["timeout"]:
        return {"green": False, "reason": "timeout"}
    ok = st["failed"] == 0 and st["targets"] == lo["targets"]
    return {"green": ok, "targets": {"loose": lo["targets"], "strict": st["targets"]},
            "failed": {"loose": lo["failed"], "strict": st["failed"]},
            "checked": {"loose": lo["checked"], "strict": st["checked"]}}


# ---------------------------------------------------------------- main

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--bin", help="sokonanoda CLI (default: cli.path of `scripts/soko version --json`)")
    ap.add_argument("--env", action="append", default=[], metavar="K=V",
                    help="extra env for STRICT runs only (repeatable)")
    ap.add_argument("--course", action="store_true", help="also run the whole-course section")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=600.0, help="per-run timeout (s)")
    ap.add_argument("--json-out", metavar="FILE")
    try:
        args = ap.parse_args()
    except SystemExit as exc:
        return 2 if exc.code else 0
    try:
        strict_env = {}
        for kv in args.env:
            if "=" not in kv or not kv.split("=", 1)[0]:
                raise Infra(f"--env expects K=V, got {kv!r}")
            k, v = kv.split("=", 1)
            strict_env[k] = v
        if args.jobs < 1:
            raise Infra("--jobs must be >= 1")
        binp = args.bin or default_bin()
        ident = bin_identity(binp)
        libs = sorted(LIB_DIR.glob("*.sokonanoda"))
        if not libs:
            raise Infra(f"no lib files under {LIB_DIR}")

        t0 = time.monotonic()
        with cf.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            lib_f = [pool.submit(lib_job, binp, f, d, strict_env, args.timeout)
                     for f in libs for d in DIALECTS]
            cnt_f = [pool.submit(counts_job, binp, rel, d, strict_env, args.timeout)
                     for rel in COUNT_FILES for d in DIALECTS]
            crs_f = ([pool.submit(course_job, binp, d, strict_env, args.timeout) for d in DIALECTS]
                     if args.course else [])
            lib_rows = [f.result() for f in lib_f]
            cnt_rows = [f.result() for f in cnt_f]
            crs_rows = [f.result() for f in crs_f]
        wall = round(time.monotonic() - t0, 2)
    except Infra as exc:
        print(f"strict-score: infra error: {exc}", file=sys.stderr)
        return 2

    gates = {
        "lib_strict": gate_lib(lib_rows, "strict"),
        "lib_loose": gate_lib(lib_rows, "loose"),
        "work_le_loose": gate_work(cnt_rows),
    }
    if args.course:
        gates["course_strict"] = gate_course(crs_rows)
    all_green = all(g["green"] for g in gates.values())
    report = {
        "schema": "soko.strict-score/1",
        "bin": ident,
        "strict_env": strict_env,
        "cold": {"SOKONANODA_NO_PROJECT_ARTIFACTS": "1", "SOKONANODA_NO_MODULE_ARTIFACTS": "1",
                 "SOKONANODA_CACHE_DIR": "<fresh temp dir per run>", "SOKO_STRICT_AT": "<unset>"},
        "green": all_green,
        "gates": gates,
        "lib": lib_rows,
        "counts": cnt_rows,
        **({"course": crs_rows} if args.course else {}),
        "wall_s": wall,
    }
    text = json.dumps(report, ensure_ascii=False, indent=2)
    print(text)
    if args.json_out:
        Path(args.json_out).write_text(text + "\n", encoding="utf-8")

    # Short human summary on stderr.
    e = sys.stderr
    print(f"strict-score  bin={ident['path']} sha256={ident['sha256']}  "
          f"env(strict)={strict_env or '{}'}  wall={wall}s", file=e)
    for name in ("lib_strict", "lib_loose"):
        g = gates[name]
        print(f"  {'GREEN' if g['green'] else 'RED  '} {name}: {g['passed']}/{g['total']}"
              + (f"  failing={','.join(Path(x).stem for x in g['failing'])}" if g["failing"] else ""),
              file=e)
    g = gates["work_le_loose"]
    print(f"  {'GREEN' if g['green'] else 'RED  '} work_le_loose", file=e)
    for f in g["files"]:
        print(f"      {'ok ' if f['green'] else 'RED'} {Path(f['file']).stem}: "
              f"infer {f['infer_calls']['strict']} vs {f['infer_calls']['loose']}  "
              f"prefix {f['prefix_bytes']['strict']} vs {f['prefix_bytes']['loose']}  "
              f"wall {f['wall_s']['strict']}s vs {f['wall_s']['loose']}s"
              + (f"  ({f['reason']})" if f.get("reason") else ""), file=e)
    if args.course:
        g = gates["course_strict"]
        print(f"  {'GREEN' if g['green'] else 'RED  '} course_strict: "
              + (g.get("reason") or f"targets {g['targets']} failed {g['failed']}"), file=e)
    return 0 if all_green else 1


if __name__ == "__main__":
    sys.exit(main())
