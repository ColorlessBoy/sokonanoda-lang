# `.sokonanoda` collaboration protocol (draft)

## Goal

The same compiler feedback is consumed by:

- a human editing a `*.sokonanoda` file in an editor;
- a code agent writing definitions and exercises in that file;
- the VS Code layer (later), which renders one view per side.

## Canonical lines (machine- and human-readable)

The CLI already emits one event per line. These are the canonical forms.

```text
checked declaration <name>
checked example
<expr>: <type>
<expr> => <value>
exercise open (fill the ???)
<line>:<col>: error: <message>
```

Every `<expr>`/`<value>` is the exact source slice of the checked expression,
so a model can re-run or display it without re-parsing.

## Structured event names (future JSON)

When a service layer is added, keep the same vocabulary:

- `file.didChange`
- `decl.added`
- `decl.checked`
- `decl.rejected`
- `expr.typed`
- `expr.reduced`
- `exercise.open`
- `exercise.solved`
- `diagnostic.*`

Each event carries `span {offset,line,column}` plus a human text and a machine
payload when relevant.

## Agent contract

An agent can drive the tool by:

1. appending declarations and exercises to the current buffer;
2. recompiling and reading only the new event lines;
3. matching `checked declaration X`, `<expr>: <type>` and `error:` lines;
4. using `#env` to list loaded declarations when starting a lesson.

## Exercise payload

`exercise open` may be extended to carry the expected kind:

```text
exercise open (fill the ???): <goal-type>
```

No editor-specific API is part of this protocol; the protocol is text/events
only, so VS Code, CLI, tests and agents can share it.
