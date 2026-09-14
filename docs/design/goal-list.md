# 设计：多目标显示（goal list）—— by 块的全部剩余目标（2026-09-14）

> 触发：用户在画布上写
> `theorem forall_and … := by apply And.intro; intro x; sorry`，
> 疑问「`intro x` 之后应该同时看到 `P x` 和 `(x : Person) -> Q x` 两个目标，
> 目前只显示了一个」。判定逻辑没错（`apply` 确实开出两个 `forall` 子目标），
> 错在**数据**：引擎每步只记录 worklist 栈顶那一个目标，其余目标在编译期就
> 被丢掉了，UI 无从显示。

## 1. 根因

- `crates/front/src/by.rs`：`run_by` 用 `nodes + worklist: Vec<usize>` 建模
  目标树（worklist 末尾 = 当前目标），但每步只把 `worklist.last()` 渲染进
  `ByStep { goal: Option<String>, binders }`（`by.rs:247-261`）；完整 worklist
  是局部变量，`run_by` 返回时消失（`by.rs:264-266`）。
- 这条单目标一路透传到 `ByStepState.goal`（`report.rs:61`）、
  `soko/stateAt.goal` 与 `soko/goals.goal`（`lib.rs:392, 500-512`）。
- VS Code「当前光标处」组每个目标只渲染一个 `cursor.goal`
  （`editor/vscode/extension.js:243-269`）。
- `soko/goals` 的 `holes`/`sub_goals` 是**按 `sorry` 分组**的期望类型，不是
  按 tactic 排序的活跃目标；多个子目标共用一个 `sorry` 位置时还会混淆
  （`docs/protocol.md:376-390`）。

## 2. 方案（front 记录全量，协议透传，客户端渲染）

### 2.1 front：`ByStep` 记录全部开放目标

```rust
pub struct ByGoal { pub ty: String, pub binders: Vec<Binder> }
pub struct ByStep { pub span: Span, pub goals: Vec<ByGoal> }
```

- 每步执行后，`goals` = 当前 worklist 里**所有**未闭合目标的 `(ty, 上下文)`，
  **当前目标在首位**（`worklist.iter().rev()`，末尾=当前）；
- 全闭合 → `goals` 为空；
- 每个目标的 `binders` = 沿父链收集的 intros（不同子目标上下文可以不同）。

`ByStep` 里旧的单 `goal`/`binders` 字段删除（避免冗余分叉），由 `goals[0]`
表达「当前目标」；`report.rs` 的 `ByStepState` 同步为
`{ span, goals: Vec<ByGoalState> }`。

### 2.2 协议：`soko/stateAt` 返回目标列表

新增 `goals: [{ "goal": "<ty>", "binders": [...] }]`（当前目标在首位）。保留
单值 `goal` / `binders`（= `goals[0]`）供旧客户端兼容；`goal: null` 仍表示
该处已无剩余目标。

`soko/goals` 的每个声明新增 `goals: [String]`：有 `by_steps` 时取最后一步的
目标列表，否则取该声明走查到的单个剩余目标（非 by 的 Open 练习）。

### 2.3 客户端：VS Code「当前光标处」

`buildCursorChildren` 遍历 `cursor.goals`（缺省回退到单个 `cursor.goal`）：
- 1 个目标：保持现状（「目标」+ 假设平铺）；
- 多个目标：每个渲染成「目标 i/n」可展开节点，各自挂自己的假设。

### 2.4 客户端：光标移动不重取目标（性能）

原实现每次光标移动（去抖 200ms）都 `refresh()` → 整棵树失效 →
`getChildren(undefined)` → `soko/goals` 全量拉取 + 重建所有练习 TreeItem
（`extension.js` 旧 `refresh/getDeclarations`），文件一大就卡。改为：

- `refresh()`（诊断 / 切文件）才丢弃并重取 declarations；
- 光标移动走新的 `refreshCursor()`：**复用缓存的 `declItems`**，只重建
  「当前光标处」组；`soko/stateAt` 本身仍只查缓存快照，无网络放大。

### 2.5 明确不做

- 不改 tactic 语义、不动 kernel；引擎只多发一份数据；
- 不做 `soko/nextHole` 的 goal-wise 导航（多子目标同源位置的限制仍在，
  见 `docs/protocol.md:376-390`）；
- webview goal 面板（方案 B）仍不在本轮。

## 3. 测试（三层）

- **front**：`by` 记录多目标（`apply And.intro` → 两个目标、当前在前）；
  单目标/闭合两旧用例改为读 `goals`；`soko/stateAt` 的 front 形状；
- **CLI e2e**（`crates/cli/tests/extension.rs`）：契约断言客户端消费
  `cursor.goals` 并渲染多个目标节点；
- **LSP protocol**（`crates/lsp/src/lib.rs` 单测）：`apply` 后 `stateAt.goals`
  长度与顺序、`goals[0]` == 单值 `goal`、`soko/goals` 的 `goals` 数组。

## 4. 验收

- `forall_and` 例子：光标在 `intro x` 之后看到 `["P x", "forall (x : Person), Q x"]`
  （渲染文本以内核 pretty-printer 为准）；
- 旧客户端只读 `goal` 不回归；`goal`/`binders` 与 `goals[0]` 恒等；
- `cargo fmt/clippy/test` 全绿（`sokonanoda gate` PASS）；
- `docs/protocol.md`、`REQUIREMENTS.md §9`、`STATUS.md` 同步；扩展版本
  0.26.0 → 0.27.0。

## 5. as-built（2026-09-14）

- front：`ByStep { span, goals: Vec<ByGoal> }`、`ByGoal { ty, binders }`；
  `by_step_states` 同步；旧 `goal`/`binders` 字段移除。
- LSP：`soko/stateAt` 增 `goals: [{goal, binders}]`（`goal`/`binders` 保留、
  等于 `goals[0]`）；`soko/goals` 的 `GoalDeclInfo` 增 `goals: [String]`。
- 扩展：`buildCursorChildren` / `buildOpenChildren` 渲染多目标（>1 时
  编号节点、各自假设）；光标移动走 `refreshCursor` 复用缓存的 `declItems`，
  不再每次 `soko/goals`（性能，用户报告「vscode 很卡」）。
- 版本 0.26.0 → 0.27.0（Cargo workspace + VSIX + CHANGELOG）。
