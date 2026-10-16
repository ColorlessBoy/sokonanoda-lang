#!/usr/bin/env python3
"""**簿记预算**：别让"关于工作的账"吃掉注意力（ratchet，照 `docs-budget.json` 的先例 ✓）。

## 为什么要有这个文件（2026-10-15 · `SUB-orchestrator-attention-structure` 【5】/【5'】）

用户实测（本会话，2026-10-14 起 246 笔提交）：

    ① **队列/CI/基建类提交 = 137/246 = 56%** ✗
    ② 改动行数：**生成物/台账 76,280** · 工具 13,690 · **产品 4,572** · 其它文档 3,942
       ⇒ **生成物/台账 : 产品 ≈ 17 : 1** ✗
    ③ 改动最频繁的前 4 个文件**全是队列簿记**（合计 259 次触达）✗

⇒ 病根不是"不努力"，是**注意力结构**：账本身不断出问题（`ux-queue.py` 54 次）
⇒ 自指的循环 ✗。⇒ 【5'】要的就是"**给簿记设预算并判红**"✓。

## 判据（**两档**，照 `scripts/target-hygiene.py` 的先例 ✓）

  B1 **生成物/台账 : 产品** —— 告警 `RATIO_WARN` · 红线 `RATIO_MAX`
  B2 **簿记类提交占比**   —— 告警 `SHARE_WARN` · 红线 `SHARE_MAX`
  超告警线 ⇒ 出声（`::warning::`）**不判红** ✓；超红线 ⇒ **判红**（exit 1）✗

## ⭐ 2026-10-16：两处口径**同源** + 「**红必须有出路**」（洞 `BUDGET-WARN-VS-GATE-RED-INCONSISTENT`）

**改前实测的病（两个，别混成一个）**：

  ① **step 标签是第二份真相** ✗：`scripts/ci-local.sh:226` 把两档**写死在标签里**
     （`run "gates：簿记预算（告警 17:1 / 57% · 红线 19:1 / 60%）" …`）—— 那是**种子**，
     而判据读的是 `docs/attention-budget-ratchet.json` 的**本轮 caps**（已收紧到
     `16:1 / 55%` · `18:1 / 58%`）⇒ pre-push 印的 caps 与判据用的 caps **不是一组** ✗；
     读数落在告警档时，人看到的是「上一轮的 caps + `exit=1`」⇒ **误诊成「告警被当成红」** ✗。
     ⇒ 修：`--label` **从 ratchet 状态读**（本文件 = 那份状态的**唯一读者** ✓），
     接缝判据（`seam_problems`）机械咬住"标签又被写死" ✗。
  ② **一条没有出路的红** ✗：`judge_round()`（ratchet 空转）**无条件**判红，而它的出路
     （`--open-round`）要求读数**严格低于上一轮** —— 读数比上一轮**差**时那条路**物理上走不通**
     ⇒ 空转红盖住了「告警档只告警不拦」⇒ 拦住推送 ✗。
     ⇒ 修：**有出路才判红**（照兄弟守卫 `scripts/threshold_ratchet.py` 的 `judge_idle()`：
     「再开轮也降不下去 ⇒ 判红会是一条**没有出路的红** ✗」——同一条纪律 ✓）；
     没出路 ⇒ **告警**（看得见 + 写明出路 = 落产品）✓。
     ⚠ **两档 caps 一个都没动** ✓（红线只紧不松：改前改后都读同一份 `caps` ✓）。

## ⭐ 2026-10-16：从「防止变坏」改成「**逼它变好**」（真 ratchet · 用户 2026-10-15【②】）

⚠ **改前的病**（用户实测，公平且准确）：`RATIO_MAX = 19.0` 而实测 **10.14 : 1** ·
`SHARE_MAX = 0.60` 而实测 **56.6%** ⇒ **两档都在现状上方 ⇒ 永远绿 ⇒ 装饰** ✗。
而本文件自己的注释写着：第一版把红线贴着实测取 17.0 ⇒ 落地当刻判红 ⇒ **抬到 19.0** ✗
—— 用户 2026-09-26 拍过板：「**不许靠抬上限达标 —— 上限方向只能是收紧**」✗。

⇒ 纠正 = **排程收紧**（`RATCHET_STEP_*`，**每轮降一档**）+ 红线定义改成
「**本轮没比上轮低 ⇒ 判红**」（`--open-round` 的闸门）✗→✓。

  · **状态**（**读数可查** ✓）= `docs/attention-budget-ratchet.json`（**tracked** ⇒ 逐轮读数进 git）：
    `round` · `head`（本轮基点 sha）· `caps`（本轮生效的两档）· `history[]`（逐轮读数 + 用户面读数）
  · **收紧是排程的**（`tighten()` 只看上一轮的 caps，**不看实测** ⇒ 不会"照现状设" ✗）；
    **单调不增**、且**永不高于种子**（种子 = 落地时那两档 ⇒ 本次改动**一个阈值都没放松** ✓）
  · **谁开轮**（**不是纪律** ✓）：`state.head` 之后落了超过 `ROUND_MAX_COMMITS` 笔还没开轮
    ⇒ **默认跑就判红**（"ratchet 空转"）✗ —— 想消红只能 `--open-round`，
    而 `--open-round` 要求**读数真的降了** ⇒ **红有出路**（多落产品）✓，不是假红生成器 ✓
    ⚠ **但"出路"必须是**那条红**当场**走得通的**（2026-10-16 修）：读数比上一轮**差**时
    `--open-round` **物理上走不通** ⇒ 那条红**没有出路** ⇒ 降为**告警**（照兄弟守卫
    `threshold_ratchet.judge_idle()` 的 `converged()` 先例 ✓）；读数**真的降了**而还空转
    ⇒ **照样判红**（出路 = 一条命令，就在手边 ✓）。
  · **③ 产品面最低配额**：`--open-round` **必须**带 `--user-reading`
    （一条**真编辑器**命令 + `⇒` 实测读数 ⇒ **形状可判** ✓）；缺 ⇒ rc=1，本轮**不算收口** ✗
  · **④ 新守卫必须自带咬住证明**：`--open-round` 调 `scripts/check-guard-bite.py --check --since <基点>`
    —— `基点..HEAD` 里**新增/修改的守卫**每一个都要有**可复跑的咬住证明**
    ⇒ 缺 ⇒ rc=1，本轮**不算收口** ✗（口径与枚举**只在那边一处定义** ✓）

⚠ **⑤ 的机械边界（如实说）**：`--user-reading` 能机械判的只有"**形状**"——
**有**一条编辑器侧命令 + **有**一条实测读数。至于它是不是**用户的原始现场**、
那条读数是不是真的，**脚本判不了** ⇒ 只能靠**外部验收方**（接口 agent 复跑）✓。

## ⚠ 为什么当初是两档（**实测**，不是设想）—— 这条理由仍然成立，ratchet 是它的出路

第一版把红线贴着实测值取（17.0）⇒ **落地当刻就判红** ✗（`main` 在切出 commit 之后
又落了 2 笔 ⇒ 17.13）—— 而那条红**没有出路**：本窗生成物的 **97.9% 是 `docs/e2e/logs/**`**，
而 e2e 台账是 `AGENTS.md` **明文要求每批记一条**的动作 ⇒ 拿"今天的实测值"当红线
= **惩罚被规定的行为** ✗。⇒ **"红必须有出路"是对的**，错的只是当时的解法（抬上限）✗。
⭐ ratchet 给的是**另一条出路**：红 = "这一轮没变好"，而**多落产品就能变好** ✓
（实测：本窗 302 笔里加 1 笔非簿记提交 ⇒ 56.6% → 56.4% ✓，**出路是真的** ✓）。

⚠ **窗口是"固定锚点 → HEAD"**（不是滑动窗口 ✗）：滑窗会在"一笔大产品提交老化出窗"时
把比值**推高** ⇒ **假红** ✗。固定锚点下三边都只增 ⇒ 比值只因**新提交**变化 ✓
（锚点见 `WINDOW_SINCE`；批次边界上**手改那一行**往前推，评审可见 ✓）。

⚠ **浅克隆 ⇒ 算不了**：`exit 3` + 人话（**绝不静默判过** ✗）—— 与仓库其它门禁同口径 ✓
（CI 里挂在 `queue-lint`，它本来就 `fetch-depth: 0` ✓）。

## 退出码

  0 = 在红线内（**含只超告警线** ✓） / 1 = 超红线（判红） / 2 = 用法错 /
  3 = 环境错（浅克隆 / 状态文件缺失之类，**不假装判过** ✗）
  ⚠ **空转那条红**：**有出路**（`--open-round` 当场走得通）⇒ 1 ✓；**没出路** ⇒ 只告警 ⇒ 0 ✓。

## 用法

  python3 scripts/check-attention-budget.py             # 告警/判红/绿 + 读数
  python3 scripts/check-attention-budget.py --json
  python3 scripts/check-attention-budget.py --label     # **只印 step 标签**（caps 从状态读 ⇒ 与判据同源）
  python3 scripts/check-attention-budget.py --seam      # 接缝：step 标签**真的读 ratchet**（不是写死的字面量）
  python3 scripts/check-attention-budget.py --seam --source-rev <旧 rev>   # 咬住证明：打在**历史版本**上
  python3 scripts/check-attention-budget.py --until <旧 rev> --head-rev <旧 rev> --state <旧状态>  # 咬住证明：**三根钉子**（窗口 · tip · 状态）⇒ 输入与今天无关
  python3 scripts/check-attention-budget.py --recent 50 # 另给一个近窗读数（**只报不判**）
  python3 scripts/check-attention-budget.py --init      # 只在落地那一次：写第 0 轮基线
  python3 scripts/check-attention-budget.py --open-round \\
      --user-reading 'code ⇒ 打开 x.sokonanoda ⇒ Infoview 显示 def 卡片第二行 := 42'
  python3 scripts/check-attention-budget.py --selftest
"""
from __future__ import annotations

import argparse
import collections
import datetime
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ── **两档**（照 `scripts/target-hygiene.py` 的先例：告警线 / 红线 ✓）────────────
#   **告警线**（超 ⇒ 出声，**不判红** ✓）· **红线**（超 ⇒ 判红 ✗）
#
# ⚠ **下面这四个 = 种子（round 0），不是"永远生效的红线"** ✗：
#   2026-10-16 起真正生效的两档在 `docs/attention-budget-ratchet.json` 的 `caps`，
#   由 `tighten()` **每轮降一档**（`--open-round`）。种子的作用是**上界** ——
#   `tighten()` 单调不增 ⇒ **任何一轮的 caps 都不会高于种子** ✓
#   ⇒ 本次改动**一个阈值都没放松** ✓（用户 2026-09-26：「不许靠抬上限达标」✗）。
#
# ⚠ **为什么当初是两档，而不是"上限 = 今天的实测值"** —— 实测，不是设想：
#   第一版把 `RATIO_MAX` 贴着实测 16.98 取 17.0，结果**落地当刻就判红** ✗
#   （main 在我切出 commit **之后**又落了 2 笔 ⇒ 17.13）—— 而**这条红没有出路**：
#   本窗 97.9% 的生成物是 `docs/e2e/logs/**`，而 e2e 台账是 `AGENTS.md`
#   **明文要求每批记一条**的动作 ⇒ 拿"今天的实测值"当红线 = **惩罚被规定的行为** ✗
#   ⇒ 会变成一台**假红生成器**。
# ⇒ 告警线 = **今天的状态**（要的是"看得见"✓）；红线 = **本会话实测最差点之上**
#   （要的是"再坏就拦"✓）。
#   · 实测本会话最差 = `--until c82097fe`（那笔 **53,595 行**的 e2e 归档）⇒ **19.32 : 1**
#   · 实测今天（main `2f67da90`）= **17.13 : 1** · 簿记提交 **150/263 = 57.0%**
RATIO_WARN = 17.0
RATIO_MAX = 19.0
SHARE_WARN = 0.57
SHARE_MAX = 0.60
# 兼容旧名（`--ratio-max` / `--share-max` 的默认值就是红线 ✓）
BOOKKEEPING_SHARE_MAX = SHARE_MAX
# 窗口锚点：用户给读数的那一天（**固定** ⇒ 三边只增 ⇒ 没有"老化出窗"的假红 ✓）。
WINDOW_SINCE = "2026-10-14T00:00:00+08:00"

# ── ⭐ ratchet（2026-10-16 · 用户 2026-10-15【②】）────────────────────────────
# **状态**（读数可查 ✓）：逐轮的 caps + 读数 + 用户面读数。tracked ⇒ 进 git ⇒ 评审可见 ✓。
RATCHET_STATE = ROOT / "docs" / "attention-budget-ratchet.json"
# **排程收紧**：每轮降一档。⚠ 这两个数字是**用户 2026-10-15 原话**（"SHARE_MAX 每轮
# −2 个百分点"）✓ —— 不是照实测设的 ✗（照实测设 = 改前那个病 ✗）。
RATCHET_STEP_SHARE = 0.02     # 每轮 −2 个百分点
RATCHET_STEP_RATIO = 1.0      # 每轮 −1.0 : 1
# **收敛目标** = 收紧的**终点**（到了就守住；⚠ 它是"收紧停在哪"，**不是放宽** ✗ ——
# 它**低于**种子，且 caps 单调不增 ⇒ 方向仍然只有收紧 ✓）。
RATCHET_FLOOR_SHARE = 0.40
RATCHET_FLOOR_RATIO = 4.0
# 告警线**跟着红线一起收紧**（保持"两档"的相对位置 ✓）—— 种子那两档的间距就是这两个值。
RATCHET_WARN_GAP_SHARE = 0.03
RATCHET_WARN_GAP_RATIO = 2.0
# ⭐ **"谁开轮"的机械约束**（不是纪律 ✓）：基点之后落了这么多笔还没开轮 ⇒
# **默认跑就判红**（ratchet 空转 ✗）。⇒ 想消红只能 `--open-round`，而它要求**读数真的降了** ✓。
ROUND_MAX_COMMITS = 40

# ── 分桶（**一处定义** ✓；与用户的读数口径逐条对齐）────────────────────────
# 生成物/台账 = 可**机器重生成**或**纯簿记**的东西：e2e 产物（含每轮日志）· 各本
# `ledger.jsonl` · 队列三件 · perf 台账。
# ⚠ 口径与用户 2026-10-15 的实测对齐（他数出 gen=76,280 · tools=13,690 · product=4,572
#   · docs=3,942 —— 本文件逐桶复算，差 0 行 ✓；`docs/gaps/ledger.jsonl` 算 gen ✓）。
_GEN_PREFIXES = ("docs/e2e/", "docs/perf/", "docs/ux-queue-")
_GEN_SUFFIXES = ("ledger.jsonl",)
_TOOL_PREFIXES = ("scripts/", ".github/", "editor/")
_PRODUCT_PREFIXES = ("crates/", "courses/")

# 簿记类提交（标题命中）—— 与用户 ① 的判据**同一条正则** ✓（他把 137/246 数成 56%）。
_BOOKKEEPING_SUBJECT = re.compile(
    r"queue|ci|workflow|ledger|.*-lint|checker|guard|栅栏|复核")

# ③ **"在真编辑器里量的"** 的可判形状 = 命令里出现**编辑器侧**的东西。
# ⚠ 这只是**形状**判据（见 docstring 的 ⑤ 边界说明）：它拦得住"随手写一句散文" ✗，
#   拦不住"编一条假读数" ✗ —— 后者只有**外部验收方复跑**能拦 ✓。
_EDITOR_TOKENS = ("vscode-e2e", "npx vscode-test", "test-extension-host", "SOKO_E2E_GREP",
                  "editor/vscode", "code ", "vscode", "Infoview", "infoview",
                  "site-screenshot", "--browser")
# 读数的形状 = "命令 ⇒ 观测"（`⇒` / `->` / `=>` 任一；仓库里读数一律这么写 ✓）
_READING_SEP = re.compile(r"⇒|->|=>")

# ── **接缝**（A 层产出 caps、B 层消费 ⇒ 必须有 A∖B 对账；照 `audit-wire-fields.py` /
#    `threshold_ratchet.seam_problems()` 的先例 ✓）──────────────────────────────
# 病（2026-10-16 实测，洞 `BUDGET-WARN-VS-GATE-RED-INCONSISTENT`）：**caps 被印了两遍** ——
# 判据读 `docs/attention-budget-ratchet.json`（对 ✓），而 gate 的 **step 标签**把种子常量
# 写死在 `scripts/ci-local.sh:226`（`告警 17:1 / 57% · 红线 19:1 / 60%`）✗ ⇒
# 状态收紧之后标签**不动** ⇒ pre-push 印的 caps 与判据用的 caps **不是一组** ✗
# （读数在告警档时人看到「上一轮的 caps + exit=1」⇒ 误诊成"告警被当成红" ✗）。
#
# 判据 = **两个标记**（机械、可复跑 ✓）：
#   ① 消费点里有 `check-attention-budget.py --label`（**真的从状态读标签** ✓）；
#   ② 消费点里**没有** caps 的字面量（形状 = 改前那一行的 `告警 17:1` / `红线 19:1` ✗）。
# `--source-rev <旧 rev>` = 从**历史版本**读源码 ⇒ 用来做**咬住证明**：改前的 `ci-local.sh`
# **没有** `--label` 而**有**字面量 ⇒ 本判据在那份**已存在的**坏输入上**必须报红** ✓。
_CAPS_LITERAL = re.compile(r"(告警|红线)\s*[0-9]+(\.[0-9]+)?\s*(:\s*1|%)")
SEAM: tuple[dict, ...] = (
    {"rel": "scripts/ci-local.sh",
     "marker": "check-attention-budget.py --label",
     "why": "pre-push / `ci-local.sh --fast` 的 **step 标签**必须从 ratchet 状态读 "
            "（改前写死种子常量 ⇒ 印的是**上一轮的 caps** ✗）"},
    {"rel": "scripts/soko",
     "marker": "check-attention-budget.py",
     "why": "贡献者 gate（`--fast` 也跑）必须**真的调判据**；caps 一律由本文件的报告印 ⇒ 同源 ✓"},
    {"rel": ".github/workflows/ci.yml",
     "marker": "check-attention-budget.py",
     "why": "CI 的 `queue-lint` 必须**真的调判据**；step 名里不许再写死 caps ✗"},
)


def caps_label(caps: dict, round_no: int | None = None) -> str:
    """**本轮 caps 的 step 标签**（`--label` 用它 ⇒ 标签与判据**同源** ✓，唯一读者在本文件）。

    形状照改前那一行（`gates：簿记预算（告警 … · 红线 …）` ✓），只是数字**从状态来** ✓。
    """
    def num(v: float) -> str:
        return f"{v:g}"

    out = (f"簿记预算（告警 {num(caps['ratio_warn'])}:1 / "
           f"{caps['share_warn'] * 100:.0f}% · 红线 {num(caps['ratio_max'])}:1 / "
           f"{caps['share_max'] * 100:.0f}%")
    if round_no is not None:
        out += f" · 第 {round_no} 轮"
    return out + "）"


def seam_problems(root: Path | None = None, source_rev: str | None = None) -> list[str]:
    """**每个消费点必须真的读 ratchet**（否则 caps 是第二份真相：状态收紧、标签不动 ✗）。

    ⚠ `source_rev` 指到**历史版本**时，`git show` 失败 ⇒ **判红**（**不假装判过** ✗）。
    """
    root = root or ROOT
    red: list[str] = []
    for spec in SEAM:
        rel, marker = spec["rel"], spec["marker"]
        if source_rev:
            proc = subprocess.run(["git", "show", f"{source_rev}:{rel}"], cwd=str(root),
                                  capture_output=True, text=True, encoding="utf-8",
                                  errors="replace")
            if proc.returncode != 0:
                red.append(f"`git show {source_rev}:{rel}` rc={proc.returncode} ⇒ 量不到 ⇒ "
                           f"**不假装判过** ✗")
                continue
            src = proc.stdout
        else:
            path = root / rel
            if not path.exists():
                red.append(f"消费点 `{rel}` 不存在 ⇒ **量不到 ≠ 绿** ✗")
                continue
            src = path.read_text(encoding="utf-8")
        if marker not in src:
            red.append(f"消费点 `{rel}` 里找不到 `{marker}` ⇒ **它没在读 ratchet** ✗"
                       f"（{spec['why']}）")
        hits = [ln.strip()[:88] for ln in src.splitlines() if _CAPS_LITERAL.search(ln)]
        if hits:
            red.append(f"消费点 `{rel}` 里出现了 **caps 的字面量** ✗（caps 只许住在 "
                       f"`docs/attention-budget-ratchet.json`；标签用 "
                       f"`check-attention-budget.py --label` ✓）—— {hits[:2]}")
    return red


def seed_caps() -> dict:
    """第 0 轮的两档 = **落地时那两档**（**上界**：`tighten()` 永不越过它 ✓）。"""
    return {"ratio_warn": RATIO_WARN, "ratio_max": RATIO_MAX,
            "share_warn": SHARE_WARN, "share_max": SHARE_MAX}


def tighten(caps: dict) -> dict:
    """**排程收紧一档**。**纯函数** ✓（selftest 直接喂合成 caps）。

    ⚠ **只看上一轮的 caps，不看实测** ✓ —— 这是"不许照现状设"✗→✓ 的机制本身。
    ⚠ **单调不增**：新 caps 的每一档都 `<=` 旧的对应档（floor 也只**停住**收紧，
      不会把已经收下去的抬回来 ✓）⇒ **方向只有收紧** ✓。
    """
    ratio_max = max(RATCHET_FLOOR_RATIO, round(caps["ratio_max"] - RATCHET_STEP_RATIO, 4))
    share_max = max(RATCHET_FLOOR_SHARE, round(caps["share_max"] - RATCHET_STEP_SHARE, 4))
    return {
        "ratio_max": ratio_max,
        "ratio_warn": max(0.0, round(ratio_max - RATCHET_WARN_GAP_RATIO, 4)),
        "share_max": share_max,
        "share_warn": max(0.0, round(share_max - RATCHET_WARN_GAP_SHARE, 4)),
    }


# ── 状态读写（**唯一一处** ✓）──────────────────────────────────────────────
def state_load(path: Path) -> tuple[dict | None, str | None]:
    """读 ratchet 状态。返回 (状态, 错误)。**缺失/坏 ⇒ 错误**（**绝不静默判过** ✗）。"""
    if not path.exists():
        return None, (f"ratchet 状态不存在：`{path}` ⇒ **不假装判过** ✗。"
                      f"落地那一次用 `--init` 写第 0 轮基线 ✓")
    try:
        st = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as err:
        return None, f"ratchet 状态读不了：`{path}`：{err}"
    for key in ("round", "head", "caps", "history"):
        if key not in st:
            return None, f"ratchet 状态缺字段 `{key}`：`{path}`"
    return st, None


def state_save(path: Path, st: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(st, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def _reading_of(agg: dict) -> dict:
    """读数里**进状态**的那几项（别把 top_files 那种会漂的东西写进 git ✓）。"""
    return {"ratio": agg["ratio"], "share": agg["bookkeeping_share"],
            "commits": agg["commits"], "bookkeeping_commits": agg["bookkeeping_commits"],
            "product_lines": agg["lines"].get("product", 0),
            "gen_lines": agg["lines"].get("gen", 0)}


# ── ⭐ 纯函数：**开一轮**（判红 / 收紧）—— selftest 直接驱动它 ✓ ──────────────
def advance_round(st: dict, reading: dict, user_reading: str | None,
                  guards_ok: bool, guards_detail: str = "",
                  now: str | None = None, head: str | None = None,
                  user_case: str | None = None,
                  user_case_ok: bool = True) -> tuple[dict, list[str]]:
    """**纯函数**：把状态推进一轮 ⇒ (新状态, 判红)。红非空 ⇒ 状态**不动** ✗。

    五道闸（顺序 = 判红信息的顺序）：
      ① **本轮没比上轮低 ⇒ 判红**（用户 2026-10-15【②】的原话）✗→✓
      ② **③ 产品面最低配额**：必须带一条**编辑器侧**读数（形状可判 ✓）
      ③ **⑤ 收口判据绑用户现场**：`--user-case` 必须点到一个**存在的**仓库路径 ✓
         （⚠ 机械上只能判"绑没绑到一个真东西"；它**是不是**用户的原始现场，
          **脚本判不了** ⇒ 外部验收方 ✓）
      ④ **④ 新守卫必须自带咬住证明**：`guards_ok`（由 `check-guard-bite.py` 判 ✓）
      ⑤ 过了才 `tighten()` 一档（**排程收紧** ✓）
    """
    red: list[str] = []
    hist = st.get("history") or []
    if hist:
        prev = hist[-1]["reading"]
        if not (reading["ratio"] < prev["ratio"]):
            red.append(f"**① 本轮没比上轮低** ✗：B1 比值 {reading['ratio']:.2f} : 1 "
                       f"vs 上轮（第 {hist[-1]['round']} 轮）{prev['ratio']:.2f} : 1 "
                       f"⇒ **ratchet 要求严格下降**（'从防止变坏改成逼它变好' ✓）。"
                       f"出路：**多落产品行**（`crates/` / `courses/`）把它摊薄 ✓")
        if not (reading["share"] < prev["share"]):
            red.append(f"**① 本轮没比上轮低** ✗：B2 簿记提交占比 {reading['share'] * 100:.1f}% "
                       f"vs 上轮 {prev['share'] * 100:.1f}% ⇒ **严格下降**才开得了下一轮。"
                       f"出路：**多落非簿记提交**（标题不含 queue/ci/ledger/lint/guard ✓）")
    if not user_reading:
        red.append("**③ 产品面最低配额** ✗：本轮没有**用户面判据** ⇒ 不算收口。"
                   "必须带 `--user-reading '<真编辑器命令> ⇒ <实测读数>'` ✓")
    elif not _READING_SEP.search(user_reading):
        red.append(f"**③ 读数的形状不对** ✗：`{user_reading[:60]}` 里没有 `⇒`/`->`/`=>` "
                   f"⇒ 读数是「命令 ⇒ 观测」，不是散文 ✗")
    elif not any(tok in user_reading for tok in _EDITOR_TOKENS):
        red.append(f"**③ 读数不是编辑器侧的** ✗：`{user_reading[:60]}` 里没有 "
                   f"{list(_EDITOR_TOKENS)} 任一 ⇒ ③ 要的是「**打开编辑器 ⇒ 看到什么**」✓")
    if not user_case:
        red.append("**⑤ 收口判据要绑用户现场** ✗：必须带 `--user-case <用户写的那段代码 / "
                   "他做的那个动作所在的仓库路径>`（⑤：收口判据 = **用户的原始现场**，"
                   "不是'我这条测试绿了' ✗）")
    elif not user_case_ok:
        red.append(f"**⑤ 用户现场** `{user_case}` **在仓库里不存在** ✗ ⇒ 收口判据绑到了一个"
                   f"**假现场**（⑤ 要的是他写的那段代码 / 他做的那个动作 ✓）")
    if not guards_ok:
        red.append(f"**④ 新守卫没有咬住证明** ✗ ⇒ 本轮不算收口：{guards_detail or '（无细节）'}")
    if red:
        return st, red

    new_caps = tighten(st["caps"])
    round_no = int(st.get("round", 0)) + 1
    entry = {"round": round_no, "head": head, "at": now, "caps": new_caps,
             "reading": reading, "user_reading": user_reading, "user_case": user_case}
    new = dict(st)
    new["round"] = round_no
    new["caps"] = new_caps
    new["head"] = head
    new["at"] = now
    new["history"] = list(hist) + [entry]
    return new, []


def open_round_would_pass(state: dict, reading: dict | None) -> bool:
    """**纯函数**：`--open-round` **现在能不能成**（= 那条空转红**有没有出路** ✓）。

    判据 = `advance_round()` 的 ① 闸门（**唯一一条你没法"补"的闸门** —— 另外三条
    （`--user-reading` / `--user-case` / 守卫咬住证明）都是操作方**当场能提供**的输入 ✓）：
    读数必须**严格低于**上一轮（B1 与 B2 都要，与 `advance_round` 逐字同口径 ✓）。
    `history` 为空 ⇒ 没有可比的一轮 ⇒ 能开（`advance_round` 在空 history 下不做这条比较 ✓）。
    """
    hist = state.get("history") or []
    if not hist or reading is None:
        return True
    prev = hist[-1].get("reading") or {}
    return (reading["ratio"] < prev.get("ratio", float("inf"))
            and reading["share"] < prev.get("share", float("inf")))


def judge_round(state: dict, n_since: int, reading: dict | None = None,
                round_max: int = ROUND_MAX_COMMITS) -> tuple[list[str], list[str]]:
    """**纯函数**：**开轮纪律**的机械面 —— 基点之后落太多笔还没开轮 ⇒ `(判红, 告警)`。

    为什么必须有这一条：`--open-round` 如果**没人调**，ratchet 就停在原地
    ⇒ 又变成装饰 ✗（"不跑的守卫等于没有"，`docs/CI-FAILURES.md:448` ✓）。
    ⇒ 把它做成**默认跑**的约束 ✓（消红的唯一出路是 `--open-round`，而它要求读数真的降了 ✓）。

    ⚠ **但"红必须有出路"**（2026-10-16 修，洞 `BUDGET-WARN-VS-GATE-RED-INCONSISTENT`）：
    读数比上一轮**差**时 `--open-round` **物理上走不通** ⇒ 无条件判红 = 一条**没有出路的红**
    ⇒ 它盖住「告警档只告警不拦」（实测：读数 `57.1%` 在告警档、`exit=1` ⇒ 推送被拦 ✗）。
    ⇒ **有出路才判红**（照兄弟守卫 `scripts/threshold_ratchet.py:judge_idle()` 的
    `converged()` 先例 ✓）；没出路 ⇒ **告警**（看得见 + 写明出路 ✓）。
    ⚠ **两档 caps 一个都没动** ✓；空转**有出路**时照样判红 ⇒ 守卫没变装饰 ✓。
    """
    if n_since <= round_max:
        return [], []
    head = str(state.get("head"))[:12]
    common = (f"**ratchet 空转**：第 {state.get('round')} 轮的基点 `{head}` 之后已经落了 "
              f"**{n_since} 笔**提交（> `ROUND_MAX_COMMITS = {round_max}`）还没开轮 "
              f"⇒ 阈值停在原地 = **装饰** ✗")
    if open_round_would_pass(state, reading):
        return [f"{common}。出路：`--open-round --user-reading '…'` ✓"
                f"（读数已经比上轮低 ⇒ **那条路当场走得通** ✓ ⇒ 这条红有出路 ✓）"], []
    return [], [f"{common} —— 但**本轮开不了轮**（读数没比上轮低 ⇒ `--open-round` 的 ① 闸门"
                f"过不去）⇒ 那会是一条**没有出路的红** ✗ ⇒ **只告警、不拦** ✓。"
                f"出路：**多落产品行**（`crates/` / `courses/`）把读数压下去 ⇒ 再 `--open-round` ✓"]


def bucket(path: str) -> str:
    """改动路径 → 区域。**纯函数** ✓（selftest 直接喂合成路径）。"""
    if path.startswith(_GEN_PREFIXES) or path.endswith(_GEN_SUFFIXES):
        return "gen"
    if path.startswith(_TOOL_PREFIXES):
        return "tools"
    if path.startswith(_PRODUCT_PREFIXES):
        return "product"
    return "docs"


def collect(since: str, until: str | None = None, cwd: Path = ROOT) -> tuple[list, str | None]:
    """`git log --numstat` ⇒ [(added, deleted, path, subject), …]。**只读** ✓。

    返回 (行, 环境错误)。浅克隆 ⇒ 环境错误（**不静默** ✗）。
    """
    shallow = subprocess.run(["git", "rev-parse", "--is-shallow-repository"],
                             cwd=cwd, capture_output=True, text=True)
    if shallow.returncode != 0:
        return [], f"`git rev-parse --is-shallow-repository` rc={shallow.returncode}"
    if shallow.stdout.strip() == "true":
        return [], ("仓库是浅克隆 ⇒ 这个窗口**算不了**（不是判过 ✗）："
                    "`git fetch --unshallow`，或 CI 里给这个 job `fetch-depth: 0` ✓")
    # ⚠ **键必须是 sha，不是标题** ✗：本窗里有 **7 对同标题提交**（实测 2026-10-15：
    # 255 笔提交只有 248 个不同标题）⇒ 拿标题当键会把它们**合并**、分母少 7 ✗
    # （用户 ① 数的是**提交数** 143/255 ✓）。这条 bug 是自检咬出来的形状之一 ✓。
    argv = ["git", "log", "--numstat", f"--since={since}",
            "--format=@@%H|%s"]
    if until:
        argv.insert(3, until)
    out = subprocess.run(argv, cwd=cwd, capture_output=True, text=True)
    if out.returncode != 0:
        return [], f"`git log` rc={out.returncode}：{out.stderr.strip()[:200]}"
    rows: list = []
    subject = None
    seen_subject = False
    for line in out.stdout.splitlines():
        if line.startswith("@@"):
            # ⭐ **每笔提交都要进分母**（哪怕它只有二进制改动 ⇒ 一行 numstat 都没有 ✗）：
            # 用户 ① 数的是**提交分类**（143/255 = 56.1%），不是"有文本改动的提交" ✗。
            # ⇒ 没行数的提交发一条**哨兵行**（`path is None`）⇒ 进 `subjects`、不进任何桶 ✓。
            if subject is not None and not seen_subject:
                rows.append((0, 0, None, subject))
            subject, seen_subject = line[2:], False        # `sha|标题` ✓
            continue
        if not line.strip():
            continue
        parts = line.split("\t", 2)
        if len(parts) != 3:
            continue
        added, deleted, path = parts
        if added == "-":                      # 二进制：不计行数，但**这笔提交仍算数** ✓
            continue
        rows.append((int(added), int(deleted), path, subject))
        seen_subject = True
    if subject is not None and not seen_subject:
        rows.append((0, 0, None, subject))
    return rows, None


def aggregate(rows: list) -> dict:
    """行 → 读数。**纯函数** ✓。"""
    lines = collections.Counter()
    touches = collections.Counter()
    subjects: dict[str, str] = {}
    for added, deleted, path, subject in rows:
        subjects[subject] = subject           # `sha|标题` ⇒ 同标题不合并 ✓
        if path is None:                      # 哨兵行：只贡献分母 ✓
            continue
        b = bucket(path)
        lines[b] += added + deleted
        touches[b] += 1
    total = sum(lines.values())
    book = [k for k, title in subjects.items() if _BOOKKEEPING_SUBJECT.search(title)]
    product = lines["product"]
    return {
        "lines": dict(lines),
        "touches": dict(touches),
        "total_lines": total,
        "commits": len(subjects),
        "bookkeeping_commits": len(book),
        "bookkeeping_share": (len(book) / len(subjects)) if subjects else 0.0,
        "ratio": (lines["gen"] / product) if product else float("inf"),
        "top_files": _top_files(rows),
    }


def _top_files(rows: list, n: int = 4) -> list:
    per = collections.Counter()
    for added, deleted, path, _ in rows:
        if path is None:
            continue
        per[path] += added + deleted
    return [[p, v] for p, v in per.most_common(n)]


def judge(agg: dict, ratio_max: float, share_max: float,
          ratio_warn: float, share_warn: float) -> tuple[list[str], list[str]]:
    """**纯函数**：读数 → (判红, 告警) ✓（selftest 直接喂合成读数）。

    ⚠ 四个阈值**一律由调用方给**（= 状态里**本轮生效**的两档 ✓）——
    旧签名默认读模块常量 ⇒ 那正是"照现状设、永远绿"的病根 ✗（改前 `RATIO_MAX`/`SHARE_MAX`
    是默认参数 ⇒ `judge()` 不传就是种子，ratchet 根本进不来 ✗）。**去掉默认值** = 让
    "忘了传 ratchet 的 caps"变成 **TypeError**（响亮 ✗），不是静默退回种子 ✓。
    """
    red: list[str] = []
    warn: list[str] = []
    ratio, share = agg["ratio"], agg["bookkeeping_share"]
    b1 = (f"B1 **生成物/台账 : 产品 = {ratio:.2f} : 1**"
          f"（生成物 {agg['lines'].get('gen', 0)} 行 vs 产品 {agg['lines'].get('product', 0)} 行）")
    if ratio > ratio_max:
        red.append(f"{b1} ⇒ **超过本轮红线 {ratio_max}** ✗ 这一窗的改动又在往「账」上堆 —— "
                   f"出路：**做产品**（生成物里 97.9% 是 `docs/e2e/logs/**`，"
                   f"而它是每批**被规定**要记的 ⇒ 只能靠多落产品把它摊薄 ✓）")
    elif ratio > ratio_warn:
        warn.append(f"{b1} ⇒ 超过**本轮告警线** {ratio_warn}（未超红线 {ratio_max}）· "
                    f"本窗生成物里 **97.9% 是 `docs/e2e/logs/**`**（每批被规定要记）⇒ "
                    f"把它摊薄的唯一办法是**多落产品** ✓")
    b2 = (f"B2 **簿记类提交占比 = {share * 100:.1f}%**"
          f"（{agg['bookkeeping_commits']}/{agg['commits']} 笔在维护「关于工作的账」）")
    if share > share_max:
        red.append(f"{b2} ⇒ **超过本轮红线 {share_max * 100:.0f}%** ✗")
    elif share > share_warn:
        warn.append(f"{b2} ⇒ 超过**本轮告警线 {share_warn * 100:.0f}%**（未超红线 "
                    f"{share_max * 100:.0f}%）")
    return red, warn


def report(agg: dict, caps: dict, state: dict | None = None, n_since: int = 0) -> None:
    lines = agg["lines"]
    print(f"簿记预算（窗口 = {WINDOW_SINCE} → HEAD）：{agg['commits']} 笔提交 · "
          f"{agg['total_lines']} 行改动")
    for key, label in (("gen", "生成物/台账"), ("tools", "工具"),
                       ("product", "产品"), ("docs", "其它文档")):
        n = lines.get(key, 0)
        pct = (100 * n / agg["total_lines"]) if agg["total_lines"] else 0.0
        print(f"  {label:10s} {n:7d} 行  {pct:5.1f}%")
    print(f"  ⇒ **生成物/台账 : 产品 = {agg['ratio']:.2f} : 1**"
          f"（本轮告警 {caps['ratio_warn']} · 本轮红线 {caps['ratio_max']}）")
    print(f"  ⇒ 簿记类提交 {agg['bookkeeping_commits']}/{agg['commits']} = "
          f"{agg['bookkeeping_share'] * 100:.1f}%"
          f"（本轮告警 {caps['share_warn'] * 100:.0f}% · 本轮红线 {caps['share_max'] * 100:.0f}%）")
    print("  触达最多的 4 个文件：" + " · ".join(f"{p} {v}" for p, v in agg["top_files"]))
    if state is not None:
        hist = state.get("history") or []
        print(f"  ⇒ **ratchet 第 {state.get('round')} 轮**"
              f"（基点 `{str(state.get('head'))[:12]}` · 之后落了 {n_since} 笔 · "
              f"开轮上限 {ROUND_MAX_COMMITS}）· 历史 {len(hist)} 轮")
        if n_since > ROUND_MAX_COMMITS:
            # ⭐ **空转红有没有出路**（改前这里只能从 stderr 的一条红里猜 ✗）——
            # 出路 = `--open-round`（它的 ① 闸门 = 读数**严格低于上一轮** ✓）。
            can = open_round_would_pass(state, _reading_of(agg))
            print(f"     ⚠ 空转：{n_since} > {ROUND_MAX_COMMITS} ⇒ "
                  f"`--open-round` {'**走得通** ⇒ 空转判红 ✓' if can else '**走不通**（读数没比上轮低）⇒ 空转**只告警、不拦** ✓'}")
        if hist:
            prev = hist[-1]["reading"]
            print(f"     vs 上一轮（第 {hist[-1]['round']} 轮）：比值 {prev['ratio']:.2f} → "
                  f"{agg['ratio']:.2f}（{'↓ 变好 ✓' if agg['ratio'] < prev['ratio'] else '↑ 没变好 ✗'}）"
                  f" · 占比 {prev['share'] * 100:.1f}% → {agg['bookkeeping_share'] * 100:.1f}%"
                  f"（{'↓ 变好 ✓' if agg['bookkeeping_share'] < prev['share'] else '↑ 没变好 ✗'}）"
                  f"  ← **只报不判**（判在 `--open-round` ✓）")
            if hist[-1].get("user_reading"):
                print(f"     上一轮的用户面读数（③）：{hist[-1]['user_reading']}")
        seed = seed_caps()
        tightened = any(state["caps"][k] < seed[k] for k in seed)
        print(f"     ⇒ 相对种子（{seed['ratio_max']} / {seed['share_max'] * 100:.0f}%）"
              f"：{'**已收紧** ✓' if tightened else '**尚未收紧**（第 0 轮）'}")


def recent(n: int, cwd: Path = ROOT) -> tuple[dict, str | None]:
    """近窗读数（**只报不判** —— 滑窗会老化 ⇒ 拿它判红就是假红 ✗）✓。"""
    out = subprocess.run(["git", "log", "--numstat", f"-{n}",
                          "--format=@@%H|%s"], cwd=cwd, capture_output=True, text=True)
    if out.returncode != 0:
        return {}, f"`git log -{n}` rc={out.returncode}"
    rows, subject = [], ""
    for line in out.stdout.splitlines():
        if line.startswith("@@"):
            subject = line[2:]
            continue
        parts = line.split("\t", 2)
        if len(parts) == 3 and parts[0] != "-":
            rows.append((int(parts[0]), int(parts[1]), parts[2], subject))
    return aggregate(rows), None


# ── git 小工具（ratchet 用）────────────────────────────────────────────────
def _git(argv: list[str], cwd: Path) -> tuple[str, str | None]:
    out = subprocess.run(["git", *argv], cwd=cwd, capture_output=True, text=True)
    if out.returncode != 0:
        return "", f"`git {' '.join(argv)}` rc={out.returncode}：{out.stderr.strip()[:200]}"
    return out.stdout.strip(), None


def head_sha(cwd: Path) -> tuple[str, str | None]:
    return _git(["rev-parse", "HEAD"], cwd)


def commits_since(cwd: Path, since_sha: str | None,
                  tip: str = "HEAD") -> tuple[int, str | None]:
    """`since_sha..tip` 的**提交数**（含 merge/空提交 ✓）。`since_sha` 为空 ⇒ 0。

    ⚠ `tip` 默认 `HEAD`，但**咬住证明必须把它钉死**（`--head-rev <旧 rev>` ✗→✓）：
    这个数**只增不减**，而 ratchet 一开轮（`--open-round`，它的设计就是每 ≤40 笔开一次）
    `state.head` 就跳到**当时**的 `HEAD` ⇒ `since_sha..HEAD` **归零** ⇒ 「ratchet 空转」
    那段文本**整段消失** ⇒ 证明里的 `grep` 不中 ⇒ 命令 `exit 0` ⇒ C2 印「这条守卫咬不住」
    —— 而真相是**输入随时间变了**（洞 `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES`
    的同族，实测：把 `state.head` 换成当时的 `HEAD` ⇒ `--check --all` **rc=1** ✗）。
    ⇒ 证明给 `--head-rev` ⇒ 读数与 `HEAD` **无关** ✓（与 `--until` 钉窗口右端同一条纪律 ✓）。
    """
    if not since_sha:
        return 0, None
    out, err = _git(["rev-list", "--count", f"{since_sha}..{tip}"], cwd)
    if err:
        return 0, err
    return int(out or 0), None


def guard_bite_check(cwd: Path, since_sha: str | None) -> tuple[bool, str]:
    """**④ 的判定入口**：调 `scripts/check-guard-bite.py --check --since <基点>`。

    ⚠ **一处定义**（`AGENTS.md` 硬规则 0(b)）：守卫的枚举与"咬住证明怎么跑"**只在那边** ✓
    ⇒ 这里**只调它、不另抄一份** ✗。⚠ 那个脚本**不在** ⇒ `(False, …)`（**不静默判过** ✗）。
    """
    script = cwd / "scripts" / "check-guard-bite.py"
    if not script.exists():
        return False, (f"`{script.relative_to(cwd) if script.is_relative_to(cwd) else script}` "
                       f"不存在 ⇒ ④ 判不了（**不假装判过** ✗）")
    argv = [sys.executable, str(script), "--check"]
    if since_sha:
        argv += ["--since", since_sha]
    out = subprocess.run(argv, cwd=cwd, capture_output=True, text=True,
                         encoding="utf-8", errors="replace")
    detail = "\n".join(r for r in (out.stdout + out.stderr).splitlines() if r.strip())[-800:]
    return out.returncode == 0, detail


# ── 判据通道自检（反向验证：故意造出该现象 ⇒ 必须判红）──────────────────────
def _fixture_repo(tmp: Path) -> tuple[Path, str | None]:
    """造一个**真 git 夹具仓**（ratchet 的端到端咬住证明用 ✓）。

    形状：**账多产品少**（4.25 : 1）+ **簿记提交占 1/3**；之后每轮落产品
    ⇒ **两档都严格下降** ⇒ **连开两轮**可达，且两轮的 caps 必须**一轮比一轮低**
    （② 的判据 ✓）。

    ⚠ 仓里还要有 `scripts/check-guard-bite.py` + `docs/guard-bite.jsonl`
    （④ 的判定入口 —— `--open-round` 会调它 ✓；**照真仓的形状**，不是打桩 ✗）。
    ⚠ 还要有 `.github/workflows/ci.yml`（2026-10-16 加）：咬住证明的判据 **C4 接线**
    要核"每个 `ci_job` 都在 `ci.yml` 里**真的存在且真的有一跑它**" ⇒ 夹具里没有那份
    工作流时，**真台账**（它声明了 `ci_job: editor` ✓）会以"判不了接线"判红 ✗。
    同一条口径：**照真仓的形状**（拷真文件），不是把 C4 关掉 ✗。
    """
    repo = tmp / "repo"
    repo.mkdir(parents=True, exist_ok=True)
    init = _git(["init", "-q", "-b", "main"], repo)
    if init[1]:
        return repo, init[1]
    _git(["config", "user.email", "fixture@example.invalid"], repo)
    _git(["config", "user.name", "fixture"], repo)

    def commit(path: str, body: str, subject: str) -> None:
        p = repo / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body, encoding="utf-8")
        _git(["add", "-A"], repo)
        _git(["commit", "-q", "-m", subject], repo)

    # ④ 的判定入口：**照真仓的形状**（拷真文件，不打桩 ✗）
    for rel in ("scripts/check-guard-bite.py", "docs/guard-bite.jsonl",
                ".github/workflows/ci.yml"):
        src, dst = ROOT / rel, repo / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        if src.exists():
            dst.write_text(src.read_text(encoding="utf-8"), encoding="utf-8")
    # 第 0 轮基线：账多产品少（4.25 : 1）+ 簿记提交占 1/3
    commit("docs/e2e/logs/a.log", "\n".join(f"l{i}" for i in range(17000)),
           "chore(queue): 记台账")
    commit("crates/front/src/a.rs", "\n".join(f"f{i}" for i in range(1000)), "feat(front): 产品")
    commit("crates/front/src/b.rs", "\n".join(f"g{i}" for i in range(3000)), "feat(front): 再做产品")
    # ⑤ 的**用户现场**：夹具里放一份真存在的文件（`--user-case` 必须点到真东西 ✓）
    commit("playground.sokonanoda", "theorem t : 1 = 1 := by rfl\n", "docs: 用户画布")
    return repo, None


def selftest() -> int:
    bad = 0

    def case(name: str, rows: list, want_red: bool, expect: str = "",
             ratio_max: float = RATIO_MAX, share_max: float = SHARE_MAX,
             ratio_warn: float = RATIO_WARN, share_warn: float = SHARE_WARN) -> None:
        nonlocal bad
        agg = aggregate(rows)
        red, _warn = judge(agg, ratio_max, share_max, ratio_warn, share_warn)
        hit = bool(red) == want_red
        if not hit:
            bad += 1
        print(f"  {'✓' if hit else '✗'} {name}：判红 {len(red)} 条"
              + (f"（{red[0][:64]}）" if red else "")
              + ("" if hit else f"  ← 期望 {'红' if want_red else '绿'}"))
        if expect and not any(expect in r for r in red):
            bad += 1
            print(f"      ✗ 没有期望的那条判红（找 {expect!r}）")

    def gate(name: str, ok: bool, note: str = "") -> None:
        """ratchet 闸门的**逐条**判定（每一条都要有正反两个方向 ✓）。"""
        nonlocal bad
        if not ok:
            bad += 1
        print(f"  {'✓' if ok else '✗'} {name}{('：' + note) if note else ''}")

    _seq = [0]

    def row(path, n, subject="feat(x): 做产品"):
        _seq[0] += 1
        return (n, 0, path, f"sha{_seq[0]:04d}|{subject}")

    print("判据通道自检（合成读数；每组都带**正对照** ✓）：")
    # 正对照：产品为主 ⇒ 必须绿
    case("正对照（产品为主）", [row("crates/front/src/a.rs", 5000),
                          row("docs/e2e/logs/x.log", 500)],
         want_red=False, ratio_max=19.0, ratio_warn=17.0)
    # B1：生成物压倒产品（**17:1 那个形状**）
    case("B1 生成物:产品 超红线", [row("crates/front/src/a.rs", 1000),
                            row("docs/e2e/logs/x.log", 20000)],
         want_red=True, expect="B1", ratio_max=19.0, ratio_warn=17.0)
    # B1 边界：正好等于**红线** ⇒ 不判红（≤ 闭区间 ✓）
    case("B1 正对照：正好等于红线", [row("crates/front/src/a.rs", 1000),
                             row("docs/e2e/logs/x.log", 19000)],
         want_red=False, ratio_max=19.0, ratio_warn=17.0)
    # ⭐ **告警档不许判红**（第一版就是在这里变成假红生成器的 ✗）
    case("B1 正对照：在告警线与红线之间 ⇒ 只告警不判红",
         [row("crates/front/src/a.rs", 1000), row("docs/e2e/logs/x.log", 18000)],
         want_red=False, ratio_max=19.0, ratio_warn=17.0)
    # B1：产品为 0 ⇒ inf（不许除零崩、必须判红 ✓）
    case("B1 正对照：产品为 0 行", [row("docs/e2e/logs/x.log", 10)],
         want_red=True, expect="B1", ratio_max=19.0, ratio_warn=17.0)
    # B2：簿记提交占比超上限
    case("B2 簿记提交占比超上限",
         [row("scripts/ux-queue.py", 10, "chore(queue): 收口 X"),
          row("crates/front/src/a.rs", 10, "feat(front): 真干活")],
         want_red=True, expect="B2", ratio_max=1e9, share_max=0.4, share_warn=0.3)
    # B2 正对照：簿记占比在限内 ⇒ 绿
    case("B2 正对照：簿记占比在限内",
         [row("scripts/ux-queue.py", 10, "chore(queue): 收口 X"),
          row("crates/front/src/a.rs", 10, "feat(front): 真干活"),
          row("crates/front/src/b.rs", 10, "feat(front): 再干活")],
         want_red=False, ratio_max=1e9, share_max=0.4, share_warn=0.3)
    # 分桶本身：每个桶各一格（口径回归 —— 改口径就会在这里红 ✓）
    cases = [("crates/x.rs", "product"), ("courses/x.md", "product"),
             ("scripts/gap.py", "tools"), (".github/workflows/ci.yml", "tools"),
             ("editor/vscode/server.js", "tools"), ("docs/e2e/logs/a.log", "gen"),
             ("docs/e2e/ledger.jsonl", "gen"), ("docs/gaps/ledger.jsonl", "gen"),
             ("docs/perf/ledger.jsonl", "gen"), ("docs/ux-queue-events.jsonl", "gen"),
             ("docs/ONBOARDING.md", "docs"), ("AGENTS.md", "docs")]
    wrong = [(p, bucket(p), want) for p, want in cases if bucket(p) != want]
    if wrong:
        bad += 1
        print(f"  ✗ 分桶口径漂了：{wrong}")
    else:
        print(f"  ✓ 分桶口径：{len(cases)} 格全对（gen/tools/product/docs）")

    # ── ⭐ ratchet（②③④）的正反两向 ──────────────────────────────────────
    print("\nratchet 自检（②③④；每条都有**反向**：故意坏 ⇒ 必须判红 ✓）：")
    # ②-a `tighten()` **单调不增**、且**永不高于种子**
    caps, seed = seed_caps(), seed_caps()
    mono, above = True, []
    for i in range(1, 41):
        nxt = tighten(caps)
        if any(nxt[k] > caps[k] for k in caps):
            mono = False
        caps = nxt
        above += [k for k in seed if caps[k] > seed[k]]
    gate("② `tighten()` 40 档：**单调不增**", mono)
    gate("② 40 档后**没有任何一档高于种子**（不许靠抬上限达标 ✓）", not above,
         f"越界 {above[:4]}" if above else "比值 → "
         f"{caps['ratio_max']} · 占比 → {caps['share_max'] * 100:.0f}%")
    gate("② 收敛目标**低于**种子（floor 是收紧的终点，不是放宽 ✗）",
         RATCHET_FLOOR_RATIO < RATIO_MAX and RATCHET_FLOOR_SHARE < SHARE_MAX,
         f"floor {RATCHET_FLOOR_RATIO} < {RATIO_MAX} · "
         f"{RATCHET_FLOOR_SHARE * 100:.0f}% < {SHARE_MAX * 100:.0f}%")

    # ②-b ⭐ **判据**：连开两轮 ⇒ 第二轮 caps **低于**第一轮（`advance_round` 纯函数驱动）
    st = {"version": 1, "round": 0, "head": "base", "at": "t0", "caps": seed_caps(),
          "history": [{"round": 0, "head": "base", "at": "t0", "caps": seed_caps(),
                       "reading": {"ratio": 10.14, "share": 0.566},
                       "user_reading": "code ⇒ 基线"}]}
    r1 = {"ratio": 9.5, "share": 0.54}
    st1, red1 = advance_round(st, r1, "code ⇒ 第 1 轮：Infoview 显示 def 卡片 := 42",
                              True, now="t1", head="h1", user_case="AGENTS.md")
    gate("② 第 1 轮开得起来（读数降了 + 有用户面读数 + 守卫有证明）", not red1,
         f"caps {st1['caps']['ratio_max']} / {st1['caps']['share_max'] * 100:.0f}%")
    r2 = {"ratio": 8.0, "share": 0.51}
    st2, red2 = advance_round(st1, r2, "code ⇒ 第 2 轮：点 ∈ 能跳转",
                              True, now="t2", head="h2", user_case="AGENTS.md")
    gate("② 第 2 轮也开得起来", not red2,
         f"caps {st2['caps']['ratio_max']} / {st2['caps']['share_max'] * 100:.0f}%")
    gate("⭐⭐ ② 的判据：**第二轮阈值低于第一轮**（两个读数都在状态里 ⇒ 可查 ✓）",
         st2["caps"]["ratio_max"] < st1["caps"]["ratio_max"]
         and st2["caps"]["share_max"] < st1["caps"]["share_max"],
         f"比值 {st1['caps']['ratio_max']} → {st2['caps']['ratio_max']} · "
         f"占比 {st1['caps']['share_max'] * 100:.0f}% → {st2['caps']['share_max'] * 100:.0f}%")

    # ②-c **反向**：读数**没降** ⇒ 必须判红，且状态**不动**（"done 不是免费的" ✓）
    stx, redx = advance_round(st, {"ratio": 10.14, "share": 0.566},
                              "code ⇒ 没变", True, now="t1", head="h1",
                              user_case="AGENTS.md")
    gate("② 反向：读数**没比上轮低** ⇒ 判红", bool(redx) and stx is st,
         (redx[0][:60] if redx else "**没判红** ✗"))
    # ②-d **反向**：读数**变坏** ⇒ 必须判红
    stx2, redx2 = advance_round(st, {"ratio": 12.0, "share": 0.60},
                                "code ⇒ 更坏", True, now="t1", head="h1",
                                user_case="AGENTS.md")
    gate("② 反向：读数**变坏** ⇒ 判红", bool(redx2), (redx2[0][:60] if redx2 else "**没判红** ✗"))

    # ③ **正向**（好形状）/ **反向**（缺、散文、非编辑器）三条
    for name, ur, want in (
            ("③ 正对照：编辑器命令 + `⇒` 读数 ⇒ 放行",
             "code ⇒ 打开 playground.sokonanoda ⇒ Infoview 显示 def 卡片 := 42", False),
            ("③ 反向：**没有**用户面读数 ⇒ 判红", None, True),
            ("③ 反向：**散文**（没有 `⇒` 读数）⇒ 判红", "我打开编辑器看了一下，没问题", True),
            ("③ 反向：有读数但**不是编辑器侧**（纯 CLI）⇒ 判红",
             "python3 scripts/gap.py check ⇒ rc=0", True)):
        _s, _r = advance_round(st, {"ratio": 9.0, "share": 0.50}, ur, True, now="t",
                               head="h", user_case="AGENTS.md")
        gate(name, bool(_r) == want, (_r[0][:56] if _r else "绿 ✓"))

    # ④ **反向**：守卫没有咬住证明 ⇒ 必须判红
    _s4, _r4 = advance_round(st, {"ratio": 9.0, "share": 0.50},
                             "code ⇒ 读数", False, guards_detail="`scripts/x.py` 没有咬住证明",
                             now="t", head="h", user_case="AGENTS.md")
    gate("④ 反向：新守卫**没有咬住证明** ⇒ 本轮不算收口（判红）", bool(_r4),
         (_r4[0][:56] if _r4 else "**没判红** ✗"))
    # ④ **正向**：有证明 ⇒ 放行（上面 ②-b 第 1 轮已经是正对照 ✓，这里再钉一次形状）
    _s4b, _r4b = advance_round(st, {"ratio": 9.0, "share": 0.50}, "code ⇒ 读数", True,
                               now="t", head="h", user_case="AGENTS.md")
    gate("④ 正对照：咬住证明齐全 ⇒ 放行", not _r4b)

    # ⑤ **反向**：`--user-case` 点到一个**不存在的路径** ⇒ 必须判红（绑了假现场 ✗）
    _s5, _r5 = advance_round(st, {"ratio": 9.0, "share": 0.50}, "code ⇒ 读数", True,
                             now="t", head="h", user_case="不存在的现场.sokonanoda",
                             user_case_ok=False)
    gate("⑤ 反向：`--user-case` 指向**不存在的路径** ⇒ 判红", bool(_r5),
         (_r5[0][:56] if _r5 else "**没判红** ✗"))
    _s5b, _r5b = advance_round(st, {"ratio": 9.0, "share": 0.50}, "code ⇒ 读数", True,
                               now="t", head="h", user_case=None)
    gate("⑤ 反向：**没有** `--user-case`（收口判据没绑用户现场）⇒ 判红", bool(_r5b),
         (_r5b[0][:56] if _r5b else "**没判红** ✗"))

    # ②-e **ratchet 空转**：基点之后落太多笔 ⇒ 判红（"谁开轮"不是纪律 ✓）；
    # ⚠ 但**红必须有出路**（2026-10-16 修）：读数没比上轮低 ⇒ `--open-round` 走不通 ⇒ **告警** ✓
    _idle_red, _idle_warn = judge_round(st, ROUND_MAX_COMMITS + 1, r1)
    gate("② 反向：**ratchet 空转** + 读数**比上轮低**（`--open-round` 走得通）⇒ 判红",
         bool(_idle_red), (_idle_red[0][:60] if _idle_red else "**没判红** ✗"))
    gate("② 反向：空转**判红**时读数**没有**降（`--open-round` 走不通）⇒ 只告警不判红 ✗→✓",
         not judge_round(st, ROUND_MAX_COMMITS + 1,
                         {"ratio": 99.0, "share": 0.99})[0],
         judge_round(st, ROUND_MAX_COMMITS + 1, {"ratio": 99.0, "share": 0.99})[1][0][:60])
    gate("② 正对照：刚开完轮（0 笔）⇒ 不判红也不告警",
         judge_round(st, 0, r1) == ([], []))
    gate("② 正对照：`--open-round` 走得通/走不通的**纯函数**判定",
         open_round_would_pass(st, r1) and not open_round_would_pass(st, {"ratio": 99.0, "share": 0.99})
         and open_round_would_pass({"history": []}, r1),
         f"r1={r1['ratio']} < 上轮 {st['history'][-1]['reading']['ratio']} ⇒ 走得通 ✓")

    # ── ⭐ **接缝**（A 层产出 caps、B 层消费）的正反两向 ─────────────────────
    print("\n接缝（caps **只许有一份**：step 标签必须从 ratchet 状态读 ✓）：")
    _seam_root = Path(tempfile.mkdtemp(prefix="soko-seam-"))
    (_seam_root / "scripts").mkdir(parents=True, exist_ok=True)
    (_seam_root / ".github" / "workflows").mkdir(parents=True, exist_ok=True)
    # **反向**（改前那一行的形状）：标签写死 caps 的字面量 ⇒ 必须判红
    (_seam_root / "scripts" / "ci-local.sh").write_text(
        'run "gates：簿记预算（告警 17:1 / 57% · 红线 19:1 / 60%）" "queue-lint" bash -c \'\n'
        'python3 scripts/check-attention-budget.py --selftest || exit 1\n'
        'python3 scripts/check-attention-budget.py || exit 1\'\n', encoding="utf-8")
    (_seam_root / "scripts" / "soko").write_text("check-attention-budget.py\n", encoding="utf-8")
    (_seam_root / ".github" / "workflows" / "ci.yml").write_text(
        "python3 scripts/check-attention-budget.py\n", encoding="utf-8")
    _bad = seam_problems(_seam_root)
    gate("⭐ 反向：step 标签写死 caps 字面量（`告警 17:1`）⇒ **接缝判红**", bool(_bad),
         (_bad[0][:70] if _bad else "**没判红** ✗"))
    # **正对照**：标签从状态读（`--label`）⇒ 接缝齐
    (_seam_root / "scripts" / "ci-local.sh").write_text(
        'run "gates：$(python3 scripts/check-attention-budget.py --label)" "queue-lint" bash -c \'\n'
        'python3 scripts/check-attention-budget.py --selftest || exit 1\n'
        'python3 scripts/check-attention-budget.py || exit 1\'\n', encoding="utf-8")
    gate("正对照：标签用 `--label` ⇒ 接缝齐", not seam_problems(_seam_root))
    # **反向**：消费点根本没在读 ratchet ⇒ 判红
    (_seam_root / "scripts" / "ci-local.sh").write_text('run "gates：簿记预算" "queue-lint" true\n',
                                                        encoding="utf-8")
    gate("反向：消费点**没在读 ratchet**（没有 `--label` 标记）⇒ 判红",
         bool(seam_problems(_seam_root)),
         (seam_problems(_seam_root)[0][:70] if seam_problems(_seam_root) else "**没判红** ✗"))
    # **反向**：`--source-rev` 指到一个量不到的 rev ⇒ 判红（**不假装判过** ✗）
    gate("反向：`--source-rev` 量不到 ⇒ 判红（不假装判过）",
         bool(seam_problems(ROOT, source_rev="0000000000000000000000000000000000000000")))
    # **正对照**：真仓的接缝齐（`ci-local.sh` 用 `--label` ✓）
    gate("正对照：本仓的接缝齐（ci-local.sh / soko / ci.yml 三个消费点 ✓）",
         not seam_problems(ROOT), "；".join(seam_problems(ROOT))[:70])
    # **caps_label** 的形状（step 标签与判据**同源** ⇒ 数字必须来自传进来的 caps ✓）
    _lbl = caps_label({"ratio_warn": 16.0, "ratio_max": 18.0,
                       "share_warn": 0.55, "share_max": 0.58}, 1)
    gate("正对照：`caps_label()` 的数字**来自 caps**（不是模块常量 ✓）",
         "16:1" in _lbl and "18:1" in _lbl and "55%" in _lbl and "58%" in _lbl
         and "第 1 轮" in _lbl, _lbl)

    # ── ⭐⭐ 端到端：**真 git 夹具仓**上连开两轮（② 的判据在真读数上也成立 ✓）──
    print("\n端到端（**真 git 夹具仓** · `--since` 用远古日期 ⇒ 夹具提交全进窗 ✓）：")
    try:
        with tempfile.TemporaryDirectory(prefix="soko-budget-") as td:
            tmp = Path(td)
            repo, ferr = _fixture_repo(tmp)
            if ferr:
                bad += 1
                print(f"  ✗ 夹具仓建不起来：{ferr}")
            else:
                state = tmp / "ratchet.json"
                base = ["--since", "2000-01-01T00:00:00+08:00", "--root", str(repo),
                        "--state", str(state)]
                rc0 = main(base + ["--init"])
                rows, cerr = collect("2000-01-01T00:00:00+08:00", None, cwd=repo)
                agg0 = aggregate(rows) if not cerr else {}
                gate("端到端 `--init` 写第 0 轮（rc=0）", rc0 == 0, f"rc={rc0}")
                gate("端到端 第 0 轮 caps = 种子（**没放松** ✓）",
                     (state_load(state)[0] or {}).get("caps") == seed_caps(),
                     f"{agg0.get('ratio', float('nan')):.2f} : 1 · "
                     f"{agg0.get('bookkeeping_share', 0) * 100:.1f}%")
                # 落一笔产品 ⇒ 读数变好 ⇒ 开第 1 轮
                p = repo / "crates/front/src/c.rs"
                p.write_text("\n".join(f"h{i}" for i in range(4000)), encoding="utf-8")
                _git(["add", "-A"], repo)
                _git(["commit", "-q", "-m", "feat(front): 第 1 轮产品"], repo)
                rc1 = main(base + ["--open-round", "--user-reading",
                                   "code ⇒ 打开 x.sokonanoda ⇒ Infoview 显示 := 42",
                                   "--user-case", "playground.sokonanoda"])
                st_1 = state_load(state)[0] or {}
                # 再落一笔产品 ⇒ 再变好 ⇒ 开第 2 轮
                q = repo / "crates/front/src/d.rs"
                q.write_text("\n".join(f"i{i}" for i in range(4000)), encoding="utf-8")
                _git(["add", "-A"], repo)
                _git(["commit", "-q", "-m", "feat(front): 第 2 轮产品"], repo)
                rc2 = main(base + ["--open-round", "--user-reading",
                                   "code ⇒ 点 ∈ ⇒ 跳到声明",
                                   "--user-case", "playground.sokonanoda"])
                st_2 = state_load(state)[0] or {}
                gate("端到端 第 1 轮开成（rc=0）", rc1 == 0, f"rc={rc1}")
                gate("端到端 第 2 轮开成（rc=0）", rc2 == 0, f"rc={rc2}")
                gate("⭐⭐ 端到端 ② 的判据：第 2 轮 caps **低于**第 1 轮",
                     bool(st_1) and bool(st_2)
                     and st_2["caps"]["ratio_max"] < st_1["caps"]["ratio_max"]
                     and st_2["caps"]["share_max"] < st_1["caps"]["share_max"],
                     f"第 1 轮 {st_1.get('caps', {}).get('ratio_max')} / "
                     f"{(st_1.get('caps', {}).get('share_max') or 0) * 100:.0f}% → "
                     f"第 2 轮 {st_2.get('caps', {}).get('ratio_max')} / "
                     f"{(st_2.get('caps', {}).get('share_max') or 0) * 100:.0f}%")
                gate("端到端 状态里**逐轮读数可查**（history ≥ 3 条 = 0/1/2 轮）",
                     len(st_2.get("history") or []) >= 3,
                     f"history {len(st_2.get('history') or [])} 条")
                # **反向**：再开一轮但**不落任何提交** ⇒ 读数没降 ⇒ 必须判红
                rc3 = main(base + ["--open-round", "--user-reading", "code ⇒ 没干活",
                                   "--user-case", "playground.sokonanoda"])
                gate("端到端 反向：**没落提交就开轮**（读数没降）⇒ 判红 rc=1",
                     rc3 == 1, f"rc={rc3}")
                # **反向**：缺 `--user-reading` ⇒ 必须判红（③）
                rc4 = main(base + ["--open-round"])
                gate("端到端 反向：缺 `--user-reading`（③）⇒ 判红 rc=1", rc4 == 1, f"rc={rc4}")
    except Exception as err:                                  # noqa: BLE001
        bad += 1
        print(f"  ✗ 端到端夹具炸了：{type(err).__name__}: {err}")

    print(f"\n自检：{'✓ 全过' if bad == 0 else f'✗ {bad} 组不对'}"
          f"（B1×2 + 正对照×3 · B2×1 + 正对照×1 · 分桶 {len(cases)} 格"
          f" · ratchet ②③④ 正反两向 + **空转有/无出路两向** + **接缝正反两向**"
          f" + 真 git 夹具端到端）")
    return 0 if bad == 0 else 1


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="簿记预算（ratchet）")
    ap.add_argument("--since", default=WINDOW_SINCE)
    ap.add_argument("--until", default=None, help="窗口右端（默认 HEAD；反向验证用）")
    # ⚠ **咬住证明的第二根钉子**（`--until` 钉窗口、这条钉 tip）：`--head-rev` 只影响
    #   **只读**的 `commits_since()`（空转那条读数）⇒ 证明的输入与**当前 HEAD 无关** ✓；
    #   `--init` / `--open-round` 写状态时**仍然**用真 `HEAD`（**不许**被这条改 ✗）。
    ap.add_argument("--head-rev", default="HEAD", metavar="<rev>",
                    help="把「tip」钉死（默认 HEAD；**咬住证明用** ✓ —— 空转读数随提交数变）")
    ap.add_argument("--root", default=str(ROOT), help="仓库根（夹具用；默认本仓）")
    ap.add_argument("--state", default=str(RATCHET_STATE), help="ratchet 状态文件")
    # ⚠ **默认 None**（不是模块常量 ✗）：真值一律来自状态里**本轮生效**的 caps ✓ ——
    # 只有显式给（反向验证 / 夹具）才覆盖，且会**响亮**印出来 ✓。
    ap.add_argument("--ratio-max", type=float, default=None, help="覆盖 B1 红线（反向验证用）")
    ap.add_argument("--ratio-warn", type=float, default=None, help="覆盖 B1 告警线")
    ap.add_argument("--share-max", type=float, default=None, help="覆盖 B2 红线")
    ap.add_argument("--share-warn", type=float, default=None, help="覆盖 B2 告警线")
    ap.add_argument("--init", action="store_true",
                    help="写第 0 轮基线（只在落地那一次；已存在 ⇒ rc=1，`--force` 才覆盖）")
    ap.add_argument("--force", action="store_true", help="配合 `--init` 覆盖已有状态")
    ap.add_argument("--open-round", action="store_true",
                    help="⭐ 开下一轮：要求**读数严格下降** + 用户面读数 + 守卫咬住证明")
    ap.add_argument("--user-reading", default=None, metavar="'<命令> ⇒ <读数>'",
                    help="③ 本轮的用户面判据（**真编辑器**里量的 ✓）")
    ap.add_argument("--user-case", default=None, metavar="<仓库路径>",
                    help="⑤ 用户的原始现场（他写的那段代码 / 他做的那个动作所在的路径）")
    ap.add_argument("--recent", type=int, default=0, metavar="N",
                    help="另给一个近 N 笔的读数（**只报不判** ✓）")
    ap.add_argument("--label", action="store_true",
                    help="只印 **step 标签**（caps 从 ratchet 状态读 ⇒ 与判据**同源** ✓）")
    ap.add_argument("--seam", action="store_true",
                    help="接缝：step 标签**真的读 ratchet**（不是写死的 caps 字面量 ✓）")
    ap.add_argument("--source-rev", default=None, metavar="<rev>",
                    help="配合 `--seam`：从**历史版本**读消费点（**咬住证明**用 ✓）")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    if args.selftest:
        return selftest()

    root = Path(args.root).resolve()
    state_path = Path(args.state)
    if not state_path.is_absolute():
        state_path = (root / state_path).resolve()

    # ── `--seam`：**caps 只许有一份**（标签必须从状态读 ✓）──────────────────
    if args.seam:
        problems = seam_problems(root, source_rev=args.source_rev)
        where = f"（源 = `{args.source_rev}` 的历史版本 ✓）" if args.source_rev else ""
        if problems:
            print(f"✗ 簿记预算的**接缝**不齐（{len(problems)} 条）{where}：", file=sys.stderr)
            for m in problems:
                print(f"  · {m}", file=sys.stderr)
            return 1
        print(f"✓ 簿记预算接缝齐（{len(SEAM)}/{len(SEAM)} 个消费点都**真的读 ratchet** ✓ · "
              f"caps 没有第二份字面量 ✓）{where}")
        return 0

    # ── `--label`：step 标签**从状态读**（改前是 `ci-local.sh` 里写死的种子常量 ✗）──
    if args.label:
        st, serr = state_load(state_path)
        if serr:
            # ⚠ **不假装读到了** ✗：印一条响亮的标签（**不是**旧 caps ✗），rc=3 与判据同口径 ✓
            print("簿记预算（caps **读不到** ⇒ 见 check-attention-budget.py ✗）")
            print(f"error: {serr}", file=sys.stderr)
            return 3
        print(caps_label(st["caps"], st.get("round")))
        return 0

    rows, err = collect(args.since, args.until, cwd=root)
    if err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    agg = aggregate(rows)
    now = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")
    head, herr = head_sha(root)

    # ── `--init`：写第 0 轮基线（**只在落地那一次**）────────────────────────
    if args.init:
        if state_path.exists() and not args.force:
            print(f"✗ `{state_path}` 已存在 ⇒ 第 0 轮基线**只写一次** ✗（要覆盖用 `--force`，"
                  f"但那会**丢掉收紧历史** ⇒ 评审可见地解释为什么 ✓）", file=sys.stderr)
            return 1
        if herr:
            print(f"error: {herr}", file=sys.stderr)
            return 3
        st = {"version": 1, "round": 0, "head": head, "at": now, "caps": seed_caps(),
              # ⚠ 第 0 轮 = **基线**：`user_reading` / `user_case` 一律 **null** ✓ ——
              # **不许**编一条没量过的读数 ✗（③/⑤ 从**第 1 轮**起强制 ✓）。
              "history": [{"round": 0, "head": head, "at": now, "caps": seed_caps(),
                           "reading": _reading_of(agg),
                           "user_reading": None, "user_case": None}]}
        state_save(state_path, st)
        print(f"✓ ratchet 第 0 轮基线写入 `{state_path}`：caps = {seed_caps()}（**= 种子，"
              f"一个阈值都没放松** ✓）· 读数 {agg['ratio']:.2f} : 1 · "
              f"{agg['bookkeeping_share'] * 100:.1f}%")
        return 0

    st, serr = state_load(state_path)
    if serr:
        print(f"error: {serr}", file=sys.stderr)
        return 3

    # ── caps：状态里的**本轮生效**两档；显式覆盖要**响亮** ────────────────
    caps = dict(st["caps"])
    overridden = []
    for flag, key in (("ratio_max", "ratio_max"), ("ratio_warn", "ratio_warn"),
                      ("share_max", "share_max"), ("share_warn", "share_warn")):
        given = getattr(args, flag)
        if given is not None:
            caps[key] = given
            overridden.append(f"{flag}={given}")
    if overridden:
        print(f"⚠ **阈值被显式覆盖**（只该用于反向验证/夹具）：{', '.join(overridden)}"
              f" —— 状态里的本轮 caps 是 {st['caps']}", file=sys.stderr)

    n_since, nerr = commits_since(root, st.get("head"), args.head_rev)
    if nerr:
        print(f"error: {nerr}", file=sys.stderr)
        return 3

    # ── `--open-round`：⭐ 本轮不算收口的四道闸 ─────────────────────────────
    if args.open_round:
        guards_ok, guards_detail = guard_bite_check(root, st.get("head"))
        case_ok = bool(args.user_case) and (root / args.user_case).exists()
        new_st, red = advance_round(st, _reading_of(agg), args.user_reading,
                                    guards_ok, guards_detail, now=now, head=head,
                                    user_case=args.user_case, user_case_ok=case_ok)
        if red:
            print(f"✗ **本轮不算收口**（{len(red)} 条）✗ —— ratchet **不动** "
                  f"（第 {st['round']} 轮仍是 {st['caps']}）:", file=sys.stderr)
            for m in red:
                print(f"  · {m}", file=sys.stderr)
            return 1
        state_save(state_path, new_st)
        old, new = st["caps"], new_st["caps"]
        print(f"✓ ratchet **第 {st['round']} → {new_st['round']} 轮** ✓（`{state_path}`）")
        print(f"  读数：比值 {st['history'][-1]['reading']['ratio']:.2f} → {agg['ratio']:.2f} ✓"
              f" · 占比 {st['history'][-1]['reading']['share'] * 100:.1f}% → "
              f"{agg['bookkeeping_share'] * 100:.1f}% ✓（**都严格下降** ✓）")
        print(f"  ⭐ **阈值收紧一档**：B1 红线 {old['ratio_max']} → {new['ratio_max']} · "
              f"告警 {old['ratio_warn']} → {new['ratio_warn']} ｜ "
              f"B2 红线 {old['share_max'] * 100:.0f}% → {new['share_max'] * 100:.0f}% · "
              f"告警 {old['share_warn'] * 100:.0f}% → {new['share_warn'] * 100:.0f}%")
        print(f"  ③ 用户面读数：{args.user_reading}")
        print(f"  ⑤ 用户现场：`{args.user_case}`（在仓库里**真存在** ✓）")
        print(f"  ④ 守卫咬住证明：`check-guard-bite.py --check --since "
              f"{str(st.get('head'))[:12]}` ⇒ rc=0 ✓")
        return 0

    # ── 默认跑：读数 vs **本轮** caps + **开轮纪律** ─────────────────────────
    red, warn = judge(agg, caps["ratio_max"], caps["share_max"],
                      caps["ratio_warn"], caps["share_warn"])
    # ⚠ **空转**：有出路（`--open-round` 当场走得通）⇒ 红 ✓；没出路 ⇒ **告警**（不拦 ✓）
    idle_red, idle_warn = judge_round(st, n_since, _reading_of(agg))
    red += idle_red
    warn += idle_warn
    can_open = open_round_would_pass(st, _reading_of(agg))

    payload = {"reading": agg, "red": red, "warn": warn, "since": args.since,
               "until": args.until, "head_rev": args.head_rev,
               "root": str(root), "state": str(state_path),
               "ratchet": {"round": st["round"], "head": st["head"],
                           "commits_since_head": n_since,
                           "round_max_commits": ROUND_MAX_COMMITS,
                           "open_round_possible": can_open,
                           "caps_in_force": caps, "caps_stored": st["caps"],
                           "label": caps_label(caps, st.get("round")),
                           "seed": seed_caps(), "history_len": len(st.get("history") or []),
                           "user_readings": [h.get("user_reading")
                                             for h in (st.get("history") or [])]},
               "caps": caps}
    if args.recent:
        r, rerr = recent(args.recent, cwd=root)
        payload["recent"] = r if not rerr else {"error": rerr}

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        report(agg, caps, st, n_since)
        if args.recent and "error" not in payload.get("recent", {}):
            r = payload["recent"]
            print(f"  近 {args.recent} 笔（**只报不判** ✓）：比值 {r['ratio']:.2f} : 1 · "
                  f"簿记提交 {r['bookkeeping_share'] * 100:.1f}%")
        for m in warn:
            print(f"::warning:: 簿记预算（告警，**不判红**）：{m}")
        if red:
            print(f"\n✗ 簿记预算 **{len(red)} 条红**：", file=sys.stderr)
            for m in red:
                print(f"  · {m}", file=sys.stderr)
            return 1
        if warn:
            print(f"✓ 簿记预算未超红线 ✓（{len(warn)} 条**告警** —— 看得见，但不拦 ✓）")
        else:
            print("✓ 簿记预算在告警线内 ✓")
    return 0 if not red else 1


if __name__ == "__main__":
    sys.exit(main())
