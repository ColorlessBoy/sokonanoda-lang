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
        for lineno, raw in enumerate(text.splitlines(), 1):
            # 注释里的提及不算（守卫判的是**调用** ✓）；文档字符串里的 `fn x(` 也不算 ✓。
            code = raw.split("//", 1)[0]
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
