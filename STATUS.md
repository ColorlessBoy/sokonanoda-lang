# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-15（第七十八轮：编译结果缓存 + Infoview 细节；0.48.0）
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

## 本轮进度（2026-09-15，第七十六轮：`match` 作为 tactic）

> 续 HANDOVER §3 B / ROADMAP I6：`by` 块内可用 `match`（设计与白名单此前待定）。

1. **tactic 集**：`by` 白名单加 `match`——`match c with | p => <项> …`，臂体是
   **项**（同值位 match），以当前目标为期望类型判定，语义等价 `exact (match …)`；
   `parse_tactic` 复用 `parse_match` + `tactic_keyword_ahead` 纳入 `match`。
2. **judge 修复（根因）**：`judge_terms` 合成文件原 `src: String::new()`，
   `command.span().start` 前缀切片为空 → `match` 的宇宙查询（`judge_infer` 看
   不到 `Color` 等声明）失败，报 `elab-match-no-expected-type`。改为把真实
   `prefix_src` 作为文件 `src`、合成声明 span 放到前缀之后。副产品：
   `by exact match …` 也可用。
3. **测试**：parser `match_is_a_tactic_in_a_by_block`（白名单 + 降到 Exact）；
   front `by_block_with_match_tactic_checks` / `by_block_with_exact_match_checks`；
   CLI `cli_by_match_tactic_checks_via_kernel`。
4. **文档**：`by-tactics.md` §2 表 + 0.46.0 更新、architecture、TESTING。
5. **验收**：`sokonanoda gate` PASS；版本 0.45.0 → **0.46.0**（新语法 minor）。
   注：臂体是「项」；「每个臂里再写一串 tactic」是后续可选扩展（设计 §9 留白）。

