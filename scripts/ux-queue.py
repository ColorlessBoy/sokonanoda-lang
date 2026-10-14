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
  ⚠ **第二向原先只写在规格里、没实现**（`#3`，2026-10-14 · 第七次独立复核实测 ✗）：一条
  `scheduled` 而 §0.2 无该行 ⇒ 合并的 `--check` **rc=0 · 从不点名**，而 `--next` 推得出它、
  `--json.open` 含它 ⇒ 三视图打架；`--selftest` 那条臂也没有鉴别力（只断言"补上行 ⇒ 转绿"，
  **删掉那行照样绿** ✗）⇒ 现在两向都有牙，自检有「删行 ⇒ 必红 + 点名」的对照臂 ✓。
* `--check-uncovered --live` —— 再加上**读数自检**：逐条跑探针，实读与「台账写的处置」
  不符 ⇒ 判红 ✗（例：写着 `fix` 的条目探针已经 exit 1 ⇒ 该改成 `fixed` 并记账）。

## 来源（四个，逐条都有 id ⇒ 可对账）

| # | 来源 | 它是什么 | 谁在维护 |
|---|---|---|---|
| ① | `docs/gaps/ledger.jsonl`（**全部 `status`**，含 `fixed`） | 历史缺口台账（126 条） | `scripts/gap.py` |
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
⇒ 判据里的"已关账"**就是这四个**（`CLOSED_DISPOSITIONS`，与上面这句同源）；
`--next` 的"不在做"是 `CLOSED_STATES = {done, wo_filed} ∪ {boundary, control}`。

**权威状态只有一份**（2026-10-14 · 第五轮独立复核）：`effective_state(row, cur)` ——
事件表有该 id ⇒ 事件状态优先；**没有事件**（= 上面那句"删掉事件文件就回到来源台账的处置"，
一条合法操作 ✓）⇒ 由**来源处置**翻译（`DISPOSITION_STATES`）。判据与显示**读同一份**：
`--next` 的可动（`is_pushable`）· `--json.open`/§0.2 的对账 · `--check` 的两层漂移
（`check_plan`，域 = `plan_domain` = **视图 ∪ 事件表** ⇒ **无事件的 id 也进判定** ✗ 不许再
只遍历 `state`）都调它 —— 谁都不许自己再算第三遍 ✗。**两层漂移 = 一条不变量的两个方向**：
来源"已关账" ⟺ 调度器"不在做"（`done`/`wo_filed` 这一向要求来源记的是**工作结论**
`WORK_DONE_DISPOSITIONS` ⇒ 与"关没关账"是两个问题，见常量块 ✓）。

## 用法

```bash
python3 scripts/ux-queue.py                       # 人看的队列表 + 计数
python3 scripts/ux-queue.py --json                # 机器可读（与 §0.2 对账用）
python3 scripts/ux-queue.py --schedule <id> --title '<做什么>' --reason '<为什么/下一轮的口径>'  # id ≤ 64 字符
python3 scripts/ux-queue.py --check-uncovered     # 覆盖自检（源码级，秒级；进 gate --fast）
python3 scripts/ux-queue.py --check-uncovered --live   # 覆盖 + 读数自检（跑探针，慢）
python3 scripts/ux-queue.py --repoint-all --reason '<why>' [--dry-run] [--limit N]  # 回填存量指针
python3 scripts/ux-queue.py --selftest            # 判据通道自检（不碰仓库状态）
```

退出码：0 = 通过 · 1 = 有红（覆盖/读数/§0.2 不一致）· 2 = 用法或环境错误。
⚠ **环境编码是契约的一部分**（`#5a`，2026-10-14 · 第七次独立复核实测 ✗）：本脚本的数据是
**UTF-8**（commit subject 大量中文）⇒ 非 UTF-8 环境（例 `LC_ALL=C PYTHONUTF8=0`）**fail fast**：
**rc=2 + 人话**（说清是环境编码 + 给出 `PYTHONUTF8=1` / `LC_ALL=C.UTF-8` 两条正解），
**不丢栈** ✗（改前是 `UnicodeEncodeError`/`UnicodeDecodeError` 栈 + rc=1 ✗）。
孪生/回填路径的 git 读取另有**显式 UTF-8**（`_patch_id`/`_commit_subject`/`_ancestors_by_subject`）
⇒ 正常环境里内容判定不随 locale 漂 ✓；`main()` 还把任何残留 `UnicodeError` 兜成 rc=2 ✓。
⚠ **id 有长度上限 `ID_MAX_LEN=64`**（`#5b`，同轮实测 ✗：3003 字符的 id 原先 **rc=0 收下**）：
`--schedule`/`--submit`/`--add-child` 三个"生 id 的入口"**共用** `_id_length_problem()` ⇒ 拒 ✓
（真台账在用的最长 id 是 `MODULE-ARTIFACTS-KNIFE2-ARTIFACT-KEY`，37 字符 ⇒ 64 留足余量）。

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

**回填存量收口指针**（2026-10-14 · `RELEASE-EVIDENCE-GATE` · 编排者拍定的口径 ✓）：
上面那条豁免**不是**长期方案 —— 长期豁免 ⇒ 那 60 条证据在 main 上**永远核不了** ✗。
口径 = **两步、顺序不许反** ✗：**① 先把存量回填到 0**（`--repoint`）⇒ **② 再把豁免线改成
「非祖先必须 0 条」的断言**（先收紧会把那 60 条一起拒掉 ✗）。`--repoint`：
```bash
python3 scripts/ux-queue.py --repoint-all --reason '<为什么>' --dry-run   # ① 先过目（不写账）
python3 scripts/ux-queue.py --repoint-all --reason '<为什么>'             # ② 真回填（逐条可回滚）
python3 scripts/ux-queue.py --repoint <id> --commit <sha> --reason '<why>' # 单条、显式指针
```
它是**改账**，不是收口入口：**只追加**一条 `repoint` 事件（`fold()` 覆盖 `commit`，旧 sha 记进
`repointed_from`），**不动只追加账、不动 git 历史、不改旧事件** ✗。
孪生按**三关**自动找（**同 subject → `git patch-id --stable` 一致 → `is-ancestor`** ✓）；
**找不到孪生或有歧义 ⇒ 不硬填** ✗（如实列出 —— 那是真问题）。
新指针与 `--advance`/`--triage`/`--release` **共用** `closure_pointer_problem()`（别自己再写一遍 ✗）。

⚠ **显式指针也过同一条三关**（2026-10-14 · **第七次独立复核实测 ✗ 后补齐**，`#1`）：
`--repoint <id> --commit <sha>` 原先**只**走 `closure_pointer_problem()`（存在性 + 祖先性），
**不比 subject / patch-id** ⇒ 复核者实测 `--repoint G-01 --commit 92ef75e1`（main 的 **M0 根提交**、
内容完全无关）**rc=0**，再 `--check` 打出「非祖先 0 条 ✓ 没有豁免」、`--release add` 也 rc=0 收下
⇒ **证据链可被"指向任意祖先"掏空** ✗。⇒ 显式路与自动路**共用** `twin_gate_problem()`
（三关一处定义 ✓），显式指针过不了三关就**拒**（`#1` 选的是「闸门 + 审计字段」两件都做）。
⚠ 事件里**记 `subject` + `patch_id`（新旧两侧）**：旧事件只有 `{commit, old, reason, owner}` ⇒
**事后无从审计** ✗ ⇒ 现在 `_repoint_event()` 是唯一出处，`check_plan` 还会**复查**
「现在这个指针还是不是当时那个内容」（手改事件表换 `commit` ⇒ 判红 ✓）。

**收口指针必须过祖先性闸门**（2026-10-14 · `queueW` 修三个视图一致性缺陷 ⑤）：`--commit` 不只看
"对象存在"（`git cat-file -e`），还看 `git merge-base --is-ancestor <sha> HEAD` —— 实测 99 条带
指针的 `done` 里 **60 条**的指针是 **rebase 前的孪生**（对象在、内容也落了 main，但**解析不到
main 的历史**）✗。口径 **只对新增收口**要求祖先性（写入口硬拒 + `check_plan` 复查"先写后 rebase"），
**存量**原先给一条**带日期**的豁免（`ANCESTRY_GATE_SINCE`）——
⚠ 该豁免（2026-10-14 同日晚）**已按上面的两步收口删掉**：存量全量回填到 0 之后，血缘口径是
**「非祖先必须 0 条」的断言**，每次 `--check` 都把这条断言打出来（**不许静默** ✓）。

**三个视图必须同数**（2026-10-14 · 同轮 ①②③）：`--json` 的 `open`/`summary`、`--next` 的推序、
§0.2 的 `QUEUE:COUNTS` **一律从 `sched_rows()` 的那份视图算**（`json_view` 不再自己从源行另算 ✗）；
子任务**有源行就用源行**（真处置），`check_plan` 的处置以**来源台账**为权威 ⇒「反向两层漂移」
对子任务也咬得住 ✓。

**一份输入，一份合并**（2026-10-14 · 第五次独立复核的「跨来源处置冲突」✗）：来源只从
`source_rows()`（**全量**：extras 全行 · gap 全 `status` 含 `fixed` · 索引只给身份）读一次，
`build_queue()` 合并它（**「一个 id 只能有一个处置」的闸门就在这个循环里**）⇒ 合并出来的
行**就是权威行**。下游一律读它：`--next`（`cmd_next`）· `--json`（`json_view`）· §0.2
（`render_onboarding`）· `check_plan`（漂移判据）· `source_dispositions()`（派生视图）。
⚠ **闸门必须看得见终态行** ✗：复核者 A/B 实测 —— 同一个 done 事件 + 同一个 extras(`fix`) 行，
**只加一行 gap `status=fixed`**，改前就把 `--check` 的 rc=1 变成 **rc=0 静默** ✗（因为
`build_queue()` 用的 `load_gaps()` 默认口径把那行 `continue` 掉了，而 `check_plan` 的权威表
走另一条全量读法 ⇒ **同一个 `effective_state` 喂两种行、得两个状态**：`--next` 推 `todo`、
`--check` 判 `done`）。而同样冲突但 gap `status=wo-filed` ⇒ **rc=2** ✓ = **闸门有牙、
只是被那个过滤器绕开** ⇒ 终态缺口行**必须**得到同一条判红（rc=2，**不许更松** ✗）。
终态行**留在队列里**（= 上面「处置」一节的"已关账，不许从队列里删除 ✗"）但**不进 open** ✓。

**来源可见性**（2026-10-14 · `queueX` 缺陷 ⑥；第五次复核统一成一份输入）：漂移规则要的是
**来源的终态**，而 `load_gaps()` 的默认口径把 `status=="fixed"` 直接 `continue` ✗ ⇒
「来源终态 `fixed` + 调度器非终态」这条**对缺口来源永远咬不住**（实测：`G-110` gap `fixed`
+ 事件 `in_progress` ⇒ `--check` 静默假绿 ✗；`G-112` gap `fixed` + 事件 `todo` 同形）。
**上一轮的补法**（单开一条全量读法 `source_dispositions()` 并覆盖视图行）把这条修好了，
却造出**两份输入** ⇒ 就是上面那个"喂两种行得两个状态" ✗。⇒ 现在的口径：**全量输入 +
一次合并**（`source_rows()` / `build_queue()`），三个来源**逐条**在 `--selftest`
（`selftest_source_visibility` + `selftest_source_state_grid` 的**冲突轴**）里断言
「终态看得见**且**过同一道冲突闸门」✓ —— 将来加第四个来源就照那份清单补一行，
**不许只补一处** ✗（AGENTS.md 硬规则 0(b)）。

**来源 × 调度器状态的网格**（2026-10-14 · 第三轮复核 ⑤；第五次复核加**冲突轴**）：光
"每个来源都看得见"还不够 —— 规则里**再写死一个状态豁免**（实测：`and st != "submitted"`）
就能让整整**一列 6 格静默**（而 `--next` 照样推得出来），且旧夹具只造 todo/in_progress
两种状态 ⇒ 漏格活到自检全绿 ✗。⇒ 判据是**两维网格**：3 来源 × 2 来源终态 × 6 调度器状态
= 36 格，逐格断「判红点名 + 收口指示**逐步可执行**」（`selftest_source_state_grid`）。
⚠ **网格的轴原先全是"单来源"**（每个 id 只被一个来源登记）⇒ 「**同一个 id 被两个来源
登记、处置冲突**」这一族**结构上看不见** ✗ ⇒ 第五次复核加**冲突轴**（`GRID_CONFLICTS`：
视图处置 × 来源处置 × 三态，**单来源臂做正对照**）。**加来源 / 加状态只改那一处**
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
# **id 长度上限**（`#5b`，2026-10-14 · 第七次独立复核实测 ✗）：`--schedule` 原先只查形状、
# **没有长度上限** ⇒ 实测 **3003 字符**的 id **rc=0 收下**（写进 extras + 事件表 ⇒ 之后每个
# 视图 / 对账 / §0.2 都得背着它）✗。取 **64**：真台账在用的最长 id 是
# `MODULE-ARTIFACTS-KNIFE2-ARTIFACT-KEY`（37 字符）⇒ 留足余量，又挡住"整段话当 id" ✓。
ID_MAX_LEN = 64


def _id_length_problem(ident: str) -> str | None:
    """**id 长度上限**（`#5b`）—— 三个"生 id 的入口"（`--schedule`/`--submit`/`--add-child`）
    **共用这一处** ⇒ 不会"只修了 `--schedule`、漏了另外两处" ✗（`AGENTS.md` 硬规则 0(b)）。

    返回 `None` = 过；否则返回判红正文。
    ⚠ **只闸"生 id 的入口"**：引用**已有** id 的子命令（`--claim`/`--advance`/`--triage`/
    `--repoint`/`--release add`）不需要长度闸门 —— 那些 id 必须在来源/事件表里存在，
    超长 id 自然查不到 ✓。
    """
    if len(ident) > ID_MAX_LEN:
        return (f"✗ id 太长：**{len(ident)} 字符 > 上限 {ID_MAX_LEN}**（`ID_MAX_LEN`）⇒ 拒 ✗ "
                f"—— id 是**键**（要进来源台账 / 事件表 / §0.2 对账 / `--next`），"
                f"不是标题：把说明写进 `--title`/`--reason`/`--why`，id 留短且唯一 ✓")
    return None


def _id_shape_problem(ident: str) -> str | None:
    """`--schedule` 的 id 闸门：`QUEUE_ID_RE` 形状 + `_id_length_problem()`（长度只有一处 ✓）。

    ⚠ **形状这一关只在 `--schedule`**（它的 id 要能被 `QUEUE_ID_RE` 解析 ⇒ 才能进 §0.2 对账）；
    本轮**不扩大**它到 `--submit`/`--add-child` —— 那会顺手改掉外部提交面的既有行为 ✗
    （实测：既有自检的正对照用的是中文 id `SUB-新-9`，形状闸门一扩大就把它拒了 ✗）——
    那两处只加**长度**上限（新的那条规则，共用 ✓）。
    """
    if not QUEUE_ID_RE.fullmatch(ident):
        return (f"✗ id `{ident[:60]}` 形状不对：要 `字母开头 + 至少一个 -/_`（例 `MODULE-X-DOWNLOAD`）"
                f"—— id 是键，形状漂了就没法对账 ✗")
    return _id_length_problem(ident)


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


def _guard_encoding() -> None:
    """**环境编码闸门**（`#5a`，2026-10-14 · 第七次独立复核实测 ✗）：非 UTF-8 环境 ⇒ 契约错。

    实测（`LC_ALL=C PYTHONUTF8=0`，而仓库 subject 大量中文）：
      · 打印中文 ⇒ `UnicodeEncodeError`（stdout 是 US-ASCII）；
      · 孪生搜索里 `git log`/`git show` 用 `text=True`（= `locale.getpreferredencoding()`）
        解码中文 subject ⇒ `UnicodeDecodeError`；
      · 两条都是**未捕获的栈 + rc=1** ✗，而契约里环境错是 **rc=2** ✓。
    口径：本脚本的数据是 **UTF-8**（git 的默认输出编码）⇒ 环境表示不了它就 **fail fast**
    （rc=2 + 人话 + 给出正解），**不许丢栈** ✗。
    ⚠ **判定类**读取**不许**用 `errors="replace"` 掩盖 —— 那会让 subject / patch-id 比对
    **悄悄错**（判定类代码的红线 ✗）：判定类 git 读取只有**一处**（`_git_text()`，
    `errors="surrogateescape"` ⇒ **逐字节保真** ✓，见那里的实测）。
    ⚠ **展示**类例外**只有一处**且必须写明理由：`run_probe()` 的 tail（判定只看 `rc`）。
    ⚠ 与它配套的三件事：① 判定类 git 读取**显式 UTF-8 + surrogateescape**（`_git_text()`）
    ⇒ 既不随 locale 漂 ✓、也不把字节折成 U+FFFD ✓；② `main()` 把 `UnicodeError` 也翻成
    rc=2（兜底：任何残留的编码错都不是栈 ✓）；③ 进事件表的 subject 过 `_subject_text()`
    （孤立代理不能直接进 UTF-8 文件 ⇒ 折成可见转义；**判定**走 `patch_id` ✓）。
    """
    import locale as _locale
    enc = _locale.getpreferredencoding(False) or ""
    if not enc:
        return
    try:
        "中".encode(enc)
    except (UnicodeEncodeError, LookupError):
        raise UsageError(
            f"环境编码不是 UTF-8（`locale.getpreferredencoding()` = `{enc}`）⇒ 本脚本的数据是 "
            f"UTF-8（commit subject 大量中文），在这个环境里读/写都会坏 ✗。"
            f"正解：`PYTHONUTF8=1` 或 `LC_ALL=C.UTF-8`（或 `LANG=…UTF-8`）再跑 ✓") from None


# ── 来源 ①：缺口台账 ────────────────────────────────────────────────────────

def load_gaps(include_terminal: bool = False) -> list[dict]:
    """`docs/gaps/ledger.jsonl` 的行 —— 默认**只给 `status != fixed`**（= **只读消费方**口径）。

    ⚠ **`include_terminal=True` = 全量口径**（`status=="fixed"` 也算）—— **队列与判据的
    唯一输入走这一支**（`source_rows()` ⇒ `build_queue()`）：`fixed` 的缺口虽然已经
    **离开 open 视图**，但它仍是**登记过的一个处置**，两个地方必须看得见它 ✗ ——
      ① 「**一个 id 只能有一个处置**」的冲突闸门（`build_queue()` 的合并）：
         默认口径把它 `continue` 掉 ⇒ 闸门对**终态缺口行完全瞎**（2026-10-14 · 第五次
         独立复核实测：同一个 done 事件 + 同一个 extras(`fix`) 行，**只加一行 gap
         `status=fixed`** 就把 `--check` 的 rc=1 变成 rc=0 静默 ✗，而同样冲突但 gap
         `status=wo-filed` ⇒ rc=2 当场判红 ✓ —— 闸门有牙，只是被这个过滤器绕开了）；
      ② 「来源终态 + 调度器非终态」的**反向两层漂移**（`check_plan`）：`G-110` gap
         `fixed` + 事件 `in_progress` ⇒ 默认口径下静默假绿 ✗（`queueX` 缺陷 ⑥）。
    两个口径的**处置**都不许改（`fixed` ⇒ `"fixed"`，与 extras 的字段同名同义 ✓）。

    ⚠ **默认口径只剩"只要 open 的只读消费方"**（`--check-uncovered` 的覆盖 ①）：它问的是
    "**还有哪条没进队列**"，终态行不进 open ⇒ 不需要它们 ✓。**队列/判据一律不许用它** ✗
    —— 那正是"两份输入"的病根（见 `source_rows()` 的说明）。
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


def source_rows() -> list[dict]:
    """**全量来源输入 —— 唯一的一处**（三个加载器各读一次，终态一行不丢）。

    ⚠ **为什么必须有这个名字**（2026-10-14 · 第五次独立复核实测的「跨来源处置冲突」✗）：
    在这之前，同一个 id 会**被两处分别读出来**，而两处的过滤器不同 ⇒ 同一个
    `effective_state()` 喂两种行、得两个状态 ✗：

      | 读法 | 谁在用 | 缺口 `status=fixed` 的行 |
      |---|---|---|
      | `load_gaps()`（**默认**，丢 `fixed`） | `build_queue()` 的**合并**（= 「一个 id 只能有一个处置」的闸门 + `--next`/`--json` 的视图行） | **看不见** ✗ |
      | `load_gaps(include_terminal=True)` | `check_plan` 的处置权威表（`source_dispositions()`，**覆盖**视图那一行） | 看得见 ⇒ 权威 `fixed` ⇒ 派生 `done` ✓ |

      ⇒ 视图行说 `fix`（`--next` 推它、`--json.open` 含它），权威行说 `fixed`
      （`--check` 判它 `done` ⇒ 两条漂移规则都放过）—— **同一个 id、两个状态、两种绿** ✗。
      而**冲突闸门只长在合并里** ⇒ 它对终态缺口行**完全瞎**：复核者的 A/B 实测，
      同一个 done 事件 + 同一个 extras(`fix`) 行，**只加一行 `status=fixed` 的缺口行**
      就把 rc=1 变成 rc=0 静默 ✗（gap `wo-filed` 的同样冲突 ⇒ rc=2 ✓ = 闸门有牙，
      只是被默认口径绕开）。

    ⇒ 口径：**合并（`build_queue()`）走全量输入**，于是
      ① 冲突闸门对三个来源的**每一行**都看得见（终态行也一样 ⇒ rc=2，与 `wo-filed`
         的对照**对称**，不许更松 ✗）；
      ② 合并出来的那一行**就是权威行** ⇒ `--next`/`--json`/§0.2/`check_plan` 读的是
         **同一份**（`source_dispositions()` 也改成由它派生 ⇒ 不再有第二次读来源 ✗）。
    """
    return load_extras() + load_gaps(include_terminal=True) + load_index()


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
    ⚠ **哪些条目必须在 §0.2 里有行**（`#3`，2026-10-14 · 第七次独立复核实测 ✗ 后明确）：
    只有处置 = `scheduled`（= **排期**，见 docstring 的「处置」一节）的条目 ——
    上面那句"不再逐条列 open"说的是 `fix`（细粒度队列）**不要求**进 §0.2 ✓。
    两个方向都由 `check_uncovered` 判：§0.2 有而队列没有 ⇒ 幽灵行 ✗；队列是 `scheduled`
    而 §0.2 没行 ⇒ 缺行 ✗（旧实现**只有前一向** ⇒ 缺行静默而 `--next` 照样推 ✗）。
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

    ⚠ **输入 = `source_rows()`（全量，终态一行不丢）**（2026-10-14 · 第五次独立复核）：
    这里曾经写 `load_extras() + load_gaps() + load_index()` —— `load_gaps()` 的**默认口径**
    把 `status=="fixed"` 的缺口行丢掉，而**「一个 id 只能有一个处置」的闸门就长在这个循环里**
    ⇒ 闸门对**终态缺口行完全瞎** ✗（实测：同一个 done 事件 + 同一个 extras(`fix`) 行，
    只加一行 gap `status=fixed` ⇒ `--check` 从 rc=1 变 rc=0 静默；而同样冲突但 gap
    `status=wo-filed` ⇒ rc=2 —— 闸门有牙，只是被这个过滤器绕开）。
    全量输入之后：**合并出来的行就是权威行** ⇒ 下游（`--next`/`--json`/§0.2/`check_plan`/
    `source_dispositions()`）读的是同一份，`effective_state` 不再"喂两种行得两个状态" ✓；
    终态行**留在队列里**（= docstring「处置」一节的"已关账，不许从队列里删除 ✗"），
    但**不进 open 集合**（`open_set()`/`summary()` 只看 `OPEN_DISPOSITIONS` ✓）。
    """
    merged: dict[str, dict] = {}
    # 处置的**出处**（id → 谁写的那一条），只为冲突信息**点名**用：两个来源各说各话时，
    # "谁 vs 谁"必须能从判红信息里读出来（只报两个处置名的话，人还得自己回台账里翻 ✗）。
    prov: dict[str, str] = {}
    for row in source_rows():
        ident = row["id"]
        if ident not in merged:
            merged[ident] = row
            prov[ident] = _src_label(row)
            continue
        old = merged[ident]
        old["sources"] = sorted(set(old["sources"] + row["sources"]))
        if row["disposition"] is None:
            continue                                  # 索引只是 reference，处置以 extras 为准 ✓
        if old["disposition"] is None:
            old["disposition"] = row["disposition"]
            old["expect"] = row.get("expect")
            prov[ident] = _src_label(row)
        elif old["disposition"] != row["disposition"]:
            raise UsageError(
                f"id `{ident}` 在两处来源里处置不一致：`{old['disposition']}`"
                f"（{prov[ident]}）vs `{row['disposition']}`（{_src_label(row)}）"
                f"—— **一个 id 只能有一个处置** ✗；这是**跨来源处置冲突**，不是漂移："
                f"两边说的不是同一件事，脚本无从判断谁对 ⇒ 先把两处改成同一个处置"
                f"（终态的缺口行也**照样**过这道闸门，不因为它 `status=fixed` 就看不见 ✗）")
        for key in ("owner", "note", "where"):
            if not old.get(key) and row.get(key):
                old[key] = row[key]
    undecided = sorted(k for k, v in merged.items() if v["disposition"] is None)
    rows = [v for v in merged.values() if v["disposition"] is not None]
    rows.sort(key=lambda r: (SEVERITY_RANK[r["severity"]], r["id"]))
    return rows, undecided


def _src_label(row: dict) -> str:
    """来源行的**人话出处**（冲突信息里点名"谁 vs 谁"用；不参与任何判定 ✓）。"""
    srcs = row.get("sources") or []
    if "gap-ledger" in srcs:
        return "缺口台账 `docs/gaps/ledger.jsonl`"
    if "extras" in srcs:
        extra = [s for s in srcs if s != "extras"]
        return "`docs/ux-queue-extras.jsonl`" + (f"（{extra[0]}）" if extra else "")
    return f"`{INDEX.name}` 索引（无处置）"


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
        # ⚠ **本文件唯一允许 `errors="replace"` 的地方**（`#1` 的横向排查结论，2026-10-14）：
        # 这条 `subprocess` 读的是**探针的输出**，而它在这里**只用于展示** ——
        # 判定完全由 `rc` 决定（`judge_rc(kind, rc)`，`rc` 是整数、不经文本层 ✓），
        # 文本只进 `tail`（下面那一行，截断成一行给人看）。
        # ⇒ 它不是"内容比对"⇒ `replace` 不会让任何判定**悄悄错** ✓（`_guard_encoding` 禁的是
        # 判定类读取，见那里的口径）；反过来，若在这里用 `surrogateescape`，孤立代理会一路
        # 流进报告文本（`print` 只能靠 `backslashreplace` 兜住），**可读性更差** ⇒ 展示用 `replace` ✓。
        # ⚠ **判定类** git 读取不许照抄这里 ⇒ 一律走 `_git_text()`（逐字节保真 ✓）。
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
    # ⭐ **双向的另一半**（`#3`，2026-10-14 · 第七次独立复核实测 ✗）：docstring 的「处置」一节
    # 与 `--check-uncovered` 的 help 都承诺「**任何 `scheduled` 条目没进 §0.2 表 ⇒ 判红**」，
    # 而这里**只有 §0.2 → 队列那一向**（幽灵行 / 陈旧行）⇒ 实测：一条 `scheduled`、§0.2 无该行
    # ⇒ 合并的 `--check` **rc=0 · 从不点名** ✗，而 `--next` 推得出它、`--json.open` 含它
    # ⇒ 三个视图打架（「唯一入口」看不见这条排期）。`scheduled` 的语义就是「§0.2 里必须有它
    # 的一行」⇒ 缺行即判红 ✓（`fix` 是细粒度队列，**不要求**进 §0.2 —— 那是 `load_onboarding_queue`
    # 说的"40+ 条明细塞不进有行数上限的必读文档" ✓，所以只对 `scheduled` 判）。
    for _r in rows:
        if _r["disposition"] == "scheduled" and _r["id"] not in doc_ids:
            print(f"✗ §0.2 缺行：`{_r['id']}` 是 `scheduled`（= **排期**），但 "
                  f"{_label(ONBOARDING)} §0.2 表里没有它的一行 ⇒ 「唯一入口」看不见这条排期 ✗ "
                  f"（补一行；或把处置改成 `fix`（细粒度队列）/ `wo-filed`（定案不做）—— "
                  f"**别让处置与入口两头漂移** ✗）", file=sys.stderr)
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
# ── **"已关账"的单一出处**（2026-10-14 · 第五轮独立复核：`--check` 的判据域与
#    `effective_state` 的域不一致 ⇒ 三族静默格）
#
# 本文件 docstring 的「处置」一节**就是规格**：`fixed` / `wo-filed` / `boundary` / `control`
# 四个处置**并称"已关账"**、**都不进 open 集合** ✓。而判据原先只认 `{fixed, wo-filed}` ✗
# ⇒ 两处不一致（复核者实测：`boundary`/`control` × 6 个调度器状态 = **12 格静默且被
# `--next` 推出来** ✗）。⇒ 谓词改成与 docstring **同源**，两侧各写成**一处**定义：
#   · `CLOSED_DISPOSITIONS` —— **来源侧**"已关账"（= `DISPOSITIONS - OPEN_DISPOSITIONS`；
#     自检里有一条**结构性**守卫钉死这条恒等式 ⇒ 以后加处置忘了归类会先红 ✗）；
#   · `DERIVED_CLOSED_STATES` / `CLOSED_STATES` —— **调度器侧**"不在做"。
#     `boundary`/`control` **只可能由处置派生**（`TRANSITIONS` 里没有它们的入口 ⇒
#     只有"没有事件"时才出现）⇒ 它们与来源的 `boundary`/`control` 正是**一致**的形状 ✓
#     （真台账唯一一行 `r02` = `control` + 无事件 ⇒ 不红 · 不推 · 不进 open ✓；
#      反过来说：这一支若不认它们，`r02` 会被误判成"来源关账而调度器没关账" ✗）。
#   · `WORK_DONE_DISPOSITIONS` —— 来源里记的**工作结论**（`done`/`wo_filed` 的对应物）。
#     ⚠ 它**故意**比 `CLOSED_DISPOSITIONS` 小：`boundary`/`control` 的语义是"**从来不算
#     缺口**"，而 `done`/`wo_filed` 是调度器记下的"**我做完了 / 定案不做**"这个结论 ⇒
#     调度器记了结论而来源写着"从来不算缺口"**仍然是漂移**（来源必须记同一个结论）。
#     两个集合回答的是两个不同的问题（"关没关账" vs "结论记的是哪一个"）⇒ 这一处是
#     **更严**（没有任何一格从红变绿），不是"两处不一致" ✓。
CLOSED_DISPOSITIONS = {"fixed", "wo-filed", "boundary", "control"}
WORK_DONE_DISPOSITIONS = {"fixed", "wo-filed"}
DERIVED_CLOSED_STATES = {"boundary", "control"}
CLOSED_STATES = TERMINAL_STATES | DERIVED_CLOSED_STATES
# `--next` 的口径 = 已关账 **再加** `umbrella`/`blocked` —— 这两个是"**开着但不可动**"，
# 不是已关账 ⇒ 与 `CLOSED_STATES` 分开（否则 `--check` 会把"拆解中 / 被阻塞"误判成关账 ✗）。
NOT_PUSHABLE_STATES = CLOSED_STATES | {"umbrella", "blocked"}
# 处置 → 无事件时的**派生状态**（唯一映射表：`effective_state` 与自检的网格都读它 ✓）
DISPOSITION_STATES = {"fix": "todo", "scheduled": "todo", "fixed": "done",
                      "wo-filed": "wo_filed", "boundary": "boundary", "control": "control"}
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


def closure_target(disp: str | None) -> str | None:
    """来源处置对应的**平账目标态**（`fixed`→`done` · `wo-filed`→`wo_filed`）。

    `boundary`/`control` ⇒ `None`（**如实**）：它们在 `TRANSITIONS` 里**没有入口**
    （只由处置派生 ⇒ 无事件时才出现）⇒ 平账**不是**"再做一步迁移"，而是
    **删掉事件表里那几条**（回到来源台账的处置）**或**改来源处置 ✓ ——
    调用方据此打印"表里没有能平账的迁移"，而不是印一条**死路指示** ✗
    （旧代码对所有处置都印 `--to done`/`--to wo_filed`，而 `boundary`/`control` 走到那些
    终态会**再**撞上 `WORK_DONE_DISPOSITIONS` 那一条 ⇒ 照指示敲只会换个红 ✗）。
    """
    return {"fixed": "done", "wo-filed": "wo_filed"}.get(disp or "")


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
        elif kind == "repoint":
            # **回填收口指针**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）：`--repoint` 只改**指针**。
            # ⚠ 三条不许动：① **不许动 `state`**（修的是"指针在 main 上核不到"，不是"这条没做完" ✗）；
            #   ② **不许动 `reading`/`reason`**（证据正文一个字不换，换的只是它指向哪个 commit）；
            #   ③ **不许改旧事件**（只追加账 ⇒ 原来那条 `advance` 原样留着，旧 sha 记进
            #      `repointed_from` ⇒ 改账本身可审计、可逐条回滚 ✓）。
            if ev.get("commit"):
                _from = list(cur.get("repointed_from") or [])
                if cur.get("commit") and cur["commit"] not in _from:
                    _from.append(cur["commit"])
                cur["repointed_from"] = _from
                cur["commit"] = ev["commit"]
                cur["repointed_ts"] = ev.get("ts")
                # ⭐ **审计字段折进状态**（`#1`）：`check_plan` 的读侧复查要拿它比对
                # "现在这个指针还是不是当时那个内容"（手改事件表换 commit ⇒ 判红 ✓）。
                for _k in ("subject", "patch_id"):
                    if ev.get(_k):
                        cur[f"repoint_{_k}"] = ev[_k]
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
                # ⚠ **受理也是收口 ⇒ 必须记 `advanced_ts`**（2026-10-14 · 第六次独立复核实测 ✗）：
                # 旧代码只有 `advance` 分支写它 ⇒ triage-applied 的指针**永远没有时间戳** ⇒
                # `check_plan` 的祖先性复查把 `_ts = advanced_ts or ""` 当成"存量"⇒
                # **记于口径起点之后的假指针被豁免** ✗（见 `ANCESTRY_GATE_SINCE` 的说明）。
                if ev.get("ts"):
                    cur["advanced_ts"] = ev["ts"]
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
#   · **存量**（`ANCESTRY_GATE_SINCE` **之前**记的 done 指针）原先**不判红**（带日期豁免）。
#
# ⭐ **`ANCESTRY_GATE_SINCE` 现在只是历史标记，不再是豁免线**（2026-10-14 · 同日晚 ·
#   `RELEASE-EVIDENCE-GATE` · 编排者拍定的口径 ✓）：豁免**已删**，读侧口径改成
#   **「非祖先必须 0 条」的断言** —— 理由是长期豁免会让那 60 条证据在 main 上**永远核不了**，
#   且 main 再 rebase 一次会把口径起点之后的整批一起判红（豁免救不了）✗。
#   两步走、**顺序不许反** ✗：① `--repoint-all` 把存量**回填到 0**（真台账读数
#   60 条 → 0 条，38 个 sha **38/38** 找到孪生：同 subject + `git patch-id --stable` 一致 +
#   `is-ancestor` ✓）⇒ ② 才把豁免线改成断言。
#   ⚠ **若将来 main 再被整体 rebase** ⇒ 回填过的指针会再次解析不到 main（它们**不是**
#   "内容没落 main"，是"历史被重写"）⇒ 那时**照同一条命令再回填一次**（`--repoint-all`
#   幂等：已经是的不会再出现 ✓），**不要**去恢复日期豁免 ✗。
#   ⚠ **"存量"这个词只给"带时间戳且早于起点"的那一桶**（2026-10-14 · 第六次独立复核实测 ✗）：
#     `ts` 是**判"新增/存量"的唯一依据**，而 `fold` 原先**只在 `advance` 分支写
#     `advanced_ts`** ⇒ `--triage … --to applied` 记的指针**永远没有时间戳** ⇒
#     读侧 `_ts = advanced_ts or ""` ⇒ `"" < 起点` ⇒ 一条**记于起点之后**的假指针被当成
#     "存量 rebase 孪生"**豁免**、描述与事实相反 ✗（复核者实测：`--triage --to applied
#     --commit deadbeef…` ⇒ `--check` rc=0，stderr 只有"⚠ 存量收口指针 1 条 … EXT-1@deadbeef"）。
#     ⇒ 两处一起修：① `fold` 的 triage-applied 分支**也写 `advanced_ts`**（与 advance 同形）；
#     ② 读侧**按 ts 分两桶报** —— 有 ts 且 < 起点 = **存量**（豁免）；**没有 ts** = **判不出**
#     （旧格式/手写事件）⇒ 也从宽，但**不许叫"存量"** ✗（那是在断言一件没核过的事）。
#     真台账实测：最后一条 `advance`/`triage-applied` **全部**带 ts（0 条无 ts）⇒ 分桶只影响
#     措辞，不动任何一条的判绿/判红 ✓。
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


def closure_pointer_problem(commit: str) -> str | None:
    """**收口指针的两条硬拒**（`cmd_advance` 与 `cmd_triage` **共用这一处** ⇒ 同口径 ✓）。

    返回 `None` = 过；否则返回判红正文（调用方 `print(..., file=sys.stderr)` 后 `return 1`）。
      · **存在性**：`git cat-file -e <sha>^{commit}`（2026-10-12 `SUB-advance-commit-unchecked`）；
      · **祖先性**：`git merge-base --is-ancestor <sha> HEAD`（2026-10-14 · `queueW` 缺陷 ⑤）。

    ⚠ **两条都必须有，而且两个写入口都必须走它**（2026-10-14 · 第六次独立复核实测 ✗）：
    `cmd_triage` 原先**一条都没有** ⇒ 复核者实测 `--triage … --to applied --commit
    deadbeef…deadbeef`（对象**不存在**，`git cat-file -e` rc=128）**rc=0 通过**，账上留下一个
    "有 commit"的**假终态**；而 `--release add/check` 只看 `done` + 有 `commit` ⇒ **假证据能入版** ✗。
    ⇒ 抽成**一个**函数（不是把 `cmd_advance` 那两段抄一遍 ✗ —— 抄一遍就是"同一规则两处写"，
    下次只改一处又会分叉，正是 `AGENTS.md` 硬规则 0(b) 说的病）。
    """
    import subprocess as _sp
    probe = _sp.run(["git", "cat-file", "-e", f"{commit}^{{commit}}"],
                    cwd=ROOT, capture_output=True, text=True)
    if probe.returncode != 0:
        return (f"✗ `--commit {commit}` 在仓库里**不存在**（`git cat-file -e` 非零）"
                f" ⇒ 收口证据是假的，不许记 ✗（先把这个环节真的提交掉）")
    if not commit_is_ancestor(commit):
        return (f"✗ `--commit {commit}` **不是 `HEAD` 的祖先**"
                f"（`git merge-base --is-ancestor` 非零）⇒ 指针解析不到 main 的历史上"
                f"（多半是 rebase 前的孪生，或别的分支）⇒ 收口证据在 main 上**找不到**，"
                f"不许记 ✗（用**真落进 main 的那个 commit**；口径起点 "
                f"`ANCESTRY_GATE_SINCE={ANCESTRY_GATE_SINCE}`）")
    return None


# ── 收口指针**回填**（`--repoint`，2026-10-14 · `RELEASE-EVIDENCE-GATE`） ─────────
# 缺口（第六次独立复核实测 ✓）：`--release add/check` 原先**只要求 `done` + 有 `commit`** ⇒
# **手写的假指针能进发布节点** ✗（实测 `release_add_rc=0`，即使那个 commit 对象**不存在**）。
# 而存量里 60 条 `done` 指针是 **rebase 孪生**：对象在 ✓、内容也确实落了 main ✓（抽样 3/3
# `git patch-id --stable` 逐字节一致 ✓），只是指针**解析不到 main 的历史**。
# 口径（编排者已拍定）：**先把存量回填到 0，再把豁免线改成「必须为空」的断言** ✓。
#   ⚠ **顺序不许反** ✗：先收紧会把那 60 条一起拒掉。
#   ⚠ **不选长期豁免** ✗：长期豁免 ⇒ 那 60 条证据在 main 上**永远核不了**；且 main 再 rebase
#     一次会把口径起点之后的整批一起判红。
# `--repoint` = **改账**（只追加事件），**不动 git 历史、不改旧事件** ✗：
#   · 孪生**三关**：**同 subject → `git patch-id --stable` 一致 → `is-ancestor`** ✓
#     （⚠ 少一关都不行：只比 subject 会撞重名 —— 实测 main 3632 个 commit 里只有 3342 个
#     唯一 subject ⇒ **有重名**；只比 patch-id 会把"内容相同、语义不同"的两次提交当成一次）；
#   · 找不到孪生 / 有歧义 ⇒ **不许硬填** ✗（如实列出 —— 那是真问题）；
#   · 新指针必须过 `closure_pointer_problem()`（与 `--advance`/`--triage`/`--release`
#     **同一个谓词** ✓ —— 别自己再写一遍 ✗，那是 `AGENTS.md` 硬规则 0(b) 说的病）。
_TWIN_CACHE: dict[str, tuple[str | None, str]] = {}
_ANCESTORS_BY_SUBJECT: dict[str, list[str]] | None = None

# ── **判定类 git 读取的唯一出处**（`#1`，2026-10-14 · 第八次独立复核实测 ✗） ────────
# 缺口：`5c4e1779` 给 `_guard_encoding` 的 docstring 白纸黑字写了「**不许**用
# `errors="replace"` 掩盖 —— 那会让 subject 比对**悄悄错**（判定类代码的红线 ✗）」，
# 而**同一个 commit 自己**的 `_commit_subject`/`_patch_id`/`_ancestors_by_subject`
# 三处**全都**传了 `errors="replace"` ⇒ **代码与自己的注释互相打脸** ✗。
# 实测后果（本文件 `.cache/ux/fix-utf8E/s1_utf8.py` 的夹具，两个自建仓 commit）：
#   两个 commit 的 diff 只差**一个非法字节**（`0x80` vs `0x81`）⇒ 都折成 U+FFFD ⇒
#   我们的 `_patch_id` **两者相同**、而 **git 自己的** `patch-id --stable` **两者不同**
#   ⇒ 孪生三关的第 2 关**假绿** ⇒ `--repoint <id> --commit <内容真不同的那个>` **rc=0** ✗
#   （指针被指向内容真不同的 commit，且下一条 `--check` 也绿 —— 证据链被掏空）。
# 口径（**二选一，选前者**）：
#   · ✅ `errors="surrogateescape"` —— **逐字节保真**：`git show` 的字节原样回灌给
#     `git patch-id`（`subprocess` 的 stdin/stdout 都按同一对 `encoding/errors` 包成
#     `TextIOWrapper`）⇒ 哈希的就是**真字节** ⇒ 与 git 自己的算法**逐位一致** ✓；
#   · ✗ 「遇到非法字节就拒（rc=2）」—— 会把**真的**含非法字节的孪生**误拒** ✗
#     （比原缺口更糟：把合法证据判成环境错）；而非法字节在这里**本来就该被如实比对**。
# ⚠ **一处定义**：本文件里所有"读了要拿来做判定"的 git 调用**都走 `_git_text()`** ⇒
#   以后不会再有"只改一处、另一处又漂"（`AGENTS.md` 硬规则 0(b)）✗。
# ⚠ 唯一允许 `errors="replace"` 的地方是**展示**：`run_probe()` 的 tail（判定只看 rc）。
_GIT_TEXT: dict = {"text": True, "encoding": "utf-8", "errors": "surrogateescape"}


def _git_text(argv: list[str], stdin: str | None = None) -> subprocess.CompletedProcess:
    """**判定类** git 读取（唯一出处）：UTF-8 + `surrogateescape` ⇒ **逐字节保真** ✓。

    ⚠ 别退回 `text=True`（= locale 编码 ⇒ `LC_ALL=C` 下抛 `UnicodeDecodeError` 栈 ✗，
    见 `_guard_encoding`），也别退回 `errors="replace"`（⇒ 判定**悄悄错** ✗，见上面）。
    """
    return subprocess.run(["git", *argv], cwd=ROOT, capture_output=True,
                          input=stdin, **_GIT_TEXT)


def _subject_text(subject: str) -> str:
    """subject 折成**能进 UTF-8 事件表**的文本（写入与读侧复查**共用本函数** ✓）。

    ⚠ 为什么需要（`#1` 的配套）：`surrogateescape` 解出的**孤立代理**（= 原字节非 UTF-8）
    **不能直接进 UTF-8 文件** —— `json.dumps(ensure_ascii=False)` + `write_text(utf-8)`
    会抛 `UnicodeEncodeError`（`main()` 会把它翻成 rc=2 ✓ 不丢栈，但那本来是判定场景 ✗）。
    ⇒ 折成**可见转义**（`\\x80` 之类）：对**合法 UTF-8 是恒等**（既有 subject 一个字节不变 ✓）。
    **判定**不靠它（靠 `patch_id`，十六进制 ⇒ 永远 ASCII ✓），它只是"事后能核"的旁证。

    ⚠ **如实说明**：这是**防御性**守卫，不是"本平台已复现"的路径 —— 实测本机
    （macOS + 系统 iconv）`git log --format=%s` / `git cat-file commit` 对**非法字节的
    commit message** 会**归一化成合法 UTF-8**（`0x80` → `c2 80`，即 U+0080）⇒ subject 这一路
    拿不到孤立代理。但（a）**别的平台/带 `encoding` 头的 commit** 未必如此，而代价是一条
    `UnicodeEncodeError` 栈路；（b）本函数是**写入与复查的唯一折算处** ⇒ 两侧同源 ✓。
    真正**天天走** `surrogateescape` 的是 `_patch_id()` 的 `git show` **diff 字节**
    （非法字节就在那里，且它逐字节回灌给 `git patch-id` ⇒ 与 git 自己逐位一致，见那里的读数 ✓）。
    """
    return subject.encode("utf-8", "surrogateescape").decode("utf-8", "backslashreplace")


def _ancestors_by_subject() -> dict[str, list[str]]:
    """`HEAD` 全历史按 subject 分组（**一次** `git log`；`--repoint-all` 要问几十次）。"""
    global _ANCESTORS_BY_SUBJECT
    if _ANCESTORS_BY_SUBJECT is None:
        out: dict[str, list[str]] = {}
        raw = _git_text(["log", "--format=%H%x01%s", "HEAD"]).stdout
        for line in raw.splitlines():
            if "\x01" in line:
                sha, subj = line.split("\x01", 1)
                out.setdefault(subj, []).append(sha)
        _ANCESTORS_BY_SUBJECT = out
    return _ANCESTORS_BY_SUBJECT


_PATCH_ID_CACHE: dict[str, str | None] = {}
_SUBJECT_CACHE: dict[str, str] = {}


def _commit_subject(commit: str) -> str:
    """`git log -1 --format=%s`（**UTF-8 + 逐字节保真**；对象不在 ⇒ `""`）。"""
    if commit not in _SUBJECT_CACHE:
        _SUBJECT_CACHE[commit] = _git_text(["log", "-1", "--format=%s", commit]).stdout.strip()
    return _SUBJECT_CACHE[commit]


def _patch_id(commit: str) -> str | None:
    """`git patch-id --stable`（对 `git show <commit>` 的 diff 算；算不出返回 `None`）。

    ⚠ **显式 UTF-8**（`#5a`，2026-10-14 · 第七次独立复核）：旧写法 `text=True` 用
    `locale.getpreferredencoding()` 解码 ⇒ 非 UTF-8 环境（`LC_ALL=C`）下**在孪生搜索里
    抛 `UnicodeDecodeError` 栈**（rc=1 ✗，不是契约的 rc=2）；仓库 subject 大量中文 ⇒ 必踩。
    这里把它钉成 UTF-8（git 的默认输出编码），环境不参与内容判定 ✓。

    ⚠ **`surrogateescape` 而不是 `replace`**（`#1`，2026-10-14 · 第八次独立复核实测 ✗）：
    见 `_GIT_TEXT` 的说明 —— `replace` 会把 diff 里的非法字节折成 U+FFFD ⇒ 两个**内容不同**
    的 commit 得到**同一个** patch-id ⇒ 第 2 关假绿 ✗。逐字节保真后与 git 自己的
    `patch-id --stable` **逐位一致** ✓（两个读数见 `.cache/ux/fix-utf8E/VERDICT.md`）。
    """
    if commit not in _PATCH_ID_CACHE:
        _PATCH_ID_CACHE[commit] = None
        show = _git_text(["show", commit])
        if show.returncode == 0:
            probe = _git_text(["patch-id", "--stable"], stdin=show.stdout)
            if probe.returncode == 0 and probe.stdout.strip():
                _PATCH_ID_CACHE[commit] = probe.stdout.split()[0]
    return _PATCH_ID_CACHE[commit]


def twin_gate_problem(old: str, cand: str) -> str | None:
    """**孪生三关**（同 subject → `patch-id --stable` 一致 → 是 `HEAD` 的祖先）的**唯一出处**。

    返回 `None` = `cand` 是 `old` 的合法孪生；否则返回判红正文。
    ⚠ **自动路与显式路共用本函数**（`find_twin()` 的搜索、`cmd_repoint --commit` 的显式指针）
    —— 别写第二遍 ✗（`AGENTS.md` 硬规则 0(b)：同一规则两处写 ⇒ 下次只改一处又分叉）。

    ⚠ **为什么必须有**（`#1`，2026-10-14 · 第七次独立复核实测 ✗）：`--repoint <id> --commit <sha>`
    原先**只查存在性 + 祖先性**（`closure_pointer_problem`），**不比 subject / patch-id** ⇒
    复核者实测 `--repoint G-01 --commit 92ef75e1`（main 的 **M0 根提交**，内容完全无关）
    **rc=0**，且下一条 `--check` 打出「非祖先 0 条 ✓ 没有豁免」⇒ **证据链被"指向任意祖先"掏空** ✗
    （而省略 `--commit` 的自动路**本来就过三关** ⇒ 同一规则两条口径）。

    ⚠ **判定顺序**（只影响"先报哪一条"，三关是**合取**）：先**存在性 + 祖先性**
    （"这个指针在不在 main 的历史上"）⇒ 再 subject ⇒ 再 patch-id。理由：非祖先的指针
    该拿到**最可操作**的那条诊断（"不是 `HEAD` 的祖先"），别被内容差异掩盖 ✓
    （也让 `--advance`/`--triage` 那条共用谓词的报错逐字不变 ✓）。
    """
    old_subj = _commit_subject(old)
    if not old_subj:
        return (f"✗ 旧指针 `{old[:12]}` 读不出 subject（`git log -1` 空 —— 那个对象真的在吗？）"
                f"⇒ **内容闸门无法核对**（没有可比对的旧内容）⇒ 不许用显式指针硬填 ✗："
                f"先 `--advance <id> --to todo --reason '<为什么>'` 重开，再按**真落进 main** 的"
                f"那个 commit 收口 ✓")
    cand_subj = _commit_subject(cand)
    if not cand_subj:
        return (f"✗ 新指针 `{cand[:12]}` 读不出 subject（对象不存在 / 不是 commit）⇒ 内容闸门"
                f"过不了 ✗")
    if not commit_is_ancestor(cand):
        return (f"✗ `{cand[:12]}` **不是 `HEAD` 的祖先**（`git merge-base --is-ancestor` 非零）"
                f"⇒ 指针解析不到 main 的历史上（孪生三关的第 3 关）✗")
    if cand_subj != old_subj:
        return (f"✗ **subject 不一致**：旧 `{old_subj[:60]}` vs 新 `{cand_subj[:60]}`"
                f"（孪生三关的第 1 关）⇒ 这不是同一个环节的 rebase 孪生 ✗")
    mine, theirs = _patch_id(old), _patch_id(cand)
    if mine is None or theirs is None:
        return (f"✗ `git patch-id --stable` 算不出（旧 {mine or 'N/A'} / 新 {theirs or 'N/A'}）"
                f"⇒ 内容一致性核不了（孪生三关的第 2 关）✗")
    if mine != theirs:
        return (f"✗ **patch-id 不一致**：旧 `{mine[:12]}` vs 新 `{theirs[:12]}`（孪生三关的第 2 关）"
                f"⇒ 内容**没落** main，只是 subject 撞了 ✗")
    return None


def find_twin(commit: str) -> tuple[str | None, str]:
    """**三关找 rebase 孪生**：同 subject → `patch-id --stable` 一致 → 是 `HEAD` 的祖先。

    返回 `(孪生 sha | None, 人话说明)`。**歧义也返回 `None`**（不硬填 ✓）。
    ⚠ 三关的**规则本体**在 `twin_gate_problem()`（**一处定义**，显式指针那条路也调它 ✓）；
    本函数只负责"在同 subject 的祖先里**搜索**"这件搜索活。
    """
    if commit in _TWIN_CACHE:
        return _TWIN_CACHE[commit]
    subj = _commit_subject(commit)
    if not subj:
        _TWIN_CACHE[commit] = (None, f"`{commit[:8]}` 读不出 subject（对象真的在吗？）")
        return _TWIN_CACHE[commit]
    cands = _ancestors_by_subject().get(subj, [])
    if not cands:
        _TWIN_CACHE[commit] = (None, f"同 subject 的**祖先** commit **0 个**"
                                     f"（subject：{subj[:70]}）⇒ 没有孪生")
        return _TWIN_CACHE[commit]
    same = [c for c in cands if twin_gate_problem(commit, c) is None]
    if not same:
        _TWIN_CACHE[commit] = (None, f"同 subject 的祖先有 {len(cands)} 个，但**三关**"
                                     f"（subject / `patch-id --stable` 一致 / `is-ancestor`）"
                                     f"一个都不成立 ⇒ 内容没落 main")
        return _TWIN_CACHE[commit]
    if len(same) > 1:
        _TWIN_CACHE[commit] = (None, f"**歧义**：{len(same)} 个祖先的 subject 与 patch-id 都一致"
                                     f"（{'、'.join(c[:8] for c in same)}）⇒ 不硬填")
        return _TWIN_CACHE[commit]
    _TWIN_CACHE[commit] = (same[0], f"✓ 同 subject + `patch-id --stable` 一致"
                                    f"（同 subject 的祖先 {len(cands)} 个里唯一匹配）")
    return _TWIN_CACHE[commit]


def repoint_targets() -> list[tuple[str, str, str]]:
    """存量里**指针核不到**的 `done` 条目：`[(id, 旧 sha, 记录时刻)]`（按 id 排序）。

    ⚠ 口径与 `check_plan` 的祖先性复查**同一处**：`fold()` 的**事件状态** `state == "done"`
    （不是 `effective_state` —— 由来源处置推出来的 `done` 没有调度器指针 ✓）。
    """
    out: list[tuple[str, str, str]] = []
    for ident, cur in sorted(fold().items()):
        if cur.get("state") != "done":
            continue
        _c = cur.get("commit")
        if _c and not commit_is_ancestor(_c):
            out.append((ident, _c, cur.get("advanced_ts") or ""))
    return out


def _repoint_event(ident: str, new: str, old: str, reason: str, owner: str) -> dict:
    """`repoint` 事件的**唯一定义**（`--repoint` 与 `--repoint-all` 共用 ⇒ 别写两遍 ✗）。

    ⚠ **必须带 `subject` + `patch_id`**（`#1` 审计字段，2026-10-14 · 第七次独立复核实测 ✗）：
    旧事件只有 `{commit, old, reason, owner, ts}` ⇒ **连"换过去的内容是什么"都没留** ⇒
    事后**无从审计**（复核者实测：`--repoint … --commit <无关祖先>` 之后，账上读不出任何
    内容痕迹）。现在记**新旧两侧**的 subject + patch-id ⇒ ① 事后可复查"当时三关确实一致"；
    ② `check_plan` 可复查"现在这个指针还是不是当时那个内容"（手改事件表换 commit ⇒ 判红 ✓）。
    """
    return {"event": "repoint", "id": ident, "commit": new, "old": old,
            "subject": _subject_text(_commit_subject(new)), "patch_id": _patch_id(new),
            "old_subject": _subject_text(_commit_subject(old)), "old_patch_id": _patch_id(old),
            "reason": reason, "owner": owner}


def cmd_repoint(ident: str, commit: str, reason: str, owner: str, dry_run: bool) -> int:
    """**回填一条收口指针**（只追加事件）。

    `--commit <sha>` 显式给新指针；**省略**（或 `auto`）⇒ 按三关**自动找孪生** ✓。
    ⚠ **显式指针也过同一条三关**（`twin_gate_problem()`，与自动路**共用** ✓）——
    见 `twin_gate_problem` 的说明（`#1`：只查祖先性 ⇒ 证据链可被"指向任意祖先"掏空 ✗）。
    """
    state = fold()
    cur = state.get(ident)
    if cur is None:
        raise UsageError(f"事件表里没有 `{ident}` ⇒ `--repoint` 只修**已有**的收口指针"
                         f"（它是「改账」，不是收口入口 ✗）")
    if cur.get("state") != "done" or not cur.get("commit"):
        raise UsageError(f"`{ident}` 不是 done（state={cur.get('state')!r}）或没有 `commit` "
                         f"⇒ 没有可回填的指针")
    if not reason:
        print("✗ `--repoint` 是**改账** ⇒ 必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
        return 1
    old = cur["commit"]
    if commit_is_ancestor(old):
        print(f"✗ `{ident}` 的指针 `{old[:8]}` **已经是 `HEAD` 的祖先** ⇒ 没有可回填的"
              f"（`--repoint` 修的是「在 main 上核不到」的指针，**不是**改历史 ✗）",
              file=sys.stderr)
        return 1
    explicit = bool(commit.strip()) and commit.strip() != "auto"
    if explicit:
        # ⭐ **显式指针也要过内容闸门**（`#1`，2026-10-14 · 第七次独立复核实测 ✗）：
        # 旧写法只走 `closure_pointer_problem()`（存在性 + 祖先性）⇒ 实测
        # `--repoint Q-1 --commit 92ef75e1`（main 的 M0 根提交、内容完全无关）**rc=0**，
        # 下一条 `--check` 还打出「非祖先 0 条 ✓」⇒ 证据链被"指向任意祖先"掏空 ✗。
        # ⇒ 与自动路**共用** `twin_gate_problem()`（三关：subject / patch-id / is-ancestor）。
        new, note = commit.strip(), "（显式给的指针）"
        why = twin_gate_problem(old, new)
        if why is not None:
            print(f"✗ `{ident}`：显式指针 `{new[:12]}` **过不了孪生三关** ⇒ 拒绝 ✗"
                  f"（`--repoint` 修的是「**同一个环节**在 main 上的孪生」，"
                  f"不是「随便指一个祖先」✗）", file=sys.stderr)
            print(f"  {why}", file=sys.stderr)
            print(f"  ⇒ 正解：① 省略 `--commit` 让三关**自动找孪生**（推荐 ✓）；"
                  f"或 ② 若那个对象真的不在，先 `--advance {ident} --to todo "
                  f"--reason '<为什么>'` 重开，再按**真落进 main** 的 commit 重新收口 ✓",
                  file=sys.stderr)
            return 1
    else:
        new, note = find_twin(old)
        if new is None:
            print(f"✗ `{ident}`@`{old[:8]}` **找不到孪生** ⇒ **不硬填** ✗：{note}",
                  file=sys.stderr)
            return 1
    if new == old:
        print(f"✗ `{ident}` 的新指针与旧指针相同（`{old[:8]}`）⇒ 这不是回填 ✗", file=sys.stderr)
        return 1
    why = closure_pointer_problem(new)
    if why is not None:
        print(why, file=sys.stderr)
        return 1
    if dry_run:
        print(f"[dry-run] `{ident}`：{old[:8]} → {new[:8]} {note}（**没写事件** ✓）")
        return 0
    append_event(_repoint_event(ident, new, old, reason, owner))
    print(f"✓ 回填 `{ident}`：`{old[:8]}` → `{new[:8]}` {note}"
          f"（只追加事件：旧指针留在账上 + `repointed_from` ✓；"
          f"审计字段 subject/patch-id 已记 ✓）")
    return 0


def cmd_repoint_all(reason: str, owner: str, dry_run: bool, limit: int) -> int:
    """**一条命令把存量非祖先指针推到 0**（按 sha 分组 ⇒ 同一 sha 的所有 id **同批**写）。

    ⚠ `--limit N` = 每批最多处理 **N 个 sha**（整 sha 一起写 ⇒ 账本没有"半截"状态）；
    下一批重跑同一条命令即可（已回填的不再是非祖先 ⇒ 自动接着往下做 ✓）。
    ⚠ 找不到孪生的 sha：**如实列出、一条都不写** ✗（那是真问题，报告里点名 ✓）。
    """
    if not reason:
        print("✗ `--repoint-all` 是**改账** ⇒ 必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
        return 1
    targets = repoint_targets()
    if not targets:
        print("✓ 存量收口指针里**没有非祖先的**（0 条）—— 没什么要回填的 ✓")
        return 0
    groups: dict[str, list[str]] = {}
    for ident, old, _ts in targets:
        groups.setdefault(old, []).append(ident)
    shas = sorted(groups)
    if limit and limit > 0:
        shas = shas[:limit]
    plan, no_twin = [], []
    for old in shas:
        twin, note = find_twin(old)
        (plan if twin else no_twin).append((old, sorted(groups[old]), twin, note))
    print(f"存量非祖先指针：**{len(targets)} 条** / **{len(groups)} 个 sha**"
          f"（本批 {len(shas)} 个 sha）")
    for old, ids, _twin, note in no_twin:
        print(f"  ✗ `{old[:8]}`（{len(ids)} 条：{'、'.join(ids)}）**找不到孪生** ⇒ 不硬填 —— {note}",
              file=sys.stderr)
    n_ids = sum(len(ids) for _o, ids, _t, _n in plan)
    for old, ids, twin, note in plan:
        print(f"  {'[dry-run] ' if dry_run else ''}{old[:8]} → {twin[:8]}"
              f"（{len(ids)} 条：{'、'.join(ids)}）{note}")
    if dry_run:
        print(f"[dry-run] 计划回填 {n_ids} 条 / {len(plan)} 个 sha"
              f"{f'；{len(no_twin)} 个 sha 找不到孪生' if no_twin else ''}（**没写任何事件** ✓）")
        return 1 if no_twin else 0
    for old, ids, twin, _note in plan:
        for ident in ids:
            append_event(_repoint_event(ident, twin, old, reason, owner))
    print(f"✓ 回填 **{n_ids} 条** / {len(plan)} 个 sha（只追加事件；旧指针留在账上 ✓）")
    # **别信自己的账**：回填完**再问一次 fold**（重新读事件表 ⇒ 真复查 ✓）。
    # ⚠ 复查**只对本批点过名的 id 判红** ✗：`--limit` 是**分批**用的，剩下的非祖先是
    # **下一批的活**、不是这一批的失败 —— 否则"成功的一批"会打 rc=1 ⇒ **读数与事实相反** ✗。
    done_ids = {i for _o, ids, _t, _n in plan for i in ids}
    left = repoint_targets()
    stuck = [(i, c) for i, c, _t in left if i in done_ids]
    if stuck:
        print(f"✗ 本批点过名的 **{len(stuck)} 条没写进去**（重新 fold 后仍是非祖先）⇒ 判红 ✗：" +
              " · ".join(f"{i}@{c[:8]}" for i, c in stuck), file=sys.stderr)
        return 1
    if no_twin:
        print(f"✗ 还有 **{len(no_twin)} 个 sha** 找不到孪生 ⇒ **没回填到 0** ✗（见上面的清单）",
              file=sys.stderr)
        return 1
    if left:
        print(f"⏳ 还剩 **{len(left)} 条** / **{len({c for _i, c, _t in left})} 个 sha** 非祖先"
              f" ⇒ 下一批（重跑同一条命令即可，已回填的不会再出现）✓")
        return 0
    print("✓ 复查：存量非祖先收口指针 **0 条** ✓")
    return 0


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

    ⚠ **合成行不该有"出处"**（2026-10-14 · 第六次独立复核实测 ✗）：它是**视图的投影**，
    来源台账里**没有**这一行 ⇒ 它的 `sources` 只能是**诚实的空表** `[]`，**不许**拿这一行的
    存在去给"这个 id 有没有出处"作证。旧代码连 `sources` 键都没有 ⇒ 两处坏在同一处：
      · `--json.view` 里合成行与源行**异形 schema**（少 `sources` ⇒ 按源行形状读的消费者
        `KeyError` ✗ —— 补成 `[]` = "**没有来源**"，不是"忘了写" ✓）；
      · `check_plan` 的幽灵判据原先读 `ids(rows=视图)` ⇒ **合成行给自己作了证** ⇒
        「无源子任务」对幽灵判据**不可见** ✗（复核者纯 CLI 复现：`--schedule ADOPT-1` +
        `--add-child ADOPT-1 --child GHOST-KID-1` ⇒ `--check` rc=0 · stderr 0 行 · 从不提它，
        而 `--next` 推得出它、`--json.open` 含它 ⇒ 三视图不一致 ✗）。
    ⇒ 「有没有出处」**只由 `check_plan` 问来源台账**（三处）定，视图行不能作证 ✓。
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
            "owner": cur.get("owner") or "", "where": _label(EVENTS),
            # ⚠ `sources: []` = **诚实的"没有来源"**（不是"忘了写"）：合成行是视图的投影，
            # 台账里没有它那一行 ⇒ 不许拿它给自己作出处（见本函数 docstring 的说明）。
            # 与源行**同形 schema** ⇒ `--json` 的消费者不必分两种形状读 ✓。
            "sources": []}
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
    # ⚠ **已关账（权威状态）就不许认领**（2026-10-14 · 第五次独立复核的同族）：旧写法只看
    # `cur["state"]`（调度器**写过**的状态）⇒ 「**来源已关账 + 没有事件**」的 id 在
    # `state` 里根本没有键 ⇒ 闸门静默放过：`--claim` 返回 0，而**下一条 `--check` 立刻判红**
    # （来源 `fixed` + 调度器 `in_progress` ⇒ 反向两层漂移）。判据不是"没静默"就够了 ——
    # 写侧也不许**主动制造**一个已知必红的形状 ✗。
    # ⚠ 这条闸门在**全量输入**（`source_rows()`）之后尤其必要：终态缺口行现在**在视图里**
    # （`build_queue()` 不再把它丢掉）⇒ 以前那种 id 走 `row is None` 那一支被**顺带**拦住，
    # 现在会走到这里 ⇒ 不补这条就是**本轮新开的洞** ✗。
    # 口径 = **与 `check_plan` 的反向漂移判据同一个谓词**（`effective_state` + `CLOSED_STATES`
    # ⇒ 一处定义）：`fixed`/`wo-filed`/`boundary`/`control` 与派生的 `done`/`wo_filed`/
    # `boundary`/`control` 一律拒；`submitted`/`triage_rejected`/`umbrella`/`blocked`
    # **不在这里拦**（它们的"能不能动"是另一条规则的事，别顺手放宽/收紧 ✗）。
    st = effective_state(row or {"id": ident}, cur)
    if cur.get("state") in TERMINAL_STATES or st in CLOSED_STATES:
        got = cur.get("state") or f"由来源处置 `{row.get('disposition') if row else '?'}` 派生"
        print(f"✗ `{ident}` 已是终态 `{got}`（权威状态 `{st}`）⇒ 要重开得先 "
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
    # ⚠ **"已关账"用权威状态判**（`effective_state` + `CLOSED_STATES`，与 `cmd_claim` /
    # `check_plan` 同一个谓词 —— 2026-10-14 · 第六次独立复核实测 ✗）：旧写法只看**事件状态**
    # `frm in TERMINAL_STATES` ⇒ 两种坏形状：
    #   ① 「来源已关账 + **无事件**」的 id 连这个分支都进不来 ⇒ 掉进"非法迁移 `todo → todo`"
    #      （信息错、也没有出路指示 ✗）；
    #   ② 「来源已关账 + 事件终态」进来之后**照写 `reopen` 事件** ⇒ 造出
    #      「来源 `fixed` + 事件 `todo`」的**必红**状态（下一条 `--check` = 反向两层漂移 ✗、
    #      `--next` 又把它推出来 ✗），而它打印的却是「✓ 状态回到来源处置」—— **与事实相反**
    #      （来源根本没回到，是**事件覆盖**了来源）✗。对照：来源 `fix` + 事件 `done` 时
    #      reopen rc=0 ✓（那个 `done` 正是要重开的东西）⇒ **不许误拒** ✗。
    row = next((r for r in rows if r["id"] == ident), None) or {"id": ident}
    if to == "todo" and effective_state(row, cur) in CLOSED_STATES:
        if not reason:
            print("✗ 重开终态条目必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
            return 1
        # **来源侧**还关着账 ⇒ 光写事件改不动它（来源是权威 ⇒ `check_plan` 立刻判红）✗
        if source_closed(row):
            _d = row.get("disposition")
            print(f"✗ `{ident}` 的**来源处置**是 `{_d}`（**已关账**）⇒ 光写 `reopen` 事件"
                  f"**回不到来源处置** ✗：来源才是权威（`--check` 会判「来源关账 + 调度器 "
                  f"todo」= **反向两层漂移**，`--next` 又会把它推出来）—— 要重开就**先把来源"
                  f"处置改回 `fix`**：\n"
                  f"    · extras 里的条目：`--triage {ident} --to deferred --reason '<为什么>'`"
                  f"（**同轮**把来源翻成 `fix` + 事件回 `todo` ✓）\n"
                  f"    · 缺口台账里的条目：直接改 {_label(GAPS_LEDGER)} 那一行的 `status`"
                  f"（脚本没有 CLI 入口）\n"
                  f"  改完再 `--advance {ident} --to todo --reason '<为什么>'` ✓", file=sys.stderr)
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
    # ⚠ **收口指针要过两条硬拒**（**存在性 + 祖先性**，两处写入口共用
    # `closure_pointer_problem` ⇒ 同口径 ✓）：旧闸门只查"非空字符串" ⇒ 实测
    # `--commit deadbeefdeadbeef…`（`git cat-file -e` 非零）**rc 0** 通过，账上留下一个
    # "有 commit"的**假终态**；而 `--release check`（切版收拢）也只看 done + 有证据
    # ⇒ **假证据能把不该入版的条目放进版本** ✗（2026-10-12 `SUB-advance-commit-unchecked`）。
    # 2026-10-14（缺陷 ⑤）再加**祖先性**：存在性查不出「指针不是 main 的祖先」—— 实测 32 条
    # 历史 done 指针是 **rebase 前的孪生**（对象在、内容也落了 main，但
    # `git merge-base --is-ancestor <sha> HEAD` 非零）⇒ 证据**没法在 main 上核** ✗。
    # 口径：**新增收口**必须过闸门（存量走 `check_plan` 的带日期豁免，见常量块说明）。
    if to == "done":
        _why = closure_pointer_problem(commit)
        if _why is not None:
            print(_why, file=sys.stderr)
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
        # ⚠ "子任务关了没有"只有**一处**口径（`open_children` ⇒ `effective_state` +
        # `CLOSED_STATES`）：否则 `--check` 的"子任务全关了"提示与这里的闸门会各说各话 ✗。
        open_kids = open_children(kids, rows, state)
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
    # ⚠ **子任务 id 也是"生 id 的入口"** ⇒ 与 `--schedule`/`--submit` **共用同一个谓词**
    # （`_id_length_problem`：长度上限；横向排查，别只修一处 ✗）。
    _shape = _id_length_problem(child)
    if _shape is not None:
        print(_shape, file=sys.stderr)
        return 1
    # ⚠ **父闸门也走权威状态**（`effective_state` + `CLOSED_STATES`，**照 `cmd_claim` 的形状** ——
    # 2026-10-14 · 第六次独立复核实测 ✗）：旧写法只看 `state[父]["state"]`（**调度器写过的**
    # 状态）⇒ 「**来源已关账 + 没有事件**」的父条目在 `state` 里根本没有键 ⇒ 闸门**静默放过**：
    # `--add-child` rc=0，而 `split` 事件把父折叠成 `umbrella` ⇒ 下一条 `--check` 立刻判红
    # （来源 `fixed` + 调度器 `umbrella` = 反向两层漂移）✗ —— 写侧不许**主动制造**一个已知必红的形状。
    # 口径与 `cmd_claim` / `check_plan` 的漂移判据**同一个谓词** ⇒ 三处不可能各说各话 ✓；
    # 这一改**只严不松**（`TERMINAL_STATES ⊆ CLOSED_STATES`，另加 `boundary`/`control` 两个派生态）。
    _prow = next((r for r in rows if r["id"] == parent), None) or {"id": parent}
    _pst = effective_state(_prow, state.get(parent, {}))
    if _pst in CLOSED_STATES:
        _pd = _prow.get("disposition")
        _how = (f"**先把来源处置改回 `fix`**（`--triage {parent} --to deferred --reason '<为什么>'`"
                f" 同轮翻来源 ✓），再 `--advance {parent} --to todo --reason …` ✓"
                if source_closed(_prow) else
                f"先 `--advance {parent} --to todo --reason '<为什么>'` ✓")
        print(f"✗ 父条目 `{parent}` 已是**已关账**状态 `{_pst}`"
              f"（来源处置 `{_pd}`）⇒ 不许再拆解：`split` 事件会把父折成 `umbrella`，"
              f"而来源仍是关账的 ⇒ 下一条 `--check` **必红**（反向两层漂移）✗ —— {_how}",
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
    """**权威状态**：`--next` / `--json` / §0.2 / `check_plan` 的漂移判据**共用这一份**。

    事件状态优先；没有事件 ⇒ 用来源的处置翻译（`fix`/`scheduled` 都是**可动**的 todo）✓。

    ⚠ **已关账的处置不许被 `external-submission` 分支短路**（2026-10-14 · 第五轮独立复核）：
    旧代码把 `external-submission` 判断放在处置表**之前** ⇒ 一条 `disposition=fixed` 的
    外部提交（它的 `triage` 早已 applied）在**没有事件**（= docstring 明说的合法操作
    "删掉事件文件就回到来源台账的处置"）时被翻译成 `submitted`（**非终态**）⇒ 真台账上
    **42 条**这样的条目被 `--next` 推出来，而 `--json.open=[]`、§0.2 `total=0`、
    `--check` 也看不见（判据域只含有事件的 id ✗）⇒ **三个视图三个答案** ✗。
    ⇒ 口径：**关账的处置先答**（`fixed`→done · `wo-filed`→wo_filed · `boundary`→boundary ·
    `control`→control，见 `DISPOSITION_STATES`/`CLOSED_STATES`）；只有**开着**的处置
    （`fix`/`scheduled`）才轮到"外部提交在受理前不是可动的 todo" ⇒ `submitted` ✓。
    这与 `cmd_submit` 的既有口径自洽：**正常提交路径写的处置是 `fix`**（非终态）
    ⇒ 这一支不进短路；`--triage` 受理/拒收把处置翻成 `fixed`/`wo-filed` 时，
    该条目本来就该是 `done`/`wo_filed` ✓。
    """
    if cur.get("state"):
        return cur["state"]
    mapped = DISPOSITION_STATES.get(row.get("disposition"))
    if mapped in CLOSED_STATES:
        return mapped
    if "external-submission" in (row.get("sources") or []):
        return "submitted"          # 外部提交在受理前不是"可动的 todo"（没人认领它）
    return mapped or "todo"


def source_closed(row: dict) -> bool:
    """**来源侧「已关账」**（唯一谓词）：处置 ∈ `CLOSED_DISPOSITIONS`。

    等价于 `effective_state(row, {}) in CLOSED_STATES`（同一份 `DISPOSITION_STATES` 口径；
    自检有一条**结构性**守卫逐处置断言这个等价 —— 加处置忘了归类会先红 ✗）。

    ⚠ 为什么写侧闸门**必须**用它，而不是 `effective_state(row, cur)`（2026-10-14 · 第六次
    独立复核实测 ✗）：`effective_state` 在"**事件**已终态"时也答"已关账" ⇒ 拿它当
    "来源能不能动"的判据会把**合法**的 reopen 一并拒掉 ✗（来源 `fix` + 事件 `done` 是
    **正常**形状：那个 `done` 正是要重开的东西）。写侧问的是"**来源**放不放行" ⇒ 只能看处置。

    用途（**一处定义、三处消费**）：
      · `cmd_advance --to todo`：来源关着账 ⇒ 光写 `reopen` 事件**回不到来源处置** ✗；
      · `cmd_add_child` 的父闸门：来源关着账的父条目**不许**再拆解（拆了必红）✗；
      · 自检的结构性守卫（`source_closed` ≡ `effective_state(row, {}) in CLOSED_STATES`）。
    """
    return row.get("disposition") in CLOSED_DISPOSITIONS


def is_pushable(row: dict, cur: dict, owner: str | None = None) -> bool:
    """`--next` 的可动判据（**唯一出处**：`cmd_next` / 自检的三视图断言 / 网格都调它 ✓）。

    ⚠ 为什么抽出来（2026-10-14 · 第五轮复核的**同族第五处**）：这条口径原先在
    `cmd_next` 里写死一遍、又在 `--selftest` 的"三视图同数"断言里**手抄一遍** ⇒
    "哪些条目可动"有两个真相（改一处不改另一处 ⇒ 断言自己就漂移了 ✗）。
    """
    st = effective_state(row, cur)
    if st in NOT_PUSHABLE_STATES:
        return False
    return not (st == "in_progress" and cur.get("owner") not in (None, owner))


def pushable_ids(view: list[dict], state: dict[str, dict],
                  owner: str | None = None) -> list[str]:
    """`--next` **推得出来的** id 集合（同一个 `is_pushable` ⇒ 读数与真命令同源 ✓）。"""
    return sorted(r["id"] for r in view if is_pushable(r, state.get(r["id"], {}), owner))


def authoritative_row(by_id: dict[str, dict], ident: str,
                      disp: dict[str, str | None]) -> dict:
    """某个 id 的**权威行** = 视图那一行（可能没有）+ **来源的处置**（来源是权威 ⇒ 覆盖视图）。

    ⚠ 视图里可能**没有**这一行（`status=="fixed"` 的缺口按队列视图口径**不进视图**），
    而处置仍要从**全量来源**取（`source_dispositions()`）⇒ 别把"视图里没有"当成"没有处置"
    （那会把已关账的 id 判成开着的 `todo` ✗）。**一处定义**：`check_plan` 与 `open_children`
    都走它 ✓。
    """
    row = by_id.get(ident) or {"id": ident}
    if disp.get(ident) is not None:
        row = {**row, "disposition": disp[ident]}
    return row


def open_children(children: list[str], rows: list[dict], state: dict[str, dict],
                  disp: dict[str, str | None] | None = None) -> list[str]:
    """父条目仍**没关**的子任务（**唯一出处**：`check_plan` 的提示 与 `cmd_advance` 的闸门共用）。

    ⚠ 同一条"关了没有"的口径不许有两份（否则 `--check` 说"子任务全关了"而 `--advance`
    照样拒收口 ✗）：这里也走 `effective_state` + `CLOSED_STATES` —— 子任务**没有事件**时
    由它自己的处置定（`fixed`/`wo-filed`/`boundary`/`control` = 已关账 ✓），有事件时由事件定。
    `disp` = 权威处置表（`check_plan` 已经算好 ⇒ 传进来，别重算 ✗）；不传就现取
    （`source_dispositions()`；只 `cmd_advance` 这一条路，不在热路径上 ✓）。
    """
    by_id = {r["id"]: r for r in rows}
    if disp is None:
        try:
            disp = source_dispositions()
        except (UsageError, OSError, KeyError, ValueError):
            disp = {}
    return [k for k in children
            if effective_state(authoritative_row(by_id, k, disp), state.get(k, {}))
            not in CLOSED_STATES]


def plan_domain(rows: list[dict], state: dict[str, dict]) -> list[str]:
    """`check_plan` 的**判定域**（**一处定义**）：视图里的每个 id ∪ 事件表里的每个 id。

    ⚠ 为什么要有这个名字（2026-10-14 · 第五轮独立复核的**最重一族**）：漂移判据原先的域是
    `for ident, cur in sorted(state.items())` —— **只遍历"有事件"的 id** ✗，而
    `effective_state`（`--next`/`--json` 用的那份）**能对"无事件"的 id 推出** `todo`/`submitted`
    ⇒ **判据域 ⊊ 生效域** ⇒ 复核者实测：删掉事件文件（docstring 明说的合法操作）后
    `--check` rc=0 · stderr 0 行，而 `--next` 照样推 42 条 ⇒ 三视图不一致 ✗。
    ⇒ 域 = **视图 ∪ 事件表**（与 `--next` 迭代的 `sched_rows(rows, state)` 同一份）✓；
    两边的**状态**都由 `effective_state` 算（不自己再算一遍 ✗ —— 那会造出第三个真相）。
    """
    return sorted({r["id"] for r in rows} | set(state))


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
        # ⚠ 可动判据只有**一处**（`is_pushable` ⇒ `effective_state` + `NOT_PUSHABLE_STATES`）：
        # `--check` 的漂移判据、`--json.open` 的对账、自检的三视图断言都读同一份 ✓
        if not is_pushable(row, cur, owner):
            continue
        st = effective_state(row, cur)
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

    ⚠ **现在它是 `build_queue()` 的派生视图，不是第二次读来源**（2026-10-14 · 第五次独立
    复核的「跨来源处置冲突」）：上一轮（`queueX` 缺陷 ⑥）为了让漂移规则看得见终态缺口行，
    单开了**第二条读来源的路**（`load_gaps(include_terminal=True)`）并让 `check_plan` 用它
    **覆盖**视图行 —— 那修好了"终态看得见"，却造出了**两份输入** ✗：
      * `build_queue()` 的合并仍用 `load_gaps()` 默认口径 ⇒ **冲突闸门对终态缺口行瞎**，
        而且 `--next`/`--json` 读的是视图行（extras `fix`）；
      * 这份表读全量 ⇒ `check_plan` 读的是权威行（gap `fixed`）⇒ 同一个 `effective_state`
        **喂两种行得两个状态**（`--next` 说 `todo`、`--check` 说 `done`）✗。
    ⇒ 根本修法在**输入**那一层：`source_rows()` 全量 + `build_queue()` 合并（含冲突闸门）
    ⇒ 合并出来的行**就是权威行**；本函数退化成它的投影（`None` 那一支 = 索引里登记过、
    还没人写处置的 id ⇒ 由 `--check-uncovered` 的 ①/①b 判红，不是漂移、也不是幽灵 ✓）。

    **三个来源各自怎么"看得见"**（编号与本文件的来源分节一致；`selftest_source_visibility`
    逐条断言；加来源就照这份清单补一行，别只补一处 ✗）：
      * 来源 ①（`docs/gaps/ledger.jsonl`）：**必须显式要全量** —— `load_gaps()` 默认把
        `status=="fixed"` 丢掉（那只留给"只要 open 的只读消费方"），而终态行既要在
        **冲突闸门**里、又要在漂移规则里看得见 ✗ ⇒ 走 `source_rows()`；
      * 来源 ②（索引）：**只给身份**（`disposition=None`）⇒ 它的处置由 extras 那一行提供；
        没有 extras 行的索引 id 属于「**登记了但没人写处置**」⇒ 本表里是 `None` ✓；
      * 来源 ③（extras）：**不过滤** ⇒ 全量（处置的唯一出处 ✓）。
    """
    rows, undecided = build_queue()
    disp: dict[str, str | None] = {r["id"]: r["disposition"] for r in rows}
    for ident in undecided:
        disp.setdefault(ident, None)
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

    ⚠ **来源可见性**（2026-10-14 · 缺陷 ⑥；第五次复核把口径**统一成一份输入**）：
    这里原先有两份输入 —— `rows`/`source_rows` 是**视图**口径（`load_gaps()` 默认把
    `status=="fixed"` 丢掉 ✗），于是「来源终态 + 调度器非终态」对缺口来源**永远咬不住**
    （`G-110`/`G-112` 实测假绿），补法是**再开一条全量读法**（`source_dispositions()`）
    并在这里覆盖一遍 ⇒ 修好了那一条，却让**同一个 id 有两个出处**：视图行（extras `fix`）
    与权威行（gap `fixed`）⇒ 同一个 `effective_state` **喂两种行得两个状态**
    （`--next` 推 `todo`、这里判 `done`）✗。
    ⇒ 现在**只有一份输入**：`build_queue()` 用 `source_rows()`（全量）合并 ⇒ **视图行就是
    权威行**；本函数只认 `rows`（+ `source_rows` 对"视图被遮蔽"的防御，`selftest_child_drift`
    ⑤ 钉着它）—— 没有第三遍 ✗。

    ⚠ **判定域 = 视图 ∪ 事件表**（2026-10-14 · 第五轮独立复核 ①）：原先的域是
    `sorted(state.items())` —— **只遍历"有事件"的 id** ✗，而 `--next`/`--json` 用的
    `effective_state` **能对"无事件"的 id 推出状态**（处置翻译）⇒ **判据域 ⊊ 生效域**。
    实测（真台账 + 按 docstring 的合法操作删掉事件文件）：改前 `--check` **rc=0 · stderr 0 行**，
    而 `--next` 照样推 **42 条**（42/42 的权威处置都是 `fixed` = 已关账）⇒ 三个视图三个答案 ✗；
    **只把域对齐**（不动权威口径）⇒ 同一条命令 **rc=1 · 42 个 id 被点名** ✓（域就是那个洞）；
    再把共享权威（`effective_state` 的短路顺序）修对 ⇒ 两边**一致地**判"已关账"（0 红 ✓）。
    ⇒ 域由 `plan_domain()` 一处定义；**状态**一律由 `effective_state()` 算（不自己再算一遍 ✗）。

    ⚠ **域 ⊇ 出处**（2026-10-14 · 第六次独立复核实测 ✗）：**判定的域**（上面那条 = 视图 ∪
    事件表）与**"这个 id 有没有出处"**是**两个**问题，别混用同一份输入 ✗ ——
    `sched_rows` 会给**没有源行的子任务**合成一行（`sources=[]`），而旧代码拿**视图**当
    `known` ⇒ **合成行给自己作了证** ⇒「无源子任务」静默 ✗（`--add-child` 只写 `split`
    事件、三个来源里都没有那一行）。⇒ `known` **只从来源台账算**（`source_rows` 或单输入
    夹具的 `rows`）+ `source_history_ids()`；**视图行不能作证** ✓（`selftest_ghost_child`
    逐条钉着它，改前那一臂必红 ✗）。
    ⚠ **两个状态名不要混用**：证据类判据（`done` 的 commit+reading / `wo_filed` 与
    `triage_rejected` 的理由 / `umbrella` 的子任务）只认 `cur["state"]`（调度器**写过**的状态）
    —— 由来源处置推出来的状态**不需要**调度器证据（证据在来源台账里）⇒ 否则删掉事件文件后
    每条已关账的条目都会被误判"没证据的收口" ✗；漂移类判据认**权威状态**。
    """
    state = fold()
    bad = 0
    # ⚠ **幽灵判据只认来源台账，不认视图行**（2026-10-14 · 第六次独立复核实测 ✗）：
    # 这里原先写 `{r["id"] for r in rows}`，而 `rows` 是**视图**（`main()` 传的是
    # `sched_rows(rows, state)`）⇒ `sched_rows` 给"没有源行的子任务"合成的那一行
    # **给自己作了证** ⇒ 幽灵判据被它自己的合成行绕过 ✗（复核者纯 CLI 复现：
    # `--schedule ADOPT-1` + `--add-child ADOPT-1 --child GHOST-KID-1` ⇒ `--check` rc=0 ·
    # stderr 0 行 · 从不提那个 id，而 `--next` 推得出它、`--json.open` 含它 ⇒ 三视图不一致 ✗）。
    # ⇒ 域仍然 = **视图 ∪ 事件表**（`plan_domain`，第五轮那个洞的修法不变 ✓），
    # 但「**有没有出处**」只问**来源台账**：有 `source_rows`（真 CLI 路径 `check_plan(view, rows)`）
    # 就用它；单输入夹具（`check_plan(rows)`）退回 `rows`。
    # ⚠ 这一改**只严不松**：`{ids(source_rows)} ⊆ {ids(视图)}`（视图 = 源行 + 合成行）
    # ⇒ `known` 只会变小（少认了合成行），没有任何一格从红变绿 ✓。
    # ⚠ **发版节点也要算出处**（`#2`，2026-10-14 · 第八次独立复核实测 ✗）：`--release open`
    # 只写 `release-open` 事件（**没有来源行**，那是它正确的形状 —— 节点不是工作条目）
    # ⇒ 不认它就会在**开节点之后的下一条 `--check`** 判「幽灵条目」✗（gate/CI 第一步就撞红）。
    # ⇒ `known` 加上 `release_node_ids()`（**只认 `release-open`**；没有它的 id 照样判红 ✓，
    # 见该函数的说明与 `selftest_release_open_seam` 的「不许放宽」臂）。
    src = rows if source_rows is None else source_rows
    known = {r["id"] for r in src} | source_history_ids() | release_node_ids()
    disp = {r["id"]: r.get("disposition") for r in rows}
    for _r in (source_rows or []):     # **来源的处置是权威**（覆盖视图里可能被遮蔽的值）
        if _r.get("disposition") is not None:
            disp[_r["id"]] = _r["disposition"]
    # ⚠ **不再有三遍**（2026-10-14 · 第五次独立复核）：这里原先还有一遍
    # `for _id, _d in source_dispositions().items(): disp[_id] = _d` —— 那是"两份输入"的
    # 第二次读来源（`source_rows`/`rows` 是**视图**口径、`source_dispositions()` 是**全量**
    # 口径）⇒ 同一个 id 的处置有两个出处。现在**输入只有一份**（`source_rows()` 全量 +
    # `build_queue()` 的合并 ⇒ 视图行**就是**权威行），所以那一遍是纯冗余：它只会把
    # 合并已经定好的值再抄一遍。删掉它 = 判据与显示读同一份 ✓（`source_dispositions()`
    # 本身也改成由 `build_queue()` 派生 ⇒ 留着的调用点（`cmd_submit`/`cmd_schedule` 的
    # "id 是键"闸门、`open_children` 的兜底）读的仍是同一份 ✓）。
    bad_pointers: list[tuple[str, str, str]] = []    # **非祖先收口指针** ⇒ 一律判红（没有豁免 ✓）
    rows_by_id = {r["id"]: r for r in rows}
    # ⚠ **判定域 = 视图 ∪ 事件表**（#1 的根本修法，见 `plan_domain`）：`--next`/`--json` 对
    # **无事件**的 id 也能推出状态（`effective_state` 的处置翻译）⇒ 判据必须看见**同一批 id**，
    # 否则"删掉事件文件"（docstring 明说的合法操作）会让判据**静默**而 `--next` 照样推 ✗。
    for ident in plan_domain(rows, state):
        cur = state.get(ident, {})
        # 处置的权威是**来源** ⇒ 权威行也按来源算（视图那一行可能不存在、或被合成行遮蔽 ✓）
        row = authoritative_row(rows_by_id, ident, disp)
        # 两个状态名各司其职，**不混用**：
        #   · `st_ev` = 调度器**写过**的状态 ⇒ 证据类判据（done 的证据 / wo_filed 的理由 /
        #     triage_rejected 的理由 / umbrella 的子任务）只认它 —— 由**来源处置推出来**的
        #     状态**不需要**调度器证据（它的证据在来源台账里：`fixed`/`wo-filed` 本身就是
        #     处置结论）⇒ 否则删掉事件文件后每条已关账的条目都会被判"没证据的收口" ✗✗；
        #   · `st` = **权威状态**（`effective_state`，与 `--next` 同一个函数）⇒ 漂移类判据认它。
        st_ev = cur.get("state")
        st = effective_state(row, cur)
        # ⚠ 这里**不划测试命名空间**（2026-10-12 接口 agent 提交 `SUB-ghost-guard-diluted`）：
        # 曾经为了让验证脚本不弄红而跳过 `R-*`/`P-*` —— 那是**放宽守卫** ✗。正解是让那些脚本
        # **别往真台账写**（用 `SOKO_QUEUE_EVENTS=<临时文件>`，见 `EVENTS` 的说明）⇒
        # 幽灵判据保持满强度：任何不在来源里的 id 一律判红 ✓。
        if ident not in known:
            # ⚠ **无源子任务**（2026-10-14 · 第六次独立复核实测 ✗）：`--add-child` 只写
            # `split` 事件 ⇒ 子任务在**三个来源里都没有行**。旧代码被 `sched_rows` 的
            # 合成行绕开 ⇒ 静默；现在合成行的 `sources` 是空的、判据只认台账 ⇒ 判红并
            # **点名怎么补**（写进 extras 那一行就是它的出处 ✓）。
            if cur.get("parent"):
                print(f"✗ 子任务 `{ident}` 在**三个来源**（{_label(EXTRAS)} / "
                      f"{_label(GAPS_LEDGER)} / {_label(INDEX)}）里都查不到 ⇒ **无源子任务** ✗ "
                      f"（拆解自 `{cur.get('parent')}`）：`--add-child` **只写 `split` 事件**，"
                      f"视图里那一行是**合成的**（`sources=[]` ⇒ 不能给自己作出处）⇒ "
                      f"把它登记进 {_label(EXTRAS)}（`disposition=fix` + `why`）再 `--check` ✓",
                      file=sys.stderr)
            else:
                print(f"✗ 事件里的 `{ident}` 不在任何来源里 ⇒ 幽灵条目（删来源了？）✗",
                      file=sys.stderr)
            bad += 1
            continue
        if st_ev == "triage_rejected" and not cur.get("reason"):
            print(f"✗ `{ident}` 是 `triage_rejected` 却没写理由 ⇒ 拒收也要讲清为什么 ✗",
                  file=sys.stderr)
            bad += 1
        if st_ev == "done":
            if not (cur.get("commit") and (cur.get("reading") or cur.get("triage") == "applied")):
                print(f"✗ `{ident}` 标了 done 却没有 `commit`+`reading` ⇒ **没证据的收口** ✗",
                      file=sys.stderr)
                bad += 1
            # **收口指针必须在 main 的历史上**（2026-10-14 · 缺陷 ⑤）：存在性查不出祖先性。
            # ⭐ **口径（2026-10-14 · `RELEASE-EVIDENCE-GATE`）：非祖先必须 0 条，没有豁免** ✓
            # 这条线原先按 `advanced_ts` 分三桶、给"存量（早于口径起点）"一条**带日期的豁免**。
            # 存量 60 条（38 个 sha）已用 `--repoint` **全量回填到 0**（38/38 找到孪生：
            # 同 subject + `git patch-id --stable` 一致 + `is-ancestor`）⇒ 豁免线**删掉**，
            # 改成下面那条**断言**（每次 `--check` 都打出来 ⇒ 不许静默 ✓）。
            # ⚠ 为什么不留长期豁免 ✗：长期豁免会让那 60 条证据在 main 上**永远核不了**；
            # 且 main 再 rebase 一次会把口径起点之后的整批一起判红（豁免根本救不了）。
            # ⚠ **顺序不许反** ✗：先收紧 = 把还没回填的那 60 条一起拒掉（所以先回填、后收紧）。
            _c = cur.get("commit")
            if _c and not commit_is_ancestor(_c):
                _ts = cur.get("advanced_ts") or ""
                bad_pointers.append((ident, _c, _ts))
                print(f"✗ `{ident}` 的收口 commit `{_c}` **不是 `HEAD` 的祖先**"
                      f"（记录于 {_ts or '无时间戳'}）⇒ 证据在 main 上**找不到**"
                      f"（被 rebase 甩掉了？）✗ —— 存量孪生指针用 "
                      f"`--repoint {ident} --reason '<why>'` 三关自动回填；"
                      f"或按新历史重开再收口 / 改记**真落进 main** 的那个 commit", file=sys.stderr)
                bad += 1
            # ⭐ **审计复查**（`#1` 的后半，2026-10-14 · 第七次独立复核）：`repoint` 事件现在
            # 记了 `subject` + `patch_id` ⇒ 读侧复查"**现在这个指针还是不是当时那个内容**"。
            # 靶子 = 手改事件表把 `repoint` 的 `commit` 换成别的（审计字段不会跟着变）⇒ 当场判红 ✗。
            # ⚠ 只对**带审计字段的** repoint 事件生效（旧账 / 手写事件没有该字段 ⇒ 不误判 ✓）。
            # ⚠ subject 两侧都过 `_subject_text()`（**同一处**折算 ⇒ 非法字节不会两边口径不同 ✗）。
            _rec_pid, _rec_subj = cur.get("repoint_patch_id"), cur.get("repoint_subject")
            if _c and _rec_pid:
                _now_pid, _now_subj = _patch_id(_c), _subject_text(_commit_subject(_c))
                if _now_pid != _rec_pid or (_rec_subj and _now_subj != _rec_subj):
                    print(f"✗ `{ident}` 的收口指针与它的 `repoint` 事件**对不上**："
                          f"账上记的 subject/patch-id = `{(_rec_subj or '')[:40]}` / "
                          f"`{_rec_pid[:12]}`，而 `{_c[:12]}` 实际是 "
                          f"`{_now_subj[:40]}` / `{(_now_pid or 'N/A')[:12]}`"
                          f" ⇒ 指针被**事后换过**（手改事件表？）✗ —— `--repoint` 的审计字段"
                          f"就是为这条复查留的 ✓", file=sys.stderr)
                    bad += 1
        # ── **两层漂移 = 两个方向的同一条不变量**：来源"已关账" ⟺ 调度器"不在做"。
        # 两个方向合起来才是"三视图一致"：open（来源）· `--next`（调度器）· `--check`（对账）。
        _d = disp.get(ident)
        # **正向**：调度器记了一个**工作结论**（`done`/`wo_filed`，都带证据：commit+reading /
        # reason）⇒ 来源必须记**同一个结论**（`fixed`/`wo-filed` ✗ `boundary`/`control` 不算 ——
        # 它们的语义是"从来不算缺口"，见 `WORK_DONE_DISPOSITIONS`）。
        # ⚠ **镜像方向原先一条牙都没有**（第五轮复核 ④）：`--advance X --to wo_filed --reason …`
        # （来源仍 `fix`）⇒ `--check` 静默、`--next` 不动它（终态），而 `--json.open`/§0.2
        # **永远报"1 条未完成"** ✗ ⇒ 三视图不一致。这里把 `done` 那一向的牙**对称**补上 ✓。
        if st_ev == "done" and _d not in WORK_DONE_DISPOSITIONS:
            print(f"✗ `{ident}` 调度器里 done，来源处置却是 `{_d}` ⇒ "
                  f"**两层漂移**（把来源台账改成 fixed/wo-filed）✗", file=sys.stderr)
            bad += 1
        if st_ev == "wo_filed" and _d not in WORK_DONE_DISPOSITIONS:
            print(f"✗ `{ident}` 调度器里 wo_filed（**定案不做**，带理由），来源处置却是 `{_d}` ⇒ "
                  f"**两层漂移**：来源没记这个结论 ⇒ `--json.open`/§0.2 会**永远**把它算成"
                  f"未完成 ✗（收口：把来源台账改成 `wo-filed`，或 "
                  f"`--advance {ident} --to todo --reason '<为什么>'` 退回重开）", file=sys.stderr)
            bad += 1
        # **反向两层漂移**（`SUB-inflight-claims-never-closed`，2026-10-12）：
        # 上面只查了 `done` + 来源不是终态这一个方向；**反方向一直没人查** ✗ ——
        # 实测事故：`r07`/`G-110` 的修复早已落地、来源处置也已是 `fixed`，但事件表里
        # **只有 claim、没有 advance** ⇒ 调度器停在 `in_progress` ⇒ 仍算 **open**：
        # 队列对"还剩什么"**多报**，`--next` 会把**已经做完**的条目推荐出来 ✗。
        # ⇒ 补上这个方向：来源已关账、调度器却没到"不在做" ⇒ 判红（并指名怎么收口 ✓）。
        # ⚠ **`submitted` 不豁免**（2026-10-14 · 第三轮独立复核实测）：`37139ca7` 曾在这里
        # 加过 `and st != "submitted"`（**没写理由** ✗）。实测它的**唯一**观察效果就是让
        # 「来源已是终态 × 调度器 `submitted`」这一格**静默**（3 来源 × {fixed,wo-filed}
        # = **6/6 格静默**，而同一时刻 `--next` 照样把那条推出来 —— `submitted` 既不在
        # `TERMINAL_STATES`、也不在 `NOT_PUSHABLE_STATES` 里 ⇒ 正是本规则要咬的形状）。
        # 正常提交路径**不受影响**：`cmd_submit` 写的处置是 `fix`（非终态）⇒ 这一支根本
        # 进不来（`cmd_triage` 受理/拒收才把处置翻成 `fixed`/`wo-filed`，那时也应该收口 ✓）。
        # ⇒ 网格逐格钉住（`selftest_source_state_grid`）：删掉这个豁免前 6 格静默、删掉后 36 格全红 ✓。
        # ⚠ **口径与 docstring 同源**（第五轮复核 ②）：docstring 把**四个**处置并称"已关账"、
        # 都不进 open 集合 ⇒ 谓词认 `CLOSED_DISPOSITIONS`（四个），**不是**只认两个 ✗ ——
        # 否则 `boundary`/`control` × 6 个调度器状态 = **12 格静默且被 `--next` 推** ✗
        # （真台账唯一一行 `r02` 就是 `control` ⇒ 这条只差一条命令之遥）。
        # ⚠ **`st` 用权威状态而不是事件状态**：无事件的 id 由处置派生（`fixed`→done ⇒ 一致、
        # 不红 ✓），有事件的 id 由事件定（`in_progress` ⇒ 红 ✓）⇒ 判据域与 `--next` **同一份**。
        if _d in CLOSED_DISPOSITIONS and st not in CLOSED_STATES:
            # **收口指示由表算出来**（同族第三处，见 `closure_steps`）：手写 `--to done`
            # 在 `todo`/`blocked`/`submitted`/`triage_rejected` 上都是**非法迁移**（rc=1）✗；
            # 而 `boundary`/`control` 在 `TRANSITIONS` 里**没有入口** ⇒ 正如实说"没有能平账的
            # 迁移"（`closure_target` 返回 `None`），**不许**印一条照做只会换个红的死路指示 ✗。
            _want = closure_target(_d)
            if _want is None:
                _hint = (f"⚠ 来源处置 `{_d}` 是**派生态**（`TRANSITIONS` 里没有它的入口）⇒ "
                         f"表里**没有能平账的迁移**：要么把事件表里 `{ident}` 的这几条删掉"
                         f"（回到来源台账的处置 ✓），要么把来源处置改成 `fix`/`fixed`/`wo-filed`"
                         f"（改完再按对应的收口路走）")
            else:
                _steps = closure_steps(st, _want)
                _hint = (" → ".join(_step_cmd(ident, _s) for _s in _steps) if _steps
                         else "⚠ `TRANSITIONS` 里没有可执行路径 ⇒ 只能手改事件表，并在 commit "
                              "里写明为什么（别再照一条死路指示敲 ✗）")
            _why = {"fixed": "：**修好了没登记** ⇒ 队列多报、`--next` 会把做完的推出来",
                    "wo-filed": "：定案不做的条目还留在调度器里（算成还没做完）",
                    "boundary": "：来源说它是**边界 / 设计如此**（不算缺口）",
                    "control": "：来源说它是**对照臂 / 探针自检**（不算缺口）"}.get(_d, "")
            print(f"✗ `{ident}` 来源处置已是 `{_d}`（**已关账**，不进 open 集合），调度器却停在 "
                  f"`{st}` ⇒ **反向两层漂移**{_why} ✗（收口：{_hint}）", file=sys.stderr)
            bad += 1
        if st_ev == "umbrella":
            kids = cur.get("children", [])
            open_kids = open_children(kids, rows, state, disp)
            if not kids:
                print(f"✗ `{ident}` 是 umbrella 却没有子任务 ⇒ 拆解没落地 ✗", file=sys.stderr)
                bad += 1
            elif not open_kids:
                print(f"⏳ `{ident}` 的子任务全关了 ⇒ 该收口了（`--advance {ident} --to done "
                      f"--commit … --reading …`）", file=sys.stderr)
        if st_ev == "wo_filed" and not cur.get("reason"):
            print(f"✗ `{ident}` 是 wo_filed 却没写理由 ✗", file=sys.stderr)
            bad += 1
    # ⭐ **发版节点的读侧复查**（`#3`，2026-10-14 · 第八次独立复核实测 ✗）—— 与 `repoint` 的
    # 读侧复查（上面那一支）**同一形状**：`#1` 给 `repoint` 补了读侧 ⇒ 手改事件表换 commit
    # 会被咬 ✓；而 `release-cut` 只有**写侧**闸门 ⇒ 手写假 `release-cut` **无人咬** ✗
    #（且 `#2` 写进事件的 `subject`/`patch_id` 一个读者都没有 ✗）。
    # ⚠ **一处定义**：判据在 `cut_audit_problem()`（`--release check` 也调它 ✓）。
    for _rel, _node in sorted(release_nodes().items()):
        _cut_why = cut_audit_problem(_node)
        if _cut_why is not None:
            print(_cut_why, file=sys.stderr)
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
    # ⭐ **祖先性口径 = 断言「必须为空」**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）：
    # 这一行原先打的是「⚠ 存量收口指针 N 条 ⇒ 豁免」（**只报告、不判红**），现在换成**断言**——
    # 存量 60 条 / 38 个 sha 已用 `--repoint` 全量回填到 0 ⇒ **非祖先必须是 0 条** ✓。
    # ⚠ **每次 `--check` 都要打出来**（绿也要打 ✓）：不许静默 —— 上一版的病正是"看不见"。
    if bad_pointers:
        print(f"✗ 收口指针祖先性：**非祖先 {len(bad_pointers)} 条**（口径：存量已全量回填 ⇒ "
              f"**必须 0 条**，**没有豁免** ✗）："
              + " · ".join(f"{i}@{c[:8]}" for i, c, _t in bad_pointers), file=sys.stderr)
    else:
        print(f"✓ 收口指针祖先性：**非祖先 0 条**（口径：口径起点 {ANCESTRY_GATE_SINCE} 之前"
              f"记的存量已用 `--repoint` 全量回填到 0 ⇒ 现在**必须**为 0，**没有豁免** ✓）")
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
#
# ⭐ **切版指针也过证据闸门**（`#2`，2026-10-14 · 第七次独立复核实测 ✗）：`--release cut --commit <sha>`
# 原先**自身一条闸门都没有** ⇒ 干净夹具（3 条 done 入版、`--release check` rc=0）里
# `--commit deadbeef…`（对象**不存在**）与「存在但**非祖先**」**都 rc=0** 并写入 `release-cut` ✗
# —— 入版/收口两处都要求"证据必须是真的"，切版这一处却什么都收 ⇒ **同一规则两处口径**
# （`AGENTS.md` 硬规则 0(b)）⇒ 现在与它们**共用** `closure_pointer_problem()`（存在性 +
# `is-ancestor`），且**闸门在 `check` 之前**（假 sha 先拿"证据是假的"这条诊断 ✓）；
# `release-cut` 事件另记 `subject` + `patch_id`（同 `#1` 的理由：事后能核"切的是哪个内容" ✓）。
#
# ⭐⭐ **"分支必须合回"这条栅栏当前不可满足 —— 如实写在这里，不静默、也不放宽**（`#4`，
# 2026-10-14 · 第七次独立复核实测 ✗）：真仓库此刻有 **15 条** `agent/*` 没合回 main，
# 而 `--force` 只越过「条目数 ≥3」与「多余 worktree 提醒」两条**克制**规则 ⇒
# **`--release check` 现在永远过不了** ✗。口径（二选一里选**如实写清**，不加旁路）：
#   · **不放宽** ✗：栅栏判红是**真的** —— 未合回 = "发出去的不是完整版本"（用户 2026-10-12
#     的原话），加 `--force` 旁路等于把这条守卫关掉 ⇒ **不许为了让它绿而放宽**；
#   · **不静默** ✗：判红信息**逐条**印出 `scripts/agent-worktree.sh land <id>`（可照做 ✓）
#     + 明说「这条栅栏**没有** `--force` 旁路」（免得有人以为加开关就能发 ✓）；
#   · **正确做法 = 先合回**（或对已废弃的分支由**人**决定 `git branch -D`）⇒ 合回之后
#     `--release check` **自然**转绿，不靠任何开关 ✓。
#   ⚠ `--selftest` 有一条臂钉住"`--force` **不**越过这一条"（改前那条臂没有 ⇒ 将来有人
#     偷偷加旁路会当场判红 ✗）。

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
    # ⚠ **长度上限与 `--schedule` 共用同一个谓词**（`_id_length_problem`；`#5b` 的 3003 字符
    # id 也能从这条入口进来 ⇒ 横向排查，别只修一处 ✗）。形状这一关**不扩大**到这里 ✓。
    _shape = _id_length_problem(ident)
    if _shape is not None:
        print(_shape, file=sys.stderr)
        return 1
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
    # ⚠ **形状 + 长度一处定义**（`_id_shape_problem`）：`#5b` 的 3003 字符 id 就是从这里进来的
    # ⇒ 与 `--submit`/`--add-child` 共用同一个谓词（别写第二遍 ✗）。
    _shape = _id_shape_problem(ident)
    if _shape is not None:
        print(_shape, file=sys.stderr)
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
    """**主流受理提交**：applied 必须带 commit（证据）；rejected/deferred 必须带理由。

    ⚠ **`applied` 的 commit 要过与 `cmd_advance` 完全相同的两条硬拒**（2026-10-14 ·
    第六次独立复核实测 ✗）：`applied` 的语义就是"**已进主线**"（`fold` 把它折成 `done`），
    而这里原先**只查"非空字符串"** ⇒ 复核者实测 `--triage EXT-1 --to applied --commit
    deadbeef…deadbeef`（对象**不存在**，`git cat-file -e` rc=128）**rc=0** ⇒ 事件表里留下
    一条 `done + 假指针`，而 `--release add/check` 只看 `done` + 有 commit ⇒ **假证据入版** ✗。
    祖先性同样缺（对象在、历史不在的孪生指针）。
    ⇒ 与 `cmd_advance` **共用 `closure_pointer_problem`**（一处定义 ⇒ 两条口径不可能分叉 ✓）。
    """
    rows = _read_jsonl(EXTRAS)
    if ident not in {r["id"] for r in rows}:
        raise UsageError(f"inbox 里没有 `{ident}`")
    if to == "applied" and not commit:
        print("✗ `applied` 必须带 `--commit <sha>`（**受理 = 已进主线**，没 commit 就是没受理 ✗）",
              file=sys.stderr)
        return 1
    if to == "applied":
        # ⚠ **在写任何东西之前**判（`EXTRAS.write_text` / `append_event` 都在下面）⇒
        # 被拒时**来源与事件表一个字节都不动** ✓（半途而废的写入才是最难查的形状 ✗）。
        _why = closure_pointer_problem(commit)
        if _why is not None:
            print(_why, file=sys.stderr)
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
            # ⚠ **审计字段必须折进来**（`#3`，2026-10-14 · 第八次独立复核实测 ✗）：旧写法只留
            # `{commit, ci, ts}` ⇒ `#2` 写进事件的 `subject`/`patch_id` **一个读者都没有** ✗
            #（"咬不住的守卫等于没有"）⇒ `cut_audit_problem()` 读不到它，读侧复查无从谈起。
            out[ev["id"]]["cut"] = {"commit": ev.get("commit"), "ci": ev.get("ci"),
                                    "ts": ev.get("ts"),
                                    "subject": ev.get("subject"), "patch_id": ev.get("patch_id")}
    return out


def cut_audit_problem(node: dict) -> str | None:
    """**切版事件的读侧复查**（`#3`，2026-10-14 · 第八次独立复核实测 ✗）—— 与 `repoint` 的
    读侧复查**同一形状**（**一处定义**；`--release check` 与 `--check` 都调它 ✓）。

    缺口（两条，同一条病根）：
      · `#2` 给 `release-cut` 记了 `subject` + `patch_id` 并声称「事后能核」✓，而**实测没有
        任何生产代码读这两个字段** ✗（`release_nodes()` 折叠时把它们丢了；唯一读者是自检
        自己的断言 ✓）⇒ **咬不住的守卫** ✗；
      · `release-cut` 与 `repoint` **不对称**：`#1` 给 `repoint` 补了**读侧复查**
        （手改事件表换 `commit` ⇒ `check_plan` 判红 ✓），而 `release-cut` **只有写侧闸门**
        ⇒ **手写一条假 `release-cut` 无人咬** ✗。

    靶子 = 手写 `release-cut`：审计字段（`subject`/`patch_id`）与 `commit` **对不上**
    ⇒ 当场判红 ✗（正常 CLI 路径写不出这个形状：`cmd_release cut` 的两个字段就是从
    `args.commit` **现算**的 ✓）。

    ⚠ **只对带审计字段的** `release-cut` 生效（旧账 / 手写事件没有该字段 ⇒ 不误判 ✓ ——
    与 `check_plan` 的 repoint 复查、`closure_pointer_problem` 的分工同口径）。
    ⚠ `patch_id` 是**主判据**（十六进制 ⇒ 与编码无关）；`subject` 是旁证 ⇒ 两侧都过
    `_subject_text()`（**同一处**折算 —— 不许两边口径不同 ✗）。
    """
    cut = (node or {}).get("cut") or {}
    commit, rec_pid, rec_subj = cut.get("commit"), cut.get("patch_id"), cut.get("subject")
    if not commit or not rec_pid:
        return None
    now_pid, now_subj = _patch_id(commit), _subject_text(_commit_subject(commit))
    if now_pid != rec_pid or (rec_subj and now_subj != rec_subj):
        return (f"✗ 发版节点 `{(node or {}).get('id')}` 的 `release-cut` 与它的审计字段"
                f"**对不上**：账上记的 subject/patch-id = `{(rec_subj or '')[:40]}` / "
                f"`{rec_pid[:12]}`，而 `{commit[:12]}` 实际是 `{now_subj[:40]}` / "
                f"`{(now_pid or 'N/A')[:12]}` ⇒ 切版指针被**事后换过**（手改事件表？）✗ "
                f"—— `release-cut` 的审计字段就是为这条复查留的 ✓")
    return None


def release_node_ids() -> set[str]:
    """**发版节点**的 id 集合（`release-open` 建的 ⇒ `--release open` 是唯一出处）。

    ⚠ 为什么幽灵判据必须认识它（`#2`，2026-10-14 · 第八次独立复核实测 ✗）：
    `cmd_release open` 原先**只写 `release-open` 事件、不登记来源行**，而幽灵判据
    （`check_plan`）只认三个来源（缺口台账 / 索引 / extras）⇒ 开节点后**下一条 `--check`
    当场判红**「`REL-9.9.9` 不在任何来源里 ⇒ 幽灵条目」✗ —— 而 `scripts/soko gate --fast`
    与 CI 的 `ledger` job 跑的正是 `--check` ⇒ **§9.20 发版工作流第一步就撞红** ✗
    （实测：开节点前 rc=0 ⇒ 开节点后 rc=1；三个 `selftest_release_*` 都只调 `cmd_release`、
    **从不调 `check_plan`** ⇒ 这条接缝没有任何自检跑过 ✗）。

    口径（**二选一，选这条**）：**让幽灵判据认识发版节点** ✓ —— 而**不是**给 `open`
    补一条来源行 ✗。理由：来源行 = **工作条目**（会进 `--next` / `--json.open` /
    §0.2 的"未完成"计数）⇒ 补来源行等于把**节点**算成"没做完的活"，那是**为绿而改语义** ✗；
    节点不是工作条目，它的出处就是它自己的创建事件 ✓。

    ⚠ **不是放水**（自检 `selftest_release_open_seam` 逐条钉着）：
      · 只认 `release-open`（**节点自己的创建事件**，形状固定、只能由 `cmd_release` 写）；
      · 手写一条 `release-open` 与手写一条来源行**同权**（都是显式登记）—— 而**没有**
        `release-open` 的 id（例如手写一条 `release-add` 引用一个不存在的条目）**照样判红** ✓。
    ⚠ **一处定义**：节点集合由 `release_nodes()` 算（它就是"发版节点是什么"的唯一出处）
    ⇒ 不在这里再扫一遍事件、也不另写一套判据（`AGENTS.md` 硬规则 0(b)）✓。
    """
    return set(release_nodes())


def unmerged_agent_branches() -> list[str]:
    """`main` 上**没合回**的 `agent/*` 分支（发版栅栏的输入；抽成函数便于 selftest 隔离 ✓）。

    ⚠ **这条栅栏没有 `--force` 旁路**（`#4` 的口径，见 `cmd_release` 的说明）：发版要的是
    "**完整**版本" ⇒ 未合回的分支就是"还差东西"，`--force` 只越过**条目数**与 worktree
    两条（那两条是"克制"），不越过"并行收拢未完成" ✗。
    """
    return subprocess.run(["git", "branch", "--no-merged", "main", "--list", "agent/*",
                           "--format=%(refname:short)"], cwd=ROOT, capture_output=True,
                          text=True, encoding="utf-8").stdout.split()


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
              f"   ③ `--release check` 过一遍，再 `--release cut --commit <sha> --ci <run>`"
              f"（切版指针也过**证据闸门**：存在性 + `is-ancestor` ✓）")
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
        # ⭐ **证据闸门**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）：缺口 = 原先这里**只看
        # "done + 有 commit"** ⇒ **手写的假指针能进发布节点** ✗（第六次独立复核实测：
        # `release_add_rc=0`，即使那个 commit 对象**不存在**）。而 `#2` 之后任何 CLI 写路径
        # 都造不出该形状（`cmd_advance`/`cmd_triage` 都有存在性 + 祖先性硬拒，且闸门**在写之前**）
        # ⇒ **只剩手改事件表一条路**，所以闸门必须也长在**入版**这一侧。
        # ⚠ **共用 `closure_pointer_problem()`**（别自己再写一遍 ✗ —— 那正是硬规则 0(b) 的病：
        # 同一规则两处写 ⇒ 下次只改一处又分叉）。
        _why = closure_pointer_problem(cur["commit"])
        if _why is not None:
            print(_why, file=sys.stderr)
            print(f"  ⇒ **入版闸门与收口闸门同口径**（都走 `closure_pointer_problem()` ✓）："
                  f"存量孪生指针用 `--repoint {args.id} --reason '<why>'` 回填（三关自动找孪生），"
                  f"或先把这个环节**真的**提交掉再入版 ✓", file=sys.stderr)
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
        # ⭐ **节点级读侧复查**（`#3`，2026-10-14 · 第八次独立复核实测 ✗）：`release-cut` 的
        # 审计字段（`subject`/`patch_id`）与 `commit` **对不上** ⇒ 判红（手写假 `release-cut`
        # 原先**无人咬** ✗）。⚠ 与 `--check` **共用** `cut_audit_problem()`（一处定义 ✓）。
        _cut_why = cut_audit_problem(node)
        if _cut_why is not None:
            print(_cut_why, file=sys.stderr)
            bad += 1
        for ident in node["items"]:
            cur = state.get(ident, {})
            if cur.get("state") != "done" or not cur.get("commit"):
                print(f"✗ 入版条目 `{ident}` 未 done 或无证据 ⇒ **发出去就不是完整可用版本** ✗",
                      file=sys.stderr)
                bad += 1
                continue
            # ⭐ **读侧也过同一道证据闸门**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）：
            # `add` 只挡"**新**入版的"；手改事件表把现成条目的指针换掉、或把条目**手写进**
            # `release-add` 事件 ⇒ 只有这里咬得住。两处**共用** `closure_pointer_problem()` ✓。
            _why = closure_pointer_problem(cur["commit"])
            if _why is not None:
                print(f"✗ 入版条目 `{ident}` 的证据过不了闸门：{_why}", file=sys.stderr)
                bad += 1
        live = live_claims()
        if live:
            print(f"✗ **收拢未完成**：还有 {len(live)} 个在飞认领 "
                  f"{[c.get('id') for c in live]} ⇒ 等它们收口（或 `--release block` 记录）✗",
                  file=sys.stderr)
            bad += 1
        branches = unmerged_agent_branches()
        if branches:
            # ⭐ **这条栅栏没有 `--force` 旁路**（`#4` 的口径，2026-10-14 · 第七次独立复核实测 ✗）：
            # 实测真仓库有 **15 条** `agent/*` 没合回 main，而 `--force` 只越过「条目数」与
            # 「worktree 提醒」两条**克制**规则 ⇒ `--release check` **当前永远过不了** ✗。
            # 处置 = **如实说清 + 给出路**（不静默、也不为让它绿而放宽 ✗）：
            #   · 栅栏判红是**真的**（未合回 = 版本不完整）⇒ 不许放宽；
            #   · 出路 = 把活**先合回**（逐条命令印出来 ⇒ 可照做 ✓）；
            #   · 明确印出"`--force` **不**越过这一条"⇒ 没人会以为加个开关就能发 ✓。
            print(f"✗ **并行收拢未完成**：还有 {len(branches)} 条 agent 分支没合回 main "
                  f"⇒ 发版要的是**完整**版本（`#4`：这条栅栏**没有** `--force` 旁路 —— "
                  f"`--force` 只越过「条目数 ≥{MIN_ITEMS_PER_RELEASE}」与「多余 worktree 提醒」"
                  f"两条**克制**规则，**不**越过「分支必须合回」✗）", file=sys.stderr)
            for _b in branches:
                _id = _b.split("/", 1)[1] if "/" in _b else _b
                print(f"    `{_b}` ⇒ `scripts/agent-worktree.sh land {_id}`", file=sys.stderr)
            print(f"  ⇒ 正解 = **先合回**（逐条 land ✓）；已废弃的分支就**删掉它**"
                  f"（`git branch -D <branch>`，那是人的决定 —— 脚本不替你做 ✗）⇒ "
                  f"合回之后 `--release check` 自然转绿（**不靠开关** ✓）", file=sys.stderr)
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
        # ⭐ **切版指针也要过证据闸门**（`#2`，2026-10-14 · 第七次独立复核实测 ✗）：
        # 干净夹具（3 条 done 入版 · `--release check` rc=0）里 `--release cut --commit deadbeef…`
        # （对象**不存在**）与「存在但**非祖先**」**都 rc=0** 并写入 `release-cut` ✗
        # ⇒ **同一规则两处口径**（`AGENTS.md` 硬规则 0(b)）⇒ 与收口/入版**共用**
        # `closure_pointer_problem()`（别自己再写一遍 ✗）。
        # ⚠ 放在 `check` **之前**：假 sha 该拿到"证据是假的"这条诊断，而不是先看一屏栅栏读数 ✓。
        _why = closure_pointer_problem(args.commit)
        if _why is not None:
            print(f"✗ `--release cut --commit {args.commit[:12]}` 过不了**证据闸门**：{_why}",
                  file=sys.stderr)
            print(f"  ⇒ 切版指针与收口/入版指针**同一处口径**（都走 `closure_pointer_problem()` ✓）："
                  f"切版 commit 必须是 main 上**真实存在、且是 `HEAD` 祖先**的那一个 ✗ "
                  f"（假 sha / 别的分支上的 sha 都不算判据）", file=sys.stderr)
            return 1
        if cmd_release(argparse.Namespace(**{**vars(args), "release": "check"}), rows) != 0:
            return 1
        # ⚠ **切版事件也留审计字段**（同 `#1` 的理由：`{commit, ci}` 事后核不出"切的是哪个内容"）：
        # `release-cut` 事件记 `subject` + `patch_id` ⇒ 账上能复查"当时切的是哪一个 commit" ✓。
        append_event({"event": "release-cut", "id": rel, "commit": args.commit, "ci": args.ci,
                      "subject": _commit_subject(args.commit), "patch_id": _patch_id(args.commit),
                      "owner": args.owner})
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
      ③ 源 `fixed` + 调度器非终态 ⇒ `check_plan` **判红**（反向两层漂移对子任务咬得住 ✓；
         **点名 `K-2`** ⇒ 读数可归属 ✓）；
      ③b **无源子任务判红**（2026-10-14 · 第六次复核 #1）：`K-3` 是"只有 `split` 事件、
         三处来源都没有"的子任务 ⇒ 判红 + 点名 + 说清「无源子任务」；随后**登记进来源**
         （真工作流的形状）⇒ ⑥/⑦ 才可能转绿 ✓。**改前这一臂静默**（合成行给自己作出处 ✗）。
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
            _buf, _err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(_buf), contextlib.redirect_stderr(_err):
                _rc3 = check_plan(view, rows)
            expect(f"③ 源 `fixed` + 调度器 `{state['K-2']['state']}` ⇒ 反向两层漂移判红"
                   f"（**点名 K-2** ⇒ 读数可归属 ✓；实测红 {_rc3} 条）",
                   _rc3 >= 1 and "K-2" in _err.getvalue()
                   and "反向两层漂移" in _err.getvalue())
            payload = json_view(rows, state, view)
            expect(f"④ 合成子任务（K-3 没源行）必须在 `--json` 的 open 里（实测 {payload['open']}）",
                   "K-3" in payload["open"])
            expect("④ `--json` 的 open == 视图的 open 集合（单一真相）",
                   payload["open"] == sorted(open_set(view)))
            shadow = [{**r, "disposition": "fix"} if r["id"] == "K-2" else r for r in view]
            expect("⑤ 视图那一行被遮蔽时，处置仍以**来源**为准 ⇒ 照样判红",
                   check_plan(shadow, rows) >= 1)
            # ⚠ **无源子任务现在判红**（2026-10-14 · 第六次复核 #1）：这个夹具原先靠"合成行
            # 给自己作出处"让 K-3 **静默** —— 那正是病根本身 ✗ ⇒ 先断言它**被判红并点名**，
            # 再按真工作流把它**登记进来源**（否则 ⑥/⑦ 的"转绿 / 三视图同数"永远不成立 ✗）。
            _b3, _e3 = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(_b3), contextlib.redirect_stderr(_e3):
                _rc3b = check_plan(view, rows)
            expect(f"③b 无源子任务 `K-3` ⇒ **判红 + 点名**（改前静默 ✗；实测红 {_rc3b} 条）",
                   _rc3b >= 1 and "K-3" in _e3.getvalue()
                   and "无源子任务" in _e3.getvalue())
            with EXTRAS.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(
                    {"id": "K-3", "source": "ux-index", "facet": "语言", "severity": "medium",
                     "repro": "bash z3.sh", "disposition": "fix",
                     "why": "夹具：拆解出来的子任务**要登记来源**（第六次复核 #1）"},
                    ensure_ascii=False) + "\n")
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
            # ⚠ 这里**不许再手抄一遍**可动口径（2026-10-14 · 第五轮复核的同族第五处）：
            # 原先这行把 `cmd_next` 的判据抄了一遍 ⇒ "哪些条目可动"有两个真相。
            _pushable = pushable_ids(view2, state2)
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
      ④ **终态行在队列里、但不在 open 里**（2026-10-14 · 第五次复核**改了这条的口径**）：
         改前这条写的是「终态行**仍不进** `build_queue()` 的 rows」—— 那正是**病根的口径** ✗
         （终态缺口行不进合并 ⇒ ①「一个 id 只能有一个处置」的闸门看不见它、②`--next`/`--json`
         拿不到权威行 ⇒ 同一个 `effective_state` 喂两种行得两个状态 ✗）。现在的口径与
         docstring 的「处置」一节同源：**已关账的条目留在队列里**（会计上不许删 ✗）
         **但不在 open / §0.2 / `--next` 里** ✓；要判的是**漂移**（由 `check_plan` 出）✓。
      ⑤ **一份输入**：`build_queue()` 的键 = 三个来源的全部 id（∪ 未定处置）—— 改前终态缺口行
         **不在** `rows` 里 ⇒ 这条必红 ✗（它咬住的正是"冲突闸门对终态缺口行瞎"那个历史 bug）；
      ⑥ **同族第二处**：`--schedule` 的"id 是键"闸门也看得见**终态** id（已关账缺口的 id
         不许被重新排期 ⇒ 那是覆盖历史 ✗）；
      ⑦ **索引行不许静默丢**：§2–§4 里少一列的数据行 ⇒ 响亮报错（点名行号 + 列数）。
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

            # ④ **终态行在队列里、但不在 open 里**（口径在第五次复核改了，见 docstring）：
            #    改前这条断的是"终态行**仍不进** `build_queue()` 的 rows"——**那正是病根** ✗
            #    （终态缺口行不进合并 ⇒ 冲突闸门看不见它、`--next`/`--json` 拿不到权威行）。
            #    现在与 docstring「处置」一节同源：**已关账的条目留在队列里**（会计上不许删 ✗）
            #    **但不在 open / §0.2 / `--next` 里** ✓；要判的是**漂移**（由 `check_plan` 出）。
            #    ⚠ 两种形状要分开说（别把"无事件"与"有事件"混成一条断言 ✗）：
            #      · `G-8`（终态缺口 + **零事件**）⇒ 状态由**处置派生** `done` ⇒ **不进 open、
            #        不被推**（改前它连 `rows` 都进不去 —— 那正是冲突闸门瞎掉的原因 ✗）；
            #      · `G-9`（终态缺口 + `split` 事件 ⇒ 调度器 `todo`）⇒ 视图行**用源行**（真处置
            #        `fixed`，**不是**合成行的 `fix` ✗）⇒ `check_plan` 判「反向两层漂移」（②已断），
            #        而它**仍在 open / 被推**是**对的**：调度器说它没做完 ⇒ 两个视图与那条红
            #        讲的是**同一个故事**（按收口指示 `--advance … --to done` 走完一起消失 ✓）。
            payload = json_view(rows, state, view)
            expect(f"④ 终态缺口行**在** `build_queue()` 的 rows 里（面板 = 会计账，不许删 ✗；"
                   f"实测 ids={sorted(r['id'] for r in rows)}）",
                   {r["id"] for r in rows} == {"P-9", "E-9", "I-9", "G-9", "G-8"})
            expect(f"④ open/§0.2 = **来源口径**（`disposition ∈ OPEN_DISPOSITIONS`）⇒ 只有父 "
                   f"`P-9`（{payload['open']}；total={_s['total']}）—— 终态缺口行一律不算未完成 ✓",
                   payload["open"] == ["P-9"] and _s["total"] == len(payload["open"]) == 1
                   and "QUEUE:COUNTS total=1 " in render_onboarding(view))
            _g9 = next(r for r in view if r["id"] == "G-9")
            expect(f"④ `G-9` 的视图行 = **源行**（真处置 `{_g9.get('disposition')}`，不是合成行的 "
                   f"`fix` ✗）—— 改前它被 `load_gaps()` 默认口径丢掉 ⇒ 合成行硬编码 `fix` ⇒ "
                   f"`--json.open` 说它没做完、而来源说它已关账（同一个 id 两个状态 ✗）",
                   _g9.get("disposition") == "fixed")
            # ⚠ **边界（如实记，不许说成"一致" ✗）**：`G-9` **有** `split` 事件 ⇒ 调度器说它
            #   `todo` ⇒ `--next` 推它（`is_pushable` 是**调度器口径**），而 `open` 是**来源
            #   口径**（`fixed` ∉ OPEN）⇒ 两边**故意不同**：这个 id 正处在漂移里，`check_plan`
            #   为它判红并给出收口指示（②已断）⇒ 按指示走完两处一起收敛 ✓。口径本身在
            #   `selftest_child_drift` ⑦ 钉着（**绿的时候** `--next` ⊆ `open`）。
            expect(f"④ `G-9` 被 `--next` 推（调度器 `{state['G-9']['state']}`）而**不在** open "
                   f"（来源 `fixed`）—— 这正是②那条红在说的形状（不是静默：`--check` 点名它）",
                   "G-9" in pushable_ids(view, state) and "G-9" not in open_set(view))
            _leak4 = sorted(i for i in ("G-8", "E-9", "I-9")
                            if i in open_set(view) or i in pushable_ids(view, state))
            expect(f"④ **零事件**的已关账行（`G-8`/`E-9`/`I-9`）既不进 open、也不被 `--next` 推"
                   f"（实测漏进去的：{_leak4 or '无'}）—— 改前 `G-8` 连 `rows` 都进不去 ✗",
                   not _leak4)

            # ⑤ **一份输入**（第五次复核的根因）：队列（合并）的键 = 三个来源的**全部** id
            #    ⇒ 冲突闸门与漂移规则不可能再看漏一行。⚠ 这条是**结构性**守卫：它咬住的正是
            #    "终态缺口行不进 `build_queue()` ⇒ 冲突闸门对它瞎"那个历史 bug（改前必红 ✗）。
            #    ⚠ 比对的是**加载器**（不是 `source_dispositions()`）—— 拿派生视图比自己会
            #    变成恒真式（tautology），咬不住任何东西 ✗。
            _all_ids = ({e["id"] for e in load_extras()}
                        | {e["id"] for e in load_gaps(include_terminal=True)}
                        | {e["id"] for e in load_index()})
            _in_queue = {r["id"] for r in rows} | set(_undecided)
            expect(f"⑤ 队列的键 == 三个来源的全部 id（{len(_in_queue)} vs {len(_all_ids)}；"
                   f"漏的：{sorted(_all_ids - _in_queue) or '无'}）—— 改前终态缺口行不在里面 ✗",
                   _in_queue == _all_ids)
            expect("⑤ 权威表是**同一份**的投影（`source_dispositions()` ⊇ 队列键；不再是自己"
                   "读一遍来源 ⇒ 不可能与视图两个口径）",
                   set(source_dispositions()) == _all_ids)

            # ⑥ **同族第二处**（AGENTS.md 硬规则 0(b)）：`--schedule` 的"id 是键"闸门也得
            #    看得见**终态** id —— 否则一个已关账缺口的 id 能被重新排期（= 覆盖历史 ✗）。
            #    ⚠ 夹具必须是**没有任何事件**的终态 id（`G-8`）：`G-9` 有 `split` 事件 ⇒
            #    `fold()` 里就有它 ⇒ 旧口径也能拒 ⇒ 这条断言会**空转**（实测踩到过一次 ✗）。
            #    ⚠ 这一条必须在 ⑦ **之前**跑：它要 `source_dispositions()` 能正常读索引 ✓。
            args = argparse.Namespace(schedule="x", id="G-8", title="夹具", reason="夹具",
                                      owner="t", where="", facet="", severity="", repro="")
            rc3, _out3, err3 = out_err(cmd_schedule, args)
            expect(f"⑥ 已 fixed、**无事件**的缺口 id 不许被 `--schedule` 重用（rc={rc3}）",
                   rc3 == 1 and "已经在队列里" in err3)
            # ⑥b **同族第二处**（2026-10-14 · 第三轮复核实测）：`--submit` 的同一闸门
            #     原先只看 `_read_jsonl(EXTRAS)` ⇒ gap `fixed` 的 id **rc=0 被接受** ✗，
            #     而同一时刻 `--schedule` 拒 ✓（同一个病只修了一处）。⇒ 两条来源臂都用
            #     **无事件**的 id 钉住（`G-8` = 缺口 `fixed`；`I-8` = 只在索引里、没处置），
            #     外加**正对照**：一个三来源都没有的新 id 必须仍被接受（防"闸门变成全拒" ✗）。
            def _submit_ns(ident: str) -> argparse.Namespace:
                base = dict(submit="fix", id=ident, why="夹具", evidence="rc 0", patch="",
                            facet="", severity="", repro="", where="夹具", owner="t")
                return argparse.Namespace(**base)

            rc4, _out4, err4 = out_err(cmd_submit, _submit_ns("G-8"))
            expect(f"⑥b 已 fixed、**无事件**的缺口 id 不许被 `--submit` 重用（rc={rc4}；"
                   f"改前 rc=0 静默接受 ✗）", rc4 == 1 and "已经在队列里" in err4)
            rc5, _out5, err5 = out_err(cmd_submit, _submit_ns("I-8"))
            expect(f"⑥b 只在索引里（没处置）的 id 也不许被 `--submit` 重用（rc={rc5}）",
                   rc5 == 1 and "已经在队列里" in err5)
            rc6, _out6, _err6 = out_err(cmd_submit, _submit_ns("SUB-新-9"))
            expect(f"⑥b 正对照：三来源都没有的新 id 仍可提交（rc={rc6}）", rc6 == 0)

            # ⑦ 索引里的数据行少一列 ⇒ 响亮报错（不许静默丢）
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
                expect("⑦ 索引 §2 里少一列的数据行 ⇒ 响亮报错（不许 `continue` 静默丢）", False)
            except UsageError as exc:
                expect(f"⑦ 索引里少一列的数据行 ⇒ 响亮报错（点名 I-7 + 列数）：{exc}",
                       "I-7" in str(exc) and "6 列" in str(exc))
        finally:
            (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING) = saved[:5]
            globals()["lock_file"] = saved[5]
            globals()["probe_inventory"] = saved[6]


# ── 「来源 × 处置 × 调度器状态」网格（`--selftest` 的 ⑤）的**三轴** ────────────
# ⚠ **加来源 / 加处置 / 加调度器状态只改这里**（格数与自检摘要里的条数都从它算 ✓）——
# 这一处就是"以后不许再只修一处"的那一处（AGENTS.md 硬规则 0(b)）。
GRID_PAIRS = (("GF", "gap", "fixed"), ("GW", "gap", "wo-filed"),
              ("EF", "extras", "fixed"), ("EW", "extras", "wo-filed"),
              ("IF", "索引+extras", "fixed"), ("IW", "索引+extras", "wo-filed"),
              # ⭐ 第三轮的网格只覆盖 `{fixed, wo-filed}` ⇒ docstring 里另外两个**并称
              # "已关账"**的处置（`boundary`/`control`）**结构上不可能被网格看见** ✗
              # （第五轮复核 ②：真台账唯一一行 `r02` 就是 `control` ⇒ "一条命令之遥"）
              # ⇒ 来源轴扩到**全部四个已关账处置**，由结构性守卫钉住"一个都不许漏" ✓。
              ("EB", "extras", "boundary"), ("EC", "extras", "control"))
GRID_STATES = (("todo", "todo"), ("inprog", "in_progress"), ("umb", "umbrella"),
               ("blk", "blocked"), ("sub", "submitted"), ("trej", "triage_rejected"))
# ⭐ **第三轴 = 无事件 / 有事件**（第五轮复核 ①，**最重的一族**）：有事件那一轴由
# `GRID_STATES` 的 6 个状态构成（每格都用**真事件**造出来 ✓）；**无事件**那一轴
# （= docstring 明说的合法操作"删掉事件文件就回到来源台账的处置" ✓）的状态由**来源处置
# 派生**（`DISPOSITION_STATES`）⇒ 每个 pair 一格，**非缺口来源**的 pair 再加一格带
# `external-submission` 标记（`effective_state` 的那个短路分支正是这一族 42 格的病根 ✗；
# 缺口来源的行 `sources=["gap-ledger"]` ⇒ 没有这个标记，不硬凑 ✓）。
GRID_LAYERS = (("ev", "有事件（事件造状态）"), ("noev", "无事件（处置派生状态）"))
GRID_CELLS = len(GRID_PAIRS) * len(GRID_STATES)          # 48 = 8 × 6（**有事件**层）
GRID_NOEVENT_CELLS = len(GRID_PAIRS) + sum(              # 14 = 8 + 6（非缺口来源的标记格）
    1 for _p, _s, _d in GRID_PAIRS if _s != "gap")
# 无事件层的**正对照**（防"整层恒绿"的空转夹具 ✗）：**开着**的处置 ⇒ 派生 `todo`/`submitted`
# ⇒ 必须**可推**且**在 open 里**（否则"已关账的都不推"这条断言等于没测 ✗）。
GRID_NOEVENT_OPEN = (("PF", "fix", None), ("PFx", "fix", "external-submission"),
                     ("PS", "scheduled", None))
# ⭐ **第四轴 = 终态层**（第五轮复核 ④）：`GRID_STATES` 只覆盖**非终态**（结构性守卫 ⓪① 钉着），
# 而「来源 `fix` + 调度器 `wo_filed`」这一族住在**终态**里 —— 改前 **`done` 那一向有牙、
# `wo_filed` 这一向一条都没有** ✗（`--check` 静默 · `--next` 不动它 · 而 `--json.open`/§0.2
# 永远报"还没做完"）。⇒ 终态层逐格钉：来源记的是**工作结论**（`fixed`/`wo-filed`）⇒ 一致 ✓；
# 否则（`boundary`/`control` = "从来不算缺口"，或还开着的 `fix`/`scheduled`）⇒ **正向两层漂移** ✗。
GRID_TERMINAL = (("done", "done"), ("wof", "wo_filed"))
GRID_TERMINAL_CELLS = len(GRID_PAIRS) * len(GRID_TERMINAL)
GRID_TERMINAL_OPEN = (("PF", "fix"), ("PS", "scheduled"))    # 正对照（开着的处置 + 终态 ⇒ 必红）
GRID_TERMINAL_PAIR_REDS = (sum(1 for _p, _s, d in GRID_PAIRS
                               if d not in WORK_DONE_DISPOSITIONS) * len(GRID_TERMINAL))
GRID_TERMINAL_CTRL_REDS = len(GRID_TERMINAL_OPEN) * len(GRID_TERMINAL)
GRID_TERMINAL_REDS = GRID_TERMINAL_PAIR_REDS + GRID_TERMINAL_CTRL_REDS
GRID_REDS = GRID_CELLS + GRID_TERMINAL_REDS               # 48 + 8 = 56（判红格总数）
# ⭐ **第五轴 = 跨来源处置冲突**（2026-10-14 · 第五次独立复核实测 ✗）：上面四层每一格都是
# **单来源**（一个 id 只被一个来源登记）⇒「**同一个 id 被两个来源登记、处置不一致**」这一族
# 在网格里**结构上看不见** ✗。它只有**终态缺口行**能造出来（`load_gaps()` 默认把
# `status=="fixed"` 丢掉），后果是**同一个 `effective_state` 喂两种行得两个状态**：
# `--next`/`--json` 读视图行（extras `fix`）⇒ 推 `todo`；`check_plan` 读权威行（gap `fixed`）
# ⇒ 判 `done` ⇒ 两条漂移规则都放过（rc=0 静默 ✗），而 gap `wo-filed` 的同样冲突 rc=2 ✓。
# ⇒ 轴 = **视图开着的处置 × 缺口加载器的值域**里**所有不等对**（相等对不是冲突 ⇒ 排除）：
#   · 值域 = `GRID_GAP_DISPOSITIONS`（`load_gaps()` 只会给 `fixed`/`wo-filed`/`scheduled`），
#     逐对在夹具里用**真加载器**核一遍（见 ⑨ 的 `load_gaps` 断言 ⇒ 不是"我以为它会这么翻译" ✗）；
#   · 状态轴 = `GRID_CONFLICT_STATES`（无事件 / done / wo_filed）—— 冲突是**输入层**的事，
#     调度器状态**一格都不许**改变结论（"只在某一态静默"那一族正是改前的形状 ✗）。
GRID_GAP_DISPOSITIONS = ("fixed", "wo-filed", "scheduled")
GRID_CONFLICTS = tuple((_v, _a) for _v in sorted(OPEN_DISPOSITIONS)
                       for _a in GRID_GAP_DISPOSITIONS if _v != _a)
GRID_CONFLICT_STATES = (("noev", None), ("done", "done"), ("wof", "wo_filed"))
# = **5 × 3 = 15 格**（`|OPEN_DISPOSITIONS|` 2 × `GRID_GAP_DISPOSITIONS` 3 − **1 个相等对**
#   `scheduled`×`scheduled`）。⚠ 这一行注释原写 `4 × 3 = 12`，与实测 **5 × 3 = 15** 不符 ✗
#   （2026-10-14 · 第六次独立复核实测；自检收尾打印里的两个因子**本来就是 `len()` 算的**
#   ⇒ 它一直印 15 ✓，只有这处注释在漂）⇒ 注释改成由表推出来的口径，**别手抄因子** ✗。
GRID_CONFLICT_CELLS = len(GRID_CONFLICTS) * len(GRID_CONFLICT_STATES)


def selftest_source_state_grid(expect) -> None:
    """**来源 × 处置 × 调度器状态的三轴网格**（2026-10-14 · 第三/五轮独立复核 ⑤①②）：
    漏格不许活到自检全绿。

    事故一（第三轮）：漂移规则漏了 `submitted` 那一格 —— 复核者实测
    {fixed,wo-filed} × {todo,in_progress,umbrella,blocked} **24/24 判红**，而 `submitted`
    那一列 **6/6 静默**（同一时刻 `--next` 照样把那条推出来 ✗）；病根是规则里写死的
    `and st != "submitted"`。而**旧夹具只造两种状态**（`split` ⇒ todo、`claim` ⇒
    in_progress）⇒ 漏格能一路活到自检全绿 ✗（`AGENTS.md` 硬规则 0(b)：收到一条 ⇒
    先问"同类还有哪些"，一次改齐 + 落**可查**的守卫）。

    事故二（第五轮）：网格**每格都用事件造状态** ⇒ 「**无事件**」那一族**结构上不可见** ✗，
    而它正是最重的一族 —— 真台账上按 docstring 的合法操作删掉事件文件 ⇒ `--check`
    **rc=0 · stderr 0 行**，而 `--next` 照样推 **42 条**（42/42 的权威处置都是 `fixed` =
    已关账）✗。病根两条：① 谓词只认两个处置而 docstring 说**四个**已关账；②
    `effective_state` 把 `external-submission` 短路放在处置表**之前** ⇒ 已关账的处置被
    翻译成 `submitted`（非终态）。

    ⇒ 夹具 = **三轴**：8 个「来源 × 处置」组合（3 来源 × docstring 的**全部四个**已关账处置）
    × 6 个调度器状态 × 2 层，逐格断言：

      **有事件层**（`GRID_CELLS` = 8 × 6 = 48 格，每格用真事件造状态）：
      ① 判红**并点名**该 id + 「反向两层漂移」（改前 `submitted` 那 6 格静默 ⇒ 这条必红 ✗）；
      ② 判红信息里的**收口指示逐步走一遍**都合法、且落终态（改前 `todo`/`blocked` 的
         `→done` 与 `triage_rejected` 的 `→wo_filed` 是死路 ⇒ 这条必红 ✗）；而
         `boundary`/`control` 在 `TRANSITIONS` 里**没有入口** ⇒ 指示必须**如实说"没有能平账
         的迁移"**且**不许**印 `--advance`（⑥ 再把那条老"死路"真走一遍 ⇒ 仍然红 ⇒ 证明
         不印它是对的 ✓，不是偷懒 ✗）。
      **无事件层**（`GRID_NOEVENT_CELLS` = 14 格，**一事件都不写** —— 这一层就是
      "删掉覆盖层"那一臂的守卫 ✓）：
      ③ 权威状态 = 处置派生出的状态（`fixed`→done · `wo-filed`→wo_filed ·
         `boundary`→boundary · `control`→control）—— **改前**带 `external-submission`
         标记的已关账格会给 `submitted` ⇒ **这条必红** ✗；
      ④ **不可推**（`is_pushable` 假）且**不在** `--json.open`/§0.2 里 ⇒ **三视图同数** ✓；
      ⑤ 判定域含这些**无事件**的 id（`plan_domain` = 视图 ∪ 事件表）—— 域退回"只遍历事件表"
         ⇒ **这条必红** ✗（这正是第五轮那个洞本身）；
      ⑥ 正对照：开着的 3 格**必须**可推 + **必须**在 open 里（防"整层恒绿"的空转夹具 ✗）；
      ⑦ 子任务"关了没有"与 `--check` **同一口径**（`open_children`）。
      **形状/正对照**：48 格状态都造对 + 12 格都**没有事件** + 八个来源终态在全量权威表里
      看得见 + 非终态来源的 8 个子任务一个都不许红 + 红条数恰好 48（没有多余红）
      ⇒ 防"夹具空转"与"守卫放宽"两类假绿 ✓。

    结构性守卫（纯表、不碰夹具）：⓪① 覆盖**所有**非终态 · ⓪② 每个非终态都有**到终态的
    合法路径** · ⓪③ 覆盖**所有**处置（`DISPOSITIONS`）· ⓪④ "已关账"恰好是 docstring 的
    那四个（`DISPOSITIONS` 的**不相交划分**）· ⓪⑤ **冲突轴**覆盖**所有**「视图处置 × 缺口
    处置」不等对 + 状态轴含「无事件 + 两个终态」—— 加状态 / 加处置而忘了补网格或忘了归类
    ⇒ **先红** ✗。

    **第五轴 = 跨来源处置冲突**（2026-10-14 · 第五次独立复核实测 ✗）：上面每一格都是
    **单来源**（一个 id 只被一个来源登记）⇒「**同一个 id 被两个来源登记、处置不一致**」
    这一族**结构上看不见** ✗ —— 而它只有**终态缺口行**造得出来（`load_gaps()` 默认把
    `status=="fixed"` 丢掉）⇒ ①「一个 id 只能有一个处置」的闸门（长在 `build_queue()` 的
    合并里）对它**瞎**；②`check_plan` 的权威表走**另一条**全量读法 ⇒ 同一个
    `effective_state` 喂两种行得两个状态（`--next` 推 `todo`、`--check` 判 `done`）✗。
    复核者 A/B：同一个 done 事件 + 同一个 extras(`fix`) 行，**只加一行 gap `status=fixed`**
    ⇒ 改前 rc=1 → **rc=0 静默** ✗（gap `wo-filed` 的同样冲突改前就 rc=2 ✓ = 闸门有牙、
    只是被那个过滤器绕开）。⇒ 逐格（`GRID_CONFLICTS` × `GRID_CONFLICT_STATES` = **12 格**）断：
      a) **单来源臂**（A/B 的 A）**不许**判成冲突，且读数 = 漂移规则的正常判据（无事件 0、
         `done`/`wo_filed` 1）⇒ 判红**可归属**到"多了一行终态登记"（防整轴恒红的空转夹具 ✗）；
      b) **双来源臂**（B）**rc=2 当场判红**、点名该 id + 两个处置 + **各自出处**；
         `fixed` 与 `wo-filed` 两支**逐格同判** ⇒ **不许更松** ✗；
      c) `--next`/`--json` 与 `--check` **走同一条闸门**（同一个输入 ⇒ 不可能"一个命令静默、
         另一个推得出来" ✗）；
      d) 轴的前提由**真加载器**核（`load_gaps(include_terminal=True)` 把夹具那行翻成同一个
         处置）⇒ 不是硬编码假设 ✓。
    判据打在**真 CLI 路径**（`main()` + `sys.argv`，`--check-plan` 正是 `soko gate`/CI 跑的
    `--check` 里的同一条）⇒ 咬的是用户敲的那条命令 ✓。
    """
    import contextlib
    import io
    import tempfile
    # ⓪ **结构性守卫**（纯表、不碰夹具）：漏格不许再靠"人记着补" ✗
    #  ① 网格必须覆盖**所有**非终态 —— 将来往 `SCHED_STATES` 加一个状态而忘了补网格 ⇒ 这条先红；
    #  ② 每个非终态都必须有**到终态的合法路径** —— `triage_rejected` 曾经一条出口都没有
    #     ⇒ 判红指示 rc=1（"唯一出路是手改事件表"）✗；这条把它变成表级不变量；
    #  ③ 网格必须覆盖**所有**处置 —— 加一个处置而忘了补 pair ⇒ 这条先红（第五轮复核 ②）；
    #  ④ "已关账"的集合必须**恰好**是 docstring 的那四个（对 `DISPOSITIONS` 的不相交划分）
    #     —— 判据的谓词与 docstring 从此**同源**，谁想再"只认两个"必须先改这一条 ✓。
    _want_states = {st for _s, st in GRID_STATES}
    _nonterminal = set(SCHED_STATES) - TERMINAL_STATES          # ⚠ `SCHED_STATES` 是**元组**
    expect(f"网格覆盖**所有**非终态（`SCHED_STATES` 非终态 {sorted(_nonterminal)}"
           f" vs 网格 {sorted(_want_states)}）—— 加状态忘了补网格 ⇒ 这条先红 ✗",
           _want_states == _nonterminal)
    _no_exit = [st for st in sorted(_nonterminal) if not closure_steps(st)]
    expect(f"每个非终态都有**到终态的合法路径**（无出口的：{_no_exit or '无'}）"
           f"—— `triage_rejected` 曾经无出口 ⇒ 判红指示 rc=1 ✗", not _no_exit)
    _want_disps = {d for _p, _s, d in GRID_PAIRS} | {d for _p, d, _e in GRID_NOEVENT_OPEN}
    expect(f"网格覆盖**所有**处置（`DISPOSITIONS` {sorted(DISPOSITIONS)}"
           f" vs 网格 {sorted(_want_disps)}；已关账的走 `GRID_PAIRS`、开着的走 "
           f"`GRID_NOEVENT_OPEN` 正对照）—— 加处置忘了补网格 ⇒ 这条先红 ✗"
           f"（改前只覆盖 {{fixed, wo-filed}} ⇒ `boundary`/`control` 结构上不可见 ✗）",
           _want_disps == set(DISPOSITIONS))
    expect(f"「已关账」**恰好**是 docstring 的那四个处置（开放 {sorted(OPEN_DISPOSITIONS)}"
           f" · 已关账 {sorted(CLOSED_DISPOSITIONS)}）—— 两边必须是不相交划分，"
           f"否则判据的谓词会与 docstring 两处不一致 ✗",
           OPEN_DISPOSITIONS | CLOSED_DISPOSITIONS == set(DISPOSITIONS)
           and not (OPEN_DISPOSITIONS & CLOSED_DISPOSITIONS))
    #  ⑤ **冲突轴必须覆盖所有的"不等对"**（第五次复核）：加一个开放处置、或给缺口加载器加
    #     一个值域，而忘了补冲突轴 ⇒ 这条**先红** ✗（"结构上看不见"不许再靠人记着补）。
    _want_cx = {(v, a) for v in OPEN_DISPOSITIONS for a in GRID_GAP_DISPOSITIONS if v != a}
    expect(f"⑤ 冲突轴覆盖**所有**「视图处置 × 缺口处置」不等对"
           f"（应有 {len(_want_cx)} 对 {sorted(_want_cx)} vs 轴里 {len(set(GRID_CONFLICTS))} 对）"
           f"—— 相等对不是冲突（那是两处记账一致 ✓）⇒ 排除",
           set(GRID_CONFLICTS) == _want_cx)
    expect(f"⑤ 冲突轴的状态轴覆盖「无事件 + 两个终态」（{[s for s, _w in GRID_CONFLICT_STATES]}）"
           f"—— 冲突是**输入层**的事，调度器状态一格都不许改变结论（改前"
           f"「无事件」那两格还被 `--next` 推 ✗）",
           [w for _s, w in GRID_CONFLICT_STATES] == [None, "done", "wo_filed"])
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
            # 终态层的 `done` 要**真 sha**（`check_plan` 会核证据的存在性 + 祖先性 ✓）
            real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip()
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
                                   "why": f"夹具（有事件层 · 来源 {source}）：终态 `{disp}`"})
                # ⭐ **无事件层**（第五轮复核 ①）：**一事件都不写** ⇒ 状态由来源处置派生。
                # 非缺口来源再加一格带 `external-submission` 标记（那一支就是 42 格的病根 ✗）。
                for suffix, ext in (("noev", None), ("noevx", "external-submission")):
                    if suffix == "noevx" and source == "gap":
                        continue
                    ident = f"{prefix}-{suffix}"
                    if source == "gap":
                        gaps.append({"id": ident, "title": f"夹具 {ident}", "kind": "tooling",
                                     "severity": "painful", "status": disp, "found": "2026-01-01",
                                     "repro": f"docs/gaps/repro/{ident}.sh",
                                     "where": {"area": "夹具"}})
                        continue
                    if source == "索引+extras":
                        index.append(ident)
                    extras.append({"id": ident, "source": ext or "ux-index", "facet": "语言",
                                   "severity": "medium", "repro": f"bash {ident}.sh",
                                   "disposition": disp,
                                   "why": f"夹具（**无事件**层 · 来源 {source}"
                                          f"{' + external-submission' if ext else ''}）"})
                # ⭐ **终态层**（第四轴，第五轮复核 ④）：来源 × {done, wo_filed}
                # —— `done` 要证据（commit+reading）、`wo_filed` 要理由 ✓（夹具照真形状给齐，
                # 否则"没证据的收口"会多一条红 ⇒ 红条数就说不清了 ✗）。
                for suffix, _tst in GRID_TERMINAL:
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
                                   "why": f"夹具（终态层 · 来源 {source}）：`{disp}` × `{_tst}`"})
                # umbrella 格各要一个子任务（否则「umbrella 却没有子任务」会多一条红 ⇒
                # 红条数就说不清了 ✗）；子任务处置 = `fix`（**非终态**）⇒ 它自己不许红
                # ⇒ 它同时是"守卫没放宽"的**正对照** ✓。
                extras.append({"id": f"{prefix}-umb-c", "source": "ux-index", "facet": "语言",
                               "severity": "medium", "repro": "bash c.sh", "disposition": "fix",
                               "why": "夹具：umbrella 的子任务（非终态 ⇒ 不许红）"})
            # 无事件层的**正对照**（开着的处置 ⇒ 必须可推 + 在 open 里；防整层空转 ✗）
            for prefix, disp, ext in GRID_NOEVENT_OPEN:
                extras.append({"id": f"{prefix}-open", "source": ext or "ux-index",
                               "facet": "语言", "severity": "medium",
                               "repro": f"bash {prefix}.sh", "disposition": disp,
                               "why": f"夹具（无事件层正对照）：开着的处置 `{disp}`"
                                      f"{' + external-submission' if ext else ''}"})
            # 终态层的**正对照**：来源还开着（`fix`/`scheduled`）+ 调度器已终态 ⇒ 正向漂移必红 ✓
            for prefix, disp in GRID_TERMINAL_OPEN:
                for suffix, _tst in GRID_TERMINAL:
                    extras.append({"id": f"{prefix}-t{suffix}", "source": "ux-index",
                                   "facet": "语言", "severity": "medium",
                                   "repro": f"bash {prefix}.sh", "disposition": disp,
                                   "why": f"夹具（终态层正对照）：开着的处置 `{disp}` × `{_tst}`"})
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
                # ⭐ **终态层**的事件（`done` 带 commit+reading · `wo_filed` 带理由 ⇒ 证据判据
                # 全绿，红只可能来自"正向两层漂移" ✓ —— 否则红条数就说不清了 ✗）
                for suffix, _tst in GRID_TERMINAL:
                    append_event({"event": "advance", "id": f"{prefix}-{suffix}", "to": _tst,
                                  "commit": real_sha if _tst == "done" else None,
                                  "reading": "夹具：终态层" if _tst == "done" else None,
                                  "reason": "夹具：终态层（定案不做）" if _tst == "wo_filed" else None,
                                  "owner": "夹具"})
            for prefix, _disp in GRID_TERMINAL_OPEN:      # 正对照：开着的处置 × 终态 ⇒ 必红
                for suffix, _tst in GRID_TERMINAL:
                    append_event({"event": "advance", "id": f"{prefix}-t{suffix}", "to": _tst,
                                  "commit": real_sha if _tst == "done" else None,
                                  "reading": "夹具：终态层正对照" if _tst == "done" else None,
                                  "reason": "夹具：终态层正对照" if _tst == "wo_filed" else None,
                                  "owner": "夹具"})

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
            expect(f"网格夹具：**有事件层** {GRID_CELLS} 格调度器状态都造对了"
                   f"（造错的：{wrong_state or '无'}）", not wrong_state)
            fdisp = source_dispositions()
            wrong_disp = [f"{p}-{s}" for p, _a, d in grid_pairs for s, _w in grid_states
                          if fdisp.get(f"{p}-{s}") != d]
            expect(f"网格夹具：{len(grid_pairs)} 个「来源 × 处置」组合在**全量权威表**里"
                   f"都看得见（看不见/不对的：{wrong_disp or '无'}）", not wrong_disp)
            # ⭐ **无事件层**的 id 与形状（第五轮复核 ①：这一层测的就是"删掉覆盖层"那一臂）
            noev_closed = [f"{p}-noev" for p, _s, _d in grid_pairs] + \
                          [f"{p}-noevx" for p, s, _d in grid_pairs if s != "gap"]
            noev_open = [f"{p}-open" for p, _d, _e in GRID_NOEVENT_OPEN]
            _noev_ids = noev_closed + noev_open
            expect(f"网格夹具：**无事件层** {len(_noev_ids)} 格（{GRID_NOEVENT_CELLS} 已关账 + "
                   f"{len(noev_open)} 正对照）**一个事件都没有**（有事件的："
                   f"{[i for i in _noev_ids if i in state] or '无'}）—— 这一层是"
                   f"「删掉覆盖层」那一臂的守卫 ✓", not [i for i in _noev_ids if i in state])
            _noev_want = {f"{p}-noev": d for p, _s, d in grid_pairs}
            _noev_want.update({f"{p}-noevx": d for p, s, d in grid_pairs if s != "gap"})
            _noev_want.update({f"{p}-open": d for p, d, _e in GRID_NOEVENT_OPEN})
            _noev_bad_disp = [i for i, d in _noev_want.items() if fdisp.get(i) != d]
            expect(f"网格夹具：无事件层 {len(_noev_want)} 格的处置在全量权威表里都对"
                   f"（不对的：{_noev_bad_disp or '无'}）", not _noev_bad_disp)

            # ①② 逐格：判红点名（**只看这一格自己那一行**，别拿全局文本当证据 ✗）+ 收口指示
            for prefix, source, disp in grid_pairs:
                for suffix, want in grid_states:
                    ident = f"{prefix}-{suffix}"
                    st = state.get(ident, {}).get("state")
                    line = next((ln for ln in text.splitlines() if f"`{ident}`" in ln), "")
                    expect(f"网格 {ident}（{source}·`{disp}` × `{want}`）⇒ 判红并点名"
                           f"「反向两层漂移」（改前静默的是 `submitted` 那一列 —— 当时 6 格、现在 8 格 ✗）",
                           "反向两层漂移" in line)
                    steps = re.findall(rf"--advance {re.escape(ident)} --to (\w+)", line)
                    if closure_target(disp) is None:
                        # `boundary`/`control` 在 `TRANSITIONS` 里**没有入口** ⇒ 指示必须**如实**
                        # 说"没有能平账的迁移"，**不许**印一条照做只会换个红的死路指示 ✗
                        # （⑥ 把那条老死路真走一遍 ⇒ 仍然红 ⇒ 证明不印它是对的 ✓）。
                        expect(f"网格 {ident}：`{disp}` 无入口 ⇒ 如实说'没有能平账的迁移'、"
                               f"**不印** `--advance`（实测指示 `{'→'.join(steps) or '无'}`）",
                               not steps and "没有能平账的迁移" in line)
                        continue
                    here, walkable = st, bool(steps)
                    for nxt in steps:
                        if nxt not in TRANSITIONS.get(here, set()):
                            walkable = False
                        here = nxt
                    if walkable and here not in TERMINAL_STATES:
                        walkable = False
                    expect(f"网格 {ident}：收口指示逐步可执行、且落终态"
                           f"（实测 `{'→'.join(steps) or '无指示'}`）", walkable)

            # ⭐ ④⑤⑥ **无事件层**（第五轮复核 ①：一事件都不写 ⇒ 状态由来源处置派生）
            by_id = {r["id"]: r for r in view}
            wrong_eff = []
            for prefix, source, disp in grid_pairs:
                for suffix in ("noev", "noevx"):
                    ident = f"{prefix}-{suffix}"
                    if ident not in _noev_want:
                        continue
                    row = by_id.get(ident) or {"id": ident, "disposition": fdisp.get(ident),
                                               "sources": (["extras", "external-submission"]
                                                           if suffix == "noevx" else [])}
                    got = effective_state(row, state.get(ident, {}))
                    if got != DISPOSITION_STATES[disp]:
                        wrong_eff.append(f"{ident}:{got}≠{DISPOSITION_STATES[disp]}")
            expect(f"④ 无事件层 {GRID_NOEVENT_CELLS} 格的**权威状态 = 处置派生的状态**"
                   f"（`fixed`→done · `wo-filed`→wo_filed · `boundary`→boundary · "
                   f"`control`→control）—— **改前**带 `external-submission` 标记的已关账格"
                   f"会给 `submitted` ⇒ 这条必红 ✗（不对的：{wrong_eff or '无'}）", not wrong_eff)
            # ⚠ 可推性只在**视图里**问（`--next` 迭代的就是视图；`status=fixed` 的缺口按口径
            # 本来就不进队列视图 ✓ —— 两种口径下它都推不出来 ✓）
            _pushed = [i for i in noev_closed if i in by_id and is_pushable(by_id[i], state.get(i, {}))]
            expect(f"④ 已关账 + 无事件 ⇒ **不可推**（`--next` 不许推做完的；实测被推的："
                   f"{_pushed or '无'}）—— 改前真台账这里会推 **42 条** ✗", not _pushed)
            _leak = [i for i in noev_closed if i in open_set(view)]
            expect(f"④ 已关账 + 无事件 ⇒ **不在** `--json.open`/§0.2 里（三视图同数；"
                   f"实测漏进去的：{_leak or '无'}）", not _leak)
            _vis_open = [i for i in noev_open if i in by_id]
            expect(f"④ 正对照：开着的 {len(_vis_open)} 格**必须可推**且**在 open 里**"
                   f"（防\"整层恒绿\"的空转夹具 ✗）",
                   sorted(i for i in _vis_open if is_pushable(by_id[i], state.get(i, {})))
                   == sorted(_vis_open) and set(_vis_open) <= open_set(view))
            expect(f"⑤ **判定域含无事件的 id**（`plan_domain` = 视图 ∪ 事件表）—— 改前域是"
                   f"`sorted(state.items())` ⇒ 这一族**结构上不可见** ✗（可见的 "
                   f"{len([i for i in _noev_ids if i in by_id])} 格全都必须在域里）",
                   set(i for i in _noev_ids if i in by_id) <= set(plan_domain(view, state)))
            expect(f"⑤ 无事件层**一条都不红**（来源与调度器一致 ⇒ 域对齐后两边同判"
                   f"\"已关账\"；红条数 {rc} 应当恰好 = 有事件层 {GRID_CELLS} + 终态层 "
                   f"{GRID_TERMINAL_REDS}）",
                   not [i for i in _noev_ids if f"`{i}`" in text])
            expect("⑥ 子任务\"关了没有\"与 `--check` **同一口径**（`open_children` ⇒ 已关账的"
                   "无事件子任务不算\"在开\"；否则父条目永远收不了口 ✗）",
                   not open_children([f"{grid_pairs[0][0]}-noev"], view, state)
                   and open_children([f"{grid_pairs[0][0]}-umb-c"], view, state)
                   == [f"{grid_pairs[0][0]}-umb-c"])

            # ⭐ ⑧ **终态层**（第四轴，第五轮复核 ④ 的镜像方向）：来源记的是工作结论 ⇔ 一致
            _term_ids = [f"{p}-{s}" for p, _a, _d in grid_pairs for s, _t in GRID_TERMINAL]
            _term_open_ids = [f"{p}-t{s}" for p, _d in GRID_TERMINAL_OPEN
                              for s, _t in GRID_TERMINAL]
            _term_red, _term_green = [], []
            for prefix, source, disp in grid_pairs:
                for suffix, _tst in GRID_TERMINAL:
                    ident = f"{prefix}-{suffix}"
                    line = next((ln for ln in text.splitlines() if f"`{ident}`" in ln), "")
                    if line:
                        _term_red.append(ident)
                    else:
                        _term_green.append(ident)
                    expect(f"⑧ 终态层 {ident}（{source}·`{disp}` × `{_tst}`）⇒ "
                           f"{'判红（来源不是工作结论）' if disp not in WORK_DONE_DISPOSITIONS else '**不红**（两处记账一致）'}"
                           f"（实测 `{'红' if line else '绿'}`）",
                           (disp not in WORK_DONE_DISPOSITIONS) == bool(line))
                    if line:
                        expect(f"⑧ 终态层 {ident}：判红理由必须是**正向**两层漂移"
                               f"（不是反向那条 —— 那是另一个方向的说法）",
                               "两层漂移" in line and "反向两层漂移" not in line)
            expect(f"⑧ 终态层（{len(grid_pairs)} 个来源 × 终态）：红 {len(_term_red)} 格 = "
                   f"{GRID_TERMINAL_PAIR_REDS}、绿 {len(_term_green)} 格 = "
                   f"{GRID_TERMINAL_CELLS - GRID_TERMINAL_PAIR_REDS}"
                   f"（`fixed`/`wo-filed` 的来源**都**该绿 ✓；`boundary`/`control` 该红 ✗）",
                   len(_term_red) == GRID_TERMINAL_PAIR_REDS
                   and len(_term_green) == GRID_TERMINAL_CELLS - GRID_TERMINAL_PAIR_REDS)
            _term_open_lines = [next((ln for ln in text.splitlines() if f"`{i}`" in ln), "")
                                for i in _term_open_ids]
            expect(f"⑧ 终态层正对照：开着的处置 × 终态 {len(_term_open_ids)} 格**必须都红**"
                   f"（防『整层恒绿』的空转夹具 ✗ —— 改前 `wo_filed` 这一向**一条牙都没有**："
                   f"`--check` 静默而 `--json.open` 永远报『还没做完』）",
                   all("两层漂移" in ln and "反向两层漂移" not in ln for ln in _term_open_lines))

            expect(f"网格：红条数恰好 {GRID_REDS}（= 有事件层 {GRID_CELLS} + 终态层 "
                   f"{GRID_TERMINAL_REDS}；多一条就是夹具/规则溢出，实测 {rc}）",
                   rc == GRID_REDS)
            kids = [f"{p}-umb-c" for p, _a, _b in grid_pairs]
            expect(f"网格正对照：非终态来源的 {len(kids)} 个子任务一个都不许红"
                   f"（`fix` 不是终态 ⇒ 规则不该命中）",
                   not any(f"`{k}`" in text for k in kids))

            # ⑦ **`boundary`/`control` 的"没有能平账的迁移"不是偷懒**：把老指示（一路走到
            #    `done`）**真走一遍** ⇒ **换一个红，不是转绿**（来源是 `boundary`/`control`，
            #    不属于 `WORK_DONE_DISPOSITIONS` ⇒ 反向那条消失、正向那条立刻顶上）⇒
            #    不印那条指示是对的（印了只会换个红 ✗）。⚠ 只在夹具的临时事件表里走，
            #    走完**立刻还原**（跑完 `grep` = 0 ✓）。
            _dead = f"{grid_pairs[-1][0]}-todo"          # EC-todo = `control` × todo
            _ev_saved = EVENTS.read_text(encoding="utf-8")
            append_event({"event": "advance", "id": _dead, "to": "done", "commit": real_sha,
                          "reading": "夹具：把老指示真走一遍（死路验证）", "owner": "夹具"})
            _rows9, _u9 = build_queue()
            _err9 = io.StringIO()
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(_err9):
                _rc_dead = check_plan(sched_rows(_rows9, fold()), _rows9)
            EVENTS.write_text(_ev_saved, encoding="utf-8")
            _line9 = next((ln for ln in _err9.getvalue().splitlines()
                           if f"`{_dead}`" in ln), "")
            expect(f"⑦ `{_dead}` 按老指示走到 `done`（带证据）**仍然红**（实测 {_rc_dead} 条 = "
                   f"{GRID_REDS} ⇒ 反向那条消失、正向那条顶上 ⇒ **换一个红、不是转绿**）"
                   f"⇒ 判红信息如实说'没有能平账的迁移'是对的 ✓",
                   _rc_dead == GRID_REDS and "两层漂移" in _line9
                   and "反向两层漂移" not in _line9)

            # ⭐ ⑨ **第五轴 = 跨来源处置冲突**（2026-10-14 · 第五次独立复核实测 ✗）：
            #    上面每一格都是**单来源**（一个 id 只被一个来源登记）⇒「同一个 id 被两个来源
            #    登记、处置不一致」这一族**结构上看不见** ✗ —— 而它的后果最重：**只有终态缺口
            #    行**能造出它（`load_gaps()` 默认把 `status=="fixed"` 丢掉），于是
            #      ① 「一个 id 只能有一个处置」的闸门（长在 `build_queue()` 的合并里）对它**瞎**；
            #      ② `check_plan` 的权威表走**另一条**全量读法 ⇒ 同一个 `effective_state`
            #         喂两种行得两个状态（`--next` 推 `todo`、`--check` 判 `done`）✗。
            #    复核者的 A/B：同一个 done 事件 + 同一个 extras(`fix`) 行，**只加一行 gap
            #    `status=fixed`** ⇒ 改前 rc=1 → **rc=0 静默** ✗；而 gap `wo-filed` 的同样冲突
            #    改前就 rc=2 ✓（= 闸门有牙，只是被那个过滤器绕开）。
            #    ⇒ 逐格断（4 组「视图处置 × 来源处置」× 3 个调度器状态 = 12 格）：
            #      a) **单来源臂**（只有 extras 那一行，= A/B 的 A）**不许**冲突 ⇒ 判红可归属 ✓
            #        （防"整轴恒红"的空转夹具 ✗）；
            #      b) **双来源臂**必须冲突 ⇒ 真 CLI 路径 `--check-plan` **rc=2**、点名该 id +
            #        两个处置的**出处**（终态那一支与 `wo-filed` 对照**逐格同判** ⇒ 不许更松 ✗）；
            #      c) 状态（无事件 / done / wo_filed）**一格都不许改变结论** —— 冲突是输入层的
            #        事，调度器状态管不着 ⇒ "只在某一态静默"那一族**结构上**不可能再出现 ✓。
            _cx = Path(tmp) / "cx"
            _cx.mkdir()
            _cx_saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING)
            _argv_saved = list(sys.argv)
            try:
                for _cvi, (_vdisp, _adisp) in enumerate(GRID_CONFLICTS):
                    for _csuf, _cstate in GRID_CONFLICT_STATES:
                        ident = f"CX{_cvi}-{_csuf}"
                        EVENTS = _cx / f"events-{ident}.jsonl"
                        EXTRAS = _cx / "extras.jsonl"
                        INDEX = _cx / "index.md"
                        GAPS_LEDGER = _cx / "ledger.jsonl"
                        ONBOARDING = _cx / "ONBOARDING.md"
                        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                                         "|---|---|---|---|---|---|---|\n"
                                         f"| {ident} | 语言 | 夹具：跨来源冲突 | 高 | `bash c.sh` "
                                         "| front |  |\n", encoding="utf-8")
                        ONBOARDING.write_text("### 0.2 夹具\n", encoding="utf-8")
                        _gaprow = {"id": ident, "title": f"夹具 {ident}", "kind": "tooling",
                                   "severity": "painful", "status": _adisp, "found": "2026-01-01",
                                   "repro": f"docs/gaps/repro/{ident}.sh",
                                   "where": {"area": "夹具"}}
                        _extrarow = {"id": ident, "source": "ux-index", "facet": "语言",
                                     "severity": "medium", "repro": f"bash {ident}.sh",
                                     "disposition": _vdisp, "why": "夹具：视图（extras）那一行"}
                        EXTRAS.write_text(json.dumps(_extrarow, ensure_ascii=False) + "\n",
                                          encoding="utf-8")
                        # 事件（`done` 要真 sha + 读数 ⇒ 证据判据不会多出红来 ✗）
                        if _cstate == "done":
                            EVENTS.write_text(json.dumps(
                                {"event": "advance", "id": ident, "to": "done",
                                 "commit": real_sha, "reading": "夹具：done", "owner": "夹具"},
                                ensure_ascii=False) + "\n", encoding="utf-8")
                        elif _cstate == "wo_filed":
                            EVENTS.write_text(json.dumps(
                                {"event": "advance", "id": ident, "to": "wo_filed",
                                 "reason": "夹具：定案不做", "owner": "夹具"},
                                ensure_ascii=False) + "\n", encoding="utf-8")
                        else:
                            EVENTS.write_text("", encoding="utf-8")

                        def _cli(argv: list[str]) -> tuple[int, str, str]:
                            """**真 CLI 路径**（`main()` 读 `sys.argv`）—— 判据打在 `--check-plan`
                            这一条上（`soko gate`/CI 跑的 `--check` 也走它）⇒ 咬的是**用户敲的
                            那一条命令**，不是内部函数 ✓。"""
                            sys.argv = ["ux-queue.py", *argv]
                            _b, _e = io.StringIO(), io.StringIO()
                            with contextlib.redirect_stdout(_b), contextlib.redirect_stderr(_e):
                                _rc = main()
                            return _rc, _b.getvalue(), _e.getvalue()

                        # a) **单来源臂**（A/B 的 A）：只有视图那一行 ⇒ 不许有冲突
                        GAPS_LEDGER.write_text("", encoding="utf-8")
                        _rc_a, _o_a, _e_a = _cli(["--check-plan"])
                        expect(f"⑨ {ident} 单来源臂（只有 extras `{_vdisp}`、状态 `{_csuf}`）"
                               f"**不许**判成处置冲突（实测 rc={_rc_a}；`error:` 说明撞上了闸门 ⇒ "
                               f"整轴就是恒红的空转夹具 ✗）",
                               "处置不一致" not in _e_a)
                        expect(f"⑨ {ident} 单来源臂的读数（rc={_rc_a}）= 漂移规则的正常判据"
                               f"（无事件/两处一致 ⇒ 0；`done`/`wo_filed` + 来源开着 ⇒ 1）",
                               _rc_a == (0 if _cstate is None else 1))
                        # b) **双来源臂**（A/B 的 B）：加一行**终态缺口行** ⇒ 必须硬判红
                        GAPS_LEDGER.write_text(json.dumps(_gaprow, ensure_ascii=False) + "\n",
                                               encoding="utf-8")
                        _rc_b, _o_b, _e_b = _cli(["--check-plan"])
                        expect(f"⑨ {ident} 双来源臂（extras `{_vdisp}` + gap `{_adisp}`、"
                               f"状态 `{_csuf}`）⇒ **rc=2 当场判红**（实测 rc={_rc_b}）—— "
                               f"改前 `{_adisp}`=fixed 这一支**全静默** ✗（"
                               f"`wo-filed` 那一支改前也 rc=2 ⇒ 两行**必须同判** ＝ 不许更松 ✗）",
                               _rc_b == 2)
                        expect(f"⑨ {ident} 的判红信息**点名**该 id + 两个处置 + 各自出处"
                               f"（不能只报两个词：人还得自己回台账里翻 ✗）",
                               ident in _e_b and f"`{_vdisp}`" in _e_b and f"`{_adisp}`" in _e_b
                               and "extras" in _e_b and "缺口台账" in _e_b)
                        expect(f"⑨ {ident} 冲突 ⇒ `--next`/`--json` **也**走同一条闸门"
                               f"（同一个输入 ⇒ 不可能『一个命令静默、另一个推得出来』✗）",
                               _cli(["--next"])[0] == 2 and _cli(["--json"])[0] == 2)
                        # d) **轴的前提是真的**：夹具那一行 `status=<adisp>` 必须被**真加载器**
                        #    翻成同一个处置 —— 否则上面 b) 咬的是"我以为的翻译"（空转 ✗）。
                        _loaded = load_gaps(include_terminal=True)
                        expect(f"⑨ {ident}：缺口加载器把 `status={_adisp}` 翻成 `{_adisp}`"
                               f"（实测 {_loaded[0].get('disposition') if _loaded else '（没读到）'}）"
                               f"—— 轴的前提由**真加载器**核过，不是硬编码假设 ✓",
                               len(_loaded) == 1 and _loaded[0].get("disposition") == _adisp)
            finally:
                (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING) = _cx_saved
                sys.argv[:] = _argv_saved
        finally:
            (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, ONBOARDING) = saved[:5]
            globals()["lock_file"] = saved[5]


def selftest_ghost_child(expect) -> None:
    """**无源子任务必须进幽灵判据**（2026-10-14 · 第六次独立复核实测 ✗）—— #1 的防复发轴。

    复核者纯 CLI 复现：`--schedule ADOPT-1` + `--add-child ADOPT-1 --child GHOST-KID-1`
    ⇒ 改前 `--check` **rc=0 · stderr 0 行**、从不提那个 id，而 `--next` 推得出它、
    `--json.open` 含它 ✗。病根 = `sched_rows` 给"没有源行的子任务"**合成一行**，
    而 `check_plan` 的 `known` 读的是**视图** ⇒ **合成行给自己作了证** ✗。

    断言（① 就是"把 `known` 改回读视图"的定向突变靶子：那一改会让 ① 变绿 ⇒ 必红 ✗）：
      ① 无源子任务 ⇒ `check_plan(view, rows)` **判红 + 点名 + 说清「无源子任务」**；
      ② 合成行的 `sources` 是**诚实的空表**（不是缺键 ⇒ `--json.view` 与源行同形 schema ✓）；
      ③ 正对照：把子任务**登记进来源** ⇒ 转绿（守卫不许咬到合法的子任务 ✗）；
      ④ **不许更松**：**没有父**的幽灵 id（三处都没有）**照样**判红（老那条路还在 ✓）；
      ⑤ 域没变（`plan_domain` = 视图 ∪ 事件表）且条目**没被藏起来**
         （仍在 `--json.open` / `--next` 里 —— 会计账口径：判红 ≠ 从面板上消失 ✓）。
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

        def ext(ident: str, disp: str) -> dict:
            return {"id": ident, "source": "ux-index", "facet": "语言", "severity": "medium",
                    "repro": f"bash {ident}.sh", "disposition": disp, "why": "夹具"}

        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| P-1 | 语言 | 夹具（可拆解的父） | 高 | `bash p.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text(json.dumps(ext("P-1", "fix"), ensure_ascii=False) + "\n",
                          encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")

        def snapshot():
            _r, _u = build_queue()
            _s = fold()
            return _r, _s, sched_rows(_r, _s)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, err.getvalue()

        try:
            rows, state, view = snapshot()
            rc, err = capture(cmd_add_child, "P-1", "K-1", "夹具子任务", "t", view)
            expect(f"夹具：拆解成功（rc={rc}）", rc == 0)
            rows, state, view = snapshot()
            kid = next((r for r in view if r["id"] == "K-1"), None)
            src_row = next((r for r in rows if r["id"] == "P-1"), None)
            expect("② 合成行**在**视图里（不丢行 ✓）", kid is not None)
            expect("② 合成行的 `sources` = 诚实的空表（改前连键都没有 ⇒ 异形 schema ✗）",
                   kid is not None and kid.get("sources") == [])
            expect("② 源行也有 `sources` 键（两处同形）",
                   src_row is not None and "sources" in src_row)
            rc, err = capture(check_plan, view, rows)
            expect(f"① 无源子任务 ⇒ **判红**（改前 rc=0 静默 ✗；实测 {rc}）", rc >= 1)
            expect("① 判红信息**点名** `K-1` + 说清「无源子任务」",
                   "K-1" in err and "无源子任务" in err)
            expect("⑤ 域没变：它仍在 `plan_domain`（视图 ∪ 事件表）里",
                   "K-1" in plan_domain(view, state))
            expect("⑤ 条目**没被藏起来**：仍在 `--json.open` / `--next` 里",
                   "K-1" in open_set(view) and "K-1" in pushable_ids(view, state))
            with EXTRAS.open("a", encoding="utf-8") as fh:      # ③ 正对照：登记来源
                fh.write(json.dumps(ext("K-1", "fix"), ensure_ascii=False) + "\n")
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"③ 登记来源之后 ⇒ **转绿**（不许误咬合法子任务；实测红 {rc} 条）", rc == 0)
            append_event({"event": "advance", "id": "GHOST-X", "to": "blocked",
                          "reason": "夹具：纯幽灵（三处都没有）", "owner": "t"})
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect("④ **没有父**的幽灵 id 照样判红（老那条路还在 ⇒ 没放宽 ✓）",
                   rc >= 1 and "GHOST-X" in err and "幽灵条目" in err)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]


def selftest_triage_pointer(expect) -> None:
    """**`--triage` 的假证据必须被硬拒**（2026-10-14 · 第六次独立复核实测 ✗）—— #2 的防复发轴。

    复核者纯 CLI 复现：`--submit fix --id EXT-1 …` ⇒ `--triage EXT-1 --to applied
    --commit deadbeef…`（对象**不存在**，`git cat-file -e` rc=128）⇒ 改前 **rc=0** ⇒
    事件表里留下 `done + 假指针`，`--check` rc=0、stderr 只有「⚠ 存量收口指针 1 条 …」✗。
    病根两条：① `cmd_triage` **一条硬拒都没有**（`cmd_advance` 两条都有）；② `fold` 的
    triage 分支**不写 `advanced_ts`** ⇒ 读侧把它当**存量 rebase 孪生**豁免（描述与事实相反 ✗）。

    断言（①/② 是"把 `closure_pointer_problem` 从 `cmd_triage` 摘掉"的靶子；⑤ 是"把
    `advanced_ts` 从 triage 分支摘掉"的靶子 ⇒ 两个定向突变各自咬住 ✓）：
      ① **存在性**硬拒（对象不存在 ⇒ rc=1）；② **祖先性**硬拒（对象存在、非祖先 ⇒ rc=1）；
      ③ **不许半途而废**：被拒时来源处置与事件表**一个字节都不动**（闸门在写之前 ✓）；
      ④ 正对照：真祖先 ⇒ rc=0 且来源处置翻成 `fixed`（不许误拒 ✓）；
      ⑤ 读侧复查：手写「起点之后 + 非祖先」的 triage-applied ⇒ `check_plan` **判红**；
      ⑥ **措辞分桶**：有 `ts` 且早于起点 ⇒ 从宽且**叫「存量」** ✓；**没有 `ts`** ⇒ 从宽但
         **不许叫「存量」** ✗（改前两桶混成一个 ⇒ ⑥b 必红 ✗）。
    """
    import contextlib
    import datetime as _dt
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    bad_sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"

        def ext(ident: str, disp: str) -> dict:
            return {"id": ident, "source": "external-submission", "facet": "语言",
                    "severity": "medium", "repro": f"bash {ident}.sh", "disposition": disp,
                    "why": "夹具：外部提交"}

        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| E-1 | 语言 | 夹具（外部提交） | 高 | `bash e.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text(json.dumps(ext("E-1", "fix"), ensure_ascii=False) + "\n",
                          encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")
        real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                  capture_output=True, text=True).stdout.strip()
        dangling = subprocess.run(
            ["git", "-c", "user.name=ux-queue-selftest", "-c", "user.email=selftest@local",
             "commit-tree", f"{real_sha}^{{tree}}", "-m", "ux-queue selftest: triage 悬垂"],
            cwd=ROOT, capture_output=True, text=True).stdout.strip()

        def disp_of(ident: str) -> str:
            return next((r["disposition"] for r in _read_jsonl(EXTRAS) if r["id"] == ident), "?")

        def triage_events() -> list[dict]:
            return [e for e in load_events() if e.get("event") == "triage"]

        def snapshot():
            _r, _u = build_queue()
            _s = fold()
            return _r, _s, sched_rows(_r, _s)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, err.getvalue()

        try:
            expect(f"夹具：非祖先 sha 造得出且对象存在（{dangling[:8]}…）",
                   bool(dangling) and not commit_is_ancestor(dangling))
            rc, err = capture(cmd_triage, "E-1", "applied", "", bad_sha, "t")
            expect(f"① 对象**不存在** ⇒ 硬拒（改前 rc=0 ✗；实测 {rc}）",
                   rc == 1 and "不存在" in err)
            expect("③ 被拒时来源处置**没动**（仍 `fix`）", disp_of("E-1") == "fix")
            expect("③ 被拒时**没有** triage 事件落表", not triage_events())
            rc, err = capture(cmd_triage, "E-1", "applied", "", dangling, "t")
            expect(f"② 对象存在但**非祖先** ⇒ 硬拒（改前 rc=0 ✗；实测 {rc}）",
                   rc == 1 and "不是 `HEAD` 的祖先" in err)
            expect("③ 同样一个字节都没写（无半途而废的写入 ✓）",
                   disp_of("E-1") == "fix" and not triage_events())
            rc, err = capture(cmd_triage, "E-1", "applied", "", real_sha, "t")
            expect(f"④ 真祖先 ⇒ 受理通过（**不许误拒**；实测 {rc}）", rc == 0)
            expect("④ 受理把来源处置翻成 `fixed`（受理 = 关账 ✓）", disp_of("E-1") == "fixed")
            _cut = _dt.datetime.fromisoformat(ANCESTRY_GATE_SINCE)
            # ⑤ 起点**之后** + 非祖先 ⇒ 读侧判红（改前 `advanced_ts` 缺席 ⇒ 走豁免桶 ⇒ rc=0 ✗）
            EVENTS.write_text("", encoding="utf-8")
            append_event({"event": "triage", "id": "E-1", "to": "applied", "commit": dangling,
                          "ts": (_cut + _dt.timedelta(minutes=1)).isoformat(), "owner": "t"})
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"⑤ 起点之后的 triage-applied 假指针 ⇒ **判红**（改前 rc=0 ✗；实测红 {rc}）",
                   rc >= 1)
            expect("⑤ 判红点名 `E-1` + 说「不是 `HEAD` 的祖先」",
                   "E-1" in err and "不是 `HEAD` 的祖先" in err)
            # ⑥a 起点**之前**（真存量）⇒ ⭐ **也判红**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）
            #     ⚠ 这一臂**改前是「从宽 + 叫存量」** —— 豁免已按编排者口径删掉
            #     （存量全量回填到 0 ⇒ 口径 = "非祖先必须 0 条"的断言）⇒ 期望**反过来**。
            EVENTS.write_text("", encoding="utf-8")
            append_event({"event": "triage", "id": "E-1", "to": "applied", "commit": dangling,
                          "ts": (_cut - _dt.timedelta(minutes=1)).isoformat(), "owner": "t"})
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"⑥a 起点之前的 triage-applied 指针 ⇒ **也判红**（豁免已删；改前 rc=0 ✗；"
                   f"实测红 {rc} 条）", rc >= 1)
            expect("⑥a 判红点名 `E-1`（**不许**再叫「存量收口指针」✗）",
                   "E-1" in err and "存量收口指针" not in err)
            # ⑥b **没有 ts**（旧格式/手写）⇒ 同样判红（`ts` 现在**不影响**判绿/判红 ✓）
            #     ⚠ `append_event` 会自动补 `ts` ⇒ 这一臂必须**直接写事件表**。
            EVENTS.write_text(json.dumps(
                {"event": "triage", "id": "E-1", "to": "applied", "commit": dangling,
                 "owner": "t"}, ensure_ascii=False) + "\n", encoding="utf-8")
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"⑥b **无时间戳**的指针 ⇒ 也判红（`ts` 不再分桶；改前 rc=0 ✗；实测红 {rc} 条）",
                   rc >= 1)
            expect("⑥b 判红点名 `E-1` 且**不再有「从宽/存量收口指针」那套分桶**"
                   "（那个概念已随豁免一起删掉 ✓）",
                   "E-1" in err and "存量收口指针" not in err and "从宽" not in err)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]


def selftest_repoint_and_release_gate(expect) -> None:
    """**证据闸门 + 指针回填**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）—— 本轮缺口本身的防复发轴。

    缺口（第六次独立复核实测 ✓）：`--release add/check` **只要求 `done` + 有 `commit`** ⇒
    **手写的假指针能进发布节点** ✗（实测 `release_add_rc=0`，即使那个 commit 对象**不存在**）。
    病根与修法：入版闸门**必须与收口闸门同口径** ⇒ 两处都走 `closure_pointer_problem()`
    （存在性 `git cat-file -e` + 祖先性 `git merge-base --is-ancestor`）✓。
    配套：存量 60 条 rebase 孪生用 `--repoint` 回填到 0（`selftest_ancestry_gate` ③ 钉住
    「豁免已删、非祖先必须 0 条」）。

    断言（每条都点了名靶子 ⇒ 定向突变必红）：
      ① 夹具：**真孪生**造得出（同树 + 同父 + 同 subject ⇒ 同 `patch-id --stable`，
         但**不是** `HEAD` 的祖先）—— 这是「摘掉三关里的任何一关」的靶子；
      ② `--repoint-all --dry-run` ⇒ rc=0 且**一个字节都不写**；
      ③ `--repoint` 自动找孪生 ⇒ rc=0 · 指针换成真祖先 · **state 仍是 done** ·
         **旧事件原样留着** + `repointed_from` 记旧 sha（只追加账、可逐条回滚 ✓）；
      ④ 回填后 `check_plan` 转绿（**这一对是因果对照**：③ 之前必红、③ 之后必绿）；
      ⑤ 显式给**非祖先** / **不存在**的指针 ⇒ 硬拒（走 `closure_pointer_problem` 的两半）；
      ⑥ **找不到孪生 ⇒ 不硬填**（rc=1 且**零写入**）；**已经是祖先 ⇒ 拒**（没有可回填的）；
         **缺 `--reason` / id 不存在 ⇒ 拒**；
      ⑦ ⭐ **`--release add` 假指针 ⇒ rc≠0**（**就是本轮的缺口**）+ **不写 `release-add` 事件**；
         正对照：真祖先 + 真对象 ⇒ **仍 rc=0**（**不许误拒** ✗）；
      ⑧ `--release check`（读侧，手写 `release-add` 把假指针条目塞进节点）**也咬得住**；
      ⑨ **真 CLI 路径**（子进程 + `SOKO_QUEUE_EVENTS` 重定向）跑 ⑦ 的两半
         —— ⚠ 用 `Path(__file__)`，**不是** `ROOT/scripts/ux-queue.py`（那是主仓那份，
         在 land 之前是旧的 ⇒ 测到的是别的构建 ✗）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    bad_sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True).stdout.strip()
    _TWIN_CACHE.clear()
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"],
                 globals()["worktree_dirty"], globals()["probe_inventory"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["worktree_dirty"] = lambda: ""
        globals()["probe_inventory"] = lambda: []
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| R-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash r.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")

        def git(*argv) -> str:
            return subprocess.run(["git", *argv], cwd=ROOT, capture_output=True,
                                  text=True).stdout.strip()

        def make_twin(sha: str) -> str:
            """同树 + 同父 + 同 subject ⇒ 同 `patch-id --stable`，但**不是**祖先（新对象、无 ref）。"""
            return git("-c", "user.name=ux-queue-selftest", "-c", "user.email=selftest@local",
                       "commit-tree", f"{sha}^{{tree}}", "-p", f"{sha}^",
                       "-m", git("log", "-1", "--format=%s", sha))

        def dangling_new(subject: str) -> str:
            """**没有孪生**的悬垂 commit（subject 独一份）。"""
            return git("-c", "user.name=ux-queue-selftest", "-c", "user.email=selftest@local",
                       "commit-tree", f"{real_sha}^{{tree}}", "-m", subject)

        # ⚠ `base_sha` 是**最新**的那个唯一 subject 提交（`git log` 倒序）⇒ 它**可能等于**
        # `real_sha`(HEAD) ⇒ ⑤b 的"无关祖先"**不能**用 `real_sha`（那正是孪生的基 ✗，实测
        # 夹具自己踩到：subject/patch-id 全一致 ⇒ 反而 rc=0）。用**根提交**：它有父的
        # `base_sha` 的 subject 在全历史唯一 ⇒ 根的 subject 必然不同 ✓，且它一定是祖先 ✓。
        unrelated_anc = git("rev-list", "--max-parents=0", "HEAD").splitlines()[-1] \
            if git("rev-list", "--max-parents=0", "HEAD") else ""

        # 找一个"subject 在全历史里唯一 + 有父"的真祖先当孪生的**基**
        by_subj = _ancestors_by_subject()
        base_sha = ""
        for line in git("log", "--no-merges", "--format=%H%x01%s", "HEAD").splitlines():
            if "\x01" not in line:
                continue
            _h, _s = line.split("\x01", 1)
            if len(by_subj.get(_s, [])) == 1 and git("rev-parse", "--verify", "--quiet",
                                                     f"{_h}^"):
                base_sha = _h
                break
        twin = make_twin(base_sha) if base_sha else ""
        lonely = dangling_new("ux-queue selftest: 这条 subject 独一份（没有孪生）")

        def ext(ident: str) -> dict:
            return {"id": ident, "source": "ux-index", "facet": "语言", "severity": "medium",
                    "repro": f"bash {ident}.sh", "disposition": "fixed", "why": "夹具：已关账"}

        def seed(*pairs: tuple[str, str]) -> None:
            """事件表 = 每条一个 `advance … done`（`ts` 由 `append_event` 补）。"""
            EVENTS.write_text("", encoding="utf-8")
            for ident, sha in pairs:
                append_event({"event": "advance", "id": ident, "to": "done", "commit": sha,
                              "reading": "夹具读数", "owner": "t"})

        def snapshot():
            _r, _u = build_queue()
            _s = fold()
            return _r, _s, sched_rows(_r, _s)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, err.getvalue()

        def capture_both(fn, *a, **kw):
            """`(rc, stdout+stderr)` —— 报告类断言走 stdout、判红走 stderr，两边都要看时用它。"""
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, buf.getvalue() + err.getvalue()

        def ns(**kw):
            base = dict(release="", rel="", version="", id="", commit="", ci="", reason="",
                        owner="t", force=False)
            base.update(kw)
            return argparse.Namespace(**base)

        try:
            expect(f"夹具①：真孪生造得出（{base_sha[:8]} → {twin[:8]}）",
                   bool(twin) and not commit_is_ancestor(twin))
            _t, _note = find_twin(twin)
            expect(f"夹具①：三关认得它（孪生 = {(_t or 'None')[:8]}，期望 {base_sha[:8]}）",
                   _t == base_sha)
            expect("夹具①：**只比 subject 不够**（孪生与基 subject 相同但 sha 不同 ⇒ "
                   "真跑得通三关才算数）", twin != base_sha)
            extra_rows = [ext("R-1"), ext("R-2")]
            EXTRAS.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in extra_rows),
                              encoding="utf-8")
            seed(("R-1", twin), ("R-2", twin))
            rows, _st, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"夹具①：两条孪生指针 ⇒ check_plan **判红 2 条**（实测 {rc}）", rc == 2)
            expect("夹具①：判红点名 R-1 与 R-2", "R-1" in err and "R-2" in err)

            # ② dry-run ⇒ 一个字节都不写
            _before = EVENTS.read_text(encoding="utf-8")
            rc, out = capture_both(cmd_repoint_all, "夹具 dry-run", "t", True, 0)
            expect(f"② `--repoint-all --dry-run` rc={rc}", rc == 0)
            expect("② dry-run **零写入**（事件表逐字节不变 ✓）",
                   EVENTS.read_text(encoding="utf-8") == _before)
            expect("② dry-run 报告里点名两条 id", "R-1" in out and "R-2" in out)

            # ③ 单条自动找孪生
            rc, out = capture(cmd_repoint, "R-1", "", "夹具：回填孪生", "t", False)
            expect(f"③ `--repoint` 自动找孪生 rc={rc}", rc == 0)
            _s = fold()
            expect(f"③ 指针换成真祖先（{_s['R-1'].get('commit', '')[:8]}）",
                   _s["R-1"].get("commit") == base_sha)
            expect("③ **state 仍是 done**（回填改的是指针，不是「这条做完了吗」✗）",
                   _s["R-1"].get("state") == "done")
            expect("③ `reading` 一个字没换", _s["R-1"].get("reading") == "夹具读数")
            expect("③ **旧事件原样留着**（只追加账 ⇒ 旧 sha 还能审出来 ✓）",
                   any(e.get("event") == "advance" and e.get("commit") == twin
                       for e in load_events()))
            expect("③ `repointed_from` 记了旧 sha", twin in (_s["R-1"].get("repointed_from") or []))
            rows, _st, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"④ 回填后**少一条红**（还剩 R-2 ⇒ 1 条；实测 {rc}）", rc == 1)
            expect("④ 剩下的红**只**点名 R-2（R-1 已清 ✓）", "R-2" in err and "R-1" not in err)

            # ⑤ 两条硬拒（共享谓词的两半）
            rc, err = capture(cmd_repoint, "R-2", lonely, "夹具：显式非祖先", "t", False)
            expect(f"⑤ 显式给**非祖先**指针 ⇒ 硬拒 rc={rc}", rc == 1
                   and "不是 `HEAD` 的祖先" in err)
            rc, err = capture(cmd_repoint, "R-2", bad_sha, "夹具：显式不存在", "t", False)
            expect(f"⑤ 显式给**不存在**的指针 ⇒ 硬拒 rc={rc}", rc == 1 and "不存在" in err)

            # ⑤b ⭐ **显式指针也要过内容闸门 + 事件留审计字段**（`#1`，2026-10-14 · 第七次
            # 独立复核实测 ✗）：靶子 = `--repoint <id> --commit <一个**是祖先**但内容无关的 sha>`
            # —— 旧写法只查存在性 + 祖先性 ⇒ **rc=0**，且下一条 `--check` 打出「非祖先 0 条 ✓」
            # ⇒ 证据链被"指向任意祖先"掏空 ✗。这里把两条腿都钉住：
            #   · 拒绝（rc=1 + 点名 subject 不一致）+ **零写入**；
            #   · 正对照：显式给**真孪生** ⇒ 仍 rc=0（**不许误拒** ✗）；
            #   · 审计字段：事件里必须有新旧两侧的 subject + patch_id（**事后可审计** ✓）；
            #   · 读侧复查：手改 `repoint` 的 `commit`（审计字段不动）⇒ `check_plan` 判红 ✓。
            _before = EVENTS.read_text(encoding="utf-8")
            rc, err = capture(cmd_repoint, "R-2", unrelated_anc, "夹具：显式无关祖先", "t", False)
            expect(f"⑤b 显式给**是祖先但内容无关**的指针 ⇒ 拒 rc={rc}", rc == 1
                   and "subject 不一致" in err)
            expect("⑤b 且**零写入**（假指针一个字都没留下 ✓）",
                   EVENTS.read_text(encoding="utf-8") == _before)
            rc, _out = capture(cmd_repoint, "R-2", base_sha, "夹具：显式真孪生", "t", False)
            expect(f"⑤b 正对照：显式给**真孪生** ⇒ 仍 rc={rc}（不许误拒 ✗）", rc == 0)
            _reps = [e for e in load_events() if e.get("event") == "repoint"
                     and e.get("id") == "R-2"]
            _rep = _reps[-1] if _reps else {}
            expect("⑤b 事件记了**审计字段**（新旧两侧 subject + patch_id）",
                   bool(_rep.get("subject") and _rep.get("patch_id")
                        and _rep.get("old_subject") and _rep.get("old_patch_id")))
            expect("⑤b 审计字段与事实一致（patch-id 逐字）",
                   _rep.get("patch_id") == _patch_id(base_sha)
                   and _rep.get("old_patch_id") == _patch_id(twin))
            # 读侧复查：手改事件表把 `repoint` 的 `commit` 换成别的（审计字段原样不动）
            _ev = [json.loads(l) for l in EVENTS.read_text(encoding="utf-8").splitlines()
                   if l.strip()]
            for _e in _ev:
                if _e.get("event") == "repoint" and _e.get("id") == "R-2":
                    _e["commit"] = unrelated_anc
            EVENTS.write_text("".join(json.dumps(e, ensure_ascii=False) + "\n" for e in _ev),
                              encoding="utf-8")
            rows, _st, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"⑤b 手改 `repoint` 的 commit（审计字段不动）⇒ 判红（实测 {rc}）",
                   rc == 1 and "对不上" in err)

            # ⑥ 不硬填 / 已祖先 / 缺理由 / id 不存在 —— 四条都必须拒，且零写入
            seed(("R-1", twin), ("R-2", lonely))
            _before = EVENTS.read_text(encoding="utf-8")
            rc, err = capture(cmd_repoint, "R-2", "", "夹具：找不到孪生", "t", False)
            expect(f"⑥ **找不到孪生 ⇒ 不硬填** rc={rc}", rc == 1 and "找不到孪生" in err)
            expect("⑥ 且**零写入**（事件表逐字节不变 ✓）",
                   EVENTS.read_text(encoding="utf-8") == _before)
            seed(("R-1", real_sha))
            rc, err = capture(cmd_repoint, "R-1", "", "夹具：已经是祖先", "t", False)
            expect(f"⑥ **已经是祖先 ⇒ 拒**（没有可回填的）rc={rc}",
                   rc == 1 and "已经是 `HEAD` 的祖先" in err)
            rc, err = capture(cmd_repoint, "R-1", "", "", "t", False)
            expect(f"⑥ 缺 `--reason` ⇒ 拒 rc={rc}", rc == 1 and "--reason" in err)
            try:
                cmd_repoint("NO-SUCH-ID", "", "x", "t", False)
                expect("⑥ id 不存在 ⇒ UsageError", False)
            except UsageError:
                expect("⑥ id 不存在 ⇒ UsageError", True)

            # ⑦ ⭐ 证据闸门：假指针不许入版；真祖先**不许误拒**
            #    （正对照那条的指针**必须是真祖先** —— 拿孪生当正对照会得到一条假红 ✗）
            seed(("R-1", real_sha), ("R-2", twin))
            EVENTS.write_text(EVENTS.read_text(encoding="utf-8")
                              + json.dumps({"event": "release-open", "id": "REL-0.0.1",
                                            "version": "0.0.1", "owner": "t"},
                                           ensure_ascii=False) + "\n", encoding="utf-8")
            _before = EVENTS.read_text(encoding="utf-8")
            rc, err = capture(cmd_release, ns(release="add", rel="REL-0.0.1", id="R-2"), [])
            expect(f"⑦ ⭐ `--release add` **假指针（非祖先）⇒ rc≠0**（实测 {rc}；改前 rc=0 ✗）",
                   rc != 0)
            expect("⑦ 判红走的是**收口那一套措辞**（同谓词 ✓）",
                   "不是 `HEAD` 的祖先" in err and "--repoint" in err)
            expect("⑦ 被拒时**没写 `release-add` 事件**（闸门在写之前 ✓）",
                   EVENTS.read_text(encoding="utf-8") == _before)
            rc, err = capture(cmd_release, ns(release="add", rel="REL-0.0.1", id="R-1"), [])
            expect(f"⑦ 正对照：**真祖先 + 真对象** ⇒ 仍 **rc=0**（不许误拒 ✗；实测 {rc}）", rc == 0)
            expect("⑦ 正对照真的写了 `release-add` 事件",
                   any(e.get("event") == "release-add" and e.get("id") == "R-1"
                       for e in load_events()))
            # ⑦b 对象**根本不存在**的指针（手改事件表那一族的第二半）
            EVENTS.write_text("", encoding="utf-8")
            append_event({"event": "release-open", "id": "REL-0.0.2", "version": "0.0.2",
                          "owner": "t"})
            append_event({"event": "release-add", "rel": "REL-0.0.2", "id": "R-3", "owner": "t"})
            append_event({"event": "advance", "id": "R-3", "to": "done", "commit": bad_sha,
                          "reading": "假证据", "owner": "t"})
            rc, err = capture(cmd_release, ns(release="add", rel="REL-0.0.2", id="R-3"), [])
            expect(f"⑦b `--release add` 对象**不存在** ⇒ rc≠0（实测 {rc}；改前 rc=0 ✗）",
                   rc != 0 and "不存在" in err)
            # ⑧ 读侧：手写 `release-add` 把假指针条目**塞进**节点 ⇒ `check` 也咬
            rc, err = capture(cmd_release, ns(release="check", rel="REL-0.0.2"), [])
            expect(f"⑧ `--release check` **也咬得住**（手写事件塞进来的假指针）rc={rc}",
                   rc != 0 and "入版条目 `R-3` 的证据过不了闸门" in err)

            # ⑨ 真 CLI 路径（子进程）—— ⚠ 用 `__file__`（不是主仓那份 ⇒ 否则测到别的构建 ✗）
            _me = str(Path(__file__).resolve())
            for _ver, _sha, _want in (("0.0.3", bad_sha, False), ("0.0.4", real_sha, True)):
                _ev = Path(tmp) / f"cli-{_ver}.jsonl"
                _ev.write_text("".join(json.dumps(e, ensure_ascii=False) + "\n" for e in (
                    {"event": "release-open", "id": f"REL-{_ver}", "version": _ver, "owner": "t"},
                    {"event": "advance", "id": "Z-1", "to": "done", "commit": _sha,
                     "reading": "夹具", "owner": "t"})), encoding="utf-8")
                _p = subprocess.run(
                    [sys.executable, _me, "--release", "add", "--rel", f"REL-{_ver}",
                     "--id", "Z-1", "--owner", "t"],
                    capture_output=True, text=True, cwd=str(ROOT),
                    env=dict(os.environ, SOKO_QUEUE_ROOT=str(ROOT), SOKO_QUEUE_EVENTS=str(_ev)))
                expect(f"⑨ 真 CLI：`--release add` {'真祖先 ⇒ rc=0' if _want else '假指针 ⇒ rc≠0'}"
                       f"（实测 rc={_p.returncode}）", (_p.returncode == 0) == _want)
            # ⑩ CLI 契约：新增的四个开关真在 parser 里
            _parser = build_arg_parser()
            _pa = _parser.parse_args(["--repoint", "X", "--reason", "w", "--dry-run",
                                      "--limit", "3"])
            expect("⑩ 真 parser 认 `--repoint … --reason/--dry-run/--limit`",
                   _pa.repoint == "X" and _pa.reason == "w" and _pa.dry_run and _pa.limit == 3)
            expect("⑩ 真 parser 认 `--repoint-all`",
                   _parser.parse_args(["--repoint-all"]).repoint_all is True)

            # ⑪ `--repoint-all` 端到端（两条共用一个 sha ⇒ **整 sha 一起写**）
            seed(("R-1", twin), ("R-2", twin))
            rc, out = capture_both(cmd_repoint_all, "夹具：整批回填", "t", False, 0)
            expect(f"⑪ `--repoint-all` 端到端 rc={rc}", rc == 0)
            expect("⑪ 打完复查 ⇒ 非祖先 **0 条**", "非祖先收口指针 **0 条**" in out)
            expect("⑪ 两条都换成真祖先",
                   fold()["R-1"].get("commit") == base_sha
                   and fold()["R-2"].get("commit") == base_sha)
            rows, _st, view = snapshot()
            rc, _e = capture(check_plan, view, rows)
            expect(f"⑪ 且 `check_plan` 转绿（红 {rc} 条）", rc == 0)
            # ⑪b **有找不到孪生的 ⇒ 明说没回填到 0**（不许打"0 条"的假绿 ✗）
            seed(("R-1", twin), ("R-2", lonely))
            rc, _out = capture(cmd_repoint_all, "夹具：一半找不到孪生", "t", False, 0)
            expect(f"⑪b 一半找不到孪生 ⇒ rc={rc}（**不许**假绿）", rc == 1)
            expect("⑪b 找不到孪生的那条**没被硬填**（指针还是旧的）",
                   fold()["R-2"].get("commit") == lonely)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]
            globals()["worktree_dirty"] = saved[5]
            globals()["probe_inventory"] = saved[6]


def selftest_release_cut_gate(expect) -> None:
    """⭐ **切版指针的证据闸门**（`#2`，2026-10-14 · 第七次独立复核实测 ✗）。

    缺口：`--release cut --commit <sha>` **自身一条闸门都没有** —— 干净夹具（3 条 done 入版、
    `--release check` rc=0）里 `--commit deadbeef…`（对象**不存在**）与「存在但**非祖先**」
    **都 rc=0** 并写入 `release-cut` ✗ ⇒ 入版/收口两处都要求"证据必须是真的"，切版这一处
    什么都收 ⇒ **同一规则两处口径**（`AGENTS.md` 硬规则 0(b)）。

    断言（每条都点了名靶子 ⇒ 定向突变必红）：
      ① 不存在 / 非祖先的切版指针 ⇒ **rc=1 且零写入**（**就是本轮的缺口**，改前 rc=0 ✗）；
      ② 正对照：真祖先 + 真对象 ⇒ **仍 rc=0**（**不许误拒** ✗）；
      ③ 切版事件记了 `subject` + `patch_id`（审计字段，同 `#1` 的理由）；
      ④ 缺 `--ci` / 缺 `--commit` ⇒ 用法错（rc=2，原有口径不变 ✓）；
      ⑤ ⭐ **读侧复查**（`#3`，第八次复核）：手写假 `release-cut`（审计字段与 `commit`
         对不上）⇒ **`--release check` 与 `check_plan` 两条读侧都判红 + 点名** ✓
         （改前**两条都静默** ✗ —— 审计字段没有任何生产读者 = 咬不住的守卫 ✗）。
    ⚠ 用 `unmerged_agent_branches()` 的**接缝**把真仓库的 15 条未合回分支隔离掉
    （否则夹具的 `check` 永远过不去 ⇒ 正对照测不了；`#4` 的那条臂反过来要它返回非空 ✓）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True).stdout.strip()
    dangling = subprocess.run(["git", "-c", "user.name=ux-queue-selftest",
                               "-c", "user.email=selftest@local", "commit-tree",
                               f"{real_sha}^{{tree}}", "-m", "ux-queue selftest: 切版夹具的悬垂"],
                              cwd=ROOT, capture_output=True, text=True).stdout.strip()
    bad_sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"],
                 globals()["worktree_dirty"], globals()["probe_inventory"],
                 globals()["unmerged_agent_branches"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["worktree_dirty"] = lambda: ""
        globals()["probe_inventory"] = lambda: []
        globals()["unmerged_agent_branches"] = lambda: []
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| R-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash r.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text("".join(
            json.dumps({"id": f"FX-{i}", "source": "orchestrator-schedule", "facet": "主线",
                        "severity": "medium", "repro": "bash fx.sh", "disposition": "fixed",
                        "why": "夹具：已关账"}, ensure_ascii=False) + "\n" for i in (1, 2, 3)),
            encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")
        for i in (1, 2, 3):
            append_event({"event": "advance", "id": f"FX-{i}", "to": "done", "commit": real_sha,
                          "reading": "夹具读数", "owner": "t"})
        append_event({"event": "release-open", "id": "REL-9.9.9", "version": "9.9.9", "owner": "t"})
        for i in (1, 2, 3):
            append_event({"event": "release-add", "rel": "REL-9.9.9", "id": f"FX-{i}", "owner": "t"})

        def ns(**kw):
            base = dict(release="", rel="", version="9.9.9", id="", commit="", ci="", reason="",
                        owner="t", force=False)
            base.update(kw)
            return argparse.Namespace(**base)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, buf.getvalue() + err.getvalue()

        try:
            _r, _u = build_queue()
            rows = _r
            rc, out = capture(cmd_release, ns(release="check"), rows)
            expect(f"夹具：干净节点 `--release check` rc={rc}（下面三条都在这上面比）", rc == 0)
            # ① 两条假指针都必须拒，且零写入
            _before = EVENTS.read_text(encoding="utf-8")
            rc, out = capture(cmd_release, ns(release="cut", commit=bad_sha, ci="1"), rows)
            expect(f"① `cut --commit deadbeef…`（对象**不存在**）⇒ 拒 rc={rc}", rc == 1
                   and "证据闸门" in out and "不存在" in out)
            expect("① 且**零写入**（没写 `release-cut` ✓）",
                   EVENTS.read_text(encoding="utf-8") == _before)
            rc, out = capture(cmd_release, ns(release="cut", commit=dangling, ci="1"), rows)
            expect(f"① `cut --commit <存在但非祖先>` ⇒ 拒 rc={rc}", rc == 1
                   and "证据闸门" in out and "不是 `HEAD` 的祖先" in out)
            expect("① 且**零写入**（没写 `release-cut` ✓）",
                   EVENTS.read_text(encoding="utf-8") == _before)
            # ② 正对照：真祖先 + 真对象 ⇒ 仍 rc=0（**不许误拒** ✗）
            rc, out = capture(cmd_release, ns(release="cut", commit=real_sha, ci="7"), rows)
            expect(f"② 正对照：`cut --commit <真祖先>` ⇒ 仍 rc={rc}（不许误拒 ✗）", rc == 0)
            _cuts = [e for e in load_events() if e.get("event") == "release-cut"]
            expect("② 真写了 `release-cut` 事件", len(_cuts) == 1 and _cuts[0].get("ci") == "7")
            # ③ 审计字段
            expect("③ 切版事件记了 `subject` + `patch_id`（事后可核「切的是哪个内容」✓）",
                   bool(_cuts and _cuts[0].get("subject") and _cuts[0].get("patch_id")
                        and _cuts[0]["patch_id"] == _patch_id(real_sha)))
            # ④ 原有两条用法错不变
            for kw, what in ((dict(release="cut", commit=real_sha), "缺 `--ci`"),
                             (dict(release="cut", ci="1"), "缺 `--commit`")):
                try:
                    capture(cmd_release, ns(**kw), rows)
                    _rc = 0
                except UsageError:
                    _rc = 2
                expect(f"④ {what} ⇒ 用法错 rc={_rc}（原有口径不变 ✓）", _rc == 2)
            # ⑤ ⭐ **读侧复查**（`#3`，2026-10-14 · 第八次独立复核实测 ✗）：`#2` 记进事件的
            #    `subject`/`patch_id` 原先**一个读者都没有**（`release_nodes()` 折叠时把它们
            #    丢了 ⇒ 唯一读者是自检自己的断言 ✗）⇒ **手写一条假 `release-cut` 无人咬** ✗。
            #    现在 `cut_audit_problem()` 在**两条读侧**（`--release check` 与 `check_plan`）
            #    都咬 ✓ —— 本臂就是那个靶子（**改前必红** ✗）。
            _legit = [e for e in load_events() if e.get("event") == "release-cut"][-1]
            append_event({"event": "release-cut", "id": "REL-9.9.9", "commit": dangling,
                          "ci": "7", "subject": _legit.get("subject"),
                          "patch_id": _legit.get("patch_id")})
            _rows2, _u2 = build_queue()
            _view2 = sched_rows(_rows2, fold())
            rc, out = capture(check_plan, _view2, _rows2)
            expect(f"⑤ 手写假 `release-cut`（commit 与审计字段对不上）⇒ `check_plan` 判红 rc={rc}",
                   rc >= 1 and "release-cut" in out and "对不上" in out)
            rc, out = capture(cmd_release, ns(release="check", rel="REL-9.9.9"), _view2)
            expect(f"⑤ 且 `--release check` 也判红 rc={rc}（两条读侧**共用一处判据** ✓）",
                   rc == 1 and "对不上" in out)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]
            globals()["worktree_dirty"] = saved[5]
            globals()["probe_inventory"] = saved[6]
            globals()["unmerged_agent_branches"] = saved[7]


def selftest_release_open_seam(expect) -> None:
    """⭐ **`--release open` 与幽灵判据的接缝**（`#2`，2026-10-14 · 第八次独立复核实测 ✗）。

    缺口：`cmd_release open` **只写 `release-open` 事件、不登记来源行**（那是它正确的形状 ——
    节点不是工作条目），而幽灵判据只认三个来源 ⇒ 开节点后**下一条 `--check` 判红**
    「`REL-9.9.9` 不在任何来源里 ⇒ 幽灵条目」✗。而 `scripts/soko gate --fast` 与 CI 的
    `ledger` job 跑的正是 `--check` ⇒ **§9.20 发版工作流第一步就撞红** ✗，且输出/文档一个字
    没提要"先登记"。
    ⚠ **这条接缝原先没有任何自检跑过** ✗：三个 `selftest_release_*` **都只调 `cmd_release`、
    从不调 `check_plan`** ⇒ 本函数专钉它（**改前必红 / 改后必绿**）。

    断言（每条都点了名靶子）：
      ① 基线：干净夹具 `check_plan` **rc=0**（下面那条红不是"夹具本来就红"）；
      ② ⭐ `cmd_release open` rc=0 ⇒ **紧接着 `check_plan` 仍 rc=0**（**改前这里是 rc=1 +
         「幽灵条目」** ✗）；
      ③ 正对照：`--release add/check/cut` 三条后续步骤**仍 rc=0**（**别为绿而放宽** ✗）；
      ④ **不是放水**：节点 id 确实进了 `known`（`release_node_ids()`），但手写一条**没有
         `release-open`** 的 `release-add`（引用无来源的 id）⇒ **照样判红 + 点名** ✓。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True).stdout.strip()
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"],
                 globals()["worktree_dirty"], globals()["probe_inventory"],
                 globals()["unmerged_agent_branches"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["worktree_dirty"] = lambda: ""
        globals()["probe_inventory"] = lambda: []
        globals()["unmerged_agent_branches"] = lambda: []
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| R-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash r.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text("".join(
            json.dumps({"id": f"FX-{i}", "source": "orchestrator-schedule", "facet": "主线",
                        "severity": "medium", "repro": "bash fx.sh", "disposition": "fixed",
                        "why": "夹具：已关账"}, ensure_ascii=False) + "\n" for i in (1, 2, 3)),
            encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")
        for i in (1, 2, 3):
            append_event({"event": "advance", "id": f"FX-{i}", "to": "done", "commit": real_sha,
                          "reading": "夹具读数", "owner": "t"})

        def ns(**kw):
            base = dict(release="", rel="", version="9.9.9", id="", commit="", ci="", reason="",
                        owner="t", force=False)
            base.update(kw)
            return argparse.Namespace(**base)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, buf.getvalue() + err.getvalue()

        def view():
            """当前夹具的**队列视图**（= `main()` 喂给 `check_plan`/`cmd_release` 的那一份）。"""
            _rows, _u = build_queue()
            return sched_rows(_rows, fold())

        def plan():
            """**真 CLI 的 `--check` 那一半**：`main()` 里就是 `check_plan(view, rows)`。"""
            _rows, _u = build_queue()
            return capture(check_plan, sched_rows(_rows, fold()), _rows)

        try:
            # ① 基线
            rc, out = plan()
            expect(f"① 基线：干净夹具 `check_plan` rc={rc}（下面那条红不是夹具自带的）", rc == 0)
            # ② 开节点 ⇒ 接缝必须仍是绿的（**改前这里是 rc=1 + 幽灵条目** ✗）
            nrc, nout = capture(cmd_release, ns(release="open"), view())
            expect(f"② `--release open` rc={nrc}（开节点本身一直是绿的）", nrc == 0)
            rc, out = plan()
            expect(f"② ⭐ 开节点后 `check_plan` 仍 rc={rc}（**改前 rc=1 + 幽灵条目** ✗）", rc == 0)
            expect("② 且输出里不再把 `REL-9.9.9` 当幽灵条目点名",
                   "REL-9.9.9" not in out)
            # ③ 正对照：发版工作流后续步骤仍能走（**别为绿而放宽** ✗）
            for i in (1, 2, 3):
                arc, _o = capture(cmd_release, ns(release="add", rel="REL-9.9.9", id=f"FX-{i}"),
                                  view())
                expect(f"③ 正对照 `--release add FX-{i}` rc={arc}（入版闸门不受影响 ✓）", arc == 0)
            crc, cout = capture(cmd_release, ns(release="check", rel="REL-9.9.9"), view())
            expect(f"③ 正对照 `--release check` rc={crc}（节点就绪，不是恒红 ✗）", crc == 0)
            krc, kout = capture(cmd_release, ns(release="cut", rel="REL-9.9.9", commit=real_sha,
                                                ci="7"), view())
            expect(f"③ 正对照 `--release cut --commit <真祖先> --ci 7` rc={krc}", krc == 0)
            # ④ **不是放水**：没有 `release-open` 的 id 照样判红
            expect("④ 节点 id 确实进了 `known`（`release_node_ids()`）",
                   "REL-9.9.9" in release_node_ids())
            append_event({"event": "release-add", "rel": "REL-0.0.1", "id": "GHOST-SEAM-9",
                          "owner": "t"})
            _rows2, _u2 = build_queue()
            rc, out = capture(check_plan, sched_rows(_rows2, fold()), _rows2)
            expect(f"④ 手写 `release-add`（无 `release-open`、id 无来源）⇒ 判红 rc={rc}",
                   rc == 1 and "GHOST-SEAM-9" in out and "幽灵" in out)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]
            globals()["worktree_dirty"] = saved[5]
            globals()["probe_inventory"] = saved[6]
            globals()["unmerged_agent_branches"] = saved[7]


def selftest_release_branch_fence(expect) -> None:
    """⭐⭐ **"分支必须合回"这条栅栏：如实判红 · 给出路 · `--force` 不越过**（`#4`）。

    缺口（2026-10-14 · 第七次独立复核实测 ✗）：真仓库有 **15 条** `agent/*` 没合回 main，
    而 `--force` 只越过「条目数」与「worktree 提醒」⇒ `--release check` **永远过不了**；
    旧实现既没说清"这条栅栏没有旁路"，也没有逐条印出可照做的出路 ⇒ 读的人只能猜。

    断言（每条都点了名靶子）：
      ① 未合回分支 ⇒ **rc=1**，输出**逐条**印 `scripts/agent-worktree.sh land <id>`；
      ② 输出**明说**「没有 `--force` 旁路」（口径写在白纸黑字里，不是靠读代码猜 ✓）；
      ③ ⭐ **`--force` 也 rc=1**（**不许有人偷偷加旁路** —— 这条臂就是那个靶子 ✗）；
      ④ 正对照：合回（分支表为空）⇒ **rc=0**（栅栏不是恒红 ✗）。
    """
    import contextlib
    import io
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True).stdout.strip()
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"],
                 globals()["worktree_dirty"], globals()["probe_inventory"],
                 globals()["unmerged_agent_branches"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["worktree_dirty"] = lambda: ""
        globals()["probe_inventory"] = lambda: []
        globals()["unmerged_agent_branches"] = lambda: ["agent/alpha", "agent/beta"]
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| R-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash r.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text("".join(
            json.dumps({"id": f"FX-{i}", "source": "orchestrator-schedule", "facet": "主线",
                        "severity": "medium", "repro": "bash fx.sh", "disposition": "fixed",
                        "why": "夹具：已关账"}, ensure_ascii=False) + "\n" for i in (1, 2, 3)),
            encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")
        for i in (1, 2, 3):
            append_event({"event": "advance", "id": f"FX-{i}", "to": "done", "commit": real_sha,
                          "reading": "夹具读数", "owner": "t"})
        append_event({"event": "release-open", "id": "REL-9.9.9", "version": "9.9.9", "owner": "t"})
        for i in (1, 2, 3):
            append_event({"event": "release-add", "rel": "REL-9.9.9", "id": f"FX-{i}", "owner": "t"})

        def ns(**kw):
            base = dict(release="", rel="", version="9.9.9", id="", commit="", ci="", reason="",
                        owner="t", force=False)
            base.update(kw)
            return argparse.Namespace(**base)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, buf.getvalue() + err.getvalue()

        try:
            _r, _u = build_queue()
            rows = _r
            rc, out = capture(cmd_release, ns(release="check"), rows)
            expect(f"① 有未合回分支 ⇒ 判红 rc={rc}", rc == 1 and "并行收拢未完成" in out)
            expect("① 输出**逐条**给出路（`scripts/agent-worktree.sh land <id>`）",
                   "agent-worktree.sh land alpha" in out and "agent-worktree.sh land beta" in out)
            expect("② 输出**明说**这条栅栏没有 `--force` 旁路",
                   "没有" in out and "`--force` 旁路" in out)
            rc_f, out_f = capture(cmd_release, ns(release="check", force=True), rows)
            expect(f"③ ⭐ **`--force` 也 rc={rc_f}**（不许偷偷加旁路 ✗）", rc_f == 1
                   and "并行收拢未完成" in out_f)
            globals()["unmerged_agent_branches"] = lambda: []
            rc, out = capture(cmd_release, ns(release="check"), rows)
            expect(f"④ 正对照：合回（分支表空）⇒ 仍 rc={rc}（栅栏不是恒红 ✗）", rc == 0)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]
            globals()["worktree_dirty"] = saved[5]
            globals()["probe_inventory"] = saved[6]
            globals()["unmerged_agent_branches"] = saved[7]


def selftest_id_and_encoding_gates(expect) -> None:
    """`#5`：**id 长度上限** + **非 UTF-8 环境的契约错**（2026-10-14 · 第七次独立复核实测 ✗）。

    缺口：① `--schedule` 只查 `QUEUE_ID_RE`、**没有长度上限** ⇒ 实测 **3003 字符**的 id
    **rc=0 收下**（写进 extras + 事件表）✗；② `LC_ALL=C PYTHONUTF8=0` 下打印中文抛
    `UnicodeEncodeError`、孪生搜索的 `git log`/`git show` 抛 `UnicodeDecodeError`
    ⇒ **栈 + rc=1** ✗（契约里环境错是 **rc=2** ✓）。

    断言（每条都点了名靶子）：
      ① 三个**生 id 的入口**（`--schedule`/`--submit`/`--add-child`）对 3000 字符 id **都拒**
         （横向排查：只修 `--schedule` 会漏另外两处 ✗）+ **零写入**；
      ② 边界：`ID_MAX_LEN` 字符**过**、`ID_MAX_LEN+1` **拒**（上限是数字，不是形容词 ✓）；
      ③ `_guard_encoding()` 在 `US-ASCII` 的 locale 下 ⇒ `UsageError`（rc=2 那一类，**不是**栈 ✗）；
      ④ **真 CLI 路径**（子进程 + `LC_ALL=C PYTHONUTF8=0`）：**没有 Traceback**，且
         rc ∈ {0, 2}（环境真能表示 UTF-8 的平台走 0 = 正对照；否则必须是 2 + 人话 ✓）
         —— 这一条咬住的正是"栈 + rc=1"那个形状 ✗。
    """
    import contextlib
    import io
    import locale
    import tempfile
    global EVENTS, EXTRAS, INDEX, GAPS_LEDGER
    long_id = "FX-" + "A" * 3000
    with tempfile.TemporaryDirectory() as tmp:
        saved = (EVENTS, EXTRAS, INDEX, GAPS_LEDGER, globals()["lock_file"],
                 globals()["worktree_dirty"], globals()["probe_inventory"])
        EVENTS = Path(tmp) / "events.jsonl"
        EXTRAS = Path(tmp) / "extras.jsonl"
        INDEX = Path(tmp) / "index.md"
        GAPS_LEDGER = Path(tmp) / "ledger.jsonl"
        globals()["lock_file"] = lambda: Path(tmp) / "q.lock"
        globals()["worktree_dirty"] = lambda: ""
        globals()["probe_inventory"] = lambda: []
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         "| R-9 | 语言 | 夹具（没处置 ⇒ 不进 rows） | 高 | `bash r.sh` | front |  |\n",
                         encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        # ⚠ 夹具必须有一个**有处置**的父条目（`build_queue` 把"没处置"的行只放进
        # `undecided`、**不进 rows** ⇒ `--add-child` 的父闸门会抛 UsageError ✗，实测踩到）。
        EXTRAS.write_text(json.dumps(
            {"id": "FX-P", "source": "orchestrator-schedule", "facet": "主线", "severity": "medium",
             "repro": "bash fx.sh", "disposition": "fix", "why": "夹具：父条目"},
            ensure_ascii=False) + "\n", encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, buf.getvalue() + err.getvalue()

        def sched_ns(ident: str) -> argparse.Namespace:
            return argparse.Namespace(schedule=ident, id=ident, title="夹具", reason="夹具",
                                      owner="t", where="", facet="", severity="", repro="")

        def submit_ns(ident: str) -> argparse.Namespace:
            return argparse.Namespace(submit="fix", id=ident, why="夹具", evidence="rc 0",
                                      patch="", facet="", severity="", repro="", where="夹具",
                                      owner="t")

        try:
            _r, _u = build_queue()
            rows = _r
            # ① 三个生 id 的入口都要拒超长 id，且零写入
            for name, fn, ns in (("--schedule", cmd_schedule, sched_ns(long_id)),
                                 ("--submit", cmd_submit, submit_ns(long_id))):
                _before = EVENTS.read_text(encoding="utf-8") + EXTRAS.read_text(encoding="utf-8")
                rc, out = capture(fn, ns)
                expect(f"① `{name}` 收 3000 字符 id ⇒ 拒 rc={rc}", rc == 1 and "id 太长" in out)
                expect(f"① `{name}` 被拒时**零写入**",
                       EVENTS.read_text(encoding="utf-8") + EXTRAS.read_text(encoding="utf-8")
                       == _before)
            # ⚠ 父闸门可能抛 UsageError（父不在队列里）⇒ 兜住它，别让整条自检崩成 rc=2 ✗
            try:
                rc, out = capture(cmd_add_child, "FX-P", long_id, "夹具子任务", "t", rows)
            except UsageError as _err:
                rc, out = 2, str(_err)
            expect(f"① `--add-child --child <3000 字符>` ⇒ 拒 rc={rc}",
                   rc == 1 and "id 太长" in out)
            expect("① `--add-child` 被拒时**零写入**（没写 `split`）",
                   EVENTS.read_text(encoding="utf-8") == "")
            # ② 边界：上限正好过、上限 +1 拒
            _ok = "FX-" + "B" * (ID_MAX_LEN - 3)
            _no = "FX-" + "B" * (ID_MAX_LEN - 2)
            expect(f"② 长度上限是数字：{len(_ok)} 字符 ⇒ 过（`_id_shape_problem` = None）",
                   len(_ok) == ID_MAX_LEN and _id_shape_problem(_ok) is None)
            expect(f"② {len(_no)} 字符 ⇒ 拒（> ID_MAX_LEN={ID_MAX_LEN}）",
                   _id_shape_problem(_no) is not None and "id 太长" in _id_shape_problem(_no))
            # ③ 编码闸门：locale 表示不了中文 ⇒ UsageError（rc=2 那一类）
            _saved_enc = locale.getpreferredencoding
            locale.getpreferredencoding = lambda *a, **kw: "US-ASCII"      # noqa: ARG005
            try:
                try:
                    _guard_encoding()
                    _rc_enc = 0
                except UsageError as _e:
                    _rc_enc, _msg_enc = 2, str(_e)
                expect(f"③ locale=US-ASCII ⇒ `_guard_encoding` 抛 UsageError（rc={_rc_enc}）",
                       _rc_enc == 2 and "环境编码不是 UTF-8" in _msg_enc)
            finally:
                locale.getpreferredencoding = _saved_enc
            # ④ 真 CLI 路径：不许有栈，rc 只能是 0（正对照）或 2（契约错）
            _alt = Path(tmp) / "enc-events.jsonl"
            _env = dict(os.environ, LC_ALL="C", PYTHONUTF8="0", LANG="C",
                        SOKO_QUEUE_ROOT=str(ROOT), SOKO_QUEUE_EVENTS=str(_alt))
            _p = subprocess.run([sys.executable, str(Path(__file__)), "--json"],
                                capture_output=True, text=True, env=_env, cwd=str(ROOT))
            _e = (_p.stderr or "") + (_p.stdout or "")
            expect(f"④ 非 UTF-8 环境下真 CLI **没有 Traceback**（rc={_p.returncode}）",
                   "Traceback" not in _e)
            expect(f"④ 真 CLI rc ∈ {{0, 2}}（实测 {_p.returncode}；**不许是 1 + 栈** ✗）",
                   _p.returncode in (0, 2))
            expect("④ rc=2 时必须是**人话**（说清环境编码 + 给出正解）",
                   _p.returncode != 2 or ("环境编码不是 UTF-8" in _e and "PYTHONUTF8=1" in _e))
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]
            globals()["worktree_dirty"] = saved[5]
            globals()["probe_inventory"] = saved[6]


def selftest_write_gates(expect) -> None:
    """**两个写入口都要看权威状态**（2026-10-14 · 第六次独立复核实测 ✗）—— #3 的防复发轴。

    改前实测（纯 CLI，临时根）：
      · `--advance X --to todo --reason …`（来源 `fixed` + 事件 `done`）⇒ **rc=0** 且打印
        「✓ 状态回到来源处置」—— **实际没回到**（来源仍是 `fixed`，事件成了 `todo`）⇒
        下一条 `--check` **判红**（反向两层漂移）、`--next` **又推出它** ✗（对照：来源 `fix`
        时 reopen rc=0 ✓）；
      · `--add-child` 到「来源 `fixed` + **无事件**」的父 ⇒ **rc=0**（闸门只看
        `state[父]["state"]`，而那种父在 `state` 里**根本没有键** ✗）⇒ `split` 把父折成
        `umbrella` ⇒ `--check` 判红 ✗。

    断言（① ② 是"把闸门改回只看事件状态"的靶子 ⇒ 各自必红 ✗）：
      ① 来源关账 + 事件终态 ⇒ `--advance --to todo` **硬拒** + 给**走得通**的指示 + 不写事件；
      ② 来源关账 + **无事件** ⇒ `--add-child` **硬拒** + 不写 `split` 事件；
      ③ 正对照（**不许误拒**）：来源 `fix` + 事件 `done` ⇒ reopen **rc=0** 且转绿；
      ④ 正对照（**不许误拒**）：来源 `fix` 的父 ⇒ `--add-child` **rc=0** 且（子任务登记后）转绿；
      ⑤ **谓词只有一份**：`source_closed(row)` ≡ `effective_state(row, {}) in CLOSED_STATES`
         逐处置成立（6 个处置全覆盖 ⇒ 以后加处置忘了归类会先红 ✗）。
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

        def ext(ident: str, disp: str) -> dict:
            return {"id": ident, "source": "ux-index", "facet": "语言", "severity": "medium",
                    "repro": f"bash {ident}.sh", "disposition": disp, "why": "夹具"}

        ids = ("X-1", "X-2", "X-3", "P-1", "P-2")
        INDEX.write_text("## 2. 🔴 blocker / high\n\n"
                         "| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核（观察） |\n"
                         "|---|---|---|---|---|---|---|\n"
                         + "".join(f"| {i} | 语言 | 夹具 | 高 | `bash {i}.sh` | front |  |\n"
                                   for i in ids), encoding="utf-8")
        GAPS_LEDGER.write_text("", encoding="utf-8")
        EXTRAS.write_text("".join(json.dumps(ext(i, d), ensure_ascii=False) + "\n" for i, d in
                                  (("X-1", "fixed"), ("X-2", "fix"), ("X-3", "fixed"),
                                   ("P-1", "fixed"), ("P-2", "fix"))), encoding="utf-8")
        EVENTS.write_text("", encoding="utf-8")
        real_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                  capture_output=True, text=True).stdout.strip()

        def snapshot():
            _r, _u = build_queue()
            _s = fold()
            return _r, _s, sched_rows(_r, _s)

        def capture(fn, *a, **kw):
            buf, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                rc = fn(*a, **kw)
            return rc, err.getvalue()

        try:
            # ⑤ 谓词等价（纯表，不碰夹具）—— "来源侧"与"调度器侧"两个集合不许各说各话
            for _d in sorted(DISPOSITIONS):
                _row = {"id": "Z-9", "disposition": _d}
                expect(f"⑤ `source_closed({_d})` ≡ `effective_state(row, {{}}) in CLOSED_STATES`",
                       source_closed(_row) == (effective_state(_row, {}) in CLOSED_STATES))
            # ① 来源关账 + 事件终态（一致形状）⇒ reopen 硬拒
            append_event({"event": "advance", "id": "X-1", "to": "done", "commit": real_sha,
                          "reading": "夹具：已收口", "owner": "t"})
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"① 前置：来源 `fixed` + 事件 `done` 是**一致**的（红 {rc} 条）", rc == 0)
            rc, err = capture(cmd_advance, "X-1", "todo", "", "", "夹具：重开", "t", False, view)
            expect(f"① 来源关账 ⇒ reopen **硬拒**（改前 rc=0 制造必红状态 ✗；实测 {rc}）",
                   rc == 1 and "已关账" in err and "来源处置" in err)
            expect("① 指示**给得出路**（说清先把来源处置改回 `fix`）",
                   "--triage X-1 --to deferred" in err)
            expect("① 被拒时**没有** `reopen` 事件落表",
                   not [e for e in load_events() if e.get("event") == "reopen"])
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"① 且状态**没被弄红**（红 {rc} 条）", rc == 0)
            # ①b 来源关账 + **无事件**（`X-3`）⇒ 也必须拒，且**不许**是"非法迁移 `todo → todo`"
            #     那种信息错（旧闸门只看 `frm in TERMINAL_STATES` ⇒ 这种 id 连分支都进不来 ⇒
            #     打的是"非法迁移"、也没有出路指示 ✗）—— 这是"分支条件换成权威状态"的靶子 ✓
            rows, state, view = snapshot()
            rc, err = capture(cmd_advance, "X-3", "todo", "", "", "夹具：重开", "t", False, view)
            expect(f"①b 来源关账 + **无事件** ⇒ 也硬拒，且说清「已关账」而**不是**「非法迁移」"
                   f"（实测 rc={rc}）", rc == 1 and "已关账" in err and "非法迁移" not in err)
            # ③ 正对照：来源 `fix` + 事件 `done` ⇒ reopen 仍 rc=0 且转绿
            append_event({"event": "advance", "id": "X-2", "to": "done", "commit": real_sha,
                          "reading": "夹具：已收口", "owner": "t"})
            rows, state, view = snapshot()
            rc, err = capture(cmd_advance, "X-2", "todo", "", "", "夹具：重开", "t", False, view)
            expect(f"③ 来源 `fix` 的合法 reopen **仍必须 rc=0**（不许误拒；实测 {rc}）", rc == 0)
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"③ 且**转绿**（红 {rc} 条）", rc == 0)
            # ② 来源关账 + 无事件 ⇒ `--add-child` 硬拒
            rows, state, view = snapshot()
            rc, err = capture(cmd_add_child, "P-1", "K-1", "夹具子任务", "t", view)
            expect(f"② 来源关账的父 ⇒ `--add-child` **硬拒**（改前 rc=0 ✗；实测 {rc}）",
                   rc == 1 and "已关账" in err)
            expect("② 被拒时**没有** `split` 事件落表",
                   not [e for e in load_events() if e.get("event") == "split"])
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"② 且状态没被弄红（红 {rc} 条）", rc == 0)
            # ④ 正对照：来源 `fix` 的父 ⇒ rc=0；子任务登记来源后转绿
            rows, state, view = snapshot()
            rc, err = capture(cmd_add_child, "P-2", "K-2", "夹具子任务", "t", view)
            expect(f"④ 来源 `fix` 的父 **仍必须 rc=0**（不许误拒；实测 {rc}）", rc == 0)
            with EXTRAS.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(ext("K-2", "fix"), ensure_ascii=False) + "\n")
            rows, state, view = snapshot()
            rc, err = capture(check_plan, view, rows)
            expect(f"④ 且**转绿**（红 {rc} 条）", rc == 0)
        finally:
            EVENTS, EXTRAS, INDEX, GAPS_LEDGER = saved[0], saved[1], saved[2], saved[3]
            globals()["lock_file"] = saved[4]


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
            # ③ **存量**（记录时刻 < 口径起点）⇒ ⭐ **也判红**（2026-10-14 · `RELEASE-EVIDENCE-GATE`）
            #    ⚠ 这一臂**改前是「不判红 + 打豁免」** —— 豁免已按编排者口径**删掉**
            #    （存量全量回填到 0 ⇒ 口径改成"非祖先必须 0 条"的断言）⇒ 期望**反过来**。
            #    这是"把豁免线改回带日期豁免"的**靶子**：改回去 ⇒ 这一臂必红 ✓。
            EVENTS.write_text("", encoding="utf-8")
            append_event({"event": "advance", "id": "G-2", "to": "done", "commit": dangling,
                          "reading": "夹具读数",
                          "ts": (_cut - _dt.timedelta(minutes=1)).isoformat(), "owner": "t"})
            _r3, _s3, _v3 = snapshot()
            rc, err = capture(check_plan, _v3, _r3)
            expect(f"③ 存量（早于口径起点）非祖先 ⇒ **也判红**（豁免已删；改前 rc=0 ✗；实测红 {rc} 条）",
                   rc >= 1)
            expect("③ 判红点名 `G-2` + 说「不是 `HEAD` 的祖先」+ 给出 `--repoint` 出路"
                   f"[点名={'G-2' in err} / 祖先性={'不是 `HEAD` 的祖先' in err} / "
                   f"repoint={'--repoint' in err}]",
                   "G-2" in err and "不是 `HEAD` 的祖先" in err and "--repoint" in err)
            # ③b **非祖先 0 条 ⇒ 断言也要打出来**（绿也要可见 ⇒ 不许静默 ✓）
            #     ⚠ 断言走 **stdout**（✓ 是报告、✗ 才是 stderr）⇒ 这里要自己抓 stdout
            #     （`capture()` 只回 stderr ⇒ 用它就会**假红** ✗）。
            EVENTS.write_text("", encoding="utf-8")
            _r3b, _s3b, _v3b = snapshot()
            _ob, _eb = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(_ob), contextlib.redirect_stderr(_eb):
                rc = check_plan(_v3b, _r3b)
            expect(f"③b 非祖先 0 条 ⇒ 不判红（红 {rc} 条）", rc == 0)
            expect("③b 且**打出「非祖先 0 条」的断言**（绿也不许静默 ✓）",
                   "非祖先 0 条" in _ob.getvalue())
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
            # B2. ⭐ **双向的另一半必须有牙**（`#3`，2026-10-14 · 第七次独立复核实测 ✗）：
            #     B 只断言"补上行 ⇒ 转绿"，而**删掉那一行照样 rc=0** ✗（同一臂的反向没有鉴别力）
            #     ⇒ 这里删掉 §0.2 那一行（渲染块留着）⇒ **必须判红 + 点名 X-1**。
            tmp_onb.write_text("## 0.2 下一步\n\n" + render_onboarding(rows) + "\n\n"
                               "| id | 线 |\n|---|---|\n", encoding="utf-8")
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"B2 **删掉 `scheduled` 的 §0.2 行 ⇒ 判红**（rc={rc}）", rc == 1)
            expect("B2 判红信息里**点名**了那条 id", "X-1" in err.getvalue())
            expect("B2 判红信息说的是「§0.2 缺行」",
                   "§0.2 缺行" in err.getvalue())
            # B3. 正对照：处置换成 `fix`（细粒度队列）⇒ **不要求**进 §0.2 ⇒ 仍转绿 ✓
            #     （`load_onboarding_queue` 的口径：§0.2 不逐条列 open 明细 ⇒ 只有 `scheduled`
            #      才要求有行 —— 这一臂钉住"没有把 `fix` 也一起判红" ✗）
            tmp_extras.write_text(json.dumps(
                {"id": "X-1", "source": "ux-index", "facet": "语言", "severity": "high",
                 "repro": "bash x.sh", "disposition": "fix", "why": "夹具"},
                ensure_ascii=False) + "\n", encoding="utf-8")
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
                _r, _u = build_queue()
                rc = check_uncovered(_r, live=False, timeout=5, undecided=_u)
            expect(f"B3 正对照：`fix` 不要求进 §0.2 ⇒ 仍转绿（rc={rc}）", rc == 0)
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
    # ③h **祖先性闸门**（2026-10-14：`cat-file -e` 查不出"不是 main 祖先" ⇒ 新增硬拒 + 读侧复查。
    #     ⚠ 口径已于同日改成**「非祖先必须 0 条」的断言**（存量全量回填到 0 ⇒ 豁免已删）✗）
    selftest_ancestry_gate(expect)
    # ③h2 ⭐ **证据闸门 + 指针回填**（2026-10-14 · `RELEASE-EVIDENCE-GATE`：`--release add/check`
    #     原先只要 `done` + 有 `commit` ⇒ **手写的假指针能进发布节点** ✗；配套 `--repoint`
    #     把存量 rebase 孪生回填到 0——三关找孪生、找不到**不硬填**、只追加事件可逐条回滚）
    selftest_repoint_and_release_gate(expect)
    # ③h3 ⭐ **切版指针的证据闸门**（`#2`，2026-10-14 · 第七次独立复核实测 ✗：`--release cut
    #     --commit <sha>` 自身一条闸门都没有 ⇒ 不存在的 / 非祖先的 sha **都 rc=0** 并写入
    #     `release-cut` ⇒ 与入版/收口两处**同一规则两条口径**）
    selftest_release_cut_gate(expect)
    # ③h3b ⭐ **`--release open` 与幽灵判据的接缝**（`#2`，2026-10-14 · 第八次独立复核实测 ✗：
    #     `open` 只写 `release-open` 事件、不登记来源行 ⇒ 开节点后下一条 `--check` 判
    #     「幽灵条目」✗，而 `soko gate --fast` 与 CI 的 `ledger` job 跑的正是 `--check`；
    #     三个 `selftest_release_*` 原先**都只调 `cmd_release`、从不调 `check_plan`**
    #     ⇒ 这条接缝没有任何自检跑过 ✗）
    selftest_release_open_seam(expect)
    # ③h4 ⭐⭐ **「分支必须合回」栅栏：如实判红 · 逐条给出路 · `--force` 不越过**（`#4`，
    #     2026-10-14 · 第七次独立复核实测 ✗：真仓库 15 条未合回 ⇒ `--release check` 永远过不了；
    #     口径选"如实写清 + 不放宽"⇒ 这条臂钉住「`--force` 也 rc=1」，将来偷加旁路当场判红 ✗）
    selftest_release_branch_fence(expect)
    # ③h5 **id 长度上限 + 非 UTF-8 环境的契约错**（`#5`，2026-10-14 · 第七次独立复核实测 ✗：
    #     3003 字符 id rc=0 收下 · `LC_ALL=C PYTHONUTF8=0` ⇒ 栈 + rc=1 而不是 rc=2）
    selftest_id_and_encoding_gates(expect)
    # ③i **无源子任务**（2026-10-14 · 第六次复核 #1：`sched_rows` 的合成行给自己作出处 ⇒
    #     幽灵判据对它静默，而 `--next` 推得出它 ✗ —— `known` 改回读视图这条必红 ✗）
    selftest_ghost_child(expect)
    # ③j **`--triage` 的假证据**（2026-10-14 · 第六次复核 #2：`cmd_triage` 一条硬拒都没有 +
    #     `fold` 的 triage 分支不写 `advanced_ts` ⇒ 起点之后的假指针被当"存量"豁免 ✗）
    selftest_triage_pointer(expect)
    # ③k **两个写入口的权威状态闸门**（2026-10-14 · 第六次复核 #3：`--advance --to todo` 与
    #     `--add-child` 都只看事件状态 ⇒ rc=0 制造已知必红的状态 ✗）
    selftest_write_gates(expect)

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
          f" + **子任务视图 15 条**（源行优先/顺序不变/不丢行/无重复/反向两层漂移咬得住**且点名 K-2**/"
          f"**`--json` 单一真相**/视图被遮蔽仍以来源为准/"
          f"**③b 无源子任务判红 + 点名 + 说清「无源子任务」**（第六次复核 #1）/收口转绿/"
          f"**三视图同数**/`--next` 子集一致）"
          f" + **来源可见性 17 条**（gap/extras/索引三个来源的终态都进全量权威表/"
          f"三条反向两层漂移都判红且无多余红/索引登记过的 id 不算幽灵但覆盖自检照样判红/"
          f"**终态行进 `build_queue()` 的 rows（合并 = 权威）但不进 open/§0.2/`--next`**"
          f"（第五次复核把这条的口径从病根改成不变量）/**一份输入**：队列的键 = 三来源全部 id/"
          f"**索引少一列的数据行响亮报错/**已关账缺口的 id 不许被重新排期/"
          f"`--submit` 同族闸门也看得见终态 id + 正对照**）"
          f" + **来源 × 处置 × 调度器状态网格**"
          f"（{len(GRID_PAIRS)} 个「来源 × 已关账处置」组合 = 3 来源 × docstring 的 "
          f"{len(CLOSED_DISPOSITIONS)} 个已关账处置 × {len(GRID_STATES)} 个调度器状态"
          f" = **{GRID_CELLS} 格**「有事件」层 · **{GRID_NOEVENT_CELLS} 格**「无事件」层"
          f"（状态由处置派生 · = docstring 的『删掉事件文件』那一臂）+ **{GRID_TERMINAL_CELLS} 格**"
          f"终态层（`done`/`wo_filed` 与来源的工作结论对账 ⇒ 镜像方向也有牙）"
          f" + {len(GRID_NOEVENT_OPEN)} + {GRID_TERMINAL_CTRL_REDS} 格正对照："
          f"逐格「判红点名 + 收口指示**真走得通**，或 `boundary`/`control` **如实说没有平账路**」"
          f" + 夹具形状 4 条 + 域/三视图/子任务口径 + **判红格总数恰好 {GRID_REDS}**"
          f" + {len(GRID_CONFLICTS)} × {len(GRID_CONFLICT_STATES)} = **{GRID_CONFLICT_CELLS} 格**"
          f"**跨来源处置冲突轴**（第五次复核：网格原先每格都是**单来源** ⇒ 这一族结构上看不见 ✗；"
          f"逐格 = 单来源臂不许冲突 + 双来源臂**真 CLI `--check-plan` rc=2**、点名 id + 两处置 + "
          f"**各自出处**、`fixed` 与 `wo-filed` 两支**同判**（不许更松 ✗）、`--next`/`--json` "
          f"走同一条闸门 + 轴的前提由**真加载器**核）"
          f" + 5 条**结构性**守卫（覆盖所有非终态/所有处置 · 每个非终态有出口 · 已关账 = "
          f"docstring 的四处置 · 冲突轴覆盖所有不等对）：改前 `submitted` 列 6/6 静默、"
          f"`wo_filed` 一向无牙、`boundary`/`control` 12 格静默且被 `--next` 推、"
          f"**删掉覆盖层**后 42 格静默而 `--next` 照推、**终态缺口行让冲突闸门失明** ⇒ 逐格必红 ✗）"
          f" + **祖先性闸门 11 条**（非祖先 sha 收口**硬拒**/新增非祖先判红/"
          f"⭐ **口径已改成「非祖先必须 0 条」的断言**：存量（记于 `ANCESTRY_GATE_SINCE` **之前**）"
          f"**也判红** —— 带日期豁免**已按编排者口径删掉** ✗/绿也要打出「非祖先 0 条」·真祖先不误伤）"
          f" + ⭐ **证据闸门 + 指针回填 40 条**（`RELEASE-EVIDENCE-GATE`：**真孪生夹具**"
          f"（同树 + 同父 + 同 subject ⇒ 同 `patch-id --stable`、但**非**祖先）/三关认得它/"
          f"`--dry-run` **零写入**/`--repoint` 自动找孪生 ⇒ 指针换真祖先 · **state 仍 done** · "
          f"旧事件留着 + `repointed_from` 记旧 sha/回填前红 2 条 ⇒ 回填后红 1 条（**因果对照**）/ "
          f"显式非祖先与不存在**两半硬拒**/**找不到孪生不硬填**且零写入/已祖先拒/缺 reason 拒/"
          f"id 不存在 UsageError/ ⭐ **`--release add` 假指针 rc≠0**（**就是本轮的缺口**，改前 rc=0 ✗）"
          f"+ 被拒时**不写事件** + **正对照真祖先仍 rc=0**（不许误拒）/对象**不存在**那一半/"
          f"`--release check` **读侧也咬**（手写事件塞进来的）/ **真 CLI 子进程**跑前面两半/"
          f"`--repoint-all` 端到端 + 一半找不到孪生**不许假绿**/四个新开关真在 parser 里；"
          f"摘掉任一处 `closure_pointer_problem` 或三关里任一关 ⇒ 对应那条必红 ✗）"
          f" + **无源子任务 10 条**（第六次复核 #1：**判红 + 点名 + 说清「无源子任务」**/"
          f"合成行 `sources` = 诚实的空表且与源行同形/登记来源后转绿/**没有父的幽灵照样判红**/"
          f"仍在 `plan_domain` 与 `--json.open`/`--next` 里 ⇒ **判红 ≠ 从面板消失**；"
          f"把 `known` 改回读视图 ⇒ ① 必红 ✗）"
          f" + **triage 假证据 14 条**（第六次复核 #2：**存在性 + 祖先性两条硬拒**/"
          f"被拒时**一个字节都不写**/真祖先不误伤/起点之后的 triage-applied 假指针**读侧判红**/"
          f"⭐ 口径收紧后：**起点之前 / 无 `ts` 的假指针也判红**（`ts` 不再分桶 ⇒ "
          f"「存量 / 从宽」那套措辞已随豁免一起删掉 ✓）；"
          f"摘掉 `closure_pointer_problem` 或 triage 分支的 `advanced_ts` ⇒ 对应那条必红 ✗）"
          f" + **写入口权威状态 19 条**（第六次复核 #3：来源关账的 reopen **硬拒 + 给出路 + 不写事件**/"
          f"来源关账的父 `--add-child` **硬拒 + 不写 `split`**/"
          f"**不许误拒**：来源 `fix` 的 reopen 与拆解仍 rc=0 且转绿/"
          f"`source_closed` ≡ `effective_state(row, {{}}) in CLOSED_STATES` 逐处置成立/**来源关账 + 无事件** ⇒ 也硬拒且**不许**印「非法迁移」✗）"
          f" + **切版证据闸门 9 条**（`#2`，第七次复核：`--release cut --commit` 原先**自身零闸门** ⇒ "
           f"不存在 / 非祖先的 sha **都 rc=0** 并写入 `release-cut` ✗ ⇒ 现在与入版/收口共用 "
           f"`closure_pointer_problem()`：两条假 sha **都拒 + 零写入**、正对照真祖先仍 rc=0、"
           f"事件带 `subject`/`patch_id`、缺 `--ci`/`--commit` 仍是用法错 ✓）"
           f" + **发版节点接缝 10 条**（`#2`，第八次复核：⭐ **`--release open` 之后 `check_plan` 仍 rc=0**（**改前 rc=1 + 幽灵条目** ✗）· 节点 id 进 `known` · **不许放宽**：手写无 `release-open` 的 `release-add` 照样判红 + 点名 · 正对照 `add`/`check`/`cut` 三条后续步骤仍 rc=0） + **分支栅栏 4 条**（`#4`：未合回 ⇒ 判红 + **逐条** `agent-worktree.sh land` + "
           f"**明说没有 `--force` 旁路** + ⭐ **`--force` 也 rc=1**（偷加旁路当场判红 ✗）+ "
           f"正对照合回后 rc=0）"
           f" + **id 长度 + 环境编码 9 条**（`#5`：3000 字符 id 在 `--schedule`/`--submit`/"
           f"`--add-child` 三个入口**都拒且零写入**、上限 64 是**数字**（64 过 / 65 拒）、"
           f"`locale=US-ASCII` ⇒ `UsageError`、真 CLI 在 `LC_ALL=C PYTHONUTF8=0` 下**没有栈**"
           f"且 rc ∈ {{0,2}}（**不许是 1 + 栈** ✗））"
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
    parser.add_argument("--repoint", metavar="ID",
                        help="**回填**一条 done 条目的收口指针（省略 `--commit` ⇒ 三关自动找 "
                             "rebase 孪生；只追加事件 ⇒ 可逐条回滚）")
    parser.add_argument("--repoint-all", action="store_true",
                        help="把存量**非祖先**收口指针**批量回填到 0**（找不到孪生的如实列出、不硬填）")
    parser.add_argument("--dry-run", action="store_true",
                        help="只打印计划、**不写事件**（配 `--repoint` / `--repoint-all`）")
    parser.add_argument("--limit", type=int, default=0, metavar="N",
                        help="`--repoint-all` 每批最多处理 N 个 **sha**（0 = 不限；整 sha 一起写）")
    parser.add_argument("--release", metavar="ACTION",
                        help="发版节点：open/add/block/drop/check/cut（栅栏 + 克制 + 防无限 bug 分支）"
                             "。⚠ `--release check` 的「**分支必须合回**」栅栏**没有 `--force` "
                             "旁路**（`--force` 只越过条目数与 worktree 提醒两条克制规则）—— "
                             "真仓库当前有 15 条 agent/* 未合回 ⇒ 它**永远过不了**，正确做法 = "
                             "先 `scripts/agent-worktree.sh land <id>` 合回（或由人删掉废弃分支）✓")
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
    # **stdio 钉成 UTF-8**（`#5a` 的第一半）：非 UTF-8 环境下 `print` 中文会抛
    # `UnicodeEncodeError`（栈 + rc=1 ✗）；`errors="backslashreplace"` ⇒ **打印永不抛**
    # （报错路径上不能再抛一次 —— 否则人话还没说完就变成栈 ✗）。
    for _stream in (sys.stdout, sys.stderr):
        try:
            _stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass
    try:
        _guard_root()
        _guard_encoding()
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
        if args.repoint:
            return cmd_repoint(args.repoint, args.commit or "", args.reason or "", args.owner,
                               args.dry_run)
        if args.repoint_all:
            return cmd_repoint_all(args.reason or "", args.owner, args.dry_run, args.limit)
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
    except UnicodeError as err:
        # **兜底**（`#5a`）：任何残留的编码错都算**环境/契约错**（rc=2 + 人话），
        # **不许丢栈** ✗（实测改前这里是一条 `UnicodeDecodeError` 栈 + rc=1 ✗）。
        print(f"error: 编码/环境错误（{err.__class__.__name__}）：{err} ⇒ 本脚本的数据是 UTF-8；"
              f"用 `PYTHONUTF8=1` 或 `LC_ALL=C.UTF-8` 再跑 ✓", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
