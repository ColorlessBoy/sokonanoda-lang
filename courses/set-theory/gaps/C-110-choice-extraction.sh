#!/usr/bin/env bash
# ============================================================================
# 110 复现（课程侧 · 靠"墙已修"才能写的关键步）：**从 `∀∃` 取出函数**（选择公理的取数据）
#
# 关键步：`choice_family` / `choice_right_inverse` 把 `∀ a ∈ A, ∃ b ∈ B, P a b` **取出**成
#   函数 `∃ f, ∀ a ∈ A, f a ∈ B ∧ P a (f a)` —— 这一步以前写不了，靠的是
#   台账 **G-58**（Prop→Type 大消去）已修 + `lib/Choice` 的 `choice` 公理到位。
#
# 登记状态：**closed-green**（`units/I.6/unit110-choice.sokonanoda` 落地，4 条全绿）
# 期望：exit 0 = 与登记一致（定理在 · **证明里真的调用了 `choice`** · 判卷 checked>0 / open==0）
#       exit != 0 = 与登记不一致（关键步被拿掉或换成 sorry ⇒ 缺口回归）
#
# 反向验证（必须做）：把 `choice_family` 的证明体换成 `sorry` ⇒ 本脚本必须 exit != 0；
#   换回来 ⇒ exit 0。实测数字记在 `OPEN-ITEMS.md`。
#
# 用法：bash courses/set-theory/gaps/C-110-choice-extraction.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOL="$ROOT/courses/set-theory/units/solutions/I.6/unit110-solution.sokonanoda"
fail=0

[ -f "$SOL" ] || { echo "missing solution file: $SOL"; exit 1; }

for name in choice_family choice_right_inverse; do
  if grep -q "^theorem $name " "$SOL"; then
    echo "OK  present: $name"
  else
    echo "BAD 110: missing theorem: $name (regression)"
    fail=1
  fi
done

if grep -q "choice α β I (Set.univ β)" "$SOL" && grep -q "choice β α B A" "$SOL"; then
  echo "OK  the key step really goes through the choice axiom (both call sites)"
else
  echo "BAD 110: the choice call sites are gone -- the extraction step was bypassed"
  fail=1
fi

node "$ROOT/scripts/soko" query check --file "$SOL" > /tmp/c110-repro.json 2>&1
rc=$?
python3 - "$rc" <<'PYEOF'
import json, sys
rc = int(sys.argv[1])
try:
    d = json.load(open("/tmp/c110-repro.json"))
except Exception as exc:
    print(f"BAD 110: grader output is not JSON (exit={rc}): {exc}"); sys.exit(1)
data = d.get("data") or {}
counts = data.get("counts") or {}
failed = data.get("failed") or []
checked, open_ = counts.get("decl_checked", 0), counts.get("exercise_open", 0)
if rc != 0 or failed:
    print(f"BAD 110: grading rejected (exit={rc}, failed={len(failed)})"); sys.exit(1)
if checked <= 0 or open_ != 0:
    print(f"BAD 110: checked={checked} open={open_} (a proof was replaced by sorry?)"); sys.exit(1)
print(f"OK  110 matches the register: checked={checked} exercise_open=0 failed=0")
PYEOF
rc2=$?
[ "$rc2" -eq 0 ] || fail=1
exit $fail
