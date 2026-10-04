#!/usr/bin/env bash
# **G-89**：相等性探查预算耗尽被当成「不相等」✗（`PROBE_CAP=2048` · `kernel/src/conv.rs`）。
#
# 退出码：0 = 缺口仍在（`PROBE_CAP` 还在）· 1 = 行为变了（已去掉 / 已改成弃权）· 2 = 环境不对
#
# ⚠ 静态判据的理由同 G-88 ✓（`gap.py check` 每次 gate/CI 都跑 ⇒ 必须秒级 ✓）。
# 真实触发次数（整本课程 · release · 2026-10-04 ✓）：`probe_exhausted=0` ✓。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

CONV="crates/kernel/src/conv.rs"
[ -f "$CONV" ] || { echo "环境不对：找不到 $CONV" >&2; exit 2; }

# ① 这道闸还在吗？（**Lean 4 根本没有此物** ✓ ⇒ 终点是**去掉** ✓，
#    **不许换个数字继续留着** ✗）
#    ⚠ 2026-10-04 第 15 棒起，取值走 `gates::limits::probe_cap()`（**默认 2048 不变** ✓，
#    但可注入 ⇒ 「拧到 1 结论会不会变」能做实验 ✓）—— 判据因此改成查**闸本身** ✓。
if ! grep -q 'fn probe_cap() -> u32' "$CONV"; then
  echo "行为变了：conv.rs 里探查预算那道闸不见了（去掉或已改路）⇒ 更新台账"
  exit 1
fi
if ! grep -q 'unwrap_or(2048)' crates/kernel/src/gates.rs; then
  echo "行为变了：探查预算的默认值不再是 2048（已对齐 Lean 的「没有此物」= 去掉）⇒ 更新台账"
  exit 1
fi
# ② 计数出口还在吗？（G-91 的要求 ✓：凡「超过某个数字就换路」必须可见 ✗）
if ! grep -q 'crate::gates::PROBE_EXHAUSTED.bump();' "$CONV"; then
  echo "行为变了：PROBE_CAP 的计数出口不见了 ⇒ 更新台账"
  exit 1
fi
# ③ 耗尽**必须**只表示「弃权走全量」✓，**不许**表示「不相等」✗ ——
#    `probe_pass` 必须读 `probe_exhausted` 并把它当**未决**处理 ✓。
if ! grep -q 'if self.tc_cache.probe_exhausted {' "$CONV"; then
  echo "行为变了：probe_pass 不再把「耗尽」当未决处理 ⇒ 更新台账"
  exit 1
fi
echo "缺口仍在：G-89 PROBE_CAP=2048 还在（耗尽 ⇒ 弃权，但 Lean 没有这道闸）"
echo "  计数出口 = crate::gates::PROBE_EXHAUSTED ✓（课程实测 0 次 ✓）"
echo "  终点 = **去掉**（不许换数字留着 ✗）；去掉前先看清触发次数 ✓"
exit 0
