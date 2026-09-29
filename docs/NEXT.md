# 开工单（P1 编译提速 · 交接合一页）

> 快照：**2026-09-29 21:3x**（会话 `goal-b9f6e160` 的 12 轮跑完时）。
> 目的：把**散在各处的计划**收成一处 —— 新会话只读本文 + 下面 §4 点名的 3 份文档即可开工。
> **权威顺序**：用户口头要求 > `REQUIREMENTS.md` §9 > 本文件 > 其它。
> 本文**不取代** `docs/PLAN-0.74-0.79.md`（值守记账）与 `STATUS.md`（轮次流水），只是它们的**开工索引**。

---

## 0. 一页速览

**现在在哪**：P1（编译提速 · 内核层级）的**前两档已落地并发布**；剩 `by` 路径一档 + 三条用户实测的 UI 缺陷。

| 线 | 状态 | 一句话 |
|---|---|---|
| **P1-a** 就地判定（`infer_type_text`） | ✅ 已发（v0.78.1，**默认 on**） | 课程编译 **215.0s → 158.9s** |
| **P1-b 第一档**（`guarded_binder_type`/`solve_prefix_args`） | ✅ 已落地（`SOKO_JUDGE_INPLACE_WIDE`，**默认关**） | 再降到 **134.8s（1.60×）**；六条判据齐备，**待值守定"默认开 + v0.78.2"** |
| **P1-b 剩余**：`by` 路径 536 趟 | ⬜ **下一刀**（勘明在附九） | 收益上界 ≈16s（1.13×） |
| **P1-c / P1-d**：前缀环境可保存/恢复（`with_env_scope` 那条腿） | ⬜ 未开 | 这才是"大头"：`runs` 要从 1086 再大幅降 |
| **用户实测三条 UI 缺陷** | 🔴 已登记未修（`STATUS.md` 未决项） | ③ 版本标签 → ① tick 噪音 → ② Rebuild 进度 |
| **Q1 / Q2**（插件 build 超时配置 / 插件自带安装更新 CLI） | ⬜ 排队（**P1 收口后**同批做） | PLAN §2 Q 组 |
| **E19 甲案 = v0.79.0**（高风险，单独发版）· E20 乙案 = 降级路径 | ⬜ 未做 | PLAN §0/§1 |

**下一步（我建议的顺序）**：**③ → ① → ②**（最便宜且直接回答"联动"的疑问的在前面），然后回 **`by` 路径**（P1-b 剩余），最后 **P1-c**。

---

## 1. 已交付与读数（同机同口径：release · 冷缓存 · `SOKONANODA_BUILD_JOBS=1` · 全课程 `build --json courses/set-theory`）

| 读数 | `off`（基线） | P1-a | **P1-b 第一档** |
|---|---|---|---|
| `JUDGE_PREFIX runs` | 3759 | 1881 | **1086** |
| `JUDGE_PREFIX bytes` | 174,213,583 | 93,858,420 | **54,570,202** |
| `passes` / `doc_passes` | 4126 / 266 | 2248 / 266 | **1452 / 265** ⚠ |
| `judge_ms` | 146,580 | 120,359 | **111,772** |
| **墙钟** | **216.14 s** | **158.90 s** | **134.77 s（1.60×）** |
| `--json`（剔 `build.tick`） | 基线 | 逐字节相同 ✓ | **逐字节相同** ✓（2691 行 / 0 行不同） |
| shadow | — | diff=0 | **diff=0** |
| 反向判据 | — | ✅ | ✅（P1-a 两点 + wide 两点） |

⚠ `doc_passes` 266 → 265 是**计数口径**差 1（输出逐字节相同 ⇒ 不影响判定），**如实记账**。
**发版**：`v0.78.1` = Latest（2026-09-29T14:17:55Z，26 资产 = 8 CLI + 8 LSP + 9 VSIX + `SHA256SUMS`）。
**开关**：`SOKO_JUDGE_INPLACE=off|shadow|on`（默认 `on`）· `SOKO_JUDGE_INPLACE_WIDE=1`（默认关）。

---

## 2. 计划散落在哪（**先看这张表，别重复读全文**）

| 文档 | 管什么 | 什么时候读 |
|---|---|---|
| **本文件** | 开工索引 + 下一步 + prompt | **每次开工先读** |
| `docs/design/p1a-measurements.md` | **动手细节**：附一~六（量具/对账/读数）· **附七**（P1-a 判据 + 三个实测坑）· **附八**（1881 趟逐条归因 + 第一档读数）· **附九**（`by` 路径逐级函数表 + 阶段顺序 + wide ⑤ 夹具 recipe） | 动 P1 任一档前 |
| `STATUS.md`（未决项 + 最近 3 轮） | 轮次流水 · **用户实测的三条 UI 缺陷**（含行号/复现/修法选项） | 每次开工 + 收尾记账 |
| `docs/PLAN-0.74-0.79.md` §0/§2 | 值守总账（发版点、环节表、P/Q 组） | 需要"这一版包含什么"时 |
| `docs/design/incremental-environment.md` §18–§19 | P1-c/d 的设计来源（`with_env_scope` / K-2 / 候选 A 存活、B/D 判死） | 开 P1-c 前 |
| `docs/HANDOVER-slice1.md` | 切片 1（已挂起）的接口清单与零件表 | 只在碰切片 1 时 |
| `crates/front/tests/judge_inplace{,_on,_wide}.rs` | 三个档位的**判据模式**（照抄即可） | 加新接线点时 |

---

## 3. 下一步：三条开放线（每条都给了入口与判据）

### A. 用户实测的三条 UI 缺陷（`STATUS.md` 未决项；**建议先做**）

| # | 症状 | 入口（已定位） | 判据（必须绑"屏幕上看什么"） | 成本 |
|---|---|---|---|---|
| ③ | 同一 Infoview 里 `server 0.78.1` vs 项目区块 `… · 0.78.0` | 「项目」区块（E30）取数处；先判是**项目清单 `requires`** 还是**产物版本标记**（`<模块根>/.sokonanoda/compiled/`） | 两者同源或**各自带标签**（"项目要求" / "服务器"）；一条命令就能判死 | 小 |
| ① | 终端每秒刷 `{"type":"build.tick","file":""}` | `crates/cli/src/build.rs:15`（`TICK_MS=1000`）+ `:44-70` `Heartbeat` | 人看的终端**不再刷**（三选一：非管道不发 / 周期 5s / `file` 空不发）；机器消费者仍能拿到心跳 | 小 |
| ② | Rebuild 长时间 `0%` → 突跳"编译已完成" | `build.decl`/`build.file` → LSP → webview（E23/E29 线） | `scripts/vscode-e2e.sh` 加"**rebuild 中途必须看到非 0 百分比**"；⚠ 必须在**真宿主**里复现 | 中 |

### B. P1-b 剩余：`by` 路径 536 趟 / 19.8 MB（**勘明在附九**）

管道（6 个函数 + 3 级递归）：`elab_expr → by::run_by(663) → run_by_inner(722) → run_tactics(797) → canonical_goal_type(499) / canonical_goal_with_spec(555) → judge_render_type(1605)`。
顺序：**先只接 `canonical_goal_type`（`by.rs:763`）一处** → 加 `judge_render_type` 的就地兄弟（形状同 `infer_type_text_inplace`：源 AST + scratch hovers + `infer_type_text_at` + `proofs=true`）→ **shadow + 全课程对拍** → 再铺另三处（`by.rs:837/1101/1332`）→ 最后把 `SOKO_JUDGE_INPLACE_WIDE` 并进主开关。
⚠ **风险**：`by` 引擎的判定**决定后续 tactic 步进** ⇒ 分叉会以"步进不同"出现，**影子档是必需品**。

### C. P1-c：前缀环境"可保存/可恢复"（**真正的大头**）

`with_env_scope`（`crates/kernel/src/builder.rs:138`）**至今零调用点**；设计来源 `docs/design/incremental-environment.md` §18–§19（`with_env_scope` 版 A-2 = 当时定的最小切口；候选 B/D 已判死）。
目标：让**前缀编译产物跨 query 复用** ⇒ `JUDGE_PREFIX runs` 从 1086 再大幅降（不是再降 20%）。
**开工前**：先勘"per-前缀 builder 池 vs judge 接收调用方 builder"两条路的借用与生命周期，再定刀口。

---

## 4. 纪律与今天踩出来的坑（照抄，别重踩）

1. **口径**：报数先贴 **failed + 三条计数**（`JUDGE_PREFIX runs/bytes`、`hits/misses`、judge pass 数）**再谈墙钟**；计数减半 ≠ 墙钟同比；**<5% 差不许当结论**（本机方差 ±2.7%）。
2. **两条红线**：`--json` **剔除 `build.tick`**（它是按设计随墙钟变的心跳）后**逐字节不变**；**反向判据**（依赖/前缀真变 ⇒ 必须重算）必须有**实测**。
3. **"只做未命中"不许破**：就地路径对**每一次**调用生效会把 15 万次廉价命中换成 ~1ms/次 ✗（实测 400s 跑不完）⇒ 新接线点先问"这一趟在慢路上原本花不花前缀钱"。
4. **收尾 push 三步**：① `git fetch` ② `gh run list` 看在飞（**在飞时推会 cancel 它** ✗）③ `pull --rebase` 后再推。
   ⚠ CI 的 `e2e ledger (commit back on main)` job **每轮结束都往 main 回写提交** ⇒ 不 fetch 必被 non-fast-forward 拒。
5. **判据不许空转**：每个新接线点的测试都要断言"**就地路径真被走到**"（`INPLACE_USED` 增长）；夹具没踩到接线点时它**当场判红**（本会话靠它发现过一次）。
6. **三个实测坑**（附七 §4 有全文）：① pp 必须与 `kernel_phase:199` 同档设 `proofs=true`，且 `quiet_catch` 要包在 `with_env` **内层**；② 判定查询文本要用 `render_expr`（**不是** pp 回读入口，记法会解析失败）；③ 就地路径**不解析**（源 AST 直接造项），否则每问解析 46KB ✗。
7. **记法路径守卫**的指纹 = **去空白后的整行** ⇒ rustfmt 把闭包折成一行会变成"新增绕过"；改判定查询文本的形状前先看 `scripts/notation-paths-baseline.txt`。
8. **同一处连红 3 次 ⇒ 换招或降级**，别闷头试第 4 次；**半成品不上机**（宁可留可执行计划）。

---

## 5. 给新会话的 prompt（可直接粘贴）

```text
【续做 · P1 编译提速 —— 先读开工单，别重新勘】

开局只读三份：`docs/NEXT.md`（开工单，含"计划散落在哪"的索引）+
`docs/design/p1a-measurements.md` 的【附七/附八/附九】+ `STATUS.md` 的「未决项」。
其余文档按开工单 §2 的表**按需**查，别全文读。

## 现在在哪
P1-a（就地判定，默认 on）与 P1-b 第一档（`SOKO_JUDGE_INPLACE_WIDE`，默认关）**已落地并发布
v0.78.1**；同口径读数：墙钟 216.14s → 158.90s → **134.77s** · `JUDGE_PREFIX runs` 3759 → 1881
→ **1086** · `passes` 4126 → 2248 → **1452** · `shadow_diff=0` · off/on `--json`（剔 `build.tick`）
逐字节相同 · 反向判据三档各有实测。CI 真绿、release 26 资产。

## 现在做什么（按序，一档一个 commit）
1. **用户实测的三条 UI 缺陷**（`STATUS.md` 未决项，已定位到行）：③ Infoview 里两个"版本"没标签
   → ① 终端每秒刷 `build.tick`（`crates/cli/src/build.rs:15` + `:44-70`，修法三选一）
   → ② Rebuild 进度 0% 突跳（**必须在真 VS Code 宿主**里复现 + 加"rebuild 中途必须看到非 0
   百分比"的 e2e）。**判据必须绑"屏幕上看什么"**（AGENTS.md 验证设计纪律 §0）。
2. **P1-b 剩余：`by` 路径 536 趟 / 19.8 MB**（附九 §1–§4 有逐级函数表与顺序）：
   先只接 `canonical_goal_type`（`by.rs:763`）一处 + `judge_render_type` 的就地兄弟
   → shadow + 全课程对拍 → 再铺 `by.rs:837/1101/1332` → 最后并把 `SOKO_JUDGE_INPLACE_WIDE`
   并进主开关（那一档六条判据已齐备，默认开与否**请值守定**）。
   ⚠ `by` 引擎的判定决定后续 tactic 步进 ⇒ **影子档是必需品**，不许裸开。
3. **P1-c（真正的大头）**：前缀环境"可保存/可恢复"（`with_env_scope` 仍零调用点；
   设计见 `docs/design/incremental-environment.md` §18–§19）。目标：`runs` 从 1086 再大幅降。
   开工前先勘"per-前缀 builder 池 vs judge 接收调用方 builder"两条路的借用/生命周期。

## 纪律（照开工单 §4，别重踩）
- 报数**先计数后墙钟**；`--json` 剔 `build.tick` 逐字节不变；反向判据要实测；`<5%` 不算结论。
- **只做未命中**（就地路径对每次调用生效 = 负优化，实测 400s 跑不完）。
- 收尾 push 三步：`git fetch` → `gh run list`（在飞就别推，会 cancel）→ `pull --rebase` 再推
  （CI 的 `e2e ledger` job 每轮回写 main）。
- 每个新接线点的测试都要断言"**路径真被走到**"（不空转）；半成品不上机；连红 3 次就换招。
```

---

## 6. 收尾义务提醒（给值守/下一会话）

- **PLAN 状态列**已回写（`b77d94ce`）；本文件是**索引**，PLAN 才是发版总账 ⇒ 发版点变化时改 PLAN，别只改这里。
- `STATUS.md` 只保留最近 3 轮，旧的进 `docs/STATUS-ARCHIVE.md`；`scripts/docs-lint.py` / `status-lint.py` / `plan.py check` 三个门禁在提交前都要绿（本文件按"新文件计入基线一次"记进 `scripts/docs-budget.json`）。
