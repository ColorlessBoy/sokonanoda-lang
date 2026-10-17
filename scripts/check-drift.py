#!/usr/bin/env python3
"""**「声明说 X，实现是 Y」的漂移守卫**：一本登记表 + 一个驱动器（`SUB-narrow-guards-to-one-layer` ① ✓）。

## 合并前 = 两个文件（**同一个关注点被拆成两条守卫** ✗）

| # | 文件 | 行数 | 断言 | 量什么 |
|---|---|---|---|---|
| ① | `scripts/check-doc-drift.py` | 228 | **15**（F1–F5 事实 + D1a–D1e/D2a–D2c/D3a/D3b 文档） | harness 接线的**文档** vs 实现（三处 README ↔ 四份实现文件） |
| ② | `scripts/check-launcher-extension-link.py` | 294 | **4**（C1–C4） | `scripts/soko` 的「VS Code 扩展自带」那一格：**声明**（头注释 / `AGENTS.md` 写着解析链含它）vs **实现**（曾经是死分支 ✗） |

⇒ 合并成 **`scripts/check-drift.py`**（条目原文 ①：「合成一个 `check-drift.py`：
**登记表（声明 → 断言） + 一个驱动器**」✓）。

## 为什么这两条是**同一个关注点**（不是"看着像" ✗ —— 逐条对齐过）

* **同一条判据**：两条都在量「**声明/文档说 X，实现是 Y**」。
  ① 的 X 写在三处 README 里（**散文**）；② 的 X 写在 `scripts/soko` 的头注释与 `AGENTS.md` 里，
  而 Y 是一条**调用链**（`extensionServer()` 有定义、`canExec()` 有分支、**没有调用点** ✗）。
  ⇒ 断言的**形状**不同（① 读文本 · ② 跑真启动器夹具），**关注点相同** ✓。
* **同一个锚**：`docs/guard-anchors.jsonl` 里两行都是 `class=B` · `anchors=["A-HARNESS-DOCS-MATCH"]` ✓。
* **同一个 CI job**：`ci.yml` 的 `wiring-lint`（两条各一步 `run:`，本文件落地后并成一步 ✓）。
* **同一批接线点**：`ci.yml` · `scripts/ci-local.sh` · `scripts/soko gate` —— 三条路都**成对**出现 ✓
  （改一条忘另一条就是"接了一半" ✗）。
* **同一个病的两半**：都是 2026-10-15 LINK-B 那轮的产物，抬头写着同一句
  「对齐做完了、却**没有守卫钉住它**」⇒ 一个管**文档侧**、一个管**代码侧** ✓。

## 两层（合并后仍分得开 ✓ —— 域不同，**一个字都没收窄** ✗）

* **层① 文档漂移**（15 条）：域 = **读文本**。先量事实（`opencode.json` 无 `lsp` 块 ·
  shim 只转发 · `scripts/soko` **读** `SOKONANODA_OFFLINE`、从不读 LSP 那个别名 ·
  插件改写 `lsp.sokonanoda.command`），再对三处 README 的**事实**（**不冻结散文** ✗ ——
  换词/重排/加句子都不判红，判红的只有"事实被写反"）。
* **层② 扩展链接**（4 条）：域 = **跑真启动器**（自带夹具：临时 `HOME` / 缓存 /
  空 `CARGO_TARGET_DIR` / `SOKONANODA_OFFLINE=1` ⇒ 一个字节都不碰真环境 ✓）。
  四条都是「**用户动作 ⇒ 可见结果**」（`AGENTS.md` 验证设计纪律 0(a) ✓）。

⚠ **两条域都没有被收窄来换合并成功** ✗（`gs2` 的判例 (iii)）：① 仍然读**全部七份**真文件、
三处 README 一条不少；② 仍然四条、仍然真跑启动器。合并**只动夹具** ——
`Fixture` 现在能接受一份**从 git rev 取出来的启动器**（`--rev`，见下）✓。

## 咬住证明：`--rev <rev>`（**整个守卫指向历史某一版树** ✓）

`755e5302^`（= 2026-10-15 那轮对齐**之前**的 main）是**一条同时带着两个坏输入的 rev**：
* 层①：三处 README 都是**漂移前**的旧文（`README.md` 把接线归给 `opencode.json` +
  用 `SOKONANODA_LSP_OFFLINE` · `skills/README.md` 没有「只做转发」·
  `editor/vscode/README.md` 写着 "repo-root `opencode.json` already wires"）；
* 层②：`scripts/soko` 里 `extensionServer()` **有定义、没有调用点**（正是那个洞的形状 ✗）。

⇒ `python3 scripts/check-drift.py --rev 755e5302^` **一条命令**两层同时判红并**点名**
（层① 的 D1b/D1d/D2a/D3b + 层② 的 C1/C2）—— 这就是 `docs/guard-bite.jsonl` 那一行的
**复合 `bite`** ✓（`gs2` 的判例 (ii)：**原来两条判的每一条坏输入，合并后仍然判红** ✓）。
⚠ `--rev` 是**只读**的（`git show`，`git_env.clean_git_env()` ✓）—— **不换工作区文件** ✗，
因此不会踩 `BITE-RESTORE-CLOBBERS-UNCOMMITTED`（把开发者未提交的改动抹掉）那条洞 ✓。

## 判据登记表（**声明 → 断言**；驱动器在文件尾）

`REQUIRED_ASSERTIONS`（冻结期望 **22** 条 = 绿跑 19 + 锚点回退 3）⇄ `CRITERIA`（登记表）
——**分开写** ⇒ 从登记表里删掉一条 ⇒ `--selftest` 报出**缺的那条** ✓（验收 ③ 的原话）；
`GREEN_PATH_ASSERTIONS`（**19** 条）另有一条更硬的闸：**一次绿跑必须把 19 条全部产出**
（删掉代码里的断言、只留表 ⇒ 照样判红 ✓ —— 这比"表 ⇄ 表"对账更咬得住 ✓）。

## 用法

    python3 scripts/check-drift.py                    # 判据（gate）· **两层都跑**
    python3 scripts/check-drift.py --layer doc|link    # 只跑一层（两层域不同，调试用 ✓）
    python3 scripts/check-drift.py --json             # 机器可读（形状与合并前逐字相同 ✓）
    python3 scripts/check-drift.py --selftest         # **两套自检 + 登记表对账 + 绿跑对账**
    python3 scripts/check-drift.py --rev 755e5302^    # 咬住证明：指向历史某一版树（两层都判红 ✓）

退出码：0 = 一致 · 1 = **漂移**（判红并点名哪一条）· 2 = 形状异常（锚点/段落找不到 ·
夹具造不出来 ⇒ **判不了就不假装判过** ✗）· 3 = **仓库/环境形状不对**（缺文件 · 没有 node
—— 与 gate 其它步同一条纪律：探不到就 exit 3，绝不静默通过 ✗）。

⚠ **合并后 `node` 成了整条守卫的前提**（层②要真跑启动器）：以前只跑
`check-doc-drift.py` 的机器不需要 node。这不是收窄、是**前提并集** ✓ ——
两条路（`ci.yml` 的 `wiring-lint` · `scripts/ci-local.sh`）本来就**成对**跑，
CI runner 自带 node，`ci-local.sh` 的 `editor：stub 宿主` 早就硬依赖它 ✓。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve()
REPO = HERE.parent.parent

sys.path.insert(0, str(HERE.parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓ —— 洞
#                 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`：仓库发现只许由 cwd 决定 ✓）

LAYER_DOC = "doc"
LAYER_LINK = "link"

# ══════════════════════════════════════════════════════════════════════════════
# 层① 文档漂移（原 `scripts/check-doc-drift.py`，15 条断言，逐字保留 ✓）
#
# 判据：**先量事实、再对文档**
#
# **事实**（从实现里量出来，不是抄文档 ✓）：
#
# | # | 事实 |
# |---|---|
# | F1 | `opencode.json` **没有** `lsp` 块（LSP 由启动插件接线） |
# | F2 | shim `.opencode/lsp/sokonanoda-lsp.sh` **只做转发**（`exec "$launcher" lsp`） |
# | F3 | shim 声明 `SOKONANODA_BIN` 是逃生门 |
# | F4 | `scripts/soko` 认 `SOKONANODA_OFFLINE`、**不认** `SOKONANODA_LSP_OFFLINE`（后者是**二进制**下载路的合法别名，见 `crates/cli/src/env/target.rs`，不是启动器这条路的名字） |
# | F5 | 插件改写 `lsp.sokonanoda.command` |
#
# **文档**（三处，必须与上面的事实一致）：
#
# | # | 处 | 断言 |
# |---|---|---|
# | D1 | `README.md` 的 `Editor feedback for agents` 段 | 点名插件 · 不把接线归给 `opencode.json` · shim = forward only · 用 `SOKONANODA_OFFLINE`（不用 `SOKONANODA_LSP_OFFLINE`）· 点名 `SOKONANODA_BIN` |
# | D2 | `skills/README.md` 的 shim 段 | 「只做转发」· 「自己不选二进制」· 点名 `SOKONANODA_BIN` |
# | D3 | `editor/vscode/README.md` 的 opencode 段 | 点名插件 · 不把接线归给 `opencode.json` |
#
# ⚠ 文档侧**只锚定「谁接线 / 谁是转发 / 认哪个环境变量 / 逃生门叫什么」这类事实**，
# **不冻结散文** —— 换词、重排、加句子都不判红 ✓（判红的只有"事实被写反"）。
# ══════════════════════════════════════════════════════════════════════════════

DOC_README_ANCHOR = "Editor feedback for agents"
DOC_SKILLS_ANCHOR = "sokonanoda-lsp.sh"
DOC_EDITOR_ANCHOR = ".opencode/plugins/sokonanoda.ts"

# 反向验证用的**旧文**（`git show 755e5302^` 的原文；`--selftest` 把它们放回去，
# 三处必须各自判红 ✓）。**只作夹具**，不是文档的第二个真相。
OLD_README = """Editor feedback for agents needs no extra tooling: the repo-root
`opencode.json` starts the LSP through `.opencode/lsp/sokonanoda-lsp.sh`, which
resolves an installed VS Code extension binary, a local build, or the
version-pinned GitHub Release binary (auto-download; set
`SOKONANODA_LSP_OFFLINE=1` to forbid the network step). CLI-based agents get
the same contract via `--json` and `watch`."""
OLD_SKILLS = """其他 harness 若需要 LSP：`.opencode/lsp/sokonanoda-lsp.sh` shim →
`scripts/soko lsp`。"""
OLD_EDITOR = """The repo-root `opencode.json` already wires the language server for"""

# 层① 要读的**七份**文件（四份实现 + 三处文档）。`--rev` 时全部从那个 rev 取 ✓。
DOC_FILES: tuple[str, ...] = (
    "opencode.json",
    ".opencode/lsp/sokonanoda-lsp.sh",
    ".opencode/plugins/sokonanoda.ts",
    "scripts/soko",
    "README.md",
    "skills/README.md",
    "editor/vscode/README.md",
)


def paragraph(text: str, anchor: str) -> str | None:
    """锚点所在的整段（到空行为止）。找不到 ⇒ `None`（判红，而不是丢栈 ✗）。"""
    idx = text.find(anchor)
    if idx < 0:
        return None
    start = text.rfind("\n\n", 0, idx) + 2
    end = text.find("\n\n", idx)
    return text[start : end if end >= 0 else len(text)]


def _read(root: Path, rel: str, overrides: dict[str, str]) -> str:
    return overrides.get(rel, (root / rel).read_text(encoding="utf-8"))


def doc_results(root: Path, overrides: dict[str, str] | None = None) -> list[dict]:
    """层① 全部判据（15 条）。`overrides`（相对路径 → 文本）供 `--selftest` / `--rev` 喂 ✓。"""
    ov = overrides or {}
    results: list[dict] = []

    def add(cid: str, what: str, ok: bool, detail: str = "") -> None:
        _add(results, cid, what, ok, detail)

    # ── 事实（从实现里量）────────────────────────────────────────────────
    cfg_text = _read(root, "opencode.json", ov)
    cfg = json.loads(cfg_text)
    add("F1", "opencode.json 没有 `lsp` 块", "lsp" not in cfg, f"keys={list(cfg.keys())}")

    shim = _read(root, ".opencode/lsp/sokonanoda-lsp.sh", ov)
    add("F2", "shim 只做转发（`exec \"$launcher\" lsp`）", 'exec "$launcher" lsp' in shim)
    add("F3", "shim 声明 `SOKONANODA_BIN` 是逃生门", "SOKONANODA_BIN" in shim)

    soko = _read(root, "scripts/soko", ov)
    # ⚠ 判据是**行为**（读哪个环境变量），不是"这个名字出现过没有" —— 2026-10-15 LINK-B
    # 实测：第一版按裸名字计数，结果**咬到了自己的注释**（gate 里那句「认 X 不认 Y」
    # 把 Y 写了出来 ⇒ 假红 ✗）。守卫不该禁止**记录**这个事实，只该禁止**读它** ✓。
    n_off = soko.count("process.env.SOKONANODA_OFFLINE")
    n_lsp_off = soko.count("process.env.SOKONANODA_LSP_OFFLINE")
    add(
        "F4",
        "scripts/soko **读** SOKONANODA_OFFLINE、从不读 SOKONANODA_LSP_OFFLINE",
        n_off > 0 and n_lsp_off == 0,
        f"process.env.OFFLINE={n_off} process.env.LSP_OFFLINE={n_lsp_off}",
    )

    plugin = _read(root, ".opencode/plugins/sokonanoda.ts", ov)
    add("F5", "插件改写 `lsp.sokonanoda.command`", "lsp.sokonanoda.command" in plugin)

    # ── 文档（必须与事实一致）────────────────────────────────────────────
    readme = _read(root, "README.md", ov)
    para = paragraph(readme, DOC_README_ANCHOR)
    if para is None:
        add("D1", f"README.md 有 `{DOC_README_ANCHOR}` 段", False, "锚点找不到（段落被改名/删掉）")
    else:
        add("D1a", "README: 点名插件是接线的那一方", ".opencode/plugins/sokonanoda.ts" in para)
        add("D1b", "README: 不把接线归给 `opencode.json`", "`opencode.json` starts" not in para)
        add("D1c", "README: shim 是 forward only", "forward only" in para)
        add("D1d", "README: 用 SOKONANODA_OFFLINE、不用 SOKONANODA_LSP_OFFLINE",
            "SOKONANODA_OFFLINE" in para and "SOKONANODA_LSP_OFFLINE" not in para)
        add("D1e", "README: 点名逃生门 SOKONANODA_BIN", "SOKONANODA_BIN" in para)

    skills = _read(root, "skills/README.md", ov)
    seg = paragraph(skills, DOC_SKILLS_ANCHOR)
    if seg is None:
        add("D2", f"skills/README.md 有含 `{DOC_SKILLS_ANCHOR}` 的段", False, "锚点找不到")
    else:
        add("D2a", "skills/README: shim「只做转发」", "只做转发" in seg)
        add("D2b", "skills/README: 「自己不选二进制」", "自己不选二进制" in seg)
        add("D2c", "skills/README: 点名逃生门 SOKONANODA_BIN", "SOKONANODA_BIN" in seg)

    editor = _read(root, "editor/vscode/README.md", ov)
    # 这条是**全文件**断言（不依赖段落锚点）⇒ 锚点被删时它照样判红 ✓
    add("D3b", "editor/vscode/README: 不把接线归给 `opencode.json`",
        "repo-root `opencode.json` already wires" not in editor)
    eseg = paragraph(editor, DOC_EDITOR_ANCHOR)
    if eseg is None:
        add("D3", f"editor/vscode/README.md 有含 `{DOC_EDITOR_ANCHOR}` 的段", False, "锚点找不到")
    else:
        add("D3a", "editor/vscode/README: 点名插件接线", DOC_EDITOR_ANCHOR in eseg)

    return results


def _doc_counterexamples(root: Path) -> list[tuple[str, dict[str, str], tuple[str, ...]]]:
    """三处旧文（每处一个反例）+ 该处**必须**判红的 id ✓。"""
    readme = (root / "README.md").read_text(encoding="utf-8")
    skills = (root / "skills/README.md").read_text(encoding="utf-8")
    editor = (root / "editor/vscode/README.md").read_text(encoding="utf-8")

    def swap(text: str, anchor: str, old: str) -> str | None:
        seg = paragraph(text, anchor)
        return None if seg is None else text.replace(seg, old, 1)

    cases = []
    r1 = swap(readme, DOC_README_ANCHOR, OLD_README)
    if r1 is not None:
        cases.append(("README.md（旧文：opencode.json 接线 + SOKONANODA_LSP_OFFLINE）",
                      {"README.md": r1}, ("D1b", "D1d")))
    r2 = swap(skills, DOC_SKILLS_ANCHOR, OLD_SKILLS)
    if r2 is not None:
        cases.append(("skills/README.md（旧文：没有「只做转发」）", {"skills/README.md": r2}, ("D2a",)))
    r3 = swap(editor, DOC_EDITOR_ANCHOR, OLD_EDITOR)
    if r3 is not None:
        cases.append(("editor/vscode/README.md（旧文：repo-root opencode.json 接线）",
                      {"editor/vscode/README.md": r3}, ("D3b",)))
    return cases


def selftest_doc() -> int:
    """层① 自检（原 `check-doc-drift.py --selftest`，逐字保留 ✓）：三处旧文必须判红。"""
    cases = _doc_counterexamples(REPO)
    if len(cases) < 3:
        print(f"层① --selftest: 只造出 {len(cases)}/3 个反例（锚点找不到）⇒ exit 2 ✗", file=sys.stderr)
        return 2
    bad = 0
    for name, ov, must_be_red in cases:
        results = doc_results(REPO, ov)
        red = {r["id"] for r in results if not r["ok"]}
        hit = [cid for cid in must_be_red if cid in red]
        ok = len(hit) == len(must_be_red)
        bad += 0 if ok else 1
        print(f"  {'✓' if ok else '✗'} 层①反例：{name} ⇒ 判红 {sorted(red) or '（无！）'}")
    if bad:
        print(f"  ⇒ **咬不住**：{bad}/3 个反例没被判红 ✗", file=sys.stderr)
        return 1
    print("  ⇒ 层①反向验证成立：三处旧文各自判红并点名 ✓（守卫真的在咬）")
    return 0


# ══════════════════════════════════════════════════════════════════════════════
# 层② 启动器「VS Code 扩展自带」链接（原 `scripts/check-launcher-extension-link.py`，4 条，逐字保留 ✓）
#
# ## 为什么需要它
#
# `scripts/soko` 的头注释与 `AGENTS.md` 一直写着解析链含「VS Code 扩展自带」，
# `extensionServer()` 正是为它写的函数、`canExec()` 也留着 `extension` 分支 ——
# 但**没有任何调用点**：`grep -n extensionServer scripts/soko` = 1 定义 / **0 调用**，
# 真实链 = override → repo-build → cache → download ⇒ 那一格是**死的** ✗。
# 后果是**产品行为**（不只是文档）：**通用 VSIX + 用户没跑过「Install CLI」+ 离线**
# ⇒ `scripts/soko lsp` 只能下载（或被 offline 拒绝），而盘上明明有一份能用的服务器 ✗。
#
# ⚠ **为什么不能只靠 `crates/cli/tests/launcher.rs`**：那是**冻结用例**，而且它的夹具
# **不设 `HOME`**（`run_env` 只 `env_remove` 了几个 `SOKONANODA_*`）⇒ 它测不了
# 「扩展在场 / 不在场 / 版本对不对」这一格（本机装了同版本 VSIX 时反而会**偶然**走到）。
# 本守卫自带夹具：**临时 HOME + 临时缓存 + 空 `CARGO_TARGET_DIR` + `SOKONANODA_OFFLINE=1`**
# ⇒ 一个字节都不碰真环境 ✓（真 `~/.vscode` 里那 8 个扩展目录不参与）。
#
# ## 判据（四条，都是「用户动作 ⇒ 可见结果」）
#
# | # | 夹具 | 期望 |
# |---|---|---|
# | C1 | 钉 0.90.0 + 扩展自带 0.90.0 + 空缓存 + 离线 | `version --json` 的 `lsp.source == "extension"` 且路径 = 自带那份；`scripts/soko lsp` **真的跑**它（哨兵 + rc 0） |
# | C2 | 同上，扩展是 0.89.0（**与钉不匹配**） | `lsp.source != "extension"`；`lsp` **rc 3 且哨兵没跑**（`canExec` 不许松 ✓）；拒绝消息**点名**那份扩展（否则用户看着已装的 VSIX 无从理解 ✗） |
# | C3 | 版本**匹配的缓存条目** + 匹配的扩展 | `lsp.source == "cache"` —— 缓存先于扩展（顺序与头注释 / opencode 插件一致 ✓） |
# | C4 | 空缓存 + 匹配的扩展，走 **CLI**（`grade`） | CLI 仍是「什么都没解析到」（rc 3）—— 扩展那一格**只给语言服务器** ✓（文档承诺的就是它） |
#
# ⚠ `target` 从**启动器自己**的 `version --json` 读（不在这里复制一份平台映射 ✗）。
# ⚠ **Windows**：夹具里的哨兵是 `sh` 脚本（跑不了）⇒ C1/C3 的「真的跑」退化成
# `doctor --json` 的 `lsp.ready == true`（**仍然**是「exec 闸门开了」的读数），
# 并**如实打印**这个降级 ✓（不是静默跳过 ✗）。
# ══════════════════════════════════════════════════════════════════════════════

LAUNCHER = REPO / "scripts" / "soko"

# 夹具自己的版本钉（夹具仓没有 `Cargo.toml` ⇒ 走 `sokonanoda-version.txt` ✓）。
# **有意不读仓库版本**：判据与"今天是哪个版本"无关 ⇒ 不会每次 bump 就假红 ✓。
PIN = "0.90.0"
MISMATCH = "0.89.0"

WINDOWS = sys.platform == "win32"
SENTINEL_EXT = "SENTINEL-EXTENSION-LSP-EXECUTED"
SENTINEL_CACHE = "SENTINEL-CACHE-LSP-EXECUTED"


def _env(home: Path, cache: Path, target_dir: Path) -> dict[str, str]:
    """最小环境：真环境里的 `SOKONANODA_*` / 真 `HOME` 一律不带进去 ✓。"""
    env = {
        "PATH": os.environ.get("PATH", ""),
        "HOME": str(home),
        "USERPROFILE": str(home),  # Windows 上启动器读的是它
        "SOKONANODA_CACHE_DIR": str(cache),
        "CARGO_TARGET_DIR": str(target_dir),
        "SOKONANODA_OFFLINE": "1",
    }
    if os.environ.get("SystemRoot"):
        env["SystemRoot"] = os.environ["SystemRoot"]  # node 在 Windows 上要它
    return env


class Fixture:
    """一个自足的假课程仓：`repo/scripts/soko` + 版本钉 + 假 HOME / 缓存 / 空 target。

    ⚠ `--rev` 时 `launcher` 是**从 git rev 取出来的那一份**（写进临时目录的副本 ✓）——
    夹具**照旧只认一个路径**，不因此收窄任何断言 ✓（合并的判例 (iii)：**改夹具**，不改域）。
    """

    def __init__(self, root: Path, launcher: Path):
        self.root = root
        self.launcher = launcher
        self.home = root / "home"
        self.cache = root / "cache"
        self.empty_target = root / "empty-target"
        self.repo = root / "repo"
        for path in (self.home, self.cache, self.empty_target, self.repo / "scripts"):
            path.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(launcher, self.repo / "scripts" / "soko")
        (self.repo / "sokonanoda-version.txt").write_text(f"{PIN}\n", encoding="utf-8")

    def run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["node", str(self.repo / "scripts" / "soko"), *args],
            capture_output=True,
            text=True,
            env=_env(self.home, self.cache, self.empty_target),
            timeout=120,
        )

    def binary_name(self, base: str) -> str:
        return f"{base}.exe" if WINDOWS else base

    def _sentinel(self, path: Path, marker: str) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f'#!/bin/sh\necho "{marker}"\n', encoding="utf-8")
        path.chmod(0o755)
        return path

    def extension(self, version: str, target: str, marker: str) -> Path:
        """通用 VSIX 的安装目录形状（`editor/vscode/server.js:bundledBinaryPath`）。"""
        folder = self.home / ".vscode" / "extensions" / f"sokonanoda-lang.sokonanoda-{version}-{target}"
        return self._sentinel(folder / "bin" / target / self.binary_name("sokonanoda-lsp"), marker)

    def cache_entry(self, base: str, marker: str | None, sentinel: str) -> Path:
        binary = self._sentinel(self.cache / self.binary_name(base), sentinel)
        if marker is not None:
            (self.cache / f"{base}.version").write_text(f"{marker}\n", encoding="utf-8")
        return binary


def _version_json(fx: Fixture) -> dict:
    out = fx.run("version", "--json")
    if out.returncode != 0 or not out.stdout.strip():
        raise RuntimeError(f"`version --json` 没给出 JSON（rc={out.returncode}）: {out.stderr.strip()[:400]}")
    return json.loads(out.stdout)


def _exec_readings(fx: Fixture, sentinel: str) -> tuple[bool, str]:
    """「真的跑了自带那份」的读数；Windows 上退化成 `doctor --json` 的 `lsp.ready` ✓。"""
    if WINDOWS:
        out = fx.run("doctor", "--json")
        try:
            ready = bool(json.loads(out.stdout).get("lsp", {}).get("ready"))
        except (ValueError, AttributeError):
            ready = False
        return ready, f"doctor --json lsp.ready={ready}（Windows：sh 哨兵跑不了 ⇒ 只读闸门 ✓）"
    out = fx.run("lsp")
    ran = sentinel in out.stdout and out.returncode == 0
    return ran, f"`lsp` rc={out.returncode} stdout={out.stdout.strip()[:60]!r}"


def _same_path(a: str | None, b: Path) -> bool:
    if not a:
        return False
    return Path(a).resolve() == b.resolve()


def link_results(launcher: Path, root: Path) -> tuple[list[dict], str | None]:
    """层② 四条判据。返回 (results, fatal)；`fatal` 非空 = 形状异常（exit 2）。"""
    results: list[dict] = []

    def add(cid: str, what: str, ok: bool, detail: str) -> None:
        _add(results, cid, what, ok, detail)

    # `target` 从启动器自己读（不复制平台映射 ✓）
    probe = Fixture(root / "probe", launcher)
    target = _version_json(probe).get("target")
    if not target:
        return results, f"启动器没报出 target（`version --json` 的 `target` 为空）⇒ 夹具无法造"

    # C1 —— 洞本身：通用 VSIX + 空缓存 + 离线 ⇒ 必须走扩展自带
    fx = Fixture(root / "c1", launcher)
    bundled = fx.extension(PIN, target, SENTINEL_EXT)
    report = _version_json(fx)
    ran, reading = _exec_readings(fx, SENTINEL_EXT)
    add(
        "C1",
        f"空缓存 + 扩展自带 v{PIN}（= 钉）+ 离线 ⇒ lsp 走 extension 且真的跑它",
        report.get("lsp", {}).get("source") == "extension"
        and _same_path(report.get("lsp", {}).get("path"), bundled)
        and ran,
        f"source={report.get('lsp', {}).get('source')!r} path={report.get('lsp', {}).get('path')!r} · {reading}",
    )

    # C2 —— 不许松：版本不匹配的扩展**不能**被 exec，而且要被点名
    fx = Fixture(root / "c2", launcher)
    fx.extension(MISMATCH, target, SENTINEL_EXT)
    report = _version_json(fx)
    out = fx.run("lsp")
    named = "extension" in out.stderr and MISMATCH in out.stderr
    add(
        "C2",
        f"扩展自带 v{MISMATCH}（≠ 钉 v{PIN}）⇒ 不 exec + 拒绝时点名",
        report.get("lsp", {}).get("source") != "extension"
        and out.returncode == 3
        and SENTINEL_EXT not in out.stdout
        and named,
        f"source={report.get('lsp', {}).get('source')!r} rc={out.returncode} "
        f"点名={'yes' if named else 'NO'} stdout={out.stdout.strip()[:40]!r}",
    )

    # C3 —— 顺序：版本匹配的缓存条目仍然先赢（回归守卫）
    fx = Fixture(root / "c3", launcher)
    fx.extension(PIN, target, SENTINEL_EXT)
    cached = fx.cache_entry("sokonanoda-lsp", f"{PIN} {target}", SENTINEL_CACHE)
    report = _version_json(fx)
    ran, reading = _exec_readings(fx, SENTINEL_CACHE)
    add(
        "C3",
        "版本匹配的缓存条目 + 匹配的扩展 ⇒ lsp 仍走 cache（缓存先于扩展）",
        report.get("lsp", {}).get("source") == "cache"
        and _same_path(report.get("lsp", {}).get("path"), cached)
        and ran,
        f"source={report.get('lsp', {}).get('source')!r} · {reading}",
    )

    # C4 —— 范围：这一格只给语言服务器；CLI 不许因此从扩展里被 exec
    fx = Fixture(root / "c4", launcher)
    fx.extension(PIN, target, SENTINEL_EXT)
    report = _version_json(fx)
    out = fx.run("grade", "u.sokonanoda")
    add(
        "C4",
        "同一夹具走 CLI（grade）⇒ 仍然拒绝（扩展那一格只给 LSP）",
        "source" not in report.get("cli", {})
        and out.returncode == 3
        and SENTINEL_EXT not in out.stdout,
        f"cli.source={report.get('cli', {}).get('source')!r} rc={out.returncode}",
    )

    return results, None


def _unwire(launcher: Path, dest: Path) -> bool:
    """把链接**拆掉**（正是洞的形状：函数在、调用点不在）—— 反向验证用。"""
    text = launcher.read_text(encoding="utf-8")
    wired = "      const bundled = this.extensionLink()\n      if (bundled) return bundled\n"
    if wired not in text:
        return False
    dest.write_text(text.replace(wired, "      // unwired (--selftest)\n"), encoding="utf-8")
    return True


def selftest_link(root: Path) -> int:
    """层② 自检（原 `check-launcher-extension-link.py --selftest`，逐字保留 ✓）：拆掉链接必须判红。"""
    unwired = root / "soko-unwired.mjs"
    if not _unwire(LAUNCHER, unwired):
        print("层② --selftest: 造不出反例（找不到要拆的那两行）⇒ exit 2 ✗", file=sys.stderr)
        return 2
    results, fatal = link_results(unwired, root / "unwired")
    if fatal:
        print(f"层② --selftest: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
        return 2
    _print_layer(LAYER_LINK, results)
    if all(r["ok"] for r in results):
        print("  ⇒ **咬不住**：把链接拆掉之后四条判据仍然全绿 ⇒ 这守卫是空的 ✗", file=sys.stderr)
        return 1
    red = [r["id"] for r in results if not r["ok"]]
    print(f"  ⇒ 层②反向验证成立：拆掉链接 ⇒ {', '.join(red)} 判红 ✓（守卫真的在咬）")
    return 0


# ══════════════════════════════════════════════════════════════════════════════
# 共用形状 + 层①/层② 的入口表（**一处定义** ✓ —— 合并前两个文件各写了一遍 ✗）
# ══════════════════════════════════════════════════════════════════════════════

LAYER_TITLE = {
    LAYER_DOC: "① 文档漂移（harness 接线的三处 README ↔ 实现）",
    LAYER_LINK: "② 扩展链接（启动器「VS Code 扩展自带」那一格）",
}


def _add(results: list[dict], cid: str, what: str, ok: bool, detail: str = "") -> None:
    """结果形状（**一处定义** ✓）：两层共用同一份 `{"id","what","ok","detail"}`。"""
    results.append({"id": cid, "what": what, "ok": bool(ok), "detail": detail})


def _print_layer(layer: str, results: list[dict]) -> None:
    """一层的可读输出（**一处定义** ✓ —— 合并前两个文件各有一份 `_print` ✗）。"""
    print(f"[{LAYER_TITLE[layer]}] {len(results)} 条")
    for r in results:
        print(f"  {'✓' if r['ok'] else '✗'} {r['id']:<5} {r['what']}"
              f"{(' — ' + r['detail']) if r['detail'] else ''}")
    bad = [r["id"] for r in results if not r["ok"]]
    print(f"  ⇒ {'一致 ✓' if not bad else '判红：' + ', '.join(bad) + ' ✗'}")


def _print(by_layer: dict[str, list[dict]], as_json: bool) -> None:
    results = [r for layer in (LAYER_DOC, LAYER_LINK) for r in by_layer.get(layer, [])]
    if as_json:
        print(json.dumps({"results": results, "ok": all(r["ok"] for r in results)},
                         ensure_ascii=False, indent=2))
        return
    print("漂移守卫（scripts/check-drift.py）：一本登记表 + 一个驱动器 · 两层 = ① 文档漂移 · ② 扩展链接")
    for layer in (LAYER_DOC, LAYER_LINK):
        if layer in by_layer:
            _print_layer(layer, by_layer[layer])
    bad = [r["id"] for r in results if not r["ok"]]
    if not bad:
        print("  ⇒ 两层都一致 ✓")
        return
    # 逐层点名（`--rev` 的复合 bite 就 grep 这两段 ✓）
    parts = []
    for layer in (LAYER_DOC, LAYER_LINK):
        red = [r["id"] for r in by_layer.get(layer, []) if not r["ok"]]
        if red:
            parts.append(f"{'①' if layer == LAYER_DOC else '②'} {', '.join(red)}")
    print(f"  ⇒ 漂移（判红）：{' · '.join(parts)} ✗")


# ══════════════════════════════════════════════════════════════════════════════
# 判据登记表（**声明 → 断言**）+ 驱动器（`SUB-narrow-guards-to-one-layer` ① ✓）
# ══════════════════════════════════════════════════════════════════════════════
#
# 合并前是**两个文件**（`check-doc-drift.py` 228 行 · `check-launcher-extension-link.py`
# 294 行），各有自己的判据；合并后**一条判据都不能少** ✗ ⇒ 这张表把"少了哪条"变成
# **可机械判**的（验收 ①/③ 的原话：「合并前每条判据都要能在合并后的登记表里找到对应断言，
# 逐条对账，缺一条 ⇒ 判红」/「从登记表里故意删掉一条断言 ⇒ 驱动器必须报出缺的那条」✓）。
#
# ⚠ **期望（`REQUIRED_ASSERTIONS`）与登记表（`CRITERIA`）分开写** ⇒ 删掉登记表里的一条
#   ⇒ 驱动器报**缺的那条** ✓；两条一起删 = 两次编辑（评审看得见 ✓）。⚠ 如实划的边界 ✗：
#   这不是"外部锚"（同一份文件里的两个常量）—— 真正的外部记录是 `docs/guard-bite.jsonl`
#   那一行（它写的是**坏输入**与**命令**，不是断言清单 ✓）。
#
# ⚠⭐ 比 `gs2` 更硬的一档（**本轮加的**）：`GREEN_PATH_ASSERTIONS` 由 `--selftest` 在
#   **一次真绿跑**上对账 ⇒ 只删代码里的断言、登记表留着 ⇒ **照样判红** ✓（"表 ⇄ 表"对账
#   挡不住那种删法 ✗）。

# **绿跑必须全部出现的 19 条**（= 合并前两条判据的并集：① 15 条 + ② 4 条 ✓）
GREEN_PATH_ASSERTIONS: tuple[str, ...] = (
    "F1", "F2", "F3", "F4", "F5",
    "D1a", "D1b", "D1c", "D1d", "D1e",
    "D2a", "D2b", "D2c",
    "D3a", "D3b",
    "C1", "C2", "C3", "C4",
)

# 层① 的三条**锚点缺失回退**：只在锚点被改名/删掉时出现（那时本来就判红 ✓）⇒
# 它们**不**在"绿跑必须出现"里，但**必须**在登记表里（删掉回退 = 静默降级 ✗）。
ANCHOR_FALLBACK_ASSERTIONS: tuple[str, ...] = ("D1", "D2", "D3")

REQUIRED_ASSERTIONS: tuple[str, ...] = GREEN_PATH_ASSERTIONS + ANCHOR_FALLBACK_ASSERTIONS

# (id, 层, 一句话（判什么）, 判据入口（本文件里的函数名）)
CRITERIA: tuple[tuple[str, str, str, str], ...] = (
    ("F1", LAYER_DOC, "事实：opencode.json 没有 `lsp` 块（LSP 由启动插件接线）", "doc_results"),
    ("F2", LAYER_DOC, "事实：shim 只做转发（`exec \"$launcher\" lsp`）", "doc_results"),
    ("F3", LAYER_DOC, "事实：shim 声明 `SOKONANODA_BIN` 是逃生门", "doc_results"),
    ("F4", LAYER_DOC, "事实：scripts/soko **读** SOKONANODA_OFFLINE、从不读 LSP 别名", "doc_results"),
    ("F5", LAYER_DOC, "事实：插件改写 `lsp.sokonanoda.command`", "doc_results"),
    ("D1", LAYER_DOC, "回退：README.md 有 `Editor feedback for agents` 段（锚点在不在）", "doc_results"),
    ("D1a", LAYER_DOC, "README：点名插件是接线的那一方", "doc_results"),
    ("D1b", LAYER_DOC, "README：不把接线归给 `opencode.json`", "doc_results"),
    ("D1c", LAYER_DOC, "README：shim 是 forward only", "doc_results"),
    ("D1d", LAYER_DOC, "README：用 SOKONANODA_OFFLINE、不用 SOKONANODA_LSP_OFFLINE", "doc_results"),
    ("D1e", LAYER_DOC, "README：点名逃生门 SOKONANODA_BIN", "doc_results"),
    ("D2", LAYER_DOC, "回退：skills/README.md 有含 `sokonanoda-lsp.sh` 的段", "doc_results"),
    ("D2a", LAYER_DOC, "skills/README：shim「只做转发」", "doc_results"),
    ("D2b", LAYER_DOC, "skills/README：「自己不选二进制」", "doc_results"),
    ("D2c", LAYER_DOC, "skills/README：点名逃生门 SOKONANODA_BIN", "doc_results"),
    ("D3", LAYER_DOC, "回退：editor/vscode/README.md 有含插件路径的段", "doc_results"),
    ("D3a", LAYER_DOC, "editor/vscode/README：点名插件接线", "doc_results"),
    ("D3b", LAYER_DOC, "editor/vscode/README：不把接线归给 `opencode.json`（全文件断言）", "doc_results"),
    ("C1", LAYER_LINK, "空缓存 + 扩展自带（= 钉）+ 离线 ⇒ lsp 走 extension **且真的跑它**", "link_results"),
    ("C2", LAYER_LINK, "扩展版本 ≠ 钉 ⇒ 不 exec + 拒绝时**点名**（canExec 不许松）", "link_results"),
    ("C3", LAYER_LINK, "版本匹配的缓存条目 + 匹配的扩展 ⇒ lsp 仍走 cache（顺序）", "link_results"),
    ("C4", LAYER_LINK, "同一夹具走 CLI（grade）⇒ 仍拒绝（那一格只给 LSP）", "link_results"),
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


def produced_problems(by_layer: dict[str, list[dict]]) -> list[str]:
    """**一次绿跑必须把 19 条绿路断言全部产出**（比"表 ⇄ 表"更硬的一档 ✓）。

    ⚠ 为什么要有（`gs2` 如实留的那条边界 ✗ → 本轮补上 ✓）：期望与登记表在**同一个文件**
    里 ⇒ 只删**代码**里的断言、表留着 ⇒ "表 ⇄ 表"对账**照样绿** ✗。这条拿**真读数**对账
    ⇒ 那种删法**当场判红** ✓（并且报出**缺的那条**，与验收 ③ 同口径 ✓）。
    """
    produced = {r["id"] for r in by_layer.get(LAYER_DOC, [])} | {r["id"] for r in by_layer.get(LAYER_LINK, [])}
    missing = [cid for cid in GREEN_PATH_ASSERTIONS if cid not in produced]
    if not missing:
        return []
    return [f"**断言缺产出** ✗：{', '.join(missing)} 在冻结期望（绿路 19 条）里、"
            f"但**这一次绿跑根本没产出它** ⇒ 代码里的那条断言被删了/改名了（表还在 ⇒ "
            f"只对表是**看不出来**的 ✗）⇒ 补回判据本体，或（真要退役）**两处一起改**（评审可见 ✓）"]


# ── `--rev`：把整个守卫指向历史某一版树（**咬住证明**，只读 ✓）────────────────

def _git_show(rev: str, rel: str) -> str | None:
    """`git show <rev>:<rel>`（**干净 git 环境** ✓）；取不到 ⇒ `None`（判红，不丢栈 ✗）。"""
    out = subprocess.run(["git", "show", f"{rev}:{rel}"], cwd=REPO, capture_output=True,
                         text=True, timeout=120, env=git_env.clean_git_env())
    return out.stdout if out.returncode == 0 else None


def _rev_overrides(rev: str) -> dict[str, str] | None:
    """层① 的七份文件全部从那个 rev 取（缺一份 ⇒ `None` = 形状异常 ✓）。"""
    ov: dict[str, str] = {}
    for rel in DOC_FILES:
        text = _git_show(rev, rel)
        if text is None:
            return None
        ov[rel] = text
    return ov


# ══════════════════════════════════════════════════════════════════════════════
# 驱动器（合并后**唯一**的入口 ✓）：`--selftest` → 两套自检 + 两向对账；
# `--rev` → 咬住证明；`--layer` → 只跑一层；其余（默认）→ 两层都跑。
# ══════════════════════════════════════════════════════════════════════════════

def _collect(layers: tuple[str, ...], launcher: Path, root: Path,
             overrides: dict[str, str] | None) -> tuple[dict[str, list[dict]], str | None]:
    """两层的结果（按层分组）。返回 `(by_layer, fatal)`；`fatal` 非空 = 形状异常（exit 2）。"""
    by_layer: dict[str, list[dict]] = {}
    if LAYER_DOC in layers:
        by_layer[LAYER_DOC] = doc_results(REPO, overrides)
    if LAYER_LINK in layers:
        results, fatal = link_results(launcher, root)
        if fatal:
            return by_layer, fatal
        by_layer[LAYER_LINK] = results
    return by_layer, None


def _preconditions(layers: tuple[str, ...]) -> int | None:
    """前提（**探不到就 exit 3，绝不静默通过** ✗）。返回非 `None` = 直接退这个码。"""
    if LAYER_DOC in layers:
        missing = [rel for rel in DOC_FILES if not (REPO / rel).exists()]
        if missing:
            print(f"check-drift: 缺文件 {', '.join(missing)} ⇒ exit 3 ✗", file=sys.stderr)
            return 3
    if LAYER_LINK in layers:
        if shutil.which("node") is None:
            print("check-drift: 找不到 node —— 启动器就是 Node 脚本，层②的判据无法跑（exit 3）✗",
                  file=sys.stderr)
            return 3
        if not LAUNCHER.is_file():
            print(f"check-drift: 找不到 {LAUNCHER}（exit 3）✗", file=sys.stderr)
            return 3
    return None


def selftest_all() -> int:
    """**两套自检 + 登记表对账 + 绿跑对账**（`--selftest` 的唯一入口 ✓）。"""
    print(f"判据登记表（声明 → 断言）：冻结期望 **{len(REQUIRED_ASSERTIONS)}** 条"
          f"（绿路 {len(GREEN_PATH_ASSERTIONS)} + 锚点回退 {len(ANCHOR_FALLBACK_ASSERTIONS)}）· "
          f"登记 **{len(CRITERIA)}** 条")
    for cid, layer, what, entry in CRITERIA:
        print(f"  · {cid:<5} [{'①' if layer == LAYER_DOC else '②'}] {what}  ← {entry}()")
    red = registry_problems()
    for m in red:
        print(f"✗ {m}", file=sys.stderr)
    print()
    rc_doc = selftest_doc()
    print()
    with tempfile.TemporaryDirectory(prefix="soko-drift-selftest-") as tmp:
        rc_link = selftest_link(Path(tmp))
    print()
    # ⭐ 绿跑对账（**真读数** ⇄ 冻结期望）
    rc_green = 0
    pre = _preconditions((LAYER_DOC, LAYER_LINK))
    if pre is not None:
        rc_green = pre
    else:
        with tempfile.TemporaryDirectory(prefix="soko-drift-green-") as tmp:
            by_layer, fatal = _collect((LAYER_DOC, LAYER_LINK), LAUNCHER, Path(tmp), None)
        if fatal:
            print(f"✗ 绿跑对账：{fatal} ⇒ exit 2 ✗", file=sys.stderr)
            rc_green = 2
        else:
            green_red = produced_problems(by_layer)
            live_red = [r["id"] for r in by_layer[LAYER_DOC] + by_layer[LAYER_LINK] if not r["ok"]]
            for m in green_red:
                print(f"✗ {m}", file=sys.stderr)
            if live_red:
                print(f"✗ 绿跑对账：现役树上有判红的断言 {live_red} ✗", file=sys.stderr)
            rc_green = 1 if (green_red or live_red) else 0
            if rc_green == 0:
                print(f"  ⇒ 绿跑对账 ✓：{len(GREEN_PATH_ASSERTIONS)} 条绿路断言全部产出、且全绿 ✓")
    if red or rc_doc or rc_link or rc_green:
        print(f"✗ 合并后的自检判红：登记表 {len(red)} 条 · 层① rc={rc_doc} · 层② rc={rc_link} · "
              f"绿跑 rc={rc_green}", file=sys.stderr)
        return 1
    print(f"✓ 合并后的自检全过：登记表齐（{len(CRITERIA)}/{len(REQUIRED_ASSERTIONS)}）"
          f"+ 层① + 层② + 绿跑对账 ✓")
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true", help="机器可读")
    ap.add_argument("--selftest", action="store_true", help="两套自检 + 登记表/绿跑对账")
    ap.add_argument("--layer", choices=(LAYER_DOC, LAYER_LINK), default=None,
                    help="只跑一层（默认两层都跑 ✓）")
    ap.add_argument("--rev", default=None,
                    help="咬住证明：整个守卫指向历史某一版树（两层都跑 ⇒ 两层都判红 ✓）")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    if args.selftest:
        if args.layer == LAYER_DOC:
            return selftest_doc()
        if args.layer == LAYER_LINK:
            with tempfile.TemporaryDirectory(prefix="soko-drift-link-") as tmp:
                return selftest_link(Path(tmp))
        return selftest_all()

    layers: tuple[str, ...] = (args.layer,) if args.layer else (LAYER_DOC, LAYER_LINK)

    if args.rev:
        overrides = _rev_overrides(args.rev) if LAYER_DOC in layers else {}
        if overrides is None:
            print(f"check-drift --rev: rev `{args.rev}` 上取不到层①要的七份文件 "
                  f"⇒ 形状异常（exit 2）✗", file=sys.stderr)
            return 2
        with tempfile.TemporaryDirectory(prefix="soko-drift-rev-") as tmp:
            root = Path(tmp)
            if LAYER_LINK in layers:
                text = _git_show(args.rev, "scripts/soko")
                if text is None:
                    print(f"check-drift --rev: rev `{args.rev}` 上取不到 `scripts/soko` "
                          f"⇒ 形状异常（exit 2）✗", file=sys.stderr)
                    return 2
                launcher = root / "soko-rev.mjs"
                launcher.write_text(text, encoding="utf-8")
                if shutil.which("node") is None:
                    print("check-drift --rev: 找不到 node ⇒ 层②判不了（exit 3）✗", file=sys.stderr)
                    return 3
            else:
                launcher = LAUNCHER
            by_layer, fatal = _collect(layers, launcher, root, overrides)
        if fatal:
            print(f"check-drift --rev: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
            return 2
        _print(by_layer, args.json)
        return 0 if all(r["ok"] for rs in by_layer.values() for r in rs) else 1

    pre = _preconditions(layers)
    if pre is not None:
        return pre
    with tempfile.TemporaryDirectory(prefix="soko-drift-") as tmp:
        by_layer, fatal = _collect(layers, LAUNCHER, Path(tmp), None)
    if fatal:
        print(f"check-drift: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
        return 2
    _print(by_layer, args.json)
    return 0 if all(r["ok"] for rs in by_layer.values() for r in rs) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
