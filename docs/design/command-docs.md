# 命令文档进 VSIX + 命令关键字 F12 跳文档（设计先行，2026-10-11）

**状态**：**已落地**（2026-10-11，用户要求：「把命令文档加入 vsix，`set_option` `#print`
`#check` 等等 F12 跳转到对应文档里」）。与 `docs/design/tactic-docs.md`（tactic 侧）
**同形**：那一份是四层机制 + P1–P7 的原文，本份只写**命令侧的差异与决策**，不重复。

**起因（两条，同日）**：① 用户实测「`#print Set.singleton` 里 `Set.singleton` 不支持
F12 跳转」⇒ 已修（台账 **G-116**：`walk.rs::print` 从不 push `CmdHover`）；② 用户要求
命令文档**随插件分发**且命令字**能 F12 跳到那篇**。①是"参数名跳声明"，②是"命令字跳
文档"——两条相反方向、**必须同时成立**（本设计的第一条硬约束）。

## §1 现状与缺口（每条带 `文件:行`）

- 命令文档正文**已在**：`reference/commands/{README,check,reduce,print,set_option}.md`
  （2026-10-11 落地，判据 `crates/front/tests/command_docs.rs`）。
- 但**只有源**：打包链路 `editor/vscode/scripts/stage-docs.js` 只镜像 `reference/tactics/`
  ⇒ VSIX 里没有命令文档；扩展只把 `SOKONANODA_DOCS_DIR`（= `<ext>/docs/tactics`）交给
  LSP（`editor/vscode/extension.js`）⇒ 命令侧**没有第二档**。
- LSP 的 `goto_definition` 只对 `tactic_doc_at` 走文档分支（`crates/lsp/src/lib.rs`）
  ⇒ 命令字上 F12 落到既有链（关键字不是名字/记法 ⇒ `null`）。
- **命令字的可识别性**：`report.checks[].span` 是**表达式** span（不含 `#check`）·
  `report.prints[].span` 只覆盖 `#print` · `#reduce` 的 `CheckedEvent::Reduced` 落进
  `session.rs` 的 `_ => {}` · `set_option` 的 span 在 `walk.rs` 被 `..` 丢掉 ⇒ 报告里
  **没有**"命令字在哪"这条通道 ✗。

## §2 决策

| # | 决策 | 理由 |
|---|---|---|
| **D1** | 新增真相表 `crates/front/src/commands.rs`：`CommandDoc { name, stem, summary, markdown }` + `COMMAND_DOCS`（4 条），`markdown` 走 `include_str!` | 与 `tactics.rs` 同款：表里有行、文件不在 ⇒ **编译失败**；`summary` 同时是索引行与正文第 3 行（逐字判据） |
| **D2** | `parser::is_reserved_command` 的**命令那半改为查表**（`commands::is_command`） | "加了命令忘了补文档"在构造上不可能（同 `is_tactic_keyword` → `is_tactic`）；**反向**守卫留在测试里：直接读 `is_reserved_command` 的 `#` 字面量 ⇒ 绕开表加字面量照样判红 |
| **D3** | 五档落点机械**抽一份**：`crates/front/src/reference_docs.rs`，`tactics::doc_path` 与 `commands::doc_path` 都转发 | 两页的落点规则逐条相同 ⇒ 各写一份必然在"加一档/改子目录"时静默分叉 ✗ |
| **D4** | 命令字走**词法通道**认：`front::semantic::semantic_tokens` 的 `SemanticKind::Keyword` run（精确 token span），**不动** `DocumentReport` | 零结构体改动、解析失败也退化得出、**不碰判定**；真做进报告要动 `DocumentReport`（它进缓存回放 + `drop_replayed_duplicates` 那道闸）= 方案 B，**本刀有意不做** |
| **D5** | ② 档用**新键** `SOKONANODA_COMMAND_DOCS_DIR`（不是把 `SOKONANODA_DOCS_DIR` 改成父目录） | 改父目录会让既有 wire 判据（插件目录那一条）与 e2e P7 的路径断言**同时失效** ✗ ⇒ 只加一个新键，零破坏 ✓ |
| **D6** | `range` = **落点文档**里 `# <命令>` 标题那个名字 token（复用 `doc_heading_token` + `range_of_offsets`） | D5 的教训（`Location.range` 是**目标文件**坐标）：搬源坐标会把光标停在 `.md` 的空白行 ✗ |
| **D7** | **hover 一个字不动** | `command_line_hover` 是 `hover` 链上**第一个**（C3 契约：`#check`/`#print` 那一行显示命令自己的输出，5 条判据盯着）⇒ 命令字 hover 不抢它 |

## §3 as-built（落点清单）

**front**：`commands.rs`（表 + `doc`/`is_command`/`names`/`doc_path`）·
`reference_docs.rs`（五档）· `tactics.rs`（转发 + `PLUGIN_DOCS_DIR_ENV`）·
`lib.rs`（`pub mod commands` / `mod reference_docs`）· `parser.rs`（查表）。

**LSP**：`lib.rs` 的 `command_keyword_at`（`semantic_tokens` → 表）+
`goto_definition` 里 tactic 分支**之后**的一段（`doc_path` → `Url` → `doc_heading_token`
→ `Location`）。

**打包/扩展**：`stage-docs.js` 泛化成 `PAIRS`（tactics + commands，`--check` 逐目录报）·
`extension.js` 写第二个 env 键 · `release.yml` 的 `COMMAND_DOCS` + `check_docs` 对
**9 个产物**逐字节查 · e2e 的 `docs/commands` 计数（5）与 P8 用例。

**契约**：`docs/protocol.md` §The command keyword definition。

## §4 判据（三层各司其职，缺一层就是洞）

| 层 | 判据 |
|---|---|
| 真相（front） | `crates/front/tests/command_docs.rs` **9 条**：表↔文件双向 · parser 字面量反向 · 行为判据（最小片段真能解析 + 表外 `#eval` 必须失败）· `KEYWORDS` 覆盖 · 内嵌 == 磁盘 · 五节/预算 · 索引逐字 · 例子逐块喂真内核 |
| wire（LSP，真 RPC） | `navigation.rs`：`goto_definition_on_every_command_keyword_lands_in_its_reference_file`（4 条 × 头/尾字符 × 五件断言）· `…_does_not_steal_command_arguments`（`#check id`/`#print id` 仍落**声明**；`pp.explicit`/`true` 仍 `null`）· `…_on_an_unknown_hash_command_has_no_doc_target`（`#eval` ⇒ null）· `…_uses_the_plugin_doc_dir`（② 档 + 反向"删掉插件那份继续物化，不是死指针"） |
| 渲染（真 VS Code） | `extension.test.js` 的 **P8**：插件目录 5 篇 → `executeDefinitionProvider` 必须答插件那份 → `editor.action.revealDefinition` 真按键 ⇒ 那一篇真的被打开、首行 `# <命令>` |
| 载荷（VSIX 字节） | `release.yml` 的 `check_docs`：`extension/docs/commands/<name>` 与 `reference/commands/<name>` **逐字节**，对 8 个平台包 + universal **逐个**查 |
| 接缝 | `crates/cli/tests/extension.rs::doc_channels_agree_on_their_env_keys_and_dirs`：扩展写的键 ↔ front 读的键 ↔ `stage-docs.js` 的目录，三处逐字对齐（这条今天**没有**别的守卫） |

**反向验证（实跑）**：① `is_reserved_command` 里加 `#eval` 字面量 ⇒ front 判据 2 判红；
② 删正文一个 `##` 小节 ⇒ 判据 6 判红；③ 例子写错（`#check nope`）⇒ 判据 8 判红；
④ 藏起仓库真源 ⇒ wire/e2e 必须落到**插件那份**（P8 的 ① 档隐藏就是这一条）；
⑤ 删 `docs/commands` 的 stage 那一步 ⇒ e2e 计数与 release 冒烟同时判红。

## §5 边界（如实登记）

- **`#reduce` 的报告通道仍缺**（`session.rs` 丢 `CheckedEvent::Reduced`）：本刀**不依赖**
  它（走词法），但"命令输出"那条链对 `#reduce` 仍不完整 —— 独立议题，别混进来。
- `#env` / `#prove` / `#help` 是 **REPL 交互命令**（不在语言命令清单里）⇒ 不进表、没有
  文档落点。
- `set_option` 的**选项名/值**（`pp.explicit` / `true`）**有意没有落点**（它们不是引用）——
  判据 `…_does_not_steal_command_arguments` 钉住。
- CI 的 host-VSIX 冒烟（`.github/workflows/ci.yml`）今天对文档**零断言**（tactic 侧也
  一样）⇒ 本刀**不扩大**那条 job；源↔包的守卫在 release 冒烟（9 产物）✓。
- **Lean 4 对照：n/a** —— 这是编辑器导航/分发的**产品设计**，不是语义对齐项；Lean 4 的
  `#check`/`#print` 也没有"F12 跳文档"这回事（它的文档在手册里）。
