# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第六十一轮：tactic hover 呈现升级——代码围栏 + 高亮 + 排版；0.33.1）
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

## 本轮进度（2026-09-14，第六十一轮：tactic hover 呈现升级）

> 用户反馈：tactic hover 内容有了，但只有单/两行裸文本，无排版无高亮，体验不行。

1. **表头**：`` `<tactic>` · tactic k/n ``（k/total 为 per-tactic 进度）；
   闭合显示 `已无剩余目标 ✓`。
2. **目标块**：包进 ` ```sokonanoda ` 代码围栏 → 等宽对齐 + **语法高亮**
   （扩展自带 TM 语法，hover fenced code 用该语言着色）；多目标加 `**目标 i/n**`。
3. **半截表达式 hover** 同步 `⊢` + 同款围栏。
4. **测试**：tactic hover 断言表头/围栏；half-expression 断言 `⊢`/围栏。
5. **验收**：`sokonanoda gate` PASS；版本 0.33.0 → **0.33.1**（呈现 patch）；
   设计 `docs/design/tactic-hover.md` §5 as-built。

## 本轮进度（2026-09-14，第六十轮：elaborator `match` v1）

> 续 TODO 清账（Phase 2 首切片）：按 `docs/design/match.md`（本轮 spike 定稿）
> 落地 `match`（限源内非递归 `inductive`）。

1. **可行性 spike**：手写 `Color.rec.{1} (fun _ => Color) green red c` 被内核
   接受，缺 `. {level}` 被拒 → 降低必须从期望类型 Sort 推 level。
2. **前端**：`Expr::Match`/`MatchArm`、`TokenKind::Pipe`、`parse_match`（裸 ctor、
   按声明序重排、恰好覆盖一次）；`InductiveTable` 登记表 + `ElabCtx`/`expected_src`
   贯通；降低为 `<Ind>.rec.{level} (fun _ => R) minors… e`；`goals`/`spine`/
   `proof`/`semantic`/`suggest` 同步。+19 测试（含与手写 recursor 的等价契约）。
3. **错误码**：`elab-match-{bad-arm,not-inductive,no-expected-type,recursive-unsupported,non-exhaustive}`
   （已入 protocol + 穷尽清单）。
4. **课程 + CLI**：unit5 新增「match 分情况」小节（自定义非递归枚举 + rec/iota，
   zh/en/钥匙逐字节镜像，2 练习）+ golden `(4,3,1)→(6,5,2)`、汇总
   `checked 48→50 / open 36→38`；CLI e2e +4。
5. **验收**：`sokonanoda gate` PASS；版本 0.32.1 → **0.33.0**（新语法 minor）。
6. **v1 边界（未做）**：递归归纳（IH）、依赖/参数化归纳、prelude `Nat`/`Eq`、
   `match` tactic、嵌套/字面量/守卫模式、无注解 `let`。

## 本轮进度（2026-09-14，第五十九轮：I8 early-cutoff + arena 基准立项）

> 续 TODO 清账（R58）：ROADMAP I8 验收余项「受影响后缀的依赖精确化」+ TESTING
> §5 perf 基准立项。

1. **设计** `docs/design/early-cutoff.md`（机制/soundness/测试/边界）。
2. **early-cutoff（保守 sound）**：每条命令在 `try_check_declar` 前用内核
   结构化 `debug_print` 渲染「环境贡献签名」（kind+name+宇宙+type+**body**+
   hint+ctor/recursor/iota；归纳块串联），存 `CmdSnapshot.signature`（不改
   `--json`/LSP 形状）。单点编辑时累积 `[i, j)` 签名，遇到文本不变且签名与上轮
   相同的 `j` 即停止，`[j, n)` 快照复用、内核检查跳过；任何内核拒绝或多点编辑
   一律退回旧后缀重查；prelude 形状守卫变化整文件重建。body 进签名保证 delta
   可观察性 sound。
3. **效果**（测试实测 kernel_checks）：`def one := 1 → (1)` 4→**1**；axiom 3→**1**；
   Nat 归纳块 6→**1**；改 body/宇宙元数/多点编辑不 cut（正确重查）。
4. **arena 基准立项**：`scripts/perf-arena.sh`（opt-in，`LEAN_KERNEL_ARENA`
   门控，未设给获取指引并跳过；不 vendor、不进 CI、不引入官方 Lean 工具链）+
   `docs/PERF.md`「External baseline」；TESTING §5 盲区第 6 条更新。
5. **文档/清单**：`docs/design/i8-i9.md` §4 更新（early-cutoff 已补做）；
   `ROADMAP.md` I8 余项勾选。
6. **验收**：`sokonanoda gate` PASS（front 298、perf 3、cli 105、lsp 112）；版本
   0.32.0 → **0.32.1**（内部性能优化 → patch）。
