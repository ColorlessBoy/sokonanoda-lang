#!/usr/bin/env bash
# r20-goal-extra-parens.sh —— 目标渲染多出一层括号，与源码写法对不上
# 现象: 源码写 `{a} ⊆ A`，`query state` 渲染成 `({a}) ⊆ A`（多一层括号）；
#       `↔` 目标渲染成 `(({a}) ⊆ A) ↔ (a ∈ A)`。初学者拿 state 的字符串去对照
#       源码/画布时会以为不是同一个命题。
# 期望: 渲染与源码写法一致（`{a} ⊆ A`），或渲染风格在文档里说明。
# 判据: exit 0 = 问题仍在（goal 里出现 `({a})`）/ exit 1 = 已一致
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
U="$REPO/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
st=$(timeout 300 "$REPO/scripts/soko" query state --compact --file "$U" --line 80 --col 3 2>&1)
goal=$(grep -o '"goal":"[^"]*"' <<<"$st" | head -1)
echo "state goal: $goal"
if grep -q '({a})' <<<"$goal"; then
  echo "STILL REPRODUCES: 渲染比源码多一层括号"; exit 0
fi
echo "FIXED: 渲染与源码一致"; exit 1
