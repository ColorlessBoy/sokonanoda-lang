use sokonanoda_front::{parse, Command, Expr};
use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut src = String::new();
    let input = std::env::args().nth(1);
    let read_result = match input.as_deref() {
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

    match parse(&src) {
        Ok(file) => {
            println!("parsed {} command(s)", file.commands.len());
            for command in &file.commands {
                match command {
                    Command::Def { name, ty, val, .. } => {
                        println!("def {name} : {} := {}", expr_label(ty), expr_label(val));
                    }
                    Command::Theorem { name, ty, val, .. } => {
                        println!("theorem {name} : {} := {}", expr_label(ty), expr_label(val));
                    }
                    Command::Example { val, .. } => println!("example := {}", expr_label(val)),
                    Command::Axiom { name, ty, .. } => {
                        println!("axiom {name} : {}", expr_label(ty))
                    }
                    Command::Check { expr, .. } => println!("#check {}", expr_label(expr)),
                    Command::Reduce { expr, .. } => println!("#reduce {}", expr_label(expr)),
                }
            }
            ExitCode::SUCCESS
        }
        Err(diag) => {
            let label = match input.as_deref() {
                None | Some("-") => "<stdin>".to_string(),
                Some(path) => path.to_string(),
            };
            eprintln!(
                "{}:{}:{}: error: {}",
                label, diag.span.start.line, diag.span.start.column, diag.message
            );
            ExitCode::FAILURE
        }
    }
}

fn expr_label(expr: &Expr) -> &str {
    match expr {
        Expr::Hole { .. } => "???",
        _ => "<expr>",
    }
}
