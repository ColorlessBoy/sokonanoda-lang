#!/usr/bin/env bash
# **G-91**：闸类可观测性 ✗ —— 全仓 `SOKO_*` 开关里**没有一个是「闸被触发」的计数出口**。
#
# 退出码：0 = 缺口仍在（出口还没铺全）· 1 = 行为变了（全部铺齐 ⇒ 更新台账）· 2 = 环境不对
#
# 本笔（2026-10-04）已铺**内核侧六个**出口 ✓（`crates/kernel/src/gates.rs` ✓）：
#   probe_exhausted · sig_overflow · sig_arity_clamped ·
#   unify_rounds_exhausted · unify_no_progress · meta_budget_exhausted
# 并接进 `STAGE_STATS`（`SOKO_STAGE_STATS=1` ✓）⇒ **CI / 人都看得见** ✓。
# ⚠ **乙类 4 处还没铺** ✗（`judge.rs` 的两张表容量 · 目标分解失败 · `SKELETON_MAX_LAYERS` ✓）
# ⇒ 本复现**保持判"缺口仍在"** ✓（0 = 与 `status: open` 自洽 ✓）。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

GATES="crates/kernel/src/gates.rs"
[ -f "$GATES" ] || { echo "环境不对：找不到 $GATES" >&2; exit 2; }

# ① 六个出口都得有 `bump()` 落点 ✓（只声明不写 = 空转 ✗）。
missing=""
for name in PROBE_EXHAUSTED SIG_OVERFLOW SIG_ARITY_CLAMPED \
            UNIFY_ROUNDS_EXHAUSTED UNIFY_NO_PROGRESS META_BUDGET_EXHAUSTED; do
  if ! grep -rq "gates::$name.bump()" crates/kernel/src crates/front/src; then
    missing="$missing $name"
  fi
done
if [ -n "$missing" ]; then
  echo "行为变了：这些出口没有写入点（= 空转 ✗）：$missing ⇒ 更新台账"
  exit 1
fi
# ② 出口必须**可见** ✓（接进 `STAGE_STATS` ✓）—— 没有可见性 = 等于没有 ✗。
if ! grep -q 'sokonanoda::gates::report()' crates/front/src/compile/check/mod.rs; then
  echo "行为变了：闸类计数没接进 STAGE_STATS ⇒ 更新台账"
  exit 1
fi
# ③ 乙类 4 处**还没铺** ✗ ⇒ 缺口仍在 ✓（铺齐之后这里会判 1 ⇒ 回来关账 ✓）。
echo "缺口仍在：G-91 内核侧六个出口已铺 ✓，**乙类 4 处还没铺** ✗"
echo "  已铺：crates/kernel/src/gates.rs + STAGE_STATS（课程实测见台账 today ✓）"
echo "  未铺（乙类）：judge.rs 两张表容量 · 目标分解失败（goals.rs）· SKELETON_MAX_LAYERS"
exit 0
