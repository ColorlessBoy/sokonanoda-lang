# 设计：**模块级产物 + 内容寻址**（对齐 Lean `.olean` / Coq `.vo`）

> 用户 2026-09-28 22:54：「project 的编译方案对齐业内标准」「**这不是批编**（把源文本合起来一起跑）；
> **这是结果复用**（每个模块的结果被存下来、被别人直接拿来用）」。
> 本文是**今晚的交付①**：产物存什么 / key 怎么算 / 失效怎么判 / **要动哪些文件**。
> 缺口 **G-68**（`docs/gaps/ledger.jsonl`）· 复盘 `docs/perf/recompile-waste-retrospective-2026-09-28.md`。

## 1. 与"批编"的分界（**别混**）

| | 批编（09-25，已否决 ✗） | **模块级产物（本文 ✓）** |
|---|---|---|
| 做什么 | 把 N 个 unit 的**源文本合成一趟**跑 | 每个模块**各跑各的**，跑完把**结果**存下来 |
| 环境 | **一个**大平坦环境（⇒ 重名撞车、前缀暴涨） | **每模块自己的环境**（⇒ 无撞车、前缀不变） |
| 缓存粒度 | 整趟（一处变 ⇒ 全趟重来） | **每模块**（一处变 ⇒ 只有它 + 下游失效） |
| 实测 | 真课程切片 6,026ms vs 970ms（**慢 6.2×**） | 目标：模块编译次数 **174 → ~42** |

## 2. 产物存什么（**核心**）

一个模块 M 的产物 = **编译 M 所需的全部状态**，分三块：

| 块 | 内容 | 为什么必须有 |
|---|---|---|
| **A. 内核环境** | M 的声明（名字 → 类型/值 + 宇宙/归纳块记账）在**内核里的注册状态** | 入口要在它**之上**继续 elaborate/检查 —— 今天缓存存的 `{report, output}` **给不了这个**（这正是 `.olean` 与我们的差别） |
| **B. 前端表** | `known` / `inductives` / `defs` / 记法表 / `GoalTemplates` 的来源（命令表） | 入口的记法消解、隐式参数、`by` 引擎都要看依赖的声明 |
| **C. 报告/事件** | `ModuleReport` + `CompileOutput`（今天缓存已有） | 不重编也能回答 UI（Infoview / `query` / 课程门禁） |

**内存 vs 磁盘（两刀，别一次做完）**：
* **刀 1（今晚的切片）· 进程内**：产物 = **同一 arena 里的一份环境分叉**（`EnvBuilder` fork）。
  一次 `build <dir>` 里每个模块只编一次、入口从依赖的**分叉**上起跑 ⇒ 修 `rebuild`（用户抱怨的那条）。
* **刀 2（P3）· 磁盘**：把 A/B 序列化（Lean 的 `.olean`）⇒ 跨进程复用（LSP 重启、`lake exe cache get` 式的下载）。
  **依赖刀 1 的表示定型**，先不做。

## 3. key 怎么算（Merkle，与今天的 `digest` 同形、粒度不同）

```
key(M) = H( format, 编译器版本, build stamp, prelude 模式,
            源文本(M), [ name(D), key(D) for D in 直接 import ] )
```
* 与今天 `ProjectPlan::digest`（`crates/front/src/project/cache.rs:20-33`）**同一个哈希函数、同一条 Merkle 链**，
  只是**每个模块一条键**（今天是把整条闭包折成一条键 ⇒ 42 个入口各存一份 ⇒ 共享依赖各编一遍 ✗）；
* **依赖变 ⇒ key(D) 变 ⇒ 所有下游 key 变** ⇒ 天然不会错编（这条是红线，见 §5 判据）；
* 单文件（无 import）键**逐字节不变**（A1 纪律）。

## 4. 失效与回滚

| 情形 | 处置 |
|---|---|
| 依赖源文本变 | 它的 key 变 ⇒ 下游全 miss ⇒ 重编（**必须**，§5 有反例判据） |
| 模块自己有诊断 | **不入库**（沿用 `store_if_clean*`，T-A05 已把判据收成一处 ✓）——半成品绝不进产物 |
| 产物损坏 / 格式不符 | 当作不存在（`CACHE_FORMAT` + `meta.json` schema 两道校验，今天已有） |
| 版本 / build stamp 变 | 全 miss（今天已有：`CARGO_PKG_VERSION` + `build_stamp()`） |
| 部分失败 | 失败的模块**不写**产物；它的下游按"缺依赖"照常报错，**不许复用上一次的环境** |

## 5. 判据（缺一不算做成）

1. **正确优先**：**改依赖 ⇒ 必须 miss 且重编** —— 造一个反例（改 `lib/Shared` 一行 ⇒ 入口必须重编，`--json` 不得命中旧产物）；
2. **数字**：`python3 scripts/check-recompile-factor.py` —— 3 入口共享 1 依赖的 `by_calls` **3 → 1**；
   真课程 **174 → ~42**、rebuild **222.1s → ?**（同一台机器、同一命令，改前数字已在 `docs/perf/rebuild-baseline-2026-09-28.md`）；
3. **三件套**：全语料对拍 · `--json` **逐字节不变** · 课程门禁 **0 判负**；
4. **诊断/声明语义不变**：③ 的逐字节就是它的守卫。

## 6. 要动哪些文件（清单）

| 文件 | 改动 | 备注 |
|---|---|---|
| `crates/kernel/src/builder.rs` | **新增 `EnvBuilder::fork()`**（把 `dag`/`declars`/`notations`/`mutual_block_sizes` 深拷贝进**同一个 arena**，得到一个可继续 `add_declar` 的新 builder） | ⚠ **内核改动**：`EnvBuilder` 今天**没有 `Clone`**（`:26`），`snapshot()` 只给**只读**副本（`:75`）；T-K12c 死因 = "往独立环境 `add_declar` 改写共享 `decl_idx` 槽位" ⇒ fork 必须**深拷贝**（不共享槽位）。**纯能力新增、不改任何判定路径**，但按硬规矩需要授权（走 `kernel/*` 分支或 main 白名单） |
| `crates/front/src/project/session.rs`（新） | `ProjectSession`：按拓扑序**每模块编一次**；每个入口**从依赖分叉**起跑、只编自己的命令 | 09-23 的 K2 设计（`docs/design/closure-incremental.md:106-107`），当时"实现未做" |
| `crates/front/src/project/cache.rs` | 键改成**每模块一条**（Merkle）；保留入口级条目（`--json`/LSP 的现有消费者不变） | 今天 `plan.digest` = 整条闭包 |
| `crates/front/src/compile/check/{mod,walk}.rs` | "**从分叉环境起跑**"的入口（今天 `run_pass` 总是新建 `EnvBuilder`）；`walk` 的 `known`/`inductives`/`defs` 也要能从分叉点恢复 | 分叉点 = 模块边界（命令下标） |
| `crates/cli/src/build.rs` | `build <dir>` 走 session（**替换**逐文件循环） | `--json` 事件形状不变（additive：`build.decl`/`build.tick` 已在 P2 落地 ✓） |
| 判据 | `crates/front/tests/`（等价 + 依赖变必 miss 反例）· `crates/cli/tests/`（`--json` 逐字节）· 课程门禁 | 见 §5 |

## 7. 今晚的切片（**先证 3 → 1，再谈 42**）

1. 夹具：3 个入口共享 1 个依赖（`scripts/check-recompile-factor.py` 的同一夹具）；
2. 期望：`by_calls` **3 → 1**（依赖只 elaborate 一次；第 2、3 个入口**加载产物**，不重编）；
3. **反例**：改依赖一行 ⇒ `by_calls` 回到 3（**必须重编**）；
4. 过了 ⇒ 再上真课程，量 **174 → ?** 与 **222.1s → ?**。

**卡点（如实说）**：切片第 1 步就落在 §6 第一行 —— **`EnvBuilder::fork()` 是内核改动**。
在授权之前，前端侧（`ProjectSession` 骨架、模块级 key、判据）可以先写，但**跑不出 3 → 1**。
⇒ 按用户规矩：**停下来报授权**，不硬推。

## 8. 刀 2（= P3）：**磁盘产物 + 可下载**（本节**只做设计**，不动手）

用户 2026-09-28：「P3 可下载编译 cache（从 GitHub 拉）+ 外部项目库源码下载：**只做设计**
（缓存键 / 失效 / 完整性与安全校验 / 离线降级）」。业界对应物 = 官方构建工具的远程预编译产物缓存。

### 8.1 存哪、叫什么
`<模块根>/.sokonanoda/artifacts/<key>.bin`（自忽略；`key` = §3 的 per-module Merkle 键）
+ `<key>.meta.json`（`{format, 编译器版本, build stamp, target triple, prelude 模式, 字节数, 产物自身摘要}`）。
**文件名即内容寻址** ⇒ 同一模块在不同项目/机器上落到同一个名字，天然可共享（与 P3 的"下载"直接对齐）。

### 8.2 完整性（先于使用，失败**当不存在**）
三道，全过才用：① `meta.json` 的 `format/版本/build stamp/target/prelude` 与本机**逐项相同**；
② 产物字节的摘要 == `meta.json` 里记的摘要（**截断/损坏的下载**在这里挡掉）；
③ 键可重算：本机按 §3 用**磁盘上的源文本**算出 `key(M)`，与文件名一致 ⇒ 产物**只可能**对应
我们手上这一份源文本（源文本变一个字 ⇒ 键变 ⇒ 旧产物再也匹配不上）。
任一条不过 ⇒ **照常本地重编**（绝不"猜着用"）。

### 8.3 安全模型（**如实写清，不粉饰**）
⚠ 加载产物 = **把声明直接注入内核**（等价于官方工具链加载编译产物）⇒ 一个**伪造的产物**
可以塞进不成立的声明 ⇒ **错编**。而"内容寻址"**挡不住这个**：键由**输入**算出，
不校验产物**内容**是否真的由这份输入编译而来（能挡的只有 §8.2 的损坏与错配）。
⇒ 因此：
* **默认关**（设置项默认 `false`）；打开需用户显式同意；
* 只从**固定发布者**拉（仓库/课程仓的 GitHub Release，**按 tag 锁定**，禁 `latest` —— 与现有下载纪律同源）；
* 传输 HTTPS + 清单**签名**（清单里记每个 `key` 的产物摘要 ⇒ 伪造需要同时伪造签名）；
* **不静默**：用了下载产物就在 build 输出里报一行来源（谁给的、哪个 key）；
* 教学场景的兜底：课程仓可把"允许的产物清单"钉进仓库（`artifacts.lock.json`），
  与版本钉文件同一条"按版本钉"的纪律。

### 8.4 离线降级（**永不阻塞**）
下载失败 / 超时 / 离线 ⇒ **静默退回本地编译**（只是慢，不是错），并记一条
`artifacts: offline (N 个模块本地重编)`；`SOKONANODA_OFFLINE=1` 直接跳过网络。
"外部项目库源码下载"同一条：源码拉不到 ⇒ 报缺依赖（今天的行为），**不发明新错误形态**。

### 8.5 与刀 1 的关系
刀 2 **依赖刀 1 把产物的表示定下来**（§2 的 A/B 两块）；刀 1 只做进程内 fork 时，
磁盘格式还没定型 ⇒ **先刀 1、后刀 2**。两刀共用**同一条键**（§3）⇒ P3 不是第二套缓存，
只是同一条键的**第二个来源**（本地编译 vs 下载）。

## 9. 切片 1b：**一个 builder 贯穿全场**（跨 builder 播种已被证否）

**改后的做法（全部前端，内核零改动）**：
1. **一个 session arena + 一个 `EnvBuilder`** 贯穿整次 `build <dir>`（arena 提升 = 切片 1a ✓ 已合入）；
2. 先编**共享库层**（各 `lib/*` 各一次），在库层边界留一份 **`declars` 检查点**
   （`hide_declars()` 取走、`Clone` 留一份 —— `Clone` 由 `snapshot()` 已在用，不必命名那个 `pub(crate)` 类型）；
3. 每个入口：`restore_declars(库层检查点)` ⇒ 只走**入口自己的命令** ⇒ 编完 `hide_declars()` 丢掉入口的声明；
4. ⇒ **单元之间从不共处一个环境**（09-25 假"重复声明"的结构性根因消失），
   **前缀也不膨胀**（每个入口的前缀仍是它自己的闭包），而**共享库只编一次**。

**判据（`crates/front/tests/module_reuse.rs`）**：① 复用路径 vs 今天逐入口路径的报告/事件
**逐字节相同**（错编红线，唯一验收口径）· ② `by_calls` **3 → 1**（`check-recompile-factor.py`
同口径；**改前实测 = 3**）· ③ **改依赖一行 ⇒ 必 miss 重编**（反例）· ④ 入口顺序交换结果不变 ·
⑤ 真课程 **174 → ?** / **222.1s → ?**。
**待实验回答的唯一未知**：walk 的前端表（`known`/`inductives`/`defs`）能否跨入口复用，还是必须逐入口重建
（① 会直接给出答案）。
**已落地的三刀（2026-09-28，全部实测行为零变化、内核零改动）**：
① `Walked.builder` 改借用 + `finish_pass` 用 `builder.with_env` **借出**环境（不再消费 builder）；
② 影子环境改 `Option<EnvBuilder>`（解开"影子重放主 arena 的 `Declar` ⇒ 两套环境被 `ops` 焊死"）；
③ **`run_pass_with(builder, shadow, …) -> (PassResult, EnvBuilder<'a>)`**（`'a: 's`；`units: &'a [SourceUnit<'a>]`；
`PassResult` 已 `pub(crate)`）。⚠ `Walk` 的生命周期方向必须是 **`<'arena: 'shadow, 'shadow>`**（主环境寿命 ⊇ 影子；
反过来 `walk.rs:230` 报 `lifetime may not live long enough`）。
判据：`clippy --all-targets` exit 0 · fmt ✓ · front lib **757 passed** · CLI imports **21 passed** ·
`check-recompile-factor.py` 仍 **`by_calls=3`**（这三刀不改行为）。

**下一步（只剩"用起来"，无结构未知）**：`ProjectSession`（新文件 `crates/front/src/project/session.rs`）——
session 持 arena + builder ⇒ 库层**编一次** ⇒ `hide_declars()` 留检查点 ⇒ 每个入口 `restore_declars(检查点)`
→ 用**该入口自己的 units** 调 `run_pass_with` → 编完 `hide_declars()` ⇒ 跑 ① 逐字节等价 → ② `by_calls` 3 → 1 → ③ 改依赖必 miss → ⑤ 真课程 174 → 42 / 222.1s → ?。
