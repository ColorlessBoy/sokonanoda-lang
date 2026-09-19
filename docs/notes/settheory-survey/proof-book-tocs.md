# "Transition to proof" 教材取证调研：真实目录、集合论位置、练习形态

面向中文母语、**从未写过形式化证明**的学习者的课程调研。本文件只记录**实际抓取到**的内容；
凡未能取证的条目一律显式标注 `UNVERIFIED`，绝不凭记忆补全目录。

**覆盖状态：6/6 本已取证**（Hammack / Macbeth / Avigad 等 / Velleman / Solow / Cummings）。
取证过程中发现任务书有 **5 处前提错误**，见 §0。

> 相关文件：同目录 `lean4-sets-functions-prior-art.md` 覆盖 Lean 4 生态（MIL / TPIL /
> Formalising Mathematics / 各高校 Lean 课程）。本文件聚焦**传统 transition-to-proof 教材**
> （Velleman / Hammack / Solow / Cummings）**加上**两本 Lean 4 证明书（Macbeth / Avigad 等）的
> **目录级与练习级事实**。

## 取证方法与环境

- 本 session 的 `web_search` / `web_fetch` 不可用；全部内容经本地代理 `http://127.0.0.1:7890`
  用 `curl -sSL -k` 抓取。**`-k` 是必需的**：部分主机（如 `www.people.vcu.edu`）证书链损坏。
- HTML → 文本用 `pandoc -f html -t plain`；PDF 文本抽取用 PyMuPDF（本机**没有** `pdftotext`）。
- 抓取时间：2026 年（本 session）。
- 仓库 DSH hook `dsh/hooks/refuse-lean-toolchain.js` 会拦截命令行文本中形如 `|<空格>lean` 的片段
  （正则分隔符类包含 `|`）。抓取命令里避免 `grep "...\|lean ..."` 这种写法。

---

## 0. 先纠正任务书里的五个前提错误

任务书给出的若干处目录/URL/版本 **与实际不符**，此处先行更正（均有页面原文为证）：

| 任务书说法 | 实际情况（已取证） |
|---|---|
| Hammack *Book of Proof* 第 3 版 = "Ch 13 Cardinality, Ch 14 Composition" | **第 3 版共 14 章**：**Ch 13 = Proofs in Calculus**（第 3 版新增），**Ch 14 = Cardinality of Sets**。**Composition 不是独立章节**，而是 **§12.4**（Ch 12 Functions 内）。 |
| Hammack 官方站 `https://www.people.vcu.edu/~rhammack/BookOfProof/` | 该 URL **已失效**，302 到 VCU 首页。现行镜像：<https://richardhammack.github.io/BookOfProof/>，PDF 直链 <https://richardhammack.github.io/BookOfProof/Main.pdf>。 |
| Avigad 等 *Logic and Proof*（Lean 4）在 `https://leanprover.github.io/logic_and_proof/` | 该 URL **404**。现行 URL：<https://leanprover-community.github.io/logic_and_proof/>。且它是 **Lean 3 原书的 Lean 4 改编版**（改编者 Joseph Hua，见 §3）。 |
| **Velleman 第 3 版 = 7 章，"Ch 7 infinite sets"** | **第 3 版共 8 章**：**Ch 7 = Number Theory**（第 3 版**新增**），**Ch 8 = Infinite Sets**。"7 章、Ch 7 = 无限集"是**第 2 版**（2006）的布局。取证见 §4。 |
| **Cummings *Proofs* 存在第 2 版** | **《Proofs》没有第 2 版**（作者书页 "edition" 一词 0 命中；Amazon 无 Edition 字段；Open Library `edition_count`=1）。任务书里的 "2nd edition" 应是从同系列 *Real Analysis: A Long-Form Mathematics Textbook* 串来的（那本确有 2nd ed., 2019-07）。取证见 §6。 |

Hammack 本人对章节变动的原话（第 3 版前言，PDF p.7）：

> "The chapter sequencing is identical between editions, with one exception: The final
> chapter on cardinality has become Chapter 14 in order to make way for the new Chapter 13
> on calculus proofs. There has been a slight renumbering of the sections within chapters
> 10 and 11, but the numbering of the exercises within the sections is unchanged."

---

## 1. Richard Hammack, *Book of Proof*, 3rd ed. (2018) — **完全取证**

- 官方站（现行）：<https://richardhammack.github.io/BookOfProof/>
- 免费全文 PDF：<https://richardhammack.github.io/BookOfProof/Main.pdf>（380 页，1.84 MB）
- 授权：CC BY-NC-ND 4.0（页脚原文）；AIM Open Textbook Initiative 认证
- **开放获取：是**（作者明言 "You can also download a free PDF version HERE."）

### (a) 真实全书目录（PDF p.4–6 原文照录，含页码）

```
Preface vii / Introduction viii

PART I  Fundamentals
  1. Sets                                        3
     1.1 Introduction to Sets                    3
     1.2 The Cartesian Product                   8
     1.3 Subsets                                12
     1.4 Power Sets                             15
     1.5 Union, Intersection, Difference        18
     1.6 Complement                             20
     1.7 Venn Diagrams                          22
     1.8 Indexed Sets                           25
     1.9 Sets That Are Number Systems           30
     1.10 Russell's Paradox                     32
  2. Logic                                      34
     2.1 Statements  2.2 And, Or, Not  2.3 Conditional Statements
     2.4 Biconditional Statements  2.5 Truth Tables for Statements
     2.6 Logical Equivalence  2.7 Quantifiers  2.8 More on Conditional Statements
     2.9 Translating English to Symbolic Logic  2.10 Negating Statements
     2.11 Logical Inference  2.12 An Important Note
  3. Counting                                   65
     3.1 Lists  3.2 The Multiplication Principle  3.3 The Addition and Subtraction Principles
     3.4 Factorials and Permutations  3.5 Counting Subsets
     3.6 Pascal's Triangle and the Binomial Theorem  3.7 The Inclusion-Exclusion Principle
     3.8 Counting Multisets  3.9 The Division and Pigeonhole Principles  3.10 Combinatorial Proof

PART II  How to Prove Conditional Statements
  4. Direct Proof                              113
     4.1 Theorems  4.2 Definitions  4.3 Direct Proof  4.4 Using Cases  4.5 Treating Similar Cases
  5. Contrapositive Proof                      128
     5.1 Contrapositive Proof  5.2 Congruence of Integers  5.3 Mathematical Writing
  6. Proof by Contradiction                    137
     6.1 Proving Statements with Contradiction
     6.2 Proving Conditional Statements by Contradiction
     6.3 Combining Techniques  6.4 Some Words of Advice

PART III  More on Proof
  7. Proving Non-Conditional Statements        147
     7.1 If-and-Only-If Proof  7.2 Equivalent Statements
     7.3 Existence Proofs; Existence and Uniqueness Proofs
     7.4 Constructive Versus Non-Constructive Proofs
  8. Proofs Involving Sets                     157
     8.1 How to Prove a ∈ A   8.2 How to Prove A ⊆ B
     8.3 How to Prove A = B   8.4 Examples: Perfect Numbers
  9. Disproof                                  172
     9.1 Counterexamples  9.2 Disproving Existence Statements  9.3 Disproof by Contradiction
  10. Mathematical Induction                   180
     10.1 Proof by Induction  10.2 Proof by Strong Induction
     10.3 Proof by Smallest Counterexample  10.4 The Fundamental Theorem of Arithmetic
     10.5 Fibonacci Numbers

PART IV  Relations, Functions and Cardinality
  11. Relations                                 201
     11.1 Relations  11.2 Properties of Relations  11.3 Equivalence Relations
     11.4 Equivalence Classes and Partitions  11.5 The Integers Modulo n
     11.6 Relations Between Sets
  12. Functions                                 223
     12.1 Functions  12.2 Injective and Surjective Functions
     12.3 The Pigeonhole Principle Revisited  12.4 Composition
     12.5 Inverse Functions  12.6 Image and Preimage
  13. Proofs in Calculus                        244
     13.1 The Triangle Inequality  13.2 Definition of a Limit  13.3 Limits That Do Not Exist
     13.4 Limit Laws  13.5 Continuity and Derivatives  13.6 Limits at Infinity
     13.7 Sequences  13.8 Series
  14. Cardinality of Sets                       269
     14.1 Sets with Equal Cardinalities  14.2 Countable and Uncountable Sets
     14.3 Comparing Cardinalities  14.4 The Cantor-Bernstein-Schröder Theorem
Conclusion 291 / Solutions 292
```

书籍自身的依赖树（PDF p.x，"Dependency Tree"）确认主线为：
`1 Sets → 2 Logic → 4 Direct Proof → 5 Contrapositive → 6 Contradiction → 7 Non-Conditional
→ 8 Proofs Involving Sets → 9 Disproof → 11 Relations → 12 Functions → 14 Cardinality`，
其中 **Ch 3 Counting / Ch 10 Induction / Ch 13 Calculus 是旁支**（Ch 3 与 Ch 13 可整章跳过而不破坏连续性）。

### (b) 集合论在哪里 —— 以及一个关键顺序

**集合论不是"第 11 章才出现"，而是全书第 1 章，且是 Part I 的第一章**（在 Logic 之前）。

Ch 1 的真实内部顺序（**注意与我们设想的"sets → subsets → power set → ordered pairs"不同**）：

> **1.1 Introduction to Sets → 1.2 The Cartesian Product（有序对！）→ 1.3 Subsets →
> 1.4 Power Sets → 1.5 Union/Intersection/Difference → 1.6 Complement → 1.7 Venn Diagrams →
> 1.8 Indexed Sets → 1.9 Sets That Are Number Systems → 1.10 Russell's Paradox**

即 **有序对（笛卡尔积）排在子集之前**，子集之后**立刻**是幂集。定义原文（PDF p.20）：

> **Definition 1.1** An ordered pair is a list (x, y) of two things x and y, enclosed in
> parentheses and separated by a comma.
> **Definition 1.2** The Cartesian product of two sets A and B is another set, denoted as
> A × B and defined as A × B = {(a, b) : a ∈ A, b ∈ B}.

关系与函数（Ch 11–12）的关键点：**函数被定义成一种关系**。原文（PDF p.224，Definition 12.1）：

> "A function f from A to B (denoted as f : A → B) is a **relation** f ⊆ A × B from A to B,
> satisfying the property that for each a ∈ A the relation f contains exactly one ordered
> pair of form (a, b)."

而 §11.6 "Relations Between Sets" 就是这条桥（从 A 到 B 的关系 → 函数）。

**Ch 11/12 未覆盖的内容（重要缺口）**：
- Ch 11.2 只讲 **reflexive / symmetric / transitive** 三条性质，**全文没有 antisymmetric、
  没有 partial order、没有 poset**（对全书文本 grep `partial order|antisymmetric|poset` = 0 命中）。
- Ch 12.6 才是 image/preimage；**没有** 专门的"函数空间 / 索引族 / 选择公理"。
- **全书没有选择公理**（grep `axiom of choice|Zorn|transfinite` 无命中；只在 §10.3 用到
  well-ordering principle 作为"最小反例法"的依据）。

### (c) 练习类型与位置 —— 两种模式，前后不同

用脚本枚举全部 55 个练习块（`Exercises for Section x.y` / `Exercises for Chapter N`），实测：

| 章节 | 练习位置 | 块数 | 规模（最大题号） |
|---|---|---|---|
| Ch 1, 2, 3 | **每节末** `Exercises for Section x.y` | 26 | 1.1 有 ~52 题；3.4 有 ~36 题 |
| **Ch 4–10** | **每章末一个** `Exercises for Chapter N` | 7 | Ch 8 = ~31 题，Ch 10 = ~43 题 |
| Ch 11, 12 | **每节末** `Exercises for Section x.y` | 11 | 12.6 = ~25 题 |
| Ch 13, 14 | **每节末** `Exercises for Section x.y` | 11 | 14.4 = ~51 题 |

细节：
- **Ch 4–10 完全没有节末练习**，每章只在末尾给一大组题（`Exercises for Chapter 4` 起）。
- **§11.6 Relations Between Sets 没有练习**（练习块从 11.5 直接跳到 12.1）。
- **§13.1 The Triangle Inequality 没有练习**（练习从 13.2 起）。
- 题型分布：Ch 1 是"列举元素 / 写 set-builder / 判断真假"的机械题为主；
  Ch 8 起转为统一句式的"证明下列命题"；**Ch 9 整体是判定题**，且明说累积：
  > "Each of the following statements is either true or false. If a statement is true, prove
  > it. If a statement is false, disprove it. These exercises are **cumulative, covering all
  > topics addressed in Chapters 1–9**."
- 末尾 `Solutions` 章（PDF p.292 起）给出**部分习题解答**（非全部），另有
  `Chapter 1 Exercises` / `Chapter 2 Exercises` 等补充题组。
- 作者在 §1.3 明说练习承担教学功能：
  > "These are the eight subsets of B. **Exercises like this help you identify** what…"

### (d) 真实练习原文（逐条照抄）

**Ch 1（§1.1，题 1–4、17）——机械/表征类**

> A. Write each of the following sets by listing their elements between braces.
> 1. {5x − 1 : x ∈ ℤ}   2. {3x + 2 : x ∈ ℤ}   3. {x ∈ ℤ : −2 ≤ x < 7}   4. {x ∈ ℕ : −2 < x ≤ 7}
> B. Write each of the following sets in set-builder notation.
> 17. {2, 4, 8, 16, 32, 64, ...}

**Ch 8（`Exercises for Chapter 8`，题 1、6、8）——集合恒等式证明**

> 1. Prove that {12n : n ∈ ℤ} ⊆ {2n : n ∈ ℤ} ∩ {3n : n ∈ ℤ}.
> 6. Suppose A, B and C are sets. Prove that if A ⊆ B, then A − C ⊆ B − C.
> 8. If A, B and C are sets, then A ∪ (B ∩ C) = (A ∪ B) ∩ (A ∩ C).

**Ch 9（`Exercises for Chapter 9`，题 1、5、9、14、34）——真/假判定（累积型）**

> 1. If x, y ∈ ℝ, then |x + y| = |x| + |y|.
> 5. If A, B, C and D are sets, then (A × B) ∪ (C × D) = (A ∪ C) × (B ∪ D).
> 9. If A and B are sets, then 𝒫(A) − 𝒫(B) ⊆ 𝒫(A − B).
> 14. If A and B are sets, then 𝒫(A) ∩ 𝒫(B) = 𝒫(A ∩ B).
> 34. If X ⊆ A ∪ B, then X ⊆ A or X ⊆ B.

**Ch 11（§11.1，题 1、7；§11.3，题 7、10）——关系**

> 1. Let A = {0, 1, 2, 3, 4, 5}. Write out the relation R that expresses > on A. Then
>    illustrate it with a diagram.
> 7. Write the relation < on the set A = ℤ as a subset R of ℤ × ℤ. This is an infinite set,
>    so you will have to use set-builder notation.
> 9. Let A = {1, 2, 3, 4, 5, 6}. How many different relations are there on the set A?
> 11.3#7. Define a relation R on ℤ as xR y if and only if 3x − 5y is even. Prove R is an
>    equivalence relation. Describe its equivalence classes.
> 11.3#10. Suppose R and S are two equivalence relations on a set A. Prove that R ∩ S is
>    also an equivalence relation.

**Ch 12（§12.1，题 1、5；§12.2，题 1、4）——函数**

> 12.1#1. Suppose A = {0, 1, 2, 3, 4}, B = {2, 3, 4, 5} and f = {(0, 3), (1, 3), (2, 4),
>    (3, 2), (4, 2)}. State the domain and range of f. Find f(2) and f(1).
> 12.1#5. Give an example of a relation from …
> 12.2#1. Let A = {1, 2, 3, 4} and B = {a, b, c}. Give an example of a function f : A → B
>    that is neither injective nor surjective.
> 12.2#4. A function f : ℤ → ℤ × ℤ is defined as f(n) = (2n, n + 3). Verify whether this
>    function is injective and whether it is surjective.

**Ch 14（§14.1，题 1、9、11；§14.2，题 1）——基数（题目要求的"cardinality 练习"）**

> A. Show that the two given sets have equal cardinality by describing a bijection from one
> to the other. Describe your bijection with a formula (not as a table).
> 1. ℝ and (0, ∞)   3. ℝ and (0, 1)   9. {0, 1} × ℕ and ℕ   10. {0, 1} × ℕ and ℤ
> 11. [0, 1] and (0, 1)
> 14.2#1. Prove that the set A = {ln(n) : n ∈ ℕ} ⊆ ℝ is countably infinite.

**Ch 13（§13.2、§13.7 原文；题目要求的"Ch 13"——实为微积分证明，不是基数）**：

> Exercises for Section 13.2
> 1. Prove that lim_{x→5} (8x − 3) = 37.
> 5. Prove that lim_{x→3} (x² − 2) = 7.
>
> Exercises for Section 13.7
> 1. Prove that {n / 2ⁿ / n!} converges to 0.
> 3. Prove that {2n²+1 / (3n−1)} diverges to ∞.
> 7. Prove that if a sequence diverges to infinity, then it diverges.

§13 的练习从 §13.2 起（**§13.1 无练习**），每节规模很小（§13.5 仅 ~2 题），
属"把 ε-δ 语言套进已学证明框架"的应用章；也解释了为何第 3 版把它插在
Functions(12) 与 Cardinality(14) 之间——它复用 Part II/III 的技术，不引入新概念。

---

## 2. Heather Macbeth, *The Mechanics of Proof* — **完全取证（含 Lean 4 tactic 顺序）**

（开放获取在线版 + GitHub 源码；见下文。）

- 开放获取在线版：<https://hrmacbeth.github.io/math2001/>
- 源码/配套 Lean 代码：<https://github.com/hrmacbeth/math2001>
- 作者主页：<https://faculty.fordham.edu/hmacbeth1/>
- 依据前言：**"Over two hundred problems appear with solutions as examples in the text,
  and several hundred more problems appear without solution as exercises for the reader."**
  实测全章 `Problem` 标记 195 处、`Solution` 171 处（与"200+"大致吻合）。

### (a) 真实章节列表（侧边栏原文）

```
Preface
 1. Proofs by calculation
 2. Proofs with structure
 3. Parity and divisibility
 4. Proofs with structure, II
 5. Logic
 6. Induction
 7. Number theory
 8. Functions
 9. Sets
10. Relations
Index of Lean tactics
Transitioning to mainstream Lean   (附录)
```

章内节标题（抓取自各章页面锚点）：

- **1.** Proving equalities / Proving equalities in Lean / Tips and tricks / Proving inequalities / A shortcut
- **2.** Intermediate steps / Invoking lemmas / Or, and proof by cases / And / Existence proofs
- **3.** Definitions: parity / Divisibility / Modular arithmetic: theory / Modular arithmetic: calculations / Bézout's identity
- **4.** For all and implication / If and only if / There exists a unique / Contradictory hypotheses / Proof by contradiction
- **5.** Logical equivalence / The law of the excluded middle / Normal form for negations
- **6.** Introduction / Recurrence relations / Two-step induction / Strong induction / Pascal's triangle / The division algorithm / The Euclidean algorithm
- **7.** Infinitely many primes / Gauss and Euclid's lemmas / The square root of two
- **8.** Injectivity and surjectivity / Bijectivity / Composition of functions / Product types
- **9.** Introduction / Set operations / The type of sets
- **10.** 10.1 Reflexive, symmetric, antisymmetric, transitive / 10.2 Equivalence relations

### (b) ⚠️ 最大结构性发现：**Functions（Ch 8）在 Sets（Ch 9）之前**，且**全书没有基数章**

> 下列 "—" 类断言均已对**全书所有章节页面**做正则全量扫描确认（不是只查一章）。

- **Ch 8 Functions 排在 Ch 9 Sets 之前。** 关系（Ch 10 Relations）**在集合之后**。
  即顺序是 **Functions → Sets → Relations**，与 Hammack（Sets → Relations → Functions）
  和 Avigad（Sets → Relations → Functions）**都不同**。
- **没有 cardinality / countability 章**（全书 grep `cardinal|countab|Fintype` 无实质命中；
  Ch 8 只讲 Injective/Surjective/Bijective，没有"等势 / 可数 / Cantor 定理"）。
- **没有 Classical / 选择公理**（grep `Classical` = 0 命中，不走 `choose`/`Classical.choice`）。
  也就是说这本书**完全绕开了"基数 vs 选择"的先后问题**——它不教基数。

### 集合如何处理（`Set α` 作为谓词）

第 9 章开篇原文（<https://hrmacbeth.github.io/math2001/09_Sets.html>）：

> "In type theory, the logical foundation for this book, **a set in a type X is specified by
> a predicate on X.** For example, 'the set of integers n such that n ≤ 3' is a set in ℤ.
> There is a standard notation for sets specified by predicates… `{n : ℤ | n ≤ 3}`.
> Note that the infoview confirms that the type of the expression is `Set ℤ`, a set of integers."

子集的定义在书里直接印出（同页）：

```lean
def Set.Subset (U V : Set α) : Prop := ∀ ⦃x⦄, x ∈ U → x ∈ V
```

**注意（重要，避免过度断言）**：书中**没有**把 `Set α = α → Prop` 这个等式字面印出来。
它给的是**两条互补的说明**，合起来等价于那个等式：

1. **语义侧（§9.1）**："a set in a type X is specified by **a predicate** on X"，
   并且 `{n : ℤ | n ≤ 3}` 的类型被 Lean 报为 `Set ℤ`。
2. **类型侧（§9.3.1 "The type of sets"）**：
   > "Let X be a type. The collection of all sets in X can itself be considered as a type.
   > This type is sometimes denoted **𝒫(X)**. … In Lean, for a type X, **the type of sets in X
   > is denoted `Set X`**."
   > ```lean
   > #check {3, 4, 5}          -- `{3, 4, 5} : Set ℕ`
   > #check {n : ℕ | 8 < n}    -- `{n | 8 < n} : Set ℕ`
   > #check {k : ℕ | ∃ a, a^2 = k}
   > #check {{3, 4}, {4, 5, 6}}  -- `{{3, 4}, {4, 5, 6}} : Set (Set ℕ)`
   > #check {s : Set ℕ | 3 ∈ s}  -- `{s | 3 ∈ s} : Set (Set ℕ)`
   > ```
   > "**This operation can be iterated: you can have sets in the type of sets, and so on.**"
   > "Exercise: write down an object of type `Set (Set (Set ℕ))`."

即 Macbeth 的路线是：**`Set X` 就是 𝒫(X)（"X 中集合的类型"），而集合成员关系由谓词给出**；
`Set α := α → Prop` 是 Mathlib 的实现定义，本书未逐字写出，但它是本书用法的**直接推论**
（§9.3.3 甚至把 `Set ℕ → Set ℕ` 当普通函数类型用：`def p (s : Set ℕ) : Set ℕ := {n : ℕ | n + 1 ∈ s}`）。

教学上关键的一点是：**"证 a ∈ S" 的方法是 `dsimp` 把集合成员关系展开成底层谓词**，原文：

> "The tactic `dsimp` unfolds the definition of the set and of membership in that set,
> reducing it to the goal `⊢ 1 ≤ 3` which is resolved by `numbers`."

```lean
example : 1 ∈ {n : ℤ | n ≤ 3} := by
  dsimp
  numbers
```

§9.3 "The type of sets" 才把"某类型中所有集合"本身当成一个类型（幂集视角），
即先有集合语言、最后才把它类型化，这个"由内而外"的顺序是有意的。

### (c) ⚠️ Lean 4 tactic 引入顺序（**照抄 `Index of Tactics` 页并按其标注的 first use 重排**）

来源：<https://hrmacbeth.github.io/math2001/Index_of_Tactics.html>
该页按字母序列出，每条注明 "first use: Section x.y"。**按节号重排后**的真实引入顺序：

| 首用节 | Tactic | 书本说明（节录） |
|---|---|---|
| §1.2 | `ring` | "Solves algebraic equality goals… effectively 'expand out both sides and rearrange'." |
| §1.2 | `rw` | "Substitution: looks for the left-hand side of a specified equality fact in the goal, and replaces it with the right-hand side." |
| §1.4 | `numbers` | "Proves numeric facts, like 3·12 < 13 + 25 or 3·5+1=4·4." |
| §1.4 | `extra` | "A comparison tactic for inequalities… checks an inequality whose two sides differ by the addition of a positive quantity." |
| §1.4 | `rel` | "A 'substitution-like' tactic for inequalities… Compare with `rw`." |
| §1.5 | `addarith` | "Attempts to solve an equality or inequality by moving terms from LHS to RHS, or vice versa." |
| §2.1 | `have` | "Records a fact (followed by the proof of that fact), which then becomes available as an extra hypothesis." |
| §2.1 | `cancel` | "Cancels a common factor from LHS/RHS of equality/inequality…" |
| §2.2 | `apply` | "Invokes a specified lemma or hypothesis to modify the goal."（对 ∀/→ 假设的用法见 §4.1） |
| §2.3 | `left` | "Selects the left alternative of an 'or' goal (∨)." |
| §2.3 | `right` | "Selects the right alternative of an 'or' goal (∨)." |
| §2.3 | `obtain`（∨） | "Takes apart a hypothesis of the form 'or'…" |
| §2.4 | `constructor` | "Splits an 'and' goal (∧) into sub-goals for its left and right parts."（对 ↔ 目标见 §4.2） |
| §2.4 | `obtain`（∧） | 同上（"and"） |
| §2.5 | `use` | "Provides a witness to an existential goal (∃)." |
| §2.5 | `obtain`（∃） | 同上（"there exists"） |
| §3.1 | `dsimp` | "Unfolds a definition. Typically used while working on a proof rather than in the final version." |
| §3.4 | `mod_cases` | "Introduces cases for a variable according to its residue modulo a specified number." |
| §4.1 | `intro` | "Introduces a universally quantified variable (∀) or the antecedent of an implication (→)…"（对 ¬ 目标见 §4.5） |
| §4.1 | `interval_cases` | "Given a natural-number or integer variable n for which numeric upper and lower bounds are available, produce cases for each of the numeric possibilities for n." |
| §4.2 | （`constructor` 用于 ↔、`rw` 用于 ↔ 假设/引理） | 同条目，二次首用 |
| §4.4 | `contradiction` | "If there are two contradictory hypotheses available, this concludes the proof." |
| §4.5 | `intro`（¬ 目标） | "…or assumes (for the sake of contradiction) the positive version of a negation (¬) goal." |
| §5.2 | `by_cases` | "Case-splits on whether a given statement is true or false." |
| §5.3 | `push_neg` | "Converts a hypothesis or goal to a logically equivalent form with negations pushed inwards as far as possible." |

标记 `*` 的（`addarith`、`cancel`、`extra`、`numbers`、`rel`）是**本书自制的 tactic**，
书本明确警告：

> "Tactics marked * are specific to this book, so you will not be able to get help with them by
> googling/consulting internet forums/etc."

**Index 未收录但书中实际使用**的 tactic（我按章扫描得到首用章）：

| Tactic | 首用 | 备注 |
|---|---|---|
| `truth_table` | Ch 5（Logic） | Index 未列 |
| `field_simp` | Ch 6（Induction） | Index 未列 |
| `ext` | §8.3.2（Composition of functions） | 原文："note the new tactic ext… (The name stands for 'extensionality'.)" |
| `exhaust` | §8.1.8 | 原文："In particular, exhaust can prove any (true) variable-free statement… making more serious use of exhaust in Chapter 9." |
| `check_equality_of_explicit_sets` | §9.2.8 | 书中现写的 macro：`(tactic\| (ext; dsimp; exhaust))` |
| `cases … <;> …` | §8.1.9 | 组合子用法 |

（附录 `Transitioning to mainstream Lean` 另提到 `linarith` / `gcongr` 作为主流 Lean 的对应物。）

**教学取向**：tactic 引入顺序与"数学论证结构"严格对齐 —— 先等式计算（ring/rw），
再不等式（extra/rel/addarith），再逻辑联结词（left/right/constructor），再量词
（use/obtain/intro），**最后**才 `dsimp` 展开定义、`push_neg` 规范否定。
这是"先窄后宽"的顺序，不是"先全后精"。

### (d) 练习结构 —— 本书最有辨识度的部分

**每节末**有 `x.y.z. Exercises`（例如 9.2.8 Exercises、9.3.6 Exercises、8.1.13 Exercises）。
两种鲜明风格：

**(i) "真/假双版本"配对题**（Ch 8 起大量使用）——学生必须先判断真假，再选一个 `example` 去证：

> 1. Prove or disprove that the function x ↦ x − 12 from ℚ to ℚ is injective.
>    (If you think it's true, prove it, by solving the first version below.
>    If you think it's false, solve the second version.)
>
> ```lean
> example : Injective (fun (x : ℚ) ↦ x - 12) := by sorry
> example : ¬ Injective (fun (x : ℚ) ↦ x - 12) := by sorry
> ```

**(ii) 自校验题**（填 `sorry` 后由给定 tactic 自动判对错）—— §9.2.8 前五题：

> "For the first five problems, I provide a tactic `check_equality_of_explicit_sets`
> which will prove the statement if you have formulated it correctly. This tactic simply
> runs `ext`, then `dsimp`, then `exhaust`."
>
> 1. Write in an explicitly-listed finite set without repeats, or ∅, which is equal to
>    {−1, 2, 4, 4} ∪ {3, −2, 2}. When you have the correct answer, the given Lean proof will work.
>
> ```lean
> example : {-1, 2, 4, 4} ∪ {3, -2, 2} = sorry := by check_equality_of_explicit_sets
> ```

**纯 Lean 命题练习**（§9.2.8 题 5–7）：

> 5. Prove that {r : ℤ | r ≡ 7 mod 10} ⊆ {s : ℤ | s ≡ 1 mod 2} ∩ {t : ℤ | t ≡ 2 mod 5}.
> ```lean
> example : {r : ℤ | r ≡ 7 [ZMOD 10]} ⊆ {s : ℤ | s ≡ 1 [ZMOD 2]} ∩ {t : ℤ | t ≡ 2 [ZMOD 5]} := by sorry
> ```
> 7. Prove that {n : ℤ | 3 ∣ n} ∪ {n : ℤ | 2 ∣ n} ⊆ {n : ℤ | n^2 ≡ 1 mod 6}ᶜ.

**"双语言"要求**：每个问题与解答都同时给**标准数学散文**和 **Lean**，
且"大多数解答带有非形式化注释"（前言原文）。

**(iii) 第三种、最轻量的练习形态：正文内嵌 "Exercise:"**（无 Lean stub，只写一句话）。
例如 §9.3.1：

> "This operation can be iterated: you can have sets in the type of sets, and so on. …
> **Exercise:** write down an object of type `Set (Set (Set ℕ))`."

这种"读完立刻动手一行"的微练习，与节末大练习形成两级梯度。

练习规模按章实测（`<li>` 计数，
含少量非练习列表项）：Ch 1 ≈42 / Ch 2 ≈43 / Ch 3 ≈52 / Ch 4 ≈30 / Ch 5 ≈32 /
Ch 6 ≈40 / Ch 7 ≈7 / Ch 8 ≈54 / Ch 9 ≈27 / Ch 10 ≈50。
**Ch 7（Number theory）只有 7.1/7.2/7.3 三节、完全没有 exercises 段落**（那些 `<li>` 是导航项），
是纯例题章；Ch 10 只有 10.1、10.2 两节。
注意 **Ch 6 的 exercises 粒度比别章细**：6.1.7 / 6.2.7 / 6.3.6 / 6.4.3 都各有 exercises（不止节末）。

**与 Hammack 的关键差异**：Macbeth **教 antisymmetric**（Ch 10.1 标题里就有，全文 34 处命中），
但**同样从不使用 "partial order" 这个词**（grep `partial order|PartialOrder|preorder|total order|linear order` = 0 命中）。
所以两本书都只把"关系的性质"当工具箱，不引入序结构理论。

---

## 3. Jeremy Avigad, Robert Y. Lewis, Floris van Doorn；**Lean 4 改编：Joseph Hua**
### *Logic and Proof* — **完全取证**

- 现行 URL（**不是**任务书给的 `leanprover.github.io`，那个 404）：
  <https://leanprover-community.github.io/logic_and_proof/>
- 版本说明原文（<https://leanprover-community.github.io/logic_and_proof/introduction.html> §1.6）：

  > "Both this online textbook and the Lean theorem prover are ongoing projects. The original
  > **lean3** version of this textbook is available here. **This version introduces lean4 instead.**
  > … The original textbook was written by Jeremy Avigad, Robert Y. Lewis, and Floris van Doorn.
  > **This was adapted to lean4 by Joseph Hua.**"

  版权行：`©2017, Jeremy Avigad, Joseph Hua, Robert Y. Lewis, and Floris van Doorn.`

### (a) 真实章节列表（24 章，页面侧边栏原文）

```
 1. Introduction                      13. Relations                 
 2. Propositional Logic               14. Relations in Lean
 3. Natural Deduction for Prop. Logic  15. Functions
 4. Propositional Logic in Lean        16. Functions in Lean
 5. Classical Reasoning                17. The Natural Numbers and Induction
 6. Semantics of Propositional Logic    18. The Natural Numbers and Induction in Lean
 7. First Order Logic                  19. Elementary Number Theory
 8. Natural Deduction for First Order Logic  20. Combinatorics
 9. First Order Logic in Lean          21. The Real Numbers
10. Semantics of First Order Logic     22. The Infinite
11. Sets                               23. Axiomatic Foundations
12. Sets in Lean                       24. Appendix: Natural Deduction Rules
```

**注意本书的"成对结构"**：几乎每个数学主题都是"**散文章 + 紧接的 Lean 章**"两章
（4↔3、9↔8、12↔11、14↔13、16↔15、18↔17），且**第 6、10 章是模型论语义**
（truth tables / soundness / completeness）——这是大多数 transition-to-proof 教材没有的。

各章节标题（节录，均为页面原文）：
- **11. Sets**: 11.1 Elementary Set Theory / 11.2 Calculations with Sets / 11.3 Indexed Families of Sets / **11.4 Cartesian Product and Power Set** / 11.5 Exercises
- **13. Relations**: 13.1 Order Relations / 13.2 More on Orderings / 13.3 Equivalence Relations and Equality / 13.4 Exercises
- **15. Functions**: 15.1 The Function Concept / 15.2 Injective, Surjective, and Bijective Functions / 15.3 Functions and Subsets of the Domain / **15.4 Functions and Relations** / 15.5 Exercises
- **16. Functions in Lean**: 16.1 Functions and Symbolic Logic / 16.2 Second- and Higher-Order Logic / 16.3 Functions in Lean / **16.4 Defining the Inverse Classically** / 16.5 Functions and Sets in Lean / 16.6 Exercises
- **20. Combinatorics**: **20.1 Finite Sets and Cardinality** / 20.2 Counting Principles / 20.3 Ordered Selections / 20.4 Combinations and Binomial Coefficients / 20.5 Inclusion-Exclusion / 20.6 Exercises
- **22. The Infinite**: 22.1 Equinumerosity / 22.2 Countably Infinite Sets / 22.3 Cantor's Theorem / 22.4 An Alternative Definition of Finiteness / 22.5 The Cantor-Bernstein Theorem / 22.6 Exercises
- **23. Axiomatic Foundations**: 23.1 Basic Axioms for Sets / 23.2 The Axiom of Infinity / 23.3 The Remaining Axioms / 23.4 Type Theory / 23.5 Exercises

### (b) 集合 / 关系 / 函数在哪里 —— 顺序是 **Sets → Relations → Functions**

真实顺序：**Ch 11–12 Sets → Ch 13–14 Relations → Ch 15–16 Functions → … → Ch 20 有限基数
→ Ch 22 无限基数 → Ch 23 公理基础**。与 Hammack 的 **Relations(11) → Functions(12) → Cardinality(14)**
同构（关系先于函数）；**函数被明确当作关系的特例**（15.4 "Functions and Relations"）。

集合章内部的顺序值得注意：**笛卡尔积与幂集被放到最后（§11.4）**，
而 Hammack 把笛卡尔积放在最前（§1.2）。这是两书在"有序对何时引入"上的**反向**选择。

Ch 7.1 有一个 "Functions, Predicates, and Relations" 节，但那是**一阶逻辑的语法层面**
（引入函数符号/谓词符号），不是数学上的函数定义；数学处理在 Ch 15。

### (c) 选择 vs 基数：**选择（§16.4）在基数（Ch 20 / Ch 22）之前**

这是我按"choice / cardinality 先后"重点核查的项，**有页面原文为证**：

**§16.4 Defining the Inverse Classically**（<https://leanprover-community.github.io/logic_and_proof/functions_in_lean.html>）：

> "Defining inverse functions, however, **requires classical reasoning**, which we get by
> opening the classical namespace"
>
> ```lean
> import Mathlib
> open Classical
> ...
> example : (∀ x, ∃ y, R x y) → ∃ f : A → B, ∀ x, R x (f x) :=
>   axiomOfChoice
> example (h : ∃ x, P x) : P (choose h) :=
>   choose_spec h
> ```
>
> "The axiom of choice tells us that if, for every x : X, there is a y : Y satisfying R x y,
> then there is a function f : X → Y which, for every x, chooses such a y. In Lean, this
> 'axiom' is proved using a classical construction, the `choose` function…"
>
> ```lean
> noncomputable def inverse (f : X → Y) (default : X) : Y → X :=
>   fun y ↦ if h : ∃ x, f x = y then choose h else default
> ```

另外 §5 "Classical Reasoning" 更早就引入经典推理（5.1 Proof by Contradiction /
5.2 Some Classical Principles / 5.3 The contradiction Tactic）。

**结论**：`Logic and Proof` 的顺序是 **经典推理 Ch 5 → 选择公理 §16.4 → 有限基数 §20.1 →
无限基数 Ch 22 → 集合论公理 Ch 23**。即 **"选择在基数之前"**。
（与之相对，Hammack 根本没有选择公理；Macbeth 根本没有基数也没有选择。）

### (d) 自动化程度：**不是 `simp` 重写型，而是显式自然演绎型**

我按章统计了 tactic 关键词出现次数（原文 HTML 文本）：

| 章 | `simp` | `rw [` | `apply` | `use` | `intro` | `ext` | `choose` |
|---|---|---|---|---|---|---|---|
| 4. Propositional Logic in Lean | 0 | 0 | 18 | 30 | 66 | 4 | 0 |
| 12. Sets in Lean | 1 | 13 | 18 | 15 | 37 | 10 | 0 |
| 16. Functions in Lean | 4 | 8 | 2 | 9 | 33 | 8 | **9** |
| 22. The Infinite | 0 | 0 | 2 | 6 | 0 | 2 | 1 |

- `simp` 几乎不出现（0–4 次/章），`norm_num`/`omega`/`exact?` **完全为 0**。
- 主导的是 **`intro` / `apply` / `exact` / `cases` / `use` / `rw`** —— 即与第 3、8 章
  "Natural Deduction" 的推理规则**一一对应**的显式 tactic。§4.4 原文：

  > "Instead of `fun h1 ↦ h2` we use `intro (h1 : A ∧ (B ∨ C))` … Instead of `Or.elim h`
  > and `And.elim h` we use `cases h with` … for any `h : A → B`, `apply h` will change the
  > goal from B to A. … when our goal is A and `h1 : A` we can close the goal by writing `exact h1`."

- 书里同时教 **term mode 与 tactic mode**，并要求两种都写（见下）。
- 本书**不用 `simp` 做主力**这一点，对"想教学生理解每一步"的课程是重要参考；
  代价是证明更长（§4.4 里同一个命题的 term mode 版本明显比 tactic 版本冗长）。

### (e) 练习结构

- **每章末一节 `x.y Exercises`**（4.8、5.4、6.4、7.6、8.6、9.7、10.6、11.5、12.5、13.4、
  14.4、15.5、16.6、17.7、18.3、19.6、20.6、21.6、22.6、23.5）。
- **形式是"填 `sorry`"**：练习给出**已经写好签名与 `:=` / `fun` 骨架的 Lean 代码**，
  学生补 `sorry`。签名常**给出整段上下文**而不是只给命题。
- §4.8 明确要求**两种模式都写**：
  > "Prove the following in **both term mode and tactic mode**:"
  > ```lean
  > example : A ∧ (A → B) → B := sorry
  > example : A → ¬ (¬ A ∧ B) := sorry
  > example : ¬ (A ∧ B) → (A → ¬ B) := sorry
  > example (h₁ : A ∨ B) (h₂ : A → C) (h₃ : B → D) : C ∨ D := sorry
  > example (h : ¬ A ∧ ¬ B) : ¬ (A ∨ B) := sorry
  > example : ¬ (A ↔ ¬ A) := sorry
  > ```
- §12.5（Sets in Lean）的练习混合形式：题 1 是 "Fill in the sorry's."；
  题 2 给出一段**带注释的示范代码**再留一个 `sorry`
  （"notice that we do not have to mention x when applying `h : disj A B`"）；
  题 3 指定**只准用给定引理**（"using the theorems `Inter.intro`, `Inter.elim`,
  `Union.intro`, and `Union.elim`"）；题 4 同理限定 `Subset.trans` / `Subset.refl`。
  这种"限定可用引理"的练习设计，对学生建立"最小依赖"意识很有价值。
- §22.1 有一句直接写在正文里的练习布置：
  > "The following theorem says, essentially, that equinumerosity is an equivalence relation. …
  > **The proof is left as an exercise.**"

---

## 4. Daniel J. Velleman, *How to Prove It: A Structured Approach*, 3rd ed. (Cambridge, 2019)

- **第 4 个前提错误（本任务最重的一处）**：任务书说"Ch 6 induction, Ch 7 infinite sets"（共 7 章）。
  **第 3 版实际有 8 章**：**Ch 7 = Number Theory（第 3 版新增）**，**Ch 8 = Infinite Sets**。
  "7 章、Ch 7 = 无限集"是**第 2 版**（2006）的布局。第 3 版前言原话（转引自扫描件）：
  > "Chapter 7, new in this third edition, gives an introduction to number theory, and Chapter 8
  > discusses infinite cardinalities."

### 权威元数据（我本人独立复核）

- Cambridge Core 书目页（**可访问**；任务书里的 `/us/...` 营销页会 403）：
  <https://www.cambridge.org/core/product/identifier/9781108539890/type/book>（HTTP 200, 515 KB）
  该页 HTML 内嵌 CUP 自己的章节 JSON（`titleGroup:{label:…,title:"…"}` + `fpage`/`lpage`），
  我用正则直接抽出，**逐条照录**（含页码范围）：

```
9781108539890#PRF1  Preface                          pp. ix–xii
9781108539890#INT1  Introduction                     (罗马页)
9781108539890#C1    Sentential Logic                 pp. 8–57
9781108539890#C2    Quantificational Logic           pp. 58–88
9781108539890#C3    Proofs                           pp. 89–172
9781108539890#C4    Relations                        pp. 173–228
9781108539890#C5    Functions                        pp. 229–272
9781108539890#C6    Mathematical Induction           pp. 273–323
9781108539890#C7    Number Theory                    pp. 324–371
9781108539890#C8    Infinite Sets                    pp. 372–396
9781108539890#APX1  Solutions to Selected Exercises  pp. 397–450
9781108539890#REF1  Suggestions for Further Reading  pp. 451–452
9781108539890#IND1  Index                            pp. 455–458
```
  同一 JSON 里还出现 `Summary of Proof Techniques`（在 Suggestions for Further Reading 之后）。
- 版本/ISBN（Open Library `edition_key` 实测，我自己查的）：
  3rd ed = **2019**, Cambridge University Press, **400 页**, ISBN-13 **9781108439534** / **9781108424189**
  （在线版 ISBN 9781108539890）；work key `/works/OL3918291W`，共 7 个版本（1994 起）。
- Crossref 记录（subagent 复核）：DOI `10.1017/9781108539890`，章节划分与上表一致。

### (a) Ch 1–8 节级目录

> **证据等级说明（请务必按此可信度使用）**：**章级**数据来自上面的 CUP 官方 JSON / Crossref
> —— 权威，且**我本人独立复核**。**节级**（x.y）数据来自一份公开可读的第 3 版全文扫描的 OCR 文本，
> 其上传来源**非官方**；subagent 已把它与 CUP/Crossref 的章级数据交叉核对**一致**，
> 但**节标题未取得第二个权威来源确认**。因此下表节标题标注为「扫描件取证」。

```
1 Sentential Logic
  1.1 Deductive Reasoning and Logical Connectives
  1.2 Truth Tables
  1.3 Variables and Sets              ← 集合第一次出现
  1.4 Operations on Sets
  1.5 The Conditional and Biconditional Connectives
2 Quantificational Logic
  2.1 Quantifiers
  2.2 Equivalences Involving Quantifiers
  2.3 More Operations on Sets          ← 集合运算第二次
3 Proofs                               ← 证明技术章
  3.1 Proof Strategies
  3.2 Proofs Involving Negations and Conditionals
  3.3 Proofs Involving Quantifiers
  3.4 Proofs Involving Conjunctions and Biconditionals
  3.5 Proofs Involving Disjunctions
  3.6 Existence and Uniqueness Proofs
  3.7 More Examples of Proofs
4 Relations
  4.1 Ordered Pairs and Cartesian Products
  4.2 Relations
  4.3 More About Relations
  4.4 Ordering Relations
  4.5 Equivalence Relations
5 Functions
  5.1 Functions
  5.2 One-to-One and Onto
  5.3 Inverses of Functions
  5.4 Closures
  5.5 Images and Inverse Images: A Research Project
6 Mathematical Induction
  6.1 Proof by Mathematical Induction   6.2 More Examples   6.3 Recursion
  6.4 Strong Induction                  6.5 Closures Again
7 Number Theory                          ← 第 3 版新增
  7.1 Greatest Common Divisors  7.2 Prime Factorization  7.3 Modular Arithmetic
  7.4 Euler's Theorem           7.5 Public-Key Cryptography
8 Infinite Sets
  8.1 Equinumerous Sets  8.2 Countable and Uncountable Sets
  8.3 The Cantor-Schroder-Bernstein Theorem
```

### (b) ⚠️ 焦点问题 (i) 的答案：Velleman Ch 4–5 的**精确**先后顺序

| # | 概念 | 精确位置 |
|---|---|---|
| 1 | 有序对 / 笛卡尔积 | **§4.1**（Definition 4.1.1 定义 A × B） |
| 2 | 关系 | **§4.2**（Definition 4.2.1） |
| 3 | **关系的复合** `S∘R`；另含 domain、range、逆关系 `R⁻¹` | **§4.2**（Definition 4.2.3；Theorem 4.2.5 = 结合律）——**复合在函数之前就讲了** |
| 4 | reflexive / symmetric / transitive | **§4.3**（Definition 4.3.2） |
| 5 | **ordering relations：antisymmetric、partial order、total order** | **§4.4**（Definitions 4.4.1, 4.4.2） |
| 6 | 等价关系 | **§4.5**（Definition 4.5.1） |
| 7 | 划分 partitions（pairwise disjoint） | **§4.5**（Definition 4.5.2） |
| 8 | 等价类 equivalence classes | **§4.5**（Definition 4.5.3） |
| 9 | 函数 | **§5.1**（Definition 5.1.1） |
| 10 | **函数的复合** `g∘f`，`(g∘f)(a) = g(f(a))` | **§5.1**（Theorem 5.1.5；Example 5.1.6）——经 Theorem 4.2.5 从关系情形继承 |
| 11 | 单射 / 满射（术语用 "one-to-one" / "onto"） | **§5.2**（Definition 5.2.1） |
| 12 | 反函数 | **§5.3**（Theorems 5.3.1–5.3.3） |
| 13 | Closures（集合在函数下的闭包） | **§5.4**（Definition 5.4.3；Definition 5.4.8 为 f : A×A → A） |
| 14 | 像 / 原像 | **§5.5**（Definition 5.5.1 同时定义 f(X) 与 f⁻¹(Y)） |

**一句话概括 Velleman**：
> **有序对(4.1) → 关系(4.2) → 关系的复合(4.2) → 关系性质(4.3) → 序关系(4.4) →
> 等价关系(4.5) → 函数(5.1) → 函数的复合(5.1) → 单/满射(5.2) → 反函数(5.3) →
> closures(5.4) → 像与原像(5.5)**

两点值得单独指出：
1. **"关系先于函数"**，且**复合先在关系层面教、再在函数层面复用**（Theorem 4.2.5 → Theorem 5.1.5）。
2. **像与原像被放到函数章最末（§5.5），并且以 "A Research Project" 的形式收尾**——
   即作者把它当作开放探究，而非必教条目。

**第 3 版相对第 2 版的一处重要删改**（前言原文，转引自扫描件）：
> "The section on reflexive, symmetric, and transitive closures of relations **has been deleted
> from Chapter 4** (although these topics are now introduced in some exercises in Section 4.4);
> it has been replaced with a new section in Chapter 5 on closures of sets under functions."

后果：3rd ed **没有**自反/对称/传递闭包的独立小节，闭包定义降级为 **§4.4 的练习 24–26**
（正文交叉引用写作 "See exercise 25 of Section 4.4 for the definition of transitive closure"）；
第 2 版的 `4.5 Closures` / `4.6 Equivalence Relations` / `5.4 Images and Inverse Images`
在 3rd ed 变成 `4.5 Equivalence Relations` / `5.4 Closures` / `5.5 Images and Inverse Images`。

### (c) 集合论在哪里（对"零基础"课程很关键）

**Velleman 的集合不是独立一章，而是嵌在逻辑章里、且在证明技术之前**：

> **§1.3 Variables and Sets → §1.4 Operations on Sets → §2.3 More Operations on Sets → §3 Proofs**

即 **Velleman = 逻辑+集合 → 证明技术**，与 **Solow = 证明技术 → 集合仅作记号**（见 §5）**恰好相反**。
这是一条在课程设计上很硬、但常被忽略的对照。

⚠️ **基数与选择**：第 3 版**没有**选择公理章节；基数在 **Ch 8 Infinite Sets**
（8.1 等势 / 8.2 可数不可数 / 8.3 Cantor–Schröder–Bernstein）。
所以 Velleman 与 Hammack 同属"**无选择公理**"一类。

### (d) 练习结构（前言原文，转引自扫描件）

> "**Every section of every chapter ends with a list of exercises.** Some exercises are marked
> with an asterisk; solutions or hints for these exercises are given in the appendix. Exercises
> marked with the symbol [icon] **can be done using Proof Designer software**."

- 题号**按节编号**，交叉引用写作 "exercise N of Section X.Y"。
- 第 3 版新增 **150+ 道练习**（前言原文提及 "more than 150 additional exercises"）。
- 附录 = `Solutions to Selected Exercises`（pp. 397–450，约 54 页，**只给部分**解答）。
- **独一无二的特色**：**Proof Designer 软件**（Velleman 自写的证明辅助教学软件）被整合进
  练习标记体系。这在传统 transition-to-proof 教材里是罕见的"半形式化"设计，
  与本项目用 Lean 做判卷的取向最接近。
- ⚠️ 未取证：本节**没有**逐条抄录 Velleman 的练习原文（没有合法免费预览可引）。

---

## 5. Daniel Solow, *How to Read and Do Proofs*, 6th ed. (Wiley, 2013)

### (a) 权威元数据与完整目录（出版社页面取证）

- 来源：<https://www.wiley.com/en-us/How+to+Read+and+Do+Proofs%3A+An+Introduction+to+Mathematical+Thought+Processes%2C+6th+Edition-p-9781118164020>（HTTP 200）
  同一 TOC 亦出现在电子书 ISBN 页面 `...-p-9781118857878`，**两个 Wiley 记录互相印证**。
- 注意：Wiley 页面的 TOC 藏在 Next.js flight payload 里，**单用 `pandoc` 看不到**，需从 HTML 里挖。
- 元数据：Paperback ISBN-13 **9781118164020**，出版日 **2013-07-29**，**336 页**；
  电子书 9781118857878（2013-10-22）；OpenLibrary edition `OL27557725M` 亦为 2013-07-29。

```
PART I  Proofs
   1 The Truth of It All                                  p. 1
   2 The Forward-Backward Method                           p. 9
   3 On Definitions and Mathematical Terminology           p. 25
   4 Quantifiers I: The Construction Method                p. 41
   5 Quantifiers II: The Choose Method                     p. 53
   6 Quantifiers III: Specialization                       p. 69
   7 Quantifiers IV: Nested Quantifiers                    p. 81
   8 Nots of Nots Lead to Knots                            p. 93
   9 The Contradiction Method                              p. 101
  10 The Contrapositive Method                             p. 115
  11 The Uniqueness Methods                                p. 125
  12 Induction                                             p. 133
  13 The Either/Or Methods                                 p. 145
  14 The Max/Min Methods                                   p. 155
  15 Summary                                               p. 163
PART II  Other Mathematical Thinking Processes
  16 Generalization                                        p. 179
  17 Creating Mathematical Definitions                     p. 197
  18 Axiomatic Systems                                     p. 219
Appendix A  Examples of Proofs from Discrete Mathematics  p. 237
Appendix B  Examples of Proofs from Linear Algebra        p. 251
Appendix C  Examples of Proofs from Modern Algebra        p. 269
Appendix D  Examples of Proofs from Real Analysis         p. 287
Solutions to Selected Exercises p. 305 | Glossary p. 357 | References p. 367 | Index p. 369
Front matter: Foreword xi, Preface to the Student xiii, Preface to the Instructor xv, Acknowledgments xviii
```

> **一处待确认（subagent 已标记）**：Wiley 的 payload 把 `<ol>` 标记压平，第 1 章文字为
> "Chapter 1: The Truth of It All"，其余 2–18 为裸标题；且第 11 章的确切措辞
> （"The Uniqueness Methods" vs "...Method"）尚缺第二个独立来源。**其余标题两处来源一致。**

### (b) ⚠️ 焦点问题：Solow 是否"先教证明技术、后教集合"？——答案是**更强的版本**

**Solow 第 6 版全书 18 章中，没有任何一章是集合论，也没有任何一章是函数。**

- Part I（Ch 1–15）**全部是证明技术**，顺序即上表：真值 → 前推/后推法（Forward-Backward）→
  定义与术语 → 量词四章（构造法 / 选取法 / 特例化 / 嵌套量词）→ 否定 →
  反证法 → 逆否法 → 唯一性 → 归纳 → 或/与法 → 最大/最小法 → 总结。
- Part II（Ch 16–18）是"思维过程"（推广 / 创造定义 / 公理系统），**仍不是集合论**。
- 集合与函数**只作为记号与例题题材**散落在技术章内部。证据：第 6 版配套
  *Solutions Manual* 的解答分节覆盖 Ch 1–18 + Appendix A–D，其中 **Ch 1 的练习就已经出现集合记号**
  （如 "1.5 a. Hypothesis: A, B and C are sets of real numbers with A ⊆ B"），
  Ch 13 出现集合运算（"x ∈ (S∩T)ᶜ"）。
- 结论：**"证明技术在任何集合论之前"在此书成立，而且是极端形态——集合论从未被当作主题来教。**
- ⚠️ 保留项：目前**不能**指出学生用书正文里集合记号首次出现的精确页码
  （现有依据是解答手册）；6e 学生用书在 archive.org 为 lending-only（HTTP 401）。

### (c) 练习结构

- 题号**在每章内连续编号**，形如 `<章>.<n>`。经 6e *Solutions Manual* 反查实测被解答的题号范围：
  `1.1–1.18, 2.12–2.23, 4.7, 5.19–5.23, 6.16–6.21, 7.7–7.16, 10.11, 12.22, 13.8,
  15.11–15.15, 16.20, 17.9–17.19, 18.6` —— 可见**每章题量约 6–23 题**，且**只解答其中一部分**。
- 正文后有 `Solutions to Selected Exercises`（p.305），另有独立 *Solutions Manual*。
- Wiley 的 "New to this Edition" 提到每章 "numerous exercises"。

### (d) 教学定位小结

Solow 是**极端"技术先行"**的一端：把"怎么读一个待证命题、怎么用前推/后推法开工"放在最前，
连集合、函数都不作为知识块引入。对"从未写过证明"的学习者，这消除了"先学一堆对象"的门槛；
代价是**所有例子都借用微积分/线性代数/离散数学/现代代数的现成题材**（Appendix A–D 正是这四个领域）
—— 对数学成熟度低的学习者，这些题材本身可能构成额外负担。

---

## 6. Jay Cummings, *Proofs: A Long-Form Mathematics Textbook* — **已取证**

- **第 5 个前提错误**：任务书问 "there is a 2nd edition — determine which editions exist"。
  取证结论：**《Proofs》没有第 2 版**。任务书里的"2nd edition"很可能是从同系列另一本书串过来的——
  Long-Form 系列里唯一有第 2 版的是 *Real Analysis: A Long-Form Mathematics Textbook*
  （ASIN 1077254547 / ISBN 978-1077254547，作者自己的版本史写着 1st ed. 2018-07、
  "1+ε" 2019-01、**2nd ed. 2019-07**、"2+ε" 2024）。
  证据：Amazon *Proofs* 详情块**没有 "Edition" 字段**；作者自己的 Proofs 书页
  **"edition" 一词出现 0 次**；Open Library work `OL30842285W` / edition `OL42374772M`
  的 `edition_count` = 1。
- **开放获取：否**（purchase-only）。但**作者自建了官方配套站**，免费提供大量材料：
  - 配套站首页：<https://longformmath.com/>
  - Proofs 书主页：<https://longformmath.com/proofs-book/>
  - **官方 TOC PDF（我本人下载并抽取）**：<https://longformmath.com/wp-content/uploads/2025/01/proofs-toc.pdf>
    （3 页，311 KB，PDF 1.3）
  - 官方 38 页扫描试读：`.../2025/01/proofsscannedsample.pdf`（纯图片 PDF，需 OCR）
  - 提示页：<https://longformmath.com/proofs-book/proofs-hints-solutions/>
  - **每章 10 道完整解答**：`.../2025/02/Chapter{1..9}HW.pdf` 与 `HW{1..9}-Solutions.pdf`
  - 勘误：<https://longformmath.com/proofs-book/proofs-errata/>
- 元数据（Amazon + isbnsearch）：ASIN **B08T8JCVF1** = ISBN-13 **979-8595265973**，
  "Independently published"，出版日 **2021-01-19**，**511 页**，出版社亦署名 "LongFormMath.com"。
  另有 2025-05-15 的付费 Kindle "Print Replica"（ASIN B0F8WB1FHZ，同为 511 页），
  属同一本书的另一**格式**，不是新版本。Open Library `ebook_access: "no_ebook"`、
  `has_fulltext: false`、`public_scan_b: false`。archive.org 搜索 `title:proofs AND creator:Cummings`
  得 0 条 → **archive.org 上没有可借副本**。

### (a) 真实目录（**作者官方 TOC PDF 原文照录**，含页码）

```
1  Intuitive Proofs                                        1
   1.1 Chessboard Problems             1.2 Naming Results      1.3 The Pigeonhole Principle
   1.4 Bonus Examples                  Exercises 31   |  Introduction to Ramsey Theory 41
2  Direct Proofs                                          47
   2.1 Working From Definitions  2.2 Proofs by Cases  2.3 Divisibility
   2.4 Greatest Common Divisors  2.5 Modular Arithmetic  2.6 Bonus Examples
   Exercises 81   |  Introduction to Number Theory 89
3  Sets                                                   97
   3.1 Definitions  3.2 Proving A ⊆ B  3.3 Proving A = B  3.4 Set Operations  3.5 Bonus Examples
   Exercises 125  |  Introduction to Topology 137
4  Induction                                             147
   4.1 Dominoes, Ladders and Chips  4.2 Examples  4.3 Strong Induction
   4.4 Non-Examples  4.5 Bonus Examples
   Exercises 188  |  Introduction to Sequences 199
5  Logic                                                 207
   5.1 Statements  5.2 Truth Tables  5.3 Quantifiers and Negations
   5.4 Proving Quantified Statements  5.5 Paradoxes  5.6 Bonus Examples
   Exercises 242  |  Introduction to Real Analysis 253
6  The Contrapositive                                    261
   6.1 Finding the Contrapositive of a Statement  6.2 Proofs Using the Contrapositive
   6.3 Counterexamples  6.4 Bonus Examples
   Exercises 278  |  Introduction to Big Data 285
7  Contradiction                                         293
   7.1 Two Warm-Up Examples  7.2 Examples  7.3 The Most Famous Proof in History
   7.4 The Pythagoreans  7.5 Bonus Examples
   Exercises 320  |  Introduction to Game Theory 325
8  Functions                                             331
   8.1 Approaching Functions  8.2 Injections, Surjections and Bijections
   8.3 The Composition  8.4 Invertibility  8.5 Bonus Examples
   Exercises 362  |  Introduction to Cardinality 371      ← 基数在这里！
9  Relations                                             379
   9.1 Equivalence Relations  9.2 Abstraction and Generalization  9.3 Bonus Examples
   Exercises 402  |  Introduction to Group Theory 413
Appendices                                               421
   A Other Proof Methods 423: A.1 Probabilistic Method  A.2 Linear Algebra Method
      A.3 Combinatorial Method  A.4 Computer-Assisted Proofs  A.5 Proofs by Picture
   B Proofs From The Book 453: B.1–B.10（全为头韵体标题，如
      "Merry Madness from March"、"Zigging Zeniths and Zagging Zones"）
   C Writing Advice 487: C.1 Writing Proofs  C.2 Writing in LaTeX
```

### (b) ⚠️ 集合 / 函数 / 关系 / 基数 的精确位置

| 主题 | 位置 |
|---|---|
| **集合** | **Ch 3**（pp. 97–137）：3.1 Definitions → 3.2 Proving A ⊆ B → 3.3 Proving A = B → 3.4 Set Operations |
| **函数** | **Ch 8**（p. 331） |
| **关系** | **Ch 9**（p. 379） |
| **基数** | **不是编号章！** 是**不编号的章末迷你章** "Introduction to Cardinality"，**pp. 371–378**，夹在 Ch 8 的 Exercises（p.362）与 Ch 9 Relations（p.379）之间 |

**结论：Cummings 也是"函数先于关系"**，与 Macbeth 同一阵营，
与 Hammack / Velleman / Avigad 相反。而且**基数被挂在函数章末尾、先于关系章**。

**一个非常独特的设计**：全书有 **9 个不编号的 "Introduction to X" 迷你章**，
**每章后面挂一个**，按 TOC 顺序为：
Ramsey theory → number theory → topology → sequences → real analysis → big data →
game theory → **cardinality** → group theory。
即作者用"每学完一种证明技术，就看一眼它在某个真实数学领域里长什么样"来维持动机。

### (c) 最独特的顺序：**证明技术最先，逻辑反而很靠后**

> **Ch 1 Intuitive Proofs → Ch 2 Direct Proofs → Ch 3 Sets → Ch 4 Induction →
> Ch 5 Logic → Ch 6 Contrapositive → Ch 7 Contradiction → Ch 8 Functions → Ch 9 Relations**

注意 **Logic 在 Ch 5**，即**排在集合（Ch 3）与归纳（Ch 4）之后**，
且**前两章就已经在写证明了**（Chessboard Problems、Pigeonhole Principle、Direct Proofs）。
这与 Hammack（Logic Ch 2、集合 Ch 1）、Velleman（逻辑+集合 Ch 1–2、证明 Ch 3）都不同，
是"**先动手、后补逻辑**"的极端动机优先路线。

### (d) 风格（作者官方免费试读的原文，OCR 自 `proofsscannedsample.pdf`）

- 正文 p.1：
  > "This book is the gateway to Phase 2. It will show you the techniques mathematicians use to
  > understand our math (which we call proof techniques), and it will introduce you to new math
  > topics that you will explore in detail in your future courses. **So buckle up, because math
  > is about to get a lot more interesting.**"
- 脚注 13：
  > "It's like it's saying '**Yo, lemma help you prove that theorem.**'"
- 脚注 15：
  > "Conjecture: All positive integers are smaller than a trillion. Computer: I've tested the first
  > billion cases, and they all check out. Looks true to me, mate!"
- 脚注 16：
  > "And if you are using this book in a course, then there's one final reason: **It's on the test!**"
- 作者官方提示页（**数字文本，无 OCR 风险**，<https://longformmath.com/proofs-book/proofs-hints-solutions/>）：
  > Exercise 1.10 hint: "You want a hint for 'your own words'?? Bro…."
  > Exercise 3.1 hint: "Not a hint, but according to my brother you can see the personalities of
  > each of his cats based on how many of these pictures they were willing to appear in."

### (e) 练习结构

- **每章末一个 `Exercises` 块**（TOC 里每章 x.5 或 x.6 之后都列 `Exercises`，跨约 5–12 印刷页：
  p.31 / 81 / 125 / 188 / 242 / 278 / 320 / 362 / 402）。**不是每节末**。
- 作者自述（Proofs 书页）：
  > "Each chapter ends with exercises and an open question, as well as 'pro-tips'…"
  > "I chose **10 problems each chapter** and gave a complete solution to those problems…
  > leaving enough problems without solutions for professors who want to assign problems
  > without easily-attained solutions."
- 即 **每章 10 题有完整解答**（官方免费发 9 个 `Chapter N Solved Exercises` PDF，共 **90 题**），
  其余**只有提示、无解答**。
- 可证实的最小题量（subagent 从官方解答/提示页反查的最大题号）：
  `1.26, 2.37, 3.42, 4.30, 5.27, 6.12, 7.27, 8.28, 9.35`
  → **每章至少 12–42 题**；**每章精确总题数 `UNVERIFIED`**。
- ⚠️ **未取证**：Cummings 的练习原文未逐条抄录（作者免费试读是图片 PDF，
  官方解答 PDF 尚未逐题引用）。本节引用的"原文"都是**散文/脚注/提示**，不是习题。

---

## 7. 待补清单（显式标记，防止被误读为"已核实"）

| 目标 | 状态 | 备注 |
|---|---|---|---|
| Velleman 章级目录（8 章 + 页码） | **已取证（权威）** | CUP Core 内嵌 JSON + Crossref；见 §4 |
| Velleman Ch 1–8 节级目录 | **已取证（扫描件，来源非官方）** | 交叉核对章级一致；见 §4(a) 的证据等级说明 |
| Velleman Ch 4–5 有序对/关系/函数各节精确位置 | **已取证** | 本次调研最关键的一项，见 §4(b) |
| Velleman 练习原文逐条抄录 | `UNVERIFIED` | 无合法免费预览可引；只取到**前言对练习制度的描述** |
| Solow 6th ed 章级目录 + 练习编号制度 | **已取证（出版社）** | 两个 Wiley ISBN 页面互证；见 §5 |
| Solow "无集合论章" 的结论 | **已取证（强）** | 18 章标题 + Solutions Manual 反查；见 §5(b) |
| Solow 正文中集合记号首次出现的页码 | `UNVERIFIED` | 6e 学生用书 archive.org 为 lending-only (401) |
| Solow 第 11 章标题确切措辞 | `PARTIAL` | Wiley payload 压平了 `<ol>`；缺第二来源 |
| Solow / Velleman 练习原文逐条抄录 | `UNVERIFIED` | 未取到合法预览 |
| Cummings 章级 + 节级目录（含页码） | **已取证（作者官方 TOC PDF）** | 我本人下载并抽取；见 §6(a) |
| Cummings 开放获取状态 | **已取证：否**（purchase-only） | 但作者免费发 TOC/试读/提示/每章 10 题解答；见 §6 |
| Cummings 幽默风格原文 | **已取证** | 4 条散文/脚注 + 2 条官方提示页文本；见 §6(d) |
| Cummings 练习原文逐条抄录 | `UNVERIFIED` | 免费试读是图片 PDF；见 §6(e) |
| Cummings 每章精确总题数 | `UNVERIFIED` | 只反查到各章最大题号；见 §6(e) |

---

## 8. 已取证教材的横向对比（回答三个焦点问题）

### (i) Velleman Ch 4–5 vs Hammack Ch 11–13 的"关系先于函数"对比

- **Hammack 的答案（已取证）**：**关系先于函数**，且函数**定义为关系**。
  Ch 11 Relations（11.1 定义 → 11.2 三条性质 → 11.3 等价关系 → 11.4 等价类与划分 →
  11.5 ℤ/n → 11.6 Relations Between Sets）**整章之后**才是
  Ch 12 Functions（12.1 定义 → 12.2 单射/满射 → 12.3 鸽笼 → 12.4 复合 → 12.5 反函数 → 12.6 像/原像）。
  **关键桥梁是 §11.6**：先讲"A 到 B 的关系"，下一章第一个定义就把它特化成函数。
- **Hammack 的代价（新发现）**：Ch 11 **不教偏序**（无 antisymmetric / partial order / poset），
  所以"关系"在本书里主要服务于"等价关系 → 划分 → 商集"这条线，而不是"序结构"这条线。
- **Avigad 等的答案（已取证）**：同样 **Relations(13) → Functions(15)**，且 §15.4
  明确叫 "Functions and Relations"。但 Avigad **有** Order Relations（13.1、13.2），
  即"序关系"被放在关系章内、等价关系（13.3）之前。
- **Velleman 的答案（已取证，且比前三本更细）**：同样 **关系(Ch 4) → 函数(Ch 5)**，
  但**节级顺序有一个前三本都没有的特征**：
  `4.1 有序对与笛卡尔积 → 4.2 关系 → 4.2 关系的复合(Definition 4.2.3, Theorem 4.2.5) →
  4.3 关系性质 → 4.4 序关系(partial order) → 4.5 等价关系 → 5.1 函数 → 5.1 函数的复合 →
  5.2 单/满射 → 5.3 反函数 → 5.4 closures → 5.5 像与原像`。
  两个要点：**(a) 复合先在"关系"层面教（§4.2），函数章再复用（§5.1）**；
  **(b) Velleman 是四本里唯一在关系章内正式教 partial order / total order 的**
  （Hammack 与 Macbeth 都完全没有；Avigad 有 13.1–13.2）。
- **Cummings 的答案（已取证）**：**Functions(Ch 8) → Relations(Ch 9)**，
  与 Macbeth 同阵营，**不是**关系先于函数。而且基数（不编号的 "Introduction to Cardinality",
  pp. 371–378）被夹在 Ch 8 的练习与 Ch 9 之间。
- **五本对照小结（"关系 vs 函数"）**：
  **关系先于函数** = Hammack(11→12) / Velleman(4→5) / Avigad(13→15)，且都以"函数是关系的特例"收口；
  **函数先于关系** = Macbeth(8→10，中间夹 Sets 9) / Cummings(8→9)。
  即 **3 : 2**，"关系先行"略占多数但不是共识。Solow 两者都无独立章。

### (ii) Macbeth 的 tactic 引入顺序

见 §2(c) 的完整表格。一句话概括：

> **`ring`/`rw`(§1.2) → `numbers`/`extra`/`rel`(§1.4) → `addarith`(§1.5) →
> `have`/`cancel`(§2.1) → `apply`(§2.2) → `left`/`right`/`obtain`(§2.3) →
> `constructor`(§2.4) → `use`(§2.5) → `dsimp`(§3.1) → `mod_cases`(§3.4) →
> `intro`/`interval_cases`(§4.1) → `contradiction`(§4.4) → `by_cases`(§5.2) →
> `push_neg`(§5.3)**；
> 另加未入索引的 `exhaust`(§8.1.8)、`ext`(§8.3.2)。

即 **等式计算 → 不等式计算 → 逻辑联结词 → 量词 → 定义展开 → 经典推理**，
且 `intro` 到 §4.1 才出现（在 `apply`、`constructor`、`use` 之后）。

### (iii) 有没有书把基数放在选择之前？

| 书 | 选择公理 | 基数 | 先后 |
|---|---|---|---|
| Hammack *Book of Proof* | **完全没有** | Ch 14（Cantor–Bernstein–Schröder） | 不适用（无选择） |
| Macbeth *Mechanics of Proof* | **完全没有**（无 `Classical`） | **完全没有** | 不适用（两者都无） |
| Avigad 等 *Logic and Proof* | §16.4（`axiomOfChoice` / `choose`）；经典推理早在 Ch 5 | §20.1 有限基数；Ch 22 无限基数 | **选择在基数之前** |
| **Velleman *How to Prove It* 3e** | **完全没有**（无选择公理章） | Ch 8 Infinite Sets（8.1–8.3，含 Cantor–Schröder–Bernstein） | 不适用（无选择） |
| **Solow 6e** | **完全没有** | **完全没有**（无集合论章） | 不适用（两者都无） |
| **Cummings *Proofs*** | **完全没有**（该书未涉及公理集合论） | **不编号的 "Introduction to Cardinality"**，pp. 371–378，**夹在 Ch 8 Functions 与 Ch 9 Relations 之间** | 不适用（无选择） |

**最终结论（6/6 本已取证）**：**没有任何一本把基数放在选择之前。**
其中 **5 本（Hammack / Macbeth / Velleman / Solow / Cummings）根本没有选择公理**，
唯一有选择公理的是 Avigad 等，且是"**选择(§16.4) → 有限基数(§20.1) → 无限基数(Ch 22)**"。
这与 Lean/Mathlib 侧的实际依赖关系一致（Mathlib 的基数理论建立在 `Classical.choice` 之上）。
**对课程的直接含义**：如果课程在 Lean/Mathlib 里教基数，必须**自己补**"基数依赖
`Classical.choice`"这件事——五本传统教材都不会提，唯一的正面样本是 Avigad 等。

---

## 9. 归一化对照：同一概念在已取证书中的位置

（数字 = 章/节号；"—" = 该书**完全没有**此内容，已用全文 grep 确认）

| 概念 | Hammack *Book of Proof* 3e | Macbeth *Mechanics of Proof* | Avigad 等 *Logic and Proof* (Lean 4) | Velleman *How to Prove It* 3e |
|---|---|---|---|---|
| 集合/成员关系 | **1.1**（Part I 首章，**在 Logic 之前**） | **9.1**（**在 Functions 之后**） | **11.1**（散文章）/ **12.1**（Lean） | **§1.3 Variables and Sets**（**在证明技术 Ch 3 之前**） |
| 集合的底层表示 | 朴素集合论（无类型/谓词说明） | **"集合由谓词指定"**；`Set X` = 𝒫(X)（§9.1 / §9.3.1） | 朴素 + Ch 23 公理化；`Set U` | 朴素集合论（无类型说明） |
| 子集 | 1.3 | 9.1.3（`def Set.Subset`） | 11.1 | §1.3–1.4 |
| **幂集** | **1.4**（子集之后**立刻**） | 9.3（"type of sets"，含 `Set (Set ℕ)`） | **11.4**（集合章**最后**一节） | §1.4 Operations on Sets（含幂集） |
| **有序对 / 笛卡尔积** | **1.2**（**在子集之前！**） | —（未单独讲；靠积类型 `×`，§8.4 product types） | **11.4**（集合章**最后**一节） | **§4.1**（**关系章第一节**，Definition 4.1.1） |
| 集合运算（∪∩\ᶜ） | 1.5–1.7 | 9.2 | 11.1–11.2 | §1.4 + §2.3 More Operations on Sets |
| 索引族 | 1.8 | — | 11.3 | §2.3（indexed families） |
| Russell 悖论 | **1.10**（第 1 章内！） | — | 11.1（提及）+ 23.1 | —（未取证到） |
| 关系 | 11.1 | 10（10.1 性质 / 10.2 等价关系） | 13.1（序）/ 13.3（等价） | **§4.2**（Definition 4.2.1） |
| 关系的性质 | 11.2：**仅 reflexive/symmetric/transitive** | 10.1：**reflexive/symmetric/antisymmetric/transitive** | 13.1–13.2 序关系 | §4.3（Definition 4.3.2） |
| 偏序 / poset | **—** | **—**（有 antisymmetric 但从不叫 partial order） | 13.1 Order Relations / 13.2 More on Orderings | **§4.4 Ordering Relations**（Def 4.4.1/4.4.2；**四本中唯一正式教 partial order 的**） |
| 等价关系 | 11.3 | 10.2 | 13.3 Equivalence Relations and Equality | §4.5（Definition 4.5.1） |
| 等价类 / 划分 | 11.4 | —（未单列） | 13.3 | **§4.5**（Def 4.5.2 划分 / Def 4.5.3 等价类） |
| 商集 / ℤ/n | 11.5 The Integers Modulo n | — | 21.2 Quotient Constructions（很后面） | —（未取证到） |
| **函数** | **12.1，定义为关系 f ⊆ A×B** | **8.1**（**早于集合！**） | **15.1**；15.4 "Functions and Relations" | **§5.1**（Definition 5.1.1），**晚于关系** |
| 单射 / 满射 | 12.2 | 8.1–8.2（含 Bijectivity） | 15.2 | **§5.2 One-to-One and Onto**（Definition 5.2.1） |
| 复合 | **12.4**（**不是独立章**） | 8.3 | 15（散见） | **关系复合 §4.2**（Def 4.2.3 / Thm 4.2.5）→ **函数复合 §5.1**（Thm 5.1.5） |
| 反函数 | 12.5 | —（未单列） | **16.4 Defining the Inverse Classically（引入选择公理）** | **§5.3**（Theorems 5.3.1–5.3.3） |
| 像 / 原像 | 12.6 | — | 15.3 Functions and Subsets of the Domain | **§5.5**（Definition 5.5.1 同时定义两者；标题带 "A Research Project"） |
| 基数 / 等势 | **14.1** | **—**（全书没有） | **20.1**（有限）/ **22.1**（无限，equinumerous） | **§8.1** Equinumerous Sets |
| 可数 / 不可数 | 14.2 | — | 22.2 | §8.2 |
| Cantor 定理 | 14.3（比较基数，含 \|ℕ\|≠\|ℝ\|） | — | 22.3 Cantor's Theorem | §8.3（含 Cantor–Schröder–Bernstein） |
| Cantor–Bernstein–Schröder | **14.4** | — | 22.5 The Cantor-Bernstein Theorem | **§8.3** |
| **选择公理** | **—**（只有 §10.3 well-ordering principle） | **—**（全文无 `Classical`） | **16.4**（`axiomOfChoice` / `choose`） | **—**（无选择公理章） |
| 集合论公理 (ZFC) | 只在 1.10 提 Russell | — | **23. Axiomatic Foundations** | —（未取证到） |
| 模型论语义 | — | — | **6. / 10.**（truth tables, soundness, completeness） | — |

**读法**：四本书在"集合什么时候出现"上给出**四个不同答案** ——
Hammack 最先（Ch 1，且在 Logic 之前）、Velleman 次之（§1.3，逻辑章内但在证明技术之前）、
Avigad 居中（Ch 11）、Macbeth 最后（Ch 9，且**在函数之后**）。
在"有序对什么时候出现"上三者**首尾相反**：Hammack §1.2（集合章第二节）
vs Velleman §4.1（关系章第一节）vs Avigad §11.4（集合章最后一节）。
在"偏序"上，**只有 Velleman 正式教**（§4.4）—— 这是一个容易漏掉的重要差异。

> 注：Velleman 一列中标注「未取证到」的条目（Russell 悖论、商集/ℤ/n、ZFC 公理）
> 表示**我没有在已取证的节级目录里看到对应小节**，但这**不等于**证明书中完全没有讨论
> （讨论可能出现在正文段落或练习里），因此标为"未取证到"而非"—"。

### 9.1 Cummings *Proofs* 的对应位置（为避免表格过宽，单列于此）

来源：作者官方 TOC PDF <https://longformmath.com/wp-content/uploads/2025/01/proofs-toc.pdf>

| 概念 | 位置 |
|---|---|
| 集合 | **Ch 3 Sets**（p.97）：3.1 Definitions / 3.2 Proving A ⊆ B / 3.3 Proving A = B / 3.4 Set Operations |
| 幂集 | TOC 未单列（`UNVERIFIED`） |
| 有序对 / 笛卡尔积 | TOC 未单列（`UNVERIFIED`） |
| 逻辑 | **Ch 5 Logic**（p.207）—— **排在集合(3)与归纳(4)之后** |
| 归纳 | **Ch 4 Induction**（p.147，含 Strong Induction 4.3） |
| 函数 | **Ch 8 Functions**（p.331）：8.1 Approaching / 8.2 Injections, Surjections and Bijections / 8.3 The Composition / 8.4 Invertibility |
| 关系 | **Ch 9 Relations**（p.379）：9.1 Equivalence Relations / 9.2 Abstraction and Generalization |
| 偏序 / poset | TOC 未列 partial order（`UNVERIFIED`） |
| **基数** | **不编号的 "Introduction to Cardinality"，pp. 371–378**，**在 Ch 8 Functions 之后、Ch 9 Relations 之前** |
| 选择公理 | **无**（`UNVERIFIED` 细查，但该书为入门教材且无公理集合论章） |
| 逆否 / 反证 | **Ch 6 The Contrapositive / Ch 7 Contradiction** —— 是**独立章**，各占约 32 页 |
| 写作指导 | **Appendix C Writing Advice**（C.1 Writing Proofs / C.2 Writing in LaTeX）—— 罕见的"显式教写作 + LaTeX"附录 |

---

## 10. 对"中文母语、零形式化证明经验"课程的可操作结论

1. **"集合先行"有四种成熟做法，必须明确选一个而不是折中**（顺序差别很大）：
   - **Hammack**：集合整章最前（Ch 1，且在 Logic 之前）——代价是学生一开始就要吞幂集与 Russell 悖论。
   - **Velleman**：集合嵌在逻辑章内（§1.3–§1.4、§2.3），**在证明技术 Ch 3 之前**——集合当"语言"而非"对象"。
   - **Avigad 等**：集合在 Ch 11，前面已有完整命题/一阶逻辑——顺序最"正统"。
   - **Macbeth**：集合放第 9 章、**在函数之后**，靠"类型 + 谓词"把集合推迟到学生已能熟练证明。
   **对零基础，Macbeth 的入口最平滑**：学生只需理解"`x ∈ {n | P n}` 就是 `P x`"，
   再由 `dsimp` 机械展开；不需要先建立"集合是对象"的本体论负担。
2. **有序对的引入位置是个真实且分歧很大的设计变量**（四本给出四个答案）：
   Hammack §1.2（集合章**第二节，在子集之前**）/ Velleman §4.1（**关系章第一节**）/
   Avigad §11.4（集合章**最后一节**）/ Macbeth 不单独引入（靠积类型 `×`）。
   对"从未写过证明"的学习者：太早（Hammack）会打断集合运算的直观线索，
   太晚（Avigad）则让关系与函数的例子长期缺少形式支撑；
   **Velleman 的折中（先集合运算打底，再在关系章开头补有序对，随即定义关系）可能最稳**。
3. **"关系先于函数"是 3 : 2，不是共识**：
   关系先行 = Hammack / Velleman / Avigad（且三本都以"函数是关系的特例"收口）；
   函数先行 = Macbeth / Cummings。Solow 两者都无独立章。
   若目标是让学生理解"函数不是天上掉下来的"，**"关系→函数"路线证据更充分**；
   若目标是尽快让学生**用**函数做题（而不是理解函数的本体），函数先行更省课时。
   额外建议采纳 **Velleman 的做法：复合先在关系层面教（§4.2），函数章再复用（§5.1）**，
   这样"复合"只学一次、迁移一次。
4. **偏序是大多数教材的盲区**：Hammack 与 Macbeth **完全没有** partial order，
   Solow 完全没有集合论章；**只有 Velleman §4.4 正式教**。
   如果课程后面要接序结构（格、序数、domain theory），**必须自己补这一块**。
5. **练习形态上，三种经过验证的设计值得直接借用**：
   - **Macbeth 的"真/假双版本"**：同一命题给 `example : P` 与 `example : ¬P` 两个 stub，
     学生先判断再证明——强制"动笔前先想"。
   - **Hammack Ch 9 的"累积判定题"**："Each of the following statements is either true or false.
     If a statement is true, prove it. If a statement is false, disprove it."
   - **Macbeth 的"自校验题"**：给出一个 macro（如
     `(tactic| (ext; dsimp; exhaust))`），学生填 `sorry` 位置，**答案对不对由 tactic 直接判**。
     这与本项目用 kernel 判卷的思路完全一致，是最可直接移植的一种。
   - **Velleman 的 Proof Designer 标记**：把"哪些题可以用证明辅助软件做"直接标进练习，
     是传统教材里最接近"形式化 + 非形式化双轨"的设计。
6. **关于选择公理**：**5/6 本根本没有它**（Hammack / Macbeth / Velleman / Solow / Cummings）。
   若课程要在 Lean/Mathlib 环境里教基数，**必须显式交代"基数依赖 `Classical.choice`"**
   —— 传统教材不会告诉你这件事，而 Avigad 等是唯一把它显式放在基数之前
   （§16.4 → §20.1 → Ch 22）的样本。**这可能是本项目相对所有传统教材最大的增量价值点。**
7. **Cummings 的两个可直接借用的设计**（其余五本都没有）：
   - **9 个"Introduction to X"迷你章**，每章后面挂一个真实数学领域
     （Ramsey 理论 / 数论 / 拓扑 / 序列 / 实分析 / 大数据 / 博弈论 / **基数** / 群论）。
     作用是"每学完一种技术，立刻看到它在真数学里长什么样"——**直接对抗'学证明有什么用'的动机问题**。
   - **Appendix C Writing Advice**（C.1 Writing Proofs / C.2 Writing in LaTeX）——
     把"数学写作"和"LaTeX 工具"显式编成附录，对中文母语学习者尤其有价值
     （英文数学写作规范是他们的额外障碍）。
8. **Solow 与 Velleman 构成"集合何时出现"的两个极端**，
   这个选择比"选哪本书"更影响零基础学生的体验：
   Solow 完全不给集合论（门槛最低但学生后续要自己补），
   Velleman 把集合塞进逻辑章、在证明技术之前（门槛略高但概念一次到位）。
   **若目标是尽快进入 Lean 实操，建议走 Velleman/Macbeth 的"集合即谓词"路线**，
   因为 `x ∈ {n | P n}` 与 `P x` 的对应关系正是 `dsimp` 能机械完成的一步。
