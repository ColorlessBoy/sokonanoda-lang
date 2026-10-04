#!/usr/bin/env bash
# **G-93**：`Eq.refl` 形状的证明**一带 `import` 就被误拒** ✗
# —— **与 G-72 同一族**（「同一个模块，单文件判绿、被 `import` 时判红」✗），
#    是 G-72 那次修复**没覆盖到的另一个形状** ✗。
#
# 退出码（`docs/gaps/README.md` 的约定 ✓）：
#   0 = 缺口仍在（**两态不一致** ✗：单文件判绿 ✓、带 `import` 判红 ✗）
#   1 = 行为变了（两态一致 ⇒ 修好了 ⇒ 回来更新台账 ✓）
#   2 = 环境不对
#
# ## 实测（2026-10-04 第 17 棒 · release · 两个二进制都复现 ⇒ **既有** ✓ · 确定性 ✓）
#
# | 声明（都在同一个模块根下） | 单文件 | 带 `import Dep` |
# |---|---|---|
# | `theorem t (α : Type) (a : α) : Eq α a a := Eq.refl α a` | **绿** ✓ | **红** ✗ |
# | `theorem t (α : Type) (f : α → α) (a : α) : Eq (f a) (f a) := Eq.refl α (f a)` | **绿** ✓ | **红** ✗ |
# | `theorem t (P : Prop) : P → P := fun h => h` | 绿 ✓ | 绿 ✓ |
# | `theorem t (P : Prop) (h : P) : P := h` | 绿 ✓ | 绿 ✓ |
# | `theorem t (P : Prop) (h : P) : P ∧ P := And.intro h h` | 绿 ✓ | 绿 ✓ |
#
# ⇒ **触发条件是 `Eq` / `Eq.refl` 本身** ✓（其余形状两态都对 ✓）；
# 且**与依赖内容无关** ✓（`Dep` 只放一个 `def id2 …` ✓；把 `Dep` 换成任何能编过的模块都一样 ✓）。
#
# ⚠ **为什么这条重要** ✗：`Eq.refl` 是**课程里最常见的证明** ✓ ——
# 学习者在**任何带 `import` 的文件**里写 `:= Eq.refl α a` 都会被判红 ✗，
# 而把同一条声明**单独存一个文件**又判绿 ✓ ⇒ 两态不一致本身就是最硬的证据 ✓。
#
# ## 判据为什么是**两态**（照 G-72 的 `why_open` ✓）
#
# G-72 的 `why_open` 原话：「修的时候**必须两态各留一条判据**（同文件 + import），
# 否则又会漏掉「库自己绿」这一半 ✗」⇒ 本复现件**两个相位都断言** ✓。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

BIN="${SOKO_BIN:-}"
if [ -z "$BIN" ]; then
  for cand in "$ROOT/target/release/sokonanoda" "$ROOT/target/debug/sokonanoda"; do
    [ -x "$cand" ] && BIN="$cand" && break
  done
fi
[ -n "$BIN" ] && [ -x "$BIN" ] || { echo "环境不对：找不到 sokonanoda 二进制" >&2; exit 2; }

DIR="$(mktemp -d "${TMPDIR:-/tmp}/soko-g93-XXXXXX")"
trap 'rm -rf "$DIR"' EXIT
printf '[project]\nname = "g93"\n' > "$DIR/sokonanoda.toml"
printf 'def id2 (α : Type) (a : α) : α := a\n' > "$DIR/Dep.sokonanoda"

DECL='theorem t (α : Type) (f : α → α) (a : α) : Eq (f a) (f a) := Eq.refl α (f a)'

# **相位 A**：同一条声明，**单文件**（无 import）⇒ 必须判绿 ✓
printf '%s\n' "$DECL" > "$DIR/Alone.sokonanoda"
rm -rf "$DIR/cA"
a_out="$(SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_CACHE_DIR="$DIR/cA" \
  timeout 120 "$BIN" build "$DIR/Alone.sokonanoda" 2>&1 | tail -1)"
a_ok=0
case "$a_out" in *"0 failed"*) a_ok=1 ;; esac

# **相位 B**：**同一条声明 + `import`** ⇒ 现在判红 ✗
printf 'import Dep\n%s\n' "$DECL" > "$DIR/WithImport.sokonanoda"
rm -rf "$DIR/cB"
b_out="$(SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_CACHE_DIR="$DIR/cB" \
  timeout 120 "$BIN" build "$DIR/WithImport.sokonanoda" 2>&1 | tail -1)"
b_ok=0
case "$b_out" in *"0 failed"*) b_ok=1 ;; esac

echo "G-93 两态读数："
echo "  相位 A（单文件 · 无 import）: $a_out"
echo "  相位 B（同一条 + import）  : $b_out"

if [ "$a_ok" = "1" ] && [ "$b_ok" = "0" ]; then
  echo "缺口仍在：G-93「单文件判绿、带 import 判红」✗（与 G-72 同族 ✓）"
  echo "  触发条件 = \`Eq\`/\`Eq.refl\` 本身 ✓（其余形状两态都对 ✓）· 与依赖内容无关 ✓"
  echo "  判据 = **两态各一条** ✓（G-72 的 why_open 原话 ✓）"
  exit 0
fi
if [ "$a_ok" = "1" ] && [ "$b_ok" = "1" ]; then
  echo "行为变了：两态**一致**（都绿 ✓）⇒ 这条修好了 ⇒ 回来更新台账 ✓"
  exit 1
fi
echo "环境/形状异常：相位 A=$a_ok 相位 B=$b_ok（相位 A 必须是绿 ✓ —— 它红了说明夹具坏了 ✗）" >&2
exit 2
