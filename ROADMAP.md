# sokonanoda-lang —— `.sokonanoda` 协作式 Lean 4 教学 ROADMAP

> 状态：active（v2 LSP-first，已落地第一段垂直切片；最新进度先看 `STATUS.md`）
> 基线：sokonanoda `7b51784`
> 日期：2026-09-06（终版快照）
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

执行 M0：建仓并完整迁移 kernel，跑绿全部现有测试，公开 `check_expr` / `reduce_expr` /
声明添加 / 显式错误 API。这是后续所有层的地基，不依赖任何 UI、agent 或官方 Lean。

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

仍待办（按 I6–I9 与课程）：prelude 扩充与占位体对齐、elaborator 推进（binder 推断、
`let`、`match`）、第一门课（5 单元 × 3–8 练习 + golden）、真正增量缓存（check-then-add）、
goal 视图深化与 `#prove` 入库、VS Code 扩展打包。


---

## 10. 待办（已确认，按依赖排序）

> 详细验收与设计依据：`docs/design/infrastructure.md`（F1–F8、工作流 I0–I9）；
> 进度快照：`STATUS.md`。

### I6 —— prelude 对齐 + elaborator 推进
- prelude：补 Bool / Eq / `rfl` 所需受信任基元；核对 `Nat.succ`/`Nat.add`
  占位自引用体（当前依赖名字特判 + 原生快路径，需写清边界并加裸名 `#reduce` 测试）。
- elaborator：binder 类型推断（先非依赖情形）→ `let` → 单构造子 `match`/递归
  （M4 课程第 5 单元的前置）。
- 验收：每个语法点走 TDD 三件套（front 单测 + CLI e2e + 课程用例），白名单同步更新。

### I7 —— 第一门课（M4，内容层）
- 5 单元：① 表达式与类型 ② 函数与箭头 ③ 命题与证明项 ④ 等式与 `rfl`
  ⑤ 归纳与 `match`；每单元 3–8 个练习。
- 组织：`course/lesson-XX-*.sokonanoda`（正文+练习），`course/course.json` 顺序清单；
  判定走 kernel（目标类型/化简断言），不做文本比对。
- 验收：零基础用户按顺序完成；CI 跑全部课程文件并比对 golden 事件。

### I8 —— 真正增量（服务层前置）
- [x] check-then-add：失败的声明不进环境（pass2 重算，ByIndex/ByName 可见性）；
      归纳块纳入 kernel 判定。
- [x] Session（front::session）：版本号、delta 事件、内容未变零重编译。
- [x] watch：`sokonanoda watch <file>` JSON Lines 流。
- [x] **真增量后缀重查（2026-09-07）**：TrustPlan 信任前缀跳过内核重查 +
      逐命令快照复用 + span 重映射；`SessionUpdate.stats.kernel_checks`
      可验证；LSP 切换到 Session。设计见 `docs/design/i8-i9.md` §1。
- [ ] 验收余项：受影响后缀的**依赖精确化**（当前为保守 suffix；early-cutoff
      签名比较是可选优化）。

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
- [ ] goal 视图余项：声明宇宙参数携带 ✅（已并入 judge）；refine 的子洞
      kernel 级 expected type（spine meta）；VS Code goal 面板 ✅。

### I10 —— 值位 `apply` 关键字（新语法，三件套）

> 设计（已完成，勿再重复讨论方案）：`docs/design/term-apply.md`。
> 核心结论：**不能照抄 `intro`**——`apply` 需要「被应用名字的类型」，而 front 侧
> 没有可用的类型表（`GoalTemplates` 丢 codomain、局部假设不在表内、内核无查询 API），
> 因此降低走 `by` 引擎已验证的 `judge_infer` 路线（内核推断类型，判定仍在填洞后）。

- **S1 front 骨架**：`parse_value` 第三分支（实参用 `parse_app` 消费）、AST
  `Expr::Apply`、穷尽 match 补齐、`compile/apply.rs`（复用 `judge_infer` /
  `parse_expr_text` / `peel_pi` / `unify_spine` / `substitute`）、
  `DeclState.apply_skeleton`、`check.rs::lower_value` 接线、三个新错误码 + `protocol.md`。
- **S2 合成洞分派**：`suggest.rs` 把带骨架的声明（`intro`/`apply`）一律路由到
  `judge_terms`，绕开 `judge_hole_fill` 的「洞位源码必须恰为 `sorry`」守卫
  （`judge.rs:316-322`）+ 回归测试。
- **S3 编辑器面**：`keyword_at` 通用化（收敛现有三套位置选取）、hover、补全项、
  VS Code 命令 `sokonanoda.expandApply`（含 `markdown.isTrusted` 放行）。
- **S4 课程与白名单**：单元里加「`exact` / `apply` / `intro` / `by` 对照」一节 + 练习
  （zh/en + 钥匙 + golden）；`semantic::KEYWORDS` 与白名单文档同步。
- **验收**：`cargo test --workspace --locked` 全绿（fmt/clippy 无新警告）；`apply` 的
  「不展开也等价」与「排版无关」两条契约测试；课程 golden 更新并说明新旧计数；
  版本 bump（新增命令 → minor）。
- **subagent 切分**：S1 由主会话做（公共接线点最多）；S2 与 S3 可并行派发，
  文件集互斥（`suggest.rs`+`compile/tests.rs` ↔ `lsp/*`+`editor/vscode/*`），
  任务书必须写死允许修改的文件清单与验收命令。

### I11 —— 真人输入测试体系（覆盖 `sorry`/`intro`/`apply`/`by` 共存）

> 设计（已完成）：`docs/design/real-input-tests.md`（含共存风险矩阵、测试清单、flake 预算）。

- **S0 前置（主会话，必须先做）**：`testutil::type_script` / `type_chars`（逐段/逐字符的
  真实编辑序列）；`keyword_at` 收敛位置选取（现 `intro_at` / `select_state_at` /
  `render::decl_at` 三套并存）。
- **S1 `by` 引擎缺陷**（调研发现的既有 bug，正交但会被共存测试暴露）：
  `by` 末个 tactic 是 `apply` 且留 ≥2 个子目标时，所有洞 span 相同
  （`by.rs:263/375` 取 `by.rs:269-271` 的单一 `hole_span`）→ `nextHole` 跳不动、
  inlay 叠位且类型可能错。**先写红测试钉住现状，再修**。允许改：`crates/front/src/by.rs`、
  `crates/front/src/compile/tests.rs`。
- **S2 front/session 五条**（F1–F5）：`sorry`↔`intro`/`by`/`apply` 往返、四种排版重排、
  同文档四写法改一行（`recompiled_from` 精确 + span remap）。
- **S3 LSP 逐字符四条**（L1–L4）：前缀不触发、整词才触发、词尾+空格仍触发、
  `by` 块内不串台。
- **S4 与 I10 合流**：L5–L8（展开后的 inlay/nextHole 可寻址、四写法混排 stateAt、
  注释编辑 remap）、V1–V4（VS Code 手势烟测）。
- **验收**：新增测试全部登记进 `docs/TESTING.md` 测试地图；**LSP/front 层零 sleep**；
  VS Code 层新增 sleep 必须在文件里登记理由；`ci.yml` 的集成测试步骤保持绿。

### I12 —— 项目官网（GitHub Pages）

> 设计（已完成）：`docs/design/site.md`（含托管方案决策、单一事实源机制、信息架构、页面清单）。

- **S0 修文档漂移**（`site.md` §5.1 的 6 条，含 README 写死 `V=0.9.0`、课程单元数
  「5 vs 6」三处口径矛盾）：与官网同批做，避免官网一上线就带错数字。
- **S1 站点骨架**：新目录 `site/`（零构建手写 HTML/CSS）+ `.github/workflows/pages.yml`
  （`configure-pages` / `upload-pages-artifact` / `deploy-pages`，`paths:` 过滤）
  + `scripts/gen-site-data.py`（python3 标准库，CI 生成 `site/data/site.json`）。
- **S2 单一事实源**：`STATUS.md` 顶部加机器可读块（version/round/date/tests），
  CI 断言其 version 与 `Cargo.toml` 一致；版本与下载链接在页面用 Releases API 取。
- **S3 页面**：`index` / `get-started` / `course` / `vision`（**必须新写**，现有全是
  agent 口吻）/ `progress`（生成）/ `agents` / `docs` / `about` / `en`。
- **S4 防漂移**：站内链接检查 + 「禁止写死版本号」断言 + README/AGENTS 挂官网入口。
- **前置人工动作（阻塞项）**：Settings → Pages → **Source = GitHub Actions**
  ——只能由仓库拥有者做；不做则 workflow 报 `Get Pages site failed. Not Found`。
- **验收**：站点可访问；`progress` 页的版本号/单元数与 `Cargo.toml`/`course/course.json`
  一致；链接检查与版本断言绿；`ci.yml`/`release.yml` 行为不变。

### 依赖与并行

- **I11-S0 先于 I10-S3 与 I11-S2/S3**（测试基建与位置收敛是公共接线点）；
- I10 与 I12 **互不依赖**，可并行（不同文件集：`crates/**` ↔ `site/**`+`workflows/**`）；
- I12-S0（修漂移）不依赖任何代码改动，可最先做。

### L2/L3 —— 编辑器与 agent（M5+，远期）
- L2：VS Code 扩展打包（语法、进度树、goal 面板），接 LSP 事件。
- L1/L3：compiler service 事件流（`file.didChange` 等，见 protocol.md 未来事件名）、
  讲课 agent 消费同一文档状态自动出题。
- **业内标准补全清单**：`docs/notes/gap-analysis.md`（2026-09-07 调研）——
  **开发清单全部清零**（第十五轮收尾：spine meta 方案 B′ 深度实例化、
  失败声明建议梯子 kernel-rfl/Reset/Restart）。剩余仅运营项（release 首跑、
  教学回环、VS Code 集成测试）与远期设计项（spine meta 方案 A）。
  发布流水线已落地（`release.yml` + `docs/RELEASE.md`）。
