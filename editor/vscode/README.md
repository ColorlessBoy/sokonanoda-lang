# sokonanoda VS Code extension (experimental)

Thin client for the `sokonanoda-lsp` language server: grammar, language
configuration and one plain `extension.js`. No TypeScript, no bundler;
packaging for the marketplace is a later task.

## 1. Build the server

```bash
# from the repo root
cargo build -p sokonanoda-lsp
# -> target/debug/sokonanoda-lsp
```

## 2. Run the extension locally

- Open `editor/vscode` as the workspace and press **F5** ("Launch sokonanoda
  client" — `.vscode/launch.json` builds the server first via a pre-launch
  task, then opens an Extension Development Host window), or
- open the repo, `code editor/vscode`, and open any `.sokonanoda` file; the
  extension activates on `onLanguage:sokonanoda` and spawns the server over
  stdio.

### Server discovery (`sokonanoda.serverPath` empty by default)

1. `sokonanoda.serverPath` setting (absolute path wins);
2. `SOKONANODA_LSP_BIN` environment variable;
3. `target/debug/sokonanoda-lsp`, then `target/release/sokonanoda-lsp`, in
   each workspace folder and in the repo checkout next to the extension;
4. `sokonanoda-lsp` on `PATH`.

The extension runs as a workspace extension (`extensionKind: ["workspace"]`)
so the binary is spawned on the machine holding the files (WSL/remote-safe).
In restricted (untrusted) workspaces the workspace-level `serverPath` setting
is ignored and default discovery is used.

## 3. Feedback you get

- **Semantic highlighting**: on by default, no configuration needed. The
  client auto-registers the provider because the server advertises
  `textDocument/semanticTokens/full` (zero settings, zero `extension.js`
  logic — vscode-languageclient v9 does the wiring). It colors:
  - keywords (`def`, `theorem`, `example`, `axiom`, `inductive`, `ctor`,
    `rec`, `iota`, `end`, `fun`, `forall`/`∀`, `#check`/`#reduce`/`#print`);
  - sorts `Prop`/`Type`/`Sort` and inductive type names (as types);
  - definition/theorem names (as functions), axiom names, constructors
    (as enum members), binders (as parameters) and `???` holes (macro —
    stands out as "exercise open here");
  - numbers, and unknown identifiers (variables) as a fallback.
  The classic TextMate grammar stays in place as an instant fallback (it
  paints while the server starts and wherever semantic tokens are absent,
  e.g. before the first `initialize` round-trip).
- **Diagnostics** per declaration with stable codes (`kernel-rejected`,
  `unexpected-token`, …) and teaching hints.
- **Hover**: inferred type of the expression under the cursor, or the goal of
  an open `???` exercise.
- **Code lenses / document symbols**: every declaration is labelled with its
  status (`exercise: open` / `solved ✓` / `failed`); clicking a lens shows the
  status summary.
- **Quick fix `intro`**: on an open exercise whose goal starts with binders —
  inserts the first lambda step (tactics are just lambdas).
- **`sokonanoda: show exercise status` (`alt+s`)**: quick pick listing
  `name — kind/status`; selecting an entry reveals the declaration.

## 4. Settings

| Setting | Meaning |
|---|---|
| `sokonanoda.serverPath` | Explicit server binary path; empty = auto-discovery above. |
| `sokonanoda.trace.server` | `verbose` logs the full JSON-RPC traffic to the "sokonanoda" output channel. |

All client/server logging goes to that output channel — never to stdout,
which carries the LSP stream.
