# 设计：多洞（multi-hole）与 refine —— I9 goal 深化第二段（2026-09-07）

> 依据：goal 视图 UX 调研（docs/design-i8-i9.md §2 引用）、Lean refine/TryThis
> 范式、ocaml-lsp typed-holes。目标：把"洞 = 剩余目标占位"从**单一尾洞**
> 推广到**构造子 spine 上的多洞**，并提供 kernel 知情的 `refine` 建议。

## 1. 动机

`intro` 已经能"把洞换成 lambda + 尾洞"（refine 的 Pi 形态）。另一半：
目标头部是构造子时（`And a b`、`Or a b`），Lean 用户期望 `refine And.intro`
——把洞换成 `And.intro ??? ???`，编辑器里出现多个子目标。当前语法把
非尾洞判为 `elab-hole-misplaced`，refine 无从落地。

## 2. 方案（v1：AST 层实例化，kernel 终审不变）

### 2.1 walk 扩展（front/compile/check.rs `goal_under_binders`）

在现有 lambda 剥层之后，支持**构造子 spine 对齐**：

- value 侧：`App` 链，头是 `Ident(ctor)`，参数 0..n（任意位置可为 Hole）；
- 类型侧：同头 `Ident(ctor)` 的 `App` 链（头构造子按名字对齐）；
- 对每个 Hole 参数：产生一个**子目标**（span = 洞 span）；
- lambda 与 spine 可混合（先剥 lambda，再对 spine）；
- 类型侧头与 value 侧头不一致 / 不是 App 链 → 回退现状（None → 照旧报
  `elab-hole-misplaced`）。

### 2.2 子目标的期望类型（AST 实例化）

子目标期望类型 = 构造子字段类型在"结果头参数"处的实例化，来源是同一
教学骨架的**兄弟 axiom**（命名约定 `{C}.intro`，与官方 Lean 同构）：

- 在文件**前缀命令**里找 `axiom {C}.intro : <类型>`；
- 其类型 AST 剥箭头到结果头 `C …`：字段类型列表 + 结果头参数列表；
- 实例化映射 `{兄弟 binder 名 → 渲染(goal 头对应参数)}`（按位置对齐）；
- 映射不完整时尽力渲染（建议 ≠ 判定：kernel 永远终审）；
- 找不到兄弟 axiom → 子目标类型为 `None`（面板显示 `?`）。

原则声明：这是**建议生成**（模板），不是判定；判定仍由完整 kernel 走。
与 Lean `exact?` 生成后验证的差别在文档里注明（v1 不做模板验证）。

### 2.3 数据模型

- `DeclState` 增：
  - `holes: Vec<Span>` —— 全部洞位（含子洞），按 offset 排序（server 端
    从 walk 产出；nextHole / 面板消费）；
  - `sub_goals: Vec<SubGoal { span: Span, ty: Option<String> }>` —— 子洞
    期望类型（可缺省）。
- `open_goal` 返回值扩展；`OpenExercise` op / PendingOp / DeclState 贯通。
- 主 `goal` 字段保持"整份剩余目标"语义不变。

### 2.4 LSP / 面板

- code action 新增 `refine <Ctor>（生成 N 个子目标）`：把**主洞**替换为
  `Ctor ??? … ???`（仅当 walk 判定 goal 头是 ctor 且兄弟存在）；
- `soko/goals`：每声明新增 `holes`（范围数组）与 `sub_goals`；面板对
  open 练习列出子洞子项（`洞 i` + 期望类型）；
- `soko/nextHole`：遍历 `holes`（跨声明、跨子洞），环绕语义不变；
- hover：主洞显示主目标（现状）；子洞 hover v1 不进 hover 表（无内核
  上下文），面板承担展示——记录为后续工作（需要 kernel spine meta）。

### 2.5 明确不做（v1）

- 子洞的 kernel 级 expected type（需要 elaborator spine meta，下一阶段）；
- 洞在 lambda binder 类型里 / 非尾 lambda 体里的多洞；
- refine 后的模板 kernel 预验证（Lean #7192 形态）。

## 3. 测试计划

- front：多洞 walk（Open 状态而非 hole-misplaced）、子目标文本实例化、
  混合实参（`And.intro True ???`）、非 ctor 回退、单洞回归全绿；
- LSP：refine action 出现/不出现、`soko/goals` 携带 holes/sub_goals、
  nextHole 跨子洞环绕；
- 契约：skill/protocol 文档同步（soko/goals 形状）。

## 4. 并行 subagent 分工（本轮）

- A：内核拒绝分类学审计（panic 站点清单 → 稳定 code/提示设计提案）；
- B：发布流水线（release.yml + VSIX artifact）实现；
- C：业内标准差距对标（补全功能清单 → ROADMAP 输入）。
