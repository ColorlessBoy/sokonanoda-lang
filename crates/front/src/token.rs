//! 词法器：`TokenKind`/`Token`/`Lexer` 与 `tokenize` 入口。
//!
//! **声明驱动的符号表**（G-04 第二刀，设计 `docs/design/notation-subset.md`
//! §10.2）：源码里声明过的记法符号（`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`…）在**本文件里
//! 是保留的**——`tokenize_with_symbols` 在常规分支之前先做「声明符号最长匹配」，
//! 命中就产出 `Sym`。`tokenize`（空符号表）与第一刀逐字节相同。

use super::diagnostic::{Diagnostic, DiagnosticKind, Result};
use crate::span::{Pos, Span};

/// 记法命令的拼写（`parser::is_reserved_command` 与
/// [`scan_notation_symbols`] 共用**同一份**清单）。
///
/// 第三刀（§12）加 `binder_notation`：它的符号出现在 **binder 位置**
/// （`∃ x, p`），但符号本身照样是声明驱动的（`∃` 在数学符号类里，
/// 而 `binder_notation "Π" => …` 这类字母符号要靠预扫描进符号表）。
pub(crate) const NOTATION_COMMANDS: &[&str] = &[
    "infix",
    "infixl",
    "infixr",
    "prefix",
    "postfix",
    "notation",
    "binder_notation",
];

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Num(String),
    /// 字符串字面量（`" ∈ "`）。只给记法命令用：表达式里没有字符串。
    Str(String),
    /// 记法符号（`∈`/`⊆`/`∅`/`\`…，以及声明驱动的 `𝒫`/`''`/`×ˢ`）。
    /// 见 `is_math_symbol` 与 [`scan_notation_symbols`]。
    Sym(String),
    Hole,
    Colon,
    ColonEq,
    Arrow, // ->
    Plus,
    Pipe,     // |
    FatArrow, // =>
    Forall,   // ∀ or forall
    At,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    src: &'a str,
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    offset: usize,
    line: usize,
    column: usize,
    /// 声明过的记法符号，**按长度降序**（最长匹配优先）。空表 ⇒ 第一刀行为。
    symbols: Vec<String>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self::with_symbols(src, &[])
    }

    /// 带**声明符号表**的词法（第二刀）：`symbols` 是源码里记法命令声明过的
    /// 符号文本（[`scan_notation_symbols`] 的产出）。
    pub fn with_symbols(src: &'a str, symbols: &[String]) -> Self {
        let mut sorted: Vec<String> = symbols.to_vec();
        // 最长匹配优先：`''` 与 `'` 同时声明时先试 `''`。
        sorted.sort_by_key(|symbol| std::cmp::Reverse(symbol.chars().count()));
        Self {
            src,
            chars: src.chars().peekable(),
            offset: 0,
            line: 1,
            column: 1,
            symbols: sorted,
        }
    }

    /// 当前位置开始的**最长**声明符号。
    fn declared_symbol_ahead(&self) -> Option<String> {
        let rest = self.src.get(self.offset.min(self.src.len())..)?;
        self.symbols
            .iter()
            .find(|symbol| rest.starts_with(symbol.as_str()))
            .cloned()
    }

    /// 当前 token 之前的本行正文（用于把 `import` 行里的 `-` 认出来，
    /// 见 `next_token` 的 `-` 分支）——只在错误冷路径调用。
    fn line_prefix(&self) -> &'a str {
        let before = &self.src[..self.offset.min(self.src.len())];
        let start = before.rfind('\n').map_or(0, |index| index + 1);
        &self.src[start..self.offset.min(self.src.len())]
    }

    /// 本行是不是一条 `import`（允许行尾 `--` 注释）。
    fn on_import_line(&self) -> Option<&'a str> {
        let line = self.line_prefix();
        let code = line.split("--").next().unwrap_or(line).trim();
        let rest = code.strip_prefix("import")?;
        // `importFoo` 不算：import 后面必须是空白或文件结束。
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            Some(rest.trim())
        } else {
            None
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
                    } else if let Some(partial) = self.on_import_line() {
                        // `import unit1-propositions-proofs`：文件名里的横线不是
                        // 模块名字符（与官方 Lean 同规则），给专门的教学提示，
                        // 而不是让通用的 "expected `->` or `--`" 糊过去。
                        let end = self.pos();
                        // `partial` 是**当前行已经写下的部分**，扫描到 `-` 才停，
                        // 所以它通常已经带上了这个横线——再补一个就成了 `my--…`
                        // （实测踩过）。这里只在真的缺横线时补。
                        let shown = if partial.ends_with('-') {
                            partial.to_string()
                        } else {
                            format!("{partial}-")
                        };
                        return Err(Diagnostic::new(
                            DiagnosticKind::ImportNotAModuleName {
                                module: shown.clone(),
                                message: format!("`import {shown}…`：模块名里不能有 `-`"),
                                hint: crate::project::module_name::DASH_HINT,
                            },
                            Span::new(start, end),
                            format!("`import {shown}…`：模块名里不能有 `-`"),
                        ));
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
        // 声明驱动的符号（第二刀）：**最长匹配优先**，且在常规分支之前——
        // `𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ` 里前四个是标识符字符、`''` 今天根本不是
        // 合法 token，只有这条路能把它们读成 `Sym`。
        if let Some(symbol) = self.declared_symbol_ahead() {
            for _ in symbol.chars() {
                self.bump();
            }
            return Ok(Token {
                kind: TokenKind::Sym(symbol),
                span: Span::new(start, self.pos()),
            });
        }
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
            ';' => self.single(TokenKind::Semicolon, start),
            '+' => self.single(TokenKind::Plus, start),
            '|' => self.single(TokenKind::Pipe, start),
            '"' => self.lex_string(start),
            // `∀`（U+2200）落在数学符号码点类里，但它今天就是一个 token：
            // 这一臂必须留在符号分支之前，否则关键字失效。
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
                    // 不可达：`-` 在跳过空白/注释的循环里就被消费或报错了
                    // （见 `next_token` 顶部的 `Some('-')` 分支）。
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
            // `'` 单独出现时是**符号**而不是词法错误（第二刀）：`''`（像）就是
            // 两个撇号。`'` 只是**续接**字符，所以 `x'` 仍是一个标识符；但
            // 没有声明过 `''` 的文件里它给的是「未声明符号」这条**专用**诊断
            // （与第一刀把 `\` 从词法错误改成 `Sym` 同一个理由：诊断更教学），
            // 也让"入口用了 import 来的 `''`"能被分发逻辑认出来。
            '\'' => self.lex_symbol_run(start),
            ch if is_math_symbol(ch) => self.lex_symbol(start),
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

    /// `"…"`：字符串字面量（记法命令的符号）。未闭合给专用诊断，span 指向
    /// **开引号**（学习者要看到的是那个没配对的引号）。
    fn lex_string(&mut self, start: Pos) -> Result<Token> {
        self.bump(); // 开引号
        let mut text = String::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.bump();
                    return Ok(Token {
                        kind: TokenKind::Str(text),
                        span: Span::new(start, self.pos()),
                    });
                }
                Some('\n') | None => {
                    return Err(Diagnostic::new(
                        DiagnosticKind::UnterminatedString,
                        Span::new(start, start),
                        "字符串没有闭合：记法命令里的符号要写在一对引号之间，例如 infix:50 \" ∈ \" => Set.mem".to_string(),
                    ));
                }
                Some(ch) => {
                    text.push(ch);
                    self.bump();
                }
            }
        }
    }

    /// 连续**撇号**算一个 `Sym`（`''` / `'''`）：`'` 不在数学符号码点类里，
    /// 所以单独给它一条臂（第二刀 §10.2）。
    fn lex_symbol_run(&mut self, start: Pos) -> Result<Token> {
        let mut text = String::new();
        while self.peek() == Some('\'') {
            text.push('\'');
            self.bump();
        }
        Ok(Token {
            kind: TokenKind::Sym(text),
            span: Span::new(start, self.pos()),
        })
    }

    /// 连续数学符号字符算**一个** `Sym`（最大吞噬）：`⁻¹'` 是一个 token，
    /// `∈` 也是。见 `is_math_symbol` 的码点类。
    fn lex_symbol(&mut self, start: Pos) -> Result<Token> {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if is_math_symbol(ch) {
                text.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        Ok(Token {
            kind: TokenKind::Sym(text),
            span: Span::new(start, self.pos()),
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
            // 声明过的符号在标识符**内部**也优先断开（第二刀）：`Aᶜ` 必须是
            // `Ident("A") + Sym("ᶜ")`，否则 `ᶜ` 会被吃进 `Aᶜ` 这个标识符。
            if self.declared_symbol_ahead().is_some() {
                break;
            }
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

/// 标识符起始字符（模块名分量复用同一份谓词，见 `crate::project::module_name`）。
///
/// **数学符号码点不是标识符字符**（G-04 / WO-011）：否则 `a∈b` 会是一个
/// `Ident`，记法永远不可能被 parser 看见。希腊字母与数学斜体字母（`α`、`𝒫`、
/// `ᶜ`）**仍然是**标识符字符——它们不在符号码点类里。
pub(crate) fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || ((c as u32) >= 0x80 && !is_math_symbol(c))
}

/// 标识符续接字符（含 `.`——所以 `Foo.Bar` 在词法层是**一个** `Ident`）。
pub(crate) fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit() || c == '\'' || c == '!' || c == '?' || c == '.'
}

/// 记法符号字符的码点类（`docs/design/notation-subset.md` §3.2）：
///
/// * `U+2200–U+22FF`：数学算子（`∈`U+2208 / `⊆`U+2286 / `∅`U+2205 /
///   `∪`U+222A / `∩`U+2229 / `∘`U+2218 …）；
/// * `U+2A00–U+2AFF`：为第二刀的 `⋃`/`⋂` 预留；
/// * `\`（U+005C）：今天直接是词法错误，改成符号 token 后 parser 能报
///   「未声明符号」——比「不是一个合法 token」更教学。
///
/// **`𝒫`（U+1D4AB）与 `ᶜ`（U+1D9C）是 Unicode 字母，不在本类里**：它们仍是
/// 标识符字符。第二刀（0.60.0）用**声明驱动的符号表**处理它们——见
/// [`scan_notation_symbols`] 与 [`Lexer::with_symbols`]（设计 §10.2）。
pub(crate) fn is_math_symbol(c: char) -> bool {
    matches!(c as u32, 0x2200..=0x22FF | 0x2A00..=0x2AFF) || c == '\\'
}

/// 纯 ASCII 标识符词（`in`/`e`/`Set`）：**不能**当记法符号——声明驱动的词法会把
/// 整个文件里的这个名字都收走（`in` 甚至还是 `infix` 的前缀，会把关键字拆掉）。
pub(crate) fn is_ascii_word_symbol(symbol: &str) -> bool {
    !symbol.is_empty()
        && symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 符号里有没有**词法在符号匹配之前就消费掉**的字符（`∀` / `->` / `--` /
/// `#check` / 字符串引号）——有就永远命中不了。
pub(crate) fn lexer_reserved_symbol_char(symbol: &str) -> Option<char> {
    symbol.chars().find(|c| matches!(c, '∀' | '-' | '#' | '"'))
}

/// 记法符号的**词法合法性**：`parser::parse_notation_symbol`（给教学诊断）与
/// [`scan_notation_symbols`]（决定保留哪些字符序列）**共用同一份判据**——两边
/// 一旦分叉，一个被拒的符号会照样进符号表，把 `infix` 拆成 `in`+`fix` 这种
/// 事就会发生（实测踩过）。
pub(crate) fn is_valid_notation_symbol(symbol: &str) -> bool {
    !symbol.is_empty()
        && !is_ascii_word_symbol(symbol)
        && lexer_reserved_symbol_char(symbol).is_none()
}

/// **词法级 `import` 扫描**（G-04 第二刀 §10.3）：每一条 `import <模块名>` 的
/// `(模块名, span)`，按书写顺序、按名字去重。
///
/// 判据与 [`Lexer::on_import_line`] **同款**：`import` 必须在行首（允许前导
/// 空白）、后面跟空白，名字是紧随其后的标识符（`.` 是续接字符 ⇒ `lib.Set`
/// 是一个名字）；行尾的 `--` 注释不算内容。
///
/// 用途只有一个：闭包加载器要在**解析一个模块之前**拿到它的 `import` 边
/// （`project/graph.rs`），因为被导入模块的记法必须先就位——否则入口用了库
/// 记法时会死在 `notation-unknown-symbol` 上，而"解析失败 ⇒ 收不到边 ⇒ 依赖
/// 不加载"是个死锁。**判定永不使用它**（判定只认 kernel）。
pub(crate) fn scan_import_lines(src: &str) -> Vec<(String, Span)> {
    let mut out: Vec<(String, Span)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut line_start = 0usize;
    for (index, line) in src.lines().enumerate() {
        let this_line = line_start;
        line_start += line.len() + 1; // `\n`；最后一行没有也只会多算 1，不再使用
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("import") else {
            continue;
        };
        // `importFoo` 不算（`import` 后面必须是空白或行尾）。
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let indent = line.len() - trimmed.len();
        let name_start = indent + "import".len() + (rest.len() - rest.trim_start().len());
        let name: String = line[name_start..]
            .chars()
            .take_while(|c| is_ident_continue(*c))
            .collect();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        let column = line[..name_start].chars().count() + 1;
        let start = Pos {
            offset: this_line + name_start,
            line: index + 1,
            column,
        };
        let end = Pos {
            offset: start.offset + name.len(),
            line: index + 1,
            column: column + name.chars().count(),
        };
        out.push((name, Span::new(start, end)));
    }
    out
}

pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    tokenize_with_symbols(src, &[])
}

/// 带**声明符号表**的词法（第二刀，设计 §10.2）：`symbols` 之外一切逐字节
/// 等于 [`tokenize`]。
pub fn tokenize_with_symbols(src: &str, symbols: &[String]) -> Result<Vec<Token>> {
    let mut lexer = Lexer::with_symbols(src, symbols);
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

/// **预扫描**（第二刀，设计 §10.2）：源码里所有记法命令声明的符号文本。
///
/// 一趟字符扫描，**跳过 `--` 行注释与字符串字面量**；状态机只有两态：
/// 「刚读完一条记法命令关键字」（含可选的 `:N`）⇒ 紧跟着的字符串字面量就是
/// 符号。它**不解析**文件（不认识命令、不检查目标名）——只回答"哪些字符序列
/// 在本文件里是记法符号"，交给 [`tokenize_with_symbols`] 做最长匹配。
///
/// 预扫描看得见**整份源码**的声明（包括声明点之前的），所以「声明之前使用」
/// 得到的是 parser 的 `notation-unknown-symbol`（N5 的专用诊断），不是
/// `unknown identifier`——与第一刀 `∈` 的行为一致。
pub(crate) fn scan_notation_symbols(src: &str) -> Vec<String> {
    let chars: Vec<char> = src.chars().collect();
    let mut symbols: Vec<String> = Vec::new();
    let mut expecting_symbol = false;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // `--` 行注释（`->` 不是注释：这里要求两个 `-`）。
        if c == '-' && chars.get(i + 1) == Some(&'-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' && chars[i] != '\n' {
                text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                i += 1;
            }
            if expecting_symbol {
                let symbol = text.trim().to_string();
                // 与 parser 的合法性判据**同一份**：被拒的符号不进符号表
                // （否则 `"in"` 会把 `infix` 拆成 `in` + `fix`）。
                if is_valid_notation_symbol(&symbol) && !symbols.contains(&symbol) {
                    symbols.push(symbol);
                }
            }
            expecting_symbol = false;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if is_ident_start(c) {
            let mut text = String::new();
            while i < chars.len() && is_ident_continue(chars[i]) {
                text.push(chars[i]);
                i += 1;
            }
            expecting_symbol = NOTATION_COMMANDS.contains(&text.as_str());
            continue;
        }
        // `infix:50 " … "`：`:` 与数字不改变"下一个字符串是符号"的状态。
        if c == ':' && expecting_symbol {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            continue;
        }
        expecting_symbol = false;
        i += 1;
    }
    symbols
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

    /// `import my-lib`：模块名里的横线要被专门指出来，且**只显示一个**横线
    /// （回归：`partial` 已经含掉当前这个 `-`，早先又补了一个，消息成了 `my--…`）。
    #[test]
    fn dashed_import_module_name_is_shown_once() {
        let err = tokenize("import my-lib\n").expect_err("dash must be rejected");
        let DiagnosticKind::ImportNotAModuleName {
            module, message, ..
        } = &err.kind
        else {
            panic!("expected ImportNotAModuleName, got {:?}", err.kind);
        };
        assert_eq!(module, "my-", "the module name keeps exactly one dash");
        assert!(message.contains("`import my-…`"), "{message}");
        assert!(!message.contains("my--"), "{message}");
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

    // ---- 记法（G-04 / WO-011，docs/design/notation-subset.md §3）----------

    #[test]
    fn notation_command_lexes_with_a_string_literal() {
        // WO-011 验收：`infix:50 " ∈ " => mem` 的 token 形状。
        let toks = tokenize("infix:50 \" ∈ \" => mem").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident("infix".into()),
                TokenKind::Colon,
                TokenKind::Num("50".into()),
                TokenKind::Str(" ∈ ".into()),
                TokenKind::FatArrow,
                TokenKind::Ident("mem".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn math_symbols_lex_as_sym_not_as_part_of_an_ident() {
        // 今天 `a∈b` 是**一个** Ident（`is_ident_start` 把 ≥0x80 一律当标识符
        // 首字符）⇒ 记号永远不可能被 parser 看见。收窄之后必须是三个 token。
        let toks = tokenize("x∈A").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Sym("∈".into()),
                TokenKind::Ident("A".into()),
                TokenKind::Eof,
            ]
        );
        // 无空格同样切得开（v1 比「要求两侧空格」更强，设计 §3.3）。
        let toks = tokenize("A⊆B").unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("⊆".into()));
        let toks = tokenize("∅").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Sym("∅".into()));
    }

    #[test]
    fn consecutive_symbol_characters_are_one_sym_token() {
        // 最大吞噬：连续符号字符算一个 token。这里用 `⋃⋂`（都在
        // U+2200–U+22FF 内）钉住规则本身；第二刀的真实多字符符号（`×ˢ`）
        // 还牵涉码点类之外的字符，见设计 §7。
        let toks = tokenize("A ⋃⋂ B").unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("⋃⋂".into()));
    }

    #[test]
    fn backslash_is_a_sym_token_not_a_lex_error() {
        // 今天 `\` 直接落进 `other =>` 报 unexpected-token；改成符号 token 后
        // parser 能报「未声明符号」——比「不是一个合法 token」更教学。
        let toks = tokenize("A \\ B").unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("\\".into()));
    }

    #[test]
    fn greek_and_math_italic_letters_stay_identifiers() {
        // `α` 与 `α'1` 逐字不变（既有回归）；`𝒫`/`ᶜ` 是 Unicode **字母**，
        // 不在数学符号码点类里 ⇒ 仍是标识符字符（设计 §3.4，第二刀另设计）。
        for text in ["α", "α'1", "𝒫", "ᶜ", "β2"] {
            let toks = tokenize(text).unwrap();
            assert_eq!(
                toks[0].kind,
                TokenKind::Ident(text.into()),
                "`{text}` must stay one identifier"
            );
            assert_eq!(toks[1].kind, TokenKind::Eof);
        }
    }

    #[test]
    fn forall_keeps_its_own_token_inside_the_symbol_range() {
        // `∀` U+2200 落在数学符号码点类里，但它今天就是一个 token：符号分支
        // 必须让路，否则 `∀` 会变成 `Sym` 而关键字失效。
        let toks = tokenize("∀ (a : Prop), a").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Forall);
        let toks = tokenize("⊢ a").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Sym("⊢".into()));
    }

    #[test]
    fn unterminated_string_is_a_parse_diagnostic_at_the_opening_quote() {
        let err = tokenize("infix:50 \" ∈ => mem").expect_err("unterminated string");
        assert_eq!(err.code(), "unterminated-string");
        assert_eq!(err.span.start.column, 10, "span points at the opening `\"`");
        assert_eq!(err.span.start.line, 1);
    }

    // ---- 声明驱动的符号表（第二刀，设计 §10.2）------------------------------

    /// 一份声明了五个卷 I 符号的源码（预扫描的靶子）。
    const SECOND_CUT_DECLS: &str = "\
prefix:100 \" 𝒫 \" => Set.powerset\n\
postfix:100 \" ᶜ \" => Set.compl\n\
infixr:80 \" '' \" => Set.image\n\
infixr:80 \" ⁻¹' \" => Set.preimage\n\
infixr:80 \" ×ˢ \" => Set.prod\n";

    #[test]
    fn scan_finds_every_declared_symbol() {
        let symbols = scan_notation_symbols(SECOND_CUT_DECLS);
        assert_eq!(
            symbols,
            vec![
                "𝒫".to_string(),
                "ᶜ".into(),
                "''".into(),
                "⁻¹'".into(),
                "×ˢ".into()
            ]
        );
        // 第一刀的四条命令也走同一条路。
        let symbols =
            scan_notation_symbols("infix:50 \" ∈ \" => Set.mem\nnotation \"∅\" => Set.empty\n");
        assert_eq!(symbols, vec!["∈".to_string(), "∅".into()]);
    }

    #[test]
    fn scan_skips_comments_and_strings_that_are_not_notation_symbols() {
        // 注释里的记法行**不算**声明（否则 `-- infix:50 " x "` 会把 `x` 变成符号）。
        let symbols = scan_notation_symbols("-- prefix:100 \" 𝒫 \" => Set.powerset\n𝒫 A\n");
        assert!(
            symbols.is_empty(),
            "a commented-out declaration is not a declaration"
        );
        // `=>` 之后的字符串（v1 没有这种东西）也不改变状态。
        let symbols = scan_notation_symbols("def x : Prop := Prop\n\"nope\"\n");
        assert!(symbols.is_empty(), "symbols: {symbols:?}");
        // `->` 不是注释；`--` 之后的整行都被跳掉。
        let symbols = scan_notation_symbols("def f : A -> B := x -- prefix:100 \" 𝒫 \" => y\n");
        assert!(symbols.is_empty(), "symbols: {symbols:?}");
    }

    #[test]
    fn declared_symbols_lex_as_sym_with_longest_match() {
        let symbols = scan_notation_symbols(SECOND_CUT_DECLS);
        // `𝒫 A`：数学斜体字母本来是标识符字符。
        let toks = tokenize_with_symbols("𝒫 A", &symbols).unwrap();
        assert_eq!(toks[0].kind, TokenKind::Sym("𝒫".into()));
        assert!(matches!(&toks[1].kind, TokenKind::Ident(s) if s == "A"));
        // `Aᶜ`（无空格）：`ᶜ` 是**续接**字符，必须在标识符内部断开。
        let toks = tokenize_with_symbols("Aᶜ", &symbols).unwrap();
        assert!(matches!(&toks[0].kind, TokenKind::Ident(s) if s == "A"));
        assert_eq!(toks[1].kind, TokenKind::Sym("ᶜ".into()));
        // `f '' A`：`''` 今天根本不是合法 token。
        let toks = tokenize_with_symbols("f '' A", &symbols).unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("''".into()));
        // `A ⁻¹' B` 与 `A ×ˢ B`。
        let toks = tokenize_with_symbols("A ⁻¹' B", &symbols).unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("⁻¹'".into()));
        let toks = tokenize_with_symbols("A×ˢB", &symbols).unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("×ˢ".into()));
        // 未声明的数学斜体字母仍是标识符（**没有**声明驱动的收窄）。
        let toks = tokenize("𝒫 A").unwrap();
        assert!(matches!(&toks[0].kind, TokenKind::Ident(s) if s == "𝒫"));
    }

    #[test]
    fn tokenize_without_symbols_is_byte_identical_to_the_first_cut() {
        // A1 的守护：空符号表 ⇒ 与 `tokenize` 完全一样（含 `''` 仍是词法错误）。
        let src = "infix:50 \" ∈ \" => Set.mem\na ∈ A\n";
        assert_eq!(
            tokenize_with_symbols(src, &[]).unwrap(),
            tokenize(src).unwrap()
        );
        // `''` 从「词法错误」升级为「未声明符号」（第二刀：`'` 单独出现是 `Sym`）
        // ——诊断更教学，也让"入口用了 import 来的 `''`"能被分发逻辑认出来。
        let toks = tokenize("f '' A").unwrap();
        assert_eq!(toks[1].kind, TokenKind::Sym("''".into()));
    }
}
