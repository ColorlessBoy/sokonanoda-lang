#!/usr/bin/env python3
"""**Infoview 视觉层级守卫**（2026-09-28，G-66 / AGENTS.md 验证设计纪律第 0 条 (b)）。

用户 18:24 实测（**一个月内第二次同形反馈**）：侧边栏分区标题「目标 / 声明 / 项目」
**比正文还小**，而它们是 `<h2>` ⇒ **层级更高、视觉却更弱，方向反了** ✗。
`media/infoview.css` 的 `.section-title` = `font-size: 0.8em` + `opacity: 0.7`
⇒ 比正文**小 20%、暗 30%** ✓。

**⇒ 本脚本把三条原则变成可判红的判据**（只写注释不算 —— E18 已证明文字纪律几轮就失效）：

- **① 标题字号 ≥ 正文字号**：任何标题类选择器（`.section-title` / `h1`–`h6` 等）
  的 `font-size` **不许小于正文** ✗ —— `em` 基准 = `1em`，`px` 基准 = **13px**
  （`--vscode-font-size` 的默认值）；两种单位都实测，`calc(...)`/变量不猜 ✓。
- **② 不许双重压暗**：**同一个规则里**不许同时出现「小于正文的 `font-size`」**和**
  `opacity < 1` ✗ —— 前景色本来就暗一档（`descriptionForeground`），再乘 opacity
  就是**双重压暗**（2026-09-26 修 `.decl-ty` 定下的原则 ✓；当时**没横向排查** ⇒
  同一个坑留到今天 ✗）。
- **③ 标题不许用 `opacity` 降档**（2026-09-28 用户要求「降一档**只用颜色**不用 opacity」✓）：
  标题类选择器里出现 `opacity < 1` ⇒ 判红 ✗，**与字号无关** —— 2026-10-07 收口审计
  实测：原先只有判据 ② ⇒ `font-size: 1em; opacity: 0.7` **溜过去了** ✗（用户要求的
  那一半没人守 ⇒ 补上 ✓）；`px` 形态（`font-size: 12px`）当时两条守卫都**不咬** ✗
  ⇒ 判据 ① 同时按 em/px 实测 ✓。

⚠ **为什么不是"一律禁止 opacity"** ✗：`opacity` 本身没错 —— `.empty`（空态提示）
**单次降档**（它没有小于正文的 `font-size`，继承正文字号）⇒ **允许** ✓。
判据抓的是**叠加**与**标题用 opacity 降档**，不是 opacity 本身 ✓。
（`.binder-ty` 曾属"允许"那类；2026-10-03 同族横向排查后它的 opacity 已删 ⇒
现在 `editor/vscode/media/*.css` 里只剩 `.empty` 一处 opacity ✓。）

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
BASELINE_PX = 13.0


def rules(css: str) -> list[tuple[str, str]]:
    """`(selector, body)` 列表（已去掉注释与空白噪声）。"""
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    return [(" ".join(sel.split()), body) for sel, body in re.findall(r"([^{}]+)\{([^}]*)\}", css)]


def size_of(body: str) -> tuple[float, str] | None:
    """规则里的 `font-size` 折成 `(值, 单位)`；不是纯 `Nem`/`Npx`（如 `calc(...)`）⇒ None。"""
    m = re.search(r"font-size:\s*([\d.]+)(em|px)\b", body)
    return (float(m.group(1)), m.group(2)) if m else None


def fmt(size: tuple[float, str] | None) -> str:
    return f"{size[0]:g}{size[1]}" if size else "inherit"


def smaller_than_body(size: tuple[float, str]) -> bool:
    """`em` 比 `1em` 小、`px` 比 `13px` 小 ⇒ 比正文小 ✗（两种单位都实测 ✓）。"""
    value, unit = size
    return value < BASELINE_EM if unit == "em" else value < BASELINE_PX


def dimming_opacity(body: str) -> float | None:
    """`opacity < 1` 的实测值；没声明 / `opacity: 1` ⇒ None（`;` 不是必需的 ✓）。"""
    m = re.search(r"(?<![-a-z])opacity:\s*([\d.]+)", body)
    return float(m.group(1)) if m and float(m.group(1)) < 1.0 else None


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
        size = size_of(body)
        dim = dimming_opacity(body)
        is_title = any(t in sel for t in TITLE_SELECTORS)

        # ① 标题字号 ≥ 正文
        if is_title and size is not None and smaller_than_body(size):
            bad.append(
                f"① 标题类选择器 `{sel}` 的 font-size = **{fmt(size)} < 正文**"
                f"（{BASELINE_EM:g}em / {BASELINE_PX:g}px）✗ "
                f"—— 标题层级更高、字号却更小 ⇒ **方向反了**"
            )

        # ② 同一规则里「小于正文」+ opacity ⇒ 双重压暗
        if size is not None and smaller_than_body(size) and dim is not None:
            bad.append(
                f"② `{sel}`：font-size **{fmt(size)}**（小于正文）**又叠了 `opacity: {dim:g}`** ✗ "
                f"—— 前景色本来就暗一档，再乘 opacity 是**双重压暗** ⇒ "
                f"降一档**只用颜色**（如 `color: var(--vscode-descriptionForeground)`）"
            )

        # ③ 标题不许用 opacity 降档（**与字号无关** ⇒ 1em + opacity 也判红 ✓）
        if is_title and dim is not None:
            bad.append(
                f"③ 标题类选择器 `{sel}` 用 `opacity: {dim:g}` 降档 ✗ —— "
                f"标题降一档**只用颜色**（`color: var(--vscode-descriptionForeground)`）✓"
            )

    if bad:
        print(f"infoview-hierarchy：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad:
            print(f"  ✗ {b}", file=sys.stderr)
        print(
            "\n依据：`AGENTS.md` 验证设计纪律**第 0 条 (b)**（同类问题横向排查，不许修单点）"
            "与 2026-09-28 用户 18:24 实测（G-66）✓。\n"
            "修法：标题字号回 **正文**（`1em` / `13px`）、降一档**只用颜色**（不许 opacity）；"
            "小于正文的辅助信息**去掉 opacity** ✓。",
            file=sys.stderr,
        )
        return 1
    print(f"infoview-hierarchy: ✓ {len(rs)} 条规则 —— 标题字号 ≥ 正文、无双重压暗、标题不用 opacity")
    return 0


def report(path: Path = CSS) -> int:
    """逐选择器打印字号/透明度（**判据的实测依据** ✓）。"""
    rs = rules(path.read_text(encoding="utf-8"))
    print("%-30s %-8s %-8s %s" % ("selector", "size", "opacity", "判定"))
    for sel, body in rs:
        size = size_of(body)
        dim = dimming_opacity(body)
        is_title = any(t in sel for t in TITLE_SELECTORS)
        if size is None and dim is None:
            continue
        if is_title and dim is not None:
            verdict = "★ 标题用 opacity 降档 ⇒ 判红"
        elif is_title and size is not None and smaller_than_body(size):
            verdict = "★ 标题小于正文 ⇒ 判红"
        elif size is not None and smaller_than_body(size) and dim is not None:
            verdict = "★ 双重压暗 ⇒ 判红"
        elif size is not None and smaller_than_body(size):
            verdict = f"小字辅助信息（{fmt(size)}，无 opacity）✓"
        elif dim is not None:
            verdict = "单次降档（字号继承正文）✓"
        else:
            verdict = "—"
        print("%-30s %-8s %-8s %s" % (sel[:29], fmt(size), f"{dim:g}" if dim is not None else "no", verdict))
    return 0


def selftest() -> int:
    """**反向验证**：故意坏的 CSS 必须被判红（咬不住的守卫等于没有 ✓）。"""
    import tempfile

    cases = [
        # (CSS 片段, 期望判红?)
        (".section-title { font-size: 0.8em; font-weight: 600; }", True),
        (".section-title { font-size: 0.8em; opacity: 0.7; }", True),
        (".decl-kind { font-size: 0.8em; opacity: 0.7; }", True),
        (".decl-val-label { font-size: 0.78em; opacity: 0.55; }", True),
        (".section-title { font-size: 1em; font-weight: 600; }", False),
        (".decl-kind { font-size: 0.8em; color: var(--vscode-descriptionForeground); }", False),
        (".empty { opacity: 0.7; font-style: italic; }", False),
        (".decl-ty { font-size: calc(1em * var(--soko-font-scale, 1)); opacity: 0.9; }", False),
        ("h2 { font-size: 0.9em; }", True),
        # 2026-10-07 收口审计补的洞（当时这四条**都不咬** ✗ —— 现在必须判红 ✓）：
        (".section-title { font-size: 1em; opacity: 0.7; }", True),  # ③ 字号对了、仍用 opacity 降档
        (".section-title { font-size: 0.8em; opacity: 0.7 }", True),  # ② 无尾分号也要咬住
        (".section-title { font-size: 12px; }", True),  # ① px 形态（< 13px 正文）
        ("h2 { font-size: 1.1em; opacity: 0.9; }", True),  # ③ 字号更大也不许用 opacity
        (".section-title { font-size: 13px; }", False),  # ① px = 正文，不算小 ✓
        (".empty { opacity: 0.7 }", False),  # 非标题、无字号 ⇒ 单次降档（无尾分号）✓
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
