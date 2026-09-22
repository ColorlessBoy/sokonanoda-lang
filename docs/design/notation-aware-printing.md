# goal / 类型行用记法（线 C 的设计）

> 2026-09-21 起。计划条目见 `docs/design/vscode-editor-feedback-plan.md` §6；
> 用户原始反馈：「infoview 里的 goal 展现没有用 notation 的方式」。

## 1. 实测：四个生产者，两种行为（T-C01）

**这一节先做**——不量清楚就会修错 surface（改了"声明卡片"而用户看的是"目标栏"，
或者反过来）。

### 1.1 四个生产者

| # | 生产者 | 谁在用 | 文本来源 | 记法 |
|---|---|---|---|---|
| 1 | **根状态**（光标在 `by` / `sorry` 上） | `soko/stateAt` 的目标栏 | `DeclState.ty_text` = **内核 pp**（`compile/check/kernel_phase.rs` → `pp.pp_expr`） | **丢** |
| 2 | **声明级目标**（`soko/goals` 的 `decl.goal`） | 目标栏（光标不在 tactic 上时） | `render_expr(ty)`（`compile/goals.rs`） | **保留** |
| 3 | **声明列表的 `ty`** | Infoview 的声明卡片 | 同 #1（内核 pp） | **丢** |
| 4 | **`by` 步进**（`ByGoal.ty`） | 目标栏的进度 | `render_expr(&nodes[id].ty)`（`by.rs`）——**但 `apply` 的子目标例外** | **多数保留**，`apply` 丢 |

### 1.2 逐格实测（2026-09-21，0.64.2，`./target/debug/sokonanoda`）

命令模板（每个格子都能重跑）：

```bash
sokonanoda query state --file <入口> --line <行> --col 3   # 生产者 1 / 4
sokonanoda query goals --file <入口>                        # 生产者 2 / 3
```

**`units/unit01-sets-membership.sokonanoda`**（6 个开练习）：

| 位置 | 生产者 | 实测文本 | 记法 |
|---|---|---|---|
| `demo_subset_def` 的 `ty` | 3 | `forall (α : Type 0) (A B : Set α), Iff (Set.subset α A B) (forall (x : α), A x -> B x)` | **丢** |
| `mem_of_subset` 的 `goal` | 2 | `(a ∈ A) -> a ∈ B` | **保留** |
| `mem_of_subset` 的 `ty` | 3 | `forall (α : Type 0) (A B : Set α), Set.subset α A B -> (forall (a : α), Set.mem α a A -> …)` | **丢** |
| `mem_of_subset` 的 `sorry` 行（L45） | **1** | `forall (α : Type 0) (A B : Set α), Set.subset α A B -> (forall (a : α), Set.mem α a A -> Set.mem α a B)` | **丢** |
| `demo_mem_def` 的 `by` 行（L33） | **1** | `forall (α : Type 0) (a : α) (A : Set α), Iff (Set.mem α a A) (A a)` | **丢** |
| 同行 `constructor`（L34） | 4 | `(a ∈ A) -> A a` | **保留** |
| `intro h`（L35） | 4 | `A a` | 保留 |
| `exact h`（L36） | 4 | `A a -> a ∈ A` | 保留 |
| 最后一行（L37） | 4 | `a ∈ A` | 保留 |

**`units/unit08-images-preimages.sokonanoda`**（9 个开练习）：

| 位置 | 生产者 | 实测文本 | 记法 |
|---|---|---|---|
| `preimage_union` 的 `goal` | 2 | `(f ⁻¹' (B ∪ C)) = ((f ⁻¹' B) ∪ (f ⁻¹' C))` | **保留** |
| `preimage_union` 的 `sorry` 行（L72） | **1** | `forall (α β : Type 0) (f : α -> β) (B C : Set β), Eq (Set.preimage α β f (Set.union β B C)) (Set.union α (Set.preimage α β f B) (Set.preimage α β f C))` | **丢** |
| `preimage_compl` 的 `goal` | 2 | `(f ⁻¹' (B ᶜ)) = ((f ⁻¹' B) ᶜ)` | **保留** |

**`units/unit12-synthesis.sokonanoda`**（9 个开练习）：

| 位置 | 生产者 | 实测文本 | 记法 |
|---|---|---|---|
| `xlat_compl_union` 的 `goal` | 2 | `((A ∪ B) ᶜ) = ((A ᶜ) ∩ (B ᶜ))` | **保留** |
| `xlat_injective_comp` 的 `goal` | 2 | `Function.Injective α γ (Function.comp α β γ g f)` | 保留（该文件对这些名字没有记法） |
| `demo_rel_comp_assoc` 的 `by` 行（L125） | **1** | `forall (A B C D : Type 0) (r : Rel A B) …, Eq (Rel.comp A C D (Rel.comp A B C r s) t) …` | 丢（该文件无对应记法） |

**`units/unit04-extensionality-identities.sokonanoda`**（生产者 4 的**例外**，专门量它）：

| 位置 | 生产者 | 实测文本 | 记法 |
|---|---|---|---|
| `demo_ext_pattern` 的 `by` 行（L41） | **1** | `forall (α : Type 0) (A B : Set α), (forall (x : α), Iff (Set.mem α x A) (Set.mem α x B)) -> Eq A B` | **丢** |
| `apply Set.ext`（L42） | 1 | 同上（光标在 tactic 上 = **进入它之前**的状态） | 丢 |
| `intro x`（L43） | **4** | `(x : α) -> Iff (A x) (B x)` | **丢**（`∈` 与 `↔` 都没了） |
| `exact h x`（L44） | 4 | `Iff (A x) (B x)` | **丢** |

**解答**（`units/solutions/unit08-solution.sokonanoda`，`:= by` + `exact`）：

| 位置 | 生产者 | 实测文本 | 记法 |
|---|---|---|---|
| `demo_mem_image` 的 `exact` 行（L25） | 1 | `forall (α β : Type 0) (f : α -> β) (A : Set α) (y : β), Iff (Set.mem β y (Set.image α β f A)) (Exists α (fun (x : α) => And (A x) (Eq (f x) y)))` | **丢** |

> `unit01-solution` / `unit12-solution` 已经是**项风格**（线 L），没有 `by` 步进可量；
> 它们的 `goal` 走生产者 2（保留）。

### 1.3 三条从实测里读出来的结论

1. **"光标在不在 tactic 上"决定走哪一支**（半开区间 `start <= cursor < end`）：
   光标在 tactic 内 ⇒ 生产者 **1**（内核 pp，**点名**）；在 tactic 之后 ⇒ 生产者
   **2**（`render_expr`，**保留记法**）。
   **学习者写证明时光标就在 tactic 上 ⇒ 他看到的就是点名形式**——这正是用户的
   抱怨。写 e2e 时注意：`revealRange` 把光标停在 range **末尾**（= 之后）⇒ 会**假绿**，
   必须显式设 `editor.selection`（用例 #6 踩过一次）。
2. **"`by` 步进保留记法"只对结构型 tactic 成立**：`constructor` / `intro` 之后是
   `render_expr` ✓；**`apply` 之后不是**——子目标来自被应用引理的**内核 pp 望远镜**
   （`by.rs` 的 `judge_infer` → `f_ty`），所以 `apply Set.ext` 之后是
   `(x : α) -> Iff (A x) (B x)`，`∈` 与 `↔` 一起消失。
   `cases` 的 scrutinee 类型、`intro` 遇到 def-headed 目标时的规范化同族。
3. **同一个声明在两个 surface 上会给出不同的文本**（`mem_of_subset`：`goal` 是
   `(a ∈ A) -> a ∈ B`，`sorry` 行是 `forall (α : Type 0) …, Set.mem α a A -> …`）。
   ⇒ 修的时候不能只改一处：**用户抱怨的是 #1（根状态）与 #3（声明卡片）**，
   而 #2 已经是对的、#4 一半对。

### 1.4 所以线 C 要做什么（给后面的环节定靶）

| 要改的 | 不改的 |
|---|---|
| 生产者 **1**（根状态）与 **3**（声明 `ty`）——它们都是内核 pp 的文本 | 生产者 **2**（`render_expr`，已经保留记法） |
| 生产者 **4** 里经 `apply`/`cases`/规范化出来的那些子目标（同族：pp 文本回读） | 生产者 **4** 里 `constructor`/`intro` 出来的（`render_expr`） |

**红线（§6 开头那两条，别忘）**：`render_expr` 不能改（它的产物同时是 judge 的
回读输入）；内核 `pp_expr` 也不能改（它同时是 `#check`/`#reduce`/`#print` 的出口，
直接进 `--json`）。⇒ 唯一的缝是在**显示出口之后**做重写。

## 2. 消费者审计（T-C02）：**哪些文本会被回读**

> **这是线 C 的安全边界。** 一个文本字段只要被"喂回 parser/kernel"，就不能为了
> 好看去改它——改了就是**静默改坏判卷**（学习者看到"判过了"，或者本该过的被判红）。

### 2.1 五个字段 × 消费者

| 字段 | 生产者（哪来的文本） | 消费者（`file:line`） | 类别 |
|---|---|---|---|
| **`DeclState.ty_text`** | 内核 pp（`compile/check/kernel_phase.rs:170`、`:207`；`quiet_catch` 包着） | `front/src/query/mod.rs:640`（`goals()` 的 `ty`）· `:651`（同）· `front/src/query/state.rs:54`（根状态）· `lsp/src/lib.rs:1597`（hover 签名）· `lsp/src/lib.rs:1881`（补全/文档弹窗） | **全是给人看** |
| **`DeclState.goal`** | `render_expr`（`compile/check/walk.rs:455`、`:678`、`:989` 经 `goals.rs`） | 给人看：`front/src/query/mod.rs:657/661`（wire）· `lsp/src/query_map.rs:74`；**回读**：`front/src/suggest.rs:412`（`hole_goal_text`）→ `:418 open_spec` → judge | **两者都有** |
| **`DeclState.binders[].ty`**（`GoalBinder`） | 书写类型 / 借用声明层（`goals.rs`） | **回读**：`suggest.rs:425`（`open_spec`）· `compile/goals.rs:499`（`binder_spec`）· `by.rs:501-506`（`canonical_goal_type`）；给人看：`query_map.rs`（假设行） | **两者都有** |
| **`DeclState.sub_goals[].ty`** | `render_expr` | **回读**：`suggest.rs:412`（`hole_goal_text` 的另一支）→ judge | **两者都有** |
| **`ByGoalState.goals[].goal`**（`ByGoal.ty`） | `render_expr`（`by.rs:1161`） | 给人看：`query/state.rs:41/57` → `soko/stateAt` 的目标栏 | **给人看** |

### 2.2 回读链（**一个都不许改到**）

| 入口 | 它吃什么文本 | 去处 |
|---|---|---|
| `suggest.rs:418 open_spec` | `DeclState.goal`（或 `sub_goals[].ty`）+ `binders[].ty` | `OpenGoalSpec` → `judge_terms` |
| `goals.rs:499 binder_spec`（调用点 `:461-462`） | `GoalBinder.ty` | `GoalBinderSpec` → `judge_infer_with`（`:464`）→ 内核 pp 文本 → `parse_expr_text` |
| `by.rs:501-506 canonical_goal_type` | `render_expr(ty)` + `render_expr(b.ty)` | `judge_render_type` → 内核 pp 文本 → `parse_expr_text` |
| `by.rs:1239`（`apply` 的子目标） | `judge_infer` 的 pp 文本 | `parse_expr_text` → 新目标 |
| `elab.rs:1357/1389`（记法前导参数） | `judge_infer` 的 pp 文本 | `parse_expr_text` → `notation_telescope` |
| `judge.rs:1475 fold_declared` / `:1518 wrap_binders` | `GoalBinderSpec.ty` | `parse_expr_text_with(text, notations)` |
| `proof.rs:53 parse_expr_text_with` | 任意文本 | parser |

**结论（线 C 的实现约束）**：

* **`ty_text` 可以就地改**——它只有"给人看"的消费者（§2.1 第一行）。这是最便宜的
  一刀（计划里的 T-C20「生产者 1+3」）。
* **`goal` / `binders[].ty` / `sub_goals[].ty` 不能就地改**——它们同时是 judge 的
  输入。要改就得在**显示出口**（`lsp/src/query_map.rs` 组 wire 的那一处、
  CLI 打印的那一处）做重写，让"回读拿到的"与"人看到的"是两份文本。
* `by.rs` 的 `canonical_goal_type` 那条**本来就是**"pp → parse 回来"——它证明
  这条回路今天已经在跑，也正是为什么**不能**把记法塞进 pp。

### 2.3 判据提醒：线 C 会让 `kernel-diff.sh` 报差异（这是**预期**的）

`scripts/kernel-diff.sh` 对拍的是 `grade --json` **逐字节**，而 `--json` 里
**包含** goal / `ty` 这些显示文本。线 C 一旦在显示出口做重写，对拍必然报差异
——**那不等于判定变了**。

⇒ 线 C 的每个环节要分开看两件事：

| 看什么 | 判据 |
|---|---|
| **判定正确性**（红线） | 课程门禁计数**逐项不变**（`36 目标 · 328 checked · 99 open · 0 判负`）+ `cargo test --workspace` 全绿 + **接受/拒绝集合不变** |
| **显示文本**（本线要改的） | `--json` 的 golden **有意更新**，且 diff 里只出现 goal/`ty` 这类显示字段（T-C40） |

对拍仍然有用：它把"显示文本到底改了哪几处"**逐字节摊开**，比人眼扫一遍可靠。
