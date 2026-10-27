# 设计：高阶 / pattern 合一（#16）—— metavar 引擎的 **HO 线**（**设计先行，未实现**）

> 日期：2026-10-25。**用户拍板**：「metavar 要做，要做好设计，先设计方案再执行」——
> 本文是 `docs/design/metavar-engine.md` §1.2 表 **#16** 的设计；**在 HO4 验收前不改判定代码**。
> **触发现场**（2026-10-25 unit02，用户亲测）：
>
> ```text
> theorem eq_empty_iff_forall_notMem (α : Type) (A : Set α) :
>     (A = ∅) ↔ (∀ (x : α), ¬ (x ∈ A)) := by
>   constructor
>   intro h x
>   apply Eq.subst (Eq.symm h)      -- 期望：目标变 ¬ (x ∈ ∅)；今天：elab-tactic-failed
> ```
>
> 用户追问「原理上为什么不行 / Lean 4 可以吗」⇒ 结论：**原理上可以，Lean 4 可以**（§1）；
> 今天不行是**实现边界**（§2），本设计把它补上。

## 0. 一句话

给 `MetaCtx`/`MetaStore` 加一条 **pattern 赋值规则**（`?m a₁…aₙ =?= t` ⇒ 抽象成 `fun a₁…aₙ => t`，
带类型检查），并让 `apply` 在**剥穿 `Not` 之前**先拿未展开的结论与目标对齐；
两层合起来，`apply Eq.subst (Eq.symm h)` 与 Lean 4 同判（留下子目标 `¬ (x ∈ ∅)`）。

## 1. 事实：Lean 4 会接受（源码对照）

- `apply` 不求值期望类型地 elaborate 项（`Elab/Tactic/ElabTerm.lean:294`）；`MVarId.apply` =
  `forallMetaTelescopeReducing` + `isDefEqApply`（`Meta/Tactic/Apply.lean:160/169`）；
  `ApplyConfig.approx := true`（`Init/Meta/Defs.lean:1601`）⇒ `approxDefEq` 打开
  `fo/ctx/quasi`（`Meta/Basic.lean:2050`）。
- 合一器对 `?m a₁…aₙ =?= t` 有 pattern 解（严格条件 `Meta/ExprDefEq.lean:523`；参数在 lctx 里时
  `:1199` 允许近似，`:1180` 抽象 + type check）。
- 本例：`?p A =?= ¬ (x ∈ A)` ⇒ `?p := fun X => ¬ (x ∈ X)` ⇒ 子目标 `?p ∅` = `¬ (x ∈ ∅)`。
  常量解 `fun _ => ¬ (x ∈ A)` 也满足方程（留下循环目标）⇒ **必须取最一般解**（§3.2）。
- 反面：高阶合一一般**不可判定**（Huet）⇒ 只做 pattern 片段 + 有界近似。

## 2. 今天为什么不行（两层，分开修）

**2.1 表面匹配 vs 定义相等**（本例的直接原因）：`spine::peel_pi_delta` 把 `Not` 看穿成
`P → False` ⇒ 项的结论变 `False`；目标仍是 `Not (…)`；`unify_spine` 是源级表面匹配
（同头 + 实参位），不调内核 `def_eq` ⇒ 对不上（by.rs 已记这条偏离）。

**2.2 谓词合成**：`Eq.subst` 的 `motive` 是隐式元变量；今天 `apply` 从期望类型合成（5 实参已对），通用形态需要 pattern 合一。

## 3. 引擎设计

### 3.1 表示（扩展 `MetaStore` / `MVar`）

- `MVar` 今天只有 `ty`/`value`/`kind`/`out_of_scope`（**负向**作用域）⇒ 本片补**正向**
  `lctx: Vec<LocalBinder>`（`name`+`ty`+`is_let`，从调用点 binder 表来；`depth` 在 `MetaCtx`，不是每 mvar 字段）。
- 本语言的「fvar」= **局部名**（`Expr::Ident` 且在 lctx 里、非 let）；**不加新 AST 变体**（仍是 `\0soko_m{id}` 名字编码）。
- 作用域判据沿用 #3（`v` 不许提到 lctx 之外的名字）。

### 3.2 `pattern_assign(m, args, t) -> Tri`

1. `zonk` 两侧；`args` 必须是**互异的局部名**（在**当前目标上下文**里、非 let；**不要求**在 `lctx(m)` 里）；否则 ⇒ `Undef`（交待定/FO）。
2. **严格 pattern**（Lean 条件 1–5）：`t` 的自由变量 ⊆ `lctx(m)` ∪ `args`；occurs / type-occurs
   （#5）；`args ∩ lctx(m) = ∅`（条件 2）⇒ `v := fun args => t`，**type check**（§3.3）⇒ `assign`。
3. **quasi / FO / ctx 近似**（对齐 Lean `approxDefEq`；**`constApprox` 不做**，Lean 在 `apply` 也关它）：quasi 允许 `args ⊆ lctx(m)`，FO/ctx 处理非 fvar / 上下文不匹配；但必须
   (i) `fun args => t` **类型检查通过**；(ii) `SOKO_META_APPROX=on`（HO3 前默认 off）；
   (iii) 该元变量**未被赋值**、且没有把它往别的方向拉的待定约束（避开 A2 的坏解）。
4. 失败 ⇒ `Undef`（进待定队列 / 最后由 defaulting 或报错收口），**绝不猜常量函数**。
5. 预算：走 #7 的三上限；失败 `restore`（与 `checkpointDefEq` 同款）⇒ 判定中性。

### 3.3 抽象后的类型检查（健全性）

- 抽象只在**赋值处**做；`fun args => t` 的类型必须在内核里成立（`m.ty` 与 lctx）。
- 实现：合成本地 `fun` 项，调**内核**的**完整检查**（`judge_terms`；`InferOnly` 不够——它不查实参类型）——只在这条路径上付一次；
  答不出 ⇒ `Undef`（不猜）；**绝不**把未检查的 λ 写进解。
- 出口不变：元变量**不进核项**（K1 的硬不变式不变；`zonk` 自检照旧）。

### 3.4 接线（`apply` / `solve_prefix`）

- **HO2（不依赖 B2）**：`apply` 的剥层循环加**早停** —— 每剥一层先试
  `unify_spine(剩余类型, goal, layers)`；命中即停（本例剥掉第一个箭头后 `¬ (x∈A)` 就等于目标，
  前提 `¬ (x∈∅)` 成为子目标）。**旧链保留**：早停不命中再走今天的 peel / 目标展开 / 内核兜底。
- **HO3（引擎本体）**：`apply` 的主链失败后，把 `(term 类型, goal)` 交给 `MetaCtx::unify`（`MetaStore` 只存数据）
  （pattern 规则），前提层作为子目标；`unify_spine` 保留为快路径。
- **B2 依赖**：`apply` 的 motive 元变量今天由 elab 的期望类型合成（不跨调用）⇒ HO2 不依赖 B2；
  HO4 的通用形态需要 B2（元变量活到 elaborate 期）⇒ **HO4 排在 B2 之后**。

## 4. 切片（HO 线；一片一档一 commit；每片带前后结构计数）

| 片 | 内容 | 判据（可执行） | 回退 |
|---|---|---|---|
| **HO1** | 本设计 + 复现件 + 读数基线（**不改代码**） | 本文件；`docs/gaps/repro/` 的 `.sh` 复现件：今天 exit≠0（缺口仍在） | — |
| **HO2** | `apply` 剥层**早停**（剥层前匹配守卫；HO4 **保留**，引擎管 mvar 方程） | unit02 练习 6 第一问：`apply Eq.subst (Eq.symm h)` ⇒ 目标 `¬ (x ∈ ∅)`，`exact Set.notMem_empty x` 判绿；反向 = `apply Eq.subst h` 仍红、`exact … (Set.notMem_empty x)` 在目标 `x ∈ A` 上仍红；**逃生门 `SOKO_APPLY_PREPEEL=0` 下**整本课程 `--json` 逐字节不变（打开时只许目标形状差异，逐条列出） | `SOKO_APPLY_PREPEEL=0` |
| **HO3** | `MetaCtx` + `pattern_assign`（严格 pattern + FO/ctx/quasi；近似默认 off）+ 单测 | 11 条真值层单测（分解 / occurs / type-occurs / scope / rollback / fuel / 常量解拒绝…）+ M0 13 形状 26 读数不变 + `SOKO_METAVAR=sibling` 逐字节不变 | `SOKO_META_PATTERN=0` |
| **HO4** | 近似翻默认 + `apply` 接引擎 + **B2 收口** + 全量验收 | 用户现场 + `apply Eq.subst _ hx` / `apply Eq.subst hx hy` 各形态；课程 `--json` 两态；`gate` 全绿；发版 | `SOKO_META_APPROX=0` |

**顺序**：HO1 ✅ → HO2（非发布里程碑）→ **B2-P0/P1/P2** → HO3（引擎本体）→ **B2-P4** → HO4（**唯一门**；P5 延后）。
与 `metavar-engine.md` 的 M 线并行：HO3 只动 `meta.rs`，接线在 HO2/HO4。

## 5. 判据与反向验证（绑用户动作）

- **正向**：`apply Eq.subst (Eq.symm h)` + `exact Set.notMem_empty x` ⇒ `decl.checked`；
  Goal 面板显示 `¬ (x ∈ ∅)`。
- **反向 1**：同一目标上 `apply Eq.subst h`（方向反）⇒ 仍红（`elab-implicit-argument-unsolved`）。
- **反向 2**：`exact Eq.subst (Eq.symm h) (Set.notMem_empty x)` 在目标 `x ∈ A` 上 ⇒ 仍红（`elab-tactic-failed`）。
- **回归**：`SOKO_APPLY_PREPEEL=0`（HO2 旧行为臂）/ `SOKO_META_PATTERN=0` / `SOKO_METAVAR=sibling` 下整本课程 `--json` 逐字节相同；
  M0 的 13 形状 26 读数不变；`crates/front` 真值层单测全绿。
- **性能**：结构计数自比，不用毫秒。

## 6. 风险 / 非目标 / 决策点

- 非目标：通用高阶合一（Huet）、二阶匹配、η（G-61 另算）、类型类。
- 风险：quasi 可能给坏解（A2）⇒ 无相反约束 + 类型检查通过才用。
- 风险：抽象检查走内核的成本 ⇒ 只在失败路径；结构计数把关。
- **D11**：quasi 是生产唯一可达分支（strict 恒不成立）⇒ **HO4 必须 on**；HO3 默认 off 仅对照（2026-10-26 定）。
- **D12**：`apply` 的对齐是否统一换成内核 `def_eq_at`（HO2 只做**失败重试**里的早停，不换快路径）。

## 7. 完整性：这个计划「完整」到什么程度（2026-10-25 自审）

- **不是**通用高阶合一（Huet：不可判定）⇒ 目标是 **Lean 对齐的完整**：
  严格 pattern + `approxDefEq` 的三档近似（`foApprox` / `ctxApprox` / `quasiPatternApprox`；
  **`constApprox` 不做**）+ 有界回退。η 属 G-61 转换规则，不在本片。
- **已知妥协**：① HO2 是**剥层前匹配守卫**（Lean 的 apply 也先比未展开 codomain；不是 mvar 规则）⇒ HO4 **保留**，
  `SOKO_APPLY_PREPEEL=0` 是回归臂，引擎只管 mvar 方程；② HO4 依赖 **B2 收口**（元变量进项）⇒ 前置，不是可选项；
  ③ quasi/FO/ctx 都是**近似** ⇒ 类型检查 + 无相反约束 + `Undef` 兜底；
  ④ pattern 赋值必须做**完整内核检查**（`judge_terms`），`InferOnly` 不算；⑥ `?p a…` 形状仍无 pattern 规则（P4b 只能排除）⇒ HO3/HO4；⑤ `apply` 快路径仍表面匹配；HO4 验收要求失败重试覆盖本族（D12）。
- **DoD（「完整」的判据）**：用户现场 + §5 正/反 + **Lean 形状矩阵**（`Eq.subst` 三形态 · 多实参 pattern · 一个非 `Not` 的 def 头 ·
  occurs/type-occurs/scope/let/重复实参 · 非 pattern 必须 `Undef` 不猜 · 常量解陷阱必须取最一般解）+
  `SOKO_METAVAR=sibling` / `SOKO_APPLY_PREPEEL=0` 逐字节 + B2 的 G-30/G-33/G-62 复现件。
  **矩阵不全 = 不算 HO4 收口。**

## 8. 参考

- Lean 4：`Meta/Tactic/Apply.lean:160/169` · `Init/Meta/Defs.lean:1601` · `Meta/Basic.lean:2050` ·
  `Meta/ExprDefEq.lean:523/1180/1199` · `Elab/Tactic/ElabTerm.lean:294`（commit 见 `metavar-engine.md`）。
- 本仓：`metavar-engine.md`（#16/M 线）· `meta.rs` · `by.rs`（apply）· `spine.rs`（`peel_pi_delta`）· `metavar-m0.md`（13 形状）· 现场 `unit02-subsets-empty.sokonanoda`。

## 9. 执行（2026-10-25 起）

HO1 复现/基线 ✅（`crates/cli/tests/apply_eqsubst.rs`）· HO2 早停 ✅（剥层前匹配守卫）·
HO3a ✅ · HO3b ✅ · B2 ✅ · **HO4 ✅（pattern/approx 默认开；quasi 捕获闸；full gate PASS）**。

**分工**：每片 subagent 实现 + 主会话验收（判据两遍、串行）；**HO4 的 DoD 全绿 = 唯一完成定义**；进度只写本行。
**依据**：G-30/33/62 假绿；R1/R2/R5。
**P4b 前**：P4a 已让 gate-on == gate-off（课程绿、gate-off 逐字节）；P0 仍红因缺 App 结果 isDefEq（`?β ≟ Set ?α`）。
