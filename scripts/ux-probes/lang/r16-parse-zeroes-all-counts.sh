#!/usr/bin/env bash
# r16-parse-zeroes-all-counts.sh —— 一处解析错误把全部计数清零，学习者失去所有进度信号
# 现象: 文件里只有一条声明写错（`·` 子弹），`query check` 返回
#       counts 全 0（decl_checked=0 / exercise_open=0 …），exit 1。
#       画布上另有 4 条已写对的练习，学习者**完全看不到**「哪些已经对了」，
#       也无法自测——只能全文件反复试。
# 期望: 解析错误时至少报告「已成功解析/判定的前 N 条声明」，或明说「因解析失败未判定任何声明」。
# 判据: exit 0 = 问题仍在（counts 全 0 且无任何提示）/ exit 1 = 已有部分进度
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
F="$REPO/.cache/ux/lang/repro/.work/r16.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem good_one (α : Type) (A : Set α) (a : α) (h : a ∈ A) : a ∈ A := h

theorem broken (p q : Prop) (hp : p) (hq : q) : p ∧ q := by
  constructor
  · exact hp
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
echo "$(grep -o '"decl_checked": [0-9]*' <<<"$out") $(grep -o '"exercise_open": [0-9]*' <<<"$out")"
if grep -q '"decl_checked": 0' <<<"$out" && ! grep -q 'good_one' <<<"$out"; then
  echo "STILL REPRODUCES: 一条坏声明让 good_one 也不可见，counts 全 0"; exit 0
fi
echo "FIXED: 解析失败时仍报出了已完成的部分"; exit 1
