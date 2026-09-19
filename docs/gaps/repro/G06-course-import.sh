#!/usr/bin/env bash
# G-06 自断言复现：**`sokonanoda course <manifest>` 只按单文件编译，不认 `import`**
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（与台账 today 一致）· 1 = 行为变了（修好后的期望态）
#   · 2 = 环境不满足（含"既不是修前形状也不是修后形状"）
#
# 夹具：`docs/gaps/repro/G06-course-import/`（proj/ 是一个真项目：sokonanoda.toml +
# Lib2.sokonanoda + U4.sokonanoda（import Lib2）；course.json 只列这个单元）。
#
# 两种形状（本脚本**两种都断言**，不删任何一条）：
#   修前（台账 today）：course.unit = checked:0 / failed:1（同一文件走项目模式是 1/0）
#   修后（WO-007 落地）：course.unit = checked:1 / failed:0；summary 同判
#
# 根因（读代码定位，非推测）：crates/cli/src/course.rs 的 count_unit 曾把源码文本交给
# `compile_cached(&file, src, &CompileOptions::default())`，而 `CompileOptions`
# （crates/front/src/compile/prelude.rs:31）只有 `prelude` 一个字段——没有路径/模块根，
# 所以项目闭包根本没加载，`mythm` 就成了 unknown identifier。
# WO-007 之后：有 `import` 的单元走 `project_cache::plan/load` + `compile_plan`
# （与 `grade`/`query check` 同一份闭包、同一个模块根、同一个摘要键），
# 计数只取入口模块，`failed` = 闭包内所有模块 errors 之和。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
FIX="docs/gaps/repro/G06-course-import"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
[ -f "$FIX/course.json" ] || { echo "夹具缺失：$FIX" >&2; exit 2; }

echo "== ① 项目模式判卷（正确路径；同一文件）=="
Q="$( node "$SOKO" query check --file "$PWD/$FIX/proj/U4.sokonanoda" --compact 2>&1 )"
printf '%s\n' "$Q" | head -1
ok_project=0
printf '%s' "$Q" | grep -q '"decl_checked":1' && printf '%s' "$Q" | grep -q '"failed":\[\]' || ok_project=1
echo "   → 1 checked / 0 failed（预期）：$([ "$ok_project" = 0 ] && echo yes || echo NO)"
echo

echo "== ② 课程聚合（G-06 的落点）=="
C="$( node "$SOKO" course "$FIX/course.json" --json 2>&1 )"
printf '%s\n' "$C"
UNIT="$(printf '%s\n' "$C" | grep '"type":"course.unit"' | head -1)"
SUM="$(printf '%s\n' "$C" | grep '"type":"course.summary"' | head -1)"

# 修后形状：入口 checked:1 / failed:0（且没有 error 字段）＋ summary failed:0。
fixed=0
printf '%s' "$UNIT" | grep -q '"checked":1,"failed":0' || fixed=1
printf '%s' "$UNIT" | grep -q '"error"' && fixed=1
printf '%s' "$SUM" | grep -q '"checked":1,"failed":0' || fixed=1
echo "   → course 认 import（checked:1 / failed:0）：$([ "$fixed" = 0 ] && echo yes || echo NO)"

# 修前形状：checked:0 / failed:1（台账 today）。
broken=0
printf '%s' "$UNIT" | grep -q '"checked":0,"failed":1' || broken=1
printf '%s' "$SUM" | grep -q '"checked":0,"failed":1' || broken=1
echo "   → 仍是修前形状（checked:0 / failed:1）：$([ "$broken" = 0 ] && echo yes || echo NO)"
echo

if [ "$ok_project" != 0 ]; then
  echo "结论：项目模式都不绿 ⇒ 环境/回归问题，先查判卷。" >&2
  exit 2
fi
if [ "$fixed" = 0 ]; then
  echo "结论：G-06 已修（course 认 import）⇒ 更新 docs/gaps/ledger.jsonl 的 G-06（关账）。"
  exit 1
fi
if [ "$broken" = 0 ]; then
  echo "结论：G-06 仍在（项目模式绿、course 聚合红）——与台账一致。"
  exit 0
fi
echo "结论：既不是修前形状也不是修后形状 ⇒ 先查回归。" >&2
exit 2
