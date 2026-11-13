## M3 单元122–123：尾块判别的反方向与交换前两项（2026-11-13 · issue #19 / PR #41）
- `unit122-tail-reverse`：3 题。`tailSum_shift` 用 `add_assoc` 对齐尾块下标；`summable_of_tail` 用 `summable_iff_tail`、`leNat_exists_add` 与 `leNat_add_cancel_right` 把尾块估计读回原级数；`summable_tail_iff` 与 121 的 `summable_tail` 合成充要条件。正文 1 checked / 3 open，解答 4 / 0。
- `unit123-series-swap`：3 题。`swapFirst` 交换前两项；`swapFirst_eq_of_two_le` 证明下标 ≥ 2 不再改项；`tailSum_swapFirst` 把尾块相等抬起来；`summable_swapFirst_iff` 用 `summable_iff_tail` 与 `leNat_max` 收口。正文 2 checked / 3 open，解答 10 / 0。
- 判据：`check.py --json` rc=0（319/3118/614/0，M3 51/51）；`notation-lint` rc=0（330 文件）；文案 0 处；`style-lint --selftest` rc=0。边界：122 只走反方向；123 只交换前两项，一般置换与和类不变不在本单元。不新增公理。

## M3 单元120–121：望远镜级数与级数的尾块（2026-11-13 · issue #19 / PR #41）
- `unit120-telescope-series`：3 题。`series_telescope` 把 111 的有限望远镜搬到部分和：相邻差的和只剩 `b n - b 0`；`summable_telescope` 用 `cauchy_congr` 把可和性从「`b` 加一个常数」的平移搬到差分级数；`cauchy_of_telescope_summable` 走反方向，把 `b 0` 加回 `b n - b 0`。正文 1 checked / 3 open，解答 4 / 0。
- `unit121-series-tail`：2 题。`cauchy_shift_left` 用 `Eq.subst` 把 `cauchy_shift` 的 `x (n + k)` 改成尾块要的 `x (k + n)`；`series_tail_eq` 认出尾块级数的部分和是 `TailSum`；`summable_tail` 由 `tail_eq_sub` 与 `cauchy_add_const` 得到「可和 ⇒ 尾块可和」。正文 2 checked / 2 open，解答 4 / 0。
- `lib/SeriesTelescope`（4）+ `lib/SeriesTailSum`（4）进库；`lib/Demo` 补两条自检（33）。
- 判据：`check.py --json` rc=0（313 目标 · 3083 checked · 608 open · 0 rejected，M3 49/51）；`notation-lint` rc=0（324 文件，15 处豁免）；本轮改动文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0。边界：望远镜只处理相邻差；尾块只证「可和 ⇒ 尾块可和」。不新增公理。下一单元 122。

## M3 单元118–119：比较判别法与绝对收敛（2026-11-13 · issue #19 / PR #41）
- `unit118-comparison`：3 题。`tailSum_le` 把逐项比较搬到尾块；`tail_close_of_le` 说明被压住的尾块也一致地小（非负见证取自身，`ratAbsRel_le` 与 `ratLtRep_of_le_lt` 收口）；`summable_of_le` 用 114 的尾块判别合成比较判别法，`not_summable_of_le` 是逆否形式。正文 2 checked / 3 open，解答 5 / 0。
- `unit119-abs-convergence`：2 题。`ratLtRep_of_le_close` 把「y 在 ε 内靠近零」转成「不超过 y 的 d 也小于 ε」；`summable_of_abs` 用 117 的 `sumTo_abs_le` 把 f 的尾块压在 g 的尾块下；`AbsSummable` 把绝对收敛写成存在见证序列，`abs_summable_summable` 消去见证。正文 3 checked / 2 open，解答 5 / 0。
- `lib/SeriesCompare`（5）+ `lib/SeriesAbs`（5）进库；`lib/Demo` 补两条自检（31）。
- 判据：`check.py --json` rc=0（307 目标 · 3060 checked · 603 open · 0 rejected，M3 47/51）；`notation-lint` rc=0（318 文件，15 处豁免）；本轮改动文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0。边界：比较要求逐项成立且 f 非负；不证反方向（收敛但绝对值不可和），那需要交错级数；不新增公理。下一单元 120。
