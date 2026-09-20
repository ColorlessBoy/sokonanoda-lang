# sokonanoda-lang —— `.sokonanoda` 协作式 Lean 4 教学 ROADMAP

> 状态：active（v2 LSP-first；最新进度先看 `STATUS.md`，本文 §10 是待办与验收标准）
> 基线：v0.20.0（本文只描述计划与验收，已完成的条目就地打勾并标注版本）
> 配套文档：`STATUS.md`（当前状态与进度日志，agents 先读）、
> `docs/architecture.md`（深度理解）、`docs/notes/research.md`（外部调研）、
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

**I11 余项（真人输入测试 S2–S4）**——基建与关键缺陷修复（S0/S1）已落地，
剩余 F1–F5 / L3–L8 / V2–V4 见 §10。M0（kernel 迁移）等早期里程碑均已完成。

---

## 9. 当前执行清单（按顺序完成）

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

1. [x] 文档：`docs/architecture.md`（架构与内核深度理解）、`docs/notes/research.md`
      （外部调研）、`docs/design/infrastructure.md`（基础设施设计脑暴）、
      `docs/protocol.md` 刷新为"文本 + JSON Lines"双视图协议。
2. [x] CLI `--json`：每条事件一行 JSON（`decl.checked` / `expr.typed` /
      `expr.reduced` / `decl.printed` / `exercise.open` / `diagnostic`），带 span 与 human 文本。
3. [x] 错误 stage/code：parse（`unexpected-token`/`unexpected-eof`）、
      elab、kernel 三阶段；人类视图 `error[stage]:`，JSON 视图带 `stage`+`code`。
4. [x] CI：`.github/workflows/ci.yml`（workspace 测试 + 全部 examples 语料 +
      `--json` 事件为合法 JSON）。
5. [x] 语料回归：`crates/cli/tests/examples.rs` 遍历全部 `examples/*.sokonanoda`。

> 后续：设计已确认（v2：文件无 `#` 命令、练习=带洞声明、LSP-first）；已完成的
> 逐声明状态/错误细分/类型图/LSP 见下方"第二轮进度"，剩余全部在 §10。


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
（见 `docs/HANDOVER.md` §3 D / §4）。


---

## 10. 待办（已确认，按依赖排序）

> 详细验收与设计依据：`docs/design/infrastructure.md`（F1–F8、工作流 I0–I9）；
> 进度快照：`STATUS.md`。

### I6 —— prelude 对齐 + elaborator 推进
- prelude：~~补 Bool~~（✅ 0.41.0：非递归真实可信归纳，`Bool.true`/`Bool.false` + `Bool.rec`）/ Eq / `rfl` 所需受信任基元；核对 `Nat.succ`/`Nat.add`
  占位自引用体（当前依赖名字特判 + 原生快路径，需写清边界并加裸名 `#reduce` 测试）。
- elaborator：binder 类型推断（✅ 0.45.0：应用位置从实参类型推断）→ `let`（✅ 0.28.0；✅ 无注解
  `let` 0.34.0）→ `match`（✅ v1 0.33.0：源内非递归 `inductive`；递归/依赖/
  参数化/prelude `Nat`·`Eq` 待后续；嵌套/字面量/守卫模式 ✅ 0.42.0，见 `docs/design/match-patterns.md`）。
- 验收：每个语法点走 TDD 三件套（front 单测 + CLI e2e + 课程用例），白名单同步更新。

### I7 —— 第一门课（M4，内容层）✅ 完成（锁定 10 单元）
- **现状（10 单元，`course/course.json`）**：① `unit1-propositions-proofs`
  命题与证明项 ② `unit2-equality-rfl` 等式与 `rfl`
  ③ `unit3-functions-arrows` 函数与箭头 ④ `unit4-by-tactics` `by` 写法：
  tactic 证明 ⑤ `unit5-universes-sort` 宇宙：函数类型的类型
  ⑥ `unit6-induction-recursion-1` 归纳与递归 Ⅰ
  ⑦ `unit7-induction-recursion-2` 归纳与递归 Ⅱ
  ⑧ `unit8-quantifiers` 量词（`forall`/`Exists`）
  ⑨ `unit9-relations-connectives` 关系与联结词（`Or` 升级为真 inductive +
  自动派生 `Or.rec`、`Iff` 定义、`Le`/`Even` 归纳关系 + 手写消去子）
  ⑩ `unit10-reading-proofs` 读证明与综合（自解释三问、formal↔informal
  互译、评阅错证明、期末小项目）；每单元 5–10 个练习，
  汇总 `units=10 checked=78 open=59 failed=0`。
- 组织：`course/unitN-*.sokonanoda`（中文权威）+ `course/en/`（英文镜像：
  代码逐字节一致、`--` 注释可不同）+ `course/solutions/` 钥匙 +
  `course/course.json` 顺序清单；判定走 kernel（目标类型/化简断言），
  不做文本比对。
- **已完成**：P1（内容修补 + 测试加固）/ P2（`by` 提前、现 U5 拆 Ⅰ/Ⅱ）/
  P3（新增 #9 关系与联结词、#10 读证明与综合）全部落地，大纲按
  `docs/design/course-syllabus.md` §0 锁定为 10 单元。单元⑨落地时发现两处
  产品缺口——索引递归 `Prop` 的 recursor 自动派生失败、`inductive` 参数不
  接受多名字 binder 组——记在 `docs/HANDOVER.md` §3（前者：`Le`/`Even` 需
  手写 `rec`/`iota`）。
- 验收：零基础用户按顺序完成；CI 跑全部课程文件并比对 golden 事件。

### I8 —— 真正增量（服务层前置）
- [x] check-then-add：失败的声明不进环境（pass2 重算，ByIndex/ByName 可见性）；
      归纳块纳入 kernel 判定。
- [x] Session（front::session）：版本号、delta 事件、内容未变零重编译。
- [x] watch：`sokonanoda watch <file>` JSON Lines 流。
- [x] **真增量后缀重查（2026-09-07）**：TrustPlan 信任前缀跳过内核重查 +
      逐命令快照复用 + span 重映射；`SessionUpdate.stats.kernel_checks`
      可验证；LSP 切换到 Session。设计见 `docs/design/i8-i9.md` §1。
- [x] 验收余项：受影响后缀的**依赖精确化**（✅ 2026-09-14 补做 conservative
      early-cutoff 签名比较：环境贡献签名相等即停止重查并复用尾部快照；
      保守 sound，见 `docs/design/early-cutoff.md`）。

### I9 —— kernel 显式错误 + goal 视图
- [x] kernel：def_eq 失败给出两端项（稳定格式 `def_eq mismatch expected: … |
      actual: …`），front 解析为「类型不匹配：期望 X / 实际 Y」。
- [x] **conv 快路径 soundness 修复（2026-09-07）**：eval/infer 闭包混用导致
      不可居住类型通过——快路径加闭包语义守卫；回归测试三层。
      见 `docs/design/i8-i9.md` §2 与 `docs/architecture.md` §6。
- [x] goal 视图：`front::judge`（合成声明交完整 kernel 裁决）；LSP exact
      kernel 判定（文本比对删除）；REPL exact/apply/assumption kernel 判定
      并反馈期望/实际；`soko/goals` + `soko/nextHole` 自定义请求。
- [x] **多洞 + refine（2026-09-07）**：构造子 spine 走查（子洞合法、期望
      类型实例化、`DeclState.holes/sub_goals/refine_template`）、LSP refine
      建议（构造子骨架、参数自动填充）、nextHole 跨子洞。
      见 `docs/design/goal-refine.md`。
- [x] **函数实参洞 + hover 开项修复（2026-09-10，第二十四轮）**：已知函数
      （prelude Eq、源内 axiom/def/theorem、归纳构造子）的**直接实参**
      `sorry` 合法，期望类型 = binder 望远镜在前置实参处实例化（含宇宙
      层级 `. {1}` → `Sort 1`）；模板 machinery 抽到
      `crates/front/src/compile/goals.rs`；同轮修内核 pp 对开项推断 panic
      （hover `Eq.subst.{1}` 显示签名、`#check` 不再假报 rejected）。
      v1 不做嵌套洞/部分应用/`sorry + 1`。见 `docs/design/goal-func-spine.md`。
- [x] **内核错误分类学（2026-09-07）**：8 个新 kernel 错误码（expected-sort /
      expected-pi / theorem-not-prop / non-positive / ctor-result / ctor-arg
      三族）+ 内核冷路径消息增强（`got:` 渲染）+ `#check`/`#reduce` panic
      守卫（此前会崩掉编译/LSP 进程）。审计见 subagent 报告，分类器
      `front::error::refine_kernel_kind`。
- [x] **呈现面高亮统一（✅ 0.43.0 围栏统一；✅ 0.49.0 分类/文本单一起源 `tm_scope`+runs 投影）**：表达式/签名 hover 的
      ` ```text ` 围栏改 ` ```sokonanoda `；声明 hover 内联签名、补全 detail/文档、
      诊断内嵌类型、hints/quick-fix 预览、练习树 tooltip 统一走 `front::semantic`
      （围栏或 runs）；见 `docs/HANDOVER.md §3 A″`、`docs/design/goal-rendering.md`。
- [x] goal 视图余项：声明宇宙参数携带 ✅（已并入 judge）；refine 的子洞
      kernel 级 expected type（✅ spine meta 方案 A，0.32.0：请求期 `judge_infer`
      探针，覆盖前置洞穿透/一层嵌套洞；更深嵌套与 def 包裹结果类型的 whnf 仍留
      B′，见 `docs/design/spine-meta-a.md`）；VS Code goal 面板 ✅（树「当前光标处」
      + webview Infoview，0.30.0）。

### I10 —— 值位 `apply` 关键字（✅ 0.18.0 完成；❌ 关键字本身已整体移除）

> **已废弃（历史存档）**：值位 `apply` 随 I13 改名 `funapply`，并于 0.22.0
> 一并移除；独立关键字通道（`Expr::Apply` 值位物化）已不存在，值位只保留
> 普通表达式与 `by` 块（`docs/design/remove-funintro.md`）。设计与 as-built
> 存 `docs/design/term-apply.md`；当前的 `apply` 仅是 `by` 块内 tactic，见
> `docs/design/by-tactics.md`。

### I11 —— 真人输入测试体系（❌ 已废弃/被取代）

> **已废弃（历史存档）**：`char_steps` 输入脚本基建已删除，值位 `intro`/`apply`
> 关键字也已移除，四写法共存矩阵随之作废（`docs/design/remove-funintro.md`）。
> 测试体系现以 `by` 引擎为中心：`docs/design/by-tactics.md`（tactic 集）、
> `docs/design/goal-list.md`（多目标）、`docs/design/tactic-hover.md`
> （tactic 高亮/hover）。
> **仅一条结论仍成立**：`soko/nextHole` 无法在同一源码位置的多子目标之间
> 导航（同址子目标只能整组导航）——已记入 `docs/protocol.md` 的
> `soko/nextHole` 小节（"Known limitation"）。

### I12 —— 项目官网（GitHub Pages）

> 设计（已完成）：`docs/design/site.md`（含托管方案决策、单一事实源机制、信息架构、页面清单）。
>
> **2026-09-20 全面重构（第一百〇九轮，I12-R1）**：用户要求「不要参考旧版本，旧版本没有
> 设计感、美感，很多 ai 味」。视觉层整体废弃，站点从 9 页扩到 **28 页**，加入功能展示页、
> 搜索、对照页、术语表、常见问题、版本历史、404 与站点文件。
> **新权威 = `docs/design/site-rebuild/`**（入口 `STATE.md`）；
> **验收 = `python3 scripts/site-verify.py`**（18 项完整性 + 正确性，当前 18/18 绿、exit 0；
> CI 跑其中 16 项，跳过的两项要 Chrome）。**三条**「站点写的是已发布事实」的判据都锚在
> **发布 tag** 上：K12 课程计数、K16 playground 计数、K17 版本号本身（站点写的是已发布
> 版本的事实，拿 HEAD 当基准会被课程门禁的版本钉拒判——理由与修法见
> `docs/design/site-rebuild/STATE.md` §5/§7.2）。
> 旧 `docs/design/site.md` 已标为被取代；`gen-site-demos.py` 与 `site/assets/demos/`
> （PIL 假截图）已删除。详见 `STATUS.md` 第一百〇九轮与 `REQUIREMENTS.md` §9（2026-09-20）。

- **S0 修文档漂移**（✅ 已完成：README 版本号、课程单元数口径等已随 I12 修正）。
- **S1 站点骨架**：新目录 `site/`（零构建手写 HTML/CSS）+ `.github/workflows/pages.yml`
  （`configure-pages` / `upload-pages-artifact` / `deploy-pages`，`paths:` 过滤）
  + `scripts/gen-site-data.py`（python3 标准库，CI 生成 `site/data/site.json`）。
- **S2 单一事实源**（✅ 按实际实现落地，与原规划不同）：不引入 STATUS 机器
  可读块——`scripts/gen-site-data.py` 直接解析 STATUS 最新轮标题 + Cargo.toml
  + course.json 生成 `site/data/site.json`；版本一致性由三重现有机制强制
  （契约测试 / release version gate / auto-tag 显式比对）。
- **S3 页面**：`index` / `get-started` / `course` / `vision`（**必须新写**，现有全是
  agent 口吻）/ `progress`（生成）/ `agents` / `docs` / `about` / `en`。
- **S4 防漂移**：站内链接检查 + 「禁止写死版本号」断言 + README/AGENTS 挂官网入口。
- **前置人工动作**（✅ 已完成 2026-09-13：Settings → Pages → Source = GitHub
  Actions，经 gh api 代启；workflow 门禁用鉴权 `gh api` 探测，未启用时礼貌跳过)。
- **验收**（✅ 0.20.0 达成）：站点可访问（colorlessboy.github.io/sokonanoda-lang）；
  版本号/单元数全部生成、零手写；`check-site.py` 卫生检查绿。

### 依赖与并行

- **I11-S0 先于 I10-S3 与 I11-S2/S3**（测试基建与位置收敛是公共接线点）；
- I10 与 I12 **互不依赖**，可并行（不同文件集：`crates/**` ↔ `site/**`+`workflows/**`）；
- I12-S0（修漂移）不依赖任何代码改动，可最先做。

### I13 —— 值位关键字 v2：funintro/funapply + 输入期补全 + 关键字组合（❌ 已废弃）

> **已废弃（历史存档）**：`funapply` 于 0.22.0 移除，`funintro` 于 0.27.0
> 移除（`docs/design/remove-funintro.md`）。值位现在只保留普通表达式与
> `by` 块；当时的改名/输入期补全/关键字组合方案均不再执行，设计存
> `docs/design/value-keywords-v2.md`（已加废弃横幅）。
> 现状实现见 `docs/design/by-tactics.md`（tactic 集与 `by` 引擎）、
> `docs/design/goal-list.md`（多目标显示）、`docs/design/tactic-hover.md`
> （tactic 高亮与 hover goal state）。

### I14 —— DeepSeek Harness 适配（设计已定稿，实现未开始）

> 设计 + 计划：**`docs/design/deepseek-harness.md`**（2026-09-17 第八十六轮，
> 只出计划）。一句话：产品内核与 harness 无关，要适配的是**接线层**
> （技能发现路径、斜杠命令、LSP 接线、二进制可达性、工具链 deny、文档/契约测试
> 的单 harness 假设）；**不需要改 Rust 语义代码**。

- **H0 技能上架**（解 G1）：`.dsh/skills/<name>/SKILL.md` 薄网关（正文唯一留在
  `skills/`）+ 新增 `crates/cli/tests/dsh.rs` 双向守卫。
- **H1 二进制可达**（解 G5）：新增零依赖 Node 启动器 `scripts/soko`（解析链与
  `.opencode/plugins/sokonanoda.ts` 同语义 + marker 版本守卫）；`AGENTS.md`
  Setup 改 harness 中立。
- **H2 LSP 接线**（解 G4 的可用部分）：项目自带 `dsh/cordis.patch.yml`
  （`lsp` + `lsp-stdio` + `tool-lsp`，`extensionToLanguage[".sokonanoda"]`）+
  `--patch` 用法；**显式写清 DSH 侧诊断不在通道内**（`publishDiagnostics` 被丢弃）。
- **H3 命令与角色**（解 G2/G3）：把 `.opencode/command/**` 与
  `.opencode/agent/teacher.md` 的正文并入 `sokonanoda-teacher` / `-dev`
  （DSH 的技能名即斜杠命令）。
- **H4 治理**（解 G6/G7/G8）：Lean 工具链 deny 的 DSH 形态（`tools/pre-execute`
  插件或 hooks 桥，桥不做项目发现）、33 处文档去 opencode 单一化、门面同步。
- **验收 A1–A6**：DSH 里"按 AGENTS.md 接手并当我的老师"能零 cargo 跑通判卷；
  `/sokonanoda-teacher` 可用；`.sokonanoda` 能 hover/跳定义；全量测试 + gate 绿；
  文档不再假定 opencode 唯一；反漂移契约测试绿。
- **待拍板**：技能进 DSH 的方式（网关 vs `customSkillDirs`）、启动器形态
  （与 REQUIREMENTS（三十二）删除 `scripts/soko.sh` 的边界）、deny 形态、版本号策略。

### I15 —— 内核真相查询通道（`query` 子命令 + MCP）✅ 0.56.0 落地，结构债 0.56.1 清零（H6-E 留 backlog）

> 设计 + 计划：**`docs/design/agent-query-channel.md`**（2026-09-17；H6-A…H6-D
> 已按设计落地，H6-E 见下）。一句话：**"内核真相"此前只有 LSP 一条出口**，而
> DSH 的 LSP host 丢弃诊断、不调自定义请求，agent 只能整文件扫事件流。落地顺序
> 与设计一致：**真相层（`front::query`，编辑器无关的类型化查询）→ 传输
> （CLI `query` + MCP）→ LSP 改为调用同一个真相层**（反过来先写 MCP 会立刻
> 产生第二份真相）。

- **H6-A ✅ 真相层 + CLI + LSP 委托**：新增 `crates/front/src/query/{mod,types,tests}.rs`
  —— 对**同一份 `QueryDoc`** 提供 `check`/`state`/`goals`/`holes`/`hints`/`reduce`；
  `QueryError` 区分"正常的没有"（空答案）与"问不出来"（带稳定 code 的结构化错误）。
  CLI 新增 `sokonanoda query <op>`，打印**一个 JSON 对象**
  （`{schema:"soko.query/1", op, version, ok, data|error{code,message}}`），flags
  `--file/--text/--line/--col/--offset/--direction/--probe/--expr/--compact`
  （`--text` 支持未落盘中间态）；退出码 **0 = 答上了**（含结构化 `ok:false` 与
  开着的 `sorry`）/ **1 = 内核拒绝** / **2 = 用法错误**；契约写进 `docs/protocol.md`。
  **同一轮把 LSP 改为委托并清掉结构债**：`soko/*` handler 的语义函数从
  `crates/lsp/src/lib.rs` 删除，新增薄 `crates/lsp/src/query_map.rs`（只做
  offset ↔ `Range`/`Position` 映射）；再把两个测试模块移出文件、抽出
  `protocol.rs`（wire 类型）与 `tokens.rs`（semantic token 辅助），
  **`lib.rs` 4256 → 3988（删重复）→ 1105 行**，A5 的 ≤1200 与"无重复实现"双达标。
- **H6-B ✅ MCP 传输**：`dsh/mcp/server.js`（零依赖 Node MCP stdio 桥）+
  `scripts/soko mcp` + `dsh/cordis.patch.yml` 一行 opt-in，暴露六个工具
  `mcp__sokonanoda__{check,state,goals,holes,hints,reduce}`，全部转发
  `scripts/soko query …`；**已在真实 DSH headless session 里实测可用**。
  默认关闭（MCP server 是 DSH 沙箱外的可信代码，用户显式 opt-in）。
- **H6-C ✅ 两个 front 缺口已修**（即原 `docs/HANDOVER.md` §3 E 的两个 TODO，
  发布版二进制实测锁定根因）：
  ① `derive_recursor` 在 ctor 字段写在结果箭头链里时**丢掉索引实参**——根因是
  `elab.rs` 用只认 Ident/App 的 `src_spine` 读 ctor 索引，改用已会剥箭头的
  `spine_of_codomain`（一处一行）；顺带修写死 `is_k: false` 导致**单构造子 `Prop`**
  派生失败。课程因此**删掉手写的 `Le`/`Even` `rec`/`iota`**。
  ② parser 接受多名字 binder 组 `inductive Foo (A B : Prop)`（参数与 ctor 字段）；
  AST/elab 未改（组感知的 `push_binders` 早已存在）。课程同步简化
  （`Or (A : Prop) (B : Prop)` → `(A B : Prop)`，中英代码逐字节一致），
  **全部 golden 事件计数不变**。
- **H6-D ✅ 收尾/门面同步**：`AGENTS.md` Setup（判卷两视图 + MCP 工具）、
  `docs/protocol.md` 契约、技能正文、`docs/HANDOVER.md`、`REQUIREMENTS.md` §9、
  `editor/vscode/`（package.json 0.56.0 / CHANGELOG / README）。
- **H6-E ⏳ backlog**：原 DSH H5 其余项——Infoview 客户端插件；`SessionStart`
  自动 provisioning；把启动器 + Lean 工具链 deny hook + `/sokonanoda-*` 命令
  打成 npm 插件。
- **验收 A1–A7 ✅**（见设计文档 §11）：真相唯一（LSP 侧无实现残留）、CLI 可用、
  MCP 可用、**CLI≡LSP 字段级一致性契约**（`crates/cli/tests/query.rs`：
  `query check` 计数 ≡ `--json` 事件流计数；`query state` ≡ 真实 LSP 服务器
  `soko/stateAt` 逐字段，含两条无 `by` 分支）、**A5 结构债双达标**
  （`rg -n "fn select_state_at" crates/` 只允许命中 `crates/front/src/query/`，
  **且** `crates/lsp/src/lib.rs` **1105 行 ≤ 1200**）、两个 TODO 修复带反向测试、
  全量回归绿且既有契约测试"只增不改"。
  > 过程留档：清理结构债时我曾**没量就**把"≤1200 行"作废（以为剩下的都是协议
  > 服务代码），量完发现 3988 行里 2638 行是测试模块——移出测试 + 抽两个模块
  > 即可达标。教训：**改验收标准之前先把被验收的东西量一遍**（`docs/LESSONS.md`）。
  > 后续（0.56.1）：`crates/lsp/src/tests/` 按特性拆成 `mod.rs`（399 行）+ 9 个文件
  > （最大 392 行），HANDOVER §4 登记的债清零。

### I16 —— 多文件 `import` 与项目管理 ✅ 0.57.0 落地（P0–P6 完成，P7 = backlog）

> 设计 + 计划：**`docs/design/imports-and-projects.md`**（2026-09-17 第九十一轮，
> 只出设计 + 计划，不动实现、不 bump）。一句话：把**编译单元**从「一个文件」
> 升级为「项目闭包」——`import Foo.Bar` 用真实 Lean 4 的置顶语法、模块名↔路径
> 用 Lean 同款规则、项目根 = 最近祖先的 `sokonanoda.toml`；跨模块声明由 front 在
> **同一个 arena / 同一个 `EnvBuilder`** 里按拓扑序构造，**内核一行不改**；
> 无 `import` 的文件行为**逐字节不变**（缓存键、事件流、golden 计数全不动）。

- **P1 语法与解析**：`Command::Import` + 置顶校验 + 模块名合法性（`-` 非法 → 教学 hint）
  + `project/resolve.rs`（纯函数）+ 3 个错误码与 TDD 三层起步。
- **P2 闭包编译（核心）**：`project/{manifest,graph,report}.rs` + `compile_project()`：
  祖先发现、DFS 拓扑序、环检测、**闭包预扫描**、**prelude 闭包级只装一次**、
  逐模块 check-then-add、开放 `sorry` 不入环境、依赖失败**单条**阻断并归因到正确文件。
- **P3 CLI 与协议**：`--root` / `--no-project`；`build` 按 DAG 项目化；
  `query <op>` 在闭包环境下求值；`docs/protocol.md` 错误码表（**只增不改**）。
- **P4 缓存与失效**：`iface` 闭包哈希（依赖变 → 下游必 miss）+ per-module 报告落盘 +
  warm cache `--json` 逐字节一致；（可选）"已检查声明"信任台账——**先量收益**再开。
- **P5 LSP / 编辑器**：项目根发现（manifest → workspace → 单文件三层）、
  反向后继重编（只 publish 已打开文档）、跨文件 `goToDefinition`/`findReferences`/
  `rename`、`soko/project`（可选）。
- **P6 教学与发布**：第 11 单元「模块与项目」（CN/EN/解答/`course.json`/两处 golden 表）
  + `skills/`、VS Code、`site/` 门面同步 + 版本 **0.57.0**（minor，用户可见新能力）。
- **量化动机（本轮 subagent 实测）**：45 个语料文件 3851 行里 **1217 行（31.6%）**
  落在"名字在 ≥2 个文件出现过"的声明块内；**71 个名字有 ≥2 种定义**、
  **20 个变体从未同单元共现**（`Or` 的 axiom/inductive 两义、`Iff` 的 def/axiom、
  `And.*` 的三种 binder 类型）——**模块边界是让"哪个 `Or`？"可回答的唯一机制**；
  `solutions/` 与画布声明骨架 19/19、19/19、27/27 逐一对应。
- **验收 A1–A8**：见设计文档 §6。核心是 **A1**：无 `import` 的 45 个语料文件
  `--json` 输出与 HEAD 逐字节一致、两处 golden 表零漂移。
- **待拍板 Q1–Q6**（每条已给推荐）：清单格式（`sokonanoda.toml` vs JSON vs 纯标记）、
  无清单时是否允许 `import`、prelude 模式的决策者、是否做已检查声明的跨进程复用、
  课程语料是否同轮重构、`watch --workspace` / `soko/project` 是否 v1 就做。
- **as-built（2026-09-18，0.57.0，用户指示「全部按建议做完一版」）**：Q1–Q7 全按
  推荐执行；P1–P6 逐阶段落 commit（`feat(front)` → `feat(cli)` → `feat(front,cli)`
  → `feat(lsp,front)` → `feat(course,cli)` → 文档/门面轮），实现实况见设计文档
  **§5.1 as-built**（含三处与设计的偏差）。**P5 全部做完**：多文档、跨文件
  `definition`/`references`/`rename`、改依赖自动刷新下游（未落盘编辑经内存覆盖
  可见、诊断只在变化时重发）。留 P7 backlog 的只有：`didChangeWatchedFiles`、
  `soko/project`、`watch` 项目模式、`[deps]`、`namespace`。交付清单：`crates/front/src/project/`（6 文件 18 单测）、
  `crates/cli/tests/imports.rs`（12 e2e）、`crates/lsp/src/tests/project.rs`（4 e2e）、
  单元⑪ + `course/unit11-project/`、三处文档层同步。**新增结构债（已登记）**：
  `crates/front/src/compile/check.rs` 1717 → 1918 行（`run_pass` 单函数 ≈1174 行），
  拆分计划见 `docs/HANDOVER.md` §4——本轮**不再往 `run_pass` 里加分支**，加之前先拆。
  **✅ 2026-09-18（第九十五轮）已还清**：`check.rs` → `check/{mod,walk,kernel_phase}.rs`
  + `compile/units.rs`（791/951/413 行），`run_pass` 只剩装配与两段调用；
  验收 = 862 条测试 + **二进制对拍**（见 `docs/TESTING.md`）。

### L2/L3 —— 编辑器与 agent（M5+，远期）
- L2：VS Code 扩展打包（语法、进度树、goal 面板），接 LSP 事件。
- L1/L3：compiler service 事件流（`file.didChange` 等，见 protocol.md 未来事件名）、
  讲课 agent 消费同一文档状态自动出题。
- **业内标准补全清单**：`docs/notes/gap-analysis.md`（2026-09-07 调研）——
  **开发清单全部清零**（第十五轮收尾：spine meta 方案 B′ 深度实例化、
  失败声明建议梯子 kernel-rfl/Reset/Restart）。剩余仅运营项（release 首跑、
  教学回环、VS Code 集成测试）与远期设计项（spine meta 方案 A）。
  发布流水线已落地（`release.yml` + `docs/RELEASE.md`）。
