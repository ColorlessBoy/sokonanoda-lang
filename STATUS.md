## M3 单元83：收敛 ⇒ Cauchy（2026-11-13 · issue #19 / PR #41）
- `unit83-converges-cauchy`：`close_pair_of_close_half`（两条 ε/2 接近性用 `close_symm` + `close_trans` 合成）；主定理 `converges_cauchy`（ε 折半，阈值同 N）；`converges_realRel_const`（三个分量：83.2、库里的 `cauchy_const`、收敛性本身）；`converges_real_eq_const`（`Real.sound`）；`demo_zero_cauchy`。画布 1 checked / 4 open，解答 5 checked / 0 open。
- 判据：`check.py --json` rc=0（203 目标 · 2466 checked · 413 open · 0 判负，M3 11/51）；`notation-lint` rc=0（211 文件，15 处豁免）；III.1 文案门禁 0 处；`style-lint --selftest` rc=0。
- 边界：只证收敛 ⇒ Cauchy；反方向在 ℚ 上不成立，要等 ℝ 的完备性。不新增公理、不碰 kernel/front。下一单元 = III.2 扩展实数与上确界。

## M3 单元81–82：尾部等价 + 单调序列的极限是上确界（2026-11-13 · issue #19 / PR #41）
- `unit81-tail-eq`：`TailEq`、`close_wd_left`、`tailEq_symm` / `tailEq_trans`、`converges_congr_tail` / `converges_congr_tail_iff`、`tailEq_of_pointwise`。画布 2 checked / 7 open，解答 9 checked / 0 open。
- `unit82-mono-lub`：`Monotone` / `StrictMono`；`monotone_le`、`monotone_limit_upper` / `monotone_limit_lub`、`strict_mono_limit_gt`。画布 3 checked / 7 open，解答 11 checked / 0 open。新建 `lib/LimitLe`。
- 判据：`check.py --json` rc=0（201 目标 · 2460 checked · 409 open · 0 判负，10/51）；`notation-lint` / III.1 文案门禁 / `style-lint --selftest` / `docs-lint` 均 rc=0。
- 边界：82 只证收敛的单调序列极限是上确界；81 允许项只是代表元等价。不新增公理、不碰 kernel/front。下一单元 = 收敛 ⇒ Cauchy。

## M3 单元79–80：极限的保号性 + 平移（2026-11-13 · issue #19 / PR #41）
- `unit79-limit-sign`：`converges_pos_of_pos`（ε 取 L/2；`close_neg_bound` → 右加 x n → 右加 -(L/2)，`ratSubRep_half` / `ratAdd_sub_self` 收口）、`close_half_neg_lt`、`neg_limit_absurd`、`converges_nonneg_of_eventually_nonneg`、`converges_ne_zero_of_pos`、`demo_pos_one`。画布 2 checked / 6 open，解答 8 checked / 0 open。新建 `lib/LimitAbs`（78 的三条 + 半减、左加再减、减界搬加界、取负反序、半数保负号）。
- `unit80-shift`：`shiftSeq k x := fun n => x (n + k)`；`eventually_shift` / `eventually_of_shift`（`leNat_exists_add` + `add_rotate`）/ `converges_shift` / `demo_shift_zero`。画布 1 checked / 6 open，解答 7 checked / 0 open。
- 判据：`check.py --json` rc=0 —— 196 目标 · 2431 checked · 395 open · 0 判负（M3 8/51）；`notation-lint` rc=0（204 文件，0 命中，15 处豁免）；III.1 画布与解答文案门禁 0 处（顺带清 73–76 的粗体小标题）；`style-lint --selftest` rc=0。
- 边界：79 只做代表元层保号性；80 只做平移。不新增公理、不碰 kernel/front。下一单元 = 尾部等价。
