## M4 单元133：局部保号（2026-11-15 · issue #20 / PR #47）
- `unit133-local-sign`：1 定义 + 11 题。`PositiveNear` 把「f 在 a 附近为正」立成一个存在命题；`positiveNear_of_continuousAt` 取 ε = f a，用负侧严格界把 f x 抬到 0 以上；`negativeNear_of_continuousAt` 取 ε = -f a 走正侧；`neZeroNear_of_continuousAt` 用序的全性把非零拆成正、负两支；`continuousAt_lt_of_lt` 把估计用在差 `g - f` 上。四条序的换形（`close_neg_lt`、`ratLtRep_add_right`、`ratLtRep_congr_fst`、`ratLtRep_of_sub_pos`）负责端点搬运。正文 1 / 11，解答 12 / 0。
- 引擎进库：新建 `lib/ContAlgebra`（6，来自 128 的加法群），`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（346 目标 · 3269 checked · 683 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 10/27）；`notation-lint` rc=0（357 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；乘积与商仍缺乘法对序的相容性，本单元转向介值定理所需的第一块局部信息。不新增公理，也不碰 kernel/front。

## M4 单元132：局部有界（2026-11-15 · issue #20 / PR #47）
- `unit132-local-boundedness`：2 定义 + 7 题。乘积的估计需要一个不随 x 变化的界，而极限只保证「离 L 近」。`LocallyBoundedAt` 管整个 δ 邻域，`LocallyBoundedPunct` 管去心邻域；连续取 ε = 1 得整邻域的盒子，有限极限因定义排掉 a 点只给去心版本；两条展开律、整邻域⇒去心、常函数与恒等函数两个样本。正文 2 / 7，解答 9 / 0。
- 引擎进库：新建 `lib/LocalBounded`（9，来自 132），`lib/RatClose` 增收绝对值非负与两个非负数之和非负三条，`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（343 目标 · 3246 checked · 0 rejected · 0 solutions_open · 0 lib_open，M4 9/27）；`notation-lint` rc=0（354 文件，15 处豁免）；文案 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：盒子依赖基点，不是整片定义域的统一界；乘积还要乘法对序的相容性，留到下一单元。不新增公理，也不碰 kernel/front。

## M4 单元130–131：一致连续与序列的像（2026-11-14 · issue #20 / PR #47）
- `unit130-uniform-continuity`：1 定义 + 7 题。`UniformlyContinuous` 把「对每个 a 存在 δ(a)」对调成「存在 δ 对每个 x、y」；展开律、常函数、恒等函数、一致连续⇒逐点连续、复合、平移各一题；引擎进库 `lib/Uniform`（8）。正文 1 / 7，解答 8 / 0。
- `unit131-uniform-continuity-sequences`：4 题。Cauchy 序列的像仍 Cauchy（ε 换 δ），收敛序列的像收敛到 `f L`；引擎进库 `lib/UniformSeq`（2），`lib/Demo` 补三条自检。正文 0 / 4，解答 4 / 0。
- 判据：`check.py` rc=0（3219 checked · 0 rejected · 0 open，M4 8/27）；`notation-lint` rc=0；文案 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：有理代表元层；非一致连续的反例与反方向留后。
