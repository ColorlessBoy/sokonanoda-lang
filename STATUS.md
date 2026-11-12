## M3 单元77：收敛序列有界（2026-11-13 · issue #19 / PR #41）
- `unit77-converges-bounded`：乘法律的前置。`Bounded x := ∃ B, ∀ n, x n ≤ B ∧ -x n ≤ B`。尾部取 ε = 1：`close_two_bounds` 从 `close` 拆出 `x - L ≤ 1` 与 `-x + L ≤ 1`（`close_sub_le` + `ratAbsRel_neg_le`），`le_of_sub_le` / `le_of_neg_add_le` 搬成 `x ≤ 1 + L` 与 `-x ≤ 1 + (-L)`；前缀的界对 N 归纳（`exists_prefix_bound`）。两处「较大者」都写成存在命题（`exists_two_sided_bound` / `exists_upper_of_two`）——全性只给析取，目标是存在时可直接消去。主定理用 `leNat_total` 分两支。画布 5 checked / 9 open，解答 14 checked / 0 open。
- 判据：`check.py --json` **rc=0** —— 189 目标 · 2394 checked · 376 open · **0 判负**（M3 5/51；M1+M2 72/72 不变）；`notation-lint --root courses/analysis` **rc=0**（0 命中）；文案门禁对本单元两个文件 **0 处**。
- 边界：有界性只是乘法律的前置之一；乘积的上界还要乘法与序相容（`0 ≤ a ≤ A`、`0 ≤ b ≤ B ⇒ ab ≤ AB`），尚未证明，留到后续单元。不新增公理、不碰 kernel/front。下一单元 = 夹逼定理。

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
