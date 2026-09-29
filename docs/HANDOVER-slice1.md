# 交接单：G-68 切片 1（同进程按 module_key 复用依赖产物）

> 快照：2026-09-29 17:1x。**权威仍是 `docs/PLAN-0.74-0.79.md` §P 组 P1′**
> （值守回写）与 `docs/design/incremental-environment.md` §21–§29（细节）。
> 本文只回答「现在在哪、下一步唯一做什么、判据是什么、哪些零件已就位」。
> **量具读数 / 0 号动作答案 / 接口对账清单 / 决定性分布 ⇒ 见
> [`docs/design/p1a-measurements.md`](design/p1a-measurements.md)**（283 行，动手用）。

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
