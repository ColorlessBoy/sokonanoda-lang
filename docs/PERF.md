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
