# 内核线交接单（换会话用）

> 覆盖式重写于 2026-10-04 19:5x ✓。**只写当前状态** ✗（历史看 `git log` ✓）。
> 分支 **main**（本地 ✓，**未推送** —— 推送由值守独占 ✓）。（`docs-expiry.json` 登记 2026-11-15 ✓）

## 0. ⚠ 工作流规则（**先读这条** ✓）—— 用户两次拍板 ✓

> **18:36**：「要修改工作流，**不要随便跑完整测试，除非到了发版大节点**」
> **19:52**：「**内核也可以贪心测试，三层回归太费时间和上下文，导致内核基本开发不动**」

* **日常每次改动（含内核改动 ✓）= 贪心三件** ✓（都便宜，秒级~十几秒 ✓）：
  **(a) 该处针对性复现件** ✓（`docs/gaps/repro/G*-*.sh`，一条命令看退出码 ✓）·
  **(b) 反向验证** ✓（撤掉修复必须重新判红；撤了仍绿 ⇒ 守卫是假的 ✗）·
  **(c) 受影响文件的逐字节比对** ✓（**只比本次改动直接碰到的文件/夹具** ✓，**不是**全语料 ✗）。
* **只在发版大节点跑一次** ✓（不再每轮 ✗）：三层回归（内核单测 + 前端单测 + CLI e2e）·
  全语料对拍 · 全量 `--json` 逐字节不变 · 整包 `cargo test --workspace` · 整本课程 `check.py`。
* ⚠ **这不是"不验证"** ✗，两条底线不许碰：① **(a)(b)(c) 一条都不许省** ✓；
  ② **唯一硬底线**：(c) 里若发现**任何输出变了** ⇒ **必须停下来定性**（修好了？改坏了？）✓
  **不许当没看见放过** ✗ —— 这条正是挡"悄悄改坏判定"的 ✓。
* ⚠⚠ **省上下文**（用户点名"费上下文" ✓）：重测试**必须重定向到文件**（`> /tmp/x.log 2>&1` ✓）·
  **只回读 `tail -20` + 结果计数行** ✓ · **禁止原始输出灌进会话** ✗。
* 声明已同步 ✓：`AGENTS.md`（硬规则 1 + CI 节奏 1 + 长命令节 ✓）· `skills/sokonanoda-dev/SKILL.md` ✓ ·
  `scripts/dev-verify.sh` 文件头 ✓ · `docs/architecture.md` §6 政策 ✓ · `docs/LESSONS.md` ✓ ·
  `fuzz/README.md` ✓ · 本文件 ✓。⚠ `AGENTS.md`/`architecture.md`/`LESSONS.md` **都卡在上限** ⇒ 加内容要**折进既有行** ✓。

## 1. HEAD 与判据现状

* **HEAD** = `bf8c4836` ✓；本会话（第 14–20 棒 ✓）十三笔，都在树上 ✓。
* **未推** ✓（本会话未 push / 未打 tag ✓）；**工作树干净** ✓。
* 判据 ✓：`lsp_keystroke_structure` **3/3** ✓ · `perf_course` **5/5** ✓（unit12 **5382ms** ✓ ≤8.5s）·
  `identity_probe` **2/2** ✓ · `gate_census` **1/1** ✓ · `gap.py check` **94/0** ✓ · `docs-lint` ✓ ·
  六条复现件**全 exit 0** ✓（G-88/89/90/91/92/93 ✓）。

## 2. 手上的 WIP

**没有** ✓ —— 全部提交 ✓，无「在树里、默认关」的东西 ✓（`SOKO_NO_SEED` 是**逃生门** ✓，默认**开** ✓）。
⚠ 第 20 棒试的 G-93「位置式」修法**已整段还原** ✓（`elab.rs` 干净 ✓，还原后语料**逐字节相同** ✓）。

## 3. 下一棒做什么（按序 ✓）

0. ✗✗ **`G-93`（blocker · 正确性 · 根因已定位 ✓）**：**显式宇宙多态的常量，宇宙层不参与推断 ⇒ 一律默认 `0`** ✗。
   * **病灶 = 一行** ✓：`crates/front/src/compile/elab.rs:4525` `params.iter().map(|_| builder.zero())` ✗
     （`grep -n 'map(|_| builder.zero())'` 在该文件**唯一**命中 ✓）。
   * **最小复现（`Eq` 完全不出场 ✓）**：`axiom myax {u} : {α : Sort u} -> (a : α) -> α` +
     `def d (α : Type) (a : α) : α := myax α a` ⇒ 红 ✗「期望 `Sort(0)`，实际是 `Sort(1)`」；
     写 `myax.{1} α a` ⇒ **绿** ✓（⇒ 就是层没推断 ✓）。`{α : Prop}` 绿 ✓（默认值碰巧对 ✓）·
     `{α : Type 1}` 红 ✗「实际是 `Sort(2)`」· 两个宇宙位红 ✗。
   * **症状面**：`theorem t (α : Type) (a : α) : a = a := Eq.refl α a` **判红** ✗ 而 `:= rfl`/`:= by rfl`
     **判绿** ✓（**同命题换写法两态** ✗✗）；**触发面两个且独立** ✓：**类型侧**（指向式 `: Eq α a a` ✗，
     记法 `a = a` ✓ 不触发）+ **值侧**（`Eq.refl α a` 当项 ✗）。
   * ⚠ **另有路径分叉**（一并修 ✓）：`build` 绿 ✓ / `grade` 红 ✗；`build` **带 `import`** 又红 ✗
     ⇒ `build`/`grade` × 单文件/闭包 **四格对不齐** ✗。
   * ✗✗ **第 20 棒试的「位置式」修法已被判据否掉** ✗ —— **别再重试** ✗：抄递归子那条、在 App 臂
     「取**首个书写实参**的类型 ⇒ 当那个宇宙位」✗ ⇒ 症状面确实转绿 ✓（`Eq.refl` 当项 ✓ · 指向式 ✓ ·
     带 `import` ✓）**但 `lib/Order` 第 474 行 `Acc` 当场 `compiled → failed`** ✗（首个实参未必对应
     宇宙位所在形参 ✗）。
   * ⇒ **唯一正确形状 = 把实参类型与形参类型合一、解出层** ✓（= **U1 的 level 元变量 + 约束存储** ✗，
     `docs/design/metavar-engine.md` §2.9 ✓）。**先例可抄** ✓：`universe_level_text_of_operands`（记法那条路
     已经在推断 ✓ —— 相位 E 绿 ✓）· `infer_recursor_universes`（补的是**结果**层 ✗，形状可抄 ✓）。
   * 复现件 ✓ `docs/gaps/repro/G93-universe-level-not-inferred.sh`（**三相位** ✓：A 缺口 · **B 对照**
     （写 `.{1}` 必须绿 ✓，它红了 ⇒ exit 2 ⇒ **夹具坏了要报** ✓）· C 症状面 ✓）。
   * ⚠ 修的时候**必须**把「单文件 vs 带 `import`」与「`build` vs `grade`」做成**成对探针**跑一遍语料 ✓
     （G-72 的 `why_open` 原话 ✓），不许再修单点 ✗。
1. **G-92：`by`/`#check` 判定的「前缀重跑」仍 O(n²)** ✗（判据在册 ✓、秒级 ✓）。
   第 16 棒已走一步 ✓（`#check` 两条路改走受信任前缀 ⇒ `unit12-synthesis` 墙钟 **10.97 → 5.75s（1.91×）** ✓），
   但换夹具后比值仍 **3.98** ✗（⚠ 第 15 棒夹具**本身编不过** ✗ ⇒ 那时量的是**失败路径** ✗，已更正 ✓；
   换夹具后 `pass_total_ms` 每个翻倍 **2.6×** ✗、墙钟 80→160 = **2.44×** ✗ ⇒ 结论更硬 ✓）。
   **根因已收窄** ✓：信任档**只跳内核检查、不跳 elaborate** ✗（`walk.rs:813` 在建完环境**之后**才早退 ✓）。
   **出路二选一** ✓：① 给就地路加「**文本 ⇒ AST**」入口 ✓（边界见 `by.rs:88-93` ✓）；
   ② 让合成编译**复用调用方的环境** ✓（= G-68 架构件 ✗，`docs/design/module-artifacts.md` §2 ✓）。
2. **③ 限制可配置化 + 按 Lean 4 对齐数值**：① 可配置化（**默认值一个不动** ✓、零行为变化 ✓）；
   ② 放宽默认值**单独一笔** ✓。**终点取 Lean 数值** ✓：`maxRecDepth 64→3200` · `maxHeartbeats 4096→20000` ·
   `maxSize 64→128` · `maxSynthDepth 8→32` ✓；**Lean 没有的**（`PROBE_CAP` ⇒ 弃权）**去掉，不许换数字留着** ✗。
   ⚠ **取证口已经在了** ✓（`SOKO_LIMIT_*` 四个 ✓，默认值**一个不动** ✓）⇒ ② 步就是换**默认值** ✓，
   判据现成：`scripts/limits-only-slow.sh --full` ✓。
3. **G-91 的乙类 4 处计数出口**（内核侧七个**已铺** ✓）：`judge.rs` 两张表容量 · 目标分解失败（`goals.rs`）·
   `SKELETON_MAX_LAYERS` ✓。复现件铺齐后**判 1** ⇒ 回来关账 ✓。
4. 排队（做不完继续往下传 ✓）：**IA-4 元参数引擎余片**（U1/U2 宇宙层**正是 G-93 的解** ✓）·
   **集合论教材线 S-A/S-B/S-C**（`docs/ONBOARDING.md` §0.2 是**唯一队列** ✓）。

## 4. 每条要带的判据（判红 / 判绿 ✓）

| 项 | 判据 | 现状 |
|---|---|---|
| 12 单元特性 | `cargo test -p sokonanoda-lsp --test lsp_keystroke_structure` ⇒ 改陈述 `prefix>0` / 改证明体 `prefix==0` | **有 ✓ 绿** |
| 冷开不退化 | `cargo test -p sokonanoda-lsp --lib perf_course` ⇒ unit12 **≤8.5s** | **有 ✓ 绿**（5382ms ✓） |
| 增量身份等价 | `cargo test -p sokonanoda-front --test identity_probe` ⇒ `probed>0` · `uncomparable==0` · `mismatches==0` · `fallbacks==0` · `evictions==0` | **有 ✓ 绿**（2612/0/0/0/0 ✓） |
| O(n²) 不许回来 | 同上第二个用例 ⇒ `reparse < 20`（实测 **0** ✓；`SOKO_NO_SEED=1` ⇒ **54** ⇒ 判红 ✓） | **有 ✓ 绿** |
| 闸类普查 | `cargo test -p sokonanoda-front --test gate_census` ⇒ 甲类闸 == 0 | **有 ✓ 绿**（反向：`MAX_DEPTH=1` ⇒ **59** ⇒ 判红 ✓） |
| **闸只变慢不变错** | `scripts/limits-only-slow.sh --full` ⇒ 三道闸拧到 1，输出**逐字节相同** ✓ + 闸**确实触发** ✓ | **有 ✓ 绿**（10589/51129/139358/1334 ✓） |
| G-88 预算不当成否 | `cargo test -p sokonanoda-front --lib meta::tests::exhausted_budget_…` | **有 ✓ 绿**（反向：换回 `Tri::No` ⇒ 判红 ✓） |
| 规模翻倍 ⇒ 耗时翻倍 | **真实课程** ✓：`SOKO_DECL_PROFILE=1` 逐声明耗时**平线** ✓ · **最坏形状** ✗：`by_calls` 比值 **3.98** ⇒ **G-92** ✓ | **有 ✓（两条分开记 ✓）** |
| **判定前缀不再重查内核** | `SOKO_JUDGE_ENV_VOUCH=shadow SOKO_JUDGE_ENV_PROBE=1` ⇒ `shadow_diff == 0` 且 `shadow_same > 0` | **有 ✓ 绿**（43/59 · 0 ✓；反向：恒不等 ⇒ **17** ⇒ 判红 ✓） |
| 真实单元的收益 | `unit12-synthesis` 冷缓存墙钟（release ✓） | **有 ✓**（10.97 → 5.75s ✓） |
| 闸类复现件 | `bash docs/gaps/repro/G9*.sh` ⇒ exit 0（缺口仍在 ✓） | **有 ✓ 六条全 0** |
| 可配置化零行为 | 整本课程 `build --json` 剔心跳逐字节相同（**改前 vs 改后都要跑** ✓） | **无 ⇒ 先建**（第 ③ 步用 ✓） |

## 5. 已知的坑（都带实测代价 ✓）

* ⚠ **判据会「全被跳过」** ✗：上一棒只读「有没有 `MISMATCH`」⇒ 读到 **0** ⇒ 判成「已证等价」✗，
  而当时**所有**条都落在「前缀解析不过 ⇒ 退回原文 ⇒ 这次不比」里 ✗（`unit08` **1505/1540** ✗）。
  ⇒ **凡"逐条比对"的判据，必须同时报「比过多少条」** ✓（`probed > 0` 是防空转的硬要求 ✓）。
* ⚠ **`canonical_prefix_id` 必须用 `parse_fragment`** ✗（前缀是**文件片段** ✓，声明落在 `namespace` 里时
  **必然**停在未闭合处 ✓ —— G-05 §4.1）。换回严格 `parse` ⇒ `uncomparable` **0→1906** ⇒ 判红 ✓。
* ⚠ **身份必须从 AST 直取** ✗：片段用了**依赖声明的记法**时，片段与整体**都解析不过** ✓
  ⇒ 「切片段再 parse」必退原文（实测 27 处 ✗）。
* ⚠ **判官合成的声明 span 是零长** ✗：它在 `file.commands` 里、**不在 `file.src` 里** ⇒ 文本路看不见、
  AST 累加看得见 ⇒ 分叉（18 处 ✗）。`command_env_id` 已跳过零长 span ✓。
* ⚠ **`canonical_prefix_table` 的 `CAP` 必须 ≥ 工作集** ✗：`4096` 比整本课程还小 ⇒ 满则挤掉**活条目**
  ⇒ **抖动**（`identity_parses=3062` ✗）。现 **65536** ✓ + 淘汰计数 ✓。**两处 CAP 必须同数** ✗。
* ⚠ **探查（`PROBE_CAP`）**：耗尽**只许**表示「这次探查不可信 ⇒ **弃权走全量**」✓，
  **绝不许**表示「不相等」✗ —— 动它之前先把这条钉住 ✓。
* ⚠ **夹具必须先证明自己编得过** ✗（第 17 棒教训 ✓：G-92 的旧夹具 20 条**全红** ✗ ⇒ 量到的是**失败路径** ✗）。
* **子进程 stderr** ✗：服务端探针看不见 ⇒ 必须 `SOKO_LSP_TEST_STDERR=1` ✓（`start_traced` 的读线程
  **只留 `LSP_TRACE` 行** ✗ ⇒ 别的探针行会被**丢掉** ✓ —— 要在 CLI 上跑才看得见 ✓）。
* **进程级计数器** ✗：判据要取**差量** ✓；课程级断言放**独立进程**（集成测试 ✓）。
* **`target/debug` 跑不动整本课程** ✗：`build courses/set-theory` **stack overflow（exit 134）** ✓
  —— **基线二进制同形** ✓（不是新引入的 ✓），release 正常 ✓ ⇒ 量课程**一律 release** ✓。
* **release 产物会过期** ✗：改源码后只 `cargo test` 只编 debug ✓ ⇒ 量 release 前必须
  `cargo build --release -p sokonanoda-cli -p sokonanoda-lsp` ✓。
* **`AGENTS.md` 行数上限 435** ✓（`docs-lint` ⑦ 判红 ✓，而 `pre-commit` **不跑** docs-lint ✗）；
  **上限只许收紧** ✗ ⇒ 加内容要**折进既有行** ✓（`architecture.md` 750 / `LESSONS.md` 556 同理 ✓）。
* **bash 3.2**：变量后紧跟**多字节字符**必须写 `${var}` ✗；`sed 's/.*passes=/'` **贪婪** ✗。
* **python heredoc 里别用 ASCII 引号** ✗（中文串里一律 `「」` ✓）；**嵌套 heredoc 会截断外层** ✗
  （写补丁脚本落盘再跑 ✓）。
* **残留进程** ✗：长跑脚本要自己 `pgrep -f <名字>` 收 ✓。
