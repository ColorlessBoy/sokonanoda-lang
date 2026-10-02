#!/usr/bin/env bash
# ============================================================================
# 112 复现（课程侧 · **未开工**的登记）：Cantor–Bernstein **尚未装配**
#
# 目标命题（Enderton §6.4 定理 6B / Halmos §22）：`A ≼ B` 且 `B ≼ A` ⇒ `A ≈ B`。
#
# 登记状态：**open**（`OPEN-ITEMS.md` C-112）
#   ⇒ 按台账约定，`open` 的期望是"**缺口仍在**" = 本脚本 **exit 0**：
#      · 零件在（单射/满射/子集的词汇 `Set.InjOn` / `LeOn` / `Set.Equiv` ✓ + `choice` 公理 ✓）；
#      · 装配件**还不在**（任何解答里都没有 CB 主定理）。
#   一旦 CB 主定理进了解答 ⇒ 本脚本 **exit 1** ⇒ 提示把登记改成 `closed` 并翻转期望。
#
# ⚠⚠ **诚实标注（必须随单元一起写进单元头）**：本课的 CB 打算**经选择公理**
#   （用 `choice` 从 `∀ b ∈ B, ∃ a ∈ A, …` 里取出前像函数）—— 而 **CB 数学上不需要选择**
#   ⇒ 这是**偏离**，不是"标准证法" ✗。零公理的 CB 需要 `Exists`→Type 的安全消去（**G-6** 一族）。
#
# 卡点（本轮报告）：不是墙，是**预算** —— 链构造（`C₀ = ⋂{C | A∖B ⊆ C ⊆ A, f(C) ⊆ C}`）
#   是多步证明，需要先立 2–3 条零件引理；G-63（`Quot`）**与 CB 无关**（CB 走单射/满射/子集 ✓）。
#
# 用法：bash courses/set-theory/gaps/C-112-cantor-bernstein.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOLS="$ROOT/courses/set-theory/units/solutions"
LIB="$ROOT/courses/set-theory/lib"
fail=0

for f in "$LIB/Equiv.sokonanoda" "$LIB/Choice.sokonanoda"; do
  [ -f "$f" ] || { echo "missing library file: $f"; exit 1; }
done
if grep -q "InjOn" "$LIB/Equiv.sokonanoda" && grep -q "choice" "$LIB/Choice.sokonanoda"; then
  echo "OK  parts present: Set.InjOn (lib/Equiv) + choice (lib/Choice)"
else
  echo "BAD 112: the parts are gone (InjOn / choice)"
  fail=1
fi

hit="$(grep -rl "^theorem cantorBernstein \|^theorem cantor_bernstein \|^theorem cb_" "$SOLS" 2>/dev/null | head -3)"
if [ -z "$hit" ]; then
  echo "OK  112 gap still open (as registered): no CB main theorem in any solution yet"
else
  echo "BAD 112: a CB main theorem IS present now: $hit"
  echo "    -> the register must move from open to closed and this script's expectation flips"
  fail=1
fi

exit $fail
