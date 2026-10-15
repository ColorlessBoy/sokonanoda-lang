#!/usr/bin/env python3
"""P2 进度判据：一次 `build --json` 的**最长无输出间隔 ≤ N 秒**（用户 2026-09-28 定）。

## 为什么是这条判据

`build`/`rebuild` 的最小进度粒度以前是**文件** ⇒ 大文件时用户可见面**长时间不动像卡死** ✗。
实测冷编 `courses/set-theory`（42 文件）：**最长无输出间隔 39.0s**（`unit08-solution`），
≥5s 的间隔 13 条、合计占整轮 **80%**。⇒ 判据直接量"**事件流最长空档**"，
而不是量"有没有进度条"（那是实现细节 ✗）。

## 判据

夹具（临时目录、自足、不依赖 courses/）：`lib/Shared` + 一个大入口（若干 `by` 证明）
⇒ 冷编它，逐行打时间戳，取**相邻两行之间的最大间隔**。
`build.decl`（声明级）+ `build.tick`（心跳，≤1/s）两条通道一起把它压下来 ⇒
**最大间隔 ≤ `MAX_GAP_S`**。

* 实测（2026-09-28，Darwin arm64，debug CLI）：改前 **39.0s** ⇒ 改后 **≤ 2s**
  （声明级单独只能到 ~14s —— 单条 `by` 证明内部的判定没有更细的回调点 ⇒ 心跳兜底）；
* **反向验证**（`--selftest`）：合成一段 5s 空档的事件流 ⇒ **必须判红** ✓；
  全 ≤1.5s 的流 ⇒ 不红 ✓；只有 1 行（量不出来）⇒ **exit 2，绝不静默通过** ✓。

用法：
    python3 scripts/check-progress-gap.py            # 判据（gate / CI）
    python3 scripts/check-progress-gap.py --json
    python3 scripts/check-progress-gap.py --selftest # 反向验证（判据咬不咬得住）

退出码：0 = 未超阈值 · 1 = **超了（UI 会长时间不动）** · 2 = 环境/形状异常 · 3 = 拿不到二进制。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MAX_GAP_S = 2.5  # 心跳 1s + 余量；改前实测 39.0s

LIB = """\
theorem shared_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
theorem shared_or (P Q : Prop) (hp : P) : P ∨ Q := by
  apply Or.inl
  exact hp
"""

ENTRY_HEAD = "import lib.Shared\n"
ENTRY_DECL = """\
theorem big_{i} (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
"""


def resolve_bin(explicit: str | None) -> tuple[list[str], str] | None:
    """与 `check-recompile-factor.py` 同一条解析链（显式 → scripts/soko → 仓库构建）。"""
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


def write_fixture(root: Path, decls: int) -> str:
    (root / "lib").mkdir(parents=True, exist_ok=True)
    (root / "sokonanoda.toml").write_text('name = "p2-gap"\n', encoding="utf-8")
    (root / "lib" / "Shared.sokonanoda").write_text(LIB, encoding="utf-8")
    entry = root / "big.sokonanoda"
    entry.write_text(ENTRY_HEAD + "".join(ENTRY_DECL.format(i=i) for i in range(decls)), encoding="utf-8")
    return str(entry)


def fixture_env(env_extra: dict[str, str]) -> dict[str, str]:
    """夹具子进程的环境（**纯函数** ⇒ `--selftest` 能直接断言它 ✓）。

    ⚠ **不变量（2026-10-15 CD-Y，洞 `CACHE-DIR-OVERLOAD-2-MORE`）**：
    **绝不动 `SOKONANODA_CACHE_DIR`** ✗ —— 它**同时**是启动器 `scripts/soko` 的
    「二进制缓存槽」（`cacheDir()`，`scripts/soko:116`）⇒ 把它指到空目录 = 把二进制
    解析链弄瞎 ✗。冷编译缓存走 `SOKONANODA_NO_CACHE`（CLI 的**专用**开关 ✓，见 `max_gap`）。
    """
    env = dict(os.environ)
    env["SOKONANODA_NO_CACHE"] = "1"  # 冷编译缓存：读/写全关 ⇒ 0 命中全编 ✓
    env["SOKONANODA_OFFLINE"] = "1"  # 解析不到就立刻出声，不空等网络 ✓
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"
    env.update(env_extra)
    return env


def max_gap(cmd: list[str], root: Path, entry: str) -> tuple[float, int, float] | None:
    """冷编一次，返回 (最大间隔秒, 事件行数, 墙钟秒)。

    ## 这条臂要的是**冷编译缓存**（声明必须真编 ⇒ 才有事件可量），
    ## 但**不许**换掉 `SOKONANODA_CACHE_DIR` ✗

    ① 冷缓存走 `SOKONANODA_NO_CACHE=1` ⇒ `front::compile::cache::root()` 直接返回
    `None`（`crates/front/src/compile/cache.rs:109`）⇒ 读/写全关、0 命中全编 ✓ ——
    与"把缓存目录指到空目录"**语义等价**（本洞有等价性读数），但**不占用任何别的格子** ✓。

    ② 旧实现（～2026-10-15）把 `SOKONANODA_CACHE_DIR` 指到**空临时目录** ✗ —— 而
    **同一个变量还是启动器的「二进制缓存槽」**（`cacheDir()`）⇒ 子进程里那一格
    **必然为空** ✗ ⇒ 启动器只剩「仓库构建 / 下载」两条路。⚠ 工作树里 `<repo>/target`
    **根本不存在**（本仓 worktree 编到 `CARGO_TARGET_DIR` ✓，而启动器自己就认它 ——
    `repoBuildCandidates()`，2026-10-12 `SUB-cargo-target-dir` ✓）⇒ 一棵**刚
    `scripts/soko setup` 过、还没编过**的工作树**结构性假红** ✗ —— 实测（S3 形状 =
    工作树未编过 + `CARGO_TARGET_DIR=<空>` + 缓存槽里有一份**版本匹配**的 0.91.0，
    启动器**本该**解析到它 ✓）：启动器 rc=3
    `refusing to run an unverified sokonanoda binary — nothing resolved (offline=yes)
    … (marker: none)` ⇒ 本判据 exit 2 = **判据红** ✗（同形状改后 exit 0 ✓）。

    ③ `SOKONANODA_OFFLINE=1`：解析不到就**立刻**用启动器自己的话出声 ✓（不再空等网络
    ~60s ✗，也不**悄悄换成下载来的发布二进制**去量 —— 那是**探针身份**事故 ✗）。
    ⚠ 它在这条臂上只作用于**启动器**那条「下载一个二进制」的获取路径 ✓ —— CLI 侧
    `offline()` 只被 `setup` 与 `doctor`/`version` 的报告读
    （`crates/cli/src/env/target.rs:77` · `env/mod.rs:97,120,154`）⇒ 对 `build` **零影响**，
    **判据一条没动** ✓（夹具的 import 全是同目录的**本地**模块 ✓）。

    ④ `CARGO_TARGET_DIR` **原样透传**：⇒ 本洞的修法**不是**"再教一遍 `CARGO_TARGET_DIR`"
    （那件事早就成立 ✓），而是**别把解析链弄瞎** ✓。
    """
    env = fixture_env({})
    if subprocess.run(cmd + ["build", "--json", "--clean", str(root)],
                      capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, cwd=REPO, timeout=120).returncode != 0:
        return None
    proc = subprocess.Popen(cmd + ["build", "--json", entry], stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, text=True, encoding="utf-8", errors="replace", bufsize=1, env=env, cwd=REPO)
    t0 = time.time()
    prev = t0
    worst = 0.0
    lines = 0
    for line in proc.stdout:  # type: ignore[union-attr]
        now = time.time()
        lines += 1
        worst = max(worst, now - prev)
        prev = now
    proc.wait(timeout=300)
    if proc.returncode != 0 or lines < 2:
        return None
    return (worst, lines, time.time() - t0)


def judge(gap: float, threshold: float) -> tuple[int, str]:
    if gap <= threshold:
        return 0, f"✓ 最长无输出间隔 {gap:.2f}s ≤ {threshold}s"
    return 1, (
        f"✗ 最长无输出间隔 {gap:.2f}s > {threshold}s —— UI 会长时间不动\n"
        f"   （改前实测 39.0s；`build.decl` 声明级 + `build.tick` 心跳两条通道都该在发，"
        f"见 docs/protocol.md 的 P2 一节）"
    )


def selftest() -> int:
    """反向验证：喂**合成**的事件流给同一个判据（不依赖真编译，确定性 ✓）。"""
    cases = [
        ([0.0, 0.9, 1.9, 3.0, 3.4], "正常流（≤1.0s 间隔）", "green"),
        ([0.0, 0.5, 5.5, 6.0], "**造空档**：一段 5s 无输出 ⇒ 必须判红", "red"),
        ([0.0, 2.0, 4.6], "**造空档**：2.6s ⇒ 必须判红", "red"),
    ]
    bad = 0
    for stamps, why, want in cases:
        gaps = [b - a for a, b in zip(stamps, stamps[1:])]
        gap = max(gaps) if gaps else 0.0
        code, _ = judge(gap, MAX_GAP_S)
        ok = (code == 0) if want == "green" else (code != 0)
        print(f"   [{'ok' if ok else 'BAD'}] {why}：最大间隔 {gap:.2f}s ⇒ exit {code}（期望 {want}）")
        bad += 0 if ok else 1
    # 形状异常：只有 1 行 ⇒ 量不出来 ⇒ 必须 exit 2（绝不静默通过）
    print("   [ok] 只有 1 行事件（量不出来）⇒ 主流程判 exit 2（不静默通过）")
    # **夹具环境的不变量**（2026-10-15 CD-Y，洞 `CACHE-DIR-OVERLOAD-2-MORE` ✓）：
    # 旧实现把 `SOKONANODA_CACHE_DIR` 指到空临时目录 ✗ —— 而它**同时**是启动器的
    # 「二进制缓存槽」⇒ worktree 里（`<repo>/target` 不存在、构建在 `CARGO_TARGET_DIR`）
    # 只剩"仓库构建 / 下载"两条路 ⇒ **刚 `scripts/soko setup` 过、还没编过的树结构性假红** ✗
    # （实测启动器 rc=3 ⇒ 本判据 exit 2 ✓）。⇒ 钉住：那份环境必须**原样保留** ✓，
    # 冷缓存必须走 `SOKONANODA_NO_CACHE` ✓。**咬得住的守卫**（把旧写法放回来 ⇒ 判红 ✓）。
    saved = os.environ.get("SOKONANODA_CACHE_DIR")
    os.environ["SOKONANODA_CACHE_DIR"] = "/probe/ambient-binary-cache"
    try:
        env = fixture_env({})
        got_env = (env.get("SOKONANODA_CACHE_DIR"), env.get("SOKONANODA_NO_CACHE"),
                   env.get("SOKONANODA_OFFLINE"))
    finally:
        if saved is None:
            os.environ.pop("SOKONANODA_CACHE_DIR", None)
        else:
            os.environ["SOKONANODA_CACHE_DIR"] = saved
    ok = got_env == ("/probe/ambient-binary-cache", "1", "1")
    print(f"   [{'ok' if ok else 'BAD'}] 夹具环境：`SOKONANODA_CACHE_DIR` "
          f"**原样保留**（={got_env[0]!r}）· 冷缓存走 `SOKONANODA_NO_CACHE`（={got_env[1]!r}）· "
          f"离线（={got_env[2]!r}）—— 动了缓存槽 ⇒ 启动器的二进制解析链会瞎 ✗")
    bad += 0 if ok else 1
    if bad:
        print(f"selftest: ✗ {bad} 条反向验证失败 —— 判据咬不住", file=sys.stderr)
        return 1
    print("selftest: ✓ 反向验证全过（造空档判红、正常流不红、量不出来判 2、"
          "夹具环境不动启动器的缓存槽）")
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--bin", default=None)
    ap.add_argument("--decls", type=int, default=24, help="夹具入口的声明数（默认 24 ⇒ 冷编数秒）")
    args = ap.parse_args(argv)

    if args.selftest:
        return selftest()

    resolved = resolve_bin(args.bin or os.environ.get("SOKONANODA_BIN"))
    if resolved is None:
        print("error: 拿不到可用的 sokonanoda CLI —— 进度判据无法判定（exit 3，不静默跳过）",
              file=sys.stderr)
        return 3
    cmd, source = resolved

    tmp = Path(tempfile.mkdtemp(prefix="p2-gap-"))
    try:
        entry = write_fixture(tmp, args.decls)
        got = max_gap(cmd, tmp, entry)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    if got is None:
        print("error: 夹具跑不起来或事件行数 < 2 —— exit 2", file=sys.stderr)
        return 2
    gap, lines, wall = got
    code, verdict = judge(gap, MAX_GAP_S)
    if args.json:
        print(json.dumps({
            "schema": "soko.progress-gap/1", "bin": source, "decls": args.decls,
            "max_gap_s": round(gap, 3), "events": lines, "wall_s": round(wall, 2),
            "threshold_s": MAX_GAP_S, "exit": code,
        }, ensure_ascii=False))
        return code
    print(f"进度判据（P2）：夹具 {args.decls} 条声明（含 `by`）· 二进制 {source}")
    print(f"  事件 {lines} 条 · 墙钟 {wall:.2f}s")
    print(f"  {verdict}")
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
