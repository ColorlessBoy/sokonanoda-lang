# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-18（第九十三轮：项目层性能例行化 + 测试扩充 + 编辑器审计修复；版本 **0.57.0**，P0–P6 完成）
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

## 本轮进度（2026-09-18，第九十二轮：I16 落地 —— `import` 闭包 + 项目管理，0.57.0）

> 用户：「新产生一个 git 分支吧，全部按照建议，你给我完整做完一版我看看。这个变化比较大。」
> 分支 **`i16-imports-and-projects`**；设计文档 §8 的 Q1–Q7 **全部按推荐执行**；
> P0–P6 全部落地（P7 = backlog）。设计 + as-built =
> **`docs/design/imports-and-projects.md`**（§5.1 有三处与设计的偏差与 P5 的实现选择）。

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
5. **LSP（P5，全做完）**：`Docs{map,order,root,active}` 多文档、`initialize` 捕获
   root、按 URI publish、`did_close` 清理、跨文件 `goto_definition` / `references` /
   `rename`（新 `project_refs.rs`：跨文件身份 = 名字、定义名 token 来自
   `front::references`、编辑按模块分组、改名成项目里已有名字先被拦下）；项目模式下
   补挂 `-- soko:hint` 阶梯（否则带 import 的入口答不出 hints）。**跨文件失效**：
   改依赖 ⇒ 含它的打开文档用"内存覆盖"（`load_closure_with_overlay`，按
   `canonicalize` 匹配）重编译重发，**未落盘的依赖编辑也可见**；诊断只在真的变化时
   才 publish（`Doc::published`）。第一版"挂住"的根因是**测试写法**（一次通知连发
   多条诊断时先等通知再读 socket ⇒ 死锁），修法是 `testutil::notify_with_drain`——
   教训写进 `docs/TESTING.md` §5.7 与架构 §8.10。`crates/lsp/src/tests/project.rs`
   8 条 e2e + `project_refs.rs` 2 条单测；真实二进制探针复核过引用/改名/依赖失效。
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
9. **LSP 侧只剩 P7 项**：`didChangeWatchedFiles`（编辑器**外**改文件不触发刷新，
   要重开文件）、`soko/project`、跨文件改名的"重命名文件/模块"形态；`watch` 项目
   模式、`[deps]`、`namespace`/`open` 同样留 P7。
10. **新增一笔结构债（已登记，不静默）**：`crates/front/src/compile/check.rs`
   1717 → **1918** 行（`run_pass` 单函数 ≈1174 行）——多 unit 泛化加在这里但没趁机
   拆函数（拆它要独立一轮，事件流/增量语义不能漂）。计划与验收见
   `docs/HANDOVER.md` §4 与 `docs/design/imports-and-projects.md` P7。

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
