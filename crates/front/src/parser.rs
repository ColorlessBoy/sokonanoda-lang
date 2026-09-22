//! 递归下降解析器：tokens → AST（命令与表达式）。

use super::ast::{
    Binder, BinderKind, CasesArm, Command, CtorDecl, Expr, FolFile, HaveValue, IotaRule, MatchArm,
    NotationAssoc, NotationDecl, OpenFilter, Pattern, RecDecl, SortKind, Tactic,
};
use super::diagnostic::{Diagnostic, DiagnosticKind, Result};
use super::span::Span;
use super::token::{
    scan_notation_symbols, tokenize_with_symbols, Token, TokenKind, NOTATION_COMMANDS,
};
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

/// 一条已声明的记法（`infix` 族 / 一元 / 零元 / binder）。
#[derive(Debug, Clone)]
struct NotationEntry {
    symbol: String,
    /// 零元与 binder 记法为 `None`。
    precedence: Option<u16>,
    assoc: NotationAssoc,
    /// 点名目标（`Set.mem`）。解析期只记下来，elaborate 期才解析。
    target: String,
}

/// 一条 `scoped` 声明（第三刀 §12.3）：**默认不生效**，等
/// `open scoped <scope>` 把它搬进 [`Parser::notations`]。
#[derive(Debug, Clone)]
struct ScopedNotation {
    scope: String,
    entry: NotationEntry,
}

/// 表达式位上的一个二元算子：内建 `+` 或一条已声明的记法。
#[derive(Debug, Clone)]
struct BinaryOp {
    precedence: u16,
    assoc: NotationAssoc,
    symbol: String,
    /// `true` ⇒ 产出 `Expr::Plus`（内建保留项），否则产出 `Expr::Notation`。
    /// 目标候选表在构造 `Expr::Notation` 时由 `notation_node` 现取（唯一来源）。
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
    /// **Lean 的 `@`（IA-1）**：`parse_atom` 见到 `@` 置位，`parse_app` 取走
    /// 并把它写进这条脊的每个 `Expr::App` 节点（`explicit_spine`）。
    saw_at: bool,
    /// **本文件已声明**的记法：符号 → 条目（声明顺序）。作用域 = 文件内、
    /// 声明之后（`docs/design/notation-subset.md` N5），外加**继承表**里的跨
    /// `import` 记法（第二刀 §10.3：被导入模块声明的记法从文件头就可用）。
    ///
    /// **同符号多条 = 记法重载**（第三刀 §12.2）：形状（结合性 + 优先级）必须
    /// 完全一致，展开期按**期望类型**选候选。`scoped` 声明不在表里——它们在
    /// [`Parser::scoped_pending`] 等 `open scoped`。
    notations: HashMap<String, Vec<NotationEntry>>,
    /// 声明过 `scoped`、还没被 `open scoped <scope>` 打开的记法（第三刀 §12.3）。
    scoped_pending: Vec<ScopedNotation>,
    /// 已经 `open scoped` 的作用域名（按出现顺序、去重）。
    opened_scopes: Vec<String>,
    /// **继承来的**符号（第二刀 §11.8）：本文件重声明它们是错误，不是重载——
    /// 判卷通道把闭包首尾相接成一份合成源码，两个模块各声明一次会在那里撞车；
    /// 与其让同一个程序在两条通道上得到不同答案，不如在源头就说不许。
    inherited_symbols: std::collections::HashSet<String>,
    /// **未闭合的 `namespace` 栈**（G-05）：从外到内。声明名加前缀（N3）与
    /// `end` 同名校验（N2）都读它；`parse_file` 在文件尾校验闭合。
    namespaces: Vec<OpenNamespace>,
}

/// 一个未闭合的 `namespace`：写出来的名字 + 累积全前缀 + 那行的 span
/// （未闭合诊断指回它，不是文件尾）。
struct OpenNamespace {
    written: String,
    full: String,
    span: Span,
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
        Self::with_inherited(tokens, &[])
    }

    /// 带**继承记法表**的 parser（第二刀 §10.3）：被导入模块声明的记法在入口
    /// 文件里从文件头就可用（与 Lean 的 import 一致）。空继承表 ⇒ [`Parser::new`]。
    ///
    /// `scoped` 的继承项（`decl.scope.is_some()`）**不进**生效表，进
    /// [`Parser::scoped_pending`]——入口文件要自己写 `open scoped <scope>`
    /// （第三刀 §12.3）。
    pub fn with_inherited(tokens: Vec<Token>, inherited: &[NotationDecl]) -> Self {
        let mut notations: HashMap<String, Vec<NotationEntry>> = HashMap::new();
        let mut scoped_pending: Vec<ScopedNotation> = Vec::new();
        let mut inherited_symbols = std::collections::HashSet::new();
        // 内建记法先入表：它们在**任何**文件里都生效，且**不能被重声明**
        // （见 `register_notation`）——内建是语言的一部分，不是可覆盖的糖。
        for (symbol, assoc, precedence, target) in BUILTIN_NOTATIONS {
            inherited_symbols.insert((*symbol).to_string());
            notations.insert(
                (*symbol).to_string(),
                vec![NotationEntry {
                    symbol: (*symbol).to_string(),
                    precedence: Some(*precedence),
                    assoc: *assoc,
                    target: (*target).to_string(),
                }],
            );
        }
        for decl in inherited {
            let entry = NotationEntry {
                symbol: decl.symbol.clone(),
                precedence: decl.precedence,
                assoc: decl.assoc,
                target: decl.target.clone(),
            };
            inherited_symbols.insert(decl.symbol.clone());
            match &decl.scope {
                None => notations
                    .entry(decl.symbol.clone())
                    .or_default()
                    .push(entry),
                Some(scope) => scoped_pending.push(ScopedNotation {
                    scope: scope.clone(),
                    entry,
                }),
            }
        }
        Self {
            tokens,
            cursor: 0,
            scrutinee_depth: 0,
            by_depth: 0,
            saw_at: false,
            notations,
            scoped_pending,
            opened_scopes: Vec::new(),
            inherited_symbols,
            namespaces: Vec::new(),
        }
    }

    /// 该符号的**主**条目（声明顺序第一条）。所有候选的形状（结合性 +
    /// 优先级）一致（重载的前提，见 [`Parser::register_notation`]），所以
    /// 形状问题问第一条即权威；一条都没有 ⇒ `None`。
    fn notation(&self, symbol: &str) -> Option<&NotationEntry> {
        self.notations
            .get(symbol)
            .and_then(|entries| entries.first())
    }

    /// 该符号的**全部候选目标**（声明顺序）。单候选 ⇒ 长度 1 的向量。
    fn notation_targets(&self, symbol: &str) -> Vec<String> {
        self.notations
            .get(symbol)
            .map(|entries| entries.iter().map(|e| e.target.clone()).collect())
            .unwrap_or_default()
    }

    pub fn parse_file(&mut self, src: &str) -> Result<FolFile> {
        self.parse_file_with(src, true)
    }

    /// 解析的**片段模式**（G-05，设计 `docs/design/namespace-open.md` §4.1）：
    /// 只跳过「文件尾还有未闭合 `namespace`」这一条校验，其它逐字相同。
    ///
    /// 判卷合成（`judge.rs`）用的前缀本来就是**文件的一个片段**——`by` 块所在
    /// 声明在 `namespace Foo` 里时，前缀必然带着一个还没闭合的 `namespace`，
    /// 后面拼上的合成 `#check`/合成声明必须落在**仍然打开的**命名空间里。
    /// 严格入口 [`parse`] 不变（真文件里未闭合就是错误）。
    pub fn parse_fragment(&mut self, src: &str) -> Result<FolFile> {
        self.parse_file_with(src, false)
    }

    fn parse_file_with(&mut self, src: &str, require_closed: bool) -> Result<FolFile> {
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
        if require_closed {
            if let Some(open) = self.namespaces.last() {
                return Err(Diagnostic::new(
                    DiagnosticKind::NamespaceUnclosed {
                        name: open.written.clone(),
                    },
                    open.span,
                    format!("`namespace {}` 没有闭合：文件到这里就结束了", open.written),
                ));
            }
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
            // `abbrev`（G-08，设计 `docs/design/abbrev.md`）：与 `def` **同语义**
            // 的拼写——本语言只有一个透明度层级，所以 Lean 的 reducibility hint
            // 不可观察，两条命令共用 `parse_def` 的每一个字段。
            TokenKind::Ident(kw) if kw == "abbrev" => self.parse_def(),
            TokenKind::Ident(kw) if kw == "theorem" => self.parse_theorem(),
            TokenKind::Ident(kw) if kw == "example" => self.parse_example(),
            TokenKind::Ident(kw) if kw == "axiom" => self.parse_axiom(),
            TokenKind::Ident(kw) if kw == "inductive" => self.parse_inductive_block(),
            TokenKind::Ident(kw) if kw == "#check" => self.parse_hash_check(),
            TokenKind::Ident(kw) if kw == "#reduce" => self.parse_hash_reduce(),
            TokenKind::Ident(kw) if kw == "#print" => self.parse_hash_print(),
            TokenKind::Ident(kw) if kw == "infix" || kw == "infixl" || kw == "infixr" => {
                self.parse_infix_command(None)
            }
            // 一元记法（第二刀 §10.1）：`prefix:N " 𝒫 " => Set.powerset`。
            TokenKind::Ident(kw) if kw == "prefix" || kw == "postfix" => {
                self.parse_unary_notation_command(None)
            }
            TokenKind::Ident(kw) if kw == "notation" => self.parse_notation_command(None),
            // binder 记法（第三刀 §12.1）：`binder_notation "∃" => Exists`。
            TokenKind::Ident(kw) if kw == "binder_notation" => {
                self.parse_binder_notation_command(None)
            }
            // `scoped <记法命令>`（第三刀 §12.3）：默认不生效，等 `open scoped`。
            TokenKind::Ident(kw) if kw == "scoped" => self.parse_scoped_notation_command(),
            TokenKind::Ident(kw) if kw == "namespace" => self.parse_namespace_command(),
            TokenKind::Ident(kw) if kw == "end" => self.parse_end_command(),
            TokenKind::Ident(kw) if kw == "open" => self.parse_open_command(),
            // `export <名字> [<子句>]`（第二刀 §N7）：把命名空间的短名导出给
            // **后续命令 + 导入本文件的调用方**（`open` 只对本文件）。
            TokenKind::Ident(kw) if kw == "export" => self.parse_export_command(),
            _ => {
                Err(self
                    .error_at_current(&format!("expected a .sokonanoda command, found {tok:?}")))
            }
        }
    }

    /// 当前命名空间的累积全前缀（没有 `namespace` 时 `None`）。
    fn current_namespace(&self) -> Option<&str> {
        self.namespaces.last().map(|open| open.full.as_str())
    }

    /// 声明名加命名空间前缀（G-05 N3）：`namespace A` 内 `def mem` ⇒ `A.mem`；
    /// 名字本身带点则**拼接**（`A.Set.mem`，与 Lean 同）。
    fn qualify_decl_name(&self, name: String) -> String {
        match self.current_namespace() {
            Some(prefix) => crate::compile::join_ns(Some(prefix), &name),
            None => name,
        }
    }

    /// `namespace <Ident>`（G-05 N1）：开一个命名空间块。
    fn parse_namespace_command(&mut self) -> Result<Command> {
        let kw = self.bump();
        let name = self.expect_namespace_name("namespace")?;
        let span = Span::new(kw.span.start, self.tokens[self.cursor - 1].span.end);
        let full = crate::compile::join_ns(self.current_namespace(), &name);
        self.namespaces.push(OpenNamespace {
            written: name.clone(),
            full,
            span,
        });
        Ok(Command::Namespace { name, span })
    }

    /// `end <Ident>`（G-05 N1/N2）：闭合最近的 `namespace`，**名字必须写出**。
    /// 裸 `end`、错配、没有可闭合的 `namespace` 都走专用码
    /// （`parse-namespace-shape` / `parse-namespace-mismatch`）。
    fn parse_end_command(&mut self) -> Result<Command> {
        let kw = self.bump();
        let tok = self.peek().clone();
        let TokenKind::Ident(name) = tok.kind.clone() else {
            return Err(Diagnostic::new(
                DiagnosticKind::NamespaceShape {
                    detail: format!("`end` 后面要跟命名空间的名字，found {:?}", tok.kind),
                },
                kw.span,
                "`end` 后面要跟命名空间的名字".to_string(),
            ));
        };
        if name.ends_with('.') || is_namespace_name_keyword(&name) {
            return Err(Diagnostic::new(
                DiagnosticKind::NamespaceShape {
                    detail: format!("`end {name}` 的名字不合法"),
                },
                tok.span,
                format!("`end` 后面要跟命名空间的名字，`{name}` 不是合法的名字"),
            ));
        }
        let Some(open) = self.namespaces.last() else {
            return Err(Diagnostic::new(
                DiagnosticKind::NamespaceMismatch {
                    found: name.clone(),
                    expected: None,
                },
                Span::new(kw.span.start, tok.span.end),
                format!("`end {name}` 没有对应的 `namespace {name}`"),
            ));
        };
        // N2：写出名或累积全前缀都算同名（`namespace A.B` 用 `end A.B`、
        // 嵌套的 `namespace B` 用 `end B`）。
        if name != open.written && name != open.full {
            return Err(Diagnostic::new(
                DiagnosticKind::NamespaceMismatch {
                    found: name.clone(),
                    expected: Some(open.written.clone()),
                },
                Span::new(kw.span.start, tok.span.end),
                format!(
                    "`end {name}` 与最近的 `namespace {}` 不匹配（应该写 `end {}`）",
                    open.written, open.written
                ),
            ));
        }
        self.namespaces.pop();
        self.bump(); // 消费名字 token（`tok` 是 peek 的克隆，没有 bump 过）
        Ok(Command::End {
            name,
            span: Span::new(kw.span.start, tok.span.end),
        })
    }

    /// `open <Ident>`（G-05 N1）：把 `<Ident>.` 加进可省略前缀集合。
    ///
    /// `open scoped <名字>`（第三刀 §12.3）：**只**打开记法作用域（把该作用域下
    /// 已声明的 `scoped` 记法搬进生效表），**不**打开名字前缀——与 Lean 一致
    /// （`open scoped Foo` 不会让 `Foo.bar` 能写成 `bar`）。
    ///
    /// 第二刀（§N7）加三条互斥子句与 `in` 形式：
    ///
    /// ```text
    /// open Foo (a b)              -- only：只让 a、b 两个短名进来
    /// open Foo hiding a b         -- 挡掉 a、b
    /// open Foo renaming a => b    -- 把 a 改名叫 b
    /// open Foo in <命令>           -- 局部：只对这一条命令生效
    /// ```
    fn parse_open_command(&mut self) -> Result<Command> {
        let kw = self.bump();
        if matches!(&self.peek().kind, TokenKind::Ident(word) if word == "scoped") {
            self.bump();
            let name = self.expect_namespace_name("open scoped")?;
            let span = Span::new(kw.span.start, self.tokens[self.cursor - 1].span.end);
            // `open scoped … in …` 明确不做（设计 §7）：记法生效表是 **parse 期
            // 全局副作用**（`notations` / `scoped_pending` / `opened_scopes`），
            // 回滚要克隆整张表；而名字 open 的 `in` 是 elab 期集合，代价为零。
            if matches!(&self.peek().kind, TokenKind::Ident(word) if word == "in") {
                let tok = self.peek().clone();
                return Err(self.namespace_shape_error(
                    "`open scoped … in …` 本轮不做：把这条命令单独写在一行即可（`open scoped Foo` 之后本文件都能用）",
                    tok.span,
                ));
            }
            self.activate_scope(&name);
            return Ok(Command::Open {
                name,
                scoped: true,
                filter: OpenFilter::default(),
                span,
            });
        }
        let name = self.expect_namespace_name("open")?;
        let filter = self.parse_open_filter("open")?;
        self.reject_a_second_clause("open")?;
        // open 头部的 span（到最后一个子句 token 为止）：`open … in …` 的合成
        // 前缀要按源码文本补一行 open（`walk.rs`），所以**不含** `in`。
        let header = Span::new(kw.span.start, self.tokens[self.cursor - 1].span.end);
        if matches!(&self.peek().kind, TokenKind::Ident(word) if word == "in") {
            self.bump();
            let inner = self.parse_command()?;
            if !is_open_in_body(&inner) {
                return Err(self.namespace_shape_error(
                    "`open … in` 后面只能跟一条声明或 `#check`/`#reduce`/`#print`",
                    inner.span(),
                ));
            }
            let span = Span::new(kw.span.start, inner.span().end);
            return Ok(Command::OpenIn {
                name,
                filter,
                inner: Box::new(inner),
                header,
                span,
            });
        }
        Ok(Command::Open {
            name,
            scoped: false,
            filter,
            span: header,
        })
    }

    /// `export <Ident> [<子句>]`（第二刀 §N7）：与 `open` 共用子句文法，
    /// 但不吃 `in`（导出是**跨文件**的长期声明，不是一条命令的临时作用域）。
    fn parse_export_command(&mut self) -> Result<Command> {
        let kw = self.bump();
        let name = self.expect_namespace_name("export")?;
        let filter = self.parse_open_filter("export")?;
        self.reject_a_second_clause("export")?;
        if matches!(&self.peek().kind, TokenKind::Ident(word) if word == "in") {
            let tok = self.peek().clone();
            return Err(self.namespace_shape_error(
                "`export` 不吃 `in`：要只影响一条命令请用 `open Foo in <命令>`",
                tok.span,
            ));
        }
        Ok(Command::Export {
            name,
            filter,
            span: Span::new(kw.span.start, self.tokens[self.cursor - 1].span.end),
        })
    }

    /// 子句**最多一条**（§N7.1）：`open Foo (a b) renaming a => c` 这类组合的
    /// 先后顺序（先过滤还是先改名）本轮**没有取证**（硬规则 2），所以不猜、
    /// 直接报**专用**形状错——不落进通用 `unexpected-token`（那个错会让人以为
    /// 是拼写问题）。
    fn reject_a_second_clause(&mut self, keyword: &str) -> Result<()> {
        let tok = self.peek().clone();
        let second = match &tok.kind {
            TokenKind::LParen => true,
            TokenKind::Ident(word) => word == "hiding" || word == "renaming",
            _ => false,
        };
        if second {
            return Err(self.namespace_shape_error(
                &format!(
                    "`{keyword}` 的子句最多写一条：`(a b)`（only）/ `hiding a b` / `renaming a => b` 三条**互斥**——组合起来的先后顺序本轮没取证，所以不猜"
                ),
                tok.span,
            ));
        }
        Ok(())
    }

    /// `open`/`export` 后面的**互斥子句**（§N7）：`(a b)` / `hiding a b` /
    /// `renaming a => b, c => d`，最多一条；一条都没有 ⇒ 空子句。
    ///
    /// 为什么互斥：组合（`open Foo (a b) renaming a => c`）在 Lean 里能写，
    /// 但两条规则的**先后**（先过滤后改名 vs 反过来）没有取证（硬规则 2），
    /// 教学语法不落没取证的语义（设计 §6 差异 7）。
    fn parse_open_filter(&mut self, keyword: &str) -> Result<OpenFilter> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::LParen => {
                self.bump();
                let mut names = Vec::new();
                loop {
                    match self.peek().kind.clone() {
                        TokenKind::Ident(_) => {
                            names.push(self.expect_short_name(keyword)?);
                        }
                        TokenKind::RParen => {
                            self.bump();
                            break;
                        }
                        other => {
                            return Err(self.namespace_shape_error(
                                &format!(
                                    "`{keyword} Foo (a b)` 里只能是短名，found {other:?}（例如 `{keyword} Foo (mem union)`）"
                                ),
                                self.peek().span,
                            ))
                        }
                    }
                }
                if names.is_empty() {
                    return Err(self.namespace_shape_error(
                        &format!("`{keyword} Foo ()` 是空的：要么写名字，要么整条子句去掉"),
                        tok.span,
                    ));
                }
                Ok(OpenFilter {
                    only: Some(names),
                    ..OpenFilter::default()
                })
            }
            TokenKind::Ident(word) if word == "hiding" => {
                self.bump();
                let mut names = Vec::new();
                // 只吃**短名**：`def`/`#check`/`in` 这些关键字（以及 `open A
                // hiding x` 后面那条命令的开头）不是列表的一部分。
                while self.short_name_ahead() {
                    names.push(self.expect_short_name(keyword)?);
                }
                if names.is_empty() {
                    return Err(self.namespace_shape_error(
                        &format!("`{keyword} Foo hiding` 后面要跟至少一个短名"),
                        tok.span,
                    ));
                }
                Ok(OpenFilter {
                    hiding: names,
                    ..OpenFilter::default()
                })
            }
            TokenKind::Ident(word) if word == "renaming" => {
                self.bump();
                let mut pairs = Vec::new();
                loop {
                    let from = self.expect_short_name(keyword)?;
                    if self.peek().kind != TokenKind::FatArrow {
                        let tok = self.peek().clone();
                        return Err(self.namespace_shape_error(
                            &format!(
                                "`{keyword} Foo renaming a => b` 里 `a` 与 `b` 之间要写 `=>`，found {:?}",
                                tok.kind
                            ),
                            tok.span,
                        ));
                    }
                    self.bump();
                    let to = self.expect_short_name(keyword)?;
                    pairs.push((from, to));
                    if self.peek().kind == TokenKind::Comma {
                        self.bump();
                        continue;
                    }
                    break;
                }
                Ok(OpenFilter {
                    renaming: pairs,
                    ..OpenFilter::default()
                })
            }
            _ => Ok(OpenFilter::default()),
        }
    }

    /// 下一个 token 是不是子句列表里的短名（`hiding a b` 的列表边界）。
    ///
    /// `hiding`/`renaming` 在这里当**子句关键字**：列表到它们就停，于是
    /// `open Foo hiding a renaming b => c`（组合子句）会落到
    /// [`Parser::reject_a_second_clause`] 的专用形状错，而不是把 `renaming`
    /// 当成一个短名默默吞掉。代价：命名空间里叫 `hiding`/`renaming` 的成员
    /// 不能用 `hiding` 列表挡（`(hiding renaming)` 的 only 列表照旧可用）。
    fn short_name_ahead(&self) -> bool {
        matches!(
            &self.peek().kind,
            TokenKind::Ident(name)
                if !is_namespace_name_keyword(name)
                    && name != "in"
                    && name != "hiding"
                    && name != "renaming"
        )
    }

    /// `open`/`export` 子句里的**短名**（不带点）：`Foo.mem` 这种点名在子句里
    /// 没有意义（候选会变成 `Foo.Foo.mem`），所以直接报形状错。
    fn expect_short_name(&mut self, keyword: &str) -> Result<String> {
        let tok = self.peek().clone();
        match tok.kind.clone() {
            TokenKind::Ident(name) if !name.ends_with('.') && !is_namespace_name_keyword(&name) => {
                if name.contains('.') {
                    return Err(self.namespace_shape_error(
                        &format!("`{keyword}` 的子句里只写**短名**（不带前缀）：`{name}` 去掉点前面的部分"),
                        tok.span,
                    ));
                }
                self.bump();
                Ok(name)
            }
            other => Err(self.namespace_shape_error(
                &format!("`{keyword}` 的子句里要跟短名，found {other:?}"),
                tok.span,
            )),
        }
    }

    /// `open scoped <scope>`：把该作用域下**已经声明**的记法搬进生效表，并记住
    /// 「这个作用域开着」——之后在同作用域里声明的 `scoped` 记法直接生效。
    fn activate_scope(&mut self, scope: &str) {
        if !self.opened_scopes.iter().any(|open| open == scope) {
            self.opened_scopes.push(scope.to_string());
        }
        let mut index = 0;
        while index < self.scoped_pending.len() {
            if self.scoped_pending[index].scope == scope {
                let pending = self.scoped_pending.remove(index);
                self.push_entry(pending.entry);
            } else {
                index += 1;
            }
        }
    }

    /// 把一条记法追加进生效表（同符号 = 重载的又一个候选）。
    fn push_entry(&mut self, entry: NotationEntry) {
        self.notations
            .entry(entry.symbol.clone())
            .or_default()
            .push(entry);
    }

    /// `scoped <记法命令>`（第三刀 §12.3）：作用域名 = 声明点所在 `namespace`
    /// 的累积全前缀（与 Lean 一致：`namespace Foo` 里的 `scoped` 记法由
    /// `open scoped Foo` 打开）。
    fn parse_scoped_notation_command(&mut self) -> Result<Command> {
        let kw = self.bump();
        let Some(scope) = self.current_namespace().map(str::to_string) else {
            return Err(self.notation_shape_error(
                "`scoped` 要写在 `namespace` 里：作用域名就是那个命名空间，之后用 `open scoped <名字>` 打开它",
                kw.span,
            ));
        };
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Ident(word) if word == "infix" || word == "infixl" || word == "infixr" => {
                self.parse_infix_command(Some(scope))
            }
            TokenKind::Ident(word) if word == "prefix" || word == "postfix" => {
                self.parse_unary_notation_command(Some(scope))
            }
            TokenKind::Ident(word) if word == "notation" => self.parse_notation_command(Some(scope)),
            TokenKind::Ident(word) if word == "binder_notation" => {
                self.parse_binder_notation_command(Some(scope))
            }
            _ => Err(self.notation_shape_error(
                "`scoped` 后面要跟一条记法命令（infix/infixl/infixr/prefix/postfix/notation/binder_notation）",
                tok.span,
            )),
        }
    }

    /// `namespace` / `open` 后面那个点分名字（N1）。
    fn expect_namespace_name(&mut self, keyword: &str) -> Result<String> {
        let tok = self.peek().clone();
        match tok.kind.clone() {
            TokenKind::Ident(name) if !name.ends_with('.') && !is_namespace_name_keyword(&name) => {
                self.bump();
                Ok(name)
            }
            other => Err(Diagnostic::new(
                DiagnosticKind::NamespaceShape {
                    detail: format!("`{keyword}` 后面要跟一个名字，found {other:?}"),
                },
                tok.span,
                format!("`{keyword}` 后面要跟一个名字（可点分，例如 A.B）"),
            )),
        }
    }

    /// `infix:N " sym " => name` / `infixl` / `infixr`（G-04 / WO-011）。
    /// `N` 必填且落在 `NOTATION_PRECEDENCE_RANGE`；符号是字符串字面量，
    /// 取 `trim` 后的内容（两侧空格是书写习惯）。
    fn parse_infix_command(&mut self, scope: Option<String>) -> Result<Command> {
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
            scope.clone(),
            span,
        )?;
        Ok(Command::Notation {
            symbol,
            precedence: Some(precedence),
            assoc,
            target,
            scope,
            span,
        })
    }

    /// `prefix:N " sym " => name` / `postfix:N " sym " => name`（第二刀 §10.1）：
    /// 一元记法。命令形状与 `infix` 族**逐段共用**（优先级必填、符号是字符串
    /// 字面量、`=>` 指目标），只有结合性字段不同。
    fn parse_unary_notation_command(&mut self, scope: Option<String>) -> Result<Command> {
        let kw_tok = self.bump();
        let TokenKind::Ident(keyword) = kw_tok.kind.clone() else {
            unreachable!("parse_unary_notation_command is only called on a unary keyword");
        };
        let assoc = if keyword == "prefix" {
            NotationAssoc::Prefix
        } else {
            NotationAssoc::Postfix
        };
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
            scope.clone(),
            span,
        )?;
        Ok(Command::Notation {
            symbol,
            precedence: Some(precedence),
            assoc,
            target,
            scope,
            span,
        })
    }

    /// `notation " sym " => name`（零元常量记法；不写优先级，照 core 的 `∅` 行）。
    fn parse_notation_command(&mut self, scope: Option<String>) -> Result<Command> {
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
            scope.clone(),
            span,
        )?;
        Ok(Command::Notation {
            symbol,
            precedence: None,
            assoc: NotationAssoc::Nullary,
            target,
            scope,
            span,
        })
    }

    /// `binder_notation " sym " => name`（第三刀 §12.1）：**binder 位置**的
    /// 记法。与 `notation` 同形（不写优先级），只是结合性字段是
    /// [`NotationAssoc::Binder`]，使用形态是 `∃ x, p` / `∃ x ∈ s, p`。
    fn parse_binder_notation_command(&mut self, scope: Option<String>) -> Result<Command> {
        let kw_tok = self.bump();
        if self.peek().kind == TokenKind::Colon {
            return Err(self.notation_shape_error(
                "binder_notation 不写优先级（binder 记法没有左右操作数）：写 binder_notation \"∃\" => Exists 即可",
                self.peek().span,
            ));
        }
        let (symbol, _) = self.parse_notation_symbol("binder_notation")?;
        self.expect_notation_arrow("binder_notation")?;
        let target = self.expect_ident("a notation target name")?;
        let end = self.tokens[self.cursor - 1].span.end;
        let span = Span::new(kw_tok.span.start, end);
        self.register_notation(
            symbol.clone(),
            None,
            NotationAssoc::Binder,
            target.clone(),
            scope.clone(),
            span,
        )?;
        Ok(Command::Notation {
            symbol,
            precedence: None,
            assoc: NotationAssoc::Binder,
            target,
            scope,
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
        // 纯 ASCII 标识符词（`in`/`e`/`Set`）不能当记法符号：声明驱动的词法
        // 会把**整个文件**里的这个名字都收走，与标识符命名空间正面撞车
        // （`in` 还是 `infix` 的前缀）。**非 ASCII** 的标识符字符不在此列
        // （第二刀 §10.2）：`𝒫`（数学斜体字母）、`ᶜ`（修饰字母）、`⁻¹'`（上标）
        // 正是声明驱动词法要救的符号。
        if super::token::is_ascii_word_symbol(&symbol) {
            return Err(self.notation_shape_error(
                &format!(
                    "`{symbol}` 是普通标识符词，不能当记法符号（它会把整个文件里的这个名字都收走）；符号要用数学符号，例如 ∈ / ⊆ / ∅ / 𝒫 / ''"
                ),
                tok.span,
            ));
        }
        // 词法**先于**符号表消费的字符（第二刀 §10.2）：`∀`/`->`/`--`/`#check`
        // 与字符串定界符在 `next_token` 里排在符号匹配之前，声明成符号也永远
        // 命中不了——早点报，别让学习者对着一个"永远不生效"的声明发呆。
        if let Some(reserved) = super::token::lexer_reserved_symbol_char(&symbol) {
            return Err(self.notation_shape_error(
                &format!(
                    "`{reserved}` 是语言关键字的一部分（∀ / -> / -- / #check / 字符串引号），不能当记法符号"
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

    /// 登记一条记法。三条规则（第三刀 §12.2/§12.3 的合成）：
    ///
    /// 1. **继承来的符号**（import 带的）重声明是错误（第二刀 §11.8，原样保留）；
    /// 2. **本文件里同符号、同形状**（结合性 + 优先级一致）⇒ **重载**：又一个
    ///    候选目标，展开期按期望类型选（第三刀 §12.2）；
    /// 3. **同符号、不同形状** ⇒ 错误：`a ⊕ b` 与 `⊕ a` 是两种读法，parser
    ///    没法在同一个符号上同时成立（要两种形状就换符号）。
    ///
    /// `scope` 是 `Some(作用域名)` 时先挂起（`scoped` 声明），等
    /// `open scoped <作用域名>` 搬进生效表。
    fn register_notation(
        &mut self,
        symbol: String,
        precedence: Option<u16>,
        assoc: NotationAssoc,
        target: String,
        scope: Option<String>,
        span: Span,
    ) -> Result<()> {
        if BUILTIN_NOTATIONS.iter().any(|(s, _, _, _)| *s == symbol) {
            return Err(self.notation_shape_error(
                &format!(
                    "符号 `{symbol}` 是**语言内建记法**（Lean core 级的逻辑符号），不需要也不能重新声明；直接用就行"
                ),
                span,
            ));
        }
        if self.inherited_symbols.contains(&symbol) {
            return Err(self.notation_shape_error(
                &format!(
                    "符号 `{symbol}` 已经声明过记法了（由 import 带进来）：同一个符号在**同一文件**里可以重载，但不能覆盖 import 来的记法；换个符号，或改依赖"
                ),
                span,
            ));
        }
        let clash = self
            .notations
            .get(&symbol)
            .and_then(|entries| entries.first())
            .map(|entry| (entry.assoc, entry.precedence))
            .or_else(|| {
                self.scoped_pending
                    .iter()
                    .find(|pending| pending.entry.symbol == symbol)
                    .map(|pending| (pending.entry.assoc, pending.entry.precedence))
            });
        if let Some((existing_assoc, existing_precedence)) = clash {
            if existing_assoc != assoc || existing_precedence != precedence {
                return Err(self.notation_shape_error(
                    &format!(
                        "符号 `{symbol}` 已经用另一种形状声明过了：同一个符号上的重载必须形状一致（结合性与优先级都相同）；要换形状就换个符号"
                    ),
                    span,
                ));
            }
        }
        let entry = NotationEntry {
            symbol,
            precedence,
            assoc,
            target,
        };
        match scope {
            Some(scope) if !self.opened_scopes.contains(&scope) => {
                self.scoped_pending.push(ScopedNotation { scope, entry });
            }
            _ => self.push_entry(entry),
        }
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

    /// 作用域命令（`namespace`/`end`/`open`/`export`）的形状错：与
    /// `namespace`/`end` 共用 `parse-namespace-shape` 与同一段 hint（第二刀
    /// 的子句/`in` 也是这三条命令的形状的一部分）。
    fn namespace_shape_error(&self, detail: &str, span: Span) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::NamespaceShape {
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
        let name = self.qualify_decl_name(name);
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
        let name = self.qualify_decl_name(name);
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

    /// `cases … with` 之后的分支序列：`| <ctor> <binder>… => <tactics>` 重复。
    ///
    /// **臂体用缩进界定**（本语言唯一的缩进敏感处，与 Lean 的 layout 同义）：
    /// 下一个 tactic 关键字出现在**比 `|` 更深**的列上 ⇒ 它属于当前臂；
    /// 否则当前臂到此为止。没有这条规则就无法区分
    /// 「臂体还有一步」与「cases 写完了、后面是外层的 tactic」——
    /// 而 `;` 分隔符在多行臂体里写起来很别扭。
    fn parse_cases_arms(&mut self) -> Result<Vec<CasesArm>> {
        let mut arms = Vec::new();
        while matches!(self.peek().kind, TokenKind::Pipe) {
            let pipe = self.bump();
            let ctor_tok = self.bump();
            let TokenKind::Ident(ctor) = &ctor_tok.kind else {
                return Err(self.error_here("`cases` 分支需要构造子名"));
            };
            let ctor = ctor.clone();
            let mut binders = Vec::new();
            while let TokenKind::Ident(name) = &self.peek().kind {
                if name == "with" || is_tactic_keyword(name) {
                    break;
                }
                let name_tok = self.bump();
                if let TokenKind::Ident(name) = name_tok.kind {
                    binders.push(name);
                }
            }
            if !matches!(self.peek().kind, TokenKind::FatArrow) {
                return Err(self.error_here("`cases` 分支需要 `=>`"));
            }
            self.bump();
            let tactics = self.parse_tactic_sequence_in_arm(pipe.span.start.column)?;
            let end = tactics
                .last()
                .map(|t| t.span().end)
                .unwrap_or_else(|| self.tokens[self.cursor - 1].span.end);
            arms.push(CasesArm {
                ctor,
                binders,
                tactics,
                span: Span::new(pipe.span.start, end),
            });
        }
        if arms.is_empty() {
            return Err(self
                .error_here("`cases … with` 后面至少要有一个分支（`| 构造子 名字… => tactic`）"));
        }
        Ok(arms)
    }

    /// 臂体的 tactic 序列：直到「下一个 `|`」或「下一个 tactic 关键字不在
    /// `pipe_column` 的更深列上」。
    fn parse_tactic_sequence_in_arm(&mut self, pipe_column: usize) -> Result<Vec<Tactic>> {
        let mut tactics = Vec::new();
        if !self.tactic_keyword_ahead() {
            return Ok(tactics);
        }
        loop {
            tactics.push(self.parse_tactic()?);
            match self.peek().kind {
                TokenKind::Semicolon => {
                    self.bump();
                }
                TokenKind::Pipe => break,
                _ => {
                    if !self.next_line_starts_a_tactic()
                        || self.peek().span.start.column <= pipe_column
                    {
                        break;
                    }
                }
            }
        }
        Ok(tactics)
    }

    fn tactic_keyword_ahead(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(kw) if is_tactic_keyword(kw))
    }

    /// `have h : T := by` 的**嵌套** tactic 序列：用**缩进**界定。
    ///
    /// 与 `cases` 臂体同一条规则（本语言仅有的两处 layout）：第一个列号
    /// **≤ `have` 所在列**的 tactic 属于**外层**块。没有它，嵌套 `by` 会把外层
    /// 剩下的 tactic 全吞掉（`next_line_starts_a_tactic` 只看行号、不看缩进）。
    fn parse_nested_tactic_sequence(&mut self, have_column: usize) -> Result<Vec<Tactic>> {
        let mut tactics = Vec::new();
        if !self.tactic_keyword_ahead() {
            return Ok(tactics);
        }
        loop {
            tactics.push(self.parse_tactic()?);
            match self.peek().kind {
                TokenKind::Semicolon => {
                    self.bump();
                }
                _ => {
                    if !self.next_line_starts_a_tactic()
                        || self.peek().span.start.column <= have_column
                    {
                        break;
                    }
                }
            }
        }
        Ok(tactics)
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
                // `intro a b c`（Lean 常态，设计
                // `docs/design/course-lean-style.md` L1.3）：吃一串名字，
                // 但**遇到 tactic 关键字就停**——`intro h` 换行后写
                // `exact h` / `apply f` / `assumption` 是常态，那些关键字
                // 绝不能被当成 binder 名（改前 `intro` 无名时会把下一行的
                // `exact` 整个吃掉，见 S2 审计的 E02）。
                let mut names: Vec<String> = Vec::new();
                let mut end = tok.span.end;
                while let TokenKind::Ident(name) = &self.peek().kind {
                    if is_tactic_keyword(name) {
                        break;
                    }
                    let name_tok = self.bump();
                    end = name_tok.span.end;
                    if let TokenKind::Ident(name) = name_tok.kind {
                        names.push(name);
                    }
                }
                if names.is_empty() {
                    return Err(self.error_here("`intro` binder name"));
                }
                Ok(Tactic::Intro {
                    names,
                    span: Span::new(tok.span.start, end),
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
            TokenKind::Ident(kw) if kw == "constructor" => {
                self.bump();
                Ok(Tactic::Constructor { span: tok.span })
            }
            TokenKind::Ident(kw) if kw == "left" => {
                self.bump();
                Ok(Tactic::Left { span: tok.span })
            }
            TokenKind::Ident(kw) if kw == "right" => {
                self.bump();
                Ok(Tactic::Right { span: tok.span })
            }
            TokenKind::Ident(kw) if kw == "exfalso" => {
                self.bump();
                Ok(Tactic::Exfalso { span: tok.span })
            }
            TokenKind::Ident(kw) if kw == "have" => {
                // `have h : T := t` / `have h : T := by …`（L3.6）。
                // 类型标注**必填**：本语言不做隐式实参推断，省了类型就判不了
                // `t : T`（要一般合一）。Lean 允许 `have h := t`，这里明确不做。
                let have_column = tok.span.start.column;
                self.bump();
                let name_tok = self.bump();
                let TokenKind::Ident(name) = &name_tok.kind else {
                    return Err(self.error_here("`have` binder name"));
                };
                let name = name.clone();
                if !matches!(self.peek().kind, TokenKind::Colon) {
                    return Err(self.error_here("`:` after the `have` binder name"));
                }
                self.bump();
                let ty = self.parse_expr()?;
                if !matches!(self.peek().kind, TokenKind::ColonEq) {
                    return Err(self.error_here("`:=` after the `have` type"));
                }
                self.bump();
                let (value, end) = if matches!(&self.peek().kind, TokenKind::Ident(kw) if kw == "by")
                {
                    self.bump(); // `by`
                    let tactics = self.parse_nested_tactic_sequence(have_column)?;
                    let end = tactics
                        .last()
                        .map(|t| t.span().end)
                        .unwrap_or_else(|| self.tokens[self.cursor - 1].span.end);
                    (HaveValue::By(tactics), end)
                } else {
                    let expr = self.parse_expr()?;
                    let end = expr.span().end;
                    (HaveValue::Term(expr), end)
                };
                Ok(Tactic::Have {
                    name,
                    ty,
                    value,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(kw) if kw == "use" => {
                self.bump();
                let expr = self.parse_expr()?;
                let end = expr.span().end;
                Ok(Tactic::Use {
                    expr,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(kw) if kw == "cases" => {
                self.bump();
                // 与 `match` 的 scrutinee 同款：`with` 不能被吃成实参
                // （`scrutinee_depth` 让 `starts_atom` 对 `with` 让路）。
                self.scrutinee_depth += 1;
                let expr = self.parse_expr();
                self.scrutinee_depth -= 1;
                let expr = expr?;
                let mut end = expr.span().end;
                let mut arms = Vec::new();
                // 可选的 `with` + 分支。`cases h`（不带 with）是合法写法：
                // 引擎按构造子声明顺序造子目标，分支假设用构造子的字段名。
                if matches!(&self.peek().kind, TokenKind::Ident(k) if k == "with") {
                    self.bump();
                    arms = self.parse_cases_arms()?;
                    if let Some(last) = arms.last() {
                        end = last.span.end;
                    }
                }
                Ok(Tactic::Cases {
                    expr,
                    arms,
                    span: Span::new(tok.span.start, end),
                })
            }
            TokenKind::Ident(kw) if kw == "sorry" => {
                self.bump();
                Ok(Tactic::Sorry { span: tok.span })
            }
            _ => Err(self.error_here(&format!(
                "未知 tactic：`by` 块只支持 intro / exact / apply / assumption / rfl / match / constructor / left / right / use / exfalso / cases / sorry（白名单），发现 {tok:?}"
            ))),
        }
    }

    fn parse_axiom(&mut self) -> Result<Command> {
        let start = self.bump().span.start;
        let name = self.expect_decl_name("axiom name")?;
        let name = self.qualify_decl_name(name);
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
        // G-05 N3：归纳块的名字也吃命名空间前缀（`namespace Set` 里的
        // `inductive Foo` ⇒ `Set.Foo`），构造子照 R1 从**已加前缀的**类型名
        // 派生规范名（`Set.Foo.mk`）——`ctor`/`rec`/`iota` 自己的名字不动。
        let name = self.expect_decl_name("inductive name")?;
        let name = self.qualify_decl_name(name);
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
            // binder 记法（第三刀 §12.1）：`∃ x, p`。与 `∀` 同一层——它一直
            // 吃到表达式结尾（body = `parse_expr`）。
            TokenKind::Sym(symbol) if self.is_binder_notation(symbol) => {
                self.parse_binder_notation()
            }
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
            // RHS 走 **`parse_expr`**（G-28）：`parse_arrow` 只认"算符链 + 原子"，
            // 于是 `A -> forall (x : Prop), …` / `A -> fun x => …` / `A -> let …`
            // 全都报 `unexpected-token`（expected an expression, found Forall），
            // 而 `↔` 的 RHS 走 `parse_expr` ⇒ 只有 `→` 不认（Lean 4 里这些都合法）。
            // 右结合性不变：`A -> B -> C` 仍然解析成 `A -> (B -> C)`。
            let rhs = self.parse_expr()?;
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
        let lhs = self.parse_app()?;
        self.parse_operators_from(lhs, min_precedence)
    }

    /// 从**已经解析好的**左操作数继续爬升。两段式 binder 的 guard（`x ∈ s`）
    /// 需要它：binder 名已经在手，不必再当原子读一遍（第三刀 §12.1）。
    fn parse_operators_from(&mut self, mut lhs: Expr, min_precedence: u16) -> Result<Expr> {
        loop {
            // **后缀记法**（第二刀 §10.1）：`Aᶜ`。与二元算子共用同一条梯子——
            // `N >= min_precedence` 才吸收，所以 N 越大绑得越紧（`A ∪ Bᶜ` 在
            // `ᶜ`=100 时是 `A ∪ (Bᶜ)`，在 50 时是 `(A ∪ B)ᶜ`）。
            if let Some(entry) = self.postfix_ahead(min_precedence) {
                let entry = entry.clone();
                let tok = self.bump();
                let span = Span::new(lhs.span().start, tok.span.end);
                lhs = self.notation_node(
                    &entry.symbol,
                    NotationAssoc::Postfix,
                    Some(Box::new(lhs)),
                    None,
                    span,
                );
                continue;
            }
            // 未声明符号：报**专用**诊断（hint 给「先声明」与「点名写法」），
            // 而不是让 `parse_file`/`parse_command` 用通用的 unexpected-token
            // 糊过去（设计 N2/N5）。
            if let TokenKind::Sym(symbol) = &self.peek().kind {
                if self.notation(symbol).is_none() {
                    let span = self.peek().span;
                    let symbol = symbol.clone();
                    return Err(self.unknown_symbol_error(&symbol, span));
                }
                // 已声明、但**不是二元算子**（前缀/零元）落在算子位上：给
                // 专用教学诊断，而不是让它悄悄结束爬升再报通用错误。
                //
                // 只在它**在这个层级本来就该被吸收**时报（`N >= min_precedence`）：
                // 绑得更松的一元符号要让爬升照常结束，交给外层吸收——
                // `A ∪ Bᶜ`（`ᶜ`=50 < `∪`=65）正是靠这一条读成 `(A ∪ B)ᶜ`。
                if let Some(entry) = self.notation(symbol) {
                    let binds_here = entry.precedence.is_some_and(|p| p >= min_precedence);
                    // binder 记法没有优先级，永远不在算子位上：`binds_here` 对它
                    // 恒为假，所以单独放行（第三刀 §12.1）。
                    if entry.assoc == NotationAssoc::Binder
                        || (!entry.assoc.is_binary() && binds_here)
                    {
                        let span = self.peek().span;
                        let message = match entry.assoc {
                            NotationAssoc::Prefix => format!(
                                "`{symbol}` 是**前缀**记法：它要写在操作数**前面**（例如 {symbol} A），不能夹在两个操作数中间"
                            ),
                            NotationAssoc::Nullary => format!(
                                "`{symbol}` 是零元记法（一个常量），不能当算子用"
                            ),
                            NotationAssoc::Binder => format!(
                                "`{symbol}` 是 **binder 记法**：它要写在表达式**开头**（例如 {symbol} x, p），不能夹在两个操作数中间"
                            ),
                            _ => format!("`{symbol}` 不能出现在这里"),
                        };
                        return Err(self.error_at(span, &message));
                    }
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
                self.notation_node(
                    &op.symbol,
                    op.assoc,
                    Some(Box::new(lhs)),
                    Some(Box::new(rhs)),
                    span,
                )
            };
        }
        Ok(lhs)
    }

    /// 右操作数：先按 `min_precedence` 继续爬升，再对**无结合** `infix` 做
    /// 同级连写检查（`a ∈ b ∈ c` 报解析错——v1 不做 Lean 的「需要括号」诊断，
    /// 设计 N3）。
    fn parse_operand(&mut self, op: &BinaryOp, min_precedence: u16) -> Result<Expr> {
        // **`∀`/`∃` 可以当算子的右操作数，并且一直吃到表达式结尾**
        // （Lean 的读法：它们的优先级最低、向右最大吞噬）。
        //
        // 没有这一条，课程里最常见的形状会直接 parse 失败：
        //     (A ⊆ B) ↔ ∀ (x : α), A x → B x      -- 报 expected an expression, found Forall
        //     P → ∃ (x : α), Q x
        // 学习者是照着数学书写 `↔ ∀ …,` 的，逼他们加一层括号不该是这门语言的行为。
        // 语义**没有新东西**：只是把「`∀`/`∃` 只能出现在表达式开头」放宽到
        // 「出现在算子右侧时，它拥有整个右侧」——与 `fun`/`let`/`match` 在实参位
        // 必须加括号的规则并不冲突（它们**不**在这里放行）。
        if self.leading_binder_ahead() {
            return self.parse_expr();
        }
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

    /// 下一个 token 是不是 `∀` 或已声明的 binder 记法（`∃`）——它们可以当
    /// 算子的右操作数，并吃到表达式结尾（见 [`Self::parse_operand`]）。
    fn leading_binder_ahead(&self) -> bool {
        match &self.peek().kind {
            TokenKind::Forall => true,
            TokenKind::Sym(symbol) => self.is_binder_notation(symbol),
            _ => false,
        }
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
                builtin_plus: true,
            },
            TokenKind::Sym(symbol) => {
                // 未声明符号：**不是**算子（`None`）——留给 `starts_atom`/调用方
                // 报「未声明符号」的专用诊断，而不是在这里假装爬升结束。
                let entry = self.notation(symbol)?;
                // 只有**二元**结合性在梯子上（第二刀：前缀/后缀/零元/binder 都不在）。
                if !entry.assoc.is_binary() {
                    return None;
                }
                let precedence = entry.precedence?;
                BinaryOp {
                    precedence,
                    assoc: entry.assoc,
                    symbol: entry.symbol.clone(),
                    builtin_plus: false,
                }
            }
            _ => return None,
        };
        (op.precedence >= min_precedence).then_some(op)
    }

    /// 下一个 token 是不是**优先级 ≥ `min_precedence`** 的后缀记法
    /// （第二刀 §10.1）。`None` ⇒ 不是后缀记法，或绑得不够紧。
    fn postfix_ahead(&self, min_precedence: u16) -> Option<&NotationEntry> {
        let TokenKind::Sym(symbol) = &self.peek().kind else {
            return None;
        };
        let entry = self.notation(symbol)?;
        if entry.assoc != NotationAssoc::Postfix {
            return None;
        }
        (entry.precedence? >= min_precedence).then_some(entry)
    }

    fn bump_operator(&mut self, op: &BinaryOp) {
        let _ = op;
        self.bump();
    }

    /// 零元记法：`Sym(s)` 且已声明为 `Nullary` ⇒ 记号节点。
    fn parse_nullary_notation(&mut self, symbol: &str, tok: &Token) -> Expr {
        self.notation_node(symbol, NotationAssoc::Nullary, None, None, tok.span)
    }

    /// 拼一个记号节点：**目标候选表**（第三刀 §12.2 的重载）从当前生效表里取，
    /// 第一个是主目标，其余进 `alternatives`（单候选时为空）。
    fn notation_node(
        &self,
        symbol: &str,
        assoc: NotationAssoc,
        lhs: Option<Box<Expr>>,
        rhs: Option<Box<Expr>>,
        span: Span,
    ) -> Expr {
        let targets = self.notation_targets(symbol);
        let (target, alternatives) = match targets.split_first() {
            Some((first, rest)) => (first.clone(), rest.to_vec()),
            None => (String::new(), Vec::new()),
        };
        Expr::Notation {
            symbol: symbol.to_string(),
            target,
            assoc,
            lhs,
            rhs,
            alternatives,
            span,
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
        let mut fun = self.parse_prefix_head()?;
        // Lean 的 `@`（IA-1）：`parse_atom` 见到 `@` 就置位 ⇒ 这条脊的每个 App
        // 节点都带 `explicit_spine`，前端**不插**隐式实参。取走即清（`@` 只
        // 作用于紧跟的那一条脊）。
        let explicit = std::mem::take(&mut self.saw_at);
        loop {
            // **一元前缀记法在实参位免括号**（第三刀 §12.5）：`f 𝒫 A` 就是
            // `f (𝒫 A)`。今天它是**响亮的 parse 错**（"前缀记法不能夹在两个
            // 操作数中间"），所以放开是纯增量——不改任何既有程序的分组。
            //
            // **后缀**不在此列：`f Aᶜ` 今天读成 `(f A)ᶜ`（后置算子在梯子上
            // 吸收整个应用），改了会**悄悄重分组**既有程序。
            let arg = if self.starts_atom() {
                self.parse_atom()?
            } else if self.prefix_notation_ahead() {
                self.parse_prefix_head()?
            } else {
                break;
            };
            let span = Span::new(fun.span().start, arg.span().end);
            fun = Expr::App {
                fun: Box::new(fun),
                arg: Box::new(arg),
                explicit_spine: explicit,
                span,
            };
        }
        Ok(fun)
    }

    /// 下一个 token 是不是**已声明的前缀记法符号**（实参位免括号的判据）。
    fn prefix_notation_ahead(&self) -> bool {
        match &self.peek().kind {
            TokenKind::Sym(symbol) => self
                .notation(symbol)
                .is_some_and(|entry| entry.assoc == NotationAssoc::Prefix),
            _ => false,
        }
    }

    /// **前缀记法**（第二刀 §10.1）：`𝒫 A`。操作数按 `parse_operators(N)` 解析，
    /// 所以 N 越大绑得越紧（`𝒫 A ∪ B` 在 `𝒫`=100 时是 `(𝒫 A) ∪ B`，在 50 时
    /// 是 `𝒫 (A ∪ B)`）。不是前缀记法 ⇒ 普通原子。
    fn parse_prefix_head(&mut self) -> Result<Expr> {
        let entry = match &self.peek().kind {
            TokenKind::Sym(symbol) => match self.notation(symbol) {
                Some(entry) if entry.assoc == NotationAssoc::Prefix => entry.clone(),
                _ => return self.parse_atom(),
            },
            _ => return self.parse_atom(),
        };
        let tok = self.bump();
        let operand = self.parse_operators(entry.precedence.unwrap_or(0))?;
        let span = Span::new(tok.span.start, operand.span().end);
        Ok(self.notation_node(
            &entry.symbol,
            NotationAssoc::Prefix,
            None,
            Some(Box::new(operand)),
            span,
        ))
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
            // 集合字面量（第三刀 §12.4）：`{a}` / `{a, b}` 是原子（`f {a}`
            // 合法）；`{x : T}` 形状**不是**（那是 binder，binder 位置在
            // `∀`/`fun`/声明里，见 `set_literal_ahead`）。
            TokenKind::LBrace => self.set_literal_ahead(),
            // 匿名构造子 `⟨a, b⟩` 是原子（`f ⟨a, b⟩` 合法）——它自带括号，
            // 不会像一元记法那样把实参边界搞糊。
            TokenKind::Langle => true,
            // 记法符号**不得**被当作应用实参：已声明的**零元**记法是一个原子
            // （`f ∅` 合法），二元/前缀记法与未声明符号都让路（设计 N2/N3、
            // 第二刀 §10.1——`f 𝒫 A` 要写成 `f (𝒫 A)`）。
            TokenKind::Sym(symbol) => self
                .notation(symbol)
                .is_some_and(|entry| entry.assoc == NotationAssoc::Nullary),
            _ => false,
        }
    }

    /// `{a}` / `{a, b}` 的 lookahead（第三刀 §12.4）：`{` 后面**不是** binder
    /// 形状（`{x : T}` / `{x y : T}`，判据与 `push_binders` 的
    /// `named_group_ahead` 同一份）就算集合字面量。
    fn set_literal_ahead(&self) -> bool {
        if self.peek().kind != TokenKind::LBrace {
            return false;
        }
        !self.brace_binder_ahead()
    }

    /// `{` 里是不是 binder 形状 `{x : T}` / `{x y : T}`（G-05 的
    /// `named_group_ahead` 只看 `(`/`{` 两种，这里只问 `{`）。
    fn brace_binder_ahead(&self) -> bool {
        let toks = &self.tokens;
        let mut i = self.cursor;
        if !matches!(toks.get(i).map(|t| &t.kind), Some(TokenKind::LBrace)) {
            return false;
        }
        i += 1;
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

    /// `{a}` / `{a, b}`（第三刀 §12.4）：1–2 个元素，展开成点名形式
    /// `Set.singleton α a` / `Set.pair α a b`（elab 侧，与 `+` → `Nat.add` 同族）。
    /// 空 `{}` 与三个以上元素给**专用诊断**（v1 不做 `insert` 链）。
    fn parse_set_literal(&mut self, open: Span) -> Result<Expr> {
        if self.peek().kind == TokenKind::RBrace {
            let close = self.peek().span;
            return Err(Diagnostic::new(
                DiagnosticKind::SetLiteralShape {
                    detail: "空集合字面量 `{}`".to_string(),
                },
                Span::new(open.start, close.end),
                "空集合字面量 `{}` 不合法：空集请写点名形式 Set.empty α".to_string(),
            ));
        }
        let mut elements = vec![self.parse_expr()?];
        while self.peek().kind == TokenKind::Comma {
            self.bump();
            elements.push(self.parse_expr()?);
        }
        let close = self.peek().clone();
        if close.kind != TokenKind::RBrace {
            return Err(Diagnostic::new(
                DiagnosticKind::SetLiteralShape {
                    detail: format!("expected `}}`, found {:?}", close.kind),
                },
                close.span,
                "集合字面量要写成 {a} 或 {a, b}：元素之间用 `,` 隔开，最后用 `}` 收尾".to_string(),
            ));
        }
        self.bump();
        if elements.len() > 2 {
            return Err(Diagnostic::new(
                DiagnosticKind::SetLiteralShape {
                    detail: format!("{} elements", elements.len()),
                },
                Span::new(open.start, close.span.end),
                "集合字面量 v1 只支持 1–2 个元素（`{a}` / `{a, b}`）：三个及以上请用点名形式 Set.pair 自己嵌套"
                    .to_string(),
            ));
        }
        Ok(Expr::SetLiteral {
            elements,
            span: Span::new(open.start, close.span.end),
        })
    }

    /// `⟨a, b⟩`（课程 Lean 化 L2.7）：**匿名构造子**，1 个及以上元素。
    ///
    /// 用哪个构造子由**期望类型**在 elab 期决定（路线 C，不引入元变量），
    /// 所以这里只负责形状：`⟨` 元素 `,` … `⟩`。逗号必需（`⟨a b⟩` 报错），
    /// 与 Lean 的 `⟨_, _⟩` 一致；空 `⟨⟩` 给专用诊断。
    fn parse_anon_ctor(&mut self, open: Span) -> Result<Expr> {
        if self.peek().kind == TokenKind::Rangle {
            let close = self.peek().span;
            return Err(Diagnostic::new(
                DiagnosticKind::SetLiteralShape {
                    detail: "空匿名构造子 `⟨⟩`".to_string(),
                },
                Span::new(open.start, close.end),
                "`⟨⟩` 里至少要写一个元素：`⟨a, b⟩` 是匿名构造子（用哪个构造子由期望类型决定）"
                    .to_string(),
            ));
        }
        let mut elements = vec![self.parse_expr()?];
        while self.peek().kind == TokenKind::Comma {
            self.bump();
            elements.push(self.parse_expr()?);
        }
        let close = self.peek().clone();
        if close.kind != TokenKind::Rangle {
            return Err(Diagnostic::new(
                DiagnosticKind::SetLiteralShape {
                    detail: format!("expected `⟩`, found {:?}", close.kind),
                },
                close.span,
                "匿名构造子要写成 ⟨a, b⟩：元素之间用 `,` 隔开，最后用 `⟩` 收尾".to_string(),
            ));
        }
        self.bump();
        Ok(Expr::AnonCtor {
            elements,
            span: Span::new(open.start, close.span.end),
        })
    }

    fn parse_atom(&mut self) -> Result<Expr> {
        let tok = self.bump();
        match tok.kind {
            // 集合字面量（第三刀 §12.4）：`{a}` / `{a, b}`。
            TokenKind::LBrace => self.parse_set_literal(tok.span),
            // 匿名构造子（课程 Lean 化 L2.7）：`⟨a, b⟩`。
            TokenKind::Langle => self.parse_anon_ctor(tok.span),
            TokenKind::At => {
                // IA-1：`@` 关闭隐式实参插入（Lean 语义）。parser 只把这条信息
                // 传给 `parse_app` 建的 App 节点（`saw_at`），elab 侧读它。
                self.saw_at = true;
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
            // 落在算子位上（`parse_operators` 消费），前缀记法在 `parse_app`
            // 头部消费；未声明符号在这里报专用诊断（**不是** `unknown
            // identifier`，设计 N2/N5）。
            TokenKind::Sym(ref symbol) => match self.notation(symbol) {
                Some(entry) if entry.assoc == NotationAssoc::Nullary => {
                    Ok(self.parse_nullary_notation(symbol, &tok))
                }
                Some(entry) if entry.assoc.is_binary() => Err(self.error_here(&format!(
                    "`{symbol}` 是二元记法符号，两边都要有操作数（例如 a {symbol} b）"
                ))),
                Some(_) => Err(self.error_at(
                    tok.span,
                    &format!(
                        "`{symbol}` 是一元记法符号：它要跟自己的操作数一起写（例如 {symbol} A）"
                    ),
                )),
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
                //
                // 层级算术（`Type (u+1)`，见 `parse_level_text`）：只吃**数字**或
                // **括号**开头的层级，**不吃**裸标识符——`Eq.refl.{2} Type A`
                // 这类「`Type` 作实参、紧跟另一个实参」的既有写法必须保持
                // 应用语义（`Type u` 仍按不支持处理，见设计 §5）。
                let paren_or_num =
                    matches!(self.peek().kind, TokenKind::Num(_) | TokenKind::LParen);
                if paren_or_num {
                    let start = tok.span.start;
                    let text = self.parse_level_text()?;
                    let end = self.tokens[self.cursor - 1].span.end;
                    let span = Span::new(start, end);
                    let sort = match text.parse::<u64>() {
                        // `Type 0` 与 `Type (0)` 走同一条（`SortKind::Sort`）。
                        Ok(n) => SortKind::Sort(
                            n.checked_add(1)
                                .ok_or_else(|| self.level_too_large_error(&text, span))?,
                        ),
                        // 纯数字但超出 u64：仍是**解析期**的"层级过大"诊断
                        // （与 0.59.0 的 `Type <巨大数字>` 同一条），不许静默
                        // 降级成 elab 期的 `unknown universe level`。
                        Err(_) if text.chars().all(|c| c.is_ascii_digit()) => {
                            return Err(self.level_too_large_error(&text, span));
                        }
                        // `Type (u+1)` = `Sort (u+1+1)`。
                        Err(_) => SortKind::Level(format!("{text}+1")),
                    };
                    return Ok(Expr::Sort { sort, span });
                }
                Ok(Expr::Sort {
                    sort: SortKind::Type,
                    span: tok.span,
                })
            }
            TokenKind::Ident(name) if name == "Sort" => {
                // `Sort u`、`Sort 1`、`Sort (u+1)`、`Sort u+1`（层级算术，
                // 见 `parse_level_text`）。
                let start = tok.span.start;
                let text = self.parse_level_text()?;
                let end = self.tokens[self.cursor - 1].span.end;
                let span = Span::new(start, end);
                let sort = match text.parse::<u64>() {
                    Ok(n) => SortKind::Sort(n),
                    Err(_) if text.chars().all(|c| c.is_ascii_digit()) => {
                        return Err(self.level_too_large_error(&text, span));
                    }
                    Err(_) => SortKind::Level(text),
                };
                Ok(Expr::Sort { sort, span })
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
                    // 层级算术（`Eq.{u+1}`）：`parse_level_text` 是 Ident/Num 的
                    // 超集，报错文案与旧版逐字相同（"expected a universe level"）。
                    levels.push(self.parse_level_text()?);
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

    /// 源码层**宇宙层级表达式**：`u`、`3`、`u+1`、`u+1+1`（括号可省/可加）。
    ///
    /// 语法面（设计 `docs/design/type-level-syntax.md` §5，白名单同轮更新）：
    /// 层级 = 原子 (`+` 数字)*；原子 = 数字 | 标识符 | `(` 层级 `)`。`+` 右边
    /// **只收数字**（Lean 的 `u+n` 形式）——`u+v`/`max u v` 要内核的
    /// `Level::Max`，而 `EnvBuilder` 没有公开构造入口（硬规则 1：内核零改动），
    /// 所以它们不在语法面内，报一条专用诊断。
    ///
    /// 返回**层级文本**（如 `"u+1"`）：AST 的层级槽位本来就是文本
    /// （`SortKind::Level` 与 `UniverseApp.levels`），这里只把"名字"放宽成
    /// "名字 + 数字后缀"，翻译成内核层级由 elab 的 `level_ptr` 一处完成。
    fn parse_level_text(&mut self) -> Result<String> {
        let mut text = self.parse_level_atom()?;
        while self.peek().kind == TokenKind::Plus {
            self.bump();
            let tok = self.bump();
            match tok.kind {
                TokenKind::Num(n) => {
                    text.push('+');
                    text.push_str(&n);
                }
                other => {
                    return Err(Diagnostic::new(
                        DiagnosticKind::UnexpectedToken {
                            found: format!("{other:?}"),
                            expected: "a numeral after `+`".to_string(),
                        },
                        tok.span,
                        "层级加法只收数字后缀（例如 `u+1`）；`max`/`u+v` 不在本语言的层级语法面内"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(text)
    }

    /// 层级数字超出 `u64`（`Sort 999…` / `Type 999…`）的解析期诊断：与
    /// 0.59.0 的 `Type <巨大数字>` 同一条，不 panic、也不降级成 elab 期错误。
    fn level_too_large_error(&self, text: &str, span: Span) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::UnexpectedToken {
                found: text.to_string(),
                expected: "a universe level".to_string(),
            },
            span,
            "universe level is too large".to_string(),
        )
    }

    /// [`parse_level_text`] 的原子：数字、标识符或括号层级。
    fn parse_level_atom(&mut self) -> Result<String> {
        let tok = self.bump();
        match tok.kind {
            TokenKind::Num(value) => Ok(value),
            TokenKind::Ident(name) => Ok(name),
            TokenKind::LParen => {
                let inner = self.parse_level_text()?;
                self.expect_kind(&TokenKind::RParen, "`)`")?;
                Ok(inner)
            }
            other => Err(Diagnostic::new(
                DiagnosticKind::UnexpectedToken {
                    found: format!("{other:?}"),
                    expected: "a universe level".to_string(),
                },
                tok.span,
                "expected a universe level".to_string(),
            )),
        }
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
        let (binders, guard) = self.parse_binder_prefix("∀")?;
        let body = self.parse_expr()?;
        let span = Span::new(start, body.span().end);
        // 两段式（第三刀 §12.1）：`∀ x ∈ s, p` 就是 `∀ x, x ∈ s -> p`。
        // binder 的类型由 `x ∈ s` 反解（elab 侧，`guarded_binder_type`）。
        let body = match guard {
            Some(guard) => Expr::Arrow {
                domain: Box::new(guard),
                codomain: Box::new(body),
                span,
            },
            None => body,
        };
        Ok(Expr::Forall {
            binders,
            body: Box::new(body),
            span,
        })
    }

    /// binder 位置的**共享解析路径**（第三刀 §12.1）：`∀`（关键字）与
    /// `binder_notation` 声明的符号都走这里，返回 `(binders, guard)`；
    /// `guard` 是两段式 `x ∈ s` 里的 `x ∈ s`（没有就是 `None`）。
    ///
    /// 两段式的判据：binder 后面紧跟一个**已声明的二元记法符号**（`∈`）——
    /// 于是 `∀ x ∈ s, p` 不必为每个关系再立一条命令，guard 就是一条普通的
    /// 记号表达式（`x ∈ s`），binder 名当它的左操作数。
    fn parse_binder_prefix(&mut self, keyword: &str) -> Result<(Vec<Binder>, Option<Expr>)> {
        let mut binders = Vec::new();
        loop {
            self.push_binders(&mut binders)?;
            if self.peek().kind == TokenKind::Comma {
                self.bump();
                return Ok((binders, None));
            }
            if self.binder_guard_ahead() {
                let last = binders
                    .last()
                    .expect("push_binders always appends at least one binder")
                    .clone();
                let lhs = Expr::Ident {
                    name: last.name.clone(),
                    span: last.span,
                };
                let guard = self.parse_operators_from(lhs, 0)?;
                if self.peek().kind == TokenKind::Comma {
                    self.bump();
                    return Ok((binders, Some(guard)));
                }
                return Err(self.error_here(&format!(
                    "两段式 binder（`{keyword} x ∈ s, p`）里的关系式后面要写 `,`"
                )));
            }
            // 两段式的关系符号**没声明**（`∀ x ∈ s, p` 但本文件没有 `∈`）：
            // 报「未声明符号」的专用诊断（hint 教先声明或点名），而不是让
            // `push_binders` 报一句 "expected a binder"（第三刀）。
            if let TokenKind::Sym(symbol) = &self.peek().kind {
                let span = self.peek().span;
                let symbol = symbol.clone();
                return Err(self.unknown_symbol_error(&symbol, span));
            }
            if self.peek().kind == TokenKind::Eof {
                return Err(self.error_here(&format!(
                    "unexpected end of file inside `{keyword}` binders"
                )));
            }
        }
    }

    /// 下一个 token 是不是**已声明的二元记法符号**（两段式 binder 的关系）。
    fn binder_guard_ahead(&self) -> bool {
        match &self.peek().kind {
            TokenKind::Sym(symbol) => self
                .notation(symbol)
                .is_some_and(|entry| entry.assoc.is_binary()),
            _ => false,
        }
    }

    /// 该符号是不是 `binder_notation` 声明的 **binder 记法**（第三刀 §12.1）。
    fn is_binder_notation(&self, symbol: &str) -> bool {
        self.notation(symbol)
            .is_some_and(|entry| entry.assoc == NotationAssoc::Binder)
    }

    /// `∃ x, p` / `∃ x ∈ s, p`（第三刀 §12.1）：展开成
    /// `Exists A (fun (x : A) => p)` / `Exists A (fun (x : A) => And (x ∈ s) p)`。
    ///
    /// parser 只构造**源级形状**（一个 lambda 操作数），binder 的类型与目标
    /// 前导参数的补全都在 elaborator 里走**与前缀记法逐字相同**的路径。
    fn parse_binder_notation(&mut self) -> Result<Expr> {
        let tok = self.bump();
        let TokenKind::Sym(symbol) = tok.kind.clone() else {
            unreachable!("parse_binder_notation is only called on a declared binder symbol");
        };
        let (binders, guard) = self.parse_binder_prefix(&symbol)?;
        let body = self.parse_expr()?;
        let span = Span::new(tok.span.start, body.span().end);
        let body = match guard {
            // 两段式 = guard ∧ body（`∃` 的读法，与 Lean 的 `∃ x ∈ s, p` 同义）。
            Some(guard) => {
                let and_span = guard.span();
                Expr::App {
                    fun: Box::new(Expr::App {
                        fun: Box::new(Expr::Ident {
                            name: "And".to_string(),
                            span: and_span,
                        }),
                        arg: Box::new(guard),
                        explicit_spine: false,
                        span: and_span,
                    }),
                    arg: Box::new(body),
                    explicit_spine: false,
                    span,
                }
            }
            None => body,
        };
        let operand = Expr::Lambda {
            binders,
            body: Box::new(body),
            span,
        };
        Ok(self.notation_node(
            &symbol,
            NotationAssoc::Binder,
            None,
            Some(Box::new(operand)),
            span,
        ))
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

    /// 教学诊断：**消息就是全部**（不要把 token 的 Debug 拼进去）——记法的
    /// 「位置不对」要读起来像人话（第二刀 §10.1）。
    fn error_at(&self, span: Span, msg: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::UnexpectedToken {
                found: "a notation symbol".to_string(),
                expected: msg.to_string(),
            },
            span,
            msg.to_string(),
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

/// **内建记法**（课程 Lean 化，设计 `docs/design/course-lean-style.md` L2.2）：
/// Lean core 级的逻辑符号，**任何 `.sokonanoda` 文件开箱可用**，不需要
/// `infix`/`prefix` 声明——地位与 Lean 的 `Init` 记法一致。
///
/// 为什么是**内建表**而不是写进 prelude：`install_l1_prelude` 的
/// `command_belongs_to`（`compile/prelude.rs`）只认 `Axiom`/`Def`/
/// `InductiveBlock`，**记法命令写进 `PRELUDE_L1_SRC` 会被静默忽略**（实测）；
/// 而内建表天然解决「让位 / 重声明 / 跨 import / 作用域」四个问题。
///
/// 优先级照 Lean core：`↔`20 < `∨`30 < `∧`35 < `¬`40（`->` 比它们都松）。
/// 目标名在**使用点**解析：`And`/`Or`/`Iff`/`Not` 来自 prelude（或课程自己
/// 声明的同名块），缺了报「未知标识符」。
///
/// `=`50 与 `≠`50 是后加的（设计 L2.4b / L2.3，SP1）：Lean core 里它们就是
/// `Eq` / `Ne` 的中缀记法，而本语言从前**连词法都没有**（`a = b` 报
/// `expected `=>``）⇒ 课程满屏 `Eq.{1} (Set α) A B`。`=`/`≠` 的**宇宙层级**
/// 由 `elab_notation` 从操作数类型的 sort 解出（`Eq`/`Ne` 各带一个 `u`，
/// 不能像 `And` 那样全填 0）。
///
/// `→`（U+2192）**不在**这张表里：函数空间不是常量，记法只产出
/// `mk_const`/`mk_app`，它没有目标名可指——`→` 走**词法别名**（`token.rs`）。
const BUILTIN_NOTATIONS: &[(&str, NotationAssoc, u16, &str)] = &[
    ("∧", NotationAssoc::Infixr, 35, "And"),
    ("∨", NotationAssoc::Infixr, 30, "Or"),
    ("↔", NotationAssoc::Infix, 20, "Iff"),
    ("¬", NotationAssoc::Prefix, 40, "Not"),
    ("=", NotationAssoc::Infix, 50, "Eq"),
    ("≠", NotationAssoc::Infix, 50, "Ne"),
];

/// 内建记法表（符号 / 结合性 / 优先级 / 目标点名）——**显示层要它**：这些记法
/// 不在任何源文本里，`notation::notation_table` 收不到（T-C20 实测）。
pub(crate) fn builtin_notations() -> &'static [(&'static str, NotationAssoc, u16, &'static str)] {
    BUILTIN_NOTATIONS
}

/// 内建记法的符号文本（喂给词法：`↔`/`¬`/`≠` 不在数学码点类里，不喂就切不出来）。
pub(crate) fn builtin_notation_symbols() -> Vec<String> {
    BUILTIN_NOTATIONS
        .iter()
        .map(|(symbol, _, _, _)| (*symbol).to_string())
        .collect()
}

/// 内建记法的**展开目标**（`=` → `Eq`、`∧` → `And`）。
///
/// hover 要告诉学习者「这个符号展开成什么」（设计 `docs/design/notation-input.md`
/// §4）：内建符号在本文件里**没有声明行**，所以目标只能从这张表来。
pub fn builtin_notation_target(symbol: &str) -> Option<&'static str> {
    BUILTIN_NOTATIONS
        .iter()
        .find(|(candidate, _, _, _)| *candidate == symbol)
        .map(|(_, _, _, target)| *target)
}

/// **喂给词法**的内建符号：与 [`builtin_notation_symbols`] 相同，但剔除
/// 「词法有专用分支」的 ASCII 符号。
///
/// 今天只有 `=` 属于这一类。**为什么必须剔除**：词法的符号匹配是**最长匹配**，
/// 且排在专用分支之前；`=` 一旦进了符号表，`=>` 就会被吃成 `=` + `>`，
/// 于是 `fun (x) => …` 全炸（实测：L1 prelude 第 9 行的 `=>` 当场解析失败）。
/// `=` 由 `token.rs` 的 `'='` 分支**原生**产出（`Sym("=")`，后面跟 `>` 时仍走
/// `FatArrow`）⇒ 词法不需要也不该再把它当候选符号。
pub(crate) fn lexer_builtin_symbols() -> Vec<String> {
    builtin_notation_symbols()
        .into_iter()
        .filter(|symbol| !LEXER_NATIVE_SYMBOLS.contains(&symbol.as_str()))
        .collect()
}

/// 词法有专用分支的 ASCII 符号（见 [`lexer_builtin_symbols`]）。
const LEXER_NATIVE_SYMBOLS: &[&str] = &["="];

pub fn parse(src: &str) -> Result<FolFile> {
    parse_with_inherited(src, &[])
}

/// 带**继承记法表**的解析（第二刀 §10.3）：被导入模块声明的记法既进词法的
/// 符号表（[`scan_notation_symbols`] 收本文件的 + `inherited` 收依赖的），
/// 也进 parser 的算子表。空继承表 ⇒ [`parse`] 逐字节相同。
pub fn parse_with_inherited(src: &str, inherited: &[NotationDecl]) -> Result<FolFile> {
    let mut symbols = scan_notation_symbols(src);
    for symbol in lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for decl in inherited {
        if !symbols.contains(&decl.symbol) {
            symbols.push(decl.symbol.clone());
        }
    }
    let tokens = tokenize_with_symbols(src, &symbols)?;
    let mut parser = Parser::with_inherited(tokens, inherited);
    parser.parse_file(src)
}

/// 解析一个**源码片段**（G-05）：只跳过「文件尾未闭合 `namespace`」的校验，
/// 其余与 [`parse`] 逐字相同。判卷合成（`judge.rs`）的前缀是文件的一部分，
/// 命名空间在那里**故意保持打开**（合成的 `#check`/合成声明要落在里面）。
pub fn parse_fragment(src: &str) -> Result<FolFile> {
    parse_fragment_with_inherited(src, &[])
}

/// [`parse_fragment`] 的继承表版本（判卷合成路径不跨模块，留作对称入口）。
pub fn parse_fragment_with_inherited(src: &str, inherited: &[NotationDecl]) -> Result<FolFile> {
    let mut symbols = scan_notation_symbols(src);
    for symbol in lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for decl in inherited {
        if !symbols.contains(&decl.symbol) {
            symbols.push(decl.symbol.clone());
        }
    }
    let tokens = tokenize_with_symbols(src, &symbols)?;
    let mut parser = Parser::with_inherited(tokens, inherited);
    parser.parse_fragment(src)
}

/// 词法级 `import` 扫描（G-04 第二刀）：闭包加载器要在解析一个模块**之前**
/// 拿到它的依赖边，所以它从这里转出（实现在 `token.rs`，与
/// `Lexer::on_import_line` 同款判据）。
pub(crate) use super::token::scan_import_lines;

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
        "intro"
            | "exact"
            | "apply"
            | "assumption"
            | "rfl"
            | "match"
            | "constructor"
            | "left"
            | "right"
            | "use"
            | "exfalso"
            | "cases"
            | "have"
            | "sorry"
    )
}

/// `namespace` / `end` / `open` 后面那个名字不能是**关键字**（G-05 N1）：
/// `namespace def` 这类写法要报「形状不对」，而不是把 `def` 当命名空间名
/// 再在下一行报一个看不懂的错误。
fn is_namespace_name_keyword(name: &str) -> bool {
    is_reserved_command(name) || is_expr_keyword(name)
}

fn is_reserved_command(name: &str) -> bool {
    NOTATION_COMMANDS.contains(&name)
        || matches!(
            name,
            "scoped"
                | "import"
                | "def"
                | "abbrev"
                | "theorem"
                | "example"
                | "axiom"
                | "inductive"
                | "ctor"
                | "rec"
                | "iota"
                | "end"
                | "namespace"
                | "open"
                | "export"
                | "#check"
                | "#reduce"
                | "#print"
        )
}

/// `open … in <命令>` 后面允许跟的命令（§N7）：**叶子命令**——声明或
/// `#check`/`#reduce`/`#print`。
///
/// 排除 `import`（置顶规则）、`namespace`/`end`（块结构会跨出 `in` 的作用域）、
/// 嵌套 `open`/`export`（作用域命令对一条命令没有意义）、记法命令（不是声明、
/// 也不解析引用）。判错给**专用形状码**，不落进通用 `unexpected-token`。
fn is_open_in_body(command: &Command) -> bool {
    matches!(
        command,
        Command::Def { .. }
            | Command::Theorem { .. }
            | Command::Axiom { .. }
            | Command::Example { .. }
            | Command::InductiveBlock { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. }
            // 嵌套的 `open A in open B in <叶子>`：内层已经按同一条规则校验过。
            | Command::OpenIn { .. }
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
    fn level_arithmetic_parses_in_sort_and_universe_args() {
        // 层级算术（设计 `docs/design/type-level-syntax.md` §5）：
        // `Sort (u+1)`、`Sort u+1`、`Type (u+1)`、`Eq.{u+1}` 都产出层级文本。
        let file = parse(
            "axiom A {u} : Sort (u+1)\n\
             axiom B {u} : Sort u+1\n\
             axiom C {u} : Type (u+1)\n\
             axiom D {u} : Eq.{u+1} (Sort u) Prop Prop\n",
        )
        .unwrap();
        let sort_of = |i: usize| match &file.commands[i] {
            Command::Axiom {
                ty: Expr::Sort { sort, .. },
                ..
            } => sort.clone(),
            other => panic!("expected Sort at {i}: {other:?}"),
        };
        assert_eq!(sort_of(0), SortKind::Level("u+1".to_string()));
        // `Sort u+1` 与 `Sort (u+1)` 逐字同形（层级文本规范化掉空白）。
        assert_eq!(sort_of(1), SortKind::Level("u+1".to_string()));
        // `Type (u+1)` = `Sort (u+1+1)`。
        assert_eq!(sort_of(2), SortKind::Level("u+1+1".to_string()));
        let Command::Axiom { ty, .. } = &file.commands[3] else {
            panic!("expected axiom");
        };
        fn head(expr: &Expr) -> &Expr {
            match expr {
                Expr::App { fun, .. } => head(fun),
                other => other,
            }
        }
        let Expr::UniverseApp { levels, .. } = head(ty) else {
            panic!("expected UniverseApp head, got {ty:?}");
        };
        assert_eq!(levels, &["u+1".to_string()]);
    }

    #[test]
    fn level_arithmetic_keeps_plain_forms_byte_identical() {
        // 既有拼写的 AST 逐字不变：`Sort 3` 仍是 `SortKind::Sort(3)`、
        // `Sort u` 仍是 `SortKind::Level("u")`、`Type 2` 仍是 `Sort(3)`。
        let file = parse("axiom A : Sort 3\naxiom B {u} : Sort u\naxiom C : Type 2\n").unwrap();
        let sort_of = |i: usize| match &file.commands[i] {
            Command::Axiom {
                ty: Expr::Sort { sort, .. },
                ..
            } => sort.clone(),
            other => panic!("expected Sort at {i}: {other:?}"),
        };
        assert_eq!(sort_of(0), SortKind::Sort(3));
        assert_eq!(sort_of(1), SortKind::Level("u".to_string()));
        assert_eq!(sort_of(2), SortKind::Sort(3));
        // `Type` 后跟**裸标识符**仍是应用（`Eq.refl.{2} Type A` 的既有写法）：
        // `Type` 只吃数字或括号开头的层级（设计 §5 的边界）。
        let file = parse("axiom T : Prop -> Prop\n#check T Type A\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[1] else {
            panic!("expected #check");
        };
        assert!(
            matches!(expr, Expr::App { .. }),
            "`Type A` must stay an application, got {expr:?}"
        );
    }

    #[test]
    fn level_arithmetic_rejects_non_numeric_suffix() {
        // `+` 右边只收数字（Lean 的 `u+n`）：`u+v`/`max` 不在语法面内，
        // 报专用诊断而不是静默吞掉（内核 `Level::Max` 无公开构造入口）。
        for src in [
            "axiom A {u, v} : Sort (u+v)\n",
            "axiom A {u, v} : Eq.{u+v} Prop Prop Prop\n",
        ] {
            let err = parse(src).unwrap_err();
            assert!(
                err.message.contains("层级加法只收数字后缀"),
                "{src:?} → {err:?}"
            );
        }
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

    /// **G-28**：`→` 右边直接跟 `∀` / `forall` 必须解析（Lean 4 里合法）。
    ///
    /// 改前 `parse_arrow` 的 RHS 直接调 `parse_arrow()`，绕过 `parse_expr`
    /// （`forall`/`fun`/`let`/`match` 的入口）⇒ 报
    /// `unexpected-token`「expected an expression, found Forall」；
    /// 而 `↔` 的 RHS 走 `parse_expr` ⇒ **只有 `→` 不认**。
    #[test]
    fn arrow_rhs_accepts_forall_fun_and_let() {
        // `A -> ∀ x, …`（课程正文里很常见的形状）。
        let file = parse("#check Prop -> forall (x : Prop), x -> x\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        // `A -> B` 是 `Expr::Arrow`（不是 Forall）——RHS 才是那个 forall。
        let Expr::Arrow { codomain, .. } = expr else {
            panic!("expected an Arrow at the top, got {expr:?}");
        };
        assert!(
            matches!(codomain.as_ref(), Expr::Forall { .. }),
            "`->` 的 RHS 必须是一个 forall，实际 = {codomain:?}"
        );

        // `∀` 的 unicode 写法与 `fun` / `let` 同样要认。
        parse("#check Prop -> ∀ (x : Prop), x -> x\n").expect("`∀` 也要认");
        parse("#check Prop -> fun (x : Prop) => x\n").expect("`fun` 也要认");
        parse("#check Prop -> let x : Prop := Prop; x\n").expect("`let` 也要认");

        // **右结合性不变**：`A -> B -> C` 仍然是 `A -> (B -> C)`。
        let file = parse("#check Prop -> Prop -> Prop\n").unwrap();
        let Command::Check { expr, .. } = &file.commands[0] else {
            panic!("expected #check");
        };
        let Expr::Arrow { codomain, .. } = expr else {
            panic!("expected an Arrow");
        };
        assert!(
            matches!(codomain.as_ref(), Expr::Arrow { .. }),
            "`->` 必须右结合（`A -> (B -> C)`），实际 = {codomain:?}"
        );
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

    // ── G-05：namespace / end / open（设计 docs/design/namespace-open.md）────

    #[test]
    fn namespace_end_open_are_commands() {
        let file = parse(
            "namespace Foo\n\
             def x : Type := Prop\n\
             end Foo\n\
             open Foo\n",
        )
        .expect("parse");
        assert_eq!(file.commands.len(), 4);
        assert!(matches!(
            &file.commands[0],
            Command::Namespace { name, .. } if name == "Foo"
        ));
        assert!(matches!(
            &file.commands[2],
            Command::End { name, .. } if name == "Foo"
        ));
        assert!(matches!(
            &file.commands[3],
            Command::Open { name, .. } if name == "Foo"
        ));
    }

    #[test]
    fn nested_namespaces_accumulate_the_prefix() {
        let file = parse(
            "namespace A\n\
             namespace B\n\
             def mem : Type := Prop\n\
             end B\n\
             end A\n",
        )
        .expect("parse");
        assert!(
            matches!(&file.commands[2], Command::Def { name, .. } if name == "A.B.mem"),
            "nested namespaces must accumulate: {:?}",
            file.commands
        );
    }

    #[test]
    fn a_dotted_namespace_prefixes_like_two_nested_ones() {
        let file = parse("namespace A.B\ndef mem : Type := Prop\nend A.B\n").expect("parse");
        assert!(matches!(&file.commands[1], Command::Def { name, .. } if name == "A.B.mem"));
    }

    #[test]
    fn a_dotted_declaration_name_is_concatenated_inside_a_namespace() {
        // Lean 同款：`namespace A` 里的 `def Set.mem` ⇒ `A.Set.mem`。
        let file = parse("namespace A\ndef Set.mem : Type := Prop\nend A\n").expect("parse");
        assert!(matches!(&file.commands[1], Command::Def { name, .. } if name == "A.Set.mem"));
    }

    #[test]
    fn end_mismatch_is_a_dedicated_parse_error() {
        let err = parse("namespace A\ndef x : Type := Prop\nend B\n").expect_err("mismatch");
        assert_eq!(err.code(), "parse-namespace-mismatch");
        assert!(
            err.message.contains('A'),
            "the message must name the expected `end A`: {err:?}"
        );
    }

    #[test]
    fn end_without_an_open_namespace_is_a_dedicated_parse_error() {
        let err = parse("end Foo\n").expect_err("no namespace");
        assert_eq!(err.code(), "parse-namespace-mismatch");
    }

    #[test]
    fn an_unclosed_namespace_is_reported_at_eof() {
        let err = parse("namespace Foo\ndef x : Type := Prop\n").expect_err("unclosed");
        assert_eq!(err.code(), "parse-namespace-unclosed");
        // span 指回那条 `namespace`（不是文件尾）。
        assert_eq!(err.span.start.line, 1);
    }

    #[test]
    fn namespace_shape_errors_are_dedicated() {
        let err = parse("namespace\n").expect_err("no name");
        assert_eq!(err.code(), "parse-namespace-shape");
        let err = parse("namespace A\ndef x : Type := Prop\nend\n").expect_err("bare end");
        assert_eq!(err.code(), "parse-namespace-shape");
        // 关键字不能当命名空间名（`namespace def x : …` 这种漏写名字的写法
        // 要报形状错，而不是把 `def` 当名字）。
        let err = parse("namespace def x : Type := Prop\n").expect_err("keyword name");
        assert_eq!(err.code(), "parse-namespace-shape");
        let err = parse("open def\n").expect_err("keyword name");
        assert_eq!(err.code(), "parse-namespace-shape");
    }

    #[test]
    fn parse_fragment_tolerates_an_unclosed_namespace() {
        // 判卷合成路径专用（judge.rs 的前缀是文件的一个片段，见设计 §4.1）。
        let file = parse_fragment("namespace A\ndef mem : Type := Prop\n").expect("fragment");
        assert!(matches!(&file.commands[1], Command::Def { name, .. } if name == "A.mem"));
        // 严格入口仍然报（同一份文本）。
        assert!(parse("namespace A\ndef mem : Type := Prop\n").is_err());
    }

    #[test]
    fn namespace_and_open_are_reserved_inside_expressions() {
        let err = parse("def f : Type := namespace\n").expect_err("keyword as expression");
        assert_eq!(err.code(), "unexpected-token");
    }

    // ── 第二刀：open 的子句 / `open … in` / `export`（设计 §N7）────────────

    #[test]
    fn open_clauses_parse_into_the_filter() {
        let file = parse(
            "open Foo\n\
             open Bar (x y)\n\
             open Baz hiding z\n\
             open Qux renaming a => b, c => d\n",
        )
        .expect("parse");
        let Command::Open { filter, .. } = &file.commands[0] else {
            panic!("expected an open command: {:?}", file.commands[0]);
        };
        assert!(filter.is_empty(), "no clause => empty filter");
        let Command::Open { filter, .. } = &file.commands[1] else {
            panic!("expected an open command");
        };
        assert_eq!(
            filter.only.as_deref(),
            Some(&["x".to_string(), "y".to_string()][..])
        );
        let Command::Open { filter, .. } = &file.commands[2] else {
            panic!("expected an open command");
        };
        assert_eq!(filter.hiding, vec!["z".to_string()]);
        let Command::Open { filter, .. } = &file.commands[3] else {
            panic!("expected an open command");
        };
        assert_eq!(
            filter.renaming,
            vec![
                ("a".to_string(), "b".to_string()),
                ("c".to_string(), "d".to_string())
            ]
        );
    }

    #[test]
    fn open_clause_names_may_collide_with_command_keywords_in_other_positions() {
        // `hiding` 后面那条命令的关键字（`def`/`#check`）不是列表的一部分。
        let file = parse("open Foo hiding x\ndef y : Type := Prop\n").expect("parse");
        assert_eq!(file.commands.len(), 2, "{:?}", file.commands);
        let Command::Open { filter, .. } = &file.commands[0] else {
            panic!("expected an open command");
        };
        assert_eq!(filter.hiding, vec!["x".to_string()]);
    }

    #[test]
    fn open_in_wraps_the_next_command_and_records_the_header() {
        let file = parse("open Foo hiding x in def y : Type := Prop\n").expect("parse");
        assert_eq!(file.commands.len(), 1, "{:?}", file.commands);
        let Command::OpenIn {
            name,
            filter,
            inner,
            header,
            span,
        } = &file.commands[0]
        else {
            panic!("expected `open … in`: {:?}", file.commands[0]);
        };
        assert_eq!(name, "Foo");
        assert_eq!(filter.hiding, vec!["x".to_string()]);
        assert!(
            matches!(inner.as_ref(), Command::Def { name, .. } if name == "y"),
            "the wrapped command must be the declaration: {inner:?}"
        );
        // header 只覆盖 open 头部（合成前缀要按它补一行 open），span 覆盖整条。
        let src = "open Foo hiding x in def y : Type := Prop\n";
        assert_eq!(
            &src[header.start.offset..header.end.offset],
            "open Foo hiding x"
        );
        assert_eq!(&src[span.start.offset..span.end.offset], src.trim_end());
    }

    #[test]
    fn nested_open_in_chains_unwrap() {
        let file = parse("open A in open B in #check x\n").expect("parse");
        let Command::OpenIn { inner, .. } = &file.commands[0] else {
            panic!("expected the outer `open … in`");
        };
        assert!(
            matches!(inner.as_ref(), Command::OpenIn { name, .. } if name == "B"),
            "inner: {inner:?}"
        );
    }

    #[test]
    fn export_parses_like_open_without_in() {
        let file = parse("export Foo (x)\n").expect("parse");
        let Command::Export { name, filter, .. } = &file.commands[0] else {
            panic!("expected an export command: {:?}", file.commands[0]);
        };
        assert_eq!(name, "Foo");
        assert_eq!(filter.only.as_deref(), Some(&["x".to_string()][..]));
    }

    #[test]
    fn open_and_export_clause_shape_errors_are_dedicated() {
        for (src, tag) in [
            ("open Foo ()\n", "empty only list"),
            ("open Foo hiding\n", "empty hiding list"),
            ("open Foo renaming a b\n", "renaming without `=>`"),
            ("open Foo (A.b)\n", "dotted name in the only list"),
            ("open Foo (a b) renaming a => c\n", "two clauses combined"),
            ("open Foo hiding a hiding b\n", "the same clause twice"),
            ("open Foo in import Bar\n", "import as the body"),
            ("open Foo in namespace A\n", "namespace as the body"),
            (
                "open Foo in infix:50 \" + \" => Plus.plus\n",
                "notation as the body",
            ),
            ("open scoped Foo in #check x\n", "open scoped with `in`"),
            ("export Foo in #check x\n", "export with `in`"),
        ] {
            let err = parse(src).unwrap_err();
            assert_eq!(err.code(), "parse-namespace-shape", "{tag}: {err:?}");
        }
    }

    // ---- abbrev（G-08，docs/design/abbrev.md）------------------------------

    /// 去掉 Debug 文本里的 `Span { … }` 段：`def` 与 `abbrev` 的**源码偏移
    /// 本来就不同**（`abbrev` 多三个字符），要比较的是 AST **形状**。
    fn strip_spans(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find("Span {") {
            out.push_str(&rest[..start]);
            let mut depth = 0usize;
            let mut end = None;
            for (i, ch) in rest[start..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(start + i + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { break };
            rest = &rest[end..];
        }
        out.push_str(rest);
        out
    }

    /// 首个命令的 `(name, universe, ty, val)`——`def` 与 `abbrev` 必须给出
    /// **逐字段相同**的 `Command::Def`（`abbrev` 不是新 AST 节点）。
    fn def_shape_of(src: &str) -> (String, Vec<String>, String, String) {
        let file = parse(src).unwrap_or_else(|e| panic!("parse {src}: {e:?}"));
        let Command::Def {
            name,
            universe,
            ty,
            val,
            ..
        } = &file.commands[0]
        else {
            panic!("expected a def command, got {:?}", file.commands[0]);
        };
        (
            name.clone(),
            universe.clone(),
            strip_spans(&format!("{ty:?}")),
            strip_spans(&format!("{val:?}")),
        )
    }

    // ---- 一元记法 prefix/postfix（G-04 第二刀，设计 §10.1）-----------------

    #[test]
    fn prefix_and_postfix_commands_parse_into_one_ast_node_each() {
        let (symbol, precedence, assoc, target) =
            notation_of("prefix:100 \" 𝒫 \" => Set.powerset\n");
        assert_eq!(
            (symbol.as_str(), precedence, assoc, target.as_str()),
            ("𝒫", Some(100), NotationAssoc::Prefix, "Set.powerset")
        );
        let (symbol, precedence, assoc, target) = notation_of("postfix:100 \" ᶜ \" => Set.compl\n");
        assert_eq!(
            (symbol.as_str(), precedence, assoc, target.as_str()),
            ("ᶜ", Some(100), NotationAssoc::Postfix, "Set.compl")
        );
        // `infixr:80 " '' "`：撇号符号（第二刀的词法增量）。
        let (symbol, _, assoc, _) = notation_of("infixr:80 \" '' \" => Set.image\n");
        assert_eq!((symbol.as_str(), assoc), ("''", NotationAssoc::Infixr));
    }

    #[test]
    fn prefix_and_postfix_require_a_precedence() {
        // 与 `infix` 族同一把尺子、同一个诊断。
        let err = parse("prefix \" 𝒫 \" => Set.powerset\n").expect_err("missing N");
        assert_eq!(err.code(), "notation-shape");
        assert!(err.hint().contains("N 取 1–1000"), "hint: {}", err.hint());
        let err = parse("postfix:0 \" ᶜ \" => Set.compl\n").expect_err("N out of range");
        assert_eq!(err.code(), "notation-shape");
    }

    #[test]
    fn declared_symbols_are_reserved_for_the_whole_file() {
        // 声明驱动的词法（设计 §10.2）：`𝒫`/`ᶜ`/`''` 这些"不是数学符号类"的
        // 符号在**本文件**里被读成 `Sym`——`Aᶜ` 必须在标识符内部断开。
        let file = parse(
            "prefix:100 \" 𝒫 \" => Set.powerset\n\
             postfix:100 \" ᶜ \" => Set.compl\n\
             infixr:80 \" '' \" => Set.image\n\
             def p (α : Type) (A : Set α) : Set (Set α) := 𝒫 A\n\
             def c (α : Type) (A : Set α) : Set α := Aᶜ\n\
             def i (α β : Type) (f : α -> β) (A : Set α) : Set β := f '' A\n",
        )
        .expect("parse");
        // 声明级 binder 会折成 `fun … => <体>`（`wrap_decl_binders`），
        // 所以记号节点在 lambda 体里。
        fn body(val: &Expr) -> &Expr {
            match val {
                Expr::Lambda { body, .. } => body,
                other => other,
            }
        }
        let val_of = |index: usize| -> Expr {
            match &file.commands[index] {
                Command::Def { val, .. } => val.clone(),
                other => panic!("expected a def, got {other:?}"),
            }
        };
        assert!(matches!(
            body(&val_of(3)),
            Expr::Notation {
                assoc: NotationAssoc::Prefix,
                rhs: Some(_),
                lhs: None,
                ..
            }
        ));
        assert!(matches!(
            body(&val_of(4)),
            Expr::Notation {
                assoc: NotationAssoc::Postfix,
                lhs: Some(_),
                rhs: None,
                ..
            }
        ));
        assert!(matches!(
            body(&val_of(5)),
            Expr::Notation {
                assoc: NotationAssoc::Infixr,
                ..
            }
        ));
    }

    #[test]
    fn a_binder_symbol_in_operator_position_is_a_teaching_error() {
        // 第三刀 §12.1：**binder** 记法符号落在算子位上仍然是教学错误
        // （它没有优先级、也不在爬升梯子上）。
        let err = parse(
            "binder_notation \" ∃ \" => Exists\n\
             axiom bad : Prop -> Prop -> Prop\n\
             axiom p : Prop\n\
             axiom q : Prop\n\
             #check p ∃ q\n",
        )
        .expect_err("binder symbol in operator position");
        assert_eq!(err.code(), "unexpected-token");
        assert!(err.message.contains("binder"), "message: {}", err.message);
    }

    /// 第三刀 §12.1：两段式 binder 的关系符号没声明时，报的是**未声明符号**
    /// 的专用诊断（不是通用的 "expected a binder"）。
    #[test]
    fn an_undeclared_two_stage_binder_relation_is_an_unknown_symbol() {
        let err = parse("axiom p : Prop -> Prop\n#check forall x ∈ p, p x\n")
            .expect_err("an undeclared relation symbol");
        assert_eq!(err.code(), "notation-unknown-symbol");
        assert!(err.message.contains('∈'), "message: {}", err.message);
    }

    /// 第三刀 §12.5：一元**前缀**记法在实参位免括号——`f 𝒫 A` 就是
    /// `f (𝒫 A)`。副作用（已记进设计 §13）：`A 𝒫 B` 也从"响亮的 parse 错"
    /// 变成 `A (𝒫 B)`——两者形状完全一样，无法只放行前者。
    #[test]
    fn a_prefix_notation_is_an_argument_without_parentheses() {
        let file = parse(
            "prefix:100 \" 𝒫 \" => Set.powerset\n\
             def bad (α : Type) (A B : Set α) : Set (Set α) := A 𝒫 B\n",
        )
        .expect("prefix notation in argument position");
        let crate::ast::Command::Def { val, .. } = &file.commands[1] else {
            panic!("expected a def");
        };
        // 声明 binder 会把值位包成 lambda 望远镜，取最内层的 body。
        let mut body = val;
        while let Expr::Lambda { body: inner, .. } = body {
            body = inner;
        }
        let Expr::App { fun, arg, .. } = body else {
            panic!("expected an application, got {body:?}");
        };
        assert!(matches!(fun.as_ref(), Expr::Ident { name, .. } if name == "A"));
        assert!(
            matches!(arg.as_ref(), Expr::Notation { symbol, .. } if symbol == "𝒫"),
            "the argument is the prefix notation: {arg:?}"
        );
    }

    #[test]
    fn abbrev_and_def_produce_the_same_command() {
        // G-08：`abbrev` 是 `def` 的**同语义拼写**——同 AST、同后续流水线
        // （设计 §2）。真 Lean 的 `abbrev` 行因此能原样编。
        let src = "Set (α : Type) : Type := α -> Prop\n";
        assert_eq!(
            def_shape_of(&format!("def {src}")),
            def_shape_of(&format!("abbrev {src}")),
            "abbrev must elaborate to the very same Command::Def as def"
        );
        // 宇宙参数与声明级 binder 也逐字继承（共享 `parse_def`）。
        let src = "f {u} (a : Sort u) : Sort u := a\n";
        assert_eq!(
            def_shape_of(&format!("def {src}")),
            def_shape_of(&format!("abbrev {src}"))
        );
    }

    #[test]
    fn abbrev_is_a_reserved_command_not_an_identifier() {
        // 与 `def`/`theorem` 同族：命令关键字在**表达式位**被拒（名字位与
        // `def theorem …` 一样宽松——既有语义，一字不改）。
        let err = parse("def f : Type := abbrev\n").expect_err("abbrev as an expression");
        assert_eq!(err.code(), "unexpected-token");
        // 反向对照：`abbrev` 真的是命令（同样两行把关键字换成 `def` 是合法文件）。
        assert!(parse("def abbrev : Type := Prop\n").is_ok());
        assert!(parse("abbrev abbrev : Type := Prop\n").is_ok());
    }

    #[test]
    fn abbrev_inside_a_namespace_gets_the_prefix() {
        // 命名空间前缀在 parser 里落定（G-05 N3），`abbrev` 走同一条路。
        let file = parse("namespace A\nabbrev Set (α : Type) : Type := α -> Prop\nend A\n")
            .expect("parse");
        assert!(matches!(&file.commands[1], Command::Def { name, .. } if name == "A.Set"));
    }
}
