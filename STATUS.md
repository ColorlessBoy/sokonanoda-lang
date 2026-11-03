## 推送例外（2026-11-03）
- `--no-verify` 推 70bb54f6（用户批准）：pre-push 只红在 `docs-lint ①` —— entry 多 1,274 B · 总量多 865 B，**HEAD 上既有**（干净 HEAD worktree 实跑同样两条，不是本轮引入）。补跑：等并发那轮把根文档压回预算后跑完整 `ci-local.sh --fast`；远端 CI 的同一 step 会照样红（既有红，非本轮）。
## strict `≤ loose` 不变量：D4 重评估（2026-11-03，未发版）

* **量具**：`scripts/strict-score.py --corpus`（全语料 357 文件逐文件三项 `strict ≤ loose` +
  allowlist ratchet，`--selftest` 四向咬住）+ `scripts/strict/exception-census.py`。读数 = **7 个文件**。
* **C 案（内核元变量赋值带 scope）**：C1–C3 已落 —— 判据 = 714 行逐字节 **0 差异** · 内核 131/131 ·
  四门绿 · 整门课 **258 / 2191 / 0 判负**。
* **A′ 落地（违反数 7 → 4）**：`operand_type_expr` 不再拿**裸隐式常量**（`∅` 这种零元记法）去问
  `judge_infer` —— 那问题**必然失败**且每次失败重编整份前缀。`unit19-solution` / `unit19-cardinal-le`
  的 calls/misses/prefix 因此与 loose **逐项相等**；714 行逐字节仍 0 差异、四门绿、课 258/2191/0。
* **C4（内核接受输入元变量）收益 = 0**，已还原：判卷能保住（靠回传）但站点全数转成
  `argmismatch_text`/`panic` —— 要真解需**声明级元变量上下文**（Lean `TermElabM`）。
* 读数/机制/逐文件结论 ⇒ `reports/2026-11-03-{strict-invariant-assessment,metavar-scope-assignments}.md`。

## strict P2-12 S0b + S4a（2026-11-01 · 分支 `strict/2026-10-29-kernel-app`，未发版）
- **S0b** 补三处回落计数 ⇒ `fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved` 闭合（前差 68/31）；**口径改写**：差额全在记法路由（`Set.Le` 的 `A ≼ B`，单文件 88 次）⇒ S0 的 59/18 是下界。
- **S4a** `congrArg.{1}` 两族改由内核解出（陈旧 `fty` 重算 + conv Pi 臂各喂各自域）；四文件 `ok_*` **10→0**，rc/诊断不变、`MISS` 不升。**负结果**：arg 类不是 Miller 失配——真因是内核 panic 被 `is_def_eq_at` 的 `catch_unwind` 吞成「不等」（G-91 反例，P2-13 候选）。
- 判据：kernel lib **123** · front kernel_app **48** · 语料开关关 **714 行 0 差异** · 新增 `crates/cli/tests/strict_fallback_census.rs`；两条内核修都反向验证过。

## 更早轮次（metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9。

## 更早轮次（原文已删，只留最近 3 轮 ⇒ `git log --all -- STATUS.md`）：v0.97.8（bump 写站点数据 · vendored 依赖 · 删逃生门 · queue-lint 拆 5 job）· v0.97.7（apply 归纳头 + Eq.subst 多现场 + 根因码，已发版 26 资产）· v0.97.6（`apply Eq.subst h hx` 判绿，已发版 26 资产）· v0.97.5（Eq.subst old-style 前缀 + 末尾 sorry 跳过 + failed 卡片 + e2e 起窗）· v0.97.4（`EntryCache::dirty_commands` 末尾 theorem 消失）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`…`v0.83.0` 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16 实测是**死格**：写死的 `v0.83.0` 落后 **15 个 release** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
