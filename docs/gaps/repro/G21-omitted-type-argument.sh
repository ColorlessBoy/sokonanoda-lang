#!/usr/bin/env bash
# G-21 自断言复现：点名调用漏了**前导类型参数**（`mymem a A` 少了 `α`）时，
# 报错能不能指出根因？
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
#
# 台账 `status = open`（**半修**，0.62.0 实测）：
#   ① 声明位已修 —— `def … : Prop := mymem a A` 报 `kernel-expected-sort`，
#      hint 里点名「点名调用漏了前导类型参数」（2026-09-21，error.rs 的
#      `classify_term_in_type_position`）；
#   ② **`by` 路径仍欠** —— `… : mymem a A -> A a := by intro h; exact h` 报
#      `elab-tactic-failed`，文案是「期望 `A a`，实际是 `mymem a A`」这种**同形**
#      对照，学习者看不出缺的是 `α`。
# 本脚本断言的正是**②仍在**：一旦 ② 也指到根因（诊断码变成
# `kernel-expected-sort`，或 hint/message 里出现「前导类型参数」），脚本转 exit 1，
# `gap.py check` 就会提醒把台账改成 `fixed`。

set -u
cd "$(dirname "$0")/../../.." || exit 2
ROOT="$PWD"
FIXTURE="$ROOT/docs/gaps/repro/G21-omitted-type-argument.sokonanoda"

OUT="$(scripts/soko grade "$FIXTURE" --json 2>&1)"
printf '%s\n' "$OUT" | python3 -c '
import json, sys

ev = []
for line in sys.stdin:
    line = line.strip()
    if line.startswith("{"):
        try:
            ev.append(json.loads(line))
        except json.JSONDecodeError:
            pass
diags = [e for e in ev if e.get("type") == "diagnostic"]
if not diags:
    print("形状异常：夹具应当判红，却一条诊断都没有", file=sys.stderr)
    sys.exit(2)

def find(needle):
    for d in diags:
        if needle in (d.get("message") or ""):
            return d
    return None

sig = find("Sort(1)") or find("kernel-expected-sort")
by_path = None
for d in diags:
    msg = d.get("message") or ""
    if "exact" in msg or "mymem a A" in msg:
        by_path = d
        break
if by_path is None:
    print("形状异常：`by` 路径那条诊断没找到：%r" % (diags,), file=sys.stderr)
    sys.exit(2)

sig_ok = sig is not None and sig.get("code") == "kernel-expected-sort" \
    and "前导类型参数" in (sig.get("hint") or "")
by_hint = ((by_path.get("hint") or "") + (by_path.get("message") or ""))
by_ok = by_path.get("code") == "kernel-expected-sort" or "前导类型参数" in by_hint

print("① 声明位：code=%s，hint 指到根因=%s" % (sig.get("code") if sig else None, sig_ok))
print("② by 路径：code=%s，message=%s" % (by_path.get("code"), (by_path.get("message") or "")[:60]))
print("   ② 指到根因=%s" % by_ok)

if not sig_ok:
    print("⚠️ ① 的修法回退了（声明位不再指根因）——请先查 error.rs 的分类。", file=sys.stderr)
if by_ok:
    print("结论：两半都指到根因 ⇒ G-21 已修。")
    sys.exit(1)
print("结论：`by` 路径仍给同形对照、看不出缺 `α` ⇒ G-21 仍在（半修）。")
sys.exit(0)
'
