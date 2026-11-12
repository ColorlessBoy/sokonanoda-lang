## M3 单元73–74：ε-N 极限定义 + 极限唯一（2026-11-13 · issue #19 / draft PR）
- `unit73-seq-limit`：M3 开工。`Eventually P := ∃ N, ∀ n ≥ N, P n`；`Converges x L := ∀ eps > 0, Eventually (fun n => close (x n) L eps)`。结清 `eventually_mono` / `eventually_and`（阈值取 58 的 `leNat_max`）/ `converges_iff` / `close_of_ratRel` / `converges_const` / 常零序列。画布 2 checked / 6 open / 0 rejected，解答 8 checked / 0 open。
- `unit74-limit-unique`：极限唯一。① 减法与序（`(x-y)+y ~ x`、`0 ~ x-y ⇒ x ~ y`、`y < x ⇒ 0 < x-y`）；② 任意 ε 内接近 ⇒ `≤`（`ratLeRep_eq_or_lt` 用 ㊺ 的 `leNat_eq_or_succ` 二选一；`close_sub_le` + `ratHalf_lt_self` 夹出矛盾）；③ `close_trans` 合 `eps/2` 得 `ratRel L L'`，`Real.sound` 得 `Real.mk (constSeq L) = Real.mk (constSeq L')`。画布 9 open / 0 rejected，解答 16 checked / 0 open。
- 判据：`check.py --json` **rc=0** —— 181 目标 · 2347 checked · 358 open · **0 判负**（M3 2/51；M1+M2 72/72 不变）；`notation-lint --root courses/analysis` **rc=0**（189 文件，0 命中，15 处显式豁免）。
- 边界：停在有理代表元层（`Real` 的函数加法仍是三元关系 `addRel`）；不新增公理、不碰 kernel/front。`lib/Limit` 收 73 的 8 条供后续单元引用；III.1 计划 11 单元，下一单元 = 收敛序列的有界性。

## M2 44/44 收口：ℚ 性质 + Cauchy + ℝ + 完备性（2026-11-12 · issue #18 / PR #32）
- 44 单元全部落盘（II.3 ㉙–㉞ · II.4 ㉟–㊹ · II.5 ㊺–54 · II.6 55–68 · II.7 69–72），每个画布 ≥2 题、解答 0 open；M1+M2 共 72 单元，`course.json` 编号 1–72 无缺号。判据：`check.py` **rc=0** —— **176 目标 · 2313 checked · 343 open · 0 判负**；`notation-lint --root courses/analysis` **rc=0**（184 文件 = 181 `.sokonanoda` + 3 `.md`，0 命中，15 处显式豁免）。
- 边界（不静默）：68 只做**代表元层** Cauchy 完备性（有理 Cauchy 序列收敛到自己的类）；一般 LUB 需 `Classical.choice` / Dedekind 切割，L2 没有 ⇒ 72 只证具体集合 `{x | x ≤ 0}` 有 LUB，差额记在 #18；不把 ℝ 立公理、不加课程 axiom（`axiom` 只出现在 C0 探针）。
- 章配额偏差随 #18 报告：II.3 6/10 · II.4 10/10 · II.5 10/10 · II.6 14/9 · II.7 4/5 = 44；练习配额差额只由 `check.py` 报告、不判红。
## M1 单元㉘ ℚ 乘法的良定义性（2026-11-10 · issue #17 / PR #24）
- `unit28-rat-mul-wd`：乘法两条代表元层义务 `ratMulRep_wd`（x~x' ⇒ x*y~x'*y）与 `ratMulRep_wd_snd`（y~y' ⇒ x*y~x*y'）。`rat_cross` 先把同分母 `ratRel` 化成 `a+b'=a'+b`，整数层 `mulRep_wd` / `mulRep_wd_snd` 把相等搬到两个分子，最后乘回公分母。画布 **80 checked / 2 open**，解答 **82 checked / 0 open**.
- 判据：课程门禁 **67 目标 · 1580 checked · 132 open · 0 判负**（0.98.4 仓库构建）；M1 单元 **28/28**（II.1 **19/19** · II.2 **9/9**）。
- 未收口：`notation-lint --root courses/analysis` 仍 **rc=1**（613 处 / 75 文件，issue #29）；在 notation 清零与审计阻塞项修完前不宣称 M1 完成。仍不下降成 `Rat → Rat → Rat`（无 `funext`，issue #28），不加公理.
