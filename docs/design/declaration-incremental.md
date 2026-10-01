# 设计：**按声明粒度的增量检查**（编辑一次按键到底编了什么）

> 日期：2026-10-01。触发（用户原话）：「不是版本的问题。我最新版本编译完之后，**每次修改代码，
> 整个文件就会被高亮**。theorem 应该能自然分块，**不需要编译其他 theorem** 才对。」
>
> 台账：**G-29**（编辑项目文件 ⇒ 从零重编整条 import 闭包，`blocks`）· **G-31**（判定每批重跑
> 整份前缀，`blocker`）。配套：`docs/design/module-artifacts.md`（模块级产物，刀 1/刀 2）。

## 0. 结论（先给答案）

* **① 整文件高亮**：**确实还在**（不是版本问题）——`setCompileDecorations` 的范围是整份文档 +
  `backgroundColor`。P7 只改了"什么时候出现"（300ms 展示延迟），没改"画多大"。
  **已修**（S1）：只留概览尺第一行标记，**一处背景色都不留**。
* **② 一次按键 = 整条闭包从零重编**：**是常态**。项目文档（有 `import`）的缓存键含入口文本
  ⇒ 一按键必 miss ⇒ `compile_project_with_overlay` 全量重来。**闭包编译连单文件那条增量路
  （I8 TrustPlan）都没有** —— 代码里写着「增量路径保持**单文件**语义（I8）：**闭包编译不使用
  TrustPlan（v1）**」（`crates/front/src/compile/check/mod.rs:608`）。
* 而且**改动位置几乎不影响成本**（见 §1.2）⇒ 不是"后缀重查"，是**整份文件 + 判定前缀重跑**。

## 1. 诊断（2026-10-01 实测，0.80.0）

### 1.1 真 LSP 进程（unit08：4 个 import · 26KB 入口 + 32KB 库）

复现件 `docs/gaps/repro/G29-edit-recompiles-whole-closure.js`（真进程 · 真 `didChange`）：

| 量 | 实测 |
|---|---|
| 冷开（空缓存） | **3093ms** |
| 热开（缓存命中） | **11ms** |
| **改一行**（`didChange` → 该版本的诊断） | **2154ms** |

⇒ 一次按键 ≈ 冷开的 **70%**。它 > P7 的 300ms 展示延迟 ⇒ 整篇高亮每次按键都亮 2 秒。

### 1.2 成本与**改动位置**无关（同一进程 · 同一缓存）

| 改哪 | 一次按键 |
|---|---|
| 第 1 条 theorem（后缀 = 整个文件） | 3157ms |
| 第 12 条（中间） | 2002ms |
| 最后一条（后缀 = 1 条） | 2350ms |

⇒ **前缀复用没生效**。若 I8 的"前缀语义不变可复用"在这条路上生效，改最后一条应当**便宜一个
数量级**。这条读数就是"整文件重编"的直接证据。

### 1.3 结构计数（`SOKO_STAGE_STATS=1`，一次闭包编译）

| 计数 | unit08（闭包） | playground（单文件） | 含义 |
|---|---|---|---|
| `passes` | **113** | 24 | 流水线趟数（单文件那条有 I8 会话 ⇒ 4.7× 差距） |
| `JUDGE_PREFIX runs` / `bytes` | **79** / 3.13MB | 6 / 0.28MB | **`judge_infer` 的前缀重跑**（见下） |
| `by_calls` / `judge_ms` | 1195 / 226ms | 162 / 357ms | `by` 引擎（`JUDGE_STATS calls=13`，已吃担保复用） |

> **⚠ 2026-10-01 更正**：`PREFIX_RUNS` **只在 `judge_infer_uncached`（`judge.rs:1941`）里加**
> —— 它数的是**类型推断**那条 `#check` 合成路（前缀 + `#check` 从零编一遍），**不是** `by`
> 批次；每次约 39.6KB，且**不受** `check_synthesized` 那套 vouch 覆盖（那条只有 `by` 走）
> ⇒ 这是**第二个**靶子（记为 **S5**，与 S2 正交）。

### 1.4 成本落在哪：**入口自己**，不是库层

| 形状 | 墙钟 |
|---|---|
| 整闭包（今天 LSP 的路） | 3571ms |
| **只编库层 4 个模块** | ~500ms（**14%**） |
| 库层 + 入口（`with_project_session` 一趟） | 3482ms |

⇒ 用户说的"不需要编译**其他 theorem**"才是大头：**入口文件自己的重查 ≈ 3.0s（85%）**，
其中判定前缀重跑（79 趟）占主要部分。**只修"闭包复用"最多省 14%** —— 优先级要按这个来。

## 2. 分片（每片一个 commit，判据都用**结构计数**）

| 片 | 做什么 | 判据（结构计数 / 比值，不用绝对毫秒） |
|---|---|---|
| **S1** ✓ 已落 | 去掉整文件高亮：装饰只留概览尺第一行、无 `backgroundColor` | stub 宿主：范围恰好 1 个、`start.line == end.line == 0`、**无** `backgroundColor`（反向验证：改回整篇必红） |
| **S2** | **闭包编译的入口接上 I8 TrustPlan**（把 `check/mod.rs:608` 的"v1 不做"推进到 v2）：`before` = 合并命令流里**首个改动命令**的下标（库层命令全部在它之前 ⇒ 天然被信任） | `JUDGE_PREFIX runs`：改**最后一条**声明时应当**显著小于**改**第一条**（今天两者相同 79）⇒ 判据是**两个位置的 runs 之比**，机器无关 |
| **S3** | **库层会话跨按键复用**（`module-artifacts.md` 刀 1 的进程内版）：保住库层检查点，按键只跑入口 | unit 编译计数：连敲 3 次从 `3×N` 降到 `N_lib + 3×N_entry`；unit08 预期 104 → ~30 |
| **S4** | **判定前缀复用**（G-31 本体）：同一份前缀在多个 `by` 批次间只跑一次 | `JUDGE_PREFIX runs` 与 `bytes`：unit08 的 79 趟 / 3.1MB 应降到"每个**不同**前缀一趟" |

**顺序的理由**：S2 不动环境、只动信任边界（改动最小、收益直接打在 85% 那一段）；S3 需要跨调用
存活的会话（`DeclarMap` 是 kernel `pub(crate)` ⇒ 不能进具名字段，得走
`with_project_session` 那种"循环在 front 里"的形状）；S4 是判定内部的深水区，放最后。

## 3. 红线与纪律

* **判定正确性不变**（硬规则 1）：TrustPlan 只声明"这段前缀的**文本**没变过、上一轮查过" ——
  文本没变 ⇒ 前缀语义不变，是既有 I8 不变式（单文件路径已用了很久）；`--json` 必须逐字节不变。
* **判据不许用绝对毫秒**（AGENTS.md）：`JUDGE_PREFIX runs`/`bytes`、unit 编译计数、passes
  都是结构量；墙钟只做数量级兜底。
* **S1 是唯一的用户可见改动**，其余三片对用户只表现为"变快" —— 但"变快"要用 §2 的计数证明，
  不靠体感。

## 4. S2 落点（**源码级**，2026-10-01 读码核实；开工前先读本节）

§2 把 S2 写成一句话（"入口接上 TrustPlan"）。逐文件读下来，**地基比预期好得多** ——
三处关键事实（都是源码事实，不是推断）：

1. **`with_project_session` 已经做了"库层一次 + 各入口各自"**（`crates/front/src/project/session.rs:21`）：
   库层编一趟 → `hide_declars()` 拿检查点 → 每个入口 `restore_declars` 后**只跑自己的命令**。
   而它调 `run_pass_with` 时第 9 个参数（`trust: Option<&TrustPlan>`）**恒传 `None`**（`:98`）
   ⇒ **入口那趟接 TrustPlan 的位置已经现成**。
2. **`run_pass_with` 已经收 `trust` + `skip` + `closure_prefixes_override` + `display_override`**
   （`crates/front/src/compile/check/mod.rs:963`）。最后两个是切片 1 路乙加的，**正是入口趟
   让 `judge_infer` 看见库层所必需的**（session.rs:84-102 已经这么传了）⇒ 不用重新发明。
3. **`run_incremental` 已经做完了"信任前缀 + early cutoff"的全部工作**
   （`check/mod.rs:596`），但它**写死了一个单元**：
   `let units = [SourceUnit::single("", file)];`（`:608`，注释写着"闭包编译不使用 TrustPlan（v1）"）
   ⇒ **S2 的第一刀就是把这一行参数化**，其余逻辑一行不用改。

### 4.1 三步（每步可独立验，别合并）

* **步 1（纯重构，零行为变化）**：`run_incremental` 收 `units: &[SourceUnit<'_>]`
  （现调用方 `session.rs:276` 传 `&[SourceUnit::single("", file)]`）。
  判据：既有 front 单测 + `--json` 逐字节不变（**这一步不该有任何计数变化**）。
* **步 2**：`with_project_session` 收 `entry_trust: Option<(&TrustPlan, &KernelFailed)>`
  并在入口趟传给 `run_pass_with`（`:98` 那个 `None`）。
  判据：`crates/front/tests/session_reuse.rs` 既有 2 例 + 新增一例"给 trust ⇒ `passes` 下降、
  报告逐字节相同"。
* **步 3**：`QueryDoc`（`crates/front/src/query/mod.rs:128 set_text_with_overlay`）**持有入口的
  逐命令快照**（今天只有单文件分支用 `self.session`；项目分支每次都 `project_compile` 全量）。
  这是**唯一有状态的一步**，也是唯一有风险的：快照要能跨 `set_text` 存活。

### 4.2 三个**必须做对**的下标（错了会静默错编，不是崩）

| 下标空间 | 关系 | 谁负责 |
|---|---|---|
| 入口**自己的**命令（`Session.keys`/`snaps` 的坐标系） | `before_entry` = 首个改动命令 | `first_diff`（`session.rs:219` 同款） |
| **合并**命令流（`run_pass` 看到的） | `before_merged = lib_n + before_entry`，`lib_n = lib_pass.n_commands` | 步 2 的调用方 |
| `prefix_failures` / 返回的 `sigs`、`cutoff` | 与 `before` **同一个坐标系**（合并流）⇒ 入口那份要**整体加/减 `lib_n`** | 步 3 |

⚠ **库层命令天然全在信任前缀里**（它们一个字节都没变）—— 这正是设计说的"库层命令全部在它之前"，
但它**只在下标算对时成立**；算错 = 把库层声明当成"要重查"或把入口声明当成"已查过" ⇒ **错编**。
所以步 3 的判据必须是**内容级**的：`--json` 逐字节 + 课程门禁 43/377/99/0，**不是**只看变快。

### 4.3 判据（结构计数，机器无关）

```bash
# ① 复现件（今天：冷开 3104ms / 热开 14ms / 改一行 2189ms）
node docs/gaps/repro/G29-edit-recompiles-whole-closure.js
# ② 结构计数（同一入口，改**第一条** vs 改**最后一条**）
SOKO_STAGE_STATS=1 <入口> … 2>&1 | grep -E "JUDGE_PREFIX|PASSES"
#    S2 的判据 = 两者的 JUDGE_PREFIX runs 之比（今天两者相同 79 ⇒ 比 ≈ 1.0）
# ③ 零回归：--json 逐字节不变 · 课程门禁 43/377/99/0 · cargo test -p sokonanoda-cli
```

### 4.4 as-built：**步 1 / 步 2 已落**（2026-10-01）

* **步 1 ✓**（纯重构）：`run_incremental` 的第一个参数由 `&FolFile` 改成
  `&[SourceUnit<'_>]`（`check/mod.rs:606`），内部那句 `SourceUnit::single("", file)` 删掉。
  两个调用方各自传单元素切片：`session.rs:276`、`judge.rs:780`。**零行为变化** ——
  判据：front **790 + 全部集成套件** 0 failed · CLI 全套 0 failed · 课程门禁 **43/377/99/0**。
* **步 2 ✓**：`with_project_session_trusted(..., entry_trust: &[Option<EntryTrust>], ...)`
  （`project/session.rs`）；`with_project_session` 变成它的薄包装（传 `&[]`）⇒ **既有调用方
  逐字节不变**。入口趟把 `trusted.map(|t| &t.failures)` / `&t.plan` 喂给 `run_pass_with`
  （原来那两个位置恒为 `None`）。
* **顺带修一个漏**：`with_project_session` 那条路**从来没把 `pass.checks` 搬进
  `out.stats.kernel_checks`**（`run` 会搬，它漏了）⇒ 合并输出的 `kernel_checks` 恒为 **0**。
  现在两趟都搬（新增 `PassResult::kernel_checks()` 访问器）—— 这正是判据要读的那个数。
* **判据**（`project/tests.rs::entry_trust_skips_the_prefix_and_keeps_the_suffix_identical`，
  两条一起断言，缺一条就是把"丢声明"当成功 ✗）：
  `PERF entry_trust kernel_checks full=8 trusted=5 entry_decls full=6 trusted=3`
  —— 4/7 条命令被信任 ⇒ 内核检查 **8 → 5**；且被信任那段**不进报告**（6 → 3 条声明），
  **后缀 t3/t4/t5 的判定与整份重查逐字节相同**。
* **还没接给用户**（= 步 3）：`QueryDoc` 仍走 `project_compile` 全量。**下一轮从步 3 开始**，
  §4.2 的三个下标是唯一的风险面。

### 4.5 更早一轮**没有**做 S2 的理由（诚实记账）

本轮交付的是**用户可见的那一条**（S1 去整文件高亮 + P7 展示延迟，见
`docs/design/edit-latency.md`）。S2 的步 3 要动 `QueryDoc` 的**有状态快照**，而 §4.2 的下标
一旦算错就是**静默错编**（红线）—— 本轮的余量不足以把"实现 + 内容级零回归验证"一起做完，
**宁可交一份可核实的落点，也不交半成品**。下一轮**从步 1 开始**（纯重构，可独立验），
步 1/步 2 落地后再碰步 3。
