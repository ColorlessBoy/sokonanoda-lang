#!/usr/bin/env bash
# G-37 复现：记法声明行里的目标名不是使用点（着色 + 跳转一起坏）。
# 真探针是隔壁的 `.js`（要起真 LSP、说 JSON-RPC）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/G37-notation-decl-target-not-a-use-point.js
