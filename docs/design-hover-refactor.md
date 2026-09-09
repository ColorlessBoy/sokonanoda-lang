# Hover 重构：良构表达式 + 高亮范围（第十八轮设计）

> 日期：2026-09-09。触发：用户反馈「括号 hover 内容乱七八糟：有些是包含括号的
> 外部表达式」「`(Not a)` 和 `(And.right a (Not a) h)` 左括号右括号内容对不上」，
> 并要求——(1) 拿到**每个字符**的 hover 内容评估哪些展示不合理；(2) 把正确行为
> 设计成单元测试再开发；(3) 最终**知道 hover 内容对应的表达式范围**（高亮）。

## 1. 现状盘点（全仓库 16 个 *.sokonanoda 文件逐字符 dump）

> 工具：LSP 层对每个非空白、非注释字符执行与 `hover()` 相同的选择优先级
> （关键字静默 → 括号 → 精确命中 → ±2 邻近 → 声明），dump 全部 11229 行。
> 无 `$N` 泄漏（前端已保证）；2506 处声明名 fallback 属预期。

### 1.1 问题 A：lambda / Pi 的 binder 名悬停整段溢出（最常见，977 处）

hover 到 binder 名字（如 `fun (a : Prop) => …` 里的 `a`、`(P : Prop) -> …` 里的
`P`）时，最小 span 是整段 lambda / Pi，于是整段表达式 + 整段类型都吐出来：

```
192:7  "a" => "fun (a : Prop) (h : And a (Not a)) => (And.right a (Not a) h) (And.left a (Not a) h : forall (a : Prop), And a (Not a) -> False"
```

这正是用户说的「有些是包含括号的外部表达式」。根因：`elab.rs` 的 Lambda/Forall
只给整段 `fun …` / `forall …` 记一条 hover 行，**binder 名字本身没有自己的行**。

### 1.2 问题 B：括号组内表达式被截断（源切片缺右括号）

AST span 不含括号（`parse_atom` 的 LParen 分支直接返回内层 `Expr`），所以
`(h : And a (Not a))` 组内最大 span 是 `And a (Not a`，切片缺 `)`：

```
192:17 "(" => "And a (Not a : Prop"
192:35 ")" => "And a (Not a : Prop"
```

### 1.3 问题 C：hover 不返回 range（`range: None`）

所有分支都返回 `Hover { range: None, … }`——VS Code 无法高亮「这个 hover 到底
在说哪个表达式」。用户明确要求能看到范围。

## 2. 决议

### D1 binder 行（front `elab.rs` + `report.rs`，冷路径）

为每个 Lambda / Forall binder 记一条 hover 行：

- `span` = `binder.span`（完整标注，如 `(a : Prop)`）；
- `expr` = binder 的类型（`a` 的类型是 `Prop`、`h` 的类型是 `And a (Not a)`）；
- 新增标志 `binder: true`（`HoverNode` 与 `HoverType` 各加一个 bool）；
- `scope` = 该 binder **push 之前**的 scope（类型在 push 前已 elaborate，
  松散变量对齐正确）；`resolution = None`（不改 goto-def/高亮/rename 语义）。

效果：hover 到 binder 名字时，最小 span 变成这个 binder 行（小于整段 lambda/Pi），
展示 `a : Prop` / `h : And a (Not a)` 而非整段。`resolve_hovers` 对 binder 行跳过
infer（text 置空——binder 行渲染用源码切片，不用类型文本）。

### D2 括号组 = 良构整体（LSP `render.rs`）

`bracket_hover_at` 改为返回 `(range_span, content)`：

1. 配对 `(open, close)`；
2. **range_span = 整组 `(open .. close+1)`**（含括号）——保证含光标，满足 VS Code
   `MarkdownHover.isValidForHoverAnchor`（range 必须包含 anchor，否则 hover 不显示/闪退）；
3. 判定组内是不是 binder 标注：存在 `binder` 行且其 span == 整组 span
   （`(a : Prop)` 的 binder 行 span 恰好 = 整组）→ content = 剥外括号的声明切片
   （`a : Prop` / `h : And a (Not a)`）；
4. 否则 content = `组内文本 : 类型`，类型来自组内最大 span 的 hover 行
   （`And.right a (Not a) h` → `: Not a`；`Not a` → `: Prop`）。组内文本天然良构
   （解析器保证括号平衡）→ 问题 B 自愈。

### D3 一般悬停切片的括号平衡（LSP `render.rs`）

`hover_content` 与 range 统一走 `balanced_span(text, span)`：从 span 右沿向前扫描
（跳过 `--` 注释）补齐缺失的 `)` 使括号配平，得到**良构表达式**。非括号悬停
（精确命中 / 邻近 / binder 行）的 range 也用 balanced span。

### D4 统一返回 range

`hover()` 所有分支都带 `range = range_of(range_span)`：
- 括号分支：`(open .. close+1)`；
- 其它分支：`balanced_span`；
- 声明 fallback：声明 span。

### D5 展示格式

- 表达式行：`表达式 : 类型`（表达式 = 良构切片）；
- binder 行：`名字 : 类型`（剥外括号，不加 `: 类型` 后缀——声明本身已含类型）；
- 关键字静默、`$N` 防御、声明 fallback 不变。

## 3. 验收（全部落地为测试）

### front（compile/tests.rs）

- `hover_rows_have_binder_declaration_rows`：`fun (a : Prop) (h : And a (Not a)) => …`
  存在 binder 行 `(a : Prop)` / `(h : And a (Not a))`，`binder == true`；
- `hover_rows_include_pi_binder_rows`：类型 `(P : Prop) -> False -> P` 存在 binder 行
  `(P : Prop)`（Forall binder 也覆盖）。

### LSP（lib.rs tests）

- `hover_on_binder_name_shows_its_type_not_the_lambda`：hover `fun (a : Prop) => …` 的
  binder `a` → `a : Prop`（**不再**整段 lambda）；
- `hover_on_binder_name_h_shows_declaration`：`(h : And a (Not a))` 的 `h` → `h : And a (Not a)`；
- `hover_on_binder_annotation_bracket_shows_declaration`：`(h : And a (Not a))` 的 `(`/`)`
  → `h : And a (Not a)`（不再 `And a (Not a : Prop`）；
- `hover_on_binder_annotation_prop_shows_declaration`：`(a : Prop)` 的 `(` → `a : Prop`；
- 既有括号测试保持/对齐（`And.right … : Not a`、`And.left … : a`、`Not a : Prop`、
  `((p))` 四括号、`)` 不漏邻居、注释内括号）。
- **range 断言**：括号 hover 与普通表达式 hover 都返回非空 range，且 range 覆盖光标位置。

## 4. 不修项

- `(a : Prop)` 这类 binder 标注组，D2 通过 binder 行 span==整组 判定，展示声明本身；
- hover 到 `=>` / `=`（lambda 箭头/等号）仍展示整段 lambda——那是该 lambda 本身的类型，
  合理；不属本次范围；
- 声明 fallback（hover 到声明名）返回声明签名，不引入 range 以外的装饰；
- 内核冻结：本轮只在 front 冷路径（elab 记录行）+ LSP 渲染层改动，kernel 一行不动。
