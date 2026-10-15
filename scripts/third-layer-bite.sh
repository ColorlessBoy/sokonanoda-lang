#!/usr/bin/env bash
# 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
#
# **第三层（e2e）与它的桥的"咬住证明"**（用户 2026-10-15【④】：
# "任何新守卫必须自带咬住证明 —— 必须有一条**已存在的坏输入**被它咬住"）。
#
# 台账：`docs/guard-bite.jsonl`（判据 `scripts/check-guard-bite.py --check --all`）。
#
# ## 咬住的是什么（**已存在的坏输入**，不是我新造的 ✓）
#
# * `e2e-bite` —— **HEAD（= 本轮基线 main）的 `Query::usable()`**：一处语法错
#   让 `goals`/`holes` 拒答 ⇒ LSP `soko/goals` 答空 ⇒ **Infoview 的声明列表
#   不回来** ✗。这就是用户 2026-10-15 亲报的那一格（原话见
#   `docs/ux-queue-events.jsonl` 的 `SUB-parse-error-kills-whole-file`）。
#   取法：`git show HEAD:crates/front/src/query/mod.rs`（**不**用 `git checkout --` ✗）。
# * `bridge-bite` —— **HEAD 的 `crates/cli/src/main.rs`**：那时**没有** `fixtures`
#   子命令 ⇒ 第三层（Node 侧）**拿不到生成器的输入面**，只能手写实例 ✗
#   （手写实例正是这条 bug 的病根：修 `SUB-parse-error-kills-whole-file` 时
#   只补了两条手写用例）。取法同上。
#
# ## 纪律
#
# * **改完必还原**（`trap` + 前后 sha256 各印一次）—— 证明跑完 `git status`
#   必须与跑之前**逐字节相同** ✓；
# * **退出码 = 被判据的退出码**（pass 期望 0 · bite 期望非 0）；
# * ⚠ **环境缺失（node_modules / 二进制）⇒ exit 3**，**绝不**让它冒充"咬住了" ✗
#   —— 所以 `pass` 那一半的 `expect_rc: 0` 会把缺环境**响亮地**判红 ✓。
#
# 用法：
#   scripts/third-layer-bite.sh e2e           # 正跑（期望 exit 0）
#   scripts/third-layer-bite.sh e2e-bite      # 咬住证明（期望 exit 1）
#   scripts/third-layer-bite.sh bridge-bite   # 桥的咬住证明（期望 exit 1）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 3

MODE="${1:-}"
GREP_NAME="${SOKO_THIRD_LAYER_GREP:-第三层}"
VSCODE_VERSION="${SOKO_VSCODE_TEST_VERSION:-1.138.0}"
PROFILE="${SOKO_THIRD_LAYER_PROFILE:-debug}"
LOG="${SOKO_THIRD_LAYER_LOG:-/tmp/soko-third-layer-bite.log}"

E2E_TEST="editor/vscode/src/test/extension.test.js"
CLI_MAIN="crates/cli/src/main.rs"
USABLE="crates/front/src/query/mod.rs"

# ── 坏版本从哪个 rev 取（**不能写 `HEAD`** ✗ —— 实测踩过）──────────────────────
#
# ⚠ **为什么不是 `HEAD`**（2026-10-16 实测的**假绿**）：修好之后 `HEAD` **就是修好的
# 那一版** ⇒ `git show HEAD:<f>` 取回的是**修好的文件** ⇒ 换版**逐字节无效** ⇒
# `bite` 命令 rc=0 ⇒ `check-guard-bite.py --all` 把这两条报成「**这条守卫咬不住**
# 它声称咬住的那条坏输入」✗（实测读数：两条 bite 都 rc=0）。
# ⇒ 坏版本必须是**修复之前**的那一版：`630c0869` = 本轮基线的 main（它已在 main
# 的历史里、**不会**因本轮 rebase 而消失 ✓，同 `check-attention-budget.py` 那条
# 证明钉 `--until c82097fe` 的先例 ✓）。
BAD_REV="${SOKO_THIRD_LAYER_BAD_REV:-630c0869}"

# ── 还原闸（**只有** bite 模式会动工作区）────────────────────────────────────
SAVED=""
SAVED_SHA=""
RESTORED=0
restore() {
  [ "$RESTORED" = 1 ] && return 0
  [ -z "$SAVED" ] && return 0
  cat "$SAVED" > "$SAVED_FILE" || return 1
  RESTORED=1
  local now
  now="$(shasum -a 256 "$SAVED_FILE" | cut -d' ' -f1)"
  if [ "$now" != "$SAVED_SHA" ]; then
    echo "error: 还原后 sha256 不一致（${SAVED_FILE}）：$SAVED_SHA → $now" >&2
    return 1
  fi
  echo "+ 还原 ✓ $SAVED_FILE sha256=${now:0:16}"
}
SAVED_FILE=""
trap 'restore' EXIT

# 把一个文件换成**已存在的坏版本**（`git show <rev>:<path>`），并记下当前内容以便还原。
swap_in_bad_version() {
  local rev="$1" path="$2"
  SAVED="$(mktemp)"
  cat "$path" > "$SAVED" || return 3
  SAVED_SHA="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  SAVED_FILE="$path"
  if ! git cat-file -e "$rev:$path" 2>/dev/null; then
    echo "error: 坏版本的 rev 取不到：$rev:$path ⇒ **不假装咬过** ✗（rev 漂了就把 SOKO_THIRD_LAYER_BAD_REV 指到修复前那一版）" >&2
    return 3
  fi
  git show "$rev:$path" > "$path" || return 3
  local bad_sha
  bad_sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  # ⚠ **空转闸**（2026-10-16 实测踩到的假绿）：坏版本若与工作区版本**逐字节相同**，
  # 这次"换版"什么都没换 ⇒ 证明必然 rc=0 ⇒ **判红**（不是"通过" ✗）。
  # 第一次就是拿 `HEAD` 当坏版本（提交之后 HEAD **就是**修好的那一版）⇒ 两条 bite
  # 都 rc=0，被 `check-guard-bite.py --all` 当场抓住 ✓。
  if [ "$bad_sha" = "$SAVED_SHA" ]; then
    echo "error: 「坏版本」与工作区版本**逐字节相同**（${rev} 已经是修好的那一版）⇒ 这条证明会**空转** ✗" >&2
    echo "       把 SOKO_THIRD_LAYER_BAD_REV 指到**修复之前**的那一版（默认 630c0869 = 本轮基线的 main）" >&2
    return 3
  fi
  echo "+ 换成坏版本 ${path}（${rev}）sha256=${bad_sha:0:16}"
  echo "  当前（工作区）版本 sha256=${SAVED_SHA:0:16}（跑完还原）"
}

need_node_modules() {
  if [ ! -d editor/vscode/node_modules ]; then
    echo "error: editor/vscode/node_modules 不在 ⇒ 跑不了真 VS Code（先 (cd editor/vscode && npm install)）" >&2
    return 3
  fi
}

build_and_stage() {
  echo "+ cargo build -p sokonanoda-lsp -p sokonanoda-cli --locked（profile=${PROFILE}）"
  if [ "$PROFILE" = release ]; then
    cargo build --release -p sokonanoda-lsp -p sokonanoda-cli --locked || return 3
  else
    cargo build -p sokonanoda-lsp -p sokonanoda-cli --locked || return 3
  fi
  echo "+ stage: editor/vscode/bin/<target>/"
  (cd editor/vscode && node scripts/stage-lsp.js --profile "$PROFILE") || return 3
}

run_e2e() {
  need_node_modules || return 3
  build_and_stage || return 3
  echo "+ e2e：SOKO_E2E_GREP=$GREP_NAME · VS Code ${VSCODE_VERSION}（日志 ${LOG}）"
  (cd editor/vscode && SOKO_E2E_GREP="$GREP_NAME" SOKO_VSCODE_TEST_VERSION="$VSCODE_VERSION" \
    npx vscode-test) > "$LOG" 2>&1
  local rc=$?
  echo "--- 读数（${LOG}）---"
  grep -E "第三层读数|passing|failing|AssertionError" "$LOG" | head -20
  echo "--- e2e rc=$rc ---"
  return $rc
}

case "$MODE" in
  e2e)
    run_e2e
    exit $?
    ;;
  e2e-bite)
    # 坏输入 = **HEAD 的 `usable()`**（用户 2026-10-15 亲报的那一格）。
    swap_in_bad_version "$BAD_REV" "$USABLE" || exit 3
    run_e2e
    rc=$?
    restore || exit 3
    exit $rc
    ;;
  bridge-bite)
    # 坏输入 = **HEAD 的 `main.rs`**（那时没有 `fixtures` 子命令 ⇒ 第三层拿不到输入面）。
    swap_in_bad_version "$BAD_REV" "$CLI_MAIN" || exit 3
    echo "+ cargo test -p sokonanoda-cli --test fixtures_dump"
    cargo test -p sokonanoda-cli --test fixtures_dump > "$LOG" 2>&1
    rc=$?
    grep -E "^test |test result|未知子命令" "$LOG" | head -10
    restore || exit 3
    exit $rc
    ;;
  *)
    echo "usage: scripts/third-layer-bite.sh {e2e|e2e-bite|bridge-bite}" >&2
    exit 2
    ;;
esac
