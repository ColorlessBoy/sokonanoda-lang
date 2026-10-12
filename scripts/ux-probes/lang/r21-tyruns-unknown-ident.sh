#!/usr/bin/env bash
# r21-tyruns-unknown-ident.sh —— 高亮元数据把绑定变量标成 unknown_ident
# 现象: `query goals` 的 ty_runs 里，绑定变量 `α` / `A` / `B` 的 kind 是 `unknown_ident`
#       （见 unit01 的 demo_subset_def：`{"kind":"unknown_ident","text":"α"}`）。
#       这是编辑器高亮用的 wire 字段；标错会让变量在编辑器里显示成「未定义标识符」色，
#       正好与学习者「我是不是名字写错了」的焦虑叠加。
# 期望: 绑定变量标 `binder`（同文件其它位置就是 `binder`），不是 `unknown_ident`。
# 判据: exit 0 = 问题仍在（ty_runs 里绑定变量为 unknown_ident）/ exit 1 = 已修
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
out=$(timeout 300 "$REPO/scripts/soko" query goals --file "$U" 2>&1)
if grep -q '"kind": "unknown_ident"' <<<"$out"; then
  n=$(grep -o '"kind": "unknown_ident"' <<<"$out" | wc -l | tr -d ' ')
  echo "STILL REPRODUCES: ty_runs 里有 $n 处 unknown_ident（含绑定变量）"; exit 0
fi
echo "FIXED: 绑定变量不再标成 unknown_ident"; exit 1
