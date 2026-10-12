#!/usr/bin/env bash
# r14-state-runs-bloat.sh —— `query state` 的输出被高亮数组撑爆，且没有精简视图
# 现象: 一个 6 个绑元的单目标状态，`query state` 出 **6228 字节**，其中绝大部分是
#       `ty_runs` / `goal_runs`（编辑器高亮用的逐段 kind）。`--compact` 只去掉空白，
#       **不删** runs ⇒ agent/初学者想只看「目标 + 假设」没有任何办法。
# 期望: 提供不带 runs 的视图（如 --no-runs / --goals-only），或默认精简、runs 走 opt-in。
# 判据: exit 0 = 问题仍在（--compact 输出仍含 ty_runs 且体积 > 2KB）/ exit 1 = 已有精简视图
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# ⚠ **M-07：干净缓存纪律**（2026-10-12 实测咬过一次）—— 内建 prelude / 判定逻辑改了以后
# **只重建不清缓存**时，旧条目会继续答旧行为（同一二进制无 import 时 `#print And.left` = theorem、
# `import lib.Set` 时却 = def）⇒ 每条探针都必须在**新 mktemp 缓存 + 关两处产物**下取读数，
# 否则「缺口仍在」可能是**旧答案**（r04 就这么假红过一次：`apply` 结论校验已修好、探针仍报 exit 0）。
FRESH_CACHE=$(mktemp -d)
export SOKONANODA_CACHE_DIR="$FRESH_CACHE"
export SOKONANODA_NO_PROJECT_ARTIFACTS=1
export SOKONANODA_NO_MODULE_ARTIFACTS=1
trap 'rm -rf "$FRESH_CACHE"' EXIT
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
