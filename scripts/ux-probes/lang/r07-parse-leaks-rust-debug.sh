#!/usr/bin/env bash
# r07-parse-leaks-rust-debug.sh —— 解析错误把 Rust 内部 Debug 结构打给用户
# 现象: 学习者写常见 Lean 语法（`·` 子弹 / `intros` / `simp`），得到的是
#       `expected a .sokonanoda command, found Token { kind: Ident("·"),
#        span: Span { start: Pos { offset: 148, line: 6, column: 3 }, end: … } }`
#       —— `Token`/`Span`/`Pos`/`offset` 是编译器内部结构，用户看不懂，
#       而且没有一句「`·` 在本子集里不支持，请改用 …」。
# 期望: 人话（点名不支持的记号 + 替代写法），不要 Debug dump。
# 判据: exit 0 = 问题仍在（报错含 `Token {` 或 `Span {`）/ exit 1 = 已改人话
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
F="$REPO/.cache/ux/lang/repro/.work/r07.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem t (p q : Prop) (hp : p) (hq : q) : p ∧ q := by
  constructor
  · exact hp
  · exact hq
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
if grep -q 'Token {' <<<"$out" || grep -q 'Span {' <<<"$out"; then
  echo "STILL REPRODUCES: 解析诊断里仍是 Rust Debug 结构"; echo "$out" | head -c 400; echo; exit 0
fi
echo "FIXED: 解析诊断已是人话"; echo "$out" | head -c 400; exit 1
