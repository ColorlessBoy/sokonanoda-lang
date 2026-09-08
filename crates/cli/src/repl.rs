//! Interactive REPL: line-based declaration/proof session loop.

use crate::check::report_output;
use crate::help::print_repl_help;
use sokonanoda_front::compile::{prelude_mode_from_source, CheckEvent, CompileOptions};
use sokonanoda_front::parse;
use sokonanoda_front::proof::ProofState;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HISTORY_FILE: &str = ".sokonanoda_history";
const HISTORY_MAX_LINES: usize = 1000;

pub(crate) fn repl() -> ExitCode {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let history_path = history_path();
    let mut history = load_history(history_path.as_deref());
    let mut buffer = String::new();
    let mut seen_events = 0usize;
    let mut declared: Vec<String> = Vec::new();
    let mut proof: Option<ProofState> = None;
    println!("sokonanoda repl — press Ctrl-D to exit");
    print_repl_help_with_history();
    loop {
        print!("{}", if proof.is_some() { "proof> " } else { "> " });
        let _ = std::io::stdout().flush();
        let Some(Ok(line)) = lines.next() else {
            break;
        };
        if !line.trim().is_empty() {
            history.push(line.clone());
            append_history_line(history_path.as_deref(), &line);
        }
        if (line.trim() == "done" || line.trim() == "#done") && proof.is_some() {
            let state = proof.as_ref().expect("proof");
            if !state.done() {
                eprintln!("error: fill the sorry with `exact <term>` first");
                continue;
            }
            buffer.push_str("\nexample : ");
            buffer.push_str(state.goal_source());
            buffer.push_str(" := ");
            buffer.push_str(&state.lambda_text());
            buffer.push('\n');
            proof = None;
            run_buffer(&buffer, &mut seen_events, &mut declared);
            continue;
        }
        if let Some(state) = proof.as_mut() {
            match line.trim() {
                "#exit" | "exit" | "quit" | "q" => {
                    proof = None;
                    continue;
                }
                _ => {}
            }
            // 判定选项跟随 buffer 的 prelude 指令（与整份重编译语义一致）。
            let options = CompileOptions {
                prelude: prelude_mode_from_source(&buffer),
            };
            if line.trim() == "undo" || line.trim() == "u" || line.trim() == "#undo" {
                if state.undo() {
                    print_proof_state(state);
                } else {
                    eprintln!("error: 没有可撤销的证明步");
                }
                continue;
            }
            if line.starts_with("intro ") {
                let name = line.strip_prefix("intro ").unwrap_or("").trim();
                match state.intro(name) {
                    Ok(()) => {
                        print_proof_state(state);
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.starts_with("exact ") {
                let term = line.strip_prefix("exact ").unwrap_or("").trim();
                // I9：kernel 先裁决术语，通过才填入；失败给出期望/实际。
                match state.exact_kernel(term, &buffer, &options) {
                    Ok(()) => {
                        println!("lambda: {}", state.lambda_text());
                        println!("kernel-checked ✓ — use `done` to append the proof.");
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.starts_with("apply ") {
                let term = line.strip_prefix("apply ").unwrap_or("").trim();
                match state.exact_kernel(term, &buffer, &options) {
                    Ok(()) => {
                        println!("lambda: {}", state.lambda_text());
                        println!("kernel-checked ✓ — use `done` to append the proof.");
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.trim() == "assumption" {
                // I9：从最内层假设起让 kernel 逐个裁决（文本比对已删除）。
                match state.assumption_kernel(&buffer, &options) {
                    Ok(()) => {
                        println!("lambda: {}", state.lambda_text());
                        println!("kernel-checked ✓ — use `done` to append the proof.");
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.trim() == "lambda" || line.trim() == "#lambda" {
                println!("lambda: {}", state.lambda_text());
                continue;
            }
            continue;
        }
        match line.trim() {
            "#help" | "help" | "?" => {
                print_repl_help_with_history();
                continue;
            }
            "#env" | "env" => {
                if declared.is_empty() {
                    println!("(no user declarations yet)");
                } else {
                    println!("user declarations:");
                    for name in &declared {
                        println!("  {name}");
                    }
                }
                continue;
            }
            "#exit" | "exit" | "quit" | "q" => break,
            "#prove" => {
                eprintln!("usage: #prove <goal type>");
                continue;
            }
            _ => {}
        }
        if let Some(rest) = line.strip_prefix("#prove ") {
            match ProofState::start(rest.trim()) {
                Ok(state) => {
                    proof = Some(state);
                    if let Some(state) = proof.as_ref() {
                        print_proof_state(state);
                    }
                }
                Err(e) => eprintln!("error: {e}"),
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        buffer.push_str(&line);
        buffer.push('\n');

        run_buffer(&buffer, &mut seen_events, &mut declared);
    }
    ExitCode::SUCCESS
}

pub(crate) fn run_buffer(buffer: &str, seen_events: &mut usize, declared: &mut Vec<String>) {
    let options = CompileOptions {
        prelude: prelude_mode_from_source(buffer),
    };
    let output = match parse(buffer) {
        Ok(file) => sokonanoda_front::compile::compile_fol_with(&file, &options),
        Err(diag) => {
            eprintln!(
                "{}:{}: error[{}]: {}",
                diag.span.start.line,
                diag.span.start.column,
                diag.stage_code(),
                diag.message
            );
            return;
        }
    };
    report_output(&output, buffer, *seen_events);
    *seen_events = output.events.len();
    if output.errors.is_empty() {
        declared.clear();
        for event in &output.events {
            if let CheckEvent::DeclarationChecked { name } = event {
                declared.push(name.clone());
            }
        }
    }
}

pub(crate) fn print_proof_state(state: &ProofState) {
    println!("goal: {}", state.goal_text());
    println!("lambda: {}", state.lambda_text());
}

fn print_repl_help_with_history() {
    print_repl_help();
    println!("历史: $HOME/.sokonanoda_history");
}

fn history_path() -> Option<PathBuf> {
    match std::env::var_os("HOME") {
        Some(home) if !home.is_empty() => Some(PathBuf::from(home).join(HISTORY_FILE)),
        _ => None,
    }
}

fn load_history(path: Option<&Path>) -> Vec<String> {
    let Some(path) = path else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let skip = lines.len().saturating_sub(HISTORY_MAX_LINES);
    lines.into_iter().skip(skip).collect()
}

fn append_history_line(path: Option<&Path>, line: &str) {
    let Some(path) = path else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}
