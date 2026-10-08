# 规划：对齐 Lean 4 交互性能机制 —— 0.85.x 改造路径

> **性质**：只读调研 + 规划（**零代码改动**：不碰 `crates/`、`STATUS.md`、
> `docs/gaps/ledger.jsonl`、`docs/notes/HANDOFF-kernel.md`）。本文只写
> **现象 → 根因（`文件:行`）→ 分阶段任务 → 判据 → 批次 + 工作量**。
>
> **上游**：`docs/notes/perf-lean4-interactive-incremental.md`（2026-10-07，Lean 4 四层缓存）。
> ⚠ 那份的基线是 **`0.82.0` / `567caf0c`**，其中 **§4.0/§4.1（"把库层检查点交给 `QueryDoc`"）
> 已被 0.83.0–0.85.2 的提交推翻**（`ArenaRef: !Send` ⇒ 持有者改成线程局部，见
> `docs/design/incremental-environment.md` §33.1②）⇒ **本文以当前树为准**，
> perf-lean4 只当 **Lean 侧机制引用**（它的 Lean 行号本轮抽检过，仍然有效）。
>
> **并行线**：`docs/notes/PLAN-cli-editor-perf.md`（CLI/编辑器体验 + A 阶段缓存，1128 行）。
> 两份共享同一批决策门：**那份**负责"用户可见的 UX 与已量出的瓶颈"，
> **本文**负责"Lean 4 四主题的机制对齐、结构前置件与批次日程"。
> 重叠处逐条标了归属（§6.3），**冲突以本文的"前置件顺序"为准**（理由给在每条的依赖栏）。
>
> **构建身份**（探针纪律）：读码基线 = **`HEAD 217877ab` · `Cargo.toml 0.85.2`**；
> 调研期间并行线提交了 3 笔（`217877ab` → `b7307754` → `1ec416da` → **`375c7836`**），
> 触及 `docs/design/incremental-environment.md`（新增**决策门记录**，§2.4/§8 已引用）、
> `docs/notes/{perf-lean4-interactive-incremental,PLAN-cli-editor-perf}.md`、
> `crates/front/src/compile/{cache,goals,mod,prelude}.rs`、`crates/front/src/display.rs`、
> `crates/cli/src/main.rs`（E1/E3 prelude 线）—— **没有一笔触及本文四个主题的分析路径**
> （`project/{mod,session}.rs` · `query/mod.rs` · `judge.rs` · `by.rs` · `compile/{elab,check/*}.rs` ·
> `crates/kernel/src/{env,builder}.rs`）⇒ **本文行号对 `375c7836` 仍逐条成立**
> （子调研在漂移后重新 grep 核对过）。
> 另有**未提交**改动（`docs/perf/ledger.jsonl`、`scripts/perf-check.sh`、新 `scripts/check-json-identity.py`，
> 属别的写者）⇒ **不与任何墙钟读数并排比较**（`AGENTS.md` 判据纪律②：绝对毫秒只兜数量级，
> 判据一律用**结构计数**或**同 run 比值**）。

---

## 0. 一页结论

### 0.1 四主题：现象 → 根因 → 结论

| 主题 | 现象（今天的读数在哪） | 根因（`文件:行`） | 结论（该做什么） |
|---|---|---|---|
| **① 产物化** | 换一个闭包（unit08→unit09）就把**整条库层**重编，≈1.2–1.8s/单元（`PLAN-cli-editor-perf` §8.11）；磁盘产物键 = **整条闭包**摘要 ⇒ 改一个字节必 miss，只服务"开档" | `import` 的语义 = 依赖**源文本拼进闭包前缀**、每次重新 elaborate（`crates/front/src/compile/check/mod.rs:1052`/`:1066`/`:1074`）；每趟 pass **新建 arena + 新建 `EnvBuilder`**（同文件 `:977`/`:985`/`:1013`）；磁盘产物只存 **C（报告/事件）**，不存 **A（内核环境）**（`docs/design/module-artifacts.md:19-31`）；内核**没有** `ExportFile → EnvBuilder` 入口（`crates/front/tests/a3_cross_entry_module_reuse.rs:14`） | 先做**接线级**的"进程内 per-module 检查点"（键 `module_keys()` 已在手，`crates/front/src/project/mod.rs:204`）；磁盘产物（刀 2）是**新机制**，要先有内核 writer + 装载口 + 安全模型，且 **mmap 只能覆盖索引/表，覆盖不了项图** |
| **② 服务器热路** | 库层已跨调用复用（G-29 ✓，`edit.modules=1 < cold=5`）；**入口趟仍从零 elaborate** —— 改最后一条声明也要把前面每条命令的环境推进重做一遍 | 检查点粒度 = **库层**（`crates/front/src/project/session.rs:67-92`），**没有命令级环境快照**；trust 只跳**内核重查**、不跳 elaborate（`docs/design/incremental-environment.md:978-983`）；**结构前提缺失**：`DeclarMap = FxIndexMap`（`crates/kernel/src/env.rs:256`）**非持久化**，`EnvBuilder: Clone` 是 O(#decls)（`crates/kernel/src/builder.rs:37-38`）⇒ 命令级快照在数据结构上是 O(N²) | ② 的**前置件 = 内核持久化声明表**（对齐 Lean `SMap`/`PHashMap`）；在此之前只做"边界检查点"（库层/模块）；**单文件内子部分复用（`withNarrowedTacticReuse` 类比）排最后**，且必须先重开 B4 决策门（§3.4） |
| **③ 活环境查询** | `run_pass_with` 占编译 **87%**（sample，`PLAN-cli-editor-perf` §8.12）；合成趟仍是主线（`judge_type_of` 8.4%）；A2a 之后 `judge_infer` 的 `prefix=0` ✓ | judge 的三条查询都把"问一句"变成"**合成一份文档 + 跑一趟 pass**"：`judge_infer`（`crates/front/src/judge.rs:2717`）、`judge_type_of`（`:2263`）、`judge_pairs`（`:1389`）→ `run_synthesized_incremental`（`:1097`）→ `run_incremental`（`check/mod.rs:654`）→ `run_pass`（`:977`，新建 arena+builder）；**所有权**阻塞：`run_pass_with` 按值收发 builder，而 judge 在 `elab_expr` 链里只有 `&mut`（`incremental-environment.md:978-983`） | 结构性修法只有两条：**(i) pass 借 builder** / **(ii) `EnvBuilder::fork()`**（`:986-989`）。⚠ 决策门"A2b 不做"关掉的只是 **`judge_infer` 那一半**（`prefix=0`）；`judge_type_of`/`judge_pairs` 那一半**没关**（§4.3 对账） |
| **④ CLI/LSP 分路** | 只有 LSP 有热路：线程局部检查点 + **单 worker** 编译 runtime（`crates/lsp/src/lib.rs:790-808`）+ 产物命中后的后台预热（A5，`crates/front/src/project/mod.rs:417`）；CLI 每次都冷编（`reuse_library=true` 只有 `crates/front/src/query/mod.rs:653`/`:662` 两处） | 分路是 **`ArenaRef: !Send` 逼出来的结果**（`session.rs:18-24`），不是设计；代价 = LSP 多文档编译**串行**、CLI 结构上拿不到进程内增量 | 把分路写成**契约**并当成 ①/② 的接口边界：**跨进程增量归产物，跨按键增量归检查点**；CLI 吃产物（依赖 ①），LSP 放开并发（依赖 ② 的持久化） |

### 0.2 与任务书/上游措辞的四处对账（**先看这节，避免照着过期句子做**）

1. **"import = 把依赖源文本拼进前缀、每次重编"** —— 对**冷开**与**入口趟**仍成立 ✓；
   但**同进程内**库层已由 G-68（按"库闭包签名"分组，`project/mod.rs:602`）与 G-29
   （跨调用检查点，`session.rs:281`）**只编一次**。⇒ ① 的剩余价值集中在
   **跨进程（冷开/重启）**与**跨闭包（换单元）**，不在"每次按键"。
2. **"检查点只在 `with_project_session` 的栈上、返回即丢、每次从 prelude 重装"** ——
   **库层已不成立**（线程局部 LRU 8 槽 + `Box::leak` arena + 上界，`session.rs:100`/`:109`/`:129`）；
   **"从 prelude 重装"也不成立**（prelude 只在库层趟装一次：`install_preludes = true`
   `session.rs:413-414`；入口趟 `false` `:541`，它克隆的是**含 prelude 的检查点**）
   ⇒ ② 的任务面只剩"**入口命令的环境推进**"，**不含** prelude。
3. **"judge 合成前缀/合成趟重跑（`run_pass_with` 占编译 87%）"** —— 87% 仍是当前读数；
   但 **`judge_infer` 的合成前缀已被 A2a 清零**（`prefix=0`）⇒ 剩下的是
   `judge_type_of` 与 `judge_pairs` 的合成趟（§4.3）。
4. **perf-lean4 §4.0/§4.1「把库层检查点交给 `QueryDoc`」** —— **已被推翻** ✗
   （`ArenaRef: !Send`；`QueryDoc` 必须 `Send + Sync`）⇒ **不要照它做**。

### 0.3 任务总表（细节与判据见各主题节）

| ID | 任务 | 规模 | 依赖 | 判据（结构计数优先） |
|---|---|---|---|---|
| **T1-A** | 进程内 **per-module 检查点**（`module_keys()` 接线 + 有界 LRU） | M | 无 | `a3_cross_entry_module_reuse` **3 → 2**（改断言值）· `session_reuse.rs:153` 去 `#[ignore]` 翻绿 |
| **T1-B** | **磁盘模块产物（刀 2）**：内核 writer + 装载口 + 完整性/安全 | XL | T1-A（表示定型） | 跨进程 `compiled` 计数下降 · 反例（改依赖必 miss）· 损坏产物当不存在 |
| **T2-B0** | 入口趟另两张派生表（记法表/定义 span 表）随检查点存活 + **先建计数器** | S | 无 | 冷开 = k、**第 2 刀起 = 0** · 反向验证 · `--json` 逐字节 |
| **T2-A** | **内核：声明表持久化/COW**（② 的结构前置件） | XL | 无（内核线） | 环境克隆的**每命令条目数** O(1) · 指针同一性单测 · 全语料 `--json` 逐字节 |
| **T2-B** | **入口趟命令级环境快照**（复用判据**沿用** `EntryCache`） | L | T2-A | 新增"elaborate 命令数"：改最后一条 **N → 1** |
| **T2-C** | 单文件 `Session` 接同一机制 | M | T2-B | `session.rs` 单测 + 既有 `CmdSnapshot` 断言不放松 |
| **T2-D** | **子部分复用**（`withNarrowedTacticReuse` 类比） | XL | T2-B/C + 重开 B4 门 | 见 §3.4（**当前不做**） |
| **T3-B1** | **纯接线**：三条仍走合成趟的判定点接就地（`cases` 被消去项 / `judge_render_type_explicit` / `level_hint_of` 记法形态） | M | 无（**今天可做**） | `prefix=` 下降 · 影子档 `diff=0` 且 `same>0` · 反向验证 |
| **T3-B2** | **架构件**：`run_pass_with` 借用变体 + judge 收 `&mut InplaceEnv`（消合成趟） | L | 内核线 WIP 让位 + **重开门** | 合成趟的 `passes`/`modules` → 0 · 影子档 `diff=0` · 全语料逐字节 |
| **T3-C** | `EnvBuilder::fork()` + `absorb()`（仅 (i) 被证否时） | XL | T3-B2 | 指针同一性两向判据 |
| **T4-A** | **分路契约化** + 一条守卫 | S | 无 | CLI 路径的线程局部泄漏恒 0 |
| **T4-B0** | CLI `build` 开 `reuse_library=true`（**先量后做**） | S | 无 | 同进程跨组复用的结构计数 > 0 |
| **T4-B** | CLI `build` 吃 per-module 产物 | L | T1-B | `build.summary.hit` · `check-recompile-factor.py` |
| **T4-C** | LSP 编译 runtime 放开 worker 数 | L | T2-A / T1-A | 并发编译结构计数 · 检查点命中数不降 |

**推荐顺序（一句话）**：**先接线（批次 A：T1-A/T2-B0/T3-B1/T4-A/T4-B0）→ 再消合成趟（T3-B2）
→ 再动内核（T2-A → T2-B/C）
→ 最后才是磁盘产物（T1-B）与子部分（T2-D）**。
理由：① 与 ③ 的**判据现成、收益立即可量**；② 的收益上限最大但要动内核数据结构
（风险与回归面完全不同的量级）；磁盘产物（T1-B）依赖 T2-A 把"产物表示"定型
（`docs/design/module-artifacts.md:118-121` 已经写死这个顺序："先刀 1、后刀 2"）。

---

## 1. Lean 4 的四层机制（本轮引用；行号已抽检）

**两侧身份**：Lean 侧 = 本机源码 `~/Documents/lean/lean4` @ `d0493e4c1e`；
本仓库侧 = 读码基线 `HEAD 217877ab`（`0.85.2`），**收尾复核于 `375c7836`**
（漂移说明见文首「构建身份」；四个主题的分析路径零改动）。
**路径约定**：Lean 侧相对 `~/Documents/lean/lean4/`；本仓库侧相对仓库根（`crates/…`）。

| # | 机制 | Lean 4（`文件:行`，原话要点） | 本仓库今天的对应物 | 差距 |
|---|---|---|---|---|
| 1 | **`.olean`** | `ModuleData{constNames, constants: Array ConstantInfo, entries}`（`src/Lean/Environment.lean:120-141`）；`importModules`（`:2317`）→ `readModuleDataParts`（`:1730`，**一次 mmap 一批**）；`finalizeImport` 把常量**直接插进表**、**不做任何内核调用**（`:2188-2296`） | 磁盘 JSON：**整闭包 digest 一条**，只存报告（`project/cache.rs:23`/`:207`/`:251`） | **A（内核环境）完全没存** |
| 2 | **持久化 `Environment`** | *"Environments are never destructively updated."*（`src/Lean/Environment.lean:216`）；`ConstMap := SMap Name ConstantInfo`（`:96`）= `map₁` HashMap（导入期独占）+ `map₂` **PHashMap**（`src/Lean/Data/SMap.lean:17-27`）；HAMT `branching = 2^5` / `maxDepth := 7`（`src/Lean/Data/PersistentHashMap.lean:40-43`） | `DeclarMap = FxIndexMap<NamePtr, Declar>`（`crates/kernel/src/env.rs:256`）—— **平坦、可变、克隆 O(n)** | **快照树的结构前提缺失**（② 的正身） |
| 3 | **快照树** | `Snapshot{stx, mpState, cmdState}`，`cmdState` 含 `env`（`Server/Snapshots.lean:28-39`）；处理器由 `mkIncrementalProcessor` 建一次并把旧 `InitialSnapshot` 交给下一次 `process`（`Language/Basic.lean:401-407`） | 单文件 `Session` 有**命令级快照**（`crates/front/src/session.rs:82`），但快照里只有**结果**（`DeclState`/events/`signature`），**没有环境** | 有"快照树"的**骨架**，没有"环境复用"的**载荷** |
| 4 | **语法等价复用** | `unchanged` 整条旧快照接上（`Language/Lean.lean:558-578`）；两级快路：完全免解析（`:580-585`）、只解析不 elaborate（`:593-604`）；**第一处语法不等即取消整条旧尾巴**（`:605-611`） | 入口趟：命令**文本切片**相等 + 起点（`query/mod.rs:186`）+ 依赖指纹（`:142`）⇒ **只跳内核重查**；单文件：`DeclKey{text,start}` + `first_diff`（`session.rs:74-78`/`:217-221`） | 判据的**形态接近**（文本 vs 语法树），但**跳掉的东西不同** |
| 5 | **CLI / 服务器分路** | CLI 主动丢快照元数据：`internal.cmdlineSnapshots.setIfNotSet opts true`（`Elab/Frontend.lean:151`；选项文档 `CoreM.lean:51-56`——"reduce information stored in snapshots to the minimum"）⇒ 增量**只活在服务器** | 只有一条冷路 + 线程局部检查点（LSP 独享）；CLI `reuse_library=false` | 分路是**结果**不是设计（§5.1） |
| 6 | **命令内子部分** | `withNarrowedTacticReuse`（`Elab/Term/TermElabM.lean:455-472`）：把 tactic 语法切成 outer/inner，outer 用 `eqWithInfoAndTraceReuse` 比，相同 ⇒ 旧快照 stx **收窄到 inner**；姊妹件 `withNarrowedArgTacticReuse`（`:474-485`） | `by` 块是**扁平** tactic 列表，逐 tactic 只有**目标快照**（`by_steps`），**没有 elaborate 快照** | 见 §3.4（**结论：当前不做**，先做前置件） |

### 1.1 一条最容易被忽略的结构事实（**它决定 ② 的顺序**）

Lean 自己的设计注释把两件事分开说（`src/Lean/Language/Lean.lean:83-92`，照抄要点）：

> *"Because of Lean's use of **persistent data structures**, incremental reuse of fully elaborated
> commands is easy because we can simply snapshot the entire state after each command … However,
> incrementality **within** elaboration of a single command such as between tactic steps is much
> harder because the existing control flow does not allow us to simply return from those points …
> we exchange the need for continuations with some limited mutability: by allocating an `IO.Promise`
> 'cell' …"*

⇒ **命令级复用**靠"持久化结构 + 快照整个状态"；**命令内（tactic 步）复用**还要额外一套
`IO.Promise` + `withNarrowedTacticReuse` 的机制。本仓库**两样都没有**：声明表不是持久化的
（`env.rs:256`），`by` 块也没有步级快照（只有目标快照）。
**所以 ② 的正确顺序是"先表、后快照、最后步级"**；反过来做（先做步级）会在一个
O(n²) 的数据结构上叠 O(n²) 的判据（B4 决策门已经因为"判据不便宜"关过一次，
`incremental-environment.md:1164-1171`）。

---

## 2. 主题① 产物化（`.olean` 等价物）

### 2.1 现象（三条，全部可判据）

| # | 现象 | 今天的读数 / 判据在哪 |
|---|---|---|
| P1-1 | **换一个闭包 = 整条库层重编**：unit08 → unit09 → unit10 依次打开，`modules=` 读数 **5 → 8 → 8**（≈1.2–1.8s/单元） | `PLAN-cli-editor-perf` §8.11（LSP 真进程 · `SOKONANODA_NO_PROJECT_ARTIFACTS=1`） |
| P1-2 | **改一个字节必 miss**：磁盘产物键 = 整条闭包摘要（每个模块的 `src` 都进 mix） | `crates/front/src/project/mod.rs:157-191`（`digest`）；`project/cache.rs:207`/`:251`（读/写） |
| P1-3 | **per-module 键已实现但零生产调用** | `project/mod.rs:204`（`module_keys`）；唯一调用点是 `project/tests.rs:529`/`:611`；`crates/front/tests/a3_cross_entry_module_reuse.rs` 是它的读数前置 |

### 2.2 根因（逐条给 `文件:行`）

1. **`import` 的语义 = 源文本拼接**：闭包编译把每个依赖模块的源文本（去掉 `import` 行）
   累加成前缀，入口那一趟在**拼出来的这份文本**上重新 elaborate ——
   `closure_prefixes_for`（`crates/front/src/compile/check/mod.rs:1052`）、
   `closure_accumulated_over`（`:1066`）、唯一累加实现 `accumulate_prefixes`（`:1074`）。
2. **每趟 pass 都从零建环境**：`run_pass` 里 `let arena = stumpalo::Arena::new()`
   （`check/mod.rs:985`）+ `run_pass_in` 里 `let builder = EnvBuilder::new(arena, …)`
   （`:1013`）。唯一的例外是 session 那条路（`run_pass_with` 收调用方的 builder，`:1270`）。
3. **产物存的是 C，不是 A/B**：`docs/design/module-artifacts.md:19-31` 把产物分三块
   （A 内核环境 / B 前端表 / C 报告事件），今天磁盘上只有 C；`project/cache.rs` 存的是
   `ProjectReport`。
4. **内核没有 `ExportFile → EnvBuilder` 的入口**：`crates/kernel/src/builder.rs` 只有
   `new`/`snapshot`/`hide_declars`/`restore_declars`/`with_env`/`finish`；
   `crates/front/tests/a3_cross_entry_module_reuse.rs:14` 把这条写成结论：
   **"刀 2 是新机制，不是接线"**。
5. **内核有 reader、没有 writer**：`crates/kernel/src/parser.rs` 是 ndjson **导入**器
   （`NdjsonParser::new` `:705`，格式 semver 3.1–3.2 校验 `:24-38`），
   全仓 `serde` 只出现 `Deserialize`（`env.rs:4`、`expr.rs:5`、`util.rs:18`）——
   **没有任何 `Serialize`**。
6. **指针同一性是红线**（决定刀 2 的形状）：`ExprPtr` 的 `PartialEq` 比**地址位**，
   内核多处按指针比较（`conv.rs` 的 `NatLit`、`eval.rs` 用地址做内容哈希、
   `NameInterner` 比 `StringPtr` 地址）⇒ `incremental-environment.md:1052-1062` 写死：
   **"绝不允许'两份真相'：检查点只由'这份 arena 自己编出来的库层'产生，不许由
   `ExportFile` 反序列化、不许跨 arena 搬"**。⚠ 刀 2 必须正面处理这条 ——
   不是绕过它，而是换一个**装载形态**（装进本趟自己的 arena、装完即"这一份"，
   而不是"反序列化一份 + 本地再编一份"）。

### 2.3 已落地 / 未落地（**别重做**）

| 已落地 ✓ | 证据 |
|---|---|
| 一个 builder 贯穿整次 `build <dir>`（切片 1b） | `docs/design/module-artifacts.md:123-150`；`session.rs` 的 `with_project_session` |
| 共享库层**按库闭包签名分组**只编一次（G-68） | `project/mod.rs:602`（`compile_entries_shared`）+ `scripts/check-recompile-factor.py`（Σ闭包 1614 → 517） |
| 库层检查点**跨调用**复用（G-29，设计 §33.1） | `session.rs:67-92`/`:281`；判据 `g29_closure_recompile.rs`（`edit=1 < cold=5`） |
| 产物命中后**后台预热**库层检查点（A5） | `crates/lsp/src/lib.rs:93`；`crates/front/src/project/mod.rs:417`（`warm_library_checkpoint`） |
| 完整性/失效的**磁盘设计**（未实现） | `module-artifacts.md` §8.1–§8.4（内容寻址、三道校验、默认关、按 tag 锁定、离线降级） |

### 2.4 分阶段任务

**T1-A · 进程内 per-module 检查点（规模 M · 纯前端接线 · 可立即开工）**

* **做什么**：把"库层一份检查点"细化为"**每个库模块一份**"，键 = `module_keys()`
  （`project/mod.rs:204`，per-module Merkle：`key(M) = cache::key(路径 + 源文本 + [name(D),key(D)]…)`）。
  形状沿用 G-29 已经跑通的机制：同一份泄漏 arena 上按拓扑序编模块，到模块边界留一份
  `EnvBuilder`（`Clone` = 浅拷贝、同 arena、同指针 ✓，`builder.rs:26-38` 的注释），
  下游入口按自己的依赖集合 `restore_declars` 到**最深的那个已缓存依赖**再继续。
* **为什么不是"新机制"**：键已经在了（`module_keys` 零生产调用），检查点的持有/上界/
  指针同一性判据也都在（`session.rs` + `g29_library_checkpoint_is_bounded`）
  ⇒ **这是把两件现成东西接起来**。
* **真实工作量在哪（诚实记）**：`DeclarMap: Clone` 是 **O(#decls)**（`env.rs:256` 的
  `FxIndexMap`）⇒ 每个模块一份检查点的内存 ≈ Σ(前缀长度) = **O(N²) 个条目指针**。
  ⇒ 必须 (a) **只给度数 ≥2 的模块**（被多个入口共享的）留检查点；
  (b) 复用 G-29 的两条上界（`MAX_LEAKED_LIB_ARENAS=8`、`MAX_REUSES_PER_CHECKPOINT=64`，
  `session.rs:100`/`:109`）；(c) 到顶回退到"整条库层一份"（今天的行为，逐字节相同）。
* **判据（缺一不算）**：
  ① `crates/front/tests/a3_cross_entry_module_reuse.rs` 的断言 **3 → 2**
     （文件头写明这是**唯一正确出口**，不许放宽）；
  ② `crates/front/tests/session_reuse.rs:153` 的 `#[ignore]` 守卫删 `#[ignore]` 后翻绿；
  ③ 新增 `module_keys` **命中计数**（结构计数，第 2 条入口 ≥1）；
  ④ **反例**：改共享依赖一行 ⇒ 必须 miss 且重编（`module-artifacts.md` §5 第 1 条已在册）；
  ⑤ 红线：全语料 `--json` 逐字节 + 指针同一性单测保持绿
     （`crates/kernel/src/builder.rs::a_reused_environment_is_pointer_identical_and_a_rebuilt_one_is_not`）。
* **风险**：中。唯一的静默错编面是"键少折了一样东西"（路径/prelude/开关/依赖序）——
  处置同 G-29：键**只许**由 `compile::cache::key` + `module_keys` 生成（单一真相）。

**T1-B · 磁盘模块产物 = 刀 2（规模 XL · 新机制 · 排在 T2-A 之后）**

* **五件套（缺一不算落地）**：
  1. **产物格式**：内核加 writer（今天只有 reader `parser.rs`）。第一版建议**直接复用
     内核自己的 ndjson 导出格式**（reader 与 semver 校验都在）⇒ 少发明一种格式；
     二进制/紧凑格式留作后置（先证"跨进程复用"，再谈字节效率）。
     ⚠ **对比一条 Lean 的硬事实**：Lean 的 `.olean` 缺失是**报错**、**不从源码重编**
     （`Environment.lean:1982-1986` 原话 "object file … does not exist"）；
     我们今天是"miss ⇒ 本地重编"（这是对的，因为**没有**产物；产物落地后要**明确选边**：
     缺产物 = 静默重编 ✓（教学场景），**不要**学 Lean 报错 ✗）。
  2. **装载入口**：`EnvBuilder` 加"从产物装载并继续 `add_declar`"的口
     （新 API；`builder.rs` 现有七个方法都不够）。
  3. **指针同一性**：装载必须发生在本趟 pass **自己的 arena** 里，装载之后所有 elaborate
     都在同一 arena —— 这条是 T1-B 的**设计约束**（不是实现细节）。
  4. **完整性/安全/离线**：照 `module-artifacts.md` §8.1–§8.4（三道校验、默认关、
     按 tag 锁定、失败当不存在、离线静默回退）。
  5. **失效规则对齐 lake**：`BuildTrace{hash, mtime}`（`src/lake/Lake/Build/Trace.lean:308-312`，
     由源文本内容 + mtime 播种，判定 `depTrace.hash == depHash && 产物存在`）
     —— 我们的 Merkle 键**已经**覆盖"源文本内容"这一半 ⇒ 只需要把 `module_keys` 的键
     接进产物**文件名**（§8.1 已设计），mtime 永不参与正确性（只当"要不要检查"的提示）。
* **阻塞清单（"把已 elaborate 的声明持久化"卡在哪，逐条带 `文件:行`）**：

  | # | 阻塞 | 证据 |
  |---|---|---|
  | B-1 | **arena 是 bump 分配器**：`ExprPtr`/`NamePtr`/`LevelPtr` 全是 arena 内裸地址 ⇒ 序列化要**整体重定位** | `crates/kernel/src/builder.rs:39`/`:190`/`:326-327` |
  | B-2 | **`ArenaRef` 是 `!Send` + `!Sync`**（`Cell<*mut u8>` + 裸指针；stumpalo 只给 `Arena` 实现 `Send`） | stumpalo 0.5.1 的 `src/arena_ref.rs:29-41`（cargo registry） · `session.rs:18-24` |
  | B-3 | **`DeclarMap` 是 `pub(crate)` 别名** ⇒ front **不能命名**它 ⇒ 检查点进不了具名字段 | `crates/kernel/src/env.rs:256` · `session.rs:3-5`/`:469-470` |
  | B-4 | **`PassTables` 同样是 `pub(crate)`** ⇒ 前端表（`known`/`inductives`/`defs`）也不能命名 | `check/mod.rs:173-178` |
  | B-5 | **自引用**：`EnvBuilder<'a>` 借 `&'a ArenaRef<'a>`，arena 要活到下一次调用 ⇒ 只有 (a) `Box::leak`（今天走的，有两条上界）/ (b) 自引用 + `unsafe` | `incremental-environment.md:1094-1102` · `session.rs:94-109` |
  | B-6 | **指针同一性红线**：`ExprPtr` 的 `PartialEq` 比**地址位**（`conv.rs` 的 `NatLit`、`eval.rs` 用地址做内容哈希、`NameInterner` 比 `StringPtr`）⇒ 不许反序列化出"第二份真相" | `incremental-environment.md:1052-1062` · 两向判据 `crates/kernel/src/builder.rs:609` |
  | B-7 | **没有 `ExportFile → EnvBuilder` 入口**（`builder.rs` 只有 `new`/`snapshot`/`hide_declars`/`restore_declars`/`with_env`/`with_env_scope`/`finish`）；CLI **无** `export`/`import` 子命令 | `crates/kernel/src/builder.rs:87`/`:115`/`:120`/`:163`/`:190`/`:561` |
  | B-8 | **独立环境 `add_declar` 会改写共享 `decl_idx` 槽位**（T-K12c 死因：它往**被 intern 的 `NameNode`** 上写 `set_decl_idx`） | `crates/kernel/src/builder.rs:451-453` |
  | B-9 | **`&ArenaRef` 存进 `ExportFile` 走不通**（历史结论：`ArenaRef` 非 `Send`/`Sync`，而 `ExportFile: Sync` 是并行检查的硬要求） | `docs/design/project-artifacts.md:307-310` |
* **mmap 的诚实定位（必须写清，别照抄招牌）**：Lean 的 mmap 让 `.olean` 的**载荷零拷贝**
  （`readModuleDataParts`，`Environment.lean:1730`），代价是**每次进程调用重建索引**
  Θ(Σ 导入常量)（`finalizeImport`，`:2188-2296`）—— 连 Lean 自己都不是"免费"
  （perf-lean4 §1.6 已记）。而本仓库的载荷是 **arena 里的项图（指针）**，必须落进 arena
  ⇒ **真 mmap 只能覆盖"索引/表"，覆盖不了项图**。⇒ 第一版做"**读入 arena 的产物**"；
  mmap 留作后置优化，且要做成 Lean 那样**映射失败就退化成一次读**的静默降级
  （`src/library/module.cpp:320-344`）。
* **判据**：跨进程（新进程冷开同一入口）的 `compiled` 模块数下降（结构计数）·
  反例：改依赖一行 ⇒ 必 miss 且重编 · 产物损坏/格式不符 ⇒ **当不存在**（不报新错误形态）·
  离线 ⇒ 本地重编且报一行来源 · 红例：`--json` 逐字节 + 课程门禁 0 判负。
* **风险**：高（跨进程信任边界 + 内核 API + 安全模型）。**建议拆 3 批**：
  批 1 = 内核 writer + 装载口 + 单进程往返判据（不进缓存目录）；
  批 2 = 接到 `<根>/.sokonanoda/artifacts/` + 完整性三道；
  批 3 = 并发/损坏/离线/`--clean` 的边界与课程门禁。

**T1-C · 失效规则收口（规模 S）**：把"键优先、mtime 只兜底"写成契约并加一条守卫
（mtime 变、内容不变 ⇒ **不许** miss；内容变、mtime 不变 ⇒ **必须** miss）。

### 2.5 工作量与风险小结

| 任务 | 规模 | 预计环节 | 批次 | 风险 |
|---|---|---|---|---|
| T1-A | M | 2–3 个 commit | 批次 A | 中（键/指针同一性，判据齐） |
| T1-B | XL | 3 批 × 3–5 commit | 批次 E/F/G | 高（内核 API + 信任边界） |
| T1-C | S | 1 | 随 T1-B 批 2 | 低 |

---

## 3. 主题② 服务器热路（长寿命增量处理器 + 每条命令快照树）

### 3.1 现象

| # | 现象 | 证据 |
|---|---|---|
| P2-1 | **库层已经跨调用复用** ✓（不是缺点了） | `session.rs:281-347`（`with_project_session_reusing`）；结构臂 `edit.modules=1 < cold=5`；上界两条都有常驻判据（`g29_library_checkpoint_is_bounded`） |
| P2-2 | **入口趟每次从零 elaborate** ✗ —— 改最后一条声明，前面每条命令的**环境推进**照样重做 | `run_entries`（`session.rs:472`）克隆检查点（`:491-492`）后整趟 `run_pass_with`（`:538-559`）；`run_incremental`（`check/mod.rs:654`）→ `run_pass`（`:977`）→ `EnvBuilder::new`（`:1013`）；trust 的语义 = **"keep the environment, skip the kernel"**（`walk.rs:878-880` 原话，`incremental-environment.md:978-983`） |
| P2-3 | 单文件路径的"快照树"**只存结果、不存环境**，且**不在项目热路上** | `session.rs:82-95`（`CmdSnapshot` 字段：`state`/`hovers`/`events`/`errors`/`warnings`/`signature`）；复用判据 `DeclKey`（`:74-78`）+ `first_diff`（`:217-221`）；`Session` 只被单文件路（`query/mod.rs:501`）与 CLI watch 用 |
| P2-4 | LSP 的编译被**钉在一条线程**上（G-29 的代价） | `crates/lsp/src/lib.rs:790-808`（`worker_threads(1)`，32MB 栈）；多文档并发编译由此**串行**（如实记在 `incremental-environment.md:1120-1125`）；长期持有者是 `Compiler::carriers`（`crates/lsp/src/lib.rs:596`），里面**只有 `QueryDoc`、没有环境** |
| P2-5 | **A4a 只兑现了 1/3**：入口趟的三张 O(闭包) 派生表里，① 闭包前缀已随检查点存（`lib_prefix`，`session.rs:514-529`）；② 记法表 `entry_display`（`:530`）与 ③ 定义 span 表 `entry_defs`（`:535`）**每刀仍重建，且没有计数器** | `check/mod.rs:501-509`（`display_notations`）/`:1632-1640`（`top_level_def_spans_over`）；`grep` 两个计数器名 = 0 行 |

### 3.2 根因

1. **检查点粒度 = 库层**（`session.rs:67-92`），**没有命令级环境快照** —— 这是"入口趟
   每次从零 elaborate"的直接原因。
2. **能跳的只有内核重查**：`TrustPlan` 的语义（`check/mod.rs:161` 起、
   `walk.rs` 的 trusted 分支）是"这段文本没变 ⇒ 不用再查内核"，而**环境仍要逐条推进**。
3. **结构前提缺失（本条最重要）**：`DeclarMap = FxIndexMap<NamePtr, Declar>`（`env.rs:256`）
   **不是持久化结构**，`EnvBuilder` 的 `Clone`（`builder.rs:37-38`）是**深拷贝 map**
   ⇒ 在每条命令边界存一份环境 = **O(N²)** 条目拷贝。
   Lean 之所以能"snapshot the entire state after each command"（`Language/Lean.lean:83-92`），
   是因为 `SMap`/`PHashMap` 的插入只重建**根到叶那一条路径**
   （`src/Lean/Data/PersistentHashMap.lean:40-43`）。
4. **`by` 块内没有 elaborate 快照**（只有目标快照 `by_steps`，`compile/report.rs:171`）—— 见 §3.4。
5. **前端表也没有"快照/回滚"能力**：`PassTables`（`check/mod.rs:173-188`）只有 `new()`，
   walk 边跑边插（`walk.rs:863`/`:902`/`:1063`/`:1160`/`:1235`…），**没有截断或 undo**
   ⇒ 按命令粒度复用它就得每条命令克隆三张表（O(n) ⇒ O(n²)）。
   内核原语 `hide_declars`/`restore_declars`（`builder.rs:115-122`）**是**"环境快照"的形状，
   但今天只在 T-D8 冗余探针里**瞬时**用（`walk.rs:976/994`、`:1346/1364`、`:1755/1773`），
   用完即还、从不按命令留存。

### 3.3 分阶段任务

**T2-B0 · 入口趟另两张派生表随检查点存活（规模 S · 纯接线 · 可立即开工）**

* **做什么**：`entry_display`（`session.rs:530`，`display_notations`）与 `entry_defs`
  （`:535`，`top_level_def_spans_over`）都是**闭包文本的纯函数**，今天每刀重算。
  正确切法（A4a 的教训 `fc636b3e`）：**拆成"库层那一段（随检查点存）+ 入口那一段（每刀只算入口）"**
  —— **不要**按整闭包文本做键（入口文本每刀在变 ⇒ 必然 miss）。
* **前置**：**先建两个计数器**（照 `closure_prefix_builds_total` 的形状，
  `check/mod.rs:1074-1095`）—— 今天这两张表**连"几次"都量不到**。
* **判据**：新增 `notation_table_builds_total` / `def_spans_builds_total`：冷开 = k、
  **第 2 刀起 = 0（或只随入口规模）**；反向验证（撤掉复用 ⇒ 回到每刀 ≥1）；
  红线 = 全课程 `--json` 逐字节 + `keystroke_structure` 的 1/1/4 保持绿。
* **风险**：低（派生表，非判定）；**天花板** = 个位数 %（`PLAN-cli-editor-perf` A4a 已记），
  价值在**去掉 O(闭包) 伸缩**。

**T2-A · 内核：声明表持久化 / COW（规模 XL · ② 的前置件）**

* **三条候选形状**（按改动面从小到大）：
  * **(c) 增量日志 + 反向回滚**：不改数据结构，在命令边界记 undo 日志，回到编辑点用 undo
    而不是克隆。改动面最小，但"回到命令 k"要能**精确撤销**环境推进（含
    `decl_idx` 槽位、`notations`、`mutual_block_sizes`）⇒ 正确性论证贵，且与
    "环境不可变（Lean 的不变量）"背道而驰。
  * **(b) 分层 `SMap`**：对齐 Lean —— 导入层用平坦 HashMap（导入期独占、允许破坏性写），
    本地新增层用 **PHashMap**（`src/Lean/Data/SMap.lean:17-27` 的原话就是这个理由）。
    与"库层=导入层、入口=本地层"的现有结构**天然同形**（`session.rs` 的 hide/restore 就是
    这个分界）⇒ **推荐**。
  * **(a) 全量 PHashMap**：最贴 `src/Lean/Data/PersistentHashMap.lean:40-43`，但把导入期的
    破坏性写优势也丢了（每个库模块的插入都变成路径拷贝）。
* **判据（缺一不算）**：
  ① **结构计数**：在命令边界取一份环境的"克隆成本"读数（每命令复制的条目数 / 路径节点数）
     —— 今天 = O(#decls)，目标 = O(log n) 或 O(1)；
  ② 指针同一性**两向**判据保持绿（`crates/kernel/src/builder.rs` 那条）；
  ③ 全语料 `--json` **逐字节**（判定红线）；
  ④ 内核单测（今天 63/63）+ `kernel-diff.sh --fast` 零差异；
  ⑤ 反例：**不许**引入第二份 intern 表（`NameNode::decl_idx` 的槽位挂在被 intern 的
     节点上，换表 = 静默取到别人的声明，`builder.rs:58-70` 的注释就是这个警告）。
* **风险**：高（内核数据结构 + 指针语义）。**建议先写一页设计 + 一条"先建先红"的
  指针同一性/成本判据，再动代码**（照 §32.4 的先例）。

**T2-B · 入口趟命令级环境快照（规模 L · 依赖 T2-A）**

* **形状**：在入口那一趟里，按命令边界留下 `(EnvBuilder, PassTables, 报告片段)`；
  编辑后从**最后一个文本未变的命令**接着编。
* **复用判据沿用现成的、不新造键**（这是本条的纪律）：
  入口命令的**文本 + 起点**（`query/mod.rs:186` `entry_command_layout`）+
  **依赖指纹**（`:142` `dependency_fingerprint`）+ 库层键（`session.rs:183` `lib_key`）。
  ⇒ 判据是**结构相等**（对齐 Lean 的 `eqWithInfo` 思路），**不是哈希缓存**
  （明确避开 B4 被否的那条路：错键 = 静默用旧目标判定，`incremental-environment.md:1164-1171`）。
* **判据**：新增结构计数"**elaborate 命令数 / 按键**"——改最后一条 ⇒ **1**
  （今天 = N）；改第 k 条 ⇒ N−k+1；`keystroke_structure.rs` 既有断言**一个字不放松**；
  全语料 `--json` 逐字节；反向验证（改依赖文件 ⇒ 命令数回到 N）。
* **风险**：中高。**错编的唯一入口是"环境身份"没进判据** ⇒ 复用条件必须同时含
  库层键 + 依赖指纹 + 入口命令前缀文本三者（少一样就是静默错编）。

**T2-C · 单文件 `Session` 接同一机制（规模 M · 依赖 T2-B）**：
`CmdSnapshot` 加环境句柄（同一套 `restore`/`Clone`）；判据 = `session.rs` 的单测
（`recompiled_from` 不变、`kernel_checks` 下降）+ 既有断言不放松。

### 3.4 单文件内子部分复用（`withNarrowedTacticReuse` 类比）—— **当前不做**

* **Lean 怎么做**（已核对）：`Term.withNarrowedTacticReuse`（`Elab/Term/TermElabM.lean:455-472`）
  把 tactic 语法 `split` 成 `(outer, inner)`；**outer** 用 `eqWithInfoAndTraceReuse` 比较
  （`:464` 的 `guard`），相同 ⇒ 把旧快照的 `stx` **收窄到 inner**（`{ old with stx := oldInner }`）。
  ⚠ **它复用的是"tactic 的 elaborate 快照状态"，不只是语法子树**：真正的复用发生在
  `Elab/Tactic/BuiltinTactic.lean:78-83`（`oldParsed.finished.get.state?` ⇒ `reusableResult?`）
  与 `:110-118`（`withRestoreOrSaveFull reusableResult?` ⇒ **整条 tactic 跳过执行、直接恢复旧 state**）。
  **两个前提**：① 外层语法逐字节相等（不等 ⇒ 该子树复用禁用，并 `cancelRec` 旧子树 `:468-470`）；
  ② tactic 必须**显式声明参与**（`@[builtin_incremental]`，注册处 `Elab/Term.lean:50-55`）
  并在函数体里调 `withNarrowedTacticReuse`/`withReuseContext`、把
  `getAndEmptySnapshotTasks` 收进快照（`src/Lean/Elab/Tactic/BuiltinTactic.lean:120-125`）。
  姊妹件 `withNarrowedArgTacticReuse`（`:474-485`）以 `stx[argIdx]` 为界，
  **只比较 argIdx 之前的子节点**。
* **本仓库的现状**：`by` 块是**扁平**的 tactic 列表（`Expr::By{tactics}`），
  逐 tactic 只有**目标快照**（`by_steps`，`crates/front/src/compile/report.rs:171`），没有 elaborate 快照；
  块内任何编辑都从第 0 条重跑（已在 `PLAN-cli-editor-perf` P3 记录）。
* **规划结论（三条理由）**：
  1. **形态不同**：已被否决的 B4 是"按 `(环境身份 + tactic 前缀 hash)` 缓存**判定结果**"
     （错键 = 静默错判，红线）；Lean 的形态是"**结构相等 + 活快照**"。
     ⇒ **不要把 B4 当成 withNarrowedTacticReuse 的实现**，也不要在 T2-A/T2-B 之前重开它。
  2. **收益面小**：本仓库的 `by` 块是扁平的，没有 Lean 那种嵌套 tactic
     （`induction … with | …`、`first | …`）；`withNarrowed*` 的用武之地
     （`src/Lean/Elab/Tactic/{BuiltinTactic,Induction}.lean:66`/`:134`/`:633`）在教学内容里**今天不存在**。
  3. **成本已经不高**：失败块的走查数已由 B1/B3 压到 **2**（与出错位置无关，
     `incremental-environment.md:1164-1171`）⇒ 剩下的 1 趟就是**严格结论本身**。
* **重开门条件（写死，免得下一轮凭感觉）**：
  (a) T2-B 落地后，实测显示"块内编辑"成为**主要**成本（结构计数，不是感觉）；
  (b) 课程出现**嵌套结构化证明**（`induction … with` 之类）且它是热路径；
  (c) 或者 T2-A 的持久化把"步级快照"的边际成本降到可忽略。
  **三者任一**才重开；重开时按 Lean 的形态（结构相等 + 活快照），**不按 B4 的形态**。

### 3.5 工作量与风险小结

| 任务 | 规模 | 预计环节 | 批次 | 风险 |
|---|---|---|---|---|
| T2-A | XL | 设计 1 + 实现 4–6 | 批次 C | 高（内核数据结构） |
| T2-B | L | 3–4 | 批次 D | 中高（环境身份进判据） |
| T2-C | M | 2 | 批次 D | 中 |
| T2-D | XL | — | **不做**（条件见 §3.4） | — |

---

## 4. 主题③ 活环境查询（消灭合成前缀 / 合成趟）

### 4.1 现象与成本结构

| # | 读数 | 出处 / 构建身份 |
|---|---|---|
| P3-1 | **`run_pass_with` 占编译 87%** | sample 20s / 40 刀 · unit08 改陈述（`PLAN-cli-editor-perf` §8.12） |
| P3-2 | 合成趟仍是主线：`infer_type_text_inplace` 17.7%，其中 `set_literal_prefix_args → judge_type_of` 的**合成趟 8.4%** | 同上 |
| P3-3 | `judge_infer` 的**合成前缀已清零**（`prefix=0`）✓ | `incremental-environment.md:1172-1175`（`lsp_keystroke_structure` 的 7 条 trace） |
| P3-4 | 历史成本分解（库层 15% / 入口趟 82%，其中 7 次合成前缀 = 58%） | `incremental-environment.md:1074-1083`（**别的构建**，只当量级参考） |

### 4.2 根因（逐条给 `文件:行`）

1. **三条查询都合成文档**：
   * `judge_infer`（`crates/front/src/judge.rs:2309`）→ `judge_infer_uncached`（`:2717`）
     拼 `#check <term>`（`:2733-2735`），再走 `run_synthesized_incremental`（`:2790`），
     没有可用担保就回退 `compile_fol_with`（`:2797`，**整份重编**）；
   * `judge_type_of`（`:2209`）→ `judge_type_of_uncached`（`:2263`）→ 同一对
     （`:2283`/`:2290`）；
   * `judge_pairs`（by 块）→ `judge_pairs_uncached`（`:1389`）→
     `synthesized_prefix`（`:3129`）拼前缀 + 追加合成命令。
2. **合成趟就是一次全新的 pass**：`run_synthesized_incremental`（`:1097`）→
   `run_incremental`（`check/mod.rs:654`）→ `run_pass`（`:977`）⇒
   **新建 arena + 新建 `EnvBuilder`**（`:985`/`:1013`）⇒ 前缀必须重新 elaborate。
   已落地的 G-31/G-92 第二刀只让**前缀里 `theorem` 的证明体**不重跑
   （`trusted_entered` ⇒ 不透明常量，`:1118`），**前缀的类型与所有 `def` 的体仍重编**
   （`incremental-environment.md:1033-1035` 如实记为"降级交付"）。
3. **所有权阻塞**：`run_pass_with(builder: EnvBuilder, …) -> (…, EnvBuilder, …)` **按值收发**
   （`check/mod.rs:1270`），而 judge 在 `elab_expr` 链里只有 `&mut`
   （`incremental-environment.md:978-983`）。
4. **已经拿到的部分**（**别重做**）：`InplaceEnv` 就地判定
   （`crates/front/src/compile/elab.rs:2917`/`:2926`；`infer_type_text_inplace` `:3097`；
   `by.rs` 多处 `InplaceEnv::reborrow`）、
   A2a（`needs_explicit` 不再压 `ByMode::Off`：**旧行号** `by.rs:749-750`（`git show 5c224ee4^`）⇒ 今天 `:786-794`）、
   A7（judge 键的 O(前缀) SipHash 改 4 条小 LRU，`9f20a1b8`）。
   ⚠ **上游笔记 §4.4 说的"`EnvView` / `judge_infer_with` 已有部分落地"是错的** ✗：
   `EnvView` trait **零实现**（`judge.rs:2594-2602` 自认），`judge_infer_with_env`
   （`judge.rs:2397`）**唯一消费者是单测**（`:3434-3480`）⇒ 生产代码**零调用点**。
   这条要在下一轮开工前改掉，否则会照着不存在的接口做设计。
5. **仍走合成趟的判定点**（T3-B1 的正身，逐条带行号）：
   `cases` 的被消去项 `judge_infer_explicit`（`by.rs:2322`，**没有就地兄弟**，
   `judge.rs:2357-2365` 只有慢路）· `judge_render_type_explicit`（`judge.rs:2370`，
   注释 `:2369` 明写"不走就地快路"）· `level_hint_of` 的**记法形态**那一支
   （`by.rs:88-93`，就地路只收源 AST ⇒ 需要"文本 ⇒ AST"入口）·
   记法目标签名 `judge_type_of_constant`（调用点 `elab.rs:1360`，定义 `judge.rs:2234`）· 不匹配诊断文本（`by.rs:2841`）。

### 4.3 ⚠ 对账：**"A2b 不做"那道决策门只关了一半**

`incremental-environment.md:1172-1175` 记的门是：**"A2a 之后还剩 >0 次前缀重跑才值得动"**，
实测 `prefix=0` ⇒ **不做**。但 `prefix=` 这个读数的定义是
**`PREFIX_RUNS`**，只在 `judge_infer_uncached` 里自增（`judge.rs:2744`）
⇒ 它**只数 `judge_infer` 的 miss**，**不数** `judge_type_of` 与 `judge_pairs` 的合成趟。
而 sample 里 `judge_type_of` 的合成趟仍占 **8.4%**（§8.12）。

⇒ **结论**：那道门关掉的是"**消 `judge_infer` 的前缀重跑**"；
"**消合成趟**"（把活环境交给 judge）**没有被关**，而且它的判据要另立一个计数器
（合成趟的 `passes`/`modules`，今天散在 `MODULE_COMPILES` 里 —— 注意
`incremental-environment.md:1085-1091` 记的**量具更正**：`modules=` 那条旧读数
把 judge 合成编译算进去了，精确读数要用 `closure_module_compiles_total()`）。
**下一轮开工前必须先补这个计数器**，否则 T3-B 没有判据。

### 4.4 分阶段任务

**T3-B1 · 三条纯接线（规模 M · 不需要内核 API · 今天可做）**

| # | 改哪 | 证据 | 判据（缺一不算） |
|---|---|---|---|
| ① | `cases` 的被消去项接就地（`by.rs:2322`）；在 `judge.rs` 加 `judge_infer_inplace_with_explicit`（照 `judge_render_type_inplace_with_explicit` `judge.rs:1754` 的受控入口形状） | `judge.rs:2357-2365` 只有慢路 | `prefix=`（`LSP_TRACE`）下降 · `SOKO_JUDGE_INPLACE_BY=shadow` ⇒ **`diff=0` 且 `same>0`** · 反向验证（explicit 档置 `false` ⇒ 同用例必红） |
| ② | 给 `judge_render_type_explicit`（`judge.rs:2370`）补就地 | `judge.rs:2369` 明写不走 | 同上 |
| ③ | `level_hint_of` 的记法形态那一支（`by.rs:88-93`） | ⚠ 这条需要"**文本 ⇒ AST**"入口 = **新入口**，不是纯接线 | `BY_LEVEL_HINT_MISS`（`by.rs:69`）计数下降 + 影子档 `diff=0` |

* **为什么必须配影子档**：`on` 档 `--json` 逐字节相同**咬不住**分叉 —— 实测第一版多剥一层，
  `on` 档全绿而影子档抓出 `shadow_diff=37508`（`judge.rs:1214-1219`/`:3069-3073`）。
* **风险**：中（判定热路径，但每条都有影子档 + 反向验证 + 既有 `judge_inplace*.rs` 判据家族）。
* **顺序建议**：①② 先做（形状与 A2a 同款，`5c224ee4` 可照抄）；③ 单独一轮（新入口）。

**T3-B2 · 消合成趟（架构件 · 规模 L · 内核 + front · 与内核线 WIP 同片地）**

* **重开门条件（必须先满足，否则不做）**：决策门 `incremental-environment.md:1172-1175` 与 `:1191-1192`
  记的"不做"针对的是 `prefix=0`（只数 `judge_infer`）；本条要动的面**另有判据**
  —— **先补"合成趟"的结构计数**（今天 `MODULE_COMPILES` 把 judge 合成编译也算进去，
  `incremental-environment.md:1085-1091` 的量具更正），量出
  `judge_type_of`/`judge_pairs` 的合成趟仍是主要成本（sample 里 8.4%）再开工。
* **形状 (i)：pass 借 builder**（首选，起手清单已在 `incremental-environment.md:990-993`）：
  ① `flush_batch`（`judge.rs:2150`）收 `Option<&mut InplaceEnv>`，透传给
  `judge_pairs_with`/`judge_pairs_uncached`（`check_synthesized` 定义 `judge.rs:931`，
  调用点 `:1526`）；
  ② `check_synthesized` 在有 env 时改走"**只含合成命令**的 pass"（`src` 仍给整份前缀供
     span 查表 —— ⚠ 这条**未验证**）；
  ③ `Walk`/`run_pass_with` 加**借用变体**（按值那版**保留**给 session）；
  ④ 合成声明用 `hide_declars`/`restore_declars` **回滚**。
  ⚠ 指针同一性上更安全：新 intern 直接进 **live 表**（不造第二份节点）。
* **形状 (ii)：`EnvBuilder::fork()` + `absorb()`**（仅 (i) 被证否时）：
  同 arena + 克隆 intern 表；⚠ fork 里新 intern 的字面量会在共享 arena 造出
  live 表不认识的节点 ⇒ live 之后 intern 同一个值会**造第二份**（`NatLit` 按指针比较
  ⇒ 假失败）⇒ 还要 `absorb()` 合并口，**面不比 (i) 小**（`:987-988`）。
  **今天 `fork` 不存在**（`grep -rn "fn fork" crates/` = 0 命中）。
* **判据（缺一不算）**：
  ① **先建读数**：合成趟的结构计数（`passes`/`modules`/`by_calls`）—— 冷开 = k、第 2 刀起 = 0
     或只算新增；
  ② 影子档 `diff=0`（比**调用方读到的结论**，不比报告形状 —— §31.3 的教训）；
  ③ 全课程 `--json` 逐字节（判定红线）；
  ④ 指针同一性单测（`crates/kernel/src/builder.rs` 的两向判据）；
  ⑤ 反向验证：`SOKO_JUDGE_PREFIX_OPAQUE=off` 的既有红/绿必须**仍然对应**；
  ⑥ `G92-…sh` / `G31-judge-prefix-rerun.sh` 保持 exit 1。
* **依赖/冲突**：与内核线 WIP（`stash@{0}: d68f2fe9-envprov-wip-1005-1010`）**同一片地**
  ⇒ **先对齐归属再动手**（不许两线并行改 `run_pass_with`）。
* **风险**：高（判定热路径），但判据齐（影子档 + 逐字节 + 指针单测三层都在）。

**T3-C · `EnvBuilder::fork()`（规模 XL · 仅在 (i) 被证否时）** —— 见上。

**T3-D · `EnvProvider` 的处置（规模 S · 收尾项）**：今天零实现零接线
（`judge.rs:2594-2602`）⇒ 二选一：**按 as-built 重写成 `InplaceEnv` 的形状并接线**，
**或删掉它**（删前先改台账 G-92 的 `expected_lean`，别留一个不存在的接口在文档里）。

### 4.5 工作量与风险小结

| 任务 | 规模 | 预计环节 | 批次 | 风险 |
|---|---|---|---|---|
| T3-B1 | M | 2（①②）+ 1（③） | 批次 A′ | 中（判定热路径；影子档必需） |
| T3-B2 | L | 计数器 1 + 实现 3–4 + 判据 1 | 批次 B（**重开门后**） | 高（判定热路径；判据齐） |
| T3-C | XL | — | 条件触发 | 高 |
| T3-D | S | 1 | 随 T3-B1 | 低 |

---

## 5. 主题④ CLI / LSP 分路

### 5.1 现象与根因

| # | 事实 | 证据 |
|---|---|---|
| P4-1 | LSP 有热路：**线程局部**库层检查点 + **单 worker** 编译 runtime（32MB 栈）+ 产物命中后台预热 | `crates/lsp/src/lib.rs:790-808`；`crates/front/src/project/mod.rs:417`（A5）；`crates/lsp/src/lib.rs:93`（`served_from_artifact`） |
| P4-2 | CLI **每个子命令各走一条**，且都不带跨调用状态：`build` = `compile_entries_shared`（多入口、按库闭包签名分组，**组内**一次 session）· `check`/`course` = `compile_plan`（**整条闭包一趟**，连 session 都不用）· `query` = 项目缓存 `load_at`（miss 再编 + 存回） | `crates/cli/src/build.rs:130` · `crates/front/src/project/mod.rs:602`/`:756` · `crates/cli/src/check.rs:82` · `crates/cli/src/course/mod.rs:417` · `crates/cli/src/query.rs:159-175` |
| P4-3 | 跨调用复用的开关只有 LSP 那条路打开 | `reuse_library=true` 只在 `crates/front/src/query/mod.rs:653`/`:662`；`project/mod.rs:512-515` 的注释自己写着 `false ⇒ CLI/测试走这条` |
| P4-4 | 分路是 `!Send` 逼出来的**结果** | `session.rs:18-24`：`ArenaRef` 内部是 `Cell<*mut u8>` + 裸指针 ⇒ `!Send`；`Doc` 必须 `Send + Sync`（`crates/lsp/src/lib.rs:461` 的 `static EMPTY: OnceLock<Doc>`、`:567`/`:1091` 的 `tokio::spawn`）⇒ 检查点只能线程局部 ⇒ 编译必须钉一条线程 |
| P4-5 | 代价（如实记） | 多文档并发编译**串行**（`incremental-environment.md:1120-1125`）；长期持有者是 `Compiler::carriers`（`crates/lsp/src/lib.rs:596`），里面**只有 `QueryDoc`、没有环境** |

**Lean 侧对照（本轮补核）**：`internal.cmdlineSnapshots` 默认 `false`，CLI 显式开 `true`
（`CoreM.lean:52-56` 的选项定义 + `Elab/Frontend.lean:151` 的 `setIfNotSet`）；
**它的实际动作是"只留 env、丢掉其余元数据"**（`Language/Lean.lean:681-684`：
`reportedCmdState := { env := reportedCmdState.env, maxRecDepth := 0 }`），
并且**同一选项会关掉增量复用**（`:758` `snap? := if cmdlineSnapshots then none else snap`；
`:643` `minimalSnapshots`）。⇒ Lean 的 CLI **不是"没有缓存"，是"主动把快照降级成只有环境"**。

### 5.2 分阶段任务

**T4-A · 分路契约化（规模 S · 可立即做）**

* **写什么**：一句话契约 —— **"跨进程增量归产物（磁盘模块产物），跨按键增量归检查点
  （线程局部环境）；CLI = 冷路 + 并行，LSP = 热路 + 单 worker"**；写进
  `docs/architecture.md` 的对应节 + 本文 §5，并在 `module-artifacts.md` 加一条指针
  （避免下一轮再把"分路"当 bug 修）。
* **守卫（一条，必须有牙）**：CLI 路径**不得**依赖线程局部检查点 ——
  在 CLI 的编译入口断言 `lib_checkpoint_arenas_leaked()` 与其等价读数恒 0
  （或更硬：CLI 侧不链接 `session::reusing`，用一条编译期/静态契约测试钉住）。
* **判据**：守卫在"故意把 CLI 拨到 `reuse_library=true`"时**必须判红**（反向验证）。

**T4-B · CLI 吃 per-module 产物（规模 L · 依赖 T1-B）**：
`build` 的 `hit/compiled` 从"整闭包 digest"升级到 per-module 产物
（`build.summary.hit`）；判据 = 冷编计数下降 + `scripts/check-recompile-factor.py`
（测**次数**，噪声免疫，已判红）+ 反例（改依赖必重编）+ `--json` 逐字节。

**T4-B0 · CLI `build` 先开 `reuse_library=true`（规模 S · 纯接线 · 先量后做）**：
分支**已经存在**（`project/mod.rs:558-566`），只差调用方选边；但 CLI 是短命进程 ⇒
收益只落在"**同进程多入口**"那一段。**先量**：`compile_entries_shared` 的"跨组残余 2.08×"
里有多少落在同进程（结构计数），>0 才做；**不要**为"让 CLI 也复用"把 `thread_local`
改成 `Mutex`（那要求 `Send` ⇒ 直接撞 B-2/B-3）。

**T4-C · LSP 放开编译并发（规模 L · 依赖 T2-A / T1-A）**：
前提是检查点不再绑线程（(a) 产物表 `Send`；(b) 或按文档分片，每片一个线程局部槽）。
判据：并发编译的结构计数 + 检查点命中数**不降** + 线程局部泄漏上界判据改造后仍绿 +
LSP 单测（`cargo test -p sokonanoda-lsp`）。

### 5.3 工作量与风险小结

| 任务 | 规模 | 预计环节 | 批次 | 风险 |
|---|---|---|---|---|
| T4-A | S | 1 | 批次 A | 低 |
| T4-B | L | 依赖 T1-B | 随 T1-B | 中 |
| T4-C | L | 依赖 T2-A/T1-A | 批次 H | 中高（并发 + 诊断时序） |

---

## 6. 批次编排

### 6.1 依赖图（→ = 前置件）

```text
批次 A（零依赖，可立即开工）
  T1-A   进程内 per-module 检查点（纯前端）
  T2-B0  入口趟另两张派生表随检查点存活（**先建计数器**）
  T3-B1  三条纯接线（cases / judge_render_type_explicit / level_hint_of 记法形态）
  T4-A   分路契约化 + 守卫
  T4-B0  CLI build 开 reuse_library（先量后做）

批次 B（与内核线同片地 ⇒ 先对齐归属；A 完成后）
  T3-B2  消合成趟（borrow 变体）── **先补"合成趟"结构计数、过重开门**
  T3-C   仅在 (i) 被证否时触发
  T3-D   EnvProvider 的处置（重写接线 或 删除）

批次 C（内核；B 让位给内核线时改做 C）
  T2-A  声明表持久化（SMap/PHashMap）── ② 的结构前置件

批次 D（依赖 C）
  T2-B  入口趟命令级环境快照
  T2-C  单文件 Session 接同一机制

批次 E/F/G（依赖 D 的表示定型；拆 3 批）
  T1-B  磁盘模块产物（刀 2）+ T1-C 失效契约
  ── 消费者：T4-B（CLI build 吃产物）

批次 H（条件触发）
  T4-C  LSP 放开并发
  T2-D  子部分复用（重开门条件见 §3.4）
```

### 6.2 批次表（内容 / 判据 / 冲突面）

| 批次 | 内容 | 收口判据（缺一不算） | 冲突面（同一时间只许一个写者） |
|---|---|---|---|
| **A** | T1-A + T2-B0 + T3-B1 + T4-A + T4-B0 | `a3_cross_entry_module_reuse` 3→2 · `session_reuse.rs` 去 `#[ignore]` 翻绿 · 改依赖必 miss · 派生表重建数第 2 刀起 = 0 · 影子档 `diff=0` · 全语料 `--json` 逐字节 · 指针同一性单测 | `crates/front/src/project/{mod,session}.rs`、`crates/front/src/by.rs`、`crates/front/src/judge.rs`、`crates/front/tests/a3_*` |
| **B** | 合成趟计数器 + T3-B2 + T3-D | 计数器先红后绿（合成趟 → 0）· 影子档 `diff=0` · `--json` 逐字节 · 指针单测 · 反向验证 | `crates/front/src/judge.rs`、`compile/check/{mod,walk}.rs` ⚠ **内核线 WIP 同片地** |
| **C** | T2-A | 克隆成本结构计数 · 指针同一性两向 · 内核 63/63 · `kernel-diff.sh --fast` 零差异 · 全语料 `--json` 逐字节 | `crates/kernel/src/{env,util,builder}.rs` |
| **D** | T2-B + T2-C | "elaborate 命令数"：改最后一条 → 1 · 既有 keystroke 断言不放松 · 反例（改依赖回 N） | `crates/front/src/{session,query}/*`、`compile/check/*` |
| **E** | T1-B 批 1（内核 writer + 装载口 + 进程内往返） | 往返等价判据（装载后编同一条声明 ⇒ 逐字节同）· 指针单测 | `crates/kernel/src/*` |
| **F** | T1-B 批 2（落盘 + 三道完整性）+ T1-C | 跨进程 `compiled` 下降 · 反例必 miss · 损坏当不存在 | `crates/front/src/project/cache.rs`、`crates/front/src/compile/cache.rs` |
| **G** | T1-B 批 3（并发/离线/`--clean`）+ T4-B | 离线静默回退 · `--clean` 两处都清 · `build.summary.hit` | `crates/cli/src/build.rs` |
| **H** | T4-C / T2-D（条件触发） | 见各条 | `crates/lsp/src/lib.rs`、`crates/front/src/by.rs` |

> **每条任务的默认验证纪律**（`AGENTS.md`，照抄）：日常 = `scripts/dev-verify.sh`（0.3s）+
> 该处复现件；环节收尾 = `scripts/soko gate --fast`；**全量 `cargo test --workspace` /
> 整本课程 `check.py` / 语料对拍只在发版大节点跑一次**；判据一律**结构计数或同 run 比值**，
> 绝对毫秒只兜数量级。

### 6.3 与并行线的归属（避免重复劳动 / 冲突）

| 并行线的项 | 状态 | 与本文的关系 |
|---|---|---|
| A5 产物命中预热库层检查点 | **已落地** | 是 ② 的一个消费者；本文不再立任务 |
| A2a 就地路接管 `needs_explicit` | **已落地** | `judge_infer` 的 `prefix=0` 由此而来（§4.1 P3-3） |
| A6/A6b/A7（内核热路径） | **已落地** | 与本文四主题正交；判据格式（结构计数 + 反向验证）被本文沿用 |
| **A3 模块级产物 / `module_keys` 接线** | 决策门已过（**值得做**）；设计文档已**收窄**为"刀 2 是新机制、建议独立立项"（`incremental-environment.md:1176-1190`） | = **本文 T1-A**（**会话内跨闭包**那一半，判据 `a3_cross_entry_module_reuse` 3→2）+ **T1-B**（跨进程那一半）。⚠ 设计记录说"会话内只剩同一闭包回头"（已由多槽 LRU 解决）；但 `a3_cross_entry_module_reuse` 钉的是"**同进程、不同闭包、共享模块**"（MainA → MainB）⇒ **这一片既没被 LRU 吃掉，也没被那条记录点名**，是 T1-A 的正身 |
| A2b「judge 合成编译复用调用方活环境」 | 决策门记"不做" | = **本文 T3-B2**（架构件，**重开门后**再做）；**今天可做**的是 **T3-B1** 三条纯接线；本文 §4.3 说明那道门量的是 `prefix=`（只数 `judge_infer`） |
| A4b telescope memo | 按数据不做（3.2% 天花板） | 本文同意不做；不进任何批次 |
| B1/B3（失败块前缀目标 / 续跑） | 已落地 | §3.4 的"成本已压到 2 趟"据此 |
| B4 逐 tactic 前缀快照缓存 | 决策门关 | 本文 §3.4 给出**重开门条件**与**形态更正** |
| B2（光标语义）/ C1–C4（UX）/ E1–E4（prelude） | 与四主题正交 | 本文不重复；批次编排以那份为准 |

---

## 7. 工作量汇总

| 任务 | 规模 | 预计环节 | 批次 | 主要风险 |
|---|---|---|---|---|
| T1-A | M | 2–3 | A | 键/指针同一性 |
| T2-B0 | S | 1（计数器）+ 1（复用） | A | 低（派生表） |
| T3-B1 | M | 2（①②）+ 1（③） | A | 中（判定热路径；影子档必需） |
| T4-A | S | 1 | A | 无（文档 + 一条守卫） |
| T4-B0 | S | 1（先量）+ 1（若做） | A | 低（分支已在） |
| T3-B2 | L | 4–5 | B | 判定热路径 + 内核线冲突 |
| T2-A | XL | 5–7 | C | 内核数据结构 |
| T2-B/C | L + M | 5–6 | D | 环境身份进判据 |
| T1-B | XL | 9–15（3 批） | E/F/G | 内核 API + 跨进程信任边界 |
| T4-B | L | 2–3 | G | 与 T1-B 同片地 |
| T4-C / T2-D | L / XL | 条件触发 | H | 并发 / 决策门 |

**合计**：**约 32–48 个 commit（≈7 个批次）**，其中**批次 A 全是"接线级"**
（进程内 per-module 复用 · 两张派生表 · 三条就地接线 · 分路契约 · CLI 开关），
**判据全部现成**；**B 消合成趟**；**C–D 是真正的架构件**（持久化声明表 + 命令级快照树）；
**E–G 是跨进程产物**。⚠ 工作量按"**环节 = 一个 commit + 它自己的判据**"计，
**不含**发版点（那是另一条线）。

**建议的"最小可信第一步"**：批次 A 的 **T1-A** —— 它零内核改动、键已在手、
判据文件已存在且写着"唯一正确出口是把 3 改成 2"，是全部任务里**风险最低、判据最硬**的一件；
**T2-B0 与 T4-A 可以同批并行**（文件不冲突：`project/session.rs` vs `docs/`）。

---

## 8. 不做清单（含重开门条件）

| 不做 | 理由（出处） | 重开门条件 |
|---|---|---|
| **B4 逐 tactic 前缀快照缓存** | `incremental-environment.md:1164-1171`：最多省 1 趟 + 错键 = 静默错判 + 判据贵 | §3.4 三条任一；且**按 Lean 形态**（结构相等 + 活快照），不按 B4 的哈希形态 |
| **A2b（`judge_infer` 那一半）** | `incremental-environment.md:1172-1175`：`prefix=0` ⇒ 前提不成立 | `prefix > 0` 重新出现 |
| **A4b telescope / 签名级 memo** | `PLAN-cli-editor-perf` §8.12：天花板 ~3.2%，键还要含记法表纪元（否则 = 错键红线） | 出现 telescope 解析占样本 ≥10% 的读数 |
| **照抄"语法逐字节相等当复用判据"** | perf-lean4 §4.5①：本仓库连入口命令布局都要重算，`importless_source` 会重建字符串 | 先有 ② 的环境复用，再谈语法级复用 |
| **照抄"编辑点之后全部重编"** | perf-lean4 §4.5②：S6 依赖脏集已领先（改无人依赖的一条 ⇒ 重查 1 条） | **永不**（这是本仓库领先的地方） |
| **真 mmap 项图** | §2.4 T1-B：载荷是指针图，必须落 arena | 产物格式定型后，且只对"索引/表"做 mmap |
| **照抄 Lean 的 CLI/服务器分路（CLI 丢快照）** | 我们的 CLI 本来就没有快照可丢 —— 分路是**结果**；要的是把契约写清（T4-A） | — |

---

## 9. 未取证 / 不确定（**不许当结论用**）

1. **本文没有新测墙钟**。引用的 87% / 8.4% / 17.7% 全部来自 `PLAN-cli-editor-perf` §8.12
   的 sample（**另一份构建**，`HEAD fe5f4ce9` 之后）；§33.1 的
   `2248ms / prefix=5` 是 **`HEAD 265e5787` + debug LSP** 的读数。
   ⇒ 它们只用来**定位**，**不**用来验收；验收一律用结构计数（§6.2 每条都写了）。
2. **`DeclarMap` 克隆的实际代价没有实测**：`FxIndexMap` 的 O(n) 是从类型读出来的
   （`crates/kernel/src/env.rs:256`），**没有**量过"一次库层检查点克隆花多少"。
   T2-A 的第一件事就应该是**建这个读数**（先建先红）。
3. **`EnvBuilder::Clone` 在库层检查点上的开销没有单独读数**：G-29 的两臂读数是
   端到端（`edit.modules` / `edit/cold` 比值），克隆本身没有分离出来。
4. **合成趟的精确结构计数今天不存在**：`MODULE_COMPILES` 把 judge 合成编译也算进去
   （`incremental-environment.md:1085-1091` 的量具更正）⇒ T3-B2 的**第一个环节必须是补计数器**，
   否则它没有判据（这是本规划里唯一"判据尚不存在"的任务）。
5. **Lean 侧全是读码 + 作者注释**，没有 profile：`PersistentHashMap` 的 HAMT 常量、
   `#extensions` 的运行时条数、`IO.Promise` 的调度开销都没有实测。
6. **`check_synthesized` 在有 env 时"只含合成命令的 pass"这条未验证**
   （`incremental-environment.md:992` 自己标了"这条未验证"）⇒ T3-B2 的②要先做原型验证。
7. **本文的批次划分没有考虑并行线的在飞改动**（工作树里 `docs/perf/ledger.jsonl`、
   `scripts/perf-check.sh`、新 `scripts/check-json-identity.py` 是**别的写者**的）；
   开工前先看 `git status --short` 与 `docs/ONBOARDING.md` §0.2 的未决项。

### 9.1 三处**已过期的设计陈述**（引用前必须改口，否则会照着不存在的东西做）

| 过期陈述 | 出处 | 今天的事实 |
|---|---|---|
| "`EnvBuilder` 今天**没有 `Clone`**" | `docs/design/module-artifacts.md:66` | `crates/kernel/src/builder.rs:37` 已有 `#[derive(Clone)]`（2026-10-08 · G-29）⇒ 刀 1b 的"卡点"**已消失** |
| "`snapshot()`/`with_env()`（T-K12/K1-b）**零生产调用**" | `docs/design/incremental-environment.md:1179`（原文） | **只对 `snapshot()` 成立**；`with_env()` 有 7 处生产调用（`elab.rs:3052`/`:3194`/`:6987`/`:7016`、`level_exit.rs:170`、`kernel_phase.rs:216`、`walk.rs:352`） |
| "LSP 带 `import` 的文档整个绕开 `QueryDoc` 的会话增量 ⇒ **服务器跑的就是 CLI 那条冷路**" | `docs/notes/perf-lean4-interactive-incremental.md:311`/`:330-332`（0.82.0 写的） | **半真**：它确实绕开**单文件 `Session`**，但走的是**项目 session** + 线程局部检查点（`query/mod.rs:653`/`:662`）⇒ "等于 CLI 冷路"**不成立**；"把检查点交给 `QueryDoc`"也**已被推翻** |

---

## 10. 登记（本文档自身）

* **登记状态（已由并行线完成 ✓，本文作者无需再动）**：`scripts/docs-budget.json` 的
  `frozen` 已有 `docs/notes/PLAN-align-lean4.md = 828`，`layer_max_lines.L2 = 12283`
  （提交 `c43aa1a0`，按 `_comment` **例外①** 走）；`scripts/docs-expiry.json` 登记
  `expires 2026-11-07 / tier process`。⇒ **本文上限 = 828 行，只许减不许增** ——
  要再加内容，先按同一例外改 `frozen`（评审可见），**别靠抬 L2 上限** ✗。
* **上游关系**：本文**不替代** `perf-lean4`（Lean 侧机制）与
  `PLAN-cli-editor-perf`（UX 与已量出的瓶颈）；三者的分工见 §0 与 §6.3。
* **开工前必读顺序**：本文 §0.2（四处对账）→ §6.3（归属）→ §9.1（三处过期陈述）
  → 目标主题那一节；**不要**从 §1 的 Lean 机制表直接开工（那张表是参照系，不是任务单）。

---

## 11. 执行进度与读数（as-built · 每轮追加）

> **纪律**：只追加**读数与落点**（计划段不动）；每条自带**构建身份**（探针纪律）。
> 绝对毫秒只作**同机前后**比较，判据一律结构计数。

### 11.1 第 1 轮（2026-10-08/09）—— 探针 + T4-A

* **按键墙钟探针已落地**（本轮新增；此前只有"同进程 `LspService`"与"合成夹具结构计数"
  两条路，都不是端到端）：`crates/lsp/tests/perf_keystroke_wallclock.rs` ——
  **真 `sokonanoda-lsp` 子进程 + 真 stdio 客户端 + 真课程 unit08**，量
  `didChange` → `publishDiagnostics`（= 用户看到 solved 那一刻）；输出 PERFJSON +
  **构建身份**（`lsp-mtime`）+ 服务端自报 `compile_ms`（把"编译"与"调度/防抖"拆开）。
  跑法：`cargo test -p sokonanoda-lsp --test perf_keystroke_wallclock -- --test-threads=1 --nocapture`。
* **基线读数**（debug 构建 · `lsp-mtime=1791471562` · 产物命中臂）：

  | 场景 | 墙钟 best / median / worst | 服务端 `compile_ms` | modules / prefix / by |
  |---|---|---|---|
  | **开档后第一刀**（不等 A5 预热） | **352.4ms** | **227** | 1 / 0 / **81** |
  | 稳态一刀 · 改证明 | 15.7 / **78.6** / 230.9ms | 91 | 1 / 0 / **9** |
  | 稳态一刀 · 改陈述 | 15.3 / 77.0 / 222.7ms | 80 | 1 / 0 / 9 |

  ⇒ **北极星说的"热按键约 300ms"= 开档后第一刀**（本机 352ms），而且它**整笔是入口趟
  重 elaborate**（`by=81` → 稳态 `9`；库层早已命中 ⇒ `modules=1`）✓ —— 与 §3.1 的
  P2-2 逐字对上。推论：**②（入口命令级复用 T2-B）是唯一能打这一刀的结构件**；
  ①（模块产物）与 ③（judge 合成趟，本轮读数 `prefix=0` ⇒ A2a 已把它挡在门外）
  对这一刀只有间接贡献 ⇒ **T2-A→T2-B 的优先级按本轮读数上调**。
  ⚠ **与 §8.1（`PLAN-cli-editor-perf`）的旧读数不可比**（那份 341/766ms 是
  A2a/A5/A6/A7 **之前**的构建；探针纪律：不同构建不并排比）。
* **T4-A 落点**：契约守卫 `crates/front/tests/cli_lsp_split_contract.rs` ——
  ① CLI 路（`compile_project`）`lib_checkpoint_arenas_leaked()` **恒 0** ✓；
  ② **判别力臂**：LSP 路（`QueryDoc` 项目编译）**≥ 1** ✓（没有这一臂，①的 0 只是
  "计数器没接上"的假绿）；
  ③ 两臂模块状态一致（分路只该影响"哪条路跑"，不该影响判定）。
  **反向验证（已做）**：把 `compile_project` 临时拨到
  `compile_plan_incremental(…, reuse_library=true)` ⇒ 守卫**判红** ✓（随后已还原，
  `git diff crates/front/src/project/mod.rs` 为空）。
* **并行线归属（开工时 `git status --short` 是干净的，本轮中途变成脏）**：
  `crates/front/src/{judge,by}.rs` 由**另一写者**在飞（T3-B1 ①：`judge_infer_inplace_with_explicit`
  + `by.rs` 的 `cases` 接线）⇒ 本轮**不碰**这两个文件；批次 A 里①（T1-A）的文件面
  （`project/{mod,session}.rs`）此时空闲。

### 11.2 T1-A 的设计约束（本轮核实 · **下一轮照这个做，别再重新发现**）

把库层趟**切成逐模块趟**才能拿到"模块边界的环境快照"（`finish_pass` 之前 walk 已
`add_declar`、但**内核拒收**的声明要到 `finish_pass` 才移除 ⇒ 中途快照不合法）。
切了之后**逐字节等价**要同时给齐下面五样（少一样就是静默错编面）：

1. **prelude 形状**：`install_all_preludes` 按**整条闭包**判让位（`prelude_shape` =
   `explicit_nat`/`explicit_bool`/`shadowed`）⇒ 复用前缀检查点前**必须比对**
   `PreludeShape`（现成的 `PartialEq` ✓）；形状不同就 miss（回退整条重编）。
2. **prelude 安装上下文**：`taken` 也取整条闭包 ⇒ 逐模块趟要多一个"安装上下文"参数。
3. **闭包身份种子**：`run_pass_with` 的 `closure_acc` 目前只在本趟内累加 ⇒ 逐模块趟
   要显式喂"前缀单元的身份"，否则 judge 缓存键**变宽**（可能错命中）。
4. **三张派生表的累计 override**：`template_closure` / `closure_prefixes_override` /
   `display_override` / `defs_override` 都有现成入口 ✓ —— 逐模块趟分别给
   `units[0..=i]`、`accumulate(units[0..i])`、`display_notations(全库)`、
   `top_level_def_spans_over(全库)`（后两张**整库**才对得上今天的行为）。
5. **计数与合并**：`out`/`reports`/`n_commands`/`prefix_commands` 逐模块累加 + 偏移，
   `run_entries` 只认最终那份（`lib.out` 的形状不变）。
6. **不等式（min/max/上界）**：只有**度数 ≥2** 的模块值得留检查点（否则 O(N²) 条目），
   上界沿用 `MAX_LEAKED_LIB_ARENAS` / `MAX_REUSES_PER_CHECKPOINT` ✓。

### 11.3 下一轮队列（按本轮读数重排）

1. **T3-B1 ①（已在飞 · 另一写者）** —— 本轮不碰；落地后按它的判据（影子档 `diff=0`）复核。
2. **T1-A** —— 文件面空闲；按 §11.2 的六条做，判据照 §2.4（`a3` 3→2 等）。
3. **T2-A → T2-B** —— 按 §11.1 的读数，这是**唯一能打"第一刀 352ms"**的那条线。
4. T2-B0 / T4-B0 —— 仍可做，但**优先级下移**（本轮读数显示它们不在要紧的那条路上）。

### 11.4 T3-B1 ①（`cases` 被消去项的就地路）—— **已落地**（`898b9718`）

* **落点**：`judge::judge_infer_inplace_with_explicit`（受控入口 · `EXPLICIT_PP` 与慢路
  **同源**）+ `by.rs::cases_tactic` 就地优先、答不出**原样**回落 `judge_infer_explicit`。
  新增**专用**读数 `inplace_cases_report()` / `inplace_cases_shadow()`（**不与**
  `judge_render_type` 那一档混 —— 混了就分不清哪条接线生效 ⇒ 判据空转 ✗）。
* **判据**：① `crates/front/tests/judge_inplace_cases.rs`（独立进程）影子档
  `same=1 · diff=0` ✓；② `SOKO_JUDGE_INPLACE_BY=0` vs 默认的 `--json` **逐字节相同**
  （夹具 + `unit04-extensionality-identities` + `unit05-solution` +
  `unit07-solution`）✓；③ 成本（同机同夹具）：`JUDGE_INFER calls 16→10 ·
  total_ms 73→15 · misses 6→2` ⇒ **少 4 趟合成前缀** ✓；④ `--lib` **875/875** ·
  就地家族 8/8 ✓。
* **对北极星零贡献**（诚实记）：§11.1 的探针显示要紧的是**开档后第一刀**（352ms ·
  `by=81` 入口趟重 elaborate），而 `cases` 全课程仅 **3 处** ⇒ 本条消的是方向③的 long tail。
* **T3-B1 ② 已由 A2a 完成**（§4.4 把它列成"今天可做"是**过期** ✓）：`judge.rs:2369` 的
  "不走就地快路"是**注释过期** —— 调用点 `by.rs:890` 早已接就地入口（`judge_render_type_inplace_with_explicit`）✓。

### 11.5 归属声明（两写者不撞同一文件 · 2026-10-09 00:12）

* **主会话**已交 `judge.rs` / `by.rs` 的 T3-B1 ①（`898b9718`）；**接着取 T3-B1 ③**
  （同一文件面：`level_hint_of` 的**记法形态**分支，需要"文本 ⇒ AST"入口 ——
  `elab.rs::universe_level_text_of_operands` 已有同形先例 ✓）。
* **T1-A 留给另一写者**（§11.2 的六条设计约束就是它的开工单 · `project/{mod,session}.rs`
  文件面空闲 ✓）；**T2-A→T2-B** 同样归另一写者（§11.1 判定它是唯一能打第一刀的结构件）。
* ⚠ **纪律**：动手前先 `git status --short`；同一文件同一时间**只许一个写者** ✗。

### 11.6 T1-A（进程内 per-module 检查点）—— **已落地**（2026-10-09 · 另一写者按 §11.2 开工）

* **落点**：`crates/front/src/project/session.rs`（`LibCursor` / `run_library_from` /
  `lib_prefix_keys` / `ResumeState`）+ `compile/check/{mod,walk}.rs`（`run_pass_with`
  新增 `snapshot_state` / `resume` 两个**默认关**参数 + `Walk` 在模块边界交出续编状态）
  + `compile/mod.rs`（`install_all_preludes`、`closure_prefixes_and_total`、`OpenEntry`
  的 `pub(crate)` 出口）。
* **机制（与 §11.2 的差异，逐条记）**：
  1. **不是"一趟 + 中途快照"** —— `walk` 无条件 `add_declar`、而内核检查是**整趟一次**
     做的（`finish_pass`）⇒ 要一份**合法**的模块边界环境，只能让那一趟**在那里结束**
     ⇒ 改成**逐模块趟** ✓（`lib_units` 的每个模块各自一趟，`install_preludes` 只在
     循环外装一次、上下文 = **整条库层** ✓）。
  2. **新发现三件（§11.2 只点了第 3 条）**：模块边界上还要带上 **`exports`**（跨单元
     可见性通道，`walk` 在单元切换处重放）与 **`example_idx`**（`_example_N` 是**整趟**
     计数器 ⇒ 不带就会在两个模块里造出同名内部声明 ✗）⇒ 与 `closure_id` 合成
     [`ResumeState`]。
  3. **复用判据多一条**：检查点存**整条库层的** `PreludeShape`，前缀命中时比对
     （prelude 是按**整条闭包**判让位的 ⇒ 前缀相同也可能落在不同 prelude 上 ✗）。
  4. **续编不建检查点**（单元是本次调用的、非 `'static` ⇒ 存不进线程局部 ✗）——
     代价如实记：续编过的库层**不留"整条"检查点** ⇒ 同一条入口**再开**仍走"前缀续编"。
  5. **上界换成 `MAX_MODULE_CHECKPOINTS = 32`（新）**：§11.2 的"只给度数 ≥ 2 的模块留"
     **没做**（度数信息到不了 session ✗）；改用**条目**上界 + LRU。泄漏的 **arena** 仍由
     `MAX_LEAKED_LIB_ARENAS` 封顶 ✓（两个上界各管一头）。
* **读数（结构计数优先）**：
  | 判据 | 落地前 | 落地后 |
  |---|---|---|
  | `a3_cross_entry_module_reuse`（`MainA → MainB` 共享 `Common`） | 3 | **2** ✓（断言已按文件头的唯一出口翻） |
  | `lsp_checkpoint_multi_slot`（编辑器层同形场景） | 3 | **2** ✓（同一个 LSP 进程、同一条编译线程） |
  | **跨入口切换探针**（真课程 `unit08 → unit09`） | 冷开 = **8** | **5**（复用 **3**）✓ |
  | 热按键（`perf_keystroke_wallclock`） | median 78.6ms / 第一刀 352.4ms | **不变**（78.2 / 375.6ms · **构建已变 ⇒ 不并排比** ✓） |
* **判据（已跑）**：`cargo test -p sokonanoda-front` **28 个测试二进制全绿**（含
  `a3` / `g29` / `a4a` / `keystroke_structure`）· `cargo test -p sokonanoda-lsp`
  **9 个全绿** · **新增** `crates/front/tests/t1a_module_checkpoint_parity.rs`
  （两臂**逐字对拍**：CLI"整条一趟" ↔ LSP"逐模块/续编"，含"续编不泄漏新 arena"的判别力臂 ✓）·
  `scripts/dev-verify.sh` 结构计数**逐字相同**（CLI 路没动 ✓）。
* **对北极星的贡献（诚实记）**：方向①要的是"**换单元不重编共享库层**"——
  真课程实测 **8 → 5 个模块**；但那一刀的墙钟（1167ms）**主要不是库层**（`by=127` =
  unit09 入口趟第一次全 elaborate）⇒ **要紧的仍是 §11.1 的"入口命令级复用"（T2-A→T2-B）**。

### 11.7 T3-B1 ③（`level_hint_of` 的记法形态）—— **已落地**（`18ac57e8`）

* **落点**：`by.rs::level_hint_of_inplace` 补**记法分支** + 新助手 `check_ast_of_text`
  （文本 ⇒ AST，与 `elab.rs::universe_level_text_of_operands` 的第二问**同形** ——
  不是新机制 ✓）；新增**专用**读数 `inplace_level_hint_report()`（不与别的档混 ✓）。
* **判据**：① `crates/front/tests/judge_inplace_level_hint.rs`（真课程 unit08）
  `used > 0`（**判据不空转** ✓）；② `SOKO_JUDGE_INPLACE=off` vs 默认的 `--json`
  **逐字节相同**（I.1/I.2/I.3 抽 **24 份** `cmp` 全同）✓；③ `BY_LEVEL_HINT_MISS`
  unit08 冷编 **3 → 0**（Off 档 16 ⇒ On 档 0）；④ `--lib` **875/875** · 就地家族全绿 ✓。
* **对北极星的贡献**：只对**冷 judge 缓存**那一刀有效（3 趟 × ≈20ms ≈ 60ms 量级）；
  稳态按键本来 `prefix=0`（缓存热）⇒ 零影响。§11.1 探针**同构建**复跑：第一刀
  **341.2ms**（`compile=213 · by=81 · modules=1 · prefix=0`）——
  ⚠ 与 §11.1 的 352.4ms **不同构建**（`lsp-mtime` 1791471562 → 1791477602）⇒
  **不并排比** ✓，只作"结构计数一字不变"的证据。
* **方向③的状态**：T3-B1 ①②③ 全落地（② 早已由 A2a 完成 ✓）⇒ 计划 §4.4 的"今天可做"
  三条做完 ✓；剩下的合成趟消去是 **T3-B2**（架构件 · 需重开门 + 内核线让位）与
  **T3-D**（`EnvProvider` 处置）。
* **独立对拍（2026-10-09 另一会话 · T1-A + T3-B1 ①③ 一并验）**：
  `python3 scripts/check-json-identity.py --baseline target/release/sokonanoda
  --new target/debug/sokonanoda --sample 3` ⇒ **101/101 逐字节相同 · 0 处不同 ·
  答得上 100/101（99%）** ✓（基线 = `0.86.0` 发布前的 release 构建 ⇒ 覆盖
  T1-A 的逐模块趟 + 本条的就地路）。

### 11.8 T3-B2 的读数（合成趟）—— **先建读数已落地**（`78f45e21`）· **T3-B2 暂不开工**

* **为什么先建**（§4.3 的对账）：`prefix=`（`PREFIX_RUNS`）**只数 `judge_infer`** ✗、
  `MODULE_COMPILES` 也不数（合成趟走 `run_incremental` ✗）⇒「合成趟还剩多少」没有出口 ✗
  ⇒ T3-B2 没有判据。
* **落点**：`judge::synthesized_report() -> (趟数, Σ命令数, 回退趟数)`
  （回退 = 没有担保 ⇒ 整份重查，比合成趟更贵 ⇒ 分开数 ✓）；判据文件
  `crates/front/tests/judge_synthesized_report.rs`（真课程 unit08 · 断言不空转 ✓）。
* **读数**（同机 · debug · unit08 · `QueryDoc`）：
  | 相位 | 趟数 | Σ命令数 | 回退 |
  |---|---|---|---|
  | 冷开（首次 `set_text`） | **37** | **1369** | 2 |
  | 第 1 刀（改证明体） | 4 | 302 | 0 |
  | **第 2 刀起** | **0** | **0** | 0 |
  逐声明 profile 复核（另一会话的 scratch）：第 1 刀那 4 趟 ≈ **51ms / 191ms**；
  调用点 = `judge_type_of`（`elab.rs:1360/2078/2135/2234` 的记法/建议签名）。
* **门槛判定（按 §4.4 的重开门条件）**：条件原文是"**量出合成趟仍是主要成本**再开工"。
  实测**稳态 = 0**、只在冷开与第 1 刀出现（≈15%）⇒ **条件不成立** ⇒
  **T3-B2（架构件 · 与内核线同片地）不开工** ✓。要紧的仍是 **T2-A→T2-B**
  （入口趟；profile 显示冷开 `by=81`、稳态 `by=9`）。
* **仍未测**：`judge_pairs`（by 路径）的合成趟占比 —— 上面的 37/4 趟里
  `judge_type_of` 是主力（trace 证据），`judge_pairs` 这一档留待有读数时再判。

### 11.9 第 3 轮（2026-10-09）—— **北极星读数修正：真实"连续键入"是 ~334ms（不是 78ms）**

* **探针补两臂**（`crates/lsp/tests/perf_keystroke_wallclock.rs`）：`typing`
  （每一刀都是**新文本**：在证明体里**插入一个空格** ⇒ 后缀字节起点**平移**）与
  `typing_equal_length`（每一刀**也是新文本**，但**等长**改名 ⇒ 起点**不动**）。
* **读数**（同机同构建 · 真课程 `unit08` · 真 stdio LSP · `[profile.test]` opt-level=3）：

  | 臂 | 每刀新文本？ | 后缀起点 | `by` | 墙钟 median | `compile` |
  |---|---|---|---|---|---|
  | **`typing`（插入字符）** | ✓ | **平移** | **81** | **333.8ms** | 204ms |
  | `typing_equal_length`（等长改名） | ✓ | 不动 | 18 | 102.1ms | 101ms |
  | `proof` / `statement`（两文本来回） | ✗（**复用旧文本**） | 平移 | 18 | 139 / 152ms | ~140ms |

  ⇒ **§11.1 与 §11.8 引用的"稳态 `by=9` / 78ms"是测量假象** ✗：那两条臂在**两份文本
  之间来回** ⇒ 第二刀起命中的是**第一次就喂热的缓存**。**真实连续键入 = 266–350ms**
  —— 与北极星那句"当前热按键约 300ms"**逐字对上** ✓（本轮把它钉成了可判据的读数）。
* **贵因（本轮 A/B 定位，不是猜的）**：
  1. 不是"库层没复用" —— 同刀 `modules=1` ✓（G-29/T1-A 都在 ✓）；
  2. 不是"信任前缀有没有开" —— `SOKO_NO_ENTRY_TRUST=1` 与默认**逐字相同**
     （`by=81` · 218ms vs 223ms）✗；
  3. **是插入/删除字符把后缀的字节起点全推了位**：`EntryCache::trusted_prefix`
     要求"命令文本**与起点**逐条相同"⇒ 改动点**之后**的命令**全部判脏** ✗
     ⇒ 后缀的 `theorem` 走不到 `walk` 的"`trusted` ⇒ `build_axiom`（**只重建类型、
     跳过证明体**）"那条快路 ⇒ `by=81`（9 个 `by` 块的 tactic 全跑）。
  对照：`typing_equal_length`（起点不动）⇒ `by=18` · 102ms ✓ —— **唯一差别就是起点**。
* **A5b 的尝试与撤回（如实记）**：本轮先试了"后台把**入口趟**也预热"
  （`front::project::warm_entry_checkpoint`）。**读数不支持**：探针那臂 `warmed=false`
  （真课程开档**不是产物命中** ⇒ 预热压根不触发），且 `lsp_artifact_warmup.rs` 的
  "立刻敲不许双付"守卫判红（预热 + 编辑 = 3 个模块编译 > 2）⇒ **已撤回**
  （`git checkout`；守卫回绿 ✓）。⇒ 打的是"偏移平移"那一刀，不是调度。
* **下一轮目标（写死）**：让 `EntryCache` 的信任判据**对纯插入/删除稳健** ——
  比较**命令文本**（放过"整体平移 δ"），并把拼回的缓存报告里 span **整体平移 δ**
  （否则诊断位置错 ✗ = 用户可见红线）。**预期**：`typing` 333.8ms → ≈ 102ms
  （= `typing_equal_length` 那一臂 ⇒ **已超过 Lean 4 的 ~200ms** ✓）。
  **判据**：① `typing` 臂 `by` **81 → ≤18**；② **新增**一条钉"插入字符后诊断仍落在
  正确行/列"的判据（span 平移的守卫）；③ 反向验证（撤掉平移判据 ⇒ 回到 81）；
  ④ 全课程 `--json` 逐字节不变。

### 11.10 第 4 轮（2026-10-09）—— §11.9 的那一刀**试了、不成立**（**已撤回** · 负结论留档）

* **做了什么**（都**未提交**、随后 `git checkout` 撤回 ✓）：`EntryCache::trusted_prefix`
  与 `dirty_commands` 改成**只看文本**（放过整体平移 δ）；`splice_entry_report` 给
  搬回来的每条 span **按 δ 平移**（`shift_decl`/`shift_hover`/`shift_error`/
  `shift_warning`，行/列按**新文本**用 `pos::line_col_of` 重算 ✓）；并把缓存里
  `Checked` 的名字表交给 `TrustPlan::trusted_entered`（让受信**定理**走
  `build_axiom` 那条"只重建类型、跳过证明体"的路 ✓）。
* **判据替我否掉了它（这是本轮唯一有价值的产出 ✓）**：新增的
  `crates/front/tests/t2b_shifted_splice_parity.rs`（**拼接臂** vs **冷编臂**逐字节对拍）
  **判红**，第一个分歧点就是根因：
  `HoverType.resolution` 里的 `Declaration { span }` **可以指向别的文件**（库模块
  `Common.sokonanoda` 里的 `P`：offset `0..14`）—— 把**入口的 δ**（+5）加到它上面
  就把它挪到了 `5..19` ✗✗。**报告里没有文件信息** ⇒ "这条 span 该不该平移"根本判不出来
  ⇒ 想走这条路必须先有**文件感知的位置模型**（或换一条不给 resolution 用缓存 span 的路）。
  ⚠ 反向验证也做了：把 δ 钉成 0 ⇒ 同一条判据**同样判红** ⇒ 语法上它有牙 ✓（不是空转）。
* **性能上也没兑现**：那套改动跑起来 `typing` 臂只从 `by=81` 掉到 **63**（不是 ≤18），
  `compile` 仍 ~325ms ⇒ **§11.9 预期的 "334 → ~102ms" 不成立** ✗。剩下的那一大笔不在
  "信任"上（候选：**judge 的合成趟** —— 插入字符改了**改动点之后的 prefix 文本** ⇒
  它们的缓存**全 miss** ⇒ 合成趟重跑；平行线 §11.8 实测第一刀 4 趟 ≈ 51/191ms）。
* **⇒ 撤回 + 两条重排**：
  1. **§11.8 的 T3-B2 门槛要在 `typing` 臂上重量**：那次读数（冷 37 / 第 1 刀 4 / 稳态 0）
  用的是**两文本来回**的臂 ⇒ 与 §11.9 的"稳态 78ms"是**同一个假象** ✗。**真实连续键入**
  下合成趟有多少，谁都还没量过 —— 量出来再决定 T3-B2 开不开工 ✓（它的门槛本来就是这个）。
  2. **T2-B（入口命令级环境快照）仍是结构正解**，但**先要文件感知的 span 模型**
  （或让受信条目**不搬 resolution**）—— 这是它此前没被记下的**前置件** ✗。

### 11.11 第 5 轮（2026-10-09）—— **T3-B2 的门槛在 `typing` 臂上是开的**（§11.8 的"稳态 0 趟"是假象 ✗）

* **读数**（`crates/front/tests/judge_synthesized_typing.rs` · `QueryDoc` = LSP 每次
  `didChange` 走的那条路 · `judge::synthesized_report()` 逐刀差量 · 真课程 unit08）：

  | 臂 | 每一刀的文本 | 逐刀趟数 | Σ趟数 | Σ合成命令数 |
  |---|---|---|---|---|
  | `alternating`（**两份文本来回** = §11.8 那次用的形状） | 旧 | `[3,0,0,0,0]` | **3** | 224 |
  | **`typing`（每一刀都新文本 = 真实做题 ✓）** | 新 | **`[4,4,4,4,4]`** | **20** | **1510** |
  | 冷开（参照） | — | — | 21 | 880 |

  ⇒ **真实连续键入下，每一刀都跑 4 趟合成、重编 302 条合成命令** ✓ —— 而 §11.8 判"门槛
  不成立"用的正是 `alternating` 那一臂（后四刀恒 0 趟）✗。
  ⚠ 与 §11.9 的"稳态 78ms"**同源**：都是"两份文本来回"偷来的假象。
* **门槛判定（按 §4.4 的重开门条件 · **改判**）**：条件原文是"**量出合成趟仍是主要成本**
  再开工"。`typing` 臂每刀 4 趟 / 302 条命令，且平行线 §11.8 已实测那 4 趟 ≈ **51ms / 191ms**
  （占一次按键 334ms 的一大截）⇒ **条件成立** ⇒ **T3-B2 开工** ✓✓。
* **下一步（写死）**：那 4 趟的调用点 = `judge_type_of`（`elab.rs:1360/2078/2135/2234` 的
  **记法/建议签名**）⇒ 与 T3-B1 第 5 条（`type_of_constant`，**已落地** ✓）**同一族**：
  把"合成一份文档"换成"**就地查活环境**"（`InplaceEnv` 那套现成机制 ✓）。
  **判据**：① 本文件的 `typing` 臂 Σ趟数 **20 → 0**（断言已写死指向这个出口 ✓）；
  ② 影子档 `diff=0`（沿用 T3-B1 的口径 ✓）；③ 全课程 `--json` 逐字节不变；
  ④ 按键墙钟（探针 `typing` 臂）改善 —— 这是**方向③**对北极星的直接读数 ✓。



### 11.12 第 6 轮（2026-10-09）—— **T3-B2 落地：真实连续键入 334ms → 81.7ms**（**北极星读数达标** ✓）

* **谁落的**：平行线（本线第 5 轮把门槛改判为开 ⇒ 他们那一族的第 5 条顺势把**剩下三处**
  `judge_type_of` 也换成 `type_of_constant_prefer_inplace`，共 4 处 = 记法/建议签名那 4 趟）。
  本线做的是**验证 + 北极星读数 + 出口断言翻转** ✓（`AGENTS.md` 的"产出必须验证后才并入"）。
* **门槛读数（本线第 5 轮那条判据 · §11.11 的 `typing` 臂）**：

  | 相位 | 落地前 | 落地后 |
  |---|---|---|
  | `typing`（真实连续键入 · 每刀新文本）逐刀趟数 | `[4,4,4,4,4]` Σ=**20** | `[0,0,0,0,0]` **Σ=0** ✓ |
  | 同臂 Σ合成命令数 | 1510 | **0** ✓ |
  | `alternating`（旧假象臂） | Σ=3 | Σ=0 |
  | 冷开（参照 ⇒ 计数器没空转 ✓） | 21 趟 / 880 条 | **15 趟 / 505 条** ✓ |

  ⇒ `crates/front/tests/judge_synthesized_typing.rs` 的断言按当时写死的出口**翻转**成
  `typing_passes == 0` ✓（**不许放宽** ✗ —— 回落到每刀 4 趟必须去修）。
* **北极星读数（`crates/lsp/tests/perf_keystroke_wallclock.rs` · 同机 · 真 stdio LSP ·
  course unit08 · 构建 `lsp-cargo-mtime=1791479842`）**：

  | 臂 | 第 3/4 轮 | **现在** | `by` | `compile` |
  |---|---|---|---|---|
  | **`typing`（真实连续键入 ✓）** | **333.8ms** | **81.7ms**（best 77.8 / worst 121.2） | 81 → **9** | 204 → **77ms** |
  | `typing_equal_length` | 102.1ms | 41.2ms | 18 → 9 | 101 → 40ms |
  | `proof` / `statement` | ~140ms | 83 / 82ms | 18 → 9 | ~140 → 83ms |

  ⇒ **"真实连续键入" 334ms → 81.7ms（4.1×）**，对照北极星那句"lean4 约 200ms" ⇒
  **本读数上已经打平并超过** ✓✓。机制归因清楚：`modules=1`（库层复用 ✓ 方向①）+ 
  `by=9`（**合成前缀趟被消灭** ✓ 方向③ T3-B1/T3-B2）+ 无 `prefix` 重跑 ✓。
* **仍要谨慎的三点**：① 这是**单机单单元**读数，且 lean4 那 200ms 是计划里的**参考值**
  （规则禁止调用官方工具链 ⇒ 无法同机对拍）; ② 它量的是"**一次按键**"，不含首个字符前的
  JVM/进程启动（那是另一个量级，见 §8.1）; ③ `statement` 改的是接口 ⇒ 82ms 说明
  依赖传播也没拖后腿 ✓。
* **四方向的账（本轮后）**：① T1-A（会话内逐模块检查点）✓ · 磁盘半件（T1-B）未做;
  ② T2-A（跨条目模块复用）✓ · T2-B（入口命令级快照）**未做**（§11.10 记了它的前置件）;
  ③ T3-B1 ①②③ + **T3-B2** ✓;**④ T4-A** ✓。⇒ 北极星读数已达标，但计划四方向仍有剩余件。
