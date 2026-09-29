# 真课程耗时剖面（2026-09-29）—— **核心原因已定位**

> **口径**：`courses/set-theory`（42 入口 / 7451 行 / **470 条声明**）·
> **release**（`cargo build --release -p sokonanoda-cli`）· **冷缓存**（全局 + 模块根两处都清）·
> 同机（Apple Silicon，10 核）· 方差 **±2%** ⇒ **<5% 的差不许当结论**。
> 量具：`scripts/profile-course.sh`（一条命令出本文件）· 埋点：`SOKO_STAGE_STATS=1`、
> `SOKO_DECL_PROFILE=1`（`SOKO_DECL_PROFILE_MS` 阈值，仿 Lean `profiler.threshold`）。

## 1. 结论（一句话）

**218.8s 里约 88% 是 `judge` 环节**，而 `judge` 的成本**随声明在文件里的序号线性增长**
（O(N²) 前缀重跑）——**这就是比 Lean 慢一个数量级的核心原因**，不是并行度、不是模块重复编译。

## 2. 按阶段（绝对秒数）

| 阶段 | 秒 | 占 218.8s |
|---|---|---|
| **judge 合计**（`judge_ms`） | **148.4–158.9** | **68%**（含 `judge_infer` + `judge_terms`） |
| `by`（tactic 引擎，含在 judge 内） | 190.2（`by_total_ms`） | — |
| 其余（parse / elab / kernel check / 报告） | ≈60–70 | ≈30% |

**决定性实验**（`SOKO_NO_JUDGE=1`，**测量专用**开关，绝不进判定路径）：

| 配置 | 墙钟 | `passes` | `judge_ms` | `compiled/failed` |
|---|---|---|---|---|
| 正常 | **218.8 / 221.0 / 224.1 / 228.4**（4 次） | 4126 | 148.4 | 42/0 |
| 只跳 `judge_terms`（`by` 判定） | 221.7 | 4126 | 150.6 | 42/0 |
| **两个都跳** | **26.8** | **387** | 19.4 | 1/41 ✗（预期：跳了判定必然编不过） |

⇒ **`judge` ≈ 218.8 − 26.8 ≈ 192s ≈ 88%**（26.8s 那次已失去判定语义，只作上界读数）。
⇒ 只跳 `judge_terms` 只有 **3%** ⇒ **`judge_infer` 才是大头**，不是 `by`。

## 3. 逐声明事件（`SOKO_DECL_PROFILE=1`）

真课程一次冷编 **256160** 条逐声明事件，分成两半：

| 来源 | 事件数 | 累计秒 |
|---|---|---|
| **文件自己的声明**（`module` 非空） | **2647** | 209.1s |
| **judge 合成的 pass**（`module` 为空） | **253513** | 251.8s |

⇒ **judge 合成了 253513 次 pass，是文件自身声明数的 95.8 倍** ✗
（两个数都 >218.8s 是因为它们按**线程内嵌套计时**累加，有重叠 —— 只看量级）。

## 4. 三个假设的判定（用户 09:09 给的）

### 假设 A（**成立** ✓）：每条声明的成本随它在文件里的序号增长 = O(N²) 前缀重跑

判据：**逐声明的 `ms` vs 它的 `index`**（同一文件内），全分辨率跑（阈值 0）：

| 模块 | n | r(index, ms) | 首条 ms | 末条 ms |
|---|---|---|---|---|
| `units.solutions.unit05-solution` | 10 | **+0.95** | ~0 | **1751.7** |
| `units.solutions.unit11-solution` | 8 | **+0.95** | ~0 | **1247.8** |
| `units.solutions.unit04-solution` | 10 | **+0.89** | ~0 | **1328.5** |
| `units.solutions.unit09-solution` | 17 | +0.83 | ~0 | 2161.5 |
| `units.solutions.unit12-solution` | 16 | +0.78 | ~0 | **4680.4** |
| `units.solutions.unit08-solution` | 34 | +0.59 | ~0 | 2322.3 |

* **41 个模块里 20 个 `r > 0.5`，0 个 `r < −0.5`**（均值 **+0.44**）；
* **末条 / 首条** 的比值普遍 **10³–10⁵×**（`unit12` 最贵的单条 **4680ms**）。

**机理**（代码位置）：`judge_infer`（`crates/front/src/judge.rs:932`）把
`prefix_src`（**整段前缀**）+ 合成声明交给**完整流水线**再跑一遍；
缓存键 `judge_cache_key(&[extra_prefix, prefix_src, …])` **含整段前缀的哈希**
⇒ 前缀随序号线性变长 ⇒ 后段声明每次都 miss ⇒ **前缀从零重跑** ⇒ O(N²) ✓

### 假设 B（**成立** ✓）：少数重声明吃掉大头

按**总耗时**排 Top 10（阈值 50ms 的那次，1084 条超阈值）：

| 声明 | 文件 | 序号 | ms |
|---|---|---|---|
| `project_chain` | `unit12-solution` | #14/16 | **7447.5** |
| `project_chain_cardinal` | `unit12-solution` | #15/16 | **4650.1** |
| `nearStep_counterexample` | `unit06-solution` | #25/27 | **4198.9** |
| `image_preimage_image_eq_preimage` | `unit08-solution` | #31/34 | 3692.6 |
| `demo_injective_unpack` | `unit12-synthesis` | #8/19 | 3682.2 |

⇒ **重声明 + 它们在文件里的位置靠后** ⇒ 与假设 A **同一个病**（不是独立的第二因）。

### 假设 C（**不成立** ✗）：固定开销 × 42 入口不是大头

判据：跳掉 judge 后墙钟 **26.8s** ⇒ 那是"42 入口的全部非 judge 工作（含 parse/elab/kernel/报告）"，
**上界只有 26.8s**（占 12%）⇒ **每入口固定开销 ≤ 0.64s**，**不是** 218.8s 的主要成分 ✓
⇒ "复用进程 / 减少每入口开销"**不是**该攻的方向（**数据：26.8s / 42 = 0.64s 上界**）。

## 5. 对用户 09:22 三个问题的回答

### ① `judge` 在 `build <dir>` 上是不是必需的？

**是必需的，但"必需"的只是它的一半**：

* **必需的**：`by` 块里每条 tactic 的**判定**（`judge_terms`）—— 它就是"这条证明对不对"的裁判；
  以及 `judge_infer` 在 **elaboration 里**当"类型/宇宙文本"的来源（`elab.rs:1363/1396/2089/…`）。
  去掉它们 = 不判卷 = **改判定**（红线 ✗，实测 `failed:41`）。
* **不必需的**：**建议 / goal 视图 / 子洞探针**（`suggest.rs`、`goals.rs` 的
  `probe_sub_goal_types`）—— 那是**交互式**功能，`build` 路径不需要它。

⚠ 但**当前的大头恰恰是 `judge_infer`**（`elab` 路径），**不是** `suggest` ✗ ——
所以"把建议改成懒触发"**省不到 88% 里的大部分**（这条要如实说，别让人以为改懒加载就行了）。

### ② "不跑 judge 的编译" = 多少？

**26.8s**（上界；已失去判定语义）。⇒ **用户体感的天花板是"218.8s → 26.8s"**，
但那**不是**可交付的目标（判定不能不要）；可交付的是"**让 judge 别重跑前缀**"。

### ③ 对照 Lean 的分工

| | Lean 4 | 我们 |
|---|---|---|
| 批量构建 | elaborate + 内核检查 + **写 `.olean`** | elaborate + **每步 judge 重跑整段前缀** + 报告 |
| 交互式建议 | `exact?`/`apply?` 是**交互策略**，**不进构建** | `suggest` 在 LSP 路径（build 里基本不跑）✓ |

⇒ **差别不在"要不要判定"，而在"判定怎么实现"**：Lean 在**一个** elaborator 里用元变量逐步细化、
最后交内核检查**一次**；我们**每步都合成一份完整文件、把整段前缀从零再跑一遍** ✗。
**这就是 O(N²) 的来源，也是"比 Lean 慢一个数量级"的核心原因** ✓

### ④ 拆分方案（**先给方案 + 数字，不动手**）

**正解 = K1-b（前缀环境复用）**，不是"把 judge 拆掉"：

1. **内核加一处显式环境入口**（`EnvBuilder` 已有 `with_env`；需要的是"**以上次前缀的环境为起点**"）；
2. **前端把上次前缀的环境喂回来** ⇒ `judge_infer` 不再从零 parse+elab 整段前缀；
3. 收益估算（**依据**）：judge ≈ **192s**，其中"重跑前缀"是主体（253513 次合成 pass /
   2647 条真实声明）；**假设前缀重跑成本归零** ⇒ 下界 **26.8s**，
   保守取"只省前缀、判定本身照做"⇒ 目标区间 **60–100s**（**这是估算，不是实测**，
   要拿 `SOKO_NO_JUDGE` 的分层数据 + K1-b 原型才定得下来）。

**不推荐**："把 judge 从 build 里拆掉" ✗ —— 判定是 build 的核心语义，
拆掉就是**不判卷**（实测 `failed:41`）。**建议/goal 懒触发**可以做（省 LSP 侧），
但对 `build` 的 88% **没有帮助** ✗。

## 6. 与 mathlib4 的归一化（分母是量级估计，只当信号）

| | 每声明秒数 | 每千行秒数 |
|---|---|---|
| 本项目（release 实测） | **0.466 s/条** | **29.4 s/千行** |
| mathlib4（20–30 分钟 / 10 万条 / 150 万行） | 0.012–0.018 | 0.8–1.2 |
| 倍数 | **26–39×** | **25–37×** |

⇒ 与 Lean 的差**不是**并行度（我们本来就单线程串行 42 入口），
**是单条声明的实现方式**（每次重跑整段前缀 vs Lean 的增量 elaborator）✓

## 7. 业界的量具做法（核实过的出处，抄进 `scripts/profile-course.sh`）

| 做法 | 出处 |
|---|---|
| `trace.profiler.threshold`（默认 **100ms**，只报超阈值的）· `profiler.threshold` | [Lean Profiling (VCA EPFL)](https://vca-epfl.github.io/wiki/lean-profiling/) |
| **逐文件单独跑**避免并行开销污染（`find … -exec lake env time lean {} \;`） | [Zulip: profiling a project](https://leanprover-community.github.io/archive/stream/270676-lean4/topic/profiling.20a.20project.html#500637969) |
| `speed.lean-lang.org` = CI 长期追踪每模块耗时 | 同上（[temci-config.run.yml](https://github.com/leanprover-community/mathlib4/blob/master/scripts/bench/temci-config.run.yml)） |

**已照抄的**：阈值过滤（`SOKO_DECL_PROFILE_MS`，默认 100ms，`=0` 全打）；
**待抄的**：① 逐入口独立进程量（现在整轮跑）；② CI 台账跨 commit 对比。

## 8. ⚠ 本次实测推翻/更正的三条旧说法

1. **`docs/PERF.md` 的「judge 占 68%」低估了**：`judge_ms` 是**线程内嵌套累加**，
   而"跳掉 judge 的墙钟差"给的是 **≈88%**（218.8 → 26.8）⇒ **以墙钟差为准** ✓
2. **「4.14× 重复编译」不是主因**：理论工作量降 11.7×（5166 → 470 次 elaboration）
   反而**慢 15%**（252.7 vs 218.8）⇒ 重复编的是**便宜的库声明**，时间在**重解答文件本身** ✗
3. **「跨机比数字无效」的归因错了**：同机同接线 **release 218.8s vs debug 260.7s（差 3%）**；
   让旧数字不可比的是**接线版本**（625s 那次是"41/42 快速失败 + 并集库层"）✓
