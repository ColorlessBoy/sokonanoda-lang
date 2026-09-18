# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-18（第九十六轮：待办批次 4 —— 项目状态视图；版本 **0.58.0**）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-18，第九十六轮：待办批次 4 —— 项目状态视图，0.58.0）

> 承第九十四轮定下的批次计划（用户「按照你的计划，从上到下依次改进」）：
> **批次 1/2/3 已完成，本轮做批次 4 = `soko/project` 项目状态可视化**。
> 「我在哪个项目里、根在哪、清单是谁、哪个模块拖坏了入口」以前只能靠 CLI 反复
> 跑或读文档推；现在它是一个只读、机器可判的查询，编辑器与 agent 同一份真相。

1. **真相层（front）**：新增 `project::ModuleStatus {Compiled, LoadFailed, Blocked}`
   + `ModuleReport::status`——此前"编译过（可能有错）/ 加载失败 / 被上游拖住"三者
   都表现为空报告，消费者分不清**根因与受害者**；`compile_plan` 按
   `failed`/`blocked`/`result_blocked` 三个已知集合填状态。
   `query::ProjectView`（wire）+ `QueryDoc::project_view()` /
   `project_view_reason()`：从**已编译的** `ProjectReport` 派生（不重跑内核、
   不算摘要、不碰缓存），路径 `canonicalize` 成绝对路径（CLI 与 LSP 对同一文件
   给出逐字相同答案）；单文件是**另一种合法状态**（`None` + `no-imports` /
   `no-path` / `parse-error`），不是错误。
2. **三个传输同一份真相**：CLI `query project`（`soko.query/1` 信封、
   `data = {project, reason}`、恒退出 0）+ help 行；MCP 工具 `project`
   （`mcp__sokonanoda__project`，薄转发，`dsh.rs` 契约从六工具改七工具）；
   LSP `soko/project`（回显 `uri`/`version`，走 `focus_request` + 未保存缓冲）。
3. **VS Code 0.58.0**：资源管理器新增「项目」树（新模块
   `editor/vscode/project-tree.js`：渲染与请求分离）——根 = 模块根 + **清单来源**
   （`sokonanoda.toml` 或"零配置"）+ 计数；子 = 拓扑序模块 + `入口`/`依赖` +
   声明/练习/错误 + 状态图标 + `message`（根因说出来缺哪个模块）；点击开模块、
   点根开清单；单文件一条占位行；状态栏 tooltip 加项目行（不新开 item）；
   `sokonanoda: refresh project view` 命令 + view/title 按钮；答案指名别的文档
   ⇒ 丢弃（沿用 `soko/goals` 的身份纪律）。
4. **测试（三层）**：front 4 条（闭包/清单/失败 vs 被阻断/单文件原因）；
   CLI 3 条 e2e（真二进制：字段齐全、根因 vs 受害者、`project:null`+reason）；
   LSP 2 条（身份回显 + 未落盘编辑改坏 import ⇒ 入口 `load-failed`）；
   扩展 stub 宿主 3 条（渲染闭包/单文件占位/丢弃他人答案）。
   顺手修好 stub 的两处不忠实（`MarkdownString` 吞掉构造参数——**测试因此看不见
   tooltip 内容**；`createStatusBarItem` 不返回实例）并清掉 5 行遗留 DEBUG 打印。
5. **版本与文档**：0.57.0 → **0.58.0**（Rust 与扩展同步，契约测试逼出来的）；
   新增设计 `docs/design/project-view.md`（§9 明确不做依赖图/写操作/模块级缓存）；
   `docs/protocol.md`（`query` op 表 + `soko/project` 小节）、TESTING（新行 +
   七工具）、architecture（仓库地图 + §4.5 第 7 步）、HANDOVER（LSP 能力/新字段）、
   AGENTS（命令面 + 七工具 + 自定义请求表）、skills、dsh/README、
   扩展 README/CHANGELOG、LESSONS（stub 忠实性）、本文件与 `REQUIREMENTS.md`
   §9（九十六）。
6. **验收**：`cargo test --workspace --locked` **871 passed / 0 failed**
   （front 466（457 + perf 3 + perf_project 6）/ cli 217 / lsp 137 / kernel 51）；
   `node editor/vscode/test-extension-host.js` **11/11**（另三个 Node 套件
   18/18、7/7、10/10）；`scripts/soko gate` PASS；site 数据重新生成。
7. **批次 1–4 全部完成**。剩下的只有 P7 长尾（`[deps]`、`namespace`/`open`、
   `watch` 项目模式、decl 级产物）与 `docs/HANDOVER.md` §4 的结构债清单
   （`compile/tests.rs` 4828 / `elab.rs` 2854 / `parser.rs` 2065 / `lsp/lib.rs` 1554）。

## 本轮进度（2026-09-18，第九十五轮：待办批次 3 完成 —— 拆 `run_pass` + 项目整理）

> 用户：「可以，前三个你先做完，把项目理干净」（批次 1/2 已在第九十四轮完成）。
> 本轮 = **批次 3**（`run_pass` ≈1174 行单函数 → 三个模块，三次提交，每刀只动位置）
> + **一轮仓库整理**（死代码、过期文档、模块地图、经验台账、STATUS 归档）。

1. **第一刀 —— 闭包装配件出 `check.rs`**：`SourceUnit` / `unit_ranges` /
   `split_report` / `compile_all_units` → `compile/units.rs`（108 行；单文件也走同一条路径）。
2. **第二刀 —— `run_pass` 尾部出 `check/kernel_phase.rs`**：`builder.finish()` 之后的
   内核 check-then-add + 事件/错误 + 每命令签名与 early cutoff + 报告装配（≈360 行）
   原样搬进 `finish_pass(Walked)`；`check.rs` 1918 → `check/mod.rs` **1522**。
3. **第三刀 —— 命令走查出 `check/walk.rs`**：`Walk`（可变累加器：builder /
   known_universes / inductives / out / ops / cmd_hovers / decl_states / example_idx）、
   `CmdCtx`（每命令派生的 `Cow` 前缀、模板、信任位）、每个 `Command` 变体一个方法；
   arm 里的 `continue` 改 `return`（8 个 arm 都没有内层循环）。最终
   `check/mod.rs` **791** + `walk.rs` **951** + `kernel_phase.rs` **413**；
   单文件仍走 `Cow::Borrowed` 前缀（零新增分配，A1 不变）。
4. **验收（方法论收获）**：除 `cargo test --workspace --locked` **862 passed / 0 failed**
   外，做**二进制对拍**——`git worktree` 取改动前的树，两个 CLI 对同一批输入
   （全部 58 个 `.sokonanoda` + `--root` / `--no-project` / stdin /
   `query check|goals|holes`）输出**逐字节相同**；8 个 arm 另做"逐字符同构"
   （空白无关）比较。方法与两个坑（`cargo fmt` 会重排；**两个 worktree 别共用
   `CARGO_TARGET_DIR`**——后建的树会静默覆盖前者的二进制）写进
   `docs/TESTING.md`「二进制对拍」与 `docs/LESSONS.md`。
5. **整理（死代码）**：删掉只写状态 `built_inductives`（唯一消费者是文件尾的
   `let _ = …`；顺带去掉归纳块每次的无用 `Vec` 克隆）与 `def` 开练习路径里推**空**
   `CmdHover` 的空操作（`resolve_hovers` 只读 `nodes`）——同样过二进制对拍。
6. **整理（性能台账口径）**：复盘台账发现**采样口径**问题——项目层 perf 套件在同一
   测试二进制里**并行**跑，把单次操作成本放大 3–4×（同一份代码：单跑 32.4ms /
   串行 33–38ms / 默认并行 118–152ms；LSP didOpen+按键 12+12ms vs 64+50ms）。
   修法：`scripts/perf-ledger.sh` / `perf-report.sh` 一律 `--test-threads=1`、
   `perf_project` 的分阶段/缩放改 `measure_best(…, 3)`；`docs/PERF.md` 的基线表
   按**串行口径**重写（教学规模 2/3/5 × 12 声明一次按键 **14/16/24ms**，4×20 编译
   32–38ms）并写明"跨口径不可比"；教训进 `docs/LESSONS.md`。**旧台账条目是并行口径，
   比较时先看是否落在 ±25% 内。**
7. **整理（文档）**：HANDOVER 里"项目入口 quick-fix 仍未做"的过期段落更正；§4 新增
   剩余结构债盘点（`compile/tests.rs` 4828 / `elab.rs` 2854 / `parser.rs` 2065 /
   `lsp/lib.rs` 1554 / `vscode/extension.js` 1493，按建议顺序）与
   "开练习的类型子表达式没有 hover 行"（**刻意保留现状**，含补法）；`architecture.md`
   仓库地图 + §4.2 补"阶段 ↔ 模块"对照；`TESTING.md` 新增「二进制对拍」小节 +
   精确测试构成（kernel 51 / front 462 / cli 214 / lsp 135）；本文件归档第九十二轮。
8. **批次 3 完成 ⇒ 待办只剩批次 4**：`soko/project` 项目状态可视化（协议 + VS Code
   状态/树：模块根、清单来源、闭包模块、失败模块）。

## 本轮进度（2026-09-18，第九十四轮：待办批次 1 —— 项目 quick-fix + 编辑器外改动刷新）

> 用户：「还有没有做的TODO吗？fix修复或者优化体验的设计」→ 我列出 A/B/C/D 四组未做项
> 与四个设计 → 用户选「批次 1（推荐）」并要求「按照你的计划，从上到下依次改进」。
> 本轮 = 批次 1（A1 + C3 + B3 与 A2）。

1. **判据前缀抽成真相层（C3）**：`judge.rs` 四个合成判定入口各增 `extra_prefix`
   变体（`judge_terms_with` / `judge_infer_with` / `judge_hole_fill_with` /
   `judge_value_replace_with`；旧签名委托 `""` ⇒ 单文件逐字节不变，缓存键含前缀）。
   `QueryDoc::judge_prefix(offset)` 是唯一真相入口（依赖源码去 `import` 行、拓扑序）；
   `importless_source` 从 `check.rs` 私有函数提成 `project::importless_source` 一份实现。
2. **项目入口恢复 quick-fix（A1）**：`front::suggest_with`、`probe_sub_goal_types_with`
   接前缀，LSP 的 code action 传 `doc.query().judge_prefix(...)`。真 LSP 探针：
   修复前 `null` → 修复后 `refine And.intro a b sorry sorry`（与单文件同形）。
3. **第三层根因**：`run_pass` 的 `GoalTemplates`（refine/intro 的构造子索引）按
   **单个单元**构建 ⇒ 项目入口看不见导入的构造子，建议凭空消失；现在按"拓扑序前缀 +
   本单元"的命令表构建（`new_for` 只读命令表，`src` 是占位）。
4. **项目模式子洞探针（B3）**：`probed_report` 不再因项目模式整段跳过；
   `query goals --probe` 在项目入口给出 `spine_x` 两个子洞期望类型 `a`/`b`（与单文件一致）。
5. **编辑器外改动自动刷新（A2）**：LSP 实现 `workspace/didChangeWatchedFiles`
   （扩展早已声明 `**/*.sokonanoda` watcher，服务端此前静默忽略）：只重编译
   **闭包里含该路径**的已打开文档、缓冲区优先；缺失模块也记着期望路径，所以
   "文件被创建出来"同样触发刷新。真二进制探针：模拟 `git checkout` 改坏依赖 →
   入口立刻报 `elab-unknown-identifier`。
6. **测试**：LSP `code_actions_work_in_a_project_entry`、
   `an_external_change_to_a_dependency_refreshes_the_open_entry`；front
   `project_documents_expose_a_judge_prefix_and_probe_sub_goals`。
   `cargo test --workspace --locked` **860 passed / 0 failed**；项目 perf 复测无回退
   （4×20 compile 131ms、缩放 1.9×、按键 139ms）。
7. **文档**：`TESTING.md` §7b 标闭环（三层根因 + 守护）、多文件 LSP 行扩写；
   架构 §4.5 判据前缀段改写；设计 P7 两项划掉；`vscode-dev-guide` 坑 15 更新；
   本文件与 `REQUIREMENTS.md` §9（九十四）。
8. **批次 2（同轮完成）—— `query` 走项目缓存 + 协议身份回显**：
   - `query` 与 `check`/`build` 共用 `crates/cli/src/project_cache.rs` 的闭包摘要键；
     `QueryDoc::check()` 不再二次编译（复用 `set_text` 存下的 `CompileOutput`，新增
     `set_cached_entry` / `compiled_output`）。3×12 实测：`query check` 冷 49→**25ms**、
     热 37→**3.4ms**；`build --json` 立刻看到入口是同一份键的 hit。
   - `soko/goals` 回显 `uri`+`version`、`soko/stateAt` 回显 `uri`；VS Code 扩展比对后
     丢弃不匹配答案（stub 宿主 8/8），协议写进 `docs/protocol.md`。
   - 测试：CLI `query_uses_the_same_project_cache_as_check_and_build`、LSP
     `custom_responses_echo_the_requested_document_identity`、扩展宿主
     `an answer that names another document is dropped`。
9. **批次 3（同轮起步）**：第一、二刀（`units.rs` + `check/kernel_phase.rs`）同轮完成，
   第三刀与收尾清理见**第九十五轮**。
10. **下一批**：批次 3 余下 → 批次 4（`soko/project` 项目状态可视化）。批次 3 已在
    第九十五轮完成；**批次 4 待做**。
