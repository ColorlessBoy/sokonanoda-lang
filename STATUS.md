# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-15（第七十九轮：共享缓存+build / Infoview 稳定与反馈 / 高亮单一起源；0.49.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-15，第七十九轮：共享缓存 + `build` / Infoview 稳定与反馈 / 高亮单一起源）

> 用户三轮反馈：(a) Infoview 面板"点几次才出现、很不稳定"，怀疑是编译卡住，要求
> 面板 UI 必须保证出现、数据可显示"渲染中"/编译进度；(b) 去掉没生效的声明点击跳转，
> 名字后加小字行号；(c) hover 的高亮与 Infoview 不一样、没有收拢。另要求
> `sokonanoda build` 这类命令配合缓存。派出 5 个 subagent 分头实现（SA-1…SA-B）。

1. **共享编译缓存（SA-1）**：缓存下沉到 `crates/front/src/compile/cache.rs`，
   条目含 `report` + `output`；key = `CACHE_FORMAT|版本|二进制构建指纹|prelude 模式|源文本`；
   `SOKONANODA_CACHE_DIR`/`SOKONANODA_NO_CACHE`；原子写；`compile_all_with` 一趟出两者。
2. **`sokonanoda build`（SA-2）**：`build [--json] [--clean] [<file>|<dir>…]` 预热/清理
   缓存并打印 hit/compiled/failed；`course` 与批量 `--json` 走 `compile_cached`
   （冷热输出逐字节一致，有测试）；CLI 测试用临时 `SOKONANODA_CACHE_DIR` 隔离。
3. **Infoview 稳定性根因（SA-3）**：视图原带 `when` + 扩展容器 `hideIfEmpty: true`
   → 无激活 `.sokonanoda` 时容器整块隐藏；且 `activate()` **先 `await
   resolveServerForStart` 才注册 provider** → 期间视图无 provider（"点几次才出现"）。
   修：视图无 `when` + `visibility: visible`；`activationEvents` 加
   `onView:sokonanoda.infoview`；provider/树**同步先注册**，慢解析后置并推 `status`；
   `openInfoview` 先开辅助栏。契约测试锁死注册顺序。
4. **UI 反馈（SA-3）**：webview 载入即骨架（`正在渲染…`），宿主推 `status`
   （`编译中…`/`已就绪 · N 个声明`/`等待 .sokonanoda 文件`），绝不静默空白。
5. **声明列表（SA-3）**：去掉点击跳转；名字后小字行号 `L<n>`（1-based）+ 类型提示。
   新增 `editor/vscode/test-webview.js`（Node DOM shim 行为测试 8 项）并入 `test:unit`。
6. **高亮单一起源（SA-A/SA-B）**：`SemanticKind::{ALL, as_str, tm_scope}` 唯一表；
   `runs_to_text`/`goal_text`/`goal_runs` 单一文本生产者；hover 目标态改由 runs 投影
   （不再手搓字符串）；TM 语法补齐 `variable.parameter` 等 scope；三处穷尽测试
   （TM scope / CSS 类 / LSP legend）防漂移。设计 `docs/design/highlighting.md`，
   并**写明平台限制**：markdown 只能 TM 着色 → 颜色近似而非全等。
7. **验收**：`cargo test --workspace --locked` 全绿 + `sokonanoda gate` PASS；
   `build` 冷/热/clean/目录/课程缓存冒烟通过；版本 0.48.0 → **0.49.0**。

## 本轮进度（2026-09-15，第七十八轮：编译结果缓存 + Infoview 细节）

> 用户四项：(1) Infoview 类型小行允许换行；(2) 目标用 `⊢` 开头；(3) 点击
> Infoview 跳转没生效；(4) 设计类似 Lean4 的编译结果文件（避免文件多了打开即编译慢）。

1. **换行**：`media/infoview.css` 的 `.decl-ty` 由「单行省略」改 `pre-wrap` +
   `word-break`（类型不再看不全）。
2. **`⊢` 开头**：`infoview.js` 的 goal 代码块加前缀 `⊢ `（与 hover/树 tooltip 一致）。
3. **点击跳转修复**：点了 webview 后 `activeTextEditor` 为空，旧实现据此直接失败。
   改为 plumb 文档 uri（树的 `onDecls(decls, uri)` → `setDecls(decls, uri)` →
   webview `focusExercise{uri,range}`），扩展用 `jumpToRange`（`visibleTextEditors`
   优先、必要时 `openTextDocument`）跳转。
4. **编译结果缓存（olean 式）**：`crates/lsp/src/cache.rs` 把内核产出的
   `DocumentReport` 以稳定 FNV 哈希 `(CARGO_PKG_VERSION, prelude 模式, 源文本)`
   落盘；`refresh` 命中则跳过 `session.update`，miss 则编译并落盘。诊断由
   `report_diagnostics` 统一构造（命中/重编一致）。`SOKONANODA_NO_CACHE=1` 关闭、
   `SOKONANODA_CACHE_DIR` 重定位；front 报告类型加 serde derive。
5. **测试**：`cache.rs` 单测 3（key 稳定/作用域/format miss）；扩展契约更新
   （`⊢`/wrap/uri 跳转）。
6. **验收**：`sokonanoda gate` PASS；版本 0.47.0 → **0.48.0**（新能力 minor）；
   设计 `docs/design/compile-cache.md`。已知边界：不缓存 Session 快照、无 LRU。

## 本轮进度（2026-09-15，第七十七轮：带索引归纳）

> 续 HANDOVER §3 B / ROADMAP I6 的最后一项：`inductive Vec (A : Type) : Nat -> Type`。

1. **索引定义**（内核契约）：索引 = `ty` 在 `num_params` 之外的 Pi 望远镜
   （`inductive.rs::check_inductive_spec_0th`）；内核本支持 `num_indices`，本轮
   只补前端。设计 `docs/design/indexed-inductives.md`。
2. **安装**：`install_inductive_block` 算 `index_binders`/`num_indices`，传入
   `add_inductive`/`RecursorData`，存入 `InductiveInfo{num_indices,index_types}`；
   `is_prop_block_ty` 先剥索引望远镜。
3. **派生 recursor**：motive = `forall indices, Ind params indices -> Sort`；rec 绑定序
   `params→motive→minors→indices→target`；minor = `motive <ctor 索引> (C 字段…)`；
   iota 自调用带字段索引实参；字段名替换同时作用于字段类型与 ctor 结果索引实参。
4. **match**：从 scrutinee 书写类型取索引实参；motive 先绑索引再绑 major；
   应用 `Ind.rec params motive minors indices scrutinee`。顺带修既有 latent bug：
   字段类型引用前面字段（`v : Vec A n`）时按「字段原名→用户绑定名」substitution。
5. **边界**：结果类型依赖索引不做（sound 拒绝；另立设计）。
6. **测试/课程**：front +3、CLI +1；课程 unit5 带索引 Vec 节 + 练习 10
   （golden `(11,9,6)→(13,10,7)`、汇总 `checked 55→57 / open 42→43`）。
7. **验收**：`sokonanoda gate` PASS；版本 0.46.0 → **0.47.0**（新语法 minor）。

