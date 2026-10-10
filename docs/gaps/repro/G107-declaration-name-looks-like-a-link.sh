#!/usr/bin/env bash
# G107 —— Infoview 声明列表里的**名字**必须是"普通链接式文字"，不是抢镜的按钮。
#
# 用户现场（2026-10-10）：「infoview 里的声明列表（declarations）里的名字的 css 样式比较
# 抢镜，尤其是黑夜模式（暗色主题下尤其刺眼）。可以改成普通的、类似于带超链接的文字
# （正常字号、不加粗/不花哨，超链接式下划线或可点击样式即可）」。
# 根因两条（同一类问题横向排查）：① `font-weight: 600`（比正文粗一档）；
# ② 它是 `<button>` 而本文件没有通用 button reset ⇒ UA 的 `buttonface` 亮灰底 + 边框
# 在暗色主题下就是一块**亮灰**，正是"刺眼"。
#
# 判据读**屏幕上生效的那份 CSS**（`editor/vscode/media/infoview.css` 的 `.decl-name` 规则）：
#   ① 不许出现 `font-weight`（字号/字重继承正文 = `font: inherit`）；
#   ② 按钮外观必须显式清掉：`background: transparent` / `border: 0` / `padding: 0`；
#   ③ "可点"用链接式表达：`text-decoration: underline` + 主题链接色
#      （`--vscode-textLink-foreground`）。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
cd "$(dirname "$0")/../../.." || exit 2
css=editor/vscode/media/infoview.css
[ -f "$css" ] || { echo "找不到 $css" >&2; exit 2; }

body=$(python3 - "$css" <<'PY'
import re, sys
css = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r"\.decl-name\s*\{([^}]*)\}", css)
if not match:
    print("infoview.css 里没有 .decl-name 规则（形状异常）", file=sys.stderr)
    raise SystemExit(1)
print(match.group(1))
PY
) || { echo "取 .decl-name 规则失败（形状异常）" >&2; exit 2; }

fail=0
if grep -q 'font-weight' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 还在写 font-weight（不许加粗/放大，要继承正文）"
  fail=1
fi
if ! grep -q 'font: inherit' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 没有 `font: inherit`（字号/字重必须继承正文）"
  fail=1
fi
for prop in 'background: transparent' 'border: 0' 'padding: 0'; do
  if ! grep -q "$prop" <<<"$body"; then
    echo "✗ 缺口仍在：`.decl-name` 缺 \`$prop\`（按钮默认外观没清掉 ⇒ 暗色主题下亮灰底）"
    fail=1
  fi
done
if ! grep -q 'text-decoration: underline' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 没有链接式下划线（可点性要看得见）"
  fail=1
fi
if ! grep -q -- '--vscode-textLink-foreground' <<<"$body"; then
  echo "✗ 缺口仍在：`.decl-name` 没用主题链接色（跟随亮/暗主题，不写死颜色）"
  fail=1
fi

if [ "$fail" = 1 ]; then
  exit 0
fi
echo '✓ 已修：`.decl-name` 是普通链接式文字（继承字号/字重 + 无按钮外观 + 下划线 + 主题链接色）'
exit 1
