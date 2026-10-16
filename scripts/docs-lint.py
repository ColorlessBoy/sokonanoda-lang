#!/usr/bin/env python3
"""文档预算 lint（用户 2026-09-26 要求「文档太重了，还没实现多少东西文档先爆炸了」✓）
—— 见 `REQUIREMENTS.md` §9 与 `docs/design/docs-diet.md`。

**为什么** ✓：`docs/` 曾 **8.4 MB / 419 文件 / 105,753 行** ✗，而其中"活规范"只占
一小部分 —— 过程记录、已收口批次的计划、调研笔记、遗留临时物全堆在一起 ✗。
**清理一次没用** ✗：没有判据就一定回胖 ✗ ⇒ 把"文档重量"变成**机械可判** ✓。

**七条判据**（逐条可判 ✓）：
  ① **活文档总量** ≤ `MAX_LIVE_BYTES` —— 活文档 = 仓根 `*.md`（git 跟踪）
     + `docs/**`（git 跟踪），**不含** `docs/archive/**`（归档是"可追溯" ✓，
     不是"每次都要读" ✗）；
  ② **单文件** ≤ `MAX_FILE_LINES` 行（任何活文档 `.md`）；
  ③ **入口文件** ≤ `MAX_ENTRY_LINES` 行 —— 接手必读的那几份必须短 ✓；
  ④ **设计文档预算**：`docs/design/**` 的**新增** `.md` ≤ `MAX_NEW_DESIGN_LINES` 行；
     **既有**（**非入口** ✓）文件按 `scripts/docs-budget.json` **冻结**
     （**只许减不许增** ✗，要放宽必须手改那份 JSON ⇒ 评审可见 ✓）；
     **入口文件不进冻结** ✓ —— 它们由 ③ 管：计划会随新批次**合法长大** ✓，
     冻结它必然**误红** ✗（2026-09-26 实测过一次真阻塞 ⇒ 用户要求"修判据不是改冻结值" ✓）；
  ⑤ **禁垃圾**：`docs/**` 下不得有 `.tmpdir` / `.tmp` / `.tmp-<pid>` / `.DS_Store`
     / `*.orig` / `*.rej` / `*~` —— **含未跟踪的本地残留** ✗（判据要咬得住
     2026-09-26 实测的那两类：`.e2-plan.md.*.tmpdir/` 与 `*.json.tmp-<pid>` ✓）；
  ⑥ **归档可追溯**（红线：**归档≠销毁** ✓）：`docs/archive/**` 每个文件都必须
     在 `docs/archive/README.md` 里被**点名** ✓，且归档总量 ≤ `MAX_ARCHIVE_BYTES`
     （防"归档变成垃圾场" ✗）。
  ⑦ **接手路径**（X1，2026-09-28）：`docs/ONBOARDING.md` 的**必读表** —— 每文件必须存在、
     不超它自己的 `max_lines`、**合计**不超 `onboarding_max_lines`（`docs-budget.json`
     的 `onboarding` 一节）⇒ 「接手要读多少」是**会判红的数字**，不是一句希望 ✓。

⚠ **文档过期（2026-09-30 的那条）不在本文件**：它在 `scripts/docs-expiry-check.py`
（`pre-commit` 钩子跑的那条）。此前 `STATUS.md` 把两条混着数成"判据 ①–⑧"、
本 docstring 又只列到 ⑥ ⇒ 三处数字互相打架（接口 agent 的 `SUB-docs-lint-criterion-count-drift`）。

`--freeze` 重算冻结表（**只会收紧** ✓，放宽要手改）；`--json` 出机器可读摘要。

退出码：0 = 通过 ✓ / 1 = 判红 ✗ / 2 = 用法或环境错。
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# ── 预算常量（改这里 = 改判据 ⇒ 必须写理由 ✓）────────────────────────────────
# 2026-09-26 用户要求：3 MB ⇒ **10 MB**。
# 理由（实测，不是拍脑袋）：这条判据是"文档减肥"时期的产物（起点 8.84 MB ⇒ 压到 2.96 MB），
# 目标早已达成；之后它却成了**只红不修**的噪音 —— 撑破它的是 `docs/e2e/logs/`（46 个
# e2e 日志，716 KB，机器产物、每次 CI 回写都涨），而那不是"人该读的文档"。
# 结果：docs-lint 在 origin/main 上长期判红 ⇒ CI 连续红 ⇒ 有人用 SOKO_SKIP_HOOK=1 绕过，
# 门禁失去意义。放宽到 10 MB 让判据重新只拦"真的文档爆炸"（≥10 MB 才是量级异常）。
# 其余判据（单文件 / 入口 ≤800 行 / 设计文档冻结 / 归档索引）**一律不动** —— 那几条才在干活。
#
# ⭐⭐ **2026-10-16：①②⑥ 的三条上限不再写在这里**（用户 2026-10-15【②】的**横向**落地）——
# 上面那段"放宽到 10 MB"的理由本身没错，错的是**解法**：上限贴在实测上方 ⇒ **永远绿 ⇒ 装饰** ✗
# （用户 2026-09-26 拍过板：「**不许靠抬上限达标 —— 上限方向只能是收紧**」✗）。
# ⇒ 真正的 caps 在 **`docs/threshold-ratchet.json`**（**tracked** ⇒ 逐轮进 git），由
# `python3 scripts/check-threshold-ratchet.py --open-round` **每轮降一档**（`tighten()` 只看
# 上一轮、**不看实测** ⇒ 单调不增、**永不高于种子** ✓）；本文件只**消费**它（下面那两行）✓。
# ⚠ **状态缺失/坏 ⇒ 入口闸门响亮退出（exit 2）**，**绝不静默退回种子判过** ✗（与 ② 同口径）——
# 见 `_ratchet_gate()`（`main()` 与 `--selftest` 的第一行都调它 ✓）。
# ⚠ 消费点是**接缝**（A 层产出 caps、B 层消费）：`scripts/check-threshold-ratchet.py` 有
# **A∖B 对账**（本文件里找不到 `_RATCHET_CAPS["docs_live_bytes"]` 就判红 ✗）⇒ 谁把这两行改回
# 常量，谁就当场被咬住 ✓（照 `audit-wire-fields.py` 的先例）。
sys.path.insert(0, str(Path(__file__).resolve().parent))
import threshold_ratchet  # noqa: E402  （同目录库：登记表 + `tighten()` 的唯一出处 ✓）
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

_RATCHET_CAPS, _RATCHET_ERROR = threshold_ratchet.caps_or_error()
MAX_LIVE_BYTES = _RATCHET_CAPS["docs_live_bytes"]  # ① 活文档总量（种子 10 MB；落地实测 6.41 MB）
MAX_FILE_LINES = 2000  # ② 任何单个活文档
MAX_ENTRY_LINES = 800  # ③ 接手必读的入口文件
MAX_NEW_DESIGN_LINES = 150  # ④ docs/design/** 新增文件
MAX_NEW_LINES_DEFAULT = 400  # ④ 其它目录新增文件
MIN_FROZEN = 20  # ④ 冻结下限（空/极短文件也允许 20 行，免得一行都不能加）
MAX_ARCHIVE_BYTES = _RATCHET_CAPS["docs_archive_bytes"]  # ⑥ 归档总量（种子 2.5 MB；实测 0.95 MB）


def _ratchet_gate() -> int | None:
    """ratchet 状态坏 ⇒ **响亮**（exit 2）。**绝不静默退回种子判过** ✗（照 ② 的口径 ✓）。"""
    if _RATCHET_ERROR:
        print(f"error: {_RATCHET_ERROR}", file=sys.stderr)
        return 2
    return None

BUDGET = Path("scripts/docs-budget.json")
DOC_REFS = Path("scripts/doc-refs-baseline.json")
SRC_LINES = Path("scripts/src-lines-baseline.json")

# ── 自检沙箱（`DOCSLINT-SELFTEST-POLLUTES-REAL-REPO`，2026-10-15 ✓）─────────────
# **病根**：`--selftest` 靠"改真文档 ⇒ 判据必须报红 ⇒ 再还原"证明判据咬得住，
# 而它**改的就是真仓库** ✗ ⇒ 两条漏法都实测到了：
#   ① **并发**：两个自检互相踩 —— A 的还原把 B 读到/写下的变异当"原文"写回去
#      ⇒ 残留（实测复现出与事故**同形**的读数：`REQUIREMENTS.md` 219→**919**、
#      `docs/LESSONS.md` 590→**2790**、`docs/design/e2-plan.md` 604→**742** ✓）；
#      更狠的变体：并发读撞上 `write_text` 的"先截断后写" ⇒ 读到空串 ⇒ 还原成
#      **空文件**（内容丢失 ✗）。
#   ② **击杀**：进程被 `timeout`/SIGTERM 打断时 `finally` **根本不会执行** ✗
#      （实测 `kill -TERM` rc=143 ⇒ `LESSONS.md` 停在 2790 行）。
# ⇒ 被测对象被自检破坏：之后 `docs-lint` 判红 ⑦（层/必读合计虚高）、
#   `gate --fast` 报「**文档判据咬不住**」✗ —— 而后者的直接成因是判据 ① 的
#   "当前总量 − 1"基线被并发进程的收放搅动（实测：并发下 `✗ ①` 假红 = 18/19 ✓）。
# **修法**：自检全程在**临时目录的副本**上做 —— 既有文件先逐字节复制进沙箱，
# 夹具只写沙箱。判据**读**路径 `_rp()` 优先取沙箱副本 ⇒ 读数与"真改"**等价**
# （保真 ✓：判据量的仍是"真仓库这批文档被改后"的样子）；**写**路径 `_wp()`
# **只可能**落在沙箱里（越界即报错，绝不静默改真仓库 ✗）。
# 副产品 ✓：每个自检进程有**私有**沙箱 ⇒ 判据 ① 的基线不再被别的进程搅动。
_SANDBOX: Path | None = None

# `--selftest` 会改写的**既有**文件（必须先复制进沙箱，否则写会漏回真仓库 ✗）。
# 只列既有文件 ✓：新建的夹具（`.tmp` / `__docref_*` / `__srclines_*` …）由 `_wp()`
# 直接建在沙箱里 —— 它们靠 `tracked()` 的沙箱并集被发现（等价于真仓库里的未跟踪文件 ✓）。
SELFTEST_MUTATED_FILES = (
    "REQUIREMENTS.md",  # ③ +700 行
    "docs/LESSONS.md",  # ②④ +2200 行
    "docs/design/e2-plan.md",  # ③ +138 / +300 行
    "docs/design/notation-subset.md",  # ④ +300 行
    "scripts/docs-budget.json",  # ⑦ 临时改 `onboarding` 上限
)


def _rp(p: str | Path) -> Path:
    """**读**路径：沙箱里有副本 ⇒ 读副本（自检的变异对判据可见 ✓），否则读真仓库。"""
    q = Path(p)
    if _SANDBOX is not None:
        cand = _SANDBOX / q
        if cand.exists():
            return cand
    return q


def _wp(p: str | Path) -> Path:
    """**写**路径：沙箱开着 ⇒ 只可能落在沙箱里（越界 ⇒ 报错，绝不静默改真仓库 ✗）。"""
    q = Path(p)
    if _SANDBOX is None:
        return q
    if q.is_absolute() or ".." in q.parts:
        raise AssertionError(f"自检写路径越界：{q}（只允许仓库相对路径）")
    cand = _SANDBOX / q
    cand.parent.mkdir(parents=True, exist_ok=True)
    return cand


def _scan_roots(sub: str) -> list[Path]:
    """目录级扫描的根：真仓库 + 沙箱（调用方去重 ✓）。

    为什么不是"沙箱有就只扫沙箱" ✗：沙箱只镜像被改写的那几份 ⇒ 只扫沙箱会让
    真仓库里的其它文件（别的垃圾、别的归档）**看不见** ⇒ 方向性用例失去保真 ✗。
    """
    roots = [Path(sub)]
    if _SANDBOX is not None and (_SANDBOX / sub).exists():
        roots.append(_SANDBOX / sub)
    return roots


def _walk_union(sub: str) -> dict[str, Path]:
    """`sub` 下的文件：真仓库 ∪ 沙箱 ⇒ `{仓库相对路径: 实际路径}`（沙箱副本优先 ✓）。"""
    out: dict[str, Path] = {}
    base = Path(sub)
    if base.exists():
        for p in base.rglob("*"):
            if p.is_file():
                out[str(p)] = p
    if _SANDBOX is not None and (_SANDBOX / sub).exists():
        sb = _SANDBOX / sub
        for p in sb.rglob("*"):
            if p.is_file():
                out[str(p.relative_to(_SANDBOX))] = p
    return out


def _sandbox_files() -> set[str]:
    """沙箱里的文件（**仓库相对路径**）—— 新建的夹具要能被 `tracked()` 看见 ✓。"""
    if _SANDBOX is None or not _SANDBOX.exists():
        return set()
    return {str(p.relative_to(_SANDBOX)) for p in _SANDBOX.rglob("*") if p.is_file()}

# ⑨ 源码行数红线（STRUCT-SRC-LINES，2026-10-12）：`AGENTS.md` 硬规则 5
# 「文件接近 ~500 行即拆分」此前**只有散文、没有机器判据** ✗ —— 判据 ②（单文件
# ≤2000 行）**只数文档**，于是 `crates/front/src/parser.rs` 涨到 **6283** 行、
# `crates/front/src/compile/elab.rs` **8336** 行都无人报 ✗。
#
# 口径（与判据 ④/⑧ 同一个"棘轮"模式，三条缺一不可）：
#  1. **只查生产源码**：`crates/*/src/**/*.rs`，**排除测试聚合文件**
#     （`tests.rs` / `tests/**` / `*_test.rs`）—— 规则 5 管的是**模块结构**，
#     而"很多小用例聚在一个文件"是本仓**有意**的形态（`compile/tests.rs` 11162 行
#     是几百条用例，不是 8000 行的模块）⇒ 对它们判红只会逼人把用例藏起来 ✗。
#     **如实写明这个边界**：这条不覆盖测试文件（不是"忘了"，是不该管 ✓）。
#  2. **新文件超 SOFT(=500) ⇒ 判红**：`AGENTS.md` 的规则从今天起**会拦新窟窿** ✓。
#  3. **存量冻结 + 只许减不许增**：已在基线的文件**不许变长**（变长 ⇒ 判红）；
#     **降回 ≤SOFT 后必须从基线删掉**（否则判红：基线腐烂 ✗）。
SRC_SOFT_LINES = 500
SRC_LINES_EXCLUDE = re.compile(r"(^|/)(tests?\.rs|tests?/)|_test\.rs$")

# ⑧ 文档引用可解析（DOC-DANGLING，2026-10-12）：**活指针必须指得到东西** ✓
#
# 起因（真实盲区）：`dae75759` 一笔删掉 64 份 `docs/design/**`（"已收口/已否决/过程记录"）
# ⇒ 全仓留下 **86 个指向已删文档的路径 · 460 处引用**，而**八条判据全绿也查不出** ✗
# （没有一条在问"你引用的这份文档还在不在"）。读者按指针去找 = 404。
#
# 口径（三条，缺一条都会变成噪声或假绿）：
#  1. **只查"活指针"**：同一行带历史标记（`git log` / `原文 ⇒` / `已删` / `归档` …）的
#     不算 —— 仓库惯例本来就是「原文 ⇒ `git log --all -- <path>`」，那种写法**是对的** ✓。
#  2. **历史台账/自检夹具豁免**：`docs/gaps/ledger.jsonl`（记录当初的工单号）·
#     `scripts/docs-expiry-check.py`（自检**故意**用 `docs/none.md` 这种假路径）——
#     对它们判红 = 让守卫咬自己的夹具 ✗。
#  3. **基线冻结 + 只许减不许增**（与 ④ 同一个模式）：存量债务记进
#     `scripts/doc-refs-baseline.json`（带 reason）；**新增悬空 ⇒ 判红** ·
#     **某条变多 ⇒ 判红** · **某条已归零却没删基线 ⇒ 判红**（逼基线收紧 ✗ 防腐烂）。
#     ⇒ 它今天**咬得住新窟窿**，而存量是**看得见、只能减**的债务，不是"绿了就算"。
DOC_REF_PAT = re.compile(r"docs/[A-Za-z0-9_./\-]+\.md")
# 同一行出现这些 ⇒ 该指针**自己说了**这是历史位置 ⇒ 不算悬空
DOC_REF_HISTORY = ("git log", "原文 ⇒", "已删", "删除", "历史", "归档", "archive/", "HANDOVER")
# 本身就是"点名已删路径"的文件 ⇒ 不参与扫描
DOC_REF_EXEMPT = {
    "scripts/docs-lint.py",  # 本文件（常量与说明里就写着判据本身）
    "scripts/docs-expiry-check.py",  # 自检夹具故意用不存在的路径
    "docs/gaps/ledger.jsonl",  # 历史台账：记的是**当初**的工单路径
    "docs/ux-queue-extras.jsonl",  # 台账：DOC-DANGLING 这条本身要写出那两个路径
    "scripts/doc-refs-baseline.json",  # 基线自己
}

ARCHIVE = Path("docs/archive")
ARCHIVE_INDEX = ARCHIVE / "README.md"

# ③ 接手必读的入口文件（AGENTS.md 的阅读顺序 + 计划入口）
ENTRY_FILES = {
    "AGENTS.md",
    "README.md",
    "REQUIREMENTS.md",
    "ROADMAP.md",
    "STATUS.md",
    "docs/README.md",
    "docs/design/e2-plan.md",
    # ⚠ **2026-09-30 清理**：这里原有 `docs/HANDOVER.md` / `docs/E2-HANDOVER.md` /
    # `docs/PLAN-0.74-0.79.md` 三条 —— 三份文件**都已删除**（内容并入
    # `docs/ONBOARDING.md`）⇒ 判据 ③ 对它们**空转** ✗（列表比现实长 = 假守卫）。
    # **入口文件不进 `docs-budget.json` 的 `frozen`** ✓（计划会随新批次合法长大，
    # 冻结必然误红 ✗）；`docs/ONBOARDING.md` 由判据 ⑦ 的 `onboarding.path` 管 ✓。
}

# ⑤ 垃圾模式（路径 → 是否目录）
JUNK_DIR_SUFFIX = ".tmpdir"
JUNK_FILE_RE = re.compile(r"(\.tmp$|\.tmp-\d+$|\.orig$|\.rej$|~$|~\d+$|^\.DS_Store$)")


def tracked() -> set[str]:
    """**会被提交的**文件（跟踪的 + 未跟踪但没被 ignore 的 ✓）。

    为什么带上未跟踪的 ✗：本地新写的文档在 `git add` 之前也该算进预算 ✓ ——
    否则"先写一大篇、再提交"就能绕过判据 ✗（本仓的纪律：**咬不住的守卫等于没有** ✓）。
    被 ignore 的本地 scratch（`.cache/`、`.sokonanoda/` …）不算 ✓。
    """
    out = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        text=True,
        check=True,
        env=git_env.clean_git_env(),   # 洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`：仓库发现只许由 cwd 决定 ✓
    ).stdout
    files = {p for p in out.split("\0") if p}
    # 自检沙箱里的夹具 = "真仓库里新建的（未跟踪）文件" ⇒ 并进来
    # （等价于 `git ls-files --others` 会列出它们 ✓ —— 见 `_sandbox_files()`）。
    return files | _sandbox_files()


def line_count(path: Path) -> int:
    return len(_rp(path).read_text(encoding="utf-8", errors="replace").splitlines())


def live_docs(files: set[str]) -> list[str]:
    """① 的"活文档"：根 *.md + docs/**（不含 docs/archive/**）。"""
    out = []
    for f in files:
        if f.startswith("docs/archive/"):
            continue
        if f.startswith("docs/") or ("/" not in f and f.endswith(".md")):
            out.append(f)
    return sorted(out)


# **文件级历史声明**（2026-10-13）：文件**开头**（前 60 行）出现这一行 ⇒ 该文件里所有
# `docs/**.md` 指针都按**历史位置**看待（不再逐行加「已删 ⇒ git log …」的尾巴）。
#
# 为什么需要它：逐行加尾巴在**代码注释**里会制造大量噪声（实测 `crates/cli/tests/
# extension.rs` 一个文件 22 处、9 个不同路径），而噪声会让人**不再读注释** ✗。
# 文件级声明把同一件事说**一次**，且同样是**评审可见**的（写在文件里、diff 里看得见 ✓）。
# ⚠ 它**不是豁免**：声明必须点名恢复方式（判据会检查同一行/邻近行有 `git log`）✓
DOC_REF_FILE_MARKER = "doc-refs: history-only"


def _declares_history_only(text: str) -> bool:
    """文件开头是否声明了「本文件的文档指针都是历史位置」（见 `DOC_REF_FILE_MARKER`）。"""
    head = text.splitlines()[:60]
    for i, line in enumerate(head):
        if DOC_REF_FILE_MARKER in line:
            # ⚠ 声明必须**自带恢复方式**（同一行或紧邻两行内有 `git log`）⇒
            # 否则它就成了"免检通行证" ✗（咬不住的守卫等于没有 ✓）。
            near = "\n".join(head[max(0, i - 2) : i + 3])
            return "git log" in near
    return False


def scan_doc_refs(files: set[str]) -> dict[str, list[str]]:
    """⑧ 扫出**悬空的活指针** ⇒ `{被引用的路径: ["文件:行", …]}`。

    口径见 `DOC_REF_*` 常量上的说明（活指针 / 历史台账豁免 / 历史标记）。
    **零扫描不许当绿**：调用方负责核对"扫过的文件数 > 0"（判据 ① 已有同款地板）。
    """
    existing = {f for f in files if _rp(f).exists()}
    targets = [
        f
        for f in sorted(files)
        if f not in DOC_REF_EXEMPT
        and (
            (f.startswith("docs/") and not f.startswith("docs/archive/"))
            or ("/" not in f and f.endswith(".md"))
            or f.startswith(("scripts/", "crates/", "editor/", "dsh/"))
        )
    ]
    out: dict[str, list[str]] = {}
    for f in targets:
        p = _rp(f)
        if not p.exists() or p.suffix in {".png", ".jpg", ".gif", ".vsix"}:
            continue
        try:
            text = p.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        if _declares_history_only(text):
            continue  # 文件级历史声明（自带恢复方式 ✓，见常量说明）
        for i, line in enumerate(text.splitlines(), 1):
            if any(h in line for h in DOC_REF_HISTORY):
                continue
            for m in DOC_REF_PAT.findall(line):
                ref = m.rstrip(".,;:)）`")
                if ref in existing:
                    continue
                out.setdefault(ref, []).append(f"{f}:{i}")
    return out


_DOC_REFS_PATHS_OVERRIDE: dict | None = None
_SRC_LINES_OVERRIDE: dict | None = None


def scan_src_lines(files: set[str]) -> dict[str, int]:
    """⑨ 生产源码里**超过 `SRC_SOFT_LINES`** 的文件 ⇒ `{路径: 行数}`（口径见常量）。"""
    out: dict[str, int] = {}
    for f in sorted(files):
        if not f.endswith(".rs") or not f.startswith("crates/"):
            continue
        if "/src/" not in f or SRC_LINES_EXCLUDE.search(f):
            continue
        p = _rp(f)
        if not p.exists():
            continue
        n = line_count(p)
        if n > SRC_SOFT_LINES:
            out[f] = n
    return out


def load_src_lines_baseline() -> dict:
    if _SRC_LINES_OVERRIDE is not None:  # 自检注入
        return _SRC_LINES_OVERRIDE
    if not _rp(SRC_LINES).exists():
        return {}
    try:
        return (json.loads(_rp(SRC_LINES).read_text(encoding="utf-8")) or {}).get("files") or {}
    except (json.JSONDecodeError, OSError):
        return {}


def check_src_lines(files: set[str]) -> tuple[list[str], dict]:
    """⑨ 的判据本体（`--check-src-lines` 单跑 + 自检都打它）。"""
    found = scan_src_lines(files)
    base = load_src_lines_baseline()
    bad: list[str] = []
    shrunk: list[str] = []
    for f, n in sorted(found.items()):
        cap = (base.get(f) or {}).get("lines")
        if cap is None:
            bad.append(
                f"⑨ **新增**超长源码文件 `{f}`：{n} 行 > 软线 {SRC_SOFT_LINES}"
                f"（`AGENTS.md` 硬规则 5：接近 ~500 行即拆分）"
                f" ⇒ 拆模块、或按 `--freeze-src-lines` 走**评审可见**的基线（只在存量清单里加 ✗）"
            )
        elif n > cap:
            bad.append(
                f"⑨ `{f}` **变长**了：{n} > 基线 {cap}（只许减不许增 ✗）"
                f" ⇒ 拆出去，别往已经超标的文件里加 ✗"
            )
        elif n < cap:
            shrunk.append(f"{f}（{cap} ⇒ {n}）")
    for f, meta in sorted(base.items()):
        if f not in found:
            bad.append(
                f"⑨ 基线过期：`{f}` 已降到 ≤{SRC_SOFT_LINES} 行（基线记着 {meta.get('lines')}）"
                f" ⇒ **删掉这条**（基线只许减不许增 ✓）"
            )
    stats = {"src_over_soft": len(found), "src_max_lines": max(found.values(), default=0)}
    if shrunk:
        stats["src_shrinkable"] = shrunk
    return bad, stats


def load_doc_refs_baseline() -> dict:
    """⑧ 基线：`{"paths": {path: {"count": N, "reason": "…"}}}`（缺失 ⇒ 空基线）。"""
    if _DOC_REFS_PATHS_OVERRIDE is not None:  # 自检注入（见 selftest ✓）
        return _DOC_REFS_PATHS_OVERRIDE
    if not _rp(DOC_REFS).exists():
        return {}
    try:
        return (json.loads(_rp(DOC_REFS).read_text(encoding="utf-8")) or {}).get("paths") or {}
    except (json.JSONDecodeError, OSError):
        return {}


def check_doc_refs(files: set[str]) -> tuple[list[str], dict]:
    """⑧ 的判据本体（抽出来是为了 `--check-doc-refs` 能单跑 + 自检能打它）。"""
    found = scan_doc_refs(files)
    base = load_doc_refs_baseline()
    bad: list[str] = []
    shrunk: list[str] = []
    for ref, sites in sorted(found.items()):
        cap = (base.get(ref) or {}).get("count")
        if cap is None:
            bad.append(
                f"⑧ **新增**悬空文档引用 `{ref}`（{len(sites)} 处，例：{sites[0]}）"
                f" ⇒ 要么修指针、要么把路径建出来；确属历史位置 ⇒ 按仓库惯例写"
                f"「原文 ⇒ git log --all -- {ref}」✓"
            )
        elif len(sites) > cap:
            bad.append(
                f"⑧ `{ref}` 的悬空引用**变多**了：{len(sites)} > 基线 {cap}（例：{sites[0]}）"
                f" ⇒ 基线只许减不许增 ✗"
            )
        elif len(sites) < cap:
            shrunk.append(f"{ref}（基线 {cap} ⇒ 实测 {len(sites)}）")
    for ref, meta in sorted(base.items()):
        if ref not in found:
            bad.append(
                f"⑧ 基线过期：`{ref}` 已无悬空引用（基线记着 {meta.get('count')} 处）"
                f" ⇒ **删掉这条**（基线只许减不许增，留着就是腐烂 ✗）"
            )
    stats = {"doc_refs_dangling": sum(len(v) for v in found.values()), "doc_refs_paths": len(found)}
    if shrunk:
        stats["doc_refs_shrinkable"] = shrunk
    return bad, stats


def load_budget() -> dict[str, int]:
    if not _rp(BUDGET).exists():
        return {}
    return json.loads(_rp(BUDGET).read_text(encoding="utf-8")).get("frozen", {})


def load_raw_budget() -> dict:
    """整份 `docs-budget.json`（判据 ⑦ 要读 `onboarding` 一节，不只是 `frozen`）。"""
    if not _rp(BUDGET).exists():
        return {}
    return json.loads(_rp(BUDGET).read_text(encoding="utf-8"))


def scan_junk() -> list[str]:
    """⑤ 扫 docs/ 下的垃圾（**含未跟踪** ✗）；自检沙箱里的夹具同样算 ✓。"""
    bad = []
    for root in _scan_roots("docs"):
        for dirpath, dirnames, filenames in os.walk(root):
            for d in list(dirnames):
                if d.endswith(JUNK_DIR_SUFFIX):
                    bad.append(f"{dirpath}/{d}/（临时目录）")
                    dirnames.remove(d)
            for f in filenames:
                if JUNK_FILE_RE.search(f):
                    bad.append(f"{dirpath}/{f}")
    return sorted(set(bad))


LAYER_GLOBS = {
    # 四层（X1 / docs/ONBOARDING.md 的同一份口径）
    "L0": ("README.md", "AGENTS.md", "ROADMAP.md", "REQUIREMENTS.md", "STATUS.md"),
    "L1": None,   # `docs/` 顶层（非递归）
    "L2": None,   # `docs/design|notes|gaps|perf|e2e`
    "L3": None,   # `docs/archive`
}


def layer_files(layer: str) -> list[str]:
    out = []
    for f in tracked():
        if not f.endswith(".md"):
            continue
        if layer == "L0":
            if f in LAYER_GLOBS["L0"]:
                out.append(f)
        elif layer == "L1":
            if f.startswith("docs/") and f.count("/") == 1 and not f.startswith("docs/archive/"):
                out.append(f)
        elif layer == "L2":
            if f.startswith(("docs/design/", "docs/notes/", "docs/gaps/", "docs/perf/", "docs/e2e/")):
                out.append(f)
        elif layer == "L3":
            if f.startswith("docs/archive/"):
                out.append(f)
    return [f for f in out if Path(f).exists()]


def layer_lines(layer: str) -> int | None:
    fs = layer_files(layer)
    if not fs:
        return None
    return sum(line_count(Path(f)) for f in fs)


def check() -> tuple[list[str], dict]:
    files = tracked()
    live = live_docs(files)
    bad: list[str] = []

    # ① 活文档总量
    live_bytes = sum(_rp(f).stat().st_size for f in live if _rp(f).exists())
    if live_bytes > MAX_LIVE_BYTES:
        bad.append(
            f"① 活文档总量 {live_bytes / 1e6:.2f} MB > 上限 {MAX_LIVE_BYTES / 1e6:.2f} MB"
        )

    # ②③④ 行数
    budget = load_budget()
    raw_budget = load_raw_budget()
    for f in live:
        p = _rp(f)
        if p.suffix != ".md" or not p.exists():
            continue
        n = line_count(p)
        if n > MAX_FILE_LINES:
            bad.append(f"② {f}：{n} 行 > 单文件上限 {MAX_FILE_LINES}")
        if f in ENTRY_FILES and n > MAX_ENTRY_LINES:
            bad.append(f"③ {f}：{n} 行 > 入口文件上限 {MAX_ENTRY_LINES}")
        cap = budget.get(f)
        if f in ENTRY_FILES:
            # **入口文件不进冻结**（2026-09-26 用户要求 ✓，修**判据**而不是改冻结值 ✓）：
            # 入口文件有自己的专属上限（判据 ③ ≤800 行 ✓ —— 它抓的是"变成巨石" ✗），
            # 而"冻结在当前行数"在"**计划随新批次长大**"时**必然误红** ✗ ——
            # 实测：另一会话往 `e2-plan.md` 加「批次 N」+138 行（**合法推进** ✓）
            # ⇒ 撞死"冻结在 379" ✗ ⇒ **真阻塞**：它会让**所有人**（含后续 notation 的推送）
            # 都推不动 ✗。⇒ 正确形态：**③ 管入口文件，④ 只管非入口的既有文档** ✓。
            pass
        elif cap is None:
            cap = MAX_NEW_DESIGN_LINES if f.startswith("docs/design/") else MAX_NEW_LINES_DEFAULT
            if n > cap:
                bad.append(f"④ {f}：{n} 行 > 新增文件预算 {cap}（没在 {BUDGET} 里）")
        elif n > cap:
            bad.append(
                f"④ {f}：{n} 行 > 冻结预算 {cap}（**只许减不许增** ✓；"
                f"确需长大 ⇒ `--freeze` 重新基线，或手改 {BUDGET} ⇒ 评审可见 ✓）"
            )

    # ⑤ 垃圾
    bad += [f"⑤ 垃圾残留：{j}" for j in scan_junk()]

    # ⑦ 接手路径（X1，2026-09-28）：**把"接手要读多少"变成会判红的数字** ✓。
    # 口径：`docs-budget.json` 的 `onboarding.path` 是**必读**（全读）清单，
    # 每项有行数上限，全表有 `onboarding_max_lines`；`layer_max_lines` 管四层总行数。
    # ⚠ 用户 2026-09-26：「把上限从 3 MB 抬到 10 MB **不算完成** —— 要的是**少消耗
    # 注意力**，不是允许更多」⇒ 这条判据的**方向只能是收紧** ✓。
    ob = raw_budget.get("onboarding") if isinstance(raw_budget, dict) else None
    if not ob:
        bad.append(f"⑦ {BUDGET} 缺 `onboarding` 一节（接手路径没有权威 ⇒ 判据 ⑦ 空转 ✗）")
    else:
        total = 0
        for item in ob.get("path", []):
            f, cap = item.get("file"), item.get("max_lines")
            if not f or not isinstance(cap, int):
                bad.append(f"⑦ onboarding.path 条目非法：{item!r}")
                continue
            fp = _rp(f)
            if not fp.exists():
                bad.append(f"⑦ 必读路径里的 {f} 不存在（接手会撞空 ✗）")
                continue
            n = line_count(fp)
            total += n
            if n > cap:
                bad.append(
                    f"⑦ 必读文件 {f}：{n} 行 > 上限 {cap}（**接手成本涨了** ✗；"
                    f"要么删内容、要么把它移出必读表 ⇒ 改 {BUDGET} 评审可见 ✓）"
                )
        ceiling = ob.get("onboarding_max_lines")
        if isinstance(ceiling, int) and total > ceiling:
            bad.append(
                f"⑦ 必读路径合计 {total} 行 > 上限 {ceiling} ⇒ **接手要读的变多了** ✗"
                f"（书就一本：要么删、要么把某文件移出必读表）"
            )
        for layer, cap in (ob.get("layer_max_lines") or {}).items():
            if not isinstance(cap, int):
                continue
            got = layer_lines(layer)
            if got is None:
                continue  # 该层今天没有文件（不判红：空层不是错误 ✓）
            if got > cap:
                bad.append(f"⑦ {layer} 层 {got} 行 > 上限 {cap}（只许减不许增 ✗）")

    if not live:
        bad.append("① 活文档 **0 个** ⇒ **没扫到 ≠ 绿** ✗（检查 cwd 与 `git ls-files`）")

    # ⑥ 归档可追溯
    arch_all = _walk_union(str(ARCHIVE))
    if ARCHIVE.exists() or arch_all:
        arch_files = sorted(k for k in arch_all if k != str(ARCHIVE_INDEX))
        arch_bytes = sum(arch_all[k].stat().st_size for k in arch_files)
        if arch_bytes > MAX_ARCHIVE_BYTES:
            bad.append(
                f"⑥ 归档总量 {arch_bytes / 1e6:.2f} MB > 上限 {MAX_ARCHIVE_BYTES / 1e6:.2f} MB"
            )
        if not _rp(ARCHIVE_INDEX).exists():
            bad.append(f"⑥ 缺归档索引 {ARCHIVE_INDEX}（归档必须有可追溯路径）")
        else:
            index = _rp(ARCHIVE_INDEX).read_text(encoding="utf-8")
            for f in arch_files:
                if Path(f).name not in index:
                    bad.append(f"⑥ {f} 没在 {ARCHIVE_INDEX} 里点名（归档≠销毁）")
    else:
        # ⚠ **零扫描 ≠ 绿** ✗✓（E00 切片 B）：原来这里只把字节数置 0、**不判红** ⇒
        # 归档目录一旦不在，⑥「**归档≠销毁**」这条红线**真空通过** ✗。
        arch_bytes = 0
        bad.append(f"⑥ 找不到归档目录 {ARCHIVE} ⇒ 判据 ⑥ **无法判定 ≠ 绿** ✗")

    # ⑧ 文档引用可解析（DOC-DANGLING）
    ref_bad, ref_stats = check_doc_refs(files)
    bad += ref_bad

    # ⑨ 源码行数红线（STRUCT-SRC-LINES）
    src_bad, src_stats = check_src_lines(files)
    bad += src_bad

    # ⚠ **零扫描 ≠ 绿** ✗✓（E00 切片 B 实测）：`cd scripts && python3 docs-lint.py` 会印
    # 「活文档 **0 个** / 0.00 MB ✓ · 归档 0 个 … 归档索引齐 ✓」并 **exit 0** ✗ ——
    # 各条判据都在"什么都没扫到"上**真空中通过** ✗。判据 ① 的本意是"活文档 ≤ 上限"，
    # **0 个活文档不是"通过"，是"没量到"** ✗ ⇒ 补地板 ✓。
    stats = {
        "live_bytes": live_bytes,
        "live_files": sum(1 for f in live if Path(f).exists()),
        "archive_bytes": arch_bytes,
        "archive_files": len(arch_files),
        "max_live_bytes": MAX_LIVE_BYTES,
        **ref_stats,
        **src_stats,
    }
    return bad, stats


def selftest() -> int:
    """**反向验证**（`AGENTS.md`：**咬不住的守卫等于没有** ✓）。

    把判据**逐条故意弄红一次** ⇒ 每条都必须被报出来 ✗；再加**三条方向性**用例
    （用户 2026-09-26 要求 ✓）：**回胖必须判红** ✓、**正常推进必须不误红** ✓、
    **非入口的冻结仍要咬** ✓。判据：`python3 scripts/docs-lint.py --selftest`
    ⇒ exit 0 + `19/19`。

    ⚠ **全程在临时沙箱里改，真仓库一个字节都不动** ✓（`DOCSLINT-SELFTEST-POLLUTES-REAL-REPO`，
    2026-10-15）：旧写法直接改真文档、靠 `finally` 还原 —— 并发两个自检会互相踩
    （还原把别人的变异写回去 ⇒ 残留），进程被 `timeout`/SIGTERM 打断时 `finally`
    **根本不执行** ⇒ 自检把**被测对象**破坏了 ✗（详见 `_SANDBOX` 那段注释 ✓）。
    现在：既有文件先逐字节复制进沙箱、夹具只写沙箱（`_wp()` 越界即报错 ✓），
    判据读路径 `_rp()` 取沙箱副本 ⇒ **读数与"真改"等价**（保真 ✓）。

    为什么要有它 ✗：本仓**三次**出现过"判据永远绿"的摆设 ✓ —— 一条判据如果
    没人证明它咬得住，就不能算守卫 ✓；而**误红**同样是坏判据 ✗（它会把所有人的
    推送一起拦下 ⇒ 真阻塞 ✓）。
    """
    global _SANDBOX, MAX_LIVE_BYTES, _DOC_REFS_PATHS_OVERRIDE, _SRC_LINES_OVERRIDE
    # ⚠ **自检也要过 ratchet 闸门**（状态坏 ⇒ 自检**不许**报绿：它量的正是那批 caps ✓）
    _gate = _ratchet_gate()
    if _gate is not None:
        return _gate
    sandbox = Path(tempfile.mkdtemp(prefix="docs-lint-selftest-"))
    # 既有文件先复制进沙箱：判据读到的就是"真仓库这批文档被改后"的样子（逐字节 ✓）。
    for rel in SELFTEST_MUTATED_FILES:
        src = Path(rel)
        if not src.exists():
            continue  # 文件不在 ⇒ 对应夹具本来就会跳过（与旧口径一致 ✓）
        dst = sandbox / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
    saved_cap = MAX_LIVE_BYTES
    _SANDBOX = sandbox
    try:
        return _selftest_body()
    finally:
        _SANDBOX = None
        MAX_LIVE_BYTES = saved_cap
        _DOC_REFS_PATHS_OVERRIDE = None
        _SRC_LINES_OVERRIDE = None
        shutil.rmtree(sandbox, ignore_errors=True)


def _selftest_body() -> int:
    """19 条夹具本体（**只许**由 `selftest()` 调用 —— 它负责沙箱与清理 ✓）。"""
    if _SANDBOX is None:
        raise AssertionError("自检必须在沙箱里跑（见 selftest()）—— 否则会改真仓库 ✗")
    results: list[tuple[str, str, bool]] = []

    def append_lines(path: str, n: int, line: str = "x\n"):
        """往沙箱里的活文档追加 n 行，返回还原函数（`finally` 里必须调 ✓）。"""
        old = _rp(path).read_text(encoding="utf-8")
        w = _wp(path)
        w.write_text(old + line * n, encoding="utf-8")
        return lambda: w.write_text(old, encoding="utf-8")

    # ① 活文档总量：**跟着上限走**（2026-09-26 修 ✓）——上限从 3 MB 提到 **10 MB**
    # 之后，原来写死的"灌 ≈800 KB"再也撑不破 ⇒ **自检自己报「① 咬不住」** ✗
    # （那正是自检该干的事 ✓ —— 判据上限一改，就得回头验判据还咬不咬得住 ✓）。
    # 修法不是把数字改大（下次提上限又失效 ✗），而是把上限**临时压到
    # "当前总量 − 1"** ⇒ 立刻超 ✓，**与上限取值无关** ✓，也省掉往仓库灌 7 MB。
    global MAX_LIVE_BYTES
    saved_cap = MAX_LIVE_BYTES
    _, base_stats = check()
    try:
        MAX_LIVE_BYTES = max(0, int(base_stats["live_bytes"]) - 1)
        bad, _ = check()
    finally:
        MAX_LIVE_BYTES = saved_cap
    # ⚠ **① 是唯一没绑夹具的** ✗：它的报错文案里**不含任何文件名**（是总量），
    #   没法按文件名绑 ✓ —— 但它本来就**不靠夹具**：把上限压到「当前总量 − 1」
    #   ⇒ **只有它可能触发** ✓（与 ②/④ 的"灌同一个文件"不同 ✓）。如实注明，不假装绑了 ✗。
    results.append(("①", "活文档总量 > 上限", any(b.startswith("①") for b in bad)))

    # ② 单文件行数 + ④ 冻结预算：往一份活文档灌 2200 行
    p = _rp("docs/LESSONS.md")
    if p.exists():
        old = p.read_text(encoding="utf-8")
        w = _wp("docs/LESSONS.md")
        w.write_text(old + ("填" * 200 + "\n") * 2200, encoding="utf-8")  # ≈800 KB / 2200 行
        try:
            bad, _ = check()
        finally:
            w.write_text(old, encoding="utf-8")
        results.append(("②", "单文件 > 2000 行", any(
            b.startswith("②") and "LESSONS.md" in b for b in bad)))
        results.append(("④", "冻结预算被撑大", any(
            b.startswith("④") and "LESSONS.md" in b for b in bad)))

    # ③ 入口文件 > 800 行
    if _rp("REQUIREMENTS.md").exists():
        restore = append_lines("REQUIREMENTS.md", 700)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(("③", "入口文件 > 800 行", any(
            b.startswith("③") and "REQUIREMENTS.md" in b for b in bad)))

    # ③/④ **方向性**（用户 2026-09-26 要求 ✓）：入口文件（`e2-plan` 这类**计划**）
    # ① 正常推进（+138 行 = 「批次 N」的量级）⇒ **不误红** ✓
    # ② 回胖（+300 行）⇒ 由 ③ 判红 ✓（**不是**由冻结判红 —— 冻结不管入口文件 ✓）
    # ③ 非入口的冻结文档回胖（+300）⇒ 由 ④ 判红 ✓（棘轮仍然活着 ✓）
    ep = _rp("docs/design/e2-plan.md")
    if ep.exists():
        restore = append_lines("docs/design/e2-plan.md", 138)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(
            ("③", "入口计划 +138 行（正常推进）⇒ **不误红**", not any(
                b.startswith(("③", "④")) and "e2-plan" in b for b in bad))
        )
        restore = append_lines("docs/design/e2-plan.md", 300)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(
            ("③", "入口计划 +300 行（回胖）⇒ 判红", any(
                b.startswith("③") and "e2-plan" in b for b in bad))
        )
    nsub = _rp("docs/design/notation-subset.md")
    if nsub.exists():
        restore = append_lines("docs/design/notation-subset.md", 300)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(
            ("④", "非入口冻结文档 +300 行 ⇒ 判红（棘轮活着）", any(
                b.startswith("④") and "notation-subset" in b for b in bad))
        )

    # ⑤ 垃圾残留
    junk = _wp("docs/__docs-lint-selftest.tmp")
    junk.write_text("junk", encoding="utf-8")
    try:
        bad, _ = check()
    finally:
        junk.unlink()
    results.append(("⑤", "docs/** 垃圾残留", any(
            b.startswith("⑤") and "__docs-lint-selftest.tmp" in b for b in bad)))

    # ⑥ 归档文件没被索引点名
    stray = _wp("docs/archive/__selftest-unindexed.md.gz")
    stray.write_bytes(b"\x1f\x8b")
    try:
        bad, _ = check()
    finally:
        stray.unlink()
    results.append(("⑥", "归档文件未被点名", any(
            b.startswith("⑥") and "__selftest-unindexed" in b for b in bad)))

    # ⑦ 接手路径：把**必读表的合计上限**临时压到 1 行 ⇒ 必须判红 ✓
    # （X1，2026-09-28：这条判据自己也要能被证明咬得住 ✓）
    budget_path = _rp(BUDGET)
    budget_old = budget_path.read_text(encoding="utf-8")
    try:
        data = json.loads(budget_old)
        data["onboarding"]["onboarding_max_lines"] = 1
        _wp(BUDGET).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        bad, _ = check()
    finally:
        _wp(BUDGET).write_text(budget_old, encoding="utf-8")
    results.append(("⑦", "接手路径合计超上限", any(
            b.startswith("⑦") and "必读路径合计" in b for b in bad)))

    # ⑦b 接手路径：把**某个必读文件的上限**压到 1 行 ⇒ 也必须判红 ✓
    try:
        data = json.loads(budget_old)
        data["onboarding"]["path"][0]["max_lines"] = 1
        _wp(BUDGET).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        bad, _ = check()
    finally:
        _wp(BUDGET).write_text(budget_old, encoding="utf-8")
    results.append(("⑦", "必读单文件超上限", any(
            b.startswith("⑦") and "必读文件" in b for b in bad)))

    # ⑧ 文档引用可解析：**三条都要咬** ——
    #   (a) 新增悬空引用 ⇒ 判红（也就是它存在的理由 ✓）；
    #   (b) 历史标记的指针 ⇒ **不许误红**（否则仓库惯例「原文 ⇒ git log --all -- …」
    #       会被自己的守卫拦下 ✗ —— 误红同样是坏判据 ✓）；
    #   (c) 基线里某条已归零却没删 ⇒ 判红（逼基线收紧，防"冻结了就不管"✗）。
    tmp_doc = _wp("docs/design/__docref_selftest__.md")
    tmp_doc.write_text("见 `docs/design/__docref_absent__.md` 的说明。\n", encoding="utf-8")
    try:
        bad, _ = check_doc_refs(tracked())
    finally:
        tmp_doc.unlink(missing_ok=True)
    results.append(("⑧", "新增悬空文档引用", any(
        b.startswith("⑧") and "__docref_absent__" in b for b in bad)))

    hist_doc = _wp("docs/design/__docref_hist_selftest__.md")
    hist_doc.write_text(
        "指针：`docs/design/__docref_absent__.md`（已删；原文 ⇒ git log --all -- docs/design/__docref_absent__.md）\n",
        encoding="utf-8",
    )
    try:
        bad, _ = check_doc_refs(tracked())
    finally:
        hist_doc.unlink(missing_ok=True)
    results.append(("⑧", "历史标记的指针**不误红**", not any(
        "__docref_absent__" in b for b in bad)))

    saved_base = load_doc_refs_baseline()
    global _DOC_REFS_PATHS_OVERRIDE
    _DOC_REFS_PATHS_OVERRIDE = {**saved_base, "docs/definitely-not-there-selftest.md": {"count": 3, "reason": "自检"}}
    try:
        bad, _ = check_doc_refs(tracked())
    finally:
        _DOC_REFS_PATHS_OVERRIDE = None
    results.append(("⑧", "基线过期（已归零却留着）", any(
        b.startswith("⑧") and "基线过期" in b for b in bad)))

    # ⑨ 源码行数红线：**三条都要咬**（与 ⑧ 同款方向性 ✓）——
    #   (a) 新文件超软线 ⇒ 判红；(b) 存量**变长** ⇒ 判红；(c) 基线过期 ⇒ 判红。
    tmp_src = _wp("crates/front/src/__srclines_selftest__.rs")
    tmp_src.write_text("// x\n" * (SRC_SOFT_LINES + 5), encoding="utf-8")
    try:
        bad, _ = check_src_lines(tracked())
    finally:
        tmp_src.unlink(missing_ok=True)
    results.append(("⑨", "新增超长源码文件", any(
        b.startswith("⑨") and "__srclines_selftest__" in b for b in bad)))

    saved_src = load_src_lines_baseline()
    probe = "crates/front/src/__srclines_grow_selftest__.rs"
    tmp_src.write_text("// x\n" * (SRC_SOFT_LINES + 50), encoding="utf-8")
    global _SRC_LINES_OVERRIDE
    _SRC_LINES_OVERRIDE = {probe: {"lines": SRC_SOFT_LINES + 10, "reason": "自检"}}
    try:
        bad, _ = check_src_lines({probe, *tracked()})
    finally:
        _SRC_LINES_OVERRIDE = None
        tmp_src.unlink(missing_ok=True)
    results.append(("⑨", "存量文件变长", any(
        b.startswith("⑨") and "__srclines_grow_selftest__" in b for b in bad)))

    _SRC_LINES_OVERRIDE = {"crates/front/src/lib.rs": {"lines": 99999, "reason": "自检"}}
    try:
        bad, _ = check_src_lines(tracked())
    finally:
        _SRC_LINES_OVERRIDE = None
    results.append(("⑨", "基线过期（已降回却留着）", any(
        b.startswith("⑨") and "基线过期" in b for b in bad)))

    # ⑧b **文件级历史声明**：三条方向性用例（与 ⑧ 同款 ✓）
    m_doc = _wp("docs/design/__docref_marker_selftest__.md")
    m_doc.write_text(
        "<!-- " + DOC_REF_FILE_MARKER + "：本文件提到的 `docs/**` 均已删除，"
        "原文 ⇒ git log --all -- <path> -->\n见 `docs/design/__docref_absent__.md`。\n",
        encoding="utf-8",
    )
    try:
        bad, _ = check_doc_refs(tracked())
    finally:
        m_doc.unlink(missing_ok=True)
    results.append(("⑧", "文件级历史声明生效（不误红）", not any(
        "__docref_absent__" in b for b in bad)))

    m_doc.write_text(
        f"<!-- {DOC_REF_FILE_MARKER} -->\n见 `docs/design/__docref_absent__.md`。\n",
        encoding="utf-8",
    )
    try:
        bad, _ = check_doc_refs(tracked())
    finally:
        m_doc.unlink(missing_ok=True)
    results.append(("⑧", "声明**没写恢复方式** ⇒ 不算（免检通行证 ✗）", any(
        "__docref_absent__" in b for b in bad)))

    ok = all(hit for _, _, hit in results)
    for code, label, hit in results:
        print(f"  {'✓' if hit else '✗'} {code} {label}")
    print(
        f"docs-lint --selftest：{sum(1 for _, _, h in results if h)}/{len(results)} "
        f"条判据咬得住 {'✓' if ok else '✗（有判据咬不住！）'}"
    )
    return 0 if ok else 1


def freeze() -> int:
    """把当前行数写进冻结表 —— **只收紧** ✓（已存在的更小 cap 不会被抬高）。

    **入口文件不写进来** ✓（2026-09-26 用户要求 ✓）：它们由判据 ③（≤800 行）管 ——
    计划会随新批次合法长大 ✓，冻结它必然误红 ✗（详见 `check()` 里那段注释）。
    """
    files = tracked()
    old = load_budget()
    new: dict[str, int] = {}
    tightened = 0
    for f in live_docs(files):
        p = Path(f)
        if p.suffix != ".md" or not p.exists() or f in ENTRY_FILES:
            continue
        n = max(line_count(p), MIN_FROZEN)
        cap = min(old[f], n) if f in old else n
        if f in old and cap < old[f]:
            tightened += 1
        new[f] = cap
    BUDGET.write_text(
        json.dumps(
            {
                "_comment": (
                    "文档预算冻结表（REQUIREMENTS.md §9 / docs/design/docs-diet.md）："
                    "每个既有**非入口**活文档的行数上限 —— **只许减不许增**。"
                    "**入口文件不在此表**（它们由判据 ③ ≤800 行管 ✓ —— 计划会随新批次"
                    "合法长大，冻结它必然误红 ✗）。"
                    "`python3 scripts/docs-lint.py --freeze` 只会收紧；"
                    "要放宽必须手改本文件（评审可见）。"
                ),
                "frozen": dict(sorted(new.items())),
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    added = len(new) - len([f for f in new if f in old])
    print(
        f"docs-budget.json 已更新：{len(new)} 条（新增 {added} · 收紧 {tightened}）"
    )
    return 0


def check_doc_refs_only() -> int:
    """`--check-doc-refs`：只跑判据 ⑧（DOC-DANGLING 的判据就是这条命令 ✓）。"""
    try:
        files = tracked()
    except subprocess.CalledProcessError as e:
        print(f"docs-lint: 环境错：{e}", file=sys.stderr)
        return 2
    bad, stats = check_doc_refs(files)
    if bad:
        print(f"docs-lint --check-doc-refs：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad:
            print(f"  - {b}", file=sys.stderr)
        return 1
    print(
        f"docs-lint --check-doc-refs ✓ 悬空引用 {stats['doc_refs_dangling']} 处 / "
        f"{stats['doc_refs_paths']} 个路径 ⇒ **都在基线内、且没有新增** ✓"
    )
    if stats.get("doc_refs_shrinkable"):
        print(f"  ⚠ 可收紧基线（存量已下降）：{' · '.join(stats['doc_refs_shrinkable'])}")
    return 0


def freeze_doc_refs() -> int:
    """`--freeze-doc-refs`：把**当前实测**的悬空引用写成新基线（只许减不许增 ✓）。

    拒绝"变大"：任何一条实测比旧基线多 ⇒ exit 2（**不许用重新基线掩盖增长** ✗）。
    """
    files = tracked()
    found = scan_doc_refs(files)
    old = load_doc_refs_baseline()
    grown = [
        f"{r}（{len(found.get(r, []))} > 基线 {meta.get('count')}）"
        for r, meta in old.items()
        if len(found.get(r, [])) > meta.get("count", 0)
    ]
    if grown:
        print(
            f"✗ 拒绝重新基线：这些路径的悬空引用**变多了** ⇒ 先修指针，别拿基线盖住 ✗\n  "
            + "\n  ".join(grown),
            file=sys.stderr,
        )
        return 2
    paths = {}
    for ref, sites in sorted(found.items()):
        paths[ref] = {
            "count": len(sites),
            "reason": (old.get(ref) or {}).get("reason")
            or "设计文档已收口删除（dae75759 等）；引用点尚未改写成历史指针 —— 存量债务，只许减不许增",
            "example": sites[0],
        }
    DOC_REFS.write_text(
        json.dumps(
            {
                "_comment": (
                    "判据 ⑧ 的存量基线（DOC-DANGLING）：这些 `docs/**.md` 路径已不存在，"
                    "但引用点还没改写成「原文 ⇒ git log --all -- <path>」。**只许减不许增** ✓ —— "
                    "新增悬空引用会立刻判红；某条归零后必须从这里删掉（否则判红：基线腐烂 ✗）。"
                ),
                "paths": paths,
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"doc-refs 基线已更新：{len(paths)} 条（悬空引用 {sum(len(v) for v in found.values())} 处）")
    return 0


def check_src_lines_only() -> int:
    """`--check-src-lines`：只跑判据 ⑨（STRUCT-SRC-LINES 的判据就是这条 ✓）。"""
    try:
        files = tracked()
    except subprocess.CalledProcessError as e:
        print(f"docs-lint: 环境错：{e}", file=sys.stderr)
        return 2
    bad, stats = check_src_lines(files)
    if bad:
        print(f"docs-lint --check-src-lines：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad:
            print(f"  - {b}", file=sys.stderr)
        return 1
    print(
        f"docs-lint --check-src-lines ✓ 超软线({SRC_SOFT_LINES})的生产源码 "
        f"{stats['src_over_soft']} 个（最长 {stats['src_max_lines']} 行）⇒ **都在基线内、且没有变长** ✓"
        "（口径：`crates/*/src/**/*.rs`，**不含**测试聚合文件）"
    )
    if stats.get("src_shrinkable"):
        print(f"  ⚠ 可收紧基线：{' · '.join(stats['src_shrinkable'])}")
    return 0


def freeze_src_lines(reason: str = "") -> int:
    """`--freeze-src-lines [--reason <为什么>]`：把当前实测写成新基线。

    **默认拒绝变大**（与 ⑧ 同规矩 ✓）；**但必须有"评审可见"的放宽通道** ✗✓ ——
    与判据 ④ 的 `docs-budget.json` 同一个模型：那条也冻结，而"确需长大 ⇒ `--freeze`
    重新基线，或手改 JSON ⇒ **评审可见** ✓"。**没有通道的棘轮 = 不可用**：
    实测（2026-10-12）合法修 bug 往 `parser.rs` 加 187 行就让整个 `docs-lint` 判红，
    而 `--freeze-src-lines` 又拒绝任何增长 ⇒ **没有出路**（要么拆一个 6283 行的文件、
    要么绕过守卫 ✗）⇒ 那等于逼人绕过守卫。
    ⇒ 带 `--reason` 时**接受增长**，并把 `grew_from`（旧值）与 `reason` **写进基线**，
    于是 `git diff` 里**逐条可见**（谁、长了多少、为什么）✓；不带 `--reason` 仍拒绝 ✓。
    """
    files = tracked()
    found = scan_src_lines(files)
    old = load_src_lines_baseline()
    grown = [
        (f, found.get(f, 0), meta.get("lines", 0))
        for f, meta in old.items()
        if found.get(f, 0) > meta.get("lines", 0)
    ]
    if grown and not reason:
        print(
            "✗ 拒绝重新基线：这些文件**变长了** ⇒ 先拆模块；"
            "确属合法修复要带 `--reason '<为什么>'`（会记进基线、评审可见 ✓）✗\n  "
            + "\n  ".join(f"{f}（{n} > 基线 {cap}）" for f, n, cap in grown),
            file=sys.stderr,
        )
        return 2
    if grown:
        print(f"⚠ 接受 {len(grown)} 个文件的增长（reason 已记入基线，评审可见 ✓）：")
        for f, n, cap in grown:
            print(f"    {f}: {cap} → {n}（+{n - cap}）")
    grown_map = {f: cap for f, _, cap in grown}
    out = {}
    for f, n in sorted(found.items()):
        meta = dict(old.get(f) or {})
        entry = {
            "lines": n,
            "reason": meta.get("reason")
            or "硬规则 5 的存量债务：模块超 ~500 行，待拆分。**只许减不许增** ✓",
        }
        if f in grown_map:
            entry["grew_from"] = grown_map[f]
            entry["growth_reason"] = reason
        out[f] = entry
    SRC_LINES.write_text(
        json.dumps(
            {
                "_comment": (
                    "判据 ⑨ 的存量基线（STRUCT-SRC-LINES）：`crates/*/src/**/*.rs` 里超过 "
                    f"{SRC_SOFT_LINES} 行的生产源码（**不含**测试聚合文件）。"
                    "**只许减不许增** ✓ —— 变长立刻判红（先拆模块）；降到 ≤ 软线后必须从"
                    "这里删掉，否则判红（基线腐烂 ✗）。"
                ),
                "soft_limit": SRC_SOFT_LINES,
                "files": out,
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"src-lines 基线已更新：{len(out)} 个文件（最长 {max((v['lines'] for v in out.values()), default=0)} 行）")
    return 0


def main(argv: list[str]) -> int:
    # ⭐ **ratchet 状态闸门在最前**（连 `--help` 都不放过 ⇒ 状态坏时**什么模式都不假装判过** ✓）
    _gate = _ratchet_gate()
    if _gate is not None:
        return _gate
    # ⚠ **不许静默回落** ✗✓（E00 切片 B 实测 ✓）：这几个脚本原来用 `"--x" in argv`
    # **子串**判模式 ⇒ **错拼的参数被静默忽略、回落成全量检查并 exit 0** ✗ ⇒
    # 表现是「**自检没跑，退出码却是绿的**」✗（我上次"它没有自检"的错结论就是这么来的 ✓）。
    # ⇒ 未知参数一律 **exit 2**（只有 `notation-lint.py` 本来就用 argparse、是对的 ✓）。
    unknown = [
        a
        for a in argv
        if a.startswith("-")
        and a
        not in {
            "--help",
            "-h",
            "--freeze",
            "--selftest",
            "--json",
            "--check-doc-refs",
            "--freeze-doc-refs",
            "--check-src-lines",
            "--freeze-src-lines",
            "--reason",
        }
    ]
    if unknown:
        print(f"✗ 未知参数 {unknown} ⇒ 拒绝执行（**不许静默回落全量检查** ✗）", file=sys.stderr)
        return 2
    if "--help" in argv or "-h" in argv:
        print(__doc__)
        return 0
    if "--check-src-lines" in argv:
        return check_src_lines_only()
    if "--freeze-src-lines" in argv:
        _r = ""
        if "--reason" in argv:
            _i = argv.index("--reason")
            _r = argv[_i + 1] if _i + 1 < len(argv) else ""
        return freeze_src_lines(_r)
    if "--check-doc-refs" in argv:
        return check_doc_refs_only()
    if "--freeze-doc-refs" in argv:
        return freeze_doc_refs()
    if "--freeze" in argv:
        return freeze()
    if "--selftest" in argv:
        return selftest()
    try:
        bad, stats = check()
    except subprocess.CalledProcessError as e:  # git 不可用
        print(f"docs-lint: 环境错：{e}", file=sys.stderr)
        return 2
    if "--json" in argv:
        print(json.dumps({**stats, "ok": not bad, "problems": bad}, ensure_ascii=False, indent=2))
        return 1 if bad else 0
    if bad:
        print(f"docs-lint：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad:
            print(f"  - {b}", file=sys.stderr)
        return 1
    print(
        f"docs-lint ✓ 活文档 {stats['live_files']} 个 / {stats['live_bytes'] / 1e6:.2f} MB "
        f"≤ {MAX_LIVE_BYTES / 1e6:.2f} MB ✓ · 归档 {stats['archive_files']} 个 / "
        f"{stats['archive_bytes'] / 1e6:.2f} MB ✓ · 垃圾 0 ✓ · 归档索引齐 ✓ · "
        f"悬空文档引用 {stats.get('doc_refs_dangling', 0)} 处（基线内、无新增 ✓）· "
        f"超软线源码 {stats.get('src_over_soft', 0)} 个（基线内、无变长 ✓）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
