# V2EX · 分享创造 — v0.100.0

Status: **ready to post**. Owner posts from their account; see `README.md` for
ordering and the claim ledger.

- **节点：** 分享创造
- **标题：** `Sokonanoda：不用装 Lean 的 Lean 风格证明语言（VS Code 插件 + 三门课）`
- **配图：** `assets/zh-cover.png`（题图）、`assets/ph-2-kernel.png`、`assets/demo-vscode.gif`

## 正文

```markdown
做了一个形式化证明的教学语言 Sokonanoda，想听听大家的意见。

## 为什么做

教人写证明时，第一步总是装 Lean、配 elan、找 mathlib，真正写证明之前先花掉一晚上。
我想做一个「装个 VS Code 插件就能写」的东西：打开课程文件，光标停在 `sorry` 上，
右边就显示内核算出来的目标；填完保存，由真内核判定，不是文本匹配。

## 它是什么

- Lean 4 风格的教学子集，文件后缀 `.sokonanoda`
- VS Code 扩展自带 LSP 和 CLI（macOS / Linux / Windows，8 个平台包），
  不用装 Lean、不用装 Rust，装完离线可用
- 三门课：入门逻辑（11 单元，有英文镜像）、集合论（112 单元）、
  实分析（175 单元），共 298 单元 / 2,022 道练习，全部有内核验证过的解答
- 另有 409 道选择题 / 195 个知识点
- 判卷和目标走同一份内核输出：`--json` 里有 `decl.checked` / `exercise.open` / `diagnostic`

## 技术底子

内核是 [intgrah/sokonanoda](https://github.com/intgrah/sokonanoda) 的 fork
（Apache-2.0，Rust 实现的 Lean 4 内核）；教学前端、LSP、CLI、课程和工具链是新的。
Lean Kernel Arena `master@30e64c1b2` 上 135/138 accept、90/92 reject
（3 个性能超时、2 个 known unsupported，均在仓库登记）。

## 先说清楚边界

- **不是 Lean**：没有 mathlib，不能 import Lean 文件，tactic 只有课程用到的那些
- 课程中文优先；入门课有英文镜像，后两门暂时没有
- 适合学证明、教学、课程作业，不适合做真正的研究级形式化

## 怎么开始

- 官网（中英双语）：https://colorlessboy.github.io/sokonanoda-lang/
- VS Code Marketplace 搜 `sokonanoda`
- 仓库（Apache-2.0）：https://github.com/ColorlessBoy/sokonanoda-lang

欢迎拍砖，尤其是教学流程：哪一步容易卡住、下一步该给什么提示。
```

## 发帖备注

- 先确认账号可以在「分享创造」发帖；不要在多个节点重复发同一篇。
- 不要自顶、不要找朋友刷感谢；V2EX 对这类行为很敏感。
- 技术质疑优先回在帖子里，能复现的问题直接给 issue 链接。
- 如果帖子被移到其他节点，不要重发；跟着节点走。
