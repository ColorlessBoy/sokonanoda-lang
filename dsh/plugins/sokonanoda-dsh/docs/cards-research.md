# DSH 聊天页自定义渲染 · 现状调研与可行方案

> 日期：2026-11-23 · 调研者：lead · 状态：**只读调研，未改任何代码**
> 问题：DSH 升级后，聊天页有没有「保持上下文文本不变、把内容渲染成卡片」的官方口子？
> 一句话答案：**有。`conversation.chat.markdown.fence`（按围栏语言分键的 hoc 插槽）就是这条口子；官方的 action-cards 已经在用同一条口子做「答案里的卡片」。**

---

## 0. 结论（TL;DR）

1. **官方已内置「卡片」范式**：模型在最终回答末尾写一个 ``` `action-card` ``` 围栏块（一行 JSON），客户端把这个块**隐藏**，在该 Turn 的末尾渲染一张真卡片；用户点卡片动作 → Host 服务记录决策（写进会话事件）→ 下一轮以状态通知告诉模型。实现在 `@deepseek-ai/dsh-action-cards`（Host）+ `@deepseek-ai/dsh-client-ui-action-cards`（Client），Web 组合默认挂载。
2. 它下面那条通用口子是 **`conversation.chat.markdown.fence`：`hoc` 类型、`session` 作用域、按「围栏语言」分键**。任何客户端插件都能为自己选的语言注册一层，把代码块**替换**成任意 React 卡片（或**装饰**它）。**会话文本、日志、模型上下文原样不动**，卡片只是客户端对同一段 Markdown 的解释。
3. 现有 `learn_card` 走的是另一条口子 **`tool.call.toolview`**（按 wire tool 名分键）。它能渲染卡片，但上下文里多出 tool call / tool result，不是「文本不变」的形态；代理侧还要处理 tool-result 回环（README 已记录这些复杂度）。
4. 交互卡（点击作答、状态回写）有两条路：
   - **官方 action-cards kind 框架**：卡片挂在 Turn 末尾、每 Turn 一张、动作走 Host Remote、决策进会话并在下一轮上报模型；要 Host+Client 两半 + skill。
   - **继续用 toolview + 自有 Host 路由**：whiteboard 现在就是这条（卡片内答题、本地判分、原地出下一题，无需模型 turn）。
5. **最小的目标路径**：给 `dsh-learn-card` 的 `client.js` 增加一个 fence 层（语言如 `learn-card`），复用现有三种卡片的渲染器；Host 不用改、纯客户端、HMR 即改即见。代理侧还可以直接从「发 tool call」改成「发带围栏块的 assistant 文本」，省掉 tool-result 回环。

---

## 1. 环境与证据基线

| 事实 | 值 / 证据 |
|---|---|
| 运行版本 | DSH **0.2.3-alpha.2**（`cordis_inspect` → Runtime.getVersion；`<INSTALL>/package.json`） |
| 安装根 | `<DSH_INSTALL>`（下称 `<INSTALL>`） |
| Profile | `superagent`，`<DSH_HOME>/profiles/superagent`；Web GUI `http://127.0.0.1:3082` |
| 已装本地插件 | `@local/dsh-learn-card`、`@local/dsh-quiz-keeper`、`@local/dsh-whiteboard`（都在 `sokonanoda-lang/.cache/`，gitignored）；`dsh-superagent`（`<DSH_SUPERAGENT_REPO>`） |
| 第三方 bundle | `@linxin666/dsh-remote-bridge`（提供 `ui_present` 工具 + toolview 渲染）、`@linxin666/dsh-remote-web-ui` |
| 官方 action-cards 已挂载 | `<INSTALL>/node_modules/@deepseek-ai/dsh-web-app/cordis.patch.yml:416-420`（`action-cards` + `client-ui-action-cards`） |
| 官方动态包 runner 已挂载 | 同文件 `:172-179`（host runner + inspect providers）、`:269-270`（client runner）、`:397-398`（ui-cordis） |

当前会话的实际工具目录里已有 `learn_card`（本地插件）与 `ui_present`（remote-bridge），说明 profile 的两条「卡」链路都在工作。

---

## 2. 聊天页架构（从会话日志到屏幕）

```
Host：Session 日志（durable events，唯一真相）
  │  API Gateway：follow / page（断线重连 = 基线替换 + 增量）
  ▼
Client：SessionEventLikeEntry 窗口（api/session-controller/client）
  │
  ├─ ui-conversation：事件定义 → Context → target snapshot
  │     · 每个 Definition 按稳定 (kind,id) 折叠状态
  │     · target = chat / trajectory / 第三方注册的 view
  ▼
Conversation shell：conversation.view [list，选中一个 target View]
  ├─ id=chat        → ChatView
  ├─ id=trajectory  → TrajectoryView
  └─ 第三方注册的 view（Conversation View 标签页）
  │
  ▼
ChatView
  ├─ conversation.chat.flow [single]
  │    └─ conversation.chat.node [keyed by ChatNodeKind]
  │         ├─ key=user           → 用户消息（conversation.chat.user.content Factory）
  │         ├─ key=assistant-step → Markdown 渲染（MarkdownDelegateProvider.renderCode）
  │         │     └─ conversation.chat.markdown.fence [hoc，按围栏语言分键]  ★
  │         │           ├─ key=math / diff / patch（ui-chat 自带）
  │         │           └─ key=action-card（官方 action-cards 用来「隐藏原块」）
  │         ├─ key=tool-call      → ToolCallTree
  │         │     └─ tool.call.toolview [keyed by wire tool name，开放键域]  ★
  │         │           ├─ key=learn_card（本地 learn-card 插件）
  │         │           ├─ key=ui_present（remote-bridge）
  │         │           └─ key=self     → tool.view.cordis（动态包自渲染区）
  │         ├─ key=turn-tail      → TurnTailNodeView
  │         │     ├─ conversation.chat.turnTail [list]  ★（官方卡片挂在 Turn 末尾）
  │         │     └─ conversation.chat.assistant-actions [list]
  │         └─ key=command / …    → 其他内置节点（commandview 也按命令名分键）
  ├─ conversation.chat.aside [single]      ← 侧栏面板（Session panel）同类位置
  ├─ conversation.composer [chain] / conversation.composer.bar 的各个输入位
  └─ MarkdownDelegateProvider：fence → renderCode(fence) → renderSlot('conversation.chat.markdown.fence', fence, { entryKey: lang, children: 默认代码块 })
```

关键点：

- **会话日志是唯一真相**。卡片的可重放性取决于它的数据能否从日志重建。fence 方案天然满足：卡片数据就是 assistant 消息文本的一部分，fork / 翻历史 / 导出全部免费。
- **客户端插件**通过 `package.json` 的 `dsh.client` 声明（`platform: web`、可选 `immediately`/`inject`），由 Host 组合成 boot graph；浏览器侧是预构建的普通 JS bundle（`window.__ModuleLoader__.load({id, factory(require){…}})`），`apply(ctx)` 里注册槽位/工厂/事件。`client.js` 改动由 `dsh-client-hmr` 轮询热替换，**不用重装、不用重启**；Host 半的 `index.js` 改动需要重启 Host。
- 组件拿不到 `ctx`：owner props + 标准 hooks（`useSession`/`useProjection`/`useChat`…）+ 注册时 `inject` 注入的回调/observable。
- `cordis_inspect` 的 Client `Slots` provider 是**活体**槽位树（`slots.snapshot(root)`），但槽位是**按需物化**的：某个 fence 槽位要等页面上真的渲染过一个围栏块才会出现——之前对它 `listSubTree` 得到 `available:false` 属正常，不是插件没装。

---

## 3. 口子总表

| # | 口子 | 类型 / 键 | 能渲染什么 | 对上下文文本的影响 | 证据 |
|---|---|---|---|---|---|
| A | `conversation.chat.markdown.fence` | **hoc** / 围栏语言（`entryKey=lang`） | 把任意 ```` ```lang ```` 块**替换**为 React 卡片，或**装饰**（渲染 `children` + 额外内容） | **不变**：文本原样在会话/上下文里，卡片只是渲染解释 | `subsystems/slots.md:75-83,164`；`ui-chat/src/client/contract/slots.ts:433`；`ui-chat/src/client/chat/ChatView.tsx:116-124`；`ui-chat/src/client/fence/register.ts` |
| B | `conversation.chat.turnTail` | list / 自己给 `id`、`order` | 在「已完成 Turn 的最终回答」之后加一张/多张卡片 | **不变**（数据来自你自己的投影或对文本的解析） | `ui-chat/src/client/contract/slots.ts:506`；官方 action-cards 用法 `ui-action-cards/src/client/mount.ts:42`、`Card.tsx:34,82` |
| C | 官方 **action-cards** 框架 | Host service + Client body registry | 官方「答案里的卡片」：围栏块 → 隐藏 → Turn 末尾卡片；按钮动作 → Host kind handler → 决策写进会话 → 下轮状态通知模型 | **原文不变**；决策在后续轮次以 state notice 追加 | `dsh-action-cards/README.md`（卡片规则）；`ui-action-cards/README.md`；升级指南 `upgrade-guide/v0.2.2-rc.3/action-card-blocks/` |
| D | `tool.call.toolview` | keyed / wire tool 名（**开放键域**） | 把工具调用行替换成任意 React 卡片（现有 `learn_card`、whiteboard、`ui_present` 都走这条） | **改变**：上下文多 tool call / tool result | `ui-tool/README.md:38-50`；活体槽位 `keyDomain: open … already taken: …` |
| E | `conversation.chat.node` + `ctx.uiConversation.events.register()` | keyed / `ChatNodeKind`（固定键域） | 自定义 Chat 行（需要事件定义）；复用一个 key 可**替换**内置渲染器 | 取决于数据来源；第三方**不要**新增 durable 事件类型 | `subsystems/conversation.md`；`practices.md`UI-21；活体槽位 `keyDomain: fixed by ChatNodeKind` |
| F | `conversation.view` | list / 自己的 View 定义 | 整个会话的**另一种视图**（Conversation View 标签页），可读同一份 session 投影 | **不变**（纯读） | `subsystems/conversation.md`；`ui-conversation/README.md:53`；活体槽位 `conversation.view [list]` |
| G | 周边槽位 | `conversation.composer`（chain）、`conversation.composer.dock`/`conversation.input.dock`（list）、`conversation.chat.aside`（single）、`shell.overlay`、`main`（keyed） | 输入条/侧栏/浮层/独立主面板里的交互组件 | 不变或按你的实现 | `subsystems/slots.md:140-230` |
| H | 动态 Cordis 包 | 程序化 `define/run/stop/undefine`（**无模型工具**） | 运行期写入的 Host+Client 包；Client 半可注册槽位（含 `tool.view.cordis` key=self） | 取决于包（来源是工具调用/外部脚本） | `cordis-client-runner/README.md`、`cordis-host-runner/README.md`、`tool-cordis/README.md` |
| I | `ui_present`（第三方 remote-bridge） | 工具 + `tool.call.toolview` key=`ui_present` | 一张整页 HTML 卡（`srcdoc` + `allow-scripts`，无 same-origin），首个动作作为工具结果返回 | **改变**：整段 HTML 进上下文（token 大） | `remote-bridge/README.md` §Interactive cards（约 :299-308） |

★ = 与「文本不变、渲染卡片」直接相关的两条。

---

## 4. 重点口子详解

### A. `conversation.chat.markdown.fence`（核心答案）

**契约**（`ui-chat/src/client/contract/slots.ts:433`）：

```ts
'conversation.chat.markdown.fence': { kind: 'hoc'; scope: 'session'; owner: MarkdownDelegateFenceInfo }
```

**owner props**（`MarkdownDelegateFenceInfo`，见 `dsh-cordis-client-runner/src/client/slot-catalog.ts:453`）：

| 字段 | 含义 |
|---|---|
| `lang` | info string 的语言词：在第一个 `[A-Za-z0-9_-]` 之外的字符处截断，再小写（`c++` → `c`） |
| `code` | 围栏正文（去掉结尾换行） |
| `streaming` | 外层文档是否仍在流式输出（`code` 可能还会变） |
| `meta` | info string 中语言词之后的原文（可当参数用） |

**注册方式**（官方 math/diff 同款，`ui-chat/src/client/fence/register.ts`）：

```js
ctx.slots.inject('conversation.chat.markdown.fence', () => ctx.slots.register({
  name: 'conversation.chat.markdown.fence',
  key: 'learn-card',                       // 小写语言词，必须是 [a-z0-9_-] 组成
  id: '@local/dsh-learn-card/fence',       // hoc 层必填 id；新 id = 新增层，不抢别人的键
}, FenceCardView))
```

**渲染语义**（`subsystems/slots.md:75-83`）：

- hoc 层把 owner 的内容（默认代码块）作为 `children` 传进来；**渲染 `children` = 装饰**，**不渲染 `children` = 完全替换**。
- 每层 `(key,id)` 是一个 shadowing cell；同 `id` 的精确 key 层会取代 wildcard 层。
- 层崩溃 → 只在该次渲染回退到 `children`（即显示普通代码块），其他围栏不受影响；挂载/卸载层会重挂内容。

**官方先例**：

- `ui-chat` 自带 `math`（流式时显示源码、结算后显示公式）、`diff`/`patch`（替换成 diff 行）——`FenceLayers.tsx`。
- `ui-action-cards` 注册 `action-card` 层且**永远返回 `null`**（隐藏 JSON）——`ui-action-cards/src/client/mount.ts:54-56`、`Card.tsx` 的 `HiddenCardBlock`。
- 它的注释直接写明：这些块在 answers、reasoning、compaction summaries 里都会被隐藏——即 **Chat flow 所有 Markdown 渲染点都会走这个槽**。

**为什么它是「文本不变」**：

- 卡片数据来自 assistant 消息文本里的围栏块；这条消息照原样进会话日志、进模型上下文、被 fork/导出/搜索。
- 不新增事件类型、不需要 Host 投影、不需要 tool call；客户端只做解释。
- 历史消息重放时，fence 层同样生效（因为它只看文本）。

**限制 / 注意**：

- 语言词文法：`[A-Za-z0-9_-]`，小写；`learn-card` / `soko-card` 合法；`card+json` 会被截成 `card`。
- 只有**带语言词且有内容**的围栏才走该槽（`ui-chat/README.md`：`renderCode` 只在有 language 和 content 时调用）。
- 只覆盖 **Chat view** 的 Markdown；Trajectory 视图、用户消息、纯文本行不走这里。用户消息有另一套 `conversation.user.text`（JSON 文本节点）。
- 流式期间 `code` 不完整：应当 `streaming ? children : 卡片`（math 层就是这么做的），或显示骨架。
- 解析失败必须「fail open」：渲染 `children`（显示原代码块），绝不空白/抛错。
- 卡片内容仍是上下文里的 token（围栏块 JSON 的大小）；比 tool call 省的是「没有 tool 行/tool result/回环」。

### B. `conversation.chat.turnTail`（卡片挂 Turn 末尾）

- `{ kind: 'list'; scope: 'session'; owner: TurnTailOwnerProps }`（`contract/slots.ts:506`）。
- 官方 ActionCardTail 的 props：`turn`、`useProjection` + `inject` 注入的 session actions（`Card.tsx:34`、`mount.ts:42-53`）。
- 卡片固定出现在「Turn 结束且最终回答存在」之后；适合「先读回答，再看/操作卡片」的节奏，不打断行内文本。
- 数据来源要自己解决：官方用 Host `actionCards` 投影；自研可以注册自己的 session projection（`ctx.sessionProjections`）或从 Chat 快照解析。

### C. 官方 action-cards 框架（交互卡的标准姿势）

- Host `@deepseek-ai/dsh-action-cards`：
  - 从 `assistant/message` 的文本块里按 CommonMark 围栏规则读 `action-card` 块（`src/fence.ts`），一个 Turn 只认**第一个**合法块；标题/简介为空、在 reasoning 里、在子 agent 会话里、被 fork 继承的块都不算。
  - 块格式：一行 JSON `{ "action", "title", "brief", "details" }`；围栏缩进最多 3 空格，可在列表项里。
  - 决策写成 `action-cards/decision` 事件；下一轮第一步用状态通知把决策告诉模型。
  - `enabled` 开关（默认 true）控制 kind 的 skill 是否进目录；已加载 skill 的会话仍能写块。
- Client `@deepseek-ai/dsh-client-ui-action-cards`：
  - `conversation.chat.markdown.fence` key=`action-card` → 隐藏原块；
  - `conversation.chat.turnTail` → 渲染卡片外框（标题、可折叠 brief）+ 注册的 body；
  - `ctx.actionCards.register({ action, locale, Body })` 注册某 action 的卡片体；动作通过 `ctx.remote.actionCards.act(sessionId, cardId, verb, payload)` 走 Host。
- 扩展一个 kind 需要：Host 插件 `ctx.actionCards.register(kind)`（action、zod schema、verbs、权限检查、handler、reconcile）+ 通过 `ctx.actionCards.skillProvider()` 注册 skill（教模型何时写块）；Client 插件注册 body。
- **适合**：自然语言回答 + 可选动作卡（"要不要建个页面/继续下一轮"）；不适合每 Turn 多张卡或高频刷新的练习卡。

### D. `tool.call.toolview`（现状）

- 按 **wire tool 名**分键，开放键域（活体槽位里 `keyDomain: open: any string the owner dispatches`，已占用列表也随插件加载变化）。
- 卡片数据从 `argsRaw` / 结果 `content` / `meta` 读；学习卡现状即此（`learn-card/README.md` 的 "Where the Client reads the payload"）。
- 上下文影响：tool call（含卡片 JSON）+ tool result（短 render 行）都进日志/上下文；如果卡片由代理生成，还要处理 tool-result 回环（learn-card README 的 proxy 改法 4 步）。
- 仍是「host 驱动交互」的正确口子：whiteboard 的点击答题 → 本地路由判分 → 原地下一题，全程不需要模型 turn。

### E. `conversation.chat.node` / 事件定义

- 可以注册新的 Chat 行 kind，或**复用内置 key 替换渲染器**（"Reusing a key replaces that node renderer"，`contract/slots.ts` 注释）。
- 但**第三方插件不要新增 durable 事件类型**：`practices.md` UI 节明确写了 `Session.append()` 不能带 `ignorable`，未知类型会让 Session 拒绝重开；新事件类型只适合 DSH 自己实验包。
- 结论：除非要替换内置行渲染或消费已有事件，否则不如 A/B/D。

### F. `conversation.view`（整个自定义视图）

- `ctx.uiConversation.views.register(...)` 注册一个 target view；它出现在 Conversation View 标签页（官方 "More conversation views" 的入口同一层）。
- 适合做成「学习面板/卡片流」这种**独立视图**（类似以前右侧 学习 tab 的定位），可以读当前会话投影、渲染自己的卡片列表。
- 文本不变，但不在行内；用户要切标签页才看到。

### G. 周边槽位

- `conversation.composer`（chain，替换 composer）、`conversation.composer.dock`（list，输入框下方）、`conversation.input.dock`（list，输入框上方）、`conversation.chat.aside`（single，聊天区侧边，Session panel 同款）、`shell.overlay`（list，全框架浮层）、`main`（keyed，可注册自己的主面板 + `sidebar.panellist` 图标）。
- 适合练习状态条、答题抽屉、侧边白板；不解决「行内文本 → 卡片」。

### H. 动态 Cordis 包（运行期写入 UI）

- Web 组合里 host/client runner 都挂着，但**没有模型工具**创建定义（`cordis-host-runner/README.md`：no model tool creates dynamic definitions；Creator 走 Plugin Manager 装持久 bundle）。
- 程序化调用（SDK/脚本）可以 define 一个带浏览器半的包：浏览器半是普通 async JS，拿到 `React/console/styles/host`，可 `inject` 服务（`slots`、`theme` 等），运行后注册槽位；key=self 的 `tool.view.cordis` 渲染在 `cordis_run` 卡片内部。
- 每次 run 需要页面批准；定义是进程内、会话内；刷新页面丢失。
- 结论：是"agent 运行期写 UI"的路，但不是当前的主通道；做教学卡片不需要。

### I. `ui_present`（第三方，已有）

- 一次一整页 HTML（≤256 KiB），`srcdoc` 沙箱（`allow-scripts`、无 same-origin），浏览器把第一个动作 `{type,payload}` 作为工具结果返回；桌面端只渲染、提示到手机作答（remote-bridge 的定位）。
- 上下文里是整段 HTML（token 成本高），且是 tool 形态；适合一次性表单/图表，不适合反复出现的教学卡。

---

## 5. 「保持上下文文本不变」判定表

| 口子 | 会话文本变化 | 模型上下文新增 | 卡片数据来源 | 重放/ fork |
|---|---|---|---|---|
| A fence 层 | 无 | 无（块本来就在文本里） | assistant 文本里的围栏块 | ✅ 免费 |
| B turnTail | 无 | 无 | 自己的投影 / 解析 | ✅ 取决于投影 |
| C action-cards | 无 | 原文无；决策在下轮以状态通知出现 | 文本里的 `action-card` 块 + Host 投影 | ✅ 投影可重放 |
| D toolview | 有（tool 行） | tool call args + result | 工具参数/结果 | ✅（事件已在日志） |
| E chat.node | 视实现 | 视实现 | 事件/投影 | ⚠️ 新事件类型不可取 |
| F conversation.view | 无 | 无 | 读同一份会话投影 | ✅ |
| H 动态包 | 视定义方式 | 视定义方式 | 运行期 | ❌ 刷新即失 |
| I ui_present | 有（tool 行） | 整段 HTML | 工具参数 | ✅（HTML 在日志） |

---

## 6. 针对现有插件的方案选项（不实现，只列）

### 方案 1（推荐）：给 learn-card 增加 fence 层

- **改哪里**：只改 `.cache/dsh-learn-card/client.js`：在现有 `tool.call.toolview` 注册旁边，再加一个 `conversation.chat.markdown.fence` 注册（key=`learn-card`，id 用新 id）；卡片数据从 `code` 解析 JSON，复用现有三套渲染函数；`streaming` 时渲染 `children`，解析失败回退 `children`。
- **数据怎么来**：
  - 代理（quiz-proxy）直接把一道题作为 assistant 文本输出，例如 4 个反引号包住的 ```` ```learn-card ```` 块 + JSON（或者在同一条文本里混排教学 prose，卡片在 prose 中）。
  - 或模型自己写：把格式写进 teaching skill / AGENTS 片段。
- **收益**：无 tool 行、无 tool-result 回环；代理侧可删掉 README 里那套「role:tool 检测 + 空 assistant 兜底」分支；卡片与文字同一条消息，教学语流更自然；历史重放/fork 免费。
- **代价/风险**：围栏 JSON 仍在上下文（token ≈ 卡片内容）；复制整条消息会带上原始块；需要约定格式并保证解析失败可见降级。
- **实施成本**：最小（client.js 一个注册 + 一个 parse 分支 + 一个测试用例）；HMR 秒级验证。

### 方案 2：官方 action-cards kind（要交互 + 模型知道用户选择时）

- 需要 Host + Client 两半 + skill + kind schema/verbs/handler；卡片固定 Turn 末尾、每 Turn 一张；动作走 Host Remote，决策记录在会话里，下一轮模型看到。
- 适合「回答完，问一句要不要继续/给个反馈按钮」这种低频交互；不适合白板那种高频答题循环。

### 方案 3：保持 toolview（host 驱动交互）

- whiteboard 现有形态不变：点击 → 本地/插件路由 → 判分 → 原地下一题。上下文里是 tool 行，但这正是它的工作方式（工具确实在执行/返回状态）。
- 若想减少 tool 行噪声，可以把「显示」迁到 fence，把「动作」留在自有 Host 路由（卡片内按钮走 `fetch`/Remote，而非 tool）。

### 方案 4：自定义 Chat Node / Conversation View（重）

- 只有需要「另一种会话视图」或替换内置行渲染时才做；新 durable 事件类型在第三方插件里不要碰。

---

## 7. 约束与坑（实现前先记住）

1. **版本是 developer preview**：`@deepseek-ai/*` 客户端包不是公共稳定 API；槽位类型/字段可能随版本变。所有事实以本次安装（0.2.3-alpha.2）为准，升级后要复查。
2. **客户端插件必须是预构建普通 JS bundle**，按 `dsh.client` 声明并由 profile 挂载；不能在页面里直接 `import` 别的 `@deepseek-ai/*` 客户端包（`practices.md` UI 节）。类型只用于阅读/d.ts 参考。
3. **HMR 范围**：`client.js` 改动 stat-poll 热替换（约 1s，需页面 SSE 活着）；`index.js`（Host 半）改动要重启 Host。
4. **围栏语言文法是 `[A-Za-z0-9_-]` 小写**；只有「有语言词 + 有内容」的围栏才进槽；`meta` 可携带参数。
5. **流式**：`streaming=true` 时 `code` 可能不完整，必须回退 `children`/骨架；结算才是最终卡片。
6. **fail-open**：任何解析/渲染错误都回退 `children`，并尽量给可见降级（不要空白）。
7. **只覆盖 Chat view 的 Markdown**：assistant 回答、reasoning、compaction summary；用户消息/Trajectory 不吃这个槽。
8. **复制行为**：原文（含围栏块）仍在消息里，复制按钮复制的是原文；若不能接受，需要额外处理（但那就不是"文本不变"了）。
9. **action-cards 的硬规则**：每 Turn 只认最终回答里第一个合法块；标题/简介不能空；块不能在 reasoning / 子 agent / 被 fork 继承的历史里；卡片动作要 Remote 权限、只读会话里写动作会被拒。
10. **页面/客户端版本对齐**：Host 升级后若页面一直没刷新，可能还在跑旧客户端 bundle；验证新槽位前先刷新页面。
11. **安全**：卡片渲染不要 `dangerouslySetInnerHTML` 渲染模型给的 HTML；parse JSON + React 文本渲染（learn-card 现状就是安全的）；外部链接只放行 http(s)。

---

## 8. 验证清单（实现阶段用，本次未做）

- [ ] 页面里渲染一个 ```` ```learn-card ```` 块后，`cordis_inspect` → Client `Slots.listSubTree`（root=`conversation.chat.markdown.fence`）能看到该层；**没渲染过围栏时 `available:false` 属正常（槽位按需物化）**。
- [ ] 改 `client.js` 后 HMR 秒级生效，无需重装/重启。
- [ ] 流式中：先看到代码块/骨架，结算后变成卡片。
- [ ] 非法 JSON：回退显示代码块（不是空白、不崩）。
- [ ] 刷新页面/翻历史/fork：卡片仍从文本重建。
- [ ] 移动窄屏、深色模式、`prefers-reduced-motion`：无横向溢出、token 颜色正确（复用 learn-card 的 `test/preview.mjs` 几何审计思路）。
- [ ] 复制消息：确认带原始块的文本是否可接受。
- [ ] token 账：对比围栏块 vs 旧 tool call 的上下文增量。
- [ ] 若走 action-cards：确认 `enabled`（Config `include:action-cards` 有 schema、默认 true）、一轮一张、动作后决策出现在下一轮状态通知里。

---

## 9. 证据索引

安装根 `<INSTALL>` = `<DSH_INSTALL>`

| 事实 | 位置 |
|---|---|
| hoc 槽位语义（替换/装饰/嵌套/崩溃回退/重挂） | `<INSTALL>/subsystems/slots.md:75-83` |
| Conversation 槽位树（fence、turnTail、toolview 的位置） | `<INSTALL>/subsystems/slots.md:140-230`（fence 在 :164，turnTail 在 :190） |
| fence 槽位类型声明 | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-chat/src/client/contract/slots.ts:433` |
| ChatView 的 `renderCode` → `renderSlot('conversation.chat.markdown.fence', fence, {entryKey: lang, children})` | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-chat/src/client/chat/ChatView.tsx:116-124,279-291` |
| 官方 math/diff/patch 层的注册与实现 | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-chat/src/client/fence/{register.ts,FenceLayers.tsx}` |
| `conversation.chat.turnTail` 类型 | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-chat/src/client/contract/slots.ts:506` |
| 官方 action-cards：隐藏块 + Turn 末尾卡片 + body 注册 | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-action-cards/src/client/{mount.ts,Card.tsx}`（mount.ts:42,54；Card.tsx:34,82） |
| action-cards Host：从回答文本读块、每 Turn 一张、决策/状态通知 | `<INSTALL>/node_modules/@deepseek-ai/dsh-action-cards/README.md` |
| action-card 块规则（缩进/位置/一 Turn 一张/子 agent 不算） | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-action-cards/README.md`「Card rules」 |
| 「卡片来自回答里的块」升级说明 | `<INSTALL>/upgrade-guide/v0.2.2-rc.3/action-card-blocks/guide.md` |
| toolview 注册契约（按 wire tool 名、open 键域、props 三阶段） | `<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-tool/README.md:38-50` |
| Conversation 数据模型 / 第三方 Definition 与 View 注册 | `<INSTALL>/subsystems/conversation.md`；`<INSTALL>/node_modules/@deepseek-ai/dsh-client-ui-conversation/README.md:53` |
| 不要新增 durable 事件类型 | `<INSTALL>/node_modules/@deepseek-ai/dsh-agent-preset/skills/dsh-doc/references/practices.md`（UI 节第 21 条） |
| 官方挂载 action-cards / cordis runner | `<INSTALL>/node_modules/@deepseek-ai/dsh-web-app/cordis.patch.yml:172-179,269-270,397-398,416-420` |
| 动态包（无模型工具、程序化 define/run、刷新即失） | `<INSTALL>/node_modules/@deepseek-ai/dsh-cordis-host-runner/README.md`；`dsh-cordis-client-runner/README.md`；`dsh-tool-cordis/README.md` |
| 现有 learn-card 的 toolview 数据来源与 proxy 改法 | `sokonanoda-lang/.cache/dsh-learn-card/README.md`「Where the Client reads the payload」「Exact proxy change spec」 |
| HMR（客户端热替换、Host 需重启） | `sokonanoda-lang/.cache/dsh-learn-card/README.md:91-113` |
| `ui_present` 的渲染与限制 | `~/.dsh/profiles/superagent/node_modules/@linxin666/dsh-remote-bridge/README.md` §Interactive cards in the transcript |

### 仍未核实 / 需实测

- 当前打开的页面所加载的客户端 bundle rev 是否与 Host 安装一致（Host 升级后若未刷新页面，需要刷新后再验证 fence 层）；`/` 页面受配对鉴权保护，脚本无法直接读取 boot graph。
- fence 层在自己的注册上是否可以用 `inject` 注入服务回调（官方 math 层未用；`hoc` 的注册 schema 支持 `inject` 面，但需在真实页面里用一次确认）。
- action-cards kind 对第三方插件开放的实际完整度（类型包、Remote 名、skill 注册路径）需按 `dsh-action-cards/src/types.ts` 再细读一遍并与一个最小 kind 联调。
- 复制按钮对隐藏围栏块的确切输出（需在页面实测）。
