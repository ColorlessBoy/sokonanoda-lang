# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第六十八轮：`match` 依赖 motive——归纳法形状可用；0.39.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-14，第六十八轮：`match` 依赖 motive）

> 续 TODO：让 `match` 的结果类型随 scrutinee 变化（`P n`），从而能写出归纳法。

1. **设计** `docs/design/match-dependent-motive.md`（触发/构造/交互/风险）。
2. **前端**：scrutinee 是裸局部变量 `x` 且 `R` 含 `x` → motive = `fun t =>
   R[x:=t]`（`substitute_names`），分支期望 = `R[x:=<ctor 项>]`、IH 类型 =
   `R[x:=<field>]`；motive/分支/IH 类型在 binder 存活的 scope 里 elaborate。
   否则保持常量 motive（完全兼容）。
3. **修缺口**：`infer_expected_level` 改为只纳入 `R` 依赖到的 binder
   （`judge_binders_for`），修掉「无关函数型 binder 破坏 judge_infer 望远镜」
   导致**声明 binder 形式**（`nat_induction`）level 查询失败的问题。
4. **goal 视图**：match-arm 走查同样代入 `x := C params v…`，分支 `sorry` 期望
   `R[x:=ctor]`。
5. **测试**：front `match_dependent_*`（含声明 binder 的 `nat_induction`）；
   CLI `cli_match_dependent_motive_checks_via_kernel`；课程 unit5 加依赖 match 节
   （`nat_induction` + 练习 8）；golden `(9,7,4)→(10,8,4)`、汇总
   `checked 53→54 / open 40→41`。
6. **验收**：`sokonanoda gate` PASS；版本 0.38.0 → **0.39.0**（新能力 minor）。
7. **已知限制**：motive 引用「类型为以箭头结尾的依赖函数」的 binder 时，
   `judge_infer` 的 render→parse 往返仍可能腐蚀 telescope（需 `judge.rs` 改
   一次性解析，或 `proof::render_expr` 给 domain 位 `Forall` 加括号）。

## 本轮进度（2026-09-14，第六十七轮：参数化归纳声明）

> 续 TODO：match 参数化的前置——教学语言 `inductive` 声明支持参数（非带索引）。

1. **设计** `docs/design/parameterized-inductives.md`（含内核期望形状与风险）。
2. **前端**：parser 解析 `(A : Type)`/`{A : Type}` 参数 → `InductiveBlock.params`；
   归纳类型 `forall params, sort`；ctor `forall (params++fields), C params`；
   `add_inductive(num_params=params.len())`；`num_fields` 字段-only 计数；
   `derive_recursor` params 最外层 + motive `(t : Ind params) -> Sort u` + iota
   lambda/自调用带 params。`InductiveInfo` 增 `num_params`/`param_names`。
3. **match**：`Ind.rec.{level} <params> motive minors scrutinee`；params 取
   scrutinee **书写源类型**头部实参；字段 `src_ty` 做 params 替换后 elaborate；
   拿不到 → 新码 `elab-match-parameterized-unsupported`。
4. **测试**：front +8（parse 2 / compile 6，含 `Option`/`List` 派生递归子与
   `match`、显式 rec/iota、iota 错 → `kernel-rec-rule-mismatch`）；CLI +4；
   课程 unit5 加 `Option` 小节 + 练习 7；golden `(7,6,3)→(9,7,4)`、汇总
   `checked 51→53 / open 39→40`。
5. **文档**：`architecture.md §4.1/§8`、`TESTING.md`；设计 as-built §9。
6. **验收**：`sokonanoda gate` PASS；版本 0.37.0 → **0.38.0**（新语法 minor）。
7. **v1 边界**：带索引归纳、宇宙多态参数、互/嵌套递归、`match` 嵌套/守卫/字面量。

## 本轮进度（2026-09-14，第六十六轮：watch stdin 客户端命令）

> 续 TODO：compiler-service-events 设计的 v1 未做面（客户端→服务命令）。

1. **命令集**（stdin JSON Lines）：`ping {id}` → `pong {id, protocol, engine}`；
   `subscribe {file}`/`unsubscribe {file}` 过滤 `--workspace` 事件（首个
   subscribe 收窄白名单；默认全发兼容旧行为）；畸形/未知命令 → `error` 事件且
   流不中断。
2. **非阻塞实现**：后台线程 `stdin().lock().lines()` + `mpsc`，轮询每 300ms
   `try_recv` 排空；stdin EOF 不杀 watch；零新依赖（仅 std）。
3. **测试**：`crates/cli/tests/watch.rs` ping/subscribe/unsubscribe/malformed
   4 项 + watch.rs 单测 2 项（用 ping→pong 同步，不 sleep）。
4. **文档**：`docs/protocol.md` watch 小节、`TESTING.md`；设计 as-built
   `docs/design/compiler-service-events.md` §9。
5. **验收**：`sokonanoda gate` PASS；版本 0.36.0 → **0.37.0**（新能力 minor）。
