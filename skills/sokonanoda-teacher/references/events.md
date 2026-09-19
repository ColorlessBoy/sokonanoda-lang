# 事件词汇与 JSON 形状（teacher 参考）

> 权威来源：`docs/protocol.md`（协议）、`crates/cli/src/json_report.rs`（实现）。
> 本文件是判卷视角的浓缩版 + 一段真实事件流。

## 两种机器视图

| 视图 | 命令 | 词汇 | 用途 |
|---|---|---|---|
| 批处理 `--json` | `sokonanoda --json <file>` | `decl.checked` `example.checked` `expr.typed` `expr.reduced` `decl.printed` `exercise.open` `diagnostic` `warning` | 一次判卷的完整事件流（agent 主用） |
| watch 流 | `sokonanoda watch <file>`（或 `--doc <file>` / `--workspace <root>`） | 握手 `service.hello`，随后 `file.didChange` + delta（`decl.checked`/`decl.failed`/`exercise.opened`/`exercise.solved`/`exercise.failed`）+ `diagnostic` | 常驻监听：每版只推状态变化 |

> watch 流第一条 JSON 行是握手 `service.hello`（`{protocol, engine, pid}`）。
> `--workspace` 下每文件一个会话、版本号**按文件独立**、事件带 `file` 字段，
> 跨文件**无全序**；opener 的 `recompiled_from: 0` 表示增量可能被合并，
> **必须视为全量重同步**。`file.changed` 是已弃用别名（不再发射）。

## 批处理事件形状

```json
{"type": "decl.checked", "human": "checked declaration id", "name": "id"}
{"type": "expr.typed", "human": "id: Prop -> Prop", "text": "id",
 "inferred_type": "Prop -> Prop", "span": {"start": {"offset": 87, "line": 5, "column": 8}, "end": {"offset": 89, "line": 5, "column": 10}}}
{"type": "exercise.open", "human": "exercise open (fill the sorry)", "name": "double"}
{"type": "diagnostic", "stage": "elab", "code": "elab-unknown-identifier",
 "message": "unknown identifier `nate`", "hint": "这个名字还没有被定义。…",
 "span": {"start": {"offset": 18, "line": 1, "column": 19}, "end": {"offset": 22, "line": 1, "column": 23}}}
{"type": "warning", "human": "warning[reserved-declaration-name]: …",
 "code": "reserved-declaration-name", "message": "`Prop` 内核已经定义过了，不能再声明一次。…",
 "hint": "删掉这一行即可；…", "span": {"start": {...}, "end": {...}}}
{"type": "warning", "human": "warning[redundant-sorry]: …",
 "code": "redundant-sorry", "message": "这一行的 sorry 是多余的：前面的项已经完成了证明，…",
 "hint": "删掉这一行 sorry，这条声明就会通过内核检查；…", "span": {"start": {...}, "end": {...}}}
```

- `span` 是 1-based 行列 + 字节 offset；`--json` 的所有诊断带 `code` 与 `hint`。
- `warning` 不改变退出码、不把文件判成错误。`reserved-declaration-name`
  表示顶层声明名撞上了内核已定义的名字（`Prop`/`Sort`/`Type`）：声明本身
  仍会 `decl.checked`，但这个名字永远不会被用到。
- `redundant-sorry`：值其实已经把目标证完了，那个 `sorry` 是**多接在后面
  的一个实参**（删掉这一行声明就能过内核）。它**仍然照常 `exercise.open`**：
  要说的是"删掉这一行"，不是"你还没证出来"；span 收窄到那个 `sorry` token
  （`docs/design/redundant-sorry.md`）。
  多文件项目里还有 `import-has-open-exercises`：依赖模块仍留着 `sorry` 时，
  入口的 `import` 行得到这条 warning（依赖里没解出的名字不会进入口环境）。
- 多文件项目（0.57.0）的诊断带 `file`/`module` 字段，属于**别的文件**；`stage`
  为 `import`（`import-not-found`/`import-cycle`/`import-dependency-failed`/
  `import-name-collision`/`import-prelude-conflict`/`import-module-invalid`）或
  `parse` 的 `import-malformed`/`import-not-a-valid-module-name`/
  `import-must-precede-declarations`/`manifest-invalid`。判卷时先修依赖文件，
  入口的 `import-dependency-failed` 会随之消失。
- `elab-*` 错误码封闭清单见 `docs/protocol.md`（doc-conformance 测试守护）。
- **`exercise.open` 只说明"这是个练习"，它不保证签名没坏**（G-01 / 0.59.0 起，
  反过来：签名坏就**不会**出现 `exercise.open`）。签名 elaborate 不了、不是一个类型、
  或 `theorem` 的不是 Prop ⇒ 一条 `diagnostic`（`elab-unknown-identifier` /
  `kernel-expected-sort` / `kernel-theorem-not-prop`）+ 声明 `failed`，**span 落在
  签名上**（不是值位）。判卷永远看 `diagnostic`，不要用 `exercise.open` 计数推断
  "签名没腐烂"。

## 判卷读法（伪代码）

```
events = run(`sokonanoda --json canvas.sokonanoda`)
for e in events:
  if e.type == "decl.checked" and e.name in 我的练习名: 记为解出
  if e.type == "exercise.open":        记为待作答（e.name 可能为 null：example）
                                       —— 走到这里说明**签名已经受检且合法**
  if e.type == "diagnostic":           按 code 分类（见 SKILL.md §3 决策表）
                                       e.hint 是给学习者的第一句话
                                       span 在签名上 ⇒ 先修签名（不是"还没做"）
```

## 增量语义（服务层事实）

- 会话记住每个命令的快照：**文本未变的命令不重查内核**；`recompiled_from`
  = 首个被重查命令的索引（`null` = 整份未变）。
- watch 流的 delta 只报状态**变化**（opened/solved/failed/failed-checked）；
  `version` 单调递增，按版本门闩消费。
- 前端 API 层（`front::session`）还暴露 `stats.kernel_checks`：
  改第 i 个声明 ≈ 只内核重查 `n-i` 条（I8 验收指标）。
