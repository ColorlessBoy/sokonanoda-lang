#!/usr/bin/env bash
# ============================================================================
# C-05 复现（课程侧 · 装配写法）：**关系版加法的结合律尚未装配**
#
# 目标命题：`AddsTo x y z` 与 `AddsTo z w v` ⇒ 存在 `u` 使
#   `AddsTo y w u` 且 `AddsTo x u v`（即 `(x + y) + w = x + (y + w)`）。
# 装配前先要**逆引理**：`AddsTo x (s y) v` ⇒ 存在 `z` 使 `v = s z` 且 `AddsTo x y z`。
#
# 登记状态：**open**（`OPEN-ITEMS.md` C-05，2026-10-02 第 681 轮登记）
#   ⇒ 按台账约定，`open` 的期望是"**缺口仍在**" = 本脚本 **exit 0**：
#      · 零件在（`AddsTo` 归纳关系 + `addsTo_zero` + `addsTo_succ`）；
#      · 装配件**还不在**（`addsTo_succ_inv` / `addsTo_assoc` 都没进解答）。
#   一旦装配件进了解答 ⇒ 本脚本 **exit 1** ⇒ 提示把登记从 `open` 改成 `closed`
#   并把本脚本的期望翻转（这才是"断言与登记一致"）。
#
# 卡点（第 681 轮实测，工程问题、非内核墙）：`AddsTo.rec` 的**调用形状**
#   （动机位/次前提参数序）判红一次；下一步用 `sorry` 叶子做**逐层隔离**定位。
#
# 用法：bash courses/set-theory/gaps/C-05-addsTo-assoc.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOL="$ROOT/courses/set-theory/units/solutions/I.6/unit109-solution.sokonanoda"
fail=0

[ -f "$SOL" ] || { echo "missing solution file: $SOL"; exit 1; }

for name in AddsTo addsTo_zero addsTo_succ; do
  if grep -q "^inductive $name \|^theorem $name \|^def $name " "$SOL"; then
    echo "OK  part present: $name"
  else
    echo "BAD C-05: part missing from the solution: $name (regression)"
    fail=1
  fi
done

for name in addsTo_succ_inv addsTo_assoc; do
  if grep -q "^theorem $name " "$SOL"; then
    echo "BAD C-05: assembly lemma IS present now: $name"
    echo "    -> the register must move from open to closed and this script's expectation flips"
    fail=1
  else
    echo "OK  C-05 gap still open (as registered): $name not yet in the solution"
  fi
done

exit $fail
