//! Name→source-position helpers for rename / find-references (seeded by the
//! integration pass; find-references consumers live with the LSP layer).
//!
//! Everything here is **token-precise**: spans come from the lexer over the
//! exact source slice, never from text scanning of identifiers.

use crate::compile::{HoverType, ResolvedTarget};
use crate::span::Span;
use crate::token::{tokenize, TokenKind};

/// The name token's span (document coordinates) inside the defining command
/// that declares `name` (`def`/`theorem`/`axiom` and inductive ctors/recs).
/// The first identifier matching `name` after the leading keyword is the
/// defining occurrence — declaration names precede their types by grammar.
///
/// G-02 / WO-005：构造子的**规范名**（`Wrap.mk`）在源里写的是裸名
/// （`ctor mk`），所以按全名找不到时回退匹配**最后一段** —— 仍是词法精确
/// 匹配（token 相等），不是文本扫描。
pub fn decl_name_span(doc: &str, decl_span: Span, name: &str) -> Option<Span> {
    let slice = slice_of(doc, decl_span)?;
    if let Some(span) = first_ident_span(slice, doc, decl_span.start.offset, Some(name)) {
        return Some(span);
    }
    let tail = name.rsplit('.').next()?;
    if tail == name {
        return None; // no dot: the full-name lookup above already covered it
    }
    first_ident_span(slice, doc, decl_span.start.offset, Some(tail))
}

/// The binder name token's span (document coordinates) inside a binder's
/// source span (`(x : Prop)` / `{x : A}`); the slice's first identifier.
pub fn binder_name_span(doc: &str, binder_span: Span) -> Option<Span> {
    let slice = slice_of(doc, binder_span)?;
    first_ident_span(slice, doc, binder_span.start.offset, None)
}

/// The target under the cursor, resolving in both directions:
/// - a use point resolves through its recorded resolution (the smallest
///   enclosing use wins, so shadowed inner binders take precedence);
/// - a definition point (cursor inside the binder or the defining command)
///   resolves to itself, found through any use whose resolution's definition
///   span covers the cursor — the same bidirectional rule as the LSP
///   document highlight (crates/lsp/src/render.rs::highlight_uses).
///
/// `None` when the cursor sits on a prelude name, on a **notation symbol**,
/// or outside any name.
///
/// **T-D30（正确性 bug）**：`doc` 是必需的——光标落在**记法符号**（`∈`/`⊆`/`∧`…）
/// 上时必须直接答 `None`。以前没有这条守卫，第二个回退（"找一条 target span
/// 覆盖光标的使用点"）会命中**外层 binder**：binder 的 span 覆盖**整段类型标注**
/// （`(h : a ∈ A)`），于是 `∈` 被解析成 `h`，`rename` 会去改 `h`。
/// 同一个病在 LSP 的 `render.rs::highlight_uses` 里也有一份（documentHighlight）。
pub fn resolve_at(
    doc: &str,
    hovers: &[HoverType],
    line: u32,
    character: u32,
) -> Option<ResolvedTarget> {
    // 光标在记法符号上 ⇒ **不是名字**，两个回退都不该往下走。
    if cursor_is_on_notation(doc, line, character) {
        return None;
    }
    // Use point: the smallest enclosing hover's own resolution.
    let smallest = hovers
        .iter()
        .filter(|h| pos_within(line, character, h.span))
        .min_by_key(|h| h.span.end.offset.saturating_sub(h.span.start.offset));
    if let Some(target) = smallest.and_then(|h| h.resolution.clone()) {
        return Some(target);
    }
    // Definition point: a use of the definition under the cursor points back.
    hovers.iter().find_map(|h| {
        let target = h.resolution.clone()?;
        pos_within(line, character, target.span()).then_some(target)
    })
}

/// 光标是不是落在**记法符号**上（T-D30 的守卫）。
///
/// 判据走词法（`notation_input::symbol_at`）：它认得本文件声明的符号、语言内建的、
/// 以及**输入法表**里的符号——正是"看起来像符号、但不是名字"的那一类。
/// 用 (line, character) 换算成 offset（都是 1-based / 0-based 的既有约定）。
fn cursor_is_on_notation(doc: &str, line: u32, character: u32) -> bool {
    let Some(offset) = offset_of(doc, line, character) else {
        return false;
    };
    crate::notation_input::symbol_at(doc, offset).is_some()
}

/// LSP (0-based) 位置 → 字节 offset。越界返回 `None`。
///
/// `character` 按**字符**计数（不是字节）：行里只要有 `α`/`∈` 这类多字节字符，
/// 按字节算就会错位——实测踩到过（`(h : a ∈ A)` 的 `∈` 前面有 4 个 `α`，
/// 按字节算的 offset 落到别处 ⇒ 守卫不触发、`rename` 照样改 `h`）。
/// 与 LSP 侧 `position_to_offset` 同一口径；星平面符号的偏差是 **T-D31** 的
/// 独立缺口（那边要改成 UTF-16 码元，两边一起改）。
fn offset_of(doc: &str, line: u32, character: u32) -> Option<usize> {
    let mut current = 0usize;
    for (index, text) in doc.split_inclusive('\n').enumerate() {
        if index == line as usize {
            let add = text
                .char_indices()
                .nth(character as usize)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            return Some(current + add);
        }
        current += text.len();
    }
    None
}

/// LSP (0-based) position inside a front (1-based) span.
fn pos_within(line: u32, character: u32, span: Span) -> bool {
    let (l, c) = (line as usize + 1, character as usize + 1);
    (l, c) >= (span.start.line, span.start.column) && (l, c) < (span.end.line, span.end.column)
}

/// All use points of `target`: spans of name uses whose resolution is
/// `target`, ascending by offset. The definition site itself is *not*
/// included (callers append it for `include_declaration`).
pub fn references_for(hovers: &[HoverType], target: &ResolvedTarget) -> Vec<Span> {
    let mut spans: Vec<Span> = hovers
        .iter()
        .filter(|h| h.resolution.as_ref() == Some(target))
        .map(|h| h.span)
        .collect();
    spans.sort_by_key(|s| s.start.offset);
    spans.dedup_by_key(|s| s.start.offset);
    spans
}

fn slice_of(doc: &str, span: Span) -> Option<&str> {
    let start = span.start.offset.min(doc.len());
    let end = span.end.offset.clamp(start, doc.len());
    doc.get(start..end)
}

/// Shift a slice-relative token span into document coordinates (offset and
/// line/column all recomputed against `doc`).
fn rebase(mut span: Span, doc: &str, base: usize) -> Span {
    span.start.offset += base;
    span.end.offset += base;
    let (line, column) = line_col_of(doc, span.start.offset);
    span.start.line = line;
    span.start.column = column;
    let (line, column) = line_col_of(doc, span.end.offset);
    span.end.line = line;
    span.end.column = column;
    span
}

/// 位置换算：字节 offset → 1-based 行/列（**列按 UTF-16 code unit** ✓）。
///
/// **审计 #5（2026-09-25）修** ✗⇒✓：这里原来是
/// `column = offset - line_start + 1` ⇒ **字节列** ✗，而消费端
/// （`lsp/render.rs`、`lsp/project_refs.rs`）把 `span.column - 1` 当 LSP 的
/// `character`（**UTF-16** ✓）发出去 ⇒ 行里有 `α`/`∈` 时 rename/highlight 的
/// range **右移** ✗。既有夹具是**纯 ASCII** ⇒ 两种口径恒等 ⇒ 一直咬不住 ✓
/// （判据已补：`line_col_counts_utf16_units_not_bytes` ✓）。
/// **唯一归属**：`crate::query::line_col_of`（re-export 自 `query::pos` ✓） ✓ —— 真相层只该有**一份**换算 ✓。
fn line_col_of(doc: &str, offset: usize) -> (usize, usize) {
    crate::query::line_col_of(doc, offset)
}

fn first_ident_span(slice: &str, doc: &str, base: usize, name: Option<&str>) -> Option<Span> {
    let tokens = tokenize(slice).ok()?;
    for (position, token) in tokens.iter().enumerate() {
        let TokenKind::Ident(text) = &token.kind else {
            continue;
        };
        // Skip the leading keyword ident (`def`/`theorem`/`axiom`/`ctor`…):
        // the defining name is never the first token of a command slice.
        // Binder slices start with a paren/brace, so nothing is skipped there.
        if position == 0 {
            continue;
        }
        let matches = match name {
            Some(want) => text == want,
            None => true,
        };
        if matches {
            return Some(rebase(token.span, doc, base));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **审计 #5（2026-09-25）**：`line_col_of` 的列必须是 **UTF-16 code unit**
    /// （= LSP 的 `character` 口径 ✓），不是**字节** ✗。
    ///
    /// 为什么必须有一条**含非 ASCII** 的判据：老实现用
    /// `offset - line_start + 1`（**字节** ✗），而整个既有夹具是**纯 ASCII** ✓
    /// ⇒ 两者**恒等** ⇒ 这个 bug 一直咬不住 ✓。消费端（`lsp/render.rs`、
    /// `lsp/project_refs.rs`）把 `span.column - 1` 当 LSP `character` 发出去 ✓
    /// ⇒ 行里有 `α`/`∈` 时 rename/highlight 的 range 会**右移** ✗。
    #[test]
    fn line_col_counts_utf16_units_not_bytes() {
        // `α` 是 2 字节、1 个 UTF-16 code unit ✓；`x` 在它后面。
        let doc = "def α : Prop := x\n";
        let x = doc.find('x').expect("夹具里有 x");
        assert_eq!(x, 17, "字节偏移（α 占 2 字节 ⇒ 比 code unit 多 1 ✓）");
        let (line, col) = line_col_of(doc, x);
        assert_eq!(line, 1);
        // 列按 **UTF-16**：`def α : Prop := ` 在 `x` 前有 16 个 code unit ⇒ 第 17 列 ✓
        assert_eq!(
            col, 17,
            "列必须是 UTF-16 code unit（LSP character 口径 ✓），不是字节 ✗"
        );
        // 更锋利的一条：**两个**多字节字符 ⇒ 两种口径的差距从 1 变 2 ✓
        let doc2 = "def αα : Prop := y\n";
        let y = doc2.find('y').expect("夹具里有 y");
        assert_eq!(y, 19, "字节偏移（两个 α ⇒ 多 2 ✓）");
        let (_, col2) = line_col_of(doc2, y);
        assert_eq!(col2, 18, "两个 α ⇒ UTF-16 列 18 ✓（字节口径会给 20 ✗）");
    }
    use crate::span::Pos;

    const DOC: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n";
    // offsets: "def " 0..4, "id" 4..6, "(x : Prop)" 29..39, "x" 30..31,
    // "#check " 45..52, final "id" 52..54.

    fn span(start: usize, end: usize) -> Span {
        let (sl, sc) = line_col_of(DOC, start);
        let (el, ec) = line_col_of(DOC, end);
        Span {
            start: Pos {
                offset: start,
                line: sl,
                column: sc,
            },
            end: Pos {
                offset: end,
                line: el,
                column: ec,
            },
        }
    }

    #[test]
    fn decl_name_span_covers_the_name_token() {
        let span = decl_name_span(DOC, span(0, 44), "id").expect("name token found");
        assert_eq!(&DOC[span.start.offset..span.end.offset], "id");
        assert_eq!(span.start.offset, 4);
        assert_eq!(span.end.offset, 6);
        assert_eq!((span.start.line, span.start.column), (1, 5));
    }

    #[test]
    fn binder_name_span_is_the_first_ident() {
        let span = binder_name_span(DOC, span(29, 39)).expect("binder name found");
        assert_eq!(&DOC[span.start.offset..span.end.offset], "x");
        assert_eq!(span.start.offset, 30);
    }

    #[test]
    fn references_for_collects_use_points_in_order() {
        let report = crate::compile::check_document(&crate::parser::parse(DOC).expect("parses"));
        let def_span = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("id"))
            .expect("the id declaration")
            .span;
        let target = ResolvedTarget::Declaration {
            name: "id".to_string(),
            span: def_span,
        };
        let uses = references_for(&report.hovers, &target);
        assert_eq!(uses.len(), 1, "the `#check id` use: {uses:?}");
        assert_eq!(&DOC[uses[0].start.offset..uses[0].end.offset], "id");
        assert_eq!(uses[0].start.offset, 52);
    }

    /// G-02 / WO-005：构造子的定义名 token 在**源**里是裸名（`ctor mk`），而
    /// 解析目标带的是规范名（`Wrap.mk`）——`decl_name_span` 必须回退匹配最后
    /// 一段，否则 ctor 的 prepareRename/goto 会静默失效。
    #[test]
    fn decl_name_span_finds_the_source_token_of_a_canonical_ctor_name() {
        const CTOR_DOC: &str =
            "inductive Wrap : Type\nctor mk : Wrap\nend\ndef w : Wrap := Wrap.mk\n";
        let report =
            crate::compile::check_document(&crate::parser::parse(CTOR_DOC).expect("parses"));
        let decl = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("Wrap"))
            .expect("the Wrap declaration");
        let span = decl_name_span(CTOR_DOC, decl.span, "Wrap.mk")
            .expect("the ctor's source token must be found under its canonical name");
        assert_eq!(&CTOR_DOC[span.start.offset..span.end.offset], "mk");
        // 使用点解析到规范名（hover/goto 回填）。
        let uses: Vec<&str> = report
            .hovers
            .iter()
            .filter_map(|h| match h.resolution.as_ref() {
                Some(ResolvedTarget::Declaration { name, span }) if span.start.offset > 0 => {
                    Some(name.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(
            uses.contains(&"Wrap.mk"),
            "the use must resolve to the canonical name: {uses:?}"
        );
    }

    #[test]
    fn references_for_empty_without_resolutions() {
        let report = crate::compile::check_document(&crate::parser::parse(DOC).expect("parses"));
        let target = ResolvedTarget::Declaration {
            name: "ghost".to_string(),
            span: Span::default(),
        };
        assert!(references_for(&report.hovers, &target).is_empty());
    }

    // ---- resolve_at：光标处双向解析（rename / find-references 共用）----
    // 光标坐标是 LSP 0-based（行号、行内字符），与 front 的行内 1-based 列对齐。

    const SHADOW: &str =
        "def f : Prop -> Prop -> Prop := fun (x : Prop) => fun (x : Prop) => x\n#check f\n";
    // offsets (line 0): outer binder "(x : Prop)" 36..46, inner binder 54..64,
    // inner body use `x` 68..69.

    const CROSS: &str =
        "def f : Prop -> Prop := fun (x : Prop) => x\ndef g : Prop -> Prop := fun (x : Prop) => x\n";
    // offsets: f's binder 28..38 (line 0); g's binder 72..82, g's body use
    // 86..87 — both on line 1, the use at line-character 42.

    /// **T-D30 的判据**：光标在 `(h : a ∈ A)` 的 **`∈`** 上时，不得解析到 `h`。
    ///
    /// 改前实测：`resolve_at` 的第二个回退（"找一条 target span 覆盖光标的使用点"）
    /// 会命中**外层 binder**——binder 的 span 覆盖**整段类型标注**（`(h : a ∈ A)`）
    /// ⇒ `∈` 被解析成 `h`，`rename` 会去改 `h`（同一个病在 LSP 的
    /// `render.rs::highlight_uses` 里也有一份）。
    ///
    /// 修法：光标落在**记法符号**上就直接答 `None`（判据走词法：本文件声明的 /
    /// 内建的 / 输入法表里的符号都认得出来）。
    #[test]
    fn resolve_at_does_not_mistake_a_notation_symbol_for_the_enclosing_binder() {
        const SRC: &str = "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
theorem t (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
        let report = check_report(SRC);
        // 第 4 行（0-based 3）里 `(h : a ∈ A)` 那个 `∈` 的列。
        let line = 3u32;
        let line_text = SRC.lines().nth(line as usize).expect("第四行");
        // ⚠ `character` 是**字符**计数（LSP 口径），而 Rust 的 `str::find` 给的是
        // **字节**下标——行里有 `α`/`∈`，两者不等（实测踩到：传字节下标 ⇒ offset
        // 落到 `) :` 上 ⇒ 守卫不触发）。
        let col = line_text[..line_text.find('∈').expect("那一行有 ∈")]
            .chars()
            .count() as u32;
        let resolved = resolve_at(SRC, &report.hovers, line, col);
        assert!(resolved.is_none(), "记法符号上没有名字可解析：{resolved:?}");

        // 对照：**同一个 binder 的 `h` 本身**仍然解析得到（别把定义点那一支修坏）
        // ——它的 span 就是 `(h : a ∈ A)` 那一段（binder 的 span 覆盖整段标注，
        // 这正是误命中的来源）。
        let h_col = line_text[..line_text.find("(h :").expect("binder 在那一行")]
            .chars()
            .count() as u32
            + 1;
        let resolved = resolve_at(SRC, &report.hovers, line, h_col);
        let Some(ResolvedTarget::Binder(span)) = resolved else {
            panic!("光标在 `h` 上仍应解析到 binder：{resolved:?}");
        };
        let binder_text = &SRC[span.start.offset..span.end.offset];
        assert!(
            binder_text.starts_with("(h :") && binder_text.contains('∈'),
            "解析到的是 `h` 那个 binder：{binder_text:?}"
        );
    }

    fn check_report(doc: &str) -> crate::compile::DocumentReport {
        crate::compile::check_document(&crate::parser::parse(doc).expect("parses"))
    }

    #[test]
    fn resolve_at_resolves_a_use_point_to_its_target() {
        let report = check_report(DOC);
        // `#check id` sits on line 1; `id` starts at line-character 7.
        let target = resolve_at(DOC, &report.hovers, 1, 7).expect("use point resolves");
        let ResolvedTarget::Declaration { name, span } = target else {
            panic!("a top-level use resolves to its declaration, got {target:?}");
        };
        assert_eq!(name, "id");
        assert_eq!(span.start.offset, 0, "the whole defining command");
    }

    #[test]
    fn resolve_at_resolves_the_definition_under_the_cursor() {
        let report = check_report(DOC);
        // Cursor on the binder `(x : Prop)`: the body use points back at it.
        let target = resolve_at(CROSS, &report.hovers, 0, 30).expect("binder resolves");
        assert!(
            matches!(target, ResolvedTarget::Binder(s) if s.start.offset == 29),
            "the binder's own span, got {target:?}"
        );
    }

    #[test]
    fn resolve_at_is_none_for_prelude_names() {
        // No use points anywhere: a cursor on the type-position `Prop`
        // (line-character 9..13) has nothing to resolve through.
        let report = check_report("def id : Prop -> Prop := fun (x : Prop) => x\n");
        assert!(resolve_at(CROSS, &report.hovers, 0, 9 + 1).is_none());
    }

    #[test]
    fn resolve_at_inner_binder_wins_over_outer_shadow() {
        let report = check_report(SHADOW);
        let target = resolve_at(SHADOW, &report.hovers, 0, 68).expect("inner body use resolves");
        let ResolvedTarget::Binder(inner) = target else {
            panic!("shadowed use must resolve to its binder, got {target:?}");
        };
        assert_eq!(
            inner.start.offset, 54,
            "the inner binder, not the outer (36..46)"
        );
        let uses = references_for(&report.hovers, &ResolvedTarget::Binder(inner));
        assert_eq!(uses.len(), 1, "only the inner use: {uses:?}");
        assert_eq!(uses[0].start.offset, 68);
        // The outer binder must not inherit the inner use.
        let outer = ResolvedTarget::Binder(span(36, 46));
        assert!(
            references_for(&report.hovers, &outer).is_empty(),
            "the outer shadowed binder has no uses of its own"
        );
    }

    #[test]
    fn references_for_same_named_binders_in_different_decls_stay_separate() {
        let report = check_report(CROSS);
        // Cursor inside g's body `x` (second line, character 42).
        let target = resolve_at(SHADOW, &report.hovers, 1, 42).expect("g's body use resolves");
        let ResolvedTarget::Binder(g_binder) = target else {
            panic!("g's use resolves to g's binder, got {target:?}");
        };
        assert_eq!(g_binder.start.offset, 72, "g's own binder, not f's");
        let uses = references_for(&report.hovers, &ResolvedTarget::Binder(g_binder));
        assert_eq!(uses.len(), 1, "only g's use: {uses:?}");
        assert_eq!(uses[0].start.offset, 86);
        let f_target = ResolvedTarget::Binder(span(28, 38));
        let f_uses = references_for(&report.hovers, &f_target);
        assert_eq!(f_uses.len(), 1, "f's own body use only: {f_uses:?}");
        assert_eq!(
            f_uses[0].start.offset, 42,
            "f's binder keeps its own use and never g's (86)"
        );
    }
}
