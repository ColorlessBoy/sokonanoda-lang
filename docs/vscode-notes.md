# VS Code 扩展接入语言服务器：调研笔记

> 目的：为 `editor/vscode` 薄壳的下一步（进度树、goal 视图、打包）提供依据。
> 关联：`docs/design-infrastructure.md`（LSP-first 产品形态）、`crates/lsp/src/main.rs`（tower-lsp 服务器）。
> 调研时间：2026-09-06。中文行文，代码/标识符保持英文。

---

## TL;DR：下一里程碑对 `editor/vscode` 的逐文件计划

| 文件 | 动作 |
|---|---|
| `package.json` | ① 显式保留 `"activationEvents": ["onLanguage:sokonanoda"]`（隐式激活有坑，见 §1.3）；② 加 `"extensionKind": ["workspace"]`；③ `engines.vscode` 升到 `^1.85.0` 并用同版本 `@types/vscode`；④ 新增 contributes：`views`（练习进度树）、`configuration`（`sokonanoda.serverPath`、`sokonanoda.trace.server`）、更多 commands（restart server / show goal）；⑤ `vscode:prepublish` 指向真实打包脚本。 |
| `extension.js` | ① 修正生命周期：`await client.start()`，`deactivate` 返回 `client?.stop()`（现在把 start 的 Promise 塞进 subscriptions 是无效的）；② 服务器路径解析顺序：设置项 `sokonanoda.serverPath` → env → `context.asAbsolutePath('out/sokonanoda-lsp')`（打包后二进制放进扩展目录）→ PATH；③ 加 `middleware`/`trace` 便于诊断；④ 注册 VS Code 端 commands（`sokonanoda.status`、`sokonanoda.showGoal`）；⑤ TreeDataProvider + 自定义请求 `sokonanoda/status`；⑥ goal webview 面板挂后（I8）。 |
| `language-configuration.json` | 确认 `--` 行注释、括号对、`folding.markers` 按 §6.2 检查（文件名以 `language-configuration.json` 结尾才有编辑器校验）。 |
| `.vscodeignore` + 打包脚本 | esbuild 打包 `extension.js` → `out/extension.js`；`.vscodeignore` 排除 `node_modules`、`src`；二进制走 per-platform VSIX（§2.3）。 |
| `.vscode/launch.json` | Launch Client + Attach to Server + compound（§7）。 |

优先级：进度树（F3 的 UI 形态）> goal 面板（F5，依赖 I8）> 打包（L2）。前三项都不需要改服务器协议，只需要在 `crates/lsp` 增加 2 个自定义请求：`sokonanoda/status`（整文件逐声明状态，喂进度树）和 `sokonanoda/goal`（光标处 goal + 假设，喂 goal 面板）——这正是 Lean 4 的做法（§3）。

---

## 1. vscode-languageclient 标准接入

### 1.1 LanguageClient 与 ServerOptions

官方路径就是 `microsoft/vscode-extension-samples` 的 [lsp-sample](https://github.com/microsoft/vscode-extension-samples/blob/main/lsp-sample/client/src/extension.ts)，完整教程见 [Language Server Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)。要点：

- `serverOptions = { run, debug }`：debug 模式仅在扩展以调试方式启动时生效，可塞 `execArgv`（Node 服务器用 `--inspect=6009`）。
- 原生二进制（我们的 Rust 服务器）用 `command` 形式 + `TransportKind.stdio`：
  ```js
  { run: { command: bin, transport: TransportKind.stdio }, debug: { command: bin } }
  ```
- 注意坑：languageclient 在 stdio 模式下会**自动附加一个 `--stdio` 参数**（LSP 规范的实现建议）。tower-lsp 会忽略多余 argv，无害；若要完全控制可用函数形式的 serverOptions（返回 `Promise<ChildProcess|StreamInfo>`）。见 [vscode-languageserver-node#1440](https://github.com/microsoft/vscode-languageserver-node/issues/1440)（dbaeumer 的答复）。
- stdio 服务器**绝不能往 stdout 打日志**，否则消息流被污染（[issue #644](https://github.com/microsoft/vscode-languageserver-node/issues/644)，dbaeumer："the server can't log anything to stdio"）。tower-lsp 的 `log_message` 走 stderr，安全。
- `LanguageClientOptions` 关键项：`documentSelector`（我们用 `[{ language: 'sokonanoda', scheme: 'file' }]`）、`synchronize.fileEvents`（fileSystemWatcher）、`initializationOptions`（把 VS Code 设置传给服务器）、`middleware`（拦截请求/通知做客户端逻辑）、`revealOutputChannelOn`（服务器崩溃时是否弹出输出面板）。

### 1.2 生命周期（8.x/9.x 版本）

自 vscode-languageclient 8.0 起（[仓库 README 更新日志](https://github.com/microsoft/vscode-languageserver-node)）：

- `client.start()` 返回 Promise：`await client.start()`；`onReady()` 已删除。
- `deactivate` 应返回 `client?.stop()`；`stop()` 本身也是异步的。
- handler 注册（`onRequest`/`onNotification`）可以在 start 之前做，避免漏消息；在客户端未就绪时发送请求会自动先启动客户端。
- **当前 `editor/vscode/extension.js:35` 的 `context.subscriptions.push(client.start())` 把 Promise 当 disposable 注册，不会随扩展卸载被清理**——需要改成 `context.subscriptions.push(client); await client.start();`。

### 1.3 激活事件（2025+ 规则）

[Activation Events 参考](https://code.visualstudio.com/api/references/activation-events)：

- 自 VS Code 1.74 起，**扩展自己贡献的语言/命令/视图**不再需要显式声明 `onLanguage:`/`onCommand:`/`onView:`——contributes 即隐式激活。
- **但有个已知坑**：隐式 `onLanguage` 只在语言声明了 `language-configuration.json` 时才生效，否则不激活（[microsoft/vscode#204333](https://github.com/microsoft/vscode/issues/204333)，官方答复"as designed，建议显式声明"）。我们已有 language-configuration.json，但**保险做法是显式写 `onLanguage:sokonanoda`**（现在的 package.json 已经这么做了，保留）。
- `workspaceContains:**/*.sokonanoda` 可选加：打开含 `.sokonanoda` 的文件夹时即使没打开文件也激活，让进度树尽早出现。
- 尽量不要用 `"*"`；`onStartupFinished` 是其温和替代。

### 1.4 打包 Rust 服务器二进制（三种流派）

[软件工程栈exchange 上的对比](https://softwareengineering.stackexchange.com/questions/426471/how-do-vs-code-langauge-extensions-distribute-a-language-server-binary)与 rust-analyzer 的实践（[VS Code 章节](https://rust-analyzer.github.io/book/vs_code.html)）：

1. **扩展内下载**：首次激活时按平台从 GitHub Releases 下载 + 校验 checksum，放到扩展安装目录或 `globalStorage`。rust-analyzer 就是这种：二进制存放在 `~/.vscode/extensions/rust-lang.rust-analyzer-*`，用户可用 `rust-analyzer.server.path` 覆盖；VSIX 也有 `no-server` 变体给手动装二进制的用户。优点 VSIX 小；缺点首次启动要网络、要校验（sha256 对照 release 附件）。
2. **per-platform VSIX**（§2.3）：每个平台打一个含对应二进制的 VSIX，marketplace 自动挑。优点零网络依赖；缺点 CI 要交叉编译 5+ 平台。
3. **让用户自己编译**：仅适合本仓库现状（`cargo build -p sokonanoda-lsp` + `SOKONANODA_LSP_BIN`），作为开发模式保留。

对我们的建议：**开发期保持现状（target/debug + env 覆盖）；正式发布走流派 2**，把 `target/{triple}/release/sokonanoda-lsp` 作为 `sokonanoda-lsp[-triple]` 放进对应 VSIX，运行时用 `process.platform + process.arch` 选；环境变量/设置覆盖永远保留（对应 rust-analyzer 的 `server.path`）。校验清单：发布时生成 `checksums.sha256` 并在客户端下载路径（若采用流派 1）逐字节校验。

---

## 2. 打包与市场（vsce）

### 2.1 基本流程与 pre-publish 检查

[官方 Publishing Extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)：

- `npm i -g @vscode/vsce` → `vsce package`（产出 `.vsix`）→ `vsce publish`（需要 publisher PAT）。
- vsce 自动执行的检查：`package.json` 图标/badge 不能是 SVG；README/CHANGELOG 里的图片必须 https；**keywords ≤ 30**；`engines.vscode` 必须合理。
- `scripts.vscode:prepublish` 在每次打包前运行（通常做编译/打包）。
- 从 Windows 打包会丢失 POSIX 可执行位——**含原生二进制的扩展必须从 Linux/macOS 打包**（文档明确警告）。我们 CI 上用 GitHub Actions matrix 解决。

### 2.2 node_modules 膨胀与 esbuild

[Bundling Extensions](https://code.visualstudio.com/api/working-with-extensions/bundling-extension)：

- vsce 会把所有 **dependencies** 打进 vsix，devDependencies 自动排除（[vscode-vsce#931](https://github.com/microsoft/vscode-vsce/issues/931)，joaomoreno 确认）。真实案例：一个 5KB 的扩展因为 node_modules 里有 typescript 膨胀到 20MB。
- 推荐做法：esbuild 打包成单文件（`--external:vscode --format=cjs --platform=node`），然后 **`.vscodeignore` 排除 `node_modules`、`out/`（中间产物）、`src/`**。官方样例的 `.vscodeignore` 就长这样。注意：即便全打包了，`.vscodeignore` 仍要写（[microsoft/vscode#136831 / #147551](https://github.com/microsoft/vscode/issues/136831) 踩坑记录）。
- vsce 会警告 "you should bundle your extension"——以文件数/JS 文件数为准，40+ 文件就该 bundle。
- 我们当前只有 `vscode-languageclient` 一个 dependency（它是 runtime 依赖，**必须**打进 bundle），可以直接 esbuild 解决。

### 2.3 平台专属扩展（native binary 的正路）

同页 [Platform-specific extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension) 章节：

- `vsce package --target <plat>` / `vsce publish --target win32-x64 win32-arm64 linux-x64 linux-arm64 darwin-x64 darwin-arm64`（vsce ≥ 1.99）。
- 不带 target 的包作为 fallback；**若某平台没有对应包，该平台用户看到的是"disabled"且装不上**——不支持的 `alpine-*` 平台要么补要么接受。
- 官方 [vscode-platform-specific-sample](https://github.com/microsoft/vscode-platform-specific-sample) 提供 CI 模板（matrix 逐平台构建+发布）。
- 参考物：rust-analyzer 按 release 附 per-platform VSIX，手动 `code --install-extension r-a.vsix`（[文档](https://rust-analyzer.github.io/book/vs_code.html)）。

### 2.4 extensionKind：ui vs workspace

[Supporting Remote Development](https://code.visualstudio.com/api/advanced-topics/remote-extensions)：

- **UI 扩展**跑在本地；**workspace 扩展**跑在工作区所在机器（remote 时在远程 host）。
- 我们要 spawn 本地二进制、读写工作区文件 → **workspace kind** 是正确语义；不声明时 VS Code 会自动推断（含 main/进程逻辑的扩展默认 workspace）。显式写 `"extensionKind": ["workspace"]` 避免 WSL/容器场景下被误装到 UI 端导致找不到二进制。
- 反面参照：纯 UI 的扩展若被误判为 workspace，远端没装就不能用（[实例讨论](https://github.com/shanalikhan/code-settings-sync/issues/870)）。可用 `Developer: Show Running Extensions` 验证运行位置。

---

## 3. 证明助手扩展的 UX 结构（对 F5 goal 视图与进度树的启示）

### 3.1 Lean 4（leanprover/vscode-lean4）——同形态的头号参照

- **架构**：扩展宿主（Node）+ 沙箱 webview 两个进程；InfoView 是 React 应用（[lean4-infoview](https://github.com/leanprover/vscode-lean4/blob/master/lean4-infoview/README.md)），通过 `EditorApi`/`InfoviewApi` 双向接口与编辑器通信（[infoviewApi.ts](https://github.com/leanprover/vscode-lean4/blob/master/lean4-infoview-api/src/infoviewApi.ts)、[DeepWiki 架构页](https://deepwiki.com/leanprover/vscode-lean4/4-infoview-system)）。
- **"光标处 goal"怎么拿**：Lean 核心开发者 gebner 的第一手说明（[LSP issue #1414](https://github.com/microsoft/language-server-protocol/issues/1414)）：
  1. `$/lean/fileProgress` **自定义通知**：携带"文件哪些区域还在处理中"，编辑器渲染成行侧橙色条（进度指示）；
  2. **自定义 RPC**（`$/lean/rpc/*` 通道）传 goal state，因为还要支持悬停子项展示隐式参数等富交互。
  - 结论：goal 视图不走 LSP 标准能力，就是**服务器自定义请求 + 客户端 webview**。我们做 `sokonanoda/goal` 自定义请求即可，无需发明协议。
- **InfoView 交互词汇**（对齐 F5）：pin（把某个位置的 tactic state 钉住，光标移开仍追踪）、pause/continue（暂停实时更新）、copy to comment、按"与光标距离"排序消息。这些是迭代多年沉淀的**教学场景刚需**——我们的 goal 面板第一版至少要有"跟随光标"和"暂停更新"两个开关（[vscode-lean4 手册](https://xubaiw.github.io/reservoir-index/leanprover.vscode-lean4.html)）。
- lean.nvim 提供了同样的面板语义：`:LeanGoal` 弹窗显示光标处 goal、`:LeanTermGoal` 显示 term 类型——正好对应我们 hover 的"表达式类型"与"练习目标"两层（[lean.nvim 手册](https://github.com/Julian/lean.nvim/wiki/The-lean.nvim-Manual)）。

### 3.2 Agda 两家——另一条路线

- **agda-mode-vscode**（banacorn）：交互目标（`{! !}` 洞）+ 语义高亮 + 面板（[DeepWiki](https://deepwiki.com/banacorn/agda-mode-vscode/7.4-interactive-goals)）；它现在也在实验 [agda-language-server](https://github.com/banacorn/agda-mode-vscode)（LSP 化）。
- **agda2-vscode**（新）：直接走 Agda 自带 `--interaction-json`，支持 load/give/refine/case split 等，**info 面板常驻显示 goal 类型、上下文、错误**；未解 meta 用背景 decoration 高亮；goal 位置用 label 装饰（`?0`）（[README](https://github.com/willtunnels/agda2-vscode)）。
- 教训：goal 演进类动作（give/refine/intro）天然是"**光标处上下文 + 一次服务器往返 + 一段 TextEdit**"，用 code action 或命令都可以承载——我们已经用 code action 做了 `intro`（`crates/lsp/src/main.rs:305`），后续 `exact/apply` 照抄即可。

### 3.3 行业共识

[LSP issue #1414（Support proof assistants）](https://github.com/microsoft/language-server-protocol/issues/1414)记录了全行业现状：Coq 用私有 XML 协议导致生态损失；Isabelle 自带 PIDE 协议（decoration 承载过程状态）；Lean/Agda 用 LSP + 自定义请求活得最好。另有一篇标准化提案 [The Specification Language Server Protocol](https://arxiv.org/abs/2108.02961) 值得在 I8 设计时翻一遍（其 `processedUpto` 想法对我们"逐声明检查"有借鉴）。

---

## 4. 标准 UX 面面观：练习状态/目标该用什么承载

依据 [UX Guidelines 总览](https://code.visualstudio.com/api/ux-guidelines/overview)与各功能指南：

| 表面 | 适合我们什么 | 时机 | 依据 |
|---|---|---|---|
| **CodeLens** | 每个练习声明头上的 `open / solved ✓ / failed` 状态 + 点击跳转/查看 goal | 已实现（服务器 codeLens provider）；短期最优性价比 | 已有 |
| **Diagnostics** | 三阶段错误 + hint | 已实现（F1） | 已有 |
| **Hover** | 表达式类型 / `???` 处目标类型 | 已实现（F2/F5 雏形） | 已有 |
| **Tree View（进度树）** | 练习列表 + 状态图标 + 点击定位；welcome content 引导 | **下一里程碑做**；放 Explorer 或自建 view container | [Tree View API](https://code.visualstudio.com/api/extension-guides/tree-view)：`contributes.views` + `TreeDataProvider.getChildren/getTreeItem`；数据可由自定义请求 `sokonanoda/status` 拉取 + 诊断事件驱动刷新；<1.74 需要 `onView:`，我们 engines ≥1.85 不必显式声明但写了无害 |
| **Webview Panel** | goal 视图（目标 + 假设 + 可点击 tactic 建议） | I8；Lean InfoView 即此形态 | [lean4-infoview](https://github.com/leanprover/vscode-lean4/blob/master/lean4-infoview/README.md) |
| **Inlay Hint** | 化简结果、期望类型小标注（F4） | 后置；记得用户可全局关掉 `editor.inlayHints.enabled`，别把关键状态只放这里 | [rust-analyzer 的 inlay 实践](https://code.visualstudio.com/docs/languages/rust) |
| **Decoration** | `???` 洞高亮、failed 声明行侧标 | 小成本补充；Agda 用它标 unsolved meta | [agda2-vscode](https://github.com/willtunnels/agda2-vscode) |
| **Quick Fix（code action）** | `intro` 等"搭 lambda"动作 | 已实现；后续 exact/apply 同构 | 已有 |
| **Snippet** | 常用声明骨架（`theorem x : T := ???`） | 顺手加 `contributes.snippets` | 低优先 |

原则（来自 guidelines 的普遍精神）：**状态类信息首选只读表面（lens/tree/hover），编辑类动作用 code action/command**；webview 只在富交互（可点击假设、折叠假设组）时才值得引入。

---

## 5. 自定义请求：LSP 之上注册私有命令

两条路线，用途不同：

1. **`workspace/executeCommand`（LSP 标准命令通道）**：服务器在 `ServerCapabilities` 声明 `executeCommandProvider`，code lens/code action 返回的 `Command.command` 名由**服务器**处理，客户端库自动把它转发成 `workspace/executeCommand` 请求。注意 VS Code 的行为：**若命令名恰好与 VS Code 端注册的命令重名，VS Code 会直接调用本地命令、跳过 executeCommand**（[atom-languageclient#183 讨论](https://github.com/atom/atom-languageclient/issues/183)，henryju 指出该行为偏离规范但很方便）——所以**给服务器命令起带前缀的名字**（`sokonanoda.server.*`）避免撞名；也可以用 `middleware.executeCommand` 在客户端拦截。
2. **完全自定义方法**：LSP 方法名只要不与标准冲突即可私有扩展（[SO: how to send a message from the server](https://stackoverflow.com/questions/51041337/vscode-language-client-extension-how-to-send-a-message-from-the-server-to-the)）：
   - 请求（有响应）：客户端 `client.sendRequest('sokonanoda/status', params)`；tower-lsp 端在 `Backend` 上监听（tower-lsp 允许对未注册 trait 方法的自定义方法走 `on_request`/`on_notification` 底层注册，或干脆放进 `LanguageServer` 实现里作为私有方法注册到 `LspService`——社区版 [tower-lsp-server](https://github.com/tower-lsp-community/tower-lsp-server) 提供了 `on_request` 一等支持）。
   - 通知（无响应）：`$/lean/fileProgress` 就是这类；我们的 `exercise.open/solved/failed` 事件（F3/I9）未来也可走 `sokonanoda/exerciseStatus` 通知推给客户端，进度树收到即 `fire(onDidChangeTreeData)`。
   - 命名约定：建议全部带 `sokonanoda/` 前缀；Lean 用 `$/lean/...`，`$/` 前缀按规范表示"实现相关、可忽略"，适合纯客户端私用消息。

对 vscode-languageclient ≥8：handler 可在 `client.start()` 之前注册（§1.2），不会再出现老版"必须先 onReady"的坑。

---

## 6. 多根工作区、信任模式与语言配置

### 6.1 多根 & 远程

- 服务器端不要用 `InitializeParams.rootPath/rootURI`，用 `workspaceFolders`（[Adopting Multi Root Workspace APIs](https://github.com/microsoft/vscode-wiki/blob/main/Adopting-Multi-Root-Workspace-APIs.md)）。我们的服务器目前是单文档内存模型，影响小，但自定义请求参数里应带 `textDocument.uri` 而不是依赖"当前根"。
- `sokonanoda.serverPath` 等**含可执行路径的设置必须按 resource scope 或 restrictedConfigurations 处理**：VS Code 内置 PHP 扩展把 `php.validate.executablePath` 限制为仅受信工作区可用（[Workspace Trust 指南](https://code.visualstudio.com/api/extension-guides/workspace-trust)）。
- Trust 声明：我们只是"读文件 + 跑一个随扩展分发的二进制"，可声明 `capabilities.untrustedWorkspaces: { supported: 'limited', description: '…', restrictedConfigurations: ['sokonanoda.serverPath'] }`——受限模式下不用工作区设置里的服务器路径，仅用用户级/扩展内置二进制。命令仍要在代码里挡住（`workspace.isTrusted` / `onDidGrantWorkspaceTrust`），因为 command 即使 UI 隐藏也可能被调。
- `documentSelector` 里 `scheme: 'file'` 目前排除了 untitled/virtual 文档；要支持"无标题新文件先写再存"或 remote/虚拟工作区，需放宽为 `['file', 'untitled']` 并在服务器端容忍无文件路径的 URI（多根同一主题）。

### 6.2 language-configuration 最佳实践

[Language Configuration Guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide)（文件名以 `language-configuration.json` 结尾可在编辑器里获得补全和校验）：

- `comments.lineComment: "--"`（对齐 Lean/Agda）；如无块注释就只写 line。
- `brackets`/`autoClosingPairs`/`surroundingPairs`：三对基础括号 + `notIn: ["string","comment"]` 控制引号类；`autoCloseBefore` 覆盖紧贴标点的场景。
- `folding`：缩进折叠是默认；若声明级折叠体验重要，长期在服务器实现 `textDocument/foldingRange`（声明 span 现成）。
- `wordPattern`：我们的标识符是字母数字下划线，默认规则基本可用，但 `?`（`???`）不属于 word，无碍。
- `indentationRules`：教学文件层级浅，`onEnterRules` 保持 `=`/`:` 后缩进即可，先从简。

---

## 7. 本地调试 client + server

[Language Server Extension Guide（调试小节）](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) 与 [debug configuration 文档](https://code.visualstudio.com/docs/debugtest/debugging-configuration)：

- **客户端**：`launch.json` 里 `type: "extensionHost"` 的 "Launch Client"（`--extensionDevelopmentPath=${workspaceFolder}`）跑扩展宿主，普通断点调试。
- **服务器（Rust）**：两条路：
  1. 直接在 `crates/lsp` 里 `rust-lldb`/`rust-gdb` attach 到正在跑的 `sokonanoda-lsp` 进程（Node 服务器时代官方是 "Attach to Server" attach 到 `--inspect=6009`；对 Rust 二进制就是 lldb attach pid）；注意当前 lsp-sample 模板已删掉 attach 配置（[SO 记录](https://stackoverflow.com/questions/77346575/vs-code-language-server-extension-guide-lacks-attach-launch-configuration)），需要自己写。
  2. 单独终端手动跑 `SOKONANODA_LSP_BIN=... RUST_LOG=debug sokonanoda-lsp` 不可行（stdio 被 LSP 占用），日志一律走 stderr。
- **协议日志**：设置 `"sokonanoda.trace.server": "verbose"`，客户端把全部 JSON-RPC 流量打进名为 "sokonanoda" 的输出通道（languageclient 内建，名称取自 LanguageClient 的 name 参数）。
- **compound**：`compounds` 可同时启动多个配置并 `stopAll`；对我们更有用的是 `preLaunchTask` 里先 `cargo build -p sokonanoda-lsp` 再启动 Extension Development Host。
- 服务器崩溃排查：`revealOutputChannelOn: RevealOutputChannelOn.Error` + fileEvents 同步检查。

---

## 附录：推荐的 extension.js 骨架（LanguageClient + 自定义命令 + TreeView）

> 目标形态：保留现有薄壳风格（无 TS/无打包也能跑），新增 goal/进度两条自定义请求通路。
> 服务器侧需要新增：`sokonanoda/status`（→ `DocumentReport.decls` 的可序列化投影）、`sokonanoda/goal`（→ 光标处 goal/假设，I8 后可用）。

```js
// extension.js
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const {
  LanguageClient, TransportKind, RequestType,
} = require("vscode-languageclient/node");

let client;

// ---- 自定义请求类型（与服务器约定，均为 $ 前缀外私有方法） ----
const StatusRequest = new RequestType("sokonanoda/status");
const GoalRequest = new RequestType("sokonanoda/goal");

function resolveServerBin() {
  // 1. 设置 2. env 3. 扩展自带 4. 仓库 target/debug（开发模式）
  const cfg = vscode.workspace.getConfiguration("sokonanoda");
  const fromCfg = cfg.get("serverPath");
  if (fromCfg) return fromCfg;
  if (process.env.SOKONANODA_LSP_BIN) return process.env.SOKONANODA_LSP_BIN;
  const bundled = path.join(__dirname, "bin",
    process.platform === "win32" ? "sokonanoda-lsp.exe" : "sokonanoda-lsp");
  if (fs.existsSync(bundled)) return bundled;
  return path.join(__dirname, "..", "..", "..", "target", "debug", "sokonanoda-lsp");
}

async function activate(context) {
  const command = resolveServerBin();
  if (!fs.existsSync(command)) {
    await vscode.window.showWarningMessage(
      "sokonanoda-lsp 未找到：设置 sokonanoda.serverPath 或 cargo build -p sokonanoda-lsp。");
    return;
  }
  const serverOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };
  client = new LanguageClient("sokonanoda", "sokonanoda", serverOptions, {
    documentSelector: [{ language: "sokonanoda", scheme: "file" }],
    synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher("**/*.sokonanoda") },
    revealOutputChannelOn: 2 /* Error */,
  });

  // handler 可在 start 前注册（>=8.0）
  client.onRequest(StatusRequest.method, async (p) =>
    client.sendRequest(StatusRequest, p)); // 直通服务器（示例占位）

  // ---- VS Code 命令：code lens 点击 / 状态弹窗 ----
  context.subscriptions.push(vscode.commands.registerCommand(
    "sokonanoda.status", async (decl) => {
      if (!decl) return;
      vscode.window.showInformationMessage(`${decl.name}: ${decl.status}`);
    }));

  // ---- 练习进度树（F3 的 UI） ----
  class ProgressProvider {
    constructor(onData) { this._onData = onData; }
    get onDidChangeTreeData() { return this._onData; }
    async getChildren() {
      const doc = vscode.window.activeTextEditor?.document;
      if (!doc || doc.languageId !== "sokonanoda") return [];
      try {
        const report = await client.sendRequest(StatusRequest, { uri: doc.uri.toString() });
        return report.decls.map((d) => ({
          label: d.name ?? `${d.kind}@${d.line}`,
          contextValue: d.status,                  // open / checked / failed
          iconPath: iconFor(d.status),
          command: { command: "sokonanoda.revealDecl",
                     title: "reveal", arguments: [doc.uri, d.range] },
        }));
      } catch { return []; }
    }
  }
  const emitter = new vscode.EventEmitter();
  const tree = vscode.window.createTreeView("sokonanodaProgress", {
    treeDataProvider: new ProgressProvider(emitter.event),
  });
  // 诊断刷新时拉一次状态（服务器 publishDiagnostics 之后状态即最新）
  client.onDidChangeDiagnostic(() => emitter.fire());
  context.subscriptions.push(tree, emitter);

  await client.start();
}

function iconFor(status) {
  const t = { checked: "check", open: "circle-outline", failed: "error" }[status];
  return { light: undefined, dark: undefined, id: t }; // ThemeIcon via { id }
}

async function deactivate() { await client?.stop(); }
module.exports = { activate, deactivate };
```

配套 `package.json` 片段（增量）：

```json
"contributes": {
  "views": {
    "explorer": [{ "id": "sokonanodaProgress", "name": "练习进度" }]
  },
  "configuration": {
    "title": "sokonanoda",
    "properties": {
      "sokonanoda.serverPath": { "type": "string", "default": "",
        "description": "sokonanoda-lsp 可执行文件路径（留空用内置/开发路径）" },
      "sokonanoda.trace.server": { "type": "string", "enum": ["off", "messages", "verbose"],
        "default": "verbose", "description": "LSP 流量日志" }
    }
  },
  "keybindings": [
    { "command": "sokonanoda.showGoal", "key": "ctrl+shift+enter",
      "when": "editorLangId == sokonanoda" }
  ]
}
```

注：`ctrl+shift+enter` 是 Lean 4 `displayGoal` 的默认键位，教学用户跨工具迁移零成本；`sokonanoda.showGoal`（webview 面板）在 I8 落地后接入 `GoalRequest`。
