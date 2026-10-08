#!/usr/bin/env bash
# ============================================================================
# C-05 复现（课程侧 · 装配）：**序数加法（关系版）的结合律** —— 已装配 ✓
#
# 目标命题：`AddsTo x y z` 与 `AddsTo z w v` ⇒ 存在 `u` 使
#   `AddsTo y w u` 且 `AddsTo x u v`（即 `(x + y) + w = x + (y + w)`）。
#   同时交付**逆引理**：`AddsTo x (σ y) v` ⇒ 存在 `z` 使 `v = σ z` 且 `AddsTo x y z`
#   （带两条**命题本身需要**的假设：`hne` = 「`zero` 不是后继」· `hinj` = 「后继单射」——
#     对任意 `σ`/`zero` 命题是假的：`σ` 恒取 `zero` 时 `v` 可以不是后继、非单射时索引对不齐）。
#
# 登记状态：**closed**（`OPEN-ITEMS.md` C-05，2026-10-08 收口）
#   ⇒ 期望 **exit 0 = 与登记一致**：两条装配件都在解答里（且画布上都有同名练习）·
#     判卷 checked>0 · exercise_open==0 · 无判负。
#   exit != 0 = 与登记不一致（装配件被删 / 被换成 `sorry` ⇒ 缺口回归）。
#
# 为什么以前写不出（第 681 轮判红）：`AddsTo.rec` 的**调用形状/动机接线** ——
#   属 **G-73 家族**（`docs/gaps/ledger.jsonl`，2026-10-08 已 `fixed`）。三条关键写法：
#     ① recursor 的 **motive 是参数化的**（索引不吃统一）⇒ 命题写成
#        `∀ (y : α), b = σ y → …` 这种「把索引当参数」的形状；
#     ② 结论是函数型 ⇒ 最后**别忘了继续应用**（`… x (σ y) v h y (Eq.refl …)`）；
#     ③ 结合律**对第二条推导归纳**即可（把 `h1` 的实例一般化）—— 不必先有逆引理。
#
# 反向验证（实测，2026-10-08）：把 `addsTo_assoc` 的证明体换成 `sorry`
#   ⇒ 本脚本 exit 1（`BAD C-05: grading rejected`）；恢复 ⇒ exit 0。
#
# 用法：bash courses/set-theory/gaps/C-05-addsTo-assoc.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOL="$ROOT/courses/set-theory/units/solutions/I.6/unit109-solution.sokonanoda"
CANVAS="$ROOT/courses/set-theory/units/I.6/unit109-ordinal-rec.sokonanoda"
fail=0

[ -f "$SOL" ] || { echo "missing solution file: $SOL"; exit 1; }

for name in AddsTo addsTo_zero addsTo_succ; do
  if grep -q "^inductive $name \|^theorem $name \|^def $name " "$SOL"; then
    echo "OK  part present: $name"
  else
    echo "BAD C-05: part missing from the solution: $name (regression)"
    fail=1
  fi
done

for name in addsTo_succ_inv addsTo_assoc; do
  if grep -q "^theorem $name " "$SOL"; then
    echo "OK  assembly lemma present in the solution: $name"
  else
    echo "BAD C-05: assembly lemma MISSING from the solution: $name (the register says closed)"
    fail=1
  fi
  if grep -q "^theorem $name " "$CANVAS"; then
    echo "OK  assembly lemma is a canvas exercise too: $name"
  else
    echo "BAD C-05: assembly lemma is not a canvas exercise: $name"
    fail=1
  fi
done

node "$ROOT/scripts/soko" query check --file "$SOL" > /tmp/c05-repro.json 2>&1
rc=$?
python3 - "$rc" <<'PYEOF'
import json, sys
rc = int(sys.argv[1])
try:
    d = json.load(open("/tmp/c05-repro.json"))
except Exception as exc:
    print(f"BAD C-05: grader output is not JSON (exit={rc}): {exc}"); sys.exit(1)
data = d.get("data") or {}
counts = data.get("counts") or {}
failed = data.get("failed") or []
checked, open_ = counts.get("decl_checked", 0), counts.get("exercise_open", 0)
if rc != 0 or failed:
    print(f"BAD C-05: grading rejected (exit={rc}, failed={len(failed)})"); sys.exit(1)
if checked < 11 or open_ != 0:
    print(f"BAD C-05: checked={checked} (expect >= 11) open={open_}"); sys.exit(1)
print(f"OK  C-05 matches the register: checked={checked} exercise_open=0 failed=0")
PYEOF
rc2=$?
[ "$rc2" -eq 0 ] || fail=1
exit $fail
