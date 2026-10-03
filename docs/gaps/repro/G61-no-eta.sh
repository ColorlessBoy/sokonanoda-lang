#!/usr/bin/env bash
# G-61 自断言复现：**没有 η**（单构造子归纳 `Box.get b = b` 判红）。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在（η 那条必须被拒）
#   1 = 行为变了（η 有了 ⇒ 回来关账，可以去做 `Setoid`/`Quotient`）
#   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-61: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① iota 对照组：消去子在构造子上**必须**算得出来
cat > "$WORK/a.sokonanoda" <<'EOF'
inductive Box (α : Type) : Type
ctor mk (a : α) : Box α
end
def Box.get (α : Type) (b : Box α) : α :=
  Box.rec.{1} α (fun (_ : Box α) => α) (fun (a : α) => a) b
theorem Box.get_mk (α : Type) (a : α) : Box.get α (Box.mk α a) = a := Eq.refl.{1} α a
EOF
a_out="$("$BIN" --json --no-project "$WORK/a.sokonanoda" 2>&1)"
if ! printf '%s' "$a_out" | grep -q '"name":"Box.get_mk","type":"decl.checked"'; then
  echo "G-61 ①：iota 对照组（Box.get_mk）也不过 —— 缺口描述失真" >&2
  printf '%s\n' "$a_out" | head -3 >&2
  exit 1
fi

# ② η 那条：必须仍被拒（报「期望 $1，实际是 (Box.[] $1)」）
cat > "$WORK/b.sokonanoda" <<'EOF'
inductive Box (α : Type) : Type
ctor mk (a : α) : Box α
end
def Box.get (α : Type) (b : Box α) : α :=
  Box.rec.{1} α (fun (_ : Box α) => α) (fun (a : α) => a) b
def eta_test (α : Type) (b : Box α) : Box.get α b = b := Eq.refl.{1} (Box α) b
EOF
b_out="$("$BIN" --json --no-project "$WORK/b.sokonanoda" 2>&1)"
# **G-49 之后这条期望值要放宽**（2026-10-03 ✓）：η 判红的 message 里原来带**裸 de Bruijn**
# （`Box.[] $1` ✗），而 G-49 把 `$N` 人话化成「第 N 个绑元」✓ ⇒ 旧的精确 grep **不再匹配** ✗
# ⇒ 该件在 CI 上误报"行为已变" ✗（`ledger (1)` 红 ✓）。**判据本身没变** ✓：η 仍必须**判红** ✓，
# 且 message 仍必须点到 `Box` ✓ ⇒ 改成"**两种写法都接受**" ✓（不再钉死内部编号的措辞 ✓）。
if ! printf '%s' "$b_out" | grep -qE 'Box\.\[\] (\$1|第 1 个绑元)'; then
  echo "G-61 ②：η 那条不再被判红了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi

echo 'G-61：缺口仍在（iota 正常 ✓，η 判红 ✗）'
exit 0
