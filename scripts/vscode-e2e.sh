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
    -h | --help)
      sed -n '2,26p' "$0"
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

echo "+ cargo build --release -p sokonanoda-lsp -p sokonanoda-cli（被测二进制）"
cargo build --release -p sokonanoda-lsp -p sokonanoda-cli --locked
echo "+ stage: editor/vscode/bin/<target>/（bundled 优先，必须是最新构建）"
(cd editor/vscode && node scripts/stage-lsp.js)

echo "+ vscode-test（VS Code ${test_version}，真宿主 + 真 LSP）"
set +e
(cd editor/vscode &&
  SOKO_E2E_LOG="$ext_log" \
    SOKO_VSCODE_TEST_VERSION="$test_version" \
    npm test) >"$raw_log" 2>&1
status=$?
set -e

passing=$(grep -oE '^ +[0-9]+ passing' "$raw_log" | tail -1 | grep -oE '[0-9]+' || echo 0)
failing=$(grep -oE '^ +[0-9]+ failing' "$raw_log" | tail -1 | grep -oE '[0-9]+' || echo 0)
pending=$(grep -oE '^ +[0-9]+ pending' "$raw_log" | tail -1 | grep -oE '[0-9]+' || echo 0)
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
trimmed="docs/e2e/logs/${date%%T*}-${short_sha}.log"
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
VSCODE_VERSION="$vscode_version" SERVER_LINE="$server_line" LSP_SHA="$lsp_sha" \
TRIMMED="$trimmed" python3 - <<'PY'
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
    },
    "exit": int(os.environ["STATUS"]),
    # 被测服务器：版本自述那行 + 二进制指纹前 16 位（能看出"测的是不是旧构建"）。
    "server": os.environ["SERVER_LINE"],
    "lsp_sha256_16": os.environ["LSP_SHA"],
    "log": os.environ["TRIMMED"],
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
