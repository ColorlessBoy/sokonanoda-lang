#!/usr/bin/env python3
"""compile-cache.py —— **编译缓存总管**（2026-10-12 用户：「rust 编译器对编译缓存文件的管理非常差，经常占了一百多 G
的存储，多 workspace 会放大这种问题，现在收口到我们这边来管理编译缓存文件」）。

## 它管什么（与 `target-hygiene.py` 的分工）

* `target-hygiene.py` = **单棵 target 的病态守卫**（`*.rcgu.o` 堆积 / 第二套 target / 40 G 报警 / 100 G 判红），
  已进 `gate` ✓ —— 它看的是"这一棵有没有坏"。
* 本脚本 = **跨工作树的总管**：4 核并行 ⇒ 4 棵 target（**必须独立**，共享会被 cargo 加锁串行 ✗），
  加起来会爆盘 ⇒ 这里做**登记 · 计量 · 预算 · 回收 · sccache 共享**。

## 核心策略（"收口到我们这边"= 一个受管根 + 一套预算 + 一个回收器）

1. **受管根**（`SOKO_COMPILE_CACHE_ROOT`，默认 **主工作树**的 `.cache/compile`，gitignore ✓）下面三样：
   `targets/<id>/`（每棵工作树一棵 target）· `sccache/`（**对象级共享缓存**，跨树复用、有容量上限）· `registry.json`。
   ⚠ **受管根是仓库级的、不是"当前 checkout 的"**（2026-10-15）：所有工作树都往
   `<主 checkout>/.cache/compile/targets/<id>` 里写（`env <id>` 就是这么生成的）⇒
   在链接工作树里也要量**同一个**根，否则守卫在 worktree 里看不见受管 target ✗。
2. **sccache 才是省盘正解**：4 棵树各自 `target/` 会各存一份**编译产物**（数十 G ✗），而 `RUSTC_WRAPPER=sccache`
   让**对象**跨树共享、按 LRU 封顶 ⇒ 既保住并行（不共享 target 目录 ⇒ 没有锁 ✗），又不重复占盘 ✓。
   本机已装（`/opt/homebrew/bin/sccache` ✓）；没装就**优雅降级**（照样能跑，只是不共享）。
3. **两个阈值（滞回）**：**判红线** = `SOKO_CACHE_BUDGET_GB`（默认 60；`registry.json` 里可被
   `--set-budget` 覆盖）—— 超了 `gate` 判红 ✗；**回收目标** = 判红线 − **余量**
   （`SOKO_CACHE_RESERVE_GB` / `registry.json` 的 `reserve_gb`，默认 **5% 预算**）。
   为什么要两个（2026-10-15 修 `CACHE-GUARD-NO-HEADROOM`）：旧 `--to-budget` **贴着判红线收**
   ⇒ 收完只差 0.1–0.3 G，而**跑一次 `gate --fast` 自己**就往受管 target 里写约 0.3 G
   ⇒ 下一轮 gate 立刻判红 ✗ —— **判据自己消耗被它判据的资源** ✗（实测 59.9 → 60.2）。
   ⇒ 判红线**不动**（"真超预算必须判红" ✓），只把**回收目标**压到线下，留出一次构建的余量 ✓。
4. **回收顺序**（保守 → 激进，先删最没用的 ✓）：
   ① **死工作树的整棵 target**（worktree 已不在 `git worktree list` 里）·
   ② 每棵的 `incremental/`（sccache 在时它的价值最低；本机实测 **23 G** ✗）·
   ③ 按**最后使用时间 LRU** 整棵删，直到落回**回收目标**内（⚠ **滞回**：**进门时越线**才动手，
   动手就一路收到**线下**留出余量 ✓；没越线 ⇒ 一步都不删，不误伤别人正在用的 target ✓）。
   有 cargo/rustc 在跑 ⇒ **拒绝回收**（删到一半的构建目录是坏的 ✗）。

## 用法

```bash
python3 scripts/compile-cache.py status              # 表格：每棵多大 · 总计 · 预算/回收目标 · sccache 命中
python3 scripts/compile-cache.py env <id>            # 打印该工作树要 export 的那几行（含 sccache）
python3 scripts/compile-cache.py reclaim             # 按上面 ①②③ 回收（打印"删了什么·拿回多少" ✓）
python3 scripts/compile-cache.py reclaim --to-budget # 只收到**回收目标**（判红线 − 余量）内
python3 scripts/compile-cache.py --json              # 机器读（进 gate 的报告段）
python3 scripts/compile-cache.py --selftest          # 判据通道自检（回收顺序 / 死树识别 / 预算 / 余量）
```

退出码：0 = 正常 · 1 = 超预算（判红）· 2 = 用法/环境错 ——
**含 GB 旋钮的值域错**（`SOKO_CACHE_BUDGET_GB` / `SOKO_CACHE_RESERVE_GB` / `set-budget <GB>`，
**rc=2 + 人话 + 正解**，**不丢栈** ✗⇒✓；唯一判定处 `_guard_gb_knobs()`）。
"""

from __future__ import annotations

import argparse
import json
import math
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

REPO = Path(__file__).resolve().parent.parent


def main_worktree() -> Path:
    """**主工作树**（`git worktree list --porcelain` 的**第一条** —— git 保证主工作树排在最前）。

    为什么需要它（2026-10-15 修 `CACHE-GUARD-NO-HEADROOM` 时实测）：受管根是**仓库级共享**的
    一个目录（`agent-worktree.sh` 与接手路径都用**主 checkout** 的
    `scripts/compile-cache.py env <id>` 生成 `CARGO_TARGET_DIR` ⇒ 所有工作树都往
    `<主 checkout>/.cache/compile/targets/<id>` 里写 ✓）。旧实现把受管根写成
    `<当前 checkout>/.cache/compile` ⇒ **在链接工作树里跑 gate** 时那个目录**不存在**
    ⇒ `managed_entries()` 一条受管 target 都枚举不到 ⇒ 守卫量到的是**另一套**（实测：
    同一时刻主 checkout 报 **59.7 GB**、worktree 报 **55.9 GB**，差的正是
    `.cache/compile/targets/*`）✗ —— 而并行 agent **一律在 worktree 里跑 gate** ⇒
    守卫在它最该生效的地方**没有牙** ✗。不是 git 仓（或 git 不可用）⇒ 退回 `REPO` ✓。
    """
    try:
        out = subprocess.run(["git", "worktree", "list", "--porcelain"], cwd=REPO,
                             capture_output=True, text=True, timeout=30,
                             env=git_env.clean_git_env()).stdout
    except (OSError, subprocess.SubprocessError):
        return REPO
    for line in out.splitlines():
        if line.startswith("worktree "):
            return Path(line.split(" ", 1)[1])
    return REPO


MAIN_WORKTREE = main_worktree()
# 受管根 = **仓库级**的那一个（主工作树的 `.cache/compile`），不是"当前 checkout 的" ✓；
# `SOKO_COMPILE_CACHE_ROOT` 仍是逃生门（显式给谁就用谁）✓。
CACHE_ROOT = Path(os.environ.get("SOKO_COMPILE_CACHE_ROOT") or MAIN_WORKTREE / ".cache" / "compile")
REGISTRY = CACHE_ROOT / "registry.json"
DEFAULT_BUDGET_GB = 60
# **回收余量**（`reclaim --to-budget` 的目标 = 预算 − 余量）。默认 5% 预算：60 G ⇒ 3 G，
# 而实测一次 `gate --fast` 往受管 target 里写约 **0.3 G** ⇒ 10× 余量 ✓（见 `reserve_bytes`）。
DEFAULT_RESERVE_FRAC = 0.05


def human(n: float) -> str:
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024 or unit == "TB":
            return f"{n:.1f} {unit}" if unit != "B" else f"{int(n)} B"
        n /= 1024
    return f"{n:.1f} TB"


def tree_bytes(path: Path) -> int:
    """表观大小，**硬链接按 inode 去重**（2026-10-12：本脚本第一版高报 75.5 G vs `du` 43 G ✗ ——
    `deps/` 里同一份 rlib 有 6 条硬链接，逐个 `st_size` 相加就是 6 倍；仓库 `target-hygiene.py`
    早就踩过并写了"6 条硬链接只算 1 份"的判据 ⇒ 这里照同一口径 ✓）。"""
    total = 0
    seen: set[tuple[int, int]] = set()
    for dirpath, dirnames, filenames in os.walk(path, onerror=lambda _e: None):
        for name in filenames:
            fp = os.path.join(dirpath, name)
            try:
                st = os.lstat(fp)
            except OSError:
                continue
            key = (st.st_dev, st.st_ino)
            if st.st_nlink > 1:
                if key in seen:
                    continue
                seen.add(key)
            total += st.st_blocks * 512   # 与 target-hygiene 同口径：实际占用 ✓
        dirnames[:] = [d for d in dirnames if d not in {".git", "node_modules"}]
    return total


def mtime(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def worktrees() -> list[dict]:
    """`git worktree list --porcelain` ⇒ [{path, branch}]（含主工作树 ✓）。"""
    out = subprocess.run(["git", "worktree", "list", "--porcelain"], cwd=REPO,
                         capture_output=True, text=True,
                         env=git_env.clean_git_env()).stdout.splitlines()
    trees, cur = [], {}
    for line in out:
        if line.startswith("worktree "):
            if cur:
                trees.append(cur)
            cur = {"path": Path(line.split(" ", 1)[1]), "branch": "(detached)"}
        elif line.startswith("branch ") and cur:
            cur["branch"] = line.split(" ", 1)[1].removeprefix("refs/heads/")
    if cur:
        trees.append(cur)
    return trees


def _is_worktree_alive(wt: Path) -> bool:
    """这棵工作树**还在不在**（`dead` = 可回收的唯一判据）。

    * 链接工作树（`git worktree add`）⇒ 根下是 **`.git` 文件**（`gitdir: …`）✓
    * 主工作树 ⇒ 根下是 **`.git` 目录** ✓
    * 两者都不是（目录被删剩空壳 / 手工 `rm -rf <wt>/*`）⇒ **不在了**，可回收 ✓

    ⚠ 只查 `.is_file()` 会把**主工作树**判死（它的 `.git` 是目录）——实测踩过一次：
    那一下把主 checkout 的 **26.9 GB** target 标成"可回收" ✗（`reclaim` 会删掉整个
    主 target ⇒ 灾难）。所以两个形状都要认 ✓。
    """
    marker = wt / ".git"
    return marker.is_file() or marker.is_dir()


def _hard_safety_check(entries: dict[str, dict]) -> None:
    """**主工作树的 target 永远不许被判可回收**（删它 = 删掉整仓构建产物 ✗）。

    这是"判据写错一次就够致命"的那类地方 ⇒ 不只靠上面的函数写对，**回收前再断言一次** ✓。
    ⚠ 判据用 `MAIN_WORKTREE`（不是 `REPO`）：从链接工作树里跑时 `REPO` **不是**主工作树，
    拿它去断言等于没断言 ✗（2026-10-15 随受管根一起收口 ✓）。
    """
    main_target = (MAIN_WORKTREE / "target").resolve()
    for e in entries.values():
        if Path(e["path"]).resolve() == main_target and e.get("dead"):
            raise SystemExit(
                f"✗ 内部不变量被破坏：主工作树的 `target/` 被判成**可回收**（{main_target}）"
                f" ⇒ 拒绝回收（判据写错了，不是磁盘问题）✗")


def managed_entries() -> dict[str, dict]:
    """`{绝对路径: 条目}`（key 就是**去重键**：同一棵 target 只出现一次）。

    ⚠ 返回的是 **dict 不是 list**（注解历史上写错了 ⇒ 调用方 `for e in managed_entries()`
    拿到的是**字符串键**、`e["path"]` 直接 `TypeError`）。2026-10-12 修 `SUB-reclaim-never-fires`
    时踩到：`cmd_reclaim` 的 ① 档本来就会崩在这上面（而它此前从未被跑到 ⇒ 崩了也没人知道）。
    现在每个调用点都显式取 `.values()`，并加一条形状断言（见下）。
    """
    """所有受管 target：主工作树的 `target/` + 每棵工作树的 target + 受管根下的 `targets/*` ✓。"""
    entries: dict[str, dict] = {}

    def add(path: Path, owner: str, kind: str, dead: bool | None = None) -> None:
        """`dead` = "**可回收**"（默认：路径不存在）。

        ⚠ 工作树那一路要传**自己的判据**：`git worktree list` 里**残留的注册**会让
        `wt["path"]` 照样出现在列表里（目录没了、或只剩个空壳），此时
        `not path.exists()` 是 `False` ⇒ 那棵还占着盘的 `target/` 被当成活的、**永远回收不了** ✗
        （`SUB-reclaim-never-fires` 的真实机制）。判据 = "**它还是不是一棵工作树**"
        （`<wt>/.git` 标记文件在不在）✓。
        """
        key = str(path.resolve()) if path.exists() else str(path)
        if key in entries:
            if dead is not None and dead:
                entries[key]["dead"] = True              # 后来者可以把它标死（反之不改 ✓）
            return
        entries[key] = {"path": path, "owner": owner, "kind": kind,
                        "bytes": tree_bytes(path) if path.is_dir() else 0,
                        "mtime": mtime(path),
                        "dead": (not path.exists()) if dead is None else dead}

    for wt in worktrees():
        t = wt["path"] / "target"
        add(t, wt["branch"], "worktree", dead=not _is_worktree_alive(wt["path"]))
    if CACHE_ROOT.is_dir():
        for child in sorted((CACHE_ROOT / "targets").glob("*")) if (CACHE_ROOT / "targets").is_dir() else []:
            add(child, child.name, "managed")
    # 老式同级 target-<branch>/（仓库历史上用过）
    for sib in sorted(REPO.parent.glob(f"{REPO.name}/target-*")) + sorted(REPO.glob("target-*")):
        add(sib, sib.name, "sibling")
    # **孤儿 target**（2026-10-12 接口 agent 提交 `SUB-reclaim-never-fires` 的真形态）：
    # `git worktree remove` / 手工 `rm -rf <wt>` 都**不会**动 `<wt>/target` ⇒ 那棵 target
    # 从此**再也没人枚举**（不是"被跳过"，是**看不见**）⇒ 永远占盘、`reclaim` 第①档永不触发 ✗。
    # 两处扫描（都**只认确实的工作树形状**，别扫无关目录 —— 实测踩过一次：
    # `REPO.parent.glob("*/target")` 会把**另一个仓库** `/…/sokonanoda/sokonanoda/target`
    # 当成孤儿 ✗）：
    #   ① `<REPO>/.worktrees/<id>/target`（我们自己的约定；目录里必须有 `.git` **文件**）
    #   ② **git 注册过、但目录已消失**的工作树（读 `.git/worktrees/*/gitdir`）—— `git worktree
    #      prune` 认得的那一类；它的 `target/` 若还在，就是货真价实的孤儿 ✓。
    for cand in sorted((MAIN_WORKTREE / ".worktrees").glob("*/target")):
        if not cand.is_dir() or str(cand.resolve()) in entries:
            continue
        if not (cand.parent / ".git").is_file():
            continue                                     # 不是链接工作树 ⇒ 不碰 ✓
        add(cand, f"（孤儿：{cand.parent.name}）", "orphan-worktree")
        entries[str(cand.resolve())]["dead"] = True       # 主人已不在工作树列表 ⇒ 可回收 ✓
    for meta in sorted((MAIN_WORKTREE / ".git" / "worktrees").glob("*/gitdir")):
        try:
            wt_path = Path(meta.read_text(encoding="utf-8").strip())
        except OSError:
            continue
        wt = wt_path.parent if wt_path.name == ".git" else wt_path
        target = wt / "target"
        # "主人还在吗"的判据**不能只看 `wt.exists()`**：手工 `rm -rf <wt>/*` / 删到一半时
        # 那个目录**还在**（于是 target 也还在、还占着盘）⇒ 只要**不再是工作树**
        # （`.git` 标记文件没了）就算孤儿 ✓。实测踩过这一次（`find -delete` 留下的空壳）。
        still_worktree = (wt / ".git").is_file()
        if still_worktree or not target.is_dir() or str(target.resolve()) in entries:
            continue
        add(target, f"（工作树已消失：{wt.name}）", "orphan-worktree")
        entries[str(target.resolve())]["dead"] = True
    _hard_safety_check(entries)
    return entries


def load_registry() -> dict:
    if REGISTRY.exists():
        try:
            return json.loads(REGISTRY.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            pass
    return {}


def save_registry(data: dict) -> None:
    CACHE_ROOT.mkdir(parents=True, exist_ok=True)
    REGISTRY.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


# ── **GB 旋钮的值域闸门**（洞 `COMPILE-CACHE-GB-ENVS-NO-GUARD`，2026-10-16）──────────────
# 旧写法是**裸 `int(float(env) * 1024**3)`**（`budget_bytes` / `reserve_bytes`）⇒ **两向都错** ✗：
#   · 非数字 / 非有限（`abc` · `nan` · `inf`）⇒ `ValueError` / `OverflowError` + **Traceback**
#     ⇒ **崩溃而不是判定** ✗（契约是"环境错 = rc=2 + 人话"，见本文件抬头）；
#   · `SOKO_CACHE_RESERVE_GB=-5` ⇒ `max(0, …)` **静默吞掉**负号 ⇒ 余量 **0 G**
#     ⇒ `reclaim_target` = **判红线本身** ⇒ **正是 2026-10-15 刚修的 `CACHE-GUARD-NO-HEADROOM`**
#     （贴着线收 ⇒ 下一次 `gate --fast` 自己就把它顶过线）✗ —— 而**没有任何读数会红** ✗。
# 修法照本仓已有的「环境变量值域闸门」范式（`gap.py` 的 `_parse_repro_timeout_env()` +
# `_guard_repro_timeout_env()`；`ux-queue.py` 的 `_parse_slots_env()` 同族 ✓）：
#   ① `_parse_gb_env()` —— **唯一读取+解析处**、**永不抛**（导入期/判定期都要活到 `main()`）；
#   ② `_guard_gb_knobs()` —— 在 `main()` 里判 **rc=2 + 人话 + 正解** ✓。
# ⚠ **两个纯函数必须继续实时读 `os.environ`** ✗⇒✓：`--selftest` 会在**运行中途**设/删这两个
#   变量（`SOKO_CACHE_BUDGET_GB=0.000000001` 那几条既有判据）⇒ **冻结成模块常量会打死它们** ✗
#   （解析放在函数内、闸门复用**同一个** `_parse_gb_env()` ⇒ 一处解析、一处判定，不会漂 ✓）。
# ⚠ 值域（**不许放宽，也不许收紧** ✗）：
#   · `SOKO_CACHE_BUDGET_GB` = **判红线** ⇒ **正数**（≤0 ⇒ 每一棵树都"超预算" ⇒ 判红线**永远红**
#     = 噪声；它没有任何"关掉预算"的文档语义 —— 同 `SOKO_QUEUE_SLOTS` 的推理 ✓）；
#   · `SOKO_CACHE_RESERVE_GB` = **回收余量** ⇒ **正数**（`0` 就是 **`CACHE-GUARD-NO-HEADROOM`
#     那个状态本身**：`reclaim_target` = 判红线 ⇒ 下一次 `gate --fast` 自己就把它顶过线；
#     `reserve` 的存在理由就是"留出一次构建的余量"（见本文件 §3）⇒ 它不是可选项 ✓；
#     ⚠ 旧代码的 `max(0, …)` 把**负数**静默变成 `0` —— 两向都落进那个状态 ⇒ 值域 = 正数）。
#   · 合法值**照旧走 `float()`** ⇒ 以前能用的写法（`200` · `60` · `0.5` · `" 200"` · `"+200"`）
#     **逐字节不变** ✓（"不许放宽"的另一面：也不许收紧到把旧写法判红 ✗）。
GB_BUDGET_ENV = "SOKO_CACHE_BUDGET_GB"
GB_RESERVE_ENV = "SOKO_CACHE_RESERVE_GB"
# 两个旋钮的**唯一描述表**（闸门与消费点共用 ⇒ 人话不会两处漂 ✗）
GB_ENV_KNOBS: dict[str, dict] = {
    GB_BUDGET_ENV: {
        "what": "**判红线**",
        "why": "预算 ≤0 会让**每一棵树**都「超预算」⇒ 判红线**永远红**（那不是判据，是噪声），"
               "而它没有任何「关掉预算」的文档语义",
        "fix": f"`unset {GB_BUDGET_ENV}` 回到默认 {DEFAULT_BUDGET_GB} G，或 `{GB_BUDGET_ENV}=60`",
    },
    GB_RESERVE_ENV: {
        "what": "**回收余量**",
        "why": "余量 **0 G** ⇒ `reclaim_target` = **判红线本身** ⇒ **`CACHE-GUARD-NO-HEADROOM` "
               "复发**（贴着线收 ⇒ 下一次 `gate --fast` 自己就把它顶过线）—— 旧写法 "
               "`max(0, …)` 把**负数静默变成 `0`** ⇒ 两向都落进这个状态 ✗",
        "fix": f"`unset {GB_RESERVE_ENV}` 回到默认（5% 预算），或 `{GB_RESERVE_ENV}=2`",
    },
}


def _parse_gb_value(name: str, raw: object, *, what: str,
                    why: str, fix: str) -> tuple[float | None, str]:
    """`(GB, 人话)`：`人话 == ""` ⇒ 合法；`GB is None` ⇒ 这个值**用不了**（调用方走自己的默认链）。

    **永不抛**（模块导入与 `main()` 的闸门都要活到能判红；`raw` 也可能是 registry 里的任意
    JSON 值 ⇒ `TypeError` 也要接住 ✓）。合法值**照旧 `float()`** ⇒ 旧写法逐字节不变 ✓。
    **值域 = 正数**（两个旋钮都是"越大越保守"的阈值，`0`/负数都没有文档语义 ✓）。
    """
    if raw is None or (isinstance(raw, str) and not raw.strip()):
        return None, ""                       # 不设 / 空串 / 纯空白 ⇒ 与"没设"同口径 ✓
    try:
        gb = float(raw)                       # type: ignore[arg-type]
    except (TypeError, ValueError):
        return None, (f"`{name}` **不是数字**（原值 `{raw!r}`）⇒ 它要的是{what}的 **GB 数**"
                      f"（正数）✗。正解：{fix}")
    if not math.isfinite(gb):
        return None, (f"`{name}` 必须是**有限**的 GB 数（原值 `{raw!r}` = {gb}）⇒ "
                      f"`nan` / `inf` 不是 GB 数（`int(nan * 1024**3)` 会丢栈 ✗）。正解：{fix}")
    if gb <= 0:
        return None, (f"`{name}` 必须是**正数**（原值 `{raw!r}` = {gb}）⇒ {why}。正解：{fix}")
    return gb, ""


def _parse_gb_env(name: str) -> tuple[float | None, str]:
    """环境变量 `name` 的**唯一读取+解析点**（消费点与闸门共用 ⇒ 一处解析、一处判定 ✓）。"""
    return _parse_gb_value(name, os.environ.get(name), **GB_ENV_KNOBS[name])


def _guard_gb_knobs() -> int | None:
    """两个 GB 旋钮**取值域错** ⇒ rc=2 + 人话 + 正解（**别丢栈** ✗）。返回 `None` = 合法。

    ⚠ 位置在 `argparse` **之前**（同 `gap.py` 的 `_guard_repro_timeout_env()`）：值域错是
    "**这台机器的环境坏了**" ⇒ 连 `--help` / `env` / `--selftest` 都不该假装正常 ✓。
    （⚠ 如实：`ux-queue.py` 那一族闸门在 `parse_args` **之后**跑 —— 两种位置本仓都有先例；
    这里选"之前"，因为这两个值**每个子命令**都会消费：`env` 读预算、`status`/`reclaim` 读两者 ✓。）
    """
    problems = [why for why in (_parse_gb_env(n)[1] for n in GB_ENV_KNOBS) if why]
    if not problems:
        return None
    for p in problems:
        print(f"error: {p}", file=sys.stderr)
    return 2


def budget_bytes(reg: dict | None = None) -> int:
    """预算字节数。⚠ `reg` 的**空字典**是合法入参（= "没有 registry"）⇒ 走默认值。

    2026-10-12 实测的 bug（接口 agent 提交 `SUB-compile-cache-selftest-false-red`）：
    旧写法 `reg = reg or load_registry()` 把**空 dict**（falsy）当成"没传" ⇒ **回落去读真
    registry.json**，而 `--set-budget` 正是写那个文件 ⇒ 用户按文档设过一次预算之后，
    `--selftest` 的 `budget_bytes({}) == DEFAULT_BUDGET_GB` **永久判红** —— 而这条 selftest
    已进 `scripts/soko gate` 与 CI（任何设过预算的机器 gate 假红 ✗）。
    判据用 `is None`（只有真的没传才回落）✓。

    ⚠ **值域**（2026-10-16 · 洞 `COMPILE-CACHE-GB-ENVS-NO-GUARD` ✗⇒✓）：环境变量这条走
    `_parse_gb_env()` ⇒ 非数字 / `nan` / `inf` / ≤0 **不再丢栈**（旧写法 `float(env)` 直接抛 ✗），
    而是**用不了** ⇒ 落回 registry/默认；**真跑 CLI 时 `_guard_gb_knobs()` 先判 rc=2 + 人话** ✓
    （这里落回默认只服务"导入期/被绕过时不炸" —— 判据在闸门，不在这里 ✓）。
    """
    gb, _why = _parse_gb_env(GB_BUDGET_ENV)
    if gb is not None:
        return int(gb * 1024**3)
    if reg is None:
        reg = load_registry()
    return int(float(reg.get("budget_gb", DEFAULT_BUDGET_GB)) * 1024**3)


def reserve_bytes(reg: dict | None = None) -> int:
    """**回收余量**：`reclaim --to-budget` 的目标 = `budget_bytes()` − 本函数。

    2026-10-15 修 `CACHE-GUARD-NO-HEADROOM`（第十四次复核实测）：旧 `--to-budget` 判的是
    `after <= budget` ⇒ **贴着判红线收**，收完只剩 **0.1–0.3 G**；而**跑一次被声称
    "应 exit 0"的 `gate --fast` 自己**就往受管 target 里写（clippy/单测）⇒ 当场顶过线
    （实测 59.9 → 60.2 ⇒ 守卫 rc=1）✗ —— **判据自己消耗被它判据的资源** ✗。
    ⇒ 判红线**不动**（"真超预算必须判红" ✓），给**回收**一个真正的余量目标 ✓。

    余量取值：`SOKO_CACHE_RESERVE_GB` → `registry.json` 的 `reserve_gb` →
    默认 **5% 预算**（60 G ⇒ 3 G；实测一次 `gate --fast` 的增量约 **0.3 G** ⇒ 10× 余量 ✓）。
    ⚠ 与 `budget_bytes` 同口径：**空 dict ≠ 没传**（`{}` = "没有 registry" ⇒ 走默认 ✓）。
    ⚠ **值域**（2026-10-16 · 洞 `COMPILE-CACHE-GB-ENVS-NO-GUARD` ✗⇒✓）：环境变量这条走
    `_parse_gb_env()` ⇒ **负数不再被 `max(0, …)` 静默吞掉**（那会让余量 0 G ⇒ `reclaim_target`
    = 判红线本身 ⇒ `CACHE-GUARD-NO-HEADROOM` 复发 ✗）；非法值落回默认链、**闸门判 rc=2 + 人话** ✓。
    """
    gb, _why = _parse_gb_env(GB_RESERVE_ENV)
    if gb is not None:
        return max(0, int(gb * 1024**3))
    if reg is None:
        reg = load_registry()
    if "reserve_gb" in reg:
        return max(0, int(float(reg["reserve_gb"]) * 1024**3))
    return max(0, int(budget_bytes(reg) * DEFAULT_RESERVE_FRAC))


def reclaim_target_bytes(reg: dict | None = None) -> int:
    """`reclaim --to-budget` 的**目标**（= 判红线 − 余量）✓ —— 与判红线分开的两个阈值（滞回）。"""
    if reg is None:
        reg = load_registry()
    return budget_bytes(reg) - reserve_bytes(reg)


def sccache_dir() -> Path:
    return Path(os.environ.get("SCCACHE_DIR") or CACHE_ROOT / "sccache")


def builders_running() -> list[str]:
    """**不靠 `ps`**（沙箱常挡 ✗）：直接试 cargo 自己的**构建锁** `<target>/.cargo-lock`
    （cargo 构建期间持 `flock`；试 `LOCK_EX|LOCK_NB` 失败 ⇒ 正有构建在跑 ✓）。
    拿不到锁文件（不存在/无权限）⇒ 记成"这一棵看不出来"，不据此拒绝。"""
    import fcntl
    busy: list[str] = []
    for e in managed_entries().values():
        lock = e["path"] / ".cargo-lock"
        if not lock.exists():
            lock = e["path"] / "debug" / ".cargo-lock"
        if not lock.exists():
            continue
        try:
            with lock.open("a+") as fh:
                try:
                    fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                    fcntl.flock(fh.fileno(), fcntl.LOCK_UN)     # 没被占 ⇒ 立刻放开 ✓
                except OSError:
                    busy.append(f"{lock} 被持有（有 cargo 构建在跑）")
        except OSError:
            continue
    if not busy:
        try:
            out = subprocess.run(["ps", "-Ao", "command"], capture_output=True, text=True).stdout
            busy += [l.strip()[:120] for l in out.splitlines()
                     if ("cargo" in l or "rustc" in l) and "compile-cache.py" not in l
                     and "grep" not in l]
        except (OSError, PermissionError):
            pass                                            # 沙箱挡了 ps ⇒ 只信锁 ✓
    return busy


def reclaim_order(entries: list[dict]) -> list[dict]:
    """**回收顺序**（纯函数，便于 selftest 钉死）：死树 → 其余按 LRU 最旧优先 ✓。"""
    dead = [e for e in entries if e["dead"]]
    live = sorted((e for e in entries if not e["dead"]), key=lambda e: e["mtime"])
    return dead + live


def cmd_status(as_json: bool, quiet: bool) -> int:
    entries = list(managed_entries().values())
    total = sum(e["bytes"] for e in entries)
    budget = budget_bytes()
    target = reclaim_target_bytes()
    over = total > budget
    if as_json:
        print(json.dumps({"total_bytes": total, "budget_bytes": budget,
                          "reserve_bytes": budget - target, "reclaim_target_bytes": target,
                          "over": over,
                          "entries": [{**e, "path": str(e["path"])} for e in entries],
                          "sccache_dir": str(sccache_dir())}, ensure_ascii=False, indent=2))
        return 1 if over else 0
    if quiet:
        print(f"编译缓存：{human(total)} / 预算 {human(budget)}"
              + (f"（回收目标 {human(target)}）" if target < budget else "")
              + ("（**超了** ⇒ `scripts/compile-cache.py reclaim` ✗）" if over else " ✓"))
        return 1 if over else 0
    print(f"{'路径':<62}{'谁':<18}{'类型':<10}{'大小':>10}  最后使用")
    for e in entries:
        age = f"{(time.time() - e['mtime']) / 3600:.1f}h 前" if e["mtime"] else "?"
        tag = " **死树**" if e["dead"] else ""
        print(f"{str(e['path'])[-60:]:<62}{e['owner'][:16]:<18}{e['kind']:<10}"
              f"{human(e['bytes']):>10}  {age}{tag}")
    print(f"\n合计 **{human(total)}** / 预算 {human(budget)}"
          + (f"（`reclaim --to-budget` 的目标 **{human(target)}**，留 {human(budget - target)} 余量 ✓）"
             if target < budget else "")
          + ("  ⇒ **超预算**：`reclaim` ✗" if over else "  ✓"))
    sc = sccache_dir()
    if sc.is_dir():
        print(f"sccache：{sc} = {human(tree_bytes(sc))}")
    if not shutil.which("sccache"):
        print("⚠ 没装 sccache ⇒ 4 棵树会各存一份产物（`brew install sccache` 能省数十 G ✓）")
    return 1 if over else 0


def cmd_env(ident: str) -> int:
    """打印某棵工作树要 export 的受管环境（**独立 target + 共享 sccache + 队列单点** ✓）。

    ⚠ **`SOKO_QUEUE_ROOT` 必须是主工作树，不是"当前 checkout"** ✗
    （洞 `COMPILE-CACHE-EXPORTS-QUEUE-ROOT`，2026-10-15）：队列状态是**主 checkout 单点**
    （`scripts/agent-worktree.sh:25` 的契约：「事件/台账只在主 checkout 上写 ✓」），
    而这里曾导出 `REPO` = **跑本脚本的那棵工作树** ⇒ 在链接工作树里 `eval "$(compile-cache.py env …)"`
    之后，`ux-queue.py` 会去读**工作树那份开树时的副本**（`ux-queue.py` 只在**没设**该变量时才拒绝，
    设了错值它照读 ✗）⇒ **守卫静默审错对象**：不报错、只是量的不是同一本账
    （实测：往工作树副本塞一条合成事件 ⇒ 同一棵树、同一条命令，沿用导出值报
    「✗ 幽灵条目」rc=1，而指回主 checkout rc=0 ✓）。
    ⇒ 与 `scripts/ci-local.sh` 的 `ledger` 步**同一件仪器**：`git worktree list` **第一行**
    （`main_worktree()`，git 保证主工作树排在最前 ✓）；量不到（不是 git 仓）就退回 `REPO` ✓。
    """
    if not ident:
        raise SystemExit("用法：compile-cache.py env <id>")
    target = CACHE_ROOT / "targets" / ident
    lines = [
        f'export CARGO_TARGET_DIR="{target}"',
        f'export SCCACHE_DIR="{sccache_dir()}"',
        f'export SCCACHE_CACHE_SIZE="{int(budget_bytes() / 1024**3)}G"',
        'export RUSTC_WRAPPER="$(command -v sccache || true)"   # 没装就留空 ⇒ 优雅降级 ✓',
        'export CARGO_INCREMENTAL=0                            # incremental 是本机最大的占用（23 G ✗）',
        # `SOKONANODA_CACHE_DIR` 与 `CARGO_TARGET_DIR` **相反**：编译产物缓存**按工作树独立**
        # （`agent-worktree.sh:26`「独立缓存（与 G-112 同族的坑）」）⇒ 这里要的就是 `REPO` ✓。
        f'export SOKONANODA_CACHE_DIR="{(REPO / ".cache" / "soko" / ident)}"',
        f'export SOKO_QUEUE_ROOT="{MAIN_WORKTREE}"',
    ]
    print("\n".join(lines))
    return 0


def cmd_reclaim(to_budget: bool) -> int:
    running = builders_running()
    if running:
        print("✗ 有 cargo/rustc 在跑 ⇒ 拒绝回收（删到一半的构建目录是坏的 ✗）：")
        for line in running[:5]:
            print(f"    {line}")
        return 2
    entries = list(managed_entries().values())
    budget = budget_bytes()
    # **目标**（不是判红线）：`--to-budget` 收到**目标**为止 ⇒ 留出一次构建的余量 ✓
    # （旧实现用 `budget` 当目标 ⇒ 贴着线收 ⇒ 下一次 `gate --fast` 立刻又超 ✗，见 `reserve_bytes`）。
    target = reclaim_target_bytes() if to_budget else budget
    before = sum(e["bytes"] for e in entries)
    freed, acted = 0, []
    # ① 死工作树的整棵 target
    for e in [x for x in entries if x["dead"]]:
        if e["path"].is_dir():
            freed += tree_bytes(e["path"])
            shutil.rmtree(e["path"], ignore_errors=True)
            acted.append(f"死树整棵 {e['path']}")
    # ② 每棵的 incremental/
    for e in entries:
        inc = e["path"] / "debug" / "incremental"
        if inc.is_dir():
            freed += tree_bytes(inc)
            shutil.rmtree(inc, ignore_errors=True)
            acted.append(f"incremental {e['path']}")
    # ③ **进门时已超判红线** ⇒ 按 LRU 整棵删，一直删到**回收目标**（判红线 − 余量）✓
    # ⚠ **触发条件 = "进门时已越线"**（`before > budget`），不是"② 之后仍越线" ✗：
    #   复核者实测的那一次正是反例 —— 60.1 越线、第②档（`incremental/`）释放 586 M
    #   ⇒ 落到 59.7 **刚好在线下** ⇒ 旧口径**第③档不跑** ⇒ 余量只有 0.3 G ✗
    #   （下一次 `gate --fast` 立刻又超 ✓）。这就是**滞回**该有的样子：
    #   **越线才动手、动手就一路收到线下** ✓。
    # ⚠ 也别把触发条件改成"超目标" ✗：那会让 `reclaim --to-budget` 在**没越线**时
    #   也去删别人**正在用**的受管 target（实测：本机 59.7/60 明明没超，却会删掉
    #   2.7 G 可复用产物 ⇒ 下一次构建冷编 ✗）—— 那不是"回收"，是"误伤" ✗。
    after = before - freed
    triggered = to_budget and before > budget
    if triggered:
        for e in reclaim_order([x for x in entries if not x["dead"] and x["kind"] == "managed"]):
            if after <= target:
                break
            if not e["path"].is_dir():
                continue
            size = tree_bytes(e["path"])
            shutil.rmtree(e["path"], ignore_errors=True)
            freed += size
            after -= size
            acted.append(f"LRU 整棵 {e['path']}")
    gone = sum(e["bytes"] for e in managed_entries().values())
    print(f"回收前：{human(before)} ⇒ 回收后：**{human(gone)}**（释放 **{human(freed)}** · "
          f"预算 {human(budget)}" + (f" · 回收目标 {human(target)}" if target != budget else "") + "）")
    for a in acted[:12]:
        print(f"  · 删了 {a}")
    if not acted:
        print("  （没有可回收的：① 无死树 ② 无 incremental ③ 未超预算 ✓）")
    if triggered and gone > target:
        # **如实说"没收到目标"**（别让一行"回收后 X"读起来像成功 ✗）：可回收的只有
        # ①②③ 三类，而**别的活工作树的 `target/`** 与**主工作树的 `target/`** 按设计不动 ✓。
        print(f"  ⚠ **没收到回收目标**：还差 {human(gone - target)} —— 可回收的只有 "
              f"死树 / `incremental/` / 受管 `targets/*`；"
              f"其余（主工作树与**活**工作树的 `target/`）按设计不动 ✗\n"
              f"     ⇒ 要么 `scripts/agent-worktree.sh rm <已完工的 id>` 清掉不用的工作树，"
              f"要么把预算调**紧**（不许靠抬预算达标 ✗）")
    print("  ⚠ 下一次构建会重编一次（慢一次，属预期 ✓）；配 sccache 时这次重编也能跨树复用 ✓")
    return 0


def selftest() -> int:
    """判据通道自检：**回收顺序** · **预算判定** · **GB 旋钮的值域**必须咬得住。

    ⚠ 2026-10-16 起**不再是"纯函数级"**：GB 旋钮那几条会**真起子进程跑真 CLI**
    （`env probe` ⇒ **零写入、不扫盘** ✓）—— 因为"我改的那个变量"不是判据，真路径才是 ✓。
    除 `tempfile` 与那几个只读子进程外不碰真盘 ✓（实测 `env -i` 下仍 rc=0 ✓）。
    """
    bad = 0
    checks = 0

    def expect(desc: str, cond: bool) -> None:
        nonlocal bad, checks
        checks += 1
        if not cond:
            bad += 1
            print(f"FAIL {desc}", file=sys.stderr)

    now = time.time()
    entries = [
        {"path": Path("/m/live-old"), "dead": False, "mtime": now - 7200, "bytes": 10, "kind": "managed"},
        {"path": Path("/m/live-new"), "dead": False, "mtime": now - 60, "bytes": 10, "kind": "managed"},
        {"path": Path("/m/dead"), "dead": True, "mtime": now - 10, "bytes": 10, "kind": "managed"},
    ]
    order = [str(e["path"]) for e in reclaim_order(entries)]
    expect(f"死树排最前（{order}）", order[0] == "/m/dead")
    expect(f"活树按 LRU 最旧优先（{order}）", order[1:] == ["/m/live-old", "/m/live-new"])

    # 反向验证：把预算设成 1 字节 ⇒ 必须判"超预算"
    os.environ["SOKO_CACHE_BUDGET_GB"] = "0.000000001"
    expect("预算 1B ⇒ 判超（反向验证）", budget_bytes({"budget_gb": 0}) <= 2)
    del os.environ["SOKO_CACHE_BUDGET_GB"]
    expect("registry 里的预算生效", budget_bytes({"budget_gb": 7}) == 7 * 1024**3)
    # ⚠ **空 dict ≠ 没传**：`{}` 是"没有 registry"⇒ 必须走默认值；用 `is None` 判（见
    # `budget_bytes` 的说明）。这里同时钉住"真 registry 会不会把它带偏"：
    # 把 `load_registry` 换成一个**非默认预算**的假 registry ⇒ 空 dict 仍须给默认值。
    saved_load = globals()["load_registry"]
    globals()["load_registry"] = lambda: {"budget_gb": 7}
    try:
        expect("空 dict ⇒ 默认 60G（**不许**回落去读真 registry ✗）",
               budget_bytes({}) == DEFAULT_BUDGET_GB * 1024**3)
        expect("没传（None）⇒ 才回落 registry", budget_bytes() == 7 * 1024**3)
    finally:
        globals()["load_registry"] = saved_load

    # ── **回收余量**（2026-10-15 修 `CACHE-GUARD-NO-HEADROOM`）──────────────────
    # 病根是"`--to-budget` 贴着判红线收 ⇒ 下一次 `gate --fast` 自己就把它顶过线" ✗
    # ⇒ 这三条钉住"**判红线不动、回收目标在线下**"这个滞回 ✓。
    os.environ["SOKO_CACHE_RESERVE_GB"] = "2"
    expect("`SOKO_CACHE_RESERVE_GB` 生效", reserve_bytes({"budget_gb": 60}) == 2 * 1024**3)
    expect("回收目标 = 判红线 − 余量（**不是**判红线 ✗）",
           reclaim_target_bytes({"budget_gb": 60}) == 58 * 1024**3)
    del os.environ["SOKO_CACHE_RESERVE_GB"]
    expect("余量默认 = 5% 预算", reserve_bytes({"budget_gb": 60}) == 3 * 1024**3)
    expect("**判红线本身不许被余量改口径**（超预算仍按 `budget` 判 ✗⇒✓）",
           budget_bytes({"budget_gb": 60}) == 60 * 1024**3)
    expect("registry 的 `reserve_gb` 生效", reserve_bytes({"budget_gb": 60, "reserve_gb": 4}) == 4 * 1024**3)
    expect("空 dict ⇒ 默认余量（同 `budget_bytes` 口径：**空 dict ≠ 没传** ✓）",
           reserve_bytes({}) == int(DEFAULT_BUDGET_GB * DEFAULT_RESERVE_FRAC) * 1024**3)

    # ── **GB 旋钮的值域闸门**（2026-10-16 · 洞 `COMPILE-CACHE-GB-ENVS-NO-GUARD`）──────────
    # 旧写法是裸 `int(float(env) * 1024**3)` ⇒ `abc` 丢栈（rc=1）、`SOKO_CACHE_RESERVE_GB=-5`
    # 被 `max(0, …)` **静默**吞掉 ⇒ 余量 **0 G** ⇒ 判红线自身成了回收目标 ✗。**两向都钉**：
    #   · 错值 ⇒ `人话` 必须非空、**点名变量**、**说清后果**、**给正解**，且**不抛**（二元组 ✓）；
    #   · 合法值 ⇒ `人话` 必须为空且**值逐字节不变**（`" 200"` / `"+200"` / `0.000000001`
    #     这些旧写法不许被收紧 ✗）。
    gb_env_cases = [
        # (说明, name, raw（None = 不设）, 期望 GB（None = 用不了）, 人话里必须出现的关键字)
        ("不设 ⇒ 无值无话", GB_BUDGET_ENV, None, None, ""),
        ("空串 ⇒ 无值无话", GB_BUDGET_ENV, "", None, ""),
        ("纯空白 ⇒ 无值无话（旧：`float('  ')` 丢栈 ✗）", GB_BUDGET_ENV, "   ", None, ""),
        ("**`abc` ⇒ 人话**（旧：栈 + rc=1 ✗）", GB_BUDGET_ENV, "abc", None, "不是数字"),
        ("**`nan` ⇒ 人话**（旧：`int(nan)` 丢栈 ✗）", GB_BUDGET_ENV, "nan", None, "有限"),
        ("**`inf` ⇒ 人话**（旧：`int(inf)` 丢栈 ✗）", GB_BUDGET_ENV, "inf", None, "有限"),
        ("**`-5` ⇒ 人话**（旧：预算负数 ⇒ 判红线永远红 ✗）", GB_BUDGET_ENV, "-5", None, "正数"),
        ("**`0` ⇒ 人话**（旧：预算 0 ⇒ 判红线永远红 ✗）", GB_BUDGET_ENV, "0", None, "正数"),
        ("`200`（**CI 口径**）⇒ 照旧 ✓", GB_BUDGET_ENV, "200", 200.0, ""),
        ("`60` ⇒ 照旧 ✓", GB_BUDGET_ENV, "60", 60.0, ""),
        ("`0.5` ⇒ 照旧 ✓", GB_BUDGET_ENV, "0.5", 0.5, ""),
        ("**`\" 200\"`（前后空格）⇒ 照旧**（不许收紧 ✗）", GB_BUDGET_ENV, " 200", 200.0, ""),
        ("**`\"+200\"` ⇒ 照旧**（不许收紧 ✗）", GB_BUDGET_ENV, "+200", 200.0, ""),
        ("`0.000000001` ⇒ 照旧（**既有判据在用** ✓）", GB_BUDGET_ENV, "0.000000001", 1e-9, ""),
        ("不设 ⇒ 无值无话（余量）", GB_RESERVE_ENV, None, None, ""),
        ("**`abc` ⇒ 人话**（旧：栈 + rc=1 ✗）", GB_RESERVE_ENV, "abc", None, "不是数字"),
        ("**`-5` ⇒ 人话**（旧：`max(0,…)` 静默吞 ⇒ 余量 0 G ✗）", GB_RESERVE_ENV, "-5", None, "正数"),
        ("**`0` ⇒ 人话**（`0` = **余量 0 G = `CACHE-GUARD-NO-HEADROOM` 那个状态本身** ✗）",
         GB_RESERVE_ENV, "0", None, "正数"),
        ("`2` ⇒ 照旧 ✓", GB_RESERVE_ENV, "2", 2.0, ""),
        ("**`\" 2\"` ⇒ 照旧**（不许收紧 ✗）", GB_RESERVE_ENV, " 2", 2.0, ""),
        ("**`\"+2\"` ⇒ 照旧**（不许收紧 ✗）", GB_RESERVE_ENV, "+2", 2.0, ""),
        ("**`nan` ⇒ 人话**（旧：`int(nan)` 丢栈 ✗）", GB_RESERVE_ENV, "nan", None, "有限"),
    ]
    saved_gb_env = {k: os.environ.get(k) for k in GB_ENV_KNOBS}
    try:
        for desc, name, raw, want_gb, hint in gb_env_cases:
            checks += 1
            for k in GB_ENV_KNOBS:
                os.environ.pop(k, None)
            if raw is not None:
                os.environ[name] = raw
            try:
                got_gb, got_why = _parse_gb_env(name)
            except Exception as exc:                  # noqa: BLE001 - 夹具要抓住"丢栈"本身
                bad += 1
                print(f"FAIL {desc}：`_parse_gb_env` **抛了**（{type(exc).__name__}: {exc}）"
                      f"—— 它必须不抛（导入期与闸门都要活到 `main()`）", file=sys.stderr)
                continue
            if got_gb != want_gb:
                bad += 1
                print(f"FAIL {desc}：值 -> {got_gb!r}，期望 {want_gb!r}", file=sys.stderr)
            elif bool(got_why) != bool(hint):
                bad += 1
                print(f"FAIL {desc}：人话的有无不对（{'有' if got_why else '无'} ⇒ {got_why[:60]!r}）",
                      file=sys.stderr)
            elif hint and hint not in got_why:
                bad += 1
                print(f"FAIL {desc}：人话没点名 {hint!r}（印出来的是：{got_why[:80]}）", file=sys.stderr)
            elif hint and name not in got_why:
                bad += 1
                print(f"FAIL {desc}：人话没**点名变量**（印出来的是：{got_why[:80]}）", file=sys.stderr)
            elif hint and "正解" not in got_why:
                bad += 1
                print(f"FAIL {desc}：人话没给**正解**（印出来的是：{got_why[:80]}）", file=sys.stderr)
    finally:
        for k, v in saved_gb_env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    # ── `set-budget <GB>` 的**同一个值域**（同族：旧写法 `float(args.arg)` 裸解析 ⇒ 栈 + rc=1；
    # `-5` 更坏 —— 它把负预算**写进 registry.json 并且一直留着** ✗）────────────────────────
    for desc, raw, want_gb, hint in (
        ("`set-budget abc` ⇒ 人话（旧：栈 + rc=1 ✗）", "abc", None, "不是数字"),
        ("`set-budget -5` ⇒ 人话（旧：把负预算**写进 registry 留着** ✗）", "-5", None, "正数"),
        ("`set-budget 0` ⇒ 人话（预算 0 ⇒ 判红线永远红 ✗）", "0", None, "正数"),
        ("`set-budget` 裸跑（空串）⇒ 无值无话（用法错由 `main()` 单说 ✓）", "", None, ""),
        ("`set-budget 60` ⇒ 照旧 ✓", "60", 60.0, ""),
        ("`set-budget \" 60\"` ⇒ 照旧（不许收紧 ✗）", " 60", 60.0, ""),
    ):
        checks += 1
        got_gb, got_why = _parse_gb_value("set-budget <GB>", raw, what="**预算**（判红线）",
                                          why="它写进 `registry.json` ⇒ 预算 ≤0 会让**每一棵树**"
                                              "都「超预算」（判红线永远红），而且**会一直留着** ✗",
                                          fix="`python3 scripts/compile-cache.py set-budget 60`")
        if got_gb != want_gb:
            bad += 1
            print(f"FAIL {desc}：值 -> {got_gb!r}，期望 {want_gb!r}", file=sys.stderr)
        elif bool(got_why) != bool(hint):
            bad += 1
            print(f"FAIL {desc}：人话的有无不对（{'有' if got_why else '无'}）", file=sys.stderr)
        elif hint and (hint not in got_why or "set-budget <GB>" not in got_why):
            bad += 1
            print(f"FAIL {desc}：人话没点名 `set-budget <GB>` / {hint!r}"
                  f"（印出来的是：{got_why[:80]}）", file=sys.stderr)

    # ⚠ **闸门的端到端**也要钉（洞的判据就是"CLI ⇒ rc=2 + 人话"）：**真起子进程 + 真 CLI**
    # （不是 monkeypatch 模块全局 —— 那测的是"我改的那个变量"，不是真路径 ✗；本仓口径：
    # "跑真程序 + 真协议，不是把正则抄一遍" ✓）。子进程走 `env probe`（**零写入**、不扫盘 ✓）。
    # ⚠ **递归闸**：子进程里不再起子进程（`SOKO_CACHE_SELFTEST_CHILD`）✗。
    if not os.environ.get("SOKO_CACHE_SELFTEST_CHILD"):
        for desc, delta, want_rc, want_tail in (
            ("`SOKO_CACHE_BUDGET_GB=abc` ⇒ rc=2 + 人话（旧：栈 + rc=1 ✗）",
             {GB_BUDGET_ENV: "abc"}, 2, "不是数字"),
            ("`SOKO_CACHE_RESERVE_GB=-5` ⇒ rc=2 + 人话（旧：**rc=0 静默** ✗）",
             {GB_RESERVE_ENV: "-5"}, 2, "正数"),
            ("`SOKO_CACHE_RESERVE_GB=0` ⇒ rc=2 + 人话（余量 0 G = 那个 bug 的状态本身 ✗）",
             {GB_RESERVE_ENV: "0"}, 2, "正数"),
            ("`SOKO_CACHE_BUDGET_GB=-5` ⇒ rc=2 + 人话（旧：判红线永远红 ✗）",
             {GB_BUDGET_ENV: "-5"}, 2, "正数"),
            ("**正对照** `SOKO_CACHE_BUDGET_GB=200`（CI 口径）⇒ rc=0", {GB_BUDGET_ENV: "200"}, 0, ""),
            ("**正对照** 两个都不设 ⇒ rc=0", {}, 0, ""),
        ):
            checks += 1
            env = {k: v for k, v in os.environ.items() if k not in GB_ENV_KNOBS}
            env.update({k: v for k, v in delta.items() if v is not None})
            env["SOKO_CACHE_SELFTEST_CHILD"] = "1"
            try:
                proc = subprocess.run(
                    [sys.executable, str(Path(__file__).resolve()), "env", "probe"],
                    capture_output=True, text=True, encoding="utf-8", errors="replace",
                    env=env, timeout=60)
            except (OSError, subprocess.SubprocessError) as exc:
                bad += 1
                print(f"FAIL 端到端 {desc}：子进程起不来（{type(exc).__name__}: {exc}）",
                      file=sys.stderr)
                continue
            tail = proc.stderr.strip().splitlines()[-1][:110] if proc.stderr.strip() else "（空）"
            if proc.returncode != want_rc:
                bad += 1
                print(f"FAIL 端到端 {desc}：rc={proc.returncode}，期望 {want_rc}（stderr 尾：{tail}）",
                      file=sys.stderr)
            elif want_rc != 0 and "Traceback" in proc.stderr:
                bad += 1
                print(f"FAIL 端到端 {desc}：rc 对了但**丢栈** ✗（判据是「判定」不是「崩溃」）",
                      file=sys.stderr)
            elif want_tail and want_tail not in proc.stderr:
                bad += 1
                print(f"FAIL 端到端 {desc}：stderr 没点名 {want_tail!r}（stderr 尾：{tail}）",
                      file=sys.stderr)

    # 孤儿的判定：`<worktree>/target` 在、工作树已消失 ⇒ 必须被枚举成 dead（可回收）
    import tempfile as _tf
    with _tf.TemporaryDirectory() as td:
        # 形状要照**受管约定**放：`<主工作树>/.worktrees/<id>/target`（`managed_entries` 的扫描面；
        # 侧放一个没有 `.git` 的兄弟目录当**反面对照** ⇒ 它必须**不被**枚举成孤儿 ✓）
        main = Path(td)
        (main / "target").mkdir(parents=True)
        # 主工作树的形状：根下是 **`.git` 目录**（`_is_worktree_alive` 两种形状都要认 ✓）——
        # 少了它，夹具里的"主工作树"会被判死，`_hard_safety_check` 立刻拒绝回收 ✓（判据工作正常 ✓）。
        (main / ".git").mkdir()
        root = main / ".worktrees" / "gone"
        (root / "target").mkdir(parents=True)
        (root / ".git").write_text("gitdir: /nowhere\n", encoding="utf-8")
        decoy = main / ".worktrees" / "not-a-worktree" / "target"
        decoy.mkdir(parents=True)
        # ⚠ **`REPO` 故意指到"另一棵 checkout"**（`td/linked`，没有 `.worktrees`/`.git`）：
        # 受管面必须跟随 **`MAIN_WORKTREE`**（仓库级共享），**不跟随当前 checkout** ✗⇒✓ ——
        # 这正是 `CACHE-GUARD-NO-HEADROOM` 那条"worktree 里守卫看不见受管 target"的判据 ✓。
        linked = main / "linked"
        linked.mkdir()
        saved = {k: globals()[k] for k in ("worktrees", "REPO", "CACHE_ROOT", "MAIN_WORKTREE")}
        globals()["worktrees"] = lambda: [{"path": main, "branch": "main"}]
        globals()["REPO"] = linked
        globals()["MAIN_WORKTREE"] = main
        globals()["CACHE_ROOT"] = main / ".cache"
        try:
            ents = list(managed_entries().values())
            orphan = [e for e in ents if e["kind"] == "orphan-worktree"]
            expect(f"孤儿 worktree target 被枚举（{len(orphan)} 条）", len(orphan) == 1)
            expect("孤儿被标 dead（⇒ reclaim 第①档才会碰它）",
                   bool(orphan) and orphan[0]["dead"] is True)
            expect("**没有 `.git` 的兄弟目录不被误认**（反面对照）",
                   all("not-a-worktree" not in str(e["path"]) for e in orphan))
            expect("⚠ 受管面跟随 **主工作树**（`REPO` 是另一棵 checkout 时也找得到 ✗⇒✓）",
                   bool(orphan) and str(orphan[0]["path"]).startswith(str(main)))
        finally:
            for k, v in saved.items():
                globals()[k] = v

    # `cmd_env` 的**根口径**（洞 `COMPILE-CACHE-EXPORTS-QUEUE-ROOT`，2026-10-15 ✗⇒✓）：
    # 同一个 env 块里两个"根"**故意相反**，两边都要钉住 —— 只钉一边就会把另一边改错 ✗：
    #   `SOKO_QUEUE_ROOT` = **主工作树**（队列状态单点，`agent-worktree.sh:25` 的契约）；
    #   `SOKONANODA_CACHE_DIR` = **当前 checkout**（编译产物缓存按工作树独立，`agent-worktree.sh:26`）。
    # 反向验证：把两者设成**两个不同目录** ⇒ 导出的值必须**各自**对上（谁跟错根都判红 ✓）。
    saved_env = {k: globals()[k] for k in ("REPO", "MAIN_WORKTREE", "CACHE_ROOT")}
    import contextlib as _cl
    import io as _io

    globals()["REPO"] = Path("/m/wt")
    globals()["MAIN_WORKTREE"] = Path("/m/main")
    globals()["CACHE_ROOT"] = Path("/m/main/.cache/compile")
    try:
        buf = _io.StringIO()
        with _cl.redirect_stdout(buf):
            cmd_env("probe")
        env = buf.getvalue()
        expect("`SOKO_QUEUE_ROOT` 导出**主工作树**（不是当前 checkout ✗）",
               'export SOKO_QUEUE_ROOT="/m/main"' in env)
        expect("`SOKONANODA_CACHE_DIR` 导出**当前 checkout**（编译缓存按树独立 ✓）",
               'export SOKONANODA_CACHE_DIR="/m/wt/.cache/soko/probe"' in env)
        expect("`CARGO_TARGET_DIR` 跟随**受管根**（主工作树 ✓）",
               'export CARGO_TARGET_DIR="/m/main/.cache/compile/targets/probe"' in env)
    finally:
        for k, v in saved_env.items():
            globals()[k] = v

    if bad:
        print(f"compile-cache.py selftest: {bad}/{checks} 条判据不成立 ✗", file=sys.stderr)
        return 1
    # ⚠ 条数是**当场数出来的**（`checks`）—— 别写死数字 ✗（"每个数字当场核"：写死的 19
    # 在本轮加了 22 + 6 + 6 = 34 条之后就变成假读数了 ✓）。
    print(f"compile-cache.py selftest: {checks} 条成立 ✓（死树优先 · LRU 最旧优先 · 预算反向验证 · "
          "registry/默认预算 · **空 dict ≠ 没传** · **回收余量 5% / `reserve_gb` / 判红线口径不变** · "
          "**GB 旋钮的值域闸门（两向：错值必须有人话/点名/正解，合法值必须逐字节不变）** · "
          "**孤儿 target 被枚举成 dead** · **受管面跟随主工作树** · "
          "**`cmd_env` 的根口径：队列单点=主工作树 / 编译缓存=当前 checkout**）")
    return 0


def main() -> int:
    # ⭐ **GB 旋钮的值域闸门**（洞 `COMPILE-CACHE-GB-ENVS-NO-GUARD`）：派发**之前**判
    # ⇒ `abc`/`nan`/`inf` 不再丢栈、`-5` 不再被静默吞掉（余量 0 G），两者都给 rc=2 + 人话 + 正解 ✓。
    rc = _guard_gb_knobs()
    if rc is not None:
        return rc
    ap = argparse.ArgumentParser(description="编译缓存总管（跨工作树：登记 · 计量 · 预算 · 回收 · sccache）")
    ap.add_argument("cmd", nargs="?", default="status",
                    choices=["status", "env", "reclaim", "set-budget", "selftest"])
    ap.add_argument("arg", nargs="?", default="", help="env 的工作树 id / set-budget 的 GB")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--quiet", action="store_true", help="只打一行总量（给 gate 的报告段用）")
    ap.add_argument("--to-budget", action="store_true", help="回收直到落回预算内")
    ap.add_argument("--selftest", action="store_true", help="判据通道自检（等价 `selftest` 子命令）")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.cmd == "status":
        return cmd_status(args.json, args.quiet)
    if args.cmd == "env":
        return cmd_env(args.arg)
    if args.cmd == "reclaim":
        return cmd_reclaim(args.to_budget)
    if args.cmd == "set-budget":
        # **同一个值域**（`_parse_gb_value` = 唯一解析处 ✓）：`set-budget abc` 旧写法是
        # `float(args.arg)` ⇒ **栈 + rc=1**（同族崩溃形 ✗）；而 `set-budget -5` 更坏 ——
        # 它把负预算**写进 registry.json 并且一直留着**（比环境变量更难发现 ✗）⇒ 拒绝写 ✓。
        gb, why = _parse_gb_value(
            "set-budget <GB>", args.arg, what="**预算**（判红线）",
            why="它写进 `registry.json` ⇒ 预算 ≤0 会让**每一棵树**都「超预算」"
                "（判红线永远红），而且**会一直留着**（比环境变量更难发现 ✗）",
            fix="`python3 scripts/compile-cache.py set-budget 60`")
        if why:
            print(f"error: {why}", file=sys.stderr)
            return 2
        if gb is None:                        # 裸跑 `set-budget`（没给值）
            print("error: 用法：`python3 scripts/compile-cache.py set-budget <GB>`"
                  "（例如 `set-budget 60`；GB 数，正数）✗", file=sys.stderr)
            return 2
        reg = load_registry()
        reg["budget_gb"] = gb
        save_registry(reg)
        print(f"✓ 预算设为 {args.arg} GB（写进 {REGISTRY}）")
        return 0
    return selftest()


if __name__ == "__main__":
    sys.exit(main())
