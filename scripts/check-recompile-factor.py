#!/usr/bin/env python3
"""重复功守卫（G-68）：一次 build 里**共享依赖被编了几次** —— 测**次数**，不测耗时。

## 为什么需要它（复盘原文 ⇒ `git log --all -- docs/perf/recompile-waste-retrospective-2026-09-28.md`）

既有性能纪律**全是相对量**：
* `perf-gate` 比的是"与 `docs/perf/ledger.jsonl` 上一次同名记录的 delta"（`docs/PERF.md:671-673`
  自己写着"**它不是'再快一点'，而是'以后慢下来会被发现'**"）⇒ **基线里已经包含的浪费在定义上
  不可见**：4.14× 重复功在每一条记录里都存在 ⇒ 恒定 `0%` delta；
* E17 的判据是"跨版本 ±4% + 每声明归一"（原文 ⇒ `git log --all -- docs/perf/E17-kernel-conclusion.md`：
  `16889/376` vs `9085/331` = **44.9 → 44.9 ms**）⇒ 常数倍率的功被归一化**除掉了**；
* `scripts/check-timing-evidence.py`（E18）只看 commit message 里有没有计时数字。

⇒ 所以必须有一条**绝对次数**的守卫。它测的不是"有没有变慢"，而是"**这些功是不是本来就不该做**"。

## 判据（ratchet，**只许减不许增**）

夹具（临时目录，自足、不依赖 courses/）：

```
lib/Shared.sokonanoda   恰好 **1 个 `by` 证明**（判据的分度就是它）
a/b/c.sokonanoda        各 `import lib.Shared`；自身**项风格**（0 个 `by`）
```

`SOKO_STAGE_STATS=1` 的 `by_calls` = **`by` 引擎被调用的次数** ⇒ 共享依赖每被 elaborate 一次
就 +1（入口自身不贡献）⇒ 它就是"共享依赖被编了几次"的**直接读数**：

| 状态 | `build a b c` 的 `by_calls` | 含义 |
|---|---|---|
| **今天**（per-entry-closure 键） | **3** | Shared 在三条闭包里各编一次 |
| 修好后（模块级环境复用） | **1** | Shared 只编一次，后两个入口复用 |

上界记在 `scripts/recompile-budget.json`（`max_by_calls`）：实测 **> 上界 ⇒ 判红**（exit 1）。
修好之后**把上界收紧到 1**（与 `docs-budget.json` 同一条纪律：只许减不许增）。

## 用法

    python3 scripts/check-recompile-factor.py             # 判据（gate / CI）
    python3 scripts/check-recompile-factor.py --json      # 机器可读
    python3 scripts/check-recompile-factor.py --selftest  # **反向验证**：造重复场景必须判红

退出码：0 = 未超上界 · 1 = **超上界（重复功变多）** · 2 = 环境/形状异常（**绝不静默通过**）
        · 3 = 拿不到可用的二进制（与 gate 的其它步骤同一条纪律：探不到就 exit 3）。
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
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BUDGET = REPO / "scripts" / "recompile-budget.json"

DEP_SRC = """\
-- G-68 夹具：共享依赖。**恰好一个 `by` 证明** —— 守卫的分度就是它。
theorem shared_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
"""

DEP_SRC_NAMED = """\
theorem {name}_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
"""

ENTRY_SRC = """\
import lib.{dep}
theorem {entry}_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := {fn} P Q hp hq
"""


def resolve_bin(explicit: str | None) -> tuple[list[str], str] | None:
    """可用的 CLI 调用前缀。优先显式覆盖 → `scripts/soko`（版本钉守卫）→ 仓库构建。

    用 `scripts/soko` 而不是直接抓二进制：启动器会**拒绝**版本不匹配/过期的构建
    （G-16），而那正是"量到未发布代码"的经典事故源。仓库构建是给没有 node 的
    CI 步骤留的退路（判据是**结构计数**，与 debug/release 无关）。
    """
    if explicit:
        p = Path(explicit)
        if p.exists() and os.access(p, os.X_OK):
            return ([str(p)], f"explicit:{explicit}")
        return None
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


def write_fixture(root: Path, entries: int, shared: bool) -> tuple[list[str], int, int]:
    """写夹具；返回 (入口路径, Σ闭包, 去重模块数)。"""
    (root / "lib").mkdir(parents=True, exist_ok=True)
    (root / "sokonanoda.toml").write_text('name = "g68-guard"\n', encoding="utf-8")
    entry_paths: list[str] = []
    if shared:
        (root / "lib" / "Shared.sokonanoda").write_text(DEP_SRC, encoding="utf-8")
        for i in range(entries):
            name = f"e{i}"
            (root / f"{name}.sokonanoda").write_text(
                ENTRY_SRC.format(dep="Shared", entry=name, fn="shared_and"), encoding="utf-8"
            )
            entry_paths.append(str(root / f"{name}.sokonanoda"))
        sigma = entries * 2          # 每个入口的闭包 = {Shared, 自己}
        distinct = entries + 1
    else:
        for i in range(entries):
            name = f"e{i}"
            (root / "lib" / f"Dep{name}.sokonanoda").write_text(
                DEP_SRC_NAMED.format(name=name), encoding="utf-8"
            )
            (root / f"{name}.sokonanoda").write_text(
                ENTRY_SRC.format(dep=f"Dep{name}", entry=name, fn=f"{name}_and"),
                encoding="utf-8",
            )
            entry_paths.append(str(root / f"{name}.sokonanoda"))
        sigma = entries * 2
        distinct = entries * 2
    return entry_paths, sigma, distinct


def measure(cmd: list[str], root: Path, entries: list[str], cache: Path) -> int | None:
    """冷编一次，返回 `by_calls`（拿不到 ⇒ None）。"""
    env = dict(os.environ)
    env["SOKONANODA_CACHE_DIR"] = str(cache)
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"  # 全落临时缓存，夹具目录零残留
    env["SOKO_STAGE_STATS"] = "1"
    shutil.rmtree(cache, ignore_errors=True)
    clean = subprocess.run(
        cmd + ["build", "--json", "--clean", str(root)],
        capture_output=True, text=True, env=env, cwd=REPO, timeout=120,
    )
    if clean.returncode != 0:
        return None
    run = subprocess.run(
        cmd + ["build", "--json", *entries],
        capture_output=True, text=True, env=env, cwd=REPO, timeout=600,
    )
    if run.returncode != 0:
        return None
    for line in reversed(run.stderr.splitlines()):
        if line.startswith("STAGE_STATS "):
            m = re.search(r"by_calls=(\d+)", line)
            if m:
                return int(m.group(1))
    return None


def judge(measured: int, budget: int) -> tuple[int, str]:
    """判据本体（纯函数，`--selftest` 直接喂它反例）。"""
    if measured <= budget:
        return 0, f"✓ by_calls={measured} ≤ 上界 {budget}"
    return 1, (
        f"✗ by_calls={measured} > 上界 {budget} —— 共享依赖被重复编译的次数**变多了**\n"
        f"   （今天 {budget} 次已经是 G-68 的缺口值；修好后应为 1 次，"
        f"届时把 {BUDGET.name} 的 max_by_calls 收紧到 1）"
    )


def load_budget() -> tuple[int, int]:
    data = json.loads(BUDGET.read_text(encoding="utf-8"))
    return int(data["fixture_shared_entries"]), int(data["max_by_calls"])


def run_once(cmd: list[str], entries: int, shared: bool) -> tuple[int | None, int, int]:
    tmp = Path(tempfile.mkdtemp(prefix="g68-guard-"))
    try:
        paths, sigma, distinct = write_fixture(tmp, entries, shared)
        got = measure(cmd, tmp, paths, tmp / ".cache")
        return got, sigma, distinct
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def selftest(cmd: list[str], budget: int) -> int:
    """**反向验证**：造重复场景必须判红；没有重复的场景必须不红；计数器坏了必须 exit 2。"""
    cases = [
        (1, True, "green", "单入口（无重复可言）"),
        (3, True, "green", "今天的基线（3 入口共享 1 依赖 = 3 次）"),
        (5, True, "red", "**造重复**：5 入口共享 1 依赖 = 5 次 ⇒ 必须判红"),
        (3, False, "green", "对照：3 个独立依赖（各编一次 = 3 次，不共享也无从省）"),
    ]
    bad = 0
    for entries, shared, want, why in cases:
        got, sigma, distinct = run_once(cmd, entries, shared)
        if got is None:
            print(f"   [exit2] {why}：量不出 by_calls（形状/环境异常）")
            return 2
        if got == 0:
            print(f"   [exit2] {why}：by_calls=0 ⇒ 计数器失效，守卫咬不住")
            return 2
        code, _ = judge(got, budget)
        ok = (code == 0) if want == "green" else (code != 0)
        print(f"   [{'ok' if ok else 'BAD'}] {why}：by_calls={got} Σ闭包={sigma} 去重={distinct} ⇒ exit {code}（期望 {want}）")
        if not ok:
            bad += 1
    if bad:
        print(f"selftest: ✗ {bad} 条反向验证失败 —— 守卫咬不住", file=sys.stderr)
        return 1
    print("selftest: ✓ 反向验证全过（重复场景判红、无重复场景不红、计数器失效判 2）")
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--bin", default=None, help="显式 CLI（默认：$SOKONANODA_BIN → scripts/soko → target/{debug,release}）")
    args = ap.parse_args(argv)

    if not BUDGET.exists():
        print(f"error: 找不到 {BUDGET} —— 上界无法判定（exit 3）", file=sys.stderr)
        return 3
    entries, budget = load_budget()

    resolved = resolve_bin(args.bin or os.environ.get("SOKONANODA_BIN"))
    if resolved is None:
        print("error: 拿不到可用的 sokonanoda CLI（node/scripts/soko 与 target/{debug,release} 都没有）"
              " —— 重复功无法判定（exit 3，不静默跳过）", file=sys.stderr)
        return 3
    cmd, source = resolved

    if args.selftest:
        return selftest(cmd, budget)

    got, sigma, distinct = run_once(cmd, entries, True)
    if got is None:
        print("error: 夹具跑不起来（build 失败或 SOKO_STAGE_STATS 没生效）—— exit 2", file=sys.stderr)
        return 2
    if got == 0:
        print("error: by_calls=0 ⇒ 计数器失效（守卫咬不住，绝不静默通过）—— exit 2", file=sys.stderr)
        return 2
    code, verdict = judge(got, budget)
    if args.json:
        print(json.dumps({
            "schema": "soko.recompile-factor/1",
            "bin": source,
            "fixture_entries": entries,
            "by_calls": got,
            "budget": budget,
            "sigma_closure": sigma,
            "distinct_modules": distinct,
            "exit": code,
        }, ensure_ascii=False))
        return code
    print(f"重复功守卫（G-68）：{entries} 个入口共享 1 个依赖（各 1 个 `by`）")
    print(f"  二进制：{source}")
    print(f"  Σ闭包 = {sigma} 次模块编译，去重后只要 {distinct} 个模块 ⇒ 重复 {sigma / distinct:.2f}×")
    print(f"  {verdict}")
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
