## M3 单元107–108：极限类的取负与减法（2026-11-13 · issue #19 / PR #41）
- `unit107-limit-class-neg`：7 题。`(-y) - ((-x) + eps)` 与 `x - (y + eps)` 认成同一个差（`ratSubRep_neg_shift`），逐点搬进 `realLeRep`（`neg_tail_le`），两条 ε 界互换后组装成 `tendsTo_neg`；极限类取负（`limitClass_neg`）。正文 7 open / 0 rejected，解答 7 checked / 0 open。
- `unit108-limit-class-sub`：5 题。逐点差 `subSeq`；106 的加法与 107 的取负接成 `tendsTo_sub`、`cauchy_sub`、`limitClass_sub`（`Exists.elim` 动机写全）；常序列实例。正文 5 open / 0 rejected，解答 6 checked / 0 open。
- `lib/LimitClassNeg`（6）+ `lib/LimitClassSub`（5）进库，`lib/Demo` 补两条自检（20 checked）。
- 判据：`check.py` rc=0（273 目标 · 2858 checked · 0 rejected，M3 36/51）；`notation-lint` rc=0；文案 BLOCK 0 / WARN 0。
- 边界：取负与减法都带序列的 Cauchy 见证；不构造实数减法。不新增公理。下一单元：单元109（完备性收口）。
## M3 单元105–106：两个 ε/2 界相加与极限类的加法（2026-11-13 · issue #19 / PR #41）
- `unit105-cauchy-add-bounds`：6 题。减法对第二个因子同余（`ratSubRep_wd_snd`）、四个加数重新配对（`ratAddRep_shift`）、两条差界相加（`ratLeRep_sub_pair`）、两个误差对齐到同一个 eps（`ratLeRep_sub_pair_eps`）、`d/2` 的两条界合成一条 `d` 界（`ratLeRep_sub_pair_half`）、零误差实例（`demo_add_shift`）。正文 0 checked / 6 open / 0 rejected，解答 6 checked / 0 open。
- `unit106-limit-class-add`：4 题。左右两半的 `realLeRep` 组装进库（`realLeRep_add_side` / `_right`）；`tendsTo_add` 把两条 `TendsTo` 合成和的收敛；`addRel_limit` 把 `a = [L]`、`b = [M]` 代回三元关系；`limitClass_add` 给出和类；常序列实例。正文 0 checked / 4 open / 0 rejected，解答 4 checked / 0 open。
- `lib/CauchyAddBounds`（7 checked）与 `lib/LimitClassAdd`（4 checked）进库，`lib/Demo` 补两个 import 与一条自检（18 checked）。
- 判据：`check.py --json` rc=0（267 目标 · 2827 checked · 547 open · 0 rejected，M3 34/51）；`notation-lint` rc=0（278 文件，15 处豁免）；III.4 与新增 lib 文案门禁 BLOCK 0 / WARN 0；`style-lint --selftest` rc=0。
- 边界：只在有极限类的序列上谈加法，不构造实数减法。不新增公理、不碰 kernel/front。下一单元：III.4 单元107。
## M3 单元104：极限类的唯一性（2026-11-13 · issue #19 / PR #41）
- `unit104-limit-class`：8 题。减法常数并进误差（`ratSubRep_add_self_sub` / `ratSubRep_add_le_two`）；`(eps/4)+(eps/4)+(eps/4)+(eps/4) ~ eps`（`ratHalf_four`）；固定下标后的尾部估计（`tendsTo_tail_le`）；`TendsTo` 给出一个方向的类序（`tendsTo_le_class`），反对称得两个极限类相等（`tendsTo_unique_class`）；`LimitClass` 谓词与唯一性（`limitClass_unique`）；常序列实例。正文 1 checked（`eps4`）/ 8 open / 0 rejected，解答 10 checked / 0 open。
- `lib/LimitClass`（9 checked）进库，`lib/Demo` 补 import 与一条自检（17 checked）。
- 判据：`check.py --json` rc=0（261 目标 · 2804 checked · 537 open · 0 rejected，M3 32/51）；`notation-lint` rc=0（269 文件，15 处豁免）；III.4 与新增 lib 文案门禁 BLOCK 0 / WARN 0；`style-lint --selftest` rc=0。
- 边界：只讨论同一个序列的两个极限类，不声称每个实数都有代表元，不在类层构造减法。不新增公理、不碰 kernel/front。下一单元：III.4 单元105。
