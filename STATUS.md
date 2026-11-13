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

## M3 单元116–117：非负项的部分和与有限和的绝对值（2026-11-13 · issue #19 / PR #41）
- `unit116-nonneg-monotone`：2 题。`sumTo_nonneg` 归纳证非负项的和非负；`sumTo_mono` 用 `leNat_exists_add` 与 `sumTo_shift` 拆出尾块，把 `S_m ≤ S_n` 归到尾块非负；`tailSum_nonneg` 是平移版。正文 2 checked / 2 open，解答 4 / 0。
- `unit117-abs-sum`：2 题。`sumTo_abs_le` 归纳：全性给新和一个见证，`ratLeRep_abs_add` 压到「旧见证 + 新项见证」之下；`sumTo_abs_self`：非负序列的部分和是自己的见证。正文 1 checked / 2 open，解答 3 / 0。
- `lib/SumNonneg`（4）+ `lib/SumAbs`（3）进库；`lib/Demo` 补两条自检（29）。
- 判据：`check.py --json` rc=0（301 目标 · 3031 checked · 598 open · 0 rejected，M3 45/51）；`notation-lint` rc=0（312 文件，15 处豁免）；本轮改动文案 0 处；`style-lint --selftest` rc=0。边界：只比较部分和、只给有限和的见证，不判断级数可和；不新增公理。下一单元 118。

