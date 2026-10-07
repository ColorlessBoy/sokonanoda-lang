#!/usr/bin/env bash
# **G-90**：常量参数 ≥ 64 个 ⇒ 签名丢精度 ✗（`MAX_TRACKED=64` · `conv.rs` + `relevance.rs`）
# —— **2026-10-07 已收口** ✓：闸值 64 → **128**（载体 `u64` → **`u128`** ✓）。
#
# 退出码：0 = 缺口仍在（64 / `u64` 又回来了，或状态不明 ✗）· 1 = 行为变了（**已加宽到 128** ✓
#         ⇒ 更新台账 ✓）· 2 = 环境不对（**判据本身失效**：闸 / 掩码 / 计数出口不见了 ✗）
#
# ## 终点与做法（台账 `expected_lean` ✓）
#
# **对齐 Lean 4** ✓：`synthInstance.maxSize` 的**默认值 = 128** ✓（`Meta/SynthInstance.lean:25`
# 的 `register_builtin_option` ✓，收口时**独立复核过** ✓）。⚠ **只有数字对齐** ✓ —— Lean 拿它
# 限「typeclass 求解中构造答案所用的实例项规模」（消费点 `SynthInstance.lean:425` 的
# `addAnswer`：`cNode.size ≥ maxResultSize` ⇒ **不产生这个答案** ✓），**不是**「签名里能记的
# 参数个数」✗；我们的是**签名相关性掩码的位宽** ✓（`relevance.rs::Sig` ✓）。
# 两侧**失败方向一致** ✓：都是「放弃捷径 / 放弃这个答案 ⇒ 变慢」✓，绝不是
# 「判不了 ⇒ 判否」✗。
#
# 做法（**语义一字没动** ✓）：载体 `u64` → **`u128`** ✓（128 位装得下 ✓）；默认值 64 → **128** ✓、
# `clamp(1, 64)` → `clamp(1, 128)` ✓（上界 = 载体宽度 ✓，**有理由** ✓）；**三个出口的方向
# 一个字没动** ✓（① `arg_is_ignorable` 掩码外 false = 照样比 ✓ · ② `result_is_not_proof`
# 掩码外 false ⇒ `statically_not_proof` 不触发 ⇒ 继续走更贵的检查 ✓ · ③ `absent_args`
# 少记 ⇒ 位更少 ⇒ 只多比 ✓）。计数出口保留 ✓ 且仍咬得住 ✓（**≥128 个参数才触发** ✓）。
#
# ## 为什么判据是**结构 + 行为两半**（同 G-88/G-89 ✓）
#
# 掩码是**纯优化** ⇒ 加宽后整门课 `--json` **逐字节相同** ✓（实测 50561 行 ✓）
# ⇒ 「闸值变了」这件事**行为上不可观测** ✗（除非语料里真出现 64..127 个参数的签名 ✓）
# ⇒ 结构那一半只能钉源码 ✓（秒级；`gap.py check` 每次 gate/CI 都跑 ✓）。
# **行为**那一半在 `crates/front/tests/g90_max_tracked.rs` ✓（**独立进程**读**计数差量** ✓：
# 64..127 个参数 ⇒ `sig_overflow`/`sig_arity_clamped` **都不许涨** ✓；128/129 ⇒ **必须涨** ✓；
# 假的相等（含「值体用到 de Bruijn 下标 127」那条 ✓）**仍必须被拒** ✗）；
# 掩码语义（哪一位能表示、掩码外必须保守 ✓）在 `crates/kernel/src/tests/sig_mask.rs` ✓。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

REL="crates/kernel/src/relevance.rs"
CONV="crates/kernel/src/conv.rs"
GATES="crates/kernel/src/gates.rs"
for f in "$REL" "$CONV" "$GATES"; do
  [ -f "$f" ] || { echo "环境不对：找不到 $f" >&2; exit 2; }
done

# ── ① **判据本身还在吗**（少了任何一条 ⇒ 本判据**失效** ⇒ exit 2 ✓，不是「已修」✓）──────────
#    闸的取值口 / 五个掩码 / 两个计数出口 —— 它们是这条缺口的**观测面** ✓。
grep -q 'pub(crate) fn max_tracked() -> u32' "$REL" \
  || { echo "环境不对：\`relevance.rs::max_tracked\` 不见了 ⇒ 本判据失效" >&2; exit 2; }
for field in prop_arg arg_known absent_arg prop_result result_known; do
  grep -qE "$field: u(64|128)," "$REL" \
    || { echo "环境不对：\`Sig::$field\` 不见了 ⇒ 本判据失效" >&2; exit 2; }
done
grep -q 'crate::gates::SIG_OVERFLOW.bump();' "$REL" \
  || { echo "环境不对：\`SIG_OVERFLOW\` 的写入点不见了 ⇒ 本判据失效" >&2; exit 2; }
grep -q 'crate::gates::SIG_ARITY_CLAMPED.bump();' "$CONV" \
  || { echo "环境不对：\`SIG_ARITY_CLAMPED\` 的写入点不见了 ⇒ 本判据失效" >&2; exit 2; }

# ── ② **缺口还在吗**（旧状态 ✓）：默认 64 / 上界 64 / 载体 `u64` ─────────────────────────
if grep -q 'unwrap_or(64).clamp(1, 64)' "$GATES" \
   || grep -qE '^(pub\(crate\) )?(prop_arg|arg_known|absent_arg|prop_result|result_known): u64,' "$REL"; then
  echo "缺口仍在：G-90 的 MAX_TRACKED=64（或 u64 载体）还在 ⇒ 常量参数 ≥64 个就丢精度 ✗"
  echo "  终点 = 对齐 Lean 的 synthInstance.maxSize = **128** ✓（载体 = u128 ✓）"
  exit 0
fi

# ── ③ **必须是新状态**（128 + u128 + 精确 used 查询 ✓）—— 不是 ⇒ 判 0（保守 ✓ 让人来看）────
#    ⚠ 这里**故意**用「不是终点 ⇒ 缺口仍在」的极性 ✓：判 0 会让 `gap.py check` 判红 ✓
#    （`status=fixed` 要求复现件 exit≠0 ✓）⇒ 状态不明时**有人来看** ✓，不会静默放行 ✗。
if ! grep -q 'unwrap_or(128).clamp(1, 128)' "$GATES"; then
  echo "缺口仍在（或状态不明）：MAX_TRACKED 的默认值/上界不是 \`unwrap_or(128).clamp(1, 128)\` ✗"
  echo "  终点 = 对齐 Lean 的 128 ✓ —— 若有意改到别处，请同时更新台账与 \`Sig\` 的说明"
  exit 0
fi
for field in prop_arg arg_known absent_arg prop_result result_known; do
  if ! grep -qE "$field: u128," "$REL"; then
    echo "缺口仍在（或状态不明）：\`Sig::$field\` 的载体不是 u128 ✗（128 位装不下 ⇒ 又丢精度 ✗）"
    exit 0
  fi
done
# 第三条出口（`absent_args` ✓）的**精确** used 查询：值体的 loose bvar 超过 64 时
# `Expr::fv_mask`（u64）够不着 ⇒ 必须逐位查 ✓（谁把它简化回位掩码快路 = **少比** = 变错 ✗）。
if ! grep -q '!self.ctx.has_loose_bvar(body, j as u16)' "$REL"; then
  echo "缺口仍在（或状态不明）：\`absent_args\` 的精确 used 查询不见了 ✗（arity > 64 时会少比 ✗）"
  exit 0
fi

echo "行为变了：G-90 的 MAX_TRACKED **已加宽到 128** ✓（载体 u64 → u128 ✓，默认 64 → 128 ✓）"
echo "  三个出口方向不变 ✓（丢精度 ⇒ 多比 ✓，绝不少比 ✗）；计数出口仍在 ✓ 且 ≥128 才触发 ✓"
echo "  行为判据 = crates/front/tests/g90_max_tracked.rs ✓ · 掩码语义 = crates/kernel/src/tests/sig_mask.rs ✓"
exit 1
