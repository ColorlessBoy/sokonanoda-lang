#!/usr/bin/env bash
# 性能台账（例行化留档）：跑全部性能套件，把每个阶段的实测值连同环境元数据
# 追加进 `docs/perf/ledger.jsonl`（提交进仓库，供跨版本/跨机器对比），并覆盖
# 一份便于阅读的 `docs/perf/latest.json`。
#
# 与 `scripts/perf-report.sh` 的分工：
#   * perf-report.sh  → 人类可读的 PERF 行（CI artifact 用它，只看"这一版多少"）
#   * perf-ledger.sh  → **机器可读**的 JSON 记录（分阶段、可累积，回答"哪一环退化了"）
#
# 用法：
#   scripts/perf-ledger.sh                 # 缺省：CLI 用 release 二进制（用户真实路径）
#   scripts/perf-ledger.sh --debug-cli     # CLI 也用测试 profile 的 debug 二进制（对比用）
#   scripts/perf-ledger.sh --no-append     # 只写 latest.json，不追加 ledger（预览）
#
# 记录为什么分两个 profile：`[profile.test] opt-level = 3`，所以 front/lsp 的
# 测试数值已经是优化过的；而 `cargo test` 为 CLI 集成测试编的 **二进制** 走
# `[profile.dev]`（opt-level 0），冷编译会慢一个量级——那不是用户看到的数字。
# 扩展随包发的是 release 二进制，所以 CLI 一律用 release 量。
set -euo pipefail
cd "$(dirname "$0")/.."

cli_profile="release"
append=1
for arg in "$@"; do
  case "$arg" in
    --debug-cli) cli_profile="debug" ;;
    --no-append) append=0 ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "error: unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
sha=$(git rev-parse HEAD)
short_sha=$(git rev-parse --short HEAD)
date=$(date -u +%FT%TZ)
raw=$(mktemp)
trap 'rm -f "$raw"' EXIT

echo "+ compiler (single file): crates/front/tests/perf.rs"
cargo test -q -p sokonanoda-front --test perf --locked -- --nocapture 2>&1 \
  | grep -E '^PERF' | tee -a "$raw" || true
echo "+ compiler (project closure): crates/front/tests/perf_project.rs"
cargo test -q -p sokonanoda-front --test perf_project --locked -- --nocapture 2>&1 \
  | grep -E '^PERF' | tee -a "$raw" || true
echo "+ editor interaction (LSP, incl. project): crates/lsp/src/tests/perf.rs"
cargo test -q -p sokonanoda-lsp --lib --locked -- perf_ --nocapture 2>&1 \
  | grep -E '^PERF' | tee -a "$raw" || true
echo "+ CLI end-to-end (project, $cli_profile binary): crates/cli/tests/perf_project.rs"
if [ "$cli_profile" = "release" ]; then
  cargo test -q --release -p sokonanoda-cli --test perf_project --locked -- --nocapture 2>&1 \
    | grep -E '^PERF' | tee -a "$raw" || true
else
  cargo test -q -p sokonanoda-cli --test perf_project --locked -- --nocapture 2>&1 \
    | grep -E '^PERF' | tee -a "$raw" || true
fi

mkdir -p docs/perf
dirty="false"
if [ -n "$(git status --porcelain)" ]; then dirty="true"; fi

VERSION="$version" SHA="$sha" SHORT_SHA="$short_sha" DATE="$date" CLI_PROFILE="$cli_profile" \
DIRTY="$dirty" RAW="$raw" APPEND="$append" python3 - <<'PY'
import json, os, pathlib, platform, sys

records = []
for line in pathlib.Path(os.environ["RAW"]).read_text().splitlines():
    if line.startswith("PERFJSON "):
        records.append(json.loads(line[len("PERFJSON "):]))

entry = {
    "schema": "soko.perf-ledger/1",
    "version": os.environ["VERSION"],
    "commit": os.environ["SHA"],
    # 记录时工作区是否有未提交改动：有 ⇒ 这条数字对应的代码比 commit 新。
    "dirty": os.environ["DIRTY"] == "true",
    "date": os.environ["DATE"],
    "cli_profile": os.environ["CLI_PROFILE"],
    "host": {
        "system": platform.system(),
        "machine": platform.machine(),
        "release": platform.release(),
    },
    "records": records,
}
latest = pathlib.Path("docs/perf/latest.json")
latest.write_text(json.dumps(entry, indent=2, ensure_ascii=False) + "\n")
if os.environ["APPEND"] == "1":
    with pathlib.Path("docs/perf/ledger.jsonl").open("a") as handle:
        handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
print(f"\nledger: {len(records)} records · v{entry['version']} {os.environ['SHORT_SHA']} "
      f"({entry['host']['system']}/{entry['host']['machine']}, cli={entry['cli_profile']})")
print(f"wrote {latest}")
if os.environ["APPEND"] == "1":
    print("appended docs/perf/ledger.jsonl")
PY
