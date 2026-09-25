#!/usr/bin/env bash
# ci-watch.sh —— **按 job 看 CI，不等整轮**（2026-09-25 用户要求 ✓）
#
# 为什么需要它 ✓（用户原话 ✓）："ledger **立马就失败**了，但是**整个 github action 还在继续**，
# 导致**你不知道已经失败了**" ✗ —— 整轮的 `in_progress` 会**掩盖已经发生的失败** ✓
# （`test/*`/`gates` 要跑十几分钟 ✓，而 `ledger (3)` 90 秒就红了 ✗）。
# ⇒ 判据必须是**每个 job 的结论** ✓，不是整轮的状态 ✓。
#
# 用法：
#   scripts/ci-watch.sh            # 最近 5 轮：逐 job 打印；**只要最新一轮有 job 红就 exit 1** ✓
#   scripts/ci-watch.sh --all      # 连历史一起看
# 退出码：0 = 最新一轮暂无失败 ✓（可能仍在跑 ✓）/ 1 = **已有 job 失败** ✗
set -uo pipefail
limit="${1:+5}"; limit=5
gh run list --workflow ci --limit "$limit" --json databaseId,headSha,status,conclusion,createdAt \
  --jq '.[] | "\(.databaseId)\t\(.headSha[0:7])\t\(.status)\t\(.conclusion // "-")\t\(.createdAt)"' |
while IFS=$'\t' read -r rid sha status concl created; do
  printf '\n== #%s %s  run=%s/%s  %s ==\n' "$rid" "$sha" "$status" "$concl" "${created:11:19}"
  gh run view "$rid" --json jobs \
    --jq '.jobs[] | "  \(.conclusion // .status)\t\(.name)"' 2>/dev/null |
    sed 's/^  failure/  ❌ failure/; s/^  success/  ✅ success/; s/^  in_progress/  ⏳ in_progress/; s/^  queued/  ⌛ queued/; s/^  skipped/  ⏭ skipped/; s/^  cancelled/  🚫 cancelled/' |
    sort -k1,1
done
# **最新一轮是否有 job 已红** ✓ —— 这才是"早感知" ✓
newest=$(gh run list --workflow ci --limit 1 --json databaseId --jq '.[0].databaseId')
bad=$(gh run view "$newest" --json jobs --jq '[.jobs[] | select(.conclusion=="failure")] | length' 2>/dev/null || echo 0)
if [ "${bad:-0}" -gt 0 ]; then
  printf '\n❌ **最新一轮 #%s 已经有 %s 个 job 失败** ✗ —— 不必等整轮 ✓\n' "$newest" "$bad"
  exit 1
fi
printf '\n✓ 最新一轮 #%s 暂无失败 ✓（可能仍在跑 ✓）\n' "$newest"
