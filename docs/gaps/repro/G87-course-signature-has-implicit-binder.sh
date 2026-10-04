#!/usr/bin/env bash
# G-87 复现件：课程签名里出现了**隐式 binder** ⇒ IA-1（隐式实参插入）的前提被打破 ✗
#
# **退出码约定**（`scripts/gap.py` 仓内约定 ✓）：0 = **缺口仍在** / 1 = **已清** / 2 = 环境不对。
#
# 背景 ✓：`crates/cli/tests/notation.rs::no_course_signature_uses_an_implicit_binder`
# 断言**课程签名里不许有隐式 binder** —— 那是 **IA-1** 的**前提**：IA-1（隐式实参插入）
# 一落地，隐式实参会在这些声明上生效 ⇒ 课程就不再是「零改动也全绿」✗。
# 现测（2026-10-04）它**红** ✗，第一处是 `lib/Rel.sokonanoda` 的 `{A B C : Type}` ✓。
#
# 用法：bash docs/gaps/repro/G87-course-signature-has-implicit-binder.sh
set -u

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
COURSE="$ROOT/courses/set-theory"
[ -d "$COURSE" ] || { echo "环境不对：课程仓缺失（可分开检出）" >&2; exit 2; }

# 声明行里的花括号 binder：`theorem/def/axiom … {A B : Type} …`（**签名位置** ✓）。
# ⚠ 只看**声明关键字开头**的行 ✓ —— 证明体里的 `{…}`（集合字面量 ✓）不算 ✗。
hits="$(grep -rnE '^(theorem|def|axiom|example|inductive)[^:]*\{[^}]*:[^}]*\}' \
  "$COURSE/lib" "$COURSE/units" 2>/dev/null | head -20)"

if [ -n "$hits" ]; then
  echo "缺口仍在 ✗：课程签名里有隐式 binder（IA-1 的前提被打破）——"
  echo "$hits"
  echo "⇒ 要么课程侧把 {…} 改回 (…) ✓，要么 IA-1 落地时同时给出「课程零改动仍全绿」的判据 ✓。"
  exit 0
fi

echo "已清 ✓：课程签名里没有隐式 binder（IA-1 的前提恢复 ✓）"
exit 1
