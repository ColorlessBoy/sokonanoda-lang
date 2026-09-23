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
    GoalDeclInfo, GoalsParams, GoalsResponse, NextHoleParams, ProjectParams, ProjectResponse,
    StateAtParams, StateAtResponse,
};
use render::{
    bracket_hover, decl_at, definition_at, diagnostic_from_compile, diagnostic_from_parse,
    expr_hover, highlight_uses, hover_type_at, range_of, scope_names_at, semantic_kind_at,
    status_label, symbol_kind,
};
use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{
    prelude_mode_from_source, CompileOptions, DeclState, DeclStatus, DocumentReport, GoalBinder,
    HoverType, PreludeMode, ResolvedTarget,
};
use sokonanoda_front::project::cache as project_cache;
use sokonanoda_front::query::{decl_name, QueryDoc};
use sokonanoda_front::semantic::{semantic_tokens as front_semantic_tokens, SemanticKind};
use sokonanoda_front::Span;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    /// 上一次**发出去**的诊断：只有真的变了才再发。多文档项目里一次通知可能
    /// 让好几份文档重新编译，但"没变的就别发"能省掉大量无谓的 publish
    /// （也避免服务端在测试/慢客户端上被自己的通知堵住）。
    published: Vec<Diagnostic>,
    /// 已经交给编译任务、但还没装回的文本（T-A30）。
    ///
    /// **为什么需要它**：编译在别的任务里跑，而"最新文本"必须**同步**可见——
    /// ① 后续 `did_change` 的"文本没变就短路"（T-A21）要比的是它；
    /// ② `did_change_watched_files` 判断"要不要重编"读的也是它；
    /// ③ 编译期间到达的只读请求看的是**上一次完成的状态**（`doc`），不是它
    ///    ——这是有意的（clangd：用此刻手上有的那一份），但"还在编什么"要看得见。
    pending_text: Option<String>,
    /// 上面那份文本对应的 LSP 版本（装回时的版本校验用）。
    pending_version: i32,
    /// 上一次编译用的**闭包摘要**（T-A30）。
    ///
    /// **项目文档的短路判据是它，不是文本**：依赖可能在**磁盘上**被改了
    /// （`git checkout` / 另一个编辑器），这份文档自己的文本一个字节没变，但闭包
    /// 结果会变——只看文本会把旧诊断一直显示下去（`docs/design/lsp-edit-concurrency.md`
    /// §6 坑③）。摘要只**读文件 + 哈希**，比"重编一遍闭包"便宜三个数量级。
    /// `None` = 单文件文档（文本 + 模式本身就决定结果）。
    compiled_digest: Option<String>,
}

impl Doc {
    fn new() -> Self {
        Self {
            doc: QueryDoc::new(),
            published: Vec::new(),
            pending_text: None,
            pending_version: 0,
            compiled_digest: None,
        }
    }

    /// 这份文档现在该发的诊断。
    ///
    /// 两条契约并存（G-20 / X15）：
    /// - **老契约**（单文件模式 / 闭包也失败）：parse 错误优先——parse 不过时报告
    ///   是空的，发它等于什么都不说。
    /// - **新契约**（闭包编译成功）：以**闭包报告**为准。用库记法的单元单文件
    ///   必然 parse 失败（记法随 `import` 传播，G-04 第二刀），而闭包是好的；
    ///   此时发 parse 错误就是**假诊断**（编辑器里一条 `notation-unknown-symbol`
    ///   红波浪线，CLI 判卷却 exit 0）。
    fn diagnostics(&self) -> Vec<Diagnostic> {
        let rescued = self.doc.project_entry_compiled();
        if !rescued {
            if let Some(diag) = self.doc.parse_error.as_ref() {
                return vec![diagnostic_from_parse(diag)];
            }
        }
        self.doc
            .report
            .as_ref()
            .map(report_diagnostics)
            .unwrap_or_default()
    }

    /// 当前文本（`doc.text()` 的读法）。
    fn text(&self) -> &str {
        &self.doc.text
    }

    /// **最新已知**文本：有在编的就用待编的那份。
    ///
    /// 与 [`Self::text`] 的分工：`text` 是**上一次编译用的**文本（它与 `report`
    /// 同源，handlers 读它才不会看到"新文本 + 旧报告"）；`latest_text` 是**用户
    /// 缓冲区里**的文本，只有"决定还要不要再编一次"的地方该用它——用 `text`
    /// 会把过时文本排进编译。
    fn latest_text(&self) -> &str {
        self.pending_text.as_deref().unwrap_or(&self.doc.text)
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
        overlay: &[(std::path::PathBuf, String)],
    ) {
        let mode = mode.unwrap_or(self.doc.mode);
        let options = CompileOptions { prelude: mode };
        // 有 `import` 的文档走**项目闭包**：单文件缓存键会张冠李戴（依赖不在
        // 键里），所以这里既不复用也不写入单文件缓存（I16 P5）。
        let has_imports = sokonanoda_front::project::is_project_source(text);
        // **闭包摘要先算**（T-A30）：项目文档的短路判据是它，不是文本——依赖可能
        // 在磁盘上被改了，文本没变但结果会变。它只读文件 + 哈希（毫秒级），而
        // 短路省下的是一次整闭包编译（秒级）。`cfg!(test)` 只挡**缓存读写**，
        // 不挡摘要本身（摘要不碰缓存目录）。
        let project_digest = if !has_imports {
            None
        } else {
            path.as_deref().map(|entry| {
                let (_, digest) = project_cache::plan(entry, Some(text), None, overlay, &options);
                digest
            })
        };
        // **文本、prelude 模式、入口路径、依赖覆盖都没变 ⇒ 不重编**（A7 / T-A21）。
        // 保存（`didSave`）与编辑器外改动（`workspace/didChangeWatchedFiles`）
        // 会带着**完全相同的文本**再走一遍这里——以前那会重编整个闭包
        // （实测 unit08：356ms，而它一个字节都没变）。
        // 覆盖也要比：依赖的未落盘编辑会改这份文档的闭包结果。
        if self.doc.text == text
            && self.doc.mode == mode
            && self.doc.path == path
            && self.doc.overlay_matches(overlay)
            && project_digest
                .as_deref()
                .is_none_or(|digest| self.compiled_digest.as_deref() == Some(digest))
        {
            self.doc.version = lsp_version as u64;
            return;
        }
        self.doc.path = path;
        // `root` 留给 CLI 的 `--root`；编辑器一律走发现规则（见 `entry_path`）。
        self.doc.root = None;
        // 项目文档（有 `import`）走**项目缓存**（T-A10）：键是
        // `ProjectPlan::digest`（拓扑序上每模块的源 + import 边 + prelude 模式
        // + 入口路径 + **依赖的内存覆盖**），命中即回放——诊断、逐模块报告、
        // 事件全都来自条目，**不重编**。这是"打开变快"的开关：
        // 实测 unit08 冷开 4.8s，命中之后是毫秒级。
        //
        // `overlay` 必须参与摘要：依赖的未落盘编辑会改变这份文档的闭包结果，
        // 不折进键里就会错命中（回放出一份按旧依赖算的报告）。
        //
        // `cfg!(test)` 时**不碰真实缓存**：单元测试并行跑，共享缓存目录会互相
        // 污染（既有纪律）。判据走真进程（`docs/gaps/repro/G25-…`）。
        //
        // 摘要**只算一次**，读（T-A10）与写（T-A11）共用同一个键。
        let cached = if cfg!(test) {
            None
        } else if let Some(digest) = &project_digest {
            project_cache::load(digest, &options)
        } else {
            // 单文件条目形状不变（`project` 恒为 `None`）。
            cache::load(text, &options)
        };
        if let Some(entry) = cached {
            self.doc.text = text.to_string();
            // 会话保持原样：`Session::update` 自己会在 prelude 模式变化时重置
            // （它按整文件重新判定模式），下一次未命中缓存时自愈。
            self.doc.mode = mode;
            self.doc.version = lsp_version as u64;
            // 项目条目连**整份报告**一起回放（T-A03）：跨文件能力
            // （definition/references/rename/`soko/project`）读的是模块表。
            let output = entry.output.unwrap_or_default();
            self.doc.set_cached_entry(
                text,
                lsp_version as u64,
                entry.report,
                output,
                entry.project,
            );
            self.compiled_digest = project_digest;
            return;
        }
        // 原地复用会话（I8 增量的关键）：prelude 模式变化时由真相层重建。
        self.doc
            .set_text_with_overlay(text, lsp_version as u64, Some(mode), overlay);
        self.compiled_digest = project_digest.clone();
        if self.doc.parse_error.is_some() && !self.doc.project_entry_compiled() {
            // LSP 既有契约：parse 失败时**没有报告**（hover / documentSymbol /
            // codeAction / inlayHint 等据此回答 `null`）。真相层用"空报告 +
            // parse_error"表达同一件事，这里把它折回 LSP 形状。
            //
            // **例外**（G-20 / X15）：项目闭包**编译成功**时报告是真的、有用的
            // （入口单文件 parse 失败只是因为记法来自 `import`，见
            // `QueryDoc::project_entry_compiled`），丢掉它会让编辑器发假诊断
            // 并把 hover/documentSymbol 全部打成 `null`。这条例外只在闭包好使时
            // 生效——闭包也失败（入口 `LoadFailed`）时仍走老契约。
            self.doc.report = None;
            return;
        }
        // **写回项目缓存**（T-A11）：不写的话，只有"用户先跑过 CLI `build`"
        // 才享受得到 T-A10 的命中——第一次打开仍然白编，而且那份成果没人存。
        // 判据与 CLI 的 `check`/`build`/`query` 同一条（`store_if_clean`
        // 内部的 `ProjectReport::is_clean`）。
        if let Some(digest) = &project_digest {
            if let Some(project) = self.doc.project_report_ref() {
                let clean = project.is_clean();
                project_cache::store_if_clean(digest, &options, project, clean);
            }
        }
        if !cfg!(test) && !has_imports {
            if let Some(report) = &self.doc.report {
                cache::store(
                    text,
                    &options,
                    &CachedCompile {
                        report: report.clone(),
                        output: None,
                        // 单文件条目：没有项目报告（T-A03 起项目条目才带它）。
                        project: None,
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
    active: Option<Url>,
}

impl Docs {
    fn new() -> Self {
        Self {
            map: std::collections::HashMap::new(),
            order: Vec::new(),
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

    fn remove(&mut self, uri: &Url) {
        self.map.remove(uri);
        self.order.retain(|item| item != uri);
        if self.active.as_ref() == Some(uri) {
            self.active = self.order.first().cloned();
        }
    }

    /// 打开文档的**内存覆盖**（依赖的未保存编辑对闭包编译可见，I16 P5）。
    ///
    /// `text` 是**这份**文档的待编文本：map 里它还是上一版，必须换掉，否则
    /// 下游重编译看到的还是上一版依赖（实测踩过：改了依赖但入口没反应）。
    fn overlay_for(&self, uri: &Url, text: &str) -> Vec<(std::path::PathBuf, String)> {
        let mut overlay: Vec<(std::path::PathBuf, String)> = self
            .order
            .iter()
            .filter_map(|open| {
                let path = open.to_file_path().ok()?;
                let doc = self.map.get(open)?;
                (!doc.latest_text().is_empty()).then(|| (path, doc.latest_text().to_string()))
            })
            .collect();
        if let Ok(changed) = uri.to_file_path() {
            if let Some(entry) = overlay
                .iter_mut()
                .find(|(path, _)| same_file(path, &changed))
            {
                entry.1 = text.to_string();
            }
        }
        overlay
    }

    /// 闭包里含 `changed` 的**其它**已打开文档（T-A23 跨文件失效的扇出面）。
    ///
    /// 只挑真的受影响的：多文档项目里"改 A 也重编译 B"是常态，但只有闭包里
    /// 含这份改动的才值得重编。
    fn stale_downstream(&self, changed: &Url) -> Vec<Url> {
        let Ok(changed_path) = changed.to_file_path() else {
            return Vec::new();
        };
        self.order
            .iter()
            .filter(|other| *other != changed)
            .filter(|other| {
                self.map
                    .get(other)
                    .and_then(|doc| doc.query().project_modules())
                    .is_some_and(|modules| {
                        modules
                            .iter()
                            .any(|module| same_file(&module.path, &changed_path))
                    })
            })
            .cloned()
            .collect()
    }

    /// 请求入口：把活跃文档切到请求指向的那份（带 URI 的请求都该先调它）。
    ///
    /// 多文档下"活跃文档"只是没有 URI 时的回退；每个请求都必须显式指向自己的
    /// 文档，否则会答出另一份文档的 hover / goals / 符号表。
    fn focus_request(&mut self, uri: &Url) {
        self.focus(uri);
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

    fn mode(&self) -> PreludeMode {
        self.active().map(Doc::mode).unwrap_or(PreludeMode::Full)
    }

    fn version(&self) -> i32 {
        self.active().map(Doc::version).unwrap_or(0)
    }
}

/// 同一个文件？按 `canonicalize` 比较（macOS 上 `/var` 与 `/private/var`
/// 是两种写法；`didOpen` 的 URI 与解析器拼出来的路径不保证逐字相同）。
fn same_file(left: &std::path::Path, right: &std::path::Path) -> bool {
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
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
    /// **共享**（`Arc`）而不是内嵌：编译要 `tokio::spawn` 出去，而 spawn 的
    /// future 必须 `'static`——`&self` 借不到。三样东西各自 `Arc` 一份给任务。
    doc: Arc<Mutex<Docs>>,
    compile: Arc<Compiler>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            doc: Arc::new(Mutex::new(Docs::new())),
            compile: Arc::new(Compiler::new()),
        }
    }

    /// 记下"这份文档有新文本"，必要时起一个编译任务。**不 await 编译**（T-A30）。
    ///
    /// 这是 `did_open` / `did_change` / `did_save` / `did_change_watched_files`
    /// 的唯一入口：handler 到这里就返回，编译在别的任务里跑，`Docs` 锁**不跨
    /// 编译**——所以编译期间到达的只读请求（`soko/stateAt` / hover / 目标栏）
    /// 读到的是**上一次完成的状态**，而不是干等（clangd：「用此刻手上有的那一份」）。
    ///
    /// **文本立即落进文档**（`Doc.pending_text`）：后续编辑的"文本没变就短路"
    /// （T-A21）与 `did_change_watched_files` 的"要不要重编"都读它，读到的必须
    /// 是最新那版。
    fn schedule_refresh(&self, uri: Url, text: String, version: Option<i32>) {
        let lsp_version = {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_or_open(&uri);
            let lsp_version = version.unwrap_or_else(|| docs.version());
            let Some(doc) = docs.map.get_mut(&uri) else {
                return;
            };
            doc.pending_text = Some(text.clone());
            doc.pending_version = lsp_version;
            lsp_version
        };
        if self.compile.schedule(uri.clone(), text, lsp_version, true) {
            let client = self.client.clone();
            let docs = Arc::clone(&self.doc);
            let compile = Arc::clone(&self.compile);
            tokio::spawn(compile_worker(uri, client, docs, compile));
        }
    }
}

/// 在飞编译的调度（T-A30）。
///
/// **为什么要有它**（实测，`docs/PERF.md`）：编译以前在 `Mutex<Docs>` 里同步跑，
/// 8.9 秒的冷编译期间第一个 `soko/stateAt` 等了 **8907ms**——整个编辑器像死了一样。
/// 两条独立的病叠在一起：① 编译占着 `Docs` 锁 ⇒ 只读请求全被挡住；② handler
/// `await` 着编译 ⇒ LSP 的**消息循环本身**也堵住（tower-lsp 串行处理请求）。
///
/// **修法**（clangd 的 `TUScheduler`，设计 `docs/design/lsp-edit-concurrency.md`）：
/// * `did_change` 只把「最新文本 + 版本」记进 `pending` 就返回；
/// * 编译由 **spawn 出去的任务**做，**不持 `Docs` 锁**；
/// * **防抖**：等一个静默期再开编；静默期里来了更新的版本就接着等
///   （消灭"敲 7 个字母编 7 次"）；
/// * 结果**按版本号**校验后装回；更新的版本已经在路上就丢掉这份
///   （clangd 的 "writes immediately followed by writes"）。
struct Compiler {
    /// 待编：`did_change` 同步写、编译任务取走。
    pending: Mutex<HashMap<Url, Job>>,
    /// 每份文档的**编译载体**：长期携带增量会话（I8），只有编译任务碰它。
    ///
    /// 为什么不与 `Docs` 里那份合并成一份：编译期间 handlers 要读到**完整**的
    /// 上一次状态，所以编译不能就地改它。两份 `Doc`，编译完**整体互换**
    /// （零克隆，见 `install`）——这也顺带绕开了上一版的两个坑：载体走的就是
    /// `Doc::set_text` 本身，所以**读缓存 / 写缓存 / `parse_error` 折叠**一样不少
    /// （上一版另起一条编译路径，把读缓存漏了 ⇒ 热开 8ms 退成 810ms）。
    carriers: Mutex<HashMap<Url, Doc>>,
    /// 有任务在飞的 URI：同一份文档不并发编译（否则增量会话会被两个任务同时用）。
    inflight: Mutex<HashSet<Url>>,
    /// 静默期。`SOKO_DEBOUNCE_MS` 可覆盖（测试用 0）。
    debounce: Duration,
    /// 每份文档**上一次编译**的耗时。
    ///
    /// **防抖只对"重建慢的文件"生效**（clangd 的原话：debouncing is applied for
    /// files whose rebuild is slow）。小文件立刻编，"编辑→诊断"的可感延迟不受
    /// 影响；大闭包才等静默期。一刀切地防抖会把每次编辑的诊断都推迟 120ms
    /// ——那是拿**反馈延迟**换**不冻结**，对小文件纯亏。
    cost: Mutex<HashMap<Url, Duration>>,
}

/// "重建慢"的门槛：上一次编译超过它，下一次编辑就等静默期。
const SLOW_REBUILD: Duration = Duration::from_millis(150);

/// `SOKO_LSP_TRACE=1` 时每次编译打一行（读一次就缓存——它在每次编译的收尾）。
fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SOKO_LSP_TRACE").is_some())
}

struct Job {
    text: String,
    version: i32,
    /// **总是发**这份文档的诊断（即使与上一次发的一模一样）。
    ///
    /// 被打开/改动的文档走 `true`：客户端要收到"这次是干净的"这个**信号**
    /// （空数组也是有意义的答复），而 `published` 初值是空数组 ⇒ 只看"变了没"
    /// 会把首次打开的空诊断吞掉。下游扇出（依赖改了顺带重编的文档）走 `false`
    /// ——那些不该重复打扰客户端（既有契约：publish-on-change）。
    always: bool,
}

impl Compiler {
    fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            carriers: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashSet::new()),
            debounce: debounce_from_env(),
            cost: Mutex::new(HashMap::new()),
        }
    }

    /// 这份文档这一次该等多久的静默期（0 = 不等）。
    fn debounce_for(&self, uri: &Url) -> Duration {
        let slow = self
            .cost
            .lock()
            .expect("cost lock")
            .get(uri)
            .is_some_and(|last| *last >= SLOW_REBUILD);
        if slow {
            self.debounce
        } else {
            Duration::ZERO
        }
    }

    fn record_cost(&self, uri: &Url, cost: Duration) {
        self.cost
            .lock()
            .expect("cost lock")
            .insert(uri.clone(), cost);
    }

    /// 记下待编；返回 `true` 表示**该起任务**（此前没有在飞的）。
    ///
    /// 锁序固定为 `inflight → pending`（`keep_going` 同序），避免死锁。
    fn schedule(&self, uri: Url, text: String, version: i32, always: bool) -> bool {
        let mut inflight = self.inflight.lock().expect("inflight lock");
        self.pending.lock().expect("pending lock").insert(
            uri.clone(),
            Job {
                text,
                version,
                always,
            },
        );
        // 已有任务在飞：它下一轮循环会取走这条 pending。
        inflight.insert(uri)
    }

    /// 任务退出前的收尾：还有待编就**留下继续跑**（返回 `true`），否则摘掉
    /// "在飞"标记（返回 `false`）。
    ///
    /// 两步必须在**同一把锁**下判定：否则"摘标记"与"新调度插入"之间有窗口——
    /// 要么起两个任务并发编译同一份文档（增量会话就废了），要么新活插进来时
    /// 任务已经退出而 `inflight` 还挂着 ⇒ **这份文档从此再也不会被编译**
    /// （实测：`perf_course_watched_unchanged_file_is_recorded` 第 2 轮起卡死）。
    fn retire(&self, uri: &Url) -> bool {
        let mut inflight = self.inflight.lock().expect("inflight lock");
        if self.pending.lock().expect("pending lock").contains_key(uri) {
            return true;
        }
        inflight.remove(uri);
        false
    }

    fn pending_version(&self, uri: &Url) -> Option<i32> {
        self.pending
            .lock()
            .expect("pending lock")
            .get(uri)
            .map(|job| job.version)
    }

    fn take_job(&self, uri: &Url) -> Option<Job> {
        self.pending.lock().expect("pending lock").remove(uri)
    }

    fn take_carrier(&self, uri: &Url) -> Doc {
        self.carriers
            .lock()
            .expect("carriers lock")
            .remove(uri)
            .unwrap_or_else(Doc::new)
    }

    fn put_carrier(&self, uri: Url, carrier: Doc) {
        self.carriers
            .lock()
            .expect("carriers lock")
            .insert(uri, carrier);
    }

    /// 文档关了：把它的载体与待编一起丢掉（否则会给已关闭的 URI 推诊断）。
    fn forget(&self, uri: &Url) {
        self.pending.lock().expect("pending lock").remove(uri);
        self.carriers.lock().expect("carriers lock").remove(uri);
        self.cost.lock().expect("cost lock").remove(uri);
    }
}

/// 防抖静默期：`SOKO_DEBOUNCE_MS`（毫秒）可覆盖，默认 120ms。
///
/// 为什么默认 120ms：clangd 的判据是"用户停手了"——敲 `foo();` 时每敲一个
/// 字母就重编，会一直看到 `unknown identifier f` 这类**中间态**诊断。
/// 120ms 比人的击键间隔（~80–200ms）短，不会让"停手后"多等。
fn debounce_from_env() -> Duration {
    std::env::var("SOKO_DEBOUNCE_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(120))
}

/// 一份文档的编译任务：防抖 → 锁外编译 → 版本校验 → 装回 → 发布 → 扇出。
///
/// **它不持 `Docs` 锁做编译**（只在取快照与装回时短暂持锁），所以只读请求
/// 不会等它。任务结束时若还有待编（编译期间又来了编辑），循环再来一轮。
async fn compile_worker(uri: Url, client: Client, docs: Arc<Mutex<Docs>>, compile: Arc<Compiler>) {
    loop {
        // ① 防抖：等静默期；期间版本变了就接着等（clangd 的"写紧跟写"）。
        while let Some(seen) = compile.pending_version(&uri) {
            let wait = compile.debounce_for(&uri);
            if wait.is_zero() {
                break;
            }
            tokio::time::sleep(wait).await;
            if compile.pending_version(&uri) == Some(seen) {
                break;
            }
        }
        let Some(job) = compile.take_job(&uri) else {
            // 没有待编 ⇒ 退休。退休与新调度在同一把锁里判定（见 `retire`）；
            // 退休前又插进来一条就接着干，别让文档卡死。
            if compile.retire(&uri) {
                continue;
            }
            return;
        };
        let version = job.version;
        let started = std::time::Instant::now();
        let out = compile_one(&client, &docs, &compile, &uri, job);
        let cost = started.elapsed();
        compile.record_cost(&uri, cost);
        // 常驻诊断（`SOKO_LSP_TRACE=1`）：每次编译一行。它直接回答"编译有没有
        // 挡住消息循环"（行与行之间能插进只读请求的应答）与"防抖有没有生效"
        // （慢文件的下一次编辑会等静默期）。
        if trace_enabled() {
            eprintln!(
                "LSP_TRACE compile {uri} v{version} {}ms publish={}",
                cost.as_millis(),
                out.len()
            );
        }
        for (target, diagnostics, version) in out {
            client
                .publish_diagnostics(target, diagnostics, version)
                .await;
        }
    }
}

/// 三段式的**中段与末段**：锁外编译，回锁内按版本校验后装回、扇出。
///
/// 三段式（设计 §6 的清单）：① 锁内取快照（overlay + 载体）→ ② **锁外**用载体
/// 编译 → ③ 回锁内校验版本、整体互换装回。
///
/// **它不是 `async`**：里面有 `MutexGuard`，而 rustc 的 generator 分析会把
/// "跨 await 仍活着的 guard"判成 future 不 `Send`（实测：只要它是 `async`，
/// `tokio::spawn` 就报 `future cannot be sent between threads safely`，且
/// 指不到具体类型）。发布是唯一需要 await 的事，交给调用方
/// [`compile_worker`] 做——返回值就是"该发什么"。
fn compile_one(
    client: &Client,
    docs: &Arc<Mutex<Docs>>,
    compile: &Arc<Compiler>,
    uri: &Url,
    job: Job,
) -> Vec<(Url, Vec<Diagnostic>, Option<i32>)> {
    // ① 快照：打开文档的内存文本就是编译器该看到的文本（未保存的编辑也算）。
    let overlay = {
        let docs = docs.lock().expect("doc lock");
        docs.overlay_for(uri, &job.text)
    };
    // ② 锁外编译。载体走的就是 `Doc::set_text`：缓存读（命中即回放）、缓存写、
    //    `parse_error` 折叠、T-A21 的"文本没变即短路"，一样不少。
    let mut carrier = compile.take_carrier(uri);
    let mode = prelude_mode_from_source(&job.text);
    let path = uri.to_file_path().ok();
    carrier.set_text(&job.text, job.version, Some(mode), path, &overlay);

    // ③ 装回（短暂持锁）。更新的版本已经在路上 ⇒ 这份作废（clangd 的
    //    "writes immediately followed by writes"），但载体要留着——增量会话
    //    是连续的，下一轮接着用。
    let published = {
        let mut docs = docs.lock().expect("doc lock");
        let superseded = compile
            .pending_version(uri)
            .is_some_and(|newer| newer > job.version);
        if superseded {
            compile.put_carrier(uri.clone(), carrier);
            return Vec::new();
        }
        let Some(committed) = docs.map.get_mut(uri) else {
            // 文档已经关了（`didClose`）：结果没人要，载体也别留。
            return Vec::new();
        };
        // **只复制视图**：载体完整保留"输入 X 的状态"（它下一次编译的短路判据
        // 读的就是它），handlers 读的那一份拿到副本。见 `QueryDoc::adopt_view`。
        committed.doc.adopt_view(&carrier.doc);
        committed.compiled_digest = carrier.compiled_digest.clone();
        committed.pending_text = None;
        committed.pending_version = job.version;
        let diagnostics = committed.diagnostics();
        let changed = job.always || diagnostics != committed.published;
        if changed {
            committed.published = diagnostics.clone();
        }
        // 依赖变了 ⇒ 打开着的下游文档跟着重编（T-A23 跨文件失效）。这里只
        // **调度**它们（各自一个任务），不在本任务里串行编——那会把一次通知
        // 变成 N 次编译的等待。
        let downstream = docs.stale_downstream(uri);
        compile.put_carrier(uri.clone(), carrier);
        (changed.then_some(diagnostics), downstream)
    };

    let mut to_publish: Vec<(Url, Vec<Diagnostic>, Option<i32>)> = Vec::new();
    if let Some(diagnostics) = published.0 {
        to_publish.push((uri.clone(), diagnostics, Some(job.version)));
    }
    for other in published.1 {
        let (text, version) = {
            let docs = docs.lock().expect("doc lock");
            match docs.map.get(&other) {
                Some(doc) => (doc.text().to_string(), doc.version()),
                None => continue,
            }
        };
        if compile.schedule(other.clone(), text, version, false) {
            let client = client.clone();
            let docs = Arc::clone(docs);
            let compile = Arc::clone(compile);
            tokio::spawn(compile_worker(other, client, docs, compile));
        }
    }
    to_publish
}

impl Backend {
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
            // 解析失败（`NotParsable`）与"还没有报告"同答空：LSP 的 parse 诊断
            // 走 `publishDiagnostics`（`Doc::diagnostics` 已经是 parse 优先），
            // `soko/goals` 的 wire 形状不改（G-17 只动 CLI/MCP 的 ok 信封）。
            .unwrap_or_default()
            .into_iter()
            .map(|decl| query_map::decl_info(decl, text))
            .collect();
        Some((text.to_string(), decls))
    }

    async fn goals(&self, params: GoalsParams) -> Result<GoalsResponse> {
        let request_uri = params.text_document.uri.clone();
        let version;
        {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_request(&request_uri);
            version = docs.version();
        }
        let decls = self
            .goal_decls(true)
            .map(|(_, decls)| decls)
            .unwrap_or_default();
        Ok(GoalsResponse {
            decls,
            // 回显请求的文档身份：客户端据此丢弃"答的是另一份文档"的过期响应。
            uri: request_uri.to_string(),
            version,
        })
    }

    /// `soko/project`：这个文档所在闭包的只读状态视图（根、清单来源、模块表、
    /// 每模块状态）。只读派生——不重跑内核、不算摘要（设计 §2 第 5 条）。
    ///
    /// 单文件答 `project: null` + `reason: "no-imports"`：扩展据此显示"单文件"
    /// 占位，而不是把它当成错误。
    async fn project(&self, params: ProjectParams) -> Result<ProjectResponse> {
        let request_uri = params.text_document.uri.clone();
        let (version, project, reason) = {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_request(&request_uri);
            let version = docs.version();
            let query = docs.query();
            let view = query.project_view();
            let reason = view
                .is_none()
                .then(|| query.project_view_reason().to_string());
            (version, view, reason)
        };
        Ok(ProjectResponse {
            uri: request_uri.to_string(),
            version,
            project,
            reason,
        })
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
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
        let forward = params.forward.unwrap_or(true);
        let cursor = position_to_offset(doc.text(), params.position);
        // 解析失败 ⇒ 没有洞可导航（`Ok(None)`），与"这个方向上没有洞"同形：
        // LSP 侧的错误通道是诊断通知，不是 `soko/nextHole` 的应答。
        Ok(doc
            .query()
            .next_hole(cursor, forward)
            .unwrap_or(None)
            .map(|hole| query_map::range_of_offsets(doc.text(), hole.start, hole.end)))
    }

    /// Hint ladder for the declaration at the cursor (docs/design/hints-
    /// suggestions.md). Stateless: the client owns progressive disclosure.
    async fn hints(&self, params: hints::HintsParams) -> Result<hints::HintsResponse> {
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(params.text_document_uri());
        let doc = &*docs;
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
        let request_uri = params.text_document.uri.clone();
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&request_uri);
        let doc = &*docs;
        let version = doc.version();
        let cursor = position_to_offset(doc.text(), params.position);
        match doc.query().state_at(cursor) {
            // 报告缺失 / parse 失败 / 位置不在任何声明内 / 越界：LSP 的 wire 没有
            // 错误通道，既有行为就是空响应（`decl: null` + 默认字段）。
            Err(_) => Ok(StateAtResponse::empty(request_uri.as_str(), version)),
            Ok(answer) => Ok(query_map::state_answer(
                request_uri.as_str(),
                doc.text(),
                answer,
            )),
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
    // 「多余的 `sorry`」（`docs/design/redundant-sorry.md`）：那个洞不是"还没
    // 证出来"，而是多写了一行——由 `redundant-sorry` 那条 warning（带 hint）
    // 说明。声明**全部**洞都属于这种情况时，不再叠一条 "not yet solved"，
    // 否则学生以为是自己没做出来。还有真缺口的声明照旧报。
    let redundant_spans: Vec<Span> = report
        .warnings
        .iter()
        .filter(|w| w.code() == "redundant-sorry")
        .map(|w| w.span)
        .collect();
    // 洞 span 与 warning span 形状未必相同（多余洞走 generic fallback 时洞是
    // 整段值、warning 收窄到 `sorry` token），所以用**包含**判定。
    let hole_is_redundant = |hole: &Span| {
        redundant_spans
            .iter()
            .any(|r| hole.start.offset <= r.start.offset && r.end.offset <= hole.end.offset)
    };
    let all_holes_redundant =
        |d: &DeclState| !d.holes.is_empty() && d.holes.iter().all(hole_is_redundant);
    if report
        .decls
        .iter()
        .any(|d| d.status == DeclStatus::Open && !d.holes.is_empty())
    {
        for d in report.decls.iter().filter(|d| d.status == DeclStatus::Open) {
            if all_holes_redundant(d) {
                continue;
            }
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

/// 记法**符号**的 hover：这个符号是什么、展开成什么、**怎么打出来**。
///
/// 用户要求（原话）：「要考虑 notation 如何输入，应该像 lean4 一样 `\xxx` 替换，
/// 同时 hover 内容提示用户如何输入对应符号」——这条 hover 就是后半句。
/// 设计 `docs/design/notation-input.md` §4。
///
/// 为什么必须插在**关键字闸门之前**：`front::semantic` 把**已声明**的记法符号
/// 归进 `SemanticKind::Keyword`（与 `∀` 同族，见 `semantic.rs` 的
/// `TokenKind::Sym` 分支），而闸门对 `Keyword` 一律 `return Ok(None)`
/// ⇒ 本文件声明的符号今天 hover **完全静默**（实测：`⊗` 无反应、内建 `∧` 有反应、
/// 光标右移一格又有了——三种形态行为不一致）。这条分支把三种形态统一到
/// 「符号 + 展开 + 怎么输入」。
///
/// 信息全部来自**前端**（[`sokonanoda_front::notation_input`] 是唯一真相源），
/// 不在客户端拼；符号作用域用词法扫描判断（不依赖 parse 成功——使用库记法的
/// 文件单文件 parse 必然失败）。类型行**有就给、没有不编**。
fn notation_symbol_hover(
    text: &str,
    offset: usize,
    report: &DocumentReport,
    pos: Position,
    query: &QueryDoc,
) -> Option<Hover> {
    use sokonanoda_front::notation_input;
    // **闭包前缀**（拓扑序里本文件之前的模块）——两处都要用：解析 `import` 来的
    // 记法的展开目标（T-D02），以及问内核要它的签名。
    let prefix = query.judge_prefix(offset);
    let (symbol, target) =
        notation_input::symbol_at_with_sources(text, offset, &[prefix.as_str()])?;
    let locally_declared = notation_input::declared_notation_at(text, offset).is_some();
    let mut lines: Vec<String> = Vec::new();
    let mut head = format!("`{symbol}` —— 记法符号");
    if locally_declared {
        head.push_str("（本文件声明）");
    }
    lines.push(head);
    if let Some(target) = target {
        lines.push(format!("展开成 `{target}`"));
        // **原始类型**（T-D02 / 用户第 6 条反馈："hover 信息也没有对应的原始类型"）：
        // "展开成什么"是一回事，"它本身是什么"是另一回事——学习者点 `∈` 常常
        // 想看的是 `Set.mem` 的签名。走内核问（`judge_type_of_constant`），
        // 它自带**常量键**缓存（`judge.rs` 的 `CACHE`），拿不到就不编这一行。
        //
        // 与 `render::hover_type_at` 的那行（外层表达式的类型）分工不同：
        // 那行说的是"这个表达式 : Prop"，这行说的是"这个符号背后的常量 :
        // 它的签名"。两行都在时先给签名（更能解释"底下站着什么"）。
        let options = sokonanoda_front::compile::CompileOptions {
            prelude: query.mode,
        };
        if let Ok(ty) = sokonanoda_front::judge::judge_type_of_constant(&prefix, &options, &target)
        {
            // 松散变量（`$N`）的文本不可信——与 `render::hover_type_at` 同一条
            // 纪律：拿不到干净的类型就不编。
            if !ty.is_empty() && !ty.contains('$') {
                lines.push(format!("`{target} : {ty}`"));
            }
        }
    }
    match notation_input::input_for(&symbol) {
        Some(entry) if entry.supported => {
            lines.push(notation_input::input_hint(&symbol).unwrap_or_default())
        }
        // 表里有这个符号但语言还没有它 ⇒ **不承诺**可输入。
        Some(_) => lines.push("输入：语言今天还没有这个符号".to_string()),
        // 表外符号（`=`、课程自定义的 `⊗`…）：直说"直接打"——
        // 沉默会让学习者以为有缩写而反复试（Lean 的 hover 也说这句）。
        None => lines.push(format!("输入：直接打 `{symbol}`（语言没有为它约定缩写）")),
    }
    // 外层表达式的类型：能拿到就附上（`a ∈ A : Prop`），拿不到不编。
    if let Some(h) = render::hover_type_at(&report.hovers, pos.line, pos.character) {
        if !h.binder && !h.text.is_empty() && !h.text.contains('$') {
            let expr = text
                .get(h.span.start.offset..h.span.end.offset)
                .unwrap_or_default()
                .trim();
            if !expr.is_empty() {
                lines.push(format!("`{expr} : {}`", h.text));
            }
        }
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: lines.join("\n\n"),
        }),
        // 不给 range：符号的 token span 由词法扫描得出，客户端按光标词高亮即可
        // （与 `hover_markup` 的表达式范围不同——那是 AST span）。
        range: None,
    })
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
        // 工作区根**不**当模块根用（见 `Docs::entry_path`）：同一份项目在
        // 编辑器与 CLI 下必须解析到同一个模块根，否则"编辑器绿、CI 红"。
        let _workspace_root = root;
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
        // **不 await 编译**（T-A30）：记下文本就返回，编译在别的任务里跑。
        // 以前这里同步编完才返回，8.9s 的冷编译期间整个消息循环是堵死的。
        self.schedule_refresh(
            params.text_document.uri,
            params.text_document.text,
            Some(params.text_document.version),
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync delivers the whole text; the last change is the final state.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.schedule_refresh(
                params.text_document.uri,
                change.text,
                Some(params.text_document.version),
            );
        }
    }

    /// **编辑器外**的改动（`git checkout`、脚本、另一个编辑器）：`synchronize.fileEvents`
    /// 已经让客户端在 `**/*.sokonanoda` 变化时发这个通知，服务端此前没有 handler，
    /// 于是打开的文件一直显示旧诊断，要重开才刷新（0.57.0 审计 RISK）。
    ///
    /// 语义：只重编译**闭包里含这个路径**的已打开文档（未打开的文档不产生诊断）；
    /// 缓冲区内容优先——打开着的文档不受磁盘改动影响（VS Code 会在文件重载后自己
    /// 发 didChange）。闭包里的失败/缺失模块也记着期望路径，所以"文件被创建出来"
    /// 同样会触发刷新。
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let changed: Vec<std::path::PathBuf> = params
            .changes
            .iter()
            .filter_map(|event| event.uri.to_file_path().ok())
            .collect();
        if changed.is_empty() {
            return;
        }
        let affected: Vec<(Url, String, i32)> = {
            let docs = self.doc.lock().expect("doc lock");
            docs.order
                .iter()
                .filter_map(|uri| {
                    let doc = docs.map.get(uri)?;
                    let in_closure = doc.query().project_modules().is_some_and(|modules| {
                        modules
                            .iter()
                            .any(|module| changed.iter().any(|path| same_file(&module.path, path)))
                    });
                    // **最新已知**文本：有在编的就用待编的那份，否则会把过时
                    // 文本排进编译（`text()` 是上一次编译用的）。
                    in_closure.then(|| (uri.clone(), doc.latest_text().to_string(), doc.version()))
                })
                .collect()
        };
        if trace_enabled() {
            eprintln!("LSP_TRACE watched: {} affected", affected.len());
        }
        for (uri, text, version) in affected {
            self.schedule_refresh(uri, text, Some(version));
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.schedule_refresh(params.text_document.uri, text, None);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // 关掉的文档从表里移除：它不该再被别人的变更"顺带刷新"（否则会给
        // 已关闭的 URI 推送诊断）。客户端自己会清掉该文档的诊断。
        {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.remove(&params.text_document.uri);
        }
        // 在飞/待编的也一起丢掉（T-A30）：不然那份结果会在文档已经关了之后
        // 才装回来，甚至给已关闭的 URI 推一条诊断。
        self.compile.forget(&params.text_document.uri);
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        // 始终对当前存储的文本重新计算：解析失败时 front 的
        // semantic_tokens 自身退化为纯词法分类，绝不复用过期报告。
        let text = {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_request(&params.text_document.uri);
            docs.text().to_string()
        };
        let spans = front_semantic_tokens(&text);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: encode_semantic_tokens(&text, &spans),
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document_position_params.text_document.uri);
        let doc = &*docs;
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
        // 记法符号（`∧` / 本文件声明的 `⊗` / import 来的 `∈`）：符号 + 展开 +
        // **怎么输入**（用户要求，D5）。必须在下面的关键字闸门**之前**——
        // 已声明的记法符号被 `front::semantic` 归进 `Keyword`，闸门会把它们
        // 一起吞掉（实测：本文件声明的符号 hover 完全静默）。
        if let Some(hover) = notation_symbol_hover(doc.text(), offset, report, pos, doc.query()) {
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
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
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
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document_position_params.text_document.uri);
        let doc = &*docs;
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
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
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

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
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
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document_position.text_document.uri);
        let doc = &*docs;
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

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
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
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
        let Some(report) = doc.report() else {
            return Ok(None);
        };
        Ok(actions::code_actions(
            params.text_document.uri.clone(),
            doc.text(),
            doc.mode(),
            report,
            params.range.start,
            // 项目模式下把闭包前缀交给 suggest：入口里对导入名字也有 quick-fix。
            &|offset| doc.query().judge_prefix(offset),
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
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
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus_request(&params.text_document.uri);
        let doc = &*docs;
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
///
/// **手写 runtime 而不是 `#[tokio::main]`**（T-A30）：编译现在跑在 `tokio::spawn`
/// 出去的 worker 线程上，而 tokio 的 worker 默认栈是 **2MB** —— 编译（深度递归的
/// `elab_expr`）会 `thread 'tokio-rt-worker' has overflowed its stack`。
/// 以前没暴露是因为编译跑在主线程（`block_on`，8MB）。这里给到 32MB。
pub fn run() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(32 * 1024 * 1024)
        .build()
        .expect("build the tokio runtime");
    runtime.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::build(Backend::new)
            .custom_method("soko/goals", Backend::goals)
            .custom_method("soko/nextHole", Backend::next_hole)
            .custom_method("soko/hints", Backend::hints)
            .custom_method("soko/stateAt", Backend::state_at)
            .custom_method("soko/project", Backend::project)
            .custom_method("soko/version", Backend::version)
            .finish();
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}
