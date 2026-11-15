## M4 单元140：导数的定义（2026-11-15 · issue #20 / PR #47）
- `unit140-derivative-definition`：3 定义 + 11 题。`IsSlope f a q` 要求在 a 之外逐点满足 `q x · (x - a) ~ f x - f a`，`HasDerivAt f a L` 要求某个斜率函数的函数极限是 L。ℚ 的倒数还没有构造出来，差商因此写成乘积等式；x 与 a 不同时它唯一决定 q x，补上倒数后与商逐字相同。常函数与恒等函数分别给出导数零与导数一。正文 14 条声明（3 定义给出、11 题），解答 14 checked、0 open。
- 引擎：新建 `lib/RatRing`（17）与 `lib/Deriv`（14），`lib/Demo` 补 import 与三条自检。
- 判据：`check.py --json` rc=0（368 目标 · 3556 checked · 761 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 17/27）；`notation-lint` rc=0（381 文件，15 处既有豁免）；文案 0；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：函数停在有理代表元层；导数的唯一性要「非零因子可消去」，留到后续单元。不新增公理，也不碰 kernel/front，不改 M1/M2/M3。

## M4 单元139：单调函数与介值性（2026-11-15 · issue #20 / PR #47）
- `unit139-monotone-ivp`：2 定义 + 13 题。单调函数不连续的方式只剩跳跃；`HasIntermediateValue` 把「不跳过中间值」写成显式假设。主定理 `continuousAt_of_monotone_ivp` 给定 ε 后在 c 右侧找分界点：右端点值不超过 f c + ε/2 就取右端点，否则介值性在 c 与 b 之间给出取到阈值的点；左侧对称。δ 取两段中较小者，x 用序的全性分左右。推论 `continuousOn_open_of_monotone_ivp`，恒等函数两个样本。正文 26 条声明（13 给出、13 题），解答 26 checked / 0 open。
- 引擎：`lib/Monotone`（26），`lib/Demo` 补 import 与两条自检。判据：`check.py` rc=0（364 目标 · 3498 checked · 750 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 16/27）；`notation-lint` rc=0（378 文件）；文案 0（BLOCK 0 / WARN 0，触及文件）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：函数停在有理代表元层；介值性是假设，不是 ℚ 上连续性的推论。不新增公理，也不碰 kernel/front。

## M4 单元137–138：无穷远处的极限（2026-11-15 · issue #20 / PR #47）
- `unit137-limit-at-infinity`：1 定义 + 7 题。`TendsToAtInfinity f L` 把 δ 换成朝右的分界线 M：越过 M 后 `close (f x) L ε`。常函数、平移、最终相等三个样本；`close_lt_add_right` 从接近性读出 `x < L + ε`，恒等函数用它给出反例（`not_tendsToAtInfinity_id`）。正文 1 / 7，解答 8 / 0。
- `unit138-limit-at-infinity-algebra`：1 定义 + 7 题。ε 折半、两条分界线并成一条得和律；取负、差、加常数依次接上。`TendsToInfinity` 与序列判据把「最终越过每条分界线」翻译成 `f (x n) → L`。正文 1 / 7，解答 8 / 0。
- 引擎：`lib/LimInfinity`（8）、`lib/LimInfinityAlgebra`（8），`lib/Demo` 补三条自检。判据：`check.py` rc=0（361 目标 · 3433 checked · 0 rejected，M4 15/27）；`notation-lint` rc=0（374 文件）；文案 0；`docs-lint` rc=0。边界：函数与序列停在有理代表元层；不把 ±∞ 当成数。不新增公理，也不碰 kernel/front。
