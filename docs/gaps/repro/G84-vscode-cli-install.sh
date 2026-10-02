#!/usr/bin/env bash
# G-84 复现：**VS Code 命令装出来的 sokonanoda CLI 版本不对，且没有 uninstall 命令**
#
# 用户报障（2026-10-02，**此前从未进过台账** —— 台账与队列双双 0 命中）：
#   「vscode 命令安装的 sokonanoda cli 版本不对，也缺少 uninstall 命令」
#
# ── 复测到的盘上事实（2026-10-02，本机）──
#   ① `~/.local/share/sokonanoda/bin/sokonanoda` 自述 **0.73.0**（09-27 的文件），
#      而插件早已 0.81.0 ⇒ 这就是"版本不对"；
#   ② 全仓 `uninstall` 在 `editor/vscode/{package.json,extension.js}` 里 **0 命中**
#      ⇒ 只有安装、没有卸载；盘上还留着 `sokonanoda-lsp.bak-0.55.0`（09-24）✗；
#   ③ 装到 `~/.local/share/sokonanoda/bin/`，而**该目录不在 PATH 上**
#      ⇒ 成功提示"重新打开终端后即可用"**是假的** ✗（`bash -lc 'command -v sokonanoda'` 空 ✓）；
#   ④ `sokonanoda.version` 内容是 `0.73.0 darwin-arm64`，与代码写的 `${version}\n`
#      **格式都对不上** ⇒ 更老路径留下的，之后从未成功覆盖 ✗；
#   ⑤ 假绿：`crates/cli/tests/extension.rs` 的值断言比的是**它自己刚拷的文件** ⇒ 天然相等 ✗。
#
# ── 判据（四条，全部由 `G84-vscode-cli-install.js` 在**用户动作级**重放）──
#   ① 版本不匹配即重装（幂等）：安装位先塞一只旧版（自述 0.0.1）⇒ 跑一次
#      `Sokonanoda: Install Command Line` ⇒ 安装位 `--version` == `package.json` 版本；
#   ② 有 `sokonanoda.uninstallCli`：跑一次 ⇒ 安装位（二进制 + `.version` + `*.bak-*`）清干净；
#   ③ PATH 落地：装完在**新开登录 shell**（`zsh -lc`/`bash -lc`，HOME = 临时家目录）里
#      `command -v sokonanoda` 必须解析到**安装位那一只**；
#   ④ **真判据**：③ 里那只跑出来的 `--version` == `package.json` 版本
#      （**不许**比"刚拷的文件" ✗ —— 那是假绿）。
#
# 退出码约定（docs/gaps/README.md）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

command -v node >/dev/null 2>&1 || { echo "G-84: 需要 node" >&2; exit 2; }
[ -f editor/vscode/package.json ] || { echo "G-84: 找不到 editor/vscode/package.json" >&2; exit 2; }
[ -x editor/vscode/bin/darwin-arm64/sokonanoda ] || [ -n "$(ls editor/vscode/bin/*/sokonanoda 2>/dev/null | head -1)" ] \
  || { echo "G-84: 扩展里没有暂存好的自带 CLI（先 npm run stage:lsp）" >&2; exit 2; }

criteria() { SOKO_G84_CRITERIA=1 node docs/gaps/repro/G84-vscode-cli-install.js "$ROOT"; }

case "${1:-}" in
  --criteria) criteria; exit $? ;;
esac

if criteria; then
  echo "✓ G-84：四条判据全部成立（版本对齐 · uninstall 存在 · PATH 落地 · 登录 shell 版本正确）"
  echo "  反向验证：模拟旧实现（只拷不问，SOKO_G84_LEGACY=1）⇒ 判据必须判红"
  scripts/expect-red.sh "撤掉修复（只拷不问）后 G-84 判据必须红" -- \
      env SOKO_G84_LEGACY=1 SOKO_G84_CRITERIA=1 node docs/gaps/repro/G84-vscode-cli-install.js "$ROOT" || exit 2
  exit 1
fi
echo "✗ G-84：缺口仍在 —— 见上方判据读数" >&2
exit 0
