//! 编译错误分类：stage/kind、机器错误码与教学提示（`CompileError`）。  // 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
use serde::{Deserialize, Serialize};

use crate::Span;

/// Which pipeline stage produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileStage {
    /// **解析**（`EntryParseFailed`：入口文件自己的语法错误 —— 长尾 ②/r06）。
    ///
    /// 以前没有这一档：入口的解析失败被包成 `ImportModuleInvalid`（`import`
    /// 阶段），于是 `--json` 的 `stage` 与 LSP 的诊断来源一起指错文件。
    Parse,
    /// 项目层的 import 解析（找不到模块、成环、依赖失败…）。
    Import,
    Elab,
    Kernel,
}

impl CompileStage {
    pub fn code(self) -> &'static str {
        match self {
            CompileStage::Parse => "parse",
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

    /// **U2-a**：常量**写出来的**宇宙实参与**解出来的**字面冲突（`Show.{0} …` 而实参要求 `Sort 1` ✓）。
    /// 单独一条码（设计 §2.9 的"超限报单独通道" ✓）—— 以前这种形状会一路落到内核，
    /// 报成 `类型不匹配：期望 Sort(1)，实际是 Sort(2)` ✗，看不出"是层级写错了"✓。
    ElabUniverseLevelConflict,
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
    /// **隐式实参补不出来**（IA-1，设计 `docs/design/implicit-arguments.md` §3.1）：
    /// 签名有前导隐式 binder，但由后续显式实参的类型反解不出（路线 C **不猜**、
    /// 不做搜索）。hint 教写全参数。
    ElabImplicitArgumentUnsolved,
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
    /// **匿名构造子**（L2.7）`⟨a, b⟩` 读不到**期望类型**：用哪个构造子只能由
    /// 期望类型决定（路线 C，不做合一）。hint 教写进有标注的位置或点名。
    ElabAnonCtorNoExpectedType,
    /// **命名实参不合法**（用户 2026-10-16 第 ② 条）：`f (α := Nat)` 里的 `α`
    /// 不是 `f` 的形参名、同一个形参给了两次、或者这个头**没法按名字绑定**
    /// （不是已知常量）。对齐 Lean 4 `Lean/Elab/App.lean:39 throwInvalidNamedArg`
    /// —— 那边也是**一个码 + 列出可用名**（`:238` 在 `namedArgs` 用不完时抛）✓。
    ElabInvalidNamedArgument,
    KernelExpectedSort,
    KernelExpectedPi,
    KernelTheoremNotProp,
    /// **没有累积性**（L-06）：内核要 `Sort(n)`（`n ≥ 1`，数据/`Type`），
    /// 学习者给的是 `Sort(0)`（`Prop` 命题）。官方 Lean 4 **同样没有**累积性
    /// （`Sort u =?= Sort v` 要求 `u ≡ v`；Lean 要用 `PLift` 显式把命题抬进
    /// `Type`）⇒ 这是与 Lean 一致的语言性质，不是内核缺陷；专用码 + hint 只是把
    /// 内核的裸类型不匹配翻译成人话（2026-10-07 复核：台账 `docs/gaps/ledger.jsonl`
    /// 的 L-06 + 复现件 `docs/gaps/repro/L-06-lean-consistent-no-cumulativity.sh`）。
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
    /// **入口文件自己**没解析成功（parse 阶段失败，2026-10-12 · 长尾 ②/r06）。
    ///
    /// 为什么必须与 [`Self::ImportModuleInvalid`] 分开：闭包加载器以前把
    /// **任何**模块的解析失败都包成 `import-module-invalid`，于是"入口自己写错
    /// 一个符号"被报成"**被导入的文件**坏了" + 甩锅 hint（r06 实测：hint 说
    /// 「先打开它、修好那里的语法错误」，而 `lib/Logic`、`lib/Set` 都是好的）。
    /// 归因判据是**真相层**的"这次 parse 失败的是哪个模块"（入口 vs 依赖），
    /// 不是文件名/路径启发式。
    ///
    /// `stage = parse`（入口自己的解析失败就该是 parse 阶段）；被导入模块那条
    /// 仍是 `import` 阶段的 [`Self::ImportModuleInvalid`]（它确实是加载期发现的）。
    EntryParseFailed,
}

impl ErrorKind {
    pub fn stage(self) -> CompileStage {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier
            | ElabUnknownConstant
            | ElabUnknownUniverseLevel
            | ElabUniverseLevelConflict
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
            | ElabImplicitArgumentUnsolved
            | ElabNotationAmbiguous
            | ElabNotationNoCandidate
            | ElabBinderNotationUnsolved
            | ElabSetLiteralUnknownTarget
            | ElabAnonCtorNoExpectedType
            | ElabInvalidNamedArgument => CompileStage::Elab,
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
            EntryParseFailed => CompileStage::Parse,
        }
    }

    pub fn code(self) -> &'static str {
        use ErrorKind::*;
        match self {
            ElabUnknownIdentifier => "elab-unknown-identifier",
            ElabUnknownConstant => "elab-unknown-constant",
            ElabUnknownUniverseLevel => "elab-unknown-universe-level",
            ElabUniverseLevelConflict => "elab-universe-level-conflict",
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
            ElabImplicitArgumentUnsolved => "elab-implicit-argument-unsolved",
            ElabNotationAmbiguous => "elab-notation-ambiguous",
            ElabNotationNoCandidate => "elab-notation-no-candidate",
            ElabBinderNotationUnsolved => "elab-binder-notation-unsolved",
            ElabSetLiteralUnknownTarget => "elab-set-literal-unknown-target",
            ElabAnonCtorNoExpectedType => "elab-anon-ctor-no-expected-type",
            ElabInvalidNamedArgument => "elab-invalid-named-argument",
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
            EntryParseFailed => "entry-parse-failed",
        }
    }

    /// First-version teaching hint. Learners read these next to the squiggle;
    /// keep them short, concrete and actionable.
    pub fn hint(self) -> &'static str {
        super::error_hint::hint(self)
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
    /// **解析诊断的原样身份**（N2 恢复式解析）：`Some` ⇒ [`CompileError::code`] /
    /// [`CompileError::hint`] 走它，不走 `kind`。
    ///
    /// 为什么要有这一格：恢复式解析把坏命令的诊断**挂进报告**（那是编辑器唯一看得见
    /// 的出口，`crates/lsp` 的 `report_diagnostics` 只读 `report.errors`），而报告里的
    /// 错误码来自 `ErrorKind`（静态、项目层，`entry-parse-failed` 那一族）—— 直接用它
    /// 就会让**同一条语法诊断**在两条通道上有两个身份：`grade --json` / `doc.code()`
    /// 报 `unexpected-token`（`Diagnostic::code()`），报告通道却报 `entry-parse-failed`
    /// ⇒ G-10/G-22 那一类"两条通道打架"。
    ///
    /// 所以这里存**原诊断的码与提示**（都是短字符串，`DiagnosticKind::code()` /
    /// `hint()` 的产物），`kind` 只留 [`ErrorKind::EntryParseFailed`] 的**阶段**
    /// （`stage() == Parse`）。
    ///
    /// `skip_serializing_if` 是**产物契约**：没有这一格的报告（今天的所有报告）
    /// 序列化逐字节不变 ⇒ 不进 `REPORT_SHAPE`；带这一格的报告必然含错误 ⇒
    /// 本来就进不了缓存（`ProjectReport::is_clean`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_code: Option<String>,
    /// 见 [`CompileError::parse_code`]：原诊断的**教学提示**（`Diagnostic::hint()`），
    /// 与码同进同出。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_hint: Option<String>,
}

impl CompileError {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind,
            expected: None,
            actual: None,
            parse_code: None,
            parse_hint: None,
        }
    }

    /// **解析诊断 ⇒ 报告错误**（N2 恢复式解析的唯一构造点）。
    ///
    /// 码与提示逐字取原诊断（[`Diagnostic::code`] / [`Diagnostic::hint`]）⇒
    /// `diagnostic_from_parse(diag)` 与 `diagnostic_from_compile(err)` 对**同一条**
    /// 语法错误给出**同一个** code / hint / message / span，只差它现在同时出现在
    /// 一份**有内容**的报告里（好命令的 hover/大纲因此不再被牵连）。
    pub fn from_parse(diag: &crate::Diagnostic) -> Self {
        Self {
            message: diag.message.clone(),
            span: diag.span,
            // 阶段 = Parse（`entry-parse-failed` 的 stage）；码/提示见上面两个字段。
            kind: ErrorKind::EntryParseFailed,
            expected: None,
            actual: None,
            parse_code: Some(diag.code().to_string()),
            parse_hint: Some(diag.hint().to_string()),
        }
    }

    pub(crate) fn elab(kind: ErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self::new(kind, message, span)
    }

    /// **A3（2026-10-12）**：`display` = 本次编译的**显示期记法表**（`None` ⇒ 只有
    /// 通用人话化，见下）。
    ///
    /// **G-49**：内核拒绝文本**一律**在这里过一层人话化 ✓。
    ///
    /// * def-eq 不匹配 ⇒ 走**唯一渲染器** [`render_def_eq_mismatch`]：
    ///   `rejected: def_eq failed: def_eq mismatch expected: Sort(1) | actual: $2` ✗
    ///   ⇒ `类型不匹配：期望 `Sort(1)`，实际是 `第 2 个绑元（α）`` ✓（`$4(β)` ⇒
    ///   「第 4 个绑元（β）」，名字来自**内核名字栈** ✓）；
    /// * 其它内核原文 ⇒ 只把裸 de Bruijn 编号换成占位 ✓（`humanize_de_bruijn`）。
    ///
    /// 为什么放在**构造点**：内核原文不止一条出口 —— `by` 路径
    /// （`judge.rs::judgement_of` 的 `_` 分支 / `query_error`）会把
    /// `rejected: def_eq failed: …` 原样抬给学习者 ✗（台账 G-49 的「同族」），
    /// 只有这里能一次盖全 ✓。
    /// ⚠ **分类必须在人话化之前**：`refine_kernel_kind` 由**调用方**在**原始**
    /// 文本上算 ✓（它认的就是 `$k` / `Sort(n)` 的形状 —— G-21 的
    /// `classify_term_in_type_position` ✓），构造点拿到的只是渲染 ✓。
    ///
    /// ⚠ **A3**：`display` 有值时两侧文本先过 [`render_both_sides`]（记法折叠到
    /// 源级 `∈`/`⊆`/`=` + 绑元点名 ✓）⇒ 中间**不再出现** `第 N 个绑元` 这种
    /// 内部形态占位 ✓；拿不到表（`by` 路径 / 反序列化回放）⇒ 行为与 A3 之前
    /// **逐字节相同** ✓（走 `humanize_de_bruijn` ✓）。
    pub(crate) fn kernel(
        kind: ErrorKind,
        message: impl Into<String>,
        span: Span,
        display: Option<&crate::display::DisplayNotations>,
    ) -> Self {
        let raw = message.into();
        let rendered = match render_def_eq_mismatch(&raw, display) {
            Some((message, _, _)) => message,
            None => humanize_de_bruijn(&raw),
        };
        Self::new(kind, rendered, span)
    }

    pub fn stage(&self) -> CompileStage {
        self.kind.stage()
    }

    /// Stable machine code (used by `--json` and the protocol).
    ///
    /// 解析诊断（[`CompileError::from_parse`]）给的是**原诊断的码**
    /// （`unexpected-token` 那一族），其余一律来自 `kind`。
    pub fn code(&self) -> &str {
        self.parse_code
            .as_deref()
            .unwrap_or_else(|| self.kind.code())
    }

    /// First teaching hint for this error.
    ///
    /// 解析诊断的提示与码同源（[`CompileError::parse_code`]）：同一个错误在
    /// 解析通道与报告通道必须说同一句话。
    pub fn hint(&self) -> &str {
        self.parse_hint
            .as_deref()
            .unwrap_or_else(|| self.kind.hint())
    }
}

/// Stable markers the kernel embeds in def-eq failure panics:
/// `def_eq mismatch expected: <E-TEXT> | actual: <A-TEXT>`
/// (the panic payload is wrapped into `rejected: ...` by `CheckError`).
pub(crate) const DEF_EQ_MARKER: &str = "def_eq mismatch expected: ";
pub(crate) const DEF_EQ_ACTUAL_SEP: &str = " | actual: ";

/// Parse the kernel's def-eq mismatch message into `(expected, actual)`.
/// Returns `None` for rejections that do not carry both sides.
/// **G-49**：把内核拒绝文本里的**裸 de Bruijn 编号**（`$4` ✓ = 内核内部编号 ✗）换成**人话** ✓。
///
/// 两种形态（名字由**内核的名字栈**给出 ✓，见 `crates/kernel/src/debug_printer.rs`
/// 的 `loose_binder_name` —— 名字栈与内核深度**严格对齐** ✓，前端拿不到这个对齐 ✗）：
/// * `$4(β)` ⇒ 「第 4 个绑元（β）」 ✓ —— **点名到人**（binder 的源级名字 ✓）；
/// * `$4` ⇒ 「第 4 个绑元」 ✓ —— 拿不到名字（匿名 binder / 名字栈与深度不齐）时的
///   **可读占位** ✓（de Bruijn 语义：从内往外数 ✓）。
///
/// ⚠ **只动文本** ✓：判定/事件计数/分类一律不变 ✓（`classify_prop_sort_gap` 那条
/// 明确不许顺手放宽 ✗ —— 见它自己的注释 ✓）；分类（`refine_kernel_kind`）在
/// **人话化之前**、用原始文本算 ✓（它认的就是 `$k` 的形状 ✓）。
pub(crate) fn humanize_de_bruijn(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                let idx = &text[i + 1..j];
                // `$k(名字)`：名字取到**配对括号**为止；名字里出现括号（理论上不会，
                // 名字是字母类记号）就**不认**这一形态 ⇒ 退化成占位 ✓（宁可少说 ✗）。
                if let Some((name, end)) = trailing_paren_name(text, j) {
                    out.push_str(&format!("第 {idx} 个绑元（{name}）"));
                    i = end;
                    continue;
                }
                out.push_str(&format!("第 {idx} 个绑元"));
                i = j;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or('\0');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `text[at..]` 形如 `(名字)` 时给出 `(名字, 名字之后的字节下标)`；否则 `None`。
/// 名字不许含括号、空白或 `$`（内核名字栈里的名字都是字母类记号 ✓）——
/// 宁可退化成占位，也不许把内核文本的别的东西误读成名字 ✗。
fn trailing_paren_name(text: &str, at: usize) -> Option<(String, usize)> {
    let rest = text.get(at..)?;
    let inner = rest.strip_prefix('(')?;
    let close = inner.find(')')?;
    let name = &inner[..close];
    if name.is_empty() || name.contains(['(', ')', ' ', '$', '\n']) {
        return None;
    }
    Some((name.to_string(), at + close + 2))
}

/// **G-49**：把内核的 def-eq 不匹配原文渲染成**教学文案** + 两侧文本。
///
/// 输出 = `("类型不匹配：期望 `X`，实际是 `Y`", X, Y)`。不是这个形状 ⇒ `None`
/// （调用方原样处理 ✓）。
///
/// **唯一实现**：声明位（`check/kernel_phase.rs` 两个构造点 ✓）、`by` 路径
/// （`judge.rs::query_error` ✓ —— 内核原文从前那里直接抬给学习者 ✗）与 G-21 的
/// 同形对照共用它，口径不会分叉 ✓。
///
/// **A3（2026-10-12）**：`display` 给得出显示期记法表时，两侧文本先过
/// [`render_both_sides`]（`Set.[]`/`$k` 这类内部形态 ⇒ 源级记法 + 点名 ✓）；
/// 给不出（`by` 路径）⇒ 与 A3 之前逐字节相同（[`humanize_de_bruijn`] ✓）。
pub(crate) fn render_def_eq_mismatch(
    raw: &str,
    display: Option<&crate::display::DisplayNotations>,
) -> Option<(String, String, String)> {
    let (expected, actual) = parse_def_eq_mismatch(raw)?;
    if source_kernel_msg_disabled() {
        let expected = humanize_de_bruijn(&expected);
        let actual = humanize_de_bruijn(&actual);
        let message = format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
        return Some((message, expected, actual));
    }
    let display = display?;
    let (expected, actual) = render_both_sides(&expected, &actual, display);
    let message = format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
    Some((message, expected, actual))
}

/// **A3 的关断开关**（`SOKO_NO_SOURCE_KERNEL_MSG=1`）：源级渲染整体退回
/// [`humanize_de_bruijn`]（「第 N 个绑元（α）」）。存在的意义与 `SOKO_NO_NOTATION_FOLD`
/// 同族 ✓ —— **反向验证**：关掉之后 r01/r15 两条探针**必须重新判红** ✓
/// （咬不住的守卫等于没有 ✓）。
pub(crate) fn source_kernel_msg_disabled() -> bool {
    std::env::var_os("SOKO_NO_SOURCE_KERNEL_MSG").is_some()
}

/// **A3 的渲染**：把内核**显式形态**的两侧类型文本变成**源级可读**形式。
///
/// 内核 def-eq 失败的原文是**内部形态**（`crates/kernel/src/debug_printer.rs` 的
/// `Debug` 实现 ✓）：`Pi (α : Sort(1)), Pi (A : (Set.[] $0(α))), …` —— 学习者
/// 看到的是一串 `Pi (… : …)`、`Set.[]`、`$4(α)`（前端渲染成「第 4 个绑元（α）」✗）
/// ⇒ **初学最常撞的那条通道完全读不懂**（长尾 ③ / r01 ✓）。这里做三件事，
/// **只动文本** ✓（判定、事件计数、错误码、分类一律不碰 ✗）：
///
/// 1. 把 `$k(名字)` / `$k` 换成**源级绑元名**（名字本来就在内核名字栈里 ✓，
///    G-49 已经把它带出来了 ✓）⇒ 不再需要「第 N 个绑元」这种占位 ✗；
/// 2. 去掉调试打印的宇宙实参后缀 `.[]`（`Set.[]` ⇒ `Set` ✓ —— 内核**有**
///    `.{1}` 形态（`Eq.{1}`），parser 认；空的 `.[]` 它不认 ✗）；
/// 3. 整段过**唯一记法接口** [`DisplayNotations::fold`] ⇒ `Set.mem α a A` 折成
///    `a ∈ A` ✓（`fold` 任何一步失败都原样返回 ✓ ⇒ 折不动时只是少折一点 ✓）。
///
/// ⚠ **不去标"分歧子串"**（试过 `⟨…⟩`，撤了 ✓）：分歧位置**由内核的选择决定**、
/// 不总是学习者眼里的那个 —— `Sort(1)` vs `Sort(0)` 会标在数字上，而
/// `∀(x : T), Sort(0)` vs `∀(_ : T), Sort(1)` 会标两处（`x`/`_` 与 `0`/`1`）⇒
/// 读起来像在说"绑元名写错了" ✗。两侧照内核原样给 ✓ —— 补上**源级记法**（`∈`）
/// 与**点名**（`α`/`a`）之后它已经可读 ✓。
pub(crate) fn render_both_sides(
    expected: &str,
    actual: &str,
    display: &crate::display::DisplayNotations,
) -> (String, String) {
    (
        render_one_side(expected, display),
        render_one_side(actual, display),
    )
}

/// 一侧：内核显式形态 ⇒ 源级文本（见 [`render_both_sides`] 的三步）。
fn render_one_side(text: &str, display: &crate::display::DisplayNotations) -> String {
    let (bound, names) = bind_vars_for_display(text);
    let normalized = pi_to_forall(&strip_empty_universe_suffix(&bound));
    let mut out = display.fold(&normalized);
    for (placeholder, name) in names {
        if placeholder != name {
            out = out.replace(&placeholder, &name);
        }
    }
    out
}

/// 调试打印里**空宇宙实参**的后缀：`Set.[]` ⇒ `Set`、`Set.mem.[]` ⇒ `Set.mem`。
///
/// 只删**空的**那一对 ✓：`Eq.{1}` 保留（parser 认这种形态 ✓）。`[{` 一起处理是
/// 因为层渲染的 `{}` 与 `[]` 之间**没有**分隔符（`LevelsPtr` 的 `Debug` 是
/// `debug_list` ⇒ `Eq.{1}` ✓、空表 ⇒ `Eq.[]` ✓）。
fn strip_empty_universe_suffix(text: &str) -> String {
    text.replace(".[]", "").replace(".{[]}", "")
}

/// 内核调试打印的 `Pi (名字 : 类型), 体` ⇒ 前端的 `∀ (名字 : 类型), 体` ✓。
///
/// **为什么要换**：`Pi` 是内核 `Expr` 变体的**调试名** ✗，前端 parser 只认
/// `∀`/`→`（实测 `Pi (α : Sort(1)), …` 报 `expected ) , found Colon` ✓）。
/// 逐 token 扫描、按**配对的括号**处理 ⇒ 嵌套 `Pi`（binder 的类型本身是 Pi ✓）
/// 与外层多个 `Pi` 都正确 ✓；**认不出就不动** ✓（宁可少折，不许切错 ✗）。
fn pi_to_forall(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if text[i..].starts_with("Pi (") && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            // `"Pi ("` 有 4 个字节 ⇒ 那个 `(` 在 `i + 3` ✓（不是 +2 ✗）。
            if let Some(close) = matching_paren(bytes, i + 3) {
                // `close` = `)` 的**字节**下标 ⇒ 切片要 `close + 1` 才**含**它 ✓
                // （`..=close` 是按字节的闭区间 ✗ —— α 这类多字节字符上会切在
                // 字符中间，实测直接掉进 `None` ⇒ 整条不转换 ✗）。
                if bytes.get(close + 1) == Some(&b',') {
                    out.push('∀');
                    out.push_str(&text[i + 3..close + 1]);
                    out.push(',');
                    i = close + 2;
                    continue;
                }
            }
        }
        let ch = text[i..].chars().next().unwrap_or('\0');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `text[open]` 是 `(` 时它**配对**的 `)` 的字节下标（`None` = 没配上 / 不是 `(`）。
pub(crate) fn matching_paren(bytes: &[u8], open: usize) -> Option<usize> {
    if bytes.get(open) != Some(&b'(') {
        return None;
    }
    let mut depth = 0usize;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// `$k(名字)` / `$k` ⇒ **能过 parser 的源级名** + 「占位 → 源名」表。
///
/// 名字直接来自内核名字栈（`crates/kernel/src/conv.rs` 的
/// `render_value_for_def_eq_error` 带的 ✓）—— **不是猜的** ✓。拿不到名字的
/// `$k`（匿名 binder / 名字栈与深度不齐）⇒ 保持原样 ✓（`humanize_de_bruijn`
/// 的语义：宁可少说 ✗），那种文本 parser 也过不去 ⇒ 调用方退回通用人话化 ✓。
fn bind_vars_for_display(text: &str) -> (String, Vec<(String, String)>) {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut names: Vec<(String, String)> = Vec::new();
    let mut next = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                let trailing = trailing_paren_name(text, j);
                if let Some((name, end)) = trailing {
                    let placeholder = format!("sokov{next}");
                    next += 1;
                    names.push((placeholder.clone(), name));
                    out.push_str(&placeholder);
                    i = end;
                    continue;
                }
                // 没有名字：保持 `$k`（`humanize_de_bruijn` 会把它变成占位，
                // 那条文本过不了 parser ⇒ 整体退回旧路径 ✓）。
                out.push_str(&text[i..j]);
                i = j;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or('\0');
        out.push(ch);
        i += ch.len_utf8();
    }
    (out, names)
}

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
/// Official Lean 4 has **no** universe cumulativity either (`Sort u =?= Sort v`
/// requires `u ≡ v`; `PLift` is how Lean lifts a `Prop` into a `Type`), so the
/// same source is rejected there too: this is a language property shared with
/// Lean, not a deviation. It still deserves a dedicated code plus a human hint
/// instead of the bare "类型不匹配" (2026-10-07 audit + Lean citations: ledger
/// L-06 and `docs/gaps/repro/L-06-lean-consistent-no-cumulativity.sh`).
///
/// **Scope (deliberate, documented in the ledger's L-06 `notes`): only the
/// bare-sort shape** — the mismatch between two *sorts* themselves
/// (`def T : Type := True`). The `Pi`-shaped variant
/// (`def bad : Prop -> Type := fun (x : Prop) => x`) is the same phenomenon,
/// but it is also the fixture that the CLI/LSP/extension contract tests use to
/// represent a **generic** kernel rejection (they pin `code ==
/// "kernel-rejected"`, stage, span and expected/actual). Widening the rule to
/// `Pi` shapes would change those fixtures, so it is left for a dedicated
/// migration rather than smuggled in here.
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

/// 一个**项**被放在了**类型**位：内核把出错那一侧渲染成本地变量（`$k`），
/// 而要类型的那一侧是 `Sort(n)`。
///
/// 这是「点名写法漏了前导类型实参」背后的形状——`Set.mem a A` 要写成
/// `Set.mem α a A`。内核在很晚才拒它，且消息里只有 `$2` 这类内部记号，
/// 学习者看不出缺什么。归类成 [`ErrorKind::KernelExpectedSort`]（语义就是
/// 「这里要类型」），由那边的提示负责把「漏了哪个参数」说出来。
fn classify_term_in_type_position(expected: &str, actual: &str) -> bool {
    actual.trim_start().starts_with('$') && trailing_sort_level(expected).is_some()
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
            // 项落在类型位（`Sort(n)` vs `$k`）：最常见的成因是点名调用漏了
            // 前导类型实参。归到 `KernelExpectedSort`，由它的提示指路。
            if classify_term_in_type_position(&expected, &actual) {
                return ErrorKind::KernelExpectedSort;
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
