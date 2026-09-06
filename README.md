# sokonanoda-lang

An independent, self-contained Lean-4 teaching stack built **on top of** the
[sokonanoda](https://github.com/intgrah/sokonanoda) kernel.

This repository is an explicit fork of `sokonanoda@7b51784`. The kernel crate
keeps the upstream name (`sokonanoda`) and its full test suite. Nothing here is
intended to hide its provenance.

## Naming

- Product language: **Follow** (tentative)
- Source file suffix: **`.fol`**
- Kernel crate: `sokonanoda` (kept for attribution and stability)
- Workspace/repository: `sokonanoda-lang`

The repository will use no official Lean tooling (`lean`, `lake`,
`lean4export`, `leanc`, `elan`) at runtime, build time, or test time.

## Layout

```text
crates/kernel   Full sokonanoda kernel (complete, unmodified core)
crates/front    .fol lexer / parser / diagnostics (restricted teaching grammar)
crates/cli      `folc` command-line front-end for .fol files
examples/       Sample .fol lesson files
```

See [ROADMAP.md](ROADMAP.md).
