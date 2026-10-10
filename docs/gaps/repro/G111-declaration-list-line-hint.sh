#!/usr/bin/env bash
# G111 —— Infoview 声明列表的每一行**还画着 `L<n>` 行号**（2026-10-11 用户要求去掉）。
#
# 用户原话：「infoview 里的『声明列表』里的行号可以不需要了，去掉吧」。
#
# ⚠ 删的是**显示**，不是数据：`decl.range` 仍在 wire 上、仍是"点名字跳到定义"的发车位
# ⇒ 本判据的后半条专门钉住"没把跳转一起删掉" ✗。
#
# 判据（读屏幕上会生效的那两份资产，`infoview.css` 先剥注释）：
#   ① `infoview.js` 里没有 `decl-line-hint`，也没有 `"L" + (line + 1)` 那种拼法；
#   ② `infoview.css`（剥注释后）里没有 `.decl-line-hint` 规则；
#   ③ 反向的一半：`infoview.js` 里 `range.start` 与 `definition` 消息**仍在**。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
cd "$(dirname "$0")/../../.." || exit 2
js=editor/vscode/media/infoview.js
css=editor/vscode/media/infoview.css
{ [ -f "$js" ] && [ -f "$css" ]; } || { echo "找不到 $js / $css" >&2; exit 2; }

fail=0
if grep -q 'decl-line-hint' "$js"; then
  echo "✗ 缺口仍在：infoview.js 还在渲染行号那一格（\`decl-line-hint\`）"
  fail=1
fi
if grep -q '"L" + (line + 1)' "$js"; then
  echo "✗ 缺口仍在：infoview.js 还在拼 1-based 行号（\"L\" + (line + 1)）"
  fail=1
fi
if python3 - "$css" <<'PY'
import re, sys
css = re.sub(r"/\*.*?\*/", "", open(sys.argv[1], encoding="utf-8").read(), flags=re.S)
raise SystemExit(0 if ".decl-line-hint" in css else 1)
PY
then
  echo "✗ 缺口仍在：infoview.css（剥注释后）还有 \`.decl-line-hint\` 规则"
  fail=1
fi
if ! grep -q 'range.start' "$js" || ! grep -q '"definition"' "$js"; then
  echo "✗ 形状异常：infoview.js 里 \`range.start\` / \`definition\` 消息不见了 —— 删行号不该动跳转"
  fail=1
fi

if [ "$fail" = 1 ]; then
  exit 0
fi
echo '✓ 已修：声明行只画「名字 + 种类」，不再有 `L<n>`；跳转的发车位（range.start → definition）原样保留'
exit 1
