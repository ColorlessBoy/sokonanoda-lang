#!/usr/bin/env bash
# L-05 复现（**已修后的回归守卫**）：「内容型」集合引理被误放进库（16 条）—— 它们应当是**练习**而不是基础设施
#
# 为什么是 .sh（2026-10-02 用户要求 ✓）：台账原来这条的 `repro` 字段写的是
# 「测试名 + 散文」⇒ `gap.py` 读成 `missing` ⇒ **静默跳过**（计不进 bad）✗ ⇒
# 「自称已修」与「真验过」在输出里长得一模一样 ✗。现在把它变成**能机械重放的 .sh** ✓。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

# 判据：这 16 条**有数学内容**的引理必须**不在课程库里**（在 = 缺口回来了 ✗）。
BANNED="subset_refl subset_trans subset_antisymm subset_antisymm_iff empty_subset
subset_empty_iff singleton_subset_iff subset_union_left subset_union_right
union_subset_iff inter_subset_left inter_subset_right subset_inter subset_inter_iff
diff_subset power_mono union_comm inter_comm"

libs=$(ls courses/set-theory/lib/*.sokonanoda 2>/dev/null)
if [ -z "$libs" ]; then
  echo "L-05: 找不到课程库（courses/set-theory/lib/*.sokonanoda）⇒ 环境不满足" >&2
  exit 2
fi

hits=""
for name in $BANNED; do
  # 声明形状：`theorem subset_refl …` / `theorem Set.subset_refl …`（前缀可有可无）
  if grep -qE "^[[:space:]]*(theorem|def|lemma|axiom)[[:space:]]+([A-Za-z_][A-Za-z0-9_]*\.)?$name\b" $libs; then
    hits="$hits $name"
  fi
done

if [ -n "$hits" ]; then
  echo "✗ 内容型引理又回到库里了：$hits ⇒ 缺口仍在（它们应当是练习）" >&2
  exit 0
fi
echo "✓ 16 条内容型引理都不在课程库里（库只剩定义/公理/纯展开引理）⇒ L-05 已修（行为已变）"
exit 1
