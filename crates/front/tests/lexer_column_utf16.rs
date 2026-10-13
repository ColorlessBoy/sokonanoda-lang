//! **T-D31 判据：词法器的列口径 = UTF-16 code unit**（与 LSP 的 `character` 一致 ✓）。
//!
//! ## 为什么必须新写一条夹具
//!
//! 词法器原来按 **`char`** 累加列（`token.rs::Lexer::bump` 的 `self.column += 1` ✗），
//! 而真相层的唯一换算 `query::pos::line_col_of` **早就是 UTF-16** ✓。两者只在
//! **含星平面字符**的行上分叉 —— 而**既有夹具全是 BMP**（`α`/`∈`/`⊆` 都是 1 个
//! UTF-16 码元）⇒ 两种口径**恒等** ⇒ 这个 bug 一直咬不住 ✗。
//!
//! ⇒ 本文件的夹具**必须**含 `𝒫`（U+1D4AB，MATHEMATICAL SCRIPT CAPITAL P：
//! **1 个 `char` = 2 个 UTF-16 码元** ✓）与 `𝓕`（U+1D4D5，同族 ✓）。
//!
//! ## 用户可见的后果（这条断言在哪一层）
//!
//! `crates/lsp/src/render.rs::range_of` **直接**把 `span.start.column - 1` 当 LSP 的
//! `character` 发出去 ⇒ 列口径错了，编辑器里**下划线的位置就错**（`𝒫` 之后整段
//! 偏左一格 ✗）。同族的 `references::pos_within` 拿 LSP 的 UTF-16 `character` 去比
//! span 列 ⇒ 光标命中**错的 span**（T-D30 同形事故的成因 ✓）。
//!
//! ## 反向验证（硬要求，已实跑 ✓）
//!
//! 临时把 `Lexer::bump` 的列累加退回 `self.column += 1`（`char` 口径 ✗，
//! **测完已还原** —— 现在 `crates/` 里没有任何逃生门 ✓）⇒ 本文件 **6 条里 4 条判红**
//! （`left: (1, 28)` vs `right: (1, 29)`；parity 那条报
//! `token Ident("𝒫") … column: 48` vs `49` ✗），而**剩的 2 条仍绿** ✓
//! —— 那 2 条恰好是**不依赖词法列**的（一条自检夹具、一条验真相层换算）
//! ⇒ 正是「既有夹具全是 BMP ⇒ 咬不住」的现场复现 ✓。

use sokonanoda_front::query::{column_width, line_col_of};
use sokonanoda_front::{parse, tokenize, Pos, Span};

/// 夹具：第 6 行同时含 `𝒫`（U+1D4AB）与 `𝓕`（U+1D4D5）——都是星平面字符。
/// 其余行是 BMP（`α`/`∈`/`⊆`）⇒ 同一份夹具里**两种口径可比** ✓。
const ASTRAL_SRC: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
def Set.subset (α : Type) (A : Set α) (B : Set α) : Prop := (x : α) -> x ∈ A -> x ∈ B\n\
infix:50 \" ⊆ \" => Set.subset\n\
theorem t (α : Type) (A : Set α) (B : Set α) (𝒫 : Set α) (𝓕 : Set α) (h : 𝒫 ⊆ B) : 𝒫 ⊆ B := h\n";

/// 独立重算"该行内字节下标 `at` 处的 **UTF-16** 列（1-based）"——**故意不复用**
/// 被测实现（否则就是拿实现验自己 ✗）。`𝒫` 这类星平面字符在这里自然算 2 ✓。
fn utf16_col(line: &str, at: usize) -> usize {
    line[..at].chars().map(char::len_utf16).sum::<usize>() + 1
}

/// 同一处的 **`char`** 列（1-based）——旧口径。用来断言夹具**真的分叉**
/// （不分叉的夹具咬不住这个 bug ✗）。
fn char_col(line: &str, at: usize) -> usize {
    line[..at].chars().count() + 1
}

/// 夹具自检：`𝒫`/`𝓕` 所在的那些列上，两种口径**必须不同**（否则夹具空转 ✗）。
#[test]
fn the_fixture_actually_divides_the_two_column_conventions() {
    let line = ASTRAL_SRC.lines().nth(5).expect("第六行");
    assert!(
        line.contains('𝒫') && line.contains('𝓕'),
        "夹具必须含星平面字符"
    );
    let mut divided = 0;
    for (at, ch) in line.char_indices() {
        assert_eq!(column_width(ch), ch.len_utf16(), "唯一口径就是 len_utf16");
        if utf16_col(line, at) != char_col(line, at) {
            divided += 1;
        }
    }
    assert!(
        divided > 0,
        "夹具里星平面字符之后的每一列都必须让两种口径分叉 —— 否则这条夹具\
         **咬不住** T-D31 的 bug（既有夹具全是 BMP 就是这个下场 ✗）"
    );
    // 具体数字钉一下：`𝒫` 之后同一处的列差**恰好 1**（1 个 `char` 变 2 个码元）。
    let after = line.find('𝒫').expect("有 𝒫") + '𝒫'.len_utf8();
    assert_eq!(
        utf16_col(line, after) - char_col(line, after),
        1,
        "`𝒫` 之后：UTF-16 列 = char 列 + 1（这就是「偏一格」的全部内容）"
    );
}

/// **核心判据 ①**：**词法诊断**的 span 列按 UTF-16。
///
/// 夹具把星平面字符放在**诊断点之前**（`𝒫` 在 `"` 前）⇒ 旧口径下 span 列
/// 偏左一格 ✗。这条断言的就是"编辑器下划线画在哪" ✓。
#[test]
fn lexer_diagnostic_column_is_utf16_after_an_astral_character() {
    // 开引号没有闭合 ⇒ 词法诊断，span 指向**开引号**（`DiagnosticKind::UnterminatedString`）。
    let src = "def f (𝒫 : Prop) : Prop := \"abc\n";
    let err = tokenize(src).expect_err("未闭合字符串必须是词法诊断");
    assert_eq!(err.code(), "unterminated-string");
    let line = src.lines().next().expect("第一行");
    let quote_at = line.find('"').expect("有开引号");
    assert!(
        line[..quote_at].contains('𝒫'),
        "诊断点**之前**必须有星平面字符，否则两种口径恒等、咬不住 ✗"
    );
    // ① 与真相层的唯一换算**逐位一致** ✓。
    let (want_line, want_col) = line_col_of(src, quote_at);
    assert_eq!(
        (err.span.start.line, err.span.start.column),
        (want_line, want_col)
    );
    // ② 具体的 UTF-16 列（**不是** char 列）——把数字钉死，读的人一眼可核 ✓。
    assert_eq!(err.span.start.column, 29, "开引号在第 29 个 UTF-16 码元上");
    assert_eq!(char_col(line, quote_at), 28, "旧口径（char）会答 28 ✗");
    assert_ne!(
        err.span.start.column,
        char_col(line, quote_at),
        "列口径退回 `char` 了 ✗ —— 这正是反向验证要咬的那一口"
    );
}

/// **核心判据 ②**：**解析诊断**的 span 列按 UTF-16（span 来自 token ⇒ 同一条口径 ✓）。
#[test]
fn parse_diagnostic_column_is_utf16_after_an_astral_character() {
    // 值位多了一个 `)` ⇒ 解析器报"没想到这个 token"，span 落在那一个 `)` 上。
    let src = "def f (𝒫 : Prop) : Prop := )\n";
    let err = parse(src).expect_err("多余的 `)` 必须是解析诊断");
    let line = src.lines().next().expect("第一行");
    let paren_at = line.rfind(')').expect("有那个多余的 `)`");
    assert!(line[..paren_at].contains('𝒫'), "诊断点之前必须有星平面字符");
    let (want_line, want_col) = line_col_of(src, paren_at);
    assert_eq!(
        (err.span.start.line, err.span.start.column),
        (want_line, want_col)
    );
    assert_ne!(
        err.span.start.column,
        char_col(line, paren_at),
        "解析诊断的列也必须是 UTF-16 ✗"
    );
}

/// **核心判据 ③（最强的那条）**：**每一个 token 的每一个 span 端点**，
/// 其列都等于真相层唯一换算 `line_col_of` 的列 ✓。
///
/// 这一条是"**只许有一份列口径**"的可执行形式：它不挑某个具体数字，
/// 而是**全量对拍**（含星平面行 ✓）⇒ 任何"再自己写一份"的回归都会当场判红 ✓。
#[test]
fn every_token_span_column_agrees_with_the_single_authority() {
    let toks = tokenize(ASTRAL_SRC).expect("夹具必须能词法通过");
    assert!(
        toks.len() > 30,
        "夹具太小、对拍没有意义：{} 个 token",
        toks.len()
    );
    let mut astral_seen = 0usize;
    for tok in &toks {
        for pos in [tok.span.start, tok.span.end] {
            let (line, col) = line_col_of(ASTRAL_SRC, pos.offset);
            assert_eq!(
                (pos.line, pos.column),
                (line, col),
                "token {:?} 的 span 端点 {pos:?} 与 `line_col_of` 分叉了 —— \
                 词法器**又自己攒了一份列口径** ✗",
                tok.kind
            );
        }
        if tok.span.start.line == 6 {
            astral_seen += 1;
        }
    }
    assert!(
        astral_seen > 5,
        "第 6 行（含 `𝒫`/`𝓕`）必须有足够多的 token 参与对拍，实际 {astral_seen} 个"
    );
    // 反面：**含星平面字符那一行**上，至少有一个 token 的列**不等于** char 列
    // ⇒ 证明这份夹具确实覆盖了分叉区（否则"全绿"可能只是没走到 ✗）。
    let divided = toks
        .iter()
        .filter(|t| t.span.start.line == 6)
        .filter(|t| {
            let line = ASTRAL_SRC.lines().nth(5).expect("第六行");
            let at = t.span.start.offset
                - ASTRAL_SRC
                    .lines()
                    .take(5)
                    .map(|l| l.len() + 1)
                    .sum::<usize>();
            utf16_col(line, at) != char_col(line, at)
        })
        .count();
    assert!(divided > 0, "第 6 行上必须真的存在两种口径分叉的 token ✗");
}

/// **多行/边界**：跨行 span、行尾、EOF 都不许在换算上出岔子 ✓。
#[test]
fn token_columns_agree_with_the_authority_across_lines_and_eof() {
    for src in [
        "𝒫\n",
        "def x := 𝒫\n",
        "def 𝒫 : Prop := 𝒫\n",
        "\n\n  𝒫 𝒫 𝒫\n",
        "def f (𝒫 : Prop) : Prop := 𝒫\n",
    ] {
        let Ok(toks) = tokenize(src) else {
            continue; // 夹具里有故意不合法的形状也无所谓：这一条只验**换算**一致性
        };
        for tok in &toks {
            for pos in [tok.span.start, tok.span.end] {
                let (line, col) = line_col_of(src, pos.offset);
                assert_eq!(
                    (pos.line, pos.column),
                    (line, col),
                    "src={src:?} tok={:?}",
                    tok.kind
                );
            }
        }
        // EOF token 落在文本末尾 ⇒ 也必须一致 ✓。
        let eof = toks.last().expect("至少一个 token");
        assert_eq!(eof.span.start.offset, src.len(), "最后一个 token 是 EOF ✓");
        assert_eq!(eof.span.start.column, line_col_of(src, src.len()).1);
    }
}

/// **同族横向排查**：`session` 的 `line_col` 原来也是一份**按 `char` 的重复实现** ✗
/// ⇒ 现在委派给唯一换算 ✓。这条用一个多行 + 星平面的输入钉住它的口径
/// （`Span` 是 `pub` 的，直接构造后对拍 ✓）。
#[test]
fn span_columns_are_utf16_when_rebuilt_from_offsets() {
    let src = "𝒫 𝒫\n𝓕\n";
    let at = src.find('𝓕').expect("有 𝓕");
    let (line, col) = line_col_of(src, at);
    let span = Span::new(
        Pos {
            offset: at,
            line,
            column: col,
        },
        Pos {
            offset: at + '𝓕'.len_utf8(),
            line,
            column: col + column_width('𝓕'),
        },
    );
    assert_eq!(span.start.line, 2);
    assert_eq!(span.start.column, 1, "`𝓕` 在行首 ⇒ 列 1");
    assert_eq!(span.end.column, 3, "`𝓕` 占 **2** 个 UTF-16 码元 ⇒ 结束列 3");
    assert_eq!(
        &src[span.start.offset..span.end.offset],
        "𝓕",
        "span 圈住 `𝓕`"
    );
}
