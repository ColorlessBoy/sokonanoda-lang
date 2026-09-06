use sokonanoda_front::compile::{compile_fol, CheckEvent};
use sokonanoda_front::parse;
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

    let label = match input.as_deref() {
        None | Some("-") => "<stdin>".to_string(),
        Some(path) => path.to_string(),
    };

    let file = match parse(&src) {
        Ok(file) => file,
        Err(diag) => {
            eprintln!(
                "{}:{}:{}: error: {}",
                label, diag.span.start.line, diag.span.start.column, diag.message
            );
            return ExitCode::FAILURE;
        }
    };

    let out = compile_fol(&file);
    for event in &out.events {
        match event {
            CheckEvent::DeclarationChecked { name } => println!("checked declaration {name}"),
            CheckEvent::ExampleChecked => println!("checked example"),
            CheckEvent::TypeChecked { text } => println!("#check : {text}"),
            CheckEvent::Reduced { text } => println!("#reduce => {text}"),
            CheckEvent::ExerciseOpen => println!("exercise open (fill the ???)"),
        }
    }
    for err in &out.errors {
        eprintln!(
            "{}:{}:{}: error: {}",
            label, err.span.start.line, err.span.start.column, err.message
        );
    }
    if out.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
