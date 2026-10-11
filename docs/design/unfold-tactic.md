# `unfold` tactic 与 `rfl` 的 whnf 逼近（2026-10-11 用户实测）

> **一句话**：新增 Lean 同名 tactic **`unfold Foo Bar`**（把目标里点名的常量展开成定义体），
> 并把同一份「源级多轮展开」机械接到 **`rfl`** 上 —— 对齐 Lean 4 的 `whnfR` 语义。
> **判定仍在内核**：展开结果先由内核验收（defeq），候选证明项照旧交内核终审。

## 1. 范围与契约

**用户要求**（2026-10-11 原话）：「`apply h` 到这一步的目标是 `⊢ a ∈ ({a})`，但是我想增加
一个 tactic，展开成 `⊢ a = a`，调研查查 lean4 有没有类似功能的 tactic」+「rfl 报错，这个
应该是 bug」。

| 项 | 契约 |
|---|---|
| 语法 | `unfold <名字>+`（**≥1 个**常量名，Lean 的 `ident+` 同口径；`Init/Tactics.lean:802`） |
| 语义 | 目标里**每一处**该常量都换成它的定义体；**逐处**（不只最外层），展开结果**继续**处理（`.visit`） |
| 判定 | 展开结果必须与展开前**定义相等**（内核 `judge_terms` 验收）⇒ 不等**不提交**、报错 |
| 报错 | 某个名字**一处都没展开** ⇒ 当场点名那个名字（Lean 的 `did not unfold 'X'` 同口径，**不静默**） |
| 边界 | **做** `at h`（点名假设；2026-10-11 同轮收口）；**不做** `at *` —— 本语言没有 `*` token（实测 `expected a valid .sokonanoda token, found *`），一次点名一个假设 |
| 课程用法 | `apply h` → `unfold Set.mem Set.singleton` → 目标变 `a = a` → `rfl` |

## 2. 机制（两处新机械，都只在前端）

1. **源级整树展开** `spine::unfold_named_in`（新）：
   * 自顶向下命中 `names` 里的头 ⇒ `unfold_one_with`（**自消 beta**、认记法头、认**内建糖头**
     `{a}` → `Set.singleton` / `{a, b}` → `Set.pair`），再对结果 `.visit`；
   * 没命中 ⇒ 递归子节点（`map_children`，与 `rename_free`/`beta_normalize` 同一条
     「哪些字段是子表达式」的纪律）；
   * `steps` 步数上限（防自引用定义把前端挂死）。
2. **糖 → 常量** 的唯一一份表 `ast::set_literal_target`（新）：`elab.rs` 的真展开与
   `spine.rs` 的源级展开**共用** —— 两处各写一张就会在"加第三种糖"时静默分叉。
3. **内核验收** `by.rs::goal_rewrite_is_defeq`（新）：把「展开后的目标」当成**一条假设的类型**
   塞进望远镜、把**原目标**当目标，判「那条假设本身」这个候选 ⇒ 类型对 ⇔ 两者 defeq。
   **不做文本比对**；`by` 块的乐观批次语义保证"乐观放过 ⇒ 严格重跑"（判定不变）。

## 3. Lean 4 对照（本机 `~/Documents/lean/lean4`）

| 机制 | Lean 4 | 本仓 |
|---|---|---|
| `unfold Foo` | `Meta.unfoldTarget`（`Lean/Meta/Tactic/Unfold.lean`）：非递归 `def` ⇒ `deltaExpand e (· == declName)`（`Meta/Tactic/Delta.lean:22`，`Core.transform` + `f.betaRev`） | `spine::unfold_named_in`（源级 delta + `beta_apply`） |
| 递归 `def` | 用生成的展开引理 `id.eq_def` 走 `simp`（只展开一层） | **n/a**：本语言没有递归 `def`（自引用报 `unknown identifier` ✓，判据防漂移） |
| `at h` 位置 | 支持（`withLocation`：`hyp*` / `*` / `⊢`） | **做** `at h1 h2`（点名假设，`by::find_binder` + `hyp_rewrite_is_defeq` 内核验收）；**不做** `at *`（没有 `*` token）、`at ⊢` 与不写同义 |
| `rfl` 的 whnf | `MVarId.applyRfl`：`whnfR <$> goal.getType` ⇒ `isDefEq lhs rhs` ⇒ `mkApp2 Eq.refl α lhs` | 源级**多轮展开逼近** whnf（`unfold_head_deep`，≤4 轮）+ 候选交内核 `judge` |

## 4. 同一份机械的另一处消费者：`rfl`（台账 G-114）

用户原形状 `apply h; rfl` 在修前报「`rfl` 需要目标是 `Eq α x y` 或 `Iff A B` 形状」✗ ——
因为源 AST 与内核 pp 的头都是 `Set.mem`，而 `rfl_candidate` 只认**语法上**的 `Eq`/`Iff` 头。

修法（**只在前端**，对齐 Lean 的 `whnfR`）：
`Tactic::Rfl` 在既有两条（源 AST → 内核 pp 规范形态）都失败后，再走 `unfold_head_deep`
（与 G-109 的 `apply` 修复**同一份**机械）逐轮试；每轮先过一道**便宜的形状闸**
（展开形态的头不是 `Eq`/`Iff` ⇒ 不花那次内核 pp，`rfl` 失败是常见路径）。

**零假绿面**：形状猜错 ⇒ 候选仍交内核 `judge` 终审 ⇒ 照样判红；最坏情况是"该过的没过"。

## 5. 判据（确切命令）

```bash
cargo test -p sokonanoda-front --lib unfold            # 4 条：展开 + 只展内层 + 不在目标里报错 + 无名字解析错
cargo test -p sokonanoda-front --lib rfl               # 含正向（用户原形状）与两条反向
cargo test -p sokonanoda-front --test tactic_docs      # 15 条真相表 ↔ 手写清单 ↔ 索引 ↔ 正文六节
python3 scripts/tactic-docs-lint.py                    # 表 ↔ parser 字面量（独立源码 lint）
python3 scripts/gap.py check                           # 台账 G-114 / G-115 的复现件
```

* **反向验证（已实测）**：把 `unfold_head_deep` 退回 `unfold_head_once` ⇒
  `rfl_closes_a_goal_that_is_an_eq_only_after_unfolding_the_sugar` 当场判红（报的正是用户那句文案）；
  把三处 `elab_type_for_hovers` 撤掉 ⇒ G-115 的判据当场判红。
* **用户动作级判据**：G-115 用**真 LSP** 的 `textDocument/definition` 打在**用户实际点的字符**
  （失败声明签名里 `{a}` 的 `{`）上，并断言**落点行**是 `def singleton`（不是"非 null"）。

## 6. 已知偏离与边界（白纸黑字）

* **实参对齐是源级近似**：`UnfoldAlign`（源形态 / 旧写法 / pp 形态）是"按名字对齐"的启发式，
  而 Lean 在 de Bruijn 层展开 ⇒ 定义体里**重新绑定形参名**时可能串名（同 `beta_apply` 的已知
  边界；课程库没有那种写法 ✓）。`unfold` 另有**内核验收**兜底 ⇒ 错位展开不会被静默提交 ✓。
* **递归 `def` 的展开引理**（`id.eq_def`）：**n/a** —— 本语言没有递归 `def`（取证：名字在
  定义体 elaborate **之后**才进环境 ⇒ `def myLoop (n : Nat) : Nat := myLoop n` 报
  `unknown identifier` ✓）。判据 `a_self_referential_def_is_rejected_so_unfold_has_no_recursive_case`
  **断言当前行为** ⇒ 将来真加了递归 `def` 它会红，逼着补分支 ✓（防漂移 ✓）。
* ~~**`display_expr` 对"部分应用的糖"会丢实参**~~ —— **已修（2026-10-11 同轮）**：根因是
  **文本折叠的固有歧义**（`Set.singleton a a` 同时读得出"开项完全应用"与"闭项完全应用
  再被应用一次"）⇒ 修在**折叠层**：糖**只折闭项形状**（`Set.singleton a` ✓ / `Set.pair a b` ✓），
  **开项形状不折** ✓ —— 它与「闭项再被应用」逐字同形，只能靠「少折一点」消歧 ✓。
  ⚠ 第一版修在**渲染层**（闭项元数的糖打成 `{a}`）**不成立**：糖的含义取决于**环境**
  （夹具里 α 是显式绑定的 ⇒ `Set.singleton α` 是部分应用，打成 `{α}` 就错 ✗），而
  `render_expr` 看不到环境 ⇒ 被既有 CLI 判据（`inequality_delta_unfolding_…`）逮住 ✓。
  判据 `an_applied_set_literal_shows_the_application_instead_of_dropping_an_argument`
  （面板与错误消息两条路）+ 反向验证 ✓。附带实证：**判定输入不许走显示层折叠** ✓。
* ~~**`unfold` 的实参名暂无 hover 行**~~ —— **已修（2026-10-11 同轮）**：成功臂补词法行，
  关键是**用源表达式**（`let val = &lowered.0` 之后那个 `val` 已是 elaborate 过的项、
  tactic 语法一个字节不剩 ✗）。判据：front `the_argument_of_unfold_keeps_a_jump_row_in_a_successful_declaration`
  + LSP `goto_definition_on_an_unfold_argument_lands_on_that_declaration`（**用户动作** ⇒ 断言落点行）；
  反向验证：撤补位 ⇒ 两条都判红 ✓。
