# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第五十一轮：tactic 关键字高亮 + hover 中间 goal state；同版含多目标显示与移除 funintro，0.27.0）
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

## 本轮进度（2026-09-14，第五十一轮：tactic 关键字高亮 + hover goal state）

> 用户反馈：`exact` 没有正确高亮；希望像 Lean 一样在每个 tactic 上 hover 看到
> 中间 goal state（Infoview 式），或按鼠标位置给 goal state。

1. **设计** `docs/design/tactic-hover.md`。
2. **高亮**：`semantic::KEYWORDS` 增补 `by`/`exact`/`assumption`/`rfl`（此前
   当普通标识符着色；`forall`/`sorry` 已由 token/Hole 正确处理）。
3. **hover**：`textDocument/hover` 首插 `tactic_goal_hover`——光标落在某 tactic
   span 内 → 用 `by_steps` + `select_state_at`（进入态语义，与 `soko/stateAt`
   同数据同规则）渲染全部目标与假设（多目标标 `目标 i/n`），range = 该 tactic；
   纯快照消费，零重编译零文本扫描。
4. **测试**：front semantic 断言四关键字均 Keyword；LSP
   `hover_on_a_tactic_shows_the_entering_goal_state`（hover `apply` → `⊢ And P Q`；
   hover `sorry` → `⊢ P` / `⊢ Q`）。
5. **协议**：`docs/protocol.md` 新增「Tactic goal-state hover」小节；并入 0.27.0。
6. **验收**：`sokonanoda gate` PASS。

## 本轮进度（2026-09-14，第五十轮：移除值位关键字 funintro）

> 用户评估：「`funintro` 跟 `funapply` 一样，实现起来稀里糊涂的，不如直接删了。」
> 确认按「彻底删」执行，与多目标显示并入 0.27.0。

1. **设计先行** `docs/design/remove-funintro.md`（根因/方案/测试/验收/as-built）。
2. **前端**：删 `Expr::Intro`、`parse_intro`/原子位/lambda 尾关键字分支、
   `KEYWORDS` 的 `funintro`、`compile/intro.rs`、`DeclState.intro_skeleton`、
   `ErrorKind::ElabIntroNotAFunction`；各 crate 匹配臂与测试同步。
3. **协议/客户端**：`soko/stateAt` 的 `goals` 相关不受影响；删 LSP 值位关键字
   补全/hover/code action/inlay 全路径、VS Code `sokonanoda.expandIntro`
   命令与 `markdown.isTrusted` 白名单；`docs/protocol.md` 值位关键字小节改为
   「已移除」说明。
4. **课程**：unit6（zh+en）改写「补充 funintro」段 + 练习 6 为综合 `by` 练习，
   钥匙同步；golden 计数不变。
5. **文档/site/技能**：architecture/README/TESTING/ROADMAP(I13 标废弃)/
   term-intro/value-keywords-v2(废弃横幅)/site hero 换图/gen-site-demos 删演示/
   teacher 技能表同步；历史归档与 CHANGELOG 原文保留。
6. **测试**：`cli_value_funintro_is_no_longer_a_keyword` 钉「已非关键字」；
   其余 funintro 测试全删。
7. **验收**：`sokonanoda gate`（fmt/clippy/test/playground 锚点）。

## 本轮进度（2026-09-14，第四十九轮：多目标显示）

> 用户报告：画布上 `apply And.intro; intro x` 之后应同时看到 `P x` 和
> `(x : Person) -> Q x` 两个待证目标，目前只显示一个。用户确认按完整流程修。
> 判定逻辑本就正确（`apply` 确开两个 `forall` 子目标），缺陷在**引擎数据**：
> `ByStep` 每步只记 worklist 栈顶目标，其余目标编译期即丢。随后用户报告
> VS Code 目标视图很卡，同轮修刷新路径。

1. **设计先行** `docs/design/goal-list.md`（根因 / 方案 / 测试 / 验收 / as-built）。
2. **前端**：`ByStep` 改为 `{ span, goals: Vec<ByGoal> }`，`ByGoal = { ty,
   binders }`；每步记**全部**未闭合目标（当前在首位，各带自己的假设链），
   闭合则为空。`by_step_states` / `ByGoalState` / re-export 同步。
3. **协议**：`soko/stateAt` 增 `goals: [{goal, binders}]`；`soko/goals` 每
   声明增 `goals: [String]`（by 声明取最后一步、非 by 取走查目标）。
   单值 `goal`/`binders` 保留且恒等于 `goals[0]`，旧客户端不回归。
4. **客户端多目标**：VS Code「当前光标处」与练习节点遍历多目标——>1 时渲染
   `目标 i/n` 可展开节点、各自挂假设；=1 保持现状。
5. **客户端性能**（用户报告「vscode 很卡」）：原实现每次光标移动都
   `refresh()` → 重取 `soko/goals` + 重建全部练习 TreeItem。改为
   `refresh()` 只处理诊断/切文件，光标移动走 `refreshCursor()` 复用缓存
   `declItems`，只重建光标组。
6. **测试三层**：front `apply_records_all_open_goals_current_first`（+改两旧
   用例读 `goals`）；LSP `state_at_lists_all_open_goals_after_apply` +
   `goals_request_lists_every_open_goal_after_apply`；CLI 契约
   `extension.rs` 钉客户端消费 `cursor.goals`/`decl.goals` 与 `declItems`/
   `refreshCursor` 缓存纪律。
7. **版本** 0.26.0 → **0.27.0**（Cargo + VSIX + CHANGELOG Added）；`docs/protocol.md`
   两节 + 已知限制小节、`REQUIREMENTS.md §9` 同步。
8. **验收**：`sokonanoda gate` PASS（fmt / clippy / test / playground 锚点）。
