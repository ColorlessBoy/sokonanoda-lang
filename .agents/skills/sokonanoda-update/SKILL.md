---
name: sokonanoda-update
description: Refresh the pinned sokonanoda CLI + LSP cache to this checkout's version (zero cargo).
whenToUse: 环境过期时（doctor 报 ready:false，或启动器 exit 3 拒绝运行）；也用于 update 报了 cache NOT refreshed 时定位原因
disable-model-invocation: true
user-invocable: true
---

# sokonanoda-update (DeepSeek Harness entry)

The complete operating manual for this command is the repository file
`skills/sokonanoda-update/SKILL.md` (relative to the sokonanoda-lang
repository root — resolve it against the repository root, e.g. the `.git` root
of the workspace, not against this entry's directory).

Read it **before** acting, then follow it exactly. It is the single source of
truth for:

- when the command is needed (a stale cache makes `scripts/soko` refuse to run
  with exit 3, or a file suddenly reports syntax errors a newer compiler
  accepts);
- the exact commands (`scripts/soko update`, then `scripts/soko version --json`);
- what `update` actually promises — a written cache. Exit `0` = refreshed;
  **exit `3` + `cache NOT refreshed` on stderr = it did not write the cache**,
  even when a usable fallback (`repo-build` / `override`) is reported on stdout;
- the only trustworthy acceptance evidence: the cache `marker`, the cached
  binary's own `--version`, and `doctor` reporting `ready: true`. A `source`
  without `STALE` merely means *some* binary was found — a repository build
  shadows the cache whenever one exists, so it proves nothing about a refresh;
- the failure actually hit in practice — `download: EPERM … copyfile … ->
  …/sokonanoda/bin/sokonanoda`: download and extraction fine, the cache write
  denied (read-only cache, full disk, or a sandboxed process) — and its
  sanctioned remedies;
- the rules that never bend: version-pinned downloads only, never
  `releases/latest`, never require the user to install Rust/cargo.

Report the measured values (exit code, `version`, `target`, `cache`, both
`source` values, cache `marker`) — and paste the `download:` line when it exits 3.

> This entry is **human-invoked**: `user-invocable: true` with
> `disable-model-invocation: true`, so it stays out of the model catalog and
> exists purely so a person can type `/sokonanoda-update`. The model reaches
> the same capability through `AGENTS.md` and the role skills.
