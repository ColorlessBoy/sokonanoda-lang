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
rid=$(gh run list --workflow ci --limit 1 --json databaseId --jq '.[0].databaseId')
echo "  ⇒ 新一轮 #$rid ✓（用 scripts/ci-watch.sh --follow 看 ✓）"
[ "$watch" = 1 ] && exec scripts/ci-watch.sh --follow
