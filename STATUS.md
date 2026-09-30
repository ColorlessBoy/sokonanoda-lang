# 当前快照（2026-09-30 · 第 516 轮）

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

- **文档过期日期机制**（第 513 轮，原文 ⇒ `git log --all -- STATUS.md`）✓：**每个活文档都有过期日期**（权威 = `scripts/docs-expiry.json`，
  **37/37 已登记**）· `git commit` 前**自动检测**（已过期/未登记 ⇒ **拒绝提交**）·
  **到期审查三选一**（续期须写特定理由 / 删 / 归档）⇒ `docs/ONBOARDING.md` **§3.2**。
- **批次 N 进度 64/66**（第 514/515 轮）✓：**T-N14**（记法路径走唯一钩子 + 实参期望类型）与
  **G-43**（lambda 实参按书写类型求解）收口 —— 两次全语料对拍各 **644 组逐字节相同**；
  剩 **T-N13**（B2 迁移本体）/ **T-N15**（C 收尾）。

## 第 516 轮（2026-09-30）：**G-69 收口 —— 归一化护栏补「逐位递归」（T-N13 的第二个前置）**

- **病根**：`by.rs::keep_if_lossless` 只比**顶层** spine 实参个数，而记法节点在 `spine_of` 里
  算 **1 个** ⇒ `Set.subset (A ∩ B) A` 与 pp 形态 `Set.subset (Set.inter A B) A` **顶层相等**、
  护栏放行 ✗ —— 可 pp 已把内层 `Set.inter` 的隐式 `α` 省掉 ⇒ `intro` 派生的假设成了**丢了参数的
  点形式** ⇒ `unfold_one` 把 `Set.inter A B x` 对成 `α := A, A := B, B := x` ✗。
- **修法**：护栏补**逐位递归**（新 `notation_positions_keep_implicit_prefix`）—— 源级记法节点在
  pp 形态里必须把该目标的**前导隐式实参**写出来（实参个数 ≥ 操作数 + `implicit_prefix`）；
  `DefInfo` 因此新增 `implicit_prefix`（两个构造点与 `KnownName::Decl` 同源 ✓）。
- **判据**：`compile/tests.rs::a_derived_hypothesis_solves_implicit_arguments_like_a_written_binder`（**反向验证** ✓：让该函数 `return true` ⇒ 当场判红）· 复现件 `G69-*.sh` **exit 1** ✓ · 对照组仍绿 ✓。
- **红线（逐项实测）**：全语料对拍 **644 组逐字节相同** ✓（今天全部记法目标 `implicit_prefix`
  都是 0 ⇒ 新判据**一次都不触发** ✓）· 课程门禁 **43/376/99/0** ✓ · front **796/0** · 记法契约 **50/0** ✓。
- **⇒ T-N13 的两个前置（G-43 / G-69）都已收口**；剩**迁移本体**（9 文件 / 19 诊断，逐站点改）。

## 第 515 轮（2026-09-30）：**G-43 收口 —— lambda 实参按「书写类型」求解（T-N13 的前置）**

- **病根（探针实测）**：**显示与判定共用一条 pp 文本** —— 内核 pp 会**丢掉第一个隐式实参**
  （`Set.image α β f A` 打成 `Set.image β f A`；后续隐式实参却留着：`unfold_apps_pp` 的
  `is_implicit_fun(fun)` **只认裸 `Const`**），而 `operand_type_expr` 把这份文本**回读成项**当
  "实参的类型" ⇒ 解出 `α := β` ✗ ⇒ `期望 Sort(1)，实际是 Pi ( : $4), $4`（**与 `import` 无关** ✓）。
- **修法（front 一处，内核零改动）**：新增 `lambda_source_type` —— lambda 的类型由**源级 binder
  注解**拼出来（`fun (h : P) => h` : `P → P`），与既有「`Ident` 取 `scope.source_type_of`」同一条
  **书写类型优先**规则 ⇒ 零内核调用、不被 pp 丢参污染 ✓；其余形状照旧问内核（不比从前差 ✓）。
- **判据**：`compile/tests.rs::a_lambda_argument_takes_its_written_type_not_the_lossy_pp_text`；**反向验证** ✓（中和成 `return None` ⇒ 当场判红）；复现件 `docs/gaps/repro/G43-*.sh` **exit 1（已修）** ✓。
- **红线（逐项实测）**：内核零改动 ✓ · 全语料对拍 **644 组逐字节相同** ✓ · 课程门禁 **43/376/99/0** 逐项相同 ✓ · front **795/0** · 记法契约 **50/0** ✓ · `gap.py check` ✓。
- **纪律教训**：本轮**两次**自造对照件被 bash 吃掉了 `''`（单引号里 `''` 会闭合引号）⇒ 量到的是 `f  A`（**另一个形状**）✗ ⇒ **对照件源码要 grep 出来核对** ✓。另：整文件批量改调用点**又一次证伪** ✗（649 处 ⇒ 16 文件判红）⇒ 只许逐站点核 ✓。

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
- **后续**：G-43 由第 515 轮收口（见下）⇒ T-N13 的**迁移本体**已无前置。**B2 取证（同轮）**：
  `lib/Set` + `lib/Image` + `Fun`/`Rel` 的 comp **签名隐式化各 0 诊断** ✓；整文件批量删类型实参
  **不可行** ✗（一次 649 处 ⇒ 16 文件判红）；剩 **9 文件 / 19 诊断**，根因 **G-69**（`unfold_one`
  对齐不认隐式前缀 ⇒ `intro` 派生的假设展开错位）⇒ **已整段还原**（课程门禁回到 43/376/99/0 ✓）。
- **纪律教训（又踩一次）**：`/tmp` 工作台的**编译缓存**会让"改动前后"量到同一份产物 —— 本轮第一次
  A/B 里基线竟把模式 B 判绿 ✗（`rm -rf <工作台>/.sokonanoda` 之后才复现 ✓）。第 511 轮移出（原文 ⇒ `git log --all -- STATUS.md`）。

## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ⬜ **批次 N 剩余 2 条**（`python3 scripts/plan.py` = **64/66**）：**T-N13**（B2 课程库改隐式风格；
  **两个前置 G-43 / G-69 都已收口** ✓ —— 剩**迁移本体**：9 文件 / 19 诊断，逐站点改，**不许批量改写** ✗）
  → **T-N15**（C 收尾：台账 + 「看得见的变化」清单 + `REQUIREMENTS.md` §9 + VS Code/skills 同步）。
  权威 = `scripts/plan.py next`。
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
