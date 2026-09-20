# C2 · 教学系统事实档案（课程 / 卷 I 集合论 / 教学循环 / 缺口台账）

> **用途**：重建项目官网时的**事实底本**。所有数字都从仓库文件数出来，命令写在原处；
> 所有事实都带 `file:line`。本文只陈述仓库里能查证的东西，不写营销话术。
> **生成日期**：本轮会话。**判卷二进制**：`target/debug/sokonanoda`，`source: repo-build`，
> `version: 0.61.0`（`python3 courses/set-theory/tools/check.py --json` 的 `channel` 字段）。

---

## 1. 教学主张 —— 为什么用机器裁判教，而不是用一本教科书

### 1.1 一句话主张

**把"判定"从人（老师/答案册/自己的直觉）手里拿走，交给一个冻结的内核；
课程内容于是只剩下"讲什么、按什么顺序讲"这一件事。**

这个主张在仓库里有逐字的操作定义，写在教师技能的第一条硬规则里：

> 「**判定永远走 kernel**——读 `--json` 结构化事件……禁止文本比对、禁止"看起来对"就判过」
> —— `skills/sokonanoda-teacher/SKILL.md:23`

以及 §0 的角色定义：

> 「你往画布里写讲解、演示定义和练习（带 `sorry` 洞的声明）；用户在洞里作答；
> **完整内核是唯一裁判**——你跑编译器读结构化事件来判卷和决策。判定永远走 kernel，
> 绝不做文本比对。」—— `skills/sokonanoda-teacher/SKILL.md:10-13`

### 1.2 实证依据：新手卡在哪三个地方

`docs/notes/settheory-survey/learning-difficulties.md` 把"过渡到证明"的困难归到三个来源，
并保留了核验级别（`[F]` 全文 / `[A]` 逐字摘要 / `[M]` 仅元数据）：

| 实证 | 数字 | 出处 |
|---|---|---|
| Moore (1994) 三大困难源 | **(a) 概念理解 (b) 数学语言与记号 (c) 不知如何下笔** | `docs/notes/settheory-survey/learning-difficulties.md:262-266`（`[F]`，DOI `10.1007/BF01273731`） |
| Selden & Selden (1995) 拆解成功率 | 简化陈述 **8.5%**，真实教材陈述 **5%** | `learning-difficulties.md:266-269` |
| Weber (2001) | 学生「**有**句法知识却用不上」 | `learning-difficulties.md:273-277` |
| Inglis & Alcock (2012) 眼动 | 新手看**表面特征**而非逻辑结构 | `learning-difficulties.md:277-279` |
| Selden & Selden (2003) 验证能力 | 「**非常有限——也许比他们自己和老师以为的还要有限**」 | `learning-difficulties.md:270-272` |
| Harel & Sowder (1998) 证明图式 | 七种，分三类（外部信念：权威/仪式/非指称符号；经验；演绎） | `learning-difficulties.md:280-285` |

课程设计者从这张表里挑出的**可控变量**只有两个，并逐字写进了大纲：

> 「**(b)(c) 正是本课程最大的可控变量**：7 个 tactic + 无记法 ⇒ 每步几乎只有一个下一步；
> hint 阶梯对准"如何下笔"」—— `docs/design/set-theory-syllabus.md:80`

也就是说：**语言记号面（b）与"下一步写什么"（c）是机器裁判能直接吃掉的**；
(a) 概念理解留给讲解与练习设计。这正是"用机器判卷"的立论位置——它不承诺解决全部三个，
它把其中两个从"靠老师盯"变成"靠内核判"。

### 1.3 三条被调研改写的设计前提

`sokonanoda-lang` 的 `Set` 与 Mathlib 逐字相同，所以"能不能教集合论"不是一个问题：

> 「**Mathlib 的 `Set` 与我们的逐字相同**：`def Set (α : Type u) := α → Prop`
> （`Mathlib/Data/Set/Defs.lean:51`）。**语义零差距，差的只有记法**（G-04）——
> 这条把"我们能不能教集合论"从"能不能"变成"记法好不好看"。」
> —— `docs/design/set-theory-syllabus.md:25-27`

第二条：**"集合论教学层"在证明助手里是明确空白**——

> 「真正把 ZF 做深的（Isabelle/ZF、Metamath `set.mm`、Mizar MML、mathlib3 `set_theory`、
> Cubical Agda）**全部只有库**（外加论文/手册）→ **"从零开始的集合论教学层"是明确空白**，
> 正是 sokonanoda 的落点。」—— `docs/notes/settheory-survey/prior-art-report.md:153`

第三条：游戏化先例存在，作者是 Velleman 本人（`djvelleman/stg4`，8 worlds / 51 关），
本项目抄的是它的三个机制（逐步解锁语法 / 三层 Hint / `Branch` 死路）而不是从零发明关卡
—— `docs/design/set-theory-syllabus.md:22-23`、`docs/notes/settheory-survey/prior-art-report.md:104-128`。

### 1.4 "消除暴力"：课程存在的另一半理由

课程线自己的判据（三层分界的总纲）是：

> 「**暴力 = 把"语言/标准库该给的东西"塞进课程内容让学习者手写。**」
> —— `docs/design/course-stdlib.md:24`

触发这句话的是用户原话（逐字引用在文档开头）：

> 「你的教程出的题目，在做的时候，你会发现要补充很多其他的定理，边边角角的定理，
> 这个是在其他的教程里头会认为是天然应该知道的，或者说是标准库里已经实现的。
> 这个我感觉有点暴力，所以我需要你去用这个项目重新做一遍，然后才能把这些暴力给消除掉。
> 你要记录下这些其实是需要实现的，其实是没有的。」
> —— `docs/design/course-stdlib.md:3-6`

**这就是"课程驱动开发"的起点**：写课程时每撞到一次"这东西本该有"，就记一条缺口；
缺口台账（本文 §7）是这条主张的工程化形态。

---

## 2. 卷 I《集合论》全表

### 2.1 计数命令（表里每个数字都由这些命令产出）

```bash
# 单元清单（结构化真相：卷→章→单元 + 先修/标签/配额）
cat courses/set-theory/course.json

# 每个画布的 sorry 洞数 / 声明数
for f in courses/set-theory/units/unit*.sokonanoda; do
  echo "$(basename $f): sorry=$(grep -c '^\s*sorry\s*$' $f) decls=$(grep -cE '^(theorem|def|inductive|axiom) ' $f)"
done

# 每个画布的具名练习（= 值位是 sorry 的具名声明）
for f in courses/set-theory/units/unit*.sokonanoda; do
  echo "### $(basename $f)"
  awk '/^(theorem|def|inductive|axiom) /{name=$2}
       /^[[:space:]]*sorry[[:space:]]*$/{if(name!=""){print "  EX:"name; name=""}}' "$f"
done

# 课程门禁的权威计数（36 目标 / checked / open / 判负）
python3 courses/set-theory/tools/check.py --json
```

实测（本轮，`0.61.0` 仓库构建）：
**36 个目标 · 329 checked · 99 open · 0 判负**；
`summary` 细分 `canvas_open: 96` / `solutions_open: 0` / `lib_open: 0`
（`python3 courses/set-theory/tools/check.py --json` 的 `summary` 字段）。
12 个单元的画布 `sorry` 洞合计 **96**，与 `canvas_open: 96` 一致；
差 3 的是记法对照页（`units/notation-cheatsheet.sokonanoda`，3 道记法练习，
它**不是单元**、不进 `course.json`，但按同一套 G1/G3/G4 判卷
—— `courses/set-theory/README.md:85`）。

### 2.2 12 个单元

**标题（中/英）**逐字取自 `courses/set-theory/course.json`（中：`:19-114` 各 `title`；英：各 `title_en`）。
**具名练习数 = `sorry` 洞数**（本卷两者恰好相等，都是 6/10/11/8/7/8/9/9/7/7/5/9）。
**章/前置**取自 `course.json` 的 `chapters[].id` 与 `chapters[].prereqs`
（`:11-14`、`:44-47`、`:71-74`、`:98-101`）；**单元级先修**是单元文件头部自己声明的更细依赖。

| # | 标题（中） | 标题（英） | 教什么概念 | 具名练习 | `sorry` 洞 | 前置（章 → 单元级） | 画布代码里出现的记法 |
|---:|---|---|---|---:|---:|---|---|
| 1 | 单元① 集合与隶属 | Unit 1 — Sets & Membership | 集合 = 论域上的谓词（`Set α := α → Prop`）→ 隶属 = 函数应用 → 外延性 `Set.ext`（作公理） | 6 | 6 | I.1（无） | 无（全点名：`Set.mem` / `Set.subset`） |
| 2 | 单元② 子集、空集与包含三律 | Unit 2 — Subsets, the Empty Set, and the Three Laws of ⊆ | `⊆` 是偏序（自反/传递/反对称）、空集是最小元、单元素集与配对集的成员判定 | 10 | 10 | I.1 | 无 |
| 3 | 单元③ 并、交、差与幂集 | Unit 3 — Union, Intersection, Difference & Powerset | 三个二元运算（并=`Or`、交=`And`、差=`Not` 定义）→ 幂集（= 子集谓词）→ 成员判定与单调性 | 11 | 11 | I.1 | `𝒫`（1 处，`:61` 的 `demo_mem_powerset` 演示） |
| 4 | 单元④ 外延性与集合等式 | Unit 4 — Extensionality & Set Identities | 外延证明套路（`Set.ext` + 逐元素 `Iff`）→ 布尔代数恒等式表 → 形式↔散文互译 | 8 | 8 | I.1 | 无 |
| 5 | 单元⑤ 序对与笛卡尔积 | Unit 5 — Ordered Pairs & Cartesian Products | 序对是语言原语 `inductive Prod` → 配对定理 → `A × B` 的成员刻画；Kuratowski 编码 `(a,b) := {{a},{a,b}}` 只作**阅读题** | 7 | 7 | I.2 ← I.1 | 无 |
| 6 | 单元⑥ 关系 | Unit 6 — Relations | 关系 = `A -> B -> Prop` → 逆与复合 → 等价关系三律 → 等价类 → 划分↔等价关系 | 8 | 8 | I.2 ← I.1 | 无 |
| 7 | 单元⑦ 函数 | Unit 7 — Functions | 函数即单值关系 **vs** 语言原语 `A -> B` → 复合（复用单元⑥）→ 单射/满射/双射 → 逆（**数据版**） | 9 | 9 | I.2 ← I.1；单元级：② 与 ⑥（`unit07-functions.sokonanoda:7-8`） | 无 |
| 8 | 单元⑧ 像与原像 | Unit 8 — Images & Preimages | `Set.image`（前推）/ `Set.preimage`（回拉）→ 原像保并交补（全保）、像只保并 → 纤维 | 9 | 9 | I.3 ← I.2；单元级：⑦ 与 ③（`unit08-images-preimages.sokonanoda:7-8`） | `''`（1 处，`:130` 的记法演示 `demo_image_mono_notation`） |
| 9 | 单元⑨ 等势（基数 Ⅰ） | Unit 9 — Equinumerosity (Cardinality I) | 等势用**数据**定义（一对互逆映射 + 域条件，必须亲手写出映射）→ 等势是等价关系 → 单射+右逆 ⇒ 等势 | 7 | 7 | I.3 ← I.2；单元级：①②（`unit09-equinumerosity.sokonanoda:5`） | 无 |
| 10 | 单元⑩ 可数与 Cantor 定理（基数 Ⅱ） | Unit 10 — Countability & Cantor's Theorem (Cardinality II) | 可数（与 `ℕ` 等势）→ **先给一个假枚举找矛盾** → 对角线 → Cantor 定理 → 选择公理（本卷唯一一次动手用） | 7 | 7 | I.3 ← I.2 | 无 |
| 11 | 单元⑪ 论域与 Russell 悖论 | Unit 11 — Universes & Russell's Paradox | 概括公理为什么不能在类型论里写 → Russell 集 → 类型论的答案（每个类型有自己的 `Set.univ α`）→ 与 ZF 的对照 | 5 | 5 | I.4 ← I.1 · I.3 | 无 |
| 12 | 单元⑫ 综合与读证明 | Unit 12 — Synthesis & Reading Proofs | 不引入新概念：形式↔散文互译 → 给错证明找错 → 期末小项目（串「集合→关系→函数→基数」的链） | 9 | 9 | I.4 ← I.1 · I.3；单元级：①–⑪（`unit12-synthesis.sokonanoda:6`） | 无 |

**合计**：12 单元 · **96** 道具名练习 · **96** 个 `sorry` 洞。

### 2.3 章结构与配额（`quota.exercises` 是**计划**，门禁只报告差额、永不判红）

| 章 | 标题 | 先修 | 标签 | 计划练习 | 画布实测 | 单元 | 出处 |
|---|---|---|---|---|---:|---:|---|---|
| I.1 | 集合、子集与集合运算 | — | membership · subset · powerset | 35 | 35 | 1–4 | `course.json:11-15` |
| I.2 | 序对、关系与函数 | I.1 | ordered-pair · product · relation · function | 24 | 24 | 5–7 | `course.json:44-48` |
| I.3 | 像、原像与基数 | I.2 | image · preimage · cardinality · cantor | 23 | 23 | 8–10 | `course.json:71-75` |
| I.4 | 论域、悖论与综合 | I.1 · I.3 | universe · russell · synthesis | 14 | 14 | 11–12 | `course.json:98-102` |

差额全为 0，逐条印在门禁的 `quota_notes` 里：

```
"chapter I.1 计划练习 35 · 画布实测 35（差额 0）"
"chapter I.2 计划练习 24 · 画布实测 24（差额 0）"
"chapter I.3 计划练习 23 · 画布实测 23（差额 0）"
"chapter I.4 计划练习 14 · 画布实测 14（差额 0）"
```

### 2.4 依赖树（大纲锁定版）

> 「依赖树：1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → {9, 10} → 12；**11 可在 4 之后随时插**。」
> —— `docs/design/set-theory-syllabus.md:172`

### 2.5 记法对照页（不是单元，但同判）

`courses/set-theory/units/notation-cheatsheet.sokonanoda` 把**同一个命题的两种写法**并排：
点名形式 ↔ 数学记法。代码里实际用到 `∈`(7) `⊆`(10) `∅`(5) `∪`(8) `𝒫`(1) `ᶜ`(1)
（`grep -vE '^[[:space:]]*--' <file> | grep -cF -- '<token>'`）。
它自身声明 4 条 core 级记法（`∈`/`⊆`/`∪`/`∅`），另外 5 条集合论专用符号
（`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`）写在 `lib/Set.sokonanoda` 末尾、随 `import` 传播
—— `courses/set-theory/README.md:165-190`、`courses/set-theory/lib/Set.sokonanoda`
（记法块在文件末尾，5 行 `prefix:`/`postfix:`/`infixr:`）。

门禁对它的计数：画布 **18 checked + 3 练习**，解答 **21 checked**
（`courses/set-theory/README.md:162-163`；本轮 `--json` 复核一致）。

---

## 3. 每单元一段人话介绍

> 每段回答三个问题：**这一单元在回答什么问题 · 你亲手证出什么 · 哪里会意外**。
> 练习名逐字取自画布文件；行号是该单元**第一个具名练习**的位置，方便回查。
> 练习类型记号（T 项填空 / B `by` 填空 / L 自证引理 / R 读评译 / X 形式↔散文 / D 证明或证伪）
> 见 `docs/design/set-theory-syllabus.md:151-153`。

### 单元① 集合与隶属（`units/unit01-sets-membership.sokonanoda`）

「集合」到底是什么？这一单元的回答是：**集合就是"问一个元素是不是它的成员"这件事本身**
——写成 `Set α := α → Prop`，于是 `x ∈ A` 不是新概念，就是"把 `x` 喂给 `A`"（函数应用）。
你要证的第一个东西（`:34` `mem_of_subset`）只有一行：手里有 `A ⊆ B`，把它当函数用在 `a` 上。
意外在练习 2（`:46` `eq_of_same_elements`）：**"两个集合元素完全相同就相等"在本课程里是一条公理**
（`Set.ext`），不是定义——你以为最显然的那一步，恰恰是要单独假设的那一步。
练习 6（`:81` `singleton_eq_singleton_iff`）会把你推到一个更细的问题上：`{a} = {b}` 与 `a = b` 是不是同一件事？

### 单元② 子集、空集与包含三律（`units/unit02-subsets-empty.sokonanoda`）

`⊆` 到底是个什么关系？这一单元用三道题（`:21` `subset_refl` · `:28` `subset_trans` · `:36` `subset_antisymm`）
把"自反、传递、反对称"一条条证出来——**反对称就是你平时"证明两个集合相等"用的那招**：
互相包含就够了。接着是空集：`:55` `empty_subset` 要你证 `∅ ⊆ A`，而它的证明**前提永远拿不到**
（你 intro 进来的 `x ∈ ∅` 展开就是 `False`），这就是"空洞蕴含"的第一次登场。
意外在于，很多初学者根本不接受 `∅` 是一个集合，理由是"里面没有元素可以指认"
——`docs/notes/settheory-survey/learning-difficulties.md:103-107`（`[A]`，Hendriyanto et al. 2024）
记录了这件事，所以本单元把它当成一个**要显式讲清的动作**而不是默认常识
（`docs/design/set-theory-syllabus.md:158`）。
最后一题 `singleton_of_singleton`（`:118`）问的是 `{a} = {b}` 里到底发生了什么。

### 单元③ 并、交、差与幂集（`units/unit03-union-inter-powerset.sokonanoda`）

有了子集，就可以造新集合。并、交、差不是"新的数学"，它们是**三个逻辑联结词换了个马甲**：
`∪` 就是 `Or`、`∩` 就是 `And`、`\` 就是 `Not`（`:39-49` 的三条演示把这件事直接摊开）。
幂集 `𝒫 A` 的定义是"子集谓词"——于是"`X ∈ 𝒫 A`"与"`X ⊆ A`"是同一句话
（`:179` `powerset_self_mem` 就是让 `A` 自己当那个 `X`）。
最容易踩的坑在本单元正中：`A ∈ 𝒫 A` 是**真的**，而 `A ⊆ 𝒫 A` **连类型都不对**
（`:172-176` 的注释逐字记录了这次勘误——大纲原稿把前者写成了假命题）。
还有一个反复出现的题型（`:162` `union_subset_inter_false`）：**"`A ∪ B ⊆ A ∩ B`"是假的**，
你得说出它什么时候才真。

### 单元④ 外延性与集合等式（`units/unit04-extensionality-identities.sokonanoda`）

怎么证明两个集合相等？这一单元把套路固定成三步：`Set.ext` → 逐元素 → 两边各证一个 `Iff`。
然后拿它去跑一张**布尔代数恒等式表**：交换律（`:70` `union_comm`）、幂等（`union_inter_idem`）、
分配律（`inter_union_distrib_left`）、差集的差集（`sdiff_sdiff`）。
意外在最后：`:165` `union_sdiff_univ_ne_univ` 是一个**假的**等式，你要证它的否定——
这是本课程第一次要求你"用反例赢"，而不是"用证明赢"。
本单元还承担一件事：把同一句话的**散文版与形式版**互相翻译（X 类题），
因为"数学语言与记号"是 Moore (1994) 列出的三大困难源之一
（`docs/notes/settheory-survey/learning-difficulties.md:262-266`）。

### 单元⑤ 序对与笛卡尔积（`units/unit05-pairs-products.sokonanoda`）

`(a, b)` 是什么？教科书里的标准答案是 Kuratowski 的集合编码 `(a,b) := {{a},{a,b}}`；
本课程走另一条路：**序对是语言原语**（`inductive Prod`），
你证的是它的行为而不是它的编码——`:76` `prod_mk_inj`（`(a,b) = (a',b') ⇒ a = a' ∧ b = b'`）
与 `:85` `prod_mk_eq_iff`。
Kuratowski 编码只作**阅读题**（5.6），原因是**没有实证研究**支撑"学生被这个编码卡住"
（ERIC `"Kuratowski ordered pair"` 返回 0 条；`learning-difficulties.md:124-129`，
大纲据此把它降级：`docs/design/set-theory-syllabus.md:82`）。
意外是 `:134` `prod_set_swap_ne`：`A × B` 与 `B × A` 一般**不相等**——交换是有代价的。

### 单元⑥ 关系（`units/unit06-relations.sokonanoda`）

"关系"是集合论里最没有神秘感、也最容易被含糊过去的概念。这里它被写成 `A -> B -> Prop`
（一个吃两个元素、返回命题的函数），于是自反/对称/传递（`:48-52`）都成了可以逐字展开的定义。
你要证的是一条链：逆关系与复合的结合律（`rel_comp_assoc`）、等价类"要么互斥要么相同"
（`equivClass_eq_iff`）、以及**划分与等价关系是一回事**（`classes_isPartition`）。
本单元最值得看的是两道 D 类题。`:213` `not_symm_trans_implies_refl` 要你**自己找反例**：
"对称 + 传递 ⇒ 自反"是错的，而反例要在**非空**论域上找——空论域上自反会空洞成立，
这层"空洞"正是初学者最容易漏掉的地方。旁边还有 `nearStep_counterexample`：
一个"看起来差一点就对"的命题，要你指出差在哪一步。

### 单元⑦ 函数（`units/unit07-functions.sokonanoda`）

这一单元的导入语直接给出它存在的理由：**Breidenbach et al. (1992) 实测，在本该 ~100% 判为"函数"的
24 个情境里，学生只有约 40% 同意**（图 19–40.7%、方程 25–31%）
—— `docs/notes/settheory-survey/learning-difficulties.md:143-153`（`[F]`），
课程把这组数字写进单元开头（`unit07-functions.sokonanoda:19-24`）。
于是本单元把两条路并排：**函数作为单值关系**（教科书路线）与**函数作为语言原语 `A -> B`**。
你证复合结合律（`:96` `comp_assoc`）、单射/满射在复合下的行为、以及"左逆 ⇒ 单射"。
最硬的实证在这一单元：`:170` 附近 `swap_converse_false` 要你证明 `∀x∃y` **换不成** `∃y∀x`
—— Dubinsky & Yiparaki (2000) 实测 **94% 的学生至少把一个 EA 陈述读成 AE**（反向只有 5%）
（`learning-difficulties.md:162-170`，⚠️ **未发表手稿**，引用须注明状态）。
本单元还有一条**语言边界**：`bijective_iff_inverse` 只能是**数据版**（逆函数由调用者交出来），
因为"满射 ⇒ 存在右逆函数"要取数据、等于选择公理（`docs/design/course-stdlib.md:224-255`、台账 L-06）。

### 单元⑧ 像与原像（`units/unit08-images-preimages.sokonanoda`）

把函数用在集合上，会得到两个方向相反的操作：前推 `f '' A` 与回拉 `f ⁻¹' B`。
这一单元的核心不对称是：**原像对并、交、补全都保**（`:72` `preimage_union` · `:81` `preimage_compl` ·
`preimage_inter`），**像只保并**。证据是两道 D 类题：`:295` `not_image_inter_eq_image_inter`
要你证 `f '' (A∩B) = f '' A ∩ f '' B` **是假的**（反例：两个点都送到同一个点），
以及 `not_image_preimage_eq`（`f '' (f ⁻¹' C) = C` 也是假的）。
这一块在文献里是**最薄的一环**——`learning-difficulties.md:189-192` 明说
「没有一篇被广泛引用的论文研究学生对像与原像的理解」，所以课程把力气放在"亲手造反例"上。
`:329` `image_preimage_image_eq_preimage` 是那道把两条反例闭环起来的桥：
反例只能告诉你"两边不等"，这道题才说出"像那一侧到底是什么"。

### 单元⑨ 等势（基数 Ⅰ）（`units/unit09-equinumerosity.sokonanoda`）

两个集合"一样大"是什么意思？答案不是数数，是**造一对互逆的映射**。
本单元强迫你**亲手把那个映射写出来**——因为 Hamza & O'Shea (2011) 实测
「援引双射判据的人没有一个在所有题上都用它，且极少真的写出映射」
（`learning-difficulties.md:210-216`）。
你要证等势是等价关系（`:48` `Set.Equiv.refl`、`Set.Equiv.symm`、`Set.Equiv.trans`），
以及"单射 + 右逆 ⇒ 等势"（`:63` `Set.Equiv.of_inj_surj`）。
意外是 `proper_subset_counterexample`：**真子集不一定"更小"**——这是无限集第一次咬人。
（本单元的等势只能是 `Prop` 值，因为语言没有累积性；`docs/design/course-stdlib.md:257-276`。
Schröder–Bernstein 与有限集/鸽笼按边界表**刻意不做**：`docs/design/set-theory-syllabus.md:167`。）

### 单元⑩ 可数与 Cantor 定理（基数 Ⅱ）（`units/unit10-cantor.sokonanoda`）

什么样的无限集是"能列出来的"？本单元先给你一个**假的枚举**（`:46` `fake_enum`），
让你自己找出那个漏掉的集合（`:48` `demo_fake_enum_escapes`）——
这个顺序是照着实证设计的：Zazkis & Mamolo (2009) 发现学过 Cantor 定理的学生
面对一个假的 ℝ 枚举只会说 "Cool!"，**根本没察觉矛盾**
（`learning-difficulties.md:217-221`；课程把这条写进单元头 `unit10-cantor.sokonanoda:15-18`）。
然后是重头戏：`:127` `cantor`（`α` 与 `𝒫α` 不等势），对角线集合是 `D := fun x => Not (f x x)`
（`:118` `diag_mem_iff` 是它的成员判定）；`:140` `powerset_nat_not_countable` 是它在 `ℕ` 上的实例。
本单元还**唯一一次**动手用选择公理（`:194` `choice_split`），
并在末尾点出"Mathlib 的基数建立在选择上"当作本课程独有的增量
（`docs/design/set-theory-syllabus.md:52`）。

### 单元⑪ 论域与 Russell 悖论（`units/unit11-universe-russell.sokonanoda`）

"所有集合的集合"为什么不能有？这一单元把 Russell 悖论放进类型论里重新问一遍。
答案分三层（`:110-114` 的注释逐字列出）：**跨类型读法**（一个集合装下所有类型的集合）——
量词必须先固定一个 `α`，写不出来；**类型内读法**（"一切"= 本类型的集合）——**真**，
就是 `Set.univ α`；**类型内但加强成"严格大于每个集合"**——**假**，
这就是 `:122` `no_univ_strictly_larger`，而它的证明里藏着 Russell 论证的那一步：
**把候选者自己代进去**（`hU U` 给出 `U ≠ U`，撞 `Eq.refl`）。
意外是 `powerset_univ_eq_univ_of_sets`：在类型内，`𝒫(univ)` 与 `univ` 是同一层的东西，
所以"一切集合"在这里有一个**不自我指涉**的落点。

### 单元⑫ 综合与读证明（`units/unit12-synthesis.sokonanoda`）

这是期末单元，**不引入任何新概念**（`:6`）。三件事：
形式↔散文互译（X 类 4 题，散文写在题面注释里，形式版必须过内核）、
给错证明找错（R 类 3 题，`:278` `flawed_equalities_refuted` 承担 D 类必破）、
以及期末小项目（`:445` `project_chain` 与 `:469` `project_chain_cardinal`）——
串起「集合 → 关系 → 函数 → 基数」的一条**七环链**（环 1–5 在前面的单元，
环 6 是等势演示，环 7 是基数收口题；`:20-24` 的注释列了这七环）。
它存在的理由是 Selden & Selden (2003) 的实测：本科生的**证明验证能力"非常有限——
也许比他们自己和老师以为的还要有限"**（`learning-difficulties.md:270-272`），
所以"读"必须单独练，不能指望它跟着"写"自动长出来。

---

## 4. L2 课程标准库（`courses/set-theory/lib/`）

### 4.1 治理规则（一句话）

> 「**Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库。**」
> —— `docs/design/course-stdlib.md:27`

三层分界（`docs/design/course-stdlib.md:31-35`）：

| 层 | 放什么 | 判据 | 归属 |
|---|---|---|---|
| **L1 prelude** | 逻辑与等式的骨架（`True`/`False`/`And.*`/`Or.*`/`Not`/`absurd`/`Iff.*`/`Eq.symm`/`Eq.trans`/`congrArg`） | **Lean core 里现成** | 语言仓（prelude 扩展） |
| **L2 课程标准库** | 集合的**词汇 + 定义展开** | Mathlib 里是 `rfl` 或一行、**没有数学内容** | 课程仓 `lib/` |
| **L3 单元练习** | 一切**有数学内容**的陈述（包含三律、分配律、像/原像、等价关系与划分、基数…） | 需要"想一下"才写得出来，哪怕 Mathlib 已有 | 课程 `units/` |

这条规则是**踩过坑之后立的**：第一遍把 16 条 L3 引理（`subset_refl` / `union_subset_iff` /
`inter_comm` / `inter_union_distrib_left` …）写进了 `lib/Set.sokonanoda`，
后果是"学习者在做练习时会发现答案已经在库里"——记入台账 **L-05**
（`docs/design/course-stdlib.md:37-45`，`docs/gaps/ledger.jsonl` 的 L-05 行）。
今天 `lib/Set.sokonanoda` 的文件头逐条列出了"只允许留哪三类"，
并写明 `grep -rn` 复核过那些内容型名字在整个 `lib/` 里**零出现**
（`courses/set-theory/lib/Set.sokonanoda:15-38`）。

### 4.2 内容与计数（9 个文件 = 8 个模块 + 1 个自检入口）

```bash
for f in courses/set-theory/lib/*.sokonanoda; do
  echo "$(basename $f): decls=$(grep -cE '^(theorem|def|inductive|axiom) ' $f) sorry=$(grep -cE '^\s*sorry\s*$' $f)"
done
# Demo 10 · Equiv 5 · Exists 3 · Fun 14 · Image 6 · Logic 0 · Prod 6 · Rel 7 · Set 23
```

| 模块 | 内容 | 声明数 |
|---|---|---:|
| `lib/Logic.sokonanoda` | **空壳模块**（P4，2026-09-19）：**0 条声明**，只剩注释——30 个名字由 prelude 自带；34 个 `import lib.Logic` 一字未改 | **0** |
| `lib/Set.sokonanoda` | 定义 12 条（`Set`/`mem`/`subset`/`empty`/`univ`/`singleton`/`pair`/`union`/`inter`/`sdiff`/`compl`/`powerset`）+ `Set.ext`（**公理**）+ 展开引理 10 条 | **23** |
| `lib/Exists.sokonanoda` | **G-03 已修（0.59.0）**：`Exists` 现在是**真归纳**（`Exists.intro` 是归纳块的构造子，不计入声明数），`Exists.elim` 由自动派生的 `Exists.rec` **定义**出来，外加便利引理 `Exists.imp`。名字与签名**逐字不变** ⇒ `units/` 里 186 处点名调用零改动（`courses/set-theory/README.md:205-212`） | **3**（`Exists` · `Exists.elim` · `Exists.imp`） |
| `lib/Prod.sokonanoda` | `inductive Prod`（构造子 `prod_mk` 是归纳块的一部分，**不计入声明数**）+ 投影 `Prod.fst`/`Prod.snd` + 展开引理 `Prod.fst_mk`/`Prod.snd_mk` + 库自检 `Prod.fst_snd_mk` | **6** |
| `lib/Rel.sokonanoda` | 关系词汇 3 条（`Rel`、`Rel.inv`、`Rel.comp`）+ `Rel.ext`（**公理**）+ 展开引理 3 条 | **7** |
| `lib/Fun.sokonanoda` | `Function.comp`；`Injective`/`Surjective`/`Bijective`；**数据版** `LeftInverse`/`RightInverse`/`Inverse`；展开引理 7 条 | **14** |
| `lib/Image.sokonanoda` | `Set.image`（用 `Exists` 写）、`Set.preimage` + `Set.mem_image`/`Set.mem_preimage` + `Set.image_mono`/`Set.image_subset_iff` | **6** |
| `lib/Equiv.sokonanoda` | 等势四条件（`Set.MapsTo`/`Set.LeftInvOn`/`Set.RightInvOn`）+ 等势本体 `Set.Equiv`（**Prop 值**）+ 构造子 `Set.Equiv.mk` | **5** |
| `lib/Demo.sokonanoda` | 自检入口：`import` 各模块并真的判卷（10 条演示：`demo_and_comm` · `demo_subset_unfold` · `demo_mem_powerset` · `demo_exists_intro` · `demo_exists_elim` · `demo_prod_fst` · `demo_rel_inv_inv_apply` · `demo_fun_comp_apply` · `demo_mem_preimage` · `demo_equiv_refl`） | **10** |

**逐文件相加 = 74 条声明，全部 `checked`、`0 open`、`0 failed`**
（`docs/design/course-stdlib.md:117-119`；本轮 `--json` 复核 `lib_open: 0`）。
**口径提醒**：门禁 `--json` 的 `summary.checked` 把 `lib/Demo.sokonanoda` 算两次
（`lib Demo` 与 `lib 自检` 判同一个文件），所以汇总额比逐文件相加多 10
（`courses/set-theory/README.md:192-196`）。

其中 `Set`+`Exists`+`Prod` = **32 条**是"从 0 补出来的标准库欠账"
（L-02…L-04 + G-02/G-03 的变形产物；原本还算上 `Logic` 的 26 条，已由 prelude 接管，
所以标准库欠账**净减 26**）—— `docs/design/course-stdlib.md:120-122`。

依赖图（`import` 实测，`docs/design/course-stdlib.md:124-133`）：

```text
Logic ──┬─ Set ──┬─ Image
        │        └─ Equiv
        ├─ Exists ─┬─ Rel
        │          ├─ Fun
        │          └─ Image / Equiv
        └─ Prod（只依赖 Logic）
```

### 4.3 三条示例声明（逐字引用）

**例 1 —— 库的根：集合就是谓词，与 Mathlib 逐字相同。**
`courses/set-theory/lib/Set.sokonanoda:51`

```sokonanoda
def Set (α : Type) : Type := α -> Prop
```

**例 2 —— 典型的 L2 声明：一条"定义展开"引理，证明是两个恒等函数，没有数学内容。**
`courses/set-theory/lib/Set.sokonanoda:83-87`

```sokonanoda
theorem subset_def (α : Type) (A B : Set α) :
    Iff (subset α A B) (forall (x : α), A x -> B x) :=
  Iff.intro (subset α A B) (forall (x : α), A x -> B x)
    (fun (h : subset α A B) => h)
    (fun (h : forall (x : α), A x -> B x) => h)
```

**例 3 —— 一条"设计判断"而不是"Mathlib 真名"：等势必须是 `Prop` 值。**
`courses/set-theory/lib/Equiv.sokonanoda:111-116`（理由在 `:61-72`：本语言**没有累积性**，
`def X : Type := <一个 Prop>` 会被内核直接拒 `类型不匹配：期望 Sort(1)，实际是 Sort(0)`；
而 `And` 与 `Exists` 都落 `Prop`，所以四条件只能是 `Prop`）

```sokonanoda
def Set.Equiv (α β : Type) (A : Set α) (B : Set β) : Prop :=
  Exists (α -> β) (fun (f : α -> β) =>
    And (Set.MapsTo α β f A B)
      (Exists (β -> α) (fun (g : β -> α) =>
        And (Set.MapsTo β α g B A)
          (And (Set.LeftInvOn α β g f A) (Set.RightInvOn α β f g B)))))
```

> 命名纪律：`lib/` 一律用 **Loogle 取证版**名字（`Set.notMem_empty` 大写 M、
> `Set.mem_powerset_iff` 不是 power、`Set.mem_sdiff` 不是 diff、
> `Set.Subset.refl/trans/antisymm` 点号在 `Subset` 上）
> —— `courses/set-theory/lib/Set.sokonanoda:40-42`、
> `docs/design/course-stdlib.md:159-178`。
> 已取证的**不存在**名字（别写进教程）：`Set.subset_antisymm`、`Set.not_mem_empty`、
> `Set.Equiv`、`Set.EquivalentOn`、`Set.Finite.card`、`Set.image_subset_image`、
> `Set.compl_compl`、`↾` 这个记法、`Mathlib/Order/Set` 目录
> —— `docs/design/course-stdlib.md:176-178`。

---

## 5. 入门课（`course/`）——11 个单元

> 标题逐字取自 `course/course.json`（中：`:2-67` 各 `title`；英：各 `title_en`）。
> 一行一句 = 这一单元让学习者第一次做到什么。

| # | 标题（中 / 英） | 一句话 |
|---:|---|---|
| 1 | 单元① 命题与证明项 / Unit 1 — Propositions & Proof Terms | 第一次明白"证明就是一个项"：`True` 的证明是 `True.intro`，`And a b` 的证明是把两个证明交进去。 |
| 2 | 单元② 等式与 rfl / Unit 2 — Equality & rfl | 第一次亲手写 `Eq.subst` 把 `a = b` 当"搬运工"用，并自己造出 `eq_symm` / `eq_trans`。 |
| 3 | 单元③ 函数与箭头 / Unit 3 — Functions & Arrows | 第一次把类型箭头读成函数：`fun` 一层层剥参数，`let` 给中间结果起名，函数可以当参数也可以当返回值。 |
| 4 | 单元④ `by` 写法：tactic 证明 / Unit 4 — Proving with `by` (tactic blocks) | 第一次用 `by` 块里的 `intro`/`exact`/`apply`/`assumption` 写证明——课程把它提前到第④单元当"反馈加速器"（`course/README.md:15-16`）。 |
| 5 | 单元⑤ 宇宙：函数类型的类型 / Unit 5 — Universes: the Type of a Function Type | 第一次回答"`Nat -> Nat` 自己的类型是什么"：`Prop = Sort 0`、`Type 0 = Sort 1`，并写出宇宙多态的 `Eq.symm`。 |
| 6 | 单元⑥ 归纳与递归 Ⅰ / Unit 6 — Induction & Recursion I | 第一次显式写 `inductive Nat` + 手写 `Nat.rec` + 用 `match` 递归（递归字段自动获得归纳假设 `ih`）。 |
| 7 | 单元⑦ 归纳与递归 Ⅱ / Unit 7 — Induction & Recursion II | 第一次处理参数化归纳（`Option`）、依赖 `match`（= 数学归纳法）、嵌套/通配模式与带索引归纳（`Vec`）。 |
| 8 | 单元⑧ 量词：forall 与 exists / Unit 8 — Quantifiers: forall & exists | 第一次用 ∀ 引入=`fun`、∀ 消去=应用、∃ 引入=交证人+性质、∃ 消去=把函数交给 `elim`（`docs/teaching-session.md:107-112`）。 |
| 9 | 单元⑨ 关系与联结词 / Unit 9 — Relations & Connectives | 第一次把 `Or` 从公理升级成真 `inductive`（自动派生 `Or.rec`）、把 `Iff` 当 `def` 展开、把 `Le`/`Even` 立成归纳关系。 |
| 10 | 单元⑩ 读证明与综合 / Unit 10 — Reading Proofs & Synthesis | 第一次"读"：自解释三问、formal↔informal 互译、评阅错证明、期末小项目——不教新语法（`course/README.md:22-24`）。 |
| 11 | 单元⑪ 模块与项目 / Unit 11 — Modules & Projects | 第一次把编译单元从"一个文件"升级为"入口文件 + `import` 闭包"：`import Foo.Bar` 置顶、模块名↔路径、`sokonanoda.toml` 项目根（`course/README.md:24-28`）。 |

### 5.1 与卷 I 的关系（四条硬差别）

| 维度 | 入门课 `course/` | 卷 I `courses/set-theory/` |
|---|---|---|
| 前置 | 无（从"什么是证明"开始） | **假设读者已学完入门课**（逻辑、`by`、归纳、量词），直接从"集合 = 谓词"开始（`courses/set-theory/README.md:7-8`） |
| 单元数 | 11（`course/course.json:1-68`） | 12（`courses/set-theory/course.json:16-117`） |
| 清单格式 | **v1 扁平数组**（一个字节都没改） | **v2 `soko.course/2`**：卷→章→单元 + 先修/标签/配额（`courses/set-theory/README.md:97-99`） |
| 标准库 | 画布**故意各自自给自足**，不 `import`（`course/README.md:53-58`）；逐字重复的块在 `shared/` 里各有一份规范副本，由 `crates/cli/tests/course_shared.rs` 双向守住 | 画布 `import lib.*` 共享 **L2 课程标准库**（9 个文件 / 74 条声明），由 `courses/set-theory/tools/check.py` 判 G1–G6 |

**共同点**：两者都是**agent 面向的素材库**，不是要照着念的固定课程——
「`course/` 只是大模型的路线图/素材库；执行层必须按用户灵活适配」，
而且「用户面对的**永远只是当前画布**（`playground.sokonanoda` 或其分支），
不是 `course/` 文件本身；`course/` 文件永不直接丢给用户当"课程"读」
—— `docs/teaching-session.md:9-18`。

**双语**：入门课有英文镜像（`course/en/`，与中文画布**代码逐字节一致、仅注释语言不同**，
由 `crates/cli/tests/course.rs` 的镜像守卫比较两版事件计数；
`course/README.md:41-51`）。卷 I 的英文镜像仍是待拍板项（`docs/design/set-theory-syllabus.md:274`）。

---

## 6. 教学循环 —— 一次真实会话的逐步流程

> 来源：`docs/teaching-session.md`（开课手册 + 事件决策表）与
> `skills/sokonanoda-teacher/SKILL.md`（教师技能正文，356 行）。
> 角色划分：**agent = 老师，用户 = 学习者，内核 = 唯一裁判**
> （`skills/sokonanoda-teacher/SKILL.md:8-13`）。

### 6.1 开课前：老师先确认环境（三条命令）

```bash
scripts/soko doctor --json      # 就绪诊断，0=就绪 3=未就绪；未就绪就 setup
scripts/soko grade playground.sokonanoda --json   # 拿当前画布状态
scripts/soko setup              # 版本锁定的 CLI + LSP → 缓存（幂等）
```

顺序写在技能的第一步里（`skills/sokonanoda-teacher/SKILL.md:15-19`）。
`scripts/soko` 是 **harness 中立启动器**：解析顺序 = `$SOKONANODA_BIN` → 版本**匹配**的
仓库构建 → 缓存（标记必须与**版本钉**一致）→ VS Code 扩展自带 → 按版本钉锁定下载；
**解析不出期望版本就绝不 exec**，缓存**过期就拒绝运行并提示**
（`AGENTS.md` 的 Setup 一节；技能侧见 `skills/sokonanoda-teacher/SKILL.md:91-96`）。
零 cargo：`skills/sokonanoda-teacher/SKILL.md:98`。

### 6.2 三步循环（讲 → 答 → 判）

| 步 | 谁做 | 做什么 | 出处 |
|---|---|---|---|
| 1 **讲课** | 老师（agent） | 往画布追加 `--` 中文讲解 + 已填好的**演示声明** + **练习**（值位写 `sorry` 的 `def`/`theorem`）。**语法点永远先出现在讲解注释里、再出现在练习里**（语法白名单即课程） | `docs/teaching-session.md:22-23`、`SKILL.md:199-201` |
| 2 **作答** | 学习者 | 编辑画布填洞。支持**部分作答**：先写几层 `fun`、最后一层留 `sorry`，剩余目标与已引入假设会出现在 hover/诊断里 | `docs/teaching-session.md:24-25`、`SKILL.md:202-203` |
| 3 **判卷** | 老师 | 跑 `--json`，读**结构化事件**（不是 exit code）决定反馈；一个练习红了不影响其他练习（逐声明容错）。然后回到第 1 步 | `docs/teaching-session.md:26-34`、`SKILL.md:204-207` |

判卷的确切命令：

```bash
scripts/soko grade playground.sokonanoda --json   # 全量事件流（每行一个 JSON 事件）
scripts/soko grade playground.sokonanoda          # 人类可读
scripts/soko watch playground.sokonanoda          # 常驻监控，每版 delta 流
```

**判卷只认两个信号**：`decl.checked`（做出来了）与 `diagnostic`（有问题）。
`exercise.open` 只是"还是个练习"——它**只有签名合法时才会出现**；
所以判断"签名有没有腐烂"永远看 `diagnostic`，不要看 open 计数
（`skills/sokonanoda-teacher/SKILL.md:126-130`）。
`sorry`（含 `by` 块里的）是**合法开放状态**，不是错误（`SKILL.md:26`）。

### 6.3 "某处还差什么"——先问，别扫

需要"某处还差什么 / 下一个洞在哪 / 这题的提示是什么"时用 `query`（单 JSON 对象）：

```bash
scripts/soko query check --file playground.sokonanoda               # 计数 + 失败 + 告警
scripts/soko query state --file playground.sokonanoda --line 327 --col 4   # 该处的目标与假设
scripts/soko query holes --file playground.sokonanoda               # 全部洞（稳定 id）
scripts/soko query hints --file playground.sokonanoda --line 323 --col 3   # 该处的 hint 阶梯
scripts/soko query goals --file playground.sokonanoda               # 全文件声明概览
```

（`skills/sokonanoda-teacher/SKILL.md:65-74`。）DeepSeek Harness 里这七个查询还包成 MCP 工具
（`mcp__sokonanoda__{check,state,goals,holes,hints,reduce,project}`）——**有工具就直接调，别绕 shell**
（`SKILL.md:85-87`、`AGENTS.md`）。

### 6.4 判卷之后：事件决策表（老师看到的 → 老师做的）

| 事件 / code | 解读 | 动作 |
|---|---|---|
| `decl.checked`（原练习名） | 解出 | 肯定 + 追加下一个概念/练习 |
| `exercise.open` 持续 | 未做/卡住（**签名合法**才会走到这里） | 指向编辑器「提示」节点逐条揭示（画布 `-- soko:hint` 阶梯）；永不直接给答案 |
| 诊断落在**签名**上（`kernel-expected-sort` / `kernel-theorem-not-prop`） | 签名自己写坏了：`sorry` 救不回来，**不是**"还没做" | 先修签名；修好前不要给证明方向的提示 |
| `elab-unknown-identifier` | 签名里的名字拼错，**或**引用了还没解出的练习（open 声明不进环境） | 先查 open 列表，再判拼写；必要时「先做练习 N」 |
| `elab-duplicate-declaration` | 重名 | 讲「单赋值世界」，换名 |
| `elab-hole-misplaced` | 洞不在可恢复位置（嵌套洞 / 非直接实参，如 `n + sorry`） | 讲「洞只能放答案末尾，或已知函数/构造子的直接实参位」 |
| `kernel-rejected`（带期望/实际） | 填了类型而非证明项 / 方向反 / 宇宙忘了 `.{1}` / 忘了 `Not` 会展开 | 让学习者对比声明类型与所填项的形状，逐参数预言类型 |
| `warning`（`redundant-sorry`） | 答案其实写全了，那行 `sorry` 是**多接的一个实参** | 直接说「把这一行的 `sorry` 删掉就完成了」——**不要**说"还没证出来" |
| 无诊断但语义不对 | 内核只判类型不判意图（如 `double := fun n => n`） | 设计「证明形状」需求：另出一题用 `Eq` 回判该定义的值 |

（`skills/sokonanoda-teacher/SKILL.md:211-222`；完整版 `docs/teaching-session.md:36-47`。）
**诊断自带教学 `hint` 字段**——那是给学习者的第一句话，老师转述即可，不要照本宣科地念 code
（`SKILL.md:224-225`）。

### 6.5 出题规范：hint 阶梯与"答案绝不进提示"

每个练习声明前挂 **2–3 条** `-- soko:hint <text>`（独占一行，挂到紧随的声明）：

1. **思路**（练什么概念）→ 2. **目标形态**（目标怎么拆）→ 3. **关键件**（构造子/引理的名字与用法）。

> 「**答案绝不写进提示**；阶梯是给用户的自助通道（编辑器「提示」逐条揭示，
> 经 `soko/hints` 请求）」—— `skills/sokonanoda-teacher/SKILL.md:231-235`

画布里的实例（单元② 练习 4，`courses/set-theory/units/unit02-subsets-empty.sokonanoda:51-56`）：

```sokonanoda
-- 练习 4（L 类）：∅ ⊆ A（**空洞蕴含**：前提永远不成立）。
-- soko:hint 思路：目标是 ∀ x, x ∈ ∅ → A x；把前提拿进来，它其实是 False。
-- soko:hint 目标形态：intro 之后手里有 hx : Set.mem α x (Set.empty α)，目标是 A x。
-- soko:hint 关键件：hx 展开就是 False；用 False.elim (A x) hx 交出目标。
theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=
  sorry
```

**提示密度随单元递减**：前 3 个单元每题都有 hint，中段只给"关键件"，后段（9–12）只给"思路"
（`docs/design/set-theory-syllabus.md:136-137`）——这条照抄 `djvelleman/stg4` 的难度曲线
（它的 `Combo` world 原文写 "For the most part, we'll leave you on your own"，
`docs/notes/settheory-survey/prior-art-report.md:128`）。

**解答钥匙的揭示纪律**：`course/solutions/` 与 `docs/teaching-session.md` §3 有全部练习的、
经完整内核验证的钥匙；**agent 专用**，只有用户明确要求答案、或同一关卡反复卡住（≥3 轮）时
才逐层揭底，**永远不要一次性贴出完整钥匙**（`skills/sokonanoda-teacher/SKILL.md:299-307`、
硬规则第 4 条 `SKILL.md:29`）。

### 6.6 学习者在编辑器里看到什么

（`skills/sokonanoda-teacher/SKILL.md:309-338`）

- 悬停任何表达式看类型；**悬停 `sorry` 看剩余目标 + 已引入假设**；
- 洞尾 inlay 提示直接标注该洞的**期望类型**（子洞有各自的期望类型）；
- 洞上灯泡：按目标形状的下一步建议，kernel 验证过的排最前并标 preferred
  （`exact <假设>` / `Eq.refl …` / `refine <构造子骨架>` / `引入 N 个 binder`）；
- 练习树每个 open 声明有「提示」节点：**逐条揭示**画布里的 `-- soko:hint` 阶梯；
- 练习树顶部「当前光标处」跟随光标显示该位置的 tactic 目标与假设；
- CodeLens 显示每个声明的练习状态（open / solved / failed）；
- **Infoview 目标面板**在右侧辅助侧栏：goal 行以 `⊢` 开头、假设逐行 `name : ty`。

### 6.7 收尾义务

> 「每轮教学交互结束：确认画布仍能整文件编译（`--json` 无 parse 错误）、
> 把本次学到的用户适配要点记进你的工作笔记（**不是 `course/`**）」
> —— `skills/sokonanoda-teacher/SKILL.md:354-356`

---

## 7. 课程驱动开发 —— 缺口台账（`docs/gaps/ledger.jsonl`）

### 7.1 机制：写课程会撞出语言缺口，缺口必须记账

**为什么需要**（四条真实教训，逐字，`docs/design/teaching-project.md:273-281`）：

1. **绕过即遗忘**：`Exists` 用公理绕了 G-03，两年（轮次意义上）没人记账；
2. **假绿**：G-01 让「开练习」全绿，作者先据此得出了错误结论；
3. **文档与实现分叉**：G-10 —— `query/mod.rs:594` 的注释承诺「`check` 会带着 parse 诊断返回」，
   实测是全零 + `ok:true`；没有台账就没人会发现这句注释在说谎；
4. **回归无痕**：语言仓的 golden 只看计数，签名级错字不改计数 ⇒ 课程烂掉不报警。

**数据结构**（一行一条 JSON，`docs/design/teaching-project.md:283-308`）：
`id` / `title` / `kind` / `severity` / `status` / `found` / `found_by` / `where` / `repro` /
`today` / `expected_lean` / `workaround` / `blocks` / `wo` / `wo_planned` / `fixed_in`。

- `kind` 五类：`language`（语法/elaborator/kernel）/ `tooling`（CLI/LSP/VS Code）/
  `infra`（清单/CI/站点）/ **`library`（标准库欠账：Lean core / Mathlib 里现成、我们却没有）** / `doc`；
- `severity` 三档：`blocker`（不修就写不了这类内容）/ `painful`（能写但成本×N 或会教坏学生）/ `nice`；
- `status`：`open` → `wo-filed` → `fixed`（`fixed_in` 必填版本）／`workaround`（长期绕行，须写清代价）／`wontfix`；
- **复现文件必须最小且入库**（`docs/gaps/repro/<id>-*.sokonanoda`）；
- **`expected_lean` 必填**：硬规则 3（教学语法是真实 Lean 4 的子集）要求写清
  「官方 Lean 里这段是什么行为」，它就是工作单的验收判据。

**工作单（WO）** 是交给"另一个 agent"的唯一接口（`docs/design/teaching-project.md:310-323`），
模板含：用户可见症状 / 最小复现 / 今天的表现、期望行为、范围（**是否动内核**）、
**不做的事**、验收（三层）、文档同步清单、门禁。

### 7.2 可执行台账：「缺口即测试」

`docs/gaps/README.md` 的退出码约定（`.sh` 复现一律遵守）：

| 退出码 | 含义 |
|---|---|
| **0** | 缺口仍在，且与台账 `today` 字段描述一致（**正常状态**） |
| **1** | 行为变了（很可能已修复）⇒ 回来更新台账（写 `fixed_in`）并升级课程 |
| **2** | 环境/前置缺失（跑不起来，不算结果） |

`.sokonanoda` 复现没有脚本外壳，`gap.py check` 的判据是「是否干净判卷 + 有没有 checked 声明」
（**故意不看 `exercise_open` 计数**——G-01 就是这么假绿的）；有些缺口的"修好"恰恰是判红
（如 G-01 钉的是「签名写错必须被拒」），这类条目用显式 `repro_expect`
（`clean` / `rejected` / `exit0` / `nonzero`）覆盖默认推导
—— `docs/gaps/README.md`（复现脚本退出码约定一节）。

日常命令（`docs/gaps/README.md`）：

```bash
python3 scripts/gap.py list                 # 按级别 + WO 排序看全部缺口
python3 scripts/gap.py list --kind library  # 标准库欠账单独看
python3 scripts/gap.py next                 # 下一条要修的缺口 + 可直接粘贴给另一个 agent 的 prompt
python3 scripts/gap.py selftest             # 自检判定规则本身（judge()，秒级，不跑判卷）
python3 scripts/gap.py check                # 跑全部复现，报告「台账 vs 现实」偏差
python3 scripts/gap.py close G-11 --version 0.59.0   # 关账（会先复跑复现，仍复现则拒绝关）
```

`selftest` + `check` 已接进两处：`scripts/soko gate` 的第四步（课程门禁之后）与
CI `test` job 的 `Gap ledger is consistent (docs/gaps)` step
—— 台账因此是**被强制执行的契约**，而不是一份会腐烂的文档（`docs/gaps/README.md`）。

### 7.3 计数（本轮实测）

```bash
python3 -c "
import json
rows=[json.loads(l) for l in open('docs/gaps/ledger.jsonl') if l.strip()]
from collections import Counter
print('total', len(rows))
print('status', Counter(r['status'] for r in rows))
print('kind', Counter(r['kind'] for r in rows))
print('severity', Counter(r['severity'] for r in rows))
print('fixed_in', Counter(r.get('fixed_in') for r in rows))
"
```

输出（逐字）：

```
total 24
status Counter({'fixed': 22, 'workaround': 2})
kind Counter({'language': 10, 'tooling': 7, 'library': 6, 'infra': 1})
severity Counter({'blocker': 12, 'painful': 10, 'nice': 2})
fixed_in Counter({'0.59.0': 18, '0.60.0': 4, None: 2})
```

**一句话**：**24 条缺口，22 条已修（0.59.0 修 18 条、0.60.0 修 4 条），2 条是长期绕行**
（L-04 课程标准库缺"定义展开"引理的那批——已用 `lib/Set` 落地为绕行；
L-06 语言没有累积性且 `Exists.elim` 的 `Q` 只能是 `Prop`）。

### 7.4 全部 24 条（`docs/gaps/ledger.jsonl`，按文件顺序）

| id | kind | severity | status | fixed_in | 标题（逐字） |
|---|---|---|---|---|---|
| G-14 | language | nice | fixed | 0.59.0 | 宇宙层级 binder 只吃一个花括号组：`{u v}`（空格）与 `{u} {v}`（两组）都解析失败 |
| G-15 | tooling | painful | fixed | 0.59.0 | `query check` 的 `failed[]` / `warnings[]` 只给裸字节 offset——没有行列、没有单位 |
| L-01 | library | blocker | fixed | 0.59.0 | prelude 缺 Lean core 的逻辑骨架（`True`/`False`/`And.elim`/`Or.elim`/`Not`/`absurd`/`Iff`） |
| L-02 | library | blocker | fixed | 0.59.0 | prelude 缺 Eq 的核心引理（`Eq.symm` / `Eq.trans` / `congrArg`） |
| L-03 | library | painful | fixed | 0.60.0 | prelude 的 `Eq.subst` 只支持 Prop motive ⇒ Type 层重写不可表达（`Eq.mp`/`Eq.mpr`/`Eq.rec`/`cast`） |
| L-04 | library | blocker | **workaround** | — | 课程标准库缺 Set 的「定义展开」引理（`subset_def`/`mem_union`/`mem_inter`/`mem_diff`/`mem_power_iff`/`not_mem_empty`/`mem_singleton_iff`） |
| L-05 | library | painful | fixed | 0.59.0 | 「内容型」集合引理被误放进库（16 条）——它们应当是**练习**而不是基础设施 |
| G-13 | language | painful | fixed | 0.59.0 | `axiom` 不吃 binder 参数表（`def`/`theorem` 吃），照 Lean 4 习惯写会被拒 |
| G-12 | language | blocker | fixed | 0.59.0 | 相对路径入口 + 祖先清单 ⇒ 模块根退化成空路径 ⇒ 所有 `import` 报找不到 |
| G-11 | tooling | blocker | fixed | 0.59.0 | 启动器在非 Rust 仓库（课程仓）没有版本源，拒绝运行且只报裸 ENOENT |
| G-10 | tooling | blocker | fixed | 0.59.0 | `query check` 对解析失败的源文本返回全零 + `ok:true` + 退出码 0 |
| G-01 | language | blocker | fixed | 0.59.0 | 开练习的签名不做类型检查（签名写错与「还没做」无法区分） |
| G-02 | language | blocker | fixed | 0.59.0 | 构造子没有命名空间，且构造子名全局唯一 |
| G-03 | language | blocker | fixed | 0.59.0 | Prop 结果 + Type 参数 + 单构造子 + 自有字段的 inductive 被内核断言拒绝 |
| G-04 | language | blocker | fixed | 0.59.0 | 没有 notation / infix（用户自定义记法） |
| G-05 | language | painful | fixed | 0.60.0 | 没有 `namespace` / `open` |
| G-06 | tooling | blocker | fixed | 0.59.0 | `sokonanoda course` 只按单文件编译，不认 `import` |
| G-07 | infra | painful | fixed | 0.60.0 | 课程清单格式扁平（无卷/章/先修/标签/练习配额） |
| G-08 | language | nice | fixed | 0.60.0 | 没有 `abbrev`（可展开的类型别名） |
| G-09 | tooling | painful | fixed | 0.59.0 | 内核断言以裸文本外泄（`left: 1` / `right: 0`），hint 是通用「类型不匹配」 |
| G-16 | tooling | blocker | fixed | 0.59.0 | 启动器在「版本未知」时会 exec 缓存里的陈旧二进制（绕过「过期即拒绝」守卫） |
| G-17 | tooling | painful | fixed | 0.59.0 | `query goals` / `holes` 对解析失败也假绿（空数组 + `ok:true`） |
| G-18 | language | painful | fixed | 0.59.0 | `def f.{u}` 被静默解析成名字 `f.`（声明消失且不报错） |
| L-06 | library | painful | **workaround** | — | 语言没有累积性（`def T : Type := <Prop 值>` 被拒）且 `Exists.elim` 的 `Q` 只能是 `Prop` ⇒ 取数据的引理写不出来 |

### 7.5 卷 I 自己撞到的那一批

> 「本卷目前撞到的（已在权威台账里）：**G-01**（开练习签名不校验）、**G-02**（构造子无命名空间）、
> **G-03**（`Exists` 形状的归纳被拒）、**G-04**（无 notation）、**G-12**（相对路径模块根）、
> **G-13**（`axiom` 不吃 binder）、**G-14**（单宇宙 binder）、**G-15**（内核错误 span）、
> **L-01…L-05**（标准库欠账与分层教训）。」
> —— `courses/set-theory/gaps/README.md:13-15`

也就是说：**卷 I 一门课撞出了台账里 24 条中的 13 条**（G-01/02/03/04/12/13/14/15 八条语言与工具缺口
+ L-01…L-05 五条标准库欠账）。发现端的原始材料（一条缺口一个文件 + 最小复现）
留在 `courses/set-theory/gaps/`，权威状态在 `docs/gaps/ledger.jsonl`
（`courses/set-theory/gaps/README.md:3-4`）。

### 7.6 两条"绕行"的真实代价（写进网站时要如实说）

- **L-04**：`lib/Set.sokonanoda` 用 10 条展开引理把这一层补上了（`courses/set-theory/lib/Set.sokonanoda:20-22`），
  代价是名字必须逐个对齐 Loogle 取证版（`Set.notMem_empty` 大写 M 这类反直觉拼写）。
- **L-06**：直接改写了单元⑦ 的一道题的**陈述形状**——`bijective_iff_inverse` 只能是数据版
  （逆函数与"它是右逆"的证据由调用者交出来，作为前提而不是结论），
  因为「满射 ⇒ 存在右逆函数」要取数据、等于选择公理
  （`docs/design/course-stdlib.md:235-255`、`docs/design/set-theory-syllabus.md:164` 的 7′ 边界行）。
  这是"语言缺口改变教学内容"的最清晰一例。

---

## 8. 学习障碍的实证结论（`docs/notes/settheory-survey/learning-difficulties.md`）

> **文件规模**：2873 行 / ~190 条来源。**核验级别**（`learning-difficulties.md:66-77`）：
> **[F]** = 全文下载并读过，结论逐字引自正文；
> **[A]** = 从出版方/仓储/API 记录取到逐字摘要；
> **[M]** = 只有书目元数据（标题/作者/年份/卷期/页码/DOI 核实过），**结论未核实**；
> **[R]** = DOI 解析独立核对过。
> 注意：Parts A–C 的部分条目用散文写核验程度（如 "Verified level: FULLTEXT (OCR of the
> author's scan)"），没有统一打方括号；下面的表**照它自己的写法**标注，不做升级。

### 8.1 最影响教学决策的七条

| # | 结论 | 数字 / 逐字 | 级别 | 落到哪个单元 |
|---:|---|---|---|---|
| 1 | `∈` 与 `⊆`、元素与单元素集 | 183 名学生里 `r ∈ M` / `s ∉ M` 几乎全对，但 **`{r} ∈ M` 无一人给出正确理由**；作者把配套任务归为"分不清 ∈ 与 =" | **[F]**（免费 PDF `EJ1428069.pdf`，结论逐字引）`learning-difficulties.md:89-94` | 单元① 同时给两种写法；单元② 单元素集 |
| 2 | `∅` 不被当成集合 | 五个"这是集合吗"的对象里两个是 `∅`：「**Many of the students did not accept this as a set. The reason is all the same: no element in it can be identified.**」 | **[F]**（同上，逐字）`learning-difficulties.md:103-107` | 单元② 显式给"∅ 是一个集合"的定位 |
| 3 | 空真 / 假前件 | Dubinsky (1997)：七道做得差的题里 **五道涉及不属于量化部分的蕴涵**；某题「除一人外所有学生都正确否定了量化部分，错误**完全**来自对蕴涵的否定」 | **[F]**（全文读过；`learning-difficulties.md:2650`）`learning-difficulties.md:117-122` | 单元② `empty_subset` |
| 4 | 函数本体论 | Breidenbach et al. (1992)：本该 ~100% 判为"函数"的场合只有 **约 40%**；按表示分：ISETL 函数 74.6–76.5%、元组 54.1–61.6%、**图 19–40.7%、方程 25–31%**、表 39.9–48.6%、物理情境 30.2–35.8%；学生"坚持要有表达式、或至少要有变量来标示输入输出"，"很多情况下……坚持要有因果性" | **FULLTEXT（作者存档扫描件的 OCR）**；⚠️ 原文自己警告「treat the exact decimals as ±OCR」`learning-difficulties.md:143-153`、`:1062-1079` | **单元⑦ 的导入语直接讲这个数字** |
| 5 | 量词顺序 `∀∃` vs `∃∀` | Dubinsky & Yiparaki (2000)，63 名学生 / 11 个陈述：**94% 的学生至少把一个 EA 陈述读成 AE**（逐陈述 11%–81%），反向只有 **5%**（0%–3%）；两条**数学**陈述只有 41% / 9% 答对 | **[F]**（全文读过）——⚠️ **未发表手稿**，引用必须注明状态（`learning-difficulties.md:2703`、更正 #14 `:383-386`） | 单元⑦ 的 D 类题 |
| 6 | 可数 / 对角线 | Hamza & O'Shea (2011)：35 名学生（含转行数学教师）出现五族误解；**援引双射判据的人没有一个在所有题上都用它，且"极少真的写出一个具体的映射"**；"countable"被读成"能物理数出来"（于是 有限=可数、无限=不可数） | **全文读过**（PDF 是纯图像，本轮 250 dpi 栅格化 + tesseract OCR，11 页 29,287 字符全文复核）`learning-difficulties.md:1547-1575` | 单元⑨/⑩ **强制"亲手写出那个映射"** |
| 7 | 过渡到证明（总论） | Moore (1994) 三大困难源 **(a) 概念理解 (b) 数学语言与记号 (c) 不知如何下笔**；Selden & Selden (1995) 拆解成功率 **8.5% / 5%**；Weber (2001)「有句法知识却用不上」；Inglis & Alcock (2012) 眼动证实新手看表面特征 | **[A][R]**（Moore `:2337`、Weber `:2363`、Selden & Selden `:2382`、Inglis & Alcock `:2418`） | **(b)(c) 是本课程最大的可控变量**（`docs/design/set-theory-syllabus.md:80`） |

另外两条与本课程直接相关：

- **证明验证能力**：Selden & Selden (2003)，8 名数学专业学生，「倾向关注**表面特征**」，
  验证能力「**非常有限——也许比他们自己和老师以为的还要有限**」——**[A][R]**
  （`learning-difficulties.md:2401`、`:270-272`）。→ 单元⑫ 把"读"单独设成一个单元。
- **接受对角线法**：Zazkis & Mamolo (2009)「Sean vs. Cantor」——一名硕士生的 ℝ 假枚举
  顶住了多次反驳；后来的一个班在**学过 Cantor 定理之后**面对同一个假枚举，
  反应是「Cool!」并点头同意，**没有察觉矛盾**——全文 PDF 取到
  （`learning-difficulties.md:217-221`）。→ 单元⑩ 先放假枚举让学习者自己找矛盾。

### 8.2 两个否定结果（**这是实测的否定，不是"没查到"**）

#### 否定结果 ①：有序对的集合编码（Kuratowski）**没有实证研究**

> 「**Documented negative result: no empirical study of the Kuratowski encoding, or of
> students proving `(a,b)=(c,d) ↔ a=c ∧ b=d`, could be found in any reachable source.**
> ERIC's `"Kuratowski ordered pair"` returns **0** results; its `"ordered pair"` hits are
> school graphing activities. The encoding-blocker claim is therefore currently
> **instructor experience, not published evidence**, and the survey should say so.」
> —— `docs/notes/settheory-survey/learning-difficulties.md:124-129`

在 §2 的更正清单里被再次点名（第 13 条）：

> 「**Blocker 3 has no empirical literature** on the encoding itself.」—— `learning-difficulties.md:382`

**课程后果**：单元⑤ 把 Kuratowski 编码**降为阅读题**，并明确"**不要写成'研究表明…'**"
（`docs/design/set-theory-syllabus.md:82`、`:158` 的行内警示）。

#### 否定结果 ②：选择公理（AC）的学生/教师态度与 AC 教学 **没有实证研究**

> 「**There is, as far as I can establish, no empirical study of students' or instructors'
> attitudes toward the Axiom of Choice, and no study of teaching AC, in the indexed
> mathematics-education literature.** I am reporting this as a positive finding backed by
> an exhaustive negative search, not as a failure to look.」
> —— `docs/notes/settheory-survey/learning-difficulties.md:1952-1955`

它把"搜索清单"逐条列了出来（`:1957-1972`），其中可核对的空结果包括：

| 查询 | 结果 |
|---|---|
| ERIC `"axiom of choice"`（加引号，全年代、全类型） | **1 条记录**，且不相关（员工奖励偏好论文，命中 "choice"） |
| ERIC `"axiom of choice" teaching` | **0 条** |
| OpenAlex `title_and_abstract.search` × 10 种措辞 | **没有教育研究**；其中 5 条返回**零**结果 |
| OpenAlex `fulltext.search:axiom of choice teaching` | 前 10 全是通用教学文献，**全文索引里没有 AC 教育论文** |
| Crossref × 5 | 只有标准专著章节（Herrlich / Jech / Pruss）与 Banach–Tarski 章节，**没有教育文章** |
| arXiv `cat:math.HO AND all:"axiom of choice"` | 7 条，**没有数学教育研究** |

结论句逐字：

> 「**Conclusion for the survey: any claim of the form "studies show students/instructors
> think X about AC" is unsupported.**」—— `learning-difficulties.md:1974-1975`

在 §2 更正清单里被列为第 12 条：「**Blocker 8 is empty and that is the finding.**」（`:380-381`）

**唯一找到的 AC 教学主张**来自 **Förster (2006)**，而它的来源类型是
**preprint / unpublished manuscript（无 DOI、无 venue，全文取不到）**；
课程只把它的观察当**导语**用，并明确标注"**这是作者观点，不是实证**"：

> 「教学惯于略过 AC 的应用……学生因此没有形成它的心智图像，日后也认不出何时被使用」
> —— 转引自 `docs/design/set-theory-syllabus.md:83`；原始条目 `learning-difficulties.md:1983-1984`

**课程后果**：单元⑩ 末尾用 Förster 的观察当导语、标注为作者观点；
`docs/design/set-theory-syllabus.md:87-88` 的引用纪律第 1 条把这条写成硬规则：
「**有序对（障碍 3）与选择公理（障碍 8）不存在实证研究**——这是**实测的否定结果**，
不是"没查到"。任何交付物不得写"研究表明学生对 AC 持 X 看法"。」

### 8.3 另外三处"有用但很小"的空结果（Blocker 7）

`learning-difficulties.md:1912-1922`（§7.3 "negative / useful-null results"）：

- ERIC `"infinite sets" cardinality students` 总共只返回 **4 条**记录；
- ERIC `"diagonalization" proof mathematics students` 返回 **1 条**（且离题）；
- ERIC `"intuitive thinking about infinity"` 与 `"Misconceptions Concerning Infinity"`
  （加引号）返回 **0 条**——**Hamza & O'Shea (2011) 没有被 ERIC 收录**，
  它只存在于 Maynooth 仓储；「Do not claim ERIC coverage for it.」

### 8.4 两条"文献最薄"的坦白（写网站时不要吹）

- **像与原像（Blocker 6）**：「**This is the thinnest blocker in the literature and should be
  reported as such.** There is no widely cited paper on students' understanding of image and
  preimage, and none measuring the `f(A∩B) ⊆ f(A)∩f(B)`-not-conversely failure.」
  —— `learning-difficulties.md:189-192`
- **APOS / Dubinsky 的实证基础压倒性地是微积分、线代、抽代，不是集合论**：
  「A survey claiming "APOS research shows students struggle with set theory" would be
  overclaiming.」—— `learning-difficulties.md:317-324`；
  引用纪律把它写成第 2 条（`docs/design/set-theory-syllabus.md:89-91`）。

### 8.5 引用纪律（课程文档必须遵守的五条，逐字）

`docs/design/set-theory-syllabus.md:85-96`：

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

---

## 9. 可展示的课程素材 —— 6 段最有吸引力的练习

> 选取口径：**题面短、惊喜强、背后有一条实证或一条真实勘误**。
> 每段给两半：**画布**（学习者看到的，`sorry` 是洞）与**解答**（`units/solutions/`，agent 专用）。
> 代码逐字引用，行号是 `file:line`。

### 素材 1 · `empty_subset` —— 最短的一道题，也是最反直觉的一道

**画布** `courses/set-theory/units/unit02-subsets-empty.sokonanoda:51-56`

```sokonanoda
-- 练习 4（L 类）：∅ ⊆ A（**空洞蕴含**：前提永远不成立）。
-- soko:hint 思路：目标是 ∀ x, x ∈ ∅ → A x；把前提拿进来，它其实是 False。
-- soko:hint 目标形态：intro 之后手里有 hx : Set.mem α x (Set.empty α)，目标是 A x。
-- soko:hint 关键件：hx 展开就是 False；用 False.elim (A x) hx 交出目标。
theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=
  sorry
```

**解答** `courses/set-theory/units/solutions/unit02-solution.sokonanoda:18-19`

```sokonanoda
theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=
  fun (x : α) => fun (hx : Set.mem α x (Set.empty α)) => False.elim (A x) hx
```

**为什么吸引人**：整个证明只有一层 `fun`——因为那个前提**永远拿不到**。
它背后是一条实证：很多学生根本不接受 `∅` 是集合，理由全都一样，"里面没有元素可以指认"
（`docs/notes/settheory-survey/learning-difficulties.md:103-107`，`[F]`）。

### 素材 2 · `powerset_self_mem` —— 附赠一条真实勘误

**画布** `courses/set-theory/units/unit03-union-inter-powerset.sokonanoda:179-181`

```sokonanoda
theorem powerset_self_mem (α : Type) (A : Set α) :
    Set.mem (Set α) A (Set.powerset α A) :=
  sorry
```

**解答** `courses/set-theory/units/solutions/unit03-solution.sokonanoda:84-88`

```sokonanoda
-- 命题甲为真：A ∈ 𝒫 A（展开即 A ⊆ A）；命题乙 A ⊆ 𝒫 A 连类型都不对（见画布注释）。
theorem powerset_self_mem (α : Type) (A : Set α) :
    Set.mem (Set α) A (Set.powerset α A) :=
  Iff.mpr (Set.mem (Set α) A (Set.powerset α A)) (Set.subset α A A)
    (Set.mem_powerset_iff α A A)
    (fun (x : α) => fun (hx : A x) => hx)
```

**为什么吸引人**：大纲原稿把 `A ∈ 𝒫 A` 写成了**假命题**，实测它是真的——
勘误留在画布注释里（`unit03-union-inter-powerset.sokonanoda:172-176`：
「大纲单元③把"A ∈ 𝒫A"列为假命题：那是笔误，随本题解答一并更正」），
并在大纲的实测偏差表里留了记录（`docs/design/set-theory-syllabus.md:175`）。
**课程把自己的错误留在原地当教学材料**，这本身就是可展示的一点。

### 素材 3 · `not_symm_trans_implies_refl` —— "证明或证伪"第一次咬人

**画布** `courses/set-theory/units/unit06-relations.sokonanoda:213-216`

```sokonanoda
theorem not_symm_trans_implies_refl :
    Not (forall (A : Type) (r : Rel A A),
      Symmetric A r -> Transitive A r -> Reflexive A r) :=
  sorry
```

**解答** `courses/set-theory/units/solutions/unit06-solution.sokonanoda:365-371`

```sokonanoda
theorem not_symm_trans_implies_refl :
    Not (forall (A : Type) (r : Rel A A),
      Symmetric A r -> Transitive A r -> Reflexive A r) :=
  fun (h : forall (A : Type) (r : Rel A A),
      Symmetric A r -> Transitive A r -> Reflexive A r) =>
    h Nat (EmptyRelation Nat)
      (demo_empty_symmetric Nat) (demo_empty_transitive Nat) 0
```

**为什么吸引人**：反例是**非空论域上的空关系**——两条前提**空洞成立**，自反却要求真有元素。
这一题把"空洞"从一个抽象说法变成一个能亲手交出来的项。
题型（D 类"证明或证伪"）来自 Macbeth 的 *The Mechanics of Proof*
（`docs/design/set-theory-syllabus.md:62`）。

### 素材 4 · `swap_converse_false` —— 全卷实证最硬的一道题

**画布** `courses/set-theory/units/unit07-functions.sokonanoda:198-201`

```sokonanoda
theorem swap_converse_false :
    Not (forall (P : Set Nat -> Set Nat -> Prop),
      (forall (x : Set Nat), Exists (Set Nat) (fun (y : Set Nat) => P x y)) ->
      Exists (Set Nat) (fun (y : Set Nat) => forall (x : Set Nat), P x y)) :=
  sorry
```

**解答** `courses/set-theory/units/solutions/unit07-solution.sokonanoda:104-126`

```sokonanoda
theorem swap_converse_false :
    Not (forall (P : Set Nat -> Set Nat -> Prop),
      (forall (x : Set Nat), Exists (Set Nat) (fun (y : Set Nat) => P x y)) ->
      Exists (Set Nat) (fun (y : Set Nat) => forall (x : Set Nat), P x y)) :=
  fun (h : forall (P : Set Nat -> Set Nat -> Prop),
      (forall (x : Set Nat), Exists (Set Nat) (fun (y : Set Nat) => P x y)) ->
      Exists (Set Nat) (fun (y : Set Nat) => forall (x : Set Nat), P x y)) =>
    (fun (bad : Exists (Set Nat) (fun (y : Set Nat) => forall (x : Set Nat), Eq.{1} (Set Nat) x y)) =>
      Exists.elim (Set Nat) (fun (y : Set Nat) => forall (x : Set Nat), Eq.{1} (Set Nat) x y)
        False bad
        (fun (y : Set Nat) => fun (hy : forall (x : Set Nat), Eq.{1} (Set Nat) x y) =>
          Set.notMem_empty Nat 0
            (Eq.subst.{1} (Set Nat)
              (fun (X : Set Nat) => Set.mem Nat 0 X)
              (Set.univ Nat) (Set.empty Nat)
              (Eq.trans.{1} (Set Nat) (Set.univ Nat) y (Set.empty Nat)
                (hy (Set.univ Nat))
                (Eq.symm.{1} (Set Nat) (Set.empty Nat) y (hy (Set.empty Nat))))
              (Set.mem_univ Nat 0))))
      (h (fun (x : Set Nat) => fun (y : Set Nat) => Eq.{1} (Set Nat) x y)
        (fun (x : Set Nat) =>
          Exists.intro (Set Nat) (fun (y : Set Nat) => Eq.{1} (Set Nat) x y) x
            (Eq.refl.{1} (Set Nat) x)))
```

**为什么吸引人**：`∀x∃y` 与 `∃y∀x` 的差别，在一个具体的集合反例上被压成"`∅ = univ` 不可能"。
背后是全文里最刺眼的数字：**94% 的学生至少把一个 EA 陈述读成 AE，反向只有 5%**
（Dubinsky & Yiparaki 2000，63 名学生 / 11 个陈述；⚠️ 未发表手稿
—— `learning-difficulties.md:162-170`、`:2703`）。
这道题让学习者**亲手把换序不可能这件事证出来**，而不是被口头告知。

### 素材 5 · `powerset_nat_not_countable` —— 一行的解答，一个世纪的结果

**画布** `courses/set-theory/units/unit10-cantor.sokonanoda:140-141`

```sokonanoda
theorem powerset_nat_not_countable :
    Not (Set.Equiv Nat (Set Nat) (Set.univ Nat) (Set.univ (Set Nat))) :=
  sorry
```

**解答** `courses/set-theory/units/solutions/unit10-solution.sokonanoda:92-94`

```sokonanoda
theorem powerset_nat_not_countable :
    Not (Set.Equiv Nat (Set Nat) (Set.univ Nat) (Set.univ (Set Nat))) :=
  cantor Nat
```

**为什么吸引人**：解答只有 `cantor Nat` 四个词——但那个 `cantor`（画布 `:127`，解答 `:44`）
是本课程里最长的一道题，它的对角线集合是 `D := fun (x : α) => Not (f x x)`
（画布 `:118` `diag_mem_iff` 是它的成员判定）。
这一单元的顺序是照着实证设计的：**先给一个假的枚举让学习者自己找矛盾**
（画布 `:46` `fake_enum` / `:48` `demo_fake_enum_escapes`），
因为 Zazkis & Mamolo (2009) 发现学过 Cantor 定理的学生面对假的 ℝ 枚举只会说 "Cool!"
（`learning-difficulties.md:217-221`）。

### 素材 6 · `no_univ_strictly_larger` —— Russell 论证的那一步

**画布** `courses/set-theory/units/unit11-universe-russell.sokonanoda:122-125`

```sokonanoda
theorem no_univ_strictly_larger (α : Type) :
    Not (Exists (Set α) (fun (U : Set α) =>
      forall (A : Set α), And (Set.subset α A U) (Not (Eq.{1} (Set α) A U)))) :=
  sorry
```

**解答** `courses/set-theory/units/solutions/unit11-solution.sokonanoda:21-32`

```sokonanoda
theorem no_univ_strictly_larger (α : Type) :
    Not (Exists (Set α) (fun (U : Set α) =>
      forall (A : Set α), And (Set.subset α A U) (Not (Eq.{1} (Set α) A U)))) :=
  fun (h : Exists (Set α) (fun (U : Set α) =>
      forall (A : Set α), And (Set.subset α A U) (Not (Eq.{1} (Set α) A U)))) =>
    Exists.elim (Set α)
      (fun (U : Set α) =>
        forall (A : Set α), And (Set.subset α A U) (Not (Eq.{1} (Set α) A U)))
      False h
      (fun (U : Set α) =>
        fun (hU : forall (A : Set α),
            And (Set.subset α A U) (Not (Eq.{1} (Set α) A U))) =>
          And.right (Set.subset α U U) (Not (Eq.{1} (Set α) U U)) (hU U)
            (Eq.refl.{1} (Set α) U))
```

**为什么吸引人**：证明的关键一击是 `hU U`——**把候选者自己代进去**，
于是"严格大于每个集合"要求 `U ≠ U`，撞上 `Eq.refl`。这正是 Russell 论证的形状。
画布注释把三层读法逐字列了出来（`unit11-universe-russell.sokonanoda:110-114`）：
跨类型读法**写不出来**、类型内读法**真**（`Set.univ α`）、
类型内加强成"严格大于"**假**（就是本题）。

### 三段备选（同样可直接上网站）

| 素材 | 画布 | 解答 | 看点 |
|---|---|---|---|
| `not_image_inter_eq_image_inter` | `unit08-images-preimages.sokonanoda:295-301` | `solutions/unit08-solution.sokonanoda:285-...` | 证明"像不保交"：反例是两个点都送到同一点；文献里**最薄的一环**（`learning-difficulties.md:189-192`） |
| `fix_image_preimage` | `unit12-synthesis.sokonanoda:251-254` | `solutions/unit12-solution.sokonanoda:85-...` | 给**错证明**找错：把"反向显然"里偷用的前提 `hC` 摆到台面上（R 类题） |
| `cantor` | `unit10-cantor.sokonanoda:127-129` | `solutions/unit10-solution.sokonanoda:44-...` | 全卷最长的一道题；对角线集合 `D := fun x => Not (f x x)` |

---

## 附：引用的一级来源与命令索引

**计数命令**都写在用到它的那一节，不在这里重复：
课程门禁与单元/练习计数 → §2.1；`lib/` 声明数 → §4.2；缺口台账统计 → §7.3。
只有记法统计的命令只出现一次，补在这里：

```bash
# 画布代码行里用到的记法（排除注释行）
for tok in '∈' '⊆' '∅' '∪' '∩' '𝒫' 'ᶜ' '×ˢ' "''" "⁻¹'"; do
  echo -n "$tok: "
  grep -vE '^[[:space:]]*--' courses/set-theory/units/unitNN-*.sokonanoda | grep -cF -- "$tok"
done
```

**本文引用的一级来源**（全部在本仓库内）：
`AGENTS.md` ·
`courses/set-theory/README.md` · `courses/set-theory/AGENTS.md` ·
`courses/set-theory/course.json` · `courses/set-theory/gaps/README.md` ·
`courses/set-theory/lib/*.sokonanoda` · `courses/set-theory/units/*.sokonanoda` ·
`courses/set-theory/units/solutions/*.sokonanoda` ·
`course/README.md` · `course/course.json` ·
`docs/design/set-theory-syllabus.md` · `docs/design/course-stdlib.md` ·
`docs/design/teaching-project.md` · `docs/gaps/README.md` · `docs/gaps/ledger.jsonl` ·
`docs/teaching-session.md` · `skills/sokonanoda-teacher/SKILL.md` ·
`docs/notes/settheory-survey/learning-difficulties.md` ·
`docs/notes/settheory-survey/prior-art-report.md`
