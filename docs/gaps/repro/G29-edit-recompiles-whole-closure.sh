#!/usr/bin/env bash
# G-29 复现的**外壳**：真正的探针是隔壁的 `G29-edit-recompiles-whole-closure.js`
# （台账要求复现件是 `.sh`）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/G29-edit-recompiles-whole-closure.js
