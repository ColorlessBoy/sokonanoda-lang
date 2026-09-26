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


---

> **过程部分已归档** ✓（6 节：现状实测 / 分期与依赖 / subagent 分工 / 风险登记 / R1 as-built 逐轮记录）
> ⇒ `docs/archive/course-lean-style-design-process-2026-09-26.md.gz`（`gunzip -c … | less` ✓）。**归档 ≠ 销毁** ✓；本文件只留**仍生效的契约与边界** ✓。
