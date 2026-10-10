//! 文档级报告：每个声明的练习状态与 hover 类型。

use super::error::CompileError;
use super::warning::CompileWarning;
use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclKind {
    Definition,
    Theorem,
    Axiom,
    Inductive,
    Example,
}

impl DeclKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeclKind::Definition => "def",
            DeclKind::Theorem => "theorem",
            DeclKind::Axiom => "axiom",
            DeclKind::Inductive => "inductive",
            DeclKind::Example => "example",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclStatus {
    /// The declaration is an open exercise (`sorry` in the value position).
    Open,
    /// The declaration passed the complete kernel.
    Checked,
    /// Elaboration or the kernel rejected it.
    Failed,
}

/// One hypothesis already introduced in a partial answer: its written name
/// and the type it carries, rendered as source text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalBinder {
    pub name: String,
    pub ty: String,
}

/// One `sorry` inside a constructor spine (multi-hole answer): its span and the
/// expected type the walk recovered for it (best-effort, `None` when the
/// constructor's field type could not be instantiated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubGoal {
    pub span: Span,
    pub ty: Option<String>,
}

/// One open goal after a tactic step: its type and the hypotheses in scope
/// for it (a different sub-goal may carry a different context).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByGoalState {
    pub ty: String,
    pub binders: Vec<GoalBinder>,
}

/// One tactic step of a `by` block, recorded by the engine as the state
/// **after** that tactic executed (`docs/design/by-tactics.md` §6): the
/// tactic's source span and **every** remaining goal (the current goal first;
/// empty when every goal is closed). In the I8 session snapshot this is the
/// editor's "goals at cursor" data (`soko/stateAt`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByStepState {
    pub span: Span,
    pub goals: Vec<ByGoalState>,
}

/// **报告形状版本** —— 缓存/快照里那些**会变的形状**的统一版本号。
///
/// **任何时候改动下面这些结构的序列化形状或语义，都必须 +1**：
/// `DocumentReport` · `DeclState` · `ByGoalState` · `ByStepState` · `GoalBinder` ·
/// `SubGoal` · `ProjectReport`（`crates/front/src/project/report.rs`）。
/// 新增 / 删除 / 改名一个字段、或让某个字段的含义变化 —— **都算**。
///
/// 为什么必须有它（**2026-10-02 值守第 8 单**，用户实测报的 ✗）：缓存键里只有
/// **源码 digest** + 编译期常量（`CACHE_FORMAT` / `CARGO_PKG_VERSION` /
/// `build_stamp` = debug|OS|ARCH）。**源码没变而二进制变了**（正是"修好了但用户看不到"
/// 的形态 ✗）⇒ 键不变 ⇒ 旧条目**命中** ⇒ 而新字段走 `#[serde(default)]` ⇒ 静默给旧答案 ✗
/// （实测：G-78 的 `DeclState.by_root` 被整库陈旧缓存挡掉 —— 250+ 条里 **0 条**含
/// `by_root`；用户删掉那一条缓存后同一条命令立刻变对 ✓）。
///
/// ⇒ 这个常量**进缓存键、进条目本身、进 `meta.json` 的 schema**：形状一变，
/// **整库不命中** ✓。**不许再用「字段可选（`#[serde(default)]`）」来兜兼容** ✗ ——
/// 那正是掩盖机制：老缓存不报错、不 miss，只给旧答案。
/// **B1（2026-10-08）把它 1 → 2**：`DeclState.by_steps` 的**含义变了** ——
/// 失败（tactic 报错 / 内核终审不过）的声明现在**也带**逐 tactic 状态
/// （以前恒为空 ⇒ `query state` 退回题面 `step:-1/total:0` ✗）。
/// 语义变化**同样算形状变化**（下面那段话的最后一句）⇒ 必须 bump，
/// 否则旧缓存条目会静默给出"没有步进"的旧答案 ✗（G-78 踩过一次）。
/// **C3（2026-10-08）把它 2 → 3**：`DocumentReport` 多了 `prints`
/// （`#print` 的结果 —— 以前那些事件**根本没进报告**）⇒ 形状变了，
/// 旧条目必须整库不命中，否则 Infoview 会拿到"没有 prints"的旧答案 ✗。
/// **C3 补口（2026-10-09）把它 3 → 4**：`prints` 的**含义**变了 —— **项目模式**
/// （入口带 `import`）里以前恒空（`run_pass_with` 只映射 `TypeChecked`；
/// `splice_entry_report` 又漏拼 `fresh.prints`）⇒ 旧条目那份"空 prints"会静默
/// 给出"课程文件里 `#print` 没反应"的旧答案 ✗。与上一条同一条纪律：
/// **语义变化同样算形状变化**（G-78 踩过一次 ✓）。
/// **重复输出（2026-10-10 用户实测）把它 4 → 5**：`CheckInfo::cmd` /
/// `PrintInfo::cmd` 进序列化（以前 `#[serde(skip)]` ⇒ 项目产物回放后 cmd 归零
/// ⇒ 拼接时同一条命令算两遍 ⇒ Infoview 的「命令输出」每编辑一次 +1 ✗）。
/// 旧条目**不含 cmd** ⇒ 必须整库不命中（否则它反序列化成 0，重复照旧 ✗）。
/// **G-108（2026-10-10 用户实测）把它 5 → 6**：`hovers` 的**含义**变了 —— 报错 /
/// `sorry` 的那段源码现在也有行（**词法行**：`expr: None` + `binder: true`，见
/// `walk.rs::push_lexical_hover_rows`），F12/悬停因此才认得那些名字。
/// 老条目里那段源码**没有行** ⇒ 回放它 = 把"跳不了"的旧行为放回来 ✗
/// ⇒ 整库不命中，重编一遍 ✓（与上面两条同一条纪律：语义变化算形状变化）。
pub const REPORT_SHAPE: u32 = 6;

/// One declaration of a `.sokonanoda` document, with its exercise status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclState {
    pub kind: DeclKind,
    pub name: Option<String>,
    pub span: Span,
    pub status: DeclStatus,
    /// Present when `status == Failed`.
    pub error: Option<CompileError>,
    /// For an open exercise: the remaining goal type, rendered as source text.
    pub goal: Option<String>,
    /// **根状态**（第一条 tactic 之前）—— 声明的 ∀ 绑元 + 剥掉它们之后的命题，
    /// 已过记法折叠。`None` = 这条声明没有 `by` 块（那时 `goal`/`binders` 就是
    /// 声明级的剩余目标/上下文）。
    ///
    /// ⚠ **为什么必须单独记**（2026-10-02 用户实测报的 bug）：`goal`/`binders`
    /// 是**洞处**的状态（走查引入的假设已进去），不是"第一条 tactic 之前"的
    /// 状态 ✗ —— `soko/stateAt` 的根状态要的是后者（Lean `goalsAt?` 语义：
    /// 定理的 ∀ 绑元在证明开始时就在上下文里）。把两者混同会让 Infoview 顶部
    /// 显示整句量词式、并把绑元标成 `unknown_ident` ✗。
    ///
    /// ⚠ **没有 `#[serde(default)]`，故意的** ✗（2026-10-02 值守第 8 单）：老缓存
    /// （写这份报告时还没有这个字段）反序列化会**直接失败** ⇒ 当 miss 重编 ✓。
    /// 带 `#[serde(default)]` 的话它会安静地读成 `None` ⇒ 根状态静默退回旧行为 ✗
    /// —— 那正是"修好了但用户看不到"的通道。形状真的变了 ⇒ 走 `REPORT_SHAPE` ✓。
    pub by_root: Option<ByGoalState>,
    /// For an open exercise: the hypotheses already introduced by the lambda
    /// binders written so far (the goal view's "context"). Empty when the
    /// answer hole has no lambda prefix yet.
    pub binders: Vec<GoalBinder>,
    /// Index of the command that produced this state (`file.commands[cmd]`),
    /// so incremental sessions and LSP tooling can map states back to source
    /// commands without span guessing.
    pub cmd: usize,
    /// The declaration's universe parameters (`{u}` …). Open exercises carry
    /// them into the goal view / tactic judging so `Sort u` goals can be
    /// judged (the judge synthesizes a declaration with the same universes).
    pub universe: Vec<String>,
    /// Open exercises: every `sorry` span in the answer (the main hole and any
    /// constructor-spine sub-holes), ordered by offset. Powers `soko/nextHole`
    /// and the goal panel; the server derives them from the walk — clients
    /// never scan text.
    pub holes: Vec<Span>,
    /// Open exercises: expected types for constructor-spine sub-holes,
    /// positionally aligned with `sub_goals` below.
    pub sub_goals: Vec<SubGoal>,
    /// Kernel-rendered declared type（声明签名文本，如
    /// `axiom And.intro : forall (a : Prop), …`）。声明名 hover 与练习
    /// 面板显示用；Open 练习来自 elaborated type。
    pub ty_text: Option<String>,
    /// **声明的值**（`:=` 之后那个东西，T-D52 / 用户第 8 条反馈）。
    ///
    /// 只有 `def`/`opaque` 有；`theorem`/`axiom`/`inductive` 是 `None`
    /// （用户要的是 `def` 的"本质"——证明是另一件事）。
    /// 与 `ty_text` 同一形状：内核 pp 出来再过一遍**线 C 的记法折叠**。
    pub val_text: Option<String>,
    /// Open exercises: when the remaining goal's head is a constructor with a
    /// known template, a full-application skeleton with auto-filled parameters
    /// and `sorry` for the proof fields (e.g. `And.intro a b sorry sorry`).
    pub refine_template: Option<String>,
    /// The declaration's hint ladder, authored in the canvas as
    /// `-- soko:hint <text>` comment directives attached to this declaration
    /// (`compile::hints::attach_hints`). Empty when the source has no hints
    /// for it or when the report was produced without a source text
    /// (e.g. `compile_fol` on a pre-parsed AST).
    pub hints: Vec<String>,
    /// Per-tactic states of a `:= by …` value, in tactic order (each state is
    /// recorded **after** its tactic ran). Empty for declarations without a
    /// `by` block. Powers `soko/stateAt` (cursor goal view).
    pub by_steps: Vec<ByStepState>,
}

/// Where a name use resolves to, together with the definition's source span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedTarget {
    /// A local binder; the span is the binder's own source span.
    Binder(Span),
    /// A top-level declaration of the same file; the span is the defining
    /// command's span.
    Declaration { name: String, span: Span },
    /// **记法符号**（T-D15）：`span` 是**使用处那个符号自己**的 span
    /// （T-D14 的 `Expr::Notation.symbol_span`），不是整段节点。
    ///
    /// 为什么需要它：以前记法行的 `resolution` 是 `None`，于是"光标在符号上"
    /// 落到两个回退里，会误命中**外层 binder**（T-D30 的守卫是权宜之计）。
    /// 有了这个变体，符号就是**一等目标**。
    ///
    /// `module` 是**声明记法的模块名**（跨模块跳转用）；本文件内声明时为 `None`
    /// ——elaborator 拿不到记法表（它在 parser 手里），所以这里先留 `None`，
    /// **声明点**由 LSP 侧的闭包记法表解析（`QueryDoc::notation_at`，T-D10/T-D16）。
    Notation {
        symbol: String,
        span: Span,
        module: Option<String>,
    },
}

impl ResolvedTarget {
    /// The definition's source span (go-to-definition / highlight target).
    pub fn span(&self) -> Span {
        match self {
            ResolvedTarget::Binder(span) => *span,
            ResolvedTarget::Declaration { span, .. } => *span,
            ResolvedTarget::Notation { span, .. } => *span,
        }
    }
}

/// A hover answer for one source span (`span -> inferred type text`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverType {
    pub span: Span,
    pub text: String,
    /// In-scope binder names at this sub-expression, outermost first, for
    /// scoped completion. Anonymous binders (`A -> B`) carry an empty name.
    pub scope_names: Vec<String>,
    /// When this span is a name use point: where the name is defined
    /// (`None` for prelude names and unresolved idents).
    pub resolution: Option<ResolvedTarget>,
    /// This row is a lambda/forall **binder declaration** (`name : ty`): the
    /// editor renders the declaration itself, not `expr : type`.
    pub binder: bool,
}

/// A `#check` command's result: the checked expression's span and the
/// kernel-pretty-printed type. Carried on the report so the editor can show
/// it persistently (inlay hint), like Lean's Infoview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInfo {
    pub span: Span,
    pub text: String,
    /// 产生它的命令下标（`file.commands[cmd]`）。跨文件编译时按它归因；
    /// **项目模式**下 [`crate::query::QueryDoc`] 的拼接（`splice_entry_report`）
    /// 也按它去重。
    ///
    /// ⚠ **必须进序列化**（2026-10-10 用户实测的重复输出根因）：以前这里是
    /// `#[serde(skip)]`，理由是"缓存条目只服务单文件" —— 但**项目产物**
    /// （`<模块根>/.sokonanoda/compiled/*.json`）回放的是**整份报告**，反序列化
    /// 后所有 `cmd` 归零 ⇒ 拼接时"缓存那份"与"新查那份"的键对不上 ⇒
    /// **同一条命令的输出出现两份**，而且产物每回放一次就再多留一份 ✗
    /// （用户现场：「每编辑一下，`#check`/`#print` 就多重复一次」，3 → 4 条）。
    /// ⇒ 语义变化同样算形状变化 ⇒ `REPORT_SHAPE` 4 → 5（旧条目整库不命中 ✓）。
    pub cmd: usize,
}

/// A `#print` command's result: the **printed declaration text** + the
/// **command's** span (C3/2026-10-08).
///
/// **为什么 span 挂"命令"而不是"名字"**：`#print` 的 `Printed` 事件
/// （`compile/check/kernel_phase.rs`）**不带 span**（它只有 `name`/`text`）⇒
/// 真相层按**命令下标**回填命令自己的 span ✓（`assemble_report` 的 `spans[j]`）。
/// 与 Lean 的 `withRef tk` 取的是同一个东西（`#print` 那条命令的位置），
/// 而 Infoview 的用法正是"**光标落在这条命令上**就显示它的输出" ✓。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintInfo {
    pub span: Span,
    pub name: String,
    pub text: String,
    /// 产生它的命令下标（与 [`CheckInfo::cmd`] **同一条纪律**：必须进序列化
    /// —— 项目产物回放后 cmd 归零正是"重复输出"的根因，见那边注释）。
    pub cmd: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentReport {
    pub decls: Vec<DeclState>,
    pub hovers: Vec<HoverType>,
    /// Parallel to `hovers`: the command index each hover belongs to, so an
    /// incremental session can cache/re-map hovers per command.
    pub hover_cmds: Vec<usize>,
    /// Parse-level diagnostics live on the `parse` result; this holds
    /// elab/kernel failures that are not attached to a declaration.
    pub errors: Vec<CompileError>,
    /// `#check` results, in source order.
    pub checks: Vec<CheckInfo>,
    /// **`#print` 结果**（C3/2026-10-08），按来源顺序 —— 以前这些事件**根本没进
    /// 报告**（`assemble_report` 只匹配 `TypeChecked`）⇒ Infoview 结构上看不见它 ✗。
    pub prints: Vec<PrintInfo>,
    /// Syntax-level warnings (e.g. a declaration colliding with a
    /// kernel-defined name).
    /// Independent of declaration status; the LSP renders these as
    /// `DiagnosticSeverity::WARNING`.
    pub warnings: Vec<CompileWarning>,
}

impl DocumentReport {
    /// **回放闸门**：一条命令的 `#check`/`#print` 结果只许留一份。
    ///
    /// 为什么要它（2026-10-10 用户实测的「每编辑一下多一份」闭环，台账 G-102）：
    /// 报告是**会被持久化再回放**的（模块根 `<root>/.sokonanoda/compiled/*.json`
    /// 与全局缓存，两者同一个条目格式）。只要某一次写出去的报告里同一条命令
    /// 带着两份结果，回放就把两份都装进内存；下一次编辑的拼接
    /// （`query::splice_entry_report`）再把缓存那份 + 新算那份一起交出来
    /// ⇒ **每编辑一次 +1**，一路累积到用户看到的 19 份 ✗。
    ///
    /// 上面两道闸（`cmd` 进序列化 + 拼接按 span 幂等）修的是**产出侧**；
    /// 这一道修的是**入口侧**：凡是从磁盘读回来的报告，进场前先把自己内部的
    /// 重复收干净 —— 与 `splice_entry_report` 的闸门同一条纪律
    /// （**同一个命令位置只留一份**），于是"旧产物/将来某个新生产者写出的重复"
    /// 都无法再靠回放复活 ✓。
    ///
    /// 判据：`cache::tests::a_replayed_report_is_sanitised_before_it_leaves_the_cache`
    /// （撤掉调用 ⇒ 判红 ✓）。
    ///
    /// **不变量**（去重键都取自"来源身份"，不是猜）：
    /// * `checks`/`prints` —— 键 = 自己的 span（命令位置；同一条命令的同一次输出
    ///   必然逐字节同一个 span，而不同命令的 span 不会相同）；
    /// * `decls` —— 键 = `(cmd, span)`（`cmd` 一条命令一个；inductive 块的一条命令
    ///   可以产多个构造子 ⇒ span 必须一起进键）。
    ///
    /// `hovers` **不动**：同一条命令里可以有多个 hover（不同的名字使用处），
    /// 它们不是重复 —— 那是"同一份报告"的合法内容 ✗ 不许拿它当重复删。
    pub fn drop_replayed_duplicates(&mut self) {
        dedup_keep_first(&mut self.checks, |c| {
            (c.span.start.offset, c.span.end.offset)
        });
        dedup_keep_first(&mut self.prints, |p| {
            (p.span.start.offset, p.span.end.offset)
        });
        dedup_keep_first(&mut self.decls, |d| {
            (d.cmd, d.span.start.offset, d.span.end.offset)
        });
    }
}

/// 按 `key` 去重、**留第一份**、保持原有顺序（`drop_replayed_duplicates` 用）。
///
/// 留第一份而不是最后一份：回放条目里那些重复是**同一个结论的副本**，
/// 谁留下都一样；而"留第一份"让这个函数对"输入里本来没有重复"是**恒等**的
/// （不会因为去重把顺序搅动 ⇒ 判据/`--json` 面零变化 ✓）。
fn dedup_keep_first<T, K: Ord>(items: &mut Vec<T>, key: impl Fn(&T) -> K) {
    if items.len() < 2 {
        return;
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut kept = Vec::with_capacity(items.len());
    for item in items.drain(..) {
        if seen.insert(key(&item)) {
            kept.push(item);
        }
    }
    *items = kept;
}
