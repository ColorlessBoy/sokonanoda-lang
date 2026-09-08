# sokonanoda — Lean-style theorem proving, taught

Part of the [sokonanoda-lang](https://github.com/ColorlessBoy/sokonanoda-lang)
suite: a self-contained Lean-4-style teaching compiler with a complete type
checker, a language server, and AI-agent skills for interactive theorem
proving lessons.

## What you get

- **Real kernel checking** — every declaration you write is checked by a
  complete Lean-4-compatible type checker (not text matching). If it's green,
  it's a real proof.
- **Exercise canvas** — `???` (or `sorry`) marks an exercise. Fill it in, save,
  and the kernel grades you instantly.
- **Goal view** — hover any expression for its type; hover `sorry` for the
  remaining goal and hypotheses.
- **Hint ladders** — each exercise has 2–3 progressive hints (authored as
  `-- soko:hint` comment directives); reveal them one at a time when stuck.
- **Precedence visualisation** — smart-select (Ctrl+Shift+→) on an operator
  highlights the binding scope.
- **Exercise panel** — Explorer tree showing every declaration's status
  (open / solved / failed) with goal, hypotheses and hole navigation.
- **Course map** — a 5-unit structured course with verified solutions
  (propositional logic first; universes when you naturally ask "what's the
  type of a function type?").

## AI-agent integration

The repo ships [agent skills](https://github.com/ColorlessBoy/sokonanoda-lang/tree/main/skills)
for Claude Code / opencode that turn an LLM into a sokonanoda teacher: it
assigns exercises, grades via `--json` kernel events, and adapts to the
learner. The repo-root `opencode.json` wires the LSP for `.sokonanoda` files
automatically.

## Quick start

1. Build the language server: `cargo build -p sokonanoda-lsp` (from the repo root)
2. Open a `.sokonanoda` file — the extension activates automatically
3. Start with exercise 1 in `playground.sokonanoda`

## Requirements

- The `sokonanoda-lsp` binary (auto-discovered from `target/debug/`, `target/release/`, or `PATH`)
- No official Lean toolchain required (fully self-contained)
