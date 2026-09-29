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
