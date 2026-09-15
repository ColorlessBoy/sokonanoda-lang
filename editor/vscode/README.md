# sokonanoda — learn theorem proving in a Lean-4-style language

Zero setup. Install the extension, open a `.sokonanoda` file, and the
language server **and the `sokonanoda` CLI** are already there — they ship
inside the extension as platform-specific packages (macOS arm64/x86_64, Linux
x64/arm64, Alpine x64/arm64, Windows x64/arm64), so no Rust toolchain, no
checkout, and **no network download**. Every exercise you finish is graded by a
**complete Lean-4-compatible kernel**: if it's green, it's a real proof.

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
- **Honest warnings** — a declaration named `Prop`, `Sort` or `Type` still
  compiles, but those names are already defined by the kernel, so the name
  can never be used; the editor flags it with a `reserved-declaration-name`
  warning.

**Guided exercises**

- **Exercise canvas** — `sorry` marks an exercise. Fill it in, save, and
  the kernel grades you instantly.
- **Goal view** — an Explorer tree shows every declaration's status
  (open / solved / failed), the remaining goal and the hypotheses you
  have introduced; `alt+n` / `alt+shift+n` jump between holes.
- **Goals at cursor** — with the caret inside a `by` proof, the same tree
  shows the goal, the hypotheses in scope and your `by` progress at that
  position; click the goal to jump to the tactic.
- **Infoview panel** — a dockable, syntax-styled goal panel (Lean-Infoview
  style) next to 「练习」: multi-goal columns with each goal's hypotheses,
  `by k/n` progress, the declaration list, and the live server version. It
  reads the same kernel-checked data as the tree, updates only on
  debounced caret moves (never refetches the declaration list on cursor
  movement), and falls back to the tree's 「当前光标处」 group when webviews
  are unavailable. Open it with `sokonanoda: 打开目标面板 (Infoview)`.
- **Hint ladders** — each exercise carries 2–3 progressive hints
  (`-- soko:hint` directives); reveal them one at a time when stuck.
- **Course map** — a 6-unit structured course with verified solutions
  (propositional logic first; universes only when you naturally ask
  "what's the type of a function type?"; `by` tactic blocks last).

**A real editing experience**

- Completions (keywords, in-scope binders, prelude names)
- Go-to-definition, document highlight, rename, find references
- Inlay hints showing the expected type at each hole — and the result of
  every `#check` (`#check Nat` → `Nat : Type 0`, Lean-Infoview style)
- Code actions: introduce-and-refine templates, `exact` suggestions,
  restart scaffolds — every suggestion is kernel-verified before it's
  offered
- Semantic highlighting (including `sorry`), folding ranges, smart
  select that visualises precedence
- Restart the language server in place (`sokonanoda: restart server`) after
  rebuilding or refreshing the binary — no window reload needed (extension
  updates themselves still apply on reload)
- Greek binder letters (`α`, `β`, …) render plainly — the extension turns
  VS Code's confusable-character box off for `.sokonanoda` files by default

## Install & use

1. Install this extension from the Marketplace — VS Code picks the package
   with the server bundled for your platform.
2. Open any `.sokonanoda` file (or the `playground.sokonanoda` that
   ships in the repo — 12 exercises, zero to theorem).
3. That's it. The server binary lives in `bin/<platform>/` inside the
   extension; the client and its bundled kernel are built from the same
   release, so there is no version drift.

The server is discovered **bundled-first** by default:

1. the bundled `bin/<target>/sokonanoda-lsp[.exe]` (repairs a lost
   executable bit automatically) — the client and its kernel are built from
   the same release, so there is no version drift;
2. the version-pinned download cache — only used by the fallback package
   for platforms with no bundled build (e.g. Linux armhf), and always
   pinned to this extension's own release tag, never `latest`.

`sokonanoda.serverPath` / `SOKONANODA_LSP_BIN` and workspace
`target/debug|release` builds are **ignored unless you opt in** with the
`sokonanoda.serverOverride` setting (default `false`). This prevents a stale
local build (a common cause of "the server is still 0.26.0" surprises) from
silently overriding the bundled server. Use **`sokonanoda: doctor`** any time
to see which server is in use, its `source` (bundled / override / cache), the
running vs extension version, and other version-skew issues.

**No Lean toolchain.** The kernel ships inside the server; on platforms
with a bundled package everything works fully offline.

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

## The course map

The 「课程」tree shells out to the `sokonanoda` CLI (it aggregates all course
units — the language server stays single-document). The CLI ships in the same
platform package as the server, so it works out of the box; in a development
host without a staged `bin/`, a workspace `target/{debug,release}` build is
used instead.

## Requirements

- VS Code 1.85+
- No network needed on platforms with a bundled package; the fallback
  package for other platforms downloads once (or use
  `sokonanoda.serverPath` to point at a local build)

## Links

- Repository & docs:
  [github.com/ColorlessBoy/sokonanoda-lang](https://github.com/ColorlessBoy/sokonanoda-lang)
- Release notes: [CHANGELOG](https://github.com/ColorlessBoy/sokonanoda-lang/blob/main/editor/vscode/CHANGELOG.md)

## License

Apache-2.0
