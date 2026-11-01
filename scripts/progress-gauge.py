#!/usr/bin/env python3
"""**进度尺的唯一写入者**（2026-11-01 用户：「文档里的总进度怎么还没变」）。

## 为什么有这个脚本（真实事故）

2026-11-01 那天，主会话在对话里报"79% → 82%"，而 `reports/2026-11-01-progress.md` 里的进度条**一直停在 64%** ——
因为它用一串 `str.replace(旧, 新)` 去改那几行，**旧字符串没命中就静默跳过**（同一类病：守卫咬不住 = 没有）。
用户点名之后改成："**表是唯一真源，数字由脚本算**"。

## 真源在哪

**`reports/2026-10-29-strict-replan.md` §4.0 的台账表**（`| 项 | 内容 | 单位 | 状态 | 证据 |`）：
第三列是单位数（`1.5` 这种小数），第四列是状态 —— `✅` 记 done；`⬜ ⏸ ⚠ ✗` 都记 todo
（`✗` = 已决定不做，但它仍在表里留痕；单位填 `—` 的行不参与计算）。

## 用法

    python3 scripts/progress-gauge.py            # 重算并**写回**（进度条 + "done/total ⇒ pct%" 那一行）
    python3 scripts/progress-gauge.py --check    # 只查：对不上就 exit 1（提交前跑）
    python3 scripts/progress-gauge.py --json     # 给 CI/别的工具读

退出码：0 = 一致（或写成功）· 1 = `--check` 发现不一致 · 2 = 用法/解析错误。
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
LEDGER = REPO / "reports" / "2026-10-29-strict-replan.md"
DOC = REPO / "reports" / "2026-11-01-progress.md"
BLOCKS = 20

ROW = re.compile(r"^\|\s*\**`?([^|`*]+?)`?\**\s*\|(.+?)\|\s*([0-9]+(?:\.[0-9]+)?|—)\s*\|\s*(✅|⬜|⏸|⚠|✗)\s*\|")
BAR_LINE = re.compile(r"^距上线\s+\[[█░]+\]\s+[0-9]+%\s+已完成\s+[0-9.]+ / [0-9.]+ 单位\s*$", re.M)
# 合计行：**在视图里可选**（真源是台账）；有就必须一致，没有不算错。
TOTAL_LINE = re.compile(r"^\*\*done = [0-9.]+ · total = [0-9.]+ ⇒ [0-9]+%\*\*$", re.M)


def parse(text: str) -> tuple[list[tuple[str, str, float, bool]], str]:
    rows = []
    for line in text.splitlines():
        m = ROW.match(line.strip())
        if m and m.group(3) != "—":
            rows.append((m.group(1), m.group(2).strip(), float(m.group(3)), m.group(4) == "✅"))
    return rows, text


def compute(rows) -> tuple[float, float, int]:
    done = sum(u for _, _, u, ok in rows if ok)
    total = sum(u for _, _, u, _ in rows)
    if total <= 0:
        raise SystemExit("progress-gauge: 单位账是空的（§1 的表格没解析出来）")
    return done, total, round(100 * done / total)


def render(done: float, total: float, pct: int) -> tuple[str, str]:
    filled = round(BLOCKS * done / total)
    bar = f"距上线   [{'█' * filled}{'░' * (BLOCKS - filled)}]  {pct}%     已完成 {done:.1f} / {total:.1f} 单位"
    return bar, f"**done = {done:.1f} · total = {total:.1f} ⇒ {pct}%**"


def main(argv: list[str]) -> int:
    check = "--check" in argv
    as_json = "--json" in argv
    if not LEDGER.is_file() or not DOC.is_file():
        print(f"progress-gauge: 找不到台账 {LEDGER} 或视图 {DOC}", file=sys.stderr)
        return 2
    rows, _ = parse(LEDGER.read_text(encoding="utf-8"))
    text = DOC.read_text(encoding="utf-8")
    if not rows:
        print("progress-gauge: 台账表格没解析出来（每行要 `| 项 | 内容 | 单位 | ✅/⬜/⏸/⚠/✗ | 证据 |`）", file=sys.stderr)
        return 2
    done, total, pct = compute(rows)
    bar, line = render(done, total, pct)
    if as_json:
        print(json.dumps({"done": done, "total": total, "pct": pct, "rows": len(rows)}))
        return 0
    ok_bar = bar in text
    has_total = TOTAL_LINE.search(text) is not None
    ok_total = (not has_total) or (line in text)
    if check:
        if ok_bar and ok_total:
            print(f"progress-gauge ✓ {pct}%（{done:.1f}/{total:.1f}）与表一致")
            return 0
        print(
            "progress-gauge ✗ 进度条/合计数与 §1 的表**不一致**（这就是 2026-11-01 那次事故的形状）\n"
            f"  表算出：{bar}\n"
            f"  缺这一行：{'有' if ok_bar else '无'} · 缺合计行：{'有' if ok_total else '无'}\n"
            "  修：跑 `python3 scripts/progress-gauge.py`（它会写回）。",
            file=sys.stderr,
        )
        return 1
    # 写回：整体替换那两处，**必须命中**，否则报错（不许静默跳过）
    new_text, n_bar = BAR_LINE.subn(bar, text)
    if n_bar != 1:
        print(f"progress-gauge: 进度条那一行没有唯一命中（命中 {n_bar} 次）", file=sys.stderr)
        return 2
    new_text, n_total = TOTAL_LINE.subn(line, new_text)
    if n_total > 1:
        print(f"progress-gauge: 合计行命中 {n_total} 次（最多一处）", file=sys.stderr)
        return 2
    DOC.write_text(new_text, encoding="utf-8")
    print(f"progress-gauge ✓ 写回 {pct}%（{done:.1f}/{total:.1f}）")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
