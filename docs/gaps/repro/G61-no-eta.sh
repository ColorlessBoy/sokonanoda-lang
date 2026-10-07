#!/usr/bin/env bash
# **G-61**：单构造子 inductive 的 **η** —— `Setoid`/`Quotient` 包装层的地基。
#
# ── 2026-10-07 收口（第 137 棒 ✓）：**η 早就在内核里** ✓ ──────────────────────────
# 本轮**没有改内核**（一个字都没改 ✗）—— 侦察 + 反向验证证明**四条 η 机制都在** ✓，
# 而**台账原复现件是坏的** ✗：它把 `Box.get α b`（类型 `α`）与 `b`（类型 `Box α`）
# 摆进同一条等式 ⇒ 内核拒的是**等式自身类型不合法**（「期望 α，实际是 Box α」），
# **不是**「η 不成立」✗。判据：`def bad (α : Type) (b : Box α) : α := b` 报**同一条**
# 消息（纯类型错 ✓）。**同样的写法在官方 Lean 4 里也是类型错** ✗。
#
# 修正后的夹具（**well-typed 的 η 律** ✓，与 Lean 的 `tests/lean/run/etaStruct.lean` 同形）：
#   · `Box.mk α (Box.get α b) = b`  ← 结构 η（`x = ⟨x.1, …, x.n⟩` ✓）
#   · `Box.id α b = b`             ← 递归子 + η（`Box.id := Box.rec … (fun a => Box.mk α a)` ✓）
#   · `Setoid.mk α (Setoid.r α s) (Setoid.refl α s) = s` ← 台账点名的 `Setoid.mk s.r s.iseqv = s` ✓
#
# 退出码（docs/gaps/README.md · `status=fixed` ⇒ 期望**非零** ✓）：
#   **1 = 契约成立** ✓（η 有 ✓ · **多构造子仍不给** η ✓ · iota 没坏 ✓ · 不挂死 ✓）
#   **0 = 回退** ✗（任一条不成立：η 缺了 / 多构造子拿到 η（**判定变宽** ✗）/ iota 坏了）
#   **2 = 判据本身失效** ✗（η 机制的源码符号不见了 / 二进制起不来）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-61: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

# ── ⓪ **判据本身还在吗** ✓（机制没了 ⇒ 本判据**失效** ⇒ exit 2，不是「已修」✗）──────────
#    四样：conv 侧的结构 η + 单位元 η、eval 侧的「主前提 η 展开」、以及闸（`can_be_struct`）。
for needle in "fn try_struct_eta" "fn try_eta_struct_v" "fn is_unit_inductive"; do
  grep -q "$needle" crates/kernel/src/conv.rs \
    || { echo "G-61: 判据失效 —— conv.rs 里 \`$needle\` 不见了" >&2; exit 2; }
done
grep -q "fn try_struct_eta_reduce" crates/kernel/src/eval.rs \
  || { echo "G-61: 判据失效 —— eval.rs 里 \`try_struct_eta_reduce\` 不见了" >&2; exit 2; }
grep -q "fn can_be_struct" crates/kernel/src/env.rs \
  || { echo "G-61: 判据失效 —— env.rs 里 \`can_be_struct\` 不见了" >&2; exit 2; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# 每个夹具一个文件：一条判红不会把后面的声明变成「unknown identifier」✗（本文件曾因此失真 ✗）。
cat > "$WORK/a_iota.sokonanoda" <<'EOF'
inductive Box (α : Type) : Type
ctor mk (a : α) : Box α
end
def Box.get (α : Type) (b : Box α) : α :=
  Box.rec.{1} α (fun (_ : Box α) => α) (fun (a : α) => a) b
theorem Box.get_mk (α : Type) (a : α) : Box.get α (Box.mk α a) = a := Eq.refl.{1} α a
EOF

cat > "$WORK/b_struct_eta.sokonanoda" <<'EOF'
inductive Box (α : Type) : Type
ctor mk (a : α) : Box α
end
def Box.get (α : Type) (b : Box α) : α :=
  Box.rec.{1} α (fun (_ : Box α) => α) (fun (a : α) => a) b
def Box.id (α : Type) (b : Box α) : Box α :=
  Box.rec.{1} α (fun (_ : Box α) => Box α) (fun (a : α) => Box.mk α a) b
theorem eta_struct (α : Type) (b : Box α) : Box.mk α (Box.get α b) = b := Eq.refl.{1} (Box α) b
theorem eta_rec (α : Type) (b : Box α) : Box.id α b = b := Eq.refl.{1} (Box α) b
EOF

# ② **多构造子绝不给 η** ✗（给了就是判定变宽 ⇒ 本判据判 0 ✓）
cat > "$WORK/c_two_ctors.sokonanoda" <<'EOF'
inductive Two : Type
ctor a : Two
ctor b : Two
end
def Two.id (x : Two) : Two :=
  Two.rec.{1} (fun (_ : Two) => Two) Two.a Two.b x
theorem two_id (x : Two) : Two.id x = x := Eq.refl.{1} Two x
EOF

# ③ **拦路石没了的证据** ✓：`Setoid.r (Setoid.mk r h) ≡ r`（iota ✓）与
#    `Setoid.mk s.r s.iseqv ≡ s`（η ✓），外加 `Quotient (Setoid.mk r h) ≡ Quot r` 的**定义相等** ✓。
#    ⚠ 只做证据 ✗：**不建课程库**（`Setoid`/`Quotient` 的正式定义是后续一笔 ✓）。
cat > "$WORK/d_setoid.sokonanoda" <<'EOF'
inductive Setoid (α : Type) : Type
ctor mk (r : α → α → Prop) (iseqv : ∀ (a : α), r a a) : Setoid α
end
def Setoid.r (α : Type) (s : Setoid α) : α → α → Prop :=
  Setoid.rec.{1} α (fun (_ : Setoid α) => α → α → Prop)
    (fun (r : α → α → Prop) (_ : ∀ (a : α), r a a) => r) s
def Setoid.refl (α : Type) (s : Setoid α) : ∀ (a : α), Setoid.r α s a a :=
  Setoid.rec.{0} α (fun (s : Setoid α) => ∀ (a : α), Setoid.r α s a a)
    (fun (r : α → α → Prop) (h : ∀ (a : α), r a a) => h) s
def Quotient (α : Type) (s : Setoid α) : Type := Quot (Setoid.r α s)
def Quotient.mk (α : Type) (s : Setoid α) (a : α) : Quotient α s := Quot.mk (Setoid.r α s) a
theorem setoid_iota (α : Type) (r : α → α → Prop) (h : ∀ (a : α), r a a) :
    Setoid.r α (Setoid.mk α r h) = r := Eq.refl.{1} (α → α → Prop) r
theorem setoid_eta (α : Type) (s : Setoid α) :
    Setoid.mk α (Setoid.r α s) (Setoid.refl α s) = s := Eq.refl.{1} (Setoid α) s
def quotient_wrap (α : Type) (r : α → α → Prop) (h : ∀ (a : α), r a a) :
    Quotient α (Setoid.mk α r h) → Quot r := fun (q : Quot r) => q
EOF

# ④ **不挂死** ✓：120 层 `Box.mkid` 链 ⇒ 每层一次 η 展开，**有界**完成 ✓（`timeout` 兜底 ✓）。
{
  echo 'inductive Box (α : Type) : Type'
  echo 'ctor mk (a : α) : Box α'
  echo 'end'
  echo 'def Box.get (α : Type) (b : Box α) : α :='
  echo '  Box.rec.{1} α (fun (_ : Box α) => α) (fun (a : α) => a) b'
  echo 'def Box.mkid (α : Type) (b : Box α) : Box α :='
  echo '  Box.rec.{1} α (fun (_ : Box α) => Box α) (fun (a : α) => Box.mk α a) b'
  echo 'def d0 (b : Box Nat) : Box Nat := b'
  for k in $(seq 1 120); do
    echo "def d$k (b : Box Nat) : Box Nat := Box.mkid Nat (d$((k - 1)) b)"
  done
  echo 'theorem stress_eta (b : Box Nat) : d120 b = b := Eq.refl.{1} (Box Nat) b'
} > "$WORK/e_stress.sokonanoda"

json() { timeout 60 "$BIN" --json --no-project "$1" 2>&1; }
# 事件流里 `decl.checked` 才代表这条声明**过**了 ✓（`exercise.open` 不算证据 ✗ —— G-01 假绿）。
is_checked() { printf '%s' "$2" | grep -q "\"name\":\"$1\",\"type\":\"decl.checked\""; }
has_diag() { printf '%s' "$1" | grep -q '"type":"diagnostic"'; }

fail=0
a_out="$(json "$WORK/a_iota.sokonanoda")" || true
if ! is_checked Box.get_mk "$a_out"; then
  echo "G-61 ①：iota 对照组（Box.get_mk）不过 —— 内核坏了，不是 η 的事" >&2
  printf '%s\n' "$a_out" | head -3 >&2
  fail=1
fi

b_out="$(json "$WORK/b_struct_eta.sokonanoda")" || true
for d in eta_struct eta_rec; do
  if ! is_checked "$d" "$b_out"; then
    echo "G-61 ②：单构造子 η 回退了 —— \`$d\` 不再 checked ✗" >&2
    printf '%s\n' "$b_out" | grep -m1 diagnostic >&2
    fail=1
  fi
done

c_out="$(json "$WORK/c_two_ctors.sokonanoda")" || true
if is_checked two_id "$c_out" || ! has_diag "$c_out"; then
  echo "G-61 ③：**多构造子拿到了 η** ✗（判定被放宽）—— two_id 必须仍判红" >&2
  printf '%s\n' "$c_out" | head -3 >&2
  fail=1
fi

d_out="$(json "$WORK/d_setoid.sokonanoda")" || true
for d in setoid_iota setoid_eta quotient_wrap; do
  if ! is_checked "$d" "$d_out"; then
    echo "G-61 ④：Setoid/Quotient 地基回退了 —— \`$d\` 不再 checked ✗" >&2
    printf '%s\n' "$d_out" | grep -m1 diagnostic >&2
    fail=1
  fi
done

e_out="$(json "$WORK/e_stress.sokonanoda")" || true
if ! is_checked stress_eta "$e_out"; then
  echo "G-61 ⑤：120 层 η 链不 bounded（判红或超时）✗ —— η 的终止性有问题" >&2
  printf '%s\n' "$e_out" | grep -m1 diagnostic >&2
  fail=1
fi

[ "$fail" = 1 ] && { echo 'G-61：契约不成立（回退 ✗）' >&2; exit 0; }
echo 'G-61：契约成立 ✓（单构造子 η 有 ✓ · 多构造子不给 η ✓ · iota 没坏 ✓ · 120 层有界 ✓）'
exit 1
