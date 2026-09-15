# 设计：elaborator `match`（Phase 2 定稿，2026-09-14）

> 前身：`docs/design/elaborator-let-match.md` §4 只做预研，本文是**实现前定稿**。
> 目标：在权威子集里加 `match`（对归纳类型分情况），kernel 冻结、判定走内核。
>
> **可行性证据（本轮 spike）**：手写
> `Color.rec.{1} (fun (_ : Color) => Color) green red c` 被完整内核接受；
> 而**不带 `. {1}` 时内核拒绝**（`expected Sort(0), actual Sort(1)`）——recursor 的
> 宇宙参数必须显式给出。`match` 的降低因此必须**从结果类型的 Sort 推出 level**。

## 1. 范围（v1）

```text
match <scrutinee> with
| <Ctor> <binder>... => <body>
| <Ctor> <binder>... => <body>
```

- `<scrutinee>`：一个表达式（变量或项），类型是某个**源内 `inductive`**；
- `<Ctor>`：**裸构造子名**（与语言既有 `ctor green : Color` 一致，不带 `Color.` 前缀）；
- 每个构造子**恰好覆盖一次**（顺序任意，前端按构造子声明序重排）；
- `<body>` 可含 `sorry`。

## 2. 明确非目标（v1 不做，另立设计再说）

- **依赖 motive**（`motive` 依赖 scrutinee 的值）；v1 motive 恒为 `fun (_ : Ind) => R`；
- ~~**递归类型**（`Nat` 等有 IH 的类型）~~ **已解除（Phase 2，见 §10）**：递归构造子字段后自动插入归纳假设 `ih`（多个递归字段依次 `ih`、`ih2`…，类型 = 结果类型 R），branch 可直接引用，递归无需自引用；v1 的 motive 仍非依赖，故只覆盖「以 scrutinee 自身构造子直接递归」的用例；
- 嵌套/字面量/`as`/守卫/多 scrutinee/`if-then-else`；
- prelude 内建 `Nat`/`Eq` 的 match（它们没有源内 `InductiveBlock` 元数据）——v1 限**源内** `inductive`；
- `match` 作为 tactic；`match` 出现在**期望类型未知**的位置（报教学错误）。

## 3. 语法与 AST

- `parse_expr` 识别 `match`（与 `let` 同级）；`starts_atom`/`named_group_ahead` 排除。
- `Expr::Match { scrutinee: Box<Expr>, arms: Vec<MatchArm>, span }`；
  `MatchArm { ctor: String, binders: Vec<Binder>, body: Expr, span }`（binder 复用 `Binder`）。
- `|` 用既有 `TokenKind` 新增（竖线）或复用；`=>` 用 `FatArrow`；`with` 作为普通标识符匹配。

## 4. 归纳登记表（前端索引，内核冻结）

在 `run_pass` 的归纳块处理处，登记源内每个 `inductive`：

- 归纳名、参数/索引个数、`isRecursive`（复用 front 已算的 `is_recursive`）、
  构造子**按声明序**的 `(ctor 名, field binders: Vec<Binder>)`、recursor 名 `<Ind>.rec`。
- 该表在命令间**前向累积**（与 `known`/`GoalTemplates` 同一层），随 `elab_expr` 传入
  （或经既有 `known` map 旁的侧表）。

## 5. 降低算法（`elab.rs` 的 `Expr::Match` 分支）

前置：`match` 的**期望类型 R 必须已知**（来自声明 codomain / `let`/`fun` binder 注解 /
外层 `match` 分支的 R）。

1. elaborate `scrutinee`（无 expected），拿到其类型头（用既有 `judge_infer` 或 front 的
   轻量类型探测）→ 归纳名 `Ind`（须在登记表中，否则教学错误）。
2. 校验 arms：构造子存在、无重复、无遗漏、参数个数 ≤ 构造子字段数（v1 要求**写满**字段名）。
3. **level**：`judge_infer`(R) → 其类型文本映射宇宙：`Prop`→0、`Type`→1、`Sort n`→n
   （复用 `judge_infer` 的 128 条有界缓存；与 `let`/`by` 同一条「编译期合成 `#check`」
   路线，仅作用于被编辑块）。
4. **motive** = `fun (_ : Ind) => R`（匿名 binder；v1 非依赖）。
5. **minors**（按构造子声明序）：`fun (x1 : T1) => ... => fun (xk : Tk) => body_用户写的分支`。
   字段类型 `Ti` 来自登记表（参数已实例化）。**递归字段后额外插入 IH binder**
   （`ih`、`ih2`…，类型 = 结果类型 R；Phase 2，见 §10）。
6. 拼应用：`<Ind>.rec.{level} motive minor_1 … minor_n scrutinee`（recursor 的显式
   宇宙实例是 spike 证实的必要条件）。
7. 判定：交给完整内核（`try_check_declar`），前端不做任何等价性文本比对。

`open_goal`/`expr_has_hole`/`spine`/`proof::render_expr` 同步加 `Expr::Match` 分支
（洞在 branch body → 期望 R；scrutinee 洞 → 其类型）。

## 6. 错误码（复用为主）

| 情形 | code |
|---|---|
| 构造子名未知 / 重复 / 遗漏 / 参数过多 | `elab-unknown-identifier`（未知 ctor）或新增 `elab-match-bad-arm`（收集式消息） |
| scrutinee 不是已知源内归纳 | 新增 `elab-match-not-inductive` |
| 期望类型未知（无法定 motive） | 新增 `elab-match-no-expected-type` |
| ~~递归归纳（v1 不支持）~~ | `elab-match-recursive-unsupported`（Phase 2 起不再触发；枚举/文档保留以稳定 code 表） |
| 未覆盖全部构造子 | 新增 `elab-match-non-exhaustive` |

新增码需同步 `protocol.md` 与穷尽清单（`protocol_doc_lists_every_error_code` 会强制）。

## 7. 测试三层 + 课程

- **front**：parse（arm/嵌套/`|` 缺失/重复 ctor）；compile（非递归枚举两/三臂 `swap`、
  结构体单臂、`Prop` 结果、arms 乱序重排、`sorry` 在 branch、未覆盖/未知/递归报错、
  期望类型未知报错）；**与手写 `Ind.rec.{level}` 的等价契约**（同判定）。
- **CLI e2e**：含 `match` 文档 `decl.checked`/`exercise.open`；错误码 + hint。
- **课程**：unit5（归纳与递归）新增「`match` 分情况」小节（zh/en/钥匙）+ golden；
  因 v1 只做非递归，示例用自定义 `inductive` 枚举/结构体。
- **白名单**：`architecture.md` §4.1 补 `match`。

## 8. 验收

- 非递归源内归纳的 `match` 在任意「期望类型已知」的 term 位置可用，被完整内核接受；
- arms 覆盖率/重排/错误诊断正确；branch 里 `sorry` 的期望类型正确；
- 与手写 recursor 应用判定一致；kernel 零改动；`gate` 全绿；
- 文档/site/技能、STATUS、REQUIREMENTS §9 同步；版本 **minor**（新语法）。

## 9. 风险

| 风险 | 缓解 |
|---|---|
| 宇宙 level 推错 → 内核拒绝 | spike 已证 `.{level}` 必需；用 `judge_infer` 取 R 的 Sort 映射；测试覆盖 Prop/Type |
| 参数化归纳的参数实例化 | v1 从登记表按声明序 + 参数实例化；无参数优先，参数化随后 |
| scrutinee 类型探测不准 | 复用 `judge_infer`；探测失败 → `elab-match-not-inductive` 而非硬猜 |
| 登记表与增量 session 交互 | 登记表随命令处理重建/累积，与 `GoalTemplates` 同层；session 快照只存渲染结果 |

---

## 10. as-built（2026-09-14，v1，0.33.0）

- **语法/AST**：`TokenKind::Pipe`；`Expr::Match { scrutinee, arms, span }` +
  `MatchArm { ctor, binders: Vec<Binder>, body, span }`；`parse_match` 在
  `parse_expr` 顶层；`match` 进 `is_expr_keyword`；`with` 在 scrutinee 处借
  `scrutinee_depth` 终止应用 spine；arms 为 `| Ctor binder… => body`（裸 ctor）。
- **登记表**：`InductiveTable`（每归纳：ctor 声明序 + elaborated 字段类型 +
  recursor 名 + 宇宙元数 + `recursive`），随 `ElabCtx { prefix_src, options,
  inductives }` 传入 `elab_expr`；`expected_src` 与内核 `expected` 并列，使
  R 在每个位置可用。
- **降低**：scrutinee → 归纳头（局部变量快路径否则 `judge_infer`）→ 校验 arms
  → level = `judge_infer(R)` 映射（`Prop`→0/`Type`→1/`Sort n`→n）→ motive
  `fun (_ : Ind) => R` → minors 按 ctor 声明序 → `<Ind>.rec.{level} motive … e`
  （小消去 Prop 无 `.{…}`）。
- **错误码**（新增，已入 protocol + 穷尽清单）：`elab-match-bad-arm`、
  `elab-match-not-inductive`、`elab-match-no-expected-type`、
  `elab-match-recursive-unsupported`、`elab-match-non-exhaustive`。
- **测试**：front +19（parse 6 / compile 13，含与手写 recursor 的等价契约）；
  CLI e2e +4；课程 unit5 新增「match 分情况」小节（zh/en/钥匙，代码逐字节
  镜像）+ golden `unit5 (4,3,1)→(6,5,2)`、汇总 `checked 48→50 / open 36→38`；
  `architecture.md §4.1` 白名单 + `TESTING.md` 同步。
- **版本** 0.32.1 → **0.33.0**（新语法 minor）。
- **v1 边界（未做，见 §2）**：递归归纳（IH）、依赖/参数化归纳、prelude
  `Nat`/`Eq`、`match` tactic、嵌套/字面量/守卫模式、无注解 `let`。

### Phase 2：递归归纳 / IH（2026-09-15，0.34.0）

- **§2 递归非目标解除**：`InductiveTable` 的 `recursive` 标志已可用；降低时对每个
  构造子的**递归字段**（字段 `src_ty` 提到归纳名）自动追加一个 binder——名字避开
  既有绑定（`ih`、`ih2`、…，`fresh` 规则见 `elab.rs`），类型 = 结果类型 R
  （v1 motive 仍非依赖），位置紧跟该字段 binder 之后；branch 在本地 scope 引
  `ih` 即可，无需自引用。level/motive/minors 拼装与 v1 相同。
- **测试**：front `match_recursive_inductive_uses_the_induction_hypothesis`
  （递归 `Nat2` 的 `pred`/`add` + `#reduce add two two`）；CLI e2e +3
  （`cli_match_recursive_inductive_inserts_the_ih` /
  `cli_match_recursive_sorry_branch_is_open_exercise` /
  `cli_match_recursive_reduces_through_the_ih`）；课程 unit5 新增「`match` + 递归」
  小节（zh/en/钥匙，代码逐字节镜像）+ golden `unit5 (6,5,2)→(7,6,3)`、汇总
  `checked 50→51 / open 38→39`；`architecture.md` §4.1/§8、`TESTING.md` 同步。
- **仍未做（Phase 2 余项）**：依赖 motive（`motive` 依赖 scrutinee 的值）、
  参数化/带索引归纳、prelude 内建 `Nat`/`Eq` 的 match（无源内 `InductiveBlock`
  元数据）、`match` tactic、嵌套/守卫/字面量模式、无注解 `let`。
