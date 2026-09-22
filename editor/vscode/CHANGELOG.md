## [0.64.2] - 2026-09-21

### Changed

- **编译不再把编辑器冻住**（T-A30）。以前语言服务器在 `Mutex<Docs>` 里同步编译：
  一次 8.9 秒的冷编译期间，第一个只读请求（目标栏 / hover / `soko/stateAt`）
  等了 **8907ms**——整个编辑器像死了一样。现在 `didOpen` / `didChange` /
  `didSave` / 文件监视通知**只记下最新文本就返回**，编译由后台任务做，编译期间
  到达的只读请求读**上一次完成的状态**。

  | 判据（真进程，进 CI） | 改前 | 改后 |
  |---|---|---|
  | 一次 ~1.2s 编译进行中的 `soko/stateAt` | **1277ms** | **< 1ms** |
  | 打开 + 连打 5 个键的**编译趟数** | 6 | **≤ 3** |

  **要付的代价（写在这里，不藏）**：重建**慢**的文件（上一次编译 ≥150ms）下一次
  编辑会等一个 **120ms 静默期**——那是 clangd 的规则，拿 120ms 的反馈延迟换
  "敲 7 个字母编 7 次"。小文件**不防抖**（立刻编）。`SOKO_DEBOUNCE_MS` 可覆盖。

- **"内容没变就不重编"的判据改成闭包摘要**（顺带修掉一个真的漏判）：以前只看
  这份文件**自己的**文本与打开文档的覆盖——依赖在**磁盘上**被改了
  （`git checkout` / 另一个编辑器）时自己的文本一个字节没变，短路会命中，
  旧诊断就一直显示下去。现在判据是**闭包摘要**（只读文件 + 哈希，毫秒级）。

### Added

- 设置项 **`sokonanoda.warmCacheOnOpen`**（**默认关**）：打开工作区时后台跑一次
  `sokonanoda build <项目根>`，把编译缓存预热好，让随后第一次打开某个单元直接
  命中。**默认关**是有意的——它占 CPU/IO，而"打开编辑器"本身会因此变慢，那正是
  另一面的抱怨。它不抢焦点（进度在 `sokonanoda build` 输出面板），也不报错
  （失败只是没预热，`sokonanoda: doctor` 能查）。

### Fixed

- 修掉一个会让文档**从此不再被编译**的竞态：后台任务的"没活了"与"新活插进来"
  之间有窗口，命中一次就再也不编了。

## [0.64.1] - 2026-09-21

### Changed

- **记法消解不再为了问一个类型就重编整份前缀**（缺口 G-34）。以前语言服务器
  每展开一次记法（`∈` / `⊆` / `∪` / `𝒫` / `''` …）都要问内核"这个操作数的类型
  是什么"，而那条路会**合成一份新文档、把整段前缀从零重编一遍**——前缀随每条
  声明增长，于是同一份文件里越靠后的声明越慢。

  现在**局部变量的类型直接取自它的书写类型**（本来就在手边，零内核调用），
  只有拿不到时才回退到问内核。以 `unit12-solution` 为例：

  | 指标 | 改前 | 改后 |
  |---|---|---|
  | 问内核的次数 | 126,105 | **51,156** |
  | 其中"重编整段前缀"的次数 | 363 | **247** |
  | 冷跑墙钟 | 11.4–12.0s | **7.5–8.4s** |

  判定结果**逐字节不变**（全语料 549 组对拍零差异），内核一个字节未改。

- 还有约 247 次"重编整段前缀"来自别处（冗余 `sorry` 探针、裸常量头、归纳类型
  安装、闭包里的记法），它们**没有局部类型可拿**，要等 T-K20′ 的"就地拿当前
  环境"才能根治。**这一版是提速，不是根治**。

## [0.64.0] - 2026-09-21

### Changed

- **编辑器打开项目文件不再从零重编**：语言服务器接上了项目编译缓存
  （含 `import` 的文档）。第一次打开照旧编译，**同时把结果写进缓存**；
  之后重启编辑器、重开同一份文件直接命中。

  | 文件 | 冷开 | 热开 |
  |---|---|---|
  | `unit01-sets-membership` | 1833ms | **2ms** |
  | `unit08-images-preimages` | 4821ms | **8ms** |
  | `unit12-synthesis` | 8838ms | **10ms** |
  | `unit12-solution` | **36259ms** | **26ms** |

  缓存键是**整个 import 闭包**的摘要（每个模块的源文本 + import 边 + prelude
  模式 + 入口路径 + 打开文档的内存覆盖），所以依赖改一个字节就失效——它不会
  让你看到"上一次的世界"。命中时**整份项目报告**一起回放，跨文件跳转/重命名
  照常工作。

- 项目缓存的键不再包含可执行文件的 mtime：以前"CLI 预热过、编辑器却不命中"
  取决于两个二进制的时间戳是否落在同一秒（本机实测四组里三组不同秒）。
- 清单 `requires` 与仓库版本不一致时**不再关掉整个缓存**；那条提示仍然会出现在
  `query check` 的 `warnings[]` 里。仓库内所有清单的 `requires` 现在由
  `python3 scripts/bump.py <x.y.z>` 统一提升，并有门禁（`soko gate` + CI）看着。

### Fixed

- 项目缓存热命中时 `query project` 不再答 `project:null / reason:"no-path"`。
- 内容逐字相同但在不同目录的两个项目不再共用缓存条目（以前第二个会拿到第一个
  的路径——跨文件跳转会跳到别的目录）。

## [0.63.3] - 2026-09-21

### Changed

- **保存 / 编辑器外改动不再重编同一份文本**：语言服务器现在比较
  「文本 + prelude 模式 + 入口路径 + 依赖的内存覆盖」，四样都没变就直接返回。
  保存已打开的文件、`git checkout`、别的工具重写同样的字节都会走这条路，
  而它们的内容往往和缓冲区一模一样——实测 8 模块的课程单元
  **361ms → 0ms**。
- 顺带去掉一次冗余编译：项目模式下不再先白编一遍入口单文件再丢弃
  （实测收益只有 ~1%——入口单独解析就失败，那次"白编"在 parse 阶段就退出了；
  改动保留是因为它确实去掉了冗余工作，但**不算性能收益**）。

### 实测

- 新增两条真实课程性能哨兵（`save_same_text` / `watched_unchanged`），
  回滚修法即红（都是 360ms 级）；
- `sokonanoda-lsp` 151 passed、`sokonanoda-front` 673 passed、
  CLI query/imports/protocol/course 全绿。

## [0.63.2] - 2026-09-21

### Fixed

- **切文档后声明栏停住**（客户端三个竞态）：过期答案被丢弃之后不再"什么都不做"
  （新文档的声明没人取）；并发取数**按 URI 分键**（以前 A 在飞时切到 B，
  B 拿回 A 的 promise，B 的取数从未发生）；"取过没有"改看 URI 而不是
  `declItems` 的真值（空列表是真值 ⇒ 之后每次取数都空转，面板永远停在
  「等待编译…」）。取数失败也不再被记成"取过了"。
- **声明栏为空时说得清原因**：真的没有声明 / 服务器还没编译完 / 读取失败，
  三种情况不再一律显示「暂无声明。」。
- **只改了类型或行号的更新会被吞掉**：声明列表的指纹以前只含
  名字/种类/状态/洞 id，类型文本变了也判"没变" ⇒ 卡片停在旧值。
- **`→` 右边直接跟 `∀` / `fun` / `let` 不解析**（G-28）：`A -> ∀ x, P` 在
  Lean 4 里合法，以前报 `unexpected-token`（只有 `→` 不认，`↔` 是好的）。

### 实测

- 真 VS Code 集成测试：声明栏、`alt+n` 跳洞两例转绿；
- stub 宿主 32 例、webview 11 例、纯 Node 单测 68 例全绿；
- 课程门禁计数逐项不变（36 目标 · 328 checked · 99 open · 0 判负）。

## [0.63.1] - 2026-09-21

### Fixed

- **Infoview「声明」栏在项目文件里恒为空**（G-22）：用 `import` 来的记法时，
  入口文件**单独**解析必然失败，而"单独解析失败但 import 闭包编译成功"这条
  判据在 `check` 与语言服务器里各写了一遍、**`goals` 漏了** —— 于是声明栏
  永远显示「暂无声明。」，而同一份文档的目标栏、悬停、文档符号全都正常。
  现在判据收敛成**一处** `QueryDoc::usable()`，三处共用。
- **`alt+n`（跳洞）在项目文件里没反应**：同一条判据的另一面
  （`nextHole → holes → goals`），一并修好。

### 实测

- 卷 I 全量 86 个文件：声明栏空 **31 → 0**；
- 真 VS Code 集成测试新增两例（声明栏 / 跳洞），修复前红、修复后绿；
- 课程门禁计数逐项不变（36 目标 · 328 checked · 99 open · 0 判负）。

## [0.63.0] - 2026-09-21

### Fixed
- **Declaration cards now follow the active document.** The Infoview's
  declaration list was only pushed from inside a `soko/goals` load, and a
  focus switch only rebuilt the *tree* — which VS Code resolves solely while
  the `sokonanoda.goals` view is visible. With the Infoview open on its own
  (tree collapsed into the side bar) the cards stayed on the previous
  document: opening another single file looked refreshed only because its
  diagnostics triggered a load, and switching focus between already-open
  files (or opening a project module, which publishes no diagnostics of its
  own) left them stale. `trackEditor` now asks for the new document's
  declarations explicitly; the existing concurrency merge keeps a visible
  tree from paying a second round trip.

## [0.62.0] - 2026-09-20

### Added
- **Type notation the Lean 4 way: `\and` + `Tab` becomes `∧`.** The editor now
  rewrites the abbreviations Lean users already have in their fingers —
  `\and` → `∧`, `\in` → `∈`, `\sub` → `⊆`, `\powerset` → `𝒫`, … — using the
  **same 18-entry table the hover text is rendered from**
  (`crates/front/src/notation_input.rs`, mirrored by
  `src/abbreviations.js` and pinned by the contract test
  `abbreviation_table_mirrors_the_single_source`). Every alias works too
  (`\wedge`, `\mem`, `\emptyset`, `\preimage`, …). Hover any notation symbol to
  see how to type it.
- **`sokonanoda.input.eager`** (default **off**): replace an abbreviation as
  soon as the word is complete, without waiting for `Tab`. While you keep
  typing letters a prefix waits (`\an` waits for `\and`; `\in` waits for
  `\inter`); a separator closes the word and finishes it (`\in ` → `∈ `,
  `\sub ` → `⊆ `), so the short forms are reachable in eager mode too. A lone
  `\` — the set-difference symbol — is never touched. A replacement is a single
  edit, so **one undo takes it back in one step**, and multiple cursors are
  rewritten in one edit without disturbing other selections.

### Notes
- `Tab` is only taken over while a `\`-abbreviation is being typed (a context
  key set by the extension), so ordinary indentation and suggestion acceptance
  in `.sokonanoda` files keep working. Design:
  `docs/design/notation-input.md` (NI-2).

## [0.61.0] - 2026-09-19

### Added
- **Anonymous constructors `⟨a, b⟩`.** The constructor is picked from the
  **expected type** — `A ∧ B` ⇒ `And.intro`, `A ↔ B` ⇒ `Iff.intro`,
  `∃ (x : α), p x` ⇒ `Exists.intro`, `Prod α β` ⇒ `Prod.mk`, and any
  single-constructor inductive ⇒ its constructor — so `exact ⟨ha, hb⟩` and
  `exact ⟨w, hw⟩` work with no lemma name. `⟨`/`⟩` are **syntax**, not notation
  symbols (they are brackets, and the lexer's symbol runs would swallow `⟨∅`),
  so they are coloured by a new `punctuation.section.anonctor` rule rather than
  the generated math-symbol class. Reading no expected type reports the
  dedicated `elab-anon-ctor-no-expected-type` with a teaching hint. Nested
  `⟨a, ⟨b, h⟩⟩` is not supported yet (use `use` step by step). Design:
  `docs/design/course-lean-style.md` L2.7.

### Fixed
- **`intro` renames the bound variable, so any name you pick works.**
  `intro y` on `A ⊆ B` used to leave the goal mentioning the *definition's*
  binder name (`x`) with nothing binding it, and the failure surfaced on a
  later tactic as ``unknown identifier `x` ``. The engine now renames free
  occurrences (capture-avoiding) when it peels a Pi layer.
- **Delta unfolding no longer silently swaps variables, and goes several
  layers deep.** `def Set.powerset (α) (A) := fun (B : Set α) => Set.subset α B A`
  unfolded at `A ∈ 𝒫 B` used to substitute `A := B` under a lambda that also
  binds `B`, turning the goal into `∀ x, A x → A x` — a *silent* wrong goal
  whose error appeared later as ``expected `A x`, got `B x` ``. Substitution is
  now capture-avoiding. `intro` / `constructor` / `left` / `right` / `use` also
  unfold through up to four definition layers, so `intro x` works directly on
  `A ∈ 𝒫 B` and `constructor` on `a ∈ B ∩ C`.
- **`query check` and `grade` agree again on library-notation units.** When a
  unit's single-file parse fails only because its symbols arrive through
  `import`, the project closure rescues it — but `query check` still reported
  that intermediate parse error in `failed[]` while `grade` exited 0 (gap G-20's
  judgement, now applied on the query channel too).
- **`≠` goals can now be proved, not just stated.** `Ne` is
  `def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False`, so
  `intro h` on `a ≠ b` has to unfold it — but neither spelling of `≠` carries
  the level: the source AST is a *notation node* and the kernel's pretty-printer
  writes the bare name `Ne` (implicit universe parameters are elided). The
  unfolded hypothesis came out as `@Eq.{u} …` with a **dangling level variable**
  and failed later with ``unknown universe level `u` ``, far from the cause.
  The `by` engine now computes a **level hint from the operands' sorts** (the
  same rule `elab_notation` uses to solve a notation's level), with the two
  shapes handled separately: a pointful `Ne (Set α) A B` takes the sort of its
  first argument (one step), a notated `A ≠ B` takes the sort of that argument's
  *type* (two steps). Wrong guesses cannot pass silently — the level is still
  judged by the kernel. The shipped unit ② canvas and solution are now written
  in pure notation (`{a} ≠ ∅`, `{a, b} ⊆ A ↔ a ∈ A ∧ b ∈ A`, `{a, b} = {b, a}`,
  `{a} ∈ {{a}}`) with every proof in tactic style. Design:
  `docs/design/course-lean-style.md` §9, `docs/design/notation-subset.md` §15.

### Added
- **`have` (the tactic) and readable type-mismatch errors.** `have h : T := t` and
  `have h : T := by …` now work inside `by` blocks (lowered to `let h : T := t;
  <rest>`, so the annotation gives the value an expected type — a nested `by`
  that uses `cases` would otherwise fail with `elab-match-no-expected-type`).
  A nested `by` is delimited by **indentation**, the same layout rule as `cases`
  arms: the first tactic at or left of the `have`'s column belongs to the outer
  block. Type mismatches now say `expected \`B\`, got \`C\`` instead of dumping the
  folded Pi telescope (this also fixes `exact`), and the error points at the
  offending line. `cases` and `have` were missing from the single keyword source
  `front::semantic::KEYWORDS`, so neither was coloured or completed — both are in
  now. Design: `docs/design/course-lean-style.md` L3.6.
- **`=` and `≠` are built-in notations, and every shipped symbol is now
  coloured.** `a = b` used to be a *lexical error* (`expected `=>``), so the
  course had to spell equality `Eq.{1} (Set α) A B`; `≠` did not exist at all.
  Both are now Lean-core spellings available in every file with **no
  declaration**: `=` targets `Eq`, `≠` targets `Ne` (new in the L1 prelude's
  `B9` family), and the **universe level is solved from the operand types** —
  `A B : Prop` ⇒ `Eq.{0}`, `A B : Set α` ⇒ `Eq.{1}` — so `A = B` works at both
  levels. The TextMate `mathsymbols` class was a hand-written codepoint range
  (`U+2200–22FF` + `U+2A00–2AFF`) that silently missed `↔ ¬ 𝒫 ᶜ ⁻¹' ×ˢ`; it is
  now an explicit class generated from the single source
  `front::notation_input::notation_symbol_chars` and pinned by
  `crates/cli/tests/extension.rs::tm_grammar_math_symbols_follow_the_single_source`.
  Hovering a notation symbol now also explains **how to type it** (`\and` for
  `∧`, "typed directly" for `=`), and hovering a symbol that came in through
  `import` works again — the LSP used to discard the whole project report when
  the single file failed to parse (gap G-20). Design:
  `docs/design/course-lean-style.md` L2.3/L2.4b, `docs/design/notation-input.md`.

- **`abbrev` and the notation spellings are highlighted as keywords (and
  completed).** `abbrev` (the Lean spelling of `def`), `prefix` / `postfix`
  (second-cut notations) and `binder_notation` / `scoped` (third cut) now sit in
  the single keyword source `front::semantic::KEYWORDS`, so the TextMate grammar,
  the semantic tokens, the Infoview code fences and the LSP completion list all
  colour and offer them in the same round — the two word lists are pinned equal
  by `crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source`.
  The grammar's `declarations` rule also colours the name an `abbrev` declares.
  Design: `docs/design/abbrev.md` §4 and `docs/design/notation-subset.md` §13.6
  (both were the explicit "hand to the main line" sync items).
- **Universe-level arithmetic `u+1`, and `cast` / `Eq.ndrec` in the prelude.**
  `Sort (u+1)` (also `Sort u+1`), `Type (u+1)` and `Eq.{u+1}` are now accepted
  everywhere a universe level is written (`Eq.{u+1}`, `@Eq.rec.{u+1, u}`, …),
  so the prelude's `Eq.mp` / `Eq.mpr` / `cast` are **universe polymorphic** —
  their signatures now match Lean core's (`{α β : Sort u} (h : α = β)`) instead
  of being Type 0 instances. `cast h a` (Lean core's `Eq.mp h a`) and
  `Eq.ndrec` (the non-dependent recursor) join the prelude, so `Eq.mp`,
  `Eq.mpr`, `cast` and `Eq.ndrec` are available as completions. **Calling
  change**: a bare `Eq.mp α β h` still instantiates `u := 0` (this language
  inserts no implicit arguments and infers no universes), so the Type 0 shape
  becomes `Eq.mp.{1} α β h`. Ledger `L-03`; design
  `docs/design/eq-type-level-rewriting.md` §4 and
  `docs/design/type-level-syntax.md` §5; regression canvas
  `docs/gaps/repro/L03-eq-type-level.sokonanoda`.
- **The course tree groups 卷 → 章 → 单元 for a v2 course manifest.** A course
  manifest can now be the structured `soko.course/2` object (volume → chapter →
  unit, with chapter `prereqs` / `tags` / planned `quota.exercises`); the CLI's
  `course.unit` events then carry `volume` / `chapter` / `tags`, and the 「课程」
  tree renders collapsible volume and chapter nodes instead of a flat list.
  The switch is data-driven — the tree only reads events, so a v1 flat manifest
  (the intro course) keeps the flat tree byte for byte. Units that fail to load
  stay visible in a 「无法分组」 fallback group. Ledger `G-07`; design
  `docs/design/course-manifest-v2.md`. Contract test:
  `crates/cli/tests/extension.rs::course_map_consumes_the_cli_course_subcommand`;
  stub-host tests: the three course-tree cases in
  `editor/vscode/test-extension-host.js`.

## [0.60.0] - 2026-09-19

### Added

- **`sokonanoda: build` and `sokonanoda: rebuild`.** The CLI has always had
  `sokonanoda build [--clean] [<file>|<dir> ...]` to warm (and clear) the shared
  persistent compile cache, but the extension never exposed it — so "the first
  keystroke is slow" and "the panels look stale after I edited a dependency
  outside the editor" had no in-editor answer. `build` (`alt+b`) compiles the
  active file (following its `import` closure) or the first workspace folder;
  `rebuild` (`alt+shift+b`) clears the cache first and then rebuilds. Both
  stream the CLI's `build.file` / `build.clean` / `build.summary` JSON Lines
  into a **sokonanoda build** output channel, report
  `files · compiled · hit · failed` in a notification (with a "显示输出"
  button), refresh the exercise/project/course views, and warn instead of
  failing silently when a file does not compile. The command is also in the
  project view's title bar. Contract test:
  `crates/cli/tests/extension.rs::build_and_rebuild_commands_warm_the_compile_cache`;
  real-VS-Code smoke: the `build / rebuild` case in the integration suite.

## [0.59.0] - 2026-09-19

### Fixed

- **An exercise's signature is type-checked too (`sorry` no longer hides it).**
  `theorem t : 3 := sorry` used to be graded as an open exercise with zero
  diagnostics — a typo'd lemma name, a conclusion that is not a `Prop`, or a
  signature that does not elaborate at all looked exactly like "not done yet"
  in a 100-exercise canvas. The signature is now elaborated and run through the
  kernel's own type test before an exercise opens: a bad signature is a
  `diagnostic` on the signature's own range (`kernel-expected-sort`,
  `kernel-theorem-not-prop`, or `elab-unknown-identifier`), the declaration is
  `failed`, and **no** `exercise.open` is emitted. Legal open exercises are
  unchanged. Ledger `G-01`, work order `docs/gaps/WO-004-open-exercise-signature.md`.

- **Constructors live in their type's namespace (`Pair.mk`, `Or.inl`).**
  `ctor mk` used to be installed under its bare name and had to be unique in
  the whole project, so `Prod.mk`/`Subtype.mk`/`Exists.intro` could not
  coexist and libraries had to invent fake-unique names (`prod_mk`). A
  constructor is now installed as `Ind.ctor` (a source name that already
  contains a dot is kept verbatim, which protects `Nat.zero`/`Bool.true`).
  The **bare name stays a resolution alias** (a teaching-subset extension,
  not Lean semantics): it resolves when it is unique in the closure, and
  reports the new `elab-ambiguous-ctor-alias` when two types claim it.
  Existing canvases (the 11-unit intro course, the set-theory volume,
  `examples/`) need no edits. **User-visible:** `#check`/`#reduce`/`#print`,
  hover, the Infoview goal text, refine skeletons and semantic highlighting
  now print canonical names — and because a source `inductive Nat` with
  `Nat.succ` now hits the kernel's native-Nat fast path, `#reduce add two two`
  prints the mixed form `Nat.succ (Nat.succ (Nat.succ 1))` instead of
  `succ (succ (succ (succ zero)))`. Ledger `G-02`, work order
  `docs/gaps/WO-005-ctor-namespace.md`.

### Added

- **User-defined notation (`infix:N` / `infixl:N` / `infixr:N` / `notation`).**
  A canvas can now declare its own notation and write paper mathematics instead
  of prefix applications:

  ```sokonanoda
  infix:50 " ∈ " => Set.mem
  notation "∅" => Set.empty
  def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A
  ```

  Mathematical symbols (`∈`, `⊆`, `∅`, … — `U+2200–22FF` and `U+2A00–2AFF`
  plus `\`) are now their own token instead of being eaten as identifier
  characters, and a declaration's symbol text is a real string literal. Scope
  is **per file, after the declaration** (notation does not cross `import`
  yet). Notation is **sugar, not a declaration**: it emits no event, never
  appears in the declaration table or the goal view, and the two spellings
  grade **identically** — the pointful form keeps working forever. The
  expansion supplies the leading type parameter itself (bare-variable matching
  against the operand types, then the expected type), so `Set.mem`'s `α` is
  never written; when nothing determines it, the new
  `elab-notation-argument-unsolved` says so. Using an undeclared symbol is
  `notation-unknown-symbol` with a hint naming both ways out. **The shipped
  course canvases are deliberately unchanged** this round (they still write
  the pointful form) — a notation-version comparison is a separate round. Not
  yet supported (second cut): `𝒫`/`ᶜ` (Unicode letters, not symbols),
  `''`/`⁻¹'`/`×ˢ`, notation across `import`, binder notation, and overloaded
  symbols. **User-visible:** semantic highlighting colours declared notation
  symbols as operators, and the TextMate grammar grew a math-symbol rule.
  Ledger `G-04`, work order `docs/gaps/WO-011-notation.md`, design
  `docs/design/notation-subset.md`.

- **The prelude now ships the logic and equality skeleton (`And`, `Or`, `Not`,
  `Iff`, `True`/`False`, `absurd`, `Eq.symm`/`Eq.trans`/`congrArg`).** Thirty
  names that official Lean 4 keeps in `Init.Core`/`Init.Logic` are installed by
  default, so a canvas can use `And.intro`/`Or.elim`/`Iff.mp`/`absurd` without
  declaring them — `And`/`Or` are **real inductive blocks** (so `match` on them
  works and lowers to their derived recursor). **User-visible:** the
  completion list grows by 30 prelude names.
  A family **yields as a whole** when the file declares any of its names
  (whoever declares owns it; dependency-closed: `Not` needs `False`, `Iff`
  needs `And`, the `Eq` lemmas need the `Eq` prelude), so the intro course's
  units that deliberately build these axioms themselves are unaffected, and
  the set-theory library keeps working unchanged. Ledger `L-01`/`L-02`, design
  `docs/design/prelude-l1-proposal.md`.

## [0.58.0] - 2026-09-18

### Added

- **Project tree (`项目` view) + project state in the status bar tooltip.**
  The Explorer now shows the import closure around the active file straight
  from the language server's new `soko/project` request: the module root and
  its source (a `sokonanoda.toml` path, or "zero config" when the entry file's
  directory is the root), every module in topological order with its status
  (`compiled` / `load-failed` / `blocked`), declaration, error and
  open-exercise counts, the project-level diagnostics, and the modules'
  `import` edges. A module that only *suffers* from another module's failure is
  marked `blocked` and points at its broken dependency, so "which module is
  the root cause?" is answered without reading the raw event stream. Clicking a
  module opens it (`vscode.open`), clicking the root opens the manifest, and
  `sokonanoda: refresh project view` refetches. A file without `import` shows a
  single "single file (no import)" row instead of an empty tree, and answers
  for another document are dropped using the echoed document identity.
- The same closure view is available to scripts and agents:
  `sokonanoda query project [--file … | --text … | -]` prints one JSON object
  (`{project, reason}`) and the MCP bridge grew a matching `project` tool.

- **A leftover `sorry` is now named as such** (ported from the 0.56.2 line): when
  the answer already proves the goal and a `sorry` merely follows it, the editor
  flags that line with a `redundant-sorry` warning instead of "exercise not yet
  solved". `soko/goals` holes carry `redundant: true`, `sokonanoda query
  goals`/`holes` and the MCP tools expose the same field, and `--json` emits
  `{"type":"warning","code":"redundant-sorry",…}` with a teaching hint. The
  verdict is the kernel's: the term with that argument removed must pass a full
  check of the declaration (`docs/design/redundant-sorry.md`).

### Notes

- Rust workspace and extension versions are bumped together (0.58.0), as the
  release pipeline builds both from the same tag.

## [0.57.0] - 2026-09-18

### Added

- **Multi-file projects: `import Foo.Bar` + optional `sokonanoda.toml`.** A
  `.sokonanoda` file may now start with `import` lines that load sibling modules
  (dependencies first, the importing file last, one shared kernel environment).
  Imports work with no manifest at all — the module root is then the entry file's
  directory; a `sokonanoda.toml` (or `--root <dir>`) sets it explicitly, and
  `--no-project` ignores the manifest. Every project diagnostic is attributed to
  the file that caused it (`import-not-found`, `import-cycle`,
  `import-dependency-failed`, `import-name-collision`, `import-prelude-conflict`,
  `import-module-invalid`, `manifest-invalid`), and an imported module that still
  has open exercises adds a warning instead of failing the entry.
- **The language server follows the import closure.** Opening an entry file
  compiles its whole project: declarations from imported modules are visible for
  diagnostics, hover, completion and code actions, and **go to definition, find
  references and rename all work across files**. Multiple documents are tracked
  at once, and **editing an imported module immediately re-checks the files that
  depend on it** — unsaved edits included, because the compiler sees open buffers
  through an in-memory overlay. Diagnostics are re-published only when they
  actually change.
- **Compile cache is project-aware.** The cache key covers every module in the
  closure (names, sources, imports, order) plus the prelude mode, so touching a
  dependency can never serve a stale entry report.
- Single-file behaviour is untouched: a file with no `import` takes the exact same
  code path and produces byte-identical output.

### Fixed

- **The exercise tree and Infoview could describe the wrong file.** Switching
  documents while a `soko/goals` request was in flight built the rows from the
  *new* file's URI but the *old* file's declarations — clicking a row then
  jumped to a range in the wrong document. The request now pins its document and
  a stale answer is dropped.
- **Any extension's diagnostics re-ran our analysis.** The diagnostics listener
  was global (a TypeScript error from another extension triggered
  `soko/goals` + a full tree/Infoview rebuild) and fired up to twice per project
  edit. It now only reacts to `.sokonanoda` files, debounces 150 ms, merges
  concurrent requests, and skips re-posting an unchanged declaration list to the
  Infoview.
- The course tree re-ran the CLI (11 unit compiles, ~320 ms warm) on every root
  resolve; results are now reused for 30 s and re-run on
  `sokonanoda: refresh course map`.
- The universal-VSIX download fallback no longer blocks the extension host while
  unpacking (`execSync tar` → async).
- **Quick fixes work inside multi-file projects again.** The suggestions (refine /
  exact / rfl) were computed against the entry file's text alone, so a file that
  used an imported constructor offered nothing at the `sorry` — the language
  server now hands the whole import closure to the suggestion engine, and the
  exercise tree / Infoview get sub-goal expected types in projects too.
- **Changes made outside the editor now refresh open files.** The client already
  watches `**/*.sokonanoda`; the server now handles
  `workspace/didChangeWatchedFiles`, so after a `git checkout` (or any tool
  rewriting an imported module) the open entry re-checks itself instead of
  showing stale diagnostics until you reopen it.
- `soko/goals` / `soko/stateAt` echo the document they describe (`uri`, and
  `version` for goals); the extension drops an answer that names another
  document, so switching files quickly can never show the other file's exercises.

### Notes

- New course unit ⑪ ("modules and projects") with a runnable two-file example
  project at `course/unit11-project/`.
- `sokonanoda --help` documents `import`, `--root` and `--no-project`.
- Known limitation (recorded for the next round): a file changed *outside the
  editor* (git checkout, another tool) is not noticed until it is reopened —
  `didChangeWatchedFiles` is not wired yet.
## [0.56.2] - 2026-09-17

### Added

- **A leftover `sorry` is no longer called "not yet solved".** When the answer
  already proves the goal and a `sorry` merely follows it (the `f h` line plus a
  stray `sorry`), the editor now flags **that line** with a `redundant-sorry`
  warning telling you to delete it, instead of
  `declaration '…' uses 'sorry' (exercise not yet solved)`. The declaration is
  still open — the verdict is the kernel's: the term with that argument removed
  must pass a full check of the declaration
  (`docs/design/redundant-sorry.md`).
- The mark is machine-readable too: `soko/goals` holes carry `redundant: true`
  (`sokonanoda query goals` / `query holes` and the DSH MCP tools expose the same
  field), so an agent says "delete that line" rather than "keep proving".
- `sokonanoda --json` emits `{"type":"warning","code":"redundant-sorry",…}` with a
  teaching hint; the human view prints one `line:col: warning[redundant-sorry]`
  line on stderr and the exit code stays 0.

### Notes

- Genuine holes are untouched: a missing argument or proof still warns as before,
  and a term that does not prove the goal never gets the mark (both pinned by
  tests, including a forward-reference guard: a constant declared *after* the
  exercise is not visible to the verdict).

## [0.56.1] - 2026-09-17

### Notes

- **Internal refactor only — the extension behaves exactly as 0.56.0.** The language
  server's in-process test suite was split from one 2567-line file into
  `crates/lsp/src/tests/` (`mod.rs` plus nine feature files, largest 399 lines),
  clearing the last structural-debt item recorded after 0.56.0
  (`crates/lsp/src/lib.rs` had already come down from 4256 to 1105 lines).
  Test bodies, assertions and test names are unchanged — verified by an
  item-by-item move comparison (107/107 items, 95/95 test names, 220 `assert`
  lines) with all 117 server tests green before and after.
- No user action needed; the shipped server, its protocol and the diagnostics you
  see are identical to 0.56.0.

## [0.56.0] - 2026-09-17

### Added

- **Agent-facing kernel-truth query channel (no extension feature change)** —
  - `sokonanoda query <op>` (`check`/`state`/`goals`/`holes`/`hints`/`reduce`)
    prints **one JSON object** (`{schema:"soko.query/1", op, version, ok,
    data|error{code,message}}`) instead of the whole event stream; `--line/--col/
    --offset/--text` target a single position, and the exit codes are 0 = answered
    (including a structured `ok:false` and an open `sorry`), 1 = kernel-rejected,
    2 = usage error. Contract: `docs/protocol.md`.
  - the LSP's `soko/*` handlers **delegate** to that same editor-independent
    truth layer (`front::query`, `crates/front/src/query/`), so query logic
    exists once: the semantic functions left `crates/lsp/src/lib.rs`, and a thin
    `crates/lsp/src/query_map.rs` only maps offsets to `Range`/`Position`;
  - `dsh/mcp/server.js` + `scripts/soko mcp` expose the six queries as
    `mcp__sokonanoda__{check,state,goals,holes,hints,reduce}` MCP tools for
    DeepSeek Harness sessions (opt-in via `dsh/cordis.patch.yml`).
- Two front-end gaps fixed: derived recursors keep index arguments written in a
  constructor's result arrow chain (the course no longer hand-writes `Le`/`Even`
  `rec`/`iota`), and `inductive` accepts multi-name binder groups `(A B : Prop)`.
  `crates/cli/tests/query.rs` pins both views to one truth (`query check` counts
  ≡ `--json` event counts; `query state` ≡ LSP `soko/stateAt`, field by field).

### Notes

- Agent tooling only: the extension code is unchanged, so VS Code users need no
  action. Two boundary behaviours now match the rest of the protocol:
  `soko/hints` at a caret exactly on a declaration's end offset (or past the last
  line) returns the hint ladder instead of `[]`, and ranges computed from offsets
  now use UTF-16 columns (identical for BMP text, different only for astral).

## [0.55.0] - 2026-09-17

### Added

- **DeepSeek Harness support (agent-side; no extension feature change)** —
  the repository now ships everything a DSH session needs, so the same teaching
  loop works without opencode:
  - `.agents/skills/{sokonanoda-teacher,sokonanoda-dev,sokonanoda-ci}/SKILL.md`
    thin entries (DSH auto-discovers this directory; the skill name is the slash
    command, e.g. `/sokonanoda-teacher`) pointing at the canonical
    `skills/<name>/SKILL.md`;
  - `scripts/soko`, a dependency-free Node launcher that resolves the
    version-pinned CLI/LSP (repo build **verified by `--version`** → cache with a
    matching marker → VS Code extension bundle → version-pinned download) and
    refuses to run a stale cache. It is the harness-neutral command every skill
    and `AGENTS.md` now uses. The extension itself is unchanged: it keeps using
    its bundled server, and `scripts/soko` prefers a version-matching repo build
    over the cache;
  - `dsh/cordis.patch.yml` (`dsh web --patch ./dsh/cordis.patch.yml`) wiring the
    `lsp` tool to `.sokonanoda` files, plus `dsh/README.md` documenting that
    **server diagnostics are not delivered by DSH** — grading always goes through
    the CLI `--json` stream;
  - `dsh/hooks/` for the official-Lean-toolchain deny (the DSH equivalent of
    `opencode.json`'s permission rule).
- New contract test `crates/cli/tests/dsh.rs` (skill entries, launcher chain,
  patch shape) and a `scripts/` prefix in the skill path guard.

### Notes

- Agent tooling only; the editor extension code is unchanged. Users who only use
  VS Code need no action.

## [0.54.0] - 2026-09-16

### Added

- **Course completed to the locked 10 units (P3 of the syllabus redesign)** —
  - **Unit 9「关系与联结词」**: `Or` as a real inductive (auto-derived `Or.rec`,
    `match` lowers to it), `Iff` as a definition `And (A -> B) (B -> A)`,
    and `Le` / `Even` as inductive relations with elimination/induction lemmas
    (the explicit "inversion lemma" pattern).
  - **Unit 10「读证明与综合」**: a self-explanation checklist applied to a worked
    proof, formal↔informal translation pairs, two "fix this rejected proof"
    evaluations (the wrong proof lives in a comment), and a cross-unit capstone.
  - Totals: **units 10, checked 78, open 59, failed 0**. Every reading/evaluation
    exercise still produces a kernel-checked declaration — no new protocol events,
    no whitelist expansion.

### Known issues (recorded)

- `front`'s derived recursor is rejected by the kernel for an **indexed recursive
  `Prop`** inductive (`Le`/`Even`) — the IH shape mismatches — so those units
  hand-write `rec`/`iota`; `Or` (non-indexed `Prop`) and `Vec` (indexed `Type`)
  derive fine. Fixable in `front` (not frozen), tracked in `docs/HANDOVER.md` §3 E.
- `inductive` **multi-name parameter groups** (`(A B : Prop)`) do not parse
  (Pi binders do); tracked in `docs/HANDOVER.md` §3 E.

## [0.53.0] - 2026-09-16

### Changed

- **Course restructured to 8 units (P2 of the syllabus redesign)** —
  - `by` tactics moved to **unit 4** (right after functions/arrows) so the tactic
    toolbox and the Infoview goal state arrive early instead of at the end;
  - the over-loaded induction unit is **split into Ⅰ/Ⅱ**: Ⅰ = explicit
    `inductive`/`rec`/`iota`, recursion via hand-written `Nat.rec`, `match` on a
    non-recursive inductive, recursive `match` with the auto-inserted IH;
    Ⅱ = parameterized `Option A`, dependent `match` (= induction), nested /
    literal / wildcard / guard patterns, indexed `Vec A n`.
  - Totals: **units 8, checked 58, open 45, failed 0** (`checked` +1 because each
    half redeclares its own `inductive Nat`).
- Docs, skills and the extension README were synced; hand-written unit counts were
  removed from the root README and the website prose (the number is generated from
  `course/course.json`).

## [0.52.0] - 2026-09-16

### Changed

- **Course content revision (P1 of the syllabus redesign)** — see
  `docs/design/course-syllabus.md`:
  - U4 (universes) gained two "read `#check` output / judge the type" exercises and a
    predict-then-prove `#reduce` exercise (it was previously 0 checked / 0 reduced);
  - U6 lost `by_ex5`, an exact duplicate of `by_ex1`;
  - U3's `three_args` / `body_uses_let` are no longer ambiguous (the expected return is
    pinned);
  - hint ladders no longer contain complete answers — the "key piece" line states the
    trigger condition and which lemma/constructor to use;
  - a stale in-file claim in U5 ("`match` only handles non-recursive inductives") is gone.
- **Course test hardening**: new `solution_covers_every_canvas_exercise` and
  `en_solutions_match_chinese_event_counts` (canvases *and* solutions are now compared,
  including `expr.typed`, which caught a real EN-solution drift in U4).

## [0.51.0] - 2026-09-16

### Added

- **Newline-separated tactics** — inside `by` blocks you may now separate tactics
  with a newline instead of `;` (Lean style), and mix both. A tactic's expression
  ends when the next line starts with a tactic keyword, so `exact f` followed by
  `apply g` on the next line stays two tactics. No indentation sensitivity.

### Fixed

- **`sokonanoda gate` refuses to run with a stale binary** — the playground anchor
  compiles with the binary's *embedded* compiler, so a stale download cache (e.g.
  v0.27.0 while the repo is newer) would silently gate with old logic and report a
  bogus source error. `gate` now compares its own version against the repo's
  `Cargo.toml` and exits 3 with an actionable message on mismatch.

## [0.50.0] - 2026-09-15

### Changed

- **The Infoview has its own fixed colour palette** (per dark/light/high-contrast),
  tuned to look like VS Code's default Dark+/Light+ token colours. Every
  `SemanticKind` is now guaranteed to be coloured — `Prop`/`Type`/`Sort` no longer
  degrade to the plain foreground because a theme lacked a `symbolIcon` variable.
  Rationale: VS Code exposes no stable API for editor token colours, and scraping
  theme JSON was judged too costly; see `docs/design/highlighting.md` §3b.

## [0.49.0] - 2026-09-15

### Added

- **`sokonanoda build`** — compile files (or a directory) and persist the kernel
  artifacts in a shared, content-addressed cache, so later opens/builds skip
  recompiling. `--clean` clears it, `--json` reports hit/compiled/failed. The
  course view and `sokonanoda --json` now reuse the same cache. Design:
  `docs/design/compile-cache.md`.

### Fixed

- **Infoview panel is now reliably visible**: the view no longer carries a
  `when` clause (VS Code hides an *empty* extension container, which is why it
  "couldn't be popped open"), the extension is activated `onView`, and the host
  registers the webview provider **before** the (possibly slow) server
  resolution, which is what made the panel appear only after toggling the side
  bar. `sokonanoda: 打开目标面板` also opens the auxiliary bar.
- **The panel never shows a silent blank**: it renders a skeleton on load and a
  live status line (`编译中…` / `已就绪 · N 个声明` / `等待 .sokonanoda 文件`).
- **Declaration rows** no longer pretend to jump (it did not work reliably);
  each now shows the declaration's line number (`L12`) in small text next to the
  name, plus its type.
- **One highlighting source**: hover goal text is now the projection of the same
  semantic runs as the Infoview, and `SemanticKind` maps to TextMate scopes, CSS
  classes and LSP token types from a single table guarded by exhaustive tests.
  See `docs/design/highlighting.md` (markdown can only be TextMate-coloured, so
  hover and Infoview colours are close but not identical by design).

## [0.48.0] - 2026-09-15

### Added

- **Persistent compile cache** — the language server now caches each file's
  kernel report keyed by (compiler version, prelude mode, source text), so
  reopening an unchanged `.sokonanoda` file skips recompiling; opening files
  stays fast as a workspace grows. The kernel remains the only judge.
  Disable with `SOKONANODA_NO_CACHE=1`. Design:
  `docs/design/compile-cache.md`.

### Fixed

- **Infoview declaration types wrap** instead of being truncated, the goal line
  starts with `⊢`, and clicking a declaration now actually jumps the editor to
  it (the document is resolved from the click, not from `activeTextEditor`).

## [0.47.0] - 2026-09-15

### Added

- **Indexed inductive declarations** — types may now carry indices alongside
  parameters, e.g. `inductive Vec (A : Type) : Nat -> Type` with
  `ctor vnil : Vec A zero` / `ctor vcons (a : A) (n : Nat) (v : Vec A n) : Vec A (succ n)`.
  The recursor is derived (motive abstracts the indices), and `match` works for
  results that do not depend on the index (e.g. computing the length). Design:
  `docs/design/indexed-inductives.md`.

## [0.46.0] - 2026-09-15

### Added

- **`match` as a tactic** — inside a `by` block you can now write
  `match c with | red => green | green => red` (arm bodies are terms, exactly like
  the value-position `match`), judged against the current goal. `by exact match …`
  works too. The judge now keeps the real source prefix, so a `match`'s universe
  query succeeds inside judgements.

## [0.45.0] - 2026-09-15

### Added

- **Binder type inference at application sites** — an unannotated `fun x => …`
  used as a function (`(fun x => x) 1`, `(fun x y => x) 1 2`) now infers its
  binder types from the argument types (kernel-checked), so fewer annotations
  are needed. Positions with neither an expected type nor arguments still
  require the annotation.

## [0.44.0] - 2026-09-15

### Added

- **Infoview declaration types + click to jump** — the declaration list now
  shows each declaration's type as a small, dim, syntax-coloured line under its
  name (coloured from the same `front::semantic` runs as the goal state, never
  re-tokenized), keeping the one-declaration-per-line layout. Clicking a
  declaration now also moves the editor to it.

## [0.43.0] - 2026-09-15

### Changed

- **One highlighting path everywhere** — every place the editor shows
  `.sokonanoda` text now uses the same `sokonanoda` markdown code fence, so it
  is coloured by the single TextMate grammar: expression/signature hovers (were
  plain `text`), declaration hovers, the tactic and half-expression hover
  headers, completion documentation, and the exercise-tree tooltips. Blocks
  that a client cannot markdown-render (diagnostics, inlay hints, tree
  descriptions, code-action titles) stay plain text, as documented in
  `docs/design/goal-rendering.md` §7.

## [0.42.0] - 2026-09-15

### Added

- **Richer `match` patterns** — value-position matches now support wildcards
  (`_`), nested constructor patterns (`| some (succ k) =>`), natural-number
  literals (`| 0 =>`, `1`, …; desugared to `succ^k zero`), and `Bool` guards
  (`| succ k if p =>`). Arms are ordered and the first match wins, so the same
  constructor may appear in several arms. Unknown bare names bind a variable
  (Lean semantics). Design: `docs/design/match-patterns.md`.

## [0.41.0] - 2026-09-15

### Added

- **Prelude `Bool`** — `Bool`, `Bool.true`, `Bool.false` and the derived
  `Bool.rec` are now installed as a trusted, non-recursive inductive (the same
  way as `Nat`), so `match b with | Bool.true => … | Bool.false => …` checks and
  reduces through the real kernel. A file that declares its own `inductive Bool`
  keeps it (the prelude steps aside).

## [0.40.0] - 2026-09-15

### Added

- **Infoview in the right side bar** — the goal panel now lives in its own
  `sokonanoda` container on the **secondary (right) side bar** instead of
  sharing the Explorer, so it can sit next to your proof. Requires VS Code
  **1.106+** (extension-contributed secondary-side-bar containers), and
  `sokonanoda: 打开目标面板 (Infoview)` reveals it.
- **Unified goal highlighting** — goal and hypothesis text in the Infoview is
  now coloured from the **same single source** as the editor's semantic tokens
  (`front::semantic`): `soko/stateAt` ships `goal_runs` / `ty_runs` (ordered
  `{text, kind}` fragments) and the webview renders them with theme-aware
  colours. No more re-tokenizing, no more drift between hover and the panel.
  Design: `docs/design/goal-rendering.md`.

### Changed

- The TextMate grammar now mirrors `front::semantic` exactly (hover code fences
  highlight `let`, `match`, `with`, `by`, `intro`/`exact`/`apply`/`assumption`/
  `rfl`, `#check`/`#reduce`/`#print`, `forall`/`∀`); the hard-coded `Nat`
  keyword is gone, and a contract test keeps the two lists equal.

### Fixed

- Opening the Infoview no longer shows the confusing
  「目标面板 (Infoview) 暂时不可用…」 warning: the view renders on demand and
  degrades silently to the Explorer tree's 「当前光标处」 group if a webview
  cannot be shown.
- Marketplace description trimmed under 300 characters (it was truncated
  mid-word) with a regression guard.

## [0.39.1] - 2026-09-14

### Fixed

- **`judge_infer` type round-trip robustness** — `render_expr` now parenthesises
  a `forall`/arrow/lambda when it sits in the **domain** of an arrow, so the
  kernel-type → text → AST round trip used by `judge_infer` (and thus the
  dependent-`match` motive level query, suggestions and hover) no longer
  corrupts a telescope containing a function-typed binder. Previously a
  dependent `match` whose result type referenced such a binder failed with
  `elab-match-no-expected-type`.

## [0.39.0] - 2026-09-14

### Added

- **`match` dependent motive** — when the result type mentions the (local
  variable) scrutinee, the motive becomes `fun t => R[x := t]`, each arm's
  expected type is the instantiated `R[x := <constructor term>]`, and a
  recursive arm's induction hypothesis has the dependent type `R[x := <field>]`.
  This makes the natural induction principle expressible:
  `theorem nat_induction (P : Nat -> Prop) (hz : P zero) (hs : …) (n : Nat) : P n := match n with | zero => hz | succ k => hs k ih`.
  Non-dependent matches are unchanged. Design:
  `docs/design/match-dependent-motive.md`.

## [0.38.0] - 2026-09-14

### Added

- **Parameterized inductive declarations (non-indexed)** — declarations may
  now take parameters, e.g.
  `inductive Option (A : Type) : Type` / `ctor none : Option A` /
  `ctor some (a : A) : Option A` / `end`, with the derived recursor. `match`
  works on them (`match o with | none => … | some a => …`); the type arguments
  come from the scrutinee's written source type, and the field type is
  instantiated (`some a` gives `a : A`). Indexed inductives,
  universe-polymorphic parameters, nested/mutual blocks and nested/guard
  patterns remain out of scope. Design:
  `docs/design/parameterized-inductives.md`.

## [0.37.0] - 2026-09-14

### Added

- **`sokonanoda watch` client commands** — the watch process now reads JSON
  Lines commands on stdin: `{"type":"ping","id":N}` replies with a `pong`,
  and `subscribe`/`unsubscribe` filter which files emit events in
  `--workspace` mode. Malformed input yields an `error` event and the stream
  keeps running; stdin EOF does not stop monitoring. Design:
  `docs/design/compiler-service-events.md`.

## [0.36.0] - 2026-09-14

### Added

- **`match` on the prelude `Nat`** — the built-in `Nat` is now a real trusted
  inductive (constructors `Nat.zero`/`Nat.succ` plus a derived `Nat.rec`), so
  `match` works on it directly:
  `def pred (n : Nat) : Nat := match n with | Nat.zero => Nat.zero | Nat.succ k => k`
  (arms use the dotted constructor names). Recursive branches get the induction
  hypothesis `ih`, exactly like source recursive inductives. Note: `#reduce`
  through `Nat.rec` may print an unary chain (e.g. `Nat.succ (Nat.succ 1)`) that
  is definitionally equal to the numeral. Design: `docs/design/match.md`.

## [0.35.1] - 2026-09-14

### Infrastructure

- **Release integrity** — every Release now ships a `SHA256SUMS` manifest
  covering the 8 LSP tarballs, 8 CLI tarballs and 9 VSIXes, plus SLSA
  build-provenance attestations (`actions/attest-build-provenance`) for all of
  them. Verify with `sha256sum -c SHA256SUMS` and
  `gh attestation verify <file> -R ColorlessBoy/sokonanoda-lang`. See
  `docs/RELEASE.md` §6. Asset count is now 26.

## [0.35.0] - 2026-09-14

### Added

- **`match` on recursive inductives (induction hypotheses)** — for a source
  recursive `inductive`, each recursive constructor field now gets an
  auto-inserted induction hypothesis named `ih` (`ih2`, …) typed as the match
  result; the branch body can reference it, so recursive functions/proofs are
  written through the recursor without self-reference
  (`def add (a b : Nat) : Nat := match a with | zero => b | succ m => succ ih`).
  Still out of scope: dependent motives, parameterized/indexed inductives, the
  prelude `Nat`/`Eq`, `match`-as-tactic and nested/guard/literal patterns.
  Design: `docs/design/match.md`.

## [0.34.0] - 2026-09-14

### Added

- **Unannotated `let`** — `let x := v; body` now infers the binder type from
  the value via the kernel (`judge_infer`, reusing its bounded cache); the type
  annotation is optional. When the value alone cannot determine the type (e.g.
  a `sorry` value), it reports the teaching error `elab-let-type-query-failed`
  asking for an explicit annotation.

## [0.33.1] - 2026-09-14

### Changed

- **Tactic hover presentation** — the tactic hover now names the tactic and its
  position, and renders each goal state in a `sokonanoda` code fence, so the
  hypotheses/goal are monospaced, aligned and syntax-highlighted (the extension
  ships the grammar). Multi-goal states show a `目标 i/n` header per goal. The
  half-expression goal hover uses the same fence.

## [0.33.0] - 2026-09-14

### Added

- **`match` (v1)** — pattern matching on **source-declared, non-recursive**
  `inductive` types: `match e with | Ctor x … => body | …`. Each constructor
  is covered once (any order; reordered to declaration order), the result type
  is the expected type at the match position, and lowering goes to
  `<Ind>.rec.{level} (fun _ => R) minors … e` (the universe level is derived
  from the expected type). Kernel-frozen and kernel-judged. Recursive
  inductives, dependent motives, parameterized inductives and the prelude
  `Nat`/`Eq` are explicit errors in v1. Course coverage added to the induction
  unit. Design: `docs/design/match.md`.

## [0.32.1] - 2026-09-14

### Changed

- **Incremental edits stop re-checking unchanged suffixes (early-cutoff)** —
  the session now compares each command's elaborated environment contribution
  (structural signature: type **and** body, so delta unfolding stays correct)
  and reuses the remaining snapshots once the signature matches. In the common
  case a one-line edit drops the kernel re-checks to 1 instead of the whole
  suffix. Conservative and sound: multi-edits, changed bodies and kernel
  rejections fall back to the previous suffix re-check. Design:
  `docs/design/early-cutoff.md`.
- Documented an opt-in external performance/soundness baseline against the
  Lean Kernel Arena (`scripts/perf-arena.sh`, `docs/PERF.md`); not part of CI.

## [0.32.0] - 2026-09-14

### Added

- **Kernel-driven expected types for refine sub-holes (spine meta, 方案 A)** —
  the goal view / inlay now fill sub-hole expected types that the syntax walk
  left empty, by probing the kernel at request time (`judge_infer` + its
  bounded cache; never on the keystroke path). Covered: preceding-hole
  penetration (`f sorry sorry`, the second using the first's expected type),
  one-level nested holes (`f (g sorry)`), and syntactic def-eq domains.
  Deeper nesting and def-wrapped *result* types still fall back to the old
  behaviour. Design: `docs/design/spine-meta-a.md`.

## [0.31.0] - 2026-09-14

### Added

- **`sokonanoda: doctor`** — a read-only self-check that reports which server
  is in use and its `source`, the running vs extension version, a stale
  extension host, ignored overrides, the download cache and old-version
  buildup. It runs automatically after activation and `restart server`, and
  offers a "Run doctor" action on problems.

### Changed

- **Bundled server is now forced by default** — `sokonanoda.serverPath` /
  `SOKONANODA_LSP_BIN` and workspace `target/{debug,release}` builds are
  ignored unless the new `sokonanoda.serverOverride` setting (default
  `false`) is enabled. A stale local build can no longer silently override
  the server that ships inside the extension. Design:
  `docs/design/extension-server-policy.md`.

## [0.30.0] - 2026-09-14

### Added

- **Infoview panel (webview)** — a Lean-Infoview-style goal panel in the
  side bar (`sokonanoda.infoview`, command `sokonanoda.openInfoview`) rendering
  every goal with its hypotheses, the `by` progress and the running server
  version. The extension host remains the only LSP client (it posts
  `soko/stateAt` / `soko/goals` / `soko/version` snapshots); cursor moves post
  only the light `state` message (no `soko/goals`, no rebuild), and the panel
  falls back silently to the existing tree group when a webview is
  unavailable. Design: `docs/design/webview-infoview.md`.

## [0.29.0] - 2026-09-14

### Added

- **Compiler service event stream** — `sokonanoda watch` now opens with a
  `service.hello` handshake (`{protocol, engine, pid}`) and its canonical
  change event is `file.didChange` (`file.changed` stays as a one-minor
  deprecation alias). New flags: `--doc <file>` (single document) and
  `--workspace <root>` (every `*.sokonanoda` under the root, one session per
  file, per-file versions, events carry `file`). Design:
  `docs/design/compiler-service-events.md`.

## [0.28.0] - 2026-09-14

### Added

- **Local bindings: `let x : T := v; body`** — the teaching language now
  supports `let` at any term position (also inside `#check`, `#reduce`, `fun`
  bodies and parentheses). It elaborates to the kernel's own `Let` (zeta), so
  the value is still judged by the complete kernel; the type annotation is
  required for now. Includes course coverage in the functions/arrows unit and
  goal-view support for `sorry` in the value or body. Design:
  `docs/design/elaborator-let-match.md` (`match` is a later milestone).

## [0.27.1] - 2026-09-14

### Fixed

- **`sokonanoda: restart server` no longer silently keeps a stale server** —
  it now re-resolves through the same path as activation, including the
  version-pinned download fallback, and aborts with a message instead of
  restarting the old binary when no usable server is found (a stale cache
  marker is rejected, not reused). It also detects a newer installed extension
  and tells you to reload the window, since extension-code upgrades cannot be
  picked up by a server restart.

## [0.27.0] - 2026-09-14

### Added

- **Multi-goal display** — after a tactic that opens several sub-goals (for
  example `apply And.intro`), the cursor goal view and the exercise panel now
  list **every** remaining goal, current first, each with its own hypotheses,
  instead of showing only the top goal. The server records the full open-goal
  list per tactic step and exposes it as `goals[]` in `soko/stateAt` and
  `soko/goals`; the single `goal`/`binders` fields remain for older clients.
- **Tactic goal-state hover** — hovering a `by` tactic (anywhere in its source
  span) shows the goal state entering that tactic — every remaining goal with
  its hypotheses, Infoview-style. It reuses the per-tactic snapshot
  (`by_steps`), so no re-check happens on hover.
- **Tactic keywords are highlighted** — `by`, `exact`, `assumption` and `rfl`
  are now classified as keywords (previously they colored as plain
  identifiers).

### Removed

- **Value-position keywords** — `funintro` (and the earlier `funapply`) are
  gone. The value position now accepts only a plain expression or a
  `by <tactic>; …` block; `funintro` is no longer a keyword, so it parses as an
  ordinary identifier. The completion/hover expand button, the
  `sokonanoda.expandIntro` command, and the `elab-intro-not-a-function` error
  code were removed with it. The `by` tactic `intro` is unchanged.

## [0.26.0] - 2026-09-14

### Added

- **Course unit 7 — quantifiers (`forall` & `exists`)** — a new bilingual
  canvas with seven kernel-graded exercises: universal introduction
  (`fun`) and elimination (application), existential witnesses and
  `Exists.elim`, the two distribution laws with `And`, `∀ → ∃` on a
  non-empty domain, and existential monotonicity. The unit carries its own
  logic skeleton and `Exists` axioms, plus an English mirror, agent answer
  keys, golden event counts, and a `#reduce` self-test. The learner canvas
  `playground.sokonanoda` gained the same lesson.

## [0.25.0] - 2026-09-14

### Fixed

- **`sorry` hover shows the precise expected type** (user report on
  playground exercise 5) — for `(And.right a (Not a) x) sorry` the hover
  now says the hole expects `a` (computed by unfolding `Not a` via the
  `Not` definition to `a -> False` and taking the arrow domain), instead
  of the whole declared type. The goal walk now handles **over-applied
  spines**: arguments beyond a function's declared telescope are matched
  against its result type, unfolding simple `def` bodies one step at a
  time. The remaining goal (`False`) and the local hypotheses (`a`,
  `x : And a (Not a)`) are shown alongside.

## [0.24.0] - 2026-09-13

### Development / Infrastructure

- **Performance tests are now routine** (user requirement: performance is
  the project's lifeline) — 6 threshold-sentinel tests run on every push:
  compiler scaling (400 blocks ≤ 12× the time of 50; O(n²) trips it),
  incremental editing (<50ms per keystroke, only the edited block is
  kernel-checked), and LSP interaction latency (didChange <50ms;
  completion/hover/goal view <10ms). Every push also uploads a
  `perf-report` artifact (version + commit) so regressions can be traced
  to the change that caused them. No user-facing behavior change;
  the version bump exists so each round of work has its own perf report.

## [0.23.0] - 2026-09-13

### Added
- **Half-expression goal state** — hovering a partial application the kernel
  rejected (e.g. `And.intro b a` against `And b a`) now lists the inferred
  remaining goals (`|- b`, `|- a`) instead of only the error. Computed on
  hover only, with a bounded fingerprint cache.

### Performance
- **Judge results are cached** (fingerprint-keyed, capped at 128) — the
  by-block tactics `apply`/`exact` infer types through a full document-prefix
  recompile on every keystroke; unrelated edits now hit the cache. This was
  the same O(n²) pattern that got value-position `funapply` removed.

## [0.22.0] - 2026-09-13

### Removed
- **Value-position `funapply`** — its lowering asked the kernel to infer the
  applied term's type, which re-compiled the entire document prefix on every
  keystroke (O(n²)); the interaction was unusably laggy (user report). The
  by-block tactic `apply` is unchanged. Course unit 6's funapply aside and
  exercises were removed with it (goldens reverted to (13, 6, 0) / 33-27).

### Changed
- **`funintro` skeleton lands with `sorry` selected** — accepting the
  completion inserts the skeleton as a snippet whose trailing `sorry` is
  preselected, so the next input overwrites it directly.

## [0.21.0] - 2026-09-13

### Added
- **`funintro` (funapply X) composition** — the keywords are now first-class
  expressions: `funintro (funapply And.intro)` peels every remaining binder and
  lowers `funapply And.intro` against the final goal
  (`And.intro b a sorry sorry`). Pure front-end; the kernel never sees a
  keyword.
- **Completion while typing** — the 「替换源代码」 item now appears from the
  first keystroke of the keyword (previously the popup was empty until the
  whole expression compiled). While the keyword is incomplete the item
  completes the word; once the argument compiles it upgrades to the full
  skeleton replacement.

### Changed
- **Breaking (teaching surface): value-position keywords renamed** —
  `intro` → `funintro`, `apply` → `funapply`, to remove the ambiguity with the
  tactic versions inside `by` blocks (which are unchanged). Course unit 6 and
  the playground were updated in the same release. Error codes keep their
  historical names (`elab-intro-not-a-function`, `elab-apply-*`); the
  human-readable hints use the new names.

## [0.20.0] - 2026-09-13

### Added
- **`intro` / `apply` now work inside a `fun` body** — the natural place a
  learner reaches after introducing some binders by hand:
  `theorem t : Q -> P := fun (x : Q) => apply proofP` (implicit replacement)
  and `theorem and_swap : … := fun (a : Prop) => fun (b : Prop) => fun (x :
  And a b) => intro` (introduces the rest). Previously the keywords were only
  recognised at the very start of the value, and inside a `fun` body `apply`
  was an ordinary identifier (`unknown identifier`).
- **`by` also works at the tail of a `fun` body** —
  `theorem t : Q -> P := fun (x : Q) => by exact proofP` enters tactic mode
  with the lambda binders as the initial context; `by_steps`, goal view and
  kernel judging behave exactly like a value-position `by`.

### Fixed
- Command-palette entries no longer print `sokonanoda: sokonanoda: …` — the
  titles of the four commands that carried a `category` lost their redundant
  `sokonanoda:` prefix (`restart server`, `揭示下一条提示`, the two expand
  commands).

## [0.19.0] - 2026-09-13

### Added
- **`soko/version`** — the server reports its version and process id. The
  `sokonanoda: restart server` command now asks before and after, so the
  receipt shows `0.16.2 (pid 1001) → 0.19.0 (pid 2002)`: proof that the old
  process died and the new one is the new version.

### Changed
- The command palette entry is now `sokonanoda: restart server` (was
  `sokonanoda: 重启语言服务器`).

### Added
- **Value-position `apply`** — the second teaching keyword next to `intro`.
  `theorem t (h : Q -> P) : P := apply h` applies a proof/function to the
  goal and leaves its premises as holes (`h sorry`). Type parameters are
  filled from the goal automatically; premises you already supply
  (`apply (f p)`) are not duplicated. Completion, hover with the
  「展开为 apply 骨架」 button, and the "keeping it is equivalent" note all
  mirror `intro`'s; the editor command is `sokonanoda.expandApply`.
- New teaching error codes `elab-apply-needs-a-term` and
  `elab-apply-not-applicable`.
- **`intro` accepts an optional answer** — `theorem t : Q -> P := intro proofP`
  implicitly replaces the keyword with `fun (x : Q) => proofP`, so finishing a
  proof no longer requires expanding first. The answer must prove the final
  goal; the kernel still judges it.
- The hover expand-button payload now follows VS Code's command-link shape
  (a JSON **array** of arguments). The previous object payload made the
  button click silently do nothing; the client handler accepts both.

### Fixed
- Inlay hints no longer show the **first** sub-goal's type on every hole of
  a `by apply …` declaration that leaves several sub-goals open — those
  holes share one source position, so the lookup had to be positional
  (`[": p", ": p"]` → `[": p", ": q"]`).


## [0.17.0] - 2026-09-12

### Added
- **Hover `intro` → one-click expansion.** The hover on a value-position
  `intro` now carries an 「展开为 fun 骨架」 button that applies the same
  in-place edit as accepting the Tab completion — no need to catch the
  suggestion popup. The payload (document, hole range, skeleton) is computed
  by the language server and applied verbatim, so the button and Tab can
  never drift apart.

### Fixed
- The `intro` expansion (completion **and** hover) now survives the caret
  sitting just after the keyword — a trailing space on the same line, or the
  caret resting at the end of the token. Previously only the exact token byte
  range matched, so a wrapped `:=` … `intro` line looked like it "stopped
  working".

### Changed
- The `intro` hover now says outright that **keeping `intro` is equivalent**
  to the expanded `fun … => sorry` skeleton — expanding is a convenience,
  not a required step.

## [0.16.2] - 2026-09-12

### Added
- Hovering the value-position `intro` keyword now shows its explicit expansion
  (`fun (a : Prop) => … => sorry`), so the skeleton stays visible even when
  the completion suggestion is not accepted.

### Fixed
- Regression coverage for the real interaction: type `intro` and accept the
  selected suggestion — the token is replaced by the explicit skeleton. Also
  covers the end-of-token completion position fixed in 0.16.1.

## [0.16.1] - 2026-09-12

### Fixed
- The value-position `intro` expansion completion now appears when the caret
  is at the **end** of the keyword — the moment you finish typing it. The
  declaration lookup used an end-exclusive range, so the item was only
  offered with the caret strictly inside the token, which never happens while
  typing.

## [0.16.0] - 2026-09-12

### Added
- **Restart the language server in place** — the new command
  `sokonanoda: 重启语言服务器` re-resolves the server binary (rebuilt
  workspace build, refreshed cache, or a changed `sokonanoda.serverPath`) and
  restarts the client, so the editor picks it up without reloading the
  window. Extension-code updates still apply on window reload.

## [0.15.0] - 2026-09-12

### Added
- **Declaration binders (Lean-style)** — `theorem f (a : A) (h : B a) : C := v`
  is now valid: the declared type becomes the matching arrow telescope and the
  value gets its `fun`s wrapped automatically. Writing `:= sorry` reports the
  codomain goal with the binders already in context (no `intro` needed), a
  closed body needs no `fun`s either, and `by` blocks start from that context.
  Universe params and implicit binders stay unambiguous
  (`{u}` vs `{x : T}`).
- Course unit 1 gained a side-by-side section (arrow spelling vs declaration
  binders) plus exercise 6.

## [0.14.0] - 2026-09-12

### Added
- **Value-position `intro`** — write `intro` as the whole value
  (`theorem t : (a : Prop) -> a -> a := intro`) and the compiler unrolls every
  remaining binder into `fun … => sorry` (anonymous layers are named `x`,
  `x2`, …). It is a legal open exercise; the kernel still judges every fill.
  A goal with no binder left is rejected as `elab-intro-not-a-function`.
- **Expansion completion** — when the caret is on that `intro` keyword the
  completion list offers `intro（展开为 fun 骨架）`; accepting it replaces the
  keyword in place with the explicit skeleton. The hole's inlay hint shows the
  remaining goal right after `intro` (e.g. `: And b a`).

### Fixed
- Pretty-printed goals and binders no longer wrap application heads in a
  redundant parenthesis (`(And a) b` → `And a b`), matching the kernel's own
  printer and the learner's source.
- Comment-only edits no longer leave stale hole / sub-goal spans in
  incremental sessions; they are remapped together with the other cached
  spans.

## [0.13.0] - 2026-09-11

### Added
- **Onboarding lives in the CLI** — `sokonanoda` now ships
  `version` / `doctor` / `setup` / `update` / `grade` / `gate` subcommands
  (plus the existing `lsp`), replacing `scripts/soko.sh`. `setup` / `update`
  fetch the version-pinned CLI + LSP with a built-in downloader (no shell,
  cross-platform); `version` / `doctor` report the cached binaries' markers
  (`--json`); `grade` batches the `--json` judge view.

### Changed
- The cache version marker (`<version> <target>`) is shared with the VS Code
  extension and the opencode plugin; a stale or missing cache is refreshed
  from the pinned release (never `latest`).

### Removed
- `scripts/soko.sh` (superseded by the binary subcommands).

## [0.12.0] - 2026-09-11

### Added
- **`Type n` universe notation** — `Type n` is now accepted as
  `Sort (n + 1)`, Lean's spelling: `Type 0` = `Sort 1`, `Type 1` = `Sort 2`,
  and a bare `Type` stays `Sort 1`. `Type u` (a universe variable) is not
  supported — use `Sort u`.

### Fixed
- Reworded the `reserved-declaration-name` warning to plain language: it now
  says `Prop` / `Sort` / `Type` are already defined by the kernel (and notes
  `Prop`'s special role in formal proofs) instead of the coined term
  "built-in sort".

## [0.11.0] - 2026-09-11

### Added
- **Warns on declarations that reuse a kernel-defined name** — `Prop`, `Sort`
  and `Type` are already defined by the kernel, so a top-level declaration
  with one of those names still compiles but is never used. The editor now
  shows a `reserved-declaration-name` warning on the name, and the CLI emits
  a matching `warning` event.

## [0.10.0] - 2026-09-10

### Added
- **`#check` results stay visible** — like Lean's Infoview, the editor now
  shows each `#check` result as an inlay hint right after the checked
  expression (`#check Nat` → `Nat : Type 0`). The result comes from the
  same kernel pass that grades exercises, and survives incremental edits.

## [0.9.1] - 2026-09-10

### Fixed
- **Greek binder letters stay plain** — VS Code's confusable-character
  highlight (Trojan Source protection, on by default) drew a box around
  `α`/`β`/`γ` in `.sokonanoda` files. The extension now ships a
  `[sokonanoda]` configuration default that turns
  `editor.unicodeHighlight.ambiguousCharacters` off for this language only
  (the same approach VS Code itself uses for plaintext and markdown). Your
  global settings are untouched.

## [0.9.0] - 2026-09-10

### Added
- **CLI bundled too** — each platform package now ships the `sokonanoda` CLI
  next to the language server, so the 「课程」course map works out of the box
  (no separate `cargo build`, no PATH setup). Standalone per-platform CLI
  tarballs are also published on GitHub Releases for headless / agent use
  (`sokonanoda-cli-<triple>.tar.gz`, version-pinned).

### Changed
- CLI discovery is now: bundled `bin/<target>/sokonanoda` → workspace
  `target/{debug,release}` build → `PATH`.

## [0.8.0] - 2026-09-10

### Added
- **More platforms** — bundled packages now also cover `linux-arm64`,
  `alpine-x64`, `alpine-arm64` and `win32-arm64` (nine packages in total, like
  other major language extensions), so ARM and Alpine users get the
  zero-download experience too. Linux binaries are now built with a **glibc
  2.28 floor** (VS Code's own Linux minimum), fixing installs on older
  distributions; Alpine binaries are statically linked musl.

### Changed
- The universal fallback package now only serves platforms without a bundled
  build (e.g. Linux armhf).

## [0.7.0] - 2026-09-10

### Added
- **Bundled language server** — the `sokonanoda-lsp` binary now ships inside
  the extension as a platform-specific package (macOS arm64/x86_64, Linux
  x86_64, Windows x86_64). Installing the extension is now truly zero setup:
  no GitHub download on first use, works offline, and the kernel version is
  always the one the extension was built and tested with — no more client /
  server version skew.

### Changed
- Server discovery is now: `sokonanoda.serverPath` / `SOKONANODA_LSP_BIN` →
  bundled `bin/<target>/` → workspace `target/` build → version-pinned cached
  download. A lost executable bit on the bundled binary is repaired
  automatically.
- The fallback download (used only by the universal package for platforms
  without a bundled build) is pinned to this extension's own release tag
  instead of `releases/latest`, so it can never pull a newer, incompatible
  server.

## [0.6.0] - 2026-09-10

### Added
- **Goals at cursor** (「当前光标处」): with the caret inside a declaration the
  exercise tree now shows the goal, the hypotheses in scope and your `by`
  progress at that position — move the cursor and the panel follows
  (debounced ~200 ms). Clicking the goal reveals the corresponding tactic.
  Powered by the new `soko/stateAt` request; position → tactic selection is
  decided by the server, the client only renders.

## [0.5.2] - 2026-09-09

### Fixed
- **Wrong `by`-block / `sorry` span ranges**: `parse_by_block` ended the block
  span at "the next token" — comments are skipped by the lexer, so the span
  ballooned across trailing multi-line comments into the next declaration (or
  EOF), and the `sorry` warning squiggle covered those comment lines. The block
  span now ends at the last tactic, and the unclosed-goal hole points at the
  `sorry` token instead of offset 0.

## [0.5.1] - 2026-09-09

### Fixed
- **Stale language-server binary after extension updates**: the auto-downloaded
  `sokonanoda-lsp` was cached forever with no version check, so after upgrading
  the extension it still ran an older server (e.g. `by sorry` reported as an
  unknown tactic). The download is now version-tracked: when the extension
  version changes, the server is re-downloaded from the latest GitHub Release.
- **Axiom connectives highlight as types**: `And`/`Or`/`True`/`False` (declared
  via `axiom`) now map to the semantic-token `type` color instead of the nearly
  invisible `variable` color, matching Lean's treatment of the logical
  vocabulary.

## [0.5.0] - 2026-09-09

### Added
- **`by` tactic blocks** (Lean-style proofs): `theorem t : T := by intro a; exact h`
  with five tactics — `intro` / `exact` / `apply` / `assumption` / `rfl` — and a
  `by sorry` placeholder for incomplete proofs. Every tactic is kernel-judged.
  New course unit 6 (zh + en) teaches it; playground gains two `by` exercises.
- Hover now returns a highlight range so the editor shows which expression a
  hover describes (well-formed, paren-balanced expressions, real binder names).

## [0.4.2] - 2026-09-09

### Changed
- Marketplace listing rewritten to match reality: zero-setup story (the
  server downloads itself from GitHub Releases on first use, rust-analyzer
  model), full feature inventory, agent-skills promotion (teacher/dev/ci)
  and the `opencode.json` wiring; stale "build the server with cargo"
  quick start removed
- Keywords refreshed for marketplace discovery

### Docs
- New hard rule in docs/vscode-dev-guide.md §7: the three marketplace
  files (README.md / description / CHANGELOG.md) must be updated in the
  same commit whenever install story, feature set, feedback behavior or
  agent integration changes

## [0.4.1] - 2026-09-09

### Fixed
- Bracket hover: `(expr)` shows `expr : type` on both `(` and `)` (paren
  matching with `--` comment skipping); no longer leaks the neighbor's
  signature on `)` (was `And.left : forall …` bug)
- Hover types show real binder names instead of de Bruijn indices
  (`$3 -> $4` was leaking on partially applied functions)
- `Not a` stays folded in hover (was unfolded to `a -> False`)
- Double hover fallbacks consolidated; `goto-def` on `)` no longer jumps
  to a neighbor identifier

### Changed
- Kernel display layer (cold path, documented in architecture.md §6):
  pp binder-name seeding for scope variables; `infer_under_binders` quotes
  without `force_all`

## [0.4.0] - 2026-09-09

### Added
- Hover proximity fallback: brackets/operators show enclosing expression type
- VS Code integration tests (@vscode/test-electron, 4 cases + CI xvfb)
- Multi-binder lambda regression test

### Fixed
- Hover loose bvars resolved to binder names (was "1 -> 1")
- Keyword hover suppressed (fun/=>/theorem silent)
- Conv soundness fix (kernel eval/infer closure conflation)

### Changed
- Hover precision limitation documented: infer_under_binders panics on
  delta-unfolding types (Not a), affected rows dropped (宁缺毋滥)

## [0.3.0] - 2026-09-09

### Added
- Hover on brackets/operators shows enclosing expression type (proximity fallback)
- Declaration hover shows full kernel-rendered signature (ty_text)
- Hover on sub-expressions shows `expr : type` (precedence visible)
- Keyword hover suppressed (clean, no noise on fun/=>/theorem)
- Multi-binder lambda support confirmed and regression-tested
- `sorry` highlight in semantic tokens and TextMate grammar

### Fixed
- Hover loose bvars resolved to binder names (was showing "1 -> 1")
- Hover rows with unresolvable names dropped (宁缺毋滥，不展示乱码)
- Conv soundness fix (upstream kernel bug: eval/infer closure conflation)

### Changed
- `???` removed; `sorry` is the sole placeholder (Lean 4 parity)


## [0.2.1] - 2026-09-08

### Changed
- CI auto-publishes to VS Code Marketplace on tag push
- Version bump to test release pipeline
## [0.2.0] - 2026-09-07

### Added
- Go-to-definition, document highlight, binder completions
- Exercise panel (练习 tree) with goal/hypotheses/hole navigation
- `alt+n` / `alt+shift+n` hole navigation
- Semantic highlighting
- `soko/goals` + `soko/nextHole` custom LSP requests
- Inlay hints (expected types at hole positions)
- Rename + find references
- Hint ladders (`-- soko:hint` directives, revealed one at a time)
- `sorry` as placeholder (Lean 4 parity, ??? removed)
- Multi-hole constructor spines with refine suggestions

### Changed
- Declaration hover shows full kernel-rendered signature
- Hover on sub-expressions shows `expr : type` (precedence visible)
- Keyword hover suppressed (clean, no noise)
- `sorry` produces warning-level diagnostic (not error)

### Fixed
- Conv soundness fix (upstream kernel bug: eval/infer closure conflation)
- Loose bvar rendering (hover showed "1 -> 1" instead of named binders)
- VSIX packaging (vscode-languageclient now properly bundled)

### Removed
- `???` placeholder (replaced by `sorry` for Lean 4 parity)

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-07

### Added

- Initial LSP feedback channel for `.sokonanoda` files: diagnostics, hover, goal view, CodeLens, semantic tokens, and code actions.
