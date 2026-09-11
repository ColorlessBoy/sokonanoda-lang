//! CLI entry: argument parsing, dispatch to file-check / repl / lsp / course.

mod check;
mod course;
mod env;
mod help;
mod json_report;
mod repl;
mod watch;

use check::check_source;
use help::print_help;
use repl::repl;
use std::io::{IsTerminal, Read};
use std::process::ExitCode;
use watch::watch;

fn main() -> ExitCode {
    let mut json = false;
    let mut bare = false;
    let mut force = false;
    let mut positionals: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--json" => json = true,
            "--bare" => bare = true,
            "--force" => force = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("sokonanoda {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            _ => positionals.push(arg),
        }
    }
    match positionals.first().map(String::as_str) {
        // Environment subcommands (binary replacement for scripts/soko.sh).
        Some("version") => env::version_cmd(json),
        Some("doctor") => env::doctor(json),
        Some("setup") => env::setup(force),
        Some("update") => env::update(),
        Some("grade") => env::grade(&positionals[1..]),
        Some("gate") => env::gate(),
        Some("repl") if !json => repl(),
        Some("repl") => {
            eprintln!("error: --json is only supported for batch checking, not the repl");
            ExitCode::FAILURE
        }
        // 单二进制分发（gleam 模式）：编辑器可用 `sokonanoda lsp` 拉起服务器。
        // stdout 是 LSP 协议帧；任何提示只进 stderr。
        Some("lsp") if !json => {
            if std::io::stdin().is_terminal() {
                eprintln!("sokonanoda lsp: starting the language server on stdio (editors spawn this; you probably want `sokonanoda repl`)");
            }
            sokonanoda_lsp::run();
            ExitCode::SUCCESS
        }
        Some("lsp") => {
            eprintln!("error: --json is only supported for batch checking, not lsp");
            ExitCode::FAILURE
        }
        Some("watch") => match positionals.get(1) {
            Some(path) => watch(path),
            None => {
                eprintln!("usage: sokonanoda watch <file.sokonanoda>");
                ExitCode::FAILURE
            }
        },
        Some("course") => match positionals.get(1) {
            Some(manifest) => course::course(manifest, json),
            None => {
                eprintln!("usage: sokonanoda course <course.json>");
                ExitCode::FAILURE
            }
        },
        None => check_path_or_stdin(None, json, bare),
        Some(p) => check_path_or_stdin(Some(p), json, bare),
    }
}

fn check_path_or_stdin(arg: Option<&str>, json: bool, bare: bool) -> ExitCode {
    let mut src = String::new();
    let read_result = match arg {
        None | Some("-") => std::io::stdin().read_to_string(&mut src),
        Some(path) => std::fs::read_to_string(path).map(|s| {
            src = s;
            src.len()
        }),
    };
    if let Err(e) = read_result {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    let label = match arg {
        None | Some("-") => "<stdin>".to_string(),
        Some(path) => path.to_string(),
    };
    if check_source(&src, &label, json, bare) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
