#!/usr/bin/env python3
"""ux-queue.py —— 「没做完的」**唯一机器可重算队列**（2026-10-12 清台账）。

## 它解决的是什么

仓库里的「还没做完」原本散在**七本台账**里（探针索引 / 缺口台账 / ROADMAP / 协议档的
Known limitation / CI 台账 / 代码里的 TODO / 开工单 §0.2），彼此不引用 ⇒
「登记 ≠ 排期、勘明 ≠ 排期，中间没人接」反复发生（`docs/ONBOARDING.md` §0.2 的
#4/#5/#6 三条用户报障就是这样掉线的）。本脚本把七处**收成一个可重算的集合**，
并长出两颗牙：

* `--check-uncovered` —— **覆盖自检**：任何来源里存在、而队列不知道的条目 ⇒ 判红 ✗；
  任何 `scheduled` 条目没进 `docs/ONBOARDING.md` §0.2 表 ⇒ 判红 ✗；
  §0.2 表里有、队列没有 ⇒ 判红 ✗（**双向**，防两头漂移）。
* `--check-uncovered --live` —— 再加上**读数自检**：逐条跑探针，实读与「台账写的处置」
  不符 ⇒ 判红 ✗（例：写着 `fix` 的条目探针已经 exit 1 ⇒ 该改成 `fixed` 并记账）。

## 来源（四个，逐条都有 id ⇒ 可对账）

| # | 来源 | 它是什么 | 谁在维护 |
|---|---|---|---|
| ① | `docs/gaps/ledger.jsonl` 里 `status != fixed` | 历史缺口台账（126 条） | `scripts/gap.py` |
| ② | `docs/ux-findings-2026-10-12.md` §2–§4 的**每一行** | 四条探针线（语言/LSP/编辑器/vim）的索引 | 探针入库时 |
| ③ | `docs/ux-queue-extras.jsonl` | 其余来源（ROADMAP §10 / `docs/protocol.md` 的 Known limitation / `docs/CI-FAILURES.md` / 代码里的 `TODO`/`FIXME` / `docs/design/*` 的"未做/第二刀"措辞） | 扫描时逐条登记 |
| ④ | `docs/ONBOARDING.md` §0.2 的表 | **唯一入口**（权威）；只消费、不生成 | 开工单 |

## 处置（`disposition`，三选一 + 两类"不算缺口"）

* `fix` —— **本轮修**（还没修完；探针必须仍报"缺口仍在"）
* `fixed` —— 已修（探针必须已转"已变 / 一致"）
* `scheduled` —— **排期**：`docs/ONBOARDING.md` §0.2 里必须有它的一行 ✗ 缺一行即判红
* `wo-filed` —— **终态**：定案不做，理由写在它的来源台账里
* `boundary` —— **边界 / 设计如此**（例：`docs/protocol.md` 的 Known limitation）**不算缺口**
* `control` —— 对照臂 / 探针自检（例：LSP 的 `A2`/`D4`/`G2`/`N3`）**不算缺口**

**队列的 open 集合** = `disposition ∈ {fix, scheduled}` ——
`scheduled` 的行**逐条**落在 §0.2 里，`fix` 的行是"本轮要做、还没做完"。
⚠ `fixed` / `wo-filed` / `boundary` / `control` **不进 open 集合**，但它们**留在队列里**
（= 会计上的"已关账"，**不许从队列里删除** ✗ —— 删条目 = 把守卫关掉 ✗）。

## 用法

```bash
python3 scripts/ux-queue.py                       # 人看的队列表 + 计数
python3 scripts/ux-queue.py --json                # 机器可读（与 §0.2 对账用）
python3 scripts/ux-queue.py --schedule <id> --title '<做什么>' --reason '<为什么/下一轮的口径>'
python3 scripts/ux-queue.py --check-uncovered     # 覆盖自检（源码级，秒级；进 gate --fast）
python3 scripts/ux-queue.py --check-uncovered --live   # 覆盖 + 读数自检（跑探针，慢）
python3 scripts/ux-queue.py --selftest            # 判据通道自检（不碰仓库状态）
```

退出码：0 = 通过 · 1 = 有红（覆盖/读数/§0.2 不一致）· 2 = 用法或环境错误。

口径与边界：**本文件 docstring 就是规格**（不另开设计档 ⇒ 免得"文档说的"与"脚本做的"两头漂）；
门禁：`scripts/soko gate --fast` 与 CI 的 `ledger` job 都跑 `--check`（秒级）⇒ 队列与来源脱钩当场判红 ✗。
调度器（2026-10-12 用户：「不能全依赖大模型运作，而是同时依靠脚本」）：`--next` 取序 · `--claim` 认领
（写锁，别人持锁判红）· `--advance … --to done --commit … --reading …` 收口（**没证据不许收口**）·
`--add-child` 就地拆解（子任务插在父之前、父转 umbrella）· **`--schedule <id> --title … --reason …`**
登记**编排者（主进程）自己排的期** · 状态 = `docs/ux-queue-events.jsonl` 的**只追加**事件折叠。

**`--schedule` 补的是什么**（2026-10-13 · `SUB-queue-registration-gaps` ①，**真实咬到过**）：
上一轮明确判定「下一轮要做（有预算空间时）把 L0 减到 1533 以下」，但 `--add-child` 是给**缺口**用的
（得先有父条目）、`--submit` 是给**外部 agent**用的（它进的是"等主流受理"的提交面）⇒ 编排者给自己
排的期**没有入口**，只能写进 commit message ⇒ `--json` 的 `open` 里查不到 ⇒ **下一个 agent 不会知道** ✗。
⇒ `--schedule` 与 `--submit` **同形**（来源行 + 一条只追加事件），但语义是"**我自己排的期**"：
处置 `fix`、状态 `todo` ⇒ `--json` 的 `open`/`queue`/`view` 查得到、`--next` 按序推得出，
收口与其它条目**同口径**（`--commit` + `--reading`，**没有豁免** ✗）。

**收口指针必须过祖先性闸门**（2026-10-14 · `queueW` 修三个视图一致性缺陷 ⑤）：`--commit` 不只看
"对象存在"（`git cat-file -e`），还看 `git merge-base --is-ancestor <sha> HEAD` —— 实测 65 条 `done`
里 **60 条**的指针是 **rebase 前的孪生**（对象在、内容也落了 main，但**解析不到 main 的历史**）✗。
口径 **只对新增收口**要求祖先性（写入口硬拒 + `check_plan` 复查"先写后 rebase"），**存量**给一条
**带日期**的豁免（`ANCESTRY_GATE_SINCE`），且每次 `--check` 都打一行 ⚠ 汇总（条数 + 日期 + 为什么 +
清单）—— **不许静默放过** ✗。

**三个视图必须同数**（2026-10-14 · 同轮 ①②③）：`--json` 的 `open`/`summary`、`--next` 的推序、
§0.2 的 `QUEUE:COUNTS` **一律从 `sched_rows()` 的那份视图算**（`json_view` 不再自己从源行另算 ✗）；
子任务**有源行就用源行**（真处置），`check_plan` 的处置以**来源台账**为权威 ⇒「反向两层漂移」
对子任务也咬得住 ✓。

**来源可见性**（2026-10-14 · `queueX` 缺陷 ⑥）：上面那条漂移规则要的是**来源的终态**，
而三个来源加载器是**为队列视图服务的** ⇒ 会把终态行丢掉：`load_gaps()` 对
`status=="fixed"` 直接 `continue` ✗ ⇒ 「来源终态 `fixed` + 调度器非终态」这条
**对缺口来源永远咬不住**（实测：`G-110` gap `fixed` + 事件 `in_progress` ⇒ `--check`
静默假绿 ✗；`G-112` gap `fixed` + 事件 `todo` 同形）。⇒ 口径：**漂移规则的权威表
必须从全量来源建**（`source_dispositions()` —— 不过滤终态，**三个来源都读**：
extras 全行 · gap 全 `status`（含 `fixed`，走 `load_gaps(include_terminal=True)`）·
索引只给身份、处置仍取 extras），且三个来源**逐条**在 `--selftest`
（`selftest_source_visibility`）里断言「终态看得见」✓ —— 将来加第四个来源就照那份清单
补一行，**不许只补一处** ✗（AGENTS.md 硬规则 0(b)）。

**来源 × 调度器状态的网格**（2026-10-14 · 第三轮复核 ⑤）：光"每个来源都看得见"还不够 ——
规则里**再写死一个状态豁免**（实测：`and st != "submitted"`）就能让整整**一列 6 格静默**
（而 `--next` 照样推得出来），且旧夹具只造 todo/in_progress 两种状态 ⇒ 漏格活到自检全绿 ✗。
⇒ 判据是**两维网格**：3 来源 × 2 来源终态 × 6 调度器状态 = 36 格，逐格断「判红点名 +
收口指示**逐步可执行**」（`selftest_source_state_grid`）。**加来源 / 加状态只改那一处**
（`grid_pairs`/`grid_states`），指示本身由 `closure_steps()` 按 `TRANSITIONS` 算出
⇒ 表里没有的路**不会被印成指示** ✓。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# **队列状态单点**（2026-10-12 并行 agent 基建）：4 棵 worktree 会各存一份 `docs/ux-queue-events.jsonl`
# ✗ ⇒ worktree 里的 agent 用 `SOKO_QUEUE_ROOT=<主 checkout>` 把状态（与来源台账）都指回主那份 ✓；
# 锁本来就在**共享 git 目录**里（`--git-common-dir`）⇒ 4 棵树天然共用一把锁 ✓。
ROOT = Path(os.environ.get("SOKO_QUEUE_ROOT") or Path(__file__).resolve().parent.parent).resolve()
GAPS_LEDGER = ROOT / "docs" / "gaps" / "ledger.jsonl"
INDEX = ROOT / "docs" / "ux-findings-2026-10-12.md"
EXTRAS = ROOT / "docs" / "ux-queue-extras.jsonl"
ONBOARDING = ROOT / "docs" / "ONBOARDING.md"

# 面 → 探针跑法（`facet` 是队列里的一列）
FACETS = {"语言", "LSP", "编辑器", "vim", "主会话", "语言·LSP", "tooling", "docs", "课程",
          "prelude", "cli+docs", "lsp+docs"}

# 严重度归一（索引里是中文，台账里是英文；队列统一成 4 档）
SEVERITY_ALIASES = {
    "blocker": "blocker", "高·blocker": "blocker", "高·blocker ": "blocker",
    "高": "high", "major": "high", "painful": "high",
    "中": "medium", "低→中": "medium", "低-中": "medium", "minor": "medium",
    "低": "low", "nice": "low",
}
SEVERITY_RANK = {"blocker": 0, "high": 1, "medium": 2, "low": 3}
DISPOSITIONS = {"fix", "fixed", "scheduled", "wo-filed", "boundary", "control"}
OPEN_DISPOSITIONS = {"fix", "scheduled"}


# §0.2 队列行的 id 形状：字母开头、带 `-` 或 `_`（`Q-…` / `G-110` / `r07` / `A3` / `HANDOFF-…`）
QUEUE_ID_RE = re.compile(r"^[A-Za-z][A-Za-z0-9]*[-_][A-Za-z0-9_.\-]+$")


def _guard_root() -> None:
    """`SOKO_QUEUE_ROOT` 指错时要**干净报错**（别丢栈 ✗）。"""
    if not (ROOT / "scripts" / "ux-queue.py").exists() or not (ROOT / "Cargo.toml").exists():
        raise UsageError(
            f"队列根不对：`{ROOT}` 里没有 scripts/ux-queue.py + Cargo.toml ⇒ "
            f"`SOKO_QUEUE_ROOT` 要指向**主 checkout**（并行 worktree 的队列单点）✗")
    # **worktree 里必须显式指回主 checkout** ✗ —— 否则 4 棵树各读自己那份副本 ⇒ 队列悄悄分叉
    # （"登记 ≠ 排期"的同类病：看不见的漂移）。锁本就在共享 git 目录里，状态也必须单点 ✓。
    def _git(*argv: str) -> str:
        return subprocess.run(["git", *argv], cwd=ROOT, capture_output=True,
                              text=True).stdout.strip()
    gd, gcd = _git("rev-parse", "--git-dir"), _git("rev-parse", "--git-common-dir")
    if gd and gcd and Path(gd).resolve() != Path(gcd).resolve() and not os.environ.get("SOKO_QUEUE_ROOT"):
        # `--git-dir` 是相对路径时按 ROOT 解析（`git rev-parse` 的输出可能是相对的 ✓）
        linked = (Path(gd) if Path(gd).is_absolute() else ROOT / gd).resolve()
        common = (Path(gcd) if Path(gcd).is_absolute() else ROOT / gcd).resolve()
        if linked != common:
            raise UsageError(
                f"你在**链接工作树**里（{ROOT}）但没设 `SOKO_QUEUE_ROOT` ⇒ 会读写这一份**本地副本**，"
                f"4 棵树就各有一本账 ✗。正解：`export SOKO_QUEUE_ROOT=<主 checkout>`"
                f"（`scripts/agent-worktree.sh new <id>` 会把该 export 打出来 ✓）")


class UsageError(Exception):
    """用法/环境错误（exit 2）——与"判红"（exit 1）分开，别把环境问题当结论。"""


# ── 来源 ①：缺口台账 ────────────────────────────────────────────────────────

def load_gaps(include_terminal: bool = False) -> list[dict]:
    """`docs/gaps/ledger.jsonl` 的行 —— 默认**只给 `status != fixed`**（= 队列视图口径）。

    ⚠ **`include_terminal=True` = 全量口径**（`status=="fixed"` 也算）—— **只有
    `check_plan` 的处置权威表用**（`source_dispositions()`）：`fixed` 的缺口已经
    **离开 open 视图**，但它的**终态**正是「反向两层漂移」要看的输入 ✗ ——
    默认口径把它 `continue` 掉 ⇒ 规则对**缺口来源永远咬不住**（2026-10-14 实测：
    `G-110` gap `fixed` + 事件 `in_progress` ⇒ `--check` 静默假绿 ✗）。
    两个口径的**处置**都不许改（`fixed` ⇒ `"fixed"`，与 extras 的字段同名同义 ✓）。
    """
    rows: list[dict] = []
    for lineno, line in enumerate(GAPS_LEDGER.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as err:
            raise UsageError(f"{GAPS_LEDGER}:{lineno} 不是合法 JSON：{err}") from err
        if entry.get("status") == "fixed" and not include_terminal:
            continue
        repro = entry.get("repro") or ""
        # wo-filed 是**终态**（理由在台账的 `wo`/`notes` 里）⇒ 队列里保留一行、不算 open ✓；
        # fixed 只在**全量口径**下走到这里 ⇒ 终态 `"fixed"`（队列视图里根本到不了这一支 ✓）。
        disposition = ("fixed" if entry.get("status") == "fixed"
                       else "wo-filed" if entry.get("status") == "wo-filed" else "scheduled")
        # ⚠ `where` 的历史形状**不齐**：**5 条**旧行是**字符串**（`L-07`…`L-10`/`G-86`，
        # 全在 `fixed` 档；`G-77` 的 `where` 是 `None` ⇒ 走"没有位置线索"那一支，
        # 2026-10-14 第三轮复核实测更正 —— 旧注释把 `G-77` 也算进「字符串」= 6 条 ✗），
        # ⇒ 全量口径第一次读到它们时 `.get` 一个字符串 ⇒ **丢栈** ✗
        # （对 open 行也是同一个潜在崩溃；`gap.py:wo_prompt` 已按同一形状补了守卫 ✓）。
        # 只认 dict，其余当没有位置线索 ✓。
        where = entry.get("where")
        where_area = where.get("area", "") if isinstance(where, dict) else ""
        rows.append({
            "id": entry["id"],
            "sources": ["gap-ledger"],
            "facet": (entry.get("kind") or "tooling"),
            "severity": SEVERITY_ALIASES.get(entry.get("severity", ""), "medium"),
            "repro": f"bash {repro}" if repro.endswith(".sh") else (
                f"scripts/soko grade {repro}" if repro else "（无复现件）"),
            "disposition": disposition,
            "owner": entry.get("owner") or where_area or "",
            "note": (entry.get("wo") or entry.get("wo_planned") or "").split("/")[-1][:40],
            "expect": None,
        })
    return rows


# ── 来源 ②：UX 索引（`docs/ux-findings-2026-10-12.md` §2–§4） ────────────────

def _cells(line: str) -> list[str]:
    return [c.strip() for c in line.strip().strip("|").split("|")]


def load_index() -> list[dict]:
    """解析索引的三张表（**id/面/严重度/复现命令/归属** = `reference`；处置**不在这里**）。

    列是契约：`| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核 |`（**七列**）。
    处置在 `docs/ux-queue-extras.jsonl`（来源 ③）—— 索引是给人看的一行一条，
    处置是机器账 ⇒ 分开存，两边由 `--check-uncovered` 对账（**双向**）。

    ⚠ **可见性契约**（见 docstring「来源可见性」）：索引这一支提供的是**身份**
    （`disposition=None` ⇒ 由 extras 填，而 `load_extras()` 不过滤终态 ⇒ 漂移规则
    照样看得见终态 ✓）；**id 登记过**这一点由 `source_history_ids()` 读本函数保证。
    ⚠ **§2–§4 里的行不许静默丢**（`AGENTS.md` 硬规则 0(b)）：以前 `len(cells) != 7`
    直接 `continue` ⇒ 少/多一列的行**整条消失**，队列与漂移规则都看不见它 ✗ ——
    现在：**长得像数据行**（首格是 id 形状）却不是七列 ⇒ 响亮报错（点名行号 + 列数）。
    """
    rows: list[dict] = []
    section = None
    for lineno, line in enumerate(INDEX.read_text(encoding="utf-8").splitlines(), 1):
        head = re.match(r"^## (\d)", line)
        if head:
            section = head.group(1)
        if not line.startswith("|"):
            continue
        cells = _cells(line)
        ident0 = cells[0] if cells else ""
        if (len(cells) != 7 and section in {"2", "3", "4"}
                and QUEUE_ID_RE.fullmatch(ident0)):
            raise UsageError(
                f"{INDEX}:{lineno}：§{section} 的数据行 `{ident0}` 有 {len(cells)} 列（契约是**七列**）"
                f" —— 别让它**静默消失**（少一列 = 这一条连队列都进不去）✗")
        if len(cells) != 7:
            continue
        ident, facet, _sym, sev, repro, owner, _obs = cells
        if ident == "id" or ident.startswith("---"):
            continue
        if section not in {"2", "3", "4"}:
            raise UsageError(
                f"{INDEX}：「{ident}」出现在 §{section}（只认 §2–§4）—— 索引的分节是队列的一部分，"
                f"挪动行 = 改严重度口径 ✗（要排除就标 boundary/control，别搬节 ✗）")
        rows.append({
            "id": ident,
            "sources": ["ux-index"],
            "facet": facet,
            "severity": SEVERITY_ALIASES.get(sev, "medium"),
            "repro": repro.strip("`").split("（")[0].strip(),
            "disposition": None,          # ← 由 extras 填；填不上 ⇒ 覆盖自检判红 ✗
            "owner": owner,
            "note": "",
            "expect": None,
        })
    if not rows:
        raise UsageError(f"{INDEX}：一条都没解析出来 —— 表结构变了？列顺序是契约（见本文件 docstring）")
    return rows


# ── 来源 ③：其它来源（`docs/ux-queue-extras.jsonl`） ────────────────────────

def load_extras() -> list[dict]:
    """`docs/ux-queue-extras.jsonl` 的**全部**行（处置的唯一出处）。

    ⚠ **可见性契约**（见 docstring「来源可见性」）：本加载器**不按处置过滤** ——
    `fixed`/`wo-filed`（终态）与 `fix`/`scheduled`（open）一视同仁地返回 ⇒
    漂移规则的权威表可以直接用它 ✓（`selftest_source_visibility` 逐来源断言这一点）。
    """
    if not EXTRAS.exists():
        return []
    rows: list[dict] = []
    for lineno, line in enumerate(EXTRAS.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as err:
            raise UsageError(f"{EXTRAS}:{lineno} 不是合法 JSON：{err}") from err
        missing = [k for k in ("id", "source", "severity", "disposition", "why") if k not in entry]
        if missing:
            raise UsageError(f"{EXTRAS}:{lineno} 缺字段 {missing}（协议见 docs/ux-queue-extras.jsonl 抬头）")
        if entry["disposition"] not in DISPOSITIONS:
            raise UsageError(f"{EXTRAS}:{lineno} 处置 `{entry['disposition']}` 非法")
        if entry["severity"] not in SEVERITY_RANK:
            raise UsageError(f"{EXTRAS}:{lineno} 严重度 `{entry['severity']}` 非法")
        rows.append({
            "id": entry["id"],
            "sources": ["extras", entry["source"]],
            "facet": entry.get("facet") or "docs",
            "severity": entry["severity"],
            "repro": entry.get("repro") or "（无复现件：判据见 `where`）",
            "disposition": entry["disposition"],
            "owner": entry.get("owner") or "",
            "note": entry["why"][:80],
            # ⚠ `why` **原文**要留着：`--inbox` 要打印"为什么"，而 `note` 是截断的（[80]）
            # ⇒ 只留 note 会让回执少半句（实测 `--inbox` 恒答空，见 `cmd_inbox`）。
            "why": entry["why"],
            "where": entry.get("where") or "",
            "expect": entry.get("expect"),
        })
    return rows


# ── 来源 ④：`docs/ONBOARDING.md` §0.2 的表（**唯一入口**，只消费） ────────────

ONB_BEGIN = "<!-- QUEUE:SUMMARY:BEGIN -->"
ONB_END = "<!-- QUEUE:SUMMARY:END -->"
ONB_COUNTS_RE = re.compile(r"<!--\s*QUEUE:COUNTS\s+([^>]*?)-->")


def _onboarding_section() -> list[str]:
    """`### 0.2 …`（**任意标题级别**）到下一个顶层 `## ` 之间的行。

    2026-10-12 修正：原实现只认 `^## 0.2 `，而真文件是 `### 0.2` ⇒ **从来没读到过** ✗
    （双向对账因此一直空转 —— "咬不住的守卫等于没有"）。
    """
    lines = ONBOARDING.read_text(encoding="utf-8").splitlines()
    out: list[str] = []
    inside = False
    for line in lines:
        if re.match(r"^#{2,4} 0\.2 ", line):
            inside = True
            continue
        if inside and re.match(r"^## [0-9]", line):
            break
        if inside:
            out.append(line)
    return out


def load_onboarding_queue() -> list[str]:
    """§0.2 里**人维护的粗粒度线**（`Q-*`）的 id。

    口径（2026-10-12 修订）：§0.2 不再逐条列 open —— 40+ 条明细塞不进有行数上限的必读
    文档 ✗，而且"登记 ≠ 排期"的病根不在文档而在没有执行器。现在：
      ① §0.2 = 一行**脚本渲染的摘要**（标记块，`--render` 写；计数不符即判红）+ 粗粒度的线；
      ② 细粒度队列的**唯一机器入口 = `python3 scripts/ux-queue.py --next/--json`** ✓。
    """
    ids: list[str] = []
    for line in _onboarding_section():
        if line.startswith("|"):
            cells = _cells(line)
            if cells and QUEUE_ID_RE.fullmatch(cells[0]):
                ids.append(cells[0])
    dupes = {i for i in ids if ids.count(i) > 1}
    if dupes:
        raise UsageError(f"{ONBOARDING} §0.2 里 id 重复：{sorted(dupes)}")
    return ids


def load_onboarding_counts() -> dict[str, int] | None:
    """§0.2 渲染摘要里的机器可读计数（没有标记块 ⇒ None ⇒ 判红，提示跑 `--render`）。"""
    for line in _onboarding_section():
        m = ONB_COUNTS_RE.search(line)
        if m:
            out: dict[str, int] = {}
            for tok in m.group(1).split():
                if "=" in tok:
                    key, _, val = tok.partition("=")
                    try:
                        out[key.strip()] = int(val)
                    except ValueError:
                        raise UsageError(f"§0.2 的 QUEUE:COUNTS 里有非整数：`{tok}`") from None
            return out
    return None


def render_onboarding(rows: list[dict]) -> str:
    """把 §0.2 的摘要块渲染成文本（**由脚本生成** ⇒ 文档不可能与队列漂移 ✓）。"""
    s = summary(rows)
    counts = s["counts"]
    top = [r for r in rows if r["id"] in open_set(rows)]
    top.sort(key=lambda r: (SEVERITY_RANK.get(r["severity"], 9), r["id"]))
    head = " · ".join(f"`{r['id']}`" for r in top[:5])
    block = [
        ONB_BEGIN,
        f"<!-- QUEUE:COUNTS total={s['total']} blocker={counts['blocker']} high={counts['high']} "
        f"medium={counts['medium']} low={counts['low']} -->",
        f"**未完成 {s['total']} 条**（blocker **{counts['blocker']}** · high **{counts['high']}** · "
        f"medium **{counts['medium']}** · low **{counts['low']}**）· 取队列："
        f"`python3 scripts/ux-queue.py --next`（或 `--json`）· 认领/收口：`--claim <id>` / "
        f"`--advance <id> --to done --commit <sha> --reading '<读数>'`",
        f"最靠前的 5 条：{head}（全量见 `--json`；本块**由脚本渲染**，手改会被 `--check` 判红 ✗）",
        ONB_END,
    ]
    return "\n".join(block)


# ── 组装队列 ───────────────────────────────────────────────────────────────

def build_queue() -> tuple[list[dict], list[str]]:
    """把四个来源并成一份队列 ⇒ `(rows, 未定处置的 id)`。

    **合并规则**：`extras` 是**处置的唯一出处**（索引只有 reference）；同一个 id 出现两处
    就合并来源、处置冲突即判红 ✗。`disposition is None` 的条目**不静默丢** ——
    收集起来由覆盖自检判红（"索引里有、账上没处置" = 没人认领 ✗）。
    """
    merged: dict[str, dict] = {}
    for row in load_extras() + load_gaps() + load_index():
        ident = row["id"]
        if ident not in merged:
            merged[ident] = row
            continue
        old = merged[ident]
        old["sources"] = sorted(set(old["sources"] + row["sources"]))
        if row["disposition"] is None:
            continue                                  # 索引只是 reference，处置以 extras 为准 ✓
        if old["disposition"] is None:
            old["disposition"] = row["disposition"]
            old["expect"] = row.get("expect")
        elif old["disposition"] != row["disposition"]:
            raise UsageError(
                f"id `{ident}` 在两处来源里处置不一致：{old['disposition']} vs {row['disposition']}"
                f"（{old['sources']}）—— 一个 id 只能有一个处置 ✗")
        for key in ("owner", "note", "where"):
            if not old.get(key) and row.get(key):
                old[key] = row[key]
    undecided = sorted(k for k, v in merged.items() if v["disposition"] is None)
    rows = [v for v in merged.values() if v["disposition"] is not None]
    rows.sort(key=lambda r: (SEVERITY_RANK[r["severity"]], r["id"]))
    return rows, undecided


def open_set(rows: list[dict]) -> set[str]:
    return {r["id"] for r in rows if r["disposition"] in OPEN_DISPOSITIONS}


def summary(rows: list[dict]) -> dict:
    counts = {"blocker": 0, "high": 0, "medium": 0, "low": 0}
    for r in rows:
        if r["disposition"] in OPEN_DISPOSITIONS:
            counts[r["severity"]] += 1
    total = len(open_set(rows))
    return {"total": total, "high": counts["blocker"] + counts["high"],
            "medium": counts["medium"], "low": counts["low"], "counts": counts,
            "queued_all": len(rows)}


def report_line(s: dict) -> str:
    return f"队列：{s['total']} 条（高 {s['high']} · 中 {s['medium']} · 低 {s['low']}）"


# ── 读数（`--live`） ───────────────────────────────────────────────────────

def _clean_env() -> dict:
    env = dict(os.environ)
    # 干净缓存纪律（M-07 的教训：旧产物会答旧行为）
    import tempfile
    env["SOKONANODA_CACHE_DIR"] = tempfile.mkdtemp(prefix="ux-queue-")
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"
    env["SOKONANODA_NO_MODULE_ARTIFACTS"] = "1"
    return env


def probe_kind(row: dict) -> str:
    """`lang` / `lsp` / `editor` / `vim` / `none` —— 决定怎么读。"""
    repro = row.get("repro") or ""
    if "ux-probes/lang/" in repro:
        return "lang"
    if "lsp-probe.mjs" in repro:
        return "lsp"
    if "editor-probe.mjs" in repro:
        return "editor"
    if "vim-probe.mjs" in repro:
        return "vim"
    return "none"


def run_probe(row: dict, timeout: int) -> tuple[str, str]:
    """跑一条探针 ⇒ `(verdict, detail)`，verdict ∈ {reproducing, fixed, error, skipped}。

    语义（**与来源探针自己的退出码约定对齐**，见索引 §7）：
      `lang/*.sh` role：exit 0 = 缺口仍在 · exit 1 = 已变 · 其它 = 环境错
      `lsp-probe.mjs` role：exit 0 = 一致（已变）· exit 1 = 不一致（缺口仍在）· ≥2 = 环境
    """
    kind = probe_kind(row)
    if kind == "none":
        return ("skipped", "这条没有可跑的探针（判据在它的来源台账里）")
    cmd = row["repro"]
    argv = ["bash", "-c", cmd] if cmd.startswith(("bash ", "node ")) else ["bash", "-c", cmd]
    try:
        proc = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True,
                              encoding="utf-8", errors="replace", env=_clean_env(),
                              timeout=timeout)
    except subprocess.TimeoutExpired:
        return ("error", f"超时（>{timeout}s）—— 环境慢或探针挂住，**没有读数** ✗")
    except OSError as err:  # pragma: no cover - 环境问题
        return ("error", f"跑不起来：{err}")
    rc = proc.returncode
    tail = " ".join((proc.stdout or proc.stderr or "").strip().splitlines()[-1:])[:120]
    verdict, note = judge_rc(kind, rc)
    return (verdict, f"{note} —— {tail}" if note else tail)


def judge_rc(kind: str, rc: int) -> tuple[str, str]:
    """探针**退出码 → 判定**（**约定按 `kind` 分流** —— 四类探针并不共用一套约定）。

    * `lang/*.sh`：exit 0 = 缺口仍在 · 1 = 已变/已修 · 其它 = 环境错
    * `vim-probe.mjs`：**与 lang 同向** —— 它判的是"**无 vim 臂绿 + 有 vim 臂红**"这个
      **冲突是否成立** ⇒ exit 0 = 冲突仍在（缺口仍在）· 1 = 冲突已不在（已修）
    * `lsp-probe.mjs` / `editor-probe.mjs`：exit 0 = 与期望一致（已变）· 1 = 不一致（缺口仍在）

    ⚠ 2026-10-12 实测的 bug（接口 agent 提交 `SUB-vim-exit-inverted`）：旧代码只有
    "lang" 与"其余一律按 lsp"两套 ⇒ **vim 被判反**：缺口仍在中报 `fixed`（假绿）、
    已修报 `reproducing`（假红）⇒ `--live` 对 vim 条目永远给反的读数。抽成纯函数是为了让
    `--selftest` 能逐档断言（见"探针退出码约定对账"）✓。
    """
    if kind in {"lang", "vim"}:
        if rc == 0:
            return ("reproducing", "")
        if rc == 1:
            return ("fixed", "")
        return ("error", f"退出码 {rc}（约定只认 0/1）")
    if rc == 0:
        return ("fixed", "")
    if rc == 1:
        return ("reproducing", "")
    return ("error", f"退出码 {rc}（2 = 用法/环境，3 = 前置缺失）")


def reading_ok(row: dict, verdict: str) -> tuple[bool, str]:
    """实读 × 处置 ⇒ (ok, 说明)。"""
    disp = row["disposition"]
    if verdict in {"error", "skipped"}:
        # **不是判红**：读数拿不到 ≠ 台账错 ✓ —— 但要说清"没被验证" ✗
        return (True, f"⏭ {verdict}（**未验证** ✗，不是红项）")
    if disp == "fix":
        return ((verdict == "reproducing"),
                "✓ 缺口仍在（与 `fix` 自洽）" if verdict == "reproducing" else
                "✗ 探针已转「已变」⇒ 该改成 `fixed` 并记账（或修回退路了）")
    if disp == "fixed":
        return ((verdict == "fixed"),
                "✓ 已变（与 `fixed` 自洽）" if verdict == "fixed" else
                "✗ 探针又报「缺口仍在」⇒ 回归了，或处置写错了 ✗")
    if disp in {"scheduled", "wo-filed", "boundary", "control"}:
        return (True, f"（{disp}：只登记读数，不作为判据）实测 {verdict}")
    return (True, "")


# ── 探针目录（**地面真相**）：加了用例没人登记 ⇒ 判红 ──────────────────────

PROBE_DIRS = ("lang", "lsp", "editor", "vim")
# 规范判据 / 对照臂脚本（不是"缺口探针"，但同属地面真相 ⇒ 必须被点名，见 `REGISTERED_PROBES`）
REGISTERED_SCRIPTS = {
    "scripts/vscode-input-vim-e2e.mjs",     # vim 线的**规范判据**（9 用例 × 2 臂）
    "scripts/vscode-input-e2e.mjs",         # 无 vim 对照臂（11 用例）
}


def probe_inventory() -> list[str]:
    """`scripts/ux-probes/**` 现有的探针（lang 是逐文件，其余是三个 .mjs 入口）。"""
    files = sorted(p for p in (ROOT / "scripts" / "ux-probes" / "lang").glob("*.sh"))
    entries = [str(f.relative_to(ROOT)) for f in files]
    for name in ("lsp-probe.mjs", "editor-probe.mjs", "vim-probe.mjs"):
        path = ROOT / "scripts" / "ux-probes" / name
        if path.exists():
            entries.append(str(path.relative_to(ROOT)))
    return entries


def check_probe_coverage(rows: list[dict], index_rows: list[dict]) -> int:
    """每一个探针都必须被**点名**：索引里有它的复现命令，或 extras 里 `where` 提到它。

    为什么：探针是**地面真相** —— 加了用例却没人登记 ⇒ 它永远不会被排期 ✓
    （这正是"加了用例没人管"的形状 ✗）。
    """
    blobs = [r["repro"] + " " + (r.get("where") or "") for r in rows]
    blobs += [r["repro"] for r in index_rows]
    blobs += sorted(REGISTERED_SCRIPTS)
    bad = 0
    for entry in probe_inventory():
        if not any(entry in blob for blob in blobs):
            print(f"✗ 探针没人登记：`{entry}` 不在索引（`{_label(INDEX)}`）也不在 "
                  f"`{_label(EXTRAS)}` 里 ⇒ 它永远不会被排期 ✗", file=sys.stderr)
            bad += 1
    return bad


# ── 检查 ───────────────────────────────────────────────────────────────────

def _label(path: Path) -> str:
    """仓库内 → 相对路径；临时夹具（selftest）→ 原样，别让报错信息自己抛异常 ✓。"""
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def check_uncovered(rows: list[dict], live: bool, timeout: int, undecided: list[str] | None = None) -> int:
    bad = 0
    known = {r["id"] for r in rows}

    # ① 来源 → 队列（任何来源里存在、而队列不知道的条目）
    try:
        index_rows = load_index()
    except UsageError as err:
        print(f"✗ {err}", file=sys.stderr)
        return 1
    for source, loader in (
        ("docs/gaps/ledger.jsonl", load_gaps),
        (_label(INDEX), lambda: index_rows),
        (_label(EXTRAS), load_extras),
    ):
        try:
            for entry in loader():
                if entry["id"] not in known:
                    print(f"✗ 覆盖漏项：{source} 里的 `{entry['id']}` 不在队列里", file=sys.stderr)
                    bad += 1
        except UsageError as err:
            print(f"✗ {err}", file=sys.stderr)
            bad += 1

    # ①b **没处置 = 没人认领**（索引里有、账上没写处置）
    for ident in (undecided or []):
        print(f"✗ 没处置：`{ident}` 出现在 {_label(INDEX)}，但 `{_label(EXTRAS)}` 里没有它的处置"
              f"（三选一：fix / scheduled / wo-filed ⇒ **不许『待定』** ✗）", file=sys.stderr)
        bad += 1

    # ①c **探针地面真相**：加了用例没人登记 ⇒ 判红
    bad += check_probe_coverage(rows + [{"id": i, "repro": "", "where": ""} for i in ()], index_rows)

    # ② §0.2：人维护的粗粒度线不许是幽灵；渲染摘要必须与队列**同数**
    try:
        doc_ids = set(load_onboarding_queue())
        doc_counts = load_onboarding_counts()
    except UsageError as err:
        print(f"✗ {err}", file=sys.stderr)
        return 1
    for ident in sorted(doc_ids - known):
        print(f"✗ §0.2 多行：`{ident}` 在 §0.2 表里，但队列不知道它（加进 "
              f"{_label(EXTRAS)} 或索引，别让唯一入口与队列两头漂移 ✗）", file=sys.stderr)
        bad += 1
    for ident in sorted(doc_ids & known):
        disp = next(r for r in rows if r["id"] == ident)["disposition"]
        if disp not in OPEN_DISPOSITIONS:
            print(f"✗ §0.2 陈旧行：`{ident}` 的处置已是 `{disp}`（终态），§0.2 该把它摘掉 ✗",
                  file=sys.stderr)
            bad += 1
    if doc_counts is None:
        print(f"✗ §0.2 没有**渲染摘要块**（{ONB_BEGIN} … {ONB_END}）⇒ 唯一入口与队列脱钩 ✗ "
              f"—— 跑 `python3 scripts/ux-queue.py --render` 生成 ✓", file=sys.stderr)
        bad += 1
    else:
        s_now = summary(rows)
        want = {"total": s_now["total"], **s_now["counts"]}
        if doc_counts != want:
            print(f"✗ §0.2 摘要与队列**不同数**：文档 {doc_counts} vs 实测 {want} ⇒ "
                  f"跑 `--render` 刷新（**手改文档不算** ✗）", file=sys.stderr)
            bad += 1

    # ④ 读数（`--live`）
    if live and not bad:
        print(f"── 读数自检（{len(rows)} 条，逐条跑探针；超时 {timeout}s）", file=sys.stderr)
        for row in rows:
            verdict, detail = run_probe(row, timeout)
            ok, why = reading_ok(row, verdict)
            mark = "✓" if ok else "✗"
            print(f"{mark} {row['id']:<12} {row['disposition']:<9} {verdict:<11} {why}")
            if not ok:
                print(f"   复现：{row['repro']}\n   现场：{detail}", file=sys.stderr)
                bad += 1

    if bad:
        print(f"\n✗ 覆盖/一致性自检：**{bad} 条红** —— 队列与来源/§0.2 不一致"
              f"（这就是「过两天又冒出一堆」的防呆 ✗）", file=sys.stderr)
        return 1
    print("✓ 覆盖自检通过：四个来源逐条都在队列里 · 探针都有登记 · §0.2 摘要与队列同数")
    return 0


# ── 调度器（**事件溯源**）：认领 / 迁移 / 拆解 / 锁 ───────────────────────────
#
# 2026-10-12 用户：「这个事件机制不能全依赖大模型运作，而是同时依靠脚本，比如 python
# 脚本当成一个事件调度器」⇒ **规则长在代码里**：人和模型都只能通过下面这几条子命令
# 改状态，非法迁移在脚本层就被拒 ✗。状态本身 = `docs/ux-queue-events.jsonl` 里
# **只追加**的事件的折叠（fold）：
#   ① 只追加 ⇒ 天然适配"一次只有一个写者"、永不覆盖别人的行；
#   ② 可重算 ⇒ 删掉事件文件就回到"来源台账的处置"（事件不是新真相源，是覆盖层）；
#   ③ 每条迁移都留痕（谁 / 何时 / 什么证据）。

# **事件表路径**（队列状态的唯一真相）。⚠ 允许用 `SOKO_QUEUE_EVENTS` 重定向 —— 这是
# 给**测试/验证脚本**的：它们会造合成 id（8 路并发认领 `R-*`、未声明资源 `P-*`）来验调度器，
# 而事件表是**主 checkout 单点**（`SOKO_QUEUE_ROOT`）⇒ 不重定向就会往真台账里灌合成条目、
# 让"幽灵条目"判据反复假红（接口 agent 提交 `SUB-ghost-guard-diluted` 指出的正是这个，
# 而"跳过 R-*/P-*"只是治标 —— 那等于**放宽守卫** ✗）。
EVENTS = Path(os.environ["SOKO_QUEUE_EVENTS"]) if os.environ.get("SOKO_QUEUE_EVENTS") \
    else ROOT / "docs" / "ux-queue-events.jsonl"
LOCK_TTL_S = 2 * 60 * 60  # 两小时没心跳 ⇒ 陈旧（默认**只警告**；`--force` 才接管）
# **并发 = 4 核**（2026-10-12 用户：「允许最多 4 个 subagent，类比 4 核心 cpu」）——
# 但**同一模块同一时间只允许一个写者** ✗ ⇒ 认领要过**资源矩阵**（不是简单的计数锁）。
MAX_SLOTS = int(os.environ.get("SOKO_QUEUE_SLOTS", "4") or 4)
# `owner` → 它要写的资源（粗粒度 = 仓库里"改动过的 crate"那一层）
RESOURCE_OF_OWNER = {
    "front": "crates/front/**", "lsp": "crates/lsp/**", "cli": "crates/cli/**",
    "kernel": "crates/kernel/**", "editor": "editor/vscode/**", "courses": "courses/**",
    "docs": "docs/**", "prelude": "prelude/**", "scripts/soko": "scripts/soko",
    "tooling": "scripts/**", "scripts": "scripts/**",
}

SCHED_STATES = ("todo", "in_progress", "blocked", "done", "wo_filed", "umbrella",
                # 外部提交面（`--submit`/`--triage`）的两个状态：**提交者只追加**，
                # 主流受理后才终态 ⇒ 用独立状态名，别借用 `done`（`done` 的证据要求是
                # "commit + 判据读数"，而受理的证据是"commit"（applied）或"理由"（rejected））。
                "submitted", "triage_rejected")
TRIAGE_STATES = {"submitted", "triage_rejected"}
TERMINAL_STATES = {"done", "wo_filed"}
# **允许的迁移表 = 规则的唯一出处**（改这里才改得动行为）
TRANSITIONS = {
    "todo":        {"in_progress", "blocked", "umbrella", "wo_filed"},
    "in_progress": {"done", "blocked", "todo", "umbrella", "wo_filed"},
    "blocked":     {"todo", "in_progress", "wo_filed"},
    "umbrella":    {"done", "wo_filed"},
    "done":        {"todo"},       # 只能靠 `--advance … --to todo --reason …`（重开）
    "wo_filed":    {"todo"},
    # **外部提交面的两个状态也必须有出口**（2026-10-14 · 第三轮独立复核实测）：
    #   · `triage_rejected` 原先**一个出口都没有** ✗ ⇒ 判红时打印的收口指示
    #     `--advance <id> --to wo_filed --reason …` **实测 rc=1 非法迁移** ⇒ 唯一出路是
    #     **手改事件表**（"规则在脚本里"这条就断了）。**拒收也是终局之一** ⇒ 要能记账：
    #     `wo_filed` = 收下这次拒收（`cmd_advance` 强制 `--reason` ✓）· `todo` = 退回重开。
    #   · `submitted` 的**正常**出口是 `--triage`（applied/deferred/rejected ✓），但删掉
    #     `check_plan` 里那个 `submitted` 豁免之后（见那里的注释），漂移规则**也会对它**
    #     打印 `--advance …` 收口指示 ⇒ 表里没有出口的话那条指示同样是死路
    #     （实测 `submitted → done` / `→ wo_filed` 都不合法）。⚠ 直接 `--advance` 绕过
    #     `--triage` **不会**变成后门：那时来源处置还是 `fix` ⇒ `check_plan` 的
    #     「done 而来源不是终态」照样判红 ✓（纪律没松）。
    "submitted":       {"done", "wo_filed", "todo"},
    "triage_rejected": {"wo_filed", "todo"},
}


def closure_steps(frm: str, want: str = "done", limit: int = 4) -> list[str]:
    """从 `frm` 走到 `want` 的**最短合法迁移序列**（不再走就退到任一终态；`TRANSITIONS` 是唯一出处）。

    ⚠ 为什么需要（2026-10-14 · 第三轮复核实测的**同族第三处**）：判红信息里手写的
    `--advance <id> --to done …` 假设"下一步就合法"，而表里 `todo → done` / `blocked → done`
    **故意**不合法（收口要先 `--claim`/`--advance … --to in_progress`；`--selftest` 有一条
    `todo → done 必须被拒` 的判据钉着它）⇒ 照指示敲就是 rc=1（实测：36 格网格里有 12 格
    的指示非法；删掉 `submitted` 豁免后变 **18 格**）。同理 `triage_rejected` 原先没有出口。
    ⇒ 指示改成**按表算出来的步序列**（每一步都能直接敲），并由网格自检逐格核对：
    把指示里的每一步按 `TRANSITIONS` 走一遍 —— 走不通就判红（守卫咬得住"指示是死路" ✗）。
    返回 `[]` = 表里确实无路可走 ⇒ 调用方要**如实说"只能手改事件表"**，不许再给一条死路指示 ✗。
    """
    for target in [want] + sorted(TERMINAL_STATES - {want}):
        queue: list[list[str]] = [[frm]]
        seen = {frm}
        while queue:
            path = queue.pop(0)
            if path[-1] == target:
                return path[1:]
            if len(path) > limit:
                continue
            for nxt in sorted(TRANSITIONS.get(path[-1], set())):
                if nxt not in seen:
                    seen.add(nxt)
                    queue.append(path + [nxt])
    return []


def _step_cmd(ident: str, to: str) -> str:
    """把一步迁移渲染成**可直接执行**的一条命令（`done` 要证据、`wo_filed` 要理由 ✓）。"""
    if to == "done":
        return f"`--advance {ident} --to done --commit <sha> --reading '<读数>'`"
    if to == "wo_filed":
        return f"`--advance {ident} --to wo_filed --reason '<为什么不做>'`"
    return f"`--advance {ident} --to {to}`"


def _now() -> str:
    import datetime
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def _age_s(ts: str | None) -> float | None:
    import datetime
    if not ts:
        return None
    try:
        then = datetime.datetime.fromisoformat(ts)
    except ValueError:
        return None
    if then.tzinfo is None:
        then = then.replace(tzinfo=datetime.timezone.utc)
    return (datetime.datetime.now(datetime.timezone.utc) - then).total_seconds()


def _pid_alive(pid) -> bool:
    try:
        os.kill(int(pid), 0)
    except (OSError, TypeError, ValueError):
        return False
    return True


def _git_common_dir() -> Path:
    """锁放**共享 git 目录**（多 worktree 共用）⇒ 不出现在 `git status` 里 ✓。"""
    try:
        out = subprocess.run(["git", "rev-parse", "--git-common-dir"], cwd=ROOT,
                             capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as err:
        raise UsageError(f"取不到 git common dir（调度器要放锁）：{err}") from err
    p = Path(out)
    return p if p.is_absolute() else (ROOT / p).resolve()


def lock_file() -> Path:
    return _git_common_dir() / "soko-ux-queue.lock"


def read_lock() -> dict:
    """锁文件 = `{"claims": [ {id, owner, pid, resources, ts} … ]}`（兼容旧的单条格式）。"""
    path = lock_file()
    if not path.exists():
        return {"claims": []}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"claims": []}
    if isinstance(data, dict) and isinstance(data.get("claims"), list):
        return data
    if isinstance(data, dict) and data.get("id"):        # 旧格式 ⇒ 当成一条认领
        return {"claims": [data]}
    return {"claims": []}


def lock_guard():
    """**跨进程互斥**：锁文件旁边的 `.guard` 上一个 `flock`（`with lock_guard():`）。

    ⚠ 为什么需要（2026-10-12 实测的 bug，由接口 agent 提交 `SUB-claim-not-atomic`）：
    `acquire_claim` 是 `read_lock()` → 判定 → `write_lock(kept)` 的 **check-then-act**，
    而 `write_lock` 是**整文件覆盖** ⇒ 同资源的 N 个并发认领**全部打印"✓ 认领"**，
    但锁里只剩**最后写者**那一条（既超发、又静默丢别人的认领）✗ —— 本脚本存在的唯一理由
    就是"4 棵 worktree 并行时的安全认领" ⇒ 不原子 = 前提不成立。
    实测（8 路子进程过同一 barrier）：`success reports = 3 | claims in lock = 1`（期望 1 / 1）。

    `fcntl` 缺席（非 POSIX）⇒ 退化成 no-op **并大声说明**（"判不了 ≠ 绿" ✗，同仓库其它守卫口径）。
    """
    import contextlib

    try:
        import fcntl
    except ImportError:                                  # pragma: no cover - 非 POSIX
        # ⚠ 这里不能静默：没有 flock 就没有原子性，调用方要能看见
        print("⚠ 没有 fcntl ⇒ `--claim` 的跨进程互斥**不成立**（并发认领可能超发）✗",
              file=sys.stderr, end="")
        return contextlib.nullcontext()

    @contextlib.contextmanager
    def _guard():
        path = lock_file().with_suffix(".guard")
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "a+", encoding="utf-8") as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            try:
                yield
            finally:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)

    return _guard()


def write_lock(claims: list[dict]) -> None:
    path = lock_file()
    if not claims:
        try:
            path.unlink()
        except OSError:
            pass
        return
    # 原子替换：**别就地截断写**（读者可能正读到一半 ⇒ 半截 JSON）——
    # `os.replace` 在同一文件系统上是原子的，读者要么看到旧文件、要么看到新文件 ✓。
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps({"claims": claims}, ensure_ascii=False), encoding="utf-8")
    os.replace(tmp, path)


def claim_is_live(claim: dict) -> tuple[bool, str]:
    """活跃判定 = **有 `pid` 就钉进程，没 `pid` 就只受 TTL 约束**。

    ⚠ 为什么"没 `pid` 也算活跃"（2026-10-12 实测的 bug）：`--claim` 是**一次 CLI 调用**，
    它 `exit` 之后 `os.getpid()` 立刻变成死进程 ⇒ 若把"无 pid"当死，**下一次任何 `--claim`
    都会把上一条当陈旧认领清掉**（实测：连认领 4 条，锁里永远只剩最后 1 条）⇒
    文档写的 `--claim <id>` … 干活 … `--advance <id> --to done` **跨调用根本跑不通**，
    资源互斥（"同一模块同一时间只许一个写者"）也就形同虚设 ✗。
    ⇒ 规则：`pid` 是**可选**的进程钉（长驻进程/测试用）；没写就按 TTL 计（默认 2h），
    过期照样可被 `--force` 接管 ✓。
    """
    pid, age = claim.get("pid"), _age_s(claim.get("ts"))
    who = (f"{claim.get('owner') or '?'}（pid {pid if pid else '—'}·{int(age or 0)}s 前认领 "
           f"`{claim.get('id')}`·资源 {','.join(claim.get('resources') or []) or '?'}）")
    if pid and not _pid_alive(pid):
        return (False, f"{who} —— 进程已不在")
    if age is not None and age > LOCK_TTL_S:
        return (False, f"{who} —— 超过 TTL {LOCK_TTL_S}s 没动静")
    return (True, who)


def live_claims() -> list[dict]:
    return [c for c in read_lock()["claims"] if claim_is_live(c)[0]]


def resources_of(row: dict | None, override: str) -> list[str]:
    if override:
        return [r.strip() for r in override.split(",") if r.strip()]
    owner = (row or {}).get("owner") or ""
    res = RESOURCE_OF_OWNER.get(owner.strip())
    return [res] if res else [f"（未声明资源：{owner or '?'}）"]


def acquire_claim(ident: str, owner: str, resources: list[str], force: bool) -> bool:
    """**4 核 + 资源互斥**：槽位满 ⇒ 拒；与活跃认领撞资源 ⇒ 拒（除非 `--force` 接管陈旧）。

    ⚠ 整段 read→判定→write 必须在 `lock_guard()` 里（见那里的说明：不原子 ⇒ 并发认领全部
    "成功"而锁里只剩最后一条 ✗）。
    """
    with lock_guard():
        return _acquire_claim_locked(ident, owner, resources, force)


def _acquire_claim_locked(ident: str, owner: str, resources: list[str], force: bool) -> bool:
    claims = read_lock()["claims"]
    live = [c for c in claims if claim_is_live(c)[0]]
    stale = [c for c in claims if not claim_is_live(c)[0]]
    if any(c.get("id") == ident and c.get("owner") == owner for c in live):
        return True                                     # 幂等：自己已经持有
    if len(live) >= MAX_SLOTS:
        print(f"✗ {MAX_SLOTS} 个槽位已满：{[c.get('id') for c in live]} ⇒ 等其中一个收口再认领 ✗",
              file=sys.stderr)
        return False
    want = set(resources)
    clash = [c for c in live if want & set(c.get("resources") or [])]
    if clash:
        who = "；".join(claim_is_live(c)[1] for c in clash)
        if not force:
            print(f"✗ 资源冲突（**同一模块同一时间只许一个写者** ✗）：{who}\n"
                  f"  你要 `{','.join(resources)}` ⇒ 等同资源的那个收口，或改用户主（`--touch` 显式声明）",
                  file=sys.stderr)
            return False
        print(f"⚠ `--force`：与 {who} 抢同一资源（只在确认它已死/放弃时用）", file=sys.stderr)
        live = [c for c in live if c not in clash]
    # ⚠ **不写 `pid`**：`--claim` 是一次 CLI 调用，写自己的 pid 等于"认领随进程一起死"
    # ⇒ 下一条 `--claim` 会把上一条当陈旧清掉（见 `claim_is_live` 的说明）✗。
    kept = live + [{"id": ident, "owner": owner,
                    "resources": resources, "ts": _now()}]
    if stale:
        print(f"⏳ 清掉 {len(stale)} 条陈旧认领（进程已死/超 TTL）："
              f"{[c.get('id') for c in stale]}", file=sys.stderr)
    write_lock(kept)
    return True


def drop_claim(ident: str, owner: str) -> None:
    with lock_guard():
        claims = read_lock()["claims"]
        kept = [c for c in claims if not (c.get("id") == ident and c.get("owner") == owner)]
        write_lock(kept)


def load_events() -> list[dict]:
    if not EVENTS.exists():
        return []
    out: list[dict] = []
    for lineno, line in enumerate(EVENTS.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError as err:
            raise UsageError(f"{_label(EVENTS)}:{lineno} 不是合法 JSON：{err}") from err
    return out


def append_event(rec: dict) -> None:
    """**只追加**（唯一写操作）—— 调用方负责先过 `TRANSITIONS` 与证据检查 ✓。"""
    EVENTS.parent.mkdir(parents=True, exist_ok=True)
    with EVENTS.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps({"ts": _now(), **rec}, ensure_ascii=False) + "\n")


def fold(events: list[dict] | None = None) -> dict[str, dict]:
    """事件 → 状态（可重算；事件文件不是真相源，是覆盖层）。"""
    state: dict[str, dict] = {}
    for ev in (load_events() if events is None else events):
        ident, kind = ev.get("id"), ev.get("event")
        if not ident or not kind:
            continue
        cur = state.setdefault(ident, {"id": ident, "state": "todo", "owner": None,
                                       "children": [], "events": 0})
        cur["events"] += 1
        if kind == "claim":
            cur.update(state="in_progress", owner=ev.get("owner"), pid=ev.get("pid"),
                       claimed_at=ev.get("ts"))
        elif kind == "release":
            if cur["state"] == "in_progress":
                cur.update(state="todo", owner=None, pid=None)
        elif kind == "advance":
            cur.update(state=ev.get("to"), owner=None, pid=None)
            for key in ("commit", "reading", "reason"):
                if ev.get(key):
                    cur[key] = ev[key]
            if ev.get("ts"):
                # 供**祖先性闸门**按 UTC 时刻分「新增 / 存量」（见 `ANCESTRY_GATE_SINCE`）：
                # 没有时间戳的 done（旧事件 / triage 受理那一支）按**存量**从宽处理 ✓。
                cur["advanced_ts"] = ev["ts"]
        elif kind == "split":
            parent = ev.get("parent")
            if parent:
                par = state.setdefault(parent, {"id": parent, "state": "umbrella",
                                                "owner": None, "children": [], "events": 0})
                par["state"] = "umbrella"
                if ident not in par["children"]:
                    par["children"].append(ident)
            cur.update(parent=ev.get("parent"), title=ev.get("title") or cur.get("title") or ident)
        elif kind == "reopen":
            cur.update(state="todo", owner=None, pid=None, reason=ev.get("reason"))
        elif kind == "triage":
            # ⚠ **受理状态必须被折叠回来**（2026-10-12 接口 agent 提交 `SUB-triage-state-unread`）：
            # 旧代码没有这一支 ⇒ `cmd_triage` 写的 triage 事件对状态**完全不可见**，
            # `--inbox` 永远显示 `todo` ⇒ REQUIREMENTS §9.20 承诺的
            # submitted / applied / deferred / rejected 四态不可达，"外部只提交、主流受理"
            # 这条跨 agent 通道无法被追踪（谁受理了 / 什么时候 / 带哪个 commit 都看不到）✗。
            # 映射口径：受理 = 这一条**关账**（`done`）· 退回 = 回到 `todo`（可重开）·
            # 拒收 = `wo_filed`（终态，理由必填 —— `cmd_triage` 已经强制了）。
            to = ev.get("to")
            if to == "applied":
                cur.update(state="done", owner=None, pid=None,
                           commit=ev.get("commit"), reason=ev.get("reason"))
            elif to == "rejected":
                cur.update(state="triage_rejected", owner=None, pid=None, reason=ev.get("reason"))
            elif to == "deferred":
                cur.update(state="todo", owner=None, pid=None, reason=ev.get("reason"))
            cur["triage"] = to
        elif kind == "submit":
            cur.update(state="submitted" if cur.get("state") == "todo" else cur["state"],
                       owner=ev.get("owner"), kind=ev.get("kind"))
        elif kind == "schedule":
            # **编排者给自己排期**（`SUB-queue-registration-gaps` ①，2026-10-13）：与 `submit`
            # 同形（来源行 + 只追加事件），但**状态是 `todo`**（可动 ⇒ `--json` 的 `open` 查得到、
            # `--next` 推得出来 ✓）。⚠ 不能借用 `submit`：那条把状态钉成 `submitted` = "等主流
            # 受理"（外部提交面的语义），与"**我自己排的期**"正相反 ✗。
            cur.update(state="todo", owner=ev.get("owner"), title=ev.get("title"),
                       reason=ev.get("reason"))
    return state


# ── 祖先性闸门（2026-10-14 · `queueW` 修三个视图一致性缺陷 ⑤） ──────────────────
# 旧闸门**只查存在性**（`git cat-file -e <sha>^{commit}`，见 `cmd_advance`）⇒ 查不出
# 「指针**不是 main 的祖先**」✗ —— 实测（2026-10-14 量）：65 条 `done` 里 **32 条**的
# commit 是 **rebase 前的孪生**（对象还在 ✓、内容也落了 main ✓，但
# `git merge-base --is-ancestor <sha> HEAD` 非零 ⇒ 指针在 main 的历史上**找不到**）⇒
# 「有 commit」看着像证据，其实**没法在 main 上核**。
# 口径（**带日期的豁免**，**不静默放过** ✗）：
#   · **新增收口**（本闸门之后写的 `done`）**必须**过祖先性 —— 写入口硬拒（`cmd_advance`）
#     + 读侧复查（`check_plan`；防"先写后 rebase"：记录时是祖先、之后被 rebase 甩掉 ⇒ 判红）；
#   · **存量**（`ANCESTRY_GATE_SINCE` **之前**记的 done 指针）**不判红**：它们多是 rebase 前的
#     孪生，改不动（对象在、历史不在）⇒ 每次 `--check` 打一行 ⚠ 汇总（条数 + 起点的日期 +
#     为什么豁免 + 清单），**不许静默** ✓。
#   ⚠ 口径**必须用 UTC 完整时刻**，不能只写本地日期 ✗（2026-10-14 **实测踩到**）：事件表的 `ts`
#     是 `_now()` 写的 **UTC** 串（`2026-10-13T17:55:10+00:00` = 本地 2026-10-14 01:55）⇒ 只比
#     日期前 10 位会把"今天刚记的"也算成存量 ⇒ 读侧复查当场**空转** ✗。起点 = **闸门落地那一刻**：
#     UTC `2026-10-13T17:54` = 本地 2026-10-14 01:54（= `3ede788d`）—— 实测最后一条存量指针记于
#     `2026-10-13T17:21:42Z`（`MODULE-ARTIFACTS-KNIFE2-ARTIFACT-KEY` @ `40e13467`）⇒ 边界两侧
#     都有余量 ✓（ISO-8601 同格式串的字典序 = 时间序 ✓）。
#   ⚠ 若 main 再被整体 rebase 一次 ⇒ 这个起点之后记的指针会（**应该**）判红：那时候它们确实
#     解析不到 main 了 ⇒ 要么按新历史重开再收口，要么**评审可见地**改这一处口径起点。
ANCESTRY_GATE_SINCE = "2026-10-13T17:54:00+00:00"

_ANCESTRY_CACHE: dict[str, bool] = {}


def commit_is_ancestor(commit: str) -> bool:
    """`git merge-base --is-ancestor <commit> HEAD`（按 sha 记忆化：`--check` 要问几十次）。"""
    if commit not in _ANCESTRY_CACHE:
        probe = subprocess.run(["git", "merge-base", "--is-ancestor", commit, "HEAD"],
                               cwd=ROOT, capture_output=True, text=True)
        _ANCESTRY_CACHE[commit] = probe.returncode == 0
    return _ANCESTRY_CACHE[commit]


def sched_rows(rows: list[dict], state: dict[str, dict]) -> list[dict]:
    """把**拆解出来的子任务**并进队列视图：插在它父条目的**前面**（执行顺序 = 队列顺序）。

    ⚠ **子任务有源行 ⇒ 用源行**（2026-10-14 · `queueW` 修三个视图一致性缺陷 ①）：
    旧代码给**每个**子任务都合成一行、且硬编码 `disposition="fix"` ⇒ 两处坏在同一处：
      · 合成行里的 `fix` 把源行的**真处置**（`fixed`/`wo-filed`）盖掉；
      · 合成行**先进 `out`** ⇒ 走到该 id 的**源行时 `id in seen` ⇒ 源行被丢弃** ✗。
    实测后果（"队列空了吗"三个视图三个答案）：`F12b-notation-blind-intro` /
    `F12b2-front-intro-notation` 在 `docs/ux-queue-extras.jsonl` 里明明是 `fixed`，
    视图里却是 `fix` ⇒ ① `--next` 把**已经做完**的推出来；② `check_plan` 的 `disp` 取到
    被遮蔽的 `fix` ⇒「反向两层漂移」**对子任务永远咬不住**（`--check` 假绿）；
    ③ §0.2（`summary(view)`）报 2 条，而 `--json.open`（当时只从源行算）报 0 条 ✗。
    ⇒ 口径：**有源行就用源行**（只挪位置、**不丢行** ✗），**没有源行的子任务才合成**
    （合成行的 `repro` 里写明"拆解自哪条"）；子任务仍插在父**之前** ⇒ 顺序不变 ✓。
    """
    kids: dict[str, list[str]] = {}
    for ident, cur in state.items():
        par = cur.get("parent")
        if par:
            kids.setdefault(par, []).append(ident)
    by_id = {row["id"]: row for row in rows}
    extra: dict[str, dict] = {}
    for ident, cur in state.items():
        if not cur.get("parent"):
            continue
        src = by_id.get(ident)
        extra[ident] = src if src is not None else {
            "id": ident, "facet": "队列", "severity": "medium", "disposition": "fix",
            "repro": f"（拆解自 `{cur.get('parent')}`：{cur.get('title') or ''}）",
            "owner": cur.get("owner") or "", "where": _label(EVENTS)}
    out: list[dict] = []
    seen: set[str] = set()
    for row in rows:
        for ident in kids.get(row["id"], []):
            if ident not in seen:
                out.append(extra[ident])
                seen.add(ident)
        if row["id"] not in seen:
            out.append(row)
            seen.add(row["id"])
    for ident, row in extra.items():          # 父条目不在队列里（异常形状）也别丢
        if ident not in seen:
            out.append(row)
            seen.add(ident)
    return out


def check_lock_or_die(owner: str, force: bool, taking: bool) -> bool:
    """返回 True = 可以继续（拿锁/放锁）。被人持有且不是自己 ⇒ 除非 `--force` 否则判红。"""
    lock = read_lock()
    live, why = lock_is_live(lock)
    if live and (lock or {}).get("owner") != owner:
        if not force:
            print(f"✗ 队列锁在别人手里：{why} ⇒ **一次只有一个写者** ✗ —— 等它放锁，"
                  f"或确认它死了再 `--force`", file=sys.stderr)
            return False
        print(f"⚠ `--force` 接管：{why}", file=sys.stderr)
    if not taking:
        try:
            if lock_file().exists() and (lock or {}).get("owner") == owner:
                lock_file().unlink()
        except OSError as err:  # pragma: no cover - 环境问题
            print(f"⚠ 放锁失败（不影响判定）：{err}", file=sys.stderr)
    return True


def cmd_claim(ident: str, owner: str, force: bool, rows: list[dict],
              touch: str = "") -> int:
    state = fold()
    row = next((r for r in rows if r["id"] == ident), None)
    if row is None and ident not in state:
        raise UsageError(f"队列里没有 `{ident}` ⇒ 先把它登记进来源（`--json` 看现有 id）")
    cur = state.get(ident, {})
    if cur.get("state") in TERMINAL_STATES:
        print(f"✗ `{ident}` 已是终态 `{cur['state']}` ⇒ 要重开得先 "
              f"`--advance {ident} --to todo --reason '<为什么>'` ✗", file=sys.stderr)
        return 1
    resources = resources_of(row, touch)
    if not acquire_claim(ident, owner, resources, force):
        return 1
    append_event({"event": "claim", "id": ident, "owner": owner,
                  "resources": resources})
    slots = len(live_claims())
    print(f"✓ 认领 `{ident}`（owner={owner} · 资源 {','.join(resources)} · "
          f"占用 {slots}/{MAX_SLOTS} 核）⇒ 做完交证据：\n"
          f"    python3 scripts/ux-queue.py --advance {ident} --to done "
          f"--commit <sha> --reading '<判据读数>'")
    return 0


def cmd_advance(ident: str, to: str, commit: str, reading: str, reason: str, owner: str,
                force: bool, rows: list[dict]) -> int:
    if to not in SCHED_STATES:
        raise UsageError(f"`--to` 只能是 {list(SCHED_STATES)}（给的是 `{to}`）")
    state = fold()
    known = {r["id"] for r in rows} | set(state)
    if ident not in known:
        raise UsageError(f"队列里没有 `{ident}`")
    cur = state.get(ident, {})
    frm = cur.get("state", "todo")
    if to == "todo" and frm in TERMINAL_STATES:
        if not reason:
            print("✗ 重开终态条目必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
            return 1
        append_event({"event": "reopen", "id": ident, "reason": reason, "owner": owner})
        print(f"✓ 重开 `{ident}`（{frm} → todo）⇒ 状态回到来源处置，事件留痕 ✓")
        return 0
    if to not in TRANSITIONS.get(frm, set()):
        print(f"✗ 非法迁移 `{frm} → {to}`（`{frm}` 允许：{sorted(TRANSITIONS.get(frm, []))}）"
              f" —— **规则在脚本里**，不是靠提示词 ✗", file=sys.stderr)
        return 1
    if to == "done" and not (commit and reading):
        print("✗ 收口必须带**证据**：`--commit <sha>` + `--reading '<判据读数>'` ✗", file=sys.stderr)
        return 1
    # ⚠ **commit 必须真的存在**（2026-10-12 接口 agent 提交 `SUB-advance-commit-unchecked`）：
    # 旧闸门只查"非空字符串" ⇒ 实测 `--commit deadbeefdeadbeef…`（`git cat-file -e` 非零）
    # **rc 0** 通过，账上留下一个"有 commit"的**假终态**；而 `--release check`（切版收拢）
    # 也只看 done + 有证据 ⇒ **假证据能把不该入版的条目放进版本** ✗。
    # 判据：`git cat-file -e <sha>^{commit}`（存在且是 commit 对象）。
    if to == "done":
        import subprocess as _sp3
        probe = _sp3.run(["git", "cat-file", "-e", f"{commit}^{{commit}}"],
                         cwd=ROOT, capture_output=True, text=True)
        if probe.returncode != 0:
            print(f"✗ `--commit {commit}` 在仓库里**不存在**（`git cat-file -e` 非零）"
                  f" ⇒ 收口证据是假的，不许记 ✗（先把这个环节真的提交掉）", file=sys.stderr)
            return 1
        # ⚠ **再加一层祖先性**（2026-10-14 · 缺陷 ⑤）：存在性查不出「指针不是 main 的祖先」
        # —— 实测 32 条历史 done 指针是 **rebase 前的孪生**（对象在、内容也落了 main，
        # 但 `git merge-base --is-ancestor <sha> HEAD` 非零）⇒ 证据**没法在 main 上核** ✗。
        # 口径：**新增收口**必须过闸门（存量走 `check_plan` 的带日期豁免，见常量块说明）。
        if not commit_is_ancestor(commit):
            print(f"✗ `--commit {commit}` **不是 `HEAD` 的祖先**"
                  f"（`git merge-base --is-ancestor` 非零）⇒ 指针解析不到 main 的历史上"
                  f"（多半是 rebase 前的孪生，或别的分支）⇒ 收口证据在 main 上**找不到**，"
                  f"不许记 ✗（用**真落进 main 的那个 commit**；口径起点 "
                  f"`ANCESTRY_GATE_SINCE={ANCESTRY_GATE_SINCE}`）", file=sys.stderr)
            return 1
    if to in {"wo_filed", "blocked"} and not reason:
        print(f"✗ `{to}` 必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
        return 1
    # **收口即放槽**：`--advance` 的每个目标状态都放开认领（不再只在 `in_progress` 时放）——
    # 否则文档写的 `--claim X` … `--advance X --to done`（不经 `in_progress`）会**漏放锁**，
    # 槽位一直被自己占着（实测：`--to done` 之后 `live_claims()` 仍是 1）✗。
    if to in {"done", "wo_filed", "blocked", "todo"}:
        # ⚠ 读取与放锁必须在**同一段** `lock_guard()` 里（否则两个收口者可以都读到"没人持有"
        # ⇒ 都放一次、都通过；原子性见 `lock_guard` 的说明）。
        with lock_guard():
            held = [c for c in read_lock()["claims"] if c.get("id") == ident]
            if held and held[0].get("owner") != owner and not force:
                print(f"✗ `{ident}` 被 `{held[0].get('owner')}` 认领着 ⇒ 别替别人收口 ✗",
                      file=sys.stderr)
                return 1
            holder = held[0].get("owner") if held else owner
            claims = read_lock()["claims"]
            write_lock([c for c in claims
                        if not (c.get("id") == ident and c.get("owner") == holder)])
    if to == "done":
        kids = cur.get("children", [])
        open_kids = [k for k in kids if state.get(k, {}).get("state") not in TERMINAL_STATES]
        if kids and open_kids:
            print(f"✗ `{ident}` 是 umbrella，还有 {len(open_kids)} 个子任务没关：{open_kids} ✗",
                  file=sys.stderr)
            return 1
    append_event({"event": "advance", "id": ident, "to": to, "commit": commit or None,
                  "reading": reading or None, "reason": reason or None, "owner": owner})
    print(f"✓ `{ident}`：{frm} → {to}" + (f" · commit {commit[:8]}" if commit else "")
          + (f" · 理由：{reason}" if reason else ""))
    return 0


def cmd_add_child(parent: str, child: str, title: str, owner: str, rows: list[dict]) -> int:
    state = fold()
    known = {r["id"] for r in rows} | set(state)
    if parent not in known:
        raise UsageError(f"父条目 `{parent}` 不在队列里")
    if child in known:
        raise UsageError(f"子任务 id `{child}` 已经存在（id 是键，不许重用）")
    if state.get(parent, {}).get("state") in TERMINAL_STATES:
        print(f"✗ 父条目 `{parent}` 已终态 ⇒ 先 `--advance {parent} --to todo --reason …` ✗",
              file=sys.stderr)
        return 1
    if not acquire_claim(f"split:{parent}", owner, resources_of(
            next((r for r in rows if r["id"] == parent), None), ""), True):
        return 1
    append_event({"event": "split", "id": child, "parent": parent, "title": title,
                  "owner": owner})
    drop_claim(f"split:{parent}", owner)
    print(f"✓ 就地拆解：`{child}`（{title}）插在 `{parent}` **之前**；"
          f"`{parent}` 转 umbrella ⇒ 子任务全关才能收口 ✓")
    return 0


def cmd_unclaim(owner: str, reason: str) -> int:
    """放掉 `owner` 的**全部**认领（4 核里的自己那几格）⇒ 条目回 todo。"""
    with lock_guard():
        claims = read_lock()["claims"]
        mine = [c for c in claims if c.get("owner") == owner]
        if not mine:
            print(f"（`{owner}` 没有持有任何认领；活跃的是 "
                  f"{[c.get('id') for c in live_claims()]}）")
            return 0
        for c in mine:
            append_event({"event": "release", "id": c.get("id"), "owner": owner,
                          "reason": reason or None})
        write_lock([c for c in claims if c.get("owner") != owner])
    print(f"✓ 放掉 {len(mine)} 条认领（{[c.get('id') for c in mine]}）"
          + (f" · {reason}" if reason else ""))
    return 0


def effective_state(row: dict, cur: dict) -> str:
    """事件状态优先；没有事件 ⇒ 用来源的处置翻译（`fix`/`scheduled` 都是**可动**的 todo）。"""
    if cur.get("state"):
        return cur["state"]
    if "external-submission" in (row.get("sources") or []):
        return "submitted"          # 外部提交在受理前不是"可动的 todo"（没人认领它）
    return {"fix": "todo", "scheduled": "todo", "fixed": "done",
            "wo-filed": "wo_filed", "boundary": "boundary", "control": "control"}.get(
        row.get("disposition"), "todo")


def cmd_render(rows: list[dict]) -> int:
    """把 `docs/ONBOARDING.md` §0.2 的摘要块按**当前队列**重渲染（幂等）。

    ⚠ 为什么必须有这条子命令（2026-10-12 实测）：`--check` 对"摘要与队列不同数"的判红
    信息写着「跑 `--render` 刷新（**手改文档不算** ✗）」，而 `--render` 当时**根本没定义**
    （`main` 里调 `cmd_render` ⇒ `NameError`）⇒ **唯一入口的刷新路是死的**：任何人加一条队列
    条目都会把 `--check` 弄红，且**没有合规的修法**（手改判红）✗。这里补上实现。
    """
    text = ONBOARDING.read_text(encoding="utf-8")
    if ONB_BEGIN not in text or ONB_END not in text:
        print(f"✗ {_label(ONBOARDING)} 里没有摘要块标记（{ONB_BEGIN} … {ONB_END}）⇒ "
              f"先手工放一对标记，再跑 `--render` ✗", file=sys.stderr)
        return 1
    head, rest = text.split(ONB_BEGIN, 1)
    _, tail = rest.split(ONB_END, 1)
    block = render_onboarding(rows)
    new = head + block + tail
    if new == text:
        print(f"✓ 摘要块已是最新（幂等：{report_line(summary(rows))}）")
        return 0
    ONBOARDING.write_text(new, encoding="utf-8")
    print(f"✓ 已重渲染 {_label(ONBOARDING)} 的摘要块 —— {report_line(summary(rows))}")
    return 0


def cmd_next(rows: list[dict], owner: str) -> int:
    state = fold()
    for row in sched_rows(rows, state):
        cur = state.get(row["id"], {})
        st = effective_state(row, cur)
        if st in TERMINAL_STATES or st in {"umbrella", "blocked", "boundary", "control"}:
            continue
        if st == "in_progress" and cur.get("owner") not in (None, owner):
            continue
        print(f"{row['id']} · {row.get('facet','?')} · {row.get('severity','?')} · "
              f"{st} · owner={cur.get('owner') or row.get('owner') or '-'}")
        print(f"  判据：{row.get('repro','')}")
        if cur.get("parent"):
            print(f"  （拆解自 `{cur['parent']}`；父条目要等所有子任务关掉）")
        return 0
    print("✓ 队列里没有可动的条目（都终态 / 阻塞 / 被别人认领）—— 要么收口，要么去清复审")
    return 0


def source_dispositions() -> dict[str, str | None]:
    """**全量来源的处置权威表**（`id → disposition`，**终态也在**；`None` = 登记了但没处置）。

    ⚠ 为什么单开一条路（2026-10-14 · `queueX` 缺陷 ⑥，**实测假绿**）：`check_plan` 的
    权威表原先只从 `rows`/`source_rows` 建，而那两份是**队列视图**口径 —— 来源加载器
    会把**终态行丢掉**（`load_gaps()` 对 `status=="fixed"` 直接 `continue` ✗）⇒
    「来源终态 + 调度器非终态」这条**反向两层漂移**对**缺口来源永远咬不住**：
    夹具 `/tmp/qfix/root2`（gap `G-999` `status=fixed` + `split` 出子任务 + 调度器 `todo`）
    ⇒ `--check-plan` **exit 0 且完全静默** ✗（真台账同形：`G-110` `in_progress`、
    `G-112` `todo`）。

    **三个来源各自怎么"看得见"**（编号与本文件的来源分节一致；`selftest_source_visibility`
    逐条断言；加来源就照这份清单补一行，别只补一处 ✗）：
      * 来源 ①（`docs/gaps/ledger.jsonl`）：**必须显式要全量** —— `load_gaps()` 默认把
        `status=="fixed"` 丢掉（那是给队列视图用的），而终态正是漂移规则的输入 ✗；
      * 来源 ②（索引）：**只给身份**（`disposition=None`）⇒ 它的处置由 extras 那一行提供；
        没有 extras 行的索引 id 属于「**登记了但没人写处置**」，由 `--check-uncovered`
        ①/①b 判红（**不是**漂移，也不是幽灵 —— 见 `source_history_ids()`）✓；
      * 来源 ③（extras）：**不过滤** ⇒ 全量（处置的唯一出处 ✓）。
    """
    disp: dict[str, str | None] = {}
    for entry in load_extras():                       # 来源 ③：处置的唯一出处（不过滤）
        disp[entry["id"]] = entry["disposition"]
    for entry in load_gaps(include_terminal=True):    # 来源 ①：全 status（含 fixed）
        if entry.get("disposition") is not None:
            disp[entry["id"]] = entry["disposition"]
    for entry in load_index():                        # 来源 ②：只登记身份（处置在 extras）
        disp.setdefault(entry["id"], None)
    return disp


def source_history_ids() -> set[str]:
    """**来源台账里出现过的所有 id**（含 `fixed`/`wo-filed` 等终态、**三个来源都算**）。

    ⚠ 为什么不能只看 `rows`（那是 open 集合）：条目**收口之后**就离开队列了，
    而它的历史事件仍在（`claim`/`advance`），于是每个已修条目都会在收口那一刻被
    `check_plan` 判成"幽灵条目" ✗（实测：`G-110` 标 `fixed` 后立刻红）。
    幽灵判据要咬的是**从来没登记过的 id**（例如并发测试脚本往共享事件表里写的合成 id）。

    ⚠ **来源可见性**（2026-10-14 · `queueX` 缺陷 ⑥）：这里原先漏了**索引**这一支 ⇒
    只登记在索引里（还没写处置）的 id 会被误报成"幽灵（删来源了？）"✗；现在
    三个来源都读（`load_gaps(include_terminal=True)` 见 `source_dispositions()` 的说明），
    「登记了但没处置」交给 `--check-uncovered` 的 ①/①b（那才是它的判据 ✓）。
    """
    out: set[str] = set()
    for loader in (load_extras, lambda: load_gaps(include_terminal=True), load_index):
        try:
            out |= {row["id"] for row in loader()}
        except (UsageError, OSError, KeyError, ValueError):
            pass
    return out


def check_plan(rows: list[dict], source_rows: list[dict] | None = None) -> int:
    """**调度器不变量**：状态与来源不许漂移、收口必须有证据、umbrella 不许空、锁要新鲜。

    ⚠ `source_rows` = **来源台账的行**（`build_queue()` 的产物；`main()` 传的就是它）。
    处置（`disposition`）的**权威是来源**，不是视图 —— 视图里子任务的行曾经被合成行遮蔽
    （见 `sched_rows` 的说明）：`disp` 一旦只从视图取 ⇒「反向两层漂移」对子任务
    **永远咬不住** ✗（2026-10-14 · 缺陷 ②：`F12b2-front-intro-notation` 来源已 `fixed`、
    调度器停在 `todo`，而 `--check` 假绿）。⇒ 这里显式以 `source_rows` 覆盖同 id 的视图处置
    （不传就退回视图；自检的夹具两种形状都造 ✓）。

    ⚠ **来源可见性**（2026-10-14 · 缺陷 ⑥）：`rows`/`source_rows` 都是**队列视图**口径
    ⇒ **终态行可能根本不在里面**（`load_gaps()` 默认把 `status=="fixed"` 丢掉 ✗）⇒
    「来源终态 + 调度器非终态」这条**对缺口来源永远咬不住**（`G-110`/`G-112` 实测假绿）。
    ⇒ 权威表最后由 `source_dispositions()`（**全量**口径，三个来源都读）覆盖一遍 ✓。
    """
    state = fold()
    bad = 0
    known = {r["id"] for r in rows} | source_history_ids()
    disp = {r["id"]: r.get("disposition") for r in rows}
    for _r in (source_rows or []):     # **来源的处置是权威**（覆盖视图里可能被遮蔽的值）
        if _r.get("disposition") is not None:
            disp[_r["id"]] = _r["disposition"]
    # **终态行也要看得见**（缺陷 ⑥）：视图口径会丢终态 ⇒ 权威表用全量来源兜底。
    # `None`（索引登记过、但还没人写处置）不覆盖 —— 那属于覆盖自检 ①/①b，不是漂移 ✓。
    for _id, _d in source_dispositions().items():
        if _d is not None:
            disp[_id] = _d
    legacy_pointers: list[tuple[str, str]] = []      # 存量非祖先指针（⚠ 报告，不判红）
    for ident, cur in sorted(state.items()):
        st = cur.get("state")
        # ⚠ 这里**不划测试命名空间**（2026-10-12 接口 agent 提交 `SUB-ghost-guard-diluted`）：
        # 曾经为了让验证脚本不弄红而跳过 `R-*`/`P-*` —— 那是**放宽守卫** ✗。正解是让那些脚本
        # **别往真台账写**（用 `SOKO_QUEUE_EVENTS=<临时文件>`，见 `EVENTS` 的说明）⇒
        # 幽灵判据保持满强度：任何不在来源里的 id 一律判红 ✓。
        if ident not in known:
            print(f"✗ 事件里的 `{ident}` 不在任何来源里 ⇒ 幽灵条目（删来源了？）✗", file=sys.stderr)
            bad += 1
            continue
        if st == "triage_rejected" and not cur.get("reason"):
            print(f"✗ `{ident}` 是 `triage_rejected` 却没写理由 ⇒ 拒收也要讲清为什么 ✗",
                  file=sys.stderr)
            bad += 1
        if st == "done":
            if not (cur.get("commit") and (cur.get("reading") or cur.get("triage") == "applied")):
                print(f"✗ `{ident}` 标了 done 却没有 `commit`+`reading` ⇒ **没证据的收口** ✗",
                      file=sys.stderr)
                bad += 1
            if disp.get(ident) not in {"fixed", "wo-filed"}:
                print(f"✗ `{ident}` 调度器里 done，来源处置却是 `{disp.get(ident)}` ⇒ "
                      f"**两层漂移**（把来源台账改成 fixed/wo-filed）✗", file=sys.stderr)
                bad += 1
            # **收口指针必须在 main 的历史上**（2026-10-14 · 缺陷 ⑤）：存在性查不出祖先性 ⇒
            # 存量（口径起点**之前**记的）走带日期豁免（只报告）；新增（起点**之后**记的，
            # 含"记录时是祖先、之后被 rebase 甩掉"这种）判红 ✓ —— 见常量块的口径说明。
            _c = cur.get("commit")
            if _c and not commit_is_ancestor(_c):
                _ts = cur.get("advanced_ts") or ""
                # ⚠ 比**完整 UTC 时刻**（事件的 `ts` 就是 UTC 串；只比日期前 10 位会把今天记的
                # 也算成存量 ⇒ 复查空转 ✗，见 `ANCESTRY_GATE_SINCE` 的说明）
                if _ts < ANCESTRY_GATE_SINCE:
                    legacy_pointers.append((ident, _c))
                else:
                    print(f"✗ `{ident}` 的收口 commit `{_c}` **不是 `HEAD` 的祖先**"
                          f"（记录于 {_ts or '无时间戳'} ≥ 口径起点 {ANCESTRY_GATE_SINCE}）⇒ 证据在 main 上"
                          f"**找不到**（被 rebase 甩掉了？）✗ —— 按新历史重开再收口，或改记"
                          f"**真落进 main** 的那个 commit", file=sys.stderr)
                    bad += 1
        # **反向两层漂移**（`SUB-inflight-claims-never-closed`，2026-10-12）：
        # 上面只查了 `done` + 来源不是终态这一个方向；**反方向一直没人查** ✗ ——
        # 实测事故：`r07`/`G-110` 的修复早已落地、来源处置也已是 `fixed`，但事件表里
        # **只有 claim、没有 advance** ⇒ 调度器停在 `in_progress` ⇒ 仍算 **open**：
        # 队列对"还剩什么"**多报**，`--next` 会把**已经做完**的条目推荐出来 ✗。
        # ⇒ 补上这个方向：来源已是终态、调度器却没到终态 ⇒ 判红（并指名怎么收口 ✓）。
        # ⚠ **`submitted` 不豁免**（2026-10-14 · 第三轮独立复核实测）：`37139ca7` 曾在这里
        # 加过 `and st != "submitted"`（**没写理由** ✗）。实测它的**唯一**观察效果就是让
        # 「来源已是终态 × 调度器 `submitted`」这一格**静默**（3 来源 × {fixed,wo-filed}
        # = **6/6 格静默**，而同一时刻 `--next` 照样把那条推出来 —— `submitted` 既不在
        # `TERMINAL_STATES`、也不在 `cmd_next` 跳过的 {umbrella,blocked,boundary,control}
        # 里 ⇒ 正是本规则要咬的「来源终态 + 调度器非终态」）。
        # 正常提交路径**不受影响**：`cmd_submit` 写的处置是 `fix`（非终态）⇒ 这一支根本
        # 进不来（`cmd_triage` 受理/拒收才把处置翻成 `fixed`/`wo-filed`，那时也应该收口 ✓）。
        # ⇒ 网格逐格钉住（`selftest_source_state_grid`）：删掉这个豁免前 6 格静默、删掉后 36 格全红 ✓。
        _d = disp.get(ident)
        if _d in {"fixed", "wo-filed"} and st not in TERMINAL_STATES:
            # **收口指示由表算出来**（同族第三处，见 `closure_steps`）：手写 `--to done`
            # 在 `todo`/`blocked`/`submitted`/`triage_rejected` 上都是**非法迁移**（rc=1）✗
            _want = "done" if _d == "fixed" else "wo_filed"
            _steps = closure_steps(st, _want)
            _hint = (" → ".join(_step_cmd(ident, _s) for _s in _steps) if _steps
                     else "⚠ `TRANSITIONS` 里没有可执行路径 ⇒ 只能手改事件表，并在 commit "
                          "里写明为什么（别再照一条死路指示敲 ✗）")
            if _d == "fixed":
                print(f"✗ `{ident}` 来源处置已是 `fixed`（终态），调度器却停在 `{st}` ⇒ "
                      f"**反向两层漂移**：**修好了没登记** ⇒ 队列多报、`--next` 会把做完的推出来 ✗"
                      f"（收口：{_hint}）",
                      file=sys.stderr)
            else:
                print(f"✗ `{ident}` 来源处置已是 `wo-filed`（终态），调度器却停在 `{st}` ⇒ "
                      f"**反向两层漂移**（收口：{_hint}）", file=sys.stderr)
            bad += 1
        if st == "umbrella":
            kids = cur.get("children", [])
            if not kids:
                print(f"✗ `{ident}` 是 umbrella 却没有子任务 ⇒ 拆解没落地 ✗", file=sys.stderr)
                bad += 1
            elif all(state.get(k, {}).get("state") in TERMINAL_STATES for k in kids):
                print(f"⏳ `{ident}` 的子任务全关了 ⇒ 该收口了（`--advance {ident} --to done "
                      f"--commit … --reading …`）", file=sys.stderr)
        if st == "wo_filed" and not cur.get("reason"):
            print(f"✗ `{ident}` 是 wo_filed 却没写理由 ✗", file=sys.stderr)
            bad += 1
    claims = read_lock()["claims"]
    live = [c for c in claims if claim_is_live(c)[0]]
    stale = [c for c in claims if not claim_is_live(c)[0]]
    if live:
        print(f"⏳ 活跃认领 {len(live)}/{MAX_SLOTS} 核：" +
              "；".join(claim_is_live(c)[1] for c in live), file=sys.stderr)
    for c in stale:
        print(f"⏳ 陈旧认领：{claim_is_live(c)[1]} ⇒ 下一个写者可用 `--force` 接管 ✓",
              file=sys.stderr)
    for c in claims:
        if c.get("id") not in known and c.get("id") not in state:
            print(f"✗ 认领 `{c.get('id')}` 不在队列里 ⇒ 幽灵认领 ✗", file=sys.stderr)
            bad += 1
    if legacy_pointers:
        # **不许静默放过** ✗：存量豁免每次 `--check` 都打出来（条数 + 起点 + 为什么 + 清单）。
        print(f"⚠ 存量收口指针 **{len(legacy_pointers)} 条**不是 `HEAD` 的祖先 ⇒ **豁免**"
              f"（口径起点 {ANCESTRY_GATE_SINCE}，即闸门落地那一刻 = 本地 2026-10-14 01:54）："
              f"它们是 rebase 前的孪生（对象在、内容也落了 "
              f"main，但指针解析不到 main 的历史）⇒ **只报告、不判红**；**新增收口**必须过"
              f"祖先性闸门（`cmd_advance` 硬拒 + 此处复查）✓ —— 清单："
              + " · ".join(f"{i}@{c[:8]}" for i, c in legacy_pointers), file=sys.stderr)
    return bad


# ── 发版节点（栅栏 + 克制 + 防无限 bug 分支）与外部**提交面**（inbox） ─────────
#
# 用户 2026-10-12 两条要求：
#   ①「在克制的前提下，增加有限的发版节点（发版耗时）」+「多 agent 同时收拢」+「发出去的是**可用的完整
#      版本**」+「防止为修发版问题陷入无限的新 bug 分支、远离主线」；
#   ②「主进程新增一个接口，支持外部 agent 调整」⇒ 清账过程中发现的**主线问题**要能**提交**进主线解决。
#
# 落点：两者都长在**同一个队列**上（不新开账本 ✗）——
#   * 发版节点 = 一条 `kind=release` 的条目 + 它的**栅栏规则**（收拢：不许有在飞认领 / 树要干净 /
#     入版条目必须 done 且有证据）；**防无限分支**：同一 blocker 的「试修」上限 = **3**（与仓库
#     "同一处连红 3 次就换招或降级"同源），第 4 次**由脚本拒绝** ✗ ⇒ 只能把它 `--release drop`
#     （带着**已知问题**发，写清理由）—— 这样"修发版 bug"永远有界 ✓。
#   * 外部提交 = 一条 `source=external-submission` 的队列条目 + `docs/inbox/<id>.patch`；外部 agent
#     **只提交不改主线** ✗（提交面是 append-only 的），主流（主会话）`--triage` 决定 applied/deferred/
#     rejected —— 这就是"一次只有一个写者"在跨 agent 场景下的落法 ✓。

INBOX_DIR = ROOT / "docs" / "inbox"
RELEASE_KINDS = ("release", "release-blocker")
MAX_RELEASE_ATTEMPTS = 3          # 与仓库"连红 3 次换招/降级"同源：第 4 次由脚本拒绝 ✗
MIN_ITEMS_PER_RELEASE = 3         # 克制：一次发版至少收 3 条（除非 `--force` 并写理由）
MIN_HOURS_BETWEEN_RELEASES = 12   # 克制：两次发版至少隔 12h（除非 `--force` 并写理由）


def _read_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    out: list[dict] = []
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError as err:
            raise UsageError(f"{_label(path)}:{lineno} 不是合法 JSON：{err}") from err
    return out


def cmd_submit(args: argparse.Namespace) -> int:
    """**外部 agent 的提交面**：只追加，不改主线 ✗ ⇒ 主流 `--triage` 才动代码。"""
    if not args.submit:
        raise UsageError("`--submit` 要配 `--id <新id>`")
    ident = args.id or args.submit
    if not args.why or not args.evidence:
        print("✗ 提交必须带 `--why '<为什么>'` + `--evidence '<判据读数>'`（**没证据的提交不受理** ✗）",
              file=sys.stderr)
        return 1
    # id 是键：来源（extras / 缺口台账 / 索引，**含终态**）+ 事件表里已有的都不许重（**别覆盖** ✗）。
    # ⚠ **与 `cmd_schedule` 的闸门同一口径**（2026-10-14 · 第三轮独立复核实测的同族第二处）：
    # 旧写法只看 `_read_jsonl(EXTRAS)`（只有一个来源、且不看事件表）⇒ 实测
    # gap `G-1 status=fixed` ⇒ `--submit fix --id G-1` **rc=0 接受**（把一个**已关账缺口**的
    # id 重新写进来源 = 覆盖历史 ✗），而同一时刻 `--schedule G-1` **rc=1 拒** ✓
    # —— 同一个病、同一份修法，上一轮只修了一处（AGENTS.md 硬规则 0(b)：横向排查）。
    # ⇒ 改用 `source_dispositions()`（全量口径，三个来源都读）+ `fold()`，与 `cmd_schedule`
    # 的 `taken` **逐字同形**（两处以后一起改，别再只改一处 ✗）。
    taken = set(fold()) | set(source_dispositions())
    if ident in taken:
        print(f"✗ `{ident}` 已经在队列里（id 是键 ⇒ 换一个 id，别覆盖 ✗）", file=sys.stderr)
        return 1
    patch_note = ""
    if args.patch:
        src = Path(args.patch)
        if not src.exists():
            raise UsageError(f"`--patch {args.patch}` 不存在")
        INBOX_DIR.mkdir(parents=True, exist_ok=True)
        dst = INBOX_DIR / f"{ident}.patch"
        dst.write_text(src.read_text(encoding="utf-8"), encoding="utf-8")
        patch_note = f" · patch `{_label(dst)}`（{len(dst.read_text(encoding='utf-8').splitlines())} 行）"
    entry = {"id": ident, "source": "external-submission", "facet": args.facet or "主线",
             "severity": args.severity or "medium",
             "repro": args.repro or "（无复现件：判据见 where）",
             "disposition": "fix", "owner": args.owner or "main",
             "why": f"[{args.submit}] {args.why}",
             "where": args.where or "（未写落点）"}
    with EXTRAS.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
    append_event({"event": "submit", "id": ident, "owner": args.owner or "external",
                  "kind": args.submit, "evidence": args.evidence})
    print(f"✓ 已提交 `{ident}`（{args.submit} · {args.where or '?'}）{patch_note}\n"
          f"  证据：{args.evidence}\n"
          f"  主流受理：`python3 scripts/ux-queue.py --triage {ident} --to applied|deferred|rejected "
          f"--reason '<为什么>' [--commit <sha>]`")
    return 0


def cmd_schedule(args: argparse.Namespace) -> int:
    """**编排者（主进程）给自己排期**：`--schedule <id> --title <做什么> --reason <为什么>`。

    ⚠ 为什么需要（`SUB-queue-registration-gaps` ①，2026-10-13 **真实咬到过**）：上一轮明确判定
    「下一轮要做（有预算空间时）把 L0 减到 1533 以下」，但 `--add-child` 是给**缺口**用的
    （得先有父条目、父转 umbrella）、`--submit` 是给**外部 agent**用的（进"等主流受理"的提交面）
    ⇒ 编排者给自己排的期**没有入口**，只能写进 commit message ⇒ `--json` 的 `open` 里查不到
    ⇒ **下一个 agent 不会知道** ✗。

    形状与 `--submit` **同形**（来源行 + 一条只追加事件），但语义/归属不同：
      * 来源 = `docs/ux-queue-extras.jsonl`（`source=orchestrator-schedule`）⇒ 它进
        `build_queue()` 的 `rows` ⇒ `--json` 的 `open`/`queue`/`view` 查得到、`--next` 推得出 ✓；
      * 处置 = `fix`（= 还没做完）⇒ 与其它条目**同口径**：收口必须 `--commit` + `--reading`，
        `check_plan` 的"两层漂移"判据一视同仁（**没有新豁免** ✗）；
      * 事件 = `schedule`（`fold()` 折成 `todo`；只追加、可重算 ✓）。
    """
    if not args.schedule:
        raise UsageError("`--schedule` 要配 `--id <新id>`")
    ident = args.id or args.schedule
    if not args.title or not args.reason:
        print("✗ 排期必须带 `--title '<做什么>'` + `--reason '<为什么/下一轮的口径>'` "
              "（**没理由的排期 = 又一个查不到的决定** ✗）", file=sys.stderr)
        return 1
    if not QUEUE_ID_RE.fullmatch(ident):
        print(f"✗ id `{ident}` 形状不对：要 `字母开头 + 至少一个 -/_`（例 `MODULE-X-DOWNLOAD`）"
              f"—— id 是键，形状漂了就没法对账 ✗", file=sys.stderr)
        return 1
    # id 是键：来源（extras / 缺口台账 / 索引）+ 事件表里已有的都不许重（**别覆盖** ✗）
    # ⚠ **终态 id 也是已用过的 id**（2026-10-14 · `queueX` 缺陷 ⑥ 的同族）：以前这里用
    # 三个加载器的**默认口径** ⇒ `load_gaps()` 把 `status=="fixed"` 的缺口丢掉 ⇒ 一个已关账
    # 缺口的 id 可以被**重新排期**（id 是键 ⇒ 那是覆盖、会把两条历史搅在一起）✗
    # ⇒ 改用 `source_dispositions()`（全量口径，三个来源都读 ✓）。
    taken = set(fold()) | set(source_dispositions())
    if ident in taken:
        print(f"✗ `{ident}` 已经在队列里（id 是键 ⇒ 换一个 id，别覆盖 ✗）", file=sys.stderr)
        return 1
    entry = {"id": ident, "source": "orchestrator-schedule", "facet": args.facet or "主线",
             "severity": args.severity or "medium",
             "repro": args.repro or "（无复现件：判据见 where）",
             "disposition": "fix", "owner": args.owner,
             "why": f"[schedule] {args.title} —— {args.reason}",
             "where": args.where or "（未写落点）"}
    with EXTRAS.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
    append_event({"event": "schedule", "id": ident, "owner": args.owner,
                  "title": args.title, "reason": args.reason})
    print(f"✓ 已排期 `{ident}`（owner={args.owner} · {entry['facet']} · {entry['severity']}）\n"
          f"  做什么：{args.title}\n  为什么：{args.reason}\n"
          f"  查得到：`python3 scripts/ux-queue.py --json` 的 `open`（或 `--next`）；"
          f"收口与其它条目同口径：`--claim {ident}` … `--advance {ident} --to done "
          f"--commit <sha> --reading '<判据读数>'`")
    return 0


def cmd_inbox(rows: list[dict]) -> int:
    """列出**外部提交**及其状态（submitted / applied / deferred / rejected）。

    ⚠ 判定键是 `sources`（**列表**）而不是 `source`（2026-10-12 实测的 bug）：
    `build_queue()`/`load_extras()` 早就把来源收进了 `sources`（`["extras", "external-submission"]`），
    行里**没有** `source` 键 ⇒ 旧写法 `r.get("source") == "external-submission"` **恒为假**
    ⇒ `--inbox` 永远答「（inbox 空：还没有外部提交）」，而同一时刻 extras 里明明有那些行 ✗
    ⇒ 提交者看不到自己的提交（也无法跟踪受理状态），整条"发现→提交→受理"通道没有回执。
    """
    state = fold()
    subs = [r for r in rows if "external-submission" in (r.get("sources") or [])]
    if not subs:
        print("（inbox 空：还没有外部提交）")
        return 0
    for r in sorted(subs, key=lambda r: r["id"]):
        cur = state.get(r["id"], {})
        st = cur.get("state", "submitted")
        print(f"{r['id']:<14}{st:<11}{r.get('severity','?'):<8}{r.get('where','')}")
        print(f"  为什么：{r.get('why') or r.get('note','')}")
        if cur.get("commit"):
            print(f"  收口：{cur['commit'][:8]} · {cur.get('reading','')}")
        if cur.get("reason"):
            print(f"  处置理由：{cur['reason']}")
    print(f"\n共 {len(subs)} 条提交（`--triage <id> --to applied|deferred|rejected --reason …`）")
    return 0


def cmd_triage(ident: str, to: str, reason: str, commit: str, owner: str) -> int:
    """**主流受理提交**：applied 必须带 commit（证据）；rejected/deferred 必须带理由。"""
    rows = _read_jsonl(EXTRAS)
    if ident not in {r["id"] for r in rows}:
        raise UsageError(f"inbox 里没有 `{ident}`")
    if to == "applied" and not commit:
        print("✗ `applied` 必须带 `--commit <sha>`（**受理 = 已进主线**，没 commit 就是没受理 ✗）",
              file=sys.stderr)
        return 1
    if to in {"rejected", "deferred"} and not reason:
        print(f"✗ `{to}` 必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
        return 1
    # ⚠ 只写事件还不够：**来源处置**才是 open 集合的口径（`OPEN_DISPOSITIONS`）
    # ⇒ 受理/拒收必须同轮把 extras 那行翻成终态，否则这条提交**永远是 open**
    # （`--check-uncovered` 会把它算进"未完成"，而 `--inbox` 又显示已受理 ⇒ 自相矛盾 ✗）。
    want = {"applied": "fixed", "rejected": "wo-filed", "deferred": "fix"}[to]
    # ⚠ 逐行改写（**保住抬头的 `#` 注释**）—— 整文件重排会把 `docs/ux-queue-extras.jsonl`
    # 顶部的说明段（它就是这份台账的协议）整段抹掉 ✗。
    lines = EXTRAS.read_text(encoding="utf-8").splitlines()
    out = []
    for line in lines:
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            row = json.loads(line)
            if row.get("id") == ident:
                row["disposition"] = want
                row["triage"] = {"to": to, "commit": commit or None, "reason": reason or None,
                                 "owner": owner, "ts": _now()}
                line = json.dumps(row, ensure_ascii=False)
        out.append(line)
    EXTRAS.write_text("\n".join(out) + "\n", encoding="utf-8")
    append_event({"event": "triage", "id": ident, "to": to, "reason": reason or None,
                  "commit": commit or None, "owner": owner})
    print(f"✓ `{ident}` ⇒ {to}（来源处置 → `{want}`）"
          + (f" · commit {commit[:8]}" if commit else "")
          + (f" · {reason}" if reason else ""))
    return 0


def worktree_dirty() -> str:
    """工作树脏不脏（发版栅栏要它干净；抽成函数便于 selftest 隔离）。"""
    return subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True,
                          text=True).stdout.strip()


def release_nodes() -> dict[str, dict]:
    """发版节点（`kind=release` 的事件折叠）：{版本: {items, blockers, cut, attempts}}。"""
    out: dict[str, dict] = {}
    for ev in load_events():
        kind = ev.get("event")
        if kind == "release-open":
            # ⚠ **别覆盖已有节点**（2026-10-12 接口 agent 提交 `SUB-release-drop-resets-attempts`）：
            # `release-open` 对同一个版本可以再来一次（重开节点），而 `blockers` 是**试修计数**
            # ⇒ 覆盖 = 把"同一问题最多试修 3 次"的账清零 ⇒ 防无限 bug 分支的守卫形同虚设 ✗。
            prev = out.get(ev["id"]) or {}
            out[ev["id"]] = {"id": ev["id"], "version": ev.get("version"),
                             "items": prev.get("items", []),
                             "blockers": dict(prev.get("blockers", {})),
                             "ts": ev.get("ts"), "cut": prev.get("cut")}
        elif kind == "release-add" and ev.get("rel") in out:
            out[ev["rel"]]["items"].append(ev["id"])
        elif kind == "release-drop" and ev.get("rel") in out:
            node = out[ev["rel"]]
            node["items"] = [i for i in node["items"] if i != ev.get("id")]
            # ⚠ **不动 `blockers`**：`drop` 是"带着已知问题发版"的合法出路，但试修计数是
            # **跨节点累计的历史**（同一问题试修过 3 次就是 3 次）⇒ 把它一起 pop 掉
            # 就等于"drop 一次又能 block 3 次、再 drop、再 3 次……"无限循环 ✗
            #（用户 2026-10-12 明确要求"同一问题最多试修 3 次，第 4 次走 drop 带已知问题发版"）。
        elif kind == "release-block" and ev.get("rel") in out:
            node = out[ev["rel"]]
            node["blockers"][ev["id"]] = node["blockers"].get(ev["id"], 0) + 1
        elif kind == "release-cut" and ev.get("id") in out:
            out[ev["id"]]["cut"] = {"commit": ev.get("commit"), "ci": ev.get("ci"),
                                    "ts": ev.get("ts")}
    return out


def cmd_release(args: argparse.Namespace, rows: list[dict]) -> int:
    state = fold()
    nodes = release_nodes()
    action = args.release
    if action == "open":
        if not args.version:
            raise UsageError("`--release open` 要配 `--version <x.y.z>`")
        cut_nodes = [n for n in nodes.values() if n["cut"]]
        if cut_nodes and not args.force:
            last = max(cut_nodes, key=lambda n: n["cut"]["ts"])
            age_h = (_age_s(last["cut"]["ts"]) or 0) / 3600
            if age_h < MIN_HOURS_BETWEEN_RELEASES:
                print(f"✗ 克制：距上次发版（`{last['id']}`）才 {age_h:.1f}h < "
                      f"{MIN_HOURS_BETWEEN_RELEASES}h ⇒ 攒一攒再发（`--force` + `--reason` 可越过）✗",
                      file=sys.stderr)
                return 1
        ident = f"REL-{args.version}"
        append_event({"event": "release-open", "id": ident, "version": args.version,
                      "owner": args.owner})
        print(f"✓ 开节点 `{ident}`（version {args.version}）—— 栅栏：\n"
              f"   ① 入版条目 `--release add <id>`（**只能加 done 且有证据的**）\n"
              f"   ② 收拢：所有写者停手（认领清零）、工作树干净\n"
              f"   ③ `--release check` 过一遍，再 `--release cut --commit <sha> --ci <run>`")
        return 0
    rel = args.rel or (f"REL-{args.version}" if args.version else "")
    if not rel or rel not in nodes:
        raise UsageError(f"没有这个发版节点：`{rel}`（先 `--release open --version …`）")
    node = nodes[rel]
    if action == "add":
        if not args.id:
            raise UsageError("`--release add` 要配 `--id <条目id>`")
        cur = state.get(args.id, {})
        if cur.get("state") != "done" or not cur.get("commit"):
            print(f"✗ 克制：`{args.id}` 还没 done（或有 done 无证据）⇒ **不许塞进版本** ✗ "
                  f"（宁可下个节点，也别让「修发版 bug」变成无限分支）", file=sys.stderr)
            return 1
        append_event({"event": "release-add", "rel": rel, "id": args.id, "owner": args.owner})
        print(f"✓ `{args.id}` 进 `{rel}`（当前 {len(node['items']) + 1} 条）")
        return 0
    if action == "block":
        if not args.id or not args.reason:
            raise UsageError("`--release block` 要配 `--id` + `--reason`")
        attempts = node["blockers"].get(args.id, 0) + 1
        if attempts > MAX_RELEASE_ATTEMPTS:
            print(f"✗ **防无限分支**：`{args.id}` 已试修 {MAX_RELEASE_ATTEMPTS} 次 ⇒ 脚本拒绝第 "
                  f"{attempts} 次 ✗ —— 按仓库「连红 3 次就换招或降级」：把它 `--release drop {args.id} "
                  f"--reason '<已知问题>'`（带**已知问题**发版）或换思路 ✗", file=sys.stderr)
            return 1
        append_event({"event": "release-block", "rel": rel, "id": args.id,
                      "reason": args.reason, "owner": args.owner})
        print(f"⚠ `{args.id}` 记为**发版阻塞**（第 {attempts}/{MAX_RELEASE_ATTEMPTS} 次试修；"
              f"超限脚本会拒绝 ⇒ 只能 drop 或换招）")
        return 0
    if action == "drop":
        if not args.id or not args.reason:
            raise UsageError("`--release drop` 要配 `--id` + `--reason`（**已知问题要写清**）")
        append_event({"event": "release-drop", "rel": rel, "id": args.id, "reason": args.reason,
                      "owner": args.owner})
        print(f"✓ `{args.id}` 移出 `{rel}`（已知问题：{args.reason}）⇒ 版本照发 ✓")
        return 0
    if action == "check":
        bad = 0
        if not node["items"]:
            print("✗ 空节点不成立：一条 done 都没入版 ⇒ 发出去的不是「可用的完整版本」✗",
                  file=sys.stderr)
            bad += 1
        elif len(node["items"]) < MIN_ITEMS_PER_RELEASE and not args.force:
            print(f"✗ 克制：本节点才 {len(node['items'])} 条 < {MIN_ITEMS_PER_RELEASE} 条 ⇒ "
                  f"再攒攒（`--force --reason …` 可越过）✗", file=sys.stderr)
            bad += 1
        for ident in node["items"]:
            cur = state.get(ident, {})
            if cur.get("state") != "done" or not cur.get("commit"):
                print(f"✗ 入版条目 `{ident}` 未 done 或无证据 ⇒ **发出去就不是完整可用版本** ✗",
                      file=sys.stderr)
                bad += 1
        live = live_claims()
        if live:
            print(f"✗ **收拢未完成**：还有 {len(live)} 个在飞认领 "
                  f"{[c.get('id') for c in live]} ⇒ 等它们收口（或 `--release block` 记录）✗",
                  file=sys.stderr)
            bad += 1
        branches = subprocess.run(["git", "branch", "--no-merged", "main", "--list", "agent/*",
                                   "--format=%(refname:short)"], cwd=ROOT, capture_output=True,
                                  text=True).stdout.split()
        if branches:
            print(f"✗ **并行收拢未完成**：还有 {len(branches)} 条 agent 分支没合回 main "
                  f"{branches} ⇒ `scripts/agent-worktree.sh land <id>`（发版要的是**完整**版本 ✗）",
                  file=sys.stderr)
            bad += 1
        trees = [l.split()[1] for l in subprocess.run(
            ["git", "worktree", "list", "--porcelain"], cwd=ROOT, capture_output=True,
            text=True).stdout.splitlines() if l.startswith("worktree ")]
        extra = [t for t in trees if Path(t).resolve() != ROOT]
        if extra and not args.force:
            print(f"⚠ 还有 {len(extra)} 棵 worktree 在（{[Path(t).name for t in extra]}）—— "
                  f"确认它们的活都合回了再切版（`--force` 越过）", file=sys.stderr)
        dirty = worktree_dirty()
        if dirty:
            print(f"✗ 工作树不干净（{len(dirty.splitlines())} 处）⇒ 先 commit，"
                  f"否则「可用的完整版本」无从谈起 ✗", file=sys.stderr)
            bad += 1
        if bad:
            print(f"\n✗ 发版节点 `{rel}` 未就绪：{bad} 条红", file=sys.stderr)
            return 1
        print(f"✓ 节点 `{rel}` 就绪：{len(node['items'])} 条入版 · 无在飞认领 · 工作树干净 ⇒ "
              f"可以 `--release cut --commit <sha> --ci <run>`（**本地 gate 与 CI 都绿**再切 ✓）")
        return 0
    if action == "cut":
        if not args.commit or not args.ci:
            raise UsageError("`--release cut` 要配 `--commit <sha>` + `--ci <run-id>`（两者都是判据）")
        if cmd_release(argparse.Namespace(**{**vars(args), "release": "check"}), rows) != 0:
            return 1
        append_event({"event": "release-cut", "id": rel, "commit": args.commit,
                      "ci": args.ci, "owner": args.owner})
        print(f"✓ 节点 `{rel}` 切了：commit {args.commit[:8]} · CI {args.ci} ⇒ "
              f"下个节点至少隔 {MIN_HOURS_BETWEEN_RELEASES}h、至少 {MIN_ITEMS_PER_RELEASE} 条 ✓")
        return 0
    raise UsageError(f"未知的 `--release {action}`（open/add/block/drop/check/cut）")


# ── 输出 ───────────────────────────────────────────────────────────────────

def json_view(rows: list[dict], state: dict[str, dict], view: list[dict]) -> dict:
    """`--json` 的**唯一**载荷（`main()` 与 `--selftest` 共用同一份 ⇒ 自检断言的是真输出）。

    ⚠ **单一真相**（2026-10-14 · 缺陷 ③）：`summary`/`open` 一律取 `view`
    （= `sched_rows(rows, state)`）—— `--next`（`cmd_next`）与 §0.2（`main()` → `cmd_render`
    → `render_onboarding`、`--check` → `check_uncovered`）消费的**就是这一份**。
    旧代码只从**源行** `rows` 算 `open`/`summary` ⇒ 实测三个视图打架：
    `--json.open = []` 而 §0.2 的 `QUEUE:COUNTS total=2`、`--next` 还推得出
    `F12b-notation-blind-intro` ✗ ⇒「队列空了吗」**三个视图三个答案**。
    `queue` 仍是**源行**（与 `view` 是两个视角：来源台账 / 调度器视图）；`open` 里的每个 id
    都能在 `view` 里查到它那一行 ✓（没有源行的子任务只出现在 `view`）。
    """
    return {"summary": summary(view), "open": sorted(open_set(view)), "queue": rows,
            "state": state, "view": view}


def print_table(rows: list[dict]) -> None:
    print(f"{'id':<12}{'面':<10}{'严重度':<9}{'处置':<10}{'归属':<12}判据 / 复现")
    for r in rows:
        if r["disposition"] not in OPEN_DISPOSITIONS:
            continue
        print(f"{r['id']:<12}{r['facet']:<10}{r['severity']:<9}{r['disposition']:<10}"
              f"{(r.get('owner') or '-'):<12}{r['repro']}")
    s = summary(rows)
    print(f"\n{report_line(s)} —— 其中 blocker {s['counts']['blocker']} · "
          f"high {s['counts']['high']} · medium {s['counts']['medium']} · low {s['counts']['low']}")
    print(f"（队列总量 {s['queued_all']} 条 = open {s['total']} + "
          f"终态/边界/对照 {s['queued_all'] - s['total']}）")


def selftest_scheduler(expect) -> None:
    """调度器判据自检：**故意造非法形状，每一条都必须被拒** ✗（规则在代码里 ⇒ 必须咬得住）。

    覆盖：迁移合法性 · 收口证据 · 两层漂移 · **4 核槽位** · **资源互斥** · 陈旧认领接管 · `--next` 过滤。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS
    rows = [{"id": f"X-{i}", "disposition": "fix", "repro": "true", "facet": "语言",
             "severity": "medium", "owner": f"mod{i}"} for i in range(1, 6)]

    def quiet(fn, *a, **kw) -> int:
        buf, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
            rc = fn(*a, **kw)
        return rc

    def m_state(ident: str = "X-1") -> str:
        return fold().get(ident, {}).get("state", "todo")

    # ⚠ 收口闸门现在会**核 commit 是否真存在**（`git cat-file -e`，见 `cmd_advance`）
    # ⇒ 自检的夹具必须用**真 sha**（`HEAD`），否则"带证据收口"那条会被新闸门正当拒掉 ✗。
    real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True).stdout.strip()

    with tempfile.TemporaryDirectory() as tmp:
        saved_events, saved_lock = EVENTS, lock_file
        EVENTS = Path(tmp) / "events.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        try:
            # ① 非法迁移：没认领就收口（todo → done）必须被拒
            rc = quiet(cmd_advance, "X-1", "done", "abc123", "读数", "", "t", False, rows)
            expect(f"todo → done 必须被拒（rc={rc}）", rc == 1)
            # ② 认领 → 无证据收口必须被拒；带证据则通过
            expect("认领 X-1", quiet(cmd_claim, "X-1", "t", False, rows, "mod/1") == 0)
            # ③b **反向两层漂移**（`SUB-inflight-claims-never-closed`，2026-10-12）：
            #     来源处置已是 `fixed`、调度器却停在 `in_progress` ⇒ **必须判红** ✓。
            #     单列的理由：上面 ③ 只测了一个方向；**实测事故是反方向** ——
            #     `r07`/`G-110` 的修复早已落地、来源也已 `fixed`，但事件表里只有 claim
            #     没有 advance ⇒ 仍算 open ⇒ 队列**多报**、`--next` 把做完的推出来 ✗。
            #     （此刻 X-1 恰是 `in_progress` ⇒ 正是要测的形状 ✓。）
            x1 = next(r for r in rows if r["id"] == "X-1")
            x1["disposition"] = "fixed"
            expect("来源 fixed 而调度器 in_progress ⇒ 判红", check_plan(rows) >= 1)
            x1["disposition"] = "fix"
            expect("还原后不再误红", check_plan(rows) == 0)
            rc = quiet(cmd_advance, "X-1", "done", "", "", "", "t", False, rows)
            expect(f"无证据收口必须被拒（rc={rc}）", rc == 1)
            rc = quiet(cmd_advance, "X-1", "done", real_sha, "probe exit 0→1", "", "t",
                       False, rows)
            expect(f"带证据收口必须通过（rc={rc}）", rc == 0)
            # ③ 两层漂移：调度器 done 了，来源处置还是 fix ⇒ check_plan 必须判红
            expect(f"done 而来源仍 fix ⇒ 判红", check_plan(rows) >= 1)
            # ④ 终态不许再认领；要重开必须带理由
            rc = quiet(cmd_claim, "X-1", "t", False, rows, "mod/1")
            expect(f"终态再认领必须被拒（rc={rc}）", rc == 1)
            expect("重开不带理由必须被拒",
                   quiet(cmd_advance, "X-1", "todo", "", "", "", "t", False, rows) == 1)
            expect("重开带理由通过",
                   quiet(cmd_advance, "X-1", "todo", "", "", "回归了", "t", False, rows) == 0)
            # ⑤ umbrella：拆解后父不许在子任务没关时收口，且子任务排在父之前
            expect("拆解 X-1 → 子 X-1a",
                   quiet(cmd_add_child, "X-1", "X-1a", "小任务", "t", rows) == 0)
            view = sched_rows(rows, fold())
            ids = [r["id"] for r in view]
            expect(f"子任务排在父之前（{ids[:3]}）", ids.index("X-1a") < ids.index("X-1"))
            expect("父转 umbrella", fold()["X-1"]["state"] == "umbrella")
            rc = quiet(cmd_advance, "X-1", "done", real_sha, "读数", "", "t", True, view)
            expect(f"umbrella 子在开 ⇒ 不许收口（rc={rc}）", rc == 1)
            # ⑥ **4 核槽位**：4 个互不相干的认领都行，第 5 个必须被拒
            write_lock([])
            oks = [quiet(cmd_claim, f"X-{i}", f"w{i}", False, rows, f"mod/{i}")
                   for i in range(1, 5)]
            expect(f"4 核：4 个不相干认领都该成功（{oks}）", all(rc == 0 for rc in oks))
            rc = quiet(cmd_claim, "X-5", "w5", False, rows, "mod/5")
            expect(f"4 核：第 5 个必须被拒（rc={rc}）", rc == 1)
            expect(f"活跃认领恰好 4 条（{len(live_claims())}）", len(live_claims()) == 4)
            # ⑦ **资源互斥**：同一模块同一时间只许一个写者
            write_lock([])
            expect("先占 mod/a", quiet(cmd_claim, "X-1", "w1", False, rows, "mod/a") == 0)
            rc = quiet(cmd_claim, "X-3", "w3", False, rows, "mod/a")
            expect(f"撞同一资源必须被拒（rc={rc}）", rc == 1)
            expect("资源不相干 ⇒ 可并发（4 核之内）",
                   quiet(cmd_claim, "X-2", "w2", False, rows, "mod/b") == 0
                   and len(live_claims()) == 2)
            # ⑧ 陈旧认领（进程已死）不算活跃 ⇒ 可被 --force 接管
            write_lock(read_lock()["claims"] + [          # 追加（别覆盖：w1/w2 还活着）
                {"id": "X-3", "owner": "ghost", "pid": 999999, "resources": ["mod/a"],
                 "ts": _now()}])
            expect("死进程认领不算活跃", all(c["owner"] != "ghost" for c in live_claims()))
            expect("陈旧认领可 --force 接管",
                   quiet(cmd_claim, "X-3", "w3", True, rows, "mod/a") == 0)
            # ⑨ 放锁：owner 的认领全部释放
            expect("释放 w3 的认领", quiet(cmd_unclaim, "w3", "测完") == 0)
            # 在 ⑧ 里 w3 `--force` 接管了 mod/a（把 w1 挤掉）⇒ 释放 w3 后应只剩 w2 的 mod/b
            expect(f"释放后只剩 w2（{len(live_claims())}）",
                   [c["owner"] for c in live_claims()] == ["w2"])
            write_lock([])
            # ⑨b **认领必须是原子的**（接口 agent 提交的 `SUB-claim-not-atomic`，2026-10-12）：
            #   8 路**真并发**抢同一资源 ⇒ 恰好 1 个成功、锁里恰好 1 条。
            #   修复前实测：`success reports = 3 | claims in lock = 1`（超发 + 静默丢认领）✗。
            #   用**子进程**跑真 `--claim`（同一把锁文件）⇒ 这才咬得住"check-then-act"。
            import subprocess as _sp
            lock_path = Path(tmp) / "atomic.lock"
            env = dict(os.environ, SOKO_QUEUE_ROOT=str(ROOT))
            script = (
                "import importlib.util,sys;"
                f"spec=importlib.util.spec_from_file_location('q',{str(Path(__file__).resolve())!r});"
                "m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);"
                f"m.lock_file=lambda: __import__('pathlib').Path({str(lock_path)!r});"
                "sys.exit(0 if m.acquire_claim('R-x', sys.argv[1], ['mod/same'], False) else 1)")
            procs = [_sp.Popen([sys.executable, "-c", script, f"w{i}"],
                               stdout=_sp.DEVNULL, stderr=_sp.DEVNULL, env=env)
                     for i in range(8)]
            oks = sum(1 for p in procs if p.wait() == 0)
            n_claims = len(json.loads(lock_path.read_text(encoding="utf-8"))["claims"])
            expect(f"8 路并发抢同一资源：恰好 1 个成功（实测 {oks}）", oks == 1)
            expect(f"8 路并发后锁里恰好 1 条（实测 {n_claims}）", n_claims == 1)
            # ⑩ **认领必须活过 CLI 调用**（2026-10-12 实测 bug 的反向验证）：`--claim` 写自己的
            # pid ⇒ 进程一退，下一条 `--claim` 就把它当陈旧清掉 ⇒ 连认领 2 条只剩最后 1 条 ✗。
            # 判据：两次 `cmd_claim` 之后**两条都在**（各自独立资源），且锁里**不写 pid**。
            expect("第 1 条 CLI 认领", quiet(cmd_claim, "X-1", "w1", False, rows, "mod/a") == 0)
            expect("第 2 条 CLI 认领", quiet(cmd_claim, "X-2", "w2", False, rows, "mod/b") == 0)
            expect(f"两条 CLI 认领都活着（不是后一条吃掉前一条，实测 {len(live_claims())}）",
                   len(live_claims()) == 2)
            expect("CLI 认领不写 pid（否则下一条会把它当陈旧 ✗）",
                   all("pid" not in c for c in read_lock()["claims"]))
            # ⑪ **收口即放槽**（文档里的标准流程：`--claim X` … `--advance X --to done`）：
            # 收口之后锁里**不许再留着它**，否则这个槽位被自己占死（实测修复前：`--to done`
            # 只在 `frm == in_progress` 时放锁 ⇒ 那条路漏放 ⇒ 槽位不归还 ✗）。
            # 判据断言的是**锁的状态**（不是内部走哪条分支 ⇒ 无论谁改实现，这条都咬得住）。
            EVENTS.write_text("", encoding="utf-8")     # 清事件表：X-1 在这套自检前面已被收口过 ⇒ 否则"已是终态"✗
            # ⚠ 形状有讲究：`cmd_claim` 自己会落一条 claim 事件 ⇒ `fold()` 把状态推成 `in_progress`
            # ⇒ 那条路**新旧代码都放锁**（实测反向验证 rc=0 ⇒ 咬不住 ✗）。真实漏洞在**状态还是
            # `todo` 时收口**（认领了但还没落事件；`todo → wo_filed` 是编排者最常用的一条路）。
            # ⇒ 直接造"锁里有认领、事件表为空"的形状，走 `--to wo_filed`。
            write_lock([{"id": "X-1", "owner": "w1", "resources": ["mod/a"], "ts": _now()}])
            expect(f"锁里有认领、状态仍是 todo（{m_state()}）", m_state() == "todo")
            expect("`todo → wo_filed` 收口",
                   quiet(cmd_advance, "X-1", "wo_filed", "", "", "定案不做", "w1", False, rows) == 0)
            expect(f"**收口后槽位归还**（实测 {len(live_claims())} 条 ⇒ 期望 0）",
                   live_claims() == [])
            write_lock([])
            # ⑧b **收口证据里的 commit 必须真存在**（2026-10-12 接口 agent 提交
            #     `SUB-advance-commit-unchecked`）：假 sha 必须被拒（旧闸门只查非空字符串 ✗）。
            write_lock([])
            EVENTS.write_text("", encoding="utf-8")
            expect("认领 X-1 以测 commit 闸门",
                   quiet(cmd_claim, "X-1", "w9", False, rows, "mod/z") == 0)
            fake = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
            expect(f"假 commit `{fake[:8]}…` 收口必须被拒",
                   quiet(cmd_advance, "X-1", "done", fake, "我编的读数", "", "w9",
                         False, rows) == 1)
            expect("真 commit 收口通过",
                   quiet(cmd_advance, "X-1", "done", real_sha, "读数", "", "w9",
                         False, rows) == 0)
            write_lock([])
            # ⑫ --next 跳过终态 / 对照臂
            nxt = io.StringIO()
            with contextlib.redirect_stdout(nxt):
                cmd_next(view, "t")
            expect("--next 不吐终态/对照",
                   not nxt.getvalue().startswith(("X-2", "X-3")))
        finally:
            EVENTS = saved_events
            globals()["lock_file"] = saved_lock


def selftest_release_inbox(expect) -> None:
    """**外部提交面 + 发版节点**的判据自检：没证据的提交/没 commit 的受理/第 4 次试修都必须被拒 ✗。"""
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INBOX_DIR
    # ⚠ `effective_state` 也要能猴子补丁：本自检里 `SUB-1` 在"提交"之后状态就是
    # `submitted`（triage 分支修好后），而**发版节点**那几条用例要的是"有条目还没 done"
    # 这个形状（与提交面无关）⇒ 那一段临时把状态口径钉成 `todo`，见 ③。
    saved_eff = globals().get("effective_state")
    # 受理闸门要求 `--commit` 真有（`cmd_advance` 那条同理）⇒ 夹具用真 sha（`HEAD`）
    real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True).stdout.strip()
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INBOX_DIR, globals()["worktree_dirty"], globals()["lock_file"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INBOX_DIR = Path(tmp) / "inbox"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["worktree_dirty"] = lambda: ""

        def quiet(fn, *a, **kw) -> int:
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc

        def ns(**kw):
            base = dict(submit="", id="", why="", evidence="", patch="", facet="", severity="",
                        repro="", where="", owner="t", release="", rel="", version="", commit="",
                        ci="", reason="", force=False)
            base.update(kw)
            return argparse.Namespace(**base)

        try:
            # ① 没证据的提交 ⇒ 拒
            expect("提交缺 why/evidence ⇒ 拒",
                   quiet(cmd_submit, ns(submit="fix", id="SUB-1", where="a.rs:1")) == 1)
            # ② 带证据 ⇒ 受理进队，inbox 看得到；triage applied 没 commit ⇒ 拒
            expect("带证据的提交进队",
                   quiet(cmd_submit, ns(submit="fix", id="SUB-1", why="主线 bug", evidence="rc 0→1",
                                        where="crates/front/a.rs:1")) == 0)
            rows = _read_jsonl(EXTRAS)
            expect("提交写进了来源（external-submission）",
                   [r["id"] for r in rows] == ["SUB-1"] and rows[0]["source"] == "external-submission")
            expect("inbox 列得出这条", quiet(cmd_inbox, rows) == 0)
            expect("triage applied 没 commit ⇒ 拒",
                   quiet(cmd_triage, "SUB-1", "applied", "", "", "main") == 1)
            expect("triage applied 带 commit ⇒ 过",
                   quiet(cmd_triage, "SUB-1", "applied", "", real_sha, "main") == 0)
            expect("triage rejected 没理由 ⇒ 拒",
                   quiet(cmd_triage, "SUB-1", "rejected", "", "", "main") == 1)
            # ③ 发版节点：克制（<3 条）、收拢（有在飞认领）、入版必须 done
            # ⚠ 这一段要的是"有条目**还没 done**"这个形状，与提交面无关 ⇒
            #   ① 状态口径钉成 `todo`（`sched_rows` 那条路）；② **清事件表**，让
            #   `fold()` 里 `SUB-1` 也回到 todo —— 否则 ⑥ 之后 `SUB-1` 已是 `done`
            #   （triage 分支修好后才成立）⇒ "入版必须 done"那条**反过来**变红 ✗。
            globals()["effective_state"] = lambda row, cur: "todo"
            EVENTS.write_text("", encoding="utf-8")
            expect("开节点", quiet(cmd_release, ns(release="open", version="9.9.9"), rows) == 0)
            expect("没 done 的条目不许入版",
                   quiet(cmd_release, ns(release="add", version="9.9.9", id="SUB-1"), rows) == 1)
            expect("check：<3 条 ⇒ 拒（克制）",
                   quiet(cmd_release, ns(release="check", version="9.9.9"), rows) == 1)
            expect("check：有在飞认领 ⇒ 拒（收拢未完成）",
                   quiet(cmd_claim, "SUB-1", "t", False, rows, "mod/x") == 0
                   and quiet(cmd_release, ns(release="check", version="9.9.9"), rows) == 1)
            write_lock([])
            expect("check：--force 越过克制但仍要条目 done",
                   quiet(cmd_release, ns(release="check", version="9.9.9", force=True), rows) == 1)
            # ④ **防无限 bug 分支**：同一 blocker 第 4 次试修 ⇒ 脚本拒绝 ✗
            oks = [quiet(cmd_release, ns(release="block", version="9.9.9", id="SUB-1",
                                         reason=f"试修{i}"), rows) for i in range(1, 4)]
            expect(f"前 3 次试修都记下（{oks}）", all(rc == 0 for rc in oks))
            expect("**第 4 次试修必须被拒**（防无限分支）",
                   quiet(cmd_release, ns(release="block", version="9.9.9", id="SUB-1",
                                         reason="再来一次"), rows) == 1)
            expect("drop 带理由 ⇒ 允许（带已知问题发版）",
                   quiet(cmd_release, ns(release="drop", version="9.9.9", id="SUB-1",
                                         reason="已知问题：留待下个节点"), rows) == 0)
            # ⑤ 切版要 commit + CI 两个判据
            try:
                quiet(cmd_release, ns(release="cut", version="9.9.9", commit="cafe", force=True), rows)
                rc_cut = 0
            except UsageError:
                rc_cut = 2
            expect(f"cut 缺 --ci ⇒ 用法错（rc={rc_cut}）", rc_cut == 2)
            # ⑤b **防无限 bug 分支不能被 `drop` 重置**（2026-10-12 接口 agent 提交
            #     `SUB-release-drop-resets-attempts`）：已有 3 次试修 → drop → 再 block
            #     ⇒ **第 4 次仍必须被拒**（计数是跨节点累计的历史 ✗）。
            expect("drop 之后第 4 次 block 仍被拒（计数不许被 drop 清零）",
                   quiet(cmd_release, ns(release="block", version="9.9.9", id="SUB-1",
                                         reason="再来一次"), rows) == 1)
            globals()["effective_state"] = saved_eff
            # 发版用例清过事件表、② 又落过 claim/advance ⇒ ⑥ 要的形状（"提交了但还没受理"）
            # 得**再清一次表**并只落一条 submit 事件（否则残留的 claim 事件会把状态钉在
            # `in_progress` ⇒ 断言读到别人的历史 ✗，实测踩过一次）。
            EVENTS.write_text("", encoding="utf-8")
            append_event({"event": "submit", "id": "SUB-1", "owner": "ext", "kind": "fix",
                          "evidence": "rc 0→1"})
            # ⑥ **受理状态必须可读**（2026-10-12 接口 agent 提交 `SUB-triage-state-unread`）：
            #    旧 `fold()` 没有 submit/triage 分支 ⇒ `--triage applied` 写的事件对状态
            #    **完全不可见**，`--inbox` 永远显示 `todo` ⇒ 四态不可达 ✗。
            #    判据 = 折叠后的状态 + 来源处置（两处都要翻，否则条目永远 open）。
            # ⚠ ② 里已经 triage 过一次（applied）⇒ extras 那行的处置已经是 `fixed`。
            #   要造"还没受理"的形状必须**先写回 `fix`**（这一步本身就是"受理会翻处置"的
            #   反向验证：翻回去 ⇒ 又变成 submitted ✓）。
            write_lock([])
            raw = _read_jsonl(EXTRAS)
            raw[0]["disposition"] = "fix"
            EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in raw),
                              encoding="utf-8")
            rows2, _u2 = build_queue()
            # ⚠ 按 **id 取**，别取 `rows2[0]`：`GAPS_LEDGER` 没被夹具替换 ⇒ 真台账的行
            # 排在前面（实测踩过一次：拿到的是某条 gap 行 ⇒ 断言读的是别人的状态 ✗）。
            _row2 = next(r for r in rows2 if r["id"] == "SUB-1")
            _st2 = fold().get("SUB-1", {})
            expect(f"还没受理 ⇒ 状态 = submitted（不是可动的 todo）"
                   f"[实测 state={_st2.get('state')} sources={_row2.get('sources')}]",
                   effective_state(_row2, _st2) == "submitted")
            expect("受理 applied 后状态 = done",
                   quiet(cmd_triage, "SUB-1", "applied", "", real_sha, "main") == 0
                   and fold()["SUB-1"]["state"] == "done")
            expect("受理后来源处置翻成 fixed（否则条目永远 open ✗）",
                   _read_jsonl(EXTRAS)[0]["disposition"] == "fixed")
            # ⚠ 受理**之后**要重建 rows：triage 会翻来源处置（fix → fixed），
            # 拿受理前的快照去 check_plan 会报"两层漂移"（那是快照陈旧，不是真漂移 ✗）。
            rows3, _u3 = build_queue()
            _bad = check_plan(rows3)
            expect(f"受理过的提交**不要求 reading**（受理证据是 commit）[check_plan 红 {_bad} 条]",
                   _bad == 0)
        finally:
            EVENTS, EXTRAS, INBOX_DIR = saved[0], saved[1], saved[2]
            globals()["effective_state"] = saved_eff
            globals()["worktree_dirty"] = saved[3]
            globals()["lock_file"] = saved[4]


def selftest_schedule(expect) -> None:
    """**编排者给自己排期**的登记入口（`SUB-queue-registration-gaps` ①，2026-10-13）。

    这条缺口**真实咬到过**：上一轮明确判定「下一轮要做（有预算空间时）把 L0 减到 1533 以下」，
    但 `--add-child` 是给**缺口**用的（得先有父条目）、`--submit` 是给**外部 agent**用的
    ⇒ 编排者给自己排的期**没有入口**，只能写进 commit message ⇒ `--json` 的 `open` 里查不到
    ⇒ **下一个 agent 不会知道** ✗。

    判据（缺一条这条缺口就还没补上）：
      ① `--schedule` 之后**真 `--json` 载荷**（`json_view()` —— 与 `main()` 同一条路，不是自检
         自己拼的副本）的 `open` 里**查得到**它、`fold()` 的状态 = `todo`（⇒ `--next` 推得出 ✓）；
      ② 它与其它条目**同口径**：`--claim` → `--advance … --to done --commit … --reading …` 能
         正常收口；且"来源处置没翻"照样触发两层漂移判红（**没有新豁免** ✓）；
      ③ **缺 `--reading` 的收口判红**（证据纪律一视同仁 ✓）；
      ④ 走的是**真 CLI 路径**（`main()` + `sys.argv`）—— 把入口关掉，这条自检必须判红 ✗
         （断言的是"条目真的落进了来源"，不只是 rc ⇒ 只查 rc 咬不住"关掉入口" ✗）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"], sys.argv[:])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        # 最小夹具：缺口台账空 · 索引一行（**没处置 ⇒ 不进 rows**）⇒ `rows` 里只剩排期条目 ✓
        INDEX.write_text(
            "## 2. 🔴 blocker / high\n\n"
            "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"   # **七列**契约
            "|---|---|---|---|---|---|---|\n"
            "| X-1 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash x.sh` | front |  |\n",
            encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                  capture_output=True, text=True).stdout.strip()

        def quiet(fn, *a, **kw) -> int:
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                return fn(*a, **kw)

        def ns(**kw):
            base = dict(schedule="", id="", title="", reason="", owner="orch", where="",
                        facet="", severity="", repro="")
            base.update(kw)
            return argparse.Namespace(**base)

        def snapshot():
            _rows, _u = build_queue()
            _state = fold()
            return _rows, _state, sched_rows(_rows, _state)

        try:
            # ① 缺 `--title`/`--reason` ⇒ 拒（**没理由的排期 = 又一个查不到的决定** ✗）
            expect("排期缺 title/reason ⇒ 拒",
                   quiet(cmd_schedule, ns(schedule="S-1", title="", reason="有理由")) == 1)
            # ② 带 title + reason ⇒ 进队（来源行 + 一条**只追加**事件）
            expect("带 title/reason 的排期 ⇒ 过",
                   quiet(cmd_schedule, ns(schedule="S-1", title="下一轮：下载路径端到端判据",
                                          reason="编排者排期（本轮预算不够）")) == 0)
            expect("只追加事件里留下 schedule（谁/为什么都在）",
                   any(e.get("event") == "schedule" and e.get("id") == "S-1"
                       for e in load_events()))
            # ③ **真 `--json` 载荷**：`open` 查得到 + 状态 `todo`（= "下一个 agent 能知道" ✓）
            rows, state, view = snapshot()
            payload = json_view(rows, state, view)
            expect(f"`--json` 的 open 含排期条目（实测 {payload['open']}）",
                   "S-1" in payload["open"])
            expect(f"`--json` 的状态 = todo（实测 "
                   f"{payload['state'].get('S-1', {}).get('state')}）",
                   payload["state"].get("S-1", {}).get("state") == "todo")
            # ④ `--next` 按序推得出来（真 `cmd_next` + 真 view；夹具里只剩它一条 ⇒ 必须头一个）
            nxt = io.StringIO()
            with contextlib.redirect_stdout(nxt):
                cmd_next(view, "orch")
            _first = (nxt.getvalue().splitlines() or [""])[0]
            expect(f"`--next` 推出排期条目（实测首行 `{_first[:36]}…`）",
                   nxt.getvalue().startswith("S-1"))
            # ⑤ 同口径：`--claim` ⇒ `in_progress`；**缺 `--reading` 的收口必须判红** ✗
            expect("认领排期条目",
                   quiet(cmd_claim, "S-1", "orch", False, view, "scripts/ux-queue.py") == 0)
            expect("缺 `--reading` 的收口 ⇒ 判红（与既有条目同口径 ✓）",
                   quiet(cmd_advance, "S-1", "done", real_sha, "", "", "orch",
                         False, view) == 1)
            # ⑥ 带证据收口 ⇒ 过；**但来源处置没翻 ⇒ 两层漂移照样判红**（排期条目不豁免 ✓）
            expect("带 `--commit`+`--reading` 的收口 ⇒ 过",
                   quiet(cmd_advance, "S-1", "done", real_sha,
                         "ux-queue.py --selftest exit 0", "", "orch", False, view) == 0)
            _rows2, _st2, view2 = snapshot()
            expect("收口后来源处置仍是 fix ⇒ 两层漂移判红（**没有新豁免** ✓）",
                   check_plan(view2, _rows2) >= 1)
            _raw = _read_jsonl(EXTRAS)
            if _raw:                 # 反向验证（入口改成只写事件）时可能没有来源行 ⇒ 别丢栈
                _raw[0]["disposition"] = "fixed"
                EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n"
                                          for r in _raw), encoding="utf-8")
            _rows3, _st3, view3 = snapshot()
            expect("来源翻 fixed 之后 ⇒ check_plan 转绿（与既有条目同一条路 ✓）",
                   check_plan(view3, _rows3) == 0)
            # ⑦ **真 CLI 路径**（反向验证的咬点）：`main()` 读 `sys.argv` ⇒ 关掉入口这条必红 ✗
            sys.argv = ["ux-queue.py", "--schedule", "S-2", "--title", "真 CLI 路径",
                        "--reason", "反向验证：关掉入口必须判红", "--owner", "orch"]
            rc = quiet(main)
            expect(f"真 CLI 路径 `--schedule` ⇒ rc 0（实测 {rc}）", rc == 0)
            expect("真 CLI 路径**真的落进了来源**（只查 rc 咬不住关掉入口 ✗）",
                   "S-2" in {r["id"] for r in _read_jsonl(EXTRAS)})
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]
            sys.argv[:] = saved[5]


def selftest_child_drift(expect) -> None:
    """**子任务视图的三条不变量**（2026-10-14 · `queueW` 修三个视图一致性缺陷 ①②③）。

    真实事故：`F12b-notation-blind-intro` / `F12b2-front-intro-notation` 在
    `docs/ux-queue-extras.jsonl` 里已是 `fixed`，调度器却停在 `todo`（被 reopen 过）——
    「反向两层漂移」本该判红 ✗，但 `sched_rows` 给子任务合成的行硬编码 `disposition="fix"`
    且**先进 `out`** ⇒ 源行被 `id in seen` 丢掉 ⇒ `check_plan` 的 `disp` 取到被遮蔽的 `fix`
    ⇒ **`--check` 假绿**；同时 §0.2 报 2 条、`--json.open` 报 0 条、`--next` 还推得出 ⇒
    「队列空了吗」**三个视图三个答案** ✗。

    夹具走**真生产路径**（`build_queue` + `fold` + `sched_rows` + `check_plan`/`json_view`/
    `render_onboarding`/`cmd_next`，与 `main()` 同一份视图），断言：
      ① 子任务**有源行** ⇒ 视图里就是那一行（真处置 `fixed`，不是合成的 `fix`）；
      ② 源行一条**不丢**（视图 ⊇ 源行）· 视图里**无重复 id** · 子任务仍插在父之前；
      ③ 源 `fixed` + 调度器非终态 ⇒ `check_plan` **判红**（反向两层漂移对子任务咬得住 ✓）；
      ④ **单一真相**：视图里 open 的子任务（含**没有源行**的合成行）必须在 `--json` 载荷的
         `open` 里 —— 还原 `json_view`（只从源行算）这条必红 ✗；
      ⑤ **处置的权威是来源**：视图那一行被遮蔽成 `fix` 时，`check_plan(view, source_rows)`
         照样判红 —— 还原「`disp` 取源处置」这条必红 ✗；
      ⑥ 收口（in_progress → done，真 sha + 读数）后转绿；⑦ 三条视图**读数一致**
         （`--json.open` == 视图 open == §0.2 的 `total` == `--next` 推的那条）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| Z-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash z.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in (
            {"id": "K-1", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash x.sh", "disposition": "fix", "why": "夹具：父条目（umbrella）"},
            {"id": "K-2", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash y.sh", "disposition": "fixed",
             "why": "夹具：**有源行**的子任务（真处置 fixed）"})), encoding="utf-8")
        real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                  capture_output=True, text=True).stdout.strip()

        def quiet(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc

        def snapshot():
            _r, _u = build_queue()
            _s = fold()
            return _r, _s, sched_rows(_r, _s)

        try:
            # 夹具形状 = **真历史**：先 `split`（K-2 有源行是**之后**才登记进 extras 的 ⇒
            # `cmd_add_child` 的"id 已是键"那道闸门挡不住这种真形状 ⇒ 直接落事件 ✓）；
            # K-3 没有源行 ⇒ 走合成行那条路。
            append_event({"event": "split", "id": "K-2", "parent": "K-1",
                          "title": "有源行的子任务", "owner": "t"})
            append_event({"event": "split", "id": "K-3", "parent": "K-1",
                          "title": "没有源行的子任务（合成行）", "owner": "t"})
            rows, state, view = snapshot()
            ids = [r["id"] for r in view]
            k2 = next(r for r in view if r["id"] == "K-2")
            expect(f"① 子任务有源行 ⇒ 视图用**源行**（K-2 处置 = {k2.get('disposition')}，期望 fixed）",
                   k2.get("disposition") == "fixed")
            expect("① 子任务仍插在父**之前**（顺序不变）",
                   ids.index("K-2") < ids.index("K-1") and ids.index("K-3") < ids.index("K-1"))
            expect(f"② 源行一条不丢（视图 {len(ids)} 行 ⊇ 源行 {len(rows)} 行）",
                   {r["id"] for r in rows} <= set(ids))
            expect(f"② 视图里没有重复 id（{len(ids)} 行 / {len(set(ids))} 个 id）",
                   len(ids) == len(set(ids)))
            expect(f"③ 源 `fixed` + 调度器 `{state['K-2']['state']}` ⇒ 反向两层漂移判红",
                   check_plan(view, rows) >= 1)
            payload = json_view(rows, state, view)
            expect(f"④ 合成子任务（K-3 没源行）必须在 `--json` 的 open 里（实测 {payload['open']}）",
                   "K-3" in payload["open"])
            expect("④ `--json` 的 open == 视图的 open 集合（单一真相）",
                   payload["open"] == sorted(open_set(view)))
            shadow = [{**r, "disposition": "fix"} if r["id"] == "K-2" else r for r in view]
            expect("⑤ 视图那一行被遮蔽时，处置仍以**来源**为准 ⇒ 照样判红",
                   check_plan(shadow, rows) >= 1)
            expect("认领 K-2（**认领即 `in_progress`** —— `fold()` 那条口径）",
                   quiet(cmd_claim, "K-2", "t", False, view, "mod/x") == 0)
            expect(f"⑥ K-2 → done（commit {real_sha[:8]} + 读数）",
                   quiet(cmd_advance, "K-2", "done", real_sha, "夹具读数：check_plan 转绿",
                         "", "t", False, view) == 0)
            rows2, state2, view2 = snapshot()
            expect("⑥ 收口后 check_plan 转绿（K-2 done + 来源 fixed）",
                   check_plan(view2, rows2) == 0)
            payload2 = json_view(rows2, state2, view2)
            _s2 = summary(view2)
            _pushable = sorted(
                r["id"] for r in view2
                if effective_state(r, state2.get(r["id"], {}))
                not in TERMINAL_STATES | {"umbrella", "blocked", "boundary", "control"})
            expect(f"⑦ 三视图**同数**：`--json.open`={payload2['open']} · 视图 open="
                   f"{sorted(open_set(view2))} · §0.2 total={_s2['total']}",
                   payload2["open"] == sorted(open_set(view2))
                   and _s2["total"] == len(payload2["open"])
                   and f"QUEUE:COUNTS total={_s2['total']} " in render_onboarding(view2))
            expect(f"⑦ `check_plan` 绿 ⇒ `--next` 推得出的（{_pushable}）都在 `--json.open` 里"
                   f"（**这正是原来坏掉的方向**：--next 推 F12b 而 --json.open 是空的 ✗）",
                   set(_pushable) <= set(payload2["open"]))
            nxt = io.StringIO()
            with contextlib.redirect_stdout(nxt):
                cmd_next(view2, "t")
            _first = (nxt.getvalue().splitlines() or [""])[0]
            expect(f"⑦ `--next` 推的就是 `--json.open` 里那条（实测首行 `{_first[:24]}…`）",
                   _first.startswith("K-3") and _first.split(" · ")[0] in payload2["open"])
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]


def selftest_source_visibility(expect) -> None:
    """**漂移规则的来源可见性**（2026-10-14 · `queueX` 缺陷 ⑥）—— 三个来源逐条断言。

    事故：`check_plan` 的权威表 `disp` 只从 `rows`/`source_rows`（**队列视图**口径）建，
    而 `load_gaps()` 默认把 `status=="fixed"` 丢掉 ✗ ⇒ 「来源终态 + 调度器非终态」这条
    **反向两层漂移对缺口来源永远咬不住**（复核者夹具 `/tmp/qfix/root2`：
    gap `G-999` `status=fixed` + `split` 出子任务 + 调度器 `todo` ⇒ `--check-plan`
    **exit 0 且完全静默** ✗）。夹具按**三个来源各一条终态**造（`G-9` gap `fixed` ·
    `E-9` extras `fixed` · `I-9` 索引 + extras `wo-filed`），断言：
      ① `source_dispositions()`（**全量**口径）里三个来源的终态**都看得见**
         （改前：`G-9` 根本不在表里 ⇒ ① 必红）；
      ② 三条「来源终态 + 调度器非终态」**都判红**、且判红信息**点名**该 id + 「反向两层漂移」
         （`E-9`/`I-9` 是**对照组**：改前也红 ⇒ 说明修的是缺口来源那一支，不是换规则 ✗）；
      ③ **索引可见性**：只在索引里（没处置）的 `I-8` **不算幽灵**（改前误报"不在任何来源里"），
         但 `--check-uncovered` 的 ①/①b 照样判红它 ⇒ **守卫没被放宽** ✓；
      ④ **视图零变化**（爆炸半径）：终态行**仍不进** `build_queue()`/`--json.open` ——
         这条修的是守卫的**输入**，不改队列读数 ✓；
      ⑤ **同族第二处**：`--schedule` 的"id 是键"闸门也看得见**终态** id（已关账缺口的 id
         不许被重新排期 ⇒ 那是覆盖历史 ✗）；
      ⑥ **索引行不许静默丢**：§2–§4 里少一列的数据行 ⇒ 响亮报错（点名行号 + 列数）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING, globals()["lock_file"],
                 globals()["probe_inventory"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        ONBOARDING = Path(tmp) / "ONBOARDING.md"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["probe_inventory"] = lambda: []      # 地面真相不在本夹具的判据里
        INDEX.write_text(
            "## 2. 🔴 blocker / high\n\n"
            "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
            "|---|---|---|---|---|---|---|\n"
            "| I-9 | 语言 | 夹具：索引里的 id（处置在 extras） | 高 | `bash i9.sh` | front |  |\n"
            "| I-8 | 语言 | 夹具：**只在索引里、没有处置** | 高 | `bash i8.sh` | front |  |\n",
            encoding="utf-8")
        EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in (
            {"id": "P-9", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash p.sh", "disposition": "fix", "why": "夹具：父条目（umbrella）"},
            {"id": "E-9", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash e.sh", "disposition": "fixed",
             "why": "夹具（来源③ extras）：终态"},
            {"id": "I-9", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash i9.sh", "disposition": "wo-filed",
             "why": "夹具（来源② 索引的 id）：处置在 extras"})), encoding="utf-8")
        GAPS_LEDGER.write_text("".join(json.dumps(e, ensure_ascii=False) + "\n" for e in (
            {"id": "G-9", "title": "夹具（来源① gap 台账）：已 fixed", "kind": "tooling",
             "severity": "painful", "status": "fixed", "found": "2026-01-01",
             "repro": "docs/gaps/repro/x.sh", "where": {"area": "夹具"}},
            # ⚠ `G-8` 是 ⑤ 的夹具：**已 fixed 且没有任何事件** ⇒ 只有"来源全量口径"看得见它
            # （`fold()` 帮不上忙 —— ⑤ 咬的正是"只靠事件表就漏了终态 id"那一支 ✓）。
            {"id": "G-8", "title": "夹具：已 fixed、无事件（id 不许被重用）", "kind": "tooling",
             "severity": "painful", "status": "fixed", "found": "2026-01-01",
             "repro": "docs/gaps/repro/y.sh", "where": {"area": "夹具"}})), encoding="utf-8")

        def out_err(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, buf.getvalue(), err.getvalue()

        try:
            # 夹具形状 = **真形状**：gap 来源的子任务（`split` ⇒ 调度器 `todo`）；
            # extras 两条各一条 `claim`（⇒ `in_progress`）；`I-8` 只在索引里、另有事件。
            append_event({"event": "split", "id": "G-9", "parent": "P-9",
                          "title": "夹具（来源① gap）：split 出来的子任务", "owner": "t"})
            for ident in ("E-9", "I-9", "I-8"):
                append_event({"event": "claim", "id": ident, "owner": "t"})

            rows, _undecided = build_queue()
            state = fold()
            view = sched_rows(rows, state)

            # ① 三个来源的终态都在**全量权威表**里（改前 G-9 不在 ⇒ 必红）
            fdisp = source_dispositions()
            expect(f"① gap 来源的终态看得见（G-9 → {fdisp.get('G-9')!r}，期望 'fixed'）",
                   fdisp.get("G-9") == "fixed")
            expect(f"① extras 来源的终态看得见（E-9 → {fdisp.get('E-9')!r}，期望 'fixed'）",
                   fdisp.get("E-9") == "fixed")
            expect(f"① 索引来源的身份看得见（I-9 → {fdisp.get('I-9')!r}，期望 'wo-filed'；"
                   f"只在索引里的 I-8 → {fdisp.get('I-8')!r}，期望 None=登记了没处置）",
                   fdisp.get("I-9") == "wo-filed" and "I-8" in fdisp
                   and fdisp.get("I-8") is None)

            # ② 三条「来源终态 + 调度器非终态」都判红（G-9 是**改前静默**的那一支）
            rc, _out, err = out_err(check_plan, view, rows)
            expect(f"② gap 来源：`fixed` + 调度器 `{state['G-9']['state']}` ⇒ 判红"
                   f"（改前 exit 0 且静默 ✗）", "G-9" in err and "反向两层漂移" in err)
            expect("② extras 来源：`fixed` + 调度器 `in_progress` ⇒ 判红（对照组）",
                   "E-9" in err)
            expect("② 索引来源：`wo-filed` + 调度器 `in_progress` ⇒ 判红（对照组）",
                   "I-9" in err)
            expect(f"② 三条都判红、且没有多余的红（实测 {rc} 条；锁/陈旧认领只是 ⏳ 不算红）",
                   rc == 3)

            # ③ 索引里登记过（没处置）⇒ **不是幽灵**；但覆盖自检照样判红它（守卫不放宽）
            expect("③ 索引里登记过的 I-8 不被判成幽灵条目（改前误报）", "幽灵" not in err)
            _s = summary(view)
            ONBOARDING.write_text(
                "## 0.2 下一步\n\n" + render_onboarding(view) + "\n\n"
                "| id | 线 |\n|---|---|\n| P-9 | 夹具 |\n", encoding="utf-8")
            rc2, _out2, err2 = out_err(check_uncovered, view, False, 5, _undecided)
            expect(f"③ 但 `--check-uncovered` 照样判红 I-8（rc={rc2}）+ 点名它",
                   rc2 == 1 and "I-8" in err2 and ("没处置" in err2 or "覆盖漏项" in err2))

            # ④ 视图零变化：终态行**仍不进** `build_queue()` 的 open 视图 —— 这条修的是守卫的
            #    **输入**，不改队列读数 ✓。⚠ 子任务的**合成行**仍跟**调度器状态**走：`G-9` 是
            #    `split` 出来的子任务 ⇒ 合成行处置 `fix` ⇒ 它仍在 open —— 这正是让
            #    「`--json.open` == §0.2 == 视图」保持自洽的那条路（改合成行为终态 ⇒
            #    "open 里没有、`--next` 却推得出" ✗ 破坏 queueW 的三视图不变量）；漂移由
            #    `--check` 判红、按它的收口指示 `--advance … --to done` 后就一起消失 ✓。
            payload = json_view(rows, state, view)
            expect(f"④ 终态行不进 `build_queue()` 的 rows（实测 ids={sorted(r['id'] for r in rows)}）",
                   {r["id"] for r in rows} == {"P-9", "E-9", "I-9"})
            expect(f"④ open/§0.2 仍从**视图**算（{payload['open']} = 合成子任务 G-9 + 父 P-9；"
                   f"total={_s['total']}）",
                   payload["open"] == ["G-9", "P-9"] and _s["total"] == len(payload["open"]) == 2
                   and "QUEUE:COUNTS total=2 " in render_onboarding(view))

            # ⑤ **同族第二处**（AGENTS.md 硬规则 0(b)）：`--schedule` 的"id 是键"闸门也得
            #    看得见**终态** id —— 否则一个已关账缺口的 id 能被重新排期（= 覆盖历史 ✗）。
            #    ⚠ 夹具必须是**没有任何事件**的终态 id（`G-8`）：`G-9` 有 `split` 事件 ⇒
            #    `fold()` 里就有它 ⇒ 旧口径也能拒 ⇒ 这条断言会**空转**（实测踩到过一次 ✗）。
            #    ⚠ 这一条必须在 ⑥ **之前**跑：它要 `source_dispositions()` 能正常读索引 ✓。
            args = argparse.Namespace(schedule="x", id="G-8", title="夹具", reason="夹具",
                                      owner="t", where="", facet="", severity="", repro="")
            rc3, _out3, err3 = out_err(cmd_schedule, args)
            expect(f"⑤ 已 fixed、**无事件**的缺口 id 不许被 `--schedule` 重用（rc={rc3}）",
                   rc3 == 1 and "已经在队列里" in err3)
            # ⑤b **同族第二处**（2026-10-14 · 第三轮复核实测）：`--submit` 的同一闸门
            #     原先只看 `_read_jsonl(EXTRAS)` ⇒ gap `fixed` 的 id **rc=0 被接受** ✗，
            #     而同一时刻 `--schedule` 拒 ✓（同一个病只修了一处）。⇒ 两条来源臂都用
            #     **无事件**的 id 钉住（`G-8` = 缺口 `fixed`；`I-8` = 只在索引里、没处置），
            #     外加**正对照**：一个三来源都没有的新 id 必须仍被接受（防"闸门变成全拒" ✗）。
            def _submit_ns(ident: str) -> argparse.Namespace:
                base = dict(submit="fix", id=ident, why="夹具", evidence="rc 0", patch="",
                            facet="", severity="", repro="", where="夹具", owner="t")
                return argparse.Namespace(**base)

            rc4, _out4, err4 = out_err(cmd_submit, _submit_ns("G-8"))
            expect(f"⑤b 已 fixed、**无事件**的缺口 id 不许被 `--submit` 重用（rc={rc4}；"
                   f"改前 rc=0 静默接受 ✗）", rc4 == 1 and "已经在队列里" in err4)
            rc5, _out5, err5 = out_err(cmd_submit, _submit_ns("I-8"))
            expect(f"⑤b 只在索引里（没处置）的 id 也不许被 `--submit` 重用（rc={rc5}）",
                   rc5 == 1 and "已经在队列里" in err5)
            rc6, _out6, _err6 = out_err(cmd_submit, _submit_ns("SUB-新-9"))
            expect(f"⑤b 正对照：三来源都没有的新 id 仍可提交（rc={rc6}）", rc6 == 0)

            # ⑥ 索引里的数据行少一列 ⇒ 响亮报错（不许静默丢）
            #    ⚠ 夹具里**同时留一条好行**（I-6）—— 否则"一条都没解析出来"那条兜底也会报错，
            #    分不清到底咬住的是"少一列"还是"整表空"（反向验证就说不清了 ✗）。
            INDEX.write_text(
                "## 2. 🔴 blocker / high\n\n"
                "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                "|---|---|---|---|---|---|---|\n"
                "| I-6 | 语言 | 夹具：合法行（必须仍解析得出来） | 高 | `bash i6.sh` | front |  |\n"
                "| I-7 | 语言 | 夹具：少一列 | 高 | `bash i7.sh` | front |\n", encoding="utf-8")
            try:
                load_index()
                expect("⑥ 索引 §2 里少一列的数据行 ⇒ 响亮报错（不许 `continue` 静默丢）", False)
            except UsageError as exc:
                expect(f"⑥ 索引里少一列的数据行 ⇒ 响亮报错（点名 I-7 + 列数）：{exc}",
                       "I-7" in str(exc) and "6 列" in str(exc))
        finally:
            (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING) = saved[:5]
            globals()["lock_file"] = saved[5]
            globals()["probe_inventory"] = saved[6]


# ── 「来源 × 调度器状态」网格（`--selftest` 的 ⑤）的两维 ─────────────────────
# ⚠ **加第四个来源 / 加一个调度器状态只改这里**（格数与自检摘要里的条数都从它算 ✓）——
# 这一处就是"以后不许再只修一处"的那一处（AGENTS.md 硬规则 0(b)）。
GRID_PAIRS = (("GF", "gap", "fixed"), ("GW", "gap", "wo-filed"),
              ("EF", "extras", "fixed"), ("EW", "extras", "wo-filed"),
              ("IF", "索引+extras", "fixed"), ("IW", "索引+extras", "wo-filed"))
GRID_STATES = (("todo", "todo"), ("inprog", "in_progress"), ("umb", "umbrella"),
               ("blk", "blocked"), ("sub", "submitted"), ("trej", "triage_rejected"))
GRID_CELLS = len(GRID_PAIRS) * len(GRID_STATES)   # 36 = 6 × 6（每格 2 条读数 + 4 条形状/对照）


def selftest_source_state_grid(expect) -> None:
    """**来源 × 调度器状态的完整网格**（2026-10-14 · 第三轮独立复核 ⑤）：漏格不许活到自检全绿。

    事故：漂移规则漏了 `submitted` 那一格 —— 复核者的网格夹具实测
    {fixed,wo-filed} × {todo,in_progress,umbrella,blocked} **24/24 判红**，而 `submitted`
    那一列 **6/6 静默**（同一时刻 `--next` 照样把那条推出来 ✗）；病根是规则里写死的
    `and st != "submitted"`。而**旧自检夹具只造两种状态**（`split` ⇒ todo、`claim` ⇒
    in_progress）⇒ **漏格能一路活到自检全绿** ✗（`AGENTS.md` 硬规则 0(b)：收到一条 ⇒
    先问"同类还有哪些"，一次改齐 + 落**可查**的守卫）。

    ⇒ 夹具改成**两维网格**：3 来源（① 缺口台账 / ③ extras / ② 索引+extras）
    × 2 来源终态（`fixed` / `wo-filed`）× 6 调度器状态（todo / in_progress / umbrella /
    blocked / submitted / triage_rejected）= **36 格**，逐格断言：

      ① 判红**并点名**该 id + 「反向两层漂移」（改前 `submitted` 那 6 格静默 ⇒ 这条必红 ✗）；
      ② 判红信息里的**收口指示逐步走一遍**都合法、且落终态（改前 `todo`/`blocked` 的
         `→done` 与 `triage_rejected` 的 `→wo_filed` 是死路 ⇒ 这条必红 ✗）—— 指示由
         `closure_steps()` 按 `TRANSITIONS` 算出，这条断言就是"指示不许是死路"的守卫 ✓；
      ③ **夹具形状对**（36 格状态都造对 + 六个来源终态在全量权威表里看得见）+ **正对照**
         （非终态来源的 6 个子任务**一个都不许红**；红条数恰好 36 ⇒ 没有多余红）
         ⇒ 防"夹具空转"与"守卫放宽"两类假绿 ✓。

    ⚠ 加第四个来源 / 加一个调度器状态时，**改 `grid_pairs`/`grid_states` 一处**即可 ——
    这就是"以后不许再只修一处"的那一处 ✓。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING
    grid_pairs, grid_states = GRID_PAIRS, GRID_STATES
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING, globals()["lock_file"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        ONBOARDING = Path(tmp) / "ONBOARDING.md"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        try:
            gaps: list[dict] = []
            extras: list[dict] = []
            index: list[str] = []
            for prefix, source, disp in grid_pairs:
                for suffix, _st in grid_states:
                    ident = f"{prefix}-{suffix}"
                    if source == "gap":
                        gaps.append({"id": ident, "title": f"夹具 {ident}", "kind": "tooling",
                                     "severity": "painful", "status": disp, "found": "2026-01-01",
                                     "repro": f"docs/gaps/repro/{ident}.sh",
                                     "where": {"area": "夹具"}})
                        continue
                    if source == "索引+extras":
                        index.append(ident)
                    extras.append({"id": ident, "source": "ux-index", "facet": "语言",
                                   "severity": "medium", "repro": f"bash {ident}.sh",
                                   "disposition": disp,
                                   "why": f"夹具（来源 {source}）：终态 `{disp}`"})
                # umbrella 格各要一个子任务（否则「umbrella 却没有子任务」会多一条红 ⇒
                # 红条数就说不清了 ✗）；子任务处置 = `fix`（**非终态**）⇒ 它自己不许红
                # ⇒ 它同时是"守卫没放宽"的**正对照** ✓。
                extras.append({"id": f"{prefix}-umb-c", "source": "ux-index", "facet": "语言",
                               "severity": "medium", "repro": "bash c.sh", "disposition": "fix",
                               "why": "夹具：umbrella 的子任务（非终态 ⇒ 不许红）"})
            GAPS_LEDGER.write_text(
                "".join(json.dumps(e, ensure_ascii=False) + "\n" for e in gaps), encoding="utf-8")
            EXTRAS.write_text(
                "".join(json.dumps(e, ensure_ascii=False) + "\n" for e in extras), encoding="utf-8")
            INDEX.write_text(
                "## 2. 🔴 blocker / high\n\n"
                "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                "|---|---|---|---|---|---|---|\n" + "".join(
                    f"| {i} | 语言 | 夹具（索引身份，处置在 extras） | 高 | `bash {i}.sh` | front |  |\n"
                    for i in index), encoding="utf-8")

            def grid_events(ident: str, suffix: str) -> list[dict]:
                """每格用**真实事件形状**造出目标状态（夹具不许自己拼状态 ✗）。"""
                if suffix == "todo":
                    return [{"event": "claim", "id": ident, "owner": "夹具"},
                            {"event": "release", "id": ident, "owner": "夹具", "reason": "夹具"}]
                if suffix == "inprog":
                    return [{"event": "claim", "id": ident, "owner": "夹具"}]
                if suffix == "umb":
                    return [{"event": "split", "id": f"{ident}-c", "parent": ident,
                             "owner": "夹具", "title": "夹具：umbrella 的子任务"}]
                if suffix == "blk":
                    return [{"event": "advance", "id": ident, "to": "blocked", "owner": "夹具",
                             "reason": "夹具：阻塞"}]
                if suffix == "sub":
                    return [{"event": "submit", "id": ident, "owner": "夹具", "kind": "finding"}]
                return [{"event": "triage", "id": ident, "to": "rejected", "owner": "夹具",
                         "reason": "夹具：拒收"}]

            for prefix, _source, _disp in grid_pairs:
                for suffix, _st in grid_states:
                    for ev in grid_events(f"{prefix}-{suffix}", suffix):
                        append_event(ev)

            rows, _undecided = build_queue()
            state = fold()
            view = sched_rows(rows, state)
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = check_plan(view, rows)
            text = err.getvalue()

            # ③ 夹具形状（**先证明夹具真造出了那 GRID_CELLS 格**，否则下面的断言就是空转 ✗）
            wrong_state = [f"{p}-{s}" for p, _a, _b in grid_pairs for s, want in grid_states
                           if state.get(f"{p}-{s}", {}).get("state") != want]
            expect(f"网格夹具：{GRID_CELLS} 格调度器状态都造对了（造错的：{wrong_state or '无'}）",
                   not wrong_state)
            fdisp = source_dispositions()
            wrong_disp = [f"{p}-{s}" for p, _a, d in grid_pairs for s, _w in grid_states
                          if fdisp.get(f"{p}-{s}") != d]
            expect(f"网格夹具：3 来源 × 2 来源终态在**全量权威表**里都看得见"
                   f"（看不见/不对的：{wrong_disp or '无'}）", not wrong_disp)

            # ①② 逐格：判红点名 + 收口指示逐步可执行
            for prefix, source, disp in grid_pairs:
                for suffix, want in grid_states:
                    ident = f"{prefix}-{suffix}"
                    st = state.get(ident, {}).get("state")
                    red = f"`{ident}`" in text and "反向两层漂移" in text
                    expect(f"网格 {ident}（{source}·`{disp}` × `{want}`）⇒ 判红并点名"
                           f"「反向两层漂移」（改前静默的那 6 格 = `submitted` 列 ✗）", red)
                    steps = re.findall(rf"--advance {re.escape(ident)} --to (\w+)", text)
                    here, walkable = st, bool(steps)
                    for nxt in steps:
                        if nxt not in TRANSITIONS.get(here, set()):
                            walkable = False
                        here = nxt
                    if walkable and here not in TERMINAL_STATES:
                        walkable = False
                    expect(f"网格 {ident}：收口指示逐步可执行、且落终态"
                           f"（实测 `{'→'.join(steps) or '无指示'}`）", walkable)
            expect(f"网格：红条数恰好 {GRID_CELLS}（多一条就是夹具/规则溢出，实测 {rc}）",
                   rc == GRID_CELLS)
            kids = [f"{p}-umb-c" for p, _a, _b in grid_pairs]
            expect(f"网格正对照：非终态来源的 {len(kids)} 个子任务一个都不许红"
                   f"（`fix` 不是终态 ⇒ 规则不该命中）",
                   not any(f"`{k}`" in text for k in kids))
        finally:
            (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING) = saved[:5]
            globals()["lock_file"] = saved[5]


def selftest_ancestry_gate(expect) -> None:
    """**祖先性闸门**（2026-10-14 · `queueW` 缺陷 ⑤）：`cat-file -e` 查不出「指针不是 main 祖先」。

    实测（2026-10-14 量）：65 条 `done` 里 **60 条**的 commit 不是 `HEAD` 的祖先
    （去重后 47 个 sha 里 **32 个**是 rebase 前的孪生）—— 对象在 ✓、内容也落了 main ✓，
    但 `git merge-base --is-ancestor <sha> HEAD` 非零 ⇒ 光查存在性 = **假证据** ✗。
    口径 = **只对新增收口**要求祖先性 + 存量给一条**带日期**的豁免（`ANCESTRY_GATE_SINCE` =
    每次 `--check` 打一行 ⚠ 汇总（条数 + 日期 + 为什么 + 清单）⇒ **不静默** ✓）。断言：
      ① 非祖先 sha（现场 `git commit-tree` 造一个**悬垂** commit：对象真的存在、但不是祖先）
         ⇒ `cmd_advance … --to done` **硬拒**（写侧）；
      ② 同一条 sha + 记录时间 ≥ 口径起点 ⇒ `check_plan` **判红**（读侧复查，防"先写后 rebase"）；
      ③ 同一条 sha 但记录时间**早于**口径起点 ⇒ **不判红**，且 stderr 里必须出现豁免口径
         （"存量" + 口径日期 + 清单）—— **不许静默放过** ✓；
      ④ 真祖先（`HEAD`）⇒ 收口通过、也不判红（别误伤 ✓）。
    ⚠ ① 会用 `git commit-tree` 往仓库里写一个**悬垂 commit 对象**（无 ref、无副作用；gc 会回收）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| Z-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash z.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                  capture_output=True, text=True).stdout.strip()
        # **非祖先指针**：拿 HEAD 的树现场造一个**悬垂** commit（HEAD 是它的父 ⇒ 它是后代，
        # 不是祖先 ⇒ `merge-base --is-ancestor` 必非零；对象真的存在 ⇒ `cat-file -e` 通过）。
        dangling = subprocess.run(
            ["git", "-c", "user.name=ux-queue-selftest", "-c", "user.email=selftest@local",
             "commit-tree", f"{real_sha}^{{tree}}", "-m", "ux-queue selftest: 悬垂 commit"],
            cwd=ROOT, capture_output=True, text=True).stdout.strip()
        if not dangling:     # 造不出来（只读仓库之类）⇒ 退回到"现成的非祖先指针"
            dangling = next((c for c in {e.get("commit") for e in load_events() if e.get("commit")}
                             if not commit_is_ancestor(c)), "")
        expect(f"夹具：非祖先 sha 造得出且对象存在（{dangling[:8]}…）",
               bool(dangling) and not commit_is_ancestor(dangling))
        EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in (
            {"id": "G-1", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash g.sh", "disposition": "fix", "why": "夹具：真祖先那条"},
            {"id": "G-2", "source": "ux-index", "facet": "语言", "severity": "medium",
             "repro": "bash h.sh", "disposition": "fixed",
             "why": "夹具：非祖先指针那条"})), encoding="utf-8")

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, err.getvalue()

        def snapshot():
            _r, _u = build_queue()
            _s = fold()
            return _r, _s, sched_rows(_r, _s)

        try:
            rows, _st, view = snapshot()
            rc, err = capture(cmd_advance, "G-1", "in_progress", "", "", "", "t", False, view)
            expect(f"G-1 → in_progress（前置）rc={rc}", rc == 0)
            rows, _st, view = snapshot()
            rc, err = capture(cmd_advance, "G-1", "done", dangling, "读数：非祖先指针",
                              "", "t", False, view)
            expect(f"① 非祖先 sha 收口 ⇒ **硬拒**（rc={rc}）",
                   rc == 1 and "不是 `HEAD` 的祖先" in err)
            # ⚠ 清事件表再往下走：① 若被**还原掉**（写侧不拦），它会往事件表里落一条
            # `G-1 done + commit=悬垂` ⇒ ② 的 `check_plan` 会因为**别的**红（G-1 两层漂移）
            # 而 ≥1 ⇒ ② 变成**假绿**（实测踩到过：RV-4 里 ② 没报 FAIL ✗）。
            # ⇒ 每一条断言都从**干净的事件表**开始，谁的红就是谁的红。
            EVENTS.write_text("", encoding="utf-8")
            # ② 新增（记录时刻 ≥ 口径起点）却非祖先 ⇒ 读侧判红（写侧本该拦住 ⇒ 复查"先写后 rebase"）
            #    ⚠ 时刻由常量**推**出来（别写字面量：口径起点是 UTC 完整时刻 ⇒ 写死会漂 ✗）。
            import datetime as _dt
            _cut = _dt.datetime.fromisoformat(ANCESTRY_GATE_SINCE)
            append_event({"event": "advance", "id": "G-2", "to": "done", "commit": dangling,
                          "reading": "夹具读数",
                          "ts": (_cut + _dt.timedelta(minutes=1)).isoformat(), "owner": "t"})
            _r2, _s2, _v2 = snapshot()
            rc, err = capture(check_plan, _v2, _r2)
            expect(f"② 新增收口指针非祖先 ⇒ check_plan 判红（红 {rc} 条）", rc >= 1)
            # ③ 存量（记录时刻 < 口径起点）⇒ 豁免：不判红，但**必须打出来**（不静默 ✗）
            EVENTS.write_text("", encoding="utf-8")
            append_event({"event": "advance", "id": "G-2", "to": "done", "commit": dangling,
                          "reading": "夹具读数",
                          "ts": (_cut - _dt.timedelta(minutes=1)).isoformat(), "owner": "t"})
            _r3, _s3, _v3 = snapshot()
            rc, err = capture(check_plan, _v3, _r3)
            expect(f"③ 存量非祖先 ⇒ **不判红**（红 {rc} 条）", rc == 0)
            expect(f"③ 但**不静默**：stderr 里出现豁免口径（存量 + 起点 + 清单）"
                   f"[G-2={'G-2@' in err} / 起点={ANCESTRY_GATE_SINCE in err}]",
                   "存量" in err and ANCESTRY_GATE_SINCE in err and "G-2@" in err)
            # ④ 真祖先（HEAD）⇒ 收口过、也不判红（别误伤 ✓）
            EVENTS.write_text("", encoding="utf-8")
            rows, _st, view = snapshot()
            rc, _e = capture(cmd_advance, "G-1", "in_progress", "", "", "", "t", False, view)
            expect(f"G-1 → in_progress（rc={rc}）", rc == 0)
            rows, _st, view = snapshot()
            rc, err = capture(cmd_advance, "G-1", "done", real_sha, "读数：真祖先", "", "t",
                              False, view)
            expect(f"④ 真祖先（HEAD）⇒ 收口通过（rc={rc}）", rc == 0)
            _raw = _read_jsonl(EXTRAS)      # 收口要把来源处置一并翻成终态（否则那是**两层漂移**）
            for _row in _raw:
                if _row.get("id") == "G-1":
                    _row["disposition"] = "fixed"
            EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in _raw),
                              encoding="utf-8")
            _r4, _s4, _v4 = snapshot()
            rc, err = capture(check_plan, _v4, _r4)
            expect(f"④ 且不误伤（红 {rc} 条）", rc == 0)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]


def cmd_selftest(args: argparse.Namespace) -> int:
    """判据通道自检：**故意造四种形状**，每一种都必须被判红/判绿正确。

    不碰仓库状态（用临时/猴子补丁），秒级。
    """
    global INDEX, EXTRAS, ONBOARDING, GAPS_LEDGER
    import tempfile
    bad = 0

    def expect(desc: str, cond: bool) -> None:
        nonlocal bad
        if not cond:
            bad += 1
            print(f"FAIL {desc}", file=sys.stderr)

    # ⓪ **CLI 契约对账**（2026-10-12 实测的 bug）：`cmd_submit()` 读 `args.why` / `args.repro`，
    #    而 argparse 里从没定义过这两个选项 ⇒ 真 CLI 路径 exit 2 / AttributeError，而**自检全绿**
    #    （它用手工 `Namespace` 绕开了 parser）✗。⇒ 现在就按**真 parser** 对账：
    #    每个 handler 源码里出现的 `args.<字段>` 都必须是 parser 真能给的。
    import inspect
    parser = build_arg_parser()
    known = set()
    for act in parser._actions:                      # noqa: SLF001 - 自检要看真定义
        known.update(act.option_strings)
    for handler in (cmd_submit, cmd_triage, cmd_release, cmd_schedule):
        fields = set(re.findall(r"\bargs\.([A-Za-z_][A-Za-z0-9_]*)",
                                inspect.getsource(handler)))
        unknown = sorted(f for f in fields if f"--{f.replace('_', '-')}" not in known)
        expect(f"`{handler.__name__}` 读的字段都在 argparse 里（缺：{unknown}）", not unknown)
    probe_args = parser.parse_args(["--submit", "fix", "--id", "X", "--why", "w",
                                    "--evidence", "e", "--repro", "bash x.sh"])
    expect("真 parser 接受 `--submit … --why/--evidence/--repro`",
           probe_args.why == "w" and probe_args.evidence == "e" and probe_args.repro == "bash x.sh")

    # ⓪b **探针退出码约定对账**（2026-10-12 接口 agent 提交 `SUB-vim-exit-inverted`）：
    #     `run_probe` 的映射按 `probe_kind` 分流，而**四类探针的约定并不相同** ——
    #     `lang/*.sh` 与 `vim-probe.mjs` 是"exit 0 = 缺口仍在"，`lsp/editor` 是
    #     "exit 0 = 一致（已变）"。旧代码只有 lang 与"其余按 lsp"两套 ⇒ vim 被判反
    #     （缺口仍在 ⇒ `fixed` 假绿；已修 ⇒ `reproducing` 假红）✗。
    #     这里直接对 `judge_rc` 逐档断言（纯函数 ⇒ 不跑真探针也能咬住）。
    for kind, rc, want in (
        ("lang", 0, "reproducing"), ("lang", 1, "fixed"),
        ("vim", 0, "reproducing"), ("vim", 1, "fixed"),        # ← vim 与 lang **同向**
        ("lsp", 0, "fixed"), ("lsp", 1, "reproducing"),
        ("editor", 0, "fixed"), ("editor", 1, "reproducing"),
    ):
        got = judge_rc(kind, rc)[0]
        expect(f"`judge_rc({kind!r}, {rc})` = {got}，期望 {want}", got == want)

    # ⓪c **事件表可重定向**（2026-10-12 接口 agent 提交 `SUB-ghost-guard-diluted` 的正解）：
    #     验证脚本会造合成 id（8 路认领 `R-*` 等），而事件表是**主 checkout 单点**
    #     ⇒ 不重定向就往真台账灌合成条目；而"跳过 R-*/P-*"是**放宽守卫** ✗。
    #     判据：设了 `SOKO_QUEUE_EVENTS` 的子进程必须把事件写进**那个文件**、
    #     真事件表**一字节不动** ✓（用真子进程 ⇒ 咬得住"忘了读这个环境变量"）。
    #     ⚠ 用 `--advance <已知 id> --to blocked`（`--claim` 会拒绝队列里没有的 id ⇒ 写不出事件）。
    import subprocess as _sp2
    real_events = ROOT / "docs" / "ux-queue-events.jsonl"
    before = real_events.read_text(encoding="utf-8") if real_events.exists() else ""
    with tempfile.TemporaryDirectory() as _tmp:
        alt = Path(_tmp) / "alt-events.jsonl"
        env2 = dict(os.environ, SOKO_QUEUE_ROOT=str(ROOT), SOKO_QUEUE_EVENTS=str(alt))
        _sp2.run([sys.executable, str(ROOT / "scripts" / "ux-queue.py"),
                  "--advance", "SUB-probe-build-drift", "--to", "blocked",
                  "--reason", "selftest-redirect-probe", "--owner", "selftest"],
                 capture_output=True, text=True, env=env2, cwd=str(ROOT))
        alt_has = alt.exists() and "selftest-redirect-probe" in alt.read_text(encoding="utf-8")
        after = real_events.read_text(encoding="utf-8") if real_events.exists() else ""
    expect("`SOKO_QUEUE_EVENTS` 生效：事件写进重定向的那个文件", alt_has)
    expect("真事件表**没被写入**（重定向时）", before == after)

    # ① 读数判定：五种处置 × 两种实读
    cases = [
        ({"disposition": "fix"}, "reproducing", True),
        ({"disposition": "fix"}, "fixed", False),
        ({"disposition": "fixed"}, "fixed", True),
        ({"disposition": "fixed"}, "reproducing", False),
        ({"disposition": "scheduled"}, "reproducing", True),
        ({"disposition": "scheduled"}, "fixed", True),
        ({"disposition": "wo-filed"}, "reproducing", True),
        ({"disposition": "boundary"}, "fixed", True),
        ({"disposition": "control"}, "fixed", True),
        ({"disposition": "fix"}, "error", True),
        ({"disposition": "fixed"}, "skipped", True),
    ]
    for row, verdict, want in cases:
        got, _ = reading_ok(row, verdict)
        expect(f"reading_ok({row['disposition']}, {verdict}) = {got}，期望 {want}", got == want)

    # ② §0.2 解析：真文件的**机器入口**必须读得出、且**与队列同数**。
    #    ⚠ 2026-10-13 修订（本轮实测到的**既有红**，不是本轮新增 ✗）：原判据是「§0.2 表里
    #    读得出 ≥1 行 id 形状的行」，而它与「§0.2 陈旧行判据」在**粗粒度线全部收口**之后
    #    互相矛盾 —— `06d7e193` 的 W-3 收口把表里最后一个 id 形状行（`| W-3 |`）摘掉
    #    （**必须摘**：留着就触发 `check_uncovered` 的"§0.2 陈旧行"判红 ✗），表里只剩
    #    `| Q |`（不是 id 形状 ⇒ 不参与机器对账）⇒ 原判据**恒红**：实测 HEAD（`119ba4ab`）
    #    上 `--selftest` = exit 1，唯一 FAIL 就是 `FAIL §0.2 能读出队列行` ✗。
    #    而 `load_onboarding_queue()` 自己的 docstring 写的口径（2026-10-12 修订）是
    #    「§0.2 = **脚本渲染的摘要块** + 粗粒度的线」⇒ 机器入口 = 摘要块。
    #    ⇒ 判据换成对**真文件**断言三件（**不是放宽**：新增的两条原来一个字都没查 ✗）：
    #       (a) 表能解析（形状是契约，抛 UsageError 就判红）；
    #       (b) 表里 id 形状的行都必须在队列里（幽灵行判红 —— 真文件版，原来只在夹具里查）；
    #       (c) 摘要块**必须存在**且**与队列同数**（手改/陈旧 ⇒ 当场判红；原判据对摘要块
    #           **一个字都没查** ✗）。
    try:
        ids = load_onboarding_queue()
        doc_counts = load_onboarding_counts()
        _rows_real, _und_real = build_queue()
        _view_real = sched_rows(_rows_real, fold())
        _known_real = {r["id"] for r in _view_real} | source_history_ids()
        _want_real = {"total": summary(_view_real)["total"], **summary(_view_real)["counts"]}
        _ghosts = sorted(set(ids) - _known_real)
        expect(f"§0.2 表能解析、且表里的行都在队列里（幽灵：{_ghosts}）", not _ghosts)
        expect(f"§0.2 有**渲染摘要块**（实测 {doc_counts}）", doc_counts is not None)
        expect(f"§0.2 摘要与真队列**同数**（文档 {doc_counts} vs 实测 {_want_real}）"
               f"—— 不同数就跑 `--render`（手改文档不算 ✗）", doc_counts == _want_real)
    except UsageError as err:
        expect(f"§0.2 解析不许抛 UsageError：{err}", False)

    # ③ **反向验证**：§0.2 与队列的两头漂移 + 探针覆盖牙，逐条都必须咬得住
    import contextlib
    import io
    with tempfile.TemporaryDirectory() as tmp:
        tmp_onb = Path(tmp) / "ONBOARDING.md"
        tmp_ledger = Path(tmp) / "ledger.jsonl"
        tmp_extras = Path(tmp) / "extras.jsonl"
        tmp_index = Path(tmp) / "index.md"
        tmp_index.write_text(
            "## 2. 🔴 blocker / high\n\n"
            "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"   # **七列**契约
            "|---|---|---|---|---|---|---|\n"
            "| X-1 | 语言 | 造的形状 | 高 | `bash x.sh` | front |  |\n",
            encoding="utf-8")
        tmp_extras.write_text(json.dumps(
            {"id": "X-1", "source": "ux-index", "facet": "语言", "severity": "high",
             "repro": "bash x.sh", "disposition": "scheduled", "why": "夹具"},
            ensure_ascii=False) + "\n", encoding="utf-8")
        tmp_ledger.write_text("", encoding="utf-8")
        tmp_onb.write_text("## 0.2 下一步\n\n| id | 线 |\n|---|---|\n", encoding="utf-8")
        saved = (INDEX, EXTRAS, ONBOARDING, GAPS_LEDGER)
        saved_inv = globals()["probe_inventory"]
        INDEX, EXTRAS, ONBOARDING, GAPS_LEDGER = tmp_index, tmp_extras, tmp_onb, tmp_ledger
        globals()["probe_inventory"] = lambda: []      # 夹具隔离真实探针目录（否则永远红 ✗）
        try:
            rows, undecided = build_queue()
            expect("最小夹具造出 1 条 scheduled", [r["id"] for r in rows] == ["X-1"])
            expect("最小夹具没有未定处置的条目", undecided == [])
            # A. 没有**渲染摘要块** ⇒ 判红（唯一入口与队列脱钩）
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = check_uncovered(rows, live=False, timeout=5, undecided=undecided)
            expect(f"缺渲染块 ⇒ 判红（rc={rc}）", rc == 1)
            expect("判红信息里点名了缺渲染块", ONB_BEGIN in err.getvalue())
            # A2. **`--render` 必须真的能修好它**（2026-10-12 实测：`cmd_render` 当时压根没定义
            #     ⇒ 判红信息让人跑一条**不存在的**子命令 ⇒ 唯一入口的刷新路是死的 ✗）
            tmp_onb.write_text("## 0.2 下一步\n\n" + ONB_BEGIN + "\n" + ONB_END + "\n\n"
                               "| id | 线 |\n|---|---|\n| X-1 | 有行 |\n", encoding="utf-8")
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc_render = cmd_render(rows)
            expect(f"`--render` 跑得通（rc={rc_render}）", rc_render == 0)
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"`--render` 之后覆盖自检转绿（rc={rc}）", rc == 0)
            # A3. 幂等：再跑一次内容不许变
            once = tmp_onb.read_text(encoding="utf-8")
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                cmd_render(rows)
            expect("`--render` 幂等（再跑一次内容不变）",
                   tmp_onb.read_text(encoding="utf-8") == once)
            # B. 补上渲染块 + 行 ⇒ 必须转绿（反向验证的另一半）
            tmp_onb.write_text("## 0.2 下一步\n\n" + render_onboarding(rows) + "\n\n"
                               "| id | 线 |\n|---|---|\n| X-1 | 补上的行 |\n", encoding="utf-8")
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"补上渲染块与行 ⇒ 转绿（rc={rc}）", rc == 0)
            # C. §0.2 里有、队列里没有 ⇒ 幽灵行判红（点名它）
            tmp_onb.write_text("## 0.2 下一步\n\n" + render_onboarding(rows) + "\n\n"
                               "| id | 线 |\n|---|---|\n| X-1 | 有行 |\n| Q-9 | 幽灵 |\n",
                               encoding="utf-8")
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"§0.2 有队列没有的行 ⇒ 判红（rc={rc}）", rc == 1)
            expect("判红信息里点名了幽灵行", "Q-9" in err.getvalue())
            # D. 摘要块**手改**成不同数 ⇒ 判红（手改文档不算 ✗）
            tmp_onb.write_text("## 0.2 下一步\n\n" + render_onboarding(rows).replace(
                "<!-- QUEUE:COUNTS total=1", "<!-- QUEUE:COUNTS total=99") + "\n\n"
                "| id | 线 |\n|---|---|\n| X-1 | 有行 |\n", encoding="utf-8")
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"手改摘要计数 ⇒ 判红（rc={rc}）", rc == 1)
            expect("判红信息里点名了不同数", "不同数" in err.getvalue())
            # E. 探针覆盖牙：地面真相里多一个**没人登记**的探针 ⇒ 判红
            globals()["probe_inventory"] = lambda: ["scripts/ux-probes/lang/zz-未登记.sh"]
            tmp_onb.write_text("## 0.2 下一步\n\n" + render_onboarding(rows) + "\n\n"
                               "| id | 线 |\n|---|---|\n| X-1 | 有行 |\n", encoding="utf-8")
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"未登记探针 ⇒ 判红（rc={rc}）", rc == 1)
            expect("判红信息里点名了那个探针", "zz-未登记" in err.getvalue())
        finally:
            INDEX, EXTRAS, ONBOARDING, GAPS_LEDGER = saved
            globals()["probe_inventory"] = saved_inv

    # ③b 调度器判据（非法迁移 / 无证据 / umbrella / 4 核 / 资源互斥 / --next）
    selftest_scheduler(expect)
    # ③c 提交面 + 发版节点（证据 / 克制 / 收拢 / 防无限分支）
    selftest_release_inbox(expect)
    # ③d **编排者给自己排期**（`SUB-queue-registration-gaps` ①：真 `--json` 的 open / 同口径收口 /
    #     缺 `--reading` 判红 / 真 CLI 路径 —— 关掉入口这条必红 ✗）
    selftest_schedule(expect)
    # ③e **子任务视图**（2026-10-14：源行优先 / 不丢行 / 反向两层漂移咬得住 / `--json` 单一真相 /
    #     视图被遮蔽仍以来源为准 / 三视图同数 —— 还原 ①②③ 任一条都有读数必红 ✗）
    selftest_child_drift(expect)
    # ③f **来源可见性**（2026-10-14：三个来源的终态都要被漂移规则看见 ——
    #     `load_gaps()` 默认丢 `fixed` ⇒ 反向两层漂移对缺口来源永远咬不住 ✗）
    selftest_source_visibility(expect)
    # ③g **来源 × 状态 的完整网格**（2026-10-14 · 第三轮复核 ⑤：旧夹具只造 todo/in_progress
    #     两种状态 ⇒ 「来源终态 × 调度器 submitted」那 6 格静默能一路活到自检全绿 ✗
    #     ⇒ 3 来源 × 2 来源终态 × 6 调度器状态 = 36 格，逐格钉「判红点名 + 收口指示可执行」✓）
    selftest_source_state_grid(expect)
    # ③h **祖先性闸门**（2026-10-14：`cat-file -e` 查不出"不是 main 祖先" ⇒ 新增硬拒 + 读侧复查 +
    #     存量带日期豁免且**不静默** ✗）
    selftest_ancestry_gate(expect)

    # ④ 真来源：三张表 + 台账 + extras 都能解析
    try:
        rows, undecided = build_queue()
        expect("真来源能造出队列（≥30 条）", len(rows) >= 30)
        expect(f"真来源里没有未定处置的条目（实测 {undecided}）", undecided == [])
        for r in rows:
            expect(f"`{r['id']}` 的处置合法", r["disposition"] in DISPOSITIONS)
            expect(f"`{r['id']}` 的严重度合法", r["severity"] in SEVERITY_RANK)
    except UsageError as err:
        expect(f"真来源解析不许抛 UsageError：{err}", False)

    if bad:
        print(f"ux-queue.py selftest: {bad} 条判据不成立 ✗", file=sys.stderr)
        return 1
    print(f"ux-queue.py selftest: {len(cases)} 条读数判定 + §0.2/摘要块解析 + "
          f"**5 条反向验证**（缺渲染块判红 · 补块转绿 · 幽灵行判红 · 手改计数判红 · 未登记探针判红）"
          f" + **调度器 22 条**（非法迁移/无证据收口/两层漂移/重开要理由/umbrella 子在开不许收口/"
          f"**4 核槽位满拒第 5 个**/**同资源互斥**/死进程认领不算活跃且可 `--force` 接管/`--release` 只放自己的/"
          f"**认领活过 CLI 调用（不写 pid）**/**收口即放槽**/--next 跳过终态与对照）"
          f" + **编排者排期 13 条**（缺理由拒/只追加事件/**真 `--json` 的 `open` 含它且状态 todo**/"
          f"`--next` 推得出/`--claim` + 带证据收口/缺 `--reading` 判红/两层漂移**不豁免**/真 CLI 路径落地）"
          f" + **子任务视图 14 条**（源行优先/顺序不变/不丢行/无重复/反向两层漂移咬得住/"
          f"**`--json` 单一真相**/视图被遮蔽仍以来源为准/收口转绿/**三视图同数**/`--next` 子集一致）"
          f" + **来源可见性 17 条**（gap/extras/索引三个来源的终态都进全量权威表/"
          f"三条反向两层漂移都判红且无多余红/索引登记过的 id 不算幽灵但覆盖自检照样判红/"
          f"终态行仍不进 open 视图且 open 与 §0.2 同数/**索引少一列的数据行响亮报错/**"
          f"已关账缺口的 id 不许被重新排期/`--submit` 同族闸门也看得见终态 id + 正对照**）"
          f" + **来源 × 状态网格 {2 * GRID_CELLS + 4} 条**"
          f"（{len(GRID_PAIRS)} 个「来源 × 终态」组合 × {len(GRID_STATES)} 个调度器状态"
          f" = **{GRID_CELLS} 格**："
          f"逐格「判红点名 + 收口指示逐步可执行且落终态」= {2 * GRID_CELLS} 条 + 夹具形状 2 条 + "
          f"红条数恰好 {GRID_CELLS} + 非终态来源的正对照 —— 改前 `submitted` 列 6/6 静默、"
          f"todo/blocked/triage_rejected 的指示是死路 ⇒ 逐格必红 ✗）"
          f" + **祖先性闸门 9 条**（非祖先 sha 收口**硬拒**/新增非祖先判红/存量带日期**豁免且不静默**/"
          f"真祖先不误伤）"
          f" + 真来源解析 全部成立 ✓")
    return 0


def _default_owner() -> str:
    import getpass
    import socket
    return os.environ.get("SOKO_OWNER") or f"{getpass.getuser()}@{socket.gethostname().split('.')[0]}"


def build_arg_parser() -> argparse.ArgumentParser:
    """**唯一的 CLI 契约**（`main()` 与 `--selftest` 共用）。

    ⚠ 为什么抽出来（2026-10-12 实测的 bug）：`cmd_submit()` 读 `args.why` / `args.repro`，
    而 argparse 里**从没定义过**这两个选项 ⇒ 真 CLI 路径 exit 2（`unrecognized arguments`）
    或 AttributeError；`--selftest` 却用手工 `Namespace` 绕开了 parser ⇒ **判据通道全绿** ✗。
    抽成函数之后，"parser 定义"与"handler 读的字段"可以用同一份现实对账（见 selftest 的
    `CLI 契约对账`）。
    """
    parser = argparse.ArgumentParser(
        description="「没做完的」唯一机器可重算队列 + 事件调度器（来源与规则见 docstring）")
    parser.add_argument("--json", action="store_true", help="机器可读输出")
    parser.add_argument("--check-uncovered", action="store_true",
                        help="覆盖自检：四个来源逐条在队列里 + `scheduled` 逐条在 §0.2 里")
    parser.add_argument("--check-plan", action="store_true",
                        help="调度器不变量：证据 / umbrella / 两层漂移 / 锁")
    parser.add_argument("--check", action="store_true", help="= --check-plan + --check-uncovered（快）")
    parser.add_argument("--live", action="store_true",
                        help="连同**读数**一起自检（逐条跑探针，慢）")
    parser.add_argument("--timeout", type=int, default=300, help="单条探针超时秒数（默认 300）")
    parser.add_argument("--selftest", action="store_true", help="判据通道自检（不碰仓库状态）")
    parser.add_argument("--next", action="store_true", help="打印**下一个可动**的条目（调度器给序）")
    parser.add_argument("--render", action="store_true",
                        help="把 §0.2 的摘要块按当前队列**重渲染**（幂等）")
    parser.add_argument("--claim", metavar="ID",
                        help=f"原子认领（**{MAX_SLOTS} 核**+资源互斥：槽位满或撞同一模块 ⇒ 判红 ✗）")
    parser.add_argument("--touch", metavar="RES", default="",
                        help="显式声明要写的资源（逗号分隔，覆盖 owner 的默认映射）")
    parser.add_argument("--advance", metavar="ID", help="状态迁移（**规则在脚本里**）")
    parser.add_argument("--to", metavar="STATE", help=f"迁移目标：{list(SCHED_STATES)}")
    parser.add_argument("--commit", metavar="SHA", help="收口/切版证据①：commit（`--advance --to done` 与 `--release cut` 共用）")
    parser.add_argument("--reading", metavar="TEXT", help="收口证据②：判据读数")
    parser.add_argument("--reason", metavar="TEXT", help="wo_filed / blocked / 重开 / 排期的理由")
    parser.add_argument("--add-child", metavar="PARENT", help="就地拆解：给父条目加一个子任务")
    parser.add_argument("--child", metavar="ID", help="子任务 id（配 --add-child）")
    parser.add_argument("--title", metavar="TEXT", help="标题（配 --add-child / --schedule）")
    parser.add_argument("--schedule", metavar="ID",
                        help="**编排者给自己排期**：登记「我排的下一轮要做的事」"
                             "（来源行 + 只追加事件 ⇒ `--json` 的 open / `--next` 查得到）")
    parser.add_argument("--release", metavar="ACTION",
                        help="发版节点：open/add/block/drop/check/cut（栅栏 + 克制 + 防无限 bug 分支）")
    parser.add_argument("--rel", metavar="ID", help="发版节点 id（默认 REL-<version>）")
    parser.add_argument("--version", metavar="X.Y.Z", help="发版节点的版本号")
    parser.add_argument("--ci", metavar="RUN", help="切版要带 CI run id（判据之一）")
    parser.add_argument("--id", metavar="ID", help="条目 id（配 --release add/block/drop）")
    parser.add_argument("--submit", metavar="KIND",
                        help="**外部提交**：fix/doc/gate/release-blocker（只追加，不改主线 ✗）")
    parser.add_argument("--where", metavar="LOC", help="落点（文件:行 / 台账 id；配 --submit / --schedule）")
    parser.add_argument("--evidence", metavar="TEXT", help="提交的证据（判据读数；**必填**）")
    # ⚠ `--why` / `--repro` 是 `cmd_submit()` 读的字段（`args.why` / `args.repro`），
    # 但 2026-10-12 之前**从没在 argparse 里定义过** ⇒ 真 CLI 路径两种结局都是坏的：
    #   ① `--submit fix --why …` ⇒ argparse `unrecognized arguments` ⇒ exit 2；
    #   ② 省略 `--why` ⇒ 进 cmd_submit 后 `AttributeError`（不是 UsageError）⇒ exit 1 + 栈。
    # 而 `--selftest` 用手工 `Namespace`（`ns(**kw)`）**绕开 argparse** ⇒ 全绿也咬不住 ✗
    # （"咬不住的守卫等于没有"）⇒ 外部提交面对**四棵 worktree 的接口 agent** 是死的。
    parser.add_argument("--why", metavar="TEXT", help="提交的理由（**必填**，配 --submit）")
    parser.add_argument("--repro", metavar="CMD", help="复现/判据命令（配 --submit / --schedule）")
    parser.add_argument("--patch", metavar="PATH", help="提交的补丁文件（拷进 docs/inbox/<id>.patch）")
    parser.add_argument("--inbox", action="store_true", help="列出外部提交及状态")
    parser.add_argument("--triage", metavar="ID", help="主流受理提交：--to applied|deferred|rejected")
    parser.add_argument("--facet", metavar="F", default="", help="提交的面（默认 主线）")
    parser.add_argument("--severity", metavar="S", default="", help="提交的严重度（默认 medium）")
    parser.add_argument("--unclaim", action="store_true", help="放掉自己的认领（槽位归还）")
    parser.add_argument("--owner", default=_default_owner(), help="认领者（默认 $SOKO_OWNER 或 用户@主机）")
    parser.add_argument("--force", action="store_true", help="接管**陈旧**锁（进程已死或超 TTL）")
    return parser


def main() -> int:
    parser = build_arg_parser()
    args = parser.parse_args()
    try:
        _guard_root()
        if args.selftest:
            return cmd_selftest(args)
        rows, undecided = build_queue()
        state = fold()
        view = sched_rows(rows, state)
        if args.claim:
            return cmd_claim(args.claim, args.owner, args.force, view, args.touch)
        if args.advance:
            if not args.to:
                raise UsageError("`--advance` 要配 `--to <state>`")
            return cmd_advance(args.advance, args.to, args.commit or "", args.reading or "",
                               args.reason or "", args.owner, args.force, view)
        if args.add_child:
            if not args.child:
                raise UsageError("`--add-child` 要配 `--child <id>`（建议再给 `--title`）")
            return cmd_add_child(args.add_child, args.child, args.title or "", args.owner, view)
        if args.unclaim:
            return cmd_unclaim(args.owner, args.reason or "")
        if args.release:
            return cmd_release(args, view)
        if args.submit:
            return cmd_submit(args)
        if args.schedule:
            return cmd_schedule(args)
        if args.inbox:
            return cmd_inbox(view)
        if args.triage:
            if not args.to:
                raise UsageError("`--triage` 要配 `--to applied|deferred|rejected`")
            return cmd_triage(args.triage, args.to, args.reason or "", args.commit or "",
                              args.owner)
        if args.next:
            return cmd_next(view, args.owner)
        if args.render:
            return cmd_render(view)
        if args.check_plan or args.check:
            bad = check_plan(view, rows)     # 处置以**来源行**为权威（见 `check_plan` 的说明）
            if args.check:
                bad += check_uncovered(view, live=args.live, timeout=args.timeout,
                                       undecided=undecided)
            if bad:
                print(f"\n✗ 队列不变量：**{bad} 条红**", file=sys.stderr)
                return 1
            print(f"✓ 队列不变量通过：收口都有证据 · umbrella 有子任务 · 两层不漂移 · "
                  f"认领 {len(live_claims())}/{MAX_SLOTS} 核")
            return 0
        if args.check_uncovered:
            return check_uncovered(view, live=args.live, timeout=args.timeout, undecided=undecided)
        if args.json:
            print(json.dumps(json_view(rows, state, view), ensure_ascii=False, indent=2))
            return 0
        print_table(view)
        return 0
    except UsageError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
