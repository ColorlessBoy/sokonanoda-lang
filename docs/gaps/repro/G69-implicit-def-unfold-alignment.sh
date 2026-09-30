#!/usr/bin/env bash
# G-69：**delta 展开的实参对齐不认前导隐式前缀** ⇒ `intro` 派生的假设（点形式）
# 展开出胡说八道的类型。
#
# 背景：B2（课程库隐式化）把库签名从 `(α : Type)` 改成 `{α : Type}` 之后，**短写**
# （`And.left hx`）要在"假设的类型"上反解隐式参数，而假设的类型是 `intro` **派生**的
# ——它写成点形式 `Set.mem x (Set.inter A B)`（**没有**前导隐式实参，源级 AST 里只有
# 写出来的 2 个实参）。`solve_prefix` 的路线 ① 要 `unfold_to_inductive` 把它展开到
# `And` 归纳头，而 `spine.rs::unfold_one` 把实参**右对齐到 `DefInfo.params`（含隐式）**：
#
#     Set.mem 的形参 = [α, a, A]（3），实参 = [x, Set.inter A B]（2）
#     ⇒ offset = 1 ⇒ a := x, A := Set.inter A B ⇒ 体 `A a` ⇒ `(Set.inter A B) x` ✓
#     再展开一层：Set.inter 的形参 = [α, A, B]（3），实参 = [A, B, x]（3）
#     ⇒ α := A, A := B, B := x ⇒ **胡说八道** ✗
#
# ⇒ 路线 ① 解不出 ⇒ `elab-implicit-argument-unsolved` ⇒ `exact And.left hx` 判红 ✗。
# **对照**（同样 10 行、只差假设是**显式 binder** 还是 `intro` 派生）：显式 binder
# （`(hx : A x ∧ B x)`）**判绿** ✓ ⇒ 触发条件是「假设的类型是**点形式的应用**」✓。
#
# 期望：缺口**仍在**时 exit 0；修好后 exit 1。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
BIN=./target/release/sokonanoda
[ -x "$BIN" ] || { echo "G-69：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }

DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
cat > "$DIR/lib.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α → Prop
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def inter {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∧ B x
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
end Set
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
infixl:70 " ∩ " => Set.inter
EOF
# ① 派生假设（`intro hx`）⇒ 应当判红（缺口仍在）
cat > "$DIR/derived.sokonanoda" <<'EOF'
import lib

theorem t (α : Type) (A B : Set α) : A ∩ B ⊆ A := by
  intro x
  intro hx
  exact And.left hx
EOF
# ② 显式 binder ⇒ 对照组，必须判绿
cat > "$DIR/explicit.sokonanoda" <<'EOF'
import lib

theorem t (α : Type) (A B : Set α) (x : α) (hx : A x ∧ B x) : A x :=
  And.left hx
EOF

bad=$("$BIN" grade --root "$DIR" "$DIR/derived.sokonanoda" 2>&1 | grep -c '"code"')
good=$("$BIN" grade --root "$DIR" "$DIR/explicit.sokonanoda" 2>&1 | grep -c '"code"')
if [ "$bad" != "0" ] && [ "$good" = "0" ]; then
  echo "G-69 仍在：派生假设的点形式展开错位 ⇒ 短写解不出隐式参数" >&2
  exit 0
fi
if [ "$bad" = "0" ] && [ "$good" = "0" ]; then
  echo "G-69 已修：派生假设也能解出隐式参数 ✓" >&2
  exit 1
fi
echo "G-69：形状异常（derived=$bad explicit=$good）" >&2
exit 2
