# 当前快照（2026-09-30 · 第 514 轮）

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
- **批次 N 进度 64/66**（第 514 轮）✓：**T-N14**（记法路径走唯一钩子 + 实参期望类型）收口 ——
  全语料对拍 **644 组逐字节相同**；剩 **T-N13**（B2，前置 = G-43）/ **T-N15**（C 收尾）。

## 第 514 轮（2026-09-30）：**T-N14 收口 —— 记法路径改走唯一钩子（隐式档）+ 实参期望类型**

- **工单**：批次 N 的 **T-N14 / B3**（B2 的前置）：记法路径改走隐式插入、收窄补参 hack。
- **两处改动**（都在 `crates/front/src/compile/elab.rs`）：① **隐式档走唯一钩子**（新
  `elab_notation_implicit`）：目标 `implicit_prefix > 0` 时把 `symbol(op₁…opₙ)` 还原成**源级应用
  脊**交给 `elab_expr` ⇒ 前缀由 `try_implicit_application` 解出，记法路径不再有第二套补参机械
  （**模式 B 收口**：`Set.univ ∩ A` 曾把 `α` 解成 `Type 0` ⇒ `Sort(2)` 撞 `Sort(1)`）；② **唯一
  钩子的实参期望类型**：原来**先** elaborate 第一个显式实参、再解前缀 ⇒ 它拿不到期望类型；改成
  **先解前缀、再逐位给「代入后」的期望类型**（**模式 D 收口**：`Set.inter Set.univ A` 一族）。
- **补参 hack 收窄（实测数字）**：`notation_prefix_args` / `solve_prefix_args` 各 **1 个**调用点，
  都在 `implicit_prefix == 0` 分支 —— 那是**护城河**：内建记法（探针实测 `Eq`/`And`/`Or`/`Iff`/
  `Not`/`Exists` 全是 `k=0`）与显式签名目标必须靠"操作数反解前导**显式**参数"（`Eq α a b`），
  应用路径**不许**补显式前导参数 ⇒ **隐式目标上执行次数 = 0** ✓（k==0 那档保留）。
- **红线（逐项实测）**：`scripts/kernel-diff.sh` 全语料 **644 组逐字节相同** ✓ · 课程门禁 **43 目标 / 376 checked / 99 open / 0 判负** ✓ · front **794/0** · 记法契约 **50/0** ✓。
- **新判据带反向验证**（`crates/front/src/compile/tests.rs`）：`…takes_its_prefix_from_the_shared_hook`
  + `the_first_explicit_argument_also_takes_its_expected_type` —— 撤掉任一改动 ⇒ 两条同时判红 ✓。
- **遗留**：**G-43**（构造子 + lambda 实参，binder 类型含导入模块的 def）仍红 ⇒ 归 **T-N13/B2**（复现件 `docs/gaps/repro/G43-*.sh` 仍 exit 0 ✓）。
- **纪律教训（又踩一次）**：`/tmp` 工作台的**编译缓存**会让"改动前后"量到同一份产物 —— 本轮第一次
  A/B 里基线竟把模式 B 判绿 ✗（`rm -rf <工作台>/.sokonanoda` 之后才复现 ✓）。第 511 轮移出（原文 ⇒ `git log --all -- STATUS.md`）。

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
## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ⬜ **批次 N 剩余 2 条**（`python3 scripts/plan.py` = **64/66**）：**T-N13**（B2 课程库改隐式风格；
  ⛔ 前置 = **G-43**：构造子（`Iff.intro`/`And.intro`）的 lambda 实参、其 binder 类型含**导入模块的
  def** ⇒ 复现件 `docs/gaps/repro/G43-*.sh` 仍 exit 0）→ **T-N15**（C 收尾：台账 + 「看得见的变化」
  清单 + `REQUIREMENTS.md` §9 + VS Code/skills 同步）。权威 = `python3 scripts/plan.py next`（规格全文）。
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
