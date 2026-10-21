# apply-goal-driven · as-built + Lean 4 对照（2026-10-21）

隔离树 `target/impl/src` @ HEAD `409385b1` + 本批。**未发布**；共享树零写入（落地另起）。

## 1. 改了什么

| 项 | 位置 | 说明 |
|---|---|---|
| 1a 失败边重试 | `by.rs::apply_tactic` + `apply_probe.rs::retry_with_goal`/`apply_type_fits` | 文本「读不回来 / 结论对不上」才用目标当期望类型重试一次；只有新文本**真的能用**才替换 ⇒ 成功路径零新增 |
| 1b 层归一 | `level_text.rs::normalize_level_mvars`（在 `judge::peel_binders` 之前） | 把 `?u.<id>` 归一成层参数名；**守卫**：`u_soko*` 不许进结论/子目标域 |
| 1b 信号 | `by.rs` 的层角色判定 | 名字出现在**后续任何一层的域**里也算类型参数（`Eq.symm.{0}` 的 `α` 只出现在 `(a b : α)` 的域里） |
| 1c 可诊断 | `judge::peel_binders` | `SOKO_PEEL_PROBE=1` ⇒ `[peel-break] parse failed …` |
| 诊断病因 | `apply_probe.rs::unreadable_message` | `f_ty` 含 `?u.` ⇒ 说「宇宙层/参数还没定下来（钉层或换写法）」，否则保留 D5 的「实参没写全」 |
| 拆分 | `by.rs`/`judge.rs` → `apply_probe.rs`/`level_text.rs` | 落地时 `docs-lint` ⑨ 判红（`by.rs` 4466 > 基线 4353、`judge.rs` 4728 > 4670）⇒ 按「拆出去」做，现 **4292 / 4663**（都回到基线内） |

## 2. 读数（同一构建内 A/B）

**代价（如实）**：每次 `apply` 多一趟**源级** `parse → 剥层 → 合一`（不进内核）；
`level_hint_of`（可能落到 `judge_infer` 重编前缀）**只在无 hint 判不过时**才买
（`apply_type_fits` 先试 `None`）。实测 4 条基准墙钟与 `--json`（剔时间字段）无差。

- `cargo test -p sokonanoda-front --lib`：**1019 passed / 0 failed**（D5 期望已改 + 4 条新判据 + 1 条 B 信号单测，全部断言 `by_steps[0].goals`）。
- `lib/Set`：`query check` 报 **28 checked / 0 failed**（两档一致）。
- 矩阵 `scripts/apply-goal-driven-matrix.sh`：**exit 0**；三个开关全关 ⇒ exit 1。
- **全课程对拍（release）**：fixed vs HEAD 行为（开关 `SOKO_APPLY_GOAL_FIRST=0 SOKO_APPLY_LEVEL_TEXT=0`）逐文件
  `grade --json` **剔除时间字段后** ⇒ **TOTAL=252 · SAME=252 · DIFF=0**（不是原始字节相同）。
- **课程门禁**（release）：**258 目标 · 2190 checked · 915 open · 0 判负 · exit 0**。
  （第一轮 14 判负全是 `gaps/*.sh` 复现脚本：隔离树没有 `<树>/target/release` ⇒ `scripts/soko` exit 3；
  把 release 放进 `target/release/` 后重跑即绿 —— **不是代码红**。）
- 两个回退开关实测：`SOKO_APPLY_LEVEL_TEXT=0` ⇒ `Eq.symm` 退回「读不回来」；`SOKO_APPLY_GOAL_FIRST=0`
  ⇒ `Eq.trans h` 退回 defaulting。**没有** `SOKO_APPLY_PENDING`（B 的结论把它删了，见 §4）。

## 3. 批次 B 的 spike：结论 = **不需要 `Pending`**（并且降级是错的）

**先更正一条误判**：`apply junk`（`junk (α : Type) (P : Prop) : P -> P`）今天给出
`['Sort 1', 'P']` —— 这**不是伪目标**：`α` 是**显式**类型参数，Lean 的 `forallMetaTelescope` 同样会为它
留一个元变量，`apply` 会把它**当成一个目标**（`⊢ Type`）✓。所以「目标集应为 `['P']`」是**我写错的期望**，
按它做的「点名报错」降级会**拒绝 Lean 合法的写法** ⇒ 已**回退**（`SOKO_APPLY_PENDING` 开关一并删除）。

**真正的 bug 只有一处**：`{α : Sort u}` 这种**隐式**类型参数被误判成子目标（`apply Eq.symm.{0}` 多出
`⊢ Prop`）。修法是 §1 的**信号**：名字出现在后续层的域里就算类型参数 ⇒ 现在目标集是 `['x = a']`，
且**闭合**的同类证明仍被内核拒（`期望 Sort(0)，实际是 Sort(1)`；带 `sorry` 时不进内核检查，所以只在闭合时暴露）。

**要不要项级 mvar（`ApplyArg::Pending`）**：**不要**，且现在也做不到 ——

- `crates/kernel/src/builder.rs:590-618`：`add_declar` **入口硬拒**含 `Expr::Meta` 的项；
- `crates/kernel/src/eval.rs:577`：`Expr::Meta => panic!`；
- `crates/kernel/src/writer.rs:244`：序列化也拒；
- 它的种类 `SyntheticOpaque` 的设计就写着「别拿类型约束去填它」（`expr.rs:95-103`）。

⇒ 内核没有「由合一赋值、能被检查器接受/报告」的项级 mvar；要做是阶段 4（apply 不再经过文本）的地基。
**B 的正确交付 = 信号修复**（已落）+ 这条「显式参数就是目标」的结论（写进设计文档，防再被当 bug 修）。

## 4. Lean 4 对照（动手前取证）

本机源码 `/Users/penglingwei/Documents/lean/lean4` @ `d0493e4c1e`。

- **`apply` 的合一顺序**：`Meta/Tactic/Apply.lean:204-212` —— `forallMetaTelescopeReducing` 之后**立刻**
  `isDefEqApply cfg.approx eType targetType`（**目标从第一步就在场**），失败再 `restore` 并加大下划线数重试。
  我们：先问类型（无期望类型）再对文本 ⇒ `Eq.trans h` 的 `c` 被 defaulting 成 `a`；1a 把「带目标」从
  「只在上一刀 Err 时」改成**失败边**重试（成功路径不动，避免 `lib/Set` 28→19 的记法目标回归）。
- **未解层的收口**：Lean 在**声明出口** `levelMVarToParam` 把未解层转 param（台账 G-71 记的
  `Term/Elab/Declaration.lean:118-127`）；`#check` 也有出口（`BuiltinCommand.lean:435`）。我们：`#check`
  走 `discharge_expr` ✓；就地路（`apply` 问类型）**没有出口** ⇒ `?u.<id>` 漏进文本 ⇒ 1b 在文本层归一。
- **有没有「pp 成文本再 parse 回来做判定」**：Lean **没有**（全程 `Expr`/`MVarId`）⇒ 我们的文本中介是
  有意偏离（台账 G-109），本批不消除，只补失败边。

## 5. 回退与边界

- 回退：`SOKO_APPLY_GOAL_FIRST=0`（1a）· `SOKO_APPLY_LEVEL_TEXT=0`（1b）。
- **慢路**：`SOKO_JUDGE_INPLACE=off` 下 1a/1b 不生效（`judge_infer` 没有「带期望类型」能力）⇒ 写进发布说明，
  由阶段 4 收口。
- 已知未修：strict `@` 的 `Sort(1) vs Sort(2)`（协作项 C，探针已定位：`conv.rs` 的 Sort 比较两侧都是
  `Zero` 基、差 **1 个 succ**，不是层 mvar，`level_solve` 不适用）。

## 6. 给方言线（`draft/strict-mode-v2`）的合并指引

- **本分支**：`apply-goal-driven`（自 `409385b1`；tip `84668c9f`，5 个提交：`55447d56` 代码 / `9836e60e` 设计+复现件 /
  `547cf4bf` STATUS / `9702260e` 评审收口 / `29064d72` e2e 台账 / `84668c9f` 信号收窄）。方言线在 worktree
  `/private/tmp/sokonanoda-strict-v2-draft` @ `4f5245b6`。
- **动了哪些文件**：`crates/front/src/{by.rs,judge.rs,lib.rs,apply_probe.rs,level_text.rs}`（前两个**变短**，
  后两个**新增**）、`crates/front/src/compile/{elab.rs,tests.rs}`、`docs/**`、`scripts/apply-goal-driven-matrix.sh`。
- **冲突热点**：`by.rs`（`apply_tactic` 里新增了 1a 的重试调用与层角色信号）、`judge.rs`（`peel_binders` 的
  probe + `normalize_level_mvars` 的 re-export）、`lib.rs`（模块声明）。**行号会漂**：注释里引用的
  `by.rs:2444` 这类位置需要按 rebase 后的实际行改。
- **可见性**：`GoalNode` 的字段**仍是私有**；只有 `level_hint_of` / `check_ast_of_text` / `def_shape` /
  `is_concrete_level` 提升为 `pub(crate)`（`level_hint_of_inplace` 搬进 `apply_probe.rs` 后要用）。
- **不要在方言线里期待本分支修 strict `@`**：`Sort(1) vs Sort(2)` 的取证（`conv.rs` 的 Sort 比较两侧
  都是 `Zero` 基、差 1 个 succ ⇒ 不是层 mvar）与本分支无关；`level_solve` 不适用。
- **回退开关**：`SOKO_APPLY_GOAL_FIRST=0`、`SOKO_APPLY_LEVEL_TEXT=0`（慢路 `SOKO_JUDGE_INPLACE=off` 下两者都不生效）。
- ⚠ **层角色信号只对「域是 `Sort`」的层生效**（`84668c9f`）：`w : A` 这种**项**参数也出现在后续层的域里
  （`h : p w`），不收窄会把 `apply Exists.intro` 的子目标全打乱（`course/solutions/unit8` 当场红）。
  你 rebase 后**请跑** `cargo test -p sokonanoda-cli --test course --locked`（6/6）——它是这条的守卫。
