# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-15（第七十一轮：prelude `Bool`；0.41.0）
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

## 本轮进度（2026-09-15，第七十一轮：prelude `Bool`）

> 续 HANDOVER §3 C / ROADMAP I6：把 `Bool` 作为真实可信归纳加进 prelude，
> 与 `Nat`（0.36.0）同法，供 `match` 与后续布尔例子使用。

1. **安装**：`prelude.rs::install_bool_prelude` 调用既有
   `install_inductive_block`，`Bool` **非递归** → 构造子 `Bool.true`/`Bool.false`
   + 派生 `Bool.rec`（两分支、无 IH），登记进 `known` 与 `match` 的
   `InductiveTable`；`PRELUDE_NAMES` 增 4 个名字（补全/目标视图）。
2. **闸**：`check.rs::run_pass` 增 `explicit_bool`——文件自带 `inductive Bool`
   时 prelude 让位（否则重复声明 panic）；`session.rs::PreludeShape` 扩成
   `(mode, explicit_nat, explicit_bool, eq_taken)`，任一变化整体重编译。
3. **内核零改动**：`Bool.true`/`Bool.false` 的 name-cache 槽位早已存在
   （原生 `Nat.beq`/`Nat.ble` 用），归约走通用构造子 iota。
4. **测试**：front `prelude_bool_is_available_without_a_source_block` /
   `match_prelude_bool_not_checks_and_reduces`（`#reduce bnot Bool.true =>
   Bool.false`）/ `prelude_bool_definitions_compose` /
   `explicit_bool_block_yields_to_the_source_declaration`；CLI
   `cli_match_on_prelude_bool_checks_and_reduces`；既有源内 `inductive Bool`
   （`tt`/`ff`）用例继续通过=闸生效。
5. **文档**：`architecture.md §5.4`、`design/match.md §2/§10 Phase 5`、
   `TESTING.md`、`protocol` 错误文案（`Nat/Bool`）；错误提示改为
   「prelude 内建的 Nat/Bool」。
6. **验收**：`sokonanoda gate` PASS；版本 0.40.0 → **0.41.0**（新能力 minor）。

## 本轮进度（2026-09-15，第七十轮：统一 goal 呈现 + Infoview 落右侧）

> 用户：Infoview 弹出「暂时不可用」很困惑、希望默认在右侧；各处 goal
> 高亮/颜色各自独立不可维护，要求对齐 VS Code 代码框标准。参照 Lean4
> Infoview（服务器下发结构化 tag + 客户端按主题渲染）。设计
> `docs/design/goal-rendering.md`。

1. **单一分类源**：`front::semantic` 新增 `tag_runs`/`tag_expr`/
   `declaration_kinds` + `SemanticKind::{ALL, as_str}`——把任意表达式文本按
   编辑器同一套规则切成 `(text, kind)` runs。
2. **协议**：`soko/stateAt`（及 `soko/goals` 的 binders）新增 `goal_runs`/
   `ty_runs`（有序 `{text, kind?}`，`kind` 用 wire 名），旧字符串字段保留；
   `docs/protocol.md` 记录词表。
3. **Infoview 渲染**：webview 用 runs 生成 `tok-<kind>` span（不再自绘规则、
   仍仅 textContent），`infoview.css` 单一映射到主题变量；契约测试断言每个
   `SemanticKind` 都有 `.tok-*` 类。
4. **落位 + fallback**：视图移出 explorer，进
   `viewsContainers.secondarySidebar` 的 `sokonanoda` 容器（**右侧**，engine
   `^1.85.0 → ^1.106.0`，已核实 1.106 为无需 proposed API 的首个稳定版）；
   删除 `waitReady`/2s 握手与「暂时不可用」提示，失败静默回退树组。
5. **防漂移**：TM 语法（hover 代码框着色）关键词/命令/sort 列表由测试断言
   == `front::semantic`（keywords + sorts + forall），删掉硬编码 `Nat`。
6. **市场门面**：`description` 348 → 247 字符（>300 被 Marketplace 硬截断、
   切在 `opencode` 中间）+ 护栏测试；README/CHANGELOG 同步。
7. **验收**：front/LSP/cli 契约测试 + `sokonanoda gate` PASS；版本 0.39.1 →
   **0.40.0**（新面板位置 + 协议字段，minor）。



> 续 TODO（HANDOVER §3 A）：消除依赖类型判定/建议里「内核类型文本 → AST」往返
> 的括号歧义。

1. **根因**：`proof::render_expr` 的 `Arrow` 分支把 **domain** 直接 `render_expr`，
   当 domain 是 Forall/箭头时输出 `(k : Nat) -> P k -> Q` 被右结合误读；
   `judge_infer` 逐层 render→parse 剥 Pi 时腐蚀 telescope → 依赖 `match` 的
   level 查询报 `elab-match-no-expected-type`。
2. **修复**：Arrow domain 位改用 `render_fun_position`（Lambda/Forall/Arrow/
   Plus/Let/Match 一律补括号）。
3. **回归**：`render_expr_round_trips` 增「Forall 作 domain」用例（含渲染→再解析
   稳定）；`match_dependent_motive_with_function_typed_binder_round_trips_safely`
   （结果类型 `Q hs n`、`hs` 为依赖函数 binder）内核通过。
4. **影响**：`judge_infer` 的所有消费方受益（依赖 `match`、suggest、半表达式
   hover、level 查询）。
5. **验收**：`sokonanoda gate` PASS；版本 0.39.0 → **0.39.1**（健壮性 patch）；
   设计 as-built `docs/design/match-dependent-motive.md` §8；HANDOVER §3 A 勾选。

