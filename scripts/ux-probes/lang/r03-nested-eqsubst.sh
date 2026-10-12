#!/usr/bin/env bash
# r03-nested-eqsubst.sh —— `Eq.subst` 嵌进实参位被**静默错误 elaborate**（用户原话的核心）
# 现象: 顶层 `Eq.subst.{1} α (fun y => y ∈ A) a b h ha` 判绿；同一项一旦嵌进
#       `And.intro … (…)` 的实参位，就产出**类型错误**的项（`a ∈ A` 而非 `b ∈ A`），
#       再被内核以 de Bruijn 黑话拒绝。lib/Rel.sokonanoda:96-100 自己记着这是已知边界
#       （与 G-72 同族，等语言线修）。用户原话：「Eq.subst α (fun (y : α) => y ∈ A) a x
#       基本上自动推导全坏了」。
# 期望: 要么正确 elaborate，要么报一条**可读**的诊断并指出这是已知边界 + 绕法
#       （lib/Rel 的 rel_subst_left/right）——而单元① 根本没 import lib.Rel。
# 判据: exit 0 = 问题仍在（顶层绿 + 嵌套红）/ exit 1 = 已不复现
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
F="$REPO/.cache/ux/lang/repro/.work/r03.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem t_toplevel (α : Type) (A : Set α) (a b : α) (h : a = b) (ha : a ∈ A) :
    b ∈ A :=
  Eq.subst.{1} α (fun (y : α) => y ∈ A) a b h ha

theorem t_nested (α : Type) (A : Set α) (a b : α) (h : a = b) (ha : a ∈ A) :
    (b ∈ A) ∧ (b ∈ A) :=
  And.intro (b ∈ A) (b ∈ A)
    (Eq.subst.{1} α (fun (y : α) => y ∈ A) a b h ha)
    ha
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1); rc=$?
top=$(grep -c 't_toplevel' <<<"$out" || true)
if [ "$rc" -ne 0 ] && grep -q '个绑元' <<<"$out"; then
  echo "STILL REPRODUCES: 顶层可判、嵌套被静默错误 elaborate，且报错是内核黑话"
  echo "$out" | head -c 500; echo; exit 0
fi
echo "FIXED（或报错已可读）"; echo "$out" | head -c 500; exit 1
