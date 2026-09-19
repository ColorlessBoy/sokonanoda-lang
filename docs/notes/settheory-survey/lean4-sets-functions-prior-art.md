# Lean 4 集合论 / 函数 教学材料调研（严格取证版）

- **访问日期**：2026-09-18（`date` → Fri Sep 18 20:23 CST 2026 / 2026-09-18 12:23 UTC）
- **取证纪律**：本报告每一条具体断言都对应一次实际 `web_fetch`；无法取证的条目标注 **未能验证**。
- **本会话限制**：`web_search` 不可用（服务端缺 `DEEPSEEK_API_KEY`，返回错误而非结果）。因此第 6 项"搜索课程"改为两条可取证路径：① 社区维护的课程总表 `teaching/courses.html` 与其数据源 `courses.yaml`；② 对具体 URL 直接探测。凡未探测到的，一律写"未能验证"，**没有编造任何课程号、文件名或章节名**。
- 本次共执行 **60 次 web_fetch**。

---

## 0. 先纠正四个前提错误（重要）

| 你的前提 | 实际情况 | 证据 |
|---|---|---|
| MIL 集合章是 C06 / 练习文件 `MIL/C06_SetsAndFunctions.lean` | 集合章是 **第 4 章**；C06 是 *Discrete Mathematics*；且 MIL 练习文件**按节拆分**，不按章 | [MIL 目录](https://leanprover-community.github.io/mathematics_in_lean/) + [GitHub API 列目录](https://api.github.com/repos/leanprover-community/mathematics_in_lean/contents/MIL/C04_Sets_and_Functions) |
| TPIL 有 "Sets and Functions" 章，用 Lean core `Set` | TPIL 4 **12 章里没有任何集合章**；Lean 3 版 11 章同样没有。"Sets and Functions" 是 **MIL 第 4 章**的标题 | [TPIL4 目录](https://leanprover.github.io/theorem_proving_in_lean4/) + [第 4 章](https://leanprover.github.io/theorem_proving_in_lean4/Quantifiers-and-Equality/) + [TPIL3 目录](https://leanprover.github.io/theorem_proving_in_lean/) |
| 《The Mechanics of Proof》第 2/4 章讲集合 | 第 2、4 章是 *Proofs with structure* I/II（自然演绎）；集合是 **第 9 章**，函数是 **第 8 章** | [MoP 目录](https://hrmacbeth.github.io/math2001/) |
| Logic and Proof 作者是 Avigad / de Moura / Kong / Ullrich | 该版页脚署名是 **Jeremy Avigad, Joseph Hua, Robert Y. Lewis, Floris van Doorn**；且这是 **Lean 3 时代**的书 | [L&P 首页](https://leanprover-community.github.io/logic_and_proof/) + [courses.yaml](https://raw.githubusercontent.com/leanprover-community/leanprover-community.github.io/lean4/data/courses.yaml) |

---

## 1. Mathematics in Lean (MIL) — 已验证

### (a) URL
- 书（v4.19.0 文档）：https://leanprover-community.github.io/mathematics_in_lean/
- **第 4 章**：https://leanprover-community.github.io/mathematics_in_lean/C04_Sets_and_Functions.html

### (b) 章节号与标题（页面原文）
**4. Sets and Functions**，下辖三节：
- **4.1. Sets**
- **4.2. Functions**
- **4.3. The Schröder-Bernstein Theorem**

### 练习文件（真实文件名，已用 GitHub API 逐一取证）
路径根是仓库 https://github.com/leanprover-community/mathematics_in_lean （默认分支 `master`）：

| 文件 | 大小 | URL |
|---|---|---|
| `MIL/C04_Sets_and_Functions/S01_Sets.lean` | 5090 B | [raw](https://raw.githubusercontent.com/leanprover-community/mathematics_in_lean/master/MIL/C04_Sets_and_Functions/S01_Sets.lean) |
| `MIL/C04_Sets_and_Functions/S02_Functions.lean` | 3902 B | [raw](https://raw.githubusercontent.com/leanprover-community/mathematics_in_lean/master/MIL/C04_Sets_and_Functions/S02_Functions.lean) |
| `MIL/C04_Sets_and_Functions/S03_The_Schroeder_Bernstein_Theorem.lean` | 2560 B | [raw](https://raw.githubusercontent.com/leanprover-community/mathematics_in_lean/master/MIL/C04_Sets_and_Functions/S03_The_Schroeder_Bernstein_Theorem.lean) |
| `.../solutions/Solutions_S01_Sets.lean` | 2916 B | [API 目录](https://api.github.com/repos/leanprover-community/mathematics_in_lean/contents/MIL/C04_Sets_and_Functions/solutions) |
| `.../solutions/Solutions_S02_Functions.lean` | 5629 B | 同上 |
| `.../solutions/Solutions_S03_The_Schroeder_Bernstein_Theorem.lean` | 2350 B | 同上 |

> 注意：`MIL/C06_SetsAndFunctions.lean`、`MIL/C04_Sets_and_Functions.lean`、`main` 分支上的同名文件 **均 404**（已实测）。

### (c) 实际讲了/证了什么
- **4.1 Sets**：`Set α` 即 `α → Prop`；`⊆ ∩ ∪ ∅ univ \`、`ext`、`Subset.antisymm`、set-builder、`mem_inter_iff`/`mem_union`/`mem_iUnion`/`mem_iInter`、有界量词 `∀ x ∈ s` / `∃ x ∈ s`（`ball`/`bex`）、`⋃₀`/`⋂₀`（`sUnion_eq_biUnion`/`sInter_eq_biInter`）。示例含 `evens ∪ odds = univ`（用 `Classical.em`）、`{n | Nat.Prime n} ∩ {n | n > 2} ⊆ {n | ¬Even n}`、`(⋂ p ∈ primes, {x | ¬p ∣ x}) ⊆ {x | x = 1}`（用 `Nat.exists_prime_and_dvd`）。
- **4.2 Functions**：`f ⁻¹' p`（preimage）与 `f '' s`（image）的分配律、`Injective/Surjective/InjOn`、`Classical.choose` 构造 `inverse`、**Cantor 定理** `∀ f : α → Set α, ¬ Surjective f`、`InjOn log {x | x > 0}`、`range exp = {y | y > 0}`。
- **4.3 Schröder-Bernstein**：定义 `sbAux : ℕ → Set α`（`0 => univ \ g '' univ`，`n+1 => g '' (f '' sbAux n)`）、`sbSet := ⋃ n, sbAux f g n`、`sbFun`（`if x ∈ sbSet then f x else invFun g x`），最后给出 `theorem schroeder_bernstein ... ∃ h : α → β, Bijective h`。

### (d) 实际布置的练习（原文照抄，`sorry` 为练习标记）
**S01_Sets.lean（12 处 `sorry`）**：
```lean
example : s ∩ t ∪ s ∩ u ⊆ s ∩ (t ∪ u) := by sorry
example : s \ (t ∪ u) ⊆ (s \ t) \ u := by sorry
example : s ∩ t = t ∩ s := Subset.antisymm sorry sorry
example : s ∩ (s ∪ t) = s := by sorry
example : s ∪ s ∩ t = s := by sorry
example : s \ t ∪ t = s ∪ t := by sorry
example : s \ t ∪ t \ s = (s ∪ t) \ (s ∩ t) := by sorry
example : { n | Nat.Prime n } ∩ { n | n > 2 } ⊆ { n | ¬Even n } := by sorry
example (h₀ : ∀ x ∈ t, ¬Even x) (h₁ : ∀ x ∈ t, Prime x) : ∀ x ∈ s, ¬Even x ∧ Prime x := by sorry
example (h : ∃ x ∈ s, ¬Even x ∧ Prime x) : ∃ x ∈ t, Prime x := by sorry
example : (s ∪ ⋂ i, A i) = ⋂ i, A i ∪ s := by sorry   -- 提示：一个方向需要经典逻辑
example : (⋃ p ∈ primes, { x | x ≤ p }) = univ := by sorry
```
**S02_Functions.lean（约 28 处 `sorry`）**，代表性的：
```lean
example : f '' s ⊆ v ↔ s ⊆ f ⁻¹' v := by sorry
example (h : Injective f) : f ⁻¹' (f '' s) ⊆ s := by sorry
example : f '' (f ⁻¹' u) ⊆ u := by sorry
example (h : Surjective f) : u ⊆ f '' (f ⁻¹' u) := by sorry
example : f '' (s ∩ t) ⊆ f '' s ∩ f '' t := by sorry
example (h : Injective f) : f '' s ∩ f '' t ⊆ f '' (s ∩ t) := by sorry
example : f '' s ∩ v = f '' (s ∩ f ⁻¹' v) := by sorry
example : (f '' ⋃ i, A i) = ⋃ i, f '' A i := by sorry
example : (f '' ⋂ i, A i) ⊆ ⋂ i, f '' A i := by sorry
example : (f ⁻¹' ⋂ i, B i) = ⋂ i, f ⁻¹' B i := by sorry
example : InjOn sqrt { x | x ≥ 0 } := by sorry
example : (range fun x ↦ x ^ 2) = { y : ℝ | y ≥ 0 } := by sorry
example : Injective f ↔ LeftInverse (inverse f) f := by sorry
example : Surjective f ↔ RightInverse (inverse f) f := by sorry
```
**S03**：`schroeder_bernstein` 本身是完整给出的；但 `sb_right_inv`（3 个）、`sb_injective`（3 个）、`sb_surjective`（1 个）在**正文里就带 `sorry`**。

### (e) 完整性
- **练习文件本体就是书的正文**，练习用 `sorry` 占位——这是 MIL 的设计，不是缺陷；`solutions/` 三份解答文件**确实存在**（已验证），所以题目有解。
- 但正文内部**确实含有未完成的洞**：S02 的 Cantor 证明中间有 `have h₂ : j ∈ S  sorry` / `have h₃ : j ∉ S  sorry`，并带注释 `-- COMMENTS: TODO: improve this`；S03 的三个 `sb_*` 引理共 7 个 `sorry`。
- MIL README 自述："The textbook and this repository are still a work in progress."（[README](https://raw.githubusercontent.com/leanprover-community/mathematics_in_lean/master/README.md)）

---

## 2. Theorem Proving in Lean 4 (TPIL) — 部分已验证，关键前提被证伪

### (a) URL
- Lean 4 版：https://leanprover.github.io/theorem_proving_in_lean4/ （页面自述针对 **Lean 4.33.0**）
- Lean 3 旧版：https://leanprover.github.io/theorem_proving_in_lean/ （标题即 *Theorem Proving in Lean 3 (outdated)*）

### (b) 章节标题/编号
TPIL4 全部 12 章（首页目录原文）：1 Introduction / 2 Dependent Type Theory / 3 Propositions and Proofs / 4 **Quantifiers and Equality** / 5 Tactics / 6 **Interacting with Lean** / 7 Inductive Types / 8 Induction and Recursion / 9 Structures and Records / 10 Type Classes / 11 The Conversion Tactic Mode / 12 Axioms and Computation。

- **没有任何一章叫 "Sets"、"Sets and Functions" 或类似名称** → 你提到的"TPIL 有集合章"**未能验证（且证据表明不存在）**。
- 最接近的两章都没有集合小节：
  - 第 4 章小节：4.1 The Universal Quantifier / 4.2 Equality / 4.3 Calculational Proofs / 4.4 The Existential Quantifier / 4.5 More on the Proof Language / 4.6 Exercises。
  - 第 6 章小节：6.1 Messages … 6.11 Using the Library … 6.15 Named Arguments（[第 6 章](https://leanprover.github.io/theorem_proving_in_lean4/Interacting-with-Lean/)）。
- TPIL3 的 11 章同样无集合章；只有 6.11 *Using the Library* 谈库的使用。

### (c)(d) 讲了什么 / 有什么练习
- 集合、函数（image/preimage）、集合上的练习：**TPIL 全书都没有** → 该条 **未能验证（内容不存在）**。
- 第 1 章 *About this Book* 自述定位是"教你写 Lean 证明"而不是数学专题（[Introduction](https://leanprover.github.io/theorem_proving_in_lean4/Introduction/)）。

### (e) "TPIL 用 Lean core `Set` 而非 Mathlib" —— 对一半，错一半
- **"不用 Mathlib"这半句：已验证**。书的构建清单 `book/lakefile.toml` 里 `[[require]]` **只有 `verso`**，没有 mathlib：
  https://raw.githubusercontent.com/leanprover/theorem_proving_in_lean4/master/book/lakefile.toml
- **"讲 `Set`"这半句：未能验证**，因为书中不存在集合内容。正确表述应是：**TPIL 不教集合**，集合/函数的教学章节在 MIL 第 4 章与 Logic and Proof 第 11/12/15/16 章。

---

## 3. The Mechanics of Proof（Heather Macbeth）— 已验证

### (a) URL
https://hrmacbeth.github.io/math2001/ ；代码仓库 https://github.com/hrmacbeth/math2001

### (b) 抓取到的完整目录（页面原文，逐条照录）
```
Preface
  About this book / Why Lean? / Contents and prerequisites / Note for instructors / Acknowledgements
1. Proofs by calculation
   1.1 Proving equalities  1.2 Proving equalities in Lean  1.3 Tips and tricks
   1.4 Proving inequalities  1.5 A shortcut
2. Proofs with structure
   2.1 Intermediate steps  2.2 Invoking lemmas  2.3 "Or" and proof by cases
   2.4 "And"  2.5 Existence proofs
3. Parity and divisibility
   3.1 Definitions; parity  3.2 Divisibility  3.3 Modular arithmetic: theory
   3.4 Modular arithmetic: calculations  3.5 Bézout's identity
4. Proofs with structure, II
   4.1 "For all" and implication  4.2 "If and only if"  4.3 "There exists a unique"
   4.4 Contradictory hypotheses  4.5 Proof by contradiction
5. Logic
   5.1 Logical equivalence  5.2 The law of the excluded middle  5.3 Normal form for negations
6. Induction
   6.1 Introduction  6.2 Recurrence relations  6.3 Two-step induction  6.4 Strong induction
   6.5 Pascal's triangle  6.6 The Division Algorithm  6.7 The Euclidean algorithm
7. Number theory
   7.1 Infinitely many primes  7.2 Gauss' and Euclid's lemmas  7.3 The square root of two
8. Functions
   8.1 Injectivity and surjectivity  8.2 Bijectivity  8.3 Composition of functions  8.4 Product types
9. Sets
   9.1 Introduction  9.2 Set operations  9.3 The type of sets
10. Relations
   10.1 Reflexive, symmetric, antisymmetric, transitive  10.2 Equivalence relations
Index of Lean tactics
Transitioning to mainstream Lean
```

### 集合章的细目（来自 [第 9 章](https://hrmacbeth.github.io/math2001/09_Sets.html)，页面锚点原文）
- 9.1 Introduction：9.1.1–9.1.9 Example，9.1.10 Exercises
- 9.2 Set operations：9.2.1 Example (`#union`)、9.2.2/9.2.3/9.2.4/9.2.5/9.2.6/9.2.7 Example、9.2.8 Exercises
- 9.3 The type of sets：9.3.1 Definition、9.3.2–9.3.5 Example、9.3.6 Exercises

### (c) 讲了/证了什么
- 9.1：`Set` 由谓词给出；`∈`；`Set.Subset` 的定义 `def Set.Subset (U V : Set α) : Prop := ∀ ⦃x⦄, x ∈ U → x ∈ V`；`⊆`、`⊈`、集合外延 `ext`、集合不等（给反例）、`{1,2,3}` 记法；例题含 `{a : ℕ | 4 ∣ a} ⊆ {b : ℕ | 2 ∣ b}`、`{x : ℤ | Int.Odd x} = {a : ℤ | ∃ k, a = 2*k - 1}`、`{x : ℝ | x^2 - x - 2 = 0} = {-1, 2}`。
- 9.2：∪ / ∩ / 补 `ᶜ` / 空集 / `univ` 的定义与例题；`{n : ℤ | n ≡ 1 [ZMOD 5]} ∩ {n : ℤ | n ≡ 2 [ZMOD 5]} = ∅`。
- 9.3：把 `Set X` 本身当作一个类型（幂集视角）。

### (d) 练习风格（这是本书最有辨识度的部分）
1. **"证明或证伪"成对出题**：每个练习都给两个 `example`，一正一反，学习者自己判断哪个可证：
```lean
example : 4 ∈ {a : ℚ | a < 3} := by sorry
example : 4 ∉ {a : ℚ | a < 3} := by sorry
example : {a : ℕ | 20 ∣ a} ⊆ {x : ℕ | 5 ∣ x} := by sorry
example : {a : ℕ | 20 ∣ a} ⊈ {x : ℕ | 5 ∣ x} := by sorry
example : {n : ℤ | Even n} = {a : ℤ | a ≡ 6 [ZMOD 2]} := by sorry
example : {n : ℤ | Even n} ≠ {a : ℤ | a ≡ 6 [ZMOD 2]} := by sorry
```
2. **"Example" 区块 + 双语调（散文证明 ↔ Lean 代码）**，每个 Problem 后面紧跟 Solution；正文例题即解答。
3. **定制的极简 tactic 方言**：`numbers`（= norm_num）、`extra`、`cancel`、`rel`、`exhaust`（命题逻辑穷举，集合章大量使用），配合 `dsimp [Set.subset_def]`、`push_neg`、`use`、`obtain`、`calc ... := by ring`、`ext x`、`interval_cases`。作者在脚注里明说全书按名调用的引理不到 50 条，并附 *Transitioning to mainstream Lean* 迁移指南。
4. 8 章函数：`def Injective (f : X → Y) : Prop := ∀ {x1 x2 : X}, f x1 = f x2 → x1 = x2`、`Surjective`、`Bijective`、`∃!` 刻画、`Musketeer` 有限归纳类型做穷举例子。

### (e) 完整性
- 前言原文："Over two hundred problems appear with solutions as examples in the text, and **several hundred more problems appear without solution as exercises** for the reader."（[Preface](https://hrmacbeth.github.io/math2001/00_Introduction.html)）
- 仓库对应文件 `Math2001/09_Sets/{01_Sets.lean, 02_Set_Operations.lean, 03_Powerset.lean}` 已取证存在；我读了 `01_Sets.lean`：**正文例题是完整解答，练习部分原样保留 `sorry`**（没有官方练习答案）。
- 正文本身**无 `sorry`**（除练习外），是完整可编译的。

---

## 4. Logic and Proof — 已验证（但 URL、作者、Lean 版本需更正）

### (a) URL
- **现行 URL**：https://leanprover-community.github.io/logic_and_proof/ （页脚 *Logic and Proof 3.18.4 documentation*）
- 旧 URL https://leanprover.github.io/logic_and_proof/ 已 **404 / 跳转**（实测返回 "Redirecting to lean-lang.org"）。

### (b) 集合章的确切标题/编号（页面原文）
- **11. Sets** — 11.1 Elementary Set Theory / 11.2 Calculations with Sets / 11.3 Indexed Families of Sets / 11.4 Cartesian Product and Power Set / 11.5 Exercises（[sets.html](https://leanprover-community.github.io/logic_and_proof/sets.html)）
- **12. Sets in Lean** — 12.1 Basics / 12.2 Some Identities / 12.3 Indexed Families / 12.4 Power Sets / 12.5 Exercises（[sets_in_lean.html](https://leanprover-community.github.io/logic_and_proof/sets_in_lean.html)）
- 另有 **15. Functions**（15.1 The Function Concept … 15.5 Exercises）与 **16. Functions in Lean**（16.1 … 16.5 Functions and Sets in Lean，16.6 Exercises）。

### (c) 它证明/讨论了什么
- 第 11 章是**纯散文数学**：Cantor 的集合定义、Russell 悖论、`∅`/`𝒰`/`∪`/`∩`/补/差/`⊆` 的符号化定义、把集合命题翻译成一阶逻辑；用自然演绎影子讲解 `A ∩ (B ∪ C) = (A ∩ B) ∪ (A ∩ C)`、`(A \ B) \ C = A \ (B ∪ C)`；11.2 用**计算式证明（布尔代数恒等式表）**证 `(A ∩ B̄) ∪ B = A ∪ B` 与 `(A \ B) ∪ (B \ A) = (A ∪ B) \ (A ∩ B)`；11.3 索引族 ∪/∩ 并证 `A ∩ ⋃ᵢ Bᵢ = ⋃ᵢ (A ∩ Bᵢ)`；11.4 有序对 `(a,b) = {{a},{a,b}}` 与 Kuratowski 配对定理、笛卡尔积、幂集。
- 第 12 章给出 Lean 版：`Set U`、`x ∈ A`、`#check A ∪ B` 等，并给出两种证明套路——term mode 的 `fun x ↦ fun (h : x ∈ A) ↦ ...` 与 tactic mode 的 `intro`/`show`；等式用 `eq_of_subset_of_subset` 或 `ext x`（`Set.ext`）；`x ∈ A ∩ B` 与 `x ∈ A ∧ x ∈ B` **定义相等**；`mem_inter`、`mem_union_left`、`not_mem_empty`、`absurd`；索引并交用 `mem_iUnion`/`mem_iInter`，并给出 `notation3 "⋃ "` 的自定义记法；12.2 用 `calc` + `rw [inter_union_distrib_right]`/`compl_union_self`/`inter_univ` 重证 `(A ∩ Bᶜ) ∪ B = A ∪ B`。
- 11.5 的 13 道练习（散文证明），例如："Prove that A ∪ (B ∩ C) = (A ∪ B) ∩ (A ∪ C)"、"Prove that A̅ \ B̅ = A̅ ∪ B"、"Prove that A × (B ∪ C) = (A × B) ∪ (A × C)"、"Prove that A ⊆ B if and only if 𝒫(A) ⊆ 𝒫(B)"。

### (d)(e) 练习与完整性
- 该书的 Lean 代码片段以 `sorry` 占位来标注"待你填"（12.1 的 `show x ∈ B from sorry` 等），页面提供 **"try it!"** 链接跳转到 live.lean-lang.org 可直接运行。
- **版本重要提示**：`courses.yaml` 把以本书为教材的 VU Amsterdam *Logic and Modelling*（2023）标为 `lean_version: 3`；书内语法也是 Lean 3 风格（`cases hx with | inl h =>`）。但页面的 "try it" 片段已改为 Lean 4 + `import Mathlib.Data.Set.Basic`。这一点对"Lean 4 教学材料"的定位要打折。
- 12.1 有一句很关键的论断（可用作我们课程的对照）："Basic set-theoretic notions like these are **defined in Lean's core library**, but additional theorems and notation are available in an auxiliary library that we have loaded with the command `import Mathlib.Data.Set.Basic`."

---

## 5. Formalising Mathematics（Kevin Buzzard / Bhavik Mehta）— 部分已验证，2025 版已换作者

### 2024 版：已验证
- 笔记：https://www.ma.imperial.ac.uk/~buzzard/xena/formalising-mathematics-2024/ （*Formalising Mathematics 0.1 documentation*，Kevin Buzzard；Part A the mathematics / Part B Lean tips / Part C Tactics）
- 仓库：https://github.com/ImperialCollegeLondon/formalising-mathematics-2024 — **存在**。GitHub API 显示：description "Formalising Mathematics; a course for undergraduate mathematicians. Ran between January and March 2024."，默认分支 `main`，`homepage` 即上面笔记 URL，`archived: false`，`pushed_at: 2025-03-18`。
- **讲义/演示文件真实名字**（API 列目录取证）：`FormalisingMathematics2024/` 下有
  - `Section01logic`、`Section02reals`、**`Section03functions`**、**`Section04sets`**、`Section05groups`、`Section06orderingsAndLattices`、`Section07subgroupsAndHomomorphisms`、**`Section08finiteness`**、**`Section09bijectionsAndIsomorphisms`**、`Section10TopologicalSpaces`、`Section11vectorSpaces`、`Section12Filters`、`Section13measureTheory`、`Section14UFDsAndPIDsEtc`、`Section15numberTheory`、`Section16commutativeAlgebra`、`Section17curvesAndSurfaces`、`Section18graphTheory`、`Section19algebraicNumberTheory`、`Section20representationTheory`、`Section21galoisTheory`，外加 **`Solutions/`**。
  - `Section04sets/` = `Sheet1.lean`(3018B)、`Sheet2.lean`、`Sheet3.lean`、`Sheet4.lean`、`Sheet5.lean`、`Sheet6.lean`
  - `Section03functions/` = `Sheet1.lean`(5081B)、`Sheet2.lean`、`Sheet3.lean`
  - （与基数/有限性相关：`Section08finiteness`、`Section09bijectionsAndIsomorphisms`）
- `Section04sets/Sheet1.lean` 开头原文：`# Sets in Lean, sheet 1 : ∪ ∩ ⊆ and all that`，`namespace Section4sheet1`，先证 `subset_def`/`mem_union_iff`/`mem_inter_iff` 都用 `rfl`（因定义相等），随后是练习：
```lean
example : A ⊆ A := by sorry
example : A ⊆ B → B ⊆ C → A ⊆ C := by sorry
example : A ⊆ A ∪ B := by sorry
example : A ∩ B ⊆ A := by sorry
example : A ⊆ B → A ⊆ C → A ⊆ B ∩ C := by sorry
example : B ⊆ A → C ⊆ A → B ∪ C ⊆ A := by sorry
example : A ⊆ B → C ⊆ D → A ∪ C ⊆ B ∪ D := by sorry
example : A ⊆ B → C ⊆ D → A ∩ C ⊆ B ∩ D := by sorry
```
（`Section04sets/Sheet1.lean` 的 raw URL 已在本次取证中抓取成功。）

### 2025 版：**你给的两个 URL 都不存在**
- https://www.ma.imperial.ac.uk/~buzzard/xena/formalising-mathematics-2025/ → **HTTP 300**，服务器列出可用文档只有 **2022 / 2023 / 2024**，没有 2025 → **未能验证（不存在）**。
- https://github.com/ImperialCollegeLondon/formalising-mathematics-2025 → GitHub API **404** → **仓库不存在**。

### 当前版本 = Bhavik Mehta 的 formalising-mathematics-notes：已验证
- 2024 仓库的 README **首段**原文："These are course notes for a course given in 2024; the more up-to-date 2025 version of the course repository is [here](https://github.com/b-mehta/formalising-mathematics-notes) and the more up-to-date version of the course notes are [here](https://b-mehta.github.io/formalising-mathematics-notes/)."
- `courses.yaml` 中 "Formalizing Mathematics 2024" 条目也写着 "Note that the [2025 version](https://github.com/b-mehta/formalising-mathematics-notes) of the course uses a more up-to-date version of mathlib!"。
- 笔记站：https://b-mehta.github.io/formalising-mathematics-notes/ ，页脚 *© Copyright 2025, Bhavik Mehta*，自述 "Written by Bhavik Mehta, for his Formalising Mathematics course, adapted from an earlier handbook by Kevin Buzzard."；结构为 Introduction / Installing Lean / Part 1: Lean tips / Part 2: Tactics。
- 仓库 https://github.com/b-mehta/formalising-mathematics-notes 的代码目录已改名为 **`FormalisingMathematics2026/`**，其中同样有 `Section03functions`、**`Section04sets/Sheet1..Sheet6.lean`**、`Section08finiteness`、`Section09bijectionsAndIsomorphisms`、`Section10types`、`Section21combinatorics`、`Solutions/` 等（API 取证）。

---

## 6. 高校 Lean 4 集合/证明课程 — 逐条取证

> 取证路径说明：`web_search` 本会话不可用；下列结论来自社区课程总表 [teaching/courses.html](https://leanprover-community.github.io/teaching/courses.html) 与它的数据源 [courses.yaml](https://raw.githubusercontent.com/leanprover-community/leanprover-community.github.io/lean4/data/courses.yaml)（含官网/repo 链接），再对链接逐个直接抓取。

### 6.1 最贴合本主题：Université Gustave Eiffel *Logique et preuve assistée*（2025, Lean 4）— 已验证
- 数据源条目原文："This is a two-part course for a small group of first-year double-major math & CS students… The second part is a **Lean 4 lab with worksheets on propositional and predicate logic, sets and functions, and natural numbers**. The focus in on elementary theorems proved in tactics-mode in **"plain vanilla" Lean (without the Mathlib)**, attempting to draw parallels with natural deduction."
- 仓库：https://github.com/niotie/logique-preuve-assistee （默认分支 `main`，`lakefile.lean` 仅 222B，`lake-manifest.json` 仅 121B → **无 Mathlib 依赖**，与描述一致）
- 集合/函数讲义（真实文件名，API 取证）：`LPA/TP3EnsemblesFonctions.lean` 依次 `import` 五个模块：
  `LPA/TP3EnsemblesFonctions/1SetDefinitions.lean`、`2SetProperties.lean`、`3FunctionsDefinitions.lean`、`4FunctionProperties.lean`、`5InjectivitySurjectivity.lean`
- **`1SetDefinitions.lean` 从零把集合搭出来**（这是本调研里最接近"纯 Lean 4 内核 + 集合教学"的材料）：
```lean
def Set (α : Type u) := α → Prop
def Mem (s : Set α) (a : α) : Prop := s a
instance : Membership α (Set α) where mem := Set.Mem
def Subset (s₁ s₂ : Set α) := ∀ ⦃a⦄, a ∈ s₁ → a ∈ s₂
@[ext] theorem ext {a b : Set α} (h : ∀ (x : α), x ∈ a ↔ x ∈ b) : a = b := by
  funext x; apply propext; exact h x
```
  还包括 `∅`（`EmptyCollection`）、`univ`（`fun _ ↦ True`）、`union`/`inter`/`compl`（`ᶜ`）/`diff`（`\`）/`powerset`（`𝒫`）/`singleton`/`insert`，全部配 `<op>_def` 引理（多为 `rfl`）。
- **`5InjectivitySurjectivity.lean` 的练习全部是 `sorry`**，且题面设计巧妙（"四个命题里只有两个为真，证明真的、给假的找反例"），例如：
```lean
theorem inj_comp (h1 : injective f) (h2 : injective g) : injective (g ∘ f) := by sorry
theorem surj_comp (h1 : surjective f) (h2 : surjective g) : surjective (g ∘ f) := by sorry
theorem inj_iff_eq_preimage_image : injective f ↔ ∀ s, f ⁻¹' (f '' s) ⊆ s := by sorry
theorem inj_iff_inter_image_sub_image_inter :
    injective f ↔ ∀ s s', f '' s ∩ f '' s' ⊆ f '' (s ∩ s') := by sorry
theorem surj_iff_exists_right_inverse : surjective f ↔ ∃ f', f ∘ f' = id := by sorry
  -- 注释原文：L'une des directions utilise `Classical.choose` et `Classical.choose_spec`
```
  并有被注释掉的"假命题"骨架（`example ... := by fail`），明确要求学习者找反例。

### 6.2 Stockholm University, *Logic II: Computability, Set Theory, and Model Theory*（2025, Lean 4）— 已验证，但**注意陷阱**
- 课程页：https://sinhp.github.io/teaching/2025-logic2-stockholm/ （Sina Hazratpour，Masters level）
- 三大块：computability & incompleteness / axiomatic foundations (ZFC, ordinals, cardinals…) / model theory；教材是 Cori–Lascar 与 Avigad 的 *Mathematical Logic and Computation*。
- 页面原文关键限定："**Lean is used as a digital diary for the first part of the course**（即 computability）。" → **集合论那部分并没有用 Lean 形式化**。Lean 代码在 https://github.com/sinhp/CompLean/ 。
- 结论：这是"有集合论内容的 Lean 4 课程"，但**不是"用 Lean 教集合"**。

### 6.3 University of Dayton, *Math 342: Set theory and Logic*（Spring 2023, Lean 3）— 已验证
- 课程页：https://lijungeometry.github.io/342.html （Jun Li，T Th 11:00–12:30）
- 20 次课的表列（真实标题）：1 Overview / **2 Set Operations** / 3 Logic / 4 Model Theory / **5 Functions and Relations** / **6 Cantor's Theorem** / 7 Real Number Axioms / 8 Lambda Calculus / 9 Induction / 10 Interactive Proof / 11 Tarski's Fixed-Point Theorem / 12 Well-Founded Induction / 13 Computability vs. Diagonal Argument / 14 Type Theory / **15 Lean（链接到 leanprover.github.io/tutorial）** / 16–17 "Stander Poster"（Lean Math Library、More Lean with Kevin Buzzard）/ 18–20 Gödel。
- 课程目标原文："We'll make use of those in a project on Lean math prover." → 集合是主线，Lean 作为项目工具；`courses.yaml` 标 `lean_version: 3`。

### 6.4 Universitat de València（Enric Cosme Llópez）— 已验证（两份材料）
- **Taller de Lean 4**：https://www.uv.es/coslloen/Lean4.html ，S0–S9 共 10 次课（Primeres definicions / Proposicions / Quantificadors / Igualtats i aplicacions / Subtipus i tipus quocients / Tipus producte i suma / Tipus inductius / Operadors clausura / L'ordre habitual dels naturals / Estructures i classes），课程页自述聚焦"an advanced course on **set theory and formal logic**"的常见结构；代码 https://github.com/encosllo/TallerLean4 。**注意页面是加泰罗尼亚语**，且没有单独的"Sets"章节名，我**未能验证**其中有专门的集合讲义。
- **教材《An Introduction to Lean 4》**（Enric Cosme Llópez & Lü Gong）：https://www.uv.es/coslloen/Lean4/ ，15 章，纯 Lean 4 core（无 Mathlib），每章含练习；**第 5 章 Functions**（https://www.uv.es/coslloen/Lean4/Leancap05.html ）给出 `injective`/`monomorphism`/`hasleftinv`/`surjective`/`epimorphism`/`hasrightinv`/`bijective`/`isomorphism` 的完整定义与例子，练习全部 `by sorry`（如 `theorem TCompInj ... : injective (g ∘ f) := by sorry`、`theorem TCarEpiSurj : surjective f ↔ epimorphism f := by sorry`）；**但没有集合章**（最接近的是第 8 章 Subtypes、第 9 章 Relations）。练习解答在 https://github.com/encosllo/IntroToLean4/ 。

### 6.5 其它已取证但非"集合专题"的课程（供背景参考）
| 机构 | 课程 / 人 | 年份 / Lean | 与集合的关系 | URL |
|---|---|---|---|---|
| CMU | Topics in formal mathematics（Patrick Massot） | 2023 / L4 | 用 MIL 走大半学期 → 间接覆盖 MIL 第 4 章 | [courses.yaml 条目](https://raw.githubusercontent.com/leanprover-community/leanprover-community.github.io/lean4/data/courses.yaml)（`repo: mathematics_in_lean`） |
| CMU | Logic and Mechanized Reasoning（Jeremy Avigad） | 2021 / L4 | 逻辑为主 | 同上（courses.yaml） |
| CMU | Interactive Theorem Proving（Avigad） | 2022 / L3 | 逻辑为主 | 同上 |
| Bonn | Formalized Mathematics in Lean（Floris van Doorn） | 2023、2024 / L4 | 用 MIL；仓库 https://github.com/fpvandoorn/LeanCourse23 、https://github.com/fpvandoorn/LeanCourse24 | 同上 |
| Düsseldorf | Computergestützte Beweisführung（Bentkamp, Eugster） | 2023 / L4 | 自述"little mathematical content"、只用 term mode → 不覆盖集合 | [课程页](https://www.math.uni-duesseldorf.de/~internet/CB-V-W23/) |
| Stanford | CS99 Functional Programming and Theorem Proving in Lean 4 | 2025 / L4 | 内容清单里明确含 "Logic, **Sets**, Computability" | [课程页](https://web.stanford.edu/class/cs99/) |
| Boston University | CS511 Formal Methods for High-Assurance SE（Assaf Kfoury） | 2025 / L4 | 教材用 TPIL4（无集合） | [课程页](https://sites.google.com/bu.edu/cs511-fall-2024/home) |
| Boston University | Combinatoric Structures（Assaf Kfoury） | 2025 / L4 | 教材 = MoP → 覆盖 MoP 第 8/9 章 | courses.yaml（`material: hrmacbeth.github.io/math2001/`） |
| Greifswald | Mathematical Proofs with Computers / Mathematics meets Computer（Nima Rasekh） | 2025、2026 / L4 | 自述按 MIL 讲 "the libraries in Mathlib (**set theory**, number theory, algebra)" → 覆盖 MIL 第 4 章 | courses.yaml |
| JHU | Computer-Verified Proof（Emily Riehl） | 2025 / L4 | 用 Lean Game Server | https://adam.math.hhu.de/#/g/emilyriehl/ReintroductionToProofs |
| Rutgers | An Introduction to (Formal) Real Analysis（Alex Kontorovich） | 2025 / L4 | 点集拓扑为主，游戏化 | https://adam.math.hhu.de/#/g/AlexKontorovich/RealAnalysisGame |
| ENS Paris / Lyon 等 | Nuccio 的多门 Lean 入门课 | 2025、2026 / L4 | 通用入门 | courses.yaml（M1_ENS_26 / GradCourse25-26） |

### 6.6 明确 **未能验证** 的项（没有编造）
- **Imperial College London "MATH40001"** 或任何 Imperial 课程号：**未能验证**。Imperial 在课程总表里只有 "Formalizing Mathematics 2024（Kevin Buzzard）"，**没有课程号**。
- **Cambridge**：未找到任何 Lean 4 集合论课程页面 → **未能验证**。
- **University of Waterloo**：同上 → **未能验证**。
- **Chapman University / Kevin Sullivan**：同上 → **未能验证**。
- **CMU "15-xxx" Lean 课程号**：课程总表里 CMU 的四门课都没有给出课程号 → **未能验证**。
- **Lean Game Server 的 "Set Theory" 游戏**：`learn.html` 只说 "The Lean Game Server hosts various learning games **including Set Theory, Logic, and Robo**"，**没有给链接**；我未能取到该游戏的具体 URL/名称 → **未能验证**。

---

## 7. Mathlib 文档 / Guides 里教集合的页面 — 已验证

| 页面 | URL | 教学相关性（页面原文/实际内容） |
|---|---|---|
| Learning Lean 4（学习资源总入口） | https://leanprover-community.github.io/learn.html | 书架推荐：MIL（"standard mathematics-oriented reference"）、**MoP**（"gentler pace… aimed at readers with less mathematical experience"）、TPIL（"foundations of type theory"）、Hitchhiker's Guide；动手：NNG4、**Lean Game Server（含 Set Theory 游戏，无链接）**、GlimpseOfLean、tactic cheatsheet；以及 API 文档、参考手册、元编程书 |
| Teaching with Lean（教学总入口） | https://leanprover-community.github.io/teaching/index.html | 三分页：**courses / resources / practices**；MoP 前言把 `https://leanprover-community.github.io/teaching/` 推荐为"搭建这类课程基础设施"的地方 |
| Courses using Lean | https://leanprover-community.github.io/teaching/courses.html | 可筛选的课程总表（lean4/lean3、beginner/advanced、语言标签…），第 6 节的数据来源 |
| Maths in Lean: **Sets and set-like objects** | https://leanprover-community.github.io/theories/sets.html | 专门讲集合类对象：**Lists**（`Mathlib.Data.List.Basic`）、**Multisets**、**Finsets**、**Sets and subtypes**（`Mathlib.Data.Set.Basic`；`Set α` 是谓词 `α → Prop`；subtype `{n : ℕ // 4 ≤ n}` 对比）、**Finite types**（`Fintype`）、**Finite sets**（`Mathlib.Data.Set.Finite`；与 `Fin n` 双射）、**Cardinals**（`Finset.card` / `Fintype.card` / `Multiset.card`、`Mathlib.SetTheory.Cardinal.Basic`）。无习题，是 API 导览 |
| A mathlib overview | https://leanprover-community.github.io/mathlib-overview.html | 有 "Data structures → **Sets**: set / finite set / multiset / ordered set" 与 "Logic and computation → **Set theory**: Ordinals / Cardinals / model of ZFC" 的指针 |
| Undergraduate mathematics in mathlib | https://leanprover-community.github.io/undergrad.html | **注意：按法国大纲组织，没有"集合与函数"专题**；分区是线性代数、群论、环论、双线性/二次型、仿射与欧氏几何、单变量实分析、单变量复分析、拓扑、多元微积分、测度与积分、概率、分布论、数值分析。集合只在零散条目里出现（如 `Set.finrank`） |
| Mathlib API docs | https://leanprover-community.github.io/mathlib4_docs/ | 首页仅 "Welcome to the documentation page / This was built using Lean 4 **4.35.0-rc2**"。它是 **API 索引，不是教学页**；`learn.html` 称其"是现存最接近综合参考手册的东西" |

---

## 8. 一页速查表

| # | 材料 | 集合章 | 函数章 | 练习带 `sorry` | 官方解答 | Lean |
|---|---|---|---|---|---|---|
| 1 | MIL | **4. Sets and Functions**（4.1 Sets / 4.2 Functions / 4.3 Schröder-Bernstein） | 同章 4.2 | 是（约 40+ 处） | **有**，`.../solutions/Solutions_S0*.lean` | L4 + Mathlib |
| 2 | TPIL | **无** | 无（只讲函数类型） | — | — | L4 core（只依赖 verso） |
| 3 | MoP | **9. Sets**（9.1/9.2/9.3） | **8. Functions**（8.1–8.4） | 是（练习） | 正文例题有解；练习无解 | L4 + 定制方言 |
| 4 | Logic and Proof | **11. Sets** / **12. Sets in Lean** | **15. Functions** / **16. Functions in Lean** | 是（代码片段占位） | 无 | Lean 3（书）／片段已迁 L4 |
| 5 | FM 2024 | `Section04sets/Sheet1–6.lean` | `Section03functions/Sheet1–3.lean` | 是 | **有** `Solutions/` | L4 + Mathlib |
| 5' | FM 2025/2026（Mehta） | `FormalisingMathematics2026/Section04sets/Sheet1–6.lean` | `Section03functions` | 是 | 有 `Solutions/` | L4 + Mathlib |
| 6 | LPA（Gustave Eiffel） | `LPA/TP3EnsemblesFonctions/1,2` | 同目录 `3,4,5` | 是 | **未能验证**（未见解答） | **L4，无 Mathlib，从零定义 Set** |
| 6' | UV《An Introduction to Lean 4》 | 无（有 Subtypes/Relations） | **第 5 章 Functions** | 是 | **有**（IntroToLean4 仓库） | L4 core |

## 9. 对我们（sokonanoda）最直接的三点

1. **唯一"从零定义 Set、不用 Mathlib、纯 Lean 4 内核、带 sorry 习题"的公开教学材料是 Gustave Eiffel 的 `LPA/TP3EnsemblesFonctions`**（`1SetDefinitions.lean` 用 `def Set (α : Type u) := α → Prop` + `Membership`/`HasSubset` 实例 + `propext`/`funext` 证 `ext`）。它与我们的"教学语法是真实 Lean 4 子集"路线最近，且证明这条路不需要 Mathlib 也能教到 image/preimage/单射满射刻画。
2. **想覆盖 image/preimage + 单射/满射刻画的现成题库**，密度最高的是 MIL `S02_Functions.lean`（约 28 题）与 LPA `5InjectivitySurjectivity.lean`（约 30 题，含"四个命题只有两个为真"的反例训练）。MoP 第 8/9 章的风格是"证明或证伪"成对出题，值得直接借鉴题型。
3. **MIL 第 4 章是 Mathlib 依赖最重的一份**（`Mathlib.Data.Set.Lattice`、`Set.Function`、`Analysis.SpecialFunctions.Log.Basic`），其 `sorry` 甚至出现在正文（Cantor、Schröder-Bernstein）；若我们要做零依赖课程，MIL 只能当题面来源，不能当语法来源。
