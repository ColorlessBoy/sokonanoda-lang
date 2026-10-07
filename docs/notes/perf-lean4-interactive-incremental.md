# 调研：Lean 4 的交互 / 增量性能机制 —— 对照本仓库编译管线（perf-lean4）

> **只读调研**（2026-10-07）。零代码改动：不碰 `crates/`、`STATUS.md`、`docs/gaps/ledger.jsonl`、
> `docs/notes/HANDOFF-kernel.md`（开发线占用）。本文只写**结论 + 判据 + 借鉴点**，不写过程。
>
> **两侧身份（跨轮比较读数前先看这里）**：
> * **Lean 4 侧** = 本机源码 `~/Documents/lean/lean4` @ `d0493e4c1e`（`v4.12.0-rc1-5351-gd0493e4c1e`）。
>   本文所有 `Lean/...` 路径都相对该树；**行号可核对**。
> * **本仓库侧** = `HEAD 567caf0c`（`0.82.0`）；探针读数一律带这条身份。
>
> 任务书里的 "dsh" 本文按「**本仓库在 DSH 下的那条交互管线**」理解 —— DSH 是消费方
> （`--json` / MCP 查询），性能主体是 `sokonanoda` 的 front/LSP，不是 harness 本身。

---

## 0. 结论（先给答案）

**四句话：**

1. **Lean 4 的"快"是四层缓存的叠加，不是"编译器特别快"**：
   ① **`.olean`**（模块预编译产物，`import` 走 **mmap**）· ② **`Environment` 持久化结构**
   （`addDecl` 是结构共享的插入，不是重建）· ③ **服务器快照树**（每条命令的完整
   `Command.State` —— 含环境 —— 都是可复用的对象）· ④ **语法等价复用**（编辑点**之前**
   的命令一条都不重新 elaborate）。
2. **它确实"依赖缓存"，而且是硬依赖**：`import` **不做任何 elaborate** —— 它 mmap `.olean`，
   把常量表与扩展状态**装进**新环境（`Environment.lean:2317` `importModules`）；而
   `importModules` 在服务器里**只有头语法真的变了才跑**（`Language/Lean.lean:471-483` 的慢路）。
   **没有 `.olean`，Lean 的每次编辑都要从 prelude 重来。**
3. **本仓库缺的不是"缓存"这个念头，是"缓存的分辨率"**：磁盘产物的键是**整条闭包**的摘要
   （`ProjectPlan::digest`，`project/mod.rs:157-191`）⇒ 改一个字节必 miss；进程内**没有**
   任何跨按键存活的**模块级**环境；per-module Merkle 键 `module_keys()`（`project/mod.rs:204`）
   **已实现但生产零调用**（调用点只有 `project/tests.rs`）。
   ⇒ **一次按键 = 整条闭包从零重编**，判据 `edit == cold == 2`（§2.3）。
4. **最该先做的一件事**：Lean 的增量**只活在服务器那条热路上**（CLI 主动丢掉快照元数据，
   `Elab/Frontend.lean:151` 的 `cmdlineSnapshots := true`）；本仓库**只有一条冷路**
   —— LSP 带 `import` 的文档整个绕开 `QueryDoc` 的会话增量。
   ⇒ **先把"库层检查点"交给一个跨调用存活的持有者**（`QueryDoc` 现成，§4.0/§4.1），
   再谈产物序列化（§4.2）。**顺序反了会白做功。**

---

## 1. Lean 4 机制：四层缓存 + 三条边界，逐条给证据

### 1.1 `.olean` —— `import` 是 mmap + 建索引，**不是**重新 elaborate

* `importModules`（`Environment.lean:2317`）→ `importModulesCore`（`:1997`）→
  `readModuleDataParts`（`:1730`，`unsafe opaque`，**一次 mmap 一批文件**）。
  注释 `:1718-1720` 明说：数据**不能用逐个 `readModuleData` 调用加载**，必须整批传文件名，
  `mod` 用来算一个确定性的 mmap 基址。
* 环境自带**读数**（`displayStats`，`:2374-2398`）能直接印出这套模型：
  `number of memory-mapped modules` · `number of imported bytes` ·
  `number of imported consts` / `number of buckets for imported consts` ·
  `number of extensions` / 每个扩展的 `number of imported entries`。
  ⇒ **导入的常量住在按桶组织的表里（`ConstMap := SMap Name ConstantInfo`，`:96`），
  不是"一串源 AST"。**
* **`.olean` 里到底存了什么**（`ModuleData`，`Environment.lean:120-141`）：
  `isModule` · `imports` · `constNames`（**常量名的冗余副本** —— 注释 `:126-131` 原话：
  *"`perf` reports that 12% of the runtime was being spent on `ConstantInfo.name` when importing
  a file containing only `import Lean`"*）· `constants : Array ConstantInfo`（真载荷：名字、宇宙参数、
  类型、值、可归约性提示）· `extraConstNames`（代码生成辅助）·
  `entries : Array (Name × Array EnvExtensionEntry)`（**每个持久扩展一份**：记法/解析表、属性、
  docstring、类型类实例、class 状态、别名、simp simproc、matcher 信息、包 id…）。
  **不含**：源文本、语法树、宏展开结果、位置信息（位置在另一个 `.ilean` 里，`import` 不读它）。
* **三层 + IR**：`.olean`（exported/public）· `.olean.server` · `.olean.private`（全私有数据）· `.ir`。
  默认导入的是 `.exported` 那一层；而且**公开层里定理/定义被弱化成同类型的公理**
  （`AddDecl.lean:108-134`，定理恒弱化 `:121-123`）⇒ 证明体只活在 `.olean.private` 里。
* **信任边界是架构性的，不是一次检查**：`finalizeImport`（`:2188-2296`）把导入的常量
  **直接插进常量表**，**没有任何内核调用**；跨模块唯一的校验是重名一致性
  （`subsumesInfo`，`:2149-2164`）。⚠ **`EnvironmentHeader.trustLevel`（`:162-165`）
  在本版里只被 `displayStats` 读**（`:2384`），`--trust` 的帮助文本说的
  "0 = type check all imported modules" **在本版找不到实现** —— 别把它当行为引用。
* **失效**：**`lean` CLI 自己完全不判 `.olean` 新旧**（`Shell.lean:535-539` 无条件跑 frontend，
  `Elab/Frontend.lean:195-197` 有 `-o` 就写）。C++ 读侧只校验
  marker / 格式版本 / flags /（仅在 `LEAN_CHECK_OLEAN_VERSION` 时）**build githash**
  （`src/library/module.cpp:246-256`）—— 那是**工具链兼容性**，不是源码级失效。
  **失效住在 lake**：每模块一条 `BuildTrace = {hash, mtime}`（`lake/Lake/Build/Trace.lean:311-320`），
  由**源文本内容 + mtime** 播种（`Build/Module.lean:31-36`），
  判定 `depTrace.hash == depHash && 产物存在`（`Build/Common.lean:214-224`，`--old` 模式退化成比 mtime）。
  语言服务器里，**头语法一变就整进程重启 worker 并重跑 `lake setup-file`**
  （`Server/FileWorker.lean:379-398`：`importsLoadedRef` 是进程级 `IO.Ref Bool`，
  第二次进来直接 `IO.Process.forceExit 2`）。⇒ 这是 Lean 侧**最贵**的一刀，
  也正是它要把"头不变"做成快路的原因。

### 1.2 `Environment` —— 持久化结构 + `EnvExtension`，`addDecl` 不重建世界

* **不变量（作者原话）**：*"Environments are never destructively updated."*（`Environment.lean:216`）。
* 内核环境（`Environment.lean:222-271`）里**所有随文件增长的表都是持久化结构**：
  * `constants : ConstMap = SMap Name ConstantInfo`（`:96`）—— **分两级**：
    `map₁ : Std.HashMap`（导入的，**导入期独占** ⇒ 允许破坏性写）+
    `map₂ : PHashMap`（本文件新增的）。`SMap.lean:20-25` 的假设原话：
    *"the number of entries … coming from imported files is much bigger than … the current file ·
    HashMap is faster than PersistentHashMap · when we are reading imported files, we have exclusive
    access to the map"*。
    本地声明的插入走 `map₂` 的 **HAMT**（`PersistentHashMap.lean:40-46`：32 路、`maxDepth := 7`、
    `maxCollisions := 4`；`insertAux` 只重建根到叶那条路径）⇒ **O(1) 摊还、≤7 个节点访问、
    绝不 O(n)**。
  * `const2ModIdx : Std.HashMap`（`:259`）—— **导入期一次建好、之后只读**
    （唯一写者是 `finalizeImport`）⇒ 所有派生环境**按引用共享**。
  * `extensions : Array EnvExtensionState`（`:263`）—— **一个扁平数组，按扩展 id 索引**
    （`modifyStateImpl`，`:1326-1335`）⇒ 扩展 A 写状态**不会碰**扩展 B 的状态。
    ⚠ **诚实的一笔**：数组写是 `Array.set`（copy-on-write）⇒ 当旧环境仍引用该数组时是
    **O(#extensions) 次指针拷贝**，不是 O(1)。`#extensions` 是常量（`src/Lean/` 里注册点约 110 个，
    grep 计数，**未实测运行时值**），但别把它说成 O(1)。
* 展开用的 `Environment`（`:554-616`）把"导入的部分"与"本分支新增的部分"**分开**：
  `base : VisibilityMap Kernel.Environment`（**eager 可用，优先从它取**）、
  `checked : Task Kernel.Environment`（异步 elaborate 完成后兑现）、
  `asyncConstsMap`（`NameMap` + `NameTrie`，持久化）、`isExporting`。
  注释 `:566-569` 原话：**"As `base` is eagerly available, we prefer taking information from it
  instead of `checked` whenever possible."**
* **类型类实例表**（按键延迟的经典热点）也是持久的、**从不重建**：
  `instanceExtension : SimpleScopedEnvExtension InstanceEntry Instances`（`Meta/Instances.lean:89-95`），
  状态 = **判别树**（`DiscrTree`：根是 `PersistentHashMap`，内层是**有序数组**二分，
  `Meta/DiscrTree/Main.lean:12-15`）+ `instanceNames : PHashMap` + `erased : PHashSet`；
  加一条 `@[simp]`/实例 = 一次路径拷贝插入（`Instances.lean:76-79`）。
* **内核重查的范围 = 只有新声明**：`lean_add_decl` → `environment::add` →
  按种类检查**新类型/新值**（`check_no_metavar_no_fvar` + `checker.check` + `is_def_eq`），
  归纳类型另跑正性检查与 recursor 构造。**没有任何"全局不变量遍历"**，不会因为环境大而变慢。
* ⇒ 于是"加一条声明"= 往持久化表里插一项 + 记一笔扩展状态；**旧环境值仍然有效且被共享**。
  这就是服务器能把"每条命令之后的环境"当作**快照对象**存下来的前提。

### 1.3 服务器：快照树 —— 每条命令的**完整状态**都是可复用对象

* `Snapshot`（`Server/Snapshots.lean:28-32`）= `{ stx : Syntax, mpState : Parser.ModuleParserState,
  cmdState : Command.State }`。**`cmdState` 里含 `env`**（`Snapshot.env`，`:38-39`）。
* 处理器由 `Language.mkIncrementalProcessor`（`Language/Basic.lean:401-407`）创建一次；
  它持有 `oldRef : IO.Ref (Option InitialSnapshot)`，**每次 `didChange` 把上一版的
  `InitialSnapshot` 作为 `old?` 交给 `process`**（`Server/FileWorker.lean:475`、`:577-591`）。
* ⇒ 复用的最小单位是**一条命令的整个命令状态**，而 `Environment` 是**指针共享**，不是拷贝。

### 1.4 语法等价复用 —— 编辑点之前**一条命令都不重 elaborate**

这是 Lean 侧最核心的一节，源码里有一整段设计注释
（`Language/Lean.lean:83-114`，`# Note [Incremental Command Elaboration]`）。原话：

> **"Because of Lean's use of persistent data structures, incremental reuse of fully elaborated
> commands is easy because we can simply snapshot the entire state after each command and then
> restart elaboration using the stored state at the next command above the point of change."**

判据是**语法相等**（不是范围相等），外加一个**全局文本编辑位置**：

| 层 | 判据 | 位置 |
|---|---|---|
| 编辑位置 | `isBeforeEditPos pos := firstDiffPos?.any (pos < ·)` —— `firstDiffPos` 由**原始文本**算一次 | `Language/Lean.lean:264-269` |
| 头 | `trimmedStx.eqWithInfo old.stx.unsetTrailing` | `:467` |
| 命令 | `stx.eqWithInfo old.stx` | `:598-602` |
| 语法相等本身 | `Syntax.eqWithInfo`：比较 `SourceInfo` **逐字段**（含 `pos`/`endPos`/`trailing`）+ 节点 kind + 原子/标识符的字面文本与**预解析名** | `Syntax.lean:175-183` |

复用构造子是 `unchanged`（`Language/Lean.lean:558-578`）：`prom.resolve <| { old with nextCmdSnap? := some {…} }`
—— **整条旧快照记录原样接上**，并继续**同步**快进。两级快路：

* **完全免解析**（`:580-585`）：旧快照及其后两条都已 finished，且编辑位置在"再下一条"之后
  ⇒ 连 `Parser.parseCommand` 都不调；
* **只解析不 elaborate**（`:593-604`）：解析后 `stx.eqWithInfo` 成立 ⇒ 复用旧 elaborate 任务。

**第一次语法不等**处（`Language/Lean.lean:605-611`）立刻 `old.nextCmdSnap?.forM (·.cancelRec)` 取消整条旧尾巴，
然后从该命令起到 EOF **重新解析 + 重新 elaborate**（`:746` 起 `old? := none`）。

**Lean 自己承认的边界**（`:373-385`，照抄）：

> **"As there is no cheap way to check whether the `Environment` is unchanged, i.e. *semantic*
> change detection is currently not possible, we must make sure to pass `none` as all follow-up
> 'previous states' from the first *syntactic* change onwards."**

⇒ Lean 的增量**也是保守的**：它只敢在"语法逐字节没变"的地方复用；但它**敢把环境当作复用物**。

### 1.5 CLI 与服务器**故意分成两条路**（这是最重要的一条结构事实）

* **CLI 没有文件内增量，而且是刻意的**：`runFrontend` 设
  `internal.cmdlineSnapshots := true`（`Elab/Frontend.lean:151`），该选项的文档是
  *"reduce information stored in snapshots to the minimum necessary for the cmdline driver"*
  （`CoreM.lean:51-56`）—— 处理器因此丢掉除环境外的所有逐命令元数据
  （`Language/Lean.lean:681-684`）并不再在快照里存语法（`:643-646`）。
  ⇒ `lean Foo.lean` **每次调用都把整份文件从零重解析 + 重 elaborate**
  （每条命令都重做宏查找 `Elab/Command.lean:469` 与命令 elaborator 查找 `:529`）。
* **增量只活在服务器那条路**（§1.3/§1.4）：它保留 `Command.State` 快照树并做语法等价复用。
* ⇒ **Lean 的"快反馈"= 一个长寿命的增量处理器**；CLI 是冷路、服务器是热路，
  **两条路共用同一套前端但保留的信息量不同**。
* **本仓库的对应关系**：`sokonanoda` CLI 与 LSP **共用** `compile_project` 这条冷路
  —— LSP 的 `QueryDoc::project_compile_incremental` 里，带 `import` 的文档**整个绕开**
  `QueryDoc` 的会话增量（`query/mod.rs:471-489`：有项目报告就只补 `parse_error`，
  **不跑**单文件流水线）。⇒ **本仓库的"服务器路"今天等于"CLI 路 + 一点内核信任前缀"**，
  这正是 §3 那张对照表的第 3、5 行。

### 1.6 诚实的边界：Lean 的 `import` **也不免费**

* **每次进程调用都重建索引**：`finalizeImport`（`Environment.lean:2188-2296`）对
  **每一个导入常量**做一次哈希插入（private + public 两遍，`:2211-2238`），
  再 `const2ModIdx`（`:2223-2226`），再 `setImportedEntries`（走遍"扩展 × 模块 × 条目"，`:1843-1861`），
  再对**每个**持久扩展调一次 `addImportedFn`（`:1896`）重建它的派生索引。
  ⇒ 代价 = **Θ(Σ 导入常量) + Θ(Σ 扩展条目)，按进程调用付，任何地方都不摊还**。
  mmap 让常量的**载荷**免费（指针指向映射区，不拷贝、不重定位），
  但**名字索引确实每次重建**。这就是"`import Mathlib.X` 哪怕只写一行也要几秒"的原因。
* **`addImportedFn` 的官方逃生门**（`Environment.lean:1552-1572` 原话）：
  *"The `addImportedFn` runs at the beginning of elaboration for every module, so it's usually
  better for performance to query the array of imported modules directly, because only a fraction
  of imported entries is usually queried during elaboration of a module."*
  ⇒ 规范做法是 `addImportedFn := fun _ => {}` + 查询期二分（`EnvExtension.lean:96,139,163`）。
* **mmap 快路会静默退化**：只有在记录的 `base_addr` 上映射成功才走 mmap；
  任何一处映射失败 ⇒ 退化成一次 `malloc` + 读全部（`module.cpp:320-344`）
  —— 那是"映射导入"与"几百 MB memcpy"的差别。
* **⇒ 对本仓库的意义**：**别把"抄一个 `.olean`"当成"免费"**。Lean 的收益来自
  (a) **不重新 elaborate**，(b) **在同一个服务器会话里只付一次**。
  本仓库要拿的是这两条，不是"序列化本身很快"。

### 1.7 调度与取消（影响"手感"，不影响总量）

* **eager 到 EOF，不按光标优先级**：`Language/Basic.lean:97-100` 明写这是 TODO；
  离开快进路后**只开一个任务**跑完剩下的全部命令（`Language/Lean.lean:636-637` + `:746` 的 `sync := false`）。
* 请求**等**快照，不驱动计算（`Server/Requests.lean:340-345` 的 `withWaitFindSnap`）。
* **取消是"只取消被排除复用的那部分"**（`Language/Lean.lean:182-229` 的 `# Note [Incremental Cancellation]`）——
  因为并行下取消整条旧调用会让快照引用到被取消的异步常量。
* 唯一的"延迟"是**报告**：`server.reportDelayMs` 默认 200ms（`Server/FileWorker.lean:193-198`）。
* 复用的快照连**交互式诊断对象**一起复用（`Server/FileWorker.lean:309-315` 读
  `interactiveDiagsRef?` 的记忆值），避免重渲染。

---

## 2. 本仓库现状（as-built，带判据）

### 2.1 管线全貌（一次按键走的路）

```text
LSP didChange
  └─ Doc::set_text                      crates/lsp/src/lib.rs:~190
       ├─ ① 文本/mode/path/overlay 全同 且 闭包摘要同 ⇒ 直接返回（save / 外部改动）
       ├─ ② project_cache::load_at(root, digest)   ← 键 = ProjectPlan::digest（整条闭包）
       │      └─ 命中 ⇒ 回放报告，不重编          ← 只服务"开档"，改一个字节必 miss
       └─ ③ QueryDoc::set_text_with_overlay        crates/front/src/query/mod.rs:441
            └─ project_compile_incremental          query/mod.rs:583
                 ├─ plan_project_with_overlay       ← 每次按键**重读 + 重解析整条闭包**
                 └─ compile_plan_incremental        project/mod.rs:485
                      └─ with_project_session_trusted   project/session.rs:62
                           ├─ 库层趟 run_pass_with(…, lib_units)   ← **整份重新 elaborate**
                           └─ 入口趟 run_pass_with(…, entry_units, trust)
                                └─ trust 只跳**内核重查**，环境仍整份 elaborate
```

判据（读码，非推断）：
* `compile_plan_prechecked`（`project/mod.rs:906`）无条件调
  `compile_all_units_with_progress(&units, …)` —— **全闭包，每次都编**；
* `run_pass`（`compile/check/mod.rs:973`）**每次新建 arena + 新建 `EnvBuilder`**（`:981`/`:1009`）
  —— 没有"从现成环境续走"的入口；
* 会话路的库层趟（`session.rs:82`）**没有任何 trust 参数**（那一趟传的是 `None, None`）；
* 入口趟的 trust（`walk.rs:473-475`）语义是
  **"Trusted prefix: keep the environment, skip the kernel."**（`walk.rs:862` 原注释）
  ⇒ **环境照 elaborate，只跳内核检查**。

### 2.2 三层缓存，三把不同的键（互不替代）

| 层 | 键 | 寿命 | 服务谁 | 改一行会怎样 |
|---|---|---|---|---|
| 全局缓存 + 模块根 `.sokonanoda/compiled/` | `ProjectPlan::digest`（**整条闭包**：每模块名字/源文本/import 边 + 入口路径 + prelude 模式） | 跨进程 | **开档** | **必 miss**（源文本进摘要） |
| `QueryDoc::entry_cache` | 入口命令布局（`keys`/`starts`）+ 依赖指纹 | 进程内 | 入口趟的**信任前缀** | 命中，但只跳内核检查 |
| `module_keys()`（per-module Merkle） | `H(格式+版本+prelude+源文本(M)+[name(D),key(D)]…)` | —— | **无人**（生产零调用） | 不参与 |

第三层的注释自己写明了差别（`project/mod.rs:193-200`）：`digest` 把**整条闭包**折成一条键
⇒ "42 个入口各存一份、共享依赖各编一遍（**G-68** ✗）"。**这一层就是 Lean 的 `.olean` 的位置，
而它是空的。**

### 2.3 判据：一次按键 = 整条闭包（**已实测，且有常驻守卫**）

`crates/front/tests/g29_closure_recompile.rs:87` 的 `g29_edit_recompiles_the_whole_closure`
**今天通过**，它断言的是**当前行为**（防漂移，不是"待修红"）：

```rust
assert_eq!(edit, cold,
  "**G-29 仍在**：改一行重编了 {edit} 个闭包模块 = 冷开（{cold}）⇒ 依赖的源文本一个\
   字节没变也照编 ✗。修好（库层检查点**跨调用**复用，设计 §33）之后这里应当是 **1**…");
```

本轮复跑（`cargo test -p sokonanoda-front --test g29_closure_recompile`）：**exit 0**，
即 `cold == edit == 2`（夹具 = `Lib` + `Main`）。
同轮 `keystroke_structure` 的读数 `closure_modules = 2` 也指向同一件事：
**一次按键编的模块数 = 闭包全部模块数**（含从头到尾没变的依赖）。

### 2.4 成本分解：库层 15% / 入口趟 82%，其中 **7 次合成前缀 = 58%**

`docs/design/incremental-environment.md:1128-1137`（unit08 · debug LSP · 真进程 · 同二进制）：

| 段 | ms | 占比 | 归属 |
|---|---|---|---|
| 库层趟（4 模块 · `run_pass_with`） | **677** | **15%** | 库层跨调用复用可省（§33） |
| 入口趟（1 模块） | 3672 | 82% | 其中 **7 次 `compile_fol_with` 合成前缀编译 = 2609ms（58%）** ⇒ **G-31/G-92** |

**两条必须一起读**：
1. 只做"闭包/库层复用"**天花板 15%** —— 别拿它当"修好了"；
2. 那 7 次不是"闭包模块编译"，是 `judge` 的**合成 `#check` 文档**编译
   （`compile_fol_with ← judge_infer_uncached`，backtrace 见 `incremental-environment.md:1141`）。
   ⇒ 「改一行 `modules=7` = 整条闭包重编」是**误归因**（已在 `check/mod.rs:1255-1273`
   加 `CLOSURE_MODULE_COMPILES` 分开数）；但**"闭包确实重编了"这个结论不变**，
   它现在的判据是 `closure_module_compiles_total()`。

### 2.5 已有的"准增量"（不是没有，是粒度不对）

* **I8 TrustPlan / S2 信任前缀 / S6 依赖脏集**：入口趟能按"改动点之前的连续前缀 +
  与改动点无依赖关系的后缀"跳过**内核重查**（`query/mod.rs:603-635`）。
  读数（`keystroke_structure`）：改无人依赖的一条 ⇒ `recomputed_commands = 1`、
  `entry_kernel_checks = 5`；改被 3 条依赖的一条 ⇒ `4` / `8`。**脏集模型是工作的**。
* **G-31/G-92 第二刀（`0.83.0`）**：把合成文档里"确实加过"的前缀 `theorem` 装成
  **不透明常量** ⇒ 前缀**证明体**不再 elaborate ⇒ `by_calls` 线性化（`10→20 = 2.00`，
  改前 `55→210 = 3.82`）。**已落地**，但它**不省前缀的类型/定义**，也不省环境重放。
* ⇒ 本仓库的增量**全部集中在"少查内核"**；Lean 的增量集中在**"少 elaborate + 少装环境"**。
  **这是两件不同的事，而按键延迟的大头在后者。**

---

## 3. 差距的根源（逐条对照）

| # | 维度 | Lean 4 | 本仓库 | 后果 |
|---|---|---|---|---|
| 1 | **模块边界是什么** | `.olean` = **已 elaborate 的常量表 + 扩展状态**，mmap 加载 | `import` = **把依赖的源文本拼进前缀**，每次重新 elaborate | 依赖的每个字节都在按键的临界路径上 |
| 2 | **产物键的粒度** | 每模块一个文件（构建系统管失效） | **整条闭包一个摘要** ⇒ 改一个字节全 miss | 磁盘缓存只服务开档，**不服务按键** |
| 3 | **进程内是否留环境** | 快照树留**每条命令**的 `Command.State`（含 env） | 检查点只在一次 `with_project_session` 的**栈上**，函数返回即丢 | 每次按键从 prelude 重装 |
| 4 | **复用的判据** | 语法 `eqWithInfo`（逐字段，含 `SourceInfo`） | 入口：命令**布局**（`keys`/`starts`）+ 依赖指纹；库层：无 | 库层**没有任何复用判据** |
| 5 | **复用跳掉什么** | **整条命令的 elaborate + 环境推进** | 只跳**内核重查**（"keep the environment, skip the kernel"） | 环境推进（最贵的那段）一次没省 |
| 6 | **单文件内部的增量** | 编辑点前不重编；命令内还能用 `withNarrowedTacticReuse` 抢救子部分 | 脏集跳内核检查；**证明体/类型仍整份 elaborate** | 大文件的"改最后一条"仍然贵 |
| 7 | **判定前缀重跑** | 无此物（查询直接在活环境上做） | `judge_infer` 合成 `#check` 文档 ⇒ **7 次整份前缀** | 58% 的入口趟成本 |
| 8 | **CLI 与服务器是否分路** | **故意分**：CLI 设 `cmdlineSnapshots := true` 主动丢掉快照元数据；增量只在服务器 | **不分**：LSP 带 `import` 的文档整个绕开 `QueryDoc` 会话增量，走的就是 CLI 那条冷路 | 服务器没有"热路"可言（§1.5） |

**一句话的根因**：

> **Lean 的复用单位是"一条命令的完整状态（含环境）"，本仓库的复用单位是"一份整闭包的报告"。**
> 中间那一层 —— **模块级环境** —— 在 Lean 是 `.olean` + 快照树，在本仓库**是空的**
> （`module_keys()` 有键无人用；`.olean` 的对应物 `docs/design/module-artifacts.md` 的**刀 2**
> 明确写着"先不做"）。

---

## 4. 可落地的借鉴点（按 ROI 排序）

> 下面每一条都标了**判据**（结构计数 / 比值，不用绝对毫秒 —— `AGENTS.md` 判据纪律②）。

### 4.0 先认一件事：**"热路"必须是一个长寿命的持有者**（Lean 的 `cmdlineSnapshots` 对照）

Lean 把 `lean` CLI 与服务器**故意分成两条路**（§1.5）：CLI 主动丢掉快照元数据、
每次调用从零重编整份文件；**增量只活在服务器那个长寿命处理器里**。
本仓库今天**只有一条路**：LSP 带 `import` 的文档整个绕开 `QueryDoc` 的会话增量
（`query/mod.rs:471-489`）⇒ 服务器跑的就是 CLI 那条冷路。

⇒ **任何"把库层/模块产物留下来"的做法，都必须先有一个跨调用存活的持有者。**
本仓库已经有现成的：`QueryDoc`（它已经持有 `entry_cache`）。
**这不是新架构，是把已有的会话对象用在项目路上。**
判据：`QueryDoc` 上加一个"库层检查点"字段之后，
`g29_closure_recompile` 的 `edit` 必须能从 **2** 降到 **1** —— 降不下来就说明持有者没生效。

### 4.1 刀 1（**最高 ROI，设计已写完，只差落地**）：库层检查点跨调用复用

* **借鉴的 Lean 机制**：`unchanged` 复用 `cmdState`（含 env）—— 即"上一轮的环境值仍然有效"。
* **本仓库的形状**：把 `with_project_session_trusted` 的库层检查点
  （`EnvBuilder` + `PassTables` + `lib_out`/`lib_reports`/`lib_ranges`/`lib_n`）留在
  **跨调用的持有者**里 —— `QueryDoc` 是天然持有者（它已经有 `entry_cache`）。
* **已知卡点（不是新发现，是设计 §33 已记的）**：`EnvBuilder<'a>` 借 `&'a ArenaRef<'a>`，
  而 arena 建在函数栈上。两条形状：**(a) `Box::leak`**（零 `unsafe`，代价是泄漏一份 arena，
  摘要一变就得换，必须有上界）；**(b) 自引用 + 生命周期转换**（不泄漏，要 `unsafe`）。
* **判据**：`crates/front/tests/g29_closure_recompile.rs` 的 `edit` 从 **2 → 1**
  （该文件现在断言 `edit == cold`，改成 1 就是"修好"的时刻 —— **改断言值，不是放宽它**）。
* **天花板提醒**：**只省 15%**（§2.4）⇒ **单独不构成"修好了"**，必须与 4.4 一起谈。

### 4.2 刀 2：**模块级产物 = 本仓库的 `.olean`**（对齐 `docs/design/module-artifacts.md`）

* **借鉴的 Lean 机制**：`.olean` 存 **A. 内核环境注册状态 + B. 前端表 + C. 报告/事件**，
  `import` 直接装载。本仓库的 `docs/design/module-artifacts.md` §2 已经把这三块列全 ——
  **差别只在"还没做"**。
* **最小切片（先内存、后磁盘）**：刀 1 是"同一 arena 里的环境分叉"；**刀 2 才是磁盘序列化**。
  Lean 的顺序也是先有内存快照树、`.olean` 由构建系统落盘。
* **失效规则直接照抄 Lean 的边界**：`importModules` 只在**头语法变了**才重跑
  ⇒ 本仓库对应"**依赖的源文本变了才重装它的产物**"，其余一律复用。
* **判据**：`module_keys()` 的键命中计数（新增一个结构计数，不判墙钟）+
  **反例**：改依赖一行 ⇒ 必须 miss 且重编（`docs/design/module-artifacts.md` §5 第 1 条已在册）。

### 4.3 把 `module_keys()` 接上（**零新算法**，纯接线）

* `ProjectPlan::module_keys`（`project/mod.rs:204`）**已经**算好了 per-module Merkle 键
  （`key(M) = H(格式+版本+build stamp+prelude+源文本(M)+[name(D),key(D)]…)`），
  与 `digest` **同一个哈希函数、同一条 Merkle 链**。
* 今天**生产零调用**（唯一调用点是 `project/tests.rs:529,611`）；
  `crates/front/tests/session_reuse.rs:153` 那条 `#[ignore]` 的 TDD 守卫写着
  「切片 1（按 `module_key` 复用）尚未实现；实现后删掉本行」。
* ⇒ **这是本仓库里"最便宜的一刀"**：键已经在手，缺的是"用键去查产物"的那一步。
  Lean 侧对应的正是"`.olean` 文件名 = 模块名，内容由构建系统按依赖失效"。

### 4.4 入口趟的 7 次合成前缀编译（**G-31/G-92 终局**，58% 的那一块）

* **Lean 侧没有对应物**：Lean 的 `#check` 直接在**活环境**上跑（`snap.runCommandElabM`，
  `Server/Snapshots.lean:51-56`），**不需要合成一份源文本文档再编译**。
* **本仓库的现状**：`judge_infer` 合成 `前缀 + #check` 文本 ⇒ `compile_fol_with` 整份重编
  （已部分修：`run_synthesized_incremental` + `trusted_entered` 不透明前缀，
  `judge.rs:1091-1135`）。**剩下的**是"前缀的类型/定义仍会重编"与"合成编译拿不到活环境"。
* **借鉴点（机制级，不是照抄）**：Lean 的做法是**查询复用调用方的 `Command.State`**
  （`Snapshot.runCommandElabM` / `runCoreM` / `runTermElabM` 三件套）。
  ⇒ 本仓库对应的是把**主编译 pass 的活环境**交给 judge 的查询，而不是让 judge 自己造一份文档。
  设计里这条路叫**出路 ②/K-2**（`docs/design/incremental-environment.md:1040-1048`），
  **已有部分落地**（`EnvView` / `judge_infer_with`），**`judge_pairs` 那一刀仍未做**。
* **判据**：`docs/gaps/repro/G92-…sh` 的 `by_calls` 比值（结构计数，机器无关）。

### 4.5 明确**不该照抄**的两条（写清楚为什么偏离）

1. **"语法逐字节相等"当复用判据**：Lean 敢这么做，是因为它**不需要**为"文本没变"付任何
   重新解析的钱 —— 旧 `Syntax` 树就在快照里。本仓库今天**连入口命令布局都要重新算**
   （`entry_command_layout`），而且 `importless_source` 会**重建字符串**（`walk.rs:498-502`
   实测踩过：剥过之后切片文本与原文不同 ⇒ 身份不同）。⇒ 直接抄会先付出"重新解析"的成本，
   **收益被吃掉**。**正确顺序是先有 4.1/4.2 的环境复用，再谈语法级复用。**
2. **"编辑点之后全部重编"**：Lean 的后缀重编之所以可接受，是因为**后缀就是真依赖后缀**
   （命令按顺序推进环境，后面的命令语义上确实依赖前面的环境）。本仓库**已经有更好的模型**
   —— S6 的**依赖脏集**（改无人依赖的一条 ⇒ 重查 1 条，不是后缀）。⇒ **这一条本仓库领先，
   不要回退**（`keystroke_structure.rs:307-322` 是它的常驻守卫）。

---

## 5. 未取证 / 不确定（不许当结论用）

1. **本轮没有测墙钟**。所有"贵/便宜"都是**结构计数**（模块编译次数、passes、`by_calls`）
   或**引用的历史实测**（`incremental-environment.md:1128-1137`，那是 `af533005` 上的读数，
   **不是本轮构建**）。跨构建的绝对毫秒**不可转移**（`AGENTS.md` 判据纪律②）。
2. **Lean 侧的渐近界是从数据结构代码读出来的，没有 profile**：`PHashMap` 的 HAMT 常量
   （`PersistentHashMap.lean:40-46`）与路径拷贝（`:114-143`）是**读码**；
   `importModules` 那个 12% 是**作者自己的实测，本文只是转引**。
   唯一**没量到**的是 `#extensions` 的**运行时**条数（§1.2 已按 grep 计数约 110 给了上界代理）。
3. **`--trust` 的帮助文本在本版是过期的**（`Shell.lean:159-160` 说 "0 = type check all imported
   modules"，但本版找不到实现；`trustLevel` 只被 `displayStats` 读）。**别引用那段帮助文本当行为。**
4. **Lean 的 `.olean` 失效判据已取证**（§1.1）：lake 用 `BuildTrace {hash, mtime}`，
   **hash 优先**，`--old` 模式退化成 mtime。本文没有实测 remote cache（`Lake/Config/Cache.lean`）。
5. **本仓库 `docs-lint` ⑦ 在 HEAD 上已经是红的**（L2 层 `9069 > 9068`，超出 1 行）。
   归因：最近几笔只动 `docs/notes/HANDOFF-kernel.md` 的提交没有同步 L2 上限
   （`git log --oneline -8 --name-only -- docs/design docs/notes` 可复核）。
   **这不是本文造成的，本文也没动那个文件**（它在禁改清单里）—— 登记本文时按
   `docs-budget.json` `_comment` 的**例外①（新增文件到该层 ⇒ 基线跟着走一次）**手改上限，
   并在 `_comment` 里**写明这 1 行的归属**，避免被当成"抬上限达标"。

---

## 6. 附录：复现命令与读数（全部只读）

```bash
# ① 一次按键 = 整条闭包（判据文件自己断言当前行为，exit 0 = 缺口仍在）
cargo test -p sokonanoda-front --test g29_closure_recompile -- --nocapture
#    ⇒ test result: ok（edit == cold == 2）

# ② 一次按键的结构读数（closure_modules / entry_kernel_checks / by）
cargo test -p sokonanoda-front --test keystroke_structure -- --nocapture
#    ⇒ PERFJSON {"case":"keystroke_structure","leaf":{…"closure_modules":2…},…}

# ③ 合成工程「改一行 ⇒ 重编闭包」的 passes（机器无关）
scripts/dev-verify.sh
#    ⇒ 冷跑 passes=13 files=10 ⇒ 改一行 passes=3 files=10

# ④ Lean 侧机制的原话（设计注释）
sed -n '83,114p'   ~/Documents/lean/lean4/src/Lean/Language/Lean.lean
sed -n '365,430p'  ~/Documents/lean/lean4/src/Lean/Language/Lean.lean   # parseHeader 三级快路
sed -n '553,625p'  ~/Documents/lean/lean4/src/Lean/Language/Lean.lean   # parseCmd 复用判据
sed -n '1718,1740p' ~/Documents/lean/lean4/src/Lean/Environment.lean     # mmap 加载
sed -n '2374,2398p' ~/Documents/lean/lean4/src/Lean/Environment.lean     # displayStats 的导入模型
```

**读法**：①②③ 是**结构计数**（机器无关，可跨机比较）；④ 是**源码引用**（可核对行号）。
**禁止**把 ① 的 `2` 与 Lean 的"1 条命令"直接比 —— 两者量的不是同一个单位
（本仓库数"模块编译次数"，Lean 的快照复用数"命令"）。

**构建身份**（`AGENTS.md` 的探针纪律：跨轮比较读数前先确认同一份构建）：
①②③ 的读数全部取自 **`HEAD 567caf0c`（`0.82.0`）· 工作树干净**，同一次会话的 22:52–22:55
（`/tmp/soko-devverify.log` 22:52:27 · `/tmp/keystroke-structure2.log` 22:53:56 ·
`/tmp/g29.log` 22:55:12）。⚠ 之后开发线在 `crates/front/src/compile/check/{mod,walk}.rs`、
`project/session.rs`、`query/mod.rs` 上有**未提交改动**（mtime 22:56 起）
⇒ **本文读数与那些改动无关，也不可与之并排比较**。
