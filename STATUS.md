# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-18（第九十二轮：I16 落地 —— `import` 闭包 + 项目管理，版本 **0.57.0**；P0–P6 完成，P7 = backlog）
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

## 本轮进度（2026-09-18，第九十二轮：I16 落地 —— `import` 闭包 + 项目管理，0.57.0）

> 用户：「新产生一个 git 分支吧，全部按照建议，你给我完整做完一版我看看。这个变化比较大。」
> 分支 **`i16-imports-and-projects`**；设计文档 §8 的 Q1–Q7 **全部按推荐执行**；
> P0–P6 全部落地（P7 = backlog）。设计 + as-built =
> **`docs/design/imports-and-projects.md`**（§5.1 有三处与设计的偏差与唯一能力缺口）。

1. **语法与解析（P1）**：`Command::Import`（AST + token）、置顶校验、模块名合法性
   （`import 1Foo`/尾点/`-` 都被拒，`-` 给教学 hint）；3 个 parse 期错误码
   `import-malformed` / `import-not-a-valid-module-name` /
   `import-must-precede-declarations`。`import` 进 `is_reserved_command`，
   否则行首 `import` 会被当作应用实参吞掉。
2. **闭包编译（P2，核心）**：`crates/front/src/project/`（`mod`/`module_name`/`resolve`/
   `manifest`/`graph`/`report` 六文件，18 单测）。`plan_project` 定根（`--root` >
   最近 `sokonanoda.toml`（上溯止于 `.git`/HOME）> 入口目录——**无清单也能 import**，
   对真 Lean 的有意分歧）；后序 DFS 装载拓扑序（`VisitOutcome::Cycle` 保证入口最后）；
   `compile_all_units` 在**同一个 arena + 同一个 `EnvBuilder`** 按序跑完，
   import 命令先入环境 ⇒ `EnvLimit` 下标不变，**内核一行未改**。诊断**按命令下标**
   （`CompileOutput.error_cmds`）归属文件，`split_report` 还原每文件报告与事件；
   闭包级重名/prelude 冲突/依赖阻断（`import-dependency-failed` 只报一条）。
3. **CLI 与协议（P3）**：`--root` / `--no-project`、`build` 项目化、`query` 闭包内求值、
   `help` 增「multi-file projects」段；`docs/protocol.md` 补全部新码 + warning
   `import-has-open-exercises`；新增 `crates/cli/tests/imports.rs`（**12 条 e2e**，
   含 A1：无 import 文件与单文件路径逐字节一致）。
4. **缓存（P4）**：`ProjectPlan::digest(options)` = 拓扑序上每个模块 (名字, 源, imports)
   + prelude 模式的稳定哈希；`CACHE_FORMAT` 1→2；依赖改动必然 miss（e2e 实测）。
5. **LSP（P5）**：`Docs{map,order,root,active}` 多文档、`initialize` 捕获 root、
   按 URI publish、`did_close` 清理、**跨文件 `goto_definition`**
   （`QueryDoc::project_definition`）；项目模式下补挂 `-- soko:hint` 阶梯
   （否则带 import 的入口答不出 hints）。`crates/lsp/src/tests/project.rs` 4 条 e2e（导入可见 / 缺失 import 只报错 / 跨文件跳转 / 本地名仍留在入口）。
   **唯一缺口**：依赖变更后不自动重编译其它已打开文档（第一版在 tower-lsp 串行
   通知 + socket 缓冲下挂住，已回退；余项登记 `docs/TESTING.md` §5.7）。
6. **教学面与门面（P6）**：单元⑪「模块与项目」（CN/EN + 两份 solution，199/249 行）
   + 可运行两文件项目 `course/unit11-project/`（`sokonanoda.toml` + Logic/Canvas/
   Exercises + solution）+ `course.json`/`course/README.md`；goldens 重钉
   （画布 (7,6,0) 双语、solution (12,0,0)；总计 11 单元 / checked 85 / open 65 /
   failed 0）；`site/data/site.json` 重新生成。
7. **收尾**：版本 **0.56.1 → 0.57.0**（`Cargo.toml` + `editor/vscode/package.json` +
   VS Code CHANGELOG）；文档同步 `docs/architecture.md`（新增 §4.5 项目流水线 +
   §8.10 两个坑，仓库地图指向 TESTING）、`docs/TESTING.md`（3 行项目守护 + §5.7 盲区）、
   `docs/design/compile-cache.md` §7、`ROADMAP.md` I16、`REQUIREMENTS.md` §9（九十二）、
   `docs/HANDOVER.md`、三个 skills（teacher 多文件命令 + events 新码 + curriculum 单元⑪）、
   `AGENTS.md`、`dsh/README.md`、`editor/vscode/README.md`、
   `docs/design/deepseek-harness.md`；`scripts/soko gate` PASS +
   `cargo test --workspace --locked` 全绿。
8. **没做什么（有意）**：课程语料不回填 import（除新增单元⑪）；`watch --workspace`
   与 `soko/project` 不项目化；不做跨进程 decl 复用（v1 只缓存报告）；产物仍在用户
   缓存目录；`namespace`/`open`/`[deps]` 留 P7。

## 本轮进度（2026-09-17，第九十一轮：多文件 `import` 与项目管理 —— 调研 + 设计 + 计划 I16）

> 用户：「我想增加 代码import +project管理，帮我调研一下其他语言都是怎么分别处理单文件，
> 和项目。项目如何维护。sokonanoda如何实现，具体执行方案是什么」。
> **本轮只出调研 + 设计 + 计划，不动实现、不 bump 版本**（沿用第八十八轮先例）。
> 设计文档 = **`docs/design/imports-and-projects.md`**（ROADMAP **I16**）。

1. **调研（3 个并行 subagent，全部直抓官方文档/源码；`web_search` 无 API key 故走
   `curl`/`web_fetch`）**：
   - `docs/notes/multifile-prior-art.md` —— Coq/Rocq、Agda、Isabelle、Idris 2、Rust、Go、
     Python、JS/TS、Haskell/OCaml、JVM 的"单文件 vs 项目"逐系统记录 + 5 问横向表 +
     可抄模式/反模式（每条带官方 URL）。
   - `docs/notes/project-roots-and-incremental-caches.md` —— LSP 契约（`rootUri` **可为 null**、
     `didChangeWatchedFiles`、诊断"替换不合并"、**明文允许从缓存读诊断**）、9 个服务器/
     扩展的根发现与错根症状、失效与产物（Lake trace / GHC 指纹 / OCaml `.cmi` 摘要 /
     Coq `.vo` digest / `.tsbuildinfo`）、原子写与并发、**"缓存判定结果是否安全"的三条规则**。
   - 代码接缝（只读勘察，`path:line`）：一次编译 = 一个 arena + 一个 `EnvBuilder`
     （`compile/check.rs:424-425`）；内核名字身份 = **指针地址**（`kernel/util.rs:133-142`）
     ⇒ 跨 arena 复用环境不可能；`EnvLimit` 只表达**扁平前缀环境**（`kernel/env.rs:224-234`）；
     两遍 check-then-add（`check.rs:339-359`）；judge 只吃文本（`judge.rs:137-265`）；
     LSP 单槽 `Mutex<Doc>`（`lsp/lib.rs:58-60,152-155`）。
2. **设计一句话**：把**编译单元**从「一个文件」升级为「**项目闭包**」——`import Foo.Bar`
   用真实 Lean 4 置顶语法、模块名↔路径用 Lean 同款规则（`-` 非法 → 教学 hint）、
   项目根 = 最近祖先的 `sokonanoda.toml`（**向上搜索止于 `.git`/workspace 根**，
   `--root` 覆盖，无清单退化为"入口文件目录 = 模块根"——**对真实 Lean 的刻意
   divergence**：官方 `lean` 的搜索路径里**没有**文件自己的目录、cwd 只影响模块名
   的计算，§2.1 有源码依据；Q2 保留改回严格对齐的选项）；跨模块声明由 front 在
   **同一个 arena / 同一个 `EnvBuilder`** 里按拓扑序 `add_declar`（导入声明先入表，
   索引 `0..k`），**内核一行不改**、`EnvLimit` 语义零改动。
3. **硬边界与不变式**：① 无 `import` 的文件**行为逐字节不变**（缓存键、事件流、
   两处 golden 计数全不动 —— A1 用 `--json` 对拍守住）；② 每个 `Span` 只属于一个文件
   （**否掉源码拼接方案**）；③ 判定仍由内核终审；④ 用户路径零 cargo。
4. **量化动机（实测）**：45 个语料文件 3851 行里 **1217 行（31.6%）** 落在"名字在
   ≥2 个文件出现过"的声明块内；**71 个名字有 ≥2 种定义**、**20 个变体从未同单元共现**
   （`Or` axiom vs inductive、`Iff` def vs axiom、`And.*` 三种 binder 类型）；
   `course/unit6:21-36` 与 `unit7:16-31` 是**逐字节相同的 16 行 Nat 块**（中英共 4 份）；
   `solutions/` 与画布骨架 19/19、19/19、27/27 逐一对应。结论：**值得做 import 的理由是
   "同名不同义今天无法表达"，不是省行数**（最大 5 组重复一共只省 122 行）。
5. **分阶段计划 P0–P7**：P0 设计契约（本轮）→ P1 语法/模块名/resolver（含 fuzz 一次）→
   P2 闭包编译（一次 prelude、失败阻断、诊断归因）→ P3 CLI+协议（`--root`/`build`/`query`）→
   P4 闭包哈希缓存（可选信任台账，**启用前必须换强哈希**）→ P5 LSP（多文档表、根发现、
   反向后继重编、跨文件跳转、`didChangeWatchedFiles`）→ P6 第 11 单元 + 门面 + 发版 0.57.0 →
   P7 backlog（decl 级产物、`namespace`、跨项目依赖、语料重构）。
6. **风险清单里最值钱的三条**：① **三道"静默错误"门**（`front/tests/perf.rs:108` 的
   `kernel_checks <= 1`、`cli/tests/watch.rs:292-298` 的每文件独立契约、judge/suggest
   的静默无建议）；② **CI/Pages 不会发现"画布不再自包含"**（anchor 只在本地 `soko gate`，
   `gen-site-demos.py` 只守产物新鲜）；③ `elab-duplicate-declaration` 被测试枚举过 4 次却
   **从未真正触发**——而"同名到达两次"正是天真 import 实现的第一症状。
7. **待用户拍板 Q1–Q7**：清单格式（TOML/JSON/纯标记）、无清单时是否允许 import、
   prelude 模式决策者、是否做已检查声明的跨进程复用、课程语料是否同轮重构、
   `watch`/`soko/project` 是否 v1 就做、产物位置（用户缓存目录 vs 项目内 `.soko/build`）。
8. **本轮产物**：`docs/design/imports-and-projects.md`（新）、
   `docs/notes/multifile-prior-art.md`（新）、`docs/notes/project-roots-and-incremental-caches.md`（新）、
   `ROADMAP.md` I16、`REQUIREMENTS.md` §9（九十一）、`docs/README.md`（设计/笔记索引）、
   `docs/HANDOVER.md` §3 G、本文；**零代码改动、零版本变更**。

## 本轮进度（2026-09-17，第九十轮：清掉 HANDOVER §4 的 LSP 测试文件债 + 0.56.1 发布）

> 用户：「继续 handover 吧，完成之后再 bump」。本轮 = 清第八十九轮登记的两笔结构债
> 里剩下的那笔（`crates/lsp/src/tests.rs` 2567 行），然后 bump 发 **0.56.1**。

1. **测试文件拆分（零语义改动）**：`crates/lsp/src/tests.rs` 2567 行 →
   `crates/lsp/src/tests/` 一目录：`mod.rs` **399**（31 个共享 const/fixture +
   `pub(crate) use` 再导出，子模块靠 `use super::*;` 取用）+ 9 个特性文件
   （`hover` 392 / `lenses` 366 / `navigation` 280 / `state` 262 / `goals` 252 /
   `lifecycle` 242 / `hover_brackets` 167 / `tokens` 122 / `perf` 117），
   `by_sorry_range_tests.rs`（60）原地保留。`lib.rs` 仍 **1105 行**——
   `#[cfg(test)] mod tests;` 自动解析到 `tests/mod.rs`，一行未改。
2. **"移动而非改写"的证据**（这次也按上轮的标准自证）：HEAD 的 `tests.rs` 里
   **107/107 顶层 item 逐字出现在新文件**、8/8 banner 注释保留、规范化代码行
   多重集 **2394 == 2394**（only-in-old 0 / only-in-new 0）；函数名 **95/95 一致**、
   测试名各出现一次（76 个测试：72 `#[tokio::test]` + 4 `#[test]`）、
   assert 记账 **214（tests/）+ 6（by_sorry）== HEAD 的 214 + 6 = 220**。
   新增行只有 plumbing：模块 doc 4 行、`use super::*;` ×10、`pub(crate) use` 再
   导出块、`mod …;` ×9；编译期唯一被迫改动是去掉再导出里没人用的 `Value`。
3. **两轮验证**：拆分中途（全部子文件首次编译通过）与冻结最终态各跑一遍
   `cargo test -p sokonanoda-lsp --locked` = **117 passed / 0 failed**；
   `cargo fmt --check` exit 0（**首次 fmt 没有改动任何文件**）；
   `cargo clippy -p sokonanoda-lsp --all-targets` exit 0、`crates/lsp/**` 零 warning。
4. **HANDOVER §4 的债清零**：`docs/HANDOVER.md` 该条从"已知债 + 拆分方案"改为
   "0.56.1 已清 + 最终布局"；`docs/TESTING.md` 的 LSP 行、设计文档 §3.2 文件表、
   `ROADMAP.md` I15 的备注同步到 `tests/` 新路径与新行数。
5. **版本 0.56.0 → 0.56.1**（内部重构，无用户可见变更）：`Cargo.toml` +
   `editor/vscode/package.json` 两处同步、`editor/vscode/CHANGELOG.md` 记
   "内部重构（测试文件拆分），扩展行为不变"。按仓库流程 push main → `ci.yml`
   auto-tag `v0.56.1` → `release.yml` 出八平台产物 + VSIX + marketplace。
6. **验收**：`cargo test --workspace --locked` 全绿（756）、fmt 零 diff、
   clippy 教学 crates 零 warning、`scripts/soko gate` **PASS**；发布 job 全绿后
   用发布产物复验（同第八十九轮的做法）。
7. **本轮产物**：`crates/lsp/src/tests/`（10 个文件）、`docs/HANDOVER.md`、
   `docs/TESTING.md`、`docs/design/agent-query-channel.md`、`ROADMAP.md`、
   `REQUIREMENTS.md` §9、`editor/vscode/CHANGELOG.md`、两处版本号。
