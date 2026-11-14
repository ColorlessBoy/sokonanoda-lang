## M4 单元137–138：无穷远处的极限（2026-11-15 · issue #20 / PR #47）
- `unit137-limit-at-infinity`：1 定义 + 7 题。`TendsToAtInfinity f L` 把 δ 换成朝右的分界线 M：越过 M 后 `close (f x) L ε`。常函数、平移、最终相等三个样本；`close_lt_add_right` 从接近性读出 `x < L + ε`，恒等函数用它给出反例（`not_tendsToAtInfinity_id`）。正文 1 / 7，解答 8 / 0。
- `unit138-limit-at-infinity-algebra`：1 定义 + 7 题。ε 折半、两条分界线并成一条得和律；取负、差、加常数依次接上。`TendsToInfinity` 与序列判据把「最终越过每条分界线」翻译成 `f (x n) → L`。正文 1 / 7，解答 8 / 0。
- 引擎：`lib/LimInfinity`（8）、`lib/LimInfinityAlgebra`（8），`lib/Demo` 补三条自检。判据：`check.py` rc=0（361 目标 · 3433 checked · 0 rejected，M4 15/27）；`notation-lint` rc=0（374 文件）；文案 0；`docs-lint` rc=0。边界：函数与序列停在有理代表元层；不把 ±∞ 当成数。不新增公理，也不碰 kernel/front。

## M4 单元135–136：最后一个负点与条件版介值定理（2026-11-15 · issue #20 / PR #47）
- `unit135-last-negative`：2 定义 + 13 题。f m < 0 且 m 在区间内部时，保号性给出全负的 δ 邻域；在 m 右侧找同时落在 δ 与 [a,b] 内的点：m + δ/2 没过 b 就迈半步，否则取 m 到 b 的中点。于是严格负点集没有最大元。解答 15 / 0。
- `unit136-ivt`：2 定义 + 11 题。非正点集有最大元 m 且 m < b：f m ≤ 0，严格负一支交给 135 排除，故 f m ~ 0；再用 `RespectsRel` 排除端点等价。解答 13 / 0。
- 引擎：`lib/LocalSign`（7）、`lib/LastNegative`（15），`lib/Demo` 补 import 与两条自检。判据：`check.py` 0 rejected、0 solutions_open（M4 13/27）；`notation-lint` rc=0（366 文件）；文案 0（7 文件）；`docs-lint` rc=0。

## M4 单元134：闭区间上的最值（2026-11-15 · issue #20 / PR #47）
- `unit134-extreme-values`：5 定义 + 5 展开 + 16 题。最值定理拆成两半：`IsMaxOn` 把「取到」与「压住所有值」分成两个分量；`maxOn_isSup` 说明最大值就是像的上确界；`maxOn_iff_isSup` 给出条件版——最大值存在恰好等价于像的上确界存在且被取到。连续性在 ℚ 上补不上最后一步，于是转向能证明的充分条件：`attMax_of_monotone` / `attMin_of_monotone` 把单调函数的右端点立成最大值、左端点立成最小值，`attMax_id_closed` 是 [0,1] 上的样本。反例 `not_attMax_id_open` 把 [0,1] 换成 (0,1)：任取 c，中点 c + (1-c)/2 仍在内且更大。正文 10 / 16，解答 26 / 0。
- 引擎进库：新建 `lib/MaxOn`（26，条件版最值定理、单调版本、中点两条与一正一反两个样本），`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（349 目标 · 3335 checked · 699 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 11/27）；`notation-lint` rc=0（360 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0，触及文件；lib 根目录 34 处为 `Equiv` 既有）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；连续函数的无条件最值定理要像的上确界被取到，ℚ 上不成立、ℝ 上靠完备性，留到完备性一章。不新增公理，也不碰 kernel/front。
