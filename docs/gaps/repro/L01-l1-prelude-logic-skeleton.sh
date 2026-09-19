#!/usr/bin/env bash
# L-01 自断言复现：prelude 缺 Lean core 的逻辑骨架
#   （True / True.intro / False / False.rec / False.elim / And.* / Or.* /
#    Not.* / absurd / Iff.*）
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（L1 没进 prelude）
#   1 = 已修（**修后形状**成立）⇒ 回来更新台账（写 fixed_in）
#   2 = 环境不满足
#
# 台账 `today`（0.58.0 实测，修前）：
#   `def … := And.intro …` → `unknown identifier \`And\``（diagnostic，exit 1）；
#   `Or` / `Iff` / `Not` / `Eq.symm` 同病。课程侧只能靠
#   `courses/set-theory/lib/Logic.sokonanoda` 手写 26 条兜底（那是绕行，不是修）。
#
# 修后契约（设计 docs/design/prelude-l1-proposal.md §1.1/§2.2/§3.2）：
#   ① Full（默认）：只用 L1 名字的文件 → exit 0 且每条都 `decl.checked`；
#   ② Bare（`--bare`）：同一文件 → 非零退出 + `error[elab-unknown-identifier]:`
#      （Bare 是「从零构造一切」的课程模式，L1 绝不能溜进去）；
#   ③ 让位是**族粒度 + 依赖闭包**（依赖边按设计 §2.2 的表：B6 用 `And.left`、
#      B5 用 `False.elim`、B7 用 `Eq.subst`）：文件自己声明 `And`（B3）⇒
#      依赖它的 `Iff.*`（B6）一起不装（`Iff.mpr` 必须 unknown），而独立的
#      `Or.elim`（B4）仍在；
#   ④ `And`/`Or` 是**真归纳块**：`match h with | Or.inl a => …` 与裸模式
#      `| inl a => …` 都必须过（钉住 `InductiveTable` 注册）。
# 任一段不满足 ⇒ echo 缺口还在 + **exit 0**（与「缺口仍在」的语义一致）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/l1.sokonanoda" <<'EOF'
def l1_and (a b : Prop) (ha : a) (hb : b) : And b a := And.intro b a hb ha
def l1_or (a b c : Prop) (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.elim a b c f g h
def l1_iff (a b : Prop) (h : Iff a b) : b -> a := Iff.mpr a b h
def l1_absurd (a b : Prop) (ha : a) (hna : Not a) : b := absurd a b ha hna
def l1_true : True := True.intro
def l1_false_elim (c : Prop) (h : False) : c := False.elim c h
def l1_iff_refl (a : Prop) : Iff a a := Iff.refl a
EOF

# ① Full：L1 必须全部 checked。
echo "== ① Full（默认）：只用 L1 名字的文件必须判卷通过 =="
F="$( node "$SOKO" --json "$TMP/l1.sokonanoda" 2>&1 )"
printf '%s\n' "$F" | tail -1
f_checked="$( printf '%s' "$F" | grep -c '"type":"decl.checked"' )"
f_diag="$( printf '%s' "$F" | grep -c '"type":"diagnostic"' )"
f_ok=1
[ "$f_checked" = 7 ] && [ "$f_diag" = 0 ] && f_ok=0
echo "   → decl.checked = ${f_checked}（修后预期 7）· diagnostic = ${f_diag}（预期 0）"
if [ "$f_ok" != 0 ]; then
  printf '%s' "$F" | grep -o 'unknown identifier[^"]*' | sort -u | head -5
fi
echo

# ② Bare：同一文件必须彻底不认识 L1。
echo "== ② Bare（--bare）：同一文件必须报 elab-unknown-identifier =="
B="$( node "$SOKO" --bare "$TMP/l1.sokonanoda" 2>&1 )"
b_ok=1
printf '%s' "$B" | grep -q 'error\[elab-unknown-identifier\]:' && b_ok=0
echo "   → 报未知标识符（修后预期）：$([ "$b_ok" = 0 ] && echo yes || echo NO)"
echo

# ③ 让位：声明 And（B3）⇒ 依赖它的 Iff（B6）连带让位；Or/Eq 仍在。
echo "== ③ 让位：声明 And（B3）⇒ Iff.* 不在，Or.elim 仍在 =="
cat > "$TMP/yield.sokonanoda" <<'EOF'
inductive And (a b : Prop) : Prop
ctor And.intro (ha : a) (hb : b) : And a b
end
def probe_or (a b c : Prop) (f : a -> c) (g : b -> c) (h : Or a b) : c := Or.elim a b c f g h
def probe_iff (a b : Prop) (h : Iff a b) : b -> a := Iff.mpr a b h
EOF
Y="$( node "$SOKO" --json "$TMP/yield.sokonanoda" 2>&1 )"
y_or=1; y_iff=1
printf '%s' "$Y" | grep -q 'unknown identifier `Or`' || y_or=0
printf '%s' "$Y" | grep -q 'unknown identifier `Iff`' && y_iff=0
echo "   → Or 族仍在（修后预期 yes）：$([ "$y_or" = 0 ] && echo yes || echo NO)"
echo "   → Iff 已让位（修后预期 yes）：$([ "$y_iff" = 0 ] && echo yes || echo NO)"
echo

# ④ Or 是真归纳块：点号模式与裸模式都要过。
echo "== ④ Or 的 match：点号模式与裸模式都必须判卷通过 =="
for pat in "Or.inl ha" "inl ha"; do
  case "$pat" in
    Or.inl*) other="Or.inr hb" ;;
    *) other="inr hb" ;;
  esac
  cat > "$TMP/match.sokonanoda" <<EOF
def l1_or_comm (a b : Prop) (h : Or a b) : Or b a :=
  match h with
  | $pat => Or.inr b a ha
  | $other => Or.inl b a hb
EOF
  M="$( node "$SOKO" --json "$TMP/match.sokonanoda" 2>&1 )"
  m_ok=1
  printf '%s' "$M" | grep -q '"type":"decl.checked"' && ! printf '%s' "$M" | grep -q '"type":"diagnostic"' && m_ok=0
  echo "   → 模式 \`$pat\` 通过（修后预期 yes）：$([ "$m_ok" = 0 ] && echo yes || echo NO)"
  [ "$m_ok" = 0 ] || M_OK_FAIL=1
done
echo

if [ "$f_ok" = 0 ] && [ "$b_ok" = 0 ] && [ "$y_or" = 0 ] && [ "$y_iff" = 0 ] \
   && [ "${M_OK_FAIL:-0}" != 1 ]; then
  echo "结论：L-01 已修（L1 进 prelude、Bare 仍干净、让位依赖闭包、Or 是真归纳块）⇒ 行为已变，更新台账。" >&2
  exit 1
fi
echo "结论：缺口仍在（prelude 没有 L1 逻辑骨架）——见 crates/front/src/compile/prelude.rs 的 PRELUDE_L1_SRC。" >&2
exit 0
