# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-18（第九十七轮：真 VS Code 集成测试例行化 + 结果台账；版本 **0.58.0**）
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

## 本轮进度（2026-09-18，第九十七轮：真 VS Code 集成测试例行化 + 结果台账）

> 用户：「你配置相关套件，启动 VSCode 实际验证一下，本来就应该做成例行化检测。
> 远程不行，本地例行化也可以接收。」——原来只有 `cd editor/vscode && npm test`
> 这条"想起来才跑"的手工路径，且极易测到旧二进制；本轮把它做成**一条命令 +
> 提交进仓库的台账**，并用它真跑了一遍。

1. **一条命令**：`SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh` ——
   构建 release（被测的就是发布形态）→ `node scripts/stage-lsp.js` stage 到
   `bin/<target>/` → `npm test`（真 VS Code + 真 LSP + 真扩展宿主）→ 记账。
   退出码 0/1/2/3（全绿/用例失败/用法/前置缺失）。
2. **台账（提交进仓库）**：`docs/e2e/ledger.jsonl`（`schema: soko.e2e/1`：
   version/commit/dirty/host/VS Code 版本/`tests{passed,failed,pending}`/exit/
   doctor 的服务器版本行/被测 LSP `sha256` 前 16 位/裁剪日志路径）+
   `docs/e2e/latest.json` + `docs/e2e/logs/<date>-<sha>.log`（doctor 块 + 用例
   清单 + 扩展接线日志 + 失败详情）。
3. **本轮实测（真宿主）**：**14/14 全绿**，用时 ~1s（外加 VS Code 启动与首次
   下载）；doctor 自述 `0.58.0 (pid …) == 扩展 v0.58.0 (source=bundled)`。
   新增 4 条用例：`.sokonanoda` 语言 id 守卫 + **项目树三条**（真 `soko/project`
   答案渲染的行：闭包 / 单文件占位 / 缺模块根因与错误图标）。
4. **过程里修掉三个真问题**：
   - `.vscode-test.mjs` 把 `--user-data-dir`/`--extensions-dir` 指到
     `<tmpdir>/soko-vscode-test`：macOS 的 unix socket 路径上限 103 字符，本仓库
     的长路径原先直接 `EINVAL` 起不来（老文档让你把扩展拷到 `/tmp/v`，现在不必）；
   - 扩展的 test-mode 返回钩子**提前 `return` 掐掉了 `client.start()`**——测试宿主里
     服务器永不启动，10 个用例集体超时（stub 层看不见这类生命周期问题）；改成
     **函数末尾**返回并写清为什么；
   - 新增 `SOKO_E2E_LOG` 文件日志（env 开关、生产零成本）：扩展宿主的 `console`
     在 `vscode-test` 输出里取不到，这条日志是 e2e 卡住时的第一现场（本轮正是靠它
     定位到上面那条）。
5. **文档**：新增 **`docs/E2E.md`**（一条命令、四层分工、台账字段、判读口径、
   环境坑、与 CI 的关系）；`docs/vscode-dev-guide.md`（测试三层 + 坑 19/20/21 +
   坑 14 更新为"配置已自解"）、`docs/TESTING.md` 集成测试小节、`AGENTS.md`
   （命令 + 扩展改动后的例行三层）、`skills/sokonanoda-dev`、`docs/README.md`
   地图、`docs/LESSONS.md`（"给扩展一条文件日志"）同轮同步。
6. **CI 也跑这条命令（0.58.0 同日）**：新增独立 **`e2e` job**（矩阵
   `ubuntu-latest` + `xvfb-run` 与 `macos-latest`，各自钉 VS Code 版本），跑的就是
   `scripts/vscode-e2e.sh`；`docs/e2e/` 上传为 artifact，`scripts/e2e-summary.py`
   的渲染写进 **job summary**；`auto-tag` 的 `needs` 加上 `e2e` ⇒ **e2e 红了不发版**。
   原来 `test` job 里那条 `xvfb-run npm test` 删除（避免同一套用例跑两遍）。
7. **CI 的 macOS 腿只在 push 到 main 时跑**（用户定：PR/分支只跑 Ubuntu，快反馈；
   main 上才加跑 macOS——真宿主差异值得守，但每个 PR 多 ~10 分钟不划算）。
   e2e job 用 job 级 `if`（`matrix.os != 'macos-latest' || push && main`），
   被跳过的腿不影响 `auto-tag` 的 `needs`。`run:` 块逐个过 `bash -n`，
   YAML 解析校验通过（GH Actions 本身推不了，没法在这里真跑）。
   最低版本 1.106.0 腿按用户规矩**先本地验证再进 CI**：本机到
   `update.code.visualstudio.com` 反复 `Recv failure: Connection reset`（curl 也断，
   87M/147M 处），暂缓；预置缓存的绕法已写进 `docs/E2E.md` §6。
8. **版本策略（调研后决定）**：`@vscode/test-cli` 的 `version` **默认 stable 频道**
   （[官方文档](https://code.visualstudio.com/api/working-with-extensions/testing-extension)
   与官方 sample 都不钉具体版本，示例用的是 `insiders`）——生态惯例是跟频道。但这一层
   的产物是**台账**：宿主随 stable 漂就没法比历史，所以本地例行默认钉 **1.138.0**
   （`--version stable` 可跟随），CI 矩阵显式给版本；升级流程与"最低版本
   （`engines.vscode ^1.106.0`）腿待补"写在 `docs/E2E.md` §5。
9. **顺手清掉不再需要的缓存**：`editor/vscode/.vscode-test/vscode-darwin-arm64-1.137.0`
   （898MB，已换 1.138.0）、旧 VSIX×5（0.18/0.19/0.20 + universal + `sokonanoda.vsix`，
   `docs/RELEASE.md` 的发布流程会重新产出）、`.ruff_cache/`（仓库没有 ruff 配置）。

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
