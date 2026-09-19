# 设计：卷 I《集合论》大纲（2026-09-18，v2）

> 触发（用户）：「也需要调研更多集合论的教程。」
> 上游计划：`docs/design/teaching-project.md`（第二大课总体计划）。
> 输入（全部落盘）：
> - `docs/notes/settheory-survey/set-theory-teaching-survey.zh.md`（课堂教学法：教材顺序之争、
>   六组"先后"证据、推荐十单元；**教育研究文献未取证**）；
> - `docs/notes/settheory-survey/prior-art-report.md`（证明助手先例：MIL/TPIL/MoP/L&P/FM/LPA、
>   **`djvelleman/stg4` 集合论游戏**、analysis §3 逐节解剖）；
> - `docs/notes/settheory-survey/proof-book-tocs.md`（教材目录逐条取证 + 五处前提勘误）；
> - `docs/notes/settheory-survey/repro/lib.sokonanoda`（**29 checked / 0 failed** 的集合库实测）
>   与 `lib2.sokonanoda`（10 checked / 1 failed，唯一失败点是需要手写 `Eq.symm` 的
>   `comp_surjective`）；
> - 角色模型：`analysis` §3（6,984 行 / 319 thm / 272 处上游 `sorry` = 留给读者的习题）。
>
> 状态：**设计（待拍板 D-S1…D-S5）**。落地按 `teaching-project.md` §8 的 P2→P4。

---

## 0. 三条被调研"翻案"的结论（先看这个）

1. **集合论游戏存在，而且作者是 Velleman 本人**：`djvelleman/stg4`（8 worlds / 51 关，
   lean4game）。**"没有集合论游戏"是错的**——我们要抄的是它的三个机制（§1.3），
   不是从零发明关卡。
2. **Mathlib 的 `Set` 与我们的逐字相同**：`def Set (α : Type u) := α → Prop`
   （`Mathlib/Data/Set/Defs.lean:51`）。**语义零差距，差的只有记法**（G-04）——
   这条把"我们能不能教集合论"从"能不能"变成"记法好不好看"。
3. **最贴近我们路线的公开材料存在**：Université Gustave Eiffel 的
   `niotie/logique-preuve-assistee`（Lean 4、**无 Mathlib**、从零 `def Set (α) := α → Prop`、
   `5InjectivitySurjectivity.lean` 约 30 道全 `sorry`）。**"无 Mathlib 纯 Lean 4 集合课"
   有先例，不是我们的臆想。**

4. **"集合论教学层"在证明助手里是明确空白**（第二轮取证）：Isabelle/ZF、Metamath `set.mm`、
   Mizar MML、mathlib3 `set_theory`、Cubical Agda 的 `CumulativeHierarchy` 这些**把 ZF 做深的
   系统全部只有库、没有教学层**；有教学层的只有两类——"证明助手通用教程"（Metamath 书 /
   Mizar 手册 / ACL2 教程）与"类型化集合论的教程章"（HOL Light 第 14 章、Isabelle/HOL
   `sets.tex`、HOL4 手册）。**"从零开始的集合论教学层"没人做**——这正是本项目的生态位。

---

## 1. 调研综合

### 1.1 教材的顺序之争（取证结论）

| 争点 | 各方做法（已取证） | 卷 I 的取舍 |
|---|---|---|
| 集合 vs 逻辑谁先 | Hammack **Ch1 集合**（在逻辑前）；Macbeth Ch9（在函数后）；Avigad Ch11 | 入门课已教逻辑 ⇒ 卷 I 直接上集合 |
| 有序对放哪 | Hammack §1.2（子集之前）；Tao §3.5（最后）；Velleman §4.1（关系之前）；Wikipedia：多数书把 Kuratowski 编码留作习题 | **序对放"集合运算"之后、"关系"之前**（= Velleman 位置，也兼容 Tao） |
| 幂集放哪 | Hammack 1.3 子集 → **1.4 幂集紧随**；Avigad 丢到 §11.4 最后；Halmos 与补集并列 | **紧随子集**（实测 `mem_power` 一个 `fun` 就过） |
| 关系 vs 函数 | **关系先** = Velleman 4→5 / Hammack 11→12 / Avigad 13→15；**函数先** = Macbeth 8→9 / Cummings 8→9（3:2） | **关系先**（靶子是 Tao §3.3；且"函数类型是语言原语、函数作为关系才是要教的对照"） |
| 复合教几次 | Velleman：**关系层先教（Def 4.2.3），函数层复用（Thm 5.1.5）** | 采纳：复合只教一次，函数章复用 |
| 基数 vs 选择 | **六本证明教材里五本完全没有选择公理**；有 AC 的都是"选择先"；**唯一"基数先"的是 Tao（§3.6 → §8.4）**；而 Mathlib 的基数建立在 `Classical.choice` 上 | **跟 Tao：先基数、后选择**；在单元 10 末尾点出"Mathlib 的基数建立在选择上"当作**本课程独有的增量** |
| Russell 放哪 | Hammack 1.10（第一章末）；Tao §3.2（标 Optional）；**Halmos 完全没有**；Enderton 在 Ch1 | 放**单元 11**（收尾对照），不放开头 |
| Schröder–Bernstein | MIL 4.3 有；Hammack Ch14 有 | **加进单元 9**（D 类题的最佳素材，且不需要选择） |

### 1.2 证明助手先例（可偷什么）

| 材料 | 结构 | 可偷的装置 |
|---|---|---|
| **analysis §3**（我们的角色模型） | 3.1 Fundamentals(1234 行/140 声明) → 3.2 Russell(203) → 3.3 Functions(810) → 3.4 Images(845) → 3.5 Products(1328) → 3.6 Cardinality(2463) → epilogue ZFSet(101) | **`image_of_inter' : Decidable (∀ …, image f (A∩B) = …)`**：把"找反例"做成可判定定义（我们无 `Decidable` ⇒ 写成 `theorem …_counter : ¬ (∀ …)`）；**epilogue 接回真库**（`card_eq_nat_card`），我们变形为"总结课对照 `Set.mem ↔ ∈`"；**`Set.power` 用子集谓词**（`mem_power := rfl`） |
| **MIL 第 4 章** | Sets → Functions → Schröder–Bernstein 一章打通 | 密度参照；题库来源（`S01_Sets`/`S02_Functions` + `solutions/`） |
| **MoP（Macbeth）** | 8 Functions → 9 Sets（3 节） | **成对"证明或证伪"**（`4 ∈ {a｜a<3}` / `4 ∉ {a｜a<3}`）→ 我们的 **D 类练习** |
| **LPA（Gustave Eiffel）** | `TP3EnsemblesFonctions/1..5` | 零依赖路线的可行性证据；`5InjectivitySurjectivity.lean` ≈30 题含反例训练 |
| **`djvelleman/stg4`（游戏）** | 8 worlds / 51 关 | 三件：**逐步解锁语法**（关卡头 `NewTactic`/`DefinitionDoc`，第 N 关只许已解锁武器）；**三层 Hint**（`Hint` / `hidden := true` / `strict := true`，且提示插值当前假设名）；**`Branch` 死路**——把常见错的选择做成可探索分支而非报错。难度曲线：`Combo` world 明说"For the most part, we'll leave you on our own" ⇒ **提示密度随进度递减** |
| **Logic and Proof** | 11–12 Sets（散文 + Lean 各一章） | "同一内容两章" = 我们的 X 类；Kuratowski 配对定理；布尔代数恒等式表证明集合等式 |

### 1.3 学习障碍（**实证版**，2026-09-18 第三轮）

> 来源：`docs/notes/settheory-survey/learning-difficulties.md`（2874 行 / ~190 条来源，
> 每条带核验级别 `[F]` 全文 / `[A]` 逐字摘要 / `[M]` 仅元数据）。
> **每条教学动作都指到证据**；没有证据的两项（有序对、选择公理）明确标为"无实证"。

| 障碍 | 实证（带出处） | 教学动作（落到单元） |
|---|---|---|
| **单元素集 `{a}` vs 元素 `a`** | Hendriyanto et al. 2024（183 人）：`r ∈ M` / `s ∉ M` 几乎全对，但 **`{r} ∈ M` 无一人给出正确理由** | **单元 2 新增 D 类题**：`{a} ∈ A` 与 `a ∈ A` 的关系（试做稿已加） |
| **`∅` 与空真** | 同一研究：许多学生**不接受 ∅ 是集合**，理由全都一样——"里面没有元素可指认"；Dubinsky 1997：七道做得差的题里**五道涉及不属于量化部分的蕴涵** | 单元 2 显式给"∅ 是一个集合"的定位 + `∅ ⊆ A` 的"唯一反例在哪"追问 |
| **函数本体论** | Breidenbach et al. 1992 **量化过**：本该 ~100% 判为函数的场合只有 **约 40%**（图 19–40.7%、方程 25–31%） | **单元 7 的导入语直接讲这个数字**——它就是本课程存在的理由 |
| **量词顺序 `∀∃` vs `∃∀`** | Dubinsky & Yiparaki 2000：**94% 的学生至少把一个 EA 陈述读成 AE**（反向仅 5%）⚠️ 未发表手稿，引用须注明状态 | 单元 7 的 D 类题（"四个命题只有两个为真"）现在有实证支撑 |
| **可数 / 对角线** | Hamza & O'Shea 2011：**援引双射判据的人没有一个在所有题上都用它，且极少真的写出映射**；Zazkis & Mamolo 2009：学过 Cantor 定理的学生面对假的 ℝ 枚举只说"Cool!" | **单元 9/10 强制"亲手写出那个映射"**；讲 Cantor 前先给一个假枚举让他找矛盾 |
| **过渡到证明（总论）** | Moore 1994 三大困难源：**(a) 概念理解 (b) 数学语言与记号 (c) 不知如何下笔**；Selden & Selden 1995 拆解成功率 8.5% / 5%；Weber 2001"有句法知识却用不上" | **(b)(c) 正是本课程最大的可控变量**：7 个 tactic + 无记法 ⇒ 每步几乎只有一个下一步；hint 阶梯对准"如何下笔" |
| 像 vs 原像 | 教材证据 + analysis 把 `image_of_inter'` 做成 Decidable | 单元 8 的 D 类题（`f '' (A∩B) = f '' A ∩ f '' B`） |
| 有序对的集合编码 | **无实证研究**（ERIC `"Kuratowski ordered pair"` = 0 条，实测否定结果） | 单元 5 降为阅读题——**不要写成"研究表明…"** |
| 选择公理 | **无实证研究**（`"axiom of choice"` + teaching = 0 条） | 单元 10 用 Förster 2006 的观察当导语："教学惯于略过 AC 的应用……学生因此没有形成它的心智图像，日后也认不出何时被使用"（**这是作者观点，不是实证**） |

### 1.4 引用纪律（写课程文档时必须遵守）

1. **有序对（障碍 3）与选择公理（障碍 8）不存在实证研究**——这是**实测的否定结果**，
   不是"没查到"。任何交付物不得写"研究表明学生对 AC 持 X 看法"。
2. **不得过度断言**：Sfard 1991 **只有元数据**，不能把结论归给它；Tall 的"三个世界"
   措辞未核实；**APOS/Dubinsky 的实证基础压倒性地是微积分/线代/抽代**——引 APOS 时
   只引到具体那条实证（如 94% 那条），不要写"APOS 表明集合论难学"。
3. **两条流行线索是幻觉**：不存在 Piatek-Jimenez 2004 的单射/满射论文；
   不存在 Cusi & Malara 2007 的量词论文。
4. **中文语境空白**：所有 §3 实证都是英文文献，中文教材/大纲一份未取到 ⇒
   "中文课堂的困难分布是否不同"**未知**（建议下一轮单独补）。
5. 课程文档里的每句教学主张，要么给出处，要么显式写"**设计判断**"。

---|---|---|
| `∈` vs `⊆` 混淆 | 教材证据（Hammack Ch1 练习要求区分元素/子集） | 单元 1 同时给两种写法 + X 类互译 |
| 空集是任意集合的子集 | 教材证据（`not_mem_empty` 类练习普遍） | 单元 2 第一题；追问"唯一反例在哪" |
| 元素 vs 单元素集 | 教材证据 | 单元 2 用 `Set.singleton` + D 类题 |
| 有序对的集合编码 | 教材证据（Tao Exercise 3.5.1 就是它） | 单元 5 把 Kuratowski **降为阅读题**（我们以 `inductive Prod` 为原语） |
| "函数 = 图的集合" | 教材证据（Hammack Def 12.1 明确定义为关系） | 单元 7 显式对照两条路 |
| 单射/满射量词顺序 | 教材证据（LPA 的"四个命题只有两个为真"） | 单元 7 的 D 类题 |
| 像 vs 原像 | 教材证据（analysis 把 `image_of_inter'` 做成 Decidable） | 单元 8 的 D 类题：`f '' (A∩B) = f '' A ∩ f '' B` |
| 可数与对角线 | 教材证据（Hammack Ch14、MIL 4.3 都有 Cantor） | 单元 10 先显式枚举、再对角线 |

---

## 2. 课程写作红线（**全部实测**，写第一个单元前必读）

1. ~~**`axiom` 不能带 binder 参数表**（台账 **G-13**）~~ —— **0.59.0 已修（WO-008）**：
   `axiom Foo (α : Type) : Prop` 现在与柯里化 `axiom Foo : (α : Type) -> Prop` 等价，
   `def`/`theorem`/`axiom` 三种声明形式的 binder 语法已一致。旧红线只在核对
   0.58.0 及更早的行为时有效。**注意内核口径**：axiom 的类型仍必须是 `Sort`，
   带 binder 的 `… -> Prop` 是 Pi 类型、会被 kernel-rejected（箭头写法同款），
   所以写公理时 codomain 要落在 `Sort n`（例如 `Sort 1`）。
2. **等式必须显式给宇宙**：裸 `Eq` 默认 `u = 0`，`Eq x a`（`α : Type`）会被拒
   （`期望 Sort(0)，实际是 $2`）；正确写法 `Eq.{1} α x a`，`Eq.subst.{1}` 同理。
   **单元素/配对/外延性全会撞上，建议做成"陷阱卡"放在单元 2。**
3. **`And` 必须走 axiom 族，不能走 `inductive`**（G-02）：`inductive And` 的构造子是裸名
   `intro`，`And.intro`/`And.left` 报 unknown identifier。`Or` 相反：用 `inductive` 得裸名
   `inl`/`inr` 恰好可用。（入门课 already 这么写，照抄即可。）
4. **`Exists` 必须立公理三件套**（G-03），与入门课单元⑧一致。
5. **凡 `decl_checked` 突降为 0，一律用 `grade` 复核**（G-10）：MCP `check` 与
   `query check` 在解析失败时给 `ok:true` + 全零。**as-built（0.59.0，G-10 已修）**：
   `query check` 现在同样带 parse 诊断 + exit 1（与 `grade` 同口径），这条复核纪律
   保留自无妨，但不再是唯一可信通道。

**取自 stg4 的三条写作纪律**（非语言约束，是教学约束）：

- **每单元开头声明"本单元只允许什么"**（对应 stg4 的逐步解锁）：例如单元 2 开头写
  "本单元只用 `intro`/`exact`/`apply`/`rfl` 与 `Set.subset` 的展开"。
- **提示分段但仍不泄答案**（我们已有 `-- soko:hint` 三段）：把"必读提醒"写进第一段，
  把"触发条件 + 引理名"写进第三段；**常见错路**改写成一道 R/D 题而不是提示里的警告。
- **提示密度随单元递减**：前 3 个单元每题都有 hint，中段只给"关键件"，后段（9–12）
  只给"思路"。

---

## 3. 锁定的大纲（**12 单元**）

> **进度（2026-09-18）**：单元 1–2 已"真做一遍"并判卷通过（`docs/gaps/spike/README.md`：
> 16 道题 + 66 条标准库，全部 0 failed），顺带产出标准库欠账 L-01…L-05 与语言缺口
> G-14/G-15 —— 分层判据见 `docs/design/course-stdlib.md`。

> 相对 `teaching-project.md` §4.2 的十单元草案：**拆细前半段**（子集/空集 与 运算/幂集
> 分成两个单元，另立"外延性与集合等式"）、**基数合成两个单元**（§3.6 是 analysis 里最大的
> 一节：2463 行）、Russell 与综合各占一个单元。总计 12 单元。

练习类型：**T** 项填空 · **B** `by` 填空 · **L** 自证引理 · **R** 读/评译 ·
**X** 形式↔散文互译 · **D** 证明或证伪（MoP 式，成对给正反两题）。
每单元 6–12 题、≥3 种类型、**必含 D 或 R**。

| # | 单元 | 靶子 | 概念顺序 | 必证 | 必破（D 类） | 配额 |
|---|---|---|---|---|---|---|
| 1 | 集合与隶属 | §3.1 前 | 谓词视角 `Set α := α → Prop` → 隶属=应用 → 外延（`Set.ext` 先作公理） | `Set.subset_def := rfl`（分析项目同款）、`A ⊆ A` | "`A = B` 就是 `∀x, A x ↔ B x`"（其实是公理，不是定义） | T4 X2 R1 |
| 2 | 子集、空集、包含三律 | §3.1 中 | `⊆` 自反/传递/反对称 → **空集是一个集合**（显式动作，Hendriyanto 2024）→ 元素 vs 单元素 | `∅ ⊆ A`、`A ⊆ ∅ → A = ∅`、反对称用外延 | **`{a} ∈ A` 与 `a ∈ A`**（实证：单元素是最脆弱点，`{r} ∈ M` 无人答对） | T4 L2 D2 |
| 3 | 并、交、差与幂集 | §3.1 后 | 三个二元运算（`Or`/`And`/`Not` 定义）→ 幂集 → `Set.mem_powerset_iff` | `A ⊆ A ∪ B`、`A ∩ B ⊆ A`、`A ⊆ B → A ⊆ C → A ⊆ B ∩ C` | "`(A ∪ B) ⊆ (A ∩ B)`"（假）、"`𝒫 A ⊆ A`"（假）⚠️ **原稿写「`A ∈ 𝒫A` 是假的」——实测它是真的**（`Set.Equiv`/`subset_refl` 一行），已勘误 | T6 L2 D2 |
| 4 | 外延性与集合等式 | §3.1（Avigad 11.2 的恒等式表） | 外延证明套路 → 布尔代数恒等式表 → 形式↔散文 | `A ∩ (B ∪ C) = (A ∩ B) ∪ (A ∩ C)`、`(A \ B) \ C = A \ (B ∪ C)` | "`(A ∪ B) \ B = A`"（假）、"`A \ B = A ∩ Bᶜ` 反过来也…" | X4 T2 R2 |
| 5 | 序对与笛卡尔积 | §3.1 尾 + §3.5 | `inductive Prod` → 配对定理 → `A × B` → 函数图（Kuratowski 作阅读题；**无实证研究，别写成"研究表明"**） | `(a,b) = (a',b') → a = a' ∧ b = b'`、`A × B` 的成员刻画 | "`A × B = B × A`"（假，除退化） | T3 L2 D1 R1 |
| 6 | 关系 | §3.3 前 | 关系 = `A -> B -> Prop` → 逆/复合 → 等价关系三律 → 等价类 → 划分 | 复合结合律、等价类"互斥或相同"、划分↔等价关系 | "自反+对称 ⇒ 传递"（假） | T4 L2 D2 |
| 7 | 函数 | §3.3 | **导入语讲 Breidenbach 1992 的 40%** → 函数即单值关系 **vs** 语言原语 `A -> B` → 复合（复用单元 6）→ 单射/满射/双射 → 逆（**数据版**：逆函数与互逆证据作参数） | 复合结合、双射↔有逆（数据版）、左逆⇒单射、右逆⇒满射 | 量词顺序交换的四个变体（实证：94% 把 EA 读成 AE） | T5 L2 D2 |
| 7′ | （边界） | — | **「满射 ⇒ 存在右逆函数」在本语言里证不出来**（`Exists.elim` 的 Q 只能是 Prop，取不出函数值 = 需要选择公理）⇒ 课程统一用数据版陈述（台账 **L-06**） | — | — | — |
| 8 | 像与原像 | §3.4 | `Set.image`/`Set.preimage` → 前推/回拉 → 纤维 | `preimage` 保并交补（全保）；`image` 只保并 | **`f '' (A∩B) = f '' A ∩ f '' B`**（假）、`f '' (f ⁻¹' C) = C`（假） | T3 L2 D3 |
| 9 | 等势（基数 Ⅰ） | §3.6 前 | 等势用**数据**定义（一对互逆映射 + 域条件，**必须亲手写出映射**，Hamza & O'Shea 2011）→ 等势是等价关系 → 从单射+右逆构造等势 | refl/symm/trans、单射+右逆⇒等势 | "真子集一定严格更小"（无限时假） | T2 L3 D2 |
| 9′ | （边界） | — | **Schröder–Bernstein 不做**（需不动点构造）；**有限集/鸽笼不做**（需 Fin/递归装置）；等势只能是 **Prop 值**（语言无累积性，`Set.Equiv … : Prop`，见 **L-06**） | — | — | — |
| 10 | 基数 Ⅱ：可数与 Cantor | §3.6 后 | 可数 → **先给一个假的 ℝ 枚举让他找矛盾**（Zazkis & Mamolo 2009）→ Cantor → 对角线 → 选择公理（导语用 Förster 2006，标注为作者观点） | Cantor 定理、**`ℕ × ℕ` 可数（亲手写映射）** | "`𝒫ℕ` 可数"（假）、"可数并仍可数"（需要选择，讲清在哪用了） | T2 L3 D2 |
| 11 | 论域与 Russell | §3.2 + epilogue | 概括公理 → Russell 集 → 为什么 `Set α` 依赖 `α` → 与 ZF 的关系（对照 `ZFSet`） | "不存在全集"的可证版本 | "存在包含一切的集合"（假） | R3 X2 |
| 12 | 综合与读证明 | §3 全书 | 形式↔散文互译 → 给错证明找错 → 期末小项目（集合→关系→函数的链条；**基数一环待补**——现版没串到单元⑨⑩） | 小项目 3–5 条串联定理 | 两份"看起来对"的错证明 | X4 R3 P1 |

依赖树：1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → {9, 10} → 12；**11 可在 4 之后随时插**。

> **实测偏差（2026-09-18，12 单元全部落地后回填；数字来自 `tools/check.py`）**：
> ① 单元③ 的 D 类原稿有数学错误（`A ∈ 𝒫 A` 是**真**的），已勘误为上表的 `𝒫 A ⊆ A`；
> ② 单元④ 实际 8 题（T4 D2 L2，配额内）、单元③ 实际 T6 L2 D2（比原稿多两道 T）；
> ③ 单元⑦ 撞到选择公理边界（L-06），`bijective_iff_inverse` 改为数据版；
> ④ 单元⑨ 的等势只能用 Prop 值定义（无累积性），S–B 与有限集按边界表砍掉；
> ⑤ 单元⑫ 的小项目链条目前到函数层，**基数一环未串**（补它需要新增练习并重排配额）；
> ⑥ 12 单元实测合计 **93 道练习 / 308 checked / 0 判负**（含 lib 与解答）。

**今天就能写的**（不依赖任何 blocker）：1、4（部分）、6、7、11。
**要等 WO 的**：3（G-04 记法）、5（G-02 构造子名）、8（G-01 签名）、9/10（G-03 `Exists`、
G-01）、12（G-01）。

---

## 4. 记法引入顺序与"无记法"替代（G-04）

> **Mathlib 记法的逐字取证（2026-09-18 第二轮）**：见 `docs/notes/settheory-survey/prior-art-report.md` §1.3。
> 三条会改我们设计的事实：
> 1. **`∈` / `⊆` / `∪` / `∩` / `\` / `∅` 是 Lean *core* 记法**（`Init/Core.lean`），
>    而 `𝒫`（`prefix:100 "𝒫 " => powerset`，**全局不是 scoped**）、`''`/`⁻¹'`、`×ˢ`、
>    `⋃ i,`/`⋃₀` 是 **Mathlib 层**记法。⇒ 将来 G-04 落地时，第一组应按 core 级对待
>    （和 `Eq` 平级，属于"最小记法集"），第二组才需要权衡。
> 2. **`↾` 这个记法根本不存在**（`Data/Set/Restrict.lean` 全文无记法；`Set.restrict` 已改名
>    `Set.domRestrict`）——旧教程里的 `f ↾ s` 是 Lean 3 时代的记忆，**不要教**。
> 3. `⊆` 现在是 `Subset` 的**语法相等**（2026-05-24 起 `le_eq_subset` 等全部 deprecated）——
>    这支持我们把 `Set.subset` 作为一等概念来教，而不是"序关系的糖"。
>
> 另外两条与基数单元（7–10）直接相关：**`Set.Finite` 是 `Prop` 值的 protected def（不是
> structure），且 `Set.Finite.card` 不存在**（用 `s.ncard` 或 `h.toFinset.card`）；
> `#s`（`Finset.card`）与 `#α`（`Cardinal.mk`）是**两个不同的 locale**，`Set.ncard` 没有记法。
> ⇒ 我们讲"有限/基数"时只用 `ncard`-式的点名函数，不引入 `#`。

## 4. 记法引入顺序与"无记法"替代（G-04）

| 顺序 | 记法 | 今天怎么写 | 出现在 |
|---|---|---|---|
| 1 | `x ∈ A` | `Set.mem α x A`（= `A x`） | 单元 1 |
| 2 | `A ⊆ B` | `Set.subset α A B` | 单元 2 |
| 3 | `∅` | `Set.empty α` | 单元 2 |
| 4 | `{a}` / `{a,b}` | `Set.singleton α a` / `Set.pair α a b` | 单元 2 |
| 5 | `A ∪ B` / `A ∩ B` / `A \ B` | `Set.union α A B` / `Set.inter α A B` / `Set.diff α A B` | 单元 3 |
| 6 | `𝒫 A` | `Set.power α A` | 单元 3 |
| 7 | `(a,b)` / `A × B` | `Prod.mk`（裸名 `mk`，见 G-02） / `Prod A B` | 单元 5 |
| 8 | `r ⁻¹` / `r ∘ s` | `Rel.inv α r` / `Rel.comp α r s` | 单元 6 |
| 9 | `f '' A` / `f ⁻¹' B` | `Set.image α β f A` / `Set.preimage α β f B` | 单元 8 |
| 10 | `A ≈ B`（等势） | `Set.equiv α β A B` | 单元 9 |

> analysis 项目**一次都没用 `𝒫`**（它用自家 `A ^ B` 幂集）——说明"幂集用子集谓词"这条
> 连成熟项目都这么选，我们照抄。
>
> 教学上"先看见糖、再见记法"反而有利：G-04 落地后，同一单元补一页"同一命题两种写法"，
> 正好是 X 类练习的素材。

---

## 5. 与缺口台账的联动

| 写作时撞到 | 台账条目 | 处置 |
|---|---|---|
| 想写 `axiom f (x : α) : β` | **G-13** | **0.59.0 已修**（WO-008）：直接照 Lean 4 写；codomain 记得落 `Sort n` |
| `Eq x a` 被拒 | 无缺口（语言设计） | 写 `Eq.{1} α x a`，并把这条做成单元 2 的陷阱卡 |
| 想 `inductive And` | **G-02** | 照入门课用 axiom 族；G-02 修好后可升级 |
| 想 `inductive Exists` | **G-03** | 先公理三件套 |
| 想写 `∈`/`⊆` | **G-04** | 点名形式 + 画布头留 TODO |
| 两个 `ctor mk` 撞名 | **G-02** | 构造子先叫 `prod_mk`，修好后机械替换 |
| 签名错字没被发现 | **G-01** | 每单元收尾手工把签名塞进一个 checked 声明 |
| 判定"有没有坏" | **G-10** | **用 `grade` 的退出码**（课程门禁判据）；`query check` 自 0.59.0 起同口径（带 parse 诊断 + exit 1），可交叉复核 |

新撞到的缺口按 `docs/gaps/README.md` 登记，然后
`python3 <语言仓>/scripts/gap.py next` 取下一张工作单（含可粘贴的 prompt）。

---

## 6. 未决问题（调研自己标的"未核实"，别当结论用）

1. **中文教材与中文课堂顺序**：徐明曜/赵春来、耿素云、张锦文等**一份目录都没取到**，
   中文课程大纲也没有 —— 而我们的读者是中文读者。**建议下一轮单独补**。
2. **§1.3 的教育研究文献全部未取证**（APOS/Dubinsky、Tall、Weber、Iannone、
   transition-to-proof）。现在的"学习障碍"表只有**教材证据**。
3. Jech / Kunen 目录未取到（Springer 反爬）。
4. ~~其他证明助手大面积未核实~~ → **已补全**（`prior-art-report.md` §4 第二轮）：
   Isabelle/ZF 38 个 `.thy` + Isabelle/HOL 教程章、Metamath `set.mm`（~26,000 主证明）、
   Mizar MML 5.94（75,158 theorem）、Coq 只有谓词式 `Coq.Sets.*`（**`Coq.ZF` 不存在**）、
   Agda 的 ZF 在 Cubical（`CumulativeHierarchy`，models "ZF − powerset"）、ACL2 集合 =
   有序表。仍未核实：NNG4/Logic Game/Robo 的关卡数与 hint 机制、EuroProofNet 产出、
   若干高校课程的课程号。
5. §3 表里标"待实测"的定理（`mem_singleton_self`、`prod_ext`、`preimage_inter`、
   `preimage_compl`、Cantor、`ℕ×ℕ≃ℕ`）**尚未判过卷** —— 写对应单元时先跑探针。
6. **调研环境事实**（写进 harness 备忘）：本机 `web_search` 缺 API key 时可用
   `bash` + `curl` 经 `HTTPS_PROXY=http://127.0.0.1:7890`；publisher 官网 + Open Library API +
   archive.org metadata 是可靠路径，Bing/Brave 会退化、DDG 反爬。

---

## 7. 待拍板

- **D-S1 记法优先级**：G-04 排在 P1 的 WO 池里，还是先写完整本"点名版"、最后统一补记法？
  （推荐**后者**：内容先行，记法作为第二遍的 X 类练习）
- **D-S2 bridge 单元**：卷 I 假设读者学过入门课，还是加 `unit00` 桥接单元？
  （推荐**加**，卷 I 要能独立）
- **D-S3 英镜像**：是否同步英文镜像（入门课是双语的）？见 `teaching-project.md` D-2。
- **D-S4 单元数**：12 单元（本文档）还是回到 10 单元（`teaching-project.md` §4.2）？
- **D-S6 分层判据确认**：`docs/design/course-stdlib.md` 的三层线（L1 prelude / L2 课程标准库 /
  L3 练习）与那条判据——**"Mathlib 有 ≠ 不该练；Mathlib 有且没有数学内容才归库"**——
  要不要作为课程写作的**硬规则**写进 `REQUIREMENTS.md`？
- **D-S5 游戏化**：要不要顺带做"关卡元数据"（每单元的允许 tactic 白名单 + 提示密度），
  以便将来直接喂给 lean4game 式的关卡系统？stg4 证明了这条路可行。
