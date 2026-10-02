#!/usr/bin/env bash
# ============================================================================
# 112 复现（课程侧）：**Cantor–Bernstein 定理已交付（经选择公理）**
#
# 目标命题（Enderton §6.4 定理 6B · Halmos §22）：`A ≼ B` 且 `B ≼ A` ⇒ `A ≈ B`。
#
# 登记状态：**closed-green**（`units/I.3/unit112-cantor-bernstein.sokonanoda` 落地：
#   画布 20 条给定件 + **12 条练习** · 解答 **32 条全绿 / 0 open** ✓）⇒ 期望
#   「**主定理在位且判绿**」= 本脚本 **exit 0**（三条断言全过）：
#   · ① **零件在**：`Set.InjOn`（`lib/Equiv`）+ `choice`（`lib/Choice`）+ `sInter`（`lib/SUnion`）✓；
#   · ② **主定理在位且判绿**：解答里有 `cantor_bernstein`，且整份解答判卷
#     `failed=0` / `exercise_open=0` / `decl_checked >= 32` ✓；
#   · ③ **墙仍在**：CB 双射的**直接定义**仍被判红（`C-112-cb-direct-def-reject.sokonanoda`，
#     诊断 `期望 Sort(0)，实际是 Sort(1)` ✓）—— **C-04 / G-6 一族**（Prop→Type 情形分析 ✗）。
#
# ⚠⚠ **诚实标注（原样保留，不许弱化）**：本课的 CB **经选择公理** —— 用 `choice` 从
#   "全 + 单值"的**关系版图**里取出双射函数 ✓。而 **CB 数学上不需要选择**
#   （`g` 单射 ⇒ 前像唯一 ⇒ 确定摹状词即可）⇒ **这是偏离，不是"标准证法"** ✗。
#   零公理的 CB 要 `Exists` → `Type` 的安全消去（**G-6** 一族，仍 open ✗）。
#
# 反向验证（三段都跑过，实测数字见 `OPEN-ITEMS.md` C-112）：
#   ① 正向 ⇒ exit 0 ；
#   ② 把解答里 `cantor_bernstein` 的**证明体**换成 `sorry` ⇒ exit 1（第 2 条断言炸）；
#      恢复 ⇒ exit 0 ；
#   ③ 注入"墙消失"（把 reject 探针换成判绿的内容）⇒ exit 1（第 3 条断言炸）；恢复 ⇒ exit 0。
#
# 用法：bash courses/set-theory/gaps/C-112-cantor-bernstein.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
LIB="$ROOT/courses/set-theory/lib"
GAPS="$ROOT/courses/set-theory/gaps"
SOL="$ROOT/courses/set-theory/units/solutions/I.3/unit112-solution.sokonanoda"
fail=0

for f in "$LIB/Equiv.sokonanoda" "$LIB/Choice.sokonanoda" "$LIB/SUnion.sokonanoda" "$SOL"; do
  [ -f "$f" ] || { echo "missing file: $f"; exit 1; }
done

# ── ① 零件在 ────────────────────────────────────────────────────────────────
if grep -q "InjOn" "$LIB/Equiv.sokonanoda" && grep -q "choice" "$LIB/Choice.sokonanoda" \
   && grep -q "sInter" "$LIB/SUnion.sokonanoda"; then
  echo "OK  parts present: Set.InjOn (lib/Equiv) + choice (lib/Choice) + Set.sInter (lib/SUnion)"
else
  echo "BAD 112: the parts are gone (InjOn / choice / sInter)"
  fail=1
fi

# ── ② 主定理在位且判绿 ──────────────────────────────────────────────────────
if grep -q "^theorem cantor_bernstein " "$SOL"; then
  echo "OK  the CB main theorem is present in the solution"
else
  echo "BAD 112: cantor_bernstein is missing from the solution (regression)"
  fail=1
fi

node "$ROOT/scripts/soko" query check --file "$SOL" > /tmp/c112-repro.json 2>&1
rc=$?
python3 - "$rc" <<'PYEOF'
import json, sys
rc = int(sys.argv[1])
try:
    d = json.load(open("/tmp/c112-repro.json"))
except Exception as exc:
    print(f"BAD 112: grader output is not JSON (exit={rc}): {exc}"); sys.exit(1)
data = d.get("data") or {}
counts = data.get("counts") or {}
failed = data.get("failed") or []
checked, open_ = counts.get("decl_checked", 0), counts.get("exercise_open", 0)
if rc != 0 or failed:
    print(f"BAD 112: the solution is rejected (exit={rc}, failed={len(failed)})"); sys.exit(1)
if checked < 32 or open_ != 0:
    print(f"BAD 112: checked={checked} (expect >= 32) open={open_} (expect 0)"); sys.exit(1)
print(f"OK  112 delivered: checked={checked} exercise_open=0 failed=0 (CB assembled via choice)")
PYEOF
rc2=$?
[ "$rc2" -eq 0 ] || fail=1

# ── ③ 墙仍在（直接定义双射仍被判红 ⇒ C-04 / G-6 一族）───────────────────────
WALL="$GAPS/C-112-cb-direct-def-reject.sokonanoda"
[ -f "$WALL" ] || { echo "BAD 112: missing wall probe: $WALL"; exit 1; }
node "$ROOT/scripts/soko" query check --file "$WALL" > /dev/null 2>&1
rcw=$?
if [ "$rcw" -ne 0 ]; then
  echo "OK  the wall is still real (C-04 / G-6 family): the direct CB bijection definition is rejected"
else
  echo "BAD 112: the wall is GONE -- the direct CB definition was ACCEPTED; re-evaluate the register"
  fail=1
fi

exit $fail
