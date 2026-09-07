//! CLI entry: argument parsing, dispatch to file-check / repl.

mod check;
mod help;
mod json_report;
mod repl;
mod watch;

use check::check_source;
use help::print_help;
use repl::repl;
use std::io::Read;
use std::process::ExitCode;
use watch::watch;

fn main() -> ExitCode {
    let mut json = false;
    let mut bare = false;
    let mut positionals: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--json" => json = true,
            "--bare" => bare = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            _ => positionals.push(arg),
        }
    }
    match positionals.first().map(String::as_str) {
        Some("repl") if !json => repl(),
        Some("repl") => {
            eprintln!("error: --json is only supported for batch checking, not the repl");
            ExitCode::FAILURE
        }
        Some("watch") => match positionals.get(1) {
            Some(path) => watch(path),
            None => {
                eprintln!("usage: sokonanoda watch <file.sokonanoda>");
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
