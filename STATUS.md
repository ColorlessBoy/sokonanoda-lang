# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-16（第八十一轮：`by` 块换行分隔 tactic；0.51.0）
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

## 本轮进度（2026-09-16，第八十一轮：`by` 块支持换行分隔 tactic）

> 用户：能不能像 Lean4 一样用**分号或回车换行**两种分隔，从而省掉行尾的 `;`
> （举了 `playground.sokonanoda` 的 `forall_and` 为例）。

1. **难点**：`exact`/`apply` 的表达式会贪婪跨行（换行只是空白），`exact f` 换行
   `apply g` 会被读成应用 `f apply g`。
2. **规则**：解析 tactic 时（`by_depth > 0`），若下一 token 在**更晚的行**且是
   **tactic 关键字**（`intro/exact/apply/assumption/rfl/match/sorry`），当前表达式结束。
3. **实现**：`Parser.by_depth`（`parse_tactic` 包一层，Ok/Err 都减）；
   `starts_atom` 在边界处返回 false（应用不吞下一行 tactic）；`parse_by_block` 在
   `;` 或「下一行 tactic 关键字」时继续。仍未引入缩进敏感。
4. **测试**：parser 5 项（换行分隔 / 边界胜过应用 / 多行项仍是单 tactic / `;` 与换行混用 /
   不吃下一个命令）；CLI `cli_by_newline_separated_tactics_check_via_kernel`。
5. **活样例**：`playground.sokonanoda` 的 `forall_and` 去掉行尾 `;`（gate 仍跑该文件）。
6. **有意不支持/歧义**（写入 by-tactics.md §11）：同行不写 `;` 不算分隔；续行以 tactic
   关键字开头的多行项会被切开（用括号/同行规避）；`sorry` 在下一行即视为新 tactic。
7. **验收**：front 378 + CLI 全绿；`playground.sokonanoda` exit 0；`sokonanoda gate` PASS；
   版本 0.50.0 → **0.51.0**。

## 本轮进度（2026-09-16，第八十轮：Infoview 自研调色板（主题解析回退））

> 用户反馈：Infoview 里 `Type`/`Prop` 没高亮、参数色与编辑器/hover 不一致，问能否
> 与主题自动对齐。调研结论：VS Code **无稳定 API** 暴露主题 token 色（2026-06 只有
> proposal #319754/#319753）；唯一路线是自己复刻主题解析。先按此实现了完整解析器
> （`theme-colors.js` + 宿主解析主题 JSON + `colors` 消息 + 测试，18 项单测），
> **用户判定代价过大** → 回退，改为 **Infoview 自研固定调色板**。

1. **根因（已修）**：`.tok-sort` 用 `--vscode-symbolIcon-structForeground`，主题未定义
   时回退 `--vscode-foreground` → `Prop`/`Type` 看着没高亮；`.tok-binder` 用 symbolIcon
   palette，天然不同于编辑器的 parameter token 色。
2. **调研**：官方仅有两个 proposal（`ColorTheme.tokenColors`、`languages.getDocumentTokens`）；
   可行但昂贵的路线是读活动主题 JSON（含内置主题）展开 `include` + `tokenColors` +
   `semanticTokenColors` + `editor.tokenColorCustomizations` 后做 TextMate 特异性匹配。
3. **回退**：删除 `editor/vscode/theme-colors.js`、`test-theme-colors.js`、`colors` 消息
   通道与宿主解析（grep 证明零悬空引用）。
4. **落地**：Infoview 自研固定调色板——`:root` + 四个 `body[data-theme=…]` 定义
   `--soko-{type,keyword,function,variable,parameter,number,enum,macro}`（dark/light 贴近
   Dark+/Light+ token 色）；`.tok-*` **只**读 `--soko-*`（不再有 symbolIcon 优先链 →
   任何主题必有着色）；workbench 前景/背景仍走 `--vscode-*`。
5. **测试**：`infoview_palette_colours_every_kind_with_a_guaranteed_fallback`（每个
   `.tok-<kind>` 解析到 `--soko-*` 且四个主题块都定义）；`test-webview.js` 10 项。
6. **验收**：node 三套 + `cargo test -p sokonanoda-cli --test extension`（32）全绿；
   `sokonanoda gate` PASS；版本 0.49.0 → **0.50.0**。
7. **诚实边界**：Infoview 颜色与编辑器/hover **不逐像素相同**（后者是主题 token 色，
   前者是自有色板）——这是平台限制 + 用户拍板的取舍，写入 `docs/design/highlighting.md` §3b。

## 本轮进度（2026-09-15，第七十九轮：共享缓存 + `build` / Infoview 稳定与反馈 / 高亮单一起源）

> 用户三轮反馈：(a) Infoview 面板"点几次才出现、很不稳定"，怀疑是编译卡住，要求
> 面板 UI 必须保证出现、数据可显示"渲染中"/编译进度；(b) 去掉没生效的声明点击跳转，
> 名字后加小字行号；(c) hover 的高亮与 Infoview 不一样、没有收拢。另要求
> `sokonanoda build` 这类命令配合缓存。派出 5 个 subagent 分头实现（SA-1…SA-B）。

1. **共享编译缓存（SA-1）**：缓存下沉到 `crates/front/src/compile/cache.rs`，
   条目含 `report` + `output`；key = `CACHE_FORMAT|版本|二进制构建指纹|prelude 模式|源文本`；
   `SOKONANODA_CACHE_DIR`/`SOKONANODA_NO_CACHE`；原子写；`compile_all_with` 一趟出两者。
2. **`sokonanoda build`（SA-2）**：`build [--json] [--clean] [<file>|<dir>…]` 预热/清理
   缓存并打印 hit/compiled/failed；`course` 与批量 `--json` 走 `compile_cached`
   （冷热输出逐字节一致，有测试）；CLI 测试用临时 `SOKONANODA_CACHE_DIR` 隔离。
3. **Infoview 稳定性根因（SA-3）**：视图原带 `when` + 扩展容器 `hideIfEmpty: true`
   → 无激活 `.sokonanoda` 时容器整块隐藏；且 `activate()` **先 `await
   resolveServerForStart` 才注册 provider** → 期间视图无 provider（"点几次才出现"）。
   修：视图无 `when` + `visibility: visible`；`activationEvents` 加
   `onView:sokonanoda.infoview`；provider/树**同步先注册**，慢解析后置并推 `status`；
   `openInfoview` 先开辅助栏。契约测试锁死注册顺序。
4. **UI 反馈（SA-3）**：webview 载入即骨架（`正在渲染…`），宿主推 `status`
   （`编译中…`/`已就绪 · N 个声明`/`等待 .sokonanoda 文件`），绝不静默空白。
5. **声明列表（SA-3）**：去掉点击跳转；名字后小字行号 `L<n>`（1-based）+ 类型提示。
   新增 `editor/vscode/test-webview.js`（Node DOM shim 行为测试 8 项）并入 `test:unit`。
6. **高亮单一起源（SA-A/SA-B）**：`SemanticKind::{ALL, as_str, tm_scope}` 唯一表；
   `runs_to_text`/`goal_text`/`goal_runs` 单一文本生产者；hover 目标态改由 runs 投影
   （不再手搓字符串）；TM 语法补齐 `variable.parameter` 等 scope；三处穷尽测试
   （TM scope / CSS 类 / LSP legend）防漂移。设计 `docs/design/highlighting.md`，
   并**写明平台限制**：markdown 只能 TM 着色 → 颜色近似而非全等。
7. **验收**：`cargo test --workspace --locked` 全绿 + `sokonanoda gate` PASS；
   `build` 冷/热/clean/目录/课程缓存冒烟通过；版本 0.48.0 → **0.49.0**。

