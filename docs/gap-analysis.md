# 业内标准差距审计（2026-09-07，外部调研 subagent 产出）

> 输入本轮头脑风暴；对标 rust-analyzer / gleam LSP / ocaml-lsp / coq-lsp /
> lean4game(NNG) / Deduce / Isabelle。全文结论浓缩；来源清单见调研原报告
> （completions 是 Deduce 课堂实证的第一痛点；MSRV 反例 = rust-analyzer
> issue #18730；洞位置 server 端计算 = ocaml-lsp #1516 教训）。
> **进度（2026-09-07 第十四轮）**：Top 10 + 附加小项全部落地（含
> criterion、fuzz、稳定 hole_id、归纳块 recursor 自动派生）。剩余：
> refine 子洞 kernel 级 expected type（方案 B，`docs/design-round14.md`
> §0.4）、失败声明 Restart 之外的针对性建议。

## 已达标（本轮盘点确认）

诊断（带版本+代码+提示）、hover/goal 视图、`soko/goals`（含稳定 hole_id）+
`soko/nextHole`、semanticTokens、codeLens、code action（exact/intro/refine/
restart）、真增量（I8）、CI lint+test+golden+conformance、agent skills、
VSIX 契约守护、release 流水线、completions、go-to-def、folding、
`soko/hints` 提示阶梯、undo、rename+references、inlay hints、下一步建议
（is_preferred 排序）、`--version`+MSRV、`sokonanoda lsp` 子命令、
课程地图（`sokonanoda course` + VS Code 课程树）、REPL 历史持久化、
criterion 基准、fuzz harness、内核错误分类学（assert_eq 全量分诊 +
rec-rule-mismatch 家族）、归纳块 recursor 自动派生。

## Top 10 补全清单（按投入产出比）

| # | 项 | 为什么是标配 | 规模 |
|---|---|---|---|
| 1 | completions（关键字+作用域内名字+白名单 lemma） | Deduce 实证第一痛点；所有对标服务器皆有 | S |
| 2 | go-to-definition（+ document highlight） | hover/exact 链路已有名字→声明解析，差 span 回填 | S |
| 3 | folding range | `inductive...end` 块刚需；gleam/ocaml-lsp/rust-analyzer 全有 | S |
| 4 | 提示分级（hint 阶梯 + `soko/hints`） | lean4game context-match/hidden 机制；Deduce 实证最高价值 | M |
| 5 | REPL `undo` + 命令历史 | lean4game undo、Isabelle back-tracking 基线 | S |
| 6 | rename + find-references | 可信度信号；名字映射已有 | S–M |
| 7 | inlay hints（洞期望类型/宇宙参数） | Lean InfoView 最小化形态 | M |
| 8 | `soko/courseStatus` + VS Code 章节地图 | lean4game 进度树是教学标配；进度=文件状态聚合 | M |
| 9 | 失败洞的"下一步建议"（按目标形状） | judge 已可合成声明；coq-lsp 信息面板同方向 | M–L |
| 10 | `--version` + MSRV（workspace `rust-version`） | CLI 零门槛标配；MSRV 缺失的教训见 rust-analyzer | S |

附加小项：criterion 基准（防 I8 增量静默劣化，本地跑）、cargo-fuzz parser
harness（定期跑）、`sokonanoda lsp` 子命令（单二进制分发，gleam 模式）、
`soko/*` 洞的稳定 hole_id（Deduce MCP 先例）。

## 明确不做（反标配）

canonical formatter（与洞/span 稳定性冲突）、call/type hierarchy、
selectionRange、pull diagnostics、salsa 查询框架（I8 已解决）、
workspace/依赖管理、浏览器/WASM 编辑器、成就系统、LSIF/SCIP、
SECURITY/GOVERNANCE（触发条件：公开贡献者/公开 VSIX）、mdbook 文档站
（触发条件：外部用户群）。
