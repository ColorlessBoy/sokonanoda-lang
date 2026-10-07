#!/usr/bin/env bash
# **G-88**：求解预算（`DEFAULT_FUEL=4096` / `MAX_DEPTH=64`）**拍脑袋**，且耗尽被当成「无解」✗。
#
# 退出码（`docs/gaps/README.md` 的约定 ✓）：
#   0 = **缺口回来了**（任一条断言不成立 ⇒ 旧行为：拍脑袋的常量 / 判不了 ⇒ 当成否 ✗）
#   1 = **修后形状成立** ✓（可配置 ✓ + 默认值 = Lean 的数值 ✓ + 「绝不判否」✓）⇒ 台账已关账 ✓
#   2 = 环境不对
#
# ⚠ **为什么用静态判据**（不是跑一遍课程 ✗）：`gap.py check` 是**每次 gate / CI 都跑**
# 的门禁 ✓ ⇒ 复现件必须**秒级**（跑课程要分钟级 ✗）。而这条缺口的"在不在"恰好
# **就是源码里的那几个分支/取值** ✓ —— 静态判据与语义**同构** ✓，不是偷懒 ✓。
# **真实触发次数**（整本课程 · release · 2026-10-04 ✓）记在台账 `today` 里 ✓：
# `meta_budget_exhausted=0` ✓（0 也留闸 + 断言 ✓，判据 = `crates/front/tests/gate_census.rs` ✓）。
#
# ⚠ **本件的极性在收口时翻过** ✓（G-88 第二笔 · 2026-10-07 ✓，照 `G13-…sh` 的先例 ✓）：
# 关账后 `status=fixed` ⇒ `gap.py check` 期望**非零** ✓。所以现在「修后形状成立」= exit 1 ✓、
# 「任一条守卫咬住」= exit 0 ✓（**旧版**是反的 ✗ —— 那会让关账后的守卫**永远空转** ✗）。
# ①②**两条断言的内容一个字没改** ✓（只把失败时的出口从 1 改成 0 ✓）。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

META="crates/front/src/compile/meta.rs"
GATES="crates/kernel/src/gates.rs"
[ -f "$META" ] || { echo "环境不对：找不到 $META" >&2; exit 2; }
[ -f "$GATES" ] || { echo "环境不对：找不到 $GATES" >&2; exit 2; }

# 缺口回来了的统一出口（= 旧行为 / 守卫被咬住 ⇒ 台账该改回 open ✓）。
gap_back() {
  echo "缺口回来了：$1" >&2
  exit 0
}

# ① **「判不了 ⇒ 当成否」那一半必须保持已修** ✓（2026-10-04 第 15 棒 ✓）——
#    撞预算 ⇒ **加大预算重试** ✓，升满仍撞 ⇒ **弃权** ✓，**绝不再 `return Tri::No`** ✗。
#    （断言内容与收口前逐字相同 ✓）
if ! grep -q 'sokonanoda::gates::META_BUDGET_ESCALATED.bump();' "$META"; then
  gap_back "meta.rs 里「加大预算重试」不见了 ⇒ G-88 的「当成否」那半**回退了** ✗"
fi
if grep -q 'return Tri::No; // 预算耗尽' "$META"; then
  gap_back "meta.rs 里又出现「预算耗尽 ⇒ Tri::No」✗ ⇒ 那半回退了 ✗"
fi
# ② 计数出口还在吗？（G-91 的要求 ✓；断言内容与收口前逐字相同 ✓）
if ! grep -q 'sokonanoda::gates::META_BUDGET_EXHAUSTED.bump();' "$META"; then
  gap_back "预算弃权的计数出口不见了"
fi

# ③ **第二笔（2026-10-07 ✓）：旋钮可配置 ✓ 且默认值 = Lean 的数值** ✓。
#    钉三件事（**不是删断言** ✗）：登记在**同一处** ✓ · 默认值 = Lean ✓ · **真的接到求解器上** ✓。
for knob in SOKO_LIMIT_MAX_HEARTBEATS SOKO_LIMIT_MAX_REC_DEPTH SOKO_LIMIT_MAX_ESCALATIONS; do
  if ! grep -q "\"$knob\"" "$GATES"; then
    gap_back "闸取值登记表里没有 $knob ⇒ 旋钮又变回不可配置 ✗"
  fi
done
# 默认值 = Lean 的数值（出处：`synthInstance.maxHeartbeats` = 20000 ✓ ·
# `maxRecDepth` = 512 ✓ —— `Meta/SynthInstance.lean:20` / `Util/RecDepth.lean:15`
# ⇒ `Init/Prelude.lean:4760`，2026-10-07 本机 Lean 复核 ✓）。
if ! grep -q 'pub const DEFAULT_MAX_HEARTBEATS: u32 = 20000;' "$GATES"; then
  gap_back "默认步数预算不再是 Lean 的 20000 ✗（拍脑袋的常量回来了）"
fi
if ! grep -q 'pub const DEFAULT_MAX_REC_DEPTH: u32 = 512;' "$GATES"; then
  gap_back "默认递归深度不再是 Lean 的 512 ✗（拍脑袋的常量回来了）"
fi
# **可配置真的接上了** ✓：求解器必须**从登记表取值**（不是本地常量 ✗）。
if ! grep -q 'gates::limits::max_heartbeats()' "$META" \
   || ! grep -q 'gates::limits::max_rec_depth()' "$META" \
   || ! grep -q 'gates::limits::max_escalations()' "$META"; then
  gap_back "meta.rs 不再从登记表取预算 ⇒ 可配置没接上 ✗"
fi
# 旧的"拍脑袋"通道必须**不在** ✓（本地常量 / 名字做键的 `meta_limit` ✗）。
if grep -q 'fn meta_limit' "$META" || grep -q '^const DEFAULT_FUEL: u32' "$META" \
   || grep -q '^const MAX_DEPTH: u32' "$META"; then
  gap_back "meta.rs 里又出现了本地预算常量 / 旧取证口 ✗"
fi

echo "缺口已关（G-88 ✓）：预算旋钮**可配置** ✓ 且默认值 = **Lean 的数值** ✓"
echo "  ①「当成否」那半**保持已修** ✓（撞预算 ⇒ 加大预算重试 ✓ / 弃权 ✓ / 绝不 Tri::No ✗）"
echo "  ② 计数出口在 ✓ = sokonanoda::gates::{META_BUDGET_ESCALATED, META_BUDGET_EXHAUSTED}"
echo "  ③ 登记表 = crates/kernel/src/gates.rs::limits ✓（SOKO_LIMIT_MAX_HEARTBEATS /"
echo "     SOKO_LIMIT_MAX_REC_DEPTH / SOKO_LIMIT_MAX_ESCALATIONS ✓；非法值/缺省 ⇒ 默认值 ✓）"
echo "     默认值：20000（Lean synthInstance.maxHeartbeats ✓）· 512（Lean maxRecDepth ✓）· 6（无对应物 ✓）"
exit 1
