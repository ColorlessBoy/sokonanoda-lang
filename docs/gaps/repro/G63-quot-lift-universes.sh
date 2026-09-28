#!/usr/bin/env bash
# G-63 自断言复现：`Quot.lift` 的**显式宇宙实参对不上内核签名**。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在（`#check` 建议的 `.{1, 0}` 必须被拒，且至少一个组合必须能过）
#   1 = 行为变了（`.{1, 0}` 也能过了 ⇒ 回来关账）
#   2 = 环境不满足
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

echo 'G-63：缺口仍在（`#check` 的 `.{1, 0}` 判红 ✗；`Quot.lift` 本体可用 ✓ —— 是宇宙实参难对准）'
exit 0
