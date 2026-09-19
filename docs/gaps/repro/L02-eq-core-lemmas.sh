#!/usr/bin/env bash
# L-02 自断言复现：prelude 缺 Eq 的核心引理（`Eq.symm` / `Eq.trans` / `congrArg`）
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在 · 1 = 已修（修后形状成立）⇒ 写 fixed_in · 2 = 环境不满足
#
# 台账 `today`（0.58.0 实测，修前）：
#   `Eq.symm.{1} Nat a b h` → `unknown constant \`Eq.symm\``（exit 1）。
#   课程侧只能在 `courses/set-theory/lib/Logic.sokonanoda` 手写三条兜底。
#
# 修后契约（设计 docs/design/prelude-l1-proposal.md §1.1 第 26–28 行）：
#   ① Full：`Eq.symm`/`Eq.trans`/`congrArg` 直接可用 → exit 0 且 checked；
#   ② 它们**不是公理**：值位真的过内核（`Eq.subst` 是它们的定义体来源）；
#   ③ B7 依赖 Eq prelude：文件自己声明 `Eq` ⇒ 整块（含三条引理）让位；
#   ④ Bare 下三者都不在（与 ① 对照，证明来源确实是 prelude）。
# 任一段不满足 ⇒ exit 0（缺口仍在）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/eq.sokonanoda" <<'EOF'
def eq_symm_probe (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h
def eq_trans_probe (a b c : Nat) (h1 : Eq.{1} Nat a b) (h2 : Eq.{1} Nat b c) :
    Eq.{1} Nat a c := Eq.trans.{1} Nat a b c h1 h2
def congr_arg_probe (f : Nat -> Nat) (a b : Nat) (h : Eq.{1} Nat a b) :
    Eq.{1} Nat (f a) (f b) := congrArg.{1} Nat Nat f a b h
EOF

echo "== ① Full：Eq 三引理必须判卷通过 =="
F="$( node "$SOKO" --json "$TMP/eq.sokonanoda" 2>&1 )"
printf '%s\n' "$F" | tail -1
f_checked="$( printf '%s' "$F" | grep -c '"type":"decl.checked"' )"
f_ok=1
[ "$f_checked" = 3 ] && f_ok=0
echo "   → decl.checked = ${f_checked}（修后预期 3）"
if [ "$f_ok" != 0 ]; then
  printf '%s' "$F" | grep -o 'unknown [a-z]*[^"]*' | sort -u | head -5
fi
echo

echo "== ② B7 依赖 Eq prelude：文件自己声明 Eq ⇒ 三条引理一起让位 =="
cat > "$TMP/own-eq.sokonanoda" <<'EOF'
axiom Eq {u} : {α : Sort u} -> α -> α -> Prop
def eq_symm_probe (a b : Nat) (h : Eq.{1} Nat a b) : Eq.{1} Nat b a := Eq.symm.{1} Nat a b h
EOF
O="$( node "$SOKO" --json "$TMP/own-eq.sokonanoda" 2>&1 )"
o_ok=1
printf '%s' "$O" | grep -q 'unknown constant `Eq.symm`' && o_ok=0
echo "   → Eq.symm 随 Eq 一起让位（修后预期 yes）：$([ "$o_ok" = 0 ] && echo yes || echo NO)"
echo

echo "== ③ Bare：三条引理都不在（对照，证明来源是 prelude）=="
B="$( node "$SOKO" --bare "$TMP/eq.sokonanoda" 2>&1 )"
b_ok=1
printf '%s' "$B" | grep -q 'error\[elab-unknown' && b_ok=0
echo "   → Bare 下未知（修后预期 yes）：$([ "$b_ok" = 0 ] && echo yes || echo NO)"
echo

if [ "$f_ok" = 0 ] && [ "$o_ok" = 0 ] && [ "$b_ok" = 0 ]; then
  echo "结论：L-02 已修（Eq.symm/Eq.trans/congrArg 进 prelude，B7 随 Eq 让位）⇒ 行为已变，更新台账。" >&2
  exit 1
fi
echo "结论：缺口仍在（prelude 没有 Eq 的核心引理）。" >&2
exit 0
