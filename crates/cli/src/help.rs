//! Help text: top-level usage and REPL command help.

pub(crate) fn print_help() {
    println!("sokonanoda — self-contained .sokonanoda compiler");
    println!();
    println!("usage:");
    println!("  sokonanoda <file.sokonanoda>   check a file");
    println!("  sokonanoda -                    check source from stdin");
    println!("  sokonanoda repl                 interactive REPL");
    println!("  sokonanoda --json <file>        emit JSON Lines events (agent/service view)");
    println!("  sokonanoda --bare <file>        compile with no prelude (file is self-contained);");
    println!(
        "                                  files can also declare `-- sokonanoda:prelude none`"
    );
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

pub(crate) fn print_repl_help() {
    println!("commands: #check <expr>, #reduce <expr>, #print <name>,");
    println!("          #env, #prove <goal>, #help, #exit");
    println!("proof mode: intro <name>, exact <term>, apply <term>, assumption, lambda, done");
    println!("declarations accumulate; one expression or declaration per line.");
}
