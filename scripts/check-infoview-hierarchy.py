#!/usr/bin/env python3
"""**Infoview 视觉层级守卫**（2026-09-28，G-66 / AGENTS.md 验证设计纪律第 0 条 (b)）。

用户 18:24 实测（**一个月内第二次同形反馈**）：侧边栏分区标题「目标 / 声明 / 项目」
**比正文还小**，而它们是 `<h2>` ⇒ **层级更高、视觉却更弱，方向反了** ✗。
`media/infoview.css` 的 `.section-title` = `font-size: 0.8em` + `opacity: 0.7`
⇒ 比正文**小 20%、暗 30%** ✓。

**⇒ 本脚本把两条原则变成可判红的判据**（只写注释不算 —— E18 已证明文字纪律几轮就失效）：

- **① 标题字号 ≥ 正文字号**：任何标题类选择器（`.section-title` / `h1`–`h3` 等）
  的 `font-size` **不许小于 `1em`** ✗（正文 = `body` 的 `--vscode-font-size` = 1em）。
- **② 不许双重压暗**：**同一个规则里**不许同时出现 `font-size: <1em` **和** `opacity`
  ✗ —— 前景色本来就暗一档（`descriptionForeground`），再乘 opacity 就是**双重压暗**
  （2026-09-26 修 `.decl-ty` 定下的原则 ✓；当时**没横向排查** ⇒ 同一个坑留到今天 ✗）。

⚠ **为什么不是"一律禁止 opacity"** ✗：`opacity` 本身没错 ——
`.empty`（空态提示）与 `.binder-ty`（绑定变量类型）都是**单次降档**（它们没有 `<1em`
的 `font-size`，继承正文字号）⇒ **允许** ✓。判据抓的是**叠加**，不是 opacity 本身 ✓。

用法：

    python3 scripts/check-infoview-hierarchy.py            # 判当前 CSS
    python3 scripts/check-infoview-hierarchy.py --selftest # 自检：反例必须判红
    python3 scripts/check-infoview-hierarchy.py --report   # 打印逐选择器的字号/透明度表

退出码：0 = 通过 · 1 = 判红 · 2 = 环境错（CSS 文件找不到）。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

CSS = Path("editor/vscode/media/infoview.css")

# 标题类选择器：**字号不许小于正文** ✓
TITLE_SELECTORS = (".section-title", "h1", "h2", "h3", "h4", "h5", "h6")

# 正文基准：`body` 用 `--vscode-font-size`（默认 13px）= 1em ✓
BASELINE_EM = 1.0


def rules(css: str) -> list[tuple[str, str]]:
    """`(selector, body)` 列表（已去掉注释与空白噪声）。"""
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    return [(" ".join(sel.split()), body) for sel, body in re.findall(r"([^{}]+)\{([^}]*)\}", css)]


def em_value(body: str) -> float | None:
    """规则里的 `font-size` 折成 em；不是纯 `Nem` 形式（如 `calc(...)` / `px`）⇒ None。"""
    m = re.search(r"font-size:\s*([\d.]+)em\s*;", body)
    return float(m.group(1)) if m else None


def has_opacity(body: str) -> bool:
    return re.search(r"(?<![-a-z])opacity:\s*[\d.]+\s*;", body) is not None


def check(path: Path = CSS) -> int:
    if not path.exists():
        print(f"infoview-hierarchy: 找不到 {path} ⇒ 环境错 ✗", file=sys.stderr)
        return 2
    rs = rules(path.read_text(encoding="utf-8"))
    if not rs:
        print(f"infoview-hierarchy: {path} 里没解析出任何规则 ⇒ **没扫到 ≠ 绿** ✗", file=sys.stderr)
        return 2

    bad: list[str] = []
    for sel, body in rs:
        em = em_value(body)

        # ① 标题字号 ≥ 正文
        if any(t in sel for t in TITLE_SELECTORS) and em is not None and em < BASELINE_EM:
            bad.append(
                f"① 标题类选择器 `{sel}` 的 font-size = **{em}em < {BASELINE_EM}em**（正文）✗ "
                f"—— 标题层级更高、字号却更小 ⇒ **方向反了**"
            )

        # ② 同一规则里 `<1em` + opacity ⇒ 双重压暗
        if em is not None and em < BASELINE_EM and has_opacity(body):
            bad.append(
                f"② `{sel}`：font-size **{em}em**（< 1em）**又叠了 `opacity`** ✗ "
                f"—— 前景色本来就暗一档，再乘 opacity 是**双重压暗** ⇒ "
                f"降一档**只用颜色**（如 `color: var(--vscode-descriptionForeground)`）"
            )

    if bad:
        print(f"infoview-hierarchy：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad:
            print(f"  ✗ {b}", file=sys.stderr)
        print(
            "\n依据：`AGENTS.md` 验证设计纪律**第 0 条 (b)**（同类问题横向排查，不许修单点）"
            "与 2026-09-28 用户 18:24 实测（G-66）✓。\n"
            "修法：标题字号回 **1em**；`<1em` 的辅助信息**去掉 opacity**、降档只用颜色 ✓。",
            file=sys.stderr,
        )
        return 1
    print(f"infoview-hierarchy: ✓ {len(rs)} 条规则 —— 标题字号 ≥ 正文、无双重压暗")
    return 0


def report(path: Path = CSS) -> int:
    """逐选择器打印字号/透明度（**判据的实测依据** ✓）。"""
    rs = rules(path.read_text(encoding="utf-8"))
    print("%-30s %-8s %-8s %s" % ("selector", "size", "opacity", "判定"))
    for sel, body in rs:
        em = em_value(body)
        op = has_opacity(body)
        if em is None and not op:
            continue
        if em is not None and em < BASELINE_EM and op:
            verdict = "★ 双重压暗 ⇒ 判红"
        elif any(t in sel for t in TITLE_SELECTORS) and em is not None and em < BASELINE_EM:
            verdict = "★ 标题小于正文 ⇒ 判红"
        elif em is not None and em < BASELINE_EM:
            verdict = f"小字辅助信息（{em}em，无 opacity）✓"
        elif op:
            verdict = "单次降档（字号继承正文）✓"
        else:
            verdict = "—"
        print("%-30s %-8s %-8s %s" % (sel[:29], f"{em}em" if em else "inherit", "yes" if op else "no", verdict))
    return 0


def selftest() -> int:
    """**反向验证**：故意坏的 CSS 必须被判红（咬不住的守卫等于没有 ✓）。"""
    import tempfile

    cases = [
        # (CSS 片段, 期望判红?)
        (".section-title { font-size: 0.8em; font-weight: 600; }", True),
        (".section-title { font-size: 0.8em; opacity: 0.7; }", True),
        (".decl-kind { font-size: 0.8em; opacity: 0.7; }", True),
        (".decl-line-hint { font-size: 0.72em; opacity: 0.55; }", True),
        (".section-title { font-size: 1em; font-weight: 600; }", False),
        (".decl-kind { font-size: 0.8em; color: var(--vscode-descriptionForeground); }", False),
        (".empty { opacity: 0.7; font-style: italic; }", False),
        (".decl-ty { font-size: calc(1em * var(--soko-font-scale, 1)); opacity: 0.9; }", False),
        ("h2 { font-size: 0.9em; }", True),
    ]
    fails = 0
    with tempfile.TemporaryDirectory() as td:
        for css, want_bad in cases:
            f = Path(td) / "t.css"
            f.write_text(css, encoding="utf-8")
            got_bad = check(f) == 1
            if got_bad != want_bad:
                fails += 1
                print(f"  ✗ selftest 反例失败：{css[:52]!r} ⇒ 判红={got_bad} 期望 {want_bad}", file=sys.stderr)
    if fails:
        print(f"infoview-hierarchy --selftest：{fails} 个反例不达预期 ✗", file=sys.stderr)
        return 1
    n_bad = sum(1 for _, w in cases if w)
    print(f"infoview-hierarchy --selftest：✓ {len(cases)} 个反例全部符合预期（含 {n_bad} 个「必须判红」）")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    if "--report" in argv:
        return report()
    if any(a.startswith("-") for a in argv):
        print(__doc__.split("用法：")[1].strip(), file=sys.stderr)
        return 2
    return check()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
