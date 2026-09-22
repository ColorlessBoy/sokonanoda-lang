# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-21（第一百二十七轮：**折叠层落地** —— 线 C 的 C-II 收口：
> 二元 infix 折叠 · arity 来源 · `scoped` 决定（并关账 G-35）· 重载处置 ·
> 三层损失护栏；`display` 17 条单测全绿；版本 **0.64.2**）
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

## 本轮进度（2026-09-21，第一百二十七轮：**折叠层落地** —— 线 C 从"定靶"到"能折"）

> 承上一轮的前置链条（实测表 / 消费者审计 / 权威设计 / `DisplayText` 护栏 /
> 记法表唯一实现），本轮把**折叠层本身**做完（5 个环节），线 C 的 C-II 收口。

1. **T-C10 折叠第一刀（二元 infix 族）**：`display::print_back(text, &dn) ->
   DisplayText`，自底向上折。**8 条单测**覆盖判据要的 5 类（左结合 / 右结合 /
   优先级括号 / 嵌套 / 不命中回退）+ 3 条护栏。
   **定下两条性质**：① **命中不了就原样**（表里没有 / 元数对不上 / 含松散变量
   `$N` / 解析不了）；② **一处都没折 ⇒ 逐字节原样**——这条比"折对了"更要紧，
   因为最后一步的 `render_expr` 会把 `forall (a b : T),` **拆成箭头链**、
   把 `Type 0` 重排成 `Sort 1`（它是回读通道的输入，故意的）⇒ 无条件重渲染会让
   **每一条不带记法的类型都跟着改样子**。显示漂移因此被限制在"真的折过"的文本里。
2. **T-C11 arity 的来源**：`arities_in_sources` / `arities_with_prelude`——parse
   源文本数 telescope 层数。判据实测：`∈` => `Set.mem` 的 telescope = **3**、
   操作数 = 2 ⇒ 前导参数 1 个（那个 `α`）。三个踩到的点：`def f (a : T) (b : T)`
   是**一个 `Forall` 带两个 binder**（数 binder 不数节点）；**parser 已经把声明名
   限定好了**（自己再拼会得到 `Foo.Foo.bar`）；归纳类型要数 `params` + 类型 binder。
3. **T-C12 `scoped` 的保真度 —— 决定 + 顺手修掉 G-35**：决定是**不做位置精确、
   但要两遍扫描**（parser 的逐命令语义是编译期的关切；读回通道只有一段前缀，
   "结束时生效"才是它要的答案）。而"结束时生效"必须真的按结束时算——一遍扫描
   会让「先声明 `scoped infix`、后 `open scoped`」（**正常写法**）漏掉那条记法，
   那就是 T-C04 发现的 **G-35**，本轮修掉并**关账**（`fixed_in = 0.64.2`）。
4. **T-C13 重载的处置**：两个方向分开看——**同一符号 N 个 target 不是歧义**
   （反向的判据是 head 名字）；**同一 target 两个符号**才是唯一残留歧义 ⇒
   **取声明顺序第一个**（测试把顺序倒过来，符号跟着变，证明判据就是顺序）。
5. **T-C14 损失护栏（三层，从强到弱）**：**类型**（`DisplayText`，T-C03b 的
   `compile_fail` doctest）/ **幂等**（再折一次一个字节不变——不幂等会叠出
   `(a ∈ A) ∈ B`）/ **可解析**。并写清"可解析 ≠ 逐字节回读等价"：折过的文本重新
   解析得到 `Expr::Notation`，结构上不等于展开后的 `App`（那正是记法的定义）。
   `render_expr_round_trips` **一字未动**（实测仍绿）。
6. **判据**：`cargo test -p sokonanoda-front display` **17 条全绿** ·
   `scripts/soko gate` **exit 0** · `gap.py check` 全绿（G-35 已关账）·
   `plan.py check` OK（120 环节）· 内核**一个字节未改**。
   **未 bump**：线 C 的 patch 点在 T-C41。
7. **下一环**：T-C20（接进生产者 1+3）——那里要拍一个决定：既然"折过就重渲染"，
   **含记法**的类型文本会连带换一种 binder 写法（`forall (α : Type 0) (A B : Set α),
   A ⊆ B` → `(α : Sort 1) -> (A : Set α) -> (B : Set α) -> A ⊆ B`）。接受，还是让
   显示出口做**源保留拼接**（只替换折过的子树）。

## 本轮进度（2026-09-21，第一百二十六轮：**线 C 开工** —— 记法进 goal / 类型行）

> 批次 3 线 C 的前置链条（计划 §6.0 的 C-0）：**先量清楚、先定靶、先立护栏**，
> 再动任何折叠代码。本轮做完全部前置（6 个环节）。

1. **T-K32 内核 pp 补单测**（线 C 的前置）：`crates/kernel/tests/pretty_printer.rs`
   **5 条**钉住 `pp_expr` 的文本输出（`->` / `forall` / `{}` 隐式 / binder 未使用就
   折成箭头 / 匿名 Pi 套具名 Pi 要括号 / `Prop` 与 `Type 0` / 应用左结合与参数括号 /
   匿名 binder 的空转义 `«»` / 层级实参默认不打印）。**特征化测试**，做过**变异
   检查**（把 `f (g x)` 的期望改成 `f g x` 实测变红）。两个建夹具的坑写进注释：
   `Config::default()` 的 `proofs = false` 会让 pp 对开项跑 `is_proof` 推断 ⇒
   `infer: loose bvar` panic；用到的常量必须真的声明。
2. **T-C01 四个生产者 × 真实文件的实测表**（`docs/design/notation-aware-printing.md` §1）：
   逐格实测 unit01/08/12 + unit04（`apply` 例外）+ 解答，每格带可重跑的命令。
   三条结论：① **光标在不在 tactic 上**决定走哪一支——学习者的光标就在 tactic 上，
   所以他看到的就是内核 pp 的点名形式（**这就是用户的抱怨**）；② "`by` 步进保留
   记法"**只对结构型 tactic 成立**——`apply Set.ext` 之后是 `(x : α) -> Iff (A x) (B x)`，
   `∈` 与 `↔` **一起消失**（子目标来自被应用引理的 pp 望远镜）；③ **同一份声明在
   两个 surface 上文本不同**（`goal` 是 `(a ∈ A) -> a ∈ B`，`sorry` 行是点名形式）。
3. **T-C02 消费者审计**（同文 §2）：五个字段 × 全部消费者，逐条 `file:line`。
   **关键是一条不对称**：`ty_text` **只有给人看的消费者** ⇒ 可就地改；
   `goal` / `binders[].ty` / `sub_goals[].ty` **同时是 judge 的输入** ⇒ 只能在
   **显示出口**重写。另加 §2.3：线 C 会让 `kernel-diff.sh` 报差异，那是预期的
   ——判定正确性看课程门禁计数逐项不变。
4. **T-C03 升格权威设计**（同文 §3）：把 `printback-feasibility.md` §4 升格；
   §3.1 写清**为什么不走内核 pp**（**发现 A**：内核的记法打印是**死代码**，
   `ExportFile.notations` 全仓库无一处 insert，且 `pp_app` 要 `args.len()` 恰好
   1/2 而 `∈` 展开成 3 个实参；**发现 B**：`pp_expr` 同时是 `#check`/`#reduce` 的
   出口 ⇒ 改它就动 `--json` 字节）；§3.2 **arity 硬规则**（只有
   `spine.len() == arity` 才是记法实例）。**两处失效理由就地作废**
   （`notation-subset.md` 与 `course-lean-style.md` 的"内核冻结"）。
5. **T-C03b `DisplayText` 护栏**（`crates/front/src/display.rs`）：无 `Deref` /
   无 `as_str` / 无 `Into<String>`，唯一读法 `as_display_str()`。判据是
   **`compile_fail` doctest**，两条都做过**变异检查**（加 `Deref` ⇒ 第 1 条红；
   加 `as_str()` ⇒ 第 2 条红；还原 ⇒ 全绿）。第三条是正向 doctest，防止把护栏
   做成"谁都读不出来"。
6. **T-C04 记法表提成唯一实现**（`crates/front/src/notation.rs::notation_table`）：
   逐字从 `judge.rs` 提取（行为不变，零额外解析开销），4 条单测。
   ⚠ **提取时发现一个真陷阱并记了台账 G-35**：这个函数是**扫一遍**而不是两遍
   ——`open scoped Foo` 写在 `scoped infix` **之后**（正常写法）时收不到那条记法，
   而注释写的是"取前缀结束时生效的那些"（两遍扫描的意图）。**没有顺手改**
   （T-C04 是纯提取），而是特征化测试钉住 + 台账 + 复现件。
7. **不变的**：内核**一个字节未改**；课程语料未动；门禁计数逐项不变。
   **未 bump**：§13 给这批前置没标 BUMP，线 C 的 patch 点在 T-C41。

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
7. **T-A52（用户拍板「做，但默认关」）**：新设置 `sokonanoda.warmCacheOnOpen`
   ——激活时后台把**工作区根** `build` 一遍预热缓存。三条纪律：不弹输出面板、
   不报错、不阻塞激活（fire-and-forget）。**默认关**的理由写在设置说明里：它占
   CPU/IO，而"打开编辑器"本身会因此变慢。两层判据：stub 宿主 **34/34**（新增
   两条：默认关时**一次 build 都不许起**；开时正好起一次、目标是工作区根）+
   真宿主 e2e 一例（`22 passing / 3 failing`，仍是那三条已知红的记法用例）。

