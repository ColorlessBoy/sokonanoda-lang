#!/usr/bin/env bash
# G-39 复现：import 的用户自定义记法符号在使用它的文件里认不出来。
# 真探针是隔壁的 `.js`（要起真 LSP、说 JSON-RPC）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/G39-imported-user-notation-has-no-navigation.js
