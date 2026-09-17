//! `textDocument/semanticTokens/full` 的编码层。
//!
//! 语言知识（哪个片段是什么 kind）在 `front::semantic`——唯一分类源；本模块只
//! 做两件事：把 [`SemanticKind`] 查表成 LSP legend 下标，并把 front 的字节
//! offset span 编码成 LSP 的相对 UTF-16 [`SemanticToken`] 序列。

use sokonanoda_front::semantic::{SemanticKind, SemanticSpan};
use tower_lsp::lsp_types::*;

/// LSP legend：front 的 [`SemanticKind`] 全部映射到标准 `SemanticTokenType`。
/// 顺序即 wire 上 `tokenType` 的下标；测试通过常量解析下标，重排是安全的。
pub(crate) fn semantic_token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::KEYWORD,
        SemanticTokenType::TYPE,
        SemanticTokenType::NUMBER,
        SemanticTokenType::MACRO,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::PARAMETER,
    ]
}

pub(crate) fn semantic_token_options() -> SemanticTokensOptions {
    SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: semantic_token_types(),
            token_modifiers: vec![],
        },
        range: None,
        full: Some(SemanticTokensFullOptions::Bool(true)),
    }
}

/// front kind → legend 下标（语言知识在 front，这里只查表）。
pub(crate) fn token_type_index(kind: SemanticKind) -> u32 {
    let ty = match kind {
        SemanticKind::Keyword => SemanticTokenType::KEYWORD,
        SemanticKind::Sort | SemanticKind::InductiveName | SemanticKind::InductiveUse => {
            SemanticTokenType::TYPE
        }
        SemanticKind::Number => SemanticTokenType::NUMBER,
        SemanticKind::Hole => SemanticTokenType::MACRO,
        SemanticKind::DefName
        | SemanticKind::DefUse
        | SemanticKind::TheoremName
        | SemanticKind::TheoremUse => SemanticTokenType::FUNCTION,
        SemanticKind::AxiomName | SemanticKind::AxiomUse => SemanticTokenType::TYPE,
        SemanticKind::UnknownIdent => SemanticTokenType::VARIABLE,
        SemanticKind::CtorName | SemanticKind::CtorUse => SemanticTokenType::ENUM_MEMBER,
        SemanticKind::Binder => SemanticTokenType::PARAMETER,
    };
    semantic_token_types()
        .iter()
        .position(|t| *t == ty)
        .expect("legend covers every SemanticKind") as u32
}

/// 把 front 的（字节 offset 坐标、已排序）span 编码成 LSP 的相对 UTF-16 编码。
///
/// `deltaLine` 相对前一个 token 的行；`deltaStart` 在同一行时相对前一个
/// token 的起点，换行后是该行内的绝对 UTF-16 偏移。所有位置都按 UTF-16
/// code unit 计数（不是字节、也不是 char），首个虚拟“前一个 token”位于
/// (line 0, utf16 0)，因此首 token 无需特判。
pub(crate) fn encode_semantic_tokens(text: &str, spans: &[SemanticSpan]) -> Vec<SemanticToken> {
    let mut spans: Vec<SemanticSpan> = spans.to_vec();
    spans.sort_by_key(|s| s.span.start.offset);

    let mut line_starts = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }

    let mut data = Vec::with_capacity(spans.len());
    let mut prev_line = 0usize;
    let mut prev_start = 0usize;
    for s in spans {
        let (from, to) = (s.span.start.offset, s.span.end.offset);
        if to <= from || to > text.len() {
            continue;
        }
        let line = line_starts.partition_point(|&start| start <= from) - 1;
        let line_start = line_starts[line];
        let start_utf16: usize = text[line_start..from].chars().map(char::len_utf16).sum();
        let length_utf16: usize = text[from..to].chars().map(char::len_utf16).sum();
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            start_utf16 - prev_start
        } else {
            start_utf16
        };
        data.push(SemanticToken {
            delta_line: delta_line as u32,
            delta_start: delta_start as u32,
            length: length_utf16 as u32,
            token_type: token_type_index(s.kind),
            token_modifiers_bitset: 0,
        });
        prev_line = line;
        prev_start = start_utf16;
    }
    data
}
