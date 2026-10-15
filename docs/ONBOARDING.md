# 接手路径（X1）—— **读这几个就够开工** ｜ 本文提到的 `docs/**` 指针都是**历史位置**（那批设计文档已收口删除）：原文 ⇒ `git log --all -- <path>`。doc-refs: history-only

> **这份文件是「接手成本」与「计划」的唯一权威** ✓（2026-09-29 起它 = 接手路径 **+ 开工单**，
> 原 `docs/NEXT.md` 已并入 ⇒ **不再有第二个文件**；2026-09-30 起 `STATUS.md` 与
> `ROADMAP.md` §10 **不再各自维护"下一步/队列"** ⇒ **队列只有本文件 §0.2 一份** ✓）。
> 判据 `python3 scripts/docs-lint.py` 的 **⑦**：下表的每个文件**必须存在**、**不得超过它的上限**、
> **全表合计不得超过 `docs-budget.json` 的 `onboarding_max_lines`** ✓。
> ⚠ **不许靠抬上限达标**（用户 2026-09-26：「把上限从 3 MB 抬到 10 MB **不算完成**」）⇒ 上限**只许收紧** ✓。

## 必读路径（全读；其余文件按需查，不必通读）

| # | 文件 | 读什么 | 行数上限 |
|---|---|---|---|
| 1 | `AGENTS.md` | 硬规则速记 + 命令 + 收尾义务（**本文件由 harness 自动读**） | 435 |
| 2 | `docs/ONBOARDING.md` | **本文件**：接手路径/预算 + **开工单**（现在在哪 / **唯一队列** / 判据 / 纪律） | 260 |
| 3 | `REQUIREMENTS.md` | 权威总账：硬规则全文 + §9 用户新要求 | 267 |
| 4 | `STATUS.md` | 当前快照 + 最近 3 轮 + **未决项**（**队列不在这里** ⇒ 见本文件 §0.2） | 240 |

**实测合计 991 / 1000**（2026-10-12：`AGENTS.md` 435 · 本文件 200 · `REQUIREMENTS.md` 267 · `STATUS.md` 89）
—— ⚠ 上表第 4 列是**逐文件上限**，**相加无意义**（逐文件由判据⑦ 核，合计上限另有 `onboarding_max_lines` = 1000）⇒ 别把 435+260+267+240 当成「已超标」✗。

**按需查（不进必读表，但接手第一个小时大概率会碰）**：

- `ROADMAP.md` **§10**（每条待办的**验收口径**；**队列不在这里**）· `docs/architecture.md`
  **§6/§8**（内核改动台账 + gotchas，**动内核前必读**）· `docs/protocol.md`（`--json` / `query`
  契约，**改输出前必读**）· `docs/PERF.md` · `docs/TESTING.md` · `docs/E2E.md`；
- **计划/提案类文档的索引 = §2**（一个入口查全部：讲什么 / 什么状态 / 何时读）；
- 查历史：`docs/archive/README.md`（归档索引，**归档 ≠ 销毁**，判据⑥会查它有没有点名 ✓）。

## 四层与预算（判据 ⑦ 同时核这里）

| 层 | 是什么 | 预算 |
|---|---|---|
| **L0 热区** | `AGENTS.md` `README.md` `ROADMAP.md` `REQUIREMENTS.md` `STATUS.md`（仓库根） | `layer_max_lines.L0` |
| **L1 规格** | `docs/` 顶层（开发者参考：架构 / 协议 / 测试 / 性能 / 开工单） | `layer_max_lines.L1` |
| **L2 台账** | `docs/design/` `docs/notes/` `docs/gaps/` `docs/perf/` `docs/e2e/`（设计与取证） | `layer_max_lines.L2` |
| **L3 归档** | `docs/archive/`（历史层；**不进活文档预算**，但**必须在索引里点名**） | `layer_max_lines.L3` + 判据 ⑥ |

**判据分工**（全在 `scripts/docs-lint.py`，已进 `scripts/soko gate` 与 CI）：
① 活文档总量 · ② 单文件行数 · ③ 入口文件行数 · ④ 既有非入口文档**冻结**（只许减不许增）·
⑤ `docs/**` 垃圾残留 · ⑥ 归档必须被索引点名 · **⑦ 本文件的必读路径**（存在性 + 每文件上限 + 合计上限）。

**怎么用**：① 接手按上表 1→4 读完，再按需查上面的清单；`ROADMAP.md` 只读 §10 ✓。
② 进必读表的文件要涨行 ⇒ **必须同时**在 `docs-budget.json` 抬它的 per-file 上限**并**说明从哪省回来
（合计上限**不动**）。③ 归档 ⇒ 必须在 `docs/archive/README.md` 点名，否则判据⑥判红。

---

# 开工单（现在在哪 / 唯一队列 / 判据 / 纪律）

> 快照：**2026-10-01 · 第 523 轮**（`v0.79.0` 已发布并闭环）。
> **权威顺序**：用户口头要求 > `REQUIREMENTS.md` §9 > 本文件 > 其它。
> ⚠ 本文件的"队列"是**唯一**的一份：`STATUS.md` 只记快照与未决项，`ROADMAP.md` §10 只记**验收口径** ✓。

## 0. 现在在哪

**一句话**：**2026-10-12 清台账** —— 七处"没做完的"收成一个机器可重算队列（§0.2 + `python3 scripts/ux-queue.py`）；
`v0.79.0`…`v0.88.2` 各批次的读数与反向验证 ⇒ `git log --all -- STATUS.md` ✓（`JUDGE_PREFIX runs` 3759→791 ·
全课 `build` 214.08s→**47.8s** · `judge_ms` −83% · `--json` 0 行不同）。

### 0.2 下一步（**唯一队列**；机器入口 = `python3 scripts/ux-queue.py`）

<!-- QUEUE:SUMMARY:BEGIN -->
<!-- QUEUE:COUNTS total=5 blocker=0 high=2 medium=3 low=0 -->
**未完成 5 条**（blocker **0** · high **2** · medium **3** · low **0**）· 取队列：`python3 scripts/ux-queue.py --next`（或 `--json`）· 认领/收口：`--claim <id>` / `--advance <id> --to done --commit <sha> --reading '<读数>'`
最靠前的 5 条：`SUB-G117-close-now` · `SUB-orchestrator-attention-structure` · `COMPILE-CACHE-GB-ENVS-NO-GUARD` · `E2E-LEDGER-JOB-RACES-LONG-GATES` · `ZERO-OBS-GATE-CENSUS-BY-BEHAVIOUR`（全量见 `--json`；本块**由脚本渲染**，手改会被 `--check` 判红 ✗）
<!-- QUEUE:SUMMARY:END -->

> **口径（2026-10-12 清台账 + 事件调度器）**：**明细只活在脚本里** —— `--next` 取队列 · `--claim <id>` 认领（**最多 4 个并发（"4 核"）**，写锁 + 
> **同一模块的写者互斥**；槽位满或撞资源即判红）· `--advance <id> --to done --commit <sha> --reading '<判据读数>'` 收口（**没有证据不许收口** ✗，
> 规则写在脚本的迁移表里，不靠提示词）；拆解大项用 `--add-child <父> --child <新id>`（子任务插在父之前，父转 umbrella）。
> 处置**只有两种**：**实现** 或 `wo-filed`（定案不做 + 理由）—— **"排期 / 以后做"不是处置** ✗（大工程可跨轮，
> 但每轮必须交**已合入的读数**）。本表只列**粗粒度的线**（明细见 `--json`，条数见上方摘要块）；摘要块由 `--render` 生成 ✗ 手改判红。

| 线 | 是什么 | 下一步最小动作 | 判据（可执行） |
|---|---|---|---|
| Q | 调度器自身的缺口（接口 agent 的 **13 条提交** + `Q-1`） | `--submit`/`--inbox`/受理四态/`--claim` 原子性（**已修**）· 其余按 `--inbox` 逐条受理 | `python3 scripts/ux-queue.py --inbox` 无 `todo` 残留 + `--selftest` exit 0 |

> **`G-110` 已收口**（2026-10-12 · commit `ead7829f`）：`assumption` 现在搜遍全部局部假设
> （逃生门 `SOKO_ASSUMPTION_FIRST_ONLY=1` 复现旧行为）⇒ 已从本表摘除 ✓。

**别的台账是守卫输入、不是队列**（`docs/{gaps,e2e,perf,courses}/ledger.jsonl`：`gap.py check` 逐条重跑复现件 ⇒ **归档 = 关掉守卫** ✗）；
索引 `docs/ux-findings-2026-10-12.md` 到期 **2026-11-12** ⇒ 到期前把仍开项搬进队列 ✓（**本表只留粗粒度线**）。

## 1. 读数（同机同口径：release · 冷缓存 · `SOKONANODA_BUILD_JOBS=1` · 全课程 `build --json courses/set-theory`）

| 读数 | `off`（基线） | P1-a | **P1-b 收口（默认全开）** | **§3.C（前缀环境）** |
|---|---|---|---|---|
| `JUDGE_PREFIX runs` | 3759 | 1881 | **791** | 791（那一刀省的是**每趟的前缀重编**，不是趟数） |
| `JUDGE_PREFIX bytes` | 174,213,583 | 93,858,420 | **43,329,596** | — |
| `passes` / `doc_passes` | 4126 / 266 | 2248 / 266 | **1157 / 265** ⚠ | — |
| `judge_ms` | 146,580 | 120,359 | 113,835 | **19,223（−83%）** |
| **墙钟** | **216.14 s** | **158.90 s** | **126.80 s**（P1-c 双数字，1.688×） | **47.8 s**（§3.C，2.65×） |
| `--json`（剔 `build.tick`/`build.progress`） | 基线 | 0 行不同 ✓ | **0 行不同** ✓（2691/2691） | **0 行不同** ✓ |
| shadow（判据级） | — | diff=0 | **diff=0** | **`shadow_same=265 · diff=0`** |

⚠ `doc_passes` 266 → 265 是**计数口径**差 1（输出逐字节相同 ⇒ 不影响判定），**如实记账**。
**开关**：`SOKO_JUDGE_INPLACE=off|shadow|on`（默认 `on`，三档全含）；单独回退
`SOKO_JUDGE_INPLACE_WIDE=0` / `SOKO_JUDGE_INPLACE_BY=0|off`；§3.C 的逃生门
`SOKO_JUDGE_ENV_VOUCH=0` / `SOKO_JUDGE_ENV_REUSE=0`；§32.4 (iii)（合成文档把前缀 `theorem`
装成不透明常量）的逃生门 `SOKO_JUDGE_PREFIX_OPAQUE=off` / 影子档 `=shadow`。
**发版**：`v0.79.0` = Latest（CI `36803030466` **28 success / 0 failure**；26 资产 = 8 CLI + 8 LSP + 9 VSIX + 1 SHA256SUMS）。

## 2. 计划/提案类文档索引（**一个入口查全部**；新计划**不许**再开新文件 ⇒ 写进本文件）

| 文档 | 讲什么 | 状态（2026-09-30） | 什么时候读 |
|---|---|---|---|
| `docs/design/e2-plan.md` | E2 计划**现行契约 + 收口索引**（§0 三条硬约束 · §1 阶段总览 · §13 一行一条清单）；逐字原文 ⇒ `git log --all -- docs/design/e2-plan.md` | **E2 阶段 A–E 全收口**；**批次 N 25/28**（`plan.py` 总进度 **63/66**，剩 T-N13/T-N14/T-N15 ⇒ `plan.py next` 权威） | 做批次 N 前（先跑 `plan.py next`） |
| **`docs/design/tactic-docs.md`** | **tactic 文档体系**（hover 摘要 · F12 跳文档 · `reference/tactics/**` 完整介绍）· **覆盖基线 = 全部 14 条**（§1.1 清单，含 `match` 这条特殊项）· **四层防漏项机制**（表派生 + `include_str!` + 手写清单判据 + 独立源码 lint，§4.2）· 八阶段 P0–P7 的交付物与**可执行判据** · **26 条风险** · **§4.5 14 条摘要已定稿** · §4.6 物化路径与 `clean` 的关系 · **§4.7 页面模板 + 判绿样板** · §1.3 错误消息逐条喂内核抄回 · **§7 边界 9：`apply` 行为修复已改派 `d9a701e4`，不归本文档** · **§4.9 D10：文档随插件打包（两条通道一份源 · 五档解析 · 四个装配线接线点 · 打包/漂移判据）** · **§12 设计自审（六条）+ 四道机械自查** · **§4.7.4 正文文风**（用户 2026-10-10 拍板：**说人话、教材式连贯叙述**，示例逐个喂真内核判绿）· **评审确认三条**（语言关键字 hover 单独立项本轮不做 · `apply` 文档以修复线 `d9a701e4` 落地当天行为为准 · `exact?` 未实现 ⇒ 不进基线、文档不写假话）· **无遗留待决项** | **设计 ✓ 已评审 · P1（机制层）已落地（2026-10-10）· P2–P7 待做**（P1 = 单一真相表 + 四层防漏项门禁 + 14 篇骨架；读数见 `STATUS.md`） | 动 tactic 关键字名单 / `tactic_goal_hover` / `goto_definition` / 写 tactic 文档前 |
| **`docs/design/metavar-engine.md`** | **IA-4 元参数引擎**：现状/差距（含「合一不解 G-48」的更正）· 引擎设计（接口/算法/集成点/sort-kind/报错契约/性能纪律）· 缺口面与边界 · **切片 M0–M4** · **决策点 D1–D7** | **设计 ✓（2026-10-01）· 实现未开始**（先收 D1） | 开 IA-4 线 / 动 `solve_prefix`、`unify_extract`、`fill_pending_by_shape` 前 |
| `docs/design/module-artifacts.md` | 模块级产物三块（内核环境 / 前端表 / 报告）· per-module Merkle 键 · §8 产物落盘 · §9 一个 builder 贯穿全场 | **切片 1 已收口**（`build_one(…, precomputed)` + `PassTables`）；**仍未做** = 产物落盘（P3 可下载 cache） | 做**模块级产物 / 结果复用**那条线前 |
| `docs/design/notation-subset.md` | 用户自定义记法子集（`infix:N` 族 + 零元 `notation`）的边界与第二刀清单；§17 是 E19 行 | 第一刀 0.59.0 落地；**E19 已收口（`v0.79.0`）**；后继 = `metavar-engine.md`（IA-4） | 动记法 / 做 IA-4 前 |
| `docs/design/v077-kernel-deficiencies.md` | 未做完的四章（ST6/7/9/11）的**根因定位**（G-56/58/59） | **仍是 E19 的取证材料** | 开 E19 前 |
| `STATUS.md`（未决项 + 最近 3 轮） | 轮次流水 · **当前未决项**（**不含队列**） | 每轮收尾更新 | 每次开工 + 收尾记账 |
| `scripts/docs-gc.py` | **文档清理机制**（报告式）：零引用 / 不在权威链 / ≥30 天没动 ⇒ 报候选，**不自删** | 机制已就位 | 每次"文档又堆了"的反馈后 |
| `crates/front/tests/judge_inplace{,_on,_wide,_by,_by_reverse}.rs` | 三个档位（+ `by` 两档）的**判据模式**（照抄即可） | 在跑 | 加新接线点时 |

## 3. 纪律与踩过的坑（照抄，别重踩）

1. **推送节奏**：一次 CI **median 24.6 分**，而**本机同一条活 217 秒 ⇒ 快约 6 倍**。**四条**：
   **(a) 中间环节不推 CI** · **(b) 攒到发版点一次推** · **(c) 推完不许 `--watch` 盯着等**
   （等的时候 agent 完全停工）· **(d) 不许连推**（`cancel-in-progress: true` ⇒ 后推的顶掉前一轮）。
   ⚠ **推送前三步**：`git fetch` → `gh run list`（有在飞就别推）→ `pull --rebase` 再推 ——
   CI 的 `e2e ledger (commit back on main)` job **每轮结束都往 main 回写提交**，不 fetch 必被拒。
2. **口径**：报数先贴 **failed + 三条计数**（`JUDGE_PREFIX runs/bytes`、`hits/misses`、judge pass 数）
   **再谈墙钟**；计数减半 ≠ 墙钟同比；**<5% 差不许当结论**。
3. **两条红线**：`--json` **剔除 `build.tick`/`build.progress`**（按设计随墙钟变的心跳）后**逐字节不变**；
   **反向判据**（依赖/前缀真变 ⇒ 必须重算）必须有**实测**。
4. **"只做未命中"不许破**：就地路径对**每一次**调用生效会把十几万次廉价命中换成 ~1ms/次 ✗
   （实测 400s 跑不完）⇒ 新接线点先问"这一趟在慢路上原本花不花前缀钱"。
5. **判据不许空转**：每个新接线点的测试都要断言"**就地路径真被走到**"（`INPLACE_USED` 增长）；
   夹具没踩到接线点时它**当场判红**。凡写「必红 / 强制 / 例行」的守卫，**逐条核对它真的拦**
   （`continue-on-error` / `|| true` / 守卫的输入是否真的存在）。
6. **三个实测坑**（附七 §4 有全文）：① pp 必须与 `kernel_phase:199` 同档设 `proofs=true`，
   且 `quiet_catch` 要包在 `with_env` **内层**；② 判定查询文本要用 `render_expr`（**不是** pp
   回读入口，记法会解析失败）；③ 就地路径**不解析**（源 AST 直接造项），否则每问解析 46KB ✗。
   ④ §3.C 的**关键一行是 `before.min(prefix_commands)`** —— 两个坐标系不同（AST 序号含 `import`，
   judge 前缀文本走 `importless_source`）⇒ 不夹会多担保合成声明 ⇒ 判定声明没被检查 ✗。
7. **记法路径守卫**的指纹 = **去空白后的整行** ⇒ rustfmt 把闭包折成一行会变成"新增绕过"；
   改判定查询文本的形状前先看 `scripts/notation-paths-baseline.txt`。
8. **同一处连红 3 次 ⇒ 换招或降级**，别闷头试第 4 次；**半成品不上机**（宁可留可执行计划）。
9. **别钉并行下的"完成顺序"**（2026-10-01，push 前 gate 抓到）：`build.progress` 的 `done` 在
   **入口级并行**下按**完成顺序**发 ⇒ 断言"必须按文件名递增"= **假红**（同一夹具连跑 6 次 **4 次红**）。
   协议（`docs/protocol.md`）只要求「每个文件编完**就**报 + `done` 是**绝对值** + 全部排在**结果段之前**」
   ⇒ 判据用**集合**（`sorted(dones) == 1..=total`）；结果段那条确定性红线**另有**断言 ✓。
10. ⭐ **判据不许由被复核的东西自己定义**（2026-10-15，`SUB-orchestrator-attention-structure`
    【1】/【2】）：`scripts/ux-queue.py` 同时定义队列语义 · 守卫 · "证据"规则 · **复核判据**
    ⇒ 工具错 = 账错 = 过程错，**没有外部检查**能发现它 ✗（它本会话被改 **54** 次、自报 **16**
    轮"独立"复核，而复核者 = 实现者）。⇒ 复核者要**换一份实现**（差分测试）+ 只读**原始读数**
    （原始事件流 · `git merge-base --is-ancestor`），被复核者的自报只是**被告**：
    `python3 scripts/check-queue-views.py`（C1 原始层卫生 · C2 两份实现折出同一状态 ·
    C3 两层不漂移 · C4 指针祖先性（折过 `repoint` 的**最终**指针）· C5 证据可独立复算
    **ratchet** · C6 外部性自证）。同族两条：**收口证据必须可独立复算**（命令或仓库内路径；
    只指 `.cache/**`（git 忽略）**不算** —— 实测 **179/199 = 90%** 的收口证据复算不了 ✗）·
    **簿记预算** `python3 scripts/check-attention-budget.py`（⭐ **2026-10-16 起是真 ratchet**
    —— 用户 2026-10-15【②】）：**本轮生效的两档在 `docs/attention-budget-ratchet.json`**
    （tracked ⇒ 逐轮读数进 git ✓），由 `--open-round` **每轮降一档**（`SHARE_MAX` 每轮
    −2 个百分点 ✓），红线定义 = 「**本轮没比上轮低 ⇒ 判红**」✗→✓；**种子仍是
    19:1 / 60%**（= **上界**：`tighten()` 单调不增 ⇒ 一个阈值都没放松 ✓）。
    ⚠ **改前它是装饰** ✗：两档贴在实测**上方**（实测 **10.14 : 1** / **56.6%**）⇒ 永远绿；
    而第一版贴实测取 17.0 ⇒ 落地当刻判红 ⇒ **抬到 19.0** ✗（用户 2026-09-26：
    「**不许靠抬上限达标 —— 上限方向只能是收紧**」）。
    ⚠ **谁开轮不是纪律** ✓：基点之后落超过 `ROUND_MAX_COMMITS = 40` 笔还没开轮 ⇒
    **默认跑就判红**（ratchet 空转 ✗）；消红的唯一出路是 `--open-round`，而它要求
    **读数真的降了**（多落产品 ✓）⇒ **红有出路**，不是假红生成器 ✓。
    **`--open-round` 另有 ③⑤④ 三道闸**：**③ 每轮至少 1 条用户面判据**（`--user-reading`，
    **真编辑器**命令 + `⇒` 实测读数 ⇒ 形状可判 ✓）· **⑤ 收口判据绑用户现场**
    （`--user-case` 必须点到一个**存在的**仓库路径 ✓）· **④ 新守卫必须自带咬住证明**
    （调 `scripts/check-guard-bite.py --check --since <基点>` ✓）。
    **咬住证明** `python3 scripts/check-guard-bite.py`（台账 `docs/guard-bite.jsonl`；
    `--check --all` 逐条真跑：`pass` 要绿、**`bite` 必须真的报非 0** ✓；覆盖基线以来
    新增/修改的守卫缺一行 ⇒ 判红 ⇒ **不算收口** ✗；`--reopen-empty` = ① 的翻账入口 ✓）。
    两条都进 CI 的 `queue-lint` **与 `scripts/soko gate`**（每推必跑 ✓）。

### 3.1 文档清理机制（2026-09-29 用户：「没有建立起来很好的 doc 清理机制」）

* **一条命令**：`python3 scripts/docs-gc.py`（`--json` / `--days N`）——报**候选**（零引用 ·
  不在权威链 · N 天没动），**只报不删** ✓（同 `perf-gate` 的先例：先报告，别一上来就拦）。
* **处置三选一**（落 commit ⇒ 评审可见）：① **删**（默认口径：并入本文件后删除，**不归档**）·
  ② 移进 `docs/archive/` 并在 `docs/archive/README.md` 点名（判据⑥会查）· ③ 加进脚本的
  `KEEP_ALWAYS` 并写明理由。
* **本文件是唯一计划入口** —— 新计划**不许**再开新文件；要写就写进 §0.2 与 §2 的表 ✓。

### 3.2 文档过期日期机制（2026-09-30 用户：「每个文档必须有过期日期，到期后审查」）

* **每个活文档都要有过期日期**；权威 = `scripts/docs-expiry.json`（过期日 + 上次续期 + **理由**；
  ⚠ **只走脚本、不许手改**，理由必填 ≥8 字）。**不加 frontmatter 的理由**：判据 ④ 冻结既有
  非入口文档行数 ⇒ 上百份各加一行 = **必然判红** ✗。口径 = 根 `*.md` + `docs/**.md`，**排除**
  `archive|perf|gaps|e2e` ⇒ `docs/gaps/README.md` 按口径被排除（归 `gap.py check` 管），是
  **已知边界**不是漏登记 ✓。
* **一条命令**：`python3 scripts/docs-expiry-check.py --check`（`--json` 机器可读）——
  **退出码就是判据**：**已过期 / 未登记 = 1（阻塞提交）** · **只快过期（≤30 天）= 0（只提醒）**。
  分档：**入口/规范 12 个月**（entry/spec）· **过程/量测 3 个月**（process/record）· 单次续期
  ≤2 年（逼出周期性复审 ✗ 一次续到天荒地老）。
* **提交前自动检测**：`scripts/install-hooks.sh` 装一次 ⇒ 之后**每次 `git commit`** 自动跑 ✓；
  已过期/未登记 ⇒ **拒绝提交** ✓。逃生门 `--no-verify` / `SOKO_SKIP_HOOK=1` ⇒ **必须写进
  `STATUS.md`** ✗ 不许当默认动作。
* ⭐ **到期审查（后续 agent 的义务）**：见到已过期 ⇒ **先审查再处置**，**三选一**（都落 commit）：
  ① **续期**（**须有特定理由**：仍被引用 / 仍是活规范 / 有追溯价值）⇒ `--renew <文件> --date
  <新过期日> --reason "为什么还留"`；② **删除**（没人引用 / 已并入他处）⇒ `git rm`；
  ③ **归档**（有追溯价值）⇒ 移进 `docs/archive/` + 在 `docs/archive/README.md` **点名**（判据⑥查）。
  ②③ 后跑 `--prune`。**没有理由就该删** ——「续期」不是默认动作 ✗。
  自检：`--selftest`（21 条，含「**零扫描 ≠ 绿**」的反向验证 ✓）。

---

## 4. 给新会话的 prompt（可直接粘贴）

```text
【接手 · 先读开工单，别重新勘】只读 `docs/ONBOARDING.md`（§0 现在在哪 / **§0.2 唯一队列** / §3 纪律 / §2 按需查）
+ `STATUS.md`「未决项」；其余按 §2 的表按需查。队列逐条口径与读数：`python3 scripts/ux-queue.py`。
纪律：报数先计数后墙钟 · `--json` 剔 `build.tick` 逐字节不变 · 反向判据要实测 · `<5%` 不算结论 ·
**只做未命中**且断言"路径真被走到" · push 三步（`git fetch` → `gh run list` → `pull --rebase`）·
半成品不上机 · 同一处连红 3 次就换招或降级。
```

## 5. 收尾义务提醒（给值守/下一会话）

- **本文件是唯一的计划入口**（旧 `PLAN-0.74-0.79.md` 已删）⇒ 档位/队列/发版点变化时改**这里**，
  别只改 `STATUS.md`；`STATUS.md` 只留**最近 3 轮**，旧轮进 `docs/archive/status-removed-rounds-2026-10-13.md.gz`
  或 `docs/archive/`（后者不受冻结表管）。
- 提交前 `python3 scripts/docs-lint.py` / `python3 scripts/status-lint.py` / `python3 scripts/plan.py check`
  都要绿（本文件的必读表与合计上限记在 `scripts/docs-budget.json` 的 `onboarding` 一节）。
- 改了 `editor/vscode/**` ⇒ 同一轮更新 `CHANGELOG.md` + **版本号**（版本号 = 发布触发器）。
