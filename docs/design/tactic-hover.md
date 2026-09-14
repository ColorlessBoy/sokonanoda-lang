# 设计：tactic 关键字高亮 + hover 中间 goal state（2026-09-14）

> 触发：用户反馈「`exact` 没有正确高亮；希望能像 Lean 的 tactic 一样，在
> 每个 tactic 上 hover 都能看到中间 goal state（Infoview 式），或按鼠标位置
> 给 goal state」。

## 1. 问题

1. **tactic 关键字不大写高亮**：`crates/front/src/semantic.rs` 的 `KEYWORDS`
   只列了 `def/theorem/example/axiom/inductive/ctor/rec/iota/end/fun/intro/
   apply/#check/#reduce/#print`，`by`/`exact`/`assumption`/`rfl` 不在内 →
   语义着色当成普通标识符（`UnknownIdent`），看起来没高亮。`forall` 由
   `TokenKind::Forall` 单独着 Keyword、`sorry` 着 Hole，均已正确。
2. **没有中间 goal state 的 hover**：`soko/stateAt` 已经按光标返回「进入某
   tactic 的目标列表」（多目标显示轮加的 `goals[]`），但 `textDocument/hover`
   里没有接这条数据；学习者在 tactic 上悬停拿不到 Lean Infoview 式的提示。

## 2. 方案

### 2.1 关键字高亮（front）

`KEYWORDS` 增补 `by` / `exact` / `assumption` / `rfl`。它们本就是教学白名单
tactic（`by` 块），着色为 Keyword 与 `intro`/`apply` 一致。顺带让补全列表也
包含这些词（`semantic::keywords()` 是补全的单一来源）。

### 2.2 tactic hover（LSP）

`textDocument/hover` 的最前面（在关键字抑制之前，因为 tactic 词现在是
Keyword）接一条：光标落在某条 tactic 的 span 内 → 用该声明已有的
`by_steps` + `select_state_at`（进入态语义，与 `soko/stateAt` 同一份数据、
同一套选择规则）渲染：

```
a : Prop
h : And a a
⊢ And a a -> a
```

多目标时按 `soko/stateAt` 的 `goals[]` 顺序（当前在前）逐条列出，并标
`目标 i/n`。hover 的 range = 该 tactic 的 span（编辑器高亮整条 tactic）。

- 纯快照消费：零重编译、零文本扫描；`by_steps` 在 didChange 时已算好。
- 语义与 `soko/stateAt` 一致：进入某 tactic 的状态 = 上一条 tactic 执行后
  （第一条 = 根状态，目标 = 完整声明类型）。
- 因为目标状态 hover 已列出全部假设，tactic span 内的标识符类型 hover
  被它覆盖是可接受的（用户明确要 place-based goal state）。

### 2.3 明确不做

- 不改 `soko/stateAt`（客户端「当前光标处」面板仍走它）；
- 不做 webview Infoview（方案 B）；
- 不改 tactic 语义/kernel。

## 3. 测试

- front：`semantic::tests` 断言 `by`/`intro`/`exact`/`assumption` 均着
  `Keyword`；
- LSP：`hover_on_a_tactic_shows_the_entering_goal_state`——hover `apply`
  显示进入态目标 `⊢ And P Q`；hover `sorry` 显示 `apply` 开出的两个子目标
  `⊢ P` / `⊢ Q`。

## 4. as-built（2026-09-14）

- `semantic.rs` KEYWORDS +4；`hover()` 首插 `tactic_goal_hover`。
- 版本并入 **0.27.0**（CHANGELOG `Added`）；`docs/protocol.md` 增说明。
