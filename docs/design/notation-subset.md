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
  词法或专门 token（WO 待确认 4，本轮只记录）。

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
| 5 | 记法**不跨 `import`**（Lean 里全局） | N5 + 闭包加载顺序 |
| 6 | 重复声明同一符号是**错误**（Lean 允许重载） | 待确认 5；v1 自定 |
| 7 | 记号路径**先解操作数、后补类型参数**：操作数未定义时报既有的 `elab-unknown-identifier`，只有"操作数合法但补不出参数"才报 `elab-notation-argument-unsolved` | §2 N4 的 as-built |

## 7. 第二刀（明确不在本轮）

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
