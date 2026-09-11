# 内核已定义名字的声明警告（reserved-declaration-name）

> 触发（2026-09-11）：学习者在画布写 `axiom Prop : Sort 1`。这行在本
> 编译器里能通过（`Prop` 内核已经定义过，代码里的 `Prop` 都指内核那个），
> 但声明出来的名字永远不会被任何引用命中。官方 Lean 里 `Prop` 已存在，
> 这行会直接报重复声明。用户要求：保留这行，但在 VS Code 里给出 warning。
> 方案取「前端产出 + CLI 与 LSP 两侧消费」（用户选定，2026-09-11）。

## 1. 问题

- `crates/front/src/parser.rs:512` 把表达式位置的 `Prop`/`Sort`/`Type`
  直接指向内核定义的那个，不查环境。于是 `axiom Prop : Sort 1` 声明出的
  常量 `Prop` 是一具空壳：可声明、可 `decl.checked`，但用不上。
- 对学习者，这行看起来像「定义 Prop」，实际用的是内核已定义的那个；在
  官方 Lean 里还会重复声明报错。需要一条 warning 点破，而不是删掉这行。

## 2. 设计

**判定纯语法**，不碰内核、不碰 elaborator：顶层声明名属于
`{"Prop", "Sort", "Type"}` 时产出一条 warning。语义与成功/失败无关
（即使声明本身内核拒绝，也照样提示名字问题）。

- 新模块 `crates/front/src/compile/warning.rs`：
  - `WarningKind::ReservedDeclarationName` → 机器码
    `reserved-declaration-name`，附教学 `hint`；
  - `CompileWarning { kind, message, span }`；
  - `collect_warnings(&FolFile) -> Vec<CompileWarning>`：遍历
    `Def`/`Theorem`/`Axiom`/`InductiveBlock`，名字命中保留集即产出，
    span 用 `references::decl_name_span` 收窄到名字 token（拿不到则退回
    声明 span）。
- 通道（两处，语义一致）：
  - `CompileOutput.warnings`（批量 CLI 视图）；
  - `DocumentReport.warnings`（LSP / agent 视图，会话零重编译路径也带）；
- 消费：
  - CLI `--json` 输出新事件 `warning`（`{type, human, code, message,
    hint, span}`，`human = warning[<code>]: <message>`），人类视图打印到
    stderr；warning 不影响退出码（只有 errors 决定成败）；
  - LSP 映射为 `DiagnosticSeverity::WARNING`（range = warning span，
    message = message + 提示），与既有 `sorry` warning 并列。

## 3. 取舍

- **不写进 `DeclState`**：DeclState 的构造点有四五处，加字段面大且与
  声明状态无关；warning 是纯语法，放独立通道更干净。会话零重编译路径
  由 `collect_warnings(&file)` 现算（解析每轮都会做，成本可忽略）。
- **不新增 `CheckEvent` 变体**：`CheckEvent` 表达内核/检查结果；warning
  不是检查结果，新增变体会牵动 course 计数等匹配点。独立 `warning`
  事件在协议层与 `diagnostic` 并列，语义更准。
- **不判此外的同名情况**（如用户数据名撞 prelude 的 `Nat`/`Eq`）：那是
  另一端（shadowing 由 `user_top_level_names` 处理），本警告只针对
  「内核已经定义、因此同名声明注定用不上」的名字。

## 4. 验收标准

1. `axiom Prop : Sort 1` 产出恰好一条 warning，code
   `reserved-declaration-name`，span 落在名字 `Prop` 上；声明本身仍
   `decl.checked`，退出码 0；
2. 普通名字（`axiom True : Prop`）零 warning；
3. `--json` 事件流含 `warning` 且仍在封闭词表内（协议一致性测试通过）；
4. LSP 在对应行给出 `WARNING` 严重级诊断，且不把文件判成错误；
5. 会话零重编译（仅改注释）后 warning 仍在；
6. `docs/protocol.md` 记录新事件；三层测试（front 单测 + CLI e2e +
   LSP）全绿；`sokonanoda gate` 通过。
