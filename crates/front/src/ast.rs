//! `.sokonanoda` 抽象语法树：排序、表达式、binder、命令与文件结构。

use crate::span::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortKind {
    Prop,
    Type,
    Sort(u64),
    Level(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Sort {
        sort: SortKind,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    UniverseApp {
        name: String,
        levels: Vec<String>,
        span: Span,
    },
    Num {
        value: String,
        span: Span,
    },
    Hole {
        span: Span,
    },
    App {
        fun: Box<Expr>,
        arg: Box<Expr>,
        /// **Lean 的 `@` 标记**（IA-1）：整条应用脊的实参是**逐位显式**的
        /// ⇒ 前端**不插**隐式实参（`@f a b` 把 `a` 落在第一个形参位上，
        /// 哪怕它是隐式的）。parser 在 `@` 之后建出的每个 App 节点都带它。
        explicit_spine: bool,
        span: Span,
    },
    Lambda {
        binders: Vec<Binder>,
        body: Box<Expr>,
        span: Span,
    },
    Forall {
        binders: Vec<Binder>,
        body: Box<Expr>,
        span: Span,
    },
    Arrow {
        domain: Box<Expr>,
        codomain: Box<Expr>,
        span: Span,
    },
    Plus {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Phase 1：`let x : T := val; body`（局部、有名字的中间值）。
    /// binder 复用 [`Binder`]；Phase 1 要求 `ty == Some`（缺注解在 elab 报
    /// `elab-untyped-binder`）。`binder.span` 收窄到名字+注解，供 hover /
    /// go-to-definition。
    Let {
        binder: Binder,
        val: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    /// `:= by <tactic 序列>`：值位是一段 tactic 脚本，由编译期引擎翻译成
    /// 普通表达式（可能带尾部 `sorry`）。
    By {
        tactics: Vec<Tactic>,
        span: Span,
    },
    /// `match <scrutinee> with | <pattern> [if <guard>] => <body> | ...`：对
    /// **源内/预置**归纳类型做模式匹配（支持通配 `_`、绑定变量、嵌套构造子、
    /// Nat 字面量、`Bool` 守卫）。降低为 `<Ind>.rec.{level} motive minor… scrutinee`；
    /// 判定仍由完整内核终审。设计 `docs/design/match-patterns.md`。
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// 记法使用点（G-04 / WO-011）：`lhs sym rhs` 或零元的 `sym`。
    ///
    /// parser 只记下**写下来的东西**（符号 + 目标名 + 操作数）；elaborator 把它
    /// 源到源降级成 `App` 形状（设计 `docs/design/notation-subset.md` §2 N4）。
    /// 操作数顺序**保持**（与 Lean `infix` 糖一致）；零元记法的 `lhs`/`rhs`
    /// 都是 `None`。
    Notation {
        symbol: String,
        target: String,
        assoc: NotationAssoc,
        lhs: Option<Box<Expr>>,
        rhs: Option<Box<Expr>>,
        /// **记法重载**（第三刀 §12.2）：同一符号的其它候选目标（声明顺序）。
        /// 单候选时为空（绝大多数程序），此时展开路径逐字节等于第二刀。
        alternatives: Vec<String>,
        span: Span,
    },
    /// **集合字面量**（第三刀 §12.4）：`{a}` / `{a, b}`。
    ///
    /// 新语法（不是记法）：展开成点名形式 `Set.singleton α a` /
    /// `Set.pair α a b`（与 `+` → `Nat.add` 同族的**内建糖**）。
    /// `{}` 今天是 binder / 宇宙参数定界符，消歧在 `parse_atom` 的 lookahead
    /// 里（`{x : T}` 形状不是字面量）。
    SetLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    /// **匿名构造子**（课程 Lean 化 L2.7）：`⟨a, b⟩`。
    ///
    /// 新语法（不是记法）：用哪个构造子由**期望类型**决定（路线 C，不引入
    /// 元变量）——`A ∧ B` ⇒ `And.intro`、`∃ (x : α), p x` ⇒ `Exists.intro`、
    /// `A ↔ B` ⇒ `Iff.intro`、`Prod α β` ⇒ `Prod.mk`、单构造子归纳 ⇒ 它的
    /// 构造子。展开复用**记法路径**（前导参数补全 + 操作数期望类型传播），
    /// 所以 `⟨w, hw⟩` 在 `∃ (x : α), p x` 上解得出 `α` 与 `p`。
    AnonCtor {
        elements: Vec<Expr>,
        span: Span,
    },
}

/// 记法命令的结合性 / 元数（`docs/design/notation-subset.md` N1 + §10.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotationAssoc {
    /// `infix:N`：无结合，两边同级。
    Infix,
    /// `infixl:N`：左结合。
    Infixl,
    /// `infixr:N`：右结合。
    Infixr,
    /// `prefix:N`：一元前缀（第二刀）——`sym a`，操作数在 `rhs`。
    Prefix,
    /// `postfix:N`：一元后缀（第二刀）——`a sym`，操作数在 `lhs`。
    Postfix,
    /// `notation "∅" => …`：零元常量记法（无优先级）。
    Nullary,
    /// `notation-binder "∃" => Exists`（第三刀）：**binder 位置**的记法——
    /// `∃ x, p` 展开成 `Exists A (fun (x : A) => p)`；两段式 `∃ x ∈ s, p`
    /// 展开成 `Exists A (fun (x : A) => And (x ∈ s) p)`。没有优先级。
    ///
    /// 与 `Prefix` 的区别只在**出现位置**与**渲染**：展开路径完全一样
    /// （一个操作数 = 一个 lambda），见 `docs/design/notation-subset.md` §12.1。
    Binder,
}

impl NotationAssoc {
    /// 二元算子（爬升梯子上的那三种）。
    pub fn is_binary(self) -> bool {
        matches!(
            self,
            NotationAssoc::Infix | NotationAssoc::Infixl | NotationAssoc::Infixr
        )
    }
}

/// 一条记法声明（`Command::Notation` 的**载荷**，不含 span）。
///
/// 用途：跨 `import` 的记法传播（第二刀，设计 §10.3）——闭包按拓扑序把前面
/// 模块声明的记法作为**继承表**喂给后面的模块，所以它必须是可复制的纯数据。
// `Serialize`/`Deserialize`：`ProjectReport` 要过项目缓存（T-D11）——记法表是
// 报告的一部分，缓存命中时它必须一起回来，否则"跳转"在热路径上会突然失效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotationDecl {
    pub symbol: String,
    /// 零元记法为 `None`。
    pub precedence: Option<u16>,
    pub assoc: NotationAssoc,
    pub target: String,
    /// **`scoped` 记法的作用域名**（第三刀 §12.3）：`Some("Foo")` ⇒ 这条记法
    /// 默认**不生效**，要 `open scoped Foo` 才生效；`None` ⇒ 一直生效（第二刀
    /// 行为）。作用域名 = 声明点所在 `namespace` 的累积全前缀。
    pub scope: Option<String>,
    /// **声明点**（T-D10）：`infix:50 " ∈ " => Set.mem` **那一行**的 span
    /// （在**声明它的模块**的坐标里）。
    ///
    /// 为什么要有：线 D 的"记法跳转"要跳到声明处——而记法是**跨 `import`
    /// 传播**的（入口里写 `∈`，声明在 `lib/Set.sokonanoda`），所以只有
    /// `target` 这个名字不够，还得知道**在哪个模块的哪一段**。
    pub span: Span,
    /// **声明它的模块名**（拓扑序里的模块名，如 `lib.Set`）。
    ///
    /// `None` = 语言内建的记法（`↔`/`∧`/`=`…）：它们**不在任何源文本里**
    /// （parser 有一张硬编码表），没有可跳的声明点。
    pub module: Option<String>,
}

/// `open` / `export` 的**过滤与改名子句**（第二刀，设计
/// `docs/design/namespace-open.md` §N7）。三条子句互斥，最多出现一条。
///
/// 语义（`compile/scope.rs` 是唯一实现）：短名 = 声明名最后一段。
/// **先过滤、后改名**——`only`/`hiding` 决定哪些声明短名能进来，`renaming`
/// 再把进来的那些换个可见名（原短名随之失效）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenFilter {
    /// `open Foo (a b)`：只让这些短名进来；`None` = 全部。
    pub only: Option<Vec<String>>,
    /// `open Foo hiding a b`：把这些短名挡在外面。
    pub hiding: Vec<String>,
    /// `open Foo renaming a => b`：`from`（声明短名）在引用位置写成 `to`。
    pub renaming: Vec<(String, String)>,
}

impl OpenFilter {
    /// 没有子句（`open Foo` / `export Foo`）。
    pub fn is_empty(&self) -> bool {
        self.only.is_none() && self.hiding.is_empty() && self.renaming.is_empty()
    }

    /// 声明短名 `decl` 在这条子句下的**可见短名**；`None` = 被过滤掉。
    ///
    /// `renaming` 的目标名也占位：`renaming a => b` 之后，名字 `b` 指的是
    /// `Foo.a`，`Foo.b`（若真有）不再作为 `b` 的候选（一条 open 对一个短名
    /// 只给一个候选——与「第一个命中即止」的解析顺序同款，见设计 §N7）。
    pub fn visible_short(&self, decl: &str) -> Option<String> {
        if let Some((_, to)) = self.renaming.iter().find(|(from, _)| from == decl) {
            return Some(to.clone());
        }
        if self.renaming.iter().any(|(_, to)| to == decl) {
            return None;
        }
        match &self.only {
            Some(only) => only.contains(&decl.to_string()).then(|| decl.to_string()),
            None => (!self.hiding.iter().any(|hidden| hidden == decl)).then(|| decl.to_string()),
        }
    }

    /// 人话写法（诊断/合成前缀用）：`""` / `" (a b)"` / `" hiding a b"` /
    /// `" renaming a => b"`。
    pub fn source_text(&self) -> String {
        let names = |names: &[String]| names.join(" ");
        if let Some(only) = &self.only {
            return format!(" ({})", names(only));
        }
        if !self.hiding.is_empty() {
            return format!(" hiding {}", names(&self.hiding));
        }
        if !self.renaming.is_empty() {
            let pairs = self
                .renaming
                .iter()
                .map(|(from, to)| format!("{from} => {to}"))
                .collect::<Vec<_>>()
                .join(", ");
            return format!(" renaming {pairs}");
        }
        String::new()
    }
}

/// `match` 的一条分支：`| <pattern> [if <guard>] => <body>`
/// （模式编译器设计 `docs/design/match-patterns.md`）。
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    /// `if <cond>`：`cond : Bool`，为假时落到后续 arm（v1，见设计 §4）。
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// 一个 `match` 模式（递归）。
///
/// parser 只产出 [`Pattern::Ident`]/[`Pattern::Wild`]/[`Pattern::Num`]；
/// `Ident` 是**构造子还是绑定变量由 elaborator 按该位置的归纳类型判定**
/// （parser 无类型信息）。编译器生成的 canonical 模式也是 `Ident`，只是
/// 名字一定是当前类型的构造子名（幂等，见设计 §4）。
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `_`：通配，不绑定。
    Wild { span: Span },
    /// `0`/`1`/…：Nat 字面量（仅被匹配类型为 Nat 形状时合法）。
    Num { value: String, span: Span },
    /// 构造子（`zero`/`Nat.succ`，可带子模式）或绑定变量（无子模式）。
    Ident {
        name: String,
        args: Vec<Pattern>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wild { span } | Pattern::Num { span, .. } | Pattern::Ident { span, .. } => {
                *span
            }
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Sort { span, .. }
            | Expr::Ident { span, .. }
            | Expr::UniverseApp { span, .. }
            | Expr::Num { span, .. }
            | Expr::Hole { span }
            | Expr::App { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Forall { span, .. }
            | Expr::Arrow { span, .. }
            | Expr::Plus { span, .. }
            | Expr::Let { span, .. }
            | Expr::By { span, .. }
            | Expr::Match { span, .. }
            | Expr::Notation { span, .. }
            | Expr::SetLiteral { span, .. }
            | Expr::AnonCtor { span, .. } => *span,
        }
    }
}

/// 教学白名单里的一个 tactic（`by` 块内）。
#[derive(Debug, Clone, PartialEq)]
pub enum Tactic {
    /// `intro a b c`：一次剥掉多层 Pi/Arrow。**多名字**是 Lean 的常态写法
    /// （课程 Lean 化，设计 `docs/design/course-lean-style.md` L1.3）；
    /// 每个名字对应一层，语义等价于连续写多个 `intro`。
    Intro {
        names: Vec<String>,
        span: Span,
    },
    Exact {
        expr: Expr,
        span: Span,
    },
    Apply {
        expr: Expr,
        span: Span,
    },
    Assumption {
        span: Span,
    },
    Rfl {
        span: Span,
    },
    /// `constructor`：按**目标头**选构造子（取**第一个**，Lean 语义），
    /// 等价于 `apply <Ind>.<第一个构造子>`。构造子表来自前端自己的归纳表
    /// （`InductiveTable`），判定仍走 kernel（合成声明）。
    Constructor {
        span: Span,
    },
    /// `left` / `right`：目标头的归纳有 ≥2 个构造子时取第 1 / 第 2 个
    /// （`Or` 上是 `Or.inl` / `Or.inr`）。
    Left {
        span: Span,
    },
    Right {
        span: Span,
    },
    /// `use w`：目标头是**单构造子**归纳（`Exists`）时交证人——等价于
    /// `apply <ctor>` 之后立刻 `exact w`（第一个子目标就是证人位）。
    Use {
        expr: Expr,
        span: Span,
    },
    /// `exfalso`：把当前目标换成 `False`（原目标记在组装里），
    /// 等价于 `apply False.elim` 但读起来是 Lean 的样子。
    Exfalso {
        span: Span,
    },
    /// `cases h` / `cases h with | ctor a b => <tactics> | …`：对假设做情形分析。
    ///
    /// **降低成 `match`**：每个臂的 tactic 序列各自组装成一个项，整个 `cases`
    /// 变成一个 `Expr::Match { scrutinee: h, arms }` —— 递归子与 iota 规则由
    /// **既有的 `match` 降低路径**处理（`compile/elab.rs`），引擎不手搓 recursor。
    /// 判定仍走内核。
    ///
    /// `arms` 为空 = 不带 `with` 的写法：按**构造子声明顺序**造子目标，
    /// 分支假设用构造子自己的字段名（`ctor Or.inl (a : A)` ⇒ `a`）。
    Cases {
        expr: Expr,
        arms: Vec<CasesArm>,
        span: Span,
    },
    /// `have h : T := t` / `have h : T := by <tactics>`（设计
    /// `docs/design/course-lean-style.md` L3.6）：在当前上下文里**引入一条
    /// 中间结论**，目标不变。
    ///
    /// **降低成 let 的应用形态**：`(fun (h : T) => <rest>) t`。所以引擎只需
    /// 把 `h : T` 当成普通假设加进上下文（沿父链的 `NodeKind::Have`），组装时
    /// 包一层应用——`t : T` 由内核在应用处再判一次（tactic 步里**已经**先判过
    /// 一次，为的是把错误报在 `have` 那一行而不是整个证明上）。
    Have {
        name: String,
        ty: Expr,
        value: HaveValue,
        span: Span,
    },
    /// `sorry`：占位——当前目标保持开放（合法 Open 状态），
    /// 与声明值位的 `sorry` 同语义（未完成证明）。
    Sorry {
        span: Span,
    },
}

/// `have` 的值位：项（`:= t`）或嵌套 tactic 块（`:= by …`）。
#[derive(Debug, Clone, PartialEq)]
pub enum HaveValue {
    Term(Expr),
    /// 嵌套 tactic 序列。**用缩进界定**（与 `cases` 臂体同一条规则）：
    /// 第一个列号 ≤ `have` 所在列的 tactic 属于**外层**块。
    By(Vec<Tactic>),
}

/// `cases` 的一个分支：`| <ctor> <binder>… => <tactics>`。
#[derive(Debug, Clone, PartialEq)]
pub struct CasesArm {
    /// 构造子名，**按用户写的拼写**（裸名 `inl` 或点号名 `Or.inl` 都行）；
    /// 引擎按归纳表的源名归一。
    pub ctor: String,
    /// 分支假设的名字（用户给的，按字段顺序）。
    pub binders: Vec<String>,
    /// 臂体的 tactic 序列。
    pub tactics: Vec<Tactic>,
    pub span: Span,
}

impl Tactic {
    pub fn span(&self) -> Span {
        match self {
            Tactic::Intro { span, .. }
            | Tactic::Exact { span, .. }
            | Tactic::Apply { span, .. }
            | Tactic::Assumption { span }
            | Tactic::Rfl { span }
            | Tactic::Constructor { span }
            | Tactic::Left { span }
            | Tactic::Right { span }
            | Tactic::Use { span, .. }
            | Tactic::Exfalso { span }
            | Tactic::Cases { span, .. }
            | Tactic::Have { span, .. }
            | Tactic::Sorry { span } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinderKind {
    Explicit,
    Implicit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binder {
    pub name: String,
    /// `None` means the binder has no explicit type (elaboration infers it).
    pub ty: Option<Box<Expr>>,
    pub style: BinderKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `import Foo.Bar`（文件级命令，必须出现在所有声明之前）。
    /// `module` 存**原始**点分名字；合法性由 `project::ModuleName` 判定。
    Import {
        module: String,
        span: Span,
    },
    Def {
        name: String,
        universe: Vec<String>,
        ty: Expr,
        val: Expr,
        span: Span,
    },
    Theorem {
        name: String,
        universe: Vec<String>,
        ty: Expr,
        val: Expr,
        span: Span,
    },
    Example {
        ty: Expr,
        val: Expr,
        span: Span,
    },
    Axiom {
        name: String,
        universe: Vec<String>,
        ty: Expr,
        span: Span,
    },
    InductiveBlock {
        name: String,
        /// Non-indexed parameters (`inductive Option (A : Type) : Type`).
        /// Empty for `num_params = 0` blocks such as the prelude `Nat`.
        params: Vec<Binder>,
        ty: Expr,
        constructors: Vec<CtorDecl>,
        recursor: Option<RecDecl>,
        iota_rules: Vec<IotaRule>,
        span: Span,
    },
    Check {
        expr: Expr,
        span: Span,
    },
    Reduce {
        expr: Expr,
        span: Span,
    },
    Print {
        name: String,
        span: Span,
    },
    /// `infix:N " sym " => name` / `infixl` / `infixr` / `prefix` / `postfix` /
    /// `notation " sym " => name`（G-04 / WO-011 第一刀 + 第二刀）。
    ///
    /// **不是声明**（N6）：不产生 `decl.checked`/`exercise.open`/`expr.typed`，
    /// 也不进声明表——与 `Command::Import` 同族（有命令、无声明）。
    /// `precedence` 只有零元记法为 `None`。
    Notation {
        symbol: String,
        precedence: Option<u16>,
        assoc: NotationAssoc,
        target: String,
        /// `scoped` 记法的作用域名（第三刀 §12.3）；`None` ⇒ 一直生效。
        scope: Option<String>,
        span: Span,
    },
    /// `namespace <name>`（G-05，设计 `docs/design/namespace-open.md`）。
    ///
    /// **不是声明**（N6）：不产生事件、不进声明表——与 `Command::Import`
    /// 同族。`name` 是**写出来的**名字（可点分）；累积全前缀由 parser 与
    /// elab 共用 `compile::scope::join_ns` 现算（两者逐字一致）。
    /// 本命令的副作用是**解析期**的：`end` 之前声明的名字自动带前缀（N3）。
    Namespace {
        name: String,
        span: Span,
    },
    /// `end <name>`（G-05）：闭合最近的 `namespace <name>`。parser 已校验
    /// 同名（错配是专用 parse 错误码），elab 只负责弹栈。
    End {
        name: String,
        span: Span,
    },
    /// `open <name>`（G-05）：把 `<name>.` 加进**可省略前缀**集合。
    /// 只影响引用解析（N4），不重命名任何东西、不产生事件。
    ///
    /// `open scoped <name>`（第三刀 §12.3）：`scoped: true` ⇒ **只**打开记法
    /// 作用域（把该作用域里 `scoped` 声明的记法搬进生效表），**不**打开名字
    /// 前缀——与 Lean 一致。两条的**副作用都在解析期**（elab 只忽略它）。
    ///
    /// `filter`（第二刀 §N7）：`open Foo (a b)` / `open Foo hiding a b` /
    /// `open Foo renaming a => b` 三条互斥子句。`scoped: true` 时恒为空
    /// （`open scoped` 不吃子句）。
    Open {
        name: String,
        scoped: bool,
        filter: OpenFilter,
        span: Span,
    },
    /// `open <name> [<子句>] in <命令>`（第二刀 §N7）：**局部 open**——只对
    /// 紧跟的那一条命令生效，命令结束即撤销。
    ///
    /// `header` 只覆盖 open 头部（`open Foo hiding a`，不含 `in`）：`by` 引擎
    /// 的合成前缀要按**源码文本**补一行 open（`walk.rs`），而 `span` 是整条
    /// 命令（含被包住的命令），用于错误归因与排序。
    OpenIn {
        name: String,
        filter: OpenFilter,
        inner: Box<Command>,
        header: Span,
        span: Span,
    },
    /// `export <name> [<子句>]`（第二刀 §N7）：把命名空间里的短名**导出**。
    ///
    /// 本语言没有模块系统，所以语义钉成两条：① **本文件**里对**后续命令**
    /// 与 `open` 完全一样（同一份候选表、同一个位次）；② 它**跨 `import`**——
    /// 导入本文件的入口文件在文件头就能用这些短名（`open` 不跨，见 N5）。
    /// 因此 `export` 是 `open` 的**传播版**，不是第二个名字真相：内核里的名字
    /// 一个都没变（设计 §N7 与 §6 差异 5）。
    Export {
        name: String,
        filter: OpenFilter,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CtorDecl {
    pub name: String,
    pub binders: Vec<Binder>,
    pub result: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecDecl {
    pub name: String,
    pub universe: Vec<String>,
    pub ty: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IotaRule {
    pub ctor_name: String,
    pub val: Expr,
    pub span: Span,
}

impl Command {
    pub fn span(&self) -> Span {
        match self {
            Command::Import { span, .. }
            | Command::Def { span, .. }
            | Command::Theorem { span, .. }
            | Command::Example { span, .. }
            | Command::Axiom { span, .. }
            | Command::InductiveBlock { span, .. }
            | Command::Check { span, .. }
            | Command::Reduce { span, .. }
            | Command::Print { span, .. }
            | Command::Notation { span, .. }
            | Command::Namespace { span, .. }
            | Command::End { span, .. }
            | Command::Open { span, .. }
            | Command::OpenIn { span, .. }
            | Command::Export { span, .. } => *span,
        }
    }

    /// 这条命令是不是 `import`（用于"import 必须置顶"与项目加载）。
    pub fn is_import(&self) -> bool {
        matches!(self, Command::Import { .. })
    }

    /// `open … in <命令>` 包住的那条命令（其余命令 ⇒ `None`）。
    ///
    /// 声明级 pass（模板、hover 回填、着色、警告）用它展开——局部 open 包住的
    /// 声明**照样是声明**；作用域级 pass（`Walk`）不展开，`OpenIn` 自己负责
    /// 压/弹作用域。
    pub fn wrapped_command(&self) -> Option<&Command> {
        match self {
            Command::OpenIn { inner, .. } => Some(inner),
            _ => None,
        }
    }

    /// `import` 声明的模块名。
    pub fn import_module(&self) -> Option<&str> {
        match self {
            Command::Import { module, .. } => Some(module),
            _ => None,
        }
    }

    /// 记法命令的**纯数据载荷**（跨 `import` 传播用，第二刀 §10.3）。
    /// 非记法命令返回 `None`。
    pub fn notation_decl(&self) -> Option<NotationDecl> {
        match self {
            Command::Notation {
                symbol,
                precedence,
                assoc,
                target,
                scope,
                span,
            } => Some(NotationDecl {
                symbol: symbol.clone(),
                precedence: *precedence,
                assoc: *assoc,
                target: target.clone(),
                scope: scope.clone(),
                span: *span,
                // 模块名由**加载层**补（`absorb_notations` 知道自己在哪个模块里，
                // 而这条命令自己不知道）——见 `project/graph.rs`。
                module: None,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FolFile {
    pub commands: Vec<Command>,
    /// 源文件原文（`parse` 时填入）。`by` 引擎按命令 span 切片取前缀源码
    /// 供 `judge_terms` 判定；judge 合成的 FolFile 置空。
    pub src: String,
}

/// 文件里的**有效命令**（第二刀 §N7）：`open … in <命令>` 展开成它包住的
/// 命令（`OpenIn` 自己也在序列里，它没有名字、对声明级 pass 是空操作）。
/// 嵌套（`open A in open B in def …`）逐层展开。
///
/// 用途：所有"按命令找声明/表达式"的 pass（`top_level_def_spans`、
/// `GoalTemplates`、着色、警告）。**不**用于 `Walk`——那里 `OpenIn` 要负责
/// 压/弹作用域，展开会把局部 open 变成全局。
pub fn effective_commands(file: &FolFile) -> Vec<&Command> {
    let mut out = Vec::with_capacity(file.commands.len());
    for command in &file.commands {
        let mut current = command;
        loop {
            out.push(current);
            match current.wrapped_command() {
                Some(inner) => current = inner,
                None => break,
            }
        }
    }
    out
}
