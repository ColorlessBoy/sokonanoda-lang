# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第五十五轮：编译器服务事件流——`file.didChange` + handshake + workspace；0.29.0）
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

## 本轮进度（2026-09-14，第五十五轮：编译器服务事件流）

> 续 TODO 清账（用户确认顺序 R57→R56→R55）：按
> `docs/design/compiler-service-events.md` 落地 watch 服务事件流。

1. **规范名**：watch 开场事件 `file.changed` → **`file.didChange`**（payload
   不变，新增稳定 `file` 字段）；`file.changed` 保留一个 minor 的弃用别名
   （`WATCH_VOCABULARY` 接受、不再发射）。
2. **握手**：stdout 第一行恒为 `service.hello {protocol:1, engine, pid}`
   （对齐 LSP `soko/version`；原纯文本 banner 移出 stdout）。
3. **作用域**：`watch <file>` / `--doc <file>` / `--workspace <root>`（互斥）；
   workspace 递归发现 `*.sokonanoda`，每文件一个 `Session` 与独立版本号、
   事件带 `file`、跨文件无全序。
4. **背压**：每文件有界缓冲（64），溢出合并为最新版本并标
   `recompiled_from: 0`（协议注明可全量重同步）。
5. **测试**：`crates/cli/tests/watch.rs` 6 项（握手/规范名/`--doc`/workspace
   独立版本/闭词汇 + `file`/protocol.md 覆盖）+ watch.rs 单测（溢出合并）；
   CLI 套件 105 pass、`skill.rs` conformance 绿。
6. **文档**：`protocol.md` watch 小节 + `TESTING.md` + teacher `events.md` +
   `help.rs` 同步。
7. **验收**：`sokonanoda gate` PASS；版本 0.28.0 → **0.29.0**（协议/功能 minor）。

## 本轮进度（2026-09-14，第五十四轮：elaborator `let`（Phase 1））

> 续第五十三轮的 TODO 清账：按 `docs/design/elaborator-let-match.md` 的
> S1–S5 落地**值位 `let`**（Phase 1）。`match` 依设计推迟到 Phase 2。

1. **设计**：`docs/design/elaborator-let-match.md` §11 切片 S1–S5 + 本文 §12 as-built。
2. **front（S1–S3）**：`Expr::Let`；`parse_expr` 识别 `let`（`starts_atom` /
   `named_group_ahead` 排除）；elab 分支（外层类型 + 期望类型 + `mk_let`
   `nondep=false`，缺注解 `elab-untyped-binder`）；`spine`/`proof`/`semantic`/
   `goals` 同步；`open_goal` 支持值位/body 洞。+21 测试（含 zeta 等价契约）。
3. **课程 + CLI（S4–S5）**：unit3 新增「局部绑定 `let`」小节（zh/en/钥匙，
   `def`/`#reduce` 逐字节镜像 + 两道 sorry 练习）；golden `unit3 (1,4,1)→(2,6,2)`、
   汇总 `checked 47→48 / open 34→36`；CLI e2e +3；`architecture.md` §4.1/§8、
   `TESTING.md` §1 同步。
4. **验收**：`sokonanoda gate` PASS（front 287、cli 97、lsp 108）。
5. **版本** 0.27.1 → **0.28.0**（新语法 = minor，Cargo + VSIX + CHANGELOG Added）。

> TODO 余项：`match`（Phase 2）、无注解 `let`、spine-meta A 实现、webview
> Infoview、事件流、perf 基准 + I8 early-cutoff、SHA256SUMS/attest。

## 本轮进度（2026-09-14，第五十三轮：TODO 清账——测试/文档债 + 大项设计）

> 用户要求：把 ROADMAP §10 未勾选项 + TESTING §5 盲区 + onboarding §5 待办
> **全部按流程做**（设计先行、TDD、多用 subagent）。本轮清掉测试/文档债、
> 重建内核 fixture、落地低风险安装子集，并为大项产出设计（实现留后续轮）。
> 全过程由 6+2 个 subagent 并行产出，主会话统一验证 + gate。

1. **穷尽守卫修复**：`protocol_doc_lists_every_error_code` 原先的 `matches!`
   自带 `_ => false`，根本守护不了。改成**不带通配分支的 `match kind {}`** +
   穷尽 `all` 数组（26 variant）；`docs/protocol.md` 补 `elab-apply-*` 两个
   漏掉的 code；`docs/TESTING.md` §5.1/§2 同步。
2. **codeLens / quick-fix 自动化**（补齐 §5.3 缺口，+3 测试）：
   `code_lens_ranges_match_each_declaration`、`code_action_refine_edit_targets_the_hole`、
   `code_action_open_goal_without_a_next_step_offers_none`；四族 quick-fix
   与 codeLens 全走进程内 rpc。
3. **内核 fixture 重建 + 解禁**（§5.2）：`RuleDomainMismatch`（伪造 iota 规则
   λ 定义域）与 `UnlistedRecursor`（未派生 recursor）两个 NDJSON fixture 人工
   构造，删掉 `crates/kernel/src/tests/util.rs` 的两个 `#[ignore]`，
   `cargo test -p sokonanoda` 两条均 pass（内核源码/语义未动）。
4. **安装/打包低风险子集**：`scripts/install.sh`（POSIX、零 cargo、版本锁定、
   禁用 latest）、`.devcontainer/devcontainer.json`（贡献者）、`docs/design/onboarding.md`
   §5 勾选与余项、README/docs 入口。
5. **大项设计（设计先行）**：`elaborator-let-match.md`（`let` Phase 1 详细 /
   `match` Phase 2 预研）、`spine-meta-a.md`（I9 余项方案 A）、`webview-infoview.md`
   （方案 B）、`compiler-service-events.md`（L1/L3 事件流）。
6. **ROADMAP 清账**：I10/I11/I13 三节陈旧「活规格」压缩为存档说明（指向
   `remove-funintro.md` 与现行 by/goal 设计），I6/I8/I9/I12 与两条未勾选项保留。
7. **验收**：`sokonanoda gate` PASS（含新解禁内核测试）；本轮机/文/脚本改动
   **未 bump 版本**（无运行时行为变化，0.27.1 保持）。

> 余项（各自独立轮，设计已就位）：`let`/`match` 实现（R54）、spine-meta A 实现
> （R55）、webview Infoview（R56）、事件流（R57）、perf 基准 + I8 early-cutoff
> （R58）、SHA256SUMS/attest immutable releases。
