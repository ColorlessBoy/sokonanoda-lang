//! 语义着色的**词法前缀**与**名字表**（从 `semantic.rs` 拆出来，2026-10-16）。
//!
//! **为什么单独一个文件**：`semantic.rs` 在 `scripts/docs-lint.py` 判据 ⑨ 的**存量冻结**
//! 清单里（**只许减不许增** ✗）。本轮 `Expr::NamedArg`（用户 2026-10-16 第 ② 条）要往
//! `semantic.rs::walk_expr` 加一行 ⇒ 按 `prefix_recovery.rs` 的先例，把这一整段
//! （`lex_prefix` + `Names` + `collect_names` + `walk_tactic`/`walk_expr`/
//! `add_pattern_binders`）搬出来，原处只留 `use` ✓。
//!
//! **本模块不进判定路径** ✓：它只产语义 token（编辑器着色用）。

use std::collections::{HashMap, HashSet};

use crate::ast::{Binder, Command, Expr, FolFile};
use crate::semantic::SemanticKind;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// 词法错误总是报在出错 token 的起始 offset，所以对 `src[..err_offset]`
/// 重新词法一次即可得到“到目前为止”的全部 token。
pub(crate) fn lex_prefix(src: &str) -> Vec<Token> {
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
pub(crate) struct Names {
    /// 声明名 → 使用处的 kind（DefUse/TheoremUse/…）。
    pub(crate) decls: HashMap<String, SemanticKind>,
    pub(crate) ctors: HashSet<String>,
    /// v1 简化：binder 名按整个文件平铺，不做作用域感知；声明点的 span
    /// 记录在 `special` 里并优先于普通使用。
    pub(crate) binders: HashSet<String>,
    /// 声明点/构造子声明点/binder 声明点 token 的起始 offset → 该处的 kind。
    pub(crate) special: HashMap<usize, SemanticKind>,
    /// `Name.{…}` 宇宙应用的完整 span：其中除首名外的 level token 一律跳过。
    pub(crate) universes: Vec<Span>,
    /// 本文件已声明记法的**符号** → kind（G-04 / WO-011）。
    ///
    /// 已声明的符号归 [`SemanticKind::Keyword`]（先例：`∀` 的 `Forall` token）；
    /// **未声明**的符号不产 run（与 `->`/`=>` 一致）——所以 `⊢` 这类只出现在
    /// 内核渲染文本里的符号不会被染成「未知标识符」（设计 §3.4）。
    /// v1 **不新增** `SemanticKind`。
    pub(crate) notations: HashMap<String, SemanticKind>,
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

pub(crate) fn collect_names(file: &FolFile, toks: &[Token], names: &mut Names) {
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
            // `set_option` 的选项名与布尔值是命令参数、不是引用（同
            // `Command::Namespace` 的 `Foo`）⇒ 不产生任何高亮名。
            Command::SetOption { .. } => {}
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
        // `·` 聚焦块：里面的 tactic 都是真的 tactic，递归收它们的名字。
        Bullet { tactics, .. } => {
            for t in tactics {
                walk_tactic(t, toks, names);
            }
        }
        Assumption { .. }
        | Rfl { .. }
        | Constructor { .. }
        | Left { .. }
        | Right { .. }
        | Exfalso { .. }
        // `unfold` 的名字是**常量引用**（不是表达式位）⇒ 着色由词法通道管，
        // 这里不递归（与 `intro` 的名字同口径）。
        | Unfold { .. }
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
        // 命名实参（2026-10-16）：形参名 `α` 是**绑定位**（不是使用点）⇒ 只走值 ✓。
        Expr::NamedArg { value, .. } => walk_expr(value, toks, names),
    }
}

/// 把模式里「绑定变量」的位置登记进名字表（构造子名交给 ctors 表；点号名一律
/// 视为构造子）。设计原文 ⇒ `git log --all -- docs/design/match-patterns.md` §5。
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
