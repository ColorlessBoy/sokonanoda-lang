#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""strict-compat.py —— 严格档 v2（`@` 语义）兼容性量具（P1，纯标准库）。

**问的问题**：同一份二进制、同一份语料，**严格档与宽松档给出的结论差在哪**？
两档各自跑一遍 `grade --json`，比较 **stdout 字节 + 退出码**（判定真相的唯一口径；
不做任何文本启发式 ✓）。

用法：

    python3 scripts/strict-compat.py --sweep [--json]
    python3 scripts/strict-compat.py --selftest [--json]
    python3 scripts/strict-compat.py --help

`--sweep`（默认动作）递归找 `*.sokonanoda`（跳过 `target/`、`.git/`、`.sokonanoda/`），
每个文件跑两次：

    SOKO_STRICT_AT=0  <bin> grade --json <file>     # 宽松档（legacy 读法）
    SOKO_STRICT_AT=1  <bin> grade --json <file>     # 严格档（正常形式不吃位置隐式实参）

另外每腿都注入 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`、`SOKONANODA_NO_CACHE=1`
（两处产物/缓存都关 ⇒ 并行跑不会互相污染，量的也不是缓存回放）。
`JUDGE_PREFIX runs=N` 由 `SOKO_STAGE_STATS=1` 在进程退出时打到 stderr
（`crates/front/src/compile/check/mod.rs` 的 atexit 打印）⇒ 本脚本默认注入它，
只抓那一行做**结构计数**（不做毫秒判据 ✗）。`--no-stage-stats` 可关掉（那两列变 null）。

**环境隔离**：默认把继承来的 `SOKO*` / `SOKONANODA*` 变量**全部清掉**再注入上面那几个
—— 否则外部设过 `SOKO_KERNEL_ARG_PP` / `SOKO_JUDGE_PREFIX_OPAQUE` / `SOKO_NO_JUDGE`
之类，量到的就不是两档的差而是环境的差（假红/假绿 ✓；同 `crates/cli/tests/
implicit_at_alignment.rs` 的 `env_remove` 纪律）。`--keep-env` 可保留（复现外部环境时用）。

输出：

* 默认：人读摘要（计数 + 前 20 条差异）到 stdout；
* `--json`：stdout **只**输出一个 JSON 对象
  `{root, bin, files, diffs, rows, jobs, timeout_s, errors, timeouts, elapsed_s}`，
  rows 每项 = `{file, loose_exit, strict_exit, same, loose_prefix, strict_prefix,
  first_diag_code, first_diag_line}`（异常时另加 `error`）；
* 进度/摘要行一律走 stderr（`--json` 时 stdout 保持干净 ✓）。

**`first_diag_code/line` 的口径**：取**严格档独有**的第一条诊断（码+行都不在宽松档里）；
严格档没有独有诊断时退到严格档第一条，再退到宽松档第一条（不再有则 null）。
`first_diag_line` 是 1 起的行号（诊断事件 `span.start.line`）。

`--selftest` 在系统临时目录造两个夹具（**不依赖 `lib/Prod`**——HEAD 上绿、R3 之后才红，
不能当基准）并断言量具有牙：

* `green.sokonanoda` = `def x : Nat := 1` ⇒ 两档都必须 exit 0 且 stdout 逐字节相同；
* `red.sokonanoda` = `h`/`b2`（正常形式塞位置隐式实参）⇒ 宽松档 exit 0、严格档 exit 1，
  且严格档必须带一条可归属的诊断（否则 1 可能只是崩溃 ⇒ 咬不住）。

**退出码**：0 = 扫描完成且无差异 / selftest 通过；1 = 有差异或 selftest 失败；
2 = 用法错误（含二进制不存在、`--root` 不是目录）。

只打印摘要，**不**把几百行事件流灌进终端/日志。
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

EXIT_OK = 0
EXIT_FAIL = 1
EXIT_USAGE = 2

SKIP_DIRS = frozenset({"target", ".git", ".sokonanoda"})
DEFAULT_JOBS = 4
DEFAULT_TIMEOUT = 180.0  # 秒/腿
DEFAULT_BIN_REL = os.path.join("target", "release", "sokonanoda")
PROGRESS_EVERY = 20
HUMAN_DIFF_LIMIT = 20

# `JUDGE_PREFIX runs=3759 bytes=... bytes_per_run=...`（stderr，进程退出时一次）
JUDGE_PREFIX_RE = re.compile(r"JUDGE_PREFIX\s+runs=(\d+)")

# 自检夹具：**不得依赖 `lib/Prod`**（HEAD 上它是绿的，R3 之后才红 ⇒ 当基准就是假绿）。
GREEN_SRC = "def x : Nat := 1\n"
RED_SRC = (
    "def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a\n"
    "def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5\n"
)


@dataclass
class Leg:
    """一次 `grade` 的读数。`code is None` = 没跑起来（超时或 exec 失败）。"""

    code: Optional[int]
    stdout: bytes
    stderr: bytes
    timed_out: bool = False
    error: Optional[str] = None


@dataclass
class Outcome:
    """一个文件的账：row 进 JSON，另带两个只用于汇总的计数。"""

    row: Dict[str, object]
    timed_out: bool = False
    exec_error: bool = False


@dataclass(frozen=True)
class Runner:
    """一档一腿的执行配置（两腿唯一差别是 `SOKO_STRICT_AT`）。"""

    bin_path: str
    cwd: str
    timeout: float
    stage_stats: bool
    keep_env: bool

    def leg(self, file_path: str, strict: str) -> Leg:
        env = child_env(strict, stage_stats=self.stage_stats, keep_env=self.keep_env)
        cmd = [self.bin_path, "grade", "--json", file_path]
        try:
            proc = subprocess.run(
                cmd,
                cwd=self.cwd,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=self.timeout,
            )
        except subprocess.TimeoutExpired:
            # `run()` 已经 kill + 回收子进程；POSIX 上异常里的 stdout 未必有 ⇒ 不指望它。
            return Leg(None, b"", b"", timed_out=True)
        except OSError as exc:
            return Leg(None, b"", b"", error="%s: %s" % (type(exc).__name__, exc))
        return Leg(proc.returncode, proc.stdout, proc.stderr)

    def outcome(self, label: str, file_path: str) -> Outcome:
        loose = self.leg(file_path, "0")
        strict = self.leg(file_path, "1")
        row, timed_out, exec_error = build_row(label, loose, strict, self.timeout)
        return Outcome(row, timed_out=timed_out, exec_error=exec_error)


def child_env(strict: str, *, stage_stats: bool, keep_env: bool) -> Dict[str, str]:
    """子进程环境：默认清掉继承的 `SOKO*`/`SOKONANODA*`，再注入本量具的三个（+可选读数）。"""
    if keep_env:
        env = dict(os.environ)
    else:
        # `SOKO` 前缀同时覆盖 `SOKO_*` 与 `SOKONANODA_*` ✓
        env = {k: v for k, v in os.environ.items() if not k.startswith("SOKO")}
    env["SOKO_STRICT_AT"] = strict
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"
    env["SOKONANODA_NO_CACHE"] = "1"
    if stage_stats:
        env["SOKO_STAGE_STATS"] = "1"
    return env


def judge_prefix_runs(stderr: bytes) -> Optional[int]:
    """抓 `JUDGE_PREFIX runs=N`（多次出现取最后一次）；没开 stage stats 时为 None。"""
    matches = JUDGE_PREFIX_RE.findall(stderr.decode("utf-8", "replace"))
    return int(matches[-1]) if matches else None


def diagnostics(stdout: bytes) -> List[Tuple[Optional[str], Optional[int]]]:
    """按协议解析 JSON Lines，只收 `type == "diagnostic"` 的 `(code, line)`。"""
    found: List[Tuple[Optional[str], Optional[int]]] = []
    for raw in stdout.splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            event = json.loads(raw.decode("utf-8", "replace"))
        except ValueError:
            continue  # 非 JSON 行（不该有）不算诊断，也不让量具崩
        if not isinstance(event, dict) or event.get("type") != "diagnostic":
            continue
        code = event.get("code")
        line = None
        span = event.get("span")
        if isinstance(span, dict):
            start = span.get("start")
            if isinstance(start, dict) and isinstance(start.get("line"), int):
                line = start["line"]
        found.append((code if isinstance(code, str) else None, line))
    return found


def pick_first_diag(
    loose: List[Tuple[Optional[str], Optional[int]]],
    strict: List[Tuple[Optional[str], Optional[int]]],
) -> Tuple[Optional[str], Optional[int]]:
    """严格档独有 → 严格档第一条 → 宽松档第一条 → (None, None)。"""
    strict_only = [d for d in strict if d not in loose]
    if strict_only:
        return strict_only[0]
    if strict:
        return strict[0]
    if loose:
        return loose[0]
    return (None, None)


def build_row(
    label: str, loose: Leg, strict: Leg, timeout: float
) -> Tuple[Dict[str, object], bool, bool]:
    """两腿 ⇒ 一行账。`same` = 退出码与 stdout 字节都相同（且两腿都真的跑完了）。"""
    same = (
        not loose.timed_out
        and not strict.timed_out
        and loose.error is None
        and strict.error is None
        and loose.code is not None
        and strict.code is not None
        and loose.code == strict.code
        and loose.stdout == strict.stdout
    )
    code, line = pick_first_diag(diagnostics(loose.stdout), diagnostics(strict.stdout))
    row: Dict[str, object] = {
        "file": label,
        "loose_exit": loose.code,
        "strict_exit": strict.code,
        "same": same,
        "loose_prefix": judge_prefix_runs(loose.stderr),
        "strict_prefix": judge_prefix_runs(strict.stderr),
        "first_diag_code": code,
        "first_diag_line": line,
    }
    notes: List[str] = []
    if loose.timed_out:
        notes.append("loose: timeout>%gs" % timeout)
    if strict.timed_out:
        notes.append("strict: timeout>%gs" % timeout)
    for name, leg in (("loose", loose), ("strict", strict)):
        if leg.error:
            notes.append("%s: %s" % (name, leg.error))
    if notes:
        row["error"] = "; ".join(notes)
    timed_out = loose.timed_out or strict.timed_out
    exec_error = bool(loose.error or strict.error)
    return row, timed_out, exec_error


def discover(root: str) -> List[Tuple[str, str]]:
    """递归找 `*.sokonanoda`（跳过三处产物目录），返回排序后的 (相对路径, 绝对路径)。"""
    found: List[Tuple[str, str]] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        for name in filenames:
            if not name.endswith(".sokonanoda"):
                continue
            abs_path = os.path.join(dirpath, name)
            rel = os.path.relpath(abs_path, root).replace(os.sep, "/")
            found.append((rel, abs_path))
    found.sort(key=lambda item: item[0])
    return found


def run_outcomes(
    runner: Runner, items: List[Tuple[str, str]], jobs: int, quiet: bool = False
) -> List[Outcome]:
    """并发跑一批 (标签, 路径)；返回与输入同序的 Outcome 列表。"""
    total = len(items)
    outcomes: List[Outcome] = [None] * total  # type: ignore[list-item]
    if total == 0:
        return []
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = {}
        for index, (label, path) in enumerate(items):
            futures[pool.submit(runner.outcome, label, path)] = index
        done = 0
        diffs = 0
        for future in concurrent.futures.as_completed(futures):
            index = futures[future]
            try:
                outcome = future.result()
            except Exception as exc:  # 工人自己崩了也要进账，不许吞掉
                row: Dict[str, object] = {
                    "file": items[index][0],
                    "loose_exit": None,
                    "strict_exit": None,
                    "same": False,
                    "loose_prefix": None,
                    "strict_prefix": None,
                    "first_diag_code": None,
                    "first_diag_line": None,
                    "error": "worker: %s: %s" % (type(exc).__name__, exc),
                }
                outcome = Outcome(row, exec_error=True)
            outcomes[index] = outcome
            done += 1
            if not outcome.row["same"]:
                diffs += 1
            if not quiet and (done % PROGRESS_EVERY == 0 or done == total):
                print(
                    "strict-compat: %d/%d files, %d diffs" % (done, total, diffs),
                    file=sys.stderr,
                    flush=True,
                )
    return outcomes


def do_sweep(root: str, runner: Runner, jobs: int, json_mode: bool) -> int:
    started = time.monotonic()
    files = discover(root)
    print(
        "strict-compat: root=%s bin=%s jobs=%d files=%d"
        % (root, runner.bin_path, jobs, len(files)),
        file=sys.stderr,
        flush=True,
    )
    if not files:
        print("strict-compat: warning: 没找到任何 .sokonanoda（--root 对吗？）", file=sys.stderr)
    outcomes = run_outcomes(runner, files, jobs)
    rows = [outcome.row for outcome in outcomes]
    diffs = sum(1 for row in rows if not row["same"])
    timeouts = sum(1 for outcome in outcomes if outcome.timed_out)
    errors = sum(1 for outcome in outcomes if outcome.exec_error)
    elapsed = round(time.monotonic() - started, 1)
    payload = {
        "root": root,
        "bin": runner.bin_path,
        "files": len(files),
        "diffs": diffs,
        "rows": rows,
        "jobs": jobs,
        "timeout_s": runner.timeout,
        "errors": errors,
        "timeouts": timeouts,
        "elapsed_s": elapsed,
    }
    if json_mode:
        emit_json(payload)
        print(
            "strict-compat: files=%d diffs=%d errors=%d timeouts=%d elapsed=%ss"
            % (len(files), diffs, errors, timeouts, elapsed),
            file=sys.stderr,
            flush=True,
        )
    else:
        print_sweep_human(payload)
    return EXIT_OK if diffs == 0 else EXIT_FAIL


def print_sweep_human(payload: Dict[str, object]) -> None:
    print("root:  %s" % payload["root"])
    print("bin:   %s" % payload["bin"])
    print(
        "files: %d   diffs: %d   errors: %d   timeouts: %d   elapsed: %ss"
        % (
            payload["files"],
            payload["diffs"],
            payload["errors"],
            payload["timeouts"],
            payload["elapsed_s"],
        )
    )
    rows = payload["rows"]
    assert isinstance(rows, list)
    bad = [row for row in rows if not row["same"]]
    for row in bad[:HUMAN_DIFF_LIMIT]:
        note = "  [%s]" % row["error"] if "error" in row else ""
        print(
            "DIFF  %s  loose=%s strict=%s  prefix=%s→%s  first_diag=%s:%s%s"
            % (
                row["file"],
                row["loose_exit"],
                row["strict_exit"],
                row["loose_prefix"],
                row["strict_prefix"],
                row["first_diag_code"],
                row["first_diag_line"],
                note,
            )
        )
    if len(bad) > HUMAN_DIFF_LIMIT:
        print("... 另有 %d 条差异（--json 看全量）" % (len(bad) - HUMAN_DIFF_LIMIT))
    if not bad:
        print("no divergence: 两档在全部文件上逐字节一致")


def do_selftest(root: str, runner: Runner, json_mode: bool) -> int:
    try:
        tmpdir = tempfile.mkdtemp(prefix="soko-strict-compat-")
    except OSError as exc:
        print("strict-compat: 建不了临时目录：%s" % exc, file=sys.stderr)
        return EXIT_USAGE
    try:
        green_path = os.path.join(tmpdir, "green.sokonanoda")
        red_path = os.path.join(tmpdir, "red.sokonanoda")
        for path, src in ((green_path, GREEN_SRC), (red_path, RED_SRC)):
            with open(path, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(src)
        green = runner.outcome("selftest/green.sokonanoda", green_path).row
        red = runner.outcome("selftest/red.sokonanoda", red_path).row
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

    checks: List[Dict[str, object]] = []

    def check(name: str, ok: bool, detail: str) -> None:
        checks.append({"name": name, "ok": bool(ok), "detail": detail})

    check(
        "green-both-dialects-accept",
        green["loose_exit"] == 0 and green["strict_exit"] == 0 and green["same"] is True,
        "green: loose=%s strict=%s same=%s（两档都必须收，且 stdout 逐字节相同）"
        % (green["loose_exit"], green["strict_exit"], green["same"]),
    )
    check(
        "red-loose-accepts",
        red["loose_exit"] == 0,
        "red: 宽松档必须 exit 0（实测 %s）" % (red["loose_exit"],),
    )
    check(
        "red-strict-rejects",
        red["strict_exit"] == 1,
        "red: 严格档必须 exit 1（实测 %s）" % (red["strict_exit"],),
    )
    check(
        "red-legs-differ",
        red["same"] is False,
        "red: 两档必须不同（实测 same=%s）" % (red["same"],),
    )
    check(
        "red-has-diagnostic",
        isinstance(red["first_diag_code"], str) and bool(red["first_diag_code"]),
        "red: 严格档必须带可归属的诊断码（实测 %s:%s；退出码 1 也可能是崩溃 ⇒ 必须有诊断）"
        % (red["first_diag_code"], red["first_diag_line"]),
    )
    ok = all(bool(item["ok"]) for item in checks)
    diffs = sum(1 for row in (green, red) if not row["same"])
    payload = {
        "root": root,
        "bin": runner.bin_path,
        "files": 2,
        "diffs": diffs,
        "rows": [green, red],
        "selftest": {"ok": ok, "checks": checks},
    }
    if json_mode:
        emit_json(payload)
        print(
            "strict-compat selftest: %s（red loose=%s strict=%s first_diag=%s）"
            % (
                "ok" if ok else "FAILED",
                red["loose_exit"],
                red["strict_exit"],
                red["first_diag_code"],
            ),
            file=sys.stderr,
            flush=True,
        )
    else:
        print("strict-compat selftest")
        print("bin:  %s" % runner.bin_path)
        for item in checks:
            print(
                "  %s %s — %s"
                % ("PASS" if item["ok"] else "FAIL", item["name"], item["detail"])
            )
        print("ok" if ok else "FAILED")
    return EXIT_OK if ok else EXIT_FAIL


def emit_json(payload: Dict[str, object]) -> None:
    """stdout **只**这一个对象（`--json` 的契约）。"""
    sys.stdout.write(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
    sys.stdout.flush()


def resolve_root(raw: Optional[str]) -> str:
    """默认 = 本脚本的上一级（`scripts/` 的父目录 = 仓库根）。"""
    if raw:
        return os.path.abspath(os.path.expanduser(raw))
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.dirname(here)


def resolve_bin(root: str, raw: Optional[str]) -> str:
    """默认 `<root>/target/release/sokonanoda`（Windows 上退 `.exe`）；显式路径按 cwd 解析。"""
    if raw:
        return os.path.abspath(os.path.expanduser(raw))
    path = os.path.join(root, DEFAULT_BIN_REL)
    if not os.path.isfile(path) and os.name == "nt" and os.path.isfile(path + ".exe"):
        path += ".exe"
    return path


def validate_bin(path: str) -> Optional[str]:
    if not os.path.isfile(path):
        return "找不到二进制：%s（先构建，或用 --bin 指向已有产物）" % path
    if os.name != "nt" and not os.access(path, os.X_OK):
        return "二进制不可执行：%s" % path
    return None


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="strict-compat.py",
        description="严格档 v2 兼容性量具：同一份二进制跑宽松档与严格档，"
        "比较 grade --json 的 stdout 字节 + 退出码。",
        allow_abbrev=False,
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--sweep", action="store_true", help="递归扫描语料（默认动作）")
    mode.add_argument(
        "--selftest", action="store_true", help="在临时目录造 green/red 夹具自检量具有牙"
    )
    parser.add_argument(
        "--json", action="store_true", help="stdout 只输出单个 JSON 对象"
    )
    parser.add_argument(
        "--root", default=None, help="仓库根（默认：本脚本的上一级）"
    )
    parser.add_argument(
        "--bin",
        default=None,
        help="二进制路径（默认：<root>/target/release/sokonanoda）",
    )
    parser.add_argument(
        "--jobs", type=int, default=DEFAULT_JOBS, help="并发文件数（默认 %d）" % DEFAULT_JOBS
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=DEFAULT_TIMEOUT,
        metavar="SECONDS",
        help="每次 grade 的超时秒数（默认 %g）" % DEFAULT_TIMEOUT,
    )
    parser.add_argument(
        "--no-stage-stats",
        action="store_true",
        help="不注入 SOKO_STAGE_STATS=1（stderr 就没有 JUDGE_PREFIX，prefix 列变 null）",
    )
    parser.add_argument(
        "--keep-env",
        action="store_true",
        help="保留外部 SOKO* 变量（默认清掉，避免档位/读数随环境漂移）",
    )
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.jobs < 1:
        parser.error("--jobs 必须 ≥ 1")
    if args.timeout <= 0:
        parser.error("--timeout 必须 > 0")
    root = resolve_root(args.root)
    if not os.path.isdir(root):
        print("strict-compat: --root 不是目录：%s" % root, file=sys.stderr)
        if args.json:
            emit_json({"root": root, "bin": None, "error": "root 不是目录"})
        return EXIT_USAGE
    bin_path = resolve_bin(root, args.bin)
    problem = validate_bin(bin_path)
    if problem:
        print("strict-compat: %s" % problem, file=sys.stderr)
        if args.json:
            emit_json({"root": root, "bin": bin_path, "error": problem})
        return EXIT_USAGE
    runner = Runner(
        bin_path=bin_path,
        cwd=root,
        timeout=args.timeout,
        stage_stats=not args.no_stage_stats,
        keep_env=args.keep_env,
    )
    if args.selftest:
        return do_selftest(root, runner, args.json)
    return do_sweep(root, runner, args.jobs, args.json)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("strict-compat: 中断", file=sys.stderr)
        sys.exit(130)
