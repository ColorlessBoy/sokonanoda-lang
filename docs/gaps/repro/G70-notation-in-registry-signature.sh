#!/usr/bin/env bash
# G-70：注册表 `signature` 里的**记法**让隐式插入整条不触发。
#
# `KnownName::Decl::signature` 存的是 `render_expr(ty)`，而 `implicit::telescope`
# 要用 `parse_expr_text`（**空记法表**）把它**回读**成望远镜 —— 记法节点回读不了
# （`∈` 是未声明符号）⇒ 解析失败 ⇒ 望远镜拿不到 ⇒ **隐式插入整条不触发** ✗
# ⇒ 实参落进隐式位：
#
#     theorem mem_univ {α : Type} (a : α) : a ∈ univ := …   -- 签名里带 `∈`
#     exact Set.mem_univ x
#     ⇒ 类型不匹配：期望 `Set.univ α x`，实际是 `(a : x) -> Set.mem a Set.univ`
#
# 修法：注册表 signature 走**判定路径**的去记法渲染（`decl_signature` =
# `render_expr(expand_notations(ty))`）⇒ 产物能被空记法表回读 ✓。
#
# 期望：缺口**仍在**时 exit 0；修好后 exit 1。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
BIN=./target/release/sokonanoda
[ -x "$BIN" ] || { echo "G-70：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }

DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
cat > "$DIR/lib.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α → Prop
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def univ {α : Type} : Set α := fun (x : α) => True
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
end Set
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
namespace Set
-- 签名里**带记法**（`a ∈ univ`）—— 这正是触发条件
theorem mem_univ {α : Type} (a : α) : a ∈ univ := True.intro
end Set
EOF
cat > "$DIR/main.sokonanoda" <<'EOF'
import lib

theorem t (α : Type) (A : Set α) : A ⊆ Set.univ α := by
  intro x
  intro hx
  exact Set.mem_univ x
EOF

out=$("$BIN" grade --root "$DIR" "$DIR/main.sokonanoda" 2>&1)
if printf '%s' "$out" | grep -q '"code"'; then
  echo "G-70 仍在：签名里的记法让望远镜回读失败 ⇒ 隐式插入不触发" >&2
  printf '%s' "$out" | grep -oE '"message":"[^"]{0,84}' | head -2 >&2
  exit 0
fi
echo "G-70 已修：签名去记法后可回读，隐式插入正常 ✓" >&2
exit 1
