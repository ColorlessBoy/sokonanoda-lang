#!/usr/bin/env bash
# ci-push.sh —— **推之前先关掉旧的没跑完的 run**（2026-09-25 用户要求 ✓）
#
# 用户原话 ✓："目前还在调整 github action 流程的情况下，**新的 github action 提交前，
# 把旧的没跑完的先关了**" ✓ —— 理由充分 ✓：旧的 run 量的是**已被取代的代码** ✗，
# 既占 runner 配额 ✓、又让队列更堵 ✗（本 session 里我连推 10 个 ⇒ 队列瘫痪 ✗）。
#
# 用法：
#   scripts/ci-push.sh                 # 关掉旧的未完成 run ⇒ `git push` ⇒ 打印新一轮 id
#   scripts/ci-push.sh --watch         # 推完接着 `ci-watch.sh --follow` ✓（**一红就退** ✓）
#   scripts/ci-push.sh --no-cancel     # 只推（**不推荐** ✗）
set -uo pipefail
watch=0; cancel=1; args=()
for a in "$@"; do
  case "$a" in
    --watch) watch=1;; --no-cancel) cancel=0;; *) args+=("$a");;
  esac
done
if [ "$cancel" = 1 ]; then
  n=0
  for rid in $(gh run list --workflow ci --limit 30 --json databaseId,status \
                 --jq '.[] | select(.status!="completed") | .databaseId'); do
    gh run cancel "$rid" >/dev/null 2>&1 && { echo "  🚫 关掉旧 run #$rid"; n=$((n+1)); }
  done
  [ "$n" = 0 ] && echo "  （没有未完成的旧 run ✓）"
fi
# ⚠ **必须按退出码判** ✓（本 session 第四次同类教训 ✗）：推送失败时**不许**报"新一轮" ✗。
# ⚠ 并且 `${args[@]:-origin main}` 会展开成**一个**参数 `"origin main"` ✗（实测 ✓：
# git 报 "repository does not exist" ✓）⇒ 空数组要**分开写** ✓。
if [ "${#args[@]}" -eq 0 ]; then
  if ! git push origin main; then echo "  ❌ 推送失败 ✗（没有产生新一轮 ✓）"; exit 1; fi
else
  if ! git push "${args[@]}"; then echo "  ❌ 推送失败 ✗"; exit 1; fi
fi
sleep 5
# **拿到 id 才打印** ✓（2026-09-25 round 234 修 ✗）：网络抖动时 `gh` 会失败 ✓，
# 旧写法仍打印 `⇒ 新一轮 # ✓`（**id 是空的** ✗）⇒ 看起来像成功、其实什么也没拿到 ✓。
# 与"没按退出码判"同源 ✗：这次是"没按**有没有拿到值**判" ✓。
rid=$(gh run list --workflow ci --limit 1 --json databaseId --jq '.[0].databaseId' 2>/dev/null || true)
if [ -n "${rid:-}" ]; then
  echo "  ⇒ 新一轮 #$rid ✓（用 scripts/ci-watch.sh --follow 看 ✓）"
else
  echo "  ⚠ 推送成功 ✓，但**取不到 run id** ✗（网络/限流 ✓）⇒ 稍后手动：gh run list --workflow ci --limit 1"
fi
[ "$watch" = 1 ] && exec scripts/ci-watch.sh --follow
