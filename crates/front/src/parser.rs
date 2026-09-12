//! 递归下降解析器：tokens → AST（命令与表达式）。

use super::ast::{
    Binder, BinderKind, Command, CtorDecl, Expr, FolFile, IotaRule, RecDecl, SortKind, Tactic,
};
use super::diagnostic::{Diagnostic, DiagnosticKind, Result};
use super::span::Span;
use super::token::{tokenize, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

/// 一个 `(a b c : T)` 多名字 binder 组（读回内核 pp 类型文本时用）。
struct BinderGroup {
    names: Vec<String>,
    ty: Expr,
    style: BinderKind,
    span: Span,
}

/// 声明 binder 糖的降级（官方 Lean 语义）：类型拼成 Forall 望远镜、值包成
/// Lambda 望远镜，于是 open-goal/elab/kernel 全部复用既有路径；`:= sorry`
/// 的剩余目标直接是 codomain，上下文即声明 binder。空 binder 原样返回。
fn wrap_decl_binders(binders: Vec<Binder>, ty: Expr, val: Expr) -> (Expr, Expr) {
    let Some(first) = binders.first() else {
        return (ty, val);
    };
    let ty_span = Span::new(first.span.start, ty.span().end);
    let val_span = Span::new(first.span.start, val.span().end);
    let ty = Expr::Forall {
        binders: binders.clone(),
        body: Box::new(ty),
        span: ty_span,
    };
    let val = Expr::Lambda {
        binders,
        body: Box::new(val),
        span: val_span,
    };
    (ty, val)
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn parse_file(&mut self, src: &str) -> Result<FolFile> {
        let mut commands = Vec::new();
        while !self.at_eof() {
            commands.push(self.parse_command()?);
        }
        Ok(FolFile {
            commands,
            src: src.to_string(),
        })
    }

    fn parse_command(&mut self) -> Result<Command> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Ident(kw) if kw == "def" => self.parse_def(),
            TokenKind::Ident(kw) if kw == "theorem" => self.parse_theorem(),
            TokenKind::Ident(kw) if kw == "example" => self.parse_example(),
            TokenKind::Ident(kw) if kw == "axiom" => self.parse_axiom(),
            TokenKind::Ident(kw) if kw == "inductive" => self.parse_inductive_block(),
            TokenKind::Ident(kw) if kw == "#check" => self.parse_hash_check(),
            TokenKind::Ident(kw) if kw == "#reduce" => self.parse_hash_reduce(),
            TokenKind::Ident(kw) if kw == "#print" => self.parse_hash_print(),
            _ => {
                Err(self
                    .error_at_current(&format!("expected a .sokonanoda command, found {tok:?}")))
            }
        }
    }

    fn parse_def(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_ident("definition name")?;
        let universe = self.parse_universe_params()?;
        let binders = self.parse_decl_binders()?;
        self.expect_colon("definition type")?;
        let ty = self.parse_expr()?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_value()?;
        let span = Span::new(start, val.span().end);
        let (ty, val) = wrap_decl_binders(binders, ty, val);
        Ok(Command::Def {
            name,
            universe,
            ty,
            val,
            span,
        })
    }

    fn parse_theorem(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_ident("theorem name")?;
        let universe = self.parse_universe_params()?;
        let binders = self.parse_decl_binders()?;
        self.expect_colon("theorem statement")?;
        let ty = self.parse_expr()?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_value()?;
        let span = Span::new(start, val.span().end);
        let (ty, val) = wrap_decl_binders(binders, ty, val);
        Ok(Command::Theorem {
            name,
            universe,
            ty,
            val,
            span,
        })
    }

    fn parse_example(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let binders = self.parse_decl_binders()?;
        self.expect_colon("example type")?;
        let ty = self.parse_expr()?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_value()?;
        let span = Span::new(start, val.span().end);
        let (ty, val) = wrap_decl_binders(binders, ty, val);
        Ok(Command::Example { ty, val, span })
    }

    /// 声明级 binder（官方 Lean 风格）：`theorem f (a : A) (h : B a) : C := v`
    /// 里名字与冒号之间的 binder 组。解析后由 [`wrap_decl_binders`] 降级成
    /// 「类型 = Forall 望远镜；值 = Lambda 望远镜」，后续流水线零改动。
    fn parse_decl_binders(&mut self) -> Result<Vec<Binder>> {
        let mut binders = Vec::new();
        loop {
            match self.peek().kind {
                TokenKind::LParen | TokenKind::LBrace if self.named_group_ahead() => {
                    self.push_binders(&mut binders)?;
                }
                TokenKind::LParen | TokenKind::LBrace => {
                    return Err(self.error_here(
                        "声明 binder 需要显式类型，例如 (a : Prop)；不支持无类型的 (a) 写法",
                    ));
                }
                _ => break,
            }
        }
        Ok(binders)
    }

    /// 值位：普通表达式、`by <tactic 序列>` 块，或 `intro`（一次引入剩余
    /// 全部 binder 的教学关键字，与 `by` 同级）。
    fn parse_value(&mut self) -> Result<Expr> {
        if let TokenKind::Ident(kw) = &self.peek().kind {
            if kw == "by" {
                return self.parse_by_block();
            }
            if kw == "intro" {
                let tok = self.bump();
                return Ok(Expr::Intro { span: tok.span });
            }
        }
        self.parse_expr()
    }

    /// `by` 块：`by <tactic> (';' <tactic>)*`。tactic 之间用 `;` 分隔
    ///（教学子集不引入缩进敏感语法）。
    fn parse_by_block(&mut self) -> Result<Expr> {
        let by_tok = self.bump();
        let start = by_tok.span.start;
        let mut tactics = Vec::new();
        // 允许空 `by`（练习从零开始）：下一个 token 不是 tactic 关键字就收尾。
        if self.tactic_keyword_ahead() {
            loop {
                tactics.push(self.parse_tactic()?);
                match self.peek().kind {
                    TokenKind::Semicolon => {
                        self.bump();
                        continue;
                    }
                    _ => break,
                }
            }
        }
        // span 终点 = 最后一个 tactic 的终点（空 by = by 关键字终点）。
        // 绝不能用 `peek().span.start`：注释会被词法器跳过，下一个 token
        // 可能是下一个命令/EOF，会把中间的多行注释整个包进 span。
        let end = tactics
            .last()
            .map(|t| t.span().end)
            .unwrap_or(by_tok.span.end);
        let span = Span::new(start, end);
        Ok(Expr::By { tactics, span })
    }

    fn tactic_keyword_ahead(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Ident(ref kw)
                if matches!(kw.as_str(), "intro" | "exact" | "apply" | "assumption" | "rfl" | "sorry")
        )
    }

    fn parse_tactic(&mut self) -> Result<Tactic> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Ident(kw) if kw == "intro" => {
                self.bump();
                let name_tok = self.bump();
                let TokenKind::Ident(name) = &name_tok.kind else {
                    return Err(self.error_here("`intro` binder name"));
                };
                Ok(Tactic::Intro {
                    name: name.clone(),
                    span: Span::new(tok.span.start, name_tok.span.end),
                })
            }
            TokenKind::Ident(kw) if kw == "exact" => {
                self.bump();
                let expr = self.parse_expr()?;
                let end = expr.span().end;
                Ok(Tactic::Exact {
                    expr,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(kw) if kw == "apply" => {
                self.bump();
                let expr = self.parse_expr()?;
                let end = expr.span().end;
                Ok(Tactic::Apply {
                    expr,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(kw) if kw == "assumption" => {
                self.bump();
                Ok(Tactic::Assumption {
                    span: tok.span,
                })
            }
            TokenKind::Ident(kw) if kw == "rfl" => {
                self.bump();
                Ok(Tactic::Rfl { span: tok.span })
            }
            TokenKind::Ident(kw) if kw == "sorry" => {
                self.bump();
                Ok(Tactic::Sorry { span: tok.span })
            }
            _ => Err(self.error_here(&format!(
                "未知 tactic：`by` 块只支持 intro / exact / apply / assumption / rfl / sorry（白名单），发现 {tok:?}"
            ))),
        }
    }

    fn parse_axiom(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_ident("axiom name")?;
        let universe = self.parse_universe_params()?;
        self.expect_colon("axiom type")?;
        let ty = self.parse_expr()?;
        let span = Span::new(start, ty.span().end);
        Ok(Command::Axiom {
            name,
            universe,
            ty,
            span,
        })
    }

    /// `{u}` / `{u, v}`（只有名字、逗号分隔）是宇宙参数；`{a : Prop}` 是隐式
    /// binder 组。声明位两者都以 `{` 开头，用这个 lookahead 消歧。
    fn universe_params_ahead(&self) -> bool {
        let toks = &self.tokens;
        let mut i = self.cursor;
        if !matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::LBrace)) {
            return false;
        }
        i += 1;
        loop {
            if !matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
                return false;
            }
            i += 1;
            match toks.get(i).map(|t| &t.kind) {
                Some(TokenKind::Comma) => i += 1,
                Some(TokenKind::RBrace) => return true,
                _ => return false,
            }
        }
    }

    fn parse_universe_params(&mut self) -> Result<Vec<String>> {
        if !self.universe_params_ahead() {
            return Ok(Vec::new());
        }
        self.bump();
        let mut out = Vec::new();
        loop {
            let name = self.expect_ident("universe parameter")?;
            if out.contains(&name) {
                return Err(self.error_here(&format!("duplicate universe parameter `{name}`")));
            }
            out.push(name);
            match self.peek().kind {
                TokenKind::Comma => {
                    self.bump();
                }
                TokenKind::RBrace => {
                    self.bump();
                    return Ok(out);
                }
                _ => return Err(self.error_here("expected `,` or `}` in universe parameters")),
            }
        }
    }

    fn parse_inductive_block(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_ident("inductive name")?;
        self.expect_colon("inductive type")?;
        let ty = self.parse_expr()?;
        let mut constructors = Vec::new();
        let mut recursor = None;
        let mut iota_rules = Vec::new();
        loop {
            let kw = self.peek().clone();
            let TokenKind::Ident(word) = &kw.kind else {
                return Err(self.error_here("expected `ctor`, `rec`, `iota` or `end`"));
            };
            match word.as_str() {
                "end" => {
                    self.bump();
                    let span = Span::new(start, kw.span.end);
                    return Ok(Command::InductiveBlock {
                        name,
                        ty,
                        constructors,
                        recursor,
                        iota_rules,
                        span,
                    });
                }
                "ctor" => constructors.push(self.parse_ctor()?),
                "rec" => recursor = Some(self.parse_rec()?),
                "iota" => iota_rules.push(self.parse_iota()?),
                _ => return Err(self.error_here("expected `ctor`, `rec`, `iota` or `end`")),
            }
        }
    }

    fn parse_ctor(&mut self) -> Result<CtorDecl> {
        let start = self.bump().span.start;
        let name = self.expect_ident("constructor name")?;
        let mut binders = Vec::new();
        while self.peek().kind == TokenKind::LParen {
            binders.push(self.parse_binder()?);
        }
        self.expect_colon("constructor result type")?;
        let result = self.parse_expr()?;
        let span = Span::new(start, result.span().end);
        Ok(CtorDecl {
            name,
            binders,
            result,
            span,
        })
    }

    fn parse_rec(&mut self) -> Result<RecDecl> {
        let start = self.bump().span.start;
        let name = self.expect_ident("recursor name")?;
        let universe = self.parse_universe_params()?;
        self.expect_colon("recursor type")?;
        let ty = self.parse_expr()?;
        let span = Span::new(start, ty.span().end);
        Ok(RecDecl {
            name,
            universe,
            ty,
            span,
        })
    }

    fn parse_iota(&mut self) -> Result<IotaRule> {
        let start = self.bump().span.start;
        let ctor_name = self.expect_ident("constructor name")?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_expr()?;
        let span = Span::new(start, val.span().end);
        Ok(IotaRule {
            ctor_name,
            val,
            span,
        })
    }

    fn parse_hash_check(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let expr = self.parse_expr()?;
        let span = Span::new(start, expr.span().end);
        Ok(Command::Check { expr, span })
    }

    fn parse_hash_reduce(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let expr = self.parse_expr()?;
        let span = Span::new(start, expr.span().end);
        Ok(Command::Reduce { expr, span })
    }

    fn parse_hash_print(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_ident("declaration name")?;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(Command::Print {
            name,
            span: Span::new(start, end),
        })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        match &self.peek().kind {
            TokenKind::Forall => self.parse_forall(),
            TokenKind::Ident(kw) if kw == "fun" => self.parse_lambda(),
            _ => self.parse_arrow(),
        }
    }

    fn parse_arrow(&mut self) -> Result<Expr> {
        if self.named_group_ahead() {
            // `(a b c : T) -> body`：同型多名字 binder 组（读内核 pp 类型文本
            // 时需要，如 `forall (a b : Prop), ...`）。展开成逐名字的 Forall 链。
            let group = self.parse_binder_group()?;
            self.expect_kind(&TokenKind::Arrow, "`->` after binder group")?;
            let body = self.parse_expr()?;
            let start = group.span.start;
            let end = body.span().end;
            let style = group.style;
            let group_span = group.span;
            let mut expr = body;
            for name in group.names.into_iter().rev() {
                expr = Expr::Forall {
                    binders: vec![Binder {
                        name,
                        ty: Some(Box::new(group.ty.clone())),
                        style: style.clone(),
                        span: group_span,
                    }],
                    body: Box::new(expr),
                    span: Span::new(start, end),
                };
            }
            return Ok(expr);
        }
        let lhs = self.parse_plus()?;
        if self.peek().kind == TokenKind::Arrow {
            self.bump();
            let rhs = self.parse_arrow()?;
            let span = Span::new(lhs.span().start, rhs.span().end);
            return Ok(Expr::Arrow {
                domain: Box::new(lhs),
                codomain: Box::new(rhs),
                span,
            });
        }
        Ok(lhs)
    }

    /// `(a b : T)` 是否在箭头位（读回内核 pp 的多名字 binder 组）。
    fn named_group_ahead(&self) -> bool {
        let toks = &self.tokens;
        let mut i = self.cursor;
        if !matches!(
            toks.get(i).map(|t| &t.kind),
            Some(TokenKind::LParen) | Some(TokenKind::LBrace)
        ) {
            return false;
        }
        i += 1;
        let mut saw_ident = false;
        while matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
            saw_ident = true;
            i += 1;
        }
        saw_ident && matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::Colon))
    }

    /// 解析 `(a b c : T)` / `{a b c : T}`，消费括号并返回多名字 + 共享类型。
    fn parse_binder_group(&mut self) -> Result<BinderGroup> {
        let tok = self.bump().clone();
        let style = match tok.kind {
            TokenKind::LParen => BinderKind::Explicit,
            TokenKind::LBrace => BinderKind::Implicit,
            _ => {
                return Err(self.error_here("expected `(` or `{` to open a binder group"));
            }
        };
        let mut names = vec![self.expect_ident("binder name")?];
        while matches!(self.peek().kind, TokenKind::Ident(_)) {
            names.push(self.expect_ident("binder name")?);
        }
        self.expect_colon("binder type")?;
        let ty = self.parse_expr()?;
        let close = match style {
            BinderKind::Explicit => TokenKind::RParen,
            BinderKind::Implicit => TokenKind::RBrace,
        };
        let closing = self.peek().clone();
        if closing.kind != close {
            return Err(self.error_here("expected `)` to close binder group"));
        }
        self.bump();
        Ok(BinderGroup {
            names,
            ty,
            style,
            span: Span::new(tok.span.start, closing.span.end),
        })
    }

    fn parse_plus(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_app()?;
        while self.peek().kind == TokenKind::Plus {
            self.bump();
            let rhs = self.parse_app()?;
            let span = Span::new(lhs.span().start, rhs.span().end);
            lhs = Expr::Plus {
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_app(&mut self) -> Result<Expr> {
        let mut fun = self.parse_atom()?;
        while self.starts_atom() {
            let arg = self.parse_atom()?;
            let span = Span::new(fun.span().start, arg.span().end);
            fun = Expr::App {
                fun: Box::new(fun),
                arg: Box::new(arg),
                span,
            };
        }
        Ok(fun)
    }

    fn starts_atom(&self) -> bool {
        match &self.peek().kind {
            TokenKind::Ident(name) => !is_reserved_command(name),
            TokenKind::Num(_) | TokenKind::Hole | TokenKind::LParen | TokenKind::At => true,
            TokenKind::Forall => true,
            _ => false,
        }
    }

    fn parse_atom(&mut self) -> Result<Expr> {
        let tok = self.bump();
        match tok.kind {
            TokenKind::At => {
                let tok = self.bump();
                match tok.kind {
                    TokenKind::Ident(name) => self.finish_const(name, tok.span),
                    other => Err(Diagnostic::new(
                        DiagnosticKind::UnexpectedToken {
                            found: format!("{other:?}"),
                            expected: "a constant name after `@`".to_string(),
                        },
                        tok.span,
                        "expected a constant name after `@`".to_string(),
                    )),
                }
            }
            TokenKind::LParen => {
                let inner = self.parse_expr()?;
                self.expect_kind(&TokenKind::RParen, "`)`")?;
                Ok(inner)
            }
            TokenKind::Hole => Ok(Expr::Hole { span: tok.span }),
            TokenKind::Num(value) => Ok(Expr::Num {
                value,
                span: tok.span,
            }),
            TokenKind::Ident(name) if name == "Prop" => Ok(Expr::Sort {
                sort: SortKind::Prop,
                span: tok.span,
            }),
            // `sorry`（与官方 Lean 同义）：未完成证明的占位符，等价于 sorry。
            // 只在表达式位置拦截；声明名字走各自的语法路径不受影响。
            TokenKind::Ident(name) if name == "sorry" => Ok(Expr::Hole { span: tok.span }),
            TokenKind::Ident(name) if name == "Type" => {
                // `Type` 单独出现是 `Sort 1`；`Type n` 是 Lean 记法，等于
                // `Sort (n + 1)`（Lean 里 `Type u = Sort (u + 1)`）。
                let next_num = match &self.peek().kind {
                    TokenKind::Num(value) => Some(value.clone()),
                    _ => None,
                };
                if let Some(value) = next_num {
                    let level_tok = self.bump();
                    let n = value.parse::<u64>().map_err(|_| {
                        Diagnostic::new(
                            DiagnosticKind::UnexpectedToken {
                                found: value.clone(),
                                expected: "a universe level".to_string(),
                            },
                            level_tok.span,
                            "Type expects a universe level".to_string(),
                        )
                    })?;
                    let n = n.checked_add(1).ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticKind::UnexpectedToken {
                                found: value.clone(),
                                expected: "a universe level".to_string(),
                            },
                            level_tok.span,
                            "universe level is too large".to_string(),
                        )
                    })?;
                    return Ok(Expr::Sort {
                        sort: SortKind::Sort(n),
                        span: Span::new(tok.span.start, level_tok.span.end),
                    });
                }
                Ok(Expr::Sort {
                    sort: SortKind::Type,
                    span: tok.span,
                })
            }
            TokenKind::Ident(name) if name == "Sort" => {
                let level_tok = self.bump();
                let level = match level_tok.kind {
                    TokenKind::Num(value) => value.parse::<u64>().map_err(|_| {
                        Diagnostic::new(
                            DiagnosticKind::UnexpectedToken {
                                found: value,
                                expected: "a universe level".to_string(),
                            },
                            level_tok.span,
                            "Sort expects a universe level".to_string(),
                        )
                    })?,
                    TokenKind::Ident(name) => {
                        return Ok(Expr::Sort {
                            sort: SortKind::Level(name),
                            span: Span::new(tok.span.start, level_tok.span.end),
                        });
                    }
                    other => {
                        return Err(Diagnostic::new(
                            DiagnosticKind::UnexpectedToken {
                                found: format!("{other:?}"),
                                expected: "a universe level".to_string(),
                            },
                            level_tok.span,
                            "Sort expects a universe level".to_string(),
                        ));
                    }
                };
                Ok(Expr::Sort {
                    sort: SortKind::Sort(level),
                    span: Span::new(tok.span.start, level_tok.span.end),
                })
            }
            TokenKind::Ident(name) if name.ends_with('.') => self.finish_const(name, tok.span),
            TokenKind::Ident(name) if is_reserved_command(&name) => Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: name.clone(),
                    expected: "an expression".to_string(),
                },
                tok.span,
                format!("command keyword `{name}` cannot appear inside an expression"),
            )),
            TokenKind::Ident(name) => self.finish_const(name, tok.span),
            other => Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: "an expression".to_string(),
                },
                tok.span,
                format!("expected an expression, found {other:?}"),
            )),
        }
    }

    fn finish_const(&mut self, name: String, span: Span) -> Result<Expr> {
        if name.ends_with('.') {
            let base = name.trim_end_matches('.').to_string();
            if base.is_empty() {
                return Err(Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        found: name,
                        expected: "a constant name".to_string(),
                    },
                    span,
                    "expected a constant name before `.{...}`".to_string(),
                ));
            }
            if self.peek().kind == TokenKind::LBrace {
                self.bump();
                let mut levels = Vec::new();
                loop {
                    let level = self.bump();
                    match level.kind {
                        TokenKind::Ident(name) | TokenKind::Num(name) => levels.push(name),
                        other => {
                            return Err(Diagnostic::new(
                                DiagnosticKind::UnexpectedToken {
                                    found: format!("{other:?}"),
                                    expected: "a universe level".to_string(),
                                },
                                level.span,
                                "expected a universe level".to_string(),
                            ));
                        }
                    }
                    match self.peek().kind {
                        TokenKind::Comma => {
                            self.bump();
                        }
                        TokenKind::RBrace => {
                            self.bump();
                            break;
                        }
                        _ => {
                            return Err(
                                self.error_here("expected `,` or `}` in universe arguments")
                            );
                        }
                    }
                }
                let span = Span::new(span.start, self.tokens[self.cursor - 1].span.end);
                return Ok(Expr::UniverseApp {
                    name: base,
                    levels,
                    span,
                });
            }
            return Err(self.error_here("expected `{...}` after `.{...}` universe marker"));
        }
        Ok(Expr::Ident { name, span })
    }

    fn parse_lambda(&mut self) -> Result<Expr> {
        let start = self.bump().span.start;
        let mut binders = Vec::new();
        while self.peek().kind != TokenKind::FatArrow {
            self.push_binders(&mut binders)?;
        }
        self.expect_kind(&TokenKind::FatArrow, "`=>`")?;
        let body = self.parse_expr()?;
        let span = Span::new(start, body.span().end);
        Ok(Expr::Lambda {
            binders,
            body: Box::new(body),
            span,
        })
    }

    fn parse_forall(&mut self) -> Result<Expr> {
        let start = self.bump().span.start;
        let mut binders = Vec::new();
        loop {
            self.push_binders(&mut binders)?;
            if self.peek().kind == TokenKind::Comma {
                self.bump();
                break;
            }
            if self.peek().kind == TokenKind::Eof {
                return Err(self.error_here("unexpected end of file inside `∀` binders"));
            }
        }
        let body = self.parse_expr()?;
        let span = Span::new(start, body.span().end);
        Ok(Expr::Forall {
            binders,
            body: Box::new(body),
            span,
        })
    }

    /// 向 `binders` 追加一个 binder（单名 `(a : T)`）或展开一个多名字组
    /// `(a b : T)`（读回内核 pp 类型文本需要）。
    fn push_binders(&mut self, binders: &mut Vec<Binder>) -> Result<()> {
        if self.named_group_ahead() && self.multi_name_group_ahead() {
            let group = self.parse_binder_group()?;
            for name in group.names {
                binders.push(Binder {
                    name,
                    ty: Some(Box::new(group.ty.clone())),
                    style: group.style.clone(),
                    span: group.span,
                });
            }
            return Ok(());
        }
        binders.push(self.parse_binder()?);
        Ok(())
    }

    /// `named_group_ahead` 已确认是 binder 组；判断是否为多名字
    /// `(a b : T)`（单名走原 `parse_binder` 以保留既有 span 语义）。
    fn multi_name_group_ahead(&self) -> bool {
        let toks = &self.tokens;
        let mut i = self.cursor + 1;
        let mut count = 0;
        while matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
            count += 1;
            i += 1;
        }
        count > 1
    }

    fn parse_binder(&mut self) -> Result<Binder> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::LParen => {
                self.bump();
                let name = self.expect_ident("binder name")?;
                self.expect_colon("binder type")?;
                let ty = self.parse_expr()?;
                self.expect_kind(&TokenKind::RParen, "`)`")?;
                let end = self.tokens[self.cursor - 1].span.end;
                Ok(Binder {
                    name,
                    ty: Some(Box::new(ty)),
                    style: BinderKind::Explicit,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::LBrace => {
                self.bump();
                let name = self.expect_ident("binder name")?;
                self.expect_colon("binder type")?;
                let ty = self.parse_expr()?;
                self.expect_kind(&TokenKind::RBrace, "`}`")?;
                let end = self.tokens[self.cursor - 1].span.end;
                Ok(Binder {
                    name,
                    ty: Some(Box::new(ty)),
                    style: BinderKind::Implicit,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(name) => {
                self.bump();
                let mut ty = None;
                if self.peek().kind == TokenKind::Colon {
                    self.bump();
                    ty = Some(Box::new(self.parse_expr()?));
                }
                let end = ty.as_ref().map(|t| t.span().end).unwrap_or(tok.span.end);
                Ok(Binder {
                    name,
                    ty,
                    style: BinderKind::Explicit,
                    span: Span::new(tok.span.start, end),
                })
            }
            other => Err(self.error_here(&format!("expected a binder, found {other:?}"))),
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<String> {
        let tok = self.bump();
        match tok.kind {
            TokenKind::Ident(name) => Ok(name),
            other => Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: what.to_string(),
                },
                tok.span,
                format!("expected {what}, found {other:?}"),
            )),
        }
    }

    fn expect_colon(&mut self, what: &str) -> Result<()> {
        self.expect_kind(&TokenKind::Colon, what)
    }

    fn expect_kind(&mut self, kind: &TokenKind, what: &str) -> Result<()> {
        if &self.peek().kind == kind {
            self.bump();
            Ok(())
        } else {
            Err(self.error_here(&format!("expected {what}")))
        }
    }

    fn at_eof(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn bump(&mut self) -> Token {
        let tok = self.tokens[self.cursor].clone();
        if tok.kind != TokenKind::Eof {
            self.cursor += 1;
        }
        tok
    }

    fn error_here(&self, msg: &str) -> Diagnostic {
        let tok = self.peek();
        Diagnostic::new(
            DiagnosticKind::UnexpectedToken {
                found: format!("{:?}", tok.kind),
                expected: msg.to_string(),
            },
            tok.span,
            format!("{msg}, found {:?}", tok.kind),
        )
    }

    fn error_at_current(&self, msg: &str) -> Diagnostic {
        let tok = self.peek();
        Diagnostic::new(
            DiagnosticKind::UnexpectedToken {
                found: format!("{:?}", tok.kind),
                expected: "a command".to_string(),
            },
            tok.span,
            msg.to_string(),
        )
    }
}

pub fn parse(src: &str) -> Result<FolFile> {
    let tokens = tokenize(src)?;
    let mut parser = Parser::new(tokens);
    parser.parse_file(src)
}

fn is_reserved_command(name: &str) -> bool {
    matches!(
        name,
        "def"
            | "theorem"
            | "example"
            | "axiom"
            | "inductive"
            | "ctor"
            | "rec"
            | "iota"
            | "end"
            | "#check"
            | "#reduce"
            | "#print"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_def_check_and_hole() {
        let src = r#"
def id : Prop -> Prop := fun (x : Prop) => x
#check id
example : Prop -> Prop := sorry
"#;
        let file = parse(src).unwrap();
        assert_eq!(file.commands.len(), 3);
        assert!(matches!(&file.commands[0], Command::Def { name, .. } if name == "id"));
        assert!(matches!(&file.commands[1], Command::Check { .. }));
        assert!(
            matches!(&file.commands[2], Command::Example { val, .. } if matches!(val, Expr::Hole { .. }))
        );
    }

    #[test]
    fn value_intro_parses_as_the_intro_keyword() {
        let file = parse("theorem t : (a : Prop) -> a := intro\n").unwrap();
        assert!(matches!(
            &file.commands[0],
            Command::Theorem {
                val: Expr::Intro { .. },
                ..
            }
        ));
    }

    #[test]
    fn intro_with_a_trailing_name_is_a_parse_error() {
        assert!(parse("theorem t : (a : Prop) -> a := intro a\n").is_err());
    }

    #[test]
    fn dotted_intro_names_are_not_the_value_keyword() {
        // `And.intro` 是带点标识符；值位只认裸的 `intro`。
        let file = parse("def t : Prop -> Prop := And.intro\n").unwrap();
        assert!(matches!(
            &file.commands[0],
            Command::Def {
                val: Expr::Ident { name, .. },
                ..
            } if name == "And.intro"
        ));
    }

    #[test]
    fn decl_binders_desugar_to_forall_and_lambda() {
        let file = parse("theorem t (a : Prop) (b : Prop) : Prop := b\n").unwrap();
        let Command::Theorem { ty, val, .. } = &file.commands[0] else {
            panic!("expected theorem");
        };
        let Expr::Forall {
            binders: tbinders, ..
        } = ty
        else {
            panic!("expected Forall type, got {ty:?}");
        };
        assert_eq!(tbinders.len(), 2);
        assert_eq!(tbinders[0].name, "a");
        assert_eq!(tbinders[1].name, "b");
        let Expr::Lambda {
            binders: vbinders, ..
        } = val
        else {
            panic!("expected Lambda value, got {val:?}");
        };
        assert_eq!(vbinders.len(), 2);
    }

    #[test]
    fn untyped_decl_binder_is_a_parse_error() {
        let err = parse("theorem t (a) : Prop := Prop\n").unwrap_err();
        assert!(
            err.message.contains("显式类型"),
            "teaching message expected, got {err:?}"
        );
    }

    #[test]
    fn universe_params_before_decl_binders_parse() {
        let file = parse("def id {u} (A : Sort u) (x : A) : A := x\n").unwrap();
        let Command::Def { universe, ty, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        assert_eq!(universe, &vec!["u".to_string()]);
        let Expr::Forall { binders, .. } = ty else {
            panic!("expected Forall type, got {ty:?}");
        };
        assert_eq!(binders.len(), 2);
        assert_eq!(binders[0].name, "A");
    }

    #[test]
    fn example_with_decl_binders_parses() {
        let file = parse("example (a : Prop) : a -> a := fun (h : a) => h\n").unwrap();
        assert!(matches!(
            &file.commands[0],
            Command::Example {
                ty: Expr::Forall { binders, .. },
                ..
            } if binders.len() == 1
        ));
    }

    #[test]
    fn reports_diagnostic_with_span() {
        let err = parse("def broken : Prop :=\n").unwrap_err();
        assert_eq!(err.span.start.line, 2, "err: {err:?}");
    }

    #[test]
    fn type_with_level_parses_as_sort_succ() {
        // Lean 记法：`Type n` = `Sort (n + 1)`；单独的 `Type` 仍是 `Sort 1`。
        let file = parse("axiom T : Type 2\naxiom U : Type\n").unwrap();
        assert!(matches!(
            &file.commands[0],
            Command::Axiom {
                ty: Expr::Sort {
                    sort: SortKind::Sort(3),
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            &file.commands[1],
            Command::Axiom {
                ty: Expr::Sort {
                    sort: SortKind::Type,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn named_arrow_parses_as_forall_with_binder() {
        let file = parse("#check (x : Prop) -> x\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        let Expr::Forall { binders, body, .. } = expr else {
            panic!("expected Forall, got {expr:?}");
        };
        assert_eq!(binders.len(), 1);
        assert_eq!(binders[0].name, "x");
        assert_eq!(binders[0].style, BinderKind::Explicit);
        assert!(matches!(
            binders[0].ty.as_deref(),
            Some(Expr::Sort {
                sort: SortKind::Prop,
                ..
            })
        ));
        assert!(matches!(body.as_ref(), Expr::Ident { name, .. } if name == "x"));
    }

    #[test]
    fn implicit_named_arrow_parses_as_forall_with_implicit_binder() {
        let file = parse("#check {x : Prop} -> x\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        let Expr::Forall { binders, .. } = expr else {
            panic!("expected Forall, got {expr:?}");
        };
        assert_eq!(binders[0].name, "x");
        assert_eq!(binders[0].style, BinderKind::Implicit);
    }

    #[test]
    fn forall_with_comma_parses_binders_and_body() {
        let file = parse("#check forall (a : Prop), a\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        let Expr::Forall { binders, body, .. } = expr else {
            panic!("expected Forall, got {expr:?}");
        };
        assert_eq!(binders.len(), 1);
        assert_eq!(binders[0].name, "a");
        assert!(matches!(body.as_ref(), Expr::Ident { name, .. } if name == "a"));
    }

    #[test]
    fn def_parses_two_universe_params() {
        let file = parse("def f {u, v} : Prop := Prop\n").unwrap();
        let Command::Def { name, universe, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        assert_eq!(name, "f");
        assert_eq!(universe, &["u".to_string(), "v".to_string()]);
    }

    #[test]
    fn duplicate_universe_param_is_rejected() {
        let err = parse("def f {u, u} : Prop := Prop\n").unwrap_err();
        assert!(
            err.message.contains("duplicate universe parameter"),
            "err: {err:?}"
        );
    }

    #[test]
    fn inductive_block_parses_ctors_and_recursor() {
        let src = r#"
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} :
  (motive : (n : Nat) -> Sort u) ->
  (mz : motive zero) ->
  (ms : (n : Nat) -> motive n -> motive (succ n)) ->
  (n : Nat) -> motive n
end
"#;
        let file = parse(src).unwrap();
        assert_eq!(file.commands.len(), 1);
        let Command::InductiveBlock {
            name,
            ty,
            constructors,
            recursor,
            iota_rules,
            ..
        } = &file.commands[0]
        else {
            panic!("expected InductiveBlock");
        };
        assert_eq!(name, "Nat");
        assert!(matches!(
            ty,
            Expr::Sort {
                sort: SortKind::Type,
                ..
            }
        ));
        assert_eq!(constructors.len(), 2);
        assert_eq!(constructors[0].name, "zero");
        assert_eq!(constructors[0].binders.len(), 0);
        assert_eq!(constructors[1].name, "succ");
        assert_eq!(constructors[1].binders.len(), 1);
        assert_eq!(constructors[1].binders[0].name, "n");
        assert_eq!(recursor.as_ref().map(|r| r.name.as_str()), Some("Nat.rec"));
        assert_eq!(
            recursor.as_ref().map(|r| r.universe.clone()),
            Some(vec!["u".to_string()])
        );
        assert!(iota_rules.is_empty());
    }

    #[test]
    fn command_keyword_inside_expression_is_rejected() {
        let err = parse("def x : Prop := def\n").unwrap_err();
        assert!(err.message.contains("command keyword"), "err: {err:?}");
    }

    #[test]
    fn example_keeps_hole_in_value_position() {
        let file = parse("example : Prop -> Prop := sorry\n").unwrap();
        let Command::Example { ty, val, .. } = &file.commands[0] else {
            panic!("expected example");
        };
        assert!(matches!(ty, Expr::Arrow { .. }));
        assert!(matches!(val, Expr::Hole { .. }));
    }
}
