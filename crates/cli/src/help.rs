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
    println!("  sokonanoda course <course.json> ... [--all]");
    println!("                                  aggregate unit progress (JSON with --json;");
    println!("                                  several manifests / --all = one aggregate map)");
    println!("  sokonanoda build [path ...]     warm the compile cache (--clean clears it)");
    println!("  sokonanoda query <op> [opts]    kernel truth as ONE JSON object (agent view):");
    println!("      check                     counts + failures + warnings for the file");
    println!("      state  --line L --col C   goal state at the caret (Lean goalsAt? semantics)");
    println!("      goals  [--probe]          every declaration: type, status, goals, holes");
    println!("      holes  [--offset N] [--direction next|prev]");
    println!("                                addressable holes (stable ids), optional step");
    println!("      hints  --line L --col C   the `-- soko:hint` ladder for that declaration");
    println!("      reduce --expr E           kernel normal form of an expression");
    println!("      project                   project closure: root, manifest, modules, statuses");
    println!("                                (single file ⇒ project:null + reason)");
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
    println!("  abbrev <name> : <type> := <value>   (same as def; the Lean spelling)");
    println!("  theorem <name> : <type> := <proof>");
    println!("  axiom <name> : <type>");
    println!("  example : <type> := <value>    (use sorry for an open exercise)");
    println!("  #check <expr>                  print the inferred type");
    println!("  #reduce <expr>                 evaluate a closed expression");
    println!("  #print <name>                  print a declaration");
    println!("  infix:N \" sym \" => name        user notation: binary, no assoc (N 1-1000)");
    println!("  infixl:N / infixr:N \" sym \" => name   left / right associative");
    println!("  prefix:N \" sym \" => name       unary before its operand (e.g. \" 𝒫 \")");
    println!("  postfix:N \" sym \" => name      unary after its operand (e.g. \" ᶜ \")");
    println!("  notation \" sym \" => name       nullary constant (e.g. \"∅\")");
    println!("                                 notation is scoped to the file + its imports");
    println!("  namespace <name> ... end <name>  declarations inside get the prefix");
    println!("                                 (`namespace A` + `def mem` => `A.mem`)");
    println!("  open <name>                    make `<name>.` omissible for references");
    println!("  open <name> (a b)              only these short names (only-clause)");
    println!("  open <name> hiding a b         every short name but these");
    println!("  open <name> renaming a => b    rename a short name (a is then gone)");
    println!("  open <name> ... in <command>   local: only that one command sees it");
    println!("  export <name> [<clauses>]      like open, and importing files see it too");
    println!("  open scoped <name>             open scoped notations only (not names)");
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
