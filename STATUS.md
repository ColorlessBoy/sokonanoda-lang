## M3 单元112–113：级数的部分和与通项判别（2026-11-13 · issue #19 / PR #41）
- `unit112-series`：6 题。`Series` 把前 n 项和排成序列，`Summable` 取它的 Cauchy 性，`SeriesSum` 取极限类；零项与后继两条计算规则；可和与有和类等价（`cauchy_iff_limitClass`），和类唯一（`limitClass_unique`）；零级数可和。正文 6 checked / 6 open / 0 rejected，解答 12 checked / 0 open。
- `unit113-term-test`：3 题。`close_step_zero` 把相邻部分和的差搬到通项；`summable_terms_tend_zero` 给出必要条件；`not_summable_const` 与常数 1 的反例。正文 1 checked / 3 open / 0 rejected，解答 4 checked / 0 open。
- `lib/Series`（12）+ `lib/SeriesTerm`（4）进库，`lib/Demo` 补 import 与两条自检（25 checked）。
- 判据：`check.py --json` rc=0（289 目标 · 2965 checked · 587 open · 0 rejected，M3 41/51）；`notation-lint` rc=0（300 文件，15 处豁免）；III.5 与新增 lib 文案门禁 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0。
- 边界：`Summable` 是部分和的 Cauchy 性，不要求有理极限；只给通项判别这一必要条件。不新增公理。下一单元：III.5 单元114。

## M3 单元109：完备性收口（2026-11-13 · issue #19 / PR #41）
- `unit109-limit-class-complete`：8 题。Cauchy 序列有唯一的极限类。估计部分是 `eps8` / `ratSubRep_add_eps` / `bound_pair` 与 `le_sub_add_eps`，两条八分之一的界合成 `eps/2`（`close_of_half_bounds`）；`tendsTo_cauchy` 把四个见证并在同一个下标，中间项消去后交 `close`。收口：`limitClass_intro` / `cauchy_limitClass` / `cauchy_iff_limitClass` / `limitClass_exists_unique` / `demo_limitClass_const`。正文 4 checked / 8 open / 0 rejected，解答 12 checked / 0 open。
- `lib/LimitClassComplete`（12）与 `lib/SubLe`（1）进库，`lib/Demo` 补 import 与一条自检（21）。
- `lib.Bounded` 与 `lib.LimitAbs` 的同名 `le_of_sub_le` 收进 `lib/SubLe`，两个模块改为 import。
- 判据：`check.py --json` rc=0（277 目标 · 2887 checked · 567 open · 0 rejected，M3 37/51）；`notation-lint` rc=0（288 文件，15 处豁免）；文案 BLOCK 0 / WARN 0；`style-lint --selftest` rc=0。
- 边界：只断言 Cauchy 序列有极限类。不新增公理。下一单元：III.5 单元110。

## M3 单元107–108：极限类的取负与减法（2026-11-13 · issue #19 / PR #41）
- `unit107-limit-class-neg`：7 题。`(-y) - ((-x) + eps)` 与 `x - (y + eps)` 认成同一个差（`ratSubRep_neg_shift`），逐点搬进 `realLeRep`（`neg_tail_le`），两条 ε 界互换后组装成 `tendsTo_neg`；极限类取负（`limitClass_neg`）。正文 7 open / 0 rejected，解答 7 checked / 0 open。
- `unit108-limit-class-sub`：5 题。逐点差 `subSeq`；106 的加法与 107 的取负接成 `tendsTo_sub`、`cauchy_sub`、`limitClass_sub`（`Exists.elim` 动机写全）；常序列实例。正文 5 open / 0 rejected，解答 6 checked / 0 open。
- `lib/LimitClassNeg`（6）+ `lib/LimitClassSub`（5）进库，`lib/Demo` 补两条自检（20 checked）。
- 判据：`check.py` rc=0（273 目标 · 2858 checked · 0 rejected，M3 36/51）；`notation-lint` rc=0；文案 BLOCK 0 / WARN 0。
- 边界：取负与减法都带序列的 Cauchy 见证；不构造实数减法。不新增公理。下一单元：单元109（完备性收口）。
