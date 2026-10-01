# sokonanoda-lang —— `.sokonanoda` 协作式 Lean 4 教学 ROADMAP

> 状态：active（v2 LSP-first；**接手与"下一步"先看 `docs/ONBOARDING.md`**，最新快照看 `STATUS.md`，
> 本文 §10 = 待办的**验收口径 + 收口状态**）
> 基线：v0.79.0（2026-10-01；本文只描述计划与验收，已完成的条目就地打勾并标注版本）
> 配套文档：`STATUS.md`（当前状态与进度日志，agents 先读）、
> `docs/architecture.md`（深度理解）、`docs/archive/notes-2026-09-26/research.md.gz`（外部调研，已归档）、
> `docs/design/infrastructure.md`（基础设施方案脑暴）、`docs/protocol.md`（事件协议）。

> 方向更新（2026-09-06 续）：编辑器形态下 `.sokonanoda` 是**纯声明式文件**
> （无 `#` 命令）；练习 = 带 `???` 洞的 `def name : T` / `theorem name : T` /
> `example : T` 声明；反馈通道是**细粒度 LSP**（hover 类型/化简、精确诊断、
> 练习状态、goal 视图）。CLI/REPL 的 `#check` 等只是调试与自测工具。
> 详见 `docs/design/infrastructure.md`（LSP-first 设计 v2）。

## 0. 终极形态

> 用户与 code agent 共同看着 VS Code 中打开的同一个 `*.sokonanoda` 文件。
> agent 从零开始逐段定义概念、写出示例、抛出练习；
> 用户在文件里作答；
> 我们自己的编译器实时给出反馈——**同一份反馈既给用户看，也给 agent 看**，
> agent 据此决定下一步教什么、怎么调整题目。

```
              ┌──────────── VS Code ────────────┐
              │  *.sokonanoda（共享画布）        │
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
但已经能解析 `.sokonanoda`、驱动完整 kernel、判定练习并输出结构化事件。

---

## 1. 不可动摇的原则

1. **kernel 保持完整。** 完整迁移 sokonanoda 内核（含 inductive、quot、proof irrelevance、完整 conv/eval、pretty printer）。受限的只是教学语法通道，不是内核能力。
2. **无官方工具依赖。** 运行、构建、测试都不调用 `lean` / `lake` / `lean4export` / `leanc` / `elan`；所有语料与测试入库。
3. **教学语法是真实 Lean 4 的子集。** 学生在 `.sokonanoda` 中学到的写法，放到官方 Lean 中依然合法。
4. **语法白名单即课程。** parser 只支持课程已引入的语法点，每个新语法必须有对应课程单元。
5. **分层推进，先 L0 后 L1/L2/L3。** 每层只依赖下一层，不在 L0 阶段做任何编辑器或 agent 集成。
6. **TDD 与重复测试。** 每个语法点/命令先写测试再实现；同一行为要在单元、kernel 端到端、CLI 三层重复覆盖。
7. **反馈即功能。** 编译器和 CLI 必须输出类型、化简、打印与环境状态，让人和大模型在无文档情况下也能驱动工具。

---

## 2. 分层架构

```text
L3  协作层（远期）
     code agent：依据编译器反馈决定教学内容、讲解、出题与答疑

L2  编辑层（远期）
     VS Code extension：打开 *.sokonanoda，双角色视图，
     用户作答区 / agent 输出区 / 实时诊断 / 练习状态

L1  服务层（中期）
     长期运行的 compiler service + 事件协议：
     监听文件变更，增量重编译，广播结构化反馈

L0  编译器层（现在只做这层）
     完整 kernel（sokonanoda 核心）
     + .sokonanoda 前端（lexer/parser/小型 elaborator/练习引擎）
     + CLI/批处理：输入文件 -> 输出结构化结果与事件流
```

每一层只依赖下一层公开的接口。L0 必须先能回答：

- 这个定义是否通过 kernel 检查？
- 这个表达式的类型是什么？化简结果是什么？
- 这道练习对了吗？错在哪一段、缺哪个概念？
- 整个文件当前处于什么状态（已定义/已检查/待作答/已完成）？

这些问题全部以**结构化的检查事件**表达，而不是混在 human-readable 文本里。

---

## 3. `.sokonanoda` 文件：共享画布

格式草案（M1 细化，原则如下）：

```text
-- 课程内容：文字、讲解与 agent 的“讲课脚本”可以写在文件里
lesson: 函数与箭头

def add1 : Nat -> Nat := fun n => n + 1

#check add1
-- 期望输出：add1 : Nat -> Nat

#exercise "恒等函数"
-- 目标：补全右侧，使该声明通过检查
example : Nat -> Nat := ???

-- 用户作答写在这里
```

`.sokonanoda` 本身同时承担四种角色：

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
- `.sokonanoda` lexer / parser（带 span）；
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
> - 已新增 `Config::default`、`ExportFile::empty`、`infer_closed_type`、`reduce_closed` 等内存 API；
> - 已新增 `EnvBuilder`：在 arena 内先构造声明、最后一次性产出 `ExportFile`，绕开了 `TcCtx` 自引用生命周期问题；
> - 已新增 `crates/front`（`.sokonanoda` lexer/parser/诊断/elaborator）与 `crates/cli`（`sokonanoda`），见 M1 进度。

目标：完整保留 sokonanoda 内核，并提供教学前端需要的稳定接口。

- 建 workspace：`kernel`、`course`、`compiler`、`cli`。
- kernel 完整迁移，不做功能裁剪；现有测试全量迁移并保持全绿（LKA arena 测试完整保留）。
- 公开 API：
  - 在内存中构造/添加声明；
  - `check_expr -> Result<Type, KernelError>`；
  - `reduce_expr -> Expr`；
  - 完整环境检查能力保留；
  - 错误用显式类型表达，不再靠 panic。
- 修复两个因缺失测试资源而失败的测试。

验收：`#check (fun x : Nat => x + 1)` 在内存中完成推断与显示；kernel 完整测试集全绿。

---

### M1 —— `.sokonanoda` 格式与编译器前端 v0（0.2）

> 进度（2026-09-06）：
> - `.sokonanoda` v0 语法与 lexer/parser 已实现：`def` / `theorem` / `example` / `axiom` / `#check` / `#reduce` / `???`、lambda、箭头、`∀`、应用；
> - span 诊断已带行列号；命令关键字不会泄漏进表达式；
> - `EnvBuilder` 可把 AST elaborate 成 kernel 声明；`ExportFile::try_check_declar` 以显式错误替代 panic；
> - `sokonanoda` CLI 已端到端工作：`.sokonanoda` -> AST -> kernel 声明 -> 完整 kernel 检查 -> 人读结果；
> - `examples/lesson-01.sokonanoda` 与 `lesson-02.sokonanoda` 已入库。
> - 数字字面量/最小 Nat 基元与 `#reduce` 已接通（`1 + 1` 可化简为 `2`）。
> - `examples/fol-basics.sokonanoda`：False/True/Not/And/Or/Iff/Exists/Forall 从零定义，
>   and_comm/or_comm/absurd/双重否定/存在证人/forall 组合均通过完整 kernel 检查。
> - 教学文件/交互统一使用 ASCII `->`；`sokonanoda repl` 支持逐行累积声明并即时
>   `#check` / `#reduce`（调试用最小 REPL）。
> - `Sort n` 数字宇宙可用（`Prop = Sort 0`、`Type = Sort 1`），函数类型本身的类型可被
>   内核检查（如 `(fun (α : Sort 2) => α) : Type 1 -> Type 1`）。
> - `Sort u` 宇宙变量、声明级 `{u, v}` 参数与显式应用 `id.{u}` 已实现；普通使用默认
>   universe = 0；axiom/def/theorem 均可带宇宙参数。
> - `@id.{u}` 语法别名与 `{α : Sort u}` 隐式 binder 已实现；`py-fol-core.sokonanoda`
>   从 py_nanobruijn 的 fol 片段移植并在 front + CLI 双层测试。
> - 命名箭头 `(x : A) -> B` / `{x : A} -> B` 等价于带 binder 的 `forall`；
>   `py-fol-core.sokonanoda` 已全量改写为箭头风并通过检查。
> - `#print <name>` 已可用；kernel 拒绝输出不再刷 panic trace。
> 未完成：归纳类型的真正声明语法、Nat.rec/induction、tactic 草稿模式与课程 UI。

目标：文件第一次能“翻译”成 kernel 可检查的内容。

- 定义 `.sokonanoda` 格式 v0：
  - 正文（`--` 注释与 lesson 元数据）；
  - `def` / `example` / `#check` / `#reduce`；
  - `#exercise` 与答案区标记。
- 实现 lexer + parser（span 全程保留）。
- 实现 v0 elaborator：
  - 显式 binder、应用、Nat 字面量、`fun`；
  - 答案区允许 `???` 作为合法“未完成”状态；
  - 任何不支持的语法产生“课程级别不可用”，而不是内部错误。
- 批处理 CLI：`sokonanoda-lang check lesson.sokonanoda`。
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
- 每课内容作为 `.sokonanoda` 文件入库；
- 每课含 3~8 个练习；
- 课程自检：CI 跑通全部 `.sokonanoda` 文件并比对 golden 事件。

验收：零基础用户按顺序完成课程；每课结束时的 `.sokonanoda` 都是合法可检查文件。

---

### M5+ —— 编辑器与 agent（后续，不进入当前阶段）

- **L1**：compiler service 常驻、增量重编译、与编辑器通过事件协议通信。
- **L2**：VS Code extension（`.sokonanoda` 语法、双角色视图、实时诊断与练习状态）。
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

> ⚠ **本节是 2026-09-06 的历史快照**（当时的答案 = I11 余项 S2–S4）—— **I11 已废弃** ✗，
> **现行答案 = `docs/ONBOARDING.md` §0.2 的唯一队列** ✓（保留原文只为查旧账）。

**I11 余项（真人输入测试 S2–S4）**——基建与关键缺陷修复（S0/S1）已落地，
剩余 F1–F5 / L3–L8 / V2–V4 见 `docs/design/infrastructure.md`。M0（kernel 迁移）等早期里程碑均已完成。

---

## 9. 当前执行清单（按顺序完成）

> ⚠ **本清单（2026-09-06）九条已全部完成** ✓；**现行队列 = `docs/ONBOARDING.md` §0.2** ✓
> —— 本节与下面的"第 7 条进度 / 本轮 / 第二轮"都是**当时的进度日志**，保留以便查旧账。

1. [x] M0 kernel 完整迁移与公开 API
2. [x] M1 `.sokonanoda` 前端 + CLI/REPL 端到端
3. [x] ASCII `->`、命名箭头 `(x : A) -> B`、`Sort n` / `Sort u` / `{u}` / `id.{u}` / `@id.{u}`
4. [x] py fol 移植：basic/true/false/and/or/not/iff/exists + Eq/propext + Eq_symm/Eq_trans
5. [x] py `theorems.fol` 剩余可移植命题（含 congrArg 宇宙边界、or_comm 全量、and_imp、not_imp、mt 全量）
6. [x] `#prove` / tactic 草稿（goal state、intro/exact/apply/assumption + partial lambda 回显）
7. [x] `inductive Nat` 声明语法 + `Nat.rec`（iota 归约）
8. [x] `nat.fol` 移植与 iota 端到端测试
9. [x] 课程/agent/编辑器层的设计文档与协议草案（docs/protocol.md）

### 第 7 条进度

- 已支持源码块：`inductive ... / ctor ... / rec ... / iota ... / end`
- 零规则与深层归约都已通过：kernel 新增 `deep_reduce`，`s (Nat.rec ...)` 内
  的再次 iota 可继续算（commit dac8e80）；`examples/py-nat.sokonanoda` 显式
  `inductive Nat` 块 + iota 规则端到端通过，`add two two` 化简为 succ 链。
- 说明：显式 `Nat` 块会覆盖内置 prelude（`compile.rs` 先探测文件里是否有同名块）。


---

### 本轮基础设施进度（2026-09-06 续）

1. [x] 文档：`docs/architecture.md`（架构与内核深度理解）、`docs/archive/notes-2026-09-26/research.md.gz`
      （外部调研，已归档）、`docs/design/infrastructure.md`（基础设施设计脑暴）、
      `docs/protocol.md` 刷新为"文本 + JSON Lines"双视图协议。
2. [x] CLI `--json`：每条事件一行 JSON（`decl.checked` / `expr.typed` /
      `expr.reduced` / `decl.printed` / `exercise.open` / `diagnostic`），带 span 与 human 文本。
3. [x] 错误 stage/code：parse（`unexpected-token`/`unexpected-eof`）、
      elab、kernel 三阶段；人类视图 `error[stage]:`，JSON 视图带 `stage`+`code`。
4. [x] CI：`.github/workflows/ci.yml`（workspace 测试 + 全部 examples 语料 +
      `--json` 事件为合法 JSON）。
5. [x] 语料回归：`crates/cli/tests/examples.rs` 遍历全部 `examples/*.sokonanoda`。

> 后续：设计已确认（v2：文件无 `#` 命令、练习=带洞声明、LSP-first）；已完成的
> 逐声明状态/错误细分/类型图/LSP 见下方"第二轮进度"，剩余见 `docs/design/infrastructure.md`。


---

### 第二轮进度（2026-09-06 晚，LSP-first 落地）

按 `docs/design/infrastructure.md`（v2）开工并完成第一段垂直切片：

1. [x] 逐声明状态 + `DocumentReport`（`check_document`）：每个声明 open/checked/failed，
      开放练习不污染环境、不影响后续声明；练习带名字（def/theorem）与目标类型。
2. [x] 错误细分：`ErrorKind` 稳定 code（`elab-*` / `kernel-rejected`…）+ 每条教学 `hint()`
      （CLI/JSON/LSP 三处都带）。
3. [x] 类型图 v1：elaboration 时按 AST 节点记录 (span, 内核项, binder 上下文)，
      内核新增 `infer_under_binders`，得到整文件 hover 表（表达式级类型）。
4. [x] `crates/lsp`（tower-lsp）：publishDiagnostics / hover（类型与 `???` 目标）/
      documentSymbol / codeLens（练习状态）/ quick-fix `intro`；`editor/vscode/` 薄壳扩展。
5. [x] `--json` 与 CLI 错误码跟随细粒度 code；`docs/protocol.md`、`docs/architecture.md`、
      README 同步。

以上均已落地（I6–I9 全绿）：prelude（`Nat`/`Bool`/`Eq`）、elaborator（
无注解 `let`、`match` 全形态含带索引、binder 推断）、第一门课 10 单元 + golden、
真增量（early cutoff）、goal 视图（树 + Infoview + 高亮统一）、VS Code 打包发布。
剩余仅远期 L2/L3（协作/远程、compiler service 跨文件转播）与已文档化技术债
（内核改动台账见 `docs/architecture.md` §6/§8；旧交接书的逐条记录见 `docs/STATUS-ARCHIVE.md`）。


---

## 10. 待办（已确认，按依赖排序）

> **本节只记「待办的验收口径 + 收口状态」** ✓ —— **"下一步 / 队列"的唯一入口是
> `docs/ONBOARDING.md` §0.2** ✗（2026-09-30 文档收敛：三份顶层计划文档不再各写一份队列）。
> 详细验收与设计依据：`docs/design/e2-plan.md`（现行契约 + 收口索引）·
> 逐轮流水：`STATUS.md` · 已收口项的逐字历史：`git log -- ROADMAP.md` / `docs/STATUS-ARCHIVE.md`。

### 10.1 已收口（一行一条；细节看设计文档，别在这里找验收）

| 项 | 状态 | 设计 / 证据 |
|---|---|---|
| **I6** prelude 对齐 + elaborator 推进 | ✅（`Bool` 0.41.0 · binder 类型推断 0.45.0 · 无注解 `let` 0.34.0 · `match` 0.33.0–0.42.0） | `docs/design/elaborator-let-match.md` · `docs/design/match-patterns.md` |
| **I7** 第一门课（M4，内容层） | ✅ 完成（**10 单元锁定**，`course/course.json`；中英镜像 + 解答钥匙） | `docs/design/course-syllabus.md` §0 |
| **I8** 真正增量（服务层前置） | ✅（check-then-add · Session · `watch` · 真增量后缀重查 · conservative early-cutoff 0.32.1） | `docs/design/i8-i9.md` §1 · `docs/design/early-cutoff.md` |
| **I9** kernel 显式错误 + goal 视图 | ✅（错误分类学 · goal 视图 + `soko/goals`/`nextHole` · 多洞/refine · 函数实参洞 · 呈现面高亮统一 0.43.0/0.49.0） | `docs/design/i8-i9.md` §2 · `docs/design/goal-refine.md` · `docs/design/goal-func-spine.md` · `docs/design/goal-rendering.md` · `docs/design/spine-meta-a.md` |
| **I10** 值位 `apply` 关键字 | ❌ 已废弃（0.18.0 完成 ⇒ 0.22.0 随 I13 整体移除） | `docs/design/term-apply.md` · `docs/design/remove-funintro.md` |
| **I11** 真人输入测试体系 | ❌ 已废弃 / 被取代（`char_steps` 基建已删） | 现状 = `by` 引擎：`docs/design/by-tactics.md` · `docs/design/goal-list.md` · `docs/design/tactic-hover.md` |
| **I12** 项目官网（GitHub Pages） | ✅（**现行权威 = 单页站点**；2026-09-20 的 28 页重构**已归档** ⇒ 别照着它新建页面 ✗） | `docs/design/site-single-page.md` · 验收 `python3 scripts/check-site.py`（10 项） |
| **I13** 值位关键字 v2 | ❌ 已废弃（`funapply` 0.22.0 / `funintro` 0.27.0 移除） | `docs/design/remove-funintro.md` |

⚠ **I11 仍成立的一条**：`soko/nextHole` **无法在同一源码位置的多子目标之间导航**
（同址子目标只能整组导航）—— 已记入 `docs/protocol.md` 的 `soko/nextHole` 小节（"Known limitation"）。

### 10.2 未收口（**验收口径在这里**；顺序与入口见 `docs/ONBOARDING.md` §0.2）

#### I14 —— DeepSeek Harness 适配（**H0–H4 ✅ 已落地；H6-E = backlog**）

> 设计 + 计划：**`docs/design/deepseek-harness.md`**。一句话：产品内核与 harness 无关，
> 要适配的是**接线层**（技能发现路径、斜杠命令、LSP 接线、二进制可达性、工具链 deny、
> 文档 / 契约测试的单 harness 假设）；**不需要改 Rust 语义代码**。
> **已落地**：H0 `.agents/skills/` 薄入口 + `crates/cli/tests/dsh.rs` 双向守卫 · H1 `scripts/soko`
> 启动器（版本钉 + 缓存标记守卫）· H2 `dsh/cordis.patch.yml`（显式写清 DSH 侧诊断不在通道内）·
> H3 技能名即斜杠命令 · H4 Lean 工具链 deny 与门面同步。

- **H6-E ⬜ backlog**：Infoview 客户端插件；`SessionStart` 自动 provisioning；把启动器 +
  Lean 工具链 deny hook + `/sokonanoda-*` 命令打成 npm 插件。
- **验收 A1–A6**：DSH 里"按 `AGENTS.md` 接手并当我的老师"能零 cargo 跑通判卷；
  `/sokonanoda-teacher` 可用；`.sokonanoda` 能 hover / 跳定义；全量测试 + gate 绿；
  文档不再假定 opencode 唯一；反漂移契约测试绿。

#### I15 —— 内核真相查询通道（✅ 0.56.0 落地，0.56.1 结构债清零；H6-E backlog）

> 设计：**`docs/design/agent-query-channel.md`**。一句话：**"内核真相"此前只有 LSP 一条出口**，
> 而 DSH 的 LSP host 丢弃诊断 ⇒ agent 只能整文件扫事件流。落地顺序与设计一致：
> **真相层（`front::query`）→ 传输（CLI `query` + MCP）→ LSP 改为调用同一个真相层**。

- **已落地**：H6-A 真相层 + CLI + LSP 委托（`crates/lsp/src/lib.rs` 4256 → 1105 行；
  ⚠ 2026-09-30 实测**又长回 2333 行** ⇒ "结构债清零"只对当天成立，见 `REQUIREMENTS.md` §4）·
  H6-B MCP（`dsh/mcp/server.js`，六个工具，默认关闭、用户显式 opt-in）·
  H6-C 两个 front 缺口（索引递归 `Prop` 的 recursor / 多名字 binder 组，**内核零改动**）·
  H6-D 门面同步。
- **验收 A1–A7 ✅**：真相唯一（LSP 侧无实现残留）· CLI 可用 · MCP 可用 ·
  **CLI≡LSP 字段级一致性契约**（`crates/cli/tests/query.rs`：计数 ≡ `--json` 事件流）·
  A5 结构债**当天**双达标（现已回涨，见上）· 两个 TODO 修复带反向测试 · 全量回归绿且既有契约测试"只增不改"。
- **H6-E ⬜ backlog**：同 I14。

#### I16 —— 多文件 `import` 与项目管理（✅ 0.57.0 落地：P0–P6 完成；**P7 = backlog**）

> 设计 + as-built：**`docs/design/imports-and-projects.md`**（§5.1 含三处与设计的偏差）。
> 一句话：把**编译单元**从「一个文件」升级为「项目闭包」——`import Foo.Bar` 用真实 Lean 4 的
> 置顶语法、模块名 ↔ 路径用 Lean 同款规则、项目根 = 最近祖先的 `sokonanoda.toml`；
> **无 `import` 的文件行为逐字节不变** ✓（缓存键、事件流、golden 计数全不动）。

- **已落地**：P1 语法与解析 · P2 闭包编译 · P3 CLI 与协议（`--root` / `--no-project`）·
  P4 缓存与失效 · P5 LSP / 编辑器（多文档、跨文件 `definition`/`references`/`rename`、
  改依赖自动刷新下游）· P6 教学与发布（单元⑪ + `course/unit11-project/`）。
- **P7 ⬜ backlog**：`didChangeWatchedFiles` · `soko/project` 的剩余项 · `watch` 项目模式 ·
  `[deps]` · `namespace`。
- **验收 A1–A8**：核心是 **A1** —— 无 `import` 的 45 个语料文件 `--json` 与 HEAD 逐字节一致、
  两处 golden 表零漂移 ✓（`crates/cli/tests/imports.rs` 12 e2e + `crates/lsp/src/tests/project.rs` 4 e2e）。
- **结构债**：`check.rs` 的 `run_pass` 巨石当年拆成 `check/{mod,walk,kernel_phase}.rs`
  + `compile/units.rs`（791/951/413 行）；⚠ **2026-09-30 实测已长回 1549/1775/635/159 行**
  ⇒ "已还清"不成立，`mod.rs`/`walk.rs` 都超 ~500 行红线 ⇒ **动 `run_pass` 前先读
  `docs/architecture.md` §6/§8** ✓。

#### L2/L3 —— 编辑器与 agent（M5+，远期）

- L2：VS Code 扩展打包（语法、进度树、goal 面板），接 LSP 事件 —— **已远超原计划**（见下 E1）。
- L1/L3：compiler service 事件流（`file.didChange` 等，见 `docs/protocol.md` 未来事件名）、
  讲课 agent 消费同一文档状态自动出题。
- **业内标准补全清单**：调研稿 `docs/notes/gap-analysis.md` **已归档** ⇒
  `docs/archive/notes-2026-09-26/gap-analysis.md.gz`；**开发清单全部清零**，
  剩余仅运营项（release 首跑、教学回环、VS Code 集成测试）与远期设计项（spine meta 方案 A）。

#### E1 —— 编辑器项目模式体验（✅ **124/124 全部收口**）

> 收口索引：**`docs/design/vscode-editor-feedback-plan.md`**（六条反馈 → 根因一句话 +
> E2 继承的四条结论）；逐字原文 ⇒ `docs/archive/vscode-editor-feedback-plan-full-2026-09-26.md.gz`。
> ⚠ **`scripts/plan.py` 现在跟踪的是 E2 + 批次 U/N**（`docs/design/e2-plan.md`），**不是 E1**。

- **六条反馈**（编译慢 / 无编译缓存 / 打开即临时编译 / 声明栏失效 / goal 不用记法 /
  记法不能跳转且 hover 无原始类型）**全部收口** ✓，结论已被 E2 继承。
- ⚠ **这四条 2026-09-30 已复核，两条要反过来读**（权威 = `docs/design/by-prefix-reuse.md` §6）：
  **T-K11（K1-a）不再"零收益"** —— 那天 `reuse_hits=0` 是因为担保还没接到主编译 pass；
  §3.C 接通后同一开关 **1.25×** 且**默认开**（全语料两态逐字节 0 差异）；
  **T-K30 已落地**（切片 1b：`build_one(…, precomputed)` + `PassTables`，见
  `docs/design/module-artifacts-slice1b-handoff.md`）。仍然成立的两条：T-K12c **不做也不再需要**
  （§3.C 走担保那条路，没有独立环境 ⇒ 没有 `decl_idx` 那堵墙，钉子测试留着钉内存 API 语义）·
  T-K31 实测无收益（已回退）。
- ⚠ **记法渲染不走内核 pp**（调研结论，防后人再走弯路）：内核的记法打印是死代码
  （`ExportFile.notations` 无一处 insert），且 `pp_expr` 同时是 `#check`/`#reduce`/`#print`
  的出口 ⇒ 改它会动 `--json` 字节；走 front 的显示边界重写（`docs/design/notation-aware-printing.md` §3）。
