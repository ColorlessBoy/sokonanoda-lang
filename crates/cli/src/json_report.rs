//! JSON Lines reporting: machine-readable view of check events and diagnostics.

use crate::check::expr_text;
use sokonanoda_front::compile::{CheckEvent, CompileOutput};

pub(crate) fn print_json_line(value: &serde_json::Value) {
    println!("{}", serde_json::to_string(value).expect("serialize event"));
}

pub(crate) fn span_json(span: sokonanoda_front::Span) -> serde_json::Value {
    serde_json::json!({
        "start": {
            "offset": span.start.offset,
            "line": span.start.line,
            "column": span.start.column,
        },
        "end": {
            "offset": span.end.offset,
            "line": span.end.line,
            "column": span.end.column,
        },
    })
}

/// Machine view of a batch check. One JSON object per line, using the event
/// vocabulary from docs/protocol.md so a service or agent can consume it.
pub(crate) fn report_json(output: &CompileOutput, src: &str) {
    use CheckEvent::*;
    for event in &output.events {
        match event {
            DeclarationChecked { name } => print_json_line(&serde_json::json!({
                "type": "decl.checked",
                "human": format!("checked declaration {name}"),
                "name": name,
            })),
            ExampleChecked => print_json_line(&serde_json::json!({
                "type": "example.checked",
                "human": "checked example",
            })),
            TypeChecked { text, span } => {
                let expr_src = expr_text(src, *span);
                print_json_line(&serde_json::json!({
                    "type": "expr.typed",
                    "human": format!("{expr_src}: {text}"),
                    "text": expr_src,
                    "inferred_type": text,
                    "span": span_json(*span),
                }));
            }
            Reduced { text, span } => print_json_line(&serde_json::json!({
                "type": "expr.reduced",
                "human": format!("{} => {text}", expr_text(src, *span)),
                "text": expr_text(src, *span),
                "value": text,
                "span": span_json(*span),
            })),
            Printed { name, text } => print_json_line(&serde_json::json!({
                "type": "decl.printed",
                "human": format!("#print {name} :\n{text}"),
                "name": name,
                "text": text,
            })),
            ExerciseOpen { name } => {
                let mut event = serde_json::json!({
                    "type": "exercise.open",
                    "human": "exercise open (fill the ???)",
                });
                if let Some(name) = name {
                    event["name"] = serde_json::json!(name);
                }
                print_json_line(&event);
            }
        }
    }
    for err in &output.errors {
        print_json_line(&serde_json::json!({
            "type": "diagnostic",
            "stage": err.stage().code(),
            "code": err.code(),
            "message": err.message,
            "hint": err.hint(),
            "span": span_json(err.span),
        }));
    }
}
