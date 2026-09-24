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
- **A broken signature is never mistaken for "not done yet"** — an exercise's
  signature is type-checked too, `sorry` or not: `theorem t : 3 := sorry`
  reports `kernel-expected-sort` on the signature, and a `theorem`'s type must
  be a `Prop` (`kernel-theorem-not-prop`). A typo'd lemma name or a wrong
  conclusion in a 100-exercise canvas surfaces as a diagnostic instead of a
  silently "open" exercise.
- **Types on hover, with real names** — hover any expression (including
  inside parentheses) for `expression : type`; partial applications
  print your actual binder names, definition heads stay folded
  (`Not a`, not `a -> False`).
- **Honest warnings** — a declaration named `Prop`, `Sort` or `Type` still
  compiles, but those names are already defined by the kernel, so the name
  can never be used; the editor flags it with a `reserved-declaration-name`
  warning. And when the answer is already complete but a leftover `sorry`
  line follows it, the editor flags that exact line with a `redundant-sorry`
  warning — the declaration stays open, and the fix is to delete the line
  rather than keep proving.

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
  style) in its **own container on the right side bar**, so it can sit next to
  your proof: multi-goal columns with each goal's hypotheses, `by k/n`
  progress, the declaration list (an **open** declaration also shows its goal as
  a coloured `⊢ …` line, one row per remaining goal), and the live server
  version. Goal and
  hypothesis text is coloured from the **same single source** as the editor's
  semantic highlighting (no separate, drifting rules), **and it uses your
  file's own notation** — a goal the kernel prints as `Set.subset α A B`
  shows up as `A ⊆ B`, exactly as you wrote it (binder grouping, `Type 0`
  and line breaks are preserved byte for byte), it reads the same
  kernel-checked data as the tree, updates only on debounced caret moves
  (never refetches the declaration list on cursor movement), and falls back to
  the Explorer tree's 「当前光标处」 group when webviews are unavailable. Open it
  with `sokonanoda: 打开目标面板 (Infoview)` (needs VS Code 1.106+).
- **Hint ladders** — each exercise carries 2–3 progressive hints
  (`-- soko:hint` directives); reveal them one at a time when stuck.
- **Your own notation** — declare `infix:50 " ∈ " => Set.mem` (or `infixl:`,
  `infixr:`, and the nullary `notation "∅" => Set.empty`) and write
  `a ∈ A` instead of `Set.mem α a A`. Mathematical symbols are first-class
  tokens, declared symbols are highlighted as operators, and notation is pure
  sugar: it emits no event, and the pointful spelling keeps grading
  identically. Scope is per file, after the declaration (not across `import`
  yet).
- **Multi-file projects** — start a file with `import Logic` and the whole
  import closure is compiled as one program: declarations from imported
  modules are in scope for diagnostics, hover, completion and code actions,
  and **go to definition jumps into the imported module**. An optional
  `sokonanoda.toml` at the project root names the module root (`Logic` ↔
  `Logic.sokonanoda`, `Lib/And.sokonanoda` ↔ `Lib.And`); without a manifest
  the entry file's own directory is the root, so two files next to each
  other just work. Errors are attributed to the file that caused them
  (`import-not-found`, `import-cycle`, …). Editing an imported module re-checks
  the files that depend on it right away — unsaved edits included — and find
  references / rename work across the whole project.
- **Project tree** — an Explorer view (`项目`) shows the import closure around
  the active file: the **module root and where it came from** (a
  `sokonanoda.toml` path, or "zero config: the entry file's directory"), every
  module in topological order with its status — `compiled`, `load-failed`
  (the root cause, e.g. a missing import), or `blocked` (a victim of another
  module's failure) — plus per-module declaration/error counts and the
  project-level diagnostics. Click a module to open it; click the root to open
  the manifest. The status bar tooltip names the project too, and a single
  file (no `import`) says so instead of showing an empty tree. Refresh with
  the view's refresh button or `sokonanoda: refresh project view`.
- **Course map** — an 11-unit structured course with verified solutions
  (propositional logic first; `by` tactic blocks early for fast feedback;
  universes only when you naturally ask "what's the type of a function
  type?"; induction split into two units; relations & connectives in unit 9,
  reading proofs & synthesis in unit 10, modules & projects in unit 11 — per
  the locked plan in `docs/design/course-syllabus.md` §0 and unit 11 in
  `docs/design/imports-and-projects.md`).

**A real editing experience**

- Completions (keywords, in-scope binders, prelude names)
- Go-to-definition, rename and find references — all **across imported
  modules**; document highlight stays within the file
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
- **Notation input, the Lean 4 way**: type `\and` and press `Tab` to get `∧`
  (`\in` → `∈`, `\sub` → `⊆`, `\powerset` → `𝒫`, …)
- **Notation is navigable**: hover a notation symbol to see three things — what
  it expands to (`Set.mem`), **its own signature** (`Set.mem : forall (α : Type 0),
  α -> Set α -> Prop`), and how to type it; the hover box covers **exactly the
  symbol** (so `⁻¹'` and `×ˢ` are framed correctly). `F12` / ctrl+click on a
  notation symbol jumps to the `infix:`/`prefix:`/`postfix:` line that declared
  it — **across `import`**, into the library module.

## Typing notation (`\and` → `∧`)

Write the abbreviation and press `Tab`:

| you type | you get | aliases |
|---|---|---|
| `\and` | `∧` | `\wedge` |
| `\or` | `∨` | `\vee` |
| `\iff` | `↔` | `\leftrightarrow` |
| `\not` | `¬` | `\neg` |
| `\to` | `→` | `\imp` |
| `\forall` | `∀` | — |
| `\exists` | `∃` | — |
| `\ne` | `≠` | `\neq` |
| `\in` | `∈` | `\mem` |
| `\sub` | `⊆` | `\subseteq` |
| `\cup` | `∪` | `\union` |
| `\cap` | `∩` | `\inter` |
| `\setminus` | `\` | — |
| `\empty` | `∅` | `\emptyset` |
| `\powerset` | `𝒫` | — |
| `\compl` | `ᶜ` | `\complement` |
| `\preim` | `⁻¹'` | `\preimage` |
| `\xs` | `×ˢ` | — |

The abbreviations are **copied verbatim from Lean 4**, so the muscle memory
transfers; hovering a symbol shows the same information (`∈` → "输入：`\in`
（别名 `\mem`）"). `Tab` is only taken over **while a `\`-word is being typed**:
ordinary indentation and suggestion acceptance in `.sokonanoda` files keep
working, and a lone `\` (the set-difference symbol) is never rewritten.

Two more behaviours worth knowing:

- **Eager mode** — set `sokonanoda.input.eager` to `true` and an abbreviation
  is replaced as soon as the word is complete, no `Tab` needed. While you keep
  typing letters a prefix waits (`\an` waits for `\and`, `\in` waits for
  `\inter`); a separator closes the word and finishes it (`\in ` → `∈ `,
  `\sub ` → `⊆ `). Off by default, because `Tab` is the explicit, reviewable
  path.
- **Undo** — a replacement is a single edit, so **one undo takes it back in
  one step**; with several cursors, each abbreviation is rewritten in that
  same single edit.

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

The same grading verdict is also available as **one JSON object**:
`sokonanoda query <op>` (`check`/`state`/`goals`/`holes`/`hints`/`reduce`)
answers a single question for agents and scripts — `query state` answers
"what is still missing at this position" without scanning the whole event
stream. `scripts/soko query …` is the harness-neutral form of the same
command.

## Build and rebuild the compile cache

> **`sokonanoda: build`（`alt+b`）在大项目上是分钟级**：它按**文件**逐个预热，
> 每个文件各编一遍自己那一份 import 闭包（文件之间不共享）。实测
> `courses/set-theory`（35 个文件）约 **2.5 分钟**（debug CLI、冷热都一样——
> 冷热差异取决于项目缓存有没有命中，见 `docs/PERF.md`）。只想快速看一个文件时
> 直接打开它即可，不必先 `build`。

The compiler keeps a **persistent compile cache** (`.sokonanoda` → compiled
report), so the second run of a file — and the first keystroke in a project —
are hits instead of full recompiles. Two commands drive it from the editor:

- **`sokonanoda: build`** (`alt+b`) — compile the active `.sokonanoda` file
  (the CLI follows its `import` closure), or the first workspace folder when no
  file is open. The result line reports `files · compiled · hit · failed`.
- **`sokonanoda: rebuild`** (`alt+shift+b`) — the same, but first runs
  `build --clean` to drop the cache, i.e. "recompile everything from scratch".

Both write the CLI's JSON Lines events (`build.file` / `build.clean` /
`build.summary`) to the **sokonanoda build** output channel, refresh the
exercise/project/course views afterwards (a warm cache changes what they show),
and warn — never fail silently — when a file does not compile.

**Warming on open** — set `sokonanoda.warmCacheOnOpen` to `true` and the
extension runs one `build` over the **workspace root** in the background when
the window activates, so the first unit you open is already a cache hit.
**Off by default**: it costs CPU/IO, and "opening the editor" itself gets
slower — the other side of the same complaint. It never steals focus (progress
goes to the *sokonanoda build* channel) and never errors (a failed warm-up just
means nothing was pre-built; `sokonanoda: doctor` tells you why).

## The course map

The 「课程」tree shells out to the `sokonanoda` CLI (it aggregates all course
units — the language server itself tracks several documents but answers one
project closure at a time). The CLI ships in the same platform package as the
server, so it works out of the box; in a development host without a staged
`bin/`, a workspace `target/{debug,release}` build is used instead.

## Requirements

- VS Code 1.106+ (the Infoview uses an extension-contributed
  secondary-side-bar container)
- No network needed on platforms with a bundled package; the fallback
  package for other platforms downloads once (or use
  `sokonanoda.serverPath` to point at a local build)

## Links

- Repository & docs:
  [github.com/ColorlessBoy/sokonanoda-lang](https://github.com/ColorlessBoy/sokonanoda-lang)
- Release notes: [CHANGELOG](https://github.com/ColorlessBoy/sokonanoda-lang/blob/main/editor/vscode/CHANGELOG.md)

## License

Apache-2.0
