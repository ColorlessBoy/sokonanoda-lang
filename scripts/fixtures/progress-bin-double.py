#!/usr/bin/env python3
"""进度判据的**二进制替身**（`scripts/check-progress.py` 的夹具缝 `--bin`）。

## 为什么需要它（第 7 轮 `gs7` · `SUB-narrow-guards-to-one-layer` 候选 ①）

两条进度判据（人看通道的**节奏** ⇄ 机器通道的**最长空档**）的**坏输入长在二进制里** ✗ ——
脚本里没有坏输入可钉。修前那两条守卫**一条咬住证明都没有**（实测 `guard-bite.jsonl` 命中 0），
而 `check-guard-bite.py` 的 **C1** 要求：**被改动的守卫必须有咬住证明** ✓。
⇒ 本夹具把**旧二进制的可观测行为**钉成仓库里的一份替身，咬住证明在**干净检出、无 cargo**
的 job（`queue-lint`）里也跑得起来 ✓。

## 两种形状（`SOKO_FIXTURE_SHAPE` 选；**默认 `legacy`**）

* **`legacy`** = **修前**的形状（两条缺陷同源：用户等待时**终端/事件流不动** ✗）：
  * 人看通道：`… {done}/{total} file(s)`，只在 `done` 是 `total/10` 的整数倍时打
    —— 逐字对 `b34a5be7^:crates/cli/src/build.rs:278`（`step: (total / 10).max(1)`）
    与 `:295-296`（`if self.human && (done.is_multiple_of(self.step) || done == self.total)`）
    ⇒ **120 个文件只有 10 行** = 用户 2026-10-08 报的"约 24 个文件才一条" ✓。
  * 机器通道：只有 `build.begin` / `build.file` / `build.summary`
    —— 逐字对 `6abaf649^:crates/cli/src/build.rs:74` / `:97` / `:110`
    ⇒ 一个入口（几十条 `by` 证明）期间**零事件**（实测课程入口 **39.0s** ✓）。
* **`current`** = **修后**的形状（判据该判绿的那一侧，供 `--selftest` 的**真绿跑**用 ✓）：
  * 人看通道：**逐文件**一行 `… {done}/{total} · <path>`（`crates/cli/src/build.rs:311-315`）
    + 人话心跳 `… still building (Ns) · <path>`（`:426`，走 stderr ✓）；
  * 机器通道：`build.begin` → `build.progress` + `build.decl`（`:303` / `:817`）→ `build.summary`，
    相邻两行间隔极小 ⇒ 最长空档远小于 `MAX_GAP_S` ✓。

⚠ **它是替身（double），不是编译出来的旧二进制** ✗：真旧二进制**不在仓库里**（也不该进 ✗），
而咬住证明必须**可复跑**。⇒ 形状**逐行钉在旧 rev 的源码上**，咬住证明自己现场
`git show <rev>:crates/cli/src/build.rs` 复核那几行**真的**长这样 ✓（不是"我说它长这样" ✗）。

⚠ **时间替身**（两处，都是"旧二进制要时间"的下界，不是新造的判据 ✗）：
* 人看通道：睡满 `5 × SOKO_BUILD_TICK_MS`（默认 0.9s）—— 让主判据的"夹具够慢"自检
  （`MIN_WALL_PERIODS × TICK_MS`）不把这次量测判成"量不出来"（真旧二进制在 200 条 `by`
  的夹具上实测 **0.23s+** ✓）；**一条心跳都不发**正是旧形状 ✓。
* 机器通道：`legacy` 在**第一条事件之后**睡 `SOKO_FIXTURE_GAP_S`（默认 **2.8s** > `MAX_GAP_S`
  的 2.5s）—— 复现"大文件期间零事件"（真旧二进制实测 **39.0s** ✓，这里取下界）。

用法（**只有判据/咬住证明调它**，人不用）：
    SOKO_FIXTURE_SHAPE=current python3 scripts/check-progress.py --layer cadence --bin scripts/fixtures/progress-bin-double.py
    python3 scripts/check-progress.py --layer gap --bin scripts/fixtures/progress-bin-double.py    # legacy ⇒ 判红 ✓

退出码：与真 CLI 同口径 —— 0 = 编译成功（夹具**只**模拟进度输出，不判课程内容 ✓）。
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

SHAPE = (os.environ.get("SOKO_FIXTURE_SHAPE") or "legacy").strip()
# legacy 的机器通道空档（真旧二进制实测 39.0s ⇒ 这里取"刚好越过阈值"的下界 ✓）。
LEGACY_GAP_S = float(os.environ.get("SOKO_FIXTURE_GAP_S") or "2.8")
# legacy 的人看通道要睡够"夹具够慢"自检的倍数（主判据里 `MIN_WALL_PERIODS = 4` ✓）。
LEGACY_TICK_FACTOR = 5
# current 的心跳条数（主判据要 ≥ 2 ⇒ 给 3，且总墙钟 ≥ 4 个周期 ✓）。
CURRENT_BEATS = 3


def decl_files(paths: list[str]) -> list[Path]:
    """夹具里的 `*.sokonanoda`（目录 ⇒ 递归；文件 ⇒ 它自己 ✓）。"""
    out: list[Path] = []
    for raw in paths:
        p = Path(raw)
        if p.is_dir():
            out.extend(sorted(x for x in p.rglob("*.sokonanoda") if x.is_file()))
        elif p.is_file():
            out.append(p)
    return out


def emit(obj: dict) -> None:
    """一行 JSON（**必须逐行 flush** ✗ —— 管道下块缓冲会把"空档"抹平成 0 ✗）。"""
    print(json.dumps(obj), flush=True)


def main(argv: list[str]) -> int:
    args = [a for a in argv if a not in ("build", "rebuild")]
    as_json = "--json" in args
    paths = [a for a in args if not a.startswith("-")]

    if "--clean" in args:
        if as_json:
            emit({"type": "build.clean", "removed": 0, "global": 0, "project": 0})
        else:
            print("removed 0 cached file(s) (0 global, 0 project)")
        return 0

    files = decl_files(paths)
    total = len(files)
    tick_ms = os.environ.get("SOKO_BUILD_TICK_MS")
    first = files[0].name if files else "x.sokonanoda"

    if as_json:
        emit({"type": "build.begin", "files": total})
        if SHAPE == "legacy":
            # 大文件期间**零事件**（旧形状：声明级/心跳两条通道都还没有 ✓）。
            time.sleep(LEGACY_GAP_S)
            for f in files:
                emit({"type": "build.file", "file": str(f), "status": "compiled"})
        else:
            for i, f in enumerate(files, 1):
                time.sleep(0.01)
                emit({"type": "build.progress", "done": i, "total": total,
                      "file": str(f)})
                emit({"type": "build.decl", "module": "M", "index": 0, "total": 1,
                      "file": str(f)})
        emit({"type": "build.summary", "files": total, "hit": 0,
              "compiled": total, "failed": 0})
        return 0

    # ── 人看通道（stderr ✓）：`--json` 的 stdout 契约一个字都不碰 ✗ ──────────
    print(f"found {total} .sokonanoda file(s)", file=sys.stderr, flush=True)
    if SHAPE == "legacy":
        step = max(total // 10, 1)  # `b34a5be7^:crates/cli/src/build.rs:278` ✓
        if tick_ms:
            time.sleep(LEGACY_TICK_FACTOR * int(tick_ms) / 1000.0)
        for done in range(step, total + 1, step):
            print(f"… {done}/{total} file(s)", file=sys.stderr, flush=True)
    else:
        for i, f in enumerate(files, 1):
            print(f"… {i}/{total} · {f.name}", file=sys.stderr, flush=True)
        if tick_ms:
            period = max(int(tick_ms), 1) / 1000.0
            for k in range(1, CURRENT_BEATS + 1):
                time.sleep(2 * period)  # 2 个周期一条 ⇒ 总墙钟 ≥ 4 个周期 ✓
                print(f"… still building ({k}s) · {first}", file=sys.stderr, flush=True)
    print(f"built {total} file(s) — 0 hit, {total} compiled, 0 failed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
