# 当前快照（2026-09-30 · 第 510 轮）

- **🚀 `v0.78.3` = Latest** ✓（CI `36655613663` **28 success / 0 failure**（1 skipped）· release
  workflow `36656518982` success · tag `v0.78.3` · 26 资产 = 8 CLI + 8 LSP + 9 VSIX + 1 源码）。
- **P 组（编译提速）全档收口** ✓：`JUDGE_PREFIX runs` **3759 → 791**（−79%）· 全课 `build` 墙钟
  **216.14s → 47.8s**（P1-c 双数字 **1.688×** + §3.C **2.65×**）· `judge_ms` **−83%** ·
  `--json`（剔心跳/进度）**0 行不同** · 影子档 `diff=0`。读数/开关/回退 ⇒ `docs/ONBOARDING.md` §1。
- **三条用户实测 UI 缺陷 + Q1/Q2 + K1 线（按前缀复用）已收口** ✓（第 505/506/509 轮，
  各带**反向验证**与**值**判据；K1-a 的两态等价判据是第 509 轮补上的）。
- **计划与队列的唯一入口 = `docs/ONBOARDING.md` §0.2** ✓（2026-09-30 第 510 轮文档收敛：
  本文件只做**当前快照 + 最近 3 轮 + 未决项**，`ROADMAP.md` §10 只做**验收口径** ——
  三处不再各写一份"下一步"；ONBOARDING 257 → 180 行 · ROADMAP 706 → 480 行）。
- **本文件的判据**：`python3 scripts/status-lint.py`（≤240 行 · 禁词 0 · 每段 ≤30 · 净增 ≤60）·
  文档预算 `python3 scripts/docs-lint.py`（判据 ①–⑦，已进 `scripts/soko gate` 与 CI）。

## 第 510 轮（2026-09-30）：**文档收敛 —— 三份顶层计划文档合一 + 计划类索引落到一个入口**
- **用户工单**（与 K1 无关）：把"计划"类信息收敛到**单一顶层入口**。
- **① 三份合一**：`docs/ONBOARDING.md`（开工单 —— **唯一队列 = §0.2**，只剩"批次 N 3 条 +
  E19/E20"；已收口 7 条"一行一条 + 证据"）· `STATUS.md`（只留**当前快照 + 最近 3 轮 + 未决项**；
  未决项 3 → **2**：删掉已被第 506/508 轮取代的"P1-b 536 趟"与"前缀环境"）· `ROADMAP.md` §10
  （只留**验收口径 + 收口状态**；I6–I13 压成一行一条；删掉重复的"下一步"；§8/§9 标为历史快照）。
- **② 索引**：计划/提案类文档索引 = `docs/ONBOARDING.md` §2（讲什么 / 状态 / 何时读）；本轮
  **核实并改正两处状态** —— 切片 1b 交接书"待接" ⇒ **已收口**（`build_one(…, precomputed)` +
  `PassTables` + 多入口守卫都在，文件降级为 as-built，133 → 103 行）；批次 N 按 `plan.py` = **25/28**。
  `docs/README.md` 指向 §2（**不重复维护**，免得两处漂移）。
- **③ 归档**：两段 2026-09-28 陈旧快照 + 第 502/503/504/506/507 轮 ⇒
  `docs/archive/status-removed-rounds-502-504-2026-09-30.md.gz`（原文逐字；判据 ⑥ 已点名）。
- **读数**：ONBOARDING **257 → 180** · STATUS **193 → 95** · ROADMAP **706 → 480** ·
  L0 **1831 → 1506** · 接手必读合计 **1129 → 954** · L2 回到 22661 以内 ✓。
- **判据**：`docs-lint ✓`（①–⑦；`--selftest` **11/11**）· `status-lint ✓` · `plan.py check ✓`（66 环节）·
  三份顶层文档**悬空引用扫描 = 0**（修 5 处指向已删/已归档文件的引用）。
- **预算**：**未抬任何上限** ✓，反而**收紧** `onboarding_max_lines` **1220 → 1000**（实测合计 954，
  留 ~5% 余量；`layer_max_lines` 未动 —— 下一轮还会合法变化）。commit：`84e26608`（三份合一）·
  `0054eff9`（索引 + 文档地图）· `9c63e252`（收尾记账）· 预算收紧（第 4 个）。

## 第 509 轮（2026-09-30）：**K1 线收尾 —— K1-a 两态终于有判据 + K1-b 的 `decl_idx` 钉子**
- **`by-prefix-reuse.md` 落后于工作区**：K1-a（T-K11）与 K1-b 的内核那一处（`with_env`，T-K12a）
  都早已落地、§3.C 后复用**默认就是开的** ⇒ 本轮正体是**补齐 §4 的验收**（内核 0 行改动）。
- **勘出一处假声明**：§4 写「两态对拍 `assert_same_both_ways` 已具备」——**错的** ✗：那条比的是
  `SOKO_NO_JUDGE_BATCH`，与 `SOKO_JUDGE_ENV_REUSE` **无关** ⇒ 这条验收**此前没有判据**。补
  `crates/cli/tests/judge_env_reuse.rs`（两态逐字节 + **不空转**）；反向验证（去掉 §3.C 的夹紧）⇒ **判红** ✓。
- ⚠ **新坑**：**模块根产物不受 `SOKONANODA_NO_CACHE` 管** —— 一热就跳过整份编译 ⇒ 两态「相同」
  是**空转的相同**（164 命中/23s vs **895 命中/283s**）⇒ 判据必须带 `…NO_PROJECT_ARTIFACTS=1`。
- **读数**：全语料两态 **128 文件 · 逐字节差异 0 · 命中 895 · 283s** · `unit12-solution` **7641ms → 6107ms（1.25×）**
  ⇒ 进 `docs/perf/ledger.jsonl`，并**更正** 2026-09-24 那条「K1-a 零收益」（只对「担保没接到主编译 pass」的那天成立）。
- **K1-b**：补 §4 要求却一直缺的 `decl_idx` 钉子（`memory_api.rs` 的
  `cross_builder_name_lookup_is_silently_positional_without_with_env`；反向验证 ✓）。
- **验收五步**（baseline = v0.77.0）：① workspace 全测 ✓ · ② 全语料逐字节 **零差异（9 组）** ✓ ·
  ③ 课程门禁 **43 目标 · 376 checked · 99 open · 0 判负** ✓ · ④ 性能台账 ✓ · ⑤ **明确跳过**（`LEAN_KERNEL_ARENA` 未设）。
- **文档**：`by-prefix-reuse.md` §6 as-built · `docs-budget.json` 153 → **183**。

## 第 508 轮（2026-09-30）：**§3.C「前缀环境」落地 —— 全课程 `build` 2.65×**
- **先勘后动**（开工单 §3.C 明写要求）：让勘的**两条路都够不着**那 791 趟
  （候选① per-前缀 builder 池 = §17 的 B″ 同死因：arena 生命周期；候选② judge 收调用方
  builder = 借用可行但**前缀位置不同**）；**我据此推断的第三处（提升 arena 作用域）
  也被逐趟明细推翻** ✗ ⇒ 真刀口是「**judge 每次都从源码重编整个前缀**」。
- **真数字**（`SOKO_JUDGE_STATS=2`）：`by` 路径 **265 趟 = 115.9s = `judge_ms` 的 99%**，
  而 `judge_infer` 的 71126 次只 38.5s（P1-b 已吃干净）；那 265 趟 **key 全不重复 ⇒ 加缓存没用** ✗。
- **刀口**：把既有的 `TRUSTED_PREFIX` **接到主编译 pass**（`walk.rs` 每检查完一条命令压栈担保）。
  ⚠ **关键一行 `before.min(prefix_commands)`** —— 两个坐标系不同（AST 序号**含 `import`**，
  judge 前缀文本走 `importless_source`）⇒ 不夹会**多担保合成声明** ⇒ 判定声明没被检查
  （`--json` **38 行不同**，与 P1-b 第一次失败同签名）✗。
- **读数**：墙钟 **126.4s → 47.8s（2.65×）** · `judge_ms` 113,659 → **19,223（−83%）** ·
  `pass_total_ms` 276,407 → **92,058** · `by_calls` 不变 ✓。
- **证据链**：影子档（**判据级**，两条路都跑）`shadow_same=265 · diff=0` · `--json`
  **0 行不同** · 反向判据（去掉夹紧 ⇒ 判红）· 带开关跑完整 `gate` PASS · `grade` 错误路径同诊断 ✓。
  ⚠ 影子档第一版比**整份报告** ⇒ 265/265 **假分叉**（差的是报告**范围**不是**判定**）✗ ⇒ 改比**判据**。
- **默认开**（收益成立才开）；逃生门 `SOKO_JUDGE_ENV_VOUCH=0` / `SOKO_JUDGE_ENV_REUSE=0`。
- 🚀 **`v0.78.3` 已发布并闭环** ✓：CI `36655613663` **28 success / 0 failure**（1 skipped）·
  tag `v0.78.3` → release workflow `36656518982` success · `gh release list` **Latest** ·
  26 个资产（8 CLI + 8 LSP + 9 VSIX + 1 源码）✓。
  ⚠ **CI 侧也看得到提速**：`gates-course` **12m26s → 6m38s**（同一 job、同一 runner 家族）✓。

## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ⬜ **批次 N 剩余 3 条**（`python3 scripts/plan.py` = **63/66**）：**T-N14**（B3 记法路径改走
  隐式插入；剩 **G-42** + 模式 B/D）→ **T-N13**（B2 课程库改隐式风格，**被 T-N14 挡**）→
  **T-N15**（C 收尾：台账 + 「看得见的变化」清单 + `REQUIREMENTS.md` §9 + VS Code/skills 同步）。
  权威 = `python3 scripts/plan.py next`（规格全文）。
- ⬜ **E19 甲案 = `v0.79.0`**（高风险，**单独发版**）· E20 乙案：给记法求解器加**元变量**；
  ⚠ 开工前**重新冻结基线**（`docs/design/notation-subset.md` §17 · 缺口的根因在
  `docs/design/v077-kernel-deficiencies.md` §三）。

## 硬事实（接手先读这 6 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 ✓ · `docs/architecture.md` §6/§8 ✓）
- **批次制**：一个批次**只 push 一次** ✓（`AGENTS.md` §CI 节奏 ✓）
- **等待流水线不用轮询** ✓：`gh run watch <id> --exit-status` ✓（`docs/CI-FAILURES.md` "轮询不是工作" ✓）
- **性能门禁只跟同宿主的记录比** ✓（`AGENTS.md` §性能回归门禁 ✓）
- **文档预算** ✓：`python3 scripts/docs-lint.py`（判据 ①–⑦，**常量在脚本里**，别抄旧数字 ✗ ·
  设计 `docs/design/docs-diet.md` ✓）；**接手成本**是判据 ⑦ 的**会判红的数字**（`docs-budget.json` 的
  `onboarding` 一节，上限**只许收紧** ✓）。
- **本文件受 lint 约束** ✓：`python3 scripts/status-lint.py` ✓（≤240 行 · 禁词 0 · 每段 ≤30 ·
  净增 ≤60 ✓）；**只保留最近 3 轮**，旧轮 ⇒ `docs/STATUS-ARCHIVE.md` 或 `docs/archive/`（后者不受冻结表管 ✓）。

---
