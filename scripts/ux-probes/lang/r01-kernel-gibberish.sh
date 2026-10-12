#!/usr/bin/env bash
# r01-kernel-gibberish.sh —— kernel-rejected 的报错是内核 de Bruijn 黑话（不可读）
# 现象: 初学者最朴素的一类错误（h 证的是 a ∈ A，目标却要 b ∈ A），报错把**整条声明**
#       的类型用内核内部记号打出来：「第 N 个绑元（α）」「Set.[]」「Set.mem.[]」
#       「Pi ( : 第 4 个绑元（α）)」。两边各 418 字符，唯一差别藏在一个 token 里。
# 期望: 渲染成 `期望 b ∈ A，实际 a ∈ A`（可读名字 + 记法），并把 span 指到出错表达式。
# 判据: exit 0 = 问题仍在（报错含「个绑元」）/ exit 1 = 已不复现
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
F="$REPO/.cache/ux/lang/repro/.work/r01.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem simple_mismatch (α : Type) (A : Set α) (a b : α) (ha : a ∈ A) :
    b ∈ A :=
  ha
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
if grep -q '个绑元' <<<"$out"; then
  echo "STILL REPRODUCES: kernel-rejected 报错仍是内核 de Bruijn 黑话"
  echo "$out" | head -c 500; echo; exit 0
fi
echo "FIXED: 报错已渲染成可读形式"; echo "$out" | head -c 500; exit 1
