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
  local f; for f in "${FAIL[@]:-}"; do [ -n "$f" ] && printf '  ❌ %s\n' "$f"; done
  [ ${#FAIL[@]} -eq 0 ] && printf '  ⇒ **本地全绿 ✓**：可以 push 了 ✓（CI 只做平台矩阵确认 ✓）\n' \
                        || printf '  ⇒ **别 push** ✗：先修上面那条 ✓\n'
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
# 判据 = 两条 **ratchet**（只许减不许增 ✓，照 `docs-budget.json` 的先例）：
# `RATIO_MAX = 17.0` · `BOOKKEEPING_SHARE_MAX = 0.57`。
# ⚠ 要**全历史**（浅克隆 ⇒ rc=3 + 人话，绝不静默判过 ✗）；实测 ~1s ✓。
run "gates：簿记预算（生成物:产品 ≤ 17 · 簿记提交 ≤ 57%）" "queue-lint" bash -c '
python3 scripts/check-attention-budget.py --selftest || exit 1
python3 scripts/check-attention-budget.py || exit 1'

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

# ④ editor（CI job `editor`）
run "editor：stub 宿主" "editor" node editor/vscode/test-extension-host.js

# ⑤ e2e（CI job `e2e`）—— 慢 ✓、且要真 VS Code ✓ ⇒ 默认不跑 ✓
if [ "$WANT_E2E" = 1 ]; then
  run "e2e：真宿主（${SOKO_VSCODE_TEST_VERSION:-默认版本}）" "e2e" scripts/vscode-e2e.sh
else
  printf '\n▶ e2e：真宿主  —— **默认跳过** ✓（批次收尾时用 `--e2e` 跑一次 ✓，结果记 docs/e2e/ ✓）\n'
fi

summary
