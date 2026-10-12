#!/usr/bin/env bash
# r14-state-runs-bloat.sh —— `query state` 的输出被高亮数组撑爆，且没有精简视图
# 现象: 一个 6 个绑元的单目标状态，`query state` 出 **6228 字节**，其中绝大部分是
#       `ty_runs` / `goal_runs`（编辑器高亮用的逐段 kind）。`--compact` 只去掉空白，
#       **不删** runs ⇒ agent/初学者想只看「目标 + 假设」没有任何办法。
# 期望: 提供不带 runs 的视图（如 --no-runs / --goals-only），或默认精简、runs 走 opt-in。
# 判据: exit 0 = 问题仍在（--compact 输出仍含 ty_runs 且体积 > 2KB）/ exit 1 = 已有精简视图
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
F="$REPO/.cache/ux/lang/repro/.work/r14.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem t (α : Type) (A : Set α) (a : α) (h : a ∈ A) : {a} ⊆ A := by
  intro x hx
  unfold Set.singleton Set.mem at hx
  apply Eq.subst α (fun (y : α) => y ∈ A) a x
  sorry
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query state --compact --file "$F" --line 8 --col 3 2>&1)
n=${#out}
echo "compact 输出字节数: $n"
if [ "$n" -gt 2000 ] && grep -q 'ty_runs' <<<"$out"; then
  echo "STILL REPRODUCES: --compact 仍带 ty_runs，单目标状态 ${n} 字节，无精简视图"; exit 0
fi
echo "FIXED: 已有精简视图"; exit 1
