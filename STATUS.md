## M4 单元135–136：最后一个负点与条件版介值定理（2026-11-15 · issue #20 / PR #47）
- `unit135-last-negative`：2 定义 + 13 题。f m < 0 且 m 在区间内部时，保号性给出全负的 δ 邻域；在 m 右侧找同时落在 δ 与 [a,b] 内的点：m + δ/2 没过 b 就迈半步，否则取 m 到 b 的中点。于是严格负点集没有最大元。解答 15 / 0。
- `unit136-ivt`：2 定义 + 11 题。非正点集有最大元 m 且 m < b：f m ≤ 0，严格负一支交给 135 排除，故 f m ~ 0；再用 `RespectsRel` 排除端点等价。解答 13 / 0。
- 引擎：`lib/LocalSign`（7）、`lib/LastNegative`（15），`lib/Demo` 补 import 与两条自检。判据：`check.py` 0 rejected、0 solutions_open（M4 13/27）；`notation-lint` rc=0（366 文件）；文案 0（7 文件）；`docs-lint` rc=0。

## M4 单元134：闭区间上的最值（2026-11-15 · issue #20 / PR #47）
- `unit134-extreme-values`：5 定义 + 5 展开 + 16 题。最值定理拆成两半：`IsMaxOn` 把「取到」与「压住所有值」分成两个分量；`maxOn_isSup` 说明最大值就是像的上确界；`maxOn_iff_isSup` 给出条件版——最大值存在恰好等价于像的上确界存在且被取到。连续性在 ℚ 上补不上最后一步，于是转向能证明的充分条件：`attMax_of_monotone` / `attMin_of_monotone` 把单调函数的右端点立成最大值、左端点立成最小值，`attMax_id_closed` 是 [0,1] 上的样本。反例 `not_attMax_id_open` 把 [0,1] 换成 (0,1)：任取 c，中点 c + (1-c)/2 仍在内且更大。正文 10 / 16，解答 26 / 0。
- 引擎进库：新建 `lib/MaxOn`（26，条件版最值定理、单调版本、中点两条与一正一反两个样本），`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（349 目标 · 3335 checked · 699 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 11/27）；`notation-lint` rc=0（360 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0，触及文件；lib 根目录 34 处为 `Equiv` 既有）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；连续函数的无条件最值定理要像的上确界被取到，ℚ 上不成立、ℝ 上靠完备性，留到完备性一章。不新增公理，也不碰 kernel/front。

## M4 单元133：局部保号（2026-11-15 · issue #20 / PR #47）
- `unit133-local-sign`：1 定义 + 11 题。`PositiveNear` 把「f 在 a 附近为正」立成一个存在命题；`positiveNear_of_continuousAt` 取 ε = f a，用负侧严格界把 f x 抬到 0 以上；`negativeNear_of_continuousAt` 取 ε = -f a 走正侧；`neZeroNear_of_continuousAt` 用序的全性把非零拆成正、负两支；`continuousAt_lt_of_lt` 把估计用在差 `g - f` 上。四条序的换形（`close_neg_lt`、`ratLtRep_add_right`、`ratLtRep_congr_fst`、`ratLtRep_of_sub_pos`）负责端点搬运。正文 1 / 11，解答 12 / 0。
- 引擎进库：新建 `lib/ContAlgebra`（6，来自 128 的加法群），`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（346 目标 · 3269 checked · 683 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 10/27）；`notation-lint` rc=0（357 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；乘积与商仍缺乘法对序的相容性，本单元转向介值定理所需的第一块局部信息。不新增公理，也不碰 kernel/front。
