#!/usr/bin/env bash
# ============================================================================
# C-04 复现（课程侧 · 语言行为边界）：**Prop → Type 的情形分析被判红**
#
# 登记状态：**closed · 非缺口（内核的正确行为）**（`OPEN-ITEMS.md` C-04，2026-10-02 第 681 轮）
# 两态（缺一不可，否则守卫是空的）：
#   ① `C-04-prop-to-type-reject.sokonanoda`  ⇒ 必须**被拒**（exit ≠ 0）
#   ② `C-04-prop-to-type-control.sokonanoda` ⇒ 必须**判绿**（exit 0）
# 期望：exit 0 = 两态都与登记一致 ✓ ；exit ≠ 0 = 与登记不一致 ✗
#
# 反向验证（实测，第 682 轮）：把两条断言的角色对调（拿 control 当 reject 用）⇒ 脚本
# **exit 1**（"① 期望被拒，实际判绿"）⇒ 断言不是真空的；换回来 ⇒ exit 0 ✓。
#
# 用法：bash courses/set-theory/gaps/C-04-prop-to-type.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
GAPS="$ROOT/courses/set-theory/gaps"
REJECT="$GAPS/C-04-prop-to-type-reject.sokonanoda"
CONTROL="$GAPS/C-04-prop-to-type-control.sokonanoda"
fail=0

run_one () {
  file="$1"
  want="$2"
  node "$ROOT/scripts/soko" query check --file "$file" > /tmp/c04-repro.json 2>&1
  rc=$?
  if [ "$want" = "reject" ]; then
    if [ "$rc" -eq 0 ]; then
      echo "BAD C-04 #1: the large-elimination probe was ACCEPTED (kernel behaviour changed; update the register)"
      fail=1
    else
      echo "OK  C-04 #1: large elimination is still correctly rejected (exit=$rc)"
    fi
  else
    if [ "$rc" -eq 0 ]; then
      echo "OK  C-04 #2: the control (motive in Prop) is green"
    else
      echo "BAD C-04 #2: the control (motive in Prop) was REJECTED (small elimination broke?)"
      fail=1
    fi
  fi
}

[ -f "$REJECT" ]  || { echo "missing file: $REJECT";  exit 1; }
[ -f "$CONTROL" ] || { echo "missing file: $CONTROL"; exit 1; }
run_one "$REJECT"  reject
run_one "$CONTROL" accept
exit $fail
