#!/usr/bin/env bash
# G-36 复现：`position_to_offset` 按 `char` 计数，不是 LSP 要求的 UTF-16 码元。
# 真正的探针是隔壁的 `G36-utf16-position-mapping.js`（要起真 LSP、说 JSON-RPC）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/G36-utf16-position-mapping.js
