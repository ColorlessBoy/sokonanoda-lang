# sokonanoda — learn theorem proving in a Lean-4-style language

Zero setup. Install the extension, open a `.sokonanoda` file, and the
language server **downloads itself** from GitHub Releases on first use
(rust-analyzer model) — no Rust toolchain, no checkout, no official Lean
required. Every exercise you finish is graded by a **complete
Lean-4-compatible kernel**: if it's green, it's a real proof.

Part of the
[sokonanoda-lang](https://github.com/ColorlessBoy/sokonanoda-lang)
suite: a self-contained teaching compiler + language server + agent
skills.

## What you get

**Proof checking that doesn't lie**

- **Real kernel grading** — declarations are checked by a full dependent
  type checker (not text matching or heuristics). Solved exercises show
  as solved; `sorry` placeholders are graded as warnings, real mistakes
  as errors.
- **Types on hover, with real names** — hover any expression (including
  inside parentheses) for `expression : type`; partial applications
  print your actual binder names, definition heads stay folded
  (`Not a`, not `a -> False`).

**Guided exercises**

- **Exercise canvas** — `sorry` marks an exercise. Fill it in, save, and
  the kernel grades you instantly.
- **Goal view** — an Explorer tree shows every declaration's status
  (open / solved / failed), the remaining goal and the hypotheses you
  have introduced; `alt+n` / `alt+shift+n` jump between holes.
- **Hint ladders** — each exercise carries 2–3 progressive hints
  (`-- soko:hint` directives); reveal them one at a time when stuck.
- **Course map** — a 5-unit structured course with verified solutions
  (propositional logic first; universes only when you naturally ask
  "what's the type of a function type?").

**A real editing experience**

- Completions (keywords, in-scope binders, prelude names)
- Go-to-definition, document highlight, rename, find references
- Inlay hints showing the expected type at each hole
- Code actions: introduce-and-refine templates, `exact` suggestions,
  restart scaffolds — every suggestion is kernel-verified before it's
  offered
- Semantic highlighting (including `sorry`), folding ranges, smart
  select that visualises precedence

## Install & use

1. Install this extension from the Marketplace.
2. Open any `.sokonanoda` file (or the `playground.sokonanoda` that
   ships in the repo — 12 exercises, zero to theorem).
3. On first use the extension downloads the matching
   `sokonanoda-lsp` binary for your platform
   (macOS arm64/x86_64, Linux x86_64, Windows x86_64) from the
   [latest GitHub Release](https://github.com/ColorlessBoy/sokonanoda-lang/releases/latest)
   and caches it under `~/.local/share/sokonanoda/bin/`.

The server is discovered in this order:

1. the `sokonanoda.serverPath` setting (or `SOKONANODA_LSP_BIN` env),
2. `target/debug|release/sokonanoda-lsp` in your workspace (repo
   checkouts — handy when developing the compiler),
3. `sokonanoda-lsp` on `PATH`,
4. the cached download above, then a fresh download as the last resort.

**No Lean toolchain.** The kernel ships inside the server; everything
works offline after the first download.

## Teaching with AI agents

The sokonanoda-lang repo ships three
[agent skills](https://github.com/ColorlessBoy/sokonanoda-lang/tree/main/skills)
that any skill-aware agent (Claude Code, opencode, …) can load:

| Skill | Turns an agent into… |
|---|---|
| `sokonanoda-teacher` | Your proof teacher: assigns exercises from the canvas, grades via kernel `--json` events, walks the hint ladder with you |
| `sokonanoda-dev` | A careful contributor to the compiler stack (frozen kernel, TDD discipline) |
| `sokonanoda-ci` | A disciplined releaser (CI verification, release pipeline) |

The repo-root `opencode.json` already wires the language server for
`.sokonanoda` files, so opening this repo in opencode gives you
kernel-checked diagnostics in the agent's own editor loop. The same
`--json` event stream (`sokonanoda --json file`, `sokonanoda watch`)
is the integration surface for any tooling you want to build.

## The course map needs one extra binary

The 「课程」tree shells out to the `sokonanoda` CLI (it aggregates all
course units — the language server stays single-document). The CLI is
not auto-downloaded yet; pick one:

- `cargo build --release -p sokonanoda-cli` (then it is discovered from
  `target/release/`), or
- put `sokonanoda` on your `PATH`.

Without it, everything else (checking, hover, goal view, hints) works
out of the box.

## Requirements

- VS Code 1.85+
- Network access for the one-time server download (or use
  `sokonanoda.serverPath` to point at a local build)

## Links

- Repository & docs:
  [github.com/ColorlessBoy/sokonanoda-lang](https://github.com/ColorlessBoy/sokonanoda-lang)
- Release notes: [CHANGELOG](https://github.com/ColorlessBoy/sokonanoda-lang/blob/main/editor/vscode/CHANGELOG.md)

## License

Apache-2.0
