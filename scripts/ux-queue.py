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
python3 scripts/ux-queue.py --check-uncovered     # 覆盖自检（源码级，秒级；进 gate --fast）
python3 scripts/ux-queue.py --check-uncovered --live   # 覆盖 + 读数自检（跑探针，慢）
python3 scripts/ux-queue.py --selftest            # 判据通道自检（不碰仓库状态）
```

退出码：0 = 通过 · 1 = 有红（覆盖/读数/§0.2 不一致）· 2 = 用法或环境错误。

口径与边界：**本文件 docstring 就是规格**（不另开设计档 ⇒ 免得"文档说的"与"脚本做的"两头漂）；
门禁：`scripts/soko gate --fast` 与 CI 的 `ledger` job 都跑 `--check`（秒级）⇒ 队列与来源脱钩当场判红 ✗。
调度器（2026-10-12 用户：「不能全依赖大模型运作，而是同时依靠脚本」）：`--next` 取序 · `--claim` 认领
（写锁，别人持锁判红）· `--advance … --to done --commit … --reading …` 收口（**没证据不许收口**）·
`--add-child` 就地拆解（子任务插在父之前、父转 umbrella）· 状态 = `docs/ux-queue-events.jsonl` 的**只追加**事件折叠。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
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


class UsageError(Exception):
    """用法/环境错误（exit 2）——与"判红"（exit 1）分开，别把环境问题当结论。"""


# ── 来源 ①：缺口台账 ────────────────────────────────────────────────────────

def load_gaps() -> list[dict]:
    """`docs/gaps/ledger.jsonl` 里 `status != fixed` 的条目（fixed = 守卫输入，不动 ✗）。"""
    rows: list[dict] = []
    for lineno, line in enumerate(GAPS_LEDGER.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as err:
            raise UsageError(f"{GAPS_LEDGER}:{lineno} 不是合法 JSON：{err}") from err
        if entry.get("status") == "fixed":
            continue
        repro = entry.get("repro") or ""
        # wo-filed 是**终态**（理由在台账的 `wo`/`notes` 里）⇒ 队列里保留一行、不算 open ✓
        disposition = "wo-filed" if entry.get("status") == "wo-filed" else "scheduled"
        rows.append({
            "id": entry["id"],
            "sources": ["gap-ledger"],
            "facet": (entry.get("kind") or "tooling"),
            "severity": SEVERITY_ALIASES.get(entry.get("severity", ""), "medium"),
            "repro": f"bash {repro}" if repro.endswith(".sh") else (
                f"scripts/soko grade {repro}" if repro else "（无复现件）"),
            "disposition": disposition,
            "owner": entry.get("owner") or (entry.get("where") or {}).get("area", "") or "",
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
    """
    rows: list[dict] = []
    section = None
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        head = re.match(r"^## (\d)", line)
        if head:
            section = head.group(1)
        if not line.startswith("|"):
            continue
        cells = _cells(line)
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
    if kind == "lang":
        if rc == 0:
            return ("reproducing", tail)
        if rc == 1:
            return ("fixed", tail)
        return ("error", f"退出码 {rc}（约定只认 0/1）—— {tail}")
    # mjs 探针（lsp / editor / vim）
    if rc == 0:
        return ("fixed", tail)
    if rc == 1:
        return ("reproducing", tail)
    return ("error", f"退出码 {rc}（2 = 用法/环境，3 = 前置缺失）—— {tail}")


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

EVENTS = ROOT / "docs" / "ux-queue-events.jsonl"
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

SCHED_STATES = ("todo", "in_progress", "blocked", "done", "wo_filed", "umbrella")
TERMINAL_STATES = {"done", "wo_filed"}
# **允许的迁移表 = 规则的唯一出处**（改这里才改得动行为）
TRANSITIONS = {
    "todo":        {"in_progress", "blocked", "umbrella", "wo_filed"},
    "in_progress": {"done", "blocked", "todo", "umbrella", "wo_filed"},
    "blocked":     {"todo", "in_progress", "wo_filed"},
    "umbrella":    {"done", "wo_filed"},
    "done":        {"todo"},       # 只能靠 `--advance … --to todo --reason …`（重开）
    "wo_filed":    {"todo"},
}


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


def write_lock(claims: list[dict]) -> None:
    path = lock_file()
    if not claims:
        try:
            path.unlink()
        except OSError:
            pass
        return
    path.write_text(json.dumps({"claims": claims}, ensure_ascii=False), encoding="utf-8")


def claim_is_live(claim: dict) -> tuple[bool, str]:
    pid, age = claim.get("pid"), _age_s(claim.get("ts"))
    who = (f"{claim.get('owner') or '?'}（pid {pid}·{int(age or 0)}s 前认领 `{claim.get('id')}`"
           f"·资源 {','.join(claim.get('resources') or []) or '?'}）")
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
    """**4 核 + 资源互斥**：槽位满 ⇒ 拒；与活跃认领撞资源 ⇒ 拒（除非 `--force` 接管陈旧）。"""
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
    kept = live + [{"id": ident, "owner": owner, "pid": os.getpid(),
                    "resources": resources, "ts": _now()}]
    if stale:
        print(f"⏳ 清掉 {len(stale)} 条陈旧认领（进程已死/超 TTL）："
              f"{[c.get('id') for c in stale]}", file=sys.stderr)
    write_lock(kept)
    return True


def drop_claim(ident: str, owner: str) -> None:
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
    return state


def sched_rows(rows: list[dict], state: dict[str, dict]) -> list[dict]:
    """把**拆解出来的子任务**并进队列视图：插在它父条目的**前面**（执行顺序 = 队列顺序）。"""
    kids: dict[str, list[str]] = {}
    for ident, cur in state.items():
        par = cur.get("parent")
        if par:
            kids.setdefault(par, []).append(ident)
    child_ids = [c for kids_of in kids.values() for c in kids_of]
    extra = {ident: {"id": ident, "facet": "队列", "severity": "medium", "disposition": "fix",
                     "repro": f"（拆解自 `{state[ident].get('parent')}`："
                              f"{state[ident].get('title') or ''}）",
                     "owner": state[ident].get("owner") or "", "where": _label(EVENTS)}
             for ident in child_ids}
    out: list[dict] = []
    seen: set[str] = set()
    for row in rows:
        for ident in kids.get(row["id"], []):
            if ident not in seen:
                out.append({**extra[ident], "id": ident})
                seen.add(ident)
        if row["id"] not in seen:
            out.append(row)
            seen.add(row["id"])
    for ident, row in extra.items():          # 父条目不在队列里（异常形状）也别丢
        if ident not in seen:
            out.append(row)
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
    append_event({"event": "claim", "id": ident, "owner": owner, "pid": os.getpid(),
                  "resources": resources})
    slots = len(live_claims())
    print(f"✓ 认领 `{ident}`（owner={owner} · pid {os.getpid()} · 资源 {','.join(resources)} · "
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
    if to in {"wo_filed", "blocked"} and not reason:
        print(f"✗ `{to}` 必须写 `--reason '<为什么>'` ✗", file=sys.stderr)
        return 1
    if frm == "in_progress" and to in {"done", "wo_filed", "blocked", "todo"}:
        held = [c for c in read_lock()["claims"] if c.get("id") == ident]
        if held and held[0].get("owner") != owner and not force:
            print(f"✗ `{ident}` 被 `{held[0].get('owner')}` 认领着 ⇒ 别替别人收口 ✗",
                  file=sys.stderr)
            return 1
        drop_claim(ident, held[0].get("owner") if held else owner)
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


def cmd_release(owner: str, reason: str) -> int:
    """放掉 `owner` 的**全部**认领（4 核里的自己那几格）⇒ 条目回 todo。"""
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
    return {"fix": "todo", "scheduled": "todo", "fixed": "done",
            "wo-filed": "wo_filed", "boundary": "boundary", "control": "control"}.get(
        row.get("disposition"), "todo")


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


def check_plan(rows: list[dict]) -> int:
    """**调度器不变量**：状态与来源不许漂移、收口必须有证据、umbrella 不许空、锁要新鲜。"""
    state = fold()
    bad = 0
    known = {r["id"] for r in rows}
    disp = {r["id"]: r.get("disposition") for r in rows}
    for ident, cur in sorted(state.items()):
        st = cur.get("state")
        if ident not in known:
            print(f"✗ 事件里的 `{ident}` 不在任何来源里 ⇒ 幽灵条目（删来源了？）✗", file=sys.stderr)
            bad += 1
            continue
        if st == "done":
            if not (cur.get("commit") and cur.get("reading")):
                print(f"✗ `{ident}` 标了 done 却没有 `commit`+`reading` ⇒ **没证据的收口** ✗",
                      file=sys.stderr)
                bad += 1
            if disp.get(ident) not in {"fixed", "wo-filed"}:
                print(f"✗ `{ident}` 调度器里 done，来源处置却是 `{disp.get(ident)}` ⇒ "
                      f"**两层漂移**（把来源台账改成 fixed/wo-filed）✗", file=sys.stderr)
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
    return bad


# ── 输出 ───────────────────────────────────────────────────────────────────

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
            rc = quiet(cmd_advance, "X-1", "done", "", "", "", "t", False, rows)
            expect(f"无证据收口必须被拒（rc={rc}）", rc == 1)
            rc = quiet(cmd_advance, "X-1", "done", "deadbeef", "probe exit 0→1", "", "t",
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
            rc = quiet(cmd_advance, "X-1", "done", "deadbeef", "读数", "", "t", True, view)
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
            expect("释放 w3 的认领", quiet(cmd_release, "w3", "测完") == 0)
            # 在 ⑧ 里 w3 `--force` 接管了 mod/a（把 w1 挤掉）⇒ 释放 w3 后应只剩 w2 的 mod/b
            expect(f"释放后只剩 w2（{len(live_claims())}）",
                   [c["owner"] for c in live_claims()] == ["w2"])
            write_lock([])
            # ⑩ --next 跳过终态 / 对照臂
            nxt = io.StringIO()
            with contextlib.redirect_stdout(nxt):
                cmd_next(view, "t")
            expect("--next 不吐终态/对照",
                   not nxt.getvalue().startswith(("X-2", "X-3")))
        finally:
            EVENTS = saved_events
            globals()["lock_file"] = saved_lock


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

    # ② §0.2 解析：真文件必须读得出 ≥1 行（否则"唯一入口"名存实亡）
    try:
        ids = load_onboarding_queue()
        expect("§0.2 能读出队列行", len(ids) >= 1)
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

    # ③b 调度器判据（非法迁移 / 无证据 / umbrella / 锁 / --next）
    selftest_scheduler(expect)

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
          f" + **调度器 20 条**（非法迁移/无证据收口/两层漂移/重开要理由/umbrella 子在开不许收口/"
          f"**4 核槽位满拒第 5 个**/**同资源互斥**/死进程认领不算活跃且可 `--force` 接管/`--release` 只放自己的/"
          f"--next 跳过终态与对照）+ 真来源解析 全部成立 ✓")
    return 0


def _default_owner() -> str:
    import getpass
    import socket
    return os.environ.get("SOKO_OWNER") or f"{getpass.getuser()}@{socket.gethostname().split('.')[0]}"


def main() -> int:
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
    parser.add_argument("--commit", metavar="SHA", help="收口证据①：commit")
    parser.add_argument("--reading", metavar="TEXT", help="收口证据②：判据读数")
    parser.add_argument("--reason", metavar="TEXT", help="wo_filed / blocked / 重开的理由")
    parser.add_argument("--add-child", metavar="PARENT", help="就地拆解：给父条目加一个子任务")
    parser.add_argument("--child", metavar="ID", help="子任务 id（配 --add-child）")
    parser.add_argument("--title", metavar="TEXT", help="子任务标题（配 --add-child）")
    parser.add_argument("--release", action="store_true", help="放锁（认领作废，条目回 todo）")
    parser.add_argument("--owner", default=_default_owner(), help="认领者（默认 $SOKO_OWNER 或 用户@主机）")
    parser.add_argument("--force", action="store_true", help="接管**陈旧**锁（进程已死或超 TTL）")
    args = parser.parse_args()
    try:
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
        if args.release:
            return cmd_release(args.owner, args.reason or "")
        if args.next:
            return cmd_next(view, args.owner)
        if args.render:
            return cmd_render(view)
        if args.check_plan or args.check:
            bad = check_plan(view)
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
            print(json.dumps({
                "summary": summary(rows),
                "open": sorted(open_set(rows)),
                "queue": rows,
                "state": state,
                "view": view,
            }, ensure_ascii=False, indent=2))
            return 0
        print_table(view)
        return 0
    except UsageError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
