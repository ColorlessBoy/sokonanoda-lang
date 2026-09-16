# 设计：高亮分类的单一起源（highlighting，2026-09-15）

> 触发（用户）：「lsp 的所有 hover 信息和 infoview 里的高亮对齐了吗？我看到样式不一样。
> infoview 的类型会有高亮，hover 里的信息没有。这种高亮统一接口出处的功能没有正确收拢。」
>
> 结论先行：**分类与文本只有一个出处**（`front::semantic`），但**颜色是两张表**
> （markdown 只能 TextMate，webview 用 CSS 类）。两张表都从 `SemanticKind` 派生并被
> 穷尽测试锁死，**不可能再各自漂移**；但 hover 与 Infoview 的**颜色永不 100% 相同**
> ——这是平台限制，不是实现缺陷。

## 1. 术语：分类 / 文本 / 颜色

- **分类**：一个词是 keyword / sort / def_use / ctor_use / binder / … ——由
  `front::semantic` 决定（编辑器语义 token、hover、Infoview 都用它）。
- **文本**：goal 状态的呈现是「`name : ty` 若干行 + `⊢ goal`」。
- **颜色**：由 *载体* 决定 —— 编辑器正文用 LSP 语义 token（主题 token 色），
  markdown 代码块只能 TextMate scope，webview 用 CSS 类（主题变量）。

## 2. 单一起源（唯一出处）

`crates/front/src/semantic.rs` 是**唯一**的语言知识来源，导出三样东西：

| 用途 | API | 载体 |
|---|---|---|
| 分类 | `SemanticKind` + `ALL` | 全部 |
| CSS 类名 | `SemanticKind::as_str()` → `.tok-<kind>` | Infoview |
| TextMate scope | `SemanticKind::tm_scope()` | 编辑器兜底 + 全部 markdown 围栏 |
| LSP 语义 token | `token_type_index()`（`crates/lsp`，查表） | 编辑器正文 |
| 文本投影 | `runs_to_text()` / `goal_text()` / `goal_runs()` | hover + Infoview |

### 2.1 canonical 映射表（`tm_scope()` / CSS / LSP token）

| kind | tm_scope | CSS 类 | LSP token |
|---|---|---|---|
| keyword | `keyword.other.sokonanoda` | `.tok-keyword` | KEYWORD |
| sort | `storage.type.sokonanoda` | `.tok-sort` | TYPE |
| inductive_name / inductive_use | `storage.type.sokonanoda` | `.tok-inductive_*` | TYPE |
| number | `constant.numeric.sokonanoda` | `.tok-number` | NUMBER |
| hole | `markup.inserted.sokonanoda` | `.tok-hole` | MACRO |
| def_name / def_use / theorem_name / theorem_use | `entity.name.function.sokonanoda` | `.tok-*` | FUNCTION |
| axiom_name / axiom_use / ctor_name / ctor_use | `entity.name.type.sokonanoda` | `.tok-*` | TYPE / ENUM_MEMBER |
| binder | `variable.parameter.sokonanoda` | `.tok-binder` | PARAMETER |
| unknown_ident | `variable.other.sokonanoda` | `.tok-unknown_ident` | VARIABLE |

## 3. 单一文本生产者

hover 与 Infowiew 的 goal 块**同源**：LSP 用 `front::semantic::goal_runs`/`goal_text`
生成文本与 runs；wire 上 `soko/stateAt`/`soko/goals` 下发同样的 `goal_runs`/`ty_runs`。
hover 的围栏内容 = `runs_to_text` 的投影（不再是另写一遍的 `format!("{} : {}")`）。
测试：LSP `hover_goal_text_equals_the_state_at_run_projection`（hover 文本 == stateAt
runs 的文本投影）。

## 4. 不可对齐的部分（平台限制，已声明）

- **markdown 代码块只能 TextMate 着色**，且 TM 是正则、**没有名字解析** → 无法区分
  「这个 `x` 是 def 还是 binder」。因此 hover 的颜色是**近似**（scope 级），
  Infoview 是**精确**（kind 级）。
- **行内代码**（单反引号）没有语言 id → 完全不着色；因此所有 hover 片段一律用
  ` ```sokonanoda `（围栏），散文里提到单个词（如 `sorry`）才用行内代码。
- **刻意纯文本**（VS Code 不渲染 markdown / 无法着色）：诊断消息、inlay hint、
  `TreeItem.description`、CodeAction 标题、CLI/REPL 输出、site 静态页。

## 5. 防漂移（测试锁）

- `crates/front/src/semantic.rs`：`tm_scope_is_total`（每个 kind 都有 scope）、
  `runs_to_text_round_trips`、`goal_runs_projects_*`。
- `crates/cli/tests/extension.rs`：
  - `tm_grammar_keywords_follow_the_single_source`（词表 == `keywords()`+`sorts()`+forall）；
  - `tm_grammar_declares_every_semantic_scope`（每个 `tm_scope()` 都出现在语法里）；
  - `infoview_colours_every_semantic_kind_from_the_single_source`（每个 kind 都有 `.tok-*`）；
  - `rendered_language_text_uses_the_sokonanoda_fence`（扩展 tooltip 用同语言围栏）。
- `crates/lsp`：`every_semantic_kind_maps_to_a_legend_entry`（LSP legend 覆盖 ALL）。

## 6. 落地记录

- 0.43.0：所有 hover/tooltip 统一 ` ```sokonanoda ` 围栏（原表达式 hover 是 `text`）。
- 0.49.0：`SemanticKind::tm_scope()` 单一表；hover 目标态改由 `goal_runs` 投影；
  TM 语法补齐 `variable.parameter` 等 scope；三处穷尽测试上线。
