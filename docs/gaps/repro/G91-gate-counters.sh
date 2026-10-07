#!/usr/bin/env bash
# **G-91**：闸类可观测性 ✗ —— 全仓 `SOKO_*` 开关里**没有一个是「闸被触发」的计数出口**。
#
# 退出码：0 = 缺口仍在（出口还没铺全）· 1 = 行为变了（全部铺齐 ⇒ 更新台账）· 2 = 环境不对
#
# 本笔（2026-10-04）已铺**内核侧六个**出口 ✓（`crates/kernel/src/gates.rs` ✓）：
#   probe_exhausted · sig_overflow · sig_arity_clamped ·
#   unify_rounds_exhausted · unify_no_progress · meta_budget_exhausted
# 并接进 `STAGE_STATS`（`SOKO_STAGE_STATS=1` ✓）⇒ **CI / 人都看得见** ✓。
# 第二笔（2026-10-07 ✓）铺齐**乙类 4 处** ✓（同一份 `gates.rs`，**追加在末尾** ✓）：
#   judge_cache_evicted（`JUDGE_CACHE_CAP` FIFO 挤掉 ⇒ 下次 miss ✓）
#   const_sig_cache_full（常量签名缓存满 ⇒ 静默停止写入 ✓）
#   goal_decompose_fallback（目标分解失败 ⇒ generic 兜底 ⇒ 显示降质 ✓）
#   skeleton_layers_clamped（`SKELETON_MAX_LAYERS=3` 截断重启骨架 ✓）
# ⇒ **四处全铺齐 ⇒ 本复现判 1** ✓（`status: fixed` 与它自洽 ✓）。
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
# ③ 乙类 4 处（2026-10-07 第二笔 ✓）：逐项核**三件事** ✓ ——
#    名字在 `gates.rs` 里（`pub static … : Counter` ✓）+ 有真实 `bump()` 落点 ✓
#    + 接进 `report()` ✓（`report()` 已进 `STAGE_STATS` ⇒ ② 那条覆盖可见性 ✓）。
#    ⚠ 断言的是「**出口铺齐**」这件事本身 ✓（不是恒真 ✗）：任一项缺 ⇒ 判 0（缺口仍在 ✓）。
missing_b=""
for pair in JUDGE_CACHE_EVICTED:judge_cache_evicted \
            CONST_SIG_CACHE_FULL:const_sig_cache_full \
            GOAL_DECOMPOSE_FALLBACK:goal_decompose_fallback \
            SKELETON_LAYERS_CLAMPED:skeleton_layers_clamped; do
  name="${pair%%:*}"; snake="${pair##*:}"
  grep -q "pub static $name: Counter" "$GATES" || missing_b="$missing_b $name(未声明)"
  grep -rq "gates::$name.bump()" crates/kernel/src crates/front/src \
    || missing_b="$missing_b $name(无落点)"
  grep -q "(\"$snake\", $name.get())" "$GATES" || missing_b="$missing_b $name(未接进 report)"
done
if [ -n "$missing_b" ]; then
  echo "缺口仍在：G-91 乙类 4 处还没铺齐 ✗：$missing_b"
  echo "  已铺：crates/kernel/src/gates.rs（内核侧七项 + 乙类已铺的那些）+ STAGE_STATS"
  exit 0
fi
echo "行为变了：G-91 已收口 ✓ —— 内核侧七项 + **乙类 4 处**出口全铺齐（声明 + 落点 + report）"
echo "  出口：judge_cache_evicted · const_sig_cache_full · goal_decompose_fallback · skeleton_layers_clamped"
echo "  可见性：SOKO_STAGE_STATS=1 的 STAGE_STATS 行（课程实测读数见台账 today ✓）⇒ 更新台账"
exit 1
