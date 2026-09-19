# WO-011 notation / infix：用户自定义记法（G-04）

> 台账行：`docs/gaps/ledger.jsonl` 第 15 行（`id=G-04`、`kind=language`、`severity=blocker`、
> `status=open`、`wo=null`、`wo_planned=null`、`fixed_in=null`）。本 WO 落地后应把该行 `wo`
> 指向本文件、`status` 改 `wo-filed`（见文末「关账」）。台账 `notes` 原话已经给了本 WO 的定位：
> 「建议先做最小子集（infix + 常量记法），binder 记法（`∃ x, …`）另立」。
> 写这一轮**没有改任何源码、复现件、台账或课程文件**，只创建了本文件；下文每个技术判断后面都
> 标了依据（跑过的命令 / 读到的 `文件:行号`）。本轮**无网络**（`curl` 与 `web_fetch` 对
> `raw.githubusercontent.com`／`cdn.jsdelivr.net` 全部超时），因此 Lean 侧的断言一律引仓库内
> 已有的逐字取证（`docs/notes/settheory-survey/prior-art-report.md` §1.3），没有二次取证的一律标
> **待确认**。
>
> **一句话结论**：复现只暴露了「命令不认识」这一层；真做起来会撞到三件事，全部在写 WO 时实跑出来了：
> ① 词法层**没有字符串 token**（`"` 直接报 unexpected-token，`token.rs:249-252`）——记法命令的符号读不进来；
> ② 符号字符今天**是标识符字符**（`is_ident_start` 把 `≥0x80` 一律当标识符首字符，`token.rs:316`），
> 所以 `a∈b` 是**一个** Ident（实测 `unknown identifier \`a∈b\``）——要么改词法，要么把「记号两侧必须空格」写进白名单；
> ③ **`∈` 藏着一个类型参数**：课程库是 `Set.mem (α : Type) (a : α) (A : Set α)`
> （`courses/set-theory/lib/Set.sokonanoda:21`），而本语言**不插入隐式实参**
> （实测 `mem2 x s` 被内核拒绝、`mem2 T x s` 通过，见下表 P7）⇒ 记法**不是纯 parser 糖**，
> 第一刀必须同时给「记号展开路径」加一条**最小的类型参数补全**，否则
> `infix:50 " ∈ " => Set.mem` 会把 `a ∈ A` 展开成一个被内核拒绝的项。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴；**绝对路径**，`$PWD` 展开后见下）：

  ```bash
  cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
  scripts/soko grade "$PWD/docs/gaps/repro/G04-notation.sokonanoda"; echo "exit=$?"
  ```

- 今天的实际输出（2026-09-18 实跑，0.58.0；`Cargo.toml:6` = `0.58.0`）：

  ```text
  {"code":"unexpected-token","hint":"这里的写法不符合当前课程语法。检查命令拼写、括号配对，以及是否多写了还没学过的符号。","message":"expected a valid .sokonanoda token, found \"","span":{"end":{"column":10,"line":18,"offset":947},"start":{"column":10,"line":18,"offset":947}},"stage":"parse","type":"diagnostic"}
  exit=1
  ```

  诊断落在复现件第 18 行 `infix:50 " ∈ " => mem` 的**第 10 列**——正是那个 `"`；
  也就是说 `infix`/`:`/`50` 都已经被当成普通标识符读过去了，卡在**字符串字面量**上
  （`crates/front/src/token.rs:249-252` 的 `other =>` 臂）。

- **同一条命令的 `query` 视图是假绿的**（台账 G-10，`status=wo-filed`〔`WO-003`〕）：
  `scripts/soko query check --file "$PWD/docs/gaps/repro/G04-notation.sokonanoda"`
  → `{"ok":true, ... "counts":{全 0}, "failed":[]}`、**exit 0**。
  ⇒ 本 WO 的一切验收**只看 `grade` 的退出码与事件流**（课程线纪律，`courses/set-theory/AGENTS.md:19-20`）。

### 附 A：探针实测（同一台 0.58.0；P1–P6 用 `/tmp/g04*.sokonanoda`，P7/P8 用 `/tmp/g04root/`（内含 `lib/` 副本））

| # | 输入要点 | `grade` 输出要点 | exit |
|---|---|---|---|
| P1 | `def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a` + `def probe (α : Type) (a : α) (A : α -> Prop) : Prop := a ∈ A` | `elab-unknown-identifier`「unknown identifier `∈`」@ `3:59` | 1 |
| P2 | `... := a∈b`（**无空格**） | `unknown identifier \`a∈b\``——**一个 token** | 1 |
| P3 | `... := A⊆B`（**无空格**） | `unknown identifier \`A⊆B\`` | 1 |
| P4 | `... := a∅A` | `unknown identifier \`a∅A\`` | 1 |
| P5 | `infix " ∈ " => mem`（缺优先级） | 与复现同一条 parse 错，落点仍在 `"` | 1 |
| P6 | `notation:max "∅" => mem` | 同一条 parse 错，落点 `"`（`3:14`） | 1 |
| P7 | `def mem2 {T : Type} (x : T) (s : T -> Prop) : Prop := s x`，三种调用 | `mem2 T x s` ✅ checked；`@mem2 T x s` ✅ checked；**`mem2 x s` ✗ `kernel-rejected`**「类型不匹配：期望 `Sort(1)`，实际是 `$2`」 | — |
| P8 | `import lib.Set` 的真课程库：`def p2 (α : Type) (a : α) : Prop := Set.mem α a Set.empty`；`(Set.empty _)` | 前者 `kernel-rejected`「期望 `(Set.[] $1)`，实际是 `Pi (α : Sort(1)), (Set.[] $0)`」；后者 `elab-unknown-identifier`「unknown identifier `_`」 | 1 |

**P7/P8 是这份 WO 最重要的一组事实**（它决定「第一刀能不能只改 parser」）：

- 隐式 binder **语法上存在**（`{T : Type}` 能声明、`@` 能显式给），但**应用时不插入**：
  `mem2 x s` 不是被 elab 拦下，而是 front 把 `App` 直接拼出来
  （`crates/front/src/compile/elab.rs:949-955`：`mk_app(fun, arg)` **不做元数/类型检查**）
  交给内核，内核报类型不匹配。⇒ 「省略 `α` 让 elaborate 自己推」这条**今天不存在**。
- 同理 `_`（占位符）也不存在：`TokenKind::Hole` 在枚举里（`token.rs:10`）但**词法器从不产出**
  （`?` 被专门拒绝并提示写 `sorry`，`token.rs:176-191`；`_` 走 `is_ident_start` 变成
  `Ident("_")` → `unknown identifier \`_\``）。
- 课程库的 `Set.mem` / `Set.subset` / `Set.empty` 都把 `α` 写成**显式**参数
  （`courses/set-theory/lib/Set.sokonanoda:21/23/25`），所以第一刀的 `∈`/`⊆`/`∅`
  **每一个都要面对这个省略**。

### 附 B：与实现直接相关的三条现状（都读到了行号）

1. **每文件独立 parse**：项目闭包加载时，**先 `parse` 当前文件、再收集 import 边**
   （`crates/front/src/project/graph.rs:227` 的 `parse(&text)` 早于 `:250-265` 的 import 收集）
   ⇒ 「记法写在 `lib/`，`units/` 直接用」**今天拿不到**：入口文件 parse 时还不知道依赖里声明了什么。
   第一刀按**文件内作用域**设计（见下 N5）。
2. **缓存键 = 源文本**：`crates/front/src/compile/cache.rs:83-98`（`key_parts(format, version,
   build, prelude_bare, src)`），项目摘要把**所有模块源文本**混进哈希
   （`crates/front/src/project/mod.rs:122-147`）⇒ 加了记法行的文件自然换键，**缓存无需改动**。
3. **全仓的 `.sokonanoda` 里这些符号只出现在注释里**（复现件除外）：`grep -rn "∈\|⊆\|∅\|∪\|∩\|𝒫"
   --include=*.sokonanoda .` 后再滤掉「行首 `--`」的注释行，**全仓唯一的非注释命中就是
   `docs/gaps/repro/G04-notation.sokonanoda:18` 那条失败的 `infix:50 " ∈ " => mem` 本身**；
   `\b(infix|infixl|infixr|notation)\b` 的非注释命中同样只有这一条（其余全是注释，例如
   `course/en/unit9-relations-connectives.sokonanoda:85`、`courses/set-theory/units/unit03-…:24`）。
   注释在词法层被整行跳过（`token.rs:114-120`）。
   ⇒ 词法/命令表的改动**不可能翻动任何现成用例**（这是「双 GOLDEN 不变」的证据之一）。

## 期望行为

### 官方 Lean 4 的逐字取证（引仓库内已有报告，`prior-art-report.md` §1.3）

| 记法 | 声明原文（已验证，逐字） | 位置 | 层级 |
|---|---|---|---|
| `∈` | `notation:50 a:50 " ∈ " b:50 => Membership.mem b a`（**交换**了操作数：集合在前） | **Lean core** `src/Init/Notation.lean:420,422` | core |
| `⊆` / `⊂` | `infix:50 " ⊆ " => Subset` | **core** `src/Init/Core.lean:539,542` | core |
| `∅` | `notation "∅" => EmptyCollection.emptyCollection`（**无 `:max`、无 precedence**） | **core** `src/Init/Core.lean:581` | core |
| `∪` / `∩` / `\` | `infixl:65 " ∪ " => Union.union`；`infixl:70 " ∩ " => Inter.inter`；`infix:70 " \ " => SDiff.sdiff` | **core** `Init/Core.lean:551,554,560` | core |
| `𝒫` | `prefix:100 "𝒫 " => powerset`——**不是 scoped**（任务书早先猜 scoped，实为全局） | Mathlib `Data/Set/Defs.lean:261` | Mathlib |
| `''` / `⁻¹'` | `infixr:80 " '' " => image` / `infixr:80 " ⁻¹' " => preimage` | Mathlib `Data/Set/Operations.lean:144,138` | Mathlib |
| `×ˢ` | `infixr:82 " ×ˢ " => SProd.sprod` | Mathlib `Data/Sprod.lean:31,36` | Mathlib |
| `⋃ i,` / `⋃₀` | `notation3 "⋃ " …, " r:60:(scoped f => iUnion f) => r`；`prefix:110 "⋃₀ " => sUnion` | Mathlib `Order/SetNotation.lean` | Mathlib |
| `↾` | **根本不存在**（`Data/Set/Restrict.lean` 全文无任何记法；`Set.restrict` 已改名 `domRestrict`） | — | **不要教** |

### 第一刀 / 第二刀：分层与理由

- **第一刀（本 WO）＝ 通用 `infix` 形式 + core 级最小记法集 `∈` / `⊆` / `∅`。**
  理由（`docs/design/set-theory-syllabus.md:185-190` 的取证块 + 本 WO 附 A）：core 级记法是
  **任何 Lean 读者默认拥有**的东西（和 `Eq` 平级，属于「最小记法集」），形状单一
  （只有一个二元 + 一个零元），而且课程库**已经有**逐字同款的 `Set.mem`/`Set.subset`/`Set.empty`
  可挂（`lib/Set.sokonanoda:21/23/25`）。
- **第二刀（明确不在本 WO）＝ Mathlib 级 + 其它形状**：`𝒫`（`prefix`，且 `𝒫` 是 Unicode
  **字母** U+1D4AB，不是符号——词法上要另设计）、`''`/`⁻¹'`（`infixr` + `'` 与今天的
  标识符续接字符冲突，`token.rs:320-322`）、`×ˢ`（`infixr`，且 `×` U+00D7 不在数学算子区）、
  `⋃ i,`（`notation3` + binder）、`∪`/`∩`/`\`（形状同 `infixl`/`infix`，但 `\` 今天是词法错误，
  `token.rs:249`）、`ᶜ`（postfix，U+1D9C 同样是字母）、`scoped`/`open scoped`、集合字面量 `{a}`/`{a,b}`。
  理由：这些各自需要**新的记法形状**（prefix/postfix/binder）或 **locale 语义**，值得单独一刀；
  把它们塞进第一刀会把「核心记法可用」这个目标拖成一次大改。

### 本教学子集的边界（v1 语义规则，逐条可测）

- **N1 命令形状**（白名单只加这四个拼写）：
  - `infix:N " sym " => name`（无结合，两边同级）
  - `infixl:N " sym " => name`（左结合）、`infixr:N " sym " => name`（右结合）
  - `notation " sym " => name`（**零元**常量记法，用于 `∅`；不写优先级，照 core 的 `∅` 行）
  - `N` **必填**（`infix` 族）；`N` 的合法范围由设计文档定（建议 1–1000，即严格落在
    `->` 与函数应用之间），越界给专用诊断而不是静默取整。
  - 符号是**字符串字面量**；两侧空格是 Lean 的书写习惯（core 逐字都带空格），v1 取 `trim` 后的内容。
  - 目标 `name` 是点名（可带 `Set.` 前缀，走既有点分名字解析）；未知目标给专用诊断。
- **N2 使用**：`lhs sym rhs` 只要求在**同一文件、声明之后**（见 N5）；操作数顺序**保持**
  （与 Lean `infix` 糖一致）。core 的 `∈` 是**显式 `notation` 并交换操作数**
  （`Membership.mem b a`），那需要具名占位（`notation:50 a:50 " ∈ " b:50 => …`）——**第二刀**；
  第一刀用保序 `infix:50 " ∈ " => Set.mem` 对准课程库的 `Set.mem (α) (a) (A)`。
- **N3 优先级**：所有记法算子与内建 `+` 共用**一条优先级梯子**，位置在
  `parse_arrow`（`->`，最松）与 `parse_app`（函数应用，最紧）之间——今天的 `parse_plus`
  就在这个缝里（`parser.rs:774` 调 `:848-861` 的 `parse_plus`）。
  `+` 作为**内建保留项**进梯子并**逐字节保持今天的行为**（仍产出 `Expr::Plus`）。
  两个不同算子同级时的结合规则：v1 自定为左结合并在设计文档写明；**Lean 的
  「优先级冲突报错」语义明确不做**（见「不做的事」）。
- **N4 展开（本 WO 的核心，见下节）**：`lhs sym rhs` → 目标名 + 操作数；目标 telescope
  比操作数多出来的**前导 `Type` 参数**按固定顺序补全。`notation "∅" => Set.empty` 同理
  （零操作数 ⇒ 只能从期望类型补）。
- **N5 作用域**：**文件内、声明之后**（file-local）。跨 `import` 的记法**第二刀**
  （附 B.1：`graph.rs:227` 逐文件 parse，做跨模块要么改闭包加载顺序、要么二次 parse，
  两条路的代价都写进设计文档再定）。
- **N6 记法不产生事件、不是声明**：`infix`/`notation` 命令**不**产生 `decl.checked`/`exercise.open`/
  `expr.typed`/`expr.reduced`，也不进声明表（与 `Command::Import` 同族：有命令、无声明）。
- **N7 教学契约（验收的语言）**：同一命题的两种写法——点名 `Set.mem α a A` 与记法 `a ∈ A`——
  **判卷结果一致**（`grade` 退出码一致 + 五元事件计数一致），且**点名形式继续可用**。
  注意契约**不含「打印文本一致」**：目标/hover 的类型文本仍由**冻结内核的 pp** 产出，
  会显示点名形式（见「不做的事」）。

### N4 的展开规则（写给实现者的契约）

1. 展开发生在 **elaborate 阶段**（不是 parser）：parser 只把 `lhs sym rhs` 记成一个带目标名的
   记号节点；elab 把它**源到源**降级成既有的 `App` 形状，之后 elab/hover/kernel/事件全部复用既有路径。
   可用的钩子先例：`crates/front/src/compile/elab.rs:790-802`（进主 `match` 前的源到源改写调用点）
   与 `:1750-1761`（`annotate_application_lambda`，应用位置的 binder 类型推断，用 `judge_infer`）。
2. 补全**只允许**发生在记号展开路径：目标 telescope 的前导 `Type` 参数，按
   **① 由操作数解出 → ② 无操作数时由期望类型解出** 的顺序。
   最小可实现的形态：目标剩余 telescope 与操作数类型/期望类型做**裸变量匹配**
   （`Set.mem : (α : Type) → (a : α) → (A : Set α) → Prop`，第 2 个参数的类型就是裸变量 `α`
   ⇒ `α := typeof(a)`；`Set.empty : (α : Type) → Set α`，结果类型 `Set α` 对上期望 `Set α₀`
   ⇒ `α := α₀`）。**不支持**一般合一/元变量；解不出给专用错误码
   （建议 `elab-notation-argument-unsolved`，hint 直接教怎么写点名形式）。
3. **回归护栏（这条是兼容性的关键）**：`Set.mem a A`（点名、省 `α`）**今天被内核拒绝，
   改后必须仍被拒绝**——补全只挂在记号路径上，不改变任何既有程序的语义。
4. 不许动内核：展开只产出 `mk_const`/`mk_app`（`elab.rs:884`、`:952` 已有原语）。

## 范围（逐项，含文件与行号）

| 层 | 文件:行号 | 改什么 |
|---|---|---|
| lexer | `crates/front/src/token.rs:6-26`（`TokenKind`） | **新增** `Str(String)`（字符串字面量）与 `Sym(String)`（记法符号）；`:98-254` 的 `next_token` 加 `"` 分支（含**未闭合字符串**诊断）与符号分支（最大吞噬） |
| lexer | `crates/front/src/token.rs:315-322` | **收窄** `is_ident_start`：`(c as u32) >= 0x80` 改为「≥0x80 且**不属于**数学符号码点集」；`is_ident_continue` 同步。码点集第一刀取 `U+2200–U+22FF`（`∈`U+2208/`⊆`U+2286/`∅`U+2205/`∪`U+222A/`∩`U+2229/`∘`U+2218 都在内）∪ `U+2A00–U+2AFF`（为第二刀的 `⋃`/`⋂` 预留）∪ `\`(U+005C，今天直接是词法错误，改为「未声明符号」诊断更教学)；**希腊字母/数学斜体字母仍是标识符字符**（`α`、`α'1` 必须逐字不变，`token.rs:421-426` 的测试守住） |
| lexer | `crates/front/src/token.rs:337-465`（tests） | 新增词法测试（见「验收」） |
| parser | `crates/front/src/parser.rs:87-101`（命令表） | 加 `infix` / `infixl` / `infixr` / `notation` 四条命令臂 + 各自的 `parse_*` |
| parser | `crates/front/src/parser.rs:1296-1313`（`is_reserved_command`） | 四个新命令**必须**加入（否则 `infix` 能当变量名，且 `starts_atom` 会把命令当实参）；`:1289-1294` 的 tactic 白名单**不动** |
| parser | `crates/front/src/parser.rs:560-568` / `:748-786` / `:848-861` / `:863-875` | 表达式层：`parse_plus` 升级为**优先级爬升** `parse_operators(min_prec)`（`parse_arrow:774` 的调用点改一次）；内建 `+` 作为保留项（优先级 65，**待确认**）产出今天的 `Expr::Plus`；记法算子产出记号节点 |
| parser | `crates/front/src/parser.rs:877-891`（`starts_atom`） | 记法符号**不得**被当作应用实参（今天 `Ident("∈")` 会走进 `Ident` 臂被吃掉）；未声明的符号给专用诊断（hint：先声明 / 用点名形式） |
| parser | `crates/front/src/parser.rs:66-85`（`parse_file`） | 记法命令按「非 import 命令」计数 ⇒ 之后的 `import` 仍报「必须置顶」（既有行为，无需改，但要有测试钉住） |
| AST | `crates/front/src/ast.rs:205-260`（`Command`） | 新增 `Command::Notation { symbol, precedence, assoc, target, span }`（零元常量记法用同一命令、`assoc` 记 distinct 值或 `arity: 0/2`） |
| AST | `crates/front/src/ast.rs:286-298` / `:300-303` / `:305-311` | `span()` / `is_import()` / `import_module()` 三处的穷尽 `match` 补臂 |
| AST | `crates/front/src/ast.rs:14-…`（`Expr` 枚举，`:14` 起） | 新增记号节点（如 `Expr::Notation { symbol, lhs, rhs, target, span }`）；`:150` 的 `Tactic` 枚举**不动** |
| elab | `crates/front/src/compile/elab.rs:790-802`（改写钩子）/ `:949-955`（`App` 臂）/ `:1750-1761`（先例） | 记号节点的降级 + N4.2 的前导 `Type` 参数补全；`elab.rs:867-885` 的 `unknown identifier` 路径**不动** |
| elab | `crates/front/src/compile/error.rs`（`ErrorKind` 表） | 新增诊断码：命令形状类（`parse-notation-*`）、未声明符号、未知目标、`elab-notation-argument-unsolved` |
| 命令分发 | `crates/front/src/compile/check/walk.rs:114-160` | `Command::Notation { .. } => {}`（无声明、无 PendingOp） |
| 命令分发 | `crates/front/src/compile/check/mod.rs:355-364`、`:622-647` | 同上一行（两处穷尽 `match` 补臂） |
| 命令分发 | `crates/front/src/compile/warning.rs:88-97` | 同（`continue`） |
| 命令分发 | `crates/front/src/compile/goals.rs:78`、`:210-215`、`:289-299` | 三处 `match command` 补臂（不参与 goal/模板提取） |
| 命令分发 | `crates/front/src/session.rs:386-392` | 同（`_ => None` 已兜住，但**必须确认**没有别的穷尽 match；实现者全仓 `grep "Command::"` 过一遍） |
| 命令分发 | `crates/lsp/src/lib.rs:830-835` | 同（`_ => return None` 已兜住） |
| pp | `crates/kernel/**` | **不动**。目标/hover 的类型文本继续由内核 pp 产点名形式；**没有** front 侧源码级 printer（全仓无此设施） |
| semantic | `crates/front/src/semantic.rs:324-335`（`Names`） | 加一个「本文件已声明记法符号 → kind」的表（建议 `notations: HashMap<String, SemanticKind>`） |
| semantic | `crates/front/src/semantic.rs:371-480`（`collect_names`）/ `:161-187`（`declaration_kinds`） | `Command::Notation` 臂：登记符号，**符号本身**归 `SemanticKind::Keyword`（先例：`∀` 的 `Forall` token 就归 Keyword，`:226`/`:629`）、目标名归既有 `DefUse/TheoremUse` |
| semantic | `crates/front/src/semantic.rs:587-615`（`classify_ident`）/ `:617-640`（`classify`） | `TokenKind::Sym` 的分类：**已声明 → Keyword**，未声明 → 与 `->`/`=>` 一致地不产出 run（`continue`）。**不新增 `SemanticKind`**（v1 免动 `:83-100` 的 `ALL` 与 `:105-124` 的 `tm_scope` 表、免动 `:983` 的锁表测试） |
| hover | `crates/front/src/compile/report.rs:148-160`（`HoverType`）/ `crates/lsp/src/render.rs:74`、`:96`、`:201-230` | 记号节点的降级要**覆盖整段 `lhs sym rhs`**，否则悬停只覆盖操作数；`resolution`（转到定义）v1 留空（见「不做的事」） |
| 补全 | `crates/lsp/src/lib.rs:1374-1398` | 关键字/宇宙/prelude 列表**不动**；可选加「本文件已声明的记法符号」（需要文档自身的解析结果，属加分项，不做也不算缺） |
| inlay / rename | `crates/lsp/src/inlay.rs:11`、`crates/front/src/references.rs:27-50` | **不动**（记法符号不是可重命名的绑定；转到定义留第二刀） |
| project | `crates/front/src/project/graph.rs:227` | **不动**（v1 文件内作用域；跨模块见 N5 与「不做的事」） |
| cache | `crates/front/src/compile/cache.rs:83-98` | **不动**（键含源文本，加了记法行自然换键） |
| 白名单文档 | `docs/architecture.md:97-148`（§4.1） | 命令清单加四个命令 + 记法子集边界（N1–N7）——硬规则 4「语法白名单即课程」的落点 |

- **是否动内核：否**（预期且必须）。展开只用到 `builder.mk_const` / `mk_app`
  （`elab.rs:884`、`:952`），`crates/kernel/**` 一行不动（`REQUIREMENTS.md:15` 第 1 条冻结快照）。
- **是否动课程内容：否**（见「兼容策略」第 2 条）。

## 不做的事（明确排除，防顺手扩大）

- **不做 `macro` / `syntax` / 自定义 parsing 框架**：v1 只有「命令 → 算子表」这一条硬编码路径，
  没有用户可扩展的 parser 组合子（`notation3`、`syntax` cat 都在此列）。
- **不做 binder 记法**：`∃ x,`、`∀ x ∈ s,`、`⋃ i,`（台账 `notes` 原话「binder 记法另立」；
  `notation3` 是 Mathlib 层）。
- **不做 `scoped` / `open scoped` / locale**：`𝒫` 在 Mathlib 里**不是 scoped**，而 `#s`/`#α`
  才是两个不同 locale（`prior-art §1.3`）——locale 语义没有第一刀的用处。
- **不做优先级冲突的完整 Lean 语义**：Lean 的「同级混用报错/需要括号」那套诊断不实现；
  v1 用固定规则（同级左结合）并在白名单文档写明。
- **不做隐式实参的一般推断**（`mem2 x s` 这类**点名**省略仍然报错，见 N4.3）：
  补全只挂在记号展开路径——这是本 WO 兼容性的护城河。
- **不做 `_` 占位符 / 元变量 / 一般合一**：那是另一条缺口（本地未登记，本轮只记录，见「待确认 7」）。
- **不做 print-back（源码级 pp）**：goal/hover/错误文本里的类型继续显示点名形式；
  把 `Set.mem α a A` 打回 `a ∈ A` 要么碰冻结内核的 pp、要么新写一个 front 侧 printer —— 都不做。
- **不做 `∪`/`∩`/`\`/`𝒫`/`''`/`⁻¹'`/`×ˢ`/`⋃`/`ᶜ`/`{a}` 的记法**（第二刀，见分层理由）。
- **不做跨 `import` 的记法传播**（第二刀）。
- **不做 `notation:max` / `prefix` / `postfix` / `infixr` 的「使用案例」**（`infixl`/`infixr`
  的**拼写**可以顺手支持，但第一刀只用 `infix:50` 与零元 `notation`）。
- **不在本 WO 里让课程改用记法**：课程采用是课程线自己一轮的事（`syllabus §4` 已定「先点名、
  后记法」，`:219-220`）；本 WO 只提供能力与等价性证据。

## 兼容策略

1. **记法只是糖，点名形式一字不改**：点名路径的任何行为（含今天的错误）都不变；
   N4.3 的回归测试就是这条的守护。
2. **课程零改动**（本 WO 的硬承诺）：`course/**` 与 `courses/set-theory/**` **一个字节都不改**。
   依据：附 B.3 的两条 grep（符号只在注释里、`infix|notation` 作为标识符 0 命中）。
3. **既有两个课程的 golden 是否变：都不变。**
   - `course/`（第一门课）：`crates/cli/tests/course.rs:86-98` 的 11 元 GOLDEN 与
     `crates/cli/tests/course_status.rs:68-80` 的 11 元 GOLDEN —— **不变**。
     依据：本 WO 不改课程文件；记法命令不产生任何事件（N6）；该课的 11 个画布里
     `∈⊆∅∪∩` 与 `infix`/`notation` 均 0 命中（附 B.3）。
   - `courses/set-theory/`（第二门课）：门禁是 `courses/set-theory/tools/check.py:31-47`
     （只按 `grade` 退出码数 `decl.checked`/`exercise.open`，**没有计数金值**）——**不变**。
   - `playground.sokonanoda` anchor（`crates/cli/src/env/mod.rs:285-303`）：无符号、无 `infix` —— **不变**。
4. **同轮必须一起改的文件**（缺一条不算完成）：`crates/front/src/{token,parser,ast,semantic}.rs`、
   `crates/front/src/compile/{elab,error,check/walk,check/mod,warning,goals}.rs`、
   `docs/architecture.md` §4.1、`docs/protocol.md`（新诊断码）、新设计文档
   `docs/design/notation-subset.md`（设计先行：仓库硬规则，`AGENTS.md` 收尾义务）、
   `docs/TESTING.md` 守护行、`editor/vscode/syntaxes/sokonanoda.tmLanguage.json`（见文档同步）、
   `STATUS.md`、`docs/HANDOVER.md`、`docs/gaps/ledger.jsonl` 的 G-04 行。
5. **课程侧的后续（不在本 WO，交给课程线）**：课程文件里那些「语言还没有 notation（台账 G-04）」
   的注释会变陈旧——实测命中 `courses/set-theory/lib/{Set:13,Fun:18,Rel:19,Exists:53,Equiv:85}.sokonanoda`
   与 `units/{unit01:15-17,unit03:24,unit05:21}`（另有大量**纯教学**的符号注释，如
   `units/unit02-subsets-empty.sokonanoda:4`、`:43`，它们不声称「语言没有记法」，不算陈旧）。
   这些都是**注释**，不影响判卷；
   按 `syllabus §4` 的「先点名、后记法」计划，等课程线那一轮统一改成「两种写法对照页」
   （本 WO 只在这里记账，不代改）。

## 验收（三层）

### 1) front 单测

- **词法**（`crates/front/src/token.rs:337-465` 的 `mod tests`）：
  - `infix:50 " ∈ " => mem` → `Ident("infix"), Colon, Num("50"), Str(" ∈ "), FatArrow, Ident("mem"), Eof`；
  - `x∈A` → **三个** token `Ident("x"), Sym("∈"), Ident("A")`（今天是一个 Ident，实测 P2）；
  - **回归**：`α'1` 仍是一个 `Ident`（`token.rs:421-426` 现有测试必须原样绿）、
    `forall ∀ foralls forall!`（`:372-379`）、`id.{u}`（`:398-412`）全部不变；
  - 未闭合字符串 `infix:50 " ∈ => mem` → 专用诊断且 span 指向开引号的行列；
  - 未声明符号 `a ∈ A`（文件里没有 `infix` 行）→ 专用诊断（**不是**今天的
    `unknown identifier`），hint 里给「先声明」与「点名写法」两条出路。
- **语法/优先级**（`crates/front/src/parser.rs` 的 `mod tests`，`:1315` 起）：
  - 四个命令各自成 AST；缺优先级、缺 `=>`、符号是标识符词（`"in"`）、符号为空串、
    重复声明同一符号 → 各自专用诊断；
  - `A ⊆ B -> C` 解析成 `(A ⊆ B) -> C`（50 紧于 `->`）；
  - `a ∈ A ∪ B` 在 `∪`=65、`∈`=50 时解析成 `a ∈ (A ∪ B)`（用一个临时声明 `infixl:65 " ∪ " => …` 造场景）；
  - `infix` 无结合：`a ∈ b ∈ c` 报解析错；`infixl` 左结合；
  - **回归**：`1 + 1 + 1` 的 AST 与今天逐字节相同（仍 `Expr::Plus`），
    `+` 与 `->`/应用的相对优先级不变。
- **elab**（`crates/front/src/compile/tests.rs`）：
  - **等价性**：同一批声明写两遍（点名版 / 记法版），`events` 的类型与计数一致、`errors` 皆空；
  - **护城河**：点名 `Set.mem a A`（省 `α`）**仍**被拒绝（`kernel-rejected`，与今天同码同 stage）；
  - `notation "∅" => Set.empty`：在期望类型已知处成功；在 `#check ∅`（无期望）→ `elab-notation-argument-unsolved`；
  - 目标不存在 → 专用诊断；记法出现在其声明**之前** → 未声明符号诊断。
- **semantic**（`crates/front/src/semantic.rs:958-1000` 一带的锁表测试**不动**）：声明的 `∈` 归
  `Keyword`；未声明的 `∈` 不产 run；`SemanticKind::ALL` / `tm_scope` 表**逐字不变**（v1 不新增 kind）。

### 2) CLI e2e

- `crates/cli/tests/`（新文件 `notation.rs` 或并入 `cli.rs`）：**同一命题两种写法**各一个文件，
  跑 `--json`，断言五元组 `(decl.checked, exercise.open, expr.reduced, expr.typed, diagnostic)`
  **逐一相等**且 `exit 0`；断言事件种类集合**没有新增**（`docs/protocol.md:48-92` 的清单不变、
  `soko.query/1` schema 不变）。
- `crates/cli/tests/protocol.rs`：新诊断码进入分级表（`stage=parse`/`elab`），
  `ok:false` 与退出码语义不变。
- 课程门禁复跑：`python3 courses/set-theory/tools/check.py` 必须**仍然全绿**
  （12 单元 + lib + 解答；判据是 `grade` 退出码，`tools/check.py:31-47`）。
- `scripts/soko gate`（fmt 三个教学 crate + clippy workspace + test workspace + playground anchor）。

### 3) 课程用例（真文件 + 真练习名）

- 靶子文件：`courses/set-theory/units/unit02-subsets-empty.sokonanoda`
  - **练习 2 `subset_trans`**（声明在 `:28-30`）：`(h1 : Set.subset α A B) (h2 : Set.subset α B C) :
    Set.subset α A C` → 记法版 `(h1 : A ⊆ B) (h2 : B ⊆ C) : A ⊆ C`（对应解答
    `courses/set-theory/units/solutions/unit02-solution.sokonanoda` 的同名 `theorem`）；
  - **练习 4 `empty_subset`**（`:55`）：`Set.subset α (Set.empty α) A` → `∅ ⊆ A`。
  - 第二刀的靶子（本轮**只记录、不验收**）：同课 `unit03-union-inter-powerset.sokonanoda` 的
    `subset_union_left`（`:61`，要 `∪`）与 `powerset_mono`（`:124`，要 `𝒫`）。
- 做法（沿用 WO-010 的纪律，**答案不许入库**）：把该单元连同 `courses/set-theory/lib/`、
    `sokonanoda.toml` 复制到 `/tmp` 的临时项目，在**副本**里加记法行、把上面两条声明改成记法版，
    用绝对路径判卷；断言 `grade` exit 0 且事件计数与原点名版**逐项相同**。
- **本 WO 的课程零改动**由 `python3 courses/set-theory/tools/check.py` 全绿 + `git status` 里
  `courses/` 无改动共同守护。

### 影响面（事件计数 / golden）

- 事件种类与计数**不变**（记法命令不是声明，N6）⇒ **双 GOLDEN 不改**：
  `crates/cli/tests/course.rs:86-98`、`crates/cli/tests/course_status.rs:68-80`。
- 唯一 golden 面 = 新增测试自身（新文件/新断言），以及**设计文档里写死的优先级表**。
- 版本：新能力 = minor（仓库惯例，`docs/RELEASE.md`）⇒ `Cargo.toml:6` 与
  `editor/vscode/package.json:5` **两处**同轮 bump（建议 `0.58.0 → 0.59.0`）。

## 文档同步清单

- **设计先行（先写、后改代码）**：`docs/design/notation-subset.md`（新）——记录 N1–N7、
  优先级梯子表、N4 的补全算法与它的边界、文件内作用域的理由（附 B.1）、以及「与 Lean 的已知差异」
  （待确认 1/2/3）。仓库硬规则：新功能先落设计（`AGENTS.md` 收尾义务、`skills/sokonanoda-dev`）。
- `docs/architecture.md:97-148`（§4.1 语法白名单，**权威**）：命令清单 + 记法边界；
  `:386` 的第 4 条边界说明可加一句「记法是糖、点名形式永久可用」。
- `docs/protocol.md`：`:93-160` 错误分级与错误码清单**加新码**（`parse-notation-*`、
  `elab-notation-unknown-target`、`elab-notation-argument-unsolved`）；
  `:48-92` 机器事件**不动**；`:623-635` agent 契约**不动**。
- `docs/TESTING.md`：分层守护表加一行「记法」：词法切分 / 优先级 / 两种写法计数一致 /
  护城河（点名省 `α` 仍被拒）/ 课程门禁全绿。`:96` 的收尾清单提到「更新 §4 白名单」，本 WO 同款。
- `skills/`：三个技能（`sokonanoda-{teacher,dev,ci}`）里凡列「语法白名单/命令表」的地方同轮补记法
  （`crates/cli/tests/skill.rs` 挡漂移）；`.agents/skills/<name>/SKILL.md` 是薄入口，
  正文仍在 `skills/`，改完确认入口不漂（`crates/cli/tests/dsh.rs`）。
- `editor/vscode/`（**用户可见语法 ⇒ 必须同轮**）：`syntaxes/sokonanoda.tmLanguage.json`
  加一条**通用数学符号**规则（scope 用已有的 `keyword.operator.sokonanoda`，**不要**动
  `keywords`/`commands`/`sorts` 三个词表——`crates/cli/tests/extension.rs:1055-1093` 把它们
  与 `front::semantic` 逐字锁死，`:1028-1053` 又要求每个 `tm_scope` 都有规则）；
  `README.md`/`CHANGELOG.md`/`package.json`（版本 + 说明），手册 `docs/vscode-dev-guide.md`。
- `AGENTS.md`（仓库根）：白名单规则处加一句指向 `docs/design/notation-subset.md`；
  `.agents/skills/` 入口若因此改动按上面确认。
- `docs/HANDOVER.md`、`STATUS.md`（只留最近 3 轮，旧轮进 `docs/STATUS-ARCHIVE.md`）。
- `docs/design/teaching-project.md`：§6.2 台账写回状态、§8 的 P1 进度（G-04 从 blocker 清零名单里划掉）。
- `REQUIREMENTS.md` §9：本 WO 不是用户新要求 ⇒ **不追加**（若用户借此把「记法子集」升成要求，再补一条带日期）。
- `docs/gaps/README.md:37-38` 的复现判据说明：若按「关账」一节的建议把 `gap.py` 的守卫扩展到
  `.sokonanoda`，同轮改这里的措辞。
- `docs/gaps/ledger.jsonl` G-04 行（见下）。

## 门禁

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
scripts/soko gate                                     # fmt(3 crate) + clippy + test + playground anchor
python3 courses/set-theory/tools/check.py             # 课程门禁（判据 = grade 退出码）
python3 scripts/gap.py check                          # 台账一致性（修好+关账后才绿）
scripts/soko grade "$PWD/docs/gaps/repro/G04-notation.sokonanoda"  # 修好后：干净判卷 + 有 decl.checked
```

> `gate` 的 anchor 用**运行中二进制**的内嵌编译器；与仓库版本不一致会直接 exit 3
> （`crates/cli/src/env/mod.rs:239-250` 的注释与实现）——先 `scripts/soko update`。
> **禁止** `cargo fmt --all`（会重排冻结内核）：只 fmt 教学 crates，或直接 `scripts/soko gate`。

## 关账

- 修好后（并已把复现件扩成**真的用一次记法**，见下）：

  ```bash
  python3 scripts/gap.py close G-04 --version 0.59.0
  ```

- **复现件同轮要动**：`docs/gaps/repro/G04-notation.sokonanoda` 现在只声明了 `infix:50 " ∈ " => mem`；
  建议补一条**使用行**（例如 `def use (α : Type) (a : α) (A : α -> Prop) : Prop := a ∈ A`），
  这样 `gap.py` 的判据（干净判卷 + 有 `decl.checked`，`scripts/gap.py:59-78`）才真的证明糖生效。
- **别被守卫骗了（实读源码）**：`scripts/gap.py:198-207` 的 `close` **只对 `kind == "script"` 拒绝**
  （`exit 0` = 缺口仍在时拒绝关账）；`.sokonanoda` 复现走的是 `:64-78` 的干净判卷分支，
  **没有拒绝分支**。所以 G-04 的「真的修好了」必须靠 ① 复现件补使用行 + ② `gap.py check`
  的状态翻转（`:166-195`：`status != fixed` 时期望「仍有失败」，翻成 `fixed` 后才期望「已判卷通过」）
  两条一起看。是否顺手把守卫扩展到 `.sokonanoda`，记在「待确认 8」。
- **手改 `docs/gaps/ledger.jsonl` G-04 行**（`gap.py close` 只写 `status`/`fixed_in`/`note`）：
  - `wo` → `docs/gaps/WO-011-notation.md`、`status` → `wo-filed`（**本 WO 落地即可做**）；
  - 关账时重写 `today`（把「parse unexpected-token（infix:50 " ∈ " => mem）」换成实测的三层现象：
    字符串 token 缺失 / 符号是标识符字符 / 类型参数无人补）与 `expected_lean`（写成「`infix:N`
    与零元 `notation` 可用；点名形式继续可用；两种写法判卷一致」，即本 WO 的 N1–N7）；
  - `blocks` 里的两条（卷 I 单元 2/6、整本书的阅读体验）在关账时复核：第一刀只覆盖
    `∈`/`⊆`/`∅`，单元 6 的 `''`/`⁻¹'` 仍要等第二刀——**不要**把 `blocks` 一次清空。
- 收尾义务：`STATUS.md` 一轮；`docs/HANDOVER.md` 同步；`courses/set-theory/AGENTS.md` 的
  判卷纪律**不改**（本 WO 没有推翻它）。

## 待确认（不许在实现里当事实用）

1. **`infix`/`infixl`/`infixr` 的精确脱糖**（左右两侧的优先级是 `p`/`p`、`p`/`p+1`、`p+1`/`p`）
   ——本轮无网络（`curl`/`web_fetch` 全部超时），**未从 Lean 源码取证**；仓库里逐字取证的只有
   `infix:50 " ⊆ "`、`infixl:65 " ∪ "`、`infixr:80 " '' "` 三条**声明行**（prior-art §1.3）。
   设计文档必须先写死 v1 的三条结合规则，再实现。
2. **`+` 在 Lean core 的优先级**（记忆中是 `infixl:65 " + "`，未取证）。v1 的实现不依赖这个数
   （`+` 行为逐字节不变），但它决定 `a ∈ A ∪ B` 这类混写的分组，写进设计文档时标来源。
3. **Lean 是否接受无空格的 `x∈A`**：本 WO 选的**符号码点类**切分会让它可用（比「要求空格」更强），
   但「Lean 里也一样」这句话**没有取证**；设计文档要么补上游取证，要么写成「我们更宽松」的已知差异。
4. **`𝒫`（U+1D4AB）与 `ᶜ`（U+1D9C）是 Unicode 字母**（数学斜体/修饰字母），不是符号——
   第二刀不能用符号类处理，需要声明驱动的词法或专门 token。本轮只记录，不做。
5. **重复声明同一符号**时 Lean 的报错文本与语义 —— 未取证；v1 自定一条专用诊断（建议 error）。
6. **`notation` 与 `infix` 的优先级缺省值**（core 的 `∅` 行没有 `:max`）——
   v1 按「零元记法不写优先级」处理；`notation:N` 的接受与否留给设计文档定。
7. **本地未登记的相关缺口**（本 WO 只记录，不开 WO）：隐式 binder 的**应用省略**不成立
   （P7：`mem2 x s` 被拒、`mem2 T x s` 通过）、`_` 占位符不存在（P8）。
   它们是 N4「为什么要自己补参数」的根因；将来若做「一般隐式实参」，本 WO 的补全路径应能被替换掉。
8. **`scripts/gap.py` 的关账守卫只覆盖 `.sh` 复现**（`:198-207`）：要不要扩展到 `.sokonanoda`
   （把「干净判卷」当作「已修好」来拒绝关账）——本 WO 不做，避免与台账工具同轮混改；
   建议在设计文档或下一轮台账轮里定。
9. **编译缓存命中与 parse 的顺序**：`cache.rs:83-98` 键是源文本（附 B.2），但
   `crates/front/src/session.rs` 侧「先 parse 再查缓存还是反过来」本轮**没有逐行确认**；
   若实现时把记法表缓存进 `CachedCompile`，必须重新论证键的充分性（源文本里已经含记法行，
   预期仍充分，但要写进设计文档）。

---

## as-built（2026-09-19 落地，第一百〇五轮）

**范围**：第一刀全部落地（Lean core 级 `∈`/`⊆`/`∅` + 通用四条命令）；第二刀未做。
**内核零改动**；**课程零改动**（`courses/` 无 diff，门禁 315 checked · 96 open · 0 判负）。
详细 as-built 见 `docs/design/notation-subset.md` §9（八条实现时才暴露的事实），
这里只记与本 WO 正文/待确认项的对应：

| 本 WO 的预测/待确认 | 实测 |
|---|---|
| 三条 key findings（无字符串 token / 符号是标识符字符 / 无人补 `Type` 参数） | **三条全部证实**；分别由 `TokenKind::Str`、`TokenKind::Sym` + `is_ident_start` 收窄、`notation_prefix_args` 解决 |
| 待确认 1（`infix` 的左右优先级） | 按设计写死 `p`/`p+1`、`p+1`/`p`（左结合 `infixl` 同级左结合），**仍未取证**；`+` 行为逐字节不变 |
| 待确认 3（`x∈A` 无空格可用） | **可用**（符号码点类切分的结果）；设计 §6 记为"我们更宽松"，未取证 |
| 待确认 5（重复符号） | v1 自定一条 error（`notation-shape`），Lean 的重载语义未取证 |
| 待确认 6（零元记法的优先级） | v1 不接受 `notation:N`（报 `notation-shape`），零元走 `starts_atom` 的 `Sym` 分支 |
| 待确认 8（`gap.py` 守卫只覆盖 `.sh`） | **未动**（按 WO 要求不与台账工具同轮混改） |
| 待确认 9（缓存键充分性） | 记法表**没有**进 `CachedCompile`（展开在 elab 内、每次编译都重跑 parse），源文本键仍充分 |
| 「关账」一节要求的 `today`/`expected_lean`/`blocks` 重写 | 已按实测重写；`blocks` **未清空**（单元 6 的 `''`/`⁻¹'` 仍等第二刀） |
| 「复现件同轮要动」 | 已补使用行 `def use (α : Type) (a : α) (A : α -> Prop) : Prop := a ∈ A`；`gap.py check` 里 G-04 = 「已判卷通过」 |

**额外发现（WO 正文没预见）**：
1. **嵌套零元记法 `∅ ⊆ A` 需要"操作数也吃期望类型"**——第一版只给整条记法一个
   期望类型，`∅` 解不出 `α`；补上 `notation_operand_expected` 后课程用例才绿。
2. **展开必须直接构造内核项**，不能把补好参数的源级 `App` 交回 `elab_expr`
   （操作数会被 elaborate 两次，且在 `Expr`/`ExprPtr` 之间搬类型）。
3. **`elab-notation-unknown-target` 只在目标名写错时触发**（在**使用点**解析），
   所以它的回归测试必须"声明拼错的目标 + 真的用一次"。
4. **既有夹具里那条 `infix:50 " e " => mem`** 从"随便写的坏文本"变成了有意义的
   `notation-shape` 诊断——三处 `query`/`protocol` 夹具改成真正的语法错
   `def p : Prop := (a`，偏移量 15/21 → 19/19。**判据没放宽**（改后仍必须
   parse 报错 + exit 1）。
