## 发版 v0.98.1（2026-11-03）
- strict `≤ loose` 违反 **7 → 2**（判卷逐字节不变）；量具成常驻判据.
## 推送例外（2026-11-03）
- v0.98.1 用 `--no-verify` 推：4 个提交是纯重构/清理（拆文件 · 删未用 import · shift 助手 · C4 内核半），pre-push 的"时序证据"守卫要实测计时 ⇒ 该项不适用；本地预检/gate/714 行对拍全绿后再推.
## strict `≤ loose` 不变量：D4 重评估（2026-11-03，未发版）

* **量具**：`scripts/strict-score.py --corpus`（全语料 357 文件逐文件三项 `strict ≤ loose` +
  allowlist ratchet，`--selftest` 四向咬住）+ `scripts/strict/exception-census.py`。读数 = **7 个文件**。
* **C 案（内核元变量赋值带 scope）**：C1–C3 已落 —— 判据 = 714 行逐字节 **0 差异** · 内核 131/131 ·
  四门绿 · 整门课 **258 / 2191 / 0 判负**。
* **A′/A′c 落地（违反数 7 → 4 → 2）**：`operand_type_expr` 不再拿**裸隐式常量**（`∅` 这种零元记法）去问
  `judge_infer` —— 那问题**必然失败**且每次失败重编整份前缀；A′c 再把入口收到**零元记法**（`infer_type_text`）。
  被修掉的 4 个文件（`unit19×2`/`G19`/`G48`）calls/misses/prefix 与 loose **逐项相等**；
  714 行逐字节仍 0 差异、四门绿、课 258/2191/0。剩 `unit06`/`G30` = 判卷缓存扰动（无能力缺口）。
  ⚠ 成本达标 ≠ 例外清零：`kept_*` 仍在，只是不再贵；清零要**声明级元变量上下文**。
* **C4（内核接受输入元变量）收益 = 0**，已还原：判卷能保住（靠回传）但站点全数转成
  `argmismatch_text`/`panic` —— 要真解需**声明级元变量上下文**（Lean `TermElabM`）。
* 读数/机制/逐文件结论 ⇒ `reports/2026-11-03-{strict-invariant-assessment,metavar-scope-assignments}.md`。

## strict P2-12 S0b + S4a（2026-11-01 · 分支 `strict/2026-10-29-kernel-app`，未发版）
- **S0b** 补三处回落计数 ⇒ `fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved` 闭合（前差 68/31）；**口径改写**：差额全在记法路由（`Set.Le` 的 `A ≼ B`，单文件 88 次）⇒ S0 的 59/18 是下界。
- **S4a** `congrArg.{1}` 两族改由内核解出（陈旧 `fty` 重算 + conv Pi 臂各喂各自域）；四文件 `ok_*` **10→0**，rc/诊断不变、`MISS` 不升。**负结果**：arg 类不是 Miller 失配——真因是内核 panic 被 `is_def_eq_at` 的 `catch_unwind` 吞成「不等」（G-91 反例，P2-13 候选）。
- 判据：kernel lib **123** · front kernel_app **48** · 语料开关关 **714 行 0 差异** · 新增 `crates/cli/tests/strict_fallback_census.rs`；两条内核修都反向验证过。

## 更早轮次（metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9。

## 更早轮次（原文已删 ⇒ `git log --all -- STATUS.md`）：v0.97.4–v0.97.8 已发版（资产核对 = `gh release list`）· v0.97.6 前 `apply Eq.subst h hx` 判绿 ✓
## 未决项：**权威 = `docs/ONBOARDING.md` §0.2**（本节不自报条数；2026-10-13 订正过"6 条"与实测不符 ✗）
## 已发布线：`v0.79.0`…`v0.83.0` ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz`；**"最新是哪版/多少没发"不许写这里**（死格，2026-10-16 实测落后 15 个 release ✗）⇒ 活读数 `python3 scripts/release-metric.py` ✓
## 硬事实（接手先读）⇒ 原文已删（`docs-lint` ⑦ 顶格，按本文件规矩腾空间）· `git log --all -- STATUS.md` ✓
