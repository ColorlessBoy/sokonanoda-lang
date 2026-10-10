## 2026-10-10 · 全量 e2e 里 `P7：…按 F12…` 判红 = **宿主窗口没有 OS 焦点**（本机全量跑，不是产品缺陷）

**症状**：该用例单跑 3 次全绿（770ms / 886ms），**全套跑**（`scripts/vscode-e2e.sh`，48 passed / 1 failed）超时 30s；报错里 `activeTextEditor` 正是夹具、可见编辑器多一个 `extension-output-…-sokonanoda doctor`。
**真因（诊断读数，不是猜）**：`vscode.window.state.focused = false` —— **VS Code 窗口本身没有 OS 焦点**；此时连 `cursorRight` 都不移动光标 ⇒ `editor.action.revealDefinition`（`when` 含 `editorTextFocus`）**静默不动** ✗。同一刻 `vscode.executeDefinitionProvider` **照常答插件目录那份** ✓ ⇒ 解析那一半没坏，坏的是"按键送不送得到"。
**为什么单跑绿**：本机 21:0x 那三次跑在测试窗口恰好是前台时 —— 真按键那支实测落点 = `extensionPath/docs/tactics/<kw>.md` ✓，反向验证 (b)（删 `SOKONANODA_DOCS_DIR`）也是**真按键**落到物化副本才判红 ✓；而开发机上用户自己的 VS Code 常年占前台 ⇒ 结果取决于运行时刻的桌面焦点 ✗。
**修法（改判据，不是加大超时 ✗）**：两半分开断言 —— ①解析（provider ⇒ 必须答插件自带那份，硬判据）；②导航：先按真 F12，**窗口有焦点却不开 ⇒ 照样判红** ✗；只有 `state.focused === false`（有实测证据说明按键送不到）才退到"打开解析出来的落点并断言它是可见编辑器"，并打一行日志指向本条目。
**预防**：拿 `editor.action.*` 当判据的 e2e，必须写明并处理"宿主窗口焦点"这个前置 —— 它与产品行为无关，却能让整条用例静默失效；`activeTextEditor` 有值**不代表**编辑器有焦点（它是"最近聚焦过的编辑器" ✗）。

## 2026-10-08 · `test (sokonanoda-lsp, tests)` 判红 = **trace 基线的 stderr 读线程竞态**（run `37795904518`）—— 判据改取"落定值" ✓

**症状** ✓：`lsp_keystroke_structure::changing_a_statement_must_invalidate_the_prefixes_after_it`
判红在 `crates/lsp/tests/lsp_keystroke_structure.rs:106` —— `assert_eq!(after, before + 1)`
实测 **left: 2, right: 1**（= `before` 读到 **0**、`after` 读到 **2** ✗）。`fast-fail` 掐掉整轮
⇒ run 结论 `cancelled`、`auto-tag` skip ✓（**不是**被新推顶掉 ✓）。

**先排除"是我们的回归"** ✓（这一步别省 ✗）：**同 commit** 本机跑该用例 **6/6 绿**（单用例）
+ **3/3 绿**（整文件 3 个用例并行）⇒ 复现不出 ✓；`scripts/soko gate` 的 `cargo test --workspace --locked` 也是 **exit 0** ✓。

**真因** ✓：`open()`/`did_change()` 返回 = **那一版诊断**到了（走 **stdout** ✓），**不保证**
`LSP_TRACE` 行（走 **stderr**、由**另一个线程** `lines()` 读）已经进 `Vec` ✗ —— 两条管道
之间**没有顺序保证** ✓（这条竞态在本仓早有记载：「stderr 由另一个线程读 ⇒ 诊断到了不等于 那一行已经收到」，`wait_for_trace_after` 就是为它写的 ✓；**但基线那一侧**一直直接读
`trace_len()` ✗）。CI 上开档那一趟是**冷编**（checkout 里没有模块根产物）⇒ 它有一行； 读基线时那一行还在读线程手里 ⇒ `before=0` ⇒ 按键那一趟到了就变 **2** ✗。

**为什么本机不复现** ✓（**夹具在不同机器上走了两条不同的路** ✗✓）：本机
`courses/set-theory/.sokonanoda/` **有**产物 ⇒ 开档走**产物命中**、那一趟**不产生** trace 行
（本机实测：整段输出里只有按键那一行 `v2` ✓）⇒ `before=0` 本来就对、`after=1` ✓； CI 的干净 checkout 走冷编 ⇒ 多一行 ✗。

**修复** ✓：新增 `Client::settled_compile_count()`（连续 **500ms** 没有新行才算"落定" ✓，
带 180s 上限、超时**大声判红** ✗）—— 基线一律取落定值；`lsp_keystroke_structure.rs` 的三处
读数（statement + proof-body 两条）与 `lsp_checkpoint_multi_slot.rs::modules_after` **同轮
一次改齐** ✓（AGENTS.md：同类问题横向排查，不许修单点 ✓）；顺带把三条用例的缓存目录从 `…-structure-<pid>`（**同一个 pid ⇒ 同一个目录**，而三条是**并行**跑的 ⇒ 互相
`remove_dir_all` ✗）改成**每条一个** ✓。

**预防** ✓：① **凡"取基线再比差量"的 trace 判据，基线必须取落定值** ✗✓ —— 直接
`trace_len()` 只在"开档一定不编"时才对，而那取决于**机器上有没有产物** ✗； ② 夹具要**显式**选一条路（冷 or 热），别让"机器状态"决定走哪条（本轮就是这么被咬的 ✓）；
③ 同文件内并行的用例**不许共用**缓存目录/产物目录 ✓。

## 2026-10-08 · `test (sokonanoda-lsp, lib)` 红一次 = **墙钟比值臂在 4 核 runner 上的假红**（run `37694095005`）—— rerun 转绿 ✓ · 阈值按纪律加宽 ✓

**症状** ✓：`perf_course::perf_course_did_open_is_recorded` 判红，卡在**形状臂**
`ratio = slowest/first < 12` —— CI 实测 unit01 **1452ms** / unit08 **2559ms** / unit12 **21496ms**
⇒ 比值 **14.8** ✗（绝对哨兵 `slowest < 300_000` **是过的** ✓）。`fast-fail` 事后掐掉整轮
⇒ run 结论 `cancelled`、`auto-tag` skip ✓（**不是**被新推顶掉 ✓）。

**先排除「是我们改慢了」** ✓（这一步别省 ✗）：**同 commit** 本机 release 构建量到
unit01 **726ms** / unit08 **1373ms** / unit12 **3612ms** ⇒ 比值 **4.98** ✓（注释里的本机基线 ~4.5）；
**且本机完整 `cargo test --workspace`（172 条并行）也过** ✓、`scripts/soko gate` **exit 0** ✓
⇒ 不是判定/编译路径的回归 ✓。

**真因** ✓（与 2026-09-28 那条同族，但这次红的是**另一条臂**）：比值臂的前提
「同 run 自比 ⇒ 与机器无关」**不成立** ✗ —— 两种情形：① 4 核 runner 上的**不对称争用** （大编译 unit12 与其余 171 条用例的突发重叠，小编译 unit01 撞在安静窗口）；
② **缓存冷热不同**（同 binary 里别的用例先预热了 unit01 的库闭包 ⇒ 分母异常小：
CI 的 unit01 只比本机慢 **2×**，unit12 却慢 **5.9×**）。历史读数同向：正常 CI ~4.1 ·
3× 慢 runner **10.4**（run `36347147870`）· 本轮 **14.8** ⇒ 12 的余量本来就不足 ✗。

**处置** ✓：**rerun 一次**（值守口径 ✓）⇒ 该 job **转绿** ✓、整轮 `completed success`
⇒ `auto-tag` 打 `v0.83.0` + dispatch release ✓（**先 rerun 再定性**，与 2026-09-27 e2e 那条同一手法 ✓）。

**修复** ✓：比值阈值 **12 → 25**（`crates/lsp/src/tests/perf_course.rs`，注释里写全本轮的
四条读数与两种机制 ✓）。**这不是"为数字放宽"** ✗：按 `AGENTS.md` 的 perf 纪律 「要拦墙钟 ⇒ 同机比值 **+ 宽天花板**」，而这条用例真正关心的"每次打开重编 ×N / O(n²)"
会把比值推到 **50+**；pre-P 组那批真回归的比值本来也只有 **~4.7** ⇒ 12 从来不是它们的判据 ✗。

**预防** ✓：① **能拦的用计数拦** —— 逐次数的守卫是 `scripts/check-recompile-factor.py`
（`gates-fast`，**判红** ✓、噪声免疫 ✓），墙钟这条只是**宽网** ✓；② 序列口径的读数看
`Performance report`（`scripts/perf-report.sh`，`--test-threads=1`）⇒ 不受并行争用影响 ✓；
③ 新增墙钟判据时，**天花板要按实测读数留 ≥2× 余量**（本轮 14.8 ⇒ 25 只有 1.7×， 但它是"几十才算事故"的量级网，不是精度判据 ✓）。

## 2026-10-06 · `lint-clippy` 红 ⇒ `auto-tag` skip ⇒ **v0.82.0 发不出去**（run `37421086685`）—— 根因 = **本地/CI 工具链漂移** ✗

**症状** ✓：发版那一推（`3f853a13`，批次只推一次 ✓）结论是 **`cancelled`** ✗ —— 逐 job 取证：
`lint-clippy` **failure**（29s）⇒ `test` / `gates-course` / `e2e` / **`auto-tag` 全 skip** ✓
⇒ tag `v0.82.0` **不存在**、release 未触发 ✓（`fast-fail` 事后掐掉整轮 ⇒ 结论显示 `cancelled`，
**不是**被新推顶掉 ✓ —— 与 `cancel-in-progress` 那条区分开 ✓）。

**根因** ✓（job log 读到，**不是猜** ✓）：`crates/front/src/compile/elab.rs:3283`
`.map_or_else(&slow, |hit| hit.ok())` ⇒ **clippy 1.99 把 `needless_borrows_for_generic_args` 扩到了闭包** ✓，
而 front 有 `[lints.rust] warnings = "deny"` ⇒ 一条 style lint 直接判红 ✓。

**⚠ 本条真正要记的是"本地为什么没拦住"** ✗：本机 `clippy 1.98.1`、CI `@stable` = **1.99.0** ⇒
**同一条 lint 在 1.98 不触发** ⇒「本地 `gate` PASS」**推不出**「CI clippy 绿」✗。
⇒ **clippy 判据的可信度 ≤ 本地与 CI 工具链的一致性** ✓（**同版才算数** ✓）。

**修** ✓：`&slow` ⇒ `slow`（clippy 的 machine-applicable 建议 ✓；该分支**随即 `return`** ⇒
条件移动不与后面两处 `slow()` 冲突 ✓）；**判定中性** ✓（同一闭包、同一 `Option`，不碰判定 ✓）。

**预防** ✓：① 发版前先对 `rustc --version` 与 CI 的 stable —— 不一致就先 `rustup update stable` ✓；
② 改 front/cli/lsp（三处都 `warnings = "deny"`）后，本地 clippy **必须与 CI 同版**才算数 ✓；
③ 长程：把 stable **钉版本**（`rust-toolchain.toml` 或 `@1.99.0`）⇒ 两边不再漂移 ✓（**未做**，留评）。

## 2026-09-29 · e2e (macos) 红 = **runner 网络抖动**（`index.crates.io` 解析不了），非代码

**症状**：`e2e (macos-latest · VS Code 1.138.0)` 在 run `36520269899` 判红，
`##[error]Process completed with exit code 101`。

**根因**（从 job log 读到，**不是猜**）：

``` warning: spurious network error (3 tries remaining): [6] Couldn't resolve host name
         (Could not resolve host: index.crates.io)
error: failed to get `bumpalo` as a dependency of package `sokonanoda v0.5.0`
Caused by: unable to update registry `crates-io` / download of config.json failed ```

⇒ **runner 的 DNS 解析不了 `index.crates.io`** ⇒ `cargo` 拉不到依赖 ⇒ 编译失败。
**与本次改动无关**（同一 commit 的 ubuntu e2e 与本地全绿）。

**修法**：**不改代码** —— 这类是基础设施抖动，重跑即过。
**预防**：CI 里 `cargo` 一律带 `--locked`（已有 ✓，它保证**不更新 registry 索引内容**，
但**仍需要**能解析 `index.crates.io` 才能下载 crate）⇒ 真要免抖，得预热
`Swatinem/rust-cache` 的 registry 缓存（**已有 ✓**）—— 本轮的抖动是**缓存没命中** （新 runner 冷启动）撞上 DNS 故障。
**判据**：同一 commit 的 ubuntu e2e **success** ⇒ 不是代码问题 ✓。

## 2026-09-29 · **perf-gate 判红一轮就假红** ⇒ 回退为"只报不拦"，计数守卫接棒

**症状**：撤掉 `continue-on-error`（用户 09:20 第②条）后**第一轮就 failure** ✗
（run `36520269899`），而**同一段代码本地跑同一序列 5 个 case 全 `exit 0`**（本地复现不出来）。

**根因**：**共享 CI runner 上的墙钟抖动**。同一套件实测 **本机 37.37s vs CI 278.58s（7.5×）**
（`AGENTS.md` 已记），而 `--threshold 50` 是**绝对百分比** ⇒ 挡不住这个量级的抖动。
上一轮加的"**同 runner 家族基线**"解决了**跨宿主比较**（那是对的 ✓ **保留**）， 但**没解决同一 runner 家族内部的 run-to-run 抖动** ⇒ 判红必然**假红**。

**修法**：**回退为"只报不拦"**（`continue-on-error: true` + 循环里 `|| true`），
但**不是"就不拦了"** ✗ —— **真正的拦截接棒给** `gates-fast` 的 `scripts/check-recompile-factor.py`（**测次数、不测耗时**）：

* 它**已经判红**（`gates-fast` 无 `continue-on-error`）；
* 它**噪声免疫**（`by_calls` 是**结构计数**，与机器快慢无关）；
* G-68 的正解就归它守 ✓。

⇒ **本轮得出的分工**：**能拦的用计数拦（已拦）· 拦不住的（墙钟）只报** ✓。

**预防**：**性能门禁优先用"计数"而非"墙钟"** —— 墙钟在共享 runner 上不可转移
（先例：`keystroke_recompile_closure` 的绝对毫秒判据、`perf-gate` 的 `+585%` 假红）。
要拦墙钟 ⇒ 必须**同机比值 + 宽天花板**，或**同一 run 内自比**（不是跨 run 比）。

## 2026-09-28 · run `36347147870` · `test (sokonanoda-lsp, tests)`

**现象**：`perf_course::perf_course_did_open_is_recorded` 判红，卡在**绝对量级哨兵**
`slowest < 180_000` —— CI 实测 unit01 **19208ms** / unit08 **75335ms** / unit12 **197064ms** ✗。
同轮**形状判据**（unit12/unit01 ≈ 10.4 < 12，与机器快慢无关）**是过的** ✓ ⇒ 没有 O(n²) 那类真退化 ✓。
该测试注释里的 CI 基线是 **62s** ⇒ 这个 runner 慢了 **~3×** ✗（180s 只有 2.9× 余量）。

**先排除"是我们改慢了"** ✓（这一步别省 ✗）：把当轮的两个改动（`notation.rs` 的指令行 span、
`project/graph.rs` 的闭包表 seed）**原地 stash 掉再量同一条命令** ⇒
**2356/6918/14412ms vs 2330/6894/14385ms ⇒ 差 ~0.2%** ✓（同机同负载）⇒ **不是我们的改动** ✓。

**修复**：绝对哨兵 **180s → 300s**（仍抓"退化成几分钟"的真事故 ✓）；**真正的守卫是那条比值** ✓。
**预防**：绝对阈值**天生跨机器**（`AGENTS.md` 的 perf-gate 教训：调到多大都追不上机器差 ✗）——
新增量级哨兵时**必须**同时给一条**与机器无关**的形状判据 ✓，且绝对那条只当"几分钟级"的事故网 ✓。

## 2026-09-27 · run `36340364435` · `e2e (macos-latest · VS Code 1.138.0)`

**现象**：`the Infoview receives the project view (E30)` 判红 —— `TypeError: Cannot read
properties of undefined (reading 'project')` at `src/test/extension.test.js:713`。
同轮 **ubuntu 两条腿都绿**、本机（darwin release）也绿 ⇒ 先按值守口径 rerun 一次
（`gh run rerun --job 108679507624`）⇒ **仍红** ✗ ⇒ 不是 flake，去取 artifact 定位 ✓。

**真因**：不是 macos 的问题，是**判据自己"等 A 断 B"** ✗✓ —— 那条 e2e 等的是
`infoview.lastProject()`，最后一条断言读的却是 `extensionApi.project.answer.project.root`
（**项目树**那份答案，另一条路填 ✗）。快机器上两条路几乎同时到 ⇒ 看不见 ✗；
macos runner 慢 ⇒ 树还没答 ⇒ `answer` 为 `undefined` ⇒ TypeError ✓。

**修复**：`waitFor` 的条件**同时**等两个信号（`infoview.lastProject()` **和**
`project.answer`）—— 等到的与要断言的一致 ✓。

**预防**：**"等 A 断 B"是 e2e 的经典假红，且只在慢 runner 上现形** ✗✓ ⇒ 把"后面要读的
每一个异步状态"都列进 `waitFor` 条件 ✓；**快机器绿、慢机器红的，先怀疑它，别叫 flake** ✗。
⚠ **rerun 只能证伪 flake，不能消红** ✓ —— 仍红就去取 artifact（`gh run download <run-id>`
⇒ `e2e-<os>-vscode-<ver>/logs/<date>-<sha>-*.log` 的「## 失败详情」✓；沙箱里取日志要
`XDG_CACHE_HOME=/tmp/ghcache` ✓）。

## 2026-09-27 · **T-A60-2 的 1.5s 窗口被「迟到的落盘」压红** ✗（run `36323798295` 的 `e2e (ubuntu-24.04 · VS Code 1.138.0)`）
**现象** ✓：同一 commit（`f2ec5d0`）在 **macos 1.138.0 与 ubuntu 1.106.0 都 32/32 绿** ✓，
只有 **ubuntu 1.138.0** 红 1 条 ✗：`rewriting an unchanged project unit does not recompile`
⇒ `AssertionError: 内容没变 ⇒ 不许写出新的缓存条目（说明闭包被重编了）` ✓。
**证据** ✓（从该 job 的 artifact 里取，**不等整轮** ✓）：基线之后多出两条 ——
`949171fcb4d6fa0c.json:2101` 与 `c2a5a26a2cc7006d.json:5174`；而**夹具闭包的条目是 155KB 级**
（u01 实测 `155213`）、`lib/Set` 单独编是 **26KB 级**（实测 `26184`）⇒ **这两条不是它的闭包** ✗
⇒ 是**别处的编译迟到了** ✓。
**根因** ✓：这条判据本身没错（"同一份字节重写 ⇒ 不许写新条目"），但它的 **1.5s 固定窗口**
对邻居用例迟到的落盘**没有免疫力** ✗ —— 这正是它自己注释里记着的老毛病
（`editor/vscode/src/test/extension.test.js` 的 T-U12 面 #3 那段：早先一次是"夹具写进 tmpDir
⇒ 落盘迟到 ⇒ 撞进这条窗口" ✗）。E22 起 build/rebuild 真的编**整个项目**、rebuild 真的
**清项目缓存** ⇒ 共享夹具的缓存被扰动得更多 ⇒ 慢 runner 上更容易撞上
（**相关性，不是已证的因果** ✗：同 commit 2/3 平台绿 ✓）。
**修复** ✓：**先等缓存静止、再取基线**（`waitFor` 连续两次 `cacheStamp()` 逐字节相同）——
与 T-A60-3 的治法同款（"先等稳定再断言"：幂等判据，**不赌时间** ✗）。
**判据一个字没松** ✓：基线之后仍必须**逐条相等** ✓。
**预防** ✓：凡「快照 → 等 N 秒 → 再快照」型判据，**先做静默期（quiescence）再取基线** ✗；
共享缓存（全局 + 项目根）是**跨用例的隐式状态** ⇒ 动了它的用例要负责还原/预热 ✓。

## 2026-09-27 · **改了课程计数，漏改一个测试钉子** ✗（run `36314757444` 的 `test (sokonanoda-cli, tests)`）
**现象** ✓：整轮 `failure`，逐 job 查只有 `test (sokonanoda-cli, tests)` 红 ✗ ——
`query_check_matches_grade_on_a_real_course_unit` FAILED：
`assertion left == right failed: …"decl_checked":4…`，`left: 4` / `right: 5` ✓。
**根因** ✓：E02 把 `def Set.prod` 从单元⑤ 画布收进 `lib/Prod` ⇒ 该画布 `decl_checked` **5 → 4**
（练习数/open 数没动 ✓）；而 E02 当时找计数钉子只 grep 了 `course.rs`/`course_status.rs`/`cli.rs`
**三个文件** ✗ —— **漏了 `query.rs`**（它拿真课程文件当夹具 ✗，名字里没有 "course" ✗）。
**修复** ✓（`a30ffc7`）：钉子改 4 + 原地写明"基线在 E02 变过"；顺手修 `front/src/compile/tests.rs`
一处陈旧注释（说 unit12 有"那 5 个 R5 标记"，其实已清零）。
**预防** ✓：**改了课程计数 ⇒ 全测试树 grep 计数钉子** ✓（`decl_checked`/`exercise_open` + 被改文件名），
**不许只 grep 名字里带 course 的那几个文件** ✗；并且**本地跑全套**
`cargo test -p sokonanoda-cli`（25 个 target）确认"只有这一处" ✓（实测 0 FAILED ✓）。
**监控教训（与上一轮同源）** ✗✓：我先前用 `sleep 300/420` 长轮询**只盯整轮 conclusion** ✗ ——
而 matrix 里一个 job 早挂了、整轮还在 `in_progress` ⇒ 白等几分钟 ✗。
**改成 60–90s 逐 job 看** ✓：`gh run view <id> --json jobs | jq -r '.jobs[] | "\(.conclusion // .status)\t\(.name)"'` ✓。

## 判据
本机行为**不变** ✓（本地本来就解析仓库构建 ✓）；**CI 侧由下一轮确认** ✓
—— 这是**必须推**的那类改动 ✓（本地绿 ≠ CI 绿 ✓ 本 session 已证 ✓）。

## 2026-09-25 · **用户指出：整轮的 `in_progress` 掩盖了已发生的失败** ✗（round 182 修 ✓）
**用户原话** ✓："ledger **立马就失败**了，但是**整个 github action 还在继续**，
导致**你不知道已经失败了**" ✓ —— 完全成立 ✓，而且我在旁边反复撞这堵墙 ✗：
* 实测 ✓（`#36151088821` ✓）：整轮 `in_progress` ✗，而 `ledger (3)` **15:09:04 就红了** ✗
  （`test/*` · `gates` 还在跑 ✓，要十几分钟 ✓）⇒ **失败信号被拖住了** ✗；
* **为什么我读不到细节** ✗：`gh run view --job <id> --log` 在**整轮结束前**回
  `run … is still in progress` ✓ ⇒ **日志拿不到** ✗（本 session 为此刻苦了半小时 ✓）。

### 两处修 ✓
1. **`scripts/gap.py` 写 `$GITHUB_STEP_SUMMARY`** ✓ —— 每片无论红绿都写一行结论 ✓
   （红时给出"本地复跑这一片"的命令 ✓）⇒ **页面上**一眼可见 ✓、**不必等整轮** ✓。
2. **`scripts/ci-watch.sh`** ✓ —— **按 job 看，不看整轮** ✓：
   `gh run view <id> --json jobs` ✓ ⇒ 逐 job 打印结论 ✓，**只要最新一轮有 job 红就 `exit 1`** ✓。
   **实测** ✓：对用户指的那轮 ⇒ "失败 job 数 = **3**" ⇒ **它会立刻判红** ✓✓。
3. **清队列** ✓：取消 3 个**排队中**的 run（`#36151294668` ✓ `#36151088821` ✓ `#36150927359` ✓），
   保留最新一轮 ✓（含 `ledger` 修复 ✓）。

### 教训（对"我"的 ✓）
**监控要盯最细的可判单元** ✗：整轮状态是**聚合量** ✓，聚合会把"已经确定的失败"
**稀释**成"还在跑" ✗。**这一条与判据设计同源** ✓：**别用聚合信号判断局部状态** ✗。

## ✅ **注解链路打通**（round 199 ✓）—— `ledger (2)` 的**真凶已指名** ✓
**监控在整轮未结束时**（15:36 ✓，整轮 15:31 起 ✓）就报出 ✓，并且注解直接给出缺口 ✓：
```
[failure] 缺口台账 G-23 与台账不一致
  G-23  fixed  script  环境异常  ← 台账写的是「复现件自己说环境/形状不对（exit 2）——修环境，别当成已修」
  ｜复现件：at process.processTimers (node:internal/timers:519:7)
  ｜ stderr: at listOnTimeout (node:internal/timers:581:17) / at process.processTimers (…)
```
**⇒ 根因** ✓：`G-23` 的复现件在 CI 上撞了 **Node 自己的定时器超时** ✗
（`listOnTimeout` / `processTimers` ✓ = Node 内部栈 ✓，**不是**我们的 `SOKO_GAP_REPRO_TIMEOUT` ✗）
⇒ 退出码 2 ⇒ `judge` 判"**环境异常**" ✓ ⇒ 与台账的 `fixed` 不一致 ⇒ 整片红 ✗。
**这正是 round 126 修过的同一族假红** ✓ —— 但那次修的是**我们自己的超时** ✓（响亮跳过 ✓），
而这一条走的是**复现件内部的 Node 超时** ✗ ⇒ 两条路都要覆盖 ✓。
**修法（下轮一步 ✓）**：① 给 `G-23` 的复现件**加长 Node 侧超时** ✓（或让它对慢机器更宽容 ✓）；
② 并把"`exit 2` + 栈里含 `listOnTimeout`/`processTimers`" 也**分档为环境异常** ✓
（与超时同款：**响亮跳过** ✓ + `--strict` 才判红 ✓）——**但必须配本地兜底判据** ✓，
否则会掩盖真回归 ✗（与 round 126 同一个取舍 ✓）。

**顺带确认两件事** ✓：`ledger (1)` ✅ 与 `ledger (3)` ✅ **都转绿了** ✓（原来三片全红 ✗）
⇒ **构建 + Node 两处修复都生效** ✓；`gates-fast` **2 分 09 秒** ✓（原来 3 分钟+ ✓）。

### round 201：`G-23` 假红的修复（**A 已落 ✓，B 留锚点 ⏳**）
**A ✅ 主修复** ✓：`G23-notation-navigation.js` 的 LSP 应答超时 **60 s ⇒ 180 s** ✓
（并支持 `SOKO_LSP_REPLY_TIMEOUT_MS` 覆盖 ✓）—— 这是 CI 慢 runner 上**直接**的成因 ✓。
**B ⏳ 分档（下一轮一步 ✓）**：把"`exit 2` + Node 定时器栈"归到 `timeout` 档 ✓
（响亮跳过 ✓，`--strict` 才红 ✓；**其余 `exit 2` 仍判红** ✓ ⇒ 2026-09-23 的保护不动 ✓）。
**锚点（精确到行 ✓）**：`scripts/gap.py` 的 `run_repro` 里，`.sh` 分支的
```python
           detail = _tail(proc.stdout) or _tail(proc.stderr)
           if detail and _tail(proc.stderr):
               detail = f"{detail} ｜ stderr: {_tail(proc.stderr, 2, 120)}"
           return ("script", proc.returncode, detail)
```
⇒ 在 `return` 之前插入：
```python
           if proc.returncode == 2 and any(
               k in ((proc.stderr or "") + (proc.stdout or ""))
               for k in ("listOnTimeout", "processTimers", "LSP 超时未应答")
           ):
               return ("timeout", proc.returncode, "LSP 应答超时（Node 自身定时器）⇒ 环境慢 ｜" + detail)
```
⚠ **我第一版的两处错（记下来 ✓）**：① 判断放进了 `judge()` ✗ —— 它只拿到调用方拼好的
`note` ✓、**拿不到 stderr 原件** ✗ ⇒ 永不触发 ✓；② 返回值写成**三元组** ✗（`judge()` 回两元 ✓）。
⇒ **判据要放在"原件在手"的那一层** ✓。

**本地复现判据（已验证可用 ✓✓）**：用新加的环境变量把 CI 的慢条件**造出来** ✓：
```bash
SOKO_LSP_REPLY_TIMEOUT_MS=1 bash docs/gaps/repro/G23-notation-navigation.sh   # ⇒ exit 2 + Node 栈 ✓
SOKO_LSP_REPLY_TIMEOUT_MS=1 python3 scripts/gap.py check --shard 2/3           # 期望 0（A+B 都上之后 ✓）
SOKO_LSP_REPLY_TIMEOUT_MS=1 python3 scripts/gap.py check --shard 2/3 --strict  # 期望 1 ✓
```
（**只上 A 时**：默认仍 1 ✗（因为 180 s 在 `SOKO_LSP_REPLY_TIMEOUT_MS=1` 下照样超 ✓）；
上完 B 后默认应转 **0** ✓ ⇒ 这就是 B 的判据 ✓。）

## 2026-09-25 · `0.72.0` 发版连续红三次 —— **三条教训**（都值得记住 ✓）

### ① bump 有两个隐藏依赖：`Cargo.lock` 与两个清单的 `requires`

**症状** ✓：CI 的 `gates-fast` 红在 `cargo build -q -p sokonanoda-cli --locked` ✓：
```
error: cannot update the lock file …/Cargo.lock because --locked was passed to prevent this
```
**原因** ✓：只改了 `Cargo.toml`（0.68.0 → 0.72.0）✗，而 **`Cargo.lock` 还记着 0.68.0** ✗
⇒ `--locked` 拒绝 ✓。
**修** ✓：`cargo build -q -p sokonanoda-cli --offline`（**不带 `--locked`** ✓）⇒ 锁文件跟着更新 ✓。

**而下一红又是同一类** ✗：`contract` 的第一步 `python3 scripts/bump.py --check` ✓ 报
**版本漂移** ✓：`course/shared/sokonanoda.toml` 与 `courses/set-theory/sokonanoda.toml`
的 `requires` 还是 `0.68.0` ✗。

**⇒ 根因（`vscode-dev-guide.md` 早就写了 ✓）**：
> **bump 用脚本，别手改**：`python3 scripts/bump.py <x.y.z>` 一次写全…
> **手改漏掉清单的 `requires` 就是 G-24 的成因。**

**⇒ 规程** ✓：**bump 一律 `python3 scripts/bump.py <x.y.z>`** ✓ ⇒ **然后 `--check` 复检** ✓
（它会打印"版本一致：x.y.z" ✓）。**`CHANGELOG.md` 仍然手写** ✓（脚本只管数字 ✓）。
**bump 其实是五处** ✓：`Cargo.toml` · `Cargo.lock` · `editor/vscode/package.json` ·
**两个 `sokonanoda.toml` 的 `requires`** ✓ —— **"两处"是脚本替你写全之后的表象** ✗。

### ② `auto-tag` 依赖**全部重活**，而 docs-only 轮把重活全 skip

```
auto-tag ✓：needs = [lint-fmt, lint-clippy, test, gates-fast, gates-course,
                    ledger, contract, editor, e2e, e2e-macos] ✓
```
⇒ **依赖被 skip ⇒ `auto-tag` 自己也 skip** ✓（GitHub 语义 ✓）
⇒ **纯 docs/toml 的 push 永远不会发版** ✓ —— 这是**第 g1 条的设计后果** ✓，**不是 bug** ✓，
但**发版那一次必须让重活真的跑** ✓（**即：那一次必须碰到 rust/courses 相关文件** ✓）。

### ③ **最贵的一条**：修 CI 的节奏与发版的节奏**相反** ✗

- **修 CI 时** ✓：每改一处就推 ✓ ⇒ `cancel-in-progress` **帮我省时间** ✓（掐掉旧轮 ✓）；
- **发版时** ✗：**每一推都掐掉唯一那轮 `rust == true` 的运行** ✓ ⇒
  **`auto-tag` 永远等不到"重活全绿"** ✓ ⇒ **release 永不触发** ✓。

**⇒ 规程** ✓：**发版窗口里，推一次就停手** ✓ —— 等它跑完（**别再推，哪怕发现小错** ✗；
要改就**攒着** ✓，下一批再说 ✓）。**`gh run rerun <id>` 只能重放同一棵树** ✗
⇒ **树里没有全部修复时，重跑一万次也没用** ✓（实测 ✓：重跑 `b4aca6e` ⇒ `contract` 照红 ✓）。
**⇒ 判据** ✓：发版前先 `git log --oneline origin/main..HEAD` 看清"这一推带上了什么" ✓，
再 `python3 scripts/bump.py --check` 与 `cargo build --locked` 两条本地门 ✓ ⇒ **然后才推** ✓。

## 2026-09-25 · 发版被两条**从来就红**的腿挡住 —— 而它们红了约 100 轮 ✗

**症状** ✓：`0.72.0` 的 bump 推上去后，`test` 矩阵里**三条腿红** ✗：
```
test (sokonanoda-front, doc)   → error: unknown start of token: \u{ff1a}   （全角冒号 ✗）
test (sokonanoda-cli, doc)     → error: no library targets found in package `sokonanoda-cli`
test (sokonanoda-cli, lib)     → 同上
```
**根因（两条，都不是本次改动造成的 ✗）**：
1. **`crates/front/src/judge.rs:442` 的围栏代码块没有语言标记** ✗ ——
   ` ``` ` 开头 ⇒ **rustdoc 当成 Rust 代码编译** ✓ ⇒ 而内容是**中文 + 全角冒号** ✗
   ⇒ 解析错 ✓。**修法** ✓：加语言标记（` ```text ` ✓）。
   **它从 round 233 起就红** ✗（约 **100 轮** ✓），**每一次发版都被它挡住** ✓。
2. **`sokonanoda-cli` 是纯 bin crate** ✗ ⇒ 矩阵里的 `--lib` / `--doc` 两条腿
   **永远 `no library targets`** ✗。**修法** ✓：`matrix.exclude` 删掉这两条
   （**12 条腿 → 10 条** ✓）。

**⇒ 规程（这一条最贵 ✓）**：**`auto-tag` 要的是"全绿"，不是"这次改的部分绿"** ✗
⇒ **发版会把所有旧账翻出来** ✓ —— 而**旧账可能已经红了几十上百轮** ✓，
**因为平时的 push 里它们是 skip 的** ✓（docs-only ✓）或**没人看** ✗。
⇒ **发版前先看 `test` 矩阵的**最近一次全量**结果** ✓（不是本次改动的结果 ✓）。
**⇒ 而诊断一条红腿，最省的办法是**本地跑 CI 的原命令** ✓**：
```bash
cargo test -p sokonanoda-front --doc --locked    # 失败输出第一行就写着文件名与行号 ✓
```
（**不要猜是谁改的** ✗ —— `unknown start of token: \u{ff1a}` 已经说了是**全角冒号** ✓。）

**⚠ 另一条** ✓：**`yaml.dump` 的输出不能当锚点** ✗ —— 它**重新缩进** ✓，
而文件里可能是**行内写法**（`kind: [lib, tests, doc]` ✓）⇒ **锚点只认"刚打印出来的文件原文"** ✓。

## 2026-09-25 · 新 run 长时间 `pending` 且 **0 个 job** ⇒ 并发组被**旧 run**占着 ✗

**症状** ✓：推上去后 `gh run view <新 id>` 一直 `pending/` ✓、`job 数 0` ✗ ——
而**前面的 run 都是秒起** ✓。
**根因** ✓：顶层 `concurrency: {group: ci-${{ github.ref }}, cancel-in-progress: true}` ✓
⇒ **同一组里有一个 `in_progress` 的旧 run** ✗（它**注定红** ✗ —— 树里没有修复 ✓）
⇒ **新 run 排队等它** ✓。`cancel-in-progress` 只在**新 run 真正开始时**才掐旧的 ✓，
而**它自己还没开始** ✗ ⇒ **死等** ✓。
**修** ✓：`gh run cancel <旧 id>` ✓ ⇒ **新 run 立刻起** ✓（实测 ✓：`job 数 10` ✓）。

**⇒ 规程** ✓：**新 run 长时间 `pending` + 0 job ⇒ 先查同组的旧 run** ✓
（`gh run list --limit 5` ✓）⇒ **取消注定失败的那个** ✓ ⇒ **别再推** ✗
（**推解决不了排队** ✗ —— 它只会再加一个排队的 ✓）。

## 2026-09-25 · 整轮 **success 却什么都没跑** —— `paths-filter` 看的是"**这一推的 diff**" ✗

**症状** ✓：`0.72.0` 的发版推之后，整轮 **`success`** ✓ 而**只有 3 个 job** ✗
（`changes` + `lint-fmt` + `lint-clippy` ✓）⇒ **重活与 `auto-tag` 全 `skipped`** ✗ ⇒ **没发版** ✓。
**日志原文** ✓（`changes` job ✓）：
```
Run dorny/paths-filter@v4
  **Matching files: none** ✗
  **Changes output set to []** ✗
```
**根因** ✓：**`paths-filter` 只比"这一推的 `before..after`"** ✓ ——
**不是"仓库里有什么"** ✗、**也不是"前几推带了什么"** ✗。
⇒ 我那次推的 diff **只有 `STATUS.md`** ✓（**rust 改动在**上一推**里** ✗）⇒ **过滤器报"无 rust"是对的** ✓。

**⇒ 规程（这是"发版推"的硬条件 ✓）**：**发版那一推必须自带过滤器认的路径** ✓：
```
crates/** · Cargo.toml · Cargo.lock · scripts/** · .github/workflows/**   ← rust ✓
editor/** ← editor ✓    courses/** · playground.sokonanoda ← courses ✓
```
⇒ **而"改 `.github/workflows/**` 也算 rust"** ✓ 是一条**很有用**的性质 ✓：
**修 CI 的推会自动触发重活** ✓ ⇒ **不用为了触发而造改动** ✓（**真实待办自己就是触发器** ✓）。

**⚠ 而一个"看起来很像"的错误结论** ✗：我一度推断是 **`git pull --rebase` 让 `before` 不可达** ✗
⇒ **错了** ✓（**快进推也一样** ✓）⇒ **真因是 diff 内容** ✓。
**⇒ 教训** ✓：**"这推带了什么"和"仓库里有什么"是两个问题** ✗ ——
**判据要问对**：`git diff --name-only origin/main...HEAD` ✓（**这一推的 diff** ✓）。

## 2026-09-25 · **`test` 提速：`nextest` 决策 = 不做** ✓（数字在此，不必再议 ✓）

**症状**：`test` 是全轮最长的杆 ⇒ 直觉是换 `cargo nextest` 做测试级并行。
**先读 `ci.yml:148-153`** —— 仓库早就写着不用它（两条理由都成立）：① **它不跑 doctest**，
而 `test` 矩阵有 `doc` 腿；② **输出格式不同**，而失败注解那步
`grep -E "^test .* FAILED$"` **依赖 libtest 的格式**。

**测量**：① `test` 已按 `pkg × kind` **10 片并行** ⇒ 最长 **12 分 13 秒**（不是记忆里的 19–37 分钟）；
② 瓶颈是**执行**还是**编译**：CI 上一个 161 测试的套件 **278.58s**，而**编译只 26s**；
③ 本机串行基线同一套件 **37.37s** ⇒ **机器差 7.5×**（CI 2 核 vs 本机多核 —— 与 `perf-gate`
的跨机器问题**同源**）。⇒ `nextest` 在 2 核上并行度最多 ~2 ⇒ **278.58s → ~140s**（省 ~2.3 分钟）
⇒ **低于预先判据「>3 分钟才做」** ⇒ **不做** ✓。

**⇒ 真正的杠杆**：278s 里 ~250s 是**编译** ⇒ 要提速就动**编译缓存**
（`Swatinem/rust-cache@v2` 已在）⇒ **下一步是看它的命中率**，而不是换测试跑法 ✓。

**⇒ 规程（改 `test` 之前先量三件事）**：① 实测耗时（别用记忆）；② 瓶颈在编译还是执行；
③ **仓库有没有否决过这个方案**（`ci.yml` 注释里写着）。
另加一条前置检查：**共享资源** —— `crates/lsp/src/tests/**` 的 161 个测试没有端口/临时目录/
全局单例 ⇒ 技术上可并行，但**收益要先量、代价（改注解）要先认** ✓。

## 2026-09-25 · **轮询不是工作** ✗ —— 等 CI 必须落成机制（用户点名 ✓）

**症状** ✓：第 **450–455 轮**（≥6 轮）输出**逐字相同** ✗ ⇒ 每轮只为确认"CI 还没变" ✗
⇒ **纯烧 token、纯占轮次预算** ✗（**cap 480，当时已 455+** ✗）。
**病根** ✓：**把"等"当成了"查"** ✗ —— "查"要开一轮、要喂上下文 ✗；"等"该由外部进程做 ✓。
**⇒ 规则已固化进 `AGENTS.md`**「长命令的工作方式」✓（异步兑现 · `gh run watch <id> --exit-status` 一轮顶完 ·
后台作业）⇒ **本条只留指针** ✓（原文 ⇒ `git log --all -- docs/CI-FAILURES.md` ✓）。
**判据** ✓：**等 CI 期间模型调用次数 = 0 或 1，而不是 N** ✓（反面教材：`STATUS.md` round 450–461
**12 轮全是"不变"** ✗ ⇒ 本可 0 轮 ✓）。

## 2026-09-26 · **整个 workflow 被 GitHub 拒绝：同一 step 里两个 `run:` 键** ✗

**症状** ✓：推上去的那一轮 CI **`completed/failure`、`jobs=0`、耗时 `0s`** ✗
—— **不是测试红，是 workflow 文件没通过校验** ✓。

**根因** ✓（**我的错** ✗）：我给 `ci.yml` 加"STATUS.md 瘦身 lint"那一步时，
把新的 `run:` **写进了上一个 step 的映射里** ✗ ⇒ 同一个 step 出现**两个 `run:` 键** ✗：
```yaml
      - name: 课程记法规则
        shell: bash
        run: python3 "$GITHUB_WORKSPACE/scripts/notation-lint.py"
        run: python3 "$GITHUB_WORKSPACE/scripts/status-lint.py"   # ← 同一个 step ✗
```

**为什么我没发现** ✓✓（**这条最贵** ✗）：我用 `python3 -c "yaml.safe_load(...)"` 验过 ✗ ——
**而 `yaml.safe_load` 对重复键不报错** ✗，**它静默取最后一个** ✗
⇒ 我甚至打印出"`status-lint` 出现 1 次 ✓" ✗ —— **那正是它把 `notation-lint` 顶掉了** ✗✓。

**⇒ 两个后果，第二个更糟** ✗：
1. GitHub 拒绝整个 workflow ⇒ 0 job、0 秒失败 ✗（**响亮** ✓，所以发现了 ✓）；
2. **如果 GitHub 接受它，`notation-lint` 会被悄悄关掉** ✗ ——
   **一条既有的课程门禁消失** ✗，而**本地看起来全绿** ✗✓。

**修复** ✓：拆成**独立的 step**（各自的 `- name:` ✓）。

**⇒ 新增守卫** ✓：**`scripts/ci-yml-lint.py`** ✓ —— 用**禁止重复键的严格 loader** ✓
（`yaml.safe_load` 不够用 ✗），并检查每个 job 有 steps、每步有 `run` 或 `uses` ✓。
已接进 **`scripts/ci-local.sh` 的 ⓪ 号阶段**（**最前面** ✓ —— 它是唯一能在 push 前拦住的地方 ✓）
与 **`ci.yml`** ✓。

**反向验证** ✓（**咬得住** ✓）：把重复键塞回去 ⇒
`ci-yml-lint：1 条不通过 ✗ - :518:9 YAML 不合法 ✗：重复键 \`run\`` ✓（**带精确位置** ✓）。

**预防** ✓：**改 workflow 之后，验证命令只能是 `python3 scripts/ci-yml-lint.py`** ✓ ——
**不要再用 `yaml.safe_load` 当门** ✗（**它连重复键都不报** ✗）。

## 2026-09-26 · **`status-lint` 挂在过滤过的 job 里 ⇒ 只改 `STATUS.md` 时它根本不跑** ✗

**症状** ✓：我把 `status-lint.py` 接进 `gates-fast` ✗ ⇒ 而 `gates-fast` 由 **`rust` 过滤器**驱动 ✓
⇒ **只改 `STATUS.md` 的推送不碰 rust 路径** ✗ ⇒ **`gates-fast` 被 `skipped`** ✗
⇒ **lint 永远不跑** ✗（**实测** ✓：`675491b` 只改 `docs/` + `STATUS.md` ⇒
那一轮 **15 个 job 里只有 3 个绿、其余全 `skipped`** ✓，含 `gates-fast` ✓）。

**⇒ 为什么这条最讽刺** ✗✓：**"只改 `STATUS.md`"恰恰是最常发生的情况** ✓
—— 每轮收尾都要改它 ✓ ⇒ **我加的那条守卫，在最需要它的场景下不跑** ✗
（**"咬不住的守卫等于没有"** ✓ 的又一个变体：**"不跑的守卫等于没有"** ✗）。

**修复** ✓：**独立的 `status-lint` job** ✓，**不设 `if:`、不加过滤器** ✓ ——
它只要 **~1 秒** ✓，所以**永远跑** ✓（**顺带避开"skipped 的依赖会拖垮 `auto-tag`"** ✗，
见 `changes` job 的注释 ✓）。同时**从 `gates-fast` 里撤掉那一步** ✓（**避免两处重复** ✓）。

**⇒ 顺带暴露的第二条** ✓：**`675491b` 那一轮 `success` 是"什么都没跑"的 success** ✗
（**3/15 绿、其余 skipped** ✓）—— 与 2026-09-25 记的那条**同一个形态** ✓：
**`paths-filter` 看的是"这一推的 before..after"** ✗，**不是仓库里有什么** ✗。

**⇒ 第三条（我自己又犯的）** ✗：**在 run 在飞的时候又推了一次** ✗
⇒ 顶层 `concurrency: cancel-in-progress: true` **把上一轮掐了** ✗
（`36207207425` ⇒ **`cancelled`** ✓，**6/27 绿** ✓）⇒ ⇒
**于是"验证 ci.yml 修复"的那一轮永远没跑完** ✗ ——
**要验证 `.github/**` 的改动，就得让那一推带 `.github/**` 路径** ✓，
**而且推完要等它跑完再推下一次** ✓。

## 2026-09-26 · **新 e2e 测试污染全局缓存 ⇒ 撞红一条既有测试**（只在 CI 复现 ✗）

**症状** ✓：CI 的 e2e 报 **`27 passed / 1 failed`** ✗，而红的**不是**我新加的那条 ✓，
是**既有的** `rewriting an unchanged project unit does not recompile` ✗：
```
AssertionError: 内容没变 ⇒ 不许写出新的缓存条目（说明闭包被重编了）
+   '5d5b302d9181a8e1.json:11257:1790385365430.0708',   ← 多出来的一条
```

**根因** ✓（**我的错** ✗）：我给 T-U12 面 #3 加的 hover 测试，
**把夹具写进了 `tmpDir`（新建文件）** ✗ ⇒ 那个文件被 LSP 编译 ⇒
**写进全局缓存** ✓（`cacheStamp()` 读的正是**全局缓存 + 工作区缓存** ✓）
⇒ 在**慢速 CI runner** 上这次落盘**迟到** ✗ ⇒ 正好落进后面那条缓存测试的
**1.5 秒窗口** ✓ ⇒ 它的 `cacheStamp()` 多出一条 ⇒ **判红** ✗。

**⇒ 为什么本地一直是绿的** ✗✓：**本机快** ⇒ 那个落盘在窗口之前就完成了 ✓
⇒ ⇒ **这是"本地绿、CI 红"的教科书形态** ✓ ——
**而它骗过我的方式很隐蔽**：我以为"新增测试"是纯增量 ✗，
**没想到它会给后面的测试留下副作用** ✗。

**⇒ 怎么发现的** ✓（**判据而不是猜** ✓）：
1. 先问"**这条测试历史上红过吗**" ✓ ⇒ 扫 **169 个历史 e2e 日志** ⇒ **一次都没有** ✗
   ⇒ **排除 flaky、锁定是我弄的** ✓；
2. 再问"**多出来的那条缓存是谁写的**" ✓ ⇒ 我那条测试是**唯一新建文件**的 ✓
   ⇒ **锁定** ✓。

**修复** ✓：**改成复用现有夹具、绝不新建文件** ✓ ——
夹具里本来就有 `lib/Set.sokonanoda` 的 `infix:50 " ⊆ " => Set.subset` ✓
与 `units/u01.sokonanoda:11` 的 `subset_mem`（**类型就是 `A ⊆ B -> A ⊆ B`** ✓）
⇒ 直接 hover 它 ✓（**零新文件 ⇒ 零缓存副作用** ✓）。

**⇒ 纪律** ✓：**e2e 里新增一个"编译单元"就是新增一条全局缓存条目** ✗ ——
`cacheStamp()` 把**全局缓存**算进指纹 ✓ ⇒ **任何新建文件的测试都会动它** ✗
⇒ **优先复用夹具** ✓；确实必须新建时，**要在测试末尾等它落盘** ✓
（或把新文件放进工作区，让它成为闭包的一部分 ✓）。

## 2026-09-26 · **改 `editor/**` 永远不会跑真宿主 e2e：skipped 的依赖把它拖走了** ✗

**症状** ✓：`afceda9`（只改 `editor/vscode/src/test/extension.test.js` + docs）那一轮
**`success`** ✓，但 **5/16 绿、11 skipped** ✗ —— **三条 e2e 全 skipped** ✗
（而 `editor` job **success** ✓）。⇒ ⇒ **"success" 是"什么都没验"的 success** ✗。

**根因** ✓：`e2e` 的条件**本来是对的** ✓：
```yaml
    if: needs.changes.outputs.rust == 'true' || needs.changes.outputs.editor == 'true'
```
**但它还写着** ✗：`needs: [changes, lint-fmt, lint-clippy, gates-fast]`
—— 而 **`gates-fast` 的条件是 `rust == 'true'`** ✗ ⇒ **editor-only 推送时它 `skipped`** ✗
⇒ ⇒ **skipped 的依赖把 `e2e` 也拖成 `skipped`** ✗✓（**仓库里记过的那个坑，这次撞在自己身上** ✗）。

**⇒ 这是一个真洞** ✗：**改 `editor/vscode/**` 时，真宿主 e2e 一次都不跑** ✗ ——
而 `if:` 里写着 `|| editor == 'true'` ✓ ⇒ **本意是要跑的** ✓，**是 `needs` 列表把它废掉了** ✗。

**修复** ✓（**两条 e2e 一起** ✓）：**`!cancelled()` + 逐条"依赖没失败"** ✓
```yaml
    if: >-
      !cancelled()
      && (needs.changes.outputs.rust == 'true' || needs.changes.outputs.editor == 'true')
      && needs.lint-fmt.result != 'failure'
      && needs.lint-clippy.result != 'failure'
      && needs.gates-fast.result != 'failure'
```
⇒ **保留原意** ✓（**便宜门禁红了就别跑贵的** ✓），**同时让 skipped 的依赖不再阻塞** ✓。

**⚠ 过程中守卫又咬了一次** ✓：我第一次改用的锚点是那行 `if:` 本身 ✗ ——
而**它和 `editor` job 的那行一字不差** ✗ ⇒ **`assert count == 1` 拦住了** ✓
（**否则会误改 `editor`** ✗）⇒ **改成带 `name:` 的锚点** ✓。
⇒ **这就是"锚点必须先验唯一性"的价值** ✓（**`AGENTS.md` 里那条** ✓）。

**预防** ✓：**新增/修改 `needs` 时，先问"它会不会在本次路径下被 skip"** ✗ ——
**skipped 的依赖会让下游静默消失** ✗，而**整轮还报 `success`** ✗。


## 2026-09-28 · **`gates-course` 顶到 20 分钟上限**（课程涨到 375 checked ✗✓）

run `36373825237`：job failure，末行 `##[error]… has timed out after 20 minutes`。
**判据是对的** ✓ —— 超时前门禁已跑完并打出 `43 个目标 —— 375 checked · 99 open · 0 个被判负`
（`--selftest` 也 PASS）⇒ **不是判据坏、不是课程坏，是上限顶死** ✗。

**根因**：门禁对**每个目标**跑一次 `grade <绝对路径> --json`，每次把该目标的 **import 闭包
冷编译**一遍 ⇒ 成本**随课程规模线性涨**。v0.77.0 新增 6 个 `lib` 模块 ⇒ 331 → 375 checked。
实测冷跑：**本机（M 系 mac，release）5 分 05 秒** / **CI（2 核 ubuntu）19 分 22 秒**（≈3.8×）。
⚠ 3.8× **不是噪声** —— 与 `docs/PERF.md` 记的同一现象一致（那次 7.5×）⇒ **是机器差** ✓。

**修复**：`timeout-minutes: 20 → 40`（与 `ledger` 的 30 分钟同族），实测数字写进 `ci.yml` 注释。
⚠ **不是"为绿色改判据"** —— G1–G5 一条没动、课程难度没降、没有 skip，改的只是给多少时间 ✓。

**预防 / 后续**：① **课程规模涨 ⇒ 先量冷跑**（`rm -rf courses/set-theory/.sokonanoda/compiled`
再跑一次），别等 CI 通知；② **真优化**（不阻塞 0.77.0）：给 `<模块根>/.sokonanoda/compiled/`
加 `actions/cache` —— 本机**热缓存 0.5 秒**（59 MB），但引入"缓存失效/污染"新失败模式 ⇒ 留 0.78。

## 2026-09-28 · **绝对毫秒判据跨机不可转移**（`keystroke_recompile_closure` ✗✓）

run `36378945287` 的 `test (sokonanoda-front, tests)` 判红：`best 1599.6ms · worst 2430.9ms`
（判据 `worst < 2000.0`）；同 job 其余 756 条全绿。
**实测直方图**（**同一份 `crates/` 代码**，`git diff 6a814c0 HEAD -- crates/` 为空）：

| 环境 | worst |
|---|---|
| 本机（8 核满载施压仍只） | 44–71ms（**59.6ms**） |
| CI `36362709263`（ST2） | 240.11ms ✓ |
| CI `36373825237`（ST15） | 343.47ms ✓ |
| CI `36378945287` attempt 1 | **2430.91ms** ✗ |
| 同上 attempt 2（**同 job 重跑**） | **510.66ms** ✓ |

⇒ **同代码同 job 差 4.8×、跨机差 55×** ⇒ 判据在**量机器**不在量代码。
⚠ **另有一条真趋势**：**240ms（ST2 前）→ 344ms（ST2 后）** —— ST2 给 prelude 加 5 条 `Quot`
公理 ⇒ 每块多 5 条声明 ⇒ 冷编译变贵（**与 `gates-course` 顶 20 分钟上限同源**）。

**修复**：判据换成**同机比值** `worst / cold_ref < 8.0`（`cold_ref` = 同一次测试做 3 次冷编译
取最快；实测 **0.60~1.65**）+ **很宽的绝对天花板** 30000ms（挡"两边一起变慢"）。
**反向验证**：阈值压到 0.1 ⇒ 比值那条断言**先咬住** ✓。全量 **1384 passed · 0 failed** ✓。

**预防**：**性能判据不许用绝对毫秒** —— 跨机不可转移（先例：`perf-gate` 的跨机器事故、
`docs/PERF.md` 的 7.5×）。要归一化就取**同机参考量**（比值 + 宽天花板）；
**别**把阈值调大了事 —— 那既掩盖真回归，也仍会被更吵的机器咬到。

## 2026-09-28：**判据命令被管道掩膜 ⇒ 误报绿**（本地事故，非 CI 红；同类第二次，故立此条）

**现象**：本 session 多轮用 `python3 scripts/docs-lint.py | tail -1` 判断"文档预算 ✓"，
而 `docs-lint` 的**不通过明细走 stderr**、只有汇总行走 stdout ⇒ `tail -1` 永远截到那句
"✓ 活文档 …"，于是**红了也看成绿的**（实测：同一棵树 `2>&1` 全量读 = **9 条不通过 ✗**）。

**根因**：把"命令退出码/完整输出"换成"管道的最后一行"——正是本仓库反复警告的掩膜。

**修复**：
1. 判据一律 **`2>&1` 全量读 + 按退出码判**（`cmd; echo exit=$?`），**禁止 `| tail`/`| grep` 决定绿红**；
2. 本 session 的真实触发还叠加了一条：工作区里有**非本次改动**的旧版本文档被换回
   （`REQUIREMENTS.md` 240 → 940 行 · `docs/LESSONS.md` 556 → 2756 行，共 4 文件 3370 行 diff）
   ⇒ 撑爆 `docs-lint` ②③④⑦ ⇒ pre-push 两次按设计拒推 ✓。已留档 `/tmp/rescue/uncommitted.patch`
   并 `git checkout --` 还原（来源未定，最可能是被中断的子代理/后台脚本把旧版本文档写回工作区
   ⇒ **动手前后各看一眼 `git status --short`**）。

**预防**：① 判据读全量输出（上）；② **推送前先 `git status --short` 确认没有"不是我做的"改动**；
③ 门禁红了先读**明细行**（哪一条、哪个文件、哪个数字），再决定改什么。

## 2026-09-29 · 切片 1b 接线：**"变快"其实是"失败得快"**（本机，非 CI）

**症状**：`build --clean courses/set-theory` + `build --json` 从基线 **222.1s** 掉到 **15s** ——
看起来像 15× 提速。**真相**：`build.summary` 是 `{'compiled': 1, 'failed': 41}` ✗ ——
41 个入口**快速失败**，所以"快"。**已回滚**（`git revert`），回滚后冷全量实测
`{'compiled': 42, 'failed': 0}` ✓（墙钟 **625s**，见下条）。

**根因（假设）**：session 的库层是**跨入口去重的并集**（按首个 plan 的闭包顺序编），
而 `assemble_report` 按**该入口自己的闭包顺序**用 `unit_ranges` 切分 ⇒ 命令区间错位
⇒ 事件/错误归到错的模块 ⇒ 大面积 `failed`。

**四条教训（都已变成动作）**：
1. **性能数字必须先看 `failed`/`compiled` 计数** —— "快"可能是"错得快"✗；
2. **revert ≠ rebuild**：`git revert` 后用**旧二进制**复测会得出假结论（本次差点据此误判
   根因不在接线）⇒ 回滚后**必须重建**再测；
3. **咬不住的守卫等于没有**：当时 `cargo test -p sokonanoda-cli --test imports` 的 **21 项全绿** ✗
   ⇒ 已补 **`build <dir>` 多入口守卫**（`build_a_directory_of_entries_sharing_a_dependency_compiles_them_all`，
   断言 `failed:0` + `compiled:4`）并做**反向验证**：接线版 **1 failed**（红的正是它）、
   正确代码 **22 passed** ✓；
4. **~~跨机比数字无效~~ ⇒ 更正为「跨接线版本比数字无效」**（2026-09-29 实测修正）：
   原写法「625s vs 222.1s ⇒ 跨机不可比」**被否定** ✗✓：同机同接线同缓存下 release
   **218.8s** vs debug **260.7s**（差 **3%**）⇒「debug 慢一个量级」是 **CLI 集成测试
   二进制**的现象，不是 `build` 的；**625s** 是**接线中间版本** ✗。⇒ 规矩：同机 + 同
   profile + 同接线版本；别拿"跨机"当解释（证据原文 ⇒ `git log --all -- docs/perf/course-profile-2026-09-29.md`）。

## 2026-09-30 — `gates-fast` 的 P2 进度判据判红（① 的"默认不发"漏了机器消费者）

**现象**：`ci` run `36632168275` 的 `gates-fast` 在
`Progress events never go quiet for long (P2)` 判红：
`✗ 最长无输出间隔 4.99s > 2.5s`（**本机同样代码只 1.14s**）。

**真因**：① 那一档把 `build.tick` 改成**默认一律不发**。工作单的原文是
「人看的终端不再刷 + **机器消费者仍能拿到心跳**」—— 我把后半句漏了 ✗。
`check-progress-gap.py` 要的是"**能力上限**：两条通道都在时能做到多好"，
机器消费者拿不到心跳 ⇒ 它立刻假红（而它**测不出**"终端刷不刷"）。

**为什么本地没复现**：夹具只有 24 条声明、本机 **1.1s 就跑完**（心跳 1s 一次，
勉强够）；CI runner 慢 ~4× ⇒ 同一份夹具跑 **5.0s**，而那一版的心跳周期被
我一起放宽到 5s ⇒ 5s 的周期**本身就达不到 2.5s**。
⇒ **教训**：改"发给谁"的时候**不许顺手改"发多快"** —— 周期是契约的一部分
（`docs/protocol.md` 写死 ≤ 2.5s），放宽它等于把契约悄悄改掉 ✗。

**修法**：口径改成工作单三选一里的**第一项「非管道不发」**：
`std::io::IsTerminal` ⇒ 终端不发、**管道照发**（周期**回到 1000ms 不动**）✓。

**预防**：
1. 判据里加"**管道必须有 tick**"（`cli_build_heartbeat_is_off_unless_asked_for`），
   并写明"终端那一半集成测试测不到"（测试 stdout 永远是管道）⇒ 靠 `IsTerminal`
   + 手工 `pty` 验证；
2. **改计时/契约类常量前先跑那条判据的 `--selftest` 与真夹具**（本轮漏了 ——
   我改完只跑了单测，没跑 `check-progress-gap.py`）；
3. 夹具规模要**能暴露慢 runner**：24 条声明在 CI 上才 5s，建议把这条判据的
   夹具时长记进台账（`wall_s` 已经在 `--json` 里 ✓，可作对比）。

**顺带修**（同一次判红暴露的）：`scripts/soko gate --fast` 把 `--lib` **写死**了
⇒ `sokonanoda-cli`（bin crate，无 lib target）**改了必红**、与代码无关。改成**按 crate 分开选**（有 `src/lib.rs` ⇒ `--lib`，否则 `--bins`）✓（`--lib --bins` 也不行：cargo 对"某个被 `-p` 指名的包没有那类 target"是硬错误）。

## 2026-09-30 — `test (sokonanoda-cli, tests)` 判红：`perf_project` 冷/热都是 ~1025ms

**现象**：`ci` run `36634513370` 的 `test (sokonanoda-cli, tests)` 判红：
`the second build must be far cheaper than a cold one (cold 1002.5ms, warm 1002.1ms)`。
⚠ **冷热两个数几乎相等**（差 0.4ms）是**关键线索** —— 噪声不会这么对称，
"两边都多付了同一笔固定开销"才会。

**真因**：**`Heartbeat::stop()` 等满一个周期** ✗。
① 的"非管道不发"让**测试（stdout 是管道）**也开始起心跳线程，而`stop()` 是"置停止位 + `join()`"，线程却在 `sleep(period)` 里 ⇒
**每次 build 都白等 up to 1000ms**（冷跑和热跑**各**白等一次 ⇒ 两个数一起变 ~1025ms，
比值判据 `warm * 2 < cold` 必红）。

**怎么定位的（二分法，值得复用）**：`git worktree add` 逐 commit 建独立检出、
各自 `cargo build --release` + 跑同一条测试 ⇒ 一次锁定到 `291cae9f`：
| commit | cold / warm |
|---|---|
| `eaf1b155`（批次前基线） | 65.0 / 5.3 ms ✓ |
| `87e3d218`（① 第一版，默认一律不发） | 63.5 / 4.8 ms ✓ |
| `9616c848`（P1-b/P1-c/Q1/Q2 全批） | 56.0 / 5.3 ms ✓ |
| **`291cae9f`（① 改"非管道不发"）** | **1024.0 / 1025.1 ms ✗** |
⇒ **是本轮引入的，且就是"管道也开始发心跳"那一步的副作用**。

**修法**：`sleep(period)` → **`Condvar::wait_timeout`** ⇒ `stop()` 置位 + `notify_all()`
**立刻**唤醒，不再等满周期 ✓。修后：**cold 72.8ms / warm 4.7ms**（比值 15×✓）；
CLI 层实测 40 条声明的夹具：管道 **2.48s** vs `NO_TICK` **2.48s**（**差 0.00s**）。

**预防**：
1. 判据加"**心跳不许拖慢 build**"（`cli_build_heartbeat_is_off_unless_asked_for` 的 ⑤）：
   同一夹具管道 vs `NO_TICK` 的**墙钟差 < 0.5s** ✓—— ⚠ 用**差值不用比值**：两者都含 ~0.66s 进程启动固定开销，比值会被它稀释；
   ⚠ 这一条**放在"夹具够慢"自检之前**：否则注入 `sleep` 时自检先 `panic`、
   ⑤ 根本跑不到 ⇒ **反向验证失效**（第一版就是这么假绿的 ✗）。
   **反向验证**：注入 `sleep(period)` ⇒ `多付 0.57s` **判红** ✓。
2. **"停一个后台线程"永远不要用 `sleep` 轮询**（`join()` 会等满一个周期）——
   用 `Condvar` / channel / `park_timeout` ✓；
3. ⚠ **本次教训的元层**：① 的第一版（一律不发）与第二版（非管道发）**各引入一个
   不同的 bug**，而两版都过了 `gate --fast` —— 因为 `--fast` **跳过集成测试**（`perf_project` 就在里面）⇒ **改了 CLI 的运行时行为，要跑那个 crate 的`--test` 全集，不能只看 `--fast`** ✓。

## 2026-09-30 — `judge_inplace_by` **间歇性**判红（约 2/3）：两个测试抢同一个 `OnceLock`

**现象**：`scripts/soko gate`（完整档）在 `-p sokonanoda-front --test judge_inplace_by`
判红，而**单独跑同一条命令却是绿的**。实测 6 连跑：**4 红 2 绿**（间歇）。

**真因**：**我自己的测试写错了** ✗ —— `SOKO_JUDGE_INPLACE*` 走 `OnceLock`
**只读一次环境**，而**同一个集成测试文件里的多个 `#[test]` 共享一个进程**
（`cargo test` 默认多线程）⇒ 影子档那条与反向判据那条**抢同一个 `OnceLock`**，
**谁先跑到谁定档**：反向判据先跑（`=1`）⇒ 影子档那条的 `same` 恒为 0 ⇒ 判红。
⚠ 讽刺的是那个文件的**头注释写的正是"集成测试各自独立进程 ⇒ 天然隔离"** ——
**声明与事实不符**，而没人核对（同 `AGENTS.md`「判据的两条硬规矩」①：
声称「必红/强制」的，逐条核对它真的成立）。

**修法**：**一个档位一个文件**（各自独立进程 = 天然隔离 ✓）——
拆成 `judge_inplace_by.rs`（影子档）与 `judge_inplace_by_reverse.rs`（反向判据）。修后 **6 连跑全绿** ✓；**反向验证**：注入附十那个真 bug（`peel_binders(… , n + 1)`）⇒ 影子档**判红** ✓（判据本身没被拆坏）。

**预防**：
1. **凡是用 `set_var` + `OnceLock` 配档位的测试，一个档位一个文件** ——
   同文件多 `#[test]` 就是**共享进程**，不是隔离；
2. ⚠ **"单独跑绿、gate 里红"就是竞态的信号**（不是"gate 有问题"）——
   遇到就先**连跑 6 次**看是不是间歇的（本次一次就复现）；
3. **注释里写"天然隔离/必红"之类的断言要当场核对**（本条的注释就是错的 ✗）。

## 2026-10-11 — `ledger (3)` 判红：G-110 复现件写死了本机绝对路径

**症状**：run `38066372589` 的 `ledger (3)` 红，注解逐字：`G-110 open script 环境异常` +
`…/G110-….sh: line 12: cd: /Users/penglingwei/…: No such file or directory`。

**原因**：新写的复现件第 12 行 `cd /Users/penglingwei/…` —— 本机那个目录**确实存在** ⇒
本机 `gap.py check` 永远绿，只有别的机器（CI）才暴露。

**修法**：照本目录其余 90 个复现件的写法自定位 `cd "$(dirname "$0")/../../.." || exit 2`；
横向排查确认 `docs/gaps/repro/` 下只有这一个 `.sh` 写死路径（另两处是缓存 JSON，不受影响）。

**预防**：**复现件与判据脚本一律自定位，禁止写死绝对路径** —— 判据若只在本机成立，
它就不是判据；这类"本机必绿"的洞只有发版节点跑全量才抓得住。
