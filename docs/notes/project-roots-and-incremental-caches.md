# Project roots & incremental caches: verified survey

Primary sources only (fetched raw; `web_search` had no API key); identifiers are verbatim;
"uncertain" = not verified in this pass.

## A1. LSP: what the client actually promises

- `InitializeParams.rootUri: DocumentUri | null` — "Is null if no folder is open. If both `rootPath`
  and `rootUri` are set `rootUri` wins."; deprecated in favour of `workspaceFolders` (same in 3.17
  and 3.18). `workspaceFolders?: WorkspaceFolder[] | null` — "only available if the client supports
  workspace folders. It can be `null` … but none are configured."
- `workspace/workspaceFolders`: "Returns `null` in the response if only a single file is open in the
  tool. Returns an empty array if a workspace is open but no folders are configured." ⇒ **a null root
  is normal, not an error**, and no single-file-mode flag exists: the server degrades itself.
  Register via server capability `workspace.workspaceFolders.supported` +
  `changeNotifications?: string | boolean`; changes arrive as `workspace/didChangeWorkspaceFolders`
  (`DidChangeWorkspaceFoldersParams{event:{added, removed}}`).
- `workspace/didChangeWatchedFiles`: `FileEvent{uri, type}` (`FileChangeType` Created/Changed/Deleted)
  under `DidChangeWatchedFilesRegistrationOptions{watchers: FileSystemWatcher[]}`; 3.17 added glob
  syntax, `RelativePattern`, client cap `relativePatternSupport`.
- Diagnostics are server-owned and server-cleared; "There is no merging … on the client side"; "When a
  project is opened all diagnostics for all files are recomputed (**or read from a cache**)".

## A2. rust-analyzer

- `ProjectManifest::discover`: `rust-project.json` → `.rust-project.json` → `Cargo.toml`, each via
  `find_in_parent_dirs` ⇒ to the filesystem root; with no `Cargo.toml` above, it looks one level
  **down** ("Only one level down to avoid cycles the easy way and stop a runaway scan with large
  projects").
- Project = "discovered by running `cargo metadata` and `rustc --print sysroot`"; failures retried
  with `--no-deps`. `ProjectWorkspaceKind::DetachedFile` = "disjoint files, not belonging to any
  particular workspace. Backed by basic sysroot crates"; source FIXME: "the set of detached files
  needs to be fixed at the beginning".
- Override: `rust-analyzer.linkedProjects` — "Disable project auto-discovery in favor of explicitly
  specified set of projects" (`Cargo.toml`, `rust-project.json`, `.rs`, or inline JSON objects).

## A3. clangd

- "clangd searches for `compile_commands.json` in parents of the source file and also in
  subdirectories named `build/`" → "if editing `$SRC/gui/window.cpp`, we search in `$SRC/gui/`,
  `$SRC/gui/build/`, `$SRC/`, `$SRC/build/`, …" (all the way up). Three names per directory:
  `compile_commands.json`, `build/compile_commands.json`, `compile_flags.txt`.
- `compile_flags.txt`: one flag per line in the source root; "Clangd will assume the compile command
  is `clang $FLAGS some_file.cc`"; "ignored if `compile_commands.json` is present";
  "background-indexing will not work, as clangd can't be sure which of the files are project
  sources." Absent ⇒ ≈ `clang $FILENAME`, hence "spurious errors about missing `#include`d files".
- Overrides: `--compile-commands-dir=<path>` ("If path is invalid, clangd will look in the current
  directory and parent paths of each source file"); config `CompilationDatabase: Ancestors`
  (default) | `None` ("do not use a compilation database, just default flags") | a path.
- "If this file has changed, you'll need to restart clangd"; `--background-index` (default true)
  persists the index on disk; `Index.Background: Build|Skip`.

## A4. TypeScript / tsserver (branch `strada`; `main` is now the Go port)

- `Session.openClientFile` → `ProjectService.openClientFileWithNormalizedPath` →
  `assignProjectToOpenedScriptInfo`; an orphan file gets an `InferredProject`.
  `forEachConfigFileLocation` tries `tsconfig.json` then `jsconfig.json` per directory, then the
  parent — but "If we started within node_modules, don't look outside node_modules … we might pick up
  a very large project and pull in the world, causing an editor delay." (`findConfigFile` walks to the
  filesystem root.) `--project`/`-p`; **no `--config` flag**; project lists are
  `references: [{ path }]`.
- Inferred project (wiki): for "a loose TS/JS file" with no config "**in the current directory or any
  parent directories**", imports are pulled in transitively. Problems: "configured projects >
  external projects > inferred projects"; "one file can belong to multiple projects of the same kind
  at the same time"; workaround `--useSingleInferredProject`; 20 MB cap
  `maxProgramSizeForNonTsFiles` (opt-out `disableSizeLimit`).
- `.tsbuildinfo`: `BuildInfo{version}` plus `fileNames, options, semanticDiagnosticsPerFile,
  fileInfos, affectedFilesPendingEmit, errors`; per file `FileInfo{version, signature,
  affectsGlobalScope}`, where `version` is a **content hash**
  (`getSourceFileVersionAsHashFromText` → `host.createHash || generateDjb2Hash`; Node = SHA-256).
  mtime also gates it (stale buildinfo ⇒ compare `version` with `currentVersion`; compiler mismatch ⇒
  `TsVersionOutputOfDate`). Key flags: `incremental` (-i), `tsBuildInfoFile`, `composite`, `force`
  (-f), `clean`, `build` (-b).

## A5. Marker-file servers (all walk to the filesystem root)

- **Pyright** — `pyrightconfig.json`, else `pyproject.toml` **with `[tool.pyright]`** (`setup.py` is
  not a marker). The CLI walks up; the **server does not** (`fromLanguageServer` guard). Override
  `-p/--project`.
- **gopls** — `go.work`, then `go.mod`; precedence `GoWork > GoMod > GOPATH > GoPackages > AdHoc`;
  no module ⇒ `AdHocView` ("we have no better choice than an ad-hoc view"). Override `GOWORK`
  (`GOWORK=off`), client `workspaceFolders`; `-remote` is *not* a root override.
- **ocaml-lsp** — `dune-project`, `dune-workspace` (not `.merlin`); fallback merlin external config;
  override `OCAMLLSP_PROJECT_ROOT` (source-only). **Agda** — any `*.agda-lib`, with the root derived
  from the **module name** (`root/A/B/C.agda`); fallback `AGDA_DIR/defaults`; override `-l/--library`,
  `--library-file`, `--no-libraries`.
- **Lean 4 VS Code** — `lean-toolchain`, core-Lean4 dir, `lakefile.lean`, `lakefile.toml`
  (`lake-manifest.json` is not a marker); fallback = workspace folder, else the file's dir; no editor
  override (Lake CLI: `-d/--dir`, `-f/--file`). HLS uses a hie-bios cradle (`hie.yaml`).

## A6. Wrong root: symptoms → mitigations

- clangd: "Lots of error diagnostics … Can't find includes within your project"; a CDB change needs a
  restart; a background-index crash "can take down the whole language server".
- rust-analyzer: many "'nothing works' problems [are because] rust-analyzer fails to understand the
  project structure"; detached files see only sysroot crates, frozen at startup.
- pyright: a stray config silently wins — "any pyright settings in a VS Code settings.json will be
  ignored". Two roots publishing one URI: no dedup ("no merging … on the client side"), last wins.
- rust-analyzer and Cargo "compete over the build lock" → "unnecessary recompilations caused by cache
  thrashing" (remedy `rust-analyzer.cargo.targetDir`). Mitigations: pin the root (`linkedProjects`,
  `--compile-commands-dir`/`CompilationDatabase`, `-p/--project`); one root per workspace folder;
  restart after marker changes; ignore artifact dirs.

## B7. What a dependent is rechecked against

- **Lean/Lake (content hash + mtime).** `BuildTrace{caption, inputs, hash, mtime}` with
  `Hash = UInt64` content hash (CRLF-normalised) and `mixHash` chaining; the JSON trace beside each
  artifact (module-path keyed under `.lake/build/lib`) carries `schemaVersion` and `depHash`, compared
  on rebuild; without a trace Lake falls back to "a newer modification time then
  `depTrace`/`oldTrace`". Traces mix source hash+mtime, every import's artifact trace, `-D` options,
  module name, package id and `leanArgs`; a hit needs `hash == self.hash <&&> checkExists info`;
  `weakLeanArgs` "do not affect the trace".
- **Lean's own `.olean` check is weak**: the header stores magic, version (2|3), `flags`,
  `lean_version[33]` and `githash[40]`, but the githash comparison is compiled out unless built with
  `-D LEAN_CHECK_OLEAN_VERSION` (CMake option default `OFF`). Rebuild correctness comes from Lake;
  `--fresh` does not exist (`--incr-save`/`--incr-load` are experimental).
- **GHC**: each `.hi` keeps "a list of the fingerprints of everything it used when it last compiled
  the file" (MD5, per interface file *and* per declaration). Reuse needs the source MD5 unchanged
  **and** `.o` mtime ≥ `.hi` mtime; else fingerprints are compared and compilation stops early
  (`-fforce-recomp` disables the check). Why mtime alone fails: for `C→B→A`, `C.hi` lists only
  `B.hi`, yet a change in `A` may not alter `B.hi` "one jot".
- **OCaml**: `.cmi` = `Config.cmi_magic_number` (mismatch → `Not_an_interface` /
  `Wrong_version_interface`), `(cmi_name, cmi_sign)`, `cmi_crcs`, `cmi_flags`; `output_cmi` prepends
  `(cmi_name, Some (Digest.BLAKE128.file filename))` — consistency by **interface digest**, not mtime.
- **Coq**: `.vo` carries two digests — one for "inconsistent assumptions" across libraries, one at the
  end for whole-file integrity; imports use `Safe_typing.digest_match ~actual ~required`, and each
  library stores `library_deps : (compilation_unit_name * Safe_typing.vodigest) array`.
- rustc incremental queries and dune digests: **uncertain**.

## B8. Where artifacts live

- Beside source (Coq `.vo`, OCaml `.cmi`, GHC `.hi` unless `-hidir`/`-ohi`/`-outputdir`): no
  discovery, but read-only trees break and configurations collide.
- Central build dir (Lake `.lake/`, oleans in `.lake/build/lib`, `lake clean` deletes `build`; Cargo
  `target/`): survives read-only sources, module-path keyed — but needs `.gitignore` and is shared
  state (lock contention above). Lake's remote cache (`lake cache get|add|clean`) yields read-only
  artifacts ("the old artifact must be deleted first").
- User cache dir (clangd's index; this repo's cache): no repo noise, but NFS/permission hazards and
  concurrent writers.

## B9. Pitfalls and escape hatches

- mtime is a hint, not a proof (clock skew, granularity, restored checkouts): Lake uses it only when
  the trace is absent (`--old` forces it); GHC keeps it as one of three conditions.
- 64-bit hashes are known-weak: Lake's `Hash` is `UInt64` with the in-source TODO "Use a secure hash
  rather than the builtin Lean hash function" (this repo uses FNV-1a 64). For a checker, a collision
  is a soundness hole.
- Atomicity: Lean writes `.olean` to `<name>.tmp.<pid>` then `std::rename` "so that we neither expose
  partially-written files nor modify possibly memory-mapped files"; this repo does the same for
  `<key>.json`. Coq: "Security weakness: file might have been changed on disk between writing the
  content and computing the checksum…".
- Concurrency: "Lake does not currently use a lock file. Previously… removed in lean4#2445", so two
  `lake build`s can race; Cargo's lock blocks rust-analyzer. Escape hatches to copy: `--no-cache`,
  `--try-cache`, `--rehash` (`trustHash := false`), `--force`, `--no-build`, `lake clean`,
  clangd `--background-index=0`.

## B10. Is caching judged results safe?

Yes, under three rules, each with precedent: (1) **kernel-produced only** — an entry comes from a
real run and a hit only skips work (LSP's "or read from a cache"; Lake replays cached outputs and log
only on a trace match); (2) **key = everything that can change the verdict** — compiler/kernel build
identity, report-schema version, source bytes, **interface hashes of every imported module**,
semantic options (GHC's "fingerprints of everything it used"; Coq's per-dependency `vodigest` with
refusal on mismatch; Lake's import traces + options + module identity); (3) **validate the artifact**
— Coq's `digest_match` plus end-of-file digest is the model, Lean's default-off githash check the
anti-pattern; re-check existence and re-emit cached diagnostics through the same constructor as a
fresh run. This repo's cache already matches (key = FNV-1a 64 over `format | version | bare? | text`,
`<cache_root>/compiled/<key>.json`, temp+rename, `SOKONANODA_CACHE_DIR`/`SOKONANODA_NO_CACHE=1`,
byte-identical cold/warm output by test) but has **no import dimension** — the gap once multi-file
modules land.

## Recommended shape for a tiny teaching toolchain

1. **Single file by default**: without a marker or `--root`, check exactly that file; never walk
   silently; a null LSP root means this mode.
2. **One ancestor marker** (`soko.toml`) stopping at the nearest `.git` or workspace-folder boundary —
   never `/` or `$HOME`.
3. **Explicit `--root <dir>`** on CLI and LSP launch, highest precedence, mirrored by an editor
   setting so CLI and server cannot disagree.
4. **Artifacts in a build dir** (`<root>/.soko/build/…`, module-path keyed) plus an optional user
   cache dir, never beside sources; ship `.gitignore` and `soko clean`.
5. **Key = `schema | kernel-build-id | module name | source hash | import interface hashes | semantic
   options | mode`** — no absolute paths, so a moved file still hits.
6. **Content hashes, not mtimes**; mtime only as a provisional fast path behind an opt-in flag.
7. **Cache the kernel's report verbatim**, never a bare "OK", re-emitted through the fresh-run path.
8. **Atomic write + validated read**: `.tmp-<pid>` then rename; verify hash, schema, kernel build id,
   artifact existence.
9. **Version-driven invalidation with knobs**: bump the schema on report/kernel changes; ship
   `--no-cache`, `--rehash`, `clean`; refuse stale binary caches loudly.
10. **Offline-first, best-effort**: cache failures warn and fall back to a fresh run; no network in
    the checking path.

## What to avoid

- Walking ancestors to the filesystem root for the marker (a stray `$HOME` marker captures unrelated
  files).
- Trusting mtime or a filename: no verdict may depend on a timestamp, and a corrupt artifact must
  never count as a hit.
- Caching a bare boolean instead of the kernel's report — loses per-file diagnostics and producer
  identity.
- A key without import-interface hashes or the kernel build id — the two ways "checked OK" goes wrong.
- Non-atomic or lock-free shared writes, and silent fallback to a stale binary or schema.

### Sources

LSP `_specifications/lsp/3.17` (+3.18) · rust-analyzer `crates/project-model/src/{lib,workspace}.rs` ·
clangd `clang-tools-extra/clangd/{GlobalCompilationDatabase.cpp,tool/ClangdMain.cpp}` +
clangd.llvm.org · TypeScript `microsoft/TypeScript@strada` `src/{server,compiler}/*.ts` · Lean/Lake
`src/lake/Lake/Build/{Trace,Common,Module}.lean`, `src/library/module.cpp`, `src/CMakeLists.txt` ·
GHC `docs/users_guide/separate_compilation.rst` · OCaml `file_formats/cmi_format.ml` · Coq
`vernac/library.ml` · markers: pyright `docs/configuration.md`, gopls `gopls/doc/workspace.md`,
ocaml-lsp `merlin_config.ml`, Agda `doc/user-manual/tools/package-system.rst`, `projectInfo.ts`.
