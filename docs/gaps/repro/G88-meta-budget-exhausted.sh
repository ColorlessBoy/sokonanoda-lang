#!/usr/bin/env bash
# **G-88**：求解预算耗尽被当成「无解」✗（`DEFAULT_FUEL=4096` / `MAX_DEPTH=64` ⇒ `Tri::No`）。
#
# 退出码（`docs/gaps/README.md` 的约定 ✓）：
#   0 = 缺口仍在（**剩下的那半**：两个旋钮还是拍脑袋的常量 ✗ —— 「当成否」那半**已修** ✓）
#   1 = 行为变了（**「当成否」那半回退了** ✗ / 数值已对齐 Lean ⇒ 回来更新台账 ✓）
#   2 = 环境不对
#
# ⚠ **为什么用静态判据**（不是跑一遍课程 ✗）：`gap.py check` 是**每次 gate / CI 都跑**
# 的门禁 ✓ ⇒ 复现件必须**秒级**（跑课程要分钟级 ✗）。而这条缺口的"在不在"恰好
# **就是源码里的那个分支** ✓ —— 静态判据与语义**同构** ✓，不是偷懒 ✓。
# **真实触发次数**（整本课程 · release · 2026-10-04 ✓）记在台账 `today` 里 ✓：
# `meta_budget_exhausted=0` ✓（0 也留闸 + 断言 ✓，判据 = `crates/front/tests/gate_census.rs` ✓）。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

META="crates/front/src/compile/meta.rs"
[ -f "$META" ] || { echo "环境不对：找不到 $META" >&2; exit 2; }

# ① **「判不了 ⇒ 当成否」那一半必须已经修好** ✓（2026-10-04 第 15 棒 ✓）——
#    撞预算 ⇒ **加大预算重试** ✓，升满仍撞 ⇒ **弃权** ✓，**绝不再 `return Tri::No`** ✗。
#    ⚠ 这一条是**防回归**：撤掉修复 ⇒ 判 1 ⇒ 回来更新台账 ✓。
if ! grep -q 'sokonanoda::gates::META_BUDGET_ESCALATED.bump();' "$META"; then
  echo "行为变了：meta.rs 里「加大预算重试」不见了 ⇒ G-88 的「当成否」那半**回退了** ✗ ⇒ 更新台账"
  exit 1
fi
if grep -q 'return Tri::No; // 预算耗尽' "$META"; then
  echo "行为变了：meta.rs 里又出现「预算耗尽 ⇒ Tri::No」✗ ⇒ 那半回退了 ✗ ⇒ 更新台账"
  exit 1
fi
# ② 计数出口还在吗？（G-91 的要求 ✓）
if ! grep -q 'sokonanoda::gates::META_BUDGET_EXHAUSTED.bump();' "$META"; then
  echo "行为变了：预算弃权的计数出口不见了 ⇒ 更新台账"
  exit 1
fi
# ③ **剩下的那半**：两个旋钮**还是拍脑袋的常量** ✗（终点 = 可配置化 + 对齐 Lean ✓）。
#    它们一旦变成可配置 / 数值对齐 ⇒ 判 1 ⇒ 回来关账 ✓。
if ! grep -q '^const DEFAULT_FUEL: u32 = 4096;' "$META" \
   || ! grep -q '^const MAX_DEPTH: u32 = 64;' "$META"; then
  echo "行为变了：DEFAULT_FUEL / MAX_DEPTH 已可配置或数值已对齐 Lean ⇒ 更新台账"
  exit 1
fi
echo "缺口仍在：G-88 的「当成否」那半**已真修** ✓（撞预算 ⇒ 加大预算重试 ✓ / 弃权 ✓）"
echo "  ⚠ 剩下的那半**还没做** ✗：DEFAULT_FUEL=4096 / MAX_DEPTH=64 仍是拍脑袋的常量"
echo "  计数出口 = sokonanoda::gates::{META_BUDGET_ESCALATED, META_BUDGET_EXHAUSTED} ✓"
echo "  终点（值守 13:21/13:24）= ① 可配置化（默认值一个不动）② 对齐 Lean：20000 / 3200"
exit 0
