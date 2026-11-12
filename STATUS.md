## M3 单元97–99：上下极限的收敛与 Cauchy 极限点（2026-11-13 · issue #19 / PR #41）
- `unit97`（上轮落盘）：尾部等价沿子序列复合，并搬运极限点与上下极限（`limitPoint_congr_tail` / `limsup_congr_tail` / `liminf_congr_tail`）。正文 2 checked / 6 open，解答 7 checked / 0 open。
- `unit98-limsup-converges`：6 道练习。收敛时上下极限等价、空隙排除收敛、极限点落在两界之间；有极限点时两界相等（`limsup_liminf_rel_of_le`）。正文 0 checked / 6 open / 0 rejected，解答 6 checked / 0 open。
- `unit99-cauchy-limit`：5 道练习。Cauchy 序列有极限点就收敛到它（`cauchy_limitPoint_converges`），极限点至多一个等价类，上下极限落在同一点。正文 0 checked / 5 open / 0 rejected，解答 5 checked / 0 open。
- 判据：`check.py --json` rc=0（246 目标 · 2714 checked · 506 open · 0 判负，M3 27/51）；`notation-lint` rc=0（254 文件，15 处豁免）；III.3 文案门禁 0 处；`style-lint --selftest` rc=0。
- 边界：98 不反推收敛（要可数选择）；99.0 保留极限点前提。不新增公理、不碰 kernel/front。下一单元：III.4。

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
