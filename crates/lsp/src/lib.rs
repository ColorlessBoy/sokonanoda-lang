//! `sokonanoda-lsp`: a language server for `.sokonanoda` teaching files.
//!
//! Feedback philosophy (docs/design/infrastructure.md):
//! - diagnostics per declaration with stable codes and teaching hints;
//! - hover shows the inferred type of the expression under the cursor
//!   (from the front-end type map) or the goal of an open exercise `sorry`;
//! - document symbols / code lenses expose exercise state
//!   (open / solved / failed);
//! - code actions turn the first proof step into text and are ordered by
//!   kernel-verified first (see docs/design/hints-suggestions.md);
//! - rename / references / inlay hints follow LSP 3.17
//!   (docs/design/rename-inlay.md).
//!
//! The crate is also a library so the single `sokonanoda` binary can host
//! the server (`sokonanoda lsp`, the gleam pattern): call
//! [`run`] from any front-end.

mod actions;
#[cfg(test)]
mod by_sorry_range_tests;
mod hints;
mod inlay;
mod project_refs;
mod protocol;
mod query_map;
mod render;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod testutil;
mod tokens;

// 声明名是真相层的词表（`docs/protocol.md`）：用它的实现，不再保逐字副本。
use protocol::{
    GoalDeclInfo, GoalsParams, GoalsResponse, NextHoleParams, StateAtParams, StateAtResponse,
};
use render::{
    bracket_hover, decl_at, definition_at, diagnostic_from_compile, diagnostic_from_parse,
    expr_hover, highlight_uses, hover_type_at, range_of, scope_names_at, semantic_kind_at,
    status_label, symbol_kind,
};
use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{
    prelude_mode_from_source, CompileOptions, DeclStatus, DocumentReport, GoalBinder, HoverType,
    PreludeMode, ResolvedTarget,
};
use sokonanoda_front::query::{decl_name, QueryDoc};
use sokonanoda_front::semantic::{semantic_tokens as front_semantic_tokens, SemanticKind};
use std::sync::Mutex;
use tokens::{encode_semantic_tokens, semantic_token_options};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// 文档状态：真相层 [`QueryDoc`]（文本 + 会话式编译 + 报告 + 版本）的 LSP 薄包装。
///
/// 查询逻辑（目标/洞/状态/提示的选择）全部在 `sokonanoda_front::query`；这里
/// 只提供 LSP 形状的访问器与 didOpen/didChange 的编译缓存路径。
struct Doc {
    doc: QueryDoc,
}

impl Doc {
    fn new() -> Self {
        Self {
            doc: QueryDoc::new(),
        }
    }

    /// 当前文本（`doc.text()` 的读法）。
    fn text(&self) -> &str {
        &self.doc.text
    }

    /// 真相层本体：`soko/*` 的每个查询入口（`goals` / `holes` / `next_hole` /
    /// `hints_at` / `state_at` / …）都是它的方法，LSP 只做形状映射。
    fn query(&self) -> &QueryDoc {
        &self.doc
    }

    /// prelude 模式（`Full` / `Bare`，可由文件注释指令覆盖）。
    fn mode(&self) -> PreludeMode {
        self.doc.mode
    }

    /// 最近一次编译报告；`None` = 尚未编译过或 parse 失败（LSP 既有契约）。
    fn report(&self) -> Option<&DocumentReport> {
        self.doc.report.as_ref()
    }

    /// 最近一次 parse 诊断。
    fn parse_error(&self) -> Option<&sokonanoda_front::Diagnostic> {
        self.doc.parse_error.as_ref()
    }

    /// The document's LSP version; versioned `WorkspaceEdit`s (rename) must
    /// carry it for atomic client-side application. LSP 的版本是 i32、真相层
    /// 是 u64——`as` 在两个方向上对非负版本恒等（负版本按位往返）。
    fn version(&self) -> i32 {
        self.doc.version as i32
    }

    /// didOpen / didChange / didSave 路径：换文本、跑（或命中）共享编译缓存、
    /// 更新真相层状态。
    ///
    /// 缓存（`front::compile::cache`）与 CLI 读同一份磁盘条目：`sokonanoda
    /// build` / `check` 预热过的画布对编辑器同样有效。命中只**重放**内核已经为
    /// 这个（编译器版本、构建、prelude 模式、文本）产出过的报告——绝不从缓存
    /// 里**推断**任何东西。单元测试跳过它（`cfg!(test)`），与 CLI 同策略，所以
    /// `cargo test` 不碰开发者的真实缓存。
    fn set_text(
        &mut self,
        text: &str,
        lsp_version: i32,
        mode: Option<PreludeMode>,
        path: Option<std::path::PathBuf>,
        root: Option<std::path::PathBuf>,
    ) {
        let mode = mode.unwrap_or(self.doc.mode);
        let options = CompileOptions { prelude: mode };
        // 有 `import` 的文档走**项目闭包**：单文件缓存键会张冠李戴（依赖不在
        // 键里），所以这里既不复用也不写入单文件缓存（I16 P5）。
        let has_imports = sokonanoda_front::parse(text)
            .map(|file| file.commands.iter().any(|command| command.is_import()))
            .unwrap_or(false);
        self.doc.path = path;
        self.doc.root = root;
        let cached = if cfg!(test) || has_imports {
            None
        } else {
            cache::load(text, &options)
        };
        if let Some(entry) = cached {
            self.doc.text = text.to_string();
            // 会话保持原样：`Session::update` 自己会在 prelude 模式变化时重置
            // （它按整文件重新判定模式），下一次未命中缓存时自愈。
            self.doc.mode = mode;
            self.doc.version = lsp_version as u64;
            self.doc.parse_error = None;
            self.doc.report = Some(entry.report);
            return;
        }
        // 原地复用会话（I8 增量的关键）：prelude 模式变化时由真相层重建。
        self.doc.set_text(text, lsp_version as u64, Some(mode));
        if self.doc.parse_error.is_some() {
            // LSP 既有契约：parse 失败时**没有报告**（hover / documentSymbol /
            // codeAction / inlayHint 等据此回答 `null`）。真相层用"空报告 +
            // parse_error"表达同一件事，这里把它折回 LSP 形状。
            self.doc.report = None;
            return;
        }
        if !cfg!(test) && !has_imports {
            if let Some(report) = &self.doc.report {
                cache::store(
                    text,
                    &options,
                    &CachedCompile {
                        report: report.clone(),
                        output: None,
                    },
                );
            }
        }
    }
}

/// 打开的文档表 + 会话级根（`initialize` 的 `rootUri`/`workspaceFolders`）。
///
/// 访问器面与旧的单文档 `Doc` 一致（都作用在**当前活跃文档**上）：19 处
/// `self.doc.lock()` 的既有 handler 因此不用逐个改；带 URI 的 handler 只要
/// 先 `focus(&uri)` 就能拿到正确的那一份（I16 P5）。
struct Docs {
    map: std::collections::HashMap<Url, Doc>,
    order: Vec<Url>,
    root: Option<std::path::PathBuf>,
    active: Option<Url>,
}

impl Docs {
    fn new() -> Self {
        Self {
            map: std::collections::HashMap::new(),
            order: Vec::new(),
            root: None,
            active: None,
        }
    }

    /// 打开（或复用）一份文档并把焦点切到它。
    fn focus_or_open(&mut self, uri: &Url) {
        if !self.map.contains_key(uri) {
            self.map.insert(uri.clone(), Doc::new());
            self.order.push(uri.clone());
        }
        self.active = Some(uri.clone());
    }

    /// 把焦点切到某份已打开的文档；没有就保持现状（单文档客户端也照旧工作）。
    fn focus(&mut self, uri: &Url) {
        if self.map.contains_key(uri) {
            self.active = Some(uri.clone());
        }
    }

    fn active(&self) -> Option<&Doc> {
        self.active.as_ref().and_then(|uri| self.map.get(uri))
    }

    fn active_mut(&mut self) -> Option<&mut Doc> {
        let uri = self.active.clone()?;
        self.map.get_mut(&uri)
    }

    fn remove(&mut self, uri: &Url) {
        self.map.remove(uri);
        self.order.retain(|item| item != uri);
        if self.active.as_ref() == Some(uri) {
            self.active = self.order.first().cloned();
        }
    }

    /// 入口路径与模块根：来自文档 URI 的目录与会话根。
    fn entry_context(&self) -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
        let path = self.active.as_ref().and_then(|uri| uri.to_file_path().ok());
        (path, self.root.clone())
    }

    // ---- 与旧 `Doc` 同形的访问器（作用在活跃文档上）----
    fn text(&self) -> &str {
        self.active().map(Doc::text).unwrap_or("")
    }

    /// 活跃文档本体；没有打开任何文档时退化为一个空的只读文档
    /// （`soko/*` 的既有语义：没文档 ⇒ 答空，而不是报错）。
    fn active_doc(&self) -> &Doc {
        static EMPTY: std::sync::OnceLock<Doc> = std::sync::OnceLock::new();
        self.active().unwrap_or_else(|| EMPTY.get_or_init(Doc::new))
    }

    fn query(&self) -> &QueryDoc {
        self.active_doc().query()
    }

    fn report(&self) -> Option<&DocumentReport> {
        self.active().and_then(Doc::report)
    }

    fn parse_error(&self) -> Option<&sokonanoda_front::Diagnostic> {
        self.active().and_then(Doc::parse_error)
    }

    fn mode(&self) -> PreludeMode {
        self.active().map(Doc::mode).unwrap_or(PreludeMode::Full)
    }

    fn version(&self) -> i32 {
        self.active().map(Doc::version).unwrap_or(0)
    }
}

/// 闭包里每个模块的 LSP 视图（跨文件引用/改名用）。
///
/// 打开的文档用**内存里的最新文本**（它的报告就是刚编译的那份），未打开的用
/// `ModuleReport::source`（编译时文本）——两者都与各自的报告自洽，span 可以直接
/// 当成编辑范围。打开的文档若 parse 失败（没有报告），该项**跳过**：宁可不改，
/// 也不拿过期 span 去编辑用户的缓冲区。
fn project_views(docs: &Docs) -> Option<Vec<project_refs::ModuleView<'_>>> {
    let modules = docs.query().project_modules()?;
    let mut views = Vec::with_capacity(modules.len());
    for module in modules {
        let uri = Url::from_file_path(&module.path).ok()?;
        match docs.map.get(&uri) {
            Some(doc) => {
                let Some(report) = doc.report() else {
                    continue;
                };
                views.push(project_refs::ModuleView {
                    uri,
                    text: doc.text(),
                    version: Some(doc.version()),
                    report,
                });
            }
            None => views.push(project_refs::ModuleView {
                uri,
                text: &module.source,
                version: None,
                report: &module.report,
            }),
        }
    }
    (!views.is_empty()).then_some(views)
}

struct Backend {
    client: Client,
    doc: Mutex<Docs>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            doc: Mutex::new(Docs::new()),
        }
    }

    async fn refresh(&self, uri: Url, text: String, version: Option<i32>) {
        // 教学文档量级小，锁内同步编译可接受（此前也是同步全量编译）。
        // 文本 →（缓存命中 / 会话式重编译）→ 状态全部由 `Doc::set_text` 负责；
        // 诊断是那份状态的**视图**，与缓存命中路径逐字一致。
        let (diagnostics, others) = {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_or_open(&uri);
            let mode = prelude_mode_from_source(&text);
            let lsp_version = version.unwrap_or_else(|| docs.version());
            let (path, root) = docs.entry_context();
            if let Some(doc) = docs.active_mut() {
                doc.set_text(&text, lsp_version, Some(mode), path, root);
            }
            let diagnostics = match docs.parse_error() {
                Some(diag) => vec![diagnostic_from_parse(diag)],
                None => docs.report().map(report_diagnostics).unwrap_or_default(),
            };
            // 依赖变了 ⇒ 打开着的下游文档要跟着刷新（v1：全部重编译重发；
            // 教学项目里同时打开的文档很少，正确性优先，见设计 §4.9）。
            // v1 语义（I16 P5）：变更的那份文档自己重编译；其它打开文档**重发**
            // 上次的诊断（不发就永远停在旧状态）。真正的"依赖变了 ⇒ 下游自动
            // 重编译"（跨文件失效）是 P5 余项——写在这里的第一版会在 tower-lsp
            // 的串行通知里挂住，先按能保证的语义发布，并在设计文档里登记。
            let others: Vec<(Url, Vec<Diagnostic>)> = docs
                .order
                .iter()
                .filter(|other| **other != uri)
                .filter_map(|other| {
                    let doc = docs.map.get(other)?;
                    let diagnostics = match doc.parse_error() {
                        Some(diag) => vec![diagnostic_from_parse(diag)],
                        None => doc.report().map(report_diagnostics).unwrap_or_default(),
                    };
                    Some((other.clone(), diagnostics))
                })
                .collect();
            (diagnostics, others)
        };
        let _ = self
            .client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
        for (other, diagnostics) in others {
            let _ = self
                .client
                .publish_diagnostics(other, diagnostics, None)
                .await;
        }
    }

    // ---- I9 goal 视图协议：结构化 goal 请求（coq-lsp `proof/goals` 模式）----

    /// 组 `soko/goals` 的 wire 数据。`probe` = 是否跑请求期 kernel 探针填
    /// 函数 spine 子洞的期望类型（`nextHole` 只看洞 span，用 `false` 不引入
    /// 内核成本）。
    fn goal_decls(&self, probe: bool) -> Option<(String, Vec<GoalDeclInfo>)> {
        let doc = self.doc.lock().expect("doc lock");
        // LSP 契约：没有报告（尚未编译 / parse 失败）时 `soko/goals` 答空。
        doc.report()?;
        let text = doc.text();
        let decls = doc
            .query()
            .goals(probe)
            .into_iter()
            .map(|decl| query_map::decl_info(decl, text))
            .collect();
        Some((text.to_string(), decls))
    }

    async fn goals(&self, params: GoalsParams) -> Result<GoalsResponse> {
        let _ = params;
        let decls = self
            .goal_decls(true)
            .map(|(_, decls)| decls)
            .unwrap_or_default();
        Ok(GoalsResponse { decls })
    }

    /// 服务器自述：版本 + 进程号。`sokonanoda: restart server` 用它在重启前后
    /// 各问一次，让「旧进程确实退出、新进程确实是新版本」变成**可见的事实**
    /// 而不是一句口头保证——扩展更新后跑着旧版服务器正是用户最常见的困惑
    /// （docs/vscode-dev-guide.md §5.6）。
    async fn version(&self, _params: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "pid": std::process::id(),
        }))
    }

    /// `soko/nextHole`：洞的定位与"下一个/上一个"的判定在真相层，这里只把
    /// 字节区间折成 `Range`、把位置折成字节 offset。
    async fn next_hole(&self, params: NextHoleParams) -> Result<Option<Range>> {
        let doc = self.doc.lock().expect("doc lock");
        let forward = params.forward.unwrap_or(true);
        let cursor = position_to_offset(doc.text(), params.position);
        Ok(doc
            .query()
            .next_hole(cursor, forward)
            .map(|hole| query_map::range_of_offsets(doc.text(), hole.start, hole.end)))
    }

    /// Hint ladder for the declaration at the cursor (docs/design/hints-
    /// suggestions.md). Stateless: the client owns progressive disclosure.
    async fn hints(&self, params: hints::HintsParams) -> Result<hints::HintsResponse> {
        let doc = self.doc.lock().expect("doc lock");
        Ok(hints::hints_for(doc.active_doc(), params))
    }

    /// Per-tactic goal state at the cursor (`soko/stateAt`,
    /// docs/design/by-tactics.md §6). Lean `goalsAt?` semantics: a cursor
    /// inside a tactic shows the state **entering** that tactic; otherwise
    /// the state after the last tactic that ended before it. The response
    /// carries the document version so clients drop stale answers.
    ///
    /// 选择语义（在哪个声明里、哪条 tactic、根状态的目标）全部在真相层
    /// （`QueryDoc::state_at`，`docs/protocol.md` §`soko/stateAt`）；这里只把
    /// "问不出来"折成既有的空响应、把字节 offset 映射成 `Range`。
    async fn state_at(&self, params: StateAtParams) -> Result<StateAtResponse> {
        let doc = self.doc.lock().expect("doc lock");
        let version = doc.version();
        let cursor = position_to_offset(doc.text(), params.position);
        match doc.query().state_at(cursor) {
            // 报告缺失 / parse 失败 / 位置不在任何声明内 / 越界：LSP 的 wire 没有
            // 错误通道，既有行为就是空响应（`decl: null` + 默认字段）。
            Err(_) => Ok(StateAtResponse::empty(version)),
            Ok(answer) => Ok(query_map::state_answer(doc.text(), answer)),
        }
    }
}

/// 0-based LSP position → byte offset（与本服务器的 char 计数约定一致）。
fn position_to_offset(text: &str, position: Position) -> usize {
    let mut offset = 0usize;
    for (i, line) in text.lines().enumerate() {
        if i == position.line as usize {
            let char_idx = text[offset..]
                .char_indices()
                .nth(position.character as usize)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            return offset + char_idx.min(line.len());
        }
        offset += line.len() + 1;
    }
    text.len()
}

/// Hover on a `by` tactic shows the goal state **entering** that tactic
/// (Lean Infoview-style, user request): every remaining goal with the
/// hypotheses in scope, computed from the per-tactic snapshot the front
/// already records (`by_steps`) — no re-check, no text scan. The whole tactic
/// span is the trigger; the goal view's hypotheses make term hovers redundant
/// inside it.
fn tactic_goal_hover(
    report: &DocumentReport,
    text: &str,
    offset: usize,
    decls: &[(String, SemanticKind)],
) -> Option<Hover> {
    let d = report
        .decls
        .iter()
        .find(|d| d.span.start.offset <= offset && offset <= d.span.end.offset)?;
    let step_index = d
        .by_steps
        .iter()
        .position(|s| s.span.start.offset <= offset && offset <= s.span.end.offset)?;
    let step = &d.by_steps[step_index];
    // 真相层的选择器（`docs/protocol.md` §`soko/stateAt`）：tactic 起点处的
    // 状态 = **进入**它的状态。
    let selection = sokonanoda_front::query::select_state_at(d, step.span.start.offset);
    let tactic_text = text
        .get(step.span.start.offset..step.span.end.offset)
        .unwrap_or("")
        .trim();
    // Header: the tactic itself + its 1-based position. The tactic is a
    // `sokonanoda` code block too, so its own syntax is highlighted (same fence
    // language as the goal state below, docs/design/goal-rendering.md §7).
    let mut value = code_block(tactic_text);
    if selection.total > 0 {
        value.push_str(&format!(
            "\ntactic {}/{}\n",
            step_index + 1,
            selection.total
        ));
    } else {
        value.push('\n');
    }
    if selection.goals.is_empty() {
        value.push_str("\n已无剩余目标 ✓\n");
    } else {
        let n = selection.goals.len();
        for (i, goal) in selection.goals.iter().enumerate() {
            value.push('\n');
            if n > 1 {
                value.push_str(&format!("**目标 {}/{}**\n", i + 1, n));
            }
            value.push_str(&goal_block(decls, &goal.binders, &goal.ty));
            value.push('\n');
        }
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range_of(step.span)),
    })
}

/// Diagnostics for a compiled [`sokonanoda_front::compile::DocumentReport`]:
/// elab/kernel errors, one WARNING per `sorry` (Lean-4 aligned), and
/// syntax-level warnings. Shared by the fresh-compile and cache-hit paths so a
/// cached open produces exactly the same diagnostics as a recompile.
fn report_diagnostics(report: &sokonanoda_front::compile::DocumentReport) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<_> = report.errors.iter().map(diagnostic_from_compile).collect();
    if report
        .decls
        .iter()
        .any(|d| d.status == DeclStatus::Open && !d.holes.is_empty())
    {
        for d in report.decls.iter().filter(|d| d.status == DeclStatus::Open) {
            let name = d.name.as_deref().unwrap_or("(anonymous)");
            diagnostics.push(Diagnostic {
                range: range_of(d.span),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("sorry".to_string())),
                source: Some("sokonanoda".to_string()),
                message: format!(
                    "declaration '{}' uses 'sorry' (exercise not yet solved)",
                    name
                ),
                ..Diagnostic::default()
            });
        }
    }
    for warning in &report.warnings {
        diagnostics.push(Diagnostic {
            range: range_of(warning.span),
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(warning.code().to_string())),
            source: Some("sokonanoda".to_string()),
            message: format!("{}\n\n提示：{}", warning.message, warning.hint()),
            ..Diagnostic::default()
        });
    }
    diagnostics
}

/// Language id used by **every** markdown code fence the server emits, so the
/// editor colours it with the `sokonanoda` TextMate grammar (which is
/// contract-tested against `front::semantic`) — the single source for any
/// `.sokonanoda` text the client renders (docs/design/goal-rendering.md §7).
const CODE_LANG: &str = "sokonanoda";

/// A fenced `sokonanoda` code block for editor markdown (hover / completion
/// docs / any place that shows language text).
fn code_block(text: &str) -> String {
    format!("```{CODE_LANG}\n{}\n```", text.trim_end_matches('\n'))
}

/// Render a goal state (hypotheses + `⊢ goal`) as one `sokonanoda` code block —
/// the same line model as the tactic hover and the Infoview. The fence text is
/// the **text projection of the front goal runs** ([`goal_runs`] +
/// [`runs_to_text`]), i.e. exactly the block the Infoview colours from the
/// `soko/stateAt` `ty_runs`/`goal_runs`, so the two can never drift.
fn goal_block(decls: &[(String, SemanticKind)], binders: &[GoalBinder], goal: &str) -> String {
    let hyps: Vec<(String, String)> = binders
        .iter()
        .map(|b| (b.name.clone(), b.ty.clone()))
        .collect();
    let runs = sokonanoda_front::semantic::goal_runs(&hyps, goal, decls);
    code_block(&sokonanoda_front::semantic::runs_to_text(&runs))
}

/// Build an LSP `Hover` from a resolved expression hover, carrying the
/// expression's source range so the editor highlights exactly what is shown.
fn hover_markup(res: render::HoverResolved) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: code_block(&res.content),
        }),
        range: Some(range_of(res.range)),
    }
}

/// 半截表达式的 goal-state hover（I13-S5，用户需求）：值写了一半、内核
/// 拒绝时（如 `And.intro b a` 还差两个前提），hover 不只给报错——把推断
/// 出的**剩余目标**列出来（`⊢ b`、`⊢ a`）。
///
/// 性能边界：只在 **hover 请求时**计算（不在按键路径上），且 `judge_infer`
/// 有缓存——同一位置重复悬停零成本；指纹含前缀文本，其它位置的编辑会
/// 失效缓存（保守但正确）。
fn half_expression_goals_hover(
    report: &DocumentReport,
    text: &str,
    offset: usize,
    decls: &[(String, SemanticKind)],
) -> Option<Hover> {
    use sokonanoda_front::compile::CompileOptions;
    use sokonanoda_front::proof::{parse_expr_text, peel_pi_layers, render_expr};
    use sokonanoda_front::{parse, Command, Expr};

    let d = report
        .decls
        .iter()
        .find(|d| d.span.start.offset <= offset && offset <= d.span.end.offset)?;
    if d.status != DeclStatus::Failed {
        return None;
    }
    let error = d.error.as_ref()?;
    if error.code() != "kernel-rejected" {
        return None;
    }
    // 值文本 = 声明切片里最后一个 `:=` 之后的部分（表达式语法不含 `:=`）。
    let slice = &text[d.span.start.offset..d.span.end.offset];
    let (_head, value) = slice.rsplit_once(":=")?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    // 值自己的 lambda 链 = 判定上下文（学习者命名的 binder 原样可用）。
    let value_expr = parse_expr_text(value).ok()?;
    let mut binders: Vec<sokonanoda_front::judge::GoalBinderSpec> = Vec::new();
    let mut body = &value_expr;
    while let Expr::Lambda {
        binders: bs,
        body: b,
        ..
    } = body
    {
        for b in bs {
            binders.push(sokonanoda_front::judge::GoalBinderSpec {
                name: b.name.clone(),
                ty: b.ty.as_deref().map(render_expr),
            });
        }
        body = b;
    }
    if binders.is_empty() {
        return None; // 没有引入 binder 的半截表达式：诊断已足够
    }
    let term_text = render_expr(body);
    // 声明目标 = 声明类型剥掉值已消耗的层数。
    let file = parse(slice).ok()?;
    let ty = match file.commands.first()? {
        Command::Theorem { ty, .. } | Command::Def { ty, .. } => ty.clone(),
        _ => return None,
    };
    let goal_ty = peel_pi_layers(&ty, binders.len())?;
    let goal_text = render_expr(&goal_ty);

    // 问内核：这一项在上下文里的类型（推断，不是判定；有缓存）。
    let prefix = &text[..d.span.start.offset];
    let inferred = sokonanoda_front::judge::judge_infer(
        prefix,
        &CompileOptions::default(),
        &binders,
        &term_text,
    )
    .ok()?;
    let inferred_expr = parse_expr_text(&inferred).ok()?;
    let mut goals: Vec<String> = Vec::new();
    let mut cur = &inferred_expr;
    loop {
        match cur {
            Expr::Arrow {
                domain, codomain, ..
            } => {
                goals.push(render_expr(domain));
                cur = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                for binder in binders {
                    goals.push(render_expr(
                        binder.ty.as_deref().unwrap_or_else(|| body.as_ref()),
                    ));
                }
                cur = body;
            }
            _ => break,
        }
    }
    if goals.is_empty() {
        return None; // 不是部分应用：诊断已足够
    }
    let codomain = render_expr(cur);
    let (headline, tail) = if codomain == goal_text {
        (
            format!(
                "这一项的结论已经对上目标：\n{}\n还差 {} 个前提：",
                code_block(&goal_text),
                goals.len()
            ),
            "\n\n继续把前提补上，或用 `by` / `fun` 继续写。".to_string(),
        )
    } else {
        (
            format!(
                "这一项的类型是：\n{}\n与目标对不上：\n{}",
                code_block(&inferred),
                code_block(&goal_text)
            ),
            String::new(),
        )
    };
    let goal_block = code_block(
        &goals
            .iter()
            .map(|g| {
                let runs = sokonanoda_front::semantic::goal_runs(&[], g, decls);
                sokonanoda_front::semantic::runs_to_text(&runs)
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("{headline}\n\n{goal_block}{tail}"),
        }),
        range: Some(range_of(d.span)),
    })
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // 会话根：`rootUri`（旧字段，仍被广泛使用）优先，其次第一个 workspace folder。
        // 单文件打开时两者都可能是 null —— 那不是错误（LSP 明文如此），
        // 此时模块根由每个文档自己的目录决定（零配置退路）。
        let root = params
            .root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().ok())
            .or_else(|| {
                params
                    .workspace_folders
                    .as_ref()
                    .and_then(|folders| folders.first())
                    .and_then(|folder| folder.uri.to_file_path().ok())
            });
        {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.root = root;
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                references_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
                    InlayHintOptions {
                        resolve_provider: Some(false),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                ))),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    completion_item: None,
                }),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(semantic_token_options().into()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.refresh(
            params.text_document.uri,
            params.text_document.text,
            Some(params.text_document.version),
        )
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync delivers the whole text; the last change is the final state.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.refresh(
                params.text_document.uri,
                change.text,
                Some(params.text_document.version),
            )
            .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.refresh(params.text_document.uri, text, None).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // 关掉的文档从表里移除：它不该再被别人的变更"顺带刷新"（否则会给
        // 已关闭的 URI 推送诊断）。客户端自己会清掉该文档的诊断。
        let mut docs = self.doc.lock().expect("doc lock");
        docs.remove(&params.text_document.uri);
    }

    async fn semantic_tokens_full(
        &self,
        _: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        // 始终对当前存储的文本重新计算：解析失败时 front 的
        // semantic_tokens 自身退化为纯词法分类，绝不复用过期报告。
        let text = {
            let doc = self.doc.lock().expect("doc lock");
            doc.text().to_string()
        };
        let spans = front_semantic_tokens(&text);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: encode_semantic_tokens(&text, &spans),
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let doc = self.doc.lock().expect("doc lock");
        if doc.report().is_none() {
            return Ok(None);
        }
        // 请求期探针：洞期望类型（含函数 spine 的 None 项）在 hover 时补齐。
        // 报告（含"哪些子洞要探、怎么对齐"）来自真相层 `QueryDoc::probed_report`。
        let report = doc.query().probed_report();
        let report = &report;
        let pos = params.text_document_position_params.position;
        let offset = position_to_offset(doc.text(), pos);
        // Declaration table computed once per hover request and reused by every
        // goal-block builder below (front goal runs need it for classification).
        let decls = sokonanoda_front::semantic::declaration_kinds(doc.text());
        // `by` tactic hover: show the goal state entering the tactic under the
        // cursor (Lean Infoview-style, user request). Before keyword suppression
        // below, because tactic words (intro/exact/…) are keywords.
        if let Some(hover) = tactic_goal_hover(report, doc.text(), offset, &decls) {
            return Ok(Some(hover));
        }
        // 半截表达式的 goal-state（内核拒绝 + 有可推断的部分应用）。
        // 只在 hover 请求时计算（不在按键路径），judge_infer 有缓存。
        if let Some(hover) = half_expression_goals_hover(report, doc.text(), offset, &decls) {
            return Ok(Some(hover));
        }
        // 关键字（fun/=>/theorem/axiom…）上不吐类型行：那一行的悬停信息
        // 应该来自名字/表达式，而不是把关键字所在的某个节点硬塞过来。
        if let Some(kind) = semantic_kind_at(doc.text(), pos.line, pos.character) {
            if matches!(kind, SemanticKind::Keyword) {
                return Ok(None);
            }
        }
        // 括号优先：光标在 ( / ) 上 → 显示括号组包住的表达式及其类型
        //（`(表达式)` 的悬停 = `表达式 : 类型`）。必须先于精确命中——
        // 外层 lambda 行的 span 覆盖整个值表达式，会遮住括号组。
        if let Some(res) = bracket_hover(doc.text(), &report.hovers, pos.line, pos.character) {
            return Ok(Some(hover_markup(res)));
        }
        if let Some(h) = hover_type_at(&report.hovers, pos.line, pos.character) {
            // 学习者需求：显示「表达式 : 类型」——表达式从源码按 span 切片
            //（括号平衡成良构），并返回表达式范围供编辑器高亮。
            return Ok(Some(hover_markup(expr_hover(doc.text(), h))));
        }
        // 邻近回退：光标 ±2 字符内命中的最小外层表达式（运算符、空白
        // 边缘等结构符号也能看到所属类型）。数据来自 hover 表（span 嵌套）。
        {
            const TOLERANCE: usize = 2;
            let nearest = report
                .hovers
                .iter()
                .filter(|h| {
                    let start = h.span.start.offset;
                    let end = h.span.end.offset;
                    // 精确包含（已被上面处理，这里补边界附近）
                    start <= offset + TOLERANCE && offset.saturating_sub(TOLERANCE) < end
                })
                .min_by_key(|h| {
                    let len = h.span.end.offset - h.span.start.offset;
                    let dist = if offset >= h.span.start.offset && offset <= h.span.end.offset {
                        0
                    } else if offset < h.span.start.offset {
                        h.span.start.offset - offset
                    } else {
                        offset - h.span.end.offset
                    };
                    (dist, len)
                });
            if let Some(h) = nearest {
                return Ok(Some(hover_markup(expr_hover(doc.text(), h))));
            }
        }
        if let Some(d) = decl_at(&report.decls, pos.line, pos.character) {
            let signature = match &d.ty_text {
                Some(ty) => format!("{} {} : {}", d.kind.as_str(), decl_name(d), ty),
                None => format!("{} {}", d.kind.as_str(), decl_name(d)),
            };
            // Signature and goal state are `.sokonanoda` text → fenced blocks so
            // the editor highlights them (docs/design/goal-rendering.md §7).
            let mut value = code_block(&signature);
            match d.status {
                DeclStatus::Open => {
                    // 光标正落在某个 `sorry` 上：先给这个洞的精确期望类型
                    //（超量应用走查经 def 展开算出，如 `(And.right a (Not a)
                    // x) sorry` 的洞期望 `a`，而不是整个声明类型）。
                    let hole_ty = d
                        .sub_goals
                        .iter()
                        .find(|s| s.span.start.offset <= offset && offset <= s.span.end.offset);
                    match (&d.goal, hole_ty) {
                        (Some(goal), Some(sg)) if sg.ty.is_some() => value.push_str(&format!(
                            "\n此处 `sorry` 的期望类型：\n{}\n\n剩余目标：\n{}",
                            code_block(sg.ty.as_deref().unwrap_or_default()),
                            goal_block(&decls, &d.binders, goal)
                        )),
                        (Some(goal), _) => {
                            value.push_str(&format!(
                                "\n目标：\n{}",
                                goal_block(&decls, &d.binders, goal)
                            ));
                        }
                        (None, _) => value.push_str("\n待作答"),
                    }
                    value.push_str("\n\n在 `sorry` 处填写一个类型为目标的项。");
                }
                DeclStatus::Checked => value.push_str("\n\n已通过内核检查"),
                DeclStatus::Failed => value.push_str("\n\n未通过，见诊断"),
            };
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(range_of(d.span)),
            }));
        }
        Ok(None)
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        // 优先级可视化（学习者需求）：光标放在某个符号/运算符上，
        // 逐级放大选中"先结合"的表达式。数据来自 hover 表——每个
        // AST 节点（含箭头/应用）都有行，span 天然嵌套。
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        let mut out = Vec::with_capacity(params.positions.len());
        for pos in &params.positions {
            let offset = position_to_offset(doc.text(), *pos);
            let mut rows: Vec<&HoverType> = report
                .hovers
                .iter()
                .filter(|h| {
                    h.span.start.offset <= offset
                        && offset < h.span.end.offset.max(h.span.start.offset + 1)
                })
                .collect();
            // 内层在前（span 小的先选中）；同一 span 只留一条。
            rows.sort_by_key(|h| h.span.end.offset - h.span.start.offset);
            rows.dedup_by(|a, b| a.span == b.span);
            // SelectionRange 的 root 是最内层选择、parent 向外指：
            // 从最外层往里建链，最后一个处理的（最小 span）成为 root。
            let mut chain: Option<SelectionRange> = None;
            let mut last_range: Option<Range> = None;
            for h in rows.into_iter().rev() {
                let range = range_of(h.span);
                if last_range == Some(range) {
                    continue;
                }
                last_range = Some(range);
                chain = Some(SelectionRange {
                    range,
                    parent: chain.map(Box::new),
                });
            }
            out.push(chain.unwrap_or(SelectionRange {
                range: Range {
                    start: *pos,
                    end: *pos,
                },
                parent: None,
            }));
        }
        Ok(Some(out))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let request_uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus(&request_uri);
        let Some(report) = docs.report() else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let Some(target) = definition_at(&report.hovers, pos.line, pos.character) else {
            return Ok(None);
        };
        // 跨文件：项目模式下目标可能住在被 import 的模块里（I16 P5）。
        // 声明名 → 模块路径由真相层回答（它握着整个闭包的报告）。
        let cross_file = match &target {
            ResolvedTarget::Declaration { name, .. } => docs
                .query()
                .project_definition(name)
                .and_then(|(path, _)| Url::from_file_path(path).ok()),
            ResolvedTarget::Binder(_) => None,
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: cross_file.unwrap_or(request_uri),
            range: range_of(target.span()),
        })))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let Some(ranges) = highlight_uses(&report.hovers, pos.line, pos.character) else {
            return Ok(None);
        };
        Ok(Some(
            ranges
                .into_iter()
                .map(|range| DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::TEXT),
                })
                .collect(),
        ))
    }

    async fn document_symbol(
        &self,
        _: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        let symbols = report
            .decls
            .iter()
            .map(|d| DocumentSymbol {
                name: decl_name(d),
                detail: Some(format!("{} — {}", d.kind.as_str(), status_label(d.status))),
                kind: symbol_kind(d.kind),
                tags: None,
                #[allow(deprecated)] // lsp-types field is deprecated in favor of `tags`
                deprecated: None,
                range: range_of(d.span),
                selection_range: range_of(d.span),
                children: None,
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn code_lens(&self, _: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        let lenses = report
            .decls
            .iter()
            .map(|d| CodeLens {
                range: range_of(d.span),
                command: Some(Command {
                    title: status_label(d.status).to_string(),
                    command: "sokonanoda.status".to_string(),
                    arguments: None,
                }),
                data: None,
            })
            .collect();
        Ok(Some(lenses))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let mut items: Vec<CompletionItem> = Vec::new();
        // In-scope binders at the cursor (smallest enclosing hover row);
        // outside any hover span the list stays keyword/prelude-only.
        let pos = params.text_document_position.position;
        // In-scope binders at the cursor (smallest enclosing hover row);
        // outside any hover span the list stays keyword/prelude-only.
        if let Some(report) = doc.report() {
            if let Some(names) = scope_names_at(&report.hovers, pos.line, pos.character) {
                for name in names {
                    if name.is_empty() {
                        continue; // anonymous arrow binder
                    }
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("本域 binder".to_string()),
                        ..Default::default()
                    });
                }
            }
        }
        // Keywords (single source: front::semantic).
        for keyword in sokonanoda_front::semantic::keywords() {
            items.push(CompletionItem {
                label: (*keyword).to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        // Sorts (Prop / Type / Sort).
        for sort in sokonanoda_front::semantic::sorts() {
            items.push(CompletionItem {
                label: (*sort).to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("宇宙".to_string()),
                ..Default::default()
            });
        }
        // Trusted prelude names (no DeclState exists for them).
        for name in sokonanoda_front::compile::PRELUDE_NAMES {
            items.push(CompletionItem {
                label: (*name).to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("prelude".to_string()),
                ..Default::default()
            });
        }
        // The document's own declarations (anonymous examples excluded).
        if let Some(report) = doc.report() {
            for decl in &report.decls {
                let Some(name) = &decl.name else {
                    continue;
                };
                if name.starts_with('_') {
                    continue; // internal names (_example_N)
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(match decl.kind {
                        sokonanoda_front::compile::DeclKind::Inductive => {
                            CompletionItemKind::STRUCT
                        }
                        sokonanoda_front::compile::DeclKind::Axiom => CompletionItemKind::CONSTANT,
                        _ => CompletionItemKind::FUNCTION,
                    }),
                    detail: Some(format!(
                        "{} · {}",
                        decl.kind.as_str(),
                        status_label(decl.status)
                    )),
                    // Signature as a `sokonanoda` fence so the docs popup is
                    // highlighted like every other surface (§7).
                    documentation: decl.ty_text.as_ref().map(|ty| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: code_block(&format!("{} {} : {ty}", decl.kind.as_str(), name)),
                        })
                    }),
                    ..Default::default()
                });
            }
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn folding_range(&self, _: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        let ranges = report
            .decls
            .iter()
            .filter_map(|d| {
                // 0-based inclusive lines; clamp the end when the span ends
                // at a line start (trailing newline).
                let start = d.span.start.line.saturating_sub(1) as u32;
                let end = if d.span.end.column <= 1 {
                    d.span.end.line.saturating_sub(2)
                } else {
                    d.span.end.line.saturating_sub(1)
                } as u32;
                (start < end).then(|| FoldingRange {
                    start_line: start,
                    end_line: end,
                    kind: Some(FoldingRangeKind::Region),
                    ..Default::default()
                })
            })
            .collect();
        Ok(Some(ranges))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        Ok(actions::code_actions(
            params.text_document.uri.clone(),
            doc.text(),
            doc.mode(),
            report,
            params.range.start,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let doc = self.doc.lock().expect("doc lock");
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        Ok(render::prepare_rename(doc.text(), report, params.position))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let request_uri = params.text_document_position.text_document.uri.clone();
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus(&request_uri);
        let Some(report) = docs.report() else {
            return Err(tower_lsp::jsonrpc::Error::invalid_params(
                "当前文档无法解析，不能改名",
            ));
        };
        let position = params.text_document_position.position;
        // 项目模式：顶层声明的名字在**整个闭包**里改写（I16 P5）。
        // 光标在 binder（局部名字）上 / 单文件文档 → 走下面的单文件路径。
        if let Some(ResolvedTarget::Declaration { name, .. }) =
            sokonanoda_front::references::resolve_at(
                &report.hovers,
                position.line,
                position.character,
            )
        {
            if let Some(views) = project_views(&docs) {
                if views.len() > 1 {
                    render::ensure_valid_new_name(&params.new_name)?;
                    if project_refs::declared_elsewhere(&views, &params.new_name, &request_uri) {
                        return Err(tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "「{}」在这个项目里已经有同名声明——改名的结果会是重名错误",
                            params.new_name
                        )));
                    }
                    let edits = project_refs::rename_edits(&views, &name, &params.new_name);
                    if edits.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(WorkspaceEdit {
                        document_changes: Some(DocumentChanges::Edits(edits)),
                        ..Default::default()
                    }));
                }
            }
        }
        render::rename(docs.text(), docs.version(), report, params)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let request_uri = params.text_document_position.text_document.uri.clone();
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus(&request_uri);
        let Some(report) = docs.report() else {
            return Ok(None);
        };
        let position = params.text_document_position.position;
        if let Some(ResolvedTarget::Declaration { name, .. }) =
            sokonanoda_front::references::resolve_at(
                &report.hovers,
                position.line,
                position.character,
            )
        {
            if let Some(views) = project_views(&docs) {
                if views.len() > 1 {
                    return Ok(Some(project_refs::references(
                        &views,
                        &name,
                        params.context.include_declaration,
                    )));
                }
            }
        }
        Ok(render::find_references(
            request_uri,
            docs.text(),
            report,
            position,
            params.context.include_declaration,
        ))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let doc = self.doc.lock().expect("doc lock");
        if doc.report().is_none() {
            return Ok(None);
        }
        // 请求期探针补齐函数 spine 子洞的期望类型（inlay 是惰性请求）：
        // 同 hover，直接用真相层的 `QueryDoc::probed_report`。
        let report = doc.query().probed_report();
        let _ = params.range;
        Ok(Some(inlay::document_hints(doc.text(), &report)))
    }
}

/// Run the LSP server over stdio. Library entry so the single `sokonanoda`
/// binary can host the server via `sokonanoda lsp` (gleam pattern); the
/// `sokonanoda-lsp` binary calls this too. stdout carries only LSP frames.
#[tokio::main]
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::build(Backend::new)
        .custom_method("soko/goals", Backend::goals)
        .custom_method("soko/nextHole", Backend::next_hole)
        .custom_method("soko/hints", Backend::hints)
        .custom_method("soko/stateAt", Backend::state_at)
        .custom_method("soko/version", Backend::version)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
