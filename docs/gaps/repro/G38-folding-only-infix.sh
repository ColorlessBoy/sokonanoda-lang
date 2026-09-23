#!/usr/bin/env bash
# G-38 复现：线 C 的记法折叠只认 infix 族（`forall` 不折成 `∀`，`𝒫`/`ᶜ`/`∅` 同）。
# 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（见 docs/gaps/README.md）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
exec node docs/gaps/repro/G38-folding-only-infix.js
