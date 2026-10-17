#!/usr/bin/env bash
# agent-worktree.sh —— 并行 agent 的**工作树基建**（2026-10-12 用户：「4 个 subagent 并行，如果使用各自的
# 分支，应该冲突会小很多，但是需要调研一下如何在同一个电脑里，做到多个 workspace」⇒「你来做掉吧，
# 搭一下基建和 prompt」）。
#
# 仓库既有规矩（`scripts/check-multi-session.py`，含用户原话）就是答案：
#   **"正解不是加锁，是 worktree 隔离"** —— 每条会话一个工作树、**独立 target（别共享，cargo 会加锁串行）**、
#   共享台账**只在 main 上改** ✓。本脚本把这三条固化成子命令（`new`/`own`/`list`/`land`/`rm`），省得每轮手敲、也顺手防"worktree 残留"。
#
# 用法：
#   scripts/agent-worktree.sh new <id> [--scope <glob,…>]
#                                           # 建 .worktrees/<id> + 分支 agent/<id>（从 main 的 HEAD）
#                                           #   + 写**主人标记** `.worktrees/<id>/.owner`（洞
#                                           #   `WORKTREE-ISOLATION-NOT-ENFORCED` 的"声明"那一源 ✓）
#   scripts/agent-worktree.sh own <id> [--scope <glob,…> | --from-claim]
#                                           # 声明 / 更新这棵树主人的**范围**（判据 ③ 用它判
#                                           #   "本树里未提交的路径属不属于主人" ✓；`--from-claim`
#                                           #   从队列的**活跃认领**自动取 ✓）
#   scripts/agent-worktree.sh list          # 列出工作树 / **活没进 main** 的 agent 分支 / 每棵树干不干净
#                                           #   + 每棵树的**主人与声明范围**（判据 ③ 的输入 ✓）
#   scripts/agent-worktree.sh land <id>     # **只能在主工作树的 main 上跑**：rebase + 快进合回
#                                           #   + **自动回填收口指针并提交**（rebase 孪生，见
#                                           #   `repoint_after_land`：不靠"记得回填" ✓）
#                                           #   ⚠ 树脏时**点名**哪些路径不属于主人（判据 ③ ✓）
#   scripts/agent-worktree.sh rm <id>       # 清理（默认拒绝**活没进 main** 的分支；--force 才强删）
#
# ⭐ **"每条会话一个 worktree"从约定变成机制**（洞 `WORKTREE-ISOLATION-NOT-ENFORCED`，high，
#   2026-10-17 · **kh1 实测** ✗→✓）：实测 `.worktrees/kh1` 里躺着 **20 处不属于它的
#   `crates/front/**`**（"拆文件 + 新模块"的形状 ✗）。修前：`--claim` 管**队列条目**、
#   本脚本管**目录**，但**没有任何东西强制两者对上** ✗。修后三个消费者**同一个谓词**
#   （`scripts/check-multi-session.py` 判据 ③ —— **一处定义** ✓）：
#     · `new` / `own` 写**声明**（本文件）→ `.owner`；
#     · 门禁 / `land` / **pre-commit 钩子**（`--staged`）逐条判"这路径属于这棵树的主人吗" ✓。
#   ⚠ 判据的**边界**（如实写，不是密封 ✗）：它判的是**未提交/暂存**的路径 ⇒ 必须跑在
#     **提交之前**；一旦别人的活被提交进本分支，它就变成了"本分支自己的证据" ✗。
#
# ⚠ **「合没合回」的判据 = `git cherry <BASE> <branch>` 的 `+` 行**（patch-id 等价），**不是祖先性** ✗：
#   `land` 走 **rebase + 快进**、而 `main` **自己也会被 rebase** ⇒ 已 land 的分支顶端是**孤儿 commit**
#   （活已进 main、hash 换了）⇒ 祖先性把**陈旧**误判成「未合回」，与发版栅栏互相锁死 ✗。
#   **一处定义** = 下面的 `branch_missing_commits()`（与 `scripts/ux-queue.py` 同名函数**同一件仪器** ✓）；
#   `git cherry` 非零 = **量不出来** ⇒ 一律按「未合回」处理（`None ≠ []` ✗ 别静默放行）。
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

# ── **主人标记**（`.worktrees/<id>/.owner`；洞 `WORKTREE-ISOLATION-NOT-ENFORCED`）──────
# ⚠ 这个文件是**判据 ③ 的输入**（`scripts/check-multi-session.py` 读它）—— 格式改动要**两边
#   一起改**（`read_owner_marker()` 是读者 ✓；字段：`id` / `branch` / `base` / `ts` / `scope`）。
# ⚠ 它**不是**活（`check-multi-session.py` 显式把它从"未提交路径"里排掉 ✓）：真仓里
#   `.worktrees/` 被 `.gitignore:53` 忽略 ⇒ 它根本不进 `git status` ✓。
OWNER_MARKER=".owner"

# marker_write <tree> <id> <branch> <base> <scope-csv> —— 写标记（python3 做 JSON 转义 ✓）。
# 返回 0 = 写了；3 = 没有 python3（**响亮地降级**：new 不该因为一个可选能力就失败 ✗，
# 但"没标记"这件事必须说出来 —— 判据 ③ 在那棵树上会落进「判不了」✓）。
marker_write() {
  local tree=${1:?} id=${2:?} branch=${3:?} base=${4:?} scope=${5:-}
  command -v python3 >/dev/null 2>&1 || return 3
  python3 - "$tree/$OWNER_MARKER" "$id" "$branch" "$base" "$scope" <<'PY' || return 3
import json, sys, datetime
path, ident, branch, base, scope = sys.argv[1:6]
data = {"id": ident, "branch": branch, "base": base,
        "ts": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "scope": [s for s in scope.split(",") if s.strip()]}
with open(path, "w", encoding="utf-8") as fh:
    json.dump(data, fh, ensure_ascii=False)
    fh.write("\n")
PY
}

# marker_scope <tree> —— 印出标记里的 `scope`（逗号分隔）；没有标记/没 python3 ⇒ 空 ✓。
marker_scope() {
  local tree=${1:?} f="$1/$OWNER_MARKER"
  [ -f "$f" ] || return 0
  command -v python3 >/dev/null 2>&1 || return 0
  python3 - "$f" <<'PY' 2>/dev/null || true
import json, sys
try:
    data = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    sys.exit(0)
print(",".join(str(s) for s in (data.get("scope") or [])))
PY
}

# live_scope <tree> —— 这棵树主人的**活跃认领资源**（判据 ③ 的"认领"源；**一处定义**：
# 走 `check-multi-session.py --print-live-scope`，不在这里重写一遍归一化 ✗）。
live_scope() {
  local tree=${1:?}
  command -v python3 >/dev/null 2>&1 || return 3
  # ⚠ **不吞 stderr**：判据自己会给"量不出来"的人话（没有认领 / 不是链接工作树）⇒ 让它说 ✓。
  ( cd "$tree" && python3 "$ROOT/scripts/check-multi-session.py" --print-live-scope )
}

cmd_own() {
  local id=${1:?用法: agent-worktree.sh own <id> [--scope <glob,…> | --from-claim]}
  shift
  local path="$WT_DIR/$id" scope="" from_claim=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --scope) scope=${2:?--scope 要一个值（逗号分隔的 glob）}; shift 2 ;;
      --from-claim) from_claim=1; shift ;;
      *) die "own 不认这个参数：$1（用法见文件头）" ;;
    esac
  done
  [ -d "$path" ] || die "没有这棵树：${path}（先 new ${id}，或核对 id）"
  if [ "$from_claim" = 1 ]; then
    [ -z "$scope" ] || die "--scope 与 --from-claim 二选一 ✗"
    scope=$(live_scope "$path") || die "取不到活跃认领（上面那条就是原因：没有认领 / 没有 python3 / 判据不在）⇒ 用 --scope 手写 ✓"
    [ -n "$scope" ] || die "这棵树的主人（${id}）**没有活跃认领** ⇒ 先 --claim 一条队列条目，或 --scope 手写 ✗"
  fi
  [ -n "$scope" ] || die "own 要一个范围：--scope 'crates/xxx/**,…' 或 --from-claim ✗"
  local branch base
  branch=$(git -C "$path" branch --show-current 2>/dev/null || true)
  base=$BASE
  if ! marker_write "$path" "$id" "${branch:-agent/$id}" "$base" "$scope"; then
    die "写不了主人标记（没有 python3？）⇒ ${path}/${OWNER_MARKER} 没写成"
  fi
  info "✓ ${path}/${OWNER_MARKER}：主人 ${id} · 范围 ${scope}"
  # ⚠ **反引号在双引号里 = 命令替换**（本文件 `cmd_list` 的注释记过同一个坑 ✗；夹具实测：
  #   这一行会把判据**当场跑一遍**、消息还被吃掉一半 ✗）⇒ 一律转义 ✓。
  info "  ⇒ 之后 \`python3 scripts/check-multi-session.py --ownership\`（在树里跑）会逐条判"
  info "     \"这棵树里未提交的路径属不属于 ${id}\" ✓；pre-commit 钩子在**提交前**也判一次（--staged ✓）"
}

cmd_new() {
  local id=${1:?用法: agent-worktree.sh new <id> [--scope <glob,…>]}
  shift
  local scope="" branch path
  while [ $# -gt 0 ]; do
    case "$1" in
      --scope) scope=${2:?--scope 要一个值（逗号分隔的 glob）}; shift 2 ;;
      *) die "new 不认这个参数：$1（用法见文件头）" ;;
    esac
  done
  branch="agent/$id"; path="$WT_DIR/$id"
  in_main || die "已经在链接工作树里 ⇒ 别再建（每条会话一棵 ✓）。要建就在主 checkout 上跑。"
  [ -e "${path}" ] && die "${path} 已存在（要重建先 rm ${id}）"
  git -C "$ROOT" show-ref --verify --quiet "refs/heads/${branch}" && die "分支 ${branch} 已存在 ⇒ 换个 id，或先 land/rm"
  mkdir -p "$WT_DIR"
  git -C "$ROOT" worktree add -b "$branch" "$path" "$BASE" >/dev/null || die "worktree add 失败"
  info "✓ 建好 ${path}（分支 ${branch}，基线 ${BASE}@$(git -C "$ROOT" rev-parse --short "$BASE")）"
  # ⭐ **写主人标记**（判据 ③ 的"声明"源 ✓）：不写也照样能建（向后兼容 ✓），但那样这棵树
  #   在判据 ③ 里会落进「判不了」⇒ 这里**如实印出来** + 给一条补的命令 ✓。
  if marker_write "$path" "$id" "$branch" "$BASE" "$scope"; then
    if [ -n "$scope" ]; then
      info "✓ 主人标记：${path}/${OWNER_MARKER}（主人 ${id} · 范围 ${scope}）"
    else
      info "ⓘ 主人标记：${path}/${OWNER_MARKER}（主人 ${id} · 范围**未声明**）"
      info "  ⇒ 判据 ③（本树里未提交的路径属不属于你）要靠它才判得动 —— 知道模块后补一条："
      info "     scripts/agent-worktree.sh own ${id} --scope 'crates/xxx/**,…'   # 或 --from-claim ✓"
    fi
  else
    info "⚠ 没有 python3 ⇒ **没写主人标记** ${path}/${OWNER_MARKER}"
    info "  ⇒ 这棵树在判据 ③ 里会落进「判不了」（不是绿 ✗）；装上 python3 后补：own ${id} --scope '…'"
  fi
  info ""
  info "进工作树前先把受管环境 source 进来（**独立 target · 共享 sccache · 队列单点** ✓）："
  info "  eval \"\$(python3 \"$ROOT/scripts/compile-cache.py\" env ${id})\"   # 由编译缓存总管生成 ✓"
  info "  （没装 sccache 也能跑：那一行会留空 ⇒ 优雅降级 ✓；预算/回收见 scripts/compile-cache.py）"
  info ""
  info "开工：cd ${path} && python3 \"\$SOKO_QUEUE_ROOT/scripts/ux-queue.py\" --next"
  info "落地：主 checkout 上跑 scripts/agent-worktree.sh land ${id}"
}

# branch_missing_commits <branch> —— 打印该分支上**活还没进 $BASE** 的提交 sha（每行一个）。
# 判据 = `git cherry "$BASE" <branch>` 的 `+` 行（patch-id 等价）——与
# `scripts/ux-queue.py:branch_missing_commits()` **同一件仪器** ✓；**一处定义**（cmd_list 与 cmd_rm 都调它 ✓）。
# 返回 0 = 量出来了（可能一行都没有 ⇒ 活已全进 $BASE ✓）；**1 = 量不出来**（`git cherry` 非零）。
# ⚠ `None ≠ []`：调用方**必须**把 1 当「未合回」（拒删 / 点名）——绝不许读成「已合回」✗。
# ⚠ 边界（与 ux-queue.py 同口径；夹具实测 /tmp/cleanJ-fixture）：空分支（tip == $BASE）⇒ 空输出 ✓；
#   **空提交**（`--allow-empty`，patch-id 为空）与**分支上的 merge 提交** ⇒ 恒 `+` ⇒ 判「未合回」
#   ——这是**保守方向**（判「未合回」≠ 放行），故**不**过滤 ✗。
branch_missing_commits() {
  local branch=${1:?用法: branch_missing_commits <branch>} out
  out=$(git -C "$ROOT" cherry "$BASE" "$branch" 2>/dev/null) || return 1
  printf '%s\n' "$out" | awk '/^\+/{print $2}'
  return 0
}

cmd_list() {
  info "── 工作树 ──"
  git -C "$ROOT" worktree list
  info ""
  info "── 活没进 $BASE 的 agent 分支（判据 = git cherry $BASE <b> 的 + 行 = patch-id 等价，**不是祖先性** ✗）──"
  local b missing bad=0
  while IFS= read -r b; do
    [ -n "$b" ] || continue
    if ! missing=$(branch_missing_commits "$b"); then
      info "  ⚠ ${b}：量不出来（git cherry 非零 ⇒ 不敢读成『已合回』 ✗）"
      bad=$((bad + 1)); continue
    fi
    [ -n "$missing" ] || continue
    info "  ${b}（$(printf '%s\n' "$missing" | wc -l | tr -d ' ') 个提交的改动不在 $BASE 上）"
    bad=$((bad + 1))
  done < <(git -C "$ROOT" branch --list 'agent/*' --format='%(refname:short)')
  [ "$bad" = 0 ] && info "（无 ✓）"
  info ""
  # ⚠ 这行原来是 `（\`*\` = 有未提交改动）`：反引号在双引号里 = **命令替换** ⇒ `list` 每次都在
  #   stderr 报 `f: command not found`、标题渲染成「（ = 有未提交改动）」✗；且 `git worktree list`
  #   与 porcelain **从不打 `*`**（实测 grep = 0）⇒ 那个承诺本来就是假的。改成如实描述。
  info "── 各树脏不脏（下面逐棵点名未提交改动）──"
  git -C "$ROOT" worktree list --porcelain | awk '/^worktree /{p=$2} /^branch /{b=$2} /^$/{if(p)printf "  %-58s %s\n", p, b; p=""}'
  while IFS= read -r w; do
    [ -d "$w" ] || continue
    local n; n=$(git -C "$w" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
    [ "$n" != "0" ] && info "  ⚠ ${w} 有 ${n} 处未提交改动"
  done < <(git -C "$ROOT" worktree list --porcelain | awk '/^worktree /{print $2}')
  # ⭐ **每棵树的主人与声明范围**（洞 `WORKTREE-ISOLATION-NOT-ENFORCED` 的"声明"源 ✓）：
  #   ⚠ 只对**有标记**的树跑判据 ③（其余 60 棵老树没有标记 ⇒ 逐棵跑 python3 是白付 ✗）；
  #   ⚠ 判据只**读**、从不写 ✓。
  info ""
  info "── 主人标记与归属（判据 ③ = 本树里未提交的路径属不属于主人；只对**有标记**的树跑 ✓）──"
  local wt id scope out rc shown=0
  while IFS= read -r w; do
    [ -d "$w" ] || continue
    case "$w" in */.worktrees/*) ;; *) continue ;; esac
    id=${w##*/}
    scope=$(marker_scope "$w")
    if [ ! -f "$w/$OWNER_MARKER" ]; then
      info "  ⓘ ${id}：**没有主人标记** ⇒ 判据 ③ 在它上面落进「判不了」（不是绿 ✗）—— 补：own ${id} --scope '…'"
      continue
    fi
    shown=$((shown + 1))
    if [ -z "$scope" ]; then
      info "  ⚠ ${id}：标记在，但**范围未声明** ⇒ 判据 ③ 判不动 —— 补：own ${id} --scope '…'（或 --from-claim）"
      continue
    fi
    out=$( cd "$w" && python3 "$ROOT/scripts/check-multi-session.py" --ownership 2>&1 ); rc=$?
    if [ "$rc" = 1 ]; then
      info "  ✗ ${id}：主人 ${id} · 范围 ${scope} ⇒ **有未提交路径不属于它** ✗（上面判据已逐条点名）"
      printf '%s\n' "$out" | sed -n 's/^  ✗ ③ 本树主人/      /p' | head -5
    elif [ "$rc" = 0 ]; then
      info "  ✓ ${id}：主人 ${id} · 范围 ${scope} ⇒ 未提交路径都在范围内 ✓"
    else
      info "  ⚠ ${id}：判据 ③ 量不出来（rc=${rc}）⇒ **不是绿** ✗"
    fi
  done < <(git -C "$ROOT" worktree list --porcelain | awk '/^worktree /{print $2}')
  [ "$shown" = 0 ] && info "  （还没有任何一棵树有声明范围 —— 用 new --scope / own <id> --scope 补 ✓）"
  true
}

# repoint_after_land <id> —— `land` 的**收尾一步**：把 rebase 甩掉的收口指针机械回填 + 提交。
#
# ⚠ **为什么必须有这一步**（`LAND-REBASE-STALES-CLOSURE-SHAS`，2026-10-15 · 结构性 ✗）：
#   本脚本的 `land` 用 **rebase** 把分支提交重写成 `$BASE` 上的**孪生** ⇒ 任何"**落 main 前**"
#   记的收口 sha（最自然的一笔就是"我的工作 = 分支 tip"）在 main 的历史上**核不到**：
#     · 本地（对象还在 reflog 里）⇒ `git merge-base --is-ancestor` **rc=1**；
#     · CI（`git clone --single-branch --branch main` ⇒ 对象集 = main 可达）⇒ **rc=128**
#       （`fatal: Not a valid commit name`）⇒ `ux-queue.py --check` 判红 ⇒ `queue-lint` 红 ✗。
#   ⇒ 而"记得回填"是**靠不住的**（同一形状重犯多次）：**这里把它变成机械动作** ✓。
#   ⚠ **顺序不许反** ✗：`--repoint-all` 只搬得动**非祖先**的现行指针 —— 在 worktree 里
#     （`HEAD` = 分支 tip）那些 sha 还是**祖先** ⇒ 它如实报「没有非祖先的（0 条）」
#     （实测：`land` 前 0 条 / `land` 后 1 条并找到孪生）⇒ 所以**只能在 rebase + 快进之后**跑。
#   ⚠ **回填完必须提交** ✗：`--check`（含 `scripts/soko gate --fast` 跑的那条）读的是
#     **工作区**，而 push 送的是 **commit** ⇒ 只回填不提交 = 本地绿、CI 红（**两层遮蔽**，
#     实测：工作区 `--check` rc=0 而 HEAD 的树里仍是旧 sha）⇒ 提交是这条判据的一半 ✓。
#   ⚠ **找不到孪生 ⇒ 响亮地失败**（`--repoint-all` 自己就 rc=1，见它的"不硬填"）：
#     `$BASE` 已经前进、回不去 ⇒ 只**如实报 + 给路**，绝不静默放过（那就是又一次假绿 ✗）。
# 返回 0 = 账已回填（有改动时也已提交）；1 = 回填失败；3 = 环境缺 python3（**绝不静默跳过** ✗）。
repoint_after_land() {
  local id=${1:?用法: repoint_after_land <id>}
  local q="$ROOT/scripts/ux-queue.py" rel="docs/ux-queue-events.jsonl"
  local reason="land ${id} 后自动回填：rebase 把分支提交重写成 ${BASE} 上的孪生"
  [ -f "$q" ] || { info "ⓘ 没有 ${q} ⇒ 跳过回填（本仓没有队列）"; return 0; }
  command -v python3 >/dev/null 2>&1 || {
    printf '%s\n' "✗ land：${BASE} 已合回，但**没有 python3** ⇒ 收口指针**没回填**（绝不静默跳过 ✗）" \
      "  ⇒ 装好 python3 后立刻补跑：SOKO_QUEUE_ROOT=${ROOT} python3 ${q} --repoint-all --reason '${reason}'" \
      "     然后**提交** ${rel}（不提交 = 本地绿、CI 红 ✗）" >&2
    return 3
  }
  local before after out rc n
  before=$(wc -l < "$ROOT/$rel" 2>/dev/null | tr -d ' '); before=${before:-0}
  info "→ 回填收口指针（rebase 孪生）：python3 scripts/ux-queue.py --repoint-all"
  out=$(cd "$ROOT" && SOKO_QUEUE_ROOT="$ROOT" python3 "$q" --repoint-all --reason "$reason" 2>&1); rc=$?
  printf '%s\n' "$out"
  if [ "$rc" != 0 ]; then
    printf '%s\n' "✗ land：${BASE} **已经合回**（回不去），但收口指针**没回填到 0**（上面那几条）✗" \
      "  ⇒ 现状 = 账上有**核不到**的收口指针（CI 里 rc=128 ⇒ queue-lint 红）—— 出路见上面每条" \
      "     自己的指示（`--repoint` 显式孪生 / `--triage`+`--advance` 按真落进 ${BASE} 的 commit 收口）✓" >&2
    return 1
  fi
  after=$(wc -l < "$ROOT/$rel" 2>/dev/null | tr -d ' '); after=${after:-0}
  n=$((after - before))
  if [ "$n" -gt 0 ]; then
    local m
    m=$(git -C "$ROOT" diff HEAD --numstat -- "$rel" | awk '{s+=$1} END{print s+0}')
    git -C "$ROOT" commit -q -m "chore(queue): land ${id} 后自动回填收口指针（rebase 孪生）—— 本次 +${n} 条 repoint（事件表相对 HEAD 共 +${m} 行）" -- "$rel" \
      || { printf '%s\n' "✗ land：回填已写进工作区，但**提交失败**（${rel}）⇒ 现在正是『本地绿、CI 红』的形状 ✗ —— 手工提交它" >&2; return 1; }
    info "✓ 回填已提交：$(git -C "$ROOT" log --oneline -1)"
  else
    info "✓ 收口指针：本次**没有**要回填的（0 条）"
    local pending
    pending=$(git -C "$ROOT" status --porcelain -- "$rel" | wc -l | tr -d ' ')
    [ "$pending" != 0 ] && info "  ⚠ 但 ${rel} 还有**未提交**的账（不是本次回填）⇒ 推送前记得提交（工作区绿 ≠ commit 绿 ✗）"
  fi
  return 0
}

cmd_land() {
  local id=${1:?用法: agent-worktree.sh land <id>}
  local branch="agent/$id" path="$WT_DIR/$id"
  in_main || die "land 只能在**主工作树**上跑（合并是单点操作 ✓）"
  [ "$(git -C "$ROOT" branch --show-current)" = "$BASE" ] || die "主工作树当前不在 $BASE 上 ⇒ 先切回来"
  git -C "$ROOT" show-ref --verify --quiet "refs/heads/${branch}" || die "没有分支 ${branch}"
  if [ -d "$path" ]; then
    local n; n=$(git -C "$path" status --porcelain | wc -l | tr -d ' ')
    if [ "$n" != "0" ]; then
      # ⭐ **拒绝信息必须可操作**（洞 `WORKTREE-ISOLATION-NOT-ENFORCED` 的 (b) ✓）：修前只说
      #   "还有 N 处未提交 ⇒ 先让那个 agent 提交干净" ✗ —— 它**不说哪些是别人的** ✗，
      #   而"哪些是别人的"正是编排者要据以决定"别把它卷进去"的那条信息 ✓。
      #   ⚠ 这里跑的是**判据 ③**（`--ownership`，**一处定义** ✓）—— 它只**读**这棵树 ✓。
      local own_out own_rc
      own_out=$( cd "$path" && python3 "$ROOT/scripts/check-multi-session.py" --ownership 2>&1 ); own_rc=$?
      if [ "$own_rc" = 1 ]; then
        printf '%s\n' "$own_out" >&2
        # ⚠ **反引号在双引号里 = 命令替换**（本文件 `cmd_list` 那条注释记过同一个坑 ✗）：
        #   `die "… `git add -A` …"` 会被 shell 执行 ⇒ 消息被吃掉一半（夹具实测 ✗→✓）
        #   ⇒ 这里的反引号一律**转义** ✓。
        die "${path} 有 ${n} 处未提交改动，其中**有点名的不属于本树主人**（上面 ✗ 那几条）✗ ⇒ 别替它 stash、**别 \`git add -A\`**（别人的活会被卷进提交 ✗）：让那个 agent 自己处置 —— 自己的提交干净；**别人的路径原样留着并报告**（别删/别 revert ✗）"
      fi
      if [ "$own_rc" != 0 ]; then
        printf '%s\n' "$own_out" >&2
        die "${path} 还有 ${n} 处未提交改动，而判据 ③ **量不出来**（rc=${own_rc}）⇒ **不是绿** ✗：先让那个 agent 提交干净（别替它 stash ✗），或给它补一条声明（own ${id} --scope '…'）"
      fi
      printf '%s\n' "$own_out" >&2
      die "${path} 还有 ${n} 处未提交改动（都在主人 ${id} 声明的范围里 ✓）⇒ 先让那个 agent 提交干净（别替它 stash ✗）"
    fi
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
  # ⭐ rebase 已经把分支提交重写掉 ⇒ **紧接着**把收口指针回填 + 提交（见函数头：这是本脚本
  #   唯一能保证"land 之后立刻 --check=0 且工作区干净"的地方 ✓）。
  repoint_after_land "$id" || return $?
  info "  收尾：scripts/agent-worktree.sh rm ${id}"
}

cmd_rm() {
  local id=${1:?用法: agent-worktree.sh rm <id>}
  local branch="agent/$id" path="$WT_DIR/$id" force=${2:-}
  # 安全闸：判「活有没有进 $BASE」（`git cherry` 的 `+` 行 = patch-id 等价），**不是祖先性** ✗。
  # ⚠ 换判据后**只松开一处**：rebase 孪生（活已进 main、hash 换了）不再被拒 ✓；
  #   **真有未合回提交**的分支**照样拒** ✓；**量不出来**（git cherry 非零）**也拒** ✓（`None ≠ []`）。
  local missing n
  if ! missing=$(branch_missing_commits "$branch"); then
    if [ "$force" = "--force" ]; then
      info "⚠ 量不出 ${branch} 有没有未合回的提交（git cherry ${BASE} ${branch} 非零：分支不在或仓库坏了）——按 --force 继续（**这是你按的旁路，不是判据说『已合回』** ✗）"
      missing=""
    else
      die "量不出 ${branch} 有没有未合回的提交（git cherry ${BASE} ${branch} 非零：分支不在或仓库坏了）⇒ **拒删**（量不出来 ≠ 已合回 ✗）；确认后加 --force"
    fi
  fi
  if [ -n "$missing" ] && [ "$force" != "--force" ]; then
    n=$(printf '%s\n' "$missing" | wc -l | tr -d ' ')
    die "${branch} 还有 ${n} 个提交的改动不在 ${BASE} 上（判据 = git cherry ${BASE} ${branch} 的 + 行 = patch-id 等价，**不是祖先性**）⇒ 要么 land，要么 --force（**真丢这 ${n} 个提交** ✗）"
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
  own) shift; cmd_own "$@" ;;
  list) shift; cmd_list "$@" ;;
  land) shift; cmd_land "$@" ;;
  rm) shift; cmd_rm "$@" ;;
  # 用法展示 = 文件头的整块注释（到第一个空行为止 ⇒ 以后加注释行不会再被截断 ✗）
  *) sed -n '3,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
