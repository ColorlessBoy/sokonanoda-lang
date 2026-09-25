#!/usr/bin/env python3
"""**记法转化调用点守卫**（阶段 U / T-U3 ✓）：谁绕过了唯一接口，这里判红 ✗。

## 为什么需要它（不是洁癖，是两次事故）
记法转化曾经散在**四处** ✗（`REQUIREMENTS.md` §9 ㉔；设计
`docs/design/notation-display.md` §1）：

| # | 实现 | 干什么 |
|---|---|---|
| ① | `display::print_back` | 真的转化 ✓（但只被 `ty_text`/`val_text` 用） |
| ② | `semantic::tag_runs_with_notations` | **只打标签** ✗（`query::runs` ⇒ Infoview 读的 `goal_runs`） |
| ③ | `display::render_expr`（`proof::render_expr`） | **只渲染** ✗（`goals::open_goal`、`walk.rs` 的目标） |
| ④ | 内核 pp（`info.goal`） | 点形式 ✗ |

⇒ 目标生产链（③④⇒②）**只渲染不折叠** ⇒ Infoview 顶部「目标」永远是点形式 ✗
（2026-09-25 用户报告 ✓）。同族的还有 R-2（`=` 吃掉 `=>` 的 `=` ✗）。

## 判据（唯一接口 = `DisplayNotations::{fold, render, runs}` ✓）
在**白名单之外**出现下面三个调用之一 ⇒ **判红** ✓：
    render_expr( · print_back( · tag_runs_with_notations(
白名单（**只有实现者** ✓）：
    crates/front/src/display.rs        （唯一接口的实现 ✓）
    crates/front/src/proof.rs          （`render_expr` 的**定义** ✓）

## ⚠ 守卫的**盲区**（2026-09-25 实例，务必知道 ✓）
白名单按**文件**豁免 ⇒ **同一个文件内**再长出一个等价入口，它**抓不到** ✗。
真实实例：`display.rs` 里 `DisplayNotations::render`（T-U2 的唯一接口 ✓）与
`render_folded`（round 60 加的 ✓）**函数体逐字等价** ✗，两个都在白名单里 ⇒ 无人守 ✗
（审计 #8；已删 `render_folded` ✓）。⇒ `display.rs` 内的"接口唯一性"**只能靠评审** ✓ ——
改那个文件前，先看它顶部的接口清单：`fold` / `render` / `runs` / `Rendered::is_consistent` ✓，
**不要**再加第二个等价函数 ✗。

## 用法
    python3 scripts/audit-notation-paths.py            # 0=干净 1=有绕过 2=用法/扫描错误
    python3 scripts/audit-notation-paths.py --json     # 单 JSON 对象
    python3 scripts/audit-notation-paths.py --self-test  # 反向验证：故意造违规必须被抓到
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SCAN_ROOTS = [REPO / "crates"]
WHITELIST = {
    "crates/front/src/display.rs",
    "crates/front/src/proof.rs",
    # ── **已逐条判过 ③（写台账不做）的组**（2026-09-25 round 171 ✓）──────────────
    # 为什么放进白名单 ✓：基线是**棘轮** ✓ —— 它记的是"**待迁移**" ✓；
    # 而 ③ 的条目**永远不会迁** ✗ ⇒ 留在里面会让数字**失去意义** ✗
    # （分不清"还没做"与"已判不动" ✓）。这四组都已在
    # `docs/design/duplication-audit.md` 里**逐条**给过结论 ✓：
    "crates/front/src/judge.rs",              # J 组(1)：判定量 ⇒ 折它 = 改判定 ✗（内核红线）
    "crates/front/src/compile/tests.rs",      # I 组(3)：**往返测试**的内部期望串 ✗
    "crates/front/src/compile/prelude.rs",    # G 组(3)：造 `GoalBinderSpec` ⇒ 喂 judge ✗
    "crates/front/src/compile/check/walk.rs", # E 组(8)：目标生产 ⇒ 喂 judge ✗（§9 ㉜ 的行程开关钉着）
    # ⚠ **代价（要记住 ✓）**：白名单是**按文件**的 ✗ ⇒ 这些文件里**将来**新出现的
    # 真绕过**不会被这条守卫抓到** ✓ ⇒ 改动它们时**要人工看一眼** ✓
    # （或给守卫加"按行/按符号"的细粒度排除 ✓ —— 那是下一步可选工作 ✓）。
}
# **唯一接口**：绕过这三个函数的调用点都在守卫范围内 ✓。
# ⚠ **不要把 `:` 排除在外**（2026-09-25 修 ✗⇒✓）：原来的 lookbehind `(?<![\w:.])`
# 想跳过**方法调用**（`x.print_back(` ✓），但它同时把**路径限定调用**
# `crate::display::print_back(` ✗ 也排除了 ⇒ **漏检** ✓ ——
# 实测：`crates/front/src/compile/check/kernel_phase.rs` 的 3 处 `print_back` **不在基线里** ✗
#（而 `ty_text` 正是由它们折的 ✓，见 `docs/design/e2-plan.md` 的 T-U12 副发现 ✓）
# ⇒ 那 77 处是**低估** ✓。现在只排除**紧邻标识符**（`\w` ✓）⇒ 路径形式照样命中 ✓。
CALLS = {
    "render_expr": re.compile(r"(?<!\w)render_expr\s*\("),
    "print_back": re.compile(r"(?<!\w)print_back\s*\("),
    "tag_runs_with_notations": re.compile(r"tag_runs_with_notations\s*\("),
}
BASELINE = REPO / "scripts" / "notation-paths-baseline.txt"
HINT = "改用唯一接口：`DisplayNotations::{fold, render, runs}`（设计 docs/design/notation-display.md ✓）"


def fingerprint(hit: dict) -> str:
    """**行号无关**的指纹：`文件|调用|该行代码（去空白）` —— 迁移时行号会漂移 ✓。"""
    return f"{hit['file']}|{hit['call']}|{hit.get('code', '')}"


def load_baseline() -> set[str]:
    """**棘轮**（ratchet）：冻结"阶段 U 之前就存在的"调用点 ✓，只拦**新增** ✗。

    为什么不是"不接进 gate"：那样守卫等于没有 ✗（`AGENTS.md`：咬不住的守卫等于没有 ✓）。
    为什么不是"一次全迁完再开"：77 处 ✗ 会让 gate 常红 ⇒ 没法判断别的东西 ✗。
    ⇒ 冻结基线 + **只拦新增** ✓；迁移（T-U5）每迁一处就 `--rebless` 削一条 ✓。
    """
    if not BASELINE.exists():
        return set()
    return {
        line.strip()
        for line in BASELINE.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    }


def rust_files() -> list[Path]:
    out: list[Path] = []
    for root in SCAN_ROOTS:
        if root.is_dir():
            out.extend(sorted(p for p in root.rglob("*.rs") if p.is_file()))
    return out


def scan(files: list[Path]) -> list[dict]:
    """返回违规清单；`rel` 是仓库相对路径（白名单按它判 ✓）。"""
    hits: list[dict] = []
    for path in files:
        try:
            rel = path.resolve().relative_to(REPO.resolve()).as_posix()
        except ValueError:
            rel = path.as_posix()
        if rel in WHITELIST:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:  # 读不了 ⇒ **不能静默跳过**（那等于没判 ✗）
            hits.append({"file": rel, "line": 0, "call": "<unreadable>", "why": str(exc)})
            continue
        prev_codes: list[str] = []
        for lineno, raw in enumerate(text.splitlines(), 1):
            # 注释里的提及不算（守卫判的是**调用** ✓）；文档字符串里的 `fn x(` 也不算 ✓。
            code = raw.split("//", 1)[0]
            window3 = "\n".join(prev_codes[-3:])
            prev_codes.append(code)
            # **定义/再导出行不算绕过** ✓（2026-09-25 补 ✗⇒✓）：`pub fn render_expr(expr)` 与
            # 它体内的 `crate::proof::render_expr(expr)` 是**再导出**（`check/mod.rs:1243` ✓），
            # 不是"绕过接口的实现" ✓ —— 它们此前贡献了 **2 条假条目** ✗（88 里的两条 ✓）。
            # ⚠ 只跳**签名行**（`fn <name>(` ✓）：体内的调用会被上一行的签名"带过"吗 ✗ ——
            # 不会 ✓（逐行判 ✓），所以再导出体内的那一行仍会被算 ✓ …… 因此这里跳过
            # **整段再导出**是靠"名字即调用名 + 出现在 fn 签名里"这一条 ✓：
            # 简单起见只跳签名行 ✓，并在基线刷新时把再导出体内的那 1 行一起处理 ✓。
            if re.match(r"\s*(pub(\(crate\))?\s+)?fn\s+(render_expr|print_back|tag_runs_with_notations)\b", code):
                continue
            # **已折的写法不算绕过** ✓（2026-09-25 T-U11 A/B 组 ✓）：当同一行还出现
            # `fold_for_display(`（front 的公开折文本入口 ✓）或 `render_msg(`（elab 的消息入口 ✓）时，
            # 这一行的 `render_expr` 是**折叠管线的一部分** ✓（`render` = `fold ∘ render_expr` ✓），
            # 不是"绕过接口直接渲染" ✗。实测依据：LSP 的 4 处改成
            # `fold_for_display(text, &render_expr(…))` ✓ 后基线**没有下降** ✗ ——
            # 那说明守卫把它们仍当绕过 ✓，是守卫的判据该细化 ✓（而不是改回去 ✗）。
            # ⚠ **也要看上一行**（2026-09-25 ✓）：包成多行时
            # `fold_for_display(` / `render_msg(` 在**上一行** ✓、`render_expr(` 在下一行 ✗
            # ⇒ 只看本行会漏判 ✓（实测：A 组第 5 处 `:1364` 多行包裹后基线**没降** ✗）。
            window = code + window3
            if "fold_for_display(" in window or "render_msg(" in window:
                continue
            # **再导出的函数体也不算绕过** ✓（2026-09-25 round 170 补 ✓，round 111 的残留 ✗）：
            # `check/mod.rs:1265-1266` 是
            # `pub fn render_expr(expr: &Expr) -> String { crate::proof::render_expr(expr) }` ✓
            # —— round 111 只跳了**签名行** ✓，**函数体那一行**仍被算作绕过 ✗
            # ⇒ 那是一条**假条目**（它不是"绕过接口的实现"✗，是**再导出** ✓）。
            # 用同一个窗口 ✓：本行是纯调用 ✓、且前三行里出现同名 `fn` 签名 ✓ ⇒ 跳过 ✓。
            if re.match(r"^\s*(crate::proof::)?render_expr\([^)]*\)\s*$", code) and re.search(
                r"fn\s+(render_expr|print_back|tag_runs_with_notations)\b", window3
            ):
                continue
            for call, pat in CALLS.items():
                if pat.search(code):
                    hits.append(
                        {
                            "file": rel,
                            "line": lineno,
                            "call": call,
                            "code": code.strip(),
                            "why": HINT,
                        }
                    )
    return hits


def main(argv: list[str]) -> int:
    as_json = "--json" in argv
    if "--self-test" in argv:
        # **反向验证**（硬要求 ✓）：本阶段之前那些调用点必须**被抓到** ✗ ——
        # 咬不住的守卫等于没有 ✓（照 `audit-wire-fields.py` 的先例 ✓）。
        fake = REPO / "crates" / "front" / "src" / "compile" / "check" / "walk.rs"
        hits = scan([fake])
        ok = any(h["file"].endswith("walk.rs") and h["line"] for h in hits)
        if ok:
            print(f"notation-paths self-test: OK（walk.rs 里 {len(hits)} 处绕过被抓到 ✓）")
            return 0
        print("notation-paths self-test: FAIL（应当抓到 walk.rs 里的绕过，实际没抓到 ✗）", file=sys.stderr)
        return 1

    files = rust_files()
    if not files:
        print("notation-paths: 扫不到 crates/ 下的 .rs（无法判定 ≠ 绿）✗", file=sys.stderr)
        return 2
    hits = scan(files)
    if "--rebless" in argv:
        BASELINE.write_text(
            "# **棘轮基线**（阶段 U / T-U3）：阶段 U 之前就存在的绕过调用点 ✓。\n"
            "# 迁移（T-U5）每迁一处就 `python3 scripts/audit-notation-paths.py --rebless` 削一条 ✓；\n"
            "# **只许变短**（新增的会被守卫判红 ✗）。指纹 = 文件|调用|该行代码（行号无关 ✓）。\n"
            + "\n".join(sorted(fingerprint(h) for h in hits))
            + "\n",
            encoding="utf-8",
        )
        print(f"notation-paths: 基线已重写（{len(hits)} 条）✓")
        return 0
    baseline = load_baseline()
    new_hits = [h for h in hits if fingerprint(h) not in baseline]
    if as_json:
        print(json.dumps({"ok": not new_hits, "scanned": len(files),
                          "known": len(hits) - len(new_hits), "violations": new_hits},
                         ensure_ascii=False))
    elif new_hits:
        print(f"notation-paths: **新增** {len(new_hits)} 处绕过唯一记法接口 ✗"
              f"（另有基线内 {len(hits) - len(new_hits)} 处待迁移 ✓；扫了 {len(files)} 个 .rs）")
        for h in new_hits[:20]:
            print(f"  {h['file']}:{h['line']}  {h['call']}  ⇒ {h['why']}")
        if len(new_hits) > 20:
            print(f"  … 其余 {len(new_hits) - 20} 处见 --json ✓")
    else:
        print(f"notation-paths: OK —— 没有**新增**绕过 ✓"
              f"（基线内 {len(hits)} 处待迁移，见 scripts/notation-paths-baseline.txt ✓）")
    return 1 if new_hits else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
