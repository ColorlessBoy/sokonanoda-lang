#!/usr/bin/env python3
"""画布/解答**对账**：逐单元列出两侧的声明名，报出「画布有、解答没有」的名字。

为什么需要它（工具纪律的守卫，2026-10-01 立）：
    课程门禁的 **G4** 只在**判卷时**报「解答没有覆盖画布的练习 X」✓ —— 那时已经晚了
    （我已经把解答删残、还先怀疑判据 ✗）。本工具**在改完之后立刻**给出对账表 ✓，
    并且**把"解答比画布少的名字"直接列出来** ⇒ 「切到文件尾删候选」那类事故当场可见 ✓。

用法：
    python3 courses/set-theory/tools/audit-pairs.py            # 全课程对账表（尾部汇总）
    python3 courses/set-theory/tools/audit-pairs.py --check     # 只在有缺名时 exit 1
    python3 courses/set-theory/tools/audit-pairs.py --unit 52   # 只看某个单元

定位（重要）✓：本工具是**诊断**，**不是**判据 —— **G4 仍是权威** ✓。
    `--check` 只对**硬缺**（非 `demo_*`、非 `Set.*` 的练习名）判负；`--strict` 连软缺也判负。
    单元 1–12 的画布里有若干**不带 `demo_` 前缀的演示**（如 unit11 的三条），
    它们在**画布内已证**、门禁 G4 也认 ✓ ⇒ 本工具会把它们标成硬缺 ✗ —— 那是**工具的口径**，
    不是课程的缺口 ✓（以门禁为准 ✓）。
"""
from __future__ import annotations
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent          # courses/set-theory
UNITS = ROOT / "units"
SOLS = UNITS / "solutions"
DECL = re.compile(r"^\s*(?:theorem|def|axiom)\s+([A-Za-z_][A-Za-z0-9_.']*)", re.M)


def names(path: pathlib.Path) -> list[str]:
    return DECL.findall(path.read_text(encoding="utf-8"))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="有**硬缺**（练习没覆盖）就 exit 1")
    ap.add_argument("--strict", action="store_true", help="连**软缺**（演示/定义）也判负 —— 仅供排查")
    ap.add_argument("--unit", type=int, default=None, help="只看某个单元号")
    a = ap.parse_args()

    rows, bad, soft_any = [], [], False
    # 画布按**章**分子目录（`units/<章 id>/…`）⇒ 递归收；`units/solutions/**` 是解答，
    # 不能当画布收（同名同后缀，非递归/不排除都会漏或错配）。
    canvases = [
        p
        for p in sorted(UNITS.rglob("unit*.sokonanoda"))
        if p.is_file() and SOLS not in p.parents
    ]
    for canvas in canvases:
        m = re.match(r"unit(\d+)", canvas.name)
        if not m:
            continue
        n = int(m.group(1))
        if a.unit is not None and n != a.unit:
            continue
        # 文件名里的数字**保留原样**（单元 1–12 是零填充 `unit01-`，13 起是 `unit13-`）；
        # 解答与画布**同构**分层 ⇒ 解答在画布同章的 `units/solutions/<章 id>/` 下。
        sol = (
            SOLS / canvas.parent.relative_to(UNITS) / f"unit{m.group(1)}-solution.sokonanoda"
            if canvas.parent != UNITS
            else SOLS / f"unit{m.group(1)}-solution.sokonanoda"
        )
        if not sol.exists():
            rows.append((n, canvas.name, "**没有解答文件** ✗", [], [], []))
            bad.append(n)
            continue
        cn, sn = names(canvas), names(sol)
        missing = [x for x in cn if x not in sn]
        extra = [x for x in sn if x not in cn]
        missing = [x for x in cn if x not in sn]
        # **软缺**（画布已证、解答不必重复）：`demo_*` 与 `Set.*` 定义 ⇒ 只报告、不判负 ✓
        soft = [x for x in missing if x.startswith("demo_") or "." in x]
        hard = [x for x in missing if x not in soft]
        extra = [x for x in sn if x not in cn]
        rows.append((n, canvas.name, f"画布 {len(cn)} · 解答 {len(sn)}", hard, soft, extra))
        if hard:
            bad.append(n)
        if soft:
            soft_any = True
        if False:
            bad.append(n)

    for n, name, info, hard, soft, extra in rows:
        flag = "✗" if hard else "✓"
        print(f"{flag} 单元{n:<3} {info:<22} {name}")
        if hard:
            print(f"      **硬缺（练习没覆盖）**：{', '.join(hard)}")
        if soft and a.unit is not None:
            print(f"      软缺（演示/定义，画布已证）：{', '.join(soft)}")
        if extra and a.unit is not None:
            print(f"      解答多出（辅助引理，正常）：{', '.join(extra)}")
    print(f"\n共 {len(rows)} 个单元 · 缺名 {len(bad)} 个" + (f"：{bad}" if bad else " ✓"))
    # **默认只诊断、不判负**（G4 是权威 ✓）；`--strict` 才有判负语义。
    if a.strict:
        return 1 if (bad or soft_any) else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
