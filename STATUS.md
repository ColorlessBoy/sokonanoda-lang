## M4 单元132：局部有界（2026-11-15 · issue #20 / PR #47）
- `unit132-local-boundedness`：2 定义 + 7 题。乘积的估计需要一个不随 x 变化的界，而极限只保证「离 L 近」。`LocallyBoundedAt` 管整个 δ 邻域，`LocallyBoundedPunct` 管去心邻域；连续取 ε = 1 得整邻域的盒子，有限极限因定义排掉 a 点只给去心版本；两条展开律、整邻域⇒去心、常函数与恒等函数两个样本。正文 2 / 7，解答 9 / 0。
- 引擎进库：新建 `lib/LocalBounded`（9，来自 132），`lib/RatClose` 增收绝对值非负与两个非负数之和非负三条，`lib/Demo` 补 import 与两条自检。
- 判据：`check.py` rc=0（343 目标 · 3246 checked · 0 rejected · 0 solutions_open · 0 lib_open，M4 9/27）；`notation-lint` rc=0（354 文件，15 处豁免）；文案 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：盒子依赖基点，不是整片定义域的统一界；乘积还要乘法对序的相容性，留到下一单元。不新增公理，也不碰 kernel/front。

## M4 单元130–131：一致连续与序列的像（2026-11-14 · issue #20 / PR #47）
- `unit130-uniform-continuity`：1 定义 + 7 题。`UniformlyContinuous` 把「对每个 a 存在 δ(a)」对调成「存在 δ 对每个 x、y」；展开律、常函数、恒等函数、一致连续⇒逐点连续、复合、平移各一题；引擎进库 `lib/Uniform`（8）。正文 1 / 7，解答 8 / 0。
- `unit131-uniform-continuity-sequences`：4 题。Cauchy 序列的像仍 Cauchy（ε 换 δ），收敛序列的像收敛到 `f L`；引擎进库 `lib/UniformSeq`（2），`lib/Demo` 补三条自检。正文 0 / 4，解答 4 / 0。
- 判据：`check.py` rc=0（3219 checked · 0 rejected · 0 open，M4 8/27）；`notation-lint` rc=0；文案 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：有理代表元层；非一致连续的反例与反方向留后。

## M4 单元128–129：连续函数的代数与单侧极限（2026-11-14 · issue #20 / PR #47）
- `unit128-continuity-algebra`：7 题。`continuousAt_add` 用 ε 折半、`ratLeRep_total` 取较小半径、`close_mono_eps` 统一半径、`close_add` 合成；`continuousAt_neg` 由 `close_neg` 搬；`continuousAt_sub` 接在两者之后；三条全局版本逐点复用；demo 是恒等函数加常数。正文 0 / 7，解答 7 / 0。
- `unit129-one-sided-limits`：2 条定义 + 8 题。`TendsToAtRight` / `TendsToAtLeft` 在 ε-δ 上加方向条件；常函数与恒等函数的右极限直接算；双侧极限用方向条件消掉「x 不是 a」；拼合定理按序的全性分左右，再用「相等或严格小」二分，相等一支要求函数在 a 处对代表元等价不敏感（写成显式假设）。正文 2 checked / 8 open，解答 10 / 0。
- 引擎进库：新建 `lib/LimAtAlgebra` +4（`close_mono_eps` / `tendsToAt_add` / `tendsToAt_neg` / `tendsToAt_sub`，来自 127），128 起直接引用。
- 判据：`check.py --json` rc=0（334 目标 · 3190 checked · 654 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 6/27）；`notation-lint` rc=0（345 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：单侧极限停在有理代表元层；拼合定理要求函数在 a 处保等价；乘积与商仍在后续单元。不新增公理。
