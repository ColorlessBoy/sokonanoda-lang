//! The restricted `.sokonanoda` front-end.
//!
//! This crate deliberately implements only the grammar points exposed by the
//! teaching curriculum. The syntax whitelist is the curriculum: adding a
//! grammar point here means adding a lesson for it.

pub mod compile;
pub mod proof;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pos {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

impl Span {
    pub fn new(start: Pos, end: Pos) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Num(String),
    Hole,
    Colon,
    ColonEq,
    Arrow, // ->
    Plus,
    FatArrow, // =>
    Forall,   // ∀ or forall
    At,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    UnexpectedEof,
    UnexpectedToken { found: String, expected: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    fn new(kind: DiagnosticKind, span: Span, message: String) -> Self {
        Self {
            kind,
            span,
            message,
        }
    }
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().peekable(),
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn pos(&self) -> Pos {
        Pos {
            offset: self.offset,
            line: self.line,
            column: self.column,
        }
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn next_token(&mut self) -> Result<Token> {
        let c = loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => {
                    self.bump();
                }
                Some('-') => {
                    let start = self.pos();
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        let end = self.pos();
                        return Ok(Token {
                            kind: TokenKind::Arrow,
                            span: Span::new(start, end),
                        });
                    } else if self.peek() == Some('-') {
                        while let Some(ch) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.bump();
                        }
                    } else {
                        return Err(self.err_unexpected(start, "expected `->` or `--`", "-"));
                    }
                }
                Some(ch) => break ch,
                None => {
                    let p = self.pos();
                    return Ok(Token {
                        kind: TokenKind::Eof,
                        span: Span::new(p, p),
                    });
                }
            }
        };
        let start = self.pos();
        match c {
            '#' => {
                self.bump();
                let mut text = String::from("#");
                while let Some(ch) = self.peek() {
                    if is_ident_continue(ch) || ch.is_ascii_alphabetic() {
                        text.push(ch);
                        self.bump();
                    } else {
                        break;
                    }
                }
                let end = self.pos();
                Ok(Token {
                    kind: TokenKind::Ident(text),
                    span: Span::new(start, end),
                })
            }
            '?' => {
                self.bump();
                if self.peek() == Some('?') {
                    self.bump();
                    if self.peek() == Some('?') {
                        self.bump();
                        let end = self.pos();
                        return Ok(Token {
                            kind: TokenKind::Hole,
                            span: Span::new(start, end),
                        });
                    }
                }
                Err(self.err_unexpected(start, "expected `???`", "?"))
            }
            ':' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    let end = self.pos();
                    Ok(Token {
                        kind: TokenKind::ColonEq,
                        span: Span::new(start, end),
                    })
                } else {
                    let end = self.pos();
                    Ok(Token {
                        kind: TokenKind::Colon,
                        span: Span::new(start, end),
                    })
                }
            }
            '(' => self.single(TokenKind::LParen, start),
            ')' => self.single(TokenKind::RParen, start),
            '{' => self.single(TokenKind::LBrace, start),
            '}' => self.single(TokenKind::RBrace, start),
            ',' => self.single(TokenKind::Comma, start),
            '+' => self.single(TokenKind::Plus, start),
            '∀' => self.single(TokenKind::Forall, start),
            '@' => self.single(TokenKind::At, start),
            '-' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    let end = self.pos();
                    Ok(Token {
                        kind: TokenKind::Arrow,
                        span: Span::new(start, end),
                    })
                } else {
                    Err(self.err_unexpected(start, "expected `->` or `--`", "-"))
                }
            }
            '=' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    let end = self.pos();
                    Ok(Token {
                        kind: TokenKind::FatArrow,
                        span: Span::new(start, end),
                    })
                } else {
                    Err(self.err_unexpected(start, "expected `=>`", "="))
                }
            }
            ch if ch.is_ascii_digit() => self.lex_number(start),
            ch if is_ident_start(ch) => self.lex_ident(start),
            other => {
                self.bump();
                Err(self.err_unexpected(start, "a valid .sokonanoda token", &other.to_string()))
            }
        }
    }

    fn single(&mut self, kind: TokenKind, start: Pos) -> Result<Token> {
        self.bump();
        let end = self.pos();
        Ok(Token {
            kind,
            span: Span::new(start, end),
        })
    }

    fn lex_number(&mut self, start: Pos) -> Result<Token> {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                text.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        Ok(Token {
            kind: TokenKind::Num(text),
            span: Span::new(start, self.pos()),
        })
    }

    fn lex_ident(&mut self, start: Pos) -> Result<Token> {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if is_ident_continue(ch) {
                text.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        let kind = if text == "forall" {
            TokenKind::Forall
        } else {
            TokenKind::Ident(text)
        };
        Ok(Token {
            kind,
            span: Span::new(start, self.pos()),
        })
    }

    fn err_unexpected(&self, start: Pos, expected: &str, found: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::UnexpectedToken {
                found: found.to_string(),
                expected: expected.to_string(),
            },
            Span::new(start, start),
            format!("expected {expected}, found {found}"),
        )
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || (c as u32) >= 0x80
}

fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit() || c == '\'' || c == '!' || c == '?' || c == '.'
}

pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    let mut lexer = Lexer::new(src);
    let mut out = Vec::new();
    loop {
        let tok = lexer.next_token()?;
        let eof = tok.kind == TokenKind::Eof;
        out.push(tok);
        if eof {
            return Ok(out);
        }
    }
}

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

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
            | Expr::Plus { span, .. } => *span,
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
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

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
    fn tokenizes_hello_sokonanoda() {
        let toks = tokenize("-- lesson\n#check Prop -> Prop").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert!(matches!(kinds[0], TokenKind::Ident(ref s) if s == "#check"));
        assert!(matches!(kinds[1], TokenKind::Ident(ref s) if s == "Prop"));
        assert_eq!(kinds[2], TokenKind::Arrow);
        assert!(matches!(kinds[3], TokenKind::Ident(ref s) if s == "Prop"));
        assert_eq!(kinds[4], TokenKind::Eof);
    }

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
}
