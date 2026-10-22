## v0.97.1 发版：`·` 的快捷输入 `\.`（2026-10-22）
- ⭐ **用户拿到什么**：在 `.sokonanoda` 里打 `\.`（Lean 的标点键；别名 `\centerdot`）⇒ `·`；`input.eager` 开时敲下 `.` 即出，关时 `Tab` 仍可用。`\cdot` 在 Lean 是 `⬝`，不映射；`·` 只进输入表，不喂词法（`notation_symbol:false`）。
- **判据（实跑）**：front lib **1026/0** · CLI extension **43/0**（含 `dot_bullet_is_reachable_via_leans_punctuation_key` + 表镜像）· LSP lib **253/0** · stub 宿主 **73/73** · docs-lint ✓ · `docs/visible-changes.md` 一行 · **真 VS Code 真按键** `dot-space` / `dot-tab` 均 PASS（DOM 与磁盘都 `·`）· `\cdot`/字母别名边界见 `docs/design/notation-input.md`。
- **改动**：`crates/front/src/notation_input.rs` 表 76 条 + `query/completion` 注释 · JS 镜像/改写器（`\.` 标点键）· 契约/桩/真 e2e 用例 · `docs/design`/README/`·.md`/teacher 技能 · CHANGELOG 0.97.1。
- **发布（实跑）**：main `0d4ee18c` → CI `39699597767` **真绿 10/10** → auto-tag `v0.97.1` → release `39701131443` success：**26 资产**（8 lsp + 8 cli + 9 vsix + SHA256SUMS）；e2e 3 条腿 55/55；Marketplace 9/9 已 publish；站点 `site.json` v0.97.1。

## v0.97.0 发版：unit01 收口（`·` 子弹 / apply 早报 / 括号 span / 失败 UI）（2026-10-21）
- ⭐ **用户拿到什么**：`·` 按 Lean `closeUsingOrAdmit` 聚焦子目标，块末未闭合直接 admit 成开放练习（有 `· sorry` 时洞落在 `sorry`，否则落在 `·`）；`apply` 的坏显式实参（direct 与 fallback）落在 `apply` 行；`apply Eq.subst (p := fun x => a ∈ x) h` 可反解隐式；失败声明不再显示「🎉 已无目标」；`)`/声明后空行仍归属声明。
- **判据（实跑）**：`front --lib` **1026/0** · `tactic_docs` 6/0 · `skill` 5/0 · `docs-lint` ✓ · webview 37/37 · extension-host 71/71 · 课程单元1 3 checked/5 open/0 判负 · 用户三份证明各 1 checked · `· sorry`/空 `·` 均 open + 一个 tactic 位洞 · direct 坏实参 `apply` 行早报。
- **发布（实跑）**：main `218fce88` → CI `39640601991` **真绿 10/10** → auto-tag `v0.97.0` → release `39643072987` success：**26 资产**（8 lsp + 8 cli + 9 vsix + SHA256SUMS）；e2e 3 条腿 55/55；Marketplace 9/9 已 publish；站点 `site.json` v0.97.0。
- **边界（如实）**：`·` 里 **tactic 报错**仍照常报错（只有“没证完”走 admit）；direct 的**部分应用普通 App**（`Eq.trans h`）仍靠内核终审（前端 `InferOnly` 限制；完全应用/`@`/命名实参已早报）。
- **改动**：`front` parser/by/apply_probe/query/state + `editor/vscode` Infoview/树 + tactic 表/`·.md`/TM grammar/skills + LSP hover/F12；`STATUS/REQUIREMENTS/CHANGELOG` 同步。
- ⚠ **推送留痕**：pre-push 的 `gates：缺口台账（--strict）` 在 G-83 长复现（~50min）上不收敛；本地完整 `scripts/soko gate` 已 PASS（非 strict 分片）⇒ 按逃生门 `git push --no-verify`（CI 跑非 strict 分片）✓。

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
