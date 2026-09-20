# notation 子集（G-04 / WO-011）——第一刀：`∈` / `⊆` / `∅` + 通用 `infix`

> 状态：**已落地（as-built）**。工作单 `docs/gaps/WO-011-notation.md`；台账
> `docs/gaps/ledger.jsonl` 的 G-04。硬规则依据：`REQUIREMENTS.md` §2 第 3 条
> （语法增量 = 课程 + 测试 + 白名单三件套）、第 4 条（判定走内核）、第 1 条
> （内核冻结快照）。本文是白名单的**边界文档**：`docs/architecture.md` §4.1
> 只列命令清单，语义规则在这里。

## 0. 一句话

记法（notation）是**糖**：parser 只把 `lhs ∈ rhs` 记成一个带目标名的记号节点，
elaborator 把它**源到源**降级成既有的 `App` 形状。点名形式（`Set.mem α a A`）
一字不改、继续可用；**没有**源码级 print-back（goal/hover 的类型文本仍由冻结
内核的 pp 产出点名形式）。

## 1. 为什么不能只改 parser（WO-011 附 A 的三条实测事实）

第一刀的目标是「core 级最小记法集 `∈`/`⊆`/`∅` 可用」。真做起来会撞到三件事，
全部是 WO 里实跑出来的：

1. **词法层没有字符串 token**。`"` 直接落进 `token.rs` 的 `other =>` 臂报
   `unexpected-token`——记法命令的符号根本读不进来。
2. **符号字符今天是标识符字符**。`is_ident_start` 把 `≥0x80` 一律当标识符首字符，
   所以 `a∈b` 是**一个** `Ident`（实测 `unknown identifier \`a∈b\``）。
3. **`∈` 藏着一个类型参数**。课程库是
   `Set.mem (α : Type) (a : α) (A : Set α) : Prop`，而本语言**不插入隐式实参**
   （`mem2 x s` 被内核拒绝、`mem2 T x s` 通过）⇒ 记法**不是纯 parser 糖**：
   展开路径必须自己补前导 `Type` 参数，否则 `infix:50 " ∈ " => Set.mem` 会把
   `a ∈ A` 展开成一个被内核拒绝的项。

## 2. 语法边界（v1 语义规则，逐条可测）

### N1 命令形状

白名单只加**四个拼写**：

```
infix:N  " sym " => name      -- 无结合，两边同级
infixl:N " sym " => name      -- 左结合
infixr:N " sym " => name      -- 右结合
notation " sym " => name      -- 零元常量记法（∅）；不写优先级
```

- `N` **必填**（`infix` 族），合法范围 **1–1000**（严格落在 `->` 与函数应用
  之间）；越界给专用诊断 `parse-notation-precedence`，不静默取整。
- 符号是**字符串字面量**；v1 取 `trim` 后的内容，两侧空格是书写习惯
  （Lean core 逐字都带空格）。符号**不得**为空串，**不得**是标识符词
  （`"in"` 会被当成普通标识符读走，记法永远不可能命中）。
- 目标 `name` 是**点名**（可带 `Set.` 前缀，走既有点分名字解析）；未知目标
  在 elaborate 期给专用诊断。
- 目标**必须已经声明**（文件内、声明之后，见 N5）。

### N2 使用

`lhs sym rhs` 只要求在**同一文件、记法声明之后**（N5）；操作数顺序**保持**
（与 Lean `infix` 糖一致）。core 的 `∈` 是**显式 `notation` 并交换操作数**
（`Membership.mem b a`），那需要具名占位（`notation:50 a:50 " ∈ " b:50 => …`）
——**第二刀**。第一刀用保序 `infix:50 " ∈ " => Set.mem` 对准课程库的
`Set.mem (α) (a) (A)`。

### N3 优先级梯子

所有记法算子与内建 `+` 共用**一条优先级梯子**，位置在 `parse_arrow`（`->`，
最松）与 `parse_app`（函数应用，最紧）之间——今天的 `parse_plus` 就在这个缝里。
`+` 作为**内建保留项**进梯子，优先级 **65**，并**逐字节保持今天的行为**
（仍产出 `Expr::Plus`）。

| 结合 | 左操作数最小优先级 | 右操作数最小优先级 |
|---|---|---|
| `infix`（无结合） | `p` | `p + 1` |
| `infixl`（左结合） | `p` | `p + 1` |
| `infixr`（右结合） | `p + 1` | `p` |

> 取证状态：`infix`/`infixl`/`infixr` 的**精确脱糖**（左右是 `p`/`p`、
> `p`/`p+1`、`p+1`/`p`）**未从 Lean 源码逐字取证**（WO-011 待确认 1，写 WO 时
> 无网络）。仓库里逐字取证的只有 `infix:50 " ⊆ "`、`infixl:65 " ∪ "`、
> `infixr:80 " '' "` 三条**声明行**（`prior-art-report.md` §1.3）。上表是 v1
> **自定的规则**，实现与测试都钉它。
>
> `+` 在 Lean core 的优先级**未取证**（待确认 2，记忆中是 `infixl:65`）。
> v1 的实现不依赖这个数：`+` 行为逐字节不变（仍产出 `Expr::Plus`），这个数
> 只影响 `a ∈ A + B` 这类混写的分组——v1 里 `+` 走**同一条爬升**、优先级 65。
> 记法符号的**默认优先级**（`notation` 不写 `:N`）对**零元**记法无意义（没有
> 左右操作数），所以 v1 只在 `infix` 族要求 `N`。

`infix`（无结合）**不做** Lean 的「优先级冲突报错」语义：v1 用固定规则——
同级同符号连写（`a ∈ b ∈ c`）是**解析错误**（`infix` 无结合 ⇒ 右操作数要求
`p+1`，同级不再吃），不同符号同级按**左结合**处理。设计取舍：把 Lean 的
「同级混用需要括号」那套诊断留到第二刀（WO「不做的事」）。

### N4 展开（本设计核心）

1. 展开发生在 **elaborate 阶段**（不是 parser）：parser 只把 `lhs sym rhs`
   记成 `Expr::Notation { symbol, precedence, assoc, lhs, rhs, target, span }`；
   elab 把它**源到源**降级成既有的 `App` 形状，之后 elab/hover/kernel/事件
   全部复用既有路径。
2. **补全只允许发生在记号展开路径**：目标 telescope 比操作数多出来的**前导
   参数**按 **① 由操作数解出 → ② 无操作数时由期望类型解出** 的顺序补。
   可实现的形态是**裸变量匹配**：
   - `Set.mem : (α : Type) → (a : α) → (A : Set α) → Prop`，第 2 个参数的
     类型就是裸变量 `α` ⇒ `α := typeof(a)`；
   - `Set.empty : (α : Type) → Set α`，结果类型 `Set α` 对上期望 `Set α₀`
     ⇒ `α := α₀`。
   **不支持**一般合一/元变量。解不出给专用错误码
   `elab-notation-argument-unsolved`（hint 直接教怎么写点名形式）。
3. **回归护栏（兼容性的护城河）**：`Set.mem a A`（点名、省 `α`）**今天被内核
   拒绝，改后必须仍被拒绝**——补全只挂在记号路径上，不改变任何既有程序的语义。
4. 不许动内核：展开只产出 `mk_const`/`mk_app`。

**类型参数求值的实现路径（as-built）**：前导参数的值 = `judge_infer` 问内核
（合成 `#check fun <当前 binder 上下文> => <操作数>`，取 `TypeChecked` 文本再
剥掉 binder 层）。补全后的**完整应用**再交给既有 `elab_expr`（`Expr::App`
链）走正常路径——所以类型错、`@`、宇宙参数等既有语义**一字不改**地复用。
**操作数先 elaborate、类型参数后补**（as-built，实测钉在
`crates/cli/tests/protocol.rs::notation_diagnostics_stage_as_parse_and_elab`）：
- 操作数本身是未定义标识符 ⇒ 既有的 `elab-unknown-identifier`（那条诊断更准，
  不遮成记法错误）；
- 操作数都合法、但类型参数补不出来（例如 `#check ∅`）⇒
  `elab-notation-argument-unsolved`，hint 指向点名写法。

### N5 作用域：文件内、声明之后（file-local）

跨 `import` 的记法**第二刀**。依据：项目闭包加载时**先 `parse` 当前文件、再收集
import 边**（`project/graph.rs`），所以「记法写在 `lib/`、`units/` 直接用」今天
拿不到：入口文件 parse 时还不知道依赖里声明了什么。做跨模块要么改闭包加载顺序、
要么二次 parse——两条路的代价都写在这里，留给第二刀定。

因此：**文件内、记法命令之后**。在记法声明**之前**使用该符号 ⇒ 未声明符号诊断
（`parse-notation-unknown-symbol`，hint 给「先声明」与「点名写法」两条出路）。

> **第二刀（0.60.0）把这一条拆成两半**：**本文件自己**的记法仍然"声明之后才
> 生效"；**被导入模块**声明的记法**从文件头就可用**（与 Lean 的 import 一致，
> §10.3）。

### N6 记法不产生事件、不是声明

`infix`/`notation` 命令**不**产生 `decl.checked`/`exercise.open`/`expr.typed`/
`expr.reduced`，也**不进声明表**（与 `Command::Import` 同族：有命令、无声明）。
⇒ 既有两个课程的 GOLDEN 计数**不变**（§5）。

### N7 教学契约（验收的语言）

同一命题的两种写法——点名 `Set.mem α a A` 与记法 `a ∈ A`——**判卷结果一致**
（`grade` 退出码一致 + 五元事件计数一致），且**点名形式继续可用**。
契约**不含「打印文本一致」**：目标/hover 的类型文本仍由**冻结内核的 pp** 产出，
会显示点名形式。

## 3. 词法设计（as-built）

### 3.1 两个新 token

| token | 触发 | 说明 |
|---|---|---|
| `Str(String)` | `"` … `"` | 字符串字面量；**只**给记法命令用（表达式里没有字符串）。未闭合 ⇒ 专用诊断 `parse-unterminated-string`，span 指向**开引号**的行列 |
| `Sym(String)` | 数学符号码点（见下） | 记法符号 token；最大吞噬（连续符号字符算一个 token） |

### 3.2 数学符号码点类

`is_ident_start` 收窄为「`≥0x80` **且不属于**数学符号码点集」。第一刀取：

- `U+2200–U+22FF`（数学算子：`∈`U+2208 / `⊆`U+2286 / `∅`U+2205 / `∪`U+222A /
  `∩`U+2229 / `∘`U+2218 都在内）；
- `U+2A00–U+2AFF`（为第二刀的 `⋃`/`⋂` 预留）；
- `\`（U+005C）：今天直接是词法错误，改为 `Sym("\\")` ⇒「未声明符号」诊断
  比「不是一个合法 token」更教学。

**希腊字母/数学斜体字母仍是标识符字符**（`α`、`α'1` 逐字不变，`token.rs` 的
`non_ascii_identifiers_lex_as_ident` 守住）。`∀`（U+2200）在**码点类之内**，
但它今天就是一个 token（`TokenKind::Forall`），所以 `next_token` 的
`'∀' => self.single(TokenKind::Forall, …)` 臂**必须留在符号分支之前**——
否则 `∀` 会变成 `Sym("∀")` 而 `forall` 关键字失效。这条由
`forall_is_a_keyword_but_prefixed_idents_are_not` 与
`turnstile_and_forall_keep_their_own_tokens` 两条测试守住。

### 3.3 已知差异（对我们更宽松）

符号码点类切分让 `x∈A`（**无空格**）可用——比「要求两侧空格」更强。
「Lean 里也一样」这句话**没有取证**（WO 待确认 3）；这里记成
**我们更宽松**的已知差异。

### 3.4 `⊢`（U+22A2）与 `𝒫`（U+1D4AB）

- `⊢` 落在 `U+2200–U+22FF` 里 ⇒ 词法层变成 `Sym("⊢")`。它**不是**源语言
  token（goal 面板的 turnstile 是**渲染文本**，不是源码），所以
  `front::semantic::tag_runs` 必须把 `Sym` 归到**不产 run**（与 `->`/`=>`
  一致）——否则 goal 文本 `⊢ a` 会被当成「未声明符号」。既有测试
  `goal_runs_projects_to_hypothesis_lines_then_turnstile` 里
  「turnstile stays a plain run」这条断言**原样保留**，正是这条的守护。
- `𝒫`（U+1D4AB）与 `ᶜ`（U+1D9C）是 Unicode **字母**（数学斜体/修饰字母），
  不在符号类里 ⇒ 仍是标识符字符。第二刀不能用符号类处理它们，需要声明驱动的
  词法或专门 token（WO 待确认 4）——**第二刀选了声明驱动的词法**，见 §10.2。

## 4. 命令 / AST / 分发的落点（as-built）

| 层 | 文件 | 落点 |
|---|---|---|
| lexer | `crates/front/src/token.rs` | `TokenKind::{Str, Sym}`；`"` 分支；符号分支；`is_math_symbol`/`is_ident_start` 收窄；`lex_string` |
| parser | `crates/front/src/parser.rs` | 四条命令臂 + `parse_notation_command`；`is_reserved_command` 加四个拼写；`parse_operators(min_prec)` 取代 `parse_plus`（`+` = 保留项 65）；`starts_atom` 让 `Sym` 让路 |
| AST | `crates/front/src/ast.rs` | `Command::Notation { symbol, precedence, assoc, target, span }`、`Expr::Notation { … }`、`NotationAssoc { Infix, Infixl, Infixr, Nullary }`；`span()`/`is_import()`/`import_module()` 补臂 |
| elab | `crates/front/src/compile/elab.rs` | `Expr::Notation` 臂：`desugar_notation` → 裸变量匹配补全 → 复用 `elab_expr` |
| elab 错误 | `crates/front/src/compile/error.rs` | `ElabNotationUnknownTarget`、`ElabNotationArgumentUnsolved` |
| parse 错误 | `crates/front/src/diagnostic.rs` | `UnterminatedString`、`NotationShape`（缺优先级/缺 `=>`/符号非法/重复声明）、`NotationUnknownSymbol` |
| 分发 | `check/walk.rs`、`check/mod.rs`、`warning.rs`、`goals.rs` | `Command::Notation { .. } => {}`（无声明、无 PendingOp、不进声明表） |
| 分发 | `session.rs`、`lsp/src/lib.rs` | 已由 `_ => None` 兜住（已确认，无需改） |
| semantic | `crates/front/src/semantic.rs` | `Names.notations: HashMap<String, SemanticKind>`；`Command::Notation` 登记符号 + 目标名（`DefUse`/`TheoremUse`/…）；`TokenKind::Sym` 分类：**已声明 → Keyword**，未声明 → `continue`（不产 run）。**不新增 `SemanticKind`**（`ALL` 与 `tm_scope` 表逐字不变） |
| print-back | `crates/front/src/proof.rs` | `render_expr(Expr::Notation)` 渲染成 `lhs sym rhs`（带括号规则）；`render_atom`/`render_fun_position` 视 `Notation` 为复合式 |
| pp 复读 | 全部 `parse_expr_text` 调用点 | 内核 pp **不会**产出记法，所以回读路径不受影响；但 `render_expr` 的往返测试要能处理记法（新增，不改变既有） |
| hover | `crates/lsp/src/render.rs` | 记号节点整段 `lhs sym rhs` 一条 hover 行（`record_hover` 的 span 是记号节点 span）；`resolution`（转到定义）v1 留空 |

## 5. 影响面（事件计数 / golden）

- 事件种类与计数**不变**（记法命令不是声明，N6）⇒ **双 GOLDEN 不改**：
  `crates/cli/tests/course.rs`、`crates/cli/tests/course_status.rs`。
- 全仓 `.sokonanoda` 里这些符号只出现在注释里（复现件除外）；`infix`/`notation`
  作为标识符 0 命中 ⇒ 词法/命令表的改动**不可能翻动任何现成用例**。
- `playground.sokonanoda` anchor：无符号、无 `infix` ⇒ 不变。
- 唯一 golden 面 = 新增测试自身 + 本文件里写死的优先级表。

## 6. 与 Lean 的已知差异（v1 明说，不假装对齐）

| # | 差异 | 依据 |
|---|---|---|
| 1 | `infix` 族的脱糖是**自定**的 `p/p+1`、`p+1/p` | 待确认 1（无网络，未取证） |
| 2 | 不做「同级混用报错/需括号」的 Lean 诊断；同级左结合 | WO「不做的事」 |
| 3 | `x∈A`（无空格）可用——**我们更宽松** | 待确认 3 |
| 4 | `notation` 不接 `:N`；`infix` 族 `N` 必填 | 待确认 6 |
| 5 | ~~记法**不跨 `import`**（Lean 里全局）~~ **已修（第二刀，0.60.0）**：记法随 `import` 传播，见 §10.3/§11.7 | N5 + 闭包加载顺序 |
| 6 | ~~重复声明同一符号是**错误**（Lean 允许重载）~~ **已升级（第三刀 §14.2）**：同符号、**同形状**（结合性 + 优先级一致）= 记法重载（按期望类型选候选）；**不同形状**仍是错误；重声明 **import 来的**符号仍是错误（§11.8 原样保留） | 待确认 5；v1 自定，第三刀改 |
| 7 | 记号路径**先解操作数、后补类型参数**：操作数未定义时报既有的 `elab-unknown-identifier`，只有"操作数合法但补不出参数"才报 `elab-notation-argument-unsolved` | §2 N4 的 as-built |

## 7. 第二刀（明确不在本轮）——**已落地，见 §10–§12**

> 本节是第一刀时写下的清单，保留原文以便对照；每一项的销账情况见 §12。
> `𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 `import`、`prefix`/`postfix` 已在 0.60.0 落地。

- **Mathlib 级 + 其它形状**：`𝒫`（`prefix`，且 `𝒫` 是 Unicode **字母**）、
  `''`/`⁻¹'`（`infixr` + `'` 与今天的标识符续接字符冲突）、`×ˢ`（`infixr`，
  且 `×` U+00D7 不在数学算子区）、`⋃ i,`（`notation3` + binder）、
  `∪`/`∩`/`\`（形状同 `infixl`/`infix`，词法已就位）、`ᶜ`（postfix，U+1D9C
  同样是字母）、`scoped`/`open scoped`、集合字面量 `{a}`/`{a,b}`。
- **跨 `import` 的记法传播**。
- **print-back（源码级 pp）**：goal/hover/错误文本里的类型继续显示点名形式。
- **binder 记法**（`∃ x,`、`∀ x ∈ s,`）：台账 `notes` 原话「另立」。
- **一般隐式实参推断**（`mem2 x s` 这类点名省略仍然报错，N4.3 是护城河）。
- **`_` 占位符 / 元变量 / 一般合一**。
- **`macro`/`syntax`/自定义 parsing 框架**：v1 只有「命令 → 算子表」一条硬编码
  路径。

## 8. 测试（三层，TDD）

1. **front 单测**：`token.rs`（`Str`/`Sym` 切分、未闭合字符串、`α'1`/`∀` 回归）、
   `parser.rs`（四条命令形状 + 优先级梯子 + `+` 逐字节不变 + 未声明符号 +
   `infix` 无结合连写报错）、`compile/tests.rs`（两种写法事件一致；点名省 `α`
   仍被拒的护城河；`∅` 在无期望类型处 `elab-notation-argument-unsolved`；
   未知目标；声明之前使用）、`semantic.rs`（声明的符号 → Keyword、未声明不产
   run、`ALL`/`tm_scope` 逐字不变）。
2. **CLI e2e**：`crates/cli/tests/notation.rs` —— 同一命题两种写法各一个文件，
   跑 `--json`，断言五元组 `(decl.checked, exercise.open, expr.reduced,
   expr.typed, diagnostic)` **逐一相等**且 `exit 0`；断言事件种类集合没有新增。
3. **课程用例**：靶子 `courses/set-theory/units/unit02-subsets-empty.sokonanoda`
   的练习 2 `subset_trans` 与练习 4 `empty_subset`（答案不入库）。做法：把该单元
   连同 `lib/`、`sokonanoda.toml` 复制到临时目录，在**副本**里加记法行、把两条
   声明改成记法版，用绝对路径判卷；断言 `grade` exit 0 且事件计数与原点名版
   逐项相同。**本 WO 课程零改动**由 `python3 courses/set-theory/tools/check.py`
   全绿 + `git status` 里 `courses/` 无改动共同守护。
   > **这一条只对 WO-011 那一轮有效**（它是"记法只是糖"的对照实验）。R2 课程
   > Lean 化之后画布改用记法，契约翻转为"两种写法仍同判"——见 §15。

守护表行（`docs/TESTING.md`）：词法切分 / 优先级 / 两种写法计数一致 /
护城河（点名省 `α` 仍被拒）/ 课程门禁全绿。

## 9. as-built（2026-09-19 落地，第一百〇五轮）

设计里的 N1–N7 全部按上面落地；这一节只记**实现时才暴露出来的事实**。

1. **`∅ ⊆ A` 逼出了"操作数也要吃期望类型"**。第一版只给整条记法一个期望类型，
   于是嵌套的零元记法 `∅` 解不出 `α`（实测 `elab-notation-argument-unsolved`，
   课程用例红）。修法是 `notation_operand_expected`：从被调用者的望远镜里按
   下标取出每个操作数的期望类型（`Set.subset` 的第 1 个参数是 `Set α`），
   逐个传进 `elab_expr` 的 `expected` 槽。**这条是"期望类型传播"在记法里的
   具体形态**，将来做一般隐式实参时应能整段替换掉。
2. **展开必须直接构造内核项，不能"再 elaborate 一遍"**。第一版把补好参数后的
   源级 `Expr::App` 交回 `elab_expr`，结果操作数被 elaborate 两次、且类型在
   `Expr`/`ExprPtr` 之间来回搬（编译错 E0308）。现在 `elab_notation` 直接
   `builder.mk_const` + `builder.mk_app` 逐个挂参数，操作数各 elaborate 一次。
3. **操作数先 elaborate ⇒ 未知操作数报的是 `elab-unknown-identifier`**，不是
   记法专用码。设计初稿曾以为会报 `elab-notation-unknown-target`；实测这条只在
   **目标名写错**（`=> Set.men`）时出现。**这是更准确的诊断**（错的是那个名字，
   不是记法），已记进 §6 的已知差异表第 7 行。
4. **`elab-notation-unknown-target` 在声明行不会触发**——目标名在**使用点**解析。
   所以它的回归测试必须"声明一个拼错的目标 + 真的用一次"，只写声明行是绿的。
5. **`notation` 不带 `:N`**：零元记法走 `starts_atom` 的 `Sym` 分支（只有声明过的
   零元符号才让路），没有优先级可写；`notation:N` 报 `notation-shape`。
6. **`query`/`protocol` 的既有夹具里那条 `infix:50 " e " => mem`** 是"随便写的
   不可解析文本"，记法落地后它变成了**有意义的教学诊断**（`notation-shape`：
   符号 `e` 全是标识符字符）。三处夹具改成 `def p : Prop := (a`（真正的语法错），
   `query.rs` 的偏移量随之从 15/21 改成 19/19。**这不是放宽判据**：改后的夹具
   仍然是不可解析的文件，`query check` 仍然必须报 parse 诊断 + exit 1（G-10）。
7. **`infix`/`infixl`/`infixr`/`notation` 四个拼写进了 `KEYWORDS`**（语义高亮 +
   TM 语法），但**没有**新增 `SemanticKind`：声明过的符号复用 `Keyword`。
   因此 `SemanticKind::ALL`、`tm_scope` 表、`crates/cli/tests/extension.rs` 的两个
   词表锁**全部逐字节不变**——只在 TM 语法里加了一条**数学符号规则**
   （`keyword.operator.sokonanoda`，`U+2200–22FF`/`U+2A00–2AFF`/`\`）。
8. **课程用例用临时副本**：`crates/cli/tests/notation.rs::a_real_course_unit_grades_identically_in_notation`
   把 `courses/set-theory/{lib,sokonanoda.toml}` 复制到 `/tmp`，再写一份改写了两处
   签名的单元②。实测两份都是 `{exercise.open: 10, decl.checked: 1}`、exit 0。
   `the_shipped_course_still_uses_the_pointful_spelling` 把"课程零改动"钉住。
   > **R2 课程 Lean 化轮次已翻转**：画布现在是记法版，两条测试都改了名与方向
   > ——见 §15。

---

# 第二刀（G-04 剩余项，0.60.0）——`prefix` / `postfix` + 声明驱动的词法 + 跨 `import` 记法

> 状态：**已落地（as-built）**。本节是第二刀的设计 + as-built 记录；上面的
> §1–§9 是第一刀（0.59.0）的记录，**语义规则 N1–N7 继续有效**，本节只写增量。
> 第一刀的"第二刀清单"（§7）在本节逐条销账，销不掉的写在 §12。

## 10. 设计（增量）

### 10.1 两条新命令：`prefix:N` / `postfix:N`

与 `infix` 族**同族**：源级糖、零事件、不进声明表、文件内作用域（+ §10.3 的
跨 `import` 传播）、点名形式永久可用、两种写法判卷一致。

```
prefix:N  " sym " => name     -- `sym a` 展开成 `name <前导参数> a`
postfix:N " sym " => name     -- `a sym` 展开成 `name <前导参数> a`
```

`N` 必填、范围 1–1000（与 `infix` 族同一把尺子、同一个诊断
`parse-notation-precedence`）；符号规则、`=>` 目标规则、重复声明规则全部复用
`parse_infix_command` 的三段（`parse_notation_precedence` / `parse_notation_symbol`
/ `expect_notation_arrow`）。

**优先级规则（v1 自定，与 N3 的表并列）**：

| 结合 | 吸收条件 | 操作数怎么解析 |
|---|---|---|
| `prefix:N` | 出现在**原子位**（`parse_app` 的头部），总吸收 | 操作数 = `parse_operators(N)` |
| `postfix:N` | 在爬升循环里 `N >= min_precedence` 才吸收 | 左操作数 = 已经爬好的 `lhs` |

于是 **N 越大绑得越紧**，两条都自洽：

* `prefix:100 " 𝒫 "` ⇒ `𝒫 A ∪ B` = `(𝒫 A) ∪ B`；`prefix:50` ⇒ `𝒫 (A ∪ B)`；
* `postfix:100 " ᶜ "` ⇒ `A ∪ Bᶜ` = `A ∪ (Bᶜ)`；`postfix:50` ⇒ `(A ∪ B)ᶜ`。

AST 复用 `Expr::Notation` 的**两个已有 `Option` 槽**：前缀 = `lhs: None,
rhs: Some(操作数)`，后缀 = `lhs: Some(操作数), rhs: None`。于是
`elab_expr` 的 `[lhs, rhs].flatten()` 直接得到单操作数列表，`elab_notation`
（N4 的补前导参数机器）**一行不改**就支持一元记法。

### 10.2 声明驱动的词法（第二刀的核心词法增量）

第一刀的符号类（`U+2200–22FF`/`U+2A00–2AFF`/`\`）**收不进**卷 I 要的五个符号：

| 符号 | 码点 | 今天的词法 | 问题 |
|---|---|---|---|
| `𝒫` | U+1D4AB | `Ident("𝒫")` | 数学斜体**字母**，`is_ident_start` 为真 |
| `ᶜ` | U+1D9C | `Ident("ᶜ")` | 修饰字母，同上（且是**续接**字符 ⇒ `Aᶜ` 是一个 Ident） |
| `''` | U+0027×2 | **词法错误** | `'` 只是续接字符，不能起头 |
| `⁻¹'` | U+207B U+00B9 U+0027 | `Ident("⁻¹'")` | 上标是标识符字符 |
| `×ˢ` | U+00D7 U+02E2 | `Ident("×ˢ")` | `×` 是标识符字符 |

所以第二刀改用**声明驱动的词法**（第一刀 §3.4 预留的两条路之一）：

1. **预扫描** `token.rs::scan_notation_symbols(src)`：一趟字符扫描（**跳过
   `--` 注释与字符串字面量**）收集源码里所有记法命令声明的符号文本。状态机
   只有三态：`Start` / `Keyword`（刚读完 `infix|infixl|infixr|prefix|postfix|
   notation`）/ 之后的可选 `:N`，紧跟着的字符串字面量就是符号。
2. **带符号表词法** `tokenize_with_symbols(src, &symbols)`：`next_token` 在
   常规分支**之前**先做「声明符号最长匹配」，命中就产出 `Sym(symbol)`；
   `lex_ident` 的续接循环里也在命中处**断开**（于是 `Aᶜ` = `Ident("A") +
   Sym("ᶜ")`）。
3. `parse(src)` = 预扫描 + 带符号表词法（`tokenize(src)` 保持 `symbols = &[]`，
   逐字节等于今天）⇒ **没有记法声明的文件行为零变化**（A1 的守护不变）。

边界（写进 §11）：声明过的符号在**本文件/本闭包**里是保留的——同名标识符
会被读成符号（与第一刀的 `∈` 已经如此，一致）。

### 10.3 跨 `import` 的记法传播

**取舍：改闭包加载顺序（重解析），不是"把记法内联复制到每个单元"。**

理由（实测驱动）：内联复制要求每个用到 `𝒫`/`''` 的单元**自己重写一遍声明行**，
而声明行里的目标名是 `Set.powerset`——它由 `lib/Set.sokonanoda` 提供。复制到
单元里就成了"单元自己声明记法"，于是 ① 每个单元多 6 行样板；② 符号的重载/
遮蔽语义变成"各文件各说各话"，与 Lean「记法是 import 带来的」直接冲突；
③ 更糟的是它把"库定义记法、单元直接用"这条课程叙事变成假的。
重解析只多一趟 parse（教学规模下是微秒级），换来的是**真语义**。

**实现（`project/graph.rs`，闭包加载的最后一步）**：

1. 快速路径：闭包里**没有任何** `Command::Notation` ⇒ 直接返回，闭包逐字节
   不变（A1 与缓存摘要都不受影响）。
2. 否则按**拓扑序**（依赖在前）重解析每个未被阻断的模块：把前面模块累积的
   记法表作为**继承表**传给 `parse_with_notations(src, &inherited)`——它同时
   喂给词法（符号表）与 parser（算子表）。重解析后把本模块自己声明的记法并进
   继承表（**同符号后来者覆盖**：本地声明遮蔽继承来的）。
3. 只重解析**有继承表**的模块；自己声明的记法在第一趟 parse 里已经生效
   （预扫描看得见本文件的声明行），所以"库自己"不会被多解析一遍。

**语义**：被导入模块的记法在入口文件**从文件头就可用**（与 Lean 的 import
一致），而不是"声明之后"——"声明之后"仍然约束**本文件自己**的记法（N5）。
本地重声明同一符号 = **遮蔽**（不再报"本文件已声明"），因为那条错误是给
"同一个文件里写两遍"准备的。点名的护城河（N4.3）在第二刀**原样保留**。

### 10.4 课程库（卷 I）的五个符号

**放 `courses/set-theory/lib/Set.sokonanoda` 末尾**（不新建 `Notation.sokonanoda`）：
它们的目标全是 `Set.*`，拆出去会多一条 import 边却没有任何分层收益；而且
`lib/Set.sokonanoda` 已经是"单元 import 的那一个库"。记法**不是声明**（N6），
所以加它们**不改变任何计数**（课程门禁的 36/329/99 逐项不变）。

| 记法 | 形状 | 目标 | 目标签名（卷 I 的 L2 层） |
|---|---|---|---|
| `𝒫` | `prefix:100` | `Set.powerset` | `(α : Type) → Set α → Set α` |
| `ᶜ` | `postfix:100` | `Set.compl` | `(α : Type) → Set α → Set α` |
| `''` | `infixr:80` | `Set.image` | `(α β : Type) → (α → β) → Set α → Set β` |
| `⁻¹'` | `infixr:80` | `Set.preimage` | `(α β : Type) → (α → β) → Set β → Set α` |
| `×ˢ` | `infixr:80` | `Set.prod` | `(α β : Type) → Set α → Set β → Set (α × β)` |

（`''`/`⁻¹'` 用 `infixr:80` 是 `prior-art-report.md` §1.3 逐字取证的 Lean 声明行；
`𝒫`/`ᶜ` 取 100 = 「贴得比 `∪`/`∩`（65）紧」，`×ˢ` 取 80。这三条是**本子集
自定**，与 N3 的取证状态同级。）

前导类型参数的补全**必须**走 N4 的裸变量匹配：`Set.powerset : (α) → Set α →
Set α` 的第 2 个参数域是 `Set α`，与 `typeof(A) = Set α₀` 头部匹配 ⇒ `α := α₀`；
`Set.image : (α β) → (α → β) → Set α → Set β` 的两个前导参数分别从第 3、第 4
个参数的类型解出。**解不出就报 `elab-notation-argument-unsolved`**，不猜。

## 11. 第二刀 as-built（0.60.0）

设计里的 §10.1–§10.4 全部落地；这一节只记**实现时才暴露出来的事实**，以及
**落地后与设计初稿不同的地方**。

1. **`parse_def` 之外的第一件事：`𝒫` 之类的符号不能再用「全是标识符字符」拒绝**。
   第一刀的 `parse_notation_symbol` 用 `symbol.chars().all(is_ident_start)` 拒掉
   标识符词；`𝒫`（数学斜体字母）正好命中，第二刀当场报 `notation-shape`。改成
   **只拒纯 ASCII 标识符词**（`in`/`e`/`Set`）：它们会把整个文件里的这个名字
   都收走（`in` 还是 `infix` 的前缀）。判据抽成
   `token.rs::{is_ascii_word_symbol, lexer_reserved_symbol_char}`，**预扫描与
   parser 共用同一份**——实测踩过：预扫描若收了 `"in"`，`infix` 会被拆成
   `in` + `fix`，报出来的错完全看不懂。
2. **`''` 从「词法错误」升级为「未声明符号」**：`'` 单独出现时给 `Sym`（第一刀
   把 `\` 从词法错误改成 `Sym` 是同一个理由——诊断更教学）。副作用是好的：
   入口用了 import 来的 `''` 时，parse 报的是 `notation-unknown-symbol`，
   分发逻辑（§11.6）才认得出"这条错可能被 import 治好"。
3. **`elab_notation` 读目标签名必须用新加的 `judge::judge_type_of`**。
   `judge_infer` 那条路要先合成 `fun (binders) => target` 再逐层剥 binder，
   **目标本身是函数**时内核 pp 的多 binder 折叠会让剥离错位——实测
   `Set.image` 的签名被读成
   `(β : Sort 1) -> … -> (α : Sort 1) (β : Sort 1) -> …`，于是 `''` 的
   **两个前导参数一个都解不出**（`elab-notation-argument-unsolved`）。
   `judge_type_of` 直接 `#check <target>`，不合成 lambda、不剥——签名是常量
   自己的，与调用点的 binder 无关。**这条是第一刀就潜伏的**（`Set.mem` 恰好
   没踩到）。
4. **`notation_prefix_args` 的 `j - missing` 会下溢**：第一刀只测过
   `missing == 1`（`Set.mem`）；`Set.image` 有 **2** 个前导参数，`j=1` 时
   `1 - 2` 在 debug 构建下直接 panic（`attempt to subtract with overflow`）。
   修成 `j.checked_sub(missing)`。
5. **`rfl` 候选文本的括号清单漏了 `Notation`**：`by.rs::atom_text` 自己维护过
   一份"哪些形状要补括号"的清单，缺 `Let`/`Match`/`Notation`，于是
   `Eq.refl.{1} (Set α) (Aᶜ) ∪ B` 被读成 `(Eq.refl.{1} (Set α) Aᶜ) ∪ B`。
   现在 `atom_text` 直接调 `proof::render_atom`（**括号规则只允许有一个实现**）。
   顺带把 `rfl` 闭合用的表达式**直接构造成 AST**，不再回读候选文本。
6. **分发要认「未声明记法符号」这一条 parse 错**：入口用了依赖声明的记法时，
   `parse(entry)` 必然失败，而 CLI/LSP/真相层都是"先单独 parse、看有没有
   `import`"来决定走不走闭包——**解析失败 ⇒ 走单文件 ⇒ 永远看不到依赖的记法**。
   修法是 `project::is_project_source`：parse 成功照旧；parse 失败时**只有**
   `notation-unknown-symbol` + 源码里有 `import` 行才改判项目（别的解析错误
   加载多少依赖都还是错，必须原样报——放宽会当场把 `project_features.rs`
   的三条用例判红）。同一份判据被 `check`/`query`/`build`/`course`/真相层共用。
7. **闭包加载的顺序对调：先收 `import` 边、访问依赖，再解析自己**。
   `visit` 原来"先 parse 自己、再从 `Command::Import` 收边"，于是
   "入口 parse 失败 ⇒ 收不到边 ⇒ 依赖不加载 ⇒ 记法传不过来"是**死锁**
   （实测闭包里只剩入口一个模块）。现在用 `token::scan_import_lines`
   （词法级、与 `Lexer::on_import_line` 同款判据）先拿边、先访问依赖，拿到它们
   **导出的记法表**之后再解析自己；解析成功时 `Command::Import` 的边是权威版本。
   记法表**按 import 边合并**（不是"闭包里全局"）：没 import 的模块的记法不泄漏。
8. **重声明继承来的记法是错误，不是遮蔽**。判卷通道会把闭包**首尾相接**成一份
   合成源码（`judge.rs` 的前缀），两个模块各声明一次 `∪` 在那里必然撞车；与其
   让同一个程序在两条通道上得到不同答案，不如在源头说不许。副作用：**记法对照页
   必须自己写 `∈`/`⊆`/`∪`/`∅`**——这正好是第一刀的设计（core 级符号课程库不声明），
   所以页面的四条声明一条都不用改（lib 声明的五个是**另外**五个符号）。
9. **`by` 块目标里的记法现在判得动了**（第一刀遗留边界）。`by` 引擎把目标
   `render_expr` 成文本再 `parse_expr_text` 回读，而回读的片段里没有记法声明。
   判卷通道本来就为合成声明解析了前缀（`parse_prefix`），现在**顺手从那份
   `FolFile` 里收记法声明**（顺序对调：前缀先解析），回读时当继承表喂进去。
   零额外解析开销；前缀里没有记法命令时逐字节等于旧行为。
10. **一元记法在实参位要加括号**（`f (𝒫 A)`、`f (Aᶜ)`、`Eq.{1} (Set α) (Aᶜ) (…)`）：
    `starts_atom` 对前缀/后缀符号都是 `false`（与二元记法一致）。**这是设计选择**：
    `Eq X Aᶜ Y` 报的是响亮的 parse 错，而不是像二元记法那样悄悄按错误的结合性
    读下去。
11. **优先级梯子多了一条"绑得太松就让路"的规则**：`postfix:50 " ᶜ "` 配
    `infixl:65 " ∪ "` 时，`A ∪ Bᶜ` 必须在**外层**（`min_precedence = 0`）吸收成
    `(A ∪ B)ᶜ`。所以"已声明但不是二元算子"的专用诊断只在
    `N >= min_precedence` 时报，否则让爬升照常结束。
12. **课程侧的演示写成 `example`**：单元③/⑧ 各一条记法演示，用 `example` 而不是
   `theorem`（单元⑧ 的"演示 8"本来就这么写）——**课程门禁的 `decl.checked`
   计数因此一个都不动**（`example.checked` 是另一类事件）。实测课程门禁
   **36 目标 · 329 checked · 99 open · 0 判负**，与改前**逐项相同**。

### 11.1 验收（实测）

| 命令 | 结果 |
|---|---|
| `cargo test --workspace --locked` | exit 0（front 611 条、CLI 全绿） |
| `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check` | exit 0 |
| `cargo clippy -p sokonanoda-front -p sokonanoda-cli --all-targets --locked` | exit 0 |
| `python3 courses/set-theory/tools/check.py` | exit 0，**36 目标 · 329 checked · 99 open · 0 判负**（与改前逐项相同） |
| `python3 courses/set-theory/tools/check.py --selftest` | exit 0 |
| `python3 scripts/gap.py check` | exit 0 |
| `node editor/vscode/test-extension-host.js` | exit 0 |

五个符号各自的判卷（真二进制 + `--json`，两种写法计数逐一相等）：

| 符号 | 形状 | 目标 | 记法版 | 点名版 |
|---|---|---|---|---|
| `𝒫` | `prefix:100` | `Set.powerset` | exit 0 | exit 0 |
| `ᶜ` | `postfix:100` | `Set.compl` | exit 0 | exit 0 |
| `''` | `infixr:80` | `Set.image` | exit 0 | exit 0 |
| `⁻¹'` | `infixr:80` | `Set.preimage` | exit 0 | exit 0 |
| `×ˢ` | `infixr:80` | `Set.prod` | exit 0 | exit 0 |

跨 `import`：`courses/set-theory/lib/Set.sokonanoda` 声明五个符号，
`crates/cli/tests/notation.rs::a_library_notation_works_in_the_entry_through_import`
把**真课程库**复制到临时目录、写一个**不重声明**任何记法的单元（`𝒫 A` / `Aᶜ` /
`by rfl`）⇒ exit 0、3 条 `decl.checked`、0 诊断。

## 12. 第二刀仍未做 —— **第三刀已落（第三刀 = 本节逐条销账）**

> 本节是第二刀留下的清单（原文与理由**保留**，作为历史）；**第三刀**（0.60.x，
> 设计与 as-built 见 §14）逐条销账，结论写在下面第三列。销不掉的移到 §13。
> 第 5 条（print-back）按任务要求**明确不做**、连理由一起进 §13.1。

| # | 第二刀未做项（原文理由保留） | 第三刀结论 |
|---|---|---|
| 1 | **binder 记法**（`∃ x, …`、`∀ x ∈ s, …`、`notation3`）——"它要动的是 **binder 位置的解析**（`parse_decl_binders`/`forall`/`exists` 的共享路径），与"算子梯子上的糖"不是同一层。第二刀的预扫描/符号表对它一点用都没有；值得单独一刀（台账原话就是「另立」）" | **已落（§14.1）**：新命令 `binder_notation "∃" => Exists`；`∃ (x : α), p` 与两段式 `∃ x ∈ s, p` 落地，`∀ x ∈ s, p` 走**同一条共享 binder 路径**（`parse_binder_prefix`）。`notation3`（dependent binder）仍在 §13.2 |
| 2 | **记法重载**（同一符号多个目标，按期望类型选）——"v1 的重复声明是**错误**……重载要求展开期做候选选择 + 歧义诊断，是**语义**增量；而记法的卖点正是"零语义"。真要做，先想清楚"选错了报什么"" | **已落（§14.2）**：同符号、同形状 = 重载，按**期望类型**选；"选错了报什么"= 两个专用码 `elab-notation-ambiguous`（≥2 个候选都说得通）与 `elab-notation-no-candidate`（一个都对不上），消息列候选与各自结果类型、hint 教点名写法 |
| 3 | **`scoped` / `open scoped`**——"依赖'重载 + 作用域栈'两件都没做的事；课程侧不需要" | **已落（§14.3）**：作用域名 = 声明点所在 `namespace` 的全前缀（复用 G-05 的栈）；`open scoped Foo` **只**开记法、不开名字 |
| 4 | **集合字面量 `{a}` / `{a,b}`**——"这是**新语法**（不是记法）：`{}` 今天是 binder/宇宙参数的定界符，`{a}` 与 `{x : T}` 的消歧要新的 lookahead 规则" | **已落（§14.4）**：`Expr::SetLiteral` + `set_literal_ahead`（`{x : T}` 形状让路给 binder）；展开成点名形式 `Set.singleton α a` / `Set.pair α a b` |
| 5 | **源码级 print-back**（goal/hover 显示 `Aᶜ` 而不是 `Set.compl α A`）——"与第一刀同一条：类型文本由**冻结内核的 pp** 产出，记法不进内核" | **不做，移到 §13.1**（任务明确要求）：内核冻结下销不掉，理由在 §13.1 |
| 6 | **一元记法在实参位免括号**——"§11.10 的设计选择；免括号要求 `parse_app` 与算子梯子合并，收益（少两个括号）不值这个复杂度" | **前缀已落（§14.5）**：`f 𝒫 A` 不再报错（今天它是响亮的 parse 错 ⇒ 纯增量）。**后缀明确不做**，移到 §13.4（`f Aᶜ` 今天读作 `(f A)ᶜ`，改了会**悄悄重分组**既有程序） |
| 7 | **编辑器里 `abbrev`/记法符号的语义高亮**——"`front::semantic::KEYWORDS` 与 `editor/vscode` 的 TM 语法词表必须**同一轮**加……而本轮 `editor/vscode/**` 禁改" | **已落（主线收尾轮）**：新拼写 `prefix`/`postfix`/`binder_notation`/`scoped`（连同 `abbrev`）进 `KEYWORDS` + TM 语法词表**同一轮**同步、两份逐字相等；as-built 见 §13.6 与 `docs/design/abbrev.md` §4 |


## 13. 第三刀仍未做 / 不该做（明确销不掉的）

> 与 §12 同一条纪律：**逐条给理由**，不写"没时间"。**本节的项一律不在第三刀**。

| # | 未做项 | 理由（不是"没时间"） |
|---|---|---|
| 1 | **源码级 print-back**（goal/hover 显示 `∃ x, …` / `{a}` / `Aᶜ` 而不是点名形式） | **内核冻结下销不掉**：类型文本由**冻结内核的 pp** 产出（硬规则 1：`crates/kernel/**` 一个字节不许动），而记法**不进内核**——内核只看见 `Exists α (fun …)`。要做得二选一：① 改内核 pp（违反硬规则 1）；② 在 elab 保留"源 → 核"的映射、把所有 pp 消费点（goal 面板、hover、错误文本、`#check`、`by` 回读）换成源级渲染——那是**两套真相**，且回读路径（§11.9）会立刻分叉。收益（面板好看一点）不抵成本与风险。**任务明确要求这一项不做**，故记在这里 |
| 2 | **`notation3` / 一般 binder 记法**（`⋃ i, f i`、`∑ i, …`、多 binder 两段式 `∀ x y ∈ s, p`） | 这类 binder 的**类型依赖前一个 binder**（`⋃ i : ι, f i` 里 `f i` 的域随 `i` 变），要求记法命令能声明**依赖 telescope** 并做代换；v1 的 binder 记法只有两个类型来源（**标注** `∃ (x : α), p` 与 **guard** `∃ x ∈ s, p`，§14.1）。课程卷 I 的 `∃`/`∀` 两段式不需要它 ⇒ 留给需要 `⋃ i,` 的那一刀 |
| 3 | **一般隐式实参推断 / 元变量 / 一般合一**（第一刀 §7 的老边界，第三刀没动） | 直接后果：① 一段式 `∃ x, p` 的 x 类型**必须**写出来（`∃ (x : α), p`）或由 guard 给；② `{a, b, c}`（≥3 元素）不做折叠；③ 重载只看**结果类型**（§14.2）。要动它就得引入元变量与合一，那是 elaborate 的架构级增量，且与"记法零语义"的卖点冲突 |
| 4 | **一元后缀记法在实参位免括号**（`f Aᶜ` = `f (Aᶜ)`） | `f Aᶜ` **今天有确定读法**：后置算子在梯子上吸收整个应用 ⇒ `(f A)ᶜ`。改成 `f (Aᶜ)` 是**悄悄重分组既有程序**（违反"只加不删"），而且两者形状完全一样、无法只放行一种。要"少写括号"就写 `f (Aᶜ)`（§11.10 的选择**只对前缀松动**） |
| 5 | **`open scoped` 的子命名空间传播**（`open scoped A` 是否带进 `A.B` 的 scoped 记法） | v1 按**精确名字**匹配（`open scoped Foo` 只开 `Foo`）。理由：静默带进一整个子树的符号与"教学语法要能一眼看出名字从哪来"冲突；Lean 的精确语义**未取证**（无网络），所以先钉最保守的一条 |
| 6 | ~~**编辑器词表同步**（`binder_notation` / `scoped` 进 `front::semantic::KEYWORDS` + TM 语法）~~ **已销（主线收尾轮）** | 原文理由（历史）：两边必须**同一轮**改（`crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source` 是守护），第三刀禁改 `editor/vscode/package.json` 的版本号 ⇒ 本轮不进 `KEYWORDS`，交给主线同一轮加。**as-built**：主线把 `abbrev` + `prefix`/`postfix`/`binder_notation`/`scoped` 同一轮加进 `KEYWORDS` 与 TM 语法 `keywords` 词表（`declarations` 规则也认 `abbrev`），守护测试绿；落点与验证见 `docs/design/abbrev.md` §4 |

## 14. 第三刀（0.60.x）——设计与 as-built

> 状态：**已落地（as-built）**。落点全部在 `crates/front/**`（+ 课程单元⑧的一条演示
> + `docs/protocol.md` 的错误码表）；**`crates/kernel/**` 一个字节未动**。语义规则
> N1–N7 继续有效，本节只写增量（记 N8–N12）。

### 14.1 binder 记法（N8）

```
binder_notation "∃" => Exists        -- 命令形状与 notation 同族（不写优先级）
```

- **出现位置**：**binder 位置**（表达式开头），与 `∀` 同一层（`parse_expr` 直接分派）；
  它一直吃到表达式结尾。落在算子位上是教学错误（"binder 记法要写在表达式开头"）。
- **一段式**：`∃ (x : α), p` ⇒ `Exists α (fun (x : α) => p)`；`∃ x : α, p` 同义。
  binder 的类型**必须**写出来（或由 guard 给）——记法不引入元变量/一般合一（§13.3），
  解不出报 `elab-binder-notation-unsolved`（hint 教补标注或点名写法）。
- **两段式**：`∃ x ∈ s, p` ⇒ `Exists α (fun (x : α) => And (x ∈ s) p)`。`∈` 不必专门
  声明给 binder：它是**普通的二元记法**（`infix:50 " ∈ " => Set.mem`），binder 名当
  它的左操作数；`∀ x ∈ s, p` ⇒ `∀ x, x ∈ s -> p`（**原生 `∀`**，共享 `parse_binder_prefix`）。
- **binder 类型的来源**（两条，都不做合一）：① 标注；② 两段式由 guard 的关系目标
  （`Set.mem` 的 telescope）**反解**——用 guard 的**其它**操作数解前导参数、取 binder
  那一层的域（`Set.mem` 的第 2 个参数域是 `α` ⇒ `x : α₀`）。
- **零事件**：命令进不了声明表（N6 不变）；点名形式 `Exists α (fun …)` 永久可用，
  两种写法判卷一致（N7）。
- **命名（as-built）**：任务建议的 `notation-binder` **在本语言的词法下拼不出来**
  ——`-` 不是标识符字符（`is_ident_continue` 只收字母/`_`/数字/`'`/`!`/`?`/`.`），
  `notation-binder` 会被切成 `Ident("notation")` + `-` + `Ident("binder")`（实测：
  词法报 `expected -> or --, found -`）。定形为 **`binder_notation`**（单标识符，
  与语言里 `Set.mem_singleton_iff` 的 `_` 命名一致）。

### 14.2 记法重载（N9）

- **判据**：同符号、**同形状**（结合性 + 优先级都相同）的重复声明 = **重载**；
  不同形状仍是 `notation-shape` 错（`a ⊕ b` 与 `⊕ a` 在同一个符号上没法同时成立）。
- **重声明 import 来的符号**仍是错误（§11.8 原样保留）：判卷通道把闭包首尾相接，
  两个模块各声明一次会在那里撞车；同文件内的重载则两条通道看到同一组候选。
- **选择**：按**期望类型**筛候选——读每个候选目标签名的**结果类型**，与期望类型做
  **头部匹配**（头同名 + 实参个数相同；模板是裸变量算通配）。恰好一个 ⇒ 选它；
  ≥2 个（或没有期望类型却有多个候选）⇒ `elab-notation-ambiguous`；一个都不匹配 ⇒
  `elab-notation-no-candidate`（消息列候选与各自的结果类型）。**"选错了报什么"**：
  两个码都带人话 hint——先写点名形式消歧，或把表达式放到带类型标注的位置。
- **单候选 = 第二刀行为逐字节**：`Expr::Notation` 只**新增** `alternatives: Vec<String>`
  字段（单候选时为空向量），展开路径一行不改。

### 14.3 `scoped` / `open scoped`（N10）

- **作用域名 = 声明点所在 `namespace` 的累积全前缀**（复用 G-05 的栈）；
  `scoped` 写在 `namespace` 外是 parse 错（专用 hint 教包一层 namespace）。
- `scoped <记法命令>` 默认**不生效**（在 `open scoped` 之前用该符号报
  `notation-unknown-symbol`，与"声明之前使用"同码）；`open scoped Foo` 把 `Foo`
  作用域下**已声明**的 scoped 记法搬进生效表，之后在同作用域里声明的也直接生效。
- `open scoped` **只**开记法，**不**开名字前缀（`Foo.bar` 仍不能写成 `bar`）——
  与 Lean 一致；接线在 `compile/check/walk.rs`（`scoped: true` 不进 `ns.open`）。
- **跨 `import`**：scoped 记法随继承表传播，但**挂起**（`Parser::scoped_pending`），
  入口文件要自己 `open scoped <作用域名>`；判卷通道（`judge.rs` 收前缀记法表）按
  "前缀里有没有 `open scoped`"过滤，与主通道同口径。

### 14.4 集合字面量 `{a}` / `{a, b}`（N11）

- **新语法**（不是记法）：`Expr::SetLiteral { elements }`，**内建糖**——展开成点名形式
  `Set.singleton α a` / `Set.pair α a b`（与 `+` → `Nat.add` 同族，目标名写死在 elab 里）。
- **消歧**：`{` 后面是 binder 形状（`{x : T}` / `{x y : T}`，判据 `brace_binder_ahead`）
  就**不是**字面量（照旧走 binder 路径）；否则是字面量。因此 `starts_atom` 对 `{`
  返回 true 只在字面量形状上（`f {a}` 合法，`f {x : T}` 的报错一字未变）。
- **1–2 个元素**；空 `{}` 与 ≥3 元素给专用 parse 码 `set-literal-shape`（hint 教
  `Set.empty α` / `Set.pair` 点名嵌套）。
- **展开**复用记法路径（前导参数补全 + 操作数期望类型传播），多一条**回退解**：
  期望类型 `Set α₀` ⇒ `α := α₀`（`{∅}` 这类"元素自己的类型也只能从期望类型解"的嵌套）。
- 目标不存在（没 import 卷 I 的 `lib/Set`）⇒ `elab-set-literal-unknown-target`（hint 教
  import 或点名），不是"未知标识符"。

### 14.5 一元前缀记法在实参位免括号（N12）

- `f 𝒫 A` 现在读作 `f (𝒫 A)`（实参 = 前缀记法 + 它自己的操作数，优先级仍按 §10.1 的
  表：操作数走 `parse_operators(N)`）。
- **纯增量**：这一形状今天报**响亮的 parse 错**（"前缀记法不能夹在两个操作数中间"），
  没有任何既有程序依赖它。
- **代价（明说）**：`A 𝒫 B` 与 `f 𝒫 A` 形状完全一样，无法只放行后者 ⇒ `A 𝒫 B` 也从
  报错变成 `A (𝒫 B)`。**后缀不动**（§13.4）。
- 括号形式 `f (𝒫 A)` 逐字照旧可用（两种写法同判）。

### 14.6 as-built（实现时才暴露出来的事实）

1. **`unify_extract` 只认 `Arrow` 会漏掉内核 pp 的 `forall`**：`infer_type_text` 对
   `fun (n : Nat) => Eq n n` 返回 **`forall (n : Nat), Eq n n`**（pp 的写法），而签名里
   的 `A -> Prop` 解析成 `Expr::Arrow` ⇒ 前导参数 `A` 一个都解不出（实测
   `elab-notation-argument-unsolved`）。修法：剥层改用 `spine::peel_pi`（`Forall` 与
   `Arrow` 的**唯一**共用剥层，多 binder 折叠也归它）。**这条是第一/二刀就潜伏的**。
2. **集合字面量的元素类型回退**：`{∅}` 里 `∅` 自己的类型解不出（judge 没有期望类型），
   而既有路径的结构化匹配够不着"结果类型 vs 期望类型"（`rest` 是折叠后的 Arrow 望远镜）
   ⇒ 给 `elab_notation` 加一个**只增不改**的 `fallback_prefix_args` 参数（既有调用点
   全传 `None`，行为逐字节不变）。
3. **`scoped` 的两处接线**：`walk.rs`（`open scoped` 不进 `ns.open`）与 `judge.rs`
   （按前缀里的 `open scoped` 过滤继承记法）——两处都只加分支，既有路径不变。
4. **`judge_cache` 容量是 FIFO 的 128**：第三刀新增的记法测试把缓存填到上限，
   `judge::tests::judge_cache_returns_identical_results_and_stores_entries` 里
   "长度变大 ⇒ 键进去了"的判据**假红**（第二刀也踩过同一条，当时靠拆出 `type_cache`
   缓解）。改成**直接问键在不在**（新增 `#[cfg(test)] judge_cache_contains`），
   与容量无关——判据更准，不是放宽。
5. **`∃` 与 `∀` 的**不对称是设计**：`∀` 是语言关键字（`TokenKind::Forall`，原生 Pi），
   `∃` 是**库里的归纳**（`lib/Exists`），所以 `∃` 必须由 `binder_notation` 说出目标名；
   两者在 binder 位置上共享同一条解析路径（`parse_binder_prefix`）。
6. **课程侧演示写成 `example`**（与第二刀 §11.12 同款）：单元⑧ 的 `∃ (x : α), …` 演示
   不产生 `decl.checked` ⇒ 课程门禁计数**逐项不动**（实测见 §14.7）。
7. **`semantic::KEYWORDS` 在第三刀一字未改**（§13.6）：新拼写当时不进编辑器词表，
   `SemanticKind::ALL`、`tm_scope` 表、TM 语法词表锁全部**逐字节不变**；声明过的符号
   仍按 `Keyword` 着色。**主线收尾轮已销**：`abbrev` 与 `prefix`/`postfix`/
   `binder_notation`/`scoped` 同一轮进 `KEYWORDS` + TM 语法（`docs/design/abbrev.md` §4），
   第三刀当时的 as-built 结论本身不变。

### 14.7 验收（实测）

| 命令 | 结果 |
|---|---|
| `DEVELOPER_DIR=… cargo test --workspace --locked` | **exit 0**（front lib 627 条、CLI 全绿） |
| `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check` | **exit 0** |
| `cargo clippy -p sokonanoda-front -p sokonanoda-cli --all-targets --locked` | **exit 0** |
| `python3 courses/set-theory/tools/check.py` | **exit 0**，**36 目标 · 329 checked · 99 open · 0 判负**（改前/改后逐项相同） |
| `python3 courses/set-theory/tools/check.py --selftest` | **exit 0** |
| `python3 scripts/gap.py check` | **exit 0** |
| `node editor/vscode/test-extension-host.js` | **exit 0** |

五项各自的判卷（真二进制 + `--json`，两种写法五元计数逐一相等）：

| 项 | 记法写法 | 点名写法 | 结论 |
|---|---|---|---|
| §14.1 binder | `∃ (n : Nat), Eq.{1} Nat n n` | `Exists Nat (fun (n : Nat) => Eq.{1} Nat n n)` | exit 0，计数相等（`crates/cli/tests/notation.rs::binder_notation_grades_like_the_pointful_exists`） |
| §14.1 两段式 | `∀ x ∈ s, p x` / `∃ x ∈ s, p x` | `forall (x : α), Set.mem α x s -> p x` / `Exists α (fun (x : α) => And (Set.mem α x s) (p x))` | exit 0，计数相等（`two_stage_binders_grade_like_the_pointful_guard`） |
| §14.2 重载 | `prefix:100 " ι "` 两条 + 期望类型 | —— | exit 0（选得对）；歧义 ⇒ `elab-notation-ambiguous` + exit 1（`an_overload_grades_by_expected_type_and_reports_ambiguity`） |
| §14.3 scoped | `namespace Foo` + `scoped infix…` + `open scoped Foo` | 同命题点名版 | 未 open ⇒ `notation-unknown-symbol` + exit 1；open 后 exit 0、计数相等（`a_scoped_notation_grades_only_after_open_scoped`） |
| §14.4 集合字面量 | `{a}` / `{a, b}` | `Set.singleton α a` / `Set.pair α a b` | exit 0，计数相等（`set_literals_grade_like_the_pointful_singleton_and_pair`） |
| §14.5 实参位免括号 | `f α 𝒫 A` | `f α (𝒫 A)` | exit 0（`a_prefix_notation_argument_keeps_the_parenthesised_spelling_working`） |

课程侧：单元⑧ 加一条**演示**（`binder_notation "∃" => Exists` + `example … := ∃ (x : α), …`），
练习数量与题意一字未动 ⇒ 门禁的 36/329/99/0 **逐项相同**；
`crates/cli/tests/notation.rs::the_shipped_course_demos_the_binder_notation` 把这条钉住。

---

## 15. R2 课程 Lean 化之后：契约翻转与 `≠` 的层级提示（2026-09-21）

R2（课程全面 Lean 化，设计 `docs/design/course-lean-style.md`）把**卷 I 单元②**
画布与解答都改成了记法版，于是本节 §8/§14.7 里两条"课程仍用点名"的记录
**到此为止**。翻转后的契约（测试名与方向都改了）：

| 旧 | 新 | 现在钉住什么 |
|---|---|---|
| `a_real_course_unit_grades_identically_in_notation`（§8 第 8 条、§14.7） | `a_real_course_unit_grades_identically_in_either_spelling` | 方向反转：画布是**记法版**，夹具 `pointful_variant` 造**点名版**；两份仍必须 `exit 0` 且五元计数相等（N7 契约不变） |
| `the_shipped_course_still_uses_the_pointful_spelling` | `the_shipped_course_uses_the_library_notation` | ① 单元**不出现** `infix`/`notation` 行（符号由 `lib/Set` 统一声明、随 `import` 传播）；② 六条记法签名真的在（含 `{a}`/`{a, b}` 字面量与 `≠`）；③ 画布**零点名残留**；④ 每条 `theorem` 的值位都是 `by`（tactic 风格） |

**`≠` 的 delta 展开必须带层级**（同一轮修的判卷器缺口）：`Ne` 是
`def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False`，
而 `≠` 的两条路**都不带 `.{u}`**——源 AST 是记法节点，过一遍内核 pp 是裸名
`Ne`（pp 省掉隐式宇宙参数）⇒ `intro h` 展开出来的 `h` 是 `@Eq.{u} …`（悬空
变量），回读报 `unknown universe level u`（报错点离根因很远）。
修法：`by` 引擎按**操作数的 sort** 算层级提示（与 `elab_notation` 给记法求层级
**同一条规则**），两档算法不同且都必须对：

| 形态 | 例 | 首实参是 | 层级 |
|---|---|---|---|
| 点名（实参 ≥ 形参） | `Ne (Set α) A B` | 那个类型参数自己 | `sort(首实参)`（**一步**；两步会得 `2`） |
| 记法（实参 < 形参） | `A ≠ B` | 该项的项 | `sort(type(首实参))`（**两步**；一步会得 `0`） |

回归：`crates/cli/tests/notation.rs::inequality_delta_unfolding_carries_the_right_universe_level`
（四条：点名对照 / `intro` 记法形态 / `apply` 点名形态 / `Prop` 档）。
判定仍在内核——层级错一条都过不了，所以"这些证明能过"本身就是判据。
