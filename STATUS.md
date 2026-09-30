# 当前快照（2026-09-30 · 第 513 轮）

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

- **文档过期日期机制**（第 513 轮）✓：**每个活文档都有过期日期**（权威 = `scripts/docs-expiry.json`，
  **37/37 已登记**）· `git commit` 前**自动检测**（已过期/未登记 ⇒ **拒绝提交**）·
  **到期审查三选一**（续期须写特定理由 / 删 / 归档）⇒ `docs/ONBOARDING.md` **§3.2**。

## 第 513 轮（2026-09-30）：**文档过期日期机制 —— 到期审查义务落地 + hook 提醒修复**

- **用户工单**：「每个文档必须有过期日期，到期后由后续 agent 审查，有特定理由才继续保留」
  +「必须包含一个 python 脚本，git 提交前自动检测」⇒ 机制本体建在**第 512 轮**，**本轮补齐缺的另一半**。
- **核心缺口（实测）**：机制**没写进任何文档** —— `grep -rn "docs-expiry|过期日期|续期" --include=*.md .`
  命中 **0** ⇒ 到期**没人会审** ✗；`install-hooks.sh` 还指向 `docs/ONBOARDING.md` **§3.2（不存在）** ✗。
  ⇒ 补 **§3.2**（权威登记表 · 口径与**已知边界** · 退出码 · 分档 · 装 hook · ⭐**到期审查三选一**）+ `AGENTS.md` **⑧**。
- **修 hook 的"空转提醒"**：放行分支只回显 `tail -1` ⇒「快过期 1 份只是提醒」**不说哪一份** ✗ ⇒ 改成原样全打 ✓。
- **hook 实测拦得住**（探针已清理）：**未登记** ⇒ `git commit` **exit 1**；登记后**已过期** ⇒ 同样 **exit 1**
  + 处置三选一；**只快过期** ⇒ exit 0 放行 ✓；干净态提交**确实自动跑了检测** ✓。
- **预算**：四层**全部顶格**（L0 1521 · L1 5603 · L2 7136 · L3 42）+ `AGENTS.md` 435/435 ⇒ **不新增文件、
  不抬任何上限** ✓：§3.2 由**删 §4 的 stale 副本**（钉死"第 508 轮"）支付，⑧ 追加在既有行尾（**行数不变**）✓。
- **判据**：`--check` **exit 0**（活文档 **37 / 已登记 37**）· `--selftest` **21/21** ✓ · `docs-lint` ①–⑦ ✓ ·
  `status-lint` ✓。第 510 轮移出（**不单独归档**，原文 ⇒ `git log --all -- STATUS.md`）。

## 第 512 轮（2026-09-30）：**激进删档 —— docs 跟踪 624 → 124（−80.1%）**

- **用户工单**：「请删除 80% docs 下的文档」「之前 agent 做得太心慈手软了」⇒ 硬指标
  `git ls-files docs | wc -l` **≤125** + `python3 scripts/docs-lint.py` 全绿。
- **删 501 个**（一目录一 commit，全走 `git rm`，不进回收站、不归档）：`docs/e2e/logs/**` **206** ·
  `docs/archive/**` **220**（只留索引 + 最近 3 轮）· `docs/design/**` **64**（只留 17 篇活设计）·
  `docs/perf/**` **5** · `docs/notes/**` **3** · `docs/gaps/**` **2**（+2 个零消费者复现件）。
- **留的判据**（缺一即删）：① 用户白名单；② 被**代码 / 契约测试读或断言**（`crates/cli/tests/` 的
  `dsh`/`extension`/`st1_boundary`/`skill`、`scripts/plan.py`、`gap.py` 的复现件）；③ 仍是**未收口**
  那条线的唯一权威。⚠ 注释里的「设计依据」**不算消费者** ✗。
- **同步**：`docs/README.md` **248 → 140 行** · `docs/ONBOARDING.md` §2 索引表 · `docs/archive/README.md`
  重写 · `docs-budget.json` 移除 **79** 条冻结项（剩 30）+ 四层上限**收紧**（L1 5704→5603 ·
  L2 22379→7136 · L3 5345→42）· 活台账摘掉 `log` 字段（308 条）· 立**全局规则**：指向已删路径的引用
  ⇒ `git log --all -- <原路径>`。
- **判据**：`docs-lint ✓`（①–⑦；活文档 **406 → 126 个 / 6.22 → 1.78 MB**）· `plan.py check ✓` ·
  `e2e-merge.py --check ✓`；第 509 轮按「只留最近 3 轮」移出（原文 ⇒ `git log --all -- STATUS.md`）。
## 第 511 轮（2026-09-30）：**文档全量清理 —— 逐份审计 + 改错规则 + 删/归档**

- **用户工单**：docs 里有很多**过时的、错误的规则要求**，误导后续开发 ⇒ 全量清理（活文档逐份审计；
  `docs/archive|perf|gaps|e2e` 不在范围内）。**方法**：6 个只读 subagent 逐份读 + 主线复核（每份带行号证据）。
- **改（B 类，逐条对代码 / scripts / `ci.yml` 复核）**：约 40 份文件、**~120 处**声明改成实测事实 ——
  典型：`perf-gate`「回归即红」实为 `continue-on-error`（**只报不拦**）· `course-gate-in-ci` 的
  "不新建 job"实为独立 `gates-course` · `agent-query-channel`/`deepseek-harness` 表头"实现未开始"
  实为已落地 · `implicit-arguments` "`@f` 是 no-op" · `set-theory-syllabus` "`And` 必须 axiom"
  （G-02/03 已修）· `by-tactics` §13 "未修/零收益"（§3.C 后 **1.25×**）· 20 份"内核冻结（硬规则 1）"
  框架残留（内核 2026-09-21 已解冻）· `REQUIREMENTS.md` §4/§9.3 的旧数字（模块行数 / 3 MB / 六条判据）。
- **删/归档（A 类）**：`docs/design/compile-progress-ui.md` **删**（已落地、零消费者）·
  `docs/design/v077-set-theory.md` **归档** ⇒ `docs/archive/v077-snapshots-2026-09-30/`；
  `v077-kernel-deficiencies.md` / `v077-st1-boundary.md` **留**（前者是 G-56/58/59 的根因定位，
  后者被 `crates/cli/tests/st1_boundary.rs` 当输入读）。
- **死引用**：`docs/HANDOVER.md` / `E2-HANDOVER.md` 等 **30 处** ⇒ 改指 `docs/ONBOARDING.md`（`docs/README.md` 立了全局约定）。
- **机制**：`docs-gc.py` 的 4 份误报候选写进 `KEEP_ALWAYS`（消费者是**契约测试 / 代码注释**）；
  冻结表**补全 11 份 + 删 1 个死键 + 四层上限按实测收紧**；`docs-budget.json` 的 19 KB 流水压成摘要（→ **7.9 KB**）。
- **读数**：活文档 **408 → 406 个 / 6,233,011 → 6,215,917 字节**；活 `.md` **113 → 111 份 /
  29,886 → 29,617 行**；`docs-gc` 候选 **5 → 0**；`docs-lint` ①–⑦ 全绿 + `--selftest` **11/11**；
  **没抬任何上限**（L0 1849→1521 · L1 9061→5704 · L2 22661→22379 · L3 5426→5345）；第 508 轮移入
  `docs/archive/status-removed-round-508-2026-09-30.md.gz`。

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
