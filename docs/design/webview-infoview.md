# 设计：VS Code webview goal 面板（方案 B）—— Lean Infoview 式目标态（2026-09-14）

> 触发：`docs/design/by-tactics.md` §6.3「方案 B（Lean Infoview 式 webview）
> 留后续」、`docs/design/tactic-hover.md` §2.3、`docs/design/goal-list.md`
> §2.5、ROADMAP §10 I9「VS Code goal 面板」。依据：现有「当前光标处」树
> （方案 A，`extension.js:307`）与其性能教训
> （`docs/design/goal-list.md` §2.4）。**本文只定方案，不改码。**
>
> **更新（0.40.0，见 `docs/design/goal-rendering.md`）**：视图已移出 explorer，
> 落到 `viewsContainers.secondarySidebar` 的 `sokonanoda` 容器（**右侧辅助侧栏**，
> 需 VS Code ≥ 1.106）；goal/假设的着色改为服务器下发的 `goal_runs`/`ty_runs`
> （单一分类源 `front::semantic`），webview 不再自绘规则；`openInfoview` 去掉
> `ready` 握手与「暂时不可用」提示（静默回退树组）。本文其余部分仍为 as-built 记录。

## 1. 现状与动机

「当前光标处」树组已经可用：选区变化去抖 200ms → `soko/stateAt`，渲染
每目标的 `goal` 文本 + 假设 + `by 进度`（`extension.js:212,307`）。数据面
完全就绪：`soko/stateAt` 返回 `goals[]`（当前在前）、每目标 `binders`、
`step/total`、文档 `version`；`soko/goals` 返回每声明的 `goals`/
`binders`/`holes`/`sub_goals`。协议见 `docs/protocol.md`。

树形渲染的天花板：TreeItem 不能富文本 / 语法着色 / 紧凑表格；
多目标与假设是平铺列表，无 Lean Infoview 的「目标分栏 + 代码风格」观感；
面板与编辑器不能并排常驻。方案 B 就是补这一层**纯展示**。

## 2. 决策

- **保留树为默认与兜底**：webview 是**增量能力**，不是替换。树组继续消费
  `soko/stateAt`，零配置可用；webview 失败时功能不丢。
- 新增一个 **`WebviewViewProvider`**（侧栏 view container `sokonanoda`
  内，与「练习」「课程」并列），命令 `sokonanoda.openInfoview` 聚焦；
  需要时允许 provider 的 view 在辅助栏打开。
- webview **不直连 LSP**：扩展宿主是唯一持有 `LanguageClient` 的一方，
  它把 `soko/stateAt` 快照 post 给 webview。协议版本 `1`。

## 3. 消息协议

宿主 ↔ webview 只走结构化 JSON，无求值、无命令字符串：

| 方向 | 消息 | 载荷 |
|---|---|---|
| host → webview | `state` | `soko/stateAt` 响应原文 + `protocol:1` |
| host → webview | `decls` | `soko/goals.decls`（诊断/切文件时；cursor 移动不发） |
| host → webview | `server` | `{running, version, pid}`（`soko/version`） |
| host → webview | `theme` | `{kind}`（dark/light/high-contrast） |
| webview → host | `ready` | 握手；宿主收到后回发 `state`/`decls` |
| webview → host | `reveal` | `{uri, range}` → 执行 `sokonanoda.revealRange` |
| host → webview | `status` | `{state: loading\|ready\|idle, decls?}`：`编译中…`/`已就绪 · N 个声明`/`等待 .sokonanoda 文件`（0.49.0） |

- 每条消息带 `protocol`；`state` 的 `version` 与文档版本一致，webview
  **丢弃过期快照**（与宿主侧 `cursorRequestSeq` 同纪律）。
- `goal`/`ty` 一律当作**不可信文本**渲染：内核 pretty-printer 的产物，
  webview 不解析 Lean、不重新着色（可选等宽字体），只 `textContent`。

## 4. 安全 / CSP

- `webview.options = { enableScripts: true, localResourceRoots:
  [context.extensionUri/media] }`；HTML 带
  `Content-Security-Policy: default-src 'none'; style-src
  ${webview.cspSource}; script-src 'nonce-<per-load>'; img-src
  ${webview.cspSource}`。
- 无远程资源、无内联事件属性（`onclick=` 禁止）、无 `eval`；nonce 每次
  创建随机。
- 数据注入只用 `textContent` / `createTextNode`，**禁止 `innerHTML`**
  （契约测试负断言守护）；消息结构固定，不反射进 DOM 选择器。

## 5. 性能（光标移动卡顿教训）

`docs/design/goal-list.md` §2.4 的根因是「每次光标移动全量 `refresh()` →
`soko/goals` + 重建整棵树」。webview 必须避开同一坑：

- **cursor 移动只发 `state`**（轻量），仍去抖 200ms；**绝不**因此发
  `decls` / `soko/goals`；
- DOM 只增删目标/假设节点（目标数小），不做整页重建；
- `retainContextWhenHidden: false`，view 隐藏即释放；provider 缓存最后一份
  `state`/`decls`，重新可见时先渲染缓存再刷新；
- `soko/stateAt` 本身是缓存快照查询（无内核、无网络放大），再加去抖即可。

## 6. 兜底

webview 不可用时（老 VS Code / `enableScripts` 被禁 / 创建失败 / `ready`
超时）→ 静默保留树的「当前光标处」组，`openInfoview` 给一条说明消息。
webview 是只读展示层：任何消息失败都不得影响诊断、树或编辑器。

## 7. 扩展版本纪律

新增 view + 命令 = 学习者能做之前做不到的事 → 按
`docs/vscode-dev-guide.md` §2 **minor** bump；同步 `CHANGELOG.md`
（`Added`）、`README.md` / `package.json` description（§7 三个门面文件）；
Rust 与扩展版本必须一致（契约测试 `cargo_and_extension_versions_match`）。

## 8. 测试三层

- **静态契约**（`crates/cli/tests/extension.rs`）：`package.json` 声明
  view/命令一致；webview 资源在 `media/` 且被引用；HTML/CSP 含 nonce；
  **负断言** webview 脚本不含 `innerHTML`、不含远程 URL、不含 `/latest/`；
  客户端继续消费 `soko/stateAt`、不文本扫洞。
- **集成**（`editor/vscode/src/test/extension.test.js`，
  @vscode/test-electron）：命令打开 view、`ready` 握手后收到 `state`、
  `reveal` 消息驱动 `revealRange`；服务器二进制缺失时 skip 不 fail
  （既有模式，`docs/TESTING.md`）。
- **手动**（F5 开发宿主）：大文件光标移动无卡顿、多目标分栏、假设可读、
  dark/light 主题、view 隐藏/恢复不泄漏上下文。

## 9. 验收

- 光标移动不触发 `soko/goals`、无可见卡顿（对照 §2.4 症状）；
- 多目标 + 各自假设 + `by k/n` 在 webview 正确渲染，`goal: null` 显示
  「已无目标 ✓」；`reveal` 跳转与树一致；
- webview 不可用时树功能零回归；
- `npm run test:unit`、静态契约、集成测试、`sokonanoda gate` 全绿；
- `docs/protocol.md`（若新增宿主→webview 契约说明）、`STATUS.md`、
  `REQUIREMENTS.md §9` 同步；版本按 §7 bump。

---

## 10. as-built（2026-09-14，0.30.0）

- **view/命令**：`sokonanoda.infoview`（`type: webview`，与「练习」「课程」
  并列）+ `sokonanoda.openInfoview`（view/title 导航）；树「当前光标处」组
  保留为默认与兜底。
- **资源**：`editor/vscode/media/{infoview.html,infoview.css,infoview.js}`；
  CSP `default-src 'none'` + 每次随机 nonce；`enableScripts:true`、
  `localResourceRoots=[media]`；渲染只用 `textContent`（契约负断言禁
  `innerHTML`/远程 URL/eval/内联事件）。
- **协议**（`protocol:1`）：host→webview `state`（`soko/stateAt` + `uri`）、
  `decls`（仅诊断/切文件）、`server`（`soko/version`）、`theme`；
  webview→host `ready`/`reveal`；按 `version` 丢弃过期
  `state`。
- **性能**：光标移动只发 `state`（去抖 200ms，复用既有 selection 监听），
  绝不因此发 `soko/goals`；provider 缓存最后 `state`/`decls`，
  `retainContextWhenHidden:false`。
- **测试**：`crates/cli/tests/extension.rs` 静态契约（view/命令一致、资源与
  CSP nonce、负断言）；`extension.test.js` 集成 smoke；`node test-server.js`
  18/18。
- **版本** 0.29.0 → **0.30.0**（新 view + 命令 = minor）。

## 11. as-built 增补（0.49.0）

- **可见性根因与修复**：视图原带 `when: resourceLangId == sokonanoda`，而扩展容器
  固定 `hideIfEmpty: true`（VS Code `viewsExtensionPoint.ts:416`）→ 无激活
  `.sokonanoda` 文件时容器为空被隐藏，聚焦命令无效。现：视图**无 `when`** +
  `visibility: visible`；`activationEvents` 增 `onView:sokonanoda.infoview`；
  `openInfoview` 依次 `focusAuxiliaryBar` → `workbench.view.extension.sokonanoda`
  → `sokonanoda.infoview.focus`。
- **激活顺序**：`activate()` 原来先 `await resolveServerForStart`（读盘/可能下载，慢）
  才注册 `registerWebviewViewProvider` → 期间视图无 provider（空白/"点几次才出现"）。
  现：**同步注册 provider/树先**，服务解析与启动放到之后的异步续段（失败 `.catch`），
  并向 webview 推送 `status`。契约测试
  `infoview_provider_is_registered_before_the_server_resolution_await`。
- **UI 反馈**：webview 载入即渲染骨架（`正在渲染…`）；宿主推 `status`，面板显示
  编译进度，绝不静默空白。
- **声明列表**：**去掉点击跳转**（未生效），改为名字后小字**行号** `L<n>`
  （1-based，来自 `range.start.line + 1`）+ 类型提示（`ty_runs` 着色、可换行）。
  测试 `test-webview.js`（Node DOM shim 行为测试）+ 静态契约。
