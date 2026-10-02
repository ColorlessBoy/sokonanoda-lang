#!/usr/bin/env bash
# G-50 复现（**已修后的回归守卫**）：hover 的类型面漏点形式（`resolve_hovers` 的内核 pp 直出、不过显示层折叠）
#
# 为什么是 .sh（2026-10-02 用户要求 ✓）：台账原来这条的 `repro` 字段写的是
# 「测试名 + 散文」⇒ `gap.py` 读成 `missing` ⇒ **静默跳过**（计不进 bad）✗ ⇒
# 「自称已修」与「真验过」在输出里长得一模一样 ✗。现在把它变成**能机械重放的 .sh** ✓。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

command -v cargo >/dev/null 2>&1 || { echo "G-50: 需要 cargo（Rust 工具链）" >&2; exit 2; }

TEST=hover_types_are_folded_like_the_other_display_surfaces
out="$(timeout 1200 cargo test -p sokonanoda-front --lib "$TEST" --locked 2>&1)"
rc=$?

if [ "$rc" -eq 0 ]; then
  echo "✓ $TEST 通过（hover 的类型面已过显示层折叠）⇒ G-50 已修（行为已变）"
  exit 1
fi
echo "✗ $TEST 判红（rc=$rc）⇒ 缺口仍在 / 行为回退：hover 的类型面又不是折叠过的形式了" >&2
printf '%s\n' "$out" | grep -E "^test |panicked|assertion" | tail -5 >&2
exit 0
