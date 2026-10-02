#!/usr/bin/env bash
# G-53 复现：**Infoview 的记法符号（`{a}` / `∈`）点不动** —— 根因是 wire 的
# semantic runs **只有 `{text, kind}`、没有源位置** ⇒ webview 拿不到"点的是源码
# 哪一处"，就没法像声明名那样发 `definition`（E27 只接了声明名）。
#
# 约定（`docs/gaps/README.md`）：exit 0 = **缺口仍在**（runs 仍无位置）；
# exit 1 = **行为已变**（runs 带上位置了 ⇒ 可以接符号点击，本条可关）。
# exit 2 = 环境/形状不对（永远判红，别当成"已修"）。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2

canvas="courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
[ -f "$canvas" ] || { echo "env: 找不到夹具 $canvas"; exit 2; }

# 用仓库启动器（harness 中立；它自己按版本钉解析 CLI）。
out=$(scripts/soko query goals --file "$canvas" 2>/dev/null) || {
  echo "env: \`scripts/soko query goals\` 跑不起来（exit≠0）"; exit 2;
}

printf '%s' "$out" | python3 -c '
import json, sys
raw = sys.stdin.read()
try:
    data = json.loads(raw)
except Exception as exc:            # noqa: BLE001 - 形状不对 = 环境问题
    print(f"env: query goals 的输出不是 JSON：{exc}")
    raise SystemExit(2)
runs = []
for decl in data.get("data", []):
    for run in decl.get("ty_runs", []) + decl.get("goal_runs", []):
        runs.append(run)
    for group in decl.get("goals_runs", []):
        runs.extend(group)
if not runs:
    print("env: 夹具里一条 run 都没有 ⇒ 这条复现件会空转")
    raise SystemExit(2)
# "位置"的任一形态：start/end（字节偏移）、range（LSP）、span。
positional = [r for r in runs if any(k in r for k in ("start", "end", "range", "span"))]
if positional:
    print(f"行为已变：{len(positional)} 条 run 带了位置 ⇒ 记法符号可以接点击了（关掉 G-53）")
    raise SystemExit(1)
keys = sorted({k for r in runs for k in r})
print(f"缺口仍在：{len(runs)} 条 run 的字段只有 {keys} ⇒ webview 无从知道"
      "「点的是源码哪一处」⇒ 记法符号（{a}/∈）接不了 `definition`（E27 只接了声明名）")
raise SystemExit(0)
'
status=$?
exit "$status"
