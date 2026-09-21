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
def representative_ms(record: dict):
    """一条记录的代表性耗时。

    有些 case 没有 `ms`/`best_ms`（例如 `did_open_and_keystroke` 只有
    `open_ms` + `keystroke_ms`）——取其中最大的那个 `*_ms`，都没有就 `None`
    （`None` 会让下面的格式化炸掉，实测踩到）。
    """
    for key in ("best_ms", "ms"):
        value = record.get(key)
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return float(value)
    times = [
        float(v)
        for k, v in record.items()
        if (k == "ms" or k.endswith("_ms")) and isinstance(v, (int, float)) and not isinstance(v, bool)
    ]
    return max(times) if times else None


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
            ms = representative_ms(r)
            if ms is not None:
                baseline[(r.get("scope"), r.get("case"), str(r.get("entry", "")))] = ms

threshold = float(os.environ["THRESHOLD"])
print()
print(f"{'scope':13s} {'case':26s} {'entry':22s} {'这次':>9s} {'台账上次':>9s} {'变化':>8s}")
print("-" * 96)
regressed = []
for r in records:
    ms = representative_ms(r)
    key = (r.get("scope"), r.get("case"), str(r.get("entry", "")))
    before = baseline.get(key)
    if ms is None:
        delta = "（这条没有 *_ms 字段）"
    elif before is None:
        delta = "（无基线）"
    elif before <= 0:
        # 基线是 0ms（"不该重编"那类哨兵实测就是 0）——除法会炸（实测踩到）。
        delta = "（基线 0ms，只看这次）"
        if ms > threshold:
            regressed.append((key, ms, before, float("inf")))
    else:
        pct = (ms - before) / before * 100.0
        delta = f"{pct:+.1f}%"
        if pct > threshold:
            regressed.append((key, ms, before, pct))
    entry_name = str(r.get("entry", "")).split("/")[-1][:22]
    shown = f"{ms:>7.0f}ms" if ms is not None else "       -"
    print(f"{r.get('scope','?'):13s} {r.get('case','?'):26s} {entry_name:22s} {shown} {str(before or '-'):>9s} {delta:>8s}")

print()
if regressed:
    print(f"退化超过 {threshold:.0f}% 的 case：", file=sys.stderr)
    for (scope, case, entry_name), ms, before, pct in regressed:
        where = f"{scope}/{case}" + (f" [{entry_name}]" if entry_name else "")
        shown = "基线 0ms" if before <= 0 else f"{before}ms"
        tail = "（从 0 变成非 0）" if before <= 0 else f"（{pct:+.1f}%）"
        print(f"  {where}: {shown} → {ms}ms{tail}", file=sys.stderr)
    print("先复测一次；仍然退化就查这一版改了什么（docs/PERF.md 的判读纪律）。", file=sys.stderr)
    raise SystemExit(1)
print(f"没有超过 {threshold:.0f}% 的退化。")
PY
