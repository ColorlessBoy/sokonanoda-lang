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
crates/kernel   Full sokonanoda kernel (complete, unmodified core)
crates/front    .sokonanoda lexer / parser / diagnostics (restricted teaching grammar)
crates/cli      `sokonanoda` command-line front-end for .sokonanoda files
examples/       Sample .sokonanoda lesson files
```

See [ROADMAP.md](ROADMAP.md).

## Quick start

```text
cargo test
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl
```

The CLI parses a `.sokonanoda` file, elaborates it into kernel declarations
and runs the complete sokonanoda kernel over them:

```text
checked declaration id
#check : Prop -> Prop
exercise open (fill the ???)
```

Numeric universes are available as `Sort 0`/`Sort 1`/… (`Prop` and `Type` are
abbreviations), so the type of function types is checkable:

```text
> #check Sort 2
#check : Type 2
> #check (fun (α : Sort 2) => α)
#check : Type 1 -> Type 1
```

A second example defines the classic logical vocabulary from scratch and
proves core theorems about it:

```text
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/fol-basics.sokonanoda
```
