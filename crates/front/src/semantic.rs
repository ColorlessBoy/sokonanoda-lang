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
    "def",
    "theorem",
    "example",
    "axiom",
    "inductive",
    "ctor",
    "rec",
    "iota",
    "end",
    "fun",
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
    for cmd in &file.commands {
        match cmd {
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
                walk_expr(ty, toks, names);
                for ctor in constructors {
                    names.ctors.insert(ctor.name.clone());
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
        }
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
    // 声明点 span 优先于一切使用侧分类（含关键字/排序名同名等边角情况）。
    if let Some(kind) = names.special.get(&offset) {
        return *kind;
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
        let src = "def two : Nat := 2\nexample : Sort 1 := ???\n";
        let spans = semantic_tokens(src);
        assert_eq!(find(src, &spans, "def").kind, SemanticKind::Keyword);
        assert_eq!(find(src, &spans, "two").kind, SemanticKind::DefName);
        assert_eq!(find(src, &spans, "Nat").kind, SemanticKind::UnknownIdent);
        assert_eq!(find(src, &spans, "2").kind, SemanticKind::Number);
        assert_eq!(find(src, &spans, "example").kind, SemanticKind::Keyword);
        assert_eq!(find(src, &spans, "Sort").kind, SemanticKind::Sort);
        assert_eq!(find(src, &spans, "1").kind, SemanticKind::Number);
        assert_eq!(find(src, &spans, "???").kind, SemanticKind::Hole);
        // 有序且互不重叠
        for pair in spans.windows(2) {
            assert!(pair[0].span.start.offset < pair[1].span.start.offset);
            assert!(pair[0].span.end.offset <= pair[1].span.start.offset);
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
}
