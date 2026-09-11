# 调研：教学型形式化证明语言与基础设施

> 目的：为 `.sokonanoda` 的 L0–L3 决策提供外部参照系。结论见文末"对我们项目的启示"，
> 逐条落实到 `docs/design/infrastructure.md`。
> 调研时间：2026-09-06。所有外部项目只做定性比对，数字均为仓库自述。

---

## 1. 外部参照系一览

### 1.1 Lean 系教学平台：NNG / lean4game / lean4-game-server

- **lean4game**（leanprover-community，adam.math.hhu.de 托管）：把 Natural Number Game 移植/扩展到 Lean 4 的服务器。
  - 内容模型：**World → Level**。一个 level 就是一个 Lean 文件：`Game`/`World`/`Level` 元数据 + `Statement`（要证的定理）+ **示例解法**（tactic proof）+ 挂在特定 goal state 上的 `Hint`。
  - 判定方式：服务端用**完整 Lean 编译器**把学生的 tactic proof 编译，成功即过关；不比较文本。
  - World 依赖顺序由各 level 的示例解法自动推导。
  - 服务器是长驻进程（watchdog + client task / ServerEvent），把 goal state 增量推给前端。
  - 教学反馈：**Hint 绑定在 goal state 上**（"当 goal 长这样时给这条提示"）。
- **lean4-game-server**（PatrickMassot）：早期原型，同样是"game 框架 = Lean 元编程 + level 文件 + runGame"。
- 启示：① 判定靠编译器/内核而非文本；② 提示系统以 goal state 为键；③ 内容 = 数据（level 文件），不是写死在代码里。

### 1.2 专门为教学造的证明助手

- **SASyLF**（"Sassy Elf"）：LF 系、专教编程语言理论的证明助手。哲学是"少而明确"：学生显式写出规则应用，系统把错误定位到单条规则应用上，学习曲线平缓，专用于 PL 理论课程（类型安全等）。
- 启示：**教学工具可以只覆盖一个小而全的领域**，把"错误定位到最小可解释步骤"当作第一等体验。

### 1.3 洞/交互开发作为教学法：Agda

- Agda 把 `?`/`{! !}` 洞当一等公民：编辑器给 goal/type，学生逐步 refine。
- 与 `.sokonanoda` 的 `???` 思想同源：**"文件里留着未完成洞是合法状态"**，编译器围绕洞给反馈。
- 启示：洞 + goal 视图 + 逐步填洞是成熟的教学交互，L0 的 `example : T := ???` 只是第一步；后续要给洞更多结构（目标类型、期望化简）。

### 1.4 "完整内核 + 受限前端"在学术/社区中的先例

- **类型检查教程/移植链**：sokonanoda 上游来自 nanoda_lib/sonanoda/still-nanoda；本地另有 nanobruijn（Haskell）与 py_nanobruijn（Python）移植，以及 "Type Checking in Lean4"（Chris Bailey）等资料。共同点：**拿官方导出（NDJSON）喂给独立实现的内核**，用基准语料验证一致性。
- **py_nanobruijn（本地）**：Python 版内核之外还长出了完整教学层——teaching REPL、fol/*.fol 世界语料、proof 状态、reduce 分步、LSP/server 草案、sessions。`.sokonanoda` 的 fol-basics / py-fol-core / py-nat 正是从 `py_nanobruijn/teaching/fol/*.fol` 移植的（Rust 前端目前只支持这些语料的子集）。
- 启示：教学层与内核分层、语料可移植、跨语言同构验证，是本仓库的路。

### 1.5 智能教学系统（ITS）的"模型追踪"

- EvoLogic / ActiveMath(Ωmega) 这类系统做 **model tracing / proof planning**：维护学生解答空间，把"卡住"映射到概念缺口。
- 启示：`diagnostic → 概念缺口 → 提示` 是可工程化的三层结构；L0 先做"错误分类"，L1/L3 再挂"提示模板与概念映射"。

### 1.6 编译器诊断与协议工程

- 主流做法（LSP 的 `publishDiagnostics`、Lean 4 的 LSP、Bazel BEP 等）：诊断带 severity/code/range，事件可增量；agent 直接消费结构化反馈。
- 本地 py_nanobruijn 也已有 `lsp/`、`services/checker.py`、`results.py`（CheckResult/Diagnostic）先例。
- 启示：**同一反馈的 human 视图与 machine 视图必须一起设计**；本仓库 `docs/protocol.md` + CLI `--json` 就是这个方向的第一版。

---

## 2. 横向对比表

| 维度 | lean4game/NNG | SASyLF | Agda（教学用法） | py_nanobruijn teaching | **sokonanoda-lang（现状/目标）** |
|---|---|---|---|---|---|
| 内核 | 官方 Lean（完整） | 自己的 LF 检查器 | 官方 Agda | 自己的 Python Lean4 kernel | 完整移植的 sokonanoda kernel |
| 与官方 Lean 语法关系 | 完全一致（含 tactic） | 自有 LF 语法 | 完全一致 | 官方子集 | **官方子集的受限白名单** |
| 练习载体 | Level 文件（Game 命令） | 文件+推导 | 文件+洞 | fol/*.fol + REPL | `.sokonanoda`（正文+代码+`???`） |
| 判定 | 服务端编译 proof | 检查证明 | 检查 | kernel 检查 | kernel 检查（已实现） |
| 学生书写 | tactic proof | 显式推导规则 | 项+洞 | term + 草案 tactic | term（`#prove` 草案 tactic） |
| 提示/反馈 | Hint 绑定 goal state | 错误定位到规则应用 | goal/type 视图 | REPL 文本 + 报告 | 文本 + JSON 事件（新）；提示模板待做 |
| 增量/服务 | 长驻 server | 批处理 | IDE | LSP/server 草案 | REPL 累积 + 批处理；service 是 M3+ |
| 离线/无官方工具 | 否（依赖 lean） | 是 | 否 | 是 | **是（硬规则）** |

---

## 3. 我们项目的关键差异与定位

1. **`无官方工具依赖` 是护城河**：NNG 系要整套 Lean 工具链；我们可以把 kernel、prelude、语料全部入库，CI 离线可跑 → 教学内容可复现、可审计、不怕上游版本漂移。
2. **`教学语法 = 真实 Lean 4 子集` 是兼容性承诺**：学生学到的东西不失效。这要求每个语法点的"白名单→课程→测试→golden 事件"四件套。
3. **term-first + tactic 草案**：大多数教学系统二选一；`.sokonanoda` 让用户先看到"证明就是构造 lambda"，`#prove` 则演示 tactic 只是搭 lambda 的脚手架——两条路线在同一内核上收敛，是教学上少见的统一视角。
4. **反馈同时给人给 agent**：协议层设计（docs/protocol.md）保证 L1 service / L2 编辑器 / L3 讲课 agent 都能消费同一事件流，避免"只对人友好"的返工。

---

## 4. 对我们项目的启示（结论）

1. 判定永远走 kernel；练习元数据（目标类型/期望化简/期望 `#check`）也要 kernel 可判，不做文本比对。
2. 提示模板以 **error taxonomy × 概念缺口** 组织，先出 v1 模板（M2），再演进为 goal-state 绑定式 Hint（对标 lean4game）。
3. 内容 = 数据：lesson/world 清单、golden 事件都入库；新增课程 = 新增 `.sokonanoda` 文件 + golden（CI 已开始强制这一点）。
4. 洞要更结构化：`example : T := ???` 只是 L0 的洞；后续至少支持"洞附带期望类型/期望化简值"和"多个洞按序填"。
5. 结构化事件（JSON Lines + span + stage/code）现在就做，因为 L1/L2/L3 都消费它；`--json` 是本协议的第一个实现。
6. 错误分类先分阶段（parse/elab/kernel），再细化到概念级；kernel panic→显式错误的改造是长期任务，别让教学前端等它。
7. 增量最小模型 = REPL 的"声明累积 + 整 buffer 重编译"；M3 服务层在此基础上做片段缓存，不另起炉灶。
8. 教学叙事（为什么 Or.rec 不用算、Nat.rec 必须算等）与代码一样是产品的一部分，应以 `.sokonanoda` 注释/正文形式入库（py_nanobruijn 的 fol 注释已是范例）。
