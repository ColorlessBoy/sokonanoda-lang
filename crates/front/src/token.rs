//! 词法器：`TokenKind`/`Token`/`Lexer` 与 `tokenize` 入口。

use super::diagnostic::{Diagnostic, DiagnosticKind, Result};
use crate::span::{Pos, Span};

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
            // `sorry` 已移除（2026-09-07，学习者反馈）：未完成证明的占位符
            // 统一为 `sorry`（与官方 Lean 一致）。遇到 `?` 给出教学引导。
            '?' => {
                self.bump();
                while self.peek() == Some('?') {
                    self.bump();
                }
                Err(Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        found: "?".to_string(),
                        expected: "`sorry`".to_string(),
                    },
                    Span::new(start, self.pos()),
                    "??? 已移除：未完成的证明请写 sorry（与官方 Lean 一致）".to_string(),
                ))
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
    fn line_comment_is_skipped() {
        let toks = tokenize("-- a comment with = ? -> junk\nx").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(kinds, vec![TokenKind::Ident("x".into()), TokenKind::Eof]);
    }

    #[test]
    fn sorry_lexes_as_ident_and_parser_treats_it_as_hole() {
        // `sorry` 是标识符 token（parse_atom 将其解释为洞）；
        // `???` 已移除，遇到 `?` 由 lexer 报教学错误。
        let toks = tokenize("sorry").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Ident("sorry".to_string()));
        assert_eq!(toks[1].kind, TokenKind::Eof);
        assert!(
            tokenize("???").is_err(),
            "??? must be rejected with guidance"
        );
    }

    #[test]
    fn forall_is_a_keyword_but_prefixed_idents_are_not() {
        let toks = tokenize("forall ∀ foralls forall!").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Forall);
        assert_eq!(toks[1].kind, TokenKind::Forall);
        assert_eq!(toks[2].kind, TokenKind::Ident("foralls".into()));
        assert_eq!(toks[3].kind, TokenKind::Ident("forall!".into()));
    }

    #[test]
    fn punctuators_lex_in_order() {
        let toks = tokenize("-> => : := @").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::Colon,
                TokenKind::ColonEq,
                TokenKind::At,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn universe_application_slice_lexes_as_plain_tokens() {
        let toks = tokenize("id.{u}").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident("id.".into()),
                TokenKind::LBrace,
                TokenKind::Ident("u".into()),
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn number_lexes_as_num_token() {
        let toks = tokenize("1234567890").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Num("1234567890".into()));
        assert_eq!(toks[1].kind, TokenKind::Eof);
    }

    #[test]
    fn non_ascii_identifiers_lex_as_ident() {
        let toks = tokenize("α'1").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Ident("α'1".into()));
        assert_eq!(toks[1].kind, TokenKind::Eof);
    }

    #[test]
    fn lone_equals_is_an_error_with_position() {
        let err = tokenize("x\n  =").unwrap_err();
        assert_eq!(
            err.kind,
            DiagnosticKind::UnexpectedToken {
                found: "=".into(),
                expected: "expected `=>`".into(),
            }
        );
        assert_eq!(err.span.start.line, 2);
        assert_eq!(err.span.start.column, 3);
    }

    #[test]
    fn hash_command_lexes_as_ident() {
        let toks = tokenize("#check #reduce #print").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Ident("#check".into()));
        assert_eq!(toks[1].kind, TokenKind::Ident("#reduce".into()));
        assert_eq!(toks[2].kind, TokenKind::Ident("#print".into()));
    }
}
