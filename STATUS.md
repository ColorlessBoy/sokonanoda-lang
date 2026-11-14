## M4 单元128–129：连续函数的代数与单侧极限（2026-11-14 · issue #20 / PR #47）
- `unit128-continuity-algebra`：7 题。`continuousAt_add` 用 ε 折半、`ratLeRep_total` 取较小半径、`close_mono_eps` 统一半径、`close_add` 合成；`continuousAt_neg` 由 `close_neg` 搬；`continuousAt_sub` 接在两者之后；三条全局版本逐点复用；demo 是恒等函数加常数。正文 0 / 7，解答 7 / 0。
- `unit129-one-sided-limits`：2 条定义 + 8 题。`TendsToAtRight` / `TendsToAtLeft` 在 ε-δ 上加方向条件；常函数与恒等函数的右极限直接算；双侧极限用方向条件消掉「x 不是 a」；拼合定理按序的全性分左右，再用「相等或严格小」二分，相等一支要求函数在 a 处对代表元等价不敏感（写成显式假设）。正文 2 checked / 8 open，解答 10 / 0。
- 引擎进库：新建 `lib/LimAtAlgebra` +4（`close_mono_eps` / `tendsToAt_add` / `tendsToAt_neg` / `tendsToAt_sub`，来自 127），128 起直接引用。
- 判据：`check.py --json` rc=0（334 目标 · 3190 checked · 654 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 6/27）；`notation-lint` rc=0（345 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：单侧极限停在有理代表元层；拼合定理要求函数在 a 处保等价；乘积与商仍在后续单元。不新增公理。

## M4 单元126–127：序列判据与函数极限的代数（2026-11-14 · issue #20）
- `unit126-sequential-criterion`：6 题。`converges_apply_of_tendsToAt` 把函数极限接到序列极限（两个阈值用 `leNat_max` 合并）；`converges_apply_of_tail` 换一条终究相同的序列；连续情形不需要「各项不取 a」，得 `converges_apply_of_continuousAt` / `_continuous`；`not_tendsToAt_of_seq` 是逆否形式。正文 0 / 6，解答 6 / 0。
- `unit127-limit-algebra`：5 题。`close_mono_eps` 先把 ε 放大；`tendsToAt_add` 用 ε 折半、`ratLeRep_total` 取较小半径、`close_add` 合成；`tendsToAt_neg` 由 `close_neg` 搬；`tendsToAt_sub` 接在两者之后。正文 0 / 5，解答 5 / 0。
- 引擎进库：`lib/LimAt` +2（`tendsToAt_const` / `tendsToAt_id`）；`lib/Cont` +6（常函数、恒等函数、复合的连续性）。
- 判据：`check.py --json` rc=0（329 目标 · 3167 checked · 639 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 4/27）；`notation-lint` rc=0（340 文件，15 处豁免）；文案 0 处（BLOCK 0 / WARN 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。边界：顺序判据只做正方向，反方向缺选择函数；乘积与商留在后续单元。不新增公理。

## M3 单元122–123：尾块判别的反方向与交换前两项（2026-11-13 · issue #19 / PR #41）
- `unit122-tail-reverse`：3 题。`tailSum_shift` 用 `add_assoc` 对齐尾块下标；`summable_of_tail` 用 `summable_iff_tail`、`leNat_exists_add` 与 `leNat_add_cancel_right` 把尾块估计读回原级数；`summable_tail_iff` 与 121 的 `summable_tail` 合成充要条件。正文 1 checked / 3 open，解答 4 / 0。
- `unit123-series-swap`：3 题。`swapFirst` 交换前两项；`swapFirst_eq_of_two_le` 证明下标 ≥ 2 不再改项；`tailSum_swapFirst` 把尾块相等抬起来；`summable_swapFirst_iff` 用 `summable_iff_tail` 与 `leNat_max` 收口。正文 2 checked / 3 open，解答 10 / 0。
- 判据：`check.py --json` rc=0（319/3118/614/0，M3 51/51）；`notation-lint` rc=0（330 文件）；文案 0 处；`style-lint --selftest` rc=0。边界：122 只走反方向；123 只交换前两项，一般置换与和类不变不在本单元。不新增公理。
