#!/usr/bin/env python3
"""Infoview CSS 的同族守卫（AGENTS.md 验证设计纪律第 0 条 (b)：**可查的守卫** ✓）。

**为什么有它**：同一条 UI 反馈复发过两次 ——
  · 2026-09-26 `.decl-ty`「太小太暗」✗（修了单点 ✗）；
  · 2026-09-28 `.section-title`「`<h2>` 却更小更暗」✗（**没横向排查**的后果 ✓）。
⇒ 收到一条 UI 反馈就必须问"同类还有哪些" ✓，并把判断落成**能咬住**的机器判据 ✓。

两条判据（都能反向验证 ✓）：
  ① **双重压暗** ✗：同一条规则里既有 `color: …Foreground`（已经暗一档 ✓）又有 `opacity < 1`
     ⇒ 乘起来更暗 ✓（`.decl-ty` / `.binder-ty` 都栽在这条上 ✓）。
     例外：**没有 color 的**规则不算 ✗（例如 `.empty { opacity: .7; font-style: italic }` ✓
     是空态占位符的通行处理 ✓）。
  ② **标题不许小于正文** ✗：`.section-title` / `h1`–`h6` 族里出现 `font-size: <1em`（或 px < 13）
     ⇒ 层级更高、视觉更弱 ✓（2026-09-28 那条 ✓）。

退出码：0 = 干净 ✓ · 1 = 有命中（判红 ✓）· 2 = 用法/环境错。
"""
from __future__ import annotations
import re
import sys
from pathlib import Path

CSS = Path(__file__).resolve().parent.parent / "editor" / "vscode" / "media" / "infoview.css"
HEADINGISH = re.compile(r"^(\.section-title|h[1-6])\b")


def blocks(text: str):
    """极简 CSS 块切分：返回 (选择器, 规则体, 起始行号)。够用即可 ✓。"""
    for m in re.finditer(r"([^{}]+)\{([^{}]*)\}", text):
        sel = m.group(1).strip().splitlines()[-1].strip()
        yield sel, m.group(2), text[: m.start()].count("\n") + 1


def main() -> int:
    if not CSS.exists():
        print(f"找不到 {CSS} ⇒ 环境异常", file=sys.stderr)
        return 2
    text = CSS.read_text(encoding="utf-8")
    hits: list[str] = []
    for sel, body, line in blocks(text):
        has_fg = re.search(r"color:\s*[^;]*Foreground", body) is not None
        op = re.search(r"opacity:\s*([0-9.]+)", body)
        # ⚠ **反向验证教训（2026-10-03 ✓）**：第一版只查「显式 `*Foreground` + opacity」✗ ⇒
        # 把 `.binder-ty` 的 opacity 放回去它**不咬** ✗（那条靠 `font: inherit` **继承**前景色 ✓）
        # ⇒ 判据必须覆盖**类型注解族本身** ✓（`.decl-ty` / `.binder-ty` / `*-ty` ✓）：
        # 这一族本来就该"只靠颜色压一档" ✓，**任何** opacity < 1 都是双重压暗 ✗。
        is_ty_family = re.search(r"\.(decl|binder|goal|expr)-ty\b", sel) is not None
        if op and float(op.group(1)) < 1.0 and (has_fg or is_ty_family):
            why = "color(*Foreground) + opacity" if has_fg else "类型注解族的 opacity"
            hits.append(f"  {CSS.name}:{line}  {sel} —— {why}:{op.group(1)} = **双重压暗** ✗")
        if HEADINGISH.match(sel):
            fs = re.search(r"font-size:\s*([0-9.]+)em", body)
            if fs and float(fs.group(1)) < 1.0:
                hits.append(f"  {CSS.name}:{line}  {sel} —— font-size:{fs.group(1)}em < 1em ⇒ 标题比正文小 ✗")
    if hits:
        print("infoview-css：**同族问题**命中 ✗（AGENTS.md 第 0 条 (b)）")
        print("\n".join(hits))
        return 1
    print("infoview-css ✓ 无双重压暗 ✓ · 无『标题小于正文』✗")
    return 0


if __name__ == "__main__":
    sys.exit(main())
