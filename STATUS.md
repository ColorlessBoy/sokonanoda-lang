## M3 单元95–96：子序列与上下极限、常数界（2026-11-13 · issue #19 / PR #41）
- `lib/SubseqLimit`：`subseqSeq`、`subseq_limitPoint`、`limsup_subseq_le`、`liminf_le_subseq`、两条收敛情形、`demo_subseq_const`；`lib/LimsupBound`：`BoundedAbove`/`BoundedBelow`、两条逐项界、两条终究界、`limsup_const_rel`、`demo_both_bounds`；`lib/Demo` 补 import 与三条自检。
- `unit95`：5 道练习；正文 2 checked / 5 open，解答 7 checked / 0 open。`unit96`：5 道练习；正文 3 checked / 5 open，解答 8 checked / 0 open。
- 判据：`check.py --json` rc=0（239 目标 · 2682 checked · 489 open · 0 判负，M3 24/51）；`notation-lint` rc=0（247 文件）；III.3 与新增 lib 文案门禁 0 处。
- 边界：子序列的界只有单向比较；常数界是充分条件。下一单元：尾部改动与上下极限。

## M3 单元93–94：上极限与下极限（2026-11-13 · issue #19 / PR #41）
- `lib/Limsup`：`Limsup`（极限点集合的上确界）、`limsup_upper`、`limsup_least`、`limsup_le_iff`、`limsup_unique`、`limsup_of_converges`、`limsup_unique_of_converges`、`limsup_const`；`lib/Liminf`：`Liminf`、`liminf_lower`、`liminf_greatest`、`le_liminf_iff`、`liminf_unique`、`liminf_of_converges`、`liminf_unique_of_converges`、`liminf_const`、`liminf_le_limsup`；`lib/Demo` 补两个 import 与两条自检。
- `unit93-limsup`：5 道练习；正文 3 checked / 5 open / 0 rejected，解答 8 checked / 0 open。
- `unit94-liminf`：6 道练习；正文 3 checked / 6 open / 0 rejected，解答 9 checked / 0 open。
- 判据：`check.py --json` rc=0（233 目标 · 2641 checked · 479 open · 0 判负，M3 22/51）；`notation-lint` rc=0（241 文件，15 处豁免）；III.3 文案门禁 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：两个界定成关系不是函数，任意序列的存在性未证（要选择公理或 Dedekind 切割）；`liminf_le_limsup` 带非空前提。不新增公理、不碰 kernel/front。下一单元：子序列与上下极限（取子序列不升上极限、不降下极限）。

## M3 单元91–92：子序列与极限点（2026-11-13 · issue #19 / PR #41）
- `lib/Subseq`：`Subseq`（严格递增的下标轨道）、`subseq_id`、`subseq_ge`（轨道不回头）、`subseq_le`（保序）、`subseq_comp`、`shiftIndex_subseq`、`subseq_converges`；`lib/Demo` 补 import 与一条自检。
- `unit91-subsequence`：6 道练习；正文 2 checked / 6 open / 0 rejected，解答 8 checked / 0 open。
- `lib/LimitPoint`：`LimitPoint`、`converges_limitPoint`、`limitPoint_const`、`limitPoint_frequently`、`limitPoint_unique_of_converges`、`not_converges_of_two_limitPoints`、`const_limitPoint_unique`；`unit92-limit-point`：5 道练习；正文 2 checked / 5 open / 0 rejected，解答 7 checked / 0 open。
- 判据：`check.py --json` rc=0（227 目标 · 2597 checked · 468 open · 0 判负，M3 20/51）；`notation-lint` rc=0（235 文件，15 处豁免）；III.3 文案门禁 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：极限点由子序列定义，反方向（频繁靠近 ⇒ 子序列）需要可数选择，未证；不新增公理、不碰 kernel/front。下一单元：limsup 的定义（尾巴上界集合的下确界）与基本性质。

