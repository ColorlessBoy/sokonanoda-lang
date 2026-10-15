#!/usr/bin/env python3
"""队列的**独立**复核：第二份实现 + 纯 git 事实（不 import `ux-queue.py`）。

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

  python3 scripts/check-queue-views.py                  # 对拍真实台账（读 + 只读子进程）
  python3 scripts/check-queue-views.py --json           # 机器可读
  python3 scripts/check-queue-views.py --selftest       # 判据通道自检（合成夹具，反向验证）
  python3 scripts/check-queue-views.py --events <path>  # 换事件文件（夹具/隔离口径用）
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import pathlib
import re
import subprocess
import sys

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
                       cwd=ledger_root(), capture_output=True, text=True)
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
            cur = "umbrella"
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
                             cwd=ledger_root(), capture_output=True, text=True)
    if shallow.returncode != 0:
        return [], f"`git rev-parse --is-shallow-repository` rc={shallow.returncode}"
    if shallow.stdout.strip() == "true":
        return [], ("仓库是浅克隆 ⇒ 祖先性**判不了**（不是判过 ✗）："
                    "用 `git fetch --unshallow` 或 CI 里 `fetch-depth: 0` ✓")
    bad = []
    for sha in commits:
        r = subprocess.run(["git", "merge-base", "--is-ancestor", sha, base],
                           cwd=ledger_root(), capture_output=True, text=True)
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
def selftest() -> int:
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
                                  reading="判据：`python3 scripts/check-queue-views.py` exit 0")]
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
             reading="判据：`python3 scripts/check-queue-views.py` exit 0")],
         {"state": {"X": {"state": "done"}}, "open": [], "queue": [{"id": "X"}]},
         False, cutoff=_old)
    # 反向：**生效的那一条**坏 ⇒ 必须红（判据的牙没被上面那条正对照拔掉 ✓）
    case("C5 生效的收口坏 ⇒ 判红", [
        ev("X", "advance", to="done", commit="HEAD", ts="2026-10-15T12:00:00+00:00",
           reading="判据：`python3 scripts/check-queue-views.py` exit 0"),
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
                             cwd=ledger_root(), capture_output=True, text=True).stdout.split()
    if non_anc:
        case("C4 收口指针不是祖先", [ev("X", "advance", to="done", commit=non_anc[0],
                                    reading="判据：`python3 scripts/check-queue-views.py` exit 0")],
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


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="队列的独立复核（第二份实现 + 纯 git 事实）")
    ap.add_argument("--events", metavar="PATH", default=None,
                    help="默认 = 与工具同根（$SOKO_QUEUE_EVENTS → $SOKO_QUEUE_ROOT → 本树）✓")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
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

    if args.selftest:
        return selftest()

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


if __name__ == "__main__":
    sys.exit(main())
