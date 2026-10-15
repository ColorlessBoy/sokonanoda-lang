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
out="$(node scripts/soko query check --file "$ROOT/courses/set-theory/units/solutions/I.12/unit44-solution.sokonanoda" 2>&1)"
rc=$?
# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE` 的横向扫）：本件的判据
# **只有 `query check` 的退出码**，而**量具空转（零输出 / 拒跑）时它恒为 0** ⇒ 逐字落进
# 「已修」= **假绿** ✗。⇒ 先要求这次查询真的**答出了本夹具**（可归属 ✓：
# `decl_checked ≥ 1` 或 `failed` 非空 —— 两者都只可能来自真的跑了夹具 ✓）；观测不到 ⇒
# **exit 2 + 人话**，不拿空观测当结论 ✗。
if ! printf '%s' "$out" | grep -qE '"decl_checked": [1-9]|"failed": \[$'; then
  echo "G-77：观测不到 query check 的答案（既没有判过的声明，也没有失败项）⇒ 环境/形状异常（exit 2），不是判据结论 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi
if [ "$rc" -ne 0 ]; then
  echo "环境不满足：单元㊹ 解答没判绿（rc=${rc}）"
  exit 2
fi
echo "外延可用（\`lib.Extensionality\` 的 funext/propext + 单元㊹ 的等式证明判绿）"
echo "⇒ G-77 是误报，缺口不成立"
exit 1
