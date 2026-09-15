# 设计：带索引归纳（`num_indices > 0`，2026-09-15）

> 触发：`docs/design/parameterized-inductives.md` §5 明确列为 v1 不做；HANDOVER
> §3 B 余项「带索引归纳」。目标：`inductive Vec (A : Type) : Nat -> Type` 可声明
> （参数 + 索引）、可派生 / 显式给出 recursor、可 `match`（常量结果类型）。
> **kernel 冻结**：内核本就支持 `num_indices`，本设计只补前端契约。

## 1. 索引的定义（内核契约）

内核把「`ty` 在 `num_params` 之外的 Pi 望远镜」当索引，残余必须是 Sort
（`crates/kernel/src/inductive.rs::check_inductive_spec_0th`）。构造子结果必须是
`Ind params indices` 的完整应用（params 按声明序为绑定变量；索引可任意但不含
自身递归出现，`which_valid_ind_app_v`）。recursor 形状固定：

```
Ind.rec : forall (params…), forall (motives…), forall (minors…), forall (indices…),
          (major : Ind params indices) -> motive indices major
motive  : forall (indices…), Ind params indices -> Sort
minor_i : forall (ctor 字段…, ih…), motive <ctor 的索引实参> (C params 字段…)
major_idx = num_params + num_motives + num_minors + num_indices
```

前端派生的 recursor 会被内核按 `def_eq` 重建比对（名字/风格无关），iota 规则
顺序/数量/形状也被重建比对。

## 2. 落地（as-built）

**声明安装**（`elab.rs::install_inductive_block`）：
- `index_binders = result_chain_binders(ty)`（`ty` 在 params 之外的 Pi 望远镜）；
  `num_indices = index_binders.len()`（u16 越界报 `elab-too-many-binders`）。
- `add_inductive(…, num_params, num_indices, …)`；`RecursorData.num_indices` 同步。
- `InductiveInfo` 增 `num_indices` + `index_types`（供 match 组 motive）。
- `is_prop_block_ty` 先剥索引望远镜再判最终 Sort（否则带索引的 Prop 块会被误判）。

**派生 recursor**（`derive_recursor`）：
- 索引给新鲜名字；`Ind params i1…ik` 贯穿 motive/目标/索引实参。
- motive = `forall (indices…), (x : Ind params indices) -> Sort`。
- ctor 字段名 → 派生名的替换同时作用到**字段类型**与**构造子结果索引实参**
  （索引可引用字段，如 `Vec A n`）；递归字段的索引实参取其 codomain 的 spine
  （`spine_of_codomain`，支持 `(x : Nat) -> Vec A x` 这类递归出现）。
- rec 绑定序 `params → motive → minors → indices → target`；结果 `motive indices target`。
- iota：自调用携带字段索引实参。

**match**（`Expr::Match` 分支）：
- 从 scrutinee 的书写类型取索引实参（params 之后），elaborate 成 `index_kernel`。
- motive 先绑索引再绑 major（索引类型代入参数实参；索引名新鲜）；major 的书写源
  类型 = `Ind <参数实参> <索引名…>`。
- recursor 应用 = `Ind.rec params motive minors index_kernel scrutinee`。
- **字段改名**：字段类型可能引用前面的字段（`v : Vec A n`），而用户 match 绑定
  名可能不同（`| vcons a m w =>`）——按「字段原名 → 用户绑定名」做 substitution
  后再 elaborate（否则 de Bruijn 指错；这也是带索引归纳暴露出的既有 latent bug）。

## 3. v1 边界（明确不做）

- **结果类型依赖索引**（如 `P n` 随 `v : Vec A n` 精化）：当前 motive 只抽象
  major，无法表达索引精化 → 内核会 sound 地拒绝；如需请另立设计。
- 索引本身含递归出现、相互/嵌套递归、宇宙多态参数、显式 `rec`/`iota` 的手写
  便利（可写但需与内核形状对齐）不做保证。

## 4. 测试 / 课程

- front `indexed_vec_checks_and_derives_recursor`、`match_on_indexed_vec_computes_with_a_constant_motive`、
  `match_field_types_follow_the_user_binder_names`；CLI `cli_indexed_vec_checks_and_reduces`。
- 课程 unit5 增「带索引归纳 Vec」节 + 练习 10（zh/en + 两份 solutions；golden
  `(11,9,6) → (13,10,7)`，汇总 `checked 55→57 / open 42→43`）。
- 白名单：`docs/architecture.md §4.1`。
