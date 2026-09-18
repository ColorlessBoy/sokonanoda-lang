# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-18（第九十八轮：合并 0.56.2 线（多余的 `sorry`）+ push 主线；
> 版本 **0.58.0** —— 两条并行线已合并：本线 I16 项目管理 + 0.56.2 的
> `redundant-sorry`，`v0.56.2` 的功能与 tag 都在历史里）
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

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-18，第九十八轮：合并 0.56.2 线（多余的 `sorry`）+ push 主线 —— 0.58.0 发布）

> 用户：「你来搞吧，push」——本线（I16 项目管理 → 0.58.0）与 `origin/main`
> （0.56.2 = `redundant-sorry` 线）已经**分叉**：远端 3 个 commit（`dd1902d`
> release 0.56.2 / `f66fee3` 0.56.2 发布文档 / `74cdbda` CI skill），本线 53 个。
> 不能强推（会丢掉 0.56.2 的功能与已发布的 `v0.56.2`），所以**先合并再 push**。

1. **合并过程（scratch worktree，19 个文件冲突逐个手心合并）**：`git worktree add
   /tmp/soko-merge` 从本线 HEAD 建 `i16-merge`，`git merge origin/main`；合并树验证
   全绿后再 `merge --ff-only` 回本线（主线保持线性、无 merge 提交噪音）。
   - **内核（冻结快照）**：0.56.2 只加不改语义（`check_declar_at` /
     `try_check_declar_at` 显式限界入口），`crates/kernel/{tc,util}.rs` 取远端；
     三层回归测试一并并入（`memory_api` 7 → 8）。
   - **前端**：`open_goal` 增第 4 个参数（候选"多余洞" sink，`Vec<Span>`）；
     `PendingOp::OpenExercise` 增 `env_before`（探针终审的可见前缀）+
     `redundant_probes`（pass 1 造、pass 2 查、**不入环境**）。
   - **搬进模块化的树**：0.56.2 写在旧 `check.rs`（1918 行）里的探针代码要手工搬到
     本线的 `check/{walk,kernel_phase}.rs`：`build_redundant_probes` → `walk.rs`，
     终审循环（`try_check_declar_at(…, EnvLimit::ByIndex(env_before))`）+
     warning 装配 → `kernel_phase.rs`；旧 `check.rs` 在合并树里 `git rm`，
     `Cargo.lock` 取本线后 `cargo metadata` 重生成。
2. **合并暴露的一处真 bug（已修 + 已加回归）—— warning 的跨模块归因**：
   `redundant-sorry` 是 pass 2 **现算**的 warning（带命令下标就有归因依据），
   而 `split_report` 原先只按单元**重算语法级** warning ⇒ 项目入口里"多写了一行
   `sorry`"会被静默丢掉（单文件看不出来）。修法：`CompileOutput` 增平行数组
   `warning_cmds` + `push_warning(cmd, w)`（与 `event_cmds` / `error_cmds` 同款
   不变量：两数组严格平行、永不失配），`split_report` 按**命令下标**归因
   （不猜 span——不同文件的 offset 不在同一个坐标空间）；语法级 warning 由
   `kernel_phase` 钉在所属单元的区间上。回归：
   `compile::tests::warnings_are_attributed_to_the_unit_that_produced_them`
   （依赖 2 条 = 语法级 + 内核终审、入口 1 条，span 各落在自己文件的坐标里）。
3. **验收（合并树上真跑）**：`cargo test --workspace --locked` **888 passed /
   0 failed / 6 ignored**（27 个测试目标 + 3 个 doc-test 目标；kernel 52 =
   lib 43 + arena 1 + memory_api 8；front 475 = lib 466 + perf 3 + perf_project 6；
   cli 223 = 单元 5 + 集成 17 个目标 218；lsp 138）；`cargo fmt` / clippy 干净。
   四个纯 Node 套件 18 / 7 / 10 / 11；真 VS Code 例行化（`scripts/vscode-e2e.sh`，
   1.138.0 与 1.106.0 各一轮）**14/14**；`scripts/e2e-merge.py --check`、
   `scripts/check-site.py`、`scripts/soko gate` 全绿。0.56.2 的功能在合并树上逐条
   复验：`redundant-sorry` 正例 / 真缺口反例 / 前瞻引用护栏（front 5 条）、
   `--json` warning 事件、`query goals|holes` 的洞级 `redundant` 标记、
   LSP 不再叠 "not yet solved"。
4. **push 前的 CI 预检（发现并修掉一个 workflow 设计错误）**：原先把三条 e2e 腿
   放在**同一个 job 的矩阵**里、用 job 级 `if: matrix.os != 'macos-latest' || …`
   表达"macOS 只在 main 上跑"——但 GitHub 的 contexts 可用性表里
   `jobs.<job_id>.if` **不含 `matrix`**，这个条件要么按空值求值（macOS 腿在 PR 上也
   跑），要么被判成未识别命名值让**整个 workflow 校验失败**（那样一条 CI 都不会跑）。
   现在拆成两个 job：`e2e`（ubuntu × 2 版本，每个 PR/分支 push）+
   `e2e-macos`（macos × 1.138.0，github-only 条件、只 main）；`auto-tag` 的 needs 与
   `e2e-ledger` 的 needs 同步带上两条。顺带加固 `scripts/e2e-merge.py` 的去重键
   （加 `host.system`/`machine`：ubuntu 与 macos 的 1.138.0 腿同秒完成时不会被当成
   重复条目丢掉）。合并树先推一个**临时预检分支**跑一遍 CI（workflow 校验 +
   ubuntu 两条腿 + 全部其它 job），绿了再 push main——**这一步立刻回本**：
   - 预检确认 workflow 被接受（job 级 `if` 引用 `matrix` 的写法确实不能用），
     `e2e-macos` 在分支 push 上如预期 **skipped**，两条 ubuntu e2e 腿
     （1.138.0 与 1.106.0，含 runner 上现下老版本 VS Code）**全绿**，
     `e2e-ledger` 也如预期只在 main 跑；
   - 但 `test` job 假红：`crates/front/tests/perf.rs` 的
     `check_document_scaling_is_linear` 报 ratio ≥ 12×，而同一棵树本地全量
     `888 passed / 0 failed`。本地复现定位：**并行**（cargo 默认）跑三个 perf 用例时
     400/50 比 = 10.9×，`--test-threads=1` 或单跑该用例 = 7.8×（8× 规模 ⇒ 线性）
     ——算法没回归，是同一个测试二进制里的重活互相抢 CPU 把长的那一档抬高了。
     修法（阈值不动，只改采样口径）：`front/tests/perf.rs` 加**进程内互斥锁串行** +
     **轮转 best-of-N 取最小**，每键延迟改用**中位数 + 最坏值天花板**；
     `lsp/src/tests/perf.rs` 的单文件/项目请求延迟改 **best-of-3**（那 130+ 用例
     并行的 lib 二进制里，10ms 阈值单次采样迟早会红）。台账
     `docs/CI-FAILURES.md`（2026-09-18 条）+ `docs/PERF.md` 采样口径段同步。
5. **发布**：push `main` → `ci.yml` 的 auto-tag 打 `v0.58.0` 并 dispatch
   `release.yml`（8 平台 CLI/LSP tarball + 9 个 VSIX）。`v0.56.2` 的 tag 与其
   功能都保留在历史里，0.58.0 的 CHANGELOG 补记"多余的 `sorry` 已并入"。
6. **发布结果（2026-09-18 实测）**：push `main` → CI **8/8 job 全绿**
   （lint / test / e2e ubuntu×2 / **e2e macos-latest 第一次真跑** / e2e-ledger /
   auto-tag / pages）→ auto-tag 打 **`v0.58.0`** 并 dispatch `release` →
   release **11 job 全 success** → Release **26 资产**（lsp ×8 / cli ×8 / vsix ×9 /
   `SHA256SUMS`）+ Marketplace 收录 **0.58.0**（10:21Z）。**发布产物实测**：
   下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz` → `shasum -c` **OK** →
   `--version` = 0.58.0 → 对 `playground.sokonanoda` 报出
   `warning[redundant-sorry]`（第 328 行，正是用户最初报的那一行）+
   `query project` 在 `course/unit11-project/` 上给出根与两个 `compiled` 模块。
   `e2e-ledger` 把三条 CI 腿的台账（Linux×2 + Darwin×1，各 14/14、`dirty=false`）
   自动回提交进 `docs/e2e/ledger.jsonl`（共 10 条）；官网进度页已换到第九十八轮。
7. **文档**：本文件（第九十五轮移入归档 + 0.56.2 线的第九十一轮续一并归档）、
   `docs/STATUS-ARCHIVE.md`、`REQUIREMENTS.md` §9（九十八）、`docs/HANDOVER.md`、
   `docs/TESTING.md`（合并后的测试构成）、`docs/LESSONS.md`、
   `editor/vscode/CHANGELOG.md`、`docs/protocol.md`（warning 码三个并列）、
   `skills/sokonanoda-teacher/references/events.md`、
   `docs/design/deepseek-harness.md`（H3 追加行）、`docs/E2E.md` §5/§7（两个 e2e job 与
   版本升级三处）、`.github/workflows/ci.yml` 注释、`docs/CI-FAILURES.md`、
   `docs/PERF.md`（采样口径）、`scripts/gen-site-data.py`（轮次头解析改成"只认第一条
   + 解析失败即报错"——本轮标题里的全角括号曾让网站 round 静默停在 97）。

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
   最低版本 1.106.0 腿按用户规矩**先本地验证再进 CI**——同一天用
   `npm_config_https_proxy=http://127.0.0.1:7890` 验过：**VS Code 1.106.0 上
   14/14 全绿**（含项目树三条），台账 `b0bcba3`；随后把它加进 CI 矩阵
   （ubuntu × 1.106.0，每个 PR 都跑）。顺带把"test-electron 只认
   `npm_config_proxy`/`npm_config_https_proxy`、不读 `HTTPS_PROXY`"写进
   `docs/E2E.md` §5/§6。
8. **CI 台账回提交（用户：「验证好就让 CI 往仓库追加吧」）**：新增收尾 job
   `e2e-ledger`（只 main，`contents: write`）——下载各腿 artifact →
   `scripts/e2e-merge.py` 合并（**幂等**：重复条目跳过、日志按记录名回填、
   台账按 date 排序）→ 一条提交推回 main（标题带各腿结果）。为什么不是每条腿各推：
   矩阵并发改同一个 `ledger.jsonl` 会互相覆盖；push 前 rebase 重试一次，
   两次都失败就报错（不静默）；`GITHUB_TOKEN` 推的提交不再触发 workflow（不自激）；
   `e2e-ledger` **不**进 `auto-tag` 的 needs（免得与它自己推的提交互相等待）。
   日志文件名同时改成带版本（`<date>-<sha>-vc<version>.log`），否则矩阵里同一天
   同一 commit 的多个版本会互相覆盖。合并逻辑在本地用**伪造 artifact** 验过：
   追加 2 条 → 再合并 0 条（幂等）→ `--check` 排序/唯一/日志齐全。
   加固（同日）：`concurrency: e2e-ledger`（同一时刻只有一个写台账的 job）+
   `fetch-depth: 0`（浅克隆 rebase 缺 parent）+ push 重试 3 次、冲突时报出
   `UU` 文件并 abort。**两条路径都用临时 bare remote + 两个 clone 演练过**：
   ① 抢占 push → rebase → 第二次成功；② 同一文件冲突 → abort + 退出码 1、
   工作区干净（重跑即可）。
9. **版本策略（调研后决定）**：`@vscode/test-cli` 的 `version` **默认 stable 频道**
   （[官方文档](https://code.visualstudio.com/api/working-with-extensions/testing-extension)
   与官方 sample 都不钉具体版本，示例用的是 `insiders`）——生态惯例是跟频道。但这一层
   的产物是**台账**：宿主随 stable 漂就没法比历史，所以本地例行默认钉 **1.138.0**
   （`--version stable` 可跟随），CI 矩阵显式给版本；升级流程与"最低版本
   （`engines.vscode ^1.106.0`）腿待补"写在 `docs/E2E.md` §5。
10. **顺手清掉不再需要的缓存**：`editor/vscode/.vscode-test/vscode-darwin-arm64-1.137.0`
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
