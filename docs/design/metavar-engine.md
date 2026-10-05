# 设计：元参数引擎（metavariable + unification）—— **IA-4（最完整版：范围 A + B + 内核）**

> 日期：2026-10-01（第 526 轮）。**用户 2026-10-01 新决策（推翻两条旧拍板）**：
> **不做缩范围**（D7 作废）、**直接做最完整版**——Lean/Coq 有的机制都要对齐；**项目无红区**，
> 功能与性能**同等重要**，**内核可以改**（允许加元变量/占位符，不被原架构约束）；D2（宇宙层合一不做）
> 一并作废 ⇒ 宇宙层合一**进入范围**。纪律不变：**一档一 commit · 遇决策点 ask · 判据逐条核验 ·
> 每片跑前后性能读数**（P1-b/§3.C 口径）✓。
> 取证：Lean 4（commit `77f336f7`）与 Coq（`440083ef`）源码逐条核对（**11 个「查无此名」的更正**见 §1.2 注）；
> M0 的基线与 13 形状清单 ⇒ `docs/design/metavar-m0.md`。

## 0. 一句话 + 三条旧结论的现状

**一句话**：把求解器从「贪心反解 + 同形兄弟复制」升级为**真元变量引擎**（元变量存储 + 三值合一 +
occurs/type-occurs + 有界待定约束 + sort/kind + **宇宙层合一**），并让元变量**活到 elaborate 期**
（期望类型沿嵌套实参传播），**内核提供占位符支持 + 硬不变式**（含元变量的声明一律拒绝）。

| 旧结论 | 现状（2026-10-01 用户新决策后）|
|---|---|
| **C1** 合一本身治不了 G-48 的 `α`（那位**没有约束**）⇒ 必须显式 **defaulting** | **仍然成立** ✓ —— 与"最完整"不冲突：Lean 同样不会替 `?α` 发明来源；我们的 defaulting 是**显式的选择规则**（§2.7）|
| **C2** 当前架构下接受面增量 **0/13**（M0 实测）⇒ 曾据此缩范围 | **不再是否做 A 的理由** —— 用户要求对齐机制本身（sort/kind · occurs · postpone · 宇宙 · 内核），
且**范围 B 会改变接受面**（G-30/G-33 一族）⇒ 以"机制对齐 + B 的缺口面"为验收，不以 A 的增量为验收 ✓ |
| **C3** 范围 B 卡在架构上（`elab_expr` 边 walk 边造核项、内核无占位符）⇒ 只写立项条件 | **解禁** ✓ —— 用户明确内核可改 ⇒ 范围 B **实现**，载体见 **D8**（内核占位符 vs 前端项 IR，§2.11）|

## 1. 现状盘点与 Lean/Coq 差距（**最完整**）

### 1.1 今天的求解器（读代码得来；M0 的实测读数 ⇒ `metavar-m0.md` §1）

* **唯一求解器** `implicit::solve_prefix(layers, result, k, arg_tys, expected, defs, is_inductive) -> Option<Vec<Expr>>`
  （`implicit.rs:127`），三个调用方：裸常量（`elab.rs:3006`）、应用钩子（`elab.rs:3417`）、路线③富余实参（`elab.rs:3277`）。
* **两条路线**：① 后续显式层的**域** vs 该实参的**类型**；② **结果类型** vs **期望类型**；都带 delta 兜底。
* **记法路径**另一份同形机械 `elab::solve_prefix_args_impl`（`elab.rs:2177`）+ 候选循环（`elab.rs:2070`）。
* **提取器** `unify_extract`（`elab.rs:3502`）：**单侧**、**只解一个变量**；无 occurs、无 sort 检查。
* **E19 窄版**：解不出的位记 `None`，走完由 `fill_pending_by_shape`（`implicit.rs:186`）与**域同形**的
  **已解兄弟**合一 —— 这是**选择规则**，不是合一（§0 的 C1）。
* **开关** `implicit::metavar_enabled()`（`implicit.rs:167`）：默认 `true`，只吃显式 `0`/`off`（逃生门）。

### 1.2 机制对照总表（**Lean 4 / Coq / 今天 / 目标 / 落在哪片**）

> ⚠ **11 个「查无此名」的更正**（研究实测；防止后人继续引幽灵 API）：`MVarId` **不是** `Expr`
> （是 `structure MVarId where name : Name`）· `MVarDecl` 真名 **`MetavarDecl`** · **没有** `ErrorKind`
> （真名 `MVarErrorKind`）· **没有** `isDefEqStar`/`_postpone`/`Level.simplify`/`LevelDefEqResult` ·
> `getParamKinds` 是 **delaborator** 专用 · Coq 的 `Postpone`/`unify_undef` 在 8.6.1→master **都不存在** ·
> `UState.check_universe`/`Evd.add_universe_constraints`/`Evd.instantiate`/`Evd.occur_evars`/`evar_normalize`
> **都不存在** · `Unification_HO`/`HO` 模块不存在 · `Sorts.sort_of_arity`/`is_sort` 其实在 `Reductionops`。

| # | 机制 | **Lean 4** | **Coq** | **今天** | **目标（最完整）** | 片 |
|---|---|---|---|---|---|---|
| 1 | 元变量身份与存储 | `Expr.mvar` 是 `Expr` 的一个构造子；身份 = `Name`；`MetavarContext.decls : MVarId → MetavarDecl`（`userName`/`lctx`/`type`/`depth`/`kind`）| `Evar.t = int`；`evar_map`（`evar_info`: concl/hyps/body/filter/source/abstract args）| 只有槽位 `Option::None`：**无 id、无类型、无作用域** | 真元变量存储（id + 声明类型 + 作用域 + kind）| M1 |
| 2 | 元变量**种类** | `natural / synthetic / syntheticOpaque`；**`syntheticOpaque` 永不被赋值**（"don't fill this hole from a typing constraint"）| 无 kind 概念（用 `evar_source`/flags 近似）| 无 | `natural` + `syntheticOpaque`（洞的语义）| M1/B2 |
| 3 | 作用域与深度 | `isAssignable = decl.depth == mctx.depth`；`withNewMCtxDepth`（**只做单侧匹配**就是它的用法）| `evar_filter`/`restrict`（上下文过滤 + aliasing）| fresh 名防捕获（`\0soko_p{i}`）| 深度 + **作用域检查（与 occurs 分开）**| M1 |
| 4 | 合一入口与**快路径** | `isDefEqQuick` 返回 **`LBool`（是/否/弃权）** + `whenUndefDo`；`t == s` 指针相等（hash-consing）| `evar_eqappr_x` 前有 `quick_fail` + `compare_heads` | `unify_extract` 单侧匹配 | 三值 + 快路径 + **两侧**结构分解 | M1 |
| 5 | occurs check | `occursCheck`（在**赋值处** `CheckAssignment.checkMVar`）+ **`typeOccursCheck`**（元变量出现在**另一个元变量的类型**里）| 在 `Evarsolve.evar_define` 查（**不在 `Evd.define`** —— 抄错层就不健全 ✗）| **无** | occurs + **type-occurs** | M1 |
| 6 | 待定约束 | **没有项级 `postpone`**：只有宇宙级 `PostponedEntry` + `processPostponed`，**停条件 = 待定计数不再严格下降** | `conv_pbs` 约束存储 + `solve_unif_constraints_with_heuristics`（**只在有进展时重来**）| 走完槽位**回头补一趟** | 约束存储 + **有界不动点**（停条件同款）| M1 |
| 7 | 终止性护栏 | `withIncRecDepth`；`maxSynthPendingDepth`；`checkpointDefEq` 失败**回滚状态** | `check_problems_are_solved`；超能力的问题报 `CannotSolveConstraint` | 无 | 深度 / 轮数 / 步数**三上限** + 失败回滚 | M1 |
| 8 | **宇宙层合一** | `Level` 带 `mvar LMVarId`；`isLevelDefEqAux` 的 `solve`（含 `solveSelfMax` 与显式近似）；**刚性宇宙约束 ⇒ 推迟到项元变量被赋值**（`postponeIsLevelDefEq`）| `UState` + `UnivProblem` 约束集（`ULe`/`UEq`/`ULub`…）+ **惰性批量检查**（`check_univ_implication`，Qed/声明期才查）| **无**（`Level(name)` 视为刚性）| level mvar + 约束存储 + **惰性检查** | U1/U2 |
| 9 | sort/kind 检查 | **没有独立的 kind 检查**：`sort ≟ sort` 在快路径归约到 `isLevelDefEqAux`；kind 正确性交给 `inferType`/内核 | `sort_of_arity`；arity 还不是 sort 时**把宇宙约束推迟**（`retyping.ml`）| **无**：错了落内核 `def_eq mismatch expected: Sort(1) \| actual: Sort(2)` | 三值语法近似 + 推迟（M3 的拍板：**作废候选 + 报既有码**）| M3 |
| 10 | 隐式实参插入 | 由**函数类型的 `BinderInfo`** 决定（`consumeImplicits`）；`@` 全显式；`..` 补缺 | `Evarconv` + 前类型系统（`Pretyping`）| 注册表 `implicit_prefix` + 风格对齐（IA-1 路线 C）| 保留；B 阶段接**期望类型** | M2/B1 |
| 11 | **期望类型传播** | `elabTermEnsuringType` + `ensureHasType`：实参**带着期望类型** elaborate；`mkAppM` 系列按合一补隐式 | `Pretyping`/`Typing` 双向检查 | **只有直接实参位**有期望类型（嵌套位没有）| **沿嵌套实参传播**（治 G-30/G-33）| B1/B2 |
| 12 | 出口（zonk）与内核边界 | `instantiateMVars`；**"送进内核的表达式不许含元变量"**（`Expr.lean` 注释）| `existential_value`/`instantiate_evar_array`（内核 `constr` 里**没有** evar）| 待定值**从不进项**（E19 硬保证）| 保留 + **内核兜底拒绝**（不变式）| B2/K1 |
| 13 | 报错契约 | `MVarErrorKind`（implicitArg / hole / custom）+ `logUnassignedUsingErrorInfos`（**只在没有别的错时**报）；三条文案；宇宙元变量**单独一条通道** | "The following term contains unresolved implicit arguments" / "Cannot infer …" / "Unsolved obligations" | 两条码（`elab-{implicit,notation}-argument-unsolved`）+ hint | **三通道**（补不出 / kind / clash）+ 位置 + 宇宙通道 | M3/U2 |
| 14 | 性能纪律 | **LBool 弃权** + `t == s` + **"不含元变量"的永久 def-eq 缓存**（`defEqPerm`）+ `Config.toKey` 键 + 递归护栏 | `quick_fail` + 惰性宇宙约束 | 严格档先跑、待定档才建 ctx | 保留 + **每片前后结构计数**（§2.8）| 每片 |
| 15 | **内核支持** | 内核与前端**共用 `Expr`**，但内核**拒绝含元变量的声明** | 内核 `constr` **不含** evar（evar 只活在 `evar_map`）| 内核**无**占位符（`Expr::Hole` 在 elab 期就报错）| **K1：占位符 + 硬不变式**（或前端项 IR，**D8**）| K1/B3 |
| 16 | 高阶 / pattern 合一 | eta、结构 eta、`foApprox` 等近似 | **Miller–Pfenning pattern 片段** + `w_unify` 的二阶分支 | 无 | **不做**（明确边界：报好错，不搜索）| — |
| 17 | 类型类合成 | `synthInstance` / `synthPending` / instance mvars | typeclass evars + `typeclass_instances` | 语言**没有** class/instance（台账 0 条）| **不做**（留接缝：kind 字段）| — |

### 1.3 关键更正（M0 实测 + 研究核对）

1. **合一不解 G-48 的 `α`**：`∅ ≈ {b}` 里 `α` 那一位**没有任何约束**（`∅` 的类型问不出来、期望类型
   `Prop` 也不提它）⇒ 让它变绿的是 E19 的**选择规则**。台账 G-48 的 `expected_lean`「两侧论域由合一
   解出」**不准确**（Lean 里 `?α` 同样没有外部来源）⇒ 处置 = **defaulting 显式化**（§2.7）✓。
2. **M0 的 0/13**：13 个形状上引擎与今天**逐条同判**（⇒ 增量在**范围 B** 与**宇宙层**，不在 A 的接受面）。
   清单另给 M1 两条硬约束：**S13 ⇒ 必须复用 delta 兜底**（否则误拒今天绿的形状 ✗）·
   **26 个读数 = `sibling` 档回归臂**（`metavar-m0.md` §2.1）。
3. **Coq 的 occurs 检查不在 `Evd.define`**：抄 `define` 而不补检查**不健全** ✗（我们把它放在**赋值处**，
   与 Lean 同层）。

### 1.4 实测基线（可复跑 ⇒ §6；完整表 ⇒ `metavar-m0.md` §1）

三指纹 `7646fe2e…`（非课程 86）/ `d0375577…`（课程 42）/ `06370a38…`（全语料 129）· 门禁
**43/377/99/0** · 冷 build `runs=886 bytes=47,437,669` · `passes=1343` · `by_calls=21,268` ·
非课程 **172 组 0 差异** · ⚠ 结构计数必须**同时**给空的 `SOKONANODA_CACHE_DIR`（否则 `hit>0` 是热启动）。

## 2. 引擎设计（**最完整版**）

### 2.1 范围（三条腿，全部在范围内）

* **范围 A（求解器内闭环）**：元变量生命周期 = 一次求解调用；入口形状/返回类型/三个调用方不变；
  **元变量永不进核项**（E19 硬保证保留）。
* **范围 B（元变量活到 elaborate 期）**：实参带着**元变量类型的期望类型**被 elaborate，声明末尾统一
  zonk ⇒ 治 **G-30**（期望类型不传播到嵌套实参）、**G-33**（无类型 binder 要语法上是 Pi）、
  **G-62**（def 形态 vs 展开形态的隐式实参那一半）。**载体由 D8 定**（内核占位符 vs 前端项 IR，§2.11）。
* **范围 K（内核支持）**：占位符 + **硬不变式**（含元变量的声明**一律拒绝**）—— 与 Lean 同构
  （内核与前端共用 `Expr`，但"送进内核的表达式不许含元变量"）。
* **宇宙层（U）**：`Level` 元变量 + 约束存储 + 惰性检查（§2.9）⇒ 治 **G-63**（手写 `.{u,v}` 对不准）。
* **明确不做**：高阶/pattern 合一（#16）· 类型类合成（#17）· η（G-61，是**转换规则**不是元变量机制）。

### 2.2 表示与接口（扩展版）

元变量 = **保留前缀的 fresh 名** `\0soko_m{id}`（不新增 AST 变体；`\0` 不可能出现在源标识符里）⇒
`substitute`/`mentions`/`same_shape` 一字不改。**范围 B 起**需要 kind/depth/作用域，仍可用同一编码 +
侧表（`MetaCtx`）。

```rust
pub(crate) struct MetaId(u32);
pub(crate) enum MetaKind { Natural, SyntheticOpaque }   // #2：洞不被类型约束填
struct MetaVar {
    ty: Expr,                 // 声明类型（= 该前导层的域）
    value: Option<Expr>,
    kind: MetaKind,
    depth: u32,               // #3：作用域/深度（withNewMCtxDepth 的对应物）
    origin: Origin,           // 报错用：哪一层/哪条路线/哪个实参位
}
pub(crate) struct MetaCtx {
    mvars: Vec<MetaVar>,
    postponed: Vec<(Expr, Expr)>,   // #6 待定约束（含宇宙约束，U1 起分表）
    fuel: u32, depth: u32, rounds: u32,   // #7 三上限
    unfold: &'a dyn Fn(&Expr) -> Expr,    // delta 兜底（S13 硬约束）
}
```

| 接口 | 语义 |
|---|---|
| `new(unfold)` | **只在待定档构造**（严格档一次都不建 ⇒ 默认路径零分配 ✓）|
| `fresh(ty, kind, origin)` | 建元变量（`depth` 取自当前 ctx）|
| `unify(l, r) -> Result<LBool>` | **三值**（是/否/弃权）+ 快路径 + 结构分解 + 待定（#4）|
| `unify_all()` | 有界不动点：**待定计数不再严格下降就停**（#6）|
| `assign(m, v)` | occurs + **type-occurs** + 作用域（#3/#5）；M3 起加 sort 检查 |
| `zonk(e)` / `unsolved()` | 出口代换（**不许含 `\0soko_m*`**）/ 报错用 |
| `default_unresolved()` | **E19 选择规则的显式化**（§2.7）|

### 2.3 合一算法（三值 + 分解 + 待定）

```
unify(l, r) -> Ok | Clash | Undef            // Undef = 弃权（不失败）
  zonk 两侧；same_shape ⇒ Ok；l 是元变量 ⇒ assign(l,r)；r 是元变量 ⇒ assign(r,l)
  (App, App)      ⇒ 头 + 实参逐位（个数不等 ⇒ Clash）
  (Arrow, Arrow)  ⇒ 域 + 陪域
  (Sort, Sort)    ⇒ SortKind 字面相等（U1 起：level 元变量走 §2.9）
  (Notation, …)   ⇒ 展开成「目标名 + 操作数」再比
  (Hole, _)       ⇒ Ok（通配）
  其余             ⇒ 先试 delta 兜底（**S13 硬约束**）；仍不成 ⇒ 含未解元变量 ? Undef : Clash
```

* **occurs / type-occurs**：`m` 不得出现在 `v` 里，也不得出现在 `v` 中**其它元变量的类型**里（#5）。
* **作用域**：`v` 不许提到**作用域外**的名字（与 occurs 分开报 —— 两者是不同 bug，#3）。
* **待定**：`unify_all` 重扫到队列空或**计数不再严格下降**（#6）；停下后交 §2.7 defaulting。
* **预算**：深度 ≤64 · 轮数 ≤8 · 步数 ≤4096；超限 ⇒ 按「无解」处理（回落既有错误码 ✓）。

### 2.4 集成点（唯一接线点；三个调用方不动）

```
solve_prefix(...)                                  // 形状不变：Option<Vec<Expr>>
  = 严格档 solve_prefix_impl(..., false)           // 逐字节不动、永远先跑 ✓
    .or_else(|| metavar_mode() == Off ? None : meta_solve(...))   // ← 引擎（取代 solve_prefix_pending）
```

* 约束与今天两条路线**同源**（不再一位一位贪心）：前导层名 → 元变量；① 每个后续层的域 ≟ 该实参类型；
  ② 结果 ≟ 期望类型；③ 记法路径额外把**操作数位**的域 ≟ 操作数类型。
* **`fill_pending_by_shape` 降级**为 §2.7 defaulting 的一部分。**记法候选循环不动**（只放宽最大候选）。
* **M2** 把同一份引擎接进 `implicit::solve_prefix`（应用 / 裸常量 / 路线③）。

### 2.5 sort/kind 检查（#9；M3）

本内核**非累积**（`KernelPropNotCumulative`）⇒ `?m := v` 要求 `v : m.ty` **恰好**成立。三值语法近似
（零内核调用）：`sort_of_value(Prop)=1` · `(Type 0)=2` · `(Sort n)=n+1` · `Level(_)/App/Lambda=不知道` ·
局部名取书写类型。判据：`m.ty` 是 `Sort n` 且 `sort_of_value(v)=Some k≠n` ⇒ **Kind 错**；任一不知道 ⇒
**放行**（只拒"确定错"的 ⇒ 不可能产生假拒绝 ✓）。
**用户 2026-10-01 拍板**：检出时 = **作废该候选 + 继续试下一个；都不行 ⇒ 报既有码**（不做 hint-only、
不新增码）—— 位置从内核挪到 elab 是**有意的**行为变化（M3 同轮重审 §2.6 的十二条）。

### 2.6 报错契约（#13；M1 不动、M3 收口）

**M1 的既有判据逐条重审**（十二条，逐条可跑）：`notation.rs` 三条（`an_unsolvable_…reports_its_own_code` ·
`implicit_arguments_are_inserted_from_the_first_explicit_argument` · `the_at_marker_disables_implicit_insertion`）·
`protocol.md:167` 码/hint 契约 · `implicit.rs` 三条单测 · `pending_solver_…never_guesses` ·
`{implicit,notation}_metavar.rs` · `judge_inplace_wide.rs` · `compile/tests.rs` 六条 ·
`no_course_signature_uses_an_implicit_binder`。
**M3 扩到三通道**（对齐 Lean 的 `MVarErrorKind`）：**补不出**（既有两条码）/ **kind 不对** / **clash**；
宇宙元变量**单独一条通道**（U2，对应 Lean 的 "don't know how to synthesize universe level metavariables"）。

### 2.7 未解元变量：defaulting（C1 的出口）

`unify_all` 后仍未解的元变量按**声明类型同形**（`same_shape`，与 E19 同判据）两两合一成**代表**；
代表也解不出 ⇒ 整体失败（**不发明类型** ✓）。这是**选择规则**不是推理 ⇒ 保证接受面 ⊇ E19
（`∅ ≈ {b}` 的 `α := β`、`Or.inl` 的 `B := Nat` 照旧，课程计数不掉）✓。**D4 已拍板：保留**。

### 2.8 性能纪律（#14；每片必跑）

1. **严格档永远先跑、逐字节不动**（E19 实测：关态计数逐项等于刀0）✓；
2. **成功路径零分配**（`MetaCtx` 只在待定档构造）；
3. **失败路径有界**（三上限 + 失败回滚 ⇒ 与 Lean 的 `checkpointDefEq` 同款）；
4. **顺序纪律**（真正的零开销是**次序**）：便宜检查在前 · **三值弃权**在中 · 昂贵路径最后 ⇒
   `unify_extract` 原样留作**第一段**，引擎只做**第二段**；
5. **每片前后读数**：`JUDGE_PREFIX runs/bytes` · `passes` · `by_calls` · `hits/misses` · `judge_ms`
   （**结构计数优先**；墙钟只兜数量级，不许当判据）。

### 2.9 宇宙层合一（#8；U1/U2，新进范围）

* **U1**：`Level(String)` → **level 元变量**（`\0soko_u{id}` 同款编码）+ 约束存储
  （`ULe`/`UEq`/`ULub` 三种够用）+ 赋值处的 occurs（`u` 出现在自己的解里 ⇒ 拒）；
  快路径：`SortKind` 字面相等 ⇒ Ok（今天的全部行为）。
* **U2**：**惰性批量检查**（对齐 Coq：约束攒着，声明收尾才查 ⇒ 不逐次合一都查图）+ 超限报**单独通道**
  （"补不出宇宙层级"）+ 与 `.{u,v}` 显式写法并存（显式优先，冲突 ⇒ 报错）。
* 判据：**G-63 复现件**（`Quot.lift.{1,0}` 一族）· 课程语料**逐字节不变**（今天所有 `.{n}` 都是显式的）。

### 2.10 范围 B：元变量进 elaborate 期（#11/#12）

* **B1** ⚠ **机制已交付 · 判据未达 ⇒ 出口改押 B2/B3**（2026-10-05 用户拍板 ★）：实参位**逐层**拿期望类型，
  嵌套实参接入点 + 门控已建 ✓（默认关 ⇒ 逐字节相同），但**十轮实测未转绿** ✗（第 8 轮 `AEP` 对
  `Iff.intro` **零命中** ✗；第 9 轮两版修**全还原** ✗）⇒ 判据**并入 B2/B3** ✓（**不再开 B1 第 5 版** ✗）。
* **B2 元变量进项 + zonk 边界**：实参带着含元变量的期望类型 elaborate；声明末尾 `zonk`；**元变量绝不
  活过声明**（内核兜底拒绝）。同时引入 `syntheticOpaque`（洞语义）⇒ **治 G-33**（期望类型 whnf 后是 Pi
  即可用）与 G-62 的一半。
* **B3 收口**：G-30/G-33/G-62 复现件逐条转绿 + 课程门禁两态 + 全语料对拍。

### 2.11 内核支持（#15；K1 —— **D8 要拍板**）

| 路线 | 做法 | 改动面 | 风险 |
|---|---|---|---|
| **(i) 内核占位符**（Lean 同构）| 内核 `Expr` 加一个 `Meta`（占位符），**判定层硬拒**（含元变量的声明一律 rejected）；前端协议**不变** | 内核 `Expr`/`Value`/pp/conv + 所有 golden | 内核面广，但有"拒绝"这条硬不变式兜底 |
| **(ii) 前端项 IR** | elab 先出**带元变量的前端项**，zonk 后再译核项 | **整个 `elab_expr` 的调用协议** + `by` 引擎 | 前端面广，内核零改动 |

**共同硬不变式**（两条都要）：**含元变量的声明一律拒绝**（错误码固定）· `--json`/golden 里**不许出现
占位符**（对拍红线）。**建议 (i)**：与 Lean/Coq 的架构一致（内核不认 evar/mvar 的**声明**），且前端
协议不变 ⇒ 切片更小；但**由用户拍板**（D8）。

## 3. 缺口面（更新：范围 B 与宇宙层把 G-30/G-33/G-62/G-63 拉进"能治"）

**能治（A + M3）**：G-48（巩固）· G-42/G-41/G-40/G-19（语义版取代语法代理）· **G-21**（早报 + 位置）·
G-43（结构合一取代按名字取）· **G-63**（U1/U2 起）· **G-60** 的一半（`{x | P x}` 的 `α` 有来源）。
**能治（B）**：**G-30**（期望类型传播）· **G-33**（期望类型 whnf 后是 Pi）· **G-62** 的隐式实参那一半 ·
**G-69/G-71 的判定侧**（项里带上插入的隐式实参 ⇒ 两种读法歧义消失；pp 那半另算）。
**不治（明确边界）**：内核归纳/消去规则（G-56/G-58/G-59/G-64/G-03/L-03/L-06 —— 但**内核现在可改**，
它们从"红线外"变成"另一条工作流"）· η（G-61）· pp 往返（G-70/G-71 的显示侧）· 缓存/重编译（P 组/K1 线）·
类型类（语言没有）。

## 4. 切片与排期（**一片一档一 commit**；每片都带**前后性能读数**）

| 片 | 内容 | 判据（可执行）| 回退 |
|---|---|---|---|
| **M0** ✅ 已收口 | 基线 + 13 形状清单（`metavar-m0.md`）| 见 M0 文档 | — |
| **M1** | 引擎内核（§2.2–2.3：元变量 + 三值合一 + occurs/type-occurs + 待定不动点 + fuel + defaulting）+ **记法路径**接线 + 开关 `SOKO_METAVAR=0\|sibling\|engine`（默认 sibling）| 真值层单测（分解/occurs/postpone/clash/fuel/zonk 无残留）· `sibling` 态**三指纹逐字节不变** + `--non-course` **0 差异** · 13 形状 26 读数不变 · 门禁 43/377/99/0 · 结构计数逐项相等 | 一个开关 |
| **M2** | 引擎接进 `implicit::solve_prefix`（应用/裸常量/路线③）；`fill_pending_by_shape` 降级为 defaulting | M1 全套 + `implicit_metavar.rs` 四条 + G-48 三态 | 开关 |
| **M3** | sort/kind 检查（§2.5）+ 报错契约三通道（§2.6）+ protocol/扩展/skills 同步 | §2.6 十二条逐条 · kind 夹具**不再落内核** `def_eq mismatch` · 新增码（若有）三处同步 | 开关 |
| **M4** | 默认开 + 发版（`sibling` 保留为回归臂）| E19 §9 同款两态摘要 + 交叉验证（sibling 态逐字节等于今天）+ CI 28/0 + release | 逃生门 |
| **U1** | 宇宙层元变量 + 约束存储（§2.9）| G-63 复现件 · 课程语料**逐字节不变** · 显式 `.{n}` 行为不变 | 开关 |
| **U2** | 宇宙约束惰性检查 + 报错通道 | 约束冲突夹具 · 课程门禁 | 开关 |
| **K1** ✅ 已收口（2026-10-04 ✓）| 内核占位符（**D8 = (i)** ✓）：`Expr::Meta` + `mk_meta` + 19 处穷尽 match ✓ + **`add_declar` 入口硬拒** ✓（固定前缀消息 ✓）| 硬不变式（含元变量 ⇒ 拒绝）✅ `tests::meta_placeholder` **4/4** ✓（含**深处嵌套** ✗ 与**对照组** ✓）· golden/`--json` 无占位符 ✅（6 文件逐字节相同 ✓，本片**惰性** ✓）· 全语料对拍 ⇒ **发版大节点** ✓ | 回退 = revert 本 commit ✓（本片无开关 ✗ —— **没有任何东西构造它** ✓ ⇒ 开关无意义 ✓） |
| **B1** ⚠ 机制已交付 · 判据未达 ★ | 期望类型沿嵌套实参传播：接入点 + 门控已建 ✓（默认关 ⇒ 逐字节相同）· **十轮实测 G-30 未转绿** ✗ | ~~G-30 转绿~~ ⇒ **并入 B2/B3**（2026-10-05 用户拍板）| 开关 |
| **B2** ◀ **当前片**（第 1 片 ✅ `4f8b6325` `MetaKind` ✓；**第 2 片 ✅ `7ca7dfb7` G-33 转绿** ✓ —— `peel_expected` 取域前**先 whnf** ✓，对齐 Lean `Binders.lean:408-421` `whnfForall` ✓；**反向验证 + (c) 逐字节** ✓）| 剩：**元变量进项**（让元变量活过一次求解调用 ✓ —— 引擎已就位 `9b6b4cfc`/`92233b0c` ✓）+ **G-30**（`And.intro` 实参位 ✗，另一条链 ✓）| G-30/G-62 复现件 + 内核拒绝不变式 + 对拍 | 开关 |
| **B3** | 范围 B 收口（三族复现件 + 课程 + 文档）| G-30/G-33/G-62 逐条 · 门禁 · 全语料 | — |

**建议次序**：M1 → M2 → M3 → M4（范围 A 收口发版）→ **D8 拍板** → K1 → B1 → B2 → B3 → U1 → U2。
**性能纪律**：每片收尾贴**同 run 的结构计数前后对照**；退化 ⇒ 先定位（`--timings`）再谈墙钟。

## 5. 决策点

| # | 决策 | 现状 |
|---|---|---|
| D1 范围 | ✅ 用户 2026-10-01：**推翻缩范围**，做最完整版（A + B + 内核 + 宇宙）| 已定 |
| D2 宇宙层合一 | ✅ 用户 2026-10-01：**作废"不做"** ⇒ 进入范围（U1/U2）| 已定 |
| D3 sort 检查强度 | 设计默认：**三值语法近似**（零内核调用）；内核 `judge_infer` 兜底留作开关候选 | 待定（可延到 M3）|
| D4 未解元变量 | ✅ 保留 E19 defaulting（接受面 ⊇ 今天）| 已定 |
| D5 开关形态 | 设计默认：`SOKO_METAVAR=0\|sibling\|engine`（默认 sibling；M4 起 engine）| 待定（M1 前定）|
| D6 报错契约 | ✅ 不新增码（M1–M2）；**M3 三通道**时是否新增 ⇒ **M3 前再问** | 部分 |
| D7 止损 | ✅ **作废**（用户要求最完整版）| 已定 |
| **D8 范围 B 的载体** | ✅ **用户 2026-10-01 拍板 = (i) 内核占位符**（Lean 同构）：内核 `Expr` 加占位符构造子，**判定层硬拒**含占位符的声明；前端 `elab_expr` 协议不变 ⇒ §2.11 | 已定 |
| D9 洞的表面语法 | ✅ **用户 2026-10-05 拍板 = 先不引入 `?_`**（B2 只做**内部** `syntheticOpaque`，无表面语法）| 已定 |
| D10 宇宙层开关与写法 | 显式 `.{u,v}` 与推断并存；逃生门形态 | 待定（U1 前）|

## 6. 复跑命令

```bash
cargo build --release -p sokonanoda-cli --bin sokonanoda
bash scripts/kernel-diff.sh --digest ./target/release/sokonanoda
bash scripts/kernel-diff.sh --non-course ./target/release/sokonanoda ./target/release/sokonanoda
SOKONANODA_BIN="$PWD/target/release/sokonanoda" python3 courses/set-theory/tools/check.py --json
./target/release/sokonanoda build --clean courses/set-theory
SOKONANODA_BUILD_JOBS=1 SOKO_STAGE_STATS=1 SOKONANODA_CACHE_DIR=/tmp/m0-cold \
  ./target/release/sokonanoda build --json courses/set-theory
cargo test -p sokonanoda-cli --test metavar_inventory && scripts/soko gate --fast
python3 scripts/gap.py check && python3 scripts/docs-lint.py && python3 scripts/status-lint.py
```

## 7. 后续计划（**IA-4 之后**：集合论教材线；队列权威 = `docs/ONBOARDING.md` §0.2）

> 用户 2026-10-01：**两个大项目最终都要做到最终形态**（元参数引擎 + 集合论教材扩展）；集合论线
> **前置障碍已解除**（E19 默认开 · 全课 `build` **214s → 47.8s**）⇒ **只排期、不写实现** ✓。

| 片 | 是什么 | 入口（先读） | 判据 |
|---|---|---|---|
| **S-A** | **重建卷 I 大纲** `docs/design/set-theory-syllabus.md`（对齐国际一流教材；现行草案口径 = Tao《Analysis I》§3.1–3.6 + epilogue，见 `teaching-project.md` §4.2，可补 Halmos/Jech/Kunen 章节对应表）+ **修全部悬空引用**（**实测 5 处**：`REQUIREMENTS.md:36` · `teaching-project.md:209` · `course-stdlib.md:10` · `courses/set-theory/README.md:4` · `courses/set-theory/AGENTS.md:80`）+ 新文件进预算表与过期登记 | `teaching-project.md` §4（卷 I 大纲 · 决策点 D-2/D-6）· `course-stdlib.md`（三层分界硬规则 10）| `grep -rn "set-theory-syllabus" --include=*.md .` 每处都指向**存在**的文件 · `docs-lint` ✓ · 课程门禁 **43/377/99/0**（不动课程）|
| **S-B** | **卷 I 深化**：ZFC · 序数 · 基数算术 · 选择公理 —— 课程单元**仍须全绿**（现 12 单元 `failed=0`）| **先读** `v077-kernel-deficiencies.md`（G-56/58/59/64）· `teaching-project.md` §5（DoD）| **先探针**（内核墙要实测）· 门禁 **12 单元 failed=0** + G1–G6 · `notation-lint` ✓ |
| **S-C** | **卷 II 分析起步**：实数构造（Dedekind 分割 / 柯西序列）→ 分析学 | `teaching-project.md` §8 **P7** · §1 · 本文 §2.10（**范围 B 是卷 II 的地基**）| **先出卷 II 设计** + **scale gate** ⇒ 再动课程 |

**顺序**：M1 → M2 → M3 → M4 →（D8）K1 → B1 → B2 → B3 → U1 → U2 → **S-A → S-B → S-C**；
两条线**不并行**动同一批文件（同一模块同一时间只允许一个写者）✓。
**记账**：M1 起每片 as-built **追加到本文**；M0 的读数在 `metavar-m0.md`。

## 8. M1 as-built（2026-10-01 **收口 ✓**）：引擎内核 + 记法路径接线

**开关三态** `SOKO_METAVAR=0|sibling|engine`（**默认 `sibling` = 今天**；`SOKO_NOTATION_METAVAR=0` 兼容
⇒ 旧逃生门仍等价）· **新模块** `crates/front/src/compile/meta.rs`（元变量 = `\0soko_m{id}` **名字编码**，
不新增 AST 变体）· **接线** `elab.rs::solve_prefix_args_meta`（记法路径的待定档按档位分流；
**一般路径归 M2**）· **元变量不进项**（`discharge` 出口自检 + `elab_expr` 兜底）。

| 判据 | 结果 |
|---|---|
| 真值层单测 11 条（两侧分解 / occurs / **type-occurs** / **作用域** / 链式赋值 / 待定不动点 / 刚性冲突 / **delta 兜底（S13）** / defaulting / zonk 无残留 / fuel）| **11/11** ✓ |
| `cargo test -p sokonanoda-front --lib` | **782/782**（771 + 11）✓ |
| M0 的 13 形状清单（`--test metavar_inventory`）| **1 passed** —— 26 个读数**一条不变** ⇒ `sibling` 回归臂成立 ✓ |
| 新判据 `--test metavar_engine`（5 个记法形状 × 4 档 + 开关等价）| **2 passed** ✓ |
| `notation` / `notation_metavar` / `implicit_metavar` | **49 / 1 / 1** 全绿 ✓ |
| **三指纹**：默认档 **与引擎档** | 两态**都逐字节等于** M0 冻结值（`7646fe2e…`/`d0375577…`/`06370a38…`）⇒ **引擎在整个语料上与窄版同判（0 增 0 失）** ✓ |
| 课程门禁（引擎档）| **43/377/99/0** ✓ |
| 冷 build 结构计数（默认档，空 `SOKONANODA_CACHE_DIR`）| `runs=886 bytes=47,437,669` · `passes=1343` · `by_calls=21,268` · `hits/misses=20,853/302` · `compiled 42/hit 0` —— **逐项等于 M0 基线** ✓（默认路径零开销）|
| `scripts/soko gate --fast` | **EXIT=0** ✓ |

**M1 抓到的两个真 bug**（都当场修掉 + 判据钉住）：

1. **作用域检查误拒**（新代码自己的）：`notation_telescope` 用的是**签名原文名**（`α`/`β`/`A`/`B`），
   拿它当「作用域外」判据会与**用户变量撞名** ⇒ 引擎档把 G-48 判红 ✗（实测 `theorem t (α β : Type)
   (b : β) : ¬ (∅ ≈ {b})`：值里的**用户 `β`** 被当成越界的望远镜参数）。**修法**：引擎路径**先 freshen
   望远镜名**（`\0soko_mp{i}`，与 `implicit::telescope` 的防捕获纪律对齐）⇒ 模板/作用域判据与用户名
   永不撞车 ✓。⚠ 这条是**三指纹判据**救回来的：只看"测试全绿"会以为引擎没问题。
2. **编译缓存键漏了开关**（**既有 bug**，release `v0.79.0` 也复现）：键 = (format, version, build,
   prelude, src) **不含** `SOKO_METAVAR` ⇒ 同一缓存目录里**先跑的那一档污染后面所有档** ⇒ 逃生门被
   静默忽略 ✗（实测：先 `default` 再 `=0` ⇒ 两次都绿）。**修法**：键里加**档位字节**
   （`cache::metavar_state`；**默认档保持 0** ⇒ 老缓存继续可用）+ 单测「不同档位必须是不同的键」✓；
   `cache.rs` 头部写死纪律：**凡改变编译结果的开关都必须进键**。
   ⚠ **残留面**（记账、未修）：`SOKO_JUDGE_INPLACE=shadow` 一类**诊断档**同样不在键里
   （`on`/`off` 的 `--json` 按设计逐字节相同 ⇒ 影响面小；`shadow` 档可能被缓存掩盖）。

**调试钩子** `SOKO_META_DEBUG=1`：打印每次引擎求解的（`missing` / ids / solved / unsolved / 每位取值）
—— 只在 `engine` 档的待定路径上付一次 `var_os` ✓。

**M1 不做**（后续片）：一般路径接线 = **M2** · sort/kind 与报错契约 = **M3** · 宇宙层 = U1/U2 ·
范围 B（内核占位符 → 期望类型传播）= K1/B1–B3 ✓。

## 9. M2 as-built（2026-10-01 **收口 ✓**）：引擎接进**一般路径**

**接线** `implicit::solve_prefix_meta`（应用 / 裸常量 / 路线③共用同一条 `solve_prefix`）——
严格档**永远先跑且不变**，失败后按档位分流（`Engine` ⇒ 引擎；`Sibling` ⇒ E19 窄版）✓。
**`fill_pending_by_shape` 降级**为 defaulting 的**参考实现**（`Sibling` 档那一份；引擎档的等价物是
`MetaCtx::default_unresolved`，判据仍是"声明类型同形"，只是从"拷一个已解兄弟的值"一般化成
"两两合一、选代表"）✓。⚠ `implicit::telescope` 的参数名**本来就是 fresh 名**（`\0soko_p{i}`）⇒
这条路**不需要**再 freshen（记法路径那条要 —— M1 实测的坑）。

| 判据 | 结果 |
|---|---|
| `cargo test -p sokonanoda-front --lib` | **782/782** ✓ |
| `implicit_metavar`（四条断言：默认绿 / 反向关红 / 不猜 / 既有码仍报）| **1 passed** ✓ |
| `metavar_inventory`（M0 的 13 形状）| **1 passed**（26 读数一条不变）✓ |
| `metavar_engine`（**3 passed**：M1 的 5 个记法形状 × 4 档 + 开关等价 + **M2 的 5 个一般路径形状 × 4 档**）| **3 passed** ✓ |
| `notation` / `notation_metavar` | **49 / 1** 全绿 ✓ |
| **三指纹：默认档 与 引擎档** | 两态**都逐字节等于** M0 冻结值 ⇒ 引擎在一般路径上也是**0 增 0 失** ✓ |
| 课程门禁（引擎档）| **43/377/99/0** ✓ |
| 冷 build 结构计数（默认档）| `runs=886 bytes=47,437,669` · `passes=1343` · `by_calls=21,268` · `hits/misses=20,853/302` · `compiled 42/hit 0` —— **逐项等于基线** ✓ |
| `scripts/soko gate --fast` | **EXIT=0** ✓ |

**M2 的读数口径再修正一次**（M0 只说了全局缓存）：冷 build 结构计数还要**项目缓存也确认是空的**
（`build --clean` 之后 `ls courses/set-theory/.sokonanoda/compiled | wc -l` = **0**），而且**别与别的
编译任务并发**（并发的 gate/门禁会**把项目缓存重新填上**）—— 实测踩到一次：项目缓存还有 41 个文件时
同一条命令给出 `compiled 30 / hit 12`、`runs=616`（**是热启动，不是回归** ✗）。

**M2 不做**（后续片）：sort/kind 与报错契约 = **M3** · 默认开 = **M4** · 宇宙层 = U1/U2 · 范围 B = K1/B1–B3 ✓。

## 10. M3 as-built（2026-10-01 **收口 ✓**）：sort/kind 检查 + 三通道归因

**引擎内的 sort/kind 闸门**（设计 §2.5，`MetaCtx::assign`）：三值语法近似 —— `sort_of_value(v)`
（`Prop`=1 · `Type`=2 · `Sort(n)`=n+1 · `Level(_)`/`App`/`Lambda`/`Ident`= **不知道**）vs
`sort_of_type(m.ty)`（`Prop`=0 · `Type`=1 · `Sort(n)`=n）；**只拒"确定错"的**（不知道 ⇒ 放行 ⇒
不可能假拒绝 ✓）。**用户拍板**：检出 = **作废该候选**（`unify` ⇒ `No`）+ 报**既有码**（不新增码）✓。

**三通道**（`MetaSolve { Solved, Unsolved, Kind, Clash }`）：引擎把失败**归因**（`MetaCtx::first_err`
记录首个硬错误，`discharge` 优先返回它 ⇒ 不会被"还有未解元变量"盖成 `Unsolved`）；调用方
（应用路径的错误点）**同一个码**下按通道换 `message` 那一句 ✓。**码与 hint 契约逐字不变**
（`Unsolved` 那条的 message **逐字等于**今天）⇒ §2.6 十二条判据**全部不动** ✓。

| 判据 | 结果 |
|---|---|
| 真值层单测（**15 条**：M1 的 11 + M3 的 `sort_of_value` 三值表 / kind 拒 / 不知道就放行 / 通道区分）| **15/15** ✓ |
| `cargo test -p sokonanoda-front --lib` | **786/786** ✓ |
| §2.6 十二条：`implicit_metavar` · `metavar_inventory` · `metavar_engine` · `notation` · `notation_metavar` | **1 / 1 / 3 / 49 / 1** 全绿 ✓ |
| **三指纹：默认档 与 引擎档** | 两态**都逐字节等于** M0 冻结值（⇒ **本轮零用户可见变化**）✓ |
| 冷 build 结构计数（默认档，项目缓存确认空）| 逐项等于基线 ✓ |
| `scripts/soko gate --fast` | **EXIT=0** ✓ |
| 契约同步 | `docs/protocol.md`（三通道说明，**码/hint 不变**）+ `skills/sokonanoda-teacher/SKILL.md`（按 message 判通道）✓；**扩展无需改**：VS Code 侧只透传 `message`/`hint`，没有码表 ✓ |

**⚠ 一条诚实的设计更正（M3 实测）：设计 §4 的「kind 夹具**不再落内核**」这一条**做不到**，
除非回退 G-21 的既有修复** ✗ —— 实测：`K Nat` / `L Nat` / `Set.powerset Nat` 这类「把类型写在
要项的位置」的形状，**严格档先跑且成功**（`unify_extract` 的裸变量位永远取得到值）⇒ 引擎根本轮不到，
它们照旧落内核 `kernel-expected-sort`（那条 hint 已经很好：① 漏了前导类型参数 ② 冒号后面是值，
G-21 的验收判据正钉在它上面）。⇒ **M3 的 sort 闸门是"引擎自己的正确性守卫"**（单元层可咬 ✓），
**不是**新的用户可见诊断；真要把它搬到 elab 期，得同时把 `kernel-expected-sort` 的 hint 内容搬过来
并更新 G-21 台账 —— **留作后续片的决策点**（不在 M3 的拍板范围内）。

**M3 不做**：默认开 = **M4** · 宇宙层 = U1/U2 · 范围 B = K1/B1–B3 · 记法路径的通道归因（记法错误点
在另一处，M3 只接了应用路径；记法那条的 message 仍走既有文案）✓。

## 11. M4 as-built（2026-10-01 **收口 ✓**）：**默认开** + 发版

**默认档翻到 `Engine`**（`SOKO_METAVAR` 不设 ⇒ 引擎；`SOKO_NOTATION_METAVAR=0` 仍把档位关掉 ✓）；
**`sibling` 永久保留为回归臂**（E19 那一份实现 + 判据继续钉着它 ✓）。**默认开的前提是两态等价**：
M1–M3 每一片都验过"引擎档与窄版档三指纹逐字节相同"，M4 再复核三态 ✓。

| 态 | 非课程(86) | 课程(42) | 全语料(129) |
|---|---|---|---|
| **默认（= 引擎，M4 起）** | `7646fe2e…` | `d0375577…` | `06370a38…` |
| **`SOKO_METAVAR=sibling`（回归臂）** | `7646fe2e…` | `d0375577…` | `06370a38…` |
| **`SOKO_METAVAR=0`（逃生门）** | `43581e06…` | `d0375577…` | `0231dcc4…` |

⇒ **默认档与回归臂逐字节相同**（引擎在整个语料上 0 增 0 失 ✓）；逃生门仍是 E19 §9 那两个值
（非课程/全语料各差一处，**恰好 G-48 那一份复现件**）✓。

| 判据 | 结果 |
|---|---|
| 真值层单测 / front lib | **15/15** / **786/786** ✓ |
| `metavar_engine`（3 passed；判据 1 改成「默认 ≡ **engine**」+ 新增「默认 ≡ sibling 回归臂」）| ✓ |
| `metavar_inventory`（M0 的 13 形状 × 2 态）· `implicit_metavar` · `notation` · `notation_metavar` | **1 / 1 / 49 / 1** ✓ |
| 课程门禁（默认档 = 引擎）| **43/377/99/0** ✓ |
| 冷 build 结构计数（默认档 = 引擎，项目缓存确认空）| 逐项等于基线 ✓ |
| `scripts/soko gate`（**完整**，含缺口台账门禁）| **EXIT=0** ✓ |
| 版本 | `0.79.0` → **`0.80.0`**（`Cargo.toml` + `editor/vscode/package.json` **必须相等**，两门课的
`sokonanoda.toml` 的 `requires` 钉同步 ✓）|

**缓存键**：默认档从 `Sibling`(0) 变成 `Engine`(1) ⇒ 升级后**旧缓存一次性失效**（安全的那个方向 ✓）。
**⚠ 没做**：`scripts/vscode-e2e.sh` 的真 VS Code 台账（按批次纪律在批次收尾跑；本片只跑 Rust/契约层）·
K1/B1–B3/U1/U2 见 §4 切片表 ✓。
