# strict 重构交接（2026-10-31 建档 · **2026-11-01 更新到 P2-12 中断现场**）

接手只读本文 + `reports/2026-10-29-strict-replan.md`（计划与逐刀记录）。本文所有数字注明了出处；没注明的当未核实。
**本轮交接起因**：上一棒在 P2-12 的 S0b/S4a 两刀上被 API 502 打断（两个子代理同时死亡），§2 是完整的中断现场。

## 1. 现在在哪

- 分支 `strict/2026-10-29-kernel-app`（远端同名），从 main `f8d231d3` 分出，领先 main 81 个提交。**只有这一条工作分支**。
- 新路径开关：`SOKO_STRICT_KERNEL_APP=1`，只在 `--dialect strict` 生效。开关关 ⇒ 不执行新代码，`--json` 与基线逐字节相同（每刀合并前都对拍过）。
- 计划进度：P0 ✓ · P1 ✓ · P2-1…P2-9 ✓ · P2-C 普查 ✓ · P2-10 ✗（降级存档）· P2-10b ✓ · P2-11a-A/B ✓ · P2-L / P2-L-fix ✓ · P2-11b ✓ · 第一、二批验收 ✓ · P2-11c ✓ · P2-11e ✓ · 第三、四批验收 ✓ · **P2-12：S0 ✓ `88204816` · S1 ✓ `9e8d68ea` · S0b ✓ `fab033bd` · S4a ✓ `ef1f7299`· S2–S6 未开始** · P2-13 / P3 / P4 未开始。
  **2026-11-01 接手补记**：上一棒 502 时的 `/tmp` 现场与 `~/Documents/lean/sokonanoda-archive-2026-10-31/` 在本机**都不存在了**（探针 worktree、`probe3.diff`、probe4 二进制、16 个变体复现件、`branches.bundle` 全部丢失）⇒ S4a 的探针是**这一棒重做**的（结论见 §2.2 末），S0b/S4a 的读数一律以本棒二进制为准。
  **进度权威 = replan §4「P2 接管」拆刀表**（本节只是摘要）。
- 推送纪律：2026-11-01 交接前一次性推送（8 笔 S0/S1/第四批/设计 + 本交接书）；CI 只跑 main 和 PR，推此分支不触发 CI。

| 刀 | 提交 | 内容 |
|---|---|---|
| P2-1…P2-9 | `9118256b`…`f8b095cf` | 推迟实参、多轮、会话内重检、记法走内核、探测模式、缓存键带开关档；细节见 replan §4 |
| P2-10b + P2-L-fix | 第二批 | 带注解 λ 体对着 codomain elaborate；记法层级逐个操作数试（m5，两档） |
| P2-11a/11b/11c/11e | 第一、三、四批 | 陈旧 Value 重试两处、surplus、宇宙多态头、`Eq.{u}` 进内核 |
| P2-12 S1 | `9e8d68ea` | 内核 `AppOutcome` 结构化载荷，`format!("?{id}")` 已删，反向验证判红过 |
| P2-12 S0 | `88204816` | 内核回落后老路结局计数 `fallback_old_<ok\|none\|err>_<why>`（`kernel_app_census.rs`） |
| P2-12 S0b | `fab033bd` | 补齐记法路由 / 零元 `try_hinted_const` / `fill_late` 三处计数 ⇒ 对账闭合；**记法路由一项就把 old_ok 从 59 抬到 88+** |
| P2-12 S2 | 本棒 | `kernel_app_report.rs` 纯函数（结局 ⇒ `CompileError`），先不接线；t1 逐字节同老路、内核名一律兜底 |
| P2-12 S4a | `ef1f7299` | `congrArg.{1}` 两族：`app_expected_def_eq`（陈旧 `fty`）+ conv Pi 臂各喂各自域（陈旧 `dx` 喂右闭包 ⇒ panic 被吞成 `ArgMismatch`）；四文件 old_ok 10→0 |

### 1.1 接手点（2026-11-01 11:57，**从这里继续就不会丢线**）

本棒已并入的提交（都在 `strict/2026-10-29-kernel-app`，按顺序）：

| 提交 | 是什么 | 关键读数 |
|---|---|---|
| `fab033bd` | S0b：三处回落计数 | 对账闭合（前差 68 / 31）；开关关语料 714 行 0 差异；**记法路由一项 88+ 次 old_ok** |
| `ef1f7299` | S4a：`congrArg.{1}` 两族 | 四文件 old_ok 10→0；kernel lib 123；两对语料 714×2 全 0 差异 |
| `e36d2a4a` | docs：S0b/S4a 回写 | docs-lint 绿 |
| `a7c7d6df` | S2：`kernel_app_report.rs` 纯函数（未接线） | 7 条单测绿；t1 逐字节同老路 |

**层级约定（别把 S 当阶段）**：计划只有一条线 —— `replan §4` 的 **P0 → P1 → P2 → P3 → P4**。
`P2`（接管）内部有 **13 刀**（`P2-1`…`P2-13`）；其中 **`P2-12` 又拆了 S0…S6**（`reports/2026-10-31-p2-12-design.md` §Q6，
交接书 §3 的 1–7 就是它）。所以：

| 层 | 名字 | 现在 |
|---|---|---|
| 阶段 | P0 基线 / P1 内核元变量+elab_app / **P2 接管** / P3 清理 / P4 发版 | P0 ✓ · P1 ✓ · **P2 收尾中** · P3 未开始 · P4 未开始 |
| P2 的刀 | P2-1…P2-11 ✓ · **P2-12（S0 ✓ S1 ✓ S0b ✓ S2 ✓ S4a ✓；余 S3 · S4b…n · S5 · S6）** · P2-13 未开始 | 在 P2-12 里 |
| P2 的总判据 | lib 17/17 · 整门课 strict 全部目标通过 · 计数 ≤ loose · loose 档 golden 与 `--json` 逐字节不变 | 前三项已达（S6 翻默认后才算全） |

⇒ 说「S3」时指的是 **P2-12 的第 3 步**，不是阶段；P3 清理要等 P2-12 的 S6 翻完默认才开始。

**S4b 的输入已就绪**：`reports/2026-11-01-p212-s4b-census.md`（357 文件全跑，`fallback_old_ok` **185 次 / 11 文件**，
非例外 130；**`Set.Le` 记法一族 121 次**，只修它就从 185 掉到 64；对账 357/357 成立）。
⚠ 该报告的**位置坐标有两类伪坐标**（前缀缓冲区 +1530、line 1 合成 span），定位源码前先换算。

**下一刀 = S3**（`SOKO_STRICT_NO_FALLBACK`，验收见 replan §4 该行）。S3 的前提已在 S2 备好：
`Report` 三态（`Error` / `Fallback(why)` / `Defer`）就是"哪些还能回落"的那张表，`Fallback` 的四种原因
（`unsupported` / `need_args` / `expected_retry` / `argmismatch_text`）逐条都是**写明的例外**。

**交接纪律的来源**：`reports/2026-11-01-craft-notes.md`（上一棒两天 64 笔的做法 + 我这一棒的对照检查 + 下一刀的检查表）——
它写的是**怎么干**，不是干了什么；接手先读它再读 replan。

**分工（2026-11-01 用户拍板）**：主会话写计划/实施单 + 审 + 并入 + 跑重判据；**开发刀交给子代理**
（每刀的"怎么做"只写在 `reports/2026-11-01-p212-s3-spec.md` 这类实施单里，子代理不带上下文也够开工）。
子代理的 prompt 必须自带三样：本仓硬纪律摘要（长命令落盘 / 连红 3 次换招 / 探针带构建身份）、**可执行的判据**、
**明确的"不许动"边界**（含"不许 commit/push / 不许跑全量 / 不许碰 `crates/kernel/`"）。

⚠ 两件必须知道的事：
1. **S0 的旧清单作废**：S3 要按 S0b 的普查重排优先级，最大一块是 `fallback_old_ok_notation_unsolved`
   （`Set.Le` 的 `A ≼ B`），不是 `congrArg`（后者已由 S4a 清零）。
2. **`is_def_eq_at` 把内核 panic 当成"不 defeq"**（S4a 的真凶）——`catch_unwind` 那一条是
   `docs/architecture.md` G-91「判不了 ⇒ 当成否」的反例，记作 P2-13 候选，别在 S3/S4 里再踩。

## 2. 中断现场（2026-11-01 03:00 左右，API 502 双杀）

### 2.1 全课程普查给出的靶子（replan §4，开关开，251 文件）⚠ **本节数字是下界，S0b 后作废**：只结算了应用路径

老路接住 **59 次 / 10 个文件** = 去掉回落会翻转的上界。写明的例外 41（`input contains a meta` 30 · `shape` 9 · `ho_motive` 2）⇒ **真正要先修 18**：`argmismatch` 10 · `expected` 4 · `unsolved` 4。
按位置（S0 位置日志）：**`congrArg.{1}` 占 11/18**（`argmismatch` 8：unit10-cantor:89 / unit10-sol；`expected` 3：unit06-relations:195 / unit06-sol:308）；其余 7 = `Set.inter`/`Set.image` `unsolved` 4（unit12-sol:177/184）· `Set.ext` 1（unit12-sol:94）· `Set.mem_preimage_elim` `expected` 1（unit22-sol:55）· `Set.mem` `argmismatch` 1（unit22）。
**S0 未覆盖**（⇒ S0b 要补）：记法路由、零元 `try_hinted_const`、`fill_late` —— `fallback_unsolved` 约 119 次对不上账，推断落这三处。**S0b 已证实**：差额全在**记法路由**（`Set.Le` 的 `A ≼ B`）——unit19-solution 一个文件就有 88 次 `old_ok`、cardinal-le 31 次 ⇒ 「真正要先修 18」只覆盖应用路径那一半，S3 之前必须用 S0b 的普查重排优先级（`fallback_old_ok_notation_unsolved` 是最大的一块）。零元 `try_hinted_const`（按 `hinted_const_fallback_<原因>` 分桶）与 `fill_late` 是**写明的例外**（不是应用级回落），只作可见性计数。

### 2.2 S4a（`congrArg.{1}` 根因）——**2026-11-01 已收口**（下面的探针现场当时丢了，本棒重做；结论在节末）

复现件（**已存档**，见 §6；`arg.sokonanoda` 报 `argmismatch @2:3`，`exp.sokonanoda` 报 `expected @5:3`）：

```sokonanoda
-- arg 类（unit10-cantor:89 形态）：λ binder 类型是 Π 时才挂
theorem t (α : Type) (f : α → α → Prop) (x : α) (g : α → Prop) (h : g = f x) : (g x) = (f x x) :=
  congrArg.{1} (fun (A : α → Prop) => A x) h
-- exp 类（unit06 形态）
theorem t (a b : Nat) (h : Nat.succ a = Nat.succ b) : a = b :=
  congrArg.{1} Nat.pred h   -- 需要文件里先 def Nat.pred
```

已排除 / 已证实（进度日志 02:35–02:55，逐条有二进制 sha）：

1. **层数假设排除**：课程 prelude 的 `congrArg` 只有**一个**宇宙参数 `{u}`（`α β : Sort u`；Lean 是 `{u v}` 两个），`.{1}` 个数正确。
2. **exp 类根因已证实**（probe2 二进制 `1ca8b3ad`）：`elab_app.rs:313` 最终期望类型检查前，quote→`instantiate_mvars`→eval 重新求值后 `defeq=true` ⇒ **赋值前存下的陈旧 Value**，与 P2-11a-B / P2-11b 同族。**修法已知**：照前两刀在这里补"本会话确实赋过值就重新求值再比一次"，改动小。
3. **arg 类差 probe4 读数**（probe3 二进制 `13493866`）：两个 Π 的定义域能对上（`defeq=true`），但 `?β` **一次都没进 Miller 赋值**；probe3 在 `apply_closure(actual infer closure, Some(dd))` panic ⇒ 假设 **`conv.rs:213-214` 把旧值 `dx` 喂给了 infer closure**（Lean 的 whnf 会经 `instantiateMVars` 跟随赋值，我们的 `force_all` 不回放已赋值的 `Value::Meta`——子代理读 Lean `ExprDefEq.lean:512-526` 后的对照）。**probe4 已编译完成（02:56），跑之前 agent 就被 502 打断**。
4. 变体矩阵：`w6`（全显式 `@congrArg`）通过；`w3`/`w4`（binder 类型是 Π）必挂 ⇒ 两类失败都不是写法/层级问题，是内核 defeq 少走一步。

**2026-11-01 重做探针的结论（取代上面的「差最后一步」）**：

- **exp 类**：根因同 2（陈旧 `Value`）。修 `elab_app::app_expected_def_eq`：最终「期望 ≟ `fty`」失败且本会话自 `gen0` 起有过赋值 ⇒ `instantiate_mvars(quote(fty))` 重算再比（与 `app_arg_def_eq` / `app_reeval_pi` 同族）。
- **arg 类**：**不是** Miller/一阶赋值的问题。读数 `[s4a] gen=1 gen0=0 dom.inst == dom`（`?α`/`?β` **未赋值**，`load` 也不是 Miller 失配）；真因是**被吞掉的 panic**：`is_def_eq_at` 用 `catch_unwind` 把内核 panic 当成「不 defeq」⇒ 报 `ArgMismatch`。panic 现场 = `infer_app_v`: `expected a pi type, got: Pi ( : $5), Sort(0)` —— `$5` 在 depth 5 上是**越界 bvar**，来自 conv Pi 臂把**左边**的域 `dx` 喂给**右闭包**（`conv.rs:218`），而 `dx` 可能是赋值前的陈旧值 ⇒ infer 闭包在错误作用域推断体。修：右闭包拿它自己的 `dy`。
- 教训（值得写进台账）：**`catch_unwind` 把「判不了」变成「判成否」**——正是 `docs/architecture.md` G-91 那条「判不了 ⇒ 走慢路 ✓ / 当成否 ✗」的反例；这次它把 8 次 `congrArg` 伪装成类型不匹配。以后在 `is_def_eq_at` 里 panic 一律当 `Unsupported` 而不是 false 值得单开一刀（P2-13 候选）。

**现场与工具**：
- 探针 worktree `/tmp/p212s4a-wt`（3 文件改动：`kernel_app.rs` 打印载荷 + `elab_app.rs` 分域探针 + `mvar_miller.rs` 赋值点探针，全部受 `P212S4A=1` 门控）；probe4 二进制还在 `target/release/sokonanoda`。
- **全部已存档** `~/Documents/lean/sokonanoda-archive-2026-10-31/p212-s4a-probe/`：`probe3.diff`（含 probe4 改动）· 16 个变体复现件（v1–v8 / w1–w8 / arg / exp）· 两份进度日志。
- ⚠ 这些是**只读探针**，一条都不许直接进提交；修法要按 AGENTS.md「卡住先读 Lean 4」走，修内核 = 开关开/关两组全语料对拍逐字节 + 反向验证。

### 2.3 S0b（补三处回落计数，前端）——**2026-11-01 已并入**（方案照下面执行，一处偏差：`elab.rs` 只剩 2 行余量，改用 `Settle` RAII 守卫，净 +2 行）

子代理 02:42 交的方案（进度日志在档）：**记法路由**在 census 里加原因前缀 + 用包装函数接住 `elab_notation` 结果按 ok/err 结算；**零元 `try_hinted_const`** 加标记区分回落来源；**`fill_late`** 每个推迟的 λ 位单独计一次。计数逻辑全部进 `kernel_app_census.rs`（现 126 行），**不动控制流**。
- ⚠ `crates/front/src/compile/elab.rs` 现 **8839 行 / 基线 8841，只剩 2 行余量**；`kernel_app.rs` 499/500。多出来的行一律进新文件。
- 落地结果（2026-11-01）：计数逻辑全在 `kernel_app_census.rs`；`elab.rs` 只加 2 行（`census::arm(kg.is_some())` + `settle.ok()`），**正好 8841 = 基线**。worktree `.claude/worktrees/agent-a03808b8050a3b64e` 在本机**不存在**（同上丢失），没用到。

## 3. 下一步（按顺序；每刀验收见 replan §4 对应行）

1. ~~**S0b**~~ ✓：三处计数已补，对账闭合（细节 replan §4 该行）。
2. ~~**S4a**~~ ✓：两处内核修（§2.2 末）；四文件 old_ok 10→0、rc/诊断不变、`MISS` 不升、探针全删。
3. **S2**：前端 `kernel_app_report.rs` 纯函数——不印内核名字（`Implicit.index` 经 `source_layer_names` 取源级名；两边仍可能含本会话元变量 id≥2^30，渲染兜底）。留 S1 的既知边界见 replan。
4. **S3**：过渡开关 `SOKO_STRICT_NO_FALLBACK`（**进所有缓存键**，照 P2-8 先例）；核对那 197 次「老路也报错」的文件去回落后仍被上层吸收。
5. **S4b…n**：按 §2.1 清单修剩余 `old_ok` 至 0（写明的例外类除外：`Unsupported`/`shape`/`ho_motive`/`untried`/具名实参）。
6. **S5**：CLI + LSP hover 测试（必备回归：`axiom g : {A : Type} → Nat → Prop` / `theorem t1 : g 0 := sorry` 必须给可读报错、位置落在 `g 0`，不再出现 `\0soko_m0`）。
7. **S6**：翻默认、删 `kernel_app::env_on`、重写 `kernel_app_keys.rs`。前提 = S3 全课程 `fallback_old_ok` = 0（例外类除外）。
8. 之后 **P2-13**（砍共享代码里的 strict 分支）→ **P3**（`audit-strict-paths` 归零）→ **P4** 发版（默认档由用户拍板；push/release 前先问用户）。

## 4. 读数（开关开，冷跑，二进制 `a5e9b3eb`，macOS/arm64；第四批复跑一致）

| 文件 | `judge_infer` 未命中 strict/loose | 耗时 strict/loose |
|---|---|---|
| unit01 | 5 / 5 | 1.05s / 1.41s |
| Equiv | 4 / 4 | 2.16s / 2.44s |
| unit12 | 4 / 4 | 4.35s / 4.16s |

lib strict 17/17。计分板 `work_le_loose` 三文件全绿。⇒ **性能目标（strict ≤ loose）已达成**；剩下的是消回落、变默认、删旧路径。
`scripts/soko gate --fast` 唯一红项是 `infoview-hierarchy`（`.section-title` 字号）——`editor/` 与 main 零差异（本交接时 `git diff main...HEAD --name-only -- editor/` = 0 行），属 main 上的既有问题。

## 5. 怎么验证（脚本已进仓库 `scripts/strict/`）

```bash
# 计分板：lib 两档 + 工作量门（strict 的未命中 ≤ loose）；--course 整门课
python3 scripts/strict-score.py --bin <cli> --env SOKO_STRICT_KERNEL_APP=1 [--course]
# 全语料逐字节对拍（tracked .sokonanoda × 两档）：先钉二进制，再 diff 两份输出
scripts/strict/corpus-hashes.sh <base-cli> /tmp/base.txt
scripts/strict/corpus-hashes.sh <new-cli>  /tmp/new.txt && diff /tmp/base.txt /tmp/new.txt
# 单文件回落计数（开关开）：一行 JSON（含 fallback_old_*）
scripts/strict/fallback-one.sh <cli> <file.sokonanoda>
# 逐位置回落日志（S0/S0b 的普查用）：SOKO_KERNEL_APP_LOG=<path> 追加 "<ok|none|err> <why> <head> <line>:<col>"
# 位置归因（18 次靶子怎么来的）：SOKO_JUDGE_STATS=1 下 grep KERNEL_APP 行的 fallback_old_* 与位置
scripts/soko gate --fast        # 环节收尾；内核改动另跑两组全语料对拍
```

坑（都踩过）：
- **/tmp 是易失的**：子代理的探针/复现件/进度日志，交接时必须先存档进 `~/Documents/lean/sokonanoda-archive-2026-10-31/`（2026-11-01 已照做），再谈别的。
- 长跑前**把二进制拷到 sha 命名的私有路径**——agent 会中途重编 target，读数混用两个二进制就作废。
- macOS 没有 `timeout`，用 `perl -e 'alarm shift; exec @ARGV' <秒> <cmd>`；zsh 不拆未加引号变量，git 传 ref 列表用 `--stdin`。
- **对拍只比同一检出目录跑出的结果**（G77 诊断带模块根绝对路径）；空哈希 `e3b0c44298fc1c14` 只许出现在 `lib/Logic`。
- 同一二进制并发跑两次对拍曾让 12 个进程卡死在 UE 状态（2026-10-31 19:28，**至今还在**：`/tmp/corpus-bin-55237c44*`，0 CPU、杀不掉、无害；`pgrep corpus-one` 会数到它们的父 shell —— 别拿它当"对拍还在跑"的证据）。脚本已改为复用同 sha 副本 + `mv`。
- 一条声明只报第一处错，判"既有问题"要拆到合取粒度复现；`lib/Logic` 两档输出本来就为空，对拍相同不说明什么。
- `elab.rs` 只剩 2 行余量（§2.3）——front 的行数基线由 docs-lint 判，超线 gate 红。

## 6. 备份（只在本机）

`~/Documents/lean/sokonanoda-archive-2026-10-31/`：
- **`p212-s4a-probe/`（本轮新增）**：`probe3.diff`（含 probe4 改动，基于 `b3665792`）· 16 个变体复现件 · `p212s4a-progress.log` / `p212s0b-progress.log`。恢复：在新 worktree `git apply probe3.diff` 后 `cargo build --release -p sokonanoda-cli`，或直接从 `/tmp/p212s4a-wt/target/release/sokonanoda` 拿 probe4 二进制（/tmp 若已清则重建）。
- `branches.bundle`：清理前全部 51 个分支（含旧方案 `strict/2026-10-29-pipeline`）；`refs.txt` 对照。恢复：`git fetch <bundle> 'refs/remotes/origin/strict/2026-10-29-pipeline:refs/heads/old-pipeline'`。replan §5 要从旧分支摘的 `strict/report.rs` 等**还没摘**，P3 要用时从这里取。
- `p0-spike.diff` + `p0_spike.rs`、`k1-agent-uncommitted.diff`、`p27-design.md`、`p2-10-blocked-hints.bundle` + `p210-front-retry.diff`、`p211e-design-b.diff`（P2-11e 方案 B，不提交的原因见 replan）。
