#!/usr/bin/env bash
# ci-watch.sh —— **按 job 拿 CI 反馈**（2026-09-25 用户要求 ✓："明明可以很快就拿到反馈数据的" ✓）
#
# 为什么不能用"整轮状态"✗（用户原话 ✓）："ledger **立马就失败**了，但是**整个 github action
# 还在继续**，导致**你不知道已经失败了**" ✓ —— 整轮 `in_progress` 会掩盖**已发生的失败** ✗。
#
# **快的数据在哪** ✓（实测 ✓）：
#   ✅ `gh run view <id> --json jobs`          → 每个 job 的**结论 + 起止时间** ✓（逐个 job 出现 ✓）
#   ✅ `gh api …/check-runs/<job>/annotations` → **失败原因** ✓，**整轮没结束就读得到** ✓
#   ✗  `gh run view --job … --log`             → 整轮结束前**不提供** ✗
#   ✗  Step summary                            → **没有 API** ✗（只在页面 ✓）
#   ✗  `curl` job 页面                         → 日志**懒加载** ✗（实测关键字零命中 ✓）
#
# 用法：
#   scripts/ci-watch.sh                 # 最新一轮：逐 job + 耗时；失败 job 打出**注解原文**；有红即 exit 1
#   scripts/ci-watch.sh --follow [secs] # 轮询（默认 20s ✓），**只打印变化** ✓，**一有失败立刻 exit 1** ✓
#   scripts/ci-watch.sh --run <id>      # 看指定一轮
#   scripts/ci-watch.sh --all           # 最近 5 轮
set -o pipefail   # **不用 -u** ✗：gh 的输出里常有空字段 ✓（skipped 的 job 没有时间 ✓），
                  # `set -u` 会让它在**循环中途**炸掉 ✗（实测两轮 ✓）

ann() {  # 打印某个 job 的注解（含逐条缺口 ✓）
  local rid="$1" name="$2"
  local jid
  jid=$(gh run view "$rid" --json jobs --jq --arg n "$name" '.jobs[] | select(.name==$n) | .databaseId' 2>/dev/null | head -1)
  [ -z "$jid" ] && { echo "          ↳ （拿不到 job id ✗）"; return 0; }
  gh api "repos/${REPO}/check-runs/$jid/annotations" \
      --jq '.[] | "        [\(.annotation_level)] \(.title // ""): \(.message)"' 2>/dev/null \
    | grep -v "ubuntu-latest label will migrate" | head -8
}

show() {
  local rid="$1"
  gh run view "$rid" --json headSha,status,conclusion,createdAt \
     --jq '"  sha=\(.headSha[0:7])  整轮=\(.status)/\(.conclusion // "-")  起=\(.createdAt[11:19])"' 2>/dev/null
  local rows fails
  rows=$(gh run view "$rid" --json jobs --jq '.jobs[] | "\(.conclusion // .status)\t\(.name)\t\(.startedAt[11:19])\t\(.completedAt[11:19])"' 2>/dev/null | sort)
  echo "$rows" | while IFS=$'\t' read -r c n a b; do
    a="${a:-}"; b="${b:-}"
    case "$c" in
      failure)   icon='❌';; success) icon='✅';; skipped) icon='⏭';;
      cancelled) icon='🚫';; queued) icon='⌛';; *) icon='⏳';;
    esac
    dur='—'
    if [ -n "$a" ] && [ -n "$b" ] && [ "$a" != "00:00:00" ] && [ "$b" != "00:00:00" ]; then dur="$a..$b"; fi
    printf '    %s %-9s %-42s %s\n' "$icon" "$c" "$n" "$dur"
  done
  fails=$(echo "$rows" | awk -F'\t' '$1=="failure"{print $2}')
  if [ -n "$fails" ]; then
    echo "    ── 失败原因（注解 ✓，**不必等整轮** ✓）──"
    while read -r n; do [ -n "$n" ] && { echo "      · $n"; ann "$rid" "$n"; }; done <<< "$fails"
  fi
}
REPO=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
mode="${1:-}"; 
case "$mode" in
  --run) show "$2";;
  --all) for r in $(gh run list --workflow ci --limit 5 --json databaseId --jq '.[].databaseId'); do show "$r"; done;;
  --follow)
    secs="${2:-20}"; last=""
    while :; do
      rid=$(gh run list --workflow ci --limit 1 --json databaseId --jq '.[0].databaseId')
      sig=$(gh run view "$rid" --json jobs --jq '[.jobs[] | "\(.conclusion // .status) \(.name)"] | sort | join("|")' 2>/dev/null)
      if [ "$sig" != "$last" ]; then printf '\n[%s] 变化 ✓\n' "$(date -u +%H:%M:%S)"; show "$rid"; last="$sig"; fi
      bad=$(gh run view "$rid" --json jobs --jq '[.jobs[] | select(.conclusion=="failure")] | length' 2>/dev/null || echo 0)
      if [ "${bad:-0}" -gt 0 ]; then printf '\n❌ 已有 %s 个 job 失败 ✗ —— 立刻退出 ✓（不等整轮 ✓）\n' "$bad"; exit 1; fi
      done_=$(gh run view "$rid" --json status --jq .status)
      [ "$done_" = "completed" ] && { echo "✓ 整轮结束 ✓（无失败 ✓）"; exit 0; }
      sleep "$secs"
    done;;
  *) rid=$(gh run list --workflow ci --limit 1 --json databaseId --jq '.[0].databaseId'); show "$rid"
     bad=$(gh run view "$rid" --json jobs --jq '[.jobs[] | select(.conclusion=="failure")] | length' 2>/dev/null || echo 0)
     [ "${bad:-0}" -gt 0 ] && { printf '\n  ⇒ ❌ 最新一轮 #%s 已有 %s 个 job 失败 ✗\n' "$rid" "$bad"; exit 1; }
     printf '\n  ⇒ ✓ 最新一轮 #%s 暂无失败 ✓\n' "$rid";;
esac
