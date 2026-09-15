# 设计：`match` 模式编译器（字面量 / 嵌套 / 通配 / 守卫，2026-09-15）

> 触发：HANDOVER §3 B / ROADMAP I6 的「嵌套/守卫/字面量模式」。现状
> （`docs/design/match.md` §2）明确列为非目标：`match` 只接受**裸构造子名 +
> 位置模式变量**，且**每个构造子恰好一条 arm**（`arm_by_ctor` HashMap，
> `elab.rs:1220-1272`）。这让 `| 0 =>`、`| some (some x) =>`、
> `| succ k if p =>` 都无法表达。
>
> 关键结论：**嵌套模式必然要求同一构造子出现多条 arm**（`| some none => …`
> 与 `| some (some x) => …`），所以「每构造子一条」必须换成**有序 arm + 模式
> 矩阵**。这是模式编译器，不是局部改动。

## 1. 目标 / 非目标

**目标（v1）**
- 模式文法（递归）：`_`（通配）、`x`（绑定变量）、`Ctor p…`（构造子，可点号
  限定 `Nat.succ k` 或预置裸名）、`0|1|2…`（Nat 字面量，`k` 脱糖为
  `succ^k zero`）；
- 守卫 `| p if cond => body`：`cond : Bool`；假时落到**后续 arm**；
- arm **有序、首个匹配者胜**；覆盖性在编译期检查；
- 与既有 `match` 能力完全兼容：依赖 motive、递归 IH、参数化归纳、
  prelude `Nat`/`Bool`、`sorry` 分支、arm 与手写 recursor 判定一致。

**非目标（v1 不做）**
- `as` 模式、or 模式（`p1 | p2`）、多 scrutinee、`if/then/else` 表达式、
  `Decidable`/Prop 守卫（守卫限 `Bool`）；
- 元组/记录语法（本语言无）；字符串字面量模式。

## 2. 语法

```
match <scrutinee> with
| <pattern> [if <guard>] => <body>
| …
```
`pattern ::= '_' | ident | Ctor ('_' | ident | Ctor | num | '(' pattern ')')* | num`
歧义消解：arm 头部的 `ident` 若**是**当前 scrutinee 归纳类型的构造子名（含
`Ind.ctor` 或预置裸名）则按构造子解析，否则按**绑定变量**；`(...)` 内的
子模式同理（用该位置字段的类型判定）。字面量只允许落回 `Nat`（见 §4.4）。

> 说明：v1 不做「构造子名与变量名的全局重名消歧」——`_` 明确表通配；不像
> Lean 有 `ctor` 关键字。解析在**不知道类型**时无法判定 ctor vs 变量，故 parser
> 只产出「标识符或构造子候选」，**由 elaborator 在知道归纳类型后判定**
> （`Pattern::Ident` 在 elab 期解析成 `Ctor` 或 `Bind`）。

## 3. AST

```rust
pub enum Pattern {
    Ident { name: String, span: Span },   // 待 elab 判定：构造子或绑定
    Wild  { span: Span },                 // `_`
    Num   { value: String, span: Span },  // Nat 字面量
}
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,   // `if <cond>`
    pub body: Expr,
    pub span: Span,
}
```
用统一 `Pattern::Ident` + elab 期判定，避免 parser 需要类型信息；`Num` 与
`Ident` 区分在 parse 层即可。elab 期把 `Ident` 解析为 `PatOk::Ctor{name,args}`
或 `PatOk::Bind(name)`。

## 4. 降低算法（编译器）

对**有序行**（每行 = 模式串 + body + guard）做列式特化，生成嵌套
`Expr::Match`（复用现有每层「motive + minors + recursor」构造）：

```
compile(cols, rows, expected):
  if cols 为空: return rows[0].body          # rows 非空（否则 non-exhaustive）
  if rows 为空: error ElabMatchNonExhaustive
  # 选一列：优先 rows[0] 中第一个「可反驳」模式所在列
  col = first i where rows[0][i] 可反驳, else 0
  ind = cols[col] 的归纳类型
  if rows[0][col] 不可反驳且无守卫:
      # 该行在此列无条件匹配；但其它列可能有可反驳模式 → 仍需特化
      # （标准矩阵：不可反驳在该列 = 对每个构造子都展开）
  for ctor in ind.ctors:
      sub_rows = rows 中在此列「适用」的行，按顺序：
          Ctor(ctor,…)  → 用子模式替换该列（新增字段列）
          Ctor(其它)     → 丢弃
          Wild/Bind      → 用 k 个通配替换该列（k=字段数），并记绑定
      arms += (ctor, sub_rows)
  return flat_match(cols[col], ind, arms, expected)
```

要点：
- **列变量**用现有 `scope`（push 字段 binder）承载；每层都按现有方式推
  motive（常量或依赖，沿用 `match-dependent-motive`）、推 IH（递归字段后
  `ih`/`ih2`…，类型 `R[x:=field]`）、求 `level`，再组 recursor 应用。**每层
  复用同一套构造**，只把「每构造子一条 arm」换成「每构造子一条 arm，其
  body = 子矩阵的编译结果」。
- **守卫**：当最前适用行有 guard 时，该行 body 包成 `Bool` 条件
  （`match guard with | Bool.true => body | Bool.false => <下一适用行>`），
  复用 `Bool.rec`；守卫假 → 下一适用行；无后续适用行 → non-exhaustive。
- **字面量**：`Num k` 在 elab 期脱糖为 `succ^k zero` 的嵌套构造子模式
  （仅当被匹配类型是 prelude/源内 `Nat` 形状：零元 ctor + 一元 succ ctor）。
  `0` → 零元 ctor；`k>0` → 套 k 层 succ。
- **覆盖性**：编译期检查——每个位置的构造子集合被 arm 覆盖到「有不可反驳
  兜底」；否则 `ElabMatchNonExhaustive`（附缺失构造子）。
- **arity**：每个 `Ctor` 子模式的参数个数必须 == 字段数，否则
  `ElabMatchBadArm`；未知构造子名 → 若该类型下无此 ctor 且不是合法变量名
  位置 → `ElabMatchBadArm`。`_`/`Ident`（变量）不再与字段数挂钩。

### 4.4 Nat 字面量规则
- 仅当该位置类型是「构造子为 `zero`（0 元）+ `succ`（1 元，递归）」的归纳
  （prelude `Nat` 或同形状源内 `Nat`）时接受 `Num`；否则
  `ElabMatchBadArm`（提示该位置需要构造子/变量/`_`）。

## 5. 消费者同步（必改）

`arm.ctor`/`arm.binders` 的所有读点必须改为遍历 `Pattern`：
- `elab.rs::mentions_ident`（Match）、`goals.rs::{expr_has_hole,
  collect_hole_spans, substitute_names, with_root_span, goal_under_binders}`、
  `spine.rs::{mentions, substitute}`、`proof.rs::render_expr`、
  `semantic.rs::walk_expr`（含 `KEYWORDS` 加 `if`？）。
- 守卫表达式也要被 hole/mention/替身遍历。
- `semantic::KEYWORDS` 与 TM 语法：**不**把 `if` 升为全局关键字（避免破坏
  标识符与既有 token 契约）；仅在 arm 解析里识别 `if` token（`Ident("if")`）。
  因此 `#check if` 仍是普通标识符。语义着色里 `if` 落在 ctor/变量分类之外，
  保持 `UnknownIdent`（可接受；文档写明）。
- 渲染（hover/`render_expr`）：`| p if g => body`、嵌套括号、字面量。

## 6. 错误码

复用：`elab-match-bad-arm`（未知构造子/arity 不符/字面量用在非 Nat）、
`elab-match-non-exhaustive`（缺构造子或守卫无兜底）。**不新增 code**（避免
协议/穷尽测试三处同步）；文档写明含义扩展。

## 7. 测试三层 + 课程

- **front 单测**：`match_*` 新增：通配 `_`、嵌套 `some (some x)`、字面量
  `0/1/2`（含与 `succ k` 混排）、守卫 true/false 分支归约、同一构造子多条
  arm 的有序性、覆盖性错误、arity 错误、守卫无兜底错误、与手写 recursor 判定
  一致、依赖 motive + 嵌套、prelude `Nat`/`Bool` + 字面量/守卫。
- **CLI e2e**：`cli_match_nested_*` / `cli_match_literal_*` / `cli_match_guard_*`
  经 kernel 归约。
- **课程**：unit5 归纳节加「字面量与嵌套模式」小节 + 1 个练习；golden 更新。
- **白名单**：`docs/architecture.md §4.1` 语法白名单把模式文法写全。

## 8. 验收

- 上述能力全绿；既有 346+ front / 67+ CLI / LSP 测试不回归；
- `sokonanoda gate` PASS；
- 版本 **0.42.0**（新语法，minor）；设计 as-built 追加本文 §9。

## 9. 分阶段（实现顺序）

- **P1** AST `Pattern` + 递归 parser（含 `if`）+ 消费者改遍历 + 渲染；
  旧 lowering 换成「每构造子一条 arm，模式树仅 Bind/Wild」的等价实现
  （回归全绿）。
- **P2** 列式编译器：通配 + 嵌套 + 字面量（无守卫）。
- **P3** 守卫（`Bool` 条件 + 落到下一 arm）。
- **P4** 课程 + 文档 + golden + 白名单。

## 10. As-built（0.42.0，2026-09-15）

- **AST**：`Pattern { Wild, Num, Ident{name, args} }`；`MatchArm { pattern, guard, body, span }`。
- **Parser**：`parse_pattern`（递归、`(...)`、`_`、数字；遇 `if`/`=>`/`|` 停）；
  守卫 `if <expr>` 只在 arm 里识别（`if` 不升全局关键字）。
- **编译器**（`elab.rs`）：`ColVar`/`PatternRow`/`Resolved` + `compile_pattern_body`
  （列式特化 → 生成嵌套 `Expr::Match`）+ `guard_chain`（守卫 → prelude `Bool` 的
  match）；每层复用既有 motive/IH/level/recursor 构造。字段名取绑定名（canonical
  幂等）、撞构造子名则新鲜名；参数化字段先代入参数。**不写 de Bruijn**。
- **语义**：有序、首个匹配者胜；未知裸名 = 绑定变量（带子模式才 bad-arm）；
  覆盖不全/守卫无兜底 = non-exhaustive。
- **消费者**：`semantic`（新增 `add_pattern_binders` + 守卫遍历）、`proof::
  render_pattern`、`spine::{mentions,substitute}`（含守卫、模式阴影）、
  `goals`（hole/替身/依赖子目标；嵌套/守卫退回常量 R）。
- **测试**：front +8、CLI +3、课程 unit5 嵌套模式节 + 练习 9；
  golden `(10,8,4)→(11,9,6)`、汇总 `checked 54→55 / open 41→42`；gate PASS。
- **已知限制（后续）**：`as` 模式、or 模式、多 scrutinee、`if/then/else` 表达式；
  嵌套/守卫下的依赖 motive 子目标类型退回常量（保守）；守卫限 prelude `Bool`。

