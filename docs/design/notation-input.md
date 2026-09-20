# 设计：记法输入法（`\xxx` 缩写）+ hover 提示

> 日期：2026-09-19。触发（用户原话）：「1. 要考虑 notation 如何输入，应该像 lean4 一样
> `\xxx` 替换，同时 hover 内容提示用户如何输入对应符号」。
>
> 调研底稿：`docs/notes/course-lean-style/notation-input-plan.md`（含 Lean 4 的逐字
> 取证与全部实测复现命令）。本文是**设计 + 分期计划**；实施时把 as-built 追加到 §8。

---

## 0. 一句话

**做三件事**：① 先修一个**阻断级 LSP 缺陷**（否则 hover 在最需要它的文件里是死的）；
② 在 VS Code 扩展里做 **Lean 同款「缩写重写器」**（`\and` → `∧`，`\` 是 leader、Tab 强制转换、
整词即时替换）；③ 在 **LSP 的 hover** 里告诉学习者「这个符号怎么打出来」。

**不做**：给缩写**再加一个 LSP 补全 provider**（注意口径——LSP **已经有** completion，见 §1；
这里说的是**不为 `\` 缩写新增一条**。理由：Lean 自己就不是这么做的，且 DSH/opencode 都不
消费补全项，而 DSH 的 `lsp` 工具有 hover ⇒ 同一条信息放 hover 里对 agent 也生效）。

---

## 1. 现状（证据见底稿 §1）

| 面 | 事实 |
|---|---|
| VS Code 扩展 | `editor/vscode/extension.js`（**1880 行**）已有 hover / 练习树 / Infoview / 命令；**没有**补全 provider、没有缩写机制 |
| LSP | `crates/lsp/src/lib.rs` 提供 hover / documentSymbol / codeAction / inlayHint **以及 `textDocument/completion`**（`lib.rs:1353-1440`：本域 binder + 关键字 + 宇宙 + prelude 名，且服务端**已经**advertise `completionProvider`，`lib.rs:961`）；hover 文本一律 ` ```sokonanoda ` 围栏（`docs/design/goal-rendering.md` §7 的契约） |
| 单一真相源纪律 | `front::semantic::KEYWORDS` 与 `editor/vscode/syntaxes/sokonanoda.tmLanguage.json` **同轮同步**，守护测试 `crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source`。新表必须接进同一条纪律 |
| hover 对**符号**的反应 | **三种行为不一致**（底稿 §1.5 实测）：内建记法（`∧`）有反应、库记法（`∈`）在**单文件 parse 失败时**没反应、`→`（词法别名）反应的是别的 |

### 1.1 【NI-0 阻断 · **已修**】单文件 parse 失败会吃掉项目报告（台账 **G-20**）

`crates/lsp/src/lib.rs:158-166`：`set_text_with_overlay(...)` **已经**装好了项目报告
（`query/mod.rs:135-151`），紧接着

```rust
if self.doc.parse_error.is_some() {
    self.doc.report = None;   // ← 把好报告丢了
    return;
}
```

而 `Doc::diagnostics`（`lib.rs:78-88`）**parse 错误优先于报告** ⇒ 把那条单文件 parse
错误当 ERROR 发出去。

**后果**（底稿用真 LSP over stdio 实测，0.61.0）：
`courses/set-theory/units/` 下**任何**用了「库声明记法」的单元（`∈ ⊆ ∪ ∅ …` 来自
`import lib.Set`）——单文件 parse 必然失败（记法不在本文件里）——
⇒ 编辑器里发**假的** `notation-unknown-symbol` 红波浪线，且 `documentSymbol`/`hover`/
`codeAction`/`inlayHint` 全部回答 `null`。而同一份文本走 CLI 判卷是 **exit 0**。

⇒ **用户要的「hover 提示怎么输入」正好死在这批文件里**。⇒ **NI-0 必须先修**（**已落**，见 §10）。

---

## 2. 缩写表（19 个符号；Lean 逐字取证）

**原则**：主缩写**逐字照抄 Lean**（硬规则 3：教学语法是真实 Lean 4 的子集 ⇒ 肌肉记忆可
迁移）；**单字母别名（`\i` `\v` `\r` `\l` `\a`）一律不抄**（前缀陷阱，对学习者只有害处）。
每条别名都进表（hover 列出全部）。

> **⚠️ 这张表是外部事实，实施时必须重新取证一次。** 底稿 `notation-input-plan.md`
> §2.1/§2.2 已按上游**逐条带行号**核对过一轮（`abbreviations.json` 的 `L328`/`L356`/…
> 就是证据），但**上游会漂移**，且本仓库**不 vendor、不下载** Lean 的任何东西
> （硬规则 2：不调用官方 Lean 工具链 ⇒ 表是**手抄**的，不是构建期拉取的）。
> ⇒ **P1 的第一件事**：拉一次
> [`lean4-unicode-input/src/abbreviations.json`](https://github.com/leanprover/vscode-lean4/blob/master/lean4-unicode-input/src/abbreviations.json)
> ＋ `AbbreviationRewriter.ts`（逻辑），与底稿 §2.2 的「行号表」**逐行 diff**，
> 把差异与取证日期写进 §8 as-built。**不允许"抄过一次就永久为真"**——
> 表里 `supported: true` 的每一项另有**本地**判据（§6.1：用真判卷跑一条最小声明），
> 那条才是仓库内可复验的部分。
>
> 本次评审时上游源码**取不到**（网络受限，`raw.githubusercontent.com` 连接失败），
> 所以**此处不声称"已再次核对"**：底稿那份带行号的取证仍是第一手证据，本次未复现。
> 这条限制本身正说明上面那句"P1 第一件事"是必需的。

| 符号 | 主缩写 | 别名 | 今天支持 |
|---|---|---|---|
| `∧` | `\and` | `\wedge` | ✅ 内建记法 |
| `∨` | `\or` | `\vee` | ✅ 内建 |
| `↔` | `\iff` | `\leftrightarrow` | ✅ 内建 |
| `¬` | `\not` | `\neg` | ✅ 内建 |
| `→` | `\to` | `\imp` | ✅ 词法别名（不是记法） |
| `∀` | `\forall` | — | ✅ 关键字 |
| `∃` | `\exists` | — | ✅ `lib/Exists` 的 `binder_notation` |
| `≠` | `\ne` | `\neq` | ✅ **0.61.0 起有**（L2.3：`Ne` 进 L1 prelude 的 B9 族 + 内建记法）——表里已翻 `supported: true` |
| `∈` | `\in` | `\mem` | ✅ `lib/Set` |
| `⊆` | `\sub` | `\subseteq` | ✅ `lib/Set` |
| `∪` | `\cup` | `\union` | ✅ `lib/Set` |
| `∩` | `\cap` | `\inter` | ✅ `lib/Set` |
| `\` | `\setminus` | — | ✅ `lib/Set`（**与 leader 同字符** ⇒ 只在「`\`+字母且整词命中」时替换） |
| `∅` | `\empty` | `\emptyset` | ✅ `lib/Set` |
| `𝒫` | `\powerset` | — | ✅ `lib/Set`（**不能用 `\P`**：Lean 里是 Π） |
| `ᶜ` | `\compl` | `\complement` | ✅ `lib/Set` |
| `''` | **不给** | — | ✅ 与 Lean 一致（直接打两个单引号）；hover 明说 |
| `⁻¹'` | `\preim` | `\preimage` | ✅ `lib/Set` |
| `×ˢ` | `\xs` | — | ✅ `lib/Set` |

### 2.1 ⚠️ 待拍板：Lean 的**多字母别名**要不要一起收

上表是底稿 §2.2「我们采用的表」，它**只收了主缩写 + 1 条常见别名**。Lean 实际上还有
一批**多字母**别名被我们漏掉了（它们**不是**前缀陷阱——规则本来就是「缩写完整且
不是更长缩写的前缀」才替换，所以 `\an` 会等到打完 `\and` 或敲非字母才落定）：

| 符号 | Lean 还有 | 收不收 |
|---|---|---|
| `∧` | `\an` | ？ |
| `↔` | `\lr` | ？ |
| `∀` | `\all` | ？ |
| `∃` | `\ex` | ？ |
| `¬` | `\lnot` | ？ |
| `∈` | `\member` | ？ |
| `⊆` | `\subset` `\ss` | ？ |
| `∪` | `\un` | ？ |
| `∅` | `\varnothing` | ？ |

**两个选项**：**(a) 全收**——最贴合「像 Lean 4 一样」（硬规则 3 的肌肉记忆迁移），
代价是表变大、hover 更长、每个别名都要有测试；**(b) 保持现状**（只主缩写 + 1 别名）——
表小、教学叙事干净，代价是学过 Lean 的人打 `\subset` **没反应**，而**这不是报错**，
更难自查（用户会以为记法坏了）。
**倾向**：收 `\subset`（`⊆` 是卷 I 最高频符号之一，且 `\sub` 只是它的前缀）与
`\member`，其余按 (b)。**这条要用户拍板**——它只影响输入手感，改动是一行一条、
**随时可加，不阻塞任何其它工作**。
**无论如何都不收**（它们是**别的符号**，收了才真错）：`\P`→Π、`\times`→×（普通乘号）、
`\ssubset`→⊂（真子集）、`\complementprefix`→∁。

---

## 3. 输入方案对比

| | **A 客户端缩写重写器（推荐）** | B VS Code 补全项 | C 命令 + 键位 | D LSP 补全 |
|---|---|---|---|---|
| 形态 | `onDidChangeTextDocument` 驱动：`\`+字母**整词命中**即时替换；Tab 命令强制替换 | `CompletionItemProvider(["\\"])` | `sokonanoda.insertSymbol` / `convertAbbreviation` + 键位 | `textDocument/completion` |
| 与 Lean 一致 | ✅ 逐条一致（Lean 就是客户端 `AbbreviationRewriter`，不是补全 provider） | ❌ Lean 不这么做 | ⚠️ 只覆盖 Tab 那一半 | ❌ |
| 估行 | 表 40 + 重写器 160–200 + 接线 30 + 测试 180 | 80–120 + 测试 60 | 60 + 测试 40 | 80 Rust + 60 |
| 前缀陷阱 | **必须处理**：`\in` 是 `\inter` 的前缀 ⇒ 只有「缩写**完整**且**不是更长缩写的前缀**」才即时替换，否则打 `\in` 会先跳出 `∩` | 同上 | — | — |
| 其他 harness 能用 | ❌ | ❌ | ❌ | ⚠️ **DSH 不消费补全**（`docs/design/deepseek-harness.md` D8：只有 definition/references/implementation/hover）；opencode 也没接 ⇒ 唯一消费者仍是 VS Code，而 VS Code 有 A 这条更好的路。**注意**：LSP 的 `textDocument/completion` **已经存在**（`lib.rs:1353`），方案 D 指的是「为 `\` 缩写**再加一条** provider」 |
| 主要风险 | 即时替换的状态机（undo / 多光标 / 与 LSP 编辑竞争） | `\` 不是 word char ⇒ 替换范围要显式 `textEdit`；与集合差 `\` 冲突；**不是 Lean 行为** | 单独用不够 | 与 A 同时上会双份候选 |

**⇒ 推荐 A（+ C 作为 A 的 Tab 那一半），D 不做**（将来若真有 harness 需要补全列表，
再作为 `Ctrl+Space` 增强单独立项）。

---

## 4. hover 提示（落点：LSP）

- **落点唯一**：`crates/lsp/src/lib.rs` 的 hover 组装处（与 `expr_hover` 同一层），
  数据来自新的 `front::notation_input`（表）——**不在客户端拼**。**顺带白赚**：DSH 的
  `lsp` 工具正好有 hover（`docs/design/deepseek-harness.md` D8），所以这条提示对
  **agent 也生效**（补全对 DSH 无效，这是不做方案 D 的又一个理由）。
- **⚠️ 必须插在「关键字闸门」之前**：`lib.rs:1100-1104` 有一道「关键字 → 返回 None」的
  闸门，而 `semantic.rs:537-541` 把**已声明的记法符号**归进 `Keyword` ⇒ 今天
  **本文件声明的符号（如 `⊗`）hover 完全静默**；内建的 `∧` 走另一条路（显示外层表达式）；
  import 来的因 §1.1 无报告。三种行为不一致（底稿 §1.5 实测）⇒ 新分支放在闸门**之前**，
  让三种形态统一到「符号 + 怎么输入 + 展开成什么」。
- **触发**：光标落在符号 token 上。
- **文案示例**（学习者视角，中文）：

  > `∧` —— 逻辑「且」，语言**内建**记法（不需要声明）。
  > 输入：`\and`（别名 `\wedge`）
  > 展开：`And A B`

- **契约**：① 表里 `supported: false` 的符号**不承诺**可输入，改说「语言今天还没有
  这个符号」（**今天一条都没有**——`≠` 随 L2.3 翻 true；表里留 `supported` 字段是为了
  将来加符号时有地方写"还没有"）；② 输入提示与「展开成什么」**同一条**（两者都来自
  `front::notation_input` 与 `token::scan_notation_decls`，不许两处各写一份）。
  （原第 ① 条「代码一律围栏」**已作废**：这条 hover 是「符号 + 展开 + 怎么输入」的
  散文 + 行内 code span，没有多行代码块；见 §10 的 as-built。）
- **⚠️ 表是静态的，作用域是动态的**（2026-09-19 评审发现的真缺口）：表里
  `∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹' ×ˢ` 标着「✅ 今天支持」，但它们**只在 `import lib.Set` 的
  文件里**可用（记法 = 文件作用域 + 跨 `import` 传播，`notation-subset.md` N5）。
  一份没 import 课程库的文件里 hover `∈`，若只说「输入 `\in`」，就是在教一件
  **这个文件里做不到**的事。⇒ 两条对策：
  1. hover 的**作用域行**必须来自**本文件**的词法扫描（`declared_notation_at`）——
     本文件声明过才说「（本文件声明）」，import 来的不说；`symbol_at` 只负责
     「这是个记法符号」；
  2. **R-3 只挡 Rust↔JS 漂移，挡不住"表 ↔ 语言"漂移**（`supported` 是手写的
     `bool`）⇒ §6.1 的本地判据（每个 `supported: true` 的符号跑一条真判卷）才是
     那条防线。`Ne` 落地后要把 `≠` 翻 true，反过来若某符号被拿掉，测试会红。

---

## 5. 落点清单 + 估行

| 文件 | 改什么 | 估行 |
|---|---|---|
| `crates/front/src/notation_input.rs`（**新建**） | 缩写表（符号 ↔ 主缩写 ↔ 别名 ↔ supported）+ 查表 API + 单测 | 120–160 |
| `crates/front/src/lib.rs` | `pub mod notation_input;` | 2 |
| `crates/lsp/src/render.rs`（或 `lib.rs` hover 处） | 符号 hover 时附「怎么输入」 | 40–60 |
| `crates/lsp/src/lib.rs:158-166` + `:78-88` | **NI-0 修复**（**已落**）：单文件 parse 失败**不再**丢掉项目报告；`diagnostics` 改为「闭包编译成功就用报告」 | 20–40 |
| `crates/front/src/query/mod.rs` | **NI-0**：`QueryDoc::project_entry_compiled()`（入口模块 `Compiled` 才算救回来了） | 10–20 |
| `editor/vscode/src/abbreviations.js`（新建） | 表（从 Rust 常量生成或双向守护） | 40 |
| `editor/vscode/src/abbreviation-rewriter.js`（新建） | 状态机：整词命中即时替换 + Tab 强制 + undo/多光标 | 160–200 |
| `editor/vscode/extension.js` + `package.json` | 接线（命令/键位/配置项 `sokonanoda.input.leader`） | 40 |
| `crates/cli/tests/extension.rs` | 守护：VS Code 侧表与 Rust 表**逐字一致**（同 `tm_grammar_keywords_follow_the_single_source` 的形制） | 40 |

---

## 6. 测试（三层 + 契约层）

1. **Rust 单测**（`front::notation_input`）：表自洽（无重复符号/缩写、主缩写在前）、
   每个 `supported: true` 的符号**今天真的能用**（用真判卷跑一条最小声明）。
2. **Rust 契约**（`crates/cli/tests/extension.rs`）：VS Code 的 `abbreviations.js`
   与 `front::notation_input` 的表**逐字相等**（挡住漂移）。
3. **stub 宿主**（`editor/vscode/test-extension-host.js`）：重写器行为——`\and` + 空格 →
   `∧`；`\an`（前缀）**不替换**；`\`（集合差）**不替换**；多光标；undo 一次撤销整次替换。
4. **真 VS Code**（`scripts/vscode-e2e.sh`）：一条用例——输入 `\in` → 出现 `∈`；
   hover `∈` → 文案含 `\in`；结果记 `docs/e2e/ledger.jsonl`。
5. **LSP 回归**（NI-0 修复，**已落**）：一个 `import lib.Set` + 用库记法的文件——断言
   **无诊断**、`documentSymbol` 非空、hover 非 null（今天三条全挂，见 §1.1）。

---

## 7. 课程侧

- `units/notation-cheatsheet.sokonanoda` 加一栏「怎么输入」（表来自同一份真相）；
- `skills/sokonanoda-teacher` 的 `references/zh-style.md` 加一句：教学习者用缩写输入符号；
- 入门课/playground 的叙事提一句「符号可以打 `\and` 出来」。

---

## 8. 分期

**编号口径**（2026-09-19 评审修正）：本文的期号一律带 `NI-` 前缀（notation input）。
主计划 `course-lean-style.md` §5 的 **R2.5** 片用的是它自己的 `P0/P1/P2/P3`，两份文档
从前**都叫 P0/P1/P2** 而含义不同（"P1"曾同时指"隐式实参"和"表 + hover"）——现在分开：
**R2.5-P0 = NI-0 + IA-0**、**R2.5-P2 = NI-1 + NI-2**、**R2.5-P1 = IA-1**。

| 期 | 内容 | 验收 |
|---|---|---|
| **NI-0** | **修 LSP 阻断缺陷**（§1.1）+ 回归测试 | `cargo test -p sokonanoda-lsp` 绿；真 LSP 探针在 `courses/set-theory/units/` 下：0 诊断 + documentSymbol 非空 + hover 非 null |
| **NI-1** | `front::notation_input` 表 + hover 提示 + 契约测试 | `cargo test --workspace --locked` exit 0；hover 探针文案含缩写 |
| **NI-2** | VS Code 缩写重写器（A + C 的 Tab 一半）+ 三层测试。**先只做 Tab 显式替换**；「即时替换」做成配置项 `sokonanoda.input.eager`，**默认关**（undo/多光标/与 LSP 编辑竞争的风险留到有 e2e 证据再翻默认） | `node editor/vscode/test-extension-host.js` 绿；`scripts/vscode-e2e.sh` 新用例绿 |
| **NI-3（可选）** | 为 `\` 缩写加一条 LSP 补全 provider（方案 D）作 `Ctrl+Space` 增强 | 只在真有 harness 需求时立项 |

---

## 9. 风险

| # | 风险 | 缓解 |
|---|---|---|
| R-1 | 即时替换的**状态机**在 undo / 多光标 / 与 LSP 的 `WorkspaceEdit` 竞争时出错 | 状态机只改「刚输入的那个词」；Tab 路径是显式命令（可单独测）；stub 宿主 + 真 e2e 两层覆盖 |
| R-2 | `\` 既是 leader 又是**集合差**符号 | 只在「`\` + 字母且**整词命中表**」时替换；`\setminus` 命中表、单独 `\` 不命中 ⇒ 不误伤 |
| R-3 | 表漂移（Rust 侧与 JS 侧两份） | 契约测试（§6.2）逐字相等；JS 侧表由脚本从 Rust 常量生成亦可 |
| R-4 | `≠` 在表里但语言没有 | **已消**（L2.3 落地，`supported` 翻 true）；字段留着给将来的符号 |
| R-5 | NI-0 的修法改动 LSP 既有契约（parse 失败 ⇒ 无报告） | 只放宽「**闭包编译成功**时」这一档；闭包也失败时仍按老契约发 parse 错误。加回归测试钉住两侧 |
| R-6 | `𝒫 ᶜ '' ⁻¹' ×ˢ` **不在数学码点类**里 ⇒ TM 语法不着色（`syntaxes/sokonanoda.tmLanguage.json` 的 `mathsymbols` 只匹配 `U+2200–22FF`/`U+2A00–2AFF`/`\`） | **已修（0.61.0）**：类改成**显式枚举**，由 `front::notation_input::notation_symbol_chars`（单一真相源）生成，守护测试 `crates/cli/tests/extension.rs::tm_grammar_math_symbols_follow_the_single_source` 钉住逐字相等；`=` 归 `operators` 规则（ASCII）。~~可选 P3~~ 已随 NI-1 落地 |
| R-7 | 顺手要修的过期文档：`skills/sokonanoda-teacher/SKILL.md:148-149` 说「第二刀未做」，而 0.60.0 已完成（证据 `courses/set-theory/lib/Set.sokonanoda:164-177`） | 同轮改（文档纪律：技能是操作手册，不许留假话） |

---

## 10. as-built（NI-0 / NI-1 已落，2026-09-19）

> 与设计的偏差、以及评审纠正过的地方，全部记在这里。**内核零改动**。

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **NI-0** | `QueryDoc` 新增 `project_entry_compiled()`（入口模块状态 `== Compiled`）；LSP 的 `Doc::set_text` 只在它为 `false` 时维持老契约，`Doc::diagnostics` 在闭包救回来时以闭包报告为准 | `crates/front/src/query/mod.rs`、`crates/lsp/src/lib.rs` | 台账 **G-20**（`docs/gaps/WO-013`）；复现件 `docs/gaps/repro/G20-lsp-drops-rescued-report.sh`：修前 exit 0（假诊断 + hover/documentSymbol 全 `null`）、修后 exit 1 |
| **NI-0 顺带** | 测试基建 bug：`testutil::lsp_pos` 按**字节差**算 LSP `character`，含多字节符号的行会偏（想 hover `⊗` 却 hover 到 `b`） | `crates/lsp/src/testutil.rs` | 改为按字符数计算；ASCII 夹具上两种算法恒等，所以这个 bug 只在非 ASCII 夹具里显形 |
| **NI-1 表** | `front::notation_input`：18 条（19 个符号，`''` 与 Lean 一致地**没有**缩写）+ `input_for` / `symbol_for_abbreviation` / `input_hint` / `symbol_at` / `declared_notation_at` | `crates/front/src/notation_input.rs`（新）、`crates/front/src/lib.rs` | 7 条单测（表自洽、前缀陷阱仍在、`supported:false` 不承诺、缩写往返、本文件声明 vs import 来） |
| **NI-1 词法** | `token::scan_notation_symbols` 重构为 `scan_notation_decls`（符号 **+ 展开目标**）的投影：hover 要告诉学习者「展开成什么」，而使用库记法的文件**单文件 parse 必然失败**，拿不到 parse 结果 | `crates/front/src/token.rs` | `scan_notation_target` 只认 `=>` 之后那个点分标识符；半成品返回 `None` 不移动游标 |
| **NI-1 hover** | 记法符号的 hover：符号 + 是否本文件声明 + 展开成什么 + **怎么输入** + 外层表达式的类型（有就给）。**插在关键字闸门之前** | `crates/lsp/src/lib.rs`（`notation_symbol_hover`） | 修前实测：本文件声明的 `⊗` hover **完全静默**（`semantic` 把已声明记法归进 `Keyword`，闸门吞掉）；现在 `⊗` → 展开 `myop`，内建 `∧` → `\\and`，import 来的 `∈` → `\\in`（三条都有测试） |

**与设计的偏差**：

1. **hover 不用 ` ```sokonanoda ` 围栏**（§4 原契约 ① 已作废）。这条 hover 是「符号 +
   展开 + 怎么输入 + 类型」的散文式回答，行内 code span 更贴 Lean 的
   `AbbreviationHoverProvider` 观感；类型行也自带反引号。多行代码块的围栏契约
   （`goal-rendering.md` §7）继续管**目标状态**类 hover，两者不冲突。
2. **作用域行是词法的、不是语义的**：`declared_notation_at` 只做词法扫描
   （`scan_notation_decls` + `tokenize_with_symbols`），因此「本文件声明」这句话在
   **声明点之前**也会说（预扫描看得见整份源码，与 parser 的
   `notation-unknown-symbol` 判据同款）。这是有意的一致，不是 bug。
3. **`scoped` 记法不特殊处理**：`open scoped Foo` 才生效的记法在词法层看不出，
   hover 会照常说「展开成 …」。记为已知边界（课程里没有 `scoped` 用法）。
4. **多字母别名仍未收**（§2.1 的待拍板项）：表里只有主缩写 + 1 条别名。

**验证（本轮实测）**：`cargo test --workspace --locked` **1180 passed / 0 failed**；
`cargo fmt … --check` exit 0；`cargo clippy -p front -p cli -p lsp --all-targets`
零 warning（仅冻结内核 62 条既有）；课程门禁 **36 目标 · 329 checked · 99 open ·
0 判负**、`--selftest` exit 0；`python3 scripts/gap.py check` + `selftest` exit 0；
`git diff --stat -- crates/kernel/` **空**。

---

## 11. as-built（NI-2 已落，2026-09-20）

| 项 | 实际改了什么 | 落点 | 证据 |
|---|---|---|---|
| **NI-2 表** | 18 条镜像表（symbol / abbreviation / aliases / supported / **顺序**），写成"一条一行、键带引号"的**纯 JSON 数组字面量** | `editor/vscode/src/abbreviations.js`（新） | 契约测试 `crates/cli/tests/extension.rs::abbreviation_table_mirrors_the_single_source`——`serde_json` 真解析后逐条比对（不是 grep 掩膜）；改一个字母即红（本轮实测） |
| **NI-2 状态机** | 词扫描（`\` + ASCII 字母整词、**大小写敏感**）→ 替换；Tab 是**强制**替换（前缀不挡：`\in`+Tab → `∈`）；eager 两种口径——还在敲字母时只认「非前缀的完整词」（`\an` 等 `\and`、`\in` 等 `\inter`），敲了分隔符则词已封口（`\in ` → `∈ `）；**一次 `editor.edit` 装下所有光标 = 一个 undo 单元**（显式 `undoStopBefore/After`）；非空选区不碰；孤立 `\` 永不命中；只理 `.sokonanoda` | `editor/vscode/src/abbreviation-rewriter.js`（新） | stub 宿主新增 15 例（`editor/vscode/test-extension-host.js`，含"一次 edit 调用 / 一条 undo 记录"的断言）；变异测试确认前缀陷阱、分隔符、undo 三条用例真的会红 |
| **NI-2 接线** | 命令 `sokonanoda.input.replaceAbbreviation`（**注册在 `extension.js`**，状态机在模块里）、键位 `tab` + `when: editorTextFocus && editorLangId == sokonanoda && sokonanoda.input.abbreviationBeforeCursor`（context key 由扩展在选区/文本变化时置位）、配置 `sokonanoda.input.eager`（默认 **false**）；扩展/Rust 版本 0.61.0 → **0.62.0** | `editor/vscode/extension.js`、`package.json`、`CHANGELOG.md`、`README.md`、`skills/**`、`docs/vscode-dev-guide.md` | 契约测试 `notation_input_tab_binding_is_gated_by_its_context_key`；真宿主 e2e 一例（打字 + hover 文案） |

**与设计的偏差**（逐条对应上面的落点）：

1. **没有 `sokonanoda.input.leader` 配置**（§5 的落点清单提过它）：leader 固定 `\`。
   表里的缩写不带 leader，LSP 侧 hover 文案也硬写 `\and`——让客户端可改 leader 会让
   hover 说的和编辑器做的不是一回事。真要支持就得两边同轮改，不是加个配置项。
2. **Tab 的 `when` 子句用自建 context key**（`sokonanoda.input.abbreviationBeforeCursor`），
   不是纯静态条件：VS Code 的 `when` 没有"光标前的文本"这种谓词。代价：`\an`（前缀，
   还没打完）上按 Tab 会被命令吃掉且**什么都不做**——这是有意的（比往源码里插一个制表符
   好）；普通代码/缩进完全不受影响（context key 为 false，Tab 照旧）。
3. **eager 的位置是 `change.range.end` + 插入文本长度**：VS Code 的
   `TextDocumentContentChangeEvent.range` 是"**被替换掉的范围**"（**旧文档坐标**），纯插入
   时它在插入文本**之前**——按"新坐标"直觉写会让 eager 在真宿主里整体错位。
   取证（不是猜测）：VS Code 1.138.0 自带源码里 `TextModel._doApplyEdits` 产出的 change 是
   `{range: 旧范围, text: 新文本}`，经 `ApplyEditsResult(reverseEdits, changes, …)` 的
   **第二个**字段上报扩展宿主。stub 宿主按同一口径造事件（`typeText`），否则测试全绿而
   真编辑器是错的。
4. **`.vscodeignore` 从 `src/**` 改成 `src/test/**`**：设计把两个模块放在 `src/`，而原来的
   整目录排除会把它们挡在 VSIX 之外（装上即 `Cannot find module './src/abbreviation-rewriter'`），
   且 stub / 契约 / e2e **三层都发现不了**（跑的是仓库文件，不是包里的）。已记进
   `docs/vscode-dev-guide.md` §5 第 22 条。
5. **e2e 用例的 hover 半边用内建 `∧`**（§6.4 原文举例 `∈`）：`∈` 只在 `import lib.Set`
   的文件里可用，而 e2e 的临时夹具工作区没有课程库；`∧` 是内建记法，走**同一条**客户端
   接线与 LSP hover 通道。打字半边仍是真命令 + `\and` → `∧`。
   **e2e 不断言 undo**：workbench 的 `undo` 命令在 vscode-test 宿主里是 no-op（实测：
   命令返回后 800 ms 文本仍是 `∧`，先 `focusActiveEditorGroup` 也一样——它要 UI 键盘
   焦点，测试宿主给不了）。「一次 edit = 一个 undo 单元」钉在 stub 宿主层，代码里另把
   `editor.edit` 的 `{undoStopBefore, undoStopAfter}` 显式写成 `true`（VS Code 1.138.0
   自带源码里就是默认值），让这条契约不依赖默认值。
6. **多字母别名仍未收**（§2.1 的待拍板项，NI-1 的偏差 4 延续）：表里只有主缩写 + 1 条别名；
   `\subset`/`\member` 这些 Lean 也认的写法今天**没反应**，而这**不是报错**——要收就得
   改 Rust 表 + JS 镜像 + hover 文案（一行一条，随时可加，不阻塞任何其它工作）。
7. **eager 的"完整"有两种口径**（比 §3 的字面规则多一条）：还在敲字母时按 §3 的前缀规则
   （`\in` 不落定，免得 `\inter` 变成 `∩ter`）；敲了**分隔符**（空格/标点）时这个词已经
   **封口**，前缀不再是理由 ⇒ `\in ` → `∈ `、`\sub ` → `⊆ `（与 Lean 的
   `AbbreviationRewriter` 同规则，也正是 §6.3「`\and` + 空格」那条用例的落点）。没有这条，
   主缩写**本身就是前缀**的符号（`\in`/`\sub`/`\to`）在 eager 模式下永远打不出来，只能按
   Tab——而"缩写打完了"的直觉恰恰就是"后面跟了个空格"。

**验证（本轮实测）**：stub 宿主 **28/28 passed**（13 既有 + 15 新）；
`cargo test -p sokonanoda-cli --test extension --locked` **37 passed / 0 failed**（35 既有 + 2 新）；
`SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh` 结果见 `docs/e2e/ledger.jsonl`
本轮条目（台账由脚本自己追加）。
