# 交接单：G-68 切片 1（同进程按 module_key 复用依赖产物）

> 快照：2026-09-29 17:1x。**权威仍是 `docs/PLAN-0.74-0.79.md` §P 组 P1′**
> （值守回写）与 `docs/design/incremental-environment.md` §21–§29（细节）。
> 本文只回答「现在在哪、下一步唯一做什么、判据是什么、哪些零件已就位」。

## 1. 目标与收益（实测，不是估算）

`build <dir>` 今天对**每个入口**走 `compile_plan_with_progress` ⇒ 每个入口都把自己的
整条闭包（`lib/*` + 自己）**从头编一遍** ✗ ⇒ 共享 `lib/*` 被编 **42** 次。
这是 **4.14×（174 次模块编译 / 42 入口）** 与「**分片无效**」（单片 **317.71s** ≈
全量 **313.78s**）的**同一个根**。

**切片 1 第一次接线的实测收益**（`21d80489` 记录，`exit=101` 崩但读数有效）：
`passes` **4126 → 271** · `judge_ms` **146.4s → 5.3s** · `doc_passes` **266 → 19**。
⚠ 这个 271 **部分是"judge 走不到"造成的假低**（见 §3 根因）⇒ **不是净收益**。

## 2. 三次接线失败的分类（**别当成同一个问题**）

| 尝试 | 症状 | 根因 | 类别 | 状态 |
|---|---|---|---|---|
| 1 | `exit=101` 越界崩（`project/mod.rs:480`）| `assemble_report` 要**整个闭包逐模块**的 `reports`，而 session 只交 `entry_reports`（长度 1）| 接口 | 已修（`merge_session_reports`）|
| 2 | `compiled:4 failed:38`、`build.decl` **0** | 把 `lib/*` **也当入口**喂进 `entries` | 接口 | 已修（分类）|
| 3 | `compiled:12 failed:30` | **并集前缀 ≠ per-entry 前缀** | 顺序/前缀 | **见 §3** |

## 3. 🎯 根因（接口对账得出，三次都栽在这）

`judge_infer` **只吃源码字符串**（`judge.rs:949`，**没有环境参数**）⇒ 必须**从源码重跑前缀**。
而 session 的入口趟 `run_pass_with(..., entry_units, ...)` 里 `units.len()==1` ⇒
`closure_prefixes` **为空** ⇒ 入口的 `prefix_src` **只有自己**
（`walk.rs:326-334` 走 `_ =>` 分支）⇒ **入口里"问库层声明的类型/宇宙"的 `judge_infer`
看不到库层** ✗。

**⇒ 一句话**：session 只有**一份**库层，而每个入口需要**自己那份**前缀
⇒ **按「库层前缀等价类」分组不是可选项，是让 session 与 per-entry 前缀自洽的唯一办法**。

**N 已实测 = 15**（41 个入口）：库模块编译 **132 → 58**（**2.28×**）；
session 趟数 56 → 56（**不变**，每个入口仍要自己那趟）。

## 3.5 🔑 已修的两个 bug（2026-09-29 17:2x 实测，别重踩）

**(a) 前缀要取"最后一格"**（`3f61ad95` + `d20cb305`）：入口趟的 `units` 是
`entry_units`（**长度 1**）⇒ walk 的 `unit_idx` **恒为 0** ⇒ 要的是
`closure_prefixes` 的**最后一格**；而第 0 格是**空串**。传整个数组 ⇒ 入口拿不到库层前缀
⇒ 实测「前缀源码无法解析」+ `≠`/`{a,b}`/`=`/`∈` 全读不到目标类型。
**修后单入口 `errors=[]`、`compiled:1 failed:0`** ✓。
⚠ **前缀数组按「闭包下标」编号，walk 的 `unit_idx` 按「本趟 units 下标」编号 —— 只跑一个单元时两者不是同一套** ✗。

**(b) 等价类分组**（CLI 侧，`build.rs`）：类键 = 库模块名序列（拓扑序）；
组内库层逐字节相同 ⇒ 与该组入口自己的前缀一致 ✓。**N=15**（41 入口）。
修后全课 `compiled:42 failed:0` ✓（此前 12/30）。

## 3.6 ⚠ **仍未接完：`build.decl` 心跳**（下一个 bug，已定位到形状）

切片 1 的编译发生在 **session 内部** ⇒ tick 必须由 session 转发
（基线由 `compile_all_units_with_progress` 发）。**不转发 ⇒ `build.decl` 2647 → 353** ✗
（`--json` 红线）。

**已试过并失败的形状**：给 `with_project_session` 加
`Option<&mut dyn ProgressSink>` 或 `Option<Box<dyn ProgressSink + '_>>`
⇒ 都在逐趟重借处撞 **E0597（does not live long enough）** / **E0521（borrowed data escapes）**。

**待试形状（按推荐序）**：
1. **每趟一个 sink 的工厂**：`sink_for: impl FnMut(usize) -> &mut dyn ProgressSink`
   —— 生命周期仍可能卡，但语义最贴；
2. **session 内建收集器**：session 自己收 `(entry_index, module, index, total)`，
   调用方从回调里一次取走（**不传 sink**）⇒ 躲开重借问题 ✓ **最可能一次过**；
3. 把入口趟的编译**留在调用方**（session 只编库层）—— 面最大，且会退回"每入口各编一趟"。

**判据**：`build.decl` **2647** · `build.file` **42** · `compiled:42 failed:0` ·
`--json` 逐字节不变。

## 3.7 ✅ 已验证可用的「等价类分组」代码形状（**照抄即可，17:2x 实测跑通**）

CLI 侧 `crates/cli/src/build.rs` 加一个函数，在批量循环**之前**调用一次：

```rust
// 返回与 files 等长的 Vec；None = 该文件不是入口（lib/*）⇒ 调用方走老路。
// 调用点：在 `let jobs = build_jobs(files.len());` **之前**
//   let precomputed = std::sync::Mutex::new(precompute_project_reports(
//       &files, root, no_project, prelude_mode_from_source));
// 并行/串行两条路径把 build_one 的最后一个参数 None 换成
//   precomputed.lock().unwrap()[index].take()
// （Mutex 不是 RefCell：后者不 Sync，thread::scope 里编译不过 ✗）
fn precompute_project_reports(files: &[PathBuf], root: Option<&str>, no_project: bool,
    prelude_of: impl Fn(&str) -> sokonanoda_front::compile::PreludeMode)
    -> Vec<Option<sokonanoda_front::project::ProjectReport>>
{
    // ① 每个文件一个 plan + precheck_plan(&mut plan, &options)   // ← 不跑 = 丢诊断
    // ② 分类：出现在「别的 plan 的非入口模块」里 ⇒ 是库（用 unit.path 收集成 HashSet<PathBuf>）
    //    entry_slots = 那些不被判为库的 plan
    // ③ 分组成 HashMap<Vec<String>, Vec<usize>>：
    //    key = units_for_modules(&plan, |m| m.path != plan.entry) 的 **name 序列**（拓扑序）
    // ④ 每组一次 session：
    //    let lib_units = units_for_modules(&plans[first].1, |m| m.path != plans[first].1.entry);
    //    let closures[i] = units_for_modules(&plans[members[i]].1, |_| true);
    //    let entries[i]  = units_for_modules(&plans[members[i]].1, |m| m.path == plans[members[i]].1.entry);
    //    with_project_session(&lib_units, &entries, &options, |i, out, entry_reports,
    //        lib_reports, _r, _e| {
    //        let reports = merge_session_reports(&closures[i], &lib_units, lib_reports, entry_reports);
    //        assemble_from_session(&plans[members[i]].1, out, reports) })
    //    ⇒ out[plans[slot].0] = Some(report)
    out
}
```

**实测结果（17:2x）**：全课 **`compiled:42 failed:0`** ✓（接之前是 12/30）·
`passes` **4141** · `judge_ms` **146.9s** · 单入口 `errors=[]` ✓。

**⚠ 唯一缺口就是 §3.6 的 `build.decl` 心跳**（2647 → 353）。

## 4. 下一步（唯一，新会话直接做）

1. **已修**：session 入口趟现在传"该入口闭包"（`lib_units ++ entry_units`）的前缀与记法表
   （`3f61ad95`）✓ —— 这是 §3 根因的修法（路乙：`run_pass_with` 加两个**可选**参数）。
2. **待做**：**CLI 接线**，且必须**按等价类分组**（N=15）：
   同类的入口共用一个 session（类内库层前缀**逐字节相同** ⇒ 语义与基线一致 ✓）；
   不同类各用各的 session。
3. **待做**：报告拼接用 `merge_session_reports`（类内顺序一致 ⇒ 不需重排）。
4. **待做**：喂 `build_one` 的 `precomputed`
   （`crates/cli/src/build.rs:367` 那个参数**从切片 1b 起就留着，一直没人喂过**）。

**⚠ 两个不许**：**不许批编**（实测慢 **6.2×**）· **不许按前缀复用**
（切片 1b 实测 `passes` 4126→**5404**、`judge_ms` 148.4→**162.9** ⇒ 更慢）。

## 5. 判据（一条都不许少）

* **`--json` 逐字节不变**（红线）：`build.decl` **2647** · `build.file` **42** ·
  `build.begin` **1** · `build.summary` **1** · `compiled:42 failed:0`
  （`build.tick` 带 `elapsed_ms` ⇒ **按设计**随墙钟变，比对时排除）；
* **先报 `failed` 与计数，再看墙钟**；**174→42 是计数 ≠ 快 4 倍**；禁止「大幅提速」；
* **反向判据**：改任一依赖 ⇒ 该 module_key **必须 miss 重编**
  （守卫 `slice1_changing_a_dependency_forces_recompile` 已绿，实现写错它会红）；
* **删掉** `slice1_shared_module_is_compiled_once_across_entries` 的 `#[ignore]`
  ⇒ **必须转绿**（它现在是 TDD 的"先红"守卫）；
* 同机同口径；方差 ±2.7% ⇒ **<5% 差不许当结论**。

## 6. ⚠ 已上 main 但**零调用点**的零件（**别重造**）

| 零件 | 位置 | 作用 |
|---|---|---|
| `ExportFile::infer_type_text_at` | `crates/kernel/src/util.rs` | 同一 `ExportFile` 上对**已 elaborate** 的 `ExprPtr` 求类型文本 |
| `EnvBuilder::with_env_scope` | `crates/kernel/src/builder.rs` | **同时**借只读 `Env` 与可变 builder（`Env` 生命周期放宽到 `'x` 是为它让路）|
| `assemble_from_session` | `crates/front/src/project/mod.rs` | 把 session 结果组装成 `ProjectReport`（够得着私有 `closure`）|
| `merge_session_reports` | 同上 | 按**该入口自己的**闭包顺序拼 `lib_reports` + `entry_reports` |
| `precheck_plan` | 同上 | 跑那两步闭包级检查（`check_name_collisions`/`check_prelude_conflicts`）——**不跑 ⇒ 丢诊断 ⇒ `--json` 会变** |
| `closure_prefixes_for` | `crates/front/src/compile/check/mod.rs` | 闭包前缀的**唯一实现**（已 re-export）|

**内核两笔**（`c625cffc` / `a3545f03`）三层回归 + `--json` 逐字节均已验 ✓，
**无条件内核授权**（用户 14:12）。

## 7. 今天的其他结论（别重踩）

* **分片无效**（决定性测量）：单片 **317.71s** ≈ 全量 **313.78s** ⇒ 已**精确 revert**
  （保留课程产物缓存 **628×** 与 `ci-green.py --selftest` 夹具修复 **6/6**）；
* **批编已否**（慢 **6.2×**）· **切片 1b session 接线实测不提速**（252.7s vs 218.8s）；
* **三条候选判死两条**：B（`ExportFile` 的 `mk_*` 在 `with_ctx` 的局部 arena ⇒ 指针同一性不成立）·
  D（`judge_infer` 结果在 `walk` 中途被消费 ⇒ 不能后移）；
* **perf 判据不许用绝对毫秒**（共享 runner 7.5×，已栽三次）⇒ 用结构计数或比值。

---

# 附：P1-a 的 0 号动作答案（2026-09-29 17:5x，**含权威计数**）

## 0. 🔢 权威计数（本轮实测，`SOKO_STAGE_STATS=1`，release · 冷缓存 · 1 job）

| 读数 | 值 | 含义（已核对代码） |
|---|---|---|
| **墙钟** | **217.27s** | 全课 `build --json` |
| `passes` | **4126** | `check::stage_stats::PASSES` = **`run_pass_with` 的调用次数**（`check/mod.rs:928`）|
| `doc_passes` | **266** | `VIA_CHECK_DOCUMENT` = 真文档趟数 |
| ⇒ **judge 子趟** | **≈ 3860** | `4126 − 266` ⇒ **每次文档 pass 要跑 ≈ 14.5 趟 judge 前缀** |
| `judge_ms` | **147.8s** | judge 累计（**含**在下面的 `by_total_ms` 里）|
| `by_calls` / `by_total_ms` | **69085** / **189.7s** | `by` 引擎（tactic）累计 |
| `pass_total_ms` | **596.6s** | **所有 pass 累计**（4126 趟 × 平均 145ms）|
| `hits`/`misses` | 66683 / 266 | judge 缓存 |

**⇒ 三个结论（修正旧说法）**：

1. **judge 占墙钟 ≈ 147.8 / 217.3 ≈ 68%**（若 judge 时间与别的阶段重叠少）。
   ⚠ 旧的「88%」出自**另一次口径**（219.3s→26.8s 的跳法比较），**不要直接引用** ✗；
2. ⚠ **旧的「合成 pass 253513 次」与今天的 `passes=4126` 不是同一个计数器** ——
   前者是 `judge::stats::PASSES`（judge 内部子趟），量级 253513 ≈ 2647 声明 × 96
   ⇒ **judge 内部那套放大是真的**，但它**不等于** `STAGE_STATS` 的 `passes`；
3. **`pass_total_ms` 596.6s ≫ 墙钟 217s** ⇒ 有并发/重叠，**逐项相减是错的** ✗。

## 1. P1-a 的收益上界（**先估，再动手** —— 值守 17:46 的要求）

* **可兑现上界 = `judge_ms` = 147.8s**（那是"从零重跑前缀"的全部时间）；
* 若就地查环境能消掉 **50–70%** 的 judge ⇒ 省 **74–103s** ⇒ 墙钟 **217s → 114–143s**
  （**1.5–1.9×**，**不是** 4×）；
* ⚠ 但「50–70%」**目前是猜的** ⇒ **P1-a 的第一件事就是把这个比例量出来**（见 §2 建议）：
  在 `judge_infer` 里按"答案来源"分类计数（`Env` 表查到 / 必须重跑前缀），
  **只加计数器、不改判定**（与 `SOKO_NO_JUDGE` 同性质，且**只许量成本**）。

## 2. 0 号动作的答案：**为什么落了零件却没接通 judge？—— 两者都有，且障碍是真障碍**

**（a）被切片 1 岔开了** —— 时间线（commit 为证）：`c625cffc`（`infer_type_text_at`，15:01）·
`a3545f03`（`with_env_scope`，15:19）落完之后，**立即转去切片 1 接线**
（`3f61ad95`…`9543405a` 全是切片 1）⇒ **judge 那条线一次都没试过** ✓。

**（b）接通前有两个**真**新障碍**（今天已逐条查实，不是猜）：

* **障碍 1（主）**：`judge_infer` 收的是 **`term: &str`（源码文本）**，
  而 `infer_type_text_at` 要 **`ExprPtr`（已 elaborate 的内核项）** ⇒
  **调用方手里没有那个 `ExprPtr`** —— 10 个调用点（`elab.rs:1363/1396/2089/3512/4212/4658/4700/4702/5136`）
  传的是 `render_expr(operand)` **渲染出来的字符串**。
  ⇒ **要么**先把该文本 elaborate 成 `ExprPtr`（需要"在同一个 arena / 同一份 `ExportFile` 上
  由源 `Expr` 造项"的能力 —— 即 **§19.4 的候选 A / K-2**，**尚未实现**），
  **要么**绕过 elaborate：把"**裸常量名的类型**"这类**最常见**的查询做成**直接查表**
  （`Env::get_declar` 给出 `Declar` ⇒ 取它的类型 ⇒ 打印）。
* **障碍 2**：`with_env_scope` 只能在 `elab_expr` **已经可变借走** `EnvBuilder` 的调用栈里
  重入 ⇒ 需要"**重入安全**"的调用形状（把 builder 临时还回来 / 传闭包而不是借用）——
  **尚未验证**。

## 3. ⇒ P1-a 从「**补零件**」开始，不是从「接线」开始

**第一步（最小，且能一次量出上界）**：在 `judge_infer` 里**只加分类计数**：
每次调用时判断"**这个查询能不能用裸常量查表答**"（源文本是 `Name` 或 `Name ...` 头），
分别计数 + 分别累计耗时 ⇒ **得到"可消掉的比例"** ⇒ 再决定 P1-a 怎么切。
**只加计数、不改判定** ⇒ 零语义风险 ✓。

**第二步**：按第一步的结果，挑**占比最大的那一类**做就地查表（`Env::get_declar` 或
`infer_type_text_at`），**源码重跑保留为开关**（可 A/B、回退便宜）。

**判据（每一步都要）**：`--json` **逐字节不变** · 报出 `passes`/`doc_passes`/`judge_ms`
**三个数** · 反向判据（前缀真变必须重算）。

---

# 附二：P1-a **接口对账清单**（第 4 次动手之前必须有；值守 17:5x 要求）

> 今天三次走空**全是接口/顺序问题**，不是算法问题 ⇒ 这份表逐点一行、附一句话风险。

## A. 签名会动到哪里

| # | 对象 | 现状 | 要变成 | 风险 |
|---|---|---|---|---|
| A1 | `judge_infer`（`judge.rs` 入口）| `(prefix_src: &str, options, binders, term: &str) -> Result<String,_>` | 多收一个**环境句柄**（见 C）| **签名改动面 = 10 个调用点**；漏一个 = 编译错（好抓）|
| A2 | `judge_infer_with`（`judge.rs:949`）| 同上 + `extra_prefix` | 同上；**保留原签名**，新加 `*_with_env` 兄弟函数 | 保留原函数 ⇒ **回退零成本** ✓ |
| A3 | `judge.rs` 的缓存（键含**整段前缀哈希**，`:932`）| `HashMap<前缀哈希, 结果>` | **就地路径不查这个缓存**（它本身就是免前缀的）| 两条路**结果必须一致** ⇒ 见 D（影子核对）|

## B. 10 个调用点逐点（`crates/front/src/compile/elab.rs`）

| 行 | 函数 | 查的是什么 | 有源 `Expr`？ | 能否"裸常量查表" |
|---|---|---|---|---|
| `1363` | `notation_prefix_args` | `head` 的类型 | ✓ `head` | 部分（`head` 常是常量/应用头）|
| `1396` | 同上 | 上一步 `ty_text` 的 sort | ✗（只有文本）| **否**（输入已是文本）|
| `2089` | `infer_type_text` | `operand` 的类型 | ✓ `operand` | **是**（最常见一类）|
| `3512` | `let` 缺标注 | `val` 的类型 | ✓ `val` | 部分 |
| `4212` | λ binder 补类型 | `args[n]` 的类型 | ✓ `args[n]` | 部分 |
| `4658` | `match` scrutinee | `scrutinee` 的类型 | ✓ `scrutinee` | 部分 |
| **`4700`** | **`infer_expected_level`** | `R` 的 **sort** | ✓ `expected_src` | **是**（`sort_text_level` 只要 `Sort(n)` 文本）|
| `4702` | 同上（另一分支）| 同上 | ✓ | **是** |
| `5136` | `field_sort_via_kernel` | `src_ty` 是否 `Prop` | ✓ `src_ty` | **是** |
| — | `judge_pairs`（`by` 引擎）| — | ✗ | 不在本次范围 |

⚠ **`1396` 那处输入已是文本**（`ty_text` 来自上一趟）⇒ **它天生不适合就地路径**
⇒ 必须保留 `judge_infer` 的原样调用 ✓（这正是"**保留源码重跑**"的必要性）。

## C. 环境句柄从哪来（**这是真障碍**）

| # | 问题 | 现状 | 风险 |
|---|---|---|---|
| C1 | `judge_infer` 的调用点**没有** `&mut EnvBuilder` | `elab_expr` 已经可变借走它 | 需**重入安全**的形状（闭包传参 / 临时还回）—— **尚未验证** |
| C2 | `infer_type_text_at` 要 `ExprPtr` | 调用点只有**源 `Expr`** + 渲染文本 | ⇒ 要么先 elaborate（**需要 §19.4 候选 A / K-2，未实现**），要么只做"**裸常量查表**"（`Env::get_declar`）✓ |
| C3 | `with_env_scope`（`builder.rs:138`）| 已上 main、零调用点 | 借出 `(&Env, &mut EnvBuilder)`；⚠ 回调期间两张表被 `mem::take` 走 ⇒ **回调里不许依赖 `declars`/`notations`** |

## D. 哪里会破 `--json` 红线（**逐条**）

| # | 破法 | 防法 |
|---|---|---|
| D1 | 就地路径与重跑路径**结果不同**（判定语义变）| **影子核对**：先两条都跑、**比对文本**，不一致就**报错并回退**（不静默）|
| D2 | `binders` 语义不同（`judge_infer` 把 binder 包成 `fun … =>`，`infer_type_text_at` 收**闭项**）| 就地路径**必须自己**处理 binder（包 λ 或忽略）—— **逐点核对** |
| D3 | 缓存命中/未命中改变**事件顺序或计数** | 就地路径**不写**原缓存；`passes`/`doc_passes` 会变 ⇒ **只许降、不许变语义**；`--json` 逐字节兜底 |
| D4 | 错误路径不同（`judge_infer` 失败时调用点 `?`/`ok()?` 的行为）| 就地路径**答不出就返回 `None`** ⇒ 调用方**自动回退**（与 `EnvProvider::infer_type_text` 的 `None` 约定一致 ✓）|

## E. ③ 开关形状（现在定死，收口时不临时塞）

* **env 名**：`SOKO_JUDGE_INPLACE`（模式串，非布尔）：
  * 未设 / `off`（**默认**）⇒ 只走源码重跑（**今天的行为，逐字节不变**）✓；
  * `shadow` ⇒ **两条都跑**，比对，**不一致时报错**（不静默）⇒ **上线前的核对档**；
  * `on` ⇒ 就地优先、答不出回退（判定语义必须与 `off` 逐字节一致）。
* **判定点（唯一的那个）**：**只**在 `infer_expected_level`（`elab.rs:4690`，含 `4700`/`4702`）
  + 可选 `field_sort_via_kernel`（`5136`）—— **P1-a 只做这一组**，铺开是 P1-b ✗。
* **默认值**：`off`；`on` 只在 D 的影子核对**全绿**之后才考虑改默认 ⇒ **P1-a 不改默认** ✓。
