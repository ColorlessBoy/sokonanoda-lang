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

## 第 531 轮（2026-10-01）：**记法输入表补全 —— 希腊字母 + 课程符号（用户反馈「`α` 打不出来」）**

- **调研先行**（上游 `vscode-lean4@master` 逐字取证 2026-10-01）：表是扁平 `{缩写: 替换}`（1865 键），
  落定 = 「已敲文本不再是任何键的前缀」或 Tab 强制，**最短的键赢**、无回溯；`\a` 是 α 的键之一。
- **补全**：表 **18 → 75** —— 希腊字母 **48**（大小写各 24，主缩写 = 拼写名，别名收 Lean 的纯字母键）
  + 课程库记法 **7**（`≈ ∘ ⁻¹ • ⊕ ⋃₀ ⋂₀`）+ 匿名构造子括号 **2**（`\<`/`\>` → `⟨`/`⟩`，唯一非字母键）。
- **`\a` 不引入歧义**：它是 `\alpha`/`\approx`/`\and` 的前缀 ⇒ 走既有的「完整表词」两态口径（还在敲
  字母时不落定，空格/标点/Tab 封口才落定，与 Lean 同规则）；指向逻辑符号的单字母（`\v` `\i` `\o` `\r`）**故意不收**。
- **新字段 `notation_symbol`**（Rust/JS 双镜像）：希腊字母与 `⟨⟩` 是**标识符/语法** ⇒ `merge_known` 不喂
  词法（否则 `α` 变 `Sym("α")`，变量 hover / `F12` / rename 守卫全坏）· 不着色；**hover 也补那一行**
  （`input_hint_at`，词法判据、与记法符号那条路不重复）。
- **判据**：`--test extension` 40 ✓（双镜像 + TM 类重生）· `--test notation` 51 ✓（分类由表驱动 + 48 个
  希腊字母当 binder + `⟨ha, hb⟩` 真判卷）· front 11 ✓ · LSP hover 30 ✓ · stub 55/55 ✓ · **e2e 39/39**（真 VS Code）· **课程门禁
  43/377/99/0** ✓ · 缺口台账 ✓ · docs-lint ✓。
- **记账**：新设计 `docs/design/notation-input.md`（调研/差距/决策 D1–D7/已知限制）· README/CHANGELOG/老师
  技能三处镜像同轮 ✓ · 第 528 轮归档 ✓。**已知限制**：TM 的 `variables` 类是 ASCII 的 ⇒ `α` 仍**不着色**。

## 第 532 轮（2026-10-01）：**编辑响应 —— 去掉整文件高亮 + 增量检查诊断（用户反馈「整个文件被高亮」）**

- **① 整文件高亮：确认仍在**（不是版本问题）—— `setCompileDecorations` 的范围是**整份文档** +
  `backgroundColor`；P7 只改了"什么时候出现"（300ms 展示延迟），没改"画多大" ⇒ 一次按键 2–3s
  ⇒ 每次都整篇亮。**已修（S1）**：只留概览尺**第一行**标记、**一处背景色都不留**
  （判据：stub 三条断言 —— 范围恰好 1 个 / `start.line == end.line == 0` / 无 `backgroundColor`）。
- **② 诊断（真 LSP + 真课程夹具，0.80.0）**：unit08 冷开 3093ms · 热开 11ms · **改一行 2154ms**；
  **改动位置几乎不影响成本**（第 1 条 3157 / 第 12 条 2002 / 最后一条 2350ms）⇒ **前缀复用没生效**。
  结构计数（`SOKO_STAGE_STATS=1`）：`passes` **113** · `JUDGE_PREFIX runs` **79** / 3.13MB ·
  unit 编译计数 **104**。拆开：库层 4 模块 ~500ms（**14%**）· **入口自己 ~3.0s（85%）**。
- **根因 = 两条台账缺口**：G-29（编辑 ⇒ 整闭包从零重编，缓存只对"打开"有效）+ G-31（判定每批重跑
  整份前缀）；代码证据 `check/mod.rs:608`「闭包编译不使用 TrustPlan（v1）」——项目文档连单文件那条
  增量路都没有。**优先级据此重排**：只修闭包复用最多省 14%，大头在入口自己的重查。
- **设计** `docs/design/declaration-incremental.md`（85 行）：S1 ✓ · **S2** 闭包入口接 TrustPlan（v2）·
  **S3** 库层会话跨按键复用（module-artifacts 刀 1 进程内版）· **S4** 判定前缀复用（G-31 本体）；
  每片判据都是**结构计数/比值**（`JUDGE_PREFIX runs`、unit 编译计数、passes），不用绝对毫秒。
- **记账**：G-29/G-31 的 `today` 追加本轮读数 + 设计指针（`status` 仍 `open`，与复现件一致 ✓）·
  第 529 轮归档 ✓。

## 第 530 轮（2026-10-01）：**编辑响应 —— P7 展示延迟（用户反馈「一闪一闪」）**

- **诊断先行**：**编译不是瓶颈**（`playground` 逐键往返 **0.1ms 中位**、24 键 24 次编译**每次 0ms**；
  VS Code 自动发的 semanticTokens/codeLens/inlayHint **≤3.1ms**；扩展发的 goals/project/stateAt
  **≤5.9ms**）；真慢的只有冷开（`unit12-solution` 首次 **7794–8407ms**，第二次 **33.7ms**）。
- **闪烁源 = 服务端每键都编一次** ⇒ `$/progress` **2.0 条/键**，每个 `begin` 给**整份文档**加背景装饰 +
  状态栏切「编译中…」+ Infoview 插进度块（`end` 再全撤）⇒ 真 VS Code **8 键亮 16 次**。
- **修法（P7）**：`begin` 只挂定时器、`end` 先到就撤 ⇒ 窗口内编完的**一次界面都不碰**；闸门放在
  `onCompileProgress`（LSP 路）而**不放 `applyProgress`**（build/rebuild 的逐文件进度必须从第一帧就报）。
- **前后（真 VS Code，同文件/操作/机）**：闪烁 **16 → 0** · 键→诊断 **122 → 99ms** · 键→面板
  **268 → 263ms**（~100ms 是 VS Code 诊断管线、150ms 是扩展的多文档去抖；服务端 **0.2ms/键**）。
- **明确不做**：不缩去抖 · 不改 `$/progress` 契约 · **不动 `crates/`** · 不删概览尺。**判据**：stub
  两条 `P7:`（计数）+ e2e `edit responsiveness`（用户看得见），修前**红**修后绿 ✓；细节见设计文档。
- **记账**：新文档 `docs/design/edit-latency.md`（这条链的唯一权威，含"明确不做"）+ 过期登记 ·
  `editor/vscode/{CHANGELOG,README}.md` + `package.json` 同轮 ✓ · 第 527 轮归档 ✓。

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
