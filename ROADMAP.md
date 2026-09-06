# sokonanoda-lang —— `.fol` 协作式 Lean 4 教学 ROADMAP

> 状态：draft
> 基线：sokonanoda `7b51784`
> 日期：2026-09-06

## 0. 终极形态

> 用户与 code agent 共同看着 VS Code 中打开的同一个 `*.fol` 文件。
> agent 从零开始逐段定义概念、写出示例、抛出练习；
> 用户在文件里作答；
> 我们自己的编译器实时给出反馈——**同一份反馈既给用户看，也给 agent 看**，
> agent 据此决定下一步教什么、怎么调整题目。

```
              ┌──────────── VS Code ────────────┐
              │  *.fol（共享画布）        │
              │  agent 写定义 / 出题             │
              │  用户作答 / 阅读解释              │
              └──────────────┬──────────────────┘
                             │ 文件变更 / 事件
                             ▼
              sokonanoda-lang compiler service（长期运行）
              完整 kernel + 受限前端 + 练习引擎
                             │
          ┌──────────────────┼──────────────────┐
          ▼                  ▼                  ▼
     用户可见反馈         agent 可见反馈      内部状态
     （类型/化简/         （结构化事件，      （定义、目标、
      对错/提示）          供 agent 推理）     进度、诊断）
```

“万里长城第一步”：上面的 UI、agent、实时通道都先不做。
**第一层编译器**先独立成立：它不依赖 VS Code、不依赖任何 agent，
但已经能解析 `.fol`、驱动完整 kernel、判定练习并输出结构化事件。

---

## 1. 不可动摇的原则

1. **kernel 保持完整。** 完整迁移 sokonanoda 内核（含 inductive、quot、proof irrelevance、完整 conv/eval、pretty printer）。受限的只是教学语法通道，不是内核能力。
2. **无官方工具依赖。** 运行、构建、测试都不调用 `lean` / `lake` / `lean4export` / `leanc` / `elan`；所有语料与测试入库。
3. **教学语法是真实 Lean 4 的子集。** 学生在 `.fol` 中学到的写法，放到官方 Lean 中依然合法。
4. **语法白名单即课程。** parser 只支持课程已引入的语法点，每个新语法必须有对应课程单元。
5. **分层推进，先 L0 后 L1/L2/L3。** 每层只依赖下一层，不在 L0 阶段做任何编辑器或 agent 集成。

---

## 2. 分层架构

```text
L3  协作层（远期）
     code agent：依据编译器反馈决定教学内容、讲解、出题与答疑

L2  编辑层（远期）
     VS Code extension：打开 *.fol，双角色视图，
     用户作答区 / agent 输出区 / 实时诊断 / 练习状态

L1  服务层（中期）
     长期运行的 compiler service + 事件协议：
     监听文件变更，增量重编译，广播结构化反馈

L0  编译器层（现在只做这层）
     完整 kernel（sokonanoda 核心）
     + .fol 前端（lexer/parser/小型 elaborator/练习引擎）
     + CLI/批处理：输入文件 → 输出结构化结果与事件流
```

每一层只依赖下一层公开的接口。L0 必须先能回答：

- 这个定义是否通过 kernel 检查？
- 这个表达式的类型是什么？化简结果是什么？
- 这道练习对了吗？错在哪一段、缺哪个概念？
- 整个文件当前处于什么状态（已定义/已检查/待作答/已完成）？

这些问题全部以**结构化的检查事件**表达，而不是混在 human-readable 文本里。

---

## 3. `.fol` 文件：共享画布

格式草案（M1 细化，原则如下）：

```text
-- 课程内容：文字、讲解与 agent 的“讲课脚本”可以写在文件里
lesson: 函数与箭头

def add1 : Nat → Nat := fun n => n + 1

#check add1
-- 期望输出：add1 : Nat → Nat

#exercise "恒等函数"
-- 目标：补全右侧，使该声明通过检查
example : Nat → Nat := ???

-- 用户作答写在这里
```

`.fol` 本身同时承担四种角色：

1. **教学内容载体**：概念卡、讲解、示例代码都留在文件里，agent 与用户共同观看；
2. **agent 的工作面**：agent 一边写定义、一边插入练习；
3. **用户作答区**：用户在指定位置补全/修改代码；
4. **编译器输入**：任何时刻文件都可以被解析、增量检查并产出反馈。

因此文件格式必须：

- 允许“正文/代码/练习/答案区”交错出现；
- 允许未完成练习（`???`、空答案）仍是合法状态，不导致整文件编译失败；
- 支持增量：只重编译被修改片段，并保持已检查定义不变；
- 每个片段可定位到行列，供用户和 agent 共同引用；
- 保留纯文本可读性，方便 agent 用语言模型直接读写。

---

## 4. 第一层编译器（L0）的边界

### L0 包含

- 完整 kernel 及其公开 API；
- 自带最小 prelude（Nat、Bool、Prop、Eq 等），运行时不依赖任何外部导出；
- `.fol` lexer / parser（带 span）；
- 受限 elaborator（只覆盖教学白名单）；
- 练习引擎（目标、答案区、对错判定、提示分类）；
- CLI / 批处理与结构化事件输出；
- 完整回归测试：kernel 测试集 + 课程语料。

### L0 不包含

- VS Code extension；
- 编译器长驻服务 / LSP；
- code agent；
- 网络与多人协作；
- notation / macro / typeclass / 完整 tactic（课程未到之前不进入白名单）。

L0 的正确形态是一个**能被任何调用方（CLI、LSP、agent、测试）使用的编译器库 + 命令行前端**。

---

## 5. 里程碑

### M0 —— kernel 完整迁移与稳定 API（0.1）

> 进度（2026-09-06）：
> - workspace `sokonanoda-lang` 已建立，kernel 完整快照迁入 `crates/kernel`；
> - 完整测试基线通过：40 个内核单元测试 + arena 集成测试 + 首个内存 API 测试；
> - 上游两个缺少 fixture 的测试已隔离并注明原因（需重建 NDJSON fixture，不是跳过内核能力）；
> - 已新增 `Config::default`、`ExportFile::empty`、`infer_closed_type`、`reduce_closed` 等内存 API。

目标：完整保留 sokonanoda 内核，并提供教学前端需要的稳定接口。

- 建 workspace：`kernel`、`course`、`compiler`、`cli`。
- kernel 完整迁移，不做功能裁剪；现有测试全量迁移并保持全绿（LKA arena 测试完整保留）。
- 公开 API：
  - 在内存中构造/添加声明；
  - `check_expr → Result<Type, KernelError>`；
  - `reduce_expr → Expr`；
  - 完整环境检查能力保留；
  - 错误用显式类型表达，不再靠 panic。
- 修复两个因缺失测试资源而失败的测试。

验收：`#check (fun x : Nat => x + 1)` 在内存中完成推断与显示；kernel 完整测试集全绿。

---

### M1 —— `.fol` 格式与编译器前端 v0（0.2）

目标：文件第一次能“翻译”成 kernel 可检查的内容。

- 定义 `.fol` 格式 v0：
  - 正文（`--` 注释与 lesson 元数据）；
  - `def` / `example` / `#check` / `#reduce`；
  - `#exercise` 与答案区标记。
- 实现 lexer + parser（span 全程保留）。
- 实现 v0 elaborator：
  - 显式 binder、应用、Nat 字面量、`fun`；
  - 答案区允许 `???` 作为合法“未完成”状态；
  - 任何不支持的语法产生“课程级别不可用”，而不是内部错误。
- 批处理 CLI：`sokonanoda-lang check lesson.fol`。
- 输出两种视图：
  - 人类可读文本；
  - JSON Lines 事件流（供未来的 service/agent 直接消费）。

验收：课程 0 示例文件能完整检查，`???` 的未完成练习不导致崩溃，事件流含行号。

---

### M2 —— 最小 prelude 与练习引擎（0.3）

目标：第一道真正“可判对错”的练习出现。

- 自带最小 prelude：
  - Nat（数字、加法）、Bool、Prop、Eq、`rfl` 所需结构；
  - kernel 名字特判与 prelude 对齐。
- 练习引擎：
  - 练习定义（目标类型/期望化简/期望 #check 类型）；
  - 作答区解析；
  - 判定通过 kernel，不靠文本比对；
  - 错误分类：解析错误 / 类型不匹配 / 未完成 / 化简不符。
- 为每个错误分类提供第一版教学提示模板。

验收：用户补全“恒等函数”后正确；补成 `fun n => n + 1` 时收到“类型不匹配，目标应为返回 n”级别的反馈。

---

### M3 —— 实时事件模型（0.4）

目标：为“同时给用户和 agent 反馈”打下协议基础，但暂不做编辑器。

- 定义编译器事件协议 v1：
  - `file.didChange`；
  - `decl.added` / `decl.rejected`；
  - `expr.type` / `expr.reduce`；
  - `exercise.updated` / `exercise.solved` / `exercise.failed`；
  - `diagnostic.*`（含 span 与错误分类）。
- 同一份事件提供两种渲染：
  - 用户视图（自然语言）；
  - agent 视图（结构化 JSON，含状态、目标、错误分类，供 agent 决策下一步）。
- 支持“文件追加一行就重编译对应片段”的增量逻辑（先做整文件级 + 片段缓存）。

验收：模拟“agent 添加定义 + 出题、用户作答、agent 修改题目”三步，事件流能完整反映每一步状态变化。

---

### M4 —— 完整 kernel 驱动的入门课程（0.5）

目标：内容层跑通第一门课。

- 课程路径（每步都受白名单约束）：
  1. 表达式与类型；
  2. 函数与箭头；
  3. 命题与证明项；
  4. 等式与 `rfl`；
  5. 归纳类型与 `match`（elaborator 到位后引入）。
- 每课内容作为 `.fol` 文件入库；
- 每课含 3~8 个练习；
- 课程自检：CI 跑通全部 `.fol` 文件并比对 golden 事件。

验收：零基础用户按顺序完成课程；每课结束时的 `.fol` 都是合法可检查文件。

---

### M5+ —— 编辑器与 agent（后续，不进入当前阶段）

- **L1**：compiler service 常驻、增量重编译、与编辑器通过事件协议通信。
- **L2**：VS Code extension（`.fol` 语法、双角色视图、实时诊断与练习状态）。
- **L3**：code agent 接入同一事件流：从零讲课、出题、根据反馈调整。

---

## 6. 外部依赖与仓库策略

- 硬规则：不调用官方 Lean 工具链；CI 离线可跑；测试输入全部入库。
- 允许少量审计过的 Rust crate；kernel 不因教学而裁剪功能或依赖。
- 另立仓库，用 git 摘取 sokonanoda `7b51784` 快照；kernel 作为独立 crate 保留完整历史与测试。

---

## 7. 风险与对策

| 风险 | 对策 |
|---|---|
| 编译器悄悄长成“第二套 Lean” | 白名单语法 + 每语法点必须有课程；L0 先定边界 |
| 教学子集与真实 Lean 不一致 | 规则：必须是真实 Lean 4 的子集 |
| “实时反馈”在 L0 阶段做过头 | L0 先做批处理事件流，服务层留给 M5+ |
| kernel 被教学需求带偏 | kernel 完整独立，前端只消费公开 API |
| 练习格式设计错导致 agent/UI 返工 | M1 先定义文件格式与事件模型，内容与协议分离 |
| 事件流只对人友好、agent 不可读 | 每条事件同时带 human 与 machine 两种表示 |

---

## 8. 当前只需要做的一件事

执行 M0：建仓并完整迁移 kernel，跑绿全部现有测试，公开 `check_expr` / `reduce_expr` /
声明添加 / 显式错误 API。这是后续所有层的地基，不依赖任何 UI、agent 或官方 Lean。
