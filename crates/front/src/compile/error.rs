//! 编译错误分类：stage/kind、机器错误码与教学提示（`CompileError`）。
use serde::{Deserialize, Serialize};

use crate::Span;

/// Which pipeline stage produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileStage {
    /// 项目层的 import 解析（找不到模块、成环、依赖失败…）。
    Import,
    Elab,
    Kernel,
}

impl CompileStage {
    pub fn code(self) -> &'static str {
        match self {
            CompileStage::Import => "import",
            CompileStage::Elab => "elab",
            CompileStage::Kernel => "kernel",
        }
    }
}

/// Stable, fine-grained error kind below the stage level. Every kind maps to a
/// machine code and a first teaching hint (docs/design/infrastructure.md F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    ElabAmbiguousCtorAlias,
    ElabTacticFailed,
    ElabApplyNeedsATerm,
    ElabApplyNotApplicable,
    ElabMatchBadArm,
    ElabMatchNotInductive,
    ElabMatchNoExpectedType,
    ElabMatchRecursiveUnsupported,
    ElabMatchNonExhaustive,
    ElabMatchParameterizedUnsupported,
    ElabLetTypeQueryFailed,
    /// 记法命令指向的目标名不存在（`infix:50 " ∈ " => Set.mem` 但 `Set.mem`
    /// 没声明）。G-04 / WO-011，设计 `docs/design/notation-subset.md` N1。
    ElabNotationUnknownTarget,
    /// 记号展开时补不出目标 telescope 的**前导类型参数**（v1 只做裸变量匹配，
    /// 不做一般合一）。hint 教点名写法。设计 N4.2。
    ElabNotationArgumentUnsolved,
    /// **记法重载**选不出候选（第三刀 §12.2）：期望类型筛完还剩 ≥2 个候选
    /// （或根本没有期望类型却有多个候选）。hint 列候选 + 教点名写法消歧。
    ElabNotationAmbiguous,
    /// **记法重载**一个候选都对不上期望类型（第三刀 §12.2）：hint 列候选与
    /// 各自的**结果类型**，让学习者看见"为什么都不匹配"。
    ElabNotationNoCandidate,
    /// **binder 记法**的 binder 类型解不出（第三刀 §12.1）：`∀ x ∈ s, p` /
    /// `∃ x ∈ s, p` 的 `x` 类型由 guard 的关系（`∈`）反解，反解不出时报这条；
    /// hint 教补 `(x : α)` 标注或写点名形式。
    ElabBinderNotationUnsolved,
    /// **集合字面量**（第三刀 §12.4）展开成的点名目标不在本文件里
    /// （`Set.singleton` / `Set.pair` 是卷 I 的库定义）。hint 教 import 或点名。
    ElabSetLiteralUnknownTarget,
    KernelExpectedSort,
    KernelExpectedPi,
    KernelTheoremNotProp,
    /// **没有累积性**（L-06）：内核要 `Sort(n)`（`n ≥ 1`，数据/`Type`），
    /// 学习者给的是 `Sort(0)`（`Prop` 命题）。官方 Lean 4 有累积性
    /// （`Prop ⊆ Type`），本语言没有——这是设计边界，不是内核缺陷；专用码 +
    /// hint 只是把内核的裸类型不匹配翻译成人话（设计
    /// `docs/design/prop-cumulativity-boundary.md`）。
    KernelPropNotCumulative,
    KernelNonPositive,
    KernelCtorResultMismatch,
    KernelCtorArgInvalidApp,
    KernelCtorArgNotType,
    KernelCtorArgTooLarge,
    KernelRecRuleMismatch,
    KernelRejected,
    KernelInternal,
    /// 找不到被 `import` 的模块（模块根下没有对应文件）。
    ImportNotFound,
    /// `import` 成环（含自导入）。
    ImportCycle,
    /// 被导入的模块没有编译成功：下游不编译，只在 import 行上报一条。
    ImportDependencyFailed,
    /// 两个模块（或本文件与某个模块）声明了同一个顶层名字。
    ImportNameCollision,
    /// 闭包内的 prelude 形状冲突（入口 Bare 而依赖 Full、依赖占用了 prelude 名字…）。
    ImportPreludeConflict,
    /// `sokonanoda.toml` 读不了或不是合法 TOML。
    ManifestInvalid,
    /// 被导入的模块文件本身有语法错误（parse 阶段失败）。
    ImportModuleInvalid,
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
            | ElabUnknownCtorForIota
            | ElabAmbiguousCtorAlias
            | ElabTacticFailed
            | ElabApplyNeedsATerm
            | ElabApplyNotApplicable
            | ElabMatchBadArm
            | ElabMatchNotInductive
            | ElabMatchNoExpectedType
            | ElabMatchRecursiveUnsupported
            | ElabMatchNonExhaustive
            | ElabMatchParameterizedUnsupported
            | ElabLetTypeQueryFailed
            | ElabNotationUnknownTarget
            | ElabNotationArgumentUnsolved
            | ElabNotationAmbiguous
            | ElabNotationNoCandidate
            | ElabBinderNotationUnsolved
            | ElabSetLiteralUnknownTarget => CompileStage::Elab,
            KernelExpectedSort
            | KernelExpectedPi
            | KernelTheoremNotProp
            | KernelPropNotCumulative
            | KernelNonPositive
            | KernelCtorResultMismatch
            | KernelCtorArgInvalidApp
            | KernelCtorArgNotType
            | KernelCtorArgTooLarge
            | KernelRecRuleMismatch
            | KernelRejected
            | KernelInternal => CompileStage::Kernel,
            ImportNotFound
            | ImportCycle
            | ImportDependencyFailed
            | ImportNameCollision
            | ImportPreludeConflict
            | ManifestInvalid
            | ImportModuleInvalid => CompileStage::Import,
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
            ElabAmbiguousCtorAlias => "elab-ambiguous-ctor-alias",
            ElabTacticFailed => "elab-tactic-failed",
            ElabApplyNeedsATerm => "elab-apply-needs-a-term",
            ElabApplyNotApplicable => "elab-apply-not-applicable",
            ElabMatchBadArm => "elab-match-bad-arm",
            ElabMatchNotInductive => "elab-match-not-inductive",
            ElabMatchNoExpectedType => "elab-match-no-expected-type",
            ElabMatchRecursiveUnsupported => "elab-match-recursive-unsupported",
            ElabMatchNonExhaustive => "elab-match-non-exhaustive",
            ElabMatchParameterizedUnsupported => "elab-match-parameterized-unsupported",
            ElabLetTypeQueryFailed => "elab-let-type-query-failed",
            ElabNotationUnknownTarget => "elab-notation-unknown-target",
            ElabNotationArgumentUnsolved => "elab-notation-argument-unsolved",
            ElabNotationAmbiguous => "elab-notation-ambiguous",
            ElabNotationNoCandidate => "elab-notation-no-candidate",
            ElabBinderNotationUnsolved => "elab-binder-notation-unsolved",
            ElabSetLiteralUnknownTarget => "elab-set-literal-unknown-target",
            KernelExpectedSort => "kernel-expected-sort",
            KernelExpectedPi => "kernel-expected-pi",
            KernelTheoremNotProp => "kernel-theorem-not-prop",
            KernelPropNotCumulative => "kernel-prop-not-cumulative",
            KernelNonPositive => "kernel-inductive-non-positive",
            KernelCtorResultMismatch => "kernel-ctor-result-mismatch",
            KernelCtorArgInvalidApp => "kernel-ctor-arg-invalid-app",
            KernelCtorArgNotType => "kernel-ctor-arg-not-type",
            KernelCtorArgTooLarge => "kernel-ctor-arg-too-large",
            KernelRecRuleMismatch => "kernel-rec-rule-mismatch",
            KernelRejected => "kernel-rejected",
            KernelInternal => "kernel-internal",
            ImportNotFound => "import-not-found",
            ImportCycle => "import-cycle",
            ImportDependencyFailed => "import-dependency-failed",
            ImportNameCollision => "import-name-collision",
            ImportPreludeConflict => "import-prelude-conflict",
            ManifestInvalid => "manifest-invalid",
            ImportModuleInvalid => "import-module-invalid",
        }
    }

    /// First-version teaching hint. Learners read these next to the squiggle;
    /// keep them short, concrete and actionable.
    pub fn hint(self) -> &'static str {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier => {
                "这个名字还没有被定义。检查拼写，或确认它出现在你前面的某个声明里（练习要在解决之后才能被后面的代码引用）。若它在该命名空间里，检查前缀或加 open。"
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
                "这个 binder 缺少类型标注。教学版本要求写全类型，例如 fun (x : Nat) => x；let 的绑定也要写类型，例如 let x : Nat := 1; x。"
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
            ElabAmbiguousCtorAlias => {
                "这个裸构造子名被两个类型各声明了一次，无法判断是哪一个。写全前缀名（例如 `P1.mk`），或给其中一个构造子换个名字。"
            }
            ElabTacticFailed => {
                "`by` 块里的 tactic 失败了：请检查当前目标与已引入的假设。"
            }
            ElabApplyNeedsATerm => {
                "`funapply` 后面要跟一个证明或函数，例如 `funapply h`；要应用的项复杂时可以用括号界定范围，例如 `funapply (f a)`。"
            }
            ElabApplyNotApplicable => {
                "`funapply h` 要求 `h` 的结论正好是当前目标（`h : … -> 目标`）。看看 `h` 类型的最后一段是不是当前目标；不是就换一个前提，或直接写答案。"
            }
            ElabMatchBadArm => {
                "match 的模式要写对：构造子名与字段数要对应（可以嵌套，如 some (succ k)）；不确定的名字按变量绑定处理，带子模式的未知名才报错。"
            }
            ElabMatchNotInductive => {
                "match 的被匹配项必须是已知的归纳类型：本文件用 inductive 声明的类型，或 prelude 内建的 Nat/Bool（分支写 Nat.zero/Nat.succ 或 Bool.true/Bool.false）。"
            }
            ElabMatchNoExpectedType => {
                "match 的结果类型必须已知：把它放在有类型标注的位置（声明类型、let/fun 的 binder 注解），或由外层 match 提供。"
            }
            ElabMatchRecursiveUnsupported => {
                "match 暂不支持递归归纳类型（v1 只做没有归纳假设的非递归分情况）；递归定义请直接用消去子 .rec。"
            }
            ElabMatchNonExhaustive => {
                "match 要覆盖该归纳类型的每一个构造子（含每个字段位置）；带守卫的 arm 还要在后面补一条不带守卫的兜底。"
            }
            ElabMatchParameterizedUnsupported => {
                "参数化归纳的 match 目前只支持：被匹配项是一个局部变量，且它的类型写成 `T 参数…`（显式给出归纳的全部参数）。换成一个这样标注的变量再 match。"
            }
            ElabLetTypeQueryFailed => {
                "无法从值推断出 `let` 绑定的类型；补上类型标注即可，例如 `let x : Nat := 1; x`。"
            }
            ElabNotationUnknownTarget => {
                "记法命令指向的目标名不存在。检查 infix/notation 行里 `=>` 后面的名字拼写（要写点名，例如 Set.mem），并确认它已经声明过。"
            }
            ElabNotationArgumentUnsolved => {
                "这个记法展开时补不出前面的类型参数（本子集只按操作数的类型补，不做一般推断）。改用点名写法把参数写全，例如 Set.mem α a A；或在两边都是已知类型的上下文里使用记法。"
            }
            ElabNotationAmbiguous => {
                "同一个符号声明了多条记法（重载），这里从期望类型看不出该用哪一条。写出点名形式（例如 Set.mem α a A）就消歧了；或者把这个表达式放到一个带类型标注的位置（例如 def … : T := 这里），让期望类型能定下来。"
            }
            ElabNotationNoCandidate => {
                "同一个符号的几条记法候选，结果类型都对不上这里的期望类型。对照错误里列出的候选结果类型，检查是不是用错了符号，或者改用点名形式。"
            }
            ElabBinderNotationUnsolved => {
                "binder 记法里的变量类型解不出：`∀ x ∈ s, p` / `∃ x ∈ s, p` 的 x 类型是从 `∈` 两边反解的。给 binder 补上类型标注（例如 ∀ (x : α) ∈ s, p），或改用点名写法（forall (x : α), Set.mem α x s -> p）。"
            }
            ElabSetLiteralUnknownTarget => {
                "集合字面量 `{a}` / `{a, b}` 展开成点名形式 Set.singleton / Set.pair，但这个文件里没有它们。先 `import` 提供它们的库（卷 I 的 lib/Set），或改用点名写法。"
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
            KernelPropNotCumulative => {
                "这里需要 Type（数据），但你给的是 Prop（命题）：本语言没有累积性，Prop 不是 Type 的子集（官方 Lean 4 有累积性，同一段代码在 Lean 里能过）。把陈述改成 Prop（例如等势用 Set.Equiv … : Prop 这样的命题版），或者交一个真正的 Type 值（如 Nat）。"
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
            ImportNotFound => {
                "找不到这个模块。`import Foo.Bar` 对应模块根下的 `Foo/Bar.sokonanoda`；检查名字拼写与文件位置（名字里不能有 `-`）。"
            }
            ImportCycle => {
                "import 成环了：A 依赖 B、B 又（直接或间接）依赖 A。把公共部分抽到一个更底层的模块里。"
            }
            ImportDependencyFailed => {
                "被导入的模块自己还有错误，所以这里先不编译——修好那个文件的第一条错误再看这里。"
            }
            ImportNameCollision => {
                "同一个名字在两个模块里各声明了一次。改掉其中一个，或把它移到一个公共模块里只声明一次。"
            }
            ImportPreludeConflict => {
                "prelude（内置基元）是整个编译单元的属性：入口文件的设置与依赖模块冲突了。统一成一种（要么都用内置 prelude，要么都在入口声明 `-- sokonanoda:prelude none`）。"
            }
            ManifestInvalid => {
                "`sokonanoda.toml` 读不了或不是合法 TOML。它只需要可选的 `name` / `requires` / `src` 三个键；不想要项目就把文件删掉（单文件模式仍然可用）。"
            }
            ImportModuleInvalid => {
                "被导入的文件本身没解析成功：先打开它、修好那里的语法错误，再回来看这里（下游不会替你猜）。"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// The **trailing** sort level of a kernel-rendered type: `Sort(0)` → `Some(0)`,
/// `Pi (x : Nat), Sort(1)` → `Some(1)`, `Nat` → `None`.
///
/// `rfind` is deliberate: the mismatch that matters is the *codomain* sort (the
/// type the value must inhabit), while earlier `Sort(…)`s belong to the
/// domains. Levels the kernel renders as variables (`Sort(u)`) parse to `None`
/// and stay unclassified.
fn trailing_sort_level(rendered: &str) -> Option<u32> {
    const OPEN: &str = "Sort(";
    let start = rendered.rfind(OPEN)? + OPEN.len();
    let rest = &rendered[start..];
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

/// Classify a def-eq mismatch as the language's **non-cumulativity** boundary
/// (L-06): the kernel wanted `Sort(n)` with `n ≥ 1` (a `Type`/data value) and
/// the term inhabits `Sort(0)` (a `Prop`).
///
/// Official Lean 4 has cumulativity (`Prop ⊆ Type`), so the same source checks
/// there; here it is a design boundary and deserves a dedicated code plus a
/// human hint instead of the bare "类型不匹配".
///
/// **Scope (deliberate, documented in
/// `docs/design/prop-cumulativity-boundary.md` §3): only the bare-sort shape**
/// — the mismatch between two *sorts* themselves (`def T : Type := True`).
/// The `Pi`-shaped variant (`def bad : Prop -> Type := fun (x : Prop) => x`) is
/// the same phenomenon, but it is also the fixture that the CLI/LSP/extension
/// contract tests use to represent a **generic** kernel rejection (they pin
/// `code == "kernel-rejected"`, stage, span and expected/actual). Widening the
/// rule to `Pi` shapes would change those fixtures, so it is left for a
/// dedicated migration rather than smuggled in here.
///
/// The mirror image (`Sort(0)` expected, `Sort(m)` actual) is deliberately
/// **not** classified at all: it is textually identical to an ordinary
/// "you wrote a `Type` where a `Prop` was expected" (`def f : Prop := Nat`),
/// so a code claiming "no large elimination" would mislabel it.
fn classify_prop_sort_gap(expected: &str, actual: &str) -> Option<ErrorKind> {
    if expected.contains("Pi ") || actual.contains("Pi ") {
        return None;
    }
    let expected_level = trailing_sort_level(expected)?;
    let actual_level = trailing_sort_level(actual)?;
    (expected_level > 0 && actual_level == 0).then_some(ErrorKind::KernelPropNotCumulative)
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

    // def_eq mismatches: check/kernel_phase.rs re-renders both sides from the marker;
    // the classifier must not interfere with them — except for the one shape it
    // can name precisely (L-06: a `Prop` where a `Type` was required, i.e. the
    // language has no cumulativity), which gets its own code + hint.
    if payload.starts_with("def_eq failed:") || payload.starts_with(DEF_EQ_MARKER) {
        if let Some((expected, actual)) = parse_def_eq_mismatch(payload) {
            if let Some(kind) = classify_prop_sort_gap(&expected, &actual) {
                return kind;
            }
        }
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
