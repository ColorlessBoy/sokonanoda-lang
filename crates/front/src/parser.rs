//! 递归下降解析器：tokens → AST（命令与表达式）。

use super::ast::{
    Binder, BinderKind, Command, CtorDecl, Expr, FolFile, IotaRule, MatchArm, NotationAssoc,
    Pattern, RecDecl, SortKind, Tactic,
};
use super::diagnostic::{Diagnostic, DiagnosticKind, Result};
use super::span::Span;
use super::token::{is_ident_start, tokenize, Token, TokenKind};
use crate::project::{ModuleName, ModuleNameError};
use std::collections::HashMap;

/// 内建 `+` 在优先级梯子上的位置（`docs/design/notation-subset.md` N3）。
///
/// `+` 是**保留项**：走同一条爬升，但产出 `Expr::Plus`（行为逐字节不变）。
/// 这个数本身**未从 Lean 源码取证**（WO-011 待确认 2）；v1 的实现不依赖它，
/// 它只影响 `a ∈ A + B` 这类混写的分组。
const PLUS_PRECEDENCE: u16 = 65;

/// 记法优先级的合法范围（N1）：严格落在 `->` 与函数应用之间。
const NOTATION_PRECEDENCE_RANGE: std::ops::RangeInclusive<u16> = 1..=1000;

/// 一条已声明的记法（`infix` 族或零元 `notation`）。
#[derive(Debug, Clone)]
struct NotationEntry {
    symbol: String,
    /// 零元记法为 `None`。
    precedence: Option<u16>,
    assoc: NotationAssoc,
    /// 点名目标（`Set.mem`）。解析期只记下来，elaborate 期才解析。
    target: String,
}

/// 表达式位上的一个二元算子：内建 `+` 或一条已声明的记法。
#[derive(Debug, Clone)]
struct BinaryOp {
    precedence: u16,
    assoc: NotationAssoc,
    symbol: String,
    target: String,
    /// `true` ⇒ 产出 `Expr::Plus`（内建保留项），否则产出 `Expr::Notation`。
    builtin_plus: bool,
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    /// `match` 的 scrutinee 解析期间 > 0：让 `with` 停止 `parse_app` 的实参
    /// 收集（`with` 是普通标识符，否则会被当成 `c` 的实参吃掉）。
    scrutinee_depth: usize,
    /// tactic 解析期间 > 0：让换行处的 tactic 关键字终止当前表达式，
    /// 使 `by` 块可以省略分隔用的 `;`（tactic 之间换行即分隔）。
    by_depth: usize,
    /// **本文件已声明**的记法：符号 → 条目。作用域 = 文件内、声明之后
    /// （`docs/design/notation-subset.md` N5：跨 `import` 的记法是第二刀）。
    notations: HashMap<String, NotationEntry>,
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
    if binders.is_empty() {
        return (ty, val);
    }
    // 类型望远镜与 `axiom` 共用同一处折叠（见 [`wrap_type_binders`]）；值位的
    // 空 binder 特例已在上面返回，所以这里直接包 Lambda。
    let ty = wrap_type_binders(binders.clone(), ty);
    let val_span = Span::new(binders[0].span.start, val.span().end);
    let val = Expr::Lambda {
        binders,
        body: Box::new(val),
        span: val_span,
    };
    (ty, val)
}

/// 只折**类型位**的 binder 望远镜：`(α : Type) : Prop` ⇒
/// `forall (α : Type), Prop`。`axiom` 没有值位，所以它与 `def`/`theorem`
/// 的 `wrap_decl_binders` 共享这一处折叠（G-13）；空 binder 原样返回，
/// 于是无 binder 的 `axiom Foo : Prop` 走逐字不变的路径。
fn wrap_type_binders(binders: Vec<Binder>, ty: Expr) -> Expr {
    let Some(first) = binders.first() else {
        return ty;
    };
    let span = Span::new(first.span.start, ty.span().end);
    Expr::Forall {
        binders,
        body: Box::new(ty),
        span,
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            cursor: 0,
            scrutinee_depth: 0,
            by_depth: 0,
            notations: HashMap::new(),
        }
    }

    pub fn parse_file(&mut self, src: &str) -> Result<FolFile> {
        let mut commands = Vec::new();
        let mut seen_declaration = false;
        while !self.at_eof() {
            let command = self.parse_command()?;
            if command.is_import() {
                if seen_declaration {
                    return Err(Diagnostic::new(
                        DiagnosticKind::ImportMustPrecedeDeclarations,
                        command.span(),
                        "`import` 必须写在所有声明之前".to_string(),
                    ));
                }
            } else {
                seen_declaration = true;
            }
            commands.push(command);
        }
        Ok(FolFile {
            commands,
            src: src.to_string(),
        })
    }

    fn parse_command(&mut self) -> Result<Command> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Ident(kw) if kw == "import" => self.parse_import(),
            TokenKind::Ident(kw) if kw == "def" => self.parse_def(),
            TokenKind::Ident(kw) if kw == "theorem" => self.parse_theorem(),
            TokenKind::Ident(kw) if kw == "example" => self.parse_example(),
            TokenKind::Ident(kw) if kw == "axiom" => self.parse_axiom(),
            TokenKind::Ident(kw) if kw == "inductive" => self.parse_inductive_block(),
            TokenKind::Ident(kw) if kw == "#check" => self.parse_hash_check(),
            TokenKind::Ident(kw) if kw == "#reduce" => self.parse_hash_reduce(),
            TokenKind::Ident(kw) if kw == "#print" => self.parse_hash_print(),
            TokenKind::Ident(kw) if kw == "infix" || kw == "infixl" || kw == "infixr" => {
                self.parse_infix_command()
            }
            TokenKind::Ident(kw) if kw == "notation" => self.parse_notation_command(),
            _ => {
                Err(self
                    .error_at_current(&format!("expected a .sokonanoda command, found {tok:?}")))
            }
        }
    }

    /// `infix:N " sym " => name` / `infixl` / `infixr`（G-04 / WO-011）。
    /// `N` 必填且落在 `NOTATION_PRECEDENCE_RANGE`；符号是字符串字面量，
    /// 取 `trim` 后的内容（两侧空格是书写习惯）。
    fn parse_infix_command(&mut self) -> Result<Command> {
        let kw_tok = self.bump();
        let TokenKind::Ident(keyword) = kw_tok.kind.clone() else {
            unreachable!("parse_infix_command is only called on an infix keyword");
        };
        let assoc = match keyword.as_str() {
            "infix" => NotationAssoc::Infix,
            "infixl" => NotationAssoc::Infixl,
            _ => NotationAssoc::Infixr,
        };
        // `N` 必填：缺了要报「记法命令的形状」，不是通用的 unexpected-token。
        let precedence = self.parse_notation_precedence(&keyword)?;
        let (symbol, _) = self.parse_notation_symbol(&keyword)?;
        self.expect_notation_arrow(&keyword)?;
        let target = self.expect_ident("a notation target name")?;
        let end = self.tokens[self.cursor - 1].span.end;
        let span = Span::new(kw_tok.span.start, end);
        self.register_notation(
            symbol.clone(),
            Some(precedence),
            assoc,
            target.clone(),
            span,
        )?;
        Ok(Command::Notation {
            symbol,
            precedence: Some(precedence),
            assoc,
            target,
            span,
        })
    }

    /// `notation " sym " => name`（零元常量记法；不写优先级，照 core 的 `∅` 行）。
    fn parse_notation_command(&mut self) -> Result<Command> {
        let kw_tok = self.bump();
        // `notation:max` 这类写法 v1 不支持（设计「已知差异 4」）：报专用诊断
        // 而不是让 `:max` 落进通用错误。
        if self.peek().kind == TokenKind::Colon {
            return Err(self.notation_shape_error(
                "notation 不写优先级（零元记法没有左右操作数）：写 notation \"∅\" => Set.empty 即可",
                self.peek().span,
            ));
        }
        let (symbol, _) = self.parse_notation_symbol("notation")?;
        self.expect_notation_arrow("notation")?;
        let target = self.expect_ident("a notation target name")?;
        let end = self.tokens[self.cursor - 1].span.end;
        let span = Span::new(kw_tok.span.start, end);
        self.register_notation(
            symbol.clone(),
            None,
            NotationAssoc::Nullary,
            target.clone(),
            span,
        )?;
        Ok(Command::Notation {
            symbol,
            precedence: None,
            assoc: NotationAssoc::Nullary,
            target,
            span,
        })
    }

    /// `:N`（`infix` 族必填，范围 1–1000）。
    fn parse_notation_precedence(&mut self, keyword: &str) -> Result<u16> {
        if self.peek().kind != TokenKind::Colon {
            return Err(self.notation_shape_error(
                &format!(
                    "`{keyword}` 必须写优先级：{keyword}:50 \" ∈ \" => Set.mem（N 取 1–1000）"
                ),
                self.peek().span,
            ));
        }
        self.bump();
        let tok = self.bump();
        let TokenKind::Num(text) = tok.kind.clone() else {
            return Err(self.notation_shape_error(
                &format!("`{keyword}:N` 的 N 要是 1–1000 的数字"),
                tok.span,
            ));
        };
        let Ok(n) = text.parse::<u16>() else {
            return Err(self.notation_shape_error(
                &format!("优先级 `{text}` 不是合法数字（N 取 1–1000）"),
                tok.span,
            ));
        };
        if !NOTATION_PRECEDENCE_RANGE.contains(&n) {
            return Err(self.notation_shape_error(
                &format!("优先级 {n} 越界：合法范围是 1–1000（`->` 最松、函数应用最紧）"),
                tok.span,
            ));
        }
        Ok(n)
    }

    /// 符号必须是**字符串字面量**，`trim` 后非空、且不是标识符词
    /// （`"in"` 会被当普通标识符读走，记法永远不可能命中）。
    fn parse_notation_symbol(&mut self, keyword: &str) -> Result<(String, Span)> {
        let tok = self.bump();
        let TokenKind::Str(raw) = tok.kind.clone() else {
            return Err(self.notation_shape_error(
                &format!(
                    "`{keyword}` 的符号要写成字符串字面量（带引号）：{keyword}:50 \" ∈ \" => Set.mem"
                ),
                tok.span,
            ));
        };
        let symbol = raw.trim().to_string();
        if symbol.is_empty() {
            return Err(self.notation_shape_error(
                "记法符号不能是空串：写成 infix:50 \" ∈ \" => Set.mem 这样一对引号之间放符号",
                tok.span,
            ));
        }
        if symbol.chars().all(is_ident_start) {
            return Err(self.notation_shape_error(
                &format!(
                    "`{symbol}` 是普通标识符词，不能当记法符号（它会被当标识符读走，记法永远命中不了）；符号要用数学符号，例如 ∈ / ⊆ / ∅"
                ),
                tok.span,
            ));
        }
        Ok((symbol, tok.span))
    }

    fn expect_notation_arrow(&mut self, keyword: &str) -> Result<()> {
        if self.peek().kind == TokenKind::FatArrow {
            self.bump();
            Ok(())
        } else {
            let span = self.peek().span;
            Err(self.notation_shape_error(
                &format!("`{keyword}` 命令要用 `=>` 指向目标名：{keyword}:50 \" ∈ \" => Set.mem"),
                span,
            ))
        }
    }

    /// 登记一条记法；同一符号重复声明是 v1 的**错误**（设计「已知差异 6」：
    /// Lean 允许重载，本子集不做）。
    fn register_notation(
        &mut self,
        symbol: String,
        precedence: Option<u16>,
        assoc: NotationAssoc,
        target: String,
        span: Span,
    ) -> Result<()> {
        if self.notations.contains_key(&symbol) {
            return Err(self.notation_shape_error(
                &format!("符号 `{symbol}` 在本文件里已经声明过记法了：同一符号只能声明一次"),
                span,
            ));
        }
        self.notations.insert(
            symbol.clone(),
            NotationEntry {
                symbol,
                precedence,
                assoc,
                target,
            },
        );
        Ok(())
    }

    fn notation_shape_error(&self, detail: &str, span: Span) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::NotationShape {
                detail: detail.to_string(),
            },
            span,
            detail.to_string(),
        )
    }

    /// `import Foo.Bar`：一行一个点分模块名，且必须是合法标识符分量。
    /// 置顶规则在 `parse_file` 里统一判定（这里只管单条命令的形状）。
    fn parse_import(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let tok = self.bump();
        let (raw, name_span) = match tok.kind {
            TokenKind::Ident(name) => (name, tok.span),
            // 以数字开头的"名字"被词法层切成 Num：这仍然是**模块名不合法**，
            // 不是写法残缺（教学上要指到"不能以数字开头"）。
            TokenKind::Num(raw) => {
                let err = ModuleNameError::BadStart {
                    component: raw.clone(),
                    ch: raw.chars().next().unwrap_or('0'),
                };
                return Err(Diagnostic::new(
                    DiagnosticKind::ImportNotAModuleName {
                        module: raw.clone(),
                        message: format!("`import {raw}`：{}", err.message()),
                        hint: err.hint(),
                    },
                    tok.span,
                    format!("`import {raw}`：{}", err.message()),
                ));
            }
            other => {
                return Err(Diagnostic::new(
                    DiagnosticKind::ImportMalformed {
                        detail: format!("`import` 后面要跟模块名，这里found {other:?}"),
                    },
                    tok.span,
                    format!("`import` 后面要跟模块名，found {other:?}"),
                ))
            }
        };
        // 一行一个：模块名之后必须换行（或文件结束）。
        let next = self.peek().clone();
        if next.kind != TokenKind::Eof && next.span.start.line == name_span.end.line {
            return Err(Diagnostic::new(
                DiagnosticKind::ImportMalformed {
                    detail: format!("`import {raw}` 后面还有内容"),
                },
                next.span,
                format!("`import {raw}` 后面还有内容：一行只能写一个模块名"),
            ));
        }
        let span = Span::new(start, name_span.end);
        match ModuleName::parse(&raw) {
            Ok(_) => Ok(Command::Import { module: raw, span }),
            Err(err) => Err(Diagnostic::new(
                DiagnosticKind::ImportNotAModuleName {
                    module: raw.clone(),
                    message: format!("`import {raw}`：{}", err.message()),
                    hint: err.hint(),
                },
                span,
                format!("`import {raw}`：{}", err.message()),
            )),
        }
    }

    fn parse_def(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_decl_name("definition name")?;
        let mut universe = self.parse_universe_params()?;
        let binders = self.parse_decl_binders(&mut universe)?;
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
        let name = self.expect_decl_name("theorem name")?;
        let mut universe = self.parse_universe_params()?;
        let binders = self.parse_decl_binders(&mut universe)?;
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
        // `Command::Example` 没有 `universe` 字段 ⇒ 宇宙参数没有落脚处。这里必须
        // **明确报错**而不是让 `parse_decl_binders` 把组吞掉（那样 `Sort u` 会
        // 报成未声明宇宙，比今天更难懂）。
        if self.universe_group_ahead() {
            return Err(self.error_here("example 不支持宇宙参数；请改用 theorem 或 def"));
        }
        let mut universe = Vec::new();
        let binders = self.parse_decl_binders(&mut universe)?;
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
    ///
    /// `{` 开头的组有两义：`{a : T}`（有冒号）是隐式 binder 组；`{u v}`
    /// （只有名字，逗号或空格分隔）是**宇宙参数组**（G-14：可连排多组）。
    /// 宇宙名字并入调用方传进来的 `universe`（与名字写在 `name` 之后的
    /// `{u, v}` 同一份 Vec），**不进** `Binder`——否则会污染 `wrap_*` 与
    /// `by` 引擎的 `initial_binders` 语义。
    fn parse_decl_binders(&mut self, universe: &mut Vec<String>) -> Result<Vec<Binder>> {
        let mut binders = Vec::new();
        loop {
            if self.universe_group_ahead() {
                self.parse_universe_group(universe)?;
                continue;
            }
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

    /// 值位：普通表达式或 `by <tactic 序列>` 块。
    fn parse_value(&mut self) -> Result<Expr> {
        if let TokenKind::Ident(kw) = &self.peek().kind {
            if kw == "by" {
                return self.parse_by_block();
            }
        }
        self.parse_expr()
    }

    /// `by` 块：`by <tactic> ((';' | '\n') <tactic>)*`。tactic 之间既可用
    /// `;` 分隔，也可直接换行（tactic 关键字出现在下一行即视为分隔），
    /// 于是末尾的 `;` 可以省略。
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
                    // 换行分隔：下一个 token 是下一行的 tactic 关键字。
                    // 不 bump——交给下一次 `parse_tactic` 消费。
                    _ if self.next_line_starts_a_tactic() => continue,
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
        matches!(&self.peek().kind, TokenKind::Ident(kw) if is_tactic_keyword(kw))
    }

    /// 刚消费完的 token（`cursor - 1`）的结束行；空输入返回 0。
    fn last_token_end_line(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }
        self.tokens[self.cursor - 1].span.end.line
    }

    /// 下一个 token 是**换行后**出现的 tactic 关键字——tactic 边界。
    fn next_line_starts_a_tactic(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(kw) if is_tactic_keyword(kw))
            && self.peek().span.start.line > self.last_token_end_line()
    }

    fn parse_tactic(&mut self) -> Result<Tactic> {
        self.by_depth += 1;
        let result = self.parse_tactic_inner();
        self.by_depth -= 1;
        result
    }

    fn parse_tactic_inner(&mut self) -> Result<Tactic> {
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
            TokenKind::Ident(kw) if kw == "match" => {
                // `match` 作为 tactic：臂体是**项**（同值位 match），语义等价于
                // `exact (match … with …)`；`by` 引擎以当前目标为期望类型判定。
                let expr = self.parse_match()?;
                let end = expr.span().end;
                Ok(Tactic::Exact {
                    expr,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(kw) if kw == "sorry" => {
                self.bump();
                Ok(Tactic::Sorry { span: tok.span })
            }
            _ => Err(self.error_here(&format!(
                "未知 tactic：`by` 块只支持 intro / exact / apply / assumption / rfl / match / sorry（白名单），发现 {tok:?}"
            ))),
        }
    }

    fn parse_axiom(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_decl_name("axiom name")?;
        let mut universe = self.parse_universe_params()?;
        // G-13：与 def/theorem 同序——宇宙参数后先吃 binder 组，再进类型位。
        // 无 binder 的 `axiom Foo : Prop` 逐字走原路（wrap_type_binders 空则原样返回）。
        let binders = self.parse_decl_binders(&mut universe)?;
        self.expect_colon("axiom type")?;
        let ty = self.parse_expr()?;
        let span = Span::new(start, ty.span().end);
        let ty = wrap_type_binders(binders, ty);
        Ok(Command::Axiom {
            name,
            universe,
            ty,
            span,
        })
    }

    /// `{u}` / `{u, v}` / `{u v}`（**只有名字**，逗号或空格分隔）是宇宙参数组。
    /// `{a : Prop}`（有冒号）是隐式 binder 组。两者都以 `{` 开头，用这个
    /// lookahead 消歧；调用点必须**宇宙参数在前**。
    fn universe_params_ahead(&self) -> bool {
        let toks = &self.tokens;
        let mut i = self.cursor;
        if !matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::LBrace)) {
            return false;
        }
        i += 1;
        if !matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
            return false;
        }
        loop {
            i += 1;
            match toks.get(i).map(|t| &t.kind) {
                Some(TokenKind::Comma) => {
                    if !matches!(toks.get(i + 1).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
                        return false;
                    }
                }
                Some(TokenKind::Ident(_)) => {}
                Some(TokenKind::RBrace) => return true,
                _ => return false,
            }
        }
    }

    /// [`universe_params_ahead`] 的别名：宇宙参数**组**（可连排）。
    fn universe_group_ahead(&self) -> bool {
        self.universe_params_ahead()
    }

    fn parse_universe_params(&mut self) -> Result<Vec<String>> {
        let mut universe = Vec::new();
        self.parse_universe_group(&mut universe)?;
        Ok(universe)
    }

    /// 吃下**一组** `{` 标识符+ `}`，名字并入 `universe`（去重在这里，所以
    /// `{u} {u}` 这种跨组重复也免费被抓到）。不是宇宙组时什么都不做。
    fn parse_universe_group(&mut self, universe: &mut Vec<String>) -> Result<()> {
        if !self.universe_group_ahead() {
            return Ok(());
        }
        self.bump();
        loop {
            let name = self.expect_ident("universe parameter")?;
            if universe.contains(&name) {
                return Err(self.error_here(&format!("duplicate universe parameter `{name}`")));
            }
            universe.push(name);
            match self.peek().kind {
                // 逗号与空格都是分隔符：`{u, v}` 与 `{u v}` 等价（G-14）。
                TokenKind::Ident(_) => {}
                TokenKind::Comma => {
                    self.bump();
                }
                TokenKind::RBrace => {
                    self.bump();
                    return Ok(());
                }
                _ => {
                    return Err(
                        self.error_here("expected `,`, a name or `}` in universe parameters")
                    )
                }
            }
        }
    }

    fn parse_inductive_block(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_decl_name("inductive name")?;
        // 零或多个参数 binder（`(A : Type)` / `(A B : Prop)` / `{A B : Type}`），
        // 在 `:` 之前。走 `push_binders` 与箭头/λ/∀/声明 binder 同一套组语法
        // （docs/design/agent-query-channel.md H6-C / TODO B）；组要求显式类型，
        // 无类型的 `(A)` 仍然是错误。
        let params = self.parse_inductive_binders("inductive 参数")?;
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
                        params,
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

    /// `inductive` 参数与 `ctor` 字段共用的 binder 列表：单名走 `parse_binder`
    /// （保留既有 span 语义），多名字组走 `push_binders` 展开。
    fn parse_inductive_binders(&mut self, what: &str) -> Result<Vec<Binder>> {
        let mut binders = Vec::new();
        loop {
            match self.peek().kind {
                TokenKind::LParen | TokenKind::LBrace if self.named_group_ahead() => {
                    self.push_binders(&mut binders)?;
                }
                TokenKind::LParen | TokenKind::LBrace => {
                    let msg =
                        format!("{what} 需要显式类型，例如 (a : Prop)；不支持无类型的 (a) 写法");
                    return Err(self.error_here(&msg));
                }
                _ => break,
            }
        }
        Ok(binders)
    }

    fn parse_ctor(&mut self) -> Result<CtorDecl> {
        let start = self.bump().span.start;
        let name = self.expect_decl_name("constructor name")?;
        // 字段同样吃 `(A B : Prop)` 组（`parse_binder` 只看 `(`，这里补上 `{`）。
        let binders = self.parse_inductive_binders("constructor 字段")?;
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
        let name = self.expect_decl_name("recursor name")?;
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
        let ctor_name = self.expect_decl_name("constructor name")?;
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
            TokenKind::Ident(kw) if kw == "let" => self.parse_let(),
            TokenKind::Ident(kw) if kw == "match" => self.parse_match(),
            _ => self.parse_arrow(),
        }
    }

    /// Phase 1 值位 `let`：`let x : T := v; body`。`: T` 可省略，缺注解时
    /// 仍产出 `Binder.ty = None`，由 elaborator 报 `elab-untyped-binder`
    ///（与设计 §3.4 一致）。`v`/`body` 都按完整 term 解析（右结合、可嵌套）。
    fn parse_let(&mut self) -> Result<Expr> {
        let start = self.bump().span.start;
        let name_tok = self.peek().clone();
        let TokenKind::Ident(name) = name_tok.kind.clone() else {
            return Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: format!("{:?}", name_tok.kind),
                    expected: "a `let` binder name".to_string(),
                },
                name_tok.span,
                "`let` 后面要跟绑定名，例如 let x : Nat := 1; x".to_string(),
            ));
        };
        self.bump();
        let mut ty = None;
        if self.peek().kind == TokenKind::Colon {
            self.bump();
            ty = Some(Box::new(self.parse_expr()?));
        }
        self.expect_kind(&TokenKind::ColonEq, "`:=` after the let binder")?;
        let val = self.parse_expr()?;
        self.expect_kind(&TokenKind::Semicolon, "`;` after the let value")?;
        let body = self.parse_expr()?;
        let binder_end = ty
            .as_ref()
            .map(|ty| ty.span().end)
            .unwrap_or(name_tok.span.end);
        let span = Span::new(start, body.span().end);
        Ok(Expr::Let {
            binder: Binder {
                name,
                ty,
                style: BinderKind::Explicit,
                span: Span::new(name_tok.span.start, binder_end),
            },
            val: Box::new(val),
            body: Box::new(body),
            span,
        })
    }

    /// `match <scrutinee> with | <Ctor> <binder>... => <body> | ...`（v1）。
    /// 分支顺序任意，前端在降低时按构造子声明序重排；`with` 只是普通标识符，
    /// 由 `scrutinee_depth` 保证它不被吃成 scrutinee 的实参。
    fn parse_match(&mut self) -> Result<Expr> {
        let start = self.bump().span.start;
        self.scrutinee_depth += 1;
        let scrutinee = self.parse_expr();
        self.scrutinee_depth -= 1;
        let scrutinee = scrutinee?;
        let with_tok = self.peek().clone();
        match &with_tok.kind {
            TokenKind::Ident(word) if word == "with" => {
                self.bump();
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticKind::UnexpectedToken {
                        found: format!("{:?}", with_tok.kind),
                        expected: "`with` after the match scrutinee".to_string(),
                    },
                    with_tok.span,
                    "`match` 的 scrutinee 后面要写 `with`，例如 match c with | … => …".to_string(),
                ));
            }
        }
        let mut arms = Vec::new();
        while self.peek().kind == TokenKind::Pipe {
            arms.push(self.parse_match_arm()?);
        }
        if arms.is_empty() {
            return Err(
                self.error_here("`match` 至少需要一条分支，例如 | red => …（`|` 开头的分支）")
            );
        }
        let end = arms
            .last()
            .map(|a| a.span.end)
            .unwrap_or(scrutinee.span().end);
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: Span::new(start, end),
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm> {
        let pipe = self.bump();
        let start = pipe.span.start;
        let pattern = self.parse_pattern()?;
        // `if <guard>`：仅在 arm 里把 `if` 当守卫关键字（设计 §5：不升为全局
        // 关键字，`if` 在别处仍是普通标识符）。
        let guard = if matches!(&self.peek().kind, TokenKind::Ident(k) if k == "if") {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_kind(&TokenKind::FatArrow, "`=>` after the match pattern")?;
        let body = self.parse_expr()?;
        let span = Span::new(start, body.span().end);
        Ok(MatchArm {
            pattern,
            guard,
            body,
            span,
        })
    }

    /// `pattern := atom+`，`atom := '_' | <num> | <ident> | '(' pattern ')'`。
    /// 遇 `if`（守卫）/`=>`/`|` 或任何不能起原子的 token 停止
    /// （`docs/design/match-patterns.md` §2）。
    fn parse_pattern(&mut self) -> Result<Pattern> {
        let mut atoms: Vec<Pattern> = Vec::new();
        loop {
            match self.peek().kind.clone() {
                TokenKind::Ident(name) if name == "if" => break,
                TokenKind::Ident(name) => {
                    let tok = self.bump();
                    atoms.push(if name == "_" {
                        Pattern::Wild { span: tok.span }
                    } else {
                        Pattern::Ident {
                            name,
                            args: Vec::new(),
                            span: tok.span,
                        }
                    });
                }
                TokenKind::Num(value) => {
                    let tok = self.bump();
                    atoms.push(Pattern::Num {
                        value,
                        span: tok.span,
                    });
                }
                TokenKind::LParen => {
                    self.bump();
                    let inner = self.parse_pattern()?;
                    self.expect_kind(&TokenKind::RParen, "`)` after a nested pattern")?;
                    atoms.push(inner);
                }
                _ => break,
            }
        }
        let Some(head) = atoms.first().cloned() else {
            let tok = self.peek().clone();
            return Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: format!("{:?}", tok.kind),
                    expected: "a pattern".to_string(),
                },
                tok.span,
                "`|` 后面要跟模式：构造子（如 `succ k`）、`_`、变量名或数字字面量".to_string(),
            ));
        };
        atoms.remove(0);
        match head {
            Pattern::Ident { name, args, span } if args.is_empty() => Ok(Pattern::Ident {
                name,
                args: atoms,
                span,
            }),
            other if atoms.is_empty() => Ok(other),
            other => Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: "pattern argument".to_string(),
                    expected: "an identifier head".to_string(),
                },
                other.span(),
                "模式要写成「构造子/变量 子模式…」；数字或 `_` 不能带子模式".to_string(),
            )),
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
        // `(let x : T := …)` / `(fun …)` 是表达式，不是多名字 binder 组；
        // `let`/`fun` 恰好也是 Ident，必须在这里让路。
        if matches!(
            toks.get(i).map(|t| &t.kind),
            Some(TokenKind::Ident(name)) if is_expr_keyword(name) || name.as_str() == "fun"
        ) {
            return false;
        }
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
        self.parse_operators(0)
    }

    /// 优先级爬升（G-04 / WO-011，设计 N3）：`+`（内建保留项）与所有已声明的
    /// 记法算子共用一条梯子，位置在 `parse_arrow`（最松）与 `parse_app`
    /// （最紧）之间。
    ///
    /// 结合规则（v1 自定，设计 §6 已知差异 1）：`infix`/`infixl` 的右操作数
    /// 要求 `p + 1`（于是同级连写停止、左结合），`infixr` 的左操作数要求
    /// `p + 1`（右结合）。`infix` 与 `infixl` 在这一层同形；`infix` 的
    /// 「不许连写」由 `parse_notation_operand` 的**同级检查**实现。
    fn parse_operators(&mut self, min_precedence: u16) -> Result<Expr> {
        let mut lhs = self.parse_app()?;
        loop {
            // 未声明符号：报**专用**诊断（hint 给「先声明」与「点名写法」），
            // 而不是让 `parse_file`/`parse_command` 用通用的 unexpected-token
            // 糊过去（设计 N2/N5）。
            if let TokenKind::Sym(symbol) = &self.peek().kind {
                if !self.notations.contains_key(symbol) {
                    let span = self.peek().span;
                    let symbol = symbol.clone();
                    return Err(self.unknown_symbol_error(&symbol, span));
                }
            }
            let Some(op) = self.binary_op_ahead(min_precedence) else {
                break;
            };
            self.bump_operator(&op);
            let next_min = if op.assoc == NotationAssoc::Infixr {
                op.precedence
            } else {
                op.precedence + 1
            };
            let rhs = self.parse_operand(&op, next_min)?;
            let span = Span::new(lhs.span().start, rhs.span().end);
            lhs = if op.builtin_plus {
                Expr::Plus {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                }
            } else {
                Expr::Notation {
                    symbol: op.symbol.clone(),
                    target: op.target.clone(),
                    assoc: op.assoc,
                    lhs: Some(Box::new(lhs)),
                    rhs: Some(Box::new(rhs)),
                    span,
                }
            };
        }
        Ok(lhs)
    }

    /// 右操作数：先按 `min_precedence` 继续爬升，再对**无结合** `infix` 做
    /// 同级连写检查（`a ∈ b ∈ c` 报解析错——v1 不做 Lean 的「需要括号」诊断，
    /// 设计 N3）。
    fn parse_operand(&mut self, op: &BinaryOp, min_precedence: u16) -> Result<Expr> {
        let rhs = self.parse_operators(min_precedence)?;
        if op.assoc == NotationAssoc::Infix {
            if let Some(next) = self.binary_op_ahead(op.precedence) {
                if next.precedence == op.precedence {
                    let s = &op.symbol;
                    let n = &next.symbol;
                    return Err(self.error_here(&format!(
                        "`{s}` 没有结合性：`a {s} b {n} c` 要写成 `(a {s} b) {n} c` 或 `a {s} (b {n} c)`"
                    )));
                }
            }
        }
        Ok(rhs)
    }

    /// 下一个 token 是不是**优先级 ≥ `min_precedence`** 的二元算子。
    /// `None` ⇒ 调用方停止爬升（不是算子，或优先级不够）。
    fn binary_op_ahead(&self, min_precedence: u16) -> Option<BinaryOp> {
        let tok = self.peek();
        let op = match &tok.kind {
            TokenKind::Plus => BinaryOp {
                precedence: PLUS_PRECEDENCE,
                assoc: NotationAssoc::Infixl,
                symbol: "+".to_string(),
                target: "Nat.add".to_string(),
                builtin_plus: true,
            },
            TokenKind::Sym(symbol) => {
                // 未声明符号：**不是**算子（`None`）——留给 `starts_atom`/调用方
                // 报「未声明符号」的专用诊断，而不是在这里假装爬升结束。
                let entry = self.notations.get(symbol)?;
                let precedence = entry.precedence?; // 零元记法不在算子位上
                BinaryOp {
                    precedence,
                    assoc: entry.assoc,
                    symbol: entry.symbol.clone(),
                    target: entry.target.clone(),
                    builtin_plus: false,
                }
            }
            _ => return None,
        };
        (op.precedence >= min_precedence).then_some(op)
    }

    fn bump_operator(&mut self, op: &BinaryOp) {
        let _ = op;
        self.bump();
    }

    /// 零元记法：`Sym(s)` 且已声明为 `Nullary` ⇒ 记号节点。
    fn parse_nullary_notation(&mut self, symbol: &str, tok: &Token) -> Expr {
        let entry = self
            .notations
            .get(symbol)
            .expect("parse_nullary_notation is only called for a declared nullary notation");
        Expr::Notation {
            symbol: entry.symbol.clone(),
            target: entry.target.clone(),
            assoc: NotationAssoc::Nullary,
            lhs: None,
            rhs: None,
            span: tok.span,
        }
    }

    /// 未声明符号的专用诊断（hint 给「先声明」与「点名写法」两条出路）。
    fn unknown_symbol_error(&self, symbol: &str, span: Span) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::NotationUnknownSymbol {
                symbol: symbol.to_string(),
            },
            span,
            format!(
                "符号 `{symbol}` 在本文件里还没有声明过记法；先用 infix/notation 命令声明它，或改用点名写法"
            ),
        )
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
        // tactic 解析期间：换行后的 tactic 关键字是边界，绝不能作为实参吃掉。
        if self.by_depth > 0 && self.next_line_starts_a_tactic() {
            return false;
        }
        match &self.peek().kind {
            TokenKind::Ident(name) => {
                let blocks_match = self.scrutinee_depth > 0 && name == "with";
                !(is_reserved_command(name) || is_expr_keyword(name) || blocks_match)
            }
            TokenKind::Num(_) | TokenKind::Hole | TokenKind::LParen | TokenKind::At => true,
            TokenKind::Forall => true,
            // 记法符号**不得**被当作应用实参：已声明的**零元**记法是一个原子
            // （`f ∅` 合法），二元记法与未声明符号都让路（设计 N2/N3）。
            TokenKind::Sym(symbol) => self
                .notations
                .get(symbol)
                .is_some_and(|entry| entry.assoc == NotationAssoc::Nullary),
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
            // 零元记法（`∅`）：已声明为 Nullary 才是一个原子；二元记法的符号
            // 落在算子位上（`parse_operators` 消费），未声明符号在这里报专用
            // 诊断（**不是** `unknown identifier`，设计 N2/N5）。
            TokenKind::Sym(ref symbol) => match self.notations.get(symbol) {
                Some(entry) if entry.assoc == NotationAssoc::Nullary => {
                    Ok(self.parse_nullary_notation(symbol, &tok))
                }
                Some(_) => Err(self.error_here(&format!(
                    "`{symbol}` 是二元记法符号，两边都要有操作数（例如 a {symbol} b）"
                ))),
                None => Err(self.unknown_symbol_error(symbol, tok.span)),
            },
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
        // body 的第一个 token 若是 `by`，按值位关键字解析——学习者拆完 binder
        // 后直接写 `by` 是主流程，不该被迫把关键字挪到值位开头。降低侧零改动：
        // `split_by_value` 本来就沿 lambda 链下降处理 `by` 节点。
        let body = match &self.peek().kind {
            TokenKind::Ident(kw) if kw == "by" => self.parse_by_block()?,
            _ => self.parse_expr()?,
        };
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

    /// 声明名（def/theorem/axiom/inductive/ctor/rec）：与 [`expect_ident`] 同，
    /// 但**拒绝以 `.` 结尾**的名字。词法层把 `.` 当标识符续接字符（`Foo.Bar`
    /// 是一个 `Ident`），于是 `def f.{u}` 曾把名字静默吃成 `f.` 并判卷通过
    /// （G-18：声明变形且不报错）。本子集不支持声明位 `.{u}` 拼写，所以这里
    /// 必须报错而不是猜；点分限定名（`Foo.Bar.baz`）照常通过。
    fn expect_decl_name(&mut self, what: &str) -> Result<String> {
        let tok = self.peek().clone();
        let name = self.expect_ident(what)?;
        if name.ends_with('.') {
            return Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: format!("{name:?}"),
                    expected: "a declaration name that does not end with `.`".to_string(),
                },
                tok.span,
                format!("声明名不许以 `.` 结尾（这里读到了 `{name}`）；本语言不支持 `f.{{u}}` 声明位拼写，请写成 `def f {{u}} …`"),
            ));
        }
        Ok(name)
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

/// 表达式关键字（term 关键字）：不是命令，但在应用位必须让路——`f let …`
/// 绝不能被当成 `f` 应用到标识符 `let`。
fn is_expr_keyword(name: &str) -> bool {
    matches!(name, "let" | "match")
}

/// `by` 块白名单里的 tactic 关键字（`parse_tactic_inner` 的 `match` 臂与
/// 换行边界判定共用同一集合）。
fn is_tactic_keyword(name: &str) -> bool {
    matches!(
        name,
        "intro" | "exact" | "apply" | "assumption" | "rfl" | "match" | "sorry"
    )
}

fn is_reserved_command(name: &str) -> bool {
    matches!(
        name,
        "import"
            | "def"
            | "theorem"
            | "example"
            | "axiom"
            | "inductive"
            | "ctor"
            | "rec"
            | "iota"
            | "end"
            | "infix"
            | "infixl"
            | "infixr"
            | "notation"
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
    fn axiom_with_decl_binders_parses() {
        // WO-008 / G-13：`axiom` 与 def/theorem 同序（宇宙参数 → binder → `:`）。
        // 表 1/2/3/4 号写法都要折成同一个 Forall 望远镜，binder 的 style 要对。
        let cases: &[(&str, &[(&str, BinderKind)])] = &[
            (
                "axiom Foo (α : Type) : Prop\n",
                &[("α", BinderKind::Explicit)],
            ),
            (
                "axiom Foo (α β : Type) : Prop\n",
                &[("α", BinderKind::Explicit), ("β", BinderKind::Explicit)],
            ),
            (
                "axiom Foo {α : Type} : Prop\n",
                &[("α", BinderKind::Implicit)],
            ),
            (
                "axiom Foo {u} (α : Sort u) : Prop\n",
                &[("α", BinderKind::Explicit)],
            ),
        ];
        for (src, want) in cases {
            let file = parse(src).unwrap_or_else(|e| panic!("{src:?} must parse: {e:?}"));
            let Command::Axiom { universe, ty, .. } = &file.commands[0] else {
                panic!("expected axiom for {src:?}");
            };
            let want_universe: Vec<String> = if src.contains("{u}") {
                vec!["u".to_string()]
            } else {
                Vec::new()
            };
            assert_eq!(universe, &want_universe, "{src:?}");
            let Expr::Forall { binders, body, .. } = ty else {
                panic!("expected Forall type for {src:?}, got {ty:?}");
            };
            let got: Vec<(&str, BinderKind)> = binders
                .iter()
                .map(|b| (b.name.as_str(), b.style.clone()))
                .collect();
            assert_eq!(&got, want, "{src:?}");
            assert!(
                matches!(body.as_ref(), Expr::Sort { .. }),
                "body must be the codomain Sort for {src:?}, got {body:?}"
            );
        }
    }

    #[test]
    fn axiom_decl_binders_match_arrow_style_ast() {
        // 表 1 号与表 7 号必须产出**等价 AST**（只有 span 不同：糖的 Forall span
        // 从 binder 起算）。比较 binder 向量（名字 + style + 类型）与 body。
        let binder_style = parse("axiom Foo (α : Type) : Prop\n").unwrap();
        let arrow_style = parse("axiom Foo : (α : Type) -> Prop\n").unwrap();
        let (Command::Axiom { ty: b_ty, .. }, Command::Axiom { ty: a_ty, .. }) =
            (&binder_style.commands[0], &arrow_style.commands[0])
        else {
            panic!("expected two axioms");
        };
        // 两份源码的排版不同（binder 在名字后 vs 类型位里），span 天然有差 ⇒
        // 归一化成「binder 名字/style」+「body 的排序」，只比结构（判据是 AST，不是文本）。
        fn shape(ty: &Expr) -> (Vec<(String, BinderKind)>, SortKind) {
            let Expr::Forall { binders, body, .. } = ty else {
                panic!("expected Forall, got {ty:?}");
            };
            let named = binders
                .iter()
                .map(|b| (b.name.clone(), b.style.clone()))
                .collect();
            let Expr::Sort { sort, .. } = body.as_ref() else {
                panic!("expected Sort body, got {body:?}");
            };
            (named, sort.clone())
        }
        assert_eq!(shape(b_ty), shape(a_ty));
    }

    #[test]
    fn axiom_untyped_decl_binder_still_errors() {
        // 表 5 号：无类型 binder 仍是 parse 错误，且位置指到 `(`、文案是教学文案。
        let err = parse("axiom Foo (α) : Prop\n").unwrap_err();
        assert!(
            err.message.contains("显式类型"),
            "teaching message expected, got {err:?}"
        );
        assert_eq!(err.span.start.line, 1);
        assert_eq!(err.span.start.column, 11);
    }

    #[test]
    fn def_cannot_end_with_a_dot() {
        // WO-009 表 20 行 / G-18：`def f.{u}` 不能被静默解析成名字 `f.`
        // （那会让声明名变形且不报错）；本子集不支持声明位 `.{u}` 拼写。
        let err = parse("def f.{u} (α : Sort u) : α -> α := fun a => a\n").unwrap_err();
        assert!(
            err.message.contains("声明名") && err.message.contains(".{u}"),
            "got {err:?}"
        );
        assert_eq!(err.span.start.column, 5);
        // 正常的点分名字（命名空间限定）不许被误伤。
        parse("def Foo.Bar.baz : Prop := Prop\n").expect("dotted qualified name must parse");
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
    fn universe_params_accept_space_separated_names() {
        // WO-009 表 3 行：`{u v}`（同组空格分隔）与 `{u, v}` 产出等价 AST。
        let spaced = parse("def f {u v} (α : Sort u) (β : Sort v) (a : α) : α := a\n").unwrap();
        let comma = parse("def f {u, v} (α : Sort u) (β : Sort v) (a : α) : α := a\n").unwrap();
        let (
            Command::Def { universe, ty, .. },
            Command::Def {
                universe: c_universe,
                ty: c_ty,
                ..
            },
        ) = (&spaced.commands[0], &comma.commands[0])
        else {
            panic!("expected two defs");
        };
        assert_eq!(universe, &["u".to_string(), "v".to_string()]);
        assert_eq!(c_universe, universe);
        let (
            Expr::Forall { binders, .. },
            Expr::Forall {
                binders: c_binders, ..
            },
        ) = (ty, c_ty)
        else {
            panic!("expected Forall on both sides");
        };
        assert_eq!(binders.len(), 3);
        // `{u v}` 与 `{u, v}` 的源码排版差一个逗号 ⇒ span 有差，按名字/style 归一化。
        let named = |bs: &[Binder]| -> Vec<(String, BinderKind)> {
            bs.iter()
                .map(|b| (b.name.clone(), b.style.clone()))
                .collect()
        };
        assert_eq!(named(binders), named(c_binders));
    }

    #[test]
    fn universe_params_accept_repeated_brace_groups() {
        // WO-009 表 4 行：`{u} {v}`（连排两组）也并进同一份 universe。
        let file = parse("def f {u} {v} : Prop := Prop\n").unwrap();
        let Command::Def { universe, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        assert_eq!(universe, &["u".to_string(), "v".to_string()]);
    }

    #[test]
    fn duplicate_universe_param_across_groups_is_rejected() {
        // WO-009 表 5 行：去重必须跨组生效。
        let err = parse("def f {u} {u} (x : Prop) : Prop := x\n").unwrap_err();
        assert!(
            err.message.contains("duplicate universe parameter"),
            "err: {err:?}"
        );
    }

    #[test]
    fn named_group_with_colon_is_still_a_binder() {
        // 有冒号 → 隐式 binder 组，永远不许被当成宇宙参数（回归钉子）。
        let file = parse("def f {α β : Type} (x : Prop) : Prop := x\n").unwrap();
        let Command::Def { universe, ty, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        assert!(universe.is_empty(), "{universe:?}");
        let Expr::Forall { binders, .. } = ty else {
            panic!("expected Forall");
        };
        assert_eq!(binders.len(), 3);
        assert_eq!(binders[0].style, BinderKind::Implicit);
        // 宇宙组后接隐式 binder 组的混排（表 6 行）。
        let file = parse("def f {u} {α : Type} (a : α) : α := a\n").unwrap();
        let Command::Def { universe, ty, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        assert_eq!(universe, &["u".to_string()]);
        let Expr::Forall { binders, .. } = ty else {
            panic!("expected Forall");
        };
        assert_eq!(binders.len(), 2);
    }

    #[test]
    fn example_cannot_declare_universe_params() {
        // WO-009 表 16 行：example 没有 universe 字段 ⇒ 必须报明确文案，不许静默吞。
        let err = parse("example {u v} (α : Sort u) (a : α) : α := a\n").unwrap_err();
        assert!(err.message.contains("不支持宇宙参数"), "got {err:?}");
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
    fn inductive_block_parses_parameters() {
        let src = "\
inductive Option (A : Type) : Type
ctor none : Option A
ctor some (a : A) : Option A
end
";
        let file = parse(src).unwrap();
        let Command::InductiveBlock {
            name,
            params,
            constructors,
            ..
        } = &file.commands[0]
        else {
            panic!("expected InductiveBlock");
        };
        assert_eq!(name, "Option");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "A");
        assert!(matches!(params[0].style, BinderKind::Explicit));
        // 字段只含 ctor 自己的 binder，不含 params。
        assert_eq!(constructors.len(), 2);
        assert_eq!(constructors[0].name, "none");
        assert_eq!(constructors[0].binders.len(), 0);
        assert_eq!(constructors[1].name, "some");
        assert_eq!(constructors[1].binders.len(), 1);
        assert_eq!(constructors[1].binders[0].name, "a");
    }

    #[test]
    fn inductive_block_parses_implicit_parameter() {
        let src = "\
inductive Box {A : Type} : Type
ctor mk (a : A) : Box
end
";
        let file = parse(src).unwrap();
        let Command::InductiveBlock { params, .. } = &file.commands[0] else {
            panic!("expected InductiveBlock");
        };
        assert_eq!(params.len(), 1);
        assert!(matches!(params[0].style, BinderKind::Implicit));
    }

    /// Pi/箭头位早就支持 `(A B : Prop)`（`parse_binder_group` +
    /// `push_binders`）；inductive 参数与 ctor 字段走的是单名 `parse_binder`，
    /// 于是同一写法在这里解析失败（docs/design/agent-query-channel.md H6-C / TODO B）。
    #[test]
    fn inductive_block_parses_multi_name_parameter_group() {
        let src = "\
inductive Or (A B : Prop) : Prop
ctor inl (a : A) : Or A B
ctor inr (b : B) : Or A B
end
";
        let file = parse(src).unwrap();
        let Command::InductiveBlock { params, .. } = &file.commands[0] else {
            panic!("expected InductiveBlock");
        };
        assert_eq!(params.len(), 2, "one Binder per name in the group");
        assert_eq!(params[0].name, "A");
        assert_eq!(params[1].name, "B");
        assert!(matches!(params[0].style, BinderKind::Explicit));
        assert!(matches!(params[1].style, BinderKind::Explicit));
        assert!(
            params.iter().all(|p| p.ty.is_some()),
            "every expanded binder carries the group's shared type"
        );
    }

    #[test]
    fn inductive_block_parses_implicit_multi_name_parameter_group() {
        let src = "\
inductive Pair {A B : Type} : Type
ctor mk (a : A) (b : B) : Pair
end
";
        let file = parse(src).unwrap();
        let Command::InductiveBlock { params, .. } = &file.commands[0] else {
            panic!("expected InductiveBlock");
        };
        assert_eq!(params.len(), 2);
        assert!(params
            .iter()
            .all(|p| matches!(p.style, BinderKind::Implicit)));
    }

    #[test]
    fn ctor_parses_multi_name_field_group() {
        let src = "\
inductive Both : Prop
ctor mk (A B : Prop) (a : A) (b : B) : Both
end
";
        let file = parse(src).unwrap();
        let Command::InductiveBlock { constructors, .. } = &file.commands[0] else {
            panic!("expected InductiveBlock");
        };
        assert_eq!(
            constructors[0].binders.len(),
            4,
            "A, B (one Binder per name in the group) + a + b"
        );
        assert_eq!(constructors[0].binders[0].name, "A");
        assert_eq!(constructors[0].binders[1].name, "B");
        assert_eq!(constructors[0].binders[2].name, "a");
        assert_eq!(constructors[0].binders[3].name, "b");
    }

    /// 组语法要求每个名字都有类型：`(A)` 仍然必须是解析错误（不能静默变成
    /// 无类型 binder）。
    #[test]
    fn inductive_untyped_parameter_group_is_still_rejected() {
        let src = "\
inductive Bad (A) : Prop
ctor mk : Bad A
end
";
        assert!(parse(src).is_err(), "`(A)` without a type must not parse");
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

    #[test]
    fn let_parses_binder_value_body_and_span() {
        let file =
            parse("def two : Nat := let one : Nat := Nat.succ Nat.zero; one + one\n").unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::Let {
            binder,
            val: let_val,
            body,
            span,
        } = val
        else {
            panic!("expected Let, got {val:?}");
        };
        assert_eq!(binder.name, "one");
        assert_eq!(binder.style, BinderKind::Explicit);
        assert!(matches!(
            binder.ty.as_deref(),
            Some(Expr::Ident { name, .. }) if name == "Nat"
        ));
        assert!(matches!(let_val.as_ref(), Expr::App { .. }));
        assert!(matches!(body.as_ref(), Expr::Plus { .. }));
        assert_eq!(span.end.offset, body.span().end.offset);
        assert!(span.start.offset < binder.span.start.offset);
    }

    #[test]
    fn let_without_annotation_keeps_none_binder_type() {
        let file = parse("#check (let x := Nat.zero; x)\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        let Expr::Let { binder, .. } = expr else {
            panic!("expected Let, got {expr:?}");
        };
        assert_eq!(binder.name, "x");
        assert!(
            binder.ty.is_none(),
            "missing annotation stays None for elab"
        );
    }

    #[test]
    fn let_nests_right_associatively() {
        let file = parse("#check (let a : Nat := 0; let b : Nat := 1; a)\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        let Expr::Let { body, .. } = expr else {
            panic!("expected outer Let, got {expr:?}");
        };
        assert!(
            matches!(body.as_ref(), Expr::Let { .. }),
            "body should be a nested Let: {body:?}"
        );
    }

    #[test]
    fn let_parses_in_fun_body_and_parentheses() {
        let file = parse("def d : Nat := fun (x : Nat) => let y : Nat := x; y\n").unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::Lambda { body, .. } = val else {
            panic!("expected Lambda, got {val:?}");
        };
        assert!(matches!(body.as_ref(), Expr::Let { .. }));
        let file2 = parse("#check (let x : Nat := 1; x)\n").unwrap();
        assert!(
            matches!(&file2.commands[0], Command::Check { expr, .. } if matches!(expr, Expr::Let { .. }))
        );
    }

    #[test]
    fn let_missing_semicolon_is_a_parse_error() {
        let err = parse("def x : Nat := let y : Nat := 1\n").unwrap_err();
        assert!(err.message.contains(";"), "err: {err:?}");
    }

    #[test]
    fn let_missing_assign_is_a_parse_error() {
        let err = parse("#check (let y : Nat; y)\n").unwrap_err();
        assert!(err.message.contains(":="), "err: {err:?}");
    }

    #[test]
    fn let_without_binder_name_is_a_parse_error() {
        let err = parse("#check (let 3 : Nat := 3; 3)\n").unwrap_err();
        assert!(err.message.contains("let"), "err: {err:?}");
    }

    #[test]
    fn let_does_not_start_an_application_argument() {
        // `f let …`：`let` 必须让路成 term 关键字，而不是被吃成 `f` 的实参。
        let err = parse("def x : Nat := Nat.succ let y : Nat := 1; y\n").unwrap_err();
        assert!(err.message.contains("command"), "err: {err:?}");
    }

    #[test]
    fn match_parses_scrutinee_arms_binders_and_span() {
        let src = "def f : Color := match c with | red => green | pair x y => x\n";
        let file = parse(src).unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::Match {
            scrutinee,
            arms,
            span,
        } = val
        else {
            panic!("expected Match, got {val:?}");
        };
        assert!(matches!(scrutinee.as_ref(), Expr::Ident { name, .. } if name == "c"));
        assert_eq!(arms.len(), 2);
        let Pattern::Ident { name, args, .. } = &arms[0].pattern else {
            panic!("expected a constructor pattern, got {:?}", arms[0].pattern);
        };
        assert_eq!(name, "red");
        assert!(args.is_empty());
        assert!(matches!(&arms[0].body, Expr::Ident { name, .. } if name == "green"));
        let Pattern::Ident { name, args, .. } = &arms[1].pattern else {
            panic!("expected a constructor pattern");
        };
        assert_eq!(name, "pair");
        assert_eq!(
            args.iter()
                .map(|p| match p {
                    Pattern::Ident { name, .. } => name.as_str(),
                    other => panic!("expected a bind, got {other:?}"),
                })
                .collect::<Vec<_>>(),
            vec!["x", "y"]
        );
        assert!(matches!(&arms[1].body, Expr::Ident { name, .. } if name == "x"));
        assert_eq!(span.end.offset, arms[1].span.end.offset);
    }

    #[test]
    fn match_nests_in_a_branch_body() {
        let src = "def f : Color := match c with | red => match d with | blue => green | green => blue | blue => red\n";
        let file = parse(src).unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::Match { arms, .. } = val else {
            panic!("expected outer Match, got {val:?}");
        };
        // 贪心：内层 match 吃掉后续的 `|` 分支，外层只剩第一条。
        assert_eq!(arms.len(), 1);
        assert!(
            matches!(&arms[0].body, Expr::Match { .. }),
            "branch body should be a nested match: {:?}",
            arms[0].body
        );
    }

    #[test]
    fn match_is_a_tactic_in_a_by_block() {
        let src = "def swap (c : Color) : Color := by match c with | red => green | green => red\n";
        let file = parse(src).unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        // `def f (c : Color) : T := …` sugar folds the declared binder into a
        // Lambda, so the `by` block sits in the lambda body.
        let expr = match val {
            Expr::Lambda { body, .. } => body.as_ref(),
            other => other,
        };
        let Expr::By { tactics, .. } = expr else {
            panic!("expected a by block, got {expr:?}");
        };
        assert_eq!(tactics.len(), 1);
        let Tactic::Exact { expr, .. } = &tactics[0] else {
            panic!("`match` must lower to Exact: {:?}", tactics[0]);
        };
        assert!(matches!(expr, Expr::Match { .. }), "got {expr:?}");
    }

    #[test]
    fn match_missing_with_is_a_parse_error() {
        let err = parse("def f : Color := match c | red => green\n").unwrap_err();
        assert!(err.message.contains("with"), "err: {err:?}");
    }

    #[test]
    fn match_without_arms_is_a_parse_error() {
        let err = parse("def f : Color := match c with\n").unwrap_err();
        assert!(
            err.message.contains("分支") || err.message.contains("|"),
            "err: {err:?}"
        );
    }

    #[test]
    fn match_missing_fat_arrow_is_a_parse_error() {
        let err = parse("def f : Color := match c with | red green\n").unwrap_err();
        assert!(err.message.contains("=>"), "err: {err:?}");
    }

    #[test]
    fn match_does_not_start_an_application_argument() {
        // `f match …`：`match` 必须让路成 term 关键字。
        let err = parse("def x : Color := red match c with | red => green\n").unwrap_err();
        assert!(!err.message.is_empty(), "err: {err:?}");
    }

    /// 取出首个 `def` 值位 `by` 块里的 tactics（无声明 binder 时直接是 By）。
    fn by_tactics(src: &str) -> Vec<Tactic> {
        let file = parse(src).unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let val = match val {
            Expr::Lambda { body, .. } => body.as_ref(),
            other => other,
        };
        let Expr::By { tactics, .. } = val else {
            panic!("expected a by block, got {val:?}");
        };
        tactics.clone()
    }

    #[test]
    fn by_block_newlines_separate_tactics() {
        let tactics = by_tactics("def t : Prop := by\n  intro a\n  exact a\n  assumption\n");
        assert_eq!(tactics.len(), 3, "tactics: {tactics:?}");
        assert!(matches!(tactics[0], Tactic::Intro { .. }));
        assert!(matches!(tactics[1], Tactic::Exact { .. }));
        assert!(matches!(tactics[2], Tactic::Assumption { .. }));
    }

    #[test]
    fn by_block_newline_boundary_beats_application() {
        // `exact f` 换行 `apply g`：`apply` 是 tactic 关键字，不能吃成 f 的实参。
        let tactics = by_tactics("def t : Prop := by\n  exact f\n  apply g\n");
        assert_eq!(tactics.len(), 2, "tactics: {tactics:?}");
        let Tactic::Exact { expr, .. } = &tactics[0] else {
            panic!("expected Exact: {:?}", tactics[0]);
        };
        assert!(
            matches!(expr, Expr::Ident { name, .. } if name == "f"),
            "`exact f` must not absorb the next tactic: {expr:?}"
        );
        assert!(matches!(tactics[1], Tactic::Apply { .. }));
    }

    #[test]
    fn by_block_multiline_application_is_one_tactic() {
        // 续行以非关键字开头时仍是同一个应用（一个 tactic）。
        let tactics = by_tactics("def t : Prop := by\n  exact f\n    a\n    b\n  assumption\n");
        assert_eq!(tactics.len(), 2, "tactics: {tactics:?}");
        let Tactic::Exact { expr, .. } = &tactics[0] else {
            panic!("expected Exact: {:?}", tactics[0]);
        };
        assert!(
            matches!(expr, Expr::App { .. }),
            "multi-line application must stay one Exact: {expr:?}"
        );
        assert!(matches!(tactics[1], Tactic::Assumption { .. }));
    }

    #[test]
    fn by_block_semicolons_still_work_and_mix_with_newlines() {
        let tactics = by_tactics("def t : Prop := by intro a; exact a\n  assumption\n");
        assert_eq!(tactics.len(), 3, "tactics: {tactics:?}");
        assert!(matches!(tactics[0], Tactic::Intro { .. }));
        assert!(matches!(tactics[1], Tactic::Exact { .. }));
        assert!(matches!(tactics[2], Tactic::Assumption { .. }));
    }

    #[test]
    fn by_block_does_not_consume_the_next_command() {
        // by 块收尾后必须停下，不能把 `#check` 也吞进去/当成实参。
        let file = parse("def t : Prop := by\n  exact a\n#check t\n").unwrap();
        assert_eq!(file.commands.len(), 2, "commands: {:?}", file.commands);
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::By { tactics, .. } = val else {
            panic!("expected by block, got {val:?}");
        };
        assert_eq!(tactics.len(), 1);
        assert!(matches!(&file.commands[1], Command::Check { .. }));
    }

    // ---- `import`（I16 P1：语法 + 置顶规则 + 模块名校验）------------------

    #[test]
    fn parses_a_leading_import_with_a_dotted_module_name() {
        let file = parse("import Lesson.Logic\n\ndef x : Nat := 1\n").expect("parses");
        assert_eq!(file.commands.len(), 2);
        let Command::Import { module, span } = &file.commands[0] else {
            panic!("expected import, got {:?}", file.commands[0]);
        };
        assert_eq!(module, "Lesson.Logic");
        assert_eq!(span.start.line, 1);
        assert_eq!(file.commands[1].import_module(), None);
    }

    #[test]
    fn parses_several_imports_before_declarations() {
        let file = parse("import A\nimport B.C\ndef x : Nat := 1\n").expect("parses");
        assert_eq!(file.commands.len(), 3);
        assert!(file.commands[0].is_import());
        assert!(file.commands[1].is_import());
        assert!(!file.commands[2].is_import());
    }

    #[test]
    fn import_after_a_declaration_is_rejected() {
        let err = parse("def x : Nat := 1\nimport A\n").expect_err("late import");
        assert_eq!(err.kind, DiagnosticKind::ImportMustPrecedeDeclarations);
        assert_eq!(
            err.span.start.line, 2,
            "the error points at the import line"
        );
        assert!(err.hint().contains("任何声明之前"));
    }

    #[test]
    fn import_without_a_name_is_malformed() {
        let err = parse("import\n").expect_err("missing module name");
        assert_eq!(err.code(), "import-malformed");
    }

    #[test]
    fn import_with_trailing_tokens_on_the_same_line_is_malformed() {
        let err = parse("import A B\n").expect_err("one module per line");
        assert_eq!(err.code(), "import-malformed");
        assert!(err.message.contains("一行只能写一个模块名"));
    }

    #[test]
    fn import_of_an_invalid_module_name_carries_the_module_hint() {
        let err = parse("import Foo.\n").expect_err("empty component");
        assert_eq!(err.code(), "import-not-a-valid-module-name");
        assert!(err.hint().contains("点分"), "hint: {}", err.hint());

        let err = parse("import 1Foo\n").expect_err("digit start");
        assert_eq!(err.code(), "import-not-a-valid-module-name");
        assert!(err.hint().contains("数字"), "hint: {}", err.hint());
    }

    #[test]
    fn import_name_with_a_dash_is_rejected_with_the_dash_hint() {
        // `-` 不是标识符续接字符，词法层就会拒绝；它必须仍给出 import 专用的
        // 教学提示，而不是通用的 "expected `->` or `--`"。
        let err = parse("import unit1-propositions-proofs\n").expect_err("dash");
        assert_eq!(err.code(), "import-not-a-valid-module-name");
        assert!(
            err.hint().contains("`-` 不是模块名字符"),
            "hint: {}",
            err.hint()
        );
    }

    #[test]
    fn dash_outside_an_import_line_keeps_the_generic_hint() {
        let err = parse("def x : Nat := 1 - 2\n").expect_err("dash in an expression");
        assert_eq!(err.code(), "unexpected-token");
        assert!(!err.hint().contains("模块名"));
    }

    #[test]
    fn import_is_a_reserved_command_in_expression_position() {
        // 与其它命令关键字同一条规则：表达式位出现 `import` 必须报
        // "command keyword ... cannot appear inside an expression"，
        // 这样 `def x : Nat := 1\nimport A` 里的 `import` 不会被吞进上一个表达式
        // （否则"import 必须置顶"的诊断根本触发不了）。
        let err = parse("def x : Nat := import\n").expect_err("import in a value");
        assert_eq!(err.code(), "unexpected-token");
        assert!(
            err.message.contains("cannot appear inside an expression"),
            "message: {}",
            err.message
        );
        // 名字位与其它命令关键字一样宽松（`def theorem := …` 今天也合法）：
        // 这里刻意只钉住"表达式位被拒"这一条语义。
        assert!(parse("def theorem : Nat := 1\n").is_ok());
    }

    // ---- 记法（G-04 / WO-011，docs/design/notation-subset.md §2）----------

    /// 首个命令是记法命令时的 `(symbol, precedence, assoc, target)`。
    fn notation_of(src: &str) -> (String, Option<u16>, NotationAssoc, String) {
        let file = parse(src).unwrap_or_else(|e| panic!("parse {src}: {e:?}"));
        let Command::Notation {
            symbol,
            precedence,
            assoc,
            target,
            ..
        } = &file.commands[0]
        else {
            panic!("expected a notation command, got {:?}", file.commands[0]);
        };
        (symbol.clone(), *precedence, *assoc, target.clone())
    }

    #[test]
    fn notation_commands_parse_into_one_ast_node_each() {
        assert_eq!(
            notation_of("infix:50 \" ∈ \" => Set.mem\n"),
            (
                "∈".to_string(),
                Some(50),
                NotationAssoc::Infix,
                "Set.mem".into()
            ),
            "the symbol is the trimmed string literal"
        );
        assert_eq!(
            notation_of("infixl:65 \" ∪ \" => Set.union\n"),
            (
                "∪".to_string(),
                Some(65),
                NotationAssoc::Infixl,
                "Set.union".into()
            )
        );
        assert_eq!(
            notation_of("infixr:80 \" '' \" => Set.image\n"),
            (
                "''".to_string(),
                Some(80),
                NotationAssoc::Infixr,
                "Set.image".into()
            )
        );
        assert_eq!(
            notation_of("notation \"∅\" => Set.empty\n"),
            (
                "∅".to_string(),
                None,
                NotationAssoc::Nullary,
                "Set.empty".into()
            ),
            "a nullary notation writes no precedence"
        );
    }

    #[test]
    fn notation_command_shape_errors_are_dedicated_diagnostics() {
        // 缺优先级（`infix` 族必填）。
        let err = parse("infix \" ∈ \" => Set.mem\n").expect_err("missing precedence");
        assert_eq!(err.code(), "notation-shape");
        assert!(err.hint().contains("infix:50"), "hint: {}", err.hint());
        // 缺 `=>`。
        let err = parse("infix:50 \" ∈ \" Set.mem\n").expect_err("missing arrow");
        assert_eq!(err.code(), "notation-shape");
        // 符号是标识符词：它会被当普通标识符读走，记法永远不可能命中。
        let err = parse("infix:50 \"in\" => Set.mem\n").expect_err("ident symbol");
        assert_eq!(err.code(), "notation-shape");
        assert!(err.message.contains("标识符"), "message: {}", err.message);
        // 符号是空串。
        let err = parse("notation \"\" => Set.empty\n").expect_err("empty symbol");
        assert_eq!(err.code(), "notation-shape");
        // 优先级越界（合法范围 1–1000，设计 N1）。
        let err = parse("infix:0 \" ∈ \" => Set.mem\n").expect_err("precedence 0");
        assert_eq!(err.code(), "notation-shape");
        let err = parse("infix:1001 \" ∈ \" => Set.mem\n").expect_err("precedence 1001");
        assert_eq!(err.code(), "notation-shape");
        // `notation` 不接优先级（设计 N1 / 已知差异 4）。
        let err = parse("notation:max \"∅\" => Set.empty\n").expect_err("notation:N");
        assert_eq!(err.code(), "notation-shape");
    }

    #[test]
    fn duplicate_notation_symbol_is_rejected() {
        // v1 自定：同一文件里重复声明同一符号是错误（Lean 允许重载；
        // 设计「已知差异 6」）。两条命令在**同一个** `parse` 里。
        let err = parse(
            "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ∈ \" => mem\n\
             infix:60 \" ∈ \" => mem\n",
        )
        .expect_err("duplicate symbol");
        assert_eq!(err.code(), "notation-shape");
        assert!(err.message.contains("∈"), "message: {}", err.message);
    }

    #[test]
    fn notation_symbol_is_reserved_in_expression_position() {
        // `infix` 是命令关键字：表达式位出现必须让路（与 import 同规则）。
        let err = parse("def x : Nat := infix\n").expect_err("command in a value");
        assert_eq!(err.code(), "unexpected-token");
        assert!(
            err.message.contains("cannot appear inside an expression"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn undeclared_symbol_is_a_dedicated_diagnostic() {
        // 文件里没有 `infix … " ∈ "` ⇒ 专用诊断（**不是** unknown identifier）。
        let err =
            parse("def p (a : Prop) (A : Prop) : Prop := a ∈ A\n").expect_err("undeclared symbol");
        assert_eq!(err.code(), "notation-unknown-symbol");
        assert!(err.message.contains("∈"), "message: {}", err.message);
        assert!(
            err.hint().contains("点名写法"),
            "the hint teaches the pointful spelling: {}",
            err.hint()
        );
    }

    #[test]
    fn notation_is_file_local_and_declaration_order_matters() {
        // 记法只在**声明之后**可用（N5）：前面的使用是未声明符号。
        let err = parse(
            "def p (a : Prop) (A : Prop) : Prop := a ∈ A\n\
             infix:50 \" ∈ \" => p\n",
        )
        .expect_err("use before declaration");
        assert_eq!(err.code(), "notation-unknown-symbol");
    }

    #[test]
    fn notation_precedence_ladder_binds_tighter_than_arrow() {
        // `A ⊆ B -> C` ⇒ `(A ⊆ B) -> C`（50 紧于 `->`）。
        let file = parse(
            "def subset : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ⊆ \" => subset\n\
             def use : Prop := (A ⊆ B) -> C\n",
        )
        .unwrap();
        let Command::Def { val, .. } = &file.commands[2] else {
            panic!("expected def");
        };
        let Expr::Arrow { domain, .. } = val else {
            panic!("expected an arrow at the top, got {val:?}");
        };
        assert!(
            matches!(&**domain, Expr::Notation { symbol, .. } if symbol == "⊆"),
            "the notation must be the arrow's domain: {domain:?}"
        );
    }

    #[test]
    fn notation_precedence_ladder_orders_two_notations() {
        // `∈`=50、`∪`=65 ⇒ `a ∈ A ∪ B` 解析成 `a ∈ (A ∪ B)`。
        let file = parse(
            "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             def uni : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ∈ \" => mem\n\
             infixl:65 \" ∪ \" => uni\n\
             def use : Prop := a ∈ A ∪ B\n",
        )
        .unwrap();
        let Command::Def { val, .. } = &file.commands[4] else {
            panic!("expected def");
        };
        let Expr::Notation { symbol, rhs, .. } = val else {
            panic!("expected the outer notation, got {val:?}");
        };
        assert_eq!(symbol, "∈");
        assert!(
            matches!(rhs.as_deref(), Some(Expr::Notation { symbol, .. }) if symbol == "∪"),
            "the tighter `∪` must be the right operand: {rhs:?}"
        );
    }

    #[test]
    fn non_associative_infix_rejects_chaining() {
        // `infix` 无结合：`a ∈ b ∈ c` 报解析错（设计 N3）。
        let err = parse(
            "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ∈ \" => mem\n\
             def use : Prop := a ∈ b ∈ c\n",
        )
        .expect_err("non-associative chaining");
        assert_eq!(err.code(), "unexpected-token");
    }

    #[test]
    fn infixl_is_left_associative() {
        // `infixl` 左结合：`a ∪ b ∪ c` ⇒ `(a ∪ b) ∪ c`。
        let file = parse(
            "def uni : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infixl:65 \" ∪ \" => uni\n\
             def use : Prop := a ∪ b ∪ c\n",
        )
        .unwrap();
        let Command::Def { val, .. } = &file.commands[2] else {
            panic!("expected def");
        };
        let Expr::Notation { lhs, .. } = val else {
            panic!("expected the outer notation, got {val:?}");
        };
        assert!(
            matches!(lhs.as_deref(), Some(Expr::Notation { symbol, .. }) if symbol == "∪"),
            "left association puts the inner notation on the left: {lhs:?}"
        );
    }

    #[test]
    fn infixr_is_right_associative() {
        // `infixr` 右结合：`a ⋃ b ⋃ c` ⇒ `a ⋃ (b ⋃ c)`。
        //
        // 注：这里刻意**不用** Lean Mathlib 的 `''`——`'` 是今天的标识符续接
        // 字符（`token.rs` 的 `is_ident_continue`），它不在数学符号码点类里，
        // 所以 `''` 是**第二刀**的事（设计 §7）。v1 只验证 `infixr` 这条**结合
        // 规则**本身。
        let file = parse(
            "def uni : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infixr:80 \" ⋃ \" => uni\n\
             def use : Prop := a ⋃ b ⋃ c\n",
        )
        .unwrap();
        let Command::Def { val, .. } = &file.commands[2] else {
            panic!("expected def");
        };
        let Expr::Notation { rhs, .. } = val else {
            panic!("expected the outer notation, got {val:?}");
        };
        assert!(
            matches!(rhs.as_deref(), Some(Expr::Notation { symbol, .. }) if symbol == "⋃"),
            "right association puts the inner notation on the right: {rhs:?}"
        );
    }

    #[test]
    fn nullary_notation_parses_as_a_notation_node() {
        let file = parse(
            "def empty : Prop := False\n\
             notation \"∅\" => empty\n\
             def use : Prop := ∅\n",
        )
        .unwrap();
        let Command::Def { val, .. } = &file.commands[2] else {
            panic!("expected def");
        };
        assert!(
            matches!(val, Expr::Notation { symbol, lhs, rhs, .. }
                if symbol == "∅" && lhs.is_none() && rhs.is_none()),
            "a nullary notation has no operands: {val:?}"
        );
    }

    #[test]
    fn plus_keeps_its_exact_ast_and_relative_precedence() {
        // 回归：`+` 仍是 `Expr::Plus`，且与 `->`/应用的相对优先级不变。
        let file = parse("def n : Nat := 1 + 1 + 1\n").unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::Plus { lhs, .. } = val else {
            panic!("`+` must stay Expr::Plus, got {val:?}");
        };
        assert!(
            matches!(&**lhs, Expr::Plus { .. }),
            "`+` stays left-associative: {lhs:?}"
        );
        // `a -> b + c` ⇒ `a -> (b + c)`：`+` 紧于 `->`（parser-only 检查，
        // 这里只钉分组，不要求这个表达式类型正确）。
        let file = parse("def t : Prop := a -> b + c\n").unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        let Expr::Arrow { codomain, .. } = val else {
            panic!("expected an arrow, got {val:?}");
        };
        assert!(
            matches!(&**codomain, Expr::Plus { .. }),
            "`+` must bind tighter than `->`: {codomain:?}"
        );
        // `1 + 1 + 1` 的 AST 与今天逐字节相同（仍 `Expr::Plus`，左结合）。
        let file = parse("def m : Nat := 1 + 1 + 1\n").unwrap();
        let Command::Def { val, .. } = &file.commands[0] else {
            panic!("expected def");
        };
        assert!(
            matches!(val, Expr::Plus { lhs, .. } if matches!(&**lhs, Expr::Plus { .. })),
            "`+` stays Expr::Plus and left-associative: {val:?}"
        );
    }

    #[test]
    fn notation_symbol_does_not_start_an_application_argument() {
        // 未声明符号绝不能被 `parse_app` 吃成实参（否则诊断会指向错的地方）。
        let err = parse("def f : Prop := g ∈ h\n").expect_err("symbol as argument");
        assert_eq!(err.code(), "notation-unknown-symbol");
    }

    #[test]
    fn notation_command_still_counts_as_a_declaration_for_import_ordering() {
        // 记法命令是「非 import 命令」⇒ 它之后的 `import` 仍报「必须置顶」
        // （既有行为，钉住）。
        let err = parse(
            "def mem : Prop -> Prop -> Prop := fun (a : Prop) => fun (b : Prop) => a\n\
             infix:50 \" ∈ \" => mem\n\
             import Foo\n",
        )
        .expect_err("import after a notation command");
        assert_eq!(err.code(), "import-must-precede-declarations");
    }
}
