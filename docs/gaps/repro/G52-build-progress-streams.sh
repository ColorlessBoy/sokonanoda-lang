#!/usr/bin/env bash
# G-52 复现（**已修后的回归守卫**）：手动 Build/Rebuild 三处显示面没有进度、且不能取消
#
# 为什么是 .sh（2026-10-02 用户要求 ✓）：台账原来这条的 `repro` 字段写的是
# 「测试名 + 散文」⇒ `gap.py` 读成 `missing` ⇒ **静默跳过**（计不进 bad）✗ ⇒
# 「自称已修」与「真验过」在输出里长得一模一样 ✗。现在把它变成**能机械重放的 .sh** ✓。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

command -v node >/dev/null 2>&1 || { echo "G-52: 需要 node" >&2; exit 2; }
[ -f editor/vscode/test-extension-host.js ] || { echo "G-52: 找不到 editor/vscode/test-extension-host.js" >&2; exit 2; }

# 该 runner 不吃 argv 过滤（整份跑，55 个用例）⇒ 判据 = **整份绿**；
# 出事时下面把用例名打出来（`build streams per-file progress to the status bar and the Infoview (E23)`）。
out="$(timeout 900 node editor/vscode/test-extension-host.js 2>&1)"
rc=$?

if [ "$rc" -eq 0 ]; then
  echo "✓ 扩展宿主 stub 套件全绿（含 `build streams per-file progress to the status bar and the Infoview (E23)`）⇒ G-52 已修（行为已变）"
  printf '%s\n' "$out" | tail -1
  exit 1
fi
echo "✗ 扩展宿主 stub 套件判红（rc=${rc}）⇒ 缺口仍在 / 行为回退" >&2
printf '%s\n' "$out" | grep -E "^not ok|^ *[0-9]+/[0-9]+ passed" | tail -5 >&2
exit 0
