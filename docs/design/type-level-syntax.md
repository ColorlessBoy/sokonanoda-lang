# `Type n` 记法（= `Sort (n + 1)`）

> 触发（2026-09-11）：学习者在画布写 `axiom Prop : Type 0`，内核报
> `rejected: expected a pi type, got: Sort(2)`。原因：本编译器只把单独的
> `Type` 当 `Sort 1`，没有实现 `Type n`，于是 `Type 0` 被读成「把 `Type`
> 作用到 `0`」。用户要求补上 `Type n`，与 Lean 的记法一致。

## 1. 语义（与 Lean 4 对齐）

Lean 里 `Type u` 是 `Sort (u + 1)` 的简写：

- `Type 0` = `Sort 1`
- `Type 1` = `Sort 2`
- 单独的 `Type` = `Sort 1`

内核打印时也按这套：`Sort 0` → `Prop`，`Sort (n+1)` → `Type n`。所以
hover 看到 `Type 0` 就是 `Sort 1`，两者指同一个项。

## 2. 设计

- **只加解析糖**：`parse_atom` 里 `Type` 后面紧跟一个数字时，消费它并把
  AST 记成 `SortKind::Sort(n + 1)`（复用现有 `SortKind::Sort`，不新增
  变体、不碰 elaborator 与内核）。`Type` 后面不是数字时保持原样 ——
  `Type`（`SortKind::Type`，即 `Sort 1`）。
- `Type u`（宇宙变量）**不支持**：`SortKind::Level(name)` 只能表达
  `level = u`，表达不了 `u + 1`；需要时写 `Sort u`。这不是新问题，课程
  里一直用 `Sort u`。
- 数字溢出用 `checked_add`，报 `Sort expects a universe level` 同族诊断，
  不 panic。

## 3. 三件套

1. **课程**：`course/unit4-universes-sort.sokonanoda`（+ `course/en/` 镜像）
   在宇宙小节写清 `Type n = Sort (n+1)`，并加一条 `#check (Type 0)`；
2. **测试**：front 解析单测 + front 编译单测 + CLI e2e（`#check Type 0`）；
3. **白名单/文档**：`docs/architecture.md` Expr 说明、`docs/teaching-session.md`
   gotcha（原「`Type 1` 不是合法输入」改为「`Type n` 合法」）。

## 4. 验收标准

1. `axiom Foo : Type 0` 通过内核；`#check Type 0` 的内核结果为 `Type 0: Type 1`
   （即 `Sort 1 : Sort 2`）；
2. 单独的 `Type` 仍是 `Sort 1`（`inductive Nat : Type` 不受影响）；
3. `Type -> Type`、`(x : Type)` 等旧用法零回归；
4. 普通名字零 behavior 变化；`Type u` 仍按现有（不支持）行为对待；
5. 课程 zh/en 事件计数仍相等；`sokonanoda gate` 全绿。
