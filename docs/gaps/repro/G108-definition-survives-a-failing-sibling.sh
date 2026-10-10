#!/usr/bin/env bash
# G108-definition-survives-a-failing-sibling 的复现外壳：真正的探针是同目录的 .js
# （台账只认 .sh —— 它用 bash 跑；驱动 LSP over stdio 需要 JSON-RPC 客户端）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/G108-definition-survives-a-failing-sibling.js
