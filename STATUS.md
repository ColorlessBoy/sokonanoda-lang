# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-18（第九十五轮：待办批次 3 完成 —— 拆 `run_pass` + 项目整理；版本 **0.57.0**）
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

## 本轮进度（2026-09-18，第九十三轮：项目层性能例行化 + 测试扩充 + 编辑器审计修复）

> 用户：「各个环节的性能例行化检测并记录在案，方便后续分析检查。再多增加点项目相关
> 的测试，功能和性能，包括 vscode 前端会不会卡，有没有实现不对的地方。」
> 本轮 = **可复现的性能台账**（分阶段、机器可读、提交进仓库）+ 项目层功能/性能测试
> 扩充 + 对扩展前端做了一次审计并把查出的**两处真 bug** 修掉。

1. **性能例行化（分阶段 + 台账）**：新增 `crates/front/tests/perf_project.rs`
   （plan / digest / compile / 一次按键 / 内存覆盖 / 线性缩放）、
   `crates/cli/tests/perf_project.rs`（冷、热、依赖改动必 miss、`build`+`query`）、
   `crates/lsp/src/tests/perf.rs` 的三条项目例（didOpen / 按键 / 改依赖刷新下游 /
   请求延迟）。每个阶段打印 `PERFJSON`（`schema: soko.perf/1`），
   **`scripts/perf-ledger.sh` → `docs/perf/ledger.jsonl`**（追加式、带
   version/commit/日期/宿主/`cli_profile`）；`docs/perf/latest.json` 便于直读。
   CI 的 "Performance report" 与 `scripts/perf-report.sh` 同步收录这三段。
2. **实测基线（教学规模无感）**：front 4×20 项目 compile 90–110ms、一次按键
   96–123ms、缩放线性（4× 规模 ⇒ 2.4–3.0×）；**教学规模 2/3/5 模块 × 12 声明
   一次按键 12–46ms**；LSP 项目按键 25–49ms 且**每次按键只发 1 份诊断**；
   CLI release 冷 23.5ms / 热 4.4ms；内存覆盖与读盘同价（33.1 vs 33.2ms）。
3. **扩展前端审计（两个真 bug，已修 + 已加回归）**：
   - **切文件竞态**：`loadDeclarations()` 在 `await` 之后读 `this.uri` 建树节点——
     A 的请求、切到 B 之后回来，树上那行的标签是 A 的声明、点击却是
     `revealRange(B, A 的洞)`（stub host 复现）。现在请求发起时钉住 URI、回来先比对。
   - **诊断监听器全窗口且无去抖/去重**：别的扩展（TS/ESLint）报错也会跑一整轮
     `soko/goals`；项目模式一次编辑的事件里会跑 **2 次** goals + 2 次 Infoview
     整表重建。现在按 URI 过滤（只理 `.sokonanoda`）+ 150ms 去抖 + 并发合并 +
     载荷指纹去重。
   - 顺手：课程树缓存一次 CLI 运行（30s TTL，热缓存一次 ~320ms / 11 个单元）、
     `server.js` 下载回退的 `execSync tar` 改 `await execFile`（不再冻结宿主）。
   - **新增测试层** `editor/vscode/test-extension-host.js`（stub 的
     vscode/languageclient/child_process + 假定时器跑真 `extension.js`，7 例，
     零依赖毫秒级），接入 `npm run test:unit`；对着**修复前**的代码 5/7 会红
     （证据）。`crates/cli/tests/extension.rs` 新增契约守住这四个 Node 文件都在册。
4. **项目层功能测试**：新增 `crates/cli/tests/project_features.rs`（11 例：两级嵌套
   模块名、菱形依赖 + 缓存失效、两个入口共享依赖且缓存不串台、依赖解析错误归因、
   文件/目录同名、import 位置与形态错误、`build` 逐文件状态、嵌套项目的
   `query` 计数一致、入口拒绝退出 1 而依赖 `sorry` 退出 0）。
5. **顺手修掉的缺陷**：`import my-lib` 的报错文案把横线写了两次（`my--…` →
   `my-…`，front token 层 + 单测）；`docs/protocol.md` 的人类输出口径写成
   `error[<code>]`，实际是 `error[<stage>]`；设计 §6 A2 承诺的"依赖 `decl.checked`
   事件带 module"与实现不符——按实现改口径（只输出入口事件，依赖的问题走诊断，
   §5.1 偏差④）。
6. **测试与门禁**：`cargo test --workspace --locked` 全绿（front 450+ / LSP 130+ /
   CLI 200+，含新增 4 个 perf 例、11 个功能例、7 个宿主例）；
   `node editor/vscode/test-extension-host.js` 7/7；`scripts/soko gate` PASS。
7. **用户第二轮追加：真跑一遍性能 + 教学内容 import 化**。性能：`scripts/perf-ledger.sh`
   11 条记录（front 4×20 compile 111ms / 按键 130ms；缩放 2.8×；LSP 项目按键 46ms
   且 1 份诊断；CLI release 冷 31.9 / 热 3.3 / 依赖改动后 27.7ms；新增"判据前缀"
   一条：入口 10 处 `match` 导入归纳类型 82.6ms）。教学内容：
   - 先量了一遍：44 个语料文件里**逐字重复**的声明块只有 232/2974 行（8%），
     And 公理 24 份、`Or` 块 12 份、显式 `Nat` 块 8 份（含中英与解答钥匙）。
     结论：**整包 import 化不划算**（会打破"单元自给自足"、golden/镜像/课程树契约
     全要重钉），但复制粘贴的漂移风险是真的。
   - 于是新增 **`course/shared/` 子项目**（`sokonanoda.toml` + 规范模块
     `And`/`Or`/`Nat` + 自检入口 `Demo.sokonanoda`，真的 import 并判卷），
     配 `crates/cli/tests/course_shared.rs` 的**双向漂移守护**（少了=副本没跟上、
     多了=抄了没登记、画布出现 `import` 也红）。画布一行未改，golden 零漂移。
   - **过程中挖出并修掉两个真 bug（同一根因）**：`match` 的宇宙层级、`by` tactic 的
     `apply`/`exact` 都靠 `judge_infer(prefix_src, …)` 合成前缀文件问内核，而项目
     模式的前缀只含**入口自己**的源码 ⇒ 入口里 `match` 被导入的归纳类型报
     `elab-match-no-expected-type`、`by apply And.intro`（导入的公理）报
     `elab-tactic-failed: unknown identifier`。修法：`run_pass` 按拓扑序预计算
     `closure_prefixes`（依赖源码去掉 `import` 行后相接 + 本文件前缀），
     单文件模式不构造（A1 逐字节不变）。两条回归测试入 `project/tests.rs`。
   - **发现但未修（已登记，P5 余项）**：编辑器 quick-fix 的 `front::suggest` 也只吃
     入口文本 ⇒ 项目入口里对导入名字给不出建议（同一文件放进单文件就有
     `refine And.intro …`，放进项目入口是 `null`；真 LSP 探针复现）。
     记在 `docs/TESTING.md` §7b 与 `docs/design/imports-and-projects.md` P7。
8. **用户第三轮提问：单文件与项目文件能自动区分吗（单文件不找项目配置、像脚本一样跑）**
   ——是，规则写进设计文档 **§4.4b** 并由 `crates/cli/tests/single_file_vs_project.rs`
   四条测试钉住：① 无 `import` 的文件**从不读 `sokonanoda.toml`**（同目录坏清单、
   `--root`、`--no-project` 全是空操作）；② 同一个坏清单在有 `import` 的文件上必须报
   `manifest-invalid`，但仍以入口目录把闭包编完；③ **依赖自己的清单永不参与**；
   ④ 零配置能 import、stdin 与文件逐字节一致、stdin 带 `import` 给出 `--root` 提示。
   **同时修掉一个真 bug（编辑器与 CLI 不一致）**：LSP 把 `initialize` 的**工作区根**
   当模块根传下去（等于跳过清单发现），于是 VS Code 打开仓库根、再打开
   `course/unit11-project/Canvas.sokonanoda` 会报 `import-not-found`，而同一文件在
   CLI 下正常。现在编辑器与 CLI 同一套发现规则（最近清单 → 入口目录），
   回归测试 `a_nested_project_resolves_against_its_own_manifest`。
