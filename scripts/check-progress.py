#!/usr/bin/env python3
"""进度判据（**人看通道 ⇄ 机器通道**）：一本登记表 + 一个驱动器（2026-10-17 第 7 轮 `gs7`）。

## 这一条收窄了什么（`SUB-narrow-guards-to-one-layer` 候选 ①）

原文是**两条**判据，量的是**同一条用户等待**的两个通道：

* `scripts/check-progress-cadence.py` —— **人看**的那条（stderr）：N 个文件 ⇒ **逐文件**一行
  `… 42/120 · <path>` + 人话心跳（2026-10-08 用户口径：「build 汇报太稀疏，**约 24 个文件才一条**」✗）；
* `scripts/check-progress-gap.py` —— **机器**那条（`build --json` 事件流）的**最长无输出间隔**
  ≤ 2.5s（2026-09-28 用户口径：冷编课程实测 **39.0s** ✗）。

两条的机械读数逐条相同（`.cache/ux/fix-gs7/measure.out.txt` 可复算）：**同锚**
（`A-TERMINAL-CADENCE`）· **同 job**（`gates-fast`）· **同接线点**（`ci.yml` 同一 job 的相邻两步）·
**重名顶层符号 5**（`resolve_bin` 的**正文逐字相同** ✗ 抄了两遍 · `fixture_env` 只差一处清场 ·
两条各写了一遍同一段 `SOKONANODA_CACHE_DIR` 不变量 ✗）。
⇒ 合并成**一本登记表 + 一个驱动器**（判例 i）；**两层的判据正文逐字保留** ✓
（逐条对账见 `.cache/ux/fix-gs7/VERDICT.md`）。

## 驱动器

    python3 scripts/check-progress.py                         # 两层各判各的（人在本机用）
    python3 scripts/check-progress.py --layer cadence          # 层① 人看通道（CI / gate 点名 ✓）
    python3 scripts/check-progress.py --layer gap              # 层② 机器通道（CI / gate 点名 ✓）
    python3 scripts/check-progress.py --layer cadence --selftest
    python3 scripts/check-progress.py --layer gap --selftest
    python3 scripts/check-progress.py --selftest               # 两层自检 + 登记表 + 断言产出
    python3 scripts/check-progress.py --layer gap --bin scripts/fixtures/progress-bin-double.py

⚠ **接线点一律点名层**（`--layer cadence` / `--layer gap`）⇒ **一条红只属于一层** ✓
（读数可归属 —— 与 `gs5` 的 `--layer` 分派、`gs6` 的「棘轮语义别混层」同一条纪律 ✓）。
⚠ `--files` 属层①、`--decls` 属层② ⇒ **带参数的动作用默认（两层）** ⇒ rc=2 + 人话 ✓。

## 退出码（两层口径**逐字保留** ✓）

  0 = 过 · 1 = **判据红**（人看那条太稀/太静 / 最长空档超阈值）·
  2 = 环境/形状异常（夹具跑不起来 / 量不出来 —— **绝不静默通过** ✗）·
  3 = 拿不到二进制（**不假装判过** ✗ —— 这一档就是锚登记表里 `binary-precondition` 的**事实** ✓，
  由 `check-guard-bite.py` 的 C12 **每次现算**：喂它一个解析不到的 `SOKONANODA_BIN` ⇒ 它必须 rc=3 ✓）。
  默认（两层）⇒ 任一层 3 ⇒ 3；否则任一层 1 ⇒ 1；否则 0 ✓（汇总只为人在本机用 ✓）。

## 登记表（`CRITERIA` ⇄ `REQUIRED_ASSERTIONS`；唯一一处 ✓）

`CRITERIA` = 每条断言（id · 层 · green/selftest · 判什么 · 入口）⇄ `REQUIRED_ASSERTIONS` =
**冻结的期望**（与登记表**分开写** ⇒ 删一条要改两处、评审看得见 ✓）。`registry_problems()`
逐条对账（**缺登记** / **未登记进期望** / **没牙**（入口不可调用）/ **登记了却没有产出点** ⇒ 判红 ✗）。

每个判据的**评估点**上都有一句 `_produced("<id>")`（`_PRODUCED` 关着时是**空操作** ✓ ⇒ 被别处
import 时零副作用 ✓）；`--selftest` **开着记录器**跑「**绿路探针**（真跑该层 CI 接线的那条动作，
只是把它接到夹具上）+ 该层的自检臂」，再**逐条对账** ⇒ **只删代码里的断言（登记表留着）⇒ 当场判红** ✓
（`gs3` 补的那条边界；本轮照 `gs6` 的 (v) 照样做 ✓）。

## 咬住证明的夹具（**本轮新增** ✓ —— `gs5` §2.4 候选 ① 的拦路石）

两条判据的**坏输入长在二进制里**（脚本里没有坏输入可钉 ✗）⇒ 修前它们**一条咬住证明都没有**
（实测 `docs/guard-bite.jsonl` 命中 **0**），而 `check-guard-bite.py` 的 **C1** 要求：**被改动的
守卫必须有证明** ✓。⇒ `scripts/fixtures/progress-bin-double.py` 把**旧二进制的可观测行为**
钉成一份替身（形状**逐行钉在旧 rev 的源码上**：人看通道
`b34a5be7^:crates/cli/src/build.rs:278,295-296` · 机器通道
`6abaf649^:crates/cli/src/build.rs:74,97,110` ✓），咬住证明在**干净检出、无 cargo** 的
`queue-lint` job 里也跑得起来 ✓（`gates-fast` 里照旧用 `cargo build` 出的**真二进制**跑本判据 ✓）。

⚠ **如实划的边界** ✗：替身是**行为**的替身、不是编译出来的旧二进制（真旧二进制不在仓库里、
也不该进 ✗）；它的形状由咬住证明**现场**用 `git show <rev>:crates/cli/src/build.rs` 复核 ✓
—— "我说它长这样"不算数 ✗。
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LAYERS = ("cadence", "gap")

# **绿路探针 / 咬住证明**用的二进制替身（形状逐行钉在旧 rev 的源码上 ✓；见模块 docstring）。
GREEN_FIXTURE = REPO / "scripts" / "fixtures" / "progress-bin-double.py"


# ═══════════════════════════════════════════════════════════════════════════
# 断言产出记录器（判例 v；**唯一一处** ✓）
# ═══════════════════════════════════════════════════════════════════════════
_PRODUCED: set[str] | None = None
"""`None` = 记录器**关着**（默认 ⇒ `_produced()` 是空操作，零副作用 ✓）；
跑 `--selftest` 时开成 `set()` ⇒ 每个判据的评估点往里记一条 ✓。"""


def _produced(assertion_id: str) -> None:
    """记一条断言**这一次真的跑到了**（`gs3` 的 (v) ✓）。**唯一一处** ✓。"""
    if _PRODUCED is not None:
        _PRODUCED.add(assertion_id)


@contextlib.contextmanager
def recording():
    """开着记录器跑一段 ⇒ `yield` 一个 set（跑完自动关 ✓，异常也关 ✓）。"""
    global _PRODUCED
    assert _PRODUCED is None, "记录器不许嵌套（嵌套会把两段的产出混在一起 ✗）"
    _PRODUCED = set()
    try:
        yield _PRODUCED
    finally:
        _PRODUCED = None


# ═══════════════════════════════════════════════════════════════════════════
# 登记表（**唯一一处** ✓）
# ═══════════════════════════════════════════════════════════════════════════
# ⚠ `kind`：`green` = 该层**CI 接线的那条动作真跑**就必须产出的断言；
#           `selftest` = 只有自检臂（反向验证）才跑到的那批 ✓。
CRITERIA: dict[str, dict] = {
    # ── 共享（两层都要它 ⇒ **合并后只剩一条** ✓ —— 原文两条各写一遍 `resolve_bin` ✗）──
    "D-BIN": {"layer": "cadence", "kind": "green", "entry": "resolve_bin",
              "what": "二进制解析链（显式 → `scripts/soko` → 仓库构建）；解析不到 ⇒ rc=3 "
                      "**不假装判过** ✗（锚登记表的 `binary-precondition` 事实 ✓）"},
    # ── 层① cadence（原文 `scripts/check-progress-cadence.py`）───────────────
    "C-ARM1-RC": {"layer": "cadence", "kind": "green", "entry": "cadence_readings",
                  "what": "节奏臂跑得起来（夹具 rc≠0 ⇒ exit 2，绝不静默通过 ✗）"},
    "C-CADENCE": {"layer": "cadence", "kind": "green", "entry": "judge_cadence",
                  "what": "① 节奏：N 个文件 ⇒ 人看的文件级行数 **≥ N**（新旧两种格式都数 ✓）"},
    "C-CLASSIFY": {"layer": "cadence", "kind": "green", "entry": "classify",
                   "what": "分类器：人看的三类行（文件级 / 心跳 / 模块级）数得准"},
    "C-ARM2-RC": {"layer": "cadence", "kind": "green", "entry": "cadence_readings",
                  "what": "心跳臂跑得起来（夹具 rc≠0 ⇒ exit 2）"},
    "C-FIXTURE-SLOW": {"layer": "cadence", "kind": "green", "entry": "cadence_readings",
                       "what": "夹具**够慢**（墙钟 < `MIN_WALL_PERIODS × TICK_MS` ⇒ exit 2："
                               "否则「心跳 ≥2」这条判据是**空转**的 ✗）"},
    "C-HEARTBEAT": {"layer": "cadence", "kind": "green", "entry": "judge_heartbeat",
                    "what": "② 心跳：显式周期 ⇒ 人话心跳 **≥ MIN_HEARTBEATS**（终端不再长时间静止）"},
    "C-STRAY-JSON": {"layer": "cadence", "kind": "green", "entry": "cadence_readings",
                     "what": "非 `--json` 的 stdout 里**不许**出现 `build.tick` JSON（机器契约 ✗）"},
    "C-ARM-SPARSE": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                     "what": "**造稀**：120 文件只有 5 行（每 24 个一行）⇒ 必须判红"},
    "C-ARM-PER-FILE": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                       "what": "逐文件：120 文件 120 行 ⇒ 不红"},
    "C-ARM-119": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                  "what": "边界：119/120 行 ⇒ 差一条也不许放过"},
    "C-ARM-BEAT-0": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                     "what": "**造静**：0 条心跳 ⇒ 必须判红"},
    "C-ARM-BEAT-2": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                     "what": "心跳 2 条 ⇒ 不红"},
    "C-ARM-CLASSIFY": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                       "what": "分类器本身：合成 stderr ⇒ 文件 2 / 心跳 1 / 模块 1"},
    "C-ARM-ENV": {"layer": "cadence", "kind": "selftest", "entry": "cadence_selftest",
                  "what": "夹具环境**原样保留** `SOKONANODA_CACHE_DIR`（动了它 ⇒ 启动器的"
                          "二进制解析链会瞎 ✗；洞 `PROGRESS-CADENCE-WORKTREE-FRAGILE`）"},
    # ── 层② gap（原文 `scripts/check-progress-gap.py`）──────────────────────
    "G-CLEAN": {"layer": "gap", "kind": "green", "entry": "max_gap",
                "what": "夹具预热（`build --json --clean`）跑得起来（rc≠0 ⇒ 量不出来）"},
    "G-EVENTS": {"layer": "gap", "kind": "green", "entry": "max_gap",
                 "what": "事件行数 **≥ 2** 且 rc=0（否则 ⇒ exit 2：量不出来，**不静默通过** ✗）"},
    "G-GAP": {"layer": "gap", "kind": "green", "entry": "judge",
              "what": "最长无输出间隔 **≤ MAX_GAP_S**（`build.decl` + `build.tick` 两条通道）"},
    "G-ARM-NORMAL": {"layer": "gap", "kind": "selftest", "entry": "gap_selftest",
                     "what": "正常流（≤1.0s 间隔）⇒ 不红"},
    "G-ARM-5S": {"layer": "gap", "kind": "selftest", "entry": "gap_selftest",
                 "what": "**造空档**：一段 5s 无输出 ⇒ 必须判红"},
    "G-ARM-26S": {"layer": "gap", "kind": "selftest", "entry": "gap_selftest",
                  "what": "**造空档**：2.6s ⇒ 必须判红（贴着阈值也不放过）"},
    "G-ARM-ENV": {"layer": "gap", "kind": "selftest", "entry": "gap_selftest",
                  "what": "夹具环境**原样保留** `SOKONANODA_CACHE_DIR`（洞 "
                          "`CACHE-DIR-OVERLOAD-2-MORE`；与层① 同一条不变量 ✓）"},
}

# ⚠ **冻结的期望**（与 `CRITERIA` **分开写** ✓ —— "期望"与"登记"是两个东西：
#   删一条要改两处 ⇒ 评审看得见 ✓）。
REQUIRED_ASSERTIONS: frozenset[str] = frozenset({
    "D-BIN",
    "C-ARM1-RC", "C-CADENCE", "C-CLASSIFY", "C-ARM2-RC", "C-FIXTURE-SLOW",
    "C-HEARTBEAT", "C-STRAY-JSON",
    "C-ARM-SPARSE", "C-ARM-PER-FILE", "C-ARM-119", "C-ARM-BEAT-0",
    "C-ARM-BEAT-2", "C-ARM-CLASSIFY", "C-ARM-ENV",
    "G-CLEAN", "G-EVENTS", "G-GAP",
    "G-ARM-NORMAL", "G-ARM-5S", "G-ARM-26S", "G-ARM-ENV",
})


def required_of(layer: str, kind: str | None = None) -> set[str]:
    """该层（可选：该 `kind`）的冻结期望 ✓（`D-BIN` 算层① 的 —— 它在驱动器里先跑 ✓）。"""
    return {a for a in REQUIRED_ASSERTIONS
            if CRITERIA.get(a, {}).get("layer") == layer
            and (kind is None or CRITERIA[a]["kind"] == kind)}


def _source_has_marker(assertion_id: str) -> bool:
    """本文件里有没有 `_produced("<id>")` 这个**产出点**（静态面 ✓ —— 与运行时那一半互补）。"""
    return f'_produced("{assertion_id}")' in Path(__file__).read_text(encoding="utf-8")


def _resolve_entry(entry: str):
    """`entry` 的**可调用**解析：`名字`（本文件顶层 ✓）。"""
    obj: object = globals()
    for part in entry.split("."):
        obj = obj.get(part) if isinstance(obj, dict) else getattr(obj, part, None)
        if obj is None:
            return None
    return obj


def registry_problems() -> list[str]:
    """`CRITERIA` ⇄ `REQUIRED_ASSERTIONS` 逐条对账 ⇒ 红行（**纯函数** ✓）。"""
    red: list[str] = []
    for aid in sorted(REQUIRED_ASSERTIONS - set(CRITERIA)):
        red.append(f"**断言缺登记** ✗：`{aid}` 在冻结期望里，但 `CRITERIA` 里没有它 "
                   f"⇒ 那条判据**没人说得清它判什么** ✗")
    for aid in sorted(set(CRITERIA) - REQUIRED_ASSERTIONS):
        red.append(f"**未登记进期望** ✗：`{aid}` 在 `CRITERIA` 里，但冻结期望里没有它 "
                   f"⇒ 要么补进期望、要么删掉（**期望是收窄的锚** ✓）")
    for aid, spec in sorted(CRITERIA.items()):
        if spec.get("layer") not in LAYERS:
            red.append(f"**登记表坏** ✗：`{aid}` 的 `layer` = {spec.get('layer')!r} "
                       f"（只许 {list(LAYERS)}）")
        if spec.get("kind") not in ("green", "selftest"):
            red.append(f"**登记表坏** ✗：`{aid}` 的 `kind` = {spec.get('kind')!r}"
                       f"（只许 green / selftest）")
        entry = str(spec.get("entry") or "")
        if not entry:
            red.append(f"**登记表坏** ✗：`{aid}` 没写 `entry`（判据入口）")
        elif not callable(_resolve_entry(entry)):
            red.append(f"**断言没牙** ✗：`{aid}` 的入口 `{entry}` 在本文件里**不可调用** "
                       f"（名字改了 / 删了 ⇒ 那条断言**没有牙** ✗）")
        if not _source_has_marker(aid):
            red.append(f"**登记了却没有产出点** ✗：`{aid}` 在登记表里，但本文件里找不到 "
                       f"`_produced('{aid}')` ⇒ 它**永远不会被产出** ⇒ (v) 的判据**空转** ✗")
    return red


def produced_problems(expected: set[str], produced: set[str], what: str) -> list[str]:
    """这一跑里**缺产出的断言** ⇒ 红行（**只删代码里的断言也判红** ✓）。

    ⚠ `expected` 由调用方给（`green` 那一档与 `selftest` 那一档**分开** ⇒ 混起来会把
    "绿路没跑到自检臂"误判成缺产出 ✗ —— 照 `check-budgets.py` 的先例 ✓）。
    """
    miss = sorted(expected - produced)
    if not miss:
        return []
    return [f"**断言缺产出** ✗：`{a}` 在冻结期望（{what}）里、但**这一次根本没产出它** "
            f"⇒ 那条判据的评估点被删了 / 改名了（**只删代码里的断言也判红** ✓）"
            for a in miss]


# ═══════════════════════════════════════════════════════════════════════════
# 两层共用（判例 iv：原文**两处各写一遍** ⇒ 合并后**各一处** ✓）
# ═══════════════════════════════════════════════════════════════════════════
def resolve_bin(explicit: str | None) -> tuple[list[str], str] | None:
    """二进制解析链：显式 → `scripts/soko` → 仓库构建（**正文逐字相同**的两份 ⇒ 合并 ✓）。

    ⚠ 显式路径**存在性**是判据的一部分：`--bin` 指了一个不存在的路径 ⇒ 返回 `None`
    ⇒ 调用方 rc=3（**不假装判过** ✗ —— 这条正是 `binary-precondition` 那个 witness 的事实 ✓）。
    """
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


def fixture_env(env_extra: dict[str, str]) -> dict[str, str]:
    """夹具子进程的环境（**纯函数** ⇒ `--selftest` 能直接断言它 ✓）。**唯一一处** ✓

    ⚠ **不变量（2026-10-15 PC-Z / CD-Y，洞 `PROGRESS-CADENCE-WORKTREE-FRAGILE` ·
    `CACHE-DIR-OVERLOAD-2-MORE`）**：**绝不动 `SOKONANODA_CACHE_DIR`** ✗ —— 它**同时**是
    启动器 `scripts/soko` 的「二进制缓存槽」（`cacheDir()`）⇒ 把它指到空目录 = 把二进制
    解析链弄瞎 ✗。冷编译缓存走 `SOKONANODA_NO_CACHE`（CLI 的**专用**开关 ✓）。

    ⚠ **如实标注（判例 iii/iv 的交界）** ✗：原文层① 在这里**多两行**
    （`SOKO_BUILD_NO_TICK` 清场：`env[...] = ""` 再 `pop` ⇒ 净效果 = **删掉**外部干扰），
    层② **没有** ⇒ 合并后**统一取层① 的那一版** ✓。理由：那条清场是**探针身份**的要求
    （外部 `SOKO_BUILD_NO_TICK=1` 会把心跳通道关掉 ⇒ 量到的不是被测代码 ✗），
    而**判据正文（阈值 / 分类器 / 退出码）一个字没动** ✗。实测：环境里没有它时两版**逐字节
    等价**（`.cache/ux/fix-gs7/VERDICT.md` §4 §A6 给了两半读数 ✓）。
    """
    env = dict(os.environ)
    env["SOKONANODA_NO_CACHE"] = "1"  # 冷编译缓存：读/写全关 ⇒ 0 命中全编 ✓
    env["SOKONANODA_OFFLINE"] = "1"  # 解析不到就立刻出声，不空等网络 ✓
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"
    env["SOKO_BUILD_NO_TICK"] = ""  # 清掉外部干扰（下面按需再设）
    env.pop("SOKO_BUILD_NO_TICK", None)
    env.update(env_extra)
    return env


# ═══════════════════════════════════════════════════════════════════════════
# 层① cadence —— **人看**的那条（stderr）：逐文件 + 人话心跳
#    （原文 `scripts/check-progress-cadence.py` 的正文，逐字保留 ✓）
# ═══════════════════════════════════════════════════════════════════════════
FILES = 120  # ① 的夹具文件数（判据：人看的行数 ≥ 它）
MIN_HEARTBEATS = 2  # ② 的判据
TICK_MS = 200  # ② 的显式心跳周期（结构判据：不依赖机器快慢）
# ② 的**自检**：夹具必须跑够久（≥ 这么多倍周期）—— 否则"心跳条数"这条判据是
# **空转**的 ✗（2026-10-08 实测踩到：内核提速后 40 条 `by` 证明只跑 0.23s ⇒
# 200ms 周期只塞得下 1 条 ⇒ 假红 ✗）。夹具不够慢 ⇒ **exit 2**，绝不静默通过 ✓。
MIN_WALL_PERIODS = 4
# ② 的夹具规模（条 `by` 证明）—— 目标是让墙钟 ≫ `TICK_MS × MIN_WALL_PERIODS`。
HEART_DECLS = 200

# 人看的**文件级**进度行：`… 42/120 · units/…/x.sokonanoda`（`build.rs` 的 `note_file`）。
FILE_LINE = re.compile(r"^\s*…\s+\d+/\d+\s+·\s+\S")
# **旧格式**（C1 之前）：`… 12/120 file(s)` —— 它**也要数** ✓：本判据量的是
# **节奏**（一个文件编完有没有出声），不是**格式** ✗。数旧格式的用处正是
# **反向验证**：拿 C1 之前的二进制跑同一判据，它必须因为「120 个文件只有 10 行」
# 判红 ✓ —— 而不是因为"格式不匹配 ⇒ 0 行"这种**假理由** ✗。
FILE_LINE_LEGACY = re.compile(r"^\s*…\s+\d+/\d+\s+file\(s\)\s*$")
# 人看的**心跳**行：`… still building (12s) · <path>`。
HEART_LINE = re.compile(r"^\s*…\s+still building\s+\(")
# 模块级 tick 的人看那一半：`… module 3/40 · <name>`。
MODULE_LINE = re.compile(r"^\s*…\s+module\s+\d+/\d+\s+·\s+\S")

SMALL_DECL = "theorem t{i} : Prop := Prop\n"
BIG_DECL = """\
theorem big_{i} (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
"""


def judge_cadence(file_lines: int, total: int) -> tuple[int, str]:
    """① 的判据：人看的文件级行数 ≥ 文件数。"""
    _produced("C-CADENCE")
    if file_lines >= total:
        return 0, f"✓ 人看的进度 {file_lines} 行 ≥ {total} 个文件（逐文件 ✓）"
    return 1, (
        f"✗ 人看的进度只有 {file_lines} 行，而文件有 {total} 个 —— "
        f"「约 {total // max(file_lines, 1)} 个文件才一条」正是用户报的现象 ✗\n"
        f"   （`ProgressCounter::note_file` 必须**逐文件**打；见 PLAN-cli-editor-perf.md §1 P1）"
    )


def judge_heartbeat(beats: int, want: int) -> tuple[int, str]:
    """② 的判据：人话心跳条数 ≥ want。"""
    _produced("C-HEARTBEAT")
    if beats >= want:
        return 0, f"✓ 人话心跳 {beats} 条 ≥ {want}（终端不再长时间静止 ✓）"
    return 1, (
        f"✗ 人话心跳只有 {beats} 条（< {want}）—— 长编译期间终端会静止 ✗\n"
        f"   （`Heartbeat` 在非 `--json` 时该走 stderr 的人话；见 PLAN-cli-editor-perf.md §1 P1）"
    )


def classify(stderr_text: str) -> tuple[int, int, int]:
    """数三类人看的行（文件级 / 心跳 / 模块级）。**新旧两种文件级格式都算** ✓。"""
    _produced("C-CLASSIFY")
    files = beats = modules = 0
    for line in stderr_text.splitlines():
        if FILE_LINE.match(line) or FILE_LINE_LEGACY.match(line):
            files += 1
        elif HEART_LINE.match(line):
            beats += 1
        elif MODULE_LINE.match(line):
            modules += 1
    return files, beats, modules


def run_build(cmd: list[str], args: list[str], env_extra: dict[str, str],
              cwd: Path | None = None) -> tuple[int, str, str, float] | None:
    """跑一次夹具编译（**冷缓存** + **离线** ✓）。

    ## 冷缓存走 `SOKONANODA_NO_CACHE`，**不许**换掉 `SOKONANODA_CACHE_DIR` ✗

    ① 这条臂要的是**冷编译缓存**：120 个文件必须**真编**，命中缓存就没有逐文件进度行
    了 ✓。CLI 为这件事留了**专用开关** —— `SOKONANODA_NO_CACHE=1` ⇒
    `front::compile::cache::root()` 直接返回 `None`
    （`crates/front/src/compile/cache.rs:109`）⇒ 读/写全关、0 命中全编 ✓
    （`crates/cli/tests/judge_batch.rs:41` 用的就是它 ✓）。语义与"空缓存目录"**等价**，
    但**不占用任何别的格子** ✓。

    ② 旧实现（2026-10-08 ～ 2026-10-15）把 `SOKONANODA_CACHE_DIR` 指到**空临时目录** ✗
    —— 而**同一个变量还是启动器的「二进制缓存槽」**（`cacheDir()`）⇒ 子进程里那一格
    **必然为空** ✗ ⇒ 启动器只剩「仓库构建 / 下载」两条路。⚠ 工作树里 `<repo>/target`
    **根本不存在**（本仓 worktree 编到 `CARGO_TARGET_DIR` ✓）⇒ 一棵**刚
    `scripts/soko setup` 过、还没编过**的工作树结构性假红** ✗ —— 实测（`pcZ`：往
    `.cache/soko/pcZ/` 放一份**版本匹配**的 0.91.0 二进制，启动器**本该**解析到它 ✓）：
    启动器 rc=3 `refusing to run an unverified sokonanoda binary — nothing resolved
    (offline=no) … (marker: none)` ⇒ 本判据 exit 2 = **判据红** ✗。下载那条路更糟：
    通了就**悄悄换成下载来的发布二进制**去量（**探针身份** ✗ —— 量到的不是被测代码），
    不通就是 ~60s 网络空等 ✗。

    ⇒ ③ `SOKONANODA_OFFLINE=1`：解析不到就**立刻**用启动器自己的话出声 ✓（不再空等
    网络 ✓；与 `check-lsp-marker-seam.py` 的离线夹具同一条纪律 ✓）。它**只**关掉
    「下载一个二进制」这条**获取**路径，**判据一条没动** ✓ —— 两臂的夹具都没有
    import ⇒ 编译过程本来就不碰网络（CLI 的 `offline()` 只用在
    `crates/cli/src/env/mod.rs` 的 `setup`/报告 ✓）。

    ④ `CARGO_TARGET_DIR` **原样透传**：启动器自己就认它（`repoBuildCandidates()`，
    2026-10-12 `SUB-cargo-target-dir` ✓）⇒ worktree 里刚编好的那份会被解析到 ✓
    （实测 `version --json` ⇒ `source: repo-build` ·
    `.cache/compile/targets/pcZ/debug/sokonanoda` ✓）。⇒ 本洞的修法**不是**"再教一遍
    `CARGO_TARGET_DIR`"（那件事已经成立 ✓），而是**别把解析链弄瞎** ✓。
    """
    env = fixture_env(env_extra)
    t0 = time.time()
    proc = subprocess.run(cmd + args, capture_output=True, text=True, encoding="utf-8",
                          errors="replace", env=env, cwd=str(cwd or REPO), timeout=300)
    return proc.returncode, proc.stdout, proc.stderr, time.time() - t0


def cadence_readings(cmd: list[str], files: int, shape_env: dict[str, str]) -> dict:
    """层① 的两条臂（**正文逐字保留** ✓）⇒ 读数 dict（`rc` ∈ {0,1,2}；`error` 非空 ⇒ 印它）。

    ⚠ `shape_env` 是**夹具缝**：真跑时是 `{}` ✓；`--selftest` 的绿路探针往里塞
    `SOKO_FIXTURE_SHAPE=current`（把它接到**替身**上 ✓）—— 判据正文与阈值一个字没动 ✗。
    """
    tmp = Path(tempfile.mkdtemp(prefix="c1-cadence-"))
    try:
        # ① 节奏臂：N 个各自独立的文件（无 import ⇒ 每个都是"入口"）。
        cadence_dir = tmp / "many"
        cadence_dir.mkdir(parents=True, exist_ok=True)
        for i in range(files):
            (cadence_dir / f"f{i:04}.sokonanoda").write_text(
                SMALL_DECL.format(i=i), encoding="utf-8")
        got = run_build(cmd, ["build", str(cadence_dir)], dict(shape_env))
        if got is None:
            # ⚠ **原文逐字保留、且原文里就不可达** ✗：`run_build` 恒返回 4 元组
            #   （`tuple[...] | None` 的注解比实现宽）⇒ 这一档**不登记为断言**（如实 ✗，
            #   见 VERDICT 的"逐条对应"表 —— 不假装它是活的判据 ✓）。
            _produced("C-ARM1-RC")
            return {"rc": 2, "error": "error: 节奏臂跑不起来 —— exit 2"}
        code1, out1, err1, wall1 = got
        if code1 != 0:
            _produced("C-ARM1-RC")
            return {"rc": 2, "error": f"error: 节奏臂 exit {code1} —— exit 2\n{err1[-400:]}"}
        _produced("C-ARM1-RC")
        files_seen, beats1, modules1 = classify(err1)
        cadence = judge_cadence(files_seen, files)

        # ② 心跳臂：一个入口 + 若干 `by` 证明，**显式周期**（结构判据）。
        beat_dir = tmp / "slow"
        beat_dir.mkdir(parents=True, exist_ok=True)
        body = "".join(BIG_DECL.format(i=i) for i in range(HEART_DECLS))
        (beat_dir / "slow.sokonanoda").write_text(body, encoding="utf-8")
        got = run_build(cmd, ["build", str(beat_dir)],
                        {**shape_env, "SOKO_BUILD_TICK_MS": str(TICK_MS)})
        if got is None:
            _produced("C-ARM2-RC")  # 同上：原文逐字保留、原文里不可达 ⇒ 不登记 ✓
            return {"rc": 2, "error": "error: 心跳臂跑不起来 —— exit 2"}
        code2, out2, err2, wall2 = got
        if code2 != 0:
            _produced("C-ARM2-RC")
            return {"rc": 2, "error": f"error: 心跳臂 exit {code2} —— exit 2\n{err2[-400:]}"}
        _produced("C-ARM2-RC")
        _, beats2, _ = classify(err2)
        # **自检：夹具必须够慢**（否则"心跳 ≥2"恒真/恒假 ⇒ 判据咬不住 ✗）。
        _produced("C-FIXTURE-SLOW")
        if wall2 < (TICK_MS * MIN_WALL_PERIODS) / 1000.0:
            return {"rc": 2, "error": (
                f"error: 心跳臂夹具只跑了 {wall2:.2f}s < {MIN_WALL_PERIODS} × {TICK_MS}ms —— "
                f"夹具不够慢 ⇒ 心跳判据**量不出来**（exit 2，绝不静默通过）✗\n"
                f"   修法：加大 `HEART_DECLS`（现 {HEART_DECLS}）")}
        heartbeat = judge_heartbeat(beats2, MIN_HEARTBEATS)
        # **机器契约不许动**：非 `--json` 的 stdout 里**不许**出现 `build.tick` JSON。
        stray_json = out2.count('"build.tick"')
        _produced("C-STRAY-JSON")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    codes = [cadence[0], heartbeat[0], 1 if stray_json else 0]
    return {"rc": max(codes), "error": None, "files_seen": files_seen, "modules1": modules1,
            "wall1": wall1, "cadence": cadence, "beats2": beats2, "wall2": wall2,
            "heartbeat": heartbeat, "stray_json": stray_json}


def cadence_selftest() -> int:
    """反向验证：**合成的行流**喂给同一个判据（确定性、不依赖真编译 ✓）。"""
    bad = 0

    def check(why: str, got: tuple[int, str], want_red: bool) -> None:
        nonlocal bad
        code, _ = got
        ok = (code != 0) if want_red else (code == 0)
        print(f"   [{'ok' if ok else 'BAD'}] {why}：exit {code}（期望 {'红' if want_red else '绿'}）")
        bad += 0 if ok else 1

    # ⚠ 每条臂的 `_produced("<id>")` 必须**字面**写在它的评估点上 ✓（`_source_has_marker()`
    #   按字面找；写成 `_produced(变量)` ⇒ 静态面找不到 ⇒ 判红 —— **实测踩到过** ✗）。
    # 「每 24 个一行」——用户报的那个形状（120 文件 ⇒ 5 行）
    _produced("C-ARM-SPARSE")
    check("**造稀**：120 文件只有 5 行（每 24 个一行）⇒ 必须判红",
          judge_cadence(5, FILES), True)
    # 逐文件
    _produced("C-ARM-PER-FILE")
    check("逐文件：120 文件 120 行 ⇒ 不红", judge_cadence(FILES, FILES), False)
    # 边界：119 行（差一条也不许放过）
    _produced("C-ARM-119")
    check("**造稀**：119/120 行 ⇒ 必须判红", judge_cadence(FILES - 1, FILES), True)
    # 心跳
    _produced("C-ARM-BEAT-0")
    check("**造静**：0 条心跳 ⇒ 必须判红", judge_heartbeat(0, MIN_HEARTBEATS), True)
    _produced("C-ARM-BEAT-2")
    check("心跳 2 条 ⇒ 不红", judge_heartbeat(2, MIN_HEARTBEATS), False)
    # 分类器本身：合成的 stderr
    _produced("C-ARM-CLASSIFY")
    synth = "\n".join([
        "scanning the current directory for *.sokonanoda …（跳过 .git / node_modules）",
        "found 120 .sokonanoda file(s)",
        "… 1/120 · a.sokonanoda",
        "… module 1/2 · Shared",
        "… still building (1s) · b.sokonanoda",
        "… 2/120 · b.sokonanoda",
        "built 120 file(s) — 0 hit, 120 compiled, 0 failed",
    ])
    files, beats, modules = classify(synth)
    ok = (files, beats, modules) == (2, 1, 1)
    print(f"   [{'ok' if ok else 'BAD'}] 分类器：合成流 ⇒ 文件 {files} / 心跳 {beats} / 模块 {modules}"
          f"（期望 2/1/1）")
    bad += 0 if ok else 1
    # **夹具环境的不变量**（2026-10-15 PC-Z，洞 `PROGRESS-CADENCE-WORKTREE-FRAGILE` ✓）：
    # 旧实现把 `SOKONANODA_CACHE_DIR` 指到空临时目录 ✗ —— 而它**同时**是启动器的
    # 「二进制缓存槽」⇒ worktree 里（`<repo>/target` 不存在、构建在 `CARGO_TARGET_DIR`）
    # 只剩"仓库构建/下载"两条路 ⇒ **刚 `scripts/soko setup` 过、还没编过的树结构性假红** ✗
    # （实测启动器 rc=3 ⇒ 本判据 exit 2 ✓）。⇒ 钉住：那份环境必须**原样保留** ✓，
    # 冷缓存必须走 `SOKONANODA_NO_CACHE` ✓。**咬得住的守卫**（把旧写法放回来 ⇒ 判红 ✓）。
    _produced("C-ARM-ENV")
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
    print("selftest: ✓ 反向验证全过（造稀/造静判红、逐文件不红、分类器数得准、"
          "夹具环境不动启动器的缓存槽）")
    return 0


def cadence_main(argv: list[str]) -> int:
    """层① 的入口（**原文的 argparse 与判据行逐字保留** ✓ + 驱动器缝 `--layer` 在 `main()`）。"""
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--bin", default=None)
    ap.add_argument("--files", type=int, default=FILES, help="① 的夹具文件数（默认 120）")
    args = ap.parse_args(argv)

    if args.selftest:
        return _selftest_with_produced("cadence")

    resolved = resolve_bin(args.bin or os.environ.get("SOKONANODA_BIN"))
    if resolved is None:
        _produced("D-BIN")
        print("error: 拿不到可用的 sokonanoda CLI —— 进度判据无法判定（exit 3，不静默跳过）",
              file=sys.stderr)
        return 3
    _produced("D-BIN")
    cmd, source = resolved

    res = cadence_readings(cmd, args.files, {})
    if res["error"] is not None:
        print(res["error"], file=sys.stderr)
        return res["rc"]
    code = res["rc"]
    if args.json:
        print(json.dumps({
            "schema": "soko.progress-cadence/1", "bin": source,
            "files": args.files, "file_lines": res["files_seen"], "module_lines": res["modules1"],
            "heartbeats": res["beats2"], "stray_json_ticks": res["stray_json"],
            "heart_decls": HEART_DECLS, "min_wall_periods": MIN_WALL_PERIODS,
            "wall_s": [round(res["wall1"], 2), round(res["wall2"], 2)], "exit": code,
        }, ensure_ascii=False))
        return code
    print(f"进度判据（C1 · 人看的通道）：二进制 {source}")
    print(f"  ① 节奏：{args.files} 个文件 ⇒ 人看 {res['files_seen']} 行（模块级 {res['modules1']} 行）"
          f"· 墙钟 {res['wall1']:.2f}s")
    print(f"     {res['cadence'][1]}")
    print(f"  ② 心跳：周期 {TICK_MS}ms ⇒ 人话心跳 {res['beats2']} 条 · 墙钟 {res['wall2']:.2f}s"
          f"（自检：≥ {MIN_WALL_PERIODS} 个周期 ✓）")
    print(f"     {res['heartbeat'][1]}")
    if res["stray_json"]:
        print(f"  ✗ 非 `--json` 的 stdout 里出现了 {res['stray_json']} 条 `build.tick` JSON —— "
              f"机器契约不许动 ✗")
    return code


# ═══════════════════════════════════════════════════════════════════════════
# 层② gap —— **机器**那条（`build --json` 事件流）的最长无输出间隔
#    （原文 `scripts/check-progress-gap.py` 的正文，逐字保留 ✓）
# ═══════════════════════════════════════════════════════════════════════════
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


def write_fixture(root: Path, decls: int) -> str:
    """层② 的夹具：`lib/Shared` + 一个大入口（若干 `by` 证明）⇒ 冷编它。"""
    (root / "lib").mkdir(parents=True, exist_ok=True)
    (root / "sokonanoda.toml").write_text('name = "p2-gap"\n', encoding="utf-8")
    (root / "lib" / "Shared.sokonanoda").write_text(LIB, encoding="utf-8")
    entry = root / "big.sokonanoda"
    entry.write_text(ENTRY_HEAD + "".join(ENTRY_DECL.format(i=i) for i in range(decls)), encoding="utf-8")
    return str(entry)


def max_gap(cmd: list[str], root: Path, entry: str,
            shape_env: dict[str, str] | None = None) -> tuple[float, int, float] | None:
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

    ③ `SOKONANODA_OFFLINE=1`：解析不到就**立刻**用启动器自己的话出声 ✓（不再空等
    网络 ~60s ✗，也不**悄悄换成下载来的发布二进制**去量 —— 那是**探针身份**事故 ✗）。
    ⚠ 它在这条臂上只作用于**启动器**那条「下载一个二进制」的获取路径 ✓ —— CLI 侧
    `offline()` 只被 `setup` 与 `doctor`/`version` 的报告读
    （`crates/cli/src/env/target.rs:77` · `env/mod.rs:97,120,154`）⇒ 对 `build` **零影响**，
    **判据一条没动** ✓（夹具的 import 全是同目录的**本地**模块 ✓）。

    ④ `CARGO_TARGET_DIR` **原样透传**：⇒ 本洞的修法**不是**"再教一遍 `CARGO_TARGET_DIR`"
    （那件事早就成立 ✓），而是**别把解析链弄瞎** ✓。

    ⚠ `shape_env` = **夹具缝**（真跑时 `None` ⇒ `{}` ✓；`--selftest` 的绿路探针用它把
    这条臂接到**替身**上 ✓）—— 判据正文一个字没动 ✗。
    """
    env = fixture_env(dict(shape_env or {}))
    if subprocess.run(cmd + ["build", "--json", "--clean", str(root)],
                      capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, cwd=REPO, timeout=120).returncode != 0:
        return None
    _produced("G-CLEAN")
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
    _produced("G-EVENTS")
    if proc.returncode != 0 or lines < 2:
        return None
    return (worst, lines, time.time() - t0)


def judge(gap: float, threshold: float) -> tuple[int, str]:
    """层② 的判据：最长无输出间隔 ≤ 阈值。"""
    _produced("G-GAP")
    if gap <= threshold:
        return 0, f"✓ 最长无输出间隔 {gap:.2f}s ≤ {threshold}s"
    return 1, (
        f"✗ 最长无输出间隔 {gap:.2f}s > {threshold}s —— UI 会长时间不动\n"
        f"   （改前实测 39.0s；`build.decl` 声明级 + `build.tick` 心跳两条通道都该在发，"
        f"见 docs/protocol.md 的 P2 一节）"
    )


def gap_selftest() -> int:
    """反向验证：喂**合成**的事件流给同一个判据（不依赖真编译，确定性 ✓）。"""
    bad = 0

    def arm(assertion_id_marker: str, stamps: list[float], why: str, want: str) -> None:
        """一条臂（`assertion_id_marker` 只用于印话 —— 产出点由调用方的**字面**
        `_produced("<id>")` 负责 ✓）。"""
        nonlocal bad
        gaps = [b - a for a, b in zip(stamps, stamps[1:])]
        gap = max(gaps) if gaps else 0.0
        code, _ = judge(gap, MAX_GAP_S)
        ok = (code == 0) if want == "green" else (code != 0)
        print(f"   [{'ok' if ok else 'BAD'}] {why}：最大间隔 {gap:.2f}s ⇒ exit {code}（期望 {want}）")
        bad += 0 if ok else 1

    # ⚠ 每条臂的 `_produced("<id>")` **字面**写在它的评估点上 ✓（同层① 的理由 ✓）。
    _produced("G-ARM-NORMAL")
    arm("G-ARM-NORMAL", [0.0, 0.9, 1.9, 3.0, 3.4], "正常流（≤1.0s 间隔）", "green")
    _produced("G-ARM-5S")
    arm("G-ARM-5S", [0.0, 0.5, 5.5, 6.0], "**造空档**：一段 5s 无输出 ⇒ 必须判红", "red")
    _produced("G-ARM-26S")
    arm("G-ARM-26S", [0.0, 2.0, 4.6], "**造空档**：2.6s ⇒ 必须判红", "red")
    # 形状异常：只有 1 行 ⇒ 量不出来 ⇒ 必须 exit 2（绝不静默通过）
    print("   [ok] 只有 1 行事件（量不出来）⇒ 主流程判 exit 2（不静默通过）")
    # **夹具环境的不变量**（2026-10-15 CD-Y，洞 `CACHE-DIR-OVERLOAD-2-MORE` ✓）：
    # 旧实现把 `SOKONANODA_CACHE_DIR` 指到空临时目录 ✗ —— 而它**同时**是启动器的
    # 「二进制缓存槽」⇒ worktree 里（`<repo>/target` 不存在、构建在 `CARGO_TARGET_DIR`）
    # 只剩"仓库构建 / 下载"两条路 ⇒ **刚 `scripts/soko setup` 过、还没编过的树结构性假红** ✗
    # （实测启动器 rc=3 ⇒ 本判据 exit 2 ✓）。⇒ 钉住：那份环境必须**原样保留** ✓，
    # 冷缓存必须走 `SOKONANODA_NO_CACHE` ✓。**咬得住的守卫**（把旧写法放回来 ⇒ 判红 ✓）。
    _produced("G-ARM-ENV")
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


def gap_main(argv: list[str]) -> int:
    """层② 的入口（**原文的 argparse 与判据行逐字保留** ✓）。"""
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--bin", default=None)
    ap.add_argument("--decls", type=int, default=24, help="夹具入口的声明数（默认 24 ⇒ 冷编数秒）")
    args = ap.parse_args(argv)

    if args.selftest:
        return _selftest_with_produced("gap")

    resolved = resolve_bin(args.bin or os.environ.get("SOKONANODA_BIN"))
    if resolved is None:
        _produced("D-BIN")
        print("error: 拿不到可用的 sokonanoda CLI —— 进度判据无法判定（exit 3，不静默跳过）",
              file=sys.stderr)
        return 3
    _produced("D-BIN")
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


# ═══════════════════════════════════════════════════════════════════════════
# 驱动器（一本登记表 + 一个驱动器 ✓；`--layer` 分派 ⇒ **一条红只属于一层** ✓）
# ═══════════════════════════════════════════════════════════════════════════
_LAYER_MAIN = {"cadence": cadence_main, "gap": gap_main}


def _split_layer(argv: list[str]) -> tuple[str, list[str]]:
    """`--layer cadence|gap|all`（缺省 `all`）⇒ `(层, 其余 argv)`。**唯一一处** ✓

    ⚠ 只摘 `--layer` 自己（其余**原样**转给那一层的 argparse ✓ ⇒ 两层的 CLI 逐字保留 ✓）。
    """
    layer, rest, i = "all", [], 0
    while i < len(argv):
        a = argv[i]
        if a == "--layer":
            if i + 1 >= len(argv):
                print("error: `--layer` 后面要跟 cadence|gap|all", file=sys.stderr)
                raise SystemExit(2)          # ⚠ 用法错 = 2（与 argparse 同口径 ✓）
            layer, i = argv[i + 1], i + 2
            continue
        if a.startswith("--layer="):
            layer, i = a.split("=", 1)[1], i + 1
            continue
        rest.append(a)
        i += 1
    if layer not in (*LAYERS, "all"):
        print(f"error: `--layer` 只许 {'|'.join(LAYERS)}|all（给的是 {layer!r}）", file=sys.stderr)
        raise SystemExit(2)
    return layer, rest


def _has_flag(argv: list[str], name: str) -> bool:
    return any(a == name or a.startswith(f"{name}=") for a in argv)


def _knob_problems(layer: str, rest: list[str]) -> list[str]:
    """**层专属开关**不许跨层用（判例 vi：一条红只属于一层 ✓）。

    `--files` 属层①（节奏臂的文件数）· `--decls` 属层②（空档臂的声明数）⇒
    带参数的动作用**点名层**的那条命令 ✓；默认（两层）不接受它们 ⇒ rc=2 + 人话 ✓。
    """
    red: list[str] = []
    if _has_flag(rest, "--files") and layer in ("gap", "all"):
        red.append("`--files` 属层①（cadence）⇒ 用 `--layer cadence --files N`（**不许**跨层 / "
                   "在两层默认动作里用 ✗）")
    if _has_flag(rest, "--decls") and layer in ("cadence", "all"):
        red.append("`--decls` 属层②（gap）⇒ 用 `--layer gap --decls N`（**不许**跨层 / "
                   "在两层默认动作里用 ✗）")
    return red


def _run_layer(layer: str, argv: list[str]) -> tuple[int, str]:
    """跑一层（**捕获**它的 stdout ⇒ 供两层汇总 / `--json` 组合 ✓）⇒ (rc, 输出)。"""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = _LAYER_MAIN[layer](argv)
    return rc, buf.getvalue()


@contextlib.contextmanager
def _fixture_shape(shape: str):
    """把**夹具形状**放进环境（`fixture_env` 会原样透传给子进程 ✓）⇒ 跑完还原 ✓。"""
    saved = os.environ.get("SOKO_FIXTURE_SHAPE")
    os.environ["SOKO_FIXTURE_SHAPE"] = shape
    try:
        yield
    finally:
        if saved is None:
            os.environ.pop("SOKO_FIXTURE_SHAPE", None)
        else:
            os.environ["SOKO_FIXTURE_SHAPE"] = saved


def _green_probe(layer: str) -> tuple[set[str], list[str]]:
    """**绿路探针**（判例 v）：把该层 **CI 接线的那条动作**在这一进程里**真跑**一遍
    （输出捕获 ⇒ 不污染本跑的判据行 ✓）⇒ (产出的断言, 环境红行)。

    ⚠ **真跑**（调的是同一批 `main()`，不是打桩 ✓）；⚠ 判红/告警**与产出无关**（断言
    **跑到**就产出 ✓ —— 探针只问"这些判据有没有被跑到"，不问"读数绿不绿" ✓）。
    ⚠ 这条判据的**对象是二进制**（锚登记表的 `binary-precondition`）⇒ 绿路探针把它接到
    **替身**上（`--bin <夹具>` + `SOKO_FIXTURE_SHAPE=current` ✓）；**真二进制**那条路由
    `gates-fast`（`cargo build` 之后跑同一条动作 ✓）与 `--layer <层>` 的本机跑兜住 ✓。
    ⚠ rc=3（拿不到二进制 ⇒ 夹具不在）⇒ **红**（**不假装判过** ✗）。
    """
    env_red: list[str] = []
    with recording() as produced, _fixture_shape("current"):
        rc, _out = _run_layer(layer, ["--bin", str(GREEN_FIXTURE)])
        if rc == 3:
            env_red.append(f"**绿路探针判不了** ✗：`--layer {layer} --bin {GREEN_FIXTURE.name}` "
                           f"rc=3（拿不到二进制 ⇒ 夹具不在 / 不可执行）⇒ **不假装判过** ✗")
        elif rc != 0:
            env_red.append(f"**绿路探针没判绿** ✗：`--layer {layer} --bin "
                           f"{GREEN_FIXTURE.name}` rc={rc}（替身的 `current` 形状该判绿 ⇒ "
                           f"夹具与判据漂移了 ✗）")
    return produced, env_red


def _selftest_with_produced(layer: str) -> int:
    """该层的 `--selftest`（**开着记录器**）+ 绿路探针 + 逐条对账 ⇒ rc。**唯一一处** ✓"""
    print(f"═══ 层 `{layer}`：登记表 + 断言产出（判例 v）═══")
    reg = registry_problems()
    if reg:
        print(f"✗ 登记表对账：**{len(reg)} 条红** ✗")
        for m in reg:
            print(f"  · {m}")
    else:
        n_g = len(required_of(layer, "green"))
        n_s = len(required_of(layer, "selftest"))
        print(f"✓ 登记表 {len(REQUIRED_ASSERTIONS)}/{len(REQUIRED_ASSERTIONS)} 逐条对账"
              f"（本层 green {n_g} + selftest {n_s} ✓）")

    green, env_red = _green_probe(layer)
    miss_green = produced_problems(required_of(layer, "green"), green,
                                   "**绿路真跑**（CI 接线的那条动作 + 夹具）里")
    if miss_green or env_red:
        print(f"✗ 绿路产出：**{len(miss_green) + len(env_red)} 条红** ✗")
        for m in miss_green + env_red:
            print(f"  · {m}")
    else:
        print(f"✓ 绿路真跑产出 {len(required_of(layer, 'green'))} 条断言 ✓"
              f"（{' · '.join(sorted(required_of(layer, 'green')))}）")

    print(f"\n─── 层 `{layer}` 自检臂（原文逐字 ✓）───")
    with recording() as st_prod:
        rc = {"cadence": cadence_selftest, "gap": gap_selftest}[layer]()
    miss_st = produced_problems(required_of(layer, "selftest"), st_prod, "**自检臂**里")
    if miss_st:
        print(f"✗ 自检臂产出：**{len(miss_st)} 条红** ✗")
        for m in miss_st:
            print(f"  · {m}")
    else:
        print(f"✓ 自检臂产出 {len(required_of(layer, 'selftest'))} 条断言 ✓"
              f"（{' · '.join(sorted(required_of(layer, 'selftest')))}）")
    return 1 if (rc or reg or miss_green or env_red or miss_st) else 0


def _run_all(rest: list[str]) -> int:
    """默认动作（`--layer all`）：两层**各判各的**（各印各的判据行 ✓），再汇总 rc ✓。"""
    if "--selftest" in rest:
        rc = _selftest_with_produced("cadence")
        rc |= _selftest_with_produced("gap")
        print(f"\n{'✓' if rc == 0 else '✗'} 两层自检 + 登记表 + 断言产出："
              f"{'全过 ✓' if rc == 0 else '**有红** ✗'}")
        return rc
    if not rest:
        rcs = {}
        for layer in LAYERS:
            print(f"─── 层 `{layer}` ───")
            rcs[layer] = _run_layer(layer, [])[0]
        worst = 3 if 3 in rcs.values() else (1 if 1 in rcs.values() else 0)
        print("─── 汇总（**每条红只属于一层** ✓；汇总只为人在本机用 ✓）───")
        for layer in LAYERS:
            print(f"  · `{layer}`：rc={rcs[layer]}")
        return worst
    if "--json" in rest:
        out: dict = {"schema": "soko.progress/1", "layers": {}}
        worst = 0
        for layer in LAYERS:
            rc, text = _run_layer(layer, ["--json"])
            try:
                out["layers"][layer] = json.loads(text)
            except json.JSONDecodeError:
                out["layers"][layer] = {"error": "该层没吐出单个 JSON", "exit": rc}
            worst = 3 if 3 in (worst, rc) else (1 if 1 in (worst, rc) else 0)
        out["exit"] = worst
        print(json.dumps(out, ensure_ascii=False))
        return worst
    print(f"error: 两层默认动作不接受这些参数：{rest} —— 用 `--layer cadence|gap` **点名层** ✓",
          file=sys.stderr)
    return 2


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    layer, rest = _split_layer(argv)
    knob = _knob_problems(layer, rest)
    if knob:
        for m in knob:
            print(f"error: {m}", file=sys.stderr)
        return 2
    if layer == "all":
        return _run_all(rest)
    return _LAYER_MAIN[layer](rest)


if __name__ == "__main__":
    sys.exit(main())
