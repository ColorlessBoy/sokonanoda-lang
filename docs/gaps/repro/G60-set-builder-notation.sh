#!/usr/bin/env bash
# G-60 自断言复现：**集合建构式 `{x ∈ A | P x}` / `{x : α | P x}` 写不出来**。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在（两种写法都必须被拒）
#   1 = 行为变了（能写了 ⇒ 回来关账、把 ST3 的记法那一半划掉）
#   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-60: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① `{x | P x}`：无类型版 —— 必须被拒（set-literal-shape）
cat > "$WORK/a.sokonanoda" <<'EOF'
import lib.Set
def t (α : Type) (P : α → Prop) : Set α := {x | P x}
EOF
a_out="$("$BIN" --json --root courses/set-theory "$WORK/a.sokonanoda" 2>&1)"
if ! printf '%s' "$a_out" | grep -q 'set-literal-shape'; then
  echo "G-60 ①：\`{x | P x}\` 不再被拒了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$a_out" | head -3 >&2
  exit 1
fi

# ② `{x : α | x ∈ A}`：带类型版 —— 必须被拒（`|` 处 parse 错）
cat > "$WORK/b.sokonanoda" <<'EOF'
import lib.Set
def t2 (α : Type) (A : Set α) : Set α := {x : α | x ∈ A}
EOF
b_out="$("$BIN" --json --root courses/set-theory "$WORK/b.sokonanoda" 2>&1)"
if ! printf '%s' "$b_out" | grep -q 'found Pipe'; then
  echo "G-60 ②：\`{x : α | x ∈ A}\` 不再在 \`|\` 处报错了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi

# ③ 对照组：点名写法**必须能用**（`Set.sep` 已落库）
cat > "$WORK/c.sokonanoda" <<'EOF'
import lib.Set
def t3 (α : Type) (A : Set α) (P : α → Prop) : Set α := Set.sep α A P
EOF
c_out="$("$BIN" --json --root courses/set-theory "$WORK/c.sokonanoda" 2>&1)"
if ! printf '%s' "$c_out" | grep -q '"type":"decl.checked"'; then
  echo "G-60 ③：对照组（点名 \`Set.sep\`）也不过了 —— 缺口描述失真" >&2
  printf '%s\n' "$c_out" | head -3 >&2
  exit 1
fi

echo 'G-60：缺口仍在（`{x | P x}` 与 `{x : α | x ∈ A}` 都被拒；点名 `Set.sep` 正常）'
exit 0
