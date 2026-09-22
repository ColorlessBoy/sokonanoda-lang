# `by` 判定的复用方案（G-31 / T-K20′）

> 2026-09-21。起因：用户质疑「lean 的 by 风格有这么耗时吗？是不是我们的 by 的实现
> 方案有问题呢？」——**质疑成立**（缺口 G-31）。用户进一步要求：
> 「那要插入方案进计划里，这个违背我们的红线，也违背我们的热编译的设计初衷。」

## 1. 现状：判定每批重跑整份前缀

`crates/front/src/judge.rs::judge_pairs_uncached` 的最后一步：

```rust
let report = check_document_with(
    &FolFile { commands, src: full_prefix },   // 整份前缀 + 合成的 `_soko_judge_k`
    options,
);
```

`check_document_with` → `run(&[SourceUnit::single("", file)], …)` → `run_pass` → 从头
elaborate + 内核检查**整份文档**。

**实测（`unit12-solution`，25 个 `by` 块，`SOKO_JUDGE_STATS=2`）**：

| 指标 | 值 |
|---|---|
| `check_document_with` 调用 | **25**（每个 `by` 块一次） |
| 前缀字节 | 13,286 → 83,187 **单调增长** |
| 合计处理 | **1,064,669 字节 ≈ 文件本身的 13 倍** |
| 单次最贵 | **28.6 秒** |
| 判定占整文件墙钟 | **68%**（81.9s / 120.4s） |

**同一批定理写成项风格快 8.4×**（10 个定理 0.42s → 0.05s；`by-tactics.md` §13
记着同一份解答 91.2s → 3.6s = 25×）。

**Lean 不是这样**：`by` 是普通命令，按顺序 elaborate **一次**，tactic 逐步改
goal state，前面的声明不重跑（`Lean.Server.FileWorker`："elaboration is executed
in a chain of tasks, where each task corresponds to the elaboration of one command"）。

## 2. 为什么今天这么写

`run_pass`（`crates/front/src/compile/check/mod.rs:497`）每次调用都：

```rust
let arena = stumpalo::Arena::new();                                  // ← 新 arena
let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
let mut known = KnownTable::new();
let mut inductives = InductiveTable::new();
let mut defs = DefTable::new();
... install_prelude(...) ...
Walk::run(...)
```

整套状态（arena / builder / 三张前端表）都是**这次 pass 的局部变量**，pass 结束就没了。
所以"再判一条"只能整份重来。`docs/design/by-tactics.md` §13 把"复用环境"写成**死路**：

> `EnvBuilder` 字段私有、`new(arena, config)` 是唯一构造入口、`finish(self)` 消费
> 自身；`ExportFile` 只读……名字下标（`NamePtr::decl_idx`）绑定在构建它的 builder 上，
> 拿新 builder 造出来的 `Declar` 去查旧环境会查错槽位。

**但那是在内核冻结（硬规则 1）时写的。** 内核现已解冻，用户明确授权为性能改内核
⇒ **这条结论过期了，必须重新评估**。

## 3. 内核的门其实是开着的（已核对的公开 API）

| API | 位置 | 说明 |
|---|---|---|
| `EnvBuilder::new(arena: &'a ArenaRef<'a>, config) -> Self` | `builder.rs:43` | 唯一构造入口，但**接受外部 arena** ⇒ arena 可以比一次 pass 活得久 |
| `EnvBuilder::add_declar(&mut self, d: Declar<'a>)` | `builder.rs:238` | 内部 `name.set_decl_idx(declars.len())` ⇒ **下标 = 插入顺序** |
| `EnvBuilder::finish(self) -> ExportFile<'a>` | `builder.rs:301` | 交出 `declars`/`notations`/`dag` |
| `Env::new(declars: &'a DeclarMap<'a>, notation: &'a NotationMap<'a>, limit) -> Self` | `env.rs:241` | **接受已有的 `DeclarMap`** ⇒ 从检查点建 Env 是现成的 |

**约束（真正要处理的）**：

1. **arena 生命周期**：`Declar<'a>` / `NamePtr<'a>` 都借 arena。要复用就必须让
   arena 活过单次 pass（`docs/architecture.md` §6 的 arena 契约）。
2. **`decl_idx` 与插入顺序绑定**：复用时必须**原序**回放（或保证同一个 map 实例）。
3. **`NatLit` 按指针比较**（`conv.rs:169`）：任何"克隆出来的快照"必须与新建声明
   在**同一个 arena** 里，否则判定会变（这是 T-K13 `snapshot` 的已知陷阱）。
4. **两趟结构**：`run` 会跑两趟（第二趟带 `skip: Option<&KernelFailed>`）。检查点
   必须说明是哪一趟之后的状态。
5. **`TrustPlan`**：`run_pass` 还带 `trust: Option<&TrustPlan>`（I13-S1 的
   `by` 引擎信任计划），复用时要一并考虑。

## 4. 三个候选

### A. **会话化**（Lean 模型，终局）

把 arena / builder / known / inductives / defs 提成一个 `Session`：

```rust
pub struct Session<'a> { arena: Box<Arena>, builder: EnvBuilder<'a>, known, inductives, defs, … }
impl Session<'_> { pub fn feed(&mut self, commands: &[Command]) -> Vec<DeclOutcome> }
```

`run_pass` 变成 `session.feed(...)`；`by` 块在**当下环境**里跑引擎，**不再有判定批处理**。

* 收益：**O(B²) → O(B)**，理论上与项风格同量级；这也是唯一"对得上 Lean"的方案。
* 代价：`run_pass`（~200 行）+ `Walk` 的所有权要重构；两趟结构、`skip`、`trust`
  都要在新的生命周期下重述。
* 风险：**高**——这是把整个编译器的状态模型从"一次调用"改成"长驻会话"。

### B. **检查点复用**（判定侧，改动有界）

正常那一趟结束后**留住** `ExportFile`（`declars` + `notations`）与三张前端表
（或它们的可重建快照）；判定时：

1. 从检查点建 `Env`（`Env::new(&declars, &notations, limit)`）；
2. 合成文档**只含 `_soko_judge_k`**（不再拼前缀）；
3. 在这份 Env 上跑"elaborate 这几条 + 内核检查"。

* 收益：去掉"前缀重新 elaborate + 重新内核检查"这部分。**能不能去掉 68% 要看
  拆解**——见 §5。
* 代价：前端要有"从检查点建表"的入口；判定路径要能吃一个外部 Env。
* 风险：中。arena 活到判定结束即可（可以在 `compile_plan` 的作用域里保活）。

### C. **窄口子：`check_term_against`**

只加一条前端 API：给定（检查点 Env，目标类型文本，候选项文本）⇒ elaborate + 内核
检查，**完全不做文档合成**。

* 收益：直接干掉判定路径里最大的一块（文档合成 + 前缀重跑）。
* 代价：最小；但要求前端 elab 能接受"外部环境"。
* 风险：低。可以作为 B 的第一步。

## 5. 分段账单（T-K20′ 的判据，2026-09-21 实测）

`SOKO_STAGE_STATS=1`（新加的常驻开关，`crates/front/src/compile/check/mod.rs`）
在进程退出前打出全程累计：

```
STAGE_STATS passes=706 pass_total_ms=331982 by_calls=10272 by_total_ms=136021 judge_ms=82707
real 121.70
```

**这比"25 次判定调用"严重得多**：

| 指标 | 值 | 读法 |
|---|---|---|
| **`run_pass` 调用次数** | **706** | 不是 25！每次判定调用内部都会再跑一整趟 pass，而那些 pass 里又遇到 `by` 块 ⇒ **递归的重跑** |
| `pass_total_ms` | 331,982ms | 各趟**嵌套累加**（所以大于墙钟 121.7s） |
| `by_calls` | **10,272** | `by` 引擎被跑了 1 万多次 |
| `by_total_ms` | 136,021ms | 同上，嵌套累加 |
| `judge_ms` | **82,707ms** | 判定累计（含嵌套）= **墙钟的 68%** |
| 墙钟 | 121.7s | |

**结论（选刀依据）**：

1. **判定（含其内部的递归 pass）占 68%**，与采样一致；
2. **递归才是主要矛盾**——706 趟 pass 里有 681 趟是判定引发的；
3. 采样给出的成分（8 秒窗口，5474 样本）：`run_by` 47.2% · `elab_expr`
   **28.3%（前端）** · `infer_value` **6.0%（内核）** · `check_declar_at`
   **1.6%（内核）**
   ⇒ **内核自身只占个位数**，**只做 `with_env` 拿不到大头**；
   必须同时消掉**前端的重新 elaborate**。

**所以：B/C 只能缩短每一趟，A（会话化、一趟走完）才消得掉 681 趟。**

### 5.1 具体修法（已逐条核对过 API，2026-09-21）

**关键发现：`by` 引擎根本不需要"合成文档"**——它只需要"把候选项对目标类型做一次
类型检查"，而这条路在**当前 pass 里**是通的：

| 已有的东西 | 位置 | 用途 |
|---|---|---|
| `Walk { builder: EnvBuilder<'arena>, … }` | `compile/check/walk.rs:36-37` | **当前 pass 的 builder 就在手边** |
| `TypeChecker::check_declar(&self, d: &Declar)` | `kernel/src/tc.rs:75` | "检查一条声明"的入口 |
| `Env::new_w_temp_ext(declars, temp_declars, notation, limit)` | `kernel/src/env.rs:247` | 临时声明**不进 `declars`**（不污染环境） |
| `EnvLimit::{Empty, ByIndex, ByName, PpUnlimited}` | `kernel/src/env.rs:253-260` | 可见性切一刀 |
| `TypeChecker::new(dag, env, arena, declar_info, tc_cache)` | `kernel/src/tc.rs:256` | 就地建一个检查器 |

**改动形状**（前端为主，内核只补一两个访问器）：

1. 把 `Walk.builder`（或一个就地建的 `TypeChecker`）**传进**
   `lower_value` → `lower_by_val` → `run_by` → `judge_*`（现在只传了
   `inductives`/`defs` 两张前端表）；
2. `judge_pairs_uncached` **不再**拼前缀、不再 `check_document_with`，改成：
   为每个 pair 建一条 `Declar { name: _soko_judge_k, ty, val }`，
   在**当前 env**（+ 需要的话 `new_w_temp_ext`）上 `check_declar`，读结果；
3. 于是**没有前缀重跑、没有递归**：706 趟 → **1 趟**。

**注意**：内核的检查现在是**延后**的（`Walk` 收 `PendingOp`，`kernel_phase` 统一
检查）。所以第 1 步可能需要"在 walk 里就地建一个 `TypeChecker`"而不是复用
`kernel_phase` 那个——这正是 §5.1 要落地的第一件事。

**风险与护栏**：
* `TcCtx`/`TcCache` 的所有权与 arena 生命周期（`docs/architecture.md` §6）；
* 就地检查会不会改变**判定结果**——`kernel-diff.sh --fast` 每步跑，全量对拍合入前跑；
* 判定失败时的**诊断文本**要与今天逐字一致（学习者看到的是它）。

## 6. 建议的顺序

1. **先做 §5 的分段账单**（只读、便宜、决定后面所有取舍）；
2. **再做 C**（窄口子，风险低）——如果分段账单显示"文档合成 + 前缀重跑"就是大头，
   C 能直接拿掉它；
3. **B 视 C 的结果**——C 不够再上检查点；
4. **A 是终局**，但它应该是"确认 B/C 到顶了"之后的决定，不是第一步。

**护栏**（每一步都必须过）：`scripts/kernel-diff.sh --fast`（每步）+
全量对拍（合入前）+ `cargo test --workspace --locked` + 课程门禁计数逐项不变。

**与线 L 的关系**：线 L（解答改项风格）是**绕开**，不是修复。K1 落地且 `by` 变便宜
之后，**重新评估** `solutions/` 要不要用回 tactic——那才是课程的原始意图。

## 7. 第二个入口：`judge_infer`（G-34，2026-09-21 实测）

> **这一节改变了上面 §5 的结论。** §5 的账单是在 `unit12-solution` **还是
> tactic 风格**时量的（`by` 判定占 68%）。把解答改成项风格之后（线 L），
> `by` 判定掉到 11 次 / 1.4s，**但整份文件仍然要 11.4–12.0 秒**——大头换了人。

**新账单（release，冷缓存，`SOKO_JUDGE_STATS=1`）**：

```
JUDGE_STATS calls=11 total_ms=934 pairs=18          ← by 判定：已经不是问题
JUDGE_INFER calls=126105 total_ms=12304 fails=626   ← 记法消解：新的大头
real 11.96
```

`judge_infer` **12.6 万次调用**，其中 **363 次未命中**——而未命中一次就是
`judge_infer_uncached` 合成 `<前缀>#check fun (binders) => term` 交给
`check_document_with`，**把整段前缀从零重跑一趟 pass**。定位它的办法是两个新的
常驻开关：`SOKO_PASS_TRACE=<n>`（第 n 趟 `run_pass` 的调用栈）与
`SOKO_INFER_TRACE=<n>|all`（每次未命中的查询 + 栈）。

**调用方**（`infer_type_text` 的入口，按量排）：

| 入口 | 问什么 | 能不能就地答 |
|---|---|---|
| `solve_prefix_args`（`elab_notation`） | 操作数的类型，用来反解记法的前导参数（`∈` 的 `α`） | **局部变量可以**（书写类型就在 `ElabScope` 里）⇒ 已修，T-K22 |
| `universe_level_text_of_operands` | 操作数类型的 sort（`=` 的宇宙层级） | 要问内核（但可以少问一次） |
| `application_arg_expected` | 应用**头**的类型（`f x ∅` 里 `f x`） | 复合项，要内核 |
| 冗余 `sorry` 探针（`redundant_probes`） | `fun (__soko_render : T) => __soko_render` 过不过 | 要内核 |

**已经做掉的一刀（T-K22）**：局部变量的类型先问 `ElabScope::source_type_of`
（书写类型，零内核调用；与 `implicit.rs` 里 `arg_tys` 的既有判据完全相同）。
126,105 → 51,156 次调用、363 → 247 次未命中、11.4s → **7.5s**。

**剩下的 247 次没有局部类型可拿** ⇒ 只能靠 §5.1 的"就地拿当前 pass 的环境"。
所以 **§5.1 的设施要同时覆盖 `judge_pairs` 与 `judge_infer`/`judge_type_of`**：
两者是同一个病（合成文档 + 整前缀重跑），只是一个喂 `Declar`、一个喂 `#check`。
判据也要同时看两个计数——`JUDGE_STATS` 的 `calls` 与 `JUDGE_INFER_SPLIT` 的
`misses`，**都要落到"一趟 pass 的量级"**。
