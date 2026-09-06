# sokonanoda VS Code extension (experimental)

Opens `*.sokonanoda` files with a thin client for the `sokonanoda-lsp` server:

- live diagnostics per declaration (stable codes + teaching hints);
- hover: inferred type of the expression under the cursor, or the goal of an
  open exercise (`???`);
- document symbols / code lenses showing exercise status (open / solved / failed);
- quick-fix `intro` on an open exercise (tactics as lambda building).

## Run

```bash
# from the repo root
cargo build -p sokonanoda-lsp
code editor/vscode
```

Then open a `.sokonanoda` file. To point at a different server binary set
`SOKONANODA_LSP_BIN=/abs/path/to/sokonanoda-lsp` before launching VS Code.

This is intentionally un-packaged (plain `extension.js`, no bundler) so it can
be iterated on; packaging for the marketplace is a later task.
