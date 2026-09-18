//! Help text: top-level usage and REPL command help.

pub(crate) fn print_help() {
    println!("sokonanoda — self-contained .sokonanoda compiler");
    println!();
    println!("usage:");
    println!("  sokonanoda <file.sokonanoda>   check a file");
    println!("  sokonanoda -                    check source from stdin");
    println!("  sokonanoda repl                 interactive REPL");
    println!("  sokonanoda watch [--doc <file> | --workspace <root>]");
    println!("                                  monitor file(s), emit versioned JSON Lines events");
    println!(
        "  sokonanoda lsp                  run the language server on stdio (editors spawn this)"
    );
    println!("  sokonanoda course <course.json> aggregate unit progress (JSON with --json)");
    println!("  sokonanoda build [path ...]     warm the compile cache (--clean clears it)");
    println!("  sokonanoda query <op> [opts]    kernel truth as ONE JSON object (agent view):");
    println!("      check                     counts + failures + warnings for the file");
    println!("      state  --line L --col C   goal state at the caret (Lean goalsAt? semantics)");
    println!("      goals  [--probe]          every declaration: type, status, goals, holes");
    println!("      holes  [--offset N] [--direction next|prev]");
    println!("                                addressable holes (stable ids), optional step");
    println!("      hints  --line L --col C   the `-- soko:hint` ladder for that declaration");
    println!("      reduce --expr E           kernel normal form of an expression");
    println!("    input: --file <path> | --text <src> | stdin;  --compact for one line");
    println!("    exit: 0 ok (open `sorry` is legal) · 1 kernel-rejected · 2 usage");
    println!("  sokonanoda --json <file>        emit JSON Lines events (agent/service view)");
    println!("  sokonanoda --bare <file>        compile with no prelude (file is self-contained);");
    println!(
        "                                  files can also declare `-- sokonanoda:prelude none`"
    );
    println!("  sokonanoda --help               this help");
    println!();
    println!("multi-file projects (import + optional sokonanoda.toml):");
    println!("  import Logic                   load another module first (must precede");
    println!(
        "                                 declarations; module name = file path, `-` not allowed)"
    );
    println!("  --root <dir>                   module root for `import` resolution (else the");
    println!(
        "                                 nearest sokonanoda.toml walking up, else the file's dir)"
    );
    println!(
        "  --no-project                   ignore sokonanoda.toml; module root = the file's dir"
    );
    println!();
    println!("language commands (same in files and REPL):");
    println!("  def <name> : <type> := <value>");
    println!("  theorem <name> : <type> := <proof>");
    println!("  axiom <name> : <type>");
    println!("  example : <type> := <value>    (use sorry for an open exercise)");
    println!("  #check <expr>                  print the inferred type");
    println!("  #reduce <expr>                 evaluate a closed expression");
    println!("  #print <name>                  print a declaration");
    println!("  universes: def id {{u}}; Sort u; explicit application @id.{{u}}");
    println!("  types: A -> B -> C; named arrows (x : A) -> B bind x");
}

pub(crate) fn print_repl_help() {
    println!("commands: #check <expr>, #reduce <expr>, #print <name>,");
    println!("          #env, #prove <goal>, #help, #exit");
    println!(
        "proof mode: intro <name>, exact <term>, apply <term>, assumption, undo, lambda, done"
    );
    println!("declarations accumulate; one expression or declaration per line.");
}
