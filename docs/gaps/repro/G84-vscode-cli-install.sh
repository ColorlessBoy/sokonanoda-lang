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
# ── 环境前提（2026-10-19）──
#   判据 ①③ 要的是「装出去的是**包里**那份、且与插件同版本」⇒ `editor/vscode/bin/**` 里
#   的暂存 CLI **必须**是插件版本（它是 gitignored 构建产物 ⇒ 开发机上可能是任一历史构建）。
#   陈旧暂存（实测 0.92.0 vs 插件 0.94.0）会让 ①③ 判红 —— 那**不是**产品缺口 ✗。
#   ⇒ 本件先核暂存版本，缺失/不等就 `node editor/vscode/scripts/stage-lsp.js` 重暂存；
#     仍不等 ⇒ **exit 2**（环境不齐）。判据四条与反向验证**一字未改** ✓。
#
# 退出码约定（docs/gaps/README.md）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

command -v node >/dev/null 2>&1 || { echo "G-84: 需要 node" >&2; exit 2; }
[ -f editor/vscode/package.json ] || { echo "G-84: 找不到 editor/vscode/package.json" >&2; exit 2; }
VERSION="$(node -p "require('./editor/vscode/package.json').version" 2>/dev/null)" || VERSION=""
[ -n "$VERSION" ] || { echo "G-84: 读不出 editor/vscode/package.json 的 version" >&2; exit 2; }

# ── **环境前提由复现件自己保证**（2026-10-19）──────────────────────────────
# `editor/vscode/bin/**` 是 **gitignored 构建产物**（`.gitignore:20`）⇒ 开发机上它可以是
# **任何一个历史构建**（实测：陈旧暂存自述 0.92.0，而插件已 0.94.0）。而四条判据问的是
# 「装出去的那只 == 插件版本」—— 只有**包里自带的那份 CLI 本身就是这个版本**时才成立。
# 旧写法遇到陈旧暂存就直接跑 ⇒ ① 判红（安装位自述 0.92.0），② 的「版本不一致」分支按
# 设计**不**调 `ensureCliOnLoginPath()` ⇒ ③ 也判红（PATH 没落地，登录 shell 回退到真安装）
# ⇒ **2 条判据不成立** ✗ —— 那不是产品缺口，是**环境前提不成立** ✗。
# ⇒ 这里先**把陈旧暂存清掉、重新暂存一份**（`stage-lsp.js` 默认取 host **release** 构建；
# CI 的 ledger job 也是同一条命令 ✓，所以两条路的产物口径一致）。
# ⚠ **判据一条都没放水** ✗：暂存更新后照样跑那四条；更新不到插件版本 ⇒ **exit 2**
# （环境不齐），绝不许报「已修」或「缺口仍在」 ✗。这也**不是**「期望判红」✗。
staged_clis() { ls editor/vscode/bin/*/sokonanoda 2>/dev/null; }
cli_ver() { "$1" --version 2>/dev/null | grep -o '[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*' | head -1; }

need_stage=0
[ -n "$(staged_clis)" ] || need_stage=1
for f in $(staged_clis); do
  [ "$(cli_ver "$f")" = "$VERSION" ] || need_stage=1
done
if [ "$need_stage" = 1 ]; then
  stage_log="$(mktemp)"
  echo "· 暂存的自带 CLI 缺失或版本 ≠ 插件 $VERSION ⇒ 重新暂存（node editor/vscode/scripts/stage-lsp.js）"
  if ! timeout 180 node editor/vscode/scripts/stage-lsp.js >"$stage_log" 2>&1; then
    echo "G-84: 重新暂存失败（先 cargo build --release -p sokonanoda-cli）—— 末行：$(tail -n 1 "$stage_log")" >&2
    rm -f "$stage_log"
    exit 2
  fi
  rm -f "$stage_log"
fi

# **暂存后仍不是插件版本** ⇒ 环境不齐（例如 `target/release/sokonanoda` 还是旧构建）⇒
# **exit 2**，不许拿它当「缺口仍在」（那会把环境问题读成产品回归 ✗）。
fresh=0
for f in $(staged_clis); do
  [ "$(cli_ver "$f")" = "$VERSION" ] && fresh=1
done
[ "$fresh" = 1 ] || {
  echo "G-84: 暂存后仍没有一只自述 $VERSION 的自带 CLI（查 target/release/sokonanoda 的版本 / stage-lsp.js）" >&2
  exit 2
}

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
