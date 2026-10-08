#!/usr/bin/env python3
"""C1 进度判据：**人看的终端**那条进度**逐文件**且**不长时间静止**（2026-10-08 用户口径）。

## 为什么需要它（P1 · `PLAN-cli-editor-perf.md` §1 P1）

用户原话要点：「build/rebuild 汇报太稀疏，**约 24 个文件才一条**」✗。根因是
`ProgressCounter` 的人看那条按 **每 10%** 打（`step = total/10`）⇒ 真课程 ~240 文件
就是每 24 个一行；而**模块级 tick 只喂 `--json`** ⇒ 一个入口编几十个闭包模块期间
终端**零输出** ✗。`scripts/check-progress-gap.py` 量的是 **`--json` 的事件流**
（机器通道，契约不变 ✓）—— 它**看不见人看的那条** ✗。本判据补的正是这个缺口。

## 判据（两条，都是**结构计数**，不比墙钟 ✓）

夹具自足（临时目录、不依赖 `courses/`）：

| 臂 | 夹具 | 期望 |
|---|---|---|
| ① **节奏** | **N=120** 个各自独立的 `*.sokonanoda`（每个一条定理） | 人看的进度行 **≥ N**（改前 `step=12` ⇒ 只有 **10** 条 ✗） |
| ② **心跳** | 一个含若干 `by` 证明的入口，`SOKO_BUILD_TICK_MS=200` | 人话心跳行 **≥ 2** 条（`… still building (`），且 stdout **没有** `build.tick` JSON |

* ②的周期用**显式开关**定 ⇒ 与机器快慢无关（结构判据 ✓）；不断言绝对毫秒。
* 两臂都读 **stderr**（人看的通道）——`--json` 的 stdout 契约**一个字都不许动** ✗。

`--selftest`（**反向验证**，硬要求）：喂**合成**的行流给同一个判据 ——
「每 24 个一行」的流（120 文件 5 行）**必须判红**；逐文件流（120 行）不红；
一行都没有（量不出来）⇒ **exit 2，绝不静默通过** ✓。

用法：
    python3 scripts/check-progress-cadence.py             # 判据（gate / CI）
    python3 scripts/check-progress-cadence.py --json
    python3 scripts/check-progress-cadence.py --selftest  # 反向验证（判据咬不咬得住）

退出码：0 = 过 · 1 = **人看的那条太稀/太静** · 2 = 环境/形状异常 · 3 = 拿不到二进制。
"""

from __future__ import annotations

import argparse
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
FILES = 120  # ① 的夹具文件数（判据：人看的行数 ≥ 它）
MIN_HEARTBEATS = 2  # ② 的判据
TICK_MS = 200  # ② 的显式心跳周期（结构判据：不依赖机器快慢）
# ② 的**自检**：夹具必须跑够久（≥ 这么多倍周期）—— 否则"心跳条数"这条判据是
# **空转**的 ✗（2026-10-08 实测踩到：内核提速后 40 条 `by` 证明只跑 0.23s ⇒
# 200ms 周期只塞得下 1 条 ⇒ 假红 ✗）。夹具不够慢 ⇒ **exit 2**，绝不静默通过 ✓。
MIN_WALL_PERIODS = 4
# ② 的夹具规模（条 `by` 证明）—— 目标是让墙钟 ≫ `TICK_MS × MIN_WALL_PERIODS`。
HEART_DECLS = 200

# 人看的**文件级**进度行：`… 42/120 · units/…/x.sokonanoda`（`build.rs` 的 `note_file`）。
FILE_LINE = re.compile(r"^\s*…\s+\d+/\d+\s+·\s+\S")
# **旧格式**（C1 之前）：`… 12/120 file(s)` —— 它**也要数** ✓：本判据量的是
# **节奏**（一个文件编完有没有出声），不是**格式** ✗。数旧格式的用处正是
# **反向验证**：拿 C1 之前的二进制跑同一判据，它必须因为「120 个文件只有 10 行」
# 判红 ✓ —— 而不是因为"格式不匹配 ⇒ 0 行"这种**假理由** ✗。
FILE_LINE_LEGACY = re.compile(r"^\s*…\s+\d+/\d+\s+file\(s\)\s*$")
# 人看的**心跳**行：`… still building (12s) · <path>`。
HEART_LINE = re.compile(r"^\s*…\s+still building\s+\(")
# 模块级 tick 的人看那一半：`… module 3/40 · <name>`。
MODULE_LINE = re.compile(r"^\s*…\s+module\s+\d+/\d+\s+·\s+\S")

SMALL_DECL = "theorem t{i} : Prop := Prop\n"
BIG_DECL = """\
theorem big_{i} (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
"""


def resolve_bin(explicit: str | None) -> tuple[list[str], str] | None:
    """与 `check-progress-gap.py` / `check-recompile-factor.py` 同一条解析链。"""
    if explicit:
        p = Path(explicit)
        return ([str(p)], f"explicit:{explicit}") if p.exists() else None
    launcher = REPO / "scripts" / "soko"
    if launcher.exists():
        node = shutil.which("node")
        if node:
            return ([node, str(launcher)], "scripts/soko")
        if os.access(launcher, os.X_OK):
            return ([str(launcher)], "scripts/soko")
    for profile in ("debug", "release"):
        cand = REPO / "target" / profile / "sokonanoda"
        if cand.exists() and os.access(cand, os.X_OK):
            return ([str(cand)], f"target/{profile}")
    return None


def judge_cadence(file_lines: int, total: int) -> tuple[int, str]:
    """① 的判据：人看的文件级行数 ≥ 文件数。"""
    if file_lines >= total:
        return 0, f"✓ 人看的进度 {file_lines} 行 ≥ {total} 个文件（逐文件 ✓）"
    return 1, (
        f"✗ 人看的进度只有 {file_lines} 行，而文件有 {total} 个 —— "
        f"「约 {total // max(file_lines, 1)} 个文件才一条」正是用户报的现象 ✗\n"
        f"   （`ProgressCounter::note_file` 必须**逐文件**打；见 PLAN-cli-editor-perf.md §1 P1）"
    )


def judge_heartbeat(beats: int, want: int) -> tuple[int, str]:
    if beats >= want:
        return 0, f"✓ 人话心跳 {beats} 条 ≥ {want}（终端不再长时间静止 ✓）"
    return 1, (
        f"✗ 人话心跳只有 {beats} 条（< {want}）—— 长编译期间终端会静止 ✗\n"
        f"   （`Heartbeat` 在非 `--json` 时该走 stderr 的人话；见 PLAN-cli-editor-perf.md §1 P1）"
    )


def classify(stderr_text: str) -> tuple[int, int, int]:
    """数三类人看的行（文件级 / 心跳 / 模块级）。**新旧两种文件级格式都算** ✓。"""
    files = beats = modules = 0
    for line in stderr_text.splitlines():
        if FILE_LINE.match(line) or FILE_LINE_LEGACY.match(line):
            files += 1
        elif HEART_LINE.match(line):
            beats += 1
        elif MODULE_LINE.match(line):
            modules += 1
    return files, beats, modules


def run_build(cmd: list[str], args: list[str], env_extra: dict[str, str], cache: Path,
              cwd: Path | None = None) -> tuple[int, str, str, float] | None:
    env = dict(os.environ)
    env["SOKONANODA_CACHE_DIR"] = str(cache)
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"
    env["SOKO_BUILD_NO_TICK"] = ""  # 清掉外部干扰（下面按需再设）
    env.pop("SOKO_BUILD_NO_TICK", None)
    env.update(env_extra)
    shutil.rmtree(cache, ignore_errors=True)
    t0 = time.time()
    proc = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8",
                          errors="replace", env=env, cwd=str(cwd or REPO), timeout=300)
    return proc.returncode, proc.stdout, proc.stderr, time.time() - t0


def selftest() -> int:
    """反向验证：**合成的行流**喂给同一个判据（确定性、不依赖真编译 ✓）。"""
    bad = 0

    def check(why: str, got: tuple[int, str], want_red: bool) -> None:
        nonlocal bad
        code, _ = got
        ok = (code != 0) if want_red else (code == 0)
        print(f"   [{'ok' if ok else 'BAD'}] {why}：exit {code}（期望 {'红' if want_red else '绿'}）")
        bad += 0 if ok else 1

    # 「每 24 个一行」——用户报的那个形状（120 文件 ⇒ 5 行）
    check("**造稀**：120 文件只有 5 行（每 24 个一行）⇒ 必须判红",
          judge_cadence(5, FILES), True)
    # 逐文件
    check("逐文件：120 文件 120 行 ⇒ 不红", judge_cadence(FILES, FILES), False)
    # 边界：119 行（差一条也不许放过）
    check("**造稀**：119/120 行 ⇒ 必须判红", judge_cadence(FILES - 1, FILES), True)
    # 心跳
    check("**造静**：0 条心跳 ⇒ 必须判红", judge_heartbeat(0, MIN_HEARTBEATS), True)
    check("心跳 2 条 ⇒ 不红", judge_heartbeat(2, MIN_HEARTBEATS), False)
    # 分类器本身：合成的 stderr
    synth = "\n".join([
        "scanning the current directory for *.sokonanoda …（跳过 .git / node_modules）",
        "found 120 .sokonanoda file(s)",
        "… 1/120 · a.sokonanoda",
        "… module 1/2 · Shared",
        "… still building (1s) · b.sokonanoda",
        "… 2/120 · b.sokonanoda",
        "built 120 file(s) — 0 hit, 120 compiled, 0 failed",
    ])
    files, beats, modules = classify(synth)
    ok = (files, beats, modules) == (2, 1, 1)
    print(f"   [{'ok' if ok else 'BAD'}] 分类器：合成流 ⇒ 文件 {files} / 心跳 {beats} / 模块 {modules}"
          f"（期望 2/1/1）")
    bad += 0 if ok else 1
    if bad:
        print(f"selftest: ✗ {bad} 条反向验证失败 —— 判据咬不住", file=sys.stderr)
        return 1
    print("selftest: ✓ 反向验证全过（造稀/造静判红、逐文件不红、分类器数得准）")
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--bin", default=None)
    ap.add_argument("--files", type=int, default=FILES, help="① 的夹具文件数（默认 120）")
    args = ap.parse_args(argv)

    if args.selftest:
        return selftest()

    resolved = resolve_bin(args.bin or os.environ.get("SOKONANODA_BIN"))
    if resolved is None:
        print("error: 拿不到可用的 sokonanoda CLI —— 进度判据无法判定（exit 3，不静默跳过）",
              file=sys.stderr)
        return 3
    cmd, source = resolved

    tmp = Path(tempfile.mkdtemp(prefix="c1-cadence-"))
    try:
        # ① 节奏臂：N 个各自独立的文件（无 import ⇒ 每个都是"入口"）。
        cadence_dir = tmp / "many"
        cadence_dir.mkdir(parents=True, exist_ok=True)
        for i in range(args.files):
            (cadence_dir / f"f{i:04}.sokonanoda").write_text(
                SMALL_DECL.format(i=i), encoding="utf-8")
        got = run_build(cmd, ["build", str(cadence_dir)], {}, tmp / ".cache1")
        if got is None:
            print("error: 节奏臂跑不起来 —— exit 2", file=sys.stderr)
            return 2
        code1, out1, err1, wall1 = got
        if code1 != 0:
            print(f"error: 节奏臂 exit {code1} —— exit 2\n{err1[-400:]}", file=sys.stderr)
            return 2
        files_seen, beats1, modules1 = classify(err1)
        cadence = judge_cadence(files_seen, args.files)

        # ② 心跳臂：一个入口 + 若干 `by` 证明，**显式周期**（结构判据）。
        beat_dir = tmp / "slow"
        beat_dir.mkdir(parents=True, exist_ok=True)
        body = "".join(BIG_DECL.format(i=i) for i in range(HEART_DECLS))
        (beat_dir / "slow.sokonanoda").write_text(body, encoding="utf-8")
        got = run_build(cmd, ["build", str(beat_dir)], {"SOKO_BUILD_TICK_MS": str(TICK_MS)},
                        tmp / ".cache2")
        if got is None:
            print("error: 心跳臂跑不起来 —— exit 2", file=sys.stderr)
            return 2
        code2, out2, err2, wall2 = got
        if code2 != 0:
            print(f"error: 心跳臂 exit {code2} —— exit 2\n{err2[-400:]}", file=sys.stderr)
            return 2
        _, beats2, _ = classify(err2)
        # **自检：夹具必须够慢**（否则"心跳 ≥2"恒真/恒假 ⇒ 判据咬不住 ✗）。
        if wall2 < (TICK_MS * MIN_WALL_PERIODS) / 1000.0:
            print(
                f"error: 心跳臂夹具只跑了 {wall2:.2f}s < {MIN_WALL_PERIODS} × {TICK_MS}ms —— "
                f"夹具不够慢 ⇒ 心跳判据**量不出来**（exit 2，绝不静默通过）✗\n"
                f"   修法：加大 `HEART_DECLS`（现 {HEART_DECLS}）",
                file=sys.stderr,
            )
            return 2
        heartbeat = judge_heartbeat(beats2, MIN_HEARTBEATS)
        # **机器契约不许动**：非 `--json` 的 stdout 里**不许**出现 `build.tick` JSON。
        stray_json = out2.count('"build.tick"')
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    codes = [cadence[0], heartbeat[0], 1 if stray_json else 0]
    code = max(codes)
    if args.json:
        print(json.dumps({
            "schema": "soko.progress-cadence/1", "bin": source,
            "files": args.files, "file_lines": files_seen, "module_lines": modules1,
            "heartbeats": beats2, "stray_json_ticks": stray_json,
            "heart_decls": HEART_DECLS, "min_wall_periods": MIN_WALL_PERIODS,
            "wall_s": [round(wall1, 2), round(wall2, 2)], "exit": code,
        }, ensure_ascii=False))
        return code
    print(f"进度判据（C1 · 人看的通道）：二进制 {source}")
    print(f"  ① 节奏：{args.files} 个文件 ⇒ 人看 {files_seen} 行（模块级 {modules1} 行）"
          f"· 墙钟 {wall1:.2f}s")
    print(f"     {cadence[1]}")
    print(f"  ② 心跳：周期 {TICK_MS}ms ⇒ 人话心跳 {beats2} 条 · 墙钟 {wall2:.2f}s"
          f"（自检：≥ {MIN_WALL_PERIODS} 个周期 ✓）")
    print(f"     {heartbeat[1]}")
    if stray_json:
        print(f"  ✗ 非 `--json` 的 stdout 里出现了 {stray_json} 条 `build.tick` JSON —— "
              f"机器契约不许动 ✗")
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
