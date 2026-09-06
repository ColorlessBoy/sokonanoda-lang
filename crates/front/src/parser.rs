//! 递归下降解析器：tokens → AST（命令与表达式）。

use super::ast::{
    Binder, BinderKind, Command, CtorDecl, Expr, FolFile, IotaRule, RecDecl, SortKind,
};
use super::diagnostic::{Diagnostic, DiagnosticKind, Result};
use super::span::Span;
use super::token::{tokenize, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn parse_file(&mut self) -> Result<FolFile> {
        let mut commands = Vec::new();
        while !self.at_eof() {
            commands.push(self.parse_command()?);
        }
        Ok(FolFile { commands })
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
        self.expect_colon("definition type")?;
        let ty = self.parse_expr()?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_expr()?;
        let span = Span::new(start, val.span().end);
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
        self.expect_colon("theorem statement")?;
        let ty = self.parse_expr()?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_expr()?;
        let span = Span::new(start, val.span().end);
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
        self.expect_colon("example type")?;
        let ty = self.parse_expr()?;
        self.expect_kind(&TokenKind::ColonEq, "`:=`")?;
        let val = self.parse_expr()?;
        let span = Span::new(start, val.span().end);
        Ok(Command::Example { ty, val, span })
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

    fn parse_universe_params(&mut self) -> Result<Vec<String>> {
        if self.peek().kind != TokenKind::LBrace {
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
        if self.named_arrow_ahead() {
            let binder = self.parse_binder()?;
            self.expect_kind(&TokenKind::Arrow, "`->` after binder")?;
            let body = self.parse_expr()?;
            let span = Span::new(binder.span.start, body.span().end);
            return Ok(Expr::Forall {
                binders: vec![binder],
                body: Box::new(body),
                span,
            });
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

    fn named_arrow_ahead(&self) -> bool {
        let open = self.tokens.get(self.cursor).map(|t| &t.kind);
        if !matches!(open, Some(TokenKind::LParen) | Some(TokenKind::LBrace)) {
            return false;
        }
        matches!(
            self.tokens.get(self.cursor + 1).map(|t| &t.kind),
            Some(TokenKind::Ident(_))
        ) && matches!(
            self.tokens.get(self.cursor + 2).map(|t| &t.kind),
            Some(TokenKind::Colon)
        )
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
            TokenKind::Ident(name) if name == "Type" => Ok(Expr::Sort {
                sort: SortKind::Type,
                span: tok.span,
            }),
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
            binders.push(self.parse_binder()?);
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
            binders.push(self.parse_binder()?);
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
    parser.parse_file()
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
example : Prop -> Prop := ???
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
    fn reports_diagnostic_with_span() {
        let err = parse("def broken : Prop :=\n").unwrap_err();
        assert_eq!(err.span.start.line, 2, "err: {err:?}");
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
        let file = parse("example : Prop -> Prop := ???\n").unwrap();
        let Command::Example { ty, val, .. } = &file.commands[0] else {
            panic!("expected example");
        };
        assert!(matches!(ty, Expr::Arrow { .. }));
        assert!(matches!(val, Expr::Hole { .. }));
    }
}
