use sokonanoda_front::compile::{compile_fol, CheckEvent, CompileOutput};
use sokonanoda_front::parse;
use std::io::{BufRead, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let _ = args.next();
    match args.next().as_deref() {
        Some("repl") => repl(),
        Some("-h") | Some("--help") => {
            println!("usage: sokonanoda [FILE|-]  |  sokonanoda repl");
            ExitCode::SUCCESS
        }
        arg => check_path_or_stdin(arg),
    }
}

fn check_path_or_stdin(arg: Option<&str>) -> ExitCode {
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
    if check_source(&src, &label) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn repl() -> ExitCode {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut buffer = String::new();
    let mut seen_events = 0usize;
    println!("sokonanoda repl — press Ctrl-D to exit");
    loop {
        print!("> ");
        let _ = std::io::stdout().flush();
        let Some(Ok(line)) = lines.next() else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        buffer.push_str(&line);
        buffer.push('\n');

        let output = match parse(&buffer) {
            Ok(file) => compile_fol(&file),
            Err(diag) => {
                eprintln!(
                    "{}:{}: error: {}",
                    diag.span.start.line, diag.span.start.column, diag.message
                );
                continue;
            }
        };
        report_output(&output, &buffer, seen_events);
        seen_events = output.events.len();
    }
    ExitCode::SUCCESS
}

fn check_source(src: &str, label: &str) -> bool {
    let file = match parse(src) {
        Ok(file) => file,
        Err(diag) => {
            eprintln!(
                "{}:{}:{}: error: {}",
                label, diag.span.start.line, diag.span.start.column, diag.message
            );
            return false;
        }
    };
    let output = compile_fol(&file);
    report_output(&output, src, 0);
    output.errors.is_empty()
}

fn report_output(output: &CompileOutput, _src: &str, seen_events: usize) {
    for event in output.events.iter().skip(seen_events) {
        match event {
            CheckEvent::DeclarationChecked { name } => println!("checked declaration {name}"),
            CheckEvent::ExampleChecked => println!("checked example"),
            CheckEvent::TypeChecked { text } => println!("#check : {text}"),
            CheckEvent::Reduced { text } => println!("#reduce => {text}"),
            CheckEvent::ExerciseOpen => println!("exercise open (fill the ???)"),
        }
    }
    for err in &output.errors {
        eprintln!(
            "{}:{}: error: {}",
            err.span.start.line, err.span.start.column, err.message
        );
    }
}
