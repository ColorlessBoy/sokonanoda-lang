# 派生 recursor 的 large-elimination 判据：逐字镜像内核（G-03 / WO-006）

> 状态：as-built（0.59.0 批次）。
> 台账：`docs/gaps/ledger.jsonl` 的 **G-03**；工作单
> `docs/gaps/WO-006-prop-type-param-inductive.md`；
> 复现件 `docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda`。

## 1. 缺口与根因

`inductive Bar (A : Type) : Prop` + `ctor mk (a : A) : Bar A` 被内核断言拒绝：

```
rejected: assertion `left == right` failed
  left: 1
 right: 0
```

根因**不在内核**，在 front 的派生判据（`crates/front/src/compile/elab.rs`
`derive_recursor`）：

```rust
let small_elim = is_prop_block_ty(ty) && constructors.len() > 1;
```

而内核的真值（`crates/kernel/src/inductive.rs`）是：

- `large_elim_test`：`is_nonzero`（结果排序在 `Type n`, `n ≥ 1`）⇒ `true`；
  否则看构造子数——**空块 ⇒ `true`**、**恰一个 ⇒ `large_elim_test_aux`**、
  **多构造子 ⇒ `false`**；
- `large_elim_test_aux`：跳过 `num_params` 个 binder，其余每个 Pi domain 若
  **不是 Prop 值**，就把它记下来；全部记下的项都必须是该归纳**结果应用
  （params ++ indices）的实参**之一（**语法**成员，见 §4）；全中 ⇒ `true`；
- `mk_elim_level`：`true` ⇒ `rec_uparams = [u] ++ uparams`；`false` ⇒
  `rec_uparams = uparams`。

`constructors.len() > 1` 只在"单构造子 Prop 且该构造子确实 large-eliminate"时
与内核巧合一致。判据写错 ⇒ 前端声明的 `uparams` 个数 ≠ 内核 `st.rec_uparams`
个数 ⇒ `assert_nonnested_recursors_def_eq` 里
`subst_expr_levels(old.info().ty, old.info().uparams, st.rec_uparams)` 的
`assert_eq!(ks.len(), vs.len())`（`crates/kernel/src/expr.rs:381-394`）炸成 panic。

## 2. 为什么不能"近似"

三条被实测钉死的反例（WO-006 的 P1–P14 对拍表）：

| 近似写法 | 立刻坏掉的形状 |
|---|---|
| "字段类型语法上是不是 `Prop`" | P10 `P -> Q`、P11 `forall (x : Nat), P`、P13 `Named`、P14 `Rel 0` —— 四个都**是** Prop 值，今天对，近似会把它们从 1 个宇宙参数改成 0 个 ⇒ 镜像方向的 `left:0/right:1` |
| "字段数 vs 参数数" / "有没有索引" | P3 与 P9 是判别性形状：都"看起来像"同一类，内核一个 `false` 一个 `true` |
| "试探 + 回退"（先按一支派生、被拒再换） | 等于把判据推给内核，把真正的教学错误磨成同一条断言，违反 `docs/architecture.md` §8 gotcha 0b 的契约（front 与内核必须**同规则镜像**） |

## 3. 本设计：判据**逐字镜像**内核，在构造子 elaborate 之后判定

### 3.1 时机：派生推迟到 ctor 类型 elaborate 之后（策略 A）

`large_elim_test_aux` 需要两样只有 elaborate 之后才有的东西：每个字段的**内核**
类型（判"是不是 Prop 值"）与结果应用的**实参表**（判子集）。因此
`install_inductive_block` 里把「派生 recursor」这一步**挪到 ctor 循环之后**：
ctor 循环已经为每个构造子算出内核 Pi 望远镜（`params ++ fields`），这正是内核
`large_elim_test_aux` 走的那条望远镜。

派生的**形状只依赖宇宙参数**，其余与最终声明逐字相同，所以挪动顺序对
`derive_recursor` 的产物没有影响——它只是晚一点拿到判据。

```rust
// ctor 循环收集每个构造子已 elaborate 的内核 Pi 望远镜
kernel_ctor_tys.push(ctor_ty);
// …循环之后：
let block_is_prop = is_prop_block_ty(ty);           // = 内核的 `is_zero`
let wants_u = large_elim_test_mirror(/* … */);      // = 内核的 `large_elim_test`
let (rec, rules) = derive_recursor(/* … */, wants_u);
```

### 3.2 镜像的三层结构

| front | 内核 | 规则 |
|---|---|---|
| `large_elim_test_mirror` | `large_elim_test`（`inductive.rs:1201`） | 非 Prop ⇒ `true`；空 Prop 块 ⇒ `true`；多构造子 ⇒ `false`；单构造子 ⇒ 下一层 |
| `large_elim_test_aux_mirror` | `large_elim_test_aux`（`inductive.rs:1164`） | 跳过 `num_params` 层，其余 domain 里非 Prop 的记下来；全部必须是结果应用实参的**语法**成员 |
| `field_type_is_prop` + `field_sort_via_kernel` | `is_prop_type`（`conv.rs:655`） | 字段**类型本身**是不是 `Sort 0` |

### 3.3 "是不是 Prop 值"必须问内核，不能看源码

字段类型的排序是**语义**问题。实测的四个反例（WO-006 的 P10/P11/P13/P14）都会
被"源码里写没写 `Prop`"判错：

| 字段类型 | 为什么是 Prop 值 | 源码近似会怎么错 |
|---|---|---|
| `P -> Q`（P10） | `imax(_, 0) == 0`（impredicativity） | 源码看不到 `Prop` 字样 ⇒ 误判 0 个宇宙参数 |
| `forall (x : Nat), P`（P11） | 同上，codomain 是 Prop | 同上 |
| `Named`（P13，`def Named : Prop := …`） | 具名定义，类型是 `Prop` | 源码只看到一个标识符 |
| `Rel 0`（P14，`def Rel (n : Nat) : Prop := …`） | 具名定义的应用 | 同上 |

于是 `field_sort_via_kernel` 用**内核**回答：`judge_infer` 合成
`#check fun <binders> => <字段类型>` 走完整流水线，取回内核渲染的排序文本，
再用已有的 `sort_text_level` 映射成 `Some(0)`（Prop）。这是**内核判定**，不是
文本比对——与 `match` 判 motive 层级用的是同一个 oracle
（[`infer_expected_level`]，`elab.rs`）。

一个例外走不了内核：字段类型引用**本块正在定义的归纳**时（`h : Bar A`），该名字
还没进 `judge_infer` 的前缀。这类字段按 Prop 处理是对的——调用方只在 Prop 块里
问这个问题，而 `Prop` 块的递归字段是证明、不是数据。

### 3.4 一次顺带修掉的 oracle bug

`judge_infer` 原来取事件流里**第一条** `TypeChecked`，但它的查询是**最后一条**
命令。文件前缀里只要已经有一条 `#check`（课程/playground 常见），取回来的就是
**旧查询**的答案。实测：前缀有 `#check Nat` 时，P10/P13 立刻退化成
`left:0/right:1`。修法：按 `event_cmds` 过滤出**最后一条命令**的事件
（不做文本比对），保留一条"最后一条 `TypeChecked`"的兜底。这条 bug 同样影响
`match` 的 motive 层级查询，属于既有缺陷、本轮一并修掉。

`derive_recursor` 收一个 `wants_u: bool`（= 内核的 `large_elim_test`）：

- `wants_u == true` ⇒ `universe = ["u"]`、motive 落在 `Sort u`；
- `wants_u == false` ⇒ `universe = []`、motive 落在 `Prop`。

其余（minor/IH/iota 规则/`rec_universe_arity`）自动跟随，无需改动。
`is_k_target` 与 `is_prop_block_ty` 一行不动（前者已正确且有测试，后者对本缺口
形状没有误判）。

## 4. 一个必须记住的内核事实：子集判据是**语法**的

`large_elim_test_aux` 末尾用的是

```rust
let (_, ind_ty_params_and_indices) = self.ctx.unfold_apps(self.arena, end_of_telescope);
non_prop_ctor_telescope_elems.iter().all(|arg| ind_ty_params_and_indices.contains(arg))
```

`contains` 比的是 `ExprPtr`（interned 指针 / 位相等），**不做 def_eq、不展开
定义**。实测（本仓 0.58.0 二进制）：

| 构造子结果 | 内核判据 | 结果 |
|---|---|---|
| `ctor ca (a : A) : PA A a` | `a` 与索引逐位相同 | 通过（1 个宇宙参数） |
| `ctor cb (a : A) : PB A (ident A a)`（`ident` 是 `def ident (A : Type) (a : A) : A := a`） | 索引是 `ident A a`，`a` 不在其中 | **`left:1/right:0`** |
| `ctor cc (a : A) : PC A (let x : A := a; x)` | 前端 elaborate 后索引就是 `a`（zeta 在 elab 期已做） | 通过 |

这正是"逐字镜像"的含义：镜像的是**内核那几行**，包括它对 `def` 不展开这件事。
本 WO 不改这一点（那是语言能力问题，不是判据问题）。

## 5. 对拍表（P1–P14 + Named/Rel，实测）

修前 **13 checked / 3 failed**（失败恰为 P1/P2/P3），修后 **16/16**。
逐条真值与前端派生结果见 WO-006 的边界表；实现后的回归在
`crates/front/src/compile/tests.rs` 的
`derived_recursor_universe_mirrors_the_kernel_large_elim_test`（表驱动，16 条声明）。

## 6. 与内核的契约边界

- 本设计**没有**改 `crates/kernel/` 的任何一个字节（冻结快照，`git diff
  crates/kernel/` 为空）。判据全部落在 front：`large_elim_test_mirror` /
  `large_elim_test_aux_mirror` / `field_type_is_prop` 是内核那几行的逐字镜像，
  "字段是不是 Prop 值"这一步则由 `judge_infer` 交给**真内核**回答。
- 于是 front 与内核的归纳块协作仍是**同规则镜像**：判据的**值**来自内核本身，
  没有第二条近似，也没有"试探 + 回退"（WO 明令禁止：那等于把判据推给内核，
  会把真正的教学错误磨成同一条断言）。
- 唯一"镜像"而非"询问"的部分是 §4 的**语法子集判据**：内核用 `ExprPtr`
  指针相等，front 用 arena 里的 hash-consed 指针相等——两边同为结构化相等，
  因为表达式在同一个 arena 里 intern。
