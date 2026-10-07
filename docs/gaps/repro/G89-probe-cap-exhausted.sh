#!/usr/bin/env bash
# **G-89**：相等性探查预算 `PROBE_CAP=2048`（`kernel/src/conv.rs`）——**2026-10-07 已去掉** ✓。
#
# 退出码：0 = 缺口仍在（闸 / 出口又回来了 ✗）· 1 = 行为变了（**已去掉** ⇒ 更新台账 ✓）·
#         2 = 环境不对（**判据本身失效**：探查不见了 ✗）
#
# ## 台账的诊断更正过一条（别再踩 ✗）
#
# 原文说「`probe_exhausted=true; return false`」= 判不等 ✗ —— 逐处读消费端后**不成立** ✓：
# `spine_probe` 的 `false` 在两个调用点都只是「**这次探查没定下来**」✓ ⇒ 调用方**继续走全量** ✓
# （只有 `return true` 才是「相等」的快路 ✓）⇒ 探查是**纯优化** ✓。
#
# ## 终点与做法（值守 2026-10-04 13:24 ✓）
#
# **A 类：Lean 4 根本没有此物** ✓ ⇒ **去掉**（**不许换个数字继续留着** ✗）。
# 去掉的三样 ✓：① 旋钮（`gates.rs::limits` 登记表里的 `PROBE_CAP`，含 `SOKO_LIMIT_*` 口子 ✗）；
# ② 步数预算字段 + 耗尽标志；③ 耗尽早退分支 ✓。**探查本身一字未动** ✓（`spine_probe` /
# `probe_pairs` / `probe_pass` ✓）—— 它**跑到底** ✓，终止性靠**结构**（不嵌套 + 参数对有限
# + 每对走普通 `unify` ✓，见 `conv.rs::spine_probe` 的说明 ✓）。
#
# **去掉 ≠ 静默** ✓：耗尽这件事**不存在了** ⇒ 计数出口**没有写入点** ⇒ **出口也一起删** ✓
# （留一个**永不 bump** 的空转出口 ✗ —— 读数永远 0，而 0 的含义从「没触发」变成
# 「这条闸不存在」✗ ⇒ 分不清 = 假守卫 ✗）。守卫换了形态 ✓（见下 ③ ✓）。
#
# ## 为什么是**静态**判据（同 G-88 ✓）
#
# 探查是纯优化 ⇒ 闸在 / 闸不在，判定输出**逐字节相同** ✓（整门课实测 50068 行 ✓）
# ⇒ 「闸没了」**行为上不可观测** ✗ ⇒ 只能钉**结构** ✓（`gap.py check` 每次 gate/CI 都跑 ⇒ 必须秒级 ✓）。
# **行为**那一半在 `crates/front/src/compile/tests.rs::probe_*` ✓（该判相等的仍判相等 ✓ /
# 刚性不等仍判不等 ✓ / **以前会耗尽旧预算的夹具现在仍判相等** ✓）+ 源码级那一半在
# `crates/kernel/src/tests/probe.rs` ✓。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

CONV="crates/kernel/src/conv.rs"
GATES="crates/kernel/src/gates.rs"
[ -f "$CONV" ] || { echo "环境不对：找不到 $CONV" >&2; exit 2; }
[ -f "$GATES" ] || { echo "环境不对：找不到 $GATES" >&2; exit 2; }

# ① **闸还在吗？**（在 ⇒ 缺口仍在 ⇒ exit 0 ✓）。查五处痕迹：
#    旋钮函数 / 预算字段 / 耗尽标志 / 只在探查内读的截断态否定缓存 / 登记表里的默认值 ✓。
if grep -q 'fn probe_cap() -> u32' "$CONV" \
   || grep -q 'probe_budget' "$CONV" \
   || grep -q 'probe_exhausted' "$CONV" \
   || grep -q 'conv_cache_neg_probe' "$CONV" \
   || grep -q 'unwrap_or(2048)' "$GATES" \
   || grep -q 'SOKO_LIMIT_PROBE_CAP' "$GATES"; then
  echo "缺口仍在：G-89 的探查预算还在（旋钮 / 预算字段 / 耗尽标志 / 截断态缓存 / 2048 默认值 ✗）"
  echo "  终点 = **去掉**（Lean 4 没有此物 ✓，不许换数字留着 ✗）"
  exit 0
fi

# ② **出口有没有变成空转？**（闸没了 ⇒ 耗尽不存在 ⇒ 出口**不许**留着 ✗）。
if grep -rq 'PROBE_EXHAUSTED' crates/kernel/src crates/front/src \
   || grep -q '("probe_exhausted"' "$GATES"; then
  echo "缺口仍在：闸已去掉，但**耗尽出口还在** ✗（没有写入点 = 空转出口 ✗）⇒ 一起删"
  exit 0
fi

# ③ **探查本身必须还在，而且仍是「纯优化」** ✓（去掉预算 ≠ 去掉探查 ✗）。
#    —— 少了任何一条 ⇒ **判据失效**（exit 2 ✓，不是「已修」✓）。
for needle in 'fn spine_probe' 'fn probe_pairs' 'fn probe_pass'; do
  if ! grep -q "$needle" "$CONV"; then
    echo "环境不对：探查 \`$needle\` 不见了 ⇒ 本判据失效（去掉预算不许动探查 ✗）" >&2
    exit 2
  fi
done
# 两个调用点都必须是「`true` 才算相等、`false` 继续走全量」✓（语义一个字不许动 ✗）。
if [ "$(grep -c 'if heads_match && self.spine_probe(depth, sx, sy, sig, limit) {' "$CONV")" -lt 2 ]; then
  echo "环境不对：探查的调用点不再是「抄近路才 return true」的形状 ⇒ 本判据失效" >&2
  exit 2
fi

echo "行为变了：G-89 的探查预算**已去掉** ✓ —— 旋钮 / 2048 默认值 / 耗尽早退 / 空转出口全没了"
echo "  探查仍在（spine_probe / probe_pairs / probe_pass ✓）且**跑到底** ✓（终止性 = 结构 ✓）"
echo "  行为判据 = crates/front/src/compile/tests.rs::probe_* ✓（含「以前会耗尽的夹具仍判相等」✓）"
exit 1
