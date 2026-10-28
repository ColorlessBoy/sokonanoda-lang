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
#   取法：**只**把 `Query::usable()` 的主体换回 `630c0869` 那一版（逐字取自
#   `git show 630c0869:crates/front/src/query/mod.rs`）—— **不再**整文件换版，
#   原因见 `inject_old_usable()` 的注释（2026-10-28 第三次编译断）。
# * `bridge-bite` —— **HEAD 的 `crates/cli/src/main.rs`**：那时**没有** `fixtures`
#   子命令 ⇒ 第三层（Node 侧）**拿不到生成器的输入面**，只能手写实例 ✗
#   （手写实例正是这条 bug 的病根：修 `SUB-parse-error-kills-whole-file` 时
#   只补了两条手写用例）。取法同上。
# * `families-bite`（`E2E-COVERAGE-4-OF-36`）—— **`8452b5b2` 的父提交
#   `638fb911`** 的 `crates/front/src/parser.rs` + `project/graph.rs`：那时
#   `by` 块里的**参数表跨行吃名字**（`intro` / `unfold` / `unfold … at` 三处
#   只看 token 类型、不看列）⇒ 下一行**顶格**的命令关键字被吃成 binder ⇒
#   未关块**后面**那条命令**整行不见** ✗（实测：`process-state/intro/theorem/blank`
#   的 wire 上只剩 `[a]`，而生成器要求 `judged >= 2`）。这正是用户 2026-10-15
#   报的「声明列表**不回来**」在**过程态 / 差分 / 边界**三族上的形态。
#   ⚠ 只换这两个文件 = `638fb911..HEAD` 里**只有 `8452b5b2` 动过它们**（实测
#   `git log --oneline 638fb911..HEAD -- <那两个文件>` 只列出一条）⇒ 换版是
#   **可归属**的、且**逐字节**等于修复前那一版 ✓。
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
#   scripts/third-layer-bite.sh e2e             # 正跑（期望 exit 0）
#   scripts/third-layer-bite.sh e2e-bite        # 咬住证明（期望 exit 1）
#   scripts/third-layer-bite.sh bridge-bite     # 桥的咬住证明（期望 exit 1）
#   scripts/third-layer-bite.sh families        # 三族正跑（期望 exit 0）
#   scripts/third-layer-bite.sh families-bite   # 三族的咬住证明（期望 exit 1）
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

# **三族（过程态 / 差分 / 边界）**的坏版本 rev —— `8452b5b2` 的**父提交**
# （`SUB-parse-error-kills-whole-file` 修复之前的那一版）。换的是两个文件：
# `parser.rs`（参数表跨行吃名字）与 `project/graph.rs`（闭包把入口的全部命令丢掉）。
FAMILIES_BAD_REV="${SOKO_THIRD_LAYER_FAMILIES_BAD_REV:-638fb911}"
PARSER="crates/front/src/parser.rs"
GRAPH="crates/front/src/project/graph.rs"

# **横向**（`AGENTS.md` 硬规则 0(b)）那条接缝的坏版本 —— `def` 卡片第二行 `:= <值>`：
# 坏输入 = **`1ae29932^`**（T-A2/A3 **R-1** 修复之前）的 `crates/lsp/src/query_map.rs`
# 在 `decl_info` 里的状态：**没有** `value` / `value_runs` 那两行 ⇒ LSP 不转发
# ⇒ Infoview 的 `def` 卡片第二行**永远不出现**（T-D52 的原始形态）。
# ⚠ **为什么不整文件换版**：`1ae29932..HEAD` 之间那个文件还改了 49 行（`goal_runs`、
#   G-53 的 run 位置、C3 的 `messages`、F6 的 `Type`）⇒ 整文件换版**编译不过**
#   （实测 `error[E0425]: cannot find function state_messages in module query_map`）
#   ⇒ 那样换出来的红**不可归属** ✗。⇒ 走**手术版**：只摘掉 `1ae29932` **自己加的行**
#   （逐字来自那一笔的 diff），并逐条核"当前文件里确实有"（空转闸）+ 核
#   "`1ae29932^` 里确实没有"（历史事实）✓。
DEF_VALUE_FIX_REV="${SOKO_THIRD_LAYER_DEF_VALUE_FIX_REV:-1ae29932}"
QUERY_MAP="crates/lsp/src/query_map.rs"

# 三族的 mocha grep（三个新用例的标题前缀；`第三层：` 后面跟族名 ✓）。
FAMILIES_GREP="${SOKO_THIRD_LAYER_FAMILIES_GREP:-第三层：(过程态|差分|边界)}"
# `def-value-bite` 只跑**过程态**那一条（`def` 后续命令只长在那一族里 ⇒ 快且可归属 ✓）。
PROCESS_STATE_GREP="${SOKO_THIRD_LAYER_PS_GREP:-第三层：过程态}"

# ── 还原闸（**只有** bite 模式会动工作区）────────────────────────────────────
# ⚠ **多文件**（`families-bite` 要同时换 `parser.rs` + `graph.rs`）：三条平行数组
# 一一对应 —— 一次换版是**全换或全还原**，不许只还原一半 ✗。
SAVED_PATHS=()
SAVED_TMPS=()
SAVED_SHAS=()
RESTORED=0
restore() {
  [ "$RESTORED" = 1 ] && return 0
  [ "${#SAVED_PATHS[@]}" = 0 ] && return 0
  RESTORED=1
  local i path tmp want now rc=0
  for i in "${!SAVED_PATHS[@]}"; do
    path="${SAVED_PATHS[$i]}"
    tmp="${SAVED_TMPS[$i]}"
    want="${SAVED_SHAS[$i]}"
    cat "$tmp" > "$path" || rc=1
    now="$(shasum -a 256 "$path" | cut -d' ' -f1)"
    if [ "$now" != "$want" ]; then
      echo "error: 还原后 sha256 不一致（${path}）：$want → $now" >&2
      rc=1
    else
      echo "+ 还原 ✓ $path sha256=${now:0:16}"
    fi
  done
  return $rc
}
trap 'restore' EXIT

# 把一个文件换成**已存在的坏版本**（`git show <rev>:<path>`），并记下当前内容以便还原。
# 可调用多次（每次追加一项）；`restore` 一次把所有换过的都还原 ✓。
swap_in_bad_version() {
  local rev="$1" path="$2"
  if ! git cat-file -e "$rev:$path" 2>/dev/null; then
    echo "error: 坏版本的 rev 取不到：$rev:$path ⇒ **不假装咬过** ✗（rev 漂了就把 SOKO_THIRD_LAYER_BAD_REV 指到修复前那一版）" >&2
    return 3
  fi
  local tmp sha bad_sha
  tmp="$(mktemp)"
  cat "$path" > "$tmp" || return 3
  sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  git show "$rev:$path" > "$path" || return 3
  # **2026-10-20（S1 之后）**：`CompileOptions` 多了 `dialect` 字段 ⇒ 旧版本的**裸字面量**
  # 编不过（E0063）。咬住证明要的是**旧语义**、不是"旧源码原样编译" ⇒ 这里只做一次
  # **机械补位**（缺的字段给 `..Default::default()`，值取默认 = 与它当年拿到的等价），
  # 与 bug 相关的那段逻辑一个字不动 ✓。⚠ 不许用它"顺手修"旧版本里的别的毛病 ✗。
  perl -0pi -e 's/CompileOptions \{ prelude: ([^,}]+?) \}/CompileOptions { prelude: $1, ..Default::default() }/g' "$path" || return 3
  # **2026-10-24**：`meta_parse.rs`（S3 的 `?mN` 内部通道）引用 `crate::parser::util`，
  # 而旧版 `parser.rs` 没声明这个模块 ⇒ 换版后**编不过**（E0433/E0603）⇒ 证明空转。
  # 只补一条声明（`util.rs` 用工作区那份），与 by 块参数表的 bug 一个字不沾 ✓。
  if [ "$path" = "$PARSER" ] && ! grep -qx 'pub(crate) mod util;' "$path"; then
    printf '\n#[allow(dead_code)]\npub(crate) mod util;\n' >> "$path" || return 3
  fi
  bad_sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  # ⚠ **空转闸**（2026-10-16 实测踩到的假绿）：坏版本若与工作区版本**逐字节相同**，
  # 这次"换版"什么都没换 ⇒ 证明必然 rc=0 ⇒ **判红**（不是"通过" ✗）。
  # 第一次就是拿 `HEAD` 当坏版本（提交之后 HEAD **就是**修好的那一版）⇒ 两条 bite
  # 都 rc=0，被 `check-guard-bite.py --all` 当场抓住 ✓。
  if [ "$bad_sha" = "$sha" ]; then
    echo "error: 「坏版本」与工作区版本**逐字节相同**（${rev} 已经是修好的那一版）⇒ 这条证明会**空转** ✗" >&2
    echo "       把 SOKO_THIRD_LAYER_BAD_REV 指到**修复之前**的那一版（默认 630c0869 = 本轮基线的 main）" >&2
    rm -f "$tmp"
    return 3
  fi
  SAVED_PATHS+=("$path")
  SAVED_TMPS+=("$tmp")
  SAVED_SHAS+=("$sha")
  echo "+ 换成坏版本 ${path}（${rev}）sha256=${bad_sha:0:16}"
  echo "  当前（工作区）版本 sha256=${sha:0:16}（跑完还原）"
}

# **R-1 / T-D52 的最小重现**（`def-value-bite` 用）：把 `decl_info` 里那一行
# `value_runs` 转发置空 ⇒ 回到 **`1ae29932^`（T-A2/A3 R-1 修复之前）的 LSP 状态**。
#
# ## 为什么不是逐字节换版（**实测**，如实标注 ✗）
#
# `1ae29932^` 的 `GoalDeclInfo` **根本没有** `value`/`value_runs` 这两个字段，
# 而 `1ae29932..HEAD` 之间那个文件还改了 49 行（`goal_runs`、G-53 的 run 位置、
# C3 的 `messages`、F6 的 `Type`）⇒ **整文件换版编译不过**（实测：
# `error[E0425]: cannot find function state_messages in module query_map`）；
# 只摘掉那两行**也**编译不过（实测：`error[E0063]: missing fields value and
# value_runs in initializer of GoalDeclInfo`——它们是结构体的必填字段 ✓）。
#
# ⇒ 用**最小重现**：只把 `value_runs` 置空（`value` **留着**）——
#   这正是 **T-D52 的机制本身**：扩展是"**有就渲染**"的宽容实现，它读的是
#   `decl.value_runs` ⇒ 数组空 ⇒ **那一行静默消失**（`value` 在不在都一样 ✗）。
#   ⇒ 这条 bite 打在**渲染那一跳**上（"数据对了 ≠ 用户看见了"）✓。
#   ⚠ **如实标注差异**：真实那一版**连 `value` 也没有**；"字段不上 wire"那一半
#     由**已登记**的 `scripts/audit-wire-fields.py --selftest` 咬 ✓
#     （两条合起来才是 R-1 的全形）。
#
# 三条闸门都在**写任何东西之前**判 ⇒ 被拒时零写入 ✓：
#   ① `rev:path` 取得到；② **历史事实**：`rev^` 的 `path` 里**确实没有**那一行
#   （否则我不是在重现它 ✗）；③ **空转闸**：那一行在当前文件里逐字找得到**且唯一**、
#   替换后**真的变了**（逐字节相同 ⇒ exit 3 ✗）。
inject_r1_state() {
  local rev="$1" path="$2"
  if ! git cat-file -e "$rev:$path" 2>/dev/null; then
    echo "error: 坏版本的 rev 取不到：$rev:$path ⇒ **不假装咬过** ✗" >&2
    return 3
  fi
  # ⚠ **逐字**（含 8 空格缩进）—— `grep -x` 是**整行**匹配，少了缩进就恒为 0
  # ⇒ 空转闸会**假红**（实测踩过：第一次写漏了缩进 ⇒ `value=0 runs=0` ✗）。
  local line_runs='        value_runs: decl.value_runs.into_iter().map(run_info).collect(),'
  if git show "$rev^:$path" 2>/dev/null | grep -qF "$line_runs"; then
    echo "error: ${rev}^:${path} 里**已经有** \`${line_runs}\` ⇒ 它不是我声称的那个坏版本 ✗" >&2
    return 3
  fi
  local tmp sha n_runs
  tmp="$(mktemp)"
  cat "$path" > "$tmp" || return 3
  sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  n_runs="$(grep -cxF "$line_runs" "$tmp")"
  if [ "$n_runs" != 1 ]; then
    echo "error: 空转闸 ✗ —— 那一行在当前文件里的条数是 ${n_runs}（要求恰好 1）" >&2
    echo "       要么 R-1 已经被别的写法修掉/改写了 ⇒ 把 SOKO_THIRD_LAYER_DEF_VALUE_FIX_REV 重新钉一版" >&2
    rm -f "$tmp"
    return 3
  fi
  awk -v b="$line_runs" '
    $0 == b { print "        value_runs: Vec::new(), // R-1/T-D52 重现：那一版**不转发** value_runs（bite 用）"; next }
    { print }
  ' "$tmp" > "$path" || return 3
  local bad_sha
  bad_sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  if [ "$bad_sha" = "$sha" ]; then
    echo "error: 替换之后**逐字节没变** ⇒ 这条证明会**空转** ✗" >&2
    rm -f "$tmp"
    return 3
  fi
  SAVED_PATHS+=("$path")
  SAVED_TMPS+=("$tmp")
  SAVED_SHAS+=("$sha")
  echo "+ 重现 R-1/T-D52（${rev}^ 的 LSP 状态）：${path} 的 \`value_runs\` 转发 ⇒ \`Vec::new()\`"
  echo "  坏版本 sha256=${bad_sha:0:16}（工作区 sha256=${sha:0:16}，跑完还原）"
}

# **`usable()` 的最小重现**（`e2e-bite` 用，2026-10-28 起）：只把当前 `usable()` 的
# 主体换回 `630c0869` 那一版，**不**整文件换版。
#
# ## 为什么不再整文件换版（**第三次**被编译打断，如实标注 ✗）
# `swap_in_bad_version` 是 `git show <rev>:<path>` 整文件覆盖；之后每**新增一个 API**
# 就断一次，而且**断的是编译**（rc=3 = 环境错）⇒ 被 `check-guard-bite.py` 判 C2 假红：
#   * 2026-10-20：`QueryDoc` 多 `dialect` ⇒ E0609 ×5（当时加了 perl 机械补位）；
#   * 2026-10-24：`meta_parse.rs` 引用 `parser::util`（families 那一路，另加了补声明）；
#   * **2026-10-28（这次）**：`crates/lsp/src/lib.rs:1454` 调
#     `sokonanoda_front::query::select_state_at_in_text` ⇒ 旧文件里没有它
#     ⇒ `error[E0425]` ⇒ `e2e-bite` rc=3（CI 40506818671 的 `editor` job 实测；
#     本地同样复现：`/tmp/e2e-bite.log`）。
# 咬住证明要的是**旧语义**，不是"旧源码原样编译" ⇒ 只换 `usable()` 的主体：
#   `self.report.is_some() && (self.parse_error.is_none() || self.project_entry_compiled())`
# 其余一字不动 ✓（旧主体**逐字**取自 `630c0869`）。
#
# 三条闸门都在**写任何东西之前**判 ⇒ 被拒时零写入 ✓：
#   ① `rev:path` 取得到；② **历史事实**：`rev` 的那一版里**确实有**旧主体（否则它不是
#   我声称的那个坏版本 ✗）；③ **空转闸**：当前文件里新主体**恰好 1 处**、替换后真的变了。
inject_old_usable() {
  local rev="$1" path="$2"
  if ! git cat-file -e "$rev:$path" 2>/dev/null; then
    echo "error: 坏版本的 rev 取不到：$rev:$path ⇒ **不假装咬过** ✗（rev 漂了就把 SOKO_THIRD_LAYER_BAD_REV 指到修复前那一版）" >&2
    return 3
  fi
  local old_body='        self.report.is_some() && (self.parse_error.is_none() || self.project_entry_compiled())'
  if ! git show "$rev:$path" 2>/dev/null | grep -qxF "$old_body"; then
    echo "error: ${rev}:${path} 里**没有**我声称的旧 \`usable()\` 主体 ⇒ 它不是我声称的那个坏版本 ✗" >&2
    return 3
  fi
  local new_snip tmp sha rc=0
  new_snip='        self.report.as_ref().is_some_and(|report| {
            report.has_content() || self.parse_error.is_none() || self.project_entry_compiled()
        })'
  tmp="$(mktemp)"
  cat "$path" > "$tmp" || return 3
  sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  python3 -c 'import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
t = open(p, encoding="utf-8").read()
n = t.count(new)
if n != 1:
    print(f"error: 空转闸 ✗ —— 当前 `usable()` 主体条数 = {n}（要求恰好 1）；它被改写过就把本函数的 new_snip 重新对齐", file=sys.stderr)
    sys.exit(3)
open(p, "w", encoding="utf-8").write(t.replace(new, old, 1))' "$path" "$old_body" "$new_snip" || rc=$?
  if [ "$rc" != 0 ]; then
    rm -f "$tmp"
    return 3
  fi
  local bad_sha
  bad_sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
  if [ "$bad_sha" = "$sha" ]; then
    echo "error: 替换之后**逐字节没变** ⇒ 这条证明会**空转** ✗" >&2
    rm -f "$tmp"
    return 3
  fi
  SAVED_PATHS+=("$path")
  SAVED_TMPS+=("$tmp")
  SAVED_SHAS+=("$sha")
  echo "+ 重现 ${rev} 的 \`usable()\`（最小换体，不整文件换版）：${path}"
  echo "  坏版本 sha256=${bad_sha:0:16}（工作区 sha256=${sha:0:16}，跑完还原）"
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
  # `$1`（可选）= 这一次跑的 mocha grep（默认 `$GREP_NAME`）——`families` 两模式
  # 只跑三个新用例（33 份读数），这样 bite 的**判红点可归属**、且跑得动 ✓。
  local grep_name="${1:-$GREP_NAME}"
  need_node_modules || return 3
  build_and_stage || return 3
  echo "+ e2e：SOKO_E2E_GREP=$grep_name · VS Code ${VSCODE_VERSION}（日志 ${LOG}）"
  (cd editor/vscode && SOKO_E2E_GREP="$grep_name" SOKO_VSCODE_TEST_VERSION="$VSCODE_VERSION" \
    npx vscode-test) > "$LOG" 2>&1
  local rc=$?
  echo "--- 读数（${LOG}）---"
  # ⚠ 只回读**计数行 + 判红点**（33 份读数全灌进日志会淹掉失败原因 ✗）。
  echo "第三层读数条数：$(grep -cE "第三层读数" "$LOG")"
  grep -E "passing|failing" "$LOG" | head -5
  grep -E "AssertionError" "$LOG" | head -4
  grep -E "第三层读数" "$LOG" | head -4
  echo "--- e2e rc=$rc ---"
  return $rc
}

case "$MODE" in
  e2e)
    run_e2e
    exit $?
    ;;
  e2e-bite)
    # 坏输入 = **`630c0869`（= 本轮基线 main）的 `Query::usable()`**：一处语法错
    # 让 `goals`/`holes` 拒答 ⇒ LSP `soko/goals` 答空 ⇒ **Infoview 的声明列表
    # 不回来** ✗（用户 2026-10-15 亲报的那一格）。
    # ⚠ 2026-10-28 起**只换 `usable()` 的主体**（不再整文件换版：那已经三次因
    # 「新 API 不在旧文件里」而编译失败 ⇒ rc=3 假红）——原因与三道闸门见
    # `inject_old_usable()` 的注释 ✓。
    inject_old_usable "$BAD_REV" "$USABLE" || exit 3
    run_e2e
    rc=$?
    restore || exit 3
    exit $rc
    ;;
  families)
    run_e2e "$FAMILIES_GREP"
    exit $?
    ;;
  families-bite)
    # 坏输入 = **`8452b5b2` 的父提交 `638fb911`** 的 `parser.rs` + `graph.rs`
    #（`SUB-parse-error-kills-whole-file` 修复之前那一版）⇒ 未关块**后面**那条
    # 命令整行不见 ✗（用户 2026-10-15 报的「声明列表不回来」在另三族上的形态）。
    swap_in_bad_version "$FAMILIES_BAD_REV" "$PARSER" || exit 3
    swap_in_bad_version "$FAMILIES_BAD_REV" "$GRAPH" || exit 3
    run_e2e "$FAMILIES_GREP"
    rc=$?
    restore || exit 3
    exit $rc
    ;;
  def-value-bite)
    # 坏输入 = **`1ae29932^`（R-1 修复之前）**的 `query_map.rs::decl_info` ——
    # LSP **不转发** `value`/`value_runs` ⇒ `def` 卡片的第二行 `:= <值>`
    # **永远不出现**（T-D52 的原始形态）。横向那条判据必须**咬得住它** ✓。
    inject_r1_state "$DEF_VALUE_FIX_REV" "$QUERY_MAP" || exit 3
    run_e2e "$PROCESS_STATE_GREP"
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
    echo "usage: scripts/third-layer-bite.sh {e2e|e2e-bite|families|families-bite|def-value-bite|bridge-bite}" >&2
    exit 2
    ;;
esac
