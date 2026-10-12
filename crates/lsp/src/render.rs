//! Rendering helpers: convert front-end compile/parse errors, spans and
//! declarations into LSP diagnostics, ranges, hovers and symbols.

use sokonanoda_front::compile::{
    DeclKind, DeclState, DeclStatus, DocumentReport, HoverType, ResolvedTarget,
};
use sokonanoda_front::references::{binder_name_span, decl_name_span, references_for, resolve_at};
use sokonanoda_front::semantic::SemanticKind;
use sokonanoda_front::{tokenize, Pos, Span, TokenKind};
use tower_lsp::lsp_types::*;

/// A `Pos` (1-based line/column, matching front spans) at a byte offset.
fn pos_at(text: &str, offset: usize) -> Pos {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = (offset - before.rfind('\n').map(|i| i + 1).unwrap_or(0)) + 1;
    Pos {
        offset,
        line,
        column,
    }
}

pub(crate) fn diagnostic_from_compile(err: &sokonanoda_front::compile::CompileError) -> Diagnostic {
    Diagnostic {
        range: range_of(err.span),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(err.code().to_string())),
        code_description: None,
        source: Some("sokonanoda".to_string()),
        message: format!("{}\n\n提示：{}", err.message, err.hint()),
        related_information: None,
        tags: None,
        data: None,
    }
}

pub(crate) fn diagnostic_from_parse(diag: &sokonanoda_front::Diagnostic) -> Diagnostic {
    Diagnostic {
        range: range_of(diag.span),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(diag.code().to_string())),
        code_description: None,
        source: Some("sokonanoda".to_string()),
        message: format!("{}\n\n提示：{}", diag.message, diag.hint()),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// **URI 规范化**（N1）：同一个文件只能有**一个** `file://` 身份。
///
/// 路径可能带 `.` / `..` —— 前端的 `prelude_source_path()` 就是从
/// `crates/front/../..` 拼出来的 ⇒ `Url::from_file_path` 把它们**原样**编进
/// URI，`Location.uri` 于是给出 `…/crates/front/../../prelude/Prelude.sokonanoda`
/// （实测 3 条落点带 `/../`）。编辑器拿它当**另一个文件**：同一个 prelude
/// 会有两份身份（已打开文档的高亮/面包屑/`documentSymbol` 全部分叉 ✗）。
///
/// 这里做**词法**规范化（逐组件消掉 `.` 与 `..`，不碰文件系统）：
/// 判据是"同一个字符串路径只有一个 URI"，不是"解析真实路径" —— 后者是
/// `canonicalize` 的活（会解析符号链接、在 macOS 上还会把 `/var` 换
/// `/private/var` ⇒ 与编辑器手里那份 URI **反而不一致** ✗）。
pub(crate) fn file_uri(path: &std::path::Path) -> Option<Url> {
    // 逃生门（反向验证用）：`SOKO_NO_URI_NORMALIZE=1` 回到"原样编码"的旧行为
    // ⇒ N1 用例必须重新判红（读数见 VERDICT）。
    if std::env::var_os("SOKO_NO_URI_NORMALIZE").is_some() {
        return Url::from_file_path(path).ok();
    }
    Url::from_file_path(normalize_path(path)).ok()
}

/// 词法规范化：消掉 `.`、就地消掉 `..`（`..` 落在根上时保留，交给 `Url` 处理）。
fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub(crate) fn range_of(span: Span) -> Range {
    Range {
        start: Position {
            line: span.start.line.saturating_sub(1) as u32,
            character: span.start.column.saturating_sub(1) as u32,
        },
        end: Position {
            line: span.end.line.saturating_sub(1) as u32,
            character: span.end.column.saturating_sub(1) as u32,
        },
    }
}

/// [`range_of`] 的**文本感知**版本：按 **byte offset** 重算行/列。
///
/// 为什么需要它（**G-36 那条教训的反方向**）：front 的 `Pos.column` 是**按 char
/// 数**算的（`crates/front/src/token.rs` 的 `self.column += 1`），而 LSP 的
/// `character` 是 **UTF-16 码元** ⇒ 行里有星平面字符（`𝒫`/emoji）时 `range_of`
/// 整条偏（G-55 实测：`prefix:70 " 𝒫 " => Set.powerset` 那行上目标名报成
/// `19..31`（框住 `" Set.powerse"`），正确是 `20..32` ✓）。offset 是两边
/// **共有**的坐标系 ⇒ 用它重算不会错 ✓。
///
/// ⚠ **只服务 G-55 那条分支**：全局 `range_of` 的口径**不动**（hover / definition /
/// 诊断 / 符号高亮都在用它，改它是**另一条**——同族未覆盖项记在
/// `docs/design/notation-subset.md` §15 的清单里 ✗）。
pub(crate) fn range_of_in(doc: &str, span: Span) -> Range {
    Range {
        start: position_of_offset(doc, span.start.offset),
        end: position_of_offset(doc, span.end.offset),
    }
}

/// byte offset → LSP 位置（`character` 按 UTF-16 码元计 ✓）。
fn position_of_offset(doc: &str, offset: usize) -> Position {
    let offset = offset.min(doc.len());
    let before = &doc[..offset];
    let line = before.matches('\n').count() as u32;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character: usize = doc[line_start..offset].chars().map(char::len_utf16).sum();
    Position {
        line,
        character: character as u32,
    }
}

/// **G-36 第三处（2026-10-03 ✓）**：按 **offset** 判"是否落在 span 内" ✓。
/// 旧的 [`pos_within_span`] 拿 LSP 的 **UTF-16 列** 去比 **span 的列** ✗ —— 而 span 的列是
/// 前端 parser 按**自己的约定**算的 ✗ ⇒ `𝒫` 这类星平面字符之后**判错 span** ✓
///（实测：光标在 `A` 上，却命中覆盖整条 `𝒫 A = 𝒫 A` 的宽 span ⇒ hover 答外层表达式 ✗）。
/// offset 是两边**共有的**坐标系 ✓ ⇒ 比它不会错 ✓。
pub(crate) fn offset_within_span(offset: usize, span: Span) -> bool {
    span.start.offset <= offset && offset < span.end.offset
}

pub(crate) fn pos_within_span(line: u32, character: u32, span: Span) -> bool {
    let l = line as usize + 1;
    let c = character as usize + 1;
    let after_start = (l, c) > (span.start.line, span.start.column)
        || (l, c) >= (span.start.line, span.start.column);
    let before_end = (l, c) < (span.end.line, span.end.column);
    after_start && before_end
}

/// **G-36 第三处**：按 **offset** 取"包含该位置的最小 span" ✓（见 [`offset_within_span`] ✓）。
pub(crate) fn hover_type_at_offset(hovers: &[HoverType], offset: usize) -> Option<&HoverType> {
    innermost(hovers.iter().filter(|h| offset_within_span(offset, h.span)))
}

/// **同 span 多行时选"这一段源码自己那一行"**（H1 的修法点）。
///
/// `hovers` 是**按 elaborate 顺序追加**的，同一段源码会被 elaboration **多次**
/// 记行 —— 实测（`node scripts/ux-probes/lsp-probe.mjs --case H1-notation-hover-type`
/// 与 `theorem t (p : Prop) : p ↔ p := by rfl` 的行转储）：记法节点 `a = b` /
/// `p ↔ p` 上最多有**三行同 span**：
///   * **目标常量自己的签名**（`∀ {α : Type 0}, α → α → Prop`，`resolution=None`）
///     —— 前端解析记法目标时顺带记下的**过程产物**；
///   * **记法节点自己那一行**（`text = Prop`，`resolution = Notation{…}`）✓ 要的就是它；
///   * **by 引擎**在同一个 span 上记的一行（`text` = 那个证明项的类型，
///     如 `p ↔ p`）—— `by rfl` 的合成项落在目标 span 上。
///
/// ⚠ 所以"取第一条"（旧）与"取最后一条"（我第一版，实测在 `by rfl` 的声明上
/// 翻车：末块给 `(a ∈ A) ↔ (a ∈ A)`）**都不对**。判据只能是**语义**：
/// 记法节点自己那一行的 `resolution` 指向**这个符号自己**（T-D15 的
/// `ResolvedTarget::Notation`，它的 span 落在本行 span **里面**）—— 这是 front
/// 已经算好的真相，不是文本启发式 ✓。没有这样一行时退回**旧行为**（第一条），
/// 改动面最小。
fn innermost<'a, I>(rows: I) -> Option<&'a HoverType>
where
    I: DoubleEndedIterator<Item = &'a HoverType> + Clone,
{
    let width = |h: &HoverType| -> u64 {
        (h.span.end.offset - h.span.start.offset)
            .try_into()
            .unwrap_or(u64::MAX)
    };
    let min = rows.clone().map(width).min()?;
    let tied: Vec<&'a HoverType> = rows.filter(|h| width(h) == min).collect();
    // 逃生门（反向验证用）：`SOKO_HOVER_FIRST_ROW=1` 回到"取第一个同宽行"
    // 的旧行为 ⇒ H1 用例必须重新判红（读数见 VERDICT）。
    if std::env::var_os("SOKO_HOVER_FIRST_ROW").is_none() {
        let own = tied.iter().rev().find(|h| {
            matches!(
                h.resolution.as_ref(),
                Some(ResolvedTarget::Notation { span, .. })
                    if span.start.offset >= h.span.start.offset
                        && span.end.offset <= h.span.end.offset
            )
        });
        if let Some(row) = own {
            return Some(row);
        }
    }
    tied.first().copied()
}

pub(crate) fn hover_type_at(hovers: &[HoverType], line: u32, character: u32) -> Option<&HoverType> {
    // 精确命中：包含光标的最小 span 优先。
    // （曾经的「起点 ±2」邻近回退会在 `)` 上命中右侧邻居——括号命中
    // 现在统一走 bracket_hover_at 的括号组匹配。）
    innermost(
        hovers
            .iter()
            .filter(|h| pos_within_span(line, character, h.span)),
    )
}

/// A resolved hover answer: the source span to highlight and the display text.
pub(crate) struct HoverResolved {
    /// Source span to highlight (always contains the cursor).
    pub(crate) range: Span,
    /// Display text (`expr : type`, or a binder declaration).
    pub(crate) content: String,
}

/// Net `(` minus `)` in `[start, end)` (byte offsets), skipping `--` comments.
/// Used to detect the truncated right edge of an AST span (`And a (Not a`).
fn paren_delta(text: &str, start: usize, end: usize) -> isize {
    let bytes = text.as_bytes();
    let mut d = 0isize;
    let mut i = start;
    while i < end {
        match bytes[i] {
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                while i < end && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => {
                d += 1;
                i += 1;
            }
            b')' => {
                d -= 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    d
}

/// Extend a hover span's right edge so slicing yields a well-formed expression
/// (AST spans exclude `)`, producing `And a (Not a`). Consumes the trailing
/// `)` needed to balance the span's own parens, skipping comments.
fn balanced_span(text: &str, span: Span) -> Span {
    let depth = paren_delta(text, span.start.offset, span.end.offset);
    if depth <= 0 {
        return span;
    }
    let bytes = text.as_bytes();
    let mut d = depth;
    let mut i = span.end.offset;
    while i < bytes.len() && d > 0 {
        match bytes[i] {
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b')' => {
                d -= 1;
                i += 1;
            }
            b'(' => {
                d += 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    Span::new(span.start, pos_at(text, i))
}

/// [`balanced_span`] 的**左边界**版本（H4）：AST 的 span 起点会落在括号
/// **里面** —— `(a ∈ A) ↔ A a` 的记法节点从 `a` 起（`symbol_span` 只保证符号
/// 自己准），于是切片右边多出一个 `)` ⇒ 括号不配对 ✗。
///
/// 先往左补齐这一层（扫到匹配的开括号），再套 [`balanced_span`] 补右边界 ⇒
/// 两端都配平。`--` 注释在**左侧**扫描里按行跳过（与 `paren_delta` 同一条）。
pub(crate) fn balanced_span_both(text: &str, span: Span) -> Span {
    let bytes = text.as_bytes();
    // ① 左边界：span 自己的括号净亏空 ⇒ 往左找匹配的开括号。
    let mut need: i32 = -paren_delta(text, span.start.offset, span.end.offset) as i32;
    let mut start = span.start.offset;
    while need > 0 && start > 0 {
        start -= 1;
        match bytes[start] {
            b')' => need += 1,
            b'(' => need -= 1,
            // `}` / `{`（集合字面量 `{a}`）走同一套：本语言不会跨类配对。
            b'}' => need += 1,
            b'{' => need -= 1,
            _ => {}
        }
    }
    let span = if start < span.start.offset {
        Span::new(pos_at(text, start), span.end)
    } else {
        span
    };
    // ② 右边界：既有 `balanced_span` 的语义（补上 AST 没吃的那个 `)`）。
    balanced_span(text, span)
}

/// The interior of a binder annotation span, with one outer `(`/`{` …
/// `)`/`}` layer stripped (`(h : And a (Not a))` → `h : And a (Not a)`).
fn strip_binder_parens(text: &str, span: Span) -> &str {
    let t = text[span.start.offset..span.end.offset].trim();
    let inner =
        if (t.starts_with('(') && t.ends_with(')')) || (t.starts_with('{') && t.ends_with('}')) {
            &t[1..t.len() - 1]
        } else {
            t
        };
    inner.trim()
}

/// Display content + highlight span for a generic hover row: `expr : type`
/// with a paren-balanced expression, or the raw declaration for binder rows.
pub(crate) fn expr_hover(text: &str, h: &HoverType) -> HoverResolved {
    if h.binder {
        return HoverResolved {
            range: h.span,
            content: strip_binder_parens(text, h.span).to_string(),
        };
    }
    let balanced = balanced_span(text, h.span);
    let expr = text[balanced.start.offset..balanced.end.offset].trim();
    if h.text.is_empty() || h.text.contains('$') {
        HoverResolved {
            range: balanced,
            content: expr.to_string(),
        }
    } else {
        HoverResolved {
            range: balanced,
            content: format!("{} : {}", expr, h.text),
        }
    }
}

/// 光标落在 `(` / `)` 上时，解析整个括号组：`(表达式)` 的悬停 =
/// `表达式 : 表达式的类型`；`(名字 : 类型)` 这类 binder 标注组直接展示声明
/// `名字 : 类型`。高亮范围 = 整组 `( … )`（含括号——保证覆盖光标位置）。
///
/// 匹配规则：按源码文本扫描配对括号（跳过 `--` 行注释）。组内是 binder 标注
/// 时（存在 span == 整组的 binder 行），展示声明本身；否则类型取自「完全落在
/// 括号组内部的最大 hover 行」。组内文本天然良构（解析器保证括号平衡），
/// 不存在切片截断。没有配对（注释里、不闭合）时返回 None。
pub(crate) fn bracket_hover(
    text: &str,
    hovers: &[HoverType],
    line: u32,
    character: u32,
) -> Option<HoverResolved> {
    let offset = line_col_to_offset(text, line, character);
    // **G-36 trace**（`SOKO_HOVER_TRACE=1` ✓，纯诊断）：看括号 hover 究竟拿到了哪个 offset ✓
    //（上一轮"改了没效果"，就是因为不知道**是哪条分支**给出的响应 ✗）。
    if std::env::var("SOKO_HOVER_TRACE").is_ok() {
        let at = text[offset.min(text.len())..]
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "(eof)".into());
        eprintln!(
            "[hover] bracket_hover line={line} character={character} ⇒ offset={offset} 落点={at:?}"
        );
    }
    let (open, close) = matching_paren(text, offset)?;
    let group_span = Span::new(pos_at(text, open), pos_at(text, close + 1));
    // Binder 标注组：binder 行 span 恰好等于整组 span。
    if let Some(b) = hovers.iter().find(|h| h.binder && h.span == group_span) {
        return Some(HoverResolved {
            range: group_span,
            content: strip_binder_parens(text, b.span).to_string(),
        });
    }
    // 否则：取组内最大 span 的 hover 行作为该表达式（保留最内层透明，
    // `((p))` 四括号都显示 `p : P`；`(And.right …)` 显示整条应用链）。
    // 类型来自该行，表达式经括号平衡良构化。
    let inside = hovers
        .iter()
        .filter(|h| h.span.start.offset > open && h.span.end.offset <= close)
        .max_by_key(|h| h.span.end.offset - h.span.start.offset)?;
    let mut res = expr_hover(text, inside);
    // 高亮整组（含括号）——保证范围覆盖光标位置（光标在括号上）。
    res.range = group_span;
    Some(res)
}

/// 光标处括号的配对位置 `(open, close)`（字节偏移，不含括号本身）。
/// 光标不在 `(`/`)` 上或配对失败（注释内、不闭合）时返回 None。
fn matching_paren(text: &str, offset: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    match bytes.get(offset).copied()? {
        b'(' => {
            let mut depth = 0usize;
            let mut i = offset;
            while i < bytes.len() {
                match bytes[i] {
                    b'-' if bytes.get(i + 1) == Some(&b'-') => {
                        i += 2;
                        while i < bytes.len() && bytes[i] != b'\n' {
                            i += 1;
                        }
                    }
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((offset, i));
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }
        b')' => {
            // 反向找配对：正向扫描到光标之前为止，维护未闭合 `(` 栈；
            // 光标处的 `)` 不入循环——它的配对就是栈顶。
            let mut stack: Vec<usize> = Vec::new();
            let mut i = 0usize;
            while i < offset {
                match bytes[i] {
                    b'-' if bytes.get(i + 1) == Some(&b'-') => {
                        i += 2;
                        while i < bytes.len() && bytes[i] != b'\n' {
                            i += 1;
                        }
                        // 注释区域跨过了光标 → 光标的 `)` 在注释里，无配对。
                        if i > offset {
                            return None;
                        }
                    }
                    b'(' => stack.push(i),
                    b')' => {
                        stack.pop();
                    }
                    _ => {}
                }
                i += 1;
            }
            let open = stack.pop()?;
            Some((open, offset))
        }
        _ => None,
    }
}

pub(crate) fn decl_at(decls: &[DeclState], line: u32, character: u32) -> Option<&DeclState> {
    decls
        .iter()
        .find(|d| pos_within_span(line, character, d.span))
}

// ── F6：内核 pp 的 `Type 0` 按 **Lean 4** 印成 `Type`（显示层，判定一行不动）──────
//
// ## Lean 4 对照（本机源码 `~/Documents/lean/lean4/src/Lean/**`）
//
// * `PrettyPrinter/Delaborator/Builtins.lean:82-91` `delabSort`：
//   `Level.zero => Prop` · **`Level.succ .zero => Type`**（**层级 0 是丢掉的**）·
//   更高层 `Type n` · 非数字层 `Sort u`。
// * `…/Builtins.lean:1009`（`delabForall`）与 `:913`（`delabLam`）的 binder 域走
//   `withBindingDomain delab` ⇒ **binder 位与结论位同一条规则，没有特判**。
// * `…/Delaborator/Options.lean:56` `pp.universes` 默认 `false`，且 `delabSort`
//   **根本不读它**（它只管常量应用的层实参，`Builtins.lean:113`）⇒ 与本规则无关。
//
// ## 我们这边的缝（F6，2026-10-13）
//
// 声明列表的 `ty` 是**内核 pp 的原样搬运** —— `crates/kernel/src/pretty_printer.rs:694`
// 的 `pp_sort` 对 `Succ` **无条件**写层 ⇒ `Type 0`；而目标卡片那一半
// （`binders[].ty`）走的是**源级**渲染（写的是 `Type`）⇒ 同一份类型两处两种写法
// （Infoview 正文里 `Type 0` ×8，全是 `ty`）。内核 pp 的修法在 `crates/kernel`，
// 本层做的是**显示层**的等价改写。
//
// ## 为什么 runs 必须一起搬
//
// `ty_runs` 是 `ty` 的着色分段（T-A5：**文本与 runs 必须成对产出**）。删掉两个字节
// 而不同步搬偏移 ⇒ 从第一个 `Type 0` 起的高亮全部错位（"数据对了 ≠ 用户看见了"）。
// 这里按**分段自己的文本**切（输入满足"runs 依次拼接 == 文本"的不变量，实测 8/8
// 条声明成立），偏移由拼接位置重算 ⇒ 输出的"文本 ↔ runs"自洽是**构造保证**的。
//
// **退役条件**：内核 `pp_sort` 按 Lean 4 丢掉 `Type 0` 的层之后，本组函数整体删掉。

/// `Type 0` 里那两个字节（` 0`）在 `text` 里的字节区间。
///
/// 判据（与 Lean 4 的 `delabSort` 同口径）：`Type` 是**独立 token**
/// （前一个字节不是标识符字符）、后面恰好是**空格 + `0`**、且 `0` 后面不是数字。
/// `Type 1` / `Type u` / `Sort u` / `Prop` **一个都不动** ✓。
fn level_zero_cuts(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut cuts: Vec<(usize, usize)> = Vec::new();
    let mut from = 0usize;
    while let Some(found) = text[from..].find("Type") {
        let at = from + found;
        from = at + "Type".len();
        let prev_is_ident = at > 0 && {
            let b = bytes[at - 1];
            b.is_ascii_alphanumeric() || b == b'_' || b == b'\'' || b == b'.' || b >= 0x80
        };
        if prev_is_ident {
            continue;
        }
        let rest = &text[from..];
        let spaces = rest.len() - rest.trim_start_matches(' ').len();
        if spaces == 0 {
            continue;
        }
        let after = &rest[spaces..];
        if !after.starts_with('0') || after[1..].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        // 删掉 ` 0` 两个字节（`spaces >= 1` ⇒ `zero_at - 1` 就是那个空格）。
        let zero_at = from + spaces;
        cuts.push((zero_at - 1, zero_at + 1));
    }
    cuts
}

/// 文本版：`Type 0` → `Type`（给 Markdown 形态的卡片用；没有 runs 要搬）。
pub(crate) fn lean4_universe_text(text: &str) -> String {
    let cuts = level_zero_cuts(text);
    if cuts.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for (start, end) in &cuts {
        out.push_str(&text[cursor..*start]);
        cursor = *end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// wire 版：`ty` + `ty_runs` **成对**过同一条规则（偏移按拼接位置重算 ⇒ 自洽）。
pub(crate) fn lean4_universe_runs(
    ty: String,
    runs: Vec<crate::protocol::RunInfo>,
) -> (String, Vec<crate::protocol::RunInfo>) {
    let cuts = level_zero_cuts(&ty);
    if cuts.is_empty() {
        return (ty, runs);
    }
    let text = lean4_universe_text(&ty);
    let mut out: Vec<crate::protocol::RunInfo> = Vec::with_capacity(runs.len());
    let mut pos = 0usize;
    let mut new_pos = 0usize;
    for run in runs {
        let len = run.text.len();
        let (s, e) = (pos, pos + len);
        pos = e;
        // 这份 run 与删掉的区间**求差**（run 的 `text` 就是它在整串里的那一段）。
        let mut kept = String::new();
        let mut k = s;
        for (cs, ce) in &cuts {
            if *ce <= s || *cs >= e {
                continue;
            }
            let a = (*cs).max(s);
            let b = (*ce).min(e);
            if a > k {
                kept.push_str(&run.text[k - s..a - s]);
            }
            k = b;
        }
        if k < e {
            kept.push_str(&run.text[k - s..]);
        }
        if kept.is_empty() {
            continue;
        }
        let has_offsets = run.start.is_some() && run.end.is_some();
        let start = has_offsets.then_some(new_pos);
        let end = has_offsets.then_some(new_pos + kept.len());
        new_pos += kept.len();
        out.push(crate::protocol::RunInfo {
            text: kept,
            kind: run.kind,
            start,
            end,
        });
    }
    (text, out)
}

/// The name use point under the cursor resolves to this definition
/// (smallest enclosing use wins; prelude/unknown idents stay unresolved).
pub(crate) fn definition_at(
    hovers: &[HoverType],
    line: u32,
    character: u32,
) -> Option<ResolvedTarget> {
    hover_type_at(hovers, line, character)
        .and_then(|h| h.resolution.as_ref())
        .cloned()
}

/// All use points of the definition at the cursor: taken from the smallest
/// use containing the cursor, or from a use whose definition contains it
/// (cursor on the binder / declaration itself).
pub(crate) fn highlight_uses(
    doc: &str,
    hovers: &[HoverType],
    line: u32,
    character: u32,
) -> Option<Vec<Range>> {
    // **T-D30**：光标在**记法符号**上时不许回退到"包含光标的任意 hover 行"——
    // binder 的 span 覆盖整段类型标注（`(h : a ∈ A)`），那个回退会把 `∈` 解析成
    // `h`（同一个病在 front 的 `references::resolve_at` 里也有一份）。
    // `documentHighlight` 以前因此会把 `h` 的每一处都点亮。
    let offset = crate::position_to_offset(doc, Position::new(line, character));
    if let Some((symbol, _)) = sokonanoda_front::notation_input::symbol_at(doc, offset) {
        // **T-D24：符号上给"这个符号在本文里的每一处"**。
        //
        // 原来这里直接 `return None`（T-D30 的守卫）⇒ 光标在 `∈` 上一片空白 ✗。
        // 但那条守卫的**真正意图**是"别把外层 binder 点亮"（binder 的 span 覆盖
        // 整段类型标注 `(h : a ∈ A)`，回退会把 `∈` 解析成 `h`）——不是"符号上
        // 什么都不给"。所以这里换成**词法**答案：该符号的每个 token。
        let ranges: Vec<Range> = sokonanoda_front::notation_input::symbol_occurrences(doc, &symbol)
            .into_iter()
            .map(range_of)
            .collect();
        return if ranges.is_empty() {
            None
        } else {
            Some(ranges)
        };
    }
    let from_use = hover_type_at(hovers, line, character).and_then(|h| h.resolution.as_ref());
    let def_span = match from_use {
        Some(target) => Some(target.span()),
        None => hovers.iter().find_map(|h| {
            let span = h.resolution.as_ref()?.span();
            pos_within_span(line, character, span).then_some(span)
        }),
    }?;
    let uses: Vec<Range> = hovers
        .iter()
        .filter(|h| {
            h.resolution
                .as_ref()
                .is_some_and(|target| target.span() == def_span)
        })
        .map(|h| range_of(h.span))
        .collect();
    (!uses.is_empty()).then_some(uses)
}

/// **G-55**：光标在**记法声明的目标名**上时，"同一个定义"包含的全部位置。
///
/// 语义定案（用户 2026-10-07 授权；设计 `docs/design/notation-subset.md` §15）：
/// 目标名是**引用**（它指向一个已经存在的声明），不是引入处 ⇒「同一个定义」=
/// 这个名字本身 **+ 这个文件里所有指到同一个定义的位置**，三层：
///
/// * **① 光标处这个名字**（`name_span`）—— **地板**：一定在结果里 ⇒ 永不 `null`、
///   永远包含"用户实际点的那个位置"。目标在闭包外（G-54 不做）时，这就是全部答案。
/// * **② 本文件里展开到这个目标的每个记法符号的每一处** —— 用与 T-D24（光标在
///   符号上）**同一份词法答案**（`symbol_occurrences`）：`𝒫 A` 与 `Set.powerset α A`
///   指的是同一个定义，符号只是**写法**不同。声明行字符串里的那个符号不在其中
///   （它是 `Str` token，不是使用处）。
/// * **③ 目标在本文件里可解析时**：它的**定义名**那一处 + 所有**点名使用处**
///   （`hovers` 里解析到同一声明的位置）。`definition` 是定义**命令**的 span，
///   由调用方用 `project_definition` 解出并**限定在本文件内** ——
///   `documentHighlight` 的 range 只属于被请求的那份文档，跨文件的位置不进结果
///   （跨文件是 `definition`/`references` 的事）。
///
/// 结果**去重 + 按位置排序**（同一处可能同时是"符号的每一处"与"点名使用处"）。
pub(crate) fn notation_target_highlight(
    doc: &str,
    hovers: &[HoverType],
    name: &str,
    name_span: Span,
    definition: Option<Span>,
) -> Vec<Range> {
    let mut ranges = vec![range_of_in(doc, name_span)];
    // ② 这个目标在这个文件里的**每一种写法**（符号 → 目标的对照表是词法扫描的产物）。
    for (symbol, target) in sokonanoda_front::notation_input::declared_notations(doc) {
        if target.as_deref() == Some(name) {
            ranges.extend(
                sokonanoda_front::notation_input::symbol_occurrences(doc, &symbol)
                    .into_iter()
                    .map(|s| range_of_in(doc, s)),
            );
        }
    }
    // ③ 定义那一处 + 点名使用处（只有定义**就在本文件里**时才有意义）。
    if let Some(def_span) = definition {
        if let Some(def_name) = decl_name_span(doc, def_span, name) {
            ranges.push(range_of_in(doc, def_name));
        }
        ranges.extend(
            hovers
                .iter()
                .filter(|h| {
                    h.resolution
                        .as_ref()
                        .is_some_and(|target| target.span() == def_span)
                })
                .map(|h| range_of_in(doc, h.span)),
        );
    }
    ranges.sort_by_key(|r| (r.start.line, r.start.character, r.end.line, r.end.character));
    ranges.dedup();
    ranges
}

/// The in-scope binder names (outermost first) at the cursor, from the
/// smallest enclosing hover row. Anonymous binders carry empty names.
/// 光标处的语义 token 类别（front::semantic 的 token 流为准）。
pub(crate) fn semantic_kind_at(text: &str, line: u32, character: u32) -> Option<SemanticKind> {
    let spans = sokonanoda_front::semantic::semantic_tokens(text);
    let offset = line_col_to_offset(text, line, character);
    spans
        .iter()
        .find(|s| {
            s.span.start.offset <= offset && offset < s.span.end.offset.max(s.span.start.offset + 1)
        })
        .map(|s| s.kind)
}

fn line_col_to_offset(text: &str, line: u32, character: u32) -> usize {
    let mut offset = 0usize;
    for (i, l) in text.lines().enumerate() {
        if i == line as usize {
            // **G-36 第二处（2026-10-03 ✓）**：与 `lib.rs::position_to_offset` **同一个 bug** ✗ ——
            // LSP 的 `character` 是 **UTF-16 码元** ✓，旧实现按 `char` 数 ✗ ⇒ `𝒫`/emoji 之后偏位 ✓。
            // 这条是 `bracket_hover` 用的 ✓（复现件里那个"整行表达式"的 hover 正是括号 hover ✓）。
            let mut units = 0usize;
            for (o, ch) in l.char_indices() {
                if units >= character as usize {
                    return offset + o;
                }
                units += ch.len_utf16();
            }
            return offset + l.len();
        }
        offset += l.len() + 1;
    }
    text.len()
}

/// 光标处那个 **hover 行**里的在域名字（补全的"最内层域"支路）。
///
/// ⚠ 它**只覆盖光标落在已存在 token 里的情形**（没有 span 命中就 `None`）——
/// 行末/声明尾巴上的兜底在 `QueryDoc::scope_binders_at`（C1）。两条在 LSP 侧
/// 并集去重，见 `Backend::completion`。
pub(crate) fn scope_names_at(hovers: &[HoverType], line: u32, character: u32) -> Option<&[String]> {
    hover_type_at(hovers, line, character).map(|h| h.scope_names.as_slice())
}

pub(crate) fn symbol_kind(kind: DeclKind) -> SymbolKind {
    match kind {
        DeclKind::Definition => SymbolKind::FUNCTION,
        DeclKind::Theorem => SymbolKind::KEY,
        DeclKind::Axiom => SymbolKind::PROPERTY,
        DeclKind::Inductive => SymbolKind::STRUCT,
        DeclKind::Example => SymbolKind::CONSTANT,
    }
}

pub(crate) fn status_label(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Open => "exercise: open",
        DeclStatus::Checked => "solved ✓",
        DeclStatus::Failed => "failed",
    }
}

// ---- rename / references（docs/design/rename-inlay.md §2–§3）----

/// The definition's name token span for a resolved target: a binder's own
/// first identifier, or the declaration's defining name token. Token-precise
/// (front lexical helpers), never a text scan of the identifier spelling.
fn definition_name_span(text: &str, target: &ResolvedTarget) -> Option<Span> {
    match target {
        ResolvedTarget::Binder(binder) => binder_name_span(text, *binder),
        ResolvedTarget::Declaration { name, span } => decl_name_span(text, *span, name),
        // **T-D15**：记法符号**不是名字**，没有可改名的 token ⇒ `None`
        // （`rename` 因此被拒；T-D30 那条"记法上 rename 不得改 `h`"现在由
        // **变体本身**保证，不再只靠"光标在符号上"的守卫）。
        ResolvedTarget::Notation { .. } => None,
    }
}

/// `textDocument/prepareRename`：光标处可改名目标的名字子 span + 原名。
/// 解析不到（prelude 名、匿名 example、声明外）→ `None`。
pub(crate) fn prepare_rename(
    text: &str,
    report: &DocumentReport,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let target = resolve_at(text, &report.hovers, position.line, position.character)?;
    let name_span = definition_name_span(text, &target)?;
    let placeholder = text
        .get(name_span.start.offset..name_span.end.offset)?
        .to_string();
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: range_of(name_span),
        placeholder,
    })
}

/// `textDocument/rename`：语义改写（use→def 反向分组 + 定义名 token）。
/// 位置不可解析 / 新名非法 → `Err`（ResponseError，绝不返回空 edit）。
pub(crate) fn rename(
    text: &str,
    version: i32,
    report: &DocumentReport,
    params: RenameParams,
) -> tower_lsp::jsonrpc::Result<Option<WorkspaceEdit>> {
    ensure_valid_new_name(&params.new_name)?;
    let position = params.text_document_position.position;
    let Some(target) = resolve_at(text, &report.hovers, position.line, position.character) else {
        return Err(tower_lsp::jsonrpc::Error::invalid_params(
            "这里没有可以改名的名字",
        ));
    };
    // The definition's name token plus every use point, from the semantic
    // use→def map — comments/strings with the same spelling stay untouched.
    let mut spans: Vec<Span> = Vec::new();
    if let Some(name) = definition_name_span(text, &target) {
        spans.push(name);
    }
    spans.extend(references_for(&report.hovers, &target));
    spans.sort_by_key(|s| s.start.offset);
    spans.dedup_by_key(|s| s.start.offset);
    let edits: Vec<_> = spans
        .into_iter()
        .map(|span| {
            OneOf::Left(TextEdit {
                range: range_of(span),
                new_text: params.new_name.clone(),
            })
        })
        .collect();
    if edits.is_empty() {
        // Defensive: a resolved target always contributes a definition site.
        return Ok(None);
    }
    Ok(Some(WorkspaceEdit {
        document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier::new(
                params.text_document_position.text_document.uri,
                version,
            ),
            edits,
        }])),
        ..Default::default()
    }))
}

/// `textDocument/references`：目标全部使用点（`include_declaration` 时
/// 定义名 Location 插在最前；按 offset 排序去重）。
pub(crate) fn find_references(
    uri: Url,
    text: &str,
    report: &DocumentReport,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let target = resolve_at(text, &report.hovers, position.line, position.character)?;
    let mut spans: Vec<Span> = Vec::new();
    if include_declaration {
        spans.extend(definition_name_span(text, &target));
    }
    spans.extend(references_for(&report.hovers, &target));
    spans.sort_by_key(|s| s.start.offset);
    spans.dedup_by_key(|s| s.start.offset);
    Some(
        spans
            .into_iter()
            .map(|span| Location {
                uri: uri.clone(),
                range: range_of(span),
            })
            .collect(),
    )
}

/// 新名字的合法性（单文件与跨文件改名共用同一条规则与同一句报错）。
pub(crate) fn ensure_valid_new_name(name: &str) -> tower_lsp::jsonrpc::Result<()> {
    if is_valid_new_name(name) {
        Ok(())
    } else {
        Err(tower_lsp::jsonrpc::Error::invalid_params(format!(
            "「{name}」不是合法标识符：只能包含字母/数字/下划线/非 ASCII 字符，且不能以数字开头"
        )))
    }
}

/// A rename target must lex as exactly one identifier token spanning the
/// whole string (`9x` lexes as number + ident, spaces split tokens — both
/// rejected without touching the kernel).
fn is_valid_new_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let Ok(tokens) = tokenize(name) else {
        return false;
    };
    let mut meaningful = tokens.iter().filter(|t| t.kind != TokenKind::Eof);
    let Some(first) = meaningful.next() else {
        return false;
    };
    if meaningful.next().is_some() {
        return false;
    }
    matches!(&first.kind, TokenKind::Ident(text) if text == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{
        call, did_open, handshake, lsp_pos, offset_of, position_json, test_service,
        wait_diagnostics, URI,
    };
    use crate::Backend;
    use serde_json::json;
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::{ClientSocket, LspService};

    const REFS: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n";
    // def-name `id` at (0,4)..(0,6); the `#check id` use at (1,7)..(1,9).

    const NO_USE: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";
    // No use points anywhere: prelude names stay unresolved.

    const DOUBLE: &str = "def double : Nat -> Nat := fun (n : Nat) => n + n\n";
    // binder name `n` at (0,32); body uses at (0,44) and (0,48).

    const SHADOW: &str = "def f : Prop -> Prop -> Prop := fun (x : Prop) => fun (x : Prop) => x\n";
    // outer binder name at (0,37); inner binder name at (0,55), its body use
    // at (0,68).

    async fn setup(src: &str) -> (LspService<Backend>, ClientSocket) {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "setup diagnostics").await;
        (service, socket)
    }

    async fn references_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let result = call(
            service,
            RpcRequest::build("textDocument/references")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                    "context": {"includeDeclaration": include_declaration},
                }))
                .id(100)
                .finish(),
        )
        .await
        .expect("textDocument/references must answer");
        serde_json::from_value(result).expect("valid references response")
    }

    async fn prepare_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<PrepareRenameResponse> {
        let result = call(
            service,
            RpcRequest::build("textDocument/prepareRename")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": position_json(lsp_pos(src, offset)),
                }))
                .id(101)
                .finish(),
        )
        .await
        .expect("textDocument/prepareRename must answer");
        serde_json::from_value(result).expect("valid prepareRename response")
    }

    fn rename_request(src: &str, offset: usize, new_name: &str) -> RpcRequest {
        RpcRequest::build("textDocument/rename")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(lsp_pos(src, offset)),
                "newName": new_name,
            }))
            .id(102)
            .finish()
    }

    async fn rename_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let result = call(service, rename_request(src, offset, new_name))
            .await
            .expect("textDocument/rename must answer");
        serde_json::from_value(result).expect("valid rename response")
    }

    /// Like `call`, but returns the error instead of panicking — needed to
    /// assert ResponseError behaviour (invalid rename params).
    async fn call_raw(
        service: &mut LspService<Backend>,
        req: RpcRequest,
    ) -> Option<Result<serde_json::Value, tower_lsp::jsonrpc::Error>> {
        let resp = service
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("service call succeeded");
        resp.map(|resp| match resp.into_parts() {
            (_, Ok(result)) => Ok(result),
            (_, Err(err)) => Err(err),
        })
    }

    /// The single document's edits of a rename answer, version included.
    fn document_edits(edit: WorkspaceEdit) -> (i32, Vec<(Range, String)>) {
        let Some(DocumentChanges::Edits(docs)) = edit.document_changes else {
            panic!(
                "expected documentChanges edits, got {:?}",
                edit.document_changes
            );
        };
        assert_eq!(docs.len(), 1, "one document touched: {docs:?}");
        assert_eq!(docs[0].text_document.uri.as_str(), URI);
        let edits = docs[0]
            .edits
            .iter()
            .map(|edit| match edit {
                OneOf::Left(edit) => (edit.range, edit.new_text.clone()),
                OneOf::Right(other) => panic!("no annotations expected, got {other:?}"),
            })
            .collect();
        let version = match &docs[0].text_document {
            OptionalVersionedTextDocumentIdentifier {
                version: Some(v), ..
            } => *v,
            other => panic!("renamed document must be versioned, got {other:?}"),
        };
        (version, edits)
    }

    // ---- references ----

    #[tokio::test]
    async fn references_from_a_use_point_list_the_use() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let use_at = offset_of(src, "#check id") + "#check ".len();
        let refs = references_at(&mut service, src, use_at + 1, false)
            .await
            .expect("a resolved use point answers with its references");
        assert_eq!(refs.len(), 1, "no declaration requested: {refs:?}");
        assert_eq!(refs[0].range.start, lsp_pos(src, use_at));
        assert_eq!(refs[0].range.end, lsp_pos(src, use_at + 2));
    }

    #[tokio::test]
    async fn references_include_declaration_puts_the_definition_first() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let use_at = offset_of(src, "#check id") + "#check ".len();
        let refs = references_at(&mut service, src, use_at + 1, true)
            .await
            .expect("references with the declaration");
        assert_eq!(refs.len(), 2, "definition name + use: {refs:?}");
        assert_eq!(refs[0].range.start, lsp_pos(src, 4), "def-name first");
        assert_eq!(refs[0].range.end, lsp_pos(src, 6));
        assert_eq!(refs[1].range.start, lsp_pos(src, use_at), "use second");
    }

    #[tokio::test]
    async fn references_from_the_definition_site_match_the_use_site() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let use_at = offset_of(src, "#check id") + "#check ".len();
        // Cursor on the def name: same bidirectional answer.
        let refs = references_at(&mut service, src, offset_of(src, "id") + 1, true)
            .await
            .expect("the declaration name resolves as a definition point");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].range.start, lsp_pos(src, 4));
        assert_eq!(refs[1].range.start, lsp_pos(src, use_at));
    }

    #[tokio::test]
    async fn references_without_a_target_return_none() {
        let (mut service, _socket) = setup(NO_USE).await;
        let refs = references_at(&mut service, NO_USE, offset_of(NO_USE, "Prop") + 1, true).await;
        assert!(refs.is_none(), "prelude names have no references: {refs:?}");
    }

    // ---- prepareRename ----

    #[tokio::test]
    async fn prepare_rename_on_a_binder_covers_the_name_token() {
        let src = DOUBLE;
        let (mut service, _socket) = setup(src).await;
        // Cursor inside the binder name `n` (0,32)..(0,33).
        let resp = prepare_at(&mut service, src, 32)
            .await
            .expect("the binder is renameable");
        let PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } = resp else {
            panic!("expected range with placeholder, got {resp:?}");
        };
        assert_eq!(range.start, lsp_pos(src, 32));
        assert_eq!(range.end, lsp_pos(src, 33));
        assert_eq!(placeholder, "n");
    }

    #[tokio::test]
    async fn prepare_rename_on_a_declaration_covers_the_name_token() {
        let src = REFS;
        let (mut service, _socket) = setup(src).await;
        let resp = prepare_at(&mut service, src, offset_of(src, "id") + 1)
            .await
            .expect("the declaration name is renameable");
        let PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } = resp else {
            panic!("expected range with placeholder, got {resp:?}");
        };
        assert_eq!(range.start, lsp_pos(src, 4));
        assert_eq!(range.end, lsp_pos(src, 6));
        assert_eq!(placeholder, "id");
    }

    // ---- G-02 / WO-005：构造子的规范名与裸名两种写法 ----

    // 两种写法各用一次：`Wrap.mk`（R1 规范名）与裸 `mk`（R2 别名）。
    const CTOR_DOC: &str = "inductive Wrap : Type\nctor mk : Wrap\nend\n\
def viaPrefix : Wrap := Wrap.mk\ndef viaAlias : Wrap := mk\n";
    // ctor 源 token `mk` at (1,5); uses at (3,21) and (4,20).

    fn ctor_use_offset(src: &str, needle: &str) -> usize {
        // 最后一个出现点（定义 token 在前，使用点在后）。
        src.rfind(needle)
            .unwrap_or_else(|| panic!("`{needle}` not found in `{src}`"))
    }

    #[tokio::test]
    async fn prepare_rename_from_a_use_covers_the_ctors_source_token() {
        // 解析目标是**规范名** `Wrap.mk`，源里写的却是裸名 `mk`：
        // `decl_name_span` 必须回退到源 token，否则 ctor 静默不可改名。
        let src = CTOR_DOC;
        let (mut service, _socket) = setup(src).await;
        let offset = ctor_use_offset(src, "Wrap.mk");
        let resp = prepare_at(&mut service, src, offset)
            .await
            .expect("the constructor use is renameable");
        let PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } = resp else {
            panic!("expected range with placeholder, got {resp:?}");
        };
        assert_eq!(placeholder, "mk", "the source spelling is the placeholder");
        let def = offset_of(src, "ctor mk") + 5;
        assert_eq!(range.start, lsp_pos(src, def));
        assert_eq!(range.end, lsp_pos(src, def + 2));
    }

    #[tokio::test]
    async fn rename_a_ctor_rewrites_the_source_token_and_both_uses() {
        let src = CTOR_DOC;
        let (mut service, _socket) = setup(src).await;
        let edit = rename_at(&mut service, src, ctor_use_offset(src, "Wrap.mk"), "pair")
            .await
            .expect("rename answers");
        let DocumentChanges::Edits(edits) = edit.document_changes.expect("edits") else {
            panic!("expected text edits");
        };
        let ranges: Vec<Range> = edits
            .into_iter()
            .flat_map(|e| {
                e.edits.into_iter().filter_map(|x| match x {
                    OneOf::Left(edit) => Some(edit.range),
                    OneOf::Right(_) => None,
                })
            })
            .collect();
        // 定义侧两条（ctor 命令 span + 定义名 token —— 既有口径，实测基线同形）
        // + 两种写法的使用点各一处。
        assert_eq!(
            ranges.len(),
            4,
            "ctor command span + name token + two uses: {ranges:?}"
        );
    }

    #[tokio::test]
    async fn references_resolve_a_ctor_from_both_spellings() {
        let src = CTOR_DOC;
        let (mut service, _socket) = setup(src).await;
        let canonical = references_at(&mut service, src, ctor_use_offset(src, "Wrap.mk"), true)
            .await
            .expect("references answer");
        let bare = references_at(&mut service, src, ctor_use_offset(src, ":= mk") + 3, true)
            .await
            .expect("references answer");
        // 定义点：ctor 的**命令 span**（`ctor mk : Wrap`）与名字 token 各一条
        // （`include_declaration` 的既有口径，与 def 一致）+ 两个使用点。
        assert_eq!(
            canonical.len(),
            4,
            "ctor command span + name token + two uses: {canonical:?}"
        );
        assert_eq!(
            canonical, bare,
            "both spellings resolve to the same declaration"
        );
    }

    #[tokio::test]
    async fn prepare_rename_is_null_for_prelude_names() {
        let (mut service, _socket) = setup(NO_USE).await;
        let resp = prepare_at(&mut service, NO_USE, offset_of(NO_USE, "Prop") + 1).await;
        assert!(resp.is_none(), "prelude names cannot be renamed: {resp:?}");
    }

    // ---- rename ----

    #[tokio::test]
    async fn rename_rewrites_binder_and_all_uses_with_version() {
        let src = DOUBLE;
        let (mut service, _socket) = setup(src).await;
        // Cursor on the first body use.
        let edit = rename_at(&mut service, src, 44, "k")
            .await
            .expect("a valid rename answers with an edit");
        let (version, edits) = document_edits(edit);
        assert_eq!(version, 1, "the didOpen version must be carried");
        let offsets = [32, 44, 48]; // binder name + both uses
        assert_eq!(edits.len(), offsets.len(), "edits: {edits:?}");
        for ((range, new_text), offset) in edits.iter().zip(offsets) {
            assert_eq!(range.start, lsp_pos(src, offset), "edit at {offset}");
            assert_eq!(range.end, lsp_pos(src, offset + 1));
            assert_eq!(new_text, "k");
        }
    }

    #[tokio::test]
    async fn rename_respects_shadowing_inner_binder_only() {
        let src = SHADOW;
        let (mut service, _socket) = setup(src).await;
        // Cursor on the inner body use; the inner binder shadows the outer.
        let edit = rename_at(&mut service, src, 68, "y")
            .await
            .expect("the inner binder is renameable");
        let (_, edits) = document_edits(edit);
        let offsets = [55, 68]; // inner binder name + its use
        assert_eq!(edits.len(), offsets.len(), "edits: {edits:?}");
        for ((range, new_text), offset) in edits.iter().zip(offsets) {
            assert_eq!(range.start, lsp_pos(src, offset));
            assert_eq!(new_text, "y");
        }
        assert!(
            edits
                .iter()
                .all(|(range, _)| range.start != lsp_pos(src, 37)),
            "the outer binder name (0,37) must stay untouched: {edits:?}"
        );
    }

    #[tokio::test]
    async fn rename_does_not_touch_comments() {
        let src = "def double : Nat -> Nat := fun (n : Nat) => n + n -- n 是个数\n";
        let (mut service, _socket) = setup(src).await;
        let edit = rename_at(&mut service, src, 44, "k")
            .await
            .expect("a valid rename answers with an edit");
        let (_, edits) = document_edits(edit);
        assert_eq!(
            edits.len(),
            3,
            "binder name + two uses, never the commented `n`: {edits:?}"
        );
    }

    #[tokio::test]
    async fn rename_rejects_invalid_new_names() {
        let src = DOUBLE;
        let (mut service, _socket) = setup(src).await;
        for bad in ["9x", "x y", ""] {
            let response = call_raw(&mut service, rename_request(src, 44, bad)).await;
            match response {
                Some(Err(err)) => assert!(
                    err.message.contains("不是合法标识符"),
                    "teaching message expected, got {err:?}"
                ),
                other => panic!("`{bad}` must be rejected, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn rename_without_a_target_is_an_error() {
        let (mut service, _socket) = setup(NO_USE).await;
        let response = call_raw(
            &mut service,
            rename_request(NO_USE, offset_of(NO_USE, "Prop") + 1, "k"),
        )
        .await;
        match response {
            Some(Err(err)) => {
                assert!(
                    err.message.contains("这里没有可以改名的名字"),
                    "teaching message expected, got {err:?}"
                );
            }
            other => panic!("an unresolvable position must be an error: {other:?}"),
        }
    }

    /// **H1**：同一段源码被 elaborate 多次时，取**这一段源码自己那一行** ——
    /// 判据是 `resolution = Notation{span 落在本行内}`（front 算好的真相），
    /// **不是**"取第一条"（那会拿到目标常量自己的签名）也**不是**"取最后一条"
    /// （by 引擎会在同一个 span 上再记一行"证明项的类型"，实测
    /// `theorem t (p : Prop) : p ↔ p := by rfl` 上末块会变成 `p ↔ p` ✗）。
    /// 端到端读数：`node scripts/ux-probes/lsp-probe.mjs --case H1-notation-hover-type`
    /// 与 `--case H4-notation-hover-slice`（后者的末块类型在同一处翻车过）。
    #[test]
    fn innermost_prefers_the_row_owning_the_span() {
        let span = Span {
            start: Pos {
                offset: 0,
                line: 1,
                column: 1,
            },
            end: Pos {
                offset: 5,
                line: 1,
                column: 6,
            },
        };
        let row = |text: &str, symbol: Option<&str>| HoverType {
            span,
            text: text.to_string(),
            scope_names: Vec::new(),
            resolution: symbol.map(|symbol| ResolvedTarget::Notation {
                symbol: symbol.to_string(),
                span: Span {
                    start: Pos {
                        offset: 1,
                        line: 1,
                        column: 2,
                    },
                    end: Pos {
                        offset: 2,
                        line: 1,
                        column: 3,
                    },
                },
                module: None,
            }),
            binder: false,
            lexical: false,
        };
        let hovers = vec![
            row("forall {a : Type}, a -> a -> Prop", None), // 目标常量自己的签名（先记的）
            row("Prop", None),
            row("Prop", Some("=")), // 记法节点自己那一行 ✓
            row("a = a", None),     // by 引擎在同一 span 上追加的一行（最后记的）
        ];
        let picked = hover_type_at(&hovers, 0, 1).expect("a row covers the cursor");
        assert_eq!(
            picked.text, "Prop",
            "取'这一段源码自己那一行'（H1），不是第一条、也不是最后一条"
        );
        // 没有"自己那一行"时退回旧行为（第一条）—— 改动面最小。
        let without_owner = vec![row("first", None), row("second", None)];
        assert_eq!(
            hover_type_at(&without_owner, 0, 1).expect("row").text,
            "first",
            "没有 owning 行时退回既定行为（第一条）"
        );
        // 宽度不同则**窄的赢**（既有语义不许被次序改掉）。
        let mut narrow = hovers.clone();
        narrow.push(HoverType {
            span: Span {
                start: Pos {
                    offset: 1,
                    line: 1,
                    column: 2,
                },
                end: Pos {
                    offset: 2,
                    line: 1,
                    column: 3,
                },
            },
            text: "narrow".to_string(),
            scope_names: Vec::new(),
            resolution: None,
            binder: false,
            lexical: false,
        });
        assert_eq!(
            hover_type_at(&narrow, 0, 1).expect("row").text,
            "narrow",
            "更窄的 span 仍然优先"
        );
    }

    /// **N1**：`..` / `.` 必须被词法消掉 —— 同一个文件只能有一个 `file://` 身份。
    #[test]
    fn file_uri_normalizes_parent_components() {
        let uri = file_uri(std::path::Path::new(
            "/repo/crates/front/../../prelude/P.sokonanoda",
        ))
        .expect("file url");
        assert_eq!(uri.path(), "/repo/prelude/P.sokonanoda", "{uri:?}");
        assert!(
            !uri.as_str().contains("/../"),
            "URI 里不许再出现 `/../`（编辑器会当另一个文件）：{uri:?}"
        );
        // `.` 与重复分隔符同样消掉；无关的路径一个字节不变。
        assert_eq!(
            file_uri(std::path::Path::new("/repo/./a//b.sokonanoda"))
                .expect("file url")
                .path(),
            "/repo/a/b.sokonanoda"
        );
        assert_eq!(
            file_uri(std::path::Path::new("/repo/lib/Set.sokonanoda"))
                .expect("file url")
                .path(),
            "/repo/lib/Set.sokonanoda"
        );
    }

    /// **H4**：记法节点的 span 起点会落在开括号**里面** ⇒ 切片两端都要配平。
    #[test]
    fn balanced_span_both_includes_the_opening_paren() {
        let text = "theorem t : (a \u{2208} A) \u{2194} A a := by\n  sorry\n";
        // 记法节点 `(a ∈ A) ↔ A a` 的 span 实测从 `a` 起（丢掉开括号）。
        let start = text.find("a \u{2208}").expect("operand");
        let end = text.find(" := by").expect("end");
        let span = Span::new(pos_at(text, start), pos_at(text, end));
        let slice = &text[balanced_span_both(text, span).start.offset
            ..balanced_span_both(text, span).end.offset];
        assert_eq!(slice, "(a \u{2208} A) \u{2194} A a", "{slice:?}");
        let open = slice.matches('(').count();
        let close = slice.matches(')').count();
        assert_eq!(open, close, "括号必须配对：{slice:?}");
    }
}

#[cfg(test)]
mod lean4_universe_tests {
    use super::{lean4_universe_runs, lean4_universe_text};
    use crate::protocol::RunInfo;

    fn run(text: &str, kind: Option<&str>) -> RunInfo {
        RunInfo {
            text: text.to_string(),
            kind: kind.map(str::to_string),
            start: None,
            end: None,
        }
    }

    /// **F6 的判据（真相层）**：内核 pp 的 `Type 0` 印成 `Type`（Lean 4 的
    /// `delabSort`），而 `Type 1` / `Type u` / `Sort u` / `Prop` 一个都不动。
    #[test]
    fn type_zero_loses_its_level_and_nothing_else_changes() {
        assert_eq!(
            lean4_universe_text("∀ (α : Type 0) (A : Set α), A ⊆ A"),
            "∀ (α : Type) (A : Set α), A ⊆ A"
        );
        // 更高层 / 变量层 / 非数字层：**逐字节不动**。
        for keep in [
            "∀ (α : Type 1), α → α",
            "∀ (α : Type u), α → α",
            "Sort u → Sort u",
            "Prop → Prop",
            // 名字里带 `Type` 的标识符不是 `Type` token。
            "MyType 0 → MyType 0",
            "Set.Type 0 → Set.Type 0",
        ] {
            assert_eq!(lean4_universe_text(keep), keep, "不许动：{keep}");
        }
    }

    /// **runs 与文本必须一起变**（T-A5）：`ty_runs` 依次拼接 == `ty`，
    /// 且 `start`/`end` 是**字节**偏移、等于拼接位置 —— 改完这条不变量照旧成立
    /// （否则从第一个 `Type 0` 起的高亮全部错位 ✗）。
    #[test]
    fn runs_move_with_the_text_and_stay_tiled() {
        let ty = "∀ (α : Type 0) (A : Type 1), A ⊆ A".to_string();
        // 分段：`Type`（带位置）/ 空格 / `0`（带位置）/ …（与 wire 上的形状同构）。
        let runs: Vec<RunInfo> = vec![
            RunInfo {
                text: "∀ (α : ".into(),
                kind: Some("keyword".into()),
                start: Some(0),
                end: Some(6),
            },
            RunInfo {
                text: "Type".into(),
                kind: Some("sort".into()),
                start: Some(6),
                end: Some(10),
            },
            run(" ", None),
            RunInfo {
                text: "0".into(),
                kind: Some("number".into()),
                start: Some(11),
                end: Some(12),
            },
            RunInfo {
                text: ") (A : ".into(),
                kind: None,
                start: Some(12),
                end: Some(19),
            },
            RunInfo {
                text: "Type".into(),
                kind: Some("sort".into()),
                start: Some(19),
                end: Some(23),
            },
            run(" ", None),
            RunInfo {
                text: "1".into(),
                kind: Some("number".into()),
                start: Some(24),
                end: Some(25),
            },
            RunInfo {
                text: "), A ⊆ A".into(),
                kind: None,
                start: Some(25),
                end: Some(33),
            },
        ];
        let (text, out) = lean4_universe_runs(ty, runs);
        assert_eq!(text, "∀ (α : Type) (A : Type 1), A ⊆ A");
        assert_eq!(
            out.iter().map(|r| r.text.as_str()).collect::<String>(),
            text,
            "runs 必须仍然**铺满**文本（拼接 == 文本）"
        );
        let mut pos = 0usize;
        for r in &out {
            if let (Some(s), Some(e)) = (r.start, r.end) {
                assert_eq!((s, e), (pos, pos + r.text.len()), "偏移 = 拼接位置（字节）");
            }
            pos += r.text.len();
        }
        // 被删掉的 `" "` 与 `"0"` 两段**整段消失**，`Type 1` 的层**原样留着**。
        assert!(out.iter().any(|r| r.text == "1"));
        assert!(!out.iter().any(|r| r.text == "0"));
    }

    /// 没有 `Type 0` 时**逐字节不变**（含 runs 原样返回）。
    #[test]
    fn nothing_changes_without_type_zero() {
        let ty = "∀ (α : Type), α → α".to_string();
        let runs = vec![run("∀ (α : Type), α → α", None)];
        let (text, out) = lean4_universe_runs(ty.clone(), runs);
        assert_eq!(text, ty);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, ty);
    }
}
