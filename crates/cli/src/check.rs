//! Batch checking: parse and compile a source, then report events or diagnostics.

use crate::json_report::{print_json_line, report_json, span_json};
use sokonanoda_front::compile::{compile_fol, CheckEvent, CompileOutput};
use sokonanoda_front::parse;

pub(crate) fn check_source(src: &str, label: &str, json: bool) -> bool {
    let file = match parse(src) {
        Ok(file) => file,
        Err(diag) => {
            if json {
                print_json_line(&serde_json::json!({
                    "type": "diagnostic",
                    "stage": "parse",
                    "code": diag.code(),
                    "message": diag.message,
                    "hint": diag.hint(),
                    "span": span_json(diag.span),
                }));
            } else {
                eprintln!(
                    "{}:{}:{}: error[{}]: {}",
                    label,
                    diag.span.start.line,
                    diag.span.start.column,
                    diag.stage_code(),
                    diag.message
                );
            }
            return false;
        }
    };
    let output = compile_fol(&file);
    if json {
        report_json(&output, src);
    } else {
        report_output(&output, src, 0);
    }
    output.errors.is_empty()
}

pub(crate) fn report_output(output: &CompileOutput, src: &str, seen_events: usize) {
    for event in output.events.iter().skip(seen_events) {
        match event {
            CheckEvent::DeclarationChecked { name } => println!("checked declaration {name}"),
            CheckEvent::ExampleChecked => println!("checked example"),
            CheckEvent::TypeChecked { text, span } => {
                println!("{}: {text}", expr_text(src, *span));
            }
            CheckEvent::Reduced { text, span } => {
                println!("{} => {text}", expr_text(src, *span));
            }
            CheckEvent::Printed { name, text } => println!("#print {name} :\n{text}"),
            CheckEvent::ExerciseOpen { .. } => println!("exercise open (fill the ???)"),
        }
    }
    for err in &output.errors {
        eprintln!(
            "{}:{}: error[{}]: {}",
            err.span.start.line,
            err.span.start.column,
            err.code(),
            err.message
        );
    }
}

pub(crate) fn expr_text<'a>(src: &'a str, span: sokonanoda_front::Span) -> &'a str {
    if span.end.offset <= src.len() {
        &src[span.start.offset..span.end.offset]
    } else {
        "<expr>"
    }
}
