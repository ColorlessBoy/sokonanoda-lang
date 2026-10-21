//! 声明头部状态与洞位置模式（自 `walk.rs` 拆出，2026-10-21）。
//!
//! 拆出的理由：`walk.rs` 已经超过软线，本轮「开放练习入环境」又要往它里面加
//! 代码；按 `AGENTS.md` 硬规则 5 / `docs-lint` 判据 ⑨，存量文件**只许减不许增**
//! ⇒ 把这些与 open-env 无关的纯显示/状态函数搬到这里。

use crate::compile::report::{ByGoalState, GoalBinder, HoleMode};
use crate::{Expr, Span};

/// **洞的位置模式**（A1，2026-10-12 blocker）：`by` 引擎装配出来的**未闭合目标**
/// （`lowered.tactic_holes`）就是 **tactic 位**；其余（声明值位、项内部的洞）是**项位**。
///
/// ⚠ 判据是洞的**出身**，不是文本启发式 ✗：`:= And.intro (by exact hp) sorry` 里那个
/// `sorry` 是**项位**（虽然同一行里有 `by`），而 `by\n apply And.intro\n sorry` 里的洞是
/// **tactic 位**（虽然降级后的值里已经看不到 `by` 关键字）⇒ 只有降级那一步知道，
/// 所以由 [`crate::by::ByOutcome::tactic_holes`] 带出来。
///
/// **不变量**：返回表与 `holes` **等长逐位对齐**（唯一生成点，见
/// `DeclState::hole_modes` 的文档）。
pub(super) fn hole_modes_for(holes: &[Span], tactic_holes: &[Span]) -> Vec<HoleMode> {
    holes
        .iter()
        .map(|h| {
            if tactic_holes.contains(h) {
                HoleMode::Tactic
            } else {
                HoleMode::Term
            }
        })
        .collect()
}

/// **声明头部的根状态**：**只剥源位 λ 链的绑元**（= 声明里写的**具名绑元**），
/// **不是**整条 ∀ 望远镜 ✗。
///
/// 为什么（**2026-10-02 值守第 9 单**，用户实测报的 ✗）：前端 lowering 会给
/// **函数型语句**（`a → b → a` / `∀ x, …`）补 λ ⇒ `split_by_value(val)` 的 λ 链
/// **含语句自身的 Π 层** ⇒ 按它剥 `peel_pi_layers(ty, n)` 会**多剥**（目标的箭头全没、
/// 上下文多出匿名 `_`）；层数不够时 `peel_pi_layers` 返 `None` ⇒ `by_root = None` ⇒
/// 退回 `ty_text` + 空 binders ⇒ 顶显示整句 `∀` ✗。
/// 实测：`theorem (a b : Prop) : a → b → a := by` ⇒ 顶 `[a,b,_,_] ⊢ a` ✗
/// vs 底（声明卡片）`[a,b] ⊢ a → b → a` ✓。
///
/// 正确口径 = **与声明卡片同源**：open 声明在调用点直接用 `open_goal` 的 `info`
/// （顶 ≡ 底 **by construction** ✓）；其余（失败 / 已证）用这里的 λ 前缀版 ✓。
///
/// **题面状态**（G-82 兜底，2026-10-03）：声明**源位 λ 链的具名绑元** + 按它们
/// 剥出的剩余目标 —— 未折记法的**真相文本**（喂 judge 的那一份 ✓）。
///
/// 为什么要有这一条：`open_goal` 分解不了值（记法头、超量应用、def 展开间接调用…）
/// 时，`walk.rs` 原先退回 **空上下文 + 整句声明类型** ✗ —— 有具名绑元的题就变成
/// 「题目参数忽然跑到目标里去了」，整句判红 ✗（用户 2026-10-02 实测报的 G-82）。
/// 走查分解不了**不等于**题面没有上下文：题面状态**永远**是"具名绑元进上下文、
/// 目标只剩剥掉它们之后的命题" ✓。
///
/// 与 [`decl_prefix_state`] **同源**（那个 = 这条 + 折一次记法）⇒ 顶 ≡ 底
/// 在这条兜底路径上也成立 ✓。`val` 必须是**源位**（lowering 之前）的值：lowering
/// 会给函数型语句补 λ，拿 lowering 之后的值数 λ 链会多剥 ✗。
pub(super) fn decl_root_state(
    ty: &Expr,
    src_val: &Expr,
    display: &crate::display::DisplayNotations,
) -> Option<(String, Vec<GoalBinder>)> {
    let (binders, _) = crate::by::split_by_value(src_val)?;
    let body = crate::proof::peel_pi_layers(ty, binders.len())?;
    // **走唯一显示接口** ✓（`scripts/audit-notation-paths.py` 的棘轮 ✓）：绕过它 ⇒
    // Infoview 顶部目标退回**点形式** ✗ —— 正是 G-81/G-83 那一族的病根 ✓。
    // `fold` 幂等 ✓ ⇒ 下面 `decl_prefix_state` 再折一次不改变结果 ✓（那两处照旧逐字节不变 ✓）。
    Some((
        display.render(&body),
        binders
            .iter()
            .map(|b| GoalBinder {
                name: b.name.clone(),
                ty: b
                    .ty
                    .as_deref()
                    .map(|t| display.render(t))
                    .unwrap_or_default(),
            })
            .collect(),
    ))
}

/// 题面状态的**显示副本**（折一次记法）：`by_root` / hover 直接渲染它 ✓。
pub(super) fn fold_root_state(
    state: &(String, Vec<GoalBinder>),
    display: &crate::display::DisplayNotations,
) -> ByGoalState {
    let (goal, binders) = state;
    ByGoalState {
        ty: display.fold(goal),
        binders: binders
            .iter()
            .map(|b| GoalBinder {
                name: b.name.clone(),
                ty: display.fold(&b.ty),
            })
            .collect(),
    }
}

/// 同 [`decl_root_state`]，但**折一次记法**（显示副本）—— 失败 / 已证声明用。
pub(super) fn decl_prefix_state(
    ty: &Expr,
    val: &Expr,
    display: &crate::display::DisplayNotations,
) -> Option<ByGoalState> {
    decl_root_state(ty, val, display)
        .as_ref()
        .map(|state| fold_root_state(state, display))
}
