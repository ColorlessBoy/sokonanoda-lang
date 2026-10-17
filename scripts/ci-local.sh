#!/usr/bin/env bash
# ci-local.sh —— **push 之前把 CI 的活在本机全跑一遍**（2026-09-25 用户要求 ✓）
#
# 缘起（用户原话 ✓）："本地没办法提前测试吗？github action 的机器性能不太行" ✓ ——
# 实测：2026-09-24T16:42Z 之后连续 14 轮 ci 全红 ✗，而两个根因（26 步巨无霸 job 卡住、
# e2e 判据数事件 + 空转 waitFor）**本地都能测出来** ✓（详见 docs/CI-FAILURES.md）。
# 结论 ✓：**CI 只当"平台矩阵确认"**（macos/ubuntu × VS Code 版本 ✓），
# **不当第一次测试** ✗；第一次测试永远是这条脚本 ✓。
#
# 用法：
#   scripts/ci-local.sh            # 全部（除 e2e，约 3–5 分钟 ✓）
#   scripts/ci-local.sh --fast     # 跳过 workspace 全套单测（约 1 分钟 ✓）
#   scripts/ci-local.sh --e2e      # 追加真宿主 e2e（约 8–10 分钟 ✓）
#   SOKO_VSCODE_TEST_VERSION=1.106.0 scripts/ci-local.sh --e2e   # 指定 VS Code 版本 ✓
#
# 退出码：0=全绿 ✓ / 1=有阶段红 ✗（打印是哪一条 ✓）
set -uo pipefail
cd "$(dirname "$0")/.."

# ⚠ **摘掉 git 自己的 `GIT_*`**（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`，2026-10-16）：
#   `GIT_DIR` **优先于 cwd** 决定仓库发现 ⇒ 从**链接工作树**的 `pre-push` 钩子里跑时
#   （实测：`git push` 在链接工作树里会把 `GIT_DIR=<主>/.git/worktrees/<wt>` 交给钩子 ✓），
#   下面每一句 `python3 scripts/…` 的**夹具**都会把 `git init`/`commit`/`config` 打到
#   **调用者的仓**上 ✗（实测后果：`core.bare=true` + 夹具提交 + 238 行 dirty ✗）。
#   ⚠ 这一行是**兜底**（口径 = `scripts/git_env.py`，所有 python 侧调用点**各自**已清 ✓）：
#   它同时覆盖 `scripts/ux-queue.py` 里那些**没清**的夹具（那个文件不在本轮的改动范围 ✓）。
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY \
      GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_PREFIX

FAST=0; WANT_E2E=0
for a in "$@"; do
  case "$a" in
    --fast) FAST=1 ;;
    --e2e)  WANT_E2E=1 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "未知参数：$a ✗（支持 --fast / --e2e）" >&2; exit 2 ;;
  esac
done

: "${DEVELOPER_DIR:=/Library/Developer/CommandLineTools}"
export DEVELOPER_DIR
: "${CARGO_TARGET_DIR:=/tmp/soko-target}"
export CARGO_TARGET_DIR
export NODE_NO_WARNINGS=1

PASS=(); FAIL=()
run() { # run <阶段名> <CI 里对应的 job> <命令...>
  local name="$1" job="$2"; shift 2
  printf '\n\033[1m▶ %s\033[0m  （CI job: %s）\n  $ %s\n' "$name" "$job" "$*"
  local t0=$SECONDS
  if "$@"; then
    printf '  ✅ %s 通过（%ss）\n' "$name" "$((SECONDS-t0))"; PASS+=("$name")
  else
    local rc=$?
    printf '  ❌ %s **失败**（exit=%s，%ss）—— 就是这一条 ✗（CI 会红在这里）\n' "$name" "$rc" "$((SECONDS-t0))"
    FAIL+=("$name (exit=$rc)")
    summary; exit 1
  fi
}
summary() {
  printf '\n\033[1m=== 汇总 ===\033[0m\n'
  local p; for p in "${PASS[@]:-}"; do [ -n "$p" ] && printf '  ✅ %s\n' "$p"; done
  local s; for s in "${SOFT[@]:-}"; do [ -n "$s" ] && printf '  ⚠️  %s（**读数** · 只出声不拦 ✓）\n' "$s"; done
  local f; for f in "${FAIL[@]:-}"; do [ -n "$f" ] && printf '  ❌ %s\n' "$f"; done
  [ ${#FAIL[@]} -eq 0 ] && printf '  ⇒ **本地全绿 ✓**：可以 push 了 ✓（CI 只做平台矩阵确认 ✓）\n' \
                        || printf '  ⇒ **别 push** ✗：先修上面那条 ✓\n'
}
SOFT=()
soft() { # soft <读数名> <命令...> —— **只出声不判红** ✗（读数 ≠ 闸；理由见 `scripts/release-metric.py` 头部）
  local name="$1"; shift
  printf '\n\033[1m▶ %s\033[0m  （**读数** · 只出声不判红 ✗）\n  $ %s\n' "$name" "$*"
  local out rc
  out=$("$@" 2>&1); rc=$?
  printf '%s\n' "$out"
  if [ "$rc" -eq 0 ]; then
    printf '  ✅ %s：绿 ✓\n' "$name"
  else
    printf '  ⚠️  %s：**红**（rc=%s）—— 出路 = 走一次发布闭环（bump 两处版本 → push main → auto-tag）✗ **不拦本次推送** ✓\n' "$name" "$rc"
  fi
  SOFT+=("$name (rc=$rc)")
}

# ⓪ **workflow 文件本身的静态校验** ✓（2026-09-26 事故 ✓）——
# 我把新 `run:` 写进了上一个 step 里 ✗ ⇒ 同一 step 两个 `run:` 键 ✗ ⇒
# `yaml.safe_load` 静默取最后一个（所以我"验过了"是假的 ✗），而 **GitHub 拒绝整个
# workflow** ✗ ⇒ 整轮 0 job、0 秒失败 ✗。**这条脚本是唯一能在 push 前拦住它的地方** ✓。
run "workflow：YAML 严格校验（禁重复键）" "gates" python3 scripts/ci-yml-lint.py
# ⭐ **推送范围基点**（2026-10-15，洞 `PUSH-RANGE-HIDES-RELEASE` ✓）—— 与 CI 的 `wiring-lint`
# 里那两步**逐字同源** ✓：`--check-wiring` 咬 `ci.yml` 的接线（`changes` 真的把算出来的 `base`
# 传给 paths-filter ✗✓、标记的 save/restore 前缀一致 ✗✓、写标记的 job 的 `if` 带状态函数 ✓），
# `--selftest` 在**合成夹具仓库**上跑范围语义（含**正对照**：连续两次纯 docs ⇒ 仍不跑重活 ✓；
# 含**负对照**：旧口径下同一推送 rust=false ⇒ 洞复现 ✓）。亚秒级 ⇒ 进 ⓪ ✓（**改了判据必须有人验** ✓）。
run "workflow：推送范围基点（(A) 的范围语义 + 接线）" "wiring-lint" bash -c '
set -u
python3 scripts/ci-push-range.py --check-wiring || exit 1
python3 scripts/ci-push-range.py --selftest || exit 1'
# ⭐ **「按改动范围决定跑不跑」的类守卫**（2026-10-15，洞 `PATHS-FILTER-CLASS` ✓）—— 与 CI 的
# `wiring-lint` 里那两步**逐字同源** ✓：`--check` 咬"job 消费的每条仓库路径都必须被门控它的
# 过滤器命中"（R1 执行路径 / R2 工作流自身 / R3 数据依赖），`--selftest` 五个对照
# （正对照：现役 `ci.yml` 必须绿 ✓；四个负对照：撤掉历史三次的修法 + 新加一个越界 job ⇒ 必须红 ✓）。
# 亚秒级 ⇒ 进 ⓪ ✓（**改了判据必须有人验** ✓）。
run "workflow：改动范围门控覆盖（R1/R2/R3）" "wiring-lint" bash -c '
set -u
python3 scripts/check-ci-gate-coverage.py --check || exit 1
python3 scripts/check-ci-gate-coverage.py --selftest || exit 1'

# ⭐ **`wiring-lint` 的另外三对判据**（2026-10-17 · 洞 `CI-ONLY-GUARDS-WIRING-LINT-TRIO` ✓）——
# 与 `ci.yml` 的 `wiring-lint` job 里那**三步**逐字同源 ✓（`check-doc-drift` ·
# `check-launcher-extension-link` · `check-lsp-marker-seam`，各跑「判据 + `--selftest`」）。
# ⚠ **为什么必须补**（**实测读数** ✓）：这三条**只**长在 `ci.yml` 的 `wiring-lint` 里，
# 本脚本里 `grep -c` = **0 / 0 / 0** ✗ ⇒ 又一处「**本地判据集 ⊂ CI 判据集**」——
# 与 `CI-LOCAL-NO-QUEUE-CHECK`（`ux-queue.py`）· `GUARD-BITE-NOT-IN-LOCAL-GATE`
# （`check-guard-bite.py`）· `CI-GREEN-SELFTEST-NOT-WIRED`（`ci-green.py --selftest`）·
# `CHECK-JSON-IDENTITY-NOT-WIRED-AT-ALL`（`check-json-identity.py`）**逐字同族**：
# 判据的红**只能走到 CI 才被发现** ✗（实测一轮 CI = **33–35 分钟** ⇒ 本地白等一轮 ✗）。
# ⚠ **这不是"新口径"，是把同一条 job 补齐** ✓：同一个 `wiring-lint` 的**另外两对**
# （`ci-push-range.py` · `check-ci-gate-coverage.py`）本脚本**早就**有 ✓ ⇒ 三缺三
# 正是「**接了一半**」的形状 ✗（洞名里的 `TRIO` 就是这三个）。
# ⚠ **零前提** ✓（`wiring-lint` job 只有 `checkout`、**没有**构建步 ⇒ 这三条不碰
# `target/`、不碰真缓存/真 HOME、不需要 Rust ✓）；**耗时实测**（本机 · 六条命令合计
# **8.24s** ✓）：`check-doc-drift` 0.09+0.07 · `check-launcher-extension-link` 2.12+0.66 ·
# `check-lsp-marker-seam` 1.90+3.40 ⇒ 与 `docs-lint` 同档，**`--fast` 也跑** ✓。
# ⚠ **`node` 不是新增前提** ✗：本脚本 :356 的 `editor：stub 宿主`（**非** FAST 门控）
# 早就硬依赖 `node` ✓ ⇒ 后两条带 node 的守卫没有引入新的前提类 ✓；探不到 node
# ⇒ 它们 **exit 3**（"判不了 ≠ 绿"）⇒ **拦**，正是要的行为 ✓。
run "workflow：harness 接线的文档漂移（三处 README ↔ 实现）" "wiring-lint" bash -c '
set -u
python3 scripts/check-doc-drift.py || exit 1
python3 scripts/check-doc-drift.py --selftest || exit 1'
run "workflow：启动器「VS Code 扩展自带」链接（C1–C4，含 canExec 不许松）" "wiring-lint" bash -c '
set -u
python3 scripts/check-launcher-extension-link.py || exit 1
python3 scripts/check-launcher-extension-link.py --selftest || exit 1'
run "workflow：LSP/CLI 槽位 marker 跨接缝（C1–C7，含两条负对照不许松）" "wiring-lint" bash -c '
set -u
python3 scripts/check-lsp-marker-seam.py || exit 1
python3 scripts/check-lsp-marker-seam.py --selftest || exit 1'

# ⭐ **四条 CI-only 判据接进本地自动面**（2026-10-17 · 洞 `CI-ONLY-GUARDS-HISTORY-INFO-TACTIC-ENVISOL` ✓）——
# 与 `ci.yml` 的 `run:` 正文**逐字同源** ✓（各跑「判据 + `--selftest`」两行；`check-infoview-hierarchy.py`
# 那一步在 CI 里还有第三行 `audit-infoview-css.py`，本脚本 **:241 早就有** ✓ ⇒ 那一步在本地**整步齐** ✓）。
# ⚠ **为什么必须补**（**修前实测读数** ✓，逐条可复算）：这四条**只**长在 `ci.yml` 里 ——
#   `grep -c <闸> scripts/ci-local.sh` = **0 / 0 / 0 / 0** ✗ · `grep -rc <闸> scripts/githooks/*` = **0** ✗
# ⇒ 又一处「**本地判据集 ⊂ CI 判据集**」（同族已有五处 ✓：`CI-LOCAL-NO-QUEUE-CHECK` ·
# `GUARD-BITE-NOT-IN-LOCAL-GATE` · `CI-GREEN-SELFTEST-NOT-WIRED` · `CHECK-JSON-IDENTITY-NOT-WIRED-AT-ALL` ·
# `CI-ONLY-GUARDS-WIRING-LINT-TRIO`）⇒ 判据的红**只能走到 CI 才被发现** ✗（一轮 CI = **33–35 分钟** ✗）。
# ⚠ **这四条正是 `ng1` 按行为普查（41 道闸 · `.cache/ux/fix-ng1/VERDICT.md`）里 (d) 缺本地那一半的** ✓：
# 修前逐条 `a=True b=True c1=True d=False`（ci=1 / local=0 ✓；复算 = 本目录 `.cache/ux/fix-ng2/census.py`）。
# ⚠ **后果**（**抄 `ng1` §3.C 的表** ✓ —— 本轮**不重做普查** ✗）：
#   · `check-history-depth-wiring.py`：「跑历史判据的 job 必须有全历史」这条**类不变量**被改坏
#     ⇒ 本地看不见 ⇒ 症状是 CI 里 `git` **rc=128** ⇒ 判据 **exit 3**（红的是**前提**、不是"咬不住" ✗）。
#   · `check-infoview-hierarchy.py`：Infoview 层级（标题字号 ≥ 正文 / 无双压暗 / 标题不用 opacity）回归
#     ⇒ 本地看不见 —— 而这是**同类反馈复发两次**后落的守卫（`.decl-ty` → `.section-title` ✗）。
#   · `tactic-docs-lint.py`：tactic 表 / parser 臂 / 文档**三处一致**被改坏 ⇒ 本地看不见。
#   · `check-test-env-isolation.py`：配 `env::set_var` 的集成测试文件多了一个 `#[test]`
#     ⇒ 两条判据抢同一档位 ⇒ **间歇判红**（症状是"单独跑绿、gate 里红"）⇒ 本地看不见。
# ⚠ **零前提** ✓（四条只读仓库文本：`ci.yml` / `media/*.css` / `tactics.rs`+`parser.rs`+`reference/` /
# `crates/*/tests/*.rs` ⇒ 不碰 `target/`、不碰真缓存/真 HOME、不需要 Rust/node/gh ✓）；
# **耗时实测**（本机 · 八条命令逐条 **0.03–0.08s** ✓）：`check-history-depth-wiring` 0.06+0.08 ·
# `check-infoview-hierarchy` 0.03+0.04 · `tactic-docs-lint` 0.05+0.05 · `check-test-env-isolation` 0.06+0.03
# ⇒ 八条**一次跑完**（= 本步新增的墙钟）三轮 **0.37 / 0.38 / 0.39s**（中位 **0.38s** ✓）
# ⇒ 亚秒级，**`--fast` 也跑** ✓。
# ⚠ **退出码口径**（照抄既有写法 ✓ · **不掩膜** ✗）：四条一律 `|| exit 1` 传播 —— `0`=绿 · `1`=判红 ·
# **`2`/`3`=判不了**（`check-infoview-hierarchy.py` 的 `2` = **CSS 找不到的环境错** · `tactic-docs-lint.py`
# 的 `2` = 用法/IO 错 · `check-history-depth-wiring.py` 的 `3` = 前提不在 —— **三条都必须拦** ✓）。
# ⚠ **别把它们当成 `target-hygiene.py` 的那个两档告警** ✗：那个档是"只报不拦"的**读数**（非 0 里的一个），
# 这四个**没有**那一档 ⇒ **不需要**显式放行 ✓（`run()` 把任何非 0 当红，正是这里要的行为 ✓）。
# ⚠ **如实划的边界** ✗：`check-history-depth-wiring.py --selftest` 的咬住证明打在**钉死的旧 rev**
# （`1cd4aba0`）⇒ **它自己也要全历史**（浅克隆里 `git show` 拿不到 ⇒ **exit 3 + 人话**，绝不静默判过 ✓）；
# 本仓/本机是**全克隆** ✓，且同族先例 `check-attention-budget.py`（下面 :347 那一步）**早就**是这个形状
# （它的注释自己写着「要全历史（浅克隆 ⇒ rc=3 + 人话）」✓）⇒ 口径一致，不是本轮新增的前提 ✗。
run "workflow：历史深度接线（跑历史判据的 job 必须有全历史）" "queue-lint" bash -c '
set -u
python3 scripts/check-history-depth-wiring.py || exit 1
python3 scripts/check-history-depth-wiring.py --selftest || exit 1'
run "workflow：Infoview 视觉层级（标题字号 ≥ 正文 / 无双压暗 / 标题不用 opacity）" "editor" bash -c '
set -u
python3 scripts/check-infoview-hierarchy.py || exit 1
python3 scripts/check-infoview-hierarchy.py --selftest || exit 1'
run "gates：tactic 清单三处一致（表 ↔ parser 臂 ↔ reference/tactics/*.md）" "tactic-docs-lint" bash -c '
set -u
python3 scripts/tactic-docs-lint.py || exit 1
python3 scripts/tactic-docs-lint.py --selftest || exit 1'
run "gates：测试环境档位隔离（配 env::set_var 的文件必须恰好一个 #[test]）" "gates-fast" bash -c '
set -u
python3 scripts/check-test-env-isolation.py || exit 1
python3 scripts/check-test-env-isolation.py --selftest || exit 1'

# ⭐⭐ **交付维度读数**（2026-10-16 · 用户追问「清账 Agent 不会检查还有多少没发版吧，我看它每次
# 给我发的会话累计」✓ · 队列 `SUB-release-metric-measures-unreleased`）：
# 报告那张「会话累计」表里的「**发版**」那一格原写「`v0.91.0` 已发布并闭环（26 资产）」——
# 那是个**二进制**事实（历史上发过 ⇒ **永远为真**）⇒ 它**永远不会**提醒"该发版了" ✗：
# 实测（用户报数那一刻 `05584b15`）**209 笔未发版 / 22.9 小时**，而那一格仍是 ✓ ✗。
# ⇒ 换成**未发版量**（可数 · 会涨 · 会判红 ✓）= `python3 scripts/release-metric.py`
# （口径 = 那个脚本的 `TIERS`，`--explain` 印出来 ✓；红线 = >50 笔 或 >24h ✓）。
# ⚠ **为什么用 `soft` 而不是 `run`** ✗：发版是**人工 ops 动作**（bump 两处版本 → push main →
# `ci.yml:426` 的 auto-tag），**不是代码能自己解决的事** ⇒ 判红会是一条**没有出路的红**
# （照 `scripts/check-attention-budget.py` 的 `judge_round()` 先例）⇒ `soft()` **故意不传播 rc** ✓；
# 但它**每次 push 都印**（`scripts/githooks/pre-push` 跑本脚本 ✓）—— 这就是"让那一格活着"的机制 ✓。
# 亚秒级（纯 git、不联网）⇒ `--fast` 也跑 ✓。
soft "读数：未发版量（交付维度 · 报告那一格）" python3 scripts/release-metric.py

# ① lint（CI job `lint`）—— 注意：**绝不** `cargo fmt --all`（kernel 的 rustfmt.toml 要 nightly ✓）
run "lint：fmt"  "lint" cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
# ⚠ **与 CI 逐字一致**：CI 是 `cargo clippy --workspace --all-targets`（**不带** `-D warnings` ✓）。
# 我第一版多写了 `-D warnings` ✗ ⇒ 内核既有的 62 条 warning 被当错误（cast_possible_truncation /
# len_without_is_empty … ✓）⇒ 脚本比 CI **更严** ✗ ⇒ 那不是"本地提前测" ✓ 而是自造红 ✗。
# 真正的 clippy **错误**（deny 级 ✓，例如 round 90 那个 let_and_return ✓）本来就会让构建失败 ✓。
run "lint：clippy" "lint" cargo clippy --workspace --all-targets

# ② test（CI job `test`）—— 本地 161 秒 ✓（CI 上却 >20 分钟 ✗，见 CI-FAILURES ✓）
if [ "$FAST" = 0 ]; then
  run "test：workspace 全套" "test" cargo test --workspace --locked --no-fail-fast
else
  printf '\n▶ test：workspace 全套  —— **--fast 跳过** ✓（push 前请别跳 ✗）\n'
fi

# ③ gates（CI job `gates`）
# **按 FAST 分档**（2026-10-03 ✓，值守实测 + 我这轮实测：`:80` 原来**无条件**跑 ⇒ 每次 push 多背
# **959 秒**（16 分钟）✗ —— 而 `scripts/githooks/pre-push:23` 的注释写着「约 1 分钟」✗
# ⇒ 又一个「注释声明 + 没有守卫」✓）。上面那行注释自己写着「③ gates（**CI job `gates`**）」
# ⇒ **远端 CI 有对应 job 兜底** ✓ ⇒ 本地快层跳过它**不会摘掉唯一守卫** ✓
#（区别于「时序证据守卫」那种只在本地有牙的 ✗ —— 那条**不动** ✓）。
if [ "$FAST" = 0 ]; then
  run "gates：课程门禁" "gates" python3 courses/set-theory/tools/check.py
else
  printf '\n▶ gates：课程门禁  —— **--fast 跳过** ✓（**本轮未验证** ✗；远端 CI 的 `gates` job 会跑 ✓）\n'
  printf '   上次读数：250 个目标 —— 2076 checked · 903 open · 0 个被判负（2026-10-03 15:40 ✓）\n'
fi
# **缺口台账带 `--strict`**（2026-09-25 ✓）：CI 上超时**跳过**（慢 runner 是环境事实 ✓），
# 但**快机器上必须跑完** ✓ —— 否则守卫就被"跳过"架空了 ✗（用户："每次都是它出问题，
# 但是从来不改" ✗ ⇒ 这一刀就是"本地兜底判据" ✓）。
# **按 FAST 分两档**（2026-10-03 ✓，值守复核 `01bd7ed1` 时逮到的缝 ✗）：pre-push 走的是
# `scripts/githooks/pre-push:23` = `./scripts/ci-local.sh --fast` ✓ ⇒ 深跑**必须**从 push
# 路径里摘出去 ✗（否则"假红"会换成"推送卡住 ~1.5h" ✗ —— 同一个"声明与实现有缝" ✗）。
# **SOKO_SKIP_GAP=1**（2026-10-06 ✓）：逃生门专用 —— **只**跳过缺口台账这条（本地慢机器
# 长复现件超时 301s 判红 ✗），**其它秒级检查（docs-lint / notation-lint / ci-yml-lint /
# status-lint / plan / bump / audit …）一律照跑** ✓（用户：「现在能在推送前查出来的都可以
# 放到推送前」✓ —— 逃生门不该连秒级检查一起跳 ✗）。
if [ "$FAST" = 0 ]; then
  # **完整跑（深跑路径 ✓）**：连长复现件一起跑 ✓ —— 这是"守卫不失去牙齿"的那条路径 ✓；
  # 超时档同时放宽到 3h ✓（默认 300s 只够常规件 ✓）。
  if [ "${SOKO_SKIP_GAP:-0}" != "1" ]; then
    run "gates：缺口台账（--strict ✓ 含长复现件 ✓）" "ledger" \
      env SOKO_GAP_REPRO_TIMEOUT=10800 python3 scripts/gap.py check --strict --include-slow
  else
    printf '\n▶ gates：缺口台账  —— **SOKO_SKIP_GAP=1 跳过** ✗（逃生门，CI ledger job 兜 ✓）\n'
  fi
else
  # **pre-push（FAST=1 ✓）**：**跳过**长复现件 ✓ —— 但**不摘条目** ✗：照样**计数** +
  # 打印**上次读数** + 标注「**本轮未验证**」✗（深跑：完整跑或定时 workflow ✓）。
  if [ "${SOKO_SKIP_GAP:-0}" != "1" ]; then
    run "gates：缺口台账（--strict ✓ 长复现件跳过 ✓ 本轮未验证 ✗）" "ledger" \
      python3 scripts/gap.py check --strict
  else
    printf '\n▶ gates：缺口台账  —— **SOKO_SKIP_GAP=1 跳过** ✗（逃生门，CI ledger job 兜 ✓）\n'
  fi
fi
# **计划一致性**（2026-09-25 round 148 补 ✓，来自一次真实的漏网 ✗）：
# round 145 我给 `T-U12` 用了 `- [~]`（想表达"进行中" ✓）⇒ `plan.py check` **不认**它 ✗
# ⇒ 报"正文里有环节但清单没有排入" ✓、进度 50 → 49 ✗ —— 而 `plan.py check` **是 gate 与 CI 的一步** ✓
# ⇒ 那会变成一次 CI 红 ✗。是**收尾时的完整性检查**抓到的 ✓，不是这道门 ✗ ⇒ 补进来 ✓。
run "gates：计划一致性" "gates" python3 scripts/plan.py check
# **折叠判据的"反向验证"**（2026-09-25 round 167 补 ✓，落实 §9.1 ✓）：
# §9.1 说"**新判据必须附一条反向验证命令**" ✗ —— 但此前**没有任何门在跑它** ✓
# ⇒ 判据可能悄悄变成"永远绿"的摆设 ✗（本 session 为此撤回过**三次**空转判据 ✓）。
# 这里把那两条**已证明咬得住**的判据在 `SOKO_NO_NOTATION_FOLD=1` 下重跑 ✓：
# 它们**必须判红** ✗ —— 若仍然绿 ⇒ 说明判据咬不住 ⇒ **门自己红** ✓。
run "gates：折叠判据的反向验证（必须判红 ✗）" "gates" bash -c '
set -u
# **用 `expect-red.sh` 而不是手写循环** ✓（round 172 ✓）：手写循环里"看输出"与"看退出码"
# 容易混 ✗ —— 而本 session **三次**就是这么把坏东西推上去的 ✓。
for t in a_kernel_pp_display_surface_must_be_folded hover_text_is_folded_like_the_lsp_does; do
  scripts/expect-red.sh "$t 在关掉折叠时" -- \
    env SOKO_NO_NOTATION_FOLD=1 cargo test -p sokonanoda-front --lib "$t" || exit 1
done'
# **守卫自检也必须按退出码拦** ✓（round 171 ✓：我正是在这里放过了 `FAIL` ✗）
run "gates：版本单一源" "gates" python3 scripts/bump.py --check
run "gates：记法规则"   "gates" python3 scripts/notation-lint.py
run "gates：STATUS 瘦身（用户 2026-09-26 ✓）" "gates" python3 scripts/status-lint.py
run "gates：文档预算（用户 2026-09-26 ✓）" "gates" python3 scripts/docs-lint.py
# **冻结余量**（2026-10-16 · 洞 `CI-FAILURES-FROZEN-BUDGET-BLOCKS-MANDATORY-APPEND` ✓）：
# `AGENTS.md` 要求「每次 CI 红了在 `docs/CI-FAILURES.md` **追加一条**」，而它被冻结在 749 行、
# 实测也 749 行 ⇒ **余量 0** ⇒ 下一次追加**必然**撞判据 ④（只许减不许增 ✗）；抬上限被用户
# 2026-09-26 明令禁止 ✗ ⇒ 出路只有**归档腾行**（判据 ⑥）。本守卫把「**强制追加的文档不许把
# 余量吃到 0**」变成判据（**别的**冻结文档余量 0 只报告、不判红 ⇒ 不造假红 ✗）。
run "gates：冻结余量（强制追加类不许吃到 0）" "gates" python3 scripts/check-frozen-headroom.py --check
run "gates：记法路径守卫" "gates" python3 scripts/audit-notation-paths.py
# **UI 同族守卫**（2026-10-03 ✓，AGENTS.md 验证设计纪律第 0 条 (b) ✓）：同一类 UI 反馈
# 复发过两次（`.decl-ty` → `.section-title` ✗）⇒ 判据落成机器可查 ✓（秒级 ✓）。
run "gates：Infoview 样式同族守卫" "gates" python3 scripts/audit-infoview-css.py
run "gates：wire 字段守卫" "gates" python3 scripts/audit-wire-fields.py
run "gates：两个守卫的自检（显式按退出码 ✓）" "gates" bash -c '
set -u
python3 scripts/audit-notation-paths.py --self-test || exit 1
python3 scripts/audit-wire-fields.py --selftest || exit 1
python3 scripts/docs-lint.py --selftest || exit 1' 

# ⭐ **`target/` 体积守卫接进自动路径**（2026-10-17 · 洞 `TARGET-HYGIENE-MANUAL-ONLY` ✓）——
# `scripts/target-hygiene.py` 此前**只**长在 `scripts/soko gate` 里（**人手敲** ✗）⇒
# 按"**按行为**普查"的口径，它在**自动路径上一处也没有** ✗。读数（逐条可复算 ✓）：
#   `grep -c target-hygiene scripts/ci-local.sh` = **1**，而那一条是**注释**（:301 引用它的先例 ✗）
#   · `grep -rc target-hygiene scripts/githooks/` = **0/0/0/0**
#   · `grep -rc target-hygiene .github/workflows/*.yml` = **0**（四份 workflow 全 0 ✓）
#   ⇒ 唯一的调用点是 `scripts/soko gate`（**人手敲** ⇒ 口径 (b) ✗）。
# ⚠ **后果不是假设**（**守卫自己的 docstring 记着两次实测** ✗）：2026-09-27 `target/` 堆到
# **205 GB**（`target/debug` 191 GB，磁盘只剩 159 GB ⇒ **开 worktree 就爆盘**）、
# 2026-10-11 又到 **142 GiB**（61.7 万个 `.rcgu.o` + `incremental/` 53 GiB）——
# **两次都是"用户问起才发现"** ✗（原话：「浪费我太多 ssd，好麻烦」✓）。根因是
# **增量编译每会话往 `deps/` 吐一批新的 `*.rcgu.o`、cargo 从不回收** ✓
# ⇒ 没有自动面 = 它会**静默**胖回去（"某个东西的前提没被验证，而它的后果落在别处" ✓）。
# ⚠ **为什么不进 CI**（照抄 `scripts/soko` 同一步的**既定理由**，**不改判据** ✗）：CI runner
# 是**一次性**的、`target/` 天生只有一次构建 ⇒ 在那边没有意义 ✓ ⇒ 接进**本地自动路径**
# （`pre-push` 的 `--fast` ✓）才是这条守卫的**正确面** ✓ —— 它管的就是**开发机**的盘 ✓。
# ⚠ **退出码口径逐字照抄 `scripts/soko`**（**只许一致、不许自创** ✗）：`0` 干净 ✓ ·
# **`2` = 告警（40 GB / 25 万个 `.rcgu.o`）只报不拦** ✓ —— 拦在告警线上会让推送在
# **日常规模**就常红 ✗（`run()` 把**任何**非 0 都当红 ⇒ 必须在这里**显式放行 2** ✓；
# 这正是"照抄"而不是"顺手写 `run`"的原因 ✓）· `1`（100 GB / 60 万个 `.rcgu.o` /
# incremental 红线）与 `3`（判不了）**真拦** ✓。
# ⚠ **如实划的边界** ✗：`--root` 默认 = **本检出**的 `target/` ⇒ 在**还没有 target/ 的
# 链接工作树**里它会印「跳过：没有 target/ ⇒ 无需检查」并 rc=0 ✓ —— 那是**设计语义**
# （没有的东西无从回收 ✓），不是空转；主 checkout / 已构建过的树里它**真的量** ✓。
# ⚠ **耗时**（实测本机 · 主 checkout `target/` = 2.9 GB / 561 个 `.rcgu.o`）：**0.09s** +
# `--selftest` **0.09s** ✓；病态规模 16–21s（只在**该知道的时候**才慢 ✓）⇒
# 与 `docs-lint` 同档，**`--fast` 也跑** ✓。
run "gates：target/ 体积守卫（本地专属 · CI runner 是一次性的）" "（无 CI 对应：本地专属）" bash -c '
set -u
python3 scripts/target-hygiene.py; rc=$?
python3 scripts/target-hygiene.py --selftest || exit 1
if [ "$rc" = 2 ]; then
  printf "  ⚠️ 告警档（未超红线）—— **只报不拦** ✓（口径与 scripts/soko 的同一步逐字一致 ✓）\n"
  exit 0
fi
exit "$rc"'

# **队列不变量**（洞 `CI-LOCAL-NO-QUEUE-CHECK`，2026-10-15 ✓）—— 与 CI 的 `queue-lint` job
# **逐字同源** ✓：`ux-queue.py --check` = 覆盖自检
# （来源逐条在队列里 · 探针都有登记 · §0.2 摘要与队列同数）+ 调度器不变量（收口有证据 ·
# umbrella 子任务全关 · 两层不漂移 · **收口指针祖先性** ✓）。
# ⚠ **本步的 job 标签曾是 `ledger`** ✗⇒✓（2026-10-15）：它原先是"`ledger` shard 1 的
# `if [ "${{ matrix.shard }}" = "1" ]` 里那一段"，而洞 `LEDGER-JOB-SKIPPED-ON-DOC-ONLY-PUSH`
# 已把 `ux-queue.py` 搬进**不设 `if:` 的 `queue-lint` job** ✓（同族的
# `LEDGER-SHARD1-REMAINING-GUARDS` 又把多会话守卫与 `compile-cache.py --selftest` 搬了过去 ✓）
# ⇒ 标签跟着改，别让"这一步对应哪个 job"悄悄漂移 ✗。
# ⚠ 此前 ci-local **根本不跑它** ✗ ⇒ **本地判据集 ⊂ CI 判据集** ⇒ `ledger (1)` 能
# **连续 20 轮 failure 而本地永远绿** ✗（实测 `38455544309` → `38675647735` = 20 轮 ✓）。
# ⚠ **ROOT 口径**（本步唯一的坑 ✗，两半都实测过）：队列是**主 checkout 单点**，而
# `ux-queue.py` 在**链接工作树**里没设 `SOKO_QUEUE_ROOT` 会**干净拒绝**（rc=2 + 人话 ✓）
# ⇒ 照抄 CI 那行会让**每棵工作树**里这一步假红 ✗。
# ⚠ 更隐蔽的另一半 ✗：**不许沿用继承来的 `SOKO_QUEUE_ROOT`** —— `compile-cache.py env <wt>`
# 会把它导成**工作树自己**（`scripts/compile-cache.py:411`），而工作树里那份
# `docs/ux-queue-events.jsonl` 是**开树时的过期副本**（实测：工作树 654 行 / `846a0779…`
# vs 主 checkout 655 行 / `b5e71ecb…`）⇒ 守卫会**静默审错对象**（"咬不住的守卫" ✗）。
# ⇒ **一律**用 `git worktree list` **第一行 = 主工作树**（`agent-worktree.sh:25` 的契约 ✓）
# 覆盖它；量不到（不是 git 仓库）就不设、让脚本按自己的位置回落 ✓。
# 秒级（实测 3.9s ✓）⇒ 与 docs-lint / status-lint 同档，**`--fast` 也跑** ✓
#（pre-push 正是要拦住 `ledger` 那种红的路径 ✓）。
# ⚠ **`--selftest` 故意没跟进来**（如实登记，不是漏了 ✓）：它 13.2s 且含**真并发**臂
# （8 路抢同一资源 ⇒ 时序敏感）⇒ 塞进 pre-push 等于把"假红"风险加到每次推送 ✗；
# CI 的 `queue-lint` job（不设 `if:` ⇒ 每推必跑）仍在跑它 ✓（要本地跑：`SOKO_QUEUE_ROOT=<主 checkout> python3 scripts/ux-queue.py --selftest`）。
run "gates：队列不变量（收口证据 · 祖先性 ✓）" "queue-lint" bash -c '
set -u
root=$(git worktree list --porcelain | sed -n "1s/^worktree //p")
if [ -n "$root" ]; then
  export SOKO_QUEUE_ROOT="$root"
  echo "  （队列单点 = $root ✓）"
fi
python3 scripts/ux-queue.py --check || exit 1'

# **队列的独立复核**（2026-10-15 · `SUB-orchestrator-attention-structure` 【1】/【2】 ✓）：
# 病根是**判据的所有权** —— `ux-queue.py` 同时定义队列语义/守卫/证据规则/**复核判据**
# ⇒ 工具错 = 账错 = 过程错，没有外部检查能发现它 ✗。`check-queue-views.py` 用
# **第二份实现**（自己折原始事件流）+ **纯 git 事实**（`merge-base --is-ancestor`）
# 与它**对拍** ⇒ 工具自报的结论只是**被告**，不是判据 ✓。
# ⚠ 它与上一步同源（都要 `SOKO_QUEUE_ROOT` 指向主 checkout ✓ —— 链接工作树里
# 那份 `docs/ux-queue-events.jsonl` 是过期副本 ✗）。
# 实测 1.5s（含 `ux-queue.py --json` 一次）⇒ 与 docs-lint 同档 ✓。
run "gates：队列独立复核（第二份实现 + 纯 git 事实 + 证据可复算 ratchet）" "queue-lint" bash -c '
set -u
root=$(git worktree list --porcelain | sed -n "1s/^worktree //p")
if [ -n "$root" ]; then export SOKO_QUEUE_ROOT="$root"; fi
python3 scripts/check-queue-views.py --selftest || exit 1
python3 scripts/check-queue-views.py || exit 1'

# **簿记预算**（2026-10-15 · 同上的【5】/【5'】 ✓）：用户实测本会话
# **生成物/台账 : 产品 ≈ 17 : 1** · **簿记类提交 56%** ⇒ 注意力结构病 ✗。
# 判据 = **两档**（告警线 ⇒ 只出声不判红 ✓ · 红线 ⇒ 判红 ✗；
# 照 `scripts/target-hygiene.py` 的先例 —— 理由见 `ci.yml` 同一步的注释 ✓）。
# ⚠ 要**全历史**（浅克隆 ⇒ rc=3 + 人话，绝不静默判过 ✗）；实测 ~1s ✓。
# ⭐ **两档的数字从 ratchet 状态读**（2026-10-16 修，洞 `BUDGET-WARN-VS-GATE-RED-INCONSISTENT`）：
# 改前这一行的标签把**种子**写死（`17:1 / 57%` · `19:1 / 60%`）✗，而判据读
# `docs/attention-budget-ratchet.json` 的**本轮 caps**（已收紧到 `16:1 / 55%` · `18:1 / 58%`）
# ⇒ **两处口径不同源**：读数落在告警档时，人看到「上一轮的 caps + exit=1」⇒ 误诊成
# "告警被当成红" ✗。⇒ 标签改由 `--label` 印（本脚本 = 那份状态的**唯一读者** ✓），
# 且**接缝判据**（`--seam`）机械咬住"又被写死" ✗（咬住证明：`docs/guard-bite.jsonl`）。
# ⚠ 本文件里**不许再出现 caps 的字面量**（`--seam` 咬这一条 ✓）—— 引用读数请只写数字。
run "gates：$(python3 scripts/check-attention-budget.py --label)" "queue-lint" bash -c '
python3 scripts/check-attention-budget.py --seam || exit 1
python3 scripts/check-attention-budget.py --selftest || exit 1
python3 scripts/check-attention-budget.py || exit 1'

# **横向阈值 ratchet**（2026-10-16 · 用户 2026-10-15【②】的**横向**落地）：上面那条是**簿记**
# 的两档；这条是**横向**那批装饰性上限（登记表 = 唯一出处：`scripts/threshold_ratchet.py` ✓）。
# 判：caps 单调不增 + 永不高于种子 · **接缝**（7/7 消费点真的读 ratchet ✗ 改回常量当场红）·
# 开轮纪律（基点后 > 40 笔没开轮 ⇒ 判红）✓。⚠ 实测 ~1.3s（**不跑** 4 个消费者 ⇒ 不需要 target/ ✓）。
run "gates：横向阈值 ratchet（状态 + 接缝 + 开轮纪律）" "queue-lint" bash -c '
python3 scripts/check-threshold-ratchet.py --selftest || exit 1
python3 scripts/check-threshold-ratchet.py || exit 1'

# **咬住证明台账**（2026-10-16 · 洞 `GUARD-BITE-NOT-IN-LOCAL-GATE` ✓）：`scripts/check-guard-bite.py`
# 此前**根本不在本脚本** ✗（实测 `grep -c` = **0**，而它的兄弟 `check-threshold-ratchet.py` = **2** ✓）
# ⇒ 又一处「**本地判据集 ⊂ CI 判据集**」：判据的红**只能走到 CI 才被发现** ✗
#（实测代价：一轮 CI = 33–35 分钟 ⇒ 本地白等一轮 ✗）。本步与 `ci.yml` 的 `queue-lint` job
# 那两步**逐字同源** ✓（`--job` 缺省 = `queue-lint` ✓ ⇒ 本地与 CI 的覆盖面一致 ✓）。
# ⚠ **ROOT 口径**（与上面两条队列步**同一个坑** ✗）：`check-queue-views.py` 那条证明的 `pass`
# 就是 `python3 scripts/check-queue-views.py`，而它在**链接工作树**里 rc=1（`ux-queue.py --json`
# rc=2：没设 `SOKO_QUEUE_ROOT` ⇒ **合法夹具假红** ✗ —— 实测未设 ROOT 时 `--check --all` rc=1 ✓）
# ⇒ 照上面两步的写法覆盖成**主工作树**（`git worktree list` 第一行 ✓）。
# ⚠ **耗时**（实测 `f878f88a` · 本机 · 已设 ROOT ✓）：`--selftest` **91.3s** · `--check --all` **70.0s**
# ⇒ 本步 **+161s**：`--fast` 从 **26.8s → ~188s**（≈3.1 分钟 ✓）—— 换来的是"台账的红在**本地**被咬住" ✓
#（对照：漏掉它的代价是**一轮 CI 33–35 分钟** ⇒ 仍然划算 ✓，但**如实登记这个数字** ✗ 不许含糊）。
# ⭐ **替代方案量过了、不采纳** ✗：增量口径 `--since`（不带 `--all`）实测 **63.6s** ⇒ **只省 9%**
#（成本是"**真跑**每条证明"、不是覆盖面 ⇒ 增量**不解决**耗时 ✗）⇒ 不值得为它放弃与 CI 同源 ✗。
# ⚠ `--selftest` 的第一条正对照**就是** `check(..., run_all=True, job=<job>)`（`check-guard-bite.py:1384`）
# ⇒ 它**已含** `--check --all`（91.3s 里 ~70s 是那一条 ✓）；两步都留着 = 与 CI 逐字同源 ✓
#（只留第一步可省 ~70s，但那要**假设**"selftest 永远含 --all" —— 那个假设**没有守卫** ⇒ 不省 ✗）。
run "gates：咬住证明台账（覆盖 + 逐条真跑 + 接线/可达性）" "queue-lint" bash -c '
set -u
root=$(git worktree list --porcelain | sed -n "1s/^worktree //p")
if [ -n "$root" ]; then
  export SOKO_QUEUE_ROOT="$root"
  echo "  （队列单点 = $root ✓）"
fi
python3 scripts/check-guard-bite.py --selftest || exit 1
python3 scripts/check-guard-bite.py --check --all || exit 1'

# **编译缓存判据自检**（2026-10-15 · 同族的 `LEDGER-SHARD1-REMAINING-GUARDS` ✓）：
# 它此前**只在 CI 跑** ✗（`scripts/soko` 的 gate 段跑的是 `status --quiet` —— 那是**预算
# 判红**，**不是**判据通道自检 ✗）⇒ 又一处"本地判据集 ⊂ CI 判据集"（本文件上面那条
# `CI-LOCAL-NO-QUEUE-CHECK` 的同族病 ✓）。纯函数级（只碰 `tempfile` ✓；实测在**非 git
# 目录** `/tmp` 下也 rc=0 ✓）· 实测 0.10–0.17s ⇒ 与 docs-lint 同档，`--fast` 也跑 ✓。
# ⚠ **`check-multi-session.py --selftest` 故意没跟进来**（如实登记 ✓，不是漏了）：
# 它有一条"工作区的 `docs/gaps/ledger.jsonl` 必须与 HEAD **逐字节一致**"的断言，而
# **主 checkout 上改台账是编排者的常态**（改完才 commit ✓）⇒ 那种状态下它 **rc=1** ✗
#（实测：合成夹具里给台账追加一行、不提交 ⇒ rc=1 + "✗ selftest 反例失败：工作区的
# `docs/gaps/ledger.jsonl` 与 HEAD 逐字节不一致" ✓）⇒ 塞进 pre-push 就是**假红生成器** ✗。
# 它在**链接工作树**里是有牙的 ✓（那里台账本来就该等于 HEAD ✓），而 CI（干净检出）与
# `queue-lint` job（每推必跑 ✓）都在跑它 ✓。
run "gates：编译缓存判据自检（回收顺序 / 预算反向验证）" "queue-lint" python3 scripts/compile-cache.py --selftest

# **真绿判据通道自检**（2026-10-16 · 洞 `CI-GREEN-SELFTEST-NOT-WIRED` ✓）：与上面那条**同族**
# —— 又一处"本地判据集 ⊂ CI 判据集"（`ci-green.py --selftest` 此前**不在任何 job / gate 里** ✗，
# `grep -rn ci-green .github/workflows/ scripts/ci-local.sh` **只命中一条注释** ✗）。
# 本步与 `ci.yml` 的 `queue-lint` job 里那一步**同一条命令** ✓（本地能先咬住，不必等 33 分钟的 CI）。
# ⚠ 离线 ✓ 亚秒级 ✓ 无前提（不读真台账、不碰 `target/`）⇒ 与 `docs-lint` 同档，**`--fast` 也跑** ✓。
# ⚠ **判据一条都没放宽** ✗：照原样跑（撤掉分类表一项 ⇒ **6/15 判红**，实测 ✓）。
run "gates：真绿判据通道自检（离线 · 不需要 gh / 网络）" "queue-lint" python3 scripts/ci-green.py --selftest

# **json 身份判据通道自检**（2026-10-16 · 洞 `CHECK-JSON-IDENTITY-NOT-WIRED-AT-ALL` ✓）：同族第三例
# —— `check-json-identity.py` 是 `AGENTS.md` 硬规则 1 的**发版红线**（全课程 `--json` 逐字节），
# 而它在 `.github/workflows/**` / `scripts/ci-local.sh` / `scripts/githooks/*` 里**全是 0 处** ✗
# （唯一跑到它的是 `scripts/release-preflight.sh:76` 的 `--sweep`，**opt-in** ⇒ 不传就不跑 ✗）；
# 更糟的是它的防空转通道 `--selftest` 改前**在任何环境里都跑不成** —— 硬编码
# `target/debug/sokonanoda`，而受管环境（本文件 :46 的 `CARGO_TARGET_DIR=/tmp/soko-target`）
# 里产物根本不在那个路径 ⇒ `FileNotFoundError` traceback（rc=1 是**崩**出来的，不是咬出来的 ✗）
# ⇒ 自检**接不进任何 gate** ✗。⇒ 本步：**判据通道自检**（三格：不同⇒1 · 相同⇒0 · 答不上⇒2 ✓），
# 与 `ci.yml` 的 `queue-lint` job 里那一步**同一条命令** ✓（本地先咬住，不必等 33 分钟的 CI ✓）。
# ⚠ 离线 ✓ 实测 1.2s ✓ **零前提**（自造假 CLI，不碰 `target/`）⇒ `--fast` 也跑 ✓。
# ⚠ 真 CLI 的全课程对拍**不在这里** ✗：它要一份"已发布 tag 的构建"当基线、251 个入口跑两遍
# ⇒ 属**发版大节点**（`AGENTS.md` 硬规则 1 + `scripts/release-preflight.sh --sweep` ✓），
# 塞进每推的本地 gate 只会是**假红生成器**（这里没有基线二进制 ✗）。
run "gates：json 身份判据通道自检（离线 · 不需要真 CLI）" "queue-lint" python3 scripts/check-json-identity.py --selftest

# ④ editor（CI job `editor`）
run "editor：stub 宿主" "editor" node editor/vscode/test-extension-host.js

# ⑤ e2e（CI job `e2e`）—— 慢 ✓、且要真 VS Code ✓ ⇒ 默认不跑 ✓
if [ "$WANT_E2E" = 1 ]; then
  run "e2e：真宿主（${SOKO_VSCODE_TEST_VERSION:-默认版本}）" "e2e" scripts/vscode-e2e.sh
else
  printf '\n▶ e2e：真宿主  —— **默认跳过** ✓（批次收尾时用 `--e2e` 跑一次 ✓，结果记 docs/e2e/ ✓）\n'
fi

summary
