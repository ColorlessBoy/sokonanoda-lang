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

## 第 537 轮（2026-10-01）：**集合论教材线开工 —— S-A 大纲重建 + I.5 章首个单元（含两条新缺口）**

- **S-A 收口** ✓：重建 `docs/design/set-theory-syllabus.md`（旧稿已随 64 份文件清理删除，而**五处活文档仍引用它**）—— 教材对照**逐条查过目录页**（Enderton 9 章 / Halmos 25 节）。诊断：**缺口不是缺库、是缺课**（ST8/10/12/13/14 的五个 lib 模块一个单元都没吃到）；卷 I 完整版 = **12 已落地 + 10 新单元**，而 I.10（秩/超限递归/Aleph）**是内核墙**（G-56/G-58）。
- **S-B 首片** ✓：`lib/Order`（42 条）+ 单元⑬（3 演示 + 11 练习）· 解答 0 open · 门禁 **46 目标 · 436 checked · 110 open · 0 判负** · `notation-lint` ✓ · `docs-lint` ✓。
- **两条新缺口（实测 + 最小复现）**：**G-72**（同一模块单文件绿、`import` 时红 —— λ 体里第二个绑定变量丢了；**假绿**，堵所有"返回函数值"的库定义）· **G-73**（定义头类型上三条 elaborator 边界）。绕法**全部收进库**，hint 只讲数学 ✓；**I.5 的两个必破题被 G-73 挡住，如实不排**（不改题绕开 —— 那会把语言缺口伪装成教学选择）。
- **台账**：`gap.py check` 只剩 **G-34 一条既存红**（HEAD 上就红、与本轮无关）；顺带把 G-07/G-44 复现件从"写死 4 章 12 单元"改成**与规模无关的形状判据**（合法加一章不该判红）。

## 第 535 轮（2026-10-01）：**S2 步 3 落地 + 我复测出的判红与修复（用户可见：改最后一条 theorem 2989ms → 314ms）**

- **落地**（并行会话 `81878509`）：`QueryDoc` 持有入口逐命令快照 + `EntryTrust` 前缀信任 ⇒ 「改一个 theorem 不再重查前面全部」。
- **复测判红**（我的验证环节）：那版 `cargo test` 红两条 ——
  `tests::project::{editing,an_external_change}_to_a_dependency_refreshes_the_open_entry`
  超时；`SOKO_LSP_TRACE` 取证 = 改依赖后刷新编译 **`publish=0`**（入口诊断与上一版逐字相同
  ⇒ 一条都不发 ⇒ 编辑器留**陈旧诊断**）。**根因**：`EntryCache` 没有依赖指纹，而
  `EntryTrust` 那段命令的**环境**来自被 `import` 的模块 ⇒ 依赖一改结论就不成立（静默错编）。
- **修复**（`a7177819`）：`dependency_fingerprint`（**非入口**模块名字+源文本 FNV）进
  `EntryCache.deps`，信任判定加门槛；不等 ⇒ `before = 0` 退回全查（宁可多查不可错编）。
  ⚠ 不用 `ProjectPlan::digest`（含入口自身 ⇒ 每键都变 ⇒ 信任永不成立 ✗）。
- **判据（全绿）**：LSP **170/0** · front **793+2**（1 ignored）· CLI 项目套件 **23+4** ·
  课程门禁 **43/377/99/0** · `fmt --check` ✓。**性能没被换掉**（真 LSP · unit08 · 三刀真改动）：
  改**最后一条** ⇒ **314/311ms · `modules=0` · `by=22` · `infer` 未命中 0 · `prefix=0`**
  （落地前 2989ms/104/1195/79/79）；改**第一条** ⇒ 2156ms · `modules=45` · `by=801`。
- **探针修正**：第二刀原来是"改回去"⇒ 命中磁盘缓存（11ms）⇒ 假快 ✗；改成 `a→b→c` 三刀真改动。
- **仍未闭合**：① 开档后**第一刀**恒整闭包（~2980ms，开档走产物缓存 ⇒ `entry_cache` 没填）；
  ② 改**中间/靠前**仍是后缀全查 ⇒ 等 **S6 脏集**（材料 `crates/front/src/depgraph.rs` 已落）；
  ③ 验收读数 `entry_kernel_checks` 在项目路仍恒 0（`pass.checks` 传播缺口）。

## 第 536 轮（2026-10-01）：**S6 落地 —— 依赖图脏传播**（用户 §5.1 的模型：改一条只重查它 + 下游）

- **机制**：`TrustPlan` 增 `trusted_extra: Vec<bool>`（**不连续**信任集 —— `before` 只能表达连续前缀，
  脏集表达不了）；`walk` 信任判定 = `idx < before || trusted_extra[idx]`；脏集 = **改动集**（文本或起点变）
  ∪ **传递依赖者**（`depgraph`，只做直接依赖会漏 ⇒ 静默错编）∪ **上一轮判负的**（翻转建不出边）；
  报告/失败表都按**信任位**拼回。环境仍整份 elaborate ⇒ 判定输入不变 ✓。
- **验收目标用例翻绿** ✓（`--ignored` 那条）：无人依赖 ⇒ 重查 **1**（原 4）· 中间 ⇒ **1**（原 7）·
  被 3 条（含 1 条**间接**）依赖 ⇒ **1+3 = 4**（原 8）；脏集实测 `{8}` / `{1,9,10,11}` = 恰好那几条 ✓。
- **零回归**：front **794** · LSP **170** · CLI **32 套** · 课程门禁 **43/377/99/0** · `fmt --check` ✓。
- **真 LSP（unit08）**：改**最后一条**仍 **314ms · modules=0 · by=22 · prefix=0**；改**第一条**
  2105 → **1665ms**（探针那次改**长度会变**的名字 ⇒ 报的是**上界**：后面每条命令起点平移都得重查）。
- ⚠ **两个坑**（写进注释）：① `depgraph` 的 `unknown` **不许**含库层/prelude 名字（`resolution == None`
  对 prelude 是常态 ⇒ 脏集 = 全部 ⇒ S6 等于没做 ✗），库层那一半由 `EntryCache::deps` 指纹兜底；
  ② 判据必须用**长度不变**的按键（改字节数 ⇒ 下游 `starts` 平移 ⇒ 它们必须进脏集 ⇒ 量到的是"后缀+平移" ✗）。

## 第 533 轮（2026-10-01）：**语义 token 读错文本（用户反馈「颜色全乱」）** —— 压缩存档

- **结论**：`semantic_tokens_full` 读了 `docs.text()`（**上一次编译用**的文本）而客户端画在**当前**缓冲区上 ⇒ 编辑点之后每个 token 错位；修 = 改用 `Docs::latest_text()`，判据 `semantic_tokens_follow_the_buffer_not_the_last_compile`（反向验证过）。兄弟缺陷（`codeLens`/`inlayHint`）记账不修。**零回归**：LSP 170/0 · 门禁 43/377/99/0。**原文 ⇒ `git log --all -- STATUS.md`**（第 533 轮那一段）。

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
