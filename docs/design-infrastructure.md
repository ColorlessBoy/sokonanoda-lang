# 基础设施全面完善：LSP-first 设计（v2）

> 状态：**设计草案（brainstorm），不是已定稿的 PRD**。
> 落地进度（2026-09-06 晚）：I1–I5 的第一段垂直切片已实现
> （DocumentReport/逐声明状态、ErrorKind+hint、hover 类型图、`crates/lsp` 服务器、
> `editor/vscode` 薄壳）；见 `docs/architecture.md` §4/§7.5。
> v2 变更：明确最终形态是 **VS Code + 细粒度反馈 LSP**；`.sokonanoda` 文件保持
> **纯声明式、无 `#` 命令**；`#check/#reduce/#print/#prove` 只是 REPL/调试玩具，
> 不属于文件格式。练习 = 一个**带洞的 `def`/`theorem`/`example` 声明**。
> 配套：`docs/architecture.md`（现状事实）、`docs/research.md`（外部参照）、
> `docs/protocol.md`（内部事件传输）。

---

## 1. 目标与约束（一句话复述）

**目标**：一门"现代形式化证明编程语言 + 教学平台"：完整内核、受限且真实的教学语法、
**以 LSP 为唯一反馈通道**（足够细致、足够详细），最终长成"用户 + agent 在同一
`.sokonanoda` 画布上协作"。
**约束**：kernel 完整、无官方工具依赖、语法是真实 Lean 4 子集、白名单即课程、
反馈即功能。**文件里出现的每个声明，填完洞之后都必须能原样放进官方 Lean**。

## 2. 形态对比：REPL 思维 vs LSP 思维

| | REPL/批处理思维（v1 文档偏此） | **LSP 思维（本版，采纳）** |
|---|---|---|
| 文件 | 混入 `#check/#reduce/#exercise` 命令 | **纯声明 + 讲解注释**，无 `#` |
| 看类型 | 自己写 `#check x` | 光标悬停 → hover 给类型 |
| 看化简 | 自己写 `#reduce` | 命令/内联动作给化简结果 |
| 判定练习 | `#exercise` + 文本输出 | 保存即诊断：solved/open/failed 状态 |
| 错误 | 一行文本 | LSP Diagnostic：span/code/message/related/hint |
| 消费方 | 人读 stdout | 编辑器渲染 + agent 读同一份结构化状态 |

`#prove` 的教学价值保留，但它**不进入文件**：它在 LSP 里变成
"goal 视图 + code action（intro/exact/…）"，且每一步仍只是搭 lambda，最终交 kernel。

## 3. 文件格式（无 `#` 命令）

### 3.1 原则

- 文件 = 交错排布的**讲解（`--` 注释/正文）**与**声明**；
- 声明只有真实 Lean 语法：`def` / `theorem` / `axiom` / `inductive … end` / `example`；
- `???` 是方言唯一的教学扩展：**未完成练习的洞**，只允许出现在声明值位；
  填成 term 后该声明必须与官方 Lean 一致；
- **练习 = 带洞的声明**。带名字的练习提供稳定 ID（诊断、进度、agent 事件都用它）。

### 3.2 命名与身份（决策 D1）

- `def name : T := ???` —— 编程/计算类练习（如 `def two : Nat`）；
- `theorem name : T := ???` —— 命题/证明类练习（内核规则：类型须在 Prop）；
- `example : T := ???` —— 匿名练习；文档模型给合成 ID（`<file>:<line>` 或序号），
  填完后**不污染环境**，适合"只练不存"。

> 注意：官方 Lean 的 `example` 不能带名字；我们不为此发明
> `example name : T` 这种非 Lean 写法。若确实想要"名字 + 不进环境"，后续可加
> `-- @exercise name` 这类**注释元数据**（仍无 `#`），把名字挂在 `example` 上，
> 填洞后 strip 成合法 Lean。默认先不做，用 `theorem`/`def` 命名 + 匿名 `example`。

### 3.3 逐声明工作（文档模型）

每个声明是独立编译单元，状态机：

```text
open(有洞 ???) ──填洞──> checked(通过 kernel) ──后续编辑──> 重新检查
      │                        │
      └── failed：诊断（parse/elab/kernel + 分类 + 提示）
```

`example : T := ???` 保持"open 是合法状态"：**只有该声明报 open，不影响其他声明**。

## 4. 细粒度反馈：LSP 能力清单（这是产品的核心）

"足够细致、足够详细" = 下面的每一项都要有。按依赖排序，全部来自同一个文档服务。

### F1 诊断（publishDiagnostics）
- 三阶段分类：parse / elab / kernel，稳定 ASCII code，human message，**精确 span**；
- kernel 拒绝时尽量给出**预期 vs 实际**：如 "类型不匹配：期望 `a -> a`，你的值是
  `Nat -> Nat`"；长期靠 kernel 显式错误，过渡期靠前端按消息归类；
- relatedInformation（指向出错 binder / 定义处）+ 首个教学提示（code → hint 模板）；
- 多错误恢复：一个文件尽量报多个独立错误，而不是停在第一个。

### F2 Hover
- 悬停任意表达式 → 推断类型（需要**任意 offset → 所在子表达式 → 该处类型**的查询，
  见 §5.2 类型图）；
- 悬停声明名 → 类型 + 前面的讲解注释；
- 悬停 `???` → 目标类型（= 该声明期望类型在当前 binder 上下文下的形态）。

### F3 练习状态与进度
- 每个练习声明：`open / solved / failed`，带时间戳与最后一次判定事件；
- 事件（`exercise.open/solved/failed`）同时广播给 agent（同一文档状态）。

### F4 化简与中间步骤
- 对选中表达式：显示 fully reduced（deep_reduce）与**分步归约**（教学价值高：
  让学生看到 `Nat.add` 一步步算，对标 py_nanobruijn 的 `reduce_steps`）；
- 先做 LSP command / inline hint，UI 形态（hover vs 面板）后定。

### F5 Goal 视图（#prove 的 LSP 形态）
- 悬停洞显示：**当前目标 + 可用假设**（binder 上下文）；
- code action：`intro x` / `exact h` / `apply f` / `assumption` → 生成/替换洞内容，
  与 `#prove` 共用同一套"搭 lambda"逻辑（`proof.rs` 从 REPL 挪成库 API）；
- 每步后 kernel 校验，失败给出 F1 式诊断。

### F6 文档结构与导航
- documentSymbol/outline：声明列表（含练习、是否已解决）；
- definition/references：名字跳转（先 definition，references 后置）。

### F7 增量
- 编辑一行 → 从**受影响声明**起重编译，之前声明缓存不动；
- 事件带版本号；agent 与编辑器各持游标。

### F8 语义信息（后置）
- semantic tokens（类型/构造子/关键字着色）；Unicode `→`/`∀` 显示与 ASCII 输入并存。

## 5. 编译器侧要补的"查询内核"（文档服务）

LSP 只是壳，真正的活在这些库 API（放 `crates/front` 或新 `crates/server`）：

### 5.1 DocumentReport（整文件一次检查的结构化结果）
```text
report
├── decls: [{ name?, kind(def/theorem/example/inductive…), span,
│             status(open/checked/failed), errors[] }]
├── diagnostics: [{ stage, code, message, span, related[], hint? }]
└── type_map: [span → 推断类型文本]   // F2 的素材
```
现在 `CompileOutput` 已有事件+错误；要补：**逐声明状态**（现在失败即整体失败）、
**type_map**、**hints**。

### 5.2 类型图（任意 offset → 类型）
- front AST 每个 `Expr` 节点已带 span；elaborate 时按节点记录
  `(node_span → kernel ExprPtr)`，检查后对每个节点在**其 binder 上下文**里
  `infer` 并 pretty 成文本；
- 难点：子表达式类型依赖外层 binder（de Bruijn/名字作用域），需要在 elaborator
  里保留"每节点作用域"或在 AST 层做类型推导（教学子集内可先做"整声明类型 +
  顶层子项"，再逐步细化）。

### 5.3 增量缓存
- 以声明为粒度：`decl_checked_upto` + 每条声明的依赖（名字引用图）；
- 改动声明 i → 重查 i..n（先保守整后缀，再做依赖裁剪）。

### 5.4 LSP 壳
- 用最小 JSON-RPC over stdio（依赖可只加 serde_json，避免大框架），实现：
  `initialize / didOpen / didChange / didSave / publishDiagnostics / hover /
  documentSymbol / (command: reduce | prove.*)`；
- VS Code extension 本体很薄：语法高亮 + 启动 LSP + 树状进度视图（L2 再画 UI）。

## 6. 与 ROADMAP 的关系（修正）

- L0 不变：编译器 + 文档服务（含 type_map、逐声明状态、增量 API 雏形）。
- **L1 从"文本事件 service"改成"文档服务 + LSP server"**：文本 JSON Lines 保留为
  agent/测试的传输层（protocol.md），编辑器走 LSP。
- L2：VS Code extension（薄壳 + 进度树 + goal 视图）。
- L3：agent 消费 LSP/文档状态讲课、出题（每个练习是带名字的声明）。

## 7. 工作流（按依赖排序）

| # | 工作流 | 内容 | 验收 |
|---|---|---|---|
| I0 | 地基（已完成大部分） | docs、`--json` 传输、错误 stage/code、CI、语料测试 | `cargo test --workspace` 绿 |
| I1 | 逐声明状态 + DocumentReport | 一个文件 → 每个声明独立 open/checked/failed；`???` 只影响自身 | 恒等练习：文件含另一条坏声明时，练习仍能单独判 solved |
| I2 | 错误细分 + 提示 | stage 之下稳定 kind；kernel 拒绝归类；hint 模板 v1；related info | `fun n => n + 1` 填进恒等练习 → "类型不匹配（期望返回 n）" |
| I3 | 类型图 v1 | 悬停子表达式给类型（先覆盖显式 binder 情形） | 编辑器原型悬停 `add1`/`x+1` 显示类型 |
| I4 | 增量 v1 | 改声明 i → 重查后缀，事件带版本 | 追加一行只重编译受影响声明（日志可验） |
| I5 | LSP server v1 | publishDiagnostics/hover/documentSymbol + didChange | 用 VS Code 打开课程文件：错误波浪线、悬停类型、练习状态 |
| I6 | prelude 对齐 + elaborator 推进 | Bool/Eq/rfl、binder 推断、`let`、`match`（课程需要） | 新增语法 TDD 三件套 |
| I7 | 第一门课（M4） | 5 单元课程 + golden | CI 全绿 |
| I8 | goal 视图 / code action | `#prove` 逻辑入库 + LSP 命令 | 在编辑器里三步完成 `a -> a` |
| I9 | agent 事件（L3 前哨） | 文档状态 → 结构化事件（已有 `--json` 词汇扩展 exercise.solved/failed） | agent 可据状态自动出下一题 |

## 8. 风险与对策（更新）

| 风险 | 对策 |
|---|---|
| 又回到"第二套 Lean" | 无 `#`、全声明式；填洞后文件 = 官方 Lean 子集 |
| LSP 壳盖在没准备好的查询 API 上 | I1–I4 先做文档服务，I5 壳薄薄一层 |
| type_map 在依赖 binder 处做不动 | 先做"整声明类型 + 顶层子项"，再逐层细化 |
| kernel 错误不够细 | 前端先按消息归类 + hint（I2），kernel 显式错误单独立项 |
| 增量缓存复杂度爆炸 | 先"声明后缀重查"，依赖图裁剪后置 |
| 反馈只给人 | 每次判定同时产出机器状态（同协议词汇） |

## 9. 待确认

1. 命名练习就用 `def name : T` / `theorem name : T`（匿名用 `example`），可以吗？
   （若坚持 `example name : T`，需要接受"填完不是直接可进官方 Lean"，或走注释元数据方案。）
2. LSP 壳用**最小 JSON-RPC**（少依赖、可控），还是引入现成框架（如 tower-lsp）？
3. 优先级：先做 I1–I2（判定与错误细节），还是直接冲 I5（先把 LSP 竖起来再补细节）？
4. goal 视图（F5 / I8）是否算第一期必需，还是第二期？
