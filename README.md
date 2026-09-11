# sokonanoda-lang

An independent, self-contained Lean-4 teaching stack built **on top of** the
[sokonanoda](https://github.com/intgrah/sokonanoda) kernel.

This repository is an explicit fork of `sokonanoda@7b51784`. The kernel crate
keeps the upstream name (`sokonanoda`) and its full test suite. Nothing here is
intended to hide its provenance.

## Naming

- Product language: **sokonanoda** (the `.sokonanoda` teaching dialect)
- Source file suffix: **`.sokonanoda`**
- Kernel crate: `sokonanoda` (kept for attribution and stability)
- Workspace/repository: `sokonanoda-lang`

The repository will use no official Lean tooling (`lean`, `lake`,
`lean4export`, `leanc`, `elan`) at runtime, build time, or test time.

## Layout

```text
crates/kernel   Full sokonanoda kernel (complete core + thin teaching API)
crates/front    .sokonanoda lexer / parser / elaborator / document engine
crates/cli      `sokonanoda` command-line front-end for .sokonanoda files
crates/lsp      `sokonanoda-lsp` language server (tower-lsp)
editor/vscode   Experimental VS Code client (unpackaged)
skills/         Agent Skills for code agents (teacher + developer)
examples/       Sample .sokonanoda lesson files
```

See [ROADMAP.md](ROADMAP.md).

Documentation:

- [STATUS.md](STATUS.md) — current status & progress log (read this
  first if you are picking the project up);
- [docs/architecture.md](docs/architecture.md) — deep architecture + kernel
  tour;
- [docs/notes/research.md](docs/notes/research.md) — survey of teaching-oriented proof
  languages and infrastructure lessons;
- [docs/design/infrastructure.md](docs/design/infrastructure.md) — gap
  analysis and design brainstorm for the next milestones;
- [docs/notes/inductive.md](docs/notes/inductive.md) — what `inductive/ctor/rec/iota` mean
  and how the kernel reduces them;
- [docs/protocol.md](docs/protocol.md) — CLI/editor/agent feedback protocol
  (human text lines + JSON Lines).

## Use it (no Rust toolchain required)

**VS Code users**: install the extension from the Marketplace — the
platform-specific package ships both the language server and the `sokonanoda`
CLI, so checking, the goal view and the course map work offline out of the
box.

**In this repo**: one idempotent command (design: `docs/design/onboarding.md`):

```bash
bash scripts/soko.sh setup && bash scripts/soko.sh doctor
```

opencode users get the same via `/sokonanoda/setup` + `/sokonanoda/doctor`
(the startup plugin provisions automatically and puts the binaries on PATH).

**Command line / agents**: download the version-pinned binaries from GitHub
Releases (the tarballs contain runnable executables — no cargo, no checkout):

```bash
V=0.9.0   # or this checkout's version: grep -m1 '^version' Cargo.toml | cut -d'"' -f2
TARGET=aarch64-apple-darwin   # linux: x86_64-unknown-linux-gnu / aarch64-unknown-linux-gnu; win: x86_64-pc-windows-msvc
BIN="$HOME/.local/share/sokonanoda/bin"; mkdir -p "$BIN"
for pkg in sokonanoda-cli sokonanoda-lsp; do
  curl -fsSL "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v${V}/${pkg}-${TARGET}.tar.gz" \
    | tar xz -C "$BIN"
done
"$BIN/sokonanoda" --json your-file.sokonanoda
```

`--json` prints one JSON event per line (the machine/agent view); errors carry
a stable code (`elab-*` / `kernel-rejected` / …) plus a teaching hint.
**Never use `releases/latest`** — always pin `v${version}`, otherwise a newer
server would be paired with an older client.

## Build from source (contributors)

Contributors need the Rust toolchain; from this checkout:

```text
cargo test
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl
cargo run -q -p sokonanoda-lsp --bin sokonanoda-lsp   # editor feedback channel
```

The editor path is LSP-first: `.sokonanoda` files stay declarative (no `#`
commands); the language server publishes per-declaration diagnostics, hover
types for every sub-expression and open-exercise goals, document symbols and
exercise status lenses. See `editor/vscode/` for the thin client.

In `repl`, declarations accumulate line by line. Commands:

- `#check <expr>` — print the inferred type;
- `#reduce <expr>` — evaluate closed terms;
- `#print <name>` — print a declaration (types and proof terms).

`#prove` shows that tactics are just building the lambda:

```text
proof> #prove {a : Prop} -> a -> a
goal: a -> a
lambda: fun {a : Prop} => sorry
proof> intro h
goal: a
lambda: fun {a : Prop} => fun (h : a) => sorry
proof> exact h
lambda: fun {a : Prop} => fun (h : a) => h
proof> done
checked example
```

The CLI parses a `.sokonanoda` file, elaborates it into kernel declarations
and runs the complete sokonanoda kernel over them:

```text
checked declaration id
id: Prop -> Prop
exercise open (fill the sorry)
```

Numeric universes are available as `Sort 0`/`Sort 1`/… (`Prop` and `Type` are
abbreviations), so the type of function types is checkable:

```text
> #check Sort 2
Sort 2: Type 2
> #check (fun (α : Sort 2) => α)
(fun (α : Sort 2) => α): Type 1 -> Type 1
```

Universe polymorphism uses declaration-level parameters and explicit
applications:

```text
def id {u} : {α : Sort u} -> (a : α) -> α :=
  fun (α : Sort u) => fun (a : α) => a

def id0 : (α : Prop) -> α -> α :=
  fun (α : Prop) => id.{0} α
```

Without an explicit application (`#check id`) the universe defaults to zero.

`@id.{u}` is accepted as an alias, and named arrows make binders part of the
arrow chain: `(x : A) -> B` means `forall (x : A), B`, and `{x : A} -> B`
means an implicit binder. The ported
[`examples/py-fol-core.sokonanoda`](examples/py-fol-core.sokonanoda) mirrors
py_nanobruijn's FOL fragments and is checked by both front-end and CLI tests.

Errors are printed as `line:col: error: ...` without kernel panic traces.

## For code agents

Code agents are first-class users of this repo, two ways:

- **As the teacher** (product vision): load the
  `skills/sokonanoda-teacher` Agent Skill — it packages the teaching loop
  (write `sorry` exercises on the canvas → run the kernel via `--json` events →
  decide the next step from `decl.checked` / `exercise.open` / `diagnostic`).
  See `skills/README.md` for installation.
- **As the developer**: `skills/sokonanoda-dev` packages the hard rules
  (frozen kernel, Lean-4-subset syntax, kernel-only judging) and the
  TDD/docs-first workflow.

Editor feedback for agents needs no extra tooling: the repo-root
`opencode.json` starts the LSP through `.opencode/lsp/sokonanoda-lsp.sh`, which
resolves an installed VS Code extension binary, a local build, or the
version-pinned GitHub Release binary (auto-download; set
`SOKONANODA_LSP_OFFLINE=1` to forbid the network step). CLI-based agents get
the same contract via `--json` and `watch`.

## Development principles

- **TDD**: every grammar point, command and error mode is added through tests
  first (`crates/front`, `crates/cli/tests/cli.rs`), then implemented.
- **Repetition**: the same behavior is exercised at unit level, end-to-end
  kernel level, and CLI level, so a regression is caught repeatedly.
- **Feedback is a feature**: the compiler and CLI output types, reductions,
  printed declarations, errors and environment state, so both a human and a
  model can drive the tool without consulting documentation.
- **Self-documenting CLI**: `--help` and REPL `#help` describe the language;
  `#env` lists what has been declared.
- **Arrow-first types**: prefer `A -> B -> C` and named arrows
  `(x : A) -> B` / `{x : A} -> B`; `forall` is only for grouping when it is
  clearer.

A second example defines the classic logical vocabulary from scratch and
proves core theorems about it:

```text
"$BIN/sokonanoda" examples/fol-basics.sokonanoda
```
