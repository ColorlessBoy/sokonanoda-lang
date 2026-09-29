# 可增量扩展的环境（架构重改）—— **阶段 0：接口设计**

> 2026-09-29。用户 10:49 拍板：「**直接架构重改，解决核心性能问题**」——
> 授权做**彻底版**（可增量扩展的 Environment），不是 K1-b 最小版。
>
> 本文只写**接口与契约**（阶段 0 的交付物）；实现分阶段，每阶段独立 commit + 独立真绿 + 独立回退。
> 前作：`docs/design/by-judge-reuse.md`（G-31/T-K20′，2026-09-21 已写好机理与候选 A/B/C）·
> `docs/design/module-artifacts.md` §9（一个 builder 贯穿全场）。

## 1. 根因（已定量，不再论证）

| 事实 | 数值 | 出处 |
|---|---|---|
| 真课程冷编墙钟 | **219.3s**（方差 2.7%） | `docs/perf/course-profile-2026-09-29.md` |
| 跳掉两个 judge 入口后 | **26.8s**（`passes` 4126 → **387**） | 同上（`SOKO_NO_JUDGE=1`） |
| ⇒ **judge 占比** | **≈88%** | 同上 |
| judge 合成的 pass 数 | **253513** = 自身声明事件（2647）的 **95.8×** | 同上 |
| 最贵单条声明 | **4680ms**（`unit12-solution`） | 同上 |
| 成本随序号增长 | 41 模块 **20 个 r>0.5**、0 个 <−0.5、均值 **+0.44** | 同上 |

**机理**：`judge_infer` / `judge_pairs` 要回答"这个项什么类型"，而它们**只拿到
`prefix_src: &str`**（`judge.rs:932` / `:228`）⇒ 只能把**整段前缀** + 合成命令交给
`check_document_with` **从零重跑一趟 pass**。缓存键含**整段前缀的哈希** ⇒ 前缀随序号
线性变长 ⇒ 后段全 miss ⇒ **O(N²)**。

**对照 Lean**：Lean 维护**可增量扩展的 `Environment`**，查常量类型是**查表**；
前面的声明**绝不重跑**（`by` 是普通命令，按顺序 elaborate 一次）。

## 2. 谁持有环境（**唯一持有者**）

**`Walk` 持有 `EnvBuilder<'arena>`**（今天就是，`check/walk.rs:36`）——阶段 0 不改这个所有权，
只**把它的只读视图**暴露给判定路径。

**为什么不新增 `Session` 类型**：`module-artifacts.md` §9 已实测——front **不能命名** kernel 的
`pub(crate)` 别名 `DeclarMap` ⇒ 检查点进不了具名字段 ⇒ "一个 builder 贯穿全场"必须靠
**闭包式 API**（`with_project_session`）。本设计沿用同一手法：**环境句柄是一个借用**，
不是新类型。

**接口（阶段 1 落地）**：

```rust
/// **当前 pass 的只读环境视图**（判定路径问"这个常量什么类型"用它，不再重跑前缀）。
pub(crate) struct EnvView<'a, 'b> {
    /// 已经落到环境里的声明（含 prelude + 本文件到当前命令为止的全部声明）。
    pub(crate) env: &'b sokonanoda::env::Env<'a>,
    /// 当前命令的**前缀字节长度** —— 失效判据的唯一输入（见 §3）。
    pub(crate) prefix_len: usize,
}
```

它挂在 `ElabCtx` 上（`elab.rs:445` 已经带借用表 `inductives`/`defs` ⇒ 加一个借用字段是
**最小切口**，不改所有权）：

```rust
pub(crate) struct ElabCtx<'a, 'b> {
    pub prefix_src: &'b str,
    // …既有字段…
    /// **阶段 1 新增**：当前 pass 的只读环境（`None` = 老路径，回退重跑前缀）。
    pub env_view: Option<&'b EnvView<'a, 'b>>,
}
```

**为什么 `Option`**：单文件/测试/`compile_fol` 等路径没有 builder 在手边 ⇒
`None` ⇒ **逐字节回退到今天的行为**（这也是"可回退"的机制：删掉一个 `Some` 就回到原状）。

## 3. 粒度与失效判据（**红线：前缀真变了必须重算**）

**粒度 = 声明级**（每条命令处理完后环境向前走一格）。理由：judge 的查询发生在
**命令内部**（elaborate 某条声明时），那时环境正好"到该命令之前"⇒ 声明级是**唯一
与查询点对齐**的粒度；模块级会把同文件后面的声明也暴露出去（**改判定** ✗）。

**失效判据**（三选一，阶段 0 定为 **①**）：

1. **`prefix_len`（字节长度）相等** —— 简单、够用、**不引入新哈希**。
   同一个 pass 内前缀是**单调增长**的同一段字符串 ⇒ 长度相等 ⇔ 是同一个前缀 ✓。
   ⚠ 跨 pass 不成立（不同文件同长度）⇒ **`EnvView` 只在单个 pass 内有效**，
   由生命周期强制（`'b` 短于 pass）。
2. 前缀的哈希（更稳，但每次查询多一次哈希）。
3. 版本号（`builder.declaration_count()`）—— 不覆盖"前缀文本变了但声明数没变"的情况 ✗。

**反向判据（硬要求）**：造一个"**前缀确实变了**"的场景（同一文件里改动**前面**某条声明
的文本，使前缀长度变化），断言 **必须重算**（不许命中旧环境的结论）。判据放在
`crates/front/tests/` 的 `judge_env_*` 系列；**它必须能咬住"缓存住错误结果"的实现**。

## 4. 与内核怎么对接（**内核零改动** —— 已核对）

| 需要的能力 | 内核现状 | 结论 |
|---|---|---|
| 从已有声明表建只读环境 | `Env::new(&DeclarMap, &NotationMap, EnvLimit)` **已是 `pub`**（`env.rs:257`） | **够用** ✓ |
| 查某个常量的声明 | `Env::get_declar(&NamePtr) -> Option<&Declar>` **已是 `pub`**（`:283`） | **够用** ✓ |
| 查某个常量的值 | `Env::get_declar_val` **已是 `pub`**（`:344`） | **够用** ✓ |
| 拿当前 builder 的 `declars` | `EnvBuilder::with_env(|&mut ExportFile| …)` **已是 `pub`**（`builder.rs:112`） | **够用** ✓ |

⇒ **阶段 1/2 都不需要动 `crates/kernel/`** ✓（红线 2 自动满足）。
⚠ 若后续发现必须加访问器 ⇒ **单独汇报、单独授权**，不顺手改。

## 5. 分阶段（每阶段独立 commit + 独立真绿 + 独立回退）

| 阶段 | 内容 | 判据（缺一不算成立） |
|---|---|---|
| **0（本文）** | 定接口：谁持有 / 粒度 / 失效 / 内核对接 / 反向判据 | 本文 + 依赖文件清单（§6） |
| **1** | **`judge` 走环境**：`judge_infer` 与 `judge_pairs` 拿到 `EnvView` ⇒ 不再合成整段前缀 | ① judge 合成 pass **253513 → 接近 2647 量级** ② `unit12-solution` 单文件墙钟下降 ③ 最贵单条 **4680ms → ?** ④ `--json`/报告**逐字节不变** ⑤ 反向判据（§3）⑥ `profile-course.py` 前后同机对比 |
| **2** | **elaboration 主路径**：`elab.rs` 查常量类型不再重跑（`infer_type_text` 等入口） | 同 ①–⑥，外加 `passes` 接近 O(N) |
| **3** | **并行下的环境复用**：只读共享 / 分片 | 重测并行扩展性，看 **4 jobs 封顶是否打开**；核秒不再随核数上升 |

**阶段 1 的最小切片**（先证收益再铺开）：只改 `judge_infer`（它是大头 —— 只跳 `by` 判定
实测只有 **3%**），`judge_pairs` 留到阶段 2。

## 6. 依赖文件清单（阶段 1）

| 文件 | 改动 | 风险 |
|---|---|---|
| `crates/front/src/judge.rs` | `judge_infer_with` 加 `Option<&EnvView>` 形参；`judge_infer_cached` 的键**含前缀长度**；未命中时**先试环境**（`EnvView` 在 ⇒ 直接问内核；不在 ⇒ 回退合成前缀） | 中：缓存键改了 ⇒ 必须对拍 |
| `crates/front/src/compile/elab.rs` | `ElabCtx` 加 `env_view`；`infer_type_text`（`:2087`）等入口透传 | 低：`ElabCtx` 已带借用表 |
| `crates/front/src/compile/check/walk.rs` | 每条命令前建 `EnvView`（`builder.with_env`）并挂到 `CmdCtx` → `ElabCtx` | **中**：`with_env` 会**挪走** `dag`/`declars` ⇒ 回调期间 `self.builder` 不可用，必须确认判定路径不回调 `walk`（否则借用冲突） |
| `crates/front/tests/` | 新增 `judge_env_*`：① 复用生效（pass 数下降）② **反向判据**（前缀变了必须重算）③ `--json` 逐字节对拍 | 低 |

**已知坑（写在这里省一次 debug）**：
* `EnvBuilder::with_env` 期间 builder 被 `mem::replace` 成占位 ⇒ **回调里不许再碰 builder**
  （`builder.rs:112` 的注释）。若判定路径需要递归用 builder ⇒ 必须先取**检查点**再出回调。
* `NatLit` **按指针比较**（`conv.rs:169`）⇒ 复用的环境必须与当前 arena **同一个**
  （切片 1a 已把 arena 提到调用方 ✓，但**跨 pass 复用要重新核对**）。
* `decl_idx` 与**插入顺序**绑定（`builder.rs:338`）⇒ 复用必须是**同一张 map 实例**，
  不许"重建一张等价表"。

## 7. 本阶段**不做**的（不是"待办"，是明确排除）

* **不**把切片 1b 的 session 接线捡回来（已实测**不提速**：42/42 通过但 252.7s vs 218.8s ✗）。
* **不**同时开"并行锁竞争"那条线（用户明令：阶段 3 之后再评估 —— 修完环境，抢锁次数会
  同比掉下来）。
* **不**动 `crates/kernel/`（§4 已核对：现有公开 API 够用）。

## 8. ⚠ 阶段 1 开工前的**结构性阻塞**（2026-09-29 实测，必须回报后重定 §2/§3）

**§2 的前提是错的**：它假设"walk 期间 builder 里已经有**到当前命令为止**的声明"。
**实测不成立** ✗ —— `Walk` 默认**不往真 `builder` 加声明**：

* walk 阶段只产出 `PendingOp`（`walk.rs`），**声明是在 `kernel_phase` 才 `add_declar`**
  （`kernel_phase.rs` 的 `check_then_add_decl`，"逐字搬过来（纯重构、零行为变化）"）；
* 唯一会"在 walk 里就加"的开关是 `SOKO_WALK_REAL_ADD`（`walk.rs:203`），
  **默认关**，而且它的文档自己写着**回退方式 = 删掉这个函数** —— 即它是个**实验品**。

⇒ 所以 `judge_infer` 在 walk 期间调 `builder.with_env(...)` 时，
**环境里只有 prelude，没有本文件的任何声明** ⇒ "就地查表"查不到东西 ✗。

### 为什么这推翻了 §3 的粒度选择

§3 选了"**声明级**"，理由是"judge 的查询发生在**命令内部**，那时环境正好到该命令之前"。
既然**命令内部环境是空的**，声明级复用就无从谈起 —— 要么改**谁在何时填环境**，
要么把复用点**移出 walk**。两者都超出"最小切片"。

### 三条出路（**请用户选**，我不擅自定）

| 出路 | 做什么 | 代价 | 风险 |
|---|---|---|---|
| **A. walk 里就填环境** | **新写**一段"walk 边 elaborate 边 check-then-add"（**今天没有这段代码**，见下） | 中 | 中：要与 `kernel_phase` **同序同语义**（两趟 + `skip` + `trust`）；对拍能兜住 ⇒ 逐字节判据 |
| **B. 复用点移出 walk** | 判定改成"先编完文件、再对**已建好的环境**逐条判" | 大：`by` 引擎要改成**两阶段**（先收集、后判定） | 中：判定结果应当不变（同一批查询），但**诊断顺序/文本**可能变 ⇒ 要逐字节对拍 |
| **C. 只复用 `judge_infer` 的**解析产物** | 不碰环境，只缓存"前缀 → `parse_prefix` 的 AST"（省掉重复 parse） | 小 | 低，但**收益也小**（§5 实测：大头是**前端重新 elaborate**，不是 parse） |

**我的建议：B**。理由：它是**唯一**同时满足"消掉前缀重跑"与"不动判定语义"的出路 ——
A 会碰 check-then-add（判定语义），C 收益太小（parse 不是瓶颈）。
但 B 的改动面比 §6 列的清单大（要动 `by` 引擎的两阶段化），**所以必须先回报再动手** ✓。

**在用户定夺前，阶段 1 不动手**（避免按错的前提铺开代码）。

### 8.1 ⚠⚠ **三次更正：§8 的"阻塞"是我误读，前提其实成立**（2026-09-29 实测）

§8 说"walk 期间 builder 是空的、声明是 `kernel_phase` 才加"。**这是错的** ✗✓ ——
逐行核对后：

**walk **无条件**地 `add_declar`**（`grep -c "self.builder.add_declar" walk.rs` = **9 处**，
其中 **4 处是无条件**的 `PendingOp::Decl` 构造点：`:704`/`:960`/`:1081`/`:1305`，
另 4 处是归纳块/构造子 `:588`/`:826`/`:1045`/`:1192`，第 9 处在 `walk_real_add` 分支里）。

⇒ **环境在 walk 期间就是逐条长起来的** ✓ —— **§2 的前提成立**，
`judge_infer` 在命令内部 `builder.with_env(...)` 时**看得到本文件到该命令为止的声明** ✓。

**§8/§8.1 的结论作废**（那两节把"walk 不填环境"当成了阻塞，实际是误读；
`SOKO_WALK_REAL_ADD` 确实填不进环境，但**它本来就不是**填环境的那条路）。

**对阶段 1 的影响（好消息）**：§11.2 的"步 1（walk 边 elaborate 边 check-then-add）"
**不需要做** —— 环境已经在了。**阶段 1 直接做步 2**：
把 `EnvView` 接进 `ElabCtx` / `judge_infer`，让判定**查表**而不是重跑前缀。

⚠ **一个仍需核对的细节**：walk 的 `add_declar` **不做内核检查**（check 在 `kernel_phase`）
⇒ 环境里可能暂时包含**后来被内核拒绝**的声明。判定要在这上面查表 ⇒
必须确认"查询点看到的环境"与"今天重跑前缀时看到的环境"**语义一致**（这正是步 2 的
逐字节判据要抓的东西）。

## 9. 阶段重定（⚠ **框架已于 2026-09-29 更正**：目标是**环境本身**）

> ⚠ **本节最初的框架写错了**（值守 11:55 更正）：它把目标写成"必须服务 `module_key` 复用"。
> **那是把大头压在小头底下** ✗ —— 实测：**环境/judge 占 88%**（219.3s → 去掉 judge **26.8s**），
> 而 `module_key` 产物复用的**墙钟只省 10–25%**（`rebuild-baseline-2026-09-28.md` §7）。
> ⇒ **头号任务 = 让 `judge` 走 `EnvView`、不再重跑前缀**；
> `module_key` 产物复用是**次要目标**（且是 P3 可下载 cache 的前提），
> **不许用"必须先服务 module_key 复用、否则砍掉"去限制阶段 1–3** ✓。

**本节的阶段划分与判据仍然有效**（它们本身就是环境那一刀的判据），只是**框架改回"目标是环境本身"**。

### 9.1 重定后的阶段划分（替代 §5 的表）

| 阶段 | 内容 | 它解决什么 | 判据 |
|---|---|---|---|
| **1a** | ~~`module_key` 落地~~ ⇒ **已完成**（`module_keys()` 已有且判据齐；本阶段只补"同一模块跨入口同一条 key"的守卫，已落 `f7069589`） | （次要目标的地基，**不是头号任务**） | ① **无关模块变 ⇒ 别的模块 key 逐字节不变** ② **依赖变 ⇒ 下游 key 必变**（反例，红线）③ 单文件键逐字节不变（A1） |
| **1b** | **同进程按 `module_key` 复用**（**次要目标**；且**已实测墙钟只省 10–25%**，见 §10） | 产物化 / P3 的前提 | ① `by_calls` **3 → 1** ② 真课程**编译计数 174 → ~42**（**先报计数**）③ 墙钟（**同机**；**不许拿计数冒充提速**）④ `--json` 逐字节不变 ⑤ 改依赖必 miss（反例） |
| **1c ⇒ 头号任务** | **环境（本设计 §2–§4 那套）**：让 `judge` 能**查表**而不是重跑前缀 | **88% 就在这**（219.3s → 26.8s 是它的上界） | ① judge 合成 pass **253513 → 接近 2647 量级** ② 真课程墙钟 ③ `unit12-solution` 单文件墙钟 ④ 最贵单条 4680ms → ? ⑤ `--json` 逐字节不变 ⑥ 反向判据（§3） |
| **2** | **产物落盘**（磁盘产物，`module-artifacts.md` §8） | **跨进程/跨 run 复用** = 真正解决 rebuild | 冷/热对拍逐字节 · 改依赖必 miss · 完整性校验先于使用 |
| **3** | **可下载 cache** | 跨机器复用 | §8.3/§8.4 的安全模型与离线降级 |

**排序理由**：**1a → 1b 是 G-68 的直线**（产物 + 内容寻址），**1c 是"另一条腿"**
（让每次编译便宜）。值守明令"服务不了 `module_key` 复用的先砍" ⇒
**1c 排在 1b 之后**（不是砍掉，是**明确它的服务对象是"单次编译成本"而非"复用"**）。

### 9.2 降级路径（值守第 3 条：卡住就拆，不许整块退回）

**切片 1（同进程按 `module_key` 复用）→ 降级 a/b/c**：

* **(a) 完整版**：模块产物（内核环境 + 前端表 + 报告）按 `key(M)` 存，下游直接取；
* **(b) 降级一档**：只复用**前端表 + 报告**，内核环境仍重放（收益小一档，但**不碰内核**）；
* **(c) 降级二档**：只把**闭包去重**做到"同一进程内同一模块只 elaborate 一次"
  （= `by_calls 3 → 1` 那条**已实现过**的判据），**墙钟收益如实报**（§6 实测只 10–25%），
  并**明确写清它不等于 rebuild 变快**。

**已实测的教训（不许重犯）**：
* 切片 1b 的 session 接线**实测不提速**（42/42 通过但 **252.7s vs 218.8s** ✗）⇒ **不许捡回来**；
* 批编**已否掉**（`module-artifacts.md` §1）⇒ **不许再测**。

### 9.3 口径纪律（值守第 4 条，写进判据）

**报任何性能数字，先给 `failed` 与编译计数，再看墙钟** ✓ ——
**174 → 42 是计数，墙钟只省 10–25%**（`docs/perf/rebuild-baseline-2026-09-28.md` §7 实测）；
**不许拿计数冒充提速** ✗。

## 10. 阶段 1b 的**真根因**（为什么 1b 不能复用"共享库层编一次"那套）

§9 定了"1b = 把 `module_key` 接进 `build`"。开工前把**上一版为什么慢**定位清楚了
（stash `切片1b 产物复用`：42/42 通过但 **252.7s vs 218.8s** ✗）——**这不是白做的，
它的失败给出了 1b 的设计约束**：

### 10.1 根因：**一个模块的"前缀"取决于它被哪个入口编译**

`judge_infer` 的缓存键含 `prefix_src` 的**哈希**（`judge.rs:932`）。而"共享库层编一次"
那套里，库模块 L 的 `prefix_src` 是**并集顺序里排在它前面的那些**：

* 入口 A 的库列表 = `[L1, L2]`，入口 B = `[L1, L2, L3]`
  ⇒ 同一个 `L3`，在 A 的闭包里前缀是 `L1+L2`、在 B 的闭包里是 `L1+L2+L3`；
* **基线**（每个入口各编自己的闭包）下，L3 被编多次、**每次前缀都不同**；
* **session** 下它只编一次、**只有一个前缀** ⇒ 与基线的键**必然对不上** ⇒
  实测 `passes` 4126 → **5404**、`judge_ms` 148.4s → **162.9s** ⇒ **更慢** ✗。

⇒ **"按前缀复用"这条路本身与"一个模块只编一次"是冲突的**（前缀是 per-entry 的）。

### 10.2 1b 的设计约束（从上面直接推出来）

**模块产物必须是"只依赖它自己"的，不许依赖"它被谁编译"** ✓：

* **每个模块编一次，前缀 = 它**自己的直接依赖**（拓扑序），与入口无关**；
* 产物按 **`module_key`** 存（key 已经是这个形状：`H(源文本, [依赖名, 依赖key])`）；
* 入口装配时**按 key 取**自己闭包里每个模块的产物 ⇒ **不再重编** ✓。

⇒ 这正是 `module-artifacts.md` §2/§3 早就写好的形状 —— 上一版错在**用"共享库层"
近似了它**（共享层是 per-session 的，不是 per-module 的）。

### 10.3 判据（阶段 1b，缺一不算成立）

1. **先报计数**：真课程**模块编译次数 174 → ~42**（`SOKO_STAGE_STATS` 的 `passes`
   与 `by_calls`；**先给 `failed` 与计数**，再看墙钟 ✓）；
2. **`by_calls` 3 → 1**（`scripts/check-recompile-factor.py` 同口径）；
3. **`--json` 逐字节不变**（红线）；
4. **改依赖必 miss**（反例：改 `lib/Shared` 一行 ⇒ 入口必须重编）；
5. **墙钟**（同机同口径；**不许拿计数冒充提速** —— 上一版就是栽在这：
   计数降了 4.14×、墙钟**反而 +15%** ✗）；
6. **不得回归 §10.1 的坑**：`passes` **不许**高于基线（这是"前缀错配"的哨兵）。

## 11. 阶段 1 的**落地计划**（实测勘明，2026-09-29）

### 11.1 出路 A 已勘明可行（不需要动内核）

逐行核对结论：

| 需要的能力 | 位置 | 状态 |
|---|---|---|
| walk 里**拿得到 `Declar`** | `walk.rs:726`/`:982`/`:1103` 的 `PendingOp::Decl { declar: decl, .. }` 构造点 | ✓ **已拿到** |
| **就地检查**一条声明 | `ExportFile::try_check_declar(&self, d) -> Result<(), CheckError>`（`util.rs:667`，**`pub`**） | ✓ 可用 |
| 检查不过 ⇒ 不进环境 | 照抄 `kernel_phase::check_then_add_decl` 的 check-then-add | ✓ 语义可对齐 |
| `ExportFile` 在 front 可见 | `check/mod.rs:22` **已经 `use sokonanoda::util::ExportFile`** | ✓ 已用 |

⇒ **出路 A = 在 walk 的 `PendingOp::Decl` 构造点补一次"`try_check_declar` + `add_declar`"**，
**内核零改动** ✓。**不需要** `SOKO_WALK_REAL_ADD`（它的 `add_declar` 在早退分支里，见 §8.1）。

### 11.2 三步（每步独立可验）

| 步 | 做什么 | 判据 |
|---|---|---|
| **1** | walk 边 elaborate 边 check-then-add（环境在 walk 期间**逐条长起来**） | ① front **758/0** ② **`--json` 逐字节不变**（红线）③ 真课程 `failed:0` ④ 内核零改动 |
| **2** | `judge_infer` 接上环境：`ElabCtx` 加 `env_view: Option<&EnvView>`；`judge_infer_with` 加形参；缓存键**含前缀长度** | ① judge 合成 pass **253513 → 接近 2647 量级** ② 真课程冷编 **219.3s → ?** ③ `unit12-solution` → ? ④ 最贵单条 **4680ms → ?** ⑤ `--json` 逐字节不变 |
| **3** | `EnvView = None` ⇒ **逐字节回退**；反向判据 | ① 反向判据**必须能咬住"缓存住错误结果"的坏实现**（先造坏实现让它红）② `None` 路径与今天逐字节相同 |

### 11.3 风险与已知坑（照 §6）

* `with_env` 期间 builder 被 `mem::replace` ⇒ **回调里不许再碰 builder**；
  而 walk 的 `PendingOp::Decl` 构造点**正要写 `self.ops`** ⇒ 必须**先出回调再 push**，
  或**先 push 再检查**（用 `hide_declars`/`restore_declars` 那两个方法而非闭包 ——
  `walk.rs:647` 的既有注释说明过：那里同时借 `&mut self.builder` 与 `&self.known`，
  闭包会让 `self` 被可变借两次 ✗）。
* `NatLit` **按指针比较** ⇒ 复用的环境必须与当前 arena 同一个（切片 1a 已把 arena 提到调用方 ✓）。
* `decl_idx` 与**插入顺序**绑定 ⇒ 必须**同一张 map 实例**、**原序**加入。

## 12. 阶段 1 步 2 的**真障碍**：借用冲突（实测勘明，2026-09-29）

§11.2 步 2 说"把 `EnvView` 接进 `ElabCtx` / `judge_infer`"。**动手时撞到硬冲突** ✗：

### 12.1 冲突的形状

| 事实 | 位置 |
|---|---|
| `elab_expr` **已经**接收 `builder: &mut EnvBuilder<'a>` | `elab.rs:2913` |
| `judge_infer` 是在 **`elab_expr` 的调用链内部**被调用的（10 处，全部持 `ctx: &ElabCtx`） | `elab.rs:1363`/`:1396`/`:2089`/`:3512`/`:4212`/`:4658`… |
| `ElabCtx` **没有** builder 字段（只有 `prefix_src`/`options`/`inductives`/`ns`/`notations`/`defs`） | `elab.rs:445` |
| `elab.rs` 里 `with_env` **用了 0 次** | `grep -c with_env elab.rs` = **0** |

⇒ 要让 `judge_infer` 用**当前环境**，就得让 `ElabCtx` 能拿到 builder 的环境；
**但 `elab_expr` 已经可变借了那个 builder** ⇒ 同时再借一次是**借用冲突** ✗
（`&mut EnvBuilder` 与 `&EnvBuilder` 不能共存）。

### 12.2 这不是"加个字段"能解决的 —— 它要求**换共享模型**

三条出路（**都需要改结构，不是改签名**）：

| 出路 | 做什么 | 代价 |
|---|---|---|
| **A′. 环境与 builder 分离** | 把 `declars`/`notations` 从"builder 独占"改成**可共享**（`Rc<RefCell<…>>` 或把 `Env` 提到 `ElabCtx` 能持有的地方）⇒ `elab_expr` 与 `judge_infer` 各持一份引用 | **中**：`EnvBuilder` 的字段是私有的，要么内核加访问器（**要授权**），要么前端把环境**镜像**出来 |
| **B′. 判定后移** | `by` 引擎两阶段化：walk 只收集、判定在"文件编完、环境齐了"之后做 | **大**：动 `by` 引擎的批次结构 |
| **C′. 只复用 parse** | 不碰环境，只缓存"前缀 → AST" | 小，但**收益也小**（§5 实测大头是**前端重新 elaborate**，不是 parse）✗ |

**⚠ 关键约束（决定选哪条）**：`EnvBuilder::with_env` 期间 builder 被 `mem::replace`
成占位 ⇒ **回调里不能再碰 builder**。而 `judge_infer` 是在 elaborate **中间**被调的
⇒ 若走 A′，**镜像**（前端持一份 `Env`）比"回调里借用"更现实 ——
因为回调式 API 与"elaborate 中途要查环境"在生命周期上对不上。

### 12.3 我（agent）的判断

**A′ 的"镜像"变体最小**：前端已经**无条件**把声明 `add_declar` 进 builder（§8.1 实测 9 处），
若再维护一份**只读的 `Env` 镜像**（`Env::new(&declars, &notations, EnvLimit::PpUnlimited)`），
`ElabCtx` 就能持有它、`judge_infer` 就能查表 —— **不必动 `elab_expr` 的 `&mut builder`** ✓。
代价是"镜像"要与 builder **同步**（每条声明后更新一次），
且**必须证明**镜像与 builder 语义一致（步 2 的逐字节判据正是抓这个）。

**但这需要内核给一个"从 builder 取只读 `Env` 引用"的口子**（或前端自建镜像）
⇒ 按硬规矩**要单独授权 `crates/kernel/`**，**不许顺手改**。

## 13. 阶段 1 步 2 的**授权请求**（精确到一行 API）

§12.3 说"镜像变体最小"。**把镜像路走到底后确认：它需要内核加一个访问器** —— 精确到：

### 13.1 为什么前端**造不出**那个镜像（逐条实测）

| 需要 | 现状 | 结论 |
|---|---|---|
| 从 builder 拿 `declars` **的引用** | `EnvBuilder` 只有 `hide_declars(&mut self) -> DeclarMap`（**挪走**）与 `restore_declars` | ✗ 没有"借出"的口子 |
| 同上（另一条路） | `snapshot(&self) -> ExportFile` ⇒ `pub declars` 可读，但 `ExportFile` 是**拥有**的 ⇒ `file.declars` 只活在闭包/局部里 | ✗ 引用出不来 |
| 拿 `notations` 的引用 | 同上（`ExportFile.notations` 是 `pub` 字段，但 `ExportFile` 拥有它） | ✗ 同上 |
| `Env::new(&DeclarMap, &NotationMap, EnvLimit)` | **是 `pub`** ✓ —— 但 `DeclarMap` 是 `pub(crate) type`（`env.rs:252`） | ✗ **类型不可命名** |
| `clone()` 一份当镜像 | `DeclarMap: Clone` ✓ | ✗ **热路径**：每条声明后 clone 整张表 ⇒ 比今天更慢 |

⇒ **前端造不出"活的、不 clone 的"镜像** ✗。

### 13.2 请求（**最小**，一行 API）

> **在 `EnvBuilder` 上加一个只读借出入口**，例如
> `pub fn with_declars<R>(&self, f: impl FnOnce(&DeclarMap<'a>, &NotationMap<'a>) -> R) -> R`
> （或等价的"借出 `Env`"形式），**不改任何现有语义**、**不动判定路径**。

* **性质**：纯**能力**新增（read-only）· 不改判定 · 不改事件计数 · 不改 `--json`；
* **为什么必须**：`DeclarMap` 是 `pub(crate)` ⇒ 前端**永远**造不出 `Env::new` 需要的那个引用；
* **替代方案（若不想加）**：把 `DeclarMap` 改成 `pub`（**更大**的面，不推荐）；
* **拿到之后**：`Walk` 用它在每条声明后建/更新一个只读 `Env` 镜像 ⇒ `ElabCtx` 持有 ⇒
  `judge_infer` 查表 ⇒ **不再重跑前缀** ⇒ 目标是那 **88%**。

⚠ 按硬规矩（`AGENTS.md`：`crates/kernel/` 在 main 上零改动，除非明确授权）——
**本请求等用户明确授权后才动手**，**不顺手改**。

## 14. ⚠⚠ **§13 的授权请求撤回** —— `snapshot()` 这条路是通的（2026-09-29 实测勘明）

§13 说"前端造不出镜像 ⇒ 要内核加访问器"。**再往下核一层后：不需要** ✗✓。
关键是我上一轮漏看的两条：

| 我上轮以为 | 实测 |
|---|---|
| `Env::new` 要 `&DeclarMap`，而 `DeclarMap` 是 `pub(crate)` ⇒ 前端造不出 | ✓ 类型确实不可命名 —— **但不必命名它**：`ExportFile` 的 `declars`/`notations` 是 **`pub` 字段**，直接当实参传即可（**类型推断**，无需写出类型名）|
| 得从 builder **借**一份（借用冲突） | ✗ **不必借**：`EnvBuilder::snapshot(&self) -> ExportFile<'a>`（**`pub`**）给的是**拥有**的一份 ⇒ 它自己就是"镜像" |

**另外两条（决定可行性）**：

* `ExportFile::with_tc(&self, EnvLimit, f)` 收 **`&self`**（`util.rs:708`）⇒
  镜像**不需要 `&mut`** ⇒ 可以**多处同时查** ✓；
* `ExportFile::new_env(&self, EnvLimit) -> Env`（`util.rs:694`）也是 `&self` ✓。

### 14.1 于是形状是（**内核零改动** ✓）

```rust
// 每条命令**之前**（此时 elab_expr 还没借走 builder）：
let mirror: ExportFile<'a> = self.builder.snapshot();   // 拥有的一份，pub API
// 把它挂到 CmdCtx → ElabCtx（ElabCtx 加一个字段）
// judge_infer 未命中时：
mirror.with_tc(EnvLimit::PpUnlimited, |tc| {
    let ty = tc.infer_closed_type(expr);
    tc.with_pp(|pp| pp.pp_expr(ty))
})
```

* **借用冲突解决**：`snapshot()` 在 `elab_expr` 借走 builder **之前**取 ⇒ 之后
  `ElabCtx` 持的是**拥有的一份**，与 `&mut builder` **不冲突** ✓；
* **内核零改动** ✓（`snapshot`/`with_tc`/`new_env`/`EnvLimit` **全是 `pub`**）；
* **`EnvProvider` 接口不用改**（`crates/front/src/judge.rs` 已落 `30ae685c`）✓。

### 14.2 唯一要量的成本（决定成败）

`snapshot()` 会 `declars.clone()` + `notations.clone()` + `dag.clone()`。
**每条命令取一次** ⇒ 整文件 O(N²) 次 map 复制。
**但**它比"重跑整段前缀"（parse + elaborate + 内核检查）**大概率便宜得多** ——
这正是 §5 账单说的"大头是**前端重新 elaborate**"。
⚠ **必须实测**（不许拍脑袋）：`SOKO_DECL_PROFILE` 量"每条命令的 snapshot 成本" vs
"judge 合成 pass 的成本"。
**降级方案（若 snapshot 太贵）**：只在 `judge_infer` **真未命中时**才取快照
（即"惰性快照"）⇒ 命中路径零成本 ✓。

### 14.3 下一步（可立即执行，无需授权）

1. `ElabCtx` 加 `mirror: Option<&ExportFile<'a>>`（或让 `EnvProvider` 的实现者持有）；
2. `Walk` 在每条命令**之前** `snapshot()`，挂进 `CmdCtx`；
3. `judge_infer` 未命中时**先试镜像**（`with_tc` + `infer_closed_type` + `pp_expr`），
   失败/无镜像 ⇒ **回退**合成前缀（逐字节等价）；
4. 判据：judge 合成 pass **253513 → 接近 2647 量级** · 真课程 **219.3s → ?** ·
   `--json` **逐字节不变** · 反向判据**能咬住"缓存住错误结果"的坏实现** ·
   `None` ⇒ **逐字节回退**。
