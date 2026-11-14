## M4 单元124–125：函数极限的 ε-δ 定义与连续（2026-11-14 · issue #20）
- `unit124-function-limit`：5 题。`TendsToAt` 把函数在 a 处的极限写成 ε-δ：δ 邻域内不等于 a 的 x 满足 `close (f x) L ε`；`tendsToAt_iff` 展开定义；常函数 `tendsToAt_const` 与恒等函数 `tendsToAt_id` 是两个可算样本；`tendsToAt_congr_off` 说明在 a 之外相等的两个函数有同一极限，`close_wd_left` 沿相等搬运接近性。正文 1 checked / 5 open，解答 6 / 0。
- `unit125-continuity`：9 题。`ContinuousAt` 把极限定义里的 L 换成 f a 并去掉「x 不是 a」，`Continuous` 是逐点连续；`continuousAt_iff` 展开；常函数与恒等函数连续；`tendsToAt_of_continuousAt` 从连续得到同点的函数极限；`continuousAt_comp` / `continuous_comp` 用两级半径证明连续对复合封闭。正文 2 checked / 9 open，解答 11 / 0。
- 判据：`check.py --json` rc=0（325 目标 · 3148 checked · 628 open · 0 rejected · 0 solutions_open，M4 2/27）；`notation-lint` rc=0（336 文件，15 处豁免）；本轮改动文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0。边界：函数取 `ratRep → ratRep`，停在有理代表元层；「极限存在 ⇒ 连续」要对 `ratRel` 二选一，不在本课程逻辑里，留作边界。不新增公理。

## M3 单元122–123：尾块判别的反方向与交换前两项（2026-11-13 · issue #19 / PR #41）
- `unit122-tail-reverse`：3 题。`tailSum_shift` 用 `add_assoc` 对齐尾块下标；`summable_of_tail` 用 `summable_iff_tail`、`leNat_exists_add` 与 `leNat_add_cancel_right` 把尾块估计读回原级数；`summable_tail_iff` 与 121 的 `summable_tail` 合成充要条件。正文 1 checked / 3 open，解答 4 / 0。
- `unit123-series-swap`：3 题。`swapFirst` 交换前两项；`swapFirst_eq_of_two_le` 证明下标 ≥ 2 不再改项；`tailSum_swapFirst` 把尾块相等抬起来；`summable_swapFirst_iff` 用 `summable_iff_tail` 与 `leNat_max` 收口。正文 2 checked / 3 open，解答 10 / 0。
- 判据：`check.py --json` rc=0（319/3118/614/0，M3 51/51）；`notation-lint` rc=0（330 文件）；文案 0 处；`style-lint --selftest` rc=0。边界：122 只走反方向；123 只交换前两项，一般置换与和类不变不在本单元。不新增公理。

## M3 单元120–121：望远镜级数与级数的尾块（2026-11-13 · issue #19 / PR #41）
- `unit120-telescope-series`：3 题。`series_telescope` 把 111 的有限望远镜搬到部分和：相邻差的和只剩 `b n - b 0`；`summable_telescope` 用 `cauchy_congr` 把可和性从「`b` 加一个常数」的平移搬到差分级数；`cauchy_of_telescope_summable` 走反方向，把 `b 0` 加回 `b n - b 0`。正文 1 checked / 3 open，解答 4 / 0。
- `unit121-series-tail`：2 题。`cauchy_shift_left` 用 `Eq.subst` 把 `cauchy_shift` 的 `x (n + k)` 改成尾块要的 `x (k + n)`；`series_tail_eq` 认出尾块级数的部分和是 `TailSum`；`summable_tail` 由 `tail_eq_sub` 与 `cauchy_add_const` 得到「可和 ⇒ 尾块可和」。正文 2 checked / 2 open，解答 4 / 0。
- `lib/SeriesTelescope`（4）+ `lib/SeriesTailSum`（4）进库；`lib/Demo` 补两条自检（33）。
- 判据：`check.py --json` rc=0（313 目标 · 3083 checked · 608 open · 0 rejected，M3 49/51）；`notation-lint` rc=0（324 文件，15 处豁免）；本轮改动文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0。边界：望远镜只处理相邻差；尾块只证「可和 ⇒ 尾块可和」。不新增公理。下一单元 122。
