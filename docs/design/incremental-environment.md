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
