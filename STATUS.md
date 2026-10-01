# 当前快照（2026-10-01 · 第 524 轮）

- **🚀 `v0.79.0` = Latest** ✓（CI `36803030466` **28 success / 0 failure**（1 skipped）· release
  workflow `36803986833` success · tag `v0.79.0` · 26 资产 = 8 CLI + 8 LSP + 9 VSIX + 1 SHA256SUMS）。
- **E19 甲案收口** ✓：三刀 + **默认开**（用户 2026-10-01 拍板：默认开 · 开关**保留为逃生门**
  `SOKO_NOTATION_METAVAR=0`/`off`）⇒ `docs/design/e19-baseline.md` **§9 as-built**（两态摘要 · 交叉验证 ·
  既有判据逐条重审）；台账 **G-48 ⇒ `fixed` / `fixed_in: 0.79.0`** ✓。
- **P 组（编译提速）全档收口** ✓：`JUDGE_PREFIX runs` **3759 → 791**（−79%）· 全课 `build` **216.14s → 47.8s**
  （**2.65×**）· `judge_ms` **−83%** · `--json` **0 行不同** · 影子档 `diff=0`；读数/开关/回退 ⇒ `ONBOARDING.md` §1。
- **三条用户实测 UI 缺陷 + Q1/Q2 + K1 线（按前缀复用）已收口** ✓（第 505/506/509 轮；逐条索引 ⇒ `docs/visible-changes.md`）。
- **计划与队列的唯一入口 = `docs/ONBOARDING.md` §0.2** ✓（**队列 #1 = IA-4 元参数引擎**：
  **设计已出**（`docs/design/metavar-engine.md`）· **实现未开始** ⇒ 开工前先收 **D1**（范围）✓）。
- **本文件的判据**：`scripts/status-lint.py`（≤240 行 · 禁词 0 · 每段 ≤30 · 净增 ≤60）+ 文档预算
  `scripts/docs-lint.py`（判据 ①–⑦，已进 `scripts/soko gate` 与 CI）。

- **批次 N 进度 66/66**（第 519 轮）✓：T-N13（第 518 轮）· T-N15（第 519 轮）⇒ 逐条索引 `docs/visible-changes.md`。
- **文档过期日期机制**（第 513 轮）✓：每个活文档都有过期日期（`scripts/docs-expiry.json`）· `git commit` 前自动检测 ⇒ 已过期/未登记**拒绝提交**。

## 第 524 轮（2026-10-01）：**IA-4 立项 —— 元参数引擎设计计划（只设计，不动实现）**

- **用户 2026-10-01 拍板**：下一大项目 = 把 E19 的窄版待定参数升级为**真正元参数**（Lean/Coq 式通用合一
  + sort/kind 检查），**先出设计计划** ⇒ `docs/design/metavar-engine.md`（§1 现状/差距 · §2 设计 ·
  §3 缺口面 · §4 切片 M0–M4 · **§5 决策点 D1–D7**）；**一片一档一 commit**（M0 基线+能力清单 →
  M1 引擎内核+记法路径 → M2 一般路径 → M3 sort/kind+报错契约 → M4 默认开+发版）✓。
- **盘点 + 关键更正**：求解器 = `solve_prefix` 两条路线 + `unify_extract` 单侧提取 + E19 的
  `fill_pending_by_shape`（域同形 ⇒ 取已解兄弟）；**合一本身治不了 G-48 的 `α`**（那位**没有任何约束**，
  今天让它绿的是「选择规则」）⇒ 引擎必须把那条规则显式化为 **defaulting**，否则 G-48 重新判红 ✓。
- **诚实的成本判断**：实参类型永远是**具体项** ⇒ 约束恒为单侧形状 ⇒ **接受面增量小**；真实增量 =
  ① **sort/kind 检查**（今天错了落内核 `def_eq mismatch expected: Sort(1) | actual: Sort(2)`）② 冲突检出
  ③ 一条机械取代三条 ④ **范围 B**（元变量进 elaborate 期 ⇒ G-30/G-33 一族）的**地基** —— 而 B 卡在架构上
  （`elab_expr` 边 walk 边造核项、内核无占位符）⇒ **只写立项条件，不排期** ✓。
- **基线（本机 release v0.79.0 实测）**：三指纹 `7646fe2e…`/`d0375577…`/`06370a38…`（**逐字节等于
  `e19-baseline.md` §9**）· 门禁 **43/377/99/0** · `runs=886 bytes=47,437,669` · `passes=1343` ·
  `by_calls=21,268` · G-48 默认 exit 0 / 逃生门 exit 1 ✓。
- **记账**：`ONBOARDING.md` §0.2 队列 #1 + §2 索引 · `docs-budget.json`（新文件登记 **300** + L2
  **7136→7396**，理由在 `_comment`）· 过期日期登记 · 第 521 轮归档 + 第 494 轮二次下沉进 L3 ✓；
  **未动任何实现代码** ✓。
- **待用户点头**：**D1 范围**（先只做 M0 能力清单 / 只做范围 A / 连范围 B 设计）等七条 ⇒ §5 ✓。

## 第 523 轮（2026-10-01）：**E19 默认开 —— 逃生门保留、默认开 + 发 `v0.79.0`**

- **用户拍板两点**：① **默认开** ✓；② 开关**保留为逃生门**（`SOKO_NOTATION_METAVAR=0`/`off` ⇒ 严格档），
  与 `SOKO_JUDGE_INPLACE` 同口径 ⇒ **两态反向验证照旧可跑** ✓（不删开关的理由就是这条）。
- **代码面**：只有 `implicit::metavar_enabled()` 一处（默认 `true`，只吃显式 `0`/`off`）；刀1/刀2 的接线点、`solve_prefix` 的返回形状、三个调用方**一字未动** ✓。
- **判据（默认态 / 逃生门）**：G-48 复现件 **exit 0 / exit 1** · 非课程 172 组 **2/172（恰好 G-48）/ 0/172**
  · 摘要 非课程 `7646fe2e…`/`43581e06…`(=R2) · **课程 `d0375577…` 两态都不动** · 全语料 `06370a38…`/`0231dcc4…`(=R4)
  · 课程门禁两态 **43/377/99/0** · front **771/0** · notation **49/0** · 两态判据各 **1/0** ✓。
- **交叉验证**：刀2 二进制 + `=1` 与默认开之后的二进制（不设开关）各跑 `--digest` ⇒ **三个 sha256
  逐字节相同** ✓ ⇒ 这次改动**只翻了默认值** ✓。
- **既有判据逐条重审**：front `solve_prefix_reads_the_expected_type` 末条（无期望类型 ⇒ 解不出）在
  默认态**不再成立**（`Or.inl` 的 `B` 跟同形兄弟 ⇒ `Or Nat Nat`）—— **用户复核这条后果后接受** ✓；
  判据改成两条（**严格档**仍 `None` + **默认态** `B := Nat`）✓；`notation.rs` **49/0** 全绿 ✓。
- **记账**：`e19-baseline.md` §9 · 台账 G-48 `fixed` · `ONBOARDING.md` §0.2（队列清空 + 一条可选）· `REQUIREMENTS.md` §9.5 · `notation-subset.md` / `e19-evaluation.md` 的"默认关"改口 ✓。
- **顺手修掉一条假红**（push 前 gate 抓到）：`cli_build_reports_file_progress_while_it_compiles` 钉了 `done` 的
  **完成顺序**（同一夹具连跑 6 次 **4 次红**）—— 那是 P1 入口级并行下的必然结果，协议只要求「每个编完就报 +
  **绝对值** + 全部排在**结果段之前**」⇒ 改成**集合**判据（反向验证：累计/漏报/重复仍红；连跑 **6/6 绿**）✓；
  **纪律进 `ONBOARDING.md` §3 第 9 条** ✓。
- **收尾**：`scripts/soko gate` **EXIT=0**（本地全绿）⇒ 一次 push ⇒ CI 绿 ⇒ auto-tag + release ✓；**下一条线由用户定** ✓。

## 第 522 轮（2026-09-30）：**E19 刀 2 —— 推广到 `solve_prefix` 一般路径**

- **切片**（同一个开关 `SOKO_NOTATION_METAVAR=1`，**默认关**）：机制上移一层到
  `implicit::solve_prefix`（应用 / 裸常量 / `by` 块的 `apply` 共用它）—— 先跑**严格档**（既有
  行为），失败且开关开 ⇒ 再跑**待定档**；刀1 的 `fill_pending_by_shape`/`metavar_enabled` 移进
  `implicit.rs` **共用一份** ✓；`solve_prefix` 的返回形状与三个调用方**都没动** ✓。
- **病根**（与 G-48 同形、入口不同）：`Set.Equiv {α β : Type}` 写成 `Set.Equiv ∅ {b}` ⇒ `α`
  只能从 `∅` 的类型解，而 `∅` 又要靠期望类型才定论域 ⇒ 鸡生蛋 ⇒ 专用码（**实测**）✓。
- **判据**：一般路径夹具 关态判红 / 开态**绿**（`α := β`）· `Set.Equiv ∅ ∅` 两态都红（不猜）·
  非课程 **172 组**：关态 **0 差异**、开态 **2/172**（**仍是** G-48 那一份 ⇒ 刀2 没再改语料）·
  关态三个 sha256 **逐字节等于刀0** · 课程门禁两态都 **43/377/99/0** · front **771/0**（+1）·
  notation **49/0** · 新判据 `crates/cli/tests/implicit_metavar.rs`（开绿 / 反向 / 不猜 /
  **既有判据开态仍红**）+ 真值层 `pending_solver_unifies_same_shape_siblings_and_never_guesses` ✓。
- **既有判据逐条重审**（4 条全部"不动"，表 ⇒ `e19-baseline.md` §7）：报码那条开态**仍报码** ✓；
  两条正向只会更宽 ✓；`@` 那条**不进**求解器 ✓；`protocol.md` 的码/hint 契约不变 ✓。
- **性能**：关态 `runs=905` / `passes=1362` / `by_calls=21,551` / `JUDGE_INFER 87,507`
  **逐项等于刀0** ✓（默认路径零开销）；开态 `runs=886`（−19）· `passes=1343` ⇒ **判定不变、
  工作量略降** ✓；墙钟 front 1.47s · notation 6.61s · 门禁 80.60s · 172 组 20.00s ⇒ **无退化** ✓。
- **下一步**：**默认开**（拆掉开关）+ 发版 `v0.79.0` —— ⚠ **需要用户点头** ✓。

## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ✅ **E19 甲案 = `v0.79.0`**（2026-10-01 **收口**）：**默认开**（逃生门 `SOKO_NOTATION_METAVAR=0` 保留）
  + 发版 ✓ ⇒ as-built `docs/design/e19-baseline.md` **§9**；台账 G-48 ⇒ `fixed` / `fixed_in: 0.79.0` ✓。
  **后继 = IA-4 元参数引擎**（设计 ✓ ⇒ `docs/design/metavar-engine.md`；开工前先收 D1）✓。
- ✅ **批次 N 全档收口（66/66）**：T-N13（第 518 轮）· T-N15（第 519 轮）✓ —— 逐条索引
  `docs/visible-changes.md`，遗留见 `docs/design/e2-plan.md` 的 T-N13 as-built「遗留」。
- ⚠ **E20 乙案不做**（用户拍板）；缺口根因 ⇒ `docs/design/v077-kernel-deficiencies.md` §三。

## 硬事实（接手先读这 6 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 · `docs/architecture.md` §6/§8）
- **批次制**：一个批次**只 push 一次** ✓；**等流水线不轮询** ✓（`gh run watch <id> --exit-status`）· **性能门禁只跟同宿主比** ✓（`AGENTS.md` §CI 节奏 / §性能回归门禁）
- **文档预算** ✓：`docs-lint` 判据 ①–⑧（**常量在脚本里**，别抄旧数字 ✗）——**接手成本**是判据 ⑦ 的**会判红的数字**（`docs-budget.json` 的 `onboarding`，上限**只许收紧** ✓）；**本文件**另受 `status-lint` 约束（≤240 行 · 净增 ≤60 · **只留最近 3 轮**，旧轮 ⇒ `docs/STATUS-ARCHIVE.md` ⇒ `.gz` ✓）

---
