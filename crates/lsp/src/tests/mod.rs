//! LSP crate 的单元/端到端测试：共享夹具 + 按特性拆分的子模块。
//!
//! 子模块用 `use super::*;` 取到这里的夹具与再导出（下方 `pub(crate) use`）；
//! 测试与夹具逐字来自拆分前的 `tests.rs`（只搬了模块位置）。

use super::*;
pub(crate) use crate::testutil::{
    call, code_of, did_open, handshake, lsp_pos, notify, offset_of, position_json, shutdown,
    test_service, type_step, wait_diagnostics, TypedStep, URI,
};
pub(crate) use crate::tokens::{encode_semantic_tokens, semantic_token_types, token_type_index};
pub(crate) use crate::Backend;
pub(crate) use serde_json::json;
pub(crate) use sokonanoda_front::semantic::SemanticSpan;
pub(crate) use tower_lsp::jsonrpc::Request as RpcRequest;
pub(crate) use tower_lsp::lsp_types::*;
pub(crate) use tower_lsp::{ClientSocket, LspService};

mod goals;
mod hover;
mod hover_brackets;
mod lenses;
mod lifecycle;
mod navigation;
mod perf;
mod perf_course;
mod state;
mod tokens;

const VALID: &str = "def id : Prop -> Prop := fun (x : Prop) => x\n";

const EXERCISE: &str = "example : Prop -> Prop := sorry\n";

const KERNEL_BAD: &str = "def bad : Prop -> Type := fun (x : Prop) => x\n";

const PARSE_BAD: &str = "def broken : Prop :=\n";

const BARE_OK: &str = "-- sokonanoda:prelude none\ndef id : Prop -> Prop := fun (x : Prop) => x\n";

const BARE_NAT: &str = "-- sokonanoda:prelude none\ndef two : Nat := 2\n";

const FULL_NAT: &str = "def two : Nat := 2\n";

/// 把相对 delta 编码还原成绝对 (line, start_utf16, length, token_type)。
/// 解码逻辑独立实现（按 LSP 规范），用来交叉检验编码器。
fn absolutize(tokens: &[SemanticToken]) -> Vec<(u32, u32, u32, SemanticTokenType)> {
    let legend = semantic_token_types();
    let mut out = Vec::new();
    let (mut line, mut start) = (0u32, 0u32);
    for t in tokens {
        line += t.delta_line;
        if t.delta_line == 0 {
            start += t.delta_start;
        } else {
            start = t.delta_start;
        }
        out.push((line, start, t.length, legend[t.token_type as usize].clone()));
    }
    out
}

async fn request_semantic_tokens(service: &mut LspService<Backend>) -> Vec<SemanticToken> {
    let result = call(
        service,
        RpcRequest::build("textDocument/semanticTokens/full")
            .params(json!({"textDocument": {"uri": URI}}))
            .id(30)
            .finish(),
    )
    .await
    .expect("semanticTokens/full must answer");
    let result: Option<SemanticTokensResult> =
        serde_json::from_value(result).expect("valid SemanticTokensResult");
    match result.expect("tokens must be returned") {
        SemanticTokensResult::Tokens(tokens) => tokens.data,
        other => panic!("expected full tokens, got {other:?}"),
    }
}

// ---- I9 goal 视图协议：soko/goals 与 soko/nextHole ----

async fn request_goals(service: &mut LspService<Backend>) -> serde_json::Value {
    call(
        service,
        RpcRequest::build("soko/goals")
            .params(json!({"textDocument": {"uri": URI}, "position": null}))
            .id(40)
            .finish(),
    )
    .await
    .expect("soko/goals must answer")
}

async fn ask_next_hole(
    service: &mut LspService<Backend>,
    pos: Position,
    forward: bool,
) -> serde_json::Value {
    call(
        service,
        RpcRequest::build("soko/nextHole")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
                "forward": forward,
            }))
            .id(41)
            .finish(),
    )
    .await
    .expect("soko/nextHole must answer")
}

// ---- soko/stateAt：光标处 tactic 目标（docs/design/by-tactics.md §6）----

async fn ask_state_at(
    service: &mut LspService<Backend>,
    src: &str,
    offset: usize,
) -> serde_json::Value {
    call(
        service,
        RpcRequest::build("soko/stateAt")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(lsp_pos(src, offset)),
            }))
            .id(43)
            .finish(),
    )
    .await
    .expect("soko/stateAt must answer")
}

const BY_OPEN: &str = "axiom And : Prop -> Prop -> Prop\n\
     theorem open : (a : Prop) -> And a a -> a := by intro a; intro h\n";

fn reconstruct_runs(runs: &serde_json::Value) -> String {
    runs.as_array()
        .expect("runs array")
        .iter()
        .map(|run| run["text"].as_str().expect("run text"))
        .collect()
}

fn run_kinds(runs: &serde_json::Value) -> Vec<&str> {
    runs.as_array()
        .expect("runs array")
        .iter()
        .filter_map(|run| run["kind"].as_str())
        .collect()
}

// ---- I9 第二段：refine / 多洞 ----

const AND_EXERCISE: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := sorry\n";

const AND_MULTI_HOLE: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro sorry sorry\n";

async fn code_actions_for(service: &mut LspService<Backend>, src: &str) -> Vec<CodeAction> {
    let hole = offset_of(src, "sorry");
    let hole_start = lsp_pos(src, hole);
    let result = call(
        service,
        RpcRequest::build("textDocument/codeAction")
            .params(json!({
                "textDocument": {"uri": URI},
                "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                "context": {"diagnostics": []},
            }))
            .id(50)
            .finish(),
    )
    .await
    .expect("codeAction must answer");
    let actions: Option<CodeActionResponse> =
        serde_json::from_value(result).expect("valid CodeActionResponse");
    actions
        .expect("code actions")
        .into_iter()
        .filter_map(|a| match a {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            CodeActionOrCommand::Command(_) => None,
        })
        .collect()
}

// ---- 行业基线补全：completions / folding ----

async fn request_completions(service: &mut LspService<Backend>) -> Vec<CompletionItem> {
    let result = call(
        service,
        RpcRequest::build("textDocument/completion")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": {"line": 0, "character": 0},
            }))
            .id(60)
            .finish(),
    )
    .await
    .expect("completion must answer");
    let response: Option<CompletionResponse> =
        serde_json::from_value(result).expect("valid CompletionResponse");
    match response.expect("completions must be returned") {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    }
}

// ---- 导航基线：go-to-definition / documentHighlight / binder 补全 ----

async fn goto_definition_at(
    service: &mut LspService<Backend>,
    src: &str,
    offset: usize,
) -> Option<GotoDefinitionResponse> {
    let result = call(
        service,
        RpcRequest::build("textDocument/definition")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(lsp_pos(src, offset)),
            }))
            .id(70)
            .finish(),
    )
    .await
    .expect("textDocument/definition must answer");
    serde_json::from_value(result).expect("valid GotoDefinitionResponse")
}

async fn document_highlight_at(
    service: &mut LspService<Backend>,
    src: &str,
    offset: usize,
) -> Option<Vec<DocumentHighlight>> {
    let result = call(
        service,
        RpcRequest::build("textDocument/documentHighlight")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(lsp_pos(src, offset)),
            }))
            .id(71)
            .finish(),
    )
    .await
    .expect("textDocument/documentHighlight must answer");
    serde_json::from_value(result).expect("valid document highlight response")
}

async fn request_completions_at(
    service: &mut LspService<Backend>,
    pos: Position,
) -> Vec<CompletionItem> {
    let result = call(
        service,
        RpcRequest::build("textDocument/completion")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(72)
            .finish(),
    )
    .await
    .expect("completion must answer");
    let response: Option<CompletionResponse> =
        serde_json::from_value(result).expect("valid CompletionResponse");
    match response.expect("completions must be returned") {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    }
}

/// 生成 N 条声明 + 1 条 open 练习的画布源码。
fn perf_canvas(n: usize) -> String {
    let mut lines = vec!["axiom P : Prop".to_string(), "axiom proofP : P".to_string()];
    for i in 1..=n {
        lines.push(format!("theorem solved_{i} : P := proofP"));
    }
    lines.push("theorem exercise : P := sorry".to_string());
    lines.join("\n") + "\n"
}

// ---- 优先级可视化：selectionRange（学习者需求）----

const DEMO_K: &str =
    "theorem demo_K : (a : Prop) -> a -> a :=\n  fun (a : Prop) => fun (h : a) => h\n";

async fn selection_range_at(
    service: &mut LspService<Backend>,
    src: &str,
    offset: usize,
) -> Option<SelectionRange> {
    let pos = lsp_pos(src, offset);
    let result = call(
        service,
        RpcRequest::build("textDocument/selectionRange")
            .params(json!({
                "textDocument": {"uri": URI},
                "positions": [position_json(pos)],
            }))
            .id(70)
            .finish(),
    )
    .await
    .expect("selectionRange must answer");
    let response: Option<Vec<SelectionRange>> =
        serde_json::from_value(result).expect("valid SelectionRange");
    response.expect("array").into_iter().next()
}

// ---- 括号 hover：`(表达式)` 的 ( 与 ) 都显示 `表达式 : 类型` ----

/// 用户指定的括号 hover 语料（and_not_absurd：括号应用 + Not 展开 +
/// `$N` 还原三个痛点都在这一行里）。
const AND_NOT_ABSURD: &str = concat!(
    "axiom False : Prop\n",
    "axiom And : Prop -> Prop -> Prop\n",
    "axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n",
    "axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a\n",
    "axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b\n",
    "def Not : Prop -> Prop := fun (a : Prop) => a -> False\n",
    "theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :=\n",
    "  fun (a : Prop) (h : And a (Not a)) => ",
    "(And.right a (Not a) h) (And.left a (Not a) h)\n",
);

async fn hover_markup_at(service: &mut LspService<Backend>, src: &str, offset: usize) -> String {
    hover_markup_opt_at(service, src, offset)
        .await
        .expect("position must have hover")
}

async fn hover_markup_opt_at(
    service: &mut LspService<Backend>,
    src: &str,
    offset: usize,
) -> Option<String> {
    let pos = lsp_pos(src, offset);
    let result = call(
        service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(95)
            .finish(),
    )
    .await
    .expect("hover must answer");
    let hover: Option<Hover> = serde_json::from_value(result).expect("valid hover");
    hover.map(|h| match h.contents {
        HoverContents::Markup(m) => m.value,
        other => panic!("expected markup hover, got {other:?}"),
    })
}

/// 组内配对 `)` 的偏移：`(And.right a (Not a) h)` 里第一个 ` h)` 的 `)`。
fn group_close(src: &str, group_open: usize) -> usize {
    group_open + src[group_open..].find(" h)").expect("group closing paren") + 2
}

/// The full `Hover` (contents + range) at a byte offset, for range assertions.
async fn hover_opt_at(
    service: &mut LspService<Backend>,
    src: &str,
    offset: usize,
) -> Option<Hover> {
    let pos = lsp_pos(src, offset);
    let result = call(
        service,
        RpcRequest::build("textDocument/hover")
            .params(json!({
                "textDocument": {"uri": URI},
                "position": position_json(pos),
            }))
            .id(96)
            .finish(),
    )
    .await
    .expect("hover must answer");
    serde_json::from_value(result).expect("valid hover")
}

async fn open_and_wait(src: &str) -> (LspService<Backend>, ClientSocket) {
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;
    let _ = wait_diagnostics(&mut socket, "bracket suite diagnostics").await;
    (service, socket)
}
mod project;
