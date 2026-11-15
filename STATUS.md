## M4 单元139：单调函数与介值性（2026-11-15 · issue #20 / PR #47）
- `unit139-monotone-ivp`：2 定义 + 13 题。单调函数不连续的方式只剩跳跃；`HasIntermediateValue` 把「不跳过中间值」写成显式假设。主定理 `continuousAt_of_monotone_ivp` 给定 ε 后在 c 右侧找分界点：右端点值不超过 f c + ε/2 就取右端点，否则介值性在 c 与 b 之间给出取到阈值的点；左侧对称。δ 取两段中较小者，x 用序的全性分左右。推论 `continuousOn_open_of_monotone_ivp`，恒等函数两个样本。正文 26 条声明（13 给出、13 题），解答 26 checked / 0 open。
- 引擎：`lib/Monotone`（26），`lib/Demo` 补 import 与两条自检。判据：`check.py` rc=0（364 目标 · 3498 checked · 750 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 16/27）；`notation-lint` rc=0（378 文件）；文案 0（BLOCK 0 / WARN 0，触及文件）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；介值性是假设，不是 ℚ 上连续性的推论。不新增公理，也不碰 kernel/front。

## M4 单元137–138：无穷远处的极限（2026-11-15 · issue #20 / PR #47）
- `unit137-limit-at-infinity`：1 定义 + 7 题。`TendsToAtInfinity f L` 把 δ 换成朝右的分界线 M：越过 M 后 `close (f x) L ε`。常函数、平移、最终相等三个样本；`close_lt_add_right` 从接近性读出 `x < L + ε`，恒等函数用它给出反例（`not_tendsToAtInfinity_id`）。正文 1 / 7，解答 8 / 0。
- `unit138-limit-at-infinity-algebra`：1 定义 + 7 题。ε 折半、两条分界线并成一条得和律；取负、差、加常数依次接上。`TendsToInfinity` 与序列判据把「最终越过每条分界线」翻译成 `f (x n) → L`。正文 1 / 7，解答 8 / 0。
- 引擎：`lib/LimInfinity`（8）、`lib/LimInfinityAlgebra`（8），`lib/Demo` 补三条自检。判据：`check.py` rc=0（361 目标 · 3433 checked · 0 rejected，M4 15/27）；`notation-lint` rc=0（374 文件）；文案 0；`docs-lint` rc=0。边界：函数与序列停在有理代表元层；不把 ±∞ 当成数。不新增公理，也不碰 kernel/front。

## M4 单元135–136：最后一个负点与条件版介值定理（2026-11-15 · issue #20 / PR #47）
- `unit135-last-negative`：2 定义 + 13 题。f m < 0 且 m 在区间内部时，保号性给出全负的 δ 邻域；在 m 右侧找同时落在 δ 与 [a,b] 内的点：m + δ/2 没过 b 就迈半步，否则取 m 到 b 的中点。于是严格负点集没有最大元。解答 15 / 0。
- `unit136-ivt`：2 定义 + 11 题。非正点集有最大元 m 且 m < b：f m ≤ 0，严格负一支交给 135 排除，故 f m ~ 0；再用 `RespectsRel` 排除端点等价。解答 13 / 0。
- 引擎：`lib/LocalSign`（7）、`lib/LastNegative`（15），`lib/Demo` 补 import 与两条自检。判据：`check.py` 0 rejected、0 solutions_open（M4 13/27）；`notation-lint` rc=0（366 文件）；文案 0（7 文件）；`docs-lint` rc=0。
