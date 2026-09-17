# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-17（第九十一轮：多文件 `import` 与项目管理设计 + 外部调研；0.56.1 已发布，本轮不动实现、不 bump）
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

## 本轮进度（2026-09-17，第八十九轮：内核真相查询通道落地 —— H6-A/H6-B/H6-C + 门面收尾）

> 续第八十八轮的设计（`docs/design/agent-query-channel.md`，ROADMAP **I15**）。按
> **H6-A → H6-B → H6-C** 逐项实现并测试，收尾做 H6-D 文档/门面同步；版本
> **0.55.0 → 0.56.0**（agent 可见的新能力 `query`，QD-7）。

1. **H6-A 真相层 + CLI（`front::query` + `sokonanoda query <op>`）**：
   `crates/front/src/query/{mod.rs,types.rs,tests.rs}` = **编辑器无关的唯一真相**
   （`QueryDoc` + `check`/`state`/`goals`/`holes`/`hints`/`reduce`）；
   `QueryError{NotParsable,OutsideDeclarations,PositionOutOfRange}` 把"正常的没有"
   与"问不出来"分开（各带稳定 code + 中文 message）。`crates/cli/src/query.rs`
   输出**单 JSON 对象**（`{schema:"soko.query/1", op, version, ok, data|error}`），
   退出码 = **0 答上了（含 `ok:false` 与开放 `sorry`）/ 1 内核拒绝 / 2 用法**；
   `--text` 支持未落盘中间态。契约写进 `docs/protocol.md`。
2. **LSP 改为调用真相层 + 结构债清零（同一轮完成，A1/A5）**：
   `soko/goals`/`stateAt`/`nextHole`/`hints` 与 hover 的 tactic 视图全部改为调
   `front::query`；新增 `crates/lsp/src/query_map.rs`（**唯一的形状映射点**：
   offset↔`Range`/`Position`、`QueryError`→既有空结果），删除 `select_state_at`/
   `StateSelection`/`runs_of`/`status_str`/重复的 `decl_name`/`goal_decls` 的 75 行
   主体等 → `crates/lsp/src/lib.rs` **4256 → 3988 行**。再按模块化硬规则把两个测试
   模块移出文件（`tests.rs` 2567 / `by_sorry_range_tests.rs` 60，**断言一字未改**，
   214+6 条 assert 与 HEAD 逐行等价）并抽出 `protocol.rs`（wire 类型，159）与
   `tokens.rs`（semantic token 辅助，107）→ **lib.rs 1105 行，≤1200 达标**。
   ⚠️ **过程留档（我自己的错）**：删完重复后我曾**没量就**把"≤1200 行"作废，
   理由是"剩下的都是协议服务代码"——`wc -l` 显示 3988 行里 **2638 行是
   `#[cfg(test)]` 模块**，非测试代码只有 ~1350 行，移出测试随手就达标。教训
   （**改验收标准之前先把被验收的东西量一遍**）进 `docs/LESSONS.md`；最终口径 =
   "**无重复实现**" **且** "**单文件 ≤1200 行**"（设计文档 §2.6/§3.2/§11 A5 已改）。
3. **⚠️ 抽层真的出过一次语义漂移（本轮最重要的教训，已进 `docs/LESSONS.md`）**：
   LSP 侧 117/117 全绿的情况下，**没有 `by` 块**的声明被错误地统一成"根状态"
   （已证声明凭空多出一个目标、半成品证明 `fun (a) (h) => sorry` 丢掉已引入的假设）。
   抓出它的不是测试而是**穷举对拍**：删除旧实现前，在 5 个画布的**每一个光标
   offset**（0..=len）上比较新旧两份实现，**709 次比较 / 424 处不一致全落在这一个
   分支**。既有测试没红是因为 LSP 唯一覆盖它的用例，画布**没有 lambda 前缀**，
   "剩余目标"恰好等于声明类型——**"新测试通过"不等于"新语义被测试"**。
   修法：`by_steps.is_empty()` 单独走"声明级目标 + 上下文"（协议 `docs/protocol.md`
   原文），红先单测 2 条（开/闭两分支）钉死，并把 CLI≡LSP 一致性契约扩到这两个
   **判别性输入**。教训同时写进设计文档 §4 as-built 3 / §12 风险表。
4. **H6-B MCP 传输 + DSH 接线**：`dsh/mcp/server.js`（零依赖 stdio 桥，
   `initialize`/`tools/list`/`tools/call`，六工具全部转发 `scripts/soko query …`；
   `server/discover` **立刻**用 `-32601` 拒绝——沉默会等满 SDK 的 60 s 超时）、
   `scripts/soko mcp`、`dsh/cordis.patch.yml` 的 `mcp-sokonanoda` 行（**默认关闭**，
   注释写清信任边界：MCP server 是 DSH 沙箱外的可信代码）。
   五个实测坑写进设计文档 §H6-B（探测进程/`capabilities.tools`/换行分隔 JSON/
   只有 `content[].text` 进模型/必须无状态可重启）。**实测验收**：DSH headless
   会话里模型调用 `mcp__sokonanoda__state` 拿到目标。
5. **H6-C 两个 front 缺口修掉**（原 `docs/HANDOVER.md` §3 E）：
   ① `derive_recursor` 在"**带索引 + 字段写在结果箭头链里**"时用只认 Ident/App 的
   `src_spine` 读索引实参 → 改为已会剥箭头的 `spine_of_codomain`（`elab.rs`），
   并顺带修掉写死的 `is_k: false`（单构造子 `Prop` 归纳因此被内核拒）；
   ② `inductive` 参数/ctor 字段不吃多名字 binder 组 `(A B : Prop)` → 解析器改调
   组感知的 `push_binders`（AST/elab 未动）。**课程随之简化**：unit9/unit10 的
   `Le`/`Even` 不再手写 `rec`/`iota`、`Or (A : Prop) (B : Prop)` 收成 `(A B : Prop)`，
   中英代码逐字节一致、**golden 事件计数不变**（unit9 `(13,8,0)`、unit10 `(7,6,0)`）。
   两条修复都先有"修复前红"的复现测试（`indexed_inductive_with_arrow_style_field_derives_recursor`
   等 4 条）；CLI e2e 三条 + 解析器三条补齐三层。
   **⚠️ 追加发现（"三层缺一不可"的实证）**：`is_k` 的第一版把它近似成"单构造子 +
   无索引 + 字段数 == 参数数"，front 单测全绿，但**内核拒了两个判别性形状**——
   `Both (A B : Prop)` + `mk (a : A) (b : B)`（字段数恰好等于参数数 → 内核要
   `is_k: false`），以及反向的 `Q : Nat -> Prop` + `q : Q 0`（**有索引但无字段 →
   内核要 `is_k: true`**）。抓出它的是新加的 CLI e2e 层。现按内核
   `init_k_target` 逐字镜像（`is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()`），
   两个反例各留一条单测；方法（镜像内核谓词 = 逐字翻译 + 给判别性输入写测试）进
   `docs/LESSONS.md`。另外首版 `arrow_style_indexed_recursor_reduces` 名字承诺 iota
   却没碰 recursor（`theorem pz_again : P 0 := pz`），已改为 `Type` 值索引族 +
   `match` + `#reduce`。
6. **一致性契约（A4，防两套真相）**：`crates/cli/tests/query.rs` 12 项，其中
   `query_check_counts_match_the_json_event_stream` 钉"同一份判卷两个视图"，新
   `query_state_agrees_with_the_lsp_state_at_request` / `query_state_and_lsp_agree_without_a_by_block`
   **起真实 `sokonanoda-lsp` 二进制**做字段级对拍（根状态 / tactic 之内 / tactic 之后 /
   无 `by` 的开放与闭合）。注意：它比对的 `target/<profile>/sokonanoda-lsp` 可能是旧
   构件——**改了 front 只跑单 crate 测试会拿旧二进制对拍**（先 `cargo build --workspace`），
   这是特性也是坑，已写进设计与教训台账。
7. **两处刻意的 wire 边界对齐**（此前无测试覆盖，已记录）：`soko/hints` 的声明命中
   与 `stateAt` 统一为**含末尾**（旧路径开区间：光标恰在声明末偏移/末行行尾之后返回
   `[]`，现在返回阶梯）；由 offset 换算的 `Range` 改用**UTF-16** 列（LSP 规范口径，
   与其它响应一致；BMP 文本逐字节相同，仅增补平面字符不同）。扩展侧无需改动。
8. **H6-D 同步**：`AGENTS.md` Setup（`query` 两视图 + 六个 MCP 工具）、
   `skills/sokonanoda-teacher`（"先问，别扫"）、`skills/sokonanoda-dev`（"真相层不得
   绕过"）、`dsh/README.md`（查询一节 + 信任边界）、`docs/protocol.md`、
   `docs/TESTING.md`、`docs/HANDOVER.md`、`ROADMAP.md` I15 as-built、VS Code
   README/CHANGELOG/`package.json` 版本同步、`site/` agent prompt 一句。
9. **H6-E backlog（不做承诺）**：DSH Infoview 客户端插件（消费 `query goals/state`）、
   `SessionStart` 自动 provisioning、把启动器 + Lean 工具链 deny 拦截 + `/sokonanoda-*`
   命令打成一个 npm 插件包。
10. **发布**：CI 全绿 → auto-tag `v0.56.0` → release **11 个 job 全 success**
    （8 平台 build + VSIX + **marketplace 发布一次成功** + GitHub Release），
    26 个产物；并**用发布产物实测**（下载 CLI：`query state` 与仓库一致；下载 LSP：
    无 `by` 的开放声明 `goal='a' binders=['a','h']`、已闭合 `goal=None`）。
11. **本轮产物**：`crates/front/src/query/*`、`crates/cli/src/query.rs`、
    `crates/cli/tests/query.rs`、`crates/lsp/src/query_map.rs` + `protocol.rs` +
    `tokens.rs` + `tests.rs`/`by_sorry_range_tests.rs`（lib/hints/render 收敛）、
    `dsh/mcp/server.js`、`dsh/cordis.patch.yml`、`scripts/soko`（`mcp` 分支）、
    `crates/front/src/parser.rs` + `compile/elab.rs`（H6-C）、课程 9 个文件简化、
    文档/门面同步（见第 8 条），版本 0.56.0。
