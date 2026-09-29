# P1-a 附录：量具 / 对账清单 / 实测读数（G-68 判定就地查环境）

> 正文与交接见 [`docs/HANDOVER-slice1.md`](../HANDOVER-slice1.md)。
> 这份是**动手用的细节**：0 号动作答案 · 接口对账清单 · 量具读数 · 决定性分布。
# 附：P1-a 的 0 号动作答案（2026-09-29 17:5x，**含权威计数**）

## 0. 🔢 权威计数（本轮实测，`SOKO_STAGE_STATS=1`，release · 冷缓存 · 1 job）

| 读数 | 值 | 含义（已核对代码） |
|---|---|---|
| **墙钟** | **217.27s** | 全课 `build --json` |
| `passes` | **4126** | `check::stage_stats::PASSES` = **`run_pass_with` 的调用次数**（`check/mod.rs:928`）|
| `doc_passes` | **266** | `VIA_CHECK_DOCUMENT` = 真文档趟数 |
| ⇒ **judge 子趟** | **≈ 3860** | `4126 − 266` ⇒ **每次文档 pass 要跑 ≈ 14.5 趟 judge 前缀** |
| `judge_ms` | **147.8s** | judge 累计（**含**在下面的 `by_total_ms` 里）|
| `by_calls` / `by_total_ms` | **69085** / **189.7s** | `by` 引擎（tactic）累计 |
| `pass_total_ms` | **596.6s** | **所有 pass 累计**（4126 趟 × 平均 145ms）|
| `hits`/`misses` | 66683 / 266 | judge 缓存 |

**⇒ 三个结论（修正旧说法）**：

1. **judge 占墙钟 ≈ 147.8 / 217.3 ≈ 68%**（若 judge 时间与别的阶段重叠少）。
   ⚠ 旧的「88%」出自**另一次口径**（219.3s→26.8s 的跳法比较），**不要直接引用** ✗；
2. ⚠ **旧的「合成 pass 253513 次」与今天的 `passes=4126` 不是同一个计数器** ——
   前者是 `judge::stats::PASSES`（judge 内部子趟），量级 253513 ≈ 2647 声明 × 96
   ⇒ **judge 内部那套放大是真的**，但它**不等于** `STAGE_STATS` 的 `passes`；
3. **`pass_total_ms` 596.6s ≫ 墙钟 217s** ⇒ 有并发/重叠，**逐项相减是错的** ✗。

## 1. P1-a 的收益上界（**先估，再动手** —— 值守 17:46 的要求）

* **可兑现上界 = `judge_ms` = 147.8s**（那是"从零重跑前缀"的全部时间）；
* 若就地查环境能消掉 **50–70%** 的 judge ⇒ 省 **74–103s** ⇒ 墙钟 **217s → 114–143s**
  （**1.5–1.9×**，**不是** 4×）；
* ⚠ 但「50–70%」**目前是猜的** ⇒ **P1-a 的第一件事就是把这个比例量出来**（见 §2 建议）：
  在 `judge_infer` 里按"答案来源"分类计数（`Env` 表查到 / 必须重跑前缀），
  **只加计数器、不改判定**（与 `SOKO_NO_JUDGE` 同性质，且**只许量成本**）。

## 2. 0 号动作的答案：**为什么落了零件却没接通 judge？—— 两者都有，且障碍是真障碍**

**（a）被切片 1 岔开了** —— 时间线（commit 为证）：`c625cffc`（`infer_type_text_at`，15:01）·
`a3545f03`（`with_env_scope`，15:19）落完之后，**立即转去切片 1 接线**
（`3f61ad95`…`9543405a` 全是切片 1）⇒ **judge 那条线一次都没试过** ✓。

**（b）接通前有两个**真**新障碍**（今天已逐条查实，不是猜）：

* **障碍 1（主）**：`judge_infer` 收的是 **`term: &str`（源码文本）**，
  而 `infer_type_text_at` 要 **`ExprPtr`（已 elaborate 的内核项）** ⇒
  **调用方手里没有那个 `ExprPtr`** —— 10 个调用点（`elab.rs:1363/1396/2089/3512/4212/4658/4700/4702/5136`）
  传的是 `render_expr(operand)` **渲染出来的字符串**。
  ⇒ **要么**先把该文本 elaborate 成 `ExprPtr`（需要"在同一个 arena / 同一份 `ExportFile` 上
  由源 `Expr` 造项"的能力 —— 即 **§19.4 的候选 A / K-2**，**尚未实现**），
  **要么**绕过 elaborate：把"**裸常量名的类型**"这类**最常见**的查询做成**直接查表**
  （`Env::get_declar` 给出 `Declar` ⇒ 取它的类型 ⇒ 打印）。
* **障碍 2**：`with_env_scope` 只能在 `elab_expr` **已经可变借走** `EnvBuilder` 的调用栈里
  重入 ⇒ 需要"**重入安全**"的调用形状（把 builder 临时还回来 / 传闭包而不是借用）——
  **尚未验证**。

## 3. ⇒ P1-a 从「**补零件**」开始，不是从「接线」开始

**第一步（最小，且能一次量出上界）**：在 `judge_infer` 里**只加分类计数**：
每次调用时判断"**这个查询能不能用裸常量查表答**"（源文本是 `Name` 或 `Name ...` 头），
分别计数 + 分别累计耗时 ⇒ **得到"可消掉的比例"** ⇒ 再决定 P1-a 怎么切。
**只加计数、不改判定** ⇒ 零语义风险 ✓。

**第二步**：按第一步的结果，挑**占比最大的那一类**做就地查表（`Env::get_declar` 或
`infer_type_text_at`），**源码重跑保留为开关**（可 A/B、回退便宜）。

**判据（每一步都要）**：`--json` **逐字节不变** · 报出 `passes`/`doc_passes`/`judge_ms`
**三个数** · 反向判据（前缀真变必须重算）。

---

# 附二：P1-a **接口对账清单**（第 4 次动手之前必须有；值守 17:5x 要求）

> 今天三次走空**全是接口/顺序问题**，不是算法问题 ⇒ 这份表逐点一行、附一句话风险。

## A. 签名会动到哪里

| # | 对象 | 现状 | 要变成 | 风险 |
|---|---|---|---|---|
| A1 | `judge_infer`（`judge.rs` 入口）| `(prefix_src: &str, options, binders, term: &str) -> Result<String,_>` | 多收一个**环境句柄**（见 C）| **签名改动面 = 10 个调用点**；漏一个 = 编译错（好抓）|
| A2 | `judge_infer_with`（`judge.rs:949`）| 同上 + `extra_prefix` | 同上；**保留原签名**，新加 `*_with_env` 兄弟函数 | 保留原函数 ⇒ **回退零成本** ✓ |
| A3 | `judge.rs` 的缓存（键含**整段前缀哈希**，`:932`）| `HashMap<前缀哈希, 结果>` | **就地路径不查这个缓存**（它本身就是免前缀的）| 两条路**结果必须一致** ⇒ 见 D（影子核对）|

## B. 10 个调用点逐点（`crates/front/src/compile/elab.rs`）

| 行 | 函数 | 查的是什么 | 有源 `Expr`？ | 能否"裸常量查表" |
|---|---|---|---|---|
| `1363` | `notation_prefix_args` | `head` 的类型 | ✓ `head` | 部分（`head` 常是常量/应用头）|
| `1396` | 同上 | 上一步 `ty_text` 的 sort | ✗（只有文本）| **否**（输入已是文本）|
| `2089` | `infer_type_text` | `operand` 的类型 | ✓ `operand` | **是**（最常见一类）|
| `3512` | `let` 缺标注 | `val` 的类型 | ✓ `val` | 部分 |
| `4212` | λ binder 补类型 | `args[n]` 的类型 | ✓ `args[n]` | 部分 |
| `4658` | `match` scrutinee | `scrutinee` 的类型 | ✓ `scrutinee` | 部分 |
| **`4700`** | **`infer_expected_level`** | `R` 的 **sort** | ✓ `expected_src` | **是**（`sort_text_level` 只要 `Sort(n)` 文本）|
| `4702` | 同上（另一分支）| 同上 | ✓ | **是** |
| `5136` | `field_sort_via_kernel` | `src_ty` 是否 `Prop` | ✓ `src_ty` | **是** |
| — | `judge_pairs`（`by` 引擎）| — | ✗ | 不在本次范围 |

⚠ **`1396` 那处输入已是文本**（`ty_text` 来自上一趟）⇒ **它天生不适合就地路径**
⇒ 必须保留 `judge_infer` 的原样调用 ✓（这正是"**保留源码重跑**"的必要性）。

## C. 环境句柄从哪来（**这是真障碍**）

| # | 问题 | 现状 | 风险 |
|---|---|---|---|
| C1 | `judge_infer` 的调用点**没有** `&mut EnvBuilder` | `elab_expr` 已经可变借走它 | 需**重入安全**的形状（闭包传参 / 临时还回）—— **尚未验证** |
| C2 | `infer_type_text_at` 要 `ExprPtr` | 调用点只有**源 `Expr`** + 渲染文本 | ⇒ 要么先 elaborate（**需要 §19.4 候选 A / K-2，未实现**），要么只做"**裸常量查表**"（`Env::get_declar`）✓ |
| C3 | `with_env_scope`（`builder.rs:138`）| 已上 main、零调用点 | 借出 `(&Env, &mut EnvBuilder)`；⚠ 回调期间两张表被 `mem::take` 走 ⇒ **回调里不许依赖 `declars`/`notations`** |

## D. 哪里会破 `--json` 红线（**逐条**）

| # | 破法 | 防法 |
|---|---|---|
| D1 | 就地路径与重跑路径**结果不同**（判定语义变）| **影子核对**：先两条都跑、**比对文本**，不一致就**报错并回退**（不静默）|
| D2 | `binders` 语义不同（`judge_infer` 把 binder 包成 `fun … =>`，`infer_type_text_at` 收**闭项**）| 就地路径**必须自己**处理 binder（包 λ 或忽略）—— **逐点核对** |
| D3 | 缓存命中/未命中改变**事件顺序或计数** | 就地路径**不写**原缓存；`passes`/`doc_passes` 会变 ⇒ **只许降、不许变语义**；`--json` 逐字节兜底 |
| D4 | 错误路径不同（`judge_infer` 失败时调用点 `?`/`ok()?` 的行为）| 就地路径**答不出就返回 `None`** ⇒ 调用方**自动回退**（与 `EnvProvider::infer_type_text` 的 `None` 约定一致 ✓）|

## E. ③ 开关形状（现在定死，收口时不临时塞）

* **env 名**：`SOKO_JUDGE_INPLACE`（模式串，非布尔）：
  * 未设 / `off`（**默认**）⇒ 只走源码重跑（**今天的行为，逐字节不变**）✓；
  * `shadow` ⇒ **两条都跑**，比对，**不一致时报错**（不静默）⇒ **上线前的核对档**；
  * `on` ⇒ 就地优先、答不出回退（判定语义必须与 `off` 逐字节一致）。
* **判定点（唯一的那个）**：**只**在 `infer_expected_level`（`elab.rs:4690`，含 `4700`/`4702`）
  + 可选 `field_sort_via_kernel`（`5136`）—— **P1-a 只做这一组**，铺开是 P1-b ✗。
* **默认值**：`off`；`on` 只在 D 的影子核对**全绿**之后才考虑改默认 ⇒ **P1-a 不改默认** ✓。

---

# 附三：🔴 **P1-a 的量具结果 —— 原定靶子是错的**（2026-09-29 18:1x 实测）

## 1. 量具（`SOKO_JUDGE_CLASSIFY=1`，只加计数、**不改判定**）

`crates/front/src/judge.rs` 的 `judge_infer_cached` 里，把每次**未命中**按
「是否裸常量」×「`term` 长度桶」记 (次数, 耗时)；另一处把 `classify()` 汇总打印。

## 2. 实测（release · 冷缓存 · 1 job · 墙钟 **216.9s**）

```
JUDGE_CLASSIFY calls=1085522 bare=44759 resolvable=43658
                all_miss=3759  all_miss_ms=235671  all_miss_share=1.595
                bare_miss=113  bare_miss_ms=10418   bare_miss_share=0.070
JUDGE_MISS_BUCKET bare=0 len=<16   n=1795 ms=65410  share=0.443   （每次 36ms）
JUDGE_MISS_BUCKET bare=0 len=<48   n= 899 ms=40758  share=0.276   （每次 45ms）
JUDGE_MISS_BUCKET bare=0 len=<160  n= 778 ms=55474  share=0.375   （每次 71ms）
JUDGE_MISS_BUCKET bare=0 len=>=160 n= 174 ms=63609  share=0.430   （每次 **366ms**）
JUDGE_MISS_BUCKET bare=1 len=<16   n= 113 ms=10418  share=0.070   （每次 92ms）
STAGE_STATS  passes=4126 judge_ms=147791 doc_passes=266
```

## 3. ⇒ 三条结论（**P1-a 原定靶子判死**）

1. 🔴 **「裸常量就地查表」的上界 = 10.4s ≈ 墙钟 4.8%** ⇒ **远低于 5% 判据** ✗
   ⇒ 它**不是**值得做的优化（做了也量不出来）。**P1-a 按原样做 = 白做**。
2. **未命中的时间 93% 花在"非裸常量"（233.9s − 10.4s）上** ⇒ 靶子应重定为
   **"未命中的复合项"**，不是"裸常量"。
3. **最贵的一类 = 长项**：`len>=160` 只有 **174 次**却吃掉 **63.6s**
   （**366ms/次**，是 `len<16` 那类的 **10×**）⇒ **"少数长前缀"才是大户** ✓
   （与旧结论"最贵单条 4680ms"、以及 `misses` 里 247 次吃掉 6.2s 同向）。

## 4. 建议的新靶子顺序（**待值守/用户拍板**）

| 序 | 靶子 | 依据 | 预估上界 |
|---|---|---|---|
| **1** | **长 `term` 的未命中**（`len>=160`，174 次 / 63.6s）| 366ms/次，10× | ~60s ≈ 墙钟 **28%** |
| **2** | 中长（`<160`，778 次 / 55.5s）+ 短（1795 次 / 65.4s）| 每次 36–71ms | 各自 ~55–65s |
| ~~3~~ | ~~裸常量查表~~ | **10.4s ≈ 4.8% ⇒ 判死** ✗ | — |

**共同机理**（三类都是）：未命中 ⇒ **整段前缀重跑一趟 pass**，成本随前缀长度涨
⇒ 真正该改的是 **"让未命中不再重跑前缀"**（= §19.4 候选 A / K-2，或把前缀**增量**化），
而**不是**"把某些查询变成查表"。**这是一个比 P1-a 更本质的靶子。**

## 5. ⚠ 计数口径（别被 `all_miss_share=1.595` 骗）

`all_miss=3759` 与 `STAGE_STATS` 的 `misses=266` **不是同一口径**
（`share` 因此会 >1）。两者都真：分母来自不同计数器/不同统计点。
**只有同一 run 内的"桶间相对比"是可靠的** ✓（上面的结论只用相对比）。

---

# 附四：✅ **P1-a 的判据读数（结构计数，噪声免疫）** —— 与"靶子重定"

值守 18:2x 指出：**miss 耗时口径 233.9s > 墙钟 216.9s ⇒ 不能拿来下结论**（并发/嵌套重复计时）。
同意，且**已弃用该口径**。改用**结构计数**（确定性、不被并发污染）作判据：

## 1. 判据读数（release · 冷缓存 · 1 job）

```
墙钟              214.97s
JUDGE_PREFIX      runs=3759   bytes=174213583   bytes_per_run=46345
STAGE_STATS       passes=4126  judge_ms=146904  doc_passes=266  misses=266
```

**⇒ 一句话**：judge **重跑了 3759 趟前缀、累计解析 1.74 亿字节**（平均每趟 **46 KB**）。
这就是 O(N²) 的**直接读数** ✓（前缀随声明序号线性变长 ⇒ 总字节随 N² 涨）。

**为什么这个数可以作判据**：**确定性的**（同输入同输出）·
**不被并发重复计时污染**（不像 `miss_ms`）✓ ⇒ 与 `passes`/`by_calls` 同类，
符合"perf 判据一律用结构计数或比值"的硬规矩 ✓。

## 2. ⚠ 两个**作废**的口径（别再引用）

| 口径 | 问题 |
|---|---|
| `all_miss_ms=235.7s` / `bare_miss_share` | **超过墙钟** ⇒ 并发/嵌套重复计时 ✗ **作废** |
| `SOKO_NO_JUDGE=1` 的 26.17s | 实测 `compiled:1 failed:41` ⇒ judge 一关**41 个入口直接失败** ⇒ **不是"省下的时间"** ✗ **作废** |

## 3. ⇒ **靶子重定**（"裸常量查表"子目标判死，**主线不变**）

* **裸常量就地查表**：**不做它** ✗。**卡在哪**：`crates/front/src/judge.rs:1046`
  （`judge_infer_cached`）未命中 ⇒ `judge_infer_uncached`（`:1204`）⇒ `:1245` 处
  `synthesized_prefix` + `crate::parse_fragment(&src)` ⇒ **每次未命中都把整段前缀重新拼串 + 重新解析** ✗。
* **真正的靶子 = 那 3759 趟 / 1.74 亿字节的"前缀重跑"本身**
  ⇒ 即用户 §2 队列的**「judge 走 EnvView」**：让判定**不再重跑前缀**。
* **可兑现上界**：`judge_ms 146.9s / 墙钟 215.0s ≈ 68%`（**两个绝对量之比**，不是并发累加 ⇒ 可用 ✓）。
  若能消掉一半 ⇒ 省 **73s** ⇒ 墙钟 **≈142s（1.5×）**。

## 4. 下一棒（立刻接，不等）

**目标**：把 `judge_infer_uncached`（`judge.rs:1204`）的
「`synthesized_prefix` 拼串 + `parse_fragment` 整段重解析」
换成**前缀的增量部件**（已解析片段复用 / 已 elaborate 的环境查表）。

**两条路（动手前先做接口对账）**：
1. **片段级增量**：前缀 = "逐单元已解析片段" ⇒ 只接**新片段**，避免 46 KB × 3759 的重复解析
   （保守、不碰判定语义）；
2. **环境查表**：用已就位的 `with_env_scope`（`builder.rs:138`）+ `Env::get_declar`
   回答"某常量的类型/sort"—— ⚠ 仍受限于**只有源 `Expr`、没有 `ExprPtr`**。

**判据**：`JUDGE_PREFIX bytes` **必须显著下降**（结构计数 ✓）·
墙钟**改前/改后双数字**（同机同口径，<5% 差不许当结论）· `--json` 逐字节不变 ·
三层回归 · 反向判据（前缀/依赖真变必须重算）。

---

# 附五：🎯 **决定性分布 —— 3759 趟只对应 488 个不同前缀（7.7× 重复）**

`SOKO_INFER_TRACE=all` 全量跑一遍（release · 冷缓存 · 1 job · 全课），对 3759 行
`INFER_MISS` 做统计：

```
n = 3759 趟
前缀长度：min = 0 · **median = 41238** · max = 95097
**不同前缀数 = 488**          ⇒ 平均**每个前缀被重跑 7.7 次**
不同 (前缀, term) 对 = 2831     ⇒ 真"新查询"2831 个；其余 928 趟是**查询重复**
```

## ⇒ 两条可兑现的结论

1. **前缀高度重复**：3759 趟里只有 **488 个不同前缀** ⇒
   **同一段前缀被 parse + elaborate 了 7.7 遍**。而每遍的 median 是 **41 KB**
   （累计 **1.74 亿字节**）⇒ **纯重复劳动**，这是 O(N²) 的**可量化**部分 ✓。
2. **查询本身也重复**：2831 个不同 `(前缀, term)` vs 3759 趟 ⇒ **928 趟是同一查询**。

## ⇒ 下一棒（**按收益/风险排序**，动手前先接口对账）

| 序 | 做法 | 收益 | 风险 |
|---|---|---|---|
| **1** | **前缀解析结果 memo**：把 `parse_fragment(前缀)` 的产物按**前缀哈希**缓存，复用 7.7× | 省掉 ≈3271 次 **41 KB 解析** | **低** —— 解析是纯函数，产物只取决于前缀文本；**不碰判定语义** ✓ |
| **2** | 把 `compile_fol_with` 的**前缀部分 elaborate 结果**也 memo（更强，但要缓存 `EnvBuilder` 状态 ⇒ 见下） | 省掉重复 elaborate | **高** —— `EnvBuilder` 不是 `Clone`；需要"快照/恢复"或 per-prefix builder 池 |
| **3** | 查询级去重（928 趟） | 小 | 低 |

**⚠ 第 2 条的真障碍（已勘明，别再重勘）**：
`judge_infer_uncached`（`judge.rs:1204`）走 `compile_fol_with`（`check/mod.rs:286`）
⇒ `run(&[SourceUnit::single(...)])` ⇒ **每次从零造 `EnvBuilder`**（`check/mod.rs:855`）
⇒ 没有任何跨调用复用。要复用就必须能**保存/恢复一个已 elaborate 的前缀环境**——
`with_env_scope`（`builder.rs:138`）能**借出**它，但**借不出 `&mut` 的所有权** ⇒
需要"**per-前缀的 builder 池**"（`HashMap<前缀哈希, EnvBuilder>`）或让 judge 接收
调用方（`elab.rs`）**已经有的那个 builder** ——
后者正是"judge 走 EnvView"的最终形态，但受限于
**调用点只有源 `Expr`、没有 `ExprPtr`**（附二 C2）。

**⇒ 建议从第 1 条开始**：收益明确（3271 次 × 41 KB 解析）、风险最低（纯函数缓存）、
**且不依赖任何未接线的新零件** ✓。

## 判据（第 1 条做完就能量）

`JUDGE_PREFIX bytes` **必须显著下降**（结构计数 ✓ —— 解析与 elaborate 是两笔账，
本条只降"解析"那笔 ⇒ **另需一个 `PARSE_BYTES` 计数器**把两笔分开）·
墙钟**改前/改后双数字**（同机同口径，<5% 差不许当结论）· `--json` 逐字节不变 ·
三层回归 · 反向判据（前缀真变必须重算）。

---

# 附六：🔴 **`JUDGE_INFER` 的权威三段账 —— 未命中 ≈ 222.6s**

`SOKO_JUDGE_STATS=1` + `SOKO_STAGE_STATS=1`（release · 冷缓存 · 1 job · 全课）：

```
JUDGE_INFER       calls=1085522  total_ms=247402  avg_us=227  fails=9353
JUDGE_INFER_SPLIT hits=1081763   misses=3759  key_ms=12127  hit_ms=12683
JUDGE_PREFIX      runs=3759      bytes=174213583  bytes_per_run=46345
STAGE_STATS       passes=4126    judge_ms=146904
```

## 结论

* **`JUDGE_INFER total_ms = 247.4s`**（judge 那本账的最大口径）；
* `hits` + `key_ms` + `hit_ms` ≈ **24.8s** ⇒ **未命中那段 ≈ 222.6s（占 90%）** ✓
  —— 与"3759 趟 × 平均 59ms"吻合（`222.6s / 3759 ≈ 59ms`）；
* ⚠ **`JUDGE_INFER_SPLIT` 的三个数是互斥分段，但实测三者之和远小于 total**
  ⇒ **未命中那段的计时没被完整捕获** ⇒ **不要直接引用 miss_ms**；
  要"未命中总耗时"就**反推**：`total_ms − key_ms − hit_ms ≈ 222.6s` ✓；
* **`key_ms = 12.1s`** ⇒ 哈希整段前缀本身只占 5% ⇒ **打哈希没用**（与旧结论同向 ✓）。

## ⇒ 可兑现上界（判据用）

**省掉那 3759 趟前缀重跑 ⇒ 上界 ≈ 222.6s**（≈ 墙钟 215s 的 100%+ —— 说明该口径
与墙钟不是同一维度，**只作"上限"用，不作"能省多少"**；真正的墙钟双数字在
实现之后同机同口径量）。

**⇒ 下一步（唯一）：让前缀不再重跑** —— 即"**前缀增量**"：
从"声明 i"到"声明 i+1"前缀只多了**一个声明**，却重跑整段（median 41 KB）
⇒ **应把"已 elaborate 的前缀状态"续用**，只处理新增声明。
⚠ 真障碍：`compile_fol_with`（`check/mod.rs:286`）⇒ `run(&[SourceUnit::single(...)])`
⇒ **每次从零造 `EnvBuilder`**；`EnvBuilder` 不是 `Clone` ⇒ 需要
**per-前缀 builder 池**（`HashMap<前缀哈希, EnvBuilder>`）或让 judge 接收
调用方已有的 builder（受限于"只有源 `Expr`、没有 `ExprPtr`"）。

---

# 附七：✅ **P1-a 第一步落地 —— 就地判定接在「一个判定点」上**（2026-09-29，默认已开）

## 1. 🔑 切法由**数据**定：3759 趟按调用点归因

把上一轮 `SOKO_INFER_TRACE=all` 的 3759 条 `INFER_MISS`（含回溯）逐条归因到**最内层调用点**：

| 调用点（`elab.rs`） | 趟数 | 字节 | 占比 |
|---|---|---|---|
| **`infer_type_text`** | **2697** | **120.5 MB** | **72% / 73%** |
| ↳ 其中经 `args_fit_layers_in_order` | 1759 | 76.0 MB | 47% |
| ↳ 其中经 `elab_notation`（`universe_level_text_of_operands` 内） | 938 | 44.5 MB | 25% |
| `universe_level_text_of_operands` 的**第二问**（输入已是文本） | 468 | 21.4 MB | 12% |
| `judge_render_type` / `lower_value`（`by` 路径） | 536 | 19.8 MB | 14% |
| `application_arg_expected` | 34 | 1.4 MB | 1% |
| **`infer_expected_level`** | **18** | 0.8 MB | **0.5%** |

**⇒ 附二 §E 原先定死的"唯一判定点 = `infer_expected_level`"被数据判死**（只占 0.5%）；
真判定点是 **`infer_type_text`（72%）**，而且它的两个上游（`args_fit_layers_in_order`
与 `elab_notation`）**手里本来就有** `&mut EnvBuilder` + `KnownTable` ✓。

## 2. 这一刀怎么切最小（对账结论）

* **不改 `judge_infer` 的签名**（那要动 10 个调用点，且仍缺"把源 `Expr` elaborate 成
  `ExprPtr`"这一环）⇒ **反向接**：让**调用方**用它手里的活环境就地答；
* 新增 `InplaceEnv { builder, known }`（两个字段都是**借用**，同一个 `dag` ⇒
  指针同一性保住 ✓ —— 这正是 §17 否掉"重建 builder"的那条红线）；
* 就地三步与慢路**逐字对齐**：`judge::synthesized_check_term`（造项的唯一实现）→
  `elab_expr`（空 `UnivMap` + scratch hovers，与 `Walk::check` 同形）→
  **`ExportFile::infer_type_text_at`**（内核里那个**零调用点**的零件**首次接线**）
  → `judge::peel_binders`（剥 binder 的唯一实现）；
* ⚠ **只做"未命中"**：命中仍走今天那条哈希快路（`judge::judge_infer_lookup`）——
  每题都走就地是**负优化**（§4 的坑③）；就地答出后写回**同一张缓存**（键不变）。

## 3. 判据（release · 冷缓存 · 1 job · 全课程 `build --json courses/set-theory`）

| 读数 | `off` | `on` | Δ |
|---|---|---|---|
| `build.decl` / `build.file` / `build.begin` / `build.summary` | 2647 / 42 / 1 / 1 | **同** | **0** ✓ |
| `--json`（剔除 `build.tick` 心跳） | — | — | **逐字节相同**（2691 行等长、0 行不同）✓ |
| `passes` | 4126 | **2248** | **−45.5%** |
| `doc_passes` | 266 | 266 | 0 ✓ |
| `JUDGE_PREFIX runs` | 3759 | **1881** | **−50.0%** |
| `JUDGE_PREFIX bytes` | 174,213,583 | **93,858,420** | **−46.1%** |
| `judge_ms` | 146,580 | **120,359** | −17.9% |
| `by_calls`（judge 重跑顺带重跑的 `by` 块） | 69085 | 37760 | −45.4% |
| `JUDGE_INPLACE used / fallback` | — | **1878 / 39** | fallback **2.0%** |
| **墙钟** | 214.19 s | **158.90 s** | **1.35×** |
| `shadow` 档两条路文本 | — | `shadow_same=555552` · **`shadow_diff=0`** | ✓ |

**账对得上**：`used(1878) + runs_on(1881) = runs_off(3759)` ✓ —— 每一趟省下的前缀重跑
都对应一次就地作答。

## 4. 三个**实测**踩到的坑（都写进了代码注释）

1. **pp 档位**：`kernel_phase.rs:199` 的 `#check` 会先把 `config.pp_options.proofs = true`；
   少了它，pp 对**每个子项**调 `is_proof`（**空局部上下文**推类型）⇒ binder 内的
   `Eq n m` 之类直接 `loose bvar in infer` panic ✗（实测 7 次）。另：`with_env` 的
   `quiet_catch` 必须包在**内层**，否则 panic 时"装回 `dag`/`declars`"会被 unwind 跳过 ⇒
   builder 停在坏状态 ✗。
2. **回读用的解析器**：只用 `proof::parse_expr_text`（**pp 文本**的回读入口）⇒ 它不认识
   前缀里声明的**源级记法**（`∈` / `ᶜ` / `''` / `⁻¹'`）⇒ `unit12-solution` 单文件实测
   **77822 次 Parse 失败**（占全部分叉的 **88%**）。
3. **前缀解析太贵**：改成"接上整段前缀再 `parse_fragment`"（慢路就是这么解析的）⇒ 解析**对**了，
   但本路径对**每一次** `infer_type_text` 都生效（含十几万次**缓存命中**的调用，慢路那边它们
   是不花前缀钱的）⇒ **400 s 跑不完** ✗✗ ⇒ 最终选"**不解析**、直接用源 AST 造项"
   （成本只随**项**大小走，不随前缀走）+ **只做未命中** ✓。

## 5. 判据怎么防"空转"

两个集成测试都**断言路径真被走到**（否则一个永远走不到的实现也能让"逐字节相同"变绿 ✗）：
`crates/front/tests/judge_inplace.rs`（`shadow_same > 0` 且 `shadow_diff == 0`）·
`crates/front/tests/judge_inplace_on.rs`（**反向判据**：换依赖里一个声明的类型 ⇒ 结论变
**且** `INPLACE_USED` 增长 ⇒ 证明是**重算**而不是捞旧结论；顺带一条"判据不空转"断言）。

## 6. 没做的 / 下一步

* **仍剩 1881 趟前缀重跑**（`off` 3759 的一半）：`by` 路径的 `judge_render_type`/`lower_value`
  （536 趟）· `universe_level_text_of_operands` 的第二问（468 趟，输入已是**文本** ⇒ 天生
  不适合就地）· 未接线的小调用点 —— 都在 `elab.rs` 的其它判定点上，属 **P1-b**。
* **真正的大头仍是"前缀环境可保存/可恢复"**（`JUDGE_PREFIX runs` 要从 1881 再大幅降，
  而不是只降 46%）：即 `with_env_scope` 那条腿（per-前缀 builder 池 / 增量环境），
  与本次"就地判定"是**两条腿**、不互相替代。

---

# 附八：P1-b 第一档 —— 剩 1881 趟的**逐条归因** + 两个未接线点（默认关）

## 1. 🔑 1881 趟逐条归因（`SOKO_INFER_TRACE=all` + `#[track_caller]` 打**行号**）

做法：`judge_infer`/`judge_infer_with`/`judge_infer_cached` 加 `#[track_caller]`，
`INFER_MISS` 行多打一个 `at=file:line` ⇒ **每个未命中直接带调用点行号**（比回溯可靠，回溯会被内联搅乱）。

| 调用点 | 趟数 | 字节 | 是什么 | 能不能就地 |
|---|---|---|---|---|
| **`elab.rs:2275`**（`infer_type_text` 里的 `slow()` 闭包） | **819** | 40.1 MB | **未接线**的两个判定点（`guarded_binder_type` / `solve_prefix_args`） | **能** ✓（本档做了）|
| `elab.rs:1423` | 468 | 21.4 MB | `universe_level_text_of_operands` 的**第二问**：输入已是**渲染文本** | ✗（上游别渲染才行）|
| `judge.rs:1601` | 320 | 12.3 MB | `judge_render_type`（`by` 引擎的根目标规范化） | 要先把 `EnvBuilder` 通进 `by` 引擎 |
| `by.rs:58` / `by.rs:1279` / `by.rs:1241` / `by.rs:1499` | 216 | 7.5 MB | `by` 引擎内部 | 同上 |
| `elab.rs:1387` | 34 | 1.4 MB | `application_arg_expected` | 要（小）|
| `elab.rs:5002` / `:5004` / `:5438` | 24 | 0.9 MB | `field_sort_via_kernel` / `infer_expected_level` 等 | 要（小）|

**⚠ 归因方法上的一个坑**：`slow` 是**闭包** ⇒ `Location::caller()` 只会给**闭包定义行**
（2275）⇒ 那一行是"**所有走 `slow()` 的未接线调用**"的合计，不是某一个调用点。
要再细分就得把两条路拆成各自的行（本档直接按下面的代码清单定位：未接线的只有两处）✓。

## 2. 本档切的两个点（819 趟 / 40.1 MB = 剩余的四成四）

* `guarded_binder_type`（binder 记法 `{x ∈ s | p}` 的 guard 反解）—— 唯一调用方
  `binder_notation_operand`（在 `elab_expr` 里）✓；
* `solve_prefix_args`（记法前导参数反解）—— 唯一调用方 `notation_prefix_args`
  （在 `elab_notation` 里）✓。

两处都只是**把 P1-a 的 `InplaceEnv` 形状沿签名多传一级**（4 个签名 + 6 个调用点），
**没有新机制**。新接线点挂在**暂存开关** `SOKO_JUDGE_INPLACE_WIDE=1` 下（**默认关** ——
值守口径「一档一个 commit，默认 `off`/`shadow`」）；关掉 ⇒ 那两条路逐字节回到今天 ✓。

## 3. 读数（release · 冷缓存 · 1 job · 全课程 `build --json courses/set-theory`）

| 读数 | `off` | P1-a（`on`） | **P1-b 本档（`on`+`wide`）** |
|---|---|---|---|
| `--json`（剔 `build.tick`） | 基线 | 逐字节相同 ✓ | **逐字节相同** ✓（2691 行 / 0 行不同）|
| `build.decl` / `build.file` / `build.summary` | 2647 / 42 / `compiled:42 failed:0` | 同 | **同** ✓ |
| `JUDGE_PREFIX runs` | 3759 | 1881 | **1086**（−42%）|
| `JUDGE_PREFIX bytes` | 174,213,583 | 93,858,420 | **54,570,202**（−42%）|
| `passes` | 4126 | 2248 | **1452**（−35%）|
| `judge_ms` | 146,580 | 120,359 | **111,772** |
| `JUDGE_INPLACE used / fallback` | — | 1878 / 39 | **2641 / 50** |
| `shadow`（wide 档） | — | diff=0 | **`shadow_same`>0 · `shadow_diff=0`** ✓ |
| **墙钟** | 216.14 s | 158.90 s | **134.77 s**（**vs off 1.60×**）|

**判据不空转**：这一档的"走到了"证据是**结构性**的 —— `runs` 从 1881 掉到 1086
（少了 795 ≈ 新增接线点的作答数）；接线死掉的话 `runs` 不会动 ✓。
⚠ `doc_passes` 266 → **265**（差 1）：**计数口径**差异（少了一次被判为"真文档"的趟），
`--json` 逐字节相同 ⇒ 不影响判定 ✓，但如实记录。

## 4. 下一刀（按同一张表）

1. **`by` 路径（536 趟 / 19.8 MB）**：`judge_render_type` 收的也是**文本**，但它的调用方
   （`by.rs:513`/`:563`）手里**有源 `Expr`**；缺的是"把 `EnvBuilder` 通进 `by` 引擎"
   —— 那是**另一条管道**（`by` 引擎现在只拿 `prefix_src`），要单独勘明成本；
2. **468 趟文本输入**：先放着（值守口径 ✓），要动就从"上游别渲染"下手；
3. 小点（`application_arg_expected` 34 · `field_sort_via_kernel`/`infer_expected_level` 24）
   —— 都是同一个形状的复制，但收益小。

---

# 附九：P1-b 第二刀的**勘明**（`by` 路径 536 趟）—— 只勘不做，成本已量到"要动哪些函数"

## 1. 目标形状

`judge_render_type`（`judge.rs:1605`）与 `judge_infer` **收的都是文本**：

```rust
pub fn judge_render_type(prefix_src, options, binders, ty: &str) -> Option<String> {
    let term = format!("fun (__soko_render : {ty}) => __soko_render");
    let text = judge_infer(prefix_src, options, binders, &term).ok()?;   // ← 未命中就重跑整段前缀
    …
}
```

⇒ 就地版需要一个**源 `Expr`**（`ty` 的 AST）与**活环境**。前者在调用点**有** ✓，
后者**没有** ✗。

## 2. 环境要从哪来（逐级，已读代码）

| 层 | 位置 | 现状 | 要加什么 |
|---|---|---|---|
| 入口 | `elab.rs` 的 `elab_expr` → `by::run_by`（`:349` 一带） | 手里**有** `&mut EnvBuilder` + `known` ✓ | 传 `Option<&mut InplaceEnv>` |
| `by.rs:489` | `fn run_by(...)` | 只有 `prefix_src/options/defs` | 同上 |
| `by.rs:722` | `fn run_by_inner(...)` | 同上（**递归**） | 同上 |
| `by.rs:797` | `fn run_tactics(...)` | 同上 | 同上 |
| `by.rs:499` | `fn canonical_goal_type(...)`（调 `judge_render_type` `:513`） | 只有文本 | 同上 + **`ty: &Expr`**（已在手 ✓）|
| `by.rs:555` | `fn canonical_goal_with_spec(...)`（`:563`） | 同上 | 同上 |

**⇒ 6 个函数、一条 3 级递归的管道**（`run_by_inner`/`run_tactics` 都要跟着改签名）。
`judge_render_type` 本身要加一个"就地兄弟"（`judge_render_type_inplace(builder, known, ctx, ty: &Expr, binders)`），
形状与 `infer_type_text_inplace` 同（**源 AST + scratch hovers + `infer_type_text_at` + proofs=true**）。

## 3. 成本 / 收益（决定值不值）

* **收益上界**：536 趟 / 19.8 MB = 剩余 runs 的 **49%**、剩余字节的 **24%**
  （按本档实测 795 趟 ≈ 24.1 s 墙钟折算 ⇒ 约 **16 s**，134.8s → **≈119s**，即 1.13×）；
* **成本**：6 个签名 + 一条递归管道；`by.rs` 是 1600 行的大模块，且 `run_tactics`
  有多处早退/回溯 ⇒ **改动面明显大于前两档**（前两档各只动 `elab.rs` 内部）；
* **风险**：中 —— `by` 引擎的判定结果**决定后续 tactic 步进**（§19.4 的"结果被中途消费"），
  所以就地与慢路**必须逐字节一致**；证据链仍用 `shadow` 档 + 全课程 `--json` 对拍 ✓。

## 4. 建议顺序（下一轮直接用，不用重勘）

1. 先给 `judge_render_type` 写就地兄弟 + 一个**只接 1 个调用点**（`canonical_goal_type`，
   `by.rs:763` 一处）的暂存开关，走一遍 shadow + 全课程对拍；
2. 通了再把 `canonical_goal_with_spec`（`:837`/`:1101`/`:1332` 三处）接上；
3. 两处都绿之后，把 `SOKO_JUDGE_INPLACE_WIDE` **并进 `SOKO_JUDGE_INPLACE`**（删暂存开关）。

⚠ **别在没量 shadow 之前默认开**：`by` 引擎的分叉会以"tactic 步进不同"的形式出现，
比 `elab` 路径更难定位 ⇒ 影子档是这一档的**必需品**，不是可选项。

## 5. ⚠ 本档**还缺**的一条判据：⑤ 反向判据的 wide 版（recipe，别再重勘）

`crates/front/tests/judge_inplace_on.rs` 只覆盖 P1-a 那两个点（它设 `SOKO_JUDGE_INPLACE=on`、
**不设** wide）。wide 那两个点（`guarded_binder_type` / `solve_prefix_args`）要各有一个
"依赖真变 ⇒ 必须重算"的实测。**触发条件已勘明**：

* `guarded_binder_type`（`binder_notation_operand` → guard 反解）：走**两段式 binder 记法**，
  即 `∃ x ∈ s, p x`（binder 不写类型、由 guard `x ∈ s` 反解）——
  见 `courses/set-theory/lib/Exists.sokonanoda:103` 的 `binder_notation "∃" => Exists`
  与注释「一段式要写标注，**两段式靠 guard**」。⇒ 夹具要自带：`Set` + `∈` 记法 + `Exists`
  + `binder_notation "∃" => Exists`，然后**改 guard 里 `∈` 的目标签名**（例如把 `Set.mem`
  的第二个参数类型换掉）⇒ 结论必须变 **且** `INPLACE_USED` 增长。
* `solve_prefix_args`（`notation_prefix_args` → 前导参数反解）：走**缺前导参数的记法**，
  如集合字面量 `{a, b}`（课程侧见 `Set.pair` 一族）；夹具同样自带声明即可。

⚠ 两条都要带**"判据不空转"断言**（`INPLACE_USED` 必须增长）—— 我这一档就是靠它
发现"夹具没踩到接线点"的（第一次写的 `c = x` 夹具 `used=0`，当场判红 ✓）。

---

# 附十：🔴 **P1-b 第二刀（`by` 路径）第 1 步 —— 接线已试、判定分叉 ⇒ 已精确回退**（2026-09-30）

**结论先行**：`judge_render_type` 的就地兄弟**写出来了、也被走到了**
（`JUDGE_INPLACE_BY used=213 fallback=0`），但它与慢路**判定不一致**
（全课程 `--json` **38 行不同**：`lib/*` 从 `compiled` 变 `failed`）⇒
按"判定正确性不变"这条红线，**接线已 `git checkout` 精确回退**（五个文件零残留）。
下面是**勘明的根因与三条死路**，下一轮直接用，别再重走。

## 1. 唯一根因：`peel_binders` 的**静默失败**

`judge_render_type` 的收尾是三步（两条路都必须逐字相同）：

| 步 | 慢路 | 就地路（原始 pp 文本） |
|---|---|---|
| ① 剥声明 binder | **`judge_infer` 自己剥了 `n` 层**（`judge_infer_uncached` 结尾 `peel_binders(ty, binders.len())`） | 没剥 ⇒ 要剥 `n` |
| ② 剥 `__soko_render` | 1 层 | 1 层 |
| ③ 回读 + `render_roundtrip` | `parse_expr_text` | **同一个** |

⚠ **`peel_binders` 失败时是 `break`，不是 `Err`**（`judge.rs:1479`）——
而它的第一步就是 `parse_expr_text(&t)`，**不认前缀里声明的源级记法**。
⇒ 剥不动时它**原样返回**，于是：

* 慢路拿到的是 `judge_infer` 已剥过 `n` 层的文本 ⇒ 那一层能剥动 ⇒ 正常；
* 就地路拿到的是**带 `n` 层 binder 的原始文本**（`… -> Set.mem α h (Set.empty α) -> …`
  里就有 `Set.empty` 这类**点名前缀**）⇒ `parse_expr_text` 直接失败 ⇒
  **一层都没剥** ⇒ 交出去的"规范形态"其实是**没剥过的类型**
  ⇒ `apply` 的目标对齐拿到一个多层的函数类型 ⇒ 报
  「期望 `Not (…) -> Not (…)`，实际是 `α`」这类**看起来毫不相干**的错 ✗。

**症状的形状**（下一轮认这个）：错误集中在 `lib/Set.sokonanoda` 的
`exact` / `constructor`，文案是"期望一个 `A -> B` 形状，实际是 `α`/`True`/`Eq a a`"
——**期望的那一项正好是当前目标的类型**（说明目标没被剥）。

**⇒ 下一轮的正确切口**：**不要**让就地路去走"文本剥层"。
就地路手里有**内核项**（`term`），应该在**项层面**剥掉 `n + 1` 层 Pi
（内核有现成的项级剥法），**再把结果 pp 成文本** ⇒ 交出去的形态与慢路天然同构，
`parse_expr_text` 那一步的记法风险也随之消失 ✓。

## 2. 三条**实测走不通**的死路（别再试）

1. **手搓内核项 `mk_lambda(nm, Default, ty_ptr, ty_ptr)`**（体错传成 `ty`）⇒
   推断出来是 `T -> T`，剥一层得不到目标 ✗（低级但当时就是这么写的）；
2. **手搓 `mk_lambda(nm, Default, ty_ptr, mk_var(0))`**（体 = de Bruijn 0）⇒
   `eval: loose bvar` **每趟 panic**：`infer_type_text_at` 走 `infer_closed_type`
   （**闭项**判定），而 `mk_var(0)` 在**它自己的**局部上下文里没有条目
   —— 与 P1-a 坑①同一个 panic 家族；
3. **把声明 binder 先 `push` 进 `ElabScope` 再 elaborate `ty`** ⇒ `ty` 里对 binder
   的引用变成**局部变量**，elaborate 出来是**开项** ⇒ 还是 `eval: loose bvar` ✗
   （这条最隐蔽：scope 看着"更完整"，其实正是病根）。

**唯一走通的那条**：把查询写成**一整条源级 λ**
`fun (b1:T1) … (bn:Tn) (__soko_render : ty) => __soko_render`，
**交给 `elab_expr` 的 `Expr::Lambda` 分支**去 elaborate
（de Bruijn 的推入/抬升全由它负责 ⇒ 产物必然是闭项 ✓）。`used=213` 就是这么来的。

## 3. 另两条**必须一起做**的纪律（本轮各踩一次）

* **只做未命中**：就地路**必须先 `judge_infer_lookup` 查同一张缓存**再动手
  —— 第一版没查，`passes` 直接从 2248 涨到 3061（把十几万次 ~11 µs 的命中
  换成了 ~1 ms 的就地）✗；补上"命中 → 就地 → 慢路"三步后 `passes` 才回到 1854；
* **就地答出的 pp 文本要写回同一张缓存**（键与慢路同源），否则下一次同样的问
  还要再算一遍（`misses 266 → 488` 那一半就是这么来的）✓。

## 4. 本轮的**副产品**（已在回退里删掉，下一轮直接重加）

`JUDGE_INPLACE_BY used=/fallback=` + `JUDGE_INPLACE_BY_WHY <原因>` 两组计数
（`INPLACE_BY_REASONS` 记 `binder-no-ty` / `query-elab` / `query-panic` /
`kernel(<内核原话>)` / `reread` / `peel`）—— **这一档没有它就是盲飞**：
本轮全靠 `WHY` 才从"`used=0`"一路定位到 `eval: loose bvar` 与 `peel`。
⚠ 打印要**截断**（未命中几万条，全打会把终端刷爆 ✗）。
