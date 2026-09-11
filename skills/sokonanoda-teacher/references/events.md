# 事件词汇与 JSON 形状（teacher 参考）

> 权威来源：`docs/protocol.md`（协议）、`crates/cli/src/json_report.rs`（实现）。
> 本文件是判卷视角的浓缩版 + 一段真实事件流。

## 两种机器视图

| 视图 | 命令 | 词汇 | 用途 |
|---|---|---|---|
| 批处理 `--json` | `sokonanoda --json <file>` | `decl.checked` `example.checked` `expr.typed` `expr.reduced` `decl.printed` `exercise.open` `diagnostic` `warning` | 一次判卷的完整事件流（agent 主用） |
| watch 流 | `sokonanoda watch <file>` | `file.changed` + delta（`decl.checked`/`decl.failed`/`exercise.opened`/`exercise.solved`/`exercise.failed`）+ `diagnostic` | 常驻监听：每版只推状态变化 |

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
```

- `span` 是 1-based 行列 + 字节 offset；`--json` 的所有诊断带 `code` 与 `hint`。
- `warning` 不改变退出码、不把文件判成错误。`reserved-declaration-name`
  表示顶层声明名撞上了内核已定义的名字（`Prop`/`Sort`/`Type`）：声明本身
  仍会 `decl.checked`，但这个名字永远不会被用到。
- `elab-*` 错误码封闭清单见 `docs/protocol.md`（doc-conformance 测试守护）。

## 判卷读法（伪代码）

```
events = run(`sokonanoda --json canvas.sokonanoda`)
for e in events:
  if e.type == "decl.checked" and e.name in 我的练习名: 记为解出
  if e.type == "exercise.open":        记为待作答（e.name 可能为 null：example）
  if e.type == "diagnostic":           按 code 分类（见 SKILL.md §3 决策表）
                                       e.hint 是给学习者的第一句话
```

## 增量语义（服务层事实）

- 会话记住每个命令的快照：**文本未变的命令不重查内核**；`recompiled_from`
  = 首个被重查命令的索引（`null` = 整份未变）。
- watch 流的 delta 只报状态**变化**（opened/solved/failed/failed-checked）；
  `version` 单调递增，按版本门闩消费。
- 前端 API 层（`front::session`）还暴露 `stats.kernel_checks`：
  改第 i 个声明 ≈ 只内核重查 `n-i` 条（I8 验收指标）。
