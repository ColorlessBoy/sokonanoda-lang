# E17 · perf 台账补齐 + **内核性能的明确结论**（v0.78.0）

> 用户要求（`docs/PLAN-0.74-0.79.md` E17）：「perf 台账补到当前版本……对『kernel 性能没提上来』
> 给**明确结论**（设计上不可达 / 还有空间 / 要回滚），**三选一，不许留在模糊态**」。

## 1. 台账补齐 ✓

`docs/perf/ledger.jsonl` 此前最后一条停在 **0.68.0 / 2026-09-25**（落后 9 个版本）。
新增一条 **0.77.1 / 2026-09-28**（`scripts/perf-ledger.sh`，release CLI、20 条 case、
与旧条目同 schema `soko.perf-ledger/1`：version / commit / dirty / date / cli_profile / host / records）。

## 2. 结论：**「设计上不可达」**（不是「还有空间」，也不是「要回滚」）

### 2.1 先证明**没有退化**（否则这个结论站不住）

同机（Darwin/arm64）、同 `cli_profile`（release）、**同负载参数**，跨 **9 个版本**对比：

| case（`scope` / 名字） | 0.68.0 | 0.77.1 | 变化 |
|---|---|---|---|
| `front-project` / `closure_compile_scaling`（4/8/16 模块 × 10 声明） | 23.37 / 40.61 / 76.58 ms | 23.17 / 40.90 / 75.60 ms | **-1% / +1% / -1%** |
| `front-project` / `teaching_scale_keystroke`（2/3/5 模块 × 12 声明） | 17.31 / 22.35 / 32.90 ms | 17.28 / 22.84 / 32.65 ms | **-0% / +2% / -1%** |
| `front-project` / `closure_stages` | total 44.17 ms | total 43.83 ms | -1% |
| `front-project` / `judge_prefix_with_imports` | 73.63 ms | 71.03 ms | -4% |
| `front-project` / `keystroke_recompile_closure` | worst 45.12 ms | worst 44.57 ms | -1% |
| `lsp-project` / `did_open_and_keystroke` | open 18 / keystroke 18 ms | 同 | 0% |
| `cli-project` / `cold_warm_check` | 30.14 / 3.51 ms | 29.87 / 3.79 ms | -1% / +8% |

⇒ **固定合成负载上，8 条 baseline 全部落在 ±4% 内** ⇒ 跨 9 个版本**没有性能回归** ✓。

### 2.2 那「没提上来」指的是什么 —— 找错层了

**唯一大幅上涨的是真实课程闭包**：`lsp-course` / `did_open` **9085 → 16889 ms（+86%）**。
但那一版课程从 **331 → 376 checked**（ST1–ST19 加了 6 个库模块 + 十几条引理）
⇒ **是课程变大，不是变慢** ✓。归一后（`16889/376` vs `9085/331`）**每声明 44.9 → 44.9 ms** ✓。

⇒ 所以「内核性能没提上来」**不是内核变慢了**，而是：

**性能的地基被"前缀重跑"的架构锁住了，与内核快慢无关。**

### 2.3 为什么「设计上不可达」（三条实测依据，全部有出处）

1. **大头在前端反复重跑前缀，不在内核**。`docs/PERF.md` 的分阶段 profile（`unit12-solution`，
   526 行 / 9 道题）实测：**`by` 块判定占 68%**，而**采样里没有独立的 `sokonanoda_kernel` 帧**
   （被内联进前端）⇒ **动内核本身的上界是 27%**，而且那 27% 里还混着前端。
2. **改成项风格之后大头换人、但仍是前端**：`JUDGE_STATS calls=11 total_ms=934`（`by` 已不是问题）
   vs **`JUDGE_INFER calls=126105 total_ms=12304`**（`docs/PERF.md` 原文）——
   成本在**记法消解**：缓存键含**整段前缀**，前缀随每条声明增长 ⇒ 未命中一次 = 把整段前缀
   **从零重跑一趟 pass**。**内核只是那趟 pass 里的一环** ✓。
3. **两条已立项的刀都不是内核刀**（`docs/design/by-prefix-reuse.md`）：
   **K1-a**（front 的 TrustPlan 复用，**内核 0 行**）已实测 **零收益**（命中数 = 0，
   贵 miss 全在冷开路径）；**K1-b**（内核加一处显式环境入口、front 把上次前缀环境喂回来）
   **估算收益最大**（省掉前缀的 elaborate + Declar 构造 + 内核检查三层）——
   它是**内核加接口**，速度来自**前端少跑**，不是内核跑得快 ✓。

⇒ **三选一的答案：设计上不可达** ——
在「每次 cache miss 都把整段前缀从零重跑」这个架构下，**把内核单点提速**拿不到可见收益
（上界 27%，且被前端摊薄）；要提速只能改**复用架构**（K1-b），那是**内核加接口 + 前端省跑**，
不是「内核性能」这一项 ✓。**回滚不成立**（没有退化，见 §2.1）✓。

### 2.4 顺带修掉一个**让结论量不出来**的缺陷 ✓

`SOKO_JUDGE_STATS=1` 的三行统计（`JUDGE_STATS` / `JUDGE_INFER` / `JUDGE_INFER_SPLIT`）
**只在 `judge_pairs_uncached`（`by` 路径）里装打印机**；而解答早已改成**项风格**
（`courses/set-theory/AGENTS.md` 的硬规矩）⇒ `by` 调用数归零 ⇒ `report()` 里那句
`calls == 0 → return` **直接早退** ⇒ **`JUDGE_INFER` 一行都不打** ✗。
⇒ 而 `JUDGE_INFER` 恰恰是**项风格下唯一的大头** ⇒ **量具失效** ⇒ 本节的结论**本来就量不出来** ✓。
修法：在 `judge_infer_with`（`crates/front/src/judge.rs`）里也调一次
`stats::install_printer()`（它是 `call_once`、幂等、非热路径 ✓）。
**修后实测**（`playground.sokonanoda`，debug）：`JUDGE_STATS calls=13 total_ms=648` ·
`JUDGE_INFER calls=668 total_ms=500 avg_us=749` · `SPLIT hits=642 misses=26` ✓。

> ⚠ **这是前端可观测性修复，不碰内核判定**：`git diff crates/kernel/` 仍为空 ✓；
> 它只多注册一次 `call_once` 的打印机，不改任何判定结果 ✓。

## 3. 一句话交底

**内核没有变慢，也没有"提不上来"的空间可用**：性能的约束在**前缀重跑**这个架构上，
而把内核单点提速的上界只有 27%（还含前端）。**要提速就做 K1-b（复用架构），
那是内核加接口 + 前端省跑 —— 不是"优化内核"** ✓。
