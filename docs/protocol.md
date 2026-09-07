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
- `kernel` stage — `kernel-rejected` (kernel said no) and
  `kernel-internal` (a kernel bug; never a learner mistake).

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
