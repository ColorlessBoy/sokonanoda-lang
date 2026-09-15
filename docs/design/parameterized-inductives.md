# 设计：参数化归纳声明（非带索引，2026-09-14）

> 触发：`match` 参数化归纳的前置——教学语言的 `inductive` **声明**目前不支持
> 参数（`inductive Nat : Type`；前端给内核传 `num_params=0`），而内核
> `add_inductive` 已支持 `num_params`。本设计加**非带索引**的参数化归纳，
> 解锁 `Option A` / `List A`，进而 `match` 它们。kernel 冻结。

## 1. 语法与 AST

```lean
inductive Option (A : Type) : Type
ctor none : Option A
ctor some (a : A) : Option A
end
```

- `parser.rs::parse_inductive_block`：在名字之后、`:` 之前解析零或多个 binder
  （复用 `parse_binder`，支持 `(A : Type)` / `{A : Type}`）→ 新
  `InductiveBlock.params: Vec<Binder>`；params 对 `ty`、每个 ctor 类型、显式
  `rec`/`iota` 均在作用域内。
- `CtorDecl.binders` **只放字段**，不含 params（字段计数依赖这一点）。
- 需要同步 destructure 的消费者：`semantic.rs`（params 入 binder 表 + 走查类型）、
  `goals.rs`、`warning.rs`、`session.rs`、`parser.rs` 测试访问器。

## 2. 安装路径（`elab.rs::install_inductive_block`）

- 归纳类型 = `forall params, ty`（params 压入 `ElabScope` 后 elaborate）。
- `add_inductive(..., num_params = params.len() as u16, num_indices = 0, ...)`。
- ctor 类型 = `forall (params ++ ctor.binders), ctor.result`；
  `ConstructorData.num_params = params.len()`；
  `num_fields = ctor_field_binders(ctor).len() − params.len()`。
- recursor `RecursorData.num_params = params.len()`；
  iota `ctor_telescope_size_wo_params = 字段数（不含 params）`。
- `InductiveInfo` 增 `num_params`（及 param 名/源类型），供 `match`。

## 3. `derive_recursor`（无显式 rec 时）

按内核 `mk_recursor_aux` 的期望形状（`inductive.rs:1727-1777`）：
- params **最外层**（沿用归纳声明的 binder 风格，顺序/类型/个数必须一致）；
- `motive : (t : Ind params) -> Sort u`；
- minors 按构造子声明序：minor 类型 = `(fields + IHs) -> motive (C params fields)`；
- iota 值 lambda 序：`params, motives, minors, ctor args`；自调用
  `Ind.rec params motive minors… rec_arg`。
- `taken` 卫生集合加入 param 名。

> 内核会**重建**递归子并 `def_eq` 断言（`assert_nonnested_recursors_def_eq`）+
> 元数据断言（`check_declared_metadata`：`num_fields == telescope − num_params`，
> `inductive.rs:646-655`）。字段计数与 params 位置是最易错点。

## 4. `match`（参数化归纳）

- 降低为 `Ind.rec.{level} params motive minors scrutinee`。
- **params 取值**：scrutinee 必须是**有书写源类型的局部变量**（`scope.src_ty`），
  取其类型头应用的前 `num_params` 个源实参作为 params；据此在
  `MatchField.src_ty` 里做 params 名替换后 elaborate 字段类型。
- 拿不到 params（scrutinee 不是带参数类型的局部量）→ 干净报错
  `elab-match-parameterized-unsupported`（新码，protocol + 穷尽清单）。
- 非依赖 motive（与现行一致）：motive = `fun (_ : Ind params) => R`。

## 5. 明确不做（v1）

- ~~**带索引**归纳（`num_indices > 0`）~~ ✅ 已落地（0.47.0，见 `indexed-inductives.md`）、依赖 motive、嵌套/互递归、宇宙多态参数
  （`{u}` 级参数）、`match` 的嵌套/守卫/字面量模式；`Nat`/`Eq` prelude 仍
  `num_params=0`（原生快路径已按 `num_params` 泛化，不回归）。

## 6. 测试三层

- **front**：parse（params + ctor + rec + iota）；compile：`Option A` 派生递归子
  可 `#check`/`#reduce`；显式 `rec`/`iota`（params 最外层）通过内核；错位/漏参数
  被内核拒绝（元数据断言）；`match` on `Option A`（`some`/`none`，params 代入
  字段类型；`some a` 的 `a : A`）；scrutinee 非参数化局部量 → 新错误码。
- **CLI e2e**：含参数化归纳 + `match` 的文档 `decl.checked`/`exercise.open`。
- **课程**：unit5 或单元⑦？——放 unit5「归纳与递归」续篇：`Option`/`List` 演示 +
  练习（zh/en/钥匙 + golden）。
- **白名单**：`architecture.md §4.1` 补「归纳可带参数」。

## 7. 验收

- `inductive Option (A : Type)` 及其派生递归子在完整内核通过（`#check`/`#reduce`）；
- `match` on `Option A` 正确（params 代入）；不支持形状报干净错误；
- kernel 零改动；`gate` 全绿；课程/golden、文档、STATUS/REQUIREMENTS §9 同步；
- 版本 **minor**（新语法）。

## 8. 风险

| 风险 | 缓解 |
|---|---|
| 字段计数含/不含 params 错一位 | 严格 `num_fields = telescope − num_params`；元数据断言会抓 |
| ctor 结果必须是 `Ind params`（顺序/参数为 Var） | 生成 `forall params, fields, C params`；测试覆盖 |
| recursor params 位置/风格与内核重建不一致 | 照 §3 顺序；`def_eq` 抓；先做单参数 `Option` |
| `match` 字段类型需 params 代入 | 用源类型实参替换 param 名；拿不到则干净报错 |

---

## 9. as-built（2026-09-14，0.38.0）

- **语法/AST**：`InductiveBlock.params: Vec<Binder>`；parser 在名字后解析
  `(A : Type)`/`{A : Type}`；`CtorDecl.binders` 仅字段。
- **安装**：归纳类型 `forall params, sort`；ctor `forall (params++fields), C params`；
  `add_inductive(num_params=params.len(), num_indices=0)`；`num_fields` 用
  字段-only 计数（= `pi_telescope_size(ctor.ty) − num_params`）；recursor/iota 元数据
  的 `num_params`/`ctor_telescope_size_wo_params` 同步。
- **derive_recursor**：params 最外层（沿用声明风格）、motive `(t : Ind params) ->
  Sort u`、minors `(fields+IHs) -> motive (C params fields)`、iota lambda 序
  `params, motives, minors, args`、自调用带 params、param 名入 hygiene。
- **match**：`Ind.rec.{level} <params> motive minors scrutinee`；params 取
  scrutinee **书写源类型**头部实参；字段 `src_ty` 做 params 名替换后 elaborate；
  拿不到 → 新码 `elab-match-parameterized-unsupported`（protocol + 穷尽清单）。
- **测试**：front +8（parse 2 / compile 6：Option 派生递归子、显式 rec/iota、
  iota 错 → `kernel-rec-rule-mismatch`、`match` on `Option Nat`、无参数源类型报错、
  `List` 递归 + match）；CLI +4；课程 unit5 加 `Option` 小节 + 练习 7；
  golden `(7,6,3)→(9,7,4)`、汇总 `checked 51→53 / open 39→40`。
- **文档**：`architecture.md §4.1/§8`、`TESTING.md`。
- **版本** 0.37.0 → **0.38.0**（新语法 minor）。
- **v1 边界**：带索引归纳、宇宙多态参数、互/嵌套递归、`match` 嵌套/守卫/字面量仍不做。
