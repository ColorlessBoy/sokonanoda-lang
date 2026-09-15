# 前端 elaborator：`let` 与 `match` 设计（2026-09-14 设计）

> **状态：只落设计，不动实现。** 本设计对应 `ROADMAP.md` §10 的 **I6**
> 「elaborator 推进：binder 类型推断 → `let` → 单构造子 `match`/递归」。
> 结论先行：**拆两个里程碑**——**Phase 1 只交付值位 `let`（本文档的
> 首个交付物，可立即开工）；`match` 推迟到 Phase 2**，本文只给设计与
> 可行性预研（§4），不写成实现规格（理由见 §1.3 / §4.1）。
>
> 硬规则约束（`REQUIREMENTS.md` §2 / `docs/architecture.md` §3）：
> 1. **kernel 冻结**，一行不改、不动热路径；只用既有内核能力；
> 2. 教学语法是真实 Lean 4 的**子集**，填完的文件放进官方 Lean 仍合法；
> 3. **新增语法 = 课程 + 测试 + 白名单三件套**（本文 §7 / §8）；
> 4. 判定永远走 kernel，禁止文本比对。

---

## 0. 一句话

`let` 是**局部的、有名字的中间值**：`let x : T := v; body`。内核早就有
对应构造（`Expr::Let` + `EnvBuilder::mk_let`，eval 走 zeta 归约），
所以 Phase 1 只需在 **parser + 前端 elaborator** 加一条路径，**不碰内核**。
`match` 要把模式编译成归纳消去子 `<Ind>.rec`，需要前端补上「归纳块登记表 +
结果类型/宇宙查询」两块基础设施，牵动面大，故拆到 Phase 2。

```lean
-- Phase 1（let）：等价于 (fun (x : T) => body) v，但保留绑定结构
def two : Nat :=
  let one : Nat := Nat.succ Nat.zero
  one + one

-- Phase 2（match，暂缓）：
def isZero (n : Nat) : Prop :=
  match n with
  | Nat.zero => True
  | Nat.succ _ => False
```

---

## 1. 动机与范围

### 1.1 为什么需要

- 课程里到处需要「先起个名字再往下写」：`let h : P := …; …`、把复杂项拆成
  可读的步骤。没有 `let`，学习者只能写整段嵌套表达式或将中间结果提升为
  顶层 `def`（污染环境、且必须写成完整类型签名）。
- 归纳类型出题（单元⑤）离不开**对构造子的分情况**。`match` 是 Lean 教程
  里模式匹配的入口，也是 I6 明确列出的下一步。
- `let` 是继续推进 elaborator 的低风险切口：它复用已冻结的内核 `Let`，
  验证「前端能安全消费内核公开 API」这条路径；`match` 则验证「前端能自己
  合成 recursor 应用」这条更重的路径，适合作为后续里程碑。

### 1.2 范围

| Phase | 交付物 | 内容 |
|---|---|---|
| **1（本文首个交付物）** | 值位 `let` | `let x : T := v; body`（带显式类型），任意 term 位置；`#check`/`#reduce`/`by` 内均可出现 |
| **2（推迟）** | 值位 `match` | 单构造子与多构造子、非依赖 motive、简单递归（IH）→ `<Ind>.rec` |

### 1.3 明确非目标（v1 与 Phase 2 均不做，除非另立设计）

- **Phase 1 不做**：无类型注解的 `let x := v`（见 §3.4）；`let` 作为 tactic；
  `let rec` / `where` / mutual；一次绑定多个名字（`let x y := …`）；
  `let` 的 pattern / `let ⟨a, b⟩ := …`。
- **Phase 2 不做**：依赖模式匹配（dependent motive）、嵌套模式、字面量模式、
  通配以外的守卫、多 scrutinee、`if/then/else`、`match` 返回类型省略、
  结构字段记法（`x.field`）、类型类/实例。
- **永远不做（硬规则）**：为 `let`/`match` 给内核加接口或改语义；用文本
  替换判定 `let` 等价；把语法塞进 parser 却不出课（白名单即课程）。

---

## 2. 语法与 AST 增加

### 2.1 关键字归属（白名单）

- `let`：**term 关键字**。在 `parse_expr`（`crates/front/src/parser.rs:418`）
  的派发里识别，与 `forall` / `fun` 同级；同时把它加进
  `starts_atom`（`parser.rs:547`）的排除集，避免 `f let …` 被当成
  `f` 与标识符 `let` 的应用。
- `match` / `|` / `=>`：Phase 2 的 term 关键字，词法器已有 `FatArrow`，
  需要新增 `|`（竖线）token。
- `let`/`match` **不是**命令关键字，不进 `is_reserved_command`
  （`parser.rs:944`）；它们是表达式，命令解析不受影响。

### 2.2 `let` 语法（Phase 1）

```text
let x : T := v ; body
```

- `x`：binder 名（单个，不支持 `(x : T)` 的括号形式——与 Lean 一致，
  `let (x : T) := …` 不是合法 term）；
- `: T`：**Phase 1 必填**（§3.4）；
- `:= v`：被绑定的值，按 term 解析（可含 `fun`/嵌套 `let`/括号）；
- `;`：与 body 的分隔符（复用已有 `TokenKind::Semicolon`），
  跨行合法（教学子集不引入缩进敏感）；
- 右结合、可嵌套：`let a : A := …; let b : B := …; body`。

`parse_let` 挂在 `parse_expr`：

```
parse_expr:
    Forall            -> parse_forall
    Ident("fun")      -> parse_lambda
    Ident("let")      -> parse_let      // 新增
    Ident("match")    -> parse_match    // Phase 2
    _                 -> parse_arrow
```

`parse_let` 产出 `Expr::Let`；`v` 用 `parse_expr`（不是 `private parse_app`：
`let` 的值可以整段是 term）。**注意**：`by` 块只在 `parse_value`
（`parser.rs:153`）识别，故 `let x : T := by …; body` 在 v1 是语法错误
（见 §5.1）。

### 2.3 `match` 语法（Phase 2 预研）

```text
match e with
| Ctor1 x1 … xk => body1
| Ctor2 …        => body2
…
```

- 只允许**裸构造子名**做模式（不做 `Ind.Ctor` 限定、不做字面量/嵌套/`_`
  以外的通配名）；
- 每个构造子必须恰好覆盖一次（顺序任意，前端按构造子声明序重排）；
- 暂不写返回类型注解（依赖结果类型；见 §4.3）。

### 2.4 提议的 `Expr` 变体（`crates/front/src/ast.rs`）

```rust
pub enum Expr {
    // …既有变体（Sort/Ident/Num/Hole/App/Lambda/Forall/Arrow/Plus/By）…

    /// Phase 1：`let x : T := val; body`。
    /// binder 复用 `Binder`（`ty` 在 Phase 1 恒为 `Some`），
    /// `binder.span` 收窄到名字+注解，供 hover / go-to-definition。
    Let {
        binder: Binder,
        val: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },

    /// Phase 2（预研，实现前可调整）：`match scrutinee with | ctor … => body`。
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
}

/// Phase 2：一条模式分支。
pub struct MatchArm {
    pub ctor: String,        // 裸构造子名
    pub binders: Vec<String>,// 模式变量（按字段位置）
    pub body: Expr,
    pub span: Span,
}
```

配套改动（机械、低风险）：

- `Expr::span()`（`ast.rs:69`）补 `Let` / `Match` 分支；
- `elab.rs::mentions_ident`（`elab.rs:745`）与 `goals.rs::expr_has_hole`
  （`goals.rs:275`）补 `Let`（递归进 `val`/`body`）；`Match` 在 Phase 2 补；
- `proof.rs::render_expr`（骨架/hover 文本）补 `let` 的打印
  （`let x : T := v; body`），用于展开补全 / 课程输出。

---

## 3. `let` 降低到内核项（Phase 1，首个交付物）

### 3.1 内核能力证据（无需改内核）

| 能力 | 证据 | 说明 |
|---|---|---|
| 构造 `Let` | `crates/kernel/src/builder.rs:186` `mk_let(name, ty, val, body, nondep)` | 公开方法，front 可直接用 |
| 类型检查 | `crates/kernel/src/infer.rs:162-175` | `Let` 在 `Check` 下检查 `val : binder_type`，再把 `dom` 推入 ctx 检查 body |
| 求值（zeta） | `crates/kernel/src/eval.rs:606-614` | `Let` 求值 = 依次把 `val` 推入 env、继续求 body（即 zeta 归约） |
| 打印 | 内核 pp 已有 `Let` 分支 | proof term / 错误信息能正确显示 |

即：**`let` 与 `(fun (x : T) => body) v` 在 kernel 里定义相等**（zeta）。
这条等价性是 §7 契约测试的基础。

### 3.2 elaborator 算法（`crates/front/src/compile/elab.rs`）

在 `elab_expr`（`elab.rs:450`）加 `Expr::Let` 分支，仿 `Expr::Lambda`
（`elab.rs:612`）的 scope 处理：

```
Expr::Let { binder, val, body, span } =>
  base = scope.len()
  // 1) binder 类型在“未引入 x”的外层 scope 里 elaborate
  ty = elab_expr(builder, binder.ty.ok_or(ElabUntypedBinder)?, scope, …)
  // 2) binder 声明行 hover（`x : T`），scope 仍是外层
  record_binder_hover(hovers, scope, binder.span, ty)
  // 3) 值在期望类型 T 下 elaborate（未注解的 lambda binder 可借此推断）
  v = elab_expr(builder, val, scope, univ, known, hovers, Some(ty))
  // 4) 引入 x，body 在扩展 scope + 外层 expected 下 elaborate
  scope.push(binder.name, ty, binder.span)
  b = elab_expr(builder, body, scope, univ, known, hovers, expected)
  scope.truncate(base)
  // 5) 拼内核 Let 并落 hover
  out = builder.mk_let(name_ptr, ty, v, b, /*nondep=*/false)
  record_hover(hovers, scope, span, out, None)
```

要点：

- **scope 顺序不能错**：类型在外层、body 在内层；`x` 在 body 里按已有
  `Ident` 分支解析成 `Var(idx)`（`elab.rs:506-545`），名字的 span 进
  `ResolvedTarget::Binder`，go-to-definition / references 自动可用
  （与 lambda/forall binder 同机制）；
- **期望类型传递**：`val` 用 `Some(ty)`，使 `let f : Nat -> Nat := fun x => x`
  这类省略 lambda 注解的写法能借期望类型推断（复用 `peel_expected`，
  `elab.rs:419`）；`body` 继续用 `expected`（整个 `let` 的期望）；
- **shadowing**：scope 用 `rposition` 就近解析，内层 `let` 遮蔽外层同名
  天然正确。

### 3.3 `nondep` 取值

内核 `LetData.nondep` 目前**只参与 hash-consing 的键**（`builder.rs:194`），
`eval`/`infer`/`quote` 均不读它（`eval.rs:606` 的循环忽略它）。因此
Phase 1 **保守取 `false`**（语义上等价于“body 可能依赖 x”），永远 sound。
若将来想省 hash 键，可加一个「body 是否引用该 binder」的自由变量扫描，
但**不在本设计内**（收益低、易错）。

### 3.4 为什么 v1 要求显式类型注解

`mk_let` 必须提供 `binder_type`，且内核在 `Check` 下会拿它与 `val` 的
推断类型做 `def_eq`（`infer.rs:166-170`）。对 `let x := v`：

- 前端 elaborator **无法**在构建期拿到 `v` 的类型——`EnvBuilder` 只暴露
  `declaration_count`，没有 `get_type`/`infer`（`kernel/builder.rs:234`）；
- 唯一的替代是 `by`/`funapply` 已用的「合成 `#check` 问内核」
  （`judge_infer`，`judge.rs:161`），但那需要把前缀源码、编译选项、
  当前 binder 上下文穿进 `elab_expr`（当前签名已 7 个参数，调用点遍布
  `build_*` / `install_inductive_block`），改动面与风险都显著更大。

按 `docs/architecture.md` §4.1 / §8 既有纪律（binder 必须显式类型，
binder 类型推断是 I6 的**前置**子任务），v1 要求
`let x : T := v`；缺注解报 `elab-untyped-binder`（复用既有码，
message 定制为「`let` 的绑定需要类型标注，例如 `let x : Nat := 1; …`」）。
无注解 `let` 作为 Phase 2+ 的独立小切片，走 §3.4 的 `judge_infer` 路线。

### 3.5 降低后的判定

`let` 只是内核 `Let` 节点，**判定完全交给完整 kernel**：
`build_def`/`build_theorem`/`build_example`（`elab.rs:284/306/327`）
照常把值交 `try_check_declar`。前端**不做任何 `let` 等价性检查**——
zeta 等价由内核的 conv 负责。

---

## 4. `match` 推迟到 Phase 2：理由与预研

### 4.1 为什么必须拆

`match` 要合成为归纳消去子应用 `<Ind>.rec motive m₁ … mₙ scrutinee`，
前端当前**缺三块基础设施**：

1. **归纳块登记表**：`elab_expr` 只拿到 `known: HashMap<String, Vec<String>>`
   （`elab.rs:13` 的 `UnivMap` 旁边，`known` 仅存**宇宙参数名**，
   `check.rs:258`），拿不到「构造子字段望远镜 / 递归字段 / 构造子声明序 /
   recursor 名」。要做 match，得先建一张从源码 `InductiveBlock`
   （`ast.rs:169`）+ prelude 派生的前端索引，并把它穿进 elab；
2. **结果类型/宇宙查询**：recursor 的 motive 是 `(x : Ind) -> Sort u`，
   需要显式实例化 `u`。前端没有类型推断（构建期无法问内核），
   无法确定 `match` 返回类型落在哪个 `Sort`；
3. **依赖 motive**：非依赖 `match` 只能覆盖玩具场景；真实的结构递归证明
   需要 `motive := fun x => P x`，这要求前端能做模式变量的类型实例化与
   索引处理——远超前端的现状。

因此 `match` 是**独立的、更重的里程碑**（Phase 2）。Phase 1 用 `let`
先把「前端消费内核公开 API」这条路走通，再回头补 `match` 的基础设施。

### 4.2 预研方案（recursor 路线，仅备查）

若启动 Phase 2，建议如下（**实现前另立设计定稿**）：

- **单构造子**：单 minor 的 `Ind.rec`；对结构体（如 `And`）可进一步降为
  投影 `mk_proj`（`builder.rs:201`，内核已有 `Expr::Proj`）——没有 recursor
  universe 负担，是 `match` 的第一块低垂果实；
- **多构造子**：`Ind.rec`，按构造子声明序生成 minors，用户写的分支重排；
- **简单递归**：`Ind.rec` 的递归字段 minor 自带归纳假设参数
  （`Nat.rec` 的 succ minor 形状 `(m : Nat) -> (ih : motive m) -> motive (succ m)`），
  前端需为用户没写的 IH 位置合成 binder 名（`ih`/`ih2`）；是否把 IH
  暴露成可引用名字、还是仅补齐占位，是 Phase 2 的开放决策；
- **motive**：v1 只做非依赖 `fun (_ : Ind) => R`；`R` 的宇宙层级用
  `judge_infer` 式内核查询确定（复用 `funapply` 的合成声明路线，
  `docs/design/term-apply.md` §1）。

### 4.3 Phase 2 的 AST（暂定）

见 §2.4 的 `Expr::Match` / `MatchArm`；细节在 Phase 2 设计里冻结，本文
只钉住「裸构造子 + 非依赖 + 结果类型由内核查询确定」这三条边界。

---

## 5. 与既有特性的交互

### 5.1 `by` tactic 块

- **值位 `let` 与 `by` 正交**：tactic 白名单（`intro/exact/apply/assumption/
  rfl/sorry`，`parser.rs:192`）**不加 `let`**，本轮不做 `let` tactic；
- `let` 的**值**不支持 `by`（`by` 只在 `parse_value` 识别，§2.2）——
  学习者若要「先战术证明再绑定」，写成
  `theorem t : T := by …`，或在 body 里用已证的名字。这是刻意的 v1 边界；
- `let` 出现在 `by` 块**降级后**的产物里目前也不可能：tactic 引擎只搬
  已有表达式，不生成 `let`。因此 `by` 的 per-step 状态（`ByStep`）零改动。

### 5.2 `funintro` 已移除——`let` 不是值位关键字补全

`funintro`/`funapply` 已在 0.27.0/0.22.0 移除
（`docs/design/remove-funintro.md`、`value-keywords-v2.md`）。值位从此只有
**普通表达式 + `by` 块**。`let` 是**正规语法**，不是「关键字触发自动补全」
的魔法命令，因此：

- **不做** LSP 展开补全 / `expandLet` 命令 / hover 按钮（与已删关键字
  划清界限）；补全层至多在识别 `let` 前缀后给一条普通语法 snippet，
  属可选增强，不在验收内；
- 语义高亮如需高亮 `let`，走 `semantic.rs::KEYWORDS` 的普通关键字项，
  不新增特殊通道。

### 5.3 `sorry` 洞

`let` 给 hole 增加两类合法位置：

- `let x : T := sorry; body`——**值位洞**，期望类型即 `T`；
- `let x : T := v; sorry`——**body 洞**，期望类型是整个 `let` 的期望。

**必须改 `open_goal`**（`goals.rs:233`）：其 walk 目前只认
lambda/构造子 spine/函数实参；要新增 `Expr::Let` 分支：

- 若 `val` 含洞 → 产出子目标 `T`（`SubGoal`），其余目标由 body 继续 walk；
- 若 `val` 闭合 → 把 `x` 加进局部假设（`GoalBinder`），继续 walk body；
- 完全闭合的 `let` 不产生额外目标。

同时补 `expr_has_hole`（§2.4）。`let` 里的 `sorry` 是**源码里真实存在的洞**，
与 `funapply` 的「合成洞」不同（`term-apply.md` §6），因此**不需要**
打标 / 路由修正——填洞判定走既有 `judge_hole_fill` 的源码切片逻辑。

### 5.4 goal view / `soko/stateAt` / hover / inlay / session

| 面 | `let` 的影响 | 依据 |
|---|---|---|
| `soko/goals` / 练习列表 | 自动生效（由 `DeclState.holes`/`sub_goals` 驱动；前提是 §5.3 的 `open_goal` 改动） | `goals.rs`；协议 `docs/protocol.md` |
| `soko/stateAt` / tactic hover | **零改动**：`let` 不是 tactic，不进 `by_steps` | `by-tactics.md` §6 |
| hover | `let` 名字 hover 显示 `x : T`（`record_binder_hover`）；表达式 hover 显示类型 | `hover-refactor.md` |
| inlay / nextHole | 由洞/子目标驱动，自动覆盖新位置 | `judge`/`suggest` |
| go-to-def / references | `scope.push` 带 binder span → 自动可用 | `references.rs` |
| session 增量 | 洞/hover 已带 span，随 `remap_prefix` 平移即自动正确；**新增独立带 span 字段才需补 remap** | `session.rs:271-319` |

---

## 6. 错误码（三件套：`ErrorKind` + protocol + hint）

Phase 1 **不新增 ErrorKind**（避免协议churn）：

| code | stage | 触发 | hint 要点 |
|---|---|---|---|
| `elab-untyped-binder`（复用，message 定制） | elab | `let x := v` 缺类型注解 | 「`let` 的绑定要写类型，例如 `let x : Nat := 1; …`」 |

- `hint()` 目前文案（`error.rs:135`）说「例如 `fun (x : Nat) => x`」，
  建议顺带补一句 `let` 例子；
- 若 Phase 2 为无注解 `let` 走 `judge_infer`，其类型查询失败需要新码
  （暂定 `elab-let-type-query-failed`），届时随 Phase 2 设计落地。
- `docs/protocol.md` 的错误码清单由
  `protocol_doc_lists_every_error_code`（`compile/tests.rs`）守护；
  复用码无需改清单。

---

## 7. 测试计划（三层）

### 7.1 front 单元测试（`crates/front/src/compile/tests.rs`）

- **parse**：`let` AST 形状（binder/ty/val/body/span）；缺 `;`、缺 `:=`、
  缺类型注解 → 教学 parse/elab 错误；嵌套 `let`；`let` 在 `fun` body /
  括号内；`starts_atom` 不把 `let` 吃成标识符；
- **compile（kernel 终审）**：
  - `def two : Nat := let one : Nat := Nat.succ Nat.zero; one + one` →
    `decl.checked`；
  - 类型不匹配 `let x : Nat := Prop; …` → `kernel-rejected`；
  - body 引用 binder、内层 shadow 外层、`let` 遮蔽后作用域正确；
  - binder hover 行 `x : T` 与 `Ident` 解析到 binder span（go-to-def）；
  - `#check (let x : Nat := 1; x)` / `#reduce (let x : Nat := 1; x + 2)`；
  - `let f : Nat -> Nat := fun y => y; f 3`（期望类型推理）；
- **open goal**：`example : Nat := let x : Nat := sorry; x` →
  Open、子目标期望类型 `Nat`；`… ; sorry` → body 洞；
- **契约（zeta 等价）**：同一 `let` 与其 beta 展开
  `(fun (x : T) => body) v` 的 `status`/`goal`/洞数一致——
  钉死「降低只做结构、判定走内核」；
- 既有断言对齐（course golden 计数变化时同步）。

### 7.2 CLI 端到端（`crates/cli/tests/cli.rs`）

- 含 `let` 的文档 `--json`：`decl.checked` / `exercise.open` 正常；
- 缺注解的诊断码 `elab-untyped-binder` + hint；
- `#check`/`#reduce` 的 `expr.typed` / `expr.reduced` 与 let 交互正确。

### 7.3 课程 golden（`crates/cli/tests/course.rs`）

- 课程新增 `let` 一节（§8）→ 更新对应单元的
  `(decl.checked, exercise.open, expr.reduced)` golden 与汇总；
- 英文镜像事件计数逐项相等（`en_mirrors_match_chinese_event_counts`）。

### 7.4 契约 / 白名单

- `docs/architecture.md` §4.1 的语法白名单补 `let`（Phase 2 补 `match`）；
- `protocol_doc_lists_every_error_code` 全绿（复用码不动清单）；
- skill / 教学手册（`skills/sokonanoda-teacher`）的判定细节不涉及新词表。

---

## 8. 课程与文档（三件套之「课程」）

- **课程切入点**：单元③「函数与箭头」新增「局部绑定 `let`」小节——
  讲清 `let x : T := v; body` 是「给中间结果起名」，与顶层 `def` 的区别，
  以及它与 `(fun (x : T) => body) v` 等价；配 1–2 道练习
  （如 `def twice : Nat -> Nat := fun n => let m : Nat := n + n; m`，
  一道值位 `sorry` 的填空题）；
- **中文 + 英文镜像 + 解答钥匙**（双语纪律，`course-bilingual.md`）：
  `course/unit3-functions-arrows.sokonanoda`、`course/en/…`、
  `course/solutions/…` 同步；
- **golden / course.json**：计数更新，文件清单不变（不新增单元号）；
- `docs/architecture.md` §4.1 白名单、`docs/TESTING.md`（如需）同步；
- **Phase 2（match）** 到时落单元⑤（显式归纳与递归）的续篇。

---

## 9. 验收标准

**Phase 1（`let`，本设计首个交付物）**

1. `let x : T := v; body` 在任意 term 位置解析、elaborate、被完整内核
   接受；`#check`/`#reduce`/`fun` body/括号内均可用；
2. `let` 值位 `sorry` 产生正确的 Open 状态与子目标期望类型；goal 视图 /
   inlay / nextHole 覆盖新位置；
3. zeta 等价契约测试通过（`let` 与 beta 展开判定一致）；
4. 缺类型注解报 `elab-untyped-binder` + 教学 hint；
5. **kernel 零改动**；fmt/clippy/`sokonanoda gate` 全绿；
6. 三层测试（front / CLI / 课程 golden）全绿，英中镜像计数相等；
7. 课程单元③（zh/en/钥匙）与白名单文档、STATUS / REQUIREMENTS §9 同步。

**Phase 2（`match`）**：本设计不承诺；待另立设计后，以「单/多构造子
非依赖 match + 简单递归 + 三层测试 + 课程」为验收，且同样 kernel 零改动。

---

## 10. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| `open_goal` 的 Let 分支漏掉 hole / 局部假设 | 练习 goal 视图错位 | §5.3 专门设计 + front 单测钉死；`expr_has_hole` 同步补 |
| parser 把 `let` 当标识符（`f let …`） | 误解析 / 难诊断 | `starts_atom` 排除 `let`；parse 单测覆盖；报「`let` 后要跟 binder 名」 |
| scope 顺序写反（类型写进内层 / body 写进外层） | 名字解析错、内核拒绝 | 严格照 Lambda 的 `base`/`truncate` 模式；依赖 binder 单测 |
| `nondep=false` 带来重复 hash 键 | 极小的 intern 开销 | 统一 `false`；`let` 出现频率低；若要优化另立切片，不阻塞 |
| 无注解 `let` 缺口 | 教学不如 Lean 顺手 | §3.4 明确为 Phase 2：走 `judge_infer` 类型查询；课程里显式讲「先写类型」 |
| `match` 被误以为随 Phase 1 一起交付 | 课程 / 文档承诺落空 | 本文 §1.3 / §4.1 / §9 三处显式声明推迟；ROADMAP I6 勾选状态按 Phase 拆 |
| Phase 2 recursor 合成出错（minor 顺序 / IH / 宇宙） | 内核拒绝或错判 | 另立设计；先用单构造子 + 投影的低风险路线；`judge` 终审不放松 |

---

## 11. 分阶段实现计划（供 subagent 任务书引用）

| 切片 | 内容 | 验收 |
|---|---|---|
| **S1** | parser：`let` 关键字 + `Expr::Let` + `span()` + `starts_atom` 排除 | parse 单测 |
| **S2** | elab：`Expr::Let` 分支 + hover/binder scope + `ElabUntypedBinder` | compile 单测、zeta 契约 |
| **S3** | goal：`open_goal`/`expr_has_hole` 的 Let walk + `render_expr` 打印 | Open/子目标单测 |
| **S4** | 课程单元③（zh/en/钥匙）+ golden + 白名单文档 | course golden、gate |
| **S5** | CLI e2e + 文档（architecture §4.1 / protocol 说明 / STATUS / REQUIREMENTS §9） | cli 测试、gate |
| **Phase 2** | `match`：另立 `docs/design/` 设计，再做归纳登记表 + recursor 合成 | 独立验收 |

---

## 12. as-built（2026-09-14，Phase 1 `let`，0.28.0）

- **AST**：`Expr::Let { binder, val, body, span }`（`ast.rs`）。
- **parser**：`parse_expr` 识别 `let`；`starts_atom` 排除 `let`；额外修
  `named_group_ahead`（`(let …)` 曾被当成多名字 binder 组）。
- **elab**：`Expr::Let` 分支（类型在外层 scope、`record_binder_hover`、值在
  `Some(ty)` 期望下、binder 入 scope 供 body、`mk_let(..., nondep=false)`）；
  缺注解 → `elab-untyped-binder`（hint 补 `let` 例子）。`spine`/`proof`/
  `semantic`/`goals` 的 `Expr` 匹配同步。
- **goal**：`expr_has_hole`/`collect_hole_spans`/`goal_under_binders` 支持
  `let`；值位洞产出子目标（期望 `T`）、body 洞把 `x : T` 加入局部假设后再走。
- **测试**：front +21（parse 8 / compile 13，含 zeta 等价契约）、CLI e2e +3、
  课程 unit3 新增 `let` 小节（zh/en/钥匙，`def`/`#reduce` 逐字节镜像）+
  golden `unit3 (1,4,1)→(2,6,2)`、汇总 `checked 47→48 / open 34→36`。
- **文档**：`architecture.md` §4.1/§8、`TESTING.md` §1 同步。
- **版本** 0.27.1 → **0.28.0**（新语法 → minor）。
- **未做（Phase 2）**：`match`、无注解 `let`（走 `judge_infer` 查询）。

---

## 13. as-built 续：无注解 `let`（2026-09-14，0.34.0）

原 v1 要求 `let x : T := v`（§3.4）。`match` 轮把 `ElabCtx { prefix_src,
options }` 贯通进 `elab_expr` 后，§3.4 的「走 `judge_infer` 查询」路线可直接落地：

- `Expr::Let` 的 elab 分支：`binder.ty` 为 `None` 时，用当前 scope 的
  `judge_binders()` + `render_expr(val)` 调 `judge_infer`（复用 128 条有界缓存）
  推断值类型，`parse_expr_text` 回 AST 后作为 binder 类型；`ty_src` 用推断出的
  AST，供 `expected_src` 与 scope 使用。
- 推断失败（如值位 `sorry`、无类型 binder）→ 新错误码
  `elab-let-type-query-failed`（protocol + 穷尽清单 + hint 已同步），提示补类型。
- 测试：front `let_without_annotation_infers_the_value_type`（`let x := Nat.zero; x`
  与 `let f := fun (n : Nat) => n; f`）+ `unannotated_let_that_cannot_be_inferred_
  reports_a_let_specific_error`；CLI `json_mode_unannotated_let_infers_or_reports_hint`；
  课程 unit3 注释更新（类型可省略）。kernel 零改动。
- 版本 0.33.1 → **0.34.0**（新能力 = minor）。

## As-built：应用位置的 binder 类型推断（2026-09-15，0.45.0）

- 现状：`fun x => …` 在**有期望类型**的位置早已能从期望望远镜推断
  （`elab_expr` 的 `Expr::Lambda` + `peel_expected`）；缺的是**无期望类型**的
  应用位置（`(fun x => x) 1`）。
- 实现：`annotate_application_lambda`（`elab.rs`）在做 `Expr::App` 之前，把
  spine `f a1 … an` 展平；若头部是带未注解 binder 的 `Lambda`，用
  `judge_infer`（kernel-backed）推断 `a_i` 的类型，作为第 i 个 binder 的注解，
  **源到源改写**后交回正常路径。支持柯里化 `(fun x y => x) a b`。
- 边界：实参不足以覆盖全部未注解 binder（`(fun x y => x) 1`）→ 仍报
  `elab-untyped-binder`；`#check fun x => x` 无期望无实参 → 同样报错。
- 测试：front `untyped_binder_is_inferred_from_the_application_argument`、
  `curried_untyped_binders_are_inferred_from_the_arguments`、
  `untyped_binder_still_errors_with_too_few_arguments`；CLI
  `cli_untyped_lambda_binder_is_inferred_from_the_argument`。

