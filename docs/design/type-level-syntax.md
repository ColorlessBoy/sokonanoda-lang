# `Type n` 记法（= `Sort (n + 1)`）与层级算术 `u+1`

> **状态：已实现**（`crates/front/src/parser.rs` 的 `Type n` → `Sort (n+1)`；
> §5 的层级算术 `u+1` 于 0.61.0 落地）。

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
- `Type u`（宇宙变量）**不支持**（`Type (u+1)` 支持，见 §5）：`Type` 后跟
  **裸标识符**必须保持**应用**语义——`Eq.refl.{2} Type A`（`Type` 作实参、
  紧跟另一个实参）是既有写法，把 `Type A` 读成一个层级会静默改变它的含义。
  需要时写 `Sort u` / `Sort (u+1)`。这不是新问题，课程里一直用 `Sort u`。
- 数字溢出用 `checked_add`，报 `Sort expects a universe level` 同族诊断，
  不 panic。

## 3. 三件套

1. **课程**：`course/unit5-universes-sort.sokonanoda`（+ `course/en/` 镜像）
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

## 5. 层级算术 `u+1`（0.61.0 落地；L-03 的 B8 扩族依赖它）

> 触发：`docs/design/eq-type-level-rewriting.md` §4-1——宇宙多态的 `Eq.mp` 要
> `h : @Eq.{u+1} (Sort u) α β`，而层级语法没有加法（0.60.0 实测 parse 错
> `expected , or } in universe arguments, found Plus`）。同轮把它做掉，
> B8 的 `Eq.mp`/`Eq.mpr`/`cast` 才能与 Lean core 的签名逐字对齐。

### 5.1 语法面（白名单 = 本小节）

**层级** = 原子 (`+` 数字)*；**原子** = 数字 | 标识符 | `(` 层级 `)`。
括号可省（`Sort u+1` ≡ `Sort (u+1)`），空白被规范化掉（文本记成 `"u+1"`）。
出现位置：

| 位置 | 例 | AST |
|---|---|---|
| `Sort` 后 | `Sort (u+1)`、`Sort u+1`、`Sort 3`、`Sort u` | `SortKind::Level("u+1")` / `SortKind::Sort(3)` / `SortKind::Level("u")` |
| `Type` 后 | `Type 2`、`Type (u+1)`（= `Sort (u+1+1)`） | `SortKind::Sort(3)` / `SortKind::Level("u+1+1")` |
| 宇宙实参 | `Eq.{u+1}`、`@Eq.rec.{u+1, u}` | `UniverseApp.levels = ["u+1", "u"]` |

实现：`parser.rs::parse_level_text`（语法 → 层级**文本**；AST 的层级槽位本来就
是文本）+ `elab.rs::level_ptr`（文本 → 内核层级，用 `EnvBuilder` 的公开
`zero`/`succ`/`level_param`）。`proof.rs::render_expr` 给带 `+` 的层级加括号
（`Sort (u+1)`），render→parse 往返无歧义。**内核零改动**（硬规则 1）。

### 5.2 明确不做（两条边界，都有实测）

1. **`+` 右边只收数字**（Lean 的 `u+n` 形式）：`u+v` / `max u v` / `imax` 要内核的
   `Level::Max`/`IMax`，而 `EnvBuilder` 只公开 `zero`/`succ`/`level_param`——
   做它必须动内核。parser 报专用诊断
   （"层级加法只收数字后缀（例如 `u+1`）；`max`/`u+v` 不在本语言的层级语法面内"），
   不静默吞。
2. **`Type u` 仍不支持**（`Type (u+1)` 支持）：见 §2 的应用歧义。
   内核把 `Sort (u+1)` 打印成 `Type u`（`pretty_printer.rs` 的 `pp_sort`），
   所以**内核渲染文本**里的 `Type u` 回读会落成应用——本语言的目标文本走
   `render_expr`（源码 AST 渲染，层级文本带括号），不走内核打印，故不受影响；
   这是"不用内核打印做目标文本"的既有约定的一个理由（写进本小节备查）。

### 5.3 三件套

1. **课程**：`course/unit5-universes-sort.sokonanoda`（+ `course/en/` 镜像）
   宇宙小节的注释补 `u+1` 与 `Eq.{u+1}`（注释不改 golden 事件计数）；
2. **测试**：parser 单测 3 条（`level_arithmetic_parses_in_sort_and_universe_args`、
   `level_arithmetic_keeps_plain_forms_byte_identical`、
   `level_arithmetic_rejects_non_numeric_suffix`）+ front 编译单测
   （`eq_mp_is_universe_polymorphic`、`eq_rec_transports_at_type_level`）+ 复现件
   `docs/gaps/repro/L03-eq-type-level.sokonanoda`（10 checked · 0 diagnostic）；
3. **白名单/文档**：本小节 + `docs/design/eq-type-level-rewriting.md` §4 +
   `docs/design/prelude-l1-proposal.md` 的 B8 扩族补记。
