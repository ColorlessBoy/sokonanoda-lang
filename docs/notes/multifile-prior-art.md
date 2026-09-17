# Single-file vs. project mode: prior art from 10 systems

Facts checked against the linked official docs (direct fetch; `web_search` unavailable). ‡ = not re-verified this pass.

## 1. Coq/Rocq [rocq-cmd][rocq-util]
- **Bare file**: `rocq c foo.v`, then `Require foo.` — no project file. The logical module name "is inferred from the name of the input file and the corresponding `-R`/`-Q` options"; a filename must be an identifier plus `.v` (`to-to.v` is invalid).
- **`_RocqProject`** (extensionless, at the tree top; "some IDEs still look for the old name `_CoqProject`"): mapping lines plus a file list — `-R theories MyPackage` then `theories` gives `.v` files logical paths `MyPackage.File1`, `MyPackage.SubDir.File2`. IDEs "apply settings from the `_RocqProject` file in `dir` or the closest ancestor directory" (ancestor walk).
- **Flags**: `-Q dir dirpath` maps logical path `dirpath` onto physical `dir`, recursively; files reached through `-Q` "must include the logical name of the package in the `From` clause" (`From X Require Import Y`) or be fully qualified. `-R` is identical but also allows unqualified `Require Import`.
- **Build/artifacts**: `rocq makefile -f _RocqProject -o RocqMakefile`, then `make -f RocqMakefile` (generated files committed); Dune support exists and is labelled experimental. `.vo` sits beside sources (plus `.vos`/`.vok`); `rocqchk` re-checks compiled libraries standalone.

## 2. Agda [agda-lib][agda-iface]
- `.agda-lib` fields, all optional: `name:`, `depend:` (library *names*, not paths), `include:` (relative to the library file), `flags:`.
- **Discovery walks upward**: root is derived from the top-level module (`A.B.C` must live in `root/A/B`); if `root` holds `.agda-lib` files the search stops (error if more than one), otherwise Agda searches parent directories until it finds some, else none — and none may sit below the root on the file's path.
- `open import`; module name must match the path. Interface files go to `_build/VERSION/` under the project root with a library, else `.agdai` beside the source (orphans after a rename). Roots = `include:` + `--library`/`-l` (one `-i` per include path) + `AGDA_DIR/libraries`/`defaults`.

## 3. Isabelle [isa-sys][isa-isar]
- **No single-file mode.** Sessions live in `ROOT` files: `session <name> = <parent> + description options sessions directories theories document_theories document_files export_files export_classpath`.
- `ROOTS` catalogs list further session directories, one per line, recursively; `-d DIR` adds them on the command line. `isabelle build` "manages dependencies between sessions, related sources of theories and auxiliary files, and target heap images"; `-b` builds heap images, `-S` is a soft build observing only sources. Heaps, logs and build databases live in `ISABELLE_HEAPS` (default `$ISABELLE_HOME_USER/heaps`).
- Theory headers import by name "according to an acyclic foundational order": `theory First_Order_Logic imports Base begin`.

## 4. Idris 2 [idris-start][idris-pkg][idris-mod][idris-env][idris-inc]
- `idris2 hello.idr -o hello` writes `build/exec/hello`; `--check` type-checks a file and its dependencies with no REPL and no manifest on disk.
- `.ipkg`: `package maths`, `modules = Maths, Maths.NumOps, …` (required for library packages), `depends = pruviloj, lightyear` (with `>=`/`&&`), `sourcedir`, `builddir`, `outputdir`, `executable`, `main`; driven by `idris2 --build test.ipkg`.
- Module identity is **declared** (`module Foo.Bar.MyModule`) **and** the path relative to `sourcedir` must match; components are capitalised; no declaration means `Main`. Roots: `sourcedir`, `depends`, `IDRIS2_PATH`, `IDRIS2_PACKAGE_PATH`, `-p/--package`. Artifacts: `build/exec`, `build/ttc/*.ttc` "regenerated if the source file changes"; the docs warn to delete `build` after switching whole-program/incremental modes.

## 5. Rust [rust-ref][rustc-cli][cargo-new][cargo-ws][cargo-cache][cargo-lock]
- `rustc single.rs` → `./single`, no manifest; rustc reads the crate root and follows `mod` declarations. `mod foo;` resolves to `foo.rs` or `foo/mod.rs` (both present is an error; ancestors become directories, so `util/config.rs`); `#[path]` overrides; `mod.rs` is the "older style, still supported". `use` paths are `crate::`/`self::`/`super::`-rooted.
- `cargo new` writes `Cargo.toml`, `src/main.rs`, `.gitignore` (`/target`). Workspaces use `[workspace] members`/`exclude`; a virtual manifest has no `[package]`; all members share one `Cargo.lock` and one `target/` at the root.
- Artifacts under `target/{debug,release}/…`; Cargo "primarily uses timestamps on files to govern whether rebuilding needs to happen" (content-hash freshness is nightly-only). Dependencies come from crates.io; `Cargo.lock` is created on first build and "when in doubt, check … into version control".

## 6. Go [go-cmd][go116][go-mod][go-spec]
- `go run single.go` works with **no `go.mod`**: with explicit `.go` files, build "treats them as a list of source files specifying a single package"; a package-pattern build (`go run .`) requires a module. Modules were opt-in in Go 1.11, default since 1.16 "regardless of whether a `go.mod` file is present".
- `go.mod` declares `module <path>` (also the package-path prefix) plus `go`, `require`, `replace`, `exclude`; `go.sum` holds dependency hashes, synced by `go mod tidy`. "Each package within a module is a collection of source files in the same directory that are compiled together"; package path = module path + subdirectory; `internal/` restricts importers.
- Artifacts: build cache under `GOCACHE` (a `go-build` directory in the OS user-cache dir), module cache in `$GOPATH/pkg/mod`; `go build` writes the executable beside sources, `go run` writes nothing there. A package may not import itself even indirectly; two modules providing one package is an error.

## 7. Python [py-tut][py-path][py-import][py-spec]
- `python script.py` needs no manifest; the script's directory goes first on `sys.path` (cwd for `-m`/`-c`), then `PYTHONPATH`, stdlib, then `site`-processed `site-packages`. A module is a `.py` file; a package is a directory with `__init__.py` (without one it is an implicit namespace package).
- `pyproject.toml` specifies exactly three tables — `[build-system]` (`requires` mandatory, `build-backend`), `[project]`, `[tool]` — none consulted to run a script. Installing copies code plus `{name}-{version}.dist-info/` (METADATA mandatory) into `site-packages`; editable installs add the source directory to the import path instead; `importlib.metadata` reads that metadata.
- `__pycache__/module.<version>.pyc` sits beside the source and is invalidated by source mtime. Failures: the script directory shadows the stdlib ("This is an error unless the replacement is intended"); the first `sys.path` hit wins; cycles are tolerated via a partially initialised `sys.modules` entry; missing names raise `ModuleNotFoundError`.

## 8. JavaScript / TypeScript [node-mod][node-pkg][node-esm][ts-ref][tsconfig][npm-ws]
- `node script.js` works with no `package.json`; `.js` is CommonJS unless the *nearest parent* `package.json` has `"type": "module"` (`.mjs`/`.cjs` are fixed).
- CJS resolution is published pseudocode: core → relative (`LOAD_AS_FILE`: exact, `.js`, `.json`, `.node`; `LOAD_AS_DIRECTORY`: `"main"`, then `index.js`) → `#imports`/self-reference → `LOAD_NODE_MODULES`, which builds `NODE_MODULES_PATHS(START)` by walking up the directory chain emitting `<ancestor>/node_modules`, then `NODE_PATH`. ESM differs: mandatory file extensions, no folder mains, `NODE_PATH` "is not part of resolving `import` specifiers", and `exports`/`imports` gate subpaths.
- `tsconfig.json` is the project manifest: `include`, `exclude` (filters `include` only), `rootDir`, `outDir`, `paths` (compile-time aliasing only), `references` (targets must set `composite`); `tsc -b` finds, checks and builds referenced projects in dependency order. npm `workspaces` symlinks workspace directories into `node_modules`.

## 9. Haskell / OCaml [ghc-sep][cabal-file][cabal-nix][ocaml-comp][ocaml-dep][dune-usage]
- GHC: `ghc hello.hs` produces `.o`, `.hi` and the executable beside the source; the search path is exactly `["."]`, extended by `-i<dir>` (a bare `-i` resets it). Declared `module A.B.C where` must live at `A/B/C.hs` (`Main` excepted); cycles must be broken by an `.hs-boot` file. `.cabal` declares `exposed-modules`/`other-modules`, `hs-source-dirs`, `build-depends`; `cabal.project` lists `packages:`; Stack adds `stack.yaml`/`package.yaml`; products land in `dist-newstyle/`. GHC recompiles on mtimes *and* matching MD5 fingerprints; Cabal warns that unlisted sources may fail to trigger recompilation.
- OCaml: module name is never declared — "the compiler always derives the module name by taking the capitalized base name of the source file"; an `x.mli` is checked against `x.cmi`. `ocaml foo.ml` runs a file as toplevel phrases; `ocamlc -I dir` adds search paths (`OCAMLTOP_INCLUDE_PATH` for the toplevel). Dune "requires at least one of `dune-project`/`dune-workspace` to operate"; `(library (name foo))`/`(executable (name foo))` consume only modules in their own directory unless `include_subdirs`; output is `_build/default`. A missing interface yields `Cannot find file mod.cmi`; a module/path mismatch is only warning 24.

## 10. JVM basics [java-man][jls7][maven-layout][maven-pom]
- `java Hello.java` runs one file with no manifest — compiled in memory, no other sources found, the file-name/type restriction deliberately not enforced; `javac Hello.java` writes `Hello.class` beside the source (`-d` redirects). Roots: `-cp`/`--class-path` (overrides `CLASSPATH`, default `.`) and `-sourcepath`. Package==directory is host-dependent — the JLS says the host system "may choose to enforce" it. Maven adds `pom.xml` with the required `<groupId>:<artifactId>:<version>` triple, roots `src/main/java`, `src/test/java`, output `target/`.

## Cross-cutting comparison

| System | Discovery / bare file | Module identity | Import roots | Artifacts & invalidation | Cycles / duplicates / missing |
|---|---|---|---|---|---|
| Rocq | ancestor walk to closest `_RocqProject`; `rocq c foo.v` alone works | derived: `-Q`/`-R` logical path + filename | flags; manifest lines; `ROCQLIB` | `.vo` beside source; rebuilt when older | cycle/dup = error; missing `.vo` fails `Require` |
| Agda | upward `.agda-lib` search from module-derived root; `agda Foo.agda` works | derived, strict (name == path) | `include:`; `-l`/`-i`; `AGDA_DIR` | `.agdai` beside source or `_build/VERSION/` | mismatch = error; orphan `.agdai` on rename |
| Isabelle | no single-file mode; `ROOT` + `ROOTS` + `-d DIR` | declared theory name; filename conventional | ROOT `theories`/`sessions`, ROOTS, `-d` | heaps/logs/DBs in `ISABELLE_HEAPS`; `-S` soft build | imports must be acyclic |
| Idris 2 | no walk; `.ipkg` explicit; `idris2 f.idr` alone works | declared **and** path must match | `sourcedir`, `depends`, `IDRIS2_PATH`, `-p` | `build/exec`, `build/ttc` (source change) | mismatch = error; stale objects on mode switch |
| Rust | Cargo ‡finds manifest in cwd/ancestors; `rustc f.rs` alone works | declared `mod` + fixed file rule | manifest (incl. `path` deps); no search env | `target/` per workspace; mtime fingerprints | `foo.rs`+`foo/mod.rs` = error; crate cycles impossible |
| Go | ancestor walk for package patterns; `go run f.go` needs none | package path = module path + directory | `go.mod module`/`require`; module cache | `GOCACHE`; exe beside source | self-import illegal; 2 modules for one package = error |
| Python | no manifest at all; script dir implicit | derived from filename, never declared | script dir, `PYTHONPATH`, stdlib, `site-packages` | `__pycache__` beside source; source mtime | cycles tolerated; first hit wins silently |
| Node/TS | nearest-parent `package.json`; runs with none | none (CJS); ESM specifier is a path | upward `node_modules` walk; `NODE_PATH` (CJS) | `node_modules`; `outDir`/`.tsbuildinfo` | cycles legal; first match in walk wins |
| GHC/Cabal | `.cabal` in cwd (no walk ‡); `ghc f.hs` alone works | declared, file must be `A/B/C.hs` | `-i` (default `["."]`), package DBs, `GHC_PACKAGE_PATH` | `.o`/`.hi` beside source; `dist-newstyle/`; mtime + fingerprints | cycles need `.hs-boot`; same module in 2 packages legal |
| OCaml/Dune | needs `dune-project`; `ocaml f.ml` alone works | derived, never declared | `-I`, `OCAMLPATH`, opam lib, `_build` | `_build/default`; `.cmi`/`.cmx` | ordered compilation; mismatch = warning 24 |
| JVM | none; Maven `pom.xml` at project root | declared `package` + type name; dir rule host-enforced | `-cp`/`CLASSPATH`, `-sourcepath`, `~/.m2` | `.class` beside source or `-d`/`target/`; newest file wins | mutual refs compile together; duplicate types = error |

**Readings.** Only Agda, Rocq's IDE integration, Node and (‡) Cargo-style workspaces walk *up* for a manifest; all but Isabelle accept a bare file. Only OCaml and Python derive module identity with no declaration at all; Rocq/Go derive it from path + root; Haskell/Idris/Isabelle declare it and check agreement.

**Teaching/LSP angle.** The systems beginners meet first are exactly those where a bare file runs (Python, Node, `rustc f.rs`, `go run f.go`, `java Hello.java`, `ghc hello.hs`, `ocaml foo.ml`). Documented friction: Coq's `-Q`/`-R` + `From` clause, Agda's module==path rule, Dune refusing to run without `dune-project`, Cabal's must-list-every-module rule, Java's classpath, TS's config matrix. For a language server, path-derived identity is a gift (the module name is computable from disk alone: OCaml, Python, Rocq-with-mapping); declared identity needs parsing plus a mismatch check. Ancestor walks make "open any file anywhere" work but hide state — always report *which* manifest was used.

## Design patterns worth copying for a teaching language

1. **Zero-config single file as the default** (Python, Rust, Go file lists, `java Hello.java`): no project concept in the first five minutes. Cost: two code paths that must not diverge.
2. **Path-derived module identity under an explicit root** (Rocq `-Q`, Agda): one canonical name per file, no declaration to drift. Cost: a crisp root rule and a clear out-of-root error.
3. **A tiny manifest that is only a mapping** (Rocq `_RocqProject`): teachable in five lines. Cost: generated build files must be deliberately committed or ignored.
4. **Ancestor walk, file's own directory as fallback** (Agda `.agda-lib`; Cargo ‡): "open any file and it works". Risk: hidden ancestors change behaviour — report the chosen manifest.
5. **One project artifact directory keyed by tool version** (Dune `_build`, Agda `_build/VERSION`, Cargo `target/`): clean tree, caches survive upgrades. Cost: staleness; a clean command.
6. **Interface file per module as the cache unit** (`.agdai`, `.cmi`, `.hi`, `.vo`): incremental checking, and the artifact feeds the LSP. Cost: key must cover dependencies and tool version.
7. **Content/interface hashing, not raw mtime** (Go build cache, GHC fingerprints, `tsc -b`): robust to `touch`, checkouts, clock skew. Cost: hashing time, richer cache format.
8. **Acyclic import graph enforced loudly** (Go, Rust crates, Isabelle): simple semantics, no partially-initialised modules. Cost: refactoring pressure; no escape hatch (`hs-boot` is the anti-model).
9. **A single project-named import root** (Go module path, Rocq logical path, Java package): imports are self-describing. Cost: two names per file; renames ripple.
10. **"Directory = set of modules", with excludes** (Dune `(modules)` defaults to every `.ml` there): new files are picked up automatically; Cabal's must-enumerate rule causes silent staleness. Cost: excludes for generated files.
11. **A standalone checker over artifacts** (`rocqchk` re-checks `.vo` without the prover): the "frozen kernel as only judge" shape, reusable by CLI, LSP and grading. Cost: serialised, replayable proofs.

## Anti-patterns to avoid

1. **Requiring a manifest to check one file** (Isabelle sessions; Dune without `dune-project`): destroys the zero-config first run.
2. **Silent module-name/path mismatch** (OCaml warning 24; javac's rule the JLS only "may" enforce): one import, different resolutions.
3. **Env-var-only search paths** (`GHC_PACKAGE_PATH` *replacing* the package-DB stack; `PYTHONPATH` shadowing stdlib): invisible global state.
4. **First-hit-wins duplicates with no detection** (Python `sys.path`, Node's `node_modules` walk): identical sources, silently different programs.
5. **Manifests that must enumerate every file or silently stop invalidating** (Cabal `other-modules`): the worst failure is a *stale success*; prefer directory defaults plus excludes, and key the cache on every input.

[rocq-cmd]: https://rocq-prover.org/doc/master/refman/practical-tools/coq-commands.html
[rocq-util]: https://rocq-prover.org/doc/master/refman/practical-tools/utilities.html
[agda-lib]: https://agda.readthedocs.io/en/latest/tools/package-system.html
[agda-iface]: https://agda.readthedocs.io/en/latest/tools/interface-files.html
[isa-sys]: https://isabelle.in.tum.de/doc/system.pdf
[isa-isar]: https://isabelle.in.tum.de/doc/isar-ref.pdf
[idris-start]: https://idris2.readthedocs.io/en/latest/tutorial/starting.html
[idris-pkg]: https://idris2.readthedocs.io/en/latest/reference/packages.html
[idris-mod]: https://idris2.readthedocs.io/en/latest/tutorial/modules.html
[idris-env]: https://idris2.readthedocs.io/en/latest/reference/envvars.html
[idris-inc]: https://idris2.readthedocs.io/en/latest/backends/incremental.html
[rust-ref]: https://doc.rust-lang.org/reference/items/modules.html
[rustc-cli]: https://doc.rust-lang.org/rustc/command-line-arguments.html
[cargo-new]: https://doc.rust-lang.org/cargo/commands/cargo-new.html
[cargo-ws]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[cargo-cache]: https://doc.rust-lang.org/cargo/reference/build-cache.html
[cargo-lock]: https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html
[go-cmd]: https://pkg.go.dev/cmd/go
[go116]: https://go.dev/doc/go1.16
[go-mod]: https://go.dev/ref/mod
[go-spec]: https://go.dev/ref/spec
[py-tut]: https://docs.python.org/3/tutorial/modules.html
[py-path]: https://docs.python.org/3/library/sys_path_init.html
[py-import]: https://docs.python.org/3/reference/import.html
[py-spec]: https://packaging.python.org/en/latest/specifications/pyproject-toml/
[node-mod]: https://nodejs.org/api/modules.html
[node-pkg]: https://nodejs.org/api/packages.html
[node-esm]: https://nodejs.org/api/esm.html
[ts-ref]: https://www.typescriptlang.org/docs/handbook/project-references.html
[tsconfig]: https://www.typescriptlang.org/tsconfig
[npm-ws]: https://docs.npmjs.com/cli/v11/using-npm/workspaces
[ghc-sep]: https://downloads.haskell.org/ghc/latest/docs/users_guide/separate_compilation.html
[cabal-file]: https://cabal.readthedocs.io/en/stable/cabal-package-description-file.html
[cabal-nix]: https://cabal.readthedocs.io/en/stable/nix-local-build.html
[ocaml-comp]: https://ocaml.org/manual/latest/comp.html
[ocaml-dep]: https://ocaml.org/manual/latest/depend.html
[dune-usage]: https://dune.readthedocs.io/en/stable/usage.html
[java-man]: https://docs.oracle.com/en/java/javase/21/docs/specs/man/java.html
[jls7]: https://docs.oracle.com/javase/specs/jls/se21/html/jls-7.html
[maven-layout]: https://maven.apache.org/guides/introduction/introduction-to-the-standard-directory-layout.html
[maven-pom]: https://maven.apache.org/pom.html
