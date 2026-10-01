# 设计：元参数引擎（metavariable + unification）—— **IA-4 立项**

> 日期：2026-10-01（第 524 轮）。触发（用户 2026-10-01 拍板）：E19 甲案已闭环（`v0.79.0`），
> 下一大项目 = 把 E19 的**窄版待定参数**升级为**真正元参数**（Lean/Coq 式通用合一 + sort/kind 检查）。
> **本文只做设计与排期，不动实现** ✓；切片 §4，**要用户点头的决策点 §5**。
> 上游（原文已删 ⇒ `git log --all -- docs/design/implicit-arguments.md`）：IA-1 把**路线 A（真元变量 +
> 合一）**列为 **IA-4：未排期**；E19 甲案是它的最小切片 ⇒ **本文 = IA-4 立项设计**。

## 0. 一句话 + 三条先说结论（都不是好消息，但都改排期）

**一句话**：把 `solve_prefix` 的「一位一位贪心反解 + 同形兄弟复制」换成**一次求解内闭环的元变量引擎**
（元变量存储 + 结构合一 + occurs check + sort/kind 检查 + 待定约束不动点 + 出口 zonk）；
**严格档逐字节不动**，引擎只在**今天已经会失败**的那条路上生效（E19 的接线口径不变）。

| # | 结论 | 证据 | 对排期的影响 |
|---|---|---|---|
| **C1** | **合一本身治不了 G-48 的 `α`** ✗ | `∅ ≈ {b}` 里 `α` **一个约束都没有**：`∅` 的类型问不出来（`operand_type_expr` ⇒ `None`），期望类型是 `Prop` 也不提 `α`。今天让它绿的是 E19 的**选择规则**「同形的已解兄弟 ⇒ 取它的值」，**不是合一** | 引擎必须把它**显式化**为 defaulting（§2.7），否则 G-48 **重新判红**、课程计数会掉 |
| **C2** | **当前架构下引擎的「接受面增量」很小** | 实参类型来自**具体源项**（`arg_tys`），**永远不含元变量** ⇒ 约束恒为「模板 ≟ 具体项」的**单侧**形状，而 `unify_extract`（`elab.rs:3502`）已经在做（含嵌套位 / Pi / 记法头 / delta 兜底）| 真实增量 = **① sort/kind 检查 ② 冲突检出 ③ 一条机械取代三条 ④ 范围 B 地基**（§3.1）。**不许**拿「接受面大涨」当排期理由 ✗ |
| **C3** | **大头在范围 B，而它卡在架构上** | `elab_expr` **边 walk 边造核项**（`builder.mk_app`，`elab.rs:3469`），内核**没有元变量/占位符**（IA-1 §1 实测；`Expr::Hole` 在 `elab.rs:4000` 直接报错）| 范围 B 只出**立项条件 + 两条路线取舍**，不排期（§2.1）|

**建议（已拍板 D1 = ③，2026-10-01）**：**先只做 M0**（基线 + 能力清单），用实测决定 A/B 取舍
（§4；**D1/D7**）。**研究输入**：Lean 4（commit `77f336f7`）与 Coq（`440083ef`）源码逐条核对，
含 **11 个「查无此名」的更正**（如 `MVarId` **不是** `Expr`、`MVarDecl` 真名 `MetavarDecl`、
Coq `Postpone`/`unify_undef` 在 8.6.1→master 都不存在、`Sorts.sort_of_arity` 其实在 `Reductionops`）
⇒ 本文只保留**承重引用**（§1.2 / §2.8）。

## 1. 现状盘点：窄版待定参数 vs Lean/Coq

### 1.1 今天的求解器（`crates/front/src/compile/`，读代码得来）

* **唯一求解器** `implicit::solve_prefix(layers, result, k, arg_tys, expected, defs, is_inductive) -> Option<Vec<Expr>>`
  （`implicit.rs:127`），**三个调用方**：裸常量（`elab.rs:3006`，只有路线②）、应用钩子（`elab.rs:3417`）、
  路线③富余实参落结果（`elab.rs:3277`）——**三者都不动**就是 E19 刀2 的口径 ✓。
* **两条路线**：① 后续显式层的**域**（已代换）vs 该实参的**类型**；② **结果类型** vs **期望类型**；
  两条都带 delta 展开兜底（模板侧 + 实参侧）。
* **记法路径**另一份同形机械 `elab::solve_prefix_args_impl`（`elab.rs:2177`）+ 候选循环
  `for missing in (1..=max_missing).rev()`（`elab.rs:2070`）——**只放宽最大候选**。
* **提取器** `elab::unify_extract(template, actual, name)`（`elab.rs:3502`）：**单侧**、**只解一个变量**
  （裸变量位 / Pi 域陪域 / 模板剥到结果 / 头名相等 + 实参右对齐 / 嵌套位递归且同变量取值须一致）。
  **没有** occurs check、没有 sort 检查、不把 actual 侧也当模板。
* **E19 窄版** `solve_prefix_impl(..., allow_pending)`（`implicit.rs:253`）：某位解不出先记 `None`，走完由
  `fill_pending_by_shape`（`implicit.rs:186`）与**域同形**（`spine::same_shape`）的**已解兄弟**合一 ——
  判据是 `layers[i].domain` 的**形状**（`{α β : Type}` ⇒ 两层域都是 `Type 0` ⇒ 同形）。
* **开关** `implicit::metavar_enabled()`（`implicit.rs:167`）：默认 `true`，只吃显式 `0`/`off`（逃生门）。

### 1.2 与 Lean 4 / Coq 的差距

| 机制 | Lean 4 / Coq | 今天 | 差在哪 |
|---|---|---|---|
| 元变量存储 | `MetavarContext`/`MetavarDecl`（type/lctx/depth/kind）· Coq `evar_map`（`Evar.t = int`） | 只有槽位 `Option::None`：**没有 id、声明类型、作用域** | 表达不了「同一个未知出现在两处」|
| 合一 | `isDefEq`：结构分解 + 延迟约束 + proof irrelevance + eta | `unify_extract`：**单侧 + 只解一个变量** | 两侧都是未知时无解；不能一次解多个 |
| occurs check | Lean 在**赋值处**查（`CheckAssignment.checkMVar`）· Coq 在 `evar_define` 查，**不在** `Evd.define`（抄错层就**不健全** ✗）| **无** | 今天靠「模板来自签名、值来自具体项」侥幸无环；引擎允许两侧有未知后**必须有** |
| sort/kind | 赋值时 `isDefEq (infer v) mvar.ty`（Lean 里免费的：`Sort u ≟ Sort v` 归约到 `isLevelDefEqAux`）；宇宙层用 `Level` mvar + 约束 | **无**：错了留给内核报 `def_eq mismatch expected: Sort(1) \| actual: Sort(2)`（**实测**：`kernel-rejected`、无 hint）| 报错晚、位置远、文本是内核内部记号 |
| 待定约束 | Lean **没有项级** `postpone`（只有宇宙级 `PostponedEntry`；推迟发生在 synthetic mvar 层 `processPostponed`）· Coq = `conv_pbs` + `solve_unif_constraints_with_heuristics` | 只有「走完槽位再回头补」**一趟** | 不到不动点、顺序敏感 |
| 出口 | `instantiateMVars` · `Evd.instantiate` | 不需要（待定值**从不进项**，E19 硬保证）| **保留** ✓ |
| 报错契约 | `dontKnowHowToSynthesizeImplicitArgument` · Coq `Unresolved evars` | `elab-{implicit,notation}-argument-unsolved` + hint | 表达不了「冲突」「kind 不对」|
| 作用域 | `withNewMCtxDepth` + 作用域检查 | fresh 名 `\0soko_p{i}` 防捕获（`telescope` 既有纪律）| 引擎沿用即可 |

### 1.3 关键更正：**合一不解 G-48**（C1 的推演，逐条可查）

`∅ ≈ {b}`（`Set.Equiv {α β : Type}`，`theorem … (α β : Type) (b : β)`）：① `arg_tys[1]` = `{b}` 的类型 =
`Set β_user`（集合字面量自己解出了论域）⇒ 路线① 给 `β := β_user`；② `arg_tys[0]` = `∅` 的类型 =
**`None`** ⇒ `α` **无约束**；③ 期望类型 `Prop` 也**不提** `α`。⇒ **诚实的合一引擎在 `α` 上仍然失败**。
⚠ 台账 G-48 的 `expected_lean`「两侧论域由合一解出」**不准确**：Lean 里 `?α` 同样没有外部来源。
⇒ 处置是**把选择规则显式化**（§2.7），不是「靠合一自动解决」。

### 1.4 实测基线（`target/release/sokonanoda` = v0.79.0，数字可复跑 ⇒ §6）

| 读数 | 值 |
|---|---|
| `--digest` 非课程(86)/课程(42)/全语料(129) | `7646fe2e…` / `d0375577…` / `06370a38…`（**逐字节等于 `e19-baseline.md` §9 默认态** ✓）|
| 课程门禁 | **43 目标 · 377 checked · 99 open · 0 判负**（canvas_open 96）|
| 冷 `build` 结构计数 | `JUDGE_PREFIX runs=886 bytes=47,437,669` · `passes=1343` · `by_calls=21,268` · `hits/misses=20,853/302` · `compiled 42 / failed 0 / hit 0` |
| G-48 复现件 | 默认 **exit 0** / `SOKO_NOTATION_METAVAR=0` **exit 1** ✓ |
| 形状探针（M0 的清单种子）| `{a} ≈ {b}` ✓绿 · `∅ ≈ {b}` ✓绿（E19）· `Set.Equiv ∅ {b}` ✓绿（刀2）· `Set.Equiv {a} ∅` ✓绿 · `Set.Equiv ∅ ∅` ✗红（无兄弟）· `G Nat`（`def G {α : Type} (A : α)`）✗红（**内核**报 `期望 Sort(0)，实际是 Pi (A : Nat), Sort(0)`——旧写法歧义，G-42 的「逐位贴合」是语法代理）|

## 2. 引擎设计

### 2.1 范围：**A 做，B 只写立项条件**

* **范围 A（排期）**：引擎是 `solve_prefix` 的**内部实现**——元变量生命周期 = **一次求解调用**；
  入口形状、返回类型、错误码、三个调用方**全不变**；**元变量永不进核项**（保留 E19 硬保证）。
* **范围 B（非目标）**：元变量活到 **elaborate 期**（实参带元变量类型的期望类型被 elaborate，最后 zonk）
  ⇒ 治 **G-30**（期望类型不传播到嵌套实参）、**G-33**（无类型 binder 要语法上是 Pi）、**G-62** 一族。
  **卡点**：`elab_expr` 边 walk 边造核项，内核没有占位符 ⇒ 两条路线：**(i) 前端项 IR**（elab 先出带
  元变量的前端项、zonk 后再译成核项；改动面 = 整个 `elab_expr` 调用协议 + `by` 引擎）；
  **(ii) 内核占位符**（`Expr` 加只在内部存活的 `Meta`；改动面 = 内核 + 所有 golden）。
  **立项条件**：G-30/G-33/G-62 里**至少 2 条**有独立复现件且判「必须修」（现 G-30/G-33 都是 `open` +
  有 repro ✓）**且** A 的 as-built 全绿（M1–M3）⇒ 再选 (i)/(ii)。

### 2.2 表示与接口（**不新增 AST 变体**）

元变量 = **保留前缀的 fresh 名** `\0soko_m{id}`（`\0` 不可能出现在源标识符里，`telescope` 的
`\0soko_p{i}` 已依赖这一点）⇒ `substitute`/`mentions`/`same_shape`/`substitute_names` **一字不改**。
代价：出口必须保证无残留（`zonk` 后断言 + `elab_expr` 遇未知名自然报错 ⇒ 双保险）。

```rust
// crates/front/src/compile/meta.rs（新）
pub(crate) struct MetaId(u32);                        // 只在一次求解内有效
struct MetaVar { ty: Expr, value: Option<Expr>, origin: Origin }
enum Constraint { Eq { lhs: Expr, rhs: Expr } }       // 待定约束（postponed）
pub(crate) struct MetaCtx { mvars: Vec<MetaVar>, postponed: Vec<Constraint>, fuel: u32 }
pub(crate) enum MetaErr { Clash, Occurs(MetaId), Kind { .. }, OutOfFuel }
```

| 接口 | 语义 |
|---|---|
| `new()` | **只在待定档构造**（严格档一次都不建 ⇒ 默认路径零分配 ✓）|
| `fresh(ty, origin)` | 建元变量；`ty` = 该前导层的**域**（已把更早的元变量代进去）|
| `unify(l, r)` | 一步：zonk → 元变量赋值 / 结构分解 / 待定 / 冲突 |
| `unify_all()` | 跑到不动点（`postponed` 空或一轮无新赋值；轮数有上限）|
| `zonk(e)` / `unsolved()` | 出口代换（**不许含 `\0soko_m*`**）/ 报错用 |
| `default_unresolved()` | **E19 选择规则的显式化**（§2.7）：声明类型同形的未解元变量两两合一、选代表 |

### 2.3 合一算法

```
unify(l, r): 1) l,r ← zonk 2) l == r ⇒ Ok 3) l 是元变量 ⇒ assign(l,r)；r 是元变量 ⇒ assign(r,l)
             4) 同头 ⇒ 结构分解（下表），逐位 unify；实参个数不同 ⇒ Clash
             5) 任一侧含未解元变量 ⇒ push postponed（不失败） 6) 否则 Clash（刚性冲突）
assign(m, v): occurs(m, v) ⇒ Occurs；sort_ok(v, m.ty) 否则 Kind；m.value := v
```

| 形状 | 分解规则 |
|---|---|
| `App`/`App` | 头 `unify` + 实参 `unify`（**个数必须相等**，否则 `Clash`）|
| `Arrow`/`Arrow`、`Forall`/`Forall` | 域、陪域/体分别 unify（沿用 `peel_pi` 把 `->` 与 `forall` 统一）|
| `Sort`/`Sort` | `SortKind` **字面**相等（`Prop`/`Type`/`Sort(n)`）；`Level(name)` **刚性**（**不做宇宙层合一**，D2）|
| `Ident`/`Ident`、`UniverseApp`/`UniverseApp` | 名字 + 层实参**字面**相等，否则 `Clash` |
| `Notation` | 展开成「目标名 + 操作数」再比（复用 `head_and_args_notation`）|
| `Hole`/任意 | 通配 ⇒ Ok（既有语义：洞不约束）|
| 其余（Lambda/Let/Match/Num 跨类）| `Clash` |

* **occurs check**：`m` 不得出现在 `v` 里（zonk 后比）。
* **待定约束**：`unify_all` 重扫 `postponed` 到队列空（成功）或**待定计数不再严格下降**（停条件与 Lean
  `processPostponed` 同款 ⇒ 不是无界工作队列 ✓）；停下后交 §2.7 defaulting，仍不解 ⇒ `unsolved` ⇒ 失败。
* **预算（终止性）**：单次递归深度 ≤ 64 · 轮数 ≤ 8 · 总步数 ≤ 4096；超限 ⇒ `OutOfFuel` ⇒ **按「无解」
  处理**（回落既有错误码，**不新增失败面** ✓）。

### 2.4 集成点（**唯一接线点，三个调用方不动**）

```
solve_prefix(...)                                  // 形状不变：Option<Vec<Expr>>
  = 严格档 solve_prefix_impl(..., false)           // 逐字节不动、永远先跑（E19 口径 ✓）
    .or_else(|| metavar_mode() == Off ? None : meta_solve(...))   // ← 引擎，取代 solve_prefix_pending
```

* **约束与今天两条路线同源**（只是不再一位一位贪心）：把每个前导层名代成它的元变量，① 对每个
  `arg_tys[j-k] = Some(actual)` 且该层域提到前导名 ⇒ `unify(domain[σ], actual)`；② `expected` 存在 ⇒
  `unify(result[σ], expected)`；③ 记法路径额外把**操作数位**的域与操作数类型合一。
* **`fill_pending_by_shape` 的去向**：降级为 §2.7 defaulting 的**一部分**（判据仍是域同形），不再直接写
  `solved[i]`。**记法候选循环不动**（引擎只接 `missing == max_missing` 那一档，E19 刀1 口径 ✓）。

### 2.5 sort/kind 检查（规则取自**本内核**，不是 Lean）

本内核**非累积**（`KernelPropNotCumulative` 就是那条边界；`Prop ⊄ Type`）⇒ `?m := v` 要求 `v : m.ty`
**恰好**成立。引擎用**三值语法近似**（零内核调用）：

| `v` 的形状 | `sort_of_value(v)`（`v : Sort n` 的 `n`）|
|---|---|
| `Prop` / `Type`(`Type 0`) / `Sort(n)` | `1` / `2` / `n + 1` |
| `Sort(Level(_))`、`App`、`Lambda`、其余 | **`None`（不知道）** |
| `Ident{name}` 且 `scope.source_type_of(name)` 或签名结果类型是 Sort | 递归取 |

判据：`m.ty` 是 `Sort n` **且** `sort_of_value(v) = Some k` **且** `k ≠ n` ⇒ `Kind`；任一 `None` ⇒
**放行**（保守：**只拒「确定错」的** ⇒ 不可能产生新的假拒绝 ✓；其余仍由内核兜底）。`Kind` 在待定档的
处置见 §2.6（D6）。

### 2.6 报错契约：`ElabImplicitArgumentUnsolved` 既有判据**逐条重审**（引擎**不许**把「解不出」放过去 ✗）

| # | 判据 | 期望 |
|---|---|---|
| 1 | `notation.rs::an_unsolvable_implicit_argument_reports_its_own_code`（夹具与 `implicit_metavar.rs` 的 `UNSOLVABLE` 逐字相同）| **仍报同码** ✓ |
| 2–3 | `notation.rs::implicit_arguments_are_inserted_from_the_first_explicit_argument`（正向）· `the_at_marker_disables_implicit_insertion` | 绿（接受面只更宽；`@` **不进**求解器）|
| 4 | `docs/protocol.md:167` 的码 / hint 契约 | **不加码、不改文本**（除非 D6 选新增码 ⇒ 同轮改 protocol + 扩展 + skills）|
| 5–6 | `implicit.rs::solve_prefix_reads_the_first_explicit_layer` · `_scans_later_explicit_layers_too` · `_reads_the_expected_type`（末条默认态 `B := Nat`）| 绿；**末条重审**：defaulting 必须仍给 `B := Nat` |
| 7 | `implicit.rs::pending_solver_unifies_same_shape_siblings_and_never_guesses`（四条）| **重审**：`α := β` ✓ / 两位都空 ⇒ `None` ✓ / 严格档 `None` ✓ / 默认态 `Some` ✓ |
| 8–9 | `cli/tests/implicit_metavar.rs`（默认绿 · 反向关红 · 不猜 · 既有码仍报）· `notation_metavar.rs`（开绿 / 反向关红 / 不猜）| 全绿 |
| 10–11 | `front/tests/judge_inplace_wide.rs`（结论必须随真变重算）· `compile/tests.rs` 六条（8577/8611/8646/8721/8779/9106）| 绿（接线点不变）|
| 12 | `notation.rs::no_course_signature_uses_an_implicit_binder` | 绿（签名面，引擎不碰）|

**D6**：`Clash`/`Kind`/`Occurs` 是**降级**到既有码（零契约变更，推荐 M1–M3），还是**新增 1–2 个精确码**
（要同轮改 `protocol.md` + `editor/vscode` + `skills/`）？

### 2.7 未解元变量：**defaulting**（E19 规则的显式化，C1 的出口）

`unify_all` 后仍未解的元变量，按**声明类型同形**（`spine::same_shape`，与 E19 同一判据）两两 `unify` 成
一个**代表**；代表也解不出 ⇒ **整体失败**（照旧报「补不出」，**不发明类型** ✓）。这条规则**不是推理、
是选择**（E19 §6 已由用户拍板接受）⇒ 保证**接受面 ⊇ E19**（`∅ ≈ {b}` 的 `α := β`、`Or.inl` 的
`B := Nat` 都照旧，课程计数不掉 ✓）。**D4**：是否保留由用户定；去掉 ⇒ 接受面变窄、E19 既有判据翻红。

### 2.8 热路径零开销（P1-b / §3.C 纪律 ⇒ 五条硬约束）

1. **严格档永远先跑、逐字节不动**：解得出的形状一次都不进引擎（E19 实测：关态计数逐项等于刀0）✓；
2. **成功路径零分配**：`MetaCtx` **只在待定档构造** ⇒ 严格档不新增任何分配；
3. **失败路径也有界**：`fuel`/深度/轮数三个上限（§2.3）⇒ 学习者写错时的额外成本是常数；
4. **默认不做内核调用**：sort 检查用三值语法近似（§2.5）；`judge_infer` 兜底只留作开关候选（D3）；
5. **量具先行**：每片先报**结构计数**（`JUDGE_PREFIX runs/bytes`、`passes`、`by_calls`、`hits/misses`）
   再谈墙钟；**墙钟不许当判据**（`AGENTS.md` §性能门禁②：同一二进制两次差过 15%）。
6. **顺序纪律**（研究结论：真正的零开销**不是开关、是次序**）：便宜的结构检查在前 · **三值（是/否/弃权）**
   的声明式拒绝在中（Lean `LBool`+`whenUndefDo` · Coq `compare_heads`/`quick_fail`）· 昂贵路径在最后
   ⇒ 映射 = **`unify_extract` 原样留作第一段**，新合一器只做**第二段**、绝不替换它 ✓。

## 3. 能治 / 部分治 / 不治（台账逐条；D7 用它止损）

**能治**（引擎是主因，可独立关账）：**G-48**（`fixed` 0.79.0 窄版 ⇒ 同一结果由**一条机械**给出，
窄版路径退役）· **G-42 / G-41 / G-40 / G-19**（都 `fixed`，各带语法代理——「逐位贴合」「结果展开
surplus」「路线②/③」⇒ 语义版取代，判定不变、代码面收窄）· **G-21**（`open`：漏写前导类型参数 ⇒
赋值/写实参处**早**报 +（D6）精确码 + hint，**诊断**增量不改判定）· **G-43**（`fixed`：结构合一取代
按名字取，pp 那半是另一条线）。

**部分治**（要另一半机制）：**G-60**（`{x | P x}` 的 `α` 有了来源；缺**花括号 binder 记法形状**）·
**G-62**（隐式实参元变量有了；缺 **def 头期望类型要 delta 展开**）· **G-34**（元变量 + 局部上下文让
就地定类型成为可能；缺探针/裸常量/归纳安装的 miss + 缓存键）· **G-69**（项里带上插入的隐式实参 ⇒
两种读法歧义消失；已修的 AST 对齐 + pp 保真守卫要保留）。

**不治**（明确边界，不是本项目的失败面）：**期望类型传播 G-30/G-33** ⇒ 范围 B（§2.1）· **内核规则**
G-56/G-58/G-59/G-64/G-03/L-03/L-06（递归子宇宙、大消去、累积性、良基递归；用户 2026-09-28 已明确
「成本最高，别自己开工」）· **η G-61**（内核转换规则；前端加 η 会**与内核判定不一致** ✗）·
**宇宙层合一 G-63**（需 `Level` 元变量 + 约束集，D2 本轮不做）· **pp 往返 G-70/G-71/G-49** ·
**缓存/重编译 G-29/G-31/G-34/G-68**（P 组/K1 线）· **类型类合成**（本语言无 class/instance，台账 0 条）。

## 4. 切片与排期（**一片一档一 commit**，可独立验证 / 可独立发布）

**共同纪律**（每片收尾，与 E19 三刀同款）：`scripts/soko gate` exit 0 · `--digest` 三指纹 · 非课程
172 组对拍 · 课程门禁 **43/377/99/0** · 结构计数（§2.8⑤）· **反向验证**（新判据要能咬住「开关其实是
假的」）· 一档一 commit、批次收尾才 push。

| 片 | 内容 | 判据（可执行）| 回退 |
|---|---|---|---|
| **M0** | **零行为变化**：冻结基线（§1.4 全部读数）+ **能力清单**（~12 形状 × 严格档/今天/引擎推演，每个形状一个夹具入库）+ 判据清单（§2.6 十二条）| `kernel-diff.sh --digest` 复现 §1.4 三 sha256 · 门禁 43/377/99/0 · 计数逐项相等 | 无（纯文档）|
| **M1** | `meta.rs`（§2.2–2.3 + 真值层单测）+ **记法路径**接线（待定档改调 `meta::solve`）；开关 `SOKO_METAVAR` 三态（`0` 严格 / `sibling` = E19 今天 / 默认 引擎，D5）| `cargo test -p sokonanoda-front --lib`（occurs/kind/postpone/clash/fuel/zonk 六条真值判据）· `-p sokonanoda-cli --test {notation,notation_metavar,implicit_metavar}` · 新 `cli/tests/metavar_engine.rs`（引擎开 ⇒ 清单里「引擎新能」的形状绿；`sibling` ⇒ **逐字节等于今天**；`0` ⇒ 严格档）· `kernel-diff --non-course <今天> <新>` 在 `sibling` 态 **0 差异** | 一个开关（`SOKO_METAVAR=sibling`）|
| **M2** | 引擎接进 `implicit::solve_prefix` **一般路径**（应用/裸常量/路线③）；`fill_pending_by_shape` 降级为 defaulting 的一部分 | M1 全套 + `implicit_metavar.rs` 四条 + `pending_solver_…never_guesses` 四条 + G-48 复现件**三态**（`0`→exit 1 / `sibling`→exit 0 / 引擎→exit 0）· 门禁两态 43/377/99/0 | 同上 |
| **M3** | **sort/kind 检查**（§2.5 三值表）+ 报错契约收口（§2.6 十二条 + D6 落地）| §2.6 十二条全绿 · kind 夹具（`def G {α : Type} (A : α)` 一族）在引擎态**不再**落内核 `def_eq mismatch expected: Sort(1) \| actual: Sort(2)` · 新增码则 `protocol.md` + `editor/vscode` + `skills/` 同轮 | 开关 |
| **M4** | **默认开 + 发版**：默认 = 引擎（`sibling` 保留为回归对照 + 两态反向验证的另一半）；bump 两处 → push → CI → tag → release | E19 §9 同款**两态摘要** · 交叉验证：`sibling` 态 `--digest` **逐字节等于今天**（证明只翻默认值）· 全语料 129 文件 **0 差异** · 门禁 43/377/99/0 · CI **28/0** + release 资产核对 | 逃生门 `SOKO_METAVAR=sibling` |

**顺序理由**：M0 先（E19 刀0 纪律：没有基线就没有「判定不变」的判据）→ M1（记法路径最小，E19 刀1
同形）→ M2（一般路径，**此时才碰 §2.6 判据**，E19 刀2 的教训）→ M3（检查与契约，风险最高、最后做）
→ M4（默认开）。
**当前授权（用户 2026-10-01 拍板 D1 = ③）**：**只做 M0** ✓ —— M1–M4 等 M0 的能力清单出来再复议
（D7 = 照做但缩范围）；**M0 不碰任何实现代码** ✓。

## 5. 决策点（**要用户点头**；实现前必须收口）

> **2026-10-01 用户拍板四条**（✅）：**D1 = ③**（先只做 M0）· **D4 = ①**（保留 E19 defaulting）·
> **D6 = ①**（M1–M2 不新增码）· **D7 = 照做但缩范围**。其余三条是**设计默认**（实现时若要改口请说）。

| # | 决策 | 选项 | 结论 |
|---|---|---|---|
| **D1** | **范围** | ① 只做范围 A ② A + 范围 B 设计 ③ 先只做 M0（能力清单 + B 可行性探针），用实测再定 | ✅ **③**：C2 说 A 的接受面增量小，先花最小成本把「值不值得」量出来 |
| **D2** | 宇宙层合一 | 做 / 不做（`Level(name)` 刚性）| 设计默认 **不做**（G-63 留开；约束集是独立机制，塞进来 M1 变两倍大）|
| **D3** | sort 检查强度 | ① 三值语法近似（零内核调用）② ①+`judge_infer` 兜底（待定档才付）| 设计默认 **①**；② 留作 M3 之后的开关候选 |
| **D4** | 未解元变量 | ① 保留 E19 defaulting（接受面 ⊇ 今天）② 严格 Lean 式（未解即失败，接受面变窄）| ✅ **①**（C1：否则 G-48 与 `Or.inl` 一类**重新判红**，课程计数会掉）|
| **D5** | 开关形态 | ① `SOKO_METAVAR=0\|sibling\|engine`（默认 engine）+ 兼容 `SOKO_NOTATION_METAVAR=0` ② 只复用旧开关（两态）| 设计默认 **①**：三态让 M1/M2 有「逐字节等于今天」的回归臂 |
| **D6** | 报错契约 | ① 不新增码（`Clash`/`Kind`/`Occurs` 降级到既有码）② 新增 1–2 个精确码 | ✅ **①（M1–M2）→ M3 再议**：契约变更要同轮改三处，不该塞进接线刀 |
| **D7** | 止损条件 | M0 清单显示「接受面零增量」⇒ 停手 / 缩到「只做 sort 检查 + 代码收窄」/ 照做 | ✅ **照做但缩范围**（C2 三条增量 + B 地基仍在），**不**为凑增量放宽接受面 ✗ |

## 6. 复跑命令（每条可直接粘）

```bash
cargo build --release -p sokonanoda-cli --bin sokonanoda
bash scripts/kernel-diff.sh --digest ./target/release/sokonanoda          # §1.4 三个 sha256
SOKONANODA_BIN="$PWD/target/release/sokonanoda" python3 courses/set-theory/tools/check.py --json
SOKONANODA_BUILD_JOBS=1 SOKO_STAGE_STATS=1 ./target/release/sokonanoda build --json courses/set-theory
./target/release/sokonanoda docs/gaps/repro/G48-notation-nullary-sugar-operands.sokonanoda   # exit 0
SOKO_NOTATION_METAVAR=0 ./target/release/sokonanoda docs/gaps/repro/G48-*.sokonanoda         # exit 1
cargo test -p sokonanoda-front --lib && cargo test -p sokonanoda-cli --test notation
python3 scripts/gap.py check && python3 scripts/docs-lint.py && python3 scripts/status-lint.py
```

## 7. 后续计划（**IA-4 之后**：集合论教材线；队列权威 = `docs/ONBOARDING.md` §0.2）

> 用户 2026-10-01 补充：**两个大项目最终都要做到最终形态**（元参数引擎 + 集合论教材扩展）。
> 集合论线的**前置障碍已解除**（记法求解 E19 `v0.79.0` 默认开 · 性能 P1-b/§3.C 全课 `build`
> **214s → 47.8s**）⇒ 排期 = **M0 之后**动工；**只排期、不写实现** ✓。

| 片 | 是什么 | 入口（先读） | 判据 |
|---|---|---|---|
| **S-A** | **重建卷 I 大纲** `docs/design/set-theory-syllabus.md`（对齐国际一流教材；现行草案口径 = Tao《Analysis I》§3.1–3.6 + epilogue，见 `teaching-project.md` §4.2，可补 Halmos/Jech/Kunen 章节对应表）+ **修全部悬空引用**（**实测 5 处**，不止点名的 2 处：`REQUIREMENTS.md:36` · `teaching-project.md:209` · `course-stdlib.md:10` · `courses/set-theory/README.md:4` · `courses/set-theory/AGENTS.md:80`）+ 新文件进预算表与过期登记 | `teaching-project.md` §4（卷 I 大纲 · 决策点 D-2/D-6）· `course-stdlib.md`（三层分界硬规则 10）| `grep -rn "set-theory-syllabus" --include=*.md .` 每处都指向**存在**的文件 · `docs-lint` ✓（登记 + 层预算）· 课程门禁 **43/377/99/0**（不动课程内容）|
| **S-B** | **卷 I 深化**：公理化集合论（ZFC）· 序数 · 基数算术 · 选择公理（含等价形式）—— 深化后课程单元**仍须全绿**（现 12 单元 `failed=0`）| **先读** `v077-kernel-deficiencies.md`（G-56/58/59/64 = 良基递归 / 大消去 / 递归子宇宙，两条 `blocker`）· `teaching-project.md` §5（每单元 DoD）· 复现件 `docs/gaps/repro/G5{6,8,9}*` `G64-*` | **先探针**（撞不撞内核墙要实测，不猜）· 课程门禁 **12 单元 failed=0** + `check.py` G1–G6 · `notation-lint` 零旧写法 · 撞墙条目**记台账**、**不改内核判定**（用户 2026-09-28 口径）|
| **S-C** | **卷 II 分析起步**：实数构造（Dedekind 分割 / 柯西序列）→ 分析学 | `teaching-project.md` §8 **P7「卷 II 及以后（scale gate 之后）—— 另立设计」** · §1（analysis 项目「大」在哪）· 本文 §2.1 **范围 B**（卷 II 大量依赖隐式实参推断与期望类型传播）| **先出卷 II 设计**（另立文档）+ **scale gate**（规模/性能读数）⇒ 再动课程；判据沿用 `check.py` G1–G6 |

**顺序理由**：S-A 是 B/C 的**判据输入**（没有大纲就说不出「深化到哪算完」）⇒ 先做；S-B 在深化卷 I 的
同时把**内核墙**探明（G-56/58/59/64 的结论直接决定卷 II 的可行边界）⇒ 次之；S-C 要**新设计 + scale gate**，
且很可能依赖 IA-4 的**范围 B** ⇒ 最后。**与 IA-4 的关系**：M0 收口时按能力清单复议 D1（继续 M1–M4 /
转集合论线）；两条线**不并行**动同一批文件（`AGENTS.md` §并行纪律第 2 条：同一模块同一时间只允许一个写者）✓。

**记账**：本设计 + 排期 ⇒ `docs/ONBOARDING.md` **§0.2**（队列唯一入口）+ `STATUS.md`（第 524 轮）；
实现开始后每片的 as-built **追加到本文**（不另开文件）。
