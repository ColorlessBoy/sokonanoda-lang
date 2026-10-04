# 内核线交接单（换会话用）

> 覆盖式重写于 2026-10-04 18:0x ✓（值守换会话单 ✓）。**只写当前状态** ✗（历史看 `git log` ✓）。
> 分支 **main**（本地 ✓，**未推送** —— 推送由值守独占 ✓）。（`docs-expiry.json` 登记 2026-11-15 ✓）

## 0. ⚠ 工作流规则（**用户 2026-10-04 18:36 拍板** ✓，先读这条 ✓）

> 「要修改工作流，**不要随便跑完整测试，除非到了发版大节点**」

* **全量测试**（`cargo test --workspace` / `cargo test -p front -p kernel` 这类**整包** ✓ ·
  整本课程 `check.py` · 语料对拍）**只在发版大节点跑一次** ✓ —— **不许当日常动作** ✗，
  **也不进每轮收尾** ✗（本会话就是照**旧声明**在每轮收尾跑整包 ✗）。
* **日常 + 每轮收尾 = `scripts/dev-verify.sh`（快路 ✓）+ 该处的针对性复现件** ✓；
  **全量覆盖交给远端 CI** ✓。
* ⚠ **边界（不许趁机降级）** ✗：**针对性复现件 + 前后翻转 + 反向验证（撤掉修复必须重新
  判红）一条都不许省** ✓ —— 挪走的**只有"全量套件"这一项** ✓。
* 声明已同步四处 ✓：`AGENTS.md`（CI 节奏 1 + 硬规则 1 ✓）· `scripts/dev-verify.sh` 文件头 ✓ ·
  `docs/LESSONS.md` ✓ · 本文件 ✓。

## 1. HEAD 与未推提交

* **结束 HEAD** = **`9b6d226d`** ✓；本棒（第 14/15 棒同一会话 ✓）七笔：
  `d3754d1d`（身份那条 O(n²) 真除）· `ae8c944d`（闸类计数出口）· `afb6dfbc`（台账/复现件）·
  `763fe103`（STATUS/交接单）· **`aba66ae9`（G-88 真修）** · **`a9df4194`（闸取值可注入 + 实验判据）** ·
  **`9b6d226d`（G-92 新账）**。
* **未推 153 笔** ✓（本会话未 push / 未打 tag ✓）；**工作树干净** ✓。
* 判据现状 ✓：`lsp_keystroke_structure` **3/3** ✓ · `perf_course` **5/5** ✓ ·
  `identity_probe` **2/2** ✓ · `gate_census` **1/1** ✓ · `meta::tests::exhausted_budget_…` ✓ ·
  **`scripts/limits-only-slow.sh --full` ✓** · **front 全量 872/0** ✓ ·
  `gap.py check` **93 通过 / 不一致 0** ✓ · `status-lint` ✓ · 课程 **243 compiled / 6 failed** ✓。

## 2. 手上的 WIP

**没有** ✓ —— 三笔全部提交 ✓，无「在树里、默认关」的东西 ✓（`SOKO_NO_SEED` 是**逃生门** ✓，
默认**开** ✓；`SOKO_PREFIX_SEED` 那个旧开关**已删** ✓）。

## 3. 下一棒做什么（按序 ✓）

0. ✗✗ **`G-93`（blocker · 正确性）**：**`Eq.refl` 形状一带 `import` 就被误拒** ✗
   —— **与 G-72 同族**（「单文件判绿、被 `import` 时判红」），是那次修复**没覆盖到的另一个形状** ✗。
   触发条件 = 声明类型里出现 **`Eq`** ✓（`P → P` / `P` / `P ∧ P` 两态都绿 ✓）；
   诊断 `kernel-rejected`「**期望 `Sort(0)`，实际是 `Sort(1)`**」✗；**两个二进制都复现** ⇒ **既有** ✓。
   复现件**两态各一条断言** ✓（`docs/gaps/repro/G93-eq-refl-rejected-when-imported.sh` ✓）。
   ⚠ `Eq.refl` 是**课程里最常见的证明** ✗ ⇒ 影响面大 ✓。**已排除**：隐式插入 ✗ ·
   `theorem` vs `def` ✗ · 依赖内容 ✗ · 显式宇宙 ✗（逐条实测 ✓）⇒ 从「闭包路上 `Eq` 的
   宇宙/解析」入手 ✓。⚠ **同类横向排查** ✗：修的时候**必须**把「单文件 vs 带 `import`」
   做成**成对探针**跑一遍语料 ✓（G-72 的 `why_open` 原话 ✓），不许再修单点 ✗。
1. **G-92：`by`/`#check` 判定的「前缀重跑」仍 O(n²)** ✗（判据在册 ✓、秒级 ✓）。
   **第 16 棒已走一步（部分修 ✓）**：`#check` 两条路改走**受信任前缀** ✓ ⇒
   `unit12-synthesis` 墙钟 **10.97 → 5.75s（1.91×）** ✓；但换过夹具后比值仍 **3.98** ✗
   （⚠ 第 15 棒的夹具**本身编不过** ✗ ⇒ 那时量的是**失败路径** ✗，已在台账更正 ✓；
   换夹具后 `pass_total_ms` 每个翻倍 **2.6×** ✗、墙钟 80→160 = **2.44×** ✗ ⇒ **结论更硬** ✓）。
   **根因已收窄** ✓：信任档**只跳内核检查、不跳 elaborate** ✗（`walk.rs:813` 的
   `if trusted { … return; }` 在建完环境**之后**才早退 ✓）⇒ 前缀里的 `by` 照样被
   elaborate 一遍 ✗。**出路二选一** ✓：① 给就地路加「**文本 ⇒ AST**」入口 ✓
   （`by.rs` 的 `infer(text)` 收文本 ✗ 而就地路只收源 AST ✗ —— 边界写在 `by.rs:88-93` ✓）；
   ② 让合成编译**复用调用方的环境** ✓（= G-68 那条**架构件** ✗，见
   `docs/design/module-artifacts.md` §2 ✓）。
2. **③ 限制可配置化 + 按 Lean 4 对齐数值**（值守 13:21/13:24 ✓）：① 可配置化
   （**默认值一个不动** ✓、零行为变化 ✓）；② 放宽默认值**单独一笔** ✓。
   **终点取 Lean 数值** ✓：`maxRecDepth 64→3200` · `maxHeartbeats 4096→20000` ·
   `maxSize 64→128` · `maxSynthDepth 8→32` ✓；**Lean 没有的**（`PROBE_CAP` ⇒ 弃权）
   **去掉，不许换数字留着** ✗。落点用**现成管道** ✓（`CompileOptions` +
   `sokonanoda.toml [limits]` + CLI ✓）。⚠ **取证口已经在了** ✓
   （`SOKO_LIMIT_PROBE_CAP` / `SOKO_LIMIT_MAX_TRACKED` / `SOKO_LIMIT_MAX_HEARTBEATS` /
   `SOKO_LIMIT_MAX_REC_DEPTH` ✓，默认值**一个不动** ✓）—— ② 步就是换**默认值** ✓，
   判据现成：`scripts/limits-only-slow.sh --full` ✓。
3. **G-91 的乙类 4 处计数出口**（内核侧七个**已铺** ✓）：`judge.rs` 两张表容量 ·
   目标分解失败（`goals.rs`）· `SKELETON_MAX_LAYERS` ✓。复现件铺齐后**判 1** ⇒ 回来关账 ✓。
4. 排队在后面（做不完继续往下传 ✓）：**IA-4 元参数引擎余片** · **集合论教材线 S-A/S-B/S-C**
   （`docs/ONBOARDING.md` §0.2 是**唯一队列** ✓）。

## 4. 每条要带的判据（判红 / 判绿 ✓，以及「现在有没有」）

| 项 | 判据 | 现状 |
|---|---|---|
| 12 单元特性 | `cargo test -p sokonanoda-lsp --test lsp_keystroke_structure` ⇒ 改陈述 `prefix>0` / 改证明体 `prefix==0` | **有 ✓ 全绿** |
| 冷开不退化 | `cargo test -p sokonanoda-lsp --lib perf_course` ⇒ unit12 **≤8.5s** | **有 ✓ 绿**（**6173ms** ✓） |
| 增量身份等价 | `cargo test -p sokonanoda-front --test identity_probe` ⇒ `probed>0` · `uncomparable==0` · `mismatches==0` · `fallbacks==0` · `evictions==0` | **有 ✓ 全绿**（2612/0/0/0/0 ✓） |
| O(n²) 不许回来 | 同上第二个用例 ⇒ `reparse < 20`（实测 **0** ✓；`SOKO_NO_SEED=1` ⇒ **54** ⇒ 判红 ✓） | **有 ✓ 绿** |
| 闸类普查 | `cargo test -p sokonanoda-front --test gate_census` ⇒ 甲类闸 == 0 | **有 ✓ 绿**（反向：`MAX_DEPTH=1` ⇒ **59** ⇒ 判红 ✓） |
| **闸只变慢不变错** | `scripts/limits-only-slow.sh --full` ⇒ 三道闸拧到 1，输出**逐字节相同** ✓ + 闸**确实触发** ✓ | **有 ✓ 绿**（10589/51129/139358/1334 ✓） |
| G-88 预算不当成否 | `cargo test -p sokonanoda-front --lib meta::tests::exhausted_budget_…` | **有 ✓ 绿**（反向：换回 `Tri::No` ⇒ 判红 ✓） |
| 规模翻倍 ⇒ 耗时翻倍 | **真实课程** ✓：`SOKO_DECL_PROFILE=1` 逐声明耗时**平线** ✓ · **最坏形状** ✗：`by_calls` 比值 **4.13** ⇒ **G-92** ✓ | **有 ✓（两条分开记 ✓）** |
| **判定前缀不再重查内核** | `SOKO_JUDGE_ENV_VOUCH=shadow SOKO_JUDGE_ENV_PROBE=1` ⇒ `shadow_diff == 0` 且 `shadow_same > 0` | **有 ✓ 绿**（43/59 · 0 ✓；反向：恒不等 ⇒ **17** ⇒ 判红 ✓） |
| 真实单元的收益 | `unit12-synthesis` 冷缓存墙钟（release ✓） | **有 ✓**（**10.97 → 5.75s** ✓） |
| 闸类复现件 | `bash docs/gaps/repro/G8{8,9}-*.sh G9{0,1,2}-*.sh` ⇒ exit 0（缺口仍在 ✓） | **有 ✓ 五条全 0** |
| 可配置化零行为 | 整本课程 `build --json` 剔心跳逐字节相同（**改前 vs 改后都要跑** ✓） | **无 ⇒ 先建**（第 ③ 步用 ✓） |

## 5. 已知的坑（都带实测代价 ✓）

* ⚠ **判据会「全被跳过」** ✗：上一棒只读「有没有 `MISMATCH`」⇒ 读到 **0** ⇒ 判成「已证等价」✗，
  而当时**所有**条都落在「前缀解析不过 ⇒ 退回原文 ⇒ 这次不比」里 ✗（`unit08` **1505/1540** ✗）。
  ⇒ **凡"逐条比对"的判据，必须同时报「比过多少条」** ✓（`probed > 0` 是防空转的硬要求 ✓）。
* ⚠ **`canonical_prefix_id` 必须用 `parse_fragment`** ✗：判定的前缀是**文件片段** ✓，
  声明落在 `namespace` 里时**必然**停在未闭合处 ✓（G-05 §4.1）。换回严格 `parse`
  ⇒ `uncomparable` **0→1906** ⇒ 判红 ✓。
* ⚠ **身份必须从 AST 直取** ✗：片段用了**依赖声明的记法**时，片段与整体**都解析不过** ✓
  ⇒ 「切片段再 parse」必退原文（实测 27 处 ✗）。
* ⚠ **判官合成的声明 span 是零长** ✗（`line: 0, column: 0` ✓）：它在 `file.commands` 里、
  **不在 `file.src` 里** ⇒ 文本路看不见、AST 累加看得见 ⇒ 分叉（18 处 ✗）。
  `command_env_id` 已跳过零长 span ✓。
* ⚠ **`canonical_prefix_table` 的 `CAP` 必须 ≥ 工作集** ✗：`4096` 比整本课程还小 ⇒
  满则挤掉**活条目** ⇒ **抖动**（`identity_parses=3062` ✗）。现 **65536** ✓ + 淘汰计数 ✓。
  **两处 CAP 必须同数** ✗（种的那侧与读的那侧 ✗）。
* ⚠ **探查（`PROBE_CAP`）**：耗尽**只许**表示「这次探查不可信 ⇒ **弃权走全量**」✓
  （`probe_pass` 读 `probe_exhausted` 后判**未决** ✓），**绝不许**表示「不相等」✗ ——
  动它之前先把这条钉住 ✓。
* **子进程 stderr** ✗：服务端探针看不见 ⇒ 必须 `SOKO_LSP_TEST_STDERR=1` ✓（`start_traced` 的读线程
  **只留 `LSP_TRACE` 行** ✗ ⇒ 别的探针行会被**丢掉** ✓ —— 要在 CLI 上跑才看得见 ✓）。
* **进程级计数器** ✗：判据要取**差量** ✓；课程级断言放**独立进程**（集成测试 ✓）。
* **`target/debug` 跑不动整本课程** ✗：`build courses/set-theory` **stack overflow（exit 134）** ✓
  —— **基线二进制同形** ✓（不是新引入的 ✓），release 正常 ✓ ⇒ 量课程**一律 release** ✓。
* **release 产物会过期** ✗：改源码后只 `cargo test` 只编 debug ✓ ⇒ 量 release 前必须
  `cargo build --release -p sokonanoda-cli -p sokonanoda-lsp` ✓。
* **`AGENTS.md` 行数上限 435** ✓（`docs-lint` ⑦ 判红 ✓，而 `pre-commit` **不跑** docs-lint ✗）；
  **上限只许收紧** ✗ ⇒ 加内容要**折进既有行** ✓。
* **bash 3.2**：变量后紧跟**多字节字符**必须写 `${var}` ✗；`sed 's/.*passes=/'` **贪婪** ✗。
* **python heredoc 里别用 ASCII 引号** ✗（中文串里一律 `「」` ✓）—— 本棒在 Rust 字符串里
  也踩了一次同形 ✓（`"没人写"` ⇒ 编译错 ✓）。
* **残留进程** ✗：长跑脚本要自己 `pgrep -f <名字>` 收 ✓（本棒收尾自查为空 ✓）。

## 6. 本会话已兑现（可复核 ✓）

* **① 「编辑慢」那条 O(n²) 真除掉了** ✓（用户点名 ✓）：`unit12` **10490 → 6173ms（1.70×）** ✓ ·
  `fallbacks` **536 → 0** ✓ · 身份分歧 **1540 → 0** ✓ · `identity_parses` **3062 → 0** ✓；
  根因**三条全是实测定位** ✓（严格 `parse` 把片段判死 · 增量身份切片段重解析 · 记忆表比工作集小 ✓）；
  复现件 + **两条反向验证** ✓；整本课程 `--json` **逐字节相同** ✓。
* **② 甲类闸**：内核侧**七个计数出口** ✓ + 接进 `STAGE_STATS` ✓ + 判据 ✓ + 五条复现件 ✓；
  **G-88 真修** ✓（预算耗尽 ⇒ 加大预算重试 ✓，不再「当成否」✗，带反向验证 ✓）；
  **G-89 / G-90 逐处核实** ✓（两条**本来就是**「只变慢」✓ —— 台账原先的诊断**有误** ✗，已更正 ✓）
  + **实验判据** `scripts/limits-only-slow.sh` ✓（三道闸拧到 1 ⇒ 整本课程判定**逐字节相同** ✓，
  而闸确实触发 10589/51129/139358/1334 次 ✓ ⇒ **不是空转** ✓）。
* **新发现 G-92** ✓（做「规模翻倍」判据时实测 ✓，如实登记 ✓）。
* **记账** ✓：G-88/89/90/91/92 的 `repro` + `today` ✓（`gap.py check` **93/0** ✓）。
