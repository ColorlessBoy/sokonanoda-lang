#!/usr/bin/env bash
# agent-worktree.sh —— 并行 agent 的**工作树基建**（2026-10-12 用户：「4 个 subagent 并行，如果使用各自的
# 分支，应该冲突会小很多，但是需要调研一下如何在同一个电脑里，做到多个 workspace」⇒「你来做掉吧，
# 搭一下基建和 prompt」）。
#
# 仓库既有规矩（`scripts/check-multi-session.py`，含用户原话）就是答案：
#   **"正解不是加锁，是 worktree 隔离"** —— 每条会话一个工作树、**独立 target（别共享，cargo 会加锁串行）**、
#   共享台账**只在 main 上改** ✓。本脚本把这三条固化成四个子命令，省得每轮手敲、也顺手防"worktree 残留"。
#
# 用法：
#   scripts/agent-worktree.sh new <id>      # 建 .worktrees/<id> + 分支 agent/<id>（从 main 的 HEAD）
#   scripts/agent-worktree.sh list          # 列出工作树 / 未合回的 agent 分支 / 每棵树干不干净
#   scripts/agent-worktree.sh land <id>     # **只能在主工作树的 main 上跑**：rebase + 快进合回
#   scripts/agent-worktree.sh rm <id>       # 清理（默认拒绝未合回的分支；--force 才强删）
#
# 环境（`new` 会把这几行打出来，进 worktree 前 source 一下）：
#   CARGO_TARGET_DIR=<worktree>/target            # **独立**：共享会被 cargo 加锁串行 ✗
#   SOKONANODA_CACHE_DIR=<worktree>/.soko-cache   # 独立缓存（与 G-112 同族的坑）
#   SOKO_QUEUE_ROOT=<主 checkout>                 # **队列状态单点**：事件/台账只在主 checkout 上写 ✓

set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
WT_DIR="${SOKO_WORKTREE_DIR:-$ROOT/.worktrees}"
BASE="${SOKO_BASE_BRANCH:-main}"

die() { printf '%s\n' "✗ $*" >&2; exit 1; }
info() { printf '%s\n' "$*"; }

in_main() { [ "$(git -C "$ROOT" rev-parse --git-dir)" = "$(git -C "$ROOT" rev-parse --git-common-dir)" ]; }

cmd_new() {
  local id=${1:?用法: agent-worktree.sh new <id>}
  local branch="agent/$id" path="$WT_DIR/$id"
  in_main || die "已经在链接工作树里 ⇒ 别再建（每条会话一棵 ✓）。要建就在主 checkout 上跑。"
  [ -e "${path}" ] && die "${path} 已存在（要重建先 rm ${id}）"
  git -C "$ROOT" show-ref --verify --quiet "refs/heads/${branch}" && die "分支 ${branch} 已存在 ⇒ 换个 id，或先 land/rm"
  mkdir -p "$WT_DIR"
  git -C "$ROOT" worktree add -b "$branch" "$path" "$BASE" >/dev/null || die "worktree add 失败"
  info "✓ 建好 ${path}（分支 ${branch}，基线 ${BASE}@$(git -C "$ROOT" rev-parse --short "$BASE")）"
  info ""
  info "进工作树前先把受管环境 source 进来（**独立 target · 共享 sccache · 队列单点** ✓）："
  info "  eval \"\$(python3 \"$ROOT/scripts/compile-cache.py\" env ${id})\"   # 由编译缓存总管生成 ✓"
  info "  （没装 sccache 也能跑：那一行会留空 ⇒ 优雅降级 ✓；预算/回收见 scripts/compile-cache.py）"
  info ""
  info "开工：cd ${path} && python3 \"\$SOKO_QUEUE_ROOT/scripts/ux-queue.py\" --next"
  info "落地：主 checkout 上跑 scripts/agent-worktree.sh land ${id}"
}

cmd_list() {
  info "── 工作树 ──"
  git -C "$ROOT" worktree list
  info ""
  info "── 未合回 $BASE 的 agent 分支 ──"
  local unmerged
  unmerged=$(git -C "$ROOT" branch --no-merged "$BASE" --list 'agent/*' --format='%(refname:short)')
  if [ -z "$unmerged" ]; then info "（无 ✓）"; else printf '  %s\n' $unmerged; fi
  info ""
  info "── 各树脏不脏（`*` = 有未提交改动）──"
  git -C "$ROOT" worktree list --porcelain | awk '/^worktree /{p=$2} /^branch /{b=$2} /^$/{if(p)printf "  %-58s %s\n", p, b; p=""}'
  while IFS= read -r w; do
    [ -d "$w" ] || continue
    local n; n=$(git -C "$w" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
    [ "$n" != "0" ] && info "  ⚠ ${w} 有 ${n} 处未提交改动"
  done < <(git -C "$ROOT" worktree list --porcelain | awk '/^worktree /{print $2}')
  true
}

cmd_land() {
  local id=${1:?用法: agent-worktree.sh land <id>}
  local branch="agent/$id" path="$WT_DIR/$id"
  in_main || die "land 只能在**主工作树**上跑（合并是单点操作 ✓）"
  [ "$(git -C "$ROOT" branch --show-current)" = "$BASE" ] || die "主工作树当前不在 $BASE 上 ⇒ 先切回来"
  git -C "$ROOT" show-ref --verify --quiet "refs/heads/${branch}" || die "没有分支 ${branch}"
  if [ -d "$path" ]; then
    local n; n=$(git -C "$path" status --porcelain | wc -l | tr -d ' ')
    [ "$n" != "0" ] && die "${path} 还有 ${n} 处未提交改动 ⇒ 先让那个 agent 提交干净（别替它 stash ✗）"
  fi
  info "→ 在 ${path} 里把 ${branch} rebase 到 ${BASE}（**分支被 worktree 占着 ⇒ 必须在它自己那棵树里 rebase** ✗）"
  if [ -d "$path" ]; then
    git -C "$path" rebase "$BASE" || die "rebase 冲突 ⇒ 在 ${path} 里解决后 git rebase --continue，再重跑 land"
  else
    git -C "$ROOT" rebase "$BASE" "$branch" || die "rebase 冲突（树已不在，手工解决）"
  fi
  info "→ 快进合回 ${BASE}"
  git -C "$ROOT" merge --ff-only "$branch" || die "快进失败（$BASE 动了？重跑 land）"
  info "✓ 已合回：$(git -C "$ROOT" log --oneline -1)"
  info "  收尾：scripts/agent-worktree.sh rm ${id}"
}

cmd_rm() {
  local id=${1:?用法: agent-worktree.sh rm <id>}
  local branch="agent/$id" path="$WT_DIR/$id" force=${2:-}
  local merged
  merged=$(git -C "$ROOT" branch --merged "$BASE" --list "$branch" --format='%(refname:short)')
  if [ -z "$merged" ] && [ "$force" != "--force" ]; then
    die "${branch} 还没合回 ${BASE} ⇒ 要么 land，要么 --force（**会丢提交** ✗）"
  fi
  if [ -d "$path" ]; then
    git -C "$ROOT" worktree remove "$path" ${force:+--force} || die "worktree remove 失败（有未提交改动？加 --force）"
  fi
  git -C "$ROOT" worktree prune
  git -C "$ROOT" branch -D "$branch" >/dev/null 2>&1 || true
  info "✓ 清掉 $path 与分支 ${branch}（worktree 残留是已知坑：Claude Code 也报过「成功但没删」 ✗）"
}

case "${1:-}" in
  new) shift; cmd_new "$@" ;;
  list) shift; cmd_list "$@" ;;
  land) shift; cmd_land "$@" ;;
  rm) shift; cmd_rm "$@" ;;
  *) sed -n '3,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
