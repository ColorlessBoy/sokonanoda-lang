## M4 单元134：闭区间上的最值（2026-11-15 · issue #20 / PR #47）
- `unit134-extreme-values`：5 定义 + 5 展开 + 16 题。最值定理拆成两半：`IsMaxOn` 把「取到」与「压住所有值」分成两个分量；`maxOn_isSup` 说明最大值就是像的上确界；`maxOn_iff_isSup` 给出条件版——最大值存在恰好等价于像的上确界存在且被取到。连续性在 ℚ 上补不上最后一步，于是转向能证明的充分条件：`attMax_of_monotone` / `attMin_of_monotone` 把单调函数的右端点立成最大值、左端点立成最小值，`attMax_id_closed` 是 [0,1] 上的样本。反例 `not_attMax_id_open` 把 [0,1] 换成 (0,1)：任取 c，中点 c + (1-c)/2 仍在内且更大。正文 10 / 16，解答 26 / 0。
- 引擎进库：新建 `lib/MaxOn`（26，条件版最值定理、单调版本、中点两条与一正一反两个样本），`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（349 目标 · 3335 checked · 699 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 11/27）；`notation-lint` rc=0（360 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0，触及文件；lib 根目录 34 处为 `Equiv` 既有）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；连续函数的无条件最值定理要像的上确界被取到，ℚ 上不成立、ℝ 上靠完备性，留到完备性一章。不新增公理，也不碰 kernel/front。

## M4 单元133：局部保号（2026-11-15 · issue #20 / PR #47）
- `unit133-local-sign`：1 定义 + 11 题。`PositiveNear` 把「f 在 a 附近为正」立成一个存在命题；`positiveNear_of_continuousAt` 取 ε = f a，用负侧严格界把 f x 抬到 0 以上；`negativeNear_of_continuousAt` 取 ε = -f a 走正侧；`neZeroNear_of_continuousAt` 用序的全性把非零拆成正、负两支；`continuousAt_lt_of_lt` 把估计用在差 `g - f` 上。四条序的换形（`close_neg_lt`、`ratLtRep_add_right`、`ratLtRep_congr_fst`、`ratLtRep_of_sub_pos`）负责端点搬运。正文 1 / 11，解答 12 / 0。
- 引擎进库：新建 `lib/ContAlgebra`（6，来自 128 的加法群），`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（346 目标 · 3269 checked · 683 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 10/27）；`notation-lint` rc=0（357 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；乘积与商仍缺乘法对序的相容性，本单元转向介值定理所需的第一块局部信息。不新增公理，也不碰 kernel/front。

## M4 单元132：局部有界（2026-11-15 · issue #20 / PR #47）
- `unit132-local-boundedness`：2 定义 + 7 题。乘积的估计需要一个不随 x 变化的界，而极限只保证「离 L 近」。`LocallyBoundedAt` 管整个 δ 邻域，`LocallyBoundedPunct` 管去心邻域；连续取 ε = 1 得整邻域的盒子，有限极限因定义排掉 a 点只给去心版本；两条展开律、整邻域⇒去心、常函数与恒等函数两个样本。正文 2 / 7，解答 9 / 0。
- 引擎进库：新建 `lib/LocalBounded`（9，来自 132），`lib/RatClose` 增收绝对值非负与两个非负数之和非负三条，`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（343 目标 · 3246 checked · 0 rejected · 0 solutions_open · 0 lib_open，M4 9/27）；`notation-lint` rc=0（354 文件，15 处豁免）；文案 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：盒子依赖基点，不是整片定义域的统一界；乘积还要乘法对序的相容性，留到下一单元。不新增公理，也不碰 kernel/front。

