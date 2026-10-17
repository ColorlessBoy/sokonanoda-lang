#!/usr/bin/env python3
"""**队列的完整性与收口指针的完整性**：一本登记表 + 一个驱动器（`SUB-narrow-guards-to-one-layer` ④ ✓）。

## 这一轮做了什么（**合并**，不是新写 ✗）

用户 2026-10-16 09:5x：「**之前加的很多层也可以清理，收窄成一层**」⇒ 条目 ④ 的原话：
「**【④ 队列自身】B 组里 2 → 1**（`queue-views` + `queue-rewrite-trigger`）⇒
`check-queue-integrity.py`」✓。合并前是**两个 B 类守卫**（同一个关注点：**队列台账**，
同一个锚 `A-LEDGER-TRUTH`，同一个 CI job `queue-lint`，同一份输入 `docs/ux-queue-events.jsonl`）：

    check-queue-views.py            696 行   → **层 ①（复核层）**
    check-queue-rewrite-trigger.py  530 行   → **层 ②（触发器层）**
    ────────────────────────────────────────
    本条                            一本登记表 + 一个驱动器

⇒ `check-guard-bite.py --counts` 的 **B 类 21 → 20**（上限 `cap` **一个字没动** ✗ ——
只有「实际条数 > cap」判红 ✓）；`docs/guard-anchors.jsonl` / `docs/guard-bite.jsonl`
**每闸一行**同步成一行 ✓。

## 两层各是什么（**合并 ≠ 混在一起** ✗）

| 层 | 域 | 判据 | 输入 |
|---|---|---|---|
| **① 复核层** | **读**（复核队列台账，只读） | R-C1…R-C6 | `docs/ux-queue-events.jsonl` + 纯 git 事实 + 工具的自报（当**被告**） |
| **② 触发器层** | **写**（`post-rewrite` 钩子：手动 rebase 后回填收口指针） | T-HOOK / T-CHECK / T-BITE / T-SELFTEST | git 钩子的 `<old> <new>` 对照表 + 工具 |

两层的**接缝**正是同一个关注点：① 的 **R-C4** 判「收口指针必须是 `HEAD` 的祖先」，
② 在**手动 rebase 把分支提交重写成新 sha 之后**把那些指针回填到孪生 ✓
（`LAND-REBASE-STALES-CLOSURE-SHAS` 的两端）。

## 判据登记表（**声明 → 断言**；驱动器在文件尾）

`REQUIRED_ASSERTIONS`（冻结期望 10 条）⇄ `CRITERIA`（登记表）——**分开写** ⇒
从登记表里删掉一条 ⇒ `--selftest` 报出**缺的那条** ✓（验收 ③ 的原话）。

## 用法（**合并后一份** ✓）

    python3 scripts/check-queue-integrity.py                  # 层①：对拍真实台账（默认）
    python3 scripts/check-queue-integrity.py --json           # 层①：机器可读
    python3 scripts/check-queue-integrity.py --events <path>  # 层①：换事件文件（夹具/隔离口径用）
    python3 scripts/check-queue-integrity.py --skip-tool      # 层①：只跑不依赖被复核工具的判据
    python3 scripts/check-queue-integrity.py --check          # 层②：接线齐（绿的那条）
    python3 scripts/check-queue-integrity.py --bite           # 层②：咬住证明（固定旧 rev 上必须报红）
    python3 scripts/check-queue-integrity.py --selftest       # **两套自检 + 登记表对账**（正反两向）
    python3 scripts/check-queue-integrity.py --from-hook rebase   # 层②：由 `scripts/githooks/post-rewrite` 调

退出码：0 = 绿 / 1 = 判红 / 2 = 用法错 / 3 = 环境错（**不假装判过** ✗）。

## 一处定义（`AGENTS.md` 硬规则 0(b)）

* **读哪一层**（`live_lines()` / `run_bodies()`）在 `scripts/threshold_ratchet.py`（第 1 轮 `gs1` 收窄 ✓）；
* **干净 git 环境**在 `scripts/git_env.py`（合并顺带把触发器那份**本地副本**删掉 ⇒
  那个模块抬头写的"已改为委托本模块"从这句话变成**真的** ✓）；
* **队列单点 / 数据根**的解析只有一处（层② 的 `queue_root()`，与 `ux-queue.py` 同链 ✓）。

---

# ══════════════════════════════════════════════════════════════════════════════
# 层 ① —— 复核层（**读**）：独立复核（第二份实现 + 纯 git 事实）
#    ⚠ 以下原文来自 `scripts/check-queue-views.py`（合并前 696 行）——
#      逐字保留（除：入口改名 `main` → `main_review`、自检改名 `selftest` → `selftest_views`、
#      `--selftest` 上移到合并后的分派器、以及把自己点自己的命令名改成合并后的路径 ✓）。
# ══════════════════════════════════════════════════════════════════════════════

队列的**独立**复核：第二份实现 + 纯 git 事实（不 import `ux-queue.py`）。

## 为什么要有这个文件（2026-10-15 · `SUB-orchestrator-attention-structure` 【1】）

病根不是"某个判据写错了"，是**判据的所有权**：

    `scripts/ux-queue.py` 同时定义了 ① 队列语义 ② 守卫 ③ "证据"规则 ④ **复核判据**
    ⇒ **工具错 = 账错 = 过程错**，而**没有任何外部检查**能发现它
    ⇒ 只能在"第 N 次复核自己"时才发现 ✗（本会话：它被改了 54 次、自报 16 轮复核）。

⇒ 本文件打破这个闭环，靠**三条结构性手段**（不是"再写一条它自己的判据"）：

  ① **第二份实现**（差分测试）：本文件**自己**把原始事件流折一遍（下面的 `fold_raw`），
     **不 import、不 exec、不读** `ux-queue.py` 的任何内部函数或常量 ⇒
     两份独立实现折出**同一个**状态才算过 ✓。工具自报的结论只是**被告**，不是**判据**。
  ② **原始读数才是真相**：只读 `docs/ux-queue-events.jsonl`（原始事件）、
     `git merge-base --is-ancestor`（纯 git 事实）、`docs/ONBOARDING.md`（另一个产物）。
     **不读**任何 `.cache/**` 的结论文件、**不读** `ux-queue.py` 的源码常量。
  ③ **外部性自证**：`check_externality()` 扫本文件自己的源码 —— 出现 `import ux_queue`
     之类的字样就判红 ✓（这份判据**可以被改**，但它改的是"复核者自己"，与
     "被复核的工具自己定义复核判据"是两件事 ✓；且它与 `--selftest` 一起进 CI ✓）。

## 判据（每条都独立于被复核工具的自我判断）

  C1 **原始层卫生**：事件流逐行是 JSON 对象、`id`/`event`/`ts` 齐、`ts` 是 ISO-8601。
     工具自己有 `load_events()` 判红，但那是**它的**判据；这里独立复算一遍 ✓。
  C2 **独立折叠对拍**：`fold_raw()` 折出的每 id 状态 vs `ux-queue.py --json` 的 `state`
     —— **逐 id 比对**，任何分歧判红（含"工具多出/少了 id"）✓。
  C3 **两层不漂移（独立口径）**：`--json.open`（**来源处置**那一层）与 `fold_raw`
     （**事件**那一层）的终态划分必须一致：`open` 里的 id 不许独立折成终态、
     `state` 里的终态 id 不许出现在 `open` 里 ✓。
  C4 **指针祖先性（纯 git 事实）**：每条收口事件（`advance --to done` /
     `triage --to applied`）的**最终指针**必须满足
     `git merge-base --is-ancestor <c> HEAD` —— **不经工具**、用 git 自己的退出码 ✓
     （浅克隆 ⇒ rc=2 + 人话，不假装判过）。
     ⚠ **"最终"是必须的**：`repoint` 只改指针（rebase 会重写 sha）⇒ 拿**原始** `advance`
     的 commit 去判会造**大批假红**（实测：197 条收口里 100+ 条报非祖先，而折完 `repoint`
     后 **0 条** ✗→✓）。这条口径与工具 `fold()` 的 `cur["commit"]` **同语义、不同实现** ✓。
  C5 **收口证据必须可独立复算**（**前瞻判据 + 向后豁免** ✓）：
     `reading` 要么带一条**可重跑的命令**（反引号/围栏里的 `python3 …` / `scripts/…` /
     `cargo …` 之类），要么指向**仓库内的路径**（打开就能看 ✓）。
     ⚠ **只指向 `.cache/**`（git 忽略）的不算** —— 复核者拿不到那份文件 ✗，
     于是只能读**工具自报的结论**，正是【2】说的那种"自证" ✗。
     ⚠ 判据只对 **`ts >= EVIDENCE_GRANDFATHER_UNTIL`** 的收口生效（**新收口**）；
     切点之前的**只报数**（事件流 append-only ⇒ 存量**改不了** ⇒ 判它是一条**没有出路**
     的红 ✗，而且**任何别的 agent 写一条收口都会把它顶破** ⇒ 会变成**假红生成器** ✗
     —— 第一版"冻结上限 ratchet"实测就是这样在落地当刻判红的 ✗）。
     实测（2026-10-15，切点时刻）：**179/199 = 90% 的收口证据复核者复算不了**；
     其中 34 条只指向 `.cache/**`；近 33 条新收口里 27 条引 `.cache/**`（**在扩散** ✗）。
  C6 **外部性自证**：本文件不许出现对被复核工具的 import/exec 依赖（见 ③）。

## 退出码

  0 = 全过 / 1 = 判红 / 2 = 用法或环境错（含"跑不起来 `ux-queue.py`"—— **绝不静默跳过** ✗）。

## 用法

  python3 scripts/check-queue-integrity.py                  # 对拍真实台账（读 + 只读子进程）
  python3 scripts/check-queue-integrity.py --json           # 机器可读
  python3 scripts/check-queue-integrity.py --selftest       # 判据通道自检（合成夹具，反向验证）
  python3 scripts/check-queue-integrity.py --events <path>  # 换事件文件（夹具/隔离口径用）


# ══════════════════════════════════════════════════════════════════════════════
# 层 ② —— 触发器层（**写**）：手动 `git rebase` 之后的收口指针回填触发器
#    ⚠ 以下原文来自 `scripts/check-queue-rewrite-trigger.py`（合并前 530 行）——
#      逐字保留（除：本地 `clean_git_env()` 删掉改用 `git_env` 那一处、
#      `_load_uxq()` 删掉改成下面的夹具副本（**C6 的代价**，理由见夹具抬头 ✓）、
#      入口改名 `main` → `main_trigger` / 自检改名 `cmd_selftest` → `selftest_trigger`、
#      以及钩子转发点/判据里点自己的名字 ✓）。
# ══════════════════════════════════════════════════════════════════════════════

**手动 `git rebase` 之后**的收口指针回填**触发器**（`LAND-REBASE-STALES-CLOSURE-SHAS` 的第三条路）。

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

  python3 scripts/check-queue-integrity.py --check       # 绿：接线齐 + 咬住口径没空转
  python3 scripts/check-queue-integrity.py --bite        # 红：固定旧 rev 上没有触发器
  python3 scripts/check-queue-integrity.py --selftest    # **两套自检 + 登记表对账**（合并后唯一入口 ✓）

退出码：0 = 绿 / 1 = 判红 / 2 = 用法错 / 3 = 环境错（**不假装判过** ✗）。
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

ROOT = pathlib.Path(__file__).resolve().parent.parent
QUEUE_TOOL = ROOT / "scripts" / "ux-queue.py"


def events_path() -> pathlib.Path:
    """事件文件的**唯一出处** —— 必须与被复核工具**读同一棵树** ✓。

    ⚠ 这是**实测踩到的坑**（2026-10-15）：第一版固定读 `<本文件所在树>/docs/…`，
    而在**链接工作树**里跑时 `SOKO_QUEUE_ROOT` 指向**主 checkout** ⇒
    工具读**主 checkout 的来源**、我读**工作树的过期事件副本** ⇒ **两个输入不同树**
    ⇒ **C2/C3 一批假红** ✗（实测：`SUB-toolchain-guard-code-fence-false-positive`
    报"独立折出 in_progress · 工具自报 done"）。
    ⇒ 口径与 `ux-queue.py:events_path_for()` **同一处**：`SOKO_QUEUE_EVENTS` 优先，
    否则 `SOKO_QUEUE_ROOT/docs/…`，否则本树 ✓（**同根** ⇒ 两个输入可比 ✓）。
    """
    env = os.environ.get("SOKO_QUEUE_EVENTS")
    if env:
        return pathlib.Path(env)
    return ledger_root() / "docs" / "ux-queue-events.jsonl"


def ledger_root() -> pathlib.Path:
    """这本账**属于哪棵树**（= `SOKO_QUEUE_ROOT`，否则本树）✓。"""
    root = os.environ.get("SOKO_QUEUE_ROOT")
    return pathlib.Path(root) if root else ROOT


def base_ref() -> str:
    """C4 的祖先性基点 = **账所在那棵树的 HEAD**（不是"我当前 checkout 的 HEAD" ✗）。

    ⚠ 同族第二个实测坑（2026-10-15）：在**链接工作树**里跑、账指向主 checkout 时，
    本树的 HEAD 是**另一条分支**（还常被 rebase 过）⇒ 拿它当基点会把 main 上的
    **正常**指针报成非祖先 ⇒ **假红** ✗（实测：`5a3fc924` 与 `d06f5092` 两条
    —— 它们都是 main 的提交，只是不在我这个 worktree 分支的历史里）。
    ⇒ 基点**从账所在那棵树推导**（detached 时 `--abbrev-ref` 返回 `HEAD` ⇒ 原行为 ✓）。
    """
    override = os.environ.get("SOKO_QUEUE_BASE_REF")
    if override:
        return override
    r = subprocess.run(["git", "rev-parse", "--abbrev-ref", "HEAD"],
                       cwd=ledger_root(), capture_output=True, text=True,
                       env=git_env.clean_git_env())
    ref = r.stdout.strip() if r.returncode == 0 else ""
    return ref or "HEAD"

# ── 常量：**本文件自己的一份**（刻意不从被复核工具 import ✗） ────────────────
# 值取自 `ux-queue.py` 的**规格**（docstring + 常量），由本文件的 `--selftest` 与 C2
# 的对拍保证"两份实现折出同一个状态"；**一旦工具改语义，C2 会当场判红** ✓
# —— 这正是差分测试要的：分歧必须被看见，而不是被某一方静默吸收 ✗。
TERMINAL_STATES = frozenset({"done", "wo_filed"})
KNOWN_EVENT_KINDS = frozenset({
    # `fold` 分派的九族 + `--release` 的五族（= 工具 `FOLD_EVENT_KINDS` +
    # `RELEASE_EVENT_KINDS` 的**同语义副本**；改语义 ⇒ C1 会当场判红 ✓）
    "claim", "release", "advance", "repoint", "split", "reopen", "triage",
    "submit", "schedule",
    "release-open", "release-add", "release-drop", "release-block", "release-cut",
})
# `advance.to` 的合法值域（= 工具 `SCHED_STATES`）。**非法值不判红、只如实折**
# （工具那边由 `check_plan` 判红）—— 本文件的职责是"折得和它一样"，不是"再判一次" ✓。
SCHED_STATES = frozenset({"todo", "in_progress", "blocked", "done", "wo_filed",
                          "umbrella", "submitted", "triage_rejected"})

# ⭐ C5 是**前瞻**判据 + **向后豁免**（2026-10-15 第二版，见下面那条"为什么要改"）：
#   · `ts >= EVIDENCE_GRANDFATHER_UNTIL` 的收口 ⇒ **必须可独立复算**，否则**判红** ✓
#   · 切点之前的收口 ⇒ **只报数**（事件流 append-only ⇒ 存量**改不了** ⇒ 判它是一条
#     **没有出路**的红 ✗）
EVIDENCE_GRANDFATHER_UNTIL = datetime.datetime.fromisoformat("2026-10-15T18:30:00+08:00")
# 切点那一刻的**存量读数**（只报不判 ✓；供下一棒看"这个病有多大"）。**它不是判据** ✓。
EVIDENCE_HISTORICAL_UNREPRODUCIBLE = 179
# ⚠ **为什么第一版（"不可复算条数 ≤ 冻结上限 177"的 ratchet）必须改** —— 实测，不是设想：
#   第一版在**落地那一刻就判红**了 ✗，因为它是**移动靶**：`main` 在我切出 commit **之后**
#   又落了 2 条收口（含**本任务自己的收口** `SUB-orchestrator-attention-structure`）
#   ⇒ 177 → **179** ⇒ 红。⇒ 那条红**没有出路**（历史改不了），而且**任何别的 agent
#   写一条收口都会把它顶破** ⇒ 它会把 CI 变成一台**假红生成器** ✗。
#   ⇒ 改成"**向前判、向后豁免**"：新收口必须可复算（**咬得住** ✓），存量只报数 ✓。

# C5 认什么算"可重跑的命令"：反引号/围栏包起来的一小段，里面出现仓库里真有的入口。
_CMD_HINT = re.compile(
    r"(?:`{1,3}[^`\n]*(?:python3|scripts/[\w./-]+\.(?:py|sh|mjs)|cargo\s+\w|npm\s+\w"
    r"|node\s+\S+\.(?:js|mjs)|git\s+\w|bash\s+\S+|\./[\w./-]+\.sh)[^`\n]*`{1,3})")
# C5 的另一条出路：证据指向**仓库里的路径**（复核者能直接打开）✓。
_REPO_PATH = re.compile(r"(?:docs|scripts|crates|courses|editor|skills|\.github)/[\w./-]+\.\w+")
# C5 的"只指向忽略路径"判据：`.cache/**` 不在 git 里 ⇒ 复核者拿不到 ⇒ 不算证据。
_IGNORED_EVIDENCE = re.compile(r"\.cache/")


def _iso_ok(value) -> bool:
    if not isinstance(value, str) or not value:
        return False
    try:
        datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))
        return True
    except ValueError:
        return False


def load_raw(path: pathlib.Path) -> tuple[list, list]:
    """读**原始**事件流 ⇒ (事件列表, 问题列表)。**不 import 工具** ✓。"""
    bad: list = []
    if not path.exists():
        return [], [f"事件文件不存在：{path}"]
    events: list = []
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError as e:
            bad.append(f"{path.name}:{lineno} 不是合法 JSON（{e.msg}）")
            continue
        if not isinstance(rec, dict):
            bad.append(f"{path.name}:{lineno} 不是 JSON 对象（{type(rec).__name__}）")
            continue
        ident, kind = rec.get("id"), rec.get("event")
        if not isinstance(ident, str) or not ident:
            bad.append(f"{path.name}:{lineno} `id` 缺失或不是非空字符串")
        if not isinstance(kind, str) or not kind:
            bad.append(f"{path.name}:{lineno} `event` 缺失或不是非空字符串")
        elif kind not in KNOWN_EVENT_KINDS:
            bad.append(f"{path.name}:{lineno} 认不出的 `event`：{kind!r}")
        if not _iso_ok(rec.get("ts")):
            bad.append(f"{path.name}:{lineno} `ts` 缺失或不是 ISO-8601：{rec.get('ts')!r}")
        events.append(rec)
    return events, bad


def fold_raw(events: list) -> dict[str, str]:
    """**第二份实现**：原始事件流 → 每 id 状态。

    规格来自队列的**事件语义**（不是抄实现）：默认 `todo`；`claim` ⇒ `in_progress`；
    `release` 只把 `in_progress` 退回 `todo`；`advance` ⇒ `to`；`repoint` **只动指针、
    不动状态**；`split` ⇒ 父 `umbrella`；`reopen` ⇒ `todo`；`triage` 三态映射
    （applied ⇒ done · rejected ⇒ wo_filed · deferred ⇒ todo）；`submit` 只把 `todo`
    推成 `submitted`；`schedule` ⇒ `todo`。
    """
    st: dict[str, str] = {}
    for ev in events:
        ident, kind = ev.get("id"), ev.get("event")
        if not isinstance(ident, str) or not ident or not isinstance(kind, str):
            continue
        cur = st.setdefault(ident, "todo")
        if kind == "claim":
            cur = "in_progress"
        elif kind == "release":
            if cur == "in_progress":
                cur = "todo"
        elif kind == "advance":
            to = ev.get("to")
            # 非法值也照折（与工具一致：它折 `_to` 原值、另由 `check_plan` 判红）✓
            cur = to if isinstance(to, str) else cur
        elif kind == "repoint":
            pass                                   # 只改指针，状态一个字不动 ✓
        elif kind == "split":
            # 规格（本函数 docstring）：`split` ⇒ **父** `umbrella`；事件的 `id` 是**子** ✓
            # ⇒ 折到 `parent` 上，且 `cur` 属于 `ident`（子）这一轮，别误改它 ✓
            par = ev.get("parent")
            if isinstance(par, str) and par:
                st[par] = "umbrella"
            continue
        elif kind == "reopen":
            cur = "todo"
        elif kind == "triage":
            to = ev.get("to")
            if to == "applied":
                cur = "done"
            elif to == "rejected":
                cur = "wo_filed"
            elif to == "deferred":
                cur = "todo"
            # 表外值 ⇒ 什么都不折（与工具一致）✓
        elif kind == "submit":
            if cur == "todo":
                cur = "submitted"
        elif kind == "schedule":
            cur = "todo"
        st[ident] = cur
    return st


def closure_events(events: list) -> list[dict]:
    """**收口事件** = 会写指针的那些（`advance --to done` / `triage --to applied`）✓。

    ⚠ 这是**全部**收口事件（含**被后来覆盖的**）—— C4 的指针折法与"历史读数"用它 ✓；
    ⚠ **C5 不许用它** ✗：一条 `done` 被重开再收口之后，**旧的** `advance` 事件仍然在表里
    （事件流 append-only ✓），但它**已经不是这条目今天带着的证据**了 ⇒ 拿它判 = 判**历史快照**
    ✗。C5 的对象是 `effective_closures()` ✓。
    """
    out = []
    for ev in events:
        kind, to = ev.get("event"), ev.get("to")
        if kind == "advance" and to == "done":
            out.append(ev)
        elif kind == "triage" and to == "applied":
            out.append(ev)
    return out


def effective_closures(events: list, states: dict[str, str]) -> list[dict]:
    """**每 id 当前生效的那一条**收口事件（= 它今天带着的证据）✓ —— C5 的唯一对象。

    ⚠ **为什么必须有这一层**（2026-10-15 实测，**我的第一版在这里判了假红** ✗）：
    `SUB-soko-update-cli-404` 与 `SUB-no-version-cleanup-command` 被**合法重开再收口**
    （`--triage --to deferred` → `--advance --to in_progress` → `--advance --to done
    --reading '<可复算的>'`）⇒ 表里对每个 id 各有 **2 条** `advance --to done`：
      · 旧的（`reading` 只引 `.cache/**` ⇒ 不可复算）—— **已被覆盖** ✗
      · 新的（`reading` 带 `crates/cli/tests/launcher.rs` 等仓库路径 ⇒ 可复算 ✓）
    第一版把**两条都**判了 ⇒ 报"切点后 2 条不可复算" ⇒ **假红** ✗ ——
    而"重开就是为了改写证据"这件事**已经做对了**，判它等于**罚一次正确的操作** ✗。

    ⇒ 口径与仓库既有纪律**同源**：`ux-queue.py --check` 对**历史（被覆盖）指针**的做法是
    「**只报告、不判红** —— 它们已被后来的指针覆盖、**不再是任何条目的证据**」
    （实测读数：现行非祖先 **0** 条 · 历史被覆盖 **108** 条只报告 ✓）⇒ 这里照抄这个形状 ✓。

    生效口径（与 `fold()` 同语义）：**每个 id 取最后一条收口事件**，且该 id 的**最终状态**
    必须是终态（`TERMINAL_STATES`）—— 重开后还没收口的条目**没有**生效证据，不进判据 ✓。
    """
    last: dict[str, dict] = {}
    for ev in closure_events(events):
        ident = ev.get("id")
        if isinstance(ident, str) and ident:
            last[ident] = ev
    return [ev for ident, ev in last.items() if states.get(ident) in TERMINAL_STATES]


def fold_pointers(events: list) -> dict[str, str]:
    """**第二份实现**：折出每 id 的**最终收口指针**（`repoint` 会覆盖它）✓。

    这是 C4 的输入 —— 判据是"账上现在写的那个 sha 在不在 main 的历史里"，
    不是"当初写下的那个 sha"（rebase 会重写 sha，`repoint` 就是为此存在的 ✓）。
    """
    ptr: dict[str, str] = {}
    for ev in events:
        kind, to = ev.get("event"), ev.get("to")
        commit = ev.get("commit")
        if not isinstance(commit, str) or not commit:
            continue
        if kind == "advance" and to == "done":
            ptr[ev["id"]] = commit
        elif kind == "triage" and to == "applied":
            ptr[ev["id"]] = commit
        elif kind == "repoint":
            ptr[ev["id"]] = commit          # 只动指针 ✓
    return ptr


def ancestry_bad(commits: list[str], base: str = "HEAD") -> tuple[list[str], str | None]:
    """纯 git 事实：每个 sha 是不是 HEAD 的祖先。**不经工具** ✓。

    返回 (非祖先的 sha 列表, 环境错误说明)。浅克隆下 git 会答不了 ⇒ **不假装判过** ✗
    （rc=2 + 人话；照仓库先例：探不到就降级并出声 ✓）。
    """
    shallow = subprocess.run(["git", "rev-parse", "--is-shallow-repository"],
                             cwd=ledger_root(), capture_output=True, text=True,
                             env=git_env.clean_git_env())
    if shallow.returncode != 0:
        return [], f"`git rev-parse --is-shallow-repository` rc={shallow.returncode}"
    if shallow.stdout.strip() == "true":
        return [], ("仓库是浅克隆 ⇒ 祖先性**判不了**（不是判过 ✗）："
                    "用 `git fetch --unshallow` 或 CI 里 `fetch-depth: 0` ✓")
    bad = []
    for sha in commits:
        r = subprocess.run(["git", "merge-base", "--is-ancestor", sha, base],
                           cwd=ledger_root(), capture_output=True, text=True,
                           env=git_env.clean_git_env())
        if r.returncode == 1:
            bad.append(sha)
        elif r.returncode not in (0, 1):
            return [], f"`git merge-base --is-ancestor {sha} {base}` rc={r.returncode}"
    return bad, None


def check_externality() -> list[str]:
    """C6：本文件不许依赖被复核工具的内部（import / 读它的源码常量）✓。"""
    src = pathlib.Path(__file__).read_text(encoding="utf-8")
    bad = []
    for pat, why in (
        (r"^\s*(?:import|from)\s+ux_queue\b", "import 了 `ux_queue` 模块"),
        (r"^\s*(?:import|from)\s+ux[-_]queue\b", "import 了 `ux-queue` 模块"),
        (r"importlib[^\n]*ux[-_]queue", "用 importlib 加载了被复核工具"),
    ):
        if re.search(pat, src, re.M):
            bad.append(f"外部性破了：{why} ⇒ 复核者与被复核者不再是两份实现 ✗")
    return bad


def tool_json(env_extra: dict | None = None) -> tuple[dict | None, str | None]:
    """把被复核工具当**黑箱**跑一次（只读它的 stdout；不看它的源码/内部）✓。"""
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run([sys.executable, str(QUEUE_TOOL), "--json"],
                       cwd=ROOT, capture_output=True, text=True, env=env)
    if r.returncode != 0:
        return None, (f"`ux-queue.py --json` rc={r.returncode}："
                      f"{(r.stderr or r.stdout or '').strip()[:400]}")
    try:
        return json.loads(r.stdout), None
    except json.JSONDecodeError as e:
        return None, f"`ux-queue.py --json` 的输出不是 JSON（{e.msg}）"


def run(events_path: pathlib.Path, env_extra: dict | None = None,
        skip_tool: bool = False,
        cutoff: datetime.datetime | None = None) -> tuple[dict, list[str]]:
    """跑全部判据 ⇒ (读数 dict, 判红列表)。**纯函数**（除两个只读子进程）✓。"""
    red: list[str] = []
    cutoff = cutoff or EVIDENCE_GRANDFATHER_UNTIL
    events, hygiene = load_raw(events_path)
    red += [f"C1 {m}" for m in hygiene]

    st = fold_raw(events)
    terminal_ids = {i for i, s in st.items() if s in TERMINAL_STATES}
    closures = closure_events(events)

    reading: dict = {
        "events_file": str(events_path),
        "events": len(events),
        "ids": len(st),
        "terminal_ids": len(terminal_ids),
        "closures": len(closures),
        "fold_sha": None,
    }
    # 折叠结果的指纹：让"两次读数是不是同一份输入"自明（探针身份纪律 ✓）
    import hashlib
    reading["fold_sha"] = hashlib.sha256(
        json.dumps(sorted(st.items()), ensure_ascii=False).encode()).hexdigest()[:16]

    red += [f"C6 {m}" for m in check_externality()]

    if not skip_tool:
        view, err = tool_json(env_extra)
        if err:
            red.append(f"环境 C7 {err} ⇒ **不静默跳过**（判红，rc=1）✗")
        else:
            tstate = {k: (v or {}).get("state") for k, v in (view.get("state") or {}).items()}
            reading["tool_ids"] = len(tstate)
            # ── C2 独立折叠对拍 ──────────────────────────────────────────
            for ident in sorted(set(st) | set(tstate)):
                mine, theirs = st.get(ident), tstate.get(ident)
                if mine != theirs:
                    red.append(f"C2 分歧 `{ident}`：独立折出 {mine!r} · 工具自报 {theirs!r}")
            # ── C3 两层不漂移（独立口径）─────────────────────────────────
            # 两层：**来源处置**（`open` / `queue` 那份）· **事件折叠**（本文件的 `fold_raw`）。
            # 漂移**两个方向都要判** ✓（工具自己的 `check_plan` 只判它想到的那些格 ✗）：
            #   ① 来源说开着（在 `open` 里）而事件已终态 ⇒ 收口没落到来源行 ✗；
            #   ② 来源说已关账（在 `queue` 里、不在 `open` 里）而事件**非**终态
            #      ⇒ 来源行单方面宣布关账 ✗（这正是"改了来源没记事件"的形状）。
            # ⚠ 判据的域取 `queue`（**来源行**）而不是 `state`（事件 id）—— 事件 id 可以
            # 比来源行多（claim 先于来源行落地之类），拿它当域会造**假红** ✗。
            open_ids = set(view.get("open") or [])
            source_ids = {r.get("id") for r in (view.get("queue") or []) if r.get("id")}
            reading["tool_open"] = len(open_ids)
            reading["tool_sources"] = len(source_ids)
            for ident in sorted(open_ids & terminal_ids):
                red.append(f"C3 漂移 `{ident}`：来源层说它开着（`--json.open`），"
                           f"事件层独立折出终态 {st[ident]!r}")
            for ident in sorted(source_ids - open_ids):
                mine = st.get(ident)
                if mine is not None and mine not in TERMINAL_STATES:
                    red.append(f"C3 漂移 `{ident}`：来源层说它已关账，"
                               f"事件层独立折出非终态 {mine!r}")

    # ── C5 证据可独立复算（**前瞻 + 向后豁免**；不依赖被复核工具 ✓）────────
    # ⚠ 对象是 `effective`（每 id 当前生效的那一条），**不是** `closures`（全部，含被覆盖的）
    #   —— 见 `effective_closures()` 的说明（第一版在这里判了假红 ✗）。
    effective = effective_closures(events, st)
    superseded = len(closures) - len(effective)
    unrepro = [e for e in effective if not _reproducible(e)]
    fresh_bad = [e for e in unrepro if _since(e, cutoff)]
    reading["evidence_total"] = len(effective)
    reading["evidence_superseded"] = superseded
    reading["evidence_unreproducible"] = len(unrepro)
    reading["evidence_historical"] = EVIDENCE_HISTORICAL_UNREPRODUCIBLE
    reading["evidence_fresh_unreproducible"] = len(fresh_bad)
    reading["evidence_citing_cache"] = sum(1 for e in unrepro if _cites_cache(e))
    for ev in fresh_bad:
        red.append(
            f"C5 `{ev.get('id')}` 的收口证据**复核者复算不了** ⇒ 只能读**自报的结论** ✗"
            f"—— 把**判据命令**或**仓库内路径**写进 `--reading` ✓"
            f"（切点 {cutoff.isoformat()} 之后新写的收口一律判 ✓；切点前 {len(unrepro) - len(fresh_bad)} 条只报数）")

    # ── C4 祖先性（纯 git · **折过 `repoint` 的最终指针**）────────────────
    ptr = fold_pointers(events)
    reading["pointers"] = len(ptr)
    base = base_ref()
    reading["base_ref"] = base
    bad, env_err = ancestry_bad(sorted(set(ptr.values())), base)
    if env_err:
        red.append(f"环境 C4 {env_err}")
    else:
        badset = set(bad)
        for sha in bad:
            owners = sorted(i for i, c in ptr.items() if c == sha)
            red.append(f"C4 非祖先指针 `{sha}`（{' · '.join(owners)}）⇒ "
                       f"这个收口在 `{base}` 的历史里核不到 ✗")
        reading["non_ancestor_pointers"] = len(badset)
    return reading, red


def _since(ev: dict, when: datetime.datetime | None = None) -> bool:
    try:
        return datetime.datetime.fromisoformat(
            str(ev.get("ts", "")).replace("Z", "+00:00")) >= (when or EVIDENCE_GRANDFATHER_UNTIL)
    except ValueError:
        return False


def _has_cmd(ev: dict) -> bool:
    return bool(_CMD_HINT.search(ev.get("reading") or ""))


def _cites_cache(ev: dict) -> bool:
    return bool(_IGNORED_EVIDENCE.search(ev.get("reading") or ""))


def _reproducible(ev: dict) -> bool:
    """复核者能不能**自己复算**这条收口？两条出路（任一满足即可 ✓）：
    ① `reading` 里有**可重跑的命令**（反引号/围栏）；② 指向**仓库内的路径**（打开就能看）✓。
    ⚠ 只指向 `.cache/**`（git 忽略）的**不算** —— 复核者拿不到那份文件 ✗。
    """
    text = ev.get("reading") or ""
    if _has_cmd(ev) and (not _cites_cache(ev)
                         or any(".cache/" not in m for m in _CMD_HINT.findall(text))):
        return True
    return bool(_REPO_PATH.search(text))


# ── 判据通道自检（反向验证：故意造出该现象 ⇒ 必须判红）──────────────────────
def selftest_views() -> int:
    import tempfile
    bad = 0

    def case(name: str, events: list, view: dict | None, want_red: bool,
             expect: str = "", cutoff: datetime.datetime | None = None) -> None:
        nonlocal bad
        with tempfile.TemporaryDirectory() as td:
            p = pathlib.Path(td) / "events.jsonl"
            p.write_text("\n".join(json.dumps(e, ensure_ascii=False) for e in events) + "\n",
                         encoding="utf-8")
            if view is None:
                reading, red = run(p, skip_tool=True)
            else:
                # 夹具：**不跑真工具** ⇒ 直接喂一份"工具自报"，只跑 C1/C2/C3/C5
                reading, red = _run_with_view(p, view, cutoff=cutoff)
        hit = bool(red) == want_red
        mark = "✓" if hit else "✗"
        if not hit:
            bad += 1
        print(f"  {mark} {name}：判红 {len(red)} 条"
              + (f"（{red[0][:70]}）" if red else "")
              + ("" if hit else f"  ← 期望 {'红' if want_red else '绿'}"))
        if expect and not any(expect in r for r in red):
            bad += 1
            print(f"      ✗ 没有期望的那条判红（找 {expect!r}）")

    def ev(ident, kind, **kw):
        d = {"ts": "2026-10-15T12:00:00+00:00", "event": kind, "id": ident,
             "owner": "selftest"}
        d.update(kw)
        return d

    print("判据通道自检（合成夹具；每组都带**正对照** ✓）：")
    # 正对照：干净的流 + 一致的自报 ⇒ 必须**不判红**
    clean = [ev("X", "claim"), ev("X", "advance", to="done", commit="HEAD",
                                  reading="判据：`python3 scripts/check-queue-integrity.py` exit 0")]
    case("正对照（干净流 + 一致自报）", clean,
         {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]}, want_red=False)
    # C1：坏行
    case("C1 认不出的事件类型", [ev("X", "advnace", to="done")], None, True, "C1")
    case("C1 ts 不是 ISO-8601", [{"ts": "昨天", "event": "claim", "id": "X"}], None, True, "C1")
    case("C1 id 不是字符串", [{"ts": "2026-10-15T12:00:00+00:00", "event": "claim",
                            "id": 7}], None, True, "C1")
    # C2：第二份实现与自报分歧（**这就是差分测试要咬的形状**）
    case("C2 自报状态与独立折叠分歧", clean,
         {"state": {"X": {"state": "todo"}}, "open": ["X"], "queue": [{"id": "X"}]}, True, "C2")
    case("C2 工具多出一条 id", clean,
         {"state": {"X": {"state": "done"}, "Y": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]}, True, "C2")
    # C3：两层漂移
    case("C3 open 里有终态 id", clean,
         {"state": {"X": {"state": "done"}}, "open": ["X"], "queue": [{"id": "X"}]}, True, "C3")
    # C5：切点钉死在很久以前 ⇒ 夹具里的收口一律算"新写的" ⇒ 判据确定性 ✓
    _old = datetime.datetime(2020, 1, 1, tzinfo=datetime.timezone.utc)
    case("C5 新收口证据不可复算", [ev("X", "advance", to="done", commit="HEAD",
                                 reading="见 .cache/ux/fix-X/VERDICT.md")],
         {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
         True, "C5", cutoff=_old)
    case("C5 证据只指向 .cache 路径 ⇒ 仍不可复算", [ev("X", "advance", to="done", commit="HEAD",
                                            reading="`cat .cache/ux/fix-X/VERDICT.md`")],
         {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
         True, "C5", cutoff=_old)
    # ⭐ **被覆盖的坏收口 + 生效的好收口 ⇒ 不许判红**（2026-10-15 **真台账**踩到的形状 ✓）：
    #   一条 `done` 被合法重开再收口之后，**旧的**坏事件仍在表里（append-only）⇒ 拿它判
    #   就是判**历史快照** ✗ —— 而"重开改写证据"这件事**已经做对了**，判它 = 罚正确操作 ✗。
    case("C5 正对照：被覆盖的坏收口 + 生效的好收口 ⇒ 不红",
         [ev("X", "advance", to="done", commit="HEAD", ts="2026-10-15T12:00:00+00:00",
             reading="读数见 .cache/ux/fix-X/VERDICT.md"),
          ev("X", "triage", to="deferred", ts="2026-10-15T12:01:00+00:00", reason="重收口"),
          ev("X", "advance", to="in_progress", ts="2026-10-15T12:02:00+00:00"),
          ev("X", "advance", to="done", commit="HEAD", ts="2026-10-15T12:03:00+00:00",
             reading="判据：`python3 scripts/check-queue-integrity.py` exit 0")],
         {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
         False, cutoff=_old)
    # 反向：**生效的那一条**坏 ⇒ 必须红（判据的牙没被上面那条正对照拔掉 ✓）
    case("C5 生效的收口坏 ⇒ 判红", [
        ev("X", "advance", to="done", commit="HEAD", ts="2026-10-15T12:00:00+00:00",
           reading="判据：`python3 scripts/check-queue-integrity.py` exit 0"),
        ev("X", "triage", to="deferred", ts="2026-10-15T12:01:00+00:00", reason="重收口"),
        ev("X", "advance", to="in_progress", ts="2026-10-15T12:02:00+00:00"),
        ev("X", "advance", to="done", commit="HEAD", ts="2026-10-15T12:03:00+00:00",
           reading="已修（读数见 .cache/ux/fix-X/VERDICT.md）")],
        {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
        True, "C5", cutoff=_old)
    case("C5 正对照：证据给了仓库内路径 ⇒ 可复算", [ev("X", "advance", to="done", commit="HEAD",
                                          reading="读数见 docs/gaps/ledger.jsonl 的 G-1 行")],
         {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
         False, cutoff=_old)
    # C4：非祖先指针（**真 git 事实** —— 拿一个 HEAD 之外的 sha 当夹具）
    # ⚠ 夹具必须与判据**同一个基点**（实测坑：这里原写死 `HEAD` + `cwd=ROOT`，
    #    而判据用的是 `base_ref()` + `ledger_root()` ⇒ 设了 `SOKO_QUEUE_ROOT` 时
    #    两者不同 ⇒ **正对照自己判红** ✗）。
    non_anc = subprocess.run(["git", "rev-list", "--all", "--not", base_ref()],
                             cwd=ledger_root(), capture_output=True, text=True,
                             env=git_env.clean_git_env()).stdout.split()
    if non_anc:
        case("C4 收口指针不是祖先", [ev("X", "advance", to="done", commit=non_anc[0],
                                    reading="判据：`python3 scripts/check-queue-integrity.py` exit 0")],
             {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
             True, "C4")
    else:
        print("  ⚠ C4 夹具跳过：这个仓库里没有 HEAD 之外的提交（判据没被验到 ✗ —— 出声不静默）")
    # C6：外部性（把 import 注进一份临时副本 —— 判据本身必须咬得住）
    src = pathlib.Path(__file__).read_text(encoding="utf-8")
    if not re.search(r"^\s*import\s+ux_queue\b", src, re.M):
        print("  ✓ C6 外部性：当前源码没有 import 被复核工具")
    else:
        bad += 1
        print("  ✗ C6 外部性：当前源码 import 了被复核工具")
    # C4：环境（浅克隆 ⇒ 必须出声，不许假装判过）
    reading, red = run(pathlib.Path(__file__), skip_tool=True)   # 非事件文件 ⇒ C1 必红
    if not red:
        bad += 1
        print("  ✗ 正对照：拿非事件文件当输入竟然不判红")
    print(f"\n自检：{'✓ 全过' if bad == 0 else f'✗ {bad} 组不对'}（C1×3 · C2×2 · C3×1 · C4×1 · C5×3 · C6×1 + 正对照×3）")
    return 0 if bad == 0 else 1


def _run_with_view(events_path: pathlib.Path, view: dict,
                   cutoff: datetime.datetime | None = None) -> tuple[dict, list[str]]:
    """自检专用：用**喂进来的**自报代替真跑工具（夹具里没有真队列）✓。"""
    real_tool = globals()["tool_json"]
    globals()["tool_json"] = lambda env_extra=None: (view, None)
    try:
        return run(events_path, cutoff=cutoff)
    finally:
        globals()["tool_json"] = real_tool


def main_review(argv=None) -> int:
    ap = argparse.ArgumentParser(description="队列的独立复核（第二份实现 + 纯 git 事实）")
    ap.add_argument("--events", metavar="PATH", default=None,
                    help="默认 = 与工具同根（$SOKO_QUEUE_EVENTS → $SOKO_QUEUE_ROOT → 本树）✓")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--grandfather-until", metavar="ISO8601", default=None,
                    help="C5 的切点（默认 = EVIDENCE_GRANDFATHER_UNTIL）；往前挪 ⇒ "
                         "把**真台账上已写下的**不可复算收口也照出来（反向验证用 ✓）")
    ap.add_argument("--skip-tool", action="store_true",
                    help="只跑不依赖被复核工具的判据（C1/C4/C5/C6）")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    cutoff = None
    if args.grandfather_until:
        try:
            cutoff = datetime.datetime.fromisoformat(args.grandfather_until)
        except ValueError as e:
            print(f"error: `--grandfather-until` 不是 ISO-8601（{e}）", file=sys.stderr)
            return 2
    reading, red = run(pathlib.Path(args.events) if args.events else events_path(),
                       skip_tool=args.skip_tool, cutoff=cutoff)
    if args.json:
        print(json.dumps({"reading": reading, "red": red}, ensure_ascii=False, indent=2))
    else:
        print(f"独立复核：事件 {reading['events']} 条 · id {reading['ids']} 个 · "
              f"终态 {reading['terminal_ids']} · 收口 {reading['closures']} · "
              f"指针 {reading.get('pointers', 0)} · 基点 `{reading.get('base_ref', 'HEAD')}` · "
              f"折叠指纹 {reading['fold_sha']}")
        if "tool_open" in reading:
            print(f"  对拍：工具自报 state {reading['tool_ids']} 个 · open {reading['tool_open']} 个")
        if "evidence_total" in reading:
            bad_n = reading["evidence_unreproducible"]
            print(f"  收口证据可复算（**前瞻 + 向后豁免**）：对象 = **每 id 当前生效的那一条**"
                  f"（{reading['evidence_total']} 条 · 另有 **{reading['evidence_superseded']}** "
                  f"条被覆盖的**只报数不判** ✓ —— 照 `--check` 对历史指针的先例）· "
                  f"切点后新收口不可复算 "
                  f"{reading['evidence_fresh_unreproducible']} 条（>0 ⇒ 判红）· "
                  f"切点前存量 {bad_n - reading['evidence_fresh_unreproducible']}"
                  f"/{reading['evidence_total']}（**只报数**）· "
                  f"其中 {reading['evidence_citing_cache']} 条只引 `.cache/**`（git 忽略）")
        if red:
            print(f"\n✗ 独立复核 **{len(red)} 条红**：", file=sys.stderr)
            for m in red:
                print(f"  · {m}", file=sys.stderr)
            return 1
        print("✓ 独立复核通过：两份实现折出同一状态 · 两层不漂移 · 指针都是祖先 · "
              "外部性自证 ✓")
    return 0 if not red else 1

# ══════════════════════════════════════════════════════════════════════════════
# 层 ② 的**代码**（触发器层）—— 原文见文件抬头 §层 ②（合并前
# `scripts/check-queue-rewrite-trigger.py` 530 行 ✓）；下面先放它自检要用的夹具。
# ══════════════════════════════════════════════════════════════════════════════


# ── 夹具（**触发器自检**用；层① 的 C6 要求本文件**不许** import 被复核工具 ⇒
#    这里是 `ux-queue.py::_fixture_queue_root()` / `_fixture_commit()` 的**夹具副本** ✗）──
#
# ⭐ **为什么是副本而不是 `_load_uxq()`**（合并时必须做的那个选择，逐条读数见
#    `.cache/ux/fix-gs2/VERDICT.md` §4）：
#      · 层① 的 **C6 外部性自证**（`check_externality()`）把「本文件用 importlib 加载了
#        被复核工具」判**红**（实测：注入 `_load_uxq()` 那一行 ⇒ `--selftest` rc=1 +
#        「C6 外部性破了：用 importlib 加载了被复核工具」✓）；
#      · 而触发器自检**需要**一个"工具认得的队列根"当夹具。
#    ⇒ 两条路：① 把 C6 的域收窄到"复核层"（**放宽判据** ✗）；② 夹具自带（**C6 一个字不动** ✓）。
#    本轮选 **②**（派单硬约束：不许放宽任何判据）—— 代价如实写在这里：
#      ⚠ **副本会漂移**：`ux-queue.py` 的 `REQUIRED_ROOT_FILES`（5 份）变了 ⇒ 这里的副本
#        就不再是合法根 ⇒ **触发器自检臂会响亮失败**（`--repoint-all` rc=2 ⇒ 臂 ①a 判红 +
#        印出「队列根不对 / 缺必需文件」）⇒ 那时**同步本函数**（不是静默降级 ✓）。
#      ⚠ 副本**只服务自检**（临时目录）⇒ 不碰真仓、不参与任何判定路径 ✓。
def _fixture_root(path: Path, ident: str, *, facet: str = "主线", repro: str = "true",
                  why: str = "触发器自检夹具") -> Path:
    """造一个**自包含的队列根**（`ux-queue.py::_guard_root()` 的 5 份必需文件 + 一行 extras + `git init`）。

    ⚠ **`git init` 必须自带**（同工具的读数）：锁文件落在**共享 git 目录** ⇒ 没有 `.git`
    的夹具根会让子进程把锁写回**真仓库** ✗。
    """
    (path / "scripts").mkdir(parents=True, exist_ok=True)
    (path / "docs" / "gaps").mkdir(parents=True, exist_ok=True)
    _git(path, "init", "-q", str(path))
    (path / "Cargo.toml").write_text("", encoding="utf-8")
    (path / "scripts" / "ux-queue.py").write_text("# 根标记（`_guard_root` 的必需文件）\n",
                                                 encoding="utf-8")
    (path / "docs" / "gaps" / "ledger.jsonl").write_text("", encoding="utf-8")
    (path / "docs" / "ux-findings-2026-10-12.md").write_text(
        "## 2 高\n\n| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核 |\n"
        "|---|---|---|---|---|---|---|\n"
        f"| {ident} | {facet} | 夹具 | 中 | `{repro}` | fixtures | — |\n",
        encoding="utf-8")
    (path / "docs" / "ONBOARDING.md").write_text(
        "### 0.2 队列\n\n<!-- QUEUE:SUMMARY:BEGIN -->\n"
        "<!-- QUEUE:COUNTS total=0 blocker=0 high=0 medium=0 low=0 -->\n**占位**\n"
        "<!-- QUEUE:SUMMARY:END -->\n\n## 3 下节\n", encoding="utf-8")
    (path / "docs" / "ux-queue-extras.jsonl").write_text(json.dumps({
        "id": ident, "source": "orchestrator-schedule", "facet": facet,
        "severity": "medium", "repro": repro, "disposition": "fix",
        "owner": "fixtures", "why": why}, ensure_ascii=False) + "\n", encoding="utf-8")
    return path


def _fixture_commit(repo: Path, filename: str, content: str, subject: str) -> str:
    """往夹具仓 `repo` 里**追加一次提交** ⇒ 返回它的 sha（`HEAD`）。

    ⚠ 身份用 `-c` **显式给** ⇒ 不依赖 runner 上有没有 `user.name`
    （CI 的 `actions/checkout` 只配 `safe.directory` ✗ —— 同 `SELFTEST-FIXTURE-FAILS-IN-CI`）。
    """
    (repo / filename).write_text(content, encoding="utf-8")
    ident = ("-c", "user.name=ux-queue-selftest", "-c", "user.email=selftest@local")
    _git(repo, *ident, "add", filename)
    _git(repo, *ident, "commit", "-qm", subject)
    return _git(repo, "rev-parse", "HEAD").stdout.strip()

HERE = Path(__file__).resolve().parent
HOOK_REL = "scripts/githooks/post-rewrite"
EVENTS_REL = "docs/ux-queue-events.jsonl"

# ⭐ **钉死的旧 rev**（本改动落地前 main 的 HEAD ✓）——**不许写 `HEAD`** ✗：
# 写 `HEAD` ⇒ 提交之后 `HEAD` 就是修好的那版 ⇒ `--bite` 变成空转（`pr1` 2026-10-15 实测踩过 ✗）。
PINNED_PRE_TRIGGER_REV = "1942bd8a"
PINNED_WHY = ("本改动落地前 main 的 HEAD（`chore(queue): 把 AGENT-DOES-NOT-PRODUCE-USER-INPUTS "
              "的收口指针改到 rebase 后的孪生（64304006）`）—— 那份树里没有 "
              f"`{HOOK_REL}`，也没有本文件")


def _git(cwd: Path, *argv: str, stdin: str | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *argv], cwd=str(cwd), capture_output=True,
                          text=True, input=stdin, env=git_env.clean_git_env())


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
    # ⚠ **子进程也必须过 `git_env.clean_git_env()`** ✗：`GIT_DIR` 会让 `ux-queue.py` 的
    #   `ancestry_rc()`（`git merge-base --is-ancestor … HEAD`，`cwd=ROOT`）拿**工作树的 HEAD**
    #   当基点 ⇒ 审错对象（见 `git_env.clean_git_env()` 的实测）✗。
    env = dict(git_env.clean_git_env(), SOKO_QUEUE_ROOT=str(qroot))
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
    if "check-queue-integrity.py" not in body:
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
    print(f"✓ 触发器接线齐：`{HOOK_REL}`（可执行）⇒ `scripts/check-queue-integrity.py "
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
    print(f"  ⇒ 落地后的形状：`{HOOK_REL}` 转发到 `scripts/check-queue-integrity.py "
          f"--from-hook rebase`（`--check` 绿 ✓）")
    return 1


# ── 行为自检（三臂，夹具） ──────────────────────────────────────────────────
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
    **不**加 `-c`：`git_env.clean_git_env()` 的口径是"**提交身份应当来自仓库配置**，不该被钩子的
    调用方注入" ✓）。⇒ runner 上没有 `user.name`/`user.email`（`actions/checkout` 只配
    `safe.directory`）且 `git` 又**自动探测不到**（CI 上是 `runner@…(none)`）时，那一次
    `git commit` 直接失败 ⇒ 回填**只写工作区、没进 HEAD** ⇒ 臂 ①d/⑤b 判红：
    `✗ 自检判红 2/11 条：①d 回填**已提交**（不提交 = 本地绿、CI 红 ✗）：<sha> fixture: pointer · ⑤b …`
    （**本地绿、CI 红** 正是这条臂的注释点名的那档 ✗）。同族先例：
    `ux-queue.py::_fixture_commit` 的 docstring 记的 `SELFTEST-FIXTURE-FAILS-IN-CI`。
    ⇒ 夹具**自带**身份（仓库配置，不是环境变量 —— `git_env.clean_git_env()` 会把 `GIT_*` 全摘掉 ✗）。
    """
    _fixture_root(d, ident)
    # 夹具仓自带身份：被考的那次提交由**触发器**打（它只认仓库配置 ✓）。
    _git(d, "config", "user.name", "fixture")
    _git(d, "config", "user.email", "fixture@local")
    _git(d, "symbolic-ref", "HEAD", "refs/heads/main")
    _git(d, "add", "-A")
    _git(d, "-c", "user.name=fixture", "-c", "user.email=f@local", "commit", "-qm", "base")
    # 分支上的原提交
    _git(d, "checkout", "-q", "-b", "agent/x")
    orig = _fixture_commit(d, "work.txt", "same content\n", "fix: 同一个环节")
    # 合法但**没落 main** 的本地提交（正对照）
    unrelated = _fixture_commit(d, "local-only.txt", "not landed\n", "fix: 只在本地")
    # 回到 main：**先走一步**（parent 变了 ⇒ 孪生的 sha 与原提交不同 ✓），再落孪生。
    _git(d, "checkout", "-q", "main")
    _fixture_commit(d, "main-only.txt", "main moved on\n", "chore: 主线自己前进一步")
    twin = _fixture_commit(d, "work.txt", "same content\n", "fix: 同一个环节")
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
    env = dict(git_env.clean_git_env(), SOKO_QUEUE_ROOT=str(d))
    env.pop("SOKO_SKIP_HOOK", None)
    return subprocess.run([sys.executable, str(HERE / "check-queue-integrity.py"),
                           "--from-hook", kind], cwd=str(d), capture_output=True, text=True,
                          input=stdin, env=env)


def selftest_trigger() -> int:
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
        r5 = subprocess.run([sys.executable, str(HERE / "check-queue-integrity.py"),
                             "--from-hook", "rebase"], cwd=str(d1), capture_output=True,
                            text=True, input=f"{orig} {twin}\n", env=env)
        expect("④b `SOKO_SKIP_HOOK=1` 逃生门生效", r5.returncode == 0 and "SOKO_SKIP_HOOK" in r5.stdout)
        # ── 臂 ⑤：**`GIT_DIR` 污染**（本文件落地时实测抓到的**真** bug 的咬住证明 ✓）——
        #   `git rebase` 在**链接工作树**里会把 `GIT_DIR` 交给 `post-rewrite` 钩子；
        #   触发器必须摘掉它，否则 `_is_linked_worktree(<主 checkout>)` 会返回 True ⇒
        #   **在最需要它的那一刻空转**（实测：打出「队列单点是链接工作树」而它明明是主 checkout ✗）。
        #   ⚠ 这里的 `GIT_DIR` **不是新造的坏输入**：它就是 **git 自己的行为**（见
        #   `git_env.clean_git_env()` 的实测读数）⇒ 把那条真实污染原样搬进夹具 ✓。
        d5 = Path(tmp) / "e"
        d5.mkdir()
        orig5, twin5, _u5 = _fixture_repo(d5, "FIX-E")
        _write_pointer(d5, "FIX-E", orig5)
        wtp = Path(tmp) / "e-wt"
        _git(d5, "worktree", "add", "-q", str(wtp), "-b", "wip")
        polluted = str(d5 / ".git" / "worktrees" / "e-wt")
        env5 = dict(os.environ, SOKO_QUEUE_ROOT=str(d5), GIT_DIR=polluted)
        env5.pop("SOKO_SKIP_HOOK", None)
        r5 = subprocess.run([sys.executable, str(HERE / "check-queue-integrity.py"),
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


def main_trigger(argv=None) -> int:
    ap = argparse.ArgumentParser(description="手动 rebase 之后的收口指针回填触发器（判据 + 触发器）")
    ap.add_argument("--from-hook", metavar="KIND", help="由 scripts/githooks/post-rewrite 调")
    ap.add_argument("--check", action="store_true", help="判据：触发器接线齐（绿）")
    ap.add_argument("--bite", action="store_true", help="咬住证明：固定旧 rev 上必须报红")
    ap.add_argument("--selftest", action="store_true", help="行为自检（夹具三臂 + 逃生门）")
    args = ap.parse_args(argv)
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

# ══════════════════════════════════════════════════════════════════════════════
# 判据登记表（**声明 → 断言**）+ 驱动器（`SUB-narrow-guards-to-one-layer` ④ ✓）
# ══════════════════════════════════════════════════════════════════════════════
#
# 合并前是**两个文件**（`check-queue-views.py` 696 行 · `check-queue-rewrite-trigger.py`
# 530 行），各有自己的判据；合并后**一条判据都不能少** ✗ ⇒ 这张表把"少了哪条"变成
# **可机械判**的（验收 ①/③ 的原话：「合并前每条判据都要能在合并后的登记表里找到对应断言，
# 逐条对账，缺一条 ⇒ 判红」/「从登记表里故意删掉一条断言 ⇒ 驱动器必须报出缺的那条」✓）。
#
# ⚠ **期望（`REQUIRED_ASSERTIONS`）与登记表（`CRITERIA`）分开写** ⇒ 删掉登记表里的一条
#   ⇒ 驱动器报**缺的那条** ✓；两条一起删 = 两次编辑（评审看得见 ✓）。⚠ 如实划的边界 ✗：
#   这不是"外部锚"（同一份文件里的两个常量）—— 真正的外部记录是 `docs/guard-bite.jsonl`
#   那一行（它写的是**坏输入**与**命令**，不是断言清单 ✓）。
REQUIRED_ASSERTIONS: tuple[str, ...] = (
    "R-C1", "R-C2", "R-C3", "R-C4", "R-C5", "R-C6",
    "T-HOOK", "T-CHECK", "T-BITE", "T-SELFTEST",
)

# (id, 层, 一句话（判什么）, 判据入口（本文件里的函数名）)
CRITERIA: tuple[tuple[str, str, str, str], ...] = (
    ("R-C1", "复核层", "原始层卫生：逐行 JSON 对象 · id/event/ts 齐 · ts 是 ISO-8601", "run"),
    ("R-C2", "复核层", "独立折叠对拍：第二份实现 vs 工具自报 state（逐 id 分歧判红）", "run"),
    ("R-C3", "复核层", "两层不漂移：来源处置层 ⇄ 事件折叠层（两个方向都判）", "run"),
    ("R-C4", "复核层", "指针祖先性：折过 repoint 的最终指针（纯 git merge-base）", "run"),
    ("R-C5", "复核层", "收口证据可独立复算（前瞻 + 向后豁免 ratchet）", "run"),
    ("R-C6", "复核层", "外部性自证：不许 import / importlib 加载被复核工具", "check_externality"),
    ("T-HOOK", "触发器层", "触发器本体：手动 rebase 后回填收口指针（永不阻塞 rebase）", "trigger"),
    ("T-CHECK", "触发器层", "接线齐：钩子在 / 可执行 / 转发本文件 + 咬住口径没空转", "cmd_check"),
    ("T-BITE", "触发器层", "咬住证明：钉死旧 rev 上没有触发器 ⇒ 判红", "cmd_bite"),
    ("T-SELFTEST", "触发器层", "行为三臂（回填 / 不误伤 / 空转闸）+ 逃生门 + GIT_DIR 污染",
     "selftest_trigger"),
)


def registry_problems() -> list[str]:
    """登记表 ⇄ 冻结期望 + 每条断言的**判据入口真的存在**（**唯一一处** ✓）。"""
    red: list[str] = []
    ids = [c[0] for c in CRITERIA]
    dup = sorted({i for i in ids if ids.count(i) > 1})
    if dup:
        red.append(f"**断言重复登记** ✗：{dup}")
    have = set(ids)
    for want in REQUIRED_ASSERTIONS:
        if want not in have:
            red.append(f"**断言缺登记** ✗：`{want}` 在冻结期望里、却**不在**登记表里 ⇒ "
                       f"合并/收窄时把它删掉了（**合并后功能不许减** ✗）⇒ 补回登记表那一行")
    for cid in sorted(have - set(REQUIRED_ASSERTIONS)):
        red.append(f"**断言未登记进期望** ✗：登记表里的 `{cid}` 不在 `REQUIRED_ASSERTIONS` 里 "
                   f"⇒ 加断言要**两处一起加**（评审可见 ✓）")
    for cid, _layer, _what, entry in CRITERIA:
        if not callable(globals().get(entry)):
            red.append(f"**断言没牙** ✗：`{cid}` 的判据入口 `{entry}()` 不存在/不可调用")
    return red


def selftest_all() -> int:
    """**两套自检 + 登记表对账**（`--selftest` 的唯一入口 ✓）。"""
    print(f"判据登记表（声明 → 断言）：冻结期望 **{len(REQUIRED_ASSERTIONS)}** 条 · "
          f"登记 **{len(CRITERIA)}** 条")
    for cid, layer, what, entry in CRITERIA:
        print(f"  · {cid:<11} [{layer}] {what}  ← {entry}()")
    red = registry_problems()
    for m in red:
        print(f"✗ {m}", file=sys.stderr)
    print()
    rc_views = selftest_views()
    print()
    rc_trigger = selftest_trigger()
    if red or rc_views or rc_trigger:
        print(f"✗ 合并后的自检判红：登记表 {len(red)} 条 · 复核层 rc={rc_views} · "
              f"触发器层 rc={rc_trigger}", file=sys.stderr)
        return 1
    print("✓ 合并后的自检全过：登记表齐（10/10）+ 复核层 + 触发器层 ✓")
    return 0


def main(argv=None) -> int:
    """**分派器**（合并后唯一的入口 ✓）：触发器三个模式 → 触发器层；`--selftest` → 两套都跑；
    其余（默认 = 复核）→ 复核层。"""
    argv = list(sys.argv[1:] if argv is None else argv)
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass
    if any(a in ("--from-hook", "--check", "--bite") for a in argv):
        return main_trigger(argv)
    if "--selftest" in argv:
        return selftest_all()
    return main_review(argv)


if __name__ == "__main__":
    sys.exit(main())
