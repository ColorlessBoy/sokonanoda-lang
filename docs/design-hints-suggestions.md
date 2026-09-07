# 设计：提示阶梯（soko/hints）+ 下一步建议（goal 形状）

> 状态：设计定稿（2026-09-07，本轮实施）。依据：`docs/gap-analysis.md` #4/#9、
> 外部调研（lean4game hint 匹配、Deduce `?` 课堂实证、Lean TryThis 三通道、
> ITS hint-ladder 四级范式）。实现落点见 §6 文件分工。

## 0. 头脑风暴与取舍（记录结论）

1. **提示存哪里？** 备选：course.json 元数据 vs 画布注释指令。选**画布注释指令**：
   - 画布是共享表面（用户与 agent 同看），提示是老师写的叙事注释的严格化，
     与 `-- sokonanoda:prelude none` 指令先例一致；
   - 不给 `course.json` 增加第二份练习清单（避免与画布漂移）；
   - LSP 天然按文件工作，无需跨文件索引。course/ 的提示内容后续由教学轮补。
2. **渐进揭示状态存哪？** 备选：服务端会话状态 vs 客户端。选**客户端**
   （VS Code workspaceState，键 = uri+声明名）：服务器保持无状态（LSP 服务器
   单文档、可重启，不丢进度——进度本就该跟编辑器走）；协议只返回完整阶梯。
   遵循 WPI 实证：按钮不显示「还剩 N 条」，只给「提示」入口。
3. **答案级提示给什么？** 教学钥匙不直接给成品证明——按 NNG 访谈结论给
   **分步骨架**（如「先 `fun (a : Prop) =>`，再看目标还剩什么」）。内容规范
   写进 teacher skill（主会话更新），机制不管内容。
4. **建议（下一步）的验证边界**：所有 exact/rfl 类建议必须**先经 kernel 判定
   再呈现**（Lean TryThis 的重放验证模式）；refine/intro 是结构生成，kernel
   在学生落笔后终审。绝不做文本比对（REQUIREMENTS §2.8）。
5. **多洞状态的 exact 是潜在 bug**：现 `code_action` 对 spine 状态（如
   `And.intro ??? ???`）把「假设匹配整个 goal」的 exact 填进第一个子洞——类型
   必错。本轮改为**逐洞判定**（judge 扩展），顺带修复。
6. **建议数量上限**：每个请求 ≤ 3 条、每洞候选 ≤ 4 条，只标第一条
   `is_preferred`（TryThis/rust-analyzer 惯例，避免列表噪音）。

## 1. 提示阶梯：画布指令语法

```text
-- soko:hint 先看目标里最外层的箭头：它在说"给你一个假设 a"。
-- soko:hint 目标形态：把 a -> b 拆成 fun (x : a) => ???，剩余目标就是 b。
-- soko:hint 关键引理：本题不需要引理，identity 即可。
-- soko:hint 骨架：fun (x : a) => x（自己誊一遍，别整段复制）。
example : (a : Prop) -> a -> a := ???
```

- 语法：整行仅 `-- soko:hint <text>`（`--` 后一个空格 + 前缀）；**必须独占
  一行**（行首仅空白），跟在代码后的行尾注释不算（避免误挂）。
- 语义：每条 hint 挂到**它之后的第一条声明**（def/theorem/example/axiom/
  inductive 块均可；跳过 `#check` 等非声明命令）。声明之前的全部未消费
  hint 归它；最后一条声明之后的 hint 忽略（不报错——老师写到一半是常态）。
- 提示阶梯是**写作规范**（依次：思路→目标形态→关键引理→骨架），机制只
  保序，不解析层级数字。

## 2. front 侧

新模块 `crates/front/src/compile/hints.rs`：

```rust
pub struct SourceHint { pub offset: usize, pub text: String }  // offset = `--` 起点
pub fn source_hints(src: &str) -> Vec<SourceHint>;             // 按源码序
pub fn attach_hints(hints: &[SourceHint], decl_start: usize, consumed: &mut usize) -> Vec<String>;
```

- `check.rs` 在产出 `DeclState` 时：`source_hints(src)` 每趟算一次，按
  「声明 span 起点」顺序消费指针挂到 `DeclState.hints: Vec<String>`。
- `report.rs`：`DeclState` 新增 `pub hints: Vec<String>`。
- Session 增量自动成立：hints 在 `DeclState` 里随逐命令快照缓存；hint 注释
  变更 → 后续命令 start 偏移变化 → key 失配 → 重编译该后缀（保守但正确），
  零重编译路径的 span 重映射不影响 hints 文本。

## 3. LSP 侧：`soko/hints` 自定义请求

- 请求：`{"textDocument": {"uri"}, "position"}`（tower-lsp custom method；
  与 soko/goals 同风格，客户端 opt-in，不入 capabilities 广告）。
- 响应：`{"hints": ["…", "…"]}` —— 位置所在声明的完整阶梯（无声明/无
  提示 = 空数组）。服务器无状态、无揭示计数。
- 处理器落点：`crates/lsp/src/hints.rs`（新），main.rs 预接桩
  `Backend::hints` + custom_method 注册（主会话）。
- VS Code（subagent A）：练习树每个 open 声明加「提示」子节点 → 点击执行
  新命令 `sokonanoda.revealHint(declName, declRange)`：调 soko/hints → 取
  第 `revealed` 条用 `showInformationMessage` 展示并 +1（workspaceState 持久）；
  已展示完则温和提示「这个练习的提示都给你了」。命令面板回退：无参数时用
  光标位置找声明。package.json 声明命令 + extension.js 注册 +
  `crates/cli/tests/extension.rs` 契约同步。

## 4. 下一步建议：front::suggest + judge 逐洞判定

### 4.1 judge 扩展（`crates/front/src/judge.rs`）

```rust
/// 把 doc 中 decl_span 命令里的 hole_span 替换为候选 term，改名合成声明
/// 后走完整流水线判定（与 judge_terms 同语义：合成声明是唯一裁判）。
pub fn judge_hole_fill(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement>;
```

- 实现：取命令切片 → 用 `front::tokenize` 定位声明名 token（`example` 声明
  把首 token 换成 `def _soko_judge_k`）→ 换名 + 把 hole 切片替换为候选 →
  追加到 `parse(prefix)` 的命令表（prelude/可见性与 judge_terms 一致）→
  `check_document_with` → `judgement_of`。名字定位复用
  `front::references::decl_name_span`（主会话预置）。
- `judge_terms` 保持不变（REPL proof.rs 依赖）。

### 4.2 建议生成（`crates/front/src/suggest.rs` 新模块）

```rust
pub enum SuggestionKind {
    Exact { binder: String, hole: usize },   // 逐洞：第 hole 个洞填该假设
    Refine,                                   // 模板骨架
    Intro,                                    // 剥一层/多层 binder
    Rfl { term: String },                     // Eq 形状的 kernel 验证 rfl
}
pub struct Suggestion { pub kind: SuggestionKind, pub verified: bool }
pub fn suggest(prefix_src: &str, options: &CompileOptions, d: &DeclState) -> Vec<Suggestion>;
```

生成与排序（每请求 ≤3 条）：

| 优先级 | 形状 | 建议 | 验证 |
|---|---|---|---|
| 1 | 单主洞且某假设类型 ≡ goal | `Exact{binder, hole:0}` | judge ✓（defeq） |
| 1' | 多洞：假设 ≡ 该子洞期望类型 | `Exact{binder, hole:i}` | judge_hole_fill ✓ |
| 1'' | goal 形如 `Eq.{u} α x y` | `Rfl{term: "Eq.refl.{u} α x"}` | judge ✓，失败即丢弃 |
| 2 | `refine_template` 存在 | `Refine` | 结构生成 |
| 3 | goal 是 Forall/Arrow | `Intro` | 结构生成 |

- Eq 形状解析：goal 文本经 `proof::parse_expr_text` 还原 AST；头为 `Ident`
  且拼写为 `Eq`、实参数 ≥3 时合成 `Eq.refl.{u} <α> <a>`（u 取声明 universe
  首参或 0；实参取 goal AST 的前两个应用实参文本）。候选必须过 judge 才呈现。
- exact 候选每洞最多取前 4 个 binder（judge 批量判定一次成型）。
- 「巧思」类（选 witness、选归纳变量）**不自动建议**——那是人工阶梯
  tier 1/2 的职责（Deduce/WaterProof 共识）。

### 4.3 LSP code action 重排（`crates/lsp/src/actions.rs`）

- `code_action` 处理器（main.rs 预接）改为调用
  `actions::code_actions(uri, &doc, range) -> Option<Vec<CodeAction>>`。
- 映射：`Exact` → 洞位替换为 binder 名（逐洞洞位来自 `d.holes[hole]`）；
  `Rfl` → 洞位替换为 term；`Refine` → 现有 refine_edit；`Intro` → 现有
  intro_edit。标题保持中文教学语气：
  - `exact h（用假设 h 补第 2 个洞）` / `exact h（用假设 h 直接结束证明）`
  - `Eq.refl …（两边本来就是同一个值，rfl 即可）`
  - `refine And.intro a b ??? ???（按构造子拆分子目标）`
  - `intro 2 个 binder（把证明写成 lambda 的第一步）`
- **`is_preferred: Some(true)` 只标第一条**；QUICKFIX kind 不变。
- 行为回归锚点：`code_action_offers_exact_for_matching_hypothesis`、
  `code_action_exact_uses_kernel_defeq_not_text_match`、
  `code_action_offers_kernel_shaped_refine_skeleton`、
  `code_action_intro_still_offered_without_matching_hypothesis` 必须继续绿
  （标题/编辑内容断言不动，仅排序与新能力叠加）。
- 新测试：多洞逐洞 exact（spine 状态下假设匹配子洞类型 → 对应洞位的编辑，
  且不再出现「把整体匹配假设填进子洞」的错位建议）；rfl 建议（`Eq` goal →
  kernel 验证过的 `Eq.refl` 候选；非 Eq goal → 不出现）。

## 5. 验收

1. `cargo test -p sokonanoda-front`：hints 指令（挂接/跳过/独占行）、
   suggest 单测（正例：建议被 kernel 接受；反例：形状近似但 kernel 拒绝，
   确认不误报）全绿；
2. `cargo test -p sokonanoda-lsp`：soko/hints 请求（有阶梯/空）、逐洞 exact、
   rfl、is_preferred 恰一个；
3. `cargo test -p sokonanoda-cli`：extension 契约（新命令注册）全绿；
4. `playground.sokonanoda`：12 题挂上阶梯后 CLI `--json` 事件计数不变
   （checked=14 / open=12 / diagnostics=0）、exit 0；
5. 全仓库门禁（fmt/clippy/test）绿。

## 6. 文件分工（互斥清单）

| owner | 允许修改 |
|---|---|
| 主会话（预接，先行完成） | `docs/protocol.md`、`crates/lsp/src/main.rs`（能力/注册/桩/Doc.version）、`crates/lsp/src/lib.rs`+`Cargo.toml`（lib 化）、`crates/front/src/compile/{report,check}.rs`（DeclState.hints 字段 + 挂接一行）、`crates/front/src/references.rs`（种子 decl_name_span）、`crates/front/src/lib.rs`（模块 re-export） |
| A（提示阶梯） | `crates/front/src/compile/hints.rs`（新，含测试）、`crates/lsp/src/hints.rs`（新，含测试）、`editor/vscode/{extension.js,package.json}`、`crates/cli/tests/extension.rs`、`crates/cli/tests/common/mod.rs`（词汇 + soko/hints）、`playground.sokonanoda`（12 题阶梯内容） |
| B（下一步建议） | `crates/front/src/{judge.rs,suggest.rs}`（suggest 新建，含测试）、`crates/lsp/src/actions.rs`（含测试） |
