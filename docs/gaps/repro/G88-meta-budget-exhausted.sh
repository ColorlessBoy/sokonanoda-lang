#!/usr/bin/env bash
# **G-88**：求解预算耗尽被当成「无解」✗（`DEFAULT_FUEL=4096` / `MAX_DEPTH=64` ⇒ `Tri::No`）。
#
# 退出码（`docs/gaps/README.md` 的约定 ✓）：
#   0 = 缺口仍在（那条「判不了 ⇒ 当成否」的分支**还在代码里** ✓）
#   1 = 行为变了（闸被去掉 / 被改成「弃权」/ 数值已对齐 Lean ⇒ 回来更新台账 ✓）
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

# ① 那条「预算耗尽 ⇒ 当成否」的分支还在吗？（`return Tri::No` ✓）
if ! grep -q 'sokonanoda::gates::META_BUDGET_EXHAUSTED.bump();' "$META"; then
  echo "行为变了：meta.rs 里 G-88 的计数出口不见了 ⇒ 闸可能已被去掉/改路 ⇒ 更新台账"
  exit 1
fi
if ! grep -q 'return Tri::No; // 预算耗尽' "$META"; then
  echo "行为变了：meta.rs 里「预算耗尽 ⇒ Tri::No」那条分支不见了 ⇒ 更新台账"
  exit 1
fi
# ② 两个旋钮还在（终点 = 可配置化 + 对齐 Lean：`maxHeartbeats` 20000 / `maxRecDepth` 3200 ✓）
if ! grep -q '^const DEFAULT_FUEL: u32 = 4096;' "$META" \
   || ! grep -q '^const MAX_DEPTH: u32 = 64;' "$META"; then
  echo "行为变了：DEFAULT_FUEL / MAX_DEPTH 的数值变了（或已可配置）⇒ 更新台账"
  exit 1
fi
echo "缺口仍在：G-88「预算耗尽 ⇒ Tri::No」还在（DEFAULT_FUEL=4096 / MAX_DEPTH=64）"
echo "  计数出口 = sokonanoda::gates::META_BUDGET_EXHAUSTED ✓（课程实测 0 次 ✓）"
echo "  终点（值守 13:21/13:24）= ① 可配置化（默认值一个不动）② 对齐 Lean：20000 / 3200"
exit 0
