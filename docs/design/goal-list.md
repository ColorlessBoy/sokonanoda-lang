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

---

## 6. as-built：声明栏的**逐文件实测**（计划 T-B01 / T-B02，2026-09-21）

用户报「Infoview 的『声明』栏经常失效，有些文件有声明，有些没有，很奇怪，
比如几个 unit 教学文件就没有」。这类"有些…有些…"的抱怨**必须逐个文件量过**，
量具是 `scripts/verify-decl-panel.py`（起一个 LSP，逐个文件问 `soko/goals` 与
`textDocument/documentSymbol`，再与 CLI 的 `query goals` 对照）。

### 6.1 全量结果（86 个文件）

| 判定 | 个数 |
|---|---|
| **声明栏空但报告好**（= G-22 的现场） | **31** |
| 正常（`LSP goals == symbols`） | 54 |
| 真的没有声明（`lib/Logic`：纯注释空壳，prelude 自带同名 30 个） | 1 |

**31 个空的一览**（`goals=0` 而 `symbols>0`）：

| 目录 | 文件 | goals | symbols |
|---|---|---|---|
| `courses/set-theory/lib/` | `Demo` `Equiv` `Fun` `Image` `Rel` | 0 | 10 / 5 / 14 / 6 / 7 |
| `courses/set-theory/units/` | `unit01` … `unit12`（12 个） | 0 | 8–27 |
| `courses/set-theory/units/` | `notation-cheatsheet` | 0 | 23 |
| `courses/set-theory/units/solutions/` | `unit01-solution` … `unit12-solution`（12 个） | 0 | 5–30 |
| `courses/set-theory/units/solutions/` | `notation-cheatsheet-solution` | 0 | 21 |

**全部正常的**：`course/`（入门课 44 个文件）· `lib/{Exists,Prod,Set}`（这三个
不用 import 来的记法，单独 parse 成功）· `playground.sokonanoda`。

### 6.2 判据（为什么是这 31 个）

相关性是 **100%** 的，判据不是"在哪个目录"，而是：

> **单独 parse 失败 ∧ 闭包编译成功** ⇒ `QueryDoc::parsable()`（`query/mod.rs:403-408`）
> 为 `Err(NotParsable)` ⇒ LSP 侧 `.unwrap_or_default()`（`lsp/src/lib.rs:495`）
> 把错误吞成 `decls: []` ⇒ **声明栏空**。

同一份文档的 `documentSymbol`（走 `report`，G-20 修过）、`soko/stateAt`（目标栏）、
`hover`、`hints` **全都正常**——所以用户看到的是"**目标栏有、声明栏没有**"这种
不对称。`course/` 的文件不用 import 来的记法（逻辑连接符是 prelude 内建），
单独 parse 成功 ⇒ 全部正常。

### 6.3 同族受损：`nextHole`（T-B02，实测）

**四个 op 逐个量过**（`scripts/verify-decl-panel.py --ops`，2026-09-21）：

| 文件 | 单文件 parse | LSP goals | `nextHole` | `stateAt`（目标栏） |
|---|---|---|---|---|
| `units/unit01-sets-membership` | 否 | **0** | **无洞** | `mem_of_subset` ✅ |
| `units/solutions/unit01-solution` | 否 | **0** | **无洞** | （无独立 sorry 行） |
| `course/unit1-propositions-proofs`（无 import） | 是 | 13 | **有洞** ✅ | （无独立 sorry 行） |
| `lib/Set`（不用 import 记法） | 是 | 23 | 无洞（本来就没有） | — |

⇒ **`soko/nextHole`（`alt+n` 跳洞）与声明栏同生共死**：它经由
`next_hole → holes → goals(true)`，撞的是**同一条 `parsable()` 判据**；
而 `soko/stateAt`（目标栏）读 `report`、不经它 ⇒ **正常**。

这正是用户看到的不对称：**目标栏有、声明栏没有、`alt+n` 没反应**。

真宿主 e2e 用例 #2（`next hole jumps inside a project unit`）现在就是红的，
钉住这一条；它**故意不依赖声明栏**（不调 `infoviewDecls`），测的就是 `nextHole` 自己。

### 6.4 修后实测（T-B03，2026-09-21）

判据收敛成**一处** `QueryDoc::usable()`（`query/mod.rs:406`）：

```rust
fn usable(&self) -> bool {
    self.report.is_some() && (self.parse_error.is_none() || self.project_entry_compiled())
}
```

`goals`（经 `parsable()`）、`check` 的 parse 诊断闸门、LSP 的 `Doc::set_text`
**共用同一条判据**——G-22 的成因就是同一条判据写了两遍而 `goals` 漏了。

**修后（同一把量具，86 个文件）**：

| 判定 | 修前 | 修后 |
|---|---|---|
| **声明栏空但报告好** | **31** | **0** |
| 正常 | 54 | 85 |
| 真的没有声明 | 1 | 1 |

四层证据：

1. **front 单测** 2 条新增（`an_entry_whose_closure_compiles_is_usable_even_if_it_does_not_parse_alone`
   / `an_entry_whose_closure_also_fails_stays_not_parsable`——**后者钉住 G-17 契约不被放宽**），
   全量 672 passed；
2. **复现转绿**：`G22-lsp-goals-empty-for-import-entry.sh` → exit 1（`decls = 2`，
   与 `documentSymbol` 一致）；`gap.py check` 一致；
3. **真宿主 e2e**：用例 #1（声明栏）与 #2（`alt+n` 跳洞）**双双转绿**；
4. **量具**：31 → 0。

回归面：`sokonanoda-lsp --lib` 149 passed、`sokonanoda-cli` 的 `query`/`imports`/
`protocol`/`course` 全绿、课程门禁计数逐项不变。

### 6.5 复现与守护

* 复现：`docs/gaps/repro/G22-lsp-goals-empty-for-import-entry.{sh,js}`（判红）
* 量具：`scripts/verify-decl-panel.py`（本节的表就是它的输出；`--json` 可机读）
* e2e：用例 #1 `declarations panel lists a project unit's declarations`（判红）
* 修法见计划 **T-B03**：把判据收敛成单一 `QueryDoc::usable()`
  （`check()` 与 `Doc::set_text` 已经打了 G-20 的补丁，**只有 `goals` 漏了**）。
