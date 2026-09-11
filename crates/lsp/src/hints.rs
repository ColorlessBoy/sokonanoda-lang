//! `soko/hints` custom request: the hint ladder of the declaration at the
//! cursor (docs/design/hints-suggestions.md, protocol.md).
//!
//! Ladders are authored in the canvas as `-- soko:hint <text>` directives
//! and attached by `front::compile::hints`. The server is stateless — the
//! client owns progressive disclosure (reveal one hint at a time); the
//! protocol never counts remaining hints.

use super::render::decl_at;
use super::Doc;
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, TextDocumentIdentifier};

#[derive(Debug, Deserialize)]
pub(crate) struct HintsParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    text_document: TextDocumentIdentifier,
    #[serde(default)]
    position: Option<Position>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HintsResponse {
    pub hints: Vec<String>,
}

/// The declaration at `position`'s full ladder; empty when there is none.
pub(crate) fn hints_for(doc: &Doc, params: HintsParams) -> HintsResponse {
    let Some(report) = &doc.report else {
        return HintsResponse { hints: Vec::new() };
    };
    let position = params.position.unwrap_or_default();
    let hints = decl_at(&report.decls, position.line, position.character)
        .map(|d| d.hints.clone())
        .unwrap_or_default();
    HintsResponse { hints }
}

#[cfg(test)]
mod tests {
    use super::super::testutil::{
        call, did_open, handshake, lsp_pos, offset_of, test_service, wait_diagnostics, URI,
    };
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;

    const HINTED: &str =
        "-- soko:hint 先看最外层箭头\n-- soko:hint 拆成 lambda\nexample : Prop -> Prop := sorry\n";

    async fn ask_hints(
        service: &mut tower_lsp::LspService<crate::Backend>,
        line: u32,
        character: u32,
    ) -> serde_json::Value {
        call(
            service,
            RpcRequest::build("soko/hints")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "position": {"line": line, "character": character},
                }))
                .id(80)
                .finish(),
        )
        .await
        .expect("soko/hints must answer")
    }

    #[tokio::test]
    async fn hints_returns_the_ladder_of_the_declaration() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, HINTED).await;
        let _ = wait_diagnostics(&mut socket, "hints diagnostics").await;

        let hole = offset_of(HINTED, "sorry");
        let pos = lsp_pos(HINTED, hole);
        let result = ask_hints(&mut service, pos.line, pos.character).await;
        let hints = result
            .get("hints")
            .and_then(|h| h.as_array())
            .expect("hints array")
            .iter()
            .map(|h| h.as_str().expect("string hint"))
            .collect::<Vec<_>>();
        assert_eq!(hints, vec!["先看最外层箭头", "拆成 lambda"], "{result:?}");
    }

    #[tokio::test]
    async fn hints_are_empty_outside_any_declaration() {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, HINTED).await;
        let _ = wait_diagnostics(&mut socket, "hints diagnostics").await;
        let result = ask_hints(&mut service, 0, 0).await;
        assert_eq!(
            result["hints"].as_array().map(Vec::len),
            Some(0),
            "the directive line itself is not a declaration: {result:?}"
        );
    }
}
