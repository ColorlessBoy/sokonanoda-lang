# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第六十七轮：参数化归纳声明（非带索引）+ match；0.38.0）
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

## 本轮进度（2026-09-14，第六十五轮：`match` 支持 prelude Nat）

> 续 TODO：移除「match 只支持源内 inductive」限制，让学生直接对 prelude `Nat`
> 分情况。

1. **根因**：prelude 的 `Nat` 是「原生 hack」（无 ctor 的 add_inductive +
   `Nat.zero` 公理 + 自引用 `Nat.succ` 定义），**没有 `Nat.rec`**，`match` 无从
   降低。改为经 **`install_inductive_block` 装成真实可信归纳**（ctor `Nat.zero`
   /`Nat.succ` + 派生 `Nat.rec` + iota，`recursive=true`，`rec_universe_arity=1`），
   并注册进 `InductiveTable`；`Nat.add` 保持原生自引用定义。
2. **效果**：`match n with | Nat.zero => … | Nat.succ k => …`（点号 ctor）可用，
   递归字段后自动 IH；`#check Nat.rec` 有签名；`#reduce 1 + 1 => 2`、
   `Nat.add 2 3 => 5`、`three => 3` 均正常。**已知显示**：经 `Nat.rec` 归约的
   结果可能是不合并的一元链（如 `addN 2 1 → Nat.succ (Nat.succ 1)`，与 numeral
   def-eq），已文档化并钉测试。
3. **测试**：front +3（prelude Nat pred/add + 源内 Nat 回归）；CLI e2e +1。
4. **文档**：`match.md` §2/§10、`architecture.md` §4.1/§4.2/§5.4/§8、
   `TESTING.md`、`protocol.md`（`elab-match-not-inductive` 文案）同步。
5. **验收**：`sokonanoda gate` PASS；版本 0.35.1 → **0.36.0**（新能力 minor）。
