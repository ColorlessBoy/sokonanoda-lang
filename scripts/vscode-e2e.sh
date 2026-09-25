#!/usr/bin/env bash
# 例行化的**真实 VS Code 集成测试**（@vscode/test-electron）+ 结果记账。
#
# 为什么单独一个脚本（而不是直接 `npm test`）：
#   1. 集成测试跑的是**扩展实际会用的那份服务器**——bundled `bin/<target>/`。
#      `bin/` 是 ignore 的目录，很容易停在几天前的旧二进制上（第一次例行跑就是
#      这样：测试对着 0.20.0 的服务器断言 0.58.0 的功能）。所以这里**先构建
#      release、再 stage**，保证被测二进制就是刚构建的。
#   2. 结果要**留档**：失败/通过都往 `docs/e2e/ledger.jsonl` 追加一条机器可读
#      记录（`soko.e2e/1`），并把人读摘要写进 `docs/e2e/latest.json`。
#   3. 失败时要能查：`SOKO_E2E_LOG` 让扩展把接线事实（服务器路径/客户端状态/
#      `soko/project` 结果）写一行文件日志——扩展宿主的 console 在 vscode-test
#      输出里看不到，这是 e2e 卡住时的第一现场。
#
# 用法：
#   scripts/vscode-e2e.sh                       # 默认钉住下面这个"已知良好"版本
#   scripts/vscode-e2e.sh --version stable      # 跟随最新稳定版（每次升级会重下 ~300MB）
#   scripts/vscode-e2e.sh --version 1.106.0     # 试某个具体版本（如声明的最低版本）
#
# **单环节快跑（L4 层，计划 T-017）**：
#   scripts/vscode-e2e.sh --grep "declarations panel" --profile debug --no-build
#     --grep <名字>         只跑名字匹配的用例（透传 SOKO_E2E_GREP；不设则跑全量，行为不变）
#     --profile debug|release  用哪个构建（默认 release）；debug 不跑 release 构建，快得多
#     --no-build            二进制比 crates/ 下任何 .rs 新时，跳过构建与 stage
#   ⚠ `--no-build` 可能让你测到旧二进制——台账里的 `lsp_sha256_16` 与 `dirty`
#     就是判读这件事的两个答案，命中时会额外打印 bin/ 里那份的 mtime 与 hash。
#
# 版本策略（0.58.0 定的，理由见 docs/E2E.md §5）：上游 `@vscode/test-cli` 的默认是
# **stable 频道**（官方文档与官方 sample 都不钉版本），但"例行化 + 台账"要求可复现：
# 同一份代码在 stable 升级那天会突然换宿主，历史条目没法比。所以本地默认钉一个
# 已知良好版本，CI 矩阵显式给版本（**改这里时同步改 ci.yml 的 e2e matrix**）。
#
# 前置：`editor/vscode` 已 `npm install`；macOS/Linux 桌面会话（Linux CI 用
# `xvfb-run -a` 包一层，见 `.github/workflows/ci.yml` 的同一套用例）。
# 网络受限时加 npm 的代理变量（`@vscode/test-electron` **只认**这两个，不读
# `HTTPS_PROXY`）：
#   npm_config_https_proxy=http://127.0.0.1:7890 scripts/vscode-e2e.sh --version 1.106.0
# 退出码：0 = 全绿；1 = 有用例失败；2 = 用法错误；3 = 前置缺失（cargo/npm/node_modules）。
set -euo pipefail
cd "$(dirname "$0")/.."

default_version="1.138.0"
test_version="${SOKO_VSCODE_TEST_VERSION:-$default_version}"
profile="release"
no_build=0
grep_name=""
while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      shift
      test_version="${1:-}"
      [ -n "$test_version" ] || {
        echo "error: --version 需要一个值" >&2
        exit 2
      }
      ;;
    --grep)
      shift
      grep_name="${1:-}"
      [ -n "$grep_name" ] || {
        echo "error: --grep 需要一个值" >&2
        exit 2
      }
      ;;
    --profile)
      shift
      profile="${1:-}"
      case "$profile" in
        debug | release) ;;
        *)
          echo "error: --profile 只吃 debug/release，收到 ${profile:-（空）}" >&2
          exit 2
          ;;
      esac
      ;;
    --no-build) no_build=1 ;;
    -h | --help)
      sed -n '2,34p' "$0"
      exit 0
      ;;
    *)
      echo "error: unknown flag: $1" >&2
      exit 2
      ;;
  esac
  shift
done

command -v cargo >/dev/null 2>&1 || {
  echo "error: 需要 cargo（构建被测服务器）；贡献者路径见 skills/sokonanoda-dev" >&2
  exit 3
}
command -v node >/dev/null 2>&1 || {
  echo "error: 需要 node> = 18" >&2
  exit 3
}
[ -d editor/vscode/node_modules ] || {
  echo "error: editor/vscode/node_modules 不存在 —— 先 (cd editor/vscode && npm install)" >&2
  exit 3
}

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
sha=$(git rev-parse HEAD)
short_sha=$(git rev-parse --short HEAD)
# `dirty` 的语义是"这条结果对应的代码比 commit 新"——必须在**跑之前**取，
# 否则下面写出的裁剪日志（新文件）会让每次记录都变成 dirty=true（第一版就这么错过）。
dirty="false"
if [ -n "$(git status --porcelain)" ]; then dirty="true"; fi
date=$(date -u +%FT%TZ)
run_dir=$(mktemp -d)
trap 'rm -rf "$run_dir"' EXIT
ext_log="$run_dir/extension.log"
raw_log="$run_dir/vscode-test.log"

host_target() {
  case "$(uname -s)/$(uname -m)" in
    Darwin/arm64) echo "darwin-arm64" ;;
    Darwin/x86_64) echo "darwin-x64" ;;
    Linux/x86_64) echo "linux-x64" ;;
    Linux/aarch64 | Linux/arm64) echo "linux-arm64" ;;
    MINGW* | MSYS* | CYGWIN*) echo "win32-x64" ;;
    *) echo "" ;;
  esac
}

staged="editor/vscode/bin/$(host_target)/sokonanoda-lsp"

# `--no-build` **无条件跳过**构建与 stage（名字就是这个意思），只在"看起来更旧"时警告。
# 为什么不做成"自动判断"：`find crates -name '*.rs' -newer` 在 checkout 之后对**所有**
# 文件都成立（mtime 都是"现在"），自动判断会永远不生效——实测踩到。
# 判读"测的是哪份二进制"靠台账里的 `lsp_sha256_16` 与下面这行打印。
if [ "$no_build" = 1 ]; then
  if [ ! -f "$staged" ]; then
    echo "error: --no-build 但 $staged 不存在——先跑一次不带 --no-build 的，或 scripts/dev-loop.sh stage-debug" >&2
    exit 3
  fi
  echo "+ --no-build：跳过构建与 stage，直接测 bin/ 里那份"
  newer=$(find crates -name '*.rs' -newer "$staged" -print -quit 2>/dev/null || true)
  if [ -n "$newer" ]; then
    echo "  ⚠ 警告：$newer 比 staged 的二进制新——你测的可能不是当前源码（台账会记 lsp_sha256_16）"
  fi
  if command -v shasum >/dev/null 2>&1; then
    echo "  staged：$(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' "$staged" 2>/dev/null || stat -c '%y' "$staged") · sha256:$(shasum -a 256 "$staged" | cut -c1-16)"
  fi
else
  if [ "$profile" = "release" ]; then
    echo "+ cargo build --release -p sokonanoda-lsp -p sokonanoda-cli（被测二进制）"
    cargo build --release -p sokonanoda-lsp -p sokonanoda-cli --locked
  else
    echo "+ cargo build -p sokonanoda-lsp -p sokonanoda-cli（debug 被测二进制）"
    cargo build -p sokonanoda-lsp -p sokonanoda-cli --locked
  fi
  echo "+ stage: editor/vscode/bin/<target>/（bundled 优先，必须是最新构建；profile=${profile}）"
  (cd editor/vscode && node scripts/stage-lsp.js --profile "$profile")

  # **stage 后断言版本**（2026-09-25 §9 ㉛ 第二条 ✓）：刚 stage 的 CLI 自述版本必须
  # 等于仓库版本 ⇒ 否则"被测二进制**不是当前源码**" ✗——本会话连续三轮真宿主 e2e
  # 被 `bin/` 里那份 0.65.0 旧件污染（症状是告警里 `server 运行 0.65.0 != 扩展 v0.67.0`），
  # 而**告警是可以被忽略的** ✗ ⇒ 这里把它变成**一次判红** ✓
  # （AGENTS：**咬不住的守卫等于没有** ✓）。只在"刚构建+stage"这条路上断言 ✓：
  # `--no-build` 是显式选择"可能测旧件" ✓，那条路由台账的 `dirty`/`lsp_sha256_16` 判读 ✓。
  repo_version=$(node "$REPO_ROOT/scripts/soko" version --json 2>/dev/null |
    sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
  staged_cli="$REPO_ROOT/editor/vscode/bin/$(host_target)/sokonanoda"
  if [ -n "$repo_version" ] && [ -x "$staged_cli" ]; then
    staged_version=$("$staged_cli" --version 2>/dev/null | awk '{print $NF}')
    if [ "$staged_version" != "$repo_version" ]; then
      echo "error: staged CLI 报 \`$staged_version\`，仓库是 \`$repo_version\` —— 被测二进制不是当前源码 ✗" >&2
      echo "       多半是构建落在 CARGO_TARGET_DIR 而 stage 没跟（见 REQUIREMENTS.md §9 ㉛）" >&2
      exit 3
    fi
    echo "+ stage 版本断言 ✓ staged=$staged_version · repo=$repo_version"
  else
    echo "  ⚠ stage 版本断言跳过（repo_version='${repo_version}' 或 staged CLI 不可执行）"
  fi
fi

echo "+ vscode-test（VS Code ${test_version}，真宿主 + 真 LSP）"
set +e
if [ -n "$grep_name" ]; then
  echo "+ vscode-test（只跑匹配 \"$grep_name\" 的用例）"
else
  echo "+ vscode-test（全量）"
fi
# **每次跑一个全新的编译缓存目录**（T-A60）：否则用例的"冷开"会命中上一次跑留下的
# 条目，`reopening a project unit hits the compile cache` 这类**冷/热对比**用例就
# 变成"拿两个热开比大小"（而且结果取决于上一次谁跑过）。用例自己也会读这个变量来
# 清缓存，所以必须是显式给的。
e2e_cache="$(mktemp -d)"
# **模块根产物也要每次清干净**（T-B5 / R-3）：项目闭包条目现在落在夹具的
# `<模块根>/.sokonanoda/compiled/`，它**不在** `SOKONANODA_CACHE_DIR` 里 ⇒ 不清的话
# "冷开"用例在下一次跑就变成热开（结果取决于上一次谁跑过 ✗ —— 与上面同一条纪律）。
# 这些目录是**自忽略**的产物（`.sokonanoda/.gitignore` 内容是一行 `*`），删掉安全。
find editor/vscode/src/test/fixtures -type d -name .sokonanoda -prune -exec rm -rf {} + 2>/dev/null || true
# 别把 111 行那个 `$run_dir` 的 trap 覆盖掉——两个目录都要清。
trap 'rm -rf "$run_dir" "$e2e_cache"' EXIT
(cd editor/vscode &&
  SOKO_E2E_LOG="$ext_log" \
    SOKO_VSCODE_TEST_VERSION="$test_version" \
    SOKO_E2E_GREP="$grep_name" \
    SOKONANODA_CACHE_DIR="$e2e_cache" \
    npm test) >"$raw_log" 2>&1
status=$?
set -e

passing=$(grep -oE '^ +[0-9]+ passing' "$raw_log" | tail -1 | grep -oE '[0-9]+' || echo 0)
failing=$(grep -oE '^ +[0-9]+ failing' "$raw_log" | tail -1 | grep -oE '[0-9]+' || echo 0)
pending=$(grep -oE '^ +[0-9]+ pending' "$raw_log" | tail -1 | grep -oE '[0-9]+' || echo 0)
# **失败用例名**（2026-09-24 加）：以前台账只有计数，CI 红了要从 runner 的临时日志里
# 找用例名——产物里根本没有（`log` 字段只有路径）。实测吃过这个亏：ubuntu 两个版本
# 同时红，却只能猜"大概是那条比值断言"。mocha 的失败列表是 `  1) <用例名>` 形状
# （套件头那一行是 `1) <套件名>`，靠"不是套件名"过滤不掉就都留着——给人和 agent 读）。
failing_cases=$(grep -oE '^ +[0-9]+\) .+' "$raw_log" | sed -E 's/^ +[0-9]+\) //' | sort -u | paste -sd '|' - || true)
# **失败断言原文**（2026-09-24 加）：只有用例名还是不够——上一轮为了拿到"到底哪条
# 断言红了"白跑了两轮 CI（产物里没有当轮日志）。这里把错误行一起记进台账。
failing_details=$(grep -aE '^ +(AssertionError|Error)[: ]' "$raw_log" | head -3 | tr '\n' ' ' | cut -c1-400 || true)
vscode_version=$(grep -m1 -oE 'Validated version: [0-9.]+' "$raw_log" | grep -oE '[0-9.]+' || echo "unknown")
server_line=$(grep -m1 'server-version：' "$raw_log" | sed 's/.*server-version：//' || true)
# macOS 有 shasum、Linux 常用 sha256sum——两个都试，取不到就记 unknown（不让记账
# 因为一个哈希工具缺失而整个失败）。
lsp_path=$(ls editor/vscode/bin/*/sokonanoda-lsp 2>/dev/null | head -1 || true)
lsp_sha="unknown"
if [ -n "$lsp_path" ]; then
  if command -v sha256sum >/dev/null 2>&1; then
    lsp_sha=$(sha256sum "$lsp_path" | cut -c1-16)
  elif command -v shasum >/dev/null 2>&1; then
    lsp_sha=$(shasum -a 256 "$lsp_path" | cut -c1-16)
  fi
fi

mkdir -p docs/e2e/logs
# 文件名带 VS Code 版本：CI 矩阵同一天同一 commit 会跑多个版本（1.138.0 / 1.106.0 …），
# 不带版本号会互相覆盖（`scripts/e2e-merge.py` 也按记录里的这个名字回填日志）。
trimmed="docs/e2e/logs/${date%%T*}-${short_sha}-vc${test_version}.log"
{
  echo "# scripts/vscode-e2e.sh — VS Code $vscode_version · extension v$version · $sha"
  echo "# 结果：$passing passing / $failing failing / $pending pending（exit=${status}）"
  echo
  echo "## doctor（服务器解析与版本自述）"
  sed -n '/--- doctor ---/,/^\[info\] 6\/6/p' "$raw_log" || true
  echo
  echo "## 用例"
  grep -E '^ +[✔✗]|^ +[0-9]+\)' "$raw_log" || true
  echo
  echo "## 扩展接线日志（SOKO_E2E_LOG）"
  cat "$ext_log" 2>/dev/null || echo "(no extension log)"
  if [ "$failing" != "0" ]; then
    echo
    echo "## 失败详情"
    sed -n '/^  [0-9]*) sokonanoda extension/,$p' "$raw_log" | head -120
  fi
} >"$trimmed"

VERSION="$version" SHA="$sha" SHORT_SHA="$short_sha" DATE="$date" \
DIRTY="$dirty" STATUS="$status" PASSING="$passing" FAILING="$failing" PENDING="$pending" \
FAILING_CASES="$failing_cases" FAILING_DETAILS="$failing_details" \
VSCODE_VERSION="$vscode_version" SERVER_LINE="$server_line" LSP_SHA="$lsp_sha" \
TRIMMED="$trimmed" PROFILE="$profile" GREP="$grep_name" python3 - <<'PY'
import json, os, pathlib, platform

entry = {
    "schema": "soko.e2e/1",
    "kind": "vscode-integration",
    "version": os.environ["VERSION"],
    "commit": os.environ["SHA"],
    # 记录时工作区是否有未提交改动：有 ⇒ 这条结果对应的代码比 commit 新。
    "dirty": os.environ["DIRTY"] == "true",
    "date": os.environ["DATE"],
    "host": {
        "system": platform.system(),
        "machine": platform.machine(),
        "release": platform.release(),
    },
    "vscode": os.environ["VSCODE_VERSION"],
    "tests": {
        "passed": int(os.environ["PASSING"]),
        "failed": int(os.environ["FAILING"]),
        "pending": int(os.environ["PENDING"]),
        # **哪些用例失败了**（不是只有计数）：CI 产物里能直接读到，
        # 不用再去 runner 的临时日志里捞（2026-09-24 加）。
        "failing_cases": [
            name for name in os.environ.get("FAILING_CASES", "").split("|") if name
        ],
        # 失败**断言原文**：只给用例名还要再跑一轮才知道红在哪（吃过这个亏）。
        "failing_details": os.environ.get("FAILING_DETAILS", ""),
    },
    "exit": int(os.environ["STATUS"]),
    # 被测服务器：版本自述那行 + 二进制指纹前 16 位（能看出"测的是不是旧构建"）。
    "server": os.environ["SERVER_LINE"],
    "lsp_sha256_16": os.environ["LSP_SHA"],
    "log": os.environ["TRIMMED"],
    # 单环节快跑（T-017）：记下这次用的构建 profile 与用例过滤，
    # 否则「1 passing」读不出"是只跑了一个还是全跑完了"。
    "profile": os.environ["PROFILE"],
    "grep": os.environ["GREP"] or None,
}
latest = pathlib.Path("docs/e2e/latest.json")
latest.write_text(json.dumps(entry, indent=2, ensure_ascii=False) + "\n")
with pathlib.Path("docs/e2e/ledger.jsonl").open("a") as handle:
    handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
print(
    f"\ne2e: {entry['tests']['passed']} passed / {entry['tests']['failed']} failed "
    f"(v{entry['version']} {os.environ['SHORT_SHA']}, VS Code {entry['vscode']}, "
    f"server {entry['server'] or 'n/a'})"
)
print(f"wrote {latest} · {os.environ['TRIMMED']}")
print("appended docs/e2e/ledger.jsonl")
PY

if [ "$status" -ne 0 ]; then
  echo "sokonanoda: VS Code 集成测试失败（完整日志见 $raw_log 的副本：${trimmed}）" >&2
fi
exit "$status"
