#!/usr/bin/env bash
# G-24 复现的**外壳**：真正的探针是隔壁的 `G24-project-cache-never-warms.py`
# （计时 + 隔离缓存目录；台账要求复现件是 `.sh`）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }
exec python3 docs/gaps/repro/G24-project-cache-never-warms.py
