//! `front::query` 的 wire 类型：**编辑器无关**的内核真相查询结果。
//!
//! 这些类型是 `soko/*` 自定义请求与新的 `query` 子命令/MCP 工具**共用的唯一
//! 形状**（`docs/design/agent-query-channel.md` H6-A）。序列化字段名与既有 LSP
//! wire 完全一致——协议里是 snake_case（`goal_runs` / `ty_runs` / `sub_goals`），
//! 所以这里**不加** `rename_all`，改动字段名等于破坏协议（`docs/protocol.md`）。
//!
//! 不变量（AGENTS.md 硬规则 §2.4）：所有文本都由完整内核产出（pretty print）或
//! 由 `front::semantic` 唯一分类；本模块**不做判定**，只搬运与组装。

use serde::{Deserialize, Serialize};

/// 语义 run（着色片段）。`kind: None` = 纯连接符（空白/标点），不着色。
/// `kind` 的取值表见 [`crate::semantic::SemanticKind::as_str`]。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunInfo {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<String>,
}

/// 一条假设（binder）及其语义 runs。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinderInfo {
    pub name: String,
    pub ty: String,
    pub ty_runs: Vec<RunInfo>,
}

/// 一个 `sorry` 洞：范围 + **稳定 id**（`<declName>:<index>`，文档版本内有效）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoleInfo {
    pub start: usize,
    pub end: usize,
    pub id: String,
    /// 这个洞是**多余的** `sorry`（删掉它整条声明就过内核），而不是"还没证
    /// 出来"——`docs/design/redundant-sorry.md`。`false` 也可能只是没有判定
    /// 依据（保守）。
    #[serde(default)]
    pub redundant: bool,
}

/// 构造子 spine 子洞的期望类型（服务端走查；`ty: None` = 走查无法确定）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubGoalInfo {
    pub start: usize,
    pub end: usize,
    pub ty: Option<String>,
}

/// 一条内核判定的代码动作建议（`docs/design/hints-suggestions.md`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeActionInfo {
    pub title: String,
    pub kind: String,
    /// 首选建议（每个请求至多一条）。
    #[serde(default)]
    pub is_preferred: bool,
}

/// 一个声明的完整状态（`soko/goals` 的单个条目 + `query goals` 的元素）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclInfo {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub start: usize,
    pub end: usize,
    /// 内核渲染的声明类型（签名）；未知时为 `None`。
    pub ty: Option<String>,
    pub ty_runs: Vec<RunInfo>,
    /// **声明的值**（`:=` 之后那个东西，T-D52 / 用户第 8 条反馈）：只有
    /// `def`/`opaque` 有。类型看不出"本质"时，这一行就是答案——
    /// `Set.mem` 的类型是 `forall (α : Type 0), α -> Set α -> Prop`，
    /// 而它的值是 `fun (α : Type 0) (a : α) (A : Set α) => A a`。
    pub value: Option<String>,
    /// 值的语义分段（与 `ty_runs` 同一口径；客户端照着上色）。
    pub value_runs: Vec<RunInfo>,
    pub goal: Option<String>,
    /// 最后一条已记录 tactic 之后的**全部**未闭合目标（当前目标在前）；
    /// 非 `by` 的开放练习是走查得到的那一个；非开放声明为空。
    pub goals: Vec<String>,
    pub binders: Vec<BinderInfo>,
    pub hole: Option<(usize, usize)>,
    pub holes: Vec<HoleInfo>,
    pub sub_goals: Vec<SubGoalInfo>,
    pub code_actions: Vec<CodeActionInfo>,
}

/// 一个未闭合目标（`query state` / `soko/stateAt` 的 `goals[]` 元素）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalInfo {
    pub goal: String,
    pub goal_runs: Vec<RunInfo>,
    pub binders: Vec<BinderInfo>,
}

/// 声明级摘要（`state`/`holes` 里回溯到"我在哪个声明里"用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclHeader {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub start: usize,
    pub end: usize,
}

/// `state` 的答案：光标处（Lean `goalsAt?` 语义）的目标状态。
///
/// 单值字段（`goal`/`goal_runs`/`binders`）恒等于 `goals[0]` 的对应项，为老客户端
/// 保留；`goals` 是完整列表（当前目标在前，`[]` = 已闭合）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateAnswer {
    pub version: u64,
    pub decl: Option<DeclHeader>,
    pub goal: Option<String>,
    pub goal_runs: Vec<RunInfo>,
    pub binders: Vec<BinderInfo>,
    pub goals: Vec<GoalInfo>,
    /// 该 tactic 的范围（根状态 = 声明范围）。
    pub span: Option<(usize, usize)>,
    /// 选中的 per-tactic 状态序号；`-1` = 根状态。
    pub step: i64,
    pub total: usize,
}

/// 一个可寻址的洞（`query holes`；`id` 是**唯一稳定引用**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocatedHole {
    pub id: String,
    pub start: usize,
    pub end: usize,
    /// 期望类型（走查或请求期内核探针补出）；`None` = 无法确定。
    pub ty: Option<String>,
    /// 所属声明名（匿名 `example` 用 `example@<line>` 形式）。
    pub decl: String,
    /// 见 [`HoleInfo::redundant`]：这是"多写的一行"而不是"还没证出来"。
    #[serde(default)]
    pub redundant: bool,
}

/// 一次判卷的摘要（`query check`；事件流的**视图**，不是替代品）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSummary {
    pub version: u64,
    /// 各事件类型的计数（与 `--json` 事件流逐项一致，契约测试钉死）。
    pub counts: CheckCounts,
    /// 内核拒绝的声明（`failed > 0` 时 `query check` 退出码为 1）。
    pub failed: Vec<FailedDecl>,
    /// 非致命告警（code + 文本 + 位置）。
    pub warnings: Vec<WarningInfo>,
}

/// 事件计数（键名与 `--json` 的 `type` 字段一致）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckCounts {
    pub decl_checked: usize,
    pub example_checked: usize,
    pub exercise_open: usize,
    pub expr_typed: usize,
    pub expr_reduced: usize,
    pub decl_printed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedDecl {
    pub name: Option<String>,
    pub code: String,
    pub message: String,
    /// 诊断起点的**字节** offset（坐标空间 = 入口文件；字段名与语义是协议，
    /// 只加不删，见 `docs/protocol.md` 的 `check` 行）。
    pub start: usize,
    /// 诊断终点的**字节** offset（同上）。
    pub end: usize,
    /// 起点 1 基行号 / 列号（与 `--json` 事件 `span.start` 同一批数字）。
    pub start_line: u32,
    pub start_col: u32,
    /// 终点 1 基行号 / 列号（与 `--json` 事件 `span.end` 同一批数字）。
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarningInfo {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
    /// 见 [`FailedDecl::start`]：字节 offset，坐标空间 = 入口文件。
    pub start: usize,
    /// 见 [`FailedDecl::end`]。
    pub end: usize,
    /// 见 [`FailedDecl::start_line`]。
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// 求值答案（`query reduce`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReduceAnswer {
    pub value: String,
    pub ty: Option<String>,
}

/// 查询失败的**机器可判**原因（不是 panic，也不是空结果）。
///
/// 与"正常的没有"严格区分：`state.goal == None` 表示证明已闭合（正常），
/// 而 `QueryError::OutsideDeclarations` 表示问不出来。旧 `soko/stateAt` 用
/// `goal: null` + 默认字段混合表达两者，agent 无法区分——这正是它只能整文件
/// 扫事件流的根源（设计文档 §4.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueryError {
    /// 源文本无法解析到可回答的程度。
    NotParsable,
    /// 位置不在任何声明内。
    OutsideDeclarations,
    /// 位置越界（超出源文本）。
    PositionOutOfRange,
}

impl QueryError {
    /// 稳定的机器码（协议里出现的就是它）。
    pub fn code(self) -> &'static str {
        match self {
            QueryError::NotParsable => "not-parsable",
            QueryError::OutsideDeclarations => "outside-declarations",
            QueryError::PositionOutOfRange => "position-out-of-range",
        }
    }

    /// 给人/模型看的一句话（进 `error.message`）。
    pub fn message(self) -> &'static str {
        match self {
            QueryError::NotParsable => "源文本无法解析，先修 parse 诊断再查询",
            QueryError::OutsideDeclarations => "该位置不在任何声明内部",
            QueryError::PositionOutOfRange => "位置超出源文本范围",
        }
    }
}

/// 查询答案的外壳：`version` 供消费者丢弃过期答案（沿用 `soko/stateAt` 语义）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer<T> {
    pub version: u64,
    pub data: T,
}

// ── 项目视图（`query project` / `soko/project`，0.58.0 批次 4）─────────────────
//
// 形状与不变量见 `docs/design/project-view.md`：只读派生（不重跑内核）、单文件是
// **另一种合法状态**（`project: null` + `reason`）而不是错误、路径一律绝对路径。

/// 闭包里的一个模块（拓扑序；入口在最后）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectModule {
    pub name: String,
    /// 模块文件绝对路径。
    pub path: String,
    /// `compiled` / `load-failed` / `blocked`（`ModuleStatus::code()`）。
    pub status: String,
    /// 是不是这次编译的入口。
    pub entry: bool,
    /// 它 `import` 的模块名（书写顺序，去重）。
    pub imports: Vec<String>,
    /// 报告里的声明数（含开放练习）。
    pub decls: usize,
    pub errors: usize,
    pub warnings: usize,
    /// 未闭合的 `sorry` 练习数。
    pub open_exercises: usize,
    /// 状态的一句话解释：`load-failed`/`blocked` 时取该模块的第一条项目诊断，
    /// 其余为 `None`。
    pub message: Option<String>,
}

/// 一条项目级诊断（归属到导入方模块）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDiagnosticInfo {
    pub code: String,
    pub message: String,
    /// 归属模块名。
    pub module: String,
    /// `error` 或 `warning`：消费者要能自己数错/警，不必维护一份 code 清单
    /// （项目层 warning 是真实存在的，例如依赖里的开放练习）。
    pub severity: String,
    pub start: usize,
    pub end: usize,
}

/// 闭包计数（视图的地图/徽章用；比让消费者自己数 `modules` 更省事）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCounts {
    pub modules: usize,
    pub compiled: usize,
    pub failed: usize,
    pub blocked: usize,
    pub decls: usize,
    pub errors: usize,
    pub warnings: usize,
    pub open_exercises: usize,
}

/// 项目状态视图：**根、清单来源、闭包模块表、每模块状态、项目级诊断**。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectView {
    /// 入口模块名（点分）。
    pub entry: String,
    /// 模块根（绝对路径）。
    pub root: String,
    /// 生效的清单路径；`None` = 零配置（根 = 入口文件目录）。
    pub manifest: Option<String>,
    /// 清单 `requires` 与当前版本不一致时的提示（不阻断）。
    pub requires_warning: Option<String>,
    /// 拓扑序，入口在最后。
    pub modules: Vec<ProjectModule>,
    pub diagnostics: Vec<ProjectDiagnosticInfo>,
    pub counts: ProjectCounts,
}
