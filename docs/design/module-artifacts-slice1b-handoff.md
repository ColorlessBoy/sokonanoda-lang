# 切片 1b 最后一刀：交接书（零探索开工）

> 目标：把 `ProjectSession`（已落地）接进 `build <dir>`，让**共享库层只编一次**。
> 已达成：**判据 ② `by_calls` 3 → 1**（`crates/front/tests/session_reuse.rs`，CI 真绿）。
> 本文只讲**怎么接**，不讲为什么（为什么见 `module-artifacts.md` §9）。
>
> **✅ 状态（2026-09-30 核实）：已收口 —— 本文降级为 as-built 记录，"怎么接"不再需要执行。**
> 接线在：`crates/cli/src/build.rs` 的 `build_one(…, precomputed)` 吃 `with_project_session`
> 预先算好的 `ProjectReport`；§4 的根因修法（`PassTables` 跨趟复用，`check/mod.rs:137`）也在。
> 守卫：`crates/cli/tests/imports.rs`「切片 1b 的守卫（多入口 `build <dir>`）」+ `session_reuse.rs`。
> **仍未做**：§5 的**刀 2（模块级产物落盘 / P3 可下载 cache）** —— 跨进程复用的前提。

## 1. 落点（as-built：这些现在都在代码里 ✓）

* **前端**（`crates/front/src/project/mod.rs`）：`PlanCompiled`（`pub(crate)`：`units` / `flat_out` /
  `reports` / `compilable` / `diagnostics` / `closure`）· `compile_plan_units` · `assemble_report`；
  老路径 `compile_plan_with_progress` = 编 + 组装（语义逐字不变 —— `--json` 逐字节等价的前提）。
* **CLI**（`crates/cli/src/build.rs`）：`build()` 在逐文件循环**之前**对同一 `root` 的项目文件
  `plan_project` → 按 digest 过滤命中 → `lib_units`（各未命中闭包**非入口模块**的拓扑去重并集）+
  `entries[i]` → **一次** `with_project_session(…)` → 回调里组成 `PlanCompiled` ⇒ `assemble_report`
  ⇒ `HashMap<PathBuf, ProjectReport>` ⇒ 循环里以 `build_one(…, precomputed)` 传入。
* ⚠ **红线见 §3**（不许用近似判据替代既有判定口径）。

## 2. 三条判据（可直接复制）

```bash
# ── 判据 ①：--json 逐字节等价（错编红线）────────────────────────────
cargo build -p sokonanoda-cli --locked
./target/debug/sokonanoda build --json courses/set-theory > /tmp/before.json   # 接线**前**的 HEAD
# （接线并 cargo build 之后）
./target/debug/sokonanoda build --json courses/set-theory > /tmp/after.json
diff /tmp/before.json /tmp/after.json && echo "① 逐字节等价 ✓"

# ── 判据 ③：改依赖必须 miss 重编（反例，缺一不可）──────────────────
cargo test -q -p sokonanoda-cli --test imports --locked        # 既有反例判据（缓存命中/依赖变失效）
cp courses/set-theory/lib/Logic.sokonanoda /tmp/Logic.bak
printf '\n-- touch: dependency changed\n' >> courses/set-theory/lib/Logic.sokonanoda
./target/debug/sokonanoda build --json courses/set-theory | grep -c '"type":"build.file"'   # 必须 > 0（重编）
cp /tmp/Logic.bak courses/set-theory/lib/Logic.sokonanoda

# ── 判据 ⑤：真课程 174 → 42 与 222.1s → ?（基线见 docs/perf/rebuild-baseline-2026-09-28.md）
SOKO_STAGE_STATS=1 ./target/debug/sokonanoda build --json courses/set-theory > /tmp/after.json 2> /tmp/stage.txt
grep STAGE_STATS /tmp/stage.txt        # by_calls（改前 69085）与 passes（改前 4126）
python3 - <<'PY'
import json, time
# 墙钟：用 /tmp/after.json 的 build.file 事件时间差或直接 time 整条命令
PY
time ./target/debug/sokonanoda build --json courses/set-theory > /dev/null
```
模块编译次数（174 → 42）用**静态口径**复核：`Σ 各入口闭包模块数 = 174`（`courses/set-theory` 42 模块），
接线后每个 `lib/*` 只被编一次 ⇒ 实际编 42 个模块。

## 3. 红线（违反即判红）

* **不许**用 `out.errors.is_empty()` 之类**近似替代**既有判定口径（`entry_module().events.errors` +
  `project.has_errors()`）—— 那会改判定 ✗；必须走 §1 的 `assemble_report`（落点）复用原逻辑。
* **`crates/kernel/` 零改动**（全程只用既有公开 API）。
* 判绿口径：`python3 scripts/ci-green.py --run <id>` **exit 0** + 重活 **10/10** + **failure 0**；
  **本地 gate 绿不算绿**。一次只让一条 run 活着，**推一次就停**。

## 4. 2026-09-29 实测：最后一刀的**真根因**与修法（已冻结）

**症状**：CLI 接线后 `build <dir>` 多入口 `failed: 3/4`；而**前端 session 本身正确**
（加强后的 `session_reuse` 绿：入口无 errors、库层报告齐）。

**真根因**（决定性实验：把判据从**裸 `SourceUnit`** 换成 **`plan_project`** 的单元后立刻转红）：
```
入口 0 有错误（CLI 路径）：["unknown identifier `Nat`"]
```
prelude 的 `Nat`/`Bool`/`Eq`/L1 登记在 **`run_pass_with` 每趟重建**的三张表里
（`crates/front/src/compile/check/mod.rs:846-849` 的 `KnownTable`/`InductiveTable`/`DefTable`），
而 `hide_declars`/`restore_declars` 只搬 `declars` ⇒ **入口趟看不到 prelude** ✗。
（装 `install_preludes=true` ⇒ `duplicate declaration Nat`；`false` ⇒ `unknown identifier Nat`。）

**修法（纯前端，不碰 `crates/kernel/`）**：让三张表**跨趟复用** ——
1. `check/mod.rs`：加 `pub(crate) struct PassTables<'a> { known, inductives, defs }`；
   `run_pass_with` 加形参 `tables: PassTables<'a>`（`:846-849` 不再 `new()`；`:853-855` 借用；
   `:953-955` 移入 `Walked`），返回值带上它；
2. `kernel_phase.rs`：`finish_pass<'a>(walked: Walked<'a, '_>) -> (PassResult, PassTables<'a>)`
   （`:580` 的 `PassResult{…}` 处一并交回）；
3. 调用点：`run_pass_in`（`:801`）与 `project/session.rs` 各建一份并**跨趟复用**
   （库层 `install_preludes=true`、入口 `false`）。

**判据**：去掉 `session_reuse.rs` 里 `session_compiles_entries_that_import_the_lib_layer` 的
`#[ignore]` ⇒ 必须绿；`session_reuse`（判据 ② `by_calls` **3 → 1**）绿；`clippy --all-targets`
**0 error**；**内核零改动**。随后才接 CLI ⇒ 多入口守卫（`failed:0` / `compiled:4`）⇒
真课程**同机 A/B**（本机冷全量基线 **625s**，**禁止**与 09-28 的 222.1s 跨机比）⇒ `--json` 对拍。

**四条已记入 `docs/CI-FAILURES.md` 的教训**：性能数字先看 `failed` 计数（15s 假提速）·
**revert ≠ rebuild** · 咬不住的守卫等于没有（已补多入口守卫并反向验证）· 跨机比数字无效。

## 5. **计数 ≠ 墙钟**（2026-09-29 值守纠正，必须写进预期）

切片 1 的收益要**分开报**，别让后人以为"能快 4 倍"：

* **计数**：`Σ 各入口闭包 = 174 次模块编译` → **42 次**（= 去重模块数）⇒ **4.14× → 1×** ✓；
* **墙钟**：**只省 10–25%** ✗ —— 基线 `docs/perf/rebuild-baseline-2026-09-28.md` §7 实测：
  222s 里 **148.9s 由 13 个重的解答文件主导**，而**重文件的依赖占比只有约 10%**
  （轻入口才到 50%）⇒ 省掉的是依赖段，不是主体 ✗。

⇒ **切片 1 的真实价值两块**：① 省掉依赖段（10–25%）；② **拿到模块级产物** —— 它是
**刀 2（落盘 / P3 可下载 cache）的前提**：没有 per-module 产物，跨进程复用与远程缓存都无从谈起 ✓。
**A/B 必须同机同口径**（本机基线 **625s**；**禁止**拿 09-28 的 222.1s 当分母 ✗）。
