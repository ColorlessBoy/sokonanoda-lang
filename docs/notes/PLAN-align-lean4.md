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
| **T2-A** | **内核：声明表持久化/COW**（② 的结构前置件；**实体内核**，与 T1-A 的"跨条目复用"无关 ✗） | XL | 无（内核线） | 环境克隆的**每命令条目数** O(1) · 指针同一性单测 · 全语料 `--json` 逐字节 |
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

**T2-A · 内核：声明表持久化 / COW（规模 XL · ② 的前置件）**（**实体内核**；"跨条目复用"是 T1-A ✗）

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

## 11. 执行进度与读数（as-built · **已归档** ⇒ [`PLAN-align-lean4-as-built.md`](PLAN-align-lean4-as-built.md)）

> **2026-10-09 归档**：本节全系列（§11.1–§11.25）**原样搬到**
> [`docs/notes/PLAN-align-lean4-as-built.md`](PLAN-align-lean4-as-built.md) ✓ —— 原因是本文
> 顶到了 `docs-lint` 判据②的**单文件上限 2000 行** ✗。
> **内容一字未改** ✓（只搬不改）；**追加纪律不变**（只追加读数与落点、每条自带构建身份 ✓）。
> ⇒ 下一棒要找**历史读数/落点**去那份文件 ✓；**新的一轮**仍追加在本文末尾 ✓。
## 12. 交接摘要（第 20 轮 · 2026-10-09 · §11 系列有 25 节 ⇒ 这里只留"下一棒要用的"）

**北极星（同机 · 真 stdio LSP · course unit08 · 判据 = `crates/lsp/tests/perf_keystroke_wallclock.rs`）**：

| 场景 | 起点 | **现在** | 参考（计划里的 lean4） |
|---|---|---|---|
| **真实连续键入**（每刀新文本） | 333.8ms | **78.5ms** ✓ | **218ms**（实测 ⇒ 2.7×）✓ |
| **开档就敲**（第一刀） | 205.6ms（118ms 纯等静默期） | **76.8ms** ✓ | 同上 |
| 跨入口切换（unit08→unit09） | 1030–1167ms | **711ms**（`modules=5` · 冷 8 ⇒ 复用 3） | — |

**四方向账**：

| 方向 | 落的 | 未落 |
|---|---|---|
| ① 产物化 | T1-A（会话内逐模块库层检查点 ⇒ 换单元 8→5 模块） | **T1-B（磁盘产物：新模块对着 mmap 环境编）＝ 最大单项**（跨入口 711ms 只剩它可降） |
| ② 服务器热路 | T1-A（跨条目模块复用 = G-29 库层检查点）· **§11.22 静默期只保护"写紧跟写"**（首刀 −126ms ✓） | T2-B（入口命令级快照；前置件 = **文件感知的 span 模型**，见 §11.10） |
| ③ 活环境查询 | T3-B1 ①②③ · **T3-B2**（合成趟 Σ20→0 ✓）· hover 回归两修法（§11.17/§11.18/§11.25） | — |
| ④ CLI/LSP 分路 | T4-A（CLI 路不泄漏 arena 的守卫） | 其余分路项（见 §5） |

**判据与验证现状**：front 34 目标 ✓ · lsp 178/178 ✓ · cli 0 失败 ✓ · `dev-verify` 结构计数与历史逐字相同 ✓ ·
`scripts/soko gate`（全量）＝ 第 19 轮起跑、写日志时仍在 `cargo test --workspace` ✗（见下"作业教训"）。
**红线**：设计档 `docs/design/incremental-environment.md` 记着**全课程 `--json` 逐字节全量 252/252 ✓**（基线 = `v0.85.2` tag 构建、`NO_PROJECT_ARTIFACTS=1` + 每臂全新缓存 ⇒ 抽样 112/112 + 补集 140/140）—— §12 早先写的"101/101"是**旧的局部对拍**，以此处为准 ✓。

**下一棒先做**：① 收 `scripts/soko gate` 的结果（重跑时**原样落盘**，别过管道 ✗）；② T1-B 的前置调研
（内核环境可持久化到什么程度 —— `ArenaRef` 是 `!Send`，先读 `docs/design/incremental-environment.md`）。

**作业教训（都踩过）**：① 探针读数**必须带构建身份**、且**别在"两份文本来回"的臂上量稳态**（§11.9 假象 ✗）；
② 长命令**原样落盘 + 超时 + 轮询**，**不要 `| tail`**（第 19 轮 gate 因此整轮不可观测 ✗）；
③ 改完 `.rs` **跟一次 `fmt --check`**（`cargo check` 不管格式 ✗，§11.24 漏过一次）；
④ 按**注释文本**切片删代码会误删邻接用例（第 3 轮吞掉一条探针 ✗，§11.13 才复原）；
⑤ 提速类改动要问"**谁在靠这次调用的副作用**"（§11.17 的根因就是这个 ✗）；
⑥ 影子档只比"**返回值**"，**拦不住**"谁写了缓存"（§11.18）。

### 13. T1-B 的**归属**已由设计档判定（本轮读档核实 · 不并进本线收尾 ✓）

**读 `docs/design/incremental-environment.md` §8.11 的结论（原文要点）**：

* 磁盘产物里**只有报告**（`cache.rs` 落 `report: entry.report.clone()`）——**不是**内核环境 ✗；
* 内核只有**进程内**的 `snapshot()`/`with_env()`（**零生产调用**）/ `hide|restore_declars` ✓；
  **没有** `ExportFile → EnvBuilder` ✗，CLI 也没有 export/import 子命令 ✓；
* ⇒ 跨进程复用模块**必须新造"环境序列化 + 导入 + 校验/键"**（**新机制，不是接线** ✓）；
* 本线的**会话内**那一半**已经被吃掉** ✓（多槽 LRU 检查点：`MainA → MainB → MainA` 回头那刀
  **3 → 1**，front + LSP 双判据 ✓）；**剩下**的只有**跨进程**那一半（CLI `build`/课程门禁/CI
  一入口一进程 ⇒ 内存检查点无从谈起 = perf ledger L34 的**跨组残余 2.08×**）；
* 设计档的处置是「**建议独立立项**（自带设计：序列化格式 · 版本/键 · 反例判据），
  **不并进本线收尾**」✓。

⇒ **对本线四方向的收口**：① 的**会话内**半件 = **T1-A ✓ 已完成**；**跨进程**半件 = 独立立项，
**不算本线的未完成项** ✓（§11.13/§12 里那句"T1-B = 最大单项"要按此**改判**：它是**另一个项目**
的最大单项 ✗→✓）。**本线真正剩下的只有 ② 的 T2-B**（入口命令级快照）。

**T2-B 下一步要做的不是开工、是先量价值**（同 §11.11 对 T3-B2 的做法 ✓）：
现在每刀 `by=9` · `compile≈77ms` · `modules=1` ⇒ 要先分解这 77ms 里**入口趟**占多少、
其中**"命令级快照能省掉的那一段"**占多少（做法：拿 `SOKO_STAGE_STATS`/逐声明 profile
量入口趟的相位构成，再按"快照命中能跳过的命令数 × 单命令均摊"估上界 ✓）；
上界若 < ~20ms ⇒ **不值得动**（它的前置件是"文件感知的 span 模型"，见 §11.10 ✗）。

### 14. T2-B 的价值读数：**粗粒度先看（by 只占 ~11%），细化读数留给下一轮**

* **读数**（`SOKO_STAGE_STATS=1 cargo test -p sokonanoda-front --test judge_synthesized_typing -- --nocapture`
  —— 该用例 = **冷开 + 5 刀真实连续键入 + 5 刀两文本来回**，所以是**整程总量**不是单刀）：

  | 相位 | 读数 |
  |---|---|
  | `passes` / `pass_total_ms` | 34 / **982ms**（≈29ms/趟） |
  | `by_calls` / `by_total_ms` | 130 / **104ms**（**pass 总量的 ~11%**） |
  | `judge_ms` | 100ms（~10%） |
  | `tc_cache_builds` | **45768**（每趟都重建 `TcCache` —— 已知的预分配成本，`docs/PERF.md` 有账） |
  | `hits=0 misses=13` · `doc_passes=0` | 合成趟已归零（§11.11 ✓） |

* **它对 T2-B 意味着什么**：T2-B 要省的是"**入口命令级快照能跳过的那段 elaborate**"，而 `by`
  只是其中一小块（~11%）⇒ **上界不高**；真正的大头在**每趟都要付的固定成本**（`tc_cache_builds`
  每趟重建、pp、`pass` 骨架）——**那些 T2-B 一个都省不掉** ✗。
* **所以先别开工**（同 §13 的纪律）。**下一轮做细化读数**：照第 3 轮那条路
  （`SOKO_DECL_PROFILE=1 SOKO_DECL_PROFILE_MS=…`）只对**一刀**做**逐声明**分解 ⇒ 得出
  "**未变命令**（= 快照候选）在这 77ms 里占多少" ✓。**判定门槛照旧：上界 < ~20ms ⇒ 不做** ✓。

### 15. 全量 `scripts/soko gate` 收回（第 19 轮起跑 · 第 23 轮拿到结果）：**判红，但不在本线** ✗

* **它到底跑了多久 / 为什么"没动静"**：不是死锁、也不是 cargo 锁（我第 22 轮的猜测**错了** ✗）——
  我 `ps` 看到它在 **课程门禁**那一步（`sokonanoda grade --json courses/set-theory/units/solutions/**`
  逐个单元跑，且用的是 **debug** 二进制 ✗）⇒ **本来就慢** ✓。日志为空是**我把输出管进 `| tail`** ✗
  （§12 教训⑤），跑完才落盘。
* **结果（判红 3 处，逐条归属）**：

  | 判据 | 读数 | 归属 |
  |---|---|---|
  | `infoview-hierarchy` | ✗ `.section-title` 字号 **12px < 正文 13px**（方向反了）· `.section-title` 用 `opacity: 0.7` · `h2` 用 `opacity: 0.9` | **VS Code 扩展 CSS**（`editor/vscode/**`）—— **不是本线**（本线只动 front/lsp/cli 的 Rust + 探针）✗ |
  | `docs-lint` ④ | ✗ `docs/architecture.md` **754 > 冻结 753** | 文档预算（**平行线**在往里加内核线内容 ✗） |
  | `docs-lint` ⑦ | ✗ **L1 层 5675 > 上限 5674**（只许减） | 同上 ⇒ L1 里某个文件长了一行 ✗ |
* **本线自己的部分全过** ✓：`cargo fmt`/`clippy`/workspace test（gate 走到课程门禁说明前三步都过了 ✓）·
  `docs-lint` 的 ①②③⑤⑥⑧ —— 我每次加 §11.x 都**同步冻结预算与 L2**（§11.24/§12/§13/§14 都做了 ✓）
  ⇒ 那两条 ④⑦ 的红**不是本线的 plan/预算改动**（本线的 `PLAN-align-lean4.md` 与
  `scripts/docs-budget.json` 是**自洽**的 ✓）。
* **修法（给归属方 · 都不需要本线动手）**：
  1. **扩展 CSS**：标题字号回正文（`1em`/13px）、降档**只用颜色**（`color: var(--vscode-descriptionForeground)`）、
     标题不许用 `opacity` ✓（依据 `AGENTS.md` 验证纪律第 0 条(b) + G-66）；
  2. **两条文档预算**：要么删一行/归档，要么**手改 `scripts/docs-budget.json`** 重新基线（评审可见 ✓）——
     L1 那 1 行溢出大概率是 `docs/architecture.md` 那条新行引起的**同一次改动** ✓。
* **尾随观察（同轮内）**：写完本节再跑 `scripts/docs-lint.py` ⇒ **已全绿 ✓**（④⑦ 那两条红
  在我复核时**已被归属方修掉** ✓）⇒ **gate 的剩余唯一红 = 扩展 CSS 的 `infoview-hierarchy`** ✗
  （`editor/vscode/**`，本线不代改 ✓）。
* **作业教训（补一条）**：`EXIT=` 取的是**管道最后一个命令**（`tail`）的状态 ⇒ **不能**用它判 gate
  成败 ✗ —— 要么不过管道、要么看门禁自己的收尾行 ✓。

### 16. T2-B 的**量级判定**：上界 ≈30ms/刀，前置件很大 ⇒ **缓做**（不是"不做"）

* **本轮读数**（`SOKO_STAGE_STATS=1` 同一用例，`SOKO_DECL_PROFILE` 这次**没出逐声明行** ✗
  —— 那个档位要么改了名、要么只在 LSP 路径生效，**没量到就别当量到了** ✗）：

  | 相位 | 读数 | 对 T2-B 的意义 |
  |---|---|---|
  | `pass_total_ms` / `passes` | 944 / 34（≈**27.8ms/趟**） | 一趟里"可跳过的命令级 elaborate"最多也就这个量级 |
  | `by_total_ms` + `judge_ms` | 104 + 99 ≈ **203ms（~21%）** | T2-B 能碰的只有其中一部分 |
  | `tc_cache_builds` | **45768** | 每趟都重建 `TcCache`（预分配）⇒ **固定成本**，T2-B 省不掉 ✗ |
  | `hits=0 misses=13` · `doc_passes=0` | — | 合成趟已归零（§11.11 ✓） |

* **上界的另一个更硬的锚**（同构建 LSP 探针）：`typing` **78.5ms** vs `typing_equal_length`
  **41ms** —— 两者都是"一刀"、都是新文本，后者**连陈述都改了**（依赖者更多、更贵 ✗）却**只要
  41ms** ⇒ **一刀的"固定底" ≈40ms** ✓ ⇒ T2-B 的理想收益 ≈ **78 − 41 = ~37ms**（还要再减去
  它自己的快照查/校验成本 ✗）⇒ **量级 ~30ms/刀**。
* **判定**：上界 ~30ms **略高于** §14 设的 20ms 门槛 ⇒ **不划掉**，但它的**前置件很大**
  （文件感知的 span 模型 §11.10 + 命令级环境快照 = 新机制 ✗）⇒ **缓做** ✓：**先做便宜的**，
  等下一棒有余量再评估。**要确认这个上界**的最简实验（写死）：造"**只追加尾部注释**"的一刀
  （不改任何声明 ⇒ 脏集空）量它的墙钟 = **真固定底** ✓；若它已经 ≈70ms ⇒ T2-B **直接划掉** ✗。

---

### 18. 第 21 轮（2026-10-09 · 平行线）—— 全量 gate ✓ + **T1-B 批 1 第 2 件落地**（装载口）

* **① 全量 `scripts/soko gate` 收回结果 = PASS** ✓（§12 的"下一棒先做①"完成）：
  第 19 轮起跑的那次，本轮**原样落盘**（`/tmp/gate-full-r3.log`）后确认收尾行
  `sokonanoda: gate PASS` ✓（⚠ 按 §12 的教训：别用 `EXIT=` 判 —— 那是管道最后一个命令的状态）。
* **② T1-B 批 1 第 2 件**（`8390799e`）：内核装载口
  **`EnvBuilder::from_export_file(arena, file)`** —— 从一份已建好的 `ExportFile`
  **继续编**（之后照常 `add_declar`/`finish`）。原七个方法都不够
  （`new` 从空、`snapshot`/`with_env` 只借只读副本、`finish` 反方向 ✗）。
  **纯能力新增、零行为变化**（当前**无调用者** ⇒ 输出逐字节不变 ✓）。
  * 判据（`builder.rs` 的 `from_export_file_carries_the_intern_tables_not_a_rebuilt_dag` ·
    **两向**）：① 装载后与产物**指针同一**；② **intern 表也搬过来了**（再 intern 同名
    ⇒ **同一个 `NamePtr`**）；③ 装载后 `add_declar` 照常 + **内核真判过**；
    ④ 反向：空环境判不过；⑤ 反向（指针侧）：重建 `Dag` ⇒ 同名得**第二个**节点。
  * 记账：`docs/architecture.md` §6 追加一行（内核改动纪律）；
    `docs-budget.json` 的 `architecture.md` 753→754 与 `L1` 5674→5675（只加那一行台账，
    按 docs-lint 自己给的例外手改 ✓）。
* **③ T1-B 批 1 的下半件 = writer（ndjson 序列化）** —— 本轮**只调研、未写码**：
  格式已核清楚（`parser.rs` 的 `ExportJsonVal`：`meta` / `str`·`num`（名）/
  `succ`·`max`·`imax`·`param`（层）/ `sort`·`bvar`·`const`·`app`·`forallE`·`lam`·`proj`·
  `letE`·`mdata`·`natVal`·`strVal`（项）/ `axiom`·`thm`·`def`·`opaque`·`inductive`·`ctor`·
  `recursor`·`quot`（声明）；每个对象带可选 `in`/`il`/`ie` 回引）。
  **形状**：post-order 给子项分配下标（子先父后）、每个唯一节点发一次（hash-cons 的
  `Dag` 天然去重 ✓）、声明引用相应下标。**判据**：单进程往返 ——
  `ExportFile → 文本 → Parser → ExportFile'` 之后，**对着 `ExportFile'` 编同一条下游
  声明**与对着原 `ExportFile` 编 ⇒ **逐字节同**（`--json`）。
  仓库里**没有** ndjson 样例（fixture 在外部 `LEAN_KERNEL_ARENA`）⇒ 格式只能从
  `parser.rs` 反推 ✓（已入账）。
* **四方向账（本轮后）**：① T1-A ✓ / T1-B **批 1 已完成一半**（装载口 ✓ · writer 未做）·
  ② T1-A ✓（跨条目复用） / T2-B 未做 · ③ ✓（T3-B1 ①②③ + §4.2 第 5 条族 + T3-B2 出口）· ④ T4-A ✓。

### 17. §16 的确认实验做完：**一刀的固定底 = 36.7ms**（T2-B 天花板 ~40ms；**新目标其实是这个底** ✓）

* **新增探针臂**（`perf_keystroke_wallclock.rs` · `trailing_comment`）：每刀**只在文件末尾追加一行注释**
  ⇒ **不移动任何命令的起点** ⇒ 脏集为空、所有命令可信任 ⇒ 量到的就是"**一刀的固定底**" ✓。
* **读数**（同一构建 `lsp-cargo-mtime=1791482786` · 同一次运行，**可比** ✓）：

  | 臂 | median | `compile` | `by` |
  |---|---|---|---|
  | **`trailing_comment`（脏集空 = 固定底）** | **36.7ms** | **36ms** | 9 |
  | `typing`（真实连续键入） | **76.5ms** | 74ms | 9 |
  | `typing_equal_length`（等长改名） | 38.3ms | 40ms | 9 |
  | `proof` / `statement` | 77.0 / 76.9ms | 110 / 78ms | 9 |

* **判定（T2-B）**：**地板 36.7ms ⇒ 天花板 = 76.5 − 36.7 ≈ 40ms**（还要减 T2-B 自己的快照查/校验 ✗）
  ⇒ **不划掉**（40ms > §14 的 20ms 门槛 ✓），**但仍缓做**（前置件 = 文件感知的 span 模型 + 命令级
  环境快照 = 新机制 ✗）。**§16 那条"若底已 ≈70ms 就直接划掉"的条件**：**不成立** ✓。
* **⚠ 更值钱的发现（改判"下一棒先打哪")**：**一刀的固定底 36.7ms 本身就是最大的一块**
  —— 它里面**一条声明的 elaborate 都没有**（脏集空 ✓），却仍要 36ms `compile`
  ⇒ 那是**每趟都要付的固定成本**（`TcCache` 重建 · pass 骨架 · 报告装配/pp）+ 服务端排空 ✗。
  ⇒ **下一棒该打的是"这个底"**（方向②"服务器热路"的正题 ✓），而不是 T2-B ✗：
  它的**收益上限**（≈40ms/刀）**比 T2-B 还大**，而且**不需要新机制**（都是现有代码里的固定开销）✓。
  **先量**：拿 `SOKO_STAGE_STATS` 对**空脏集那一刀**做相位分解（`tc_cache_builds` 每趟一次 =
  预分配 4MiB + 20 张表 ⇒ 是否有"零改动就不重建"的快路？），再决定。

### 18. 固定底（36.7ms）的第一刀：**嫌疑指到 `TcCache` 的构造次数**（假设 + 待归属，未定论）

* **原始读数（已有，不用新跑）**：`STAGE_STATS` 的 `tc_cache_builds=**45768**` 出现在
  `judge_synthesized_typing`（冷开 + 10 刀）那一整程里 ⇒ 摊到 ~34 趟 pass ≈ **1300 次/趟** ✗✗。
  每次 `with_tc` 都新建一份**预分配 ≈4 MiB + 20 张表**的 `TcCache`（`check/mod.rs:879` 的注释 ✓）。
* **⚠ 数字自洽性没对上（先说清，别当结论）**：`docs/PERF.md` 旧账写"≈**61.8µs/次**"
  ⇒ 45768 × 61.8µs ≈ **2.8s**，可这一整程的 `pass_total_ms` 只有 **944ms** ✗ ——
  要么 61.8µs **是旧构建的读数**（不可跨构建引用 ✗，`AGENTS.md` 探针身份纪律），要么这些构造
  **落在被计时的 pass 之外**。**⇒ 结论：先重量，别引用旧值** ✓。
* **构造点（已定位）**：`kernel/src/util.rs`（`with_tc` 本体）· front 调用点 =
  `elab.rs:7033/7062`（`PendingOp::Check` 的 `ByIndex(env_at)` 那条 · 注释说"不在里面分配"✗）、
  `check/level_exit.rs:171` · `check/mod.rs:1899`（内核相位）
  ⇒ **"一趟 pass 里为什么会有 1300 次"** 才是要回答的问题（若是**每条命令 × 每个 op**各来一次
  ⇒ 按趟复用 / 按命令复用就是直接收益 ✓；若真有 1300 条 op ⇒ 那是别的账）。
* **下一轮的量法（写死）**：给 `with_tc` 加**调用点标签**的分档计数（或按 `env_at` 去重的
  "同一 `env_at` 重建了几次" ⇒ 一眼看出可否缓存 ✓），并在**空脏集那一刀**（`trailing_comment` 臂）
  上量 ⇒ 得到"底里有多少是 `TcCache` 构造" ✓。**门槛**：能省 ≥10ms/刀 ⇒ 值得动 ✓。
* **为什么这轮不开工**：它落在**内核 `util.rs` + 内核相位**的分配生命周期上（`AGENTS.md`
  §8 gotchas：arena 生命周期 · `quiet_catch` 不可嵌套 ✗）⇒ 要有"调用点标签读数"才敢动 ✓。

### 19. 第 27 轮：**LSP 侧读不到 `STAGE_STATS`**（我的探针缺口 ✗）＋读数复现 ✓

* **复现（同构建 `lsp-cargo-mtime=1791482786`）**：`typing` median **75.7ms** · **`trailing_comment`
  median 36.4ms**（best 36.1 / worst 36.6 —— **极稳** ✓，说明它是真的固定底而不是噪声 ✓）
  ⇒ §17 的两个锚在**另一轮运行里重现** ✓。
* **本轮想做的事没做成（如实记）**：想在**探针里**读服务端那一侧的 `SOKO_STAGE_STATS`
  （好把 `tc_cache_builds` 按"一刀"归属 ✗）—— 实测 `grep -c STAGE_STATS` = **0** ✗：
  那是**子进程退出时**打的（`atexit`），而 `Client` 抓的是它的 **stderr 读线程**，探针**没有把它
  回显到测试 stdout** ✗ ⇒ 读不到。
* **下一轮（写死 · 二选一）**：① **补探针**：关服前把客户端捕获的 stderr 尾部（含
  `STAGE_STATS`/`JUDGE_INPLACE …` 那两行）**回显出来**（`crates/lsp/tests/common/mod.rs`，本线 ✓，
  小改动 ✓）；② 或者走 front 路：用一个"只做一刀"的 front 用例 + `SOKO_STAGE_STATS`（§18 那条路
  已经在 front 上量到 45768 ✓）—— 但 front 路**量不到 LSP 的排空开销** ✗ ⇒ **① 优先** ✓。
* **没量到就不当量到** ✓：`TcCache` 的"每刀几次、每次多少 µs"**本轮一无所获** ✗ ✓。

### 20. 第 28 轮：**归属拿到了 —— `TcCache` 构造就是固定底与真实按键之间的全部差额** ✓✓

探针现在每臂都打 `tc=`（= 这一刀的 `TcCache` 构造次数；§19 想抓的 `STAGE_STATS` 不必抓了 ✗ ——
`tc=` **每刀都有** ✓）。同构建 `lsp-cargo-mtime=1791482786` · 同一次运行：

| 臂 | median | `compile` | **`tc=`** | telescope |
|---|---|---|---|---|
| `typing`（真实连续键入 · 新文本） | 74.7ms | 74ms | **4225** | 621 |
| `proof` / `statement` | 75.5 / 75.8ms | 76ms | **4225** | 621 |
| `typing_equal_length`（等长改名） | 39.3ms | 40ms | **231** | 497 |
| **`trailing_comment`（脏集空 ＝ 固定底）** | **36.9ms** | 36ms | **113** | 497 |

* **读法**：`tc` 从 **113 → 4225**（+4112 次）↔ 墙钟从 **36.9 → 74.7ms**（+37.8ms）
  ⇒ **≈9.2µs/次**（**本构建实测** ✓ —— 顺手改正 `docs/PERF.md` 那句"61.8µs/次"：那是**旧构建**
  的数，**不可跨构建引用** ✗，`AGENTS.md` 探针身份纪律）。
  ⇒ **`TcCache` 构造吃掉了这一刀的全部增量**：4225 × 9.2µs ≈ **39ms** ≈ 那 37.8ms ✓✓。
* **为什么"改了文本"就从 113 变 4225**：脏命令的 elaborate + **judge 的查询**各自 `with_tc`；
  文本一变，judge 的**前缀键**全 miss ⇒ 每条声明的查询都重建一份（4MiB 预分配 ✗）。
* **⇒ 下一棒的目标（收益最大的一条）**：**按趟/按命令复用 `TcCache`**（而不是每个 op 新建）
  —— 上界 **≈37ms/刀**（74.7 → ≈37ms ⇒ **对 Lean 的 218ms 是 5.9×** ✓✓）。
  前置：落在内核 `util.rs` 的 `with_tc` + front 的调用点（`elab.rs:7033/7062` ·
  `level_exit.rs:171` · `check/mod.rs:1899`）⇒ 要读 `docs/architecture.md` §8 的 arena 生命周期
  gotchas ✓（**先读后动** ✗，本轮不动手）。
* **判据（落地时）**：① `typing` 臂 `tc` **4225 → ≈113** 量级；② 墙钟 ≈ 固定底；
  ③ 全课程 `--json` 逐字节不变 ✓；④ `typing_equal_length`/`trailing_comment` **不许变慢** ✓。

### 21. 第 29 轮：**先读后动** —— `with_tc` 的每调用状态与"为什么不能天真复用"（为下一棒铺路）

* **读 `kernel/src/util.rs:696-716`（本轮的成果）**：`with_tc(limit, f)` = `with_ctx(...)` 里**每次**
  新建一整套：

  | 每调用新建 | 说明 |
  |---|---|
  | `Arena::new()` + `with_scope` | 每个 tc 一份**新 arena 作用域** ✗ |
  | `bumpalo::Bump::new()` | 每份新 bump ✗ |
  | **`TcCache::new(&bump)`** | **就是那 20 张表 + 4MiB 预分配**（§20 量到的 9.2µs/次 ✓）✗ |
  | `TypeChecker::new(ctx, &env, bump, None, cache)` | 每调用一个 checker ✗ |

  而 `TcCtx` 里**长期持有的**（属于 `Source`，不每次新建 ✓）：`export_file` · `dag` ·
  `expr_cache` · `sig_cache`/`sig_computing` ✓ ⇒ **可复用与不可复用的边界很清楚** ✓。
* **为什么不能天真地"把 TcCache 提出来复用"** ✗：`TcCache<'t,'t>` 借 **bump**，bump 借
  **arena scope**，scope 的生命周期**活不出 `with_ctx` 的闭包** ✗ ⇒ 想复用就得解决
  **自引用 + 生命周期**（要么把 `TcCache` 改成**不含 arena 借用**的形状、要么走"**泄漏一份
  `'static` bump**"那条路 ✓ —— front 已有先例：`Box::leak` + `MAX_LEAKED_LIB_ARENAS = 8` 的
  库层 arena 池 ✓）。
* **`docs/architecture.md` §8 第 1 条给了硬边**（读到了 ✓）：*"`EnvBuilder`/`ExportFile`/`ExprPtr`
  都挂在同一个 `stumpalo::Arena` 上，arena 必须活得比任何检查会话久；……Session 每次 update 都开
  新 arena —— **跨 update 只复用渲染后的快照，不复用内核对象**"*
  ⇒ **跨 update 复用 `TcCache` 是明确禁止的** ✗；**可以**做的只有**同一次 update / 同一趟 pass 内**
  的复用（那时 arena 本来就在 `compile_fol` 的作用域里活着 ✓）⇒ 候选设计 **1（按趟复用）** 是
  **唯一合规**的那条 ✓，候选 2（泄漏池）要额外论证它不违反"跨 update 不复用内核对象" ✗。
* **两条候选设计（下一棒二选一，都还未开工）**：
  1. **按趟复用**（推荐先试）：在 pass 作用域里建**一次** `Bump+T cCache`，把引用**沿调用链**
     传到 `elab_*`/`judge_*`（签名要扩 ✗，但改动是**显式**的、无 `unsafe` ✓）；
  2. **池化 + 泄漏**：线程局部池（`Box::leak`，仿 front 的 arena 池 ✓），每次 `with_tc` 从池里
     取一份 `reset` 过的；**代价是有界泄漏**（要像 front 那样给上限 ✓）。
* **红线与风险（写死）**：① 它落在**内核相位**（`AGENTS.md` §8：arena 生命周期 · `quiet_catch`
  **不可嵌套** ✗）⇒ 改完必须跑**全课程 `--json` 逐字节** ✓；② 判据四条沿用 §20
  （`tc` 4225→≈113 量级 / 墙钟≈固定底 / `--json` 逐字节 / 另两臂不许变慢）✓；
  ③ **不许**为了复用改变判定语义（`TcCache` 只影响**性能**，不许影响结果 ✓）。

### 22. 第 30 轮（平行线）：**T1-B 批 1 的 writer 落地** —— 序列化闭环打通 ✓

* **落点**（`07800866`）：
  * `crates/kernel/src/writer.rs`（新）：`write_export_file(&ExportFile)` —— 名/层/项/声明
    四段，按**依赖序**分配下标（名先父后子 · 层与项先子后父）；顺序 = 名字行 → 层级行 →
    项行 → 声明行（读侧是**按行增量**填表的 ✓）。下标 **0 恒为 `Anon`/`Zero`**（读侧预置 ⇒ 不写）。
  * `parser.rs`：读侧 JSON 类型加 `serde::Serialize`（**纯派生**，`Deserialize`/别名一字不动 ✓）
    + `natVal` 的十进制字符串序列化（与反序列化**对称**）⇒ **写/读用同一套派生 ⇒ 形状
    构造性一致**（不是照文档手抄 ✗）。
  * `ExportFile::to_ndjson()`（`util.rs`）：**纯函数、不改环境** ✓。
* **判据**（`writer.rs` 的 `#[cfg(test)]`）：
  ① **往返幂等** —— `ExportFile → 文本 → Parser → ExportFile'` 之后再写一遍与第一遍
  **逐字节相同** ✓（读侧在**另一份 arena** 里重建 ⇒ 跨 arena 比指针没意义 ⇒ 幂等才是
  "结构一字不差"的判据 ✓）；
  ② 读回来的环境**真的过内核**（`check_all_declars()` ✓，含 `Pi`/`Lambda`/`Var` 与
  `theorem` 的值位）；
  ③ **反向**：换一份**不同**环境 ⇒ 文本**必不同** ✓（判据①有牙）；
  ④ 内核 **66/66** · front `--lib` **875/875** · CLI 冒烟 ✓。
* **覆盖与下一版**：**非归纳**声明（`axiom`/`thm`/`def`/`opaque`/`quot`）已打通；
  **归纳块**（`inductive`/`ctor`/`recursor`）遇到返回 `Err` ⇒ 调用方**静默回退**"本地重编"
  （**不产生半个产物** ✓）。⇒ **批 1 的下一件 = 归纳块写入**；之后才是
  **批 2**（接到 `<根>/.sokonanoda/artifacts/` + 完整性三道）。
* **四方向账（本轮后）**：① T1-A ✓ / **T1-B 批 1 = 装载口 ✓ + writer ✓（非归纳）**
  ⇒ 归纳块是批 1 唯一剩余 · ② T1-A ✓（跨条目复用） / T2-B 缓做（§16 量级判定 ~40ms 上界）·
  ③ ✓（T3-B1 ①②③ + §4.2 第 5 条族 + T3-B2 出口）· ④ T4-A ✓。

### 23. 第 31 轮（平行线）：**T1-B 批 1 完成** —— 归纳块也进 writer，序列化对全部声明形状闭环 ✓

* **落点**（`4285b94c`）：`writer.rs` 的 `collect` 改**按块**走 —— 归纳块在 `declars` 里是
  **连续**一段（ind… · ctor… · rec…），区间取 `mutual_block_sizes`（`begin/end_inductive_block`
  填的，前端每个块都调 ✓）；**一个块 ⇒ 一行 `inductive`**（`types`/`ctors`/`recs`，与读侧
  `Inductive { … }` 同形）。⚠ `is_reflexive` **读侧忽略**（`IndInfo` 里是 `..`）⇒ 恒写
  `false` —— 必须是**确定值**，否则往返不幂等 ✗。
  `parser.rs`：`IndInfo`/`Constructor`/`Recursor` 字段 `pub(crate)`；
  **`parse_export_mapped` 提为 `pub`**（`to_ndjson` 的读侧对偶 ⇒ 跨 crate 判据能**同一处**闭环 ✓）。
* **判据**：① `memory_api.rs` 新增 `my_nat_block_round_trips_through_ndjson` —— 用仓库里
  唯一一个"与教学前端同构"的**显式归纳块**（`MyNat` + 2 构造子 + 显式消去子 + iota 规则）
  ⇒ 写→读→**再写逐字节幂等** ✓ + 读回来的环境**过完整内核检查** ✓；
  ② **反向验证（已做）**：iota 规则写成空 ⇒ 判据**判红** ✓（`inductive.rs:1714` 咬住，随后还原）
  —— ⚠ 注意**幂等那一半咬不住内容丢失**（丢了规则的文本自身仍自洽）⇒ 有牙的是**内核检查那一半** ✓；
  ③ 内核 **66/66** · front `--lib` **875/875** ✓。
* **批 1 账**：`EnvBuilder::from_export_file`（装载口 ✓）· `ExportFile::to_ndjson`（writer ✓，
  **覆盖全部声明形状**）· 三条判据（非归纳幂等 + 反向 + 归纳块幂等&内核检查 ✓）。
  剩余 `Err` 出口只有"记账缺失/混合声明"两种异常 ⇒ 调用方静默回退"本地重编" ✓。
* **下一件 = T1-B 批 2**：接到 `<根>/.sokonanoda/artifacts/` + **完整性/安全/离线三道**
  （设计 `docs/design/module-artifacts.md` §8.1–§8.4：按 tag 锁定、失败当不存在、离线静默回退），
  失效规则对齐 lake 的 `BuildTrace{hash, mtime}`（**mtime 永不参与正确性** ✓）。
  ⚠ **批 2 会碰 `crates/front/src/compile/**` 与 `project/session.rs`** —— 与本线（TcCache）**同一片**，
  开工前先看 `git status --short` 与 §11.5/§11.6 的归属约定 ✓。
* **四方向账（本轮后）**：① T1-A ✓ / **T1-B 批 1 ✓✓（批 2/3 未做）** · ② T1-A ✓（跨条目复用） / T2-B 缓做 ·
  ③ ✓ · ④ T4-A ✓。

### 24. 第 32 轮（平行线）：批 2 前半落地（产物落盘 + 完整性三道）＋ **批 2 的形状定死：A + B + C，不是 A 单独** ✓

* **落地**（`25e4542a`）：新模块 `crates/front/src/project/artifacts.rs`（**不接线**）——
  `<模块根>/.sokonanoda/artifacts/<key>.bin` + `<key>.meta.json`，`key` 复用**已存在**的
  `ProjectPlan::module_keys`（每模块 Merkle 键 ✓）。**完整性三道**（§8.2）全在 `read` 里：
  ① 凭据逐项（format/版本/build stamp/target/prelude）· ② 载荷字节数 + 摘要 ·
  ③ 键即文件名。**两个"顺序"也是判据的一部分**：**先载荷后凭据**（崩在中间 ⇒ 只有
  没有凭据的载荷 ⇒ miss ✓）；写产物顺手让 `.sokonanoda/` **自忽略**（否则冒出
  `?? .sokonanoda/` ✗）。另加 §8.3 的边界：`key` 只许 `[A-Za-z0-9_-]{1,128}`
  （将来可能来自下载清单 ⇒ 含 `/`/`..` 就是**目录穿越** ✗）。
* **判据**：6 条文件内单测（往返 · 改一个字节/截断 ⇒ miss · **五个身份字段逐项**
  不符 ⇒ miss（每条补丁先断言"补丁生效"，防判据空转 ✓）· 换键/无凭据 ⇒ miss ·
  非法键拒写拒读且不留痕 ✓ · 自忽略内容 == `*\n` ✓）。front `--lib` **880/880** ·
  fmt 干净 · clippy **本文件 0 条**（顺手修掉自己引进的 `doc_lazy_continuation` ✗）。
* ⭐ **批 2 的形状定死了（本轮最重要的结论）**：产物**不能只存 A（内核环境）** ——
  消费侧要 `PassTables`（`known`/`inductives`/`defs`），而它们**不是内核环境的纯函数**：
  * `KnownName::Decl.signature` 是**源级类型文本**（`render_expr(ty)`）—— 文档自己写明
    "**不能用 pp/judge 文本**"（① 递归、② pp 省略嵌套常量的隐式实参 ⇒ 反解不出来）⇒
    从内核环境反推**没有**这条文本 ✗；
  * `implicit_prefix` / `explicit_arity` 是**源级 AST 走查**（`leading_implicit_prefix`
    / `explicit_arity`，文档原话"**零内核调用**"）⇒ 从内核 Pi binder 反推是**另一套实现**
    ⇒ 一旦与源级走查在边界情形上分叉，隐式实参插入就变 ⇒ **判定/`--json` 分叉** ✗
    （这正是"第五套实现"的守卫要抓的东西）。
  ⇒ **批 2 = A + B（序列化）+ C**。B 的可行形状（本轮读码）：`DefInfo` **全是字符串**
  （好办 ✓）；`InductiveInfo` 带 `MatchCtor`/`index_types: Vec<Expr>`（**源级 AST**）
  ⇒ 要么给前端 AST 加一条序列化，要么存文本形式；`known` 的表项本身只有
  `String`/`usize`/`Option<String>` ✓。⚠ **指针重映射**：`InductiveTable<'a>` 的表项含
  arena 指针 ⇒ 装载时**按名字**在已装载的内核环境里重解析（不能按地址 ✗）。
* **下一件**：① B 的序列化（先给 `DefInfo`/`InductiveInfo`/`KnownName` 定型）；② 写入口
  （session 在每个**模块边界**用 `builder.snapshot().to_ndjson()` 写一份 —— 那时的环境
  正好是"依赖闭包 + 本模块" ✓）；③ 消费入口（装载 A + 读回 B ⇒ 接着编）。
  ⚠ **顺序上先别只写入口**：消费侧没接之前写产物是**纯开销**（每次 build 多写 N 份、
  可能几十 MB），会直接踩性能纪律 ⇒ **先定型 B、把消费侧接上，再开写** ✓。
* **四方向账（本轮后）**：① T1-A ✓ / T1-B **批 1 ✓ · 批 2 前半 ✓（B 定型是下一个关口）** ·
  ② T1-A ✓（跨条目复用） / T2-B 缓做 · ③ ✓ · ④ T4-A ✓。

### 25. 第 33 轮（平行线）：B 块**定型 + 编解码**（`known` + `defs`）✓

* **先核实了"B 能存"**（不是想当然）：读 `walk.rs` 的登记点 —— `KnownName::Decl`
  的四个字段**只吃** `name`/`universe`/源级 `ty`（`proof::decl_signature` ·
  `leading_implicit_prefix` · `explicit_arity`），`DefInfo` 同理
  （`params`/`universes`/`implicit_prefix`/`body`/`telescope_arity`）
  ⇒ **源级 AST 能序列化 ⇒ B 就能存** ✓（这也再次印证 §24：B 是**源**的函数、
  **不是**内核环境的函数）。
* **落点**（`1b80871f`）：`ast.rs` 给前端 AST **15 个类型**加 `Serialize/Deserialize`
  （纯派生新增 ✓）· `elab.rs` 的 `KnownName`/`DefInfo` 加 serde（`DefInfo` 另加
  `PartialEq`）· 新模块 `project/tables.rs`：`encode(&KnownTable, &DefTable)` /
  `decode(&str)`。
* **两条格式纪律**（进模块文档 + 判据）：① **确定序** —— `HashMap` 直接 serde
  **顺序不定** ⇒ 同一份表可能编出**不同文本** ✗（缓存/对拍要逐字节）⇒ **按键排序** ✓；
  ② **带格式号**，不认 ⇒ `Err` ⇒ 调用方**当不存在**、静默回退（与 §8.2 同纪律 ✓）。
* **判据 3 条**：往返逐项相同（**三种** `KnownName` 变体 + 带源级 AST 体的 `DefInfo`
  全覆盖）· **插入顺序无关**（文本逐字节相同）· **反向验证**（格式号 9999 与
  **截断文本**都要被拒 ✓）。front `--lib` **884/884** · fmt 干净 · clippy 报错 0 ✓。
* ⚠ 三处 `#[allow(dead_code)]`：**消费入口接上之前没人调** —— 遵守 §24 的顺序纪律
  （先别只写入口），先定型 + 先带判据 ✓。
* **B 的剩余**：`inductives`（`InductiveInfo` 带**内核裸指针** `MatchField.ty: ExprPtr`
  ⇒ 只有两条路：(a) 序列化时把它落成"构造子 + 望远镜第 k 个 binder"的**名字式引用**、
  装载时在已装载环境里重解析 ✓；(b) 干脆从内核环境重建这一张（`ctors` 的
  `name`/`canonical`/`fields[].name`/`style` 都能从**装载来的 ctor 声明**读出来，
  `src_ty`/`index_types` 是源级 AST ⇒ 得从源走 **parse-only** 那一半）。
  ⇒ 这是 B 的最后一个关口，与**消费入口**同批做才不浪费 ✓。
* **四方向账（本轮后）**：① T1-A ✓ / T1-B **批 1 ✓ · 批 2：产物存储 ✓ · B(known+defs) ✓ ·
  B(inductives) 与写/消费入口未做** · ② T1-A ✓（跨条目复用） / T2-B 缓做 · ③ ✓ · ④ T4-A ✓。

### 26. 第 34 轮（平行线）：B 块**完成**（`inductives` 的 wire 形 + 装载时重解析）✓

* **关口的实质**：`InductiveInfo` 里**只有一样东西不可序列化** —— `MatchField.ty`
  （内核 arena 裸指针 ✗）。本轮把它做成**唯一"存引用、装载时重解析"的字段**，
  其余（含 `src_ty`/`index_types` 的**源级 AST**）照存 ✓。
* **为什么这不是"丢信息"**：那份 `ty` 是"构造子望远镜里第 k 个 binder 的类型"，
  而**构造子声明就在已装载的内核环境里** ⇒ 装载时按**规范名**取回声明、走**同一个**
  `kernel_field_binders`（本轮提为 `pub(crate)`）重取一遍 ⇒ **同源、不漂移** ✓。
* **落点**（`f551e172`）：`MatchFieldWire`/`MatchCtorWire`/`InductiveInfoWire` ·
  `encode(known, defs, inductives)` · `decode ⇒ Decoded` ·
  `rehydrate_inductives(wire, lookup)`（`lookup: Fn(&str) -> Option<Declar>` ——
  前端**不能**把字符串 intern 进已建好的 `ExportFile`，所以这条口子必须是**回调** ✓）。
  失败面一律 `Err`（回退本地重编）：构造子不在环境里 · **层数少于存的字段数** ·
  逐层 `style` 与产物不符。
* **判据 4 条**（`project/tables.rs`）：① `known`+`defs` 往返 · ② **插入顺序无关**
  （文本逐字节相同）· ③ 反向：格式号/截断 ⇒ `Err` · ④ ⭐ **`inductives` 往返 + 重解析**
  —— 造**真的**内核环境（`C : (α : Sort 0) → (x : α) → {y : α} → C`）⇒
  重解析出来的 `MatchField.ty` 与原件**指针逐位相同** ✓ + `style` 对 ✓；
  **两条反向验证**：`num_params` 写大 1 ⇒ 判红 ✓ · 规范名不在环境里 ⇒ 判红 ✓。
  front `--lib` **885/885** · fmt 干净 · clippy 0 ✓。
* ⚠ **夹具教训（写进注释了）**：`mk_pi` 是**从里往外**造的（最外层最后建）——
  建反了 binder 顺序就倒过来 ⇒ `MatchField.ty` 错位（本条判据第一版就是这么红的 ✓）。
* **B 的账（完成）**：`known` ✓ · `defs` ✓ · `inductives`（wire + 重解析）✓ ·
  **两条格式纪律**（确定序 · 带格式号）✓。
  ⇒ **批 2 剩下的就只剩"接线"**：**写入口**（session 在每个模块边界
  `builder.snapshot().to_ndjson()` + `tables::encode` 写一份）与**消费入口**
  （装载 A + 读回 B ⇒ 接着编）。⚠ 仍按 §24 的顺序：**两个一起接**，别只接写（纯开销 ✗）。
* **四方向账（本轮后）**：① T1-A ✓ / T1-B **批 1 ✓ · 批 2：产物存储 ✓ + B(known/defs/inductives) ✓
  ⇒ 只剩接线** · ② T1-A ✓（跨条目复用） / T2-B 缓做 · ③ ✓ · ④ T4-A ✓。

### 27. 第 35 轮（平行线）：批 2 的**载荷合体形**落地 —— A+B 跨 arena 往返打通 ✓

* **落点**（`23215561`）：`project/artifacts.rs` 的 `encode_payload` /
  `decode_payload` —— 一行头 `soko.module-artifact/1 <A 的字节数>\n` 后接
  A（内核环境文本）、再接 B（前端表文本）。
  ⚠ **不做 JSON 包一层**：A 是几十万字节文本，包 JSON 要整体转义（体积 ×1.2 + 多一次
  分配 ✗）；一行头 + 两段拼接**确定、可流式校验** ✓。
  `decode_payload` **收 arena**（T1-B 的设计约束：装载必须落在本趟 pass 自己的
  arena 里 ✓），内部 = 拆头（任何不自洽 ⇒ `Err`）⇒ `parse_export_mapped` 出 A ⇒
  `tables::decode` 出 B ⇒ `rehydrate_inductives` 按规范名回填 `MatchField.ty`。
  另补 `declaration_index`/`render_name`（内核**没有**公开的"`NamePtr` ⇒ 字符串"
  自由函数 ⇒ 用 `TcCtx::read_name/read_string` 递归拼，与 `pretty_printer::name_to_string`
  **同一套语义** ✓）。
* ⚠ **信任模型（§8.3）写进代码文档**：这条口子按"**自己人写出来的产物**"解析 ⇒
  传的 `Config` **放行一切公理**（产物里的 `axiom` 是本地声明回放）。真要信任
  **下载来**的产物是 §8.3 的开关 + 清单签名那件事 ✓。
* ⭐ **判据 `payload_round_trips_across_arenas`**：真的内核环境（两条公理）+ 一张前端表
  ⇒ 编码 ⇒ **过磁盘**（顺带走一遍完整性三道 ✓）⇒ **换一份 arena** 解码 ⇒
  ① 声明一条不少 ② **`check_all_declars()` 过** ✓ ③ 前端表**逐项相同** ✓；
  **两条反向验证**：**截断** ⇒ `Err` ✓ · 头部**声称长度超实际** ⇒ `Err` ✓。
  front `--lib` **886/886** · fmt 干净 · clippy 0 报错 ✓。
* ⚠ **过程教训（只影响我自己、已修正）**：`cat >>` 追加函数之后再"按最后一个 `}` 插
  测试"会把测试插进**函数体里** ✗（本轮踩了两次，第二次靠**花括号配对**才修对）；
  clippy 另外要求 **test 模块必须是文件最后一个 item**（`items_after_test_module`）⇒
  测试模块一律放文件末尾 ✓。**这两条写进下一棒的注意事项：往已有文件追加时，
  "插到测试模块里"必须用配对定位、不能数最后一个大括号** ✓。
* **批 2 的账**：产物存储 ✓ · B（known/defs/inductives）✓ · 载荷合体形 ✓ ·
  **⇒ 只剩"接进 session"**：写入口（每模块边界写一份）与消费入口（装载 A+B ⇒
  跳过库层 walk ⇒ 只走该入口自己的命令）。
  ⚠ 消费入口要动 `run_library_from` 的**控制流**（T1-A 的 `ResumeState` / judge 前缀
  偏移都挂在上面）⇒ **必须同一次改完并逐字节对拍**，别半接 ✓。
* **四方向账（本轮后）**：① T1-A ✓ / T1-B **批 1 ✓ · 批 2 的基础设施全齐，只剩接进 session** ·
  ② T1-A ✓（跨条目复用） / T2-B 缓做 · ③ ✓ · ④ T4-A ✓。

### 28. 第 36 轮（平行线）：载荷补上 **C 块** —— 批 2 的三块全部走得通 wire ✓

* **为什么 C 不能省**（本轮读 `session.rs` 才定死）：消费入口要"**跳过库层 walk**、
  只走入口自己的命令"，而 `run_entries` 把**库层输出与入口输出合并**成入口那份
  `CompileOutput`（`on_entry` 收的就是**合并结果**）⇒ 少了库层这半 ⇒ `--json`
  **不是逐字节相同** ✗。⇒ C 必须跟着产物走。
* **落点**（`8ecf1610`）：`LibPassFacts`（字段与 `session::LibCheckpoint` **一一对应**，
  除 `builder`/`tables` = A/B）：`out` · `reports` · `ranges`（`Range` 无 serde ⇒
  落 `(start,end)` 二元组）· `n_commands` · `prefix_commands` · `lib_prefix`。
  载荷头改成**两段长度** `<magic> <a_len> <b_len>\n` 后接 A、B、C；`split_payload`
  任何一段**长度不自洽**都 ⇒ `Err` ✓。
* **判据**：`payload_round_trips_across_arenas` 扩到三段 —— 真环境 + 前端表 + C 块
  ⇒ 过磁盘 ⇒ **换 arena** 解码 ⇒ 环境过 `check_all_declars` ✓ · 表逐项相同 ✓ ·
  **C 逐项相同** ✓；反向：截断 ⇒ `Err` · 头部长度超实际 ⇒ `Err` ✓。
  front `--lib` **886/886** · fmt 干净 · clippy 0 ✓。
* ⭐ **批 2 的状态**：**A / B / C 三块全部走得通 wire**（各自带判据）⇒
  剩下的**纯粹是 session 里的控制流接线**，没有未知形状了 ✓：
  1. **写入口**：`run_library_pass` 跑完 ⇒ `builder.snapshot()` 出 `ExportFile`
     ⇒ `encode_payload` ⇒ `artifacts::write(root, lib_key(lib_units, options), …)`。
     ⚠ `lib_key`（`session.rs:234`）**已经是**"库层集合+顺序+名字/路径/源文本+import 边
     +prelude 模式"的摘要 ⇒ 直接复用当产物键即可（**不需要**再引 `module_keys`）✓。
  2. **消费入口**：`artifacts::read` 命中 ⇒ `decode_payload` ⇒
     `EnvBuilder::from_export_file` + `PassTables{…}` + `LibPassFacts` **拼出一份等价于
     `LibCheckpoint` 的东西** ⇒ 直接进 `run_entries`（**不跑库层 walk** ✓）。
     ⇒ 这正是"① 产物化：import 走 mmap 不 elaborate"的落点 ✓。
  3. **必须先定的两件**（否则会半接）：① 产物的**根目录**从哪来（session 现在拿不到
     `plan.root`，只有 units 的 `path` ⇒ 需要调用方传根，或从 entry 路径推）；
     ② **默认开关**：设计 §8.3 说下载来的产物默认关 ⇒ **本地自己写的产物**可以默认开，
     但要给一条逃生门（同 `SOKONANODA_NO_PROJECT_ARTIFACTS` 的既有纪律 ✓）。
* **四方向账（本轮后）**：① T1-A ✓ / T1-B **批 1 ✓ · 批 2：A/B/C 三块 wire 全通，
  只剩 session 控制流接线** · ② T1-A ✓（跨条目复用） / T2-B 缓做 · ③ ✓ · ④ T4-A ✓。

### 29. 第 37 轮（平行线）：**接线完成** —— 磁盘产物真的省掉了库层 elaborate ✓✓

**这是 T1-B 批 2 的收口**：`import` 不再被重新 elaborate，而是**从产物装回来** ✓。

* **落点**（`session.rs`）：
  * 新增公开入口 **`with_project_session_artifacts(lib_units, entries, options, root, on_entry)`**
    —— 与 `with_project_session_reusing` **同一条路**，只是多一层磁盘产物。
    旧入口**原样不动**（LSP 的既有行为逐字节不变 ✓）。
    内部改成 `with_project_session_reusing_at(..., artifacts_root: Option<&Path>, ...)`。
  * **分层的顺序本身就是判据**：**① 线程局部检查点**（同进程同一刀，最快）⇒
    **①.5 磁盘产物**（**跨进程**）⇒ ② 模块前缀续编 ⇒ ③ 整条重建（**并把产物写出去**）。
    ⇒ 这就是 T4-A 那条契约的实现：**跨按键增量归检查点，跨进程增量归产物** ✓。
  * 命中产物 ⇒ `load_lib_checkpoint`：`decode_payload` ⇒
    `EnvBuilder::from_export_file` + `PassTables` + `LibPassFacts` **拼出一份
    `LibCheckpoint`** ⇒ 直接进 `run_entries`（**一趟库层都不跑** ✓）；顺手 `push_checkpoint`
    喂热线程局部（**下一刀就命中 ①** ⇒ 产物只为"冷进程"付一次 ✓）。
    ⚠ **只装 arena 不装单元**（产物里没有源文本 ⇒ 省一份 `leak_lib_units` ✓）。
  * 重建路径跑完 ⇒ `write_lib_artifact`（**best-effort**：任何一步失败都静默跳过 ✓
    —— 产物是加速件，写不出来只该"下次还慢"，**绝不该**让本次编译失败或改判 ✗）。
  * **逃生门**（本轮补）：`SOKONANODA_NO_MODULE_ARTIFACTS=1`（或既有的
    `SOKONANODA_NO_PROJECT_ARTIFACTS=1`）⇒ **读写都关**，一次收口
    （`let artifacts_root = artifacts_root.filter(|_| module_artifacts_enabled())`）✓。
* ⭐ **判据**（`crates/front/tests/t1b_module_artifacts.rs` · 集成测试 ⇒ 进程级计数天然隔离 ✓）：
  造"另一个进程"的办法 = 命中臂前 **`lib_checkpoint_reset()`**（线程局部清空 ⇒
  唯一还能省掉库层的**只有磁盘产物** ✓，正是新进程的等价物 ✓）。
  * ① **输出逐字节相同**：命中产物那条路的回调结果（`CompileOutput` + 入口报告 +
    **库层报告 C 块**）与冷跑**一字不差** ✓ —— 这条同时守住"产物不许改判"的红线 ✓；
  * ② **结构计数真的降**：冷 `by_calls=3` vs 命中 **更低** ✓（库层那三条 `by` 证明**没重跑**）
    —— ② 同时证明 ① 不是空转（若产物没被用上，两条路一样忙 ⇒ ② 判红 ✓）；
  * ③ 产物真的落了盘（`.bin` + `.meta.json` ≥ 2 个 ✓）。
  * **两条反向验证（都已做）**：`SOKONANODA_NO_MODULE_ARTIFACTS=1` 跑同一用例 ⇒
    **判红**（`冷 3 vs 命中 3` ✓ —— 逃生门真的关、判据真的有牙，同一件事的两面 ✓）。
* 验证：front **全部目标**绿 ✓（`cargo test -p sokonanoda-front` exit 0；lib **886/886**）·
  fmt `--check` 干净 ✓ · clippy `--all-targets` 报错 **0** ✓。
* **四方向账（本轮后）**：① T1-A ✓ / **T1-B 批 1 ✓ · 批 2 ✓✓（产物真的在用）**，
  剩批 3（并发/损坏/离线/`--clean` 的边界与课程门禁）；② T1-A ✓（跨条目复用） / T2-B 缓做；
  ③ ✓；④ T4-A ✓。
  ⚠ **还没做**：把 CLI `build` 接到 `with_project_session_artifacts`（一行接线 + 模块根）；
  在那之前"产物在用"只在测试里成立 ⇒ **下一棒第一件就是它** ✓。

### 30. 第 38 轮（平行线）：**产物接进 CLI** —— 端到端跑通 ✓（含实测读数与一处如实记的"没量出差别"）

* **接线**（两条，都在 `project/mod.rs`）：
  * `compile_plan_incremental`（LSP/`query` 那条）：`reuse_library` 两支都换成**带产物**的
    口子 —— `true` ⇒ `with_project_session_reusing_artifacts`（检查点 + 产物，装载进 LRU）；
    `false` ⇒ **`with_project_session_artifacts_trusted`**（**只有产物**）。
  * `run_shared_group`（**CLI `build <dir>` 的多入口路**）：`with_project_session` ⇒
    `with_project_session_artifacts` ✓。
* ⭐ **`with_project_session_artifacts_trusted` 是这轮的关键设计**（CLI 专用）：
  CLI 是**短命进程** ⇒ 不碰线程局部检查点（T4-A 契约 ✓），装载出来的检查点**只活这次调用**
  ⇒ **arena 建在栈上**（不需要 `'static`）⇒ **零泄漏** ✓
  （`lib_checkpoint_arenas_leaked()` 保持 **0** ⇒ **T4-A 的守卫原样绿** ✓，本轮实测确认 ✓）。
  于是"**跨进程增量归产物（CLI）· 跨按键增量归检查点（LSP）**"字面落地 ✓。
  装载内核拆成 `load_lib_checkpoint_in(arena, …)`（寿命由调用方负责）—— `'static` 与栈上
  两条路**共用同一段装载代码** ✓。顺带删掉被取代的 `with_project_session_reusing`
  （逃生门关掉之后它逐字节等价 ✓ ⇒ 不留两份实现 ✓）。
* **端到端实测（真 CLI · debug 二进制）**：
  * 夹具：`/tmp/t1b-proj`（`Lib` 8 条 `by` 证明 + 两个入口都 `import Lib` ⇒ **同一库闭包**）。
  * 冷跑 ⇒ `.sokonanoda/artifacts/` 出现 **2 个文件**（`.bin` + `.meta.json` ✓）。
  * 改入口一行（库层不动）⇒ 再跑 ⇒ **产物命中并装载成功**：`key=6f2527a4…` ·
    载荷 **182005 字节** · 装载出 **65 条声明** ✓；`SOKONANODA_NO_MODULE_ARTIFACTS=1`
    跑同一条 ⇒ **命中行 0** ✓（逃生门与判据同一件事的两面 ✓）。
  * ⚠ **如实记：这个夹具量不出墙钟差别**（`with=0.229s` vs `no=0.219s` —— 8 条小证明，
    进程启动占大头）。⇒ **不许**把它写成"CLI 变快了"✗；要读数得用真课程量级
    （`build courses/set-theory/units`，规划 §7 的 174 模块口径）—— **留给下一棒** ✓。
  * ⚠ **另一件如实记**：`courses/set-theory/units/I.2` 的 4 个单元**各自库闭包不同**
    ⇒ `compile_entries_shared` 分成 4 个**单入口组** ⇒ 单入口组**被过滤掉**
    （"会话是纯开销"✓）⇒ 那条路上**根本不会跑会话、也就没有产物** ✓。
    这是**既有设计**（不是本轮的 bug ✓），但它决定了"产物在真课程上能覆盖多少"
    ⇒ 下一棒量课程读数时要按**分组**看，别按"单元数"看 ✓。
* 验证：front **全部目标** 35/35 绿 ✓ · fmt `--check` 干净 ✓ · clippy `--all-targets` 报错 **0** ✓。
* **四方向账（本轮后）**：① T1-A ✓ / **T1-B 批 1 ✓ · 批 2 ✓✓（CLI 端到端已接）**，
  剩批 3（并发/损坏/离线/`--clean` 的边界与课程门禁）+ **真课程读数**；
  ② T1-A ✓（跨条目复用） / T2-B 缓做；③ ✓；④ T4-A ✓（守卫原样绿）。

### 31. 第 39 轮（平行线）：⭐ **方向① 的第一个真实读数 —— 冷进程省掉库层 elaborate，快 14×** ✓✓

* **夹具**（`/tmp/t1b-big` · 可复现）：`Lib.sokonanoda` = **60 条带 `by` 的证明**（每条
  `P ∧ (Q ∧ R)` 三步）+ `E.sokonanoda` = `import Lib` + 一条引用 `lib0` 的定理。
* **走哪条路**：`sokonanoda query check --file E.sokonanoda` —— 它经
  `compile_plan_incremental(..., reuse_library = true)`（**与 LSP 同一条** ✓）
  ⇒ 正是"新进程开一个**有 import** 的档"的等价物 ✓。
* **测法（A/B · 同一份二进制 · 同一夹具 · 交替跑）**：每轮先给 `E` 追加一个换行
  （**入口源变 ⇒ 入口缓存必 miss** ⇒ 会话必须跑），**库层一个字节不动** ⇒
  产物的键不变 ⇒ A 臂必命中 ✓；B 臂加 `SOKONANODA_NO_MODULE_ARTIFACTS=1` ✓。
* **读数（debug 二进制 · 三轮交替）**：

  | 臂 | 1 | 2 | 3 |
  |---|---|---|---|
  | **A 用产物** | **0.121s** | **0.117s** | **0.117s** |
  | B 关产物 | 1.736s | 1.691s | 1.700s |

  ⇒ **≈14×**（1.70s → 0.118s）✓✓ —— 这就是"**import 不 elaborate**"的直接读数 ✓。
* **产物尺寸**：`<key>.bin` = **953,238 字节**（60 条证明的内核环境 + 前端表 + 库层产物）
  ⇒ 装载 + 解析约几十毫秒，相对省下的 1.6s 完全划算 ✓。
* ⚠ **同一轮里的两个如实记（决定了"哪里有用、哪里没用"）**：
  1. **`build <dir>` 在真课程上几乎不形成共享组** —— 探了 `units/I.1`（15 文件）·
     `I.2`（4）· `I.3`（16）· **`lib/`（17 文件）** ⇒ **产物数全是 0**。
     原因：`compile_entries_shared` 按"**整条库闭包**"分组，而每个单元的 import 子集
     都不同 ⇒ **组都是单入口** ⇒ 被既有规则过滤（"会话是纯开销"✓）⇒ **那条路上
     根本不跑会话** ✓。⇒ **CLI `build <dir>` 不是产物的主战场** ✗。
  2. **主战场是"每次一个入口的冷进程"**（`query` / LSP 开档）—— 上面那 14× 就是它 ✓。
     这与设计 `module-artifacts.md` §2 的定位一致：产物要替代的是"**编译一个入口所需的
     依赖环境**"，而不是"一次编一整片目录" ✓。
* **判据（自动化那条仍在）**：`crates/front/tests/t1b_module_artifacts.rs`（输出逐字节相同 +
  `by_calls` 真的降 + 产物落盘；逃生门反向验证判红 ✓）。**墙钟只进本文档、不进 CI**
  （`AGENTS.md` 判据纪律②：perf 判据不许用绝对毫秒 ✓）。
* **四方向账（本轮后）**：① T1-A ✓ / **T1-B 批 1 ✓ · 批 2 ✓✓（端到端 + 14× 读数）**，
  剩批 3（并发/损坏/离线/`--clean` 的边界与课程门禁）；② T1-A ✓（跨条目复用） / T2-B 缓做；③ ✓；④ T4-A ✓。

### 32. 第 40–41 轮（平行线）：T1-B **批 3 收口** —— `clean` 带上产物 · 原子落盘 · 并发取证 ＋ 两条诚实结论

* **`clean` 必须清产物**：`artifacts::clean_in(root)`（清 `artifacts/` 下除 `.gitignore` 外 的一切，含 `*.tmp-*` 残留）接进 `cache::clean_at`。**为什么**：`compiled/` 那条有过 **同形教训**（R-3：只清全局 ⇒ `rebuild` 命中项目条目 ⇒ "清空了却什么都没重编"的假动作 ✗）； 产物是**第三个**存放点 ⇒ 不清它就重演 ✗。**端到端**：`query check` 后 `artifacts=2` ⇒ `sokonanoda clean` ⇒ `removed 3 (0 global, 3 project)` · `artifacts=0` · `.gitignore` 留着 ✓。
* **原子落盘**：载荷先写 `<key>.bin.tmp-<pid>` 再 `rename`（同目录原子）⇒ 读者不会看到半份； 残留 `.tmp-*` **不是** `<key>.bin` ⇒ 任何键都读不到（**命名即边界** ✓）。
* **并发取证**：`concurrent_writers_never_produce_a_torn_payload` —— 4 位写者同写一键 （载荷长度刻意不同）+ 读侧 4000 次 ⇒ "读到了就必然是某一份完整载荷" ✓（且至少读到一次 ⇒ 不空转 ✓）。
* ⭐ **诚实结论 A（纵深防御的边界）**：把原子 `rename` **去掉**再跑同一条 ⇒ **照样绿**。 ⇒ 真正兜住的是**凭据里的摘要**（第②道），**不是**原子性 ✗。原子 rename 的价值是 **减少无谓 miss 与半份读**（纵深防御），**不是正确性必需** —— 已写进代码注释 ✓。
* ⭐ **诚实结论 B（新纪律：冷开必须先清产物）**：全量 front 测试逮到一次**真实交互** ✗ —— `judge_synthesized_typing.rs` 的夹具自检（"冷开必须真的跑合成趟"）判红 **实测 0 趟**。 根因**不是 bug**：模块根里已有**磁盘产物**（CLI 探针 + 课程门禁写下的）⇒ `QueryDoc` 的 "冷开"**从产物装载库层**（**那正是特性**：`import` 不再 elaborate）⇒ 自检前提不成立 ✗。 ⇒ **纪律**：**凡断言"冷开 = 全 elaborate"的用例，冷开前先清该模块根的产物** （`.sokonanoda/` 是自忽略缓存目录 ⇒ 清它是**准备动作**、不是断言 ✓）。已按此修好该用例 （`cache::clean_at(module_root)` + 注释写明理由），并在 `artifacts.rs` 文档留同一句 ✓。 ⚠ **影响下一棒**：任何"冷开读数/断言"先问"模块根里有没有产物" ✓。
* ✅ **T1-C 收口（第 54 轮）**：契约 = **键优先、mtime 只兜底** —— **mtime 变、内容不变 ⇒ 不许 miss** ✓ · **内容变、mtime 不变 ⇒ 必须 miss** ✓（后者要命：否则"改完再把时间戳改回去"就能读到**旧环境** = 静默错判 ✗✗）。 守卫 `crates/front/tests/t1c_key_beats_mtime.rs` 用**产物文件本身**观测（命中 ⇒ 不重写 ⇒ mtime 不动 ✓）； **反向验证已做**：给 `lib_key` 临时混入时钟 ⇒ 判据**判红** ✓（守卫有牙 ✓，已还原 ✓）。
* **批 3 状态**：`clean` ✓ · 原子 ✓ · 并发取证 ✓ · 损坏（三道，已有判据）✓ · **离线 = 本版 n/a**（不联网 ✓）· **课程门禁 ✓**（`gate --fast` PASS ✓）⇒ **批 3 收口**； 方向① 只剩 §8.4 的**下载线**（独立工作流，未开工）。
* 验证：front **全部目标 35/35** 绿 · fmt 干净 · clippy **0** 报错 ✓。
* ⭐ **四方向结账（第 55 轮 · 全量 `scripts/soko gate` PASS ✓）**：**①** T1-A ✓ · T1-B 批 1/2/3 ✓✓ · 有界化 ✓ · **T1-C 契约+守卫 ✓**；**②** T1-A ✓（跨条目复用） · 静默期修复 ✓ · **T2-B0 ✓✓**（两张派生表 O(闭包)→O(入口)）· T2-B **缓做** （已量上界 ≈30–40ms、前置件大）；**③** T3 ✓ **全收口**（合成趟 Σ20→0 · A7 靶子关闭（memo 命中 90.3% ✓）· **T3-D 死线已删** ✓）； **④** T4-A ✓（契约+守卫）· T4-B 前置件就绪（**阻塞①已核实**：CLI 确实传 sink ⇒ 需先给 session 开 sink 通道）· T4-C 未做（大件 ✗）。**读数**：真实连续键入 **78.0ms** · 产物 **14×**（前端 · §31）· **9×**（CLI · §36）· 冷开第一刀 **333 → 127.2ms**（A5c/A5d ✓）。⭐ **但北极星要按两个口径读**（第 64 轮 · 并行线的真读数 `docs/notes/perf-lean4-bench.md` ✓）：**诊断全量落地 soko 81.4ms vs Lean 218.1ms ⇒ 快 2.7×** ✓✓； **goal 更新完成 soko 81.4ms vs Lean 3.1ms ⇒ 慢 26×** ✗✗（Lean 靠 **per-command 快照** ✓ = 方向② **T2-B** 那一格 ✓，前置件 T2-A **并行线在飞** ✓）⇒ **"打平并超过"只在诊断口径成立，整体还没达成** ✗。
  ⚠ 那份实测的**臂 B′（开档即敲）232.7ms** 是 **A5c/A5d 之前**的数（它自己写着 `warm-library built=true 217ms` ✓）
  —— 那条**预热赛跑**已被本线拆掉 ⇒ 同臂现在 **127.2ms** ✓，重测时要换新数 ✓。

* ⚠ **§21 的天花板要按 A6b 之后重估（第 58 轮 · 只读）**：`TcCache::new`（`kernel/src/util.rs:1291`）今天已是 **10 张空 HashMap + 三个定长数组**（`Box<[u64;1024]>` ×2 + `whnf_admit` ✓ —— A6b 已把 **4 MiB 预分配**拿掉 ✓） ⇒ 单次 ≈ **2–3µs**（不是 §20 量到的 9.2µs ✗）⇒ 稳态那一刀 `tc=4225` ⇒ 全部 ≈ **10ms / 79.7ms（~13%）** ✓。 ⇒ **"按趟复用 TcCache"（§21 候选 1）今天的天花板只有 ~10ms**，而它要动 `with_tc` 的签名链 + 内核相位 ✗ ⇒ **性价比已经不高**：下一棒别照 §20 的旧数字投 2–3 轮进去 ✓（真要碰，先重量单价再决定 ✗）。

* ⭐⭐ **A5d（第 59 轮）：模块产物在飞 ⇒ 一律不做投机预热** ✓✓ —— 同一探针、同一构建、只切 `SOKO_NO_LIB_WARMUP` 两档：**跨入口切换 911.1ms → 715.7ms**（`modules=8`/复用 **0** ⇒ `5`/复用 **3**）· **第一刀 333 → 124.7ms** · 其余四臂逐位持平 ✓。根因：A5 的预热与产物**做的是同一件事**， 而编译钉在**一条** worker 线程上 ⇒ 预热既**挡用户那一刀**、又**挤掉多槽 LRU**（"复用 3→0" ✗）。 ⇒ 判据 = **产物这条路开着 ⇒ 整条预热都是重复劳动 ⇒ 跳过** ✓（守卫 `front/tests/a5d_warmup_is_skipped.rs`： 断言**返回值**、**反向验证已做**（去掉早退 ⇒ 当场判红 ✓）；逃生门关掉产物时才照旧预热 ✓ —— A5 的三条判据正是那样跑的 ⇒ 仍 **3/3** ✓）。落地后实测：第一刀 **127.2ms** · `typing` **78.0ms** · `equal_length` **16.3ms** · **跨入口 664.8ms**（`modules=5`·复用 3 ✓；A5c 前 689.7、预热开 911.1 ✗）。

* ⭐ **逐声明归因（第 60 轮 · 内置量具 `SOKO_DECL_PROFILE=1` ＋ `_MS=0`，跑在 `judge_synthesized_typing` 的**真实连续键入**臂上 ⇒ **在进程内**、不靠猜墙钟 ✓）**：970 条事件 / 71 个声明，耗时 top： **`image_inter_subset` 120.4ms（×13）** · `Set.sep_self` 40.9（×1）· **`image_empty` 38.5（×13）** · **`image_preimage_subset` 32.4（×13）** · **`inter_singletons_empty` 25.0（×13）** · `<example>` 22.5（×39） · `Set.image_mono` 20.9（×1） · `Set.image_subset_iff` 13.3（×1）。⇒ **每刀**：声明 elaborate 合计 **≈24ms**（其中 `image_inter_subset` 一条就 ≈9.3ms ✗ —— 它排在**编辑点之后** ⇒ 每刀都被重做 ✓）， 而 78ms 的**其余 ≈54ms 在声明之外**（解析/望远镜/report/judge 的**前缀键 SipHash** ✓ —— 后者正是 §8.9 的 A7 那条 22.5% ✗，且它的红线是"**不换弱哈希**"）✓。⇒ 下一棒的两个具体靶子：① `image_inter_subset` 这类"**编辑点之后但依赖没变**"的声明（= T2-B 的深水区 ✓）；② A7 的前缀键（**先读 §8.9 的红线** ✓）。

* ⚠ **A7 的 22.5% 要先重量再投（第 61 轮 · 只读复核）**：`judge.rs` 的前缀键今天已经是 `canonical_text_key` 的**小 LRU**（4 条 · 命中判据**逐字节相等** ✓ · 哈希函数一字未动 ✓ —— 红线"**不许换弱哈希**"守住了 ✓）。⚠ 但那条 memo 的**命中路径本身是 O(前缀长度)**（`text == src`）， 而**未命中**路径要重跑整段 SipHash ⇒ 一次编译里问几百次时，**4 条够不够**是个**没量过**的问题 ✗。 ✅ **量完了（第 62 轮 · `front/tests/a7_text_hash_memo_stats.rs`）**：冷开 **4242 命中 / 758 未命中** · **每一刀 955 / 28**（四刀完全相同 ⇒ 确定性 ✓）⇒ 合计 **8062 / 870 = 命中率 90.3%** ✓。 ⇒ **`MEMO = 4` 是够的**（残余 28 次未命中/刀 × 一次 SipHash ≈ **0.14ms 量级** ✓）⇒ **A7 那条 22.5% 已经过期** ✗ ⇒ **这个靶子可以划掉、别再投** ✓（红线"不许换弱哈希"也就不用碰 ✓）。
* **产物/缓存的体格（同轮实测）**：真课程 unit08 的产物载荷 **1,148,482 字节**（`.meta.json` 158 ✓）· `compiled/` 在探针跑完后为 **0**（各臂按纪律清过 ✓）⇒ **写产物 ≈ 1–3ms 量级**，不是热按键的大头 ✓； 真正的大头仍是"**声明 elaborate ≈24ms** + 声明之外 ≈54ms"（见上一条的逐声明归因 ✓）。

* ⚠ **在制品协调（第 63 轮）**：并行线正在 `crates/kernel/{builder,util}.rs` 上做 **`EnvBuilder::clone` 常量化**（新用例 `builder::tests::env_clone_cost_is_constant_in_declaration_count` ✓ —— 那正是 **T2-A（声明表持久化/COW）** 这条 XL 前置件 ✓，也是解锁 T2-B 深水区的那一件 ✓）。 它**当前未提交且未绿**（全量 gate 判红 = 它的新用例 ✗，不是本线的改动 ✓）⇒ 本线**不碰**那两个文件， 等它落地后再重量北极星（克隆常量化会直接改热按键的成本结构 ✓）。

* ⚠ **goal 口径的臂"试过、挂住、已撤"（第 65 轮 · 时间盒）**：照 `perf-lean4-bench.md` §3.3 的 方法在**本仓探针**里加一条 goal 臂（编辑刀 = 在 `↔ (A ⊆ (f ⁻¹' B)) := by` 后插 `constructor` ✓ ⇒ 真的改目标 ✓），用 `soko/goals` 问"换没换" ✓ —— **首问就挂死在 `wait_for` 上** ✗ （编译落定前 `soko/goals` 不答该请求 ⇒ 探针阻塞 ✗）。⇒ 按纪律**已还原**（树绿 ✓）。 **下一棒要做这条臂**：先确认"未落定时 `soko/goals` 到底答不答"（要么加超时、要么只问 **落定之后**那一次、把"立刻"那一问改成**带超时的旁路** ✓）—— 别照 §3.3 直接照搬 ✗。

* ⭐ **跨入口臂也钉住状态了（第 66 轮）**：那条臂此前只清 LSP 自己的 cache 目录 ✗，**没清模块根** ⇒ 同一构建在两轮里读出过 **689.7 / 664.8 / 911.1ms**（结构计数 `modules=5`/复用 3 **vs** `8`/复用 0 ✗） —— 那是**状态差**、不是性能差 ✓。⇒ 现在开头统一清 `<模块根>/.sokonanoda/{artifacts,compiled}` ✓ ⇒ 连跑两次：**663.5 / 675.8ms**，**结构计数逐位相同**（`modules=5` · 复用 3 · `by=21` ✓） ⇒ 这一臂的读数从此**可跨轮比较** ✓（墙钟 ±2% 是正常的 ✓，结构计数才是判据 ✓）。

* ⚠ **复跑外部量具（第 67 轮）：读数**低于**文档里的数 ⇒ 先别当改进** ✗。用 `~/lean4-bench/lsp_bench.py` 跑 soko 侧（`--settle diag` 臂 **17.27ms** · `typing` 臂 **29.29ms** · 其中 `goal_ms=13.48`）—— 而文档记的是 **232.7 / 81.4ms** ✗。**根因未定**：量具的 JSON 里**没有**"goal 真的换了"这个**可机检**字段 （§3.3 的"答案换了"是**手工**核的 ✓）⇒ 13.48ms 可能是**旧 goal**（那正是要防的"陈旧答案" ✗）， 29.29ms 也可能来自**静默期合并**（连发多刀被 debounce 合掉 ✓）。⇒ **不许**把它记成"goal 口径已达标" ✗； 下一棒要复跑：给量具补一个**可机检**的"goal 变了没"（或逐刀 `waitForDiagnostics` 再发下一刀 ✓）。

* ⛔ **T4-B 的 CLI `check` 接线：试过、判红、已撤（第 68 轮）** —— 但**逮到一条真缺口** ✓✓： `crates/cli/src/check.rs` 那条路**没有 progress sink**（`compile_plan` = `…_with_progress(None)` ✓） ⇒ 本来是最安全的换法 ✓（换 `compile_plan_with_artifacts` ＋ 显式 `precheck_plan` ✓）。换完 **`cli/tests/namespace.rs::export_reaches_the_importing_file_while_open_does_not` 判红** ✗ （`unknown identifier \`mem\`` ⇒ **`export` 没传到导入方** ✗）⇒ 已还原（该测试回绿 ✓）。 ⭐ **根因缺口不在产物、在判据**：`t4b_plan_parity` 的夹具**没有 `export`** ✗ ⇒ 两条路在 "**导出传播**"这一维上**可能不同**而它看不出来 ✓（判据的覆盖缺口 ✓）。⇒ 下一棒要做 T4-B： **先把 `export` / `open` / 命名空间这几维加进 parity 夹具**（并让它先红一次 ✓），再谈换路 ✓。

  ✅ **已在 front 层最小复现（第 69 轮）**：`t4b_plan_parity.rs` 新增 `the_export_dimension_is_a_known_divergence_today`
  —— 同一份 `export` 夹具跑两条路：**整条一趟 = `[]`** ✓ · **session+产物 = `["elab-unknown-identifier"]`** ✗
  ⇒ 分歧被钉成**快速判据**（0.05s ✓，不再依赖 CLI 集成测试 ✓）。判据按降级纪律写成
  "**断言当前行为**" ✓：T4-B 对齐之后**改判**成 `assert_eq!`（不许放宽 ✗）。

* ✅ **T3-D 的处置决定（第 71 轮 · 只读取证）**：`EnvProvider` 今天确实是**死线** —— `provider:` 这个形参只出现在**定义处**（`judge.rs:2707` 的 `Option<&dyn EnvProvider>` ✓），**一个传参调用点都没有** ✗ （`grep "provider:"` 除函数签名/`trait`/注释外**零命中** ✓）。⇒ 按 T3-D 自己给的二选一，取**删掉它** ✓： 活的那条路是 `InplaceEnv`（T3-B1/B2 已落地 ✓），而"按 `InplaceEnv` 的形状重写再接线"= **再造一个 同一机制的入口** ✗。⇒ **执行清单（下一棒，一次 commit）**：① 删 `trait EnvProvider`（`judge.rs:2944`）+ 那个形参 + 测试里的 `Fake` 实现（`:3758`）；② 按纪律**先改台账** G-92 的 `expected_lean` （别在文档里留一个不存在的接口 ✗）；③ 判据 = `judge` 单测全绿 + 该处 `grep EnvProvider` 归零 ✓。
* ⚠ **本轮的边界**：并行线在 `check/{mod,walk}.rs` + `project/session.rs` 上做 T2-B（三个文件未提交、 当前 `cargo check -p sokonanoda-front` **6 个错** ✗）⇒ 本线**不动这些文件**、也**跑不了任何编译判据** ✗ （这正是 T3-D 只做到"取证 + 决定"的原因 ✓）。

* ⚠ **T3-D 的执行本轮试了 3 次、全部还原（换招：下次手工做 ✓）**：三处**脚本化删除**都误伤 —— ① 字符偏移法在文档块边界上切错（留下半行 ✗）；② 行法把 `#[test]` 属性留下 ⇒ "duplicated attribute" ✗； ③ 第三次"找悬空 `#[test]`"的判据写歪 ⇒ **删掉了旁边一条好测试的 `#[test]`** ✗（`the_text_hash_memo_never_changes_a_key`）。 ⇒ 按"同一处连红 3 次必须换招"的纪律**停手还原** ✓（还原后 `--all-targets` 0 错 · `judge` 单测全绿 ✓）。 **下次手工做的清单（已核实、别再写脚本 ✗）**：① `judge_infer_with_env`（含其长文档块）；② `provider_inputs` （含文档块）；③ `pub trait EnvProvider`（`:2944`，含那段"形状还要重设计"的长文档 —— 建议**换成一条短记**留下线索 ✓）； ④ 测试 `env_provider_is_consulted_and_none_falls_back_byte_for_byte`（含内部 `impl EnvProvider for Fake` ✓）； ⑤ 台账 G-92 的 `expected_lean`；⑥ 判据 = `judge` 单测全绿 + **代码里** `grep EnvProvider` 归零（注释里的历史线索可留 ✓）。

* ⭐ **方向③ 收口后的全臂重量（第 73 轮 · 构建 `…514384`）**：**真实连续键入 82.5ms** · 开档后第一刀 **131.9ms** · `proof`/`typing_equal_length` 16.6/17.2ms · `trailing_comment` 41.3ms · **跨入口 751.4ms**。 ⚠ **比上一组（78.0 / 127.2 / 38.1 / 663.5）整体慢 3–13%** ✗，但 **结构计数逐位相同** ✓ （`by=9`·`tc=4225`·`modules=1`；跨入口 `modules=5`·复用 3·`by=21` ✓）⇒ **按判据纪律这是噪声/负载， 不是结构性回归** ✓（两条不可比：① 构建不同（本组含 T3-D ✗）；② 并行线此刻正在同一台机上跑 自己的活 ✗）⇒ **墙钟只作同机前后参考、结构计数才是判据** ✓，这里**不做"变慢了"的结论** ✗。

* ✅✅ **`export` 漏已修（第 74 轮 · 两行 + 一个开关）** —— 这是**用户可见**的修（LSP 那条路也走 session ✓）： ① `run_library_pass` 的 `snapshot_state` 从 `false` → **`true`**（库层趟的累加状态里带着 `exports` ✓； 以前 `false` ⇒ 状态是 `None` ⇒ 入口趟拿到**空**状态 ✗）；② 检查点的 `resume` 收**真状态**（不再恒 `default()` ✓）； ③ 入口趟的 `run_pass_with` 从 `None` → **`Some(lib.resume.clone())`** ✓。⇒ 判据读数从 `整条一趟=[] session+产物=["elab-unknown-identifier"]` ✗ 变成 **两侧都是 `[]`** ✓✓ —— 且那条判据按 它自己的指示**改了判**（`the_export_dimension_is_a_known_divergence_today` → `…_agrees_on_both_paths`， `assert!(!b.is_empty())` → `assert_eq!(b, a)` ✓，**不许再放宽** ✗）。
* ⇒ **T4-B 的前置从此全绿** ✓（CLI `check` 换 `compile_plan_with_artifacts` 那条路现在安全了 ✓ —— 但**下一轮**再做， 一轮一件 ✓）：`cli/tests/namespace` **10/10** ✓ · `t4b_plan_parity` **2/2** ✓ · front **41 个目标全绿** ✓ · fmt 干净 · clippy `--all-targets` **0** 报错 ✓。

* ⚠ **红线的现状（第 75 轮）**：`export` 修复是**行为改动** ⇒ 该跑"整门课逐字节"那道 ✓。**全量 `scripts/soko gate` 判红** ✗ —— 但红在 **`crates/kernel/tests/memory_api.rs::cross_builder_name_lookup_is_silently_positional_without_with_env`** （**内核**测试 ✓，与本次改动（只在 `front/project/session.rs`）**逻辑无关** ✓；那一带正是并行线 `EnvBuilder` 指针同一性判据的地盘 ✓）⇒ 归并行线/内核线 ✓，不是本修复的回归 ✓。
* ✅ **课程那条红线**：单独跑 **`python3 courses/set-theory/tools/check.py`** ⇒ **exit 0** ✓ （**258 个目标 · 2188 checked · 916 open · 0 个被判负** ✓）⇒ 本次行为改动**过了课程门禁** ✓。 ⚠ 待办：等内核那条测试回绿后，再跑一次**全量 gate**（那里才有"整门课 `--json` 逐字节"那道 ✓）。

* ✅ **T4-B 的 CLI `check` 换路落地（第 76 轮）**：`check.rs` 从 `compile_plan`（整条闭包一趟 ✗） 换成 **`precheck_plan` + `compile_plan_with_artifacts`** ✓（session + 磁盘产物、且不碰线程局部检查点 ✓）。 三条前置都核实过：① 报告逐字节相同（`t4b_plan_parity` ✓）；② 本调用方**没有** progress sink （`compile_plan` = `…_with_progress(None)` ✓）⇒ 不存在丢进度事件的问题 ✓；③ **`export` 已对齐** ✓ （第 43 轮正是死在这条上 ✗）。验证：`cli/tests/namespace` **10/10** ✓ · **CLI 全套 40 个目标 exit 0** ✓。 ⚠ **如实记：没有量到墙钟收益** ✗ —— 夹具上两臂都是 **~0.035s**（`compiled/` 报告缓存 + 全局缓存把两臂都 短路了 ✗，与第 13/43 轮同一条教训）⇒ 本次换路的依据是**判据（报告逐字节）+ 与 `query` 共用产物路** ✓， 不是墙钟读数 ✓（要量收益得先让报告缓存 miss 而产物命中，夹具没做到 ✓）。

* ⛔ **全量 gate 的红已定位到并行线的 `b10fa175`（T2-A）—— 它踩了自己那条判据** ✗✗： 红点是 `crates/kernel/tests/memory_api.rs::cross_builder_name_lookup_is_silently_positional_without_with_env` （K1-b 的"**跨 builder 名字查询不许静默取错**"守卫 ✓），报 `const_head_type: unknown const NamePtr(0x…)` ⇒ **名字被"找到"了、找到的是别人** ✗。`git log -- crates/kernel/src/{builder,util}.rs` 的第一条就是 **`b10fa175 perf(kernel): T2-A —— 声明表分层持久化 / COW`** ✓ ⇒ 归它 ✓（内核那两个文件**没有**未提交改动 ✓）。 ⚠ **要命的是**：本规划 §T2-A 的判据清单第 ② 条**自己写着**"指针同一性**两向**判据保持绿 （`crates/kernel/src/builder.rs` 那条）" ✗ ⇒ T2-A 落地时**没守住自己的验收条件** ✓。 ⇒ **本线不修**（那是 T2-A 的语义地盘：持久化/COW 会改变跨 builder 的 interning 语义 ✓，是"守卫要重设计" 还是"实现有漏"得由内核线定 ✗）；但它**卡住全量 gate** ⇒ 也卡住本线"整门课 `--json` 逐字节"那道红线 ✗。

* ⭐⭐ **T2-B 落地后的读数（第 77 轮）** —— **结构证据（无歧义 ✓）**：并行线的读数 `PERF t2b 改最后一条：elaborated_commands=1（入口命令数 N=12）` ✓✓（12 → **1**）； 本线探针同族证据：`trailing_comment` 臂从 **`by=9` → `by=1`、`tc=0`** ✓（改**文件末尾**那条 ⇒ 只重编 1 条 ✓）。 ⇒ 这正是 Lean 的 **per-command 快照**行为 ✓（它也"编辑点之后的声明还在编" ✓ ⇒ 两者同构 ✓）。 **goal 口径（外部量具 `~/lean4-bench/lsp_bench.py` · 同一夹具 n=5）**：`goal` median **13.48 → 8.72ms** ✓ · `first_diag`/`complete` **29.29 → 27.05ms** ✓（Lean 侧：goal **3.1ms** / 诊断 218.1ms）。 ⚠ 两句限定语：① 该量具**没有**"goal 真的换了"的可机检字段（第 42 轮记过 ✗）⇒ 8.72ms 是**推断** （若答的是旧值，延迟会是 ~0.3ms 而不是 8.72ms ✓）；② 它的绝对值与本线探针**不同源** ✗（27 vs 84ms） ⇒ **只做同量具的前后比较** ✓：goal 相对 Lean 的差距从 **~4.3×** 收窄到 **~2.8×** ✓。

* ✅✅✅ **全量 `scripts/soko gate` PASS（第 78 轮）** —— 自第 55 轮以来第一次 ✓，也是本线两处**行为改动** （`export` 可见性修复 ✓ + CLI `check` 换路 ✓）的**权威红线**：fmt · clippy · `cargo test` · playground 锚点 · **课程门禁** · 台账门禁**全过** ✓（跑在含并行线 `kernel/src/env.rs` 在制品的树上 ✓ ⇒ 他们的 WIP 也编译/格式干净 ✓）。 ⇒ 第 52/54 两轮欠的那道**补上了** ✓（课程门禁 exit 0 是单跑时的证据 ✓，这里是整条 CI 门禁的合并证据 ✓）。

* ✅ **补上 T2-B 的"进程隔离"那一半（第 79 轮）**：`STATUS.md` 把"**进程隔离的 A/B**"列成 T2-B 的 **已知验证缺口** ✓（同进程两臂会被**线程局部 `EntryCache`** 互相喂缓存 ✗ ⇒ 不可比）。本线的按键探针 **天生是跨进程的**（真 `sokonanoda-lsp` 子进程 + 真 stdio ✓）⇒ 已把它的 `trailing_comment` 臂 （只往**文件末尾**追加注释、不移动任何命令起点 ✓）立成**判据**：`assert!(trailing.by <= 3)` ✓ —— 实测 **`by=1`** ✓（T2-B 前是 **9** ✗ ⇒ 回归必红 ✓，用**结构计数**不用毫秒 ✓）。 ⚠ 仍缺的：真正带**开关**的 A/B（同一构建跑两臂 ✗ —— 今天没有"关掉 T2-B"的逃生门 ✓）； 本轮交的是**跨进程复现 + 回归守卫**这一半 ✓。

* ✅✅ **T2-B 的"同一份构建 A/B"补上了（第 80 轮 · 加逃生门 `SOKO_NO_ENTRY_SNAPSHOT=1` ✓）**： `STATUS.md` 那条已知缺口（"进程隔离的 A/B ✗ · 同进程两臂被线程局部 `ENTRY_TAILS` 互相喂缓存"）**闭合** ✓ —— 逃生门只关"入口趟续编"、不碰任何判定语义 ✓（续编与否的报告已判据化为逐字节相同 ✓）。 **同一构建 `…517033` · 真子进程 · 同一夹具的 A/B**：`trailing_comment` 臂 **门开 `by=1`** ✓ vs **门关 `by=9`** ✓ —— 而且**门关那一臂当场把上一轮加的守卫判红** ✓（`assert!(by <= 3)` 报"实得 9" ✓）⇒ **守卫有牙** ✓（反向验证 ✓）。 ⇒ 三件一起成立：**跨进程读数 ✓ + 同构建 A/B ✓ + 回归守卫咬得住 ✓**。

* 🔎 **goal 口径那 8.72ms 花在哪：读到三处、定了一条候选刀（第 81 轮 · 仍未测 ✗）**： ① **静默期不是它** ✓ —— `debounce_from_env` 默认 **120ms**（`crates/lsp/src/lib.rs:803` ✓，理由写着 clangd 的"用户停手"判据 ✓）， 而两个量具**都没设** `SOKO_DEBOUNCE_MS` ✗（`grep` 零命中 ✓）；8.72ms **小于** 120ms ⇒ 说明 `goal_ms` 是**在诊断落地之后**才起表 ✓ ⇒ 它量的是**请求自身**的成本 ✓（不是"编辑→目标出现"的全链 ✓ —— 全链在真用里要加 120ms 静默期 ✓）。 ② `soko/goals` 的入口是 `goal_decls(probe = **true**)`（`lib.rs:1182` ✓）—— 即**请求期跑内核探针**填函数 spine 子洞的期望类型 ✓ （`nextHole` 那条专门用 `probe=false` 就是为了"不引入内核成本" ✓）。③ 报告本身取的是"**上一次编译完的**"（`doc.report()` ✓） ⇒ 这两条合起来指向**候选刀**：把 `probe` 变成**惰性/按需**（或对"只看目标文本"的客户端走 `probe=false` 快路 ✓）—— ⚠ **先测再动** ✗：还没量过 `goals(true)` vs `goals(false)` 的差 ✓（下一刀第一步 = 量它 ✓，别凭猜改 wire ✗）。

* ⛔ **候选刀已量、已否 ✗（第 82 轮）**：加了逃生门 `SOKO_GOALS_NO_PROBE=1`（只关请求期探针 ✓）跑**同一构建 A/B**： **goal median `7.09ms`（探针开）vs `7.30ms`（探针关）** ✓ ⇒ **探针不贵**（差在噪声内、开还略快 ✗） ⇒ 第 81 轮那条候选刀**作废** ✓，逃生门**已还原**（`git checkout` · 0 错 ✓，不留改 wire 语义的死开关 ✗）。 **redirect**：8.72ms 里**没有**内核成分 ✓ ⇒ 剩下的是**请求往返 / 锁 / JSON 组装**这类固定开销 ✓ （对照 Lean 的 3.1ms ✗ ⇒ 差距在**协议与 handler 的固定成本**，不在 kernel ✓）；⚠ 同轮 `complete` 两臂 21 vs 83ms ✗ **判为噪声**（共享机 + 该臂与 goal 独立 ✓）—— 不作结论 ✓。

* ✅ **goal 口径有了**一等读数**（第 83 轮）**：`perf_keystroke_wallclock` 里新增 `PERF goals-wallclock` —— 在**同一条真子进程 harness** 上量 `soko/goals` 的请求往返（5 次取中位 ✓），与诊断臂**同源可比** ✓ （此前这第二臂只活在外部 Python 量具里 ✗）。**首读（构建 `…517542` · unit08）**： **goal `best 5.8 / median 5.9 / worst 6.6ms`** ✓ —— 对照 Lean `plainGoal` **3.1ms** ⇒ **~1.9×** ✗ （比外部量具的 8.72ms 还好 ✓，两者**不同 harness** ⇒ 只各自与 Lean 同量具比 ✓）；同轮诊断臂 **typing median 74.9ms** ✓ vs Lean 218.1ms ⇒ **2.9×** ✓。

* ⭐ **goal 的 5.9ms 定位到**handler 自己**（第 84 轮 · 加了 `PERF rpc-floor` 读数）**：同 harness 里量一条 **几乎零计算**的请求 `soko/version`（只答 `{version,pid}` ✓）⇒ **median `0.0ms`** ✓（best 0.0 / worst 0.1 ✓） 而 `soko/goals` 是 **5.9ms** ✗ ⇒ **往返本身不是成本** ✓（第 61 轮也证了探针不贵 ✗）⇒ 全在 handler 里 ✓。 读码收口到**一处**：`QueryDoc::goals`（`query/mod.rs:1398`）**对每条声明**调 `self.runs(...)` （`:1079`）⇒ 每条类型文本**各 tokenize 一遍** ✓（unit08 十几条 ⇒ 十几遍 ✓ = 毫秒级 ✓，与读数同量级 ✓）。 ⇒ **下一刀（已定，未做 ✗）**：给 `runs` 的**结果**加一层**按 `(text, binders, notations)` 键**的 LRU ✓ （与仓里既有的 `canonical_text_key` memo 同款 ✓）—— 编辑一条时其余十几条的**类型文本没变** ⇒ 全命中 ✓ 预期 **5.9 → ~1ms**（**低于 Lean 的 3.1ms** ✓）。⚠ 实现要点：键必须含 `binders`/`notations`（它们进 `runs` ✓）、 LRU 有界 ✓、且**先建判据**（同一文档连问两次 ⇒ 第二次必须**逐字节相同** ✓ + 命中率读数 ✓）。

* ⛔ **`runs` 记忆化：写了、量了、**没收益**、已还原 ✗（第 85 轮）** —— 并**纠正第 84 轮的定位** ✗： 按"读码 + 量级猜测"把 5.9ms 归到 `QueryDoc::goals` 里**每条声明各 tokenize 一遍**的 `runs()` ✓， 照下一刀清单加了**线程局部 + 有界 LRU**（键 = 全部入参 ✓、函数内自足 ✓、`--all-targets` 0 错 ✓、 front **41 个目标全绿** ✓）⇒ 但读数**一动不动**：`goals-wallclock` **5.9ms → 5.9ms** ✓（构建 mtime 变 ✓） ⇒ **记忆化是中性**（要么没命中 ✓、要么贵因根本不在这一层 ✓）⇒ 按纪律**已还原** ✗（无读数收益的代码 = 债 ✓）。 ⭐ **教训（与我自己的纪律对着干的一条）**：第 84 轮我把"**读码推断**"当成了定位 ✗ —— 而仓规写着 "**先测再动**"；这轮就是它的代价 ✓（一轮工作量 ✗）。⇒ **下一刀第一步必须是**goals() **的内部分段计时** ✓ （`parsable` / `decl_kinds` / `notation_symbols` / `redundant_hole_spans` / 逐条 `runs` 各占多少 ✓）， **量出大头再动手** ✗，不许再靠读码猜 ✓。

* ✅ **goal 5.9ms 第一次**真分层了**（第 86 轮 · 新增 `crates/front/tests/goals_cost.rs` ✓）**： **front 层**（13 条声明的合成夹具）`goals(false)` median **0.19ms** ✓ · `check()` **0.00ms** ✓ ⇒ 对照真子进程同一请求 **5.9ms** ✗ ⇒ **大头不在 front**（也早已排除往返 0.0ms ✓ 与内核探针 ✓）。 ⚠ **一条如实的限定**：那个夹具的类型是平凡的（`P`/`Q`）⇒ 只能当**下界** ✗；换成**真 unit08 文本**的那条 读数据实**跳过**了（`import Set` ⇒ 单文件 `QueryDoc` 解不了闭包 ✗，报文里写明原因 ✓）—— **下一刀**：给那条读数摆上 `path`/`root` 走**项目模式** ✓，才能真正判定"5.9ms 在 LSP 层还是在 front 的重类型工作量" ✓。 ⚠ 顺带记：第 85 轮临时插桩**又**栽在"多块文本手术"上 ✗（`unexpected closing delimiter` ✓）⇒ 已还原 ✓； 本轮改走**只新增测试文件**的路子 ✓（零侵入 ✓）—— 这条路子本次证明是安全的 ✓，后续插桩优先用它 ✓。

* ✅✅ **5.9ms 的分账量出来了（第 87 轮）**：真 unit08 文本走**项目模式**（`doc.path = Some(entry)` ✓） ⇒ front 层 `goals(false)` median **2.16ms** ✓ · 真子进程全程 **5.9ms** ✗ · 往返 **0.0ms** ✓ ⇒ **front ≈2.2ms（37%）· LSP 层 ≈3.7ms（63%）** ✓ —— 两块都有份 ✓，下一刀可以**各打各的** ✓。
* ⛔⛔ **`runs` 记忆化：两次实测都**中性**，别再试第三次 ✗**（第 85 轮在 LSP 层 5.9→5.9 ✗；第 87 轮在 front 层 **2.16→2.08ms** ✗）⇒ `runs` **不是**这两层的贵因 ✓（两轮都已还原 ✓）。⚠ **按"同一处连红 3 次必须换招"的纪律**， 这里**预先记死**：不许再给 `runs` 加记忆化 ✗ —— 下一步只能**换层找**（front 侧：`probed_report` / `decl_kinds` / `notation_symbols` / `redundant_hole_spans` / `display.fold` 谁是大头 ✗ **仍需先量** ✓；LSP 侧：锁 / `decl_info` 映射 / 响应里**整份文档文本** ~10KB 的序列化 ✓）。量法沿用本轮证明安全的**只加测试文件** ✓。

* ✅ **front 那 2.16ms 的成本形状量清了（第 88 轮 · 三组控制实验，条数都固定 13）**： **平凡类型 0.20ms** ✓ · **深链类型（有 goal）1.11–1.31ms** ✓ · **同款深链但 `axiom`（无 goal ⇒ 不调 `fold`）0.44ms** ✓ ⇒ ① 成本**随类型文本规模走**（与条数无关 ✓，0.20→1.31 是 6.5× ✓）；② 其中 **≈0.7–0.9ms 是逐条的 `display.fold(goal)`**（只有**开放**声明才走 ✓ = 出题场景的主流 ✓）；③ 余下 ≈0.4ms 与类型文本本身有关 （`runs` 已两次实测否掉 ✗ ⇒ 剩下的是每文档一次的 `notation_symbols`/`decl_kinds` 这类 ✓）。
* ⇒ **下一刀二选一（都已量过、都要先建判据 ✓）**：(a) **front 的 `fold` 记忆化** —— 吃 ≈0.7ms（2.07→~1.3ms ✓， ~12% of 总 5.9ms ✓，**中等收益** ✗）；(b) ⭐ **LSP 那 3.7ms**（**63%** ✓ 的大头 ✓）—— 嫌疑 = `doc.lock()` / 逐条 `query_map::decl_info` 映射 / 响应里**整份文档文本 ~10KB** 的序列化 ✓ ⇒ **优先做 (b)** ✓（(a) 记着备用 ✓）。

* ⏭ **第 89 轮：LSP 那 3.7ms 的第一刀**未落地**（如实记 ✗）** —— 想做的是"先量响应体积"（`goals` 的响应里带 **整份文档文本** ~10KB ✓，若体积是主因 ⇒ 客户端 JSON 解析与序列化就是那 3.7ms 的大头 ✓）； 但给探针加体积读数的**补丁脚本半途断言失败、整脚本回滚** ✗（`open(...,"w")` 在末尾 ⇒ 一处 assert 挂 ⇒ 全不写 ✓） ⇒ **文件其实是干净的**（`git status` 只有本规划 ✓）。⚠ 这是**第 4 次**栽在"多块文本手术"上 ✗ ⇒ **改用最笨最稳的法子**：① `grep -n` 出目标行号；② **按行号整行替换**（不跨块 ✓）；③ 每步单独 `cargo check` ✓。
* ⏭ **下一刀（按优先级，二选一，先量后动 ✓）**：(b) **LSP 3.7ms** —— 三步量法：① 响应体积（≈?B）✓ （若 ~10KB ⇒ 主因在**载荷** ✓）；② 同一请求连发两次的**第一次 vs 第二次**（若第二次明显快 ⇒ 有一次性成本 ✓）； ③ 把 `text` 字段摘掉再量（**只做实验、别动 wire 契约** ✗ —— `docs/protocol.md` 是消费者契约 ✓）。 若确认在载荷 ⇒ 候选 = `soko/goals` 的 `text` 改**按需**（新增可选字段，旧字段留着 ✓ = 加法式 ✓）。

* ✅✅ **LSP 那 3.7ms 的第一刀量出来了（第 90 轮 · 新文件 `crates/lsp/tests/goals_cost_lsp.rs` ✓）**： `PERF goals-lsp: 首次 6.5ms · 后续 median 5.8ms（n=5）· **响应≈91 361B** · 文档≈24 019B` ✓ ⇒ ① **载荷是文档的 3.8 倍** ✗✗（每条声明的 `ty_runs`/`goal_runs`/`value_runs` 把**每个 token** 都做成一条 `RunInfo` ✓）⇒ 序列化 + 客户端解析 **91KB/次** ✓ ⇒ 这就是那 3.7ms 的**主因**（此前我猜"锁 / `decl_info` 映射" ✗， **量出来是载荷** ✓）；② **首次 ≈ 后续**（6.5 vs 5.8ms ✓）⇒ **没有一次性成本** ✓ ⇒ 每次请求都要付 ✗。 ⚠ 对照 Lean：`$/lean/plainGoal` **只回目标文本** ✗（它不做"逐 token 着色数据" ✓）⇒ 这一格的差距是**协议形状** 造成的 ✓，不是编译慢 ✓。
* ⇒ **下一刀是大件（跨面）**：载荷瘦身 = **改 wire 形状** ✗ ⇒ 必须**同一轮**同步 `docs/protocol.md`（消费者契约 ✓） + `editor/vscode/`（真正读 runs 的人 ✓）+ `skills/` ✓，并且**加法式**（新增可选字段、旧字段留着 ✓）。 候选：runs 改**稀疏/紧凑**编码（只发 kind 变更点 + span ✓），或让客户端**按需**索取 runs ✓。

* ⚠ **第 91 轮的"最终全量测试"读数（如实记，**不宣称干净通过** ✗）**：`scripts/soko gate` 自己打出了 `soko: gate PASS` ✓、且日志里 `FAILED/error` 计数 **0** ✓ —— **但进程退出码是 1** ✗。逐步定位到 **`scripts/gap.py check`（缺口台账门禁）** ✓：它**两次跑结论不同** ✗（一次 G-93/94/95 报"行为已变"⇒ exit 1 ✓； 紧接着再跑 ⇒ "**全部与台账一致**" ⇒ 干净 ✓）⇒ 判为**并发写者的在制品/未提交改动翻转了行为类复现** ✓ （复现件跑的是真 CLI/LSP 行为 ✓ ⇒ 会被同树上的在制品影响 ✓），**不是本线的改动** ✓。 ⇒ 结论：本轮的"最终全量测试"= **gate 各步全绿（0 失败）+ 末步台账门禁抖动** ✗ —— 这条要等两条写者都收尾后 再跑一次才能算数 ✓（第 57 轮那次是真 PASS ✓，可作为参照 ✓）。
* 📌 **另一条本轮**没做**的决定（如实记 ✗）**：`display.fold` 记忆化（实测吃掉 ≈0.7ms = 总数的 ~12% ✓）**没有落地** ✗ —— 因为它的键必须含**记法表身份**（`fold` = `print_back(text, self)` ✓，`DisplayNotations{table, arity}` **没有**便宜的指纹 ✗） ⇒ 只按文本做键会**在换表时给错答案** ✗（正确性风险 > 12% 收益 ✗）；安全版要**每文档一级**的缓存（加字段 ✗） ⇒ 收益/侵入比不划算 ✓ ⇒ **记为候选、不动手** ✓（要做得先有"表指纹"或"每文档缓存"的前置件 ✓）。

* ✅ **载荷瘦身先量后动：候选 A 只有 2.6×，**设计档的 10× 估计被实测推翻** ✓✗（第 92 轮）** —— 新档 `crates/front/tests/goals_payload_shape.rs`（真 unit08、项目模式 ✓）实读： **decls≈89 515B（文档 24 019B）· runs≈55 433B ⇒ 稀疏后 21 207B（2.6×）· 响应 113 534 → 79 308B** ✓ ⇒ ① kind 段**没那么连续**（2.6× 不是 5–10× ✗）；② 响应里**还有 ~34KB 不是 runs**（每条的 `ty`/`value`/`goal` 文本 + 名字/span ✓，必要的 ✓）⇒ **A 单独不够**（全做 A+B 也才 ~50KB ⇒ LSP 3.7 → ~2.5ms ⇒ goal ~4.7ms，**仍高于 Lean 3.1ms** ✗）。 ⇒ **结论（写进设计档 §3.1）**：载荷瘦身是**有界卫生**（−30% ✓）而**不是**"goal 反超"的那一刀 ✗ —— 真正对齐 Lean 的是**它的机制**（`plainGoal` 只读**该命令的快照**、**不产出**逐 token 着色数据 ✓）。 ⚠ 这条也是本会话第 4 次"**先量推翻了设计里的估计**" ✓ ⇒ 设计档里的数字**一律要标注"待量"** ✓。

* ⛔ **候选 B（`RunsMode`：goal 查询不产 runs）第 1 次实现：改到编译差 1 处、已**整体还原** ✗（第 93 轮）**： 设计本身没问题 ✓（加法式、缺省即旧行为 ✓），但 `crates/front/src/query/mod.rs` 的 `goals()` 区域 **第 6 次**让文本手术失手 ✗（本轮三连：① 多行 `\n` 当成**一个** list 元素 ⇒ 后续锚点找不到 ✓； ② 枚举插进了 `impl` 里 ✓；③ 批量改写 `runs` 调用点时**越出函数范围**（改了 6 处、应改 4 处 ✓） ⇒ 只剩 `runs_of` 作用域错一处 ✓）。⇒ 已 `git checkout` **整体还原** ✓（`--all-targets` 0 错 ✓）。 ⭐ **给下一棒的硬建议**：这一区**别再走"脚本按行/按块改"** ✗ —— 改法二选一： **(a) 先做一次"搬家"重构**（把 `goals()` 的 4 处 `runs` 调用抽成**同文件里的私有方法** ✗ 一次纯移动 ✓），
  之后再接 `RunsMode` ✓；**(b) 手工在编辑器里改**（人眼盯着作用域 ✓）—— 本会话 6 次失败**全部**来自脚本 ✓，
  而**新增整文件**（`goals_cost.rs` / `goals_cost_lsp.rs` / `goals_payload_shape.rs` ✓）**从未失败** ✓。
  ⇒ **纪律**：这一区只允许"**新增文件**"或"**手工单点**" ✗，不许脚本化多块改写 ✓。

* ✅ **候选 B 的**第一步已落地**（第 94 轮 · 按"只允许新增/单点"的硬纪律 ✓）**：`QueryDoc::goals_without_runs` （`query/mod.rs` 里**一处新增** ✓ —— 走"后处理清空 `*_runs`"而不是给 `goals()` 加开关 ✓， 正是为了避开那 6 次脚本化失败 ✗）⇒ `--all-targets` **0 错** ✓、`query` 单测全绿 ✓。 ⚠ **如实记两件**：① 它只省**载荷**（≈55KB/次 ✓ = 序列化 + 客户端解析 ✓），front 的 tokenize 仍跑 ✗ （那 ≈0.5–1ms 要等 `goals()` 开 `RunsMode`，按纪律得先搬家重构或手工单点 ✗）；
  ② 配套的**等价性判据**写到一半（`goals_runs_optional.rs`）时**预算用尽** ✗ —— 它编译过了，但我自己写的
  "反向前提"断言在无 runs 的声明上判红 ✗ ⇒ 按"**不留红测试**"的纪律**删掉该文件** ✓（真值判据无损失：
  旧路径一个字节没动 ✓）。**下一棒第一步**：把那条判据重写成**全局**前提（不在逐条循环里断言 ✓）再落地 ✓。

* ✅✅ **候选 B 的判据落地，且读数比估计更好（第 95 轮）**：`crates/front/tests/goals_runs_optional.rs` ✓ `PERF goals-runs-optional: 全量 decls≈89 515B（runs 共 1981 条）⇒ 剥掉后≈24 877B · 省 **64 638B（72%）**` ✓ ⇒ 判据 = **等价性**（名字/kind/status/span/ty/value/goal 逐字段相同 ✓ + 三个 `*_runs` 全空 ✓） ＋**反向前提放循环外**（"全量版总 runs > 0" ✓ —— 第 94 轮就是把它放进逐条循环才判红 ✗）；
  `1 passed` ✓。⇒ `soko/goals` 响应里那 ~55–64KB 是**真能去掉的** ✓（比我第 92 轮估的 61% 还多 ✓）。
* ⏭ **LSP 接线（下一轮，4 个单点，已点名 ✓）**：① `protocol.rs:12` 的 `GoalsParams` 加 `#[serde(default = "…")] runs: bool` （缺省 **true** = 今天的行为 ⇒ 老客户端一字不改 ✓）；② 同文件加 `fn default_true()` ✓；③ `lsp/src/lib.rs` 的 `goal_decls(probe)` 加一个"要不要 runs"的入参（**新方法**，别改旧签名 ✓）；④ `goals()`（`:1183` 的 `goal_decls(true)`） 按 `params.runs` 分派 ✓。⚠ `GoalDeclInfo` 是 LSP 自己的类型 ✗（不是 front 的 `DeclInfo` ✓）⇒ 清空要在**调用 front 之后、
  映射之前**做过 ✗ hmm ⇒ 最省事：`goal_decls` 里对 `query().goals(probe)` 换成 `goals_without_runs(probe)` ✓（front 那半已就位 ✓）。
  然后按设计档同步 `docs/protocol.md`（**加法式**：新可选字段 ✓、旧字段一字不动 ✓）+ 一条体积/端到端读数 ✓。

### 33. 第 42 轮（平行线）：方向① 落地后**重量北极星** —— 无回归 ✓（79.1ms vs lean4 218ms）

* **构建身份** `lsp-cargo-mtime=1791498090`（= 当前构建）· 探针自报 **`artifacts=on`** ✓ （即：这一轮读数是在**产物在飞**的状态下量的 ✓ —— 这正是要证的"接线没拖慢热路" ✓）。
* **读数（同机 · 真 stdio LSP · course unit08）**：

  | 臂 | median | 结构 |
  |---|---|---|
  | **真实连续键入** | **79.1ms**（best 78.4 / worst 81.3） | `by=9` · `compile=78ms` · `modules=1` |
  | 开档后第一刀 | **76.6ms** | `compile=76ms` · `by=9` |
  | `typing_equal_length` | **18.5ms** | `modules=0`（入口缓存命中 ✓） |
  | `proof` / `statement` | 18.4 / 18.3ms | `modules=0` |
  | 跨入口切换 | **674.4ms** | `modules=5`（冷 8 ⇒ 复用 3） |
* **结论**：与第 2 轮的 `typing` **79.4ms** 相比 **逐位相同（±1ms）** ⇒ **批 1/2/3 的接线对热按键零回归** ✓； 相对实测的 lean4 诊断口径 218ms ⇒ **2.8×** ✓（`typing_equal_length` 从 39.8ms 降到 18.5ms 是"入口缓存命中"那条路的既有收益，不是本轮改动 ✓，别记到本轮账上 ✗）。
* ⚠ **本轮的诚实边界**：方向② 的 **T2-B（编辑点前不重编）仍未做** —— 平行线已把它的 **量级**量出来（§16：上界 ≈30–40ms/刀）且**前置件很大**（§11.10：文件感知的 span 模型）； 同一时间平行线正在攻**一刀的固定底**（§18–§21：`TcCache` 构造 ⇒ 36.7ms）—— 两条在 **同一段热路**上 ⇒ 本线**不并进去**（避免同一文件两个写者 ✗）。
* **四方向账**：① ✓（批 1/2/3；下载线另立）· ② T1-A ✓（跨条目复用） + 静默期修复 ✓ / **T2-B 未做** · ③ ✓ · ④ T4-A ✓（+ 批 2 的 CLI 侧）。

### 34. 第 43 轮（平行线）：**全量 gate 逮到并修掉我自己引进的一处回归** ✗→✓（教训：`--fast` 不跑 CLI 集成测试）

* **症状**：全量 `scripts/soko gate` 判红 —— `cli/tests/project_recompiles_shared_deps.rs` 的 **G-68 判据**：`marginal=1.80 > 1.5`（多一个入口多 1.80 次 pass ⇒ 共享依赖又被按入口各编 ✗）。
* **根因（我的）**：批 2 把 `run_shared_group` 误接到 **`with_project_session_artifacts`** —— 那是 **reusing** 那条（**带线程局部 LRU**）✗；CLI 该走的是 **`with_project_session_artifacts_trusted`**（**不碰 LRU**、栈上 arena、零泄漏 ✓）。 两处后果：① **违反 T4-A 契约**（CLI 短命进程不该用线程局部检查点 —— 而 T4-A 的守卫只测 `compile_project`，**测不到**这条多入口路 ✗）；② G-68 判据判红 ✓。
* **修法**：改接 `_trusted` 那条 + 传 `&[]` 信任（与老路一致 ✓）⇒ G-68 **`marginal=1.20`** ✓ · T4-A 守卫 ✓ · front **35/35** ✓。
* ⭐ **教训（写给下一棒）**：**`gate --fast` 不跑 CLI 集成测试** ⇒ 跨 crate 的接线（尤其 `front/project/mod.rs` 这种被 CLI 消费的地方）**只有全量 `scripts/soko gate` 才拦得住** ✗ ⇒ 动过那类文件就**别只跑 `--fast`** ✓。

### 35. 第 44 轮（平行线）：全量 gate 第二次逮到**产物与 A5 预热的交互** ✗→✓

* **症状**：`lsp/tests/lsp_artifact_warmup.rs::artifact_hit_open_warms_the_library_checkpoint` 的**反向验证**判红 —— `SOKO_NO_LIB_WARMUP=1` 那一刀实测 **1 个模块**（该回到 2 ✗）。
* **根因（不是 bug，是两条机制重叠）**：**产物**（本线）与 **A5 预热**（平行线）**都能** 省掉库层 ⇒ 关掉预热之后，产物**照样**把库层供上了 ⇒ 反向验证**变成空转** ✗。
* **修法**：该文件量的是 **A5 预热**这条机制 ⇒ 把**产物**那条也一起关 （`SOKONANODA_NO_MODULE_ARTIFACTS=1`，四个 client 全加 ✓）⇒ 两臂回到"预热开 1 / 预热关 2" ✓ （**3/3** ✓）。
* ⭐ **教训（与 §34 同族）**：**凡"关掉某个加速件 ⇒ 必须变慢/变多"的反向验证，都要问 "还有没有别的加速件在替它"** ✓ —— 本仓现在有**三个**（`compiled/` 缓存 · 线程局部检查点 · 磁盘产物）⇒ 反向验证要**点名关掉**它要证的那一个**以及**任何能替代它的 ✓。

### 36. 第 45 轮（平行线）：T4-B **先量后做** —— 差 9×，且**换路不改报告**已证 ✓

* **量（`/tmp/t1b-big`：60 条 `by` 的 `Lib` + 一个入口 · 改一行入口后再编 · 交替三轮）**：

  | 路 | 读数 | 说明 |
  |---|---|---|
  | `build <file>`（今天 = `compile_plan_prechecked`，**整条闭包一趟**） | **0.396 / 0.435 / 0.396s** | 库层**每次重 elaborate** ✗ · **产物数 = 0**（这条路根本不碰 session ✓） |
  | `query check`（session + 产物） | 0.408s → **0.043 / 0.043s** | 第一刀写产物，之后**装载** ⇒ **≈9×** ✓ |
  ⇒ **T4-B 的价值 = 那 9×**，且它就是**库层 elaborate** 那一块 ✓（与方向① 的读数同源 ✓）。
* **本轮落地（两件，都是"敢换"的前置件 ✓）**： 1. 新公开入口 **`project::compile_plan_with_artifacts(plan, options)`** = `compile_plan_incremental` 的 `reuse_library=false` 支（**不碰线程局部检查点** ✓ + 磁盘产物 ✓ ⇒ 与 T4-A 契约一致 ✓）； 2. **前置判据** `crates/front/tests/t4b_plan_parity.rs`：同一条闭包分别走 **整条一趟** 与 **session+产物** ⇒ **`ProjectReport` 逐字节相同** ✓（序列化比较、字段一个不漏 ✓） —— 这是"**换路不改报告**"的红线 ✓（判据过了**不等于**可以无脑换 ✗，见下）。
* ⚠ **采用之前还要解决三件（如实记 · ① 已核实为真 ✓）**：① `compile_plan_incremental` **不收 `progress` sink** ⇒ 直接换会**丢掉 CLI 的进度事件**（用户可见 ✗）—— **第 53 轮核实** `build.rs:703-707`：它**确实**按 `json`/人看两态各传一个 sink ✓ ⇒ 不是假想 ✗；**第 56 轮试过接线、按时间盒撤回** ✗： `&mut dyn ProgressSink` 默认是 `&mut (dyn … + 'static)` ⇒ 要往下传就得处处写 `+ '_` ✓， 而它一路**级联进 `run_pass_with`**（`check/mod.rs`）⇒ 4 个文件、lifetime 错误滚雪球 ✗ ⇒ **这是一件 2–3 轮的重构，不是"加个参数"** ✓（已还原 ⇒ 树绿 ✓）。下一棒要动它请**单独排一轮** ✓；② CLI 还有 **`build.*` 事件流**要整门课对拍 （`--json` 逐字节 ✓）；③ `compile_entries_shared` 的**单入口组被过滤**（"会话是纯开销"） ⇒ 这条口子是给**单文件 `build`/`check`/`course`** 用的 ✓。
* 验证：front **36 个测试目标**全绿 ✓（35 + 新判据）· fmt 干净 · clippy **0** 报错 ✓。
* **四方向账**：① ✓ · ② T1-A ✓（跨条目复用） / **T2-B 未做** · ③ ✓ · ④ T4-A ✓ + **T4-B 前置件已就绪 （差 CLI 侧接线 + 事件对拍）**。

### 37. 第 46 轮（平行线）：产物目录**有界化** —— 补上批 3 漏掉的一件（无界增长 ✗→✓）

* **发现的缺口**：`compiled/` 那条**天然有界**（"一份产物对应一个入口文件" ✓），而 **产物这条没有** ✗ —— 键 = **库层闭包的 Merkle 键** ⇒ 用户每改一次库层就多一个新键 ⇒ 老产物**再也没人读**却一直占盘 ✗（设计 §8.1 只写了"文件名即内容寻址"，**没定上界** ✗）。
* **修法**：`MAX_ARTIFACT_PAIRS = 64` + `prune`（**按 mtime 最旧先走**、**成对**淘汰 ✓、 **刚写的那一对永不淘汰** ✓、best-effort ⇒ 淘汰失败不影响"这次写成功了" ✓）。
* **判据**：`the_artifact_store_stays_bounded_and_keeps_the_newest` —— 连写 `64 + 6` 份（每份隔 2ms 让 mtime 可分辨 ⇒ 顺序确定 ✓）⇒ ① 目录里 `.bin` 对数 ≤ 64 ✓ ② **最新那份读得到** ✓ ③ **最旧那份被淘汰** ✓ ④ **不许留"半对"垃圾**（`.bin` 与 `.meta.json` 要么都在要么都不在 ✓）。 **反向验证（已做）**：`SOKO_T1B_NO_PRUNE=1`（一条量具逃生门 ✓）⇒ 判据**判红** （`实得 70 > 64`）✓ ⇒ 证明它**有牙**、不是空转 ✓。
* 验证：front **36 个目标**全绿 ✓（产物单测 11 条）· fmt 干净 · clippy **0** 报错 ✓。
* **四方向账**：① ✓（批 1/2/3 **+ 本轮的有界化**）· ② T1-A ✓（跨条目复用） / **T2-B 未做** · ③ ✓ · ④ T4-A ✓ + T4-B 前置件已就绪（差 CLI 侧接线 + 事件对拍）。

### 38. 第 47 轮（平行线）：**T2-B0 开工**（方向② 的 S 件）—— 两张派生表的计数器 + 读数 ✓

* **做什么**（照 T2-B0 的"**先建两个计数器**"那一步 ✓）：给 `display_notations`（记法表）与 `top_level_def_spans_over`（定义 span 表）各加一条 **同形于 `closure_prefix_builds_total`** 的读数 ✓ —— 在此之前这两张表**连"几次"都量不到** ✗。 * 计数点选在**唯一实现**里 ⇒ 老路/会话路/单文件**同口径** ✓； * 公开读数：`compile::notation_table_builds_total()` / `def_spans_builds_total()` ✓。
* ⭐ **读数（`crates/front/tests/t2b0_table_rebuilds.rs` · 真课程 unit08 · `QueryDoc`）**：

  | | 记法表 | 定义 span 表 |
  |---|---|---|
  | **冷开** | **19** | **55** |
  | **每一刀**（三刀新文本） | **1 / 1 / 1** | **1 / 1 / 1** |

  ⇒ 两件事：① **每刀各重算一次** ✓（T2-B0 的目标 = **0** ✓）；② **冷开**那一段比"一次"
  多得多（19 / 55）⇒ 除入口趟之外还有**多处**在建（下一棒顺手看清是哪几处 ✓）。
* **判据**：断言写的是"今天的行为"（每刀 ≥1）—— **防漂移**用 ✓；T2-B0 落地后**改判成 0**（**不许放宽** ✗）。 ⚠ 文件里也写死了 §32 的纪律：**冷开前先 `clean_at(module_root)`**（否则产物会把库层供上、 冷开那一段的计数会变 ✗）。
* 验证：front **37 个目标**全绿 ✓ · fmt 干净 · clippy **0** 报错 ✓。
* **下一件（T2-B0 的正身）**：按 A4a 的切法 —— **库层那一段随检查点存**（进 `LibCheckpoint` ✓） + **入口那一段每刀只算入口** ⇒ 把"每刀 1/1"打到 **0** ✓；红线 = 全课程 `--json` 逐字节 + `keystroke_structure` 的 1/1/4 保持绿 ✓。
* **四方向账**：① ✓ · ② T1-A ✓（跨条目复用） / **T2-B0 进行中（计数器 ✓ · 复用未做）** / T2-B 缓做 · ③ ✓ · ④ T4-A ✓ + T4-B 前置件已就绪。

### 39. 第 48 轮（平行线）：**T2-B0 正身落地** —— 两张派生表从 O(闭包) 切到 O(入口) ✓✓

* **做什么**（按 A4a 的切法 ✓）：把"**库层那一段**"随 [`LibCheckpoint`] 存一次 （`lib_display` / `lib_defs` 两个新字段 ✓），每刀只建"**入口那一段**"再合并 ✓。 * **合并的坑**（先证后做 ✓）：两张表**各自都带内建记法**（`display_notations_from_commands` 会前插 `builtin_notation_decls()` ✓）⇒ 天真 `extend` 会**重复内建项** ✗ ⇒ 折叠可能分叉 ✗。 ⇒ 新增 `DisplayNotations::merged_with`（`self.table` 原样 + `other.table` **跳过内建前缀** ✓； 元数表 `self` 打底、`other` 覆盖 ✓），**前置判据** `crates/front/tests/t2b0_display_merge_parity.rs` 证"分段建 + 合并 == 一次性建"**逐位相同** ✓ （两条反向验证：丢掉入口那一段 ⇒ 必须不同 ✓ · 把同一段加两遍 ⇒ 必须不同 ✓）。 * 定义 span 表同法：库层先、入口后（与"库层 ++ 入口"的一次性建表**同序** ⇒ `or_insert` 语义一致 ✓）。 * **产物装载那条路**：这两张表**不进产物**（它们是**文本的纯函数** ✓，存了只撑大产物 ✗）⇒ 装载时按 `lib_units` **现算一次** ✓（不是每刀 ✓）。
* ⭐ **读数（`t2b0_table_rebuilds` · 真课程 unit08 · 三刀新文本）**：

  | 臂 | 每刀**调用次数** | 每刀**处理单元数** |
  |---|---|---|
  | **切分前**（`SOKO_T2B0_NO_SPLIT=1`） | 1 / 1 | **5**（= 整条闭包 ✗） |
  | **切分后**（默认） | 1 / 1 | **1**（= 入口 ✓） |

  ⇒ **调用次数不变**（入口那一段本来就该每刀建 ✓）、**单元数 O(闭包) → O(入口)** ✓✓
  —— 这正是 T2-B0 的目标（"**只随入口规模**" ✓）。
* ⚠ **判据看的是"单元数"而不是"调用次数"** ✗→✓：只数调用次数会把"已经切开了"误读成 "没做" ✗ ⇒ 新增两条读数 `notation_table_units_total` / `def_spans_units_total` ✓。 **反向验证（已做）**：`SOKO_T2B0_NO_SPLIT=1` ⇒ 判据**判红**（实测 `[(5,1),(5,1),(5,1)]` ✓）。
* 验证：front **38 个目标**全绿 ✓ · fmt 干净 · clippy **0** 报错 ✓。
* **四方向账**：① ✓ · ② T1-A ✓（跨条目复用） / **T2-B0 ✓✓** / T2-B 缓做 · ③ ✓ · ④ T4-A ✓ + T4-B 前置件已就绪。

### 40. 第 49 轮（平行线）：T2-B0 之后重量北极星 ＋ **逮到探针一处状态依赖**（读数反而更有价值 ✓）

* **构建身份** `lsp-cargo-mtime=1791504299`（= 当前构建）· 探针自报 `artifacts=on` ✓；**读数**：

  | 臂 | 本轮 | 上一轮（`…098090`） | 判读 |
  |---|---|---|---|
  | **真实连续键入** | **78.7ms**（77.0 / 80.8） | 79.1ms | **持平** ✓ —— T2-B0 的天花板本来就是"个位数 %" ✓（那两张表每刀只值 ~5 个单元的小活 ✓） |
  | `typing_equal_length` / `trailing_comment` | 18.1 / 37.2ms | 18.5 / 38.0ms | 持平 ✓ |
  | 跨入口切换 | **132.8ms**（`modules=1` · 复用 7） | 674.4ms（`modules=5` · 复用 3） | **大幅变好** —— 但**不是** T2-B0 的功劳（它只动那两张表 ✗），见下"状态依赖" ✓ |
  | 开档后第一刀 | **355.4ms**（`modules=5` · `prefix=4`） | 76.6ms（`modules=1`） | **同一构建、同一夹具 ⇒ 差 4.6×** ✗ ⇒ 见下 ✓ |
* ⭐ **逮到探针的一处状态依赖（本轮最有价值的发现）**：`first-keystroke-after-open` 那一臂 量的是"**开档**（报告可能直接命中 `compiled/` 缓存 ⇒ **一趟都不跑**）+ **第一刀**" ✓ —— 于是它取决于**模块根里有没有磁盘产物** ✓： * **有产物** ⇒ 开档命中产物 ⇒ **A5 后台预热**跑一趟 ⇒ 检查点热 ⇒ 第一刀 **1 个模块 / 76.6ms** ✓； * **没有产物**（本轮：上一轮我刚跑过 `t2b0_table_rebuilds`，它按 §32 的纪律 `clean_at(module_root)` **清掉了** ✓，`gate` 的课程那一步也跑过 ✓）⇒ 开档命中 **报告缓存**（不编译 ⇒ 不预热 ✗）⇒ 第一刀**自己把库层编一遍** ⇒ **5 个模块 / 355.4ms** ✗。 ⇒ ① **跨轮比较这一臂前必须先钉住"模块根有没有产物"** ✗（探针缺口 ⇒ 已在 §41 补上 ✓）； ② ⚠ **但 §41 之后这条判读要打折** ✗：那一臂的成本主要在**合成前缀**（`prefix=4` ✓）， 不全是库层 elaborate ⇒ "355 → 76.6ms"**不能**整笔记到方向① 头上 ✓（读数仍真、归因要收窄 ✗）。
* **四方向账**：① ✓（批 1/2/3 + 有界化；**LSP 侧 4.7× 读数** ✓）· ② T1-A ✓（跨条目复用） / **T2-B0 ✓✓** / T2-B 缓做 · ③ ✓ · ④ T4-A ✓ + T4-B 前置件已就绪。

### 41. 第 50–52 轮（平行线）：探针**状态显式化** ＋ 撤回一条错结论 ＋ **建通道拿到真答案** ✓

* **① 状态显式化（成立 ✓）**：`first-keystroke-after-open` 开头清 `<模块根>/.sokonanoda/{artifacts,compiled}` （**两处都要清** ✗→✓：只清产物的话，播种那次开档会**命中报告缓存** ⇒ 一趟都不跑 ⇒ 什么也不写 ✗，实测踩到 ✓）， 默认档再用一个**一次性 client** 开档把**产物播种**出来 ✓（= "用户已经 build 过"的现实档 ✓）⇒ 该臂**可复现** ✓。
* ⛔ **② 撤回（第 51 轮自查）**：我曾据"**加在产物读入口的 trace 一次都没打出来**"下结论 「LSP 第一刀根本没走产物路」—— **不成立** ✗✗。那条 trace 由 **LSP 子进程**打出，而子进程 stderr 被 `Client` 收走、只把 `LSP_TRACE …` 开头的行转成 trace 流 ⇒ 在**测试进程 stdout** 上 grep 它**必然为空** ✗ —— **"没看见" ≠ "没走到"** ✓（错在**读数通道** ✗，不是结论对象 ✓）。
* ⭐ **③ 建通道（第 52 轮正身）**：给**既有** trace 行加字段 `reuse=`（`lru`/`artifact`/`prefix`/`rebuilt`/`none` ✓） —— 由 `session::last_lib_source()` 报出**库层从哪来** ✓ ⇒ 探针只读既有 trace 行就能回答"走没走到产物那条" ✓。
* ✅ **答案（`reuse=lru` · 同一臂 · `lsp-cargo-mtime=1791504968`）**：**第一刀确实走了复用路** ✓ （线程局部检查点命中 ✓ ⇒ **库层没有重编** ✓）。而 `modules=5` **不是**闭包模块 —— `lib.rs:649` 早就写明 "`modules=` 与重编了几个模块**无关**，实测那 5 个**全是合成编译**" ✓ ⇒ 那 **≈330ms 花在 judge/合成那一段** ✓。
* ⭐⭐ **A5c（第 57 轮 · 本轮兑现）**：**有产物就别预热** ✓✓ —— 实测（同一臂、同一构建）： 预热**开** = **333ms** · `modules=5` · `prefix=4` · `reuse=lru` ✗；预热**关** = **129ms** · `modules=1` · `prefix=0` · `reuse=artifact` ✓✓。根因**不是**"预热没用"，而是**用户那一刀 排在它后面**：编译钉在**一条** worker 线程上（`worker_threads(1)`，P2-4）⇒ 预热跑 ~215ms、 用户开档就敲 ⇒ 等着它做完 ✗。而 T1-B 之后产物**已经**把库层供上了 ⇒ 预热是**重复劳动** ✗。 ⇒ 落点：`warm_library_checkpoint` 先查"该闭包的产物在不在盘上"（`artifacts::exists` + `session::lib_artifact_key`）⇒ **在就跳过** ✓ ⇒ **第一刀 333ms → 136.2ms（2.4×）** ✓✓， 结构计数同时回到 `modules=1` / `prefix=0` / `by=9`（= 稳态那一档 ✓）。冷档（没产物）照旧预热 ✓。
* **A5c 之后的全臂重量（第 58 轮 · 构建 `…508589`）**：**开档后第一刀 131.2ms**（`modules=1`·`prefix=0`·`by=9`， 333 ⇒ **2.5×** ✓✓）· **真实连续键入 79.7ms**（78.7 ⇒ **持平** ✓，A5c 没碰热路）· `proof`/`statement`/ `typing_equal_length` 18.1/18.2/18.4ms ✓ · `trailing_comment` 38.4ms ✓ · **跨入口切换 689.7ms**（`modules=5`·复用 3 ⇒ 换闭包的冷路，与 A5c 前逐位持平 ✓）⇒ **A5c 只动了投机预热那一条**，且第一刀与稳态的结构计数**已同档** ✓。
* ⇒ **下一根杠杆在方向③（judge 前缀），不在产物** ✓（产物在 LSP 侧的角色是**冷进程**：开档 ✓）；验证：`perf_keystroke_wallclock` 3/3 ✓ · `lsp_artifact_warmup` 3/3 ✓ · fmt 干净 ✓。
* ⛔ **A5b 试过、已撤（负结论 ✓）**：把 A5 的后台预热从"只热库层"扩成"**库层 + 入口趟**" （想借此把第一刀的合成前缀 memo 提前喂热 ✓）⇒ **实测更坏** ✗：第一刀 **333 → 437ms** （`by=22 → 31` · `tc=14656 → 18839`，`prefix=4` **不变** ✗）。⇒ 那些 memo **不是**这一步能共享的 （线程/键 ✗）⇒ 提前付的那趟**白付还多付** ✗。**已还原**（同一臂回到 **333.2ms** ✓）⇒ 此路**不通** ✓。
