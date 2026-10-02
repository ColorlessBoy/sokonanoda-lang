#!/usr/bin/env bash
# ============================================================================
# C-01 复现（课程侧 · 装配写法，不是语言缺口）
#
# 命题：`ordinal_trichotomy`（序数三歧 `E x y ∨ x = y ∨ E y x`）**真的在解答里、
#       且判卷为绿**（checked>0 · exercise_open==0 · 无拒绝）。
#
# 登记状态：**closed-green**（2026-10-02 第 681 轮装配成功；`OPEN-ITEMS.md` C-01）
# 期望：exit 0 = 与登记一致 ✓ ；exit ≠ 0 = 与登记不一致 ✗（缺口回归）
#
# 反向验证（必须做，且必须判红）：
#   把 `units/solutions/I.6/unit108-solution.sokonanoda` 里 `ordinal_trichotomy`
#   的证明体换成 `sorry` ⇒ 本脚本必须 exit ≠ 0（`exercise_open` 变 1 ⇒ 第二条断言炸）
#   ⇒ 换回来必须 exit 0。
#   实测（第 682 轮）：注入 sorry ⇒ exit 1（"exercise_open=1"）✓；恢复 ⇒ exit 0 ✓。
#
# 用法：bash courses/set-theory/gaps/C-01-trichotomy-assembly.sh
# ============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SOL="$ROOT/courses/set-theory/units/solutions/I.6/unit108-solution.sokonanoda"
NAME="ordinal_trichotomy"

if [ ! -f "$SOL" ]; then
  echo "✗ 解答文件不存在：$SOL"; exit 1
fi

# ① 声明必须在（名字与签名都在 ⇒ 不是"删掉了事"）
if ! grep -q "^theorem $NAME " "$SOL"; then
  echo "✗ C-01 回归：解答里找不到 \`theorem $NAME\`"; exit 1
fi

# ② 判卷必须是绿的：checked>0 · exercise_open==0 · 无拒绝
node "$ROOT/scripts/soko" query check --file "$SOL" > /tmp/c01-repro.json 2>&1
rc=$?
python3 - "$rc" "$NAME" <<'PY'
import json, sys
rc = int(sys.argv[1]); name = sys.argv[2]
try:
    d = json.load(open("/tmp/c01-repro.json"))
except Exception as exc:                      # 判卷器拒绝运行 / 解析失败
    print(f"✗ C-01 回归：判卷输出不是 JSON（exit={rc}）：{exc}"); sys.exit(1)
data = d.get("data") or {}
counts = data.get("counts") or {}
failed = data.get("failed") or []
checked = counts.get("decl_checked", 0)
open_ = counts.get("exercise_open", 0)
if rc != 0 or failed:
    print(f"✗ C-01 回归：判卷被拒（exit={rc}，failed={len(failed)}）"); sys.exit(1)
if checked <= 0:
    print(f"✗ C-01 回归：decl_checked={checked}（没有真判过）"); sys.exit(1)
if open_ != 0:
    print(f"✗ C-01 回归：exercise_open={open_}（`{name}` 被换成了 sorry？）"); sys.exit(1)
print(f"✓ C-01 与登记一致：`{name}` 在，checked={checked} · exercise_open=0 · failed=0")
PY
