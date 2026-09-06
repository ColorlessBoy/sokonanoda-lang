use sokonanoda_front::compile::{compile_fol, CheckEvent, CompileOutput};
use sokonanoda_front::parse;
use sokonanoda_front::proof::ProofState;
use std::io::{BufRead, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let _ = args.next();
    match args.next().as_deref() {
        Some("repl") => repl(),
        Some("-h") | Some("--help") => {
            print_help();
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
    let mut declared: Vec<String> = Vec::new();
    let mut proof: Option<ProofState> = None;
    println!("sokonanoda repl — press Ctrl-D to exit");
    print_repl_help();
    loop {
        print!("{}", if proof.is_some() { "proof> " } else { "> " });
        let _ = std::io::stdout().flush();
        let Some(Ok(line)) = lines.next() else {
            break;
        };
        if (line.trim() == "done" || line.trim() == "#done") && proof.is_some() {
            let state = proof.as_ref().expect("proof");
            if !state.done() {
                eprintln!("error: fill the ??? with `exact <term>` first");
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
            if line.starts_with("intro ") {
                let name = line["intro ".len()..].trim();
                match state.intro(name) {
                    Ok(()) => {
                        print_proof_state(state);
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.starts_with("exact ") {
                let term = line["exact ".len()..].trim();
                match state.exact(term) {
                    Ok(()) => {
                        println!("lambda: {}", state.lambda_text());
                        println!("use `done` to kernel-check this proof.");
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.starts_with("apply ") {
                let term = line["apply ".len()..].trim();
                match state.exact(term) {
                    Ok(()) => {
                        println!("lambda: {}", state.lambda_text());
                        println!("use `done` to kernel-check this proof.");
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
                continue;
            }
            if line.trim() == "assumption" {
                match state.assumption() {
                    Ok(()) => {
                        println!("lambda: {}", state.lambda_text());
                        println!("use `done` to kernel-check this proof.");
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
                print_repl_help();
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

fn run_buffer(buffer: &str, seen_events: &mut usize, declared: &mut Vec<String>) {
    let output = match parse(buffer) {
        Ok(file) => compile_fol(&file),
        Err(diag) => {
            eprintln!(
                "{}:{}: error: {}",
                diag.span.start.line, diag.span.start.column, diag.message
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

fn print_proof_state(state: &ProofState) {
    println!("goal: {}", state.goal_text());
    println!("lambda: {}", state.lambda_text());
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

fn report_output(output: &CompileOutput, src: &str, seen_events: usize) {
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

fn expr_text<'a>(src: &'a str, span: sokonanoda_front::Span) -> &'a str {
    if span.end.offset <= src.len() {
        &src[span.start.offset..span.end.offset]
    } else {
        "<expr>"
    }
}

fn print_help() {
    println!("sokonanoda — self-contained .sokonanoda compiler");
    println!();
    println!("usage:");
    println!("  sokonanoda <file.sokonanoda>   check a file");
    println!("  sokonanoda -                    check source from stdin");
    println!("  sokonanoda repl                 interactive REPL");
    println!("  sokonanoda --help               this help");
    println!();
    println!("language commands (same in files and REPL):");
    println!("  def <name> : <type> := <value>");
    println!("  theorem <name> : <type> := <proof>");
    println!("  axiom <name> : <type>");
    println!("  example : <type> := <value>    (use ??? for an open exercise)");
    println!("  #check <expr>                  print the inferred type");
    println!("  #reduce <expr>                 evaluate a closed expression");
    println!("  #print <name>                  print a declaration");
    println!("  universes: def id {{u}}; Sort u; explicit application @id.{{u}}");
    println!("  types: A -> B -> C; named arrows (x : A) -> B bind x");
}

fn print_repl_help() {
    println!("commands: #check <expr>, #reduce <expr>, #print <name>,");
    println!("          #env, #prove <goal>, #help, #exit");
    println!("proof mode: intro <name>, exact <term>, apply <term>, assumption, lambda, done");
    println!("declarations accumulate; one expression or declaration per line.");
}
