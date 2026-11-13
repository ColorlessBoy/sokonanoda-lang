## M3 单元104：极限类的唯一性（2026-11-13 · issue #19 / PR #41）
- `unit104-limit-class`：8 题。减法常数并进误差（`ratSubRep_add_self_sub` / `ratSubRep_add_le_two`）；`(eps/4)+(eps/4)+(eps/4)+(eps/4) ~ eps`（`ratHalf_four`）；固定下标后的尾部估计（`tendsTo_tail_le`）；`TendsTo` 给出一个方向的类序（`tendsTo_le_class`），反对称得两个极限类相等（`tendsTo_unique_class`）；`LimitClass` 谓词与唯一性（`limitClass_unique`）；常序列实例。正文 1 checked（`eps4`）/ 8 open / 0 rejected，解答 10 checked / 0 open。
- `lib/LimitClass`（9 checked）进库，`lib/Demo` 补 import 与一条自检（17 checked）。
- 判据：`check.py --json` rc=0（261 目标 · 2804 checked · 537 open · 0 rejected，M3 32/51）；`notation-lint` rc=0（269 文件，15 处豁免）；III.4 与新增 lib 文案门禁 BLOCK 0 / WARN 0；`style-lint --selftest` rc=0。
- 边界：只讨论同一个序列的两个极限类，不声称每个实数都有代表元，不在类层构造减法。不新增公理、不碰 kernel/front。下一单元：III.4 单元105。

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

