# sokonanoda-lang 架构与内核深度理解

> 读者对象：刚接手本仓库的 agent / 工程师。
> 本文回答三个问题：**这是什么**、**代码在哪、各管什么**、**一条 `.sokonanoda` 文件如何变成"被完整内核检查过的声明"**。
> 配套文档：`STATUS.md`（当前状态与进度日志，先读）、`ROADMAP.md`（里程碑与原则）、
> `docs/protocol.md`（反馈/事件协议）、`docs/notes/inductive.md`（归纳类型语义）、
> `docs/notes/research.md`（外部调研）、`docs/design/infrastructure.md`（基础设施方案脑暴）。

---

## 1. 30 秒版

`sokonanoda-lang` 是一个**独立、自包含的 Lean 4 教学编译器栈**，跑在 Rust 上：

- **底层**是完整移植的 [sokonanoda](https://github.com/intgrah/sokonanoda) 内核（Lean 4 类型检查器，含 inductive / quot / proof irrelevance / 完整 conv/eval / pretty printer），几乎不改动；
- **上层**是受限的教学方言 `.sokonanoda`（语法白名单 = 课程），有 lexer/parser/小型 elaborator/CLI/REPL/`#prove` 草稿；
- 产品目标是「用户 + code agent 看着同一个 `.sokonanoda` 文件协作学证明」，但 L0 阶段**只做编译器层**，不依赖 VS Code、不依赖任何 agent、不调用任何官方 Lean 工具链。

一句话：**完整内核能力 + 受限教学通道 + 可被 agent 消费的结构化反馈**。

```
 .sokonanoda 源码
   │  parse（front）
   ▼
 AST(FolFile/Command/Expr)     带 span
   │  elab（front → builder）
   ▼
 kernel Declar 序列（arena 内，按依赖顺序）
   │  EnvBuilder.finish()
   ▼
 ExportFile（完整环境 + 名字/层级/表达式 intern 表）
   │  try_check_declar / infer_closed_type / reduce_closed / with_pp
   ▼
 CheckEvent[] + CompileError[]      ← 文本行（人）/ JSON Lines（agent）
```

---

## 2. 仓库地图

```text
sokonanoda-lang/
├── Cargo.toml              # workspace：kernel / front / cli
├── README.md               # 英文快速上手
├── ROADMAP.md              # 中文里程碑 + 执行清单（当前状态入口）
├── NOTICE.md               # 上游归属（sokonanoda@7b51784，Apache-2.0）
├── .github/workflows/ci.yml  # 离线 cargo test + 课程语料 + JSON 事件自检
├── docs/                   # 开发者文档（完整地图见 docs/README.md）
│   ├── architecture.md     # 本文（§8 内核 gotchas 必读）
│   ├── protocol.md         # CLI --json 事件 + LSP 协议与自定义请求
│   └── design/ notes/      # 设计文档与调研笔记
├── crates/
│   ├── kernel/             # 完整 sokonanoda 内核快照 + 少量教学适配（见 §6）
│   │   ├── src/{expr,level,name,value,env,util,parser,...}.rs
│   │   ├── src/builder.rs  # 【新增】EnvBuilder：内存中造声明 → ExportFile
│   │   ├── src/quote.rs    # 【改动】infer_closed_type / reduce_closed
│   │   ├── tests/arena.rs  # Lean Kernel Arena 集成测试（需环境变量）
│   │   ├── tests/memory_api.rs  # M0 内存 API 验收
│   │   └── test_resources/ # 上游 NDJSON fixtures（部分测试的输入）
│   ├── front/              # .sokonanoda 前端
│   │   ├── src/{lib.rs,proof.rs,parser.rs,judge.rs,suggest.rs,session.rs}
│   │   ├── src/compile/    # 单文件流水线（§4.2）：mod.rs（入口+闭包装配）
│   │   │                   #   ├── walk.rs         命令走查：每命令一个方法 → PendingOp
│   │   │                   #   ├── kernel_phase.rs  内核 check-then-add + 签名/cutoff + 报告装配
│   │   │                   #   ├── units.rs         闭包级装配（单元/区间/报告切分，§4.5）
│   │   │                   #   └── elab.rs/goals.rs/report.rs/event.rs/error.rs/prelude.rs/cache.rs
│   │   ├── src/project/    # 【新增】import 闭包：模块名/解析/清单/拓扑序/报告（§4.5）
│   │   └── src/query/      # 内核真相查询层（§4.6）：mod.rs + state/pos/types/project.rs
│   ├── cli/                # `sokonanoda` 二进制（文件检查 / repl / --json / query / build）
│   │   ├── src/main.rs
│   │   └── tests/{cli.rs,imports.rs,query.rs,examples.rs}
│   └── lsp/                # `sokonanoda-lsp`：tower-lsp 服务器（诊断/hover/符号/练习状态/项目状态）
├── editor/vscode/          # VS Code 扩展（per-target VSIX + universal，Marketplace 上架）
│                           #   extension.js（接线）+ project-tree.js（项目树渲染）+ server.js
└── examples/               # 入库课程文件（lesson-01/02、fol-basics、py-fol-core、py-nat）
```

测试分层、实时数量与"哪条测试守哪条契约"的权威地图在 **`docs/TESTING.md`**
（本文只讲结构，不重复数字——数字会漂。

---

## 3. 不可动摇的原则（来自 ROADMAP，代码与之对齐）

1. **kernel 保持完整**：受限的是前端教学语法，不是内核能力。
2. **无官方工具依赖**：运行/构建/测试都不调 `lean`/`lake`/`lean4export`/`leanc`/`elan`；所有语料入库。
3. **教学语法是真实 Lean 4 的子集**：在 `.sokonanoda` 里学会的写法放进官方 Lean 依然合法。
4. **语法白名单即课程**：parser 只认课程引入过的语法点；新增语法必须伴随课程单元。
   记法（`infix`/`infixl`/`infixr`/`notation`，0.59.0；`prefix`/`postfix`/`binder_notation`/
   `scoped`，0.60.0）**是糖不是新语义**：它只把源文本重写成既有的 `App` 形状，点名形式
   永久可用（`docs/design/notation-subset.md`）。**层级算术 `u+1`**（`Sort (u+1)` /
   `Eq.{u+1}` / `Type (u+1)`，0.61.0）是真语法增量，白名单 =
   `docs/design/type-level-syntax.md` §5（parser + elab，内核零改动）。
   （**prelude 名字不属于语法白名单**：L1 的 30 个名字是 `PRELUDE_NAMES` 里的受信任
   声明，parser 零改动——见 §5.4.1。）
5. **分层推进**：L0（编译器）→ L1（服务）→ L2（编辑器）→ L3（agent 协作），每层只依赖下一层公开接口。
6. **TDD 与重复测试**：单元、kernel 端到端、CLI 三层重复覆盖同一行为。
7. **反馈即功能**：类型、化简、打印、环境、错误都结构化输出，人和模型都能无文档驱动工具。

---

## 4. 一条 `.sokonanoda` 文件的旅程

### 4.1 词法/语法（`crates/front/src/lib.rs`）

- `Lexer`：手工字符扫描，产出 `TokenKind`（`Ident/Num/Hole/Str/Sym/Colon/ColonEq/Arrow/Plus/FatArrow/Forall/At/括号/逗号/Eof`）。**`=`（相等）是 `Sym("=")`**（0.61.0）：`'='` 分支不跟 `>` 时产出它，跟 `>` 仍是 `FatArrow`；因为词法的符号匹配是**最长匹配**且排在专用分支之前，`=` **不能**进喂给词法的内建符号表（`parser::lexer_builtin_symbols()` 剔除它，否则 `=>` 被切成 `=` + `>`）。标识符允许 ASCII 字母/`_`/非 ASCII（≥0x80），续字符还含 `' ! ? .`；`#check` 这类命令被 lex 成 `#` 前缀的 Ident。**数学符号是独立 token**（`Sym`，0.59.0）：`U+2200–U+22FF`（运算符）与 `U+2A00–U+2AFF`（补充运算符）加 `\`，最大咬合；字符串字面量（`Str`）只用于记法声明里的符号文本（`" ∈ "`），未闭合报 `unterminated-string`（span 在开引号）。
- `--` 是行注释；`sorry` 是 Hole（未完成练习/占位符；旧的 `???` 已于 2026-09-07 移除）。
- `Parser` → `FolFile { commands: Vec<Command> }`。命令：`def` / **`abbrev`**（G-08：与 `def` **同语义**的拼写，共用 `parse_def`，设计 `docs/design/abbrev.md`）/ `theorem` / `example` / `axiom` / `inductive ... end` / `#check` / `#reduce` / `#print` / **记法声明 `infix:N` / `infixl:N` / `infixr:N` / `notation`**（0.59.0）/ **一元记法 `prefix:N` / `postfix:N`**（0.60.0）/ **binder 记法 `binder_notation "∃" => Exists`**（第三刀）/ **作用域命令 `namespace <name>` / `end <name>` / `open <name>` / `open scoped <name>`**（0.60.0 + 第三刀 + 第二刀：`open` 的 `(a b)` / `hiding a b` / `renaming a => b` 三条互斥子句、只影响一条命令的 `open <name> … in <命令>`、跨 `import` 的 `export <name> [<子句>]`）/ **`scoped <记法命令>`**（第三刀）。
- 表达式 AST（`Expr`）：`Sort(Prop/Type/Sort n/Level u)`（源码里的 `Type n` 解析成 `Sort (n+1)`，是 Lean 记法的糖）、`Ident`、`UniverseApp name.{u,...}`、`Num`、`Hole`、`App`、`Lambda`、`Forall`、`Arrow`、`Plus`、`Let`（`let x : T := v; body`）、`Match`（`match e with | <pattern> [if <guard>] => body`；pattern = `_` / 绑定名 / 构造子（可嵌套）/ Nat 字面量）、**`Notation`**（`lhs symbol rhs` 与零元 `symbol`；0.59.0；第三刀加 `alternatives` 字段做**重载**候选表）、**`SetLiteral`**（`{a}` / `{a, b}`，第三刀——**新语法**，展开成点名形式 `Set.singleton` / `Set.pair`）。
- **语言内建记法**（0.61.0，设计 `docs/design/course-lean-style.md` L2.2–L2.4b）：`∧ ∨ ↔ ¬`
  （`And`/`Or`/`Iff`/`Not`）、**`=`（`Eq`）与 `≠`（`Ne`）**——**任何文件零声明可用，
  且不能被重声明**。为什么是内建表而不是写进 prelude：`install_l1_prelude` 只认
  `Axiom`/`Def`/`InductiveBlock`，记法命令会被**静默忽略**。`=`/`≠` 各带一个宇宙参数，
  `elab_notation` **按 `KnownName::universes()` 的长度分配层级**，1 个时从**操作数类型的
  sort** 解出（`Prop`→`0`、`Type n`→`n+1`、`Sort n`→`n`）⇒ `A B : Prop` 给 `Eq.{0}`、
  `A B : Set α` 给 `Eq.{1}`（与点名 `Eq.{1} (Set α) A B` 判卷一致）。
  **同一个层级也要在 `by` 引擎的 delta 展开里解一遍**：`intro h` 在 `a ≠ b` 上
  要看穿 `Ne`，而 `≠` 的两条路都不带 `.{u}`（源 AST 是记法节点、内核 pp 是裸名
  `Ne`）⇒ `by.rs::level_hint_of` 按**操作数的 sort** 算层级提示，喂给
  `spine::resolve_levels`（点名形态取首实参的 sort、记法形态取首实参**类型的**
  sort；算不出具体数字就不填，悬空变量会响亮报错）。见
  `docs/design/course-lean-style.md` §9 残留②。
  `→` 不在表里（函数空间不是常量，走**词法别名**）。
- **匿名构造子 `⟨a, b⟩`**（0.61.0，设计 `docs/design/course-lean-style.md` L2.7）：
  **新语法**（`Expr::AnonCtor` + 专用 token `⟨`/`⟩`），用哪个构造子由**期望类型**
  决定——`And`→`And.intro`、`Iff`→`Iff.intro`、`Exists`→`Exists.intro`、
  `Prod`→`Prod.mk`、单构造子归纳→它的构造子（多构造子/非归纳头报专用码
  `elab-anon-ctor-no-expected-type`）。展开复用**记法路径**（前导参数补全 +
  操作数期望类型传播），所以 `⟨w, hw⟩` 在 `∃ (x : α), p x` 上解得出 `α` 与 `p`。
  `⟨`/`⟩` **不是数学符号**（不进 `is_math_symbol`）：`lex_symbol` 是最大吞噬的，
  进了类 `⟨∅` 会并成一个 token；它们也不是标识符字符，是**括号**。
- **用户自定义记法**（0.59.0，设计 `docs/design/notation-subset.md`，台账 G-04 第一刀）：
  `infix:N " ∈ " => Set.mem`（`infixl` = 左结合、`infixr` = 右结合、零元用
  `notation "∅" => Set.empty`）。规则 N1–N7 摘要：符号**必须是独立 token**
  （声明里的文本去掉首尾空白后不能全是标识符字符）；优先级 `N ∈ 1..=1000`
  插在 `parse_arrow`（最松）与 `parse_app`（最紧）之间的梯子上，`infix:N` 的
  左右是 `N`/`N+1`、`infixl:N` 是 `N`/`N+1`（同级左结合）、`infixr:N` 是
  `N+1`/`N`；**文件内作用域**（声明之后、同文件生效，不跨 `import`）；
  **记法不是声明**——不发任何事件、不进声明表、不进 goal 视图。展开在 elab 内
  **源到源**重写成 `App` 形状，并**自己补前导类型参数**（只做裸变量匹配，不引入
  元变量/一般合一）：先从操作数类型解、再从期望类型解，解不出报
  `elab-notation-argument-unsolved`。**兼容护城河**：点名形式永久可用，且省 `α`
  的点名写法（`Set.mem a A`）今天被内核拒绝、改后仍被拒绝。
  `SemanticKind::ALL` 与 `tm_scope` 表**逐字节不变**（记法符号在语义层分类为
  `Keyword`；未声明的符号不产生 run，所以目标文本里的 `⊢` 仍是普通 run）。
  **第二刀（0.60.0，已落地，同一篇设计 §10–§12）**：`prefix:N`/`postfix:N` 两条
  一元命令；**声明驱动的词法**（`scan_notation_symbols` + `tokenize_with_symbols`：
  源码里声明过的符号在**本文件**里被读成 `Sym`，最长匹配、标识符内部也断开 ⇒
  `𝒫 A`、`Aᶜ`、`f '' A`、`f ⁻¹' B`、`A ×ˢ B` 都读得出来）；**跨 `import` 的记法
  传播**（闭包加载器**先收 `import` 边、先访问依赖，再解析自己**，把依赖导出的
  记法表当继承表——`project::is_project_source` 是配套的分发判据，它让"入口单独
  parse 失败但写了 import"也走闭包）；**一元记法的优先级**（`prefix:N` 的操作数按
  `parse_operators(N)`、`postfix:N` 在爬升里 `N >= min_precedence` 才吸收 ⇒ N 越大
  绑得越紧）；展开期用 `judge::judge_type_of` 读目标签名（**不能**用 `judge_infer`
  ——它剥 binder 时目标本身是函数会错位）。
  **第三刀（已落地，同一篇设计 §13–§14）**：**binder 记法** `binder_notation "∃" => Exists`
  （`∃ (x : α), p` ⇒ `Exists α (fun (x : α) => p)`；两段式 `∃ x ∈ s, p` ⇒
  `Exists α (fun (x : α) => And (x ∈ s) p)`，`∀ x ∈ s, p` ⇒ `∀ x, x ∈ s -> p`；binder
  类型只从**标注**或 **guard** 来——不做元变量/合一，解不出报 `elab-binder-notation-unsolved`；
  命令拼写是 `binder_notation` 而不是 `notation-binder`，因为 `-` 不是标识符字符）；
  **记法重载**（同符号**同形状**多目标，按**期望类型的结果类型**选候选；选不出
  `elab-notation-ambiguous` / `elab-notation-no-candidate`；重声明 import 来的符号仍是错误）；
  **`scoped` / `open scoped`**（作用域名 = 声明点所在 namespace 的全前缀；`open scoped`
  **只**开记法不开名字）；**集合字面量** `{a}` / `{a, b}`（`set-literal-shape`；
  目标缺失报 `elab-set-literal-unknown-target`）；**前缀记法实参位免括号** `f 𝒫 A`
  （后缀**明确不做**：`f Aᶜ` 今天读作 `(f A)ᶜ`，改了会悄悄重分组）。**仍未做**：
  源码级 print-back（内核冻结下销不掉）、`notation3`/依赖 binder、一般隐式实参推断、
  `open scoped` 的子命名空间传播、编辑器词表同步（设计 §13 逐条给了理由）。
- **namespace / open**（0.60.0，设计 `docs/design/namespace-open.md`，台账 G-05）：
  `namespace A` … `end A` 之间的**声明名**自动带前缀（`def mem` ⇒ 全局名
  `A.mem`；名字本身带点则拼接 `A.Set.mem`）——前缀在 **parser** 里落定
  （`Command::Def{name}` 一出门就是全局名，于是 `top_level_def_spans`、闭包级
  重名检查、hover/goto 回填全部零改动）。`open A` 把 `A.` 加进**可省略前缀**
  集合：引用 `x` 的候选顺序是「当前命名空间链从内到外（`A.B.x` → `A.x`）→
  精确 `x` → `open` 的前缀（按 open 顺序）」，第一个在 `known` 里能解析的胜出
  （`compile/scope.rs` 是唯一实现，`resolve_known` 是唯一解析点）。三条命令
  **都不是声明**（零事件、不进声明表，与 `import`/记法同族）；`end` 错配与
  文件尾未闭合各有专用 parse 码（`parse-namespace-mismatch` /
  `parse-namespace-unclosed`）。作用域：`namespace` 是词法块（parser 校验闭合），
  `open` 是**文件**（单元切换处 reset，不跨 `import`）——但被导入模块的**全局名**
  本来就可见，所以入口里的 `open Set` 对依赖声明的 `Set.mem` 有效。
  判卷合成（`judge.rs`）走 `parse_fragment`（容忍片段里未闭合的 `namespace`），
  且**用了 namespace/open 的文件**里 `by` 块的根目标先过一遍内核 pp
  （`judge_render_type`）——`apply` 的 spine 对齐是文本比较，源里的短名与内核
  渲染的全名必须同源。
- **open 的子句 / 局部 open / export**（第二刀，同一篇设计 §N7/N8）：
  `open A (a b)`（only）、`open A hiding a b`、`open A renaming a => b`
  （三条**互斥**，先过滤后改名；原短名被改名占位 ⇒ 不再是候选）；
  `open A … in <命令>` 只对紧跟的那一条**叶子命令**（声明或
  `#check`/`#reduce`/`#print`）生效，`Walk` 用 mark/rollback 撤销
  （`compile/scope.rs` 的 `opens_mark`/`rollback_opens`），并且给合成前缀补一行
  **源码原文**的 open 头（`by` 引擎要看到同一个作用域）；`export A [<子句>]`
  文件内与 `open` **逐字相同**，额外记进 `Walk::exports`，单元切换（`reset`）
  时重放 ⇒ **跨 `import`**（`open` 不跨，N5）。子句与 `export` 同样是
  **作用域命令**（零事件、不进声明表）；`open … in <声明>` 包住的声明由
  `ast::effective_commands` 展开给所有**声明级** pass（`top_level_def_spans`、
  `GoalTemplates`、着色、警告）——它照样是声明，只是作用域多一层。
  **遮蔽警告**（`open-shadowed-name`，语法级、只看本文件）：两个 `open` 给出
  同一个短名、或短名与根上的同名声明撞车时给一条 warning（不是 error），
  解析顺序（N4）照旧静默取第一个。
- 值位关键字：只有 `by <tactic 序列>`（tactic 之间用 `;` **或换行**分隔，0.51.0）（`Expr::By`，进内核前由 `crates/front/src/by.rs`
降级为 lambda）。历史：值位 `funapply`（0.22.0 移除）与 `funintro`（0.27.0 移除）
均已删除，见 `docs/design/remove-funintro.md`。
- **局部绑定 `let`**（term 关键字，Phase 1，同 `fun`/`forall` 挂 `parse_expr`）：
  `let x : T := v; body`。v1 要求显式类型注解（缺注解 / `let x := v` 报
  `elab-untyped-binder`，message/hint 定制为 `let` 写法）；`let` 与
  `(fun (x : T) => body) v` 内核等价（zeta），判定完全交给 kernel。
  无注解 `let` 仍是 Phase 2（见 `docs/design/elaborator-let-match.md`）。
- **`match` 分情况**（term 关键字，Phase 2，同 `fun`/`let` 挂 `parse_expr`）：
  `match e with | <pattern> [if <guard>] => body | ...`。模式支持通配 `_`、绑定
  变量、**嵌套构造子**（`some (succ k)`）、**Nat 字面量**（`0`/`1`/… 脱糖为
  `succ^k zero`）与 **Bool 守卫**（`if cond`，假则落到后续 arm）；arm **有序、
  首个匹配者胜**，同一构造子可写多条。编译器把模式源到源 canonical 化成
  「每构造子一条 arm、参数全为绑定」，再复用既有 lowering（不手搓 de Bruijn）；
  见 `docs/design/match-patterns.md`。被匹配项是**源内
  `inductive`**（分支用裸构造子名）**或 prelude 内建 `Nat`**（分支用点号名
  `Nat.zero`/`Nat.succ`；prelude 以受信任归纳块安装，含 `Nat.rec`），每个构造子
  结果类型必须已知（否则 `elab-match-no-expected-type`）；降低为显式
  `<Ind>.rec.{level} motive minor... scrutinee`（level 由结果类型的 Sort 推出，
  显式宇宙实例是内核接受的必要条件）。**递归归纳已支持**：递归构造子字段后
  自动插入归纳假设 binder（`ih`、`ih2`…，类型为结果类型 R），branch 直接引用，
  递归无需自引用。**依赖 motive**：当结果类型 `R` 含被匹配的**裸局部变量**
  `x`（如 `P n`）时，motive = `fun t => R[x:=t]`，分支期望 = `R[x:=<构造子项>]`、
  IH 类型 = `R[x:=<递归字段>]`（否则保持常量 motive；见
  `docs/design/match-dependent-motive.md`）。覆盖率/顺序/未知构造子等错误码见
  `docs/protocol.md`，判定仍完全交给 kernel（见 `docs/design/match.md`）。
- **参数化归纳声明**（非带索引，`docs/design/parameterized-inductives.md`）：
  `inductive Option (A : Type) : Type` 在名字后、`:` 前解析零或多个 binder
  作为类型参数（复用 `(A : Type)` / `{A : Type}` binder 语法）；**参数与 ctor
  字段都吃多名字 binder 组**（`(A B : Prop)`、`{A B : Type}`，0.56.0 起，
  `parser.rs::parse_inductive_binders`；`(A)` 这种无类型组仍是显式报错）；参数对类型、
  每个构造子类型与显式 `rec`/`iota` 均在作用域内，参数个数交给内核
  （`num_params`，字段数严格为 `telescope − num_params`）。无显式 `rec` 时
  前端按内核期望形状自动派生递归子（**参数在最外层**）。`match` 对参数化
  归纳的参数实例从 scrutinee 的**书写源类型**取；scrutinee 不是带参数类型的
  局部量 → `elab-match-parameterized-unsupported`。**带索引归纳**（0.47.0：
  `inductive Vec (A : Type) : Nat -> Type`，索引 = `ty` 在 params 之外的 Pi 望远镜；
  派生 recursor 的 motive = `forall indices, Ind params indices -> Sort`、major 在索引之后；
  `match` 取 scrutinee 书写类型的索引实参；结果类型依赖索引不在 v1，见
  `docs/design/indexed-inductives.md`）。**索引 + 字段写在结果箭头链里**
  （`ctor ps (n : Nat) : P n -> P (Nat.succ n)`）自 0.56.0 起也能自动派生
  （此前只有具名字段能派生，课程因此手写 `rec`/`iota`，见 H6-C）；
  带索引归纳（`num_indices > 0`）与宇宙多态参数（`{u}` 级参数）仍不支持。
- **声明级 binder**（官方 Lean 风格）：`theorem f (a : A) (h : B a) : C := v` 在 parser 里降级为 `ty = Forall{binders → C}`、`val = Lambda{binders → v}`（`parser.rs::wrap_decl_binders`）；`by` 引擎把声明 binder 作为初始上下文（`run_by` 的 `initial_binders`），`:= sorry` 的剩余目标直接是 `C`。
- **命名箭头**：`(x : A) -> B` = 带 binder 的 `forall`；`{x : A} -> B` = 隐式 binder 的 forall；`A -> B -> C` = 匿名 binder 右结合 Pi。Pi（`A -> B`/`forall`）的 binder 必须**带显式类型**；lambda 的 binder 在**有期望望远镜**或**应用位置**（从实参类型，0.45.0）时可省略（`docs/design/elaborator-let-match.md` as-built）。
- span 全程保留（offset/line/column），诊断带行列。

### 4.2 Elaboration（`crates/front/src/compile/`）

`compile_fol(file) -> CompileOutput { events: Vec<CheckEvent>, errors: Vec<CompileError> }`：

1. 开一个 `stumpalo::Arena`，建 `EnvBuilder`（kernel 的 `builder.rs`）。
2. 若文件里没有显式 `inductive Nat ... end` 块，就 `install_prelude` 内置最小 Nat 基元（受信任归纳块 `Nat.zero`/`Nat.succ` + 派生 `Nat.rec`，并把 `Nat` 登记进 `match` 的 `InductiveTable`；见 §5.4）。
3. 顺序处理每条命令：
   - `def/theorem/example/axiom` → `build_*` 把 AST elaborate 成 kernel `Declar`，`builder.add_declar` 入表（记录每条声明在环境里的索引），随后 push `PendingOp`。
   - `#check/#reduce` → 先 elaborate 表达式并记住 `decl_before`（当前声明数），稍后用 `EnvLimit::ByIndex(decl_before)` 检查，保证 `#check` 只看到它之前的声明。
   - `inductive ... end` → `install_inductive_block`：先加 `Inductive`，再逐个加 `Constructor`，有 `rec` 则加 `Recursor`（带 `RecRule` 列表，每条 iota 规则按构造子索引绑定）；**无 `rec` 时自动派生**等价的 `RecDecl` + iota 规则（构造子类型 elaborate 之后，判据镜像内核 `large_elim_test`，见 §8 gotcha 0b）。
   - `example : T := sorry` → 不建声明，直接产出 `CheckEvent::ExerciseOpen`。
4. 若 elaboration 阶段已有错误，直接返回（不碰 kernel）。
5. 否则 `builder.finish()` 得到 `ExportFile`，设 `pp_options.proofs = true`（打印证明项本体而不是 `_`），然后逐个执行 PendingOp：
   - 声明：`env.try_check_declar(&declar)`（panic 包装成 `Result<(), CheckError>`），成功产 `decl.checked` / `example.checked`。
   - `#check`：`tc.infer_closed_type(expr)` + `pp.pp_expr(ty)` → `TypeChecked{text}`。
   - `#reduce`：`tc.reduce_closed(expr)`（kernel 新增的 deep reduce）+ pretty print → `Reduced{text}`。
   - `#print`：`env.with_pp(pp.pp_declar(ptr))` → `Printed{name,text}` 或 "unknown declaration" 错误。

**阶段 ↔ 模块（2026-09-18 拆分，只动位置不动语义）**：第 3 步 = `compile/walk.rs`
（`Walk` 持可变累加器、`CmdCtx` 持每命令派生的前缀/模板/信任位，每个 `Command`
变体一个方法）；第 5 步 = `compile/kernel_phase.rs`（`finish_pass(Walked)`：内核
check-then-add → 事件/错误 → 每命令签名与 early cutoff → 报告装配）；`compile/mod.rs`
只剩单文件入口（`compile_fol`/`check_document`）、闭包装配与助手函数；多单元装配在
`compile/units.rs`。搬移的验收方式见 `docs/TESTING.md`「二进制对拍」。

### 4.3 事件与错误（协议的第一版实现）

`CheckEvent`：`DeclarationChecked/ExampleChecked/TypeChecked/Reduced/Printed/ExerciseOpen`。

`CompileError { message, span, kind: ErrorKind }`；`ErrorKind` 细分为 `elab-unknown-identifier`、`elab-universe-arity`、`kernel-rejected` 等稳定 code，每个 kind 带一条教学 `hint()`（中文，直接贴在编辑器诊断里）。词法/语法错误是 `front::Diagnostic`（stage=parse，`unexpected-token`/`unexpected-eof`，也有 hint）。CLI 人类视图输出 `line:col: error[code]: message`；`--json` 输出 JSON Lines（见 `docs/protocol.md`）。

### 4.4 CLI / REPL（`crates/cli/src/main.rs`）

- `sokonanoda <file>` / `sokonanoda -`：批处理检查，人类可读输出。
- `sokonanoda --json <file>`：每条事件一行 JSON（agent/service 视图）。
- `sokonanoda repl`：逐行累积 buffer，整体重新 `parse + compile_fol`（最小"增量"模型 = 声明累加）；支持 `#check/#reduce/#print/#env/#help`。
- `#prove <goal>`：进入证明草稿（见 §5.5），`intro/exact/apply/assumption/lambda/done`（值位 `by` tactic：intro/exact/apply/assumption/rfl/match/sorry）。
- **编译结果缓存（0.48.0，0.49.0 共享化）**：`crates/front/src/compile/cache.rs`（LSP+CLI 共用） 把内核产出的
  `DocumentReport` 以 `(编译器版本, prelude 模式, 源文本)` 的稳定哈希落盘；打开
  未变文档直接复用，编辑则照走 Session 增量。诊断由 `report_diagnostics` 统一
  构造（命中与重编一致）。见 `docs/design/compile-cache.md`。
- `sokonanoda-lsp`：编辑器路径的**唯一反馈通道**（文件无 `#` 命令）——publishDiagnostics、hover（表达式类型 / `sorry` 的目标）、documentSymbol、codeLens（练习状态）、quick-fix `引入 N 个 binder`（把 `sorry` 变成 `fun (x : T) => sorry`；I13-S1 由 `intro` 改名）。

### 4.5 多文件项目（0.57.0，I16：`import` + `sokonanoda.toml`）

一个文件只要**没有** `import`，走的仍是 §4.4 的单文件路径（逐字节不变），而且
**从不发现/读取 `sokonanoda.toml`**——单文件就该像脚本一样直接跑（契约与测试见
`docs/design/imports-and-projects.md` §4.4b 与 `crates/cli/tests/single_file_vs_project.rs`）。
第一行出现 `import Foo.Bar` 后，编译单元从「一个文件」升级为「入口 + import 闭包」：

1. **解析**：`parse` 把 `import` 收成前置命令（`Command::Import`，占位 span）；
   违规形态在 parse 阶段就报（`import-malformed` / `import-not-a-valid-module-name`
   / `import-must-precede-declarations`）。
2. **定位模块根**：`project::plan_project(entry, src, root_override)`——
   `--root` 显式指定 > 最近的 `sokonanoda.toml`（向上走到 `.git`/HOME 即停）>
   入口文件所在目录（**无清单也能用 import**，这是与真 Lean 的有意分歧）。
3. **装载闭包**：`project::resolve` 后序遍历依赖（`VisitOutcome` 检出环），
   得到拓扑序 `SourceUnit` 列表：依赖在前、入口最后；每个模块名 = 相对模块根的
   路径（`Foo/Bar.sokonanoda` → `Foo.Bar`，`-` 不是模块名字符）。
4. **一次编译**：`compile::check::compile_all_units` 在**同一个 arena + 同一个
   `EnvBuilder`** 里按序跑完所有 unit 的命令——import 的命令先于本地命令入环境，
   所以内核 `EnvLimit` 下标不受影响，**kernel 一行未改**。
5. **归属**：错误**与警告**都按**命令下标**（`CompileOutput.error_cmds` /
   `warning_cmds`）而不是 span 归属到文件（不同文件的偏移会撞车），`split_report`
   再按 unit 的 `cmd` 区间还原成每个文件的 `DocumentReport` 与事件流；闭包级检查
   （重名 / prelude 冲突）与依赖阻断（`import-dependency-failed`）都在这一步。
   三个平行数组的不变量（`events`↔`event_cmds`、`errors`↔`error_cmds`、
   `warnings`↔`warning_cmds`）各自带一个 `push_*` 入口，**永不失配**——内核终审的
   `redundant-sorry` 是 pass 2 现算的 warning，0.58.0 合并轮之前它在项目模式下
   会被丢掉（`split_report` 当时只重算语法级 warning）。
6. **缓存**：`ProjectPlan::digest(options)` = 拓扑序上每个模块的 (名字, 源,
   imports) + **入口路径** + prelude 模式的稳定哈希（格式串 `soko.project-iface/2`）；
   依赖改动必然改摘要。**键里没有任何文件系统属性**（T-A02 起不再是可执行文件的
   mtime），条目**按模块存**（跨文件跳转要读模块表）。as-built 见
   `docs/design/compile-cache.md` §8。
7. **对外视图**：`query::QueryDoc::project_view()` 把这次编译的闭包状态派生成
   `ProjectView`（根 / 清单来源 / 拓扑序模块表 + 每模块 `status`
   （`compiled` / `load-failed` / `blocked`）/ 项目级诊断 / 计数）——**只读派生，
   不重跑内核**。三个传输共用它：CLI `query project`、MCP `project`、LSP
   `soko/project`（VS Code「项目」树渲染它）。设计 = `docs/design/project-view.md`。

消费方：CLI（`--root`/`--no-project`）、`query`（项目模式）、LSP（多文档 +
跨文件 `definition`/`references`/`rename`）、`build`（暖缓存）、`course`
（有 `import` 的单元走闭包；无 `import` 的单元仍走单文件）。设计全文与错误码表见
`docs/design/imports-and-projects.md`，协议见 `docs/protocol.md`。

**判据前缀（closure judge prefix）**：`match` 的宇宙层级、`by` tactic 的
`apply`/`exact`、无注解 binder 的推断都走"合成一个前缀文件再问内核"
（`judge_infer(prefix_src, …)`）。项目模式下前缀**必须**包含依赖的声明文本，
否则内核看不见被导入的名字——实测两个症状：入口里 `match` 导入的归纳类型报
`elab-match-no-expected-type`、`by apply And.intro`（导入的公理）报
`elab-tactic-failed: unknown identifier`。实现：`run_pass` 按拓扑序预计算
`closure_prefixes`（各依赖源码去掉 `import` 行后相接），单文件模式不构造
（A1 逐字节不变）。闭包前缀同一条也喂给**建议与探针**：`QueryDoc::judge_prefix(offset)` 是真相层入口，
`suggest_with`（quick-fix）与 `probe_sub_goal_types_with`（子洞期望类型）都接它；
`run_pass` 里 `GoalTemplates`（refine/intro 的构造子索引）也改成按"拓扑序前缀 +
本单元"的命令表构建——否则项目入口的 refine 建议会凭空消失（这三层是一个根因，
2026-09-18 一次修完，守护见 `docs/TESTING.md` §7b）。

**编辑器里的依赖编辑（未落盘）**：LSP 把**所有打开文档的当前文本**做成"内存覆盖"
（`load_closure_with_overlay`，按 `canonicalize` 后的路径匹配），改依赖时下游文档用
同一份覆盖重编译并重发诊断——所以未保存的依赖改动对入口可见，覆盖也进闭包摘要。
诊断**只在真的变化时**才 publish（`Doc::published`）。测试多文档必须用
`testutil::notify_with_drain`：一次通知可能连发多条诊断，先等通知再读 socket 会死锁
（`docs/TESTING.md` §5.7）。

---

## 5. kernel 内部（`crates/kernel`）——它为什么快、各模块做什么

### 5.1 总体设计：arena + hash-consing + 闭包求值

上游 sokonanoda 的核心卖点是**用闭包替换显式替换**：β 归约不再 O(n) 遍历替换，而是 O(1) 环境扩展；再叠加大量 hash-consing / 指针相等 / 缓存。在 Lean Kernel Arena 基准里相对官方内核约 **10–100x**（详见外层 `lean-kernel-arena-benchmark.md`）。

- **分配**：`stumpalo::Arena`（全局 arena）+ `bumpalo::Bump`（会话 bump），全局 `mimalloc`，小向量 `smallvec`，哈希 `hashbrown + rustc-hash/fx`。
- **interner（Dag）**：`Name/Level/Expr/BigUint/String` 全部按结构 hash-cons，指针相等 ≈ 结构相等。`ExprPtr` 把 **自由变量个数编码进指针 tag**，许多遍历能立刻短路。
- **Expr AST**（`expr.rs`）：`Var/Sort/Const/App/Pi/Lambda/Let/Proj/NatLit/StringLit`，多数带 `hash` 与 `fv_mask`（自由变量位图，用于环境剪枝）。`BinderStyle`（default/implicit/strictImplicit/instImplicit）**只影响打印**，不影响类型检查。
- **Level AST**（`level.rs`）：`Zero/Succ/Max/IMax/Param`；`simplify`、`leq`、`subst` 是内核自身实现（不自带 SAT/求解器）。
- **Name AST**（`name.rs`）：`Anon/Str/Num` 层级名；`NameNode` 附 `decl_idx`（环境里的槽位）与 `nat_red` 标记（见 5.4）。
- **声明**（`env.rs`）：`Declar::{Axiom, Quot, Theorem, Definition, Opaque, Inductive, Constructor, Recursor}`；`InductiveData/ConstructorData/RecursorData/RecRule` 描述归纳块。环境是 `FxIndexMap<NamePtr, Declar>`，靠 `EnvLimit::{Empty, ByIndex, ByName, PpUnlimited}` 用 `cutoff` 控制"可见前缀"，因此**可先整表解析、再按线程切可见切片**。

### 5.2 值模型与求值（`value.rs` + `eval.rs`）

值（`Value`）与表达式分离，全部 arena 化：

- `Env` = 值链（`env_extend` O(1)）；`Ctx` = 类型链（记录 binder 类型，供推断模式用）；`Closure { env, ctx, body }` 捕获环境。
- `Spine` = 消除器链表（`App(v)` / `Proj{ty,idx}`），头 + spine 表示「未展开的刚体」。
- `RigidHead`：`BVar/Axiom/Ctor/Recursor/QuotConst/Inductive`；`Unfold`：需要 delta 展开的定义头（含 `OnceCell` 记忆 head value）。
- `Value::{Rigid, Unfold, Lam, Pi, Sort, NatLit, StrLit, Thunk}`；`Thunk` 惰性求值表达式，`Unfold` 惰性展开定义。
- **eval 主循环**按 Expr 分发（`eval_no_cache`）：闭项查 `(env, expr)` 记忆化缓存；`App` 先求函数 WHNF 再 `apply`；`apply_closure` = 环境扩展 + 求值（推断模式用 `ctx_extend`）。
- **delta**：`unfold_value_go`（1437 起）——对 `Unfold` 头先试**原生 Nat 快路径**（见 5.4），不行再取 head value、把 spine 逐 elim 应用。对未满参数或可延迟情形有 `nat_red_defer`。
- **iota**：`iota_value`（1496）带正缓存 `iota_cache` 与负缓存 `iota_stuck`；`fire_recursor`/`fire_quot`/`try_k_reduce`/`try_struct_eta_reduce`/`nat_rec_natlit` 覆盖 recursor 在构造子、K 规则（小消除）、结构 eta、nat 字面量上的归约。
- **WHNF 记忆**：`whnf_head`/`note_whnf`/`store_lookup` + `global_value_cache`（结构身份键）；探测预算 `PROBE_CAP` 防指数爆炸（见 `conv.rs` 注释）。
- **教学新增** `deep_reduce`（`eval.rs` 957 行附近，lang 特有）：WHNF 后递归进入构造子/消除子参数，让 `s (Nat.rec ...)` 这类"构造子参数里的递归"也能继续算——`#reduce` 用；常规 kernel 检查只需 WHNF。

### 5.3 类型检查（`infer.rs` / `conv.rs` / `quote.rs`）

- `TypeChecker { ctx, env, tc_cache, arena, declar_info, nat_extension }` 是检查器主体；`TcCtx` 负责 arena 内的名字/层级/表达式构造与读写、各类缓存（type/conv/whnf/iota/subst/inst…）。
- `infer_value(flag, depth, env, ctx, e)`：单向类型推断 + 检查，`InferFlag::{InferOnly, Check}` 控制是否做昂贵的一致性检查（如 lambda 的 binder 类型、App 实参类型）。结果带 `type_cache`（按 `(env,e)` 键）+ `CheckScope` 判断是否可复用。
- `check_declar_info_v`（声明类型本身是 Sort、无重复宇宙参数、theorem 类型必须落在 Prop）、`check_def_like_v`（值的类型与声明类型 `def_eq`）。
- **conv（定义相等）**：基于求值后的 WHNF 对齐 + 按需 delta 展开，利用 relevance 签名（`relevance.rs` 的 `Sig`，证明/被忽略参数可跳过）、指针相等、正/负转换缓存、probe 预算。宇宙 `leq`/`eq_antisymm` 用 `level.rs` 的决策逻辑。
- **归纳/商检查器**：`inductive.rs`（正向性、构造子参数/索引、重建 recursor 规则并与导出规则比对、大消除/K/证明无关边界、mutual 块、嵌套特化）；`quot.rs`（`Quot` 声明族）。临时环境扩展（`new_w_temp_ext`）服务于嵌套归纳检查。
- **quote**：值 → 表达式（`quote.rs`）。教学 API 就加在这里：
  - `infer_closed_type(e)` = 空环境推断 + WHNF + `quote_weak`（`#check` 的 kernel 原语）；
  - `reduce_closed(e)` = `value_of` + `deep_reduce` + `quote`（`#reduce` 的 kernel 原语）。
  - `infer_under_binders(&[binder_ty], e)` = 在给定 binder 类型序列（由外到内）下推断
    开放项 `e` 的类型并 quote 回来——这是编辑 hover 类型图的 kernel 原语。
- **pretty printer**（`pretty_printer.rs`）：把表达式/声明打印成可读文本；lang 版本默认 **ASCII `->`**（与教学文件一致），打印 proof term 而非 `_`。

### 5.4 内置 prelude 与"原生 Nat"技巧（重要！）

`compile.rs::install_prelude` 在文件没有显式 `inductive Nat` 时装入最小可信基元：

```text
inductive Nat : Type
ctor      Nat.zero : Nat
ctor      Nat.succ : Nat -> Nat
rec       Nat.rec  : (motive : (n : Nat) -> Sort u) -> …   ← 由 install_inductive_block 派生
def       Nat.add  : Nat -> Nat -> Nat := Nat.add ← 占位自引用体
```

要点与坑：

- 这些声明**从不被 try_check_declar 重查**（不进 PendingOp），是"受信任的预置"；注释明确写了这一点。
- `Nat` 走与源内 `inductive` 相同的 `install_inductive_block`：`Nat.zero`/`Nat.succ` 是真正的构造子，`Nat.rec` 是带 iota 规则的递归子；同时把 `Nat` 登记进 `match` 的 `InductiveTable`，因此 `match` 能降低为 `Nat.rec.{level}`（此前 prelude Nat 无元数据，`match` 报 `elab-match-not-inductive`）。
- `EnvBuilder::finish()` 会 `dag.mk_name_cache(anon)`：在 intern 表里按名字找到 `Nat.succ/Nat.add/...`，给 `NameNode` 打 `NatRed` 标记。此后求值遇到这些头时，`unfold_value_go` 先走 `do_nat_red` **原生大整数运算**（`num-bigint`）；构造子 `Nat.succ` 在 `apply` 里还会把已是一元链的参数折叠成 `NatLit`。这就是 `#reduce 1 + 2 => 3` 与 `#reduce Nat.succ (Nat.succ Nat.zero) => 2` 的来源。`Nat.add` 仍必须是 `Definition`（Unfoldable）而非 `Axiom`，原生快路径才会接管。
- `Expr::Num` 在 front 被 elaborate 成 `NatLit`（bignum 指针）；`a + b` 是 `Nat.add a b` 的语法糖。
- **风险/待对齐**：`Nat.add` 的体是"自引用占位"，语义上等价于公理 + 原生快路径；上游真身是正常递归定义。教学 prelude 必须保证这些名字**只在有实参时被原生快路径接管**、裸名字（如 `#reduce Nat.add`）不会被 delta 无限展开——当前实测 `#reduce Nat.add => Nat.add` 可终止，但这是要长期盯住的边界（见 `docs/design/infrastructure.md` 的 prelude 工作流）。另一个已知现象：`Nat.rec` 的 `NatLit` 快路径会先给递归结果套一层未归约的一元链，`deep_reduce` 不再回收，所以 `match` 递归结果可能呈混合表示（如 `2 + 1 => Nat.succ (Nat.succ 1)`），def-eq 上仍等于 3。
**构造子命名空间（G-02 / WO-005，0.59.0）**：源内 `inductive` 的构造子也走同一条
「点名前缀 = 规范名」的路（`Ind.ctor`），所以**源文件自带 `inductive Nat` 时**，
ctor 一旦叫 `Nat.zero`/`Nat.succ`，name cache 的 `NatRed::Succ` 快路径就会接管 ——
`#reduce` 的输出从 `succ (succ (succ (succ zero)))` 变成**混合表示**
`Nat.succ (Nat.succ (Nat.succ 1))`（`apply` 把已是一元链的参数折成 `NatLit`）。
`Bool`/`Color` 这类没有 NatRed 的归纳只是纯改名（`ff` → `Bool.ff`）。逐条实测值见
`docs/design/ctor-namespace.md` §2.1。

`install_bool_prelude` 同法装入 `Bool`（**非递归**）：`Bool.true`/`Bool.false` 真构造子 +
派生 `Bool.rec`（两分支消去子，无 IH），登记进 `known` 与 `match` 的 `InductiveTable`。
名字必须是 `Bool`/`Bool.true`/`Bool.false`：内核 name cache 已预留这两个 ctor 槽位
（供原生 `Nat.beq`/`Nat.ble` 返回布尔值），故内核**零改动**即可归约。文件自带
`inductive Bool` 时 prelude 不装（与 `Nat` 同一闸；否则重复声明 panic）。

- `nat_extension` 由 `Config::default()` 默认打开；`StringLit` 类似（`string_extension`）。

#### 5.4.1 L1：逻辑与等式骨架（0.59.0；设计 `docs/design/prelude-l1-proposal.md`）

`install_l1_prelude` 在 Full 模式下把 Lean core 的逻辑与等式骨架也作为受信任预置装入
（规范源文本 `PRELUDE_L1_SRC`，与用户声明走**同一条 elaborator**：`build_axiom`/
`build_def`/`install_inductive_block`）。30 个顶层名字分 7 个**族**：

| 族 | 名字 | 形态 | 依赖（被让位时本族也让位） |
|---|---|---|---|
| B1 真伪 | `True`, `True.intro` | axiom | — |
| B2 假与爆炸 | `False`, `False.rec`, `False.elim` | axiom + def | — |
| B3 且 | `And`, `And.intro`, `And.left`, `And.right`, `And.elim` (+`And.rec`) | **真归纳块** + def | — |
| B4 或 | `Or`, `Or.inl`, `Or.inr`, `Or.elim` (+`Or.rec`) | **真归纳块** + def | — |
| B5 非 | `Not`, `Not.intro`, `Not.elim`, `absurd` | def | **B2** |
| B6 当且仅当 | `Iff`, `Iff.intro`, `Iff.mp`, `Iff.mpr`, `Iff.refl`, `Iff.symm`, `Iff.trans` | def | **B3** |
| B7 Eq 引理 | `Eq.symm`, `Eq.trans`, `congrArg` | def | **Eq prelude** |

**让位规则（谁声明谁拥有）**：粒度 = 族，不是单名；触发集合 `taken` =
整个闭包的顶层名字并集（`top_level_def_spans_over` 的键集，**含构造子与递归子**）。
命中任一名 ⇒ 该族 + 依赖它的族一起不装。依赖边的方向是**被依赖者被占用则依赖者让位**
（`Iff.mp` 的定义体用 `And.left`，所以文件声明 `And` ⇒ `Iff` 消失；反过来不成立）。

**两个实现要点（as-built，都是实测出来的）**：

1. **安装顺序是先 Eq 后 L1**：B7 的定义体引用 `Eq.subst`/`Eq.refl`，必须等 Eq 进环境；
   两者读同一个 `taken`，所以顺序不影响让位结果。
2. **重入闸 `L1_INSTALL_DEPTH`**：装 `And` 归纳块时 `install_inductive_block` 要用
   `large_elim_test_mirror` 问内核「字段类型是不是 Prop」（`field_sort_via_kernel` →
   `judge_infer` → **内层 `compile_fol_with`**），内层又会装一遍 L1 ⇒ 无限递归
   （实测 `stack overflow, SIGABRT`）。计数 > 0 时 `install_l1_prelude` 直接返回；
   L1 安装期间的探针只需内建的 `Prop`，不需要任何 L1 名字。

**白名单与豁免面**：`PRELUDE_NAMES`（补全/材料，47 条）与 `PRELUDE_NEVER_YIELDS`
（只含 `Nat`/`Bool` 家族，给 `check_name_collisions` 用）**是两个常量**。L1 名字按族
合法让位，所以**不能**进 `PRELUDE_NEVER_YIELDS`——否则两个模块各自声明 `True` 就不再报
友好的 `import-name-collision`，退化成内核裸错。parser 白名单：L1（B1–B8）不引入新语法；
0.61.0 的**层级算术** `u+1`（`Sort (u+1)` / `Eq.{u+1}`）是唯一例外，白名单 =
`docs/design/type-level-syntax.md` §5（parser + elab，内核零改动）。

**已知边界**：`congrArg` 只能同宇宙层级（G-14）；签名显式给全参数（隐式实参不自动插入），
所以填好的项不能逐字粘进官方 Lean——那是一次签名变更，届时另开提案。
`#check`/hover 对 prelude 名与 `Nat`/`Eq` 同状（受信任安装、无 `DeclState`）。
by 引擎的判定合成声明不带宇宙参数（`by.rs::spec_of` 的 `universe: Vec::new()`），
所以**目标里出现宇宙变量**时 tactic 判定失败（`Sort u` 自 0.60.0 起、`Sort (u+1)`
继承同一条；`suggest`/`judge_terms` 走 `DeclState.universe`，不受影响）。

### 5.5 `#prove`：tactic 只是"帮你搭 lambda"（`crates/front/src/proof.rs`）

`ProofState { goal, goal_source, binders, solution }` 完全不碰内核：

- `intro name`：剥掉最外层 `Forall`/`Arrow` binder，把 binder 存进 `binders`，goal 变成 body；
- `exact term` / `apply term`：把解析出的项放进 `solution`（洞）；
  （历史：早期 exact/apply 同义、`assumption` 走文本比对——两者均已由内核
  判定取代，见 §8.8。）
- `lambda_text()`：把 binders 反向包回 `fun (b : T) =>` 前缀，未完成处显示 `sorry`；
- REPL 里 `done` 把 `example : <goal> := <lambda>` 追加进 buffer 重编译，**由完整内核判定**。

设计意图：让学生先看懂"证明就是构造 lambda/证明项"，再接触 tactic 语感。

---

## 6. 内核相对上游快照的改动清单（务必先读）

> **政策（2026-09-21 用户解冻，取代旧的"冻结快照"）**：`crates/kernel` **可以改**，
> 含热路径与内部表示，目的可以是**提速**。唯一红线是**判定正确性不变**——
> 同一批输入接受/拒绝不变、事件计数不变、golden 与 `--json` 逐字节不变。
> 每次内核改动必须带**三层回归**（kernel `tests/` + front 单测 + CLI e2e）与
> **语料对拍**（全部 `*.sokonanoda` 的 stdout 逐字节比较、课程门禁计数逐项不变），
> 性能改动另记 `docs/perf/ledger.jsonl`。**改之前先读 §8 gotchas**
> （arena 生命周期、panic→Result、`quiet_catch` 不可嵌套、每命令输出通道的两个消费者）。

### 6.1 改内核的验收五步（一条命令：`scripts/kernel-check.sh`）

红线是**判定不变**——同一批输入**接受/拒绝不变、事件计数不变、golden 与 `--json`
逐字节不变**。五步就是这条红线的五个面，**缺一面等于没验**：

| # | 步骤 | 它挡的是什么 |
|---|---|---|
| ① | `cargo test --workspace --locked` | 三层回归（kernel `tests/` + front 单测 + CLI e2e） |
| ② | `bash scripts/kernel-diff.sh --fast <前> <后>` | **全语料逐字节对拍**——改动前后两个二进制的 stdout + 退出码逐字节比（改之前先 `cp target/debug/sokonanoda /tmp/sokonanoda-before`，改完用 `KERNEL_DIFF_BASELINE=` 喂回来；没给基线时退化成 `--self-test`，证明这条通道本身能发现差异） |
| ③ | `python3 courses/set-theory/tools/check.py` | **语料级**计数红线（`36 目标 · 328 checked · 99 open · 0 判负`）——内核一动最先在这里露头 |
| ④ | `bash scripts/perf-ledger.sh` | 性能台账（提速允许，**退化不行**） |
| ⑤ | `cargo test -p sokonanoda --test arena` | 真 Lean 导出语料的接受/拒绝与预期一致 |

```bash
scripts/kernel-check.sh                                    # 五步全跑（⑤ 需外部语料）
LEAN_KERNEL_ARENA=/path/to/lean-kernel-arena scripts/kernel-check.sh
LEAN_KERNEL_ARENA=… LEAN_KERNEL_ARENA_MAX_BYTES=$((1024*1024*1024)) scripts/kernel-check.sh
```

**⑤ 是"本地可选"，但绝不假装有**：`LEAN_KERNEL_ARENA` 没设时脚本**明说跳过**
（不是静默通过）——这条曾经是安慰剂：CI 里没设、测试静默空过，而收集逻辑本身
还有三个缺陷（不递归、按体积静默跳过、outcome yaml 只认平铺）叠在一起，
**"语料对拍"实际上只跑了一个用例**（T-K02 已修，并补了不依赖外部语料的夹具测试
`collect_cases_walks_subdirectories_and_finds_nested_specs`）。

快照基线 `sokonanoda@7b51784`（见 `NOTICE.md`）。下表是**已经改过**的部分
（历史记录；新增改动请追加一行）：

| 文件 | 改动 | 用途 |
|---|---|---|
| `builder.rs` | **新增** `EnvBuilder` | 在 arena 内构造声明、最后一次性 `finish()` 成 `ExportFile`，绕开 `TcCtx` 自引用生命周期 |
| `util.rs` | `ExportFile::empty`、`Config::default`、`CheckError`、`try_check_declar`（panic→Result）、`alloc_string/alloc_bignum/name_from_str` 公开 | 内存 API（M0 验收） |
| `tc.rs` + `util.rs` | **显式限界终审（第九十一轮续）**：`ExportFile::check_declar_at(d, EnvLimit)` / `try_check_declar_at(d, EnvLimit)`。原 `check_declar` 用 `EnvLimit::ByName(d.info().name)` 定可见前缀，而**刻意不入环境**的合成声明（`redundant-sorry` 探针）名字没有 `decl_idx` → 取 0 → 空环境；新入口让 front 自己给 `EnvLimit::ByIndex(env_before)`。`check_declar` = `check_declar_at(d, EnvLimit::ByName(name))`，批量路径（`run_session_inner`）逐条显式传同一个 `ByName` ⇒ **行为逐字节不变**，热路径零改动。回归：`tests/memory_api.rs::synthetic_declaration_needs_an_explicit_environment_limit` + front/CLI/LSP 三层（设计见 `docs/design/redundant-sorry.md` §8） | 探针终审能看见它引用的前缀常量；「删掉这行 sorry 就能过内核」才敢说 |
| `quote.rs` | `infer_closed_type` / `reduce_closed` | `#check` / `#reduce` 的 kernel 原语 |
| `eval.rs` | `deep_reduce` | 教学 `#reduce` 完整归约 |
| `tc.rs` | `ctx` 公开 + `TypeChecker::with_pp` | front 直接读写内核 arena |
| `pretty_printer.rs` | `→` 改 ASCII `->`；`name_from_str` 公开 | 教学文本一致 |
| `conv.rs` + `infer.rs`/`inductive.rs`/`tc.rs` | def_eq 失败分支的 panic 消息改为稳定格式 `def_eq mismatch expected: <E> \| actual: <A>`（保留 `def_eq failed` 前缀；两侧值经 quote + debug 打印，各截断到 200 字符）；仅改动失败/冷路径，热循环不变 | 内核拒绝能给教学文案「类型不匹配：期望 X，实际是 Y」（front 解析该标记，见 `front/src/compile/error.rs`） |
| `conv.rs` | **soundness 修复（2026-09-07，lang）**：`unify_direct` 的 Pi/Lam body-expr 快路径增加 `closure_ctxs_compatible` 守卫——eval 闭包（`ctx: None`）与 infer 闭包（`mk_infer`）对同一 interned body 表达式语义不同（`$0` vs `Sort 1`），混用时 `(A : Sort 1) -> A` 这类不可居住类型会被 `fun (A : Sort 1) => A` 误判可居住。热路径仅增加一个闭包语义判别分支；perf 冒烟与全量测试无回归。回归测试在 `tests/memory_api.rs` 与 CLI e2e | 判定（kernel 信任边界）正确性：tactic judge、练习判定都依赖 def_eq 不出假阳性 |
| `lib.rs` | `deny(clippy::cast_possible_truncation)` → `warn`（上游代码自身未过此 lint；冻结快照原则，教学 crates 的严格 lint 门禁经各自 `[lints]` 表实现） | lint 配置，非语义 |
| `conv.rs` + `inductive.rs` | **冷路径消息分诊（2026-09-07 第十三轮）**：`is_prop_type` 的 `.expect("expected a sort")` 改为带 `got:` 渲染、措辞区分站点的 `expected a sort in conversion`（分类器前缀不变，教学码同为 `kernel-expected-sort`）；`inductive.rs` 两处学习者可触发的裸 `assert_eq!`（iota 规则顺序/数量）改稳定 panic 消息并新增家族 `kernel-rec-rule-mismatch`。三层回归：kernel memory_api + front 分类 + CLI e2e | 错误分类学余项；冷路径，热循环零改动 |
| `util.rs` | **六个 interner + `Dag` 加 `Clone`**（T-K13 Step 1-2，2026-09-24）：`interner!` 宏里的结构体与三个手写 interner（`NameInterner`/`BigUintInterner`/`LevelsInterner`）`#[derive(Clone)]`，`Dag` 同。**纯能力新增**（不改任何判定路径）；`HashTable` 本身可克隆（编译器确认）。回归：内核 60 条测试全过 | `EnvBuilder::snapshot()` 的基础；快照是"只读副本"⇒ 不写 `decl_idx` 槽位 |
| `builder.rs` | **新增 `EnvBuilder::with_env`**（T-K12a，2026-09-24）：把 `dag`/`declars`/`notations`/`mutual_block_sizes` **借**给一个临时 `ExportFile`（`name_cache` 由 `dag.mk_name_cache(anon)` 现造、借完丢弃），回调结束后**原样装回**；`ExportFile`/`TcCtx`/`eval`/`conv`/`infer` **零改动**。回归：`tests/memory_api.rs::with_env_lends_the_intern_tables_and_takes_them_back`（借出时同一份表 ✓ / 合成声明不进环境 ✓ / 装回后照常 `add_declar`+`finish` ✓）+ 内核 60 条 | 让检查器看见与 builder **同一份** intern 表（`decl_idx` 是挂在被 intern 的 `NameNode` 上的全局槽位 ⇒ 换一份表会**静默取到别人的声明**） |
| `builder.rs` | **新增 `EnvBuilder::snapshot() -> ExportFile<'a>`**（T-K13 Step 3-4，2026-09-24）：**克隆** `dag`/`declars`/`notations`/`mutual_block_sizes`（`name_cache` 现造）⇒ 检查器拿**只读副本**用。与 `with_env` 的区别写进注释：**借出装回** vs **复制**。回归：`memory_api.rs::snapshot_is_a_read_only_copy_that_does_not_disturb_the_builder`（副本能查到前缀 ✓ / 检查**不动** builder ✓ / 副本**不跟着长** ✓） | 检查器用副本 ⇒ **不写 `decl_idx` 槽位**（T-K12c 死因正是"往独立环境 `add_declar` 改写共享槽位"） |
| `util.rs` | ~~`AdmitTable`（`whnf_admit` 复用池）~~ **已回退**（T-K31，2026-09-24）：池化后冷跑 `unit12-solution` **12.29s/11.67s** vs 改动前 **11.96s/11.67s**（噪声内）⇒ **无收益** ✗。机制：`vec![0u8; 1<<22]` 走 mmap 拿**惰性零页**（分配器通常不必真 memset），池化反而**强制** `fill(0)`。阴性结果在 `docs/perf/ledger.jsonl` | 记录一次**实测否决**：不为无收益的复杂度买单（判据含"性能无退化"） |
| `Cargo.toml` | bin 改名 `sokonanoda-kernel`；`stumpalo 0.5.1` | workspace 集成 |
| `tests/memory_api.rs` | **新增** | 无导出文件的内存检查验收 |
| `builder.rs` | `add_inductive` 返回构建的 `Declar`；新增 `begin/end_inductive_block` 与 `mutual_block_sizes` 记账 | 归纳块可被 kernel 判定（I8a check-then-add） |
| `quote.rs` + `tc.rs` + `pretty_printer.rs` | **hover 显示层（2026-09-09，第十七轮）**：`infer_under_binders` 去掉 `force_all`（`Not a` 保持折叠，quote 自行 force thunk；唯一消费者是 front 的 `resolve_hovers`）；pp 新增 `seed_binder_names(&[String])` + `TypeChecker::with_pp_scoped`——把 scope binder 名字（外层在前）预置进 `binder_names`，开项打印的松散变量直接还原真名（代数验证含 telescope 域 lift 的所有位置：`S-1-j` 恒成立）。热路径零改动 | 悬停「表达式 : 真名类型」；`$N` 索引不再泄漏（设计见 `docs/design/hover-brackets.md`） |
| `pretty_printer.rs` | **hover 开项短路（2026-09-10，第二十四轮）**：`is_implicit_fun` 对含松散变量的 fun 项直接返回 `false`——空 context 推断开项（Var 头 `p a`、依赖实参 `Eq α a`）会对松散变量 panic（`infer: loose bvar` / `eval: loose bvar`），hover/`#check` 的类型文本被 front 的 catch_unwind 吞空。纯显示层，闭项行为逐字节不变；三层回归（kernel `memory_api` + front hover + CLI/LSP） | hover `Eq.subst.{1}` 显示完整签名；`#check (Eq.subst.{1})` 不再假报 kernel-rejected（设计见 `docs/design/goal-func-spine.md`） |

上游"解析 NDJSON 导出文件 → 完整检查"路径原样保留（`parser.rs` + `Config::to_export_file` + `check_all_declars`），arena 集成测试 `tests/arena.rs` 也在（需要 `LEAN_KERNEL_ARENA` 指向用例根目录才运行）。

---

## 7. 测试金字塔与回归策略

- 每个语法点/错误模式先在 front 写单元测试（`compile.rs` 的 `#[cfg(test)]` 里有大量 py-fol/py-nat 移植断言），再在 CLI 端到端覆盖，最近再加了"所有 examples 必须能整文件通过"的语料测试。
- 判定练习**靠 kernel，不靠文本比对**（唯一例外：`proof.rs::assumption` 的草案级文本比对，待替换）。
- `tests/arena.rs` 需要环境变量；上游两个缺 fixture 的测试在 kernel 内被隔离/ignored（不是跳过内核能力，而是缺少 NDJSON fixture，重建是独立任务）。

---

## 8. 给其他 agent 的注意点（gotchas）

0. **内核交互必须包 catch_unwind**（`quiet_catch`/`resolve_hovers` 模式）：
   内核以 panic 报拒绝/内部错误，不包会崩掉编译/LSP；且 panic hook 是
   进程全局的，`quiet_catch` **不可嵌套**。经验台账见 `docs/LESSONS.md`。
0b. **front 与内核的归纳块协作契约（第十三轮起）**：内核给每个归纳块
   自算 `is_recursive`（构造子 telescope binder 类型是否提到归纳名——含
   result 箭头链的 domain）并断言 front 传入值一致；内核还要求每块注册
   `Recursor` 声明（`<ind>.rec`，每构造子一条 iota 规则）。front 侧镜像：
   `elab.rs` 从源码 AST 同规则计算 `is_recursive`；**缺 `rec` 的块自动派生**
   `RecDecl` + iota 规则（0.58.0 起；`elab-missing-inductive-rec` 已不存在）。
   **派生判据必须与内核同规则镜像**，不能近似——两条已钉死的：
   `is_k` 镜像 `init_k_target`（H6-C），recursor 的**宇宙参数个数**镜像
   `large_elim_test`（G-03 / `docs/design/prop-large-elim-mirror.md`）；
   两条都各自带判别性成对测试。
1. **arena 生命周期**：`EnvBuilder`/`ExportFile`/`ExprPtr` 都挂在同一个 `stumpalo::Arena` 上，arena 必须活得比任何检查会话久；front 在 `compile_fol` 内开 arena 并一次跑完所有 PendingOp。Session（`front/src/session.rs`）每次 update 都开新 arena——跨 update 只复用渲染后的快照（DeclState/hover/事件文本），不复用内核对象。
2. **kernel 拒绝 = panic → Result**：内核仍用 `assert!` panic 报拒绝（如 `def_eq failed`），`try_check_declar` 用 `catch_unwind` 包装成 `CheckError::Rejected/Internal`。conv 失败的 def_eq 消息带 `expected/actual`，front 解析填充 `CompileError.expected/actual`（I9 已闭环）；更细粒度的 kernel 错误仍是后续任务（见 design doc）。
3. **elab 仍受限**：binder 可由声明类型推断（I6；应用位置的未注解 `fun x => …` 也可从实参类型推断，0.45.0）、值位 `let`（Phase 1）、值位 `match`（Phase 2，源内 inductive 与 prelude `Nat`，含递归 IH `ih`/`ih2`…）与**非带索引参数化归纳**（`inductive Option (A : Type)`，含对它的 `match`；`docs/design/parameterized-inductives.md`）已落地，但未做无注解 `let`、依赖 motive、**带索引**归纳与宇宙多态参数、prelude `Eq` 的 match、`match` tactic、结构/类型类、**macro**（`notation` 已于 0.59.0 落地，见 `docs/design/notation-subset.md`；见 `docs/design/elaborator-let-match.md`、`docs/design/match.md`）。
4. **语法白名单是边界**：想加语法，先加课程 + 测试；`sorry` 只允许出现在声明（def/theorem/example）的值位。
   记法（0.59.0）是**唯一的例外面**：它不引入新语义，只是用户自定义的源级糖，
   所以它的"课程"是使用者自己写的声明行，边界由设计文档 N1–N7 钉住
   （`docs/design/notation-subset.md`）。
5. **不用官方工具链**：CI 与本地一律 `cargo`；不要引入 `lean`/`lake`/`lean4export`。
6. **新错误要带 stage/code 与 span**：CLI 已按 `error[stage]:` 输出，`--json` 是 agent 视图；改输出格式要同步 `docs/protocol.md` 与 `crates/cli/tests/cli.rs`。
7. **打印偏好**：教学文本 ASCII `->`；`pp_options.proofs=true` 由 `compile_fol` 设置（否则打印会把证明项压成 `_`）。
8. **tactic/编辑器判定走 `front::judge`**（合成完整声明交完整 kernel 裁决），不要新增文本比对；`proof.rs::assumption` 的文本比对实现已删除。建议生成（`front::suggest`：exact/rfl/refine/intro）与逐洞判定（`judge_hole_fill`：把洞替换候选后整份声明交 kernel）都只是结构生成 + kernel 终审。
9. **kernel lint**：`lib.rs` 的 `cast_possible_truncation` 已降为 warn（上游代码自身未过）；clippy 严格门禁在各教学 crate 的 `[lints.rust] warnings = "deny"`，CI 的 fmt 门禁只覆盖教学 crates（kernel 的 rustfmt.toml 需要 nightly）。
10. **多文件项目（I16）的三个坑**：① 错误**与警告**归属只能按**命令下标**
    （`error_cmds` / `warning_cmds`），绝不能按 span——不同文件的字节偏移会互相
    命中；新增"每命令输出通道"时先看 `split_report` 与
    `session::build_suffix_snapshots` 两个消费者（0.58.0 合并轮就踩过 warning 丢
    归因）；② **不要**给 `import`-free 的
    文件加任何项目开销（`plan_project` 只在解析出 import 后才被调用，A1 回归由
    `crates/cli/tests/imports.rs` 的 stdin/文件同字节断言守住）；③ 内存覆盖与诊断
    发布：覆盖的路径要 `canonicalize` 后再比（macOS `/var` vs `/private/var`），
    构建覆盖时**先把当前文档的新文本替进去**（否则下游重编译看到的还是上一版依赖
    ——实测踩过），多文档测试要边处理边排空（见 §4.5 末与 `docs/TESTING.md` §5.7）；
    ④ **编辑器不许自己发明模块根**：LSP 的 `initialize` 工作区根不是模块根
    （把工作区根当 `root_override` 等于跳过清单发现，工作区里嵌套的项目会
    `import-not-found` 而 CLI 正常）；一律交给 front 按 CLI 同款规则发现。

---

## 9. 术语表

| 术语 | 含义 |
|---|---|
| kernel / 内核 | 可信的、被完整迁移的类型检查器（crates/kernel） |
| front / 前端 | `.sokonanoda` 词法/语法/小型 elaborator/练习判定（crates/front） |
| Declar | 内核声明（axiom/def/theorem/inductive/ctor/recursor/quot/opaque） |
| Elaboration | 表面语法 → kernel 项（名字解析、universe 参数解析、binder 折叠） |
| iota | recursor 在构造子上的计算规则（pattern matching 的计算语义） |
| delta | 定义展开（def/theorem 体替换） |
| conv / def_eq | 定义相等/可转换性（类型检查核心关系） |
| WHNF | 弱头范式；常规检查只需 WHNF，`#reduce` 要 deep normal form |
| EnvLimit | 环境可见前缀控制（Empty/ByIndex/ByName/PpUnlimited） |
| CheckEvent | 前端产出的事件（decl.checked / expr.typed / exercise.open …） |
| `sorry` | 练习"未完成"洞：合法文件状态，产出 `exercise.open` |
| `#prove` | 教学 tactic 草稿：tactic 语句只在搭 lambda，最终仍由 kernel 判定 |
| NDJSON | lean4export 的导出格式（上游内核的检查输入；教学栈不走它） |

---

## 10. 快速上手指南

> 面向**贡献者**（需要 Rust）。用户/agent 的零工具链路径见根 `README.md`
> 「Use it」与 `skills/sokonanoda-teacher`（Release 二进制 / 平台插件）。

```bash
cargo test --workspace                       # 全部测试
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json examples/fol-basics.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl     # #check / #reduce / #prove
cargo run -q -p sokonanoda-lsp --bin sokonanoda-lsp      # LSP（editor/vscode 里使用）
```

想给某个语法点加测试：先在 `crates/front/src/compile.rs`（或 `lib.rs`）加单元测试 → 在 `crates/cli/tests/cli.rs` 加端到端 → 需要的话新增/改 `examples/lesson-XX.sokonanoda`（examples.rs 会自动跑它）。

## 改内核的机械判据：`scripts/kernel-diff.sh`

内核解冻（2026-09-21）之后，"判定正确性不变"这句话必须有**机械**判据，
不能靠"测试都绿了"。工具是 `scripts/kernel-diff.sh <改动前二进制> <改动后二进制>`：

```bash
# 每环节用（1.9 秒）：3 个形状不同的文件 × grade + query check
scripts/kernel-diff.sh --fast target/debug/sokonanoda /tmp/after

# 合入前用（约 6 分钟）：全部语料 × 5 个 op + 课程门禁计数逐项比较
scripts/kernel-diff.sh target/debug/sokonanoda /tmp/after

# 自检：证明对拍器本身有效（同一二进制零差异 + 人为注入一个字节必须被抓到）
scripts/kernel-diff.sh --self-test target/debug/sokonanoda
```

它比的是 **stdout 逐字节 + 退出码**，并且 `SOKONANODA_NO_CACHE=1` 强制关缓存
（条目命中会跳过编译，对拍就测不到内核；两个二进制版本相同时还会互相命中）。

**为什么测试不够**：内核改动可能"测试全绿但判定变了"（阈值、边界、归因）。
逐字节对拍把**整个语料**的判定结果钉死，这是单测覆盖不到的。

## 记法转化的**唯一接口**（阶段 U，2026-09-25 用户要求「完全统一接口」✓）

> **一句话**：**AST/文本 → 给人看的文本与分段**这件事，全仓**只有一个入口**
> `DisplayNotations::{fold, render, runs}` ✓（`crates/front/src/display.rs` ✓）。
> 谁要"带记法的文本/分段"都必须经它 ✓；绕过它 = 又长出一套实现 ✗。

### 为什么（真实事故：四处实现 ⇒ 用户 bug）
2026-09-25 用户报"Infoview 顶部「目标」里 `∃` 没转化" ✗。查证：**不是一处 bug，
是四处各自为政的实现** ✗：

| 实现 | 干什么 | 曾经的消费者 | 现状 |
|---|---|---|---|
| `display::print_back` | **真的转化**（文本→文本 ✓） | `ty_text`/`val_text` | ✅ 收进 `fold` ✓ |
| `semantic::tag_runs_with_notations` | **只打标签**（不转化 ✗） | `query::runs` ⇒ `goal_runs`/`ty_runs`（Infoview 读它 ✓） | ✅ 收进 `runs` ✓ |
| `display::render_expr` | **只渲染**（AST→文本 ✗） | `goals::open_goal`、walk 的三处目标 | ⚠ 仍是底层原语 ✓（只许接口内部调 ✓） |
| 内核 pp（`info.goal`） | 点形式 ✗ | walk 的两处 tactic 目标 | ⚠ 同上 ✓（文本进来后**必过折叠** ✓） |

**后果**：目标生产链（③④⇒②）**只渲染不折叠** ⇒ 顶部目标永远是点形式 ✓。
`AGENTS.md` 的 R-1/R-2 教训"真相与显示是两条路"在这里升级为"**连显示自己都分了四条路**" ✗。

### 三条不变量（改这块前先读 ✓）
1. **折叠只发生一次** ✓，在 **front 生产侧** ✓；query/wire/扩展**只搬运不许再折** ✗。
2. **`text` 与 `runs` 必须逐字节成对** ✓ —— 守卫：
   `query::tests::every_decl_ships_text_and_runs_in_lockstep` ✓（覆盖 `ty`/`value`/`goal`/
   `goals[i]`/`binders[i].ty` ✓，正反两向实测过 ✓）。
3. **judge 输入一个字节都不许动** ✗ —— `DeclState.goal` / `binders[].ty` / `sub_goals[].ty`
   **同时喂判卷** ✓；折它们 ⇒ `suggest::*` 当场判红 ✓（2026-09-25 踩过 ✓，见
   `crates/front/src/compile/check/kernel_phase.rs` 的注释 ✓）。
   ⇒ **要折就折"显示副本"** ✓（`ty_text`/`val_text` 就是这个形状 ✓）。

### 守卫（防第五套 ✓，都已进 `scripts/soko gate`）
* `scripts/audit-notation-paths.py` ✓ —— 白名单之外出现 `render_expr(`/`print_back(`/
  `tag_runs_with_notations(` 即判红 ✗；**棘轮**（基线 `scripts/notation-paths-baseline.txt` ✓
  冻结存量、只拦新增 ✓）；`--self-test` 反向验证 ✓。
  ⚠ **盲区**：白名单按**文件**豁免 ⇒ `display.rs` **内部**再长一个等价入口它抓不到 ✗
  （真实实例：`render_folded` ✓ 已删 ✓）⇒ 改那个文件前先看顶部接口清单 ✓。
* `scripts/audit-wire-fields.py` ✓ —— A∖B 对账（扩展读了 / LSP 从不发 ✓），
  `--selftest` 能咬住 R-1（`value_runs` ✓）。

细节与迁移表：`docs/design/notation-display.md` ✓；
审计（25 条"同一件事多处实现"）：`docs/design/duplication-audit.md` ✓。

## 9. 判据与流程纪律（2026-09-25 补 ✓，来自 T-U11/T-U12 的实测教训）

> 这两条不是风格偏好 ✓，是**本 session 反复付了学费**换来的 ✗。

### 9.1 **新判据必须附一条"反向验证命令"** ✗

**已经三次**写出"**通过但咬不住**"的判据 ✗：
* 面级 sweep 第一版 ✓（扫的夹具是**源级渲染** ⇒ 不需要折叠 ⇒ 判据空转 ✗）；
* 诊断面第一版 ✓（`QueryDoc` 没有诊断入口 ⇒ 编译不过 ⇒ 撤回 ✓）；
* 诊断面第二版 ✓（夹具产生的**不是**那 4 处已折消息 ⇒ 关掉折叠照样绿 ✗）。

**规则** ✓：任何新判据的同一提交里，必须能回答"**怎么把它弄红**" ✓，并**实际跑一遍** ✓。
本仓库现成的两个干净手法：
* **代码层**：在 `crates/front/src/display.rs` 的 `print_back` 入口注入"原样返回" ✓
  （⚠ **必须打在 `print_back`** ✗ —— 打在 `DisplayNotations::fold` 上会"看起来不咬" ✓，
  因为有些显示副本走的是**直接调 `print_back`** 的老路 ✓）；
* **开关层**（首选 ✓，不改代码 ✓）：`SOKO_NO_NOTATION_FOLD=1 cargo test …` ✓
  —— `display_notations` 见它返回**空表** ✓ ⇒ 一切折叠失效 ✓ ⇒ 判据**必须判红** ✗。

### 9.2 **退出码，无 grep 掩膜** ✗

`set -e` **不会**因为 `cmd | grep …` 里 **cmd 失败**而停下 ✗ —— 管道的退出码是 **grep** 的 ✓。
实测代价 ✓：一轮之内两次把**没修好**的提交推上去 ✗。
**规则** ✓：判据只看**退出码** ✓；要看输出就先落文件再读 ✓：
```bash
cargo clippy --workspace --all-targets > /tmp/c.log 2>&1; rc=$?
[ "$rc" -eq 0 ] || { grep -E "^error" -A4 /tmp/c.log; exit 1; }
```
**并且**：批次 push 前**无条件**跑一次 `scripts/ci-local.sh` ✓
（它按 CI 的 5 条 job 分阶段 ✓、任一条红就指名 ✓、含 `--strict` 兜底 ✓）——
本文档记的两次 CI 红，它**本来都能拦** ✗。

### 9.3 **先看工具指的那一行** ✗

`clippy` 报 `empty line after doc comment --> elab.rs:1499` ✓ 时，
我先后猜成"助手插错位置" ✗ 并去**搬代码块** ✗（第二次还把文件弄坏、回滚 ✓）。
**真相是那一行下面的一个空行** ✓ —— 删掉即可 ✓。
**规则** ✓：诊断给了**行号**就从那一行看起 ✓；要改结构，先**读**再动 ✓，
且**用行号或内容锚点定位** ✓（"向上找注释行"那种启发式会一路走到文件头的 `//!` ✗）。
