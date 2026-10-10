#!/usr/bin/env bash
# G104-command-line-hover 的复现外壳：真正的探针是共用的 `editor-hover-probe.js`（模式 `command-lines`）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/editor-hover-probe.js command-lines
