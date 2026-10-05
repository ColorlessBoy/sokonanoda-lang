#!/usr/bin/env bash
# G-63 自断言复现：**`#check` 给用户的宇宙层建议是错的**（`Quot.lift` 渲成 `{A r B : Prop}`）。
#
# ⚠ **2026-10-05 第 97 棒改语义（值守裁决 ✓）**：原先的 `1 = .{1, 0} 也能过了` ✗ 是**错的** ——
# `.{1, 0}` **本来就该被拒**（`B := Type` ⇒ `Type : Sort 2` ⇒ `v` 该是 **2** ✓）
# ⇒ 那条约定与缺口的**真实语义相反** ✗（pp 修得再对也不会 exit 1 ✗）。
# **新约定（缺口的真实语义 = 渲染建议修好）** ✓：
#   1 = **修好了**：① `#check Quot.lift` 输出含**层参数形式**（`Sort u` / `Sort v` ✓）
#       ② `.{1, 0}` **仍被拒** ✓ ③ 对照组 `.{1, 2}` **仍过** ✓ —— 三条**同时**成立 ✓
#   0 = 缺口仍在（① 不成立 ⇒ 建议仍是 `Prop` ✗）
#   2 = 环境不满足
#
# 退出码约定（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 行为变了（修好了）· 2 = 环境不满足 ✓
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-63: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① 库里的已验证模板**必须**过（`Cardinal.lift` / `Cardinal.lift_mk`）
cat > "$WORK/a.sokonanoda" <<'EOF'
import lib.Cardinal
def c1 (β : Type) (f : Type → β) (h : ∀ (α₁ α₂ : Type), Type.Equiv α₁ α₂ → f α₁ = f α₂) : Cardinal → β :=
  Cardinal.lift β f h
theorem c2 (β : Type) (f : Type → β)
    (h : ∀ (α₁ α₂ : Type), Type.Equiv α₁ α₂ → f α₁ = f α₂) (α : Type) :
    Cardinal.lift β f h (Cardinal.mk α) = f α :=
  Cardinal.lift_mk β f h α
EOF
a_out="$("$BIN" --json --root courses/set-theory "$WORK/a.sokonanoda" 2>&1)"
if ! printf '%s' "$a_out" | grep -q '"name":"c2","type":"decl.checked"'; then
  echo "G-63 ①：库模板（Cardinal.lift_mk）也不过 —— 缺口描述失真" >&2
  printf '%s\n' "$a_out" | head -3 >&2
  exit 1
fi

# ② `#check` 渲染的 `.{1, 0}`（结果落 Type）**必须**仍被拒
cat > "$WORK/b.sokonanoda" <<'EOF'
def qr (α : Type) : α → α → Prop := fun (x : α) (y : α) => x = y
def Q (α : Type) : Type 0 := Quot.{1} α (qr α)
def liftToType (α : Type) (f : α → Type) (h : ∀ (x y : α), qr α x y → f x = f y) : Q α → Type :=
  Quot.lift.{1, 0} α (qr α) Type f h
EOF
b_out="$("$BIN" --json --no-project "$WORK/b.sokonanoda" 2>&1)"
if ! printf '%s' "$b_out" | grep -q 'kernel-rejected'; then
  echo "G-63 ②：\`.{1, 0}\` 不再被判红了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi

# ③ 对照组：`Quot.lift` **能**消去进 Type（写 `.{1, 2}`）—— 证明"只进 Prop"是错的
cat > "$WORK/c.sokonanoda" <<'EOF'
def qr (α : Type) : α → α → Prop := fun (x : α) (y : α) => x = y
def Q (α : Type) : Type 0 := Quot.{1} α (qr α)
def liftToType (α : Type) (f : α → Type) (h : ∀ (x y : α), qr α x y → f x = f y) : Q α → Type :=
  Quot.lift.{1, 2} α (qr α) Type f h
EOF
c_out="$("$BIN" --json --no-project "$WORK/c.sokonanoda" 2>&1)"
if ! printf '%s' "$c_out" | grep -q '"name":"liftToType","type":"decl.checked"'; then
  echo "G-63 ③：对照组（\`.{1, 2}\` 消去进 Type）也不过 —— 缺口描述失真" >&2
  printf '%s\n' "$c_out" | head -3 >&2
  exit 1
fi

# ④ **新语义的核心断言**（第 97 棒 ✓）：`#check Quot.lift` 必须渲染出**层参数形式** ✓
#    （修前是 `forall {A r B : Prop} …` ✗ —— 层全 0 ⇒ 给用户的建议错 ✓）。
cat > "$WORK/d.sokonanoda" <<'EOF'
#check Quot.lift
EOF
d_out="$("$BIN" --no-project "$WORK/d.sokonanoda" 2>&1)"
if printf '%s' "$d_out" | grep -q 'Sort u' && printf '%s' "$d_out" | grep -q 'Sort v'; then
  echo 'G-63：**已修** ✓（`#check Quot.lift` 渲染出层参数 `Sort u`/`Sort v` ✓；`.{1, 0}` 仍被拒 ✓；对照组 `.{1, 2}` 仍过 ✓）'
  exit 1
fi
echo 'G-63：缺口仍在（`#check` 渲染成 `Prop` ✗ —— 给用户的层建议错 ✓；`.{1, 0}` 判红 ✓；对照组 `.{1, 2}` 过 ✓）'
exit 0
