#!/usr/bin/env bash
# G-110 —— `assumption` 必须搜索**全部**局部假设（与 Lean 4 的 `findLocalDeclWithType?` 对齐）。
#
# 现场（2026-10-10 编排期实测）：命中的假设**不是上下文里最新的一条**时，
# `assumption` 报「没有找到类型与目标一致的假设」，而 goal state 明确显示
# 那条假设在作用域里、类型就是目标。
#
# 判据：`via_exact`（点名 `exact ha`）**必须** checked（环境自检）；
# `via_assumption` 若也 checked ⇒ 已修（exit 1）；只要它带
# 「没有找到类型与目标一致的假设」诊断 ⇒ 缺口仍在（exit 0）。
set -u
# 自定位（本目录其余 90 个复现件同款；⚠ 绝不写死绝对路径 —— 2026-10-11 CI 就是这样判红的：
# 写死本机路径 ⇒ 本机绿、CI 里 `cd` 失败 ⇒ exit 2 ⇒ 台账判「环境异常」）。
cd "$(dirname "$0")/../../.." || exit 2
SOKO=scripts/soko
[ -x "$SOKO" ] || { echo "找不到 $SOKO" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }

dir=$(mktemp -d) || exit 2
trap 'rm -rf "$dir"' EXIT
cat >"$dir/Assumption.sokonanoda" <<'EOF'
-- 命中假设 `ha : a` 后面还跟着 `hb : b`（它才是最新的一条）
theorem via_exact (a b : Prop) (ha : a) (hb : b) : a := by
  exact ha

theorem via_assumption (a b : Prop) (ha : a) (hb : b) : a := by
  assumption
EOF

"$SOKO" grade --json "$dir/Assumption.sokonanoda" >"$dir/events.jsonl" 2>"$dir/err.txt" || true
python3 - "$dir/events.jsonl" <<'PY'
import json, sys
path = sys.argv[1]
checked, failed = set(), {}
for line in open(path, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    try:
        ev = json.loads(line)
    except json.JSONDecodeError:
        continue
    if ev.get("type") == "decl.checked":
        checked.add(ev.get("name") or "")
    if ev.get("type") == "diagnostic":
        # ⚠ 诊断**不带名字**（实测 `name: null`）⇒ 只能按消息认领
        failed.setdefault("_all", []).append(ev.get("message", ""))
if "via_exact" not in checked:
    print("环境自检失败：`exact ha` 本该 checked（诊断：%r）" % failed.get("_all"))
    sys.exit(2)
if "via_assumption" in checked:
    print("已修：`assumption` 也能找到非最新的假设 ✓")
    sys.exit(1)
msgs = " ".join(failed.get("_all", []))
if "没有找到类型与目标一致的假设" in msgs:
    print("缺口仍在：`assumption` 只看了最新的那条假设（`hb : b`），够不到 `ha : a` ✗")
    sys.exit(0)
print("形状异常：既没 checked 也不是预期诊断：%r" % msgs)
sys.exit(2)
PY
