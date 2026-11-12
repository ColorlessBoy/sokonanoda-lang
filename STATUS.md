## M3 单元79–80：极限的保号性 + 平移（2026-11-13 · issue #19 / PR #41）
- `unit79-limit-sign`：`converges_pos_of_pos`（ε 取 L/2；`close_neg_bound` → 右加 x n → 右加 -(L/2)，`ratSubRep_half` / `ratAdd_sub_self` 收口）、`close_half_neg_lt`、`neg_limit_absurd`、`converges_nonneg_of_eventually_nonneg`、`converges_ne_zero_of_pos`、`demo_pos_one`。画布 2 checked / 6 open，解答 8 checked / 0 open。新建 `lib/LimitAbs`（78 的三条 + 半减、左加再减、减界搬加界、取负反序、半数保负号）。
- `unit80-shift`：`shiftSeq k x := fun n => x (n + k)`；`eventually_shift` / `eventually_of_shift`（`leNat_exists_add` + `add_rotate`）/ `converges_shift` / `demo_shift_zero`。画布 1 checked / 6 open，解答 7 checked / 0 open。
- 判据：`check.py --json` rc=0 —— 196 目标 · 2431 checked · 395 open · 0 判负（M3 8/51）；`notation-lint` rc=0（204 文件，0 命中，15 处豁免）；III.1 画布与解答文案门禁 0 处（顺带清 73–76 的粗体小标题）；`style-lint --selftest` rc=0。
- 边界：79 只做代表元层保号性；80 只做平移。不新增公理、不碰 kernel/front。下一单元 = 尾部等价。

## M3 单元77–78：收敛序列有界 + 夹逼定理（2026-11-13 · issue #19 / PR #41）
- `unit77-converges-bounded`：乘法律的前置。`Bounded x := ∃ B, ∀ n, x n ≤ B ∧ -x n ≤ B`。尾部取 ε = 1：`close` 经 `close_sub_le` / `ratAbsRel_neg_le` 拆成 `x - L ≤ 1`、`-x + L ≤ 1`，再搬成 `x ≤ 1 + L`、`-x ≤ 1 + (-L)`；前缀对 N 归纳（`exists_prefix_bound`）；两处「较大者」都写成存在命题（全性只给析取，目标是存在时可直接消去，不需要选择）；主定理用 `leNat_total` 分两支。画布 5 checked / 9 open，解答 14 checked / 0 open。
- `unit78-limit-squeeze`：夹逼定理。`ratAbsRel_le_of_bounds`（绝对值的见证被两支界压住）、`ratNegRep_sub`（`-(x - L) ~ L - x`）、`close_neg_bound`、`close_of_two_bounds`（两侧 eps/2 ⇒ `close`）、`squeeze_step`、`converges_squeeze`（三条「终究」用 `eventually_and` 并到同一 N）、`demo_squeeze_zero`。画布 1 checked / 7 open，解答 8 checked / 0 open。
- 判据：`check.py --json` **rc=0** —— 191 目标 · 2403 checked · 383 open · **0 判负**（M3 6/51；M1+M2 72/72 不变）；`notation-lint` **rc=0**（0 命中）；文案门禁对两个新单元 **0 处**；`docs-lint` rc=0。
- 边界：有界性只是乘法律的前置之一（乘积上界还要乘法与序相容）；夹逼只做两侧极限相同的情形。不新增公理、不碰 kernel/front。下一单元 = 乘法与序相容（乘法律的第一步）。

## M3 单元75–76：极限与序 + 极限的代数（2026-11-13 · issue #19 / PR #41）
- `unit75-limit-order`：极限保持非严格不等式。`ratLeRep_of_sub_le_all`（任意 ε 内 `x − y ≤ ε` ⇒ `x ≤ y`）；`converges_le_of_eventually` 把三条「终究」并到同一 `N`；推论 `converges_nonneg` / `converges_le_const`。画布 0 checked / 4 open，解答 4 checked / 0 open。
- `unit76-limit-algebra-add`：加法群。`converges_congr_limit`、`converges_add`（`eventually_and` + `close_add`）、`converges_neg`、`converges_sub`、`demo_add_zero`。画布 1 checked / 5 open，解答 6 checked / 0 open。
- 边界：乘法律要有界性，留到后续单元；不新增公理、不碰 kernel/front。`lib/LimitOrder` 收 74 的 12 条，`lib/RealNeg` 收 59 的 5 条。
