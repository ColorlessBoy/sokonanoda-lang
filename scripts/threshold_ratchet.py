#!/usr/bin/env python3
"""**阈值 ratchet（横向）**：把"照现状设的装饰性上限"换成**排程收紧**的上限。

## 为什么要有这个文件（用户 2026-10-15【②】的**横向**落地，2026-10-16 · RO1）

【②】的原话（`docs/ux-queue-extras.jsonl` 的 `SUB-correct-the-orchestrator-incentives`）：
**"预算阈值不许'照现状设'，必须排程收紧"** —— 现状是上限贴在实测**上方** ⇒ 永远绿 ⇒
**装饰** ✗，而用户 2026-09-26 拍过板：**"不许靠抬上限达标 —— 上限方向只能是收紧"** ✗。
② 已在 `scripts/check-attention-budget.py` 落地（真 ratchet：`docs/attention-budget-ratchet.json`
+ `tighten()` + `--open-round`）。本文件把**同一个形状**用到**横向**那批阈值上 ——
**形状逐条照 ②**（不是第二套设计 ✗）：

  · **状态**（读数可查 ✓）= `docs/threshold-ratchet.json`（**tracked** ⇒ 逐轮 caps 进 git）；
  · **收紧是排程的**：`tighten()` **只看上一轮的 caps、不看实测** ✓（这是"不许照现状设"的机制本身）；
  · **单调不增**、且**永不高于种子**（种子 = 落地时那批常量 ⇒ 本次改动**一个阈值都没放松** ✓）；
  · **红线**（两半，都在 `check-threshold-ratchet.py` 里判）：
      ① **caps 没比上轮低 ⇒ 判红**（状态不变量：逐轮**严格**下降，直到 floor —— 照 ② 的
         "本轮没比上轮低 ⇒ 判红" ✓）；
      ② **读数没落进收紧后的上限 ⇒ 判红**（`--open-round` 的闸门 = "这一轮真的变好到配得上
         新一档"）。⚠ **为什么不照抄"读数必须严格低于上轮读数"**：那对**有天然地板**的量
         （STATUS.md 行数 / 归档体积 / target 体积）是**假红生成器**（它们不可能每轮都更小 ✗）；
         照搬形状、**不照搬不适用的那半条**，理由写在这里 ✓（用户【④】："同形不同病要如实标注"）。
  · **接缝**（`seam_problems()`，2026-10-16 补第三半）：A 层产出 caps、B 层消费 ⇒ ① **7 个
    消费者脚本**必须真的读 ratchet（`_RATCHET_CAPS["<id>"]` 标记 ✓）；③ **标签里不许写
    上限数字**（`.github/workflows/ci.yml` 的 step 名 —— 改前它写着种子 `≤10.0 MB` 而现行
    caps 已是 `9.50 MB` ✗，洞 `BUDGET-WARN-VS-GATE-RED-INCONSISTENT` 的同族 ✓）。
    ⚠ ⭐ **① 的"真的读"读的是哪一层**（第 1 轮 `gs1` 收窄，见 `command_layer()` 段头）：
    **命令层**（`.yml` = `run:` 正文 · 其余 = 活行）—— 收窄前是**整份源码的子串命中**
    ⇒ **注释里提一句**就能通过 ✗（**提及 ≠ 读**）⇒ 收窄后那条路**判红**（**①b**）✓。

## 登记表（**唯一一处**：种子 / 每轮步长 / 收敛目标 / 消费点）

⚠ **种子 = 落地那一刻的常量**（逐条给了 `文件:行`）——`tighten()` 永不越过它 ⇒
"一个阈值都没放松"是**结构**保证的 ✓，不是承诺。

| id | 种子 | 每轮 − | 收敛目标（floor） | 消费点 | 落地时实测 |
|---|---|---|---|---|---|
| `docs_live_bytes` | 10.00 MB | 0.50 MB | **3.00 MB** | `scripts/docs-lint.py` ① | 6.41 MB |
| `docs_archive_bytes` | 2.50 MB | 0.25 MB | 1.00 MB | `scripts/docs-lint.py` ⑥ | 0.95 MB |
| `status_total_lines` | 240 行 | 10 行 | **200 行** | `scripts/status-lint.py` ① | 85 行 |
| `site_text_bytes` | 120 KB | 12 KB | 75 KB | `scripts/check-site.py` assets | 60,823 B |
| `site_hero_bytes` | 400 KB | 40 KB | 240 KB | `scripts/check-site.py` assets | 221,763 B |
| `target_incremental_warn_bytes` | 20 GiB | 2 GiB | 6 GiB | `scripts/target-hygiene.py` 告警线 | 3.50 GiB |
| `target_incremental_max_bytes` | 60 GiB | 6 GiB | 18 GiB | `scripts/target-hygiene.py` 红线 | 3.50 GiB |

⭐ **两个 floor 不是随手取的**：`docs_live_bytes` 的 **3 MB** = 用户 2026-09-26 点名的那条
（"不许从 3 MB 抬到 10 MB"）⇒ ratchet 的**终点就是把它收回 3 MB** ✓；`status_total_lines`
的 **200** = `scripts/status-lint.py` 自己注释里写的目标（"后续统一 refactor 时…再把上限调回
200"）✓。其余 floor 取"贴着落地实测之上一点点"（照 ② 学的先例：`recompile-budget.json`
`max_by_calls=1` 而实测 1 ⇒ **正好贴线**才是活的阈值 ✓）。

## ⚠ 如实标注：**同形不同病**的那一处（`scripts/check-progress-gap.py:44`）

`MAX_GAP_S = 2.5`（实测 0.12s）看起来也是"装饰性阈值"，但它是**活性界（墙钟秒）**，
**不该**照搬 ratchet ✗ —— 理由（`AGENTS.md`「判据的两条硬规矩」②：**perf 判据不许用绝对毫秒**；
同一处已有**三次**假红实测：同代码 44ms ↔ 2431ms · 本机 37.37s vs CI 278.58s（7.5×））：
  · 它量的是"**心跳的最长无输出间隔**"——**共享 runner 上墙钟不可转移** ⇒ 按轮收紧 =
    **造一台假红生成器** ✗（而 ratchet 的语义是"红要有出路"）；
  · 它的红线口径本来就是"**数量级兜底**"（心跳 1s ⇒ 2.5s = 2.5× 余量），不是"逼它变好"；
  · 它的**出路**在别处（`build.tick`/`build.decl` 两条通道的**结构计数**，
    见 `docs/protocol.md` 的 P2 一节）。
⇒ 本轮**登记它、不动它**（`EXCLUDED` 那张表就是这条登记 ✓）—— ⚠ **不硬套** ✓。

## 用法

```bash
python3 scripts/check-threshold-ratchet.py                 # 状态不变量 + 开轮纪律（秒级）
python3 scripts/check-threshold-ratchet.py --measure       # 另打印 7 个读数
python3 scripts/check-threshold-ratchet.py --open-round    # 开一轮（要求读数落进收紧后的上限）
python3 scripts/check-threshold-ratchet.py --selftest      # 判据通道自检（正反两向）
```

本文件是**库**（登记表 + `tighten()` + 状态读写）；**判红**在 `scripts/check-threshold-ratchet.py`
（一处定义：消费点读 `cap()`，判据读同一份状态 ✓）。
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

ROOT = Path(__file__).resolve().parent.parent
STATE = ROOT / "docs" / "threshold-ratchet.json"
GIB = 1024 ** 3


class RatchetError(Exception):
    """状态缺失 / 坏 / 字段不全 ⇒ **响亮**（**绝不静默判过** ✗，与 ② 同口径）。"""


# ── 登记表（**唯一一处**）────────────────────────────────────────────────────
# ⚠ 加一条 = ① 这里加一行（种子/步长/floor/消费点）② 消费点读 `cap("<id>")`
#   ③ `check-threshold-ratchet.py` 的**接缝判据**（`SEAM`）里加标记 ④ `docs/threshold-ratchet.json`
#    重新 `--init`（round 0 的 caps = 种子）⇒ 漏一处会被 `--selftest` 的结构性臂先咬住 ✓。
REGISTRY: dict[str, dict] = {
    "docs_live_bytes": {
        "seed": 10_000_000, "step": 500_000, "floor": 3_000_000,
        "divisor": 1_000_000, "unit": "MB", "decimals": 2,
        "site": "scripts/docs-lint.py:58 `MAX_LIVE_BYTES`", "reading": "6.41 MB",
        "seam": ('_RATCHET_CAPS["docs_live_bytes"]', "scripts/docs-lint.py"),
    },
    "docs_archive_bytes": {
        "seed": 2_500_000, "step": 250_000, "floor": 1_000_000,
        "divisor": 1_000_000, "unit": "MB", "decimals": 2,
        "site": "scripts/docs-lint.py:64 `MAX_ARCHIVE_BYTES`", "reading": "0.95 MB",
        "seam": ('_RATCHET_CAPS["docs_archive_bytes"]', "scripts/docs-lint.py"),
    },
    "status_total_lines": {
        "seed": 240, "step": 10, "floor": 200,
        "divisor": 1, "unit": "行", "decimals": 0,
        "site": "scripts/status-lint.py:32 `MAX_TOTAL`", "reading": "85 行",
        "seam": ('_RATCHET_CAPS["status_total_lines"]', "scripts/status-lint.py"),
    },
    "site_text_bytes": {
        "seed": 120 * 1024, "step": 12 * 1024, "floor": 75 * 1024,
        "divisor": 1024, "unit": "KB", "decimals": 1,
        "site": "scripts/check-site.py:56 `SIZE_BUDGET`", "reading": "60,823 B",
        "seam": ('_RATCHET_CAPS["site_text_bytes"]', "scripts/check-site.py"),
    },
    "site_hero_bytes": {
        "seed": 400 * 1024, "step": 40 * 1024, "floor": 240 * 1024,
        "divisor": 1024, "unit": "KB", "decimals": 0,
        "site": "scripts/check-site.py:63 `HERO_BUDGET`", "reading": "221,763 B（两版取大）",
        "seam": ('_RATCHET_CAPS["site_hero_bytes"]', "scripts/check-site.py"),
    },
    "target_incremental_warn_bytes": {
        "seed": 20 * GIB, "step": 2 * GIB, "floor": 6 * GIB,
        "divisor": GIB, "unit": "GiB", "decimals": 2,
        "site": "scripts/target-hygiene.py:92 `MAX_INCREMENTAL_WARN`", "reading": "3.50 GiB",
        "seam": ('_RATCHET_CAPS["target_incremental_warn_bytes"]', "scripts/target-hygiene.py"),
    },
    "target_incremental_max_bytes": {
        "seed": 60 * GIB, "step": 6 * GIB, "floor": 18 * GIB,
        "divisor": GIB, "unit": "GiB", "decimals": 2,
        "site": "scripts/target-hygiene.py:93 `MAX_INCREMENTAL_BYTES`", "reading": "3.50 GiB",
        "seam": ('_RATCHET_CAPS["target_incremental_max_bytes"]', "scripts/target-hygiene.py"),
    },
}

# **不许进 ratchet 的同形项**（如实登记 + 逐条理由；"登记了但没改"不是处置 ✗ —— 这里就是处置 ✓）。
EXCLUDED: dict[str, str] = {
    "scripts/check-progress-gap.py:44 `MAX_GAP_S = 2.5`（实测 0.12s）":
        "**同形不同病**：它是**活性界（墙钟秒）**，而 `AGENTS.md` 明文「perf 判据不许用绝对毫秒」"
        "（三次假红实测：同代码 44ms↔2431ms · 本机 37.37s vs CI 278.58s）⇒ 按轮收紧 = "
        "造一台**假红生成器** ✗（共享 runner 上墙钟不可转移）。它的出路是**结构计数**"
        "（`build.tick`/`build.decl`，见 `docs/protocol.md` P2），不是 ratchet ✓。",
    "scripts/target-hygiene.py:88/90 `MAX_RCGU_O_WARN = 250_000` / `MAX_TARGET_BYTES_WARN = 40 GB`":
        "**已经是活的**（实测 230,813 = +8% · 38.5 GB = +4% ⇒ **贴线**）⇒ 不是装饰性阈值 ✗ "
        "（照 ② 学的先例：`recompile-budget.json` `max_by_calls=1` 而实测 1 ✓）。",
}


def human(cap_id: str, value) -> str:
    """读数 / 上限的人话（消息里一律用它 ⇒ 单位不会两处各写一遍 ✓）。"""
    spec = REGISTRY[cap_id]
    if spec["divisor"] == 1:                     # 行数（无量纲）⇒ 不除
        return f"{value:,.0f} {spec['unit']}"
    return f"{value / spec['divisor']:,.{spec['decimals']}f} {spec['unit']}"


def seeds() -> dict:
    """第 0 轮的 caps = **落地那一刻的常量**（= 上界：`tighten()` 永不越过它 ✓）。"""
    return {k: v["seed"] for k, v in REGISTRY.items()}


def tighten(caps: dict) -> dict:
    """**排程收紧一档**。**纯函数** ✓（selftest 直接喂合成 caps）。

    ⚠ **只看上一轮的 caps、不看实测** ✓ —— 这是"不许照现状设"✗→✓ 的机制本身。
    ⚠ **单调不增**：新 caps 的每一档都 `<=` 旧的对应档（floor 只**停住**收紧，
      不会把已经收下去的抬回来 ✓）⇒ **方向只有收紧** ✓。
    """
    out = {}
    for cap_id, spec in REGISTRY.items():
        cur = caps.get(cap_id, spec["seed"])
        out[cap_id] = max(spec["floor"], cur - spec["step"])
    return out


def converged(caps: dict) -> bool:
    """全部 caps 都已到 floor ⇒ ratchet 收敛（**这时不许再判"空转"** ✗ —— 那会是一条
    **没有出路**的红：`--open-round` 也降不下去了 ✓）。"""
    return all(caps.get(k, v["seed"]) <= v["floor"] for k, v in REGISTRY.items())


def state_path() -> Path:
    """现行状态文件。`SOKO_RATCHET_STATE` 只用于**复现 / 咬住证明**（指到别的状态文件）。

    ⚠ **它不是旁路** ✓：① 状态里的 caps **高于种子** ⇒ `caps_or_error()` 判红 + 消费点**响亮
    退出**（**任何状态文件都不可能把上限抬到种子之上** ⇒ 方向只有收紧 ✓）；② 低于收敛目标
    同样响亮 ⇒ 指到哪儿都只会**更严**，不会更松 ✓。
    """
    return Path(os.environ.get("SOKO_RATCHET_STATE") or STATE)


def state_load(path: Path | None = None) -> dict:
    """读状态。**缺失/坏/字段不全 ⇒ `RatchetError`**（**绝不静默判过** ✗）。"""
    path = path or state_path()
    if not path.exists():
        raise RatchetError(
            f"ratchet 状态不存在：`{path}` ⇒ **不假装判过** ✗。落地那一次用 "
            f"`python3 scripts/check-threshold-ratchet.py --init` 写第 0 轮基线 ✓")
    try:
        st = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as err:
        raise RatchetError(f"ratchet 状态读不了：`{path}`：{err}") from err
    for key in ("round", "head", "caps", "history"):
        if key not in st:
            raise RatchetError(f"ratchet 状态缺字段 `{key}`：`{path}`")
    return st


def state_save(st: dict, path: Path | None = None) -> None:
    path = path or state_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(st, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def caps_or_error(path: Path | None = None) -> tuple[dict, str | None]:
    """**消费点用这一条**：返回 `(caps, 错误)`。

    ⚠ 状态坏时返回的是**种子**（模块还得能 import），**但错误非空** ⇒ 调用方**必须**在入口
    响亮退出（`main()` 的头几行）—— **绝不静默退回种子判过** ✗（与 ② 的"缺失/坏 ⇒ 错误"同口径）。
    ⚠ 判"坏"用的是**同一份** `state_problems()`（状态不变量的唯一出处 ✓）：caps 高于种子 /
    低于收敛目标 / 逐轮不降 / `round` 与 `history` 不自洽 ⇒ 一律响亮 ✓。
    """
    try:
        st = state_load(path)
    except RatchetError as err:
        return seeds(), str(err)
    problems = state_problems(st)
    if problems:
        return seeds(), ("ratchet 状态不健康 ⇒ **不假装判过** ✗：" + "；".join(problems)
                         + f"（修：`python3 scripts/check-threshold-ratchet.py` 看红 / "
                           f"`--init` 重建第 0 轮）")
    caps = dict(seeds())
    caps.update({k: v for k, v in (st.get("caps") or {}).items() if k in REGISTRY})
    return caps, None


def cap(cap_id: str, path: Path | None = None) -> int:
    """单个上限（消费点直接调它 —— `MAX_LIVE_BYTES = threshold_ratchet._RATCHET_CAPS["docs_live_bytes"]`）。

    ⚠ 坏状态**不抛**（import 期抛会变成栈 ✗）：退回种子 + 由调用方的入口闸门响亮退出 ✓。
    """
    return int(caps_or_error(path)[0][cap_id])


# ── 状态不变量（**唯一一处**；`check-threshold-ratchet.py` 与 `--selftest` 都读它）──
def state_problems(st: dict) -> list[str]:
    """**纯函数**：状态本身的问题（红线 ①：**caps 没比上轮低 ⇒ 判红** ✓）。

      · `caps` **永不高于种子**（"一个阈值都没放松"的机械面 ✓）；
      · 逐轮**严格下降**（到 floor 为止）—— 照 ② 的"本轮没比上轮低 ⇒ 判红" ✓；
      · `warn < max` 这一类**同族关系**（incremental 两条）逐轮成立；
      · `round` / `history` 自洽（round = len(history) − 1、history 末条的 caps = 现行 caps）。
    """
    red: list[str] = []
    seed = seeds()
    caps = st.get("caps") or {}
    for cap_id, spec in REGISTRY.items():
        if cap_id not in caps:
            red.append(f"`{cap_id}` 不在状态的 `caps` 里 ⇒ 消费点会退回种子判过 ✗")
            continue
        if caps[cap_id] > seed[cap_id]:
            red.append(f"`{cap_id}` = {human(cap_id, caps[cap_id])} **高于种子** "
                       f"{human(cap_id, seed[cap_id])} ⇒ **放松了阈值** ✗（用户 2026-09-26："
                       f"上限方向只能是收紧）")
        if caps[cap_id] < spec["floor"]:
            red.append(f"`{cap_id}` = {human(cap_id, caps[cap_id])} **低于收敛目标** "
                       f"{human(cap_id, spec['floor'])} ⇒ 状态被手改过 ✗")
    # 同族关系：incremental 的**告警线必须低于红线**（否则告警永远先响、红线成摆设 ✗）
    if "target_incremental_warn_bytes" in caps and "target_incremental_max_bytes" in caps:
        if caps["target_incremental_warn_bytes"] >= caps["target_incremental_max_bytes"]:
            red.append("incremental **告警线 ≥ 红线** ✗（20 GiB/60 GiB 的族关系被破坏）")
    hist = st.get("history") or []
    if int(st.get("round", -1)) != len(hist) - 1:
        red.append(f"`round` = {st.get('round')} 与 `history` 长度 {len(hist)} 不自洽 ✗"
                   f"（round 应 = len(history) − 1）")
    prev = None
    for idx, entry in enumerate(hist):
        cur = entry.get("caps") or {}
        for cap_id, spec in REGISTRY.items():
            if cap_id not in cur:
                red.append(f"history 第 {entry.get('round')} 轮缺 `{cap_id}` ✗")
                continue
            if prev is not None and cap_id in prev:
                if cur[cap_id] > prev[cap_id]:
                    red.append(f"**caps 没比上轮低** ✗：`{cap_id}` 第 {entry.get('round')} 轮 "
                               f"{human(cap_id, cur[cap_id])} > 上一轮 "
                               f"{human(cap_id, prev[cap_id])} ⇒ 状态被手改过")
                elif cur[cap_id] == prev[cap_id] and cur[cap_id] > spec["floor"]:
                    red.append(f"**caps 没比上轮低** ✗：`{cap_id}` 第 {entry.get('round')} 轮与"
                               f"上一轮都停在 {human(cap_id, cur[cap_id])}（还没到收敛目标 "
                               f"{human(cap_id, spec['floor'])}）⇒ ratchet 空转了 ✗")
        # ⭐ **开轮闸门的复查**（`open_round` 要求"读数落进**收紧后**的上限"，而它在开这一轮时
        # 收紧出来的就是**这一轮自己的** `caps`（`open_round` 里 `nxt = tighten(st["caps"])`
        # 然后 `entry["caps"] = nxt`）⇒ 复查要拿 `cur`（本轮 caps）比，**不是** `hist[idx+1]` ✗。
        # ⚠ 2026-10-16 实测更正：原写法比的是**下一轮**的 caps ⇒ **off-by-one** ✗ ——
        # 第 3 轮开轮时闸门只要求 `<= 8.5 MB`（它自己的 caps ✓），而复查拿第 4 轮的 8.0 MB 去比
        # ⇒ 把"收紧的正常后果"判成了"那一轮本来开不成" ✗（逐轮读数其实**自洽** ✓）。
        for cap_id in REGISTRY:
            val = (entry.get("reading") or {}).get(cap_id)
            if val is not None and cap_id in cur and val > cur[cap_id]:
                red.append(f"**history 第 {entry.get('round')} 轮的读数越界** ✗：`{cap_id}` "
                           f"实测 {human(cap_id, val)} > 那一轮收紧后的上限 "
                           f"{human(cap_id, cur[cap_id])} ⇒ 那一轮**本来开不成**"
                           f"（history 被手改过 / `open_round` 的闸门被绕过）")
        prev = cur
    if hist and (hist[-1].get("caps") or {}) != caps:
        red.append("`caps` 与 `history` 末轮不一致 ✗（同一份真相不许两处各写一份）")
    return red


def open_round(st: dict, reading: dict, now: str, head: str) -> tuple[dict, list[str]]:
    """**纯函数**：开一轮 ⇒ `(新状态, 判红)`。红非空 ⇒ 状态**不动** ✗。

    闸门（顺序 = 判红信息的顺序）：
      ① **读数必须齐全**（7 个一个不缺 —— 缺 ⇒ "没量到 ≠ 绿" ✗）；
      ② ⭐ **读数必须落进"收紧后"的上限**（= "这一轮真的变好到配得上新一档"）；
      ③ 过了才 `tighten()` 一档 + 记 `history`（**读数进 git** ⇒ 逐轮可查 ✓）。
    """
    red: list[str] = []
    missing = [k for k in REGISTRY if k not in reading]
    if missing:
        red.append(f"**读数不全** ✗：缺 {missing} ⇒ 「没量到 ≠ 绿」（ratchet 不许拿半个读数开轮）")
    nxt = tighten(st["caps"])
    for cap_id in REGISTRY:
        if cap_id not in reading:
            continue
        if reading[cap_id] > nxt[cap_id]:
            red.append(f"**本轮没变好** ✗：`{cap_id}` 实测 {human(cap_id, reading[cap_id])} "
                       f"**高于收紧后的上限** {human(cap_id, nxt[cap_id])}"
                       f"（上一轮上限 {human(cap_id, st['caps'][cap_id])}）⇒ 开不了下一轮。"
                       f"出路：把它压下去（删/归档文档 · 回收 target · 缩短 STATUS）✓")
    if red:
        return st, red
    round_no = int(st.get("round", 0)) + 1
    entry = {"round": round_no, "head": head, "at": now, "caps": nxt, "reading": dict(reading)}
    new = dict(st)
    new.update({"round": round_no, "head": head, "at": now, "caps": nxt,
                "history": list(st.get("history") or []) + [entry]})
    return new, []


def judge_idle(st: dict, n_since: int, round_max: int) -> list[str]:
    """**纯函数**：开轮纪律的机械面 —— 基点之后落太多笔还没开轮 ⇒ 判红 ✗（ratchet 不许空转）。

    ⚠ **收敛之后不再判**（全部 caps 已到 floor ⇒ 再开轮也降不下去 ⇒ 判红会是一条**没有出路**
    的红 ✗ —— 与 ② 的"红必须有出路"同一条纪律 ✓）。
    """
    if converged(st.get("caps") or {}) or n_since <= round_max:
        return []
    return [f"**ratchet 空转** ✗：第 {st.get('round')} 轮的基点 "
            f"`{str(st.get('head'))[:12]}` 之后落了 **{n_since} 笔**（> `ROUND_MAX_COMMITS = "
            f"{round_max}`）还没开轮 ⇒ 阈值停在原地 = **装饰** ✗。出路："
            f"`python3 scripts/check-threshold-ratchet.py --open-round` ✓"
            f"（它要求**读数落进收紧后的上限** ⇒ 红有出路：把指标压下去 ✓）"]


# ── **读哪一层**（洞 `SUB-narrow-guards-to-one-layer` · 第 1 轮 `gs1`；**唯一一处** ✓）──
# 病（**实测**）：两条接缝判据（本文件的 `seam_problems()` ① 与
# `scripts/check-attention-budget.py` 的 `seam_problems()` ①）的契约都写着
# 「**每个消费点必须真的读 ratchet**」，而读法都是**整份源码的子串命中** ——
# **注释里提一句**（或 `.yml` 里只在 step **名**里点名）就能通过 ✗ ⇒ **判据比错对象**
# （与 `ng7` 收窄的 R1/C4 **同族**：那里是"**提及 ≠ 跑**"，这里是"**提及 ≠ 读**"）。
# ⇒ 口径收窄成**一层**（用户 2026-10-16「收窄成一层」= **一层一个关注点** ✓）：
#   **那一层里写的就是要执行的东西** ✓
#     · `.yml` ⇒ **`run:` 正文**（YAML 里只有 `run:` 里的是命令 —— 与 `wired_scripts()`
#       / C4 的"真的有一跑"**同一条口径** ✓）；
#     · 其余（`.py` / `.sh` / 无扩展名的 JS）⇒ **活行**（非整行注释 —— Python / shell 里
#       活行就是代码 / 命令 ✓）。
# ⚠ **如实划的边界** ✗：活行**不等于**"真调" —— Python 的字符串字面量、JS 的
#   `path.join(...)` 常量都算活行（**不解析语言**，照 C11/C5/C10 的「不解析 shell」先例 ✓）。
#   这一档**印出来**、不判红 ✓（宁可少判，不要假红 ✗）。
_RUN_KEY_RE = re.compile(r"^(\s*)(?:-\s*)?run:\s*(.*)$")


def live_lines(text: str) -> str:
    """**活行** = 去掉整行注释（`#` 或 `//` 开头）⇒ 注释里点名**不算读/调用** ✗（唯一一处 ✓）。

    ⚠ `//` 那一半是给 `scripts/soko`（JS，实测 **712** 行以 `//` 开头）用的；
    `ci.yml` / `ci-local.sh` 里以 `//` 开头的行 = **0**（实测）⇒ 对它们与只剥 `#` 逐字节同读 ✓。
    """
    return "\n".join(ln for ln in text.splitlines()
                     if not (ln.strip().startswith("#") or ln.strip().startswith("//")))


def run_bodies(text: str) -> str:
    """`ci.yml` 里**每个 `run:` 的正文**（行内 + `|`/`>` 块）。**唯一一处** ✓。

    ⚠ 为什么要挑出 `run:` 而不是扫全文（**实测** ✗）：`ci.yml` 的**注释**里到处点名
    `scripts/*.py`（`perf-compare.py` / `threshold_ratchet.py` 只在注释里出现 ⇒ 扫全文
    会把**没人调用**的文件算进口径 ✗）。step 名同理（`name:` 不是命令 ✓）。
    """
    out: list[str] = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        m = _RUN_KEY_RE.match(lines[i])
        if not m:
            i += 1
            continue
        indent = lines[i].index("run:")
        rest = m.group(2).strip()
        i += 1
        if rest and not rest.startswith(("|", ">")):
            out.append(rest)                     # 行内命令：`run: python3 scripts/x.py`
            continue
        while i < len(lines):                    # 块：收比 `run:` 缩进更深的行 ✓
            ln = lines[i]
            if ln.strip() == "":
                out.append("")
                i += 1
                continue
            if len(ln) - len(ln.lstrip()) <= indent:
                break
            out.append(ln)
            i += 1
    return "\n".join(out)


def command_layer(text: str, rel: str) -> str:
    """**命令层**：`rel` 那种文件里"写的就是要执行的东西"的那一层 ✓（**唯一一处** ✓）。

    两条接缝判据（本文件 ① 与 `check-attention-budget.py` ①）**共用**这一层定义 ⇒
    同一关注点只有**一处**维护成本（`SUB-narrow-guards-to-one-layer` ② ✓）。
    """
    return run_bodies(text) if rel.endswith((".yml", ".yaml")) else live_lines(text)


# ── 接缝（**A 层产出、B 层消费** ⇒ 必须有 A∖B 对账；照 `audit-wire-fields.py` 的先例 ✓）──
def _read_source(root: Path, rel: str, source_rev: str | None) -> tuple[str | None, str | None]:
    """读一个消费点的源码 ⇒ `(源码, 错误)`。`source_rev` ⇒ 从**历史版本**读（咬住证明用 ✓）。

    ⚠ `git show` 失败 ⇒ **错误**（**不假装判过** ✗）—— 判据、`--source-rev` 与标签接缝**共用**
    这一条（**一处定义** ✓，改前那段读源码的代码在两处各写了一遍 ✗）。
    """
    if source_rev:
        proc = subprocess.run(["git", "show", f"{source_rev}:{rel}"], cwd=str(root),
                              capture_output=True, text=True, encoding="utf-8",
                              errors="replace", env=git_env.clean_git_env())
        if proc.returncode != 0:
            return None, (f"`git show {source_rev}:{rel}` rc={proc.returncode} ⇒ 量不到 ⇒ "
                          f"**不假装判过** ✗")
        return proc.stdout, None
    path = root / rel
    if not path.exists():
        return None, f"消费点 `{rel}` 不存在 ⇒ **量不到 ≠ 绿** ✗"
    return path.read_text(encoding="utf-8"), None


# ── **标签接缝**：caps 不许在**标签**里写第二遍（同一洞的横向那一半 ✓）──────────────
# 病（2026-10-16 实测，洞 `BUDGET-WARN-VS-GATE-RED-INCONSISTENT` 的同族）：**会随时间收紧的**
# 数字被写在**第二处** ⇒ 状态收紧之后第二处**不动** ⇒ 印的是旧值 ✗。
#   · 簿记那个 ratchet 的实例 = `scripts/ci-local.sh` 的 step 标签（已修 + `--seam` 咬住 ✓）；
#   · **本 ratchet 的实例 = `.github/workflows/ci.yml` 的 step 名**（实测 2026-10-16）：
#     `:90` 写 `活文档 ≤10.0 MB`（= **种子**）而现行 caps 已是 **9.50 MB** ✗；
#     `:71` 写 `≤200 行`（= `status_total_lines` 的**收敛目标**）而现行 caps 是 **230** ✗。
# 判据 = 把标签行（`- name:`）里的 `(数字, 单位)` 抠出来，与**登记表里每个 id 的三种值**
# （种子 / 现行 caps / 收敛目标）逐个比 —— 撞上就是第二份真相 ✗。
# ⚠ **单位必须一致** ⇒ 静态常量**不误伤**：`入口 ≤800 行` / `设计 ≤150 行` / `每段 ≤30` /
# `净增 ≤60` 都不是任何 id 的 seed/caps/floor ⇒ **留着** ✓（它们不随轮次漂移，是另一类
# 「文档漂移」，由 `check-drift.py` 的 ① 那一族管 ✗ —— 别把两类混成一条规则 ✗
# （⚠ 2026-10-17 `gs3` 换名：原 `check-doc-drift.py` 已并进 `check-drift.py` 的层① ✓）。
LABEL_SEAM: tuple[dict, ...] = (
    {"rel": ".github/workflows/ci.yml",
     "why": "step 名是**评审/用户在 job 列表里看得见**的一处 ⇒ 写上 ratchet 的值就是第二份真相"},
)
_NUM_UNIT = re.compile(r"([0-9]+(?:\.[0-9]+)?)\s*(MB|KB|GiB|GB|行)")


def _shown_value(cap_id: str, value) -> float:
    """`human()` 印出来的那个数（把它再抠回来 ⇒ 比较用的是**同一份**格式化 ✓）。"""
    return float(human(cap_id, value).split()[0].replace(",", ""))


def label_cap_hits(src: str, caps: dict | None = None) -> list[str]:
    """标签行里**写着 ratchet 的哪个值**（种子 / 现行 caps / 收敛目标）⇒ 命中即第二份真相 ✗。

    **纯函数** ✓（selftest 直接喂合成源码）。`caps=None` ⇒ 现读状态（读不到 ⇒ 退回种子，
    仍能咬住"标签写着种子"那一半 ✓）。
    """
    if caps is None:
        caps = caps_or_error()[0]
    hits: list[str] = []
    for ln in src.splitlines():
        if not ln.lstrip().startswith("- name:"):
            continue
        for num, unit in _NUM_UNIT.findall(ln):
            val = float(num)
            for cap_id, spec in REGISTRY.items():
                if spec["unit"] != unit:
                    continue
                for kind, v in (("种子", spec["seed"]),
                                ("现行 caps", caps.get(cap_id)),
                                ("收敛目标", spec["floor"])):
                    if v is None:
                        continue
                    if abs(_shown_value(cap_id, v) - val) < 1e-9:
                        hits.append(f"{ln.strip()[:70]} ← 写着 `{cap_id}` 的**{kind}**"
                                    f"（{human(cap_id, v)}）")
    return hits


def seam_problems(root: Path | None = None, source_rev: str | None = None,
                  caps: dict | None = None) -> list[str]:
    """**每个消费点必须真的读 ratchet**（否则阈值是装饰的：状态收紧、消费者还用常量 ✗）。

    ⚠ 判据是**两个标记**在**命令层**里（机械、可复跑 ✓）：
      ① `threshold_ratchet`（这个库真的被 import 了）；
      ② `_RATCHET_CAPS["<id>"]`（**那一个** id 真的被这个文件消费 —— 不是"碰巧 import 了"）✓。
      ③ **标签接缝**（`LABEL_SEAM` + `label_cap_hits()`）：标签里**不许**出现 ratchet 的值 ✗。
    ⚠ ⭐ **读哪一层**（第 1 轮 `gs1` 收窄；见 `command_layer()` 的段头）：**命令层** ——
      收窄前读的是**整份源码** ⇒ **注释里提一句**就能让"真的读"通过 ✗（**提及 ≠ 读**）。
      收窄的读数（真仓 · `.cache/ux/fix-gs1/measure-seam.py`）：**7** 个消费点**全部**
      在活行里命中 ⇒ 收窄**一条都不放走**（0 格 · 0 条红变绿 ✓）。
      ⚠ **收窄本身是收紧**（不是放宽 ✗）：只在**注释**里点名的消费者，收窄前**绿**、
      收窄后**判红** —— 那条红就是 **①b「提及不算读」**（与 `ng7` 的 R1b **逐字同族** ✓）。
    `source_rev`（例 `cf259d1f`）= 从**历史版本**读源码 ⇒ 用来做**咬住证明**：
    改前的消费者**没有**这个标记 ⇒ 本判据在那份**已存在的**坏输入上**必须报红** ✓。
    """
    root = root or ROOT
    red: list[str] = []
    for cap_id, spec in REGISTRY.items():
        marker, rel = spec["seam"]
        src, err = _read_source(root, rel, source_rev)
        if err:
            red.append(err)
            continue
        layer = command_layer(src, rel)
        if "threshold_ratchet" not in layer or marker not in layer:
            if marker in src:
                # ⭐ **①b「提及不算读」**（第 1 轮 `gs1`）：**提了**、**命令层里没有**
                #   ⇒ 点名就是**声明**；声明了就必须**真读** ✓（`ng7` 的 R1b 同一条形状 ✓）。
                #   ⚠ 两种形状都算"提及"：**注释里**（`#` 行）与 **`.yml` 的 step 名/env/with**
                #   （活行、但**不是命令** ✗）—— 消息里点名是哪一种（读数可归属 ✓）。
                where = ("**活行**（非注释，但**不是命令**）里点名了" if marker in live_lines(src)
                         else "**只在注释里**点名了")
                red.append(f"**①b 提及不算读** ✗：`{cap_id}` 的消费点 `{rel}` {where} "
                           f"`{marker}`，而它的**命令层里没有**它 ⇒ 那只是**提及/声明**"
                           f"（阈值还是常量，ratchet 收紧它也不动 = 装饰 ✗）。出路二选一："
                           f"①在**命令层**里真读它 ✓；②删掉那个提及"
                           f"（**注释 / step 名都不是命令** ✓）")
            else:
                red.append(f"`{cap_id}` 的消费点 `{rel}` 里找不到 `{marker}`（或没 import "
                           f"`threshold_ratchet`）⇒ **阈值还是常量**（ratchet 收紧它也不动 = 装饰 ✗）")
    # ── ③ **标签接缝**（caps 不许在标签里写第二遍 ✓）────────────────────────
    for spec in LABEL_SEAM:
        rel = spec["rel"]
        src, err = _read_source(root, rel, source_rev)
        if err:
            red.append(err)
            continue
        hits = label_cap_hits(src, caps)
        if hits:
            red.append(f"`{rel}` 的 **step 标签里写着 ratchet 的值** ✗（caps 只许住在 "
                       f"`docs/threshold-ratchet.json` / 本文件的 `REGISTRY`；标签说"
                       f"「查什么」、不说「查到多少」✓ —— {spec['why']}）—— {hits[:2]}")
    return red
