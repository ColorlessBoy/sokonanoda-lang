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

# ⚠ **时限必须 > 真实运行时间**（2026-10-07 修 ✗→✓）：这里原先写死 `timeout 3000`（50 分钟），
#   而本口径的实测运行时间是 **≈1.5h**（台账 `repro_slow.last`：897 条开放声明 ×2 查询；
#   `gap.py::clean_env()` 固定 `SOKONANODA_NO_PROJECT_ARTIFACTS=1` ⇒ 一次 `query state`
#   2.7s（带产物 0.027s，100×）⇒ 全量 >3000s ✗）⇒ **扫描每次都被外壳自己掐死**、
#   JS 一个字都没来得及打 ⇒ 下面 grep 不到 `^开放声明 ` ⇒ **永远 exit 2**
#   （= 「环境不满足」）⇒ **这条判据永远出不了结论**（`close` 必被拒、`check` 只能跳过）✗
#   —— 属于「咬不住的守卫等于没有」那一类。⇒ 上限改成可调、默认 **6h** ✓。
#   ⚠ 只动**本仓自己写的 `.sh` 外壳**；同目录 `G83-top-equals-bottom.js` 是**用户提交件**，
#     仍然**逐字节冻结** ✗（含它的 `SOKONANODA_NO_CACHE=1`）。
#   ⚠ 外层 `gap.py` 的 `SOKO_GAP_REPRO_TIMEOUT` 必须**大于**这个值（关账用 25200 ✓）。
out="$(SOKO_BIN=./target/debug/sokonanoda timeout "${SOKO_G83_TIMEOUT:-21600}" node docs/gaps/repro/G83-top-equals-bottom.js 2>&1)"
rc=$?
printf '%s\n' "$out"

# 末行格式：`开放声明 N｜顶≡底 S｜顶≠底 B`
line="$(printf '%s\n' "$out" | grep -E '^开放声明 ' | tail -1)"
[ -n "$line" ] || { echo "G-83: 认不出扫描输出（形状变了？）⇒ 环境不满足" >&2; exit 2; }
bad="$(printf '%s' "$line" | sed -E 's/.*顶≠底 ([0-9]+).*/\1/')"

if [ "$bad" = "0" ]; then
  # ⚠ **`${line}` 必须带花括号**（2026-10-07 实测 ✗→✓）：原先写的是不带花括号的
  #   `$line` **紧跟全角 `）`**，而
  #   `）` 的 UTF-8 是 `EF BC 89` ⇒ **bash 3.2（macOS `/bin/bash`）把首字节 `EF`
  #   吃进变量名** ⇒ 在 `set -u` 下报 `line<0xEF>: unbound variable` ✗ —— 这条
  #   错误信息**本身不是合法 UTF-8** ⇒ `gap.py` 的 `text=True` 严格解码**当场崩**
  #   （`UnicodeDecodeError: byte 0xef in position 116`）✗ ⇒ 缺口明明已修，
  #   `close` 却**永远关不掉**。花括号把名字边界钉死 ⇒ 与 locale/bash 版本无关 ✓。
  echo "✓ G-83：全量开放声明 顶 ≡ 底（${line}）⇒ 已修"
  exit 1
fi
echo "✗ G-83：缺口仍在 —— ${line}" >&2
exit 0
