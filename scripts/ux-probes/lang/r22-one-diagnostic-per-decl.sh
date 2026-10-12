#!/usr/bin/env bash
# r22-one-diagnostic-per-decl.sh —— 一条声明只报第一个错，修一处要重跑一次
# 现象: 同一条声明里写两处 `exact hh`，只报**第一处**（L14），第二处（L15）不报。
#       初学者一处一处修，每修一次都要重跑判卷（画布 ~100 行，实测每次数秒）。
# 期望: 一条声明里的多个独立错误一次报全（或明说「本条只报第一个错」）。
# 判据: exit 0 = 问题仍在（两处错只报一条）/ exit 1 = 已报全
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
F="$REPO/.cache/ux/lang/repro/.work/r22.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem two_errors (α : Type) (A : Set α) (a : α) (h : a ∈ A) : a ∈ A := by
  exact hh
  exact hh
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
n=$(grep -o '"code": "elab-tactic-failed"' <<<"$out" | wc -l | tr -d ' ')
echo "diagnostic 条数: $n"
if [ "$n" -le 1 ]; then
  echo "STILL REPRODUCES: 两处独立错误只报了 $n 条"; exit 0
fi
echo "FIXED: 一次报全"; exit 1
