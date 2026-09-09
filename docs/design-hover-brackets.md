# 括号 hover 与真名还原（第十七轮设计）

> 日期：2026-09-09。触发：用户反馈「VS Code 插件的 hover 内容完全混乱」，
> 要求 `(表达式)` 的括号 hover 显示 `表达式 : 类型`，且不得出现 `$2` 这类
> de Bruijn 索引。指定测试语料：`and_not_absurd`。

## 1. 根因（三个叠加，全部实验证实）

### 1.1 `$N` 索引泄漏（`name_loose_bvars` 的映射假设错误）

`infer_under_binders` 返回**相对 de Bruijn 开项**（debug 打印证实：
`Pi ( : ((And.[] $1) (Not.[] $1))), (Not.[] $2)`，Var(d+j)，j = 松散序号）。
pp 打印时有两处系统性偏移，导致「文本里 `$N` → `scope_names[len-1-N]`」
的映射永不成立：

1. **pp 的 `binder_names` 从空开始**：只随被打印项自身的 Pi/Lam 下沉增长，
   scope 层的 binder 不在表里 → 松散引用必然走 `$dbj_idx` 分支；
2. **`parse_binders` 对 telescope 域做 lift**（`lift(ty, 0, n-i)`）：
   域在 binder 上下文之外打印，索引被抬高 → 同一变量在不同位置打印出
   `$2`/`$3`/`$4` 等不同数字（`And.right a` 的域打印 `$3`，实际是 j=1）。

结论：**任何文本后处理都无法可靠还原**——必须让 pp 自己认识 scope 名字。

### 1.2 `Not a` 被强展开

`infer_under_binders` 里的 `force_all` 把定义头展开：hover 显示
`And.right a (Not a) h : a -> False` 而不是 `: Not a`。值本身是
Unfold 头（实验：去 force_all 后 quote 出 `Not $1`），quote 的每个节点
自带 force_thunk，Unfold 头可以安全保持折叠。

### 1.3 括号位置命中错误邻居（双重回退打架）

- `hover_type_at` 的「起点 ±2」回退：`)` 上命中**右侧**起点的邻居
  （`…h) (And.left…` → 显示 `And.left : forall …`）；
- `hover()` 里另有 TOLERANCE=2 的重叠回退，两套语义不一致；
- 外层 lambda 行 span 覆盖整个值表达式，若先走精确命中，括号组永远被
  lambda 行遮住。

## 2. 决议

### D1 pp 播种（kernel，冷路径显示层，§6 记档）

- `PrettyPrinter::seed_binder_names(&[String])`：把 scope 名字（外层在前）
  预置进 `binder_names`。代数验证（含 lift）：松散 j 在 pp 下沉 d 层后
  索引 d+j，`checked_sub(1+idx)` 命中 `names[S-1-j]`——**所有位置精确**：
  - 体位置：`S+k-1-(d+j) = S-1-j`（k = 已推入的 pp binder 数）；
  - 域位置（lift n-i）：`S+d+n-1-(d+j+n) = S-1-j`，与 i 无关。
- `TypeChecker::with_pp_scoped(scope, f)`：with_pp + 播种的便捷入口。
- 播种后开项打印**零 `$N`**；闭合项不受影响；shadowing 按位置自然正确。

### D2 `infer_under_binders` 去掉 `force_all`（kernel，§6 记档）

`Not a` 保持折叠显示（与 `#check` 展示习惯一致）。唯一消费者是
`resolve_hovers`（hover），纯显示层改动；quote 自行 force thunk，
无新增不终止风险。

### D3 括号组匹配（LSP 层）

- `render::bracket_hover_at(text, hovers, line, char)`：光标字符是
  `(`/`)` 时按文本扫描配对（跳过 `--` 行注释；教学语法无块注释/字符串），
  取**完全落在括号组内部的最大 hover 行** = 括号包住的表达式
  （AST span 不含括号，内层应用链恰好是组内最大行）。
- 反向扫描修正：扫到光标**之前**为止（光标 `)` 的配对 = 栈顶）；
  注释区域跨过光标 → 显式 None。
- `hover()` 优先级改为：**关键字静默 → 括号 → 精确命中 → ±2 邻近 → 声明**。
  删除 `hover_type_at` 的「起点 ±2」回退（goto-def/highlight/completion
  同步受益：`)` 不再跳到邻居的定义）。
- `hover_content` 统一格式：text 为空 **或含 `$`** → 只显示源码切片
  （绝不让索引值抵达 VS Code）；否则 `表达式 : 类型`。

### D4 防御层保留

`name_loose_bvars` 保留为兜底（正常情况播种后无 `$`，不再触发）。

## 3. 验收（全部落地为测试）

- front `hover_rows_of_and_not_absurd_show_real_names`：用户样例 1
  `And.right a (Not a) h : Not a`、样例 2 `And.left a (Not a) h : a`、
  `h : And a (Not a)`、部分应用 `And a (Not a) -> Not a`、整链 `False`、
  lambda `forall (a : Prop), And a (Not a) -> False`、**全语料零 `$`**；
- front `hover_rows_of_partial_applications_use_scope_names`：and_swap 的
  `And.intro b a : b -> a -> And b a`、`And.right a b : And a b -> b`；
- LSP 五个括号测试：两组括号正反面、`)` 不漏邻居签名（回归）、
  `((p))` 四括号透明、注释内括号不张冠李戴；
- 旧测试对齐：`(a : Prop)` 的 `(` 显示组内最大行（`Prop : Type 0`）。

## 4. 边界与不修项

- `(a : Prop)` binder 标注组没有单一表达式行，显示组内最大行（可接受）；
- 两组应用之间空白的 hover 显示外层应用行，切片止于最后原子（span 语义
  如实； bracket 组本身已覆盖学习者的主要路径）；
- kernel 冻结纪律：两处改动均为冷路径显示层，热循环零改动，
  architecture.md §6 增行记档。
