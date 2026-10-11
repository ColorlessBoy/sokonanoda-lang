#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""target-hygiene.py —— `target/` 体积守卫（2026-09-27 前置 W2 ✓；2026-10-11 修度量 + 进 gate ✓）

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
  **磁盘只**用「**守卫**（本脚本 ✓）+ **回收**（`--reclaim` ✓）」解 ✓。

**规则不落成守卫，几轮之后必然失效** ✓ —— 这条今天已经被验证过一次（用户原话 ✓）。
所以本脚本把"别再胖回去"变成**可判红的东西** ✓。

⚠ **2026-10-11 修的三处**（用户：「浪费我太多 ssd，好麻烦」✓）：

  **(1) 度量口径错了（高报 60%）** ✗ —— 旧版 `dir_bytes()` 按 `st_size` 求和、
  **不认硬链接** ✗。实测同一天：脚本报 `target/ = 228.0 GB`，而 `du -sh` = **142 GiB**
  ⇒ 差 **86 GiB** 全是**同 inode 的多重链接**（`deps` 内部 55.7 GiB + `incremental`
  内部 30.4 GiB，两处**各自**重复，不是互相共享 —— 判据：`du` 的
  `88 + 53 + 1.3 ≈ 142` ✓）。**判红结论没错**（142 > 100 上限 ✓），但**数字是谎的** ✗，
  阈值附近会误判 ⇒ 现在改用 **`st_blocks × 512` + 按 `(dev, ino)` 去重** ✓
  （只在 `st_nlink > 1` 时才查重 ⇒ 内存只跟"有链接的文件数"走，不跟文件总数走 ✓）。
  口径从此与 `du` 一致 ✓ —— **「占了多少 SSD」就是判据本身** ✓。

  **(2) 守卫没接线** ✗ —— `grep -rn target-hygiene scripts/soko .github/workflows/` = **0** ✗
  ⇒ 从不自动跑，**这次是用户问起才发现已经 142 GiB** ✗。现在进 `scripts/soko gate` ✓
  （含 `--selftest` ✓）。**为什么不进 CI**：CI runner 是**一次性**的，`target/` 天生只有
  一次构建 ⇒ 这条守卫在那边**没有意义**（而且 `scripts/soko gate` 本来就不被 CI 调用 ✓，
  CI 按 job 拆开各跑各的同源判据 ✓）。

  **(3) 文档指针是死的** ✗ —— 旧版叫读者去看 `docs/E2-HANDOVER.md` 的陷阱清单，
  而那份**已于 2026-09-29 有意删除**（`docs/README.md`：**已删除（不归档）** ✓）。
  同一条还声称「本机 `cargo clean` 会被 EPERM 拦」—— 2026-10-11 在 `/tmp` 合成工程
  实测 **`cargo clean` exit 0** ✓（那是当年 CodeBuddy 沙箱的产物，不是 cargo 的性质 ✓）。
  ⇒ 死指针删掉 ✓；`--reclaim` 仍走**定向** `rm -rf`（**有意不调 `cargo clean`**：
  它会连 `target/release` 与 `target/gate-cache` 一起端掉，而前者正是 `scripts/soko`
  解析仓库构建时要用的 ✓）。

判据（可执行 ✓）：
  python3 scripts/target-hygiene.py            # 查仓库 target/（+ 同级第二套 target）
  python3 scripts/target-hygiene.py --json     # 单个 JSON 对象（给 agent 消费）
  python3 scripts/target-hygiene.py --reclaim  # **回收**（定向 rm -rf；先确认没有构建在跑）
  python3 scripts/target-hygiene.py --selftest # 离线判据通道自检（不需要真 target/）

退出码：0 = 干净 · 1 = **超阈（红）** · 2 = 告警 · 3 = 用法/环境错误

⚠ 阈值不是拍的 ✓：`MAX_RCGU_O` 取自 **incremental 关掉之后**的实测值再留余量
（见 `Cargo.toml` 的机制记录 ✓）。**2026-10-11 修口径后阈值原样保留** ✓ ——
它们当初就是照 `du` 量级定的（病态读数 205 GB 是 `du` 口径 ✓），现在**度量终于
和它们同口径**了 ✓；`AGENTS.md`「不许靠抬上限达标」⇒ 一条都没动 ✓。
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
# 同级第二套 target（例如 `fuzz/target`）—— **不是**红的：`fuzz/` 有自己的
# `Cargo.toml`、**不在 workspace members 里** ⇒ 它自带一套 target 是**正常**的 ✓。
# 但它是**同一块盘上的**可回收空间 ⇒ 大到值得看一眼时报一声 ✓（小的时候闭嘴，
# 否则每次 gate 都刷一行噪声 ⇒ 用户会把整条守卫当背景音 ✗）。
SIBLING_WARN_BYTES = 1 * 1024**3

BLOCK = 512  # `st_blocks` 的单位（POSIX 固定 512 B，与 `du` 同源 ✓）

EXIT_OK, EXIT_RED, EXIT_WARN, EXIT_USAGE = 0, 1, 2, 3

# 找同级 target 时要跳过的重目录（`.git` 里不可能有 cargo 布局 ✓；
# `node_modules`/`.vscode-test` 是几万个文件级的噪声源 ✓）。
_PRUNE_DIRS = {".git", "node_modules", ".vscode-test", ".sokonanoda", "__pycache__"}


def human(n: int) -> str:
    for unit, div in (("GB", 1024**3), ("MB", 1024**2), ("KB", 1024)):
        if n >= div:
            return f"{n / div:.1f} {unit}"
    return f"{n} B"


# ── 度量（2026-10-11 重写：一次遍历 + 按 inode 去重 + `st_blocks` ✓）──────────

def _iter_files(root: Path):
    """递归产出文件条目 —— 用 `os.scandir` 的 `d_type`（**目录不再多一次 stat** ✓）。

    ⚠ 有意**不用** `os.walk`：它内部已经 scandir 过一遍，外面再 `os.lstat`
    等于每个文件 stat **两次** ✗（142 GiB / 100 万文件时这是几十秒的差别 ✓）。
    """
    stack = [str(root)]
    while stack:
        d = stack.pop()
        try:
            with os.scandir(d) as it:
                for e in it:
                    try:
                        if e.is_dir(follow_symlinks=False):
                            stack.append(e.path)
                        elif e.is_file(follow_symlinks=False):
                            yield e
                    except OSError:
                        continue
        except OSError:
            continue


def _add(st: os.stat_result, seen: set) -> int:
    """该文件**实际占用的字节**（`st_blocks × 512` ✓）；硬链接只算**第一次** ✓。

    ⚠ 只在 `st_nlink > 1` 时查重 ⇒ 单链接文件（绝大多数）零额外开销 ✓；
    查重表的大小只跟"有链接的文件数"走（实测本仓 147k），不跟文件总数走（617k）✓。
    """
    b = st.st_blocks * BLOCK
    if st.st_nlink > 1:
        key = (st.st_dev, st.st_ino)
        if key in seen:
            return 0
        seen.add(key)
    return b


def subtree_bytes(p: Path) -> int:
    """单个子树的**独立**占用（自己内部去重 ✓）—— 回答"删掉它能拿回多少" ✓。

    ⚠ 两个子树若**互相**共享 inode，它们的和会大于 `target/` 的总量 ✗ ——
    2026-10-11 实测本仓 `deps` 与 `incremental` **互不共享** ✓
    （判据：`du` 的 `88 + 53 + 1.3 ≈ 142` ✓），所以这里不必做全局归属 ✓。
    """
    seen: set = set()
    total = 0
    for e in _iter_files(p):
        try:
            st = e.stat(follow_symlinks=False)
        except OSError:
            continue
        total += _add(st, seen)
    return total


def find_sibling_targets(repo: Path, main_target: Path, max_depth: int = 3) -> list[Path]:
    """找**与主 `target/` 平行**的那几套（实测形状：`fuzz/target` ✓）。

    ⚠ 旧版只查 `target/` **里面**嵌套的（`target/lang/` 那种 ✗），同级的一律看不见 ✗
    —— 而"同一块盘上的另一套 target"正是用户关心的空间 ✓。
    深度 ≤ `max_depth`、跳 `_PRUNE_DIRS` ⇒ 毫秒级 ✓（不为了找 target 去遍历全仓 ✗）。
    """
    out: list[Path] = []
    try:
        main_res = main_target.resolve()
    except OSError:
        main_res = main_target

    def rec(d: Path, depth: int) -> None:
        if depth > max_depth:
            return
        try:
            with os.scandir(d) as it:
                for e in it:
                    try:
                        if not e.is_dir(follow_symlinks=False):
                            continue
                    except OSError:
                        continue
                    if e.name == "target":
                        p = Path(e.path)
                        try:
                            if p.resolve() == main_res:
                                continue
                        except OSError:
                            pass
                        # 只认**真的是 cargo 布局**的那种（有 `debug|release/deps` ✓），
                        # 免得把随便一个叫 target 的目录算进来 ✗。
                        if (p / "debug" / "deps").is_dir() or (p / "release" / "deps").is_dir():
                            out.append(p)
                        continue
                    if e.name in _PRUNE_DIRS or e.name.startswith("."):
                        continue
                    rec(Path(e.path), depth + 1)
        except OSError:
            pass

    rec(repo, 1)
    return sorted(out)


def scan(root: Path, repo: Path | None = None) -> dict:
    """把 `target/` 的形状量成一个纯数据 dict ✓（`judge()` 只吃这个 ⇒ 可离线自检 ✓）。"""
    deps = root / "debug" / "deps"
    incr = root / "debug" / "incremental"

    stats = {
        "exists": root.is_dir(),
        "target_bytes": 0,
        "files": 0,
        "rcgu_o": 0,
        "deps_bytes": 0,
        "incremental_bytes": 0,
        "nested_targets": [],
        "sibling_targets": [],
    }
    if not stats["exists"]:
        return stats

    deps_pfx = str(deps) + os.sep
    incr_pfx = str(incr) + os.sep
    seen_global: set = set()
    seen_deps: set = set()
    seen_incr: set = set()

    for e in _iter_files(root):
        try:
            st = e.stat(follow_symlinks=False)
        except OSError:
            continue
        stats["files"] += 1
        stats["target_bytes"] += _add(st, seen_global)
        p = e.path
        if p.startswith(deps_pfx):
            if e.name.endswith(".rcgu.o"):
                stats["rcgu_o"] += 1
            stats["deps_bytes"] += _add(st, seen_deps)
        elif p.startswith(incr_pfx):
            stats["incremental_bytes"] += _add(st, seen_incr)

    # **第二套 target**：`target/<x>/debug/deps` 这种"嵌套的 cargo 布局" ✗
    # （实测 `target/lang/` 12 GB 就是这种形状；来源至今没查清 ✓）
    for child in sorted(root.iterdir()) if root.is_dir() else []:
        if child.is_dir() and ((child / "debug" / "deps").is_dir() or (child / "release" / "deps").is_dir()):
            stats["nested_targets"].append(child.name)

    # **同级**第二套 target（`fuzz/target`）—— 记路径（相对仓库）+ 占用 ✓
    if repo is not None and repo.is_dir():
        for sib in find_sibling_targets(repo, root):
            try:
                rel = str(sib.relative_to(repo))
            except ValueError:
                rel = str(sib)
            stats["sibling_targets"].append({"path": rel, "bytes": subtree_bytes(sib)})

    return stats


def judge(stats: dict, th: dict | None = None) -> dict:
    """**纯函数** ✓ —— 只吃 stats，不碰文件系统 ⇒ `--selftest` 直接喂合成数据 ✓。"""
    defaults = {
        "max_rcgu": MAX_RCGU_O,
        "warn_rcgu": MAX_RCGU_O_WARN,
        "max_bytes": MAX_TARGET_BYTES,
        "warn_bytes": MAX_TARGET_BYTES_WARN,
        "max_incr": MAX_INCREMENTAL_BYTES,
        "warn_incr": MAX_INCREMENTAL_WARN,
        "warn_sibling": SIBLING_WARN_BYTES,
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
            "（增量编译又在堆积 ⇒ 跑 --reclaim 回收）"
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
            f" > 上限 {human(t['max_incr'])} ✗（该回收了 ✓ —— 跑 --reclaim）"
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

    # 同级第二套 target：**不是**红的（`fuzz/` 是独立工程 ⇒ 自带 target 正常 ✓），
    # 只在**大到值得看一眼**时报一声 ✓。
    for sib in stats.get("sibling_targets", []):
        if sib["bytes"] > t["warn_sibling"]:
            warn.append(
                f"同级第二套 target：{sib['path']}/ = {human(sib['bytes'])}"
                "（独立 cargo 工程 ⇒ 正常，但同在一块盘上 ⇒ 要腾空间时它也算 ✓）"
            )

    if red:
        return {"verdict": "red", "exit_code": EXIT_RED, "red": red, "warn": warn}
    if warn:
        return {"verdict": "warn", "exit_code": EXIT_WARN, "red": [], "warn": warn}
    return {"verdict": "ok", "exit_code": EXIT_OK, "red": [], "warn": []}


# ── 回收（2026-10-11 新增：把"记得住 rm -rf"变成一条命令 ✓）──────────────────

def reclaim_plan(root: Path) -> list[Path]:
    """**纯函数** ✓：要删哪几个目录 ⇒ 自检可以直接断言，不必真删 ✓。

    ⚠ 有意**只**删这两个（`debug/deps` + `debug/incremental`）✓：
    `target/release`（`scripts/soko` 解析仓库构建时要用 ✓）、`target/gate-cache`
    （课程门禁的持久缓存 ✓）、`target/debug/<二进制>`（判卷直接跑的就是它 ✓）
    **都不在**计划里 ⇒ 清完工具链照常可用 ✓（2026-10-11 实测 ✓）。
    """
    return [root / "debug" / "deps", root / "debug" / "incremental"]


def format_builder_line(line: str, limit: int = 140) -> str:
    """**纯函数** ✓（可离线自检 ✓）：把 `pgrep` 那一行截到 `limit` 字符以内。"""
    line = line.strip()
    return line if len(line) <= limit else line[: limit - 3] + "…"


def _builders_running() -> list[str]:
    """有没有 cargo/rustc 在跑 —— 有就**不许**删（删到一半的构建目录是坏的 ✗）。

    ⚠ 输出**必须截断** ✗✓（2026-10-11 实测踩到 ✓）：`rustc` 的命令行是**几千字符**
    （几十个 `--check-cfg` / `-L` / `--extern`）⇒ 原样打印会**刷满终端**，
    真正要看的"是谁在跑"反而看不见 ✗。
    """
    try:
        out = subprocess.run(
            ["pgrep", "-fl", "cargo|rustc"],
            capture_output=True, text=True, timeout=10,
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return []  # 探不到就不阻断（宁可放行，也不许因为探针坏了就永久拦住回收 ✗）
    me = os.getpid()
    hits = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        pid = line.split(maxsplit=1)[0]
        if pid.isdigit() and int(pid) == me:
            continue
        # 排除探针自己（`pgrep -f` 会匹配到本进程的命令行 ✓）
        if "target-hygiene" in line:
            continue
        hits.append(format_builder_line(line))
    return hits


def reclaim(root: Path, repo: Path | None = None) -> int:
    """定向回收 ⇒ 打印"删了什么 · 拿回多少" ✓（数字必须落地，不许只说"清好了" ✗）。"""
    if not root.is_dir():
        print(f"target-hygiene --reclaim：没有 {root} ⇒ 无需回收 ✓")
        return EXIT_OK

    running = _builders_running()
    if running:
        print("✗ 有 cargo/rustc 在跑 ⇒ 拒绝回收（删到一半的构建目录是坏的 ✗）：")
        for line in running[:5]:
            print(f"    {line}")
        print("  等它跑完再试 ✓。")
        return EXIT_USAGE

    before = scan(root, repo)
    plan = reclaim_plan(root)
    # 防御：计划里的每一条都必须在 root **里面**（`--root` 给错时不许误删别处 ✗）
    root_res = root.resolve()
    for p in plan:
        try:
            p.resolve().relative_to(root_res)
        except ValueError:
            print(f"✗ 拒绝回收：{p} 不在 {root_res} 里面 ✗（--root 给错了吗？）")
            return EXIT_USAGE

    print(f"回收前：target/ = {human(before['target_bytes'])}"
          f"（deps {human(before['deps_bytes'])} · incremental {human(before['incremental_bytes'])}"
          f" · *.rcgu.o {before['rcgu_o']:,} 个）")
    t0 = time.time()
    freed = 0
    for p in plan:
        if not p.exists():
            continue
        freed += subtree_bytes(p)
        shutil.rmtree(p, ignore_errors=True)
    elapsed = time.time() - t0

    after = scan(root, repo)
    print(f"回收后：target/ = {human(after['target_bytes'])}"
          f"（*.rcgu.o {after['rcgu_o']:,} 个）")
    print(f"⇒ 删掉 {len(plan)} 个目录 · 实际释放 **{human(freed)}** · 耗时 {elapsed:.0f}s ✓")
    print("  ⚠ 下一次 cargo 构建会**全量重编**一次（慢一次，属预期 ✓）；"
          "`scripts/soko` 不受影响（二进制与 release 都还在 ✓）。")
    return EXIT_OK


def render(stats: dict, res: dict, root: Path) -> None:
    if res["verdict"] == "skip":
        print(f"target-hygiene ✓ 跳过：{res['reason']}")
        return
    print(
        f"target/ = {human(stats['target_bytes'])}（磁盘占用口径 ✓） · "
        f"deps = {human(stats['deps_bytes'])} · "
        f"deps/*.rcgu.o = {stats['rcgu_o']:,} 个 · "
        f"incremental/ = {human(stats['incremental_bytes'])}"
    )
    if stats.get("sibling_targets"):
        for sib in stats["sibling_targets"]:
            print(f"  同级 target：{sib['path']}/ = {human(sib['bytes'])}")
    else:
        print("  同级第二套 target = 无 ✓")
    print(f"  target/ 里嵌套的第二套 target = {stats['nested_targets'] or '无 ✓'}")
    for w in res["warn"]:
        print(f"  ⚠ {w}")
    for r in res["red"]:
        print(f"  ✗ {r}")
    icon = {"ok": "✓ 干净", "warn": "⚠ 告警（未超上限）", "red": "✗ 判红"}[res["verdict"]]
    print(f"  ⇒ {icon}（exit {res['exit_code']}）")
    if res["verdict"] in ("warn", "red"):
        print()
        print("  **回收路径** ✓（一条命令，先确认没有构建在跑 ✓）：")
        print(f"     python3 {Path(__file__).resolve().relative_to(REPO_ROOT)} --reclaim")
        print("   ⚠ **不要**改 `Cargo.toml` 去关增量 ✗（用户 2026-09-27 拍板：保速度 ✓）")
        print("   ⚠ 清完 `deps` 后下一次构建会全量重编（慢一次，属预期 ✓）")


# ── 自检（**判据通道自己坏了必须在这里判红** ✓）────────────────────────────

def _fixture_deps_bytes(root: Path, nlinks: int) -> tuple[int, int]:
    """造一个**真**硬链接夹具 ⇒ 返回 `(subtree_bytes 读数, 唯一字节数)` ✓。

    ⚠ 这条是**反向验证**的落点 ✓：旧版 `dir_bytes()`（按 `st_size` 求和）在这里
    会读出 `nlinks ×` ✗ —— 判据必须**咬得住** 2026-10-11 那个高报 60% 的 bug ✓。
    """
    d = root / "debug" / "deps"
    d.mkdir(parents=True, exist_ok=True)
    src = d / "sokonanoda_front-aaaa.1111111.0aaaaaa.rcgu.o"
    src.write_bytes(b"x" * (256 * 1024))  # 256 KB ⇒ 块取整误差可忽略 ✓
    for i in range(nlinks - 1):
        os.link(src, d / f"sokonanoda_front-bbbb{i}.2222222.0bbbbbb{i}.rcgu.o")
    unique = os.lstat(src).st_blocks * BLOCK
    return subtree_bytes(d), unique


def selftest() -> int:
    """**故意喂已知形状** ✓：判据通道自己坏了必须在这里判红 ✓。"""
    bad = 0
    total = 0

    def check(label: str, ok: bool, detail: str = "") -> None:
        nonlocal bad, total
        total += 1
        print(f"  {'✓' if ok else '✗'} {label:<52} {detail}")
        if not ok:
            bad += 1

    cases: list[tuple[str, dict, str, int]] = [
        (
            "干净（正常形状：2 GB / 少量 .o / 无第二套 target）",
            {"exists": True, "target_bytes": 2 * 1024**3, "rcgu_o": 900,
             "incremental_bytes": 0, "nested_targets": [], "sibling_targets": []},
            "ok", 0,
        ),
        (
            "**rcgu.o 堆积**（= 2026-09-27 的 966,858 个）",
            {"exists": True, "target_bytes": 30 * 1024**3, "rcgu_o": 966_858,
             "incremental_bytes": 0, "nested_targets": [], "sibling_targets": []},
            "red", 1,
        ),
        (
            "target 总量爆掉（= 实测 205 GB）",
            {"exists": True, "target_bytes": 205 * 1024**3, "rcgu_o": 100,
             "incremental_bytes": 0, "nested_targets": [], "sibling_targets": []},
            "red", 1,
        ),
        (
            "**第二套 target**（= 实测 target/lang 12 GB）",
            {"exists": True, "target_bytes": 3 * 1024**3, "rcgu_o": 100,
             "incremental_bytes": 0, "nested_targets": ["lang"], "sibling_targets": []},
            "red", 1,
        ),
        (
            "incremental 目录爆掉（= 实测 82 GB）",
            {"exists": True, "target_bytes": 10 * 1024**3, "rcgu_o": 100,
             "incremental_bytes": 82 * 1024**3, "nested_targets": [], "sibling_targets": []},
            "red", 1,
        ),
        (
            "告警线（50 GB > 40 GB 告警线，但 < 100 GB 红线 ⇒ 只告警）",
            {"exists": True, "target_bytes": 50 * 1024**3, "rcgu_o": 300_000,
             "incremental_bytes": 0, "nested_targets": [], "sibling_targets": []},
            "warn", 2,
        ),
        (
            "没有 target/（CI）⇒ 跳过，不误红",
            {"exists": False, "target_bytes": 0, "rcgu_o": 0,
             "incremental_bytes": 0, "nested_targets": [], "sibling_targets": []},
            "skip", 0,
        ),
        (
            "**同级第二套 target 很大**（= 实测 fuzz/target 形状）⇒ 只告警、**不判红**",
            {"exists": True, "target_bytes": 2 * 1024**3, "rcgu_o": 900,
             "incremental_bytes": 0, "nested_targets": [],
             "sibling_targets": [{"path": "fuzz/target", "bytes": 3 * 1024**3}]},
            "warn", 2,
        ),
        (
            "同级第二套 target **很小** ⇒ 闭嘴（不许每次 gate 刷噪声 ✗）",
            {"exists": True, "target_bytes": 2 * 1024**3, "rcgu_o": 900,
             "incremental_bytes": 0, "nested_targets": [],
             "sibling_targets": [{"path": "fuzz/target", "bytes": 164 * 1024**2}]},
            "ok", 0,
        ),
    ]
    print("① judge() 纯函数（合成数据）")
    for label, stats, want_v, want_c in cases:
        got = judge(stats)
        ok = got["verdict"] == want_v and got["exit_code"] == want_c
        print(
            f"  {'✓' if ok else '✗'} {label:<52} → {got['verdict']:<5}"
            f"(exit {got['exit_code']})  期望 {want_v} (exit {want_c})"
        )
        total += 1
        if not ok:
            bad += 1

    # **真扫一遍**：合成"有两套 target + rcgu.o 堆积"的目录树 ⇒ 必须判红 ✓
    print("② 真扫合成目录树")
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
        check(
            "真扫合成目录（30 个 .o + 第二套 target）",
            got["exit_code"] == EXIT_RED and st["rcgu_o"] == 30 and st["nested_targets"] == ["lang"],
            f"rcgu_o={st['rcgu_o']} nested={st['nested_targets']} exit={got['exit_code']}"
            "  期望 rcgu_o=30 nested=['lang'] exit=1",
        )
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # **硬链接去重**（2026-10-11 的 bug 就在这 ✗⇒✓）：6 条链接必须只算 1 份 ✓
    print("③ 硬链接去重（咬 2026-10-11 高报 60% 那个 bug）")
    tmp = Path(tempfile.mkdtemp(prefix="target-hygiene-hl-"))
    try:
        nlinks = 6
        measured, unique = _fixture_deps_bytes(tmp, nlinks)
        naive = unique * nlinks
        check(
            f"{nlinks} 条硬链接只算 1 份（旧版会读成 {human(naive)}）",
            measured == unique,
            f"读数 {human(measured)} · 唯一 {human(unique)} · 期望相等"
            f"（旧口径会是 {human(naive)} ⇒ 高报 {nlinks}× ✗）",
        )
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # **同级第二套 target 真能找出来** ✓
    print("④ 同级 target 检测（fuzz/target 形状）")
    tmp = Path(tempfile.mkdtemp(prefix="target-hygiene-sib-"))
    try:
        main = tmp / "target"
        (main / "debug" / "deps").mkdir(parents=True)
        (tmp / "fuzz" / "target" / "debug" / "deps").mkdir(parents=True)
        found = find_sibling_targets(tmp, main)
        check(
            "同级 fuzz/target 被找到、主 target 不被算进去",
            [str(p.relative_to(tmp)) for p in found] == ["fuzz/target"],
            f"found={[str(p.relative_to(tmp)) for p in found]}  期望 ['fuzz/target']",
        )
        st = scan(main, tmp)
        check(
            "scan() 把同级 target 记进 stats（供 judge 用）",
            [s["path"] for s in st["sibling_targets"]] == ["fuzz/target"],
            f"sibling_targets={[s['path'] for s in st['sibling_targets']]}",
        )
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # **回收计划**：只该是那两个目录 ✓（release/二进制/gate-cache 一个都不许进 ✗）
    print("⑤ 回收计划（只删两个目录）")
    plan = [str(p) for p in reclaim_plan(Path("/tmp/whatever/target"))]
    want = ["/tmp/whatever/target/debug/deps", "/tmp/whatever/target/debug/incremental"]
    check("reclaim_plan() 恰好两项且不含 release/二进制", plan == want, f"plan={plan}")

    # **拒绝回收时的输出必须截断** ✓（2026-10-11 实测：rustc 命令行几千字符 ⇒ 刷屏 ✗）
    print("⑥ 拒绝回收的提示行（rustc 命令行必须被截断）")
    long_line = "89769 /opt/homebrew/bin/sccache …/rustc --crate-name zerocopy " + "--check-cfg cfg(x) " * 400
    cut = format_builder_line(long_line)
    check(
        f"超长命令行被截到 ≤140 字符（原始 {len(long_line):,}）",
        len(cut) <= 140 and cut.endswith("…"),
        f"截后 {len(cut)} 字符",
    )
    check("短命令行原样保留", format_builder_line("123 cargo build") == "123 cargo build",
          f"got={format_builder_line('123 cargo build')!r}")

    print()
    if bad:
        print(f"✗ selftest 判红：{bad}/{total} 个用例不符 ✗")
        return 1
    print(f"✓ selftest 全过：{total}/{total} 个用例符合 ✓")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="target/ 体积守卫（rcgu.o 堆积 / 第二套 target）")
    ap.add_argument("--root", default=str(REPO_ROOT / "target"), help="target 目录（默认仓库 target/）")
    ap.add_argument("--repo", default=str(REPO_ROOT), help="仓库根（找同级 target 用；默认本仓）")
    ap.add_argument("--json", action="store_true", help="输出单个 JSON 对象")
    ap.add_argument("--reclaim", action="store_true", help="回收：定向删除 deps/ + incremental/")
    ap.add_argument("--selftest", action="store_true", help="离线判据通道自检")
    a = ap.parse_args()

    if a.selftest:
        return selftest()

    root = Path(a.root)
    repo = Path(a.repo)
    if a.reclaim:
        return reclaim(root, repo)

    stats = scan(root, repo)
    res = judge(stats)
    if a.json:
        print(json.dumps({"root": str(root), **stats, **res}, ensure_ascii=False, indent=2))
    else:
        render(stats, res, root)
    return res["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
