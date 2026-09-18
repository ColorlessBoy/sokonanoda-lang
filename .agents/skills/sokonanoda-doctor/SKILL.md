---
name: sokonanoda-doctor
description: Diagnose whether the pinned sokonanoda CLI + LSP are ready, and name the next command when they are not.
whenToUse: 会话开始、启动器拒绝运行、或判卷失败但文件看起来没问题时
disable-model-invocation: true
user-invocable: true
---

# sokonanoda-doctor (DeepSeek Harness entry)

The complete operating manual for this command is the repository file
`skills/sokonanoda-doctor/SKILL.md` (relative to the sokonanoda-lang
repository root — resolve it against the repository root, e.g. the `.git` root
of the workspace, not against this entry's directory).

Read it **before** acting, then follow it exactly. It is the single source of
truth for:

- the read-only command (`scripts/soko doctor --json`, plus the exit code);
- the exit-code meanings (`0` ready, `3` not ready, anything else is a launcher
  or usage error to quote verbatim);
- the fields worth reporting (`ready`, `version`, `target`, `cache`, and the
  `cli` / `lsp` blocks with `path` / `present` / `ready` / `marker`);
- the rule that `marker` must equal the `Cargo.toml` version, and that a
  mismatched marker means the cache is stale;
- the discipline: report first, never silently `setup` / `update` / wipe the
  cache, and never ask the user to install Rust/cargo.

On DeepSeek Harness the language server delivers no diagnostics to the agent,
so grading always goes through the CLI.

> This entry is **human-invoked**: `user-invocable: true` with
> `disable-model-invocation: true`, so it stays out of the model catalog and
> exists purely so a person can type `/sokonanoda-doctor`.
