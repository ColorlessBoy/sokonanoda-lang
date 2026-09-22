# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-21（第一百二十五轮：**批次 2 收口** —— T-A30 编译不再挡住编辑器
> （只读请求 <1ms）· T-A60 冷开 436ms / 热开 58ms · T-A50/T-A51 as-built 与台账；
> 版本 **0.64.2**）
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

## 本轮进度（2026-09-21，第一百二十五轮：**批次 2 收口** —— T-A60 / T-A50 / T-A51）

> 计划 §5.4 的三条：冷开命中缓存 / 内容没变不重编 / 改依赖刷新入口，
> 全部跑在**真 VS Code + 真 LSP** 上。

1. **三条都进套件**：`21 passing / 3 failing`——失败的正是批次 3/4 的记法三例
   （#6/#7/#8，本来就红）。性能数字进留档（`docs/e2e/logs/…`）：
   `PERF e2e cache: cold=436ms warm=58ms`（**7.5×**，判据要求 >3×）、
   `PERF e2e fanout: entry diagnostics publishes=1`（改一次依赖只发一份）。
2. **每次跑一个全新的缓存目录**（`SOKONANODA_CACHE_DIR=$(mktemp -d)`）：冷/热对比
   要有**真冷**的基准，否则"冷开"会命中上一次跑留下的条目（而且结果取决于上一次
   谁跑过）。用例自己读同一个变量定位缓存。
3. **写这三条踩到四个会假绿的坑**（全写进用例注释了）：
   * `languages.getDiagnostics(uri)` **不是"刚发来"的信号**——VS Code 按 URI 留着
     上一次的结果、也不会因 `didClose` 清掉 ⇒ "非空"让打开立刻满足条件、量到 0ms，
     而那次**根本没编译**（第一次就是这么假绿的，是"冷开必须写缓存"那条前置断言
     抓住的）；
   * **监听器要早于发布挂上**：`restartServer` 会把打开中的文档重新同步一遍；
   * **`closeAllEditors` 不等于 `didClose`**：`openTextDocument` 的 `TextDocument`
     还被引用时客户端不发 `didClose` ⇒ 服务端那份 `Doc` 还活着，重开"什么都没
     发生"（实测连续两次假绿）。冷开改成用**从没编译过的文件**；
   * **时间断言要让被测那段占主导**：夹具 3 条声明时编译只占 ~30ms，冷/热都被
     "重启 + 往返"的固定开销（~60ms）淹没（89ms vs 63ms）⇒ 冷开夹具**故意做大**
     （+120 条用库记法的定理）。
4. **保存那条为什么不用 `workbench.action.files.save`**：VS Code 对**干净缓冲区**
   的保存是 no-op（不发 `didSave`），从扩展宿主里测不到服务端短路。用例走同一条
   服务端路径的另一半（磁盘重写同样字节 ⇒ `did_change_watched_files`）；
   真 `didSave` 由进程内用例 `perf_course_save_same_text_is_recorded` 钉着。
5. **没有生产代码改动**：只动 e2e 用例与 `scripts/vscode-e2e.sh`（缓存目录）。
   内核与 LSP **零改动**。
6. **批次 2 收口（T-A50 / T-A51）**：
   * **T-A50 as-built**：`docs/design/compile-cache.md` §8（键不含任何文件系统属性、
     条目 v2 按模块、`is_clean` 判据、LSP 三件事、**跨入口仍不共享**的边界）；
     `docs/architecture.md` §4.5 第 6 条同步。
   * **T-A51 性能台账收口**：`docs/PERF.md` 补"修前 → 修后"两张表 +
     真宿主 e2e 的数字；`scripts/perf-ledger.sh` 跑通并追加条目。
     冷/热判据 ≥5× —— 真宿主实测 **7.5×**。
     台账里唯一超阈值的退化是 `lsp-course/keystroke` **381 → 526ms（+38.1%）**，
     **就是 T-A30 那 120ms 防抖**（编译本身 392ms 没变），已在 `docs/PERF.md`
     的单列里记账解释。
   * 版本 bump 到 **0.64.2**（§13 给 T-A51 标的 patch 点），CHANGELOG 写清
     用户可感的两件事（不再冻住 / 短路判据换成闭包摘要）。

## 本轮进度（2026-09-21，第一百二十四轮：**编译不再挡住编辑器** —— T-A30）

> 用户：「并发正确性……这个我认为完全可以优化，你调研一下其他开源项目怎么做的。
> **编辑同一个地方，那就取消前一个编译……不管怎么样，多次编辑不应该导致性能变差。**」

1. **先把判据写成会红的测试**（DoD 第一步）：新集成测试
   `crates/lsp/tests/lsp_edit_concurrency.rs` —— 真进程、真 stdio，1200 条声明的
   夹具（编译 ~1.2s），`didOpen` 之后**不等**诊断直接发 `soko/stateAt`。
   改前实测 **1277ms**（= 整个编译期，判据要求 <100ms）。
2. **修法**（clangd 的 `TUScheduler`，调研见 `docs/design/lsp-edit-concurrency.md`）：
   `did_open` / `did_change` / `did_save` / `did_change_watched_files` 只**同步**
   记下最新文本 + 版本就返回；编译由 `tokio::spawn` 出去的任务做，**不持
   `Docs` 锁**；编译期间到达的只读请求读**上一次完成的状态**。同一份文档同时
   只有一个编译任务（`inflight`），编辑期间来的新版本只把 `pending` 换成最新
   那份 ⇒ **N 次快速编辑最多跑 2 趟**。
3. **上一版为什么做坏了、这一版怎么还的**（设计 §6 那张清单逐条）：
   * 上一版另起了一条编译路径 ⇒ **把读缓存漏了**（热开 8ms 退成 810ms）。
     这一版**不另起路径**：编译载体就是一个 `Doc`，走的就是 `Doc::set_text`
     本身 ⇒ 读缓存 / 写缓存 / `parse_error` 折叠一样不少；
   * 装回**只复制视图**（`QueryDoc::adopt_view`），**载体完整保留"输入 X 的状态"**
     ——只同步"输入"字段、把视图留在旧状态上，短路一命中就会把两轮之前的报告
     装给 handlers（实测：`project_modules()` 变 `None`，扇出判定当场失效）；
   * `tokio::spawn` 落到 2MB 栈的 worker ⇒ 栈溢出。`run()` 换手写 `Builder`，
     `thread_stack_size(32MB)`。
4. **顺带修掉两个真 bug**（都不是这一版引入的，是"以前没人这么问过"）：
   * **项目文档的短路判据错了**：T-A21 只看"自己的文本 + 覆盖"，而依赖在
     **磁盘上**被改了（`git checkout` / 另一个编辑器）时文本一个字节没变 ⇒
     旧诊断一直显示下去。改成**闭包摘要**（只读文件 + 哈希，毫秒级）；
   * **`inflight` 退休竞态**：任务"没活了"与"新活插进来"之间有窗口，会让这份
     文档**从此再也不会被编译**（实测 `watched_unchanged` 第 2 轮起卡死）。
     改成同一把锁里判定。
5. **实测与代价（不许藏）**：

   | 判据 | 改前 | 改后 |
   |---|---|---|
   | 1.2s 编译进行中的 `soko/stateAt` | **1277ms** | **< 1ms** |
   | 打开 + 连打 5 个键的**编译趟数** | 6 | **≤ 3** |

   代价：**重建慢的文件**（上一次编译 ≥150ms）下一次编辑等 **120ms 静默期**
   （clangd 的规则，`SOKO_DEBOUNCE_MS` 可覆盖），小文件不防抖。课程侧
   `keystroke` unit08 381ms → **524ms**——**编译本身 392ms 没变**，差的 143ms
   是 120ms 防抖 + 视图克隆；`did_open` 三条 +2~5%（噪声内）。
6. **判据**：`cargo test --workspace` 全绿（37 个测试二进制）· `scripts/soko gate`
   **exit 0**（课程 36 目标 · 328 checked · 99 open · 0 判负）· 真 VS Code e2e
   **18 通过 / 3 失败**（失败的正是计划里批次 3/4 的记法三例 #6/#7/#8，本来就红）。
7. **内核一个字节未改**（这一刀全在 LSP 前端）。**未 bump**：§13 给 T-A30 没标
   BUMP，批次 2 的 patch 点在 T-A51。

## 本轮进度（2026-09-21，第一百二十三轮：**记法消解也在重编前缀** —— G-34，`judge_infer` 那一刀）

> 用户：「我还是很疑惑，lean 的 by 风格有这么耗时吗？是不是我们的 by 的实现方案
> 有问题呢？」「那要插入方案进计划里，这个违背我们的红线，也违背我们的热编译的
> 设计初衷。」「前端有问题，前端也一起配合改掉，这属于重大事故的 bug。」
>
> 上一轮把 `solutions/` 改成项风格之后，`by` 判定掉到 11 次 / 0.9s——
> **但 `unit12-solution` 仍然要 11.4–12.0 秒。大头换了人。**

1. **先定位"这几百趟 pass 到底是谁在调"**：新加两个**常驻**诊断开关
   ——`SOKO_PASS_TRACE=<n>`（第 n 趟 `run_pass` 的调用栈）与
   `SOKO_INFER_TRACE=<n>|all`（每次 `judge_infer` 未命中的查询 + 栈）。
   第一次就量出：**380 趟 pass 全部来自 `elab_notation` → `solve_prefix_args`
   → `infer_type_text` → `judge_infer`**。
2. **缺口 G-34 成立**：`judge_infer` 的缓存键**含整段前缀**，前缀随每条声明
   增长 ⇒ 同一批查询每次换一个键；**未命中一次 = 合成 `<前缀>#check fun
   (binders) => term` 把整段前缀从零重跑一趟 pass**。实测
   **126,105 次调用 / 363 次未命中**（`unit12-solution`，release，冷缓存），
   `judge_infer` 累计 12.3s。这与 G-31 是**同一个病、不同入口**。
3. **命中侧不是问题，别打错靶**：`JUDGE_INFER_SPLIT hits=50909 misses=363
   key_ms=741 hit_ms=744` ⇒ 5 万次命中只花 744ms，**优化必须打未命中**
   （即"别问内核"），不是打哈希。这条读数纪律写进了 `docs/PERF.md`。
4. **T-K22 那一刀（已落地）**：`elab.rs` 新增 `operand_type_expr`——
   **局部变量先取 `ElabScope::source_type_of`（书写类型，零内核调用）**，
   拿不到才退回 `infer_type_text`。判据与 `implicit.rs` 里 `arg_tys` 的
   **既有判据完全相同**（书写类型不但零调用，还比内核 pp 更准——pp 会丢嵌套
   常量的隐式实参）。三个入口同时换：`solve_prefix_args`（主）、
   `guarded_binder_type`、`arg_tys`。

   | 指标 | 改前 | 改后 |
   |---|---|---|
   | `judge_infer` 调用 | 126,105 | **51,156** |
   | 未命中（= 整段前缀重编一趟） | 363 | **247** |
   | `judge_infer` 累计 | 12.3s | **6.9s** |
   | **墙钟** | **11.4–12.0s** | **7.5–8.4s** |

5. **这是缓解，不是根治**（计划里明写）：剩下的 247 次未命中来自冗余 `sorry`
   探针（`fun (__soko_render : T) => __soko_render`）、`And.intro` 这类**裸常量
   头**、inductive 安装与闭包里的记法——**没有局部类型可拿** ⇒ 只能靠 T-K20′
   的「就地拿当前 pass 的环境」，而那套设施要**同时**覆盖 `judge_pairs` 与
   `judge_infer`（已写进 `by-judge-reuse.md` §7、计划 §5.6.1 T-K20′、REQUIREMENTS §9）。
6. **判据**：`scripts/kernel-diff.sh` 全语料逐字节对拍 **零差异** · `cargo test
   --workspace --locked` 全绿 · 课程门禁计数不变 · 缺口复现
   `docs/gaps/repro/G34-notation-type-query-recompiles-prefix.sh` 已进
   `gap.py check`（gate + CI）。
7. **顺带修掉一条会随机翻红的复现**：G-29 的判据 `edit*2 < cold` 余量只有 ~13%
   （4413ms vs 2489ms），机器一抖就翻面 ⇒ `gap.py check` 随机红（本轮实测翻过
   一次）。冷开里混着**进程启动**，本来就不该进分母；改成与**热开**比
   （`edit < 10×warmOpen + 200ms`，实测 2582ms vs 预算 280ms，余量 9×）。
8. **不变的**：内核**一个字节未改**（这一刀全在前端）；不调用官方 Lean 工具链；
   用户/agent 路径仍零 cargo。版本 bump 到 **0.64.1**（patch：纯提速，无新能力）。

