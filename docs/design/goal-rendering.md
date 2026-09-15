# 设计：统一 goal 呈现（结构化 tag + 单一分类源）与 Infoview 落位（2026-09-14）

> 触发（用户）：
> 1. Infoview 现在给出「目标面板 (Infoview) 暂时不可用；「练习」面板中的
>    「当前光标处」组仍然可用」这类让人发懵的报错；希望它默认在**右侧**单独开出。
> 2. tactic hover、Infoview 等处的渲染各自独立，关键词高亮/颜色完全不同，不可维护；
>    要求设计一套统一方案，**对齐 VS Code 代码框标准**。
>
> 参考实现：Lean4 Infoview（`leanprover/vscode-lean4` + `@leanprover/infoview`）——
> 服务器返回**结构化 tag**（`InteractiveGoal { hyps, type: CodeWithInfos }`，
> `CodeWithInfos` 是带子表达式引用的 TaggedText），webview 用 React + 主题 CSS
> 变量渲染；分类来源唯一，客户端不"重新字符串高亮"。
>
> 现状（本仓库）：目标数据来自 `soko/stateAt` / `soko/goals`（`goal`/`binders`
> 都是**纯文本**）；hover 用 ` ```sokonanoda ` 围栏（VS Code TM 语法着色），
> Infoview 用自写 HTML + `media/infoview.css` 自己的 class/颜色；语言分类的唯一
> 事实源其实是 `crates/front/src/semantic.rs`（语义 tokens/补全都用它）。三处各
> 写一套 → 漂移。

## 1. 目标与非目标

**目标**
- **单一分类源**：`front::semantic` 是"什么词是什么 kind"的唯一事实源；hover 与
  Infoview 的着色/高亮都从它派生，且用测试防漂移。
- **统一呈现模型**：goal 状态（假设 + `⊢ 目标`）在 hover 与 Infoview 用**同一份
  结构化片段**（TaggedText：`(text, SemanticKind)` 串），布局一致（代码框）。
- **Infoview 默认在右**：作为独立视图容器落在**右侧辅助侧栏**（secondary
  sidebar），不再和「练习/课程」挤在 explorer。
- **可用的 fallback**：正常打开 Infoview 不再出现那条"暂时不可用"；只有真正无法
  渲染（webview 脚本被禁/资源缺失）才**静默**回退到树的「当前光标处」组。

**非目标**
- 不引入 React/打包链（保持零构建手写 HTML/CSS/JS；Lean 的 React 是它自己的取舍）。
- 不在 hover 里做颜色（VS Code hover 是受限 markdown，无法着色；颜色只在代码框
  由 TM 语法/主题给）。
- 不新增第二套编译器/判定；数据仍来自 LSP 快照。
- 不改 kernel / `front` 的判定语义。

## 2. 设计

### 2.1 结构化 goal 片段（唯一呈现模型）

在 `front` 增加一个**纯函数**：把一段"代码文本"按 `semantic` 分类成
`Vec<Run { text: String, kind: SemanticKind }>`（复用 `crates/front/src/semantic.rs`
的 `classify`/`classify_ident`，对外暴露一个 `runs(text) -> Vec<Run>` 或
`tagged(text)` API）。goal 状态的呈现模型 = 

```json
{ "hypotheses": [ { "name": "n", "ty": [ {"text":"Nat","kind":"type"}, … ] }, … ],
  "goal": [ {"text":"P","kind":"function"}, {"text":" n","kind":"variable"}, … ] }
```

- 单一来源：`front::semantic`（与分析器同一套关键字/排序/构造子/绑定分类）。
- LSP 侧：`soko/stateAt` / `soko/goals` 在**保留现有 `goal`/`binders` 字符串字段**
  （兼容旧客户端）之外，增加 `tagged`（或 `goal_runs`/`binder_runs`）结构化字段。
- 这是 Lean `CodeWithInfos` 的**精简版**：我们暂时不带"子表达式引用/跳转"，
  只带 `(text, kind)`（先用够用；将来可加 `ref`）。

### 2.2 呈现层统一（hover 与 Infoview 同一形状）

- **共同布局**：每条假设一行 `name : ty`，最后 `⊢ goal`；多目标用 `目标 i/n` 分节；
  tactic hover 顶部 `tactic k/n`。这套**行模型函数**只写一处（`front` 产出结构化
  行；渲染器只负责画）。
- **hover**（`textDocument/hover` → VS Code markdown）：继续用
  ` ```sokonanoda ` 代码围栏（VS Code 标准代码框；着色由扩展的 TM 语法给）。
  既然 hover 无法着色 class，就用**同一份文本行模型**，保证内容/排版与 Infoview 一致。
- **Infoview**（webview）：把结构化 runs 渲染成
  `<pre class="codebox"><code class="language-sokonanoda">` 内的 `<span class="tok-<kind>">`；
  颜色由 `media/infoview.css` 按 `tok-<kind>` 映射到 VS Code 主题变量
  （`--vscode-symbolIcon-*` / `--vscode-charts-*` 或一组按 `data-theme` 的调色），
  映射表**集中在一处**并有注释/测试。
- **防漂移**：`front::semantic::keywords()` 与 `editor/vscode/syntaxes/sokonanoda.tmLanguage.json`
  的关键词/tactic 列表由**测试断言一致**（`crates/cli/tests/extension.rs` 或 front
  单测），任何一边新增词必须同步另一边。

> 说明：VS Code **不向 webview 暴露 TextMate/语义 token 的颜色**，因此 Infoview 无法
> 100% 复刻用户主题的 token 颜色；本设计选择"**同一分类 + 主题变量近似映射**"，并把
> 映射写成单一表，避免三处各写。若将来 VS Code 开放 token 颜色 API，替换映射即可。

### 2.3 Infoview 落位（默认右侧）

- `package.json` 增 `viewsContainers.secondarySidebar`（**小写 b**；VS Code 已支持，
  需 bump `engines.vscode`），容器 `id: sokonanoda`，把现有 `sokonanoda.infoview`
  视图放进去 → 默认落在**右侧辅助侧栏**。
- 保留 Explorer 的「练习」「课程」树不变；Infoview 视图从 explorer 移出。
- **engine bump**：`engines.vscode` 提到 **`^1.106.0`**。已核实（VS Code 源码
  `viewsExtensionPoint.ts`）：`1.104`/`1.105` 的 `case 'secondarySidebar'` 里有
  `checkProposedApiEnabled(description, 'contribSecondarySidebar')`（需 proposed
  API），**`1.106.0` 起该检查被移除**（commit 75988f8，key 统一为小写 b），是首个
  无需 proposal 的稳定版。老版本会忽略该键 → 必须 bump engine，否则视图会整块消失。
- 命令 `sokonanoda.openInfoview` 改为 `executeCommand('workbench.view.extension.sokonanoda')`
  / `sokonanoda.infoview.focus`，聚焦容器。

### 2.4 fallback 修复（去掉"暂时不可用"）

- 现状：`openInfoview` 聚焦视图后 **等 `ready` 握手 2s**，超时即弹那条消息
  （`extension.js` `INFOVIEW_READY_TIMEOUT_MS = 2000`）。视图本就该在聚焦后按需
  渲染，握手只是"增强"。
- 改：**不再用握手超时判定可用性**。聚焦后直接交给 webview 渲染；宿主缓存最后一份
  `state`/`decls` 在 `webview.onDidReceiveMessage({type:'ready'})` 时重放（已有
  机制）。只有当**明确**不可用（`enableScripts` 被禁 / 媒体资源读不到 / `resolveWebviewView`
  抛错）时，记一次输出通道日志并让树的「当前光标处」组作为静默回退；**不弹**用户级错误。
- 若确需提示，只给一次性、可忽略的 InformationMessage（不阻塞），且不再出现"仍然可用"
  这类让人困惑的措辞。

## 3. 落地切片（TDD）

- **S1 front**：`semantic` 暴露 `tagged_runs(text) -> Vec<Run>`；单测覆盖
  关键字/类型/变量/数字/构造子分类与既有 `semantic_tokens` 一致。
- **S2 行模型**：front/`lsp::render` 产出 goal 的结构化行（假设 + `⊢`）；单测。
- **S3 LSP**：`soko/stateAt`/`soko/goals` 增 `tagged` 字段（保留字符串字段）；协议
  `docs/protocol.md` 更新；进程内 rpc 测试 + 旧字段回归。
- **S4 hover**：改用同一行模型（内容不变、结构统一）；既有 hover 断言更新。
- **S5 Infoview**：渲染 `tok-<kind>` span；CSS 映射表；静态契约（webview 只
  `textContent`/`createElement`，无 `innerHTML`/远程 URL）+ 集成 smoke。
- **S6 落位**：`viewsContainers.secondarySidebar` + engine bump + `openInfoview`
  聚焦容器 + fallback 重写；`package.json`/静态契约测试。
- **S7 防漂移**：TM 语法关键词/tactic 列表 == `semantic::keywords()` 断言；扩展版本
  bump（minor：新面板位置 + 协议字段）与 CHANGELOG。

## 4. 验收

- hover 与 Infoview 的 goal 呈现**同源同形**（同一行模型、同一分类）；`rg` 可见
  分类只来自 `front::semantic`；
- Infoview 默认出现在右侧辅助侧栏；正常打开**不再**出现"暂时不可用"；
- webview 真正不可用时静默回退树组，不弹困惑错误；
- `soko/goals`/`soko/stateAt` 旧字符串字段不变（兼容）；`docs/protocol.md` 同步；
- `sokonanoda gate` 全绿（含 front semantic 一致性、LSP 协议、扩展静态契约、集成 smoke）。

## 5. 风险与取舍

| 风险 | 取舍 |
|---|---|
| VS Code 不暴露 token 颜色给 webview | 同一分类 + 主题变量近似映射（单一表）；将来有 API 再替换 |
| `secondarySidebar` 需 bump engine，排除老 VS Code | 已核实首个稳定版 **1.106.0**；engine 提到 `^1.106.0`（老版本会忽略该键 → 视图整块消失，必须 bump） |
| hover 无法 class 高亮 | hover 保持 VS Code TM 代码框；与 Infoview 共享**行模型与分类源**，不共享颜色实现 |
| `tagged` 字段增大 payload | 只在请求期计算、按目标小体积；有界、无网络放大 |
| 与既有「当前光标处」树重复 | Infoview 是增量能力；树保留为默认+兜底（见 §2.4） |

## 6. As-built（0.40.0，2026-09-15）

- **S1**：`front::semantic::{Run, tag_runs, tag_expr, declaration_kinds,
  SemanticKind::ALL, SemanticKind::as_str}`（`crates/front/src/semantic.rs`）
  + 5 个单测。
- **S2/S4**：不再单独出「行模型」——hover 的既有 `{name} : {ty}` / `⊢ {goal}`
  行模型就是客户端共用形状；hover 保持 ` ```sokonanoda ` 代码围栏，颜色由
  **同步后的 TM 语法**给（见 S7），内容与 Infoview 同源。
- **S3**：`soko/stateAt` 增 `goal_runs`（单值 + 每 goal）与 `GoalBinderInfo.ty_runs`；
  `soko/goals` 的 binders 同步增 `ty_runs`；`docs/protocol.md` 记词表。
  进程内测试 `state_at_carries_semantic_runs_for_goals_and_hypotheses`。
- **S5**：`media/infoview.js::codeBlock` 渲染 `tok tok-<kind>`（仅 textContent）；
  `media/infoview.css` 单一主题映射；契约测试
  `infoview_colours_every_semantic_kind_from_the_single_source`（遍历
  `SemanticKind::ALL`）。
- **S6**：`viewsContainers.secondarySidebar`（`sokonanoda` 容器）+
  `engines.vscode ^1.106.0` + `@types/vscode 1.106.0`；`openInfoview` 去掉
  `waitReady`/`INFOVIEW_READY_TIMEOUT_MS` 与「暂时不可用」，改聚焦容器。
- **S7**：`syntaxes/sokonanoda.tmLanguage.json` 词表 == `front::semantic`
  （keywords + sorts + forall/∀），测试
  `tm_grammar_keywords_follow_the_single_source`。
- **门面**：`description` ≤300（`marketplace_description_fits_the_gallery_limit`）、
  README/CHANGELOG/协议/规范同步；版本 0.40.0。

### 与设计的偏差
- 未做「子表达式引用/跳转」（Lean `CodeWithInfos` 的 ref 部分）：runs 只带
  `(text, kind)`，够用且不引入新协议负担。
- 未做单一大 `<pre>` 代码框：沿用既有 `goal-ty` + `binders` 结构，仅把着色
  接入 runs，避免推翻既有布局契约。

## 7. 呈现面高亮统一（0.43.0 完成）

原则：**凡渲染 `.sokonanoda` 语言文本，着色只来自 `front::semantic`**
（编辑器语义 token + TM 语法 + `tag_runs`）。统一手段是给每个 markdown 面一个
`{sokonanoda}` 代码围栏（LSP `code_block`/`CODE_LANG`，扩展 `codeMarkdown`）。

| 面 | 处理 |
|---|---|
| 表达式/签名 hover | ✅ ` ```sokonanoda `（原 ` ```text ` 不高亮） |
| 声明 hover（签名 + 目标态） | ✅ 签名与 goal（假设 + `⊢`）都是 `sokonanoda` 代码块 |
| 补全 documentation | ✅ 声明签名以 `sokonanoda` 围栏给出（`detail` 仍是纯文本，VS Code 限制） |
| 练习树 tooltip（目标 / 假设） | ✅ `MarkdownString.appendCodeblock(…, "sokonanoda")` |
| Infoview 声明列表的类型提示 | ✅ 0.44.0：`soko/goals` 增 `ty`/`ty_runs`，声明行下用小字等宽着色显示，点击跳转 |
| 诊断消息、inlay hint、TreeItem.description、CodeAction 标题 | ⛔ 保持纯文本——VS Code 不渲染 markdown / 无法着色，文档写明 |
| tactic hover | ✅ tactic 片段与 goal 都是代码块（原 tactic 是行内代码） |
| 半表达式 hover | ✅ 推断类型 / 目标 / 剩余目标都是代码块 |
| tactic/半表达式 hover、Infoview | ✅ 0.40.0 起统一（goal 状态；0.43.0 补齐 header） |

**平台限制**：VS Code 只对**带语言 id 的围栏块**着色，单反引号行内代码无法指定
语言。因此 hover 里**成块的代码一律围栏**；仅有「散文里提到单个词」的场合
（如「在 `sorry` 处填写…」）保持行内代码——那不是可着色的代码片段。

校验：`crates/lsp/src/lib.rs` 的 `code_fences_always_use_the_sokonanoda_language`
+ hover/completion 断言；`crates/cli/tests/extension.rs` 的
`rendered_language_text_uses_the_sokonanoda_fence`。
