//! 语义着色（design doc F8）：把词法/语法信息分类成带 [`SemanticKind`] 的 token。
//!
//! front 拥有全部语言知识；`sokonanoda-lsp` 只负责把 [`SemanticKind`] 映射到
//! LSP 的 legend（`SemanticTokenType` 常量），不重复任何语言规则。
//!
//! 对编辑中的半成品文件保持健壮：
//! - 词法在文件中途出错：只分类已成功收集的 token，然后停止；
//! - 语法错误：退化为纯词法分类（不做标识符解析）。

use std::collections::{HashMap, HashSet};

use crate::ast::{Binder, Command, Expr, FolFile};
use crate::span::Span;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Keyword,
    Sort,
    Number,
    Hole,
    DefName,
    TheoremName,
    AxiomName,
    InductiveName,
    CtorName,
    Binder,
    DefUse,
    TheoremUse,
    AxiomUse,
    InductiveUse,
    CtorUse,
    UnknownIdent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticSpan {
    pub span: Span,
    pub kind: SemanticKind,
}

const KEYWORDS: &[&str] = &[
    "import",
    "def",
    // `abbrev` 是 `def` 的 Lean 拼写（同一个 parser 入口、同一条流水线；
    // 设计 `docs/design/abbrev.md`）。进词表 ⇒ 语义高亮与补全自动跟上。
    "abbrev",
    "theorem",
    "example",
    "axiom",
    "inductive",
    "ctor",
    "rec",
    "iota",
    "end",
    "fun",
    "let",
    "match",
    "with",
    "by",
    "infix",
    "infixl",
    "infixr",
    // 记法相关的新拼写（第二刀 `prefix`/`postfix`、第三刀
    // `binder_notation`/`scoped`）：与 `NOTATION_COMMANDS` 同集合，
    // 设计 `docs/design/notation-subset.md` §13.6。
    "prefix",
    "postfix",
    "notation",
    "binder_notation",
    "scoped",
    "intro",
    "exact",
    "apply",
    "assumption",
    "rfl",
    // 课程 Lean 化（设计 `docs/design/course-lean-style.md` L3）新增的 tactic：
    // 与 `is_tactic_keyword`（`parser.rs`）**同集合**——编辑器词表跟着走。
    "constructor",
    "left",
    "right",
    "use",
    "exfalso",
    // `cases`（L3.1）与 `have`（L3.6）此前**漏了**——`is_tactic_keyword`
    // 里有、词表里没有，于是编辑器里它们不着色也不补全（守护测试只保证
    // 「TM 语法 = KEYWORDS」，两边一起漏是看不见的）。
    "cases",
    "have",
    "#check",
    "#reduce",
    "#print",
];

const SORTS: &[&str] = &["Prop", "Type", "Sort"];

/// The language keywords (single source for semantic tokens and completions).
pub fn keywords() -> &'static [&'static str] {
    KEYWORDS
}

/// The sort spellings (`Prop`/`Type`/`Sort`) — completions material.
pub fn sorts() -> &'static [&'static str] {
    SORTS
}

impl SemanticKind {
    /// Every kind, in declaration order — the exhaustive source for tests and
    /// client look-up tables (wire names, colour classes).
    pub const ALL: &'static [SemanticKind] = &[
        SemanticKind::Keyword,
        SemanticKind::Sort,
        SemanticKind::Number,
        SemanticKind::Hole,
        SemanticKind::DefName,
        SemanticKind::TheoremName,
        SemanticKind::AxiomName,
        SemanticKind::InductiveName,
        SemanticKind::CtorName,
        SemanticKind::Binder,
        SemanticKind::DefUse,
        SemanticKind::TheoremUse,
        SemanticKind::AxiomUse,
        SemanticKind::InductiveUse,
        SemanticKind::CtorUse,
        SemanticKind::UnknownIdent,
    ];

    /// Canonical TextMate scope for this kind. Provenance for the editor's
    /// `.tmLanguage.json` (and the hover/executable fences): the grammar must
    /// contain a rule for every value here (test-locked to [`SemanticKind::ALL`]).
    pub fn tm_scope(self) -> &'static str {
        match self {
            SemanticKind::Keyword => "keyword.other.sokonanoda",
            SemanticKind::Sort | SemanticKind::InductiveName | SemanticKind::InductiveUse => {
                "storage.type.sokonanoda"
            }
            SemanticKind::Number => "constant.numeric.sokonanoda",
            SemanticKind::Hole => "markup.inserted.sokonanoda",
            SemanticKind::DefName
            | SemanticKind::DefUse
            | SemanticKind::TheoremName
            | SemanticKind::TheoremUse => "entity.name.function.sokonanoda",
            SemanticKind::AxiomName
            | SemanticKind::AxiomUse
            | SemanticKind::CtorName
            | SemanticKind::CtorUse => "entity.name.type.sokonanoda",
            SemanticKind::Binder => "variable.parameter.sokonanoda",
            SemanticKind::UnknownIdent => "variable.other.sokonanoda",
        }
    }

    /// Stable lowercase wire name (single source for `soko/*` tagged runs and
    /// any other client that colours from [`SemanticKind`]).
    pub fn as_str(self) -> &'static str {
        match self {
            SemanticKind::Keyword => "keyword",
            SemanticKind::Sort => "sort",
            SemanticKind::Number => "number",
            SemanticKind::Hole => "hole",
            SemanticKind::DefName => "def_name",
            SemanticKind::TheoremName => "theorem_name",
            SemanticKind::AxiomName => "axiom_name",
            SemanticKind::InductiveName => "inductive_name",
            SemanticKind::CtorName => "ctor_name",
            SemanticKind::Binder => "binder",
            SemanticKind::DefUse => "def_use",
            SemanticKind::TheoremUse => "theorem_use",
            SemanticKind::AxiomUse => "axiom_use",
            SemanticKind::InductiveUse => "inductive_use",
            SemanticKind::CtorUse => "ctor_use",
            SemanticKind::UnknownIdent => "unknown_ident",
        }
    }
}

/// One renderable fragment of a text: its literal source plus the semantic
/// kind (`None` = whitespace/punctuation, drawn plain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub kind: Option<SemanticKind>,
}

/// The declaration-name table of `src` (name → use-kind, constructors as
/// [`SemanticKind::CtorUse`]) — the input [`tag_runs`] needs to colour a
/// goal/hypothesis type that lives outside the document's own spans.
pub fn declaration_kinds(src: &str) -> Vec<(String, SemanticKind)> {
    let Ok(file) = crate::parse(src) else {
        return Vec::new();
    };
    let mut out: Vec<(String, SemanticKind)> = Vec::new();
    for cmd in crate::ast::effective_commands(&file) {
        match cmd {
            Command::Def { name, .. } => out.push((name.clone(), SemanticKind::DefUse)),
            Command::Theorem { name, .. } => out.push((name.clone(), SemanticKind::TheoremUse)),
            Command::Axiom { name, .. } => out.push((name.clone(), SemanticKind::AxiomUse)),
            Command::InductiveBlock {
                name, constructors, ..
            } => {
                out.push((name.clone(), SemanticKind::InductiveUse));
                for ctor in constructors {
                    // G-02：内核渲染的目标文本里构造子是**规范名**（`Prod.mk`），
                    // 所以高亮表必须收规范名；**源名也收**（R3：源级写法继续按
                    // 源名，学员的 `| none =>` 与 `ctor none` 都还要着色）。
                    out.push((ctor.name.clone(), SemanticKind::CtorUse));
                    let canonical = crate::compile::canonical_ctor_name(name, &ctor.name);
                    if canonical != ctor.name {
                        out.push((canonical, SemanticKind::CtorUse));
                    }
                }
            }
            Command::Example { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. }
            | Command::Import { .. } => {}
            // G-05：三条作用域命令不声明名字，也没有要着色的表达式
            // （`namespace Foo` 的 `Foo` 是**命令参数**，不是引用）。
            Command::Namespace { .. } | Command::End { .. } | Command::Open { .. } => {}
            // 记法命令不声明名字（设计 N6）：`declaration_kinds` 只服务
            // goal/hypothesis 文本的着色，符号本身在那里不出现。
            Command::Notation { .. } => {}
            // 第二刀 §N7：`open … in <命令>` 自己不是声明——它包住的那条命令
            // 由 `effective_commands` 展开后**单独**走到这里（所以这里的
            // `inner` 不会被漏掉，也不会被数两次）。
            Command::OpenIn { .. } | Command::Export { .. } => {}
        }
    }
    out
}

/// Classify an arbitrary expression text (a kernel-rendered goal or hypothesis
/// type) into renderable [`Run`]s, using the very same rules as the editor's
/// semantic tokens — the single source of language knowledge
/// (`docs/design/goal-rendering.md` §2.1).
///
/// `decls` = visible declarations (see [`declaration_kinds`]); `binders` =
/// names in scope at that goal (hypotheses). Lexing failures degrade to one
/// plain run, never an error.
pub fn tag_runs(text: &str, decls: &[(String, SemanticKind)], binders: &[String]) -> Vec<Run> {
    let plain = |s: &str| Run {
        text: s.to_string(),
        kind: None,
    };
    let mut names = Names::default();
    for (name, kind) in decls {
        names.decls.insert(name.clone(), *kind);
    }
    for name in binders {
        names.binders.insert(name.clone());
    }
    let toks = match crate::token::tokenize(text) {
        Ok(mut toks) => {
            toks.pop(); // Eof
            toks
        }
        Err(_) => return vec![plain(text)],
    };
    let mut runs = Vec::new();
    let mut cursor = 0usize;
    for tok in &toks {
        if tok.span.start.offset > cursor {
            runs.push(plain(&text[cursor..tok.span.start.offset]));
        }
        let kind = match &tok.kind {
            TokenKind::Ident(name) => Some(classify_ident(name, tok.span.start.offset, &names)),
            TokenKind::Num(_) => Some(SemanticKind::Number),
            TokenKind::Hole => Some(SemanticKind::Hole),
            TokenKind::Forall => Some(SemanticKind::Keyword),
            // 记法符号：已声明 → `Keyword`，未声明 → 不产 run（与 `->`/`=>`
            // 一致）。内核渲染的目标文本里可能出现 `⊢`（U+22A2，落在数学符号
            // 码点类里），它必须保持 plain run（设计 §3.4）。
            TokenKind::Sym(symbol) => names.notations.get(symbol).copied(),
            TokenKind::Str(_) => None,
            _ => None,
        };
        runs.push(Run {
            text: text[tok.span.start.offset..tok.span.end.offset].to_string(),
            kind,
        });
        cursor = tok.span.end.offset;
    }
    if cursor < text.len() {
        runs.push(plain(&text[cursor..]));
    }
    runs
}

/// [`tag_runs`] with the declaration table pulled from `file_src` — the LSP's
/// call shape (it holds the document text, not a name table).
pub fn tag_expr(text: &str, file_src: &str, binders: &[String]) -> Vec<Run> {
    tag_runs(text, &declaration_kinds(file_src), binders)
}

/// Plain-text projection of runs: concatenate their `text`. This is the bridge
/// that keeps hover fence content identical to the Infoview's run rendering —
/// the projection of a block's runs *is* the block text, so the hover and the
/// `tok-*` spans can never drift (`docs/design/goal-rendering.md` §2.1).
pub fn runs_to_text(runs: &[Run]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

/// Canonical goal-state text: `name : ty` per hypothesis (in order), then
/// `⊢ goal`. The one line model shared by the hover's `sokonanoda` fence and
/// the Infoview, so both render the same content.
pub fn goal_text(binders: &[(String, String)], goal: &str) -> String {
    let mut out = String::new();
    for (name, ty) in binders {
        out.push_str(name);
        out.push_str(" : ");
        out.push_str(ty);
        out.push('\n');
    }
    out.push_str("⊢ ");
    out.push_str(goal);
    out
}

/// [`goal_text`] tagged with [`tag_runs`], with the hypothesis names as the
/// binders in scope. The `⊢` turnstile is not a `.sokonanoda` token, so each
/// hypothesis line and the goal are tagged separately and the turnstile is a
/// plain run. Invariant: `runs_to_text(&goal_runs(..)) == goal_text(..)`, so a
/// caller renders the block either as a `sokonanoda` fence (hover) or as
/// `tok-*` spans (Infoview) without a second content producer.
pub fn goal_runs(
    binders: &[(String, String)],
    goal: &str,
    decls: &[(String, SemanticKind)],
) -> Vec<Run> {
    let names: Vec<String> = binders.iter().map(|(name, _)| name.clone()).collect();
    let hyps: String = binders
        .iter()
        .map(|(name, ty)| format!("{name} : {ty}\n"))
        .collect();
    let mut runs = tag_runs(&hyps, decls, &names);
    runs.push(Run {
        text: "⊢ ".to_string(),
        kind: None,
    });
    runs.extend(tag_runs(goal, decls, &names));
    runs
}

/// 词法收集到的 token；出错时只保留出错点之前的干净前缀（丢弃 Eof）。
///
/// 词法错误总是报在出错 token 的起始 offset，所以对 `src[..err_offset]`
/// 重新词法一次即可得到“到目前为止”的全部 token。
fn lex_prefix(src: &str) -> Vec<Token> {
    match crate::token::tokenize(src) {
        Ok(mut toks) => {
            toks.pop(); // Eof
            toks
        }
        Err(diag) => {
            let cut = diag.span.start.offset.min(src.len());
            if !src.is_char_boundary(cut) {
                return Vec::new();
            }
            match crate::token::tokenize(&src[..cut]) {
                Ok(mut toks) => {
                    toks.pop(); // Eof
                    toks
                }
                Err(_) => Vec::new(),
            }
        }
    }
}

/// 名字表：声明名/构造子名/binder 名（按文件平铺）+ 声明点 span + 宇宙应用 span。
#[derive(Default)]
struct Names {
    /// 声明名 → 使用处的 kind（DefUse/TheoremUse/…）。
    decls: HashMap<String, SemanticKind>,
    ctors: HashSet<String>,
    /// v1 简化：binder 名按整个文件平铺，不做作用域感知；声明点的 span
    /// 记录在 `special` 里并优先于普通使用。
    binders: HashSet<String>,
    /// 声明点/构造子声明点/binder 声明点 token 的起始 offset → 该处的 kind。
    special: HashMap<usize, SemanticKind>,
    /// `Name.{…}` 宇宙应用的完整 span：其中除首名外的 level token 一律跳过。
    universes: Vec<Span>,
    /// 本文件已声明记法的**符号** → kind（G-04 / WO-011）。
    ///
    /// 已声明的符号归 [`SemanticKind::Keyword`]（先例：`∀` 的 `Forall` token）；
    /// **未声明**的符号不产 run（与 `->`/`=>` 一致）——所以 `⊢` 这类只出现在
    /// 内核渲染文本里的符号不会被染成「未知标识符」（设计 §3.4）。
    /// v1 **不新增** `SemanticKind`。
    notations: HashMap<String, SemanticKind>,
}

impl Names {
    /// 在 `after`（不含）与 `until`（含）之间找到名字 token，登记声明点 span。
    /// 声明/构造子声明的 span 从命令关键字开始，所以用严格 `>` 跳过关键字本身。
    fn add_special(
        &mut self,
        toks: &[Token],
        name: &str,
        after: usize,
        until: usize,
        kind: SemanticKind,
    ) {
        if let Some(tok) = toks.iter().find(|t| {
            t.span.start.offset > after
                && t.span.end.offset <= until
                && matches!(&t.kind, TokenKind::Ident(s) if s == name)
        }) {
            self.special.insert(tok.span.start.offset, kind);
        }
    }

    /// binder 的 span 从 `(`/`{` 或名字本身开始，所以用 `>=`。
    fn add_binder(&mut self, toks: &[Token], binder: &Binder) {
        self.binders.insert(binder.name.clone());
        if let Some(tok) = toks.iter().find(|t| {
            t.span.start.offset >= binder.span.start.offset
                && t.span.end.offset <= binder.span.end.offset
                && matches!(&t.kind, TokenKind::Ident(s) if *s == binder.name)
        }) {
            self.special
                .insert(tok.span.start.offset, SemanticKind::Binder);
        }
    }
}

fn collect_names(file: &FolFile, toks: &[Token], names: &mut Names) {
    for cmd in crate::ast::effective_commands(file) {
        match cmd {
            // `import` 不声明名字、也没有表达式要着色（模块名 token 落在
            // 未知标识符的默认样式里）。
            Command::Import { .. } => {}
            Command::Def {
                name,
                ty,
                val,
                span,
                ..
            } => {
                names.decls.insert(name.clone(), SemanticKind::DefUse);
                names.add_special(
                    toks,
                    name,
                    span.start.offset,
                    span.end.offset,
                    SemanticKind::DefName,
                );
                walk_expr(ty, toks, names);
                walk_expr(val, toks, names);
            }
            Command::Theorem {
                name,
                ty,
                val,
                span,
                ..
            } => {
                names.decls.insert(name.clone(), SemanticKind::TheoremUse);
                names.add_special(
                    toks,
                    name,
                    span.start.offset,
                    span.end.offset,
                    SemanticKind::TheoremName,
                );
                walk_expr(ty, toks, names);
                walk_expr(val, toks, names);
            }
            Command::Example { ty, val, .. } => {
                walk_expr(ty, toks, names);
                walk_expr(val, toks, names);
            }
            Command::Axiom { name, ty, span, .. } => {
                names.decls.insert(name.clone(), SemanticKind::AxiomUse);
                names.add_special(
                    toks,
                    name,
                    span.start.offset,
                    span.end.offset,
                    SemanticKind::AxiomName,
                );
                walk_expr(ty, toks, names);
            }
            Command::InductiveBlock {
                name,
                params,
                ty,
                constructors,
                recursor,
                iota_rules,
                span,
            } => {
                names.decls.insert(name.clone(), SemanticKind::InductiveUse);
                names.add_special(
                    toks,
                    name,
                    span.start.offset,
                    span.end.offset,
                    SemanticKind::InductiveName,
                );
                for param in params {
                    names.add_binder(toks, param);
                    if let Some(param_ty) = param.ty.as_deref() {
                        walk_expr(param_ty, toks, names);
                    }
                }
                walk_expr(ty, toks, names);
                for ctor in constructors {
                    // 源名（`mk`，R3 的源级写法）与规范名（`Wrap.mk`，R1）都
                    // 归 `ctor_use`（protocol 词汇不变）。
                    names.ctors.insert(ctor.name.clone());
                    names
                        .ctors
                        .insert(crate::compile::canonical_ctor_name(name, &ctor.name));
                    names.add_special(
                        toks,
                        &ctor.name,
                        ctor.span.start.offset,
                        ctor.span.end.offset,
                        SemanticKind::CtorName,
                    );
                    for binder in &ctor.binders {
                        names.add_binder(toks, binder);
                    }
                    walk_expr(&ctor.result, toks, names);
                }
                if let Some(rec) = recursor {
                    walk_expr(&rec.ty, toks, names);
                }
                for rule in iota_rules {
                    walk_expr(&rule.val, toks, names);
                }
            }
            Command::Check { expr, .. } | Command::Reduce { expr, .. } => {
                walk_expr(expr, toks, names)
            }
            Command::Print { .. } => {}
            // 记法命令（G-04 / WO-011）：登记**符号**（符号本身归 `Keyword`，
            // 与 `∀` 的 `Forall` token 同族）。目标名不在这里登记——它是**使用**
            // 点，`names.decls` 已由目标自己的声明命令填好（未知目标就该落
            // `UnknownIdent`，与点名写法同判）。
            // **不新增 `SemanticKind`**（设计 §4：`ALL` 与 `tm_scope` 表逐字不变）。
            Command::Notation { symbol, .. } => {
                names
                    .notations
                    .insert(symbol.clone(), SemanticKind::Keyword);
            }
            // G-05：三条作用域命令不声明名字、没有表达式要着色（命令参数
            // `Foo` 不是引用）——与 `Command::Import` 同族。
            Command::Namespace { .. } | Command::End { .. } | Command::Open { .. } => {}
            // 第二刀 §N7：同 `declaration_kinds`——`OpenIn` 包住的命令由
            // `effective_commands` 展开后单独着色。
            Command::OpenIn { .. } | Command::Export { .. } => {}
        }
    }
}

/// 走一个 tactic 里的表达式：`exact`/`apply`/`use` 的项，以及 `cases` 的
/// 被消去项与**各臂的 tactic 递归**（臂体是嵌套的 tactic 序列）。
fn walk_tactic(tactic: &crate::Tactic, toks: &[Token], names: &mut Names) {
    use crate::Tactic::*;
    match tactic {
        Intro { .. } => {}
        Exact { expr, .. } | Apply { expr, .. } | Use { expr, .. } => walk_expr(expr, toks, names),
        Cases { expr, arms, .. } => {
            walk_expr(expr, toks, names);
            for arm in arms {
                for t in &arm.tactics {
                    walk_tactic(t, toks, names);
                }
            }
        }
        // `have h : T := t` / `:= by …`：类型与值都是表达式，嵌套块递归。
        Have { ty, value, .. } => {
            walk_expr(ty, toks, names);
            match value {
                crate::ast::HaveValue::Term(expr) => walk_expr(expr, toks, names),
                crate::ast::HaveValue::By(tactics) => {
                    for t in tactics {
                        walk_tactic(t, toks, names);
                    }
                }
            }
        }
        Assumption { .. }
        | Rfl { .. }
        | Constructor { .. }
        | Left { .. }
        | Right { .. }
        | Exfalso { .. }
        | Sorry { .. } => {}
    }
}

fn walk_expr(expr: &Expr, toks: &[Token], names: &mut Names) {
    match expr {
        Expr::Sort { .. } | Expr::Ident { .. } | Expr::Num { .. } | Expr::Hole { .. } => {}
        Expr::UniverseApp { span, .. } => names.universes.push(*span),
        Expr::App { fun, arg, .. } => {
            walk_expr(fun, toks, names);
            walk_expr(arg, toks, names);
        }
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            for binder in binders {
                names.add_binder(toks, binder);
            }
            walk_expr(body, toks, names);
        }
        Expr::Arrow {
            domain, codomain, ..
        }
        | Expr::Plus {
            lhs: domain,
            rhs: codomain,
            ..
        } => {
            walk_expr(domain, toks, names);
            walk_expr(codomain, toks, names);
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            // binder 类型与值在外层 scope；x 只对 body 可见。
            if let Some(ty) = binder.ty.as_deref() {
                walk_expr(ty, toks, names);
            }
            walk_expr(val, toks, names);
            names.add_binder(toks, binder);
            walk_expr(body, toks, names);
        }
        Expr::By { tactics, .. } => {
            for tactic in tactics {
                walk_tactic(tactic, toks, names);
            }
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            walk_expr(scrutinee, toks, names);
            for arm in arms {
                add_pattern_binders(&arm.pattern, toks, names);
                if let Some(guard) = &arm.guard {
                    walk_expr(guard, toks, names);
                }
                walk_expr(&arm.body, toks, names);
            }
        }
        // 记号节点（G-04 / WO-011）：操作数照常着色；符号本身由 `classify`
        // 按「已声明 → Keyword」处理（不在这里登记，因为 `walk_expr` 可能被
        // 内核渲染文本的 tag_runs 走到，那里没有文件级的记法表）。
        Expr::Notation { lhs, rhs, .. } => {
            if let Some(lhs) = lhs {
                walk_expr(lhs, toks, names);
            }
            if let Some(rhs) = rhs {
                walk_expr(rhs, toks, names);
            }
        }
        // 集合字面量（第三刀 §12.4）：元素照常着色；`{`/`}`/`,` 是标点。
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            for element in elements {
                walk_expr(element, toks, names);
            }
        }
    }
}

/// 把模式里「绑定变量」的位置登记进名字表（构造子名交给 ctors 表；点号名一律
/// 视为构造子）。设计 `docs/design/match-patterns.md` §5。
fn add_pattern_binders(pat: &crate::ast::Pattern, toks: &[Token], names: &mut Names) {
    match pat {
        crate::ast::Pattern::Wild { .. } | crate::ast::Pattern::Num { .. } => {}
        crate::ast::Pattern::Ident { name, args, span } => {
            if args.is_empty() {
                // 裸名：可能是绑定，也可能是 0 元构造子（ctor 表优先）。
                if name.contains('.') || names.ctors.contains(name) {
                    return;
                }
                let binder = Binder {
                    name: name.clone(),
                    ty: None,
                    style: crate::BinderKind::Explicit,
                    span: *span,
                };
                names.add_binder(toks, &binder);
            } else {
                for arg in args {
                    add_pattern_binders(arg, toks, names);
                }
            }
        }
    }
}

/// token 是否落在某个宇宙应用 span 内部（首名 token 的起点等于 span 起点，不受影响）。
fn inside_universe(names: &Names, tok: &Token) -> bool {
    names
        .universes
        .iter()
        .any(|u| tok.span.start.offset > u.start.offset && tok.span.end.offset <= u.end.offset)
}

fn classify_universe_base(base: &str, names: &Names) -> SemanticKind {
    if let Some(kind) = names.decls.get(base) {
        return *kind;
    }
    if names.ctors.contains(base) {
        return SemanticKind::CtorUse;
    }
    SemanticKind::UnknownIdent
}

fn classify_ident(text: &str, offset: usize, names: &Names) -> SemanticKind {
    // 声明点 span 优先于一切使用侧分类（含关键字/内核已定义名字同名等边角情况）。
    if let Some(kind) = names.special.get(&offset) {
        return *kind;
    }
    // `sorry` 是未完成证明的占位符（与 sorry 等价），高亮同 sorry。
    if text == "sorry" {
        return SemanticKind::Hole;
    }
    if text.ends_with('.') {
        return classify_universe_base(text.trim_end_matches('.'), names);
    }
    if KEYWORDS.contains(&text) {
        return SemanticKind::Keyword;
    }
    if SORTS.contains(&text) {
        return SemanticKind::Sort;
    }
    if let Some(kind) = names.decls.get(text) {
        return *kind;
    }
    if names.ctors.contains(text) {
        return SemanticKind::CtorUse;
    }
    if names.binders.contains(text) {
        return SemanticKind::Binder;
    }
    SemanticKind::UnknownIdent
}

fn classify(toks: &[Token], names: Option<&Names>) -> Vec<SemanticSpan> {
    let empty = Names::default();
    let names = names.unwrap_or(&empty);
    let mut out = Vec::new();
    for tok in toks {
        if inside_universe(names, tok) {
            continue;
        }
        let kind = match &tok.kind {
            TokenKind::Ident(text) => classify_ident(text, tok.span.start.offset, names),
            TokenKind::Num(_) => SemanticKind::Number,
            TokenKind::Hole => SemanticKind::Hole,
            TokenKind::Forall => SemanticKind::Keyword,
            // 记法符号（G-04 / WO-011）：**已声明** → `Keyword`（与 `∀` 同族）；
            // 未声明 → 不产 run（与 `->`/`=>` 一致）。`⊢` 这类内核渲染文本里的
            // 符号因此不会被染成「未知标识符」（设计 §3.4）。
            TokenKind::Sym(symbol) => match names.notations.get(symbol) {
                Some(kind) => *kind,
                None => continue,
            },
            // 字符串字面量只出现在记法命令里，不进语义 token 流
            // （`keyword.other` 由 `infix`/`notation` 关键字本身给出）。
            TokenKind::Str(_) => continue,
            _ => continue,
        };
        out.push(SemanticSpan {
            span: tok.span,
            kind,
        });
    }
    out.sort_by_key(|s| s.span.start.offset);
    dedup_overlaps(&mut out);
    out
}

fn dedup_overlaps(out: &mut Vec<SemanticSpan>) {
    let mut kept: Vec<SemanticSpan> = Vec::with_capacity(out.len());
    for s in out.drain(..) {
        if kept
            .last()
            .is_some_and(|last| s.span.start.offset < last.span.end.offset)
        {
            continue;
        }
        kept.push(s);
    }
    *out = kept;
}

/// 语义 token 主入口：词法 + 解析 + AST 名字解析；任何一处失败都优雅退化。
pub fn semantic_tokens(src: &str) -> Vec<SemanticSpan> {
    let toks = lex_prefix(src);
    match crate::parse(src) {
        Ok(file) => {
            let mut names = Names::default();
            collect_names(&file, &toks, &mut names);
            classify(&toks, Some(&names))
        }
        Err(_) => classify(&toks, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 第一个源码切片等于 `text` 的语义 token。
    fn find<'a>(src: &'a str, spans: &'a [SemanticSpan], text: &str) -> &'a SemanticSpan {
        spans
            .iter()
            .find(|s| &src[s.span.start.offset..s.span.end.offset] == text)
            .unwrap_or_else(|| panic!("no semantic token with text `{text}` in {spans:?}"))
    }

    /// 源码切片等于 `text` 的所有 token 的 kind（按出现顺序）。
    fn kinds_of(src: &str, spans: &[SemanticSpan], text: &str) -> Vec<SemanticKind> {
        spans
            .iter()
            .filter(|s| &src[s.span.start.offset..s.span.end.offset] == text)
            .map(|s| s.kind)
            .collect()
    }

    /// 归为 `kind` 的所有 token 的源码切片。
    fn texts_of<'a>(src: &'a str, spans: &'a [SemanticSpan], kind: SemanticKind) -> Vec<&'a str> {
        spans
            .iter()
            .filter(|s| s.kind == kind)
            .map(|s| &src[s.span.start.offset..s.span.end.offset])
            .collect()
    }

    #[test]
    fn empty_source_yields_no_tokens() {
        assert!(semantic_tokens("").is_empty());
        assert!(semantic_tokens("-- only a comment\n").is_empty());
    }

    #[test]
    fn classifies_def_example_hole_number_sort() {
        let src = "def two : Nat := 2\nexample : Sort 1 := sorry\n";
        let spans = semantic_tokens(src);
        assert_eq!(find(src, &spans, "def").kind, SemanticKind::Keyword);
        assert_eq!(find(src, &spans, "two").kind, SemanticKind::DefName);
        assert_eq!(find(src, &spans, "Nat").kind, SemanticKind::UnknownIdent);
        assert_eq!(find(src, &spans, "2").kind, SemanticKind::Number);
        assert_eq!(find(src, &spans, "example").kind, SemanticKind::Keyword);
        assert_eq!(find(src, &spans, "Sort").kind, SemanticKind::Sort);
        assert_eq!(find(src, &spans, "1").kind, SemanticKind::Number);
        assert_eq!(find(src, &spans, "sorry").kind, SemanticKind::Hole);
        // 有序且互不重叠
        for pair in spans.windows(2) {
            assert!(pair[0].span.start.offset < pair[1].span.start.offset);
            assert!(pair[0].span.end.offset <= pair[1].span.start.offset);
        }
    }

    #[test]
    fn keyword_classification_covers_intro_and_apply_tactics() {
        // by 块的关键字（`by`/`intro`/`exact`/`apply`/`assumption`/`rfl`）都要
        // 着成 Keyword——用户报告 `exact` 不高亮（分类位置无关）。
        let src = "theorem u : (a : Prop) -> a := by intro a; exact a\n\
                   theorem v : (a : Prop) -> a -> a := by intro a; intro h; assumption\n";
        let spans = semantic_tokens(src);
        for kw in ["by", "intro", "exact", "assumption"] {
            let kinds = kinds_of(src, &spans, kw);
            assert!(
                !kinds.is_empty() && kinds.iter().all(|k| *k == SemanticKind::Keyword),
                "`{kw}` must be a keyword, got {kinds:?}"
            );
        }
    }

    #[test]
    fn keyword_table_covers_abbrev_and_the_notation_spellings() {
        // 同步项（设计 `docs/design/abbrev.md` §4、`notation-subset.md` §13.6）：
        // `abbrev`（= `def` 的 Lean 拼写）与记法相关的新拼写——第二刀的
        // `prefix`/`postfix`、第三刀的 `binder_notation`/`scoped`——必须进
        // **单一词表** `front::semantic::KEYWORDS`。TM 语法与它逐字相等
        // （守护：`crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source`），
        // 所以两份词表只能同一轮改。
        for keyword in ["abbrev", "prefix", "postfix", "binder_notation", "scoped"] {
            assert!(
                keywords().contains(&keyword),
                "`{keyword}` must be in the single keyword table: {KEYWORDS:?}"
            );
        }
        let src = "abbrev A : Type := Prop\n\
                   prefix:100 \" ι \" => A\n\
                   postfix:100 \" ᶜ \" => A\n\
                   binder_notation \" ∃ \" => A\n\
                   namespace Foo\n\
                   scoped infix:50 \" ⊕ \" => A\n\
                   end Foo\n";
        let spans = semantic_tokens(src);
        for keyword in ["abbrev", "prefix", "postfix", "binder_notation", "scoped"] {
            let kinds = kinds_of(src, &spans, keyword);
            assert!(
                !kinds.is_empty() && kinds.iter().all(|k| *k == SemanticKind::Keyword),
                "`{keyword}` must be classified as Keyword, got {kinds:?}"
            );
        }
    }

    #[test]
    fn decl_site_vs_use_site() {
        let src = "def id : Prop -> Prop := id\n#check id\n";
        let spans = semantic_tokens(src);
        assert_eq!(
            kinds_of(src, &spans, "id"),
            vec![
                SemanticKind::DefName,
                SemanticKind::DefUse,
                SemanticKind::DefUse
            ]
        );
        assert_eq!(find(src, &spans, "#check").kind, SemanticKind::Keyword);
        assert_eq!(
            texts_of(src, &spans, SemanticKind::Sort),
            vec!["Prop", "Prop"]
        );
    }

    #[test]
    fn theorem_and_axiom_kinds() {
        let src = "theorem t : Prop := Prop\naxiom a : Prop\n#check t\n#check a\n";
        let spans = semantic_tokens(src);
        assert_eq!(
            kinds_of(src, &spans, "t"),
            vec![SemanticKind::TheoremName, SemanticKind::TheoremUse]
        );
        assert_eq!(
            kinds_of(src, &spans, "a"),
            vec![SemanticKind::AxiomName, SemanticKind::AxiomUse]
        );
    }

    #[test]
    fn inductive_ctor_decl_and_use() {
        let src = "\
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} :
  (motive : (n : Nat) -> Sort u) ->
  (mz : motive zero) ->
  (ms : (n : Nat) -> motive n -> motive (succ n)) ->
  (n : Nat) -> motive n
end
#check succ
";
        let spans = semantic_tokens(src);
        assert_eq!(find(src, &spans, "Nat").kind, SemanticKind::InductiveName);
        // zero/succ：声明一次，recursor 体内与 `#check` 中各使用一次。
        assert_eq!(
            kinds_of(src, &spans, "zero"),
            vec![SemanticKind::CtorName, SemanticKind::CtorUse]
        );
        assert_eq!(
            kinds_of(src, &spans, "succ"),
            vec![
                SemanticKind::CtorName,
                SemanticKind::CtorUse,
                SemanticKind::CtorUse
            ]
        );
        // binder 声明点 + 使用处都按文件平铺归为 Binder（v1 不做作用域感知），
        // 所以 recursor 类型里的 motive/mz/ms 也算：
        // ctor n | motive n | mz motive | ms n motive n motive n | n motive n
        assert_eq!(
            texts_of(src, &spans, SemanticKind::Binder),
            vec![
                "n", "motive", "n", "mz", "motive", "ms", "n", "motive", "n", "motive", "n", "n",
                "motive", "n"
            ]
        );
        assert!(spans.iter().any(|s| s.kind == SemanticKind::Keyword
            && &src[s.span.start.offset..s.span.end.offset] == "end"));
    }

    #[test]
    fn binder_map_is_flat_per_file() {
        let src = "def k : Prop -> Prop := fun (x : Prop) => x\n";
        let spans = semantic_tokens(src);
        assert_eq!(texts_of(src, &spans, SemanticKind::Binder), vec!["x", "x"]);
        assert_eq!(find(src, &spans, "fun").kind, SemanticKind::Keyword);
    }

    #[test]
    fn universe_app_levels_are_skipped() {
        let src = "def id {w} : Sort w -> Sort w := fun (x : Sort w) => x\n#check id.{w}\n";
        let spans = semantic_tokens(src);
        // `id.` 作为宇宙应用名 → DefUse；`id.{w}` 大括号里的 level 不产生 token。
        assert_eq!(find(src, &spans, "id.").kind, SemanticKind::DefUse);
        assert_eq!(texts_of(src, &spans, SemanticKind::Binder), vec!["x", "x"]);
        // 剩下的 `w`：def 头的宇宙参数 1 个 + `Sort w` 3 个，都是 UnknownIdent。
        assert_eq!(
            kinds_of(src, &spans, "w"),
            vec![SemanticKind::UnknownIdent; 4]
        );
    }

    #[test]
    fn lex_error_keeps_collected_prefix_without_resolution() {
        let src = "def x : Prop := ?\n";
        let spans = semantic_tokens(src);
        assert_eq!(find(src, &spans, "def").kind, SemanticKind::Keyword);
        assert_eq!(find(src, &spans, "x").kind, SemanticKind::UnknownIdent);
        assert_eq!(find(src, &spans, "Prop").kind, SemanticKind::Sort);
        assert!(
            !spans.iter().any(|s| s.kind == SemanticKind::DefName),
            "lexical-only fallback must not resolve names"
        );
    }

    #[test]
    fn parse_error_falls_back_to_lexical_only() {
        let src = "def broken : Prop :=\n";
        let spans = semantic_tokens(src);
        assert_eq!(find(src, &spans, "def").kind, SemanticKind::Keyword);
        assert_eq!(find(src, &spans, "broken").kind, SemanticKind::UnknownIdent);
        assert_eq!(find(src, &spans, "Prop").kind, SemanticKind::Sort);
    }

    #[test]
    fn forall_is_keyword_and_non_ascii_identifiers_resolve() {
        let src = "def 🦀x : Prop := Prop\n#check 🦀x\n";
        let spans = semantic_tokens(src);
        assert_eq!(
            kinds_of(src, &spans, "🦀x"),
            vec![SemanticKind::DefName, SemanticKind::DefUse]
        );

        let src = "#check forall (a : Prop), a\n#check ∀ (b : Prop), b\n";
        let spans = semantic_tokens(src);
        assert_eq!(kinds_of(src, &spans, "forall"), vec![SemanticKind::Keyword]);
        assert_eq!(kinds_of(src, &spans, "∀"), vec![SemanticKind::Keyword]);
        assert_eq!(
            kinds_of(src, &spans, "a"),
            vec![SemanticKind::Binder, SemanticKind::Binder]
        );
        assert_eq!(
            kinds_of(src, &spans, "b"),
            vec![SemanticKind::Binder, SemanticKind::Binder]
        );
    }

    // ---- tag_runs / tag_expr: the shared goal-rendering classification ----

    /// `(text, kind)` view of runs, dropping the plain connector fragments.
    fn tagged(runs: &[Run]) -> Vec<(&str, SemanticKind)> {
        runs.iter()
            .filter_map(|r| r.kind.map(|k| (r.text.as_str(), k)))
            .collect()
    }

    #[test]
    fn tag_runs_covers_the_whole_text_in_order() {
        let word = vec![("P".to_string(), SemanticKind::DefUse)];
        for text in ["P -> Nat", "fun (x : Nat) => x", "Sort 1", "sorry -> P"] {
            let runs = tag_runs(text, &word, &[]);
            assert_eq!(
                runs.iter().map(|r| r.text.as_str()).collect::<String>(),
                text,
                "runs must reconstruct {text:?} exactly"
            );
        }
    }

    #[test]
    fn tag_runs_classifies_like_the_editor() {
        // `P` is a file declaration (DefUse), `x` is in scope (Binder),
        // `Nat`/`Q` are unknown to this table — same rules as the editor.
        let decls = vec![("P".to_string(), SemanticKind::DefUse)];
        let binders = vec!["x".to_string()];
        let text = "fun (x : Nat) => P -> Sort 2";
        let runs = tag_runs(text, &decls, &binders);
        assert_eq!(
            tagged(&runs),
            vec![
                ("fun", SemanticKind::Keyword),
                ("x", SemanticKind::Binder),
                ("Nat", SemanticKind::UnknownIdent),
                ("P", SemanticKind::DefUse),
                ("Sort", SemanticKind::Sort),
                ("2", SemanticKind::Number),
            ]
        );
        let runs = tag_runs("sorry -> sorry", &decls, &binders);
        assert_eq!(
            tagged(&runs),
            vec![("sorry", SemanticKind::Hole), ("sorry", SemanticKind::Hole),]
        );
    }

    #[test]
    fn tag_expr_resolves_declarations_and_constructors_from_the_file() {
        // A goal type rendered by the kernel has no spans of its own, so the
        // declaration table comes from the document text.
        let src = "inductive Nat : Type\nctor zero : Nat\nctor succ (n : Nat) : Nat\nend\n";
        let runs = tag_expr("succ zero", src, &[]);
        assert_eq!(
            tagged(&runs),
            vec![
                ("succ", SemanticKind::CtorUse),
                ("zero", SemanticKind::CtorUse),
            ]
        );
    }

    /// G-02 / WO-005：内核渲染的目标文本里构造子是**规范名**（`Wrap.mk`），
    /// 而源里写的是裸名；两种拼写都要归到 `ctor_use`（protocol 词汇不变）。
    #[test]
    fn tag_expr_classifies_canonical_and_bare_ctor_spellings() {
        let src = "inductive Wrap : Type\nctor mk : Wrap\nend\n";
        for spelling in ["Wrap.mk", "mk"] {
            let runs = tag_expr(spelling, src, &[]);
            assert_eq!(
                tagged(&runs),
                vec![(spelling, SemanticKind::CtorUse)],
                "`{spelling}` must be a ctor use"
            );
        }
        // 点号 token 也要能被 lexer 当成一个 ident（`Wrap.mk` 是一个 token）。
        assert_eq!(
            semantic_tokens("inductive Wrap : Type\nctor mk : Wrap\nend\n")
                .iter()
                .filter(|s| s.kind == SemanticKind::CtorName)
                .count(),
            1,
            "the ctor declaration name keeps its own kind"
        );
    }

    #[test]
    fn tag_runs_degrades_on_a_lex_error_instead_of_panicking() {
        let runs = tag_runs("$", &[], &[]);
        assert_eq!(
            runs,
            vec![Run {
                text: "$".to_string(),
                kind: None,
            }]
        );
    }

    // ---- tm_scope / runs_to_text / goal_runs: the canonical kind→scope
    // table and the one goal-block content producer ----

    #[test]
    fn tm_scope_is_total() {
        // Exact scope per kind, in `ALL` declaration order — locks the table so
        // a new kind cannot silently land in the wrong TextMate group.
        let expected = [
            "keyword.other.sokonanoda",
            "storage.type.sokonanoda",
            "constant.numeric.sokonanoda",
            "markup.inserted.sokonanoda",
            "entity.name.function.sokonanoda",
            "entity.name.function.sokonanoda",
            "entity.name.type.sokonanoda",
            "storage.type.sokonanoda",
            "entity.name.type.sokonanoda",
            "variable.parameter.sokonanoda",
            "entity.name.function.sokonanoda",
            "entity.name.function.sokonanoda",
            "entity.name.type.sokonanoda",
            "storage.type.sokonanoda",
            "entity.name.type.sokonanoda",
            "variable.other.sokonanoda",
        ];
        let got: Vec<&str> = SemanticKind::ALL.iter().map(|k| k.tm_scope()).collect();
        assert_eq!(got, expected, "tm_scope table drifted from ALL");
        // Total and namespaced: every kind has a non-empty `.sokonanoda` scope.
        for (kind, scope) in SemanticKind::ALL.iter().zip(&got) {
            assert!(!scope.is_empty(), "{kind:?} has an empty scope");
            assert!(
                scope.ends_with(".sokonanoda"),
                "{kind:?} scope is not namespaced: {scope:?}"
            );
        }
        // Unique where expected: the exact set of distinct scopes is the
        // documented 8 groups (no accidental cross-group sharing).
        let mut unique = got.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 8, "distinct tm_scope count: {unique:?}");
    }

    #[test]
    fn runs_to_text_round_trips() {
        // The projection of `tag_runs`'s output is exactly its input.
        let decls = vec![("P".to_string(), SemanticKind::DefUse)];
        let binders = vec!["x".to_string()];
        for text in ["P -> Nat", "fun (x : Nat) => x", "Sort 1", "sorry -> P"] {
            let runs = tag_runs(text, &decls, &binders);
            assert_eq!(runs_to_text(&runs), text, "round trip of {text:?}");
        }
    }

    #[test]
    fn goal_runs_projects_to_hypothesis_lines_then_turnstile() {
        let binders = vec![
            ("a".to_string(), "Prop".to_string()),
            ("h".to_string(), "And a a".to_string()),
        ];
        let text = goal_text(&binders, "a");
        assert_eq!(text, "a : Prop\nh : And a a\n⊢ a");
        let decls = vec![("And".to_string(), SemanticKind::AxiomUse)];
        let runs = goal_runs(&binders, "a", &decls);
        // The invariant that keeps hover and Infoview identical: the runs'
        // text projection is the plain block text.
        assert_eq!(runs_to_text(&runs), text);
        // Same classification rules as the editor/wire: hypothesis names are
        // binders (and so is the goal's `a`), `Prop` is a sort, `And` is the
        // declared axiom.
        let tagged = tagged(&runs);
        assert!(tagged
            .iter()
            .any(|(t, k)| *t == "a" && *k == SemanticKind::Binder));
        assert!(tagged
            .iter()
            .any(|(t, k)| *t == "Prop" && *k == SemanticKind::Sort));
        assert!(tagged
            .iter()
            .any(|(t, k)| *t == "And" && *k == SemanticKind::AxiomUse));
        // The turnstile stays a plain run (it is not a source token).
        assert!(runs.iter().any(|r| r.kind.is_none() && r.text == "⊢ "));
    }

    #[test]
    fn goal_runs_with_no_hypotheses_is_just_the_turnstile_goal() {
        let runs = goal_runs(&[], "P -> P", &[]);
        assert_eq!(runs_to_text(&runs), "⊢ P -> P");
    }

    // ---- 记法（G-04 / WO-011，设计 §4）：符号归 Keyword、未声明不产 run ----

    #[test]
    fn a_declared_notation_symbol_is_a_keyword() {
        let src = "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
                   infix:50 \" ∈ \" => mem\n\
                   def p (a : Prop) (A : Prop) : Prop := a ∈ A\n";
        let spans = semantic_tokens(src);
        // 使用点（`a ∈ A`）的符号是 `Sym` token ⇒ Keyword（与 `∀` 同族）。
        // `kinds_of` 按**源码切片**匹配，所以它也会命中声明行字符串字面量里的
        // 那一个 `∈`——那里是 `Str` token 的内部，`classify` 按 offset 切出的
        // 是 `UnknownIdent`（`Str` 不产 run，但它的字节仍落在文本里）。
        // 这里只钉**使用点**（`Sym` token）的分类。
        let use_offset = src.rfind("a ∈ A").expect("use site") + 2;
        let use_span = spans
            .iter()
            .find(|s| s.span.start.offset == use_offset)
            .expect("a run for the used symbol");
        assert_eq!(use_span.kind, SemanticKind::Keyword);
        // `infix` 是关键字。
        assert_eq!(find(src, &spans, "infix").kind, SemanticKind::Keyword);
        // 目标名 `mem` 在记法命令里仍是它的 def_use。
        assert_eq!(
            kinds_of(src, &spans, "mem"),
            vec![SemanticKind::DefName, SemanticKind::DefUse]
        );
    }

    #[test]
    fn an_undeclared_symbol_produces_no_semantic_run() {
        // `⊢`（U+22A2）落在数学符号码点类里，但它只是**内核渲染文本**里的
        // turnstile：未声明 ⇒ 不产 run（与 `->`/`=>` 一致）。否则 goal 面板
        // 会被染成「未知标识符」（设计 §3.4）。
        let src = "def p : Prop := Prop\n";
        let spans = semantic_tokens(src);
        assert!(
            spans
                .iter()
                .all(|s| &src[s.span.start.offset..s.span.end.offset] != "⊢"),
            "the turnstile must not become a semantic run: {spans:?}"
        );
        // 同一个 token 直接分类时也不产 run（`a` 仍是 UnknownIdent——那是
        // 既有行为，与本刀无关；这里只钉 turnstile）。
        let tagged = tag_runs("⊢ a", &[], &[]);
        assert_eq!(
            tagged.first(),
            Some(&Run {
                text: "⊢".to_string(),
                kind: None,
            }),
            "tag_runs must leave the turnstile plain: {tagged:?}"
        );
        assert_eq!(runs_to_text(&tagged), "⊢ a");
    }

    #[test]
    fn notation_does_not_add_a_semantic_kind() {
        // v1 **不新增** `SemanticKind`：锁表逐字不变（设计 §4）。
        assert_eq!(SemanticKind::ALL.len(), 16);
    }

    #[test]
    fn semantic_kind_wire_names_are_unique() {
        let mut names: Vec<&str> = SemanticKind::ALL.iter().map(|k| k.as_str()).collect();
        assert!(names
            .iter()
            .all(|n| n.chars().all(|c| c.is_ascii_lowercase() || c == '_')));
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            SemanticKind::ALL.len(),
            "wire names must be unique"
        );
    }
}
