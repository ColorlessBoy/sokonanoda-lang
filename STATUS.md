## M3 单元102–103：Cauchy 性的写法与实数差的刻画（2026-11-13 · issue #19 / PR #41）
- `unit102-cauchy-shapes`：7 题。定向形式与尾部跳跃形式都和 Cauchy 原定义等价（`cauchy_oriented` / `cauchy_tail_jump` 及其逆与 `iff`），常序列实例。正文 0 checked / 7 open / 0 rejected，解答 7 checked / 0 open。
- `unit103-realrel-diff`：5 题。`close a b ε ↔ close (a-b) 0 ε`（`close_sub_zero`）；`realRel` 等价于两个 Cauchy 序列的差收敛到零。正文 1 checked（`diffSeq`）/ 5 open / 0 rejected，解答 6 checked / 0 open。
- `lib/CauchyShapes`（7 checked）、`lib/RealDiff`（6 checked）；`lib/Demo` 补 import 与两条自检（16 checked）。
- 判据：`check.py --json` rc=0（258 目标 · 2780 checked · 529 open · 0 rejected，M3 31/51）；`notation-lint` rc=0（266 文件）；III.4 与新增 lib 文案门禁 0 处；`style-lint --selftest` rc=0。边界：102 不改判定，相邻项不足以推 Cauchy；103 不构造实数减法。下一单元：III.4 单元104。

## M3 单元100–101：Cauchy 尾部刻画与有界性（2026-11-13 · issue #19 / PR #41）
- `unit100-cauchy-tail`：6 道练习。Cauchy 性等价于「尾巴上的项都靠近 x N」（`cauchy_tail` / `cauchy_of_tail` / `cauchy_tail_iff`）；平移保持 Cauchy（`cauchy_shift`），终究为常数 ⇒ Cauchy（`cauchy_of_eventually_const`），常序列实例。正文 0 checked / 6 open / 0 rejected，解答 6 checked / 0 open。
- `unit101-cauchy-bounded`：5 道练习。Cauchy 序列尾部有界（`cauchy_eventually_bounded`）⇒ 整条有界（`cauchy_bounded`）；平移与加常数保持有界（`cauchy_bounded_shift` / `bounded_add_const`）。正文 0 checked / 5 open / 0 rejected，解答 5 checked / 0 open。
- `lib/Bounded`（77 的机器，14 checked）、`lib/CauchyTail`（100 的六条，6 checked）、`lib/Demo` 补 import 与两条自检（14 checked）。
- 判据：`check.py --json` rc=0（252 目标 · 2749 checked · 517 open · 0 rejected，M3 29/51）；`notation-lint` rc=0（260 文件，15 处豁免）；III.4 与新增 lib 文案门禁 0 处；`style-lint --selftest` rc=0。
- 边界：只证 Cauchy ⇒ 有界，不反推；有界序列未必 Cauchy。不新增公理、不碰 kernel/front。下一单元：III.4 单元102。

## M3 单元97–99：上下极限的收敛与 Cauchy 极限点（2026-11-13 · issue #19 / PR #41）
- `unit97`（上轮落盘）：尾部等价沿子序列复合，并搬运极限点与上下极限（`limitPoint_congr_tail` / `limsup_congr_tail` / `liminf_congr_tail`）。正文 1 checked / 6 open，解答 7 checked / 0 open。
- `unit98-limsup-converges`：6 道练习。收敛时上下极限等价、空隙排除收敛、极限点落在两界之间；有极限点时两界相等（`limsup_liminf_rel_of_le`）。正文 0 checked / 6 open / 0 rejected，解答 6 checked / 0 open。
- `unit99-cauchy-limit`：5 道练习。Cauchy 序列有极限点就收敛到它（`cauchy_limitPoint_converges`），极限点至多一个等价类，上下极限落在同一点。正文 0 checked / 5 open / 0 rejected，解答 5 checked / 0 open。
- 判据：`check.py --json` rc=0（246 目标 · 2714 checked · 506 open · 0 判负，M3 27/51）；`notation-lint` rc=0（254 文件，15 处豁免）；III.3 文案门禁 0 处；`style-lint --selftest` rc=0。
- 边界：98 不反推收敛（要可数选择）；99.0 保留极限点前提。不新增公理、不碰 kernel/front。下一单元：III.4。
