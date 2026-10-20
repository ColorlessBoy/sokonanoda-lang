#!/usr/bin/env bash
# L-10 复现/守卫件：`lib/SUnion` 的取用子**不对称** —— 并侧有 `intro`+`elim`，
# 交侧只有 `elim`（缺 `Set.mem_sInter_intro`）⇒ 使用者各自绕路。
#
# 历史现场（2026-10-01 修前，第 580 轮）：单元㊽ 想「引入交的成员」时无引理可用，
# 只能走 `Set.mem_sInter_iff` + `Iff.mpr`，还要先把绑定形态落地成 λ。
#
# 判据（**跑一次真判卷**）：写一个探针单元，只用 `Set.mem_sInter_intro` 证
# 「每个成员都含 x ⇒ x ∈ ⋂F」；判绿 ⇒ 引理在且**真的能用**。
# ⚠ 探针里用**点名写法** —— 记法是**文件内作用域**的（手册明说的边界）。
#
# 退出码约定（见 docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境不满足
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
PROBE="courses/set-theory/units/zzprobe-l10.sokonanoda"

cat > "$PROBE" <<'SOKO'
import lib.Set
import lib.SUnion

theorem l10_probe (alpha : Type) (F : Set (Set alpha)) (x : alpha)
    (h : forall (A : Set alpha), @Set.mem (Set alpha) A F -> @Set.mem alpha x A) :
    @Set.mem alpha x (@Set.sInter alpha F) :=
  @Set.mem_sInter_intro alpha F x h
SOKO

out="$(node scripts/soko query check --file "$ROOT/$PROBE" 2>&1)"
rc=$?
# ⚠ 探针文件先删（提到闸门**之前** ✓）—— 否则闸门 `exit 2` 时会把探针残留在 `courses/`里 ✗
rm -f "$PROBE"
# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE` 的横向扫）：本件的判据
# **只有 `query check` 的退出码**，而**量具空转（零输出 / 拒跑）时它恒为 0** ⇒ 逐字落进
# 「已修」= **假绿** ✗。⇒ 先要求这次查询真的**答出了本夹具**（可归属 ✓：
# `decl_checked ≥ 1` 或 `failed` 非空 —— 两者都只可能来自真的跑了夹具 ✓）；观测不到 ⇒
# **exit 2 + 人话**，不拿空观测当结论 ✗。
if ! printf '%s' "$out" | grep -qE '"decl_checked": [1-9]|"failed": \[$'; then
  echo "L-10：观测不到 query check 的答案（既没有判过的声明，也没有失败项）⇒ 环境/形状异常（exit 2），不是判据结论 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

if [ "$rc" -eq 0 ]; then
  echo "L-10 已修：探针判绿（Set.mem_sInter_intro 在且可用）"
  exit 1
fi
echo "L-10 仍在：探针判红（引理缺失或不可用）"
exit 0
