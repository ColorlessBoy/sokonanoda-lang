# 侧栏做题面板（C9.7）

一句话：聊天卡片负责「发题」，右侧栏面板负责「做题」——同一道代码练习，在
面板里编辑、点「运行」，结果**仍然只来自** C9.4 的
`compose.py` + `scripts/soko grade --json` 判卷桥。

## 表面

- **类型**：DSH 右栏的 page tab（`sidebarRightTabs` 注册，`kind =
  sokonanoda-exercise`，`keepMounted: true`）。一栏一题，重开同一 kind 会
  focus 已有 tab 并把新的 `params` 交给 body（`navigation.revision` 递增）。
- **深链**：代码练习卡的 `▤ 在侧栏面板中打开` 按钮调用
  `sidebarRight.openTab(kind, { params: { exerciseId, course, title } })`。
  没有 sidebar 服务时按钮不渲染，卡片保持 C9.4 的聊天提交路径。
- **body**：按 `navigation.params.exerciseId` 拉题目；编辑器预填可信模板；
  `运行` 发回 Host；`⌘/Ctrl + Enter` 同义；`恢复模板` 回到初值。
- **chip title**：`sidebar.right.pane.tab.title` 座位显示题目标题，缺省显示
  「做题面板」。

## Host 两条路由（loopback / `auth: 'anonymous'`）

| 路由 | 方法 | 作用 |
|---|---|---|
| `/sokonanoda-exercise?id=<id>` | GET | 返回面板可见题目字段（白名单，**不含 solution**） |
| `/sokonanoda-grade` | POST `application/json` | `{exerciseId, code}` → C9.4 判卷桥 → `checked/open/diagnostic` |

信任边界与理由：

1. **题目只从仓库取**。客户端只发 `exerciseId`；Host 在
   `courses/exercise-dataset/data/<course>.json`（数据集）或
   `courses/set-theory/knowledge/data/code-exercises.json`（引擎小题库）里查
   记录，签名/模板永远来自可信记录。用户代码只提供**证明体**（C9.4
   `assembleSubmission` 丢弃用户签名）。
2. **不把答案发给客户端**。`panelExerciseView()` 是显式白名单；**没有
   solution 字段**。
3. **匿名 loopback 路由**与既有 `/sokonanoda-client-report` 同档；判卷只读
   仓库、只写系统临时目录、判完即删，日志不出现用户代码。POST 强制
   `content-type: application/json`（跨站文本请求无法在不触发 preflight 的
   情况下命中），请求体上限 64 KiB + 头开销。
4. **没有第二条判卷路径**。路由直接调 `lib/code-grade.mjs` 的
   `gradeCodeSubmission`，与聊天快路径完全同一份实现；`passed` 只由
   `evaluateGrade` 产生。

## 结果状态（不伪造）

| 状态 | 何时 | 面板 |
|---|---|---|
| `pass` | Host `result.passed === true` | ✅ 通过（真内核判卷）+ checked/open/diagnostic 计数 |
| `fail` | 内核有 diagnostic / 目标仍 open / 用户代码格式错误 | ❌ 还没通过 + 目标 + 前 3 条诊断（code/行/message）或 sorry 提示 |
| `error` | 路由不可用、超时、compose/soko 缺失 | ⚠️ 判卷服务不可用 + Host 原因；**绝不显示通过** |

## 已知限制（有意）

- 面板运行**不写课程进度**（`mark-code` 仍在聊天路径上调用）。面板是「做题
  面」，不是第二套进度系统；以后要记也只在 Host 判卷完成后接同一条
  `engine.py mark-code`。
- 只编辑单条声明；多文件项目、书页/静态站、VS Code 编辑器不在本切片内。
- 面板题源限于题目集与引擎小题库；题库没有的 id 返回 `not-found`（不猜）。
- 不改 kernel、不改 C9.4 的判定口径。

## 判据

- `node dsh/plugins/sokonanoda-dsh/test/run.mjs`：Host 33 / client 45。
- Host 单测覆盖：可信题目解析（含路径穿越拒绝）、白名单不含 solution、
  **真内核**错误/正确两次判卷、路由 200/400/404/405/415、`apply()` 注册两条
  路由。
- Client 单测覆盖：deep link 打开的 kind/params、面板加载/编辑器/Run、pass
  只来自 Host 结果、diagnostic/sorry/服务失败不显示通过、加载失败可重试。
- 真 DSH Web Before/After 截图见 `reports/2026-11-27-c97-exercise-panel/`。
