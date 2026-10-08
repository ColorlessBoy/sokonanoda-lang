//! 光标处状态的选择（Lean `goalsAt?` 语义的**唯一实现**）。
//!
//! 从 `mod.rs` 拆出（模块化硬规则：单文件 ~500 行上限）。语义与协议原文见
//! [`select_state_at`] 的文档；适配器（LSP/CLI/MCP）只做坐标转换，不得重算。

use crate::compile::{ByGoalState, DeclState, DeclStatus};
use crate::span::Span;

/// 光标处的状态选择（Lean `goalsAt?` 语义的唯一实现）。
pub struct StateSelection {
    pub goals: Vec<ByGoalState>,
    pub span: Option<Span>,
    pub step: i64,
    pub total: usize,
}

/// 光标处的状态选择（**Lean `goalsAt?` 语义的唯一实现**，协议原文见
/// `docs/protocol.md` §`soko/stateAt`）：
///
/// - 光标落在某条 tactic 的 span 内（**开区间** `start < cursor < end`）→ 该 tactic
///   **执行后**的状态，即第 `i` 条执行后的状态（**与 Lean 4 的 `hoverPos > pos`
///   一致** ✓）；光标**恰在起点**（`cursor == start`）⇒ 仍是"**进入**该 tactic 之前"
///   （第 `i-1` 条之后）；光标**恰在末尾/之后** ⇒ 归入"最后一条在光标前结束的 tactic"。
///   ⚠ **2026-10-08 更正（B2 · 与 Lean 对齐）**：这里原来是 `start <= cursor < end ⇒ i-1`
///   （**半开区间取"进入态"**）⇒ 与 Lean 4 **相反** ✗。权威对照：
///   `~/Documents/lean/lean4/src/Lean/Server/InfoUtils.lean:448-481` 的
///   `useAfter := hoverPos > pos && !cs.any (hasNestedTactic pos tailPos)` ⇒ Lean 在
///   tactic **内部**显示的是 `goalsAfter`；样本
///   `tests/lean/interactive/plainGoal.lean.expected.out`：`intro a` 的 char2（起点）
///   ⇒ 前态 · char3 ⇒ **后态** ✓。**边界两条**：`cursor == span.start` 仍是进入态；
///   `cursor == span.end` 已归入"结束于光标前"（与"末尾算之后"的旧口径一致 ✓）。
/// - 否则取"最后一条在光标前（含恰好结束）结束的 tactic"之后的状态；
/// - 根状态（`step: -1`）：**第一条 tactic 之前的状态** = 声明的剩余目标
///   （`goal`）+ **声明的 ∀ 绑元**（`binders`），`span` = 声明范围。
///   ⚠ **2026-10-02 更正（用户实测报的 bug）**：这里原来渲的是**声明类型的内核
///   文本**（`ty_text` ⇒ 整句 `∀ (a b : Prop), …`）+ **空 binders** —— 那不是
///   任何 Lean 意义上的"证明状态" ✗：定理的 `∀` 绑元在证明开始时就**已经引入
///   上下文**，初始目标只剩**命题本身**（Lean 的 `goalsAt?` 同此）。旧行为让
///   Infoview 面板顶上显示整句量词式，且 `a`/`b` 被标成 `unknown_ident`
///   （应是 `binder`）✗。协议原文同步更正（`docs/protocol.md` §`soko/stateAt`）。
/// - **没有 `by` 块的声明**（`axiom`、lambda 前缀 + `sorry` 的半成品、已证完的
///   声明）：`step: -1`、`total: 0`，退回声明自己的剩余目标/上下文
///   （`goal` + `binders`；已闭合时为 `[]` ⇒ wire `goal: null`）。这与"根状态"
///   是两回事——根状态只属于有 tactic 的声明。
///
/// **题面状态（2026-10-02 用户四形矩阵定稿）**：声明头部/第一条 tactic 之前
/// （`step: -1`）答的是**这道题本身** —— `binders` = 声明的 ∀ 参数（按序，含
/// 匿名箭头的 `_`）、`goal` = 剥掉它们之后的命题。**四形都要成立**：冒号前绑元 ·
/// 冒号后箭头 · 无 `by` 块 · **失败的声明**（后者旧行为吐 `goal: null`，面板据此
/// 显示「已无目标 ✓」——对一道没通过的题是假话 ✗）。
///
/// 这些条款都是协议规定、且 LSP 客户端（VS Code Infoview / 练习树）依赖的行为；
/// 真相层必须与之逐字一致。
///
/// 逃生门 `SOKO_STATE_ROOT=legacy`：恢复**改动前**的行为（根状态吐声明类型文本 +
/// 空 binders、失败声明吐 null）——**只给反向验证用**（见
/// `docs/gaps/repro/` 的这条复现件：`scripts/expect-red.sh` 断言撤掉修复后判据必须红）。
pub fn select_state_at(d: &DeclState, cursor: usize) -> StateSelection {
    let legacy = state_root_legacy();
    // 无 `by` ⇒ 没有 per-tactic 状态可选。
    if d.by_steps.is_empty() {
        // **失败的声明**：没有 per-tactic 状态，但它**仍然有一道题**（走查记下的
        // `by_root` = 题面）。旧行为这里吐 `goal: null` ⇒ 面板显示「已无目标 ✓」✗
        // （用户 P2 实测：`constructor` 被拒 ⇒ 声明 Failed ⇒ 面板谎报已证完）。
        if !legacy && matches!(d.status, DeclStatus::Failed) {
            if let Some(root) = d.by_root.clone() {
                return StateSelection {
                    goals: vec![root],
                    span: Some(d.span),
                    step: -1,
                    total: 0,
                };
            }
        }
        return StateSelection {
            goals: d
                .goal
                .clone()
                .map(|ty| ByGoalState {
                    ty,
                    binders: d.binders.clone(),
                })
                .into_iter()
                .collect(),
            span: Some(d.span),
            step: -1,
            total: 0,
        };
    }
    // 根状态 = **第一条 tactic 之前**的状态 —— 由走查在 lowering 时记下
    // （`DeclState.by_root`：声明的 ∀ 绑元 + 剥掉它们之后的命题）。
    //
    // ⚠ **不能**用 `d.goal`/`d.binders` ✗：那是**洞处**的状态（走查引入的假设
    // 已经进去了）——`by intro a; …; sorry` 的根状态会错成"洞处那一步"✗
    // （实测：`and_swap` 的根状态被算成 `b ∧ a`，应为 `And a b → And b a`）。
    // 也不能退回 `ty_text` + 空 binders ✗（声明类型不是证明状态，见上）。
    // 三种声明写法要分开（Lean `goalsAt?` 同此）：
    // ① **具名绑元**（`theorem t (a : Prop) : P := by`）⇒ 绑元在证明开始时已在
    //    上下文里 ⇒ 用走查记下的 `by_root`（λ 前缀的绑元 + 剥掉它们之后的命题）✓；
    // ② **箭头式类型**（`theorem t : (a : Prop) -> P := by`）⇒ 陈述里**没有**
    //    具名绑元 ⇒ 初始目标就是整个 Pi 类型、上下文为空 ⇒ 退回 `ty_text` ✓
    //    （`by_root` 为 `None`，因为值位没有 λ 前缀 ✓）；
    // ③ 洞处状态（`d.goal`/`d.binders`）**任何时候都不许**用在这里 ✗ ——
    //    它是"走查走到洞那一步"的状态，不是"第一条 tactic 之前"。
    let root = || StateSelection {
        goals: if legacy {
            // **改动前**的行为（只给反向验证用 ✗）：声明类型的内核渲染文本 +
            // 空 binders —— 面板据此把整句量词式当目标（用户报的就是这条）。
            d.ty_text
                .clone()
                .map(|ty| ByGoalState {
                    ty,
                    binders: Vec::new(),
                })
                .into_iter()
                .collect()
        } else {
            d.by_root
                .clone()
                .or_else(|| {
                    d.ty_text.clone().map(|ty| ByGoalState {
                        ty,
                        binders: Vec::new(),
                    })
                })
                .into_iter()
                .collect()
        },
        span: Some(d.span),
        step: -1,
        total: d.by_steps.len(),
    };
    let selected = match d.by_steps.iter().position(|s| {
        // **开区间**（B2）：`start < cursor < end` ⇒ 该 tactic **执行后** ✓
        // （Lean `useAfter` 的 `hoverPos > pos`；`cursor == start` 仍是"进入态" ✓）。
        if state_after_legacy() {
            s.span.start.offset <= cursor && cursor < s.span.end.offset
        } else {
            s.span.start.offset < cursor && cursor < s.span.end.offset
        }
    }) {
        Some(i) => {
            if state_after_legacy() {
                i as i64 - 1
            } else {
                i as i64
            }
        }
        None => d
            .by_steps
            .iter()
            .rposition(|s| s.span.end.offset <= cursor)
            .map(|i| i as i64)
            .unwrap_or(-1),
    };
    let Some(step) = usize::try_from(selected)
        .ok()
        .filter(|i| *i < d.by_steps.len())
    else {
        return root();
    };
    let s = &d.by_steps[step];
    StateSelection {
        goals: s.goals.clone(),
        span: Some(s.span),
        step: selected,
        total: d.by_steps.len(),
    }
}

/// 逃生门（**只给反向验证用**）：`SOKO_STATE_ROOT=legacy` 恢复改动前的根状态
/// 行为。任何生产路径都不该设它 ✓。
fn state_root_legacy() -> bool {
    matches!(std::env::var("SOKO_STATE_ROOT").as_deref(), Ok("legacy"))
}

/// 逃生门（**只给反向验证用** · B2）：`SOKO_STATE_AFTER=0` 恢复改动前的
/// "**进入态**"选择（`start <= cursor < end ⇒ i-1`）。任何生产路径都不该设它 ✓
/// —— 它的存在只为让判据能**反向验证**（撤掉修复 ⇒ ①④⑤与 webview 判据必须红 ✓）。
fn state_after_legacy() -> bool {
    static LEGACY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LEGACY.get_or_init(|| matches!(std::env::var("SOKO_STATE_AFTER").as_deref(), Ok("0")))
}
