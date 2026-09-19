# 项目状态视图（`query project` / `soko/project` / VS Code 项目树）

> 状态：**已实现（0.58.0，2026-09-18，待办批次 4）**。ROADMAP I16 的 P7 项
> `soko/project` 落地；设计底稿是 `docs/design/imports-and-projects.md` §3 的
> ⑦（"新增只读请求 `soko/project`（根、manifest、模块表、依赖边、每模块状态）
> 供扩展画/导航"）与 Q6（当时留 backlog，理由是"没有真实消费者"——现在消费者
> 是 VS Code 项目树 + agent 的 `query project`）。

## 0. 一句话结论

项目模式今天只有**入口文件**的视图（诊断/hover/goal 都是"入口这一份"），
"我在哪个项目里、根在哪、清单是谁、闭包里有几个模块、哪个模块拖累了入口"
只能靠 CLI 反复跑或读文档推。本设计把**闭包状态**做成一个只读、机器可判的
查询：`front::query::QueryDoc::project_view()` 是唯一真相层，CLI `query project`
（agent/脚本）与 LSP `soko/project`（编辑器）是它的两个传输，VS Code 用一棵
项目树 + 状态栏 tooltip 呈现。

## 1. 现状与差距

| 已有 | 缺什么 |
| --- | --- |
| `ProjectReport`（`front::project`）：`entry`/`root`/`manifest`/`modules`/`diagnostics`/`requires_warning` | 只有 `--json` 事件流与诊断；**没有"项目摘要"这个对象**，CLI/LSP/扩展各拿不到一致形状 |
| `ModuleReport`：name/path/imports/source/report/events | **丢了加载状态**：`LoadedModule.failed`/`blocked` 与"依赖编译失败 ⇒ 结果被丢弃"在 `compile_plan` 里被抹平，消费者分不清"模块干净"与"模块没参与编译" |
| LSP 有闭包（`QueryDoc::project_modules()`）、`soko/goals`/`stateAt`/`hints` | 没有"项目"这个请求；扩展无法画项目树/在状态栏说明失败模块 |
| CLI `query check` 只给入口的计数 | agent 想知道"闭包里哪个模块坏了"必须自己 `--json` 扫全部事件并反推文件 |

## 2. 需求（正确性标准）

1. **同一份真相**：CLI 与 LSP 的答案由同一个函数产出（`front::query`），字段名
   一次定义（`docs/protocol.md`），两边不允许各拼一份。
2. **机器可判**：单文件（无 `import`）不是错误，是**另一种合法状态**——用
   `project: null` + `reason` 表达，绝不混进"空项目"。
3. **状态齐全**：每个闭包模块要能看出 `compiled` / `load-failed` / `blocked`，
   入口要标出来，失败模块要带得走的诊断（code + 消息 + 归属模块）。
4. **只读**：不改文件、不写缓存、不触发编译以外的事；答案里的路径是**绝对路径**
   （编辑器要拿它开文件、agent 要拿它拼命令）。
5. **零成本可重复**：答案从**已经编译过的** `ProjectReport` 派生（不重跑内核）；
   LSP 每次按键已经重编译，视图随之更新即可。
6. **不破坏 A1**：无 `import` 的文件不产生任何新行为（单文件路径逐字节不变）。

## 3. 真相层：`front::query::project_view`

```text
ProjectView {
  entry: String            // 入口模块名（点分）
  root: String             // 模块根（绝对路径，永不空）
  manifest: String | null  // 生效清单；null = 零配置（根 = 入口目录）
  requires_warning: String | null
  modules: [ModuleView]    // 拓扑序，入口在最后（与 ProjectReport 同序）
  diagnostics: [ {code, message, module, severity, start, end} ]  // 项目级（含 import 行）
  counts: { modules, compiled, failed, blocked, decls, errors, warnings, open_exercises }
}

ModuleView {
  name: String
  path: String             // 绝对路径
  status: "compiled" | "load-failed" | "blocked"
  entry: bool
  imports: [String]
  decls: usize             // 报告里的声明数（含 open 练习）
  errors: usize
  warnings: usize
  open_exercises: usize
  message: String | null   // 状态的一句话解释（load-failed/blocked 时给）
}
```

* `severity`（`error`/`warning`）让消费者自己数错/警——**不要**在客户端维护
  一份项目 code 清单（项目层确实有 warning，例如依赖里的开放练习）。
* 数据来源：`ProjectReport`（`modules` 已按拓扑序含被阻断模块）+ 新增的
  `ModuleReport::status`（见 §4）+ 每模块 `report.decls/errors/warnings`。
* `QueryDoc::project_view()` 返回 `Option<ProjectView>`；`None` 时
  `project_view_reason()` 给机器码：`no-imports`（单文件）/ `no-path`
  （stdin 或 `--text` 且没有 `--root`）/ `parse-error`。
* 这是**只读派生**：不算摘要、不碰缓存、不编译第二次。

## 4. 需要补的一处数据：`ModuleStatus`

`compile_plan` 今天把三类模块揉进了同一个 `Vec<ModuleReport>`：

| 真实状态 | 今天的样子 |
| --- | --- |
| 编译过（可能带错误） | `report` 里有内容 |
| 加载期就失败（文件缺失/解析错误/环） | `report` 是 `DocumentReport::default()` |
| 上游失败 ⇒ 没进编译，或编译过但结果被丢弃 | 同上（**与上一种不可区分**） |

新增 `project::ModuleStatus { Compiled, LoadFailed, Blocked }` 挂在
`ModuleReport.status` 上，由 `compile_plan` 在构造报告时按
`LoadedModule.failed` / `.blocked` / `result_blocked` 三个已知集合填。
`Blocked` 的 `message` 由既有诊断（`import-dependency-failed` /
`import-not-found`）提供，视图只做搬运。

## 5. 传输 A：CLI `query project`（agent 一等公民）

```
scripts/soko query project --file course/unit11-project/Exercises.sokonanoda
```

* 信封沿用 `soko.query/1`：`ok:true`，`data = {project: <ProjectView|null>, reason: <code|null>}`；
  **退出码 0**（"单文件"是合法答案，与 `state.goal == null` 同款哲学）。
* 不需要位置参数；`--file`/`--text`/stdin/`--root` 与其它 op 同规则。
* MCP 工具 `project`（`mcp__sokonanoda__project`）是它的薄包装——agent 少一次
  `--json` 事件流扫描就能回答"这个项目里哪个模块坏了"。

## 6. 传输 B：LSP `soko/project`

请求 `{textDocument: {uri}}`（与其它 `soko/*` 同款，`uri` 真的被解析，不再当死
字段），应答 `{uri, version, project, reason}`——回显身份，扩展据此丢弃"答的是
另一份文档"的过期答案（沿用 0.57.0 批次 2 的纪律）。走 `focus_request` +
未保存缓冲（`overlay`）语义：视图描述的是**编辑器里那份**闭包。

## 7. 呈现：VS Code 项目树 + 状态栏

* **树**：资源管理器里的 `sokonanoda.project`（`when: resourceLangId ==
  sokonanoda`）：
  - 根 = 项目：`<root 目录名>`，description = `N 模块 · M 失败`，tooltip 给
    **清单来源**（`sokonanoda.toml` 路径或"零配置（根 = 入口目录）"）与
    `requires` 提示；点击 = 打开清单（没有清单时打开根目录）。
  - 子 = 每个模块：文件名 + `入口`/`依赖` + `N 声明`（有练习时 `· K 练习`），
    失败/被阻断模块用 error/warning 图标 + message；点击 = 打开该模块并定位。
  - 单文件：一条 "单文件（无 import）" 的占位节点，说明项目树只对项目生效。
* **状态栏**：不新增第二个 item（现有 item 显示 goal 进度），把项目一行拼进它的
  **tooltip**：`项目：<root>（清单/零配置）· N 模块 · M 失败`。
* 刷新时机：`onDidChangeActiveTextEditor`、诊断事件之后（服务器已经重编译）、
  手动 `sokonanoda.project.refresh`。

## 8. 分阶段与验收

| 步骤 | 交付 | 验收 |
| --- | --- | --- |
| 1 | `ModuleStatus` + `ProjectView` + `QueryDoc::project_view` | front 单测：三类状态、counts、单文件返回 `None`+`no-imports` |
| 2 | CLI `query project` + MCP `project` | `crates/cli/tests/query.rs`：2 模块项目字段齐全、单文件 `project:null`+reason、MCP 工具名/参数（`dsh.rs`） |
| 3 | LSP `soko/project` | `crates/lsp/src/tests/project.rs`：身份回显、模块表、编辑后状态更新 |
| 4 | VS Code 树 + tooltip | `editor/vscode/test-extension-host.js`（stub host：单文件/项目/失败三态渲染）、`extension.rs` 静态契约 |
| 5 | 文档与版本 | `docs/protocol.md`（op 表 + 自定义请求）、TESTING/HANDOVER/STATUS/REQUIREMENTS §9、skills/AGENTS（命令面）、扩展 README/CHANGELOG + 版本 0.58.0 |

**总验收**：`cargo test --workspace --locked` 全绿 + `scripts/soko gate` PASS +
（位置搬移类的既有纪律）二进制对拍不适用于本批（纯新增，不改既有输出——
由 CLI 契约测试与 A1 断言守护）。

## 9. 明确不做（本批）

* 不画依赖图（DAG 可视化）：树只列拓扑序与 import 名；真图留给后续。
* 不做项目级操作（重命名模块文件、新建模块、`sokonanoda new`）——需要写文件。
* 不做跨进程的模块级缓存（P7 的 decl 级产物）；视图不承诺"哪个模块没重编译"。
* 不为单文件文件伪造项目（零配置退路只在**有 import 时**才成立）。
