## M3 单元116–117：非负项的部分和与有限和的绝对值（2026-11-13 · issue #19 / PR #41）
- `unit116-nonneg-monotone`：2 题。`sumTo_nonneg` 对项数归纳证非负项的和非负；`sumTo_mono` 用 `leNat_exists_add` 把 n 写成 m + k、`sumTo_shift` 拆出尾块，把 `S_m ≤ S_n` 归到尾块非负；`tailSum_nonneg` 是同一引理在平移序列上的写法。正文 2 checked / 2 open / 0 rejected，解答 4 checked / 0 open。
- `unit117-abs-sum`：2 题。`sumTo_abs_le` 对项数归纳：全性给新和一个见证，`ratLeRep_abs_add` 把它压到「旧见证 + 新项见证」之下；`sumTo_abs_self` 说明非负序列的部分和是自己的绝对值见证。正文 1 checked / 2 open / 0 rejected，解答 3 checked / 0 open。
- `lib/SumNonneg`（4）+ `lib/SumAbs`（3）进库，`lib/Demo` 补 import 与两条自检（29 checked）。
- 判据：`check.py --json` rc=0（301 目标 · 3031 checked · 598 open · 0 rejected，M3 45/51）；`notation-lint` rc=0（312 文件，15 处豁免）；本轮改动的 8 个单元/解答与 3 个库文件文案 0 处；`style-lint --selftest` rc=0。
- 边界：只比较部分和、只给有限和的绝对值见证；不判断级数可和，不谈无限项与重排。不新增公理。下一单元：III.5 单元118。

## M3 单元114–115：级数的尾块与可和级数的线性（2026-11-13 · issue #19 / PR #41）
- `unit114-series-tail`：3 题。尾块 `TailSum f m k` 取第 m 项起的 k 项和；`sumTo_shift` 把部分和拆成前段与尾块（`series_split`），两减得 `tail_eq_sub`；`tail_close_iff` 把部分和的接近性换成尾块小；`summable_iff_tail` 借 102 的跳跃形式给出尾块判别。正文 6 checked / 3 open / 0 rejected，解答 9 checked / 0 open。
- `unit115-series-linear`：4 题。`cauchy_congr` 把 Cauchy 性沿逐点 `ratRel` 搬运；`sumTo_add` / `sumTo_neg` 把逐项运算翻成部分和运算，得 `summable_add` / `summable_neg` / `summable_sub`。正文 3 checked / 4 open / 0 rejected，解答 7 checked / 0 open。
- `lib/SeriesTail`（9）+ `lib/SeriesLinear`（7）进库，`lib/Demo` 补 import 与两条自检（27 checked）。
- 判据：`check.py --json` rc=0（295 目标 · 3010 checked · 594 open · 0 rejected，M3 43/51）；`notation-lint` rc=0（306 文件，15 处豁免）；III.5 与新增 lib 文案 0 处；`style-lint --selftest` rc=0。
- 边界：尾块判别只改写可和性；线性只在可和性上封闭，不把和类相加。不新增公理。下一单元：III.5 单元116。

## M3 单元112–113：级数的部分和与通项判别（2026-11-13 · issue #19 / PR #41）
- `unit112-series`：6 题。`Series` 把前 n 项和排成序列，`Summable` 取它的 Cauchy 性，`SeriesSum` 取极限类；零项与后继两条计算规则；可和与有和类等价（`cauchy_iff_limitClass`），和类唯一（`limitClass_unique`）；零级数可和。正文 6 checked / 6 open / 0 rejected，解答 12 checked / 0 open。
- `unit113-term-test`：3 题。`close_step_zero` 把相邻部分和的差搬到通项；`summable_terms_tend_zero` 给出必要条件；`not_summable_const` 与常数 1 的反例。正文 1 checked / 3 open / 0 rejected，解答 4 checked / 0 open。
- `lib/Series`（12）+ `lib/SeriesTerm`（4）进库，`lib/Demo` 补 import 与两条自检（25 checked）。
- 判据：`check.py --json` rc=0（289 目标 · 2965 checked · 587 open · 0 rejected，M3 41/51）；`notation-lint` rc=0（300 文件，15 处豁免）；III.5 与新增 lib 文案门禁 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0。
- 边界：`Summable` 是部分和的 Cauchy 性，不要求有理极限；只给通项判别这一必要条件。不新增公理。下一单元：III.5 单元114。
