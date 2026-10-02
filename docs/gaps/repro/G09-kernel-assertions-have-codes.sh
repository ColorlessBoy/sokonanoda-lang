#!/usr/bin/env bash
# G-09 复现（**已修后的回归守卫**）：内核断言以裸文本外泄（`left: 1 / right: 0`），hint 是通用「类型不匹配」
#
# 为什么是 .sh（2026-10-02 用户要求 ✓）：台账原来这条的 `repro` 字段写的是
# 「测试名 + 散文」⇒ `gap.py` 读成 `missing` ⇒ **静默跳过**（计不进 bad）✗ ⇒
# 「自称已修」与「真验过」在输出里长得一模一样 ✗。现在把它变成**能机械重放的 .sh** ✓。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

command -v cargo >/dev/null 2>&1 || { echo "G-09: 需要 cargo（Rust 工具链）" >&2; exit 2; }

failed=0
for TEST in refine_kernel_kind_internal_shapes_stay_kernel_internal \
            kernel_fine_grained_kinds_stage_as_kernel_with_codes; do
  out="$(timeout 1200 cargo test -p sokonanoda-front --lib "$TEST" --locked 2>&1)"
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "✗ $TEST 判红（rc=$rc）⇒ 缺口仍在 / 行为回退" >&2
    printf '%s\n' "$out" | grep -E "^test |panicked|assertion" | tail -4 >&2
    failed=1
  else
    echo "  ✓ $TEST"
  fi
done

if [ "$failed" -eq 0 ]; then
  echo "✓ 两条判据都过（裸断言已归类成稳定 code `kernel-internal` + 专属 hint）⇒ G-09 已修（行为已变）"
  exit 1
fi
exit 0
