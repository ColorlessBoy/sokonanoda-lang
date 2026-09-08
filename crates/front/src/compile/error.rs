//! 编译错误分类：stage/kind、机器错误码与教学提示（`CompileError`）。

use crate::Span;

/// Which pipeline stage produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStage {
    Elab,
    Kernel,
}

impl CompileStage {
    pub fn code(self) -> &'static str {
        match self {
            CompileStage::Elab => "elab",
            CompileStage::Kernel => "kernel",
        }
    }
}

/// Stable, fine-grained error kind below the stage level. Every kind maps to a
/// machine code and a first teaching hint (docs/design-infrastructure.md F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    ElabUnknownIdentifier,
    ElabUnknownConstant,
    ElabUnknownUniverseLevel,
    ElabUniverseArity,
    ElabUntypedBinder,
    ElabHoleMisplaced,
    ElabDuplicateDeclaration,
    ElabTooManyBinders,
    ElabNatLiteralDisabled,
    ElabInvalidNatLiteral,
    ElabTooManyCtorFields,
    ElabUnknownCtorForIota,
    KernelExpectedSort,
    KernelExpectedPi,
    KernelTheoremNotProp,
    KernelNonPositive,
    KernelCtorResultMismatch,
    KernelCtorArgInvalidApp,
    KernelCtorArgNotType,
    KernelCtorArgTooLarge,
    KernelRecRuleMismatch,
    KernelRejected,
    KernelInternal,
}

impl ErrorKind {
    pub fn stage(self) -> CompileStage {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier
            | ElabUnknownConstant
            | ElabUnknownUniverseLevel
            | ElabUniverseArity
            | ElabUntypedBinder
            | ElabHoleMisplaced
            | ElabDuplicateDeclaration
            | ElabTooManyBinders
            | ElabNatLiteralDisabled
            | ElabInvalidNatLiteral
            | ElabTooManyCtorFields
            | ElabUnknownCtorForIota => CompileStage::Elab,
            KernelExpectedSort
            | KernelExpectedPi
            | KernelTheoremNotProp
            | KernelNonPositive
            | KernelCtorResultMismatch
            | KernelCtorArgInvalidApp
            | KernelCtorArgNotType
            | KernelCtorArgTooLarge
            | KernelRecRuleMismatch
            | KernelRejected
            | KernelInternal => CompileStage::Kernel,
        }
    }

    pub fn code(self) -> &'static str {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier => "elab-unknown-identifier",
            ElabUnknownConstant => "elab-unknown-constant",
            ElabUnknownUniverseLevel => "elab-unknown-universe-level",
            ElabUniverseArity => "elab-universe-arity",
            ElabUntypedBinder => "elab-untyped-binder",
            ElabHoleMisplaced => "elab-hole-misplaced",
            ElabDuplicateDeclaration => "elab-duplicate-declaration",
            ElabTooManyBinders => "elab-too-many-binders",
            ElabNatLiteralDisabled => "elab-nat-literal-disabled",
            ElabInvalidNatLiteral => "elab-invalid-nat-literal",
            ElabTooManyCtorFields => "elab-too-many-ctor-fields",
            ElabUnknownCtorForIota => "elab-unknown-ctor-for-iota",
            KernelExpectedSort => "kernel-expected-sort",
            KernelExpectedPi => "kernel-expected-pi",
            KernelTheoremNotProp => "kernel-theorem-not-prop",
            KernelNonPositive => "kernel-inductive-non-positive",
            KernelCtorResultMismatch => "kernel-ctor-result-mismatch",
            KernelCtorArgInvalidApp => "kernel-ctor-arg-invalid-app",
            KernelCtorArgNotType => "kernel-ctor-arg-not-type",
            KernelCtorArgTooLarge => "kernel-ctor-arg-too-large",
            KernelRecRuleMismatch => "kernel-rec-rule-mismatch",
            KernelRejected => "kernel-rejected",
            KernelInternal => "kernel-internal",
        }
    }

    /// First-version teaching hint. Learners read these next to the squiggle;
    /// keep them short, concrete and actionable.
    pub fn hint(self) -> &'static str {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier => {
                "这个名字还没有被定义。检查拼写，或确认它出现在你前面的某个声明里（练习要在解决之后才能被后面的代码引用）。"
            }
            ElabUnknownConstant => {
                "这里引用了一个不存在的常量。如果它带宇宙参数，请先定义它。"
            }
            ElabUnknownUniverseLevel => {
                "这个宇宙层级变量没有在当前声明里声明。用 {u} 声明它，例如 def id {u} : ...。"
            }
            ElabUniverseArity => {
                "宇宙参数个数不对。这个常量声明了几个宇宙参数，就要给几个，例如 id.{u, v}。"
            }
            ElabUntypedBinder => {
                "这个 binder 缺少类型标注。教学版本要求写全类型，例如 fun (x : Nat) => x。"
            }
            ElabHoleMisplaced => {
                "sorry 只能出现在声明的值（答案区）位置，例如 example : T := sorry。"
            }
            ElabDuplicateDeclaration => {
                "这个名字已经定义过了。Lean 里每个名字只能声明一次；换一个名字，或删掉前面的声明。"
            }
            ElabTooManyBinders => {
                "嵌套的 binder 太多，超出了内核能表示的深度。把大表达式拆成几个小定义。"
            }
            ElabNatLiteralDisabled => {
                "数字字面量没有被启用。这个版本默认打开 Nat 扩展，如遇到此错误请联系工具作者。"
            }
            ElabInvalidNatLiteral => {
                "这不是一个合法的自然数字面量。"
            }
            ElabTooManyCtorFields => {
                "构造子的字段太多了。"
            }
            ElabUnknownCtorForIota => {
                "iota 规则引用了一个不存在的构造子。检查构造子名字是否与 ctor 声明一致。"
            }
            KernelExpectedSort => {
                "这里需要写一个类型（如 Prop、Type、Nat），但你写成了一个普通的项。检查冒号/binder 后面跟的是不是类型。"
            }
            KernelExpectedPi => {
                "你把一个不是函数的值当函数用了，或者参数给多了。检查这个位置的东西的类型是不是 … -> … 形状。"
            }
            KernelTheoremNotProp => {
                "theorem 的类型必须是命题（Prop 里的东西）。想定义普通值请用 def。"
            }
            KernelNonPositive => {
                "递归引用出现在了负位置：构造子参数里 T 出现在箭头左边（如 T → Nat）。递归引用只能写在返回类型一侧。"
            }
            KernelCtorResultMismatch => {
                "构造子的返回类型必须是本 inductive 的完整应用——参数和索引都要补齐，比如 T A n 而不是只写 T。"
            }
            KernelCtorArgInvalidApp => {
                "构造子参数里的递归引用 T … 不是本 inductive 的合法应用：参数/索引的个数或取值不对。"
            }
            KernelCtorArgNotType => {
                "构造子参数的类型本身必须是一个类型，这里写成了一个项。"
            }
            KernelCtorArgTooLarge => {
                "构造子参数的类型所在的宇宙太大，装不进这个 inductive。把 inductive 声明成更大的 Type，或把该参数类型改成 Prop 里的命题。"
            }
            KernelRecRuleMismatch => {
                "显式消去子（rec/iota）与内核推导出的规则不一致：iota 规则必须按构造子声明顺序一条不落地给出，每条规则的值也要与 motive、minor 和构造子参数的形状完全匹配。"
            }
            KernelRejected => {
                "内核判定不成立：类型不匹配或证明项不完整。先对比期望类型与你的值的形状；最常见的错误是两边结构不同（例如期望 a -> a，却写成了返回 Nat 的项）。"
            }
            KernelInternal => {
                "内核内部错误（这不是你的代码问题）。请把这段代码反馈给工具作者。"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
    pub kind: ErrorKind,
    /// For kernel type mismatches (`kernel-rejected`): the expected type as
    /// rendered by the kernel, when the rejection message carried both sides.
    pub expected: Option<String>,
    /// For kernel type mismatches (`kernel-rejected`): the actual (inferred)
    /// type, when the rejection message carried both sides.
    pub actual: Option<String>,
}

impl CompileError {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind,
            expected: None,
            actual: None,
        }
    }

    pub(crate) fn elab(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self::new(kind, message, span)
    }

    pub(crate) fn kernel(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self::new(kind, message, span)
    }

    pub fn stage(&self) -> CompileStage {
        self.kind.stage()
    }

    /// Stable machine code (used by `--json` and the protocol).
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// First teaching hint for this error.
    pub fn hint(&self) -> &'static str {
        self.kind.hint()
    }
}

/// Stable markers the kernel embeds in def-eq failure panics:
/// `def_eq mismatch expected: <E-TEXT> | actual: <A-TEXT>`
/// (the panic payload is wrapped into `rejected: ...` by `CheckError`).
const DEF_EQ_MARKER: &str = "def_eq mismatch expected: ";
const DEF_EQ_ACTUAL_SEP: &str = " | actual: ";

/// Parse the kernel's def-eq mismatch message into `(expected, actual)`.
/// Returns `None` for rejections that do not carry both sides.
pub(crate) fn parse_def_eq_mismatch(msg: &str) -> Option<(String, String)> {
    let start = msg.find(DEF_EQ_MARKER)? + DEF_EQ_MARKER.len();
    let rest = &msg[start..];
    let sep = rest.find(DEF_EQ_ACTUAL_SEP)?;
    let expected = rest[..sep].trim().to_string();
    let actual = rest[sep + DEF_EQ_ACTUAL_SEP.len()..].trim().to_string();
    if expected.is_empty() || actual.is_empty() {
        return None;
    }
    Some((expected, actual))
}

/// Map a kernel rejection panic message to the most precise [`ErrorKind`].
/// The kernel reports rejections as panics; `CheckError::Rejected` wraps the
/// payload as `rejected: <payload>`. def_eq mismatches carry a stable marker
/// (parsed separately by [`parse_def_eq_mismatch`]) and stay
/// [`ErrorKind::KernelRejected`] here. Other payload shapes are classified
/// into fine-grained message families; anything that looks like a broken
/// kernel invariant (`assertion failed: …`, `unwrap()`) is an internal error,
/// not a learner mistake.
pub(crate) fn refine_kernel_kind(msg: &str) -> ErrorKind {
    let payload = msg.strip_prefix("rejected: ").unwrap_or(msg);

    // def_eq mismatches: check.rs re-renders both sides from the marker;
    // the classifier must not interfere with them.
    if payload.starts_with("def_eq failed:") || payload.starts_with(DEF_EQ_MARKER) {
        return ErrorKind::KernelRejected;
    }
    if payload.starts_with("expected a sort") {
        return ErrorKind::KernelExpectedSort;
    }
    if payload.starts_with("expected a pi type")
        || payload.contains("spine_type_with_value: expected Pi")
    {
        return ErrorKind::KernelExpectedPi;
    }
    if payload.contains("theorem type must be Prop") {
        return ErrorKind::KernelTheoremNotProp;
    }
    if payload.contains("non-positive occurrence") {
        return ErrorKind::KernelNonPositive;
    }
    if payload.starts_with("constructor must return a full application") {
        return ErrorKind::KernelCtorResultMismatch;
    }
    if payload.starts_with("recursive occurrence in constructor is not a valid application") {
        return ErrorKind::KernelCtorArgInvalidApp;
    }
    if payload.contains("constructor argument is not a type") {
        return ErrorKind::KernelCtorArgNotType;
    }
    if payload.starts_with("Constructor argument was too large") {
        return ErrorKind::KernelCtorArgTooLarge;
    }
    // Explicit recursor declarations (`rec`/`iota` in an inductive block) are
    // compared against the kernel-reconstructed rules; order, count, rule
    // value or recursor-name mismatches are learner mistakes, not kernel bugs.
    if payload.starts_with("iota rule is not listed in constructor declaration order")
        || payload.starts_with("iota rule count does not match the constructor count")
        || payload.starts_with("imported recursor rule does not match the reconstructed rule")
        || payload.starts_with("imported inductive block contains an underived recursor")
    {
        return ErrorKind::KernelRecRuleMismatch;
    }
    if payload.starts_with("assertion failed:")
        || payload.contains("called `Option::unwrap()`")
        || payload.contains("called `Result::unwrap()`")
        || payload.contains("internal error:")
    {
        return ErrorKind::KernelInternal;
    }
    ErrorKind::KernelRejected
}
