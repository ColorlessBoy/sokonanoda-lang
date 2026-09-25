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
run "gates：课程门禁"   "gates" python3 courses/set-theory/tools/check.py
# **缺口台账带 `--strict`**（2026-09-25 ✓）：CI 上超时**跳过**（慢 runner 是环境事实 ✓），
# 但**快机器上必须跑完** ✓ —— 否则守卫就被"跳过"架空了 ✗（用户："每次都是它出问题，
# 但是从来不改" ✗ ⇒ 这一刀就是"本地兜底判据" ✓）。
run "gates：缺口台账（--strict ✓ 本地必须跑完）" "ledger" python3 scripts/gap.py check --strict
run "gates：版本单一源" "gates" python3 scripts/bump.py --check
run "gates：记法规则"   "gates" python3 scripts/notation-lint.py
run "gates：记法路径守卫" "gates" python3 scripts/audit-notation-paths.py
run "gates：wire 字段守卫" "gates" python3 scripts/audit-wire-fields.py
run "gates：两个守卫的自检（反向验证 ✓）" "gates" bash -c \
  'python3 scripts/audit-notation-paths.py --self-test && python3 scripts/audit-wire-fields.py --selftest'

# ④ editor（CI job `editor`）
run "editor：stub 宿主" "editor" node editor/vscode/test-extension-host.js

# ⑤ e2e（CI job `e2e`）—— 慢 ✓、且要真 VS Code ✓ ⇒ 默认不跑 ✓
if [ "$WANT_E2E" = 1 ]; then
  run "e2e：真宿主（${SOKO_VSCODE_TEST_VERSION:-默认版本}）" "e2e" scripts/vscode-e2e.sh
else
  printf '\n▶ e2e：真宿主  —— **默认跳过** ✓（批次收尾时用 `--e2e` 跑一次 ✓，结果记 docs/e2e/ ✓）\n'
fi

summary
