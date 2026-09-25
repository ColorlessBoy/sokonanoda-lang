#!/usr/bin/env bash
# install-hooks.sh —— 把 `scripts/githooks/` 接到当前 clone（2026-09-25 ✓）
#
# 为什么是"安装"而不是"自动生效" ✗：git 的 hook 不随仓库分发 ✓ —— 必须有人执行一次 ✓。
# 所以这一步**放进 README / AGENTS.md 的 Setup** ✓，让新 clone 一次到位 ✓。
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
chmod +x scripts/githooks/* 2>/dev/null || true
git config core.hooksPath scripts/githooks
echo "✅ 已安装：core.hooksPath = $(git config core.hooksPath) ✓"
echo "   ⇒ 之后每次 git push 前会自动跑 scripts/ci-local.sh --fast（约 1 分钟 ✓）"
echo "   ⇒ 逃生门：git push --no-verify ✓ 或 SOKO_SKIP_HOOK=1 git push ✓"
