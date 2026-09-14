# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第五十二轮：restart server 版本纪律修复；0.27.1）
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

## 本轮进度（2026-09-14，第五十轮：移除值位关键字 funintro）

> 用户评估：「`funintro` 跟 `funapply` 一样，实现起来稀里糊涂的，不如直接删了。」
> 确认按「彻底删」执行，与多目标显示并入 0.27.0。

1. **设计先行** `docs/design/remove-funintro.md`（根因/方案/测试/验收/as-built）。
2. **前端**：删 `Expr::Intro`、`parse_intro`/原子位/lambda 尾关键字分支、
   `KEYWORDS` 的 `funintro`、`compile/intro.rs`、`DeclState.intro_skeleton`、
   `ErrorKind::ElabIntroNotAFunction`；各 crate 匹配臂与测试同步。
3. **协议/客户端**：`soko/stateAt` 的 `goals` 相关不受影响；删 LSP 值位关键字
   补全/hover/code action/inlay 全路径、VS Code `sokonanoda.expandIntro`
   命令与 `markdown.isTrusted` 白名单；`docs/protocol.md` 值位关键字小节改为
   「已移除」说明。
4. **课程**：unit6（zh+en）改写「补充 funintro」段 + 练习 6 为综合 `by` 练习，
   钥匙同步；golden 计数不变。
5. **文档/site/技能**：architecture/README/TESTING/ROADMAP(I13 标废弃)/
   term-intro/value-keywords-v2(废弃横幅)/site hero 换图/gen-site-demos 删演示/
   teacher 技能表同步；历史归档与 CHANGELOG 原文保留。
6. **测试**：`cli_value_funintro_is_no_longer_a_keyword` 钉「已非关键字」；
   其余 funintro 测试全删。
7. **验收**：`sokonanoda gate`（fmt/clippy/test/playground 锚点）。
