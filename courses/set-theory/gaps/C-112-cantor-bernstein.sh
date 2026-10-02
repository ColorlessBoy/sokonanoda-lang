#!/usr/bin/env bash
# ============================================================================
# 112 复现（课程侧）：Cantor–Bernstein **仍未装配** —— 且卡点已从"预算"变成**精确的墙**
#
# 目标命题（Enderton §6.4 定理 6B · Halmos §22）：`A ≼ B` 且 `B ≼ A` ⇒ `A ≈ B`。
#
# 登记状态：**open**（`OPEN-ITEMS.md` C-112）⇒ 期望"**缺口仍在**" = 本脚本 **exit 0**：
#   · 零件在（`Set.InjOn` ✓ / `choice` ✓ / `Set.mem_sInter_iff` ✓）；
#   · CB 主定理**不在**任何解答里 ✓；
#   · **墙是真的**（第 3 条 ✓）：CB 双射的**直接定义**仍被判红（`C-112-cb-direct-def-reject.sokonanoda`，
#     诊断 `期望 Sort(0)，实际是 Sort(1)` ✓）—— 这是 **C-04 / G-6 一族**（Prop→Type 情形分析 ✗）。
#
# ⚠⚠ **诚实标注（写单元时必须原样带上，不许弱化）**：本课的 CB 打算**经选择公理**
#   —— 用 `choice` 从"全 + 单值"的**关系版图**里取出双射函数 ✓。而 **CB 数学上不需要选择**
#   （`g` 单射 ⇒ 前像唯一 ⇒ 确定摹状词即可）⇒ **这是偏离，不是"标准证法"** ✗。
#   零公理的 CB 要 `Exists`→Type 的安全消去（**G-6** 一族，仍 open ✗）。
#
# 反向验证（三段都要跑）：
#   ① 正向 ⇒ exit 0 ；
#   ② 注入"CB 主定理已存在" ⇒ exit 1（第 2 条断言炸）；
#   ③ 注入"墙消失"（把 reject 探针换成判绿的内容）⇒ exit 1（第 3 条断言炸）；恢复 ⇒ exit 0。
#
# 用法：bash courses/set-theory/gaps/C-112-cantor-bernstein.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOLS="$ROOT/courses/set-theory/units/solutions"
LIB="$ROOT/courses/set-theory/lib"
GAPS="$ROOT/courses/set-theory/gaps"
fail=0

for f in "$LIB/Equiv.sokonanoda" "$LIB/Choice.sokonanoda"; do
  [ -f "$f" ] || { echo "missing library file: $f"; exit 1; }
done
if grep -q "InjOn" "$LIB/Equiv.sokonanoda" && grep -q "choice" "$LIB/Choice.sokonanoda"; then
  echo "OK  parts present: Set.InjOn (lib/Equiv) + choice (lib/Choice)"
else
  echo "BAD 112: the parts are gone (InjOn / choice)"
  fail=1
fi

hit="$(grep -rl "^theorem cantorBernstein \|^theorem cantor_bernstein \|^theorem cb_" "$SOLS" 2>/dev/null | head -3)"
if [ -z "$hit" ]; then
  echo "OK  112 gap still open (as registered): no CB main theorem in any solution yet"
else
  echo "BAD 112: a CB main theorem IS present now: $hit"
  echo "    -> the register must move from open to closed and this script's expectation flips"
  fail=1
fi

WALL="$GAPS/C-112-cb-direct-def-reject.sokonanoda"
[ -f "$WALL" ] || { echo "BAD 112: missing wall probe: $WALL"; exit 1; }
node "$ROOT/scripts/soko" query check --file "$WALL" > /dev/null 2>&1
rcw=$?
if [ "$rcw" -ne 0 ]; then
  echo "OK  the wall is still real (C-04 family): the direct CB bijection definition is rejected"
else
  echo "BAD 112: the wall is GONE -- the direct CB definition was ACCEPTED; re-evaluate the register"
  fail=1
fi

exit $fail
