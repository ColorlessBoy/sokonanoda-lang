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
    /// `:= by <tactic 序列>`：值位是一段 tactic 脚本，由编译期引擎翻译成
    /// 普通表达式（可能带尾部 `sorry`）。
    By {
        tactics: Vec<Tactic>,
        span: Span,
    },
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
            | Expr::By { span, .. } => *span,
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
            Command::Def { span, .. }
            | Command::Theorem { span, .. }
            | Command::Example { span, .. }
            | Command::Axiom { span, .. }
            | Command::InductiveBlock { span, .. }
            | Command::Check { span, .. }
            | Command::Reduce { span, .. }
            | Command::Print { span, .. } => *span,
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
