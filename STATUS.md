## v0.98.0 发版（2026-11-03）
- **默认方言 = strict**（零环境变量；`--dialect loose` 是退路）+ 四条内核 soundness（归纳块宇宙参数守卫 · 块内临时名残留 · 单边 unit-eta 判等漏洞 · 空环境不看宇宙代换；LKA 最新语料 **207 例 0 failures**）+ 三条可见修复（内部名不再泄漏 / `Eq.subst` 前缀 / 出口规则顺序）。
- 判据：整门课 **258 · 2191 · 0 判负**（两档）· 357 文件 × 两档逐字节不变 · `release-preflight.sh` **✓ 全部通过**。
- ⚠ 本轮 `push --no-verify` 绕过 `check-timing-evidence.py`：K1–K4 与 `5192a0d` 的证据是**结构计数**（`unify` 6,815,039 → ±0），不是墙钟（2026-09-29 明令不许绝对毫秒）。按规矩记在此处；收口需补 `soko:no-timing:`（rebase 91 笔，未做）。

## strict P2-12 S0b + S4a（2026-11-01 · 分支 `strict/2026-10-29-kernel-app`，未发版）
- **S0b** 补三处回落计数 ⇒ `fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved` 闭合（前差 68/31）；**口径改写**：差额全在记法路由（`Set.Le` 的 `A ≼ B`，单文件 88 次）⇒ S0 的 59/18 是下界。
- **S4a** `congrArg.{1}` 两族改由内核解出（陈旧 `fty` 重算 + conv Pi 臂各喂各自域）；四文件 `ok_*` **10→0**，rc/诊断不变、`MISS` 不升。**负结果**：arg 类不是 Miller 失配——真因是内核 panic 被 `is_def_eq_at` 的 `catch_unwind` 吞成「不等」（G-91 反例，P2-13 候选）。
- 判据：kernel lib **123** · front kernel_app **48** · 语料开关关 **714 行 0 差异** · 新增 `crates/cli/tests/strict_fallback_census.rs`；两条内核修都反向验证过。

## v0.97.11 发版：`·` 块 goal state + `@` 折叠 + `·` 回车缩进（2026-10-27）
- 用户拿到：`·` 内层 tactic 的状态、普通 tactic 内部进入态/结束后后态、`cases` 后序块入口；`@Set.mem α a A` → `a ∈ A`；VS Code `·` 回车缩进。

## 更早轮次（metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9。

## 更早轮次（原文已删，只留最近 3 轮 ⇒ `git log --all -- STATUS.md`）：v0.97.8（bump 写站点数据 · vendored 依赖 · 删逃生门 · queue-lint 拆 5 job）· v0.97.7（apply 归纳头 + Eq.subst 多现场 + 根因码，已发版 26 资产）· v0.97.6（`apply Eq.subst h hx` 判绿，已发版 26 资产）· v0.97.5（Eq.subst old-style 前缀 + 末尾 sorry 跳过 + failed 卡片 + e2e 起窗）· v0.97.4（`EntryCache::dirty_commands` 末尾 theorem 消失）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`…`v0.83.0` 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16 实测是**死格**：写死的 `v0.83.0` 落后 **15 个 release** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
