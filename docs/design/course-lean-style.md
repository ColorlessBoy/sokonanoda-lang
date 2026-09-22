# 设计：全课程 Lean 4 化（记法符号 + tactic 证明）

> 日期：2026-09-19。触发（用户原话）：「我希望整个 courses 转向像 lean4 一样的
> 可读性一点，And Or Iff Forall Exists 等等连接符用常规符号替换现在的 function
> call 形式，增加可读性。证明过程都用 tactic 过程，例题也用 by sorry。」
> 用户追加：「涉及比较多的内容，可能反过来对 sokonanoda-lang 功能本身有一定需求，
> 你先全面调研并且计划好，保证结合计划和 subagents，不会遗漏什么东西。」
>
> 本文是**计划 + 设计**（设计先行纪律：`AGENTS.md` 收尾义务）。实施时把每一轮的
> as-built 追加到本文 §9，不要另开文件。
> 调研底稿（subagent 产出，与本文同轮）：`docs/notes/course-lean-style/`。

## 摘要（30 秒）

**要做什么**：把卷 I（35 文件 / 6550 行 / 424 声明）+ 入门课（4337 行 CN+EN）+ playground
从「点名形式 + 项模式证明」改写成「数学符号 + tactic 证明」。

**两件改写都是从零开始**（不是迁移）：全课程 **0 个 `by` 块**、**0 处**符号连接词。

**必须先改语言**（用户预判对了）。三个实测阻塞项（**本计划已把修法实测验证过一遍**，见 §1.3b）：
1. **记法 × tactic 根本不兼容**——goal 里出现 `∧` 或 `↔` 就让 `apply`/`match` 报「目标不匹配」；
   `¬`/`↔` 在 `by` 块里直接 `unknown identifier`。**根因已定位到行**（`by.rs` 的源 AST vs
   内核 pp；`judge.rs:1007` 的 `wrap_binders` 丢了记法表），都是前端 bug，**内核零改动**。
2. **tactic 白名单只有 7 个**（`intro`/`exact`/`apply`/`assumption`/`rfl`/`match`/`sorry`）
   ⇒ 课程 **102–107 条**证明转不干净。要补 `cases`/`constructor`/`left`·`right`/`use`/
   `obtain`/`have`/`exfalso`/窄版 `rw`/`·`。
3. **`→` 不存在**（2 行词法别名可修）、**`=` 不存在**（要先动词法 + 宇宙层，有风险）、
   **`≠` 缺 `Ne`**（2 行）、**`⟨a,b⟩` 不存在**（150–300 行）。

**分四轮 + 一个追加片**：R1 语言地基（记法打通 + `apply` 两 bug + 最小 tactic 集 + 试点 1 单元）→
R2 引擎扩展 + 卷 I 全量 → **R2.5（D5/D6 追加）记法可输入性 + 隐式实参** → R3 入门课
（**含「删自建骨架」**——S6 发现它其实是冗余的，删掉 prelude 的真归纳就回来了）+ 收尾 →
R4（可选）显示期 print-back。

**追加的两条要求（D5/D6）改变了什么**：改写不只是「换成符号」，还得**敲得出来、看得懂怎么敲**；
而且一旦满屏 `∈`/`⊆`，`Set.mem a A` 这种**必须写全类型参数**的点名形式就成了新的可读性瓶颈
（课程 2784 处点名调用里 **2308 处（83%）带前导类型实参**）⇒ 隐式实参从「不做」变成「必须做」。
两条各有独立设计文档，且都**不动内核**。

**关键数字不变**：只改写法的话 **424 声明 / 329 checked / 99 open 不变**（`:= by sorry`
与 `:= sorry` 事件流逐字节相同）；行数会涨。**点名调用的实测口径**（2026-09-19 复算）：
**2768 处**（底稿写 2784，±2%）、其中带前导类型实参的比例 **83%**、hint/叙事里 712 处
（底稿写 765）、涉及 **35** 个文件（底稿写 30）——明细与错在哪见
`implicit-arguments.md` §5。

**最危险的三个坑**：解析错误**全文件连坐**（一处未知 tactic ⇒ 整份 `decl_checked=0`）·
`example` **不计入 `checked`**（解答写成 `example` ⇒ G3 红；具名练习写成 `example` ⇒
G4 按名覆盖**静默失效**）· 站点数字会**静默变旧**（`gen-site-data.py` 离线时沿用旧计数，
而 `check-site.py` 从不校验计数）。

---

## 0. 用户拍板的六条（2026-09-19，本计划的输入）

> D5/D6 是同日**追加**的两条要求（原话见下表），各有一份独立设计：
> `docs/design/notation-input.md`、`docs/design/implicit-arguments.md`。
> 它们**不改变 R1/R2 内部的工作项与顺序**，但**确实新增了一个片**：R2 之后、R3 之前
> 插入 **R2.5（记法可输入性 + 隐式实参）**，见 §5。两份子设计用自己的期号前缀
> （`NI-0…NI-3`、`IA-0…IA-4`），与 R2.5 的 `P0/P1/P2/P3` 的对应关系写在 §5。

| # | 问题 | 用户选择 |
|---|---|---|
| D1 | 范围 | **卷 I `courses/set-theory/` + 入门课 `course/` + `playground.sokonanoda`** |
| D2 | 语言改造深度 | **中等**：核心符号进语言层（`∧ ∨ ↔ ¬ → = ≠ ∀ ∃` 开箱可用）+ 课程实际需要的 tactic + `⟨⟩`；~~**不做**隐式实参~~（**D6 已翻案 ⇒ P1**）/ `h.1` 投影 / print-back（除非 §7 的 spike 判定低风险） |
| D3 | 例题/演示 | **演示保持已证**，但改写成 tactic 风格；**练习占位一律写成 `:= by sorry`** |
| D4 | 集合论符号 | **全部替换**（`∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹' ×ˢ`），课程彻底 Lean 化；大纲 §4「先点名后记法」改为「记法为主，点名作为底层解释」 |
| D5 | 记法**怎么敲**（追加） | 用户原话：「要考虑 notation 如何输入，应该像 lean4 一样 'xxx' 替换，同时 hover 内容提示用户如何输入对应符号」。**定案**：客户端**缩写改写器**（`\and`→`∧`，与 Lean 逐字同一张表）+ **hover 里显示输入法**；**不为缩写新增 LSP 补全 provider**（口径：LSP **已经有** `textDocument/completion`，只是 DSH / opencode 都不消费补全项，而 DSH 的 `lsp` 工具**消费 hover**）。设计：`docs/design/notation-input.md`；**NI-0 + NI-1 已落**（见 §9） |
| D6 | 隐式实参（追加） | 用户原话：「如果想支持 notation，感觉 '{x : Set}' 这种隐参数自动推导的机制不得不实现了」。**定案**：走**路线 C**（风格对齐 + 唯一确定，**不引入元变量**）——内核 `Expr`/`Value` **没有元变量槽**，且 `elab_expr` 在内核环境外运行 ⇒ probe 路径等于整段前缀重编译。设计：`docs/design/implicit-arguments.md`；**这是 D2「不做」的正式翻案**（见 N-1） |

---

## 1. 现状实测（**先看这一节**：所有结论都是真二进制跑出来的，不是读代码猜的）

命令口径：`scripts/soko grade <绝对路径> --json`（G-12：一律绝对路径）。
探针文件在 `target/scratch/`（未入库）。

### 1.1 规模与基线

| 项 | 数值 | 来源 |
|---|---|---|
| 卷 I 文件 | 35 个 `.sokonanoda`、6550 行 | `wc -l` |
| 卷 I 顶层声明 | **424**（246 条已证 theorem/example + 99 open + 其余 def/axiom/inductive） | 清单 subagent |
| 卷 I 里的 `by` 块 | **0**（`grep -rn ':= by' courses/` = 0） | 实测 |
| 入门课 | 4337 行、`:= by` **36** 处（`course/unit4-by-tactics` 是 tactic 教学单元） | 实测 |
| `playground.sokonanoda` | 362 行、`:= by` 2 处 | 实测 |
| `-- soko:hint` 提示行 | **294** 条（13 个画布文件） | `grep -c` |
| 课程门禁基线 | `36 目标 · 329 checked · 99 open · 0 判负`，exit 0，12.2s | `python3 courses/set-theory/tools/check.py --json` |

**结论 1**：这不是「term → tactic 迁移」，是「**从零引入 tactic**」。245–246 条已证声明
里 **A 类 74–79 可机械改写**、**B 类 64 `apply` 可覆盖**、**C 类 102–107 需要真正的
消去/重写 tactic**（口径差异见 S3 §3.5：是否把 `False.elim` 计入「需要新 tactic」，
以及是否把 `def Exists.elim` 的定义体算作证明）。
按 S2 审计的**去注释代码计数**（`docs/notes/course-lean-style/tactic-audit.md` §4.1），
课程实际用到的高频形状是：

| 形状 | 处数 | 今天只能写 | Lean 4 会写 |
|---|---|---|---|
| `Set.subset`（⊆ 展开） | 176 | `Set.subset α A B` | `A ⊆ B` / `rw [Set.subset]` |
| `fun` 链 | 100 | `fun (x : α) => …` | `intro x` |
| `Exists.elim` | **76** | `exact Exists.elim A p Q h (fun w hw => …)` | `obtain ⟨w, hw⟩ := h` |
| `Eq.subst` | **55** | `exact Eq.subst α (fun x => …) a b h proof` | `rw [h]` |
| `Exists.intro` | 54 | `apply Exists.intro` + `exact w` | `use w` |
| `Set.ext` | 34 | `exact Set.ext α A B (fun x => …)` | `ext x` |
| `Iff.intro` | 34 | `apply Iff.intro` | `constructor` |
| `let` | 28 | `exact let g := …; body` | `have g := …` |
| `Or.elim` | 27 | `exact Or.elim … h` | `rcases h with ha \| hb` |
| `congrArg` | 19 | `exact congrArg …` | `rw` / `exact congrArg …` |
| `False.elim` | 19 | `exact False.elim <goal> h` | `exfalso` |
| `match` | 14 | `exact match … with` | `cases` |
| `And.intro` | 12 | `apply And.intro` | `constructor` |

（两份 subagent 计数有差异——S3 按**声明**归类、S2 按**出现处**统计且剔了注释；
以 S2 的处数为准，S3 的分类用于排改写顺序。）

**一条关键的利好（S3 §3.5.1）**：本语言的 `Exists.elim` 签名是
`(A) (p) (Q) (h) (f)`，`Q` 是**常量 motive**（`lib/Exists.sokonanoda:92`，由 `Exists.rec` 定义）
⇒ 「结论不许提到证人」是**类型层面的硬约束**，所以 **76 处 `Exists.elim` 全部是
「Q 不依赖证人」的形状** ⇒ **`obtain ⟨w, hw⟩ := h` 一条就能覆盖全部 76 处，
不需要依赖消去（dependent elimination）**。这大幅降低了 L3.1/L3.5 的实现风险。

### 1.2 今天**已经能用**的（好消息，别重做）

| 能力 | 实测 |
|---|---|
| `∧ ∨ ↔ ¬` 作为用户声明记法 | `infixr:35 " ∧ " => And` / `infixr:30 " ∨ " => Or` / `infix:20 " ↔ " => Iff` / `prefix:40 " ¬ " => Not` —— **term 模式下四条全部 checked** |
| `∀ (x : α), p x` | 原生关键字，checked |
| `∃ (x : α), p x` / `∃ x ∈ s, p` | `binder_notation "∃" => Exists`，checked（**类型必须写出来**） |
| 数学码点类符号（`U+2200–22FF`、`U+2A00–2AFF`） | 独立 token，**无需声明驱动词法**，`∈ ⊆ ∪ ∩ ∅ ≤ ∘` 都在类里 |
| 声明驱动的词法 | 类外符号（`¬`U+00AC、`↔`U+2194）也能声明，term 模式可用 |
| Lean 风格签名 | 实测 `theorem p (α : Type) (A B : Set α) : (∀ (x : α), x ∈ A -> x ∈ B) -> A ⊆ B := by …` → `exercise.open`、**0 诊断** |
| `apply` 自动补前导类型参数 | `apply Or.inl` / `apply And.intro` / `apply Exists.intro` / `apply Iff.intro` 全部可用，多子目标按序消费 |
| `:= by sorry` 与 `:= sorry` | 事件完全一致（都是 `exercise.open`）；空 `by` 块、`by` 中间留 `sorry` 都合法 |
| `match` 作为 tactic | 臂体是**项**，等价 `exact (match …)` |

### 1.3 今天**不能用**的（阻塞项，按严重度排序）

| # | 现象 | 最小复现 | 根因（已定位到行） |
|---|---|---|---|
| **X1** | **goal 里出现记法 ⇒ `apply` / `match` 直接报「目标不匹配」** | `infixr:35 " ∧ " => And` + `theorem t (A B : Prop) (ha : A) (hb : B) : A ∧ B := by apply And.intro; exact ha; exact hb` → ``apply` 的目标不匹配：`And.intro` 的结果是 `And a b`，无法对齐当前目标 `A ∧ B` | `by.rs` 的 `GoalNode.ty` 是**源 AST**（含 `Expr::Notation`），而 `apply` 的 `unify_spine` 拿内核 pp 的**点名** codomain 去对齐 → 头不同。`peel_pi`/`match` 同病 |
| **X2** | **非数学码点类符号在 `by` 块里报 `unknown identifier`** | `prefix:40 " ¬ " => Not` + `theorem t (A : Prop) : ¬ A -> A -> False := by intro h; exact h` → ``exact` 判定失败：unknown identifier `¬`；`↔` 同 | `judge.rs:1007` 的 `wrap_binders` 用 **`parse_expr_text`（无记法表）** 回读 binder 类型文本，而 `fold_declared`（`:981`）用的是 `parse_expr_text_with(text, notations)`。`¬` 是标识符字符 ⇒ 被读成 `App(Ident("¬"), A)`；数学类符号读失败后走「binder 类型推断」兜底，所以**只有类外符号炸** |
| **X3** | **`→`（U+2192）完全不可用** | `#check fun (A B : Prop) => A → B` → `unknown identifier →` | 箭头不在数学码点类里，也没有目标常量可做记法（`->` 是内建语法，无「箭头常量」）。需要**词法别名** `→` ≡ `->` |
| **X4** | **`=`（等号中缀）不存在** | `theorem t (α : Type) (a b : α) : a = b -> b = a` → parse 错 | 语言里只有 `Eq.{u} α a b`，`u` 必须显式写（`Eq α a b` 在 `α : Type` 时被内核拒：期望 `Sort(0)` 实际 `Sort(1)`）。⇒ 课程满屏 `Eq.{1} (Set α) A B` |
| **X5** | **`Ne` / `≠` 不存在** | `#check fun (α : Type) (a b : α) => Ne α a b` → `unknown identifier Ne` | prelude 没有 `Ne`（`PRELUDE_NAMES` 47 个里没有） |
| **X6** | **`⟨a, b⟩` 匿名构造子不存在** | `theorem t (A B : Prop) (ha : A) (hb : B) : A ∧ B := ⟨ha, hb⟩` → parse 错（在逗号处） | 没有这个语法。`⟨`/`⟩`（U+27E8/9）**声明后可用**（S1 实测），不需要改码点类；缺的是「占位符记法」这一形状（今天只有 infix/prefix/postfix/nullary/binder 六种） |
| **X7** | **tactic 白名单只有 7 个** | `have` / `constructor` / `cases` / `obtain` / `use` / `rw` / `simp` / `exfalso` 全部 parse 错 | `parser.rs:2844 is_tactic_keyword` + `by.rs` 的 `Tactic` 枚举 |
| **X8** | **`intro a b` 不吃多名字** | `intro ha hna` → parse 错 | `parser.rs:1243` 的 `intro` 分支只 `bump()` 一个名字（Lean 也这样，但课程改写需要它） |
| **X9** | **`Exists` 不在 prelude** | `binder_notation "∃" => Exists` 在没 import `lib.Exists` 的文件里报「目标 `Exists` 不存在」 | 卷 I 走 `lib/Exists`（L2 层）；入门课单元⑧ 自己声明。**保留现状**（这是教学设计，不是缺口）⇒ 卷 I 单元 1–5 要补 `import lib.Exists` |
| **X10** | **`∀`/`∃` 不能做算子操作数**；`∃ x y, p` 不行 | `True -> ∀ x, True` → `expected an expression, found Forall`；`∃ x y, p` → 内核裸错 | S1 实测；改写时**必须加括号**、多 binder 要嵌套 |
| **X13** | **`intro` 在否定目标（`¬ A`）上报「需要一个函数目标」**（本计划**实测新发现**） | `prefix:40 " ¬ " => Not` + `theorem t (A : Prop) : A -> ¬ ¬ A := by intro ha; intro hna; exact hna ha` → ``intro` 需要一个函数目标（… -> … 或 forall …），当前目标不是函数`` | `Not` 在 prelude 里是 **def**（`Not A := A -> False`，`prelude.rs` 的 `PRELUDE_L1_SRC`），所以 `¬ A` 在源 AST 里是 **`Expr::Notation`（target `Not`）**、展开后是 `App(Not, A)`——**两种形态都不是 `Arrow`**，而 `intro` 只做语法 `peel_pi`。否定目标在集合论里遍地都是（`x ∉ A`、`A ≠ B`）⇒ **必须看穿**。**已修**：`spine.rs` 新增 `peel_pi_or_not`（认 `Notation{target:"Not"}` 与 `App(Ident("Not"), X)` 两种形态，只看穿 `Not` 这一层，不做一般 whnf——那要内核暴露归约接口，碰内核冻结） |
| **X12** | **两段式 binder（`∀ x ∈ s, p` / `∃ x ∈ s, p`）在 `by` 块里必炸**（本计划**实测新发现**，先于本轮改动就存在） | `infix:50 " ∈ " => Set.mem` + `theorem t (α : Type) (s : Set α) (p : α -> Prop) : (∀ x ∈ s, p x) -> (∀ y ∈ s, p y) := by intro h; exact h` → ``exact` 判定失败：unknown identifier `x`` | 两段式 binder 的**类型来自 guard 的反解**，所以**源 AST 里 binder 的 `ty` 是 `None`**（`docs/design/notation-subset.md` §14.1）⇒ `render_binder`（`proof.rs:485`）打成 `(x) -> …`，回读时 `x` 变成**自由变量**。走内核 pp 也救不回来（渲染文本连解析都过不了，`judge_render_type` 直接失败）。**课程处置：一律写一段式**——`∀ (x : α), x ∈ s -> p x` 与 `∃ (x : α), x ∈ s ∧ p x` **实测都通过**（`by intro h; exact h` → `decl.checked`）。修本项要动 `render_expr`（把 guard 形状打回两段式）或让 elab 回填 binder 类型——**列为 R2 候选，不阻塞** |
| **X11** | **`∃` 出现在 `by` 块的目标/假设里必炸**（本计划**实测新发现**，S1–S5 都没抓到） | `binder_notation "∃" => Exists` + `theorem t (A : Type) (p : A -> Prop) : (∃ (x : A), p x) -> (∃ (y : A), p y) := by intro h; exact h` → ``exact` 判定失败：binder 记法 `∃` 里 `x` 没有类型：一段式要写标注（∃ (x : α), p）`` | `render_binder_notation`（`proof.rs:361-371`）**渲染时丢掉了 binder 的类型标注**（只打 `∃ x, p x`）⇒ 判卷回读路径（以及 L1.2 的 canonicalize 路径）重新解析时解不出类型。**修法**：渲染成 `∃ (x : α), p x`（几行） |
| **X14** | **零元记法（`∅`）落在「被应用」的位置 + `by` 块**（R1 第四片**实测新发现**；**已修，见 §9 与台账 G-19**） | **实测三连**：① `theorem (α : Type) (A : Set α) : ∅ ⊆ A := by intro x; intro hx; exact False.elim (A x) hx` ⇒ ``记法 `∅` 展开成 `Set.empty` 时补不出前面的类型参数``（exit 1）；② **同一句 term 模式通过**；③ `Set.subset α (Set.empty α) A := by …` 通过；④ `x ∈ ∅ → False := by …` **也通过** | **根因**：`∅` 是**零元**记法 ⇒ 源 AST 是 `Notation{target:"Set.empty", operands:[]}`。作 `∈` 的操作数时能从 `Set.mem` 的参数位拿到**期望类型 `Set α`** ⇒ 解得出；作 `⊆` 的操作数时 `intro` 先把 `Set.subset`（**def**）delta 展开成 `∀ x, A x -> B x`，`∅` 就落进**被应用**的位置（`∅ x`）——**期望类型没了**。**实际修法**：`by.rs` 的 `Tactic::Intro` 在目标头剥不动（= def）时**先用内核 pp 的规范形态再剥**，源级 delta 展开只作退路。回归测试 `notation.rs::a_zero_ary_notation_survives_delta_unfolding_in_a_by_block`（改前 FAILED / 改后 ok）。**影响面 227 处 `Set.empty` 站点**；**B0 的前置已清** || **X15** | **LSP 单文件解析失败 ⇒ 整个报告被丢弃（`report = None`）**（R1 第四片 subagent **代码确认**；**已修，见 §9 与台账 G-20**） | 用 `import` 带进来的记法时，编辑器里出现**假的** `notation-unknown-symbol`，且 hover / documentSymbol / codeAction / inlayHint **全死**（不是"降级"而是"消失"）；而同一份文本走 CLI 判卷 exit 0。（口径纠正 2026-09-19：不是"任何单元"——只影响**代码里用了 import 记法**的单元；unit02 在 HEAD 上仍是点名写法、不受影响。） | `crates/lsp/src/lib.rs:158-166` 在单文件解析失败时把 `report` 置 `None`；`:78-88` 又优先用 `parse_error`。**真正的根因是两半互相矛盾**：front 层**专门**为这种情况留了退路（`project/mod.rs` 的 `is_project_source`，0.60.0 加的），`query/mod.rs:144-151` 也**确实**把闭包报告装好了——LSP 紧接着把它丢掉。**修法（已落）**：入口模块状态 `Compiled` 才算"闭包救回来了"，此时以闭包报告为准。列为 `notation-input` 计划的 **NI-0**（hover 要能显示输入法，前提是 hover 通道活着） |

**结论 2**：X1/X2/X11 是**前端 bug**（都是几行的机制问题，`crates/front/**` 内可修，内核零改动），
X3–X6 是**小语法增量**（S1 给出了逐项最小改法：`→` **2 行**、`≠` **2 行**、
内建记法 **30–80 行**、`⟨⟩` **150–300 行**），X7/X8 是**引擎扩展**（工作量主体）。

### 1.3b 核心修法的**实测验证**（本计划亲测，改完即回滚，仓库零残留）

> 方法：临时 patch `judge.rs`/`by.rs` → `DEVELOPER_DIR=/Library/Developer/CommandLineTools
> cargo build -p sokonanoda-cli`（8.1s）→ 跑探针 → `cp` 回备份 → 重建 → 复跑确认行为复原。
> **`git diff --stat` 为空、`playground.sokonanoda` 仍 exit 0**。

| 验证项 | 结果 |
|---|---|
| **L1.1（`wrap_binders` 改用 `parse_expr_text_with`）** | ✅ **3 行改动**：`prefix:40 " ¬ " => Not` + `by intro h; exact h` 从 `unknown identifier ¬` 变成 **`decl.checked`**；`↔` 同样通过 |
| **L1.2 机制 (a)（强制 `canonical_goal_type`）** | ✅ `apply And.intro` 在目标 `A ∧ B` 上 **`decl.checked`**；`apply Or.inl` 在目标 `A ∨ B` 上 **`decl.checked`**（改前都是「目标不匹配」） |
| **X11（新发现）** | ❌ 即使 L1.1+L1.2 都打上，**`∃` 目标/假设仍然炸**（原因见 X11）⇒ **L1.7 必须一起做**，否则 `obtain`/`use` 落地后 `∃` 场景全废 |
| `∀` 目标 | ✅ 本来就通过（`render_expr(Forall)` 带类型标注，往返正常） |

### 1.4 记法在「显示面」的真实行为（**S5 实测纠正了旧口径**）

> `docs/design/notation-subset.md:13-14`/`:207`/`:242`/`:539` 与
> `docs/architecture.md:145` 都写成「goal/hover 全部点名形式」——**实测不符**，
> 这几处文档债要同轮改。

| 显示位置 | 今天回显什么 | 走哪条路 |
|---|---|---|
| **open 练习的 goal、per-tactic goal、binder 类型** | **已经是记法**（`A ⊆ A`） | 前端 `render_expr(源 AST)`（`proof.rs:323-345`） |
| **根状态**（光标停在声明行） | **点名**（`Set.subset α A B -> Set.subset α A A`） | `query/state.rs:54-56` 优先用 `d.ty_text`（内核 pp） |
| 声明签名 / 表达式 hover / `#check` | **点名** | 内核 pp |

实测同一文件同一条 `soko/stateAt`：光标在声明行 → `forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A A`；
光标移到 `exact` 行 → `A ⊆ A`。⇒ **最刺眼的缺陷是「光标一动，记法就消失」**。

**处置（三档，按性价比）**：

| 档 | 内容 | 量级 | 排期 |
|---|---|---|---|
| **T0-a（强烈建议，极便宜）** | 给 `DeclState` 加 `ty_src = render_expr(源 ty)`（把源 `ty` 补进 `PendingOp::OpenExercise`，`compile/check/mod.rs:44-83`），`query/state.rs:54-56` 优先取它 ⇒ **消掉「记法随光标消失」**，完全不解析内核文本 | **~25 行 / 2 文件** | **R1** |
| **T0-b（SP2 方案 ③，R4）** | 显示边界重写：新模块 `crates/front/src/notation.rs` 的 `print_back(text) -> DisplayText`（**新类型**，让「显示文本进 parser」变成编译错误），落点**只有 5 处显示出口**（`query/mod.rs:481`、`:402-418`、reduce 出口、`lsp/render.rs:178-198`、`lsp/lib.rs:1145`/`:1429-1433`） | **270–360 行 / 1 天** | **R4（可选）** |
| **T2（明确不推荐）** | 在 `editor/vscode` 做文本替换 | — | 不做：客户端对 goal/type 文本**零替换是既有契约**（`goal-rendering.md:12`、`highlighting.md:21-29`），且 CLI/opencode/DSH 拿不到 |

**T0-b 的护栏（必须写进设计）**：`compile/**`、`judge.rs`、`by.rs`、`proof.rs`、
`suggest.rs`、**`CheckEvent`（`grade --json` 事件流）一个字节不改**。
⚠️ `suggest.rs:410-431` 拿 `d.goal`/`d.binders[].ty` 去合成判定规格（`:157-174`）——
**显示字段已经在喂判定路径**，所以绝不能改 `DeclState` 的既有字段（只能**新增** `ty_src`）。
不动 `DocumentReport` ⇒ **不必 bump `CACHE_FORMAT`**（`cache.rs:23`）。

**T0-b 的已知坑（S5 §5）**：arity 必须等于 telescope 长度（否则 `Set.mem α a` 会被错显成
`α ∈ a`）；记法表按 **import 边合并**（不能取闭包并集）；括号用 `render_atom` 的
**保守补括号**（永不少括号 ⇒ 无歧义，代价是比 Lean 略啰嗦）；print-back 还会带来
**超出「换符号」的变化**（`forall (a b : T), X` → `(a : T) (b : T) -> X`、`Type 0` → `Type`、
pp 折行压平）⇒ **必须写进 `docs/protocol.md`**；最危险的失败模式是
`judge_infer` 的回读用 `parse_expr_text`（**不带**记法表，`proof.rs:45-47`）且**静默降级**。

- **好消息（S2 §7）**：`by` 块的**每步 goal 状态是可用且好用的**——引擎每执行一个
  tactic 就记录该步之后的全部剩余目标 + 上下文（`ByStep`/`ByGoal`，`by.rs:289-303`），
  经 `soko/stateAt` 按 Lean `goalsAt?` 语义选状态，VS Code 练习树 + Infoview 已消费。
  ⇒ 改写后**编辑器体验是净提升**（从「没有中间状态」变成「逐步 goal」）。

### 1.5 环境前提（**实施前必须知道**）

| 项 | 事实 | 处置 |
|---|---|---|
| 本地 Rust 构建 | 本机 **Xcode 许可未接受**，默认 `xcrun`/`cc` 失败（HANDOVER §5 已记）；但 **Command Line Tools 可用** | **`DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo …`**（本计划已实测：`cargo build -p sokonanoda-cli --locked` → **exit 0 / 2.57s**）。所有 `cargo fmt/clippy/test` 命令都要带这个前缀，或先 `sudo xcodebuild -license` |
| 判卷二进制 | `target/debug/sokonanoda` 是 0.61.0 仓库构建（`version --json` 的缓存标记 `0.55.0` 是**陈旧标记**，`--version` 自述 0.61.0；S2 用三条证据交叉验证过） | 改 front 后必须 `cargo build` 刷新，再跑 `scripts/soko grade`；**别**被 `match:false` 吓到 |
| 解析错误爆炸半径 | **一处未知 tactic ⇒ 整份文件 `decl_checked=0`**（S2 实测 PE1）——连前面写对的声明也全丢 | 改写**逐文件、逐声明**推进；半成品**绝不进 main**（课程门禁会整单元红） |

---

## 2. 目标形态（改写后长什么样）

以单元④练习 1 为例。**今天**（term 模式、全显式实参、点名）：

```sokonanoda
theorem union_comm (α : Type) (A B : Set α) :
    Eq.{1} (Set α) (Set.union α A B) (Set.union α B A) :=
  Set.ext α (Set.union α A B) (Set.union α B A) (fun (x : α) =>
    Iff.intro (Or (A x) (B x)) (Or (B x) (A x))
      (fun (h : Or (A x) (B x)) =>
        Or.elim (A x) (B x) (Or (B x) (A x))
          (fun (ha : A x) => Or.inr (B x) (A x) ha)
          (fun (hb : B x) => Or.inl (B x) (A x) hb) h)
      (fun (h : Or (B x) (A x)) => …))
```

**目标**（tactic 模式、符号、Lean 4 手感）：

```sokonanoda
theorem union_comm (α : Type) (A B : Set α) : A ∪ B = B ∪ A := by
  apply Set.ext
  intro x
  constructor
  · intro h
    cases h with
    | inl ha => exact Or.inr ha
    | inr hb => exact Or.inl hb
  · intro h
    cases h with
    | inl hb => exact Or.inr hb
    | inr ha => exact Or.inl ha
```

练习占位：`theorem … : A ∪ B = B ∪ A := by sorry`。
演示：同上，但证明写全（**D3**）。

> **注意**：目标形态里的 `Or.inr ha`（**隐式实参**）原属 **D2 明确不做**，**D6 已翻案
> ⇒ P1**（设计 `docs/design/implicit-arguments.md`）。分期上它落在 **R2.5**：
> **P1 落语言（课程零改动）→ P3/B1–B7 批量改用**；在那之前，落地形态仍是
> `exact Or.inr (B x) (A x) ha`，**这段过渡期的课程要按显式形态写**（否则会先写出
> 一份 P1 之前判红的课）。但 `·` **子弹点要做**（L3.9，S2 判定「没有它学习者要数第几个
> 目标」）；`cases … with | inl ha => …` 的臂内多 tactic 也由 L3.1 提供。所以除隐式实参外，
> 上面的目标形态是**本轮可达的**。

---

## 3. 语言侧工作项（L 系列）

> 纪律：`crates/kernel/**` **一个字节不许动**（硬规则 1）；新增语法走
> **课程 + 测试 + 白名单三件套**（硬规则 3）；判定永远走 kernel（硬规则 4）；
> 每项都要三层测试（front 单测 → CLI e2e → 课程/golden）。

### L1 记法 × tactic 打通（**阻塞项，必须先做**）

| 项 | 内容 | 落点 | 验收 |
|---|---|---|---|
| **L1.1** | 修 `wrap_binders` 的记法丢失（X2）：改用 `parse_expr_text_with(text, notations)`，`notations` 从 `judge_terms_uncached` 传进来 | `crates/front/src/judge.rs:1003` | 新 front 单测：`prefix:40 " ¬ " => Not` + `by intro h; exact h` 判 Match；CLI e2e：`¬`/`↔` 的 by 证明 exit 0；**回归**：无记法文件事件计数逐字节不变 |
| **L1.2** | 让 `by` 引擎看穿记法（X1）：goal 与假设类型在进入引擎前**归一到点名形式**。候选机制（设计时二选一并写清理由）：(a) 复用既有 `canonical_goal_type`（`by.rs:89`，render→judge pp→parse），把开关从「文件用了 namespace」改成「文件里有记法」；(b) 在 `walk.rs` 把**已 elaborate 的类型**交给引擎（源 AST → 内核 expr 的点名 AST） | `crates/front/src/by.rs`、`crates/front/src/compile/check/walk.rs` | 新 front 单测：`∧`/`∨`/`↔`/`∈`/`∃` 出现在 goal 时 `apply`/`match`/`exact` 全部可用；CLI e2e 对拍「同一命题记法版 vs 点名版」五元计数相等；**性能**：`scripts/perf-ledger.sh` 记录 by 块的 judge 调用增量（上限：课程门禁 wall time +30%） |
| **L1.3** | `intro a b` 多名字（X8）：`Tactic::Intro { names: Vec<String> }` | `parser.rs:1243`、`ast.rs:293`、`by.rs:168` | front 单测 + CLI e2e；`intro a` 行为不变 |
| **L1.4** | **修 `apply` 的两个真实 bug**（S2 §1.4/§9.1，**各 ~20 行、收益最大、必须先做**）：<br>**① `render_roundtrip`（`judge.rs:557-575`）只处理顶层 `Forall`** —— 嵌在箭头值域位的 `Forall` 被渲染成没有 `forall` 关键字的文本，回读失败。最小复现：`theorem t (A B : Prop) (C : Nat) (h : A) : Or A B := by apply Or.inl` ⇒ 「无法解析 `Or.inl` 的类型：expected `->` after binder group, found LParen」。**同一段代码换上下文就从通过变失败**。<br>**② pp 丢 `Eq.{1}` 的层级** ⇒ `same_head`（`spine.rs:66-83`）要求 Ident/UniverseApp 形状 + 层级全等 ⇒ **`apply Set.ext` 必炸**（「结果 `Eq A B` 无法对齐 `@Eq.{1} (Set α) A B`」）。**课程 682 处显式 `Eq.{1}` 全踩**。<br>修法：把 `canonical_goal_type`（`by.rs:89-107`，今天只对 namespace/open 文件开）放开到所有 `apply`，或让 `same_head` 容忍 pp 简化 | `judge.rs`、`by.rs`、`spine.rs` | 新单测：`apply Or.inl` 在带函数值域 `Forall` 的上下文里判 Match；`apply Set.ext` 在 `Eq.{1} (Set α) A B` 目标上判 Match；回归：既有 `apply` 测试全绿 |
| **L1.5** | **`rfl` 与新 `=` 的兼容**（S2 §1.6）：`rfl` 今天只在目标形如 `Eq.{u} α x y`（显式层级 + 显式 α）时可用 ⇒ L2.4 的 `=` 糖**必须**产出这个形状，否则 `rfl` 会在全课程失效 | `by.rs:243-282` | 新单测：`a = a` 上 `rfl` 判 Match |
| **L1.6** | **给 `by.rs` 补单测**（S2 风险项：`by.rs` **零单测**——`intro a b`/缩进/嵌套/失败消息/未知 tactic 错误码全无护栏） | `crates/front/src/by.rs` 的 `mod tests` | ≥8 条：白名单 6 个 tactic 各一条 + 失败路径 2 条 |
| **L1.7** | **修 `render_binder_notation` 丢类型标注**（X11，**本计划实测新发现**）：`proof.rs:361-371` 把 `Exists α (fun (x : α) => p x)` 渲染成 `∃ x, p x`（**没有标注**）⇒ 回读路径解不出类型 ⇒ `∃` 出现在 `by` 块目标/假设里必炸。**必须渲染成 `∃ (x : α), p x`** | `crates/front/src/proof.rs:361-371` | 新单测：`render_expr` → `parse_expr_text_with` 往返对 `∃` 成立；CLI e2e：`theorem t : (∃ (x : A), p x) -> (∃ (y : A), p y) := by intro h; exact h` exit 0（**改前实测 exit 1**） |

### L2 核心符号（Lean core 级，开箱可用）

> **设计决定（待 §7 spike 后定稿）**：核心记法放**前端内建记法表**（与 `+`→`Nat.add`
> 同族的「内建糖」），**不是**写进 `PRELUDE_L1_SRC`。理由（已核实）：
> `install_l1_prelude` 的 `command_belongs_to`（`prelude.rs`）只认 `Axiom`/`Def`/
> `InductiveBlock`，**记法命令写进 prelude 会被静默忽略**；而内建表天然解决
> 「让位 / 重声明 / 跨 import / 作用域」四个问题，且与 Lean core 的地位一致。

| 项 | 内容 | 优先级 |
|---|---|---|
| **L2.1** | `→`（U+2192）词法别名 ≡ `->`（X3）。**S1 给了确切改法**：在 `token.rs:279` 的 `∀` 分支旁加 `'→' => self.single(TokenKind::Arrow, start)`，**~2 行 + 测试**；顺带把 `→` 加进 `lexer_reserved_symbol_char`（给教学诊断）。S5/S1 都证实 `→` 今天**没有任何常量可指**（记法只产出 `mk_const`/`mk_app`，函数空间不是常量）⇒ 词法别名是唯一解。**不引入新语义** | **必须** |
| **L2.2** | 内建记法：`∧`:35 infixr→`And`、`∨`:30 infixr→`Or`、`↔`:20 infix→`Iff`、`¬`:40 prefix→`Not`。**优先级数字已被 S1 实测钉死**（内核级 `Eq.refl` 判别）：`→`25 < `↔`20 … 实为 `↔`20 / `∨`30 / `∧`35 / `¬`40 / `≠`50 / `∈`50 / `⊆`50 / `∪`65，整套 t1–t5 全 checked；`+`=65（`parser.rs:20`）；`->` 比任何记法都松；应用最紧。**两个词法细节**：① `∧ ∨` 在数学码点类里，天然是符号 token；② `↔`(U+2194)/`¬`(U+00AC) **不在类里**——今天靠**声明驱动词法**才成为符号，内建化后必须把内建符号也喂进 `parse_with_inherited`（`parser.rs:2796` + `graph.rs:376` + `proof.rs:57`，**S1 估 ~30–80 行**），否则 `A ↔ B` 会被读成标识符。**并加一条测试钉住「类外码点可以当记法符号」**——S1 实测 `→↔¬⟨⟩⁻¹×` 今天都能声明，但这是最容易被后人「顺手收紧」的地方 | **必须** |
| **L2.3** | `Ne` + `≠`（X5）——**已落（0.61.0）**。⚠️ S1 实测「`{u}` 版 `Ne` 必炸」，根因是 `elab_notation` 给常量传**空宇宙层**；**本轮把那个根因修了**（见 L2.4b），所以采用**与 Lean core 同形的 `{u}` 版**：`def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False`，进 **L1 prelude 的 B9 族**（`deps: ["B2","EQ"]`——定义体用 `False` 与 `Eq`，与 `Not` 依赖 B2 同理）。比 S1 建议的「单宇宙版 + 放课程库」更好：**任何文件零声明可用**，输入法表里 `≠` 的 `supported` 也才诚实 | **已落** |
| **L2.4a** | **`=` 的词法 token**（X4 的前置阻塞项）——**已落（0.61.0）**：`'='` 分支在不跟 `>` 时产出 **`TokenKind::Sym("=")`**（不是新 token 种类）⇒ parser 的算子表、elab 的记法展开、语义着色**三处零改动**就可用。**S1 预言的坑当场命中**：把 `=` 放进内建记法表后，词法的**最长匹配**把它当候选符号，`=>` 被切成 `=` + `>`，L1 prelude 第 9 行的 `fun … => …` 当场解析失败 ⇒ 新增 `parser::lexer_builtin_symbols()`（**喂给词法**的那一份，剔除 `=`）+ `LEXER_NATIVE_SYMBOLS` 常量，`=` 由 `'='` 分支原生产出。回归测试 `crates/cli/tests/notation.rs::fat_arrow_still_lexes_inside_a_fun_with_equality_available` | **已落** |
| **L2.4b** | **`=` 的 elab 糖**——**已落（0.61.0）**：`=` 进 `BUILTIN_NOTATIONS`（`infix:50` → `Eq`），`elab_notation` 修掉「给常量传**空**宇宙层」的根因——按 `known[canonical].universes()` 的长度分配层级，**1 个宇宙参数时从操作数类型的 sort 解出**（新 `level_text_of_sort`：`Prop`→`0`、`Type n`→`n+1`、`Sort n`→`n`；`judge_infer` 拿 sort，**不做文本猜测**）。产出正是 L1.5 要的形状（`Eq.{1} (Set α) A B`）⇒ `rfl` 不受影响（课程门禁逐项不变）。S1 估的「`EnvBuilder` 有没有问层数的 API **未核实**」已核实：**不需要**，层数在 `KnownTable` 里（`KnownName::universes()`） | **已落** |
| **L2.4c** | **`=` 的取舍（S1 明确建议不做）** —— **拍板：做，且已做完**（2026-09-19）。S1 的风险判断（动词法 + 动 elab 宇宙层）**成立**，但两条都有干净的解法（见 L2.4a/L2.4b），总改动 ≈ 120 行且**内核零改动**；收益是课程里最大的一类噪音（`Eq.{1} (Set α) A B`）当场消失，并且**顺带解决 C4 第 5 条**：`A = ∅` 里 `∅` 有期望类型 ⇒ 那 37 处 `Eq.{1} (Set …) … (Set.empty …)` 不再需要点名 | **已落** |
| **L2.5** | `∃` 内建 binder 记法（目标名 `Exists`，使用点解析）——把现有 `binder_notation` 的能力内建化。**S1 实测的 binder 边界**：`∀ (x:α), p` ✅ / `∀ x : α, p` ✅ / `∀ x, p` ❌ / `∃ (x:α), p` ✅ / `∃ x : α, p` ✅ / `∃ x, p` ❌ / `∀ x ∈ s, p` ✅ / `∃ x ∈ s, p` ✅ / `∃ x y, p` ❌ / `∃ x y ∈ s, p` ❌；**逗号必需**；⚠️ **`∀`/`∃` 不能做任何算子的操作数**（`True -> ∀ x, True` 报 `expected an expression, found Forall`）⇒ 改写时**必须加括号** | **必须** |
| **L2.6** | 集合论符号（`∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹' ×ˢ`）：**不进语言层**，留在课程 lib（`lib/Set` + 各页），但**全课程启用**（D4）。**落点定案**：把今天散在 `notation-cheatsheet` 的 `∈ ⊆ ∪ ∅` 与新的 `∩ \` 一起放进 **`lib/Set.sokonanoda`** 末尾（与已有的 `𝒫 ᶜ '' ⁻¹' ×ˢ` 同处），并**删掉对照页里的本地声明**（同符号重复声明 = parse 错；且这样 targets 保持 36，不用新增 lib 模块）。24 个 unit/solution **全部** `import lib.Set`（已核实）⇒ 一处声明全课程生效。⚠️ **S1 实测：这一步会让 `notation-cheatsheet` 当场判红**（`已经声明过记法了（由 import 带进来）`）⇒ **迁移第一步就是删它的本地四条** | **必须** |
| **L2.7** | `⟨a, b⟩` 匿名构造子（X6）：新语法 `Expr::AnonCtor { elems }`，elab 按**期望类型**选构造子（单构造子归纳 / `And` / `Exists` / `Prod` / `Iff`）。**S1 估 ~150–300 行**（今天的记法只有 infix/prefix/postfix/nullary/binder 六形状，`⟨_,_⟩` 要占位符记法）。`⟨`/`⟩`（U+27E8/9）**声明后可用**（S1 实测），不需要改码点类 | **强烈建议**（`use`/`refine` 的搭档） |
| **L2.8** | 子弹点 `·` 作为子目标分隔（`·` U+00B7）——Lean 惯用；**可选**，若成本高就用「按顺序消费 + `case`」 | 可选 |
| **L2.9** | 隐式实参（`Or.inr ha`、`Set.ext h`）——**D2 原本不做，D6 已翻案 ⇒ 立项为 IA-1（R2.5-P1）**：路线 C（风格对齐 + 唯一确定，**不引入元变量**）。**关键安全性质**：不写隐式 binder 的签名行为与今天的 `mk_app` 链**逐字节相同**（前提已实测：全仓唯一一处隐式 binder 在 `lib/Exists.sokonanoda:27` 的**注释**里）⇒ IA-1 可以**独立落地且课程一字不改仍全绿**。设计、影响面、测试反转清单见 `docs/design/implicit-arguments.md` | **IA-1（R2.5-P1）** |
| **L2.10** | **修诊断级联**（S1 M7）：任何**先前失败**的声明会让后面每个记法报误导性的 `elab-notation-unknown-target`（`judge.rs:372-395` 重编译 prefix+#check 取第一条错），**~10–20 行**。改写期大量半成品 ⇒ 这个噪声会严重拖慢调试 | **建议（R1 内）** |

### L3 tactic 扩展（**工作量主体**，按课程实际需要排序）

> 引擎现状：`by.rs`（543 行）用 `GoalNode { ty, intros, parent, kind }` 建模目标树，
> context = **沿父链的 `intro` binder**，`apply` 造兄弟子目标（`ApplyArg::SubGoal`）。
> `cases`/`have`/`rw` 要求 context 可扩展、可嵌套 → **需要一次引擎重构**（把
> `intros` 升级成真正的局部 context + 支持嵌套 tactic 块）。这是本轮最大的一块。

| 项 | tactic | 课程需要量（S2 去注释计数） | 引擎改动（S2 §5.2 估量） |
|---|---|---|---|
| **L3.1** | `cases h with \| ctor args => <tactics>` | `Exists.elim` 76 + `Or.elim` 27 + `match` 14 ≈ **120 处** | **高：300–500 行**。引擎**没有「由旧目标算新目标」的机制**。可行路径：(a) 用 `judge_infer` 取递归子（`Or.rec`/`Exists.rec`）类型文本 → 造 `motive := fun _ => <当前目标>` → 复用 `apply` 的剥 Pi/造子目标逻辑；(b) 让归纳表（`ElabCtx.inductives`）参与。子目标要**预置 intros**（今天 `apply` 建子节点写死空 Vec） |
| **L3.2** | `constructor` | `Iff.intro` 34 + `And.intro` 12 + `Set.Equiv.mk` 8 + 用户归纳 | **中：100–200 行**。本质是 `apply <ctor>`，缺的是「按目标头自动选构造子」需要的归纳表 ⇒ 要把 `inductives` 传进 `run_by`（改签名，调用点 `walk.rs:312-317`） |
| **L3.3** | `left` / `right` | `Or.inl`/`Or.inr` | **低**：`apply Or.inl/inr` 的别名 |
| **L3.4** | `use w` | `Exists.intro` 54 | **低-中：~100 行**：`judge_infer` 出构造子类型后复用 `apply` |
| **L3.5** | `obtain ⟨a, b⟩ := h` / `rcases h with …` | 同 L3.1，写法更 Lean | **高**（依赖 L2.7 的 `⟨⟩` + L3.1 的机制） |
| **L3.6** | `have h : T := t` / `:= by …` —— **已落（0.61.0）** | `let` 28 + 所有多步证明 | **实际 ~120 行，且没动 `judge.rs`**：降低成 `Expr::Let`（`NodeKind::Have`），上下文侧只要 `context_binders` 沿父链多收一个 binder。原估的「`GoalBinderSpec` 加 `body`」**不需要**——判定用的规格是**折叠望远镜**（`judge_terms` 的既有机制），`let` 的绑定在组装期才出现。**两个实测坑**：① 嵌套 `by` 里的 `cases` 组装出 `Expr::Match`，作应用实参时没有期望类型 ⇒ `elab-match-no-expected-type` ⇒ 必须用 `let`（它的标注就是期望类型）而不是 `(fun h => …) t`；② 值必须在 `cur.kind` 变成 `Have` **之前**算完，否则 `context_binders` 会把 `h` 算进它自己的上下文（自己证自己） |
| **L3.7** | `exfalso` | `False.elim` 19 + `absurd` 3 | **低：~30 行**：目标换 `False`，组装时套 `False.elim` |
| **L3.8** | `rw [h]`（**窄版**：单条等式、首个出现、非依赖 motive） | `Eq.subst` 55 | **很高：300–600 行**。引擎**完全没有目标改写能力**（没有 occurrence 抽象、没有子项→lambda 的工具、没有「新目标 defeq 旧目标」入口）。必须设计成「候选新目标 → `judge_terms` 内核 defeq 终审」，否则会滑向自证 |
| **L3.9** | `·` 子目标聚焦 | 多子目标处 | **中**：解析 + 目标焦点栈。**建议做**——没有它，`apply`/`cases` 之后学习者要数「第几个目标」 |
| **L3.10** | `change` | 展开引理处 | **中**：只需「新目标与旧目标 defeq」的内核判定 |
| **L3.11** | `trivial` | — | **低**：依次试 `True.intro`/`assumption`/`rfl`，教学友好 |
| **L3.12** | `contradiction` / `by_contra` | 单元⑩⑪ 的否定推理 | 中 |

**排序纪律**：**R1 = L3.1–L3.4 + L3.7 + L3.9**（覆盖 C 类 102–107 条的绝大多数）；
**R2 = L3.5 + L3.6 + L3.8 + L3.10**；L3.11/L3.12 按单元改写时撞到再补。

**S2 判定「不需要 / 做不了」的（写进 §10 不做清单）**：

- `simp` / `push_neg`：需要引理集与目标改写地基 ⇒ **不做**（只做窄版 `rw`）；
- `funext`：**做不了**——prelude 没有 `funext` 公理（课程用 `Set.ext` 公理替代，
  `lib/Set.sokonanoda:79`）；
- `induction`：课程只有 `Nat.rec` **1 处** ⇒ 不需要；
- `specialize` / `apply … at` / `injection` / `calc`：课程 **0 处** ⇒ 不做。

### L4 编辑器与协议同步（**同轮，不许拖**）

- `front::semantic::KEYWORDS` 加新 tactic 关键字 + `→`/`⟨⟩` 相关拼写；
- `editor/vscode/` 的 TM 语法词表同轮同步（守护测试
  `crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source`）；
- 新错误码进 `docs/protocol.md` 错误码表；
- 版本 bump（feature ⇒ **minor**；`Cargo.toml` + `editor/vscode/package.json` + `Cargo.lock`）。

---

## 4. 课程侧工作项（C 系列）

### C1 卷 I `courses/set-theory/`（35 文件 / 6550 行 / 424 顶层声明）

**工作量总账（S3 清单）**：

| 工作 | 量 | 风险 |
|---|---:|---|
| 记法替换（签名 + 证明体 + def 体，**只数裸名**） | **1077 处**（`And` 469 · `Not` 177 · `Exists` 157 · `forall` 113 · `Or` 85 · `Iff` 76）。分位：签名位 `And`46/`Or`7/`Iff`73/`Exists`27/`forall`41/`Not`65；证明体 `And`404/`Or`70/`Iff`1/`Exists`109/`forall`49/`Not`102；def 体 `And`19/`Or`8/`Exists`15/`forall`21/`Not`10 | 机械但量大；`∅`/`Eq` 边界要人工判断（见 C4） |
| 证明改 `by` | **245–246 条**（A 类 74–79 可机械 / B 类 64 `apply` 可覆盖 / **C 类 102–107 需要新 tactic**） | 依赖 R1/R2；C 类里 76 处 `Exists.elim` **不需要依赖消去**（见 §1.1 利好） |
| 练习占位 `:= sorry` → `:= by sorry` | **99 条** | 零风险（事件流逐字节相同，§1.2 实测） |
| hint 改写 | **294 条**（其中 **108 条**含项模式词汇） | 独立工作量，人工复核 |
| `def` **保留**项模式 | **71 条**（56 词汇/数据 + 12 `match` 定义 + 3 recursor 定义） | 明确不改（S3 §D.6 逐条） |
| `axiom`/`inductive` | 6 + 4 = 10 条 | 明确不改 |

**数字会不会变（S3 §5.7）**：**424 声明 / 329 checked / 99 open 都不变**
（记法不是声明、`:= by sorry` 与 `:= sorry` 事件相同）——**前提是只改写法、不增删声明**。
⚠️ 若为记法新增 lib 模块，目标数 **36 → 37** ⇒ 所以 L2.6 定案「记法进 `lib/Set`，
不新增模块」。行数会涨（`by` 块比项模式长）。

| 项 | 内容 | 验收 |
|---|---|---|
| **C1.1** | **记法统一**：签名 + 证明体里 `And A B` → `A ∧ B`、`Or` → `∨`、`Iff` → `↔`、`Not` → `¬`、`forall` → `∀`、`Exists α (fun …)` → `∃ …`、`Eq.{1} (Set α) A B` → `A = B`、`Set.mem α x A` → `x ∈ A`、`Set.subset` → `⊆`、`Set.union` → `∪`、`Set.inter` → `∩`、`Set.sdiff` → `\`、`Set.empty` → `∅`、`Set.powerset` → `𝒫`、`Set.compl` → `ᶜ`、`Set.image` → `''`、`Set.preimage` → `⁻¹'`、`Set.prod` → `×ˢ` | 每单元 `scripts/soko grade <绝对路径>` exit 0 |
| **C1.2** | **证明改 tactic**：246 条已证声明全部写成 `by` 块；99 条练习占位 `:= sorry` → `:= by sorry` | **◐ 大部分已落**：卷 I 的 99 条占位本来就是 `:= by` + `sorry`（合规）；**入门课** 104 条 `:= sorry` → `:= by sorry` 同轮改完（事件流逐字节相同，实测）；项模式证明还剩 ~10 条（`lib/Demo` 4 条 + 少量解答），见 §9「R2 主体收尾」 |
| **C1.3** | **hint 改写**：294 条 `-- soko:hint` 里的项模式词汇（`And.intro`/`Or.elim`/`Exists.elim`/`Eq.subst`）换成 tactic 词汇（`constructor`/`cases`/`obtain`/`rw`）；**答案绝不进 hint** 的红线不变 | 人工复核 + 课程门禁 |
| **C1.4** | **`lib/` 9 个模块**：`Set`/`Fun`/`Rel`/`Prod`/`Image`/`Equiv`/`Exists`/`Demo` 的证明改 tactic；`Logic` 是空壳不动；`lib/Set` 末尾的记法声明保留 | **◐ `lib/Set` 已完成**（记法 + 全部证明 `by`，23 checked）；其余 6 个模块记法机械替换已做、嵌套 `∃` 与 `Demo` 的 4 条项模式证明见 §9「R2 主体收尾」 |
| **C1.5** | **`notation-cheatsheet` 页重定位**：它今天教「点名 ↔ 记法」，D4 之后全课程都用记法 ⇒ 改成**记法速查表**（`∈`/`⊆`/`∪`/`∅`/`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ` 的 Lean core/Mathlib 出处 + 优先级梯子 + 「点名形式永久可用」） | `grade` exit 0；它**不是单元**（不进 `course.json`），改动不影响配额 |
| **C1.6** | **元数据同步**：`course.json`（配额/标签不动，除非练习增减）、`README.md` 的「现状」表、`AGENTS.md` 的写作纪律（「本单元只允许用」清单要换成新 tactic 集） | `python3 courses/set-theory/tools/check.py` exit 0 |
| **C1.7** | **大纲同步**：`docs/design/set-theory-syllabus.md` §4「记法引入顺序与无记法替代」改成「记法为主 + 点名作为底层解释」；§3 单元表里「本单元只允许用」的能力清单同步 | 文档评审 |
| **C1.8** | **课程内「会变成假话」的教学叙事**（S3 逐条列出，改写时必须一起改，否则学习者读到的是错的）：`unit03:9/10/24`、`unit04:7`、`unit07:20`、`unit10:12`、`unit11:11`、`unit12:17`、`unit08:23`、`solutions/unit07:3-4`、`unit03:3`、`unit12:9`、`notation-cheatsheet-solution:14`、`notation-cheatsheet:12-15/77`；README `:77`「其它单元继续用点名形式」、`:165-190` 的记法节 | 人工逐条复核 |

### C2 入门课 `course/`（52 文件 / 4337 行；44 个教学文件 = 2993 行）

> 来源：S6 审计 `docs/notes/course-lean-style/intro-course-constraints.md`（406 行，Q1–Q8 全答）。

| 项 | 内容 | 风险 |
|---|---|---|
| **C2.1** | **CN/EN 可以一套机械替换打两边**（S6 Q1 实测）：22 对（11 单元 + 11 解答）剔掉 `^--` 行后**逐字相同**，唯一例外 unit1 画布 EN 多 3 个空行；全语料**没有行尾 `--`** ⇒ `grep -v '^--'` 与 `course_shared.rs:88-94` 的 `code_only()` 等价。**注释行必须分别重写**（EN 是按语义重构的，不是翻译） | 低 |
| **C2.2** | **`:= by` 只存在于 unit4（画布 10 + 解答 8）与 playground（2）**，其余 10 个单元**纯项模式**（S6 Q2） | — |
| **C2.3** | `course/solutions/*` + `course/en/solutions/*` + `course/unit11-project/*` 同步 | 中 |
| **C2.4** | **`course/shared/{And,Or,Nat,Demo}` 规范副本**：`course_shared.rs` 的 `AND_COPIES`(:22-47)=**24 份**、`OR_COPIES`(:50-63)=**12 份**、`NAT_COPIES`(:66-75)=**8 份**；4 个测试（`shared_modules_compile_standalone` / `shared_demo_compiles_through_the_import_closure` / `every_unit_copy_matches_the_canonical_module`（双向）/ `the_shared_library_replaces_rather_than_duplicates_the_canvas_purpose`（画布/解答不许出现 `import`））。⚠️ **盲区**：`corpus_files()` 只扫 4 个目录，`course/unit11-project/Logic.sokonanoda:5-8` 是**第 25 份 And 副本**，**未登记、没有任何测试会拦** | **高** | **◐ C2.5 之后只剩 `Nat`**：`shared/{And,Or}.sokonanoda` 两个规范模块与它们的副本表（`AND_COPIES` 25 份 / `OR_COPIES` 12 份）已随自建骨架一起删除；`NAT_COPIES`（8 份）与 `Demo` 的跨模块自检保留（`Demo` 改成用 **prelude 的真归纳**写 And/Or 两条演示，名字不变，`course_shared.rs` 4 条测试全绿） |
| **C2.5** | **【方案变更】删骨架 > 改骨架**（S6 Q3，**推翻了本计划原先的推荐**）：① `And` **永远是 `axiom`**（unit1:17-27、unit4:23-32、unit8:15-20、unit9:19-22、unit10:18-21、unit11:17-20、playground:109-116 + 全部解答）⇒ 今天**没有 `And.rec`/`And.elim`**，`constructor`/`cases` 在 `And` 上不可用；② `Or` 两种形态并存（unit1/unit4/playground 是 `axiom`；unit9:34-37/unit10:23-26/unit11:22-25 是 `inductive`）；③ **但 prelude 早已自带 Lean core 级真归纳 `And`/`Or`**（`prelude.rs:201-211`，默认 `PreludeMode::Full`），且让位是**整族**的（`prelude.rs:141-147/233-281`）——文件自己写 `axiom And` 就把 B3 整族挤掉；④ **把自建骨架块整块删掉，真归纳自动回来**，且 `And.intro/left/right` 的**位置参数个数与今天一致**；⑤ `course/unit2:45-49` **已经明说**「prelude 自带整套骨架，不需要你自己造」⇒ 删骨架是**课程自己已经写下的方向**。**处置：删 `And`/`Or` 骨架块，保留 `True`/`False` 的 `axiom` 作为单元① 的 `axiom` 教学例子**（单元① 的叙事改写为「这些骨架 prelude 自带；这里演示 `axiom` 是什么」）。⚠️ **前置实测**：删之前必须内核实测 `Or.inl` 的参数形状（`axiom` 版 4 显式参 → 归纳版「归纳参 + 构造子参」）与整族让位的副作用 | **✅ 已落（2026-09-21，用户拍板**删**；第 116 轮）**：入门课 34 个文件 + `playground` + `unit11-project` 的自建 `And`/`Or` 骨架全删，prelude 真归纳接管（`constructor`/`cases`/`left`/`right` 全课程可用）；`course/shared/{And,Or}` 两个规范模块与副本表退役（只剩 `Nat`）；计数在内核重取后重钉四处（**checked 87→54、open 66 不变**）。as-built 见 §9「R2 主体收尾」之后的 C2.5 段 |
| **C2.6** | **单元④「by 写法」整单元重写**（S6 Q5）：今天教五个 tactic，顺序 `intro → exact → assumption → apply → rfl`（+`by sorry`），画布 84 行；练习 `by_ex1`(:54 `a->a`) / `by_ex2`(:60 `And a b -> a`) / `by_ex3`(:66 `a -> Or a b`) / `by_ex4`(:72 `a->b->And a b`) / **`by_ex6`(:83，编号跳过 5)**；解答里 `by_ex6` **回退成项模式**，另有 **9 条画布上没有的遗留声明**（解答 :29-37：`h_s/Pfam/Qfam/f_dep/qfam_true/val_apply_imp/val_apply_dep/by_ex7/by_ex8`）。⇒ 新 tactic 集的教学顺序要重排，遗留声明要清理，编号缺 5 要补 | **✅ 已落（2026-09-21，第 113 轮；四条全做，画布 (13,5,0)→(14,6,0)，见 §9「R3」）** |
| **C2.7** | **会红的测试 = 4 文件 13 个**（S6 Q7）：`course.rs` 的 `GOLDEN`(:86-98，11 元组 `(checked,open,reduced)`，总 86/65) + `every_course_canvas_compiles_with_golden_event_counts`(:101) + `every_solution_twin_is_fully_solved`(:158) + `solution_covers_every_canvas_exercise`(:207) + `en_mirrors_match_chinese_event_counts`(:287) + `en_solutions_match_chinese_event_counts`(:380) + `course_json_lists_the_eleven_units_in_order`(:243，不改清单则绿)；`course_status.rs` 的 `GOLDEN`(:68-80) + `course_subcommand_aggregates_the_manifest`(:83，summary **checked==86/open==65**) + `course_subcommand_human_view_lists_units`(:124)；`cli.rs:2070`（:2097-2098 钉 **86/65**）；`course_shared.rs` 4 测试。**必须从内核重新取数再钉，不能手算** | **✅ 已落**：实测总数 **87/66**（unit4 单项 `(14,6,0)`），四处钉子同步（`course.rs` GOLDEN / `course_status.rs` 逐单元表 + summary / `cli.rs` warm-cache）；其余 10 个单元**逐项复核与 GOLDEN 一致** |
| **C2.8** | `course/course.json` 是**扁平数组**（不是 `soko.course/2`），无计数，只有 `unit4` 的**标题**要跟着改（S6 Q7） | **✅ 无需改**：标题没变（本轮不动单元定位） |
| **C2.9** | **约 30 处会变假话的叙事**（S6 Q6）：`unit1:14`+`en/unit1:17-19`「用 axiom 搭最小逻辑骨架（与官方 Lean 的 And/Or 同构）」；`unit4:12`/`en/unit4:14`「首期五个 tactic（白名单，课程就是语法）」；`unit9:9,25,27-29`+EN「Or 从公理升级为真归纳」；`unit9:74`/`en/unit9:85`「本教学语言没有 ↔ 记号」；`unit11:37`+EN「单元① 的 Or 只是公理」（**11.1 节整段教学动机失效**）；`unit8:14`/`unit9:18`/`unit10:17`/`unit11:15-16,109,170` 及各自 EN；`playground:20-21,130,308-309` | **✅ 代码片段级已清（2026-09-21）**：脚本扫「注释里的 `And X Y`/`Or X Y`/`Not X`/`Iff X Y`/`->`」共改 **404 处**（21 文件），改完用「剥注释后代码哈希」证明**代码零变化**；剩下的命中都是点名（如「And axioms」「an And shape」）。**语义级仍欠**：涉及 C2.5 的句子（「Or 从公理升级」「11.1 节」）只有拍板删骨架后才需要改 |
| **C2.10** | `course/README.md` 写死：**24/12/8**（:39,:63）、**232/2974 行**（:57，**与实测 2993 行现在就已经不一致**）、「每个单元文件各自带所需 axiom/inductive 块」（:75，**删骨架后直接变反话**）、「逐字节一致」（:34）、「单元⑩/⑪ expr.reduced 是 0」（:74）、CI 守卫 5 条（:82-89）；`unit11-project/{Logic:3, Canvas:4-5,8, Exercises:3,16}` 的「公理」措辞 | **✅ 已清（2026-09-21）**：计数改成实测 **25/12/8**、行数改成实测「44 文件 4408 行 / 288 行在重复块里 ≈6%」并注明重量办法；「逐字节一致」「expr.reduced 是 0」两条复核后仍为真。「公理」措辞**保留**——C2.5 没拍板，自建骨架确实还是 `axiom` |
| **C2.11** | **课程外同轮必改的 6 处**（S6 Q6）：`REQUIREMENTS.md:18,208-220`、`docs/design/by-tactics.md:5,176,179`、`docs/design/course-syllabus.md:225,242`、`skills/sokonanoda-teacher/references/curriculum.md:13`、`skills/sokonanoda-teacher/SKILL.md:260`、`crates/front/src/lib.rs:4` | **✅ 六处全改（2026-09-21）**：另加 `course-syllabus.md` 单元④ 行、`SKILL.md` 新增「写 Lean 风格记法」与「全量 tactic 白名单 + 按单元解锁」两条 |

> **一个有意思的观察（S6）**：CN 注释里**已经在用** `∧ ∨ → ↔ ∀`（14 处，如 `unit9:9`、`unit8:10,34`、`unit10:54`、`unit11:11,92`），但**代码里 0 处**——所以入门课的记法改写本质上是「把注释里已经在用的符号搬进代码」。

### C3 `playground.sokonanoda`

- 362 行的教学画布：记法 + tactic 同步；`:= by` 已有 2 处。
- ⚠️ **它的叙事整体建立在「用 `axiom` 搭骨架」上**（S6 Q8）⇒ C2.5 的删骨架决定会直接改到它；且它是**用户直接面对的**画布（`skills/sokonanoda-teacher` 的主循环），改动要格外小心。

### C4 记法改写的 7 条实测边界（S3 §4.7，**改写时必须逐条对照**）

| # | 边界 | 影响 / 处置 |
|---|---|---|
| 1 | **`→` 不可用**（`unknown identifier →`） | L2.1 词法别名（已列必须） |
| 2 | `∧ ∨ ↔ ¬` 可声明且可用（5 个 example 全 checked） | 无需改语言；L2.2 只是把它们**内建化**（免每文件声明） |
| 3 | `∀` 原生但**必须写类型标注**（`∀ x, p x` 报 `elab-untyped-binder`） | `forall (x : A), …` → `∀ (x : A), …` 是**一对一替换**，零风险 |
| 4 | `∃` 走 `binder_notation`，**目标必须在作用域**，且只能在表达式开头 | **单元 1–5 要补 `import lib.Exists`**；库里声明 `∃` 后必须删 `unit08:141` 的本地声明（T5 冲突） |
| 5 | **`∅` 在 `Eq` 操作数位解不出 `α`** | 课程 **37 处** `Eq.{1} (Set …) … (Set.empty …)`（`unit02:63/71`、`unit04:146`、`unit06:149`、`unit08:212/241` …）**只能继续写 `Set.empty α`**——**除非 L2.4b 的 `=` 糖落地**（`A = ∅` 里 `∅` 有期望类型）⇒ 这是 `=` 的**第二个理由**（不只是好看） |
| 6 | **同一符号全课程只能声明一次** | `∈ ⊆ ∪ ∩ \ ∅` 必须**移进 `lib/Set`** 并删掉记法对照页 + 解答里的本地声明（L2.6 定案） |
| 7 | **`×ˢ` 的目标 `Set.prod` 定义在单元⑤ 画布**（不在 lib） | 单元 1–4 不能用 `×ˢ`。**二选一**：(a) 把 `Set.prod` 挪进 `lib/Set`（推荐——Lean 里 `Set.prod` 是 core/Mathlib 级）；(b) 接受单元 1–4 不出现 `×ˢ`。**推荐 (a)**，但它是 L2/L3 分层判据（`docs/design/course-stdlib.md`）的一次调整，要写进大纲 |

> 附（S3 实测）：`infix:50 " = " => Eq` 与 `infix:50 "=" => Eq` **都 parse 错**
> （声明行自己的 `=>` 被抢）⇒ `=` 只能走语言侧内建糖（L2.4a + L2.4b），记法命令做不到。

### C5 会被改写打破的测试与复现件（**必须原子同轮改**）

> 来源：S4 审计 `docs/notes/course-lean-style/tooling-impact.md`（逐条带行号）。
> 这些不是「顺手改」，是**同一个 commit 内**必须一起动的——否则 `cargo test` /
> `gap.py check` / `soko gate` / CI 一起红。

| # | 文件 | 钉住什么 | 改写后 |
|---|---|---|---|
| T1 | `crates/cli/tests/notation.rs:567` `the_shipped_course_still_uses_the_pointful_spelling` | **禁止**课程出现 `infix`/`notation`（:573-580）+ unit02 的 `Set.subset α A C`（:582） | ✅ **已按预言的改法处理**（R2 课程改写第一站）：改名 `the_shipped_course_uses_the_library_notation`，判据改成「课程用记法」，**不是删测试** |
| T2 | `notation.rs:357` `a_real_course_unit_grades_identically_in_notation` | unit02 两行签名逐字（:308/:312）+ `open>=8`（:384） | ✅ **已重钉**：改名 `…_in_either_spelling`，夹具方向反转（`pointful_variant`），`open>=8` 保留 |
| T3 | `notation.rs:527` `the_shipped_course_library_declares_the_five_symbols` | `lib/Set:155-159` 逐字 | 记法搬家（L2.6）后要重钉 |
| T4 | `notation.rs:548` `the_shipped_course_uses_the_library_notation_in_a_demo` | unit03 的 `𝒫 `/ unit08 的 `'' ` | 演示改写后重钉 |
| T5 | `notation.rs:768` `the_shipped_course_demos_the_binder_notation` | unit08 的 `binder_notation` + `∃ (` | **与「`∃` 搬进库」正面冲突**（重复声明 = parse 错）——S4 点名的第①号翻车点 |
| T6 | `notation.rs:451` `a_library_notation_works_in_the_entry_through_import` | 拷贝真 `lib/Set` | 随 T3 |
| T7 | `crates/cli/tests/query.rs:559` `query_check_matches_grade_on_a_real_course_unit` | unit05 的 `decl_checked==5` / `exercise_open==7`（:569-570）+ 删第一个 `:=` 要 `unexpected-token`（:603） | 计数断言要按改写后重钉（**只钉形状，别钉数字**——优先改成「与 `grade` 同判」而不是硬编码 5/7） |
| T8 | `crates/cli/tests/course_manifest.rs:277` `set_theory_manifest_is_v2_and_fully_grouped` | `units==12`（:338）、`chapters>=4`（:339）、`quota>0`（:316-323） | **只动结构时才红**；本计划不动清单结构 ⇒ 预期不红，但要在 S6 跑一遍确认 |
| T9 | `docs/gaps/repro/G07-course-manifest-v2.sh:68/84-85/146` | 真 `course.json` 的 1卷/4章/12单元/4 prereqs/4 tags/4 quotas | 同上（不动结构 ⇒ 不红；跑了才算数） |

**不会红（已核实）**：两处课程 GOLDEN（`course.rs:86`、`course_status.rs:68`、
`cli.rs:2097`）钉的是**入门课 `course/`（单数）**，与 `courses/set-theory` 无关；
`course_shared.rs` / `course_project.rs` / `skill.rs` / `protocol.rs` 用自家夹具或
入门课 ⇒ 入门课改写（C2）**会**碰 `course_shared.rs`，卷 I 改写不会。

### C6 三条会**静默翻车**的坑（S4 实测，写在这里省一次 debug）

1. **`example` 不计入 `checked`**：匿名 `example` 走**独立的 `example.checked` 事件**，
   `check.py` 完全不计数（`:571-575` 只认 `decl.checked`）。⇒ **解答文件里绝不能用
   `example`**（`checked` 会掉到 0 ⇒ G3 红）；画布里的演示用 `example` 是**安全的**
   （本来就不计），但**具名练习**必须保持 `theorem`/`def`，否则 G4 的按名字覆盖会
   **静默失效**（匿名 `exercise.open` 没有 `name`）。
2. **`∃` 搬家与 T5 冲突**：`binder_notation "∃" => Exists` 今天只在 `unit08:141` 本地声明；
   要全课程统一就得搬进库并**删掉本地声明**（同符号重复声明 = parse 错）。
3. **站点数字会静默变旧**：`scripts/gen-site-data.py:200` 强制离线 ⇒ 若
   `scripts/soko doctor --json` 不是 `ready:true`，它会**沿用上次实测的计数**而
   `scripts/check-site.py` **从不校验计数** ⇒ 站点永远绿但数字是旧的。
   ⇒ 收尾时 doctor 必须 `ready:true`，且生成日志必须打印**「门禁实测」**而不是
   「沿用上次实测的计数」。
4. **两个被 import 的模块各声明同一符号 = 静默后者覆盖**（S1 实测，不报错）。
   课程里 `lib/Set` 是记法的唯一家（L2.6）⇒ 谁都不许再声明 `∈ ⊆ ∪ ∩ \ ∅`；
   `notation.rs` 的 T3/T5 要把这条**钉成测试**。
5. **`∀`/`∃` 不能做算子的操作数**（S1 实测：`True -> ∀ x, True` 报
   `expected an expression, found Forall`）⇒ 改写时**必须加括号**：`True -> (∀ x, True)`。
   同理 `∃` 在实参位也要括号。
6. **`∃ x y, p` / `∃ x y ∈ s, p` 都不行**（S1 实测）⇒ 多个 binder 要拆成嵌套
   `∃ x : α, ∃ y : β, p`（或改写时避免该形状）。

### C7 安全改写顺序（W0–W11，来自 S4 的 S0–S11，采纳并改名为 W 避免与 §6.1 的调研 subagent 编号 S1–S7 撞车）

| 步 | 动作 | 验证 |
|---|---|---|
| W0 | 留基线：`check.py --report /tmp/before.json --json` | 记 36/329/99/0 + 逐目标 names |
| W1 | 先立记法：把 `∈ ⊆ ∪ ∩ \ ∅` 加进 **`lib/Set.sokonanoda`** 末尾，**同一个 commit 删掉 `units/notation-cheatsheet.sokonanoda` 的本地四条**（S1 实测：不删就当场判红「已经声明过记法了（由 import 带进来）」），并把 `∃` 一并搬进库 + 删 `unit08:141` 的本地声明；**不动其它单元** | `check.py` exit 0（**不新增 lib 模块** ⇒ targets 保持 36） |
| W2 | 单单元试点（**单元①**：最小，且不在 `notation.rs` 硬断言里），画布+解答同改 | `check.py --only "单元 1" --only "解答 unit01"` exit 0 |
| W3 | 试点的 `--only … --bisect` 与 `--selftest` | exit 0（确认定位手段仍可用） |
| W4 | 铺开其余 11 单元 + 记法页，**绕开 T1–T6 五个接触点** | 逐目标对 `before.json`，只允许有意变化 |
| W5 | 改 `lib/` | `lib_open == 0` |
| W6 | **原子同轮**改测试与复现件（T1–T7） | `cargo test -p sokonanoda-cli --test notation --test query --test course_manifest` + `python3 scripts/gap.py check` |
| W7 | 收尾元数据（**§4 F1–F14 清单**）+ `REQUIREMENTS.md` §9 + `STATUS.md` 一轮 | 人工 |
| W8 | `check.py --ledger` + `python3 courses/set-theory/tools/test_manifest_v2.py` | 后者 **CI 不跑，必须手跑** |
| W9 | 站点：`doctor --json` 必须 `ready:true` ⇒ `gen-site-data.py` ⇒ `check-site.py` | 日志必须出现「门禁实测」 |
| W10 | `scripts/soko gate` | exit 0 |
| W11 | 提交 | 若本轮没碰 `site/**`/`STATUS.md`/`Cargo.toml`，`pages.yml` 不触发 ⇒ 需 `workflow_dispatch`（并顺手把 `pages.yml:23-31` 补上 `"courses/set-theory/**"`） |

---

### F 同步与门禁清单（**与改写同一 commit**，逐条可验证）

| # | 项 | 内容 | 验证命令 |
|---|---|---|---|
| **F1** | 课程门禁 | `python3 courses/set-theory/tools/check.py`（G1–G6）| exit 0；`--selftest` exit 0 |
| **F2** | 课程计数 | `README.md` 的「现状」表（`:126`/`:127-128`/`:130-134`/`:145-158`/`:160-163`/`:192-196`/`:198-203`/`:205-212`）、`lib/Set.sokonanoda:140` 注释、`docs/courses/ledger.jsonl` 追加一条 | `check.py --json` 实测值逐项对齐；`check.py --ledger` |
| **F3** | 编辑器词表 | `front::semantic::KEYWORDS` + `editor/vscode` TM 语法**同轮**加新 tactic 关键字与新拼写 | `cargo test -p sokonanoda-cli --test extension`（`tm_grammar_keywords_follow_the_single_source`） |
| **F4** | 技能与入口 | `skills/` 三个技能 + `.agents/skills/` 入口 + `AGENTS.md` + `docs/vscode-dev-guide.md` | `cargo test -p sokonanoda-cli --test skill --test dsh` |
| **F5** | 设计文档 | 本文 as-built §9；`docs/design/notation-subset.md` §13.1 **改写**（把「销不掉」改成「① ② 不做；显示边界重写见本报告」）；**同文 §86-89 与代码矛盾必须修**——它说「不同符号同级按左结合」，代码（`parser.rs:1929-1939`）**是报错**（S1 实测），文档或代码必有一处要改；`docs/design/by-tactics.md` 补新 tactic；`docs/design/set-theory-syllabus.md` §4；`docs/design/course-stdlib.md`（L2 分层调整）；`docs/architecture.md:145` 的「goal/hover 全部点名形式」 | 人工 + 文档链接检查 |
| **F6** | 站点 | `scripts/soko doctor --json` 必须 `ready:true` ⇒ `python3 scripts/gen-site-data.py`（日志须打印**「门禁实测」**）⇒ `python3 scripts/check-site.py`；`site/set-theory.html:60`（单元①文件名）、`:80-81`；`site/course.html:35` | 三条命令 exit 0 + 日志核对 |
| **F7** | 缺口台账 | 新增条目（缺 tactic 一族；`∃` 搬家；`=` 词法；`→` 别名）+ `scripts/gap.py check` | `python3 scripts/gap.py selftest` + `check` exit 0 |
| **F8** | 顶层文档 | `STATUS.md` 一轮（旧轮归档 `docs/STATUS-ARCHIVE.md`）；`REQUIREMENTS.md` §9 追加日期条目（`:1670`/`:1687`/`:1751`）；`docs/HANDOVER.md`（`:7`/`:440`）；`docs/TESTING.md`（`:67`/`:70`/`:100`）；`docs/design/{teaching-project,course-manifest-v2,course-gate-in-ci,site}.md` 的计数 | 人工 |
| **F9** | CI / 站点触发 | `ci.yml:173/186` 的「G1–G5」→「G1–G6」；`pages.yml:23-31` paths 补 `"courses/set-theory/**"`（否则改课程**不触发部署**）；`scripts/soko:912` 的 "34 targets" | 读文件 |
| **F10** | 版本 | bump 两处（`Cargo.toml` + `editor/vscode/package.json`）+ `Cargo.lock`；⚠️ **`courses/set-theory/sokonanoda.toml` 的 `requires = "0.61"` 必须随语言 bump**——否则 `scripts/soko` 版本不符**直接 exit 3**（启动器的版本钉守卫） | `scripts/soko version --json` + `scripts/soko doctor --json` |
| **F11** | 全量门禁 | `scripts/soko gate`（fmt + clippy + test + playground anchor + 课程门禁 + 缺口台账） | **exit 0** |
| **F12** | 手跑项 | `python3 courses/set-theory/tools/test_manifest_v2.py`（**CI 不跑**） | exit 0 |
| **F13** | 性能台账 | `scripts/perf-ledger.sh`（by 块从 8 个涨到 ~250 个 ⇒ judge 探针调用量是主要变量）；记录课程门禁 wall time（基线 **12.2s**） | 追加 `docs/perf/ledger.jsonl`；**上限 +30%**（R-6） |
| **F14** | 入门课元数据 | `course/course.json`、`course/README.md` 里写死的计数、`course/unit11-project/*` 的 README | 人工 + S6 清单 |
| **F15** | 记法缩写表**双侧** | `front::notation_input`（Rust，唯一真相源）与 `editor/vscode/src/abbreviations.js`（JS 镜像）**逐字相等**——与 F3 的 KEYWORDS↔TM 同形制，但 F3 不覆盖它 | 新增 `crates/cli/tests/extension.rs` 守护测试（Rust 侧表 ↔ JS 侧表逐字 diff） |
| **F16** | 协议码表 | 新错误码 `elab-implicit-argument-unsolved`（IA-1）与 hover 的新形状（NI-1 已落，**无新码**）进 `docs/protocol.md` | `cargo test -p sokonanoda-cli --test query`（计数一致性）+ 人工核对码表 |
| **F17** | 缺口台账（**IA-0 的第一件事**） | 补 **G-19**（X14，`open` + `WO-012`）与 **G-20**（X15，`fixed_in=0.61.0` + `WO-013`）——**已落**（2026-09-19） | `python3 scripts/gap.py selftest` + `check` exit 0 |
| **F18** | 站点与对外文案（**发布那轮**） | 站点写的是**已发布版本**的事实 ⇒ `C1-language.md:179/:1183`、`C4-status-roadmap.md:117/:433` 与 `site/` 的 non-goals 现在说「没有隐式实参自动插入」是**真话**，IA-1 发布那轮必须同轮改；`docs/design/namespace-open.md:238/:364` 的论据要补限定 | `python3 scripts/site-verify.py` exit 0（站点由另一个 agent 负责，**本计划不碰 `site/`**） |

## 5. 分期与依赖（每轮都可独立验收、可发布）

```
R1  语言地基 ── L1（X1/X2/X11 打通 + apply 两 bug + by.rs 单测）+ L2.1–L2.3/L2.5
    + L3.1–L3.4/L3.7/L3.9 + L4 + **显示 T0-a（~25 行）**
    ├─ 出口：∧∨↔¬→≠ 在 by 块里可用；constructor/left/right/use/cases/exfalso/· 可用；
    │        「光标一动记法就消失」消失
    └─ 课程侧只做「1 个试点单元」（建议单元④，因为它全是 And/Or/Iff）

R2  引擎扩展 + 卷 I 全量 ── L2.4a/L2.4b(=)/L2.7(⟨⟩) + L3.5/L3.6/L3.8/L3.10
    ├─ 出口：obtain/have/rw 可用；卷 I 35 文件改写完；门禁 exit 0
    └─ 顺带：C1.3 hint、C1.4 lib、C1.5 速查页、C1.6 元数据

R2.5（新，D5+D6 追加）  记法可输入性 + 隐式实参
    ├─ P0 = NI-0 + IA-0：X15（LSP 报告不丢）+ 缩写表进 `front::notation_input` + 台账
    ├─ P1 = IA-1：隐式实参路线 C——**先落语言、课程不动**，
    │      验收 = `cargo test --workspace --locked` exit 0 **且课程零改动**
    │      （`notation.rs:163` 的护城河**不会**在 P1 变红——它用的是全显式 binder 的
    │       测试夹具；判据反转属于 P3 的 B1/B5）
    ├─ P2 = NI-1 + NI-2：LSP hover 显示「怎么敲」（已落，见 §9）+ VS Code 缩写改写器
    │      （Tab 触发；即时替换默认关）
    ├─ P3 = IA-2：课程批量改用隐式实参（B0–B7：G-19 先修，lib 签名 → units → 测试**原子同轮**）
    └─ 出口：`\and` 敲得出 `∧`；hover 说得出怎么敲；`Set.ext h` / `Or.inr ha` 可用且
            与点名写法判卷一致；moat 契约文档与测试在 B1/B5 同轮重钉

R3  入门课 + 收尾 ── C2（**含「删自建骨架」的决定**，见 C2.5）+ C3 + L3.11/L3.12（按需）+ F 全部同步
    ├─ 出口：`course/` 与 playground 改写完；site/ledger/skills/STATUS/REQUIREMENTS 同步
    └─ 发布：bump → push main → auto-tag → release

R4（可选，SP2）  显示期 print-back（S5 方案 ③ 的 P0/P1）── ≈270–360 行 / 1 天
    ├─ 出口：根状态 / 声明签名 / 表达式 hover / `#check` 回显记法；
    │        `grade --json` 逐字节不变（最重要的验收）
    └─ 决定点：R3 结束后按性价比拍板（不阻塞任何主线；T0-a 已在 R1 落地）
```

**硬依赖**：R2 的课程改写**不能**先于 R1（没有 tactic 就写不出目标形态）；
C2.4 的副本同步**必须**在同一 commit 内。

---

## 6. Subagent 分工（用户要求「结合计划和 subagents」）

> 纪律：每个 subagent 的产出必须**可验证**（跑命令、给 exit code），
> 不许「看起来对了」。所有判卷一律 `scripts/soko grade <绝对路径>`。

### 6.1 调研阶段（产出在 `docs/notes/course-lean-style/`）

| # | subagent | 产出 | 状态 |
|---|---|---|---|
| S1 | 记法能力审计（符号码点类 / 记法命令 / 作用域 / print-back 现状 / 优先级实测 / 最小改动清单 M1–M9） | `notation-audit.md`（903 行） | **完成** |
| S2 | tactic 能力审计（白名单 / 分隔 / 架构约束 / 缺口量级 / 编辑器） | `tactic-audit.md`（767 行） | **完成** |
| S3 | 卷 I 逐文件逐声明清单（424 条全在，机器校验 0 遗漏）+ 统计 + 风险 + 元数据 | `course-inventory.md`（1701 行） | **完成** |
| S4 | 门禁 / 工具 / 文档影响面 + 安全改写顺序 S0–S11 | `tooling-impact.md`（657 行） | **完成** |
| S5 | print-back 可行性（含 §1.4 现状纠正 + `=` 词法阻塞） | `printback-feasibility.md`（614 行） | **完成** |
| S6 | 入门课 + playground **结构性约束**（CN/EN 对称性 / 自建骨架 / `course_shared` 24·12·8 副本 / 单元④ 教学顺序 / 会变假话的叙事 / 元数据 / 风险） | `intro-course-constraints.md`（406 行，Q1–Q8 全答） | **完成**（第一次派活范围过大失败，收窄重派后成功） |
| S7 | 入门课**逐声明清单**（44 文件） | `intro-course-inventory.md` | **未做——列为 R3 开工前的第一个动作**（卷 I 的同类清单 S3 已完成；入门课的那份是执行期工件，不影响本计划的形状） |
| S8 | 记法**输入面**调研（Lean 4 缩写表逐字核对 / 客户端改写器 vs LSP completion / hover 通道 / `editor/`+`skills/` 影响面） | `notation-input-plan.md`（461 行）+ 设计 `docs/design/notation-input.md` | **完成**（D5） |
| S9 | 隐式实参**可行性与影响面**（三条路线对比 / 内核元变量槽实测 / 课程 2784 处点名调用的分布 / 会变红的测试与文档清单） | `implicit-args-plan.md`（1027 行）、`course-impact-implicit-args.md`（659 行）+ 设计 `docs/design/implicit-arguments.md` | **完成**（D6） |
| S10 | LSP 报告缺陷 **X15** 代码定位 + 复现路径（R1 第四片随行发现） | 本文 X15 行 + `notation-input.md` P0 | **完成（定位）/ 未修** |

### 6.2 实施阶段（每轮开工前按此表派活）

| # | subagent | 任务 | 交付判据 |
|---|---|---|---|
| I1 | 引擎 | **L1.1–L1.7**（记法 × tactic 打通 + `apply` 两 bug + `render_binder_notation` + `by.rs` 单测）+ **显示 T0-a** | front 单测 + CLI e2e 全绿；无记法文件事件计数逐字节不变；`apply Set.ext`/`apply Or.inl` 不再假失败 |
| I2 | 引擎 | **L3.1/L3.2/L3.3/L3.4/L3.7/L3.9**（cases/constructor/left·right/use/exfalso/`·` + context 重构） | 新单测覆盖「多臂 / 嵌套 / 臂内多 tactic / 分支 intros 预置」；`cargo test --workspace --locked` 全绿 |
| I3 | 语法 | **L2.1/L2.2/L2.3/L2.4a/L2.4b/L2.5/L2.7**（`→` / 内建记法 / `Ne`·`≠` / `=` / `∃` / `⟨⟩`） | 三层测试 + 课程单元各一条真实用法；`=` 的 6 条层级探针全 checked |
| I4 | 课程 | 卷 I 单元 1–6 改写（含 S2 试点单元①） | 6 个单元 `grade` exit 0 + `check.py --only` exit 0 |
| I5 | 课程 | 卷 I 单元 7–12 + `lib/` + 记法速查页改写 | 同上 + `lib_open == 0` |
| I6 | 课程 | 入门课 CN/EN + `course/shared` 副本 + `playground` | **开工前第一件事：产出入门课逐声明清单**（S7，格式照 S3）；`course_shared.rs` 绿 + 每个画布/解答 `grade` exit 0 + 三处 GOLDEN 有意识重钉 |
| I7 | 引擎 | **L3.5/L3.6/L3.8/L3.10**（obtain / have / 窄版 rw / change） | 各自三层测试；`rw` 必须「候选新目标 → 内核 defeq 终审」 |
| I8 | 文档/站点 | **C1.5–C1.7 + F1–F12** | `check.py` 绿、`check-site.py` 绿、`gap.py check` 绿、`skill.rs`/`dsh.rs`/`extension.rs` 绿 |
| I9 | 引擎 | **X15（LSP 报告不丢）+ X14（`∅` 嵌记法）**——`notation-input.md` P0 与 `implicit-arguments.md` B0 | X15：真 LSP 探针在 `courses/set-theory/units/` 下 **0 诊断 + documentSymbol 非空 + hover 非 null**（今天三条全挂）；X14：`∅ ⊆ A := by …` 从 `elab-notation-argument-unsolved` 变 `decl.checked`，且**课程门禁逐项不变** |
| I10 | 引擎 | **隐式实参路线 C**（`implicit-arguments.md` P1：新 `compile/implicit.rs` + `elab.rs` 唯一钩子 + 签名表 + `@f` 真语义） | 单测（对齐 / 唯一确定 / occurs check / 解不出报码）；**`cargo test --workspace --locked` exit 0 且课程零改动**；`git diff --stat -- crates/kernel/` 空 |
| I11 | 编辑器/协议 | **记法输入**（`front::notation_input` 表 + LSP hover 提示 + VS Code Tab 改写器 + 契约测试） | §6 的五层测试全绿（Rust 单测 / 表逐字相等契约 / stub 宿主 / 真 VS Code e2e / LSP 回归）；`editor/vscode` 版本 bump 与 `package.json` 新配置同轮 |
| I12 | 课程 | **隐式实参课程批 B1–B7**（lib 签名 → units → 测试**原子同轮**）+ 护城河契约重钉 | 每批：`check.py` 逐项绿；B5 的三条被逐字钉死的测试**同轮**改；`notation-subset.md` 与 cheatsheet 话术同轮 |

**并行纪律**：I4/I5/I6 可以并行（不同文件），但**都要等 I1/I2/I3 落地**；
I7 与 I4/I5 可交错（撞到缺 tactic 就补）；I8 全程可并行（文档），
但**计数类字段必须等课程改写完再填**。

**R2.5 的并行纪律（D5/D6）**：**I9 必须先做**（X15 挡着 hover、X14 挡着 B0 的验收）；
I10 与 I11 **两条完全独立**（一个动 `compile/`，一个动 `lsp/`+`editor/`，不撞文件）⇒ 可同时派；
**I12 必须等 I10 落地**（签名变了才改写），且 **I9 的 X14 修复必须早于 I12 的 B1**
（否则 lib 签名改隐式后 `∅` 的 227 处站点会集体判红，分不清是哪个改动闯的祸）。
I11 的 VS Code 侧与 I8 的文档侧**同一 commit**（用户可见改动的同轮同步纪律）。

---

## 7. 待 spike 的两个问题（**两个都已有结论**）

| # | 问题 | 结论（S5/S2 报告） |
|---|---|---|
| SP1 | **`=` 的宇宙推断机制**（L2.4）：`judge_infer` 拿操作数类型 → 再拿该类型的 sort → 解析层级，这条链在 `Prop`/`Type`/`Sort u`/层级算术下是否都稳？ | **机制可行，但有前置**：S5 实测 `=` 是**词法硬错误**——`token.rs:296-308` 的 `=` 分支**只认 `=>`**（`a = b` 报 `expected =>, found =`，未声明任何记法也一样）。⇒ L2.4 要**先加 `=` 词法 token**（与 `=>` 区分），再做 elab 糖。复现见 `docs/notes/course-lean-style/printback-feasibility.md` §10.3b |
| SP2 | **print-back 复核**（S5 报告 `printback-feasibility.md`）：在**不改内核**的前提下，前端显示期把内核 pp 文本重渲染成记法，是否真的「两套真相」不可接受？ | **判定：有条件做，且推荐做**（S5 提出**方案 ③ 显示边界重写**）。三条支撑：①「内核文本 → AST」往返**今天已在生产路径上跑**（`judge.rs:526`/`:597`、`by.rs:106`）；② §13.1 的前提**已部分失效**——**大部分 goal 面板内容今天已经回显记法**（open 练习的 goal 走前端 `render_expr(源 AST)`，`proof.rs:323-345`），真正的泄漏面窄且可枚举；③ 方案 ③ 在**所有回读之后**才发生，**判卷事件流一个字节不变**。量级：**P0 ≈ 270–360 行 Rust / 5 个文件 / 1 天**，覆盖 Infoview 根状态 + 声明签名 + 表达式 hover + `query goals/state` ≈ **课程 90% 的观感问题**。**护栏**：调用点白名单测试（`print_back` 只允许出现在 5 个显示出口）+ 「`grade --json` 逐字节不变」e2e |

**⇒ SP2 的处置**：本计划**主线不做**（D2 的边界不变），但把它列为 **R4（可选独立轮）**，
在 R1–R3 落地后按性价比决定。理由：它**不阻塞**课程改写，且判卷零风险；
但它是「像 Lean 4 一样可读」的**最后一块拼图**（写的是 `A ⊆ B`，hover 也回 `A ⊆ B`）。

---

## 8. 风险登记

| # | 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|---|
| R-1 | **引擎重构（L3.4）比预期大**：`GoalNode.intros` 沿父链的模型要升级成真局部 context，可能牵动 `apply` 的目标树与 `ByStep` 的每步快照 | 高 | 高（阻塞 R2） | 先做 I1（小修）验证通道；L3.4 单独一轮、配二进制对拍；实在做不动就退回「`by` + `exact <项>`」的伪改写并**明说** |
| R-2 | **`=` 的宇宙推断做不出来** | 中 | 中 | 退回 `Eq.{u}` 显式；课程里把 `=` 的缺失写进速查页 |
| R-3 | **课程改写改变判卷计数**（`checked`/`open` 变） | 高 | 低 | 计数本来就会变（`:= by sorry` 仍 open）；门禁**不锁计数**（G1–G6 与规模无关），只需同步文档里的数字 |
| R-4 | **入门课 `course_shared.rs` 的 24/12/8 副本漂移** | 中 | 中 | C2.4 同 commit；先跑该测试再提交 |
| R-5 | **hint 泄题**（改写时把答案写进 hint） | 中 | 中 | 人工复核 + 保留「答案绝不进 hint」红线 |
| R-6 | **性能退化**（每步 tactic 一次 judge 探针；by 块从 8 个涨到 ~250 个） | 中 | 中 | `scripts/perf-ledger.sh` 记录；门禁 wall time 上限 +30%；judge 缓存扩容/键优化 |
| R-7 | **编辑器体验落差**（面板显示点名形式） | 高 | 低 | 速查页正面教学（「面板回点名形式是对的」）；SP2 复核 print-back；S2 §7 已确认 `by` 步进状态可用 ⇒ 净提升 |
| R-8 | **入门课教学设计被破坏**（单元① 的 axiom 骨架） | 中 | 中 | C2.5 **已改判**：删 `And`/`Or` 骨架块（prelude 的真归纳自动回来；课程 `unit2:45-49` 自己已写下这个方向），保留 `True`/`False` 的 `axiom` 作教学例子。⚠️ **删前必须内核实测 `Or.inl` 的参数形状与整族让位副作用** |
| R-9 | **解析错误全文件连坐**（一处未知 tactic ⇒ 整份 `decl_checked=0`） | 高 | 高 | 逐文件、逐声明推进；半成品绝不进 main；改写期用「`by exact <原项>` 兜底」保证判卷恒绿（S2 §4.3 全部实测） |
| R-10 | **`example` 静默不计入 `checked`** ⇒ 解答写成 `example` 会让 G3 红；具名练习写成 `example` 会让 G4 的按名覆盖**静默失效** | 中 | 高 | C6 已列为红线；改写时**只允许画布演示用 `example`**，解答与具名练习一律 `theorem`/`def` |
| R-11 | **`∃` 搬家与 `notation.rs:768` 冲突**（重复声明 = parse 错） | 中 | 中 | C5/T5：搬进库时**同轮删掉 `unit08:141` 的本地声明**并重钉该测试 |
| R-12 | **本机 Xcode 许可**导致 `cargo` 链接失败 | 低（有解） | 中 | §1.5：`DEVELOPER_DIR=/Library/Developer/CommandLineTools`（已实测 exit 0） |
| R-13 | **`course_shared.rs` 的「25 号盲区」**：`course/unit11-project/Logic.sokonanoda:5-8` 是第 25 份 `And` 副本，**没有任何测试会拦**（`corpus_files()` 不扫该目录） | 高 | 中 | 改 `shared/And` 时**手工核对它**；顺手把 `corpus_files()` 的扫描目录补上 `unit11-project` + 加一条守护测试 | **✅ 已关（2026-09-21）**：`corpus_files()` 已纳入 `unit11-project/` 与它的 `solutions/`；C2.5 删骨架后该文件不再含规范文本，但扫描面保留（新抄的块照样会被抓） |
| R-14 | **入门课 13 个测试会红**（4 文件，见 C2.7） | 高 | 中 | C2.7 已列全；**必须从内核重新取数再钉，不能手算** |
| R-15 | **单元④ 编号缺 5、解答有 9 条画布上没有的遗留声明**（S6 Q5） | 中 | 低 | C2.6：重排教学顺序时一并清理 |

---

## 9. as-built（实施时逐轮追加）

### R1 第一刀：记法 × tactic 打通（2026-09-19，进行中）

> 改动全在 `crates/front/**`（**内核零改动**）。每一处都配了「改前实测失败 →
> 改后实测通过」的对照，并跑过 `cargo test --workspace --locked`（exit 0）与
> 课程门禁（**逐项不变**：36 目标 · 329 checked · 99 open · 0 判负，`--selftest` exit 0）。

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **L1.1** | `wrap_binders` 回读 binder 类型时改传记法表（`parse_expr_text_with`） | `judge.rs`（签名 +1 参、调用点 +1 参） | 改前 `prefix:40 " ¬ " => Not` + `by intro h; exact h` ⇒ `unknown identifier ¬`；改后 `decl.checked`。`↔` 同 |
| **L1.7** | `render_binder_notation` 带上 binder 的类型标注（`∃ (x : α), p x`）；退化形状（无标注 / 非单 binder）保持原样 | `proof.rs` | 改前 `(∃ (x : A), p x) -> (∃ (y : A), p y) := by intro h; exact h` ⇒ `elab-binder-notation-unsolved`；改后 `decl.checked` |
| **L1.2** | `apply` 的**失败重试**：`unify_spine` 第一次失败时把目标过一遍内核 pp 再试（**不替换节点上的目标**） | `by.rs`（新增 `canonical_goal_with_spec`） | 改前 `apply And.intro` 在 `A ∧ B` 上报「目标不匹配」；改后 `decl.checked`。**踩过的坑**：把归一化放到根目标上会让 `rfl` 在 `Eq.{1} (Set α) (Aᶜ) …` 上报「需要一个 `Eq α x y` 形状的目标」（pp 丢隐式实参）——既有测试 `a_library_notation_works_in_the_entry_through_import` 当场抓住，所以改成失败重试 |
| **L1.4①** | `render_expr(Forall)` 把**多 binder 组**拆成单箭头链（`(a : T) -> (b : T) -> …`） | `proof.rs` | 改前：假设类型是多 binder `∀` 时 `exact` 报「binder 缺少类型标注」；`apply Or.inl` 报「无法解析 `Or.inl` 的类型：expected `->` after binder group, found LParen」。改后两条都 `decl.checked`。单 binder 时与旧写法**逐字节相同** |
| **L1.4②** | `apply` 的**类型参数判定**改成 `mentions(name, codomain) || mentions(name, goal)` | `by.rs` | 改前 `apply Set.ext` 在 `Eq.{1} (Set α) A B` 上虽然对齐成功，却因为 pp 把 codomain 打成 `Eq A B`（丢 `α`）而**多出一个 `Sort 1` 垃圾子目标**、真子目标错位 ⇒ `intro x` 报「需要一个函数目标」；改后 `apply Set.ext; intro x; exact h x` ⇒ `decl.checked` |
| **X13（新发现，顺手修）** | `spine.rs` 新增 `peel_pi_or_not`：`intro` 看穿 `Not`（认 `Expr::Notation{target:"Not"}` 与 `App(Ident("Not"), X)` 两种形态） | `spine.rs`、`by.rs` | 改前 `A -> ¬ ¬ A := by intro ha; intro hna; exact hna ha` ⇒「`intro` 需要一个函数目标」；改后 `decl.checked` |
| **L2.1** | `→`（U+2192）**词法别名** ≡ `->`：词法主循环新增一臂收成 `TokenKind::Arrow`（与 `∀` 同款、在符号分支之前）；并把 `→` 加进 `lexer_reserved_symbol_char`（声明成记法时给人话报错）。⚠️ **还必须把 `→` 加进 `is_math_symbol`**——那个谓词同时是「不是标识符字符」的判据，不加的话 `A→B`（无空格）会粘成**一个标识符**（实测） | `token.rs` | 改前 `#check fun (A B : Prop) => A → B` ⇒ `unknown identifier →`；改后 `(A→B)→A→B` 与 `(A → B) -> A → B` 都 `decl.checked`；`infixr:25 " → " => Nat.add` ⇒ `notation-shape`「`→` 是语言关键字的一部分…不能当记法符号」 |
| 回归测试 | `crates/cli/tests/notation.rs::notation_works_inside_by_blocks`（1 条 e2e，覆盖 X1/X2/X11 + `rfl` 回归）；`crates/front/src/token.rs::the_unicode_arrow_is_a_lexical_alias_of_the_ascii_arrow`（词法层） | `crates/cli/tests/notation.rs`、`crates/front/src/token.rs` | 两条新测试通过 |

**与设计的偏差（要记的）**：
1. **L1.2 的落点从「根目标归一化」改成「`apply` 失败重试」**——根目标归一化会打坏
   `rfl`/`match` 这类**要读目标结构**的 tactic（内核 pp 丢隐式实参），既有测试当场抓住。
2. **新发现 X12 / X13 两条边界**（S1–S6 六份审计都没抓到，是本轮实测挖出来的）：
   X12 两段式 binder 在 `by` 块里不可用（课程改写一律用一段式，已实测通过）；
   X13 已修（见上表）。
3. `L1.4①` 的修法**不是**改 `render_roundtrip`（S2 的建议），而是改
   `render_expr(Forall)` 本身——后者同时修好了「假设类型是多 binder `∀`」这条
   `render_roundtrip` 够不着的路径。

#### R1 第二刀：`intro` 多名字 + 第一批 tactic 糖（同日）

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **L1.3** | `Tactic::Intro { names: Vec<String> }`：一次剥多层；吃名字时**遇到 tactic 关键字就停** | `ast.rs`、`parser.rs`、`by.rs`、`proof.rs` | 改前 `intro a b` 是 parse 错（S2 的 E01）、裸 `intro` 会把下一行的 `exact` 吃掉（E02）；改后 `intro a b c` / `intro ha hna`（含 `¬`）都 `decl.checked`，裸 `intro` 报「`intro` binder name, found Ident("exact")`」 |
| **L3.2/L3.3/L3.4/L3.7** | 新增 `constructor` / `left` / `right` / `use w` / `exfalso` 五个 tactic。前四个是 `apply <构造子>` 的糖（构造子来自前端**归纳表**，经 `lower_value` 传进 `run_by`），`exfalso` 是「换目标 + 组装套 `False.elim`」 | `ast.rs`、`parser.rs`、`by.rs`、`proof.rs`、`semantic.rs`、`check/mod.rs`、`check/walk.rs`（3 个调用点）、`editor/vscode/syntaxes/sokonanoda.tmLanguage.json` | 五条实测全 `decl.checked`（含记法目标 `A ∧ B` / `A ∨ B` / `∃ (x : A), p x`）；目标头不是归纳时给**教学错误**（「不在归纳表里…先 `intro` 拆开试试」），不是内核裸报错 |
| **归纳表接线** | `run_by` 新增 `inductives: &InductiveTable<'_>` 参数（`pub(crate)`）；`lower_value`/`lower_by_val` 同步，`walk.rs` 三个调用点传 `&self.inductives` | `by.rs`、`check/mod.rs`、`check/walk.rs` | `cargo build` 干净；`clippy::ptr_arg` 抓了一次（`exact_tactic` 的 `&mut Vec` 改 `&mut [_]`） |
| **内建兜底（实测新发现）** | `constructor` 在 `Iff` 上失败——prelude 里 `Iff` 是 **def**（`And (A->B) (B->A)`），不在归纳表里。加一张**极小**内建表 `Iff → Iff.intro`（只列 prelude 自己定义的，课程自定义的 def 不猜） | `by.rs` | 改前报「不在归纳表里」；改后 `constructor` 在 `A ↔ B` 上给两个子目标、`decl.checked` |
| **L4（部分）** | `front::semantic::KEYWORDS` 与 `editor/vscode` TM 语法**同轮**加 5 个 tactic 拼写 | `semantic.rs`、`sokonanoda.tmLanguage.json` | 守护测试 `tm_grammar_keywords_follow_the_single_source` 绿（34 passed） |
| 回归测试 | `crates/cli/tests/notation.rs::intro_accepts_several_names_at_once` + `the_new_sugar_tactics_grade_with_notation_goals` | `crates/cli/tests/notation.rs` | notation e2e **22 passed** |

#### R1 第三刀：`cases`（L3.1，**课程改写唯一的真阻塞**）

| 项 | 实际改了什么 | 落点 |
|---|---|---|
| **语法** | `cases h`（不带 `with`：子目标按构造子声明顺序，分支假设用**构造子的字段名**）与 `cases h with \| ctor a b => <tactics> …` 两种形态。臂体是**嵌套 tactic 序列**，用**缩进**界定——下一个 tactic 关键字出现在**比 `\|` 更深的列**上 ⇒ 属于本臂（本语言唯一的缩进敏感处，与 Lean 的 layout 同义；没有它无法区分「臂体还有一步」与「cases 写完了」）。`with` 靠既有的 `scrutinee_depth` 机制不被 `parse_expr` 吃成实参 | `ast.rs`（`Tactic::Cases` + `CasesArm`）、`parser.rs`（`parse_cases_arms` / `parse_tactic_sequence_in_arm`） |
| **引擎** | **降低成 `match`**：每个分支造一个子目标节点（分支假设进 `intros`），臂体的 tactic 序列在它上面跑；父节点变 `NodeKind::Cases`，`assemble` 产出 `Expr::Match`——递归子/iota 交给**既有的 `match` 降低路径**，引擎不手搓 recursor。顺带把主循环抽成可递归的 `run_tactics`，把组装抽成 `node_body`/`arm_body`（`arm_body` **跳过**模式绑定的那几个 intro——它们由模式绑定，包成 lambda 就错了） | `by.rs` |
| **书写类型归一（关键）** | `match` 要从**书写类型**里取参数实参（`h : Or A B` ⇒ params `A B`），而源里写的是记法 `A ∨ B`（`Expr::Notation`），`src_spine` 看不见 ⇒ 报「被匹配项必须是一个书写类型为 `Or …` 的局部变量」。`cases` 会把那个假设的书写类型换成内核 pp 的规范形态（**只动这一个 binder**，且只在 `cases` 路径上） | `by.rs`（`canonicalize_binder_type`） |
| **分支假设的类型** | 来自归纳表的 `MatchField.src_ty`，用 scrutinee 的**实际参数**代换：`h : Or (A x) (B x)` 的分支假设是 `A x` 而不是声明里的参数名 `A` | `by.rs`（`cases_tactic`） |
| **错误路径** | 被消去项不是局部名 / 不是归纳类型 / 分支假设个数不对 / 目标**依赖**被消去的假设（dependent elimination）——都给**教学错误**，不是内核裸报错 | `by.rs` |
| 回归测试 | `crates/cli/tests/notation.rs::cases_splits_hypotheses_with_and_without_arms`（覆盖两种形态 + 多步臂体 + `∃` binder 记法 + `And` + 两条错误路径） | `crates/cli/tests/notation.rs` |

> **一条教训（值得记）**：实现过程中我一度以为「参数化归纳的 `match` 坏了」——
> 实测 `match h with \| Or.inl ha => Or.inl B A ha` 报 `kernel-rejected 类型不匹配：
> 期望 $2，实际是 $3`。真相是**我自己的测试写错了**：本语言**没有隐式实参**，
> `Or.inl B A ha` 里的 `ha` 必须是 `B` 而不是 `A`（从 `ha : A` 要造 `Or B A`
> 得用 `Or.inr B A ha`）。`match` 一直是好的。**教训**：报「内核类型不符」时，
> 先怀疑自己给的显式参数对不对，再怀疑语言——这条与 G-15（量具缺陷伪装成
> 内核缺陷）是同一类。

#### R1 第四刀 + W1/W2：记法立库、内建记法、源级 delta、单元① 试点（同日）

| 项 | 实际改了什么 | 证据 |
|---|---|---|
| **L2.2 内建记法** | `∧ ∨ ↔ ¬` 进 `parser.rs::BUILTIN_NOTATIONS`：**任何文件零声明可用**，且**不能重声明**（`notation-shape` + 人话）。为什么不写进 prelude：`install_l1_prelude` 的 `command_belongs_to` 只认 `Axiom`/`Def`/`InductiveBlock`，记法命令会被**静默忽略**。`↔`/`¬` 不在数学码点类里，所以内建符号也喂进 `parse_with_inherited`/`parse_fragment_with_inherited` 的词法集合 | 新 e2e `core_logic_notation_is_built_in`（正例 4 条 + 反例 4 条） |
| **`∀`/`∃` 可作算子操作数** | `parse_operand` 遇到 `Forall`/binder 记法时改走 `parse_expr`（**向右最大吞噬**，Lean 的读法）。没有它，课程里最常见的 `(A ⊆ B) ↔ ∀ (x : α), A x → B x` 直接 parse 失败（实测） | 单元① 画布 `demo_subset_def` 通过 |
| **W1 记法立库** | `∈ ⊆ ∪ ∩ \ ∅` 搬进 `lib/Set.sokonanoda`（`∩`/`\` 取 70，比 `∪` 的 65 紧，照 Lean core）；`binder_notation "∃"` 搬进 `lib/Exists.sokonanoda`；对照页（画布+解答）与单元⑧ 的本地声明删掉 | 课程门禁 **36/329/99/0 逐项不变**；`notation.rs` 的课程夹具不再自带声明 |
| **源级 delta 展开** | 新增 `DefTable`（`elab.rs`）+ 两处登记（prelude 的 L1 def、walk 的 `def`）+ `spine.rs::unfold_head_once`/`peel_pi_delta`。**为什么必须**：`A ⊆ B` 的 `Set.subset`、`¬ A` 的 `Not`、`A ↔ B` 的 `Iff` 在库里都是 **def**，语法上不是 Pi ⇒ `intro`/`apply` 报「需要一个函数目标」；而语言**没有内核 whnf 的公开入口**（内核冻结）。三个踩过的坑：① 记法节点也要认（`Expr::Notation` 的头是 `target`）；② `def` 的值位是 **lambda 包着的**，要 `strip_lambdas` 才露出定义体；③ 记法只给操作数，参数要**从右对齐**（前导类型参数保持原名，课程里它就是上下文变量） | `intro x` 在 `A ⊆ B` 上、`apply h`（`h : A ⊆ B`）都 `decl.checked` |
| **X1 的根修（顺带修掉一个既有 bug）** | `unify_spine` 改用 `spine_with_notation`：**记法节点算作 `target(操作数…)`**。以前靠「把目标过一遍内核 pp」绕，但那条路**有损**（pp 丢隐式实参与宇宙层级）⇒ σ 里塞进缺参数的坏类型。同轮：`same_head` **只比名字**（`Eq` ≡ `Eq.{1}`）、实参**从右对齐**、`keep_if_lossless` 兜底 | **最小复现（在 stash 掉的基线上同样失败，确认是既有 bug）**：`def Foo (α : Type) : Type := α -> Prop` + `namespace Foo` + `axiom ext … : Eq.{1} (Foo α) A B` + `by exact Foo.ext α A B` ⇒ 改前 `期望 Sort(0)，实际是 (Foo.[] $2)`、改后 `decl.checked` |
| **W2 单元① 试点** | 画布 + 解答全部改写成目标形态：符号签名、`by` 块、`intro`/`apply`/`constructor`、hint 换成 tactic 词汇 | 解答 6 条全 `decl.checked`（exit 0）；画布 2 演示 checked + 6 练习 open |

**下一步（R1 未完）**：L1.6（`by.rs` 单测——已由四条新 e2e 部分覆盖）、T0-a（`ty_src`）、
L2.3（`Ne`/`≠`）、L2.5（进 prelude 的 `∃`）、L2.10（诊断级联）、L3.9（`·` 聚焦）、
L4（编辑器词表 + 版本 bump）；**R2.5 的剩余项**：**IA-1（隐式实参路线 C）** 与
**NI-2 的 VS Code 缩写改写器**——两片的前置（G-19/G-20）都已在 §9 清掉。

**验证（本轮实测）**：`cargo test --workspace --locked` **exit 0**（1169 passed / 0 failed）；
`cargo fmt … --check` **exit 0**；`cargo clippy -p front -p cli -p lsp --all-targets`
**零 warning**（仅冻结内核 crate 有 62 条既有 warning）；课程门禁
**36 目标 · 329 checked · 99 open · 0 判负**（**逐项不变**）、`--selftest` exit 0；
`git diff --stat -- crates/kernel/` **空**（内核零改动）。

#### R1 计划追加：D5/D6 两份设计（同日，**未动代码**）

> 用户在 R1 中途追加两条要求（原话见 §0 的 D5/D6）。本轮**只写设计**，
> 两份文档各自独立、各自带调研底稿与实测证据；**都不动内核**，也都不改变 R1/R2 的
> 形状，只是把 R2 之后劈出一个 `R2.5` 片（§5）。

| 文档 | 回答什么 | 关键结论（一句话） |
|---|---|---|
| `docs/design/notation-input.md` | 用户怎么把 `∧` 敲出来？hover 怎么教？ | **客户端缩写改写器**（`\and`→`∧`，与 Lean 4 `@leanprover/unicode-input` **同一张表**、逐字核对 19 个符号）+ **hover 里放输入法**；**不为缩写新增 LSP 补全 provider**——DSH 与 opencode 都不消费补全项，而 **DSH 的 `lsp` 工具消费 hover**（所以 X15 必须先修，否则 hover 通道是死的）。期号 **NI-0**（X15 + 缩写表进 `front::notation_input`）/ **NI-1**（hover 提示，**已落**）/ **NI-2**（VS Code Tab 改写器）/ NI-3（可选）。~~"P2'（DSH hover 兜底）"不存在~~——DSH 的 hover 是**同一条** LSP 通道，NI-1 落地即生效，不需要单独一期 |
| `docs/design/implicit-arguments.md`（174 行） | `{α : Type}` 自动推导怎么做？ | **路线 C：风格对齐 + 唯一确定**（400–600 行，**零额外内核调用**）——显式实参按风格对齐到显式层、跳过隐式层，被跳过的层由后续显式实参的类型 + 期望类型**头部匹配唯一确定**，解不出报 `elab-implicit-argument-unsolved`，**不猜不搜索不引入元变量**。路线 A（真元变量）被内核堵死（`Expr`/`Value` 无元变量槽），路线 B（探针）每次应用要整前缀重编译 ⇒ 课程 2784 处调用会被拖垮 |

**为什么隐式实参从「不做」变成「必须做」（实测数字）**：课程点名调用 **30 文件 /
1520 行 / 2784 处**，其中 **2308 处（83%）写了前导类型实参**（`Set.mem α a A`）——
记法替换掉之后，这些「点名解释」还要留在 hint 与叙事里当作底层说明，可读性瓶颈
就从符号转移到了**必须写全参数**。另有 765 处在注释/hint 里。最重的四个文件：
`solutions/unit12` 562、`solutions/unit08` 529、`solutions/unit09` 310、`unit08` 204。

**两条设计各自的「安全性质」**（这是它们敢分阶段发布的理由）：
`notation-input` 的改写器是**纯客户端字符串替换**（不碰协议、不碰内核，用户可关）；
`implicit-arguments` 的路线 C 在**签名没有隐式 binder 时与今天的 `mk_app` 链逐字节相同**
⇒ P1 可以先落语言、**课程一字不改也全绿**，把风险与课程改写彻底解耦。

**已知会翻案的契约（P1 同轮必改，别忘）**：今天的教学契约写着「省 `α` 的点名写法
`Set.mem a A` **改前改后同样被拒**」——路线 C 落地后它**会变成合法**。这是**有意的
契约变更**，被 `crates/cli/tests/notation.rs:163`（硬红）与 `docs/design/notation-subset.md`
的 5 处、课程 `units/notation-cheatsheet` 的教学话术钉死 ⇒ 清单见
`implicit-arguments.md` §3.4/§6。

**⚠️ 两条新缺口必须先补进台账（否则会被忘掉）**：X14 与 X15 现在只活在本文与两份设计里，
而本仓的纪律是「**缺口即测试**」——台账（`docs/gaps/ledger.jsonl` + `docs/gaps/repro/`）
由 `scripts/gap.py check` **强制执行**，且已接在 `scripts/soko gate` 第四步与 CI 里。
所以：

| 新条目 | 内容 | 复现件 | 备注 |
|---|---|---|---|
| **G-19 / WO-012** | **X14**（`∅` 嵌记法 + `by`） | `.sokonanoda` + `repro_expect: rejected`（今天的契约就是「必须被拒」）→ 修好后翻 `clean` 并关账 | 现有台账 24 条、最大 `G-18`；WO 文件最大 `WO-011` ⇒ 新号应为 **G-19/G-20**、**WO-012/WO-013**（开工前先 `python3 scripts/gap.py list` 复核，别撞号） |
| **G-20 / WO-013** | **X15**（LSP 丢报告） | `kind: tooling` + `.sh` 自断言（LSP over stdio 探针：0 诊断 + documentSymbol 非空 + hover 非 null）——照 `G-10`/`G-17` 的形制 | 修好后 `scripts/soko gate` 第四步必须仍绿 |

⇒ **I9 的第一件事**就是补这两条台账（含 `repro/` 复现件），**然后**才动代码。

---

#### R2 课程改写第一站：单元②（2026-09-19 回滚 → **2026-09-21 落地**）

> 第一次尝试（09-19）画布与解答都改写完了，却被一个判卷器残留卡住，整轮
> `git checkout` 回滚。**第二轮（09-21）把两个残留都修掉，改写落地**：画布
> 1 演示 `decl.checked` + 10 练习 `exercise.open`、解答 **10/10 `decl.checked`**，
> 记法全部换成 `∈ ⊆ ∅ {a} {a, b} = ≠ ∧ ↔ ¬`、证明全部 `by` 块，**零点名残留**。
> 下面保留两次根因分析——它们是这一轮最贵的产出。

**残留①：pp 文本有损（09-19 定位，09-19 已修）**

症状是报错说「期望 `Set.subset α A (Set.empty α)`，实际是 `Set.subset α A (Set.empty α)`」
——**两边看起来一模一样**。内核原文是 `expected Sort(0), actual (Set.[] $1)`。

根因：`apply_tactic` 的**记法失败重试**为了合一，把目标换成内核 pp 的规范形态
（`A ⊆ ∅` → `Set.subset α A (Set.empty α)`），**并用其中的子表达式填类型参数**
⇒ 后续子目标的**书写类型变成 pp 文本**。而 pp 按 `BinderStyle` 省隐式实参，
**`Eq` 的类型参数是显式的、也被省了**：`Eq.{1} (Set α) A ∅` → `Eq A (Set.empty α)`。
这段文本**读不回来**：重解析成 `Eq.{0} A (Set.empty α)`（`A` 落进类型位）⇒ 判定
声明合成失败 ⇒ 每条 `exact` 都假失败。触发条件很反直觉：**单独一个声明不炸，
后面只要还有声明就炸**（`judge_terms` 合成判定声明时会重编译整个前缀）⇒ 复现件
必须**至少两条声明**。

修法：`canonical_goal_type` 与 `canonical_goal_with_spec` **两条路**都加
`is_rereadable` 护栏（`Eq` 应用必须带齐三个实参、宇宙层级必须是具体数字），
不可回读就**退回源 AST**——行为与没有规范化时逐字相同，绝不因为"规范化失败"
把好文件判红。

**残留②：`≠` 的 delta 展开没有宇宙层级（09-21 定位并修复）**

`Ne` 是 `def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False`：
`intro h` 在 `{a} ≠ ∅` 上要看穿 `Ne` 就得展开定义体，而定义体里的 `u` 只能由
**调用点**定。可 `≠` 的两条路**都不带 `.{u}`**——源 AST 是**记法节点**、过一遍
内核 pp 是**裸名 `Ne`**（pp 省掉隐式宇宙参数）⇒ 展开出来的 `h` 是 `@Eq.{u} …`
（悬空变量），回读报 `unknown universe level u`，**报错点离根因很远**（它出现在
后面某条 `exact` 上）。同一族还有 `apply h`（`h : A ≠ B` 当函数用）与 Prop 档
（`p ≠ q`）。

修法：`by` 引擎算**层级提示**喂给 `spine::resolve_levels`——与
`elab_notation` 给记法求层级**同一条规则**（`{u}` 参数的类型是 `Sort u`，
所以 `u` = 那个类型参数的值的 sort；两条路不同解就是同一命题落进不同的常量应用）。
按**实参个数**分两档，两档都必须对：

| 形态 | 例 | 首实参是 | 层级 |
|---|---|---|---|
| 点名（实参 ≥ 形参） | `Ne (Set α) A B` | 那个类型参数自己 | `sort(首实参)`（**一步**；两步会得 `2`） |
| 记法（实参 < 形参） | `A ≠ B` | 该项的项 | `sort(type(首实参))`（**两步**；一步会得 `0`） |

算不出具体数字就**不填**（保持悬空变量 ⇒ 响亮报错），绝不静默错层级。
落点：`by.rs::level_hint_of`（4 个调用点：`intro` 的两条路、`apply` 的逐轮剥层、
`cases` 的 scrutinee 展开）+ `spine.rs::resolve_levels`（`unfold_head_once` /
`unfold_one` / `unfold_to_inductive` / `peel_pi_delta` 各多一个
`level_hint: Option<&str>` 参数）。

**另一条被 park 的改动**（仍未动）：`elab.rs` 的 `application_arg_expected`
（「应用实参也吃期望类型」，本意是消掉课程 **227 处** `Set.empty α`）在
`Eq.subst.{1} (Set α) (fun …) A ∅ h …` 这种**显式实参写在隐式位上**的调用里会把
`∅` 解成 `Set.empty A`（拿上一个实参当类型）⇒ 用 `if false &&` 关着，代码与 TODO
留在原地（`elab.rs` 的 `Expr::App` 臂）。它属于 IA-2（R2.5），不在本轮。

**同轮改的测试（C5 的 T1/T2，原子同轮）**

| # | 旧 | 新 | 现在钉住什么 |
|---|---|---|---|
| T1 | `the_shipped_course_still_uses_the_pointful_spelling` | `the_shipped_course_uses_the_library_notation` | ① 单元不出现 `infix`/`notation` 行（符号由 `lib/Set` 统一声明、随 `import` 传播）；② 六条记法签名真的在（含 `{a}`/`{a, b}` 与 `≠`）；③ **零点名残留**；④ 每条 `theorem` 的值位都是 `by` |
| T2 | `a_real_course_unit_grades_identically_in_notation` | `a_real_course_unit_grades_identically_in_either_spelling` | 方向反转：画布是记法版，夹具 `pointful_variant` 造点名版；两份 `exit 0` 且五元计数相等（N7 契约不变） |

新增回归：`notation.rs::inequality_delta_unfolding_carries_the_right_universe_level`
（点名对照 / `intro` 记法形态 / `apply` 点名形态 / Prop 档，四条都 `decl.checked`）。
设计记录见 `docs/design/notation-subset.md` §15。

**验证（本轮实测）**：`cargo test --workspace --locked` 全绿；课程门禁
**36 目标 · 329 checked · 99 open · 0 判负** 逐项不变、`--selftest` exit 0；
画布 1 checked + 10 open、解答 10/10 checked；`git diff --stat -- crates/kernel/` **空**。

---

#### R2 第二刀：`have`（L3.6）+ 诊断文案（**代码已落**）

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **L3.6 `have`** | 新 `Tactic::Have { name, ty, value: HaveValue }`（`HaveValue::Term` / `By`）；引擎新增 `NodeKind::Have { name, ty, value, body }`，组装成 **`let h : T := t; <rest>`**，`context_binders` 沿父链多收一个 binder | `ast.rs`、`parser.rs`、`by.rs`、`proof.rs`、`semantic.rs` | 4 条 e2e（项形式 / 链式 / 嵌套 `by` / 嵌套 `by` 里带 `cases` / `have` 后继续 `intro`）全 `decl.checked` |
| **嵌套 `by` 的缩进规则** | `have … := by` 的 tactic 序列用**缩进**界定：第一个列号 ≤ `have` 所在列的 tactic 属于**外层**块（与 `cases` 臂体同一条 layout）。没有它，嵌套 `by` 会把外层剩下的 tactic 全吞掉（`next_line_starts_a_tactic` 只看行号） | `parser.rs::parse_nested_tactic_sequence` | 测试里 `have … := by …` 之后紧跟同级缩进的 `exact`，全部通过 |
| **诊断文案（顺带修既有缺陷）** | 内核给的 `expected`/`actual` 是**折叠回望远镜的完整声明类型**（`Pi (A : Sort(0)), …`）——`exact h1 ha` 在目标 `B` 上就长这样，学习者读不懂。新 `mismatch_message` 改说人话：**期望 `<目标>`，实际是 `<值的类型>`**（后者用 `judge_infer` 拿值本身的类型，只在**错误路径**多问一次内核）。`exact` 与 `have` 共用 | `by.rs` | 新测试钉住：错误报在 `have` 那一行、文案含 `期望 \`B\` / 实际是 \`C\``、且**不含 `Pi (`** |
| **编辑器词表（补漏）** | `cases`（L3.1）与 `have`（L3.6）此前**漏在** `semantic::KEYWORDS` 之外——`is_tactic_keyword` 里有、词表里没有 ⇒ 编辑器里不着色也不补全，而守护测试只保证「TM 语法 = KEYWORDS」，**两边一起漏是看不见的** | `semantic.rs`、`sokonanoda.tmLanguage.json` | `extension.rs::tm_grammar_keywords_follow_the_single_source` 绿（35 passed） |

| **`=` 记法暴露的三个真 bug（同轮修）** | 用**课程形状**的证明（`A ∪ B = B ∪ A`，`apply Set.ext` + `intro` + `have … := by cases`）串起来时一次暴露三个，全部修掉：① **`apply` 把类型参数判成子目标**——目标 `A = B` 的源 AST 是 `Notation{=,[A,B]}`，**丢了 `Eq` 的类型参数 `α`**；而内核 pp 把 `Set.ext` 的 codomain 打成 `Eq A B`（也丢）⇒ 两边都不提 `α`，`α` 成了类型为 `Type 0` 的子目标，`intro x` 报「需要一个函数目标」。修法：**域是宇宙（非 `Prop`）的层永远算类型参数**（`is_universe_domain`）。② **`cases` 看不穿 def**——`cases h` on `h : x ∈ A ∪ B`，`∈`/`∪` 都是 def，要展开**两层**才露出 `Or`。修法：新 `spine::unfold_to_inductive`（逐层展开到头进归纳表，上限 4）。③ **展开时多贴一个实参**——`Set.union` 的值位是 `fun (α) (A) (B) => fun (x) => …`，`strip_lambdas` 把**里面那层** `fun (x)` 也剥了 ⇒ 展开出 `Or (A x) (B x) x`。修法：`strip_lambdas_n` 只剥参数表那几层 + 新 `beta_apply` 把结果上的应用归约进去（`unfold_one` 同时认「记法在函数位」与「pp 摊平的应用链」两种形状） | `by.rs`、`spine.rs`、`elab.rs`、`prelude.rs`、`walk.rs` | 新 e2e `notation.rs::a_course_shaped_proof_uses_equality_notation_apply_have_and_cases`（**一条证明串起三个 bug**）；课程门禁逐项不变 |

**验证（本轮实测）**：`cargo test --workspace --locked` **1189 passed / 0 failed**；
课程门禁 **36 目标 · 329 checked · 99 open · 0 判负** 逐项不变、`--selftest` exit 0；
`python3 scripts/gap.py check` exit 0；`git diff --stat -- crates/kernel/` **空**。

---

#### R2 第一刀：`=` 与 `≠` 进语言（L2.3 / L2.4a / L2.4b / L2.4c，**代码已落**）

> 用户原始要求里点名要的连接符，`=` 与 `≠` 是最后两个缺口（D2 的核心符号表）。
> **内核零改动**；改动在 `crates/front/**`、`editor/vscode/syntaxes/**`、`skills/**`。

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **L2.4a 词法** | `'='` 不跟 `>` 时产出 **`TokenKind::Sym("=")`**（不是新 token 种类）⇒ parser 算子表 / elab 记法展开 / 语义着色三处零改动。**S1 预言的坑当场命中**：`=` 进了内建记法表后，词法的**最长匹配**把 `=>` 切成 `=` + `>`，L1 prelude 第 9 行当场解析失败 ⇒ 新增 `parser::lexer_builtin_symbols()`（喂给词法的那一份，剔除 `=`） | `token.rs`、`parser.rs` | 词法测试改写（`lone_equals_is_an_error_with_position` → `lone_equals_is_the_equality_symbol_and_fat_arrow_is_untouched`）；回归测试 `notation.rs::fat_arrow_still_lexes_inside_a_fun_with_equality_available` |
| **L2.4b elab** | `=` 进 `BUILTIN_NOTATIONS`（`infix:50` → `Eq`）。**根因修复**：`elab_notation` 从前给常量传**空**宇宙层——现在按 `KnownName::universes()` 的长度分配，1 个宇宙参数时**从操作数类型的 sort 解出**（新 `level_text_of_sort`：`Prop`→`0`、`Type n`→`n+1`、`Sort n`→`n`；sort 由 `judge_infer` 给，不做文本猜测） | `elab.rs` | `A B : Prop` ⇒ `Eq.{0}`、`A B : Set α` ⇒ `Eq.{1}` 都 `decl.checked`；产出正是 L1.5 要的形状 ⇒ `rfl` 与课程门禁**逐项不变** |
| **L2.3 `Ne`** | `def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False` + `Ne.intro`，进 **L1 prelude 的 B9 族**（`deps: ["B2","EQ"]`）；`≠` 进 `BUILTIN_NOTATIONS`（`infix:50` → `Ne`） | `compile/prelude.rs`、`parser.rs` | `PRELUDE_NAMES` 47 → **49**（防漂移测试同步）；`A ≠ B` 在 Prop / `Set α` 两档都 `decl.checked`；`A ≠ B -> A = B -> False` 也过（`Ne` 是 def） |
| **编辑器着色（R-6）** | TM 的 `mathsymbols` 从前是**手写码点范围**（`U+2200–22FF` + `U+2A00–2AFF`）⇒ `↔ ¬ 𝒫 ᶜ ⁻¹' ×ˢ` **一律不着色**。现在改成**显式枚举**，由 `front::notation_input::notation_symbol_chars()`（单一真相源）生成；`=` 归 `operators` 规则（ASCII） | `syntaxes/sokonanoda.tmLanguage.json`、`notation_input.rs` | 新守护测试 `extension.rs::tm_grammar_math_symbols_follow_the_single_source`（逐字相等，且禁止码点范围——范围会静默漏符号） |
| **同步** | `skills/sokonanoda-teacher/SKILL.md` 的记法段（顺带修掉「第二刀未做」的**假话**——它 0.60.0 就做了）；`editor/vscode/CHANGELOG.md` 的 `[0.61.0]` 段 | `skills/`、`editor/vscode/` | `cargo test -p sokonanoda-cli --test skill --test dsh --test extension` 全绿 |

**为什么这一刀值**：课程里 `Eq.{1} (Set α) A B` 是**最大的一类噪音**，而且它挡住了
`∅`——`A = ∅` 里 `∅` 终于有期望类型（C4 第 5 条那 37 处 `Eq.{1} (Set …) … (Set.empty …)`
不再需要点名）。

**验证（本轮实测）**：`cargo test --workspace --locked` **1185 passed / 0 failed**；
课程门禁 **36 目标 · 329 checked · 99 open · 0 判负** 逐项不变、`--selftest` exit 0；
`git diff --stat -- crates/kernel/` **空**。

---

#### R2.5 第一片：G-20（X15）+ G-19（X14）修复 + 记法输入法 NI-1（同日，**代码已落**）

> 用户 D5 的两条要求（「像 lean4 一样 `\xxx` 替换」+「hover 提示怎么输入」）与
> 计划里 X15/X14 的处置。**内核零改动**；改动全在 `crates/front/**`、
> `crates/lsp/**`、`docs/gaps/**`。

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **G-20（= X15，NI-0）** | `QueryDoc::project_entry_compiled()`（入口模块 `ModuleStatus::Compiled` 才算闭包救回来了）；LSP 只在它为 `false` 时维持「parse 失败 ⇒ 无报告」的老契约；`diagnostics` 同判据 | `crates/front/src/query/mod.rs`、`crates/lsp/src/lib.rs` | 台账 G-20 + `WO-013` + 复现件 `G20-lsp-drops-rescued-report.sh`（修前 exit 0：假诊断 + hover/documentSymbol 全 `null`；修后 exit 1）。**两条回归测试**：`imported_notation_keeps_the_report_and_the_diagnostics_honest` / `a_genuinely_broken_entry_still_reports_the_parse_error` |
| **测试基建 bug（顺带）** | `testutil::lsp_pos` 按**字节差**算 LSP `character` ⇒ 含多字节符号的行上光标落到隔壁 token（实测：想 hover `⊗` 却 hover 到 `b`）。ASCII 夹具上两种算法恒等，所以一直没显形 | `crates/lsp/src/testutil.rs` | 改为按字符数；新 hover 测试第一次就抓到了它 |
| **NI-1 表** | `front::notation_input`：18 条（19 符号，`''` 与 Lean 一致地没有缩写）+ `input_for` / `symbol_for_abbreviation` / `input_hint` / `symbol_at` / `declared_notation_at` | `crates/front/src/notation_input.rs`（新） | 7 条单测 |
| **NI-1 词法** | `token::scan_notation_symbols` 重构为 `scan_notation_decls`（符号 **+ 展开目标**）的投影——使用库记法的文件单文件 parse 必然失败，展开目标只能靠词法扫描 | `crates/front/src/token.rs` | 词法测试 23 条仍绿 |
| **NI-1 hover** | 记法符号的 hover（符号 + 是否本文件声明 + 展开成什么 + **怎么输入** + 类型行），**插在关键字闸门之前** | `crates/lsp/src/lib.rs` | 修前：本文件声明的 `⊗` hover **完全静默**；现在 `⊗` → 展开 `myop`、内建 `∧` → `\and`、import 来的 `∈` → `\in`（三条测试） |
| **G-19（= X14，B0 前置）** | `by.rs` 的 `Tactic::Intro` 改成两级：先按**源 AST** 剥（零开销）；剥不动（头是 **def**）时**先用内核 pp 的规范形态再剥**（`canonical_goal_with_spec`），源级 delta 展开只作退路 | `crates/front/src/by.rs` | 台账 G-19 + `WO-012`；复现件 `G19-zero-ary-notation-applied.sokonanoda`：修前 exit 1、修后 **exit 0（7 条全 checked）**；回归测试 `notation.rs::a_zero_ary_notation_survives_delta_unfolding_in_a_by_block`（**改前 FAILED / 改后 ok**，含两条改前就过的对照组）。**为什么不是设计时猜的那两条**：源头不在渲染，而在**源级 delta 展开把记法操作数搬进了「被应用」的位置** |
| **课程叙事（顺带修）** | `units/notation-cheatsheet.sokonanoda` 的 ④ 段仍说「本文件自己声明 `∈ ⊆ ∪ ∅`、课程库没有」——W1 已把它们搬进 `lib/Set`，这是**同轮留下的假话**；同时补了 G-19 的边界说明 | `courses/set-theory/units/notation-cheatsheet.sokonanoda` | 课程门禁逐项不变 |

**验证（本轮实测）**：`cargo test --workspace --locked` **1180 passed / 0 failed**；
`cargo fmt … --check` exit 0；`cargo clippy -p front -p cli -p lsp --all-targets`
零 warning（仅冻结内核 62 条既有）；课程门禁 **36 目标 · 329 checked · 99 open ·
0 判负**、`--selftest` exit 0；`python3 scripts/gap.py check` + `selftest` exit 0
（**G-19/G-20 都判"已修"**）；`git diff --stat -- crates/kernel/` **空**。

---

#### R2 语言刀：`⟨a, b⟩` + 四层展开 + 三条**静默错**修复（2026-09-21，**代码已落**）

> 触发：单元③⑤ 的改写 subagent 报回四条"想写的 tactic 形状写不出来"。逐条查下去
> 发现**三条是判卷器的静默错**（不是缺功能），一条才是新语法。**内核零改动**；
> 改动在 `crates/front/**`、`editor/vscode/syntaxes/**`、`skills/**`。

| 项 | 症状（实测） | 根因 | 落点 |
|---|---|---|---|
| **`intro` 不改名**（H5） | `theorem t (A B : Set α) (h : A ⊆ B) : A ⊆ B := by intro y; intro hy; exact h y hy` ⇒ `` `exact` 判定失败：unknown identifier `x` `` | `Set.subset` 的体是 `forall (x : α), A x -> B x`；`intro y` 只把 binder 名换成 `y`，**体里还写着 `x`** ⇒ 目标里留着**悬空的 `x`**，报错落在后面那条 tactic 上 | 新 `spine::rename_free`（**捕获避免**：内层同名 binder 挡住）+ `by.rs` 的 intro 臂 |
| **代换不避捕获**（静默错） | `A ∈ 𝒫 B` 上 `intro x; intro hx; exact h x hx` ⇒ `期望 `A x`，实际是 `B x`` | `def powerset (α) (A) := fun (B : Set α) => subset α B A`：展开时 σ 里 `A := B`（外层集合）而 lambda 的 binder **也叫 `B`** ⇒ 按名字硬代换得 `subset α B B`，beta 一步成 `subset α A A`——**目标被悄悄换掉** | `spine::substitute` 的 Lambda/Forall/Let 臂 + `beta_apply`：新 `rename_bound_binders`（撞名就给 binder 换 `B'`） |
| **delta 展开只做一层** | `A ∈ 𝒫 B` 上 `intro` ⇒「需要一个函数目标」；`a ∈ B ∩ C` 上 `constructor` ⇒「头 `Set.inter` 不在归纳表里」——而目标明明是集合成员关系 | `peel_pi_delta` 只展开一层（`Set.mem → Set.powerset → Set.subset` 要三层）；`constructor`/`use` 只看目标头不展开 | 新 `spine::peel_pi_delta_n`（逐层、每层现算层级提示、用 `unfold_one` 以吃下 pp 摊平的应用链）+ `by.rs::peel_pi_delta_deep` / `ctor_tactic`（展开后**临时写回节点**再 `apply`，之后还原以保显示） |
| **短名展开断链** | `A ∈ 𝒫 B` 展开第二层拿到头 `subset`（`powerset` 的体里写的是短名）⇒ 查不到 | `DefTable` 只登记规范名 `Set.subset` | `walk.rs`：登记 def 时**顺带登记不冲突的短名**（先到先得，绝不猜） |
| **`use` 在记法目标上错位** | `use w` on `∃ (x : α), p x` ⇒ 剩下的子目标里留着悬空 `A` | `unify_spine` 把 binder 记法节点当「目标名 + 1 个操作数」，而模板 `Exists A p` 有 2 个实参 ⇒ 右对齐错位 | `spine::spine_with_notation` 与 `elab::head_and_args_notation`：binder 记法补**域**那一位（`∃` 的应用形态是 `Exists α (fun …)`） |
| **`query check` 与 `grade` 打架** | 单元⑤ 改写后 `query check` 报 `∈` 未声明（`failed[]` 非空），同一份文本 `grade` exit 0、计数 5/7 | `QueryDoc::check` 无条件把 `parse_error` 合成进 `failed`——闭包**救援成功**时那条诊断是中间产物（G-20 同一条判据，LSP 侧已修） | `query/mod.rs`：只在 `!project_entry_compiled()` 时报它 |
| **`⟨a, b⟩`（L2.7，新语法）** | —— | 用哪个构造子由**期望类型**决定（路线 C，不做合一） | 新 `Expr::AnonCtor` + `TokenKind::Langle/Rangle`（`⟨`/`⟩` 是**语法**，不进数学符号类——`lex_symbol` 是最大吞噬的，`⟨∅` 会并成一个 token）；elab 复用 `elab_notation`（前导参数补全 + 操作数期望类型传播），认 `And`/`Iff`/`Exists`/`Prod`/单构造子归纳 |

**又一条已知边界（R2 实测，未修，根因已定位到行）**：**操作数全是闭项**的集合记法
解不出前导类型参数——`#check (Set.univ Nat ∪ Set.univ Nat)` 报
`elab-notation-argument-unsolved`（`∈`/`=` 不受影响；只要有一个操作数是局部变量就没事）。
根因：**内核给闭项的类型是展开过的**——`infer_type_text(Set.univ Nat)` 回
`Nat -> Prop`（不是 `Set Nat`），于是模板 `Set α` 与实际 `Nat -> Prop` 头对不上，
`solve_prefix_args` 的路线① 拿不到 `α`。修法（下一刀）：`unify_extract` 在头名
不匹配时**展开模板的头一层**再试（`defs` 里就有 `Set` 的定义体）——需要把
`DefTable` 送进 `ElabCtx`（8 个构造点 + 一处借用冲突：`walk.rs` 里 ctx 借着
`&self.defs` 而后面要可变登记 DefInfo；正解是 `Rc<RefCell<DefTable>>` 或把
`defs` 显式传给记法路径，本轮**试过并回滚**，不半途留个坏借用）。
课程侧绕法：那一条练习写点名形式（单元④ 练习 8 已这么做）。

**另一条已知边界（R2 实测，**已在「R2 修边刀 ③」关闭**）**：`cases` 消去一个
**谓词里含 `≠`** 的 `∃`（`h : ∃ (U : Set α), ∀ A, A ⊆ U ∧ A ≠ U`）时，臂里的假设
类型会带 `Ne.{0}`——根因链是「根目标被 G-05 规范化成 pp 文本 → `Ne.{1}` 丢层级 →
`cases` 把 pp 形态写回假设（`canonicalize_binder_type`）」。当时修掉两段
（规范化文本补回裸名的宇宙层级 `restore_universe_levels`；`cases` 的字段名与实参
改用源类型）；**剩下那一段的修法**（把 `restore_universe_levels` 接到
`canonicalize_binder_type` 上）当时试过并回滚（撞 `elab-match-no-expected-type`），
**后来在 R2 修边刀里连同三条配套一起落地**——见下文
「R2 修边刀：判卷器的四条静默错」的 ③ 与「同时关掉上文那条已知边界」。

**`⟨a, b⟩` 的已知边界（明说）**：**嵌套**（`⟨a, ⟨b, h⟩⟩` 对 `∃ x, ∃ y, …`）今天
不支持——内层操作数的期望类型是 `(fun (x : α) => …) w`（`notation_operand_expected`
只代前导参数、不代**前一个操作数**），解不出内层构造子。课程里没有这种形状
（`∃` 全是单层）；撞上时用 `use a` / `use b` / `exact h` 分步写。要修就是
`notation_operand_expected` 累积「已 elaborate 的操作数」——与 park 着的
`application_arg_expected`（G-21）同一块地基。

**新增回归**（`crates/cli/tests/notation.rs`）：
`anonymous_constructors_pick_the_constructor_from_the_expected_type`、
`an_anonymous_constructor_without_an_expected_type_reports_its_own_code`（`#check` 位专用码）、
`intro_renames_the_bound_variable_in_the_rest_of_the_goal`（源级 `forall` + def 头 + `have` 三条）、
`goals_whose_head_is_a_def_unfold_through_several_layers`（三层 `𝒫` / 两层 `∩` / 记法目标 `use`）；
`crates/front/src/compile/tests.rs` 的 ErrorKind 穷尽表加了新变体。

**编辑器/技能同步**：TM 语法新增 `punctuation.section.anonctor`（`⟨`/`⟩` 是括号、
不是数学符号，所以**不进** `mathsymbols` 类——那条类由
`notation_input::notation_symbol_chars()` 逐字钉死）；`editor/vscode/CHANGELOG.md`
的 `[0.61.0]` 段与 `skills/sokonanoda-teacher/SKILL.md` 的 tactic 清单同轮更新。

**验证（本轮实测）**：`cargo test --workspace --locked` 全绿（`notation` 37 条、
`query` 19 条、`extension` 35 条）；课程门禁逐项不变；`git diff --stat -- crates/kernel/` **空**。

---

#### R2 主体收尾：卷 I 全量改写（2026-09-21，**进行中**）

> **先纠正一个记账错误**：第 110–112 轮的 STATUS/汇总把「卷 I 全绿（36 目标 /
> 328 checked / 99 open / 0 判负）」写成了「卷 I 全量改写完成」。**两者不是一回事**
> ——课程门禁只证明**每个目标判卷通过**，不证明**文本改写过**。第 113 轮末用
> R3 学到的「残留扫描」复查卷 I，实测仍有 **829 处**旧写法（`->` / `forall` /
> `And X Y` / `Or X Y` / `Not X` / `Iff X Y` / `Exists X (fun …)`）与 **50 个
> 值位不是 `by` 的声明**。**R2 真正交付过的只有：语言地基 + 单元② 一个试点**
> （由 `notation.rs::the_shipped_course_uses_the_library_notation` 守着）。

**这一轮（113–114）落地的**：

| 项 | 实测 |
|---|---|
| `lib/Set.sokonanoda` 改完（记法 + 证明全 `by`） | 23 `decl.checked`；`def` 体用 `→ ∀ ∧ ∨ ¬ =`；10 条定义展开引理走 `constructor` + `intro h; exact h`（`·` 聚焦**没进语法**，实测 parse 错——设计 N-12 的 L3.9 未实现） |
| 机械记法替换（脚本 + 门禁逐轮验收） | `lib/` 67 处、`units/` 画布 74 处、`units/solutions/` 376 处，合计 **517 处**；替换后门禁**逐轮都是 328 / 99 / 0** |
| 入门课练习占位统一成 `:= by sorry`（D3） | **104 处**（行内 `:= sorry` 48 + 续行 `sorry` 56）；23 个文件的判定事件流**逐字节相同**（只有绝对字节 offset 因行变长而平移）——设计 §1.2 的预测成立 |
| 剩下的 216 处（`Exists X (fun …)` 83、跨行 `And` 87、其余 46）+ ~10 条项模式证明 | 需判断力（嵌套 binder 记法/多行分组），已写手册 `docs/notes/course-lean-style/R2-full-rewrite-brief.md` 并派出 subagent |
| ⛔ `units/notation-cheatsheet*.sokonanoda` | **不动**：它**故意**把点名与记法并列（大纲 §4 的"第二遍"教学装置）。C1.5 的"重定位成速查表"是**内容改动**，与本轮风格改写分开做 |

**这一轮又反过来改了语言一处**（第三次了——改写的需求真的会长在语言上）：

**`cases` 的头解析必须认记法**。把 `lib/Set.sokonanoda` 的 `def union` 体从
`Or (A x) (B x)` 改成 `A x ∨ B x`（只为可读性、语义完全等价）之后，卷 I 门禁
**当场从 328/0 掉到 326/2**：两条 `cases h`（`h : x ∈ A ∪ B`、`hX : x ∈ {{a}, {a,b}}`）
报「被消去项不是归纳类型的值」。根因：`cases` 的 delta 展开把 `def` 的**体**
代进来，形态完全取决于**定义体怎么写**——`unfold_to_inductive` 之后 `cases` 用的是
只走 `Expr::App` 的 `spine_of`，记法节点没有 `Ident` 头 ⇒ 认不出 `Or`。
**修法**：改用 `spine_with_notation`（记法节点的头就是它的 `target`；源实参那条路
本来就在用它），落点 `by.rs::cases_tactic`。回归测试
`notation.rs::cases_sees_through_a_definition_body_written_with_notation`；
旧/新对照就是上面那次真实门禁（旧：326/2；新：328/0）。

> **这条修复是卷 I 改写的前置条件**：课程库一旦改用记法，所有"透过 def 看归纳"的
> `cases`/`match` 都会撞上它。不修就只能把库体留在点名形式——那等于放弃可读性目标。

**仍欠**：上述 216 处 + ~10 条项模式证明（subagent 在跑）；C1.3 的 294 条 hint
词汇；C1.5 的速查表重定位；C1.6/C1.7 的元数据与大纲同步。

---

#### R2 修边刀：判卷器的**四条静默错**（2026-09-21，**代码已落**）

> 触发：R2 全量改写跑完后，**36 个课程目标里只剩 `unit12-solution` 一个红**，
> 而它那 6 条判负的声明「怎么看都对」。把每一条缩到最小复现
> （一次 ~10s）之后发现：**6 个「写作错误」里 4 个是判卷器的静默错**，只有
> 2 个是真正的引擎边界。**内核零改动**；改动全在 `crates/front/src/{by,proof}.rs`
> + 回归测试。

| # | 症状（实测复现） | 根因 | 落点 |
|---|---|---|---|
| **①`apply` 的类型参数静默填错** | `apply Set.ext` 后子目标里的元素类型变成**另一个同名变量**：元素类型叫 `β`、上下文里另有一个 `α` ⇒ 子目标成 `y : α`，之后每条 `exact` 都报「期望 `C y`，实际是 `C y`」（**字面相同**）；上下文里**没有** `α`（元素类型叫 `γ`）⇒ 合成声明报 `unknown identifier α` | `Set.ext` 的 pp 签名把 `Eq` 的类型实参丢了（`Eq A B`）⇒ 类型参数 `α` **不在任何实参位上**（`unify_spine` 只认「codomain 实参位 = 层名」），旧代码按名字回退到「同名上下文变量」——而 `Set.ext` 的参数恰好就叫 `α` | 新 `solve_type_params` + `align_ident`：拿「**已填层的 domain ↔ 该层值的类型**」反推（`A : Set α` 已填成 `f '' A`，后者类型是 `Set β` ⇒ `α := β`）；只对会被判成类型参数的层做，缺项时才付一次 `judge_infer` |
| **②`cases` 在 `have … := by` 里丢字段** | `have h1 : T := by cases hy with \| intro x hx => …` ⇒ `` `have h1` 判定失败：构造子 `Exists.intro` 有 2 个字段，但这一支写了 1 个子模式 ``；同样的 `cases` 写在定理顶层却没事 | 嵌套 `by` 的判定要把组装好的项**打回源码文本**再判卷，而 `render_pattern` **无条件**给子模式加括号：`\| intro b hb =>` 渲染成 `\| intro (b hb) =>`，回读时变成「构造子 `intro` + **一个**子模式」 | `proof.rs::render_pattern`：只给**本身带子模式**的子模式加括号（原子子模式裸写） |
| **③`cases` 臂里裸 `Eq` 丢宇宙层级** | `cases h with \| intro b hb => exact Eq.trans…`（`hb : And (Eq.{1} β a b) …`）⇒ 整条声明被内核拒：「期望 `Sort(0)`，实际是 `$N`」，报错位置在**定理那一行**、离根因极远 | `cases` 把假设的书写类型换成内核 pp 形态，pp 丢掉隐式宇宙参数（`Eq.{1} β (f a) b` → 裸 `Eq (f a) b`）。这条类型**不只判定要用**——`match` 的组装与最终声明判定都从节点上读它；裸名按默认 `.{0}` elaborate ⇒ `Eq` 的 `α : Sort u` 拿 `u = 0` 去要一个 `Prop` | ① `by::def_shape` + `trusted_prelude_arity`：`Eq`/`Eq.refl`/`Eq.subst` 是受信任安装的 **axiom**（没有定义体）⇒ 从不进 `DefTable`，补一张 `(项参数个数, 宇宙参数个数)` 表（`(3,1)`/`(2,1)`/`(6,1)`，与 `PRELUDE_EQ_SRC` 逐字对应）；② `restore_universe_levels` 补回 **pp 丢掉的前导类型实参**（`@Eq.{1} (f a) b` 会被读成 `α := (f a)`——补成 `@Eq.{1} β (f a) b`）；③ 进 `fun`/`forall` 的体先把该层 binder 加进判定上下文（否则 `Eq (g b) c` 里的 `b` 让 `judge_infer` 报 `unknown identifier b`，**同一层里只有第一个 `Eq` 被修好**）；④ 把 `restore_universe_levels` **接到 `canonicalize_binder_type` 上**（见下） |
| **④`cases` 字段类型留 beta redex** | 依赖字段的类型里嵌进参数代换后的 lambda（`Exists` 的 `h : p w` 代成 `(fun (b : β) => …) b`） | 内核**不做 beta 转换**，`apply` 的子目标早有 `beta_normalize`，`cases` 这条漏了 | `by.rs::cases_tactic` 字段类型：`beta_normalize(&substitute(…))`（与 `apply` 同一条纪律） |
| **⑤`cases` 的参数代换取**源记法**形态 ⇒ 前导类型参数撞名静默错** | `cases` 消去 `y ∈ (Function.comp α β γ g f) '' A` 后，臂里的假设类型成了 `And (A x) (@Eq.{1} **β** (Function.comp α β γ g f x) y)`——`β` 是 `Set.image` 的形参名、这里该是 `γ`；于是每条 `exact` 都报「期望 `…`，实际是 `…`」把同一条命题写成两种形态 | 源类型里的**记法节点**不带前导类型参数（`elab` 期才算得出来，源 AST 里没有），`spine::unfold_one` 只能把操作数**右对齐**到形参 ⇒ 定义体里提到前导参数的地方**留着定义自己的 binder 名**，在调用点按「同名上下文变量」解析（`Set.image` 的形参就叫 `α`/`β`，上下文里也有 `α`/`β`）——与 ① 同一类**按名字回退**的静默错 | `by.rs::cases_tactic`：实参**优先取规范形态**（内核 pp：点名 + 全实参 + 无记法），源形态只在规范头对不上时兜底；pp 丢的隐式宇宙参数已由 ③（写回节点前补齐）覆盖 |

**同时关掉上文那条已知边界**：R2 语言刀里记的「`cases` 消去谓词含 `≠` 的 `∃`
⇒ 臂里假设带 `Ne.{0}`」——真正的修法（把 `restore_universe_levels` 接到
`canonicalize_binder_type` 上）当时**试过并回滚**，撞的是 `elab-match-no-expected-type`；
这次撞上同一个坎才发现**回滚的那一刀本身是对的，缺的是上面 ③ 的三条配套**
（axiom 的宇宙参数表 + 补前导实参 + lambda binder 入判定上下文）。补上之后，
pp 文本进节点之前层级就是齐的，`match` 组装读到的不再是裸名。

**新增回归**（`crates/cli/tests/notation.rs`，41 条）：
`apply_fills_a_type_parameter_from_the_goal_not_from_a_same_named_variable`（撞名 + 无 `α` 两种）、
`cases_inside_a_have_block_keeps_every_constructor_field`、
`cases_arms_restore_universe_levels_for_bare_equality_hypotheses`、
`cases_uses_the_canonical_type_so_notation_prefix_params_do_not_capture_context_names`。
四条都做过**旧/新二进制对照**（旧 `target/release` 全红、新构建全绿），不是「写了个能过的测试」。

**验证（本轮实测）**：`cargo test --workspace --locked` 全绿（`notation` **41**、
`sokonanoda-front --lib` **661**、`cli` 100）；`scripts/soko gate` 全绿
（fmt + clippy + test + playground 锚点 + 课程门禁 + `gap.py check`）；
课程门禁 **36 目标 / 0 判负 / checked 328 / open 99 / exit 0**——**卷 I 首次全绿**。
`git diff --stat -- crates/kernel/` **空**。

**unit12-solution 的课程侧改法（5 处，都是「`cases` 有边界」的绕法）**：
`cases hC y hy`（被消去项是**应用**）→ 用 `Exists.elim` 项；
`cases hmem`/`cases hmem2`（消去 **`have` 绑定的 `∈ 像` 假设**，要两层 delta 才到
`Exists`）→ 先 `have hex : Exists … := hmem` 再 `cases hex`（**点名** `Exists`
的中间假设）；
嵌套 `cases` 里的内层（P1 第二半）→ 内层改写 `Exists.elim`；
`cases e`（书写类型是 binder 记法 `∃`）→ 先 `have e1 : Exists … := e` 再 `cases e1`。
这四类都进了 `unit12-solution.sokonanoda` 的注释（它们是**判卷器的实测边界**，
不是风格偏好）。

**仍欠（下一轮）**：R2.5（隐参数路线 C / 记法可输入性收尾）与 R3（入门课 52 文件 +
`playground` + 13 处计数钉住的测试）。卷 I 这一站**已完成**。

#### C2.5 as-built：删自建骨架（2026-09-21，用户拍板**删**）

**用户原话（本轮拍板）**：设计里这一行标着「需拍板（推荐删）」，用户选**删**。
施工手册 `docs/notes/course-lean-style/C25-delete-skeletons-brief.md`（§1 逐字删什么留什么、
§2 文件表、§3 叙事怎么改、§4 计数会变、§5 本轮不改教学法）。

| 项 | 实测 |
|---|---|
| 删掉 | ①④⑧ 的 `axiom And`(4)+`axiom Or`(3)、⑨⑩⑪ 的 `axiom And`(4)+**`inductive Or … end` 整块**、`unit11-project/Logic` 的 `axiom And`(4)、`playground` 的 7 条。**保留** `True`/`False` 的 `axiom`（单元① 的 `axiom` 教学例子） |
| 范围 | **34 个文件**（11 单元 × 中英 × 画布/解答的相应部分 + 项目 4 文件 + playground） |
| 叙事同轮 | 单元①「逻辑骨架」段 → 「`axiom` 是给你看公理长什么样；`∧ ∨` 及构造子 **prelude 自带**」；单元⑨「9.1 `Or`：从公理升级为真归纳」→ **「9.1 `Or` 的消去子」**（动机失效必须换）；单元⑪ 的「单元① 的 `Or` 只是公理」对比段换掉；`playground` 的「公理都齐了」「看 `axiom Or.inl`」等悬空引用修好 |
| 规范副本退役 | `course/shared/{And,Or}.sokonanoda` **删除**，`course_shared.rs` 的 `AND_COPIES`/`OR_COPIES` 两张表与文件头口径同步删除（`NAT_COPIES` 8 份照旧守）；`Demo.sokonanoda` 改成**只 import `Nat`**，And/Or 两条演示改用 prelude 真归纳写（**演示名不变 ⇒ CI 断言不变**） |
| 顺带暴露的真话 | 项位裸名 `inr` 在真归纳上不存在（G-02 起的构造子命名空间）⇒ 必须 `Or.inr`；**模式位** `| inl a =>` 仍可用 |
| 计数（内核重取） | 六个画布 `checked` 各减删除数（unit1 13→6、u4 14→7、u8 14→10、u9 13→8、u10 7→2、u11 7→2），**`open` 一个没动**；总计 **checked 87→54、open 66 不变**；四处钉子同步 |
| 验证 | 34 文件逐个 `grade` exit 0 诊断 0；`course`/`course_status`/`course_shared`/`cli` **114 条全绿**；**CN/EN 22 对逐字节一致**；子 agent 的仓库外探针证明 `constructor`/`left`/`right`/`cases` 现在可用 |
| 本轮**不做** | 把 `constructor`/`cases` 写进 ①④⑧ 的**教学**——那是下一轮的教学决定（§C2.6）；删除只负责让它们**变得可用** |

---

#### R3 入门课改写：记法 + tactic（2026-09-21，**课程代码已落**）

**范围与实际落点**：入门课 **44 个教学文件**（11 单元 × CN/EN × 画布/解答）
+ `course/unit11-project/`（4 个文件，**不属于** `course.json` 的 11 单元，但它
是 `import` 教学与 `scripts/soko grade` 的活样例，之前整目录漏改）
+ 规范副本 `course/shared/Nat.sokonanoda`。

| 项 | 结果 |
|---|---|
| 代码连接符 `And a b`→`a ∧ b`、`Or`→`∨`、`Not`→`¬`、`->`→`→`、`forall`→`∀`、`Exists`→`∃`（单元⑧ 本文件加 `binder_notation "∃" => Exists`） | 全 44 文件 + unit11-project 完成；**逐文件 `grade` 退出码 0、诊断 0** |
| **解答**的证明体 → `by` tactic 块 | 全 20 个解答文件完成（`def` 是函数定义、不是证明，保持项模式——单元③⑥⑦ 的解答因此 0 个 `by`） |
| 单元④ C2.6 结构专项（§8 计划） | 四条全做：补练习 `by_ex5`（教 `have`）、加演示 `demo_by_have`、删解答里 9 条早期草稿遗留（`h_s`/`Pfam`/`Qfam`/`f_dep`/`qfam_true`/`val_apply_imp`/`val_apply_dep`/`by_ex7`/`by_ex8`）、「首期五个 tactic」措辞改成实际白名单 |
| CN/EN **代码逐字节一致** | 每对文件用 `course_shared.rs::code_only` 的口径复核（只差 `--` 注释） |

**计数**（内核重取，不手算）：单元④ 画布 `(13,5,0)` → **`(14,6,0)`**（+1 演示、
+1 练习）；其余 10 个单元**逐个复核与 `GOLDEN` 一致**（纯记法改写不产生事件）。
课程总计 **checked 86 → 87、open 65 → 66**，三处钉子同步：`course.rs` 的 `GOLDEN`、
`course_status.rs`（2 处）、`cli.rs` 的 warm-cache 总计。

**这一轮反过来改语言的两处**（用户预判的「改写会对语言本身提要求」应验；
两处都在 `crates/front`，**kernel 一行不动**）：

| # | 症状（实测） | 根因 | 落点 |
|---|---|---|---|
| **A** | 目标含 `Sort u` / `Eq.{u}` 时，`:= by …` 报 `universe variable `u` is not declared in this declaration`——**宇宙多态定理根本写不了 tactic**（卷 I 的 `Set.{u}` 遍地都是） | by 引擎的 `spec_of`/`spec_of_for_judge` 硬写 `OpenGoalSpec.universe = Vec::new()`，声明的宇宙参数从没进过判定合成声明 | 把 `universe` 从 `walk.rs` 的 `def`/`theorem`（`example` 传 `&[]`）一路带到 `run_by → run_tactics → {apply,cases,ctor,exact}_tactic → {judge,judge_with_levels,spec_of,spec_of_for_judge}`。as-built 见 `docs/design/by-tactics.md` §12；测试 `by_block_carries_the_declaration_universe_parameters`；活样例 = 单元⑤ 解答的 `theorem Eq.symm {u} : {α : Sort u} → … := by …` |
| **B** | `have bc : B ∨ C := …` 之后 `cases bc` 报「被匹配项必须是一个书写类型为 `Or …` 的局部变量」——逼学习者把**块内假设**的类型写成点名形式 | `match` 降低要从**书写类型**取参数化归纳的参数实参（`src_spine`），而它不认记法节点；声明参数/`intro` 进来的假设有「归一成内核 pp」那条兜底，`have` 引入的没有 | `elab.rs::src_spine` 增 `Expr::Notation` 分支：记法的**源像**就是 `target` 那条 spine（中缀 `[lhs, rhs]`、前缀 `[rhs]`、后缀 `[lhs]`、零元 `[]`）。测试 `cases_splits_a_have_bound_hypothesis_written_with_notation` |

**同时改掉的「规范副本」**：`course/shared/Nat.sokonanoda` 的 `->` → `→`
（`course_shared.rs` 逐字守住 8 份拷贝，改规范不动副本必红——这正是那道守卫的用途）。
`examples/py-nat.sokonanoda` **不动**（不在本轮范围）：单元⑥ 画布那句
「照抄 examples/py-nat.sokonanoda」随之改成「照抄……只把箭头换成本课的 `→`」，
不再声称逐字。

**已知边界（写进课程注释或设计，不是绕法偏好）**：
① `judge_infer`（`apply`/`cases` 推断被应用函数类型）**仍不带宇宙参数**——它合成的是
`#check fun (α : Sort u) => …`，`#check` 片段没有地方声明 `u`；要修得换合成策略
（包进临时 `def` 再取类型），记为下一刀；
② 单元①④⑧ 的 `And`/`Or` 是**公理** ⇒ 那里没有 `constructor`/`cases`/`left`/`right`
（要真归纳），课程继续点名 `And.intro`/`Or.inl` + `apply`；
③ C2.5（删自建骨架、换成 prelude 真归纳）**仍未拍板**（N-10 推荐保留：共享骨架是
教学点）。

**仍欠（下一轮）**：C2.9 叙事陈词清尾（各单元里「首期五个 tactic」「没有 ↔ 记号」
「Or 从公理升级」这类句子的剩余部分）+ `course/README.md` 的写死计数
（24/12/8、232/2974、44 文件总行数）+ C2.11 的其余 4 处课程外文档 + R2.5。


---

## 10. 明确不做（有理由，不是「没时间」）

| # | 不做 | 理由 |
|---|---|---|
| N-1 | 隐式实参 / 元变量 / 一般合一 | ~~D2 明确不做~~ **已翻案（D6，2026-09-19 用户追加要求）⇒ 走路线 C 立项为 P1**。**仍然不做的是「元变量 + 一般合一」本身**：内核 `Expr`/`Value` 没有元变量槽（碰内核冻结），`elab_expr` 也在内核环境外运行（probe = 整段前缀重编译）。路线 C 用「风格对齐 + 唯一确定」拿到 95% 的可读性收益而**零内核改动**；`h.1` 投影、`∃ x, p x` 省类型（N-8）仍归 N 表。详见 `docs/design/implicit-arguments.md` |
| N-2 | `h.1` / `h.2` 投影记法 | 同上；`cases`/`obtain` 已能拆。S2 实测 `exact h.2` ⇒ `unknown identifier h.2` |
| N-3 | ~~内核 pp 的 print-back~~ **仍然不做，但理由换了（2026-09-21）** | ~~硬规则 1（内核冻结）~~ **作废**（内核已解冻）。**真正的理由**：内核的记法打印是**死代码**（`ExportFile.notations` 全仓库无一处 insert），而 `pp_expr` 同时是 `#check`/`#reduce`/`#print` 的出口 ⇒ 改它就动 `--json` 的字节。**要做的不是这一项**，而是 front 侧的**显示边界重写**（线 C，权威设计 `docs/design/notation-aware-printing.md` §3）——两者名字像、位置完全不同 |
| N-4 | `simp` / `push_neg` / `tauto` / 通用引理集 | S2 §4.2 判定：需要目标改写地基 + 引理集，性价比不成立；只做**窄版 `rw`**（单条等式、首个出现、非依赖 motive） |
| N-5 | `funext` | **做不了**：prelude 没有 `funext` 公理（课程用 `Set.ext` 公理替代，`lib/Set.sokonanoda:79`） |
| N-6 | `induction` | 课程只有 `Nat.rec` **1 处**；`cases` 落地后按需再评估 |
| N-7 | `specialize` / `apply … at` / `injection` / `calc` | 课程 **0 处**需求（S2 §4.1 普查） |
| N-8 | `∃ x, p x`（省略 binder 类型） | 需要一般合一（N-1 的直接后果）；课程一律写 `∃ x : α, p x` |
| N-9 | 把 `Exists` 搬进 prelude | 卷 I 走 L2 层（`lib/Exists`）、入门课单元⑧ 自己声明——**这是教学设计**，不是缺口 |
| N-10 | 把 `And`/`Or` 换成 prelude 的（入门课） | 同上；共享骨架是教学点（C2.5 待拍板，推荐保留） |
| N-11 | 嵌套 `by`（`exact by …`） | S2 实测不可用；但 `fun (x) => by …` **可用**，改写够用 |
| N-12 | 组合子 `<;>` / `all_goals` / `repeat` / `try` | 课程 0 处；只做 `·` 聚焦（L3.9） |

---

## 11. as-built：记法规则重建 + 基础类型隐式实参（2026-09-21）

> 用户两条指令（原文见 `REQUIREMENTS.md` §9 同日条）：①「重新设置一个 courses 的
> 规则，至少 notation 都要换掉，lib 和正文都换掉……你先实现一个检查脚本，然后一个
> 文件一个文件过」；②「基础类型的隐变量也可以尝试和 lean 对齐……`Eq.{1}` 直接就是
> 一个等于号」。
>
> 本轮的定形：**记法是硬规则（脚本判红）；隐式实参是能力（能省则省，省不动留
> 边界）**。

### 11.1 规则 = 脚本（可执行）

- 新增 **`scripts/notation-lint.py`**：旧写法检查器，覆盖
  `courses/set-theory/`（lib + units + solutions）、`course/`、`playground.sokonanoda`；
  **代码与注释都算**；`units/notation-cheatsheet*.sokonanoda` 整文件豁免；
  行内 `-- soko:notation-ok: <理由>` 的行豁免。`--json` / `--list` / `--root`。
- 接进 **`scripts/soko gate`**（第四步之后）与 **`ci.yml`** 的 `test` job。
- 施工手册：`docs/notes/course-lean-style/notation-rewrite-brief.md`。

### 11.2 语言侧（`crates/front`，内核零改动）

| 项 | 内容 |
|---|---|
| prelude 隐式化 | `False.rec/False.elim`、`And.left/right/elim`、`Or.elim`、`Not.intro/elim`、`absurd`、`Ne.intro`、`Iff.intro/mp/mpr/refl/symm/trans`、`Eq.symm/trans`、`congrArg`、`Eq.mp/mpr`、`cast` 的前导类型/命题参数改成 `{}`；**构造子**（`And.intro`/`Or.inl`/`Or.inr`/…）按 Lean 语义把归纳参数当隐式（注册表 `implicit_prefix = params.len()`）。`Eq`/`Eq.refl`/`Eq.subst` 保持前缀 0（见边界）。 |
| 签名表换成源级文本 | `KnownName::Decl` 新增 `signature: Option<String>`（`render_expr(ty)`）；`try_implicit_application` 不再 `judge_infer`（那会重编译前缀 ⇒ prelude 自举**无限递归**，实测栈溢出），改为解析存的源文本。 |
| prelude 安装护栏 | `PreludeInstallGuard`：安装期间关闭隐式插入（prelude 源文本一律写全实参，语义无损）。 |
| 旧式写全的兼容 | 实参个数 > 显式层数 ⇒ 判为「旧式逐位写全」、在 `try_implicit_application` 里一次装完（按 `layers` 对齐），**不递归到前缀**（否则 `And.right a` 会被误判成隐式短写）。既有语料逐字节不变。 |
| 路线 ② | `implicit::solve_prefix` 增「由期望类型反解」：参数只出现在结果类型里时（`Or.inl` 的 `B`、`False.elim` 的 `C`、`And.left` 的域已在 ① 覆盖），用结果模板与 `expected_src` 头部匹配。 |

### 11.3 明说的边界（不假装已对齐）

1. **宇宙多态的等式族证明项**（`Eq.refl`/`Eq.symm`/`Eq.trans`/`Eq.subst`/`congrArg`/
   `Eq.mp/mpr`/`cast`）：应用路径不做**宇宙层级推断** ⇒ 仍写显式宇宙与参数
   （`Eq.symm.{1} α a b h`、`congrArg.{1} f h`）。这是独立的一刀。
2. **`congrArg` 参数顺序**按 Lean 改成 `{α β} {a b} (f) (h)`（**契约变更**）：
   旧顺序 `congrArg.{1} α β f a b h` 判红。
3. **`Set.univ α`**：没有记法，零元应用（`Set.univ`）不在覆盖内。
4. **期望类型是 def 时**路线 ② 会做 delta 展开（2026-09-21 补：`a ∈ A ∪ B` →
   `Or …`，`Or.inl h` 因此可省参），但**仍有几档解不出**：`intro` 派生出来的目标/
   假设（期望类型传不到）、`Exists`-headed def（`Function.Surjective` 之后）、
   嵌套 `Exists.elim` 的 motive、复合记法操作数（`{aa} ∩ {bb}`、字面 λ 的
   `''`/`⁻¹'`）、`And.left h x` 这类续应用。这些按脚本的 `-- soko:notation-ok`
   标记为边界。
5. **`by rfl` 已能认 `=` 记法目标**（2026-09-21 修：`rfl` 在源 AST 认不出记法时
   改走内核 pp 的规范形态；顺带修了 `canonical_goal_with_spec` 的「先判可回读、
   后补层级」顺序与 `is_rereadable` 在部分应用上误判 Eq 元数）。
