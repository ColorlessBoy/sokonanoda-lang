# 可增量扩展的环境（架构重改）—— **阶段 0：接口设计 + as-built 对账**

> 2026-09-29。用户 10:49 拍板：「**直接架构重改，解决核心性能问题**」——
> 授权做**彻底版**（可增量扩展的 Environment），不是 K1-b 最小版。
>
> 本文只写**接口与契约**（阶段 0 的交付物）；实现分阶段，每阶段独立 commit + 独立真绿 + 独立回退。
> ⚠ **2026-10-05 恢复后新增 §0（as-built 对账）与 §32（接口更正）**：**现状读 §0**，
> §1–§31 是过程留档（含撤回，**不许当现状读** ✗）。
> 前作：`docs/design/by-judge-reuse.md`（G-31/T-K20′，2026-09-21 已写好机理与候选 A/B/C）·
> `docs/design/module-artifacts.md` §9（一个 builder 贯穿全场）。

## 0. 现状对账（as-built，**2026-10-05**）—— **先读这一节** ✓

> 本文 2026-09-30 在 docs-diet 清理中被**误删**，2026-10-05 从 `dae75759^` 恢复。
> 恢复时对照**当前代码**做了这次对账：**§1–§31 是过程留档**（含多次更正与撤回，
> 按当时的认识写 ⇒ **不许当现状读** ✗）；**现状以本节为准** ✓。
> **引用本文的代码/台账请指向本节或 §32**（§0.3 有逐处对照表 ✓）。

### 0.1 已落地（一行一条 + 读数 + 提交）

| 档 | 内容 | 读数（release · 冷缓存 · 1 job · 全课程 `build --json`） | 提交 / 判据 |
|---|---|---|---|
| **P1-a** | 就地判定第一步：`InplaceEnv`（活环境）+ `infer_type_text_inplace`（**不解析前缀、直接用源 AST**），接在 `infer_type_text` 一个判定点上；`SOKO_JUDGE_INPLACE`（默认 `on`） | `JUDGE_PREFIX runs` **3759 → 1881** · 墙钟 **216.14s → 158.90s** | `523560a4` · `0be209c8`（默认开）· `crates/front/tests/judge_inplace.rs` |
| **P1-b** | 铺开到全部判定点（wide + `by` 两档并进主开关，三档默认全开） | `runs` **3759 → 791（−79%）** · `bytes` 174.2MB → 43.3MB · `off` vs 默认 `--json` **0 行不同**（2691/2691）· 影子档 `shadow_same=44234 · diff=0` | `ffd2ab93` · `a79ac0ec`（收口）· `judge_inplace_wide.rs` / `judge_inplace_by*.rs` |
| **P1-c** | 收益兑现（端到端**双数字**） | 全课 `build` **214.08s → 126.80s = 1.688×**（各 3 轮中位数，极差 ≤0.28%） | `a4e211cc` · 读数入账 `9616c848` |
| **§3.C 前缀环境** | 把 `TRUSTED_PREFIX` 担保接到**主编译 pass**（`walk.rs` 每条命令后压栈）+ `check_synthesized` 的 `before.min(prefix_commands)` 夹取 | 全课 `build` **126.4s → 47.8s（2.65×）** · `judge_ms` **−83%** · `doc_passes` 265 → **0** · `--json` 0 行不同 · 影子档 `265/265 diff=0` | `24949995` · 本文 **§31**（含两个踩出来的坑 §31.2/§31.3） |
| **`#check` 两路受信任前缀**（G-92 的**部分**修） | `judge_infer_uncached` 的合成前缀也走 `run_synthesized_incremental`（不再无条件 `compile_fol_with`） | `unit12-synthesis` 墙钟 **10.97 → 5.75s（1.91×）** · `pass_total_ms` 25841 → 12206 | `cd8a376c` · 三条判据在册（影子 `diff=0` · 反向 · A/B 逐字节） |
| **G-29 稳态**（编辑器口径） | 就地失败**不再重跑慢路** + 就地路补 `pp_options.explicit` + `level_hint_of` 接就地 | 改一行 **2025 → 1040ms（−49%）** · `prefix_runs` **28 → 8（−71%）** · `modules` 46 → 11 · 改最后一条 **386ms**（`prefix=0`） | `260a1318` · `19075641` · `25f327f3` · 判据 `crates/lsp/tests/lsp_keystroke_structure.rs`（先红 `prefix=28`） |

⚠ **口径**：上表都是**结构计数**或**同机同口径墙钟**；`JUDGE_PREFIX runs` 量的是"**从源码重编前缀**"的
**趟数**，而 §3.C 省的是"**在 judge 里再查一遍前缀**" ⇒ **计数不动 ≠ 没收益**（§31.4 已记账 ✓）。

### 0.2 未收口（按重要性）

1. **G-92：`by` 判定的前缀重跑仍是 O(n²)** ✗（**本档唯一的大头**）。
   * **判据先红**（2026-10-05 实测 ✓ · 构建 = `20fb549d` + 工作树 `elab.rs`）：
     `docs/gaps/repro/G92-by-prefix-rerun-is-quadratic.sh` ⇒ `by_calls` 55（N=10）→ 210（N=20）
     = **3.82×**（阈值 3.0 ⇒ exit 0）；**守卫真的在跑** ✓（`gap.py check` 实测 `G-92 open script 缺口仍在`，与台账一致 ✓）；**夹具干净** ✓（`build --json` = `compiled:1 failed:0` ⇒ 量的**不是**失败路径 ✓ —— 台账第 17 棒正是被这个坑咬过 ✗）。
   * ⚠ **判据对这两条路都不敏感**（2026-10-05 同构建复测 ✓，`by_calls` **55 → 210** 三档一模一样）：
     `SOKO_JUDGE_INPLACE=off`（就地 infer 关）· `SOKO_JUDGE_ENV_REUSE=0`（受信任前缀关）
     ⇒ 它**只**量 `judge_pairs` 的合成编译**重 elaborate** ⇒ **光接 infer 那条 `EnvProvider` 不会让它动** ✗
     （能不能动它，取决于「合成编译拿到活环境」那一刀 = 出路 ② ✓）。
   * **根因（已收窄 ✓）**：`judge_pairs_uncached` 合成"整份前缀 + `_soko_judge_k`"
     再跑一趟；**受信任前缀只跳内核检查、不跳 elaborate** ✗ ⇒ 前缀里那些 `by`
     声明**又被 elaborate 一遍**。**2026-10-05 探针**（N=20 夹具 · 同一构建）：`JUDGE_STATS
     calls=20 pairs=20`（20 趟合成判定）· `JUDGE_PREFIX runs=2`（infer 那条路只 2 趟 ⇒ 放大不在它身上 ✓）
     · `passes=23`（线性 ✓）而 **`by_calls=210` = 20 + Σ(1..19)** ✗ —— 逐趟把前面所有 `by` 再跑一遍。
   * **出路（二选一，都还没做）**：① 给就地路加「**文本 ⇒ AST**」入口
     （边界 = `by.rs::level_hint_of` 的**记法形态**那一支：`infer(infer(…))` 要的是**文本**，
     而就地路只收**源 AST** ⇒ 整支回落 ✓）；② **合成编译复用调用方的环境** = §18 的 **K-2**
     = **G-68 那条架构件**（§30.2 判过"位置不成立"：调用方的 builder 只覆盖到当前命令之前）。
   * ⚠ **真实课程不呈现它**（逐声明耗时平线 ✓）—— 但判据**不许**因此写成"已达标" ✗。
2. **`EnvProvider` trait** —— **决策已定 = (a)** ✓（内核线 `edddbbae` + `787997f0` · 2026-10-05）：
   **形状已按 as-built 改成 AST 形** ✓ `fn infer_type_text(&self, binder_srcs: &[(String, Expr)], operand: &Expr)`
   （旧文本形与 `infer_type_text_inplace` 接不上 ⇒ 那是它零接线的原因之一 ✓）；**借用形态已定** ✓：
   实现方用**内部可变性**（`RefCell`）把 `&mut InplaceEnv` 收在自己身上、trait 保持 `&self` ✓。
   ⚠ **仍无真实现** ✗：`grep -rn "impl EnvProvider" crates/` = **1**，但那是**测试里的 `Fake`** ⇒ **死代码** ✓
   （内核线写着"不写已修" ✓）。**第 2 步已落** ✓：加法式入口 `judge_infer_with_env(provider, …)` +
   **文本 ⇒ AST 自己做**（= §0.2 #1 出路 ① ✓）、**无调用点 ⇒ 零行为变化** ✓；判据 = §32.3 四条。
3. **G-68 切片 1（按 `module_key` 复用产物）已停** —— §27.2/§28：并集 session 与 **per-entry 前缀**冲突，
   等价类分组只值 **2.28×** 且省不了趟数；`session_reuse.rs` 的正向守卫仍 `#[ignore]`（**不是待办** ✓）。
4. **G-29 剩下的 8 趟**：全是 `on-elab-operand`（记法 `∅` → `Set.empty` 补不出前导类型参数）= **语言层限制**（G-62 家族）⇒ 到范围边界 ✓。

### 0.3 引用本文的地方（改代码注释时照这张表 ✓）

| 引用处 | 该指向 | 为什么 |
|---|---|---|
| `judge.rs`（`EnvProvider` 文档 / `judge_env_reuse_enabled`） | **§0.2 #2 + §32** / **§30.1** | as-built 不是 `EnvView` · "T-K11 只服务 LSP 增量会话" |
| `compile/check/walk.rs`（`shadow` 字段 · `walk_real_add_enabled`） | **§8.1 + §32.3** | 两条旧注释说的"walk 不填环境/builder 被消费"**都不成立** ✗ |
| `judge_env_vouch.rs` 头 | **§31**（§31.4 数字 · §31.2/§31.3 两个坑） | 本文**没有** §21.5/§21.6 |
| `compile/check/mod.rs`（`closure_prefixes_for`）· `project/session.rs`（入口趟闭包前缀） | **§29.1 / §29.2** | 三次接线失败的同一根因 · 路乙修法 |
| `project/mod.rs`（`assemble_from_session`） | **§21.2 + §24** | 切片 1 的接线口与可见性 |
| `project/tests.rs`（`module_key` 守卫） | **§9.1**（阶段 1a 那一行） | 1a 的判据 |
| `session_reuse.rs` | **§20.3** | 两条判据就在这个文件里 |
| `lsp_keystroke_structure.rs` | **§1 + §0.2 #1** | O(N²) 机理在 §1、现状在 §0.2 |

## 1. 根因（已定量，不再论证）

> ⚠ **本节数字出自 2026-09-29 的冷编构建 ⇒ 跨构建 / 跨缓存态不可比** ✗（内核线 `a1d6bf8b`/`728a788b` 实测：热态下 judge 只占 `passes` **17%**、墙钟 ≈3.6%）—— **机理仍然成立** ✓，但**别拿它当今天的判据** ✗；现状见 §0.2 #1，判据以 **§32.3** 为准 ✓。

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

> ⚠ **本节接口未按此形状落地** ✗（2026-10-05 对账）：`EnvView` / `ElabCtx.env_view`
> **全仓零出现**；实际走 `InplaceEnv` + `infer_type_text_inplace`（**在调用点**做，
> 不进 `judge_infer`）。**原文保留**以便对照，as-built 见 **§0 + §32** ✓。

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

> ⚠ **失效判据 ①（`prefix_len`）从未实现** ✗（2026-10-05 对账）：缓存键仍是
> **前缀哈希 + 选项**（`judge_infer_key`），就地答案写回**同一张**缓存
> ⇒ 正确性不受影响 ✓，但"`EnvView` 只在单 pass 内有效"那套生命周期设计没落地。

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

**阶段 1 的最小切片**（先证收益再铺开）：只改 `judge_infer`（它是大头 —— 只跳 `by` 判定实测只有 **3%**），`judge_pairs` 留到阶段 2。

## 6. 依赖文件清单（阶段 1）

> ⚠ **本清单未按此落地** ✗（2026-10-05 对账）：`EnvView` / `ElabCtx.env_view` 全仓零出现 ⇒ as-built 见 **§0 + §32**；**下面的「已知坑」仍然有效** ✓（它们讲的是 `with_env` / 指针同一性 / `decl_idx` 的**机制**，与接口形状无关）。

| 文件 | 改动 | 风险 |
|---|---|---|
| `crates/front/src/judge.rs` | `judge_infer_with` 加 `Option<&EnvView>` 形参；`judge_infer_cached` 的键**含前缀长度**；未命中时**先试环境**（`EnvView` 在 ⇒ 直接问内核；不在 ⇒ 回退合成前缀） | 中：缓存键改了 ⇒ 必须对拍 |
| `crates/front/src/compile/elab.rs` | `ElabCtx` 加 `env_view`；`infer_type_text`（`:2087`）等入口透传 | 低：`ElabCtx` 已带借用表 |
| `crates/front/src/compile/check/walk.rs` | 每条命令前建 `EnvView`（`builder.with_env`）并挂到 `CmdCtx` → `ElabCtx` | **中**：`with_env` 会**挪走** `dag`/`declars` ⇒ 回调期间 `self.builder` 不可用，必须确认判定路径不回调 `walk`（否则借用冲突） |
| `crates/front/tests/` | 新增 `judge_env_*`：① 复用生效（pass 数下降）② **反向判据**（前缀变了必须重算）③ `--json` 逐字节对拍 | 低 |

**已知坑（写在这里省一次 debug）**：
* `EnvBuilder::with_env` 期间 builder 被 `mem::replace` 成占位 ⇒ **回调里不许再碰 builder**（`builder.rs:112` 的注释）。若判定路径需要递归用 builder ⇒ 必须先取**检查点**再出回调。
* `NatLit` **按指针比较**（`conv.rs:169`）⇒ 复用的环境必须与当前 arena **同一个**（切片 1a 已把 arena 提到调用方 ✓，但**跨 pass 复用要重新核对**）。
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
| **1c ⇒ 头号任务** | **环境（本设计 §2–§4 那套）**：让 `judge` 能**查表**而不是重跑前缀 | **88% 就在这**（219.3s → 26.8s 是它的上界） | ① judge 合成 pass **253513 → 接近 2647 量级** ② 真课程墙钟 ③ `unit12-solution` 单文件墙钟 ④ 最贵单条 4680ms → ? ⑤ `--json` 逐字节不变 ⑥ 反向判据（§3）· ⚠ **判据①的 253513 / 88% 出自 2026-09-29 构建 ⇒ 跨构建不可比** ✗（当前实测 judge 只占 `passes` 的 **17%** = 32/193，见内核线 `a1d6bf8b`）⇒ **判据以 §32.3 为准** ✓ |
| **2** | **产物落盘**（磁盘产物，`module-artifacts.md` §8） | **跨进程/跨 run 复用** = 真正解决 rebuild | 冷/热对拍逐字节 · 改依赖必 miss · 完整性校验先于使用 |
| **3** | **可下载 cache** | 跨机器复用 | `module-artifacts.md` §8.3/§8.4 的安全模型与离线降级 |

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

## 15. 下一步的**精确起手**（可复制执行，无需再勘明）

§14.3 的四步里，第 ③ 步的**实现细节**（已勘明到能直接写）：

### 15.1 `SnapshotProvider`（`EnvProvider` 的实现，放在 `judge.rs` 或 `compile/check/walk.rs`）

```rust
/// 用**拥有的一份环境快照**回答"这个项什么类型"（`EnvProvider` 的实现）。
pub(crate) struct SnapshotProvider<'a> {
    snapshot: sokonanoda::util::ExportFile<'a>,
}

impl<'a> EnvProvider for SnapshotProvider<'a> {
    fn infer_type_text(&self, binders: &[GoalBinderSpec], term: &str) -> Option<String> {
        // ① 源文本 → Expr（与今天合成 `#check` 时**同一个**回读入口，别另造一套）
        let text = crate::proof::render_closed_lambda(binders, term); // ← 复用现有的合成形状
        let expr = crate::proof::parse_expr_text(&text).ok()?;
        // ② 在快照上**就地**求类型（`with_tc` 收 `&self` ⇒ 可多处同时查 ✓）
        //    ⚠ 这里需要一个"把源 `Expr` elaborate 成 `ExprPtr`"的入口 —— 见 §15.2
        let ptr = elaborate_against_snapshot(&self.snapshot, &expr)?;
        self.snapshot
            .with_tc(sokonanoda::env::EnvLimit::PpUnlimited, |tc| {
                let ty = tc.infer_closed_type(ptr);
                tc.with_pp(|pp| pp.pp_expr(ty))
            })
            .into()
    }
}
```

### 15.2 唯一未勘明的一环：**"源 `Expr` → `ExprPtr`"用哪个入口**

`judge_infer` 今天靠"合成 `#check` 文件 → 整条流水线"完成这一步（所以慢）。
就地查表需要一个**独立的** elaborate 入口，候选：

| 候选 | 位置 | 状态 |
|---|---|---|
| `elab_expr(builder, expr, scope, univ, known, hovers, expected, expected_src, ctx)` | `elab.rs:2913` | **要 `&mut EnvBuilder`** ⇒ 与 walk 的 `&mut builder` 冲突（§12）⇒ 需**从快照新建一个 builder** |
| 从快照建 builder | `EnvBuilder::new(arena, config)` + 把 `snapshot.declars` 灌回去 | ⚠ `restore_declars` 收的是 `DeclarMap`（`pub(crate)`）⇒ **这条路要授权** ✗ |

⇒ **§15.2 是唯一还缺的一环**。两条收口方式：

1. **惰性快照 + 复用 walk 的 builder**：在 `judge_infer` 未命中时**临时**把 builder 借出来
   （用 `hide_declars`/`restore_declars` 两个**方法**而非闭包 —— `walk.rs:647` 的既有注释
   说明过为什么），elaborate 完再还回去。**但** `elab_expr` 此刻正持有 `&mut builder` ✗
   ⇒ 除非把 `judge_infer` 的调用点改成"先把 `&mut builder` 放回 `self` 再调"。
2. **把 `judge_infer` 的入参从 `&str` 改成"已经 elaborate 好的 `ExprPtr`"**：
   调用点（10 处）本来就在 `elab_expr` 内部、**手里已经有 `&mut builder` 与 `Expr`**
   ⇒ 让它**自己** elaborate 出 `ExprPtr` 再交给判定 ⇒ **不需要跨借** ✓。
   **这是最小、最干净的一条** —— 代价是改 10 个调用点（它们都在 `elab.rs` 内）。

**⇒ 结论：走 §15.2 的第 2 条**。它把"查环境"变成"查已经 elaborate 好的指针"，
**借用冲突自然消失**，且**不需要内核改动** ✓。

## 16. 阶段 1 的**最终卡点定位**（2026-09-29，勘到底）

§15.2 定了"把 `judge_infer` 的入参改成已经 elaborate 好的 `ExprPtr`"。**再往下核一层，
发现真正的卡点比 §15.2 描述的更靠前**：

### 16.1 卡点：**"elaborate 一个项"本身需要 `&mut EnvBuilder`**

`elab_expr(builder: &mut EnvBuilder<'a>, …)`（`elab.rs:2913`）**不是**只读环境查表 ——
它**用 builder 构造内核项**（实测它调 `builder.zero()`/`mk_sort`/`mk_var`/`mk_const`/
`mk_app`/`mk_lambda`/`mk_pi`… 等 **`&mut self`** 方法）。

而 `EnvBuilder` 的**公开 API 里没有任何"只读借出声明表"的口子**（实测：
`pub fn` 列表里只有 `declaration_count(&self)` 与 `add_declar(&mut self)`；
`hide_declars`/`restore_declars` 是**挪走/还回**，不是借出）。

⇒ **要"就地 elaborate + 查快照环境"，必须同时持有"一个能写项的 builder"与"一份只读环境"**
—— 而今天这两者**是同一个对象**（`EnvBuilder` 既持 `dag` 又持 `declars`，且 `declars` 私有）。

### 16.2 三条收口（**都要改结构，我无法凭现有信息判定哪条更小**）

| 出路 | 做什么 | 需要授权？ |
|---|---|---|
| **A″** | 内核加 `EnvBuilder::with_declars(&self, f)`（**只读借出**）⇒ 前端可在"写项"的同时"读环境" | **要**（`crates/kernel/`）|
| **B″** | 前端**重建一个 builder**：从快照的 `declars`（`pub`）逐条 `add_declar` 进新 builder ⇒ 用它 elaborate | 不要（但要确认**指针同一性**：`NatLit` 按指针比较，`conv.rs:169`）|
| **C″** | `by` 引擎两阶段化（判定移出 walk）—— **完全绕开**这个冲突 | 不要（但改动面最大）|

### 16.3 我的判断与**请求**

* **B″ 看起来最小**（内核零改动），但有一个**必须先验证**的点：
  从快照 `declars` 重建的 builder，与 walk 里那个 builder，**是不是同一个 arena / 同一批指针**
  —— 若不是，`NatLit` 的指针比较会让**判定结果变**（红线 ✗）。
  **这一步可以用一条单测验证**（不需要跑真课程）：建 builder → 加几条声明 → `snapshot()`
  → 用快照的 `declars` 重建 builder → 在两者上查**同一条 `Nat` 字面量**的类型 → 必须相同。
* **若 B″ 的指针同一性验证不过** ⇒ 只能走 **A″（要授权）** 或 **C″（改动面最大）**。

**⚠ 我停在"能判定"的边界上**：B″ 的指针同一性是一条**可执行的单测**，
但我这一轮的上下文已用尽，没有余量把它跑完并据此定案。
**下一步（明确、可执行）**：先跑那条指针同一性单测；过 ⇒ 走 B″；不过 ⇒ 回来请示 A″/C″。

## 17. **B″ 已被证否** —— 指针同一性过不去（2026-09-29 定案）

§16.3 说"先跑指针同一性单测：过 ⇒ B″"。**不必跑单测了 —— 静态事实直接否掉 B″** ✗：

| 事实 | 位置 | 后果 |
|---|---|---|
| `ExportFile.dag` 是 **`pub(crate)`** | `util.rs:627` | 前端**拿不到**快照的 `Dag` |
| `EnvBuilder::new(arena, config)` **总是** `Dag::new_local(&config)` | `builder.rs:140` | 重建出来的 builder **是另一个 `Dag`** |

⇒ 从快照 `declars` 重建的 builder 与 walk 那个 builder **不是同一个 `Dag`**
⇒ `NamePtr`/`LevelPtr`/`ExprPtr` 的**指针同一性不成立** ⇒
`NatLit` **按指针比较**（`conv.rs:169`）⇒ **判定结果会变**（红线 ✗）
⇒ **B″ 不可行**。

### 17.1 于是只剩两条（**都超出"顺手改"的范围**）

| 出路 | 性质 | 要授权？ |
|---|---|---|
| **A″** | 内核加 `EnvBuilder::with_declars(&self, f)`（**只读借出** `declars`/`notations`）—— 纯能力、read-only、不动判定路径 | **要**（`crates/kernel/`）|
| **C″** | `by` 引擎两阶段化（判定移出 walk，在"文件编完、环境齐"之后做）—— **完全绕开**这个冲突 | 不要，但**改动面最大** |

### 17.2 明确请求（与 §13 的差别：这次是**证否了替代方案之后**才提的）

**§13 提过一次又撤回**（当时以为 `snapshot()` 够用）。**现在 `snapshot()` 已被证明不够**
（它给的是**另一个 DAG 的拥有副本**，elaborate 要的是**同一个 DAG 的 `&mut`**）。

⇒ **请授权 A″**（`EnvBuilder::with_declars(&self, f)` 或等价形式），或者**指定走 C″**。

**A″ 的性质**（逐条对齐红线）：
* **read-only**：只借出，不改 `declars`/`notations`；
* **不动判定路径**：不改变任何现有调用点的行为（新方法，无人调用 ⇒ 零行为变化）；
* **不改事件计数 / `--json`**：与判定无关的纯能力；
* **拿到之后**：`Walk` 在每条命令前借出环境 ⇒ 与 `&mut builder` **同时**持有
  （一个是 `&`、一个是 `&mut` ⇒ **仍然冲突** ✗）
  ⚠ **等等 —— 这一条要再核**：`with_declars(&self)` 与 `&mut self` 也不能共存。
  正解可能是"借出 `Env` 后**把 `&mut builder` 放回 `self`**"（改 `judge_infer` 调用点
  的所有权形状）—— 那属于 **C″ 的轻量版**，**前端可做**。

### 17.3 修正后的建议

**先做"轻量 C″"（前端可做、不需授权）**：把 `judge_infer` 的调用点改成
"**先把 `&mut builder` 还回 `self`、再调判定、判定完再取回**"——
即在 `elab_expr` 的调用链上，把"需要判定的那一刻"从"持着 `&mut builder` 的中途"
挪到"可以暂时放手的位置"。**这需要改 `elab.rs` 的 10 个调用点的所有权形状**，
但**不碰内核** ✓。
**若那条也走不通** ⇒ 回来请示 **A″**（内核只读借出）。

## 18. **内核需要提供什么**（用户 2026-09-29 14:12 无条件授权改内核；14:18 要求落进设计）

**授权口径**（用户原话）：「**改内核能加速编译的话，就是能改内核，不需要『否则』**」——
`crates/kernel/` **无条件授权**（用于编译性能），**不必先穷尽前端方案**，不必逐次请示。
「judge pass <10,000 且墙钟降 ≥50%」那条**降级为参考**，**不再是转内核的闸门**。

**但两条不是"否则"，是"正确"的定义**（用户认可这个区分）：
* **判定语义绝对不许变** —— `--json`/报告**逐字节不变**。核心价值是"**不发明第二套判定
  逻辑，由完整 kernel 当裁判**" ⇒ 语义一变就是换了个编译器 ✗
  （B″ 被否**不是**因为"不能改内核"，是**因为那样改是错的**：`NatLit` 按指针比较）；
* **反向判据**：前缀/依赖真变了**必须重算**。

### 18.1 前端为拿到增量环境，逐条需要什么

| # | 需要的内核能力 | 为什么现有 API 不够 | 语义影响 |
|---|---|---|---|
| **K-1** | **只读借出当前环境**（`EnvBuilder::with_declars(&self, f)` 或等价的"借出 `Env`"） | `EnvBuilder` 只 `pub` 了 `declaration_count`/`add_declar`；`hide_declars`/`restore_declars` 是**挪走/还回**而非借出；`snapshot()` 给的是**另一个 DAG 的拥有副本**（`ExportFile.dag` 是 `pub(crate)`）⇒ 指针同一性不成立 ⇒ **`NatLit` 按指针比较会变**（§17 证否 B″ 的就是这条） | **零**（纯 read-only；无人调用 ⇒ 行为零变化）|
| **K-2** | **同一 DAG 内、可增量的 `EnvBuilder`**：能在**不重建 DAG** 的前提下把已编好的声明**追加**进环境 | 今天"重建 builder"必然 `Dag::new_local()` ⇒ 新 DAG ⇒ 与旧项指针不同源 | **零**（追加语义与 `add_declar` 同）|
| **K-3** | **`EnvLimit` 能按"声明下标"截断**（已有 `ByIndex`）**且跨 builder 有效** | `ByIndex` 的语义绑定"本环境内的插入顺序" ⇒ 跨环境复用时要能**按同一份声明序列**重建同样的下标 | **零**（同一序列 ⇒ 同一下标）|
| **K-4** | **环境与 arena 的可分离性**：一份环境能被**多个只读查询**同时使用 | `with_tc` 收 `&self` ✓（已够），但 `with_env` 会把 builder `mem::replace` 成占位 ⇒ **回调期间不能再碰 builder** | **零** |

**K-1/K-2 是**同一个诉求的两面**：前端要的是"**在当前 DAG 上，既能写新项、又能读已落地的
声明**"。今天 `EnvBuilder` 把 `dag` 与 `declars` **绑在一个私有结构里**，所以两者不可分离。

### 18.2 为什么这能解决 88%

`judge_infer` 今天只拿到 `prefix_src: &str`（`judge.rs:932`）⇒ 只能把**整段前缀**合成文件、
`check_document_with` **从零重跑一趟 pass**。缓存键含**整段前缀哈希** ⇒ 前缀随声明序号
线性变长 ⇒ 后段全 miss ⇒ **O(N²)**（实测：judge 占墙钟 **≈88%**、合成 pass **253513** 次
= 自身声明事件的 **95.8×**、成本随序号增长 r≈+0.44、最贵单条 **4680ms**）。

拿到 **K-1/K-2** ⇒ `judge_infer` 可以**在当前环境上就地查表**（`Env::get_declar` /
`with_tc` + `infer_closed_type`，**都已是 `pub`** ✓）⇒ 不再重跑前缀 ⇒ 打的就是那 88%。

### 18.3 改内核时的流程（**流程，不是闸门**）

* 内核改动**单独 commit**，message 写清"**动了什么 · 为什么现有 API 不够 · 怎么保证语义不变**"；
* **三件套**：三层回归（kernel `tests/` + front 单测 + CLI e2e）· **全语料对拍** ·
  `--json` **逐字节不变** —— 数字贴进 message；
* **每行改动都要能解释**；**可回退**；
* 反向判据（§3 的"前缀真变了必须重算"）必须有对应测试。

### 18.4 与"打包 + bump"的关系

性能改动**用户感觉不到**，除非**打包 + bump 版本**（用户 14:18 队列第 3 项）——
所以内核改动落地后，**必须**走 bump（本版 = **v0.78.1**，不动 v0.79.0「良基递归」）。

## 19. 2026-09-29 实测：三条候选的**判死与存活**（B″ 之后的新一轮）

§18 列了 K-1/K-2。动手时把候选逐条验证，结论如下（**都有实测/代码依据，不是推理**）：

### 19.1 候选 B（前端对着 `ExportFile` 造项）—— **判死** ✗

`ExportFile` **确实有全套 `mk_*`**（`mk_var`/`mk_sort`/`mk_const`/`mk_app`/`mk_lambda`/
`mk_pi`/`mk_let`/`mk_proj`/`mk_string_lit`/`mk_nat_lit`，`util.rs:932-1036`）⇒
一开始看起来"把 `elab_expr` 的目标从 `EnvBuilder` 换成 `ExportFile` 就行"。

**但它们的签名是 `&mut self, … -> ExprPtr<'t>`，而 `'t` 来自 `with_ctx` 内部的
`Arena::new()`**（`util.rs:696-705`）——**那个 arena 是 `with_ctx` 现造的局部**，
闭包一结束就没了 ⇒ **造出来的 `ExprPtr` 出不了那个作用域** ✗。
`EnvBuilder` 的 `mk_*` 用的却是 builder 自己的 `'a` arena（与所有已落地声明**同一个**）
⇒ **两者不是同一个 arena ⇒ 指针同一性不成立** ⇒ 判死（与 §17 否 B″ 同一条理由）。

### 19.2 候选 A（内核提供"在同一个 `ExportFile` 上 elaborate 源 `Expr`"）—— **存活，但面最大**

前端 `elab_expr` 的"造项"部分要下沉/暴露到内核，且要**与已落地声明同一个 arena**。
这正是 §18 的 **K-2** 的精确形状。

### 19.3 候选 D（**本轮新发现**）：`judge` 的查询**后移**，用主 pass 的环境

**观察**：`judge_infer` 之所以要重跑前缀，是因为它**在 `walk` 中途**被调用
（`elab_expr` 调用链内部）——那时它**没有**当前环境（`elab_expr` 正可变借走 builder）。

**但**：`walk` **已经无条件把声明 `add_declar` 进 builder**（§8.1 实测 9 处）⇒
**主 pass 结束时，环境里就有整份文件的声明** ✓。而 `judge` 问的**永远是前缀**
（当前命令之前的部分）—— 主 pass 结束时环境是**前缀的超集**，但
**judge 的语义要求"只看前缀"** ⇒ 直接用整份环境会**改判定** ✗（不可接受）。

⇒ **D 要成立，必须有"按声明下标截断环境"的能力** —— `EnvLimit::ByIndex` **已有** ✓，
但它绑定"**本环境内的插入顺序**"。若 `judge` 的查询**后移**到主 pass 之后，
用 `EnvLimit::ByIndex(该命令的下标)` 就能精确重现"只看前缀" ✓。

**⚠ D 的两个未验证前提**（下一步第一件事）：
1. `judge` 的结果**是否必须在 `walk` 中途**就拿到（即：它是否**决定后续 elaborate**）？
   若是 ⇒ 后移**不可能** ✗；若只是"记录判定结果/出诊断" ⇒ 后移**可行** ✓。
2. `EnvLimit::ByIndex` 的"下标"与 `judge` 需要的"前缀边界"是否**同一套编号**
   （`decl_idx` 与插入顺序绑定，`builder.rs:338` 的注释提过）。

**D 的潜在收益**：把 O(N²)（每个 judge 调用重跑前缀）变成
**O(N)（一次主 pass + N 次按下标查表）** ⇒ 直接打那 **88%**。
**D 不需要新的内核能力**（`ByIndex` 已有）——**只需重排调用时机** ⇒ **比 A 小得多**。

### 19.4 候选 D 的**前提 ① 已被否** ✗ —— `judge` 的结果**决定后续 elaborate**

逐条读那 10 个调用点的返回值用途（**实测，不是推理**）：

| 调用点 | 返回值用途 | 是否决定后续 elaborate |
|---|---|---|
| `elab.rs:1363` `notation_prefix_args` | `ty_text` ⇒ `notation_telescope` ⇒ **解出要补的前导参数** | **是** ✗ |
| `elab.rs:1396` | `sort_text` ⇒ 判断操作数是不是 `Sort` | **是** ✗ |
| `elab.rs:2089` `infer_type_text` | 被 `:1396`/`notation_prefix_args` 消费 | **是** ✗ |
| `elab.rs:3512` | `let` 绑定缺类型标注时**推断出类型文本** | **是** ✗ |
| `elab.rs:4212` | `text` ⇒ `parse_expr_text` ⇒ **给无标注 λ binder 补类型** | **是** ✗ |
| `elab.rs:4658` | `match` 的 scrutinee 类型 | **是** ✗ |
| `elab.rs:4700`/`:4702`/`:5136` | `sort_text_level(text)` ⇒ **宇宙层级** | **是** ✗ |

⇒ **`judge` 的结果在 `walk` 中途就被消费**（补隐式参数、补 λ binder 类型、定宇宙），
**后移不可能** ✗（后移会让这些决策没有输入）。

**⇒ 三条候选的最终状态**：**B 判死**（arena 不同）· **D 判死**（结果被中途消费）·
**A 存活**（内核提供"在同一个 `ExportFile` / 同一个 arena 上把源 `Expr` elaborate 成
`ExprPtr`"）—— 即 §18 的 **K-2**，**面最大但唯一可行**。

### 19.5 下一步（唯一，且**已勘明到可直接动手**）

走 **A / K-2**。形状（两条，选一条实现）：

* **A-1（内核侧）**：内核暴露"**在 builder 自己的 arena 上**由已解析的源 `Expr` 造项"
  的能力 —— 但内核**不认识前端的 `Expr` AST** ⇒ 需要前端**把 `elab_expr` 的造项部分
  传进去**（回调），或内核提供一个**极简的 term builder trait**；
* **A-2（前端侧，更小）**：**把 `elab_expr` 的 `&mut EnvBuilder` 换成"能同时读环境、
  写项"的东西** —— 观察：`EnvBuilder` 的 `mk_*` 只依赖 `dag`（+ `arena`），
  而 `declars`/`notations` 是**另一个字段** ⇒ **Rust 允许同时 `&mut self.dag`
  与 `&self.declars`** ✓（不同字段）⇒ **加一个内核方法
  `EnvBuilder::with_env_and_builder(|env: &Env, b: &mut EnvBuilder| …)`**
  —— 或更简单：**`EnvBuilder::env(&self) -> Env<'_, 'a>`**（借用 `declars`/`notations`）
  + 调用方**同时**持有 `&mut EnvBuilder` **是不行的**（同一个 `self`）✗。
  ⇒ 正解是**把"读环境"与"写项"拆成两个参数**：内核提供
  `fn with_env_scope<R>(&mut self, f: impl FnOnce(&Env<'_, 'a>, &mut EnvBuilder<'a>) -> R) -> R`
  —— 内部 `split` 借用（`&self.declars` + `&mut self.dag` 等）⇒ **Rust 允许** ✓。

**A-2 是新的最小切口**（比 A-1 小：不改前端的 AST，只加一个"同时借"的内核方法）。
**下一步第一件事**：确认 `EnvBuilder` 的字段能否这样 split（读 `builder.rs` 的字段定义）。

## 20. G-68 **切片 1** 的实现形状（同进程内按 `module_key` 复用依赖产物）

**值守 15:05 第 1 条钉死**：本轮交付物 = **切片 1**（同进程内按 `module_key` 复用依赖产物，
per-module 产物 + 内容寻址，对齐 `.olean`/`.vo`）。`EnvView`/阶段 1 **只许当支撑**。

### 20.1 键（用户 14:0x 定死，**必须含依赖内容哈希**）

```
key(M) = H( format, 编译器版本, prelude 模式,
            源文本(M), [ name(D), key(D) for D in **直接** import ] )
```
`ProjectPlan::module_keys()` **已经是这个形状**（`project/mod.rs:204`）✓ ——
它按**拓扑序**算、**与入口无关** ✓（前置判据
`module_keys_are_dependency_scoped_not_entry_scoped` 已绿：
无关模块变 ⇒ 别人键不变；**依赖变 ⇒ 下游键必变**）。

### 20.2 复用点：`compile_plan` 的**逐模块**编译（不是整闭包一趟）

**今天**：`compile_plan_with_progress` 把整个闭包（`lib/*` + 入口）**一次**交给
`compile_all_units_with_progress` ⇒ 一趟 `run` ⇒ **每个入口各编一遍共享库** ✗
（这就是 4.14× 与"分片无效"的同一个根：**共享闭包被重复编**）。

**切片 1 改法**：按**拓扑序逐模块**编，每个模块编前查 `module_key` 缓存：

```
for M in closure (拓扑序):
    k = key(M)
    if cache 有 k:  复用 M 的产物（CompileOutput 片段 + DocumentReport）
    else:           编 M（前缀 = M 自己的直接依赖，与入口无关）⇒ 存 cache[k]
入口 = 闭包里最后一个模块 ⇒ 它的产物就是整个入口的报告
```

* **前缀 = M 自己的直接依赖**（拓扑序）——**不是**"入口闭包的前缀" ⇒
  同一个 `M` 在任何入口下**前缀相同** ⇒ 键相同 ⇒ **复用成立** ✓
  （这正是否掉"共享库层"那套的原因：那套的前缀是 per-entry 的 ⇒ 键对不上 ✗）；
* **不许批编**（实测慢 **6.2×**）· **不许按前缀复用** ✗。

### 20.3 两条判据（已在 `crates/front/tests/session_reuse.rs`）

| 判据 | 状态 |
|---|---|
| **正向** `slice1_shared_module_is_compiled_once_across_entries` | **`#[ignore]`（TDD 先红）** —— 实现完成 ⇒ **删掉 `#[ignore]` 那一行** ⇒ 变成真判据 ✓ |
| **反向** `slice1_changing_a_dependency_forces_recompile` | **绿，不 ignore** —— 改依赖 ⇒ 必须重编（咬"缓存住错误结果"）|

**读数**：`module_compiles_total()`（`compile/mod.rs` 已导出；`run()` 入口按 `units.len()` 累加）
—— 它就是 **174 → 42** 那条口径的**直接读数** ✓（`by_calls` 数的是 `by` 引擎，**量错了东西** ✗）。

### 20.4 报数规矩（值守第 4 条）

任何性能数字**先报 `failed` 与合成 pass 计数，再看墙钟**；
**174→42 是计数、≠ 快 4 倍**（实测墙钟只省 10–25%）；**禁止写"大幅提速"**。

## 21. 切片 1 的**落地位置已定位**（2026-09-29，精确到行）

### 21.1 关键发现：`build <dir>` **今天对每个入口各编一遍共享库**

`crates/cli/src/build.rs` 的批量循环：

* 串行路径 `:255`：`build_one(file, &src, root, no_project, progress, **None**)`；
* 并行路径 `:228`：同样传 **`None`**；
* `build_one`（`:367`）收到 `precomputed: None` ⇒ 走
  `:405` `compile_plan_with_progress(plan, &options, progress)` ——
  **每个入口独立编它自己的整条闭包**（`lib/*` + 自己）。

⇒ 共享 `lib/*` 被**每个入口各编一遍** ✗ —— **这正是 4.14×（174 次模块编译 / 42 入口）
与"分片无效"（单片 **317.71s** ≈ 全量 **313.78s**）的同一个根**。

**`precomputed` 这个参数本来就是为切片 1b 留的接口**（注释写着"由 `with_project_session`
预先算好的结果"），只是**从来没有人喂过它** —— 因为 `with_project_session` 是
**per-entry** 形状（`lib_units` + `entries`），而 `build <dir>` 需要
**一次 session 覆盖全部 42 个入口**。

### 21.2 改法（最小）

**`build <dir>` 先跑一次跨全部入口的 session**，再把结果喂给 `build_one`：

1. 收集阶段：把 `files` 里**有 `import` 的**（项目源）逐个 `plan_project`，
   取出各自的闭包单元 ⇒ **库层并集**（去重，拓扑序）+ 每入口自己的单元；
2. **一次** `with_project_session(lib_units, entries, options, …)` ⇒
   库层**只编一次**，每个入口只编自己的命令；
3. 把每个入口的 `ProjectReport` 存进 `Vec<Option<ProjectReport>>`（按 `files` 下标）；
4. 批量循环里把 `None` 换成 `precomputed[index]` ⇒ `build_one` 直接用，
   **不再自己编闭包** ✓。

**判据（值守第 3/4 条）**：
* **先报计数**：`module_compiles_total()` 的 **174 → ?**（期望 ≈ 库模块数 + 入口数）；
* 再看墙钟（同机同口径；**174→42 是计数 ≠ 快 4 倍**）；
* **`--json` 逐字节不变**（红线；`build.decl`/`build.file`/`build.summary` 必须逐字节相同）；
* **反向判据**：改 `lib/Shared` 一行 ⇒ 该 `module_key` **必须 miss 重编**
  （守卫 `slice1_changing_a_dependency_forces_recompile` 已绿，实现写错它会红）；
* 删掉 `slice1_shared_module_is_compiled_once_across_entries` 的 `#[ignore]` ⇒ 必须转绿 ✓。

### 21.3 已知的两个坑（前面踩过，别再踩）

1. **前缀必须 per-entry 保持原样**：切片 1b 的实测失败（42/42 通过但 **252.7s vs 218.8s**）
   根因是"共享库层"用了**并集顺序的前缀**，与基线 per-entry 闭包前缀**对不上** ⇒
   `passes` 4126→**5404**、`judge_ms` 148.4→**162.9** ⇒ 更慢 ✗。
   ⇒ 切片 1 必须**保持每个入口自己的闭包前缀不变**（只把"库层只编一次"这件事做对）。
2. **不许批编**（实测慢 **6.2×**）· **不许按前缀复用**（前缀是 per-entry 的）。

## 22. 切片 1 的**确切 API 缺口**（2026-09-29 实测；这就是卡点，不是"待办"）

> ⚠ 本节三个小标题原写作 `31.x`（**与 §31 撞号** ✗，2026-10-05 恢复时改成 `22.x`；
> 内容一字未动 ✓ —— 这样 §31.x 的引用才不歧义）。

动手接线时撞到**一处**具体缺口。记在这里，避免下一轮重复勘。

### 22.1 缺口：session 的回调**交不出** `build_one` 需要的 `ProjectReport`

`build_one`（`crates/cli/src/build.rs:367`）要的是**完整** `ProjectReport`，
它由 `assemble_report(PlanCompiled { … })`（`project/mod.rs:381`）组装，而 `PlanCompiled` 要：

| 字段 | 谁能给 |
|---|---|
| `compilable: Vec<usize>` | plan 的 `closure.compilable()` ✓ |
| `flat_out: CompileOutput` | session 回调的 `out` ✓ |
| `reports: Vec<DocumentReport>` | session 回调的 `entry_reports` ✓ |
| `closure: &Closure` | **plan**（`PlanCompiled` 借它）✓ |
| `diagnostics` / `entry_path` / `root` / `manifest_path` / `requires_warning` | plan ✓ |

**⇒ 只差一样**：`assemble_report` 内部要**重建** `units`（它注释写着
"`units` 借用 `closure` ⇒ 不能与它同处一个结构体（自引用 ✗）；这里**重建**"），
而重建用的是**该入口自己的闭包顺序**。session 回调**没有把它交出来**
（它只给 `lib_ranges`（并集顺序的库区间）与 `entry_range`）。

### 22.2 两种收口（下一轮选一，都不大）

* **收口甲（改 session 签名，最小）**：回调多交一个参数
  —— **该入口自己的闭包单元列表**（或它的 `Vec<Range<usize>>`）。
  session 内部本来就有 `entry_units`（`entries[i]`）与 `lib_units`，
  按**该入口的闭包顺序**（`units_for_modules(plan, |_| true)`）拼出来即可 ✓。
  改动面：`session.rs` 的签名 + 它的 2 个调用点（本文件测试 + CLI）。
* **收口乙（不改 session）**：让 `build_one` 接受"**片段**"而不是完整 `ProjectReport`，
  由调用方在 session 回调里就地组装 —— 但 `build_one` 的缓存写盘
  （`cache::store_at`，`:411`）要完整报告 ⇒ 面更大 ✗。

**⇒ 选甲。**

### 22.3 为什么必须走 session（而不是别的路）

* **不许批编**（实测慢 **6.2×**）✗；
* **不许按前缀复用**（前缀是 per-entry 的 ⇒ 切片 1b 实测 `passes` 4126→**5404**、
  `judge_ms` 148.4→**162.9** ⇒ 更慢 ✗）；
* **不许按 module_key 逐模块编**（那是"重写编译模型"，会改事件/诊断 ⇒ 破 `--json` 红线 ✗）；
* ⇒ **唯一保持 per-entry 前缀不变、又让共享库只编一次的路 = 一次 session 覆盖全部入口** ✓
  （这正是 §21.2，也是"收口甲"要补的那一个参数）。

## 23. ⚠ **撤回 §22 的"收口甲"** —— 那个参数是多余的，且有 §21.3 的风险

§22.2 我判"session 回调缺一个参数（该入口自己的闭包单元列表）"，并照此加了参数
（commit `0a83378f`）。**再核一层后：判错了** ✗✓，**已 revert**（`7ac6447f`）。

### 23.1 为什么多余

`assemble_report`（`project/mod.rs:381`）**自己就会重建 `units`**：

```rust
let units: Vec<SourceUnit<'_>> = compilable.iter()
    .map(|&index| SourceUnit { name: &closure.modules[index].name, … })
    .collect();
```

它从 **`closure`**（plan 里的 `Closure`）按 `compilable` 的下标重建 ⇒
**根本不需要调用方再交一份单元列表** ✗。§22.1 的表里我自己抄了那段注释
（"这里**重建**"），却没读出"**从 closure 重建**"这半句 ⇒ 判错。

### 23.2 而且加了它**有风险**

session 回调里能拼出来的"闭包单元列表"只能是 **`lib_units`（并集）+ 入口** ——
而 §21.3 记着：**并集顺序的前缀**正是切片 1b 失败的根因
（`passes` 4126→**5404**、`judge_ms` 148.4→**162.9** ⇒ 更慢 ✗）。
把一个"看起来对、实际是并集"的东西交出去，**等于把坑递给接线方** ✗。

### 23.3 那 §22 的真缺口是什么

**重新判**：`build_one` 要的是**完整 `ProjectReport`**，而它由
`assemble_report(PlanCompiled { … })` 组装，`PlanCompiled` 需要：

| 字段 | session 能给？ |
|---|---|
| `flat_out` / `reports` | ✓（回调已有）|
| `compilable: Vec<usize>` | ✗ **没给**（但它是 **plan** 的 `closure.compilable()` ⇒ **接线方自己就有** ✓）|
| `closure: &Closure` | ✗ 没给（但同样是 **plan** 的 ⇒ 接线方自己就有 ✓）|
| `diagnostics`/`entry_path`/`root`/`manifest_path`/`requires_warning` | 全是 **plan** 的 ⇒ 接线方自己就有 ✓ |

⇒ **真缺口不是"缺参数"，而是"session 的回调签名把 `ProjectReport` 的组装拆散了"**：
接线方手里**有 plan**（⇒ `closure`/`compilable`/诊断全都有），**只差**
`flat_out` 与 `reports`（这两个正是回调给的）⇒ **接线方可以自己调
`assemble_report`**，**不需要新参数** ✓。

**⇒ 收口 = 在 `build` 侧写一个 adapter**（拿 plan + 回调的 `out`/`entry_reports`
⇒ 组 `PlanCompiled` ⇒ `assemble_report` ⇒ 喂 `build_one` 的 `precomputed`），
**`session.rs` 不用再改** ✓。

## 24. adapter 的**最后一道门**：`ProjectPlan.closure` 是**私有的**

§23.3 判"接线方自己调 `assemble_report` 即可"——**再核一层**：还差**一样可见性** ✗。

`PlanCompiled` 要 `closure: &'a Closure`（`project/mod.rs:373`），而：

* `ProjectPlan` 的字段 `closure: Closure`（`project/mod.rs:142`）**没有 `pub`** ✗；
* 它只暴露 `pub fn modules(&self) -> &[LoadedModule]`（`:146`）——**不是 `&Closure`** ✗；
* `Closure` 类型**本身是 `pub`**（`:28` 的 re-export）✓ ⇒ 只是**拿不到 plan 里那一份**。

⇒ **接线方（CLI）拿不到 `&Closure`** ⇒ 组不出 `PlanCompiled` ⇒ 调不了 `assemble_report` ✗。

### 24.1 两种收口（都很小，选一）

* **收口 1（最小，前端加一个访问器）**：
  `ProjectPlan` 加 `pub fn closure(&self) -> &Closure { &self.closure }`。
  **纯新增、零语义变化** ✓。接线方就能自己组 `PlanCompiled` ✓。
* **收口 2（前端直接给一个"从 session 结果出报告"的函数）**：
  在 `project/mod.rs` 加
  `pub fn assemble_from_session(plan: &ProjectPlan, flat_out: CompileOutput, reports: Vec<DocumentReport>) -> ProjectReport`
  —— 把"组 `PlanCompiled`"这件事**收进前端**（不让 CLI 知道 `PlanCompiled` 的细节）✓
  **更干净**（CLI 少依赖一个内部结构），推荐这条。

**⇒ 选收口 2。**

### 24.2 于是切片 1 的最后一步是（三处小改，都已定位）

1. **前端**：`project/mod.rs` 加 `assemble_from_session(plan, flat_out, reports) -> ProjectReport`
   （内部组 `PlanCompiled` + 调 `assemble_report`；`plan` 的私有 `closure` **在前端内部** ⇒ 够得着 ✓）；
2. **CLI**：`build <dir>` 先跑**一次**跨全部入口的 `with_project_session`
   （`lib_units` = 各入口库层的**并集**（去重、拓扑序），`entries` = 各入口自己的单元），
   回调里对每个入口调 `assemble_from_session` ⇒ 存进 `Vec<Option<ProjectReport>>`；
3. **CLI**：批量循环把 `build_one(..., None)` 的 `None` 换成 `precomputed[index]`
   （`crates/cli/src/build.rs:228` 并行路径 / `:255` 串行路径）；
   `build_one` 里 `:405` 的 `match precomputed { Some(p) => p, None => compile_plan_with_progress(...) }`
   **已经写好了** ✓ —— **这个参数从切片 1b 起就留着，一直没人喂过**。

## 25. 切片 1 接线**第一次尝试：收益巨大但组装错了**（实测数字，非常重要）

把 §24.2 的三步接上（CLI 侧，本机 release、冷缓存、1 job）后实测：

| 读数 | 基线 | 接线后 | 倍率 |
|---|---|---|---|
| `passes` | **4126** | **271** | **15.2×** ↓ |
| `judge_ms` | **146.4s** | **5.3s** | **27.6×** ↓ |
| `doc_passes` | **266** | **19** | **14×** ↓ |
| `misses` | 266 | **19** | — |

⇒ **方向完全正确**（这正是 88% 那一刀该有的形状 ✓）。

**但它 `exit=101` 崩了**：`crates/front/src/project/mod.rs:480`
`index out of bounds: the len is 1 but the index is 1`。

### 25.1 崩因（已定位到行）

`assemble_report` 里：

```rust
let mut compiled: HashMap<usize, usize> = compilable.iter().enumerate()
    .map(|(slot, &index)| (index, slot)).collect();     // slot = 闭包内第几个**模块**
…
events.errors = reports[slot].errors.clone();            // ← 这里 :480
```

它要的 `reports` 是 **整个闭包（`lib/*` + 入口）逐模块**的报告（长度 = `compilable.len()`）。
而 session 回调交的是 **`entry_reports`（只有入口那一个模块）** ⇒ `len == 1`，
而 `slot` 到了 `1` ⇒ **越界** ✗。

**正解**：`reports` 必须是 **`lib_reports` + `entry_reports` 按该入口闭包顺序拼起来** ——
这正是 session 回调**为什么要交 `lib_ranges`（并集顺序下每个库模块的区间）** 的原因
（它的注释原文："修法 A：按各入口自己的闭包顺序拼接 + 重编号"）✓。

⇒ **接线方必须做"拼接 + 重编号"**，而不是直接把 `entry_reports` 递进去 ✗。
**这不是"缺参数"，是"接线方少做了一步变换"** —— §22/§23 两次都判偏了，
**这次有越界 panic 的精确行号**（`:480`）作证。

### 25.2 下一步（唯一，且形状已明）

在 `build` 侧（或前端加一个 helper）把 session 交的两份报告拼成"该入口闭包的逐模块报告"：

1. 用 `entry_closure`（**该入口自己的**闭包单元顺序 —— 注意**不是** `lib_units` 并集顺序）
   算出**期望的报告序列**；
2. 用 `lib_ranges`（并集顺序的库模块区间）把 `lib_reports` 映射到该入口的库模块；
3. 拼 `lib_reports' ++ entry_reports` ⇒ 交给 `assemble_from_session`。

⚠ **§21.3 的坑仍适用**：拼接必须按**该入口自己的闭包顺序**，用并集顺序会
`--json` 对不上 ✗。

## 26. 切片 1 接线**第二次尝试**：不再崩，但**结果错了**（`compiled:4 failed:38`）

按 §25.2 加了 `merge_session_reports`（按**该入口自己的**闭包顺序拼
`lib_reports` + `entry_reports`）后重接 CLI 线：

| 读数 | 基线 | 第一次（§25） | **第二次** |
|---|---|---|---|
| `exit` | 0 | **101**（越界崩） | **0** ✓ |
| `passes` | 4126 | 271 | **590** |
| `judge_ms` | 146.4s | 5.3s | **6.9s** |
| `doc_passes` | 266 | 19 | **90** |
| `compiled / failed` | **42 / 0** | —（崩） | **4 / 38** ✗ |
| `build.decl` | **2647** | — | **0** ✗ |

⇒ **不崩了，但结果错** ✗ —— `--json` 与基线**不一致**（红线），
**已 revert 接线**（只留 `merge_session_reports` 这个纯新增 helper）。

### 26.1 失败形状（实测）

`build.summary` = `{compiled: 4, failed: 38, files: 42}`；
`build.file` 的 failed 里**包括 `lib/*.sokonanoda`**（`lib/Cardinal`、`lib/Choice`…）
—— 而**基线里 `lib/*` 根本不是 `build.file` 的目标**（基线 42 个 `build.file` 全是入口）。

⇒ **根因方向**：我把**全部 42 个文件**都当成"项目入口"喂进 session 的 `entries`，
但其中 **8 个是 `lib/*`（库模块，不是入口）** ⇒ 它们被当入口编 ⇒ 失败；
而真正的入口因为报告槽位错配也判 failed ⇒ `build.decl` 一个都没发。

**⚠ 这条修正了 §21.2 第 ① 步的表述**：`build <dir>` 的 `files` **包含库模块**，
必须**先按"是不是入口"分类**（库模块只作库层、不当 entry），
而不是"把所有有 `import` 的都当入口" ✗。

### 26.2 下一步（唯一，形状已更精确）

1. **分类**：`files` 里哪些是**入口**（被别的模块 `import` 的 ⇒ 库模块；其余 ⇒ 入口）。
   现成的判据：`plan.closure` 的拓扑序里**入口恒在最后**，且
   `units_for_modules(plan, |m| m.path == plan.entry)` 给的就是入口 ——
   但**跨入口**判断"这个文件是不是别人的库"需要**全局**看一眼（例如
   "它出现在别的 plan 的 `m.path != plan.entry` 集合里" ⇒ 它是库）。
2. **库层 = 全部库模块的并集**；**entries = 只有入口**；
3. 回调里对**每个入口**用 `merge_session_reports` 拼报告 ⇒ `assemble_from_session`。

**判据（一个都不许少）**：`--json` **逐字节不变**（`build.decl` **2647**、
`build.file` **42**、`compiled:42 failed:0`）· `passes`/`judge_ms` 下降 ·
**反向判据**（改依赖必须 miss）· 删 `#[ignore]` 后正向守卫转绿。

## 27. 切片 1 接线**第三次尝试**：分类修好了，但撞上 **§21.3 那堵墙**（`failed:30`）

按 §26.2 加了"**入口 / 库**分类"（一个文件若出现在**别的** plan 的非入口模块里 ⇒ 它是库）
后重接：

| 读数 | 基线 | 第一次 | 第二次 | **第三次** |
|---|---|---|---|---|
| `exit` | 0 | 101（崩） | 0 | **0** |
| `compiled / failed` | **42 / 0** | —（崩） | 4 / 38 | **12 / 30** ✗ |
| `passes` | 4126 | 271 | 590 | **667** |
| `judge_ms` | 146.4s | 5.3s | 6.9s | **8.3s** |

⇒ 分类**确实修好了一部分**（4→12 compiled，38→30 failed），**但仍有 30 个入口失败** ✗。

### 27.1 根因：**session 的库层是"并集"，与每个入口自己的闭包前缀对不上** —— 这就是 §21.3

`with_project_session` 收到的是 **`lib_units` = 所有入口库层的并集**，
它把这**一整坨**当库层编一趟；而每个入口那趟的前缀是**这一整坨并集**。
但**基线**下每个入口的库层前缀是**它自己那条闭包**（子集、且顺序可能不同）⇒
**前缀不同 ⇒ 环境/记法/解析上下文都不同** ⇒ 有 import 的入口里
引用了"自己闭包里有、并集里顺序不同"的声明 ⇒ **失败** ✗。

**这正是 §21.3 记的坑**（切片 1b 当年就是栽在这：`passes` 4126→**5404**、
`judge_ms` 148.4→**162.9** ⇒ 更慢）—— 我这次**又踩了一遍**，
只是症状从"更慢"变成"30 个入口失败"（因为这次还叠了报告拼接的改动）。

### 27.2 ⇒ 结论（重要，别再绕）

**"一次 session 覆盖全部入口"这条路与"保持 per-entry 前缀"是矛盾的** ✗ ——
session 的库层只能有一份，而每个入口需要的前缀**各不相同**。

**唯一自洽的形状**：**按"库层前缀的等价类"分组** ——
前缀相同的入口共用一个 session；前缀不同的入口各用各的 session。
即：**`key = 该入口的库层单元序列`** ⇒ 同 key 的入口合并进一次 session ⇒
库层**按 key 只编一次**（而不是"全课只编一次"）。

* **收益**：42 个入口若归成 N 个等价类 ⇒ 库层编 **N** 次（而不是 42 次）✓；
  真课程里 `lib/*` 只有 8 个模块、入口的闭包前缀**高度重叠** ⇒ N 很可能远小于 42；
* **不破前缀**：同一 key 内的入口，库层前缀**逐字节相同** ⇒ 语义与基线一致 ✓；
* **下一步第一步**：**量出 N**（`grep`/脚本算每个入口的库层单元序列并去重）——
  **这是一条只读测量，不需要改代码** ⇒ 先量 N，再决定这条值不值得做。

## 28. **决定性测量：等价类 N = 15**（只读，实测；回答"分组值不值得做"）

§27.2 说"按库层前缀的等价类分组"。**先量 N**（用 `query project` 的 `modules[].entry`
标志 —— 它直接告诉我们谁是库、谁是入口；`SOKONANODA_BIN` 跑 41 个入口）：

```
entry files: 41
distinct library-prefix sequences (N): 15
  n= 11  len=2  (lib.Logic, lib.Set)
  n=  5  len=3  (lib.Logic, lib.Exists, lib.Set)
  n=  3  len=1  (lib.Logic)
  n=  2  len=4  (lib.Logic, lib.Exists, lib.Set, lib.Fun)
  …（其余 11 类各 1–2 个入口）
```

| 读数 | 今天 | 按等价类分组后 | 倍率 |
|---|---|---|---|
| **库模块编译次数** | **132** | **58** | **2.28× ↓** |
| **session 趟数** | **56** | **56** | **1.00×（无变化）** |

### 28.1 ⇒ **分组能省的是"库模块编译次数"，省不了"趟数"** —— 而 4.14× 的账要重算

* 今天 **132** 次库模块编译 = **4.14×** 那个数的来源之一（41 入口 × 平均 3.2 个库）；
* 分组后 **58** 次 ⇒ 库层那部分**省 2.28×** ✓；
* **但 `session` 趟数一点没省**（56 → 56）—— 因为**每个入口仍要自己那一趟**。

⇒ **分组是"真收益但中等"**（省的是库层重复，不是入口趟）。

### 28.2 ⚠ 更重要的对照：**§25 那次的 `passes` 4126→271（15×）不是分组带来的**

§25 第一次尝试**没有分组**（用的是并集 session），却把 `passes` 打到 **271**。
⇒ **那个 15× 来自"库层只编一次 + 入口趟只编自己"**，**不是**来自"等价类分组"。
而它崩/错的原因是**报告拼接**（§25/§26）与**并集前缀**（§27）——
**也就是说：真正的 15× 收益被两个工程细节挡住了，不是被"N 太大"挡住**。

### 28.3 ⇒ 下一步（唯一，且已有 15× 的证据支撑）

**先修 §27 的并集前缀问题**（这是 15× 的拦路虎），**而不是**去做 §27.2 的分组
（分组只值 2.28×，且不能替代前者）：

* **并集前缀的修法**：session 的 `lib_units` **不能是并集** ——
  但 session 只有一份库层 ⇒ **必须按等价类分组**才能"每份库层 = 某入口自己的前缀" ✓
  ⇒ **分组不是可选项，而是"让 session 与 per-entry 前缀自洽"的**唯一**办法** ✗；
* ⇒ **§27.2 的分组与"修并集前缀"是同一件事**（我上面把它们当成两件事，判错了）：
  按等价类分组 ⇒ 类内库层前缀**逐字节相同** ⇒ 既省 2.28×、又保住语义 ✓；
* **N=15 完全可接受**（15 个 session 而不是 1 个或 42 个）；
* **判据**：`--json` 逐字节不变（`build.decl` **2647** · `compiled:42 failed:0`）·
  `passes` 下降 · 反向判据 · 删 `#[ignore]` 后正向守卫转绿。

## 29. 🎯 **三次接线失败的同一个根因找到了**（接口对账，2026-09-29 17:0x）

按值守规则①（**动手前先做接口对账**）逐条核对后，找到了**三次都栽在同一处**的根因 ——
**不是报告拼接、也不是并集前缀，而是：session 的入口趟拿不到"闭包前缀"** ✗。

### 29.1 对账结果（逐条，全部读代码确认）

| # | 项目 | 基线（`compile_plan_with_progress`）| session 的入口趟 | 一致？ |
|---|---|---|---|---|
| 1 | `units` 传给谁 | `compile_all_units_with_progress(&units, …)`，`units` = **整个闭包**（`lib/*` + 入口）| `run_pass_with(builder, …, **entry_units**, …)` = **只有入口** | ✗ |
| 2 | `closure_prefixes` | `units.len() > 1` ⇒ **按拓扑序累加每个单元的声明文本**（`check/mod.rs:973`）| `entry_units.len() == 1` ⇒ **`Vec::new()`（空）** | ✗ |
| 3 | 入口的 `prefix_src` | `Cow::Owned(format!("{deps}{own_prefix}"))`（`walk.rs:326-334`）⇒ **含库层** | 走 `_ =>` 分支 ⇒ **只有入口自己** | ✗ |
| 4 | `display: display_notations(units)` | `units` = **整个闭包** ⇒ 记法表覆盖全闭包 | `units` = **只有入口** ⇒ 记法表**不覆盖库层** | ✗ |

**⇒ 根因一句话**：`judge_infer` 只吃**源码字符串**（`extra_prefix` + `prefix_src`，
`judge.rs:949`，**没有环境参数**）⇒ 它必须**从源码重跑前缀** ⇒
session 的入口趟 `prefix_src` **不含库层声明** ⇒ 入口里任何"问库层声明的类型/宇宙"的
`judge_infer` 都**看不到它们** ⇒ 失败 ✗。

**这也解释了 §25 的怪现象**：第一次尝试 `passes` 只有 **271**（judge 几乎不跑）
—— 不是"省了"，是**入口的 judge 因为看不到库层而大量走不到**；随后表现为崩/失败。

### 29.2 ⇒ 修法（**唯一**，且这正是 goal 里那句"记法表覆盖整个闭包"）

**session 的入口趟必须拿到"该入口闭包"的前缀与记法表** —— 也就是 §21.2 里
"`lib_units` 只当库层"这件事**不够**：入口趟还需要
**`closure_prefixes`（按该入口闭包顺序）** 与 **`display_notations(该入口闭包)`**。

**两条实现路**（下一轮选一，**都已勘明**）：

* **路甲（改 session 签名，最小）**：`with_project_session` 的入口趟把
  **`lib_units ++ entry_units`** 作为 `units` 传给 `run_pass_with`（这样
  `closure_prefixes` 与 `display` 都自动覆盖全闭包 ✓），但**只走入口自己的命令** ——
  需要一个"跳过已编单元"的机制（`skip`/`TrustPlan` 已有类似能力，`run_pass_with` 的
  `skip: Option<&KernelFailed>` 与 `trust: Option<&TrustPlan>` 就是干这个的）；
* **路乙（改 `run_pass_with` 签名）**：显式加 `closure_prefixes: Option<&[String]>`
  与 `display: Option<&DisplayNotations>` 两个可选参数 ⇒ session 直接喂
  "该入口闭包"的那一份（**不改变 units**）⇒ 语义最清晰 ✓。

**⇒ 选路乙**（不动 `units` ⇒ 不碰"哪些命令被走"的语义 ⇒ 风险最小）。
**对账清单（下一轮动手前逐条勾）**：① `units` 语义不变 ✓ ② `closure_prefixes` 按
**该入口自己的闭包顺序** ③ `display` 覆盖**该入口闭包** ④ 报告拼接仍走
`merge_session_reports`（但**类内顺序一致 ⇒ 不需要重排**，见 §28.3）⑤ 可见性（`closure` 私有 ⇒
用 `assemble_from_session`）。

---

## 30. §3.C 开工前的**勘明**（2026-09-30）—— 含**一条被实测推翻的推断**

开工单 §3.C 要求「先勘"per-前缀 builder 池 vs judge 接收调用方 builder"两条路，
再定刀口」。结论：**两条路都够不着那 791 趟**，且**我据此推断的第三个刀口也是错的**。

### 30.1 那 791 趟走的是**主编译 pass**，不是增量会话（实测）

| 路径 | 入口 | `TRUSTED_PREFIX` 压栈？ | 本课程命中 |
|---|---|---|---|
| **主 pass**（`build` 走这条） | `compile_all_units_with_progress` → `run_pass` | ❌ 不压 | — |
| 增量会话（LSP 改文件） | `run_incremental` → `with_trusted_prefix` | ✅ 压 | **0** |

`SOKO_JUDGE_ENV_REUSE=1` 与 `=0` 的 `JUDGE_PREFIX runs` **完全相同（791/791）**、
`JUDGE_ENV_REUSE` **命中 0 次** ⇒ **既有的 T-K11 在 `build` 路上从不触发** ✓
（它只服务 LSP 增量会话，而 `build` 是冷编译、没有"上一次会话"可担保）。

### 30.2 两条候选的死因

* **① per-前缀 builder 池**：**判死** —— 与 §17 的 **B″ 同一死因**：`EnvBuilder`
  持有 `&'a ArenaRef<'a>` 而 `run_pass` **每 pass 新建 arena** ⇒ 跨 pass 复用即
  **悬垂/跨 arena 指针** ✗；要池化就得共用一个 arena ⇒ 等于重做 §12 的"换共享模型"。
* **② judge 收调用方 builder**：**借用可行**（§19.5 的 A-2：`with_env_scope` 给
  `(&Env, &mut EnvBuilder)`，`declars`/`notations` 与 `dag` 是不同字段 ✓），
  **但位置不成立**：调用方那个 builder 只覆盖到"当前命令之前"，而 judge 要的是
  "合成文档的前缀" ⇒ 省不掉"重新建前缀环境" ✗。

### 30.3 ⚠ **被实测推翻的那一步**（留档，别再走）

§30.2 之后我**推断**"真刀口 = ③-a：把 `run_pass` 的 arena 从'一 pass 一个'提升到
'一模块一个'"，并据此请求授权（属 §12 的"换共享模型"级别）。
**打开 `SOKO_JUDGE_STATS=2` 逐趟明细量了一遍之后，那个推断是错的** ✗ ——
真刀口不是 arena，而是"**judge 每次都从源码重编整个前缀**"。
⇒ **教训：从代码结构推出的瓶颈，必须用逐趟明细证实之后再动手** ✓
（省掉一次"换共享模型"级别的返工）。

### 30.4 真数字（全课程 · release · 冷缓存 · 1 job）

| 读数 | 值 |
|---|---|
| `passes` / `doc_passes` | **1157** / 26 |
| `judge_ms` | **117214 ms**（= `pass_total_ms` 285348 的 **41%**） |
| **`JUDGE_CALL`（`by` 路径）** | **265 趟 · 115935 ms** ← **真正的大头** |
| `JUDGE_INFER calls` | 71126 次，但只 **38530 ms**（均 541 µs，命中率 99%） |
| `JUDGE_PREFIX runs/bytes` | 791 / 43.3 MB |

⇒ ① **`by` 那 265 趟 = `judge_ms` 的 99%**（P1-b 已把 `judge_infer` 侧吃干净）；
② **arena 不是瓶颈**（每趟重编 38 KB 前缀）；③ 那 265 趟 **key 全不重复**
⇒ **加缓存没用** ✗ —— 要做的是"**同一份前缀别每次从源码重编**"。

### 30.5 真刀口：把 `TRUSTED_PREFIX` 接到**主编译 pass**

主编译路径有**更强**的事实可用：**judge 合成文档的前缀，就是"当前正在编的这个
文件的前缀"，而它刚在同一次 `run_pass` 里被逐条检查过** ✓。落地见 §31。


## 31. ✅ §3.C **已落地**（2026-09-30）：担保接到主编译 pass ⇒ `build` **2.65×**

§30.5 那条路**接通了，并且验证通过**。

### 31.1 形状（两处改动，都不动 arena ✓）

1. **`walk.rs`**：主编译 pass 每检查完一条命令，就把"**这条命令之前**已核的
   命令数"压进 `TRUSTED_PREFIX`（`with_trusted_prefix(idx, &Default::default(), …)`）
   —— 与 `run_incremental` **同一个机制** ✓；
2. **`judge.rs::check_synthesized`**：⚠⚠ **必须把 `before` 夹到 `prefix_commands`**
   （`before.min(prefix_commands)`）。

### 31.2 ⚠⚠ 第 2 条是**踩出来的**，不是设计出来的（本档最重要的教训）

**第一版直接传 `idx` 当 `before`** ⇒ 全课程 `--json` 里 **38 行不同**
（`compiled` 变 `failed`，**与 P1-b 第一次失败的签名一模一样** ✗）。

**根因**：两个**坐标系**不同 ——
* 调用方的 `idx` 是 **AST 命令序号**（**含** `import` 那条命令）；
* judge 的 `prefix_commands` 是**它自己那份前缀文本**解析出的命令数，而那份文本走
  `importless_source`（**去掉 `import` 行**）⇒ **有 `import` 就错位**。

**探针量到的**（`SOKO_JUDGE_ENV_PROBE=1`，只读、零行为变化）：
`exact=86 · overshoot=179`（`before` 恒比 `prefix_commands` **大 2**）
⇒ 不夹的话会**多担保 2 条命令**，而那 2 条正是追加的合成声明 `_soko_judge_k`
⇒ **判定声明根本没被检查** ✗✗。

**为什么夹是安全的**：`before >= prefix_commands` 只说明"调用方已核的**至少覆盖**了
前缀"（前缀文本是调用方文本的**前段** ⇒ 它的命令必然在 `[0, before)` 里 ✓）
⇒ 担保上界就是 `prefix_commands` 本身；夹完只会"少担保 ⇒ 多检查" ✓。

### 31.3 ⚠⚠ 影子档第一版也是**错的**（第二个教训）

影子档第一版拿 `Debug` 比**整份 `DocumentReport`** ⇒ **265/265 全判 diff** ✗，
而那是**假分叉**：`run_incremental` 的 `decls` **只含它这一段新查的**
（`_soko_judge_*`），`check_document_with` 的 `decls` 含**整个前缀**的声明
（实测 `trusted` 28,342 字符 vs `full` 224,204 字符，首个不同就在
`_soko_judge_0` vs `Exists`）⇒ 差的是**报告的范围**，不是**判定** ✗。

⇒ **修法：比"判据"不比"报告"** —— 调用方只读 `judgement_of(report, k)`
（`judge_pairs_uncached` 结尾那句）⇒ 影子就比它 ✓。
修后 **`shadow_same=265 · shadow_diff=0`** ✓。
（同 `AGENTS.md` 验证设计纪律第 1 条：**断言用户可见的结果**。）

### 31.4 读数（release · 冷缓存 · 1 job · 全课程 `build --json`）

| 读数 | 逃生门 `SOKO_JUDGE_ENV_REUSE=0` | **默认** | Δ |
|---|---|---|---|
| **墙钟**（各 3 轮） | 126.44 / 126.50 s | **47.74 / 47.77 / 47.79 s** | **2.65×** |
| `judge_ms` | 113,659 | **19,223** | **−83%** |
| `pass_total_ms` | 276,407 | **92,058** | −67% |
| `doc_passes` | 265 | **0** | judge 全走增量路 ✓ |
| `by_calls` | 18,973 | **18,973** | 不变 ✓ |
| `JUDGE_PREFIX runs/bytes` | 791 / 43.3 MB | **791 / 43.3 MB** | 不变 ✓ |

⚠ **`JUDGE_PREFIX runs` 不变是对的**：那两个计数器量的是"**从源码重编前缀**"的
趟数，而本档省掉的是"**在 judge 里再查一遍前缀**"—— 计数器没覆盖这一半
（**如实记账**，别把"计数没动"当成"没收益"）。

### 31.5 证据链（缺一不算，全部实测）

1. **影子档**（判据级，两条路都跑）：**`shadow_same=265 · shadow_diff=0`** ✓；
2. **红线**：`off` vs **默认**的 `--json`（剔 `build.tick`/`build.progress`）
   **逐行不同 0 行**（2691/2691）✓；
3. **反向判据**：`crates/front/tests/judge_env_vouch.rs` —— **去掉夹紧 ⇒ 判红** ✓
   （⚠ 夹具**必须是带 `import` 的多单元项目**：单文件夹具探针是
   `exact=3 · overshoot=0`，**复现不出那个 bug**，注入后判据照样绿 ⇒
   **反向验证失效** ✗ —— 这是实测踩出来的）；
4. **带开关跑完整 `gate` PASS** ✓（含 LSP 那 12.8 万次判卷与课程门禁）；
5. **错误路径**：`grade` 在必然判错的 `by` 块上给出**同一条诊断**、exit 1 相同 ✓。

### 31.6 开关（默认开，两个逃生门）

* `SOKO_JUDGE_ENV_VOUCH` = `0|off`（关这一档）· `shadow`（两条路都跑、返回整份）·
  **默认 `On`** ✓；
* `SOKO_JUDGE_ENV_REUSE` = `0`（关更底层的"复用前缀"开关，**压过上面的默认**）✓；
* `SOKO_JUDGE_ENV_PROBE=1`（只读取证：`exact/overshoot/clamped_bad/shadow_*`）✓。

---

## 32. **as-built 接口更正**（2026-10-05）：§2 的 `EnvView` 没落地，实际是 `InplaceEnv`

§2 设计的接口是 `EnvView { env: &Env, prefix_len }` + `ElabCtx.env_view`，
**全仓零出现** ✗（`grep -rn "EnvView" crates/` 只命中注释）。落地的是**另一形状** ——
本节按**当前代码**记，作为后续（**G-92 终局**）的设计基线 ✓。

### 32.1 实际接口（`crates/front/src/compile/elab.rs`）

```rust
pub(crate) struct InplaceEnv<'e, 'a> {
    pub builder: &'e mut EnvBuilder<'a>,   // 同一个 DAG ⇒ 指针同一性保住（§17 那条红线 ✓）
    pub known: &'e KnownTable,             // 名字解析表
}
pub(crate) enum InplaceFail { ElabBinder, ElabOperand, Kernel }
/// 在**活环境**上求类型文本（三步与慢路逐字对齐：造项 → elaborate → 求类型）。
pub(crate) fn infer_type_text_inplace(env, ctx, binder_srcs: &[(String, Expr)], operand: &Expr, n: usize)
    -> Result<String, InplaceFail>;
/// `judge_render_type` 的就地兄弟（差别在**项层面剥 binder** 再 pp）。
pub(crate) fn inplace_render_type(/* 同上 */) -> Result<String, InplaceFail>;
```

**调用形状**：判定点（`infer_type_text_with` · `level_hint_of_inplace` · `by` 的两处）
**先查缓存 → 未命中才试就地 → 答不出回退** ✓；`None` = 老路 ⇒ **逐字节回退** ✓。

### 32.2 为什么不是 §2 那个形状（三条，都是实测）

| §2 的假设 | 实际 | 后果 |
|---|---|---|
| 判定路径**只读**环境 ⇒ 挂一个 `&Env` 就够 | 就地路**还要写项**（`mk_lambda`/`mk_app`/`mk_const` 都是 `&mut self`）⇒ 读环境与写项**必须是同一个 builder**（§16.1） | 不能拆成"只读视图 + 另一个 writer" |
| `EnvView` 挂 `ElabCtx`、由 `judge_infer` 查 | 调用点**本来就在 `elab_expr` 里**、手里已经有 `&mut builder` ⇒ 就地判定在**调用点**做，**不进** `judge_infer` | `ElabCtx` 不需要新字段；`judge_infer` 保持"只吃源码字符串" |
| 失效判据用 `prefix_len`（§3） | 缓存键仍是**前缀哈希 + 选项**；就地答案写回**同一张**缓存 | `prefix_len` 从未实现（**没有它也不影响正确性** ✓） |

### 32.3 阶段划分（as-built 版，替代 §5 / §9.1 的表）

| 阶段 | 内容 | 状态 |
|---|---|---|
| **1** | 判定点接**就地环境**（`InplaceEnv`）—— 即 P1-a / P1-b | ✅ **已落地**（§0.1） |
| **2** | **`judge_pairs` 的合成编译复用调用方环境**（G-92 终局）—— 需要"同一 DAG 上既能写新项、又能读已落地声明"（§18 的 **K-2** = G-68 架构件），**或**给就地路加「文本 ⇒ AST」入口 | 🟡 **第 1/2 步已落**（`edddbbae` 形状 + `787997f0` 入口与文本⇒AST）· **真 provider 未接**（等 `elab.rs` 收口）· 合成编译那一刀未做（§0.2 #1） |
| **3** | 并行下的环境复用（§5 原表那一行） | ❌ 未动 |

**阶段 2 的判据（缺一不算）**：① G-92 repro 的 `by_calls` 比值 **< 3.0**（线性 ≈ 2.0）；
② 全课程 `--json` 与基线**逐字节相同**；③ **反向判据**（改前缀里的依赖 ⇒ 必须重算）；
④ 影子档 `diff=0` —— 比"**调用方读到的结论**"、**不比报告形状**（§31.3 的教训 ✓）。

⚠ **阶段 2 的精确阻塞（2026-10-05 二次勘明 ✓）**：**内核侧使能件已经齐了** ✓ ——
`with_env`（借出**真 DAG** 的 `ExportFile`）· `with_env_scope`（`(&Env, &mut EnvBuilder)`，
**同样是零调用** ✗）· `run_pass_with`（**收调用方拥有的 builder** ✓，session 在用）；
缺的是**所有权**：`run_pass_with(builder: EnvBuilder, …) -> (…, EnvBuilder, …)` **按值**收发，
而 judge 在 `elab_expr` 链里只有 `&mut`（§12）⇒ 要么让 pass **借** builder，要么让判定点先**放手**（§30.2；另一条出路见 §0.2 #1）。
**归属**：内核线已登记本片为**下一片**（`aa3a6ef8` · 用户 2026-10-05 指定）。
