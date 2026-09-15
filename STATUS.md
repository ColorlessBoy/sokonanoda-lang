# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-15（第七十轮：统一 goal 呈现 + Infoview 落右侧；0.40.0）
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

## 本轮进度（2026-09-14，第六十八轮：`match` 依赖 motive）

> 续 TODO：让 `match` 的结果类型随 scrutinee 变化（`P n`），从而能写出归纳法。

1. **设计** `docs/design/match-dependent-motive.md`（触发/构造/交互/风险）。
2. **前端**：scrutinee 是裸局部变量 `x` 且 `R` 含 `x` → motive = `fun t =>
   R[x:=t]`（`substitute_names`），分支期望 = `R[x:=<ctor 项>]`、IH 类型 =
   `R[x:=<field>]`；motive/分支/IH 类型在 binder 存活的 scope 里 elaborate。
   否则保持常量 motive（完全兼容）。
3. **修缺口**：`infer_expected_level` 改为只纳入 `R` 依赖到的 binder
   （`judge_binders_for`），修掉「无关函数型 binder 破坏 judge_infer 望远镜」
   导致**声明 binder 形式**（`nat_induction`）level 查询失败的问题。
4. **goal 视图**：match-arm 走查同样代入 `x := C params v…`，分支 `sorry` 期望
   `R[x:=ctor]`。
5. **测试**：front `match_dependent_*`（含声明 binder 的 `nat_induction`）；
   CLI `cli_match_dependent_motive_checks_via_kernel`；课程 unit5 加依赖 match 节
   （`nat_induction` + 练习 8）；golden `(9,7,4)→(10,8,4)`、汇总
   `checked 53→54 / open 40→41`。
6. **验收**：`sokonanoda gate` PASS；版本 0.38.0 → **0.39.0**（新能力 minor）。
7. **已知限制**：motive 引用「类型为以箭头结尾的依赖函数」的 binder 时，
   `judge_infer` 的 render→parse 往返仍可能腐蚀 telescope（需 `judge.rs` 改
   一次性解析，或 `proof::render_expr` 给 domain 位 `Forall` 加括号）。

