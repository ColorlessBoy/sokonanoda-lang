# 集合论教学调研（课堂与教材）

> **取证日期**：2026-09-18。**取证方式（重要）**：本会话的 `web_search` / `web_fetch` 两个工具
> **全程不可用**（`web_search` 报「no API key for DEEPSEEK_API_KEY」；`web_fetch` 报 `fetch failed`）。
> 全部来源改由 `bash` + `curl` 经本机代理 `HTTPS_PROXY=http://127.0.0.1:7890` 取得，
> HTML 用 `pandoc` / `python3` 转文本，PDF 用 PyMuPDF 抽取。**凡未能取到的一律标注「未核实」，
> 本报告不含任何凭记忆补写的目录或引文。**
>
> **本项目实测**：§5 的「必证定理」列凡带 ✓ 者，均已在 sokonanoda 真实内核上判过
> （探针 `repro/lib.sokonanoda` = 29 checked / 0 failed；`repro/lib2.sokonanoda` = 10 checked / 1 failed）。
> 带「待实测」者为设计推断，尚未判卷。
>
> **适用约束**：声明只有 `def`/`theorem`/`example`/`axiom`/`inductive…ctor…end`/`import`；
> 项只有 `fun`/`let`/`match`/应用/箭头/`by`；**无类型类、无结构体、无 notation/infix、无 namespace、
> 无 rewrite/simp/omega**；tactic 只有 `intro`/`exact`/`apply`/`assumption`/`rfl`/`match`/`sorry`；
> 集合恒为谓词 `Set α := α → Prop`（无 ZF 全域，`∈` 非原始）；受众为**零形式化证明经验的中文学习者**。

---

## §0 一屏结论

1. **「集合先讲还是函数先讲」没有共识**：Hammack 把集合放全书第 1 章（甚至在逻辑之前），
   Macbeth 把集合放到第 9 章、**在函数（8）与关系（10）之后**。我们的语言里
   `Set α := α → Prop` 本身就是函数，所以 **Macbeth 路线（函数→集合→关系）在本语言里更自然**。
2. **「关系先于函数」是 3:2，不是共识**：关系先 = Velleman(4→5)、Hammack(11→12)、Avigad(13→15)；
   函数先 = Macbeth(8→9→10)、Cummings(8→9)。Velleman 有一个可偷的招：
   **复合只在关系层教一次，函数层直接复用**。
3. **六本证明教材里五本根本没有选择公理**（Hammack / Macbeth / Velleman / Solow / Cummings）。
   唯一有 AC 的 Avigad 是「选择(§16.4) → 有限基数(§20.1) → 无限基数(Ch22)」。
   唯一的**反例**是 Tao *Analysis I*：基数在 §3.6，选择公理远在 §8.4。
   而 Mathlib 的基数建立在 `Classical.choice` 上 —— **这是传统教材不会告诉学生、
   而本课程可以提供的最大增量**。
4. **有序对的集合编码（Kuratowski）在成熟教材里普遍被降级为练习**：Tao 把它做成
   Exercise 3.5.1，Velleman 全文不出现「Kuratowski」。本语言应取**原始积类型**，
   并把 Kuratowski 做成阅读材料（**明确标注这是偏离 ZF 忠实路线的取舍**）。
5. 内核实测暴露三条**必须写进课程写作规范**的硬约束：`axiom` 不能带 binder 参数表；
   等式必须写 `Eq.{1}`；`Eq.subst` / `Eq.refl` 必须显式给宇宙。

---

## §1 教材对照表

> 表内每一行的「结构」列都来自**实际抓到的目录**（URL 见 §7）。标「未核实」者本轮未取到证据。

| 书 | 结构（已核实的章序） | 集合 / 函数 / 关系 / 基数 的位置 | 显著装置 | 一句话 takeaway |
|---|---|---|---|---|
| **Halmos, _Naive Set Theory_**（Van Nostrand 1960 / Springer UTM 1974） | 25 章：1 外延公理 · 2 概括公理 · 3 无序对 · 4 并交 · **5 补与幂** · 6 有序对 · 7 关系 · 8 函数 · 9 族 · 10 逆与复合 · 11 数 · 12 Peano 公理 · 13 算术 · 14 序 · **15 选择公理** · 16 Zorn · 17 良序 · 18 超限递归 · 19 序数 · 20 序数集 · 21 序数算术 · 22 Schröder–Bernstein · 23 可数集 · 24 基数算术 · 25 基数 | 集合/∈ 从第 1 章起；**子集没有独立章**；**幂集与补集合并在第 5 章**；关系(7)→函数(8)；**选择(15) 远早于基数(22–25)** | 全书 25 章很短，每章是「一条公理 → 一段散文」；把「集合论是数学的底层」当叙事主线 | 唯一「选择先于基数」的经典；**没有 Russell 悖论章**（25 章目录里不存在） |
| **Enderton, _Elements of Set Theory_**（Academic Press 1977） | 1 Introduction（Baby Set Theory · Sets—An Informal View · Classes · Axiomatic Method · Notation · Historical Notes）· 2 Axioms and Operations · **3 Relations and Functions**（Ordered Pairs · Relations · n-Ary Relations · Functions · Infinite Cartesian Products · Equivalence Relations · Ordering Relations）· 4 Natural Numbers · 5 Construction of the Real Numbers · **6 Cardinal Numbers and the Axiom of Choice** · 7 Orderings and Ordinals · 8 Ordinals and Order Types · 9 Special Topics | **关系与函数合为一章**：有序对 §3.1 → 关系 §3.2 → 函数 §3.4 → 等价关系 §3.6 → 序关系 §3.7；**基数与选择公理合为一章**，AC 夹在「基数算术」与「可数集」之间 | **每章末 Review Exercises**；Ch1 先摆「类 / 公理化方法 / 历史注记」再上公理 | 最像「教科书」的一本：概念成对打包（关系+函数、基数+选择），适合当**参考手册**而非入门 |
| **Jech, _Set Theory_**（3rd millennium ed.） | **未核实**（Springer 反爬，本轮 3 次尝试均返回 Client Challenge；Open Library 有 3 条记录但无目录） | 未核实 | 未核实 | 研究生/研究向，**不建议**用作本课程的结构参照 |
| **Kunen, _Set Theory_** | **未核实**（本轮未取得目录） | 未核实 | 未核实 | 同上；独立性证明路线，与本课程目标无关 |
| **Tao, _Analysis I_**（第 3 版）第 3 章 | **3.1 Fundamentals · 3.2 Russell's Paradox (Optional) · 3.3 Functions · 3.4 Images and Inverse Images · 3.5 Cartesian Products · 3.6 Cardinality of Sets**；选择公理在 **§8.4** | 集合从 3.1 起；**函数(3.3) 在笛卡尔积(3.5) 之前**；**像与原像(3.4) 在笛卡尔积之前**；基数 3.6 收尾；**选择公理远在 Ch8** | Def 3.3.1 用**垂直线检验**定义函数（函数是「对象」，不是集合对）；Remark 3.3.2 把编码成 `(X,Y,图)` 降级为**练习 3.5.10**；Exercise 3.5.1 把 **Kuratowski 定义本身做成练习**；Exercise 3.5.2 反过来**用函数定义 n-元组** | 对本课程最有价值的一本：**「函数原始、序对延后、编码是练习」**正是谓词化集合论该有的姿态 |
| **Velleman, _How to Prove It_ 3e** | 8 章：1 Sentential Logic（1.3 Variables and Sets · 1.4 Operations on Sets）· 2 Quantificational Logic（2.3 More Operations on Sets）· 3 Proofs · **4 Relations**（4.1 Ordered Pairs and Cartesian Products · 4.2 Relations · 4.3 More About Relations · 4.4 Ordering Relations · 4.5 Equivalence Relations）· **5 Functions**（5.1 Functions · 5.2 One-to-One and Onto · 5.3 Inverses · 5.4 Closures · **5.5 Images and Inverse Images: A Research Project**）· 6 Mathematical Induction · 7 Number Theory · 8 Infinite Sets | **集合藏在逻辑章里、且在一切证明技巧之前**（§1.3/1.4/2.3）；有序对 §4.1 → 关系 §4.2 → 函数 §5.1；**无选择公理**；基数 = Ch8 | Def 5.1.1：**函数就是关系** `F ⊆ A×B` 满足 `∀a∈A ∃!b∈B (a,b)∈F`；**复合先在关系层教（Def 4.2.3 / Thm 4.2.5），函数层直接复用（Thm 5.1.5）**；附录 **Proof Designer**（半形式化证明器）；5.5 把像/原像做成**研究项目** | 「读证明」路线的标杆；**唯一系统讲偏序/全序**（§4.4）。注意：3e 是 8 章（Ch7 数论为新章），2e 才是「Ch7 = 无限集」 |
| **Hammack, _Book of Proof_ 3e** | 14 章，四部分：**I 基础** 1 Sets（1.1 Introduction · **1.2 The Cartesian Product** · 1.3 Subsets · 1.4 Power Sets · 1.5 并交差 · 1.6 补 · 1.7 Venn · 1.8 索引族 · 1.9 数系 · **1.10 Russell's Paradox**）· 2 Logic · 3 Counting；**II 条件命题** 4 Direct · 5 Contrapositive · 6 Contradiction；**III 更多证明** 7 Non-Conditional · 8 Proofs Involving Sets · 9 Disproof · 10 Induction；**IV 关系、函数与基数** 11 Relations · 12 Functions · 13 Proofs in Calculus · 14 Cardinality of Sets | **集合是全书第 1 章，且在逻辑之前**；**有序对(1.2) 在子集(1.3) 与幂集(1.4) 之前**；关系 11 → 函数 12（**函数定义为关系**）；基数 14；**完全没有选择公理**（只有 §10.3 良序原理） | 练习分块：Ch1–3 与 Ch11–14 **按节**给题，**Ch4–10 每章只有一块**；Ch9 是跨 Ch1–9 的累积式「证明或推翻」；书末有解答 | 免费开放（CC BY-NC-ND）；**结构上最激进**：集合/幂集/悖论全部前置到第 1 章 |
| **Solow, _How to Read and Do Proofs_ 6e** | 18 章，**没有任何集合论章、没有任何函数章**。Part I（Ch1–15）= 纯证明技巧：The Truth of It All · Forward-Backward · Definitions and Terminology · **Quantifiers I–IV**（Construction / Choose / Specialization / Nested）· Nots of Nots · Contradiction · Contrapositive · Uniqueness · Induction · Either-Or · Max/Min · Summary；Part II（Ch16–18）= Generalization / Creating Definitions / Axiomatic Systems；附录 A–D = 来自离散数学、线性代数、抽象代数、实分析的例题 | 集合与函数**从不作为主题讲授**，只在第 1 章起当记号使用 | 技巧优先到极端：把「量词」拆成四章逐一训练；练习编号 `<章>.<n>`，每章 6–23 题，只有一部分给答案 | 证明技巧的**分解粒度**值得偷（量词四拆）；但**结构上不能照搬**——本课程必须讲集合 |
| **Cummings, _Proofs: A Long-Form Mathematics Textbook_** | 9 章：1 Intuitive Proofs · 2 Direct Proofs · **3 Sets** · 4 Induction · **5 Logic** · 6 The Contrapositive · 7 Contradiction · **8 Functions** · 9 Relations；附录 A 其他证明方法 · B Proofs From The Book · C 写作建议（含 C.2 用 LaTeX 写作） | 集合 Ch3（**先于逻辑**）；**函数(8) 先于关系(9)**；**基数是一个不编号的插章，夹在函数与关系之间（pp.371–378）** | 9 个不编号的「Introduction to X」小插章（Ramsey 理论、拓扑、序列、实分析、大数、博弈论、基数、群论）；每章 10 题给完整解答、其余只给提示 | 非开放获取（需购买）；**「先把证明写起来再补逻辑」**的胆量值得注意 |
| **Macbeth, _The Mechanics of Proof_** | 10 章：1 Proofs by calculation · 2 Proofs with structure · 3 Parity and divisibility · 4 Proofs with structure II · 5 Logic · 6 Induction · 7 Number theory · **8 Functions · 9 Sets · 10 Relations** | **函数(8) 先于集合(9)、先于关系(10)**；**没有基数章、没有选择公理** | **集合 = 谓词**（「a set in a type X is specified by a predicate on X」）；隶属靠 `dsimp` 展开；练习是**成对 stub**（`example : P := by sorry` / `example : ¬P := by sorry`），另有「给一条 tactic，能用当且仅当你表述正确」的自检题 | **Lean 4 教材里与本课程最近的形态**；「集合=谓词 + 展开」正是我们要的入口 |
| **Avigad/Lewis/van Doorn；Lean 4 改编 Joseph Hua, _Logic and Proof_** | 24 章，散文章与 Lean 章成对：… 11 Sets · 13 Relations · 15 Functions · 20 Combinatorics（有限基数）· 22 The Infinite · 23 Axiomatic Foundations | 集合 11 → 关系 13 → 函数 15；**笛卡尔积与幂集被塞进 §11.4，集合章的最后一节**（与 Hammack 的 §1.2 恰好相反）；**选择 §16.4 → 有限基数 §20.1 → 无限基数 Ch22** | **不是 `simp` 重写型**：每章 `simp` 0–4 次、`norm_num`/`omega`/`exact?` 为 0，主力是 `intro`/`apply`/`exact`/`cases`/`use`/`rw`（即自然演绎规则的同构）；**§4.8 要求同一命题同时用 term mode 与 tactic mode 写两遍** | 自动化程度与我们的 tactic 白名单最接近的一本；**双写法练习**是「形式↔非形式互译」的现成范式 |
| **中文教材**（徐明曜/赵春来《集合论》、耿素云《集合论与图论》、张锦文《公理集合论导引》等） | **全部未核实** | 未核实 | 未核实 | 本轮中文取证子任务被中止，**未取得任何一份中文教材目录或课程大纲**；见 §3 末尾与 §7 |

---

## §2 顺序之争

每一小节 = 「来源实际怎么做」+「本课程的建议」+ 来源链接。

### 2.1 子集 与 幂集：谁先？

- **实际做法**：**Hammack** 1.3 Subsets → **1.4 Power Sets**，幂集**紧跟**子集，
  且**有序对(1.2) 还在子集之前**。**Halmos** 根本没有独立的「子集」章，
  幂集与补集合并成第 5 章 _Complements and Powers_，排在并交(4) 之后、有序对(6) 之前。
  **Avigad** 把幂集与笛卡尔积一起丢到 **§11.4，集合章的最后一节**。
- **建议**：**子集 → 幂集紧随**（Hammack 式）。理由是本语言里的成本极低：
  `def Set.power (α : Type) (A : Set α) : Set (Set α) := fun (B : Set α) => Set.subset α B A`，
  而「`B ∈ 𝒫(A)` 就是 `B ⊆ A`」**一次 `fun` 展开即证**——实测 `mem_power` ✓ 通过（`repro/lib.sokonanoda`）。
  把幂集延后（Avigad 式）只会让后面「`Set (Set α)` 是什么宇宙」这个问题积压到更难的位置。
- 来源：[Hammack 目录](https://richardhammack.github.io/BookOfProof/Main.pdf) ·
  [Halmos 官方目录](https://link.springer.com/book/10.1007/978-1-4757-1645-0)

### 2.2 有序对：Kuratowski 编码 还是 原始？

- **实际做法**：**Tao** 正文**不给编码**——他用**垂直线检验**把函数定义成原始对象
  （Definition 3.3.1），然后在 Remark 3.3.2 里说「函数对象的存在性公理是多余的，
  因为可以编码成有序三元组 `(X, Y, {(x, f(x)) : x ∈ X})`，见 **Exercise 3.5.10**」；
  **Kuratowski 定义本身就是 Exercise 3.5.1(i)**，同一题还要求证 (ii) 短定义
  `(x,y) := {x, {x,y}}`（并提示需要正则公理）与 (iii) 「无论用哪种编码，`X × Y` 都是集合」。
  **Velleman** 只给直觉说明，**2e 全文检索「Kuratowski」为 0 次**。
  **Enderton** 则在 §3.1 Ordered Pairs 正面给出集合编码。
  Wikipedia 的 [Ordered pair](https://en.wikipedia.org/wiki/Ordered_pair) 条目直接总结了这个现象：
  「Even those mathematical textbooks that give an informal definition of ordered pairs
  will often mention the formal definition of Kuratowski **in an exercise**」，
  并给出 Kuratowski 1921 为 now-accepted、Wiener 1914 与 Hausdorff 1914 的更早定义。
- **建议**：**取原始积类型** `inductive Prod (α : Type) (β : Type) : Type / ctor mk …`，
  把 Kuratowski 降级为**阅读材料 + 一道「比较两种编码」的思考题**（直接借 Tao Exercise 3.5.1 的三段式）。
  **必须明确标注这是偏离 ZF 忠实路线的取舍**：在 `Set α := α → Prop` 下做 Kuratowski 需要
  `Set (Set α)`，而其特征性质 `(a,b)=(c,d) → a=c ∧ b=d` 的证明需要大量 `match`/`Eq.subst` 甚至外延性，
  在「只有 intro/exact/apply/assumption/rfl/match」的白名单下**不可行**。
  实测：`inductive Prod` + `ctor mk` 单独判卷 ✓ 通过。
- 来源：[Tao Exercise 3.5.1 原文](https://archive.org/details/terrence-tao-analysis-i) ·
  [Ordered pair (Wikipedia)](https://en.wikipedia.org/wiki/Ordered_pair)

### 2.3 函数：集合对 还是 映射？

- **实际做法**：**Velleman Definition 5.1.1**（原文取自 2e PDF）：
  「Suppose F is a relation from A to B. Then F is called a **function** from A to B if for every
  a ∈ A there is exactly one b ∈ B such that (a,b) ∈ F. In other words … `∀a ∈ A ∃!b ∈ B((a,b) ∈ F)`」
  ——函数**就是**关系。**Hammack** 同样在 Def 12.1 把函数定义为关系 `f ⊆ A×B`。
  **Tao** 相反：函数是**原始对象**（Def 3.3.1），编码是注释 + 练习。
- **建议**：本语言站在 **Tao 一侧**。`α → β` 是原始类型构造子，`Set α := α → Prop` 本身就是函数，
  所以「函数 = 集合对」在本语言里**只能是反向的对照单元**：
  `def Graph (α : Type) (β : Type) (f : α -> β) (a : α) (b : β) : Prop := Eq.{1} β (f a) b`，
  然后让学生看到「单值性」在这个写法下是 `rfl` 级的平凡事实。
  教学价值在于**让学生看到两种本体论都存在且都自洽**——这是数学约定，不是真理。
- 来源：[Velleman（2e 全文 PDF，METU 课程镜像）](https://users.metu.edu.tr/serge/courses/111-2011/textbook-math111.pdf) ·
  [Tao Analysis I](https://archive.org/details/terrence-tao-analysis-i) ·
  [Hammack](https://richardhammack.github.io/BookOfProof/Main.pdf)

### 2.4 关系 先于 还是 后于 函数？

- **实际做法（3:2，不是共识）**：**关系先** = Velleman(4→5)、Hammack(11→12)、Avigad(13→15)；
  **函数先** = Macbeth(**8 Functions → 9 Sets → 10 Relations**)、Cummings(**8 Functions → 9 Relations**)；
  **Solow 两者都没有**。
- **建议**：**函数先**（Macbeth / Cummings 路线）。理由有二：
  ① 类型论里 `α → β` 是原始，`Set α := α → Prop` 是它的一个特例，
  「关系 = 返回 `Prop` 的二元函数」`α → β → Prop` 是**顺着类型往下走**；
  ② **Velleman 的可偷之招**——复合只教一次。我们可以在函数层教 `Fun.comp`，
  关系层直接复用同一个定义，省掉一整个单元。
  实测：`Fun.comp` ✓、`comp_injective` ✓、`Rel.comp` ✓、`rel_comp_assoc` ✓ 全部通过内核。
- 来源：[Velleman 3e（CUP 官方章节 JSON）](https://www.cambridge.org/core/product/identifier/9781108539890/type/book) ·
  [Macbeth 目录](https://hrmacbeth.github.io/math2001/) ·
  [Hammack](https://richardhammack.github.io/BookOfProof/Main.pdf)

### 2.5 基数 先于 还是 后于 选择公理？

| 来源 | 选择公理 | 基数 | 先后 |
|---|---|---|---|
| Halmos | Ch 15 | Ch 22–25 | **选择先** |
| Enderton | Ch 6 内（基数算术之后、可数集之前） | Ch 6 | 同一章，选择居中 |
| **Tao** | **§8.4** | **§3.6** | **基数先** ← 唯一反例 |
| Avigad 等 | §16.4 | §20.1 有限 / Ch22 无限 | **选择先** |
| Hammack | **无** | Ch 14 | 不适用 |
| Velleman | **无** | Ch 8 | 不适用 |
| Macbeth | **无** | **无** | 不适用 |
| Solow | **无** | **无** | 不适用 |
| Cummings | **无** | 不编号插章 | 不适用 |

- **结论**：**六本证明教材里五本完全没有选择公理**；有 AC 的都是「选择先」。
  **唯一把基数放在选择之前的是 Tao**，而且他做得很显式：Exercise 3.6.8 的题干里就写明
  「The converse to this statement requires the axiom of choice; see Exercise 8.4.3」，
  §3.5.12 也把「有限选择」与「无限选择需要新公理」分开。
- **建议**：**基数先、选择后**（Tao 路线），但**必须加一段显式对照**：
  「在 Lean/Mathlib 里基数理论建立在 `Classical.choice` 之上；本课程先做基数，
  是因为我们只在**能显式构造双射**的场合用基数，凡需要选择的地方我们会点名。」
  这正是 §0 结论 3 说的增量：**传统教材不会告诉学生这件事**。
- 来源：[Halmos](https://link.springer.com/book/10.1007/978-1-4757-1645-0) ·
  [Enderton](https://shop.elsevier.com/books/elements-of-set-theory/enderton/978-0-12-238440-0) ·
  [Tao](https://archive.org/details/terrence-tao-analysis-i) ·
  [Avigad 等](https://leanprover-community.github.io/logic_and_proof/)

### 2.6 Russell 悖论放哪里？

- **实际做法**：**Hammack 1.10**——第 1 章**最后一节**，学完幂集/索引族之后；
  **Tao 3.2**——紧跟 Fundamentals，**标题里直接写 `(Optional)`**，用途是解释
  「为什么不能有无限制概括公理」；**Halmos 完全没有这一章**（25 章目录里不存在）；
  **Enderton** 放在 Ch1 的 Classes / Axiomatic Method 一带（目录级证据，未读正文）。
- **建议**：放在 **第 9 单元「论域与为什么没有全集」**，作为**读一读 + 一道反证练习**，
  **不要放开头**。依据：两位作者一个把它放章末、一个明确标 Optional，
  说明它在教学上是「补充说明」而不是地基；而在谓词化集合论里，
  学生需要先理解 `Set α` **依赖 `α`**，才可能理解「不存在所有集合的集合」在本语言里
  其实是一句**关于论域的话**，而不是关于 ZF 的话。
- 来源：[Hammack](https://richardhammack.github.io/BookOfProof/Main.pdf) ·
  [Tao](https://archive.org/details/terrence-tao-analysis-i) ·
  [Halmos](https://link.springer.com/book/10.1007/978-1-4757-1645-0)

---

## §3 学习障碍

> **证据等级声明（必读）**：本节两栏。**「教材证据」栏**记录成熟教材**用哪些题防哪个坑**，
> 全部已核实。**「教育研究证据」栏**来自专门的文献取证（`learning-difficulties.md`，2874 行 / ~190 条来源），
> 每条带核验级别：[F]=读到全文、[A]=取到逐字摘要、[M]=仅元数据（**其结论未经核实**）。
> **两处「无文献」是实测的否定结果，不是没查到**：有序对编码（障碍 3）与选择公理（障碍 8）
> **不存在实证研究**——凡声称「研究表明学生对 AC 如何如何」的表述都是过度断言。

| 障碍 | 教材证据（已核实） | 教育研究证据（带核验级别） | 教学动作 |
|---|---|---|---|
| **`∈` 与 `⊆` 混淆；元素 vs 单元素** | Hammack §1.3 A 组把 `∅`（第 4 题）与 `{∅}`（第 5 题）并列要求「列出全部子集」；C 组要求判断 `ℝ³ ⊆ ℝ³` / `ℝ² ⊆ ℝ³` 真假并解释 | **Bagni 2006**, _ESM_ 62(3) [A]——唯一一篇正面研究「属于 vs 包含」的同行评审论文；困难 = 无法「区分并协调各符号系统（言语/图示/符号）的意义」。**最锋利的实测**：**Hendriyanto et al. 2024**, _J. Math. Educ._ 15(2):517–544 [F]——`M={r,s,t}`，183 名学生几乎全对 `r∈M`/`s∉M`，但 **`{r}∈M` 无人给出正确理由**，学生拒绝单元素集，作者归因为「分不清 ∈ 与 =」。另：Shaker & Berger 2016 [A]（把并/笛卡尔积的**定义**读错）；Zazkis & Gunn 1997 [M]（元素/子集/空集，ISETL 环境） | 开局就把两符号写成同一种东西的两种读法：`A x`（隶属，返回 `Prop`）与 `Set.subset α A B`（全称箭头）。**必须专门练 `{a}`**——单元素集是实证上最脆弱的一点 |
| **空集与空真** | Hammack §1.3 B 组第 11 题 `{X : X ⊆ {3,2,a} and \|X\| = 4}`——答案只能是 `∅` | 同一 Hendriyanto 2024 [F]：五个「这是集合吗」的对象里两个是 `∅`，「**许多学生不接受它是一个集合，理由全都一样：里面没有元素可被指认**」；一个只在教师权威下接受 `∅` 的学生被追问时「沉默，无法解释」。假前件：**Lee, Park & Kim 2024**, _IJSME_ [A]（社会契约式「许诺/违约」情境有效）；**Durand-Guerrier 2003**, _ESM_ 53(1):5–34 [A]（须从「联结词式蕴涵」转到**开语句**上的蕴涵）。**最强的课堂量化证据**：**Dubinsky 1997** [F]——「学生做得差（<60%）的七道题里，**五道涉及一个不属于量化部分的蕴涵**」；某题除一人外全都正确否定了量词，**错的全在蕴涵上** | 用 `False.rec` 的**类型**当教具：`(C : Prop) -> False -> C`，`C` 任意 ⇒「从不可能出发什么都能证」是**规则**不是诡计。实测最短证明 `empty_subset ✓`。**先让学生猜「证 `∅ ⊆ A` 需要做什么」** |
| **有序对编码（Kuratowski）** | Tao Exercise 3.5.1 三段；Velleman 全文无「Kuratowski」；Wikipedia 指出编码常被降级为练习 | **⚠️ 实测否定结果：不存在关于 Kuratowski 编码的实证研究**——ERIC 查 `"Kuratowski ordered pair"` = **0 条**。可引用的替代：**Mirin, Weber & Wasserman 2020** PME-NA [F]——两种不等价的「函数」定义（Bourbaki 三元组 vs 集合对）**同时在使用**，对同一学生问题给出不同答案，而作者们「常常不说明自己用的是哪个定义」；**Kanamori 2003**, _BSL_ [A]（`⟨x,y⟩`/`∅`/`{a}` 需要从内涵观点转向外延观点）。**本报告不得为这一条引用「研究表明」** | 用原始 `inductive Prod`；Kuratowski 只做阅读题。**把「为什么教材都把它放进练习」本身当讨论题**——这同时是 Mirin et al. 那条「定义不唯一」的活例子 |
| **函数 = 集合对 vs 规则/映射** | Velleman Def 5.1.1（函数就是关系）与 Tao Def 3.3.1（函数是原始对象）**正面对撞** | **Breidenbach, Dubinsky, Hawks & Nichols 1992**, _ESM_ 23(3):247–285 [F]——本体论问题**被测量过**：24 个情境、约 60 名本科生，本该 ~100% 判为函数的场合**只有约 40%** 说「是函数」；分表征看 图 19–40.7%、方程 25–31%、表格 39.9–48.6%；学生「坚持要有因果性」才肯承认一个过程。理论框架：Sfard 1991 [**M，仅元数据——不得归因任何结论**]、Vinner 1983、Tall & Vinner 1981、Tall & Bakar 1992、Thompson 1994、Even 1990/1993 | 做「同一命题两种本体论各写一遍」的对照题（借 Avigad §4.8 双写法装置），结论落在「这是约定」。**把 40% 这个数字讲给学生听**——它就是这门课存在的理由 |
| **单射/满射的量词顺序与否定** | Hammack §12.2 整块练习：`f(n)=2n+1`（单射非满射）· `f(m,n)=3n−4m` · `f(m,n)=2n−4m` · `θ(X)=X̄` · `f(x,y)=(xy,x³)` | **Dubinsky & Yiparaki 2000** [F]（**⚠️ 未发表手稿，引用时须注明状态**）：63 名学生 11 个陈述，**94% 至少把一个 EA 陈述读成 AE**（逐题 11%–81%），反向只有 **5%**（0%–3%）；两个**数学**陈述只有 41% 和 9% 正确，而自然语言陈述有 78% 给出有效论证——**学生察觉不到歧义**。同行评审补充：Dubinsky 1997 [F]；Selden & Selden 1995 [A]（**8.5% / 5%** 的「拆解」成功率）；**Shipman 2015**, _TMA_ [A]——点名教材病因：用真值表教蕴涵，「把 P 和 Q 当成各有真值的陈述」，**丢掉了隐藏的 `∀x`**。单射/满射专文：**Bansilal, Brijlall & Trigueros 2017**, _JMB_ 48:22–37 [M]（**付费墙，本节第 1 号待补缺口**）。**⚠️ 更正两条线索：不存在 Piatek-Jimenez 2004 的单射/满射论文；不存在 Cusi & Malara 2007 的量词论文** | 直接借这批题型，**每个都要求「先判真假再写证明」**。实测：`Injective`/`Surjective`/`comp_injective` ✓ 可证；`comp_surjective` 需先手写 `Eq.symm` |
| **像 vs 原像（前推/回拉）** | **Velleman 把 Images and Inverse Images 做成 §5.5「A Research Project」**；Tao §3.4 明确「`f` 不必可逆，`f⁻¹(U)` 也有意义」；Hammack §12.6 独立成节 | **文献最薄的一条，须如实说明**：不存在针对 `f(A∩B) ⊆ f(A)∩f(B)` 不可逆这一失败点的测量研究。最接近的一手来源是 Breidenbach et al. 1992 [F]，其四周教学**显式区分**正向的 image 计算与「**把函数的过程反过来跑**……计算 preimage、构造反函数、1-1 与 onto」，理论主张是学生能封装却**不能「解封装」**。最贴题的专门来源：**Hamdan 2006**, _ESM_ 62(2):127–147 [A]——APOS **遗传分解**，其中心对象**就是原像**：「函数的**纤维结构**（即所有 `{b}` 的原像之集）」，连到划分与等价类。**⚠️ Asiala et al. 1996 全文检索 preimage/pre-image/inverse image 零命中——不得用它支撑本条** | 用「保交 vs 只保包含」当分水岭：实测 `image_subset_image` ✓。**必须先让学生猜「像保不保交」**，因为文献说明这里没有现成的错误模式可预告 |
| **可数/不可数与对角线** | Hammack Ch14（14.1–14.4）；Velleman Ch8；Tao §3.6 | **Hamza & O'Shea 2011**, MEI 4:192–202 [F]：35 人（含跨科数学教师）——「可数」被读成「**能被物理地数**」，于是可数≡有限、无限≡不可数；断言「不可数集的子集都不可数」「不可数集都等势」；**援引双射判据的人没有一个在所有题上都用它，且极少真的写出映射**。对角线接受度：**Zazkis & Mamolo 2009**, _FLM_ 29(3):53–56 [F]「Sean vs. Cantor」——一名硕士生的假 ℝ 枚举抵挡了多次反驳；**另一批已学过 Cantor 定理的学生只说「Cool!」并点头，没有发现矛盾**。整体—部分直觉：Monaghan 1986 华威博士论文 [F]（测量情境会让学生给超集**更大的基数**）。另：Dubinsky/Weller/McDonald/Brown 2005 _ESM_ Parts 1&2 [A] | 对角线论证在本语言里**只需 `fun`**。**必须让学生亲手写那个映射**——这正是实证上学生最常跳过的一步。Hammack 14.1「描述一个双射」的题型直接可用 |
| **选择公理** | 六本证明教材里五本没有 AC；唯一有的 Avigad 是「选择先于基数」；Tao 是唯一「基数先」 | **⚠️ 实测否定结果，且很强：不存在关于学生或教师对 AC 的态度的实证研究，也不存在 AC 教学研究。**ERIC 查 `"axiom of choice"`（加引号、全年代）= **1 条且不相关**；加 `teaching` = **0 条**；十种 OpenAlex 题摘措辞 + OpenAlex 全文检索 + 五次 Crossref + arXiv math.HO 扫描均无所获。**任何交付物都不得声称「研究表明学生对 AC 持 X 看法」。**唯一明确的 AC 教学论断是 **Förster 2006**（预印本，无 DOI，全文不可达）：「大学教学惯于**略过**选择公理的应用……**学生因此没有形成该公理的心智图像，日后也认不出它何时被使用**」。可用近邻：Bell 的 SEP 条目；**Incatasciato & Sánchez Terraf 2024** [arXiv:2404.11638]——**在 Lean 中验证过的 Zorn 引理最精简证明**，对「让 AC 保持显式」的课程直接可用；Wan, Xu & Cao 2023（Coq 版 ZFC 教学）；Dawkins 2018 [A]（学生**认为公理是什么**，五类，最成问题的是「指称式」观点） | 做成**对照单元**而非证明单元：讲清哪些命题需要它（Tao §3.5.12 有限选择 vs 无限选择、Exercise 3.6.8 的反向蕴涵），并**显式说明 Mathlib 的基数依赖 `Classical.choice`**。Förster 那句话可以直接当单元导语 |
| **过渡到证明（总论）** | Velleman/Solow 整本书的存在本身；Avigad §4.8 双写法 | **Moore 1994**, _ESM_ 27(3):249–266 [A]——奠基性研究，三大困难源：「**(a) 概念理解、(b) 数学语言与记号、(c) 不知如何下笔**」。**Selden & Selden 1995**, _ESM_ 29(2):123–151 [A]——61 名学生，「**仅 8.5% 的拆解尝试成功**」（简化陈述），真实教材陈述降到 **5%**；提出 *statement image* 与 *proof framework*。**Selden & Selden 2003**, _JRME_ 34(1):4–36 [A]——学生「**聚焦表面特征**」，其验证能力「**非常有限——可能比师生自己意识到的更有限**」。**Weber 2001**, _ESM_ 48(1):101–119 [A]——本科生「知道也**能应用**所需事实却仍然失败」，因为「**用不上自己拥有的句法知识**」。**Inglis & Alcock 2012**, _JRME_ 43(4):358–390 [A]——眼动实验证实新手看表面特征而非逻辑结构。**Harel & Sowder 1998** [A]——七种证明图式（外部信念：权威/仪式/非指称符号；经验：归纳/知觉；演绎：变换/公理）。**Selden 2012** ICMI [A]——**单篇最佳综述引用**，其困难清单几乎逐条对应本表 1/2/5/6 行 | (b)「数学语言与记号」这一条**正是本课程最大的可控变量**：sokonanoda 只有 7 个 tactic、无记法，所以「不知如何下笔」可以被形式化地消解——每一步都有唯一的下一步。把 Selden & Selden 的「拆解」训练做成 U10 的主线 |
| **APOS / Dubinsky 与集合论** | —（教材层无对应项） | 规范引用：**Dubinsky & McDonald** [A]（APOS「被 RUMEC 成员有组织地使用」，「**解释学生困难并预测成败**」）+ **Arnon et al. 2014** Springer 专著 [M]。RUMEC 与集合论**相邻**的实证工作 = 量化（Dubinsky 1997；Dubinsky & Yiparaki 2000；Dubinsky/Elterman/Gong 1988）、函数与关系（Breidenbach et al. 1992；Dubinsky & Harel 1992）、无限（Dubinsky et al. 2005）、ISETL 教材传统（Baxter/Dubinsky/Levin 1989）。最接近「专门 APOS 集合论研究」的是 **Hamdan 2006**（等价类、划分、纤维）。近期集合论学习研究是计算中介的（Martinez 2022 [A] / Martinez IV 2024 [A]）。**⚠️ 框架警告：Dubinsky/RUMEC 的实证工作压倒性地是微积分、线性代数、抽象代数与函数，不是集合论。凡「APOS 研究表明学生学集合论有困难」的说法都是过度断言** | 引用 APOS 时**只引到具体的那条实证**（如「EA/AE 混淆 94%」），不要升格为「APOS 支持本课程的设计」。本课程真正的实证依据是障碍 1/2/5/7/9 那五条 |
---

**中文教材这一栏的空白（本报告最大的未决问题）**：负责中文取证（徐明曜/赵春来《集合论》、
耿素云《集合论与图论》、张锦文《公理集合论导引》及中文课程大纲）的子任务在收口时被中止，
**本轮没有取得任何一份中文教材目录或课程大纲**。因此「中文课堂的既有顺序与我们的推荐是否冲突」
目前**无证据**；§3 的教育研究证据也**全部来自英文文献**，中文语境下的困难分布是否有差异同样未知。
建议下一轮单独补这两项。

---

## §4 练习类型学

> 「例子」列凡标【原文】者为**从来源 PDF 直接抄录**的真实题目；其余为据该来源题型设计的对照题。

| 类型 | 例子 | 适合哪个单元 |
|---|---|---|
| **证 / 反证 二选一（成对 stub）** | Macbeth 的练习签名成对出现：`example : P := by sorry` 与 `example : ¬P := by sorry`，题干写「Prove or disprove… If you think it's false, solve the second version」 | 全单元通用（尤其 U1/U2/U5）。**这是本语言的原生形态**：两个 `theorem` 各留一个 `sorry` |
| **找反例** | 【原文】Hammack Example 9.1：猜想「对一切 `n ∈ ℤ`，`f(n) = n² − n + 11` 是素数」，表格里 `n = −3…10` 全是素数，**`n = 11` 时 `f(11) = 11² = 121` 崩掉** | U2（子集）、U5（积）、U7（单射/满射） |
| **列举 / 判定** | 【原文】Hammack §1.3 A 组：「List all the subsets of the following sets.」第 4 题 `∅`、第 5 题 `{∅}`、第 3 题 `{ {ℝ} }` | U2、U3 |
| **判断真假并解释** | 【原文】Hammack §1.3 C 组第 13–16 题：`ℝ³ ⊆ ℝ³` / `ℝ² ⊆ ℝ³` / `{(x,y)∈ℝ² : x−1=0} ⊆ {(x,y)∈ℝ² : x²−x=0}` 及其反向 | U2、U4 |
| **形式 ↔ 非形式互译** | 【原文】Hammack §2.9 Translating English to Symbolic Logic；**Avigad §4.8 要求同一命题同时用 term mode 与 tactic mode 写两遍** | U1、U10 |
| **同一概念两种编码对照** | 【原文】Tao Exercise 3.5.1：**(i)** 证 Kuratowski `(x,y) := {{x},{x,y}}` 满足特征性质；**(ii)** 证短定义 `(x,y) := {x,{x,y}}` 也满足（提示需正则公理）；**(iii)** 证无论用哪种编码 `X × Y` 都是集合 | U5（可改造为「两种函数本体论对照」，见 §3） |
| **填空式证明** | 【原文】Avigad 等的练习体例是逐章 `x.y Exercises` + 「Fill in the `sorry`'s」 | 全单元；**这就是 sokonanoda 的练习形态本身** |
| **给定函数判性质** | 【原文】Hammack §12.2 第 1–14 题（详见 §3「量词顺序」行） | U7 |
| **纠错 / 找茬** | Hammack §5.3 Mathematical Writing、§6.4 Some Words of Advice 把「怎么写错」当独立内容 | U10（读证明） |
| **累积式「证明或推翻」大块** | Hammack **Ch9 整章**跨 Ch1–9 累积出题 | U10 期末项目 |
| **自检题（能被机器判定）** | Macbeth 提供一条 tactic，学生写出命题后「能用当且仅当表述正确」 | 全单元；**与内核判卷机制同构**，是本项目相对纸媒的天然优势 |

---

## §5 推荐 10 单元大纲

**设计公理（每条都有 §2 的来源支撑）**：
① 集合先行，但用 Macbeth 的谓词框架（`Set α := α → Prop`，隶属 = 应用）作为第一句话；
② 子集 → 幂集紧随（Hammack）；③ 有序对用原始 `inductive Prod`，Kuratowski 降为阅读（Tao）；
④ **函数先于关系**（Macbeth/Cummings），复合只教一次（Velleman 红利）；
⑤ **基数先于选择公理**（Tao），并显式点名 Mathlib 相反；
⑥ Russell 放 U9（Hammack 放章末、Tao 标 Optional）。

**依赖**：1→2→3→4→{5,6}→7→8→{9,10}。每单元 **6–12 题 + 2–4 个 worked demo + 解答键**。

| 单元 | 概念 | 必证定理（✓ = 已内核实测） | 必破谬误 | 练习类型配额 |
|---|---|---|---|---|
| **U1 命题与隶属** | `Prop`/`False.rec`/`And`/`Or`；`Set α := α → Prop`；**隶属 = 应用**（`A x`）；相等 `Eq.{1}` | `mem_singleton_self`（待实测）；`empty_has_no_member` ✓（`fun h => h`） | 「`x ∈ A` 是一个集合」（它是 `Prop`）；「`A x` 和 `x ∈ A` 是两种东西」 | 形式↔非形式互译 40% · 判定真假 30% · 填空 30% |
| **U2 子集与空集** | `Set.subset`；自反 / 传递；`∅` 是最小元；外延性公理的必要性 | `subset_refl` ✓ · `subset_trans` ✓ · `empty_subset` ✓ · `not_mem_empty` ✓ | 「证 `∅ ⊆ A` 要先找到 `∅` 的元素」；「`A ⊆ ∅` 对一切 `A` 成立」 | 找反例 30% · 证明 40% · 判定真假 30% |
| **U3 集合运算与幂集** | 并 / 交 / 差 / 补；`Set.power`；「`B ∈ 𝒫(A)` 就是 `B ⊆ A`」 | `subset_union_left` ✓ · `inter_subset_left` ✓ · `union_subset_iff` ✓ · `mem_power` ✓ | 「`A ∈ A`」；「`𝒫(A) ⊆ A`」 | 列举 20% · 证明 50% · 反证 30% |
| **U4 外延性与集合等式** | 用 `Set.ext` 证等式（**公理，非定理**）；De Morgan 律 | `inter_comm`（待实测补全）· `union_empty`（待实测）· 一条 De Morgan（待实测） | 「`A ∪ B = A ∩ B`」；「互相包含与相等是一回事」——**在谓词表示下不是：需要外延性公理** | 证明 50% · 纠错 25% · 找反例 25% |
| **U5 序对与笛卡尔积** | `inductive Prod`；`fst`/`snd` 用 `match`；积的成员 | `fst_mk` ✓（`rfl`）· `snd_mk`（待实测）· `prod_ext`（待实测） | 「`(a,b) = (b,a)`」；「`A × ∅ = ∅` 需要选择」 | 编码对照 25% · 证明 50% · 判定 25% |
| **U6 关系** | `Rel α β := α → β → Prop`；逆与复合；自反/对称/传递；等价关系与划分 | `rel_comp_assoc` ✓ · 等价类「互斥或相同」（待实测） | 「关系复合可交换」；「对称 + 传递 ⇒ 自反」 | 证明 50% · 找反例 30% · 填空 20% |
| **U7 函数、单射、满射** | `f : α → β`；`Fun.comp`；单射 / 满射 / 双射；**量词顺序** | `comp_injective` ✓ · `comp_surjective`（**需先手写 `Eq.symm`**；本轮未通过）· `Injective`/`Surjective` 定义 ✓ | 「`f(n)=2n+1` 从 ℤ 到 ℤ 是满射」；「单射 ⇒ 满射」；「复合保满射不需要选择」（**其实需要——埋给 U10**） | 给定函数判性质 40% · 证明 40% · 找反例 20% |
| **U8 像与原像** | `Set.image` / `Set.preimage`；**前推 vs 回拉** | `image_subset_image` ✓ · `preimage_inter`（待实测）· `preimage_compl`（待实测） | 「`f(A ∩ B) = f(A) ∩ f(B)`」（只有 `⊆`）；「`f(Aᶜ) = f(A)ᶜ`」 | 找反例 35% · 证明 45% · 判定 20% |
| **U9 论域、Russell 与「没有全集」** | `Set α` 为什么依赖 `α`；`A ∈ A` 的后果；把 Russell 当读一读 + 反证 | 对具体类型（如 `Bool`）证「不存在包含一切集合的集合」（**待实测**；一般 `α` 需额外假设） | 「存在包含一切集合的集合」；「`Set α` 就是 ZF 的集合全域」 | 读证明 40% · 反证 30% · 互译 30% |
| **U10 基数与可数性** | 等势（双射）/ 有限 / 可数；Cantor 定理与对角线；**选择公理作为对照** | Cantor 定理「不存在 `α → (α → Prop)` 的满射」（**待实测**，纯 `fun` 应可行）· `ℕ × ℕ` 与 `ℕ` 等势（待实测） | 「一切无限集等势」；「`ℕ` 与 `ℕ × ℕ` 不等势」 | 证明 30% · 找茬 25% · 双射构造 25% · **期末链式项目 20%** |

**每条「必证」为什么必须在白名单内可行**——本轮已实测的关键事实：

- `subset_trans` 只要 `fun x => fun hx => h2 x (h1 x hx)`；`empty_subset` 只要 `False.rec`。
- `rel_comp_assoc` / `image_subset_image` 需要 `Exists.elim` + `Exists.intro` + `And.left/right`，
  **而 `Exists` 必须立成公理三件套**（G-03：Prop 结果 + Type 参数的 inductive 被内核断言拒绝）。
- `comp_surjective` 是本课程**最难的一步**：需要在 `Eq` 上做改写，
  而本语言 prelude 只给 `Eq` / `Eq.refl` / `Eq.subst`，**`Eq.symm` 要学生自己写**（U2 的产物）。
- `match` 只有在结果类型能被内核推断时才可用（实测报 `elab-match-no-expected-type`），
  所以 `fst`/`snd` 这类「返回元素类型」的 `match` 可以，**返回 `Prop` 的 `match` 不行**。

---

## §6 记法引入顺序 + 无记法时的替代写法

### 6.1 硬事实：本语言没有 notation

G-04（无 `notation`/`infix`）在缺口台账里状态为 **open**，实测 `infix:50 " ∈ " => mem` 直接
parse 失败。所以**§6 的「替代写法」不是备选方案，而是唯一方案**；
`∈` / `⊆` / `∅` / `𝒫` / `×` / `∘` / `⁻¹` 这些符号在本课程里**从不出现在画布上**，
它们只出现在**散文与注释里**，作为「你在纸上会看到的写法」。

### 6.2 引入顺序（按学习者第一次需要它的时候）

| 序 | 纸上符号 | 本语言写法（前缀） | 何时引入 | 可否推迟 |
|---|---|---|---|---|
| 1 | 隶属 `x ∈ A` | `A x`（**先给人看 `Set.mem α x A`，再立刻说它就是 `A x`**） | U1 | **不可延** |
| 2 | 相等 `x = y` | `Eq.{1} α x y` | U1（必须先于 `∈` 的解释，否则没法说「同一个元素」） | 不可延 |
| 3 | 子集 `A ⊆ B` | `Set.subset α A B` | U2 | 不可延（U2 之后一切都靠它） |
| 4 | 空集 `∅` | `Set.empty α` | U2 | 不可延（`∅` 是「不可能」的化身） |
| 5 | 并 `A ∪ B` / 交 `A ∩ B` | `Set.union α A B` / `Set.inter α A B`（**并靠 `Or`，交靠 `And`**） | U3 | 可延到 U3（U2 只需要 `⊆`） |
| 6 | 补 `Aᶜ` / 差 `A \ B` | `Set.compl α A` / `fun x => And (A x) (Not (B x))` | U3–U4 | 延到 U4（De Morgan 才真正需要） |
| 7 | 幂集 `𝒫(A)` | `Set.power α A` | U3 | **可以延，但不建议**——实测一次 `fun` 就能证 `mem_power`，性价比极高 |
| 8 | 单元素 `{a}` | `Set.singleton α a` = `fun x => Eq.{1} α x a` | U1–U2（举例用） | 延到 U2 |
| 9 | 无序对 `{a,b}` | `Set.pair α a b` = `fun x => Or (Eq.{1} α x a) (Eq.{1} α x b)` | U3 | 延到 U3 |
| 10 | 笛卡尔积 `A × B` | 类型层：`Prod α β`（`inductive`） | U5 | 延到 U5 |
| 11 | 序对 `(a,b)` | `mk a b`（构造子裸名，见 G-02） | U5 | 延到 U5 |
| 12 | 复合 `g ∘ f` | `Fun.comp α β γ g f` = `fun x => g (f x)` | U7（**关系层 `Rel.comp` 直接复用**） | 延到 U7 |
| 13 | 逆 `f⁻¹` | **不引入为「函数」**：只引入原像 `Set.preimage α β f B` = `fun x => B (f x)` | U8 | 延到 U8 |
| 14 | 等势 `A ≃ B` | **本语言不引入**（无结构体/无存在唯一记号）；改为显式写「存在双射」`Exists (α -> β) (fun f => And (Injective …) (Surjective …))` | U10 | 延到 U10，或**整门课都不引入** |

### 6.3 三条必须写进课程写作规范的硬约束（本轮内核实测）

1. **~~`axiom` 不能带 binder 参数表~~（0.59.0 已修，WO-008 / 台账 G-13）**。修前
   `axiom Foo (α : Type) : Prop` 被解析器拒绝
   （`grade` 报 `{"code":"unexpected-token","message":"expected axiom type, found LParen","stage":"parse"}`），
   只能柯里化。修后 binder 参数表与 `def`/`theorem` 一致（本次调研的
   「最容易绊倒课程作者的一条」已消失）。
   *（修前记录在 `docs/design/decl-binders.md` 的边界一节；该处已同步成 as-built。）*
2. **等式必须显式给宇宙**。裸写 `Eq` 默认 `u = 0`，于是
   `def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => Eq x a`
   被内核拒绝：`类型不匹配：期望 Sort(0)，实际是 $2`。
   正确写法是 `Eq.{1} α x a`。同理 `Eq.subst` / `Eq.refl` 也要显式给宇宙
   （`Eq.subst.{1} β p a b h`）。**后果**：`∈` 的天然定义 `A x` 不带等式，
   但**单元素、配对、外延性一律要写 `. {1}`**，这会在 U2 集中爆发，必须提前做「陷阱卡」。
3. **`And` 要走 axiom 族，不能走 inductive**。`inductive And` 的构造子是**裸名 `intro`**，
   拿不到 `And.intro`（G-02：构造子无命名空间且全局唯一）；实测 `And.left` / `And.intro`
   报 `unknown identifier`。改用 axiom 三件套后，`repro/lib.sokonanoda` **29/29 全部通过**。
   `Or` 相反：用 `inductive` 得到裸名 `inl`/`inr`，恰好可用。
4. **`Exists` 必须立公理**（G-03：Prop 结果 + Type 参数 + 单构造子 + 自有字段的 inductive
   被内核断言拒绝），三件套 `Exists` / `Exists.intro` / `Exists.elim` 实测可用。

### 6.4 关于 G-10 的复现警告

本报告取证期间**复现了 G-10**（台账状态 open）：对上文那条 `axiom Foo (α : Type) : Prop`，
MCP 工具 `mcp__sokonanoda__check` 返回 `{"decl_checked":0, "failed":[], "ok":true}`——
**全零 + 无失败 + ok**，而同一文本用 `scripts/soko grade` 判卷会正确报出 parse 诊断。
**给课程生产的直接建议**：凡「`decl_checked` 突然变成 0」的文件，
一律再用 `grade` 复核一次，不要相信 `check` 的 `ok`。
复现文件：`repro/P1-axiom-params.sokonanoda` 与对照 `repro/P1b-axiom-curried.sokonanoda`。

---

## §7 参考来源清单

**已核实的教材目录 / 正文来源**

- Halmos, _Naive Set Theory_ —— 官方 25 章目录（含页码）：https://link.springer.com/book/10.1007/978-1-4757-1645-0
- Halmos 书目信息（Wikipedia）：https://en.wikipedia.org/wiki/Naive_Set_Theory_(book)
- Enderton, _Elements of Set Theory_ —— Elsevier 官方目录（Ch1–Ch9 节级）：https://shop.elsevier.com/books/elements-of-set-theory/enderton/978-0-12-238440-0
- Tao, _Analysis I_ —— §3.1–3.6 节名、Def 3.3.1、Remark 3.3.2、Exercise 3.5.1/3.5.2、§3.5.12、Exercise 3.6.8、§8.4 均取自该扫描本（OCR，仅用于核对目录与题干）：https://archive.org/details/terrence-tao-analysis-i
- Velleman, _How to Prove It_ 3e —— CUP 官方章节数据（章级 + 页码范围）：https://www.cambridge.org/core/product/identifier/9781108539890/type/book
- Velleman 2e —— 全文 PDF（METU 课程镜像；Def 5.1.1、Def 4.1.1、Ch7 目录取自此处）：https://users.metu.edu.tr/serge/courses/111-2011/textbook-math111.pdf
- Hammack, _Book of Proof_ 3e —— 作者托管全文 PDF（TOC pp.4–6、Exercise 1.3、Exercise 9.1、Exercise 12.2 原文）：https://richardhammack.github.io/BookOfProof/Main.pdf
- Hammack 书页（入口）：https://richardhammack.github.io/BookOfProof/
- Macbeth, _The Mechanics of Proof_ —— 完整目录（Ch1–Ch10）：https://hrmacbeth.github.io/math2001/
- Avigad/Lewis/van Doorn，Lean 4 改编 Joseph Hua, _Logic and Proof_：https://leanprover-community.github.io/logic_and_proof/
- Solow, _How to Read and Do Proofs_ 6e —— Wiley 书目页：https://www.wiley.com/en-us/How+to+Read+and+Do+Proofs%3A+An+Introduction+to+Mathematical+Thought+Processes%2C+6th+Edition-p-9781118164020
- Cummings, _Proofs: A Long-Form Mathematics Textbook_ —— 作者页（免费目录 PDF / 样本 / 提示 / 解答）：https://longformmath.com/proofs-book/
- Cummings 目录 PDF：https://longformmath.com/wp-content/uploads/2025/01/proofs-toc.pdf
- Ordered pair（Kuratowski / Wiener / Hausdorff 定义与「编码常被降级为练习」的总结）：https://en.wikipedia.org/wiki/Ordered_pair
- Axiom of choice（背景）：https://en.wikipedia.org/wiki/Axiom_of_choice

**未核实（本轮未取得证据，**不要**当作已确认）**

- **Jech, _Set Theory_**（3rd millennium ed.）：Springer 反爬，3 次尝试均返回 `Client Challenge`；Open Library 有 3 条记录（`OL2668795W` / `OL9076052W` / `OL17417833W`）但无目录字段。**目录未核实。**
- **Kunen, _Set Theory_**：未取得任何目录。**未核实。**
- **全部中文教材与中文课程大纲**：徐明曜/赵春来《集合论》、耿素云《集合论与图论》、张锦文《公理集合论导引》，以及中国大学 MOOC / 各校教学大纲——**均未核实**（取证子任务被中止）。
- **§3 的教育研究文献已于收口后补齐**（见下方「教育研究来源」）。原报告曾把整节标为未核实；
  现已由专门取证补齐约 190 条来源，逐条带核验级别（[F] 全文 / [A] 逐字摘要 / [M] 仅元数据）。
  **仍有两处是实测的否定结果，不是检索失败**：有序对编码（障碍 3）与选择公理（障碍 8）
  **不存在实证研究**。**两条不得过度断言**：Sfard 1991 仅取到元数据（其结论未核实）；
  Tall 的「三个世界」（embodied/proceptual/formal）措辞未从任何来源核实，**不要引用该措辞**。
- **Velleman / Solow / Cummings 的练习原文**：这三本没有合法的免费预览，**未引用任何题目原文**（Hammack 与 Tao 的题目因开放获取 / 可核对而引用）。
- **Velleman 3e 的节级标题**：章级与页码范围来自 CUP 官方数据（可靠），节级来自一份来源非官方的全文扫描（与官方章级一致）。
- **`mem_singleton_self`、`prod_ext`、`preimage_inter`、`preimage_compl`、Cantor 定理、`ℕ × ℕ ≃ ℕ`**：§5 中标注「待实测」的必证定理，**尚未在内核上判过**。
- **comp_surjective**：本轮尝试未通过（需要学生先在 U2 手写 `Eq.symm`，本语言 prelude 不含它）；**判为设计依赖而非不可行**，但需下一轮实测确认。

**教育研究来源（§3，详见 `learning-difficulties.md`）**

- Bagni 2006, _ESM_ 62(3)（属于 vs 包含）：https://doi.org/10.1007/s10649-006-8545-3
- Hendriyanto et al. 2024, _J. Math. Educ._ 15(2):517–544（`{r}∈M` 无人正确；`∅` 不被接受为集合）：https://files.eric.ed.gov/fulltext/EJ1428069.pdf
- Shaker & Berger 2016（集合论证明中的定义误读）：https://eric.ed.gov/?id=EJ1147782 · Zazkis & Gunn 1997（元素/子集/空集，ISETL）[仅元数据]：https://eric.ed.gov/?id=EJ543540
- Lee, Park & Kim 2024, _IJSME_（假前件）：https://eric.ed.gov/?id=EJ1410917
- Durand-Guerrier 2003, _ESM_ 53(1):5–34（开语句上的蕴涵）：https://doi.org/10.1023/A:1024661004375
- Dubinsky 1997（差题五分之三出在蕴涵）：https://www.math.kent.edu/~edd/LearningQuant.pdf · https://eric.ed.gov/?id=EJ567950
- Mirin, Weber & Wasserman 2020, PME-NA（两种不等价的「函数」定义同时在使用）：https://files.eric.ed.gov/fulltext/ED629969.pdf
- Kanamori 2003, _Bull. Symbolic Logic_（内涵→外延观点）：https://doi.org/10.2178/bsl/1058448674
- Breidenbach, Dubinsky, Hawks & Nichols 1992, _ESM_ 23(3):247–285（函数本体论被测量：约 40%）：https://doi.org/10.1007/BF02309532 · https://www.math.kent.edu/~edd/PROCESSFUNC.pdf
- Sfard 1991, _ESM_ 22(1):1–36 [**仅元数据，勿归因结论**]：https://doi.org/10.1007/BF00302715
- Vinner & Dreyfus 1989：https://doi.org/10.5951/jresematheduc.20.4.0356 · Tall & Vinner 1981：https://doi.org/10.1007/BF00305619 · Tall & Bakar 1992：https://doi.org/10.1080/0020739920230105
- Dubinsky & Yiparaki 2000（**未发表手稿**；EA/AE 混淆 94%）：https://www.math.kent.edu/~edd/OlgaPaper.pdf
- Selden & Selden 1995, _ESM_ 29(2):123–151（拆解成功率 8.5%/5%）：https://doi.org/10.1007/BF01274210
- Shipman 2015, _TMA_（教材用真值表教蕴涵、丢掉隐藏 `∀x`）：https://doi.org/10.1093/teamat/hrv007
- Piatek-Jimenez 2010, _MERJ_ 22(3):41–56（量词）[**更正：不存在其 2004 单射/满射论文**]：https://doi.org/10.1007/BF03219777
- Bansilal, Brijlall & Trigueros 2017, _JMB_ 48:22–37（单射/满射的 APOS 研究）[**付费墙，第 1 号待补**]：https://doi.org/10.1016/j.jmathb.2017.08.002
- Hamdan 2006, _ESM_ 62(2):127–147（APOS 遗传分解，中心对象是原像/纤维）：https://eric.ed.gov/?id=EJ748150 · https://doi.org/10.1007/s10649-006-5798-9
- Hamza & O'Shea 2011, MEI 4:192–202（可数≡有限；双射判据用得前后不一）：https://mural.maynoothuniversity.ie/6977/
- Zazkis & Mamolo 2009, _FLM_ 29(3):53–56（「Sean vs. Cantor」）：https://flm-journal.org/Articles/492E35FADC6DE2DD1D825A1FEEB71.pdf
- Monaghan 1986 华威博士论文（整体—部分直觉）：http://wrap.warwick.ac.uk/34626/1/WRAP_THESIS_Monaghan_1986.pdf
- Dubinsky, Weller, McDonald & Brown 2005, _ESM_ Parts 1&2：https://doi.org/10.1007/s10649-005-2531-z · https://doi.org/10.1007/s10649-005-0473-0
- Bell, _SEP_ "The Axiom of Choice"：https://plato.stanford.edu/entries/axiom-choice/ · Incatasciato & Sánchez Terraf 2024（**Lean 中验证过的 Zorn 引理**）：https://arxiv.org/abs/2404.11638 · Dawkins 2018（学生认为「公理」是什么）：https://eric.ed.gov/?id=EJ1188492
- Moore 1994, _ESM_ 27(3):249–266（三大困难源）：https://doi.org/10.1007/BF01273731
- Selden & Selden 2003, _JRME_ 34(1):4–36：https://doi.org/10.2307/30034698 · Selden 2012 ICMI（**单篇最佳综述**）：https://doi.org/10.1007/978-94-007-2129-6_17
- Weber 2001, _ESM_ 48(1):101–119（有句法知识却用不上）：https://doi.org/10.1023/A:1015535614355
- Inglis & Alcock 2012, _JRME_ 43(4):358–390（眼动）：https://doi.org/10.5951/jresematheduc.43.4.0358
- Harel & Sowder 1998（七种证明图式）：https://doi.org/10.1090/cbmath/007/07
- Tall 2013, CUP [**书已核实，「三个世界」措辞未核实**]：https://doi.org/10.1017/cbo9781139565202 · Styliandes & Stylianides 2009（认知冲突教学设计）：https://doi.org/10.5951/jresematheduc.40.3.0314
- Dubinsky & McDonald（APOS 规范引用）：https://doi.org/10.1007/0-306-47231-7_25 · https://www.math.kent.edu/~edd/ICMIPaper.pdf
- Arnon et al. 2014, _APOS Theory_, Springer：https://doi.org/10.1007/978-1-4614-7966-6
- Martinez IV 2024（计算中介的集合论学习）：https://eric.ed.gov/?id=EJ1417658
- **完整取证报告（2874 行、~190 条来源、含否定结果与待补清单）**：`docs/notes/settheory-survey/learning-difficulties.md`

**本轮自建的可复跑探针（工作区内）**

- `docs/notes/settheory-survey/repro/lib.sokonanoda` —— 集合论共享库 + U2/U3/U4 核心定理，**29 checked / 0 failed**。
- `docs/notes/settheory-survey/repro/lib2.sokonanoda` —— U6/U7/U8 核心定理，**10 checked / 1 failed**（`comp_surjective`）。
- `docs/notes/settheory-survey/repro/P1-axiom-params.sokonanoda` / `P1b-axiom-curried.sokonanoda` —— `axiom` binder 与 G-10 复现对照。
- `docs/notes/settheory-survey/proof-book-tocs.md` —— 六本证明教材的逐条取证报告（约 1190 行，含 5 处任务书前提错误的更正与逐条 URL）。
- `docs/notes/settheory-survey/lean4-sets-functions-prior-art.md` —— 同工作区另一份 Lean 侧（MIL / TPIL / MoP / L&P）取证报告，**本轮未独立复核其 URL**。
