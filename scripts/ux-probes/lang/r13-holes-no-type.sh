#!/usr/bin/env bash
# r13-holes-no-type.sh —— `query holes` 给不出洞的期望类型，且与 `query goals` 形状不一致
# 现象: 画布 unit01 有两个开放练习，`query holes` 返回
#       `{"decl":"singleton_subset_iff","id":"singleton_subset_iff:0","ty":null,…}`
#       —— `ty` 字段在、但恒为 null；`query goals --probe` 里 holes[] 干脆**没有** ty 键。
#       `query goals` 的 goals[] 里**有**目标（`(({a}) ⊆ A) ↔ (a ∈ A)`），
#       但按 hole 取不到 ⇒ 学习者/agent 问「我这个洞要证什么」时拿不到答案。
# 期望: holes[].ty 填上期望类型（--probe 时至少填上），两个 op 的形状一致。
# 判据: exit 0 = 问题仍在（ty 为 null 或缺键）/ exit 1 = 已填
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
h=$(timeout 300 "$REPO/scripts/soko" query holes --file "$U" 2>&1)
g=$(timeout 300 "$REPO/scripts/soko" query goals --probe --file "$U" 2>&1)
echo "query holes  ty: $(grep -o '"ty": *null' <<<"$h" | head -1)"
echo "query goals  holes 有 ty 键: $(grep -c '"holes": \[{[^}]*"ty"' <<<"$g")"
if grep -q '"ty": *null' <<<"$h" && ! grep -q '"holes": \[{[^}]*"ty"' <<<"$g"; then
  echo "STILL REPRODUCES: 洞的期望类型拿不到（holes.ty=null / goals.holes 无 ty 键）"; exit 0
fi
echo "FIXED: 洞的期望类型已可读"; exit 1
