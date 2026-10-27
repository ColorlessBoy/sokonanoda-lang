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

/// **光标落在哪一条声明上**（真相层的唯一定位实现；`QueryDoc::state_at` 用它）。
///
/// 判据两条：
/// 1. `span.start <= cursor <= span.end` —— 既有口径（**两端都闭**；声明 span 不含
///    行尾换行，见 `crates/lsp/src/tests/goals.rs` 的边界注释）；
/// 2. **外加"声明末尾的空白尾巴"**（2026-10-09 需求 4 用户实测）：光标停在声明体
///    之后的**行尾空白**或**空白行**上时，仍算这条声明。
///
/// 第 2 条要的是"光标在题目证明过程后面的空行上，面板不许说『光标不在任何声明内』"——
/// 那时该声明的剩余目标（或「🎉 已无目标 ✓」）才是用户要看的东西 ✓。
/// **边界**：尾巴在遇到**第一行非空白内容**时立刻断（下一条声明、注释、`#check`/
/// `#print` 命令行都算内容）⇒ C3 的 `#check` 那一格仍然答"不在任何声明内"，
/// 于是命令输出块照旧显示 ✓（不能把 `#check` 行首吞进上一条声明 ✗）。
///
/// 判据：`crates/front/src/query/tests.rs::state_at_owns_the_blank_lines_after_a_declaration`
/// （真相层）+ `crates/lsp/src/tests/state.rs` 的同名 wire 判据（屏幕上看得见的那一层）。
pub fn decl_at_cursor<'a>(
    text: &str,
    decls: &'a [DeclState],
    cursor: usize,
) -> Option<&'a DeclState> {
    decls.iter().find(|d| {
        (d.span.start.offset <= cursor && cursor <= d.span.end.offset)
            || (cursor > d.span.end.offset && in_blank_tail(text, d.span.end.offset, cursor))
    })
}

/// 光标是不是落在 `end` 之后的**空白尾巴**上（行尾空白 / 空白行）。
///
/// **逐行判**，不是"`end` 到光标之间字节全是空白" ✗ —— 后者会把 `#check` 行的
/// **行首**吞掉（`def myid … := x\n#check myid` 里中间那个 `\n` 是空白，
/// 但光标所在的那一行不是空白行 ✗）。
///
/// * ① 声明自己那一行：`end` 到行尾只许空白（行尾空格 / 制表符 / 换行 ✓）；
/// * ② 跨行之后：**光标所在行为止，每一行都必须是空白行**（含其换行）；
/// * ③ 光标落在空白行上的任何位置都算（行首 / 行尾 / 换行处 / EOF 那个空行）。
fn in_blank_tail(text: &str, end: usize, cursor: usize) -> bool {
    debug_assert!(cursor >= end, "只有 `end` 之后的位置才谈得上尾巴");
    let bytes = text.as_bytes();
    let len = bytes.len();
    let end = end.min(len);
    let cursor = cursor.min(len);
    // ① 本行行尾。
    let mut line_end = end;
    while line_end < len && bytes[line_end] != b'\n' {
        line_end += 1;
    }
    if !text[end..line_end].trim().is_empty() {
        // 声明后面还有别的内容（注释 / 命令 / 下一条声明）⇒ 没有空白尾巴。
        return false;
    }
    if cursor <= line_end {
        return true;
    }
    // ② 跨行：逐行要求整行空白，直到光标落在其中某一行上。
    let mut line_start = line_end + 1;
    loop {
        let mut line_end = line_start;
        while line_end < len && bytes[line_end] != b'\n' {
            line_end += 1;
        }
        if !text[line_start..line_end].trim().is_empty() {
            return false;
        }
        if cursor <= line_end {
            return true;
        }
        if line_end >= len {
            // 光标在文件末尾之后（调用方已排除越界；这里只兜底）。
            return false;
        }
        line_start = line_end + 1;
    }
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
    select_state_at_in_text(d, cursor, "")
}

/// 与 [`select_state_at`] 同一语义，但能看见**源码文本**（判定 `·` 的步进）。
///
/// 为什么要文本：`·` 的那一格记的是**聚焦目标**（块体开跑之前），而普通 tactic
/// 记的是**执行后** —— 选择器必须能区分这两者（`span` 本身分不出来，`have` 的
/// span 也覆盖它的嵌套步）。本文本只用于看 `span.start` 处是不是 `·`，不参与
/// 任何判定。
pub fn select_state_at_in_text(d: &DeclState, cursor: usize, text: &str) -> StateSelection {
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
    let at = |step: usize| -> StateSelection {
        let s = &d.by_steps[step];
        // **失败声明的最后一个 step 可能是空的**（tactic 全绿、内核终审才拒绝）——
        // `goal: null` 在协议里 = 证明已闭合 ⇒ 不许把这种情况显示成「已无目标」✗
        // （G-78 的同一原则）。回退到题面：这道题仍然没通过。
        if !legacy && matches!(d.status, DeclStatus::Failed) && s.goals.is_empty() {
            return root();
        }
        StateSelection {
            goals: s.goals.clone(),
            span: Some(s.span),
            step: step as i64,
            total: d.by_steps.len(),
        }
    };

    // **逃生门**（只给反向验证用）：保留改动前的"进入态"选择。
    if state_after_legacy() {
        let selected = match d
            .by_steps
            .iter()
            .position(|s| s.span.start.offset <= cursor && cursor < s.span.end.offset)
        {
            Some(i) => i as i64 - 1,
            None => d
                .by_steps
                .iter()
                .rposition(|s| s.span.end.offset <= cursor)
                .map(|i| i as i64)
                .unwrap_or(-1),
        };
        return match usize::try_from(selected)
            .ok()
            .filter(|i| *i < d.by_steps.len())
        {
            Some(step) => at(step),
            None => root(),
        };
    }

    // **进入第 i 条 step 的状态** = 它前面那条**兄弟**执行后的状态。
    //
    // ⚠ `by_steps` 对**带块的 tactic**（`cases` / `match` / `·`）是**后序**：
    // 块体的 step 先入表、外层 step 后入表 ⇒ 不能直接取 `i - 1`（那可能是
    // **块体里的最后一步**，状态已闭合）。判据：若 `i - 1` 的 span **被 i 的 span
    // 包含**，它就不是兄弟，跳过它，继续找真正的兄弟；都没有 ⇒ 根状态。
    let entry = |i: usize| -> StateSelection {
        let cur = &d.by_steps[i];
        let mut j = i;
        while j > 0 {
            let prev = &d.by_steps[j - 1];
            if cur.span.start.offset <= prev.span.start.offset
                && prev.span.end.offset <= cur.span.end.offset
            {
                j -= 1;
            } else {
                break;
            }
        }
        if j == 0 {
            root()
        } else {
            at(j - 1)
        }
    };

    // ① **光标恰在一条 tactic 的起点** ⇒ 答"进入"它的状态。
    //
    // 普通 tactic 的进入态 = **上一条**执行后的状态（同一分支里的兄弟）。
    // `·` 的步进是个例外：它记的是**聚焦目标**（引擎在跑块体之前推的那一格），
    // 而它前面那条可能是**上一个分支**的最后一步 ⇒ 必须用它自己的 goals。
    //
    // 为什么不能只靠下面那条"最内层包含"：`cursor == start` 时开区间不成立，
    // 而 `·` 的 span 覆盖整个块 ⇒ 会选中 `·` 的聚焦态；对块里第二条之后的
    // tactic 来说那已经过期（用户 2026-10-27 报的"`intro x hx` 状态不对"）。
    if let Some(i) = d
        .by_steps
        .iter()
        .position(|s| s.span.start.offset == cursor)
    {
        if is_bullet_step(d.by_steps[i].span, text) {
            return at(i);
        }
        return entry(i);
    }

    // ② 否则：**最后一条已经开始的** tactic 之后的那个状态。
    //
    // 源码顺序就是嵌套顺序（外层 `·` 的 `start` 更早、内层兄弟更晚）⇒ 取
    // `rposition(start < cursor)` 自动是**最内层**的那一条；`·` 的块入口态
    // 会被块里后面的兄弟盖住（2026-10-27 用户实测的 `intro x hx`）。
    //
    // 这同时覆盖两种旧行为：tactic **内部**（`start < cursor`）答它自己的后态；
    // 两条 tactic **之间**（gap）答前一条的后态 —— 不是块入口的旧聚焦态。
    // ② 否则：**最后一条已经开始的** tactic 之后/内部 ⇒ 它自己的后态。
    //
    // 既有协议：tactic **起点**（①）答进入态，**内部**答它执行后的状态（老测试
    // `state_at_inside_a_tactic_shows_the_state_after_it` 钉的）；`cases` 这类
    // **后序**块的起点由 ① 的 `entry` 修正，内部仍按后态。
    if let Some(i) = d
        .by_steps
        .iter()
        .rposition(|s| s.span.start.offset < cursor)
    {
        return at(i);
    }

    // ③ 第一条 tactic 之前 ⇒ 根状态（题面）。
    root()
}

/// 这一步是不是 `·` 的步进（`ByStep.span` 从 `·` 开始）。
///
/// 只用于选择语义；`have` 的 span 从 `have` 开始 ⇒ 不会被误判 ✓。
fn is_bullet_step(span: Span, text: &str) -> bool {
    text.get(span.start.offset..)
        .is_some_and(|rest| rest.starts_with('·'))
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
