# 规划附录：**执行进度与读数（as-built）** —— 从 `PLAN-align-lean4.md` §11 归档

> **为什么单独成文**（2026-10-09）：`PLAN-align-lean4.md` 顶到了 `docs-lint` 判据②的
> **单文件上限 2000 行** ✗ ⇒ 按仓规把**历史 as-built 段**（§11 全系列）搬到这里 ✓，
> 正文只留**指针** ✓。**内容一字未改** ✓（只搬不改 ✓）。
>
> **追加纪律不变**：只追加**读数与落点**（计划段不动）；每条自带**构建身份**（探针纪律 ✓）；
> 绝对毫秒只作**同机前后**比较，判据一律结构计数 ✓。

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
### 11.13 第 7 轮（2026-10-09）—— 跨入口切换读数（**恢复被误删的探针** + 新鲜读数）

* **先修一个自伤**：第 3 轮删"入口预热臂"时，那段 python 按**文档注释**定位、把
  **跨入口切换**那条用例**一起吞掉**了 ✗（`perf_keystroke_wallclock.rs` 只剩 2 条）。
  本轮从 `554e5b16^` 取回原文并复原 ✓（`fn perf_course_cross_entry_switch_is_recorded`）。
  ⚠ 教训：按**注释文本**切片会误删邻接用例 —— 删东西前后要 `grep -c "^fn perf_course"` 数一数。
* **读数**（同机 · 真 stdio LSP · `lsp-cargo-mtime=1791479842` · unit08 → unit09）：

  | 相位 | 第 3 轮 | **现在** |
  |---|---|---|
  | 墙钟 | 1030–1167ms | **711.0ms** |
  | `compile` | — | 710ms |
  | `modules`（这一次真编了几个） | 5 | **5**（冷开同一入口 = **8** ⇒ **复用 3**） |
  | `by` | 127 | **21** |

  ⇒ 方向③ 的收益在这条路上也兑现了（`by` 127 → **21**、墙钟 −30%）✓；
  **但"换单元"的贵仍不在入口**：`modules=5` 是**另一条闭包**里 unit08 检查点**没有**的
  模块（unit09 引入 `lib.Equiv` 那一支）⇒ 它们**必须**真编 ✗。
  ⇒ 这一格要再降，只有**磁盘产物**（方向①的磁盘半件 **T1-B**：新模块对着 mmap 的环境编、
  不 elaborate 依赖）**或**入口命令级快照（② T2-B，但换入口时它不适用）——**不是**调度问题。
* **四方向账（本轮后）**：① T1-A ✓ / **T1-B（磁盘半件）未做 = 当前最大单项** · ② T2-A ✓ /
  T2-B 未做（前置件 §11.10）· ③ T3-B1 ①②③ + T3-B2 ✓ · ④ T4-A ✓。
* **北极星账面**（§11.12 · 同构建）：真实连续键入 **81.7ms**、开档后立刻敲 **201.0ms**
  （compile 78ms ⇒ 差额是首刀的服务端排队/排空）、跨入口切换 **711ms**。
### 11.14 第 8 轮（2026-10-09）—— **全量验证抓到一条用户可见回归**：就地签名文本 ≠ 慢路文本

* **跑法**（`AGENTS.md` 的"最终全量测试"）：`scripts/dev-verify.sh` +
  `cargo test -p sokonanoda-front -p sokonanoda-lsp -p sokonanoda-cli`。
* **结果**：`dev-verify` ✓（冷 `passes=13` / 改一行 `passes=3`，与历史逐字相同 ⇒ CLI 路没动 ✓）；
  front 各目标**全绿** ✓；**`--lib` 1 条红** ✗：
  `tests::hover::hover_on_a_locally_declared_notation_symbol_explains_it`
  （期望 hover 里有原始类型 `myop : (a : Prop) -> (b : Prop) -> Prop`，
  实际只有折叠形态 `` `a ⊗ b : Prop` ``）。
* **A/B 归因（决定性）**：

  | 开关 | 该条 |
  |---|---|
  | 默认（`SOKO_JUDGE_INPLACE` 未设 = **On**） | **FAILED** ✗ |
  | `SOKO_JUDGE_INPLACE=off`（回到合成趟） | **ok** ✓ |

  ⇒ 根因 = **就地路返回的签名文本与慢路不同**：就地读 `known.signature()`（**过了显示记法折叠**
  ⇒ `a ⊗ b : Prop`），而慢路走 `#check` 出口拿的是**未折叠**的
  `(a : Prop) -> (b : Prop) -> Prop` ✗。
* **这意味着什么（要紧）**：§11.12 那笔 **334ms → 81.7ms** 的收益**当前带着这条显示回归** ✗
  —— 而 `judge_type_of_constant_inplace` 的文档注释自己写着"**文本必须与慢路逐字节相同**"
  并以**影子档**（`type_of_constant_shadow()` 的 `diff`）为前提 ⇒ 本条的教训是
  **影子档的 `diff == 0` 必须先量出来、并且真的当门用** ✗（现在它是"只报不拦"）。
* **下一步（写死）**：① 量 `type_of_constant_shadow()` 的 `same/diff`（本用例应能咬住）；
  ② 让就地路返回**与慢路同文本**的签名（要么用同一套 pp 参数、要么就地失败时回落慢路）；
  ③ 判据 = 原 hover 用例转绿 **且** `typing` 臂的 `by` 仍为 9 / 墙钟仍 ~80ms（**不许用
  "把显示改回去"当解法** ✗——那是拿回归换性能）。
### 11.15 第 9 轮（2026-10-09）—— §11.14 的下一步①**被读数否掉**：影子档是干净的（不是这条路的锅）

* **读数**（`SOKO_JUDGE_INPLACE=shadow cargo test -p sokonanoda-front --test
  judge_inplace_type_of_constant -- --nocapture`）：

  | 档 | same | diff | 说明 |
  |---|---|---|---|
  | `type-of-constant`（T3-B1 第 5 条 = §11.14 怀疑的那条） | **78246** | **0** | 就地文本与慢路**逐字节相同** ✓ |
  | `JUDGE_INPLACE_BY`（by 就地） | 1642 | 0 | 同上 ✓ |

  ⇒ **§11.14 的"① 量影子档、它应当咬住"不成立** ✗ —— 就地**常量签名**那条路的文本是好的。
* **改判**：失败的 hover 用例走的是**别的**就地路径（记法符号 hover 的**类型推断**那一支，
  不是 `judge_type_of_constant`）—— 而那条路**没有影子对照**（现有两个影子档都不数它 ✗）
  ⇒ 所以 `SOKO_JUDGE_INPLACE=off` 一转就好，但影子档看不见 ✗。
* **下一步（写死 · 已收窄）**：① 先给那条路**补一个影子读数**（照 `type_of_constant_shadow`
  的形状：就地结果 vs 合成结果**逐字节**），让它**先能报红**；② 再修文本分叉；
  ③ 判据 = 原 hover 用例转绿 **且** `typing` 臂 `by=9` / 墙钟 ~80ms 不动 ✓。
  ⚠ 不许跳过①直接改（否则改完没有任何东西能证明"分叉真的消失了" ✗）。
### 11.16 第 10 轮（2026-10-09）—— 那条 hover 回归**不是"文本被折叠"**（定位到具体那一行）

* **读源码定位**（`crates/lsp/src/lib.rs::notation_symbol_hover`）：这个 hover 由**两行**拼成，
  而失败信息里**整条**是：

  | 行 | 来源 | On 档下 |
  |---|---|---|
  | ① 原始类型 `` `myop : (a : Prop) -> (b : Prop) -> Prop` `` | `judge::judge_type_of_constant(&prefix, &options, &target)` | **整行不见了** ✗（不是被折叠 ✗） |
  | ② 外层表达式类型 `` `a ⊗ b : Prop` `` | `report.hovers`（walk 的 hover） | 在 ✓ |

  ⇒ §11.14 的定性（"就地签名文本 ≠ 慢路文本 ⇒ 折叠形态"）**说错了** ✗：真实的失败形态是
  **①那条查询失败/返回空** ⇒ 该行被跳过（代码是 `if let Ok(ty) = … { if !ty.is_empty() … }`）。
* **档位 A/B（更细）**：`SOKO_JUDGE_INPLACE=shadow` ⇒ **用例通过** ✓ 且退出报告
  `JUDGE_INPLACE used=0 fallback=0 shadow_same=0 shadow_diff=0`、`JUDGE_INPLACE_BY` 同样全 0
  ⇒ 这个场景里**两个被计数的就地档一次都没被走到** ✗。
  ⚠ **但这条读数要打折**：`On` 档那一跑是 **FAILED**（panic）⇒ 退出报告**没打出来** ✗，
  所以"On 档下也没走就地"**不能**从"shadow 档是 0"推出来。
* **下一步（收窄到可执行）**：① 先在 `On` 档下把**退出报告强制打出来**（panic 不吞计数器 ——
  照 `judge_inplace_report()` 的形状加一条 `catch_unwind` 后打印，或把该用例改成
  "不断言、先打印再断言"）⇒ 拿到 `used/fallback` 与 `INPLACE_WHY` 的原因串;
  ② 有了原因再决定是"就地这一支失败"还是"缓存写回污染了 `judge_type_of_constant`" ✗；
  ③ 判据不变 = 原用例转绿 **且** `typing` 臂 `by=9` / 墙钟 ~80ms 不动。
### 11.17 第 11 轮（2026-10-09）—— **那条 hover 回归的根因找到**：hover 靠"编译的缓存副作用"

* **证据（front 层复现 · `crates/front/tests/inplace_notation_hover_probe.rs` · 与 LSP 用例同一夹具）**：

  | 调用 | `On`（默认） | `off` |
  |---|---|---|
  | `judge_type_of_constant(整份源, opts, "myop")` | `Ok("(a : Prop) -> (b : Prop) -> Prop")` · `used=3` | 同 ✓ · `used=0` |
  | 同函数、**LSP 真正传的那个 prefix**（`QueryDoc::judge_prefix`） | `prefix.len()=**0**` | 同 |

* **根因链（三档现象全解释 ✓）**：
  1. LSP 的"原始类型"那行读的是 `query.judge_prefix(offset)` —— 它**只拼闭包里的其他模块**
     （`modules.take(len-1)`），**入口自己的文本不在里面**；单文件（无 import）⇒
     `project_modules()` 为 `None` ⇒ **空串** ✗（`query/mod.rs:720`）。
  2. 于是 hover 那一刻的查询是 `judge_type_of_constant("", …)` —— 单看它**必然失败**
     （合成文档里没有 `myop` 的声明）。
  3. 它今天还能出那行，**纯粹靠缓存副作用**：编译期 `elab_notation` 已经用
     **单元自己的文本**问过一次同名常量 ⇒ `judge_type_of_constant` 的函数级 `CACHE`
     （键 = `options_key|name`，**不含前缀** ✓）里有货 ⇒ hover 那次"空前缀"调用**命中缓存** ✓。
  4. **就地档把这条副作用掐了** ✗：`type_of_constant_prefer_inplace` 的
     `InplaceMode::On` 命中分支**只读** `known.signature()` 计数返回
     （`judge.rs:2452-2456`），**不写** `judge_type_of_constant` 的 `CACHE`；
     `Shadow`/`Off` 会走慢路 ⇒ 顺手写缓存 ✓ ⇒ 这就是"`off` 一跑就绿、`On` 红"的全部原因。
* **修法（两条，选一；都不许拿"把显示改回去"当解法 ✗）**：
  * **(a) 正确解 · 在 LSP 侧**（`crates/lsp/src/lib.rs::notation_symbol_hover`）：目标若
    **本文件声明**（`locally_declared` ✓ 已有这个布尔）⇒ 传给 `judge_type_of_constant` 的
    prefix 要**含入口自己的文本**（不要依赖任何缓存副作用 ✓）。代价 = 这一次 hover 会
    重跑一份合成文档（用户点一次才付一次 ✓，可接受）。
  * **(b) 让就地路顺手写缓存**（`judge.rs`）：把 On 命中分支也 `store` 进
    `judge_type_of_constant` 的 `CACHE` —— 但那要先把函数里的 `static CACHE` 提出来成
    可写回的形状（改动更大，且是"继续依赖副作用" ✗）。
  ⇒ **判据（两条都要）**：① `crates/lsp` 的那条 hover 用例在**默认（On）**档转绿 ✓；
  ② `typing` 臂 `by=9` / 墙钟 ~80ms **不动** ✓（不许用它换性能）。
### 11.18 第 12 轮（2026-10-09）—— **修法 (a) 落地**：那行 hover 转绿，`typing` 臂读数不动 ✓

* **落点**：
  * `crates/front/src/query/mod.rs`：新增 **`QueryDoc::judge_prefix_with_entry(offset)`**
    = 闭包前缀 **+ 入口自己的文本**（过 `importless_source` —— 合成文档里 `import` 行会失败 ✓）。
    语义与 `judge_prefix`（"闭包，不含入口"）**分开**，不改变后者的消费者 ✓。
  * `crates/lsp/src/lib.rs::notation_symbol_hover`：目标**本文件声明**（`locally_declared`）时
    用新前缀问 `judge_type_of_constant` —— **不再依赖任何缓存副作用** ✓。
* **判据（两条都过 ✓）**：

  | 判据 | 结果 |
  |---|---|
  | `crates/lsp` 那条 hover 用例（**默认 `On` 档**） | **ok** ✓（`off` 档也 ok ✓） |
  | `cargo test -p sokonanoda-lsp --lib` | **177 passed / 0 failed** ✓（修前 176/1 ✗） |
  | `typing` 臂（性能不许换） | median **77.7ms** · `by=9` · `compile=77ms` · `modules=1` ✓ 不动 |
  | `SOKO_JUDGE_INPLACE=off` 对照 | 同样 ok ✓ ⇒ 两档一致 ✓ |
* **代价（诚实记）**：本文件声明的记法 hover 现在会**多跑一份合成文档**（只在用户点符号时付一次）
  ✓；`typing`/首刀/跨入口三条**热路读数不变** ✓。
* **教训（回写）**：`type_of_constant_prefer_inplace` 的 `On` 命中分支**只读不写**缓存（正确 ✓），
  而**下游有一处消费者靠那张缓存的副作用活着** ✗ ⇒ 提速类改动必须问一句
  "**谁在靠这次调用的副作用？**"（本轮就是 `judge_prefix` 的空串 + 缓存副作用组合出来的
  定时炸弹）。影子档拦不住它（它比的是"就地 vs 慢路"的**返回值**，不是"谁写了缓存" ✗）。
### 11.19 第 13 轮（2026-10-09）—— 首刀那 128ms 差额**不是** A5 预热挡的（探针加读数）

* **读数**（`perf_keystroke_wallclock.rs` 的 `first-keystroke` 臂新加一行）：

  ```
  PERF first-keystroke-warmup: warm_lines=0 last=None
  PERF first-keystroke-after-open …: 205.6ms · compile=78ms modules=1 prefix=0 by=9
  ```

  ⇒ 用户"开档就敲"的那一刀**到达诊断时 A5 的预热行还没出现**（`warm_lines=0`）——
  要么它被"有 pending 编辑就跳过"的退让挡掉了（**设计如此 ✓**）、要么还没跑完；
  两种情况下它**都不是**那 128ms 差额的来源 ✓（上一轮的怀疑被否掉）。
* **⇒ 差额在服务端热路**（方向②）：候选依次是（a）`didChange` 处理排在开档那条流水线
  （publish/进度条 End/下游调度）之后；（b）防抖静默期；（c）编译 worker 的排队。
  **下一步（写死）**：在 LSP 侧给**一条** `didChange` 打三段时刻（收到通知 / 编译任务上 worker /
  发诊断），与 `compile=` 并排 ⇒ 一眼看出差额落在哪一段 ✓（`crates/lsp/src/lib.rs`，本线可改 ✓）。
  判据：三段之和 ≈ 现有墙钟（自洽 ✓），且改完 `typing` 臂读数不动 ✓。
### 11.20 第 14 轮（2026-10-09）—— 首刀那 128ms **就是 120ms 防抖静默期**（决定性 A/B ✓）

* **读代码**：`crates/lsp/src/lib.rs` 有**防抖静默期**（默认 **120ms**，`SOKO_DEBOUNCE_MS` 可覆盖），
  而它**只对"重建慢的文件"生效**（`SLOW_REBUILD = 150ms`；`debounce_for` 读**这份文档上一次
  编译的耗时**，≥150ms 才等静默期 ✓ —— clangd 的规则，防的是"连打时每键都重编"）。
* **决定性 A/B**（同一构建 `lsp-cargo-mtime=1791480854`）：
  `perf_course_first_keystroke_after_open_is_recorded`

  | 环境 | 墙钟 | `compile` |
  |---|---|---|
  | 默认 | **205.6ms** | 78ms |
  | `SOKO_DEBOUNCE_MS=0` | **95.0ms** ✓ | 94ms |

  ⇒ 差额 **110ms ≈ 静默期 120ms** ✓✓ —— §11.19 那 128ms 的谜底就是它，**不是**预热、也不是 worker 排队。
* **机制**（为什么"开档就敲"会中招）：这一臂的**开档**是冷的（真编了一遍，记下的 `cost` ≥ 150ms）
  ⇒ 紧接着的第一刀被判定成"慢文件编辑" ⇒ 等满静默期 ✗。**用户形状**恰恰是"打开 → 读一眼 → 敲"，
  第一刀等 120ms 是纯亏（他根本没在连打）。
* **下一步（写死 · 一个有原则的收窄，不是一刀切关掉）**：静默期只在**"用户可能正在连打"**时生效
  —— 即在 `debounce_for` 里加一条**空闲复位**：距**上一次编辑**超过一个阈值（如 ≥ 1s）、
  或这是**开档后的第一刀** ⇒ **不等**静默期 ✓；连打（间隔 < 阈值）时保持今天的行为 ✓（冻结保护不丢）。
  **判据**：① `first-keystroke` 臂 205.6 → **≈95ms** ✓（对齐 `SOKO_DEBOUNCE_MS=0` 那一臂）；
  ② **连打保护不丢**：造一个"每 30ms 敲一次、编译 >150ms"的用例，断言**编译次数 < 编辑次数** ✓；
  ③ `typing` 臂读数不动 ✓。
### 11.21 第 15 轮（2026-10-09）—— §11.20 的空闲复位**第一版没生效**（已撤回）＋**判据改成"写紧跟写"**

* **做了什么**（随后 `git checkout` 撤回 ✓）：按 §11.20 加了"**连打窗口**"——`schedule` 时把
  **上一次调度时刻**记进 `edit_prev`，`debounce_for` 里 `slow && prev.elapsed() < 1s` 才等静默期。
* **读数否掉了它**：`first-keystroke` 臂仍然 **201.8ms**（`compile=79ms`）✗ —— 没有变成
  `SOKO_DEBOUNCE_MS=0` 那一臂的 95.0ms。
* **为什么**（这一步才是本轮的收获）：**开档自己也会调度一次编译** ⇒ 紧随其后的第一刀
  距"上一次调度"只有几毫秒 ⇒ 被算成"连打" ✗。**时间窗分不出"开档"与"编辑"**，
  所以这条判据从根上是错的 ✗。
* **改对的判据（写死 · clangd 原话的字面义）**：连打 = "**写紧跟写**" = **这次编辑到来时，
  已经有 pending/inflight 的编译任务** ✓。落到代码：`schedule` 时看一眼
  `pending`/`inflight` 里有没有这条 uri ⇒ 有 ⇒ 本次 `debounce_for` 才等静默期 ✓；
  没有 ⇒ 不等 ✓（开档后的第一刀、以及"编译已跑完、用户隔了一会儿又敲"都归这一类 ✓）。
  这条规则**自调节**：编译比敲键快 ⇒ 永远不防抖（本来也不该 ✓）；编译比敲键慢 ⇒ 队列里
  总有活儿 ⇒ 自动防抖 ✓✓。
* **判据（三条，落地时逐条跑）**：① `first-keystroke` 臂 ≈ **95ms**（对齐 `SOKO_DEBOUNCE_MS=0` 臂 ✓）；
  ② **连打保护不丢**：冷开（记下 `cost ≥ 150ms`）后**连发 5 刀、每刀间隔 30ms** ⇒ 断言
  **编译次数 < 编辑次数** ✓（新用例：`crates/lsp/tests/` · 本线可写 ✓）；
  ③ `typing` 臂读数不动（现 **80.8ms** ✓）。
### 11.22 第 16 轮（2026-10-09）—— **§11.21 的判据落地**：首刀 **205.6 → 79.6ms** ✓（连打保护已钉住 ✓）

* **落点**（`crates/lsp/src/lib.rs`）：`Compiler` 加 `burst: Mutex<HashMap<Url, bool>>`；
  `schedule` 里判 **"这次编辑到来时这份文档已经有待编/在飞的任务"**（`pending.contains_key
  || inflight.contains`）写进 `burst` ✓；`debounce_for` 从"**上次编译慢就等**"改成
  "**慢 ∧ 写紧跟写**才等" ✓（时间窗那版已在 §11.21 撤回 ✗）。
* **读数（同构建 `lsp-cargo-mtime=1791481239`）**：

  | 臂 | 改前 | **改后** |
  |---|---|---|
  | **`first-keystroke-after-open`（打开就敲）** | 205.6ms（compile 78ms ⇒ 118ms 纯等） | **79.6ms** · compile 79ms ✓（**墙钟 ≈ 编译** ⇒ 白等没了） |
  | `typing`（真实连续键入） | 77.7 / 80.8ms | **78.5ms** ✓ 不动 |
  | `proof` / `statement` | ~82ms | 81.7ms ✓ |
  | `typing_equal_length` | 41ms | 40.6ms ✓ |
  | `cross-entry-switch` | 711ms | 跑通 ✓（探针 3 条全绿） |
* **连打保护（新判据 · `crates/lsp/tests/lsp_debounce_burst.rs`，两条都绿 ✓）**：
  * `a_burst_of_edits_is_still_coalesced`：冷开（真编 ⇒ 记下 `cost ≥ 150ms`）后**连发 5 刀、
    间隔 30ms** ⇒ 断言 `1 ≤ 编译次数 < 5` ✓（**保护没丢**）；
  * `spaced_out_edits_are_not_debounced`：每刀**等诊断回来**（≈"读一眼再敲"）⇒ 断言**恰好 3 次** ✓
    （**不该吞**）。
  ⚠ 诚实记：这两条是**防退化**的守卫（旧规则也能过它们 ✓）；**证明"这次改对了"的是
  `first-keystroke` 臂那条读数**（205.6 → 79.6ms ⇒ 对齐 `SOKO_DEBOUNCE_MS=0` 的 95.0ms ✓）。
* **全量**：`cargo test -p sokonanoda-lsp` **各目标全绿** ✓（177 lib + 全部集成目标，含新判据 2/2 ✓）。
### 11.23 第 17 轮（2026-10-09）—— 收尾验证（本轮的改动面已全部覆盖，cli 全量顺延）

* **起因**：第 12 轮（`judge_prefix_with_entry` + hover 接线）与第 16 轮（静默期）之后，
  **front / cli 两个 crate 的全量还没重跑过** ✗（只跑过点名的几个判据文件）⇒ 本轮补。
* **读数**：

  | 套件 | 结果 |
  |---|---|
  | `scripts/dev-verify.sh`（CLI 路结构计数） | ✓ 冷 `passes=13` / 改一行 `passes=3`（**与历史逐字相同** ⇒ CLI 路没动 ✓） |
  | `cargo test -p sokonanoda-front` | ✓ **34 个目标全 ok**（0 FAILED / 0 error） |
  | `cargo test -p sokonanoda-lsp`（第 16 轮同构建） | ✓ 177 lib + 全部集成目标（含新判据 `lsp_debounce_burst` 2/2） |
  | `cargo test -p sokonanoda-cli` | ✓ **0 FAILED / 0 error**（第 18 轮回收那次运行的结果：判定用的 `grep -E "^test result: FAILED\|FAILED\|^error"` **一行都没出** ✓；ok 计数那一遍是我脚本里多余的重复运行，不影响结论 ✓） |
  | 全课程 `--json` 逐字节 | 沿用平行线 §11.7 的独立对拍（101/101 ✓，那之后**判定路径**只动过 LSP 的 hover 前缀与静默期 —— 都不在 CLI 判定路上 ✓，但仍以 cli 全量重跑为准） |
* **记账**：四方向的剩余件不变（① T1-B · ② T2-B）；北极星账面（同机 · unit08）：
  **真实连续键入 78.5ms** · **开档就敲 79.6ms** · 跨入口切换 711ms。

### 11.24 第 18 轮（2026-10-09）—— 收尾三件：cli 读数回填 ✓ · clippy 干净 ✓ · **fmt 漏了一次已补** ✓

* **cli 全量**（回收第 17 轮那次运行）：判定用的
  `grep -E "^test result: FAILED|FAILED|^error"` **一行都没出** ⇒ **0 FAILED / 0 error** ✓
  （`dev-verify` 的 CLI 结构计数也与历史逐字相同 ⇒ CLI 路没动 ✓）。
* **clippy**（`-p sokonanoda-lsp -p sokonanoda-front --all-targets`）：全部 warning 都落在
  **`crates/kernel/**`**（**既有**，内核按 `AGENTS.md` 本就不参与教学 crate 的 fmt/clippy 收尾 ✓）；
  **我改过的文件一条都没有** ✓（`grep -B3` 逐个文件核过）。
* **fmt：抓到并修掉一次自伤** ✗→✓：`cargo fmt -p … -- --check` 在
  **`crates/lsp/src/lib.rs:745`**（第 16 轮那条 `self.burst.lock()…insert(…)` 链式调用）
  报 diff ⇒ `rustfmt` 收尾后 `--check` **干净** ✓。
  **教训**：改完 `.rs` **必须跟着跑一次 `--check`**（我这轮前只跑过 `cargo check` ✗ —— 它不管格式）。
* **复查（fmt 之后重跑，证明只是格式）**：`lsp_debounce_burst` **2/2 ✓** ·
  `first-keystroke-after-open` **76.8ms**（`compile=76ms`）✓ ⇒ 与 §11.22 的 79.6ms 同档 ✓。

### 11.25 第 19 轮（2026-10-09）—— 写穿缓存的**第二个消费者**（平行线补 · 带反向验证）

* **背景**：§11.17 把 hover 回归的根因定到"就地档掐掉了 `judge_type_of_constant` 的
  **缓存副作用**"✓；§11.18 选了修法 **(a) LSP 侧换前缀**（`judge_prefix_with_entry`），
  并把修法 **(b) 就地路写穿缓存**评为"改动更大、且继续依赖副作用 ✗"。
  ⇒ **平行线保留了 (b)**（`1dc71749`）—— 理由是**消费者不止一个** ✗。
* **第二个消费者**（本轮实测 · 此前**零判据**）：`crates/lsp/src/lib.rs` 的
  「**记法的目标**」分支（`notation_target_at` ⇒ `judge_type_of_constant("", …)`，
  **空前缀**）—— 空前缀**合成不出**声明 ⇒ 它**只能靠编译期那次调用的缓存副作用**
  活着。§11.18 的 (a) 只改了「记法符号」那一条 ⇒ **这一条仍会静默少一行签名** ✗。
* **落点**：
  * `1dc71749`（`judge.rs`）：把 `judge_type_of_constant` 的函数级 `CACHE` 抽成
    `type_of_constant_cache_get/put`，就地路答上时**按同一把键写回** ✓
    （只缓存成功，沿用原语义；`CAP = 4096` 的闸类出口照旧 ✓）。
  * `5c5100af`（`crates/lsp/src/tests/hover.rs`）：新增
    `hover_on_a_notation_target_name_shows_its_signature` —— hover
    `infix:50 " ⊗ " => myop` 的目标名 ⇒ 必须给原始签名 ✓。
* **判据**：① `cargo test -p sokonanoda-lsp --lib` **178/178** ✓（修前 176/1）；
  ② **反向验证（已做）**：临时停掉 `type_of_constant_cache_put` 那行 ⇒ **新用例判红** ✓
  （守卫有牙，随后已还原）；③ `typing` 臂 Σ合成趟仍 **0**（写缓存**不**重新引入合成趟 ✓）·
  影子档 `same=78246 · diff=0` ✓；④ `--json` On/Off 逐字节相同（抽 24 份）✓ ·
  `cargo test -p sokonanoda-front --lib` **875/875** ✓。
* **两修法并存（互补，不是重复 ✗）**：(a) 让「记法符号」那条**不再依赖**副作用 ✓；
  (b) 让**所有**消费者的旧行为**一字不变** ✓（第二消费者仍在靠它，直到它也拿到正确前缀）。

### 11.25 第 19 轮（2026-10-09）—— 同一条根因的**两个修法都在树上**（互为补充 ✓）

* **两条都落了**：
  * **本线 (a)** `2399fcca`（§11.18）：`notation_symbol_hover` 里目标**本文件声明**时改用
    `judge_prefix_with_entry`（闭包前缀 + 入口自己的文本 ✓）——**不再依赖任何缓存副作用** ✓；
  * **平行线 (b)** `1dc71749`：`judge_type_of_constant` 的**就地路写穿常量签名缓存**
    （把 §11.17 说的那条被掐掉的副作用补回来 ✓）＋ `5c5100af` 给它**补判据**
    （`hover_on_a_notation_target_name_shows_its_signature` · **带反向验证**：临时停掉
    `type_of_constant_cache_put` ⇒ 判红 ✓ ⇒ 守卫有牙 ✓）。
* **为什么两条都需要（不是重复）**：
  * (a) 修的是**记法符号**那条 hover（它自己拼前缀 ⇒ 可以自足 ✓）；
  * (b) 修的是**第二个消费者**「记法的目标」分支（`notation_target_at` ⇒
    `judge_type_of_constant("", …)`，**空前缀**）—— 它**只能**靠缓存副作用 ✗。
* **读数**：`cargo test -p sokonanoda-lsp --lib` **178/178 ✓**（我第 16 轮是 177 ⇒ 他们 +1 条 ✓）；
  两条 hover 用例（符号 / 目标名）**都 ok** ✓；北极星收益不变（§11.22 的 76.8ms ✓）。
* **仍留的设计味（记账，不是缺陷）**：`notation_target_at` 那条**至今仍走空前缀 + 缓存副作用** ✗
  —— (b) 让它继续成立并有守卫 ✓，但**干净终点**是让它也像 (a) 一样**自足**（把入口文本带上），
  那样两个消费者都不再依赖"谁先调用过" ✓。**留给后续**（要动 `notation_target_at` 的调用面）。

