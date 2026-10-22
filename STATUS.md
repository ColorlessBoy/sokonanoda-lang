## v0.97.3 发版：CLI PATH 误报 + 扩展升级自动升 CLI（2026-10-22）
- ⭐ **用户拿到什么**：① `.zprofile` 已有那行（手工写/扩展写过）⇒ 不再误报「不在 PATH」；② 扩展升级后，已安装的 `~/.local/share/sokonanoda/bin/sokonanoda` **自动换成新 VSIX 自带版本**，不用每次手动 Install Command Line（没装过仍不代装；降级插件不降 CLI）。
- **判据（实跑）**：桩 **76/76**（PATH 手工行/marker 行/注释行 + 版本只升不降）；`extension.js` 纯函数 `cliPathLinePresent` / `cliVersionNeedsUpgrade` 由 `__cli` 测试缝驱动，临时 home 不碰真家目录。
- **性能口径（回应「3ms」）**：`git log` 的 `da1dd28f`/`a583175a` 量的是 **`soko/stateAt` / `soko/goalAt` 请求成本**（1.0–1.5ms / 0.7ms），不是按键→屏幕；真按键实测 `stateAt` 中位 **2ms**、目标可见约 **258ms**（200ms 选区 + 150ms 诊断去抖）。98/252ms 是 e2e 的**诊断/面板**口径，不是这轮引入的回归。
- **发布**：待 push → CI 真绿 → auto-tag `v0.97.3` → release/Marketplace/站点核对。

## v0.97.2 发版：Infoview 假「光标不在任何声明内」（2026-10-22）
- ⭐ **用户现场**：unit01 最后一题打 `constructor` / `intro h`，Infoview 说「光标不在任何声明内」，Restart Server 才有目标。
- **真因**：`soko/stateAt` 答的是**上一次编译的文本**；新文本还没编完时空答案（有时版本号还相同）被面板当成结论。扩展现在：编译在飞 / 编辑后 3s / 缓冲区 dirty 时，**空答案有界重问**（250ms × 120）；只有这些窗口都过去仍空才显示「光标不在任何声明内」。
- **判据（实跑）**：真 VS Code 真按键（`/tmp` 探针，真实 unit01 文件）——修复前 seq 9–11 空答案被接受，修复后打字期间只接受 `decl=singleton_eq_singleton_iff`，最终 `goal=a = b`；桩 **75/75**（含 stale-version 与 same-version-empty 两条）；真机 53.5s 跑完。
- **发布**：待 push → auto-tag `v0.97.2` → release/Marketplace/站点核对。

## v0.97.1 发版：`·` 的快捷输入 `\.`（2026-10-22）
- ⭐ **用户拿到什么**：在 `.sokonanoda` 里打 `\.`（Lean 的标点键；别名 `\centerdot`）⇒ `·`；`input.eager` 开时敲下 `.` 即出，关时 `Tab` 仍可用。`\cdot` 在 Lean 是 `⬝`，不映射；`·` 只进输入表，不喂词法（`notation_symbol:false`）。
- **判据（实跑）**：front lib **1026/0** · CLI extension **43/0**（含 `dot_bullet_is_reachable_via_leans_punctuation_key` + 表镜像）· LSP lib **253/0** · stub 宿主 **73/73** · docs-lint ✓ · `docs/visible-changes.md` 一行 · **真 VS Code 真按键** `dot-space` / `dot-tab` 均 PASS（DOM 与磁盘都 `·`）· `\cdot`/字母别名边界见 `docs/design/notation-input.md`。
- **改动**：`crates/front/src/notation_input.rs` 表 76 条 + `query/completion` 注释 · JS 镜像/改写器（`\.` 标点键）· 契约/桩/真 e2e 用例 · `docs/design`/README/`·.md`/teacher 技能 · CHANGELOG 0.97.1。
- **发布（实跑）**：main `0d4ee18c` → CI `39699597767` **真绿 10/10** → auto-tag `v0.97.1` → release `39701131443` success：**26 资产**（8 lsp + 8 cli + 9 vsix + SHA256SUMS）；e2e 3 条腿 55/55；Marketplace 9/9 已 publish；站点 `site.json` v0.97.1。

## 未完成 `sorry` 练习按 Lean 4 进环境：goal state 不再被 `unknown identifier` 阻断（2026-10-21 · 未发版）
- ⭐ **用户拿到什么**：`unit01` 的 `singleton_subset_iff` 引用尚未证明的 `mem_of_subset_singleton` / `subset_of_mem_singleton` ⇒ 不再报错、正常 elaborate；被引用处给 `uses-open-exercise` warning（不是 error）；部分证明留着 `sorry` 时 `query state` 照常给出目标。
- **改法**：开放练习的签名通过后，按**同签名 `Declar::Axiom`** 进环境（对齐 Lean 4 的 `sorryAx`），登记 `known` / `entered` / `open_exercises`；检查/事件语义不变（该声明仍是 `exercise.open`）。
- **判据（实跑）**：probe = 2 `exercise.open` + 1 `decl.checked` + 1 `uses-open-exercise` + exit 0；`unit01` canvas 5 open / 3 checked、solution 6 checked 均 exit 0；`query state` 目标 `a ∈ A → {a} ⊆ A`。
- **边界**：签名不过的声明仍 Failed、不进环境（G-01 红线）；引用 Failed 声明仍是 `elab-unknown-identifier`。
## 课程冗余 `@` 清理：两批 176 处（2026-10-21 · 未发版）
- ⭐ **用户拿到什么**：第一批 90 + 第二批 86；共 **176 处**。
- **判据**：课程门禁 **258 / 2190 / 915 / failed 0**；`notation-lint` selftest **18/18**。
- **纪律**：delete-first；`@Eq.*`/多前导/零元/记法目标不进表；`@Set.mem_univ α` 局部应用保留 `@`。

## 更早轮次（含本轮删掉的 `v0.96.0 发版` / `apply 目标先参与`；**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：**CI 转绿：0.95.0 的 6 个红 job**（2026-10-20）· `fix-flip3` **第八刀（严格档 `@` 残余第二批，`rejected 22 → 16`）** · `fix-flip3` **第七刀（清 2 条语料/判据拼法 rejected + 构建身份更正）** · 第六刀（`courses/set-theory/**` 严格档 `@` 残余，`rejected 29 → 24`）** · **第四刀（族 (A)：`@` 脊上裸常量实参按该层 domain 重新 elaborate）** · 引擎侧三刀 + 输入方言迁移（a18）· 发版 `0.94.0` · `land-a15` + 补回 e1 · `SUB-implicit-args-strict-default`（严格档翻默认）· 发版 `0.93.0` · 严格档第 1 批（a1→a5）· 发版 `0.92.0` · `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS` ✓ （2026-10-19 第六/七刀按本文件自己的规矩 + `docs-lint` ⑦ 腾空间 ⇒ 只留最近 3 轮 ✓）
## 旧轮墓碑（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`CI-GREEN-SELFTEST-NOT-WIRED` · `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` · `BITE-PROOF-DOES-NOT-BITE-IN-CI` · `SUB-implicit-named-args` · `SUB-implicit-args-align-lean4` · `SUB-release-metric-measures-unreleased` · `CI-GREEN-READOUT-ROTTED` · `SUB-break-the-guard-recursion` · `BITE-PROOF-READS-LIVE-SHARED-LEDGER` · `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES` · `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI` · `E2E-COVERAGE-4-OF-36` · `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（第三层）✓
## 旧轮（2026-10-15 · **原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：用户五条**激励纠正**（23:xx 亲提 →「你做吧」）· `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（判据的输入面 = 生成式夹具）· `soko update` 的 404 + `scripts/soko prune` · `#check <def>` 带定义体 + 记法（§9.22）· 清台账第 **7/8** 轮（队列 0 · 隐变量 13/13 收口 · 构建链与缓存身份两族修完）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`（E19 甲案 + P 组 · 批次 N 66/66）· `v0.80.0` · `v0.81.0` · `v0.82.0` · `v0.83.0` —— 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16：本行原写「`v0.83.0`（最新 ✓ · 2026-10-08）」而当时真实最新是 `v0.91.0` —— 落后 **15 个 release**（`v0.84.0`…`v0.91.0`，`gh release list` 实测）的**死格** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓ —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
