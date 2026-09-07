//! `sokonanoda-lsp` binary entry: the server lives in the library so the
//! single `sokonanoda` binary can also host it (`sokonanoda lsp`).

fn main() {
    sokonanoda_lsp::run();
}
