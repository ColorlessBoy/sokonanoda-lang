# 记法输入法（`\xxx` 缩写）+ hover 提示：设计与实现计划

> 类型：**只读调研 + 设计计划**（本轮只新建本文件，未改任何现有文件）。
> 审计对象：`sokonanoda` **0.61.0**（`Cargo.toml` 的 `version`），工作树 = 0.61.0
> 轮次的**未提交改动**（`git status` 89 项，其中 `courses/set-theory/lib/Set.sokonanoda`
> 与 `units/notation-cheatsheet.sokonanoda` 正在做「记法搬进库」的迁移——见 §1.6）。
> 日期：2026-09-19。证据纪律与 `docs/notes/course-lean-style/notation-audit.md` 同款：
> **实测优先于读代码**，每条给 `文件:行号`；只读代码没实测的标 **【代码读】**。
> 判卷/编译一律走本机命令（禁止官方 Lean 工具链）：
> `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json <文件>`。
>
> 用户原话（`REQUIREMENTS.md` §9 待追加）：
> 「1. 要考虑 notation 如何输入，应该像 lean4 一样 'xxx' 替换，同时 hover 内容提示用户如何输入对应符号」。

> **⚠️ 评审修正（2026-09-19）**：① §2.2「我们采用的表」只收了主缩写 + 1 条别名，
> Lean 还有一批**多字母**别名没收（`\an \lr \all \ex \lnot \member \subset \ss \un
> \varnothing`）——设计 §2.1 已把它列为**待拍板**项；② 本文的 Lean 取证是**手抄**
> （本仓不 vendor 也不下载 Lean 的任何东西，硬规则 2），上游会漂移 ⇒ 实施时（NI-1）
> 要按设计 §2 的注**重新逐行 diff** 一次；③ 本次评审时上游源码取不到（网络受限），
> 所以**没有**复现那份取证。

---

## 0. 结论速览

| # | 问题 | 结论 |
|---|---|---|
| 1 | VS Code 扩展有补全/hover provider 吗 | **没有**。`editor/vscode/extension.js`（1880 行）只注册树/命令/Infoview，**零** `registerCompletionItemProvider` / `registerHoverProvider`；补全与 hover 全部来自 LSP（`editor/vscode/test-extension-host.js:178-180` 的 `languages` stub 只有 `onDidChangeDiagnostics` 就是证据） |
| 2 | 语法高亮在哪 | `editor/vscode/syntaxes/sokonanoda.tmLanguage.json`（139 行）；符号走 `mathsymbols` 规则（`:133-136`，匹配 `U+2200–22FF`/`U+2A00–2AFF`/`\`）。**声明驱动的符号**（`𝒫 ᶜ '' ⁻¹' ×ˢ`）**不在**码点类里 ⇒ 今天不被这条规则着色 |
| 3 | 缩写表放哪 | **Rust 单一真相源**（新模块 `crates/front/src/notation_input.rs`）+ VS Code 侧一份**受守护测试比对的镜像**（先例：`front::semantic::KEYWORDS` ↔ tmLanguage JSON，守护测试 `crates/cli/tests/extension.rs:1079-1116`） |
| 4 | 输入机制怎么做 | **Lean 忠实的做法不是补全项**，而是客户端**缩写重写器**（`@leanprover/unicode-input`：`\` 是 leader，Tab = `lean4.input.convert`，缩写成词且不是更长缩写的**前缀**时即时替换）——见 §3 方案 A |
| 5 | hover 落点 | **只在 LSP 侧**（VS Code hover 走 LSP；DSH 的 `lsp` 工具有 hover，见 `docs/design/deepseek-harness.md` D8；扩展里再注册 `HoverProvider` 会被 VS Code 合并成两份重复内容） |
| 6 | hover 今天对符号有反应吗 | **实测：三种行为不一致**——内建 `∧` 显示外层表达式（`a ∧ b : Prop`）；**本文件声明**的符号（`⊗`）**完全静默**（被关键字闸门吃掉）；import 来的符号今天连报告都没有（§1.6 缺陷） |
| 7 | 有没有前置阻断 | **有，P0**：`crates/lsp/src/lib.rs:160-166` 在单文件 parse 失败时把**已经装好的项目报告**丢掉 ⇒ 编辑器里所有「用 import 记法的课程文件」hover/documentSymbol/goals 全死 + 假 `notation-unknown-symbol` ERROR（§1.6，新缺口，未进台账） |
| 8 | 量级 | P0 约 30 行 + 测试 40；P1（表 + hover）约 350 行；P2（客户端重写器）约 400 行；P3（课程/文档/技能）约 80 行。**合计 700–900 行**（含测试） |

---

## 1. 现状（逐条带证据）

### 1.1 VS Code 扩展

| 关注点 | 现状 | 证据 |
|---|---|---|
| 补全 | 无客户端 provider，只有 LSP 补全 | `grep -n "CompletionItemProvider\|HoverProvider" editor/vscode/extension.js` → 无命中（`node_modules/` 与 `.vscode-test/` 的命中不算） |
| hover | 无客户端 provider；hover 由 `vscode-languageclient` 转发给服务器 | `editor/vscode/extension.js:23`（`require("vscode-languageclient/node")`）；`package.json` 的 `contributes` 无 `hover`/`completion` 相关项 |
| 语法高亮 | TM 语法一份；关键字/命令/sort/构造子/常量/变量/数字/洞/运算符/binder + **数学符号** | `editor/vscode/syntaxes/sokonanoda.tmLanguage.json:20-37`（patterns）、`:133-136`（`mathsymbols`） |
| 触发字符 | **没有**（扩展不注册 provider ⇒ 无 `triggerCharacters` 可配） | 同上 |
| snippets | **没有** `contributes.snippets` | `editor/vscode/package.json`（全文读过；`contributes` = languages/grammars/commands/viewsContainers/views/menus/configuration/configurationDefaults/keybindings） |
| 现有命令/键位 | 14 条命令 + 5 条键位（`alt+s/n/shift+n/b/shift+b`），全部 `editorLangId == sokonanoda` | `editor/vscode/package.json` `contributes.commands` / `contributes.keybindings`；契约测试 `crates/cli/tests/extension.rs:71-101`、`:1376-1380` |
| 注册顺序约束 | **每个 view/命令必须在第一个 `await` 之前注册** | `editor/vscode/extension.js:1683-1693`（ORDER IS LOAD-BEARING，附历史事故） |
| 代码围栏辅助 | `codeMarkdown(text)` = `appendCodeblock(text, "sokonanoda")` | `editor/vscode/extension.js:404-408`；契约 `crates/cli/tests/extension.rs:858-873` |
| 隐藏字符高亮 | 已为 `[sokonanoda]` 关掉 `editor.unicodeHighlight.ambiguousCharacters` | `editor/vscode/package.json` `configurationDefaults`；契约 `crates/cli/tests/extension.rs:351-365` |

### 1.2 LSP

| 关注点 | 现状 | 证据 |
|---|---|---|
| 能力声明 | `completionProvider` 有，但 **`trigger_characters: None`**；`hoverProvider: true` | `crates/lsp/src/lib.rs:961-967`、`:941` |
| 补全来源 | ① 光标处作用域 binder ② `front::semantic::keywords()` ③ `sorts()` ④ `compile::PRELUDE_NAMES` ⑤ 本文档自己的声明（`decl.ty_text` 作 `documentation`） | `crates/lsp/src/lib.rs:1353-1440`；`:1364-1377`、`:1379-1385`、`:1387-1394`、`:1396-1403`、`:1405-1438` |
| 补全 `documentation` | **有**，且用 `sokonanoda` 围栏（`code_block`） | `crates/lsp/src/lib.rs:1429-1434` |
| hover 数据源 | 请求期 `probed_report()` + `declaration_kinds(doc.text())`；分支顺序：tactic hover → 半表达式 → **关键字闸门** → 括号 → 精确 span → ±2 邻近回退 → 声明 | `crates/lsp/src/lib.rs:1071-1188`（闸门 `:1100-1104`，声明分支 `:1144-1187`） |
| 围栏契约 | `const CODE_LANG = "sokonanoda"` + `code_block()`，**凡渲染 `.sokonanoda` 文本一律围栏** | `crates/lsp/src/lib.rs:739-749`；设计 `docs/design/goal-rendering.md:170-194` §7；测试 `crates/lsp/src/tests/hover.rs:230`（`code_fences_always_use_the_sokonanoda_language`） |
| 符号在 hover 里 | **没有任何记法分支**：hover 从不回答「这个符号是哪条记法、怎么输入」 | `grep -n "notation" crates/lsp/src/lib.rs` → 0 命中 |
| 关键字闸门 | `semantic_kind_at(...) == Keyword` ⇒ **`return Ok(None)`** | `crates/lsp/src/lib.rs:1100-1104`；`crates/lsp/src/render.rs:347-356` |

### 1.3 单一真相源 + 守护测试的既有纪律（新表要接进同一条纪律）

| 环节 | 现状 | 证据 |
|---|---|---|
| Rust 真相源 | `KEYWORDS`（25 条）+ `SORTS`；`pub fn keywords()` / `sorts()` 是编辑器补全与语义高亮的唯一来源 | `crates/front/src/semantic.rs:42-87`、`:89-99` |
| 客户端镜像 | `syntaxes/sokonanoda.tmLanguage.json` 的 `keywords`/`commands`/`sorts` 交替式**逐字相等** | `editor/vscode/syntaxes/sokonanoda.tmLanguage.json:60-75` |
| 守护测试 | `tm_grammar_keywords_follow_the_single_source`：读 JSON → `alternation_words()` 抽词 → 与 `keywords()+sorts()+(forall,∀)` 排序去重后 `assert_eq!` | `crates/cli/tests/extension.rs:1079-1116`；辅助 `:1007-1026`；`vscode_dir()` `:11-16` |
| 第二道守护 | `tm_grammar_declares_every_semantic_scope`（每个 `SemanticKind::tm_scope()` 都有规则） | `crates/cli/tests/extension.rs:1051-1078` |
| Node 层清单守护 | `npm run test:unit` 必须覆盖 4 个 Node 测试文件（新增文件要同步这里） | `crates/cli/tests/extension.rs:602-627` |
| 版本守护 | `Cargo.toml` version == `package.json` version | `crates/cli/tests/extension.rs:975-1006` |

**新表接法（建议）**：Rust 常量 `front::notation_input::ABBREVIATIONS`（唯一真相源）
→ LSP hover/补全直接读它；
→ VS Code 侧 `editor/vscode/src/abbreviations.json`（checked-in 镜像，只含 `{abbrev: symbol}`）
→ 新守护测试 `abbreviation_table_follows_the_single_source`（同 `:1079` 的写法：读 JSON、
按 key 排序、与 Rust 常量逐项 `assert_eq!`）。改一个缩写必须同轮改两处，否则 CI 红。

### 1.4 记法表在哪（hover 要的数据从哪来）

| 层 | 数据 | 能不能拿到 | 证据 |
|---|---|---|---|
| 语言内建 | `∧ ∨ ↔ ¬`（`And/Or/Iff/Not`，优先级 35/30/20/40） | `const BUILTIN_NOTATIONS` 是 **`const`（私有）**，只有 `pub(crate) fn builtin_notation_symbols()`；LSP 用不了 ⇒ 需要新 `pub` 访问器 | `crates/front/src/parser.rs:2991-2996`、`:2998-3004`；入表 `:147-164`；不可重声明 `:966-973` |
| `→` / `∀` | **不是记法**：`→` 是词法别名（`TokenKind::Arrow`）、`∀` 是 `TokenKind::Forall` | 与记法表无关 | `crates/front/src/token.rs:480-491`（`is_math_symbol` 注释）；`crates/front/src/semantic.rs:264`（`Forall ⇒ Keyword`） |
| 文件内声明 | `infix:N` / `infixl:N` / `infixr:N` / `prefix:N` / `postfix:N` / `notation` / `binder_notation` → `Command::Notation{ symbol, precedence, assoc, target, scope, span }` | ✅ 公开可读；`Command::notation_decl() -> Option<NotationDecl>`（`NotationDecl` **没**从 `lib.rs` re-export，但字段可读） | `crates/front/src/ast.rs:129-163`、`:609-627`；`crates/front/src/token.rs:17-25`（`NOTATION_COMMANDS`） |
| 库/import 声明 | 例：`∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹' ×ˢ` 十条全在 `courses/set-theory/lib/Set.sokonanoda:164-177`；`∃` 在 `lib/Exists.sokonanoda:100` | ⚠️ 只能从**闭包模块源码**里扫：`QueryDoc::project_modules() -> Option<&[ModuleReport]>`，`ModuleReport.source` 是编译时那份文本 | `crates/front/src/query/mod.rs:229-237`；`crates/front/src/project/report.rs:90-105` |
| 语义 token 表 | `Names.notations: HashMap<symbol, SemanticKind>` **只收本文件 `Command::Notation`**，且符号一律归 `Keyword` | ❌ 不是记法真相源；且 `semantic_tokens()` 用 `lex_prefix`（`tokenize` 空符号表）⇒ 声明驱动的符号（`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`）根本不产 `Sym` token | `crates/front/src/semantic.rs:378-385`、`:532-541`、`:741-744`、`:775-785`、`:343-363` |
| CLI/`query` | 今天**没有**任何记法查询 op（`check/state/goals/holes/hints/reduce/project` 七个） | 可选新增 `query notations`（非必须） | `docs/protocol.md`；`crates/front/src/query/mod.rs:291-676` |

**结论**：hover 需要一张「符号 → （缩写，目标名，来源模块）」表。P1 最小实现 =
`BUILTIN_NOTATIONS`（新 `pub` 访问器）+ **容错扫描**：对 `doc.text()` 与
`QueryDoc::project_modules()` 的每份 `source` 做 `tokenize_with_symbols` + 识别
`NOTATION_COMMANDS` 三元组（**不能**直接 `crate::parse`：入口文件在单文件模式下本来
就 parse 失败，见 §1.6）。**容错扫描**要新写约 40 行 `pub fn scan_notation_decls(src)`，
放 `notation_input.rs`，与 `token.rs:600` 的 `scan_notation_symbols`（`pub(crate)`）同族。

### 1.5 hover 今天对符号有没有反应（**实测**，三种行为不一致）

方法：真 LSP over stdio（`target/debug/sokonanoda-lsp`，本轮新构建），`initialize` +
`didOpen`（内存文本）→ `textDocument/hover`。

| 样例 | 符号归属 | hover 结果 |
|---|---|---|
| `theorem and_comm_demo (a b : Prop) (h : a ∧ b) : b ∧ a := …` | **语言内建**（文件里没声明） | ```` ```sokonanoda\na ∧ b : Prop\n``` ```` ——显示**外层记法表达式** |
| `infix:35 " ⊗ " => And` + `theorem t (a b : Prop) (h : a ⊗ b) : a := And.left a b h` | **本文件声明** | **`null`（完全静默）**——被 `lib.rs:1100-1104` 的关键字闸门吃掉（`semantic.rs:537-541` 把已声明符号归 `Keyword`） |
| 同文件 `And.left` | 普通声明名 | `And.left : forall (a b : Prop), And a b -> a` ✅ |
| `courses/set-theory/units/unit01-*.sokonanoda` 的 `∈`（import 来的） | **import 声明** | `null`——**不是**闸门，是 §1.6 的报告缺失 |

⇒ 同一个符号的 hover 行为**取决于它是内建 / 本文件声明 / import 来的**。新功能必须
在关键字闸门**之前**插一条记法分支，把三种情况收敛成一种。

### 1.6 【P0 阻断】单文件 parse 失败会吃掉项目报告（新缺口，未进台账）

**症状**（真 LSP 实测，0.61.0 工作树）：

| 文件 | 发布的诊断 | `documentSymbol` | `hover`（整行暴力扫 50 列） | `soko/goals` | `soko/project` | CLI `--json` |
|---|---|---|---|---|---|---|
| `courses/set-theory/units/unit01-sets-membership.sokonanoda` | **`notation-unknown-symbol`（ERROR）** | `null` | 全 `null` | `{decls:[]}` | 3 模块 compiled / **0 errors** | **exit 0** |
| `…/units/notation-cheatsheet.sokonanoda` | 同上 | `null` | 全 `null` | `{decls:[]}` | 3 模块 / 0 errors / 46 decls | **exit 0**（18 checked + 3 open） |
| `…/units/unit04-extensionality-identities.sokonanoda`（不用 import 记法） | 正常 | 11 符号 | OK | 11 | 3 模块 | exit 0 |
| `course/unit11-project/Exercises.sokonanoda`（能单文件 parse） | `sorry` | 2 | OK | 2 | 2 模块 | exit 0 |

**最小复现**（同一 LSP 进程内只换文本；文件路径取课程目录下、`import lib.Set`）：

```text
import lib.Set

theorem t (α : Type) (A B : Set α) : Set.subset α A B := sorry   → diags=[sorry]，symbols=1，hover=OK
import lib.Set

theorem t (α : Type) (A B : Set α) : A ⊂ B := sorry              → diags=[notation-unknown-symbol]，symbols=null，hover=null
```

**根因（两处）**：

1. `crates/lsp/src/lib.rs:158-166`——`set_text_with_overlay(...)` **已经**把项目入口报告装进
   `self.doc.report`（`crates/front/src/query/mod.rs:132-151`：`if let Some(report) = self.project…entry_report()`），
   紧接着 LSP 又做
   `if self.doc.parse_error.is_some() { self.doc.report = None; return; }`。
   `parse_error` 是**单文件** parse 的产物；「用了 import 来的记法」正是
   `is_project_source`（`crates/front/src/project/mod.rs:62-79`）**特意**要改判走闭包的那种
   失败——闭包编译成功，报告却被这一行丢掉。
2. `crates/lsp/src/lib.rs:78-88`（`Doc::diagnostics`）——`parse_error` **优先于**报告，
   于是那条单文件 parse 错误被当成 ERROR 发布（假诊断；`soko/project` 与 CLI 都判它绿）。

**影响**：`hover` / `documentSymbol` / `codeAction` / `inlayHint` / `soko/goals`（`parsable()`
闸门，`crates/front/src/query/mod.rs:375`、`:452-453`）在**所有已迁移到库记法的课程文件**里
全死。**这正是用户要的 hover 提示最需要工作的那批文件。** 台账 `docs/gaps/ledger.jsonl`
无对应条目（无 G-xx 覆盖「LSP 项目报告被单文件 parse_error 丢弃」）。

**建议修法（P0）**：闭包编译成功时清掉 `parse_error`（或改成
`if self.doc.parse_error.is_some() && <没有可用的项目报告> { report = None }`），
并让 `Doc::diagnostics` 在项目报告可用时以项目报告为准；补 LSP 回归测试
（`crates/lsp/src/tests/project.rs`：`import` + 依赖记法 ⇒ 诊断为空、`documentSymbol`
非空、hover 非 null）。

---

## 2. 缩写表

### 2.1 Lean 4 取证（web 调研，2026-09-19）

**机制**：Lean 4 的「Unicode input」**不是** LSP 补全，而是 npm 包
[`@leanprover/unicode-input`](https://github.com/leanprover/vscode-lean4/tree/master/lean4-unicode-input)
的**客户端缩写重写器**：

- 表：[`abbreviations.json`](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json)（1859 行，key = 不带 leader 的缩写）；
- 逻辑：[`AbbreviationProvider.ts`](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/AbbreviationProvider.ts)、[`AbbreviationRewriter.ts`](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/AbbreviationRewriter.ts)、[`TrackedAbbreviation.ts`](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/TrackedAbbreviation.ts)；
- VS Code 接线：[`vscode-lean4/src/abbreviation/`](https://github.com/leanprover/vscode-lean4/tree/master/vscode-lean4/src/abbreviation)（含 **`AbbreviationHoverProvider.ts`**——正是用户要的「hover 提示怎么输入」）；
- 手册：[Unicode input](https://github.com/leanprover/vscode-lean4/blob/master/vscode-lean4/manual/manual.md#unicode-input)。

**触发方式（逐条）**：

- `\` 是**可配置的 leader**（`lean4.input.leader`，默认 `\`）——**不是**补全触发字符
  （Lean 服务器的 `completionProvider.triggerCharacters` 只有 `"."`，见
  [`Watchdog.lean`](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/Watchdog.lean#L1567)）；
- **Tab** = `lean4.input.convert`（`"when": "editorTextFocus && lean4.input.isActive"`）；
  空格**不是**确认键，但任何**不能再延长**缩写的字符会让它 `finished = true` 并强制替换；
- **即时替换**只在「缩写完整**且**不是任何更长缩写的前缀」时发生
  ⇒ `\forall` 立刻变 `∀`，而 `\to` / `\in`（是 `toa`/`int`/`inter`… 的前缀）**不会**，
  要空格或 Tab 收尾；
- **大小写敏感**（`l`→← vs `L`→Λ）；**一个符号多别名**（hover 会列出全部）；
- `lean4.input.customTranslations` 可覆盖/追加。

**表（逐字取证；`L` = abbreviations.json 行号）**：

| 符号 | Lean 4 缩写 | 来源 |
|---|---|---|
| ∧ | `\and` `\an` `\wedge` | [L328](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L328) |
| ∨ | `\or` `\v` `\vee` | [L356](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L356) |
| ↔ | `\iff` `\lr` `\lr-` `\<->` `\leftrightarrow` | [L269](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L269) |
| ¬ | `\not` `\neg` `\lnot` `\!` | [L176](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L176) |
| → | `\to` `\r-` `\->` `\imp` `\r` | [L441](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L441) |
| ∀ | `\forall` `\all` | [L1200](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L1200) |
| ∃ | `\exists` `\ex` | [L245](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L245) |
| ≠ | `\ne` `\neq` `\eqn` `\=n` | [L202](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L202) |
| ∈ | `\in` `\member` `\mem` | [L256](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L256) |
| ⊆ | `\sub` `\subset` `\subseteq` `\ss` `\subseteqq` | [L750](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L750) |
| ∪ | `\cup` `\union` `\un` | [L985](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L985) |
| ∩ | `\cap` `\inter` `\intersection` `\i` | [L1031](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L1031) |
| `\` | `\setminus`（打 `\\` 也出 `\`） | [L809](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L809) |
| ∅ | `\empty` `\emptyset` `\varnothing` | [L247](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L247) |
| 𝒫 | `\powerset` `\McP`（**`\P` 是 Π，不是这个**） | [L585](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L585) |
| ᶜ | `\compl` `\complement` `\^c`（`\complementprefix` → ∁，另一个字符） | [L996](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L996) |
| `''` | **无**（表里没有 `''`/`\image`/`\im`；`\prime` 给的是单引号 `′`） | — |
| ⁻¹' | `\preim` `\preimage` | [L581](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L581) |
| ×ˢ | `\xs`（`\xf` → ×ᶠ） | [L1845](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json#L1845) |

**被证伪的候选**：`\P`→Π · `\times`→×（普通乘号）· `\prod`→∏ · `\ssubset`→⊂（真子集）·
`\xssubset`/`\timesˢ`→**不存在**（用 `\xs`）。

### 2.2 我们采用的表（19 个符号；建议落进 `front::notation_input`）

原则：**主缩写逐字照抄 Lean**（硬规则 3：教学语法是真实 Lean 4 的子集 ⇒ 肌肉记忆可迁移）；
**单字母别名（`\i` `\v` `\r` `\l` `\a`）一律不抄**——它们是前缀陷阱，对学习者只有害处；
每条别名都进表（hover 列出全部）。

| 符号 | 主缩写 | 别名 | 我们今天的支持 | 备注 |
|---|---|---|---|---|
| ∧ | `\and` | `\wedge` | ✅ 内建记法 `parser.rs:2992` | |
| ∨ | `\or` | `\vee` | ✅ 内建 `:2993` | |
| ↔ | `\iff` | `\leftrightarrow` | ✅ 内建 `:2994` | |
| ¬ | `\not` | `\neg` | ✅ 内建 `:2995` | |
| → | `\to` | `\imp` | ✅ 词法别名（非记法） | hover 文案改成「函数空间，语言内建」 |
| ∀ | `\forall` | — | ✅ 关键字 | |
| ∃ | `\exists` | — | ✅ `lib/Exists.sokonanoda:100` 的 `binder_notation` | |
| ≠ | `\ne` | `\neq` | ❌ **语言今天没有 `≠`**（全仓无 `Ne` 名字，见 `notation-audit.md` §0 第 5 条） | 表里先**留位并标 `supported: false`**，hover 不承诺 |
| ∈ | `\in` | `\mem` | ✅ `lib/Set.sokonanoda:164` | |
| ⊆ | `\sub` | `\subseteq` | ✅ `:165` | |
| ∪ | `\cup` | `\union` | ✅ `:166` | |
| ∩ | `\cap` | `\inter` | ✅ `:169` | |
| `\` | `\setminus` | — | ✅ `:170` | **与 leader 同字符**，重写器必须只在「`\`+字母且整词命中」时替换 |
| ∅ | `\empty` | `\emptyset` | ✅ `:171` | |
| 𝒫 | `\powerset` | — | ✅ `:173` | `\P` **不能**用（Lean 里是 Π） |
| ᶜ | `\compl` | `\complement` | ✅ `:174` | |
| `''` | **不给** | — | ✅ `:175` | 与 Lean 一致：直接打两个单引号（hover 明说「Lean 也没有缩写」） |
| ⁻¹' | `\preim` | `\preimage` | ✅ `:176` | |
| ×ˢ | `\xs` | — | ✅ `:177` | |

---

## 3. 方案对比（输入）

| | 方案 A：客户端缩写重写器（**推荐**） | 方案 B：VS Code 补全项 | 方案 C：命令 + 键位 | 方案 D：LSP 侧补全 |
|---|---|---|---|---|
| 形态 | `workspace.onDidChangeTextDocument` 驱动，`\`+字母整词命中 → 即时替换；Tab 命令强制替换 | `registerCompletionItemProvider(["\\"])`，`insertText` = 符号 | `sokonanoda.insertSymbol` / `convertAbbreviation` 命令 + 键位 | `textDocument/completion` 增 `\` 触发 + `textEdit` |
| 与 Lean 一致 | ✅ 逐条一致（leader/Tab/即时替换/别名） | ❌ Lean 不这么做 | ⚠️ 只覆盖 Tab 那一半 | ❌ |
| 改哪些文件 | 新 `editor/vscode/src/abbreviations.js`（表）+ `abbreviation-rewriter.js`（状态机）；`extension.js` 接线；`package.json`（命令/键位/配置）；`test-extension-host.js` | `extension.js`；`package.json` 无需改 | `extension.js`；`package.json` | `crates/lsp/src/lib.rs`；`crates/front/src/notation_input.rs` |
| 估行 | 表 40 + 重写器 160–200 + 接线 30 + 测试 180 | 80–120 + 测试 60 | 60 + 测试 40 | 80 Rust + 测试 60 |
| 非 VS Code harness 能用吗 | ❌（客户端功能） | ❌ | ❌ | ⚠️ **DSH 不消费补全**（`docs/design/deepseek-harness.md` D8：DSH 的 `lsp` 只有 goToDefinition/findReferences/goToImplementation/hover；本机 `grep -rn completion packages/lsp`（DSH 检出）无命中）；opencode 也没接补全 ⇒ **唯一消费者仍是 VS Code**，而 VS Code 有 A 这条更好的路 |
| 主要风险 | 即时替换的状态机（undo/多光标/与 LSP 编辑竞争） | `\` 不是 word char ⇒ `filterText`/替换范围要显式 `textEdit`；与 `\`（集合差）触发冲突；**不是 Lean 行为** | 单独用不够 | 与方案 A 同时上会双份候选 |

**推荐：A（+ C 作为 A 的 Tab 那一半），D 不做。** 若将来有 harness 需要补全列表，
再把 D 作为 `Ctrl+Space` 增强（P3 可选）。

---

## 4. hover 提示（唯一落点：LSP）

### 4.1 落点与数据流

```text
hover(pos)
 ├─ 若光标落在某个「记法符号」上（新分支，必须在 lib.rs:1100 的关键字闸门之前）
 │    ├─ 符号表 = BUILTIN_NOTATIONS（新 pub 访问器）
 │    │            + scan_notation_decls(doc.text())
 │    │            + scan_notation_decls(每个 project_modules() 的 source)
 │    ├─ 缩写 = front::notation_input::abbreviations_for(symbol)
 │    ├─ 外层表达式/类型 = 沿用现有 hover_type_at/expr_hover（有就带上，没有就只给输入提示）
 │    └─ 组 markdown：```sokonanoda 表达式/类型``` + 散文行「输入方式：`\in`」+（P2）```sokonanoda 记法声明```
 └─ 其余分支**逐字不动**
```

### 4.2 文案示例（中文，学习者视角）

内建符号（无声明可显示）：

> ````markdown
> ```sokonanoda
> a ∧ b : Prop
> ```
> 输入方式：`\and`（也可用 `\wedge`）——按 Tab 或空格确认。
> ````

import 来的符号（P2 加第三块）：

> ````markdown
> ```sokonanoda
> a ∈ A : Prop
> ```
> 输入方式：`\in`（也可用 `\mem`）——按 Tab 或空格确认。
> ```sokonanoda
> infix:50 " ∈ " => Set.mem
> ```
> ````

`''`（Lean 无缩写）：

> ````markdown
> ```sokonanoda
> f '' s : Set β
> ```
> 输入方式：直接输入两个单引号 `''`（Lean 4 也没有这个缩写）。
> ````

未支持符号（`≠`）：

> ````markdown
> ```sokonanoda
> a ≠ b
> ```
> `≠` 还没有进这门语言（先写 `Not (Eq a b)`）；Lean 4 里输入方式是 `\ne`。
> ````

### 4.3 契约

- **§7 围栏**：凡 `.sokonanoda` 文本（表达式/类型/记法声明）一律 ` ```sokonanoda `；
  「输入方式：…」是散文，允许行内代码（`docs/design/goal-rendering.md:188-190` 明说
  单反引号行内代码不可着色，只用于散文里提到单个词）。新 hover 要进
  `code_fences_always_use_the_sokonanoda_language` 的断言面。
- **只增不改**：现有 `hover_on_operator_shows_enclosing_type`（`crates/lsp/src/tests/hover_brackets.rs:42-74`）
  断言 `->` 或 `Prop` 出现在值里——追加输入提示行**不会**让它红。
- **关键字闸门**：`hover_on_keyword_returns_none`（`crates/lsp/src/tests/hover.rs:270-295`）用的是
  `fun`，不是记法符号 ⇒ 新分支在前不会让它红。
- 不承诺：tactic 词、`→`（非记法）、未声明符号（继续静默，避免噪音）。

---

## 5. 落点清单 + 估行

| 阶段 | 文件 | 改什么 | 估行 |
|---|---|---|---|
| **P0** | `crates/lsp/src/lib.rs`（`:160-166`、`:78-88`） | 项目报告可用时不清 `report`；诊断以项目报告为准 | 10–20 |
| | `crates/lsp/src/tests/project.rs` | 回归：`import` + 依赖记法 ⇒ 无诊断 + `documentSymbol` 非空 + hover 非 null | 40 |
| **P1** | `crates/front/src/notation_input.rs`（**新**） | `ABBREVIATIONS` 表 + `abbreviations_for(symbol)` + `scan_notation_decls(src)` + `builtin_symbols()` | 120–150 |
| | `crates/front/src/lib.rs` | `pub mod notation_input;`（re-export 保持稳定） | 2 |
| | `crates/front/src/parser.rs`（`:2998-3004`） | 内建符号访问器提升为 `pub`（或由 `notation_input` 包一层） | 5 |
| | `crates/lsp/src/lib.rs`（hover） | 记法分支（在 `:1100` 之前）+ 文案装配 | 60 |
| | `crates/lsp/src/tests/hover.rs` | 内建/本文件/import 三种符号的 hover 文案 + 围栏断言 | 90 |
| | `crates/front/src/notation_input.rs`（tests） | 表不变量（每符号 ≥1 缩写、缩写唯一、纯 ASCII、主缩写不是别名的前缀） | 60 |
| | `editor/vscode/src/abbreviations.json`（**新**） | 镜像表 | 40 |
| | `crates/cli/tests/extension.rs` | `abbreviation_table_follows_the_single_source` | 40 |
| **P2** | `editor/vscode/src/abbreviations.js`（**新**） | 读 JSON + 前缀/别名查询（纯函数，可单测） | 60 |
| | `editor/vscode/src/abbreviation-rewriter.js`（**新**） | 文档变更驱动的状态机（leader、即时替换、Tab 强制、光标离开收尾） | 160–200 |
| | `editor/vscode/extension.js` | 接线（`onDidChangeTextDocument` + 命令 + `setContext("sokonanoda.input.isActive")`），**在第一个 await 之前注册**（`:1683`） | 30 |
| | `editor/vscode/package.json` | 命令 `sokonanoda.convertAbbreviation` / `sokonanoda.insertSymbol`；键位 Tab（`when: editorTextFocus && editorLangId == sokonanoda && sokonanoda.input.isActive`）；配置 `sokonanoda.input.leader` | 40 |
| | `editor/vscode/test-extension-host.js` | stub 增 `workspace.onDidChangeTextDocument` / `TextEdit` 捕获（`languages` stub 见 `:178-180`）+ 3–5 条用例 | 120 |
| | `editor/vscode/src/test/extension.test.js` | 真宿主：键入 `\and` + Tab → 文档变 `∧`；hover `∈` 含 `\in` | 60 |
| **P3** | `courses/set-theory/units/notation-cheatsheet.sokonanoda` | 「怎么输入」一栏 + 一段说明（**加在对照表旁，不重写页面**） | 20 |
| | `skills/sokonanoda-teacher/SKILL.md`（`:131-149`） | 加「教学习者用缩写」；顺手修 **stale ⑦**（「第二刀未做」在 0.60.0 已完成） | 12 |
| | `editor/vscode/README.md` / `CHANGELOG.md` / `package.json.version` | 同一轮同步（硬规则）；minor bump（新能力） | 20 |
| | `docs/design/notation-input.md`（**新**，实施前先写） | 设计定稿（本笔记的结论搬过去 + as-built 回填） | 150 |

合计 **≈ 700–900 行**（含测试），其中 Rust ≈ 400，JS ≈ 400，文档/课程 ≈ 200。

---

## 6. 测试（三层 + Rust 契约层）

| 层 | 命令 | 新用例 |
|---|---|---|
| front 单测 | `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test -p sokonanoda-front --locked` | 表不变量；`scan_notation_decls` 对「声明驱动的符号」「parse 不过的入口」「scoped 记法」的行为 |
| LSP 单测 | `cargo test -p sokonanoda-lsp --locked` | P0 回归（project.rs）；三种符号的 hover 文案与围栏（hover.rs）；`trigger_characters` 若改（navigation.rs） |
| CLI 契约 | `cargo test -p sokonanoda-cli --test extension` | `abbreviation_table_follows_the_single_source`；`unit_test_script_covers_every_node_layer`（新增 Node 文件时） |
| 纯 Node 单测 | `cd editor/vscode && npm run test:unit` | `abbreviations.js` 的前缀/别名查询（**纯函数**，不需要宿主）；重写器的状态机（喂假文档变更 → 断言替换范围/文本） |
| stub 宿主 | `node editor/vscode/test-extension-host.js`（`test:unit` 第 4 个文件） | 真 `extension.js` 里：`\and`+空格/Tab 的替换、`A \ B` 不被误替换、光标离开收尾、命令可用 |
| 真 VS Code | `SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh`（结果进 `docs/e2e/ledger.jsonl`） | 键入 `\in` + Tab → 文本 `∈`；`\and` + 空格 → `∧`；`A \ B` 原样；hover `∈` 出现「输入方式」 |
| 门禁 | `scripts/soko gate` | 以上全部 + 课程门禁 |

**注意**：stub 宿主**不能**验证 VS Code 的补全过滤/`textEdit` 语义（那是真宿主的活）——
所以方案 A 把重写逻辑做成纯函数（可单测），stub 层只验接线。

---

## 7. 课程侧

1. `courses/set-theory/units/notation-cheatsheet.sokonanoda`：**加一栏**「输入」，
   放在现有「点名形式 ↔ 数学记法」对照表旁（`:5-11` 那段注释表），并把
   「怎么打这些符号」写成 3–5 行散文（`\in` + Tab/空格；`\powerset` 不是 `\P`；
   `''` 没有缩写）。**不重写整页**（页头已声明「其它单元继续用点名形式」）。
2. `skills/sokonanoda-teacher/SKILL.md`：在记法要点里加一条
   「教学习者输入：`\in`+Tab/空格；hover 会提示」；**同时修 stale ⑦**
   （`:148-149` 说「第二刀未做：`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 import 的记法」——
   0.60.0 已做，现在 `lib/Set.sokonanoda:164-177` 就是证据）。
3. `editor/vscode/README.md` + `CHANGELOG.md` + `package.json` 版本：同一轮（硬规则）。
4. `REQUIREMENTS.md` §9 追加本次用户要求（带日期）——**实施轮**做，本轮只读。

---

## 8. 分期

| 期 | 内容 | 验收 |
|---|---|---|
| **P0** | 修 LSP 项目报告被 parse_error 丢弃（§1.6） | 课程 12 个 unit + cheatsheet 在编辑器里：0 假诊断、hover/documentSymbol 恢复；LSP 回归测试绿；CLI 判定不变 |
| **P1** | Rust 缩写表 + 容错记法扫描 + **LSP hover「输入方式」** | 三种符号（内建/本文件/import）hover 都给出输入提示；契约测试 `abbreviation_table_follows_the_single_source` 绿；**DSH 的 `lsp` hover 也能看到**（零客户端改动） |
| **P2** | VS Code 缩写重写器（即时替换 + Tab + 命令/键位/配置） | `\and`+空格→`∧`；`\in`+Tab→`∈`；`A \ B` 不动；三层测试绿 |
| **P3** | 课程速查页 + teacher skill + README/CHANGELOG/版本 + `docs/design/notation-input.md` 定稿 | `scripts/soko gate` 绿；e2e 台账新增一条 |

---

## 9. 风险与未决

1. **P0 必须先做**：不做，hover 提示在课程文件里没有可测的落点。
2. **`\` 双重身份**：`\` 既是 leader 又是集合差符号（`lib/Set.sokonanoda:170`）。
   重写器只在「`\` 紧跟 ASCII 字母且整词命中缩写」时替换；`A \ B`、`\ `、行尾 `\` 不动。
3. **前缀陷阱**：`\in` 是 `\inter` 的前缀、`\to` 是 `\to0…` 的前缀。必须实现
   「完整且不是更长缩写的前缀才即时替换」，否则学习者打 `\in` 会先看到 `∩`。
4. **单字母别名不抄**（`\i`/`\v`/`\r`/`\l`）——与 Lean 有意偏离，写进设计文档。
5. **重写器与编辑器编辑竞争**：即时替换会与 LSP 客户端的编辑/undo 交互。
   缓解：P2 先只做 **Tab 显式替换**（零风险），即时替换在状态机有单测覆盖后再开
   （或做成配置 `sokonanoda.input.eager`，默认关）。
6. **`𝒫 ᶜ '' ⁻¹' ×ˢ` 今天不被 TM 语法着色**（`tmLanguage.json:133-136` 只匹配码点类）。
   可选 P3：把课程符号加进 `mathsymbols` 交替式（配一条守护测试）。**不影响输入功能**。
7. **`≠` 未支持**：表里留位但标 `supported: false`，hover 不承诺；真要支持是语言改动
   （`notation-audit.md` §0 第 5 条：加一条 `def Ne` 即可）。
8. **hover 噪音**：未声明符号、`→`、tactic 词继续静默；只有「确实是记法符号」才出提示。
9. **协议面**：不需要新 `soko/*` 请求（P1 全在 hover 里）；若将来要 `query notations`
   再进 `docs/protocol.md`。
10. **单一真相源的执行成本**：新增缩写要同轮改 Rust 表 + JSON 镜像，否则 CI 红——
    这是**有意**的（同 tmLanguage 的纪律）。

---

## 10. 复现命令（本轮实测用过的，可直接粘贴）

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang

# 构建被测 LSP（本机必须带 DEVELOPER_DIR 前缀）
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo build -p sokonanoda-lsp

# CLI 判定（课程文件在闭包路径下是绿的，编辑器里却报 notation-unknown-symbol）
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo run -q -p sokonanoda-cli --bin sokonanoda -- \
  --json courses/set-theory/units/notation-cheatsheet.sokonanoda; echo "exit=$?"

# LSP：真 hover 探测（Python over stdio；把 <file> 换成目标文件）
#   initialize → initialized → textDocument/didOpen(内存文本) → textDocument/hover
#   关键请求：textDocument/hover、textDocument/documentSymbol、soko/goals、soko/project
#   §1.5/§1.6 的全部结论都由这套探针实测得出
```

证据索引（本笔记引用的锚点）：

- 扩展：`editor/vscode/extension.js:23,404-408,1607-1650,1683-1693`；
  `editor/vscode/package.json`（contributes/keybindings/configurationDefaults）；
  `editor/vscode/syntaxes/sokonanoda.tmLanguage.json:20-37,60-75,133-136`；
  `editor/vscode/test-extension-host.js:178-180`；`editor/vscode/src/test/extension.test.js:1-60`
- 契约测试：`crates/cli/tests/extension.rs:11-16,602-627,858-873,975-1006,1007-1026,1051-1078,1079-1116`
- LSP：`crates/lsp/src/lib.rs:78-88,158-166,739-749,941,961-967,1071-1188,1353-1440`；
  `crates/lsp/src/render.rs:298-302,347-356`；
  `crates/lsp/src/tests/hover.rs:270-295`；`crates/lsp/src/tests/hover_brackets.rs:42-74`
- front：`crates/front/src/semantic.rs:42-99,264,343-363,378-385,532-541,741-744,775-785`；
  `crates/front/src/parser.rs:147-164,966-973,2991-3004`；
  `crates/front/src/token.rs:17-25,480-491,600`；
  `crates/front/src/ast.rs:129-163,609-627`；
  `crates/front/src/query/mod.rs:42-69,132-151,229-237,375,452-453`；
  `crates/front/src/project/mod.rs:48-79`；`crates/front/src/project/report.rs:90-105`
- 设计/纪律：`docs/design/goal-rendering.md:170-194`；`docs/vscode-dev-guide.md:21-58,59-76`；
  `docs/design/deepseek-harness.md` D8；`AGENTS.md`（VS Code + skills 同轮同步、模块化）
- 课程：`courses/set-theory/lib/Set.sokonanoda:164-177`；`courses/set-theory/lib/Exists.sokonanoda:100`；
  `courses/set-theory/units/notation-cheatsheet.sokonanoda:79-95`；
  `skills/sokonanoda-teacher/SKILL.md:131-149`
- 同系列调研：`docs/notes/course-lean-style/notation-audit.md`（记法能力审计，0.61.0）
