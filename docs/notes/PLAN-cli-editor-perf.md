# 规划：CLI / 编辑器体验 + front 编译链路性能（下一轮执行纲领）

> **性质**：只读调研 + 规划（**零代码改动**）。本文是 DSH 下一轮的**执行纲领**：
> 现象 → 根因（文件:行）→ 任务 → **可执行判据** → 依赖与并行。
>
> **构建身份**（探针纪律：跨轮比较读数前先确认同一份构建）：本轮开工时 `HEAD 768c5c72`；
> 发版会话在 **16:34 又提交了 `2c39f57a`**（`chore(release): v0.85.2`）——它没触及本轮的
> 分析路径（`state.rs` 末次改动 2026-10-02，其余 ≤10-08 11:36）。
> **本文件里的 CLI 探针**用 `target/release/sokonanoda`（自述 `0.85.2`，16:00）；
> ⚠ **MCP 工具链的 marker 是 `0.83.0`**（`scripts/soko` 解析到缓存/旧构建）⇒
> **MCP 读数与 CLI 读数不是同一份二进制**，两者只作"机制一致"的交叉印证，不并排比数字。
> 工作树有 **9 个**发版会话的未提交改动（`Cargo.toml`/`Cargo.lock`/`STATUS.md`/
> `editor/vscode/{package.json,CHANGELOG.md}`/两个 `sokonanoda.toml`/
> `crates/cli/tests/notation.rs`）——**它们不属于本轮，也不许动**。
>
> **与发版线的关系**：课程线 **v0.85.2** 正在由另一会话发布。本文只写规划；
> 下一轮开工前先确认发版闭环（`gh release list` + 版本文件干净），
> **不许 bump 版本、不许 commit/push、不许碰 `STATUS.md`/台账/版本文件/Cargo 文件**。
>
> **上游调研**：`docs/notes/perf-lean4-interactive-incremental.md`（2026-10-07，Lean 4
> 四层缓存 + 本仓库差距）。⚠ **它有两处已被 2026-10-08 的提交推翻**，见 §0.2 ①/② ——
> 下一轮**以本文为准**，perf-lean4 只当 Lean 侧机制引用。

---

## 0. 一页结论

### 0.1 七个现象 → 一句话根因 → 归属层

| # | 现象（用户原话要点） | 一句话根因 | 归属层 | 优先级 |
|---|---|---|---|---|
| P1 | build/rebuild 汇报太稀疏，**约 24 个文件才一条** | 人看的那条进度是**每 10% 一行**（`step = total/10`）⇒ 240 文件 = 每 24 个一行；且**模块级 tick 只喂 `--json`**，终端连心跳都默认关 | `crates/cli/src/build.rs` | **C1（快赢）** |
| P2 | build 必须跟路径，期望**默认当前路径** | **已落地**（`build` 无参 = cwd）✓；**残留**：`clean`/`rebuild` 无参**只清全局**，模块根产物原地不动 ⇒ rebuild 全是 `hit`（假重编） | `crates/cli/src/build.rs` | **C2（快赢）** |
| P3 | 最后一条 tactic 报错 ⇒ **前面所有 goal 全坏**；感觉整段证明全量重编 | ① **显示**：`run_by` 出错时把已算出的 `steps` 整份丢掉（`?`）⇒ `DeclState.by_steps` 为空 ⇒ 整份声明退回题面（`step:-1,total:0`）；② **重算**：`by` 块是一条命令，块内任何编辑都从第 0 条 tactic 重跑 | `front/by.rs`·`walk.rs`·`query/state.rs` | **B1** |
| P4 | 光标在某 tactic 上，应显示**该 tactic 作用后**的 goal；末尾正确应显 🎉/Q.E.D | 选择器写的是"**进入**该 tactic"（`start <= cursor < end ⇒ step i-1`）——**与 Lean 4 相反** | `front/query/state.rs` | **B2** |
| P5 | `#check`/`#print` 在 infoview **没有内容** | `#check` 只走 **inlay hint**（不是 infoview）；`#print` 的 `Printed` 事件**根本没进 `DocumentReport`** ⇒ LSP 结构上看不见它 | `front/compile/report.rs`·`lsp`·`infoview.js` | **C3** |
| P6 | 打开/写/改/提交每一步丝滑（后台异步、差量重编、不闪烁） | 闪烁**已修**（P7 展示延迟，有守卫）✓；剩下的"数秒"在**冷开**（缓存未命中）与**每次 bump 全库作废** | LSP + 缓存链路 | **C4 / A3** |
| P7 | 编译链路本身要快：telescope/签名级缓存、库层/模块级增量、模块产物 | **库层检查点已落地**（G-29，2026-10-08）✓；剩下的大头 = **入口趟**：一次按键 `prefix=5` 的 judge 合成前缀重编（≈一半成本，**全是 G-71 闸挡住的 `needs_explicit` 探针**）+ 入口自身 elaborate；`module_keys()` 仍零生产调用 | `front/judge`·`by.rs`·`project` | **A2a**（纯接线）→ A4a/A2b/A3 |
| P8 | **prelude 要独立成显式源文件**，不要嵌在 Rust 里；学生/工具能**看到与修改**（教学工具初心） | **半成品**：仓库已有 `prelude/Prelude.sokonanoda`，但它是**常量 ⇒ 文件**的"镜子"（生成物）；**真相仍在** `crates/front/src/compile/prelude.rs` 的 3 个字符串常量（≈109 行）⇒ 改文件**行为不变**；Nat/Bool 9 个名字连源文本都没有（手搓 AST）；F12 落到**缓存副本**而非仓库文件 | `front/compile/prelude.rs`·`prelude/` | **E1**（方向翻转）→ E2/E3/E4 |

### 0.2 已经落地的（**别重做** ✗）—— 本轮实测/读码核对

1. **G-29 库层检查点跨调用复用**：**2026-10-08 已落地**（`fa14b97c`，设计 §33.1）。
   `crates/front/src/project/session.rs` 的 `with_project_session_reusing` +
   线程局部 `LIB_CHECKPOINT` + `Box::leak` arena + 上界（8 份 arena / 64 次复用）+
   回退路；判据 `crates/front/tests/g29_closure_recompile.rs` 今天断言
   **`edit == 1`（冷开 2）**，另有 `g29_library_checkpoint_is_bounded`。
   ⇒ perf-lean4 §4.0/§4.1「把检查点交给 `QueryDoc`」**已被推翻**（`ArenaRef` 是 `!Send`，
   持有者改成线程局部；LSP 编译因此钉在**单 worker** runtime 上）。
2. **G-31/G-92 合成前缀的不透明常量**（`0.83.0` 已落地）：`by_calls` 线性化
   （3.82 ✗ → **2.00** ✓），全课程 `--json` 逐字节不变、影子档 `diff=0`
   （`incremental-environment.md` §32.4(iii)）。⚠ **降级交付**：前缀的**类型/定义**仍重编。
3. **G-68 分组会话 + 重复功 ratchet**：`compile_entries_shared` +
   `scripts/check-recompile-factor.py`（测**次数**，只许减不许增）✓。
4. **P1 的一半**：`build.progress`（逐文件，**仅 `--json`**）+ `build.decl`（声明级，
   编译**结束后**按 `files` 顺序重放）+ `build.tick`（心跳，**默认只给管道**）+
   失败明细；扩展三处渲染齐（状态栏 / Infoview / 概览尺）。
5. **P2 的主体**：`build` 无参 = **当前目录** + 跳过 `.git`/`node_modules`/`target`/…
   （`b99801fb`，守卫 `crates/cli/tests/cli_surface.rs:361`）；`clean`/`rebuild` 子命令已存在。
6. **P6 的闪烁**：已修（`docs/design/edit-latency.md` §2：`showDelayMs=300`，
   `flicker_frames 16 → 0`，三条守卫）；**warm 按键 0.1–0.7ms**，
   **不是**瓶颈（§1.1 的三把尺子）。
7. **P3/P4 的地基**：逐 tactic 目标快照 `by_steps`（`ByStepState{span,goals}`，
   **每步 = 该步执行后**的剩余目标）**已经在算**，`soko/stateAt`/hover/CLI/MCP 都消费它。

### 0.3 任务总表（依赖 → 见 §3）

| 阶段 | ID | 任务 | 判据形态 | 依赖 |
|---|---|---|---|---|
| A | **A2a** | **就地路接管 `needs_explicit` 的 goal**（抬 G-71 闸 + explicit 档 + shadow） | `prefix=` 5→0 · shadow `diff=0` | 无（**纯接线 · 最高 ROI**） |
| A | A2b | judge 合成编译**复用调用方活环境**（阶段 2 (i)/(ii)） | `prefix`/`passes` → 1 | A2a 的读数 + 内核线 |
| A | A3 | **模块级产物 / `module_keys()` 接线**（新机制） | 跨进程重编模块数 | **决策门已过（§8.11：值得做，等当前批次）** |
| A | A4a | **库层三张 O(闭包) 派生表进检查点**（纯接线） | 表重建数 = 0（第 2 刀起） | 无 |
| A | A4b | **telescope / 签名级缓存**（先建计数器） | telescope 解析次数 | 与 A4a 同片地 ⇒ 串行 |
| A | **A6** | **`whnf_admit` 4 MiB → 64 KiB + 索引移位由位数推出**（**已落地** · §8.8） | `tc=` 不变 · 按键墙钟 −40%（同二进制自比）· 226 文件 `--json` 逐字节 | 无（**纯内核 · 最高单项 ROI**） |
| A | **A6b** | `TcCache::new` 里 ~20 张表的**预分配 → 惰性**（**已落地** · §8.9） | `TcCache::new` 样本占比 22.9% → **4.0%** · `--json` 逐字节 | 无（**纯内核**） |
| A | **A7** | **judge 缓存键的 O(前缀) SipHash**（§8.9 实测 **22.5%** 编译样本） | 新增"前缀哈希字节数"读数 · `--json` 逐字节 | 决策门（不换哈希函数优先） |
| B | B1 | **失败证明保留前缀目标**（P3 显示面） | `by_steps.len()` / `total` | 无（可先做） |
| B | B2 | **光标语义改为"作用后"** + 末尾 🎉/Q.E.D（P4） | `state.step` 值 | 无（可先做） |
| B | B3 | 失败块严格趟**从失败步续跑**（P3 成本面） | 走查数（pass_count） | B1 |
| B | B4 | 逐 tactic 前缀快照缓存（**决策门 · 最后做**） | tactics executed/total | B3 的读数 |
| C | C1 | **build 进度逐文件 / 短间隔**（P1） | stderr 进度**行数** | 无（快赢） |
| C | C2 | **`clean`/`rebuild` 无参也清模块根**（P2 残留） | `build.summary.hit` | 无（快赢） |
| C | **C3** | **`#check`/`#print` 进 infoview**（P5，三层）—— **已落地**（`91c72399`+`95c78795`） | `stateAt.messages` 三层各一条 + 反向验证 | ✓（协议 §`soko/stateAt` 已写） |
| C | C4 | 冷开 / 提交链路的**结构判据**（P6） | `hit`/模块编译数 | A3 |
| D | D1 | Lean 4 infoview / `#check` 机制对照 | 源码行号（**本轮已完成**） | — |
| D | D2 | build CLI 惯例（cargo/rustc/tsc/lean） | 结论件 | 与 C1 并行 |
| D | D3 | 端到端做题基准（一次按键 → 目标更新） | 结构计数为主 | 与 A/B/C 并行 |
| E | E1 | **prelude 方向翻转**：`prelude/*.sokonanoda` 成真相，Rust 只留 `include_str!` | Rust 里源文本行数 = 0 | 无（独立） |
| E | **E2** | **Nat/Bool 源化 或 如实登记边界** —— **已落地：如实登记**（spike 结论见 `prelude/L1.sokonanoda` 的 `builtin-rust` 区） | 登记行逐字 + `prelude_def_span` 9 名**全 None**（反向验证 ✓） | ✓ E1 |
| E | E3 | **运行时覆盖** `--prelude`/`SOKO_PRELUDE_PATH`（决策门） | 缓存键折内容哈希 + 诊断非 panic | E1 |
| E | E4 | **撤影子档的 prelude 排除**（`course-stdlib.md` §7 的目标） | 排除分支计数 = 0 · shadow `diff=0` | E1 |

**建议顺序（按用户给的优先级：缓存 → 前缀/目标 → UX → 调研；**P8 独立**）**：
⚠ **2026-10-08 开工 profiling 后的修订**（见 **§8**）：新增 **A5（产物命中后台预热库层检查点）**
排在**批次 1 之首** —— 实测它是**最大的单笔**（首次按键 **1233ms → ~320ms**，结构计数
`modules=` 5→1），且是**纯调度**改动（不碰判定）。**A2a 的天花板同时被下修**
（5 次前缀 ≈ 按键成本的 **~15%**，不是"一半" ⇒ 仍做，但不再是最高 ROI）。
**批次 1（文件不冲突）** = **A5**（`project/*`+`lsp`）→ **A2a**（`by.rs`/`judge.rs`，纯接线）
+ **C1/C2**（`crates/cli/src/build.rs`，快赢）+ **E1**（`prelude.rs`+`prelude/*`，独立）
→ **批次 2** = B1（`by.rs`/`walk.rs`/`check/mod.rs`）
→ **批次 3** = B2（`state.rs`/`lsp`/`infoview`/协议）→ **批次 4** = A4a + B3 → **批次 5** = C3（协议面）
→ **之后** = A2b/A3/A4b/B4/C4/D*（**按读数决策**；每一条都要先过它自己的决策门）。
**D2/D3 可全程并行**（只读）。

---

## 1. 逐条归因（P1–P7）

### P1 · build/rebuild 汇报太稀疏（约 24 个文件才一条）

**复现**（本轮实测 · release `0.85.2`）：

```bash
mkdir -p /tmp/p1 && cd /tmp/p1 && for i in $(seq 1 100); do printf 'theorem t%s : Prop := Prop\n' "$i" > "f$i.sokonanoda"; done
sokonanoda build 2>&1 | grep -c "file(s)"     # ⇒ 12 = 10 条进度 + 「found 100」+「built 100」
```

**根因（读码 + 实测，三条叠加）**：

1. **人看的那条是"每 10%"**：`ProgressCounter::new` 里 `step = (total / 10).max(1)`
   （`crates/cli/src/build.rs:278`），`note_file` 只在 `done % step == 0` 时打印
   （`:295-297`）。⇒ **N 个文件 ⇒ 每 N/10 个一行**；真课程 ~240 文件 ⇒ **每 24 个一行**
   —— 用户报的"约 24 个"就是这个数（13 文件时 `step=1`，所以小目录看起来正常）。
2. **模块级 tick 只喂 `--json`**：`ProgressTick{module,index,total}` 的 sink 在
   **非 json 时是 `None`**（`build.rs:606-607`（预跑）、`:660-661`（并行）、
   `:691-692`（串行））⇒ 一个入口要编 40 个闭包模块时，终端**整个入口期间零输出**。
3. **终端没有心跳兜底**：`tick_period_ms()` 默认"**是管道才发**"（`:225-228`），
   人在终端 = **不发**（2026-09-29 的 JSON 刷屏投诉留下的口径）⇒ 单条最贵 ~14s
   的声明（`build.rs:301-304`）、冷编课程首条 `build.decl` **160.8s**（`:246`）
   这些区间里，可见面**完全静止**。

**已落地的一半**：`build.progress` 逐文件事件（`--json`）、`build.decl` 声明级重放、
心跳（管道）、失败明细（文件 + 原因）、扫描起止行 —— 都在，**扩展侧三处渲染齐**
（`editor/vscode/extension.js:2321-2336`/`:2359-2370`/`:2371-2384`）。
**缺口正身 = 人看的终端**（`--json` 契约不动）。

**判据纪律**：`--json` 的**确定性红线**（并行只并行编译，输出仍按 `files` 顺序重放 ⇒
与串行逐字节相同）**不许破**（`build.rs:546-548`；测试 `crates/cli/tests/cli.rs:2230+`
钉住 `build.progress` 全排在首条 `build.decl` 之前）。⇒ 新增的人看输出**只能走 stderr**。

### P2 · build 默认当前路径（已落地 + 一处残留）

**已落地**：`warm()` 无参 ⇒ `vec![PathBuf::from(".")]`（`build.rs:474-478`），
扫描期打 `scanning the current directory …` / `found N …`（`:483-489`），
帮助文本写清（`help.rs:18-20`），守卫 `crates/cli/tests/cli_surface.rs:361`。

**残留（本轮实测，真 bug）**：`clean`/`rebuild` 无参时**只清全局缓存**，
模块根 `.sokonanoda/compiled/` **原地不动** ⇒ 紧接着的 `build` 全是 `hit`。

```bash
cd /tmp/p2b   # 一个 lib/Shared + Main（import）的小项目
sokonanoda build .        # ⇒ built 2 file(s) — 0 hit, 2 compiled, 0 failed
sokonanoda rebuild        # ⇒ built 2 file(s) — 1 hit, 1 compiled, 0 failed   ✗ 假重编
```

**根因**：`clean_stores(args)` 的模块根**只从参数解**（`project_roots(args)`，
`build.rs:822-841`；`args` 空 ⇒ 空集），而 `warm` 的默认是 cwd —— **两条路的默认不一致**。
现在只有一条 `note:` 提示（`:441-445`），语义上没修（设计 §3.7「解不出就不清、也不假装」
在**有参数**时是对的；无参数时**能**解出来，只是没去解）。

**另需对齐的一处（不是 bug，是文档）**：扩展的 Build/Rebuild **目标是模块根**
（E22，`extension.js:2397`/`:2399` 恒传 `target`）——与 CLI 的"当前目录"**不是同一件事**；
下一轮把这条写进 `docs/vscode-dev-guide.md`/`docs/visible-changes.md`，避免又被当成 bug 报。

### P3 · 最后一条 tactic 报错 ⇒ 前面所有 goal 全坏

**复现（本轮实测 · CLI，两种文件只差最后一条 tactic）**：

```bash
cd /tmp/p3
# ok.sokonanoda  : apply And.intro / exact hp / exact hq
# bad.sokonanoda : apply And.intro / exact hp / exact hp      ← 最后一条错
sokonanoda query state --file ok.sokonanoda  --line 3 --col 4   # ⇒ step 0, total 3, goals [P, Q]
sokonanoda query state --file ok.sokonanoda  --line 4 --col 4   # ⇒ step 1, total 3, goals [Q]
sokonanoda query state --file bad.sokonanoda --line 2 --col 4   # ⇒ step -1, total 0, goals [P ∧ Q]
sokonanoda query state --file bad.sokonanoda --line 3 --col 4   # ⇒ step -1, total 0, goals [P ∧ Q]  ✗
sokonanoda query state --file bad.sokonanoda --line 4 --col 4   # ⇒ step -1, total 0, goals [P ∧ Q]  ✗
```

**根因 ①（显示面 · 必做）**：`run_by_inner` 末尾 `run_tactics(…)?`
（`crates/front/src/by.rs:1302`）—— 第一条失败的 tactic 让 `Err` 冒泡，
**已累积的 `steps` 整份丢掉**（`steps` 建在 `:1279`；`steps.push` 在 `:1775`，
只在成功步之后）。调用侧三个失败分支（`walk.rs:819` 的 `def`、`:1190-1199` 的
`theorem`、`:1622` 的 `example`）只造 `failed_state(...)` + `st.by_root`，
而 `failed_state` 里 **`by_steps: Vec::new()`**
（`crates/front/src/compile/check/mod.rs:1773`）；内核终审拒绝那条路同样
（`crates/front/src/compile/check/kernel_phase.rs:182`）—— 即**即使 `by` 块跑通、
只是终审不过，per-tactic 状态也丢**。查询层于是走"失败的声明 ⇒ 退回题面"那条路
（`crates/front/src/query/state.rs:52-80`）⇒ **整份声明只剩一个根目标**
（`step:-1`、`total:0`）——这就是"前面所有 goal 全坏"。

**⚠ 被排除的假设（实测，别再猜）**：错误 span **不是**整段 —— 实测
`elab-tactic-failed` 只覆盖失败那条 tactic（offsets 200-208 = 第 3 行 `exact hp`）；
judge 也**不是**"整份失败"（它逐 pair 返回 `Judgement`）。

**根因 ②（成本面 · 决策门）**：`by` 块是**一条命令**（`Expr::By{tactics}`，
`ast.rs:77`/`parser.rs:1253`），块内任何编辑都让 `run_tactics` 从第 0 条重跑
（无 per-tactic 检查点）。**额外的一笔**：任一判定非 `Match` ⇒ `flush_batch`
返回 `false`（`judge.rs:2058`/`:2095-2097`）⇒ `run_by` **丢掉乐观趟、从头严格重跑整段**
（`by.rs:1198-1217`）。实测（`SOKO_JUDGE_STATS=1`，3 条 tactic 的夹具）：
全过 **calls=1 pairs=2** · 第 1 条错 **calls=2** · 第 3 条错 **calls=3 pairs=4**
⇒ 严格重跑**从头重放前面每一条**，成本随出错位置增长。
另外 judge 的合成前缀编译（`judge_infer_uncached → compile_fol_with`）已由 G-31
砍掉"前缀里 `by` 的证明体"，但**前缀的类型/定义**仍重编（as-built 读数：
改一行 `prefix=5` 次合成编译 ≈ 0.7–1.1s，`incremental-environment.md:1146-1148`）。

**今天的守卫（P3 无守卫 ✗）**：现有三条只钉**成功/开放**块 ——
`compile::tests::partial_by_block_records_per_step_states`（Open ⇒ `by_steps.len()==2`）、
`checked_by_block_records_closed_final_step`（Checked ⇒ 3）、
`wrong_exact_reports_tactic_error`（只断错误码）；LSP 侧
`lsp/src/tests/state.rs:258 state_at_without_by_steps_returns_the_declaration_goal`
钉的是"**无 `by_steps` 就退声明目标**"——**正是 P3 那条退路**（它必须保持绿：
无 `by` 的声明行为不变）。⇒ **失败 `by` 块的 goal 状态两个方向都没有测试**。

**Lean 4 对照**（`~/Documents/lean/lean4` @ `d0493e4c1e`）：
- 每条命令的**完整 `Command.State`（含 env）**都是快照、可复用；第一次**语法**不等处
  才取消整条尾巴（`src/Lean/Language/Lean.lean:558-611`）。
- 一条 tactic 内部的子部分还能用 `withNarrowedTacticReuse` 抢救。
- `TacticInfo` 同时带 `goalsBefore`/`goalsAfter` ⇒ **一次失败不会抹掉之前的状态**。

### P4 · infoview 目标语义：应显示"该 tactic 作用后"

**今天的语义（读码 + 实测）**：`select_state_at`
（`crates/front/src/query/state.rs:124-136`）：

```text
光标落在 tactic i 的 span 内（start <= cursor < end） ⇒ step = i-1（进入它之前）
否则 ⇒ 最后一条在光标前结束的 tactic 之后
```

文档与测试都按这个写：`crates/front/src/query/mod.rs:1240-1250`、
`crates/lsp/src/lib.rs:1115-1119`、`docs/protocol.md:453-455`、
`crates/front/src/query/tests.rs:252-274`（`state_at_inside_a_tactic_shows_the_entering_state`）、
`crates/cli/tests/query.rs:133`。
**实测（同一条 tactic 只差一列，语义翻转）**：`exact …` 的 span `[62,103]` ——
第 2 行 col43（tactic 最后一列）⇒ `step:-1, goals:["b ∧ a"]`（**前**）；
col44（行尾 = `end`）⇒ `step:0, goals:[]`（**后**）。

**Lean 4 的真实语义（源码级 · 这就是 P4 的答案）**：
`src/Lean/Server/InfoUtils.lean:448-481` 的 `InfoTree.goalsAt?`：

```lean
useAfter := hoverPos > pos && !cs.any (hasNestedTactic pos tailPos)
```

即**光标只要严格在 tactic 起点之后**（且其前没有嵌套 tactic）就取 **`goalsAfter`**；
`TacticInfo` 同时存 `mctxBefore/goalsBefore/mctxAfter/goalsAfter`
（`src/Lean/Elab/InfoTree/Types.lean:150-155`），
`src/Lean/Server/FileWorker/RequestHandling.lean:195-198` 据此二选一。
**权威行为样本**（Lean 自带测试）`tests/lean/interactive/plainGoal.lean.expected.out`：
`  intro a` 的 char2（起点）⇒ `⊢ α → α`（前）· char3 ⇒ `a : α ⊢ α`（后）·
`simpa …` 的 char43（行尾）⇒ `{"rendered":"no goals","goals":[]}`。
⇒ **Lean 在 tactic 内部显示的是"作用后"**，我们显示"作用前"——**与 Lean 相反**。

**改法（方案 A · 唯一实现处一行规则）**：`state.rs:127-129` 的
`start <= cursor < end ⇒ i-1` 改成 **`start < cursor < end ⇒ i`**
（`cursor == start` 仍是"进入态"）。**hover 不受影响**：`crates/lsp/src/lib.rs:1208`
故意把光标钉在 `step.span.start.offset`（hover 与 Infoview 的既有分工，
`lib.rs:1185-1190`）。**不要**在 LSP 层改写光标再问（违反 `query_map.rs:1-9`
"适配器只换算坐标"；CLI/MCP 会与 LSP 分叉，`crates/cli/tests/query.rs:970-1005`
的 CLI≡LSP 契约会抓）。

**两条安全性（实测，防"照抄出 bug"）**：
① **我们的 `sorry` 不闭合目标**（两条 `apply And.intro` + `sorry` 的开放练习，
末条 `sorry` 之后 `step:2, goals:[b,a]` 仍非空）⇒ 照抄 `useAfter` **不会**把开放练习
谎报成"已证完"（G-78 那条"谎报已无目标"不会回归）；
② **给不出中间态**（诚实登记）：Lean 靠**嵌套** `TacticInfo` 能显示 `rw [h1,h2]`
中间的局部状态；我们的 `by_steps` 是**扁平**顶层列表
（`crates/front/src/compile/report.rs:160-163`）⇒ 只能给"整条 tactic 之后"。

**🎉/Q.E.D. 不需要新 wire 字段**：`soko/stateAt` 已有 `goals`（`[]`=闭合）、
`goal`（`null`）、`step`、`total` 与 `decl.status`
（`crates/lsp/src/protocol.rs:183-203`）。“证明在此闭合”的判据 =
**`total > 0 && goals.length === 0 && step === total - 1`**（`checked` 的 `def`/`axiom`
是 `total:0`，`crates/front/src/query/tests.rs:239-252` 钉着；开放练习末条 `sorry`
之后 goals 非空）。渲染分支**已存在**：`media/infoview.js:392-398` 今天一律画
「已无目标 ✓」⇒ 只需按 status/闭合判据分成「🎉 恭喜，证完了（Q.E.D.）」；
树侧同理（`extension.js:572-578`）。**建议先不加新字段**（加 = 动 wire 形状，
要跑 `audit-wire-fields.py` + 三层判据，收益仅可读性）。

### P5 · `#check` / `#print` 在 infoview 没有内容

**三层逐层定位（本轮实测 + 读码，精确到行）**：

| 层 | `#check` | `#print` |
|---|---|---|
| 真相层 | ✓ 算了：`kernel_phase.rs:448-486`（`PendingOp::Check` → `CheckEvent::TypeChecked{text,span}`）→ `session.rs:587-596` 收进 `DocumentReport.checks`（`report.rs:222-230`） | **✗ 没进报告**：`kernel_phase.rs:519-534`（`PendingOp::Print` → `CheckEvent::Printed{name,text}`，**无 span**）→ `session.rs:587-596` **只匹配 `TypeChecked`** ⇒ `Printed` 被丢；`DocumentReport` 没有这个字段（`report.rs:232-243`：只有 `decls/hovers/hover_cmds/errors/checks/warnings`） |
| CLI `--json` | ✓ `expr.typed{inferred_type,text}` | ✓ `decl.printed{name,text}` |
| LSP wire | 只走标准 `textDocument/inlayHint`（`inlay.rs:13-32`，`lib.rs:2536-2547`）——**内联灰字，不是 Infoview** | **零出口**（`grep Printed crates/lsp/` = 0） |
| 扩展渲染 | 无 infoview 面 | 无（Infoview 只认 `state/progress/decls/project/status/server/theme`，`media/infoview.js:586-608`） |

⇒ **断点**：`#check` = **wire 层**（真相有、只进 inlayHint、Infoview 无字段）；
`#print` = **真相层 + wire + 渲染三层全断**。
**补齐成本低**：`Printed` 事件本身就在命令快照里（`session.rs:86` 的 `events`，随缓存复用）。

**Lean 4 对照（机制级）**：`#check`/`#print` 都是**命令 elaborator 里的消息**——
`src/Lean/Elab/BuiltinCommand.lean:421-441`（`logInfoAt tk m!"{e} : {type}"`）·
`src/Lean/Elab/Print.lean:224-226`（`printIdCore` → `logInfo`）；消息进 snapshot 的
message log（`src/Lean/Server/FileWorker.lean:310-325` 转 `InteractiveDiagnostic`），
Infoview 用 RPC `Lean.Widget.getInteractiveDiagnostics`，参数
**`GetInteractiveDiagnosticsParams{lineRange?}`**（`WidgetRequests.lean:113-117`），
服务端按 `fullRange` 与行范围相交过滤（`FileWorker.lean:754-775`）。
⇒ **决定**：给 Infoview 加"消息"面最贴 Lean，最小形态 = **按光标行取**
（`lineRange?` 的退化版）；`#print` 先要进报告，**得先定 range 挂命令 span 还是名字 span**
（Lean 用 `withRef tk`，`Print.lean:224-226` —— 我们 `Printed` 今天**没有 span**）。
另外 Lean 的 `sorry` 是 term-tactic、会 `admitGoal` 掉目标（`Elab/Tactic/Basic.lean:17-21`），
**我们不会**（实测）—— 这是两边 infoview 在 `sorry` 处表现不同的已知偏离，别照抄。

### P6 · 端到端做题体验（打开/写/改/提交）

**已落地的四件（别重做）**：

* **后台异步**：`did_change` 只记 `pending` 就返回；编译在 **spawn 出去的任务**里、
  **不持 `Docs` 锁**；只读请求读"上一次完成的状态"（`crates/lsp/src/lib.rs:523-551`）；
  慢文件才防抖（`SLOW_REBUILD=150ms`，`:592-593`），默认静默期 120ms（`:750-761`）。
* **不闪烁**：`showDelayMs=300` 展示延迟 + `begin/end` 不进节流（`edit-latency.md` §2）；
  守卫三条（stub 宿主 `P7:` ×2 + 真 VS Code `flicker_frames < keystrokes`）。
* **第一屏**：`node editor/vscode/test-project-first-screen.js --check`
  （三问：编完了吗 / 哪个版本 / 有没有问题）。
* **编译期间不冻编辑器**：实测把 8907ms 的 `stateAt` 干等修掉了（`:554-559`）。

**剩下的"数秒"在哪（已记账）**：**冷开**（缓存未命中）—— 同一进程连开三个课程文件，
冷 1804 / 9884 / 7015ms，热 11.0 / 19.3 / 30.6ms（`edit-latency.md:95-100`）；
且 `front::compile::cache` 的键**嵌了 `CARGO_PKG_VERSION`** ⇒ **每次 bump 全库作废**
（`:102-105`，本项目一天 bump 两次）⇒ 这是**反复发生**的，不是一次性预热。

**"提交"**：编辑器**没有**提交/判卷命令（`package.json` 的 18 条命令里没有）——
做题者的"提交"= 老师/agent 跑 `scripts/soko grade <file>`（或 `course`）。
它的成本 = 闭包编译 + judge；warm 时应命中产物（结构判据见 C4）。

### P7 · 编译链路本身要快（front 缓存）

**今天的成本结构（一次按键 · as-built 读数）**：

| 段 | 读数 | 归属 |
|---|---|---|
| 库层趟（依赖模块） | **已被检查点复用吃掉**（`modules=1`，改前 `5`） | G-29 ✓ 已落地 |
| 入口趟 · judge 合成前缀编译 | `prefix=5` 次 ≈ **一半按键成本**（实测 `702ms`；5 个 key **互不相同**、前缀 40295→43393 B ⇒ ≈207KB/按键） | **A2a**（抬 G-71 闸）→ A2b |
| 入口趟 · 三张 O(闭包) 派生表 | 各 1 遍/按键（`session.rs:421-431`） | **A4a**（纯接线） |
| 入口趟 · 入口自身 elaborate + telescope 解析 | ≈ 550ms；telescope 解析**无计数器** | **A4b**（先建读数） |

**`prefix=5` 是什么（P7 子调研实测，`/tmp/p7-missprobe2.log`）**：5 次全是
`judge_render_type` 的**冗余探针**，目标都含 `f '' A`/`f ⁻¹' B`
（`(f ⁻¹' (B ∪ C)) = …` 等 5 条）⇒ `Set.image {α β}` 的前导隐式 = 2 ⇒
`mentions_multi_implicit` 为真 ⇒ `by.rs:749-753` 压成 `ByMode::Off`（G-71 闸）。
A/B（同一二进制）：默认 **5 次/704ms** vs `SOKO_JUDGE_INPLACE=off` ⇒ **387 次/11234ms**
⇒ 就地路本来就在扛大头，**这 5 次是仅剩的漏网**。⇒ **A2a 就是冲它去的**。

**三把不同的键（互不替代）**：① 磁盘闭包级产物 `<模块根>/.sokonanoda/compiled/`
（键 = 整条闭包 `digest` ⇒ 改一字节必 miss，**只服务开档**）；② **进程内库层检查点**
（G-29，**服务按键**）；③ `ProjectPlan::module_keys()`（per-module Merkle，
**仍零生产调用**：`crates/front/src/project/mod.rs:204`，调用点只有 `project/tests.rs`）。

**"telescope / 签名级缓存"在本仓库指什么**（用户点名的方向，先对齐词）：
本仓库的 **telescope = 签名剥 Pi 层的元数概念**（`display.rs:741`·
`elab.rs:2449 notation_telescope`·`implicit.rs:46 telescope`），
**作为缓存层 as-built 不存在**——两处 telescope **每次都 `parse_expr_text`，无 memo**
（`elab.rs:2450`/`implicit.rs:47`）。已有的近似物只有
`judge_type_of_constant`（`judge.rs:2137`，键 =(options, 规范名)，只缓存**成功**的签名**文本**）
与 `KnownName::Decl::signature`（源级签名文本，随 `PassTables` 跨调用复用）。
⇒ 用户要的等价物 = **"签名文本 → telescope/arity"的记忆化**（A4b）。

**别照抄的两条**（perf-lean4 §4.5，仍然成立）：
① 别抄"语法逐字节相等当复用判据"（我们连入口命令布局都要重算）；
② 别回退到"编辑点之后全部重编"（S6 依赖脏集已领先，守卫
`crates/front/tests/keystroke_structure.rs`）。
**另加一条（本轮新增）**：`crates/front/tests/session_reuse.rs:153` 那条
`#[ignore]` **不是待办** ✗ —— 设计 §27.2/§28 明确"切片已停"；
`module_keys()` 要不要接（A3）**必须先过决策门**（§2 A3），不许当成"最便宜的一刀"直接开工。

---

## 2. 开发任务（按优先级）

> 每条给：现状 / 切入点（文件:行）/ **可执行判据** / 依赖 / 风险。
> **判据纪律**（`AGENTS.md`）：**结构计数优先**；绝对毫秒只兜数量级；
> 改断言值 = **往紧的方向改**，不许放宽；"必红"的守卫要能**反向验证**（撤掉修复必须红）。

### 阶段 A · 编译链路缓存（front）

> **A 阶段的形状（P7 调研后的排序）**：一次按键（unit08，改陈述）今天的读数 =
> `modules=1 by=91 infer=5/90 prefix=5` · **702ms**。其中
> **`prefix=5` 的合成前缀重编 ≈ 一半成本**（5 个 key **互不相同**、前缀
> 40295→43393 B ⇒ ≈**207KB/按键**重新 parse + 新建 arena/`EnvBuilder` + 重 elaborate
> 前缀全部声明的**类型**）。⇒ **先做 A2a（纯接线，直取这 5 次）**，再谈架构件。

#### A2a · 让就地路接管 `needs_explicit` 的 goal（**最高 ROI · 纯接线**）
* **现状（读码 + P7 子调研实测）**：这 5 次前缀重编**全是 `judge_render_type` 的冗余探针**
  —— 探针的目标都含 `f '' A`/`f ⁻¹' B` 这类**前导隐式 ≥ 2** 的 def
  （实测 `PREFIX_MISS` 5 条：`(f ⁻¹' (B ∪ C)) = …`、`(f '' (A ∪ B)) = …` …），
  于是 `by.rs:741` 的 `needs_explicit = mentions_multi_implicit(ty, defs)` 为真 ⇒
  `by.rs:749-753` 把 `by_mode` 压成 **`ByMode::Off`**（G-71 闸，0.81.0）⇒ 走慢路。
* **闸的理由已过期**：闸注释说"就地路的内核 pp 用默认选项 ⇒ 文本形态会分叉"，
  但 **2026-10-04 已修**：就地路在 `elab.rs:3061` 设
  `pp_options.explicit = judge::explicit_pp_active()`（`#check` 出口同款）。
  而慢路的 explicit 正是 `judge_render_type_explicit` = `ExplicitPpGuard::new(true)`
  + 同一个 `judge_render_type`（`judge.rs:2273-2281`）⇒ **两边读的是同一个线程局部**
  （`judge.rs:2238-2255`）。
* **改法（两步，别合并）**：① 给就地路（含 **`ByMode::Shadow` 分支**，`by.rs:757-790`）
  包一层 `ExplicitPpGuard::new(needs_explicit)`（该 guard 今天**是私有的**，要开一个
  受控入口）；② 再抬闸（`needs_explicit` 不再压 `Off`）。
* **判据（结构计数 · 缺一不算）**：
  ① `SOKO_LSP_TRACE=1` 在 `unit08-images-preimages` 上"改陈述"一刀：
  **`prefix=` 5 → 0**（`infer=` 相应下降）· ② **`SOKO_JUDGE_INPLACE_BY=shadow` 下
  `inplace_by_shadow()` 的 `diff=0` 且 `same>0`**（⚠ 该分支今天**完全没有 shadow 覆盖**
  —— 先补覆盖、先看红/绿，再抬闸）· ③ 全课程 `--json` **逐字节 0 行不同** ·
  ④ `G92-…sh` 仍 exit 1 · ⑤ **反向验证**：`SOKO_JUDGE_INPLACE=off` ⇒ `prefix` 回到数百
  （A/B 实测：默认 5 次/704ms vs off ⇒ **387 次/11234ms** —— 说明就地路本来就在扛大头，
  这 5 次是**仅剩的**漏网）。
* **依赖/并行**：无，**可立即开工**；与 A4a、B1 并行（文件不冲突：`by.rs`/`judge.rs`）。
* **风险**：**G-71 同类**——文本分叉可能被 `keep_if_lossless`/`is_rereadable` 吞掉 ⇒
  影子档必须比**tactic 级结果**，不能只比"有没有报错"；天花板 ≈ 按键成本的一半。

#### A2b · 判定的"前缀编译一次、回答多次"（**新机制 · A2a 之后再评估**）
* **现状**：judge 每次 miss 都**新建 arena + 新建 `EnvBuilder`**、把整份前缀重跑一遍
  （`judge.rs:2620 judge_infer_uncached` → `:1084 run_synthesized_incremental` →
  `check/mod.rs:654 run_incremental`）。A2a 之后如果还剩 >0 次，才值得动这一刀。
* **设计已写好的两条形状**：**(i) pass 借 builder**（`run_pass_with` 加借用变体）·
  **(ii) `EnvBuilder::fork()`**；精确阻塞 = **所有权**
  （`run_pass_with(builder, …) -> (…, builder, …)` 按值收发，judge 在 `elab_expr` 链里只有
  `&mut`，`incremental-environment.md:978-996`）。
* **判据**：`prefix`/`passes` 从 N → 1（或 0，若 A2a 已清空）；`by_calls` 不变；
  `--json` 逐字节 + 指针同一性单测
  （`crates/kernel/src/builder.rs::a_reused_environment_is_pointer_identical_and_a_rebuilt_one_is_not`）·
  `G-31` 的死结正是"`EnvBuilder` 没有从命令 k 续走的入口"（`declaration-incremental.md:305-306`）。
* **依赖**：A2a 的读数 · **与内核线 WIP（`stash@{0}: d68f2fe9-envprov-wip-1005-1010`）
  同一片地** ⇒ 先对齐归属。

#### A3 · 模块级产物 / `module_keys()` 接线（**先决策，后动手**）
* **现状（P7 调研核实）**：`module_keys()` **仍零生产调用**（`project/mod.rs:204`，
  唯一调用点 `project/tests.rs:529,611`）；**刀 2 未落地** —— 磁盘仍是
  `<根>/.sokonanoda/compiled/<整闭包 digest>.json`（`project/cache.rs:31/113/315`，
  一入口一条）；`session_reuse.rs:153` 的 `#[ignore]` 是"**已停，不是待办**"；
  **内核没有 `ExportFile → EnvBuilder` 的入口**（`builder.rs` 只有
  `new/hide_declars/restore_declars/with_env/finish`）⇒ **刀 2 = 新机制**，不是接线。
* **决策门（必须先做，判据是读数不是意见）**：量三个数 ——
  ① 冷开一个课程文件时"库层模块 vs 入口模块"的编译次数与占比（`closure_module_compiles_total()`）；
  ② 同一进程内**依次**打开 N 个共享同一库闭包的入口，库层被编几次（`by_calls`/模块计数）；
  ③ 版本 bump 后第一轮打开的重编量。**只有当"跨入口/跨进程的库层重复"仍是主要成本**，
  才做刀 2；否则**记一笔"不做"**（照 `declaration-incremental.md` §7.1 的先例，写清按数据不做的理由）。
* **判据（若做）**：`session_reuse.rs:153` 的守卫**删 `#[ignore]` 翻绿**（≤1 模块）；
  `module_keys` 命中计数（新增）；**反例**：改依赖一行 ⇒ 必须 miss 且重编；
  与 CLI build 的跨组残余 **2.08×** 同源（`Σ闭包 517→249`，perf ledger L34）。

#### A4a · 库层三张 O(闭包) 派生表进检查点（**纯接线**）
* **现状**：即使库层检查点命中，**每次按键仍重建三张 O(闭包) 表**
  （`project/session.rs:421-431`）：`closure_prefixes_for`
  （`check/mod.rs:1048` —— **建 N 份只用最后一份**）· `display_notations`
  （`check/mod.rs:501`，clone 全部命令 + 建表 + arity）· `top_level_def_spans_over`
  （`check/mod.rs:1599`）。表是**闭包源文本的纯函数**，键 `lib_key` 现成。
* **判据**：新增结构计数（"派生表重建数"）：冷开 = 1、**第 2..N 刀 = 0**
  （或 `closure_prefixes_for` 调用数第 2 刀起 = 0）；`--json` 逐字节不变。
* **天花板（诚实记）**：**个位数 %**（perf ledger 的 `closure_stages`：`plan_ms 0.35`
  vs `compile_ms 43.48`）——价值在**去掉 O(闭包) 伸缩**，不在绝对量。
* **依赖**：无，与 A2a 并行。

#### A4b · telescope / 签名级缓存（**用户点名的方向 · 今天 as-built 无此物**）
* **现状（P7 调研核实）**：本仓库的 telescope = 签名剥 Pi 层的元数概念
  （`display.rs:741`·`elab.rs:2449 notation_telescope`·`compile/implicit.rs:46 telescope`），
  **作为缓存层不存在**：两处 telescope **每次都 `parse_expr_text`，无 memo**
  （`elab.rs:2450`/`compile/implicit.rs:47`）。已有的近似物只有
  `judge_type_of_constant`（`judge.rs:2137`，键 =(options, 规范名)，CAP 4096，
  只缓存**成功**的签名**文本**）与 `KnownName::Decl::signature`（源级签名文本，
  随 `PassTables` 跨调用复用）。**用户要的等价物 = "签名文本 → telescope/arity" 的记忆化**。
* **切入点**：给 telescope 解析加 memo（键 = 签名文本 + options），
  先补**计数器**（今天 `telescope` 解析次数**没有任何出口** ⇒ 先建读数再谈优化）。
* **判据**：新增计数"telescope 解析次数/按键"（冷开 = k、第 2 刀起 = 0 或只算新声明）；
  `--json` 逐字节 + 影子档 `diff=0`。
* **依赖**：**先建计数器**（否则无法证明收益）· 与 A4a 同一片地（`session.rs`）⇒ 串行。

### 阶段 B · 前缀增量 + 目标语义（P3/P4）

#### B1 · 失败证明**保留前缀目标**（P3 显示面 · **本阶段最高 ROI**）
* **目标**：最后一条 tactic 错 ⇒ 前面每条 tactic 的目标**照样看得到**；
  `total` 不再是 0（面板能显示"第 2/3 条"）。
* **切入点（候选 A · 最小）**：`crates/front/src/by.rs:1302` 让 `run_tactics` 的 `Err`
  **把已算的 `steps` 一起交还**（形状自选：`Result<ByOutcome, ByFailure{error, steps}>`
  或 `ByOutcome{expr, steps, error: Option<…>}`）→ `crates/front/src/compile/check/mod.rs:386/438`
  的 `Result` 带 partial → 三个失败分支（`walk.rs:819`/`:1190-1199`/`:1622`）
  与 `kernel_phase.rs:182` 写进 failed 的 `by_steps`。
  ⚠ **`query/state.rs` 零改动**：`by_steps` 非空即自动走逐 tactic 选择，`by_root` 仍管题面。
* **判据（三层，缺一层就是洞）**：
  ① **front 真相层**（`crates/front/src/compile/tests.rs` 新增失败块用例）：
  `theorem … := by apply And.intro; exact hp; exact hp` ⇒ `status == Failed` 且
  **`by_steps.len()`：0 → 2**；`select_state_at(d, 第 2 条 tactic 起点).goals`：
  `[题面]` → **`[Q]`**。
  ② **LSP wire**（`crates/lsp/src/tests/state.rs` 新增）：同一夹具
  **`total`：0 → 2 · `step`：-1 → 1 · `goals[0].goal`：题面 → `"Q"`**。
  ③ **e2e / webview**：光标停在第 2 条 tactic ⇒ 面板显示它**作用后**的目标（与 B2 合用一条）。
* **反向验证**：把 `walk.rs` 那两处改回不写 `by_steps` ⇒ ①②必须红
  （`scripts/expect-red.sh` 的形状）。
* **零回归（先跑一遍存基线）**：`--json` 事件流逐字节不变（错误条数/span/事件计数不变）；
  `keystroke_structure` 的 `recomputed_commands` 1/1/4 · `g29` 的 `edit==1` ·
  LSP `state_at_without_by_steps_returns_the_declaration_goal`（无 `by` 声明）**保持绿**。
* **红线/坑**：`by_steps` 的语义变了（失败声明也有步进）⇒ 按
  `crates/front/src/compile/report.rs:75-90` 的纪律**必须 bump 报告形状版本**
  （否则旧缓存条目会静默给"没有步进"的旧答案 —— G-78 踩过）；
  协议文档（`docs/protocol.md` §`soko/stateAt`）同轮改。
* **依赖**：无。**可第一个做**（与 C1/C2 并行，文件不冲突）。

#### B2 · 光标语义改为"**作用后**" + 末尾 🎉/Q.E.D（P4）
* **改法（一处规则 + 三处文档/测试）**：`crates/front/src/query/state.rs:124-136`：
  `start <= cursor < end ⇒ step i-1` 改成 **`start < cursor < end ⇒ step i`**
  （光标恰在起点 ⇒ 仍是"进入"；恰在末尾/之后 ⇒ 已归入"最后一条结束于光标前"，
  与 Lean 的 `hoverPos > pos` 一致）。
* **判据（可执行，逐条给值）**：
  ① **front**：`crates/front/src/query/tests.rs:252-274`
  （`state_at_inside_a_tactic_shows_the_entering_state`）**改名 + 改值**：
  光标在 `apply And.intro` 的 `find+3` ⇒ **`step` 2 → 3**、**`goals.len()` 1 → 2**
  （它现在是第 4 条 tactic，作用后 = `And.intro` 的两个子目标）。
  ② `crates/front/src/query/tests.rs:276-298`（`state_at_after_apply_lists_every_sub_goal`）
  的取点**不在任何 tactic 内**（在 `apply` 末尾与 `sorry` 之间）⇒ **不受影响**，保持绿。
  ③ **边界**：`cursor == span.start` 仍"进入态"；根状态判据
  `crates/front/src/query/tests.rs:68` 不变。
  ④ **LSP**：`crates/lsp/src/tests/state.rs:11`（传 `intro h` **起点**）与
  `:118-130`（传 `intro h` **末尾**）**都不受影响** ⇒ 正好当**双向守卫**；
  新增一条：光标在**末条 tactic 之内**（非起点）⇒ `step == total-1`、`goals == []`、
  `goal == null`、`decl.status == "checked"`。
  ⑤ `crates/cli/tests/query.rs:133`（`--col 3` = tactic 起点）与 `:1142`（都取起点）
  **不受影响**；CLI≡LSP 契约 `crates/cli/tests/query.rs:970-1005` 保持绿。
  ⑥ **文档**：`docs/protocol.md:453-455`（唯一活契约）· `state.rs:17-48` 的注释 ·
  `crates/front/src/query/mod.rs:1246-1250` · `crates/lsp/src/lib.rs:1115-1119`。
  ⚠ `docs/design/by-tactics.md`/`goal-rendering.md` **已被删除**（`dae75759`），
  但代码注释仍在引用 ⇒ 顺手把悬空引用改成 `docs/protocol.md`（文档卫生，别新建文件）。
* **🎉/Q.E.D.**：`media/infoview.js:392-398`（今天一律「已无目标 ✓」）与
  `extension.js:572-578`（树项）按 **`total > 0 && goals.length === 0 && step === total-1`**
  + `decl.status === "checked"` 分叉出 **「🎉 恭喜，证完了（Q.E.D.）」**；
  `open`/`failed`/`total:0`（`def`/`axiom`）⇒ 保留「已无目标 ✓」。
  **不需要新 wire 字段**（`protocol.rs:183-203` 已有全部判据）。
  判据：`node editor/vscode/test-webview.js` 喂
  `{decl:{status:"checked"},goals:[],goal:null,step:1,total:2}` ⇒ 断言 DOM 出现
  🎉/Q.E.D.；喂 `{total:0,goals:[]}` ⇒ 断言**不**出现。
  e2e（用户可见）：`editor/vscode/src/test/extension.test.js` 的夹具
  `.../fixtures/workspace/units/u01.sokonanoda` **末尾追加**一条完整两条-tactic 定理
  （追加不动既有行号），光标放末条 tactic **中间**，
  `waitFor(lastState().goals.length === 0 && total === 2)`（今天此处非空 ⇒ **先红后绿**）。
  **反向验证**：加 `SOKO_STATE_AFTER=0` 逃生门（照 `SOKO_STATE_ROOT=legacy` 的先例）
  或 stash 选择器改动 ⇒ ①④⑤与 webview 判据**必须红**。
* **依赖**：无（但**改的是协议语义** ⇒ 设计先行一段 + 协议文档同轮改，属"用户可见改动"，
  要按收尾义务同步 `editor/vscode/README`/`CHANGELOG`/`skills/` 三处）。

#### B3 · 失败块的**严格趟从失败步续跑**（P3 成本面 · 依赖 B1）
* **现状**：任一判定非 `Match` ⇒ `flush_batch` 只回一个 bool（`judge.rs:2058`/`:2095-2097`）
  ⇒ `run_by` 丢掉乐观趟、用**完整** `tactics` 从头严格重跑（`by.rs:1198-1217`）。
  实测：失败块的走查数 = **1 + N**（N = 出错位置），随出错位置增长。
* **改法（候选 C）**：`flush_batch` 返回**首个非 Match 的下标**；`run_by` 只从该步续跑，
  前缀状态由 B1 的 partial `steps`/节点图提供。
* **判据**：`judge.rs::tests::one_by_block_pays_a_single_document_pass`（`:3612-3649`）
  家族 —— 失败块（第 3 条错）走查数 **3 → 2**（1 乐观批 + 1 续跑），并新增一条
  **"走查数不随出错位置增长"**（fail1 与 fail3 相等）。
* **红线**：诊断/文案必须与"整段重跑"**逐字一致**（否则 golden `--json` 判红）；
  乐观趟控制流等价性论证（`by.rs:1174-1181`）只在全 Match 时成立 ⇒ 续跑必须重新论证。
* **依赖**：B1（先有 partial 状态）。

#### B4 · 逐 tactic 前缀快照缓存（**决策门 · 最后做**）
* **候选 B（重）**：在 `by.rs:1277-1288` 前挂一层按（环境身份 + tactic 前缀 hash）
  索引的 `(nodes, worklist, steps)` 快照，载体可放 `Session::CmdSnapshot`（`session.rs:82`）
  或仿 `judge_cache`（`judge.rs:266-287`）。
* **为什么放最后**：**错键 = 静默用旧目标判定**（红线）—— `EntryCache` 的三次静默错编
  就是前车之鉴（`query/mod.rs:123-130`/`:259-264`）；而 B1+B3 已经消掉用户看到的症状
  与"失败块 1+N"的主要成本。⇒ **先量 B3 之后的读数，再决定做不做**；
  不做就照 `declaration-incremental.md` §7.1 的先例**写清按数据不做的理由**。

### 阶段 C · 做题 UX（P1/P2/P5/P6）

#### C1 · build 进度：逐文件 + 短间隔（P1 · **快赢**）
* **改法（全部走 stderr，`--json` 一个字不动）**：
  ① 人看的那条从"每 10%"改成**每个文件一条**（`build.rs:295-297`），
  带**文件名**（`done/total · <相对路径>`，复用 `shorten_path`，`:787-797`）；
  ② 模块级 tick **也喂人看的那条**（把 `:606-607`/`:660-661`/`:691-692` 的
  `if json { Some(&mut sink) } else { None }` 改成"json ⇒ 事件；非 json ⇒ 人看的一行"），
  并按**时间**节流（≥150–250ms 才打一条，避免 861 文件刷屏）；
  ③ 终端默认给**人类可读**心跳（≥1s 且距上一条输出 ≥1s），**不恢复** JSON 心跳
  （2026-09-29 投诉的是 JSON 刷屏，不是"动起来"）。
* **判据（结构计数，可执行）**：新守卫 `scripts/check-progress-cadence.py`
  （或给 `check-progress-gap.py` 加 `--human`）：N=120 个文件的夹具
  ⇒ **stderr 进度行数 ≥ N**（今天 `step=12` ⇒ **只有 10 条**）；
  单入口长编译夹具 ⇒ **最长无输出间隔 ≤ 2.5s**（人看的那条）。
  `--selftest` 反向验证（合成"每 24 个一行"的流 ⇒ 必须判红）。
  **零回归**：`crates/cli/tests/cli.rs:2230+`（`build.progress` 顺序）、
  `crates/cli/tests/imports.rs:1061`（逐入口事件逐字节）保持绿。
* **依赖**：无。**风险**：低（纯输出层）；唯一红线是 stdout 的确定性。

#### C2 · `clean`/`rebuild` 无参也清模块根（P2 残留 · **快赢**）
* **改法**：`clean_stores` 在 `args` 为空时用与 `warm` **同一个默认**（cwd + 同一份
  `SKIPPED_DIRS` 扫描）去解模块根；`clean` 的"只清不编"语义不变；
  `:441-445` 的提示改成"扫到的 N 个模块根也清了"。
* **判据（可执行）**：新 CLI 测试（`crates/cli/tests/cli_surface.rs` 或 `artifacts.rs`）：
  项目夹具（`lib/Shared` + `Main` import）⇒ 冷 `build` → **无参 `rebuild`**
  ⇒ `build.clean.project ≥ 1` **且 `build.summary.hit == 0`**（今天 `hit == 1`）；
  **反向验证**：把默认改回空集 ⇒ 必须红。
* **依赖**：无。**风险**：低；注意**别把 `clean` 变成"会编"**（它必须只清不编）。

#### C3 · `#check` / `#print` 进 infoview（P5 · 三层）
* **改法**：
  ① **真相层**：`DocumentReport` 增加命令输出（`#check` 已有 `checks`；
  `#print` 要把 `Printed{name,text}` 带进报告 —— 建议统一成
  `outputs: Vec<CommandOutput{kind, span, text}>` 并**保留 `checks`**（inlay hint 在用）；
  **必须 bump 报告形状版本**）；
  ② **wire**：新增自定义请求（建议 `soko/commandOutput`，返回本文档全部命令输出
  + range；或把"光标处命令的输出"并进 `soko/stateAt`）——
  **设计先行**，写进 `docs/protocol.md`，跑 `python3 scripts/audit-wire-fields.py`；
  ③ **渲染**：`media/infoview.js` 加一块「命令输出」（`#check` ⇒ `文本 : 类型`，
  `#print` ⇒ 定义文本），光标落在该命令时显示。
* **判据（三层，缺一即洞 · 逐条给值）**：
  ① **front 真相层**：`crates/front/src/session.rs` 新增
  `report.prints.len() == 1 && text.contains(":=")`（今天恒 0）；`#check` 已有
  （`report.checks`，`crates/front/src/compile/tests.rs` 覆盖）。
  ② **LSP wire**：`crates/lsp/src/tests/state.rs` 新增 —— `#check` 消息字段存在性
  （`messages[i].text` 含类型 + `range` 存在；`crates/lsp/src/inlay.rs:307` 已有 inlay 侧）
  · `#print`：`prints[0].name == "myId"` 且 `text` 非空；
  `python3 scripts/audit-wire-fields.py` 覆盖新字段（A∖B 对账）。
  ③ **渲染（用户可见）**：`node editor/vscode/test-webview.js` 断言 DOM 里出现
  `#check` 的类型与 `#print` 的定义文本；e2e 侧用新访问器
  （`editor/vscode/src/test/extension.test.js` 现在把 `#check` 断言成 **inlay hint**，
  `:244-271` —— 新增消息面断言，别改掉 inlay 那条）。
  **反向验证**：去掉 `printed` 字段 ⇒ ②必须红。
* **设计要定的两件事**：① 通道形态（新请求 `soko/commandOutput` vs 并进 `soko/stateAt`；
  Lean 的对应物是 `getInteractiveDiagnostics{lineRange?}` ⇒ **按光标行取**最贴）；
  ② `Printed` 今天**没有 span**（`kernel_phase.rs:519-534`）⇒ range 挂**命令 span**
  还是**名字 span**（Lean 用 `withRef tk`）。
* **依赖**：协议设计先行；与 B2 同轮会**同时改 `infoview.js`** ⇒ 串行或同一作者。

#### C4 · 冷开 / 提交链路的判据（P6）
* **目标**：把"打开/提交"从"感觉慢"变成**会判红的数字**。
* **判据**：① warm 下 `sokonanoda grade <项目入口>` ⇒ 结构上"产物命中"
  （`build.summary.hit == files`，用 `build` 代理）· ② **冷开**：首屏守卫
  `node editor/vscode/test-project-first-screen.js --check` 在**编译中**也必须答得上三问 ·
  ③ 版本 bump 的影响：记录"bump 后首轮打开重编的模块数"（今天的键嵌 `CARGO_PKG_VERSION`
  ⇒ 全库 miss）；若 A3 做了模块级产物 ⇒ **该数只算入口模块**（结构判据）。
* **依赖**：A3（跨进程产物）· 与 D3 共用夹具。

### 阶段 D · 调研补充（只读，可全程并行）

* **D1 · Lean 4 infoview / `#check`/`#print` 机制对照**：**本轮已完成**（P4 的
  `InfoUtils.lean:448-481` + `RequestHandling.lean:195-198` + `InfoTree/Types.lean:150-155`
  + 权威样本 `tests/lean/interactive/plainGoal.lean.expected.out`；P5 的
  `BuiltinCommand.lean:421-441` / `Print.lean:224-226` / `WidgetRequests.lean:113-117`
  消息通道；§附录 B 已收）。**剩余一条**：`#print` 的消息 range 到底取 `tk` 还是 `id`
  （`Print.lean:224-226` 的 `withRef tk`）—— 进 C3 的设计时一并确认。
* **D2 · build CLI 惯例**（支撑 C1）：逐条取证（**本机跑**，不靠记忆）：
  `cargo build`（逐 crate 一行？`--timings`？）· `tsc --pretty`（逐文件？）·
  `rustc`/`lean`/`lake` 的进度粒度与**默认路径语义**（cargo 无参 = 当前目录？
  `lake build` 无参？）。**产出**：一张"粒度 / 默认路径 / 是否可取消"的对照表 +
  C1 的口径结论（逐文件 vs 短间隔）。**判据**：每条给命令 + 输出片段（可复跑）。
* **D3 · 端到端做题基准**：定义并落一条**结构计数为主**的基准：
  一次按键 → 目标面板更新的**结构读数**（模块编译数 / `by_calls` / `prefix_runs` /
  `stateAt` 的 `step`），墙钟只兜数量级；夹具 = `playground.sokonanoda` +
  一个 `import` 项目（`unit08` 同款）。**产出**：`docs/PERF.md` 的新一节（不是新文件，
  避免文档预算）。

### 阶段 E · prelude 显式化（P8 · 教学工具初心 · 与 A/B/C 独立，可插进任一批次）

> **用户原话要点（2026-10-06 提过、一直未落地）**：prelude 代码要**显式独立**出来，
> 不要嵌到 Rust 代码里；教学工具要保持初心，把更多内容**显式暴露**出来 —— 预置的
> prelude / 课程基础代码应该是**独立、显式、可查看的源文件**，让学生/工具能显式看到与修改。
> 已有记档：`docs/design/course-stdlib.md` **§7**（2026-10-06，"待办、不做排期"）。

#### P8 现状（as-built · 读码 + 实测 · 这决定了 E1 的形状）

1. **真相 = Rust 字符串常量**：`crates/front/src/compile/prelude.rs` 的
   `PRELUDE_EQ_SRC`（`:175-179`，4 行）· `PRELUDE_L1_SRC`（`:210-298`，88 行）·
   `QUOT_TYPES_SRC`（`:317-334`，17 行）—— 共 **≈109 行源文本**，用 `"\` 续行写在 Rust 里。
2. **仓库里已有一份"镜子"**：`prelude/Prelude.sokonanoda`（**已跟踪**，108 行 / 9287 B）
   = `prelude_source()`（`:341-344`，三段拼接）的**物化副本**；守卫
   `crates/front/tests/prelude_mirror.rs:35-78` 断言"**常量 ⇒ 文件**逐字节相等"
   （注释原话：单一真相留在编译期常量，镜子是产物，**绝不**改成运行时读文件
   —— 会碰 `PreludeMode::Bare` 那条红线）。
3. ⇒ **学生今天看得到一份，但改它没用**：改 `prelude/Prelude.sokonanoda` 只会让镜子守卫
   判红，**编译行为一个字不变** ✗（真相在 Rust 里）。
4. **F12 的落点是缓存副本**：`prelude_source_path()`（`:373-413`，A4/2026-09-26）把 prelude
   幂等写到 `<cache>/prelude/Prelude.sokonanoda`（兜底系统临时目录），LSP 用它做跳转
   （`crates/lsp/src/lib.rs:2089`/`:2163`）⇒ 学生打开的是**缓存里的副本**，不是仓库那份。
5. **Nat/Bool 家族 9 个名字完全没有源文本**：`install_prelude`（`:924-1008`）与
   `install_bool_prelude`（`:1009-`）是**手搓 Rust AST**（`Expr::Sort`/`CtorDecl`/
   `Span::default()`）⇒ `prelude_def_span` 对它们**如实**返回 `None`（`:346-371`）。
   （对照：`And`/`Or` 是**真归纳块**、写在 `PRELUDE_L1_SRC` 里，由 `install_l1_prelude`
   parse 后走同一条受信任安装路 —— 见 `:210-231`。）
6. **一处已知不对称**（`course-stdlib.md` §7 的目标）：慢路合成重跑**不含 prelude**
   ⇒ prelude 安装期的 recursor 判定无法重跑 ⇒ 影子档把该期**排除出比对**
   （`crates/front/src/compile/elab.rs:3312-3326`）⇒ 那段没有严格比对。
7. **悬空引用**：设计文档 `docs/design/prelude-l1-proposal.md` **已被删除**，
   但代码注释与活文档仍在引用（`prelude.rs:77`/`:181`、`crates/cli/tests/cli.rs:966`、
   `crates/front/src/compile/tests.rs:1780`、`teaching-project.md:465/479/486`、
   `course-stdlib.md:50/75/333`）。

#### E1 · **方向翻转：文件 ⇒ 常量**（推荐先做 · 低风险 · 直击用户原话）

* **改法**：三段源文本落成**三个真源文件** —— `prelude/Eq.sokonanoda`（≈4 行）·
  `prelude/L1.sokonanoda`（≈88 行）· `prelude/Quot.sokonanoda`（≈17 行）；
  Rust 里**只留** `include_str!("../../../prelude/L1.sokonanoda")` 之类
  （**保持编译期常量语义** ⇒ `PreludeMode::Bare` 红线、离线/自足分发、零运行时 IO 全不变）。
  `prelude/Prelude.sokonanoda` **保留**为**生成的合并视图**（内容与行号与今天逐字节相同
  ⇒ 课程文档的 `:20` 引用不漂），生成命令不变
  （`SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror`）；
  镜子守卫**翻转方向**（文件 ⇒ 视图），并新增"**Rust 里不许再有 prelude 源文本**"判据。
* **为什么不是"单文件 + 运行期切分"**（诚实记）：`install_l1_prelude`（`:584`）·
  `install_eq_prelude`（`:858`）· `goals.rs:79/101` · `display.rs:844-845`
  **分别**解析 Eq 与 L1 ⇒ 单文件要么引入切分标记（新语法面）、要么改四处消费点
  ⇒ 风险不必要。**"一个文件"这件事由合并视图满足**（学生读它、课程引用它）；
  真要单文件真相源，放 E3 之后另立决策门。
* **判据（结构计数 · 可执行 · 缺一不算）**：
  ① **哨兵行 grep**（三条各自唯一 —— 今天各 **1** 命中，就是那三个常量）：
  `axiom Classical.em : (p : Prop)` · `def Or.elim {a b c : Prop}` ·
  `axiom Quot.mk {u} : {A : Sort u}` ⇒ 在 `crates/**/*.rs` 里 **= 0**。
  ⚠ **别拿 `axiom True : Prop` 当哨兵**：它同时是**大量测试夹具**的一行
  （今天 `crates/` 29 命中）⇒ 会假红 ·
  ② `cargo test -p sokonanoda-front --test prelude_mirror`（**改断言方向，不是删**）：
  三段文件拼接 == 视图逐字节 **且** 新增"Rust 源里没有 prelude 文本行"·
  ③ `prelude_source()` **字节不变** ⇒ 全课程 `--json` 逐字节 0 行不同 +
  `scripts/kernel-diff.sh --fast` 零差异 + `python3 courses/set-theory/tools/check.py` 绿 ·
  ④ **反向验证**：改 `prelude/L1.sokonanoda` 一行（如把 `True` 改名）⇒ 重编后
  **行为必须变**（一条依赖它的探针判红）—— 今天改文件只让镜子红、行为不变 ⇒
  这条正是"文件是不是真相源"的判据 ·
  ⑤ 课程文档行号不漂：`sed -n '20p' prelude/Prelude.sokonanoda` 仍是
  `def Or.elim …`（`courses/set-theory/OPEN-ITEMS.md:152` 引用它）·
  ⑥ F12：`cargo test -p sokonanoda-lsp --lib -- tests::navigation` 保持绿，
  **且检出仓库时落点是仓库里的 `prelude/*.sokonanoda`**（不再写缓存副本）·
  ⑦ 悬空引用清理：`grep -rn "prelude-l1-proposal"` 归零（改指
  `prelude/*.sokonanoda` 或 `course-stdlib.md`；**不新建文件**）。

#### E2 · Nat/Bool：**源化 或 如实登记边界**（先 spike，再定）

* **问题**：9 个名字（`Nat`/`Nat.zero`/`Nat.succ`/`Nat.rec`/`Bool`/…）今天没有源位置
  ⇒ "prelude 全部显式"这一半做不到，**要么源化、要么如实登记**。
* **源化的依据（不是空想）**：L1 的 `And`/`Or` 就是**教学语法里的真归纳块**
  （`prelude.rs:210-231`），走的是 parse → 受信任安装同一条路 ⇒
  `inductive Nat` 原则上可写。**但** Nat/Bool 是**最先装**的（`check/mod.rs:931-960`），
  且有 `PRELUDE_NEVER_YIELDS`/`prelude_shape` 的"文件自带就让位"逻辑 ⇒ 需要一次 spike。
* **spike 判据**：源化版与手搓版在**全课程 `--json`** 上**逐字节相同** +
  `prelude_def_span("Nat")` 从 `None` 变成**真 span**（F12 可用）+ 课程门禁绿。
  做不到 ⇒ 在 `prelude/Prelude.sokonanoda` 加一条
  `-- sokonanoda:builtin-rust "Nat/Bool 家族由内核手搓，无源位置"` 登记行
  （照 `builtin-sugar` 登记区的先例），并给 `prelude_def_span == None` 补一条测试钉住。

#### E3 · 运行时覆盖 `--prelude` / `SOKO_PRELUDE_PATH`（**决策门** · "可修改"的另一半）

* **动机**：学生用 release VSIX，没有 cargo ⇒ 只有**运行时覆盖**才能"改 prelude 就见效"。
* **约束（缺一不可）**：默认**关**（自足分发不变）· 覆盖内容**哈希进缓存键**
  （否则旧缓存会服务旧结果）· 畸形覆盖 ⇒ **诊断而不是 panic**
  （`install_l1_prelude` 的 `panic!("L1 prelude source parses")` 在覆盖下**变成可达路径** ✗）·
  `--bare` 行为不变 · 受信任安装语义不变（覆盖的仍是 prelude）。
* **判据**：① 覆盖后一条依赖 prelude 的探针**行为改变**（结构计数/判定翻转）·
  ② **反例**：换覆盖内容 ⇒ 缓存**必须 miss**（键折了内容哈希）·
  ③ 畸形覆盖 ⇒ 退出码/诊断符合契约、**不 panic** · ④ 不设覆盖时全课程 `--json` 逐字节不变。

#### E4 · 撤影子档的 prelude 排除（依赖 E1 · `course-stdlib.md` §7 的目标）

* **改法**：prelude 成为**普通源文本** ⇒ 合成前缀可以把它拼进去 ⇒
  撤 `elab.rs:3312-3326` 的"prelude 安装期不比"分支，恢复该期的严格比对。
* **判据**：排除分支的**命中计数 = 0**（新增计数出口）· 全课程影子档
  **`same > 0 && diff = 0`**（今天靠排除才不出 diff）· 全课程 `--json` 逐字节不变。
* **风险**：这一段是**受信任安装**的判定 ⇒ 出 diff 就是真分歧（不是噪声）⇒
  按判定红线的三层回归走；**先让 shadow 在排除仍在的情况下能跑通**，再撤排除。

#### 阶段 E 的依赖与并行

* **E1 与 A/B/C 完全独立**（只碰 `crates/front/src/compile/prelude.rs` + `prelude/*` +
  `crates/front/tests/prelude_mirror.rs`）⇒ **可以插进任一批次**；唯一约束：
  **E1 与 E2 同文件（`prelude.rs`）⇒ 串行**；E4 碰 `elab.rs`（与 A2b 同片地 ⇒ 串行）。
* **建议**：E1 作为一个**独立小批次**（改动机械、判据硬）→ E2 spike → E4 → E3（决策门）。

---

## 3. 依赖、并行与冲突面

### 3.1 与发版闭环的关系

* **必须等发版闭环的**：任何 **bump/版本文件/`STATUS.md`/`CHANGELOG`/台账**动作；
  e2e 台账（按批次记一条）；release 相关的 push。
* **不必等的**：`crates/**` 的代码改动本身（在**不 bump** 的前提下）、文档、
  只读调研。⇒ C1/C2/B1/B2/A2a 可以**发版一结束就开工**；A2b/A3/A4b 反正要先出设计。
* **合入纪律**：一个环节一个 commit；批次收尾才 push（`AGENTS.md` 批次制）。

### 3.2 文件占用 / 冲突矩阵（同一时间只允许一个写者）

| 文件/区域 | A2a | A2b | A3 | A4a | A4b | B1 | B2 | B3 | C1 | C2 | C3 | E1 | E2 | E3 | E4 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `crates/front/src/by.rs` | ✎ | | | | | ✎ | | ✎ | | | | | | | |
| `crates/front/src/judge.rs` | ✎ | ✎ | | | | | | | | | | | | | |
| `crates/front/src/compile/check/mod.rs` | | ✎ | | ✎ | | ✎ | | | | | ✎ | | | | |
| `crates/front/src/compile/check/walk.rs` | | ✎ | | | | ✎ | | | | | | | | | |
| `crates/front/src/compile/elab.rs` | | | | | | | | | | | | | | | ✎ |
| `crates/front/src/compile/prelude.rs` | | | | | | | | | | | | ✎ | ✎ | ✎ | |
| `prelude/*.sokonanoda` + `prelude_mirror.rs` | | | | | | | | | | | | ✎ | ✎ | | |
| `crates/front/src/query/state.rs` | | | | | | | ✎ | | | | | | | | |
| `crates/front/src/compile/report.rs` | | | | | | ✎ | | | | | ✎ | | | | |
| `crates/front/src/project/*` | | ✎ | ✎ | ✎ | ✎ | | | | | | | | | | |
| `crates/cli/src/build.rs` | | | | | | | | | ✎ | ✎ | | | | | |
| `crates/lsp/src/*` | | | | | | | ✎ | | | | ✎ | | | | |
| `editor/vscode/media/infoview.js` | | | | | | | ✎ | | | | ✎ | | | | |
| `docs/protocol.md` | | | | | | ✎ | ✎ | | | | ✎ | | | | |

⇒ **建议批次**（同列相撞 = 必须串行）：
**批次 1** = **A2a**（`by.rs`+`judge.rs`，纯接线）**∥ C1+C2**（`cli/build.rs`，另一片地）
**∥ E1**（`prelude.rs`+`prelude/*`，独立小批次）
→ **批次 2** = B1（`by.rs`+`walk.rs`+`check/mod.rs`+`report.rs`）→
**批次 3** = B2（`state.rs`+`lsp`+`infoview`+协议）→ **批次 4** = A4a（`project/*`+`check/mod.rs`）
+ B3（`judge.rs`+`by.rs`）→ **批次 5** = C3（协议+`report.rs`+`infoview`）→
**之后** = A2b/A3/A4b/B4/C4/E2/E3/E4（先过各自的决策门；A2b/E4 与内核线协调）。
⚠ **A2a 与 B1 都改 `by.rs`**、**A2b 与 B3 都改 `judge.rs`**、**E1/E2/E3 都改 `prelude.rs`**
⇒ 不许并行（`AGENTS.md`：同一模块同一时间只允许一个写者）。

### 3.3 与内核线/其他会话的协调点

* 内核线的 WIP 停在 `stash@{0}: d68f2fe9-envprov-wip-1005-1010`（A2b 的同族）；
  开工前先读 `docs/notes/HANDOFF-kernel.md` 并确认归属。
* `docs/notes/perf-lean4-interactive-incremental.md` 的两处过期（§0.2 ①/②）：
  **不要**按它去实现 §4.0/§4.1；引用时只引 Lean 侧机制。

---

## 4. 判据总表（照抄可执行）

| 任务 | 命令 | 期望（结构计数优先） |
|---|---|---|
| A2a | `SOKO_LSP_TRACE=1` 在 `unit08-images-preimages` 上"改陈述"一刀，读 `prefix=` | **5 → 0**（`infer=` 相应下降；今天 `modules=1 by=91 infer=5/90 prefix=5`） |
| A2a | `SOKO_JUDGE_INPLACE_BY=shadow`（先补该分支的 shadow 覆盖） | **`diff=0` 且 `same>0`**（今天该分支零覆盖） |
| A2a | `SOKO_JUDGE_INPLACE=off`（反向对照，同一二进制） | 回到 `prefix≈387`（证明这 5 次是"仅剩的漏网"） |
| A2a/A2b/A4a/A4b | 全课程 `SOKONANODA_BUILD_JOBS=1 sokonanoda build --json courses/set-theory`（剔心跳） | 与基线**逐字节 0 行不同** |
| A2b | `bash docs/gaps/repro/G92-by-prefix-rerun-is-quadratic.sh` | `exit 1`（`by_calls` 比值 < 3.0） |
| A3 | `python3 scripts/check-recompile-factor.py` | ratchet 不增；跨入口 case 命中计数达标；`session_reuse.rs:153` 去 `#[ignore]` 后翻绿 |
| A4a | 新增"派生表重建数"读数 | 冷开 = 1、第 2 刀起 = **0** |
| A4b | 新增"telescope 解析次数/按键"读数（今天**没有出口**） | 冷开 = k、第 2 刀起 = 0（或只算新声明） |
| B1 | front：`cargo test -p sokonanoda-front --lib -- compile::tests`（新增失败块用例） | `by_steps.len()` 0 → 2；`select_state_at` 目标 题面 → `[Q]` |
| B1 | LSP：`cargo test -p sokonanoda-lsp --lib -- tests::state` | `total` 0→2 · `step` -1→1 · `goals[0]` 题面→`"Q"`；无 `by` 声明那条**保持绿** |
| B1 | `sokonanoda --json <夹具>` | 事件流与改前**逐字节相同** |
| B3 | `cargo test -p sokonanoda-front --lib -- judge::tests::one_by_block_pays_a_single_document_pass` | 失败块走查数 3 → 2；不随出错位置增长 |
| B2 | `cargo test -p sokonanoda-cli --test query` | 光标在第 2 条 tactic ⇒ `step == 1`（今天 `0`） |
| B2 | `node editor/vscode/test-webview.js` | 可见文本含 🎉/Q.E.D.（`checked`）；`open` 不含 |
| C1 | `python3 scripts/check-progress-cadence.py`（新） | N=120 ⇒ stderr 进度行 **≥120**；最长间隔 ≤2.5s |
| C1 | `python3 scripts/check-progress-gap.py` | 仍绿（`--json` 心跳契约不变） |
| C2 | `cargo test -p sokonanoda-cli --test cli_surface` | 无参 `rebuild` ⇒ `hit == 0`、`clean.project ≥ 1` |
| C3 | front 单测 + LSP 单测 + `node editor/vscode/test-webview.js` + `python3 scripts/audit-wire-fields.py` | 三层各一条；wire 字段存在；可见文本有类型/定义 |
| C4 | `node editor/vscode/test-project-first-screen.js --check` | 编译中三问答得上（exit 0） |
| E1 | 三条**哨兵行** grep（`axiom Classical.em : (p : Prop)` 等，今天各 1 命中） | 在 `crates/**/*.rs` 里 **0 命中** |
| E1 | `cargo test -p sokonanoda-front --test prelude_mirror`（方向翻转） | 三段文件拼接 == 视图逐字节 + Rust 里无 prelude 文本行 |
| E1 | `sed -n '20p' prelude/Prelude.sokonanoda` | 仍是 `def Or.elim …`（课程文档 `OPEN-ITEMS.md:152` 的引用不漂） |
| E1 | **反向验证**：改 `prelude/L1.sokonanoda` 一行 ⇒ 重编 | **行为必须变**（今天只让镜子红、行为不变） |
| E2 | spike：源化 `Nat`/`Bool` 后跑全课程 `--json` | 与手搓版**逐字节相同** + `prelude_def_span("Nat")` 有真 span |
| E4 | 撤 `elab.rs:3312-3326` 排除分支后跑全课程影子档 | 排除命中计数 = 0 · `same>0 && diff=0` · `--json` 逐字节 |
| E3 | 覆盖 + 换内容 + 畸形覆盖三连 | 行为变 · 缓存 **miss** · **诊断而非 panic** · 不设覆盖时逐字节不变 |
| 全部 | `scripts/dev-verify.sh`（0.3s）· 收尾 `scripts/soko gate --fast` | 绿 |

**反向验证是硬要求**：B1/B2/C1/C2/C3 各要一条"撤掉修复 ⇒ 判据必须红"的证据
（`scripts/expect-red.sh` 的形状，见 `docs/gaps/repro/`）。

---

## 5. 边界与红线（下一轮开工前先读）

1. **判定正确性不变**（内核红线）：接受/拒绝不变 · 事件计数不变 · golden 与 `--json`
   逐字节不变。A2a/A2b/A4a/A4b/B3 是热路径，按发版大节点的三层回归走。
2. **判据不许用绝对毫秒**（`AGENTS.md` 判据纪律②，已三次同形事故）：
   一律结构计数或**同一次运行内的比值**；绝对毫秒只兜数量级（20×+）。
3. **不许放宽既有断言**：改断言值只能**往紧的方向**（例：`edit == 1` 已是紧的；
   `check-recompile-factor.py` 的上界只许减）。
4. **不许破 `--json` 的确定性**（并行只并行编译，输出按 `files` 顺序重放）。
5. **人看的输出只能走 stderr**（stdout 是机器契约）。
6. **设计先行**：B2/C3 改协议语义/形状 ⇒ 先写设计（可并入本文或对应 `docs/design/*`），
   协议文档同轮改；`REPORT_SHAPE_VERSION` 该 bump 就 bump。
7. **文档预算/过期**：新文件要登记（§7）；**不许靠抬上限达标**；活文档只许减不许增。
8. **本文件自身的占用**：`docs/notes/PLAN-cli-editor-perf.md` 是**下一轮的执行纲领**，
   执行完即归档（过期日见 §7）。

---

## 6. 未取证 / 不确定（**不许当结论用**）

1. **本轮没测墙钟**（除 §P6 引用的历史读数）。所有"贵/便宜"都是结构计数或
   **引用** `incremental-environment.md`/`declaration-incremental.md` 的历史实测
   （那是别的构建上的读数，跨构建**不可转移**）。P7 子调研的 `704ms`/`11234ms`
   取自 `target/debug`，只兜数量级。
2. **A2a 的因果链是"强假设"（未做抬闸实验）**：探针读数（5 个 goal 全含 `f ''`/`f ⁻¹'`）
   + 读码（`Set.image {α β}` ⇒ 前导隐式 2 ⇒ `mentions_multi_implicit` ⇒ `ByMode::Off`）
   两条一致，但**"抬闸后 shadow 全同"没有实测过**（要改代码）⇒
   **A2a 的第一步就是补 shadow 覆盖并看读数**，不是直接抬闸。
3. **A4b 的收益无读数**：telescope 解析次数**今天没有任何计数器**
   （§2 A4b 的"几次"是调用点推断，不是实测）⇒ 先建计数器再谈优化。
4. **A3 的三个读数还没量**（§2 A3 的决策门）⇒ "模块产物值不值得做"**今天没有答案**；
   而且刀 2 需要**新机制**（内核无 `ExportFile → EnvBuilder` 入口）。
5. **C1 的节流阈值**（150ms？250ms？）没定：要么 D2 的对照表给依据，要么按
   "≥1 条/秒 + 逐文件"实现（结构判据只认行数，不认阈值）。
6. **P1 的"24"** 是从 `step = total/10` 推的（N≈240），**没有**拿真课程冷编数过行数
   —— 下一轮 C1 的判据用**合成夹具**（N=120）就够，不必冷编整门课。
7. **B2 的"哪些测试会红"是读码推出、未实跑**（P4/P5 子调研只读）⇒
   开工时先跑一遍存基线（`query/tests.rs:252-274`、`lsp/src/tests/state.rs:11/118-130`）。
8. **`#print` 的消息 range 取 `tk` 还是 `id`** 未逐字确认（Lean `Print.lean:224-226`
   用 `withRef tk`）—— 进 C3 设计时一并定。
9. **`query state` 的 `data` 信封**：本文的探针读数从 `{"data": {...}}` 取；
   `ok`/`error` 信封（G-17）在 `query` 通道上另有约定 —— 写测试时别拿错层。
10. **探针身份有两处不一致**（跨轮比较读数前必须重取）：① MCP 工具链 marker 是
    `0.83.0` 而 CLI 是 `0.85.2`；② `target/debug/sokonanoda-lsp` 与缓存的 LSP
    marker 都是 `0.73.0` ⇒ **要跑 e2e 判据前先 `scripts/soko update`**。
11. **P8/E2 的"Nat/Bool 能不能源化"没有答案**：L1 的 `And`/`Or` 是源文本归纳块（可源化的
    依据），但 Nat/Bool **最先装**且带 `PRELUDE_NEVER_YIELDS`/`prelude_shape` 让位逻辑 ⇒
    必须一次 spike；**不许**在 spike 前承诺"全部显式"。
12. **P8/E3（运行时覆盖）的成本没估**：缓存键折内容哈希、畸形覆盖的诊断路径、
    `--bare` 交互三处都要动 ⇒ 它是**决策门**，不是既定任务。
13. **两处文档/注释已过期**（顺手修，不新建文件）：
    `docs/design/module-artifacts.md §6` 说 `EnvBuilder` 无 `Clone`（今天 `builder.rs:37` 有）；
    `crates/front/src/project/module_plan.rs` 引用的 `docs/design/module-batch.md` **已不存在**。

---

## 7. 登记（本文档自身的文档预算 / 过期）

本文是**新增文件**，按仓库既有机制要走两步登记（都是**文档基础设施**，不是抬上限达标）：

1. **文档预算** `scripts/docs-budget.json`：`onboarding.layer_max_lines.L2`
   按 `_comment` 的**例外①（新增文件到该层 ⇒ 基线跟着走一次）**更新为计入本文后的实测值，
   并在 `_comment` 追加一条（写明：新增 `docs/notes/PLAN-cli-editor-perf.md` =
   下一轮 CLI/编辑器体验 + front 缓存线的**唯一执行纲领**，登记日期与理由）。
   ⚠ **追加 P8（用户 2026-10-08 点名）后本文行数增长** ⇒ 按同一份 `_comment` 的先例
   （`by-prefix-reuse.md` 153→183、`declaration-incremental.md` 175→309）**手改本文件
   把上限跟到新实测值，并在 `_comment` 写明是"用户点名的必需修订"** ——
   不是抬上限达标；该层此后仍**只许减不许增**。
2. **过期登记** `scripts/docs-expiry.json`：
   `python3 scripts/docs-expiry-check.py --renew docs/notes/PLAN-cli-editor-perf.md --date 2027-01-06 --tier process --reason "…"`
   （plan 属 process 档；执行完就该归档或删除 ⇒ 90 天，不是 365）。

验收：`python3 scripts/docs-lint.py` 与 `python3 scripts/docs-expiry-check.py --check`
在本文登记后**必须只剩既有红**（本文不许**新增**判红项）。

---

## 8. 开工 profiling（2026-10-08 · as-built · 端到端真实做题路径）

> **构建身份**：`HEAD c8ed4232` · `target/release/sokonanoda-lsp` sha256 `d699e1e489d6`（自述 `0.85.2`，
> 与 `crates/` 源码同源 —— 16:00 之后只有 docs 提交）· 夹具 `courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda`。
> **方法**：真 stdio LSP 客户端（`/tmp/prof/lsp_profile.py`，**不入库**）走「`didOpen` → `didChange` → 等
> `publishDiagnostics` → `soko/stateAt`」，读 `SOKO_LSP_TRACE=1` 的**差量结构计数**；墙钟只作定位
> （跨机不可转移 ⇒ 不进判据）。所有读数**同二进制、同夹具、同序列**。

### 8.1 两条路对照（同一二进制 · 同一按键）

| 开档路 | `didOpen` | **首次按键**（改 `sorry` 行） | 第二刀 | 改陈述（改名） |
|---|---|---|---|---|
| **产物命中**（磁盘 `.sokonanoda/compiled/`） | **19ms**（`modules=0`） | **1233ms** · `modules=5 by=87 prefix=16` | 341ms · `modules=1 by=63` | 888ms · `modules=1 by=91 prefix=5` |
| **冷编**（`SOKONANODA_NO_PROJECT_ARTIFACTS=1`） | 1474ms（`modules=5 by=118 prefix=16`） | **321ms** · `modules=1 by=63 prefix=0` | 341ms · 同上 | 766ms · `modules=1 by=91 prefix=5` |

**读法**：冷编那一臂**在开档时**把库层检查点（G-29）与 `judge` 缓存都喂热了 ⇒ 首次按键只要 **321ms**；
产物命中那一臂开档**一趟都没跑** ⇒ **首次按键把整条库闭包重编了一遍**（`modules=5`）。

### 8.2 N1（**新发现 · 最大单笔 · 已立 A5**）：产物命中不预热库层检查点

* **根因（读码 + 实测）**：`crates/lsp/src/lib.rs:237-258` 命中磁盘产物就
  `set_cached_entry(...) → return` —— **任何 pass 都不跑**。而 `set_cached_entry`
  （`crates/front/src/query/mod.rs:757-763`）只预热 **S2 的 `entry_cache`**（2026-10-01 修，
  当时把首次按键 2977ms → 1233ms ✓），**没有**预热 **G-29 的线程局部 `LIB_CHECKPOINT`**
  （那个机制 **2026-10-08 才落地**，比这次预热晚一周）⇒ 首次按键走
  `with_project_session_reusing` 的 **miss 分支**，整条库闭包重编。
* **⇒ 新增任务 A5（纯调度 · 不进判定路径）**：产物命中、**且诊断已发出之后**，
  在后台（同一 worker，可被后续按键任务自然排在后面）跑**库层趟**把检查点喂热。
  **为什么不会更坏**：库层趟是"首次按键本来就必须做的那部分功"的**子集** ——
  按键先到就由按键自己的编译建检查点（总功不变），按键后到就省下它 ✓。
  **形状**：`project/mod.rs` 增 `warm_library_checkpoint(plan, options)`（复用 `units_for_modules`
  + `session::warm_library`，只跑 `run_library_pass`、不跑入口趟、不产报告、不发诊断）。
* **判据（结构计数 · 逐条可执行）**：① 真 LSP：产物命中开档后**首次按键**的 `LSP_TRACE`
  ⇒ **`modules=` 5 → 1**、`by=` 87 → ≈63、墙钟 1233 → ≤400ms（只兜数量级）；
  ② **开档本身不许变慢**：产物命中开档仍 `modules=0` 且 ≤30ms，且**诊断先于预热**
  （结构：`publishDiagnostics` 的时刻 < 预热 trace 行的时刻）；
  ③ **一次开档只发一次诊断**（预热**不发**诊断 ⇒ `publish=` 不增）；
  ④ **反向验证**：逃生门 `SOKO_NO_LIB_WARMUP=1` ⇒ 首次按键 `modules=` **回到 5**；
  ⑤ 全课程 `--json` 逐字节不变（预热不产报告 ⇒ 本该零影响）；
  ⑥ **不更坏**：`didOpen` 后**立刻** `didChange`（预热尚未跑完）⇒ 结构计数 ≤ 今天
  （库层功相等，只多一次调度）。
* **A5b（本次不做 · 读数决策）**：连入口趟一起预热（首次按键 `prefix=` 16 → 0，≈ 再省 300ms）——
  但入口趟**不是**首次按键功的子集（改末条声明时按键只需重查一小段）⇒ 会白烧背景 CPU；
  等 A5 落地后的读数再定。

### 8.3 N2：目标更新（`soko/stateAt`）**不是瓶颈** ✓

`stateAt` 实测 **0.7–1.2ms**（连打 5 次，最坏 1.2ms），`soko/goals` 5.8ms、`soko/project` 1.2ms、
`soko/hints` 0.1ms。⇒ **B2 是语义正确性修复（对齐 Lean），零性能收益**；P4 的"感觉卡"不在这一层。
（与规划 §0.2 #6「warm 按键 0.1–0.7ms」一致 ✓。）

### 8.4 N3：A2a 的天花板**下修**到 ~15%（不是"一半"）

* 同二进制反向对照复现了规划 §4 的读数：`SOKO_JUDGE_INPLACE=off` ⇒ 改陈述一刀
  **`prefix=387` · 10803ms**（默认档 `prefix=5` · 888ms）✓ ⇒ 「就地路本来就在扛大头」成立 ✓。
* 但**边际成本**要按"趟"折算：冷编臂 `prefix=16`（1474ms）vs `prefix=0`（321ms）同源同夹具
  ⇒ **一次前缀趟 ≈ 20ms**（387 趟 ≈ 7.7s 也自洽）⇒ 改陈述那刀的 5 趟 ≈ **100ms / 888ms ≈ 11–15%**，
  不是规划 §2 A2a 写的"≈ 一半成本"（那是把**整条 judge 路**的功算到了 5 趟头上）。
* **结论**：A2a 仍做（纯接线、且能去掉 O(N²) 的放大源），但**排在 A5 之后**，判据里的
  墙钟期望改为"按键成本降 **≥10%**（同二进制自比）"而不是"减半"。

### 8.5 N4：改库文件 ⇒ 依赖它的入口**整条库闭包重编**（A3 决策门的读数③）

改 `lib/Image.sokonanoda` 一行 ⇒ 依赖它的 unit08 被扇出重编 **976ms · `modules=5 by=104 prefix=5`**
（库层键 = **整条闭包摘要** ⇒ 改一个模块 ⇒ 全部库模块重编）。这正是规划 §2 A3「跨入口/跨进程的
库层重复」决策门要量的数 —— **本次读数：库层 4 个模块、≈600ms/次**。A3 做不做仍按 §2 A3 的三读数定。

### 8.6 N5：产物库的**单槽索引**会把编辑器预热好的产物挤掉（gotcha · 不立任务）

`courses/set-theory/.sokonanoda/` 实测 **1.38 GB / 276 条**（≈5MB/入口文件，`update_index` 一条目一入口，
**有界设计** ✓ 无孤儿 ✓）。但索引是**每个入口一条、后写覆盖前写**（`cache.rs:update_index` ①替换）
⇒ 任何**带不同开关**的编译（例如本轮的 `SOKO_JUDGE_INPLACE=off` A/B、`docs/gaps/repro/*.sh`）
会**替换**该入口的产物 ⇒ **下一次编辑器开档退化成冷开（1474ms）**。
⇒ 记一条：**"开档偶发慢"先查 `meta.json` 的 `entries` 是不是被别的开关档挤过**（`--clean` 或重跑一次默认档即恢复）。
**不立任务**（这是"有界索引"的既定代价，改它要动设计）。

### 8.7 与 P1–P8 的关系（**哪些是真瓶颈**）

| 现象 | profiling 判决 |
|---|---|
| P1/P2（build 汇报 / 无参 clean） | 与按键热路径**无关** ⇒ C1/C2 照做（UX 面，非性能） |
| P3（失败块） | **未推翻**：成本面（`1+N` 走查）与显示面都在规划里 ✓ |
| P4（目标语义） | **不是性能问题**（`stateAt` 0.7ms）⇒ B2 只为对齐 Lean |
| P5（`#check`/`#print`） | 与热路径无关 ⇒ C3 照做 |
| P6（打开/写/改/提交丝滑） | **真瓶颈 = 首次按键**（N1/A5，1233ms）；冷开 1474ms 是次要（A3/A5b） |
| P7（编译链路要快） | 真瓶颈排序：**库闭包重编（A5）> 入口趟（A2b/A4a/A4b）> 前缀 5 趟（A2a，~15%）**；⚠ **§8.8 又插进一笔更大的**：`TcCache` 重建（**A6，已落地 −40%**） |

### 8.8 N6（**并行会话 · 本会话实测 · 已落地 A6**）：`TcCache` 每次 `with_tc` 都重建 ⇒ 4 MiB 清零 × 6876 次/按键

> **构建身份**：`HEAD d97b3148` · `target/release/sokonanoda-lsp`（18:00 构建）· 夹具同 §8。
> **方法**：macOS `sample`（1ms，26s / 40 刀）+ 新增结构计数 `tc=`（`LSP_TRACE`）。

* **现象**：`TcCache::new` 占一次按键编译样本 **63%**（9279 / 14710）—— 调用方：
  `finish_pass::{closure#0}` 52% · `infer_type_text_inplace` 20% · `TypeChecker::apply` 18% ·
  `collect_mvars` 8%。
* **根因（每次 `with_tc` 都新建一份预分配）**：① **`whnf_admit` 固定 4 MiB**（`1<<22`）——
  微基准（临时 `#[test]`，跑完即撤）**53.5 µs/次**，而整个 `TcCache::new` **61.8 µs/次** ⇒ **87%
  是这张表的清零** ✗；② 另 ~20 张 `with_capacity(4096/8192)` 哈希表 ≈ **8 µs/次**。
* **规模（新计数器 `tc=`）**：热按键 = **6876 次**构造（冷开 18194；CLI 冷编 unit08 = 17528 次 /
  52 趟 ≈ 337 次/趟）⇒ 4 MiB × 6876 ≈ **26 GB 清零/按键** ✗；两条独立证据自洽：6876 × 61.8 µs
  ≈ **425 ms** ↔ `sample` 的 63% × ~650 ms ≈ **410 ms** ✓。
* **T-K31 要重新解读**（`docs/perf/ledger.jsonl`）：那次"池化无收益"的真因**不是**"mmap 惰性
  零页"（`sample` 明确有 `__bzero`）✗，而是**池化每次取出都 `fill(0)`** ⇒ memset 照付 ✓
  ⇒ 出路是**把表变小**（清零量 ÷64），不是池化（已同步改 `docs/PERF.md`/`architecture.md` ✓）。
* **已落地（A6 · 纯内核 · 不动判定）**：`WHNF_ADMIT_BITS = 16`（4 MiB → **64 KiB**），
  且把索引移位**由位数推出**（`>> (64 - WHNF_ADMIT_BITS)`）。
  ⚠ **必须同时改移位**：老代码 `>> 42` 与 `1 << 22` 手抄两处 ⇒ 只改表长会**越界 panic** ✗，
  而 panic 被 `quiet_catch` 吞掉 ⇒ 量出的"变快"是假象 ✗（本轮先踩了一次，单测当场判红 ✓）。
  这张表只是"同一 digest 见过 ≥2 次才准进 `whnf_store`"的**启发式计数器** ⇒
  表小 = 碰撞多 = **假准入变多**（多进几条缓存），**不改判定** ✓。
* **读数（同二进制自比 · 结构计数优先）**：

  | | 基线（4 MiB） | A6（64 KiB） |
  |---|---|---|
  | 热按键墙钟（5 刀） | 637–772 ms | **395–434 ms**（≈ **−40%**） |
  | 冷开 | 1496–1502 ms | **821 ms**（≈ **−45%**） |
  | `tc=`（构造次数）· `modules/by/infer/prefix` | 6876 · 1/91/5/5 | **6876**（只变单价）· **逐字相同** ✓ |
  | kernel 单测 · 226 文件 `--json` 对拍 | — | **63/63 绿** ✓ · **逐字节 0 行不同** ✓ |

* **第二笔（A6b）已落地** ⇒ 读数与判据见 §8.9 ✓。

### 8.9 A6 之后的复测（同一方法 · 22s 窗口 / 40 刀）——**下一笔在哪**

> 构建身份：`HEAD fe5f4ce9` 之后（A6 = 64 KiB 表）· 同夹具 unit08 改陈述。
| 项 | A6 前样本（占编译） | A6 后样本（占编译） | 判读 |
|---|---|---|---|
| `TcCache::new` | 9279（**63%**） | **2646（22.9%）** → A6b 后 **467（4.0%）** | A6 砍 3.5× ✓；**A6b（~20 张表改惰性）再砍 5.7× ✓ 已落地** |
| judge 合成前缀重编 | 5740（39%） | **4763（41%）** | **现在是最大一笔**：`judge_type_of`（elaborate 里 `set_literal_prefix_args` 触发）**3046（26.3%）** + 嵌套的 `judge_render_type`（by 引擎探针，A2a 的正身）**1717（14.8%）** |
| **judge 缓存键的 SipHash** | —（未单独看） | **2607（22.5%）** ✗ | **新发现**：`judge_infer_cache_key` 每次调用都对**整份前缀文本**跑 SipHash（`canonical_prefix_cached` → `judge_cache_key(&[src])`，前缀 ~40 KB × 每次两遍）⇒ 与 `TcCache` 同级 ✗ |

⇒ **新增 A7（决策门 · 先建读数）**：judge 缓存键的 **O(前缀) 哈希**（判据：新增"前缀哈希字节数"
读数；**红线**：换弱哈希 = 碰撞 ⇒ 静默用旧答案 ⇒ 首选"避免重复哈希同一份前缀"）。
**A6b 落地**（`604513e5`）：`--json` 226 文件逐字节 · kernel 63/63+21 全绿 ✓（会话路不在热路径 ✓）。

### 8.12 第三次复测（A2a/A6/A6b 之后）+ **A4b 决策：按数据不做**（2026-10-08）

同一方法（`sample` 20s / 40 刀 · unit08 · 同屏 `by=81 prefix=0 telescope=4443`）：最大一笔 =
**`infer_type_text_inplace` 17.7%**（其中 `set_literal_prefix_args → judge_type_of` 的**合成趟**
8.4%）· `run_pass_with` 占编译 87% —— 与 §8.9 相比 **`TcCache` 已不在榜上**（A6/A6b ✓），
**judge 合成趟仍是主线**（A2b 的正身）。**A4b**：`telescope` 4443 次/按键但只占样本 **3.2%**
⇒ 天花板 ~3%、memo 只省解析那一半，键还要含"记法表纪元"（否则 = B4 错键红线 ✗）⇒ **不做**。

### 8.11 A3 决策门读数（2026-10-08 · 只读 · **结论：值得做，但它是新机制 ⇒ 排在当前批次之后**）

同一 LSP 进程依次 `didOpen`、`SOKONANODA_NO_PROJECT_ARTIFACTS=1`（真编）读 `modules=`：**同一闭包**
（unit08 画布 → 它的解答）⇒ **5 → 1** ✓；**不同闭包**（unit08 → unit09 → unit10）⇒ **5 → 8 → 8** ✗
（每换一个闭包就把**整条库层**重编，≈1.2–1.8s/单元）。⇒ `lib_key` = 整条闭包摘要，而课程里各单元的
库模块集合本就不同 ⇒ **跨入口的库层重复仍是主要成本** ⇒ **刀 2 值得做**（per-module Merkle +
模块产物），但内核**没有 `ExportFile → EnvBuilder` 入口** ⇒ 是新机制、不是接线。

---

## 附录 A · 本轮实测复现命令（带构建身份）

**构建身份**：`HEAD 768c5c72` · `target/release/sokonanoda` = `0.85.2`（2026-10-08 16:00）·
工作树 9 个发版会话改动（与本文读数无关）。

```bash
# P1：人看的那条是"每 10%"（100 文件 ⇒ 10 条进度）
mkdir -p /tmp/p1 && cd /tmp/p1 && for i in $(seq 1 100); do printf 'theorem t%s : Prop := Prop\n' "$i" > "f$i.sokonanoda"; done
sokonanoda build 2>&1 | grep -c "file(s)"          # ⇒ 12（10 进度 + found + built）

# P2 残留：无参 rebuild 不清模块根 ⇒ 假重编
cd /tmp/p2b && sokonanoda build . && sokonanoda rebuild
# ⇒ built 2 file(s) — 1 hit, 1 compiled, 0 failed   ✗（项目产物仍在 .sokonanoda/compiled/）

# P3：最后一条 tactic 错 ⇒ 整份声明退回题面（step -1 / total 0）
cd /tmp/p3
sokonanoda query state --file ok.sokonanoda  --line 3 --col 4   # step 0  total 3  goals [P, Q]
sokonanoda query state --file ok.sokonanoda  --line 4 --col 4   # step 1  total 3  goals [Q]
sokonanoda query state --file bad.sokonanoda --line 3 --col 4   # step -1 total 0  goals [P ∧ Q] ✗
sokonanoda query state --file bad.sokonanoda --line 4 --col 4   # step -1 total 0  goals [P ∧ Q] ✗

# P4：光标"在 tactic 内" ⇒ 进入它之前（比 Lean 差一步）
#     ok 的 L3 = 第 2 条 tactic `exact hp`，今天答 step 0（= 第 1 条之后），应为 step 1

# P5：真相层两层都有，LSP 只把 #check 做成 inlay hint、#print 没有出口
printf 'def myid (x : Nat) : Nat := x\n#check myid\n#print myid\n' > /tmp/p5/t.sokonanoda
sokonanoda --json /tmp/p5/t.sokonanoda | head -3   # ⇒ expr.typed + decl.printed 都在

# P3 成本面：失败块的严格趟从头重放（结构计数，SOKO_JUDGE_STATS=1）
#   3 条 tactic 的夹具：全过 calls=1 pairs=2 · 第 1 条错 calls=2 · 第 3 条错 calls=3 pairs=4

# P7：一次按键的 5 次前缀重编 = G-71 闸挡住的 `needs_explicit` 探针（P7 子调研实测）
#   unit08-images-preimages 改陈述：LSP_TRACE … modules=1 by=91 infer=5/90 prefix=5（702ms）
#   PREFIX_MISS 5 条全是 `f ⁻¹' …` / `f '' …`（前导隐式 2 的 def）
#   反向对照（同一二进制）：SOKO_JUDGE_INPLACE=off ⇒ prefix=387 / 11234ms
```

## 附录 B · Lean 4 对照（源码行号，本机 `~/Documents/lean/lean4` @ `d0493e4c1e`）

| 主题 | Lean 4 怎么做 | 我们怎么做 | 决定 |
|---|---|---|---|
| 光标处目标语义（P4） | `useAfter := hoverPos > pos && !cs.any (hasNestedTactic pos tailPos)`；取 `ti.goalsAfter`/`goalsBefore`（`Server/InfoUtils.lean:448-480` · `Server/FileWorker/RequestHandling.lean:192-205`） | `start <= cursor < end ⇒ step i-1`（"进入"） | **改成 Lean 语义**（B2） |
| 一次失败不抹掉历史 | 每条 `TacticInfo` 同时带 `goalsBefore`/`goalsAfter` | 失败即丢 `steps`（`by.rs:1302`） | **保留前缀步进**（B1） |
| 命令状态复用 | 每条命令的完整 `Command.State`（含 env）可复用；首次语法不等处才取消尾巴（`Language/Lean.lean:558-611`） | 命令级信任前缀 + 库层检查点（G-29 ✓）；**入口趟仍整份 elaborate** | A2b/A4b 的正身 |
| 查询不合成文档 | `#check` 直接在**活环境**上跑并 `logInfo`；`Snapshot.runCommandElabM` | `judge` 合成 `#check` 文档 ⇒ `compile_fol_with`（G-31 已砍一半；**A2a 再砍掉 G-71 闸挡住的那 5 次**） | A2a/A2b；P5 的渲染走"命令输出"面 |
| CLI 与服务器分路 | CLI 主动丢快照元数据（`cmdlineSnapshots := true`），增量只在服务器 | 只有一条冷路 + 线程局部检查点 | 不照抄分路；沿用"检查点 + 单 worker" |

> **记法**：`Lean4: n/a` 只适用于**纯构建/缓存/CLI 输出**类任务（C1/C2/D2）；
> 语义/判定类（A2a/A2b/A4a/A4b/B1/B2/B3）开工前必须有上面这样的对照。
