# 性能测试与基线（I13-S5c）

性能是本项目的生命线（REQUIREMENTS §9 四十四）。本文档描述性能测试的
三层结构、阈值设计原则与当前基线；**每次 push 都会在 CI 上例行执行**，
回归即红，且每版留档（`perf-report` artifact，带版本 + commit SHA）。

## 分层结构

> I16（0.57.0）起，项目层（`import` 闭包）与**编辑器宿主**各自多了一层探测；
> 分阶段的**机器可读台账**在 `docs/perf/ledger.jsonl`（`scripts/perf-ledger.sh`），
> 详见本文末「项目层与编辑器宿主」一节。

### 第 1 层：阈值断言（回归哨兵，CI 强制）

### 第 1 层：阈值断言（回归哨兵，CI 强制）

不是 microbenchmark（那需要 criterion），而是**算法级回归哨兵**——
O(n²) 或意外的前缀重编译必然触发，CI 噪声不会误报：

| 测试 | 位置 | 断言 |
| --- | --- | --- |
| `check_document_scaling_is_linear` | `crates/front/tests/perf.rs` | 编译 400 块 ≤ 编译 50 块的 12× 时间（线性=8×，O(n²)=64×） |
| `incremental_edit_anywhere_is_fast` | 同上 | 50 块文件里逐个编辑 10 个练习，每次 `Session::update` < 50ms，且 `kernel_checks ≤ 1`（只重查被编辑块） |
| `editing_first_exercise_does_not_slow_down_with_file_length` | 同上 | 编辑第一个练习的延迟：250 块 ≤ 50 块的 8×（线性=5×） |
| `perf_did_change_latency` | `crates/lsp/src/lib.rs` | didChange→诊断 round-trip < 50ms（50 块文件） |
| `perf_completion_and_hover_latency` | 同上 | completion / hover 各 < 10ms |
| `perf_goals_view_latency` | 同上 | `soko/goals`（goal 视图，教学核心）< 10ms |

### 第 2 层：judge 缓存正确性

`judge_cache_returns_identical_results_and_stores_entries`
（`crates/front/src/judge.rs`）：缓存命中必须与直算逐字节一致，
不同 prelude 模式不串台。缓存是性能设施，**正确性测试钉住它**。

### 第 3 层：性能留档（每版本可追溯）

- **CI**：`test` job 的 "Performance report" 步骤跑全部 perf 测试，
  提取 `PERF` 行写入 `perf-report-v<version>-<sha>.txt` 并上传为
  `perf-report` artifact（每次 push 都有，不只在失败时）。
- **本地**：`scripts/perf-report.sh [输出文件]`，同一口径。
- 排查回退：对比相邻两个版本的报告 → 定位劣化的具体场景
  （编译器缩放？增量编辑？哪个 LSP 请求？）→ `git log` 找到改动。

## 阈值设计原则

1. **只抓算法级回归**：阈值按「线性理论值 × 1.5~1.6 余量」设定，
   CI runner 2-3 倍的性能波动不会误报；O(n²)（16-64×）必然触发。
2. **绝对延迟阈值宽松**（50ms/10ms）：抓的是「用户可感」的交互劣化，
   不是 benchmark 精度。
3. **覆盖编辑器全路径**：VS Code 扩展的所有特性都经 LSP——
   didChange（诊断）、completion、hover（含半截表达式 goal-state）、
   `soko/goals`（goal 视图）。扩展层没有独立的计算路径需要测。

## 当前基线（2026-09-13，v0.23.0，Apple Silicon 本机）

| 场景 | 实测 |
| --- | --- |
| check_document 缩放 [50/200/400 块] | ~45/116/158 ms（3.5×/8×大小，线性） |
| 增量编辑（50 块文件，逐练习） | 初始 ~47ms，每键 <5ms，kernel_checks=1 |
| 编辑首练习缩放 [50→250 块] | ~2.2→4.4 ms（1.9×，线性=5×） |
| LSP didChange round-trip（50 块） | <1ms |
| LSP completion / hover / goals | <1ms |

教学规模（20-50 块）每键 3-8ms——无感。500+ 块仍 <100ms。

## External baseline：Lean Kernel Arena（opt-in，非 CI）

仓库内的 perf 套件是**哨兵**（抓算法级回归），不是与外部实现的**基准对比**。
真正的横向基准用上游 [`leanprover/lean-kernel-arena`](https://github.com/leanprover/lean-kernel-arena)
的 NDJSON 语料：它给出 accept/reject 期望，能同时验证我们内核的**性能**与
**soundness**（例如 `extra-rec` 未派生 recursor、`nat-rec-rules` 伪造 iota 规则）。

- 已把语料跑法固化成 `scripts/perf-arena.sh` + 既有的
  `crates/kernel/tests/arena.rs`（`LEAN_KERNEL_ARENA` 环境变量门控；未设置时自动
  跳过，CI 不依赖它）。
- 语料很大且属外部仓库，**不 vendor**；获取：`git clone` 后
  `uv run lka.py build-test` 生成 `_build/tests/*.ndjson`。
- 跑法：`LEAN_KERNEL_ARENA=/path/to/lean-kernel-arena scripts/perf-arena.sh`
  （输出各 corpus 的通过情况与总耗时）。
- 该基准面向**贡献者**（需要克隆外部语料），不属于用户/agent 路径；不引入
  官方 Lean 工具链（只消费 NDJSON）。

## 历史教训（为什么有这些测试）

- **funapply 的 O(n²)**（0.22.0 移除）：judge_infer 全前缀重编译 ×
  每键 × 每块。judge 结果指纹缓存（封顶 128）兜底 by 块 tactic 同源问题。
- 纪律（`docs/LESSONS.md`）：**front 降低/判定路径禁止 per-keystroke 的
  全文档重编译**；需要内核信息的特性要么缓存、要么只在显式请求时计算。

---

## 项目层与编辑器宿主（I16，0.57.0）

`import` 闭包让"一次按键"的成本从"一个文件"变成"**整个闭包**"（项目模式不走
单文件增量路径，见 `docs/architecture.md` §4.5）。所以项目层单独有一套哨兵，
并且**每个阶段**都留一行机器可读记录。

### 新哨兵（第 1 层，CI 强制）

| 测试 | 位置 | 断言 |
| --- | --- | --- |
| `project_closure_stage_costs_are_recorded` | `crates/front/tests/perf_project.rs` | 4×20 项目总成本 < 2000ms（数量级哨兵）；`plan+digest` 不主导编译成本 |
| `project_closure_compile_scales_linearly` | 同上 | 模块数 ×4 ⇒ 时间 < 6.4×（线性 4×，O(n²) 16×） |
| `project_keystroke_recompiles_the_closure_within_budget` | 同上 | 改一行重编译整个闭包 < 2000ms（数量级哨兵） |
| `project_compile_with_overlay_costs_the_same_order` | 同上 | 内存覆盖（LSP 每次通知走它）不得比读盘慢一个量级 |
| `project_cli_cold_and_warm_costs_are_recorded` | `crates/cli/tests/perf_project.rs` | 热缓存必须快于冷跑；依赖改动后必 miss |
| `project_cli_query_and_build_costs_are_recorded` | 同上 | `build` 之后 `query` 必命中 |
| `perf_project_did_open_and_keystroke` | `crates/lsp/src/tests/perf.rs` | 一次按键 < 300ms，且**只发一份文档的诊断**（`publishes_per_keystroke == 1`） |
| `perf_project_dependency_edit_refreshes_dependents` | 同上 | 改依赖 ⇒ 下游被重发；扇出不超过已打开文档数 |
| `perf_project_requests_are_interactive` | 同上 | 项目入口的 hover / definition / goals 各 < 50ms |
| `editor/vscode/test-extension-host.js`（7 例） | 扩展宿主 stub | 诊断过滤/合并、并发 goals 合并、切文件丢弃过期答案、webview 去重、课程树缓存 |

### 编辑器打开项目文件：冷 vs 热（T-A10/T-A11/T-A14，2026-09-21 实测）

真课程、**真的 `sokonanoda-lsp` 进程**（`scripts/soko lsp`，debug 构建，含进程
启动与 prelude 初始化），量的是 `didOpen → 第一条 publishDiagnostics`。

热的那次**不靠 CLI 预热**——第一次 LSP 打开自己就把条目写下了（T-A11），
第二次换个进程、同一份缓存。

| 文件 | 冷开（无缓存） | 热开（第二次） | 倍数 |
|---|---|---|---|
| `unit01-sets-membership` | 1833ms | **2ms** | 916× |
| `unit08-images-preimages` | 4821ms | **8ms** | 603× |
| `unit12-synthesis` | 8838ms | **10ms** | 884× |
| `unit12-solution` | **36259ms** | **26ms** | 1395× |

冷开那几列与本文 §2.1 的修前基线**逐项吻合**（1812/1830、4729/4800、7910/7871）
——所以这不是"换了个夹具量出来的好看数字"，是同一条路径。

**用户视角**：`unit12-solution` 那份文件以前每打开一次等 **36 秒**；现在第二次
起 **26ms**。热开已经和"同文本再 didChange"同量级（后者实测 0ms，见 T-A21），
也就是**剩下的成本就是算摘要**（读 + 解析整个闭包）——那是缓存的地板，
再往下要等线 K 的跨模块增量。

**一个必须知道的限制**：摘要按**依赖的磁盘内容 + 打开文档的内存覆盖**算。
所以在编辑器外改了一个依赖（`git checkout`、别的工具写文件）之后，第一次打开
仍然要重编——那是正确的（闭包真的变了），缓存不是"永远不编"。

### 分阶段 profile：`unit12-solution` 的 120 秒花在哪（T-K03，2026-09-21）

**最坏样本** `courses/set-theory/units/solutions/unit12-solution.sokonanoda`
（526 行、9 道题、全 tactic 风格）。debug CLI、隔离缓存、冷跑。

两个口径互相印证：

| 口径 | 数字 |
|---|---|
| 墙钟（`/usr/bin/time -p`） | **120.4s** |
| `SOKO_JUDGE_STATS=1` 的判定分阶段 | **81.9s / 25 次调用**，平均 **3.3s/次**，123 对判定，前缀合计 1,064,669 字节 |
| `sample` 采样（8 秒窗口，5583 个样本） | `run_by → judge_pairs_uncached → check_document_with` = **64.0%** |

| 阶段 | 占比 | 说明 |
|---|---|---|
| **`by` 块判定**（`run_by` → `flush_batch` → `judge_pairs_uncached` → `check_document_with`） | **68%**（81.9s / 120.4s） | 每次判定都把**整份前缀重新 `check_document_with` 一遍**（闭包前缀 + 本文件已判过的声明）。25 次调用吃掉了 2/3 的墙钟 |
| 主 pass（elaboration + 内核检查 + pp） | ~32%（38.5s） | |
| ├ 其中 `build_def` / `elab_expr` | 判定里的 33%（采样） | 深递归的 `elab_expr` |
| └ 其中 `infer` | 判定里的 36%（采样） | |

**这直接给出两把刀的收益上界**：

* **T-K11（`by` 块判定的前缀复用）= 68%** —— 如果前缀复用能做到零成本，
  这是它最多能省的。**这才是这个文件的第一刀**。
* **只省"内核检查"的刀最多 27%** —— 采样里没有独立的 `sokonanoda_kernel` 帧
  （被内联进前端），所以内核自身的检查不是主要矛盾；主要矛盾是**前端反复重跑
  同一段前缀**。

**为什么 profile 随文件形状变化很大**（同一批次的实测）：

| 文件 | `by` 块 | 判定占比 | 说明 |
|---|---|---|---|
| `unit08-images-preimages` | 9 个 | **7%** | 把 9 个 `by` 全换成 `sorry` 只从 4.96s 降到 4.60s |
| `unit12-solution` | 9 道题全 tactic | **68%** | 本表 |

⇒ **不能只按一个文件选刀**。课程语料里 tactic 风格的解答（`solutions/`）是
`by` 块密集的，那才是最坏样本。

**怎么重量**（常驻开关，不再是一次性探针）：

```bash
SOKO_JUDGE_STATS=1 scripts/soko grade courses/set-theory/units/solutions/unit12-solution.sokonanoda
# → JUDGE_STATS calls=25 total_ms=81925 avg_ms=3277 pairs=123 prefix_bytes=1064669

# 采样（macOS 自带，不用改代码）：
SOKONANODA_CACHE_DIR=$(mktemp -d) ./target/debug/sokonanoda grade <入口> &
PID=$(pgrep -n sokonanoda); sample $PID 8 -f /tmp/sample.txt
```

### 编译期间**整个 LSP 冻结**（T-A30 实测，2026-09-21）

冷编译 unit12（8.9s）**进行中**，连发 5 次 `soko/stateAt`：

| 第几次 | 延迟 |
|---|---|
| 第 1 次 | **8907ms**（= 整个编译期） |
| 第 2–5 次 | 1 / 2 / 2 / 1ms |

⇒ 编译期间 `docs` 那把 `Mutex` 被占住，**所有**只读请求（hover / 目标栏 /
`soko/goals` / 补全）一起冻结。8.9 秒里编辑器像死了一样。
（另一次冷编译 unit12-solution 时量到 **24393ms**。）

**为什么"把编译挪出锁"不够**（试过，已回退）：LSP 的**消息循环是串行的**
——`did_change` 的 handler `await` 着 `refresh`，后面的请求要等它返回，
跟锁没关系。真正的修法是**把编译 spawn 出去**，让 handler 立刻返回。

**为什么这一轮没做**：试的过程中撞到两件事，都需要先有护栏：

1. **短路与"依赖在磁盘上变了"互相纠缠**：`set_text` 里那条"文本/模式/路径/覆盖
   都没变就不重编"（T-A21）在锁外编译时**必须带着当前状态**才成立，而
   `QueryDoc` 里的 `Session` **不可 `Clone`**（它持有上一版快照）。
   要么给 `Session` 实现 `Clone`，要么把短路判据改成**摘要**（摘要含依赖的
   磁盘内容 + 覆盖 ⇒ 这才是正确判据）。
2. **并发正确性**：spawn 之后两次编辑会并发编译，写回必须靠版本校验，
   而版本校验的判据要重做（第一版用"文本相等"，首次打开时文档还是空的 ⇒
   结果被丢弃，18 个测试红了）。

**结论**：T-A30 **未完成**，但缺口已量清。它应当与 T-K01/K02 的护栏一起做
（先有语料对拍，再动并发），或者与 K1 一起做（编译变便宜之后，冻结的绝对时长
也会跟着降）。

### 扇出：改一个依赖 = N 份文档各编一遍（T-A23，2026-09-21 实测）

打开 3 个课程单元（都 `import lib/Set`），改 `lib/Set` **一行**，量"从发出
`didChangeWatchedFiles` 到 3 份文档全部重新发布诊断"：

| | 耗时 |
|---|---|
| 打开 3 个单元（冷） | 1835 / 796 / 1731ms |
| **扇出总耗时** | **4872ms** |

≈ 3 × 单份闭包编译 —— 因为**每一份**受影响文档都各自把**自己的整条闭包**
从零重编（`Doc::set_text` → `project_compile`）。

**这跟 G-29（编辑重编整条闭包）是同一个病**：扇出只是把它乘上了"打开了几份
文档"。所以修法也是同一个：线 K 的 K1（复用依赖已编译好的环境）——
依赖没变时，N 份文档里那些**共享的依赖**只该编一次，而不是编 N 次。

**已经做对的两件事**（不要再"优化"它们）：只重编**闭包里含这个路径**的文档；
未受影响的文档直接复用上次诊断而不重编。

### `build <目录>` 是 O(文件数 × 闭包)（T-A25，2026-09-21 实测）

`crates/cli/src/build.rs` 对目录里**每个** `*.sokonanoda` 各跑一次
`plan_project` + `compile_plan` —— 每个入口都编**自己那一份完整闭包**，
文件之间不共享。实测 `courses/set-theory`（35 个文件，debug CLI、隔离缓存）：

| | 耗时 | build 摘要 |
|---|---|---|
| 冷跑 | **2m29.8s** | `0 hit, 35 compiled, 0 failed` |
| 热跑（修 G-24 **之前**） | **2m30.2s** | `1 hit, 34 compiled, 0 failed` |
| 热跑（修 G-24 **之后**，T-A05） | **0.176s** | **`35 hit, 0 compiled, 0 failed`** |

**热跑几乎不省的原因**（G-24）：那个 `requires = "0.61"` 的版本漂移让
`requires_warning` 有值 ⇒ `is_clean()` 为假 ⇒ 项目缓存永不写。
T-A05 把"漂移"从"不干净"里摘了出去（它是**可回放的确定性事实**——条目里带着
`requires_warning`，回放时警告一起回来），并把那条警告补进 `query check` 的
`warnings[]`（否则机器可读通道里就再也没有它的影子）。

**0.176s vs 2m30.2s ≈ 850×**。这是"打开变快"的第一块地基：CLI 侧已经好了，
编辑器侧还差 T-A10（LSP 接上这份缓存）。

**含义**：编辑器里的 `sokonanoda: build`（`alt+b`）在大课程上就是**分钟级**，
而且它**不是**"编一次全项目"——它是"每个文件各编一遍闭包"。
真正的修法是 T-K30（按模块根分组、每个模块只编一次），依赖线 K 的跨模块增量。

### 性能回归怎么判（`scripts/perf-compare.py`，T-022）

以前只有**人肉规则**（"优先比 `best_ms`、±25% 内算同档"）——100+ 个环节推进时，
"修 A 弄慢 B"必然发生，而它**只有机械判据能拦住**。现在有脚本：

```bash
python3 scripts/perf-compare.py                 # 台账最后一条 vs 上一条
python3 scripts/perf-compare.py --since 0.58.0  # 指定基准（sha 前缀 / 版本号 / -N）
python3 scripts/perf-compare.py --json          # 机器可读
python3 scripts/perf-compare.py --self-test     # 自检判定规则（7 条）
```

**四条判定规则**（`--self-test` 逐条钉住）：

| # | 情况 | 判定 |
|---|---|---|
| ① | 某个 `*_ms` 指标退化 **> 阈值**（默认 25%） | **红** |
| ② | **新增**的 `(scope, case)` | 提示"无基线"，**不红** |
| ③ | **消失**的 `(scope, case)` | **红**——悄悄删掉哨兵是最隐蔽的回归 |
| ④ | 两条记录**宿主或 `cli_profile` 不同** | 只提示"不可比"，**不红**（连"消失"也不判——两条记录本就不是同一口径） |

**两条额外的实践规则**（都是真台账上立刻暴露出来的）：

* **噪声带**：低于 **5ms** 的指标不判红（`--floor-ms`）。第一次跑真台账就撞上
  `dependency_edit_refresh.elapsed_ms` 从 1.0ms 到 3.0ms = "+200%" ——那是噪声，不是回归。
* **夹具不同**（`modules`/`decls_per_module` 等描述字段不一致）⇒ 标"夹具不同"、
  不判红，但值得人看一眼：那说明哨兵本身被改了。

**判读纪律**（沿用本文的既有教训）：红了**先复测一次**；仍然退化再查这一版改了什么。
放宽阈值前必须先做两件事——① 改采样口径（串行 + best-of-N）；
② 与 pre-batch 二进制同夹具对拍。两步都排除掉，才谈阈值（见本文「噪声地板与采样口径」）。

**CI**：`ci.yml` 的 "Performance report" 步骤现在**直接调用 `scripts/perf-report.sh`**
（不再复制粘贴命令）——口径只有一份真相，新增套件自动带上。
`perf-compare.py` 是**本地每环节**用的（CI 上跨 runner 的实例方差有 4×，
不适合自动判红）。

### 真实课程闭包（T-007 / T-020，2026-09-21 起）

> **为什么单独一套**：上面那张表的夹具是 2–5 模块 × 10–12 声明，比
> `courses/set-theory` **小 1–2 个数量级**——既有数字 12–14ms，而用户真实的
> "打开一个单元"是**秒级**。合成夹具量不出用户感受到的那个量级。

`crates/lsp/src/tests/perf_course.rs` **直接用仓库里的真课程**
（`courses/set-theory/`，读不到就跳过——课程仓可以分开检出），
打 `PERFJSON` 时 `scope: "lsp-course"`；重活互相串行（`COURSE_PERF_LOCK`）。

**修前基线（2026-09-21，v0.63.0，Apple Silicon，release LSP）**：

| case | 入口 | 实测 | 说明 |
|---|---|---|---|
| `did_open` | `units/unit01-sets-membership`（2 import） | **1812ms** | 6 条诊断 |
| `did_open` | `units/unit08-images-preimages`（4 import） | **4729ms** | 9 条诊断 |
| `did_open` | `units/unit12-synthesis`（7 import） | **7910ms** | 9 条诊断 |
| `did_open_same_session` | 同会话重开 unit01 | **121ms** | `Session` 的"内容未变零重编译"是好的 ⇒ **贵的是第一次打开** |
| `keystroke` | unit08 上改一条声明的名字（8 模块闭包重编译） | **371ms** | 来回改 3 次取最小 |
| `by_block_did_open` | `units/solutions/unit12-solution`（8 模块、25 个 `by`） | **36.1s** | **默认跳过**（会让 `cargo test` 多花几十秒）；`SOKO_PERF_COURSE_SLOW=1` 打开 |

**量级哨兵**：最慢一次 `didOpen` < 60s（抓的是"退化成分钟级"，不是 ±20% 波动）。

**判读**：这套数字是批次 2（线 A）与批次 5（线 K）的"修前"对照物。
预期修后：`did_open` 首次降到与 `did_open_same_session` 同量级（缓存命中），
`by_block_did_open` 从 36.1s 降到个位数秒（线 K 的 K1-b）。

### 台账：`scripts/perf-ledger.sh` → `docs/perf/ledger.jsonl`

每个测试打印一行 `PERFJSON {…}`（`schema: soko.perf/1`），脚本把它们连同
`{version, commit, date, cli_profile, host{system,machine}}` 追加进
`docs/perf/ledger.jsonl`（**提交进仓库**：跨版本/跨机器对比"哪一环退化了"），
并覆盖一份 `docs/perf/latest.json`。人读版仍由 `scripts/perf-report.sh` / CI 的
"Performance report" 步骤产出（两者都包含项目层三段）。

**为什么 CLI 用 release、front/lsp 用测试 profile**：`[profile.test] opt-level = 3`，
所以 front/lsp 的测试数值已经接近发布；而 `cargo test` 为 CLI 集成测试编的
**二进制**走 `[profile.dev]`（opt-level 0），冷编译会慢一个量级——那不是用户看到
的数字（实测同一 3×12 项目：release 冷 24ms / debug 冷 405ms）。CLI 一律
`--release`，台账里记 `cli_profile`。

### 当前基线（2026-09-18，v0.58.0，Apple Silicon，`cli_profile=release`）

口径：**2026-09-18 起为串行**（`--test-threads=1` + 用例内 best-of-N）。
更早的基线（front compile 90–110ms、LSP 按键 25–49ms 等）是并行口径，约偏高 3–4×。

| 场景 | 实测（串行口径） |
| --- | --- |
| front 分阶段（4 模块 × 20 声明） | plan 0.3ms · digest ~0.004ms · **compile 32–38ms** · total ≈ 33–39ms |
| front 缩放（4/8/16 模块 × 10 声明） | 19 / 35 / 65 ms，4× 规模 ⇒ 3.0–3.4×（线性） |
| front 一次按键（4×20，全部重编译） | **best 33–39ms**（同轮 `worst` 34–70ms，作为保守上界） |
| front 教学规模一次按键（2/3/5 模块 × 12 声明） | **14 / 16 / 24ms** |
| front 内存覆盖 vs 读盘（4×20） | 38.7 vs 38.8ms（覆盖无额外成本） |
| front 判据前缀（入口 10 处 `match` 导入的归纳类型，2 模块） | 70–83ms（每次判据都合成"闭包前缀"；judge 缓存按前缀+项命中） |
| LSP 项目 didOpen / 一次按键（2×12） | 12ms / **12ms，每次按键 1 份诊断** |
| LSP 改依赖 ⇒ 下游刷新（3×12，两文档打开） | 1–2ms，2 份诊断；下游 1 条 `import-dependency-failed` |
| LSP 项目 hover / definition / goals | 各 < 1ms |
| CLI 项目冷 / 热 / 依赖改动后（3×12，release） | **29.6ms / 3.4ms / 23.7ms**（必 miss） |
| CLI `build` / `query` 冷 / `query` 热（3×12） | 29.6ms / **20.4ms** / **3.3ms**（0.57.0 起 query 与 check/build 共用闭包缓存，见下） |
| 扩展：一次诊断事件（修复后） | **1 × `soko/goals` + 1 × `soko/stateAt`**（修复前 2 goals + 2 次 webview 整表重建） |
| 扩展：无关语言（`.ts`）的诊断事件 | 0 次请求（修复前 1 × goals） |
| 扩展：光标移动（200ms 去抖） | 1 × `stateAt`，~1KB / 49 DOM 节点 / 0.17ms |
| 扩展：Infoview `decls` 整表重建（50 条） | 28.6KB / 1200 节点 / 1.2–2.1ms（内容不变时不发） |
| 扩展：课程树一次 CLI 运行（11 单元，release 热缓存） | ~320ms；现在 30s 内复用（原来每次 resolve 都重跑） |
| 键盘路径的客户端合并 | `vscode-languageclient` 9.x FULL sync **250ms trailing** 批量：连打只发一次最终文本 |

**测量纪律（踩过）**：macOS 上**刚构建出来的二进制第一次 spawn 要付 ~425ms**
（代码签名校验/页缓存，`--version` 也一样），与编译无关。CLI 性能测试必须先
`warm_up()` 打掉它，否则"冷跑"记的是首次执行成本（第一次写这套测试时记成了
527ms，真实值 29.8ms）。同理，front 的 perf 测试都有显式预热。

**噪声地板与采样口径（2026-09-18 量过，重要）**：这套用例是**哨兵**不是基准，
而它的最大噪声源不是机器，是**同一个测试二进制里用例并行跑**。同一份代码实测：

| 跑法 | `closure_stages.compile`（4×20） | LSP didOpen / 按键（2×12） |
| --- | --- | --- |
| 只跑这一个用例 | **32.4ms** | — |
| 6 个 perf 用例 `--test-threads=1` | **33–38ms** | 12ms / 12ms |
| 6 个 perf 用例默认并行 | **118–152ms**（同一提交两次记录差 28%） | 64ms / 50ms |

即并行口径把单次操作成本放大约 **3–4×**（重活互相抢 CPU/内存带宽/分配器）。
所以纪律是：① **`scripts/perf-ledger.sh` / `perf-report.sh` 一律带
`--test-threads=1`**（2026-09-18 起）——更早的台账条目与 `docs/PERF.md` 里 90–110ms
一类的数字都是**并行口径**，只能和同口径条目比；② 用例内部用 **best-of-N 取最小**
（`perf_project.rs` 的分阶段/缩放在 2026-09-18 改为 `measure_best(…, 3)`；
`front/tests/perf.rs` 的三个用例在 **2026-09-18 CI 假红后**改为"进程内互斥锁串行 +
轮转 best-of-N"，每键延迟断言改用**中位数 + 最坏值天花板**；LSP 的单文件延迟
（didChange/completion/hover/stateAt/goals）与项目请求延迟改为 **best-of-3**——
它们跑在 130+ 用例并行的 lib 测试二进制里，单次采样必然偶发假红）。
**2026-09-19 补齐第二例**：项目级用例 `perf_project_did_open_and_keystroke` 当时仍是
**单次采样 + 300ms 预算**，在 2 核 CI runner 上实测 480ms 假红（同机单跑 17ms / 满负载
并行 86ms；pre-batch 与当前二进制同夹具对拍 best 26ms vs 25ms ⇒ 无产品回归）。
修法同族：按键延迟改**来回编辑 best-of-3**，三个 project 用例加 `PROJECT_PERF_LOCK`
（`tokio::sync::Mutex`）**互相串行**；阈值不动，修后满负载并行连跑 3 次 = 17/20/19ms。
**纪律升级为：所有性能哨兵（含项目级）默认「串行 + best-of-N」，新增哨兵按此写。**

**2026-09-19 第二次修正（同一用例，阈值标定）**：补上采样口径后它**仍然**在 CI 红
（`35413113457`：best-of-3 + 串行下 336ms > 300ms）。这次不再怀疑代码，而是量**分布**
——同一份代码在托管 runner 上的四次实测：`85ms`（0.58.0，单次）/`134ms`/`153ms`（0.59.0，
best-of-3）/`336ms`（红那次），本地（M 系 mac）17–20ms。**4× 的跨实例方差**说明 300ms
这个预算落在噪声带里（约 1/3 概率假红）。处置：预算改 **800ms**（对最慢一次实测 2.4×
余量，而哨兵要抓的量级回归是秒级 ⇒ 判别力不变），并在用例注释里写死这组数字。
教训：**阈值必须按"最慢受支持 runner 的实测分布"标定**，不能用"本地数字 × 一个感觉系数"；
同一条先做采样口径、再做二进制对拍，两步都排除掉再谈放宽阈值。
③ 比较台账数字先看是否落在 ±25% 内，超出再复测，别拿单次差异下结论；
④ **优先比 `best_ms`**：串行口径下多数指标两次记录相差 ≤17%，但 `worst_ms`
（5 次取最大）这类 max 统计量能差 45%——`keystroke_recompile_closure` 因此同时
记 `best_ms`（对比用）与 `worst_ms`（哨兵用的保守上界）。

**`query` 与 `check`/`build` 共用闭包缓存（2026-09-18 修复）**：三条命令都走
`crates/cli/src/project_cache.rs`（键 = `ProjectPlan::digest(options)`）。另外
`QueryDoc::check()` 原先会**再编译一遍**（项目模式下等于整个闭包重编译两次），现在
直接复用 `set_text` 存下的 `CompileOutput`。实测 3×12 项目：`query check` 冷
49→25ms、热 37→3.4ms（台账 `docs/perf/ledger.jsonl` 同轮记录）；`--text` 的中间态
仍不缓存（磁盘上没有对应源码，摘要会失真）。

结论（串行口径）：**教学规模（2–5 个模块、每模块 ~12 条声明）一次按键 14–24ms，
编辑器完全无感**；4×20 的"大项目"约 33–39ms（并行口径下曾被放大到 ~0.1s，
见上「噪声地板」）。项目模式没有跨模块增量——
真要优化，方向是"按模块复用已查环境"（设计 §1.3 已明确 v1 不做，见 P7）。
