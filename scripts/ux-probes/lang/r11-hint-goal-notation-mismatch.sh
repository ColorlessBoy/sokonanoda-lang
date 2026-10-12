#!/usr/bin/env bash
# r11-hint-goal-notation-mismatch.sh —— hint 的「目标形态」用点名写法，state 用记法，对不上
# 现象: 练习 4 的 hint 第 76 行写「目标是 A x」；而 `query state` 在那一行给的是 `x ∈ A`。
#       两者是同一个命题的两种渲染，但初学者没法把 `A x` 与 `x ∈ A` 对上——
#       这正是用户抱怨「不知道当前目标长什么样」的来源。
# 期望: hint 的「目标形态」直接抄 state 的渲染（`x ∈ A`），或两种都写。
# 判据: exit 0 = 问题仍在（hint 写点名 `A x`、state 写 `x ∈ A`）/ exit 1 = 已统一
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
U="$REPO/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
hint=$(sed -n '76p' "$U")
st=$(timeout 300 "$REPO/scripts/soko" query state --compact --file "$U" --line 81 --col 3 2>&1)
goal=$(grep -o '"goal":"[^"]*"' <<<"$st" | head -1)
echo "hint 目标形态: $hint"; echo "state goal   : $goal"
if grep -q '目标是 A x' <<<"$hint" && grep -q 'x ∈ A' <<<"$goal"; then
  echo "STILL REPRODUCES: hint 用点名 `A x`，state 用记法 `x ∈ A`，两种渲染没打通"; exit 0
fi
echo "FIXED: hint 与 state 的渲染已一致"; exit 1
