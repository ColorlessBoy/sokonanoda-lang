#!/usr/bin/env bash
# r18-id-not-defined.sh —— `id` 不在 prelude 里
# 现象: `id (b ∈ A) (Eq.subst …)` 报 `unknown identifier `id``。
#       `id` 是 Lean core 最基本的函数，任何教程都会用；报错也没说「本子集没有 id，直接写项即可」。
# 期望: prelude 补上 `id`，或报错点名「本子集无 id，去掉它即可」。
# 判据: exit 0 = 问题仍在（unknown identifier `id`）/ exit 1 = 已有 id
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
F="$REPO/.cache/ux/lang/repro/.work/r18.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem uses_id (α : Type) (A : Set α) (a : α) (h : a ∈ A) : a ∈ A :=
  id (a ∈ A) h
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
if grep -q 'unknown identifier `id`' <<<"$out"; then
  echo "STILL REPRODUCES: prelude 里没有 id"; echo "$out" | head -c 300; echo; exit 0
fi
echo "FIXED: id 可用"; exit 1
