## strict P2-12 S0b + S4a（2026-11-01 · 分支 `strict/2026-10-29-kernel-app`，未发版）
- **S0b** 补三处回落计数 ⇒ `fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved` 闭合（前差 68/31）；**口径改写**：差额全在记法路由（`Set.Le` 的 `A ≼ B`，单文件 88 次）⇒ S0 的 59/18 是下界。
- **S4a** `congrArg.{1}` 两族改由内核解出（陈旧 `fty` 重算 + conv Pi 臂各喂各自域）；四文件 `ok_*` **10→0**，rc/诊断不变、`MISS` 不升。**负结果**：arg 类不是 Miller 失配——真因是内核 panic 被 `is_def_eq_at` 的 `catch_unwind` 吞成「不等」（G-91 反例，P2-13 候选）。
- 判据：kernel lib **123** · front kernel_app **48** · 语料开关关 **714 行 0 差异** · 新增 `crates/cli/tests/strict_fallback_census.rs`；两条内核修都反向验证过。

## v0.97.11 发版：`·` 块 goal state + `@` 折叠 + `·` 回车缩进（2026-10-27）
- 用户拿到：`·` 内层 tactic 的状态、普通 tactic 内部进入态/结束后后态、`cases` 后序块入口；`@Set.mem α a A` → `a ∈ A`；VS Code `·` 回车缩进。

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
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`…`v0.83.0` 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16 实测是**死格**：写死的 `v0.83.0` 落后 **15 个 release** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
