#!/usr/bin/env bash
# G107 —— Infoview 声明列表里的**名字**要"普通粗体"，不许再抢镜（**两轮**收口 ✓）。
#
# 用户现场：
#   ① 2026-10-10「声明列表（declarations）里的名字的 css 样式比较抢镜，尤其是黑夜模式
#      （暗色主题下尤其刺眼）。可以改成普通的、类似于带超链接的文字（正常字号、
#      不加粗/不花哨，超链接式下划线或可点击样式即可）」；
#   ② 2026-10-11「声明的名字，**超链接的蓝色和下划线也都太抢镜了**，直接改成**普通粗体**，
#      不带其他样式了」—— 第 ① 轮的"链接式"修法被第 ② 轮**取代** ⇒ 本判据跟着改成
#      **普通粗体**（同一处 UI、同一条缺口，台账 `today` 里记着这次判据变更 ✓）。
#
# 判据读**屏幕上生效的那份 CSS**（`editor/vscode/media/infoview.css` 的 `.decl-name` 规则；
# **先剥注释** —— 注释里提到旧样式（`underline`/`textLink`）不算数 ✗）：
#   ① 必须是**粗体**：`font-weight: 700`（用户第 ② 轮点名的样式）；
#   ② 字号继承正文：`font: inherit`；
#   ③ 按钮默认外观清掉：`background: transparent` / `border: 0` / `padding: 0`；
#   ④ **不借链接的视觉语言**：不许 `text-decoration: underline`、不许 `--vscode-textLink*`。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
cd "$(dirname "$0")/../../.." || exit 2
css=editor/vscode/media/infoview.css
[ -f "$css" ] || { echo "找不到 $css" >&2; exit 2; }

body=$(python3 - "$css" <<'PY'
import re, sys
css = open(sys.argv[1], encoding="utf-8").read()
css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
match = re.search(r"\.decl-name\s*\{([^}]*)\}", css)
if not match:
    print("infoview.css 里没有 .decl-name 规则（形状异常）", file=sys.stderr)
    raise SystemExit(1)
print(match.group(1))
PY
) || { echo "取 .decl-name 规则失败（形状异常）" >&2; exit 2; }

fail=0
if ! grep -q 'font-weight: 700' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 不是普通粗体（缺 \`font-weight: 700\`）"
  fail=1
fi
if ! grep -q 'font: inherit' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 没有 \`font: inherit\`（字号必须继承正文）"
  fail=1
fi
for prop in 'background: transparent' 'border: 0' 'padding: 0'; do
  if ! grep -q "$prop" <<<"$body"; then
    echo "✗ 缺口仍在：`.decl-name` 缺 \`$prop\`（按钮默认外观没清掉 ⇒ 暗色主题下亮灰底）"
    fail=1
  fi
done
if grep -q 'text-decoration: underline' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 还带链接式下划线（用户 2026-10-11：下划线太抢镜）"
  fail=1
fi
if grep -q -- '--vscode-textLink' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 还在借主题链接色（用户 2026-10-11：蓝色太抢镜）"
  fail=1
fi

if [ "$fail" = 1 ]; then
  exit 0
fi
echo '✓ 已修：`.decl-name` 是普通粗体（font-weight: 700 + 继承字号 + 无按钮外观 + 无下划线/链接色）'
exit 1
