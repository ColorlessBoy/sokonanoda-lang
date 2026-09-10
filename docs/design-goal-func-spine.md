# 设计：函数实参洞（function-spine holes）+ hover 开项 pp 修复（2026-09-10，第二十四轮）

> 触发：用户以 `playground.sokonanoda:233`（`Eq.subst.{1} Nat (fun (x : Nat) => …) a b h …`）
> 为例提两个需求——(1) hover `Eq.subst.{1}` 应显示其类型，现在只显示源码切片；
> (2) 该行的谓词实参改写成 `(sorry)` 后应成为合法练习，VSCode 提示该洞需要的类型
> （`Nat -> Prop`）。经评估两者都属「简单档」：前者是内核 pp 的崩溃兜底把类型
> 吞掉，后者复用既有 AST 模板机制即可，均不需要 metavariable / 内核语义改动。

## 1. 现状与根因

### 1.1 hover `Eq.subst.{1}` 无类型（真 bug）

- 类型推断本身成功；崩溃发生在**内核 pretty-printer**：`is_implicit_fun`
  （`crates/kernel/src/pretty_printer.rs:613`）为判断「是否省略隐式参数」，
  开一个**空 TC/ctx** 对子项做 `infer_whnf_weak`。打印 `Eq.subst`/`Eq.refl`
  的类型时，类型体里存在开项（`p a`：Var 头应用；`Eq α a`：依赖实参
  展开需 `eval` 松散变量），空 ctx 直接 panic（`infer.rs:88`
  `loose bvar in infer` / `eval.rs:576` `eval: loose bvar`）。
- front 的 `resolve_hovers`（`check.rs:1750-1763`）用 catch_unwind 兜底，
  panic 后把 `text` 置空；LSP 见空 text 只渲染源码切片（`render.rs:187`），
  于是 hover = `Eq.subst.{1}`（无 `: 类型`）。
- 同一 bug 使 CLI `#check (Eq.subst.{1})` / `#check (Eq.refl.{1})` 报假
  `kernel-rejected`（`kernel_check` 路径同样 panic）——不只是显示问题。
- 复现锚点：`Eq.{1}`（类型体无应用）正常，`Eq.subst.{1}`/`Eq.refl.{1}`
  失败；`And.left` 等不依赖实参的应用正常（`infer_app_v` 的
  `InferOnly` 分支仅在 Pi 体依赖实参时才 `arg_value`）。

### 1.2 `sorry` 不能当函数实参

- 当前合法洞位（walk 结果，`check.rs:561-633`）：①整个值；②lambda 链尾部；
  ③**构造子 spine** 的直接实参（`ctor_spine_case`，要求「目标头有模板」且
  「值头 == 模板名」，模板仅来自源内 axiom/inductive，`check.rs:259-310`、
  `497-499`）。
- `Eq.subst` 是 prelude 受信任安装（`prelude.rs:70-106`，源文本
  `PRELUDE_EQ_SRC`），不产生 `Command::Axiom`，进不了模板表；即使有模板，
  `ctor_spine_case` 的「值头==构造子」语义也不适用于任意函数（目标是
  `Eq b a`，与 `Eq.subst` 的结果头无关）。于是 `Eq.subst.{1} Nat (sorry) …`
  落回 elab 报 `elab-hole-misplaced`。
- 反例对照：源内 `axiom And.intro …` 的 `And.intro a b sorry sorry` 已合法，
  LSP inlay 已会显示 `: a` / `: b`——**展示管道现成**，缺的只是模板来源与
  匹配形状。

## 2. 决议

### D1 内核 pp：`is_implicit_fun` 对开项短路（显示层 bugfix）

`is_implicit_fun` 开头加：

```rust
if fun.num_loose_bvars() > 0 { return false; }
```

- 开项无法在空 context 下推断隐式参数风格；`false`（按显式打印）是唯一
  不撒谎的选择。闭项行为逐字节不变。
- 纯显示层：不触碰 `infer`/`conv`/`eval` 热路径；`num_loose_bvars` 由
  `ExprPtr` tag 直接读出（O(1)，无遍历）。
- 按内核冻结规则（REQUIREMENTS §2 第 1 条、architecture §6）带**三层回归**：
  kernel（`memory_api.rs`：构造依赖应用类型 → infer + pp 不 panic）；
  front（hover 文本非空且含 `p a`）；CLI（`#check (Eq.subst.{1})` 出
  `expr.typed` 而非 `kernel-rejected`）；另加 LSP hover e2e（用户原始场景）。

### D2 front：模板 machinery 抽到 `compile/goals.rs`

- `check.rs` 已 1816 行（远超 ~500 行红线，REQUIREMENTS §4）。把模板与
  walk 整体迁出为 `crates/front/src/compile/goals.rs`：`GoalTemplates`、
  `OpenGoalInfo`、`open_goal`、`expr_has_hole`、substitute 家族、渲染辅助。
  check.rs 只留调用点（行为不变，纯搬迁 + re-import）。
- 新代码全部落在 goals.rs，check.rs 不再膨胀。

### D3 front：函数实参洞（v1 只支持直接实参）

- 模板表扩展为双索引：
  - `ctors: HashMap<族头, CtorTemplate>`（现有语义，目标头→构造子）；
  - `funcs: HashMap<函数名, FuncTemplate>`（`binder_names`/`binder_tys` 即
    函数望远镜）。
- `funcs` 的来源（按权威顺序，同名后者覆盖前者）：
  1. 源内 `axiom`（全部，含结果不是族应用的 `False.rec` 类）；
  2. 源内 `def` / `theorem` / `inductive` 构造子（ctor 同时进两表：
     ctor 表管「目标头对齐」，func 表兜「值头即该名字」的多构造子族，
     如 `Or.inr a b (sorry)`——现有 ctor 表每族只留第一个构造子，
     这是顺带修的真实缺口）；
  3. `PreludeMode::Full` 且文件未占用 Eq 三名时，解析 `PRELUDE_EQ_SRC`
     得 `Eq` / `Eq.refl` / `Eq.subst`（Bare 模式不装，文件自定义的
     `axiom Eq.subst` 走来源 1，两条路等价）。
- walk 的 `_` 分支改为 `ctor_spine_case(...).or_else(|| func_spine_case(...))`：
  ctor 语义优先（参数位可从目标自动判定，信息更多），函数兜底。
- `func_spine_case`：值 `f v1 … vn`，查 `funcs[ f ]`；第 i 个实参是洞时，
  期望类型 = `binder_tys[i]` 用 `{binder_names[0..i] → val_args[0..i]}` 深度
  AST 替换（复用 `substitute_names`，遮蔽守卫已有）后渲染。
  - 前置实参本身是洞 → 替换结果含 `sorry` → 该子洞 `ty = None`（面板显示
    `?`，与现有「映射不完整」一致）；不影响其它洞。
  - 隐式 binder 按位置对齐（教学语法不做隐式补全，位置显式给全，已在
    playground 单元②固化）。
- `refine_template` 保持 `None`（函数缺省值不生成 refine 骨架，v1）。
- 宽松度取舍：任何「已知函数的直接实参洞」都会让声明成为 Open（函数结果
  头无法用 AST 廉价对齐目标）。形状写错推迟到填洞后的 kernel 终审——与
  整值 `sorry` 的既有宽松一致；这是「建议生成 / 判定分离」的代价，记录在案。

### D4 明确不做（v1）

- 嵌套洞（`f (g sorry)`、`fun (x : Nat) => sorry` 在实参里）——需要
  bidirectional/expected 穿透，留下一阶段；
- 部分应用自动补参（`Eq.subst.{1} Nat (sorry)` 后续全缺）——只支持
  已写实参中的洞；
- `sorry + 1`（`Expr::Plus`）——需要 Nat.add 模板的语法糖通道，后议；
- 子洞 kernel 级 expected type（spine meta，设计文档既定远期项，不变）。

## 3. 验收（全部落成测试）

kernel（`crates/kernel/tests/memory_api.rs`）：
- `pp_of_dependent_applications_with_loose_bvars_does_not_panic`：手工搭
  `axiom C : (A : Sort 1) -> (p : A -> Prop) -> (a : A) -> p a` 之类类型，
  `infer_closed_type` + `pp_expr` 成功且文本含 `p a`（修复前 panic）。

front（`compile/tests.rs`）：
- hover：`Eq.subst.{1}` 的 hover 行 text 非空且含 `p a`（同一 bug 的
  front 层护栏）；`Eq.refl.{1}` 同理；
- `Eq.subst.{1} Nat (sorry) a b h (Eq.refl.{1} Nat a)` → Open，洞期望
  `Nat -> Prop`；
- 后续洞：`Eq.subst.{1} Nat p a b h (sorry)` → 期望 `p a`；前置洞时
  `ty = None`；
- `Eq.refl.{1} Nat (sorry)` → 期望 `Nat`；
- 源内多构造子族：`Or.inr a b (sorry)` → 期望 `b`（ctor 表未覆盖的缺口）；
- 回归：`And.intro a b sorry sorry` 仍走 ctor 语义（期望 `a`/`b`）；
- 回归：嵌套洞 `Eq.subst.{1} Nat (fun (x : Nat) => sorry) a b h …` 仍
  `elab-hole-misplaced`（D4 边界显式化）。

CLI（`crates/cli/tests/cli.rs`）：
- `#check (Eq.subst.{1})` → `expr.typed` 且无 diagnostic；
- 函数实参洞文件 → `exercise.open` 且无 diagnostic。

LSP（`crates/lsp/src/lib.rs` / `inlay.rs` tests）：
- hover `Eq.subst.{1}` 文本含 `: forall`（用户原始症状）；
- inlay：函数实参洞显示 `: Nat -> Prop`。

## 4. 文件分工

主会话单人实施（改动集中且互斥）：
`crates/kernel/src/pretty_printer.rs`、`crates/kernel/tests/memory_api.rs`、
`crates/front/src/compile/{goals.rs(new),mod.rs,check.rs,prelude.rs(只读引用),tests.rs}`、
`crates/cli/tests/cli.rs`、`crates/lsp/src/{lib.rs,inlay.rs}`、本设计文档、
`docs/STATUS.md`、`docs/REQUIREMENTS.md` §9。
