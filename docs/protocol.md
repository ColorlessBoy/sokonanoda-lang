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
| `#prove` | goal view + code actions over the `sorry` hole |
| printed errors | `publishDiagnostics` with spans and codes |

The event vocabulary below is the **internal transport** (batch CLI, tests,
agent, and later the service); the LSP maps the same data onto LSP protocol
messages. See `docs/design/infrastructure.md` for the feedback capability list.

## Canonical text lines (human)

```text
checked declaration <name>
checked example
<expr>: <type>
<expr> => <value>
exercise open (fill the sorry)
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
| `exercise.open` | `name` (optional) | an open answer slot (`def`/`theorem`/`example` with `sorry` value) whose **signature elaborated and passed the kernel's type test** |
| `diagnostic` | `stage`, `code`, `message`, `span` | any error |
| `warning` | `code`, `message`, `hint`, `span` | a non-fatal lint (never affects the exit code) |

**`exercise.open` 的边界（G-01，2026-09-19）**：签名**不是**"值位是 `sorry`
就免检"的——签名 elaborate 不了、不是一个类型，或 `theorem` 的签名不是 `Prop`，
都会（与值位错误同罪）变成一条 `diagnostic`、声明是 Failed、**不**发
`exercise.open`。所以 `exercise.open` 的计数**不能**单独用来判断"签名有没有腐烂"：
判据要看 `diagnostic` / `failed`（这也正是 `docs/design/teaching-project.md` §6.4
那条纪律的样板）。签名诊断的 `span` 取**签名自身**的源范围，不照抄内核消息里的
span（G-15）。

Example:

```json
{"type":"decl.checked","name":"id","human":"checked declaration id"}
{"type":"expr.typed","human":"id: Prop -> Prop","inferred_type":"Prop -> Prop","span":{"start":{"offset":52,"line":2,"column":8},"end":{"offset":54,"line":2,"column":10}},"text":"id"}
{"type":"exercise.open","human":"exercise open (fill the sorry)","name":"ex"}
{"type":"diagnostic","stage":"kernel","code":"kernel-rejected","message":"rejected: def_eq failed","span":{...}}
```

`text` is always the exact source slice of the checked expression, so a model
can re-run or display it without re-parsing.

`warning` events use the same span shape as diagnostics but carry no `stage`
and never change the exit code. Codes today:

- `reserved-declaration-name`: `Prop` / `Sort` / `Type` are already defined by
  the kernel and cannot be declared again, so a top-level declaration with one
  of those names is accepted but never used
  (`docs/design/reserved-decl-warning.md`).
- `redundant-sorry`: the value already proves the goal and the `sorry` is an
  extra argument tacked onto a complete term, so deleting that line is what
  makes the declaration check. The declaration stays `exercise.open` (semantics
  unchanged) — the warning only says *which* line is the leftover. The verdict
  is the kernel's: the term with that argument removed must pass a full check
  of the declaration (`docs/design/redundant-sorry.md`).
  **对照**：`redundant-sorry` 是"值位已经证完、声明仍是 `exercise.open`"；
  签名的毛病是**另一个方向**——签名不过就不是练习，必须报 diagnostic
  （修 G-01 时明确**不**把它做成 warning：warning 不改退出码，
  课程侧就永远发现不了签名腐烂）。
- `import-has-open-exercises`: the imported module still has `sorry`s; their
  declarations never enter the environment, so downstream code cannot see
  those names (`docs/design/imports-and-projects.md` §4.5).
- `open-shadowed-name`: an `open`/`export` gave a short name a **second**
  candidate (two opens claim the same short name, or the short name collides
  with a root declaration). This language resolves by a fixed order and
  silently takes the first candidate, so the warning says *which* one wins
  (`docs/design/namespace-open.md` §N8). It is syntax-level and file-local: a
  candidate declared by an *imported* module is not visible to the warning
  pass, so that case is not reported.

## Error staging and codes

Errors carry a stable machine `code` and a stage. Codes are fine-grained so
that a model or editor can react to the *kind* of mistake, not the wording:

- `parse` stage — `unexpected-token`, `unexpected-eof`, plus the `import`
  shape errors `import-malformed` (missing module name, or more than one thing
  on the line), `import-not-a-valid-module-name` (a component is not an
  identifier — the classic case is a `-` from a file name) and
  `import-must-precede-declarations` (an `import` after a declaration);
  plus the notation family (G-04 / WO-011, `docs/design/notation-subset.md`):
  `unterminated-string` (a `"` with no closing quote in a notation command —
  the span points at the **opening** quote), `notation-shape` (a malformed
  notation command: missing precedence, missing `=>`, an identifier-word or
  empty symbol, out-of-range precedence, a `notation:N` spelling, or a
  duplicate symbol) and `notation-unknown-symbol` (a symbol used with no
  `infix`/`notation` declaration before it — the hint teaches both "declare it
  first" and the pointful spelling);
  plus the namespace family (G-05, `docs/design/namespace-open.md`):
  `parse-namespace-mismatch` (an `end <name>` whose name is not the nearest
  open `namespace`, or an `end` with nothing to close),
  `parse-namespace-unclosed` (the file ends inside a `namespace` — the span
  points back at that `namespace` line) and `parse-namespace-shape` (a
  `namespace`/`end`/`open`/`export` with a missing or malformed name; a bare
  `end` is this code too, because the name is required in this subset; since
  the second cut it also covers the `open`/`export` clauses — `open Foo ()`,
  `open Foo hiding` with no names, `renaming a b` without `=>`, a dotted name
  in a clause (clauses take **short** names), and an `open … in` body that is
  not a leaf command);
- `elab` stage — e.g. `elab-unknown-identifier`, `elab-unknown-constant`,
  `elab-unknown-universe-level`, `elab-universe-arity`, `elab-untyped-binder`,
  `elab-hole-misplaced`, `elab-duplicate-declaration`, `elab-too-many-binders`,
  `elab-nat-literal-disabled`, `elab-invalid-nat-literal`,
  `elab-too-many-ctor-fields`, `elab-unknown-ctor-for-iota`,
  `elab-ambiguous-ctor-alias` (裸构造子名被两个类型各声明了一次——G-02 之后
  构造子的**规范名**是 `Ind.ctor`，裸名只是解析别名（子集扩展），重复即歧义；
  消息点名两个候选，写全前缀名即可解决),
  `elab-tactic-failed` (`by` 块里的一个 tactic 失败：目标形状不匹配 /
  内核拒绝，消息带期望/实际), `elab-apply-needs-a-term` (apply 类 tactic
  后面缺少要应用的项), `elab-apply-not-applicable` (要应用的项的结论不是
  当前目标),   `elab-match-bad-arm` (`match` 的模式写错：带子模式的未知名、
  构造子字段数与子模式数不符), `elab-match-not-inductive` (`match` 的被匹配项
  不是已知归纳类型——本文件用 `inductive` 声明，或 prelude 内建的 `Nat`/`Bool`),
  `elab-match-no-expected-type`
  (`match` 的结果类型未知，无法定 motive 及其宇宙), 
  `elab-match-recursive-unsupported` (`match` 暂不支持递归归纳类型),
  `elab-match-non-exhaustive` (`match` 未覆盖全部构造子/字段位置，或带守卫的 arm
  没有兜底),
  `elab-match-parameterized-unsupported` (`match` 参数化归纳时，被匹配项不是
  一个书写类型为 `T 参数…` 的局部变量——拿不到参数实例),
  `elab-let-type-query-failed` (无类型标注的 `let` 无法从值推断绑定类型),
  `elab-notation-unknown-target` (记法命令 `=>` 后面的目标名不存在),
  `elab-notation-argument-unsolved` (记号展开时补不出目标 telescope 的
  **前导类型参数**：v1 只按操作数/期望类型做裸变量匹配，不做一般推断；
  hint 教点名写法。G-04 / WO-011),
  `elab-implicit-argument-unsolved` (**隐式实参**补不出来：签名有前导隐式
  binder（`{α : Type}` 这种），而路线 C 只按**后续显式实参的类型**反解、
  不搜索不回溯；hint 教把参数写全——点名/写全参数永远可用。IA-1
  `docs/design/implicit-arguments.md` §3),
  `elab-notation-ambiguous` / `elab-notation-no-candidate` (**记法重载**：
  同一符号多条记法按**期望类型**选候选——≥2 个候选都说得通时报前者，一个
  都对不上时报后者；消息列出候选与各自的结果类型，hint 教点名写法消歧。
  第三刀 `docs/design/notation-subset.md` §12.2),
  `elab-binder-notation-unsolved` (binder 记法 `∀ x ∈ s, p` / `∃ x ∈ s, p`
  里 x 的类型从 `∈` 两边反解不出来；hint 教补 `(x : α)` 或点名。§12.1),
  `elab-set-literal-unknown-target` (集合字面量 `{a}` / `{a, b}` 展开成
  `Set.singleton` / `Set.pair`，但本文件里没有它们；hint 教 `import` 或点名。
  §12.4);
- `kernel` stage — `kernel-rejected` (kernel said no; conversion failures
  carry the expected/actual sides), and the fine-grained families
  `kernel-expected-sort` (a term appeared where a type was required),
  `kernel-expected-pi` (a non-function value was used as a function, or an
  application was over-applied), `kernel-theorem-not-prop` (a theorem whose
  type is not a proposition — **也出现在开练习上**：签名不是 Prop 的
  `theorem … := sorry` 报这一条，而不是 `exercise.open`，见上文的边界),
  `kernel-prop-not-cumulative` (内核要 `Sort(n)`（`n ≥ 1`，数据/`Type`），
  给的却是 `Sort(0)`（`Prop`）：本语言**没有累积性**（官方 Lean 4 有
  `Prop ⊆ Type`）。L-06 的专用码 + 人话 hint；设计
  `docs/design/prop-cumulativity-boundary.md`),
  `kernel-inductive-non-positive` (a recursive
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
  bug; never a learner mistake);
- `import` stage — project-level resolution of `import Foo.Bar`
  (`docs/design/imports-and-projects.md`): `import-not-found` (no
  `<module root>/Foo/Bar.sokonanoda`; the message lists the path that was tried,
  and the hint carries "did you mean" clues for a case mismatch or a dashed
  file name), `import-cycle` (the message spells the cycle), and the three
  closure rules `import-dependency-failed` (an imported module did not compile,
  so its importers are not compiled either — the error sits on the `import`
  line and points at the dependency's first problem),
  `import-name-collision` (two modules declare the same top-level name) and
  `import-prelude-conflict` (the closure disagrees about the built-in
  prelude); `manifest-invalid` reports an unreadable or malformed
  `sokonanoda.toml`, and `import-module-invalid` reports an imported file that
  does not parse at all.

Human output prints `error[<stage>]: <message>` — the bracket carries the
**pipeline stage** (`parse`/`import`/`elab`/`kernel`), not the code; the stable
code and the teaching `hint` are JSON-only (`--json`). JSON diagnostics carry
`stage`, `code`, `message` and a `hint`. The LSP maps the same data onto
`publishDiagnostics` (message + hint, code, range) and `hover` (type map /
goal text); see `docs/design/infrastructure.md` F1–F8.

## Live session: `sokonanoda watch <file>` (implemented)

`watch` is the CLI form of the service layer: it monitors a file and prints one
JSON object per change:

```json
{"type":"service.hello","protocol":1,"engine":"0.28.0","pid":12345}
{"type":"file.didChange","file":"canvas.sokonanoda","version":3,"recompiled_from":0}
{"type":"exercise.solved","file":"canvas.sokonanoda","name":null,"version":3}
```

- The first JSON line is always the service handshake `service.hello`
  (mirroring LSP `soko/version`); everything after is versioned document state.
- `file.didChange` is the canonical opener (alias: `file.changed`, deprecated
  but accepted for one minor cycle — never emitted alongside it).
- `recompiled_from` is the index of the first changed command; `null` means
  the commands did not change (comment-only edit) and nothing was recompiled.
- Delta events (from `sokonanoda_front::session`): `exercise.opened`,
  `exercise.solved`, `exercise.failed`, `decl.checked`, `decl.failed` — each
  carries `name` (when named), the stable `file` path and `version`.
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
KEYWORD, TYPE (Sort / inductive), NUMBER, MACRO (`sorry`), FUNCTION
(def/theorem names and uses), VARIABLE (axioms, unresolved idents),
ENUM_MEMBER (constructors), PARAMETER (binders). Encoding is UTF-16 correct.

## Value-position keywords (removed)

The value position once accepted `funintro` (and before it `funapply`) teaching
keywords. Both were removed — `funapply` in 0.22.0, `funintro` in 0.27.0
(`docs/design/remove-funintro.md`). The value position now accepts only a plain
expression or a `by <tactic>; …` block. `funintro` is no longer a keyword: it
parses as an ordinary identifier and fails elaboration as an unknown name.
Declarations may still carry Lean-style binders
(`theorem t (a : Prop) (h : a) : a -> a := by …`): they desugar to a Forall type
plus a Lambda value, so `:= sorry` reports the codomain goal with the
declaration binders already in context.

## Half-expression goal state (0.23.0)

A value that the kernel rejects can still carry useful structure: hovering
`And.intro b a` against the goal `And b a` shows the inferred remaining goals
(`|- b`, `|- a`) instead of only the error. Computed **on hover only** (never
on the keystroke path) via `judge_infer`, whose results are cached (bounded
128-entry fingerprint cache — see `docs/LESSONS.md`).

## Tactic goal-state hover (0.27.0)

`textDocument/hover` on a `by` tactic (anywhere in its source span) shows the
goal state **entering** that tactic — every remaining goal with its
hypotheses, Lean-Infoview style:

```text
P : Prop
Q : Prop
⊢ And P Q
```

Data comes from the same per-tactic snapshot as `soko/stateAt` (`by_steps`),
with the same entry semantics: entering tactic *i* is the state after tactic
*i-1* (the root for the first tactic; multi-subgoal steps list every goal,
current first). No re-check and no text scan happen at hover time. The hover
range is the tactic's span. Editors need no extra work: it is ordinary hover.

## Custom LSP requests (goal view, I9)

Beyond standard LSP, the server answers five custom requests (tower-lsp
`custom_method`; clients opt in, servers don't advertise them in
capabilities):

### `soko/goals`

Request params: `{"textDocument": {"uri"}, "position"}` (position reserved).

**文档身份回显**（0.57.0）：响应带 `uri`（请求指向的文档）与 `version`（答的是哪一版）。
多文档下客户端据此丢弃"答的是另一份文档/更旧版本"的过期响应——服务端已经按请求 URI
聚焦（`Docs::focus_request`），回显是给客户端**自证**用的（VS Code 扩展会比对，
不匹配就当过期答案丢掉）。`soko/stateAt` 同样回显 `uri`（它的 `version` 早就有）。

Response:

```json
{"uri": "file:///…/playground.sokonanoda", "version": 7,
 "decls": [{
  "name": "and_swap", "kind": "theorem", "status": "open",
  "range": {"start": {...}, "end": {...}},
  "ty": "And a b -> And b a",
  "ty_runs": [{"text": "And", "kind": "axiom_use"}, {"text": " a b -> And b a"}],
  "goal": "And b a",
  "goals": ["And b a"],
  "binders": [{"name": "a", "ty": "Prop", "ty_runs": [{"text": "Prop", "kind": "sort"}]},
              {"name": "h", "ty": "And a b", "ty_runs": [...]}],
  "hole": {"start": {...}, "end": {...}},
  "holes": [{"start": {...}, "end": {...}, "id": "and_swap:0", "redundant": false}, ...],
  "sub_goals": [{"range": {"start": {...}, "end": {...}}, "ty": "b"}, ...]
}]}
```

- one entry per declaration (all statuses); `goal`/`binders`/`hole` are
  present for open exercises (`hole` is the exact `sorry` range);
- `ty` is the declaration's kernel-rendered type (signature) and `ty_runs` its
  semantic runs (same vocabulary as `goal_runs`, §`soko/stateAt`) — the Infoview
  declaration list renders `ty_runs` as a small coloured hint and uses `range`
  to jump to the declaration;
- `goals` lists **every** open goal after the last recorded tactic (the current
  goal first) for `by` declarations, or the single walked remaining goal for
  non-`by` open exercises; empty for non-open declarations. It is the
  declaration-level counterpart of `soko/stateAt`'s per-cursor `goals` — a
  multi-subgoal `apply` shows all of its sub-goals here, not just one;
- `holes` lists every `sorry` (multi-hole constructor/function spines
  included) as objects
  `{"range": {…}, "id": "<declName>:<index>", "redundant": false}` — the id
  is stable per (declaration, hole order) within a document version
  (anonymous examples use the `example@<line>` name form) and is the stable
  reference for external tools; `redundant: true` marks a **leftover** `sorry`
  (the answer already proves the goal, deleting that line is what makes the
  declaration check — `docs/design/redundant-sorry.md`), so an agent must say
  "delete this line", never "not yet solved"; `false` also covers "no verdict"
  (the kernel probe did not run or did not pass);
  `sub_goals` pairs each hole with its
  expected type (server-side walk: constructor parameter positions expect
  the goal's own argument, proof positions the instantiated field type;
  function argument holes expect the function binder's type instantiated at
  the preceding arguments, e.g. `Eq.subst.{1} Nat (sorry) …` → `Nat ->
  Prop`); `ty` is `null` when the walk cannot determine it (unknown
  template, or a preceding argument is itself a hole), but at request time
  the server may fill such a `null` with a kernel-driven probe (the Pi domain
  of the partial application), so clients must not assume `null` means
  "undeterminable";
- multi-hole documents are naturally supported (one entry per declaration);
- hover remains the degraded, human-readable view of the same data;
- kernel-judged code actions (next-step suggestions, per goal shape; see
  `docs/design/hints-suggestions.md`): at most three per request, the first
  one carries `is_preferred: true`:
  - `exact <hypothesis>` — a hypothesis the kernel judges defeq to the hole's
    expected type (per hole: in a constructor spine each sub-hole is judged
    against its own expected type, never against the outer goal);
  - `Eq.refl …` — for `Eq`-shaped goals, a `rfl` candidate that the kernel
    validated before it is offered (dropped when rejected);
  - `refine <skeleton>` — a constructor skeleton recovered from the
    declaration's own axiom/ctor shape (e.g. `And.intro a b sorry sorry`);
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

### `soko/stateAt`

Request params: `{"textDocument": {"uri"}, "position"}` (the caret). Response
(all fields present; `null` where noted):

```json
{
  "version": 5,
  "decl": {"name": "open", "kind": "theorem", "status": "open", "range": {}},
  "goal": "And a a -> a",
  "goal_runs": [{"text": "And", "kind": "axiom_use"}, {"text": " "},
                {"text": "a", "kind": "binder"}, {"text": " -> "},
                {"text": "a", "kind": "binder"}],
  "binders": [{"name": "a", "ty": "Prop",
               "ty_runs": [{"text": "Prop", "kind": "sort"}]}],
  "goals": [{"goal": "And a a -> a", "goal_runs": [...], "binders": [...]}],
  "span": {"start": {...}, "end": {...}},
  "step": 1,
  "total": 2
}
```

- `goal_runs` / `ty_runs` carry the **semantic runs** of `goal` / a
  hypothesis's `ty` (`docs/design/goal-rendering.md` §2.1): an ordered list of
  `{"text"}` fragments that concatenate back to the exact field text, each
  optionally carrying a `kind`. The vocabulary is the editor's own semantic
  classification, owned by `front::semantic::SemanticKind` and spelled by
  `SemanticKind::as_str()` (`keyword`, `sort`, `number`, `hole`, `def_name`,
  `def_use`, `theorem_name`, `theorem_use`, `axiom_name`, `axiom_use`,
  `inductive_name`, `inductive_use`, `ctor_name`, `ctor_use`, `binder`,
  `unknown_ident`); a run **without** `kind` is plain connector
  (whitespace/punctuation) and is drawn unstyled. Clients that colour goals
  (the Infoview) must use these runs and never re-tokenize the text — that is
  what keeps hover and the Infoview from drifting apart.
- Selects the goal state at the caret with Lean `goalsAt?` semantics: inside
  a tactic's source span → the state **entering** that tactic; otherwise the
  state after the last tactic that ended at or before the caret; before the
  first tactic → the root state (`step: -1`, `span` = the declaration's
  range, goal = the full kernel-rendered declared type).
- `goals` is the **full** remaining-goal list at that position (current goal
  first, `[]` = closed), each entry carrying its own `goal` text and
  `binders` — so a multi-subgoal tactic (`apply And.intro`) shows both
  sub-goals at once. The single-value `goal`/`binders` fields are kept for
  older clients and always equal `goals[0]` (`goal: null` when `goals` is
  empty).
- `step` indexes the declaration's per-tactic states (front `by_steps`,
  recorded after each tactic runs); `total` is the tactic count. For a
  declaration without a `by` block both are `-1`/`0` and the response falls
  back to the declaration's remaining goal/context (`step: -1`).
- `goal: null` = no remaining goals at that position (the proof is closed).
  `decl: null` = the caret is not inside any declaration; all other fields
  then default (`goal: null`, empty `binders`, `span: null`, `step: -1`,
  `total: 0`).
- The response carries the document `version` so clients drop stale answers.
  Selection is entirely server-side (clients never scan the source);
  `soko/goals` is unaffected.


### `soko/project`

Request params: `{"textDocument": {"uri"}}`. Response:

```json
{"uri": "<the requested document>", "version": 3,
 "project": {"entry": "Canvas", "root": "/abs/project", "manifest": "/abs/sokonanoda.toml",
             "requires_warning": null,
             "modules": [{"name": "Logic", "path": "/abs/Logic.sokonanoda",
                          "status": "compiled", "entry": false, "imports": [],
                          "decls": 5, "errors": 0, "warnings": 0, "open_exercises": 0,
                          "message": null}],
             "diagnostics": [{"code": "import-not-found", "message": "…", "module": "Canvas",
                              "severity": "error", "start": 7, "end": 14}],
             "counts": {"modules": 2, "compiled": 2, "failed": 0, "blocked": 0,
                        "decls": 7, "errors": 0, "warnings": 0, "open_exercises": 2}},
 "reason": null}
```

- The **project closure state** around the document: module root, which
  `sokonanoda.toml` (if any) is in effect, every module in topological order
  (dependencies first, entry last) with its status, the project-level
  diagnostics and the counts the editor/tree needs. Paths are **absolute**
  (`canonicalize`d when the path exists), so a client can open them directly.
  `root` is **never empty**: the entry path is made absolute *before* the
  ancestor-manifest ascent (the working directory is used exactly once, there),
  so whether a client spells the entry relatively or absolutely — and from
  which directory it runs — never changes the root or the module names.
- Each project diagnostic carries `severity` (`error`/`warning`) so a consumer
  can count failures without keeping a list of codes.
- `status` is `compiled` (it took part in the compile — the report may still
  contain errors), `load-failed` (its own load failed: missing file, parse
  error, `import` cycle) or `blocked` (it never compiled because an upstream
  module failed, or its result was discarded because a dependency's *compile*
  failed). `message` names the cause for the two failure states.
- `project: null` + `reason` is a **legal answer, not an error**: `no-imports`
  (a single file — the same code path as before projects existed),
  `no-path` (the document has `import` but no entry path: stdin / unsaved
  buffer without `--root`) or `parse-error` (fix the syntax first). The
  extension renders the first two as a one-line placeholder instead of an
  empty tree.
- Read-only derivation: the answer comes from the already-compiled closure
  (no recompile, no cache write, no digest). `uri`/`version` are echoed so a
  client can drop answers for another document (same discipline as
  `soko/goals`). Design: `docs/design/project-view.md`.
- Consumers: the VS Code **project tree** (`sokonanoda.project`) and its status
  bar tooltip; `sokonanoda query project` and the MCP `project` tool are the
  CLI/agent transports of the same view.

### `soko/version`

Request params: `{}`. Response: `{"version": "<CARGO_PKG_VERSION>",
"pid": <server pid>}`. Consumers: the extension's
`sokonanoda: restart server` command asks before and after a restart — the
receipt (`0.16.2 (pid 1001) → 0.19.0 (pid 2002)`) turns "old process died,
new process is the new version" into a verifiable fact.

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
  server-side walk, **aligned with `holes` by position** — several sub-goals
  may share one source position, so never look them up by span; the remaining
  goal for a lone main hole), markdown tooltip with the goal and introduced
  hypotheses. Checked/failed declarations produce no hints.

### `soko/nextHole`

Request params: `{"textDocument": {"uri"}, "position", "forward": true}`.
Response: `null` or the `range` of the next open hole after (or, with
`forward: false`, before) the cursor. The server owns hole-position logic
(ocaml-lsp lesson: clients should not re-derive positions).

**Known limitation (multi sub-goals at one position).** When a `by` block
leaves several open sub-goals, they share a single source position: `assemble`
hands every leaf hole the same `hole_span` (`crates/front/src/by.rs`), because
those premises genuinely have no source text of their own. Consequences:

- `soko/nextHole` cannot step **between** such sub-goals — they are the same
  offset, so the search either returns the current position or skips the whole
  group. Navigation is group-wise, not goal-wise;
- the stable identity for programmatic consumers is `soko/goals`'
  `holes[i].id` (unique by index), **not** the range;
- inlay hints do show each sub-goal's own type (they align by position
  index), so the information is visible even though it is not addressable;
- the full open-goal list **is** addressable as data: `soko/stateAt`'s
  `goals[]` and `soko/goals`' `goals[]` list every goal (current first),
  so the gap is navigation only, not visibility (docs/design/goal-list.md).

Not a regression to fix by inventing positions: fabricating distinct offsets
would produce bogus ranges for `documentHighlight` / `selectionRange`.

Kernel-judged tactics: `exact` code actions are computed by `front::judge`
(a synthesized complete declaration checked by the full kernel) — no text
matching anywhere in the editor path.

## Watch stream (L1 CLI form)

`sokonanoda watch` emits one JSON object per line. The **first line is always
the service handshake**:

- `service.hello` (`{type, protocol, engine, pid}`) — `protocol` is the stream
  protocol version (currently `1`), `engine` is the binary's
  `<CARGO_PKG_VERSION>`, `pid` is the process id. Clients use it to check
  compatibility and to detect a restart (it mirrors LSP `soko/version`).

After the handshake, each new document version opens with `file.didChange`, then
the versioned delta of declaration and exercise state transitions, then the
current diagnostics. The delta vocabulary is closed:

- `file.didChange` (`{type, file, version, recompiled_from}`) — canonical
  opener; `file.changed` is a **deprecated alias accepted for one minor cycle**
  (never emitted together with the canonical name).
- `decl.checked` / `decl.failed` (`{type, file, name, version}`)
- `exercise.opened` / `exercise.solved` / `exercise.failed` (`{type, file, name, version}`)
- `diagnostic` (`{type, file, stage, code, message, hint, span, version}`)

`recompiled_from` is the index of the first re-checked command (`null` when
nothing changed): with the I8 incremental session, editing command `i` only
kernel-rechecks commands `i..n`. `SessionUpdate.stats.kernel_checks` (front
API) exposes the same fact as a count.

### Scope: `--doc` and `--workspace`

`watch` accepts the document as a positional path, as `--doc <path>`, or
monitors a whole tree via `--workspace <root>`:

- `sokonanoda watch <file>` / `sokonanoda watch --doc <file>` — one `Session`
  for that document; the device is the same as the positional form.
- `sokonanoda watch --workspace <root>` — recursively monitors every
  `*.sokonanoda` under `<root>` (skipping `target`, `.git` and `node_modules`),
  one `Session` per file.

Every event for a file carries a stable `file` field (the path used to discover
the document). **Version counters are per file** and there is **no global
ordering across files** — a client must aggregate by `file` and gate on that
file's `version`.

### Backpressure and resync

Each file has a bounded pending-event buffer. If buffered events for a file
exceed that bound, the service **coalesces** to the latest version and emits a
`file.didChange` opener with `recompiled_from: 0`. Clients **must treat
`recompiled_from: 0` as a full resync**: the incremental delta for that version
may have been dropped, so re-read the document (e.g. one batch `--json` run)
instead of assuming a contiguous event history.

### Client → service commands (stdin)

While polling, `watch` also reads **JSON Lines commands from stdin** (one object
per line) on a background thread, so the poll loop never blocks. Responses are
written to the same stdout event stream:

- `ping` (`{type, id?}`) → `pong` (`{type, id, protocol, engine}`): `id` is
  echoed as-is (`null` when omitted); `protocol`/`engine` mirror `service.hello`.
- `subscribe` / `unsubscribe` (`{type, file}`): select which files emit events.
  With no command every watched file emits (backward compatible). The first
  `subscribe` switches to an allowlist — only subscribed files emit, so send one
  `subscribe` per file; `unsubscribe` removes a file. `unsubscribe` before any
  `subscribe` starts from all currently-known files. `file` is the same stable
  path carried by that file's events.
- Unknown, malformed or non-JSON lines → `error` (`{type, message}`); the
  command has no other effect and the stream keeps running.
- EOF on stdin is not fatal: the command channel closes and watching continues
  (this is the default when stdin is `/dev/null`).

## Course map: `sokonanoda course <path>... [--all]`

Aggregates the units of one or more course manifests (the agent-facing material
library) into a progress map; the VS Code course tree consumes it. Each
positional is a manifest file or a **directory holding `course.json`**; `--all`
walks each directory **recursively** for every `course.json` (sorted by path;
hidden directories, `target/` and `node_modules/` are skipped), and `--all`
without a positional means `.`. Unit paths resolve relative to their manifest's
directory (the manifest path is canonicalized first, so `course` does not depend
on the cwd).

**Several manifests at once** (additive; design
`docs/design/course-manifest-v2.md` §4.5 — the §7 "不做" item):

* every manifest is read and parsed **before any event is emitted**: one
  unreadable or refused manifest fails the run with **no events** (a batch never
  prints half a map), and `--all` finding no manifest is an error too — an empty
  map would look like a green course;
* `course.unit` gains `manifest` (the path as built from the arguments) **only
  when more than one manifest is aggregated**; a single-manifest run keeps the
  frozen unit shape byte for byte;
* `course.summary` gains `manifests` (the number of manifests aggregated —
  always present, `1` for the classic single-manifest invocation);
* the same manifest given twice (file + directory, or two spellings of one path)
  is reported **once**; the first spelling wins. No new event type: consumers
  group by the `manifest` field.

**Two manifest shapes, both read** (ledger G-07; design
`docs/design/course-manifest-v2.md`):

* **v1** — a flat JSON array `[{file, title, title_en, unit}, …]`
  (`course/course.json`, `scripts/new-course-repo.sh` skeletons);
* **v2** — an object `{schema: "soko.course/2", name, title,
  volumes[].chapters[].units[]}` (`courses/set-theory/course.json`), where the
  `units[]` entries keep the v1 shape **verbatim** and each chapter adds
  `prereqs` (chapter ids), `tags` and `quota.exercises` (the **planned**
  exercise count — metadata, never a grading criterion).

A JSON object whose `schema` is present and not `soko.course/2` is refused
(exit non-zero, no events); an object without `schema` is read as v2. The
**v1 event shape is frozen**: a v1 manifest's events carry no v2 key at all.

A unit that declares `import` is compiled through the **project closure** —
the same closure, module root and `ProjectPlan::digest` cache key as
`grade`/`query check`/`build`. Its module root is the nearest
`sokonanoda.toml` above the unit (a unit inside a nested sub-project keeps
its own manifest), falling back to the **directory holding `course.json`**
so the usual `<course>/{course.json,lib/,units/}` layout resolves
`import lib.Set`. A unit without `import` keeps the single-file pipeline.

- `course.unit` (`{type, file, title, unit, checked, open, failed, reduced[,
  error]}`) — per-unit counts (`checked` = `decl.checked`, `open` = open
  exercises, `reduced` = `expr.reduced`) taken from the **entry module
  only** (a dependency's declarations are not part of the unit's score;
  `example.checked` stays out of `checked` as before); `failed` counts the
  entry's rejections **plus** the closure-level ones, so `failed == 0` ⇔
  `grade <unit>` exits 0; `error` carries a message when the unit file
  could not be read or parsed;
  **v2 additions** (present only for a v2 manifest): `volume`
  (`{id, title}`), `chapter` (`{id, title, tags}`) and `tags` (a flat copy
  of the chapter's tags, so a tag filter need not descend);
  **aggregation addition** (present only when >1 manifest is aggregated):
  `manifest` (the manifest path);
- `course.summary` (`{type, units, checked, open, failed, volumes,
  chapters, manifests}`) — totals; `volumes`/`chapters` are structure counts
  and `manifests` is the number of aggregated manifests — all three always
  present, `volumes`/`chapters` being `0` and `manifests` `1` for a classic
  single-manifest invocation.

Human view: one line per unit (`unit 1 命题与证明 —— 12 checked · 5 open ·
0 failed`; a v2 unit appends `（卷 … / 章 …）`) plus a totals line; aggregating
several manifests prints a `── <manifest> ──` header per manifest and
`共 2 份清单 · 23 单元 …` in the totals. Exit code
is 0 even with open/failed exercises (progress is not an error); only an
unreadable manifest fails.

## Build cache: `sokonanoda build`

`sokonanoda build [--json] [--clean] [<file> | <dir> ...]` warms (or clears)
the shared persistent compile cache that `check`/`course`/LSP reads
(`docs/design/compile-cache.md`). Each positional is a `.sokonanoda` file or a
directory walked recursively (sorted) for `*.sokonanoda`; no positional means
the current directory. A build is a hit when the same `(compiler version,
build stamp, prelude mode, source text)` already produced a kernel report.
Warming is best-effort and never changes the kernel's verdict; a read or parse
failure counts as `failed` but does not abort the batch. Exit is 0 whenever at
least one file resolved (nothing resolved is usage, exit non-zero).

- `build --clean` removes every cached entry and prints `removed N cached
  file(s)`;
- `--json` emits one `build.file` per resolved file
  (`{type, file, status}` with `status` ∈ `hit` / `compiled` / `failed`),
  then `build.summary` (`{type, files, hit, compiled, failed}`); `build --clean
  --json` emits `build.clean` (`{type, removed}`);
- the human summary is `built K file(s) — H hit, M compiled, F failed`.

Environment: `SOKONANODA_CACHE_DIR` relocates the cache root, and
`SOKONANODA_NO_CACHE=1` disables it (loads always miss, stores are skipped).

## REPL history

`sokonanoda repl` appends every non-empty input line to
`$HOME/.sokonanoda_history` (created on demand; silently disabled when
`HOME` is unset, truncated to the most recent 1000 lines on load).
Line-editing / arrow-key recall is out of scope.

## Future structured event names (service layer)

The service stream vocabulary is now implemented (see "Watch stream" above).
Future additions:

- `decl.rejected` (today: a `diagnostic` with stage `kernel`; if added, it is
  an alias of `decl.failed` — the two are never emitted together)
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

## `query` subcommand: kernel truth as one JSON object

`--json` emits the **full event stream**; `query` emits the **same truth** as a
single JSON object (counts, addressable holes, the goal state at a caret) so an
agent or script parses once instead of reassembling state from lines. Both come
from the same `front::session` compile, and a contract test asserts their
counts agree.

```
sokonanoda query <op> [options]
```

| op | needs | answer (`data`) |
|---|---|---|
| `check` | — | `{version, counts{decl_checked,example_checked,exercise_open,expr_typed,expr_reduced,decl_printed}, failed[{name,code,message,start,end,start_line,start_col,end_line,end_col}], warnings[{code,message,hint,start,end,start_line,start_col,end_line,end_col}]}`. **Two coordinate systems per entry, both spelled out** (G-15 / WO-010): `start`/`end` are **byte** offsets into the **entry file**, and `start_line`/`start_col`/`end_line`/`end_col` are the same 1-based line/column pair the `--json` event `span` prints — no counting on the consumer side (measuring a byte offset as a *char* index is what produced the G-15 false gap). Dependencies live in their own coordinate space: a broken module surfaces **only** as `import-dependency-failed` on the entry's `import` line (use `grade --json`, whose diagnostic carries `file`/`module`, or `query project` for the dependency's own error). `failed[]` carries **both** kernel rejections and parse diagnostics: when the source text does not parse, `counts` stays all-zero (nothing was checked), `failed[]` has exactly the parse diagnostic (`code` one of `unexpected-token`/`unexpected-eof`/`import-malformed`/`import-not-a-valid-module-name`/`import-must-precede-declarations`, `name: null`), `ok` stays `true` (the query *was* answered) and the exit code is `1` — the same verdict `grade` gives |
| `state` | `--line L --col C` or `--offset N` | `{version, decl{name,kind,status,start,end}\|null, goal, goal_runs, binders, goals[{goal,goal_runs,binders}], span[start,end]\|null, step, total}` |
| `goals` | — (`--probe` runs the kernel probe) | `[{name,kind,status,start,end,ty,ty_runs,goal,goals,binders,hole[start,end]\|null,holes[{start,end,id,redundant}],sub_goals[{start,end,ty}],code_actions}]`; if the source text does not parse the whole answer is the failure envelope `ok:false` + `error.code: "not-parsable"` (exit `1`) — an empty array would read as "this canvas has no declarations" |
| `holes` | — (optional `--offset N --direction next\|prev`) | `{holes[{id,start,end,ty,decl,redundant}], navigated<hole>\|null}`; on a parse failure the same `not-parsable` failure envelope as `goals` (never `holes: []` + `navigated: null`) |
| `hints` | `--line L --col C` or `--offset N` | `{hints[string]}` |
| `reduce` | `--expr E` | `{value, ty}` |
| `project` | — (optional `--root <dir>`) | `{project: <ProjectView>\|null, reason: "no-imports"\|"no-path"\|"parse-error"\|null}` — the closure around this file: root, manifest, modules with status (`compiled`/`load-failed`/`blocked`) + imports + counts, project diagnostics. `project: null` is a legal answer (a single file), never an error. Same view as `soko/project` (`docs/design/project-view.md`) |

Input: `--file <path>`, `--text <src>`, or stdin (a bare `-` also means stdin).
`--compact` prints one line instead of pretty JSON.

Envelope (every answer, success or failure):

```json
{"schema": "soko.query/1", "op": "state", "version": 1, "ok": true, "data": {…}}
{"schema": "soko.query/1", "op": "state", "version": 1, "ok": false,
 "error": {"code": "outside-declarations", "message": "该位置不在任何声明内部"}}
```

- **`ok:false` is not "empty"**: no remaining goal (`goal: null`) and "no next
  hole" (`navigated: null`) are *successful* answers. `error.code` is one of
  `not-parsable` / `outside-declarations` / `position-out-of-range`. Mixing the
  two is what forced agents to re-derive state from text before. A source text
  that does not parse is **never** answered with an empty result: `check` reports
  the parse diagnostic in `failed[]`, `goals`/`holes` answer `not-parsable`.
- **Exit codes**: `0` = answered (an open `sorry` exercise is a legal state),
  `1` = the file was rejected — kernel-rejected declarations, a **parse failure**
  (its `check.failed[]` carries the parse diagnostic; `goals`/`holes` answer
  `not-parsable`), or `reduce` failed — `2` = usage error. **Decide on the JSON,
  not the exit code.**
- Positions are 1-based `line`/`col`; `--offset` is a **byte** offset. The column
  counts `char`s today (`crates/front/src/token.rs`) — identical to the LSP's
  UTF-16 `character` for BMP text (all course material), and one short per astral
  character (emoji); that drift is a separate, separately tracked gap, not
  something a consumer of this protocol should compensate for. Every position in
  this document (event `span`, `query check`'s `start_line`/`start_col`/…, LSP
  diagnostics) is the **same** number, so read it, don't recompute it.
- `query check`'s `failed[]`/`warnings[]` report in the **entry file**'s
  coordinate space. A dependency module's own error is not in `failed[]`; it
  appears there only as `import-dependency-failed` (on the entry's `import`
  line). For the dependency's error use `grade --json` (its diagnostic carries
  `file`/`module`) or `query project` (per-module `errors`).
- `holes[].id` (`<declName>:<index>`)
  is the stable reference for programmatic consumers — two sub-goals of one
  `apply` share a source position, so positional navigation steps over them as a
  group (see the `soko/nextHole` limitation above).
- Fields are **additive only**; renaming one is a breaking change (minor bump +
  docs + tests). `soko.query/1` is the schema tag.
