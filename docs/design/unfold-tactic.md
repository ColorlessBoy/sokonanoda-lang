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
| 边界 | **不做** `at h` 位置（只改主目标）—— 与 `intro`/`cases` 的子集口径一致 |
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
| 递归 `def` | 用生成的展开引理 `id.eq_def` 走 `simp`（只展开一层） | **不做**（课程库里没有递归 `def` 的展开需求；如实登记为边界） |
| `at h` 位置 | 支持（`withLocation`） | **不做**（子集边界） |
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

* **实参对齐是源级近似**：`unfold_one_with` 的 `UnfoldAlign`（源形态 / 旧写法 / pp 形态）
  是"按名字对齐"的启发式，而 Lean 在 de Bruijn 层展开 ⇒ 定义体里**重新绑定形参名**时可能串名
  （`substitute` 不 capture-avoiding，同 `beta_apply` 的已知边界）。课程库里没有那种写法 ✓；
  `unfold` 另有**内核验收**兜底 ⇒ 错位展开**不会**被静默提交 ✓。
* **递归 `def` 的展开引理**（`id.eq_def`）不做 —— 今天没有消费者（课程库全是非递归 `def`）。
* **`display_expr`（错误消息用的折叠渲染）对"部分应用的糖"会丢实参**（2026-10-11 实测）：
  `unfold Set.mem` 之后的目标 AST 是 `App(SetLiteral{a}, a)`，`render_expr` 忠实给出
  `({a}) a` ✓，而 `display_expr`/`fold_for_display` 折成 **`{a}`** ✗（把外层应用吃掉了）。
  **今天的影响面**：只影响**错误消息**里对这类项的渲染（目标面板/`query state` 走的是
  另一条路，实测显示 `({a}) a` ✓ 正确）⇒ 本轮**不动**它，如实登记在这里；真要修，
  与"记法折叠只折**完全应用**"（`34cc5264`）同一条规则（糖也要照做）。
  ⚠ 这也是"判定输入**不许**走显示层折叠"的实证：本设计一开始用 `display_expr` 造
  验收文本 ⇒ 判据当场判红 ✓（现已改用 `crate::proof::render_atom` ✓）。
* **`unfold` 的实参名暂无 hover 行**：它不进证明项（只改目标），而 hover 行来自构建期的
  `elab_expr` ⇒ 成功声明里 `unfold Foo` 的 `Foo` 目前没有 hover/F12（失败声明走 G-108 的
  词法行，可跳 ✓）。需要时另立一条（列在这里，免得当成 bug 查）。
