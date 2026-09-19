# 声明级 binder（Lean 风格 `theorem f (a : A) : B := v`）设计（2026-09-12 设计 + 同日实现）

> as-built（2026-09-12 实现轮）：parser 侧 `wrap_decl_binders` 降级为
> 「Forall 类型 + Lambda 值」（`crates/front/src/parser.rs`）；`by` 引擎
> `run_by` 新增 `initial_binders` 参数，声明 binder 进根节点上下文、类型先
> 剥对应层数；`intro` 降低沿 lambda 链递归（`compile/intro.rs`，共用
> `proof::peel_pi_layers`）；课程落 unit1「两种拼写」+ 练习 6。验收：
> front 265 / lsp 98 / cli 51 / course 4 全绿，golden unit1 (13,6,1)。

> 触发（用户原话）：「接下来我希望支持 lean4 里这样的语法，省去 intro 的
> 麻烦事」：
>
> ```lean
> theorem and_swap2 (a : Prop) (b : Prop) (h : And a b) : And b a := sorry
> ```
>
> 用户拍板：0.14.0（值位 `intro`）先发布；本设计随下一轮实现（0.15.0）。
> 三件套（语法白名单 + 课程 + 测试）随实现落地。

## 1. 目标体验

1. **声明 binder 直接进上下文**：`:= sorry` 时剩余目标就是 codomain
   （`And b a`），上下文已含 `a b h`——不用写 `fun`、不用 `intro`。
2. **正文直接写**：闭合时 `:= And.intro b a (And.right a b h) (And.left a b h)`
   合法，编译器自动把这些 binder 包成 lambda——与官方 Lean 一致。
3. **与箭头写法完全等价**：现有的
   `theorem foo : (a : Prop) -> ... := fun ... => ...` 一行不改；
   两种写法可混用（声明 binder 只是「箭头 + 自动 fun」的糖）。

## 2. 语法（白名单）

```text
def     name {u, v} (binders) : T := v
theorem name {u, v} (binders) : T := v
axiom   name {u, v} (binders) : T          -- 无值位：binder 只折类型（G-13）
example            (binders) : T := v
```

- `binders` = 零个或多个 binder 组，复用 `forall` 的解析：
  `(x : T)`、`(x y : T)`（同型多名字）、`{x : T}`（隐式）、多组连排；
- 与**宇宙参数**消歧：`{u}` / `{u, v}` / `{u v}` / `{u} {v}`（只有名字，
  逗号或空格分隔，可连排多组）是宇宙参数；`{x : T}`（名字后有冒号）是隐式
  binder。实现：花括号 lookahead（ident 列表后接 `}` → 宇宙组；接 `:` →
  binder 组），调用点顺序必须**宇宙参数在前**；
- 声明 binder 必须带显式类型（与 `forall` 一致）；`(a)` 报教学 parse 错误；
- binder 组之间共享类型的依赖顺序保持书写顺序
  （`(a : Prop) (h : a) : ...` 合法且 `h` 的类型能引用 `a`）。

## 3. 语义（与官方 Lean 对齐）

- **类型侧**：声明 binder 望远镜拼在 codomain 前面：
  `(a : Prop) -> (b : Prop) -> And a b -> And b a`（显式/隐式风格保留）。
- **值侧**：值在 binder 已引入的上下文中书写；声明最终的值是
  `fun (a : Prop) => fun (b : Prop) => fun (h : And a b) => v`。

### 3.1 v1 实现：parser 侧 desugar（零新流水线）

`parse_def` / `parse_theorem` / `parse_example` 解析出 `binders` 后：

```text
ty'  = Forall { binders, body: T }
val' = Lambda { binders, body: v }
```

于是既有机制全部免费复用：

- `:= sorry` → `fun binders => sorry`：`open_goal` 的 lambda walk 直接给出
  **goal = codomain、binders = 声明 binder**（正是要的体验），inlay/hover/
  `soko/goals`/补全无需改动；
- 闭合值 → `fun binders => v`：elaborate/kernel 与手写 `fun` 完全同路径；
- 空 binder（`theorem foo : T := v`）行为不变。

### 3.2 与 `intro` 的互动

声明 binder 被 desugar 成外层 lambda 后，值尾部的 `intro` 处于 lambda 体内；
现有 `lower_intro_val` 只认顶层 `Expr::Intro`。实现时把它推广为**沿 lambda
链下降**：每匹配一层「lambda ↔ Pi」就递归到体部；遇到 `intro` 时按剩余类型
全剥，返回降级后的值与「相对 intro 位置的骨架」（补全仍替换 intro token 为
剩余展开，如 `fun (x : a) => sorry`）。`:= by …` 不受影响：by 块里的
`intro` 只剥 codomain 剩下的 binder。

### 3.3 边界

- `axiom` 的**类型**位接受 binder 组（无值位，故不产生 Lambda；类型侧折叠由
  `wrap_type_binders` 与 `wrap_decl_binders` 共用一处，G-13 / WO-008）；
  `inductive` 参数早已支持，其 `{u}` 宇宙参数仍不支持（另一条缺口）；
- `example` 不支持宇宙参数（`Command::Example` 没有 `universe` 字段）：
  `example {u} …` 报明确文案「example 不支持宇宙参数」，不静默吞；
- 声明位 `def f.{u}` 是 **parse 错误**（名字不许以 `.` 结尾，G-18）：本子集不
  支持 canonical Lean 的 `.{u}` 拼写，宁可报错也不把它静默吃成 `f.`；
- `#check` / `#reduce` / REPL 不涉及；
- 非依赖与依赖 binder 都按书写顺序拼，不做隐式泛化（auto-bound）;
- 缺类型 `(a)` → parse 错误（沿用现有检查与文案）。
  **勘误**：本文件早先写的「重复 binder 名、binder 与宇宙参数重名 → parse 错误」
  与实测不符——`def f (a : Prop) (a : Prop) : Prop := a` 今天就是 checked（内核
  接受遮蔽），而跨组重名的**宇宙参数**（`def f {u} {u} …`）才报
  `duplicate universe parameter`。所以不要在实现里补出「重复 binder 名」检查。

## 4. 测试计划（三层）

**front**

- parser：explicit/implicit/多名字/多组/`{u}`+binder 混排/AST 等价
  （`theorem`、`def`、`example`）；缺类型与重复名的 parse 错误；
- compile：`and_swap2 := sorry` → Open、goal `And b a`、binders a/b/h、
  零内核；闭合正文（无 fun）被内核接受；`:= intro` 组合（剩余 codomain
  剥层 + 骨架文本）；与箭头写法产出的事件/判定一致（同一证明两种拼写）；
- 既有断言对齐（course golden 若计数变化）。

**axiom / 宇宙参数组（G-13 + G-14 + G-18，2026-09-18 落地）**

- parser：`axiom_with_decl_binders_parses`（1/2/3/4 号写法折成 Forall）、
  `axiom_decl_binders_match_arrow_style_ast`（binder 与箭头产出等价 AST，
  span 归一化后比结构）、`axiom_untyped_decl_binder_still_errors`、
  `universe_params_accept_space_separated_names`（`{u v}`）、
  `universe_params_accept_repeated_brace_groups`（`{u} {v}`）、
  `duplicate_universe_param_across_groups_is_rejected`（跨组去重）、
  `named_group_with_colon_is_still_a_binder`（有冒号仍是 binder）、
  `example_cannot_declare_universe_params`、`def_cannot_end_with_a_dot`（G-18）；
- compile：`checks_axiom_with_decl_binders`、
  `axiom_decl_binders_match_arrow_style_outcomes`（binder 与箭头各一条，
  再用两条闭合定理消费，四条都 Checked）、
  `checks_space_separated_and_split_universe_params`；
- CLI e2e：`cli_axiom_decl_binders_compile`、`cli_axiom_untyped_decl_binder_is_a_parse_error`、
  `cli_axiom_curried_forms_still_compile`、
  `cli_space_separated_and_split_universe_params_compile`、
  `cli_universe_param_name_cannot_end_with_a_dot`；
- 复现件：`docs/gaps/repro/G13-axiom-binder-params.sh`（修后 exit 1 = 行为已变）、
  `docs/gaps/repro/G14-single-universe-binder.sokonanoda`（修后 exit 0 / 3 条 checked）。

**注意（内核口径，写测试时的坑）**：`axiom` 的类型必须仍是 `Sort`。带 binder 的
`axiom Foo (α : Type) : Prop` 折成 `(α : Type) -> Prop` 是 **Pi 类型**，会被
kernel-rejected（与 binder 语法无关，箭头写法同款）；想让 binder 形式的公理
真的被判卷通过，codomain 要取 `Sort n`（例如 `: Sort 1`）。

**CLI e2e**：含声明 binder 的文档 `--json`：`decl.checked` /
`exercise.open` 正常；错误路径诊断码。

**LSP / 协议**：声明 binder 的 Open 练习在 inlay/hover/`soko/goals` 上与
手写 `fun` 版本一致（复用现有断言字段）；`intro` 补全在 binder 版本下
仍给「剩余」骨架。

**课程 / 契约**：unit1（或 unit3）新增「声明级 binder」一节 + 练习 + 钥匙
（中文 + en 镜像）；golden 计数更新；白名单文档（architecture §4.1）同步。

## 5. 课程与文档

- 课程切入点：unit1「命题与证明项」——同一道题给两种拼写（箭头 + `fun` vs
  声明 binder + 正文），点明「声明 binder = 把 `intro`/`fun` 的活交给声明
  行」；unit3 提依赖 binder（`(a : Prop) (h : a)`）；
- `docs/protocol.md`：若事件/错误码无变化则只补语法说明；
- `docs/architecture.md` §4.1：声明 binder 的 desugar 规则；
- `docs/README.md` 设计索引收录本文。

## 6. 非目标（v1）

- `[inst : C a]` instance binder、optional/default 值、`autoBound`；
- `variable` / section 机制、`where`、mutual；
- 隐式参数自动泛化、tactic 模式下的 `intros` 命名控制。

## 7. 验收

- 用户给的例子两行都能跑：`:= sorry` 进入 Open 且 goal/上下文正确；
  填正文闭合被内核接受；
- 箭头写法、`intro`、`by` 全量回归零失败；fmt/clippy/gate 全绿；
- 课程 zh/en/钥匙与 golden 通过；STATUS / REQUIREMENTS §9 / 文档同步。
