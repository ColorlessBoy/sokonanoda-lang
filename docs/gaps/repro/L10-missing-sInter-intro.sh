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
    (h : forall (A : Set alpha), Set.mem (Set alpha) A F -> Set.mem alpha x A) :
    Set.mem alpha x (Set.sInter alpha F) :=
  Set.mem_sInter_intro alpha F x h
SOKO

node scripts/soko query check --file "$ROOT/$PROBE" >/dev/null 2>&1
rc=$?
rm -f "$PROBE"

if [ "$rc" -eq 0 ]; then
  echo "L-10 已修：探针判绿（Set.mem_sInter_intro 在且可用）"
  exit 1
fi
echo "L-10 仍在：探针判红（引理缺失或不可用）"
exit 0
