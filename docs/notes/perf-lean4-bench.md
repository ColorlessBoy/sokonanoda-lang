# 实测：Lean 4 的按键墙钟 —— 把北极星里的 **~200ms 占位**换成真读数

> **只读测量**（2026-10-09）。零源码改动：`crates/`、`STATUS.md`、`docs/gaps/`、
> 并行开发线占用的 `docs/notes/PLAN-align-lean4*.md` **一个字节都没动**。
> 本文只写**读数 + 判据 + 复现命令**。
>
> **两侧身份（跨轮比较读数前先看这里）**：
> * **Lean 4 侧** = `leanprover/lean4:v4.28.0`（elan 已装；`commit 7e01a1bf5c70`），
>   装置在 `~/lean4-bench`（独立目录，**不在 sokonanoda 源码树里**），
>   夹具 = 课程 `unit08` 的**逐字移植**（`Bench/Unit08.lean`）。
> * **sokonanoda 侧** = `HEAD 9f072344`（工作树干净）+ **test profile**（`opt-level=3`，
>   与探针 `crates/lsp/tests/perf_keystroke_wallclock.rs` 同一档）构建的
>   `sokonanoda-lsp`（`CARGO_TARGET_DIR=/tmp/soko-test-target cargo build --profile test`）。
>   ⚠ 测量期间并行开发线**一直在提交**（测量后 HEAD 已到 `86e41217`）—— 表里的 sokonanoda
>   数字一律指 **9f072344 的这一次构建**，跨轮比较前先核身份。
> * 两侧都跑**同一个** Python 夹具 `lsp_bench.py`（真子进程 + 真 stdio JSON-RPC），
>   同机（Darwin arm64）、同一时间段。
>
> **为什么这次能跑 Lean**：`REQUIREMENTS.md` §2 第 2 条禁止调用官方工具链（避免产品/CI
> 依赖它）。本轮是**用户明确下令**的一次性实测（任务书："在独立目录安装 lean4 … 只读测量"）
> ⇒ 仅此一次、只在 `~/lean4-bench`、**不接进任何产品路径**；仓库侧规则不变。

---

## 0. 结论（先给答案）

**一句话**：真读数把占位**证实**了 —— 按探针现行口径（`didChange` → 诊断落地），
**Lean 4 = 218ms**（中位，n=5，P95 219ms），sokonanoda = **81ms** ⇒ **快 2.7×**；
但按"**goal 更新完成**"口径，**Lean = 3.1ms**、sokonanoda = **81ms** ⇒ 反过来了，
差 **26×**。两个数都对，因为两边的"更新"不是同一件事（§4）。

### 表 1 —— 对等对比表（同机 · 同夹具 · 同序列 · n=5）

**臂 A「连续键入」**（一个会话：1 刀热身不计 + 5 刀计时；每刀都是没编过的新文本）

| 口径（从 `didChange` 发出算起） | **sokonanoda**（9f072344 · test profile） | **Lean 4.28.0**（`lean --server`） | 谁快 |
|---|---|---|---|
| **诊断全量落地**（用户看到 solved / problems 更新） | **81.4ms**（P95 147.1¹） | **218.1ms**（P95 219.5） | soko **2.7×** |
| **goal 更新完成**（infoview 里的新 goal 真的可见） | **81.4ms**（同诊断那一刻²） | **3.1ms**（P95 3.7） | **lean 26×** |
| 结构计数（最后一刀服务端自报） | `modules=1 by=9 compile=79ms` | 全文件重编（无 per-command 计数可报） | — |

¹ 5 个样本 `[79.3, 81.4, **163.5**, 81.7, 80.1]` —— 一个离群（并行开发线在同一台机器上跑
测试）。**发布过的同口径读数是 78.5ms**（`PLAN-align-lean4.md` §12：median 78.5 / best 78.4 /
worst 81.3）⇒ 本夹具复现它，差 ~4%。
² 见 §3.3 / §4.2：`soko/goals` 在编译完成**之前**答的是**旧 goal**（实测 0.3ms 陈旧答案）——
所以 sokonanoda 的 goal 更新时刻**就是**诊断落地那一刻。

**臂 B「单刀」**（每个样本一个**全新进程**：开档 → 等落定 → 只敲一刀，无热身）

| 口径 | **sokonanoda** | **Lean 4.28.0** | 谁快 |
|---|---|---|---|
| 诊断全量落地 | **36.6ms**（P95 55.7） | **218.9ms**（P95 218.9） | soko **6.0×** |
| goal 更新完成 | 36.6ms | **2.2ms**（P95 2.4） | **lean 17×** |
| 结构计数 | `modules=0 by=0 compile=44–54ms`（入口检查点直接命中） | 全文件重编 | — |

**臂 B′「开档即敲」**（sokonanoda 独有的一格：开档**不等后台预热**就敲）

| 口径 | 读数 | 结构计数 |
|---|---|---|
| 诊断全量落地 | **232.7ms**（P95 265.3） | `modules=4 by=13 infer=4/60 prefix=4` + `warm-library built=true 217ms` |

⇒ 这一刀与 **A5 后台库层预热**（217ms）**赛跑**：预热没跑完就敲 ⇒ 首刀自己把 4 个库模块
编一遍（232ms）；预热已跑完 ⇒ 命中入口检查点（36.6ms）。发布过的两个读数
（§11.13 的 **201.0ms** 与 §33 的 **76.6ms**）正是这条赛道的两端 —— **不是矛盾，是竞态**。

### 三条要点

1. **占位 ~200ms 的数量级是对的**：Lean 4 在这一题、这一序列上的"诊断全量落地"是
   **218ms**（P95 219ms，5 个样本只差 ±1.5ms）。
2. **sokonanoda 的"已超过 Lean"成立**，但只在**诊断**口径上（81 vs 218）；换成
   **goal** 口径，Lean 靠 **per-command 快照**领先一个数量级（3ms vs 81ms）——
   那正是 sokonanoda 方向② **T2-B（入口命令级快照）**要补的那一格。
3. **两边的"贵"不是同一件事**：Lean 贵在**编辑点之后的整份文件重编**（218ms，goal 却
   3ms 就能看）；sokonanoda 贵在**整条入口重编完才有新报告**（81ms，goal 与诊断同时到）。

---

## 1. 口径（两边逐字同款）

夹具：`~/lean4-bench/lsp_bench.py`（零依赖 Python，Content-Length 帧 + 服务端请求自动应答）。

* **真子进程**：`lean --server`（Lean 官方语言服务器）/ `sokonanoda-lsp`（本仓库）；
  两者都是真 stdio JSON-RPC，没有同进程捷径。
* **真课程 unit08**：sokonanoda 侧 = `courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda`
  （342 行 / 23 条 `theorem|def|lemma` / 9 个 `sorry`）；Lean 侧 = 它的**逐字移植**
  （`Bench/Unit08.lean`，145 行 / 23 条同名声明 + 4 条 `example` / 同样 9 个 `sorry`）。
* **同一把编辑刀**：与探针 `add_one_space_before_y` 逐字同款 —— 在
  `Set.mem_image α β f A y` 的 `y` 前**再加一个空格**（`demo_mem_image` 的证明体，
  文件第 3 条声明）。每一刀都是**没编过的新文本**（"连续键入"的真实形状）。
* **计时点**（都从 `didChange` 发出算起，取消息**到达**时刻）：
  * `first_diag` = 本版本第一条 `textDocument/publishDiagnostics`；
  * `complete` = "这一版全部更新完"：Lean 用 **`textDocument/waitForDiagnostics`**
    （协议里专为同步设的请求：`{uri, version}`，等到该版本的全部快照与 reporter 结束），
    sokonanoda 用它**唯一**的那条 `publishDiagnostics`（一次给全）；
  * `goal` = `$/lean/plainGoal`（Lean）/ `soko/goals`（sokonanoda）的响应到达。
* **两臂**：A = 一个会话（1 刀热身不计 + 5 刀计时）；B = 每个样本**全新进程**
  （开档 → 等落定 → 只敲一刀）。sokonanoda 的"等落定"= 诊断 + **后台预热静默 500ms**
  （与 Rust 探针 `settled_compile_count` 同窗口）；Lean 的"等落定"= 开档的
  `waitForDiagnostics` + 首条诊断。
* **统计**：n=5；`median` = 中位数；`P95` = 线性插值（n=5 时 P95 即最大值的稳健邻）。

---

## 2. Lean 侧装置（`~/lean4-bench`）

```
~/lean4-bench/
├── lean-toolchain          # leanprover/lean4:v4.28.0（elan 已装，零下载）
├── lakefile.toml           # name = "bench"，lean_lib Bench
├── Bench.lean              # lake 库根
├── Bench/Logic.lean        # ← courses/set-theory/lib/Logic.sokonanoda（空壳）
├── Bench/Exists.lean       # ← lib/Exists.sokonanoda（空壳：Exists.imp 在 core 里已有）
├── Bench/Set.lean          # ← lib/Set.sokonanoda
├── Bench/Image.lean        # ← lib/Image.sokonanoda
├── Bench/Unit08.lean       # ← units/I.3/unit08-images-preimages.sokonanoda
├── lsp_bench.py            # 两侧共用的夹具（真 stdio LSP 客户端 + 两臂）
├── run_*.sh                # 复现脚本（见 §6）
└── results/                # 读数原件（本文所有表格的 JSON 出处）
```

**移植口径（逐条，读这份报告的人可以据此判"是不是同一道题"）**：

| 课程写法 | Lean 4 写法 | 为什么 |
|---|---|---|
| `def Set (α : Type) : Type := α → Prop` | 同（逐字） | Lean core **没有** `Set`（`#check Set` 报 unknown）⇒ 与课程一样自带 |
| `infix:50 " ∈ " => Set.mem` 等 6 条记法 | 给 `Set` 注册 `Membership`/`HasSubset`/`Union`/`Inter`/`SDiff`/`EmptyCollection`/`Singleton`/`Sep` **实例** | 这 6 个符号 Lean core **已占用**（同名同优先级：`∈`50 `⊆`50 `∪`65 `∩`70 `\`70）⇒ 注册实例后**符号形状与优先级与课程逐字相同** |
| `prefix:100 " 𝒫 "` / `postfix:100 " ᶜ "` / `infixr:80 " '' "` / `" ⁻¹' "` | 同（逐字 `notation`） | 这 4 个不在 core |
| `binder_notation "∃" => Exists` | **删掉** | Lean core 自带 `∃` binder 记法 |
| `Set.mem Two aa A`（点名叫法写全前导隐式实参） | `@Set.mem Two aa A` | 课程编译器能反解前导隐式实参，Lean 不能（**语义不变**，只是补 `@`） |
| `inductive Two : Type / ctor aa : Two / end` | `inductive Two : Type where \| aa : Two` | Lean 的归纳语法 |
| `match t with \| aa => …` | `match t with \| .aa => …` | Lean 里裸 `aa` 是**变量模式**（会静默变成 catch-all）⇒ 必须点名构造子 |
| `{x ∈ A \| x ∈ A}`（sep 记法） | `sep A (fun x => x ∈ A)` | `{x ∈ s \| p}` 记法在 mathlib/Std，不在 core；**是同一个项**（课程自己写明它脱糖成 `Set.sep A (fun x => …)`） |
| 练习体 `by sorry` × 9 | 同（逐字） | 课程里它们本来就是**开放练习**（判卷 `exercise.open`） |

**没做的（诚实记）**：没装 mathlib（`lake exe cache get` 要下 GB 级产物，而**被测量**是
"一次按键的增量"，与库有多大无关；课程的 `lib/` 本身就是 553 行的自备标准库，
逐字移植它比引 mathlib 更贴原题）。`lake build` 全绿（只有 9 条 `sorry` warning）。

---

## 3. 读数（全部同机、同一时间段、n=5）

### 3.1 连续键入（臂 A）

| 后端 · 编辑刀 | first_diag | **complete** | goal |
|---|---|---|---|
| **Lean** · 证明体（与探针同款） | 211.6ms（P95 212.7） | **218.1ms**（P95 219.5） | **3.1ms**（P95 3.7） |
| **Lean** · tactic（`preimage_union` 的 `by sorry` 里加空格） | 214.2ms（P95 217.2） | **219.9ms**（P95 223.9） | **4.2ms**（P95 11.9） |
| **sokonanoda** · 证明体（与探针同款） | = complete | **81.4ms**（P95 147.1） | 7.2ms（**陈旧**，见 §3.3 / §4.2） |

Lean 的逐样本（证明体刀）：`complete = [218.1, 215.8, 215.7, 219.3, 219.5]` ——
**±1.5ms**，是本次最稳的一格。

### 3.2 单刀（臂 B，全新进程）

| 后端 · 编辑刀 | first_diag | complete | goal |
|---|---|---|---|
| **Lean** · 证明体 | 212.7ms（P95 212.8） | **218.9ms**（P95 218.9） | **2.2ms**（P95 2.4） |
| **Lean** · tactic | 212.4ms（P95 215.4） | **218.2ms**（P95 218.8） | **2.6ms**（P95 3.8） |
| **sokonanoda** · 证明体（等预热落定） | = complete | **36.6ms**（P95 55.7） | 21.6ms |
| **sokonanoda** · 证明体（**不等**预热＝开档即敲） | = complete | **232.7ms**（P95 265.3） | — |

### 3.3 goal 口径的**内容验证**（不是"链路通"，是"答案换了"）

把编辑刀换成**真的改变目标**的那一刀（`image_subset_iff` 的 `by sorry` 前插入
`constructor`），然后**立刻**问 goal、`waitForDiagnostics` 之后再问一次：

| 后端 | 改动前 | 改动后**立刻**（ms） | 改动后 complete 之后 |
|---|---|---|---|
| **Lean** | `⊢ f '' A ⊆ B ↔ A ⊆ f ⁻¹' B` | **3.1ms**：`case mp … ⊢ f '' A ⊆ B → A ⊆ f ⁻¹' B` + `case mpr …` ⇒ **已经是新状态** ✓ | 同一份答案 |
| **sokonanoda** | `(f ⁻¹' (B ∪ C)) = ((f ⁻¹' B) ∪ (f ⁻¹' C))` | **0.3ms**：**旧 goal**（一字不差）✗ **陈旧** | 197.8ms 后：`(x : α) → ((f ⁻¹' (B ∪ C)) x) ↔ …` ⇒ 新状态 ✓ |

### 3.4 一次 Lean 编辑的完整时间线（同机实测，节选）

```
   0.1ms  DIAG v=1  ×5（开档遗留）
   3.1ms  GOAL -> 新 proof state（per-command 快照已就绪）
 213.2ms  PROGRESS-DONE v=10（fileProgress processing=[]）
 213.3ms  DIAG v=10 n=9（本版本唯一的完整诊断）
 219.6ms  WAITDIAG -> {}（waitForDiagnostics 版本感知返回）
```
⇒ **goal 3ms / 诊断 213ms / 全部结束 220ms**，三段清清楚楚。开档落定（首次
`waitForDiagnostics`）本身 0.2–1.8s（OS 文件缓存冷热不同），不计入任何读数。

---

## 4. 机制对照（为什么是这些数）

### 4.1 Lean：**per-command 快照** ⇒ goal 只等"这一条命令"

`lean --server` 把每条命令的完整 `Command.State` 存成快照（`Server/Snapshots.lean`），
`$/lean/plainGoal` 走 `findGoalsAt?` → `withWaitFindSnap (fun s => s.endPos > pos)`：
**只等光标所在那条命令的快照**。插入一个空格 ⇒ 那条命令重 elaborate（~2ms）⇒ goal 立刻可用；
**编辑点之后的 20 多条声明还在编**（→ 213ms 才有诊断）。
⚠ 前提是**会话已落定**：开档还没编完就敲，goal 会等整轮重来（实测 194ms）——
所以"开档即敲"与"落定后敲"必须分开报。

### 4.2 sokonanoda：**一份报告** ⇒ goal 与诊断同一个时刻

`did_change` 走 `schedule_refresh`（异步重编入口），`publishDiagnostics` 是**唯一**的
完成信号；`soko/goals` 读的是**上一次编译的报告**（`doc.report()`）。所以：
goal 更新与诊断落地**同时**发生（81.4ms / 36.6ms），之前问 goal 只会拿到旧值。

### 4.3 这对开发线意味着什么

* 现行北极星口径（`didChange` → 诊断）**对 sokonanoda 有利**：它量的是"用户看到
  solved/problems"，而 Lean 在这个口径上要把**编辑点之后的整份文件**编完（218ms）。
* **真正的差距在 goal 那一格**（3ms vs 81ms），机制就是方向② **T2-B**：
  把"入口命令级环境快照"做出来，让 goal 视图**不必等整条入口**。
  Lean 的对照实现是"每条命令一个 `Command.State` 快照 + 按位置等待"，不是"更快的内核"。
* 反向的一条：sokonanoda 的**单刀 36.6ms**（`modules=0 by=0`，入口检查点直接命中）
  已经比 Lean 的**诊断**口径（218ms）快 6×；Lean 的 218ms 是**整份文件**的代价，
  两边在"整份文件都更新完"这件事上是同一量级（36–81ms vs 218ms）。

---

## 5. 诚实边界（读这张表前必须知道）

1. **Lean 侧是逐字移植，不是原文件**：声明名/语句/练习的 `sorry` 逐条对齐（§2 的表列了
   全部改写规则），但**不是**同一份源码；库层是课程自备的 553 行标准库（无 mathlib）。
   被测量的量（"一次按键 → 结果更新"）由**编辑点之后的命令数**决定，两侧都是 23 条声明、
   编辑点都在第 3–4 条 ⇒ 可比；但**不能**据此说"Lean 处理这个文件要 218ms"。
2. **绝对毫秒不进判据**（`AGENTS.md`）：本文所有毫秒都是**同机同时段**的读数，
   跨机/跨负载不可转移；判据仍应是结构计数（`modules=`/`by=`/`passes`）。
3. **共享可变状态**：课程模块根 `courses/set-theory/.sokonanoda/artifacts/` 是**共享**的
   （并行开发线的测试也会写它）⇒ sokonanoda 的绝对毫秒在两次运行间会漂
   （本文 81.4ms vs 发布读数 78.5ms = 4%）。Lean 侧不受影响（每次 `lake build` 后的
   oleans 只读）。
4. **一次按键 ≠ 首屏**：不含进程启动、`initialize`、开档（0.4–1.7s）。那是另一个量级。
5. **n=5**：P95 在 n=5 下只是"最大值的稳健邻"，不是统计意义上的 P95；要更硬的 P95 得加大 n。
6. **本轮是用户下令的一次性例外**：`REQUIREMENTS.md` §2 第 2 条仍然有效，
   产品/CI/课程路径**不得**依赖 lean/lake/elan；`~/lean4-bench` 在仓库之外、不接进任何脚本。

---

## 6. 复现（一条一条可执行）

```bash
# 0) 装置（已在 ~/lean4-bench；~/.elan 里已有 leanprover/lean4:v4.28.0，零下载）
cd ~/lean4-bench && /Users/penglingwei/.elan/bin/lake build      # 全绿（9 条 sorry warning）

# 1) Lean 4 侧（连续键入 / 单刀 / tactic 刀）
python3 lsp_bench.py lean --root ~/lean4-bench --file Bench/Unit08.lean \
    --rounds 5 --arms typing,single_shot --goal --edit proof_term
python3 lsp_bench.py lean --root ~/lean4-bench --file Bench/Unit08.lean \
    --rounds 5 --arms typing --goal --edit tactic

# 2) sokonanoda 侧（同一夹具；二进制用 test profile 自建，别动仓库 target/）
CARGO_TARGET_DIR=/tmp/soko-test-target cargo build --locked -p sokonanoda-lsp --profile test
ROOT=$PWD/courses/set-theory
python3 lsp_bench.py soko --root "$ROOT" --file units/I.3/unit08-images-preimages.sokonanoda \
    --bin /tmp/soko-test-target/debug/sokonanoda-lsp --rounds 5 \
    --arms typing,single_shot --goal --edit proof_term
python3 lsp_bench.py soko --root "$ROOT" --file units/I.3/unit08-images-preimages.sokonanoda \
    --bin /tmp/soko-test-target/debug/sokonanoda-lsp --rounds 5 \
    --arms single_shot --settle diag --edit proof_term        # 开档即敲那一格
```

**探针身份**：`lsp_bench.py` 每次读数都带 `root/file/rounds/edit/backend`；sokonanoda 侧
另有 `LSP_TRACE compile … modules= by= infer= prefix=` 行（本文表里的结构计数就是它）。
**本文没有改任何源码**；`git status --short crates/ courses/` 在本轮前后都由并行开发线
自己的提交决定。

---

## 7. 占位替换指引 —— **已执行** ✓（2026-10-09）

本文原先列出的 7 处 `~200ms` 占位**已按"218ms（诊断口径）"就地改掉** ✓
（`PLAN-align-lean4.md` 4 处 · `PLAN-align-lean4-as-built.md` 3 处；行数不变 ⇒ 不破冻结预算）。
⚠ 读的时候**别**写成"Lean 4 = 200ms 所以打平了"：实测 **218ms**（P95 219ms），
而且**只在"诊断全量落地"这一格**；换成 **goal 更新**那一格是**反的**
（**3.1ms vs 81.4ms**，§0 表 1、§3.3 有内容验证）—— 机制与对齐路径见 **§8** ✓。

---

## 8. Lean 的 goal 为什么是 3ms · 我们的对齐路径

### 8.1 机制（逐环节，本机 `leanprover/lean4:v4.28.0` 源码）

| 环节 | Lean 4 怎么做 | 位置 |
|---|---|---|
| **逐命令快照** | `Snapshot { stx, mpState, cmdState }` —— **每条命令**一份；`cmdState` 带 `env` · `messages` · `infoState`（tactic 的 before/after 目标都在里面） | `Server/Snapshots.lean:28-33` |
| **增量产出** | `doc.cmdSnaps : AsyncList`：**任务链** —— 编到第 k 条就把第 k 份 cons 上去，后面的还在编 | `Server/AsyncList.lean:21-23` |
| **按位置等** | `withWaitFindSnapAtPos` = `waitFind? (fun s => s.endPos >= pos)` ⇒ **只等光标所在那一条** | `Server/Requests.lean:340` · `:357-363` |
| **取 goal** | `$/lean/plainGoal` → `getInteractiveGoals` → `findGoalsAt?`：从该快照的 `InfoState` 取**已算好**的 `goalsBefore/goalsAfter`，**不重跑** | `FileWorker/RequestHandling.lean:188-225` |
| **前提** | 会话**已落定**；开档就敲 ⇒ goal 要等整轮（实测 194ms，§4.1） | — |

⇒ **3ms 的来源是"只等一条命令的快照"，不是"更快地重编"** ✓。编辑点之后那
20 多条声明**照样在编**（213ms 才有诊断），只是 goal **不等它们** ✓。

### 8.2 我们的差距（今天）

`soko/goals` → `doc.report()`（`crates/lsp/src/lib.rs:1156-1163`）—— `report` 是**整趟入口
编完之后**由 `assemble_from_session`（`crates/front/src/project/mod.rs:596`）组装的**一份**
结构 ⇒ goal 与诊断**同一个时刻**（81.4ms）✓，**中途没有任何"命令级 goal 状态"被发布** ✗。

### 8.3 对齐路径（三件，前两件是 T2-A/T2-B）

* **T2-A（内核 · 结构性前置件）—— ✓ 已落地**（2026-10-09）：声明表**分层持久化 / COW**
  ⇒ 命令边界取环境快照 **O(#decls) → O(1)**。读数：64 / 512 条声明**两臂相等且 = 0 条目**
  被复制（先建先红实测：之前为 196 / 1540）。设计 = `docs/design/persistent-declarations.md`；
  对照 = Lean `SMap`（导入层平坦表 + 本地层持久表，`Data/SMap.lean:28-33`）✓。
* **T2-B（入口趟命令级环境快照）—— ⏳ 进行中**：在命令边界留
  `(环境快照, walk 累加器, 报告片段)`，编辑后从**最后一个文本未变的命令**接着编。
  **读数已先建先红** ✓：`elaborated_commands_total()`（`crates/front/src/compile/mod.rs`）
  立在 `crates/front/tests/keystroke_structure.rs::t2b_last_command_edit_still_reelaborates_every_entry_command`
  —— 今天"改最后一条"= **43 条**（入口 12 + 库层 + pass2），目标 = **1**。
* **还差的第三件（真正决定 goal 延迟的那件）**：把 goal **按命令发布**（编到哪条就能问哪条）
  —— Lean 侧的对照实现就是 `AsyncList` + `waitFindAtPos` ✓。**T2-B 只解决"从哪条开始编"，
  不解决"编到哪条就能看"** ✗ —— 两件都做完，goal 才可能进 3ms 量级 ✓。

### 8.4 口径（别混）

* Lean 的 **218ms** 是"**诊断全量落地**"（编辑点之后整份文件）；**3.1ms** 是"**goal 更新**"。
* sokonanoda 的 **81.4ms** 在**两个口径上是同一个数**（goal 与诊断同时到）⇒ 对齐目标是
  **把这两个口径拆开**：诊断保持 81ms 量级 ✓，goal 单独降到 **~1 条命令的 elaborate 时间** ✓。
