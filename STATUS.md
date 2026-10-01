# 当前快照（2026-10-01 · 第 524 轮）

- **🚀 `v0.79.0` = Latest** ✓（CI `36803030466` **28 success / 0 failure**（1 skipped）· release
  workflow `36803986833` success · tag `v0.79.0` · 26 资产 = 8 CLI + 8 LSP + 9 VSIX + 1 SHA256SUMS）。
- **E19 甲案收口** ✓：三刀 + **默认开**（用户 2026-10-01 拍板：默认开 · 开关**保留为逃生门**
  `SOKO_NOTATION_METAVAR=0`/`off`）⇒ `docs/design/e19-baseline.md` **§9 as-built**（两态摘要 · 交叉验证 ·
  既有判据逐条重审）；台账 **G-48 ⇒ `fixed` / `fixed_in: 0.79.0`** ✓。
- **P 组（编译提速）全档收口** ✓：`JUDGE_PREFIX runs` **3759 → 791**（−79%）· 全课 `build` **216.14s → 47.8s**
  （**2.65×**）· `judge_ms` **−83%** · `--json` **0 行不同** · 影子档 `diff=0`；读数/开关/回退 ⇒ `ONBOARDING.md` §1。
- **三条用户实测 UI 缺陷 + Q1/Q2 + K1 线（按前缀复用）已收口** ✓（第 505/506/509 轮；逐条索引 ⇒ `docs/visible-changes.md`）。
- **计划与队列的唯一入口 = `docs/ONBOARDING.md` §0.2** ✓（**队列 #1 = IA-4 元参数引擎**：
  **设计已出**（`docs/design/metavar-engine.md`）· **实现未开始** ⇒ 开工前先收 **D1**（范围）✓）。
- **本文件的判据**：`scripts/status-lint.py`（≤240 行 · 禁词 0 · 每段 ≤30 · 净增 ≤60）+ 文档预算
  `scripts/docs-lint.py`（判据 ①–⑦，已进 `scripts/soko gate` 与 CI）。

- **批次 N 进度 66/66**（第 519 轮）✓：T-N13（第 518 轮）· T-N15（第 519 轮）⇒ 逐条索引 `docs/visible-changes.md`。
- **文档过期日期机制**（第 513 轮）✓：每个活文档都有过期日期（`scripts/docs-expiry.json`）· `git commit` 前自动检测 ⇒ 已过期/未登记**拒绝提交**。

## 第 529 轮（2026-10-01）：**IA-4 M3 收口 —— sort/kind 闸门 + 三通道归因**

- **引擎内的 sort/kind 闸门**（`MetaCtx::assign`，设计 §2.5）：三值语法近似（`Prop`=1 · `Type`=2 ·
  `Sort(n)`=n+1 · 其余 = **不知道 ⇒ 放行**）——**只拒"确定错"的** ⇒ 不可能假拒绝 ✓；检出 =
  **作废该候选** + 报**既有码**（用户拍板，不新增码）✓。
- **三通道** `MetaSolve { Solved, Unsolved, Kind, Clash }`：`first_err` 记首个硬错误、`discharge`
  优先归因；应用路径的错误点**同一个码**下按通道换 `message` 那一句（`Unsolved` 那条**逐字等于**
  今天）⇒ 码与 hint 契约不变 ✓。
- **判据**：真值层 **15/15** · front **786/786** · §2.6 十二条（`implicit_metavar`/`metavar_inventory`/
  `metavar_engine`/`notation`/`notation_metavar` = **1/1/3/49/1**）· **三指纹两态都逐字节等于冻结值**
  （本轮**零用户可见变化**）· 结构计数逐项等于基线 · `gate --fast` **EXIT=0** ✓。
- **契约同步**：`docs/protocol.md`（三通道说明；码/hint 不变）+ `skills/sokonanoda-teacher/SKILL.md`
  （按 message 判通道）；**扩展无需改**（VS Code 只透传 message/hint，没有码表）✓。
- **⚠ 一条诚实更正**：设计 §4 的「kind 夹具不再落内核」**做不到**（除非回退 G-21 的既有修复 ✗）——
  那些形状**严格档先跑且成功** ⇒ 引擎轮不到 ⇒ 照旧落内核 `kernel-expected-sort`（hint 已很好，
  G-21 正钉在它上面）⇒ M3 的闸门是**引擎自己的正确性守卫**，不是新的用户可见诊断；
  真要搬去 elab 期需同轮搬 hint 内容 + 改 G-21 台账 ⇒ **留作后续决策点** ✓。
- **记账**：`metavar-engine.md` **§10 as-built** · `ONBOARDING.md` §0.2（M3 ✓，下一片 **M4**）·
  第 526 轮归档 · 预算（cap 400→418、protocol 897→900）✓。

## 第 528 轮（2026-10-01）：**IA-4 M2 收口 —— 引擎接进一般路径**

- **接线** `implicit::solve_prefix_meta`（应用 / 裸常量 / 路线③共用同一条 `solve_prefix`）：严格档
  **永远先跑且不变**，失败后按档位分流（`Engine` ⇒ 引擎 / `Sibling` ⇒ E19 窄版）✓；
  **`fill_pending_by_shape` 降级**为 defaulting 的**参考实现**（`Sibling` 档那一份，引擎档的等价物 =
  `MetaCtx::default_unresolved`）✓。
- **判据**：front **782/782** · `implicit_metavar` **1 passed**（四条断言）· `metavar_inventory`
  **1 passed** · `metavar_engine` **3 passed**（M1 记法 5 形状 + M2 一般路径 5 形状，各 × 4 档）·
  `notation`/`notation_metavar` **49/1** · **三指纹：默认档与引擎档都逐字节等于冻结值**（0 增 0 失）·
  引擎档门禁 **43/377/99/0** · 冷 build 结构计数**逐项等于基线** · `gate --fast` **EXIT=0** ✓。
- **读数口径再修正**（M0 只说了全局缓存）：冷 build 计数还要求**项目缓存确认是空的**
  （`build --clean` 后 `ls courses/set-theory/.sokonanoda/compiled | wc -l` = 0）且**别与别的编译并发**
  —— 实测踩到 `compiled 30 / hit 12`、`runs=616`（**热启动，不是回归**）✗。
- **记账**：`metavar-engine.md` **§9 as-built** · `ONBOARDING.md` §0.2（M2 ✓，下一片 **M3**）·
  第 525 轮归档 · 预算 `_comment`（cap 360→400）✓。

## 第 527 轮（2026-10-01）：**IA-4 M1 收口 —— 引擎内核 + 记法路径接线（默认档 = 今天）**

- **新模块 `crates/front/src/compile/meta.rs`**（元变量 = `\0soko_m{id}` **名字编码**）：三值合一
  （`Undef` = 弃权）+ 快路径 + **两侧**分解 · **occurs/type-occurs/作用域**三闸门 · 有界待定不动点
  （计数不降就停）· 三上限 · defaulting（E19 规则显式化）· zonk 无残留 · **delta 兜底**（S13）✓。
- **接线 + 三态开关** `SOKO_METAVAR=0|sibling|engine`（**默认 sibling = 今天**；旧逃生门仍等价）；
  记法路径按档位分流（**一般路径 = M2**）✓。
- **判据**：真值 **11/11** · front **782/782** · M0 清单 **1 passed**（26 读数一条不变）· 新
  `metavar_engine` **2 passed** · notation 系 **49/1/1** · **三指纹：默认档与引擎档都逐字节等于
  M0 冻结值**（⇒ 引擎 0 增 0 失）· 门禁 **43/377/99/0** · 结构计数**逐项等于基线** · gate **EXIT=0** ✓。
- **抓到两个真 bug（都修 + 钉判据）**：① **作用域误拒**（`notation_telescope` 用签名原文名 ⇒ 与用户
  变量撞名）⇒ 引擎路径**先 freshen 望远镜名**；② **缓存键漏开关**（**既有**，`v0.79.0` 也复现 ⇒ 逃生门
  被静默忽略）⇒ 键加**档位字节**（默认档保持 0）；残留面 `SOKO_JUDGE_INPLACE=shadow` **记账未修** ✓。
- **记账**：`metavar-engine.md` §8 as-built · `ONBOARDING.md` §0.2（M1 ✓，下一片 M2）· 第 524 轮归档 ·
  预算 `_comment`（cap 320→360）✓。

## 未决项（**只有这两条**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

- ✅ **E19 甲案 = `v0.79.0`**（2026-10-01 **收口**）：**默认开**（逃生门 `SOKO_NOTATION_METAVAR=0` 保留）
  + 发版 ✓ ⇒ as-built `docs/design/e19-baseline.md` **§9**；台账 G-48 ⇒ `fixed` / `fixed_in: 0.79.0` ✓。
  **后继 = IA-4 元参数引擎**（设计 ✓ ⇒ `docs/design/metavar-engine.md`；开工前先收 D1）✓。
- ✅ **批次 N 全档收口（66/66）**：T-N13（第 518 轮）· T-N15（第 519 轮）✓ —— 逐条索引
  `docs/visible-changes.md`，遗留见 `docs/design/e2-plan.md` 的 T-N13 as-built「遗留」。
- ⚠ **E20 乙案不做**（用户拍板）；缺口根因 ⇒ `docs/design/v077-kernel-deficiencies.md` §三。

## 硬事实（接手先读这 6 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 · `docs/architecture.md` §6/§8）
- **批次制**：一个批次**只 push 一次** ✓；**等流水线不轮询** ✓（`gh run watch <id> --exit-status`）· **性能门禁只跟同宿主比** ✓（`AGENTS.md` §CI 节奏 / §性能回归门禁）
- **文档预算** ✓：`docs-lint` 判据 ①–⑧（**常量在脚本里**，别抄旧数字 ✗）——**接手成本**是判据 ⑦ 的**会判红的数字**（`docs-budget.json` 的 `onboarding`，上限**只许收紧** ✓）；**本文件**另受 `status-lint` 约束（≤240 行 · 净增 ≤60 · **只留最近 3 轮**，旧轮 ⇒ `docs/STATUS-ARCHIVE.md` ⇒ `.gz` ✓）

---
