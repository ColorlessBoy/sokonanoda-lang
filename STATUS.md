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

## M3 单元114–115：级数的尾块与可和级数的线性（2026-11-13 · issue #19 / PR #41）
- `unit114-series-tail`：3 题。尾块 `TailSum f m k` 取第 m 项起的 k 项和；`sumTo_shift` 把部分和拆成前段与尾块（`series_split`），两减得 `tail_eq_sub`；`tail_close_iff` 把部分和的接近性换成尾块小；`summable_iff_tail` 借 102 的跳跃形式给出尾块判别。正文 6 checked / 3 open / 0 rejected，解答 9 checked / 0 open。
- `unit115-series-linear`：4 题。`cauchy_congr` 把 Cauchy 性沿逐点 `ratRel` 搬运；`sumTo_add` / `sumTo_neg` 把逐项运算翻成部分和运算，得 `summable_add` / `summable_neg` / `summable_sub`。正文 3 checked / 4 open / 0 rejected，解答 7 checked / 0 open。
- `lib/SeriesTail`（9）+ `lib/SeriesLinear`（7）进库，`lib/Demo` 补 import 与两条自检（27 checked）。
- 判据：`check.py --json` rc=0（295 目标 · 3010 checked · 594 open · 0 rejected，M3 43/51）；`notation-lint` rc=0（306 文件，15 处豁免）；III.5 与新增 lib 文案 0 处；`style-lint --selftest` rc=0。
- 边界：尾块判别只改写可和性；线性只在可和性上封闭，不把和类相加。不新增公理。下一单元：III.5 单元116。
