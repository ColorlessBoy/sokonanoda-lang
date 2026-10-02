#!/usr/bin/env bash
# ============================================================================
# 111 复现（课程侧 · 靠"墙已修"才能写的关键步）：**用 `Nat.rec` 递归定义 + 归纳**
#
# 关键步：prelude **只有 `Nat` 的构造子**（`Nat.zero`/`Nat.succ`）与 `Nat.rec`，
#   **没有 `Nat.add`/`Nat.mul`** ⇒ 课程自己用 `Nat.rec` 定义它们（**动机落 `Type`** ⇒
#   需要大消去 ✓），再用 `Nat.rec` + **Prop 动机**证 11 条算术律。
#   这一族以前写不了，靠的是台账 **G-76**（Nat 算术律展开）已修。
#
# 登记状态：**closed-green**（`units/I.6/unit111-nat-arith.sokonanoda` 落地，13 条全绿）
# 期望：exit 0 = 与登记一致（定义**真的走 `Nat.rec`** · 11 条律在 · 判卷 checked>0 / open==0）
#       exit != 0 = 与登记不一致（关键定义被换掉 / 某条律被换成 sorry ⇒ 缺口回归）
#
# 反向验证（必须做）：把 `add_comm` 的证明体换成 `sorry` ⇒ 本脚本必须 exit != 0；
#   换回来 ⇒ exit 0。实测数字记在 `OPEN-ITEMS.md`。
#
# 用法：bash courses/set-theory/gaps/C-111-nat-induction.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOL="$ROOT/courses/set-theory/units/solutions/I.6/unit111-solution.sokonanoda"
fail=0

[ -f "$SOL" ] || { echo "missing solution file: $SOL"; exit 1; }

# 关键步：两个定义必须**真的**落在 Nat.rec 上（动机落 Type）
if grep -q "^def add (a b : Nat) : Nat :=$" "$SOL" && grep -q "Nat.rec (fun (_ : Nat) => Nat) a" "$SOL"; then
  echo "OK  add is defined by Nat.rec (Type-valued motive) -- the large-elimination step"
else
  echo "BAD 111: add no longer goes through Nat.rec"
  fail=1
fi
if grep -q "^def mul (a b : Nat) : Nat :=$" "$SOL" && grep -q "Nat.rec (fun (_ : Nat) => Nat) Nat.zero" "$SOL"; then
  echo "OK  mul is defined by Nat.rec (Type-valued motive)"
else
  echo "BAD 111: mul no longer goes through Nat.rec"
  fail=1
fi

for name in add_zero add_succ zero_add succ_add add_assoc add_comm add_rotate mul_zero mul_succ zero_mul mul_distrib; do
  if grep -q "^theorem $name " "$SOL"; then
    echo "OK  present: $name"
  else
    echo "BAD 111: missing law: $name (regression)"
    fail=1
  fi
done

node "$ROOT/scripts/soko" query check --file "$SOL" > /tmp/c111-repro.json 2>&1
rc=$?
python3 - "$rc" <<'PYEOF'
import json, sys
rc = int(sys.argv[1])
try:
    d = json.load(open("/tmp/c111-repro.json"))
except Exception as exc:
    print(f"BAD 111: grader output is not JSON (exit={rc}): {exc}"); sys.exit(1)
data = d.get("data") or {}
counts = data.get("counts") or {}
failed = data.get("failed") or []
checked, open_ = counts.get("decl_checked", 0), counts.get("exercise_open", 0)
if rc != 0 or failed:
    print(f"BAD 111: grading rejected (exit={rc}, failed={len(failed)})"); sys.exit(1)
if checked < 13 or open_ != 0:
    print(f"BAD 111: checked={checked} (expect 13) open={open_}"); sys.exit(1)
print(f"OK  111 matches the register: checked={checked} exercise_open=0 failed=0")
PYEOF
rc2=$?
[ "$rc2" -eq 0 ] || fail=1
exit $fail
