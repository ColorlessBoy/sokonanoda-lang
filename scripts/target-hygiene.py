#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""target-hygiene.py —— `target/` 体积守卫（2026-09-27 前置 W2 ✓）

**为什么必须有它** ✗✓（实测 ✓）：2026-09-27 实测 `target/` = **205 GB**
（`target/debug` 191 GB），而**磁盘只剩 159 GB** ⇒ 开 worktree 就爆盘 ✗。
拆开看，两个大头都是**没人回收**的：

  ① `target/debug/deps/*.rcgu.o` —— **966,858 个**（171 GB）✗
     文件名形如 `sokonanoda_front-<cratehash>.<每会话随机id>.0rfs8ra.rcgu.o`
     ⇒ **增量编译**（`-C incremental`）每个会话产生**一批新的** CGU 目标文件 ✗，
     **cargo 从不回收** ✗。最早产物 2026-09-06，叠上每天上百次 cargo 调用 ⇒ 爆炸 ✓。
  ② `target/debug/incremental/` —— **82 GB** ✗（同一根因的另一半 ✓）。

  同一个根因还解释了**sccache 命中率 0%** ✗✓：sccache 见到 `-C incremental`
  会**主动放弃缓存** ✓。

**结论 = 保住增量 ✓、磁盘另解 ✓**（用户 2026-09-27 明确：**性能/迭代速度最重要** ✓）：
  ⚠ **不许**为了省磁盘去关增量 ✗ —— 关掉会让「改一行就重编」变慢 ✗，而那正是
  用户最在意的东西 ✓。实测（探针 `/tmp/incr-probe.sh`，改内容强制真重编 ✓）：
    incremental **ON**  ⇒ 一次重编 **Δ = +233** 个 `.rcgu.o`（522 → 755）· incremental/ 426 MB
    incremental **OFF** ⇒ 一次重编 **Δ = 0**（CGU 目标名确定 ⇒ 原地覆盖）· incremental/ 0 B
  ⇒ 这两个数字**只作机制记录** ✓（见 `Cargo.toml` 的机制记录 ✓），
  **磁盘只**用「**守卫**（本脚本 ✓）+ **回收**（超阈时脚本会把命令打出来 ✓）」解 ✓。

**规则不落成守卫，几轮之后必然失效** ✓ —— 这条今天已经被验证过一次（用户原话 ✓）。
所以本脚本把"别再胖回去"变成**可判红的东西** ✓。

判据（可执行 ✓）：
  python3 scripts/target-hygiene.py            # 查仓库 target/
  python3 scripts/target-hygiene.py --json     # 单个 JSON 对象（给 agent 消费）
  python3 scripts/target-hygiene.py --selftest # 离线判据通道自检（不需要真 target/）

退出码：0 = 干净 · 1 = **超阈（红）** · 2 = 告警 · 3 = 用法/环境错误

⚠ 阈值不是拍的 ✓：`MAX_RCGU_O` 取自 **incremental 关掉之后**的实测值再留余量
（见 `docs/PERF.md` 或本轮 commit message 的实测数字 ✓）。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# ── 阈值（**实测校准** ✓，不是拍的 ✗）────────────────────────────────────────
# 实测（2026-09-27，incremental 关掉后）：见 commit message。
# 这里给的是"正常上限"，超了就是又开始堆积了 ✓。
# ⚠ 阈值**要高到日常开发不会误触发** ✓（用户 2026-09-27 定的口径 ✓）：
#   三周才涨到 205 GB ⇒ **40 GB ≈ "两周没清"** 的量级 ⇒ 日常碰不到 ✓。
#   全部取值都**远高于正常、又明显低于实测病态值** ✓（病态值见上面 docstring ✓）。
MAX_RCGU_O_WARN = 250_000              # deps/*.rcgu.o 个数（病态值 966,858）
MAX_RCGU_O = 600_000
MAX_TARGET_BYTES_WARN = 40 * 1024**3   # target/ 总量 **40 GB ⇒ 报警**（用户指定 ✓）
MAX_TARGET_BYTES = 100 * 1024**3       # 100 GB ⇒ 判红
MAX_INCREMENTAL_WARN = 20 * 1024**3    # target/debug/incremental/（病态值 82 GB）
MAX_INCREMENTAL_BYTES = 60 * 1024**3

EXIT_OK, EXIT_RED, EXIT_WARN, EXIT_USAGE = 0, 1, 2, 3


def human(n: int) -> str:
    for unit, div in (("GB", 1024**3), ("MB", 1024**2), ("KB", 1024)):
        if n >= div:
            return f"{n / div:.1f} {unit}"
    return f"{n} B"


def dir_bytes(p: Path) -> int:
    total = 0
    for root, _dirs, files in os.walk(p, onerror=lambda _e: None):
        for f in files:
            try:
                total += os.lstat(os.path.join(root, f)).st_size
            except OSError:
                pass
    return total


def scan(root: Path) -> dict:
    """把 `target/` 的形状量成一个纯数据 dict ✓（`judge()` 只吃这个 ⇒ 可离线自检 ✓）。"""
    deps = root / "debug" / "deps"
    rcgu = 0
    if deps.is_dir():
        try:
            rcgu = sum(1 for n in os.listdir(deps) if n.endswith(".rcgu.o"))
        except OSError:
            rcgu = -1  # 读不了 ⇒ 交给 judge 当异常

    incr = root / "debug" / "incremental"
    # **第二套 target**：`target/<x>/debug/deps` 这种"嵌套的 cargo 布局" ✗
    # （实测 `target/lang/` 12 GB 就是这种形状；来源至今没查清 ✓）
    nested: list[str] = []
    if root.is_dir():
        for child in sorted(root.iterdir()):
            if not child.is_dir():
                continue
            if (child / "debug" / "deps").is_dir() or (child / "release" / "deps").is_dir():
                nested.append(child.name)

    return {
        "exists": root.is_dir(),
        "target_bytes": dir_bytes(root) if root.is_dir() else 0,
        "rcgu_o": rcgu,
        "incremental_bytes": dir_bytes(incr) if incr.is_dir() else 0,
        "nested_targets": nested,
    }


def judge(stats: dict, th: dict | None = None) -> dict:
    """**纯函数** ✓ —— 只吃 stats，不碰文件系统 ⇒ `--selftest` 直接喂合成数据 ✓。"""
    defaults = {
        "max_rcgu": MAX_RCGU_O,
        "warn_rcgu": MAX_RCGU_O_WARN,
        "max_bytes": MAX_TARGET_BYTES,
        "warn_bytes": MAX_TARGET_BYTES_WARN,
        "max_incr": MAX_INCREMENTAL_BYTES,
        "warn_incr": MAX_INCREMENTAL_WARN,
    }
    # ⚠ **部分覆盖也要安全** ✓：caller 少给一个键不许 KeyError ✗（自检里就撞到过 ✓）
    t = {**defaults, **(th or {})}
    if not stats.get("exists"):
        return {"verdict": "skip", "exit_code": EXIT_OK, "red": [], "warn": [],
                "reason": "没有 target/ ⇒ 无需检查（CI 上就是这样 ✓）"}

    red: list[str] = []
    warn: list[str] = []

    if stats["rcgu_o"] < 0:
        red.append("读不了 target/debug/deps ⇒ 无法判定 ✗")
    elif stats["rcgu_o"] > t["max_rcgu"]:
        red.append(
            f"target/debug/deps/*.rcgu.o = {stats['rcgu_o']:,} 个 > 上限 {t['max_rcgu']:,} ✗"
            "（增量编译又在堆积 ⇒ 检查 Cargo.toml 的 incremental 是不是被打开了）"
        )
    elif stats["rcgu_o"] > t["warn_rcgu"]:
        warn.append(f"*.rcgu.o = {stats['rcgu_o']:,} 个 > 告警线 {t['warn_rcgu']:,}")

    if stats["target_bytes"] > t["max_bytes"]:
        red.append(
            f"target/ = {human(stats['target_bytes'])} > 上限 {human(t['max_bytes'])} ✗"
        )
    elif stats["target_bytes"] > t["warn_bytes"]:
        warn.append(f"target/ = {human(stats['target_bytes'])} > 告警线 {human(t['warn_bytes'])}")

    if stats["incremental_bytes"] > t["max_incr"]:
        red.append(
            f"target/debug/incremental/ = {human(stats['incremental_bytes'])}"
            f" > 上限 {human(t['max_incr'])} ✗（该回收了 ✓ —— 见下面的命令）"
        )
    elif stats["incremental_bytes"] > t["warn_incr"]:
        warn.append(
            f"target/debug/incremental/ = {human(stats['incremental_bytes'])}"
            f" > 告警线 {human(t['warn_incr'])}"
        )

    if stats["nested_targets"]:
        red.append(
            "**存在第二套 target** ✗：target/"
            + " · target/".join(stats["nested_targets"])
            + " —— 一个仓库只该有一套 target ✓（两套会互相不认识 ⇒ "
            "「本地明明编好了，跑起来还是旧的」✗）"
        )

    if red:
        return {"verdict": "red", "exit_code": EXIT_RED, "red": red, "warn": warn}
    if warn:
        return {"verdict": "warn", "exit_code": EXIT_WARN, "red": [], "warn": warn}
    return {"verdict": "ok", "exit_code": EXIT_OK, "red": [], "warn": []}


def render(stats: dict, res: dict) -> None:
    if res["verdict"] == "skip":
        print(f"target-hygiene ✓ 跳过：{res['reason']}")
        return
    print(
        f"target/ = {human(stats['target_bytes'])} · "
        f"deps/*.rcgu.o = {stats['rcgu_o']:,} 个 · "
        f"incremental/ = {human(stats['incremental_bytes'])} · "
        f"第二套 target = {stats['nested_targets'] or '无 ✓'}"
    )
    for w in res["warn"]:
        print(f"  ⚠ {w}")
    for r in res["red"]:
        print(f"  ✗ {r}")
    icon = {"ok": "✓ 干净", "warn": "⚠ 告警（未超上限）", "red": "✗ 判红"}[res["verdict"]]
    print(f"  ⇒ {icon}（exit {res['exit_code']}）")
    if res["verdict"] in ("warn", "red"):
        print()
        print("  **回收路径** ✓（⚠ 本机 `cargo clean` 会被 **EPERM** 拦 ✗ ——")
        print("   见 `docs/E2-HANDOVER.md` 陷阱清单；要用 `rm -rf`，批量删除阈值")
        print("   由 `CODEBUDDY_SAFE_DELETE_BULK_THRESHOLD` 放开 ✓）：")
        print("     rm -rf target/debug/deps target/debug/incremental")
        print("   ⚠ **不要**改 `Cargo.toml` 去关增量 ✗（用户 2026-09-27 拍板：保速度 ✓）")
        print("   ⚠ 清完 `deps` 后下一次构建会全量重编（慢一次，属预期 ✓）")


def selftest() -> int:
    """**故意喂已知形状** ✓：判据通道自己坏了必须在这里判红 ✓。"""
    cases: list[tuple[str, dict, str, int]] = [
        (
            "干净（正常形状：2 GB / 少量 .o / 无第二套 target）",
            {"exists": True, "target_bytes": 2 * 1024**3, "rcgu_o": 900,
             "incremental_bytes": 0, "nested_targets": []},
            "ok", 0,
        ),
        (
            "**rcgu.o 堆积**（= 2026-09-27 的 966,858 个）",
            {"exists": True, "target_bytes": 30 * 1024**3, "rcgu_o": 966_858,
             "incremental_bytes": 0, "nested_targets": []},
            "red", 1,
        ),
        (
            "target 总量爆掉（= 实测 205 GB）",
            {"exists": True, "target_bytes": 205 * 1024**3, "rcgu_o": 100,
             "incremental_bytes": 0, "nested_targets": []},
            "red", 1,
        ),
        (
            "**第二套 target**（= 实测 target/lang 12 GB）",
            {"exists": True, "target_bytes": 3 * 1024**3, "rcgu_o": 100,
             "incremental_bytes": 0, "nested_targets": ["lang"]},
            "red", 1,
        ),
        (
            "incremental 目录爆掉（= 实测 82 GB）",
            {"exists": True, "target_bytes": 10 * 1024**3, "rcgu_o": 100,
             "incremental_bytes": 82 * 1024**3, "nested_targets": []},
            "red", 1,
        ),
        (
            "告警线（50 GB > 40 GB 告警线，但 < 100 GB 红线 ⇒ 只告警）",
            {"exists": True, "target_bytes": 50 * 1024**3, "rcgu_o": 300_000,
             "incremental_bytes": 0, "nested_targets": []},
            "warn", 2,
        ),
        (
            "没有 target/（CI）⇒ 跳过，不误红",
            {"exists": False, "target_bytes": 0, "rcgu_o": 0,
             "incremental_bytes": 0, "nested_targets": []},
            "skip", 0,
        ),
    ]
    bad = 0
    for label, stats, want_v, want_c in cases:
        got = judge(stats)
        ok = got["verdict"] == want_v and got["exit_code"] == want_c
        print(
            f"  {'✓' if ok else '✗'} {label:<42} → {got['verdict']:<5}"
            f"(exit {got['exit_code']})  期望 {want_v} (exit {want_c})"
        )
        if not ok:
            bad += 1

    # **真扫一遍**：合成一个"有两套 target + rcgu.o 堆积"的目录树 ⇒ 必须判红 ✓
    tmp = Path(tempfile.mkdtemp(prefix="target-hygiene-"))
    try:
        (tmp / "debug" / "deps").mkdir(parents=True)
        for i in range(30):
            (tmp / "debug" / "deps" / f"c-1.{i}.rcgu.o").write_bytes(b"x")
        (tmp / "lang" / "debug" / "deps").mkdir(parents=True)
        st = scan(tmp)
        got = judge(st, th={"max_rcgu": 10, "warn_rcgu": 5,
                            "max_bytes": 10**9, "warn_bytes": 10**8,
                            "max_incr": 10**9})  # ← 故意**不给** warn_incr：验"部分覆盖"安全 ✓
        ok = (
            got["exit_code"] == EXIT_RED
            and st["rcgu_o"] == 30
            and st["nested_targets"] == ["lang"]
        )
        print(
            f"  {'✓' if ok else '✗'} {'真扫合成目录（30 个 .o + 第二套 target）':<42} → "
            f"rcgu_o={st['rcgu_o']} nested={st['nested_targets']} "
            f"exit={got['exit_code']}  期望 rcgu_o=30 nested=['lang'] exit=1"
        )
        if not ok:
            bad += 1
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print()
    total = len(cases) + 1
    if bad:
        print(f"✗ selftest 判红：{bad}/{total} 个用例不符 ✗")
        return 1
    print(f"✓ selftest 全过：{total}/{total} 个用例符合 ✓")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="target/ 体积守卫（rcgu.o 堆积 / 第二套 target）")
    ap.add_argument("--root", default=str(REPO_ROOT / "target"), help="target 目录（默认仓库 target/）")
    ap.add_argument("--json", action="store_true", help="输出单个 JSON 对象")
    ap.add_argument("--selftest", action="store_true", help="离线判据通道自检")
    a = ap.parse_args()

    if a.selftest:
        return selftest()

    root = Path(a.root)
    stats = scan(root)
    res = judge(stats)
    if a.json:
        print(json.dumps({"root": str(root), **stats, **res}, ensure_ascii=False, indent=2))
    else:
        render(stats, res)
    return res["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
