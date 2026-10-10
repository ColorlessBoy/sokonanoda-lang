## [Unreleased]

> **One card for every hover, an honest artifact version, and quieter declaration names.**

### Fixed

- **报错不再打死符号跳转（G-108，用户实测）**：`sorry` 或一条失败的 tactic 以前会让**同一份文件里正确代码**的
  F12/跳转失效（`Eq.refl` 返回空）—— 原因是那段源码根本没有 hover 行（开放练习只 elaborate 签名、失败路径把已收到
  的行丢掉）⇒ 现在给"没走到检查的那段"补**词法行**（不推类型、不编造；**判定不变**），并且悬停这些名字时
  给的是**同一张声明卡片**（与别处 hover 同名同形，而不是只显示一个名字）。
- **`apply` 支持定义展开（defeq）对齐 Lean 4（G-109，用户实测）**：`apply h; apply Eq.refl a` 在目标
  `a ∈ ({a})`（与 `a = a` **defeq**）上以前报「目标不匹配」而 `exact` 能过 ⇒ 现在失败路径会**递归展开**目标/结论头，
  再不行就复用内核判等（`exact` 那条）闭合目标。**今天能过的 `apply` 一字不变**（只在原本报错的输入上多走一趟）。

- **`#check`/`#print` 的输出不再随编辑累积重复**（用户报「2→3→4→19 份」）：除了产出侧的两道闸
  （命令号进序列化 + 拼接按位置幂等），**回放侧**也加了闸 —— 从 `<模块根>/.sokonanoda/compiled/`
  或全局缓存读回来的报告，进场前先把「同一条命令两份结果」收干净。判据：真宿主 e2e
  `C3 重复输出（产物回放）…`（冷编 → **重启服务器走产物回放** → 连编辑两刀 → 每步 `#check`/`#print`
  各恰好一条），以及 CDP DOM 层 `docs/E2E.md` §8 的逐步读数。
- **「由编译器 X 写入」不再说谎**（用户报「0.87.2 编完、装上 0.87.3 还没 rebuild 就显示 0.87.3」）：
  写者现在记在**产物条目自己**（文件名），面板照实显示；当产物不是当前编译器写的（那些条目
  也**不可能被命中**）时，后面会提示「建议 Rebuild」。
- **所有 hover 收口成同一张声明卡片**（用户报「`#check` 的 hover 缺 def 头与 `:=` body」+
  「使用处（`h : A ⊆ B`）也要完整信息……数据收口到一处」）：声明名、任何使用处、`#check X`、
  `#print X` 现在给**逐字节相同**的卡片（签名块 + `def`/`opaque` 的 `:=` 值块）；`#check` 的
  非名字形态（`#check fun x => x`）仍是 Lean 的 `表达式 : 类型`。
- **声明列表里的名字收成链接式文字**（用户报「css 比较抢镜，暗色主题尤其刺眼」）：正常字号、
  不加粗，去掉按钮默认的亮灰底与边框，用主题链接色 + 下划线表达"可点"（点击行为不变）。

## [0.87.3] — 2026-10-10

> **Hover is one surface now, and the Infoview's command output stops duplicating.**
> Every piece of `.sokonanoda` text in a hover (a tactic's name type, notation
> signatures, a declaration's signature, a `def`'s value) is rendered through the
> same fenced code block, so it gets the same syntax colours as the goal state.
> Hovering a `#check`/`#print` line shows that command's own output — the same
> truth the Infoview's 「命令输出」 block shows. `def` declarations now show both
> their type and their `:=` body. And the command output no longer duplicates on
> every edit (it did grow by one per edit, because a replayed project artifact
> lost the command attribution the incremental splice de-duplicates on).

### Fixed

- **hover 的语言文本统一走围栏块**（用户报「不是从统一渲染接口获取的，也没有正常的代码高亮」）：
  tactic 里名字的类型行、记法符号/目标名/内建登记名的签名行，以前是**行内码**
  （VS Code 不给行内码上色）⇒ 现在一律是**独立的 ```sokonanoda 围栏块**，与
  tactic 行 / goal state / 声明卡片**同一个 `code_block`**。判据：LSP hover
  5 条结构断言（围栏 + `名字 : ` + 非空类型；**不锁类型字面**）+ 真 LSP 复现件
  `docs/gaps/repro/G103-hover-unified-rendering.sh`。
- **`def` 的 hover 带上 `:=` 值块**（用户：「hover 信息应该把类型和 `:=` 后面的含义块
  也显示出来……跟 infoview 里面的声明列表对齐」）：类型块之后多一个 `:= <值>` 围栏块
  （与 Infoview 的 `decl-val-line` 同语义）；`theorem`/`axiom`/`inductive` 的 `val_text`
  是 `None` ⇒ **逐字节不变**。
- **`#check` / `#print` 那一行的 hover**（用户报「`#check Eq.refl` 只有一个高亮的
  `Eq.refl`，没有有效的类型信息」·「`#print Set.singleton` 上没有 hover 信息弹出」）：
  光标落在命令行上 ⇒ 显示**那条命令自己的输出**（真相层 `QueryDoc::messages_at`，
  与 Infoview 的「命令输出」块**同一份真相**）。`#print` 那一行以前**一个 hover 行
  都没有**（`walk.rs::print` 从不 push `cmd_hovers`）⇒ 完全静默。`#check Eq.refl`
  的真根因是**内核 pp 在未解层元变量上 panic**、被 `resolve_hovers` 静默吞成空文本
  ⇒ `leq_core` 按 Lean 4（`level.h:57` / `level.cpp:507`，无 panic 路径）补齐 `MVar`
  臂（**只覆盖以前 panic 的输入**）。
- **Infoview 的「命令输出」不再每编辑一次 +1**（用户报「每编辑一下，`#check` 和
  `#print` 就会多重复一次」，现场 3 → 4 条）：根因 = 项目产物/全局缓存**回放**时
  `CheckInfo::cmd`/`PrintInfo::cmd`（`#[serde(skip)]`）**归零**，而增量拼接按 `cmd`
  去重 ⇒ 缓存那份永远留下 + 新查那份照样追加。修法 = `cmd` **进序列化**
  （`REPORT_SHAPE` 4 → 5，旧条目整库不命中）+ 拼接加**与 cmd 无关的幂等闸门**
  （按源位置去重、留 fresh 那份）。判据：front 两条 + 真 LSP/真产物复现件
  `docs/gaps/repro/G102-command-output-not-duplicated.sh`（修前 `["check","check"]`）。

## [0.87.2] — 2026-10-10

> **Four reported editor-feedback items are closed — plus two seams found while fixing
> them.** The prelude's own source now compiles clean (it never did) and a guard keeps it
> that way; `=` has a go-to-definition target and the target names inside the prelude's
> registration comments are navigable; `rfl` accepts `Iff` goals whose sides are
> definitionally equal (matching Lean 4, where `rfl` is literally `exact Iff.rfl`) and
> `Iff.rfl` exists; hovering a name inside a tactic shows that name's type after a
> divider. While fixing those, two seams were found and closed: hovering a name could
> show a **stale** type after you edited the constant's signature, and `soko gate --fast`
> never actually ran the second crate's unit tests.

### Fixed

- **prelude 的生效源自己编译不过**（用户报「「prelude」文件自己都编译不过自己」）。
  `prelude/L1.sokonanoda` 的 `Classical.byContradiction` 里 `Or.elim` **漏了动机位 `c`**
  （`Classical.em p` 被塞进 `f`）⇒ 那份源 43 条声明里 1 条判红，而它是**受信任安装**
  （装进环境后内核从不重查）⇒ 体的类型错误被静默装进环境，**没有任何判据**判过它。
  修法 = 只修体 + **新守卫**把**生效源**（F12 打开的那一份）当普通文档判（0 诊断 +
  声明计数），与 `scripts/soko grade` 同一条通道。判据：`grade` exit 1 → **exit 0 /
  43 checked / 0 诊断**；**反向验证**：把体改回错的 ⇒ 守卫当场判红 ✓。
- **`=` 上没有 F12 · 指令登记注释行里的目标名不可跳 · 内建记法的 hover 说假话**。
  prelude 登记区补 `-- sokonanoda:builtin-notation "=" => Eq`（与另外 5 条对称；**词法
  符号表一行未动** ⇒ `=>` 的分词由 R-2 用例钉住）；`notation_target_at` 同时认**注释
  形态** ⇒ 注释里的目标名走已有真相通道落到**定义那一行**；hover 删掉「没有源码声明，
  `F12` 无处可跳」（E10 之后它与 `definition` 当场自相矛盾）。判据都**断言落点行号 +
  非自跳**（E05/G-37 的教训），**反向验证**：删登记行 ⇒ front + LSP 逐字判红 ✓。
- **`rfl` 在 `↔` 形状的目标上不可用**（用户报「这类都不能使用 rfl」）。**对齐 Lean 4**：
  `rfl` 接受 `Iff` 头（候选 `Iff.refl`，**仍由内核裁决** —— 两边不 defeq 的 `a ↔ b`
  **仍判红** ✓）；补全与 quick-fix 在 `Iff` 形状目标上也给**内核验证过的** `Iff.refl`；
  形状错误文案改成**指出出路**；并按 Lean core 加上 **`Iff.rfl`**（Mathlib 的
  `mem_empty_iff_false` 原文就是它）。
- **tactic 里的名字上 hover 没有它本身的类型**（用户报「我希望 hover 除了 goal state，
  分割线后再加上 `Set.ext` 本身的类型」）。光标在**名字**上 ⇒ goal state + Markdown
  水平线 + 该名字的类型（折记法）；在 **tactic 关键字**上**一个字节都不加**（与改前
  逐字节相同）；拿不到干净类型（`$N` / unknown）⇒ **不编那一行**。`range` 仍是整条
  tactic（写进 `docs/protocol.md`）。
- **改完签名再 hover 会说旧签名**（做上一条时横向排查逮到的**假话**）：hover 里问常量
  签名走的是进程级**名字键**缓存（没有失效路径）⇒ `def myop : Nat := 0` 改成
  `def myop : Bool := …` 之后，goal state 是新的而名字那行仍是 `Nat`。修法 = `lib.rs` 里
  **所有 hover 路径**改走**前缀键**的查询（零性能代价）。

## [0.87.1] — 2026-10-09

> **Three reported defects are closed, and one gesture now matches Lean 4.** `#check`
> output no longer repeats or accumulates (it grew by one copy per compile — 19 copies
> after a few exercises); `#print` answers in project files (entries with an `import`
> showed nothing at all); and command output in the Infoview gets the same notation
> folding and syntax highlighting as every other card. Inlay hints stop echoing `#check`
> (the Infoview block is the complete view), the goal panel reads conditions-then-goal
> like Lean 4, the blank line after a proof still belongs to its declaration, and typing
> `\alpha` followed by **space** becomes `α` (`sokonanoda.input.eager` now defaults on;
> `Tab` stays as an internal explicit path). Patch semantics: no new protocol and no new
> capability — existing features became correct and usable.

### Fixed

- **`#check` 的输出不再重复、也不再随每次操作累积**（2026-10-09 用户报「重复输出 2 次」、
  「操作几下后直接输出 19 次」；第二次拍板给了精确触发条件：「**做完后面几道题目之后，
  回头把光标移动回 `#check` 那一行**」）。根因在增量拼接 `front::query::splice_entry_report`：
  一条**被信任**的命令，它的结论**可能**已经在这一趟的 `fresh` 报告里（没有可用快照时前缀
  照走，`#check`/`#print` 这类命令会重新 elaborate 并产事件），也可能**只**在缓存里
  （T2-B 命令级快照命中 ⇒ 前缀整段不走查）——旧代码无条件把缓存那份补回去，于是"两处都有"
  时**同一条命令算两遍**，每编译一次 +1（`#check`、`#print`、那一行的 hover、以及它产生的
  **诊断**全都长）。修法 = 拼接的唯一口径改成「**`fresh` 里已经有的命令，不再从缓存补**」：
  谁算过谁说话，缓存只负责这一轮没算的那些命令，与走哪条路无关。判据（两个相反方向都钉）：
  front `a_check_in_the_resumed_prefix_is_spliced_back_exactly_once`（用户的形状：`#check` 在最上面、
  后面几道题、来回改）· front `repeated_edits_keep_command_outputs_single`（相反方向：
  `#check` 在改动点之后）· LSP `revisiting_the_check_line_never_appends_a_second_output`
  （真项目 + 连续编辑 + **纯光标往返 + hover**）· 真宿主 e2e「C3 重复输出…」。
  **反向验证**：撤 `checks` 过滤器 ⇒ (2,1,10) 判红 · 撤诊断按位置去重 ⇒ 2 条判红 ·
  撤 hovers 过滤器 ⇒ 11 vs 10 判红。
- **`#check` 的行内提示（inlay hint）已去掉**（2026-10-09 用户：「**inline 提示已不需要**，
  `#check` 尾部仍带且看不全、无意义」）。`textDocument/inlayHint` 现在**只**答开放练习的
  洞的期望类型；命令输出在 Infoview 的「命令输出」块里完整可见（含记法与高亮），行内那一截
  只会被行宽截断。判据：`check_results_are_not_inlay_hints`（带 `sorry` **正对照**——否则
  "没有提示"可能是因为整条 inlay 路坏了 ✗）。**反向验证**：把 `#check` 那段循环放回去 ⇒
  3 条 vs 1 条判红。
- **输入 `\alpha` 现在敲完就是 `α`：按【空格】自动转换，不再依赖 `Tab`**（2026-10-09 用户
  拍板：「对齐 Lean 4 —— 输入 `\a` 或 `\alpha` 后按【空格】即自动转成 α，不要依赖 +Tab」；
  并指出「原 `+Tab` 触发在编辑器 UI 里**没有任何提示**、用户全程不知情」本身就是暴露问题）。
  `sokonanoda.input.eager` **默认翻成开**：**分隔符**（空格/标点）封口即换
  （`\alpha ` → `α `、`\a ` → `α `、`\in ` → `∈ `），而"不再是任何更长缩写的前缀"的词敲完
  即换（`\alpha` → `α`）。`Tab` 键位**保留**为内部显式路径（`input.eager: false` 时它是唯一
  一条），但**不再是对用户教的姿势**：README / 设置说明 / teacher 技能一律改教【空格】。
  判据：真宿主**真按键** `scripts/vscode-input-e2e.mjs`（逐字符 keydown + 真 DOM + 存盘字节）
  —— `\alpha `→`α ` · `\a `→`α ` · `\and `→`∧ ` · `\in `→`∈ ` · `\alpha` 敲完即换 ·
  一次 undo 回到 `\in` · 对照臂（Tab 仍缩进 / `.txt` 不动）· 逃生门（`false` 时空格不换、
  Tab 才换）；stub 宿主 `Space closes the word…` + `eager replacement is the default, not Tab`；
  清单契约 `notation_input_tab_binding_is_gated_by_its_context_key`（默认值 = true）。
  **反向验证**：把清单默认值改回 `false` ⇒ Rust 契约与 stub 两条同时判红 ✓。
- **命令输出与面板其它内容同一套观感：记法 + 语法高亮**（2026-10-09 用户报「`#check` 在
  infoview 里的打印缺少 notation 显示和语法高亮」）。「命令输出」块以前把 `text` 画成一色
  `<pre>`；现在走**与目标/条件/声明卡片同一个** `codeBlock`（`runs` 分段 + `tok-*` 上色），
  `#check` 的**类型那一半还会折记法**（`->` ⇒ `→`，与目标行一模一样）。分段由服务端的
  **唯一显示接口**产出（`display.fold` + `display.runs`，与 `goal_runs` 同源），
  扩展不重新分词、也不硬编码样式。
- **`#print` 在带 `import` 的文件里不再「完全没有反应」**（2026-10-09 用户报）。报告组装
  有两处只映射了 `#check`：项目模式走的 `run_pass_with` 组装段（`prints` 恒空）与增量
  拼接 `splice_entry_report`（新查那一段的 `prints` 没拼回来）⇒ 课程文件（**都有**
  `import`）里 `#print` 什么也不显示。两处补齐后，入口自己的声明与**库里**的声明都能
  打印，`def … := fun (n : Nat) => …` 这样的 **lambda 定义会打出它的 λ 本体**。
  报告形状随之 **3 → 4**（旧缓存条目整库不命中，不会静默给「没反应」的旧答案）。
- **目标栏与 Lean 4 对齐：条件在上、目标在下**（2026-10-09 用户报）。`renderGoals()` 原来先渲染
  目标行（`⊢ 目标`）再渲染假设列表 ⇒ 与 Lean 4 Infoview 正好相反。现在顺序是
  目标头（`目标 i/n`，可点 reveal，对应 Lean 的 `case` 行）→ 假设（`h : A`）→ `⊢ 目标`；
  假设的 `name : type` 冒号照旧。
- **无目标只有一句文案：`🎉 已无目标 ✓`**（2026-10-09 用户拍板）。以前"证明在末条 tactic
  闭合"会额外道贺（`🎉 恭喜，证完了（Q.E.D.）`），尾部还有 `sorry` 的声明则是中性的
  `已无目标 ✓` ⇒ 同一格两种字样。现在 Infoview 与「当前光标处」树项**同句**
  （"证完了"是替学习者下的判决，"已无目标"才是看得见的事实）。
- **声明体之后的空白行仍算这条声明**（`soko/stateAt`）。光标停在证明末尾下面的空行/行尾上时，
  面板以前说「光标不在任何声明内。」；现在照旧显示该声明的剩余目标或「🎉 已无目标 ✓」。
  边界：下一行是 `#check`/`#print` 时**仍然**不算（那一行的「命令输出」块靠这条路）。
- **产物版本号不再显示成裸问号**。老产物目录的 `meta.json` 停在旧 schema
  （`soko.artifacts/1`，而当前是 `soko.artifacts/2.r3`）⇒ 读侧把整个目录当不存在 ⇒
  面板写「由编译器 ? 写入」。现在写产物时把 schema 与版本戳**一起**升到当前值；
  万一真读不到，那一行也改说「（未知）」而不是 `?`。

## [0.87.0] — 2026-10-09

> **The goal view is now faster than Lean 4's 3.1 ms.** The second half of the
> "align with Lean 4" batch lands: the new on-demand `soko/goalAt` answers **one** goal —
> the one at the caret — in **1.0–1.5 ms / 3.8–6.6 KB** where `soko/goals` needs
> **3.5–5.3 ms / 91 KB** for the same 27 declarations (and `soko/goals` itself is
> **untouched**: its response stays byte-for-byte identical). A scope **line-start index**
> takes the round trip **6.4 → 4.0 ms** and a scope **folding cache** takes the light mode
> to **2.5 ms**, below Lean's 3.1 ms. In the kernel, declaration tables are now **layered
> and persistent**: the command-boundary environment snapshot costs the *same* small
> constant at 64 and 512 declarations (**O(#decls) → O(1)**, T2-A), and editing the last
> command re-elaborates **1** command instead of 12 (T2-B). The hot-keystroke north star
> reads **79.0 ms vs Lean 218.1 ms (2.8× ahead)**. The red line holds: the course's
> `--json` output is **byte-identical, 251/251 course entries**.

### Added

- **`soko/goalAt` — the caret's single goal, on demand** (T2; the protocol-level half of
  aligning with Lean 4's per-command goal query). `soko/goals` is untouched: its response
  stays **byte-identical** (91 361 B for 27 declarations) and every item it returns still
  carries its own runs. The new request answers only the declaration under the caret, and
  that item is **field-for-field equal** to its `soko/goals` counterpart, runs included.
  Readings on one build, real course `unit08`, same caret: `soko/goals` **91 361 B /
  3.5–5.3 ms** → `soko/goalAt` **3 777–6 560 B / 1.0–1.5 ms** (external cross-check on the
  `typing` arm: median **1.29 ms**) ⇒ the **default** mode (`runs: true`) is below Lean 4's
  3.1 ms. Guards: payload ratio **≤ 1/10** (measured 4.1 %–7.2 %, i.e. ≥ 14×), field
  equality with `soko/goals`, and runs still present — plus two reverse verifications
  ("always answer the first declaration" ⇒ red; emptying `goal_runs` ⇒ red). The extension
  uses it to locate a declaration for the hint command instead of pulling the whole list.

### Changed

- **Faster goal view** (same probe, one machine; structural counts plus wall clock):
  - **Scope line-start index.** `line_col_of` used to rescan the text from offset 0 for
    every declaration and hole (27+ times per request). A request-scoped index — keyed by
    the text's `(ptr, len)`, falling back to the original scan in every other case — takes
    `goals(runs=false)` **6.4 → 4.0 ms** and `goals(runs=true)` **6.1 → 4.6 ms**. Response
    bytes are unchanged (26 723 B / 91 361 B) and a 413-point parity guard pins the index
    against the scan it replaced.
  - **Scope folding cache.** A scope already guarantees one notation table, so the fold
    result is cached for the scope's lifetime and cleared on exit (behaviour outside a scope
    is unchanged): `goals(runs=false)` **4.0 → 2.5 ms**, `goals(runs=true)` **4.6 → 3.5 ms**
    ⇒ the light mode is below Lean 4's 3.1 ms. Bytes unchanged; parity plus reverse
    verification.

- **Kernel: declaration tables are layered and persistent (T2-A).** `DeclarMap` is now a
  **library layer** (exclusive before `seal()`, then `Arc` read-only sharing) plus an
  **entry layer** (`Arc` + copy-on-write) with a `stage1` index (Lean's `SMap.stage₁`);
  `notations` and `mutual_block_sizes` became `CowMap`s; and a new `EnvSnapshot` is the
  command-boundary environment, whose `Clone` is **O(1)** — it holds no `Dag`, justified by
  the monotone-intern lemma. Cloning that snapshot copies the **same small constant** at 64
  and 512 declarations (before T2-A: **196 vs 1540** entries, i.e. `declars` + `Dag`), it
  shares its tables with the builder (`Arc::ptr_eq`, checked — not promised), and sealing
  keeps the library layer out of the copy-on-write. Kernel unit tests **69/69**,
  `kernel-diff --fast` zero difference (9 groups), corpus `--json` byte-identical.

- **Command-level environment snapshot (T2-B).** `WalkCheckpoint` with `checkpoint()` /
  `restore_from()` (plus `resume` / `snapshot_tail` / `count_commands`) makes an edit to the
  **last** command re-elaborate **1** command — the entry pass's command count is **12**, and
  before the snapshot every one of them was re-walked. A guard rejects a resumed report in
  which any entry declaration appears twice, and a **process-isolated** A/B shows the resume
  arm and the full-pass arm are byte-identical (diagnostics 2117 B, `soko/goals` 104 100 B).
  `SOKO_NO_ENTRY_SNAPSHOT=1` is the escape hatch.

### Notes

- **North star (hot keystroke, real stdio LSP, course `unit08`)**: real continuous typing
  **79.0 ms** vs Lean 4's **218.1 ms** (2.8× ahead); first keystroke after open 127.8 ms;
  equal-length rename 16.2 ms; trailing comment 21.0 ms with `by=1`.
- **Honest boundaries.** ① The keystroke path itself is `soko/stateAt` (497 B / 0.7 ms);
  `goalAt` serves the hint command's declaration lookup. ② On the `typing` arm the reported
  `goal_ms` is the request's own cost and may answer the *previous* report — a content probe
  verified this and it is the designed concurrent behaviour, so it must not be read as "the
  goal updates faster". ③ The `runs` default stays `true`; `goalAt` exists so a consumer can
  ask for one goal without breaking the existing contract.
- **Guards added with the readings**: the scope index has a 413-point parity test against
  the scan, the folding cache has parity plus reverse verification, and `soko/goalAt` has
  the ratio/field/runs trio. The `--json` red line is the full course:
  **251/251 byte-identical** against a build of the published `v0.86.0` tag.

## [0.86.0] — 2026-10-08

> **The editing loop got much faster** — the hot keystroke's dedup table in the kernel
> is 64× smaller (A6: **637–772 → 395–434 ms**), the cache-key hash stops re-reading the
> whole prefix (A7), the artifact-hit open prewarms the library checkpoint (A5), and the
> entry pass reuses more of what it already computed (A2a/A3/A4a/B3) · **`#check` and
> `#print` now show their output in the Infoview** (C3) · **the prelude is real source you
> can edit**, and `SOKO_PRELUDE_DIR` overrides it at run time without cargo (E1/E3) ·
> **a failing last tactic no longer erases the goals of the steps that worked** (B1) ·
> **the cursor inside a tactic answers the state *after* it**, and a proof closed at its
> last tactic says 🎉 Q.E.D. (B2) · **`build` reports every file** and a no-argument
> `clean`/`rebuild` really clears the project (C1/C2). The red line holds: the course's
> `--json` output is **byte-identical, 251/251 course entries** (`scripts/check-json-identity.py`).

### Added

- **`#check` / `#print` output in the Infoview** (C3; user report 2026-10-08: "`#check` /
  `#print` 在 infoview 没有内容"). `#check` was an inlay hint only, and `#print`'s
  `Printed` event never entered `DocumentReport` — so the LSP structurally could not show
  it. `DocumentReport` now carries the command outputs (`REPORT_SHAPE` 2 → 3),
  `soko/stateAt` answers the outputs **on the caret's line** (the same shape as Lean's
  `getInteractiveDiagnostics{lineRange?}`), and the Infoview renders them in a
  **命令输出** block (`#check` ⇒ `expression : type`, `#print` ⇒ the definition text).
  The `#check` inlay hint stays, and nothing is drawn when the caret's line has no
  output. Guarded on three layers (front report · LSP wire · webview render) plus a
  reverse verification.

- **The prelude is source now** (E1). `prelude/Prelude.sokonanoda`,
  `prelude/Eq.sokonanoda`, `prelude/L1.sokonanoda` and `prelude/Quot.sokonanoda` are the
  truth; Rust keeps only `include_str!`. Editing one of those files really changes
  behaviour (the old "mirror" file was generated from the constants, so editing it did
  nothing), and **F12 on a prelude name lands in the repository file**, not in a cache
  copy. Guarded by `the_three_source_files_are_the_truth_and_rust_holds_no_prelude_text`
  plus a reverse verification (renaming `True` in `prelude/L1.sokonanoda` reddens the
  `True.intro` probe).

- **`SOKO_PRELUDE_DIR` — a run-time prelude override** (E3). Point it at a directory
  containing any of `Eq.sokonanoda` / `L1.sokonanoda` / `Quot.sokonanoda` and that file
  replaces the built-in one; the ones you do not provide stay built in. This is the half
  of "the prelude is modifiable" that works for a **release install with no cargo**: drop
  a file in a directory, restart the server, see the change. The override content is
  folded into the cache key, and a malformed override exits **2 with the reason on
  stderr** instead of panicking.

### Changed

- **`build` reports progress per file** (C1; user report: "约 24 个文件才一条" / "build
  没有反应"). The human channel (stderr) now prints one line per compiled file
  (`… 42/240 · <relative path>`; the old rule was one line per 10 %, i.e. every 24 files
  on a 240-file project), plus a heartbeat (`… still building (12s)`) that a real event
  displaces. The **machine channel is unchanged**: `--json` stdout keeps its heartbeat
  contract (off for a terminal, on for a pipe), `SOKO_BUILD_TICK_MS=1` forces it and
  `SOKO_BUILD_NO_TICK=1` is the escape hatch. Guarded by
  `scripts/check-progress-cadence.py`, which reddens on the previous binary (10 lines for
  N=120) and has a `--selftest`.

- **A no-argument `clean` / `rebuild` also clears the module root** (C2). They used to
  clear only the global cache, so the prewarm that followed was all cache `hit` — a
  "rebuild" that rebuilt nothing. Both now resolve the module root from the current
  directory and clear both places, and the extension's Rebuild / Clean commands call the
  same subcommands.

- **A failing `by` block keeps the goals of the steps that did succeed** (B1). One bad
  last tactic used to discard the whole `by_steps` list, so the declaration fell back to
  its statement (`step:-1, total:0`) and every earlier goal looked broken. The steps that
  ran are kept and the failing step is marked; the strict pass then resumes **from the
  failing step** (B3) — the walk count is `2`, not `1+N` growing with the error position.

- **The cursor inside a tactic answers the state *after* it** (B2, aligning with Lean 4's
  `useAfter`). Being strictly inside a tactic's span now selects the goals *after* that
  tactic; exactly at its start it still shows the entering state. When the last tactic
  closes the proof, the Infoview and the cursor tree both say **🎉 恭喜，证完了
  （Q.E.D.）** (`open` / `failed` / tactic-less declarations keep the neutral 「已无目标 ✓」).

- **Faster editing loop** (P7; pinned by structural counts, wall clock only as an order of
  magnitude):
  - **A6 (kernel)** — `whnf_admit`'s dedup table goes **4 MiB → 64 KiB** with the index
    shift derived from the bit width (the old code hard-coded `>> 42` and `1 << 22` in two
    places, so shrinking only the table panicked — and `quiet_catch` swallowed the panic,
    which is how a "faster" reading can be an illusion). Hot keystroke
    **637–772 → 395–434 ms**, cold open **1496–1502 → 821 ms**; the `TcCache` construction
    count is unchanged (`tc=6876`), so only the unit price moved.
  - **A6b (kernel)** — `TcCache::new`'s ~20 tables are lazy instead of preallocated:
    construction **61.8 µs → 9.21 µs**, its share of compile samples **22.9 % → 4.0 %**.
  - **A7 (front)** — the judge cache key no longer re-hashes the whole prefix text on
    every call (4 small LRUs; the hit predicate is **byte equality**, so a hit depends on
    content alone, and the hash function is untouched): SipHash frames **43.8 % → 8.9 %**
    of compile samples.
  - **A5 (LSP)** — an artifact hit now prewarms the library-layer checkpoint in the
    background, so the first keystroke's `modules=` goes **5 → 1** (≈1233 ms → ≈320 ms).
  - **A2a** — the in-place path takes over `needs_explicit` goals: editing a statement's
    `prefix` goes **5 → 0**. **A4a** — the closure prefix is stored once with the library
    checkpoint (0 recomputations from the second keystroke on). **A3** — the checkpoint is
    a multi-slot LRU, so switching closures no longer evicts the previous entry (the
    "back to the previous entry" edit goes **3 → 1**).

### Notes

- **`Nat` / `Bool` are registered as a boundary, not hidden** (E2). Those 9 names have no
  source text — they are hand-built AST — so `prelude_def_span` answers `None` for them,
  and `prelude/L1.sokonanoda`'s `builtin-rust` section says so. Source-ifying them fails
  the byte-identity criterion and is recorded as an open item instead.
- **E4's second step was attempted and reverted** (2026-10-08). Removing the shadow
  mode's install-time prelude exclusion made the shadow comparison cover more, but the
  course-level shadow run then showed **61 text-level diffs** — not semantic ones (the
  in-place path omits implicit arguments, the slow path prints `@`-explicit ones; and
  `Acc.rec`-style cases answer `None` in-place and fall back to the slow path). Since the
  output stayed byte-identical (252/252), the correct handling was to **restore the
  exclusion and register the finding**, not to accept `diff > 0` as noise. The remaining
  question ("why do the two paths' *texts* differ at install time?") is filed separately.
- **New repro scripts**: `scripts/check-json-identity.py` (the release red line above —
  one command, compares against a build of the published tag, has `--selftest`) and
  `scripts/check-progress-cadence.py` (C1). The identity checker now enumerates **files
  only**: `rglob("*.sokonanoda")` also matched the module-root artifact **directory**
  `<module root>/.sokonanoda/`, so the same tree counted **251** or **252** depending on
  whether that directory existed at that moment — a reading that cannot be compared
  across runs. The course has **251** `.sokonanoda` files.

## [0.85.2] — 2026-10-08

> **Course content only — no language or kernel change.** The set-theory course's
> 2026-10-08 收口批次 ships: **21 course items closed** (C-113…C-132, plus C-05),
> **+24 library lemmas** and **3 new unit exercises**, with the course gate at
> **258 targets · 2188 checked · 916 open · 0 rejected**. Patch semantics: the
> binaries are rebuilt and republished so the shipped course is the closed one;
> nothing in `crates/` moved.

### Added

- **Course library, +24 lemmas** (`courses/set-theory/lib/`), the pieces the closed
  items were waiting on:
  - **`Equiv` +6** — `Type.Equiv` is now an equivalence relation in the course's own
    terms: `Type.Equiv.symm` · `Type.Equiv.trans` (built on `comp`, with the two
    `comp_left_aux`/`comp_right_aux` halves) · `Type.Equiv.elim` (eliminate a
    `Type.Equiv` into an arbitrary `Prop`) · `Type.Equiv.of_inverses''` (a
    bi-inverse pair with the domain hypotheses named).
  - **`Cardinal` +8** — `Cardinal.mk` is **injective** (`Cardinal.mk_inj`, so
    `|α| = |β|` reflects to `Type.Equiv α β`), and `≼` gains its working set:
    `Set.Le.of_equiv` (`≈ ⇒ ≼`) · `Set.Le.powerset` (`A ≼ B ⇒ 𝒫 A ≼ 𝒫 B`) ·
    `Set.image_mapsTo_powerset` · `Set.injOn_of_leftInvOn` ·
    `Set.image_injOn` · `Set.mem_right_of_image_eq`.
  - **`ZF` +4** — the `IsPair`/`Extensional`/`IsRegular` projections the ordered-pair
    work needs: `extensional_def_apply` · `isRegular_apply` · `isPair_mem_left` ·
    `isPair_elem_or`.
  - **`Rel` +3** — `Rel.inv_comp_apply` (the inverse–composition exchange) ·
    `Rel.transClosure_trans` (transitive closure is transitive) ·
    `Rel.ofPartition_trans` (a partition's relation is transitive).
  - **`Ordinal` +2** — `isSuccOf_elem_iff` · `isSuccOf_elems_elems_transitive`
    (successor-of, and successor elements compose).
  - **`SUnion` +1** — `Set.mem_pi_univ`: every `f : ι → α` is a member of the
    **all-`univ` product**, so `λ` lands in `Set.pi`'s argument position.
- **Three new exercises** in the canvas + solutions:
  - **unit109 (ordinal arithmetic, exercises 6–7)** — `addsTo_succ_inv`, the inverse
    lemma (`x + σ y = v` splits into a predecessor half, with the two hypotheses the
    proposition genuinely needs: `σ w ≠ zero` and `σ` injective) and `addsTo_assoc`,
    the **associativity** of relation-version addition (Enderton §8.1).
  - **unit113 (exercise 7)** — **Cantor's theorem** `cantor_no_le`:
    `𝒫 ℕ` is not `≼` `ℕ` (Enderton §6.4 6A · Halmos §22), constructively, via the
    new `∃`-version `cantorDiagonal`. The unit now also records *why* the older
    single-point `diagonal` cannot carry this proof (from `f A = m` you only get
    `f {m} ≠ m`, never `A = {m}`).
- **Repro scripts 7 → 14** (`courses/set-theory/gaps/*.sh`): seven new one-command
  scripts (`C-113-C-114-cantor.sh` · `C-115-C-116-powerset-le.sh` ·
  `C-118-C-125-ordinal-succ.sh` · `C-119-C-126-C-129-zf-extraction.sh` ·
  `C-120-C-121-C-130-cardinal-laws.sh` · `C-123-C-128-rel.sh` ·
  `C-124-C-127-pi-univ.sh`), each with its reverse verification, plus the two
  `.sokonanoda` shape probes (`C-113-or-defapp` · `C-122-binder-annotations`).

### Notes

- **Residual boundaries, recorded rather than hidden** (`courses/set-theory/OPEN-ITEMS.md`):
  a `def`-headed function still cannot be applied directly in continuation position
  (land a `λ` instead) · a leading-implicit constant in a **nested argument** position
  cannot have its implicits filled · a `λ` body written as a **type annotation**
  `(e : T)` still fails to parse · **`Or.elim`'s argument order is the reverse of
  Lean 4's** (the main premise comes last) · `|>` is not in the teaching syntax.
- **Gate numbers for this batch** (`python3 courses/set-theory/tools/check.py`):
  258 targets · 2188 checked · 916 open · **0 rejected**; `--gaps-only` **14/14**;
  `check-lib-closure.sh` green; `notation-lint.py` **300 files, zero legacy forms**;
  `build` on 251 files = 245 hit + 4 compiled + only the **2 deliberately-rejected**
  wall probes.

## [0.85.1] — 2026-10-08

> **“失败 2” now says *which* files and *why*** — a failing build/rebuild names the failing
> files in the notification and prints the kernel's reason in the output panel (and in a
> `失败明细` block on the CLI's stderr).

### Fixed

- **A failing build names the file and the reason** (user report, 2026-10-08: "我运行
  `sokonanoda:rebuild` 会报 **失败 2**，但是我又不知道哪里失败的"). Two gaps, both closed:
  - **the CLI only reported counts**: a project-mode failure returned `Ok("failed")` and the
    report's diagnostics were **dropped**, so the `build.file` event carried
    `status: "failed"` with **no reason** — a consumer could not name the file even if it
    wanted to. Now the failure carries `Err(<module>: <line>:<col>: <code>: <message>)`, the
    event gains an **additive `error` field**, and human mode prints a
    `失败明细（N 个文件）` block (relative path + one reason per file) plus a hint that
    **deliberately-rejected probes** (`gaps/*-reject.sokonanoda`, e.g. the course's C-04 /
    C-112 wall probes) are expected to show up there;
  - **the editor's notification only showed the count** — it now appends
    `— 失败：<file>、<file>` (up to three, then `等 N 个`) and writes a `失败明细` block with
    every reason to the **sokonanoda build** output channel, which the notification's
    显示输出 button opens.

## [0.85.0] — 2026-10-08

> **The CLI gets `clean` and `rebuild`** (the same three verbs the editor has) · **`build`
> no longer looks dead** (skips `target`/`node_modules`/…, narrates the scan, prints progress
> every ~10% in a terminal) · **an unknown subcommand is a usage error** (exit 2, named, with
> a pointer to `--help`) · and **error messages name the file**.

### Added

- **`sokonanoda clean` / `sokonanoda rebuild`** (user report, 2026-10-08: "rebuild 和 clean
  没有实现"). `clean` ≡ `build --clean` (clear **both** stores, compile nothing) and
  `rebuild` clears both stores **then** warms — the editor's `Rebuild` semantics in a **single
  process**, with the same `build.clean` → `build.begin`/`file`/`summary` event order, so
  `--json` consumers keep working. `build --clean` still works unchanged. The editor's
  **Rebuild** and **Clean Cache** commands now call these subcommands instead of hand-rolling
  a two-process `build --clean` + `build` dance ⇒ one implementation, not two.

### Changed

- **`sokonanoda build` without a path is no longer silent** (user report, 2026-10-08: "build
  没有反应"). Its default root is the current directory, and it used to walk *everything* —
  in this repository that is **269k files** (`target/` 240k + `node_modules/` 15k), with
  nothing on screen but a 1-second heartbeat for minutes. It now **skips**
  `.git`/`.hg`/`.svn`/`node_modules`/`target`/`.sokonanoda`/`.vscode-test`/`__pycache__`/
  `.venv` (an **explicit** root is never skipped — `sokonanoda build target/` still walks it),
  narrates the scan (`scanning …` / `found N …` on stderr), and prints progress every ~10%
  when it is compiling a batch.

### Fixed

- **An unknown subcommand is a usage error, not a missing file** (user report, 2026-10-08:
  "cli 很多命令有问题"). `sokonanoda rebuild` used to fall through to "check this file" and
  answer `error: No such file or directory (os error 2)` with exit **1**. Now the message
  names the token (`未知子命令 \`rebuildd\``), points at `--help`, and exits **2** — the
  documented usage code.
- **Error messages name the file**: `check` and `grade` on a missing path say
  `读不到 <path>：…` (it used to be a bare OS error with no filename); a **directory**
  argument adds "use `sokonanoda build <dir>`"; `build` on a source-less directory says
  `… 没有 .sokonanoda 文件` **before** the usage line (it used to print usage only, which read
  as "you typed the command wrong"); `watch` on a not-yet-existing file says it is waiting
  (instead of looking hung), while `watch --workspace` on a missing root is now a usage error.

## [0.84.0] — 2026-10-08

> **The Infoview keeps exactly one jump** — notation symbols are no longer links
> (the declaration name still goes to its definition) · **hypotheses read
> `h : A ⊆ B`** · **artifact sizes pick their own unit** (`2 KiB`, `1.1 GiB`) ·
> **`import lib.Set` is a link that opens the module file** · and **the goal after
> `apply h a` keeps its notation** (`a ∈ A`, not the unfolded `A a`).

### Added

- **`import` lines link to the module file** (user report, 2026-10-08: "import
  这一行的代码增加跳转功能，打开对应的文件"). `textDocument/documentLink` answers one
  link per `import <module>` line whose module resolves in the closure: the range
  covers the **module name** (`lib.Set`), the target is that module's file. A
  single-file document answers an empty array, and an `import-not-found` module
  gets **no** link (a dead link is worse than a link-less line — the diagnostic
  already explains why). The scan is lexical and reads the buffer, so it stays
  correct while typing; module→path comes from the closure's module table (the same
  one cross-file definition/rename use) and is never re-derived here. VS Code
  renders it with no extension-side change (the language client registers
  `DocumentLinkFeature`). Design: `docs/design/import-links.md`.

### Changed

- **Infoview notation symbols are no longer clickable** (user decision, 2026-10-08:
  "符号 notation 啥的不需要跳转链接，声明列表里开头的 theorem 名字能跳转就行"). Every
  classified run used to become an underlined link (G-53), which turned the whole
  goal panel into clickable tokens and made a symbol the easiest thing to hit by
  accident. Now only the declaration card's **name** jumps (to its definition,
  E27); the goal's `目标` label still reveals its source span. The wire keeps the
  runs' source positions — only the consumer changed.
- **Hypotheses read `name : type`** (user report, 2026-10-08). Infoview binder rows
  now put a colon between the name and its type (`hA : a ∈ A`) instead of running
  the two together.
- **Artifact sizes scale their unit** (user report, 2026-10-08: "1160652678 字节
  可以改成更智能的单位，根据数值大小变化"). The project block prints `512 B`,
  `20 KiB`, `1.1 GiB` (1024-based, IEC names) instead of a raw byte count.

### Fixed

- **The goal after applying a `⊆` hypothesis keeps its notation** (user report,
  2026-10-08: "sorry 这一行的目标 'A a' 有办法显示成 'a ∈ A' 吗"). `apply h a` builds
  the new goal from the **definition body** of `Set.subset`; the course library wrote
  that body in its unfolded form (`∀ x, A x → B x`), so the goal came out as `A a` —
  and the display layer can only fold a notation whose **target name is still in the
  text** (`Set.mem a A` → `a ∈ A`), never invert an arbitrary definition body. The
  body (and `Set.subset_def`'s statement) now name `Set.mem`/`∈`, matching Mathlib's
  `Set.Subset s₁ s₂ := ∀ ⦃a⦄, a ∈ s₁ → a ∈ s₂`, so the goal displays `a ∈ A` again.
  Guarded by `docs/gaps/repro/G95-subset-goal-keeps-notation.sh`, whose reverse half
  pins that an unfolded body still shows `A a` (the check cannot go vacuous).

## [0.83.0] — 2026-10-08

> **Editing a declaration in a project is ~25% faster** (G-29 — the shared library
> layer is compiled once and its environment is reused across keystrokes; the
> structural reading `edit.modules` goes **5 → 1**) · **`build` / `rebuild` stop
> paying for a shared dependency once per entry** (G-68 — Σ closure compiles
> **1614 → 517**) · **set-builder notation works** (G-60) · **Infoview notation
> targets answer `documentHighlight`** (G-55) · **error messages name the binder and
> the missing leading type parameter** (G-49 / G-21) · and a parallel `build` no
> longer aborts with a stack overflow (G-94).

### Added

- **Set-builder notation** (G-60). `{x : α | P x}` desugars to `fun (x : α) => P x`
  (pure kernel, no library needed), and `{x ∈ A | P x}` desugars to
  `Set.sep A (fun x => P x)` — the same head constant as the explicit `Set.sep A P`,
  so `mem_sep_iff` / `sep_subset` rewrite through it. `{x | P x}` stays rejected:
  this language has no metavariables, so `x`'s type would have no source. Its code
  changes from `set-literal-shape` (which pointed at the wrong thing) to the
  dedicated `set-builder-shape`, and the hint names the two accepted forms.
  Desugaring happens entirely in the parser, so the display side, `by` stepping,
  semantic highlighting, spine and goals are untouched.

### Changed

- **Editing a declaration in a project no longer recompiles the library layer**
  (G-29, the "editing is slow" report). The project session compiles the shared
  library layer once, keeps its environment as a **checkpoint**, and reuses it for
  the next keystroke when the library summary is byte-identical (names, absolute
  paths, source text, order — prelude, version and the `SOKO_*` switches are folded
  into the cache key). On `unit08` the structural reading — module compiles
  triggered by one edited line — goes **5 → 1**, and the edit wall clock goes
  **3406 ms → 2555 ms (−25%)**; the two earlier rounds on the same entry are
  included (the synthesized prefix compiles inside the entry pass go back through
  the incremental path, **−20%**; `infer_expected_level` joins the in-place path,
  **−19%**, `prefix` 10 → 5). Judgement-neutrality is pinned by `kernel-diff`
  (1724 cases, zero differences) plus the whole course's `build --json`,
  `course --json` and `query check --json` byte-for-byte. Two honest costs: the
  language server compiles on a dedicated single worker thread (concurrent compiles
  of different documents are serialized), and the reuse is bounded (≤ 8 arenas,
  ≤ 64 reuses per checkpoint) — at the ceiling the previous full-recompile path
  returns verbatim.
- **A synthesized judgment document no longer loads the prefix's `theorem`s as
  opaque constants** (G-31 / G-92). The synthesized document used to wrap each
  prefix theorem in an opaque constant, which forced a second, structurally
  different judgment: `by_calls` **3.82× → 2.00**, and the reproduction's wall clock
  **44.23 s → 2.98 s (14.8×)**. A shadow arm compared 665 questions with **0
  differences**.
- **The solver's budgets are configurable, and their defaults now follow Lean 4**
  (G-88): `fuel` 4096 → 20000 (`synthInstance.maxHeartbeats`) and recursion depth
  64 → 512 (`maxRecDepth`), registered with the other limits and reachable through
  the same `SOKO_LIMIT_*` switches. Hitting a budget still retries with a bigger
  one and, at the ceiling, **abstains** (`Tri::Undef`) — never "no".
- **`build` / `rebuild` no longer compiles a shared dependency once per entry**
  (G-68). The CLI now runs a pre-pass that groups project entries by their
  **library closure signature** and compiles each shared library layer **once per
  group** (groups run in parallel; every entry still starts from its own
  checkpoint, so its judgments are unchanged). On the full course (249 files, 248
  entries with imports, 49 distinct library closures) the sum of closure module
  compiles drops from **1614 to 517 (3.12×)**. The acceptance readings are
  structural: the reproduction script's `by_calls` goes **3 → 1**, the
  `check-recompile-factor.py` budget is tightened **3 → 1**, and
  `project_recompiles_shared_deps`'s marginal pass count goes **1.80 → 1.20**
  (its threshold is tightened back to 1.5). The full course's cold `build --json`
  and the project artifact store (whole `ProjectReport`s) are byte-for-byte
  identical before and after. `build.progress` keeps reporting one event per
  finished entry, now emitted from inside the shared compile phase.

### Fixed

- **`documentHighlight` on a notation declaration's target name now answers**
  (G-55). `prefix:70 " 𝒫 " => Set.powerset` puts a *reference* to `Set.powerset`
  in the declaration line, but it is not a use point in the AST, so the
  definition→references reverse lookup came up empty and the server returned
  `null` for all 11 notation targets in the real course library. The name under
  the cursor is now recognized lexically (the same check F12 uses) and the
  answer is the cumulative "same definition" set: the name under the cursor
  (always — so the result is never `null` and always contains the position the
  user clicked), every occurrence of the notation symbols this file declares
  for that target, and — when the target resolves inside this file — its
  definition name plus its explicit uses. Ranges on this path are recomputed
  from byte offsets, so a line containing an astral character such as `𝒫` no
  longer shifts the highlight by one column. Lean 4's
  `handleDocumentHighlight` (definition range + `usages`) is the reference
  semantics; the deviation from the minimal "only this one name" reading is
  deliberate — a single-range answer would *replace* the editor's own
  text-based occurrence highlighting and hide the definition occurrence.
- **A cold `build` with entry-level parallelism could abort with a stack
  overflow** (G-94). Compile worker threads used the platform default stack
  (2 MB on macOS), and elaborating a deeply recursive file overflows it
  (`thread '<unknown>' has overflowed its stack`, `Abort trap: 6`) — the main
  thread has 8 MB, which is why `SOKONANODA_BUILD_JOBS=1` worked and the default
  (one thread per core) did not. Worker threads now get 32 MB, the same fix the
  language server already carried for the identical reason.
- **A short implicit call in argument position now receives its expected type**
  (G-30's second reproduction). `theorem t3 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl hp)`
  used to be rejected — `Or.inl`'s `?B` had no argument to read and fell back to the
  "same shape as its sibling" default (`?B := ?A := P`) — because the expected-type
  propagation hook (`elab::b1_local_expected`) was built but **gated off by default**.
  The gate is now **on by default**: the whole course's `build --json` is byte-for-byte
  identical with the gate on and off (50065 lines, diff 0), so this is judgment-neutral
  on real corpora while the shape above turns green. Escape hatch `SOKO_ARG_EXPECTED=0`
  restores the previous behavior exactly.
- **Infoview goal text for a module that uses an imported notation now matches the
  declaration card** (G-81 / G-83). `QueryDoc::compute_display` parsed every module
  in the closure independently, with an empty inheritance table, so a module that
  used a notation declared in a dependency (`⋃₀` from `lib/SUnion`, `∃` from
  `lib/Exists`) failed to parse as a whole and its own notations never entered the
  table — the root state folded them, the card did not. The display copy now builds
  its table the way the closure loader does (topological order +
  `parse_with_inherited`), so the two agree by construction; the fields that feed
  judgement are untouched. Full scans: **470/470** open declarations across 113
  canvases, **901/901** at the root-state level.
- **A type error names the binder instead of a raw de Bruijn index** (G-49): `$4`
  now reads "the 4th binder (β)", with the name taken from the kernel's name stack
  and aligned with the kernel depth; the `by` path, `query_error` and
  `prefix_error` exits all use it.
- **A `by`-path type mismatch reuses the declaration-position diagnostic** (G-21)
  and names the leading type parameter that was left implicit.
- **Kernel hardening that changes no verdict**: the equality probe's step cap
  `PROBE_CAP=2048` is gone (Lean 4 has no such thing — G-89), and the signature
  mask grows **64 → 128** with its carrier `u64` → `u128` (G-90).
- **Hint text that said the wrong thing about cumulativity and `Exists.elim`**
  (L-06): both halves match official Lean 4, so the wrong words are gone (kernel
  unchanged).

## [0.82.0] — 2026-10-06

> **The `Iff.intro` / `And.intro` wall comes down** (G-30 — the last blocker on the
> course's item-style proofs) · **Infoview notation symbols become clickable**
> (G-53) · **editing a declaration gets ~2× faster** (the `by` judgment's O(n²)
> prefix rerun is gone) · and the compile cache stops replaying a verdict that was
> produced under a different `SOKO_*` switch setting.

### Added

- **Infoview runs are clickable → go to definition** (G-53). Runs that carry a
  source position — notation symbols such as `{a}` / `∈` / `⊆`, identifiers,
  prelude names — now resolve to the declaration they came from. The data side
  (`semantic` runs carry spans), the wire side (the LSP forwards them) and the UI
  side (the webview renders them as links) each have their own assertion, and the
  e2e asserts the **user action** (click ⇒ jump) rather than "the link exists".

### Changed

- **Editing a declaration is no longer priced by the file's length.** The `by`
  judgment's O(n²) prefix rerun is gone: the incremental identity stopped
  re-parsing fragments and the identity table no longer thrashes. Cold opens in a
  real host: `unit12` **10490 ms → 6173 ms (1.70×)**, `unit01` 975 → **851 ms**,
  `unit08` 2555 → **2113 ms**. `#check`'s two paths now go through the same
  trusted prefix (`unit12-synthesis` **10.97 s → 5.75 s, 1.91×**).
- **`SOKO_UNIVERSE_METAVAR` is retired.** Constant-level universe metavariables are
  now always generated and solved, the way Lean's `mkFreshLevelMVars` +
  `levelMVarToParam` do it. The switch only ever produced a second, conflicting
  solution, so it is inert in both positions (the positive example `Show 0 5` —
  dropped leading implicits + a surplus argument + a level solved from the
  argument's type — checks either way).
- **The compile cache keys on every `SOKO_*` judgment switch.** A verdict produced
  with `SOKO_ARG_EXPECTED=0` could previously be replayed for
  `SOKO_ARG_EXPECTED=1` (same source text, same cache directory) — which is what
  made CI's `test` job red while the same tests passed locally. The switch state is
  now folded into the key **order-independently** (so an entry warmed by
  `sokonanoda build` is still reusable by the editor, which runs in a different
  process) and `CACHE_FORMAT` is bumped to 5.

### Fixed

- **Expected types now reach the arguments of nested applications** (G-30, the
  course's last blocker). `Iff.intro (fun (_ : {a} = {b}) => h2) (fun (_ : a = b) => h1)`
  — and the same shape with `And.intro` — now check. Three parts, each aligned with
  Lean 4: the `Eq` family registers its real leading-implicit arity (an implicit
  binder never consumes a written argument — `Elab/App.lean:752-775`);
  `type_head_fits_layer` accepts `Sort u`, so a partially applied
  `Eq.subst.{1} Nat …` is not misread as the short form; and a `λ` argument
  receives its binder's type as the expected type (`elabAppArgs`'s
  `elabArg arg binderType`). The consequence is the important one: **`Eq`'s
  universe level is solved (`Eq.{1}`) instead of defaulting to `0`**, which is what
  made `{a} = {b}` unprovable. The three definition-head boundaries of G-73 and the
  three equivalence laws of G-62 flip with it.
- **A notation symbol glued to an identifier no longer steals the identifier**
  (G-86): `r''` is one identifier (Lean likewise requires whitespace around `''`),
  so `lib/Prod`, `lib/Equiv` and `lib/Demo` compile again, while `Aᶜ` / `Bᶜ`
  (109 + 65 sites) keep splitting as before.
- **LSP positions are UTF-16** (G-36): hovering or selecting past a non-BMP
  character no longer lands on the wrong span (three call sites fixed).
- **Explicitly universe-polymorphic constants take part in level inference**
  (G-93): `myax α a` no longer silently defaults the level to `0` while
  `myax.{1} α a` worked — the two spellings now agree.

## [0.81.0] — 2026-10-03

> **Nine language/kernel blockers cleared** — the walls the set-theory course's
> "what Volume I can now write" list was waiting on: **well-founded recursion
> (`Acc`)** · **large elimination** · **classical logic** · **quotients**. Every
> one of them ships with its minimal reproduction flipped from red to green, a
> byte-for-byte corpus comparison, the course gate at **0 rejected**, and the
> real-VS-Code e2e at **40 passed / 0 failed**.

### Added

- **`Acc` (well-founded recursion) works**, in the shape Lean core uses —
  `inductive Acc (α : Type) (r : α → α → Prop) : α → Prop` with the index written
  in the **return** position (G-56/G-64). Indexed inductive families whose
  recursive occurrences change the index (`TC`, transitive closure) elaborate,
  and their recursors **eliminate into `Type`**, so `rank`-style functions and
  `∈`-induction over ordinals are expressible. The header-index form
  (`(x : α)` as a *parameter*) stays rejected, exactly as upstream Lean rejects it.
- **Large elimination** (G-58/G-59): a `Prop`-valued inductive block whose
  non-`Prop` fields are all parameters or indices now eliminates into `Type`
  (`Acc.rec … → Type`), and the recursor's universe arguments are inferred from
  the expected type instead of defaulting to `0`.
- **Classical logic** (G-74): the prelude now provides `Classical.em`
  (`∀ p : Prop, p ∨ ¬p`) and `Classical.byContradiction`. Proofs by contradiction
  and case splits on `em` type-check.
- **`Quot.exact`** (G-75): equality of two quotient representatives reflects back
  to the relation, in the **sound** form that carries the equivalence-relation
  hypotheses (reflexivity/symmetry/transitivity).
- **`Nat.add` / `Nat.mul` compute on variables** (G-76): the recursive equations
  `Nat.add m 0 ≡ m`, `Nat.add m (succ n) ≡ succ (Nat.add m n)` (and the `mul`
  analogues) fire on the deep reduction path, so `#reduce` and `rfl` see through
  a variable-headed `Nat.add`.

### Fixed

- **`Sokonanoda: Install Command Line (安装命令行)` 装完**真的**能用**（G-84）：安装目录会写进
  登录 shell 读的 profile（带标记、幂等），提示文案改成**真话**（写不进就给出该加的那一行）；
  覆盖安装时报告被替换的旧版本，并清掉 `*.bak-*` 残留。
- **新增 `Sokonanoda: Uninstall Command Line (卸载命令行)`**（G-84）：清掉安装位的二进制、
  `.version` 版本标记与 `*.bak-*` 残留，并撤掉安装时写入的 PATH 行（同目录的语言服务器不动）。
- 判据改成**用户动作级**：装完在**新开登录 shell** 里 `sokonanoda --version` 必须 == 插件版本
  （不再比"刚拷贝的那份文件" —— 那是假绿 ✗）。复现件
  `docs/gaps/repro/G84-vscode-cli-install.sh`（含反向验证 ✓）。
- **`cases` on a membership in an image** (G-71): `cases hy` with
  `hy : y ∈ (f '' (f ⁻¹' C))` used to report "the eliminated term is not an
  inductive value" with every argument shifted, and the arm's `exact` could not
  read its hypothesis back. The tactic context's types are now rendered with a
  **fully explicit** pretty-printer form (`@Set.image α β f A`), so the text it
  re-reads round-trips.
- **A definition whose return type is a literal arrow** (G-72): `def mkRel (α)
  (r) : α → α → Prop := fun (a b : α) => …` was green when the module was graded
  on its own and red when imported (`unknown identifier b`) — the return type's
  arrows were counted as parameters, so the recorded body lost two binders.


## [0.80.0] — 2026-10-01

> **三件事一起发**：编辑响应延迟收口（`declaration-incremental.md` §7）· 卷 I 集合论
> **69 个单元全部落地**（课程门禁 **158 目标 · 1431 checked · 616 open · 0 判负**）·
> 输入缩写表 **18 → 75 条**。

### Changed

- **Editing a declaration is no longer priced by the file's length.** The editor used
  to re-check the whole prefix on every keystroke; a session snapshot plus the
  dependency-graph dirty propagation now re-check **only the declarations that
  actually depend on what you touched**. Same file, same operations, measured
  before → after (structure counts, wall clock only as an order of magnitude):
  editing the **last** declaration **~2154–3039 ms → 321–346 ms** (the library layer
  recompiles **zero** times); editing an **earlier** one **~2154–3039 ms → 1159–1231 ms**
  (**−39%**); the **first keystroke after opening** a project file **~3390 ms → 1737–1997 ms**
  (**−46%**). In a real VS Code host the steady-state keystroke → diagnostics path
  measures **~110 ms**. Full tables, the guard that pins them
  (`crates/front/tests/keystroke_structure.rs`) and the one thing deliberately
  **not** done: `docs/design/declaration-incremental.md` §7/§7.1.
- **The course finished.** Volume I of the set-theory course is complete:
  **69 units** (canvas + solution + the standard-library lemmas each one needs),
  from sets/subset/union/intersection through power sets, relations, functions,
  images, equinumerosity, Cantor, Russell and the synthesis unit, up to
  order-theoretic inclusion, adjunction laws for `⋃`/`⋂` and the power set as a
  complete lattice. Gate: **158 targets · 1431 checked · 616 open · 0 rejected**,
  with the nine course guards green.

### Fixed

- **The whole file's colours no longer scramble as you type.** Syntax colouring
  for sokonanoda comes from the language server's *semantic tokens*, and the
  server was computing them from the **last compiled** text instead of the text
  in your editor. VS Code paints those tokens onto the **current** buffer, so
  every token after your edit landed on the wrong characters — one line off per
  inserted line, and it got worse the more you typed. It is fixed at the source
  (`crates/lsp/src/lib.rs`: the token handler is a pure function of the source
  text, so it now reads the buffer, not the previous compile). Project files
  made it obvious: one keystroke costs ~2.2 s of compilation there, so for those
  2.2 s *every* token request answered for the old text.
- Pinned by `crates/lsp/src/tests/tokens.rs::semantic_tokens_follow_the_buffer_not_the_last_compile`
  (insert a line at the top → `theorem` must be reported on line 2; with the old
  code it is reported on line 1, so the guard bites).


### Added

- **Greek letters — and every other symbol the course uses — now have typing
  abbreviations.** `\alpha` (or just `\a`) + `Tab` → `α`; `\Gamma` → `Γ`,
  `\Omega` → `Ω`, … all **48 Greek letters** (24 lower case + 24 upper case).
  Also added: the set-theory library's `≈ ∘ ⁻¹ • ⊕ ⋃₀ ⋂₀` and the
  anonymous-constructor brackets **`\<` / `\>` → `⟨` `⟩`**. The table grew
  **18 → 75 entries**; the keys are copied key-for-key from Lean 4
  (`leanprover/vscode-lean4`), so muscle memory transfers.
  - **One documented exception**: Lean 4 has no `\Mu` — its capital mu is `\GM`
    only — so the table accepts **both** `\GM` (Lean's key) and `\Mu` (the
    spelled-out name the other 23 capitals use).
  - **Hovering tells you how to type it** — including on a Greek-letter
    *variable* (`α : Prop` plus “输入：`\alpha`（别名 `\a`）”), which is exactly
    where the old hover stayed silent.
  - **`\a` is unambiguous**: it is a prefix of `\alpha` / `\approx` / `\and`, so
    it waits while you keep typing and lands only once the word is closed
    (space, punctuation or `Tab`) — the same rule Lean uses. Eager mode
    (`sokonanoda.input.eager`) follows that rule too.
  - Lean's single-letter keys that point at *logic* symbols (`\v` → ∨,
    `\i` → ∩, `\o` → ∘, `\r` → →) are deliberately **not** taken: `\i` giving ∩
    while `\in` gives ∈ is a trap. Design + upstream evidence:
    `docs/design/notation-input.md`.

> 编辑响应：**「敲一个字闪一下」没了**（2026-10-01 用户反馈「一闪一闪」）。

### Fixed

- **Typing no longer makes the editor flash.** The server recompiles on *every*
  keystroke and a teaching-sized file finishes in about a millisecond, so the
  `$/progress` `begin`/`end` pair landed on every single key — and each `begin`
  painted a **whole-document** background decoration, flipped the status bar to
  *compiling…*, and inserted an Infoview progress block (then `end` took all
  three away again). Measured in a real VS Code host: **8 keystrokes lit the UI
  16 times**. The *compiling…* affordance is now shown only after
  `sokonanoda.progress.showDelayMs` (default **300 ms**); a compile that finishes
  inside that window **never touches the UI at all**. Genuinely slow compiles
  (a cold open of the largest course file measured 7.8 s) still light up exactly
  as before, and `Sokonanoda: Build/Rebuild`'s per-file progress is untouched.
  `0` restores the old immediate behaviour.
- **A stale "already compiling" flag could suppress the indicator forever** after
  a window reload: the state is module-level and was never reset on activation.
  `activate()` now resets it.

### Changed

- **Measured end to end, before → after** (real VS Code 1.138.0, same fixture,
  same operation): flicker **16 → 0** lights per 8 keystrokes; keystroke →
  diagnostics **122 ms → 99–113 ms**; keystroke → goal panel **268 ms → 262 ms**.
  The remaining panel latency is ~100 ms of VS Code's own diagnostics pipeline
  plus the extension's 150 ms multi-document debounce — the compiler itself is
  **0.2 ms per keystroke** on the same fixture. Full breakdown and the things
  deliberately *not* changed: `docs/design/edit-latency.md`.
- **Project mode measured too** (`import` closures are a separate compile path):
  a same-run A/B over 9 keystrokes — `progress.showDelayMs = 0` (the old
  behaviour) lit the UI **18** times, exactly 2 per keystroke; the default lit it
  **0** times. Same process, same machine, same fixture, so the difference cannot
  be machine noise.

## [0.79.0] — 2026-10-01

> 求解器的**待定参数**（E19 甲案）成为默认：今天判红的"两侧都是零元糖"形状变绿。

### Changed

- **`∅ ≈ {b}` now elaborates.** When a leading type parameter of a notation or of
  an application cannot be inferred from any operand or from the expected type,
  the solver now lets it follow a **same-shaped already-solved sibling** instead
  of failing. `Set.Equiv {α β : Type} (A : Set α) (B : Set β)` written
  `Set.Equiv ∅ {b}` therefore reads as `Set.Equiv β β ∅ {b}` — the domain is
  **chosen**, not derived. It still refuses to guess when there is nothing to
  follow (`∅ ≈ ∅`) or when the parameter appears nowhere. Escape hatch:
  `SOKO_NOTATION_METAVAR=0` (or `off`) restores the old strict solver.
- **Only the file the fix targets changed.** The non-course corpus is
  byte-for-byte identical except the G-48 repro (2/172 comparisons); the course
  corpus and the course gate counts (**43 targets · 377 checked · 99 open ·
  0 rejected**) are unchanged. Gap ledger: **G-48 → fixed (0.79.0)**.
- **The course standard library no longer makes you spell out leading type
  parameters.** `Set.image` / `Set.preimage` / `Set.mem` / `Set.subset` and
  friends, plus `Function.comp` / `Rel.comp`, now take `{α : Type}` implicitly,
  so `a ∈ A`, `f '' A`, `A ⊆ B`, `A ∩ B`, `(∅) x` mean what they read as — no
  `Two`, no `α β`. The pointful spelling (`Set.mem α a A`) still grades
  identically; the course now uses the short one.
- **Six front-end root causes behind that change were fixed** (delta-unfold
  alignment for `cases`, nullary notation in function position, nested implicit
  positions, template-side unfolding, the old-style gate on route ③, expected
  types for solved implicit arguments) — each with a unit test that was verified
  to fail when the fix is reverted. Course gate: **43 targets · 377 checked ·
  99 open · 0 rejected**; the non-course corpus is byte-for-byte unchanged.

## [0.78.3] — 2026-09-30

> 编译提速的**最后一条腿**：判定不再重复检查它刚刚检查过的前缀。

### Changed

- **A cold `Build` of the whole course is now 2.65× faster than 0.78.2**
  (same machine, cold cache, 1 job, median of 3 runs): **126.4 s → 47.8 s**. The
  time spent judging fell **83%** (113.7 s → 19.2 s).
- **Why it was slow**: every time a `by` block asked the kernel a question, the
  judge re-parsed and re-checked the *entire* prefix of the file from source —
  265 times in one course build, 115.9 s of the 117.2 s total judging time.
  The prefix it re-checked was the very text the compiler had **just** checked in
  the same pass, so the re-check was pure repetition.
- **Nothing you see changes.** The compiler now vouches for that prefix instead of
  re-checking it. `build --json` (excluding the time-based `build.tick` and the
  new `build.progress`) is **byte-for-byte identical**: 2691 lines, **zero
  differing lines**. A shadow mode that runs *both* ways on every judgement and
  compares the **verdicts** reported `shadow_same = 265`, `shadow_diff = 0`.
  A deliberately-wrong `by` block still reports exactly the same error.
- Two escape hatches, if you ever need the old behaviour:
  `SOKO_JUDGE_ENV_VOUCH=0` (this tier only) or `SOKO_JUDGE_ENV_REUSE=0` (the
  underlying prefix-reuse switch). `SOKO_JUDGE_ENV_VOUCH=shadow` runs both ways
  and reports differences without changing behaviour.

## [0.78.2] — 2026-09-30

> 「编译提速」的**第二、三档**（P1-b 收口 + P1-c）与三条用户实测 UI 缺陷的收口。

### Changed

- **Compilation is now ~1.69× faster than the 0.78.1 baseline** (cold cache, 1 job,
  same machine, median of 3 runs): wall clock **214.08 s → 126.80 s**. The judge
  answers in place for the `by` engine too, so prefix re-runs fell
  **3759 → 791** (−79%). Structural counts (noise-immune): re-parsed prefix bytes
  **174,213,583 → 43,329,596**, internal pass count **4126 → 1157**.
- **Nothing you see changes.** `build --json` (excluding the time-based
  `build.tick` and the new `build.progress`) is **byte-for-byte identical** between
  the old baseline and the new default: 2691 lines, **zero differing lines**. A
  shadow mode that runs *both* ways over the whole course reported
  `shadow_diff = 0` across 44234 comparisons.
- **The Infoview no longer shows two unreconciled version numbers.** The project
  block's number is the artifact stamp from `<module root>/.sokonanoda/meta.json`,
  which used to be written **once** when the directory was created — so after an
  upgrade it kept showing the old version forever. It is now refreshed whenever
  artifacts are written, and both version numbers carry labels
  (`服务器 …` / `由编译器 … 写入`).
- **`build --json` no longer prints a heartbeat every second.** All 159 ticks in a
  cold course build carried `file: ""`, so they were pure noise for anyone
  watching a terminal. The heartbeat is now **off unless asked for**
  (`SOKO_BUILD_TICK_MS=<ms>`; `SOKO_BUILD_NO_TICK=1` forces it off).

### Added

- **`Sokonanoda: Install Command Line (安装命令行)`** — install the CLI bundled in
  this extension into `~/.local/share/sokonanoda/bin/`. Offline, no download, and
  the version is the extension's version by construction (verified by running
  `--version`, not by assertion).
- **`sokonanoda.build.timeoutMs`** — the `build`/`rebuild` timeout (default
  300000 ms, **`0` = no limit**). A cold build of a whole course takes ~314 s, so
  the old hard-coded 300 s made the first build after clearing the cache fail.
- **`build.progress`** (`{type, done, total, file}`) in `build --json` — one event
  per finished file, emitted **while compiling**. `build.file` only arrives after
  every file is done, which is why the progress bar used to sit at `0%` and then
  jump to `100%`. Additive: older consumers ignore unknown types.

### Fixed

- **Rebuild no longer shows `0%` and then jumps to "done".** Reproduced in a real
  VS Code host (`percent` values seen: `[null, 0, 100]`) before the fix; the
  progress bar now moves from the fourth second onward.

## [0.78.1] — 2026-09-29

> 「编译提速」：判定就地查环境（P1-a 第一步）。**课程/练习的编译快 1.35×**。

### Changed

- **Compilation is ~1.35× faster on the full set-theory course** (cold cache, 1 job,
  same machine): wall clock **214.19 s → 158.90 s**. The judge no longer re-parses and
  re-elaborates the whole document prefix on every cache miss — it answers in place
  against the environment the elaborator already holds, and only on a miss
  (`crates/front/src/judge.rs`, `crates/front/src/compile/elab.rs`).
  Structural counts (noise-immune): prefix re-runs **3759 → 1881** (−50.0%),
  re-parsed prefix bytes **174,213,583 → 93,858,420** (−46.1%), internal pass count
  **4126 → 2248** (−45.5%), `judge_ms` **146.6 s → 120.4 s**.
- **Nothing you see changes.** `build --json` is byte-for-byte identical with the
  feature off vs on (excluding the time-based `build.tick` heartbeat): 2691 lines,
  zero differing lines, `build.decl` 2647 / `build.file` 42 / `build.begin` 1 /
  `build.summary` 1 on both sides. A shadow mode that runs *both* ways and compares
  the answer text reported `shadow_diff = 0` across the whole course
  (`shadow_same = 555552`).
- **Escape hatch**: `SOKO_JUDGE_INPLACE=off` restores the previous behaviour exactly;
  `=shadow` runs both paths and reports any divergence.

## [0.78.0] — 2026-09-28

> 「性能与纪律固化」（E17–E18）。

### Added

- **A guard that makes "waiting on a black box" fail the build.** If a commit
  changes kernel source, or its subject claims a performance result, its message
  must carry a measured timing number (`123ms`, `1.5s`, `1m23s`, a `PERFJSON`
  line, `ratio 1.02`, …) or an explicit, line-anchored exemption
  (`soko:no-timing: <reason>`). Otherwise `scripts/check-timing-evidence.py`
  exits 1 and names the commit. It runs in the `pre-push` hook and in CI.
  The trigger was narrowed against real history: a first version that watched all
  of `crates/**` produced eleven false positives, and a second that watched
  non-test `crates/*/src/**` still flagged ordinary feature work — requiring
  timings there would only push people to invent numbers. The shipped version
  watches kernel source and self-declared perf commits: three commits out of the
  last fifty, zero false positives.

### Changed

- **The performance ledger is current again.** It had been frozen at
  `0.68.0 / 2026-09-25`; a `0.77.1` entry was measured and appended (same schema,
  same 20 cases).

### Fixed

- **`SOKO_JUDGE_STATS` printed nothing for term-style files.** The reporter was
  only installed on the `by`-block path, so once the course solutions moved to
  term style the `by` call count was zero and the whole report bailed out early —
  including `JUDGE_INFER`, which is precisely the dominant cost in that style.
  It is now also installed on the `judge_infer` path. This is an observability
  fix: no kernel judgement changed.

> **The kernel performance question now has an answer.** Measured across nine
> versions on one machine with identical workloads, every synthetic baseline sits
> within ±4% — there is no regression to roll back. The one large increase is the
> real course closure (+86%), and it tracks the course growing from 331 to 376
> checked declarations: per-declaration cost is flat at 44.9ms. The reason kernel
> speed has not moved the needle is architectural — profiling puts `by`-block
> judgement at 68% with no separate kernel frame, and after the switch to term
> style the cost is `JUDGE_INFER` re-elaborating the whole prefix on every cache
> miss. Making the kernel faster has a 27% ceiling; the speedups worth doing
> reuse the prefix instead. Details: `docs/perf/E17-kernel-conclusion.md`.

## [0.77.1] — 2026-09-28

> 「课程尾巴：速查表清账 + 记法边界登记」（ST16–ST19）。

### Added

- **`abbrev triple` in the set-theory library** — a pointful stand-in for the
  three-element set literal `{a, b, c}`, which the language still refuses with a
  dedicated `set-literal-shape` diagnostic. This is the first real use of
  `abbrev` in the course, and it gives learners an explicit landing spot instead
  of leaving "why do three elements not work?" unanswered.

### Changed

- **The notation cheat sheet is reconciled with the implementation.** Two claims
  were marked "to be filed in the ledger" back in 0.62.0 and had never been
  re-measured. Both were re-measured on 0.77.0: the diagnostic for a pointful
  `Set.mem a A` that omits `α` is unchanged, and `∅ = A` / `∅ ≠ A` is still
  rejected — now with the verbatim text
  ``def_eq failed: def_eq mismatch expected: Sort(0) | actual: Sort(1)``. Two
  forms that *do* work today were added: `(Set.empty α) = (Set.empty α)` and
  `(Set.empty α) = A → A = (Set.empty α)`.

### Fixed

- **Nothing in the kernel.** `git diff crates/kernel/` is empty for this release
  as well.

> **A `scoped` notation was tried and rejected.** `scoped notation "⋂ₚ" => Set.sep`
  works inside the module that declares it, but fails for every consumer that
  writes `open scoped Set` — `elab-notation-argument-unsolved`. A control case
  proves this is **not** the `scoped` mechanism: the same shape written *without*
  `scoped` in another file reports the identical error. The real boundary is that
  leading type parameters are only solved when the notation and its target live in
  the same module. The notation was removed rather than shipped, and the boundary
  is now recorded in the library itself.

## [0.77.0] — 2026-09-28

> 「卷 I 集合论：类型论自身能表达多少」（ST1–ST5 · ST8 · ST10 · ST12–ST15）。

### Added

- **Quotients are usable from source.** `Quot` / `Quot.mk` / `Quot.lift` /
  `Quot.ind` / `Quot.sound` are installed by the prelude (the kernel already had
  `Declar::Quot`; what was missing was the front end producing it). `Quot.lift`
  and `Quot.ind` really compute on `Quot.mk` — the regression asserts it with
  `Eq.refl`-level goals, not by looking at text. A file that declares its own
  `Quot` still takes over the family, as before.
- **Six new course libraries for predicate-style set theory**
  (`courses/set-theory/lib/`): `Set.sep` (separation) · `SUnion`
  (`Set.sUnion`/`Set.sInter`, notations `⋃₀`/`⋂₀`) · `Sum` (notation `⊕`) ·
  `Set.pi` · `Ordinal` (ordinals **as predicates** — no new type) ·
  `Cardinal` (`Cardinal := Quot` of type equivalence; `Cardinal.sound` is
  `Quot.sound`, and `Cardinal.lift_mk` is `Eq.refl`-level, so the quotient
  reduction is exercised for real) · `Choice` (`axiom choice`) · `ZF`
  (`axiom regularity` + a table saying which ZF axioms are theorems here) ·
  `Extensionality` (`propext` + dependent `funext`).
- **`docs/design/v077-kernel-deficiencies.md` — the release's actual deliverable.**
  Every gap the course ran into, with a self-asserting reproduction under
  `docs/gaps/repro/`, its blast radius, its workaround, and the source location of
  the blocker. Seven entries: **G-56** (`Acc` cannot be declared — a recursive
  occurrence whose index changes trips the kernel's uniformity check) · **G-58**
  (large elimination is unavailable, so `Prop`-valued inductives cannot eliminate
  into `Type`) · **G-59** (a `Type`-valued block's recursor does not eliminate
  into `Type` either) · **G-60** (set-builder braces are not a notation shape) ·
  **G-61** (no η, so Lean core's `Quotient`/`Setoid` wrappers cannot be defined) ·
  **G-62** (`def` and its unfolding are not interchangeable, so the three
  equivalence laws of a `def`-encoded relation cannot be written) · **G-63**
  (`Quot.lift`'s explicit universe arguments are hard to line up).

### Changed

- **The course is checked against the kernel on both ends.** ST1's decision record
  (`docs/design/v077-st1-boundary.md`) is reconciled with four self-contained
  probes: the record's claimed diagnostic text and checked counts are compared
  against what the kernel actually says, in both directions.
- **`STATUS.md` / `docs/design/v077-set-theory.md`** record which chapters are done
  and which are blocked, so "blocked" is a documented state rather than a silence.

### Fixed

- **`docs-lint` no longer trips on `docs/design/v077-set-theory.md`** — the design
  notes were compressed back under the 150-line budget for new design docs.

> **Not fixed on purpose.** ST6 (transitive closure), ST7 (rank), ST9
> (transfinite recursion) and ST11 (order types / Aleph) are blocked by G-56 and
> G-58. The kernel was **not** touched in this release: `git diff` over
> `crates/kernel/` is empty. Each gap is registered instead of worked around
> silently — that registration *is* the deliverable.

## [0.76.0] — 2026-09-28

> 「prelude 显式化 + 两跳」（E09–E11）。

### Added

- **The prelude is a real file in the repository now.** `prelude/Prelude.sokonanoda`
  is the exact text the checker installs (`Eq`, `And`, `Or`, `Iff`, `Not`, `Ne`,
  `False.elim`, the recursors …), so you can read what the logic you use is built
  from instead of reading Rust. It is a **mirror**: the compile-time constant is
  still the single source of truth, and a guard fails the build if the two drift
  by even one byte (regenerate with
  `SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror`).
- **Built-in notations have a declaration point.** `∧ ∨ ↔ ¬ ≠` are language-level
  (re-declaring them is refused on purpose), so they used to have no source line
  anywhere — `F12` on `∧` did nothing. They are now registered in the prelude with
  the repository's existing `-- sokonanoda:builtin-notation "∧" => And` comment
  convention (no new syntax), and `F12` lands on that line.
- **Built-in sugar is registered too.** `{a}` → `Set.singleton`, `{a, b}` →
  `Set.pair`, and `⟨a, b⟩` → *decided by the expected type* (which is what the
  elaborator really does — the registry says so instead of inventing a target).
  A guard keeps the registry and `elab.rs` word-for-word consistent.

## [0.75.0] — 2026-09-27

> 「跳转与高亮」（E05–E08）。

### Fixed

- **`F12` on a notation declaration's target name lands on the definition.** The
  name after `=>` (`prefix:100 " 𝒫 " => Set.powerset`) used to answer with **the
  cursor's own span**, so `F12` moved the editor to the line you were already on —
  visually "nothing happened". In the course library `Set.powerset` landed on
  L125 instead of L81 and `Set.compl` on L126 instead of L79. The real definition
  span was being computed and then dropped (`if let Some((path, _)) = …`); it is
  now the one used. A name whose definition lives **outside** the current file's
  closure (e.g. `Set.image`, defined in `lib/Image.sokonanoda`) still answers
  `null` on purpose — that is the same answer `F12` gives for a name that is not
  in scope, and fabricating a location would be worse than admitting it. Notation
  target names are also painted **one** colour now (they were already uniform in
  the current corpus; the criterion that pins it is new, so a regression cannot
  slip through silently).

## [0.74.0] — 2026-09-27

> 「记法补齐 + 编译体验修复 + Infoview 面」（E01–E04 · E21–E23 · E27–E31）。
> 这一节只收 **v0.73.0 tag 之后**新加的条目；tag 当时已写进 0.73.0 的那几条
> （记法显示、`{a}` 跳转、前导类型参数补全）留在下面，不重复搬。

### Added

- **The Infoview shows the project.** Manifest path, module root, entry, the
  module table (status · declarations · errors/warnings · which module is the
  entry), the counts and the artifact/compiler line — in the centre panel, so you
  no longer have to switch to the sidebar project tree to see them. The data is
  the language server's `soko/project` answer, forwarded as-is; the CLI's
  `query project` writes and deletes `compiled/*.tmp`, so polling it would
  recompile on every refresh.
  A `requires` mismatch is a **visible warning block at the top of the section**,
  never a tooltip: when the manifest's `requires` drifts, `is_clean()` goes false
  and the **project cache is silently switched off** — every file is then
  recompiled from scratch on every open, and a tooltip is exactly how that stays
  invisible.

- **`Sokonanoda: Clean Cache (清除编译缓存)`** — clear the compile cache
  *without* compiling anything. The CLI has always had this (`build --clean`
  clears both the global cache and every module root's `.sokonanoda/compiled/`,
  then returns), but the extension only exposed `Build` and `Rebuild` — and
  `Rebuild` is "clean, then build" in one step, so there was no way to just
  clear. The notification reports **three** numbers —
  `清掉 N 条缓存（全局 X · 项目 Y）` — taken verbatim from the CLI's
  `build.clean` event: "cleared 0 project entries" is precisely the fake action
  the CLI comments warn about (a global-only clean leaves project entries in
  place and the next build is all hits).

### Changed

- **`Build` / `Rebuild` show real progress — and can be cancelled.** The CLI
  already emitted one `build.file` event per file, but the extension buffered the
  subprocess output until exit and only read `build.clean` / `build.summary`, so a
  manual build was "an output panel scrolling JSON, then a notification" — no
  status bar, no Infoview progress, nothing on the overview ruler (the complaint
  behind this: *"我要求有进度条，现在是没有进度"*). The stream is now consumed
  line by line and drives the same three surfaces the automatic (language-server)
  compiles already use — status bar (`$(sync~spin) Sokonanoda: 3/13 文件 ·
  lib/Set.sokonanoda`), the Infoview's three-line progress block, and the overview
  ruler — plus a VS Code progress notification with a **cancel** button. A new
  additive CLI event, `build.begin` (`{type, files}`), carries the total so the
  count can read `3/13` instead of just "3 so far". The output panel now renders
  those events as readable lines (`[3/13] lib/Set.sokonanoda ✓`) instead of raw
  JSON.

- **`Build` / `Rebuild` compile the project, not the file you happen to have
  open.** Both commands passed the active `.sokonanoda` file to the CLI, and the
  CLI compiles exactly what you hand it — so on a 35-file course the status line
  said `1 个文件` and every other file's cache stayed cold (the complaint that
  started this: *"点哪个文件，编译哪个文件"*). The target is now the **module
  root** the language server reports for the active document (`soko/project` →
  `root`), falling back to the workspace folder while the server has not answered
  yet. It is a directory in every case, never a single file; to compile one file
  on purpose, use the CLI: `sokonanoda build <file.sokonanoda>`.

### Fixed

- **A running `Build` / `Rebuild` can be stopped.** `runBuild` had no
  cancellation path at all: once started, the only way out was the five-minute
  timeout. The run is now wrapped in `window.withProgress(…, cancellable: true)`
  and the cancellation token is wired to `child.kill()`.
- **`Rebuild` really rebuilds.** Its `--clean` step ran without a target, and
  `sokonanoda build --clean` with no path only clears the **global** cache — the
  module root's `.sokonanoda/compiled/` entries survived, so the build that
  followed was all cache hits and the summary said `清掉 0 条缓存`. The clean step
  now names the same project target, so "clear the cache and recompile" does that.
- **Declaration names in the Infoview jump to the definition.** Clicking the name
  on a declaration card now asks the same question `F12` asks
  (`vscode.executeDefinitionProvider`) instead of scrolling to the source span —
  the two look alike (the editor moves either way), which is exactly why a plain
  `reveal` had been passing as "the jump works". The result is a real definition
  location, correct across files. ⚠ Notation symbols (`{a}`, `∈`) inside types and
  goals are **not** clickable yet: the semantic runs on the wire carry
  `{text, kind}` and no source position, so there is nothing to ask the definition
  provider *about* (tracked as gap G-53).

## [0.73.0] — 2026-09-27

### Changed

- **You can see that it is compiling.** The language server reports `$/progress`
  around every compile; the extension turns it into a status-bar *compiling…*
  state, a three-line progress block in the Infoview, and a whole-document mark
  on the overview ruler — so you can tell *that* something is happening and
  *where*. `begin`/`end` are never throttled (they are the paired state
  boundaries); `sokonanoda.progress.throttleMs` (default `250`) throttles only
  the in-between refreshes. No fake percentage is shown: the server does not
  know one yet, so the block says 进行中…

- **The Infoview is readable now.** Declaration types, `:=` values and `⊢` goals
  were rendered at `0.78em` **and** dimmed twice (a grey foreground multiplied by
  `opacity: 0.85`), so the one thing the panel exists to show was the hardest
  thing to read. They are now at body size, `line-height: 1.5`, with no extra
  opacity, and rows have real padding. New setting
  `sokonanoda.infoview.fontScale` (default `1`, range `0.8`–`2`) scales them
  relative to the editor font; changing it applies on the next state update —
  no need to reopen the panel.

### Fixed

- **`F12` on a prelude name now works.** `Or` / `And` / `Iff` / `False` /
  `Eq.refl` … are installed by the trusted prelude, so their hover rows carry no
  `resolution` — jumping did **nothing** (silently). The command now materializes
  the prelude source (byte-identical to what the checker installs) and lands on
  the declaration line. `Nat` / `Bool` families are built from a hand-written AST
  and still have no source; they return **no** location rather than a made-up one.
- **Notation reads as notation everywhere the user sees text** (Infoview
  `⊢` target, declaration types, goals): `->` is folded to `→`, and
  `Set.singleton α a` / `Set.pair α a b` fold back to `{a}` / `{a, b}`.
- **`F12` on a `{a}` set literal** now lands on `Set.singleton` (it used to
  return nothing).
- **Leading type arguments in notation are filled in for you.** Writing `a ∈ A`
  or `f '' A` no longer requires spelling the leading type parameter out
  (`Set.mem α a A`); it is inserted from the operands, so the notation you write
  is the notation that is checked.

## [0.72.0] — 2026-09-25

> **维护版**：本次没有用户可见的行为变化 —— 改动是**性能回归门禁**、**文档收口**
> 与**两个默认不调用的内核原语**。

### Internal

- **性能回归门禁进 CI**（`perf-gate`，第 15 个 job，进快层）：每次 rust 改动的 push 跑一组
  smoke（合计 **~1 秒**），与 `docs/perf/ledger.jsonl` 的上一次同名记录比较，**大幅退化就红**。
  它**不是"再快一点"，而是"以后慢下来会被发现"**。第一轮为**只报不拦**（`continue-on-error`）
  —— CI runner 比本地吵，必须先量一次抖动再定阈值。
- **内核新增两个原语** `EnvBuilder::hide_declars` / `restore_declars`（只挪声明表、**DAG 不动**，
  保住指针同一性）；**默认路径不调用** ⇒ **判定行为零变化**（三层回归：kernel `tests/`、
  front 单测、CLI `--json` 逐字节相同）。
- **文档收口**：`docs/architecture.md` §6 内核改动台账 · `docs/PERF.md` 性能门禁一节 ·
  `AGENTS.md` · `skills/sokonanoda-ci` · `STATUS.md` / `docs/E2-HANDOVER.md`。

### 说明

- **阶段 D 的三刀（前缀复用）全部落地/量清，收益均不可测** ⇒ **默认全关**
  （`SOKO_JUDGE_ENV_REUSE` / `SOKO_WALK_REAL_ADD` 都是 opt-in），**不进入本版行为**。
  它们的热路径（同一会话再判一次，**134ms**）现在由上面的 `perf-gate` 守着。

## [0.68.0] — 2026-09-25

### Fixed

- **Infoview 目标现在稳定显示记法**：类型里含 `fun … =>` 时，内建记法 `=` 会在词法阶段
  抢走 `=>` 的 `=`（声明符号的匹配发生在常规分支之前），导致 `print_back` 解析失败、
  整条类型退回点形式 —— 学习者看到的是"好多目标都没记法化"。修法是**基础多字符算符
  更长时让路**（`=>`/`->` 优先于声明符号），并补上专门判据（撤掉修复即报根因症状）。
- `scripts/audit-wire-fields.py` **真正接进 `scripts/soko gate`**（文档此前已宣称"已进"，
  实测从未自动跑），并补上**反向验证**（`--selftest` 能咬住 R-1 的 `value_runs` 漏映射）。
- e2e 环境：`scripts/stage-lsp.js` 现在尊重 `CARGO_TARGET_DIR`（此前会静默拷到仓库里那份
  旧构建 ⇒ 真宿主 e2e 测的不是当前源码），并在 stage 后**断言版本**（不等即 exit 3）。

## [0.67.0] - 2026-09-24

### Added

- **编辑器里编出来的产物也落进项目自己的 `.sokonanoda/`**：以前只有 CLI 会把项目闭包
  产物写进模块根，语言服务器仍写全局缓存 ⇒ 编辑器编出来的东西与 `build` 预热出来的
  **不在同一处**（"vscode 和 code agent 都从这里取编译后的数据"只完成了一半）。
  现在两边都落 `<模块根>/.sokonanoda/compiled/`，`query project` / `soko/project` 的
  `artifacts` 字段能直接看到条目数与体积。**边界**：带**未落盘编辑**的编译结果仍只进
  全局缓存（项目目录只放"磁盘状态的产物"）—— 判据是"overlay 里每份文本都与磁盘一致"，
  不是"没有 overlay"（编辑器里 overlay 永远非空）。

> **本版没有**模块级批量编译：阶段 C 的实测结论是批编对课程形状**不等价**（同一模块根里
> 重名的单元会互相污染 ⇒ 假的重复声明错误）且**更慢**（同口径 3×），所以那条路按计划的
> 刹车**不接 `build`**、只留内部 API 与判据（`docs/design/module-batch.md`）；
> `build <dir>` 的行为**逐字节不变**。

- **项目产物目录 `<模块根>/.sokonanoda/`**（R-3）：项目（带 `import`）文件第一次
  `build`/`grade` 之后，编译产物落在**模块根**下，vscode 与 code agent 都从那里取，
  不再重复计算。目录里是 `compiled/<key>.json`（与全局缓存**同格式**）、`meta.json`
  （schema / 编译器版本 / build stamp / 平台）和一份**自忽略**的 `.gitignore`
  （内容一行 `*`）—— 默认什么都不用配，也不会脏你的仓库。
  实测**同一模块第二次 `build`：40.3ms → 3.6ms（11.3×）**。
  单文件（无 `import`）**不产生**这个目录；每个模块根最多留 32 条（按时间淘汰最旧）；
  `SOKONANODA_NO_PROJECT_ARTIFACTS=1` 可退回旧行为（只写全局缓存）。
- `query project` 与 `soko/project` 多一个**只读**字段
  `artifacts {dir, entries, bytes, compiler}` —— 一眼看到产物在哪、有多少、多大；
  目录不存在时是 `null`（**查询不会创建它**）。

### Changed

- **命令面板 15 条命令统一成 `Sokonanoda: <Command> (说明)`**：以前有的把前缀写进
  标题、有的靠 category，于是出现 `sokonanoda: sokonanoda: 打开目标面板 (Infoview)`、
  `sokonanoda: doctor: 诊断服务器与版本` 这类**前缀双写**。现在统一
  `category: "Sokonanoda"` + 纯标题，命令词首字母大写、括号内中文说明、括号统一半角。
- **`build --clean` 两处都清**（全局缓存 + 当前模块根的 `.sokonanoda/`）：以前项目
  条目只清全局，`rebuild` 会命中项目条目 ⇒ 表面清空、实际什么都没重编。
  `--json` 的 `build.clean` 多两个计数 `global`/`project`（`removed` 仍是总数，老消费者不变）。
- 顺手修：`SOKONANODA_NO_CACHE=1` 时 `--clean` 恒报 `removed 0`（关掉缓存后就再也
  清不掉已经写下的条目）。
- 「先清后编」的预热链路也跟着变快：CLI `build` 预热出来的产物，语言服务器现在会
  **从模块根直接读**（否则产物挪窝后编辑器读不到预热 = 变慢）。

## [0.66.0] - 2026-09-24

### Added

- **声明栏里开放练习多一行带色的目标**：卡片在类型行（`def` 还有 `:=` 值行）下面显示
  `目标 ⊢ <目标>`；多目标时逐行标 `目标 i/n`。着色与目标面板**同一批语义分段**
  （`front::semantic` 的单一来源），所以 `⊆`/`∈`/`∧` 在卡片上也是关键字色。
  闭合声明**不出现**这一行。
  `soko/goals` 的每个声明因此多了两个字段：`goal_runs`（`goal` 的分段）与
  `goals_runs`（与 `goals` **按位置对齐**；两个数组长度恒相等）——见 `docs/protocol.md`。

### Fixed

- **含 λ 的目标/类型文本整段掉色（用户报的 R-2）**：`fun (x : Nat) => z` 这类文本
  以前会**整段变成一个无色片段**。真因在词法：内建记法表里的 `"="` 被当成"声明
  符号"做最长匹配，把 `=>` 的 `=` 吃掉，剩下的 `>` 没有匹配臂 ⇒ 分类器放弃整段。
  现在 `=`（以及词法保留符号）不再交给词法符号表，`=` 仍照旧着关键字色。
  实测（同一光标）：`flawed_equalities_refuted` 的目标 **1 段 → 313 段**、
  `project_chain` **1 → 216 段**；对照组（不含 λ）**94 段不变**。

## [0.65.5] - 2026-09-24

### Added

- **声明栏多一行"真正定义"**：`def` 的卡片在类型下面显示 `:=` 之后那个值——
  类型常常看不出一个定义的本质（`Set.mem` 的类型是
  `forall (α : Type 0), α -> Set α -> Prop`，而它的值是
  `fun (α : Type 0) (a : α) (A : Set α) => A a`）。
  `theorem`/`axiom`/`inductive` 没有值，所以不显示那一行。
  树视图里同样能看到（悬停声明名）。
  `soko/goals` 的每个声明因此多了 `value` / `value_runs` 两个字段
  （`docs/protocol.md`）。

## [0.65.4] - 2026-09-24

### Added

- **记法声明的目标名现在能悬停、能跳转**：在 `infixr:80 " '' " => Set.image`
  这样的行上，悬停 `=>` 后面的名字会说清"它是谁的记法目标"；名字**在本项目闭包
  里**时（`Set.powerset`/`Set.compl`）`F12` / ctrl+点击直接跳到定义。
  名字不在闭包里（声明在别的模块、这份文件没 import）时**诚实不给跳转**，
  但**仍然按"已知引用"着色**——以前它们被当成未知标识符，看起来像"没高亮"。

- **声明栏与类型显示里 `forall` 显示成 `∀`**，`𝒫`/`ᶜ`/`∃`/`∅` 等一元与 binder
  记法也一并折出来（以前只折二元中缀）。折的时候**只换记法那几个字节**——
  binder 分组（`(A B : Set α)`）与 `Type 0` 逐字节保留。

## [0.65.3] - 2026-09-23

### Added

- **记法符号的 hover 现在给出精确范围**：悬停框**只框住符号本身**。以前不给范围、
  由编辑器按"光标词"自己猜——对 `∈` 这种单字符还行，对 `⁻¹'`/`×ˢ` 这种多字符
  符号、`𝒫` 这种星平面符号就会歪。判据里的夹具特意把光标停在 `⁻¹'` 的**中间**
  （最容易歪的位置），断言框正好覆盖 3 个字符。

### Changed

- **记法符号是一等目标（内部）**：`ResolvedTarget` 增加 `Notation` 变体，
  `Expr::Notation` 增加 `symbol_span`（"光标是不是压在符号上"现在 AST 侧直接
  有答案，不必重扫文本）。行为上：记法符号**不再**被 `rename` 当成外层 binder
  的名字（它本来就不是名字 ⇒ 改名被拒），`documentHighlight` 也不会再把外层
  binder 的每一处点亮。

## [0.65.2] - 2026-09-23

### Added

- **悬停记法符号时给出它的"原始类型"**。悬停 `∈` 现在会说清三件事：它展开成
  什么（`Set.mem`）、**它本身是什么**（`Set.mem : forall (α : Type 0),
  α -> Set α -> Prop`）、以及怎么打出来（`\in`）。本文件声明的（`⊗`）、
  语言内建的（`∧`）、`import` 来的（`∈`）三种都覆盖。
  那一行**故意不折记法**——它叫"**原始**类型"，要回答的就是"底下站着什么"。

- **记法符号可以跳转**（F12）：光标放在 `∈` 上跳到**声明它的库**里那一行
  （`lib/Set.sokonanoda` 的 `infix:50 " ∈ " => Set.mem`）。记法跨 `import`
  传播，所以跳转走的是**闭包记法表**（符号 → 声明点 + 模块名），不是本文件的
  词法扫描。

### Changed

- **贡献者门禁提速**（不影响用户）：新增 `scripts/soko gate --fast`
  （**~30 秒**，迭代用：fmt + clippy + 改动过的 crate 的单测 + 课程门禁）；
  完整 `scripts/soko gate` 从 ~15 分钟降到 **~5 分钟**（课程门禁改走持久编译
  缓存，164s → 0s）。完整门禁的**语义一字未改**，仍是提交/推送/CI 的判据。

## [0.65.1] - 2026-09-21

### Fixed

- **记法符号在目标栏里有了颜色**。0.65.0 让 goal / 类型行显示记法（`A ⊆ B`），
  但那些符号本身在语义着色里是**裸文本**——`⊆` 没有 kind、`↔` 更糟（被当成
  **未知标识符**，因为 `↔`/`¬`/`≠` 不在数学符号码点类里，不喂给词法就切成
  标识符）。现在源里声明的与语言内建的记法符号都着成关键字色
  （`∧ ∨ ↔ ¬ = ≠` 与 `∈ ⊆` 之类）。

- **目标文本里的导入名不再标成"未知标识符"**。以前只看**入口文件**的声明表
  ⇒ 项目文件里 `Set`/`Set.mem`/`Set.subset` 全是未知标识符（0.65.0 之后 goal 里
  全是这些名字，一眼就看得出来）。现在闭包级声明表参与分类。

- **折过的子树被应用时不再改变语义**（内部正确性）。`(Set.mem α a A) B` 这种
  "把一个完整应用当作函数再用一次"的写法，以前会折成 `a ∈ A B`——它**重新解析**
  是 `Set.mem α a (A B)`，**换了个意思**。现在折到应用脊的根并保留括号：
  `(a ∈ A) B`。

### Notes

- 判定**一个字节没动**：折叠只作用在展示副本上（`ty_text` 与 `by` 步进的报告层
  字段），judge 的输入（`goal` / `binders[].ty` / `sub_goals[].ty`）没碰。
  课程门禁计数逐项不变（36 目标 · 328 checked · 99 open · 0 判负）。
- 诊断用开关 `SOKO_NO_NOTATION_FOLD=1` 可关掉折叠。

## [0.65.0] - 2026-09-21

### Changed

- **目标栏 / 声明卡片 / 假设行第一次显示记法**（线 C）。以前 Infoview 里看到的
  是**内核 pp 的点名形式**——`Set.subset α A B`、`Set.mem α a A`、`Iff …`；
  源文件里写的是 `A ⊆ B`、`a ∈ A`、`↔`。用户报的那条（缺口 **G-26**）就是它，
  现在**关账**。

  ```
  以前 | forall (α : Type 0) (A B : Set α), Set.subset α A B -> (forall (a : α), Set.mem α a A -> Set.mem α a B)
  现在 | forall (α : Type 0) (A B : Set α), A ⊆ B -> (forall (a : α), a ∈ A -> a ∈ B)
  ```

  **只有记法那几段被替换**：binder 分组（`(A B : Set α)`）、`Type 0` 的写法、
  折行与缩进都**逐字节保留**——不做整句重渲染。

  覆盖到的 surface：

  | surface | 之前 | 现在 |
  |---|---|---|
  | 根状态（光标在 `by` / `sorry` 上） | 点名 | **记法** |
  | 声明卡片的类型 | 点名 | **记法** |
  | `by` 步进（含 `apply` 出来的子目标） | `apply` 之后是点名 | **记法** |
  | 假设行的类型 | 本来就有 | 不变（源级渲染） |
  | 无 `by` 的开练习目标 | 本来就有 | 不变（源级渲染） |

  判定**一个字节没动**：折叠只作用在**展示副本**上（`ty_text` 与 `by` 步进的
  报告层字段），judge 的输入（`goal` / `binders[].ty` / `sub_goals[].ty`）没碰。
  课程门禁计数逐项不变（36 目标 · 328 checked · 99 open · 0 判负）。

  诊断用开关 `SOKO_NO_NOTATION_FOLD=1` 可关掉折叠（关掉后根状态 / 声明类型 /
  `by` 步进都回到点名，另外两个 surface 不受影响——它们的记法来自源级渲染）。

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
