## M3 单元75–76：极限与序 + 极限的代数（2026-11-13 · issue #19 / PR #41）
- `unit75-limit-order`：极限保持非严格不等式。`ratLeRep_of_sub_le_all`（任意 ε 内 `x−y ≤ ε` ⇒ `x ≤ y`，反证用 74.3/74.5）；`converges_le_of_eventually` 把三条「终究」并到同一 `N`，`(L−x n)+(x n−M) ≤ ε/2+ε/2`，再用 `ratSubRep_add`/`ratHalf_add_self` 换形；推论 `converges_nonneg`/`converges_le_const`。画布 0 checked / 4 open，解答 4 checked / 0 open。
- `unit76-limit-algebra-add`：极限律的加法群。`converges_congr_limit`（`L ~ L'` 换极限）、`converges_add`（`eventually_and` + `close_add`）、`converges_neg`（`close_neg`）、`converges_sub`（定义展开）、`demo_add_zero`。画布 1 checked / 5 open，解答 6 checked / 0 open。
- 判据：`check.py --json` **rc=0** —— 187 目标 · 2375 checked · 367 open · **0 判负**（M3 4/51；M1+M2 72/72 不变）；`notation-lint --root courses/analysis` **rc=0**（195 文件，0 命中，15 处豁免）。
- 引擎进库：`lib/LimitOrder`（74 的 12 条）与 `lib/RealNeg`（59 的 5 条）。边界：乘法律要有界性，留到后续单元；不新增公理、不碰 kernel/front。下一单元 = 收敛序列的有界性。

## M3 单元73–74：ε-N 极限定义 + 极限唯一（2026-11-13 · issue #19 / draft PR）
- `unit73-seq-limit`：M3 开工。`Eventually P := ∃ N, ∀ n ≥ N, P n`；`Converges x L := ∀ eps > 0, Eventually (fun n => close (x n) L eps)`。结清 `eventually_mono` / `eventually_and`（阈值取 `leNat_max`）/ `converges_iff` / `close_of_ratRel` / `converges_const` / 常零序列。画布 2 checked / 6 open，解答 8 checked / 0 open。
- `unit74-limit-unique`：① 减法与序（`(x-y)+y ~ x`、`0 ~ x-y ⇒ x ~ y`、`y < x ⇒ 0 < x-y`）；② 任意 ε 内接近 ⇒ `≤`（`ratLeRep_eq_or_lt` 用 `leNat_eq_or_succ` 二选一；`close_sub_le` + `ratHalf_lt_self` 夹逼）；③ `close_trans` 合 `eps/2` 得 `ratRel L L'`，`Real.sound` 得两个类相等。画布 9 open，解答 16 checked / 0 open。
- 判据：`check.py --json` **rc=0** —— 181 目标 · 2347 checked · 358 open · **0 判负**；`notation-lint` **rc=0**（189 文件，0 命中，15 处豁免）。
- 边界：停在有理代表元层（`Real` 的函数加法仍是 `addRel`）；不新增公理、不碰 kernel/front。`lib/Limit` 收 73 的 8 条。

## M2 44/44 收口：ℚ 性质 + Cauchy + ℝ + 完备性（2026-11-12 · issue #18 / PR #32）
- 44 单元全部落盘（II.3 ㉙–㉞ · II.4 ㉟–㊹ · II.5 ㊺–54 · II.6 55–68 · II.7 69–72），每个画布 ≥2 题、解答 0 open；M1+M2 共 72 单元，`course.json` 编号 1–72 无缺号。判据：`check.py` **rc=0** —— **176 目标 · 2313 checked · 343 open · 0 判负**；`notation-lint --root courses/analysis` **rc=0**（184 文件 = 181 `.sokonanoda` + 3 `.md`，0 命中，15 处显式豁免）。
- 边界（不静默）：68 只做**代表元层** Cauchy 完备性（有理 Cauchy 序列收敛到自己的类）；一般 LUB 需 `Classical.choice` / Dedekind 切割，L2 没有 ⇒ 72 只证具体集合 `{x | x ≤ 0}` 有 LUB，差额记在 #18；不把 ℝ 立公理、不加课程 axiom（`axiom` 只出现在 C0 探针）。
- 章配额偏差随 #18 报告：II.3 6/10 · II.4 10/10 · II.5 10/10 · II.6 14/9 · II.7 4/5 = 44；练习配额差额只由 `check.py` 报告、不判红。
