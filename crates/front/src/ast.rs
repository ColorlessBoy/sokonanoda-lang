//! `.sokonanoda` 抽象语法树：排序、表达式、binder、命令与文件结构。

use crate::span::Span;

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
        span: Span,
    },
}

/// 记法命令的结合性 / 元数（`docs/design/notation-subset.md` N1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationAssoc {
    /// `infix:N`：无结合，两边同级。
    Infix,
    /// `infixl:N`：左结合。
    Infixl,
    /// `infixr:N`：右结合。
    Infixr,
    /// `notation "∅" => …`：零元常量记法（无优先级）。
    Nullary,
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
            | Expr::Notation { span, .. } => *span,
        }
    }
}

/// 教学白名单里的一个 tactic（`by` 块内）。
#[derive(Debug, Clone, PartialEq)]
pub enum Tactic {
    Intro {
        name: String,
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
    /// `sorry`：占位——当前目标保持开放（合法 Open 状态），
    /// 与声明值位的 `sorry` 同语义（未完成证明）。
    Sorry {
        span: Span,
    },
}

impl Tactic {
    pub fn span(&self) -> Span {
        match self {
            Tactic::Intro { span, .. }
            | Tactic::Exact { span, .. }
            | Tactic::Apply { span, .. }
            | Tactic::Assumption { span }
            | Tactic::Rfl { span }
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
    /// `infix:N " sym " => name` / `infixl` / `infixr` / `notation " sym " => name`
    /// （G-04 / WO-011）。
    ///
    /// **不是声明**（N6）：不产生 `decl.checked`/`exercise.open`/`expr.typed`，
    /// 也不进声明表——与 `Command::Import` 同族（有命令、无声明）。
    /// `precedence` 只有零元记法为 `None`。
    Notation {
        symbol: String,
        precedence: Option<u16>,
        assoc: NotationAssoc,
        target: String,
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
            | Command::Notation { span, .. } => *span,
        }
    }

    /// 这条命令是不是 `import`（用于"import 必须置顶"与项目加载）。
    pub fn is_import(&self) -> bool {
        matches!(self, Command::Import { .. })
    }

    /// `import` 声明的模块名。
    pub fn import_module(&self) -> Option<&str> {
        match self {
            Command::Import { module, .. } => Some(module),
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
