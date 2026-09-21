//! 内核真相查询层：**编辑器无关**的类型化查询。
//!
//! 这是 `soko/*` 自定义请求、`sokonanoda query` 子命令与 MCP 工具**共用的唯一
//! 真相**（`docs/design/agent-query-channel.md` H6-A）。任何一个适配器都**不得**
//! 自己重算"光标处是哪个目标""下一个洞在哪"——那会产生第二份真相，违反
//! `REQUIREMENTS.md` §2 第 4 条（判定永远走内核，禁止近似实现）。
//!
//! 设计要点：
//! - 位置在真相层用**字节 offset**（唯一、无编码歧义）；行/列只在适配器层出现
//!   （LSP 用 0-based UTF-16，CLI/MCP 用 1-based，见 [`line_col_of`]）。
//! - "正常的没有"（证明已闭合、没有下一个洞）返回 `None`/空数组；"问不出来"
//!   返回 [`QueryError`]。两者绝不混用。
//! - 所有文本来自完整内核（pretty print）或 `crate::semantic` 的唯一分类。

mod pos;
mod project;
mod state;
mod types;

pub use pos::{line_col_of, offset_of_line_col};
pub use state::{select_state_at, StateSelection};
pub use types::{
    Answer, BinderInfo, CheckCounts, CheckSummary, CodeActionInfo, DeclHeader, DeclInfo,
    FailedDecl, GoalInfo, HoleInfo, LocatedHole, ProjectCounts, ProjectDiagnosticInfo,
    ProjectModule, ProjectView, QueryError, ReduceAnswer, RunInfo, StateAnswer, SubGoalInfo,
    WarningInfo,
};

use crate::compile::{
    compile_all_with, probe_sub_goal_types_with, ByGoalState, CompileOptions, DeclState,
    DeclStatus, DocumentReport, PreludeMode,
};
use crate::semantic::{self, SemanticKind};
use crate::session::Session;
use crate::span::Span;
use crate::Diagnostic;

/// 一份被查询的文档：文本 + 会话式编译状态 + 最近的报告。
///
/// 这是 LSP 的 `Doc` 与 CLI/MCP 共用的载体——查询逻辑全部挂在这里，适配器只做
/// 坐标转换与序列化（LSP 侧 `Doc` 现在是它的薄包装）。
pub struct QueryDoc {
    pub text: String,
    /// 会话式编译（I8）：持有上一版本快照，编辑只重查受影响后缀。
    pub session: Session,
    /// prelude 模式（`Full` / `Bare`，可由文件注释指令覆盖）。
    pub mode: PreludeMode,
    /// 最近一次编译的报告；`None` = 尚未编译过。
    pub report: Option<DocumentReport>,
    /// 最近一次 parse 诊断（报告为空时用它解释"为什么问不出来"）。
    pub parse_error: Option<Diagnostic>,
    /// 文档版本（随每次 `set_text` 递增），供消费者丢弃过期答案。
    pub version: u64,
    /// 入口文件路径（`--text`/stdin 为 `None`）：文本里有 `import` 时，
    /// 项目闭包编译需要它来定位模块根（设计 §4.9）。
    pub path: Option<std::path::PathBuf>,
    /// `--root` 显式模块根（跳过清单发现）。
    pub root: Option<std::path::PathBuf>,
    /// 文本里有 `import` 且能定位入口时的项目编译结果。
    project: Option<crate::project::ProjectReport>,
    /// 最近一次编译的**事件流**（单文件来自会话增量、项目来自入口模块）。
    ///
    /// `check()` 以前会为了一次计数再编译一遍（项目模式下等于每次查询重编译整个
    /// 闭包）——现在直接读这里；`query check` 冷跑因此省掉一次完整编译。
    output: crate::compile::CompileOutput,
    /// 最近一次编译用的内存覆盖（打开文档的路径 → 文本）。`check`/`reduce`
    /// 会重跑闭包编译，必须复用同一份覆盖，否则答案与 `report` 不同源。
    overlay: Vec<(std::path::PathBuf, String)>,
    /// [`Self::project_view_reason`] 的缓存（T-A24）。
    ///
    /// 那个问题**每次诊断事件都会被问一次**（VS Code 的 `soko/project`），
    /// 而它以前每次都重新 `parse` 整份文本。原因只由（`project` 是否存在、
    /// 文本能不能 parse、有没有 `import`）决定，而这三样在 `set_text` 之后就定了
    /// ⇒ 算一次存下来。`None` = 还没算过（`set_text` 之前）。
    project_reason: Option<&'static str>,
}

impl Default for QueryDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryDoc {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            session: Session::new(CompileOptions::default()),
            mode: PreludeMode::Full,
            report: None,
            parse_error: None,
            version: 0,
            path: None,
            root: None,
            project: None,
            output: crate::compile::CompileOutput::default(),
            overlay: Vec::new(),
            project_reason: None,
        }
    }

    /// 换文本并重编译（版本递增）。`mode` 为 `None` 时沿用当前模式。
    pub fn set_text(&mut self, text: &str, version: u64, mode: Option<PreludeMode>) {
        self.set_text_with_overlay(text, version, mode, &[]);
    }

    /// 同 [`Self::set_text`]，但项目闭包编译时把 `overlay`（打开文档的内存文本）
    /// 当成依赖的源文本——编辑器未保存的依赖编辑因此对这份文档可见（I16 P5）。
    pub fn set_text_with_overlay(
        &mut self,
        text: &str,
        version: u64,
        mode: Option<PreludeMode>,
        overlay: &[(std::path::PathBuf, String)],
    ) {
        self.overlay = overlay.to_vec();
        if let Some(mode) = mode {
            if mode != self.mode {
                self.mode = mode;
                self.session = Session::new(CompileOptions { prelude: mode });
            }
        }
        self.version = version;
        self.text = text.to_string();
        // 有 `import` 时环境来自整个闭包：单文件会话看不到被导入的声明，
        // 所以报告要用项目编译的入口报告（无 import 时零变化）。
        //
        // **先算闭包**：它成功时下面那次单文件全量编译会被整个覆盖 ⇒ 纯浪费
        // （T-A20）。实测：项目模式一次 `set_text` 原本要**编两遍**——先编一遍
        // 入口单文件、再编整个闭包，然后把前者的报告与事件全丢掉。
        self.project = self.project_compile(text);
        let project_entry_report = self
            .project
            .as_ref()
            .and_then(|p| p.entry_report())
            .cloned();
        match project_entry_report {
            Some(mut report) => {
                // 闭包成功：只补 `parse_error`（它是 `usable()` 的一半判据——
                // 「单独 parse 失败但闭包好」算可用），**不跑**单文件流水线。
                self.parse_error = crate::parse(text).err();
                // 项目编译走的是 `compile_all_units`，不经过 Session 的 hint 挂接：
                // 这里补上，否则带 `import` 的入口会丢掉 `-- soko:hint` 阶梯
                // （`soko/hints` 与 MCP `hints` 都会答空）。
                crate::compile::attach_hints_to_report(text, &mut report);
                self.report = Some(report);
                // 事件同样取入口模块的（闭包整体在 `project` 里，消费者要它
                // 就用 `project_view`/`entry_module`）。
                self.output = self
                    .project
                    .as_ref()
                    .and_then(|p| p.entry_module())
                    .map(|module| module.events.clone())
                    .unwrap_or_default();
            }
            None => {
                // 单文件，或闭包失败（入口 `LoadFailed`）：走原来的会话路径。
                let update = self.session.update(text, version);
                self.parse_error = update.parse_error;
                // 注意：这里手工装配 `CompileOutput`，绕过 `push_event`/`push_error`/
                // `push_warning` ⇒ `event_cmds`/`error_cmds`/`warning_cmds` 三个平行数组
                // 在**本文件内**是空的（与 `units.rs` 的 `debug_assert_eq!` 不变量字面
                // 冲突，WO-010 的 R4）。今天无害：`check()` 只读 `errors`/`warnings`。
                // 谁要在这里读 `*_cmds`，先改成走 `push_*`，别静默拿空数组当"没有归属"。
                self.output = crate::compile::CompileOutput {
                    events: update.events.clone(),
                    errors: update.report.errors.clone(),
                    warnings: update.report.warnings.clone(),
                    ..Default::default()
                };
                self.report = Some(update.report);
                if let Some(output) = self
                    .project
                    .as_ref()
                    .and_then(|p| p.entry_module())
                    .map(|module| module.events.clone())
                {
                    self.output = output;
                }
            }
        }
        // 缓存"为什么没有项目视图"（T-A24）：`soko/project` 每次诊断事件都会被
        // 问一次，而它以前每次重新 parse 整份文本。
        self.project_reason = Some(Self::compute_project_reason(&self.project, &self.text));
    }

    /// [`Self::project_view_reason`] 的本体（纯函数；`set_text` 之后调用一次）。
    fn compute_project_reason(
        project: &Option<crate::project::ProjectReport>,
        text: &str,
    ) -> &'static str {
        if project.is_some() {
            return "available";
        }
        match crate::parse(text) {
            // 解析不了 ⇒ 先修语法；这和"单文件"是两回事。
            Err(_) => "parse-error",
            Ok(file) if !file.commands.iter().any(|command| command.is_import()) => "no-imports",
            // 有 `import` 但没有入口路径（stdin / `--text` 且没有 `--root`）。
            Ok(_) => "no-path",
        }
    }

    /// 整份项目报告（`None` = 单文件）。缓存条目要存它（T-A03）：
    /// LSP 的跨文件能力读的是模块表，只存入口报告会让命中缓存的文档
    /// "能显示、不能跳转"。
    pub fn project_report_ref(&self) -> Option<&crate::project::ProjectReport> {
        self.project.as_ref()
    }

    /// 这份文档上次编译时用的**内存覆盖**是否与 `overlay` 一致。
    ///
    /// 供 LSP 的"文本没变就短路"判断（A7 / T-A21）：**覆盖变了也必须重编**——
    /// 依赖的未落盘编辑会改变这份文档的闭包结果，哪怕它自己的文本一个字节没动。
    pub fn overlay_matches(&self, overlay: &[(std::path::PathBuf, String)]) -> bool {
        self.overlay == overlay
    }

    /// 文本里有 `import` 且能定位入口（`--file` 或 `--root`）时，编译整个
    /// 闭包并返回项目报告；否则 `None`（单文件路径，行为与今天一致）。
    fn project_compile(&self, text: &str) -> Option<crate::project::ProjectReport> {
        // `is_project_source`：parse 失败时退回 `import` 代码行扫描——入口用了
        // 依赖声明的记法时（G-04 第二刀），单独 parse 失败但闭包能编。
        if !crate::project::is_project_source(text) {
            return None;
        }
        let root = self.root.as_deref();
        let path = self
            .path
            .clone()
            .or_else(|| root.map(|root| root.join("Main.sokonanoda")))?;
        Some(crate::project::compile_project_with_overlay(
            &path,
            Some(text),
            &self.options(),
            root,
            &self.overlay,
        ))
    }

    /// **判据前缀**：`judge_*` 合成文件时要放在文档前缀之前的"闭包上下文"。
    ///
    /// 项目模式 = 各依赖模块的源码（去掉 `import` 行，拓扑序）拼起来；单文件 = 空串。
    /// 有了它，`front::suggest`（quick-fix）、`probe_sub_goal_types`（子洞期望类型）
    /// 与 `match`/`by` 的判据才看得见被导入的名字（`docs/TESTING.md` §7b）。
    ///
    /// `offset` 是**入口文档坐标**：只用于判断调用方想要哪一段前缀（当前实现返回
    /// 整个闭包前缀，与偏移无关；保留参数是为了将来按偏移裁剪依赖）。
    pub fn judge_prefix(&self, offset: usize) -> String {
        let _ = offset;
        let Some(modules) = self.project_modules() else {
            return String::new();
        };
        let mut out = String::new();
        // 入口是最后一个模块（拓扑序）——它的文本由 `self.text` 提供，不在这里拼。
        for module in modules.iter().take(modules.len().saturating_sub(1)) {
            out.push_str(&crate::project::importless_source(&module.source));
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }

    /// 用**闭包缓存**里的入口报告与事件装配文档（命中时零内核工作）。
    ///
    /// CLI `query --file` 用它与 `check`/`build` 共用同一份摘要键；`project` 结构
    /// 不参与（`query` 的六个 op 都只读入口报告；跨文件能力由 LSP 走另一条路径）。
    /// 报告里的 `-- soko:hint` 阶梯这里补挂（缓存里存的是原始报告）。
    pub fn set_cached_entry(
        &mut self,
        text: &str,
        version: u64,
        report: crate::compile::DocumentReport,
        output: crate::compile::CompileOutput,
        project: Option<crate::project::ProjectReport>,
    ) {
        self.version = version;
        self.text = text.to_string();
        self.parse_error = None;
        // **整份项目报告也要回放**（T-A03）：LSP 的跨文件能力
        // （definition/references/rename/`soko/project` 的模块表/扇出判定）
        // 读的都是 `project_modules()`。只回放入口报告的话，命中缓存的文档
        // 会"能显示、不能跳转"。
        self.project = project;
        self.output = output;
        let mut report = report;
        crate::compile::attach_hints_to_report(text, &mut report);
        self.report = Some(report);
        // 原因也要跟着重算（它由 `project` 是否存在决定）。
        self.project_reason = Some(Self::compute_project_reason(&self.project, &self.text));
    }

    /// 最近一次编译的产物（CLI 在项目编译后据此写缓存）。
    pub fn compiled_output(&self) -> &crate::compile::CompileOutput {
        &self.output
    }

    /// 项目闭包的模块列表（拓扑序、入口最后）；单文件文档为 `None`。
    /// LSP 的跨文件引用/改名按它遍历每个模块的报告（I16 P5）。
    pub fn project_modules(&self) -> Option<&[crate::project::ModuleReport]> {
        self.project
            .as_ref()
            .map(|project| project.modules.as_slice())
    }

    /// 项目模式下：入口模块这次**编译成功**（`ModuleStatus::Compiled`）。
    ///
    /// LSP 用它决定「单文件 parse 失败」时是否仍然发报告（G-20 / X15）。
    /// 为什么需要它：记法随 `import` 传播（G-04 第二刀）之后，用库记法的单元
    /// **单文件必然 parse 失败**（`∈ ⊆ ∪ ∅` 不在本文件里），而闭包是好的
    /// ——`is_project_source`（`project/mod.rs`）专门为这种情况留了退路。
    /// 把闭包报告丢掉会让编辑器发**假**诊断、并让 hover / documentSymbol /
    /// codeAction / inlayHint 全部回答 `null`，而同一份文本走 CLI 判卷 exit 0。
    ///
    /// 入口模块状态是 `LoadFailed`（**真的**有语法错误）时返回 `false`：
    /// 那时老契约（parse 错误优先）才对，闭包报告是空的、会误导。
    pub fn project_entry_compiled(&self) -> bool {
        self.project
            .as_ref()
            .and_then(|project| project.entry_module())
            .is_some_and(|module| module.status == crate::project::ModuleStatus::Compiled)
    }

    /// 项目模式下：这个名字由**哪个模块**声明（返回模块路径与声明 span）。
    /// 跨文件跳转用（LSP `textDocument/definition`，I16 P5）；单文件模式返回
    /// `None`（调用方回退到请求文档自身）。
    pub fn project_definition(&self, name: &str) -> Option<(std::path::PathBuf, crate::Span)> {
        let project = self.project.as_ref()?;
        for module in &project.modules {
            for decl in &module.report.decls {
                if decl.name.as_deref() == Some(name) && decl.status != DeclStatus::Failed {
                    return Some((module.path.clone(), decl.span));
                }
            }
        }
        None
    }

    /// 当前 prelude 模式对应的编译选项。
    fn options(&self) -> CompileOptions {
        CompileOptions { prelude: self.mode }
    }

    /// 文档的声明类型表（每次查询算一次，供 runs 分类复用）。
    fn decl_kinds(&self) -> Vec<(String, SemanticKind)> {
        semantic::declaration_kinds(&self.text)
    }

    /// 把一段内核文本切成 wire runs（着色单一来源）。
    fn runs(
        &self,
        decls: &[(String, SemanticKind)],
        text: &str,
        binders: &[String],
    ) -> Vec<RunInfo> {
        semantic::tag_runs(text, decls, binders)
            .into_iter()
            .map(|run| RunInfo {
                text: run.text,
                kind: run.kind.map(|k| k.as_str().to_string()),
            })
            .collect()
    }

    // ── check ───────────────────────────────────────────────────────────────

    /// 整文件判卷摘要。**与 `--json` 事件流同源**（同一个 `front::session` +
    /// 同一个 `compile_all_with`），契约测试断言两者计数一致（设计文档 A4）。
    ///
    /// `failed` 含**两类**失败：内核拒绝的声明 + 源文本的 parse 诊断（G-10）。
    /// 解析失败时 `counts` 仍是全 0（诚实：一条声明都没验过），而 parse 诊断
    /// 让摘要不再"假绿"——它的 `code`/`message`/span 与 `--json` 事件流同源
    /// （`crate::Diagnostic` 的唯一真相）。
    ///
    /// 每条失败/警告同时给出**两种坐标**（G-15 / WO-010）：`start`/`end` 是
    /// **字节** offset（坐标空间 = 入口文件），`start_line`/`start_col`/
    /// `end_line`/`end_col` 是 1 基行列（与事件 `span` 同一批数字）。加字段是
    /// 协议允许的（`docs/protocol.md`：fields are additive only），`start`/`end`
    /// 的名字与字节语义**不许改**——`--offset`、缓存摘要、LSP 着色都依赖它。
    pub fn check(&self) -> CheckSummary {
        // 用最近一次编译的产物：`set_text` 已经算过（项目模式是整个闭包），
        // 这里再编译一遍纯属浪费——`query check` 冷跑曾因此慢一倍。
        let output = self.output.clone();
        let mut counts = CheckCounts::default();
        for event in &output.events {
            use crate::compile::CheckEvent::*;
            match event {
                DeclarationChecked { .. } => counts.decl_checked += 1,
                ExampleChecked => counts.example_checked += 1,
                ExerciseOpen { .. } => counts.exercise_open += 1,
                TypeChecked { .. } => counts.expr_typed += 1,
                Reduced { .. } => counts.expr_reduced += 1,
                Printed { .. } => counts.decl_printed += 1,
            }
        }
        let mut failed: Vec<FailedDecl> = output
            .errors
            .iter()
            .map(|e| {
                let (start_line, start_col, end_line, end_col) = line_col(e.span);
                FailedDecl {
                    name: None,
                    code: e.kind.code().to_string(),
                    message: e.message.clone(),
                    start: e.span.start.offset,
                    end: e.span.end.offset,
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                }
            })
            .collect();
        // 解析失败时 `output` 是空产物（`session` 的 parse 失败分支），唯一真相
        // 是 `self.parse_error`。不合成进 `failed`，agent 的主判卷通道就会把
        // "这份文本根本解析不了"读成"文件里什么都没有"（G-10）。
        // `name` 保持 `None`：解析失败时没有可信的声明名。
        //
        // **例外：闭包把它救回来了**（G-20 同一条判据，LSP 侧已修）。记法随
        // `import` 传播之后，用库记法的单元单文件**必然** parse 失败，而闭包是
        // 好的——那条 parse 诊断是**救援过程的中间产物**，不是这份文件的结论。
        // 报给 agent 会让正常文件看起来是坏的（实测：单元⑤ 改写后 `query check`
        // 报 `∈` 未声明，而同一份文本 `grade` exit 0、计数 5/7——两条通道打架）。
        // 闸门与 `goals` 共用同一条判据（[`Self::usable`]，G-22 的教训：
        // 同一条判据写两遍，就会有一处漏掉）。
        if let Some(diag) = self.parse_error.as_ref().filter(|_| !self.usable()) {
            let (start_line, start_col, end_line, end_col) = line_col(diag.span);
            failed.push(FailedDecl {
                name: None,
                code: diag.code().to_string(),
                message: diag.message.clone(),
                start: diag.span.start.offset,
                end: diag.span.end.offset,
                start_line,
                start_col,
                end_line,
                end_col,
            });
        }
        let warnings = output
            .warnings
            .iter()
            .map(|w| {
                let (start_line, start_col, end_line, end_col) = line_col(w.span);
                WarningInfo {
                    code: w.code().to_string(),
                    message: w.message.clone(),
                    hint: Some(w.hint().to_string()),
                    start: w.span.start.offset,
                    end: w.span.end.offset,
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                }
            })
            .collect();
        // **清单 `requires` 漂移也要说出来**（T-A05 / G-24 的另一半）。
        //
        // 为什么必须在这里说：那条漂移**不再关掉缓存**了（`is_clean()` 把它摘了
        // 出去——它是可回放的确定性事实），于是机器可读通道里就再也没有它的影子。
        // 用户看不到提示、只看到"版本对不上"的困惑，比缓存不工作更糟。
        // 归到**入口文件第 1 行**（清单不在入口文件里，但漂移是关于这份文档的）。
        let mut warnings: Vec<WarningInfo> = warnings;
        if let Some(note) = self.project.as_ref().and_then(|p| p.requires_warning.clone()) {
            warnings.push(WarningInfo {
                code: "manifest-version".to_string(),
                message: note,
                hint: Some(
                    "把清单的 `requires` 改成当前版本（`sokonanoda version --json` 的 \
                     `version`）可以消掉这条提示；它不影响判卷，也不影响缓存。"
                        .to_string(),
                ),
                start: 0,
                end: 0,
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            });
        }
        CheckSummary {
            version: self.version,
            counts,
            failed,
            warnings,
        }
    }

    /// 这份文档的**真相层**是否可用——「单独 parse 失败但 `import` 闭包编译成功」
    /// 算**可用**（G-20 的语义）。
    ///
    /// **一处判据、三处引用**（`goals` 经 [`Self::parsable`]、`check` 的 parse
    /// 诊断闸门、LSP 的 `Doc::set_text`）。以前这条判据在 `check` 与 LSP 各写了
    /// 一遍，**`goals` 漏了** ⇒ 项目入口的声明栏恒为空（**G-22**）：
    /// 记法随 `import` 传播之后，"用库记法的单元单文件必然 parse 失败"是**常态**，
    /// 那条 parse 诊断是**救援过程的中间产物**，不是这份文件的结论。
    ///
    /// 闭包也失败（入口 `LoadFailed`，真的有语法错误）时返回 `false` ⇒ 老契约
    /// （`NotParsable` / parse 诊断优先）才对（G-17）。
    fn usable(&self) -> bool {
        self.report.is_some() && (self.parse_error.is_none() || self.project_entry_compiled())
    }

    /// 源文本可解析？不可解析 ⇒ [`QueryError::NotParsable`]（"问不出来"）。
    ///
    /// 这是"正常的没有"（空数组 / `None`）与"问不出来"（`QueryError`）分界的
    /// 唯一入口（设计 §4.1）：报告类查询（`goals`/`holes`/`nextHole`）在解析失败
    /// 时必须走这里，而不是答一个空数组（G-17）。`check` **不**调它——`check`
    /// 的答案就是"这份文本解析不了"，它把 `parse_error` 合成进 `failed`。
    ///
    /// **判据本体是 [`Self::usable`]**（别在这里另写一份，G-22 就是这么来的）。
    fn parsable(&self) -> Result<(), QueryError> {
        if self.usable() {
            Ok(())
        } else {
            Err(QueryError::NotParsable)
        }
    }

    // ── state（Lean `goalsAt?` 语义）─────────────────────────────────────────

    /// 光标处的目标状态。语义与 `soko/stateAt` **完全一致**（同一实现）：
    /// 光标在某 tactic 的 span 内 → **进入**该 tactic 之前的状态；否则停在最后
    /// 一条在光标前结束的 tactic 之后；首个 tactic 之前 → 根状态。
    pub fn state_at(&self, cursor: usize) -> Result<StateAnswer, QueryError> {
        if cursor > self.text.len() {
            return Err(QueryError::PositionOutOfRange);
        }
        let report = self.report.as_ref().ok_or(QueryError::NotParsable)?;
        let Some(d) = report
            .decls
            .iter()
            .find(|d| d.span.start.offset <= cursor && cursor <= d.span.end.offset)
        else {
            return Err(QueryError::OutsideDeclarations);
        };
        let selection = select_state_at(d, cursor);
        // 一次 parse，然后每个 goal/binder 纯分类（goal-rendering §2.1）。
        let decls = self.decl_kinds();
        let goal = |g: &ByGoalState| {
            let names: Vec<String> = g.binders.iter().map(|b| b.name.clone()).collect();
            GoalInfo {
                goal: g.ty.clone(),
                goal_runs: self.runs(&decls, &g.ty, &names),
                binders: g
                    .binders
                    .iter()
                    .map(|b| BinderInfo {
                        name: b.name.clone(),
                        ty: b.ty.clone(),
                        ty_runs: self.runs(&decls, &b.ty, &names),
                    })
                    .collect(),
            }
        };
        let goals: Vec<GoalInfo> = selection.goals.iter().map(goal).collect();
        let first = goals.first().cloned();
        Ok(StateAnswer {
            version: self.version,
            decl: Some(DeclHeader {
                name: decl_name(d),
                kind: d.kind.as_str().to_string(),
                status: status_str(d.status).to_string(),
                start: d.span.start.offset,
                end: d.span.end.offset,
            }),
            goal: first.as_ref().map(|g| g.goal.clone()),
            goal_runs: first
                .as_ref()
                .map(|g| g.goal_runs.clone())
                .unwrap_or_default(),
            binders: first
                .as_ref()
                .map(|g| g.binders.clone())
                .unwrap_or_default(),
            goals,
            span: selection.span.map(|s| (s.start.offset, s.end.offset)),
            step: selection.step,
            total: selection.total,
        })
    }

    // ── goals（声明级）────────────────────────────────────────────────────

    /// 每个声明的类型/状态/开放目标/洞。`probe: true` 时用请求期内核探针补
    /// 子洞期望类型（`docs/design/spine-meta-a.md`）。
    ///
    /// 解析失败 ⇒ [`QueryError::NotParsable`]（不是"空的声明列表"）：这份文本里
    /// 有多少声明**问不出来**，与"画布上确实没有声明"是两件事（G-17）。
    pub fn goals(&self, probe: bool) -> Result<Vec<DeclInfo>, QueryError> {
        self.parsable()?;
        let report = if probe {
            self.probed_report()
        } else {
            self.report.clone().unwrap_or_default()
        };
        // 洞的"多余"标记来自**同一份报告**的 kernel 终审 warning，绝不另算
        // （`docs/design/redundant-sorry.md`）。
        let redundant_spans = redundant_hole_spans(&report);
        let decls = self.decl_kinds();
        Ok(report
            .decls
            .iter()
            .map(|d| {
                let name = decl_name(d);
                let binder_names: Vec<String> = d.binders.iter().map(|b| b.name.clone()).collect();
                let ty_runs = d
                    .ty_text
                    .as_deref()
                    .map(|ty| self.runs(&decls, ty, &binder_names))
                    .unwrap_or_default();
                let open = d.status == DeclStatus::Open;
                DeclInfo {
                    name,
                    kind: d.kind.as_str().to_string(),
                    status: status_str(d.status).to_string(),
                    start: d.span.start.offset,
                    end: d.span.end.offset,
                    ty: d.ty_text.clone(),
                    ty_runs,
                    goals: if open {
                        d.by_steps
                            .last()
                            .map(|s| s.goals.iter().map(|g| g.ty.clone()).collect())
                            .unwrap_or_else(|| d.goal.clone().into_iter().collect())
                    } else {
                        Vec::new()
                    },
                    goal: d.goal.clone(),
                    binders: d
                        .binders
                        .iter()
                        .map(|b| BinderInfo {
                            name: b.name.clone(),
                            ty: b.ty.clone(),
                            ty_runs: self.runs(&decls, &b.ty, &binder_names),
                        })
                        .collect(),
                    hole: if open {
                        d.holes.first().map(|s| (s.start.offset, s.end.offset))
                    } else {
                        None
                    },
                    holes: if open {
                        d.holes
                            .iter()
                            .enumerate()
                            .map(|(index, span)| HoleInfo {
                                start: span.start.offset,
                                end: span.end.offset,
                                id: format!("{}:{index}", decl_name(d)),
                                redundant: hole_is_redundant(span, &redundant_spans),
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    sub_goals: if open {
                        d.sub_goals
                            .iter()
                            .map(|sub| SubGoalInfo {
                                start: sub.span.start.offset,
                                end: sub.span.end.offset,
                                ty: sub.ty.clone(),
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    // 内核判定的下一步建议来自 LSP 侧的 code-action 层（需要
                    // judge 上下文）；CLI/MCP 侧目前留空，见 H6-A 的 as-built。
                    code_actions: Vec::new(),
                }
            })
            .collect())
    }

    /// 请求期内核探针后的报告：只补开放练习里 `sub_goals[i].ty == None` 的项
    /// （`docs/design/spine-meta-a.md` §2/§4）。**绝不进 keystroke 路径**。
    ///
    /// 公开是给**需要原始报告**的适配器用的（LSP 的 hover / inlay 要 `hovers`、
    /// `by_steps`、`ty_text`、`sub_goals`，这些不在类型化查询结果里）——它们必须
    /// 复用这一份探针逻辑，不得各自再写一遍"哪些子洞要探、按什么键对齐"。
    pub fn probed_report(&self) -> DocumentReport {
        let Some(report) = &self.report else {
            return DocumentReport::default();
        };
        let needs_probe = report
            .decls
            .iter()
            .any(|d| d.status == DeclStatus::Open && d.sub_goals.iter().any(|s| s.ty.is_none()));
        if !needs_probe {
            return report.clone();
        }
        let mut report = report.clone();
        for d in &mut report.decls {
            if d.status != DeclStatus::Open || !d.sub_goals.iter().any(|s| s.ty.is_none()) {
                continue;
            }
            let probed = probe_sub_goal_types_with(
                &self.judge_prefix(d.span.start.offset),
                &self.text,
                &self.options(),
                d.span,
            );
            for sub in &mut d.sub_goals {
                if sub.ty.is_none() {
                    if let Some(ty) = probed
                        .iter()
                        .find(|(offset, _)| *offset == sub.span.start.offset)
                        .map(|(_, ty)| ty.clone())
                    {
                        sub.ty = Some(ty);
                    }
                }
            }
        }
        report
    }

    // ── holes（可寻址）─────────────────────────────────────────────────────

    /// 全部洞（文件序），带**稳定 id** 与期望类型。
    ///
    /// `id`（`<declName>:<index>`）是程序化消费者的唯一稳定引用——同一源码位置
    /// 可能有多个子目标，`soko/nextHole` 的按位置导航在那种情形下不可用
    /// （`docs/protocol.md` 的 Known limitation）。
    ///
    /// 解析失败 ⇒ [`QueryError::NotParsable`]（不是"没有洞"）：空数组是**答案**，
    /// 不能让 agent 把不可解析的画布读成"没有剩下的洞"（G-17）。
    pub fn holes(&self) -> Result<Vec<LocatedHole>, QueryError> {
        let decls = self.goals(true)?;
        let mut out: Vec<LocatedHole> = decls
            .iter()
            .flat_map(|d| {
                d.holes.iter().map(|h| LocatedHole {
                    id: h.id.clone(),
                    start: h.start,
                    end: h.end,
                    // 子洞按位置对齐期望类型（几处可能同址，故按位置索引找）。
                    ty: d
                        .sub_goals
                        .iter()
                        .find(|s| s.start == h.start && s.end == h.end)
                        .and_then(|s| s.ty.clone()),
                    decl: d.name.clone(),
                    // 与 `HoleInfo` 同一来源（`goals` 里算好），不重复判定。
                    redundant: h.redundant,
                })
            })
            .collect();
        out.sort_by_key(|h| (h.start, h.end));
        Ok(out)
    }

    /// 相对 `from` 的下一个（`forward`）或上一个洞——服务端定位，客户端不扫文本。
    ///
    /// 解析失败 ⇒ [`QueryError::NotParsable`]；`Ok(None)` 才是"这个方向上没有洞了"。
    pub fn next_hole(&self, from: usize, forward: bool) -> Result<Option<LocatedHole>, QueryError> {
        let holes = self.holes()?;
        Ok(if forward {
            holes.into_iter().find(|h| h.start > from)
        } else {
            holes.into_iter().rev().find(|h| h.start < from)
        })
    }

    // ── hints ─────────────────────────────────────────────────────────────

    /// 光标所在声明的 `-- soko:hint` 阶梯（无状态：从不数剩余条数）。
    pub fn hints_at(&self, cursor: usize) -> Vec<String> {
        let Some(report) = self.report.as_ref() else {
            return Vec::new();
        };
        report
            .decls
            .iter()
            .find(|d| d.span.start.offset <= cursor && cursor <= d.span.end.offset)
            .map(|d| d.hints.clone())
            .unwrap_or_default()
    }

    // ── reduce ────────────────────────────────────────────────────────────

    /// 对给定表达式求值（与 REPL `#reduce` 同一真相）。
    pub fn reduce(&self, expr: &str) -> Option<ReduceAnswer> {
        let src = format!("{}\n#reduce {expr}\n", self.text);
        let output = match self.project_compile(&src) {
            Some(project) => project
                .entry_module()
                .map(|module| module.events.clone())
                .unwrap_or_default(),
            None => compile_all_with(&parse_or_empty(&src), &self.options()).0,
        };
        output.events.iter().find_map(|e| match e {
            crate::compile::CheckEvent::Reduced { text, .. } => Some(ReduceAnswer {
                value: text.clone(),
                ty: None,
            }),
            _ => None,
        })
    }
}

/// 解析源文本；失败时给一个空文件（`check` 会带着 parse 诊断返回）。
fn parse_or_empty(src: &str) -> crate::ast::FolFile {
    crate::parse(src).unwrap_or(crate::ast::FolFile {
        commands: Vec::new(),
        src: String::new(),
    })
}

/// 声明名（匿名 `example` 用 `example@<line>` 形式，与既有协议一致）。
pub fn decl_name(d: &DeclState) -> String {
    match &d.name {
        Some(n) => n.clone(),
        None => format!("{}@{}", d.kind.as_str(), d.span.start.line),
    }
}

/// 声明状态的稳定文本（协议 wire 用的就是它）。
pub fn status_str(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Checked => "checked",
        DeclStatus::Open => "open",
        DeclStatus::Failed => "failed",
    }
}

/// 「多余的 `sorry`」的 warning span（内核终审过的那种，
/// `docs/design/redundant-sorry.md`）。
fn redundant_hole_spans(report: &DocumentReport) -> Vec<Span> {
    report
        .warnings
        .iter()
        .filter(|w| w.code() == "redundant-sorry")
        .map(|w| w.span)
        .collect()
}

/// 洞 span 与 warning span 形状未必相同（多余洞走 generic fallback 时洞是整段
/// 值、warning 收窄到 `sorry` token）⇒ 用**包含**判定（与 LSP 侧同一条规则）。
fn hole_is_redundant(hole: &Span, redundant: &[Span]) -> bool {
    redundant
        .iter()
        .any(|r| hole.start.offset <= r.start.offset && r.end.offset <= hole.end.offset)
}

/// 把报告里的洞 span 转成 offset 区间（供 `holes`/`next_hole` 复用）。
pub fn span_offsets(span: Span) -> (usize, usize) {
    (span.start.offset, span.end.offset)
}

/// 诊断 span 的 wire 行列（G-15 / WO-010）：`(start_line, start_col, end_line,
/// end_col)`，1 基，与 `--json` 事件的 `span.{start,end}` **同一批数字**。
///
/// `start`/`end` 仍是**字节** offset（`FailedDecl`/`WarningInfo` 的既有字段，
/// 语义不许改）；这四个字段只是把同一位置的另一种坐标一起给出来，省得消费者
/// 自己拿字符下标去换算（那正是 G-15 的假缺口现场）。
fn line_col(span: Span) -> (u32, u32, u32, u32) {
    (
        span.start.line as u32,
        span.start.column as u32,
        span.end.line as u32,
        span.end.column as u32,
    )
}

#[cfg(test)]
mod tests;
