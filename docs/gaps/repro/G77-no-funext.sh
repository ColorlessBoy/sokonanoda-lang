#!/usr/bin/env bash
# G-77 **改正件**（2026-10-01 第 570 轮）：原报告「内核没有函数外延」是**误报** ✗
#
# 退出码约定（同其他 repro 脚本，见 docs/gaps/README.md）：
#   0 = 缺口仍在（外延不可用）
#   1 = 已"修"（外延可用 ⇒ 这条缺口**不成立**）
#   2 = 环境不满足（判卷没跑绿）
#
# 判据：**单元㊹ 的解答**（它就是"用外延把等式证出来"的那一片）能判绿 ⇒ 外延可用 ✓
# 误报原因：当时的探针只 import 了 `lib.Rel`/`lib.Set`/`lib.Order`，
#           **没 import `lib.Extensionality`** ⇒ `funext` 当然 `unknown identifier` ✗
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
node scripts/soko query check --file "$ROOT/courses/set-theory/units/solutions/unit44-solution.sokonanoda" >/dev/null 2>&1
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "环境不满足：单元㊹ 解答没判绿（rc=$rc）"
  exit 2
fi
echo "外延可用（`lib.Extensionality` 的 funext/propext + 单元㊹ 的等式证明判绿）"
echo "⇒ G-77 是误报，缺口不成立"
exit 1
