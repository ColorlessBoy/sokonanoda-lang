#!/usr/bin/env bash
# **G-90**：常量参数 ≥ 64 个 ⇒ 签名丢精度 ✗（`MAX_TRACKED=64` · `conv.rs` + `relevance.rs`）。
#
# 退出码：0 = 缺口仍在（`MAX_TRACKED=64` 还在）· 1 = 行为变了（已对齐 128 等）· 2 = 环境不对
#
# 真实触发次数（整本课程 · release · 2026-10-04 ✓）：
# `sig_overflow=0` · `sig_arity_clamped=0` ✓（0 也留闸 + 断言 ✓）。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

REL="crates/kernel/src/relevance.rs"
CONV="crates/kernel/src/conv.rs"
[ -f "$REL" ] && [ -f "$CONV" ] || { echo "环境不对：找不到 relevance.rs / conv.rs" >&2; exit 2; }

if ! grep -q 'pub(crate) const MAX_TRACKED: u32 = 64;' "$REL"; then
  echo "行为变了：MAX_TRACKED 不再是 64（已对齐 Lean 的 128，或已改成动态）⇒ 更新台账"
  exit 1
fi
# 两处计数出口（丢精度的两个落点 ✓）
if ! grep -q 'crate::gates::SIG_OVERFLOW.bump();' "$REL" \
   || ! grep -q 'crate::gates::SIG_ARITY_CLAMPED.bump();' "$CONV"; then
  echo "行为变了：MAX_TRACKED 的计数出口不见了（两处都要 ✓）⇒ 更新台账"
  exit 1
fi
echo "缺口仍在：G-90 MAX_TRACKED=64 还在（签名掩码只有 64 位 ⇒ 丢精度）"
echo "  计数出口 = crate::gates::SIG_OVERFLOW / SIG_ARITY_CLAMPED ✓（课程实测各 0 次 ✓）"
echo "  终点 = 对齐 Lean 的 synthInstance.maxSize = 128 ✓"
exit 0
