//! `soko/*` 自定义请求的 wire 类型（`docs/protocol.md`）。
//!
//! 这里是 wire 形状的**唯一定义处**：字段名与语义就是协议本身，改字段名等于
//! 破坏协议。`query_map` 只是把真相层的答案映射到这些类型上。

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, Range, TextDocumentIdentifier};

// ---- soko/* 自定义请求的 wire 类型（docs/protocol.md）----

#[derive(Debug, Deserialize)]
pub(crate) struct GoalsParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    #[allow(dead_code)]
    #[serde(default)]
    pub(crate) position: Option<Position>,
    /// **可选**：要不要逐 token 的着色数据（设计 `docs/design/goals-payload-slimming.md` §3
    /// 候选 B ✓）。**缺省 = `true` = 今天的行为** ⇒ 老客户端一字不改 ✓；`false` ⇒ 各 `*_runs`
    /// 空数组 ✓（实测真 unit08 上 decls 载荷 **−72%**：89 515B → 24 877B ✓）。
    #[serde(default = "default_true")]
    pub(crate) runs: bool,
}

/// `runs` 的缺省值（**必须 true** ✓ —— 缺省即"今天的行为" ✓）。
fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub(crate) struct RunInfo {
    pub(crate) text: String,
    /// [`sokonanoda_front::semantic::SemanticKind::as_str`]; absent = plain
    /// connector (whitespace/punctuation), drawn unstyled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    /// **源码字节区间**（G-53 ✓）：`Some` = 这段 run 对应源文本的哪一段 ✓ ⇒ webview 的
    /// `tok-*` span 可以变成可点 ✓（发 `definition` ✓）；`None` = 不知道 ✓ ⇒ **不可点** ✓。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) end: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoalBinderInfo {
    pub(crate) name: String,
    pub(crate) ty: String,
    /// Semantic runs of `ty` (`docs/protocol.md` §`soko/stateAt`): the same
    /// classification the editor's semantic tokens use, so hover and the
    /// Infoview can never drift.
    pub(crate) ty_runs: Vec<RunInfo>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HoleInfo {
    pub(crate) range: Range,
    /// Stable per (declaration, hole order) within a document version
    /// (`<declName>:<index>`, docs/protocol.md); anonymous examples use the
    /// `example@<line>` name form.
    pub(crate) id: String,
    /// This hole is a **redundant** `sorry` (deleting that line makes the whole
    /// declaration check), not an unsolved goal
    /// (`docs/design/redundant-sorry.md`).
    pub(crate) redundant: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoalDeclInfo {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) range: Range,
    /// The declaration's kernel-rendered type (signature), when known — used by
    /// the Infoview declaration list as a small hint (`docs/protocol.md`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ty: Option<String>,
    /// Semantic runs of `ty` (same single source as `goal_runs`, §2.1).
    pub(crate) ty_runs: Vec<RunInfo>,
    /// **声明的值**（T-D52 / R-1）：`def`/`opaque` 的 `:=` 之后的真正定义
    /// （内核 pp + 线 C 折叠，与 `ty` 同一形状 ✓）。`theorem`/`axiom` 没有。
    ///
    /// 为什么必须转发它 ✗：Infoview 的 `def` 卡片要显示第二行 `:= <值>`
    /// （扩展侧 `media/infoview.js` 早就按 `decl.value_runs` 渲染 ✓），
    /// 而这里以前**漏映射** ⇒ 扩展恒拿 `undefined` ⇒ 那一行永远不出现 ✓✗。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) value: Option<String>,
    /// `value` 的语义分段（与 `ty_runs` 走**同一个** `run_info` 实现 ✓
    /// —— 两条路共用一份实现，就不会再分叉 ✓）。
    pub(crate) value_runs: Vec<RunInfo>,
    pub(crate) goal: Option<String>,
    /// **`goal` 的语义分段**（T-A5 / R-2）：声明卡片照它给目标行上色。
    ///
    /// 为什么必须转发它 ✗：真相层算好了 runs、卡片也照 `tok-*` 渲染，但这里
    /// 以前**没有这个字段** ⇒ 扩展恒拿 `undefined` ⇒ 目标行只能画纯文本
    /// （与 R-1 的 `value_runs` 同形的断链；守卫 `scripts/audit-wire-fields.py`）。
    pub(crate) goal_runs: Vec<RunInfo>,
    /// Every open goal after the last recorded tactic (current goal first),
    /// or the single walked remaining goal for non-`by` open exercises.
    /// Empty for non-open declarations. Powers the multi-goal exercise panel.
    pub(crate) goals: Vec<String>,
    /// **`goals` 的语义分段**，与 `goals` **按位置对齐**（`goals_runs[i]` 是
    /// `goals[i]` 的 runs；长度恒相等）。`goals` 保持字符串数组是协议要求
    /// （练习树还在按字符串读它），runs 是给声明卡片上色的那一半。
    pub(crate) goals_runs: Vec<Vec<RunInfo>>,
    pub(crate) binders: Vec<GoalBinderInfo>,
    pub(crate) hole: Option<Range>,
    /// Every `sorry` in the answer (main hole + constructor-spine sub-holes),
    /// as `{range, id}` objects (`id` = `<declName>:<index>`).
    pub(crate) holes: Vec<HoleInfo>,
    /// Expected types for the sub-holes, positionally aligned with `holes`
    /// subset that came from a constructor spine (server-side walk).
    pub(crate) sub_goals: Vec<SubGoalInfo>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SubGoalInfo {
    pub(crate) range: Range,
    pub(crate) ty: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoalsResponse {
    pub(crate) decls: Vec<GoalDeclInfo>,
    /// **回显请求指向的文档 URI 与版本**（`docs/protocol.md` §soko/goals）：
    /// 多文档下客户端据此丢弃"答的是另一份文档"的过期响应，不必只靠本地守卫
    /// （服务端已按请求 URI 聚焦，这是给客户端自证用的）。
    pub(crate) uri: String,
    pub(crate) version: i32,
}

/// `soko/goalAt` 请求：**光标处那一条**声明的 goal 视图 —— Lean `$/lean/plainGoal`
/// 的声明级对应物（设计 `docs/design/persistent-declarations.md` §7.9 ✓）。
///
/// 为什么不复用 `soko/goals` ✗：那一条答的是**整份入口的声明列表**（实测真 unit08
/// 上 27 条 ⇒ 91 361B ✗），而"光标处的 goal"只要**一条**（≈1–3KB ✓）。
/// **老形状与老消费者一字不动** ✓（`soko/goals` 的响应逐字节不变 ✓）。
#[derive(Debug, Deserialize)]
pub(crate) struct GoalAtParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    pub(crate) position: Position,
}

/// `soko/goalAt` 响应：光标处那条声明 + 请求身份回显。
///
/// `decl` 与 `soko/goals` 的 `decls[i]` **同一形状** ✓ ⇒ 消费方的卡片渲染**不降级** ✓
/// ——返回的**每一份都带自己的** `ty_runs`/`value_runs`/`goal_runs`/`goals_runs` ✓
/// （"按目标按需返回"，不是"摘掉 runs" ✗：摘掉 = 用户可见渲染静默降级 ✗）。
#[derive(Debug, Serialize)]
pub(crate) struct GoalAtResponse {
    /// 回显请求指向的文档 URI 与版本（见 [`GoalsResponse::uri`]）。
    pub(crate) uri: String,
    pub(crate) version: i32,
    /// `None` = 光标不在任何声明里（空白行/注释/文件首尾）——与 Lean `plainGoal`
    /// 在该处答 `null` **同形** ✓，**不是**错误。
    pub(crate) decl: Option<GoalDeclInfo>,
}

/// `soko/project` 请求（0.58.0 批次 4）：这个文档所在闭包的只读状态视图。
#[derive(Debug, Deserialize)]
pub(crate) struct ProjectParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
}

/// `soko/project` 响应：项目视图 + 请求身份回显。
///
/// `project: None` **不是**错误：单文件（无 `import`）/ 定位不到入口都是合法
/// 状态，机器码见 `reason`（`docs/design/project-view.md` §5）。
#[derive(Debug, Serialize)]
pub(crate) struct ProjectResponse {
    /// 回显请求指向的文档 URI 与版本（见 [`GoalsResponse::uri`]）。
    pub(crate) uri: String,
    pub(crate) version: i32,
    pub(crate) project: Option<sokonanoda_front::query::ProjectView>,
    /// `no-imports` / `no-path` / `parse-error`；有项目时为 `None`。
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NextHoleParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    pub(crate) position: Position,
    #[serde(default)]
    pub(crate) forward: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StateAtParams {
    #[serde(rename = "textDocument")]
    #[allow(dead_code)]
    pub(crate) text_document: TextDocumentIdentifier,
    pub(crate) position: Position,
}

#[derive(Debug, Serialize)]
pub(crate) struct StateDeclInfo {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) range: Range,
}

/// One open goal inside `soko/stateAt`'s `goals` array.
#[derive(Debug, Serialize)]
pub(crate) struct StateGoalInfo {
    pub(crate) goal: String,
    /// Semantic runs of `goal` (`docs/protocol.md` §`soko/stateAt`).
    pub(crate) goal_runs: Vec<RunInfo>,
    pub(crate) binders: Vec<GoalBinderInfo>,
}

/// 光标所在**行**的一条命令输出（`#check` / `#print`）—— **C3/2026-10-08**。
///
/// 与 Lean 的 `getInteractiveDiagnostics{lineRange?}` 同口径（**按行取**）：
/// 选择语义在**真相层**（`front::query` 的 `messages_at`），这里只是 wire 形状 ✓。
#[derive(Debug, Serialize)]
pub(crate) struct StateMessageInfo {
    /// `"check"` | `"print"`。
    pub(crate) kind: String,
    /// `#check` ⇒ `表达式 : 类型`；`#print` ⇒ 打印出来的定义文本。
    pub(crate) text: String,
    /// 那条输出的范围（`#check` = 被检查的表达式；`#print` = 那条命令）。
    pub(crate) range: Range,
}

/// `soko/stateAt` response (docs/protocol.md): the goal state at the cursor,
/// plus enough declaration info for the client to label and reveal it.
#[derive(Debug, Serialize)]
pub(crate) struct StateAtResponse {
    pub(crate) version: i32,
    /// 回显请求指向的文档 URI（见 `GoalsResponse::uri`）。
    pub(crate) uri: String,
    pub(crate) decl: Option<StateDeclInfo>,
    /// `None` = no remaining goals (the proof is closed at this position).
    /// Single-value, equal to `goals[0].goal`; kept for older clients.
    pub(crate) goal: Option<String>,
    /// Semantic runs of `goal` (= `goals[0].goal_runs`); kept for older clients.
    pub(crate) goal_runs: Vec<RunInfo>,
    /// Hypotheses of `goal` (single-value, equal to `goals[0].binders`).
    pub(crate) binders: Vec<GoalBinderInfo>,
    /// Every remaining goal, current goal first (`[]` = closed). Clients that
    /// render goal lists should prefer this over the single `goal`.
    pub(crate) goals: Vec<StateGoalInfo>,
    /// The tactic's range (root state: the declaration's range).
    pub(crate) span: Option<Range>,
    /// Index of the selected per-tactic state; `-1` = root.
    pub(crate) step: i64,
    pub(crate) total: usize,
    /// **光标所在行的命令输出**（`#check`/`#print`；`[]` = 这一行没有）—— C3。
    pub(crate) messages: Vec<StateMessageInfo>,
}

impl StateAtResponse {
    pub(crate) fn empty(uri: &str, version: i32) -> Self {
        Self {
            uri: uri.to_string(),
            version,
            decl: None,
            goal: None,
            goal_runs: Vec::new(),
            binders: Vec::new(),
            goals: Vec::new(),
            span: None,
            step: -1,
            total: 0,
            messages: Vec::new(),
        }
    }
}
