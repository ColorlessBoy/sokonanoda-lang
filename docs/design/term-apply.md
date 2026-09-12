# 值位 `apply` 关键字 + 展开补全（2026-09-13 设计）

> 触发（用户原话）：「我希望参考刚才的 intro 设计新的 command apply，也能触发自动
> 补全和等价部分表达式。必要的时候可以要求用括号确定范围。」
>
> 姊妹篇：`docs/design/term-intro.md`（值位 `intro`，已实现并发布 0.17.0）。
> 本文**只落设计**；实现按 §9 的分阶段计划执行。

## 0. 一句话

`intro` 把**目标**拆成 binder；`apply` 把**一个已知的证明/函数**接到目标上，把它的
前提留成洞。两者是同一枚硬币的两面：

```lean
-- 目标 P，手上 h : Q -> P
theorem t (h : Q -> P) : P := apply h
-- 等价于（展开后）：
theorem t (h : Q -> P) : P := h sorry
```

`apply h` 与手写 `h sorry` **判定完全一致**；展开只是让结构显形，不展开也合法
（与 `intro` 同一条契约，见 §5）。

## 1. 为什么不能照抄 `intro`

`intro` 的全部输入是**声明类型**——它在 parse 阶段就已经是 AST，所以
`lower_intro_val`（`crates/front/src/compile/intro.rs:16-21`）可以纯 front 侧完成，
值**永不进内核**（`term-intro.md` §2.2 的核心不变量）。

`apply` 需要另一件东西：**被应用名字的类型望远镜**。调研结论（见 §8 证据表）：

| 需要的输入 | front 侧现状 | 结论 |
|---|---|---|
| 局部假设 `h` 的类型 | 只有渲染后的 String（`GoalBinder.ty`，`compile/report.rs:40-44`）；AST 上下文只存在于 `by` 引擎的 `GoalNode.intros`（`by.rs:53`） | **拿不到** |
| 全局名字的类型 | 有 `GoalTemplates`（`compile/goals.rs:41-44`，`check.rs:281` 构造），但 `peel_type`（`goals.rs:249-267`）**丢掉了 codomain**，`FuncTemplate` 只有 `binder_names`/`binder_tys`（`goals.rs:20-25`） | **不完整** |
| prelude 内置名 | 只有 Eq 家族进表（`goals.rs:55-66`），且限 Full 模式；`Nat` 是硬编码（`prelude.rs:112-149`）不入表 | **覆盖不全** |
| 内核查询 | `EnvBuilder` 只暴露 `declaration_count`（`kernel/builder.rs:234`），无 `get_type`/`lookup` | **无 API** |

`apply` 的判定核心是「`h` 的 codomain 与目标合一」，缺了 codomain 这张表就做不了。
而**最常用的教学场景恰恰是局部假设**（`theorem t (h : Q -> P) : P := apply h`），
它连表都不在——所以「扩展 `GoalTemplates`」这条路要么补一个并行的类型模型（与内核
二份真相，违反 LESSONS「判定永远走 kernel」），要么就得问内核。

### 决策：走内核推断，复用 `by` 引擎的既有机械

`by` 块的 tactic `apply` 已经解决了同一个问题（`crates/front/src/by.rs:275-368`）：

1. `judge_infer(prefix_src, options, &spec.binders, &term)`（`by.rs:288-298`，
   定义 `judge.rs:161-225`）——把 `#check fun <binders> => <term>` 合成源码送进
   完整流水线，取内核渲染的类型文本；
2. `parse_expr_text` 把类型文本读回 AST（`by.rs:299-308`，`proof.rs:45`）；
3. `peel_pi` 剥 Pi 链（`by.rs:457-488`）；
4. `unify_spine` 位置合一 codomain 与目标，把出现过的命名 binder 认成类型参数
   （`by.rs:492-516`）；
5. `substitute` 实例化类型参数、其余 binder 变成子目标（`by.rs:572-638`）；
6. `assemble` 拼 `Expr::App`（`by.rs:372-402`）。

**这六步值位 `apply` 全部可复用**，且天然支持局部假设（`spec.binders` 就是当前
上下文）。代价是 lowering 会触发一次内核推断——这是**刻意的偏离**，写在这里以便
未来不误判为违规：

- `intro` 的「值不进内核」是为了保证「降低只做结构展开、不做判定」；
- `apply` 的降低**同样不做判定**（不检查候选答案对不对），它只是**问内核要一个
  类型**——和 `by` 每个 tactic 都在做的事完全同源；
- 最终判定仍然由内核在「填洞后的合成声明」上出（`judge.rs`），一以贯之。

## 2. 语法

### 2.1 挂点与实参

- 挂点：`parse_value`（`crates/front/src/parser.rs:154-165`），与 `by`/`intro` 同级，
  新增第三个分支；
- 实参语法：用 **`parse_app`**（`parser.rs:538-550`）而不是 `parse_expr`。
  理由：`parse_app` 只吃应用 spine（`starts_atom`，`parser.rs:552-559`），遇到命令
  关键字会停（`is_reserved_command`，`parser.rs:943-958`），跨命令安全；用
  `parse_expr` 会让 `apply h -> x` 被吃成箭头类型这种边界。
- **必须消费实参**：`intro` 消费完就返回（`parser.rs:159-162`），所以 `:= intro a`
  在 `parse_command` 处报「尾随 token」（`parser.rs:994-995`）。`apply` 有实参，
  若照抄就会把实参当成尾随 token 报错。

### 2.2 括号：实参范围靠括号界定

`(f a b)` 这种分组**已支持**，无需新语法：`parse_atom` 处理 `LParen`
（`parser.rs:578-582`），箭头位的 `named_group_ahead`（`parser.rs:472-488`）只截获
带冒号的 binder 组，`(f a b)` 不会被误判。

于是「必要的时候用括号确定范围」落到两条可用写法上：

```lean
theorem t (h : Q -> P) : P := apply h          -- 应用 spine：apply (h)
theorem t (f : (a : Prop) -> Q a -> P a) : P :=\n  apply (f a)   -- 括号界定实参范围
theorem t : P := apply (fun (q : Q) => hfun q) -- 复合表达式必须加括号
```

### 2.3 语义：把目标「倒过来」消费

给定目标 `G` 与实参项 `e`（可能是 spine `f a1 a2`）：

1. 推断 `e` 的类型 `T`（§1 的 1-2 步）；
2. 剥 `T` 的 Pi 链得 `layers` 与 `codomain`；
3. 把 `codomain` 与目标 `G` 位置合一：`codomain` 里出现过的命名 binder = 类型参数
   （用目标实参填充），其余 = **前提**；
4. 实参项 `e` 里**已经给出的实参**按位置从 `layers` 前部消耗（这就是「部分表达式」
   的来源：`apply h a1` 只给第一个前提）；
5. 剩下的前提逐个生成 `Expr::Hole`；
6. 拼 `Expr::App`，洞的 span 全部取 `apply` token（与 `intro` 一致）。

展开形态示例：

| 输入 | 目标 | 手上的东西 | 展开 |
|---|---|---|---|
| `apply h` | `P` | `h : Q -> P` | `h sorry` |
| `apply h` | `P` | `h : P` | `h`（零洞 → 直接成为 Checked 的完整证明） |
| `apply f a` | `P a` | `f : (a : Prop) -> Q a -> P a` | `f a sorry` |
| `apply And.intro` | `And a b` | 构造子 | `And.intro a b sorry sorry`（与既有 refine 模板同形） |

零洞的情形**不是特例**：它就是 `exact`，判定交给内核，通过即 Checked。

### 2.4 非目标（v1）

- **不做**嵌套位置的关键字（`fun (a : Prop) => apply h`）：`intro` 的值位关键字只在
  `parse_value` 识别（本文件 §2.1 同一挂点），lambda 体内的 `apply` 是普通标识符。
  与 `term-intro.md` §9 保持一致。
- **不做**`by` 块内的改动：`by apply h` 走 `parse_tactic`（`parser.rs:228-236`）与
  `apply_tactic`（`by.rs:275-368`），是**另一条路径**，本设计不碰。两条路径并存是
  刻意的（作用域不同：值位=整个值，tactic=当前目标），课程里必须对照讲清。
- **不做** `apply` 到构造子以外的「高阶合一」：合一强度与 `by` 的
  `unify_spine` 完全一致（只要求头相同、实参个数相等），不引入更强的高阶匹配。
- **不做**省略实参的 `apply`（`:= apply`）：报教学错误（§4）。

## 3. 降低模块

新增 `crates/front/src/compile/apply.rs`，签名与 intro 对称：

```rust
pub(crate) fn lower_apply_val(
    ty: &Expr,            // 声明类型（目标）
    val: &Expr,           // 值位 AST（含 Expr::Apply）
    ctx: &ApplyCtx<'_>,   // 前缀源码 + options + 当前声明 binder（供 judge_infer）
) -> Result<Option<(Expr, String)>, CompileError>;
```

- 入口与 `lower_intro_val` 并列，在 `check.rs` 的 `lower_value`（`check.rs:142-154`）
  里加第三个分支；
- 复用的既有函数：`judge_infer`（`judge.rs:161`）、`proof::parse_expr_text`
  （`proof.rs:45`）、`peel_pi`（`by.rs:457`）、`unify_spine`（`by.rs:492`）、
  `substitute`（`by.rs:572`）；
- 骨架文本（`render_expr`）与 `intro_skeleton` 同款，落到新的
  `DeclState.apply_skeleton: Option<String>`（`report.rs:106-108` 旁）。
  **单一事实源**：编辑器不重算。
- 模块化纪律：`apply.rs` 预计 150–200 行；`by.rs`（约 640 行）与
  `compile/check.rs` 已接近/超过红线，**不往里塞新代码**，共享机械若需提升可见性，
  抽到 `compile/spine.rs`（新增，只做 Pi/spine/替换三件事）。

## 4. 错误码（三件套：`ErrorKind` + protocol + hint）

| code | stage | 触发 | hint 要点 |
|---|---|---|---|
| `elab-apply-needs-a-term` | elab | `:= apply` 无实参 | 「`apply` 后面要跟一个证明或函数，例如 `apply h`」 |
| `elab-apply-not-applicable` | elab | 实参的 codomain 与目标头不一致 / 剥不出 Pi 且类型不等于目标 | 「`apply h` 要求 `h` 的结论正好是这个目标；先看 `h : … -> P` 的最后一段是不是当前目标」 |
| `elab-apply-unresolved-name` | elab | 实参里的名字推不出类型（未知标识符） | 复用既有 unknown identifier 措辞，不新造 |

三个 code 都要进 `docs/protocol.md` 的错误码清单（`protocol.md:96-97` 旁），并由
`protocol_doc_lists_every_error_code`（`compile/tests.rs`）守护。

## 5. 「不展开也等价」＝契约（与 `intro` 同款）

`intro` 已把这条钉成测试（`intro_is_equivalent_to_typing_the_skeleton_out_by_hand`、
`value_intro_is_layout_independent`）。`apply` 必须同样钉住：

- `apply_is_equivalent_to_typing_the_skeleton_out_by_hand`：`apply h` vs 手写
  `h sorry`，必须同 `status`/`goal`/`binders`/洞数；
- `value_apply_is_layout_independent`：同页 / 换行 / 尾随空格三种排版判定一致。

## 6. 合成洞必须与源码 `sorry` 区分开（**已知硬风险**）

`judge_hole_fill` 有一条字面守卫：洞位源码切片必须恰为 `"sorry"`
（`crates/front/src/judge.rs:316-322`）。`intro` 之所以绕过它，是因为它的
`sub_goals` 为空，`suggest.rs:135-145` 把「无子目标」路由到 `judge_terms`
（按折叠声明判定，不碰源码文本）。

`apply` 的展开形态是 `h sorry` —— **App（spine）**，`open_goal` 会走
`func_spine_case`/`ctor_spine_case`（`goals.rs:524-576`）产出**非空 `sub_goals`**。
于是：

- 恰好 1 个洞时 → `suggest.rs:138-140` 走 `judge_hole_fill` → 守卫因
  `doc_src[hole] == "apply"` 判为 parse 错误 → **该洞的 exact/rfl 补全会坏**；
- ≥2 个洞时 → `suggest.rs:141-144` 走 `judge_terms`，反而没事。

修法（v1 采用 (a)）：

- **(a) 给合成洞打标**：`DeclState` 增一个「洞是合成的」判据（例如
  `synthetic_holes: bool`，或直接看 `intro_skeleton.is_some() ||
  apply_skeleton.is_some()`），`suggest.rs` 据此把该声明的所有洞路由到
  `judge_terms`。语义正确性：合成洞在源码里没有可替换的位置，**本来就不该**走
  「替换源码切片」的判定路径。
- (b) 放宽守卫（接受关键字 token 作为洞位）——**不采用**：`judge_hole_fill` 会把
  候选答案写进那个 span 再重编译，洞位在 `apply` token 上会把源码改成
  `:= 答案 h` 这种垃圾。
- (c) 把合成洞的 span 设成零宽插在关键字后——**不采用**：`nextHole`/inlay 需要
  真实位置，零宽洞会退化成不可导航。

> 这条风险对**展开之后**的文档不存在：学习者一旦接受补全，源码里的洞就是真的
> `sorry`，全链路行为与手写完全一致。

## 7. 编辑器面（与 `intro` 同构）

| 能力 | 处理 | 依据 |
|---|---|---|
| 补全 | 门控命中时给一项 `apply …（展开为 …）`，`textEdit` 覆盖整个 `apply <term>` 区间，`newText` = 骨架 | 对标 `lsp/lib.rs:1073-1095`（intro 项构造）、`1113-1116`（裸关键字去重） |
| hover | 展开式 + 「不替换也完全等价」+ `command:sokonanoda.expandApply?{uri,range,newText}` | 对标 `lib.rs:700-725`；编码复用 `percent_encode_component`（`lib.rs:673-691`） |
| VS Code 命令 | `sokonanoda.expandApply`：注册 + `contributes.commands` + 命令面板隐藏 + `markdown.isTrusted.enabledCommands` 放行 | `extension.js:427-454/633/707`、`package.json:101-105/139-142`；契约测试 `cli/tests/extension.rs:38-67` 强制「声明↔注册」一致 |
| 语义高亮 | `KEYWORDS` 加 `apply`（`semantic.rs:42-57`）。**副作用**：`by apply h` 里的 `apply` 会从 VARIABLE 变 Keyword（与 `intro` 对称，认可） | `semantic.rs:315-317` |
| inlay / goals / nextHole / stateAt / rename / references / documentSymbol / selectionRange | **自动生效**，无需改动 | 全部由 `DeclState`/`hovers` 驱动（见调研 §3 表） |
| session 增量 | 复用 `remap_prefix`（`session.rs:271-319`）——只要洞并入 `holes`/`sub_goals` 就自动被 remap | `session.rs:297-302`；新增独立带 span 字段才需补 remap |

命中区间（hover/补全）复用 `intro_hit`/`intro_at` 的规则，**抽成按骨架字段取的通用
helper**（`keyword_at(report, text, offset, which)`），避免出现第三套命中规则
（现在已有 `intro_at`、`state_at`、`render::decl_at` 三套选取逻辑，见风险矩阵）。

## 8. 关键证据索引

| 结论 | 证据 |
|---|---|
| `intro` 降低纯 front 侧 | `compile/intro.rs:16-109`；`check.rs:142-154` |
| `apply` 需要名字类型 | `by.rs:288-298`（`judge_infer`）、`judge.rs:161-225` |
| front 侧类型表不完整 | `goals.rs:20-25`（无 codomain）、`goals.rs:249-267`（`peel_type` 丢 codomain）、`goals.rs:141-145` |
| 局部假设不在表内 | `goals.rs:531-534`（`func_spine_case` 只查全局表）；`report.rs:40-44`（局部类型只有 String） |
| 内核侧无类型查询 API | `kernel/builder.rs:234`（只有 `declaration_count`） |
| 括号已支持 | `parser.rs:578-582`、`parser.rs:472-488` |
| 尾随 token 现状 | `parser.rs:159-162`、`parser.rs:994-995` |
| `judge_hole_fill` 守卫 | `judge.rs:316-322`；绕过路径 `suggest.rs:135-145` |

## 9. 分阶段实现计划

见 `ROADMAP.md` §10「I10 —— 值位 `apply` / I11 —— 真人输入测试 / I12 —— 官网」。
每阶段的验收标准与 subagent 任务书要求写在那里，本文只给技术方案。

## 10. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| `judge_infer` 的成本被放大 | 每个 `:= apply h` 多一次合成编译 | 只在「值位是 `apply` 且目标未闭合」时触发；与 `by` 每次 tactic 同量级，可接受；实现轮用 `Session::stats.kernel_checks` 量化并写进文档 |
| 合成洞被误当源码洞 | exact/rfl 补全坏（§6） | 打标 + 路由到 `judge_terms`；回归测试钉死 |
| `apply` 进 KEYWORDS 影响 `by` 块高亮 | semantic golden 变化 | 视为正确变化，同步 golden 与测试 |
| 两条 `apply` 路径（值位 / tactic）语义混淆 | 学习者困惑、课程讲述负担 | 课程单元里必须对照（`intro` 已有「值位 vs tactic」对照节，`term-intro.md` §8）；文档明确二者作用域 |
| 与 `intro` 的骨架字段分叉 | 编辑器两套命中规则 | 抽通用 `keyword_*` helper，不许出现第三套 |
