#!/usr/bin/env python3
"""**手动 `git rebase` 之后**的收口指针回填**触发器**（`LAND-REBASE-STALES-CLOSURE-SHAS` 的第三条路）。

## 缺的是什么（**实测**，不是推演 ✗）

`LAND-REBASE-STALES-CLOSURE-SHAS` 这个族有**三条**产生路，前两条已堵：

| # | 路 | 谁堵的 | 机制 |
|---|---|---|---|
| ① | `scripts/agent-worktree.sh land` | `b175bbe9`（v1） | `repoint_after_land()`：rebase + 快进**之后**跑 `--repoint-all` **并提交** |
| ② | 「在被 rebase 的那棵树里收口」 | `9d53e538`（qr1） | `_guard_root()`：`ROOT` 是**链接工作树**就一律拒（`gd != gcd`） |
| ③ | **手动 `git rebase`** | ← **本文件** | 下面 |

⚠ **第三条路为什么是敞的**（本会话实测 3 次：`086fd7d5`→`9d53e538` · `b9dd26f8`→`64304006`
· 本文件落地前又造了一次）：`land` 的机械回填**只挂在 `land` 上** ✗。而 agent 在 worktree 里
**手敲** `git rebase origin/main` 是日常动作（同步主线）—— 那一步把分支提交重写成新 sha，
**没有任何触发器**去回填那些"落 main 前"记的收口指针 ⇒ 指针留在账上，直到 CI 的
`queue-lint` 在 main 上判红（`rc=1`；对象没推上去时 `rc=128`）✗。

⚠ **为什么不能只靠"再跑一次 `--repoint-all`"** ✗：它的判据是**相对当前 `HEAD` 的祖先性**
（`scripts/ux-queue.py` 的 `repoint_targets()`）——
  · 在**正被 rebase 的那棵树**里（`HEAD` = 分支 tip，rebase 前）那些 sha 还是**祖先** ⇒
    如实报「没有非祖先的（0 条）」（实测 ✓，v1 的复核也记了同一读数）；
  · rebase **之后**、**落 main 之前**，孪生还没进 main ⇒ `find_twin()` 找不到 ⇒ 只报不填 ✓。
⇒ 所以**触发时机**就是这条路的全部：`git` 的 **`post-rewrite` 钩子**是唯一在"手动 rebase
刚刚发生"这一刻被调用、且能拿到 `<old> <new>` 对照表的钩子 ✓（`core.hooksPath` 本仓
**已装**：`scripts/install-hooks.sh` ⇒ `scripts/githooks` ✓，所以这不是"新造一条安装步骤" ✗）。

## 它做什么（三条，**一条都不许静默** ✗）

1. `--from-hook rebase`（由 `scripts/githooks/post-rewrite` 调）：rebase 结束后，对**队列单点**
   （`git worktree list` 第一行 = 主 checkout ✓，与 `ci-local.sh` / `compile-agent` 同一件仪器）
   跑一次 `ux-queue.py --repoint-all`：
   * **孪生已经在 main 上**（主 checkout 也刚被 rebase 过 / 分支的活已落 main）⇒ **真回填**，
     并在事件表**落地前就是干净的**时候**顺手提交**（照 `repoint_after_land()` 的形状 ✓
     —— 只回填不提交 = 本地绿、CI 红，**两层遮蔽** ✗）；
   * **孪生还没进 main**（最常见：刚 rebase、还没 land）⇒ `--repoint-all` 自己 rc=1 且
     **一条都不写** ✓ ⇒ 这里**响亮地点名**那些 id + 给出下一步（`land` 会自动做 ✓）；
   * 没有可回填的 ⇒ 一行 `✓ 0 条`（**不许静默**：绿也要出声 ✓）。
   ⚠ **永不阻塞 rebase** ✗：钩子一律 `exit 0`（账目问题不该让 `git rebase` 失败 —— 那会把人
   引到错的地方）；代价是"必须响亮"，所以上面每一条都印字 ✓。
   ⚠ **不提交的两种情形**（都**明确印出来** ✗）：事件表**本来就有未提交改动**（可能是 agent
   自己的 `--advance`）⇒ 不抢着替你提交；`git commit` 失败（`pre-commit` 判红之类）⇒ 明说
   "现在正是『本地绿、CI 红』的形状" ✓。

2. `--check`（判据·绿的那条）：触发器**接线齐**（钩子文件在、可执行、指向本文件）+ 覆盖
   **`--bite` 的口径没空转**（下面那条固定旧 rev 真的没有触发器 ✓）。

3. `--bite`（判据·**咬住证明**）：在**钉死的旧 rev** `PINNED_PRE_TRIGGER_REV` 上问同一组问题
   ⇒ 那份树**没有触发器** ⇒ **必须报红**（rc=1）✓。⚠ 为什么钉固定 rev（`pr1` 实测的坑 ✗）：
   写 `HEAD` 的话，本改动一提交，`HEAD` 就**是**修好的那版 ⇒ 证明**逐字节空转** ✓。

## 判据（一处定义，别处不抄 ✗）

  python3 scripts/check-queue-rewrite-trigger.py --check       # 绿：接线齐 + 咬住口径没空转
  python3 scripts/check-queue-rewrite-trigger.py --bite        # 红：固定旧 rev 上没有触发器
  python3 scripts/check-queue-rewrite-trigger.py --selftest    # 行为三臂（回填 / 不误伤 / 空转闸）

退出码：0 = 绿 / 1 = 判红 / 2 = 用法错 / 3 = 环境错（**不假装判过** ✗）。
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
HOOK_REL = "scripts/githooks/post-rewrite"
EVENTS_REL = "docs/ux-queue-events.jsonl"

# ⭐ **钉死的旧 rev**（本改动落地前 main 的 HEAD ✓）——**不许写 `HEAD`** ✗：
# 写 `HEAD` ⇒ 提交之后 `HEAD` 就是修好的那版 ⇒ `--bite` 变成空转（`pr1` 2026-10-15 实测踩过 ✗）。
PINNED_PRE_TRIGGER_REV = "1942bd8a"
PINNED_WHY = ("本改动落地前 main 的 HEAD（`chore(queue): 把 AGENT-DOES-NOT-PRODUCE-USER-INPUTS "
              "的收口指针改到 rebase 后的孪生（64304006）`）—— 那份树里没有 "
              f"`{HOOK_REL}`，也没有本文件")


def clean_git_env(base: dict | None = None) -> dict:
    """**去掉 git 自己的 `GIT_*` 环境变量**（本文件**所有** git 调用与子进程的唯一环境来源 ✓）。

    ⭐⭐ **为什么必须有**（本文件落地时**实测抓到**，是这条路上最阴的一个坑 ✗）：
    `git rebase` 在**链接工作树**里会把 **`GIT_DIR`** 交给 `post-rewrite` 钩子（实测读数：
    `GIT_DIR=/private/tmp/mb1-gd3/r/.git/worktrees/wt`，而 `GIT_WORK_TREE`/`GIT_COMMON_DIR` 都没设）。
    而 `GIT_DIR` **优先于 cwd** 决定仓库发现 ⇒ 从那一刻起：
      · `_is_linked_worktree(<主 checkout>)` 的 `git rev-parse --git-dir` 会返回**工作树**的 git 目录、
        `--git-common-dir` 返回主的 ⇒ **两个不同 ⇒ 判"队列单点是链接工作树" ⇒ 跳过回填** ✗
        （**触发器在最需要它的那一刻变成空转** —— 实测：R3 打出「队列单点是链接工作树」而
        队列单点明明是主 checkout ✗）；
      · 更糟的是 `ux-queue.py` 子进程**也继承**它 ⇒ `ancestry_rc()` 的
        `git merge-base --is-ancestor <sha> HEAD`（`cwd=ROOT`）会拿**工作树的 HEAD** 当基点 ⇒
        **审错对象**（正是本族要堵的形状 ✗）。
    ⇒ 口径：**仓库发现只许由 `cwd` 决定** ⇒ 一律把 `GIT_*` 全部摘掉（`GIT_AUTHOR_*`/
    `GIT_COMMITTER_*` 一并摘掉也**更好**：提交身份应当来自仓库配置，不该被钩子的调用方注入 ✓）。
    ⚠ 本函数是**唯一一处**（别在每个 `subprocess.run` 边上各写一遍 ✗ —— 硬规则 0(b)）。
    """
    src = os.environ if base is None else base
    return {k: v for k, v in src.items() if not k.startswith("GIT_")}


def _git(cwd: Path, *argv: str, stdin: str | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *argv], cwd=str(cwd), capture_output=True,
                          text=True, input=stdin, env=clean_git_env())


def _first_worktree(cwd: Path) -> str | None:
    """`git worktree list --porcelain` 第一行 = **主工作树**（与 `ci-local.sh` 同一件仪器 ✓）。"""
    out = _git(cwd, "worktree", "list", "--porcelain")
    if out.returncode != 0:
        return None
    for line in out.stdout.splitlines():
        if line.startswith("worktree "):
            return line[len("worktree "):].strip()
    return None


def queue_root() -> Path:
    """队列单点（**与 `ux-queue.py` 同一个解析链** ✓）：`SOKO_QUEUE_ROOT` → 主 checkout → 本仓。

    ⚠ **`SOKO_QUEUE_ROOT` 优先**（实测踩过 ✗）：`ux-queue.py` 自己的 `ROOT` 就是
    `SOKO_QUEUE_ROOT or <脚本所在仓>` ⇒ 本文件若忽略它，触发器就会把 `--repoint-all`
    打到**另一棵树**上（自检夹具里实测：事件表在夹具、命令跑在真仓 ⇒ 报「0 条」而夹具那份
    **一字节没动**）—— 那正是本族要堵的"审错对象" ✗。`compile-cache.py env` / `ci-local.sh`
    都会 export 它（= 主 checkout ✓），所以正常路径下两者一致 ✓。
    """
    raw = os.environ.get("SOKO_QUEUE_ROOT")
    if raw:
        return Path(raw).resolve()
    first = _first_worktree(ROOT)
    return Path(first).resolve() if first else ROOT


def _uxq() -> Path:
    """**工具本体** = 本文件的同目录兄弟 `ux-queue.py`（**不是** `<队列根>/scripts/…` ✗）。

    ⚠ **为什么必须分开**（实测踩过 ✗）：**代码**与**数据**是两件事 ——
      · **代码** = "我现在跑的是哪一版工具"（= 本文件所在的那棵树 ✓）；
      · **数据** = "账在哪一棵树"（= `queue_root()`，`SOKO_QUEUE_ROOT` → 主 checkout ✓）。
    `ci-local.sh` 的 `queue-lint` 步就是同一个形状（脚本按自己的位置找，`SOKO_QUEUE_ROOT`
    只覆盖**数据根** ✓）。自检夹具里 `<队列根>/scripts/ux-queue.py` 是 `_fixture_queue_root()`
    写的**根标记占位文件**（一行注释）⇒ 拿它当工具跑 = **什么都不做、rc=0** ⇒ 臂 ① 假红
    （实测：事件表 1 → 1 行、`--repoint-all` 一个字都不印）✗。
    """
    return HERE / "ux-queue.py"


def _is_linked_worktree(path: Path) -> bool:
    gd = _git(path, "rev-parse", "--git-dir").stdout.strip()
    gcd = _git(path, "rev-parse", "--git-common-dir").stdout.strip()
    if not gd or not gcd:
        return False
    return (Path(gd) if Path(gd).is_absolute() else path / gd).resolve() != \
           (Path(gcd) if Path(gcd).is_absolute() else path / gcd).resolve()


# ── 触发器本体（`--from-hook`） ───────────────────────────────────────────────
REPOINT_REASON = "手动 rebase 后自动回填：`post-rewrite` 触发器（rebase 把分支提交重写成孪生）"


def trigger(kind: str) -> int:
    """`post-rewrite` 钩子的**唯一实现**（钩子本体只是三行转发 ⇒ 逻辑只有一处 ✓）。

    ⚠ **永远返回 0** ✗：账目问题不该让 `git rebase` 失败（那会把人引到错的地方）；
    代价是**每一档都必须印字**（绿也要出声 ✓）。
    """
    if os.environ.get("SOKO_SKIP_HOOK") == "1":
        print(f"[post-rewrite] SOKO_SKIP_HOOK=1 ⇒ 跳过收口指针回填 ✗"
              f"（例外要留痕：在 STATUS.md 写明原因）")
        return 0
    if kind != "rebase":
        # `commit --amend` 也走本钩子（`kind == "amend"`）。口径**只在 rebase** 这一条路上
        # 落地（本缺口记的就是 `git rebase`）—— amend 不动分支基底、不产生"落 main 前"的
        # 新 sha ⇒ 这里**如实说**，不假装覆盖 ✗（要扩是另一条账）。
        print(f"[post-rewrite] kind=`{kind}` ⇒ 本触发器只覆盖 `git rebase`（见 docstring 的口径）"
              f"⇒ 不动作 ✓")
        return 0
    pairs = [ln.split() for ln in sys.stdin.read().splitlines() if ln.strip()]
    qroot = queue_root()
    q = _uxq()
    # **数据侧**的根标记：`<队列根>/scripts/ux-queue.py`（`_guard_root` 的必需文件之一 ✓）
    if not (qroot / "scripts" / "ux-queue.py").exists() or not q.exists():
        print(f"[post-rewrite] ⓘ 队列根 `{qroot}` 里没有 `scripts/ux-queue.py` ⇒ 本仓没有队列 ⇒ 跳过 ✓")
        return 0
    if _is_linked_worktree(qroot):
        # 与 `ux-queue.py::_guard_root()` **同一口径**（别自己再写一遍 ✗）：队列单点必须是主 checkout。
        print(f"[post-rewrite] ⚠ 队列单点 `{qroot}` 是**链接工作树** ⇒ `ux-queue.py` 会拒（rc=2）"
              f"⇒ 跳过回填（正解：`git worktree list` 第一行那个主 checkout ✓）")
        return 0
    py = shutil.which("python3") or sys.executable
    if not py:
        print(f"[post-rewrite] ⚠ 没有 python3 ⇒ **判不了**（判不了不是绿 ✗）⇒ 手动补跑："
              f"`SOKO_QUEUE_ROOT={qroot} python3 scripts/ux-queue.py --repoint-all --reason '…'`",
              file=sys.stderr)
        return 0

    # ⚠ **不抢着替你提交**（可能盖掉 agent 自己的 `--advance`）✗：落地前先看事件表干不干净。
    dirty_before = _git(qroot, "status", "--porcelain", "--", EVENTS_REL).stdout.strip()
    # ⚠ **子进程也必须过 `clean_git_env()`** ✗：`GIT_DIR` 会让 `ux-queue.py` 的
    #   `ancestry_rc()`（`git merge-base --is-ancestor … HEAD`，`cwd=ROOT`）拿**工作树的 HEAD**
    #   当基点 ⇒ 审错对象（见 `clean_git_env()` 的实测）✗。
    env = dict(clean_git_env(), SOKO_QUEUE_ROOT=str(qroot))
    before = _line_count(qroot / EVENTS_REL)
    out = subprocess.run([py, str(q), "--repoint-all", "--reason", REPOINT_REASON],
                         cwd=str(qroot), capture_output=True, text=True, env=env)
    after = _line_count(qroot / EVENTS_REL)
    wrote = max(0, after - before)

    print(f"[post-rewrite] 手动 rebase 完成（改写 {len(pairs)} 个提交）⇒ 收口指针回填检查"
          f"（队列单点 = {qroot} ✓）")
    for line in (out.stdout + out.stderr).splitlines():
        print(f"  {line}")
    if out.returncode != 0:
        print(f"  ✗ **没回填到 0**（rc={out.returncode}）⇒ 上面每一条自己的出路就是出路；"
              f"⚠ 最常见的那档是「孪生还没进 main」（刚 rebase、还没 land）⇒ "
              f"`scripts/agent-worktree.sh land <id>` 会在快进之后**自动**做掉 ✓", file=sys.stderr)
        return 0
    if wrote == 0:
        return 0
    print(f"  ✓ 已回填 **{wrote} 行**事件（repoint，只追加 ⇒ 可逐条回滚 ✓）")
    if dirty_before:
        print(f"  ⚠ **没替你提交**：`{EVENTS_REL}` 在你这次 rebase **之前**就有未提交改动 "
              f"⇒ 抢着提交会把它一起卷进来 ✗。⚠ 但**只回填不提交 = 本地绿、CI 红**（两层遮蔽 ✗）"
              f"⇒ 请连同它一起提交：`git -C {qroot} commit -m 'chore(queue): 手动 rebase 后回填收口指针' "
              f"-- {EVENTS_REL}`", file=sys.stderr)
        return 0
    msg = (f"chore(queue): 手动 rebase 后自动回填收口指针（post-rewrite 触发器）—— "
           f"本次 +{wrote} 行 repoint 事件")
    c = _git(qroot, "commit", "-q", "-m", msg, "--", EVENTS_REL)
    if c.returncode != 0:
        print(f"  ✗ **提交失败** ⇒ 现在正是『本地绿、CI 红』的形状 ✗（工作区已回填、HEAD 里没有）："
              f"{c.stderr.strip()[:200]}\n     手工提交它：`git -C {qroot} commit -m '{msg}' "
              f"-- {EVENTS_REL}`", file=sys.stderr)
        return 0
    head = _git(qroot, "log", "--oneline", "-1").stdout.strip()
    print(f"  ✓ 已提交：{head}")
    return 0


def _line_count(path: Path) -> int:
    try:
        return len(path.read_text(encoding="utf-8", errors="surrogateescape").splitlines())
    except OSError:
        return 0


# ── 判据：接线齐（绿的那条） ────────────────────────────────────────────────
def wiring_problems(code_root: Path) -> list[str]:
    """触发器**接线**的四条硬要求（**唯一一处**：`--check` 与 `--bite` 共用 ✓）。

    ⚠ 参数是**代码侧**的树（本文件所在的那棵 = `ROOT`）**不是**队列根 ✗：钩子是**代码**
    （"这一版工具接上触发器了吗"），而队列根是**数据**。开发期两者常不同树
    （改动在 worktree、账在主 checkout）⇒ 混用会让 `--check` 在 worktree 里假红 ✗。
    """
    bad: list[str] = []
    hook = code_root / HOOK_REL
    if not hook.exists():
        bad.append(f"`{HOOK_REL}` 不存在 ⇒ **手动 `git rebase` 之后没有任何触发器** ✗"
                   f"（这正是本缺口：`land` 的机械回填只挂在 `land` 上）")
        return bad
    if not os.access(hook, os.X_OK):
        bad.append(f"`{HOOK_REL}` 不可执行 ⇒ git 不会调它（`chmod +x`）✗")
    body = hook.read_text(encoding="utf-8", errors="replace")
    if "check-queue-rewrite-trigger.py" not in body:
        bad.append(f"`{HOOK_REL}` 没有转发到本文件（`--from-hook`）⇒ 触发器与判据**两处口径** ✗")
    if "--from-hook" not in body:
        bad.append(f"`{HOOK_REL}` 没有传 `--from-hook` ⇒ 触发器进不去本文件的唯一实现 ✗")
    return bad


def cmd_check() -> int:
    qroot = queue_root()
    bad = wiring_problems(ROOT)
    # ⚠ **咬住口径不许空转**：`--bite` 靠"固定旧 rev 上没有触发器"成立 ⇒ 这里顺手核一次 ✓。
    rev_has = _rev_has_hook(PINNED_PRE_TRIGGER_REV)
    if rev_has is None:
        print(f"✗ 量不到固定旧 rev `{PINNED_PRE_TRIGGER_REV}` ⇒ `--bite` 的口径**没法核** ✗"
              f"（{PINNED_WHY}）", file=sys.stderr)
        return 3
    if rev_has:
        print(f"✗ 空转闸：固定旧 rev `{PINNED_PRE_TRIGGER_REV}` **已经有** `{HOOK_REL}` ⇒ "
              f"`--bite` 会**逐字节空转** ✗ ⇒ 换一个**真的没有触发器**的旧 rev（别写 `HEAD` ✗）",
              file=sys.stderr)
        return 1
    if bad:
        for b in bad:
            print(f"✗ {b}", file=sys.stderr)
        return 1
    print(f"✓ 触发器接线齐：`{HOOK_REL}`（可执行）⇒ `scripts/check-queue-rewrite-trigger.py "
          f"--from-hook`（唯一实现）· 队列单点 = `{qroot}`")
    print(f"✓ 咬住口径没空转：固定旧 rev `{PINNED_PRE_TRIGGER_REV}` 上确实没有 `{HOOK_REL}` ✓")
    hp = _git(qroot, "config", "--get", "core.hooksPath").stdout.strip()
    if hp:
        print(f"✓ `core.hooksPath = {hp}` ⇒ 钩子真的会被 git 调 ✓（`scripts/install-hooks.sh` ✓）")
    else:
        print(f"ⓘ `core.hooksPath` **没装** ⇒ 本钩子在本 clone 里**不会被调**（新 clone 的正常状态 ✓）"
              f"⇒ 装：`scripts/install-hooks.sh`（**不是判红**：那是 Setup 的一步，"
              f"CI/新 clone 都没有 ✗）")
    return 0


def _rev_has_hook(rev: str) -> bool | None:
    """固定旧 rev 的那份树里有没有 `scripts/githooks/post-rewrite`（量不到 ⇒ `None`）。"""
    out = _git(ROOT, "cat-file", "-e", f"{rev}:{HOOK_REL}")
    if out.returncode == 0:
        return True
    out2 = _git(ROOT, "cat-file", "-t", rev)
    if out2.returncode != 0:
        return None
    return False


def cmd_bite() -> int:
    """**咬住证明**：在固定旧 rev 上，同一组接线判据**必须报红** ✓。

    ⭐ **咬住的是一条已存在的坏输入**（用户 2026-10-15【④】）——不是新造夹具：
    它就是**本仓自己的历史状态** `PINNED_PRE_TRIGGER_REV`（本改动落地前的 main），
    那份树里 `land` 的机械回填**只挂在 `land` 上**，手动 `git rebase` 之后没有任何触发器
    ⇒ 本会话实测 3 次同形（`086fd7d5` → `9d53e538` · `b9dd26f8` → `64304006` · 复现件再造 1 次）。
    ⚠ **空转闸**：若那个 rev 上居然**有**触发器 ⇒ rc=3（口径失效，**不许**当成"咬住了" ✗）。
    """
    rev_has = _rev_has_hook(PINNED_PRE_TRIGGER_REV)
    if rev_has is None:
        print(f"✗ 量不到固定旧 rev `{PINNED_PRE_TRIGGER_REV}` ⇒ 咬住证明**没法跑** ✗", file=sys.stderr)
        return 3
    if rev_has:
        print(f"✗ 空转闸：`{PINNED_PRE_TRIGGER_REV}` 上**已经有** `{HOOK_REL}` ⇒ 这条证明"
              f"**逐字节空转** ✗（钉的 rev 必须真的没有触发器；别写 `HEAD` ✗）", file=sys.stderr)
        return 3
    # 在**那份树**里问接线（用 `git show` 取那一版的钩子文件清单 ⇒ 不切树、不动工作区 ✓）。
    print(f"✗ 手动 `git rebase` 之后**没有任何触发器**（固定旧 rev `{PINNED_PRE_TRIGGER_REV}`："
          f"`{HOOK_REL}` 不存在）⇒ 那些「落 main 前」记的收口指针没人回填 ⇒ CI 的 `queue-lint` 在 "
          f"main 上判红 ✗")
    print(f"  ⇒ {PINNED_WHY}")
    print(f"  ⇒ 落地后的形状：`{HOOK_REL}` 转发到 `scripts/check-queue-rewrite-trigger.py "
          f"--from-hook rebase`（`--check` 绿 ✓）")
    return 1


# ── 行为自检（三臂，夹具） ──────────────────────────────────────────────────
def _load_uxq():
    """把 `scripts/ux-queue.py` 当模块载入（复用它的**夹具构造器**，别抄第二份 ✗）。"""
    spec = importlib.util.spec_from_file_location("mb1_uxq", ROOT / "scripts" / "ux-queue.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["mb1_uxq"] = mod
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


def _fixture_repo(d: Path, ident: str) -> tuple[str, str, str]:
    """造一个**自包含**夹具：主分支 `main` 上有孪生 `twin`，`agent/x` 上有原提交 `orig`。

    返回 `(orig, twin, unrelated)`：
      · `orig` —— 分支上的提交（= 「落 main 前」记的收口指针）；
      · `twin` —— `main` 上**同 subject + 同 patch-id** 的孪生（rebase 的目标 ✓）；
      · `unrelated` —— **合法但还没推**的本地提交（内容**没**落 main ⇒ 正对照，不许误伤 ✗）。

    ⚠ **`orig != twin` 是夹具的硬前提** ✗（实测踩过：`orig` 与 `twin` 的 tree / parent /
    message 全一样 ⇒ **sha 逐字节相同** ⇒ 指针"已经是祖先" ⇒ 触发器报 0 条 ⇒ 臂 ① 假红）。
    ⇒ 让 `main` 在孪生之前**先走一步**（多一个 `main-only.txt` 提交）⇒ parent 不同 ⇒ sha 不同，
    而 `patch-id --stable` 仍逐字节相同（diff 一样 ✓）—— 这正是 `git rebase` 重写的形状 ✓。

    ⚠⚠ **身份必须写进夹具仓的 `config`**（2026-10-16 实测，CI 红 ✗→✓）：
    本文件上面的提交都自带 `-c user.name=…`，**但被考的那一次提交不是我们打的** ——
    是**触发器自己**在 `trigger()` 里打的（`_git(qroot, "commit", …)`，见那里为什么
    **不**加 `-c`：`clean_git_env()` 的口径是"**提交身份应当来自仓库配置**，不该被钩子的
    调用方注入" ✓）。⇒ runner 上没有 `user.name`/`user.email`（`actions/checkout` 只配
    `safe.directory`）且 `git` 又**自动探测不到**（CI 上是 `runner@…(none)`）时，那一次
    `git commit` 直接失败 ⇒ 回填**只写工作区、没进 HEAD** ⇒ 臂 ①d/⑤b 判红：
    `✗ 自检判红 2/11 条：①d 回填**已提交**（不提交 = 本地绿、CI 红 ✗）：<sha> fixture: pointer · ⑤b …`
    （**本地绿、CI 红** 正是这条臂的注释点名的那档 ✗）。同族先例：
    `ux-queue.py::_fixture_commit` 的 docstring 记的 `SELFTEST-FIXTURE-FAILS-IN-CI`。
    ⇒ 夹具**自带**身份（仓库配置，不是环境变量 —— `clean_git_env()` 会把 `GIT_*` 全摘掉 ✗）。
    """
    uxq = _load_uxq()
    uxq._fixture_queue_root(d, ident)
    # 夹具仓自带身份：被考的那次提交由**触发器**打（它只认仓库配置 ✓）。
    _git(d, "config", "user.name", "fixture")
    _git(d, "config", "user.email", "fixture@local")
    _git(d, "symbolic-ref", "HEAD", "refs/heads/main")
    _git(d, "add", "-A")
    _git(d, "-c", "user.name=fixture", "-c", "user.email=f@local", "commit", "-qm", "base")
    # 分支上的原提交
    _git(d, "checkout", "-q", "-b", "agent/x")
    orig = uxq._fixture_commit(d, "work.txt", "same content\n", "fix: 同一个环节")
    # 合法但**没落 main** 的本地提交（正对照）
    unrelated = uxq._fixture_commit(d, "local-only.txt", "not landed\n", "fix: 只在本地")
    # 回到 main：**先走一步**（parent 变了 ⇒ 孪生的 sha 与原提交不同 ✓），再落孪生。
    _git(d, "checkout", "-q", "main")
    uxq._fixture_commit(d, "main-only.txt", "main moved on\n", "chore: 主线自己前进一步")
    twin = uxq._fixture_commit(d, "work.txt", "same content\n", "fix: 同一个环节")
    assert orig != twin, "夹具前提破了：孪生与原提交同 sha ⇒ 臂 ① 会假绿 ✗"
    return orig, twin, unrelated


def _write_pointer(d: Path, ident: str, commit: str) -> None:
    (d / EVENTS_REL).write_text(json.dumps(
        {"ts": "2026-10-16T00:00:00+00:00", "event": "advance", "id": ident, "to": "done",
         "commit": commit, "reason": None, "owner": "selftest"},
        ensure_ascii=False) + "\n", encoding="utf-8")
    _git(d, "add", EVENTS_REL)
    _git(d, "-c", "user.name=fixture", "-c", "user.email=f@local", "commit", "-qm",
         "fixture: pointer")


def _run_trigger_in(d: Path, kind: str = "rebase", stdin: str = "") -> subprocess.CompletedProcess:
    """在夹具仓 `d` 里跑触发器（`SOKO_QUEUE_ROOT` 指到夹具自己 ⇒ 零真台账写入 ✓）。"""
    env = dict(clean_git_env(), SOKO_QUEUE_ROOT=str(d))
    env.pop("SOKO_SKIP_HOOK", None)
    return subprocess.run([sys.executable, str(HERE / "check-queue-rewrite-trigger.py"),
                           "--from-hook", kind], cwd=str(d), capture_output=True, text=True,
                          input=stdin, env=env)


def cmd_selftest() -> int:
    arms: list[tuple[str, bool, str]] = []

    def expect(what: str, ok: bool) -> None:
        arms.append((what, ok, ""))

    with tempfile.TemporaryDirectory(prefix="mb1-rewrite-") as tmp:
        # ── 臂 ①：孪生**已在 main 上** ⇒ 必须**真回填**（改后的那条读数 ✓）
        d1 = Path(tmp) / "a"
        d1.mkdir()
        orig, twin, unrelated = _fixture_repo(d1, "FIX-A")
        _write_pointer(d1, "FIX-A", orig)
        before = (d1 / EVENTS_REL).read_text(encoding="utf-8")
        r = _run_trigger_in(d1, stdin=f"{orig} {twin}\n")
        after = (d1 / EVENTS_REL).read_text(encoding="utf-8")
        expect(f"①a 触发器 rc=0（**永不阻塞 rebase**）：实测 {r.returncode}", r.returncode == 0)
        expect(f"①b 事件表**真的被回填**（{len(before.splitlines())} → "
               f"{len(after.splitlines())} 行）", len(after.splitlines()) > len(before.splitlines()))
        expect("①c 回填到的是 **main 上的孪生**", f'"commit": "{twin}"' in after
               or f'"commit":"{twin}"' in after)
        log = _git(d1, "log", "--oneline", "-1").stdout
        expect(f"①d 回填**已提交**（不提交 = 本地绿、CI 红 ✗）：{log.strip()[:60]}",
               "post-rewrite" in log)
        # ── 臂 ②（**正对照**）：合法但还没推的本地提交 ⇒ **不许误判/误改** ✗✗
        d2 = Path(tmp) / "b"
        d2.mkdir()
        orig2, twin2, unrelated2 = _fixture_repo(d2, "FIX-B")
        _write_pointer(d2, "FIX-B", unrelated2)
        before2 = (d2 / EVENTS_REL).read_text(encoding="utf-8")
        r2 = _run_trigger_in(d2, stdin=f"{orig2} {twin2}\n")
        after2 = (d2 / EVENTS_REL).read_text(encoding="utf-8")
        expect(f"②a 正对照：事件表**一字节未动**（合法本地提交不许被误改）",
               before2 == after2)
        expect("②b 而且**响亮地说出来**（不许静默 ✗）", "没回填到 0" in (r2.stdout + r2.stderr))
        # ── 臂 ③：**空转闸** —— 指针已是祖先 ⇒ 0 条、零写入（触发器不许空转写账 ✗）
        d3 = Path(tmp) / "c"
        d3.mkdir()
        orig3, twin3, unrelated3 = _fixture_repo(d3, "FIX-C")
        _write_pointer(d3, "FIX-C", twin3)
        before3 = (d3 / EVENTS_REL).read_text(encoding="utf-8")
        r3 = _run_trigger_in(d3, stdin=f"{orig3} {twin3}\n")
        expect("③a 已是祖先 ⇒ 触发器报 0 条、零写入",
               before3 == (d3 / EVENTS_REL).read_text(encoding="utf-8")
               and "0 条" in (r3.stdout + r3.stderr))
        # ── 臂 ④：逃生门 + 非 rebase 不动作（口径**只在 rebase** ✓）
        r4 = _run_trigger_in(d1, kind="amend", stdin=f"{orig} {twin}\n")
        expect("④a `kind=amend` ⇒ 不动作（如实说，不假装覆盖 ✗）",
               r4.returncode == 0 and "只覆盖 `git rebase`" in r4.stdout)
        env = dict(os.environ, SOKO_SKIP_HOOK="1", SOKO_QUEUE_ROOT=str(d1))
        r5 = subprocess.run([sys.executable, str(HERE / "check-queue-rewrite-trigger.py"),
                             "--from-hook", "rebase"], cwd=str(d1), capture_output=True,
                            text=True, input=f"{orig} {twin}\n", env=env)
        expect("④b `SOKO_SKIP_HOOK=1` 逃生门生效", r5.returncode == 0 and "SOKO_SKIP_HOOK" in r5.stdout)
        # ── 臂 ⑤：**`GIT_DIR` 污染**（本文件落地时实测抓到的**真** bug 的咬住证明 ✓）——
        #   `git rebase` 在**链接工作树**里会把 `GIT_DIR` 交给 `post-rewrite` 钩子；
        #   触发器必须摘掉它，否则 `_is_linked_worktree(<主 checkout>)` 会返回 True ⇒
        #   **在最需要它的那一刻空转**（实测：打出「队列单点是链接工作树」而它明明是主 checkout ✗）。
        #   ⚠ 这里的 `GIT_DIR` **不是新造的坏输入**：它就是 **git 自己的行为**（见
        #   `clean_git_env()` 的实测读数）⇒ 把那条真实污染原样搬进夹具 ✓。
        d5 = Path(tmp) / "e"
        d5.mkdir()
        orig5, twin5, _u5 = _fixture_repo(d5, "FIX-E")
        _write_pointer(d5, "FIX-E", orig5)
        wtp = Path(tmp) / "e-wt"
        _git(d5, "worktree", "add", "-q", str(wtp), "-b", "wip")
        polluted = str(d5 / ".git" / "worktrees" / "e-wt")
        env5 = dict(os.environ, SOKO_QUEUE_ROOT=str(d5), GIT_DIR=polluted)
        env5.pop("SOKO_SKIP_HOOK", None)
        r5 = subprocess.run([sys.executable, str(HERE / "check-queue-rewrite-trigger.py"),
                             "--from-hook", "rebase"], cwd=str(d5), capture_output=True,
                            text=True, input=f"{orig5} {twin5}\n", env=env5)
        out5 = r5.stdout + r5.stderr
        after5 = (d5 / EVENTS_REL).read_text(encoding="utf-8")
        expect(f"⑤ `GIT_DIR={polluted[-28:]}` 被摘掉 ⇒ 不空转、照常回填",
               "链接工作树" not in out5 and f'"commit": "{twin5}"' in after5)
        expect("⑤b 而且回填被**提交**（同 ①d 的口径 ✓）",
               "post-rewrite" in _git(d5, "log", "--oneline", "-1").stdout)

    for what, ok, _ in arms:
        print(f"  {'✓' if ok else '✗'} {what}")
    red = [w for w, ok, _ in arms if not ok]
    if red:
        print(f"✗ 自检判红 **{len(red)}/{len(arms)}** 条：{' · '.join(red)}", file=sys.stderr)
        return 1
    print(f"✓ 自检全过：**{len(arms)} 条臂 · 0 条红**")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="手动 rebase 之后的收口指针回填触发器（判据 + 触发器）")
    ap.add_argument("--from-hook", metavar="KIND", help="由 scripts/githooks/post-rewrite 调")
    ap.add_argument("--check", action="store_true", help="判据：触发器接线齐（绿）")
    ap.add_argument("--bite", action="store_true", help="咬住证明：固定旧 rev 上必须报红")
    ap.add_argument("--selftest", action="store_true", help="行为自检（夹具三臂 + 逃生门）")
    args = ap.parse_args()
    if args.from_hook is not None:
        return trigger(args.from_hook)
    if args.check:
        return cmd_check()
    if args.bite:
        return cmd_bite()
    if args.selftest:
        return cmd_selftest()
    ap.print_help(sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
