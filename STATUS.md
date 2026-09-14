# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第五十三轮：TODO 清账——测试/文档债 + 内核 fixture + 安装子集 + 大项设计）
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

## 本轮进度（2026-09-14，第五十二轮：restart server 版本纪律修复）

> 用户报告：下载 0.27.0 插件后 LSP 仍报 0.26.0，疑发布流程有问题；要求
> `sokonanoda: restart server` 应校验版本、不重用旧的缓存 LSP。

1. **核实发布**：下载 `v0.27.0` 的 darwin-arm64 VSIX，内置 LSP 实测
   `soko/version = 0.27.0`；已安装 0.27.0 扩展的 `server.js` 解析到内置
   0.27.0。发布无误；0.26.0 来自本地下载缓存（`sokonanoda version` 报的是
   缓存，非运行中的服务器）与宿主未重载。
2. **定位缺陷**：`resolveServerCommand` 本就拒绝过期缓存（stale → undefined，
   有单测钉住），但 `restartServer` **没有激活路径的版本锁定下载兜底**：
   解析返回 undefined 时它保持 `serverOptions` 不变并 `client.restart()`，
   于是静默重启了旧的（可能来自过期缓存的）服务器。
3. **修复**：抽出 `resolveServerForStart`（激活/重启共用：显式路径 → 内置 →
   工作区 → 当前缓存 → `v<扩展版本>` 锁定下载）；restart 用它，拿不到可用
   服务器时报错而不重启旧命令；新增 `newestInstalledExtensionVersion`，检测到
   磁盘上更新的扩展而当前宿主仍旧时提示 `Developer: Reload Window`。
4. **测试**：CLI 契约 `restart_server_re_resolves_and_never_keeps_a_stale_command`
   钉住共用解析器 + 下载兜底 + 更新提示；`node test-server.js` 18/18 绿。
5. **版本** 0.27.0 → **0.27.1**（fix → patch）；CHANGELOG Fixed、
   `docs/vscode-dev-guide.md` §5.6 同步。

## 本轮进度（2026-09-14，第五十一轮：tactic 关键字高亮 + hover goal state）

> 用户反馈：`exact` 没有正确高亮；希望像 Lean 一样在每个 tactic 上 hover 看到
> 中间 goal state（Infoview 式），或按鼠标位置给 goal state。

1. **设计** `docs/design/tactic-hover.md`。
2. **高亮**：`semantic::KEYWORDS` 增补 `by`/`exact`/`assumption`/`rfl`（此前
   当普通标识符着色；`forall`/`sorry` 已由 token/Hole 正确处理）。
3. **hover**：`textDocument/hover` 首插 `tactic_goal_hover`——光标落在某 tactic
   span 内 → 用 `by_steps` + `select_state_at`（进入态语义，与 `soko/stateAt`
   同数据同规则）渲染全部目标与假设（多目标标 `目标 i/n`），range = 该 tactic；
   纯快照消费，零重编译零文本扫描。
4. **测试**：front semantic 断言四关键字均 Keyword；LSP
   `hover_on_a_tactic_shows_the_entering_goal_state`（hover `apply` → `⊢ And P Q`；
   hover `sorry` → `⊢ P` / `⊢ Q`）。
5. **协议**：`docs/protocol.md` 新增「Tactic goal-state hover」小节；并入 0.27.0。
6. **验收**：`sokonanoda gate` PASS。
