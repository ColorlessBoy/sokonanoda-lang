#!/usr/bin/env bash
# G-83 复现（**用户提交的全量口径**，脚本原样照搬 ✓）：**全量开放声明的顶 ≡ 底**
#
# 判据（用户 2026-10-02 判据升级）：卷 I 全部 `courses/set-theory/units/*/*.sokonanoda`，
# 每条 `status=open` 的声明，光标停在它的**第一个 tactic**，比
#   `query state`（顶） 与 `query goals` 里对应声明（底）
# 的 `(binders 名序列, goal)` **逐字相等** ⇒ 统计 `开放声明 / 顶≡底 / 顶≠底`。
#
# 用户本机读数（他的二进制 sha 前 16 位 656fd70e1baf77cf，19:55）：
#   `开放声明 875｜顶≡底 29｜顶≠底 841`
#   （840 = 顶空上下文 + 目标整句量词式；1 = 顶多出幻影 `_`；5 = 上下文同、目标文本不同）
# 本仓当前树的读数见脚本输出（若与该读数差很多 ⇒ 先报告差异，别急着改代码 ✓）。
#
# ⚠ 为什么用 `.sh` 外壳 + 同目录 `.js`：台账只认 `.sh`（`gap.py` 用 `bash` 跑），
#   而这份判据是用户用 Node 写的 ⇒ 外壳 `exec node` 到 `G83-top-equals-bottom.js` ✓
#   （约定见 `docs/gaps/README.md` 的「LSP 层的缺口」那行 ✓）。**脚本一个字节没改** ✓。
#
# 退出码约定（docs/gaps/README.md）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

command -v node >/dev/null 2>&1 || { echo "G-83: 需要 node" >&2; exit 2; }
[ -x target/debug/sokonanoda ] || { echo "G-83: 找不到 target/debug/sokonanoda（先 cargo build -p sokonanoda-cli）" >&2; exit 2; }

out="$(SOKO_BIN=./target/debug/sokonanoda timeout 3000 node docs/gaps/repro/G83-top-equals-bottom.js 2>&1)"
rc=$?
printf '%s\n' "$out"

# 末行格式：`开放声明 N｜顶≡底 S｜顶≠底 B`
line="$(printf '%s\n' "$out" | grep -E '^开放声明 ' | tail -1)"
[ -n "$line" ] || { echo "G-83: 认不出扫描输出（形状变了？）⇒ 环境不满足" >&2; exit 2; }
bad="$(printf '%s' "$line" | sed -E 's/.*顶≠底 ([0-9]+).*/\1/')"

if [ "$bad" = "0" ]; then
  echo "✓ G-83：全量开放声明 顶 ≡ 底（$line）⇒ 已修"
  exit 1
fi
echo "✗ G-83：缺口仍在 —— $line" >&2
exit 0
