#!/usr/bin/env bash
# **单场景性能快跑**：改完一个环节，立刻知道"快了没有"以及"有没有把别处弄慢"。
#
# 为什么需要它：`scripts/perf-ledger.sh` 跑全部四个套件（分钟级），不能每环节跑。
# 这里只跑**一个过滤**（cargo test 的测试名子串），十几秒出数字，并与台账里
# 上一次同名 `(scope, case)` 的记录比一比。
#
# 用法：
#   scripts/perf-check.sh --case course          # 真实课程闭包那几条（perf_course）
#   scripts/perf-check.sh --case perf_project    # 项目层那几条
#   scripts/perf-check.sh --case did_open        # 名字含 did_open 的
#   scripts/perf-check.sh --case perf_project --suite cli   # CLI 那条（要 release 构建，慢）
#   scripts/perf-check.sh --list                 # 台账里有哪些 (scope, case)
#
# **默认只跑快的三个套件**（lsp / front / front-project）：CLI 那条要走 release
# 构建，第一次会多花一两分钟，不适合每环节跑——要它就显式 `--suite cli`。
#
# 退出码：0 = 跑完且无退化；1 = 有 case 退化超过阈值（默认 25%，`--threshold` 可调）；
#         2 = 用法错误或没跑到任何 case。
#
# 口径（`docs/PERF.md` 的纪律）：一律 `--test-threads=1`（并行会把单次成本放大 3–4×）；
# 与台账比较时**优先比 `best_ms`**，超出 ±25% 先复测再下结论。

set -u
cd "$(dirname "$0")/.." || exit 2

filter=""
threshold=25
list_only=0
suite="fast"
while [ $# -gt 0 ]; do
  case "$1" in
    --case) shift; filter="${1:-}"; [ -n "$filter" ] || { echo "error: --case 需要一个值" >&2; exit 2; } ;;
    --threshold) shift; threshold="${1:-25}" ;;
    --suite) shift; suite="${1:-fast}"; case "$suite" in fast | cli | all) ;; *) echo "error: --suite 只吃 fast/cli/all" >&2; exit 2 ;; esac ;;
    --list) list_only=1 ;;
    -h | --help) sed -n '2,22p' "$0"; exit 0 ;;
    *) echo "error: unknown flag: $1" >&2; exit 2 ;;
  esac
  shift
done

if [ "$list_only" = 1 ]; then
  python3 - <<'PY'
import json, pathlib
path = pathlib.Path("docs/perf/ledger.jsonl")
if not path.exists():
    print("（还没有台账：先跑 scripts/perf-ledger.sh）")
    raise SystemExit(0)
entry = json.loads(path.read_text(encoding="utf-8").strip().splitlines()[-1])
print(f"台账最后一条：v{entry['version']} {entry['commit'][:7]} ({entry['date']}, cli={entry['cli_profile']})")
for r in entry["records"]:
    ms = r.get("best_ms", r.get("ms"))
    print(f"  {r.get('scope','?'):14s} {r.get('case','?'):34s} {ms}ms")
PY
  exit 0
fi

[ -n "$filter" ] || { echo "error: 需要 --case <测试名子串>（--list 看台账里有什么）" >&2; exit 2; }

raw=$(mktemp)
trap 'rm -f "$raw"' EXIT

echo "+ 跑名字含 \"$filter\" 的 perf 用例（串行，suite=${suite}）"
# 过滤不中的套件会秒退（cargo 只在有匹配时才真跑）。
DEVELOPER_DIR="${DEVELOPER_DIR:-}" cargo test -q -p sokonanoda-lsp --lib --locked -- "$filter" \
  --nocapture --test-threads=1 2>&1 | grep -E '^PERFJSON ' | tee -a "$raw" >/dev/null || true
DEVELOPER_DIR="${DEVELOPER_DIR:-}" cargo test -q -p sokonanoda-front --test perf --locked -- "$filter" \
  --nocapture --test-threads=1 2>&1 | grep -E '^PERFJSON ' | tee -a "$raw" >/dev/null || true
DEVELOPER_DIR="${DEVELOPER_DIR:-}" cargo test -q -p sokonanoda-front --test perf_project --locked -- "$filter" \
  --nocapture --test-threads=1 2>&1 | grep -E '^PERFJSON ' | tee -a "$raw" >/dev/null || true
if [ "$suite" != "fast" ]; then
  DEVELOPER_DIR="${DEVELOPER_DIR:-}" cargo test -q --release -p sokonanoda-cli --test perf_project --locked -- "$filter" \
    --nocapture --test-threads=1 2>&1 | grep -E '^PERFJSON ' | tee -a "$raw" >/dev/null || true
fi

RAW="$raw" THRESHOLD="$threshold" python3 - <<'PY'
import json
import os
import pathlib
import sys

raw = pathlib.Path(os.environ["RAW"])
records = []
for line in raw.read_text(encoding="utf-8").splitlines():
    if line.startswith("PERFJSON "):
        records.append(json.loads(line[len("PERFJSON "):]))
if not records:
    print("没有跑到任何 case——检查 --case 的名字（cargo test 按**测试函数名**过滤）。", file=sys.stderr)
    raise SystemExit(2)

# 台账里上一次的 (scope, case) → ms
baseline = {}
ledger = pathlib.Path("docs/perf/ledger.jsonl")
if ledger.exists():
    for line in ledger.read_text(encoding="utf-8").strip().splitlines()[-5:]:
        try:
            entry = json.loads(line)
        except Exception:  # noqa: BLE001
            continue
        for r in entry.get("records", []):
            ms = r.get("best_ms", r.get("ms"))
            if ms is not None:
                baseline[(r.get("scope"), r.get("case"))] = ms

threshold = float(os.environ["THRESHOLD"])
print()
print(f"{'scope':14s} {'case':34s} {'这次':>10s} {'台账上次':>10s} {'变化':>8s}")
print("-" * 80)
regressed = []
for r in records:
    ms = r.get("best_ms", r.get("ms"))
    key = (r.get("scope"), r.get("case"))
    before = baseline.get(key)
    if before is None:
        delta = "（无基线）"
    else:
        pct = (ms - before) / before * 100.0
        delta = f"{pct:+.1f}%"
        if pct > threshold:
            regressed.append((key, ms, before, pct))
    print(f"{r.get('scope','?'):14s} {r.get('case','?'):34s} {ms:>8}ms {str(before or '-'):>10s} {delta:>8s}")

print()
if regressed:
    print(f"退化超过 {threshold:.0f}% 的 case：", file=sys.stderr)
    for (scope, case), ms, before, pct in regressed:
        print(f"  {scope}/{case}: {before}ms → {ms}ms（{pct:+.1f}%）", file=sys.stderr)
    print("先复测一次；仍然退化就查这一版改了什么（docs/PERF.md 的判读纪律）。", file=sys.stderr)
    raise SystemExit(1)
print(f"没有超过 {threshold:.0f}% 的退化。")
PY
