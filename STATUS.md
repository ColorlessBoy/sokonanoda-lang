## metavar 真实状态（2026-10-26 更新）
- **M1–M4 = 隐式实参求解器**；**#16 HO 线**：HO1/HO2 ✅（HO2 症状级）、**B2 P0–P4c ✅**（P0 现场 gate-on 绿；gate 默认开 + full gate PASS）、**HO3a/HO3b ✅**（pattern/FO/checker，默认翻 on）；**B2 ✅ 默认开**；**HO4 ✅（pattern/approx 默认开；quasi 捕获闸；gate PASS）**；状态只认 `metavar-pattern.md` §9。
- 更正：完整 mvar ≠ M1–M4；HO3/HO4 未收口。

## LKA 发版门禁 + 内核三修（2026-10-24 · 未发版）
- 用户要求：发版前跑最新 LKA、内核改动不报错、可信度进 README/官网。落地 `scripts/arena-gate.py` + `bump.py` 强制 + `release-preflight.sh` ④；内核三修：嵌套 iota 字段数、unsafe/partial `decline`、`MetaContext` O(1) 闸（`app-lam` 30s+ → 0.46s）。
- 200 条语料：**126 接受 · 71 拒绝 · 2 declined · 1 超时 · known slow 1**（`perf/magma-list-pair-n21`）；官网/README 写 **126/127 + 1 性能超时**。判据：kernel 76/76 · gate PASS · `check-site` 10/10。

## v0.97.8：bump 写站点数据 · 依赖 vendored · 删逃生门 · queue-lint 拆 5 job（2026-10-24）
- site.json 版本源改成**版本钉**、`bump.py` 同一步写它；release gate 追加 site 检查；`ci.yml` 的 `on.push` 只跑 main。**queue-lint 拆成 5 个并行 job**（`board`/`guard-bite`/`budgets`/`site`/`hygiene`）⇒ 失败一眼可读；三处 `needs` 与 `guard-bite.jsonl` 的 `ci_job` 同步。
- 依赖：`scripts/vendor/yaml`（PyYAML 6.0.3）+ `_yaml.py`；`SOKO_SKIP_HOOK` 删除；pre-push 缺口台账改成与 CI 同一条分片（60s/例）+ 先重建两侧二进制（否则量旧二进制：91 条假红 → 重建后全绿）。
- 判据：`check-site` 10/10 · `docs-lint` 绿 · `guard-bite --check` exit 0 · `ci-yml-lint` ✓ · 咬住证明与 `ci_job` 映射同步。
## v0.97.7 发版：apply 归纳头 + Eq.subst 多现场 + 根因码（2026-10-23）
- ⭐ 用户拿到：`apply Or.inl`/`And.intro` 在记法目标上判绿；`apply Eq.subst h`、`exact Eq.subst (Eq.symm hx) h` 判绿；`∉` 可用；tactic 失败报**根因码**；Infoview 失败声明照常画目标/假设。
- 内核/引擎：S1–S4 元变量线 + S5 延迟（默认关）。判据：语料 357/357 · 课程 2191/0 · front 1048 · kernel 74。已发版 `v0.97.7`（26 资产）。
## 更早轮次（原文已删，只留最近 3 轮 ⇒ `git log --all -- STATUS.md`）：v0.97.6（`apply Eq.subst h hx` 判绿，已发版 26 资产）· v0.97.5（Eq.subst old-style 前缀 + 末尾 sorry 跳过 + failed 卡片 + e2e 起窗）· v0.97.4（`EntryCache::dirty_commands` 末尾 theorem 消失）✓
## 未完成 `sorry` 练习按 Lean 4 进环境（2026-10-21 · 未发版）
- 签名通过后按同签名 `Declar::Axiom` 进环境（对齐 `sorryAx`），被引用处给 `uses-open-exercise` warning；签名不过仍 Failed、不进环境（G-01）。
## 课程冗余 `@` 清理：两批 176 处（2026-10-21 · 未发版）
- ⭐ **用户拿到什么**：第一批 90 + 第二批 86；共 **176 处**。
- **判据**：课程门禁 **258 / 2190 / 915 / failed 0**；`notation-lint` selftest **18/18**。
- **纪律**：delete-first；`@Eq.*`/多前导/零元/记法目标不进表；`@Set.mem_univ α` 局部应用保留 `@`。

## 更早轮次（含本轮删掉的 `v0.97.3 发版：CLI PATH 误报 + 扩展升级自动升 CLI` / `v0.96.0 发版` / `apply 目标先参与`；**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：**CI 转绿：0.95.0 的 6 个红 job**（2026-10-20）· `fix-flip3` **第八刀（严格档 `@` 残余第二批，`rejected 22 → 16`）** · `fix-flip3` **第七刀（清 2 条语料/判据拼法 rejected + 构建身份更正）** · 第六刀（`courses/set-theory/**` 严格档 `@` 残余，`rejected 29 → 24`）** · **第四刀（族 (A)：`@` 脊上裸常量实参按该层 domain 重新 elaborate）** · 引擎侧三刀 + 输入方言迁移（a18）· 发版 `0.94.0` · `land-a15` + 补回 e1 · `SUB-implicit-args-strict-default`（严格档翻默认）· 发版 `0.93.0` · 严格档第 1 批（a1→a5）· 发版 `0.92.0` · `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS` ✓ （2026-10-19 第六/七刀按本文件自己的规矩 + `docs-lint` ⑦ 腾空间 ⇒ 只留最近 3 轮 ✓）
## 旧轮墓碑（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`CI-GREEN-SELFTEST-NOT-WIRED` · `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` · `BITE-PROOF-DOES-NOT-BITE-IN-CI` · `SUB-implicit-named-args` · `SUB-implicit-args-align-lean4` · `SUB-release-metric-measures-unreleased` · `CI-GREEN-READOUT-ROTTED` · `SUB-break-the-guard-recursion` · `BITE-PROOF-READS-LIVE-SHARED-LEDGER` · `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES` · `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI` · `E2E-COVERAGE-4-OF-36` · `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（第三层）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`…`v0.83.0` 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16 实测是**死格**：写死的 `v0.83.0` 落后 **15 个 release** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
