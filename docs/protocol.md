# `.sokonanoda` collaboration protocol (v1, implemented + draft)

## Goal

The same compiler feedback is consumed by:

- a human editing a `*.sokonanoda` file in an editor;
- a code agent writing definitions and exercises in that file;
- the VS Code layer (later), which renders one view per side.

The protocol has **one vocabulary and two renderings**: human text lines and
machine JSON Lines (`sokonanoda --json <file>`). Both are implemented today in
`crates/cli/src/main.rs`; a future service layer keeps the same event names.

## Editor surface: LSP, not `#` commands

In the VS Code vision the `.sokonanoda` file stays **declarative**: only
`def` / `theorem` / `axiom` / `inductive … end` / `example` declarations and
`--` narrative comments. `#check` / `#reduce` / `#print` / `#prove` are REPL
and self-test conveniences, **not part of the teaching file format**.

Feedback that a REPL would get from a `#` command comes from the editor:

| REPL command | LSP surface |
|---|---|
| `#check x` | hover on `x` → type |
| `#reduce e` | command/inline action → reduced value |
| `#prove` | goal view + code actions over the `???` hole |
| printed errors | `publishDiagnostics` with spans and codes |

The event vocabulary below is the **internal transport** (batch CLI, tests,
agent, and later the service); the LSP maps the same data onto LSP protocol
messages. See `docs/design-infrastructure.md` for the feedback capability list.

## Canonical text lines (human)

```text
checked declaration <name>
checked example
<expr>: <type>
<expr> => <value>
exercise open (fill the ???)
<line>:<col>: error[<stage>]: <message>
```

`<stage>` is one of `parse` / `elab` / `kernel` (see "Error staging" below).

## Machine events (JSON Lines, `--json`)

One JSON object per line on stdout. Every event carries `type` and `human`;
payload fields are additive and machine-meaningful.

| `type` | payload | from |
|---|---|---|
| `decl.checked` | `name` | declaration passed the complete kernel |
| `example.checked` | — | an `example` with a filled value passed the kernel |
| `expr.typed` | `text` (source slice), `inferred_type`, `span` | `#check` |
| `expr.reduced` | `text` (source slice), `value`, `span` | `#reduce` |
| `decl.printed` | `name`, `text` | `#print` |
| `exercise.open` | `name` (optional) | an open answer slot (`def`/`theorem`/`example` with `???` value) |
| `diagnostic` | `stage`, `code`, `message`, `span` | any error |

Example:

```json
{"type":"decl.checked","name":"id","human":"checked declaration id"}
{"type":"expr.typed","human":"id: Prop -> Prop","inferred_type":"Prop -> Prop","span":{"start":{"offset":52,"line":2,"column":8},"end":{"offset":54,"line":2,"column":10}},"text":"id"}
{"type":"exercise.open","human":"exercise open (fill the ???)","name":"ex"}
{"type":"diagnostic","stage":"kernel","code":"kernel-rejected","message":"rejected: def_eq failed","span":{...}}
```

`text` is always the exact source slice of the checked expression, so a model
can re-run or display it without re-parsing.

## Error staging and codes

Errors carry a stable machine `code` and a stage. Codes are fine-grained so
that a model or editor can react to the *kind* of mistake, not the wording:

- `parse` stage — `unexpected-token`, `unexpected-eof`;
- `elab` stage — e.g. `elab-unknown-identifier`, `elab-unknown-constant`,
  `elab-unknown-universe-level`, `elab-universe-arity`, `elab-untyped-binder`,
  `elab-hole-misplaced`, `elab-duplicate-declaration`, `elab-too-many-binders`,
  `elab-nat-literal-disabled`, `elab-invalid-nat-literal`,
  `elab-too-many-ctor-fields`, `elab-unknown-ctor-for-iota`;
- `kernel` stage — `kernel-rejected` (kernel said no; conversion failures
  carry the expected/actual sides), and the fine-grained families
  `kernel-expected-sort` (a term appeared where a type was required),
  `kernel-expected-pi` (a non-function value was used as a function, or an
  application was over-applied), `kernel-theorem-not-prop` (a theorem whose
  type is not a proposition), `kernel-inductive-non-positive` (a recursive
  occurrence in a negative position of a constructor argument),
  `kernel-ctor-result-mismatch` (a constructor does not return a full
  application of its inductive), `kernel-ctor-arg-invalid-app` (a recursive
  occurrence in a constructor argument is not a valid application — wrong
  number or values of parameters/indices), `kernel-ctor-arg-not-type` (a
  constructor argument type is a term, not a type),
  `kernel-ctor-arg-too-large` (a constructor argument type lives in a
  universe too large for the inductive), `kernel-rec-rule-mismatch` (an
  explicitly declared recursor/`iota` rule set does not match the
  kernel-derived one — rules missing, out of constructor order, wrong rule
  value, or a wrong recursor name), plus `kernel-internal` (a kernel
  bug; never a learner mistake).

Human output prints `error[<code>]: <message>`; JSON diagnostics carry
`stage`, `code`, `message` and a `hint`. The LSP maps the same data onto
`publishDiagnostics` (message + hint, code, range) and `hover` (type map /
goal text); see `docs/design-infrastructure.md` F1–F8.

## Live session: `sokonanoda watch <file>` (implemented)

`watch` is the CLI form of the service layer: it monitors a file and prints one
JSON object per change:

```json
{"type":"file.changed","version":3,"recompiled_from":0}
{"type":"exercise.solved","name":null,"version":3}
```

- `recompiled_from` is the index of the first changed command; `null` means
  the commands did not change (comment-only edit) and nothing was recompiled.
- Delta events (from `sokonanoda_front::session`): `exercise.opened`,
  `exercise.solved`, `exercise.failed`, `decl.checked`, `decl.failed` — each
  carries `name` (when named) and `version`.
- Diagnostics for the new version follow, with `stage`/`code`/`message`/
  `hint`/`span` as in batch mode.

## Kernel diagnostics with expected/actual

A kernel rejection whose cause is a conversion failure now carries both sides:
the human message is rendered as
"类型不匹配：期望 `<E>`，实际是 `<A>`", and `--json` diagnostics expose the
same text (plus `hint`). `CompileError` additionally carries the machine fields
`expected`/`actual` when present.

## Editor semantic tokens

The LSP server implements `textDocument/semanticTokens` (full). Legend:
KEYWORD, TYPE (Sort / inductive), NUMBER, MACRO (`???`), FUNCTION
(def/theorem names and uses), VARIABLE (axioms, unresolved idents),
ENUM_MEMBER (constructors), PARAMETER (binders). Encoding is UTF-16 correct.

## Custom LSP requests (goal view, I9)

Beyond standard LSP, the server answers two custom requests (tower-lsp
`custom_method`; clients opt in, servers don't advertise them in
capabilities):

### `soko/goals`

Request params: `{"textDocument": {"uri"}, "position"}` (position reserved).
Response:

```json
{"decls": [{
  "name": "and_swap", "kind": "theorem", "status": "open",
  "range": {"start": {...}, "end": {...}},
  "goal": "And b a",
  "binders": [{"name": "a", "ty": "Prop"}, {"name": "h", "ty": "And a b"}],
  "hole": {"start": {...}, "end": {...}},
  "holes": [{"start": {...}, "end": {...}}, ...],
  "sub_goals": [{"range": {"start": {...}, "end": {...}}, "ty": "b"}, ...]
}]}
```

- one entry per declaration (all statuses); `goal`/`binders`/`hole` are
  present for open exercises (`hole` is the exact `???` range);
- `holes` lists every `???` (multi-hole constructor spines included) as
  objects `{"range": {…}, "id": "<declName>:<index>"}` — the id is stable
  per (declaration, hole order) within a document version (anonymous
  examples use the `example@<line>` name form) and is the stable reference
  for external tools; `sub_goals` pairs each spine hole with its expected
  type (server-side walk; parameter positions expect the goal's own
  argument, proof positions the instantiated field type); `ty` is `null`
  when no template is known;
- multi-hole documents are naturally supported (one entry per declaration);
- hover remains the degraded, human-readable view of the same data;
- kernel-judged code actions (next-step suggestions, per goal shape; see
  `docs/design-hints-suggestions.md`): at most three per request, the first
  one carries `is_preferred: true`:
  - `exact <hypothesis>` — a hypothesis the kernel judges defeq to the hole's
    expected type (per hole: in a constructor spine each sub-hole is judged
    against its own expected type, never against the outer goal);
  - `Eq.refl …` — for `Eq`-shaped goals, a `rfl` candidate that the kernel
    validated before it is offered (dropped when rejected);
  - `refine <skeleton>` — a constructor skeleton recovered from the
    declaration's own axiom/ctor shape (e.g. `And.intro a b ??? ???`);
    structural, the kernel judges what the learner writes into sub-holes;
  - `intro` — peel the next binder(s) into a lambda prefix.

### `soko/hints`

Request params: `{"textDocument": {"uri"}, "position"}`. Response:

```json
{"hints": ["先看目标最外层的箭头…", "目标形态：拆成 fun (x : a) => …"]}
```

- The full hint ladder authored in the canvas as `-- soko:hint <text>`
  comment directives (one hint per line, whole line a comment; each hint
  attaches to the first declaration after it), for the declaration at
  `position`; empty array when there is none;
- The server is stateless: progressive disclosure (reveal one hint at a
  time) is a client concern; the protocol never counts remaining hints.

## Rename, references & inlay hints (LSP 3.17)

- `textDocument/prepareRename` — resolves the target at the cursor
  (binder or declaration); returns `{range, placeholder}` covering the
  **name token** only; unresolved (prelude names, anonymous `example`)
  → `null`.
- `textDocument/rename` — semantic rewrite only: every use point that
  resolves to the same target plus the definition's name token
  (`WorkspaceEdit.documentChanges` with the document's version). Illegal
  identifiers and unresolvable positions are **ResponseErrors**, never
  empty edits or text scans.
- `textDocument/references` — all use points of the target; with
  `include_declaration` the definition's name location is prepended;
  results are sorted by offset and deduplicated.
- `textDocument/inlayHint` — one type hint per open-exercise hole, rendered
  after the hole: `label = ": <expected type>"` (sub-hole types from the
  server-side walk; the remaining goal for a lone main hole), markdown
  tooltip with the goal and introduced hypotheses. Checked/failed
  declarations produce no hints.

### `soko/nextHole`

Request params: `{"textDocument": {"uri"}, "position", "forward": true}`.
Response: `null` or the `range` of the next open hole after (or, with
`forward: false`, before) the cursor. The server owns hole-position logic
(ocaml-lsp lesson: clients should not re-derive positions).

Kernel-judged tactics: `exact` code actions are computed by `front::judge`
(a synthesized complete declaration checked by the full kernel) — no text
matching anywhere in the editor path.

## Watch stream (L1 CLI form)

`sokonanoda watch <file>` emits one JSON object per line: a `file.changed`
opener per new document version, then the versioned delta of declaration and
exercise state transitions, then the current diagnostics. The delta vocabulary
is closed:

- `file.changed` (`{type, version, recompiled_from}`)
- `decl.checked` / `decl.failed` (`{type, name, version}`)
- `exercise.opened` / `exercise.solved` / `exercise.failed` (`{type, name, version}`)
- `diagnostic` (`{type, stage, code, message, hint, span, version}`)

`recompiled_from` is the index of the first re-checked command (`null` when
nothing changed): with the I8 incremental session, editing command `i` only
kernel-rechecks commands `i..n`. `SessionUpdate.stats.kernel_checks` (front
API) exposes the same fact as a count.

## Course map: `sokonanoda course <course.json>`

Aggregates the units of `course/course.json` (the agent-facing material
library) into a progress map; the VS Code course tree consumes it. Unit
paths resolve relative to the manifest's directory.

- `course.unit` (`{type, file, title, unit, checked, open, failed, reduced[,
  error]}`) — per-unit counts (`checked` = `decl.checked`, `open` = open
  exercises, `failed` = declarations the kernel/elab rejected, `reduced` =
  `expr.reduced`); `error` carries a message when the unit file could not
  be read or parsed;
- `course.summary` (`{type, units, checked, open, failed}`) — totals.

Human view: one line per unit (`unit 1 命题与证明 —— 12 checked · 5 open ·
0 failed`) plus a totals line. Exit code is 0 even with open/failed
exercises (progress is not an error); only an unreadable manifest fails.

## REPL history

`sokonanoda repl` appends every non-empty input line to
`$HOME/.sokonanoda_history` (created on demand; silently disabled when
`HOME` is unset, truncated to the most recent 1000 lines on load).
Line-editing / arrow-key recall is out of scope.

## Future structured event names (service layer)

When the resident service replaces `watch`, keep the same vocabulary and add:

- `file.didChange`
- `decl.rejected` (today: a `diagnostic` with stage `kernel`)
- `diagnostic.*` kinds once sub-stage codes exist

Each event carries `span {offset,line,column}` plus a human text and a machine
payload when relevant.

## Agent contract

An agent can drive the tool by:

1. appending declarations and exercises to the current buffer;
2. recompiling and reading only the new event lines (`--json` for machine view);
3. matching `decl.checked`, `expr.typed`, `exercise.open`, `diagnostic` events;
4. using `#env` in the REPL (or `decl.checked` history) to list loaded
   declarations when starting a lesson.

No editor-specific API is part of this protocol; the protocol is text/events
only, so VS Code, CLI, tests and agents can share it.
