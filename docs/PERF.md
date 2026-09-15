# 性能测试与基线（I13-S5c）

性能是本项目的生命线（REQUIREMENTS §9 四十四）。本文档描述性能测试的
三层结构、阈值设计原则与当前基线；**每次 push 都会在 CI 上例行执行**，
回归即红，且每版留档（`perf-report` artifact，带版本 + commit SHA）。

## 三层结构

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
