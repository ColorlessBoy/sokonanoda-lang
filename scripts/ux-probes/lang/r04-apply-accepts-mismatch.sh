#!/usr/bin/env bash
# r04-apply-accepts-mismatch.sh —— `apply` 接受结论对不上的 Eq.subst，静默造出不可能的子目标
# 现象: 目标 `A x`，写 `apply Eq.subst α (fun y => A y) x a`（x a 写反）。
#       `Eq.subst α P x a : x = a → P x → P a`，结论 `P a` 与目标 `A x` **对不上**，
#       `apply` 本该失败（对照：`apply hpr` 在结论不匹配时正确报
#       「`apply` 的目标不匹配：`hpr` 的结果是 `r`，无法对齐当前目标 `q`」）。
#       实测它**成功了**，并留下子目标 `x = a`（对任意 x a 不可证）+ 原目标。
#       ⇒ 错误行不报错，错误在 2 行后以「`exact` 类型不匹配」冒出来（画布 p03 实况）。
# 期望: apply 必须校验结论；对不上就在**写错的那一行**报错。
# 判据: exit 0 = 问题仍在（apply 行无诊断且留下 x = a 子目标）/ exit 1 = 已修
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
F="$REPO/.cache/ux/lang/repro/.work/r04.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem apply_should_fail (α : Type) (A : Set α) (a x : α) (ha : A a) : A x := by
  apply Eq.subst α (fun (y : α) => A y) x a
  sorry
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1); rc=$?
st=$(timeout 300 "$REPO/scripts/soko" query state --file "$F" --line 7 --col 3 2>&1)
echo "grade rc=$rc"; echo "state entering line 7: $(grep -o '"goal":"[^"]*"' <<<"$st" | tr '\n' ' ')"
if [ "$rc" -eq 0 ] && grep -q 'x = a' <<<"$st"; then
  echo "STILL REPRODUCES: apply 未报错，且留下不可证的子目标 x = a"; exit 0
fi
echo "FIXED: apply 已拒绝结论不匹配的项"; echo "$out" | head -c 400; exit 1
