# 证明助手里的集合论课程与形式化调研

> **访问日期**：2026-09-18（`date` → `Fri Sep 18 20:27 CST 2026`）。
> **取证纪律**：每条具体断言都对应一次实际抓取（`web_fetch` / `curl`），并给出 URL；标注 **未核实** 的是"没抓到证据"，不是"大概没有"。
> **会话限制**：`web_search` 不可用（缺 `DEEPSEEK_API_KEY`），全部靠 `web_fetch` + `curl`（`raw.githubusercontent.com`、`cdn.jsdelivr.net/gh/…`、`api.github.com`）。
> **读者**：正在给 sokonanoda（Lean 4 子集教学语言）设计「集合论」教程的人。

## §1 Mathlib `Set` API：入门 50 引理与记法表

**1.1 地基（一条决定骨架的事实）** `Mathlib/Data/Set/Defs.lean:51` 原文 `def Set (α : Type u) := α → Prop`；`Basic.lean` 模块 docstring 原文 "Sets in Lean are homogeneous… thus defined as `Set X := X → Prop`. Note that this function need not be decidable."，并指路 "See also the directory `Mathlib/SetTheory/ZFC/`"。→ **sokonanoda 的 `Set α := α → Prop` 与 Mathlib 逐字相同**。
（https://raw.githubusercontent.com/leanprover-community/mathlib4/master/Mathlib/Data/Set/Defs.lean ，…/Data/Set/Basic.lean ）

**1.2 目录组织（GitHub Contents API 实测，非猜测；docs 快照 commit `797def14…`）**
`Mathlib/Data/Set/` 共 **46 项 = 41 个 `.lean` + 5 个子目录**。子目录：`Card/`(→`Arithmetic.lean`)、`Finite/`(`Basic Lattice Lemmas List Monad Powerset Range`)、`Lattice/`(`Bounded Disjoint Image Indexed Order`)、`Pairwise/`(`Basic Chain Lattice List`)、`Pointwise/`(`Support.lean`)。文件：`Accumulate Basic BoolIndicator BooleanAlgebra Card CoeSort Constructions Countable Defs Disjoint Dissipate Enumerate Equitable FiniteExhaustion Function Functor Image Inclusion Insert Lattice List MemPartition Monotone MulAntidiagonal NAry Notation Operations Opposite Order Piecewise PowersetCard Prod Restrict SMulAntidiagonal Semiring Sigma Subset Subsingleton Sups SymmDiff UnionLift`。
**⚠️ 纠正任务书四处**：①`Mathlib/Order/Set` **目录不存在**（404）——但 `Mathlib/Order/` 下有 5 个平铺文件 `Set.lean`(629 B，内容仅 `WithBot.range_eq`/`WithTop.range_eq`)、`SetAccumulate.lean`、`SetDissipate.lean`、`SetIsMax.lean`、`SetNotation.lean`（**索引族 `⋃ i,`/`⋂ i,`/`⋃₀`/`⋂₀` 的记法就在这里**）；②`Mathlib/Data/Set/Lattice.lean` 自 **2026-08-22** 起是弃用空壳，内容拆到 `Lattice/{Bounded,Disjoint,Image,Indexed,Order}`；③**`Set.Equiv` 不存在**（Loogle 精确查询返回 `unknown identifier 'Set.Equiv'`，`Set.EquivalentOn` 同样不存在），正确的是 `Equiv.Set.*`（`Mathlib/Logic/Equiv/Set.lean`：`Equiv.Set.univ`、`Equiv.Set.powerset`、`Equiv.Set.union/singleton/compl/prod`、`Equiv.setCongr`、`Equiv.image`）；与"集合等价"最贴近的真名是 `Set.BijOn.equiv (f) (h : BijOn f s t) : ↑s ≃ ↑t` 与 `Set.equivOfEq (h : s = t) : ↑s ≃ ↑t`；④**没有 `Data/Set/Cardinal.lean` 也没有 `Data/Set/Equiv.lean`/`Preimage.lean`**（真名是 `Card.lean` + `Card/Arithmetic.lean`）。`MapsTo/InjOn/SurjOn/BijOn/EqOn/LeftInvOn/RightInvOn/InvOn` 全在 `Mathlib/Data/Set/Function.lean`（其文件头 docstring 逐条列出这 7 个概念，可当我们的概念清单直接抄）。

**1.3 记法表（❌ = 我们的语言没有；全部记法我们都缺。声明位置以**逐字原文**为准）**

| Mathlib 记法 | 声明原文（已验证） | 位置 | sokonanoda |
|---|---|---|---|
| `∈` / `∉` | `notation:50 a:50 " ∈ " b:50 => Membership.mem b a` / `notation:50 a:50 " ∉ " b:50 => ¬ (a ∈ b)` | **Lean core** `src/Init/Notation.lean:420,422` | ❌ 只能 `Set.mem α x s` 或直接 `s x` |
| `⊆` / `⊂` | `infix:50 " ⊆ " => Subset` / `infix:50 " ⊂ " => SSubset` | **core** `src/Init/Core.lean:539,542` | ❌ `Set.subset α s t` |
| `∪` / `∩` / `\` | `infixl:65 " ∪ " => Union.union`；`infixl:70 " ∩ " => Inter.inter`；`infix:70 " \ " => SDiff.sdiff` | **core** `Init/Core.lean:551,554,560` | ❌ |
| `∅` | `notation "∅" => EmptyCollection.emptyCollection` | **core** `Init/Core.lean:581` | ❌ `Set.empty α` |
| `sᶜ` | `class Compl` + `postfix:1024 "ᶜ" => compl` | `Mathlib/Order/Notation.lean:44,51` | ❌ |
| `𝒫` | **`prefix:100 "𝒫 " => powerset`，注意：不是 `scoped`**（任务书猜 scoped，实为全局） | `Mathlib/Data/Set/Defs.lean:261` | ❌ |
| `''` / `⁻¹'` | `infixr:80 " '' " => image` / `infixr:80 " ⁻¹' " => preimage` | `Data/Set/Operations.lean:144,138` | ❌ |
| `×ˢ` | `class SProd` + `infixr:82 " ×ˢ " => SProd.sprod` | `Mathlib/Data/SProd.lean:31,36` | ❌ |
| `⋃ i, f i` / `⋂ i, f i` | `notation3 "⋃ " …, " r:60:(scoped f => iUnion f) => r` 等 | **`Mathlib/Order/SetNotation.lean`**（不是 `Lattice/Indexed`） | ❌ 只能手写高阶函数 |
| `⋃₀` / `⋂₀` | `prefix:110 "⋃₀ " => sUnion` / `prefix:110 "⋂₀ " => sInter` | `Mathlib/Order/SetNotation.lean` | ❌ |
| `#s` | **`scoped prefix:arg "#" => Finset.card`——是 `Finset` locale，不是 `Nat.card`** | `Mathlib/Data/Finset/Card.lean:50` | ❌ |
| `#α` | `scoped prefix:max "#" => Cardinal.mk`（**`Cardinal` locale**） | `Mathlib/SetTheory/Cardinal/Defs.lean:92` | ❌ |
| `Set.univ` | `def univ : Set α := {_a \| True}`——**是 `def` 不是记号**（另有 `⊤`，`Set.top_eq_univ`） | `Data/Set/Defs.lean:215` | ❌ 只能 `Set.univ α` 或 `fun _ => True` |
| `Set.Icc` | `def Icc (a b : α) := { x \| a ≤ x ∧ x ≤ b }`——**无记号**，`open Set` 后写 `Icc a b` | `Mathlib/Order/Interval/Set/Defs.lean:76` | ❌（区间理论超出教程范围） |
| `↾` | **⚠️ 不存在**：`Data/Set/Restrict.lean` 全文无任何 `notation/infix/prefix/postfix/scoped`，docs 页也无 `↾`；`Set.restrict` 现为 `Set.domRestrict` 的 deprecated alias（since 2026-07-19），后者用点号调用 `s.domRestrict f` | `Data/Set/Restrict.lean:32,202` | ❌（本就不用） |
| `≃` / `↪` | `Equiv` / `Function.Embedding` | — | ❌ |
| `{a}` / `insert a s` | `Singleton`，`Defs.lean:230`；`Insert`，`Defs.lean:223` | `Data/Set/Defs.lean` | ❌（STG4 全程用 `{x}`） |

**⚠️ 2026 年重命名坑（写教程引用时必定撞到，逐字验证）**：①`Set.setOf`→**`Set.ofPred`**（deprecated since 2026-07-09）：`mem_setOf_eq`→`mem_ofPred_eq`、`eq_mem_setOf`→`eq_mem_ofPred`、`notMem_setOf_iff`→`notMem_ofPred_iff`、`sep_setOf`→`sep_ofPred`、`setOf_mem_eq`→`ofPred_mem_eq`；②`Set.restrict`→**`Set.domRestrict`**（since 2026-07-19），`range_restrict`→`range_domRestrict`、`image_restrict`→`image_domRestrict`；③序关系层 `Set.le_eq_subset`/`lt_eq_ssubset`/`le_iff_subset`/`lt_iff_ssubset` 全部 `@[deprecated "This is now a syntactic equality" (since := "2026-05-24")]`——**`⊆` 现在就是 `Subset` 的语法相等**。
结论：**Mathlib 集合记法我们一个都没有**——这是设计约束不是缺陷，变形见 §7。可行性已有正向证据：`docs/gaps/repro/OK-set-spike.sokonanoda`（`def Set (α : Type) : Type := α -> Prop` + `Set.mem` + `Set.subset` + `Set.empty/union/power`，11 声明全绿，0.58.0 实测）。

**1.4 「入门 50 引理」清单（✅ = 我在 raw 源码逐行见到；⚠️ = 未逐行核对）**

| # | 引理（✅） | 位置 |
|---|---|---|
| 1–4 | `Set.ext {a b} (h : ∀ x, x ∈ a ↔ x ∈ b) : a = b`；`Set.ext_iff`；`Set.mem_ofPred_eq {x p} : (x ∈ {y \| p y}) = p x`（**注意：`Set.mem_setOf_eq` 仍存在，但已是 `@[deprecated (since := "2026-07-09")] alias mem_setOf_eq := mem_ofPred_eq`**）；`Set.mem_of_mem_of_subset` | `Defs.lean:83`；`Operations.lean:79–81`；`Basic.lean:195` |
| 5–9 | `Set.subset_def`；**`Set.Subset.antisymm` / `Set.Subset.antisymm_iff` / `Set.eq_of_subset_of_subset`**（任务书猜的 `Set.subset_antisymm` 确不存在，但这三个都在）；`Set.Subset.refl`/`Set.Subset.trans`；`Set.empty_subset`；`Set.subset_univ`；`Set.subset_iff_notMem` | `Basic.lean:249,276,441,574,285` |
| 10–14 | `Set.mem_univ`；`Set.mem_empty_iff_false`；**`Set.notMem_empty`**（✅ 存在——大写 M，我先前用小写查找以致误判为"未能定位"）；`Set.eq_empty_iff_forall_notMem`；`Set.subset_empty_iff`；`Set.eq_univ_iff_forall`；`Set.mem_sep` | `Basic.lean:426,448,445,582` |
| 15–19 | `Set.mem_union`；`Set.union_subset_iff`；`subset_union_left/right`；`Set.union_nonempty`；`Set.union_comm`/`inter_comm` | `Basic.lean:380` 等 |
| 20–24 | `Set.mem_inter_iff`；`inter_subset_left/right`；`Set.subset_inter` 与 `Set.subset_inter_iff`；`Set.inter_nonempty` | `Basic.lean:389` 等 |
| 25–27 | `Set.mem_compl_iff := Iff.rfl`；`Set.mem_sdiff := Iff.rfl`；`Set.sdiff_eq`（别名 `Set.diff_eq`）；`compl_compl : xᶜᶜ = x`（**根命名空间，`[BooleanAlgebra α]`，对 `Set α` 适用；没有 `Set.compl_compl`**）；`Set.disjoint_iff_inter_eq_empty`、`Set.disjoint_left` | `Operations.lean:117,124`；`Lattice/Disjoint.lean` |
| 28–39 | `Set.mem_image (f) (s) (y) : y ∈ f '' s ↔ ∃ (x : α), x ∈ s ∧ f x = y`；`mem_image_of_mem`；`mem_preimage := Iff.rfl`；`image_subset_iff : f '' s ⊆ t ↔ s ⊆ f ⁻¹' t`（docs 注 "image and preimage are a Galois connection"）；`subset_image_iff : t ⊆ f '' s ↔ ∃ u, u ⊆ s ∧ f '' u = t`；`image_preimage_subset` 与对偶 `subset_preimage_image`；`preimage_image_eq (h : Injective f)` 与对偶 `image_preimage_eq (h : Surjective f)`；`image_union`；`preimage_inter`；`preimage_union`；`preimage_compl`；`image_mono`/`preimage_mono`/`image_id`/`image_comp`/`image_eq_empty`/`mem_range`/`range_eq_empty_iff`/`forall_mem_image`/`exists_mem_image` | `Operations.lean:147,151,141`；`Image.lean`（全部）；⚠️ `Set.image_inter` 需 `Injective` 假设，在 `Function.lean` |
| 40–46 | `Set.mem_powerset_iff (x s) : x ∈ 𝒫 s ↔ x ⊆ s`；`powerset_mono`/`powerset_univ`/`powerset_empty`；**`Set.mem_iUnion`/`Set.mem_iInter` 在 `Order/SetNotation.lean`**、而 `Set.mem_iUnion_of_mem`/`mem_iInter_of_mem`/`iUnion_subset_iff`/`subset_iInter_iff`/`subset_iUnion`/`iInter_subset` 在 `Lattice/Indexed.lean:39,46,136,143,149,152` | `Basic.lean`；`Order/SetNotation.lean`；`Lattice/Indexed.lean` |
| 47–50 | `Set.mem_prod`；`Set.mem_singleton_iff`；`Set.mem_insert_iff`；`Set.eq_singleton_iff_unique_mem` | `Operations.lean:236`；`Insert.lean` |
| 51–53（补充） | `Set.Subsingleton`；`Set.Finite`/`Set.finite_def`/`Set.Finite.toFinset`/`Set.ncard`/`Nat.card`；`Set.EqOn`（**无 `EqOn.refl`**）、`Set.MapsTo`、`Set.InjOn`、`Set.SurjOn`（**无 `SurjOn.image_eq`**）、`Set.BijOn`/`Set.BijOn.equiv` | `Subsingleton.lean`；`Basic/Finite/Defs.lean`；`Function.lean` |

**1.5 函数在集合上（第 2 阶段）—— 逐字定义可直接当我们教程的"目标形态"**
✅ `Set.MapsTo {α β} (f : α → β) (s : Set α) (t : Set β) : Prop := ∀ ⦃x⦄, x ∈ s → f x ∈ t`；`Set.InjOn (f) (s) : Prop := ∀ ⦃x₁⦄, x₁ ∈ s → ∀ ⦃x₂⦄, x₂ ∈ s → f x₁ = f x₂ → x₁ = x₂`；`Set.SurjOn (f) (s) (t) : Prop := (t ⊆ f '' s)`；`Set.BijOn (f) (s) (t) : Prop := (Set.MapsTo f s t ∧ Set.InjOn f s ∧ Set.SurjOn f s t)`；`Set.EqOn (f₁ f₂) (s) : Prop := ∀ ⦃x⦄, x ∈ s → f₁ x = f₂ x`；`Set.Subsingleton (s) : Prop := ∀ ⦃x⦄, x ∈ s → ∀ ⦃y⦄, y ∈ s → x = y`（全在 `Data/Set/Function.lean` / `Subsingleton.lean`；`InjOn`/`BijOn` 亦见 `Data/Set/Card.lean`）。
`EqOn` API：`EqOn.eq_of_mem`、`eqOn_empty`、`eqOn_singleton`、`eqOn_univ`、`EqOn.symm/trans/image_eq/mono`、`eqOn_union`（⚠️ 注意命名：小写的 `Set.eqOn_refl` **存在**，但点号形式 `Set.EqOn.refl` **不存在**——Loogle `unknown identifier`）。`MapsTo` API：`mapsTo_iff_image_subset`、`mapsTo_iff_subset_preimage`、`MapsTo.comp/mono/subset_preimage/image_subset/restrict`。⚠️ `Set.SurjOn.image_eq` **不存在**（但有 `Set.BijOn.image_eq`、`Set.BijOn.equiv : ↑s ≃ ↑t`、`Set.InjOn.image_inter`）。

**1.6 有限性与基数（第 3 阶段，⚠️ 命名与位置都反直觉）** **`Set.Finite` 不在 `Data/Set/Finite/Basic.lean`，而在 `Mathlib/Basic/Finite/Defs.lean:185`**：`protected def Finite (s : Set α) : Prop := Finite s` —— **是一个 `Prop`（protected def），不是 structure**；`Finite` 本身是 `class inductive Finite (α : Sort*) : Prop | intro {n : ℕ} : α ≃ Fin n → Finite _`（同文件 `:101`）。docstring 原文 "A finite set is defined to be a set whose coercion to a type has a `Finite` instance."
`Set.Finite.toFinset`（`protected noncomputable def`，`Data/Set/Finite/Basic.lean:75`，需要 `h : s.Finite`）与可计算的 `Set.toFinset`（需要 `[Fintype ↑s]`，模块 `Mathlib.Data.Fintype.Sets`）并存，由 `Set.Finite.toFinset_eq_toFinset` 打通。**⚠️ `Set.Finite.card` 不存在**（Loogle `unknown identifier`）→ 用 `s.ncard` 或 `h.toFinset.card`。
**四种"基数"并存，且 `#` 有两个不同 locale**：`Finset.card`（`#s`，**`open scoped Finset`**）、`Cardinal.mk`（`#α`，**`open scoped Cardinal`**）、`Set.ncard`（**无记号**）、`Set.encard : ℕ∞`、`Nat.card`（`SetTheory/Cardinal/Finite`）、`ENat.card`。桥接引理：`Set.ncard_univ : univ.ncard = Nat.card α`、`Nat.card_coe_set_eq : Nat.card ↑s = s.ncard`、`Set.ncard_le_card`、`Set.ncard_def : s.ncard = s.encard.toNat`。`Set.ncard (s) : ℕ := ENat.toNat s.encard`（`Data/Set/Card.lean:613`，**无限时是 junk value 0**）、`Set.encard`（`:67`）、`ncard_le_ncard`（`:655`）、`ncard_prod`（`:702`）、`ncard_powerset`（`:706`）。
`Set.Subsingleton`（`Data/Set/Subsingleton.lean`：`Subsingleton.inter_singleton`、`eq_empty_or_singleton_of_subsingleton`、`Subsingleton.eq_singleton_of_mem`）。`Disjoint`：**是 `Mathlib/Order/Disjoint.lean:48` 的通用定义** `def Disjoint (a b : α) : Prop := ∀ ⦃x⦄, x ≤ a → x ≤ b → x ≤ ⊥`（`[PartialOrder α] [OrderBot α]`，**不是 Set 专用**），集合版引理 `Set.disjoint_iff_inter_eq_empty : Disjoint s t ↔ s ∩ t = ∅` 与 `Set.not_disjoint_iff`、`Set.disjoint_left` 在 `Data/Set/Lattice/Disjoint.lean`。
**取舍建议**：1.4 的 50 条是"零依赖集合论"子集；1.5 需要函数概念；1.6 需要 `Finite`/`Cardinal` 这类我们没有的装置 → 放最后或不做（替代见 §7 #15）。

## §2 Lean 入门课程里的集合论（逐个）

**2.1 Mathematics in Lean (MIL) — ✅** 书 https://leanprover-community.github.io/mathematics_in_lean/ ；集合章 = **第 4 章 "Sets and Functions"**（4.1 Sets / 4.2 Functions / 4.3 The Schröder-Bernstein Theorem，https://leanprover-community.github.io/mathematics_in_lean/C04_Sets_and_Functions.html ）。练习文件**按节拆分**：`MIL/C04_Sets_and_Functions/S01_Sets.lean`（5090 B，12 处 `sorry`）、`S02_Functions.lean`（3902 B，约 28 处）、`S03_The_Schroeder_Bernstein_Theorem.lean`（2560 B），`solutions/` 三份解答**确实存在**。**⚠️** `MIL/C06_SetsAndFunctions.lean`、`MIL/C04_Sets_and_Functions.lean` 在 master/main 上均 404。
覆盖：`Set α = α → Prop`、`⊆ ∩ ∪ ∅ univ \`、`ext`、`Subset.antisymm`、有界量词 `∀ x ∈ s`/`∃ x ∈ s`、`⋃₀`/`⋂₀`、`f '' s`/`f ⁻¹' s` 分配律、`Injective/Surjective/InjOn`、`Classical.choose` 造 `inverse`、**Cantor 定理** `∀ f : α → Set α, ¬ Surjective f`、`InjOn log {x | x > 0}`，收尾用 `sbAux`/`sbSet`/`sbFun` 证 `schroeder_bernstein`。
练习原文：`example : s ∩ t ∪ s ∩ u ⊆ s ∩ (t ∪ u) := by sorry`；`example : s ∩ t = t ∩ s := Subset.antisymm sorry sorry`；`example : (s ∪ ⋂ i, A i) = ⋂ i, A i ∪ s := by sorry`（提示：一个方向需经典逻辑）；`example : f '' s ⊆ v ↔ s ⊆ f ⁻¹' v := by sorry`；`example (h : Injective f) : f '' s ∩ f '' t ⊆ f '' (s ∩ t) := by sorry`。
**⚠️ 完整性缺陷**：正文内部本身有洞——S02 的 Cantor 证明里 `have h₂ : j ∈ S  sorry` 带 `-- COMMENTS: TODO: improve this`；S03 的 `sb_right_inv`(3)、`sb_injective`(3)、`sb_surjective`(1) 共 7 处 `sorry`。MIL README 自述 "still a work in progress"。
**能偷**：4.1 的"集合等式三件套"顺序（`ext` / `Subset.antisymm` / 展定义）；4.2 的 image/preimage↔单射满射题库；4.3 的 `sbAux → sbSet → sbFun` 三段式。**不能偷**：依赖 `Mathlib.Data.Set.Lattice`、`Set.Function`、`Analysis.SpecialFunctions.Log.Basic`，且重 `simp`/`rw`/`aesop` → 只能当题面来源。

**2.2 Theorem Proving in Lean 4 (TPIL) — ⚠️ 关键前提被证伪** https://leanprover.github.io/theorem_proving_in_lean4/ （针对 Lean 4.33.0）。"不用 Mathlib"**成立**（`book/lakefile.toml` 的 `[[require]]` 只有 `verso`），但 **TPIL 根本没有集合章**：12 章（1 Introduction / 2 Dependent Type Theory / 3 Propositions and Proofs / 4 Quantifiers and Equality / 5 Tactics / 6 Interacting with Lean / 7 Inductive Types / 8 Induction and Recursion / 9 Structures and Records / 10 Type Classes / 11 The Conversion Tactic Mode / 12 Axioms and Computation）无一章叫 Sets；Lean 3 旧版 11 章同样没有。"Sets and Functions" **是 MIL 第 4 章的标题**。**能偷**：第 7/8 章是 `inductive ... ctor ... end` 教学法来源；第 9 章 Structures 我们用不了。

**2.3 The Mechanics of Proof（Heather Macbeth）— ✅** https://hrmacbeth.github.io/math2001/ ，代码 https://github.com/hrmacbeth/math2001 。**⚠️ 纠正**：第 2、4 章不是集合（是 Proofs with structure I/II）；**集合 = 第 9 章 Sets**（9.1 Introduction / 9.2 Set operations / 9.3 The type of sets），**函数 = 第 8 章 Functions**（8.1 Injectivity and surjectivity / 8.2 Bijectivity / 8.3 Composition / 8.4 Product types），另有第 10 章 Relations。仓库 `Math2001/09_Sets/{01_Sets,02_Set_Operations,03_Powerset}.lean` 已取证。9.1 给 `def Set.Subset (U V : Set α) : Prop := ∀ ⦃x⦄, x ∈ U → x ∈ V`——**与我们最接近的定义方式之一**。
**最大特色（强烈建议偷）：成对"证明或证伪"**：
```lean
example : 4 ∈ {a : ℚ | a < 3} := by sorry      example : 4 ∉ {a : ℚ | a < 3} := by sorry
example : {a : ℕ | 20 ∣ a} ⊆ {x : ℕ | 5 ∣ x} := by sorry
example : {a : ℕ | 20 ∣ a} ⊈ {x : ℕ | 5 ∣ x} := by sorry
```
外加正文 Example 区块的**双语调**（散文↔Lean）与定制极简方言（`numbers`/`extra`/`cancel`/`rel`/`exhaust`）配 `dsimp [Set.subset_def]`、`push_neg`。前言原文："Over two hundred problems appear with solutions as examples in the text, and several hundred more problems appear without solution as exercises for the reader." → **正文例题有解、练习无解**。

**2.4 Logic and Proof — ✅ 但作者与版本需更正** 现行 URL https://leanprover-community.github.io/logic_and_proof/ （页脚 "Logic and Proof 3.18.4"）；旧的 `leanprover.github.io/logic_and_proof/` 已 404/跳转。**作者不是** "Avigad, de Moura, Kong, Ullrich"，页脚是 **Jeremy Avigad, Joseph Hua, Robert Y. Lewis, Floris van Doorn**；这是 **Lean 3 时代**的书（语法 `cases hx with | inl h =>`；`courses.yaml` 把用它当教材的 VU Amsterdam 课标为 `lean_version: 3`），但 "try it" 片段已改成 Lean 4 + `import Mathlib.Data.Set.Basic`。
集合章：**11. Sets**（11.1 Elementary Set Theory / 11.2 Calculations with Sets / 11.3 Indexed Families of Sets / 11.4 Cartesian Product and Power Set / 11.5 Exercises，13 道散文题）与 **12. Sets in Lean**（12.1 Basics / 12.2 Some Identities / 12.3 Indexed Families / 12.4 Power Sets / 12.5 Exercises）；函数在 **15. Functions** / **16. Functions in Lean**。第 11 章纯散文（自然演绎影子 + 布尔代数恒等式表 + Kuratowski 有序对定理）；12.1 有一句有用的话："Basic set-theoretic notions like these are **defined in Lean's core library**, but additional theorems and notation are available in an auxiliary library that we have loaded with the command `import Mathlib.Data.Set.Basic`."
**能偷**：11→12 的"先散文后形式"分层；11.2 的恒等式表（`(A ∩ B̄) ∪ B = A ∪ B`、`(A \ B) ∪ (B \ A) = (A ∪ B) \ (A ∩ B)`）可直接变成等式练习链。

**2.5 Formalising Mathematics（Buzzard / Mehta）— ✅ 2024 版存在，2025 版不存在** 2024 笔记 https://www.ma.imperial.ac.uk/~buzzard/xena/formalising-mathematics-2024/ ，仓库 https://github.com/ImperialCollegeLondon/formalising-mathematics-2024 （description 原文 "Ran between January and March 2024"）。集合讲义真实路径 **`FormalisingMathematics2024/Section04sets/Sheet1–6.lean`**，函数 **`Section03functions/Sheet1–3.lean`**，另有 `Section08finiteness`、`Section09bijectionsAndIsomorphisms`、`Solutions/`。`Sheet1.lean` 开篇原文 `# Sets in Lean, sheet 1 : ∪ ∩ ⊆ and all that`，先以 `rfl` 证 `subset_def`/`mem_union_iff`/`mem_inter_iff`（**定义相等**），再出 8 道 `sorry`：`A ⊆ A`；`A ⊆ B → B ⊆ C → A ⊆ C`；`A ⊆ A ∪ B`；`A ∩ B ⊆ A`；`A ⊆ B → A ⊆ C → A ⊆ B ∩ C`；`B ⊆ A → C ⊆ A → B ∪ C ⊆ A`；`A ⊆ B → C ⊆ D → A ∪ C ⊆ B ∪ D`；`A ⊆ B → C ⊆ D → A ∩ C ⊆ B ∩ D`。**这 8 题几乎可以直接进我们的教程。**
**2025 版不存在**：`~/xena/formalising-mathematics-2025/` 返回 HTTP 300（服务器只列 2022/2023/2024）；`ImperialCollegeLondon/formalising-mathematics-2025` 404。当前版是 Bhavik Mehta 的 https://github.com/b-mehta/formalising-mathematics-notes （笔记 https://b-mehta.github.io/formalising-mathematics-notes/ ，© 2025），代码目录已改名 `FormalisingMathematics2026/`，同样有 `Section04sets/Sheet1–6.lean`。

**2.6 最贴近我们路线的宝藏：Université Gustave Eiffel《Logique et preuve assistée》— ✅** 仓库 https://github.com/niotie/logique-preuve-assistee （Lean 4，**无 Mathlib**：`lakefile.lean` 222 B、`lake-manifest.json` 121 B）。courses.yaml 原文："worksheets on propositional and predicate logic, **sets and functions**, and natural numbers… in **"plain vanilla" Lean (without the Mathlib)**"。真实文件 `LPA/TP3EnsemblesFonctions.lean` import 五个模块 `LPA/TP3EnsemblesFonctions/{1SetDefinitions,2SetProperties,3FunctionsDefinitions,4FunctionProperties,5InjectivitySurjectivity}.lean`。
`1SetDefinitions.lean` **从零搭集合**：`def Set (α : Type u) := α → Prop`、`def Mem`、`Membership` 实例、`def Subset`、`@[ext] theorem ext … := by funext x; apply propext; exact h x`、`∅`/`univ`/`∪`/`∩`/`ᶜ`/`\`/`𝒫`/`{a,b}` 全套 + `<op>_def` 引理。`5InjectivitySurjectivity.lean` 约 30 道全 `sorry`，含 `inj_iff_eq_preimage_image`、`inj_iff_inter_image_sub_image_inter`、`surj_iff_exists_right_inverse`（注释提示 `Classical.choose`），并有"四命题只有两个为真、给假的找反例"设计。**公开材料里唯一"从零定义 Set + 不用 Mathlib + 纯 Lean 4 + 带 sorry 习题"的一份。**

**2.7 其它已取证课程** ①Universitat de València《An Introduction to Lean 4》https://www.uv.es/coslloen/Lean4/ （15 章纯 core；**第 5 章 Functions** 有 injective/surjective/monomorphism/isomorphism 全套 + 全 `sorry` 练习，解答 https://github.com/encosllo/IntroToLean4/ ；**无集合章**，最接近的是第 8 章 Subtypes、第 9 章 Relations）。②University of Dayton Math 342 "Set theory and Logic"（Jun Li, Spring 2023, **Lean 3**）https://lijungeometry.github.io/342.html ，课表含 "2 Set Operations"、"5 Functions and Relations"、"6 Cantor's Theorem"、"15 Lean"。③Stockholm《Logic II: Computability, Set Theory, and Model Theory》（2025）https://sinhp.github.io/teaching/2025-logic2-stockholm/ ——**陷阱**：原文 "Lean is used as a digital diary for the **first part**（computability）"，**集合论部分没用 Lean**。④Mathlib 教学页 https://leanprover-community.github.io/theories/sets.html （"Maths in Lean: Sets and set-like objects"：List/Multiset/Finset/`Set α = α → Prop`+subtype/Fintype/Set.Finite/Cardinals，**无习题**）是唯一专门讲集合类的教学页；https://leanprover-community.github.io/undergrad.html **按法国大纲组织，没有"集合与函数"专题**。
**未能验证（没有编造）**：Imperial 课程号 "MATH40001"；Cambridge；Waterloo；Chapman/Kevin Sullivan；CMU 的 "15-xxx"。

## §3 游戏化先例

**3.1 结论先给：「集合论游戏」存在，作者是 Dan Velleman（《How to Prove It》作者）。** `https://raw.githubusercontent.com/leanprover-community/lean4game/main/README.md` 的游戏表原文列出（逐字）：**Set Theory Game — https://github.com/djvelleman/stg4 — Dan Velleman**；Knights and Knaves（jadabouhawili/knightsandknaves-lean4game）；Linear Algebra Game（zrtmrh/linearalgebragame）；Logic Game（trequetrum/lean4game-logic）；NNG（leanprover-community/nng4）；Real Analysis Game（alexkontorovich/realanalysisgame）；Reintroduction to Proofs（emilyriehl/reintroductiontoproofs）；Robo / Scribble（hhu-adam/robo）。旁证：https://leanprover-community.github.io/learn.html 原文 "The Lean Game Server hosts various learning games **including Set Theory, Logic, and Robo**"。

**3.2 STG4 结构（我逐文件抓取，`cdn.jsdelivr.net/gh/djvelleman/stg4@main/…`）**
`Game.lean` 原文给出 **8 个 World 与依赖**：`Dependency Intersection → Union`、`Dependency FamInter → FamUnion`、`Dependency Combination → FamCombo`。`Info` 区块原文：版本 **4.4**；**"The game stores your progress in your local browser storage."**；Creator **Daniel J. Velleman**；based on NNG by Kevin Buzzard；Game Engine: Alexander Bentkamp, Jon Eugster, Patrick Massot；Spanish Translation: Miguel Marco；`Languages "en" "es"`；`CaptionLong`："In this game you will learn the basics of theorem proving in Lean by proving theorems about unions, intersections, and complements of sets."
**51 关（6+5+8+6+5+6+7+8），关卡文件名全部实测**：

| World | 数 | 关卡文件名 |
|---|---|---|
| Subset World | 6 | L01exact, L02subhyp, L03have, L04imp, L05subref, L06subtrans |
| Complement World | 5 | L01contra, L02compdef, L03compsub, L04compcomp, L05compsubiff |
| Intersection World | 8 | L01and, L02elt_inter_elt_right, L03inter_sub_left, L04proveand, L05subint, L06inter_sub_swap, L07inter_comm, L08inter_assoc |
| Union World | 6 | L01or, L02subunion, L03cases, L04union_sub_swap, L05union_comm, L06union_assoc |
| Combination World | 5 | L01compunion, L02compint, L03inter_distrib_union, L04union_distrib_inter, L05union_sub_inter_sub |
| Family Intersection World | 6 | L01intersub, L02intersubinter, L03interpair, L04interunion, L05subinter, L06eltwiseunion |
| Family Union World | 7 | L01proveexists, L02subunion, L03unionsubunion, L04unionpair, L05unionunion, L06unionsub, L07eltwiseinter |
| Family Combination World | 8 | L01compunion, L02compinter, L03commonelt, L04threefam, L05unionintcompunion, L06unionintunion, L07unionintcompint, L08singleton |

**关卡语法骨架（`Subset/L01exact.lean` 逐字）**：`import Game.Metadata` / `open Set` / `namespace STG4` / `variable {U : Type}` / `World "Subset"` / `Level 1` / `Title "The exact tactic"` / `Introduction "…# Read this first…"` / `TacticDoc exact` / `NewTactic exact` / `DefinitionDoc elt as "∈"` / `NewDefinition elt` / `Statement (x : U) (A : Set U) (h : x ∈ A) : x ∈ A := by Hint "…type `exact h`…" ; exact h` / `Conclusion "…"`。→ **`NewTactic`/`NewDefinition`/`NewLemma` 实现"逐步解锁语法"**，`TacticDoc`/`DefinitionDoc` 同时是说明书。
**三种 Hint（逐字取证）**：①`Hint "…"` 正常提示；②`Hint (hidden := true) "…"` **默认折叠**（`Subset/L03have` 原文 "As we saw in the last level, `h2 {h4}` is now a proof of the goal, so `exact h2 {h4}` will close the goal."）；③`Hint (strict := true) "…"` **必读才能继续**（`FamUnion/L01proveexists` 原文 "Your goal says that there is a set that is a subset of `A`. The theorem `Subset.refl` suggests such a set."）。④**`Branch` 故意留的死路**，`FamUnion/L01proveexists` 逐字：
```lean
Branch
  use ∅
  Hint "Although `∅` is a reasonable choice for a set that is a subset of `A`, it is difficult
  to complete the proof with this choice using only methods developed so far in this game.
  Go back and try a different choice."
```
**提示会插值当前状态**：`Hint "Notice that `{h4} : x ∈ B` has been added…"`（`{h4}` 显示玩家当前的假设名）。
**难度曲线**：`Combo.lean` 的 world 介绍原文 "In this world you'll prove theorems combining complements, intersections, and unions. **For the most part, we'll leave you on your own** to figure out these proofs." → 提示密度随 world 递减（`Subset/L05subref` 用 4 条提示串成一串，后期只给一句方向）。
**题型样本（逐字）**：`FamCombo/L08singleton`：`Statement (A : Set U) (h1 : ∀ F, (⋃₀ F = A → A ∈ F)) : ∃ x, A = {x}`；`Combo/L05union_sub_inter_sub`：`Statement (A B C : Set U) (h1 : A ∪ C ⊆ B ∪ C) (h2 : A ∩ C ⊆ B ∩ C) : A ⊆ B`；`Union/L03cases`：`Statement (A B C : Set U) (h1 : A ⊆ C) (h2 : B ⊆ C) : A ∪ B ⊆ C`。即**只用 `⊆ ∪ ∩ ᶜ ⋃₀ ⋂₀ {x}`，不碰基数、不碰像**。
**进度**：localStorage（`Info` 原文），无服务器账号。`GupilChhabs/lean-set-theory-game-solutions`（0★）是第三方解答——**内容未核实**。

**3.3 其它游戏（**只有游戏表与仓库 URL 已验证；各游戏关卡数/hint 机制未核实**）** NNG4 https://github.com/leanprover-community/nng4 （Lean 3 旧版 https://www.ma.imperial.ac.uk/~buzzard/xena/natural_number_game/ ）；Logic Game https://github.com/trequetrum/lean4game-logic ；Robo https://github.com/hhu-adam/robo ；Real Analysis Game https://github.com/alexkontorovich/realanalysisgame （在线 https://adam.math.hhu.de/#/g/AlexKontorovich/RealAnalysisGame ）；Reintroduction to Proofs https://github.com/emilyriehl/reintroductiontoproofs （在线 …/#/g/emilyriehl/ReintroductionToProofs ）。**⚠️ 服务端无法直接枚举**：`https://adam.math.hhu.de/` 只返回 SPA 壳 "Lean Game Server"，`GET/POST /api/games`、`/api/level/...`、`/api/game/...` 全部 404（实测）→ 可靠来源是 lean4game README 的游戏表而非服务器端点。**未能验证**：NNG4/Logic Game/Robo 的 world 数与 hint 写法；EuroProofNet 的游戏化产出（**未核实，勿引用**）。

## §4 其他证明助手：有教学层，还是只有库？

**判定口径**：**教学层** = 面向初学者的教程/教材/习题集；**库** = 供其他证明复用的形式化代码 + API 文档。四种判定：**有集合论教学层** / **有通用教学层但集合论只有库** / **只有库** / **无集合论内容**。
**规模对照（✅ https://www.cs.ru.nl/~freek/100/ ，末行 "last modification 2026-09-17"）**：HOL Light 95 ｜ Isabelle 95 ｜ Lean 83 ｜ Rocq 80 ｜ Metamath 74 ｜ Mizar 71 ｜ nqthm/ACL2 48 ｜ ProofPower 43 ｜ PVS 26 ｜ Imandra 20 ｜ Megalodon 12 ｜ Naproche 10 ｜ NuPRL 8。⚠️ 三处数字互不一致：Lean 侧页写 **85**、Metamath 自己的 `mm_100.html` 写 **75**（并把 Lean 记作 82）→ 全部并列，不判断谁对。

| 系统 | 集合表示 | 库（✅ 已验证） | 教学层 | **判定** |
|---|---|---|---|---|
| **Isabelle/ZF** | `typedecl i` + `mem :: [i,i] ⇒ o`，公理化 | `src/ZF/` 共 **38 个 `.thy`**（`AC Arith Bin Bool Cardinal CardinalArith Cardinal_AC Datatype Epsilon EquivClass Finite Fixedpt Inductive InfDatatype Int IntDiv List Nat OrdQuant Order OrderArith OrderType Ordinal Perm QPair QUniv Sum Trancl Univ WF ZF ZFC ZF_Base Zorn equalities func pair upair`）⚠️ 任务书猜的 `union.thy`/`power.thy`/小写 `ordinal.thy` **不存在**；`ROOT` 会话：`ZF ZFC ZF-AC ZF-Coind ZF-Constructible ZF-IMP ZF-Induct ZF-Resid ZF-UNITY ZF-ex` | ZF 侧**只有 Paulson 论文**：`set-I.pdf` *Set Theory for Verification: I. From Foundations to Functions*（40 页，含 "6 From Replacement to Separation"、"9 Ramsey's Theorem in ZF"）、`set-II.pdf`、`AC.pdf`（证明 **7 种良序定理表述等价 + 20 种 AC 表述**）、`constructible-theory.pdf`（254 页）、`UCAM-CL-TR-551.pdf`。**真正给初学者的集合论教学章在 Isabelle/HOL 的《Tutorial》里**：`src/Doc/Tutorial/document/sets.tex` 的 `\chapter{Sets, Functions and Relations}`（Sets / Functions / Relations / Well-Founded Relations and Induction / Fixed Point Operators） | **有教学层**（ZF 侧=论文级；HOL 侧=教程章） |
| **Metamath `set.mm`** | 纯 ZFC 一阶语言 + 类理论 | 公理逐字：`ax-ext`、`ax-rep`、`ax-pow`、`ax-un`、`ax-reg`、`ax-inf`、`ax-ac`、`ax-groth`；**separation 是派生的冗余公理**——`axsep` 页原文 "Axiom scheme of separation ax-sep 5251 derived from the axiom scheme of replacement ax-rep 5232 … (New usage is discouraged.)"；规模：`mmset.html` 原文 "over 26,000 completely worked out proofs in its main sections (and over 41,000 counting mathboxes)" | **有通用教学层**：《Metamath: A Computer Language for Mathematical Proofs》（247 页 PDF，第 2 章 "Your First Proof"、第 3 章 "Abstract Mathematics Revealed"）、MPE 首页自带 "How Metamath Proofs Work"、`lamp-guide.metamath.org`（交互式指南）、`mmsolitaire`（Java applet，页面自注现代浏览器已不能运行=历史装置）。入门样例：`unss1`（`⊢ (A ⊆ B → (A ∪ C) ⊆ (B ∪ C))`）**完整证明 6 步**；`eqid`（`⊢ A = A`）**2 步** | **有教学层**（教"读/写证明"+公理，不教集合论概念） |
| **Mizar MML** | 无类型 ZF（`hidden.miz` 原始概念 `mode set -> object; pred x in X;`，`tarski_0.miz` 五公理，`tarski.miz` 含 `scheme Replacement`，`tarski_a.miz` Tarski A） | `mml.txt` 目录：**MML 5.94.1493**；任务书列的 **14 个文章名全部存在**（`XBOOLE_0 XBOOLE_1 ZFMISC_1 SUBSET_1 SETFAM_1 ORDINAL1 CARD_1 FUNCT_1 FUNCT_2 RELAT_1 TARSKI ENUMSET1 BOOLE PARTFUN1`；注意 `XBOOLE_0/1` 属 §IV EMM、`TARSKI` 属 §II Addenda）。规模（对 1500 个 `.miz` 逐词计数并自校验）：**3,706,915 行 / 75,158 `theorem` / 14,801 `definition` / 912 `scheme` / 15,647 `registration`** | **薄且旧的通用教学层**：Wiedijk *Writing a Mizar article in nine easy steps*（54 页，约 15 道编号习题，前置要求写明需要 "some basic set theory"）、1990/1993 的两本 PC Mizar 入门、CICM 2016 手把手教程（含骨架+解答，但题目是 Liouville 数）。反面证据：Nutshell 自述 "This paper is not a step by step tutorial…, but rather a reference manual" | **库 + 薄教学层（非集合论专属）** |
| **Coq/Rocq** | `Definition Ensemble := U -> Prop.` + 唯一公理 `Extensionality_Ensembles` | `Coq.Sets.*` 实际 **22 个模块**（`Classical_sets Constructive_sets Cpo Ensembles Finite_sets Finite_sets_facts Image Infinite_sets Integers Multiset Partial_Order Permut Powerset Powerset_Classical_facts Powerset_facts Relations_1 Relations_1_facts Relations_2 Relations_2_facts Relations_3 Relations_3_facts Uniset`）。⚠️ **`Coq.Sets.Ordinals`/`Cardinals`/`ZF`、`Coq.ZF` 全部不存在（404）**。stdpp：`stdpp.sets` 原文 "implements some tactics to automatically solve goals involving sets"，核心是 `ElemOf/Equiv/SubsetEq/Disjoint` **类型类**；`gset` 是 `stdpp.gmap` 里的一个 `Definition`（`stdpp.gset.html` 本身 404）。MathComp 2.6.0 索引里有 `mathcomp.boot.finset`，**`fset` 完全不出现** | SF（`softwarefoundations…/lf-current/toc.html`：`set theory` **0 命中**）与 Coq'Art（"solution of 170 over 200 exercises"）都是**通用**教材，**不教集合论** | **只有库** |
| **Agda** | ⚠️ 术语碰撞，官方文档原文 "The fundamental sort in Agda is named **Set** and it denotes the universe of small types." | agda-stdlib（1,114 模块）`zf/zfc/settheory/ordinal/cardinal/cantor` **全部 0 命中**，只有 `Relation.Binary.*`/`Relation.Unary.*`/setoid（关系代数）。**真正的 ZF 在 Cubical Agda**：`Cubical/HITs/CumulativeHierarchy/{Base,Constructions,Properties}`、`Cubical/HITs/Replacement/*`、`Cubical/Data/{Ordinal,Cardinal}/*`；`CumulativeHierarchy/Base.agda` 文件头原文 "This file models **\"ZF - powerset\"** in cubical agda, via a cumulative hierarchy, in the sense given in the HoTT book §10.5"（`Constructions.agda` 实现 ∅/配对/单点/并/ω/replacement/separation——**无幂集、无选择**） | 无集合论教学（stdlib 的 `doc/README/`、cubical 的 "Learning materials" 都不是集合论） | **只有库（Cubical）/ 无内容（stdlib）** |
| **Idris** | — | Idris 2 的 `libs/` 共 305 模块，`ordinal/cardinal/zf/zfc/cantor/transfinite` **全 0 命中**；唯一 `Set` 命中是 `Data.SortedSet`（数据结构）。Idris 1 同样 0 | 通用教程（*A Crash Course in Idris 2*、*Theorem Proving*）**不教集合论**，也没有对象可教 | **无集合论内容** |
| **ACL2** | **集合 = 有序表** | `books/std/osets/top.lisp` 原文 "A finite set theory implementation for ACL2 based on **fully ordered lists**. … set equality is just `equal`, and set operations like union, intersect … have O(n) implementations."；另有 `books/data-structures/set-{defuns,defthms,theory}.lisp`（无序表版）与 `books/finite-set-theory/`（J Moore，用 urelement）。⚠️ **`std/sets` 不存在（404）**；`defset` 在已核位置不存在 | 首页 "Tutorials" 整行被 **HTML 注释掉**，注释原文 "…the tutorials, which are not elementary enough. I think we should write some appropriate tutorials. Meanwhile, this entry is left blank."；现存教程全是 ACL2 通用 | **只有库** |
| **HOL Light** | 谓词即集合 | `sets.ml`（5,217 行）文件头原文 "**Very basic set theory (using predicates as sets).**"；`let IN = new_definition … x IN P <=> P x` | **有集合论教学层**：官方 Tutorial（230 页）**第 14 章 "Sets and functions"**（14.1 Choice and the select operator；14.2 Function calculus；14.3 Some cardinal arithmetic），正文原文 "HOL's set-theoretic notation is defined for predicates…"、"There is a simple automated rule `SET_RULE`, and a corresponding tactic `SET_TAC`" | **有集合论教学层** |
| **HOL4** | 特征函数 `α → bool` | `src/pred_set/src/pred_setScript.sml`（314,729 B，文件头 "a simple theory of predicates-as-sets"）；手册原文 "Sets are represented by functions of the type `α → bool`…"，给出 `IN_DEF/EXTENSION/EMPTY_DEF/UNION_DEF/INTER_DEF/POW_DEF/FINITE_DEF/CARD_DEF/SET_TAC`（自述 ported from HOL Light） | Tutorial 共 10 章**无集合论章**（`pred_set` 命中 0），只有手册散文 | **库 + 手册（无教程）** |
| **Isabelle/HOL** | 类型化谓词集合 | `src/HOL/Set.thy`（2,046 行）`section ‹Set theory for higher-order logic›` / `typedecl 'a set` + `Collect`/`member` 公理化；《Logics》LaTeX 源原文 "these sets are distinct from those of ZF set theory, and behave more like ZF classes" | **有集合论教学层**：见上，Tutorial `sets.tex` 专章 | **有集合论教学层** |
| **Lean 3 mathlib** | 依模块而异 | ⚠️ **不存在 `lean-3` 分支**（`…/mathlib/lean-3/README.md` 404）；Lean 3 mathlib 在 **`master`**。`src/set_theory/` 实测：`cardinal/{basic,cofinality,continuum,divisibility,finite,ordinal,schroeder_bernstein}`、`game/{basic,birthday,domineering,impartial,nim,ordinal,pgame,short,state}`、`lists`、`ordinal/{arithmetic,basic,cantor_normal_form,exponential,fixed_point,natural_ops,notation,principal,topology}`、`surreal/{basic,dyadic}`、`zfc/{basic,ordinal}` | 只验证到 API 文档（`mathlib_docs`），**未验证到针对 `set_theory` 的教程/习题集** | **只有库** |

**一句话结论（对我们是好消息）**：**没有哪个主流证明助手为"集合论"提供独立的教学层**。教学层要么是"证明助手通用"（Metamath 书、Mizar 手册、ACL2 教程、SF/Coq'Art），要么是"类型化集合论的教程章"（HOL Light 第 14 章、Isabelle/HOL Tutorial 的 sets 章）。真正把 ZF 做深的（Isabelle/ZF、Metamath `set.mm`、Mizar MML、mathlib3 `set_theory`、Cubical Agda）**全部只有库**（外加论文/手册）→ **"从零开始的集合论教学层"是明确空白**，正是 sokonanoda 的落点。
**本节未能验证**：`isabelle.in.tum.de/dist/…` 全域（301 到死主机 `dist.isabelle.cit.tum.de`）→ 故 ZF 库索引页与 `logics.pdf` 未取到，改用手册 LaTeX 源；Paulson 的 *"The Consistency of the Continuum Hypothesis"* 在其实验室论文页 16 个链接中**找不到该标题**（找到的是 set-I/set-II/AC/Constructible/TR551）；`mmstats.html`、`mpeuni/mm100.html`、`us.metamath.org/downloads.html`、"A Tour of Metamath" 各候选 URL **全部 404**；Mizar 的 `mizar.org`(80)/`mmlquery`/`wiki.mizar.org`/`fm.mizar.org` 不可达；`Coq'Art` 是否含集合论专章未逐章核对，"Ailing" 库查无此物；Cubical 集合论模块的引入 commit 未验证。

## §5 Lean 里的 ZF 与集合论定理形式化现状

**5.1 `ZFSet` 是什么（✅ 读的是 raw 源码）** `inductive PSet : Type (u+1) | mk (α : Type u) (A : α → PSet)`（`Mathlib/SetTheory/ZFC/PSet.lean`）；`def ZFSet : Type (u + 1) := Quotient PSet.setoid`（`ZFC/Basic.lean:48`）→ **是商类型，不是 structure**。`Mathlib/SetTheory/ZFC/` 实际只有 **7 个文件** `PSet Basic Class Ordinal Cardinal Rank VonNeumann`。**⚠️ `ZFC/Constructible.lean` 不存在**（404）→ Mathlib 里**没有可构成宇宙 L**。

**5.2 公理的真实名字（⚠️ 与常见猜测不同）** ✅ 存在：`ZFSet.ext`/`ext_iff`、`ZFSet.empty`/`notMem_empty`、`ZFSet.pair`/`mem_pair`、`ZFSet.mem_union`（`Basic.lean:545`）、`ZFSet.mem_powerset`（`:422`）、`ZFSet.mem_sep`（`:390`）、`ZFSet.regularity (x) (h : x ≠ ∅) : ∃ y ∈ x, x ∩ y = ∅`（`:595`）、`ZFSet.mem_wf`（`:571`）、`ZFSet.inductionOn`（`:577`，∈-归纳）。
❌ **不存在**：`ZFSet.mem_replacement`（替换以 `ZFSet.mem_range`（`:641`）/`mem_image`（`:621`）/`mem_iUnion`（`:662`）+ `ZFSet.Definable/Definable₁/Definable₂` 的形式出现）、`ZFSet.mem_infinity`（真名 `ZFSet.omega`（`:358`）、`omega_zero`（`:362`）、`omega_succ`（`:366`））、`ZFSet.mem_choice`（真名在 `Class.lean`：`ZFSet.choice`（`:333`）、`choice_mem`（`:345`）、`choice_isFunc`（`:341`））。
`Class`：`def Class : Type (u+1) := Set ZFSet`，含 `Class.univ_notMem_univ`（无全集）与 `ZFSet.isOrdinal_notMem_univ`（Burali-Forti）。层谱：`ZFSet.rank : ZFSet → Ordinal`、`noncomputable def vonNeumann (o : Ordinal) : ZFSet`（记 `V_`）、`mem_vonNeumann : x ∈ V_ o ↔ rank x < o`、`iUnion_vonNeumann`。

**5.3 可用性：是展品，不是基础设施（✅ GitHub 代码搜索）** `repo:leanprover-community/mathlib4` 搜 `ZFSet` → **total_count = 9**，其中 6 个是 ZFC 自己的文件，另 3 个是一个记法标签（`Mathlib/Tactic/SetNotationForOrder.lean`）、一个 lint 名单、一个文档索引（`docs/overview.yaml`）。**没有任何其他数学领域用到 `ZFSet`**，其 API 正在被持续 deprecate（`ZFSet.Mem` since 2026-03-16 等）。→ **不能作为教学载体**；但可以当收尾彩蛋（见 §6.8）。

**5.4 序数与基数（✅ 逐条核对源码）** `Ordinal`：`instance Ordinal.isEquivalent : Setoid WellOrder` + `def Ordinal := Quotient …`（`SetTheory/Ordinal/Basic.lean:96-107`）= **良序按序同构作商**。⚠️ `Ordinal.induction` **不存在**，真名 **`Ordinal.inductionOn`**（`:197`）；`Ordinal.lt_wf` ✅（`:465`）；`Ordinal.sup` **不存在**，真名 **`Ordinal.bsup`/`blsub`**（`Ordinal/Family.lean`）；**`Ordinal.omega` 不是 ω**（是"枚举无穷初始序数"的函数，`Cardinal/Aleph.lean:205`），**ω 的真名是 `Ordinal.omega0`**（`Basic.lean:772`，文档原文 "This is not to be confused with the first infinite ordinal `Ordinal.omega0`"）。康托尔范式真名 `Ordinal.CNF`/`coeff`/`eval`（`Ordinal/CantorNormalForm.lean`）。`SetTheory/Ordinal/` 共 16 文件（`Arithmetic Basic CantorNormalForm Commute Enum Exponential Family FixedPoint FixedPointApproximants FundamentalSequence Notation Principal Rank Topology Univ Veblen`）。
`Cardinal`：`def Cardinal := Quotient Cardinal.isEquivalent`（`Type u` 按双射作商）。`Cardinal.cantor (a : Cardinal.{u}) : a < 2 ^ a`（`Cardinal/Order.lean:337`，docstring 就是 "Cantor's theorem"）；`Cardinal.cantor'`（`Basic.lean:317`）；`Cardinal.mk_union_le`（`Basic.lean:819`）；`Cardinal.aleph : Ordinal ↪o Cardinal`（`Aleph.lean:414`）、`aleph_zero`/`aleph_succ`/`aleph_limit`；`Cardinal.continuum`（`Continuum.lean:31`）、`two_power_aleph0 : 2 ^ ℵ₀ = 𝔠`（`:37`）。⚠️ `Cardinal/Cofinality.lean` 只剩 292 B 空壳（`deprecated_module (since := "2026-05-10")`），真内容搬到 `Cardinal/Cofinality/{Basic,Club,Enum,Ordinal}.lean`；cofinality 真名是 **`Order.cof`** 与 **`Ordinal.cof`**，没有 `Cardinal.cof`。`Cardinal/Continuum.lean` **只有基数恒等式，没有 CH**。

**5.5 Cantor / Schröder–Bernstein / 良序定理的真实名字（⚠️ 与任务书猜测差异很大）**
| 内容 | 真实声明 | 文件 |
|---|---|---|
| Cantor（无满射） | **`Function.cantor_surjective {α} (f : α → Set α) : ¬Surjective f`** | `Mathlib/Logic/Function/Basic.lean:373` |
| Cantor（无单射） | `Function.cantor_injective {α} (f : Set α → α) : ¬Injective f` | 同上 `:379` |
| Cantor（基数） | `Cardinal.cantor` | `Cardinal/Order.lean:337` |
| Schröder–Bernstein | **`Function.Embedding.schroeder_bernstein {f : α → β} {g : β → α} (hf : Injective f) (hg : Injective g) : ∃ (h : α → β), Bijective h`** | `SetTheory/Cardinal/SchroederBernstein.lean` |
| 序数版 S–B | `Function.Embedding.antisymm : (α ↪ β) → (β ↪ α) → Nonempty (α ≃ β)` | 同文件 `:97` |
| 良序定理 | **`exists_wellFoundedLT : ∃ (_ : LinearOrder α), WellFoundedLT α`**（docstring 原文 "The well-ordering theorem (or Zermelo's theorem)"） | `Cardinal/Order.lean:548`，**根命名空间**，不在 `Cardinal.` 下 |
❌ 不存在：`Set.cantor`、`Cardinal.schroeder_bernstein`、`Set.schroeder_bernstein`、`WellOrderingTheorem`、`Cardinal.wellOrderable`、`exists_wellFounded`。**教程不要用 `Set.cantor` 这个名字。**

**5.6 Mathlib 之外的 Lean 集合论项目（★ 截至 2026-09-18，✅ 均实际抓取）**
| 仓库 | ★ | 内容 |
|---|---|---|
| **FormalizedFormalLogic/Foundation** | 277 | 一阶 ZFC 公理语句（`Axioms.lean`）+ 模型（`Universe.lean` 的 `models_zf`/`models_zfc`）+ **`zfc_consistent`**；**无 L、无 forcing** |
| flypitch/flypitch | 147 | **Lean 3**（`leanpkg.toml: leanprover/lean3:3.4.2`），`theorem independence_of_CH`；Wiedijk #24 的 Lean 条目就是它 |
| leanprover-community/con-nf | 88 | Quine 的 NF 一致性（不是 ZFC） |
| vihdzp/combinatorial-games | 109 | PGame/surreal（mathlib 已无 `SetTheory/Surreal`） |
| VTrelat/ZFLean | 7 | 在 mathlib `ZFSet` 之上做 relational calculus + 4 个 tactic |
| lanxinge/YesMetaZFC | 4 | 裸 ZFC + Gödel/Rosser/Tarski/Löb + 布尔值模型（仅依赖 Lean/Std） |
| cameronfreer/forcing | 1 | Lean 4 力迫法，ROADMAP M1–M6 complete、**M7 in progress**，尚无独立性定理 |
| 05-02-07/lean-constructible-universe | 0 | 可构成宇宙 L、`modelsCH_lCarrier`；CI 绿（2026-07-23），**数学内容未独立核对** |
**❌ 不存在的仓库**（逐个 404）：`leanprover-community/zfc`、`fpvandoorn/lean-zfc`、`kmill/lean-zfc`、`digama0/zfc`、`model-theory-in-lean`；`Pepe853973422267/lean-ZFC-forcing` 存在但**完全为空**。**Lean 4 侧没有已完成的 CH 独立性证明。**

**5.7 Wiedijk「100 定理」的集合论条目**（✅ 抓 https://www.cs.ru.nl/~freek/100/ 与 https://leanprover-community.github.io/100.html ）
| # | 页面原样标题 | Lean 状态 |
|---|---|---|
| 22 | The Non-Denumerability of the Continuum | ✅ `Cardinal.not_countable_real`（`Mathlib/Analysis/Real/Cardinality.lean`，**不在 SetTheory 目录**） |
| 24 | The Undecidability of the Continuum Hypothesis | ⚠️ **是 Lean 3 的 flypitch** |
| 25 | Schroeder-Bernstein Theorem | ✅ `Function.Embedding.schroeder_bernstein`（spelling 无变音符号） |
| 63 | Cantor's Theorem | ✅ **`Function.cantor_surjective`**（作者 Hölzl & Carneiro） |
**⚠️ 三个必须纠正的猜测**：① #1 是 "The Irrationality of the Square Root of 2"，**Cantor's theorem 是 #63**；② #13 = Polyhedron Formula、#34 = Divergence of the Harmonic Series，**Fermat's Last Theorem 是 #33**（Lean，**不在 Mathlib**，链接 anthropics/fermats-last-theorem）；③ **「良序定理」根本不在 100 条里**（逐条抽取 1–100 标题，无 well-ordering / Zermelo）。边缘条目：#52 "The Number of Subsets of a Set" 在 Lean 是 `Finset.card_powerset`（有限计数）。

## §6 `analysis` 项目 §3 逐节大纲（**本地文件，本报告置信度最高的部分**）

**仓库定位**：本地 `/Users/penglingwei/Documents/lean/analysis`；`git remote -v` → `upstream = git@github.com:teorth/analysis.git`（上游）、`origin = git@github.com:ColorlessBoy/analysis.git`（fork）；当前分支 **`plw/solution`**（已把练习解出）。README 原文（决定"练习=sorry"的协议）："Portions of the text that were left as exercises to the reader are rendered in this translation as `sorry`s. Readers are welcome to fork the repository here to try their hand at these exercises, but I do not intend to place solutions in this repository directly." → **上游 main 上练习是 `sorry`；本地这个分支填了 4,362 行答案。**

**6.1 逐节规模**（本地解答版行数/声明数；上游练习数用 `git show origin/main:…` 统计，并用 `curl raw.githubusercontent.com/teorth/analysis/main/Analysis/Section_3_1.lean` 复核 3.1 = 72 处，两法一致）
| 节 | 行 | thm | lemma | def | abbrev | inst | struct | ex | **上游 `sorry`（=留习题数）** |
|---|---|---|---|---|---|---|---|---|---|
| 3.1 Fundamentals | 1234 | 94 | 17 | 3 | 7 | 19 | 0 | 45 | **72** |
| 3.2 Russell's paradox | 203 | 12 | 0 | 0 | 1 | 0 | 0 | 0 | **10** |
| 3.3 Functions | 810 | 60 | 0 | 5 | 24 | 0 | 1 | 16 | **36** |
| 3.4 Images / inverse images | 845 | 47 | 3 | 5 | 7 | 2 | 0 | 3 | **36** |
| 3.5 Cartesian products | 1328 | 46 | 3 | 12 | 9 | 2 | 2 | 2 | **64** |
| 3.6 Cardinality of sets | 2463 | 59 | 10 | 10 | 4 | 1 | 0 | 0 | **54** |
| 3 epilogue (ZFSet) | 101 | 1+ | — | 1 | — | 1 | — | — | **0（上游亦完成）** |
| **合计** | **6984** | 319 | 33 | 36 | 52 | 25 | 3 | 66 | **272** |
依赖图（`grep '^import'`）：3_2→3_1；3_3→3_1 + `Analysis.Tools.ExistsUnique`；3_4→3_1；3_5→3_1,3_2,3_4；3_6→3_3,3_5；epilogue→`Mathlib.SetTheory.ZFC.PSet`、`ZFC.Basic`、`Tools/ExistsUnique`、3_1。

**6.2 §3.1 Fundamentals —— 教程地基，也是"公理化 vs 类型论"的教学范本** `class SetTheory`（`Section_3_1.lean:80-110`）把 Tao 的 Axiom 3.1–3.12 **逐条变成 class 字段**：`Set`、`Object`、`set_to_object : Set ↪ Object`(3.1)、`mem`、`extensionality`(3.2)、`emptyset`/`emptyset_mem`(3.3)、`singleton`/`singleton_axiom`(3.4)、`union_pair`/`union_pair_axiom`(3.5)、`specify`/`specification_axiom`(3.6)、`replace`/`replacement_axiom`(3.7，带唯一性假设 `hP`)、`nat`/`nat_equiv`(3.8)、`regularity_axiom`(3.9)、`pow`/`function_to_object`/`powerset_axiom`(3.11)、`union`/`union_axiom`(3.12)。
**记法/强制层（"SetTheory class/coercion 层做了什么"的答案）**：`export SetTheory (Set Object)` + `variable [SetTheory]`，再登记 `Membership Object Set`（`∈`）、`Coe Set Object`、`EmptyCollection Set`（`∅`）、`Singleton Object Set`（`{a}`）、`Union Set`（`∪`）、`Insert Object Set`、`HasSubset Set`（`⊆`）、`HasSSubset Set`（`⊂`）、**`CoeSort Set (Type v)`（让 `x : A` 这种"集合当类型"成立**，配 `abbrev Set.toSubtype (A) := Subtype (fun x ↦ x ∈ A)`，文件里用 `example` 同时演示 `x'.val ∈ A` 与 `x : A`）、`Pow Set Set`、`CoeOut (X → Y) Object`、`SProd Set Set Set`（`×ˢ`）。
Tao 编号覆盖：Axiom 3.1–3.12 / Definition 3.1.1, 3.1.14, 3.1.22, 3.1.26 / Lemma 3.1.5, 3.1.12 / Proposition 3.1.17, 3.1.27 / Remark 3.1.9, 3.1.11, 3.1.15 / Example 3.1.10,16,17,24,25,28,30,31 / Exercise 3.1.1–3.1.13。代表作：`union_comm`、`union_assoc`、`union_self`、`union_empty`、`subset_trans`、`subset_antisymm`、`ssubset_trans`、`empty_unique`、`singleton_uniq`、`pair_uniq`、`pair_eq_pair`、`emptyset_neq_singleton`、`specification_axiom`（三个变体）、`Set.subtype_mk`/`subtype_property`/`coe_inj`。

**6.3 §3.2 Russell's paradox（203 行 / 13 声明）—— 最省的一节，适合做"反证法"关** `abbrev axiom_of_universal_specification : Prop := ∀ P : Object → Prop, ∃ A : Set, ∀ x, x ∈ A ↔ P x`；`theorem Russells_paradox : ¬ axiom_of_universal_specification`（证明刻意照原文结构：`set P := fun x ↦ ∃ X:Set, x = X ∧ x ∉ X`，`choose Ω hΩ using h P`，再 `by_cases`）；`SetTheory.Set.axiom_of_regularity {A} (h : A ≠ ∅) : ∃ x:A, ∀ S:Set, x.val = S → Disjoint S A`。Exercise 3.2.1（要求**不用** `emptyset`/`singleton` 等公理地证 `emptyset_exists`、`singleton_exists`、`pair_exists`、`union_exists`、`specify_exists`、`replace_exists` 六条存在性）、3.2.2（`not_mem_self`、`not_mem_mem`）、3.2.3（`univ_iff`、`no_univ`：**不存在全集**）。

**6.4 §3.3 Functions（810 行 / 108 声明）—— "集合论函数 vs 类型论函数"的桥** `structure Function (X Y: Set) where P : X → Y → Prop; unique : ∀ x, ∃! y, P x y`（Definition 3.3.1）；`noncomputable def Function.to_fn : X → Y`（文件注明用了 choice）；`abbrev Function.mk_fn (f : X → Y) : Function X Y`；`Function.eq_iff : f = g ↔ ∀ x, f x = g x`；`comp_eval`、`comp_assoc`；`abbrev one_to_one`/`onto`/`bijective` 及对 `Function.Injective/Surjective/Bijective` 的 `iff` 版本；`Function.inverse`（配 `inverse_eval`、`inverse_eq`）；**`Function.bijective_incorrect_def`（故意给错定义让读者找反例）**；`f_3_3_24`/`g_3_3_24`/`h_3_3_24` 三个有限例子用 `decide` 判单射/满射。Tao：Definition 3.3.1/8/13/20/23；Example 3.3.3–3.3.25；Exercise 3.3.1–3.3.8。**`Analysis/Tools/ExistsUnique.lean`（为这一章专写的工具文件）**：`#check` 出 Mathlib 的 `existsUnique_of_exists_of_unique` 等，再补 `ExistsUnique.choose`（唯一选择公理）、`choose_spec`、`choose_eq`、`choose_iff`、`Subsingleton.choose`，以及 `ExistsUnique.iff_subsingleton_nonempty` + `#print axioms` 证明**不需要选择**。

**6.5 §3.4 Images and inverse images（845 行 / 72 声明）—— 与 sokonanoda 最接近的一节** `abbrev image {X Y} (f:X → Y) (S: Set) : Set := X.replace (P := fun x y ↦ f x = y ∧ x.val ∈ S) …`（Definition 3.4.1，**定义不要求 `S ⊆ X`**）；`mem_image`（用 `replacement_axiom` 展开）；**`image_eq_specify`（同一概念用 specification 另定义一遍并证明相等——极好的教学装置）**；`preimage {X Y} (f:X → Y) (U: Set) : Set := X.specify (P := fun x ↦ (f x).val ∈ U)`；`mem_preimage`、`mem_preimage'`；`image_of_inter`/`image_of_diff`/`image_of_union`（**`⊆` 与 `=` 的对比**）；**`image_of_inter'` / `image_of_diff'` 用 `Decidable` 判定"这个等式一般成立吗"**：`def Set.image_of_inter' : Decidable (∀ X Y:Set, ∀ f:X → Y, ∀ A B: Set, image f (A ∩ B) = (image f A) ∩ (image f B))`——**把"找反例"做成可判定的 `def`**。幂集：`instance Pow Set Set`、`powerset`、`mem_powerset`、`exists_powerset`、`powerset_of_triple`、`powerset_axiom`（Axiom 3.11）；索引族 `iUnion`/`mem_iUnion`/`iUnion_eq`/`iUnion_of_empty`/`iInter'`/`mem_iInter`（**要求 `I ≠ ∅`**）、`union_iUnion`、`inter_iInter`、`compl_iUnion`、`compl_iInter`（De Morgan）；`partial_functions`（Exercise 3.4.6 完整解）。Tao：Definition 3.4.1, 3.4.4；Lemma 3.4.10；Example 3.4.2,3,6,7,9,12；Exercise 3.4.1–3.4.11。

**6.6 §3.5 Cartesian products（1328 行 / 91 声明）** `structure OrderedPair where fst : Object; snd : Object` + `OrderedPair.eq` + `toObject : OrderedPair ↪ Object` + `inst_coeObject`；`Set.slice`、`cartesian`（`instance SProd Set Set Set` 让 `X ×ˢ Y` 可用）、`mem_cartesian`、`pair_eq_fst_snd`、`mk_cartesian`、`fst_of_mk_cartesian`、`snd_of_mk_cartesian`；索引积 `tuple`、`iProd`、`mem_iProd`、`tuple_mem_iProd`、`tuple_inj`、`empty_iProd_equiv`、`iProd_equiv_prod_aux`、`iProd_equiv_prod_triple_aux`；有限集 `Set.Fin (n:ℕ) : Set := nat.specify (fun m ↦ m < n)` + `mem_Fin`、`Fin_mk`、`Fin.toNat_spec`、`coe_inj`、`Fin_embed`，以及 **`finite_choice : (∀ i, X i ≠ ∅) → iProd X ≠ ∅`**（Exercise 3.5.4/3.5.5 的核心）；`structure Tuple (n:ℕ)`；积的分配律 `prod_union`/`prod_inter`/`prod_diff`/`union_prod`/`inter_prod`/`diff_prod`（**六个全是 `=`，很适合入门**）、`inter_of_prod`/`union_of_prod`/`diff_of_prod`（`⊆`；后两个是 `def … : Decidable (…)`，**又把反例做成可判定**）、`prod_subset_prod`；`direct_sum`（Exercise 3.5.11 解）；`graph`/`graph_inj`/`is_graph`（函数的图刻画）；收尾 `Set.recursion`（`nat` 上递归，Exercise 3.5.12）与 `nat_unique`（Exercise 3.5.13，**Tao 的"自然数系在同构意义下唯一"**）。Tao：Definition 3.5.1, 3.5.4, 3.5.6；Lemma 3.5.11；Example 3.5.5, 3.5.8, 3.5.10；Exercise 3.5.1–3.5.13。

**6.7 §3.6 Cardinality of sets（2463 行 / 85 声明）—— 最大也最难移植的一节** `abbrev EqualCard (X Y:Set) : Prop := ∃ f : X → Y, Function.Bijective f`（Definition 3.6.1）→ `EqualCard.refl/symm/trans` → **`instance EqualCard.inst_setoid : Setoid Set`** → `abbrev has_card (X:Set) (n:ℕ) : Prop := X ≈ Fin n`（Definition 3.6.5）→ `has_card_iff`、`Remark_3_6_6`、`has_card_zero : X.has_card 0 ↔ X = ∅`、`card_erase`、**`card_uniq : X.has_card n → X.has_card m → n = m`**（Proposition 3.6.4，技术心脏）、`card_fin_eq`、`Fin_card`、`Fin_finite`、`finite (X:Set) : Prop := ∃ n, X.has_card n`、`infinite := ¬ finite`、**`nat_infinite : infinite nat`**、`noncomputable def card (X:Set) : ℕ := if h:X.finite then h.choose else 0`、`has_card_card`、`card_to_has_card`、`empty_iff_card_eq_zero`、`card_insert`、`card_union`、`card_union_disjoint`、`card_subset`、`card_ssubset`、`card_image`、`card_image_inj`、`card_prod`、`card_pow`；结尾 **`finite_iff_finite : X.finite ↔ Finite X`**、**`card_eq_nat_card : X.card = Nat.card X`**、**`card_eq_ncard : X.card = (X: _root_.Set Object).ncard`**（**把自制基数接到 Mathlib 的 `Nat.card` / `Set.ncard` 上——完美的最后一课落点**）；`pow_fun_equiv : ↑(A ^ B) ≃ (B → A)`；`Fin.succAbove`/`predAbove` 与 `Permutations_card`。Tao：Definition 3.6.1, 3.6.5；Proposition 3.6.4, 3.6.8, 3.6.14；Theorem 3.6.12；Lemma 3.6.9；Remark 3.6.6；Example 3.6.2, 3.6.3, 3.6.7；Exercise 3.6.1–3.6.12。

**6.8 §3 epilogue（101 行，上游也无 sorry）—— 我们大概率不做，但值得知道** `PSet.ofNat_mem_ofNat_of_lt`、`PSet.mem_ofNat_iff`、`PSet.eq_of_ofNat_equiv_ofNat`、`noncomputable def ZFSet.nat_equiv : ℕ ≃ omega`，然后 **`noncomputable instance ZFSet.inst_SetTheory : Chapter3.SetTheory.{u+1,u+1}`**：把 §3.1 的 class 用 Mathlib 的 `ZFSet` 实例化（`ext`、`notMem_empty`、`mem_singleton`、`mem_union`、`ZFSet.sep`、`image`、`omega`、`regularity`、`funs`、`sUnion` …），从而**证明"ZF 公理系统有一个模型"**。docstring 注明构造来自 Edward van de Meent，并给 Zulip 讨论链接（https://leanprover.zulipchat.com/#narrow/channel/113489-new-members/…/527305173 ）。**这是"把教学用公理化集合论接回真 Lean"的漂亮收尾，也是 §5.1 里 `ZFSet` 唯一被"用起来"的地方。**

**6.9 §3 实际依赖的记法与 tactic**（`grep -o | wc -l`，本地解答版）
记法：`∈` 287、`∅` 118、`∪` 179、`∩` 76、`⊆` 108、`⊂` 13、`×ˢ` 120、`\` 126、`⟨·,·⟩` 859、`≃` 38、`↪` 4、`∃!` 17、`¬` 31、`≈`(Setoid) 8、`''` 98、`⁻¹'` 6、`⋃` 4、`⋂` 0、**`𝒫` 0**（analysis 用自家 `A ^ B` 幂集，不是 `𝒫`——**比 Mathlib 更接近我们的处境**）。
tactic 频次（**这些正是我们全都缺的**）：`rw` 918、`have` 914、`intro` 693、`simp` 497、`exact` 496、`apply` 407、`use` 243、`constructor` 172、`rfl` 169、`omega` 128、`choose` 125、`rcases` 125、`obtain` 89、`ext` 75、`aesop` 57、`by_cases` 46、`refine` 29、`simp_all` 20、`norm_num` 18、`grind` 16、`tauto` 16、`induction` 14、`contrapose` 13、`order` 8、`specialize` 7、`match` 5、`decide` 5、`positivity` 3、`assumption` 2、`wlog` 1。
对照 sokonanoda 只有 `intro`/`exact`/`apply`/`assumption`/`rfl`/`match`/`sorry`：**§3 里能"照抄"的只有这六种**，其余（`rw`/`simp`/`use`/`constructor`/`choose`/`ext`/`by_cases`/`omega`）要么改写、要么成为缺口工单。

## §7 「可偷装置」清单（每条：来源 + 解决什么 + 在无记法/无类型类的小语言里怎么变形）

| # | 装置 | 来源（已验证） | 解决什么 | 在 sokonanoda 里的变形 |
|---|---|---|---|---|
| 1 | **集合 = 谓词** `def Set (α) := α → Prop` | Mathlib `Defs.lean:51`；LPA `1SetDefinitions.lean`；`OK-set-spike` | 零新语法就有集合 | **已成立**，直接 `def Set (α : Type) : Type := α -> Prop` |
| 2 | **隶属 = 应用 + 一层 wrapper** | Mathlib `Membership`；LPA `def Mem` | 把 `A x` 读成 `x ∈ A` | 无记法 → 保留 `Set.mem α a A`，在**注释**里写 `-- x ∈ A 就是 Set.mem α x A`。**不要**发明 `∈` |
| 3 | **子集 = 全称箭头** | `Set.subset_def`；analysis `Set.subset_def`（`rfl` 可证） | "证明子集" = `intro x hx` | `def Set.subset α A B := forall (x : α), A x -> B x`，配 `theorem subset_def : … := rfl` 当第一关 |
| 4 | **逐步解锁语法（`NewTactic`/`NewDefinition`/`NewLemma`）** | STG4 每关文件头 | 防止初学者面对完整语法瘫痪 | **杀手锏**：第 1 关只给 `intro`/`exact`，后面逐关引入 `apply`/`assumption`/`match`/`rfl`；用 `-- soko:hint` 声明"本关只允许 X" |
| 5 | **三层 Hint（普通/`hidden`/`strict`）+ `Branch` 死路** | STG4 `FamUnion/L01proveexists`、`Subset/L03have` | 提示不过早剧透又能兜底；把常见错误做成可探索分支 | sokonanoda 已有 `-- soko:hint` 阶梯（**从不含答案**）。可加"**反例分支**"惯例：先让学习者撞 `use ∅` 这类死路再引回；`Branch` 的等价物 = 一道故意写假的 `example` + 提示"找反例" |
| 6 | **"证明或证伪"成对出题** | MoP 第 9 章 | 逼学习者先判断真假，而非机械 `intro` | 每 2–3 关配一道**假命题**，作答要求给反例（构造 `¬` 证明） |
| 7 | **把"一般等式不成立"做成可判定 `def`** | analysis `Set.image_of_inter' : Decidable (∀ …, image f (A∩B) = …)` | 让"找反例"成为**合法且被内核检验**的作答 | 无 `Decidable`/`instance` → 写成 **`theorem image_of_inter_counter : ¬ (forall …)`** + 让学习者构造反例，等价效果 |
| 8 | **同一概念两种定义 + 证明相等** | analysis `image`(用 replace) vs `image_eq_specify`(用 specify) | 教"定义不是天赐的，可以换" | e.g. `Set.union` 用 `Or` 定义 vs 展开式；`Set.subset` 用 `forall` vs term 写法 |
| 9 | **公理系统 = 逐条编号的字段** | analysis `class SetTheory`（Axiom 3.1–3.12） | 让"公理化"看得见、可数、可对照教科书 | 无类型类 → 改成 **`axiom` 逐条列出**（我们正好有 `axiom`）：`axiom Set.ext …`、`axiom Set.empty …`；或更好：**在纯谓词模型里把公理变成定理**（`Set.empty := fun _ => False` 后 `rfl` 可证），把"公理化"留作最后一章对比 |
| 10 | **"集合当类型"（`CoeSort` + `Subtype`）** | analysis `Set.toSubtype` + `CoeSort Set (Type v)` | 让 `x : A` 与 `x : Object` 并存 | 无类型类 → **不做**。改用显式 `theorem mem_of_subset (h : Set.subset α A B) (hx : A x) : B x` 之类把 coercion 显式化 |
| 11 | **索引族用"index 集合 + 函数"而非 `⋃ i,`** | analysis `iUnion (I:Set) (A: I → Set)`、`mem_iUnion {I} (A: I→Set) (x:Object) : x ∈ iUnion I A ↔ ∃ i:I, x ∈ A i` | 在无记法语言里表达任意并/交 | 直接采用 analysis 签名（`I : Set` 可改成 `I : Type` 更省事）；`Set.iInter` 必须带 `hI : I ≠ ∅` 的**非空前提**（很好的教学点：空族的交没有意义） |
| 12 | **幂集用"子集谓词"而非 `𝒫`** | Mathlib 有 `𝒫`（`Defs.lean:261`）但 analysis **一次没用**，改用 `Set.power (A) : Set (Set α) := fun B => Set.subset α B A` | 幂集关：**幂集是"集合的集合"** | 用 analysis 写法：`def Set.power (α) (A : Set α) : Set (Set α) := fun B => Set.subset α B A`；`theorem mem_power : Set.power α A B ↔ Set.subset α B A := rfl` |
| 13 | **用 ExistsUnique 承载"函数"** | analysis `structure Function (X Y:Set) where P : X → Y → Prop; unique : ∀ x, ∃! y, P x y` | 集合论函数 = 满足唯一性的关系 | 无 structure → (a) 直接用类型论函数 `X → Y`（analysis 从 §3.4 起就这么做）；(b) 若要做"关系式函数"，`def IsFunction (R : X → Y → Prop) : Prop := (forall x, Exists (R x)) /\ …`；**对小语言 (a) 更省** |
| 14 | **有意给错定义让读者找反例** | analysis `Function.bijective_incorrect_def` | 破除"定义必然对"的错觉 | 教学关卡放 `def BadBij …` + `theorem bad_bij_counter : ¬ (forall …, …)` |
| 15 | **有限例子落地到具体数字** | analysis `Function.f_3_3_24 : Fin 3 → ({3,4}:Set ℕ)` 等 | 让抽象定义可算 | 我们有 `inductive`+`match`：定义 `inductive Three ctor a ctor b ctor c end` 再写具体函数；**不要**依赖 `Fin`/`decide` |
| 16 | **epilogue：把自制理论接到真库** | analysis `Section_3_epilogue.lean` 的 `instance ZFSet.inst_SetTheory` | 收尾回答"我们造的和真的是一回事吗" | 无类型类 → 改成"**总结课**"：把教程的 `Set α` 与真实 Lean 的 `Set α := α → Prop` 逐条对照（`Set.mem`↔`∈`、`Set.subset`↔`⊆`、`Set.union`↔`∪`），说明**唯一障碍是记法而非语义**——正是 `docs/gaps/repro/G04-notation.sokonanoda` 的动机 |
| 17 | **先散文后形式** | Logic and Proof 第 11 章（散文）→ 第 12 章（Lean）；MoP 的 Example 双语调 | 先建数学直觉 | 每个 unit 头部用 `--` 写散文命题 + 一句"Lean 里它长什么样"，再给 `example` |
| 18 | **提示密度随进度递减** | STG4 `Combo.lean` world 介绍原文 "we'll leave you on your own" | 避免永久依赖提示 | unit 元数据显式声明 hint 预算：前 2 关每题 3 条，中段 1 条，末段 0 条 |

## §8 参考来源清单（访问日期 2026-09-18）

**Mathlib 源码 / 文档 / API / Contents API**（docs 快照 commit **`797def14ae4c973c2fc3634045d952cc8e2a2952`**，从抓到的 docs 页面 source 链接读出）
Lean core 记号定义：https://github.com/leanprover/lean4/blob/master/src/Init/Core.lean（`⊆ ⊂ ∪ ∩ \ ∅`）· …/src/Init/Notation.lean（`∈ ∉`）
https://raw.githubusercontent.com/leanprover-community/mathlib4/master/Mathlib/Data/Set/{Defs,Basic,Operations,Image,Function,Insert,Subsingleton,Disjoint,Card,Restrict}.lean ·
…/Mathlib/Data/Set/Finite/Basic.lean · …/Mathlib/Data/Set/Lattice.lean（292 B 弃用壳）· …/Mathlib/Data/Set/Lattice/{Indexed,Image,Bounded,Disjoint}.lean ·
…/Mathlib/Basic/Finite/Defs.lean（`Set.Finite` 真身）· …/Mathlib/Order/Notation.lean（`ᶜ`）· …/Mathlib/Order/SetNotation.lean（`⋃ i,`/`⋂ i,`/`⋃₀`/`⋂₀`）· …/Mathlib/Order/Disjoint.lean ·
…/Mathlib/Data/SProd.lean（`×ˢ`）· …/Mathlib/Data/Finset/Card.lean（`#s`）· …/Mathlib/Logic/Equiv/Set.lean · …/Mathlib/Order/Interval/Set/Defs.lean（`Set.Icc`）·
…/Mathlib/SetTheory/ZFC/{Basic,PSet}.lean · …/Mathlib/SetTheory/Cardinal/{Defs,Order,Basic,SchroederBernstein,Finite}.lean · …/Mathlib/SetTheory/Ordinal/Basic.lean
https://api.github.com/repos/leanprover-community/mathlib4/contents/Mathlib/{Data/Set, SetTheory, SetTheory/ZFC, SetTheory/Cardinal, SetTheory/Ordinal, SetTheory/Descriptive, Order, Order/Set}（最后一项 404）
**Loogle 判定"不存在"的证据**：https://loogle.lean-lang.org/json?q=Set.Equiv （返回 `unknown identifier 'Set.Equiv'`）
https://leanprover-community.github.io/mathlib4_docs/Mathlib/Data/Set/{Defs,Basic,Operations,Image,Finite/Basic,Card,Subsingleton}.html ·
…/Mathlib/Data/Set/Lattice/Disjoint.html · …/Mathlib/Basic/Finite/Defs.html · …/Mathlib/Order/SetNotation.html · …/Mathlib/Logic/Equiv/Set.html ·
…/Mathlib/Data/Finset/Card.html#Finset.«term#_» · …/Mathlib/SetTheory/Cardinal/{Order.html#Cardinal.cantor,Defs.html} · …/Mathlib/Logic/Function/Basic.html#Function.cantor_surjective

**课程与教材**
https://leanprover-community.github.io/mathematics_in_lean/ · https://leanprover-community.github.io/mathematics_in_lean/C04_Sets_and_Functions.html ·
https://raw.githubusercontent.com/leanprover-community/mathematics_in_lean/master/MIL/C04_Sets_and_Functions/{S01_Sets,S02_Functions,S03_The_Schroeder_Bernstein_Theorem}.lean 及 `solutions/` ·
https://leanprover.github.io/theorem_proving_in_lean4/ · https://raw.githubusercontent.com/leanprover/theorem_proving_in_lean4/master/book/lakefile.toml ·
https://hrmacbeth.github.io/math2001/ · https://github.com/hrmacbeth/math2001 ·
https://leanprover-community.github.io/logic_and_proof/ · …/sets.html · …/sets_in_lean.html ·
https://www.ma.imperial.ac.uk/~buzzard/xena/formalising-mathematics-2024/ · https://github.com/ImperialCollegeLondon/formalising-mathematics-2024 ·
https://github.com/b-mehta/formalising-mathematics-notes · https://b-mehta.github.io/formalising-mathematics-notes/ ·
https://github.com/niotie/logique-preuve-assistee · https://www.uv.es/coslloen/Lean4/ · https://github.com/encosllo/IntroToLean4/ ·
https://lijungeometry.github.io/342.html · https://sinhp.github.io/teaching/2025-logic2-stockholm/ ·
https://leanprover-community.github.io/theories/sets.html · https://leanprover-community.github.io/learn.html · https://leanprover-community.github.io/undergrad.html ·
https://leanprover-community.github.io/teaching/courses.html · https://raw.githubusercontent.com/leanprover-community/leanprover-community.github.io/lean4/data/courses.yaml

**游戏**
https://raw.githubusercontent.com/leanprover-community/lean4game/main/README.md （游戏表 = STG4 存在的证据）· https://github.com/djvelleman/stg4 ·
https://cdn.jsdelivr.net/gh/djvelleman/stg4@main/Game.lean ·
https://cdn.jsdelivr.net/gh/djvelleman/stg4@main/Game/Levels/{Subset,Comp,Inter,Union,Combo,FamInter,FamUnion,FamCombo}.lean ·
https://cdn.jsdelivr.net/gh/djvelleman/stg4@main/Game/Levels/{Subset/L01exact,Subset/L03have,Subset/L05subref,Union/L03cases,Inter/L05subint,Comp/L01contra,FamUnion/L01proveexists,FamCombo/L08singleton,Combo/L05union_sub_inter_sub}.lean ·
https://adam.math.hhu.de/ （SPA；`/api/*` 端点实测 404）· https://github.com/leanprover-community/nng4 · https://github.com/trequetrum/lean4game-logic ·
https://github.com/hhu-adam/robo · https://github.com/alexkontorovich/realanalysisgame · https://github.com/emilyriehl/reintroductiontoproofs

**Lean 项目与 100 定理**
https://www.cs.ru.nl/~freek/100/ · https://leanprover-community.github.io/100.html · https://github.com/FormalizedFormalLogic/Foundation ·
https://github.com/flypitch/flypitch · https://github.com/cameronfreer/forcing · https://github.com/VTrelat/ZFLean · https://github.com/lanxinge/YesMetaZFC ·
https://github.com/leanprover-community/con-nf · https://github.com/vihdzp/combinatorial-games · https://github.com/05-02-07/lean-constructible-universe

**本仓库内的对照材料**
`docs/gaps/repro/OK-set-spike.sokonanoda`（集合论可行性正向探针，11 声明全绿）· `docs/gaps/repro/{G04-notation,G05-namespace-open,G02-ctor-namespace}.sokonanoda` ·
`course/unit8-quantifiers.sokonanoda`、`course/unit9-relations-connectives.sokonanoda`（集合论单元的前置）·
`docs/notes/settheory-survey/lean4-sets-functions-prior-art.md`（本调研 §2 的长版底稿，含 MIL/LPA 练习全文）·
本地 `analysis` 仓库（`plw/solution` 分支）`Analysis/Section_3_1.lean` … `Section_3_6.lean`、`Section_3_epilogue.lean`、`Analysis/Tools/ExistsUnique.lean` ·
上游对照 https://github.com/teorth/analysis/blob/main/Analysis/Section_3_1.lean （`git show origin/main:` 与 `curl raw…` 两法核对 72 处 `sorry`）

**未核实清单（明确留给后续）**：~~Isabelle/ZF 理论文件与教学层；Metamath `set.mm` 规模与公理名；Mizar MML 文章清单；Coq `Coq.Sets.*`/`Coq.ZF`/stdpp/MathComp；Agda/Idris/ACL2/HOL 的集合方案；Lean 3 mathlib `src/set_theory/`~~ → **已在 §4 补齐**（含逐条已验证的否定结论）。§1 的 Mathlib 部分已按 Loogle + docs 逐字复核（§1.2/1.3/1.4/1.5/1.6 均已修订）。
**已验证的"不存在"**（属"验证了不存在"，不是"未能验证"）：`Set.Equiv`、`Set.EquivalentOn`、`Set.Finite.card`、`Set.iInter_subset_iff`、`Set.EqOn.refl`、`Set.SurjOn.image_eq`、`Set.image_subset_image`、`Set.compl_compl`/`Set.compl_union`/`Set.compl_inter`、`Set.subset_antisymm`、`Set.not_mem_empty`（真名 `Set.notMem_empty`）、`↾`（`Data/Set/Restrict.lean` 全文无任何记号声明）、`Coq.Sets.Ordinals`/`Cardinals`/`ZF`、`Coq.ZF`、`std/sets`（ACL2）、`Mathlib/Order/Set` 目录、`Mathlib/Data/Set/Finite.lean`/`Cardinal.lean`、`lean-3` 分支。
**仍未核实的**：NNG4/Logic Game/Robo 的关卡数与 hint 机制；EuroProofNet 游戏化产出；Imperial 课程号/MATH40001、Cambridge/Waterloo/Chapman 的 Lean 集合论课；Paulson 的 "The Consistency of the Continuum Hypothesis"（论文页 16 个链接中无此标题）；Cubical Agda 集合论模块的引入 commit；`isabelle.in.tum.de/dist/*`、`us.metamath.org/mpeuni/mmstats.html`、`mizar.org:80`、`web.archive.org`（域名/路径本身不可达或 404）。
