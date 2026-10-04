# 内核线交接单（换会话用）

> 覆盖式重写于 2026-10-04 16:0x ✓（值守换会话单 ✓）。**只写当前状态** ✗（历史看 `git log` ✓）。
> 分支 **main**（本地 ✓，**未推送** —— 推送由值守独占 ✓）。（`docs-expiry.json` 登记 2026-11-15 ✓）

## 1. HEAD 与未推提交

* **结束 HEAD** = **`afb6dfbc`** ✓（G-88…91 台账 + 四条复现件 ✓）；本棒三笔：
  `d3754d1d`（O(n²) 真除）· `ae8c944d`（闸类计数出口）· `afb6dfbc`（台账/复现件）。
* **未推 149 笔** ✓（本会话未 push / 未打 tag ✓）；**工作树干净** ✓。
* 判据现状 ✓：`lsp_keystroke_structure` **3/3** ✓ · `perf_course` **5/5** ✓ ·
  `identity_probe` **2/2** ✓ · `gate_census` **1/1** ✓ · **front 全量 872/0** ✓ ·
  `gap.py check` **92 通过 / 不一致 0** ✓ · `status-lint` ✓ · 课程 **243 compiled / 6 failed** ✓。

## 2. 手上的 WIP

**没有** ✓ —— 三笔全部提交 ✓，无「在树里、默认关」的东西 ✓（`SOKO_NO_SEED` 是**逃生门** ✓，
默认**开** ✓；`SOKO_PREFIX_SEED` 那个旧开关**已删** ✓）。

## 3. 下一棒做什么（按序 ✓）

1. **③ 限制可配置化 + 按 Lean 4 对齐数值**（**唯一在排的下一步** ✓，值守 13:21/13:24 已合并成一件 ✓）：
   ① **可配置化**（**默认值一个不动** ✓、**零行为变化** ✓，判据 = 整本课程 `build --json`
   剔心跳逐字节相同 ✓）；② **放宽默认值单独一笔** ✓。**终点取 Lean 数值** ✓：
   `maxRecDepth 64→3200` · `maxHeartbeats 4096→20000` · `maxSize 64→128` · `maxSynthDepth 8→32` ✓；
   **Lean 没有的**（`PROBE_CAP` ⇒ 弃权 · `PARSE_LIMIT` ⇒ 退回原文）**去掉，不许换数字留着** ✗。
   落点用**现成管道** ✓（`CompileOptions` + `sokonanoda.toml [limits]` + CLI `--max-depth=N` ✓），
   **不新造配置系统** ✗。⚠ 动 `PROBE_CAP` 前先读 §5 的「探查」那条 ✓。
2. **G-91 的乙类 4 处计数出口**（内核侧六个**已铺** ✓）：`judge.rs` 两张表容量 ·
   目标分解失败（`goals.rs`）· `SKELETON_MAX_LAYERS` ✓。复现件 `G91-gate-counters.sh`
   会在铺齐后**判 1** ⇒ 那时回来关账 ✓。
3. 排队在后面（做不完继续往下传 ✓）：**IA-4 元参数引擎余片** · **集合论教材线 S-A/S-B/S-C**
   （见 `docs/ONBOARDING.md` §0.2，那是**唯一队列** ✓）。

## 4. 每条要带的判据（判红 / 判绿 ✓，以及「现在有没有」）

| 项 | 判据 | 现状 |
|---|---|---|
| 12 单元特性 | `cargo test -p sokonanoda-lsp --test lsp_keystroke_structure` ⇒ 改陈述 `prefix>0` / 改证明体 `prefix==0` | **有 ✓ 全绿** |
| 冷开不退化 | `cargo test -p sokonanoda-lsp --lib perf_course` ⇒ unit12 **≤8.5s** | **有 ✓ 绿**（**6173ms** ✓） |
| 增量身份等价 | `cargo test -p sokonanoda-front --test identity_probe` ⇒ `probed>0` · `uncomparable==0` · `mismatches==0` · `fallbacks==0` · `evictions==0` | **有 ✓ 全绿**（2612/0/0/0/0 ✓） |
| O(n²) 不许回来 | 同上第二个用例 ⇒ `reparse < 20`（实测 **0** ✓；`SOKO_NO_SEED=1` ⇒ **54** ⇒ 判红 ✓） | **有 ✓ 绿** |
| 闸类普查 | `cargo test -p sokonanoda-front --test gate_census` ⇒ 四个甲类闸 == 0 | **有 ✓ 绿**（反向：`MAX_DEPTH=1` ⇒ **59** ⇒ 判红 ✓） |
| 闸类复现件 | `bash docs/gaps/repro/G8{8,9}-*.sh docs/gaps/repro/G9{0,1}-*.sh` ⇒ exit **0**（缺口仍在 ✓） | **有 ✓ 四条全 0** |
| 可配置化零行为 | 整本课程 `build --json` 剔心跳逐字节相同（**改前 vs 改后都要跑** ✓） | **无 ⇒ 先建**（第 ③ 步用 ✓） |
| 预算耗尽有信号 | `[limits] max_depth=2` ⇒ 必须报错；`=10000` ⇒ 长证明过 | **无 ⇒ 先建**（第 ③ 步用 ✓） |
| 开发内环 | `scripts/dev-verify.sh [--granularity]` ⇒ 冷跑 ≤ 2s | **有 ✓** |

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

## 6. 本棒已兑现（可复核 ✓）

* **① 「编辑慢」这条线的 O(n²) 真除掉了** ✓（用户点名 ✓）：`unit12` **10490 → 6173ms（1.70×）** ✓ ·
  `fallbacks` **536 → 0** ✓ · 身份分歧 **1540 → 0** ✓ · `identity_parses` **3062 → 0** ✓；
  根因**三条全是实测定位** ✓（不是猜的 ✓）；复现件 + **两条反向验证** ✓；
  整本课程 `--json` **逐字节相同** ✓。
* **② 闸类普查第一步** ✓（值守 13:12 ✓）：内核侧**六个计数出口** ✓（`crates/kernel/src/gates.rs` ✓）
  + 接进 `STAGE_STATS` ✓ + 判据 ✓ + 四条复现件 ✓ + 台账真实读数 ✓
  （四个甲类闸**课程上 0 次** ✓ ⇒ 留闸 + 断言 ✓）。
* **记账** ✓：G-88/89/90/91 的 `repro` + `today` ✓（`gap.py check` 92/0 ✓）。
