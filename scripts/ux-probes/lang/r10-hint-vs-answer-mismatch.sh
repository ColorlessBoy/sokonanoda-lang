#!/usr/bin/env bash
# r10-hint-vs-answer-mismatch.sh —— 练习 4 的 hint 与画布**自己写好的答案**不是同一条路
# 现象: unit01 第 77 行 hint 说「Set.mem_singleton_iff α x a 把 hx 变成等式 x = a；
#       再用 Eq.symm 与 Eq.subst 把这个等式搬进目标（本单元还没有 rw，所以**写成 exact 的实参**）」；
#       而同一题的画布答案（78-84 行，已判绿）用的是 `unfold Set.singleton Set.mem at hx`
#       + `apply Eq.subst …` + `apply Eq.symm …`——**完全没用 Set.mem_singleton_iff，
#       也不是 exact 的实参**。hint 没提 unfold，答案没提那个引理。
# 期望: hint 与画布示范一致（或 hint 明说「另有 unfold 路线，见下面示范」）。
# 判据: exit 0 = 问题仍在（hint 提到 Set.mem_singleton_iff/exact 而画布答案没有）/ exit 1 = 已一致
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
U="$REPO/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
hint=$(sed -n '77p' "$U")
body=$(sed -n '78,84p' "$U")
if grep -q 'Set.mem_singleton_iff' <<<"$hint" && ! grep -q 'Set.mem_singleton_iff' <<<"$body" \
   && grep -q 'exact 的实参' <<<"$hint" && grep -q 'unfold' <<<"$body" && ! grep -q 'unfold' <<<"$hint"; then
  echo "STILL REPRODUCES: hint 说的路（Set.mem_singleton_iff + exact 实参）与画布答案（unfold + apply）不一致"
  echo "hint : $hint"; echo "答案 :"; sed 's/^/  /' <<<"$body"; exit 0
fi
echo "FIXED: hint 与画布答案已一致"; exit 1
