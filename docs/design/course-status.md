# 设计：课程地图（sokonanoda course）+ REPL 历史持久化 + course 提示阶梯

> **状态：已实现**（`crates/cli/src/course.rs`、REPL 历史、`sokonanoda.courseMap`、
> 单元提示阶梯）。§4 的 golden 计数只是**示例**（非第二真源）——现为 11 单元
> （⑪ = I16 追加），
> 权威值在 `crates/cli/tests/course.rs`（`GOLDEN`）与
> `crates/cli/tests/course_status.rs`（`GOLDEN` + 汇总）。

> 状态：设计定稿（2026-09-07，第十二轮实施）。依据：`docs/notes/gap-analysis.md`
> #8（`soko/courseStatus` + VS Code 章节地图）与小项（REPL 命令历史）、
> STATUS 第十一轮余项（course/ 提示阶梯内容）。

## 0. 头脑风暴与取舍

1. **聚合在哪一侧？** 备选：LSP 自定义请求（服务器读工作区文件）vs CLI 子
   命令 + 客户端拉起。选 **CLI 子命令**：服务器是单文档模型（有意取舍，
    `docs/architecture.md` §8），让它读任意工作区文件是方向性扩张；CLI
    聚合一次跑完 8 个单元天然并行、可被 CI/agent 直接消费；VS Code 端
   serverPath 发现机制已有，复用来定位二进制即可。
2. **进度语义**：course 单元是「素材库」，学习者实际面对的是画布——因此
   课程地图是**给 agent/教师视角的可解性+内容地图**（每单元多少题、多少
   已有钥匙解出、多少开放、多少失败），不是单一学习者的进度记录（学习者
   进度 = 画布自身状态，天然持久化）。命名用 `sokonanoda course`。
3. **事件词汇**：新事件 `course.unit` / `course.summary` 进封闭词汇表 +
   protocol.md（conformance 测试守护）；exit 恒 0（进度不是错误；文件级
   IO/解析错误才非零）。
4. **REPL 历史存储位置**：`$HOME/.sokonanoda_history`（零依赖，`dirs` crate
   不引入）；只追加写入 + 启动载入去重展示？——最小实现：启动时读入内存、
   退出/每条追加；**不做行编辑/上箭头召回**（那是 readline 级改造，另立项），
   历史文件先保证「命令可追溯」。
5. **course/ 提示阶梯内容**：与 playground 同规范（`-- soko:hint`，2–3 条/
   题，思路→目标形态→关键件，**答案不进提示**）；golden 计数不受影响
   （注释不产事件——playground 第十一轮已验证同机制）。

## 1. `sokonanoda course <course.json>`

- 输入：course.json（`[{file, title, unit}]`，相对 course.json 所在目录解析
  单元文件路径）；
- 对每个单元：`compile_fol_with(unit_src, Full)` 聚合
  `decl.checked` / `exercise.open` / failed / `expr.reduced` 计数 + 单元
  标题；单元文件缺失/解析失败 → `course.unit` 带 `"error"` 字段并继续；
- 输出（JSON Lines，stdout）：

```json
{"type":"course.unit","file":"unit1-propositions-proofs.sokonanoda","title":"单元① 命题与证明项","unit":1,"checked":13,"open":6,"failed":0,"reduced":1}
{"type":"course.unit","file":"unit2-equality-rfl.sokonanoda","title":"单元② 等式与 rfl","unit":2,"checked":2,"open":5,"failed":0,"reduced":2}
{"type":"course.unit","file":"unit3-functions-arrows.sokonanoda","title":"单元③ 函数与箭头","unit":3,"checked":2,"open":6,"failed":0,"reduced":2}
{"type":"course.unit","file":"unit4-by-tactics.sokonanoda","title":"单元④ by 写法：tactic 证明","unit":4,"checked":13,"open":5,"failed":0,"reduced":0}
{"type":"course.unit","file":"unit5-universes-sort.sokonanoda","title":"单元⑤ 宇宙：函数类型的类型","unit":5,"checked":0,"open":6,"failed":0,"reduced":1}
{"type":"course.unit","file":"unit6-induction-recursion-1.sokonanoda","title":"单元⑥ 归纳与递归 Ⅰ","unit":6,"checked":7,"open":6,"failed":0,"reduced":3}
{"type":"course.unit","file":"unit7-induction-recursion-2.sokonanoda","title":"单元⑦ 归纳与递归 Ⅱ","unit":7,"checked":7,"open":4,"failed":0,"reduced":4}
{"type":"course.unit","file":"unit8-quantifiers.sokonanoda","title":"单元⑧ 量词：forall 与 exists","unit":8,"checked":14,"open":7,"failed":0,"reduced":1}
{"type":"course.unit","file":"unit9-relations-connectives.sokonanoda","title":"单元⑨ 关系与联结词","unit":9,"checked":13,"open":8,"failed":0,"reduced":0}
{"type":"course.unit","file":"unit10-reading-proofs.sokonanoda","title":"单元⑩ 读证明与综合","unit":10,"checked":7,"open":6,"failed":0,"reduced":0}
{"type":"course.summary","units":10,"checked":78,"open":59,"failed":0}
```

- 人类视图（无 --json）：逐单元一行
  `unit 1 单元① 命题与证明项 —— 13 checked · 6 open · 0 failed`；末行
  `共 11 单元 —— 85 checked · 65 open · 0 failed`；
- CLI 接线：`Some("course") => positionals.get(1)`（--json 旗标同样适用）。

## 2. VS Code 课程地图

- 新树视图 `sokonanoda.courseMap`（「课程」面板，与既有「练习」树同
  viewContainer）：每单元一个节点（图标按 open/failed 着色），子节点 =
  `checked / open / failed` 计数摘要 + 「打开单元」命令跳转文件；
- 数据源：激活时与手动刷新命令 `sokonanoda.courseRefresh` 时，用既有
  serverPath 发现逻辑定位 `sokonanoda` 二进制跑
  `sokonanoda course <workspace 的 course/course.json>`（找不到 course.json
  或二进制 → 空树 + 温和提示，不报错刷屏）；
- 契约测试同步（package.json 声明 = extension.js 注册；新视图不破坏既有
  命令断言）。

## 3. REPL 历史持久化

- 文件：`$HOME/.sokonanoda_history`（HOME 缺失 → 静默禁用）；
- 启动：读入（截尾 1000 行）；每条非空输入后追加一行；`#exit`/EOF 正常退出；
- 不改判定语义、不加行编辑（明确写在 help 里：`历史: ~/.sokonanoda_history`）。

## 4. course/ 提示阶梯内容（素材库完善）

- 每个单元的 open 练习挂 2–3 条 `-- soko:hint`（规范同 playground）；
- 钥匙来源：`course/solutions/`（全部经内核验证）；阶梯只给思路/形态/关键件；
- **验收锚点（示例，权威见测试）**：`crates/cli/tests/course.rs::GOLDEN` 与
  `course_status.rs::GOLDEN` 的逐单元 `(checked, open, reduced)` 现为
  （13,6,1 / 2,5,2 / 2,6,2 / 13,5,0 / 0,6,1 / 7,6,3 / 7,4,4 / 14,7,1 /
  13,8,0 / 7,6,0）、汇总
  `units=10 checked=78 open=59 failed=0`、solutions 零诊断——注释级改动不产事件
  （playground 已验证）。

## 5. 文件分工（互斥清单）

| owner | 允许修改 |
|---|---|
| 主会话（预接） | `docs/design/course-status.md`、`docs/protocol.md`（course.unit/summary 事件 + REPL 历史）、`crates/cli/src/main.rs`（course 分支 + mod）、`crates/cli/src/help.rs`（course/历史两行） |
| E（course 聚合） | `crates/cli/src/course.rs`（新）、`crates/cli/tests/course_status.rs`（新）、`crates/cli/tests/common/mod.rs`（词汇 + course.unit/course.summary）、`crates/cli/src/json_report.rs`（仅当事件序列化需要） |
| F（VS Code 课程地图） | `editor/vscode/{extension.js,package.json}`、`crates/cli/tests/extension.rs` |
| G（REPL 历史） | `crates/cli/src/repl.rs`、`crates/cli/tests/cli.rs`（repl 测试追加） |
| H（course 阶梯内容） | `course/unit*.sokonanoda`（10 个单元文件；**solutions/ 不动**） |
