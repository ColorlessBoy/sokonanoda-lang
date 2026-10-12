#!/usr/bin/env bash
# r23-no-subst-wrapper-in-unit01.sh —— 单元① 让学习者手写底层 Eq.subst，而具名包装在 lib.Rel 里
# 现象: 用户被迫写 `Eq.subst α (fun (y : α) => y ∈ A) a x`（画布 82 行、解答也是）。
#       lib/Rel.sokonanoda:96-100 明确写着「本语言里 Eq.subst 嵌在别的应用里会被
#       elaborator 判红」并因此提供具名包装 rel_subst_left / rel_subst_right。
#       但 unit01 只 `import lib.Logic` + `import lib.Set`，**拿不到这两个包装**，
#       hint（第 77 行）也只说「用 Eq.symm 与 Eq.subst」，没提包装或该 import 什么。
# 期望: 单元① 的 import 集里就有搬运引理（或 hint 明说要 import lib.Rel 并用包装）。
# 判据: exit 0 = 问题仍在（unit01 未 import lib.Rel 且 hint 只提裸 Eq.subst）/ exit 1 = 已提供
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
U="$REPO/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
echo "unit01 imports: $(grep -o '^import .*' "$U" | tr '\n' ' ')"
echo "hint 77: $(sed -n '77p' "$U")"
if ! grep -q '^import lib.Rel' "$U" && grep -q 'Eq.subst' <<<"$(sed -n '77p' "$U")" \
   && ! grep -qE 'rel_subst' <<<"$(sed -n '77p' "$U")"; then
  echo "STILL REPRODUCES: 单元① 拿不到搬运包装，hint 只指裸 Eq.subst"; exit 0
fi
echo "FIXED: 已提供具名搬运引理"; exit 1
