#!/usr/bin/env bash
# ============================================================================
# C-120-C-121-C-130-cardinal-laws.sh —— 课程侧复现（G7 重放）：**登记 closed ⇒ 装配件都在且判卷全绿** ✓
#
# 覆盖：C-120（`Type.Equiv` 消去子 + 对称/传递律）· C-121（`Cardinal.mk` 单射性）· C-130（合取前提包装）
# 登记：**closed**（`courses/set-theory/OPEN-ITEMS.md`，2026-10-08 收口）
#   ⇒ 期望 **exit 0 = 与登记一致**：下列声明都在 **且**判卷全绿
#     （`checked >= N` · `exercise_open == 0` · 无判负）；
#   exit != 0 = 与登记不一致（声明被删 / 证明体被换成 `sorry` ⇒ 缺口回归）。
#
# 背景：这些形态当年都记在 **G-73 家族**（`docs/gaps/ledger.jsonl` 的 G-73 / G-85，
#   现为 `fixed` ✓）⇒ 今天可直接写 ✓（各条的关键写法见 OPEN-ITEMS 对应条目）。
#
# 反向验证（实测，2026-10-08）：把 `Type.Equiv.symm` 的证明体换成 `sorry` ⇒ **exit 1** ✓；恢复 ⇒ exit 0 ✓
# 用法：bash courses/set-theory/gaps/C-120-C-121-C-130-cardinal-laws.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
fail=0

have() {  # $1 = 文件（绝对路径）· $2 = 声明名
  if grep -q "^theorem $2 \|^def $2 \|^inductive $2 " "$1"; then
    echo "OK  present: $2  ($(basename "$1"))"
  else
    echo "BAD C-120-C-121-C-130-cardinal-laws.sh: missing declaration: $2  ($1)"
    fail=1
  fi
}

graded() {  # $1 = 文件 · $2 = 期望最少 checked 数
  local f="$1" min="$2"
  local out="/tmp/soko-repro-$(basename "$f").json"
  node "$ROOT/scripts/soko" query check --file "$f" > "$out" 2>&1
  local rc=$?
  python3 - "$rc" "$min" "$f" "$out" <<'PYEOF'
import json, sys
rc, mn, f, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], sys.argv[4]
try:
    d = json.load(open(out))
except Exception as exc:
    print(f"BAD: grader output is not JSON (exit={rc}): {exc}")
    sys.exit(1)
data = d.get("data") or {}
counts = data.get("counts") or {}
failed = data.get("failed") or []
checked, open_ = counts.get("decl_checked", 0), counts.get("exercise_open", 0)
if rc != 0 or failed:
    print(f"BAD: grading rejected (exit={rc}, failed={len(failed)}) in {f}")
    sys.exit(1)
if checked < mn or open_ != 0:
    print(f"BAD: {f} checked={checked} (expect >= {mn}) open={open_}")
    sys.exit(1)
print(f"OK  graded green: {f.rsplit('/', 1)[-1]} checked={checked} open=0 failed=0")
PYEOF
  [ $? -eq 0 ] || fail=1
}

F="$ROOT/courses/set-theory/lib/Cardinal.sokonanoda"
have "$F" "Type.Equiv.elim"
have "$F" "Type.Equiv.symm"
have "$F" "Type.Equiv.comp_left_aux"
have "$F" "Type.Equiv.comp_right_aux"
have "$F" "Type.Equiv.comp"
have "$F" "Type.Equiv.trans"
have "$F" "Type.Equiv.of_inverses''"
have "$F" "Cardinal.mk_inj"
graded "$F" 19

exit $fail
