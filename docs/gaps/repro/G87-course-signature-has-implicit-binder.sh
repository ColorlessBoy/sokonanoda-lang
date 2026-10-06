#!/usr/bin/env bash
# G-87 复现件：课程签名里的隐式 binder 与**契约清单（棘轮）**是否逐条对得上。
#
# **退出码约定**（`docs/gaps/README.md` ✓）：0 = 缺口仍在 / 1 = 已清 / 2 = 环境或形状不对。
#
# ── 缺口原文（2026-10-04 登记 ✓）────────────────────────────────────────────
# 标题：课程签名里出现了**隐式 binder** ⇒ IA-1（隐式实参插入）的前提被打破
#       （`crates/cli/tests/notation.rs::no_course_signature_uses_an_implicit_binder` 判红）。
# 当时那是一条**前提守卫**：断言课程签名里一个隐式 binder 都不许有 —— 那是
# 「IA-1 落地时课程**零改动**也全绿」的前提。第一处是 `lib/Rel` 的 `{A B C : Type}`。
#
# ── 2026-10-06 收口（本判据换成**棘轮契约** ✓）──────────────────────────────
# 台账 `expected_lean` 给的第二条路走通了：
#   · **IA-1 已落地**（`implicit_prefix` + 求解器；G-40/G-42 已修 ✓）；
#   · 卷 I 课程库按 **IA-2 / T-N13（0.79.0）** *有意*把前导类型参数隐式化
#     （与 Lean 4 写法对齐 ✓）⇒ 前提被**有意的契约变更**取代，**不是回归** ✗；
#   · 那条闸因此改成**棘轮白名单** `INTENTIONAL`（59 条）—— 「加签名要改这里，
#     评审可见」✓；`lib/Rel` 的 19 条在 `253e9504`（v0.82.0 ✓）补齐后守卫转绿 ✓。
# ⇒ 本复现件断言的就是这份契约：**课程签名里的隐式 binder 集合 == 契约清单**
#   （两个方向都咬：多 ⇒ 未登记 ✗；少 ⇒ 清单腐烂 ✗）。
#
# ⚠ **旧版复现件两个方向都不准** ✗（2026-10-06 实测，本文件因此重写）：
#   · 多报：`grep` 不认识 `INTENTIONAL` ⇒ 把 59 条**有意**的隐式签名全报成「缺口仍在」✗；
#   · 漏报：关键字表没有 `abbrev` ⇒ `lib/Set` 的 `abbrev triple {α : Type} …` 从没被
#     它看见 ✗（实测 grep 命中 57 条、白名单 59 条）；只看**单行** ⇒ 换行写的
#     `{α : Type}` 也看不见 ✗。
#
# 判据本体在同目录的 `G87-course-signature-has-implicit-binder.py`（薄外壳约定见
# `docs/gaps/README.md`：台账只认 `.sh`，探针放隔壁 ✓）。清单**只有一处权威** ——
# 脚本直接从 `crates/cli/tests/notation.rs` 的 `INTENTIONAL` 里读，不另存一份 ✓。
#
# 用法：bash docs/gaps/repro/G87-course-signature-has-implicit-binder.sh
#       bash docs/gaps/repro/G87-course-signature-has-implicit-binder.sh --selftest
set -uo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROBE="$DIR/G87-course-signature-has-implicit-binder.py"
[ -f "$PROBE" ] || { echo "G-87: 缺探针 $PROBE" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "G-87: 需要 python3" >&2; exit 2; }

exec python3 "$PROBE" "$@"
