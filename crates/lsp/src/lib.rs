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
    GoalAtParams, GoalAtResponse, GoalDeclInfo, GoalsParams, GoalsResponse, NextHoleParams,
    ProjectParams, ProjectResponse, StateAtParams, StateAtResponse,
};
use render::{
    bracket_hover, decl_at, definition_at, diagnostic_from_compile, diagnostic_from_parse,
    expr_hover, highlight_uses, hover_type_at_offset, notation_target_highlight, range_of,
    scope_names_at, semantic_kind_at, status_label, symbol_kind,
};
use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{
    fold_for_display, prelude_mode_from_source, CompileOptions, DeclState, DeclStatus,
    DocumentReport, GoalBinder, HoverType, PreludeMode, ResolvedTarget,
};
use sokonanoda_front::project::cache as project_cache;
use sokonanoda_front::query::{decl_name, import_lines, QueryDoc};
use sokonanoda_front::semantic::{semantic_tokens as front_semantic_tokens, SemanticKind};
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
    /// **A5（2026-10-08）**：这一次编译是不是**产物命中**（一趟 pass 都没跑）。
    ///
    /// 为什么需要它：命中那条路（[`Self::set_text`] 的 `set_cached_entry` 分支）
    /// **不跑任何 pass** ⇒ front 的**线程局部库层检查点**（G-29）没被喂热 ⇒
    /// **开档后的第一次编辑**要把整条库闭包重编一遍（实测 unit08：`modules=5`
    /// · 1233ms，而检查点热的同一刀只要 321ms ✗）。
    /// `compile_worker` 据此在**诊断发出之后**起一次后台库层预热
    /// （`sokonanoda_front::project::warm_library_checkpoint`）——
    /// 那一趟是"首次编辑本来就要做的功"的**子集** ⇒ 最坏情况 = 今天 ✓。
    served_from_artifact: bool,
}

impl Doc {
    fn new() -> Self {
        Self {
            doc: QueryDoc::new(),
            published: Vec::new(),
            pending_text: None,
            pending_version: 0,
            compiled_digest: None,
            served_from_artifact: false,
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
        // R-3（T-B5）：**顺便留下模块根** —— 项目条目现在落在
        // `<模块根>/.sokonanoda/compiled/`，读的时候要用它。
        // 之前这里把 plan 丢掉（`let (_, digest)`）⇒ 编辑器读不到 CLI 预热出来的
        // 产物 = "`build` 之后打开"变慢 ✗（那是性能退化，不是新功能缺失）。
        let (project_root, project_digest) = if !has_imports {
            (None, None)
        } else {
            match path.as_deref() {
                Some(entry) => {
                    let (plan, digest) =
                        project_cache::plan(entry, Some(text), None, overlay, &options);
                    (Some(plan.root.clone()), Some(digest))
                }
                None => (None, None),
            }
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
            // 文本一字没变 ⇒ 这次没有"新编译"，也就没有产物命中可言
            // （检查点状态维持原样 ✓）。
            self.served_from_artifact = false;
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
        } else if let (Some(root), Some(digest)) = (&project_root, &project_digest) {
            project_cache::load_at(root, digest, &options)
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
            // **A5**：命中 = 这一趟**一个 pass 都没跑** ⇒ 库层检查点是冷的。
            self.served_from_artifact = true;
            return;
        }
        // 走到这里 = 真的编了（库层检查点与 judge 缓存都热了）⇒ 不需要预热。
        self.served_from_artifact = false;
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
                // R-3（T-C6）：**磁盘状态的产物落模块根** `<root>/.sokonanoda/` ——
                // 这样编辑器编出来的东西与 CLI 预热出来的**落在同一处**，
                // "vscode 与 code agent 一处取用"才是完整的 ✓。
                // **不变量**：带未落盘编辑（overlay 非空）的摘要仍进**全局缓存** ——
                // 项目目录只放"磁盘状态的产物"（要给人和 agent 读，瞬时条目是噪声）。
                // 判据不是 `overlay.is_empty()`（编辑器里**永远非空** —— 打开文档
                // 本身就带着文本 ✗），而是"**overlay 里每份文本都与磁盘一致**"
                // ⇒ 这份摘要代表的就是磁盘状态 ✓。有未落盘编辑 ⇒ 退回全局缓存。
                let disk_state = overlay.iter().all(|(path, text)| {
                    std::fs::read_to_string(path)
                        .map(|on_disk| &on_disk == text)
                        .unwrap_or(false)
                });
                match (&project_root, disk_state) {
                    (Some(root), true) => {
                        project_cache::store_if_clean_at(root, digest, &options, project)
                    }
                    _ => project_cache::store_if_clean(digest, &options, project),
                }
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

    /// **用户缓冲区那份**（活跃文档）—— `text()` 是"上一次编译用的"，
    /// 两者分工见 `Doc::latest_text` 的注释。纯词法的 handler（语义 token）
    /// 必须读这一份：客户端把结果画到**当前**缓冲区上。
    fn latest_text(&self) -> &str {
        self.active_doc().latest_text()
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
    /// **这份文档"这次编辑是不是接在前一条写后面"**（§11.22，2026-10-09）。
    ///
    /// = `schedule` 那一刻，这份文档**已经有待编或在飞的任务** ✓（clangd 原话
    /// "**写紧跟写**"的字面义）。静默期只保护这一种情况；"打开 → 读一眼 → 敲"里的
    /// **第一刀**没有前一条写可保护 ⇒ 不该白等 120ms ✗（实测 205.6ms 里 ≈120ms 就是它）。
    burst: Mutex<HashMap<Url, bool>>,
}

/// "重建慢"的门槛：上一次编译超过它，下一次编辑就等静默期。
const SLOW_REBUILD: Duration = Duration::from_millis(150);

/// `SOKO_LSP_TRACE=1` 时每次编译打一行（读一次就缓存——它在每次编译的收尾）。
/// **结构计数的快照**（G-29 的判据读数）：模块编译次数 · `by` 引擎调用 ·
/// 类型推断（未命中/调用）· 重跑前缀的趟数。
///
/// 全部来自 front 的**进程级**计数器（`#[doc(hidden)]`，语义是"只给判据用"）——
/// 它们此前只在进程退出时打，而 LSP 不响应 `exit` ⇒ 按键那条路量不到 ✗。
#[derive(Clone, Copy)]
struct StructuralCounters {
    modules: u64,
    by: u64,
    infer_calls: u64,
    infer_miss: u64,
    prefix_runs: u64,
    /// **`TcCache` 构造次数**（2026-10-08 端到端 profiling 的新读数）：每次 `with_tc`
    /// 都新建一份预分配 ≈ 4 MiB + 20 张表的 `TcCache` ⇒ 实测 61.8 µs/次、
    /// 占一次按键编译样本的 **63%**（见 `util::TC_CACHE_BUILDS`）。
    tc_cache_builds: u64,
    /// **telescope 解析次数**（A4b 的判据读数，2026-10-08）：两处 telescope 每次都
    /// `parse_expr_text`、**无 memo** ⇒ 先数"一次按键解析几次"，再谈签名级缓存 ✓。
    telescope_parses: u64,
}

fn structural_counters() -> StructuralCounters {
    let (infer_calls, _hits, infer_miss, prefix_runs, _bytes) =
        sokonanoda_front::judge::infer_totals();
    StructuralCounters {
        // ⚠ **必须是「闭包模块编译次数」**（G-29 第 3 棒 · 2026-10-07 换）✗→✓：
        // `module_compiles_total()` 只在 `check::run` 里按 `units.len()` 累加 ⇒
        // **会话那条路一次都不计** ✗（`project/session.rs` 的库层趟 + 入口趟走
        // `run_pass_with` ✓），却把 judge 的**合成文档**（`compile_fol_with` ⇒
        // `run(units=1)`）算进去 ✗ ⇒ 判据读到的 `modules=` 与"重编了几个模块"**无关**
        // （实测：改一行 `modules=7` 里的 7 **全是**合成编译，真闭包模块 5 一次没计 ✓）。
        // `closure_module_compiles_total()` 的计数点在 `run_pass_with`（**所有 pass 的
        // 唯一收口** ✓）且只数 `path: Some(..)` 的**真模块** ⇒ 老路/会话路**同口径** ✓。
        // ⚠ 判据侧影响：`docs/gaps/repro/G29-…` 的**结构臂**因此读的是真值
        // （会话路重编库层 ⇒ `edit == cold` ⇒ 缺口仍在 ✓，不许被"合成编译不算数"蒙混 ✗）。
        modules: sokonanoda_front::compile::closure_module_compiles_total(),
        by: sokonanoda_front::compile::by_calls_total(),
        infer_calls,
        infer_miss,
        prefix_runs,
        tc_cache_builds: sokonanoda_front::compile::tc_cache_builds_total(),
        telescope_parses: sokonanoda_front::compile::telescope_parses_total(),
    }
}

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
            burst: Mutex::new(HashMap::new()),
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
        // **§11.22**：静默期保护的是"**写紧跟写**"（连打时每键重编会冻结编辑器）。判据不是
        // "距上次多久"（时间窗分不出"开档"与"编辑" —— §11.21 实测那版无效 ✗），而是
        // **这次编辑接在前一条写之后**（`burst`，由 `schedule` 判定 ✓）。
        let bursting = self
            .burst
            .lock()
            .expect("burst lock")
            .get(uri)
            .copied()
            .unwrap_or(false);
        if slow && bursting {
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
        let mut pending = self.pending.lock().expect("pending lock");
        // **§11.22**：**这次编辑到来时，这份文档已经有待编/在飞的任务** ⇒ 用户在**连打**
        // ⇒ 静默期才该生效 ✓（开档后的第一刀、以及"编译已跑完、隔了一会儿又敲"都**不是**
        // 连打 ⇒ 不等 ✓）。这条规则**自调节**：编译比敲键快 ⇒ 队列里总是空的 ⇒ 永不防抖；
        // 编译比敲键慢 ⇒ 队列里总有活儿 ⇒ 自动防抖 ✓✓。
        let burst = pending.contains_key(&uri) || inflight.contains(&uri);
        pending.insert(
            uri.clone(),
            Job {
                text,
                version,
                always,
            },
        );
        drop(pending);
        // 已有任务在飞：它下一轮循环会取走这条 pending。
        let scheduled = inflight.insert(uri.clone());
        self.burst.lock().expect("burst lock").insert(uri, burst);
        scheduled
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

/// **编译专用 runtime**（1 个 worker 线程）：见 `compile_worker` 里调用处的注释
/// （G-29 / 设计 §33 的库层检查点活在**线程局部**里 ⇒ 编译必须钉在一条线程上）。
///
/// 与 `run()` 的 runtime **分开**：主 runtime 要跑消息循环与只读请求（多线程 ⇒
/// 编译不挡它们），而编译要**单线程**（检查点可见性 + 可预测的 32MB 栈）。
fn compile_runtime() -> &'static tokio::runtime::Handle {
    static COMPILE_RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    COMPILE_RT
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                // 与 `run()` 同档：`elab_expr` 递归很深，tokio 默认 2MB 会 overflow ✗。
                .thread_stack_size(32 * 1024 * 1024)
                .enable_all()
                .build()
                .expect("build the compile runtime")
        })
        .handle()
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
        // **编译进度 P1**（2026-09-26 用户需求）：慢文件（或在编辑器外改了依赖）
        // 编译时，编辑器在此之前**一个信号都没有** ⇒ 看起来像"冻住了" ✗。
        // 起止各报一次 `$/progress`：令牌按 **uri** 定（同一文件的连续编译复用
        // 同一个令牌 ⇒ 客户端不会堆出一串假任务 ✓）；**百分比不编**
        // （`percentage: None` ⇒ 界面画"进行中…"，绝不画假进度 ✓）。
        let token = progress_token(&uri);
        send_progress(
            &client,
            &token,
            WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: "sokonanoda".to_string(),
                cancellable: Some(false),
                message: Some(format!("编译 {}", uri.path())),
                percentage: None,
            }),
        )
        .await;
        let started = std::time::Instant::now();
        // **结构计数**（G-29 的判据读数，2026-10-01）：编译前后各取一次差。
        // 为什么要在这里取：**按键那条路的真实成本**只有 LSP 层量得到，而计数
        // 此前只在**进程退出**时打（`atexit`）——LSP 又**不响应 `exit`**
        // （实测：发 `shutdown`/`exit` 后进程不退出）⇒ 外面根本读不到 ✗。
        // 墙钟在共享机器上会翻面（`AGENTS.md`：判据不许用绝对毫秒），计数不会 ✓。
        let counters_before = structural_counters();
        // **编译专用 runtime（1 个 worker 线程）**：G-29 / 设计 §33 的**库层检查点**
        // 活在 front 的**线程局部**里（内核环境借 `&ArenaRef`，而 `ArenaRef` 是
        // `!Send` ⇒ 带检查点的 `QueryDoc` 立刻撞 `tokio::spawn` 的 `Send` 界 ✗）。
        // 检查点只在**同一条线程**上可见 ⇒ 编译必须钉在一条线程上，否则
        // 「开档建的检查点、改一行时找不到」✗。`spawn`（不是 `spawn_blocking`）
        // 是为了拿到与今天同一档的**线程栈**（32MB —— `elab_expr` 递归很深，
        // 2MB 会 overflow，见 `run()` 的注释）。
        let outcome = compile_runtime()
            .spawn({
                let client = client.clone();
                let docs = std::sync::Arc::clone(&docs);
                let compile = std::sync::Arc::clone(&compile);
                let uri = uri.clone();
                async move { compile_one(&client, &docs, &compile, &uri, job) }
            })
            .await
            .unwrap_or_else(|err| std::panic::resume_unwind(err.into_panic()));
        let out = outcome.to_publish;
        // **成对**：`Begin` 之后任何路径都要 `End`（否则客户端那把进度条永远转 ✗）。
        // 这里 `compile_one` 不返回 `Result`，所以顺序执行就够；将来它要是会早退，
        // 必须换成 guard（见缺口台账的纪律：成对通知要能被"漏发"抓住）。
        send_progress(
            &client,
            &token,
            WorkDoneProgress::End(WorkDoneProgressEnd { message: None }),
        )
        .await;
        let cost = started.elapsed();
        compile.record_cost(&uri, cost);
        // 常驻诊断（`SOKO_LSP_TRACE=1`）：每次编译一行。它直接回答"编译有没有
        // 挡住消息循环"（行与行之间能插进只读请求的应答）与"防抖有没有生效"
        // （慢文件的下一次编辑会等静默期）。
        if trace_enabled() {
            // **结构计数**（机器无关）与墙钟一起打：`modules` = 模块编译次数
            // （库层有没有被重编）· `by` = `by` 引擎调用 · `infer` = 类型推断
            // （调用/未命中）· `prefix` = 重跑整份前缀的趟数 · `tc` = `TcCache`
            // 构造次数（每次预分配 ≈ 4 MiB + 20 张表 ⇒ 61.8 µs/次）。判据用计数，
            // 墙钟只做数量级兜底（`AGENTS.md`）。
            let now = structural_counters();
            eprintln!(
                "LSP_TRACE compile {uri} v{version} {}ms publish={} modules={} by={} \
                 infer={}/{} prefix={} tc={} telescope={} reuse={}",
                cost.as_millis(),
                out.len(),
                now.modules - counters_before.modules,
                now.by - counters_before.by,
                now.infer_miss - counters_before.infer_miss,
                now.infer_calls - counters_before.infer_calls,
                now.prefix_runs - counters_before.prefix_runs,
                now.tc_cache_builds - counters_before.tc_cache_builds,
                now.telescope_parses - counters_before.telescope_parses,
                // **诊断（2026-10-09）**：这一趟的**库层从哪来**（`lru`/`artifact`/`prefix`/
                // `rebuilt`/`none` ✓）—— 只读这一行就能回答"走没走到产物那条" ✓
                // （第 50 轮那个**错**结论就是没有这条通道下的 ✗，见 PLAN §41 ✓）。
                sokonanoda_front::project::session::last_lib_source(),
            );
        }
        for (target, diagnostics, version) in out {
            client
                .publish_diagnostics(target, diagnostics, version)
                .await;
        }
        // **A5（2026-10-08）**：产物命中的开档 ⇒ 补一趟**后台库层预热**。
        //
        // 为什么放在这里（诊断**发完之后**）：第一屏是"打开就能看见"，
        // 预热是"第一次按键不要卡" —— 顺序反了就把 1.2s 从按键挪到了开档 ✗。
        // 为什么走 `compile_runtime().spawn`：库层检查点活在 front 的**线程局部**
        // 里，只有**那一条** worker 线程看得见（见 `compile_runtime` 的注释）。
        // 为什么可以先看一眼 `pending_version`：有**待编的编辑**就跳过 ——
        // 那一趟本来就要由它自己的编译做（预热只是提前做，不是额外做）✓。
        if let Some(warm) = outcome.warm {
            let has_pending = compile.pending_version(&uri).is_some();
            let still_open = docs.lock().expect("doc lock").map.contains_key(&uri);
            if !has_pending && still_open && !lib_warmup_disabled() {
                // 观测出口（`SOKO_LSP_TRACE=1`）：**预热自己一行**（前缀与编译那行
                // 不同 ⇒ `compile_count()`/`last_trace()` 那套判据不受影响 ✓）。
                // 判据要读它两件事：① 它出现在 `publishDiagnostics` **之后**
                // （第一屏不被推迟 ✓）；② 它真的建了检查点（`built=true`）。
                let traced = trace_enabled();
                let uri_for_trace = uri.clone();
                compile_runtime().spawn(async move {
                    let started = std::time::Instant::now();
                    let built = sokonanoda_front::project::warm_library_checkpoint(
                        &warm.entry,
                        &warm.text,
                        warm.root.as_deref(),
                        &warm.overlay,
                        &warm.options,
                    );
                    if traced {
                        eprintln!(
                            "LSP_TRACE warm-library {uri_for_trace} built={built} {}ms",
                            started.elapsed().as_millis()
                        );
                    }
                });
            }
        }
    }
}

/// **A5 的逃生门**（默认**开**）：`SOKO_NO_LIB_WARMUP=1` ⇒ 不起后台预热。
///
/// 用途只有一个：**反向验证** —— 撤掉预热后，产物命中那一臂的首次按键
/// `LSP_TRACE` 的 `modules=` **必须回到 5**（判据见 `PLAN-cli-editor-perf.md` §8.2）。
/// 生产路径零影响（不设它 = 预热生效）✓。
fn lib_warmup_disabled() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("SOKO_NO_LIB_WARMUP").is_some())
}

/// **编译进度的令牌**（P1，2026-09-26）：按 **uri** 定 ⇒ 同一文件的连续编译
/// 复用同一个令牌（客户端不会堆出一串假任务 ✓），不同文件各自一条 ✓。
fn progress_token(uri: &Url) -> NumberOrString {
    NumberOrString::String(format!("sokonanoda/compile{}", uri.path()))
}

/// 发一条 `$/progress`（P1）。失败**不影响编译** ✓（结果丢掉即可）。
///
/// ⚠ **两条写在这里免得下一个人踩**（S2 调研实测到源码行，2026-09-26）：
///
/// 1. **不要加"客户端声明了才发"的判断**：`initialize` 从来不保存
///    `params.capabilities`（`lib.rs` 的 initialize 只读 `root_uri`/`workspace_folders`），
///    而且 `tower-lsp 0.20.0` 的 `Client::send_notification`（`service/client.rs:442`）
///    **只看服务端自己的 `State`**、根本不看能力表 ⇒ 这个判断做不了、也没必要 ✓。
///    （本节最初的注释写成"客户端没声明时会失败"，是**错的** —— 已改 ✓。）
/// 2. **这里刻意*不*发 `window/workDoneProgress/create`**，而由扩展**裸读**
///    `$/progress` 自己渲染。两者**互斥**：`vscode-jsonrpc` 的通知处理表按方法名唯一
///    ⇒ 扩展 `client.onNotification("$/progress", …)` 会**覆盖**内建分发器；
///    这时若再发 create，`ProgressPart` 会被造出来却永远收不到 `begin`
///    ⇒ 永不 resolve，而且**每次编译泄漏一个**（`activeParts` 只由 `end→done` 清）。
///    ⇒ 要么 create + 走内建（扩展不裸读），要么**不 create + 扩展裸读**（现状 ✓）。
///    选现状的理由：token 是**按 uri** 定的，且 `compile_one` 的下游扇出编的**不是**
///    当前文档 ⇒ 内建 `client.onProgress` 那条路（要求事先知道 token）**必然被丢** ✓。
async fn send_progress(client: &Client, token: &NumberOrString, value: WorkDoneProgress) {
    let _ = client
        .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
            token: token.clone(),
            value: ProgressParamsValue::WorkDone(value),
        })
        .await;
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
) -> CompileOutcome {
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
            return CompileOutcome::default();
        }
        let Some(committed) = docs.map.get_mut(uri) else {
            // 文档已经关了（`didClose`）：结果没人要，载体也别留。
            return CompileOutcome::default();
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
        // **A5**：产物命中 ⇒ 记下后台预热的材料（诊断发完之后由 `compile_worker`
        // 用）。只对**项目文档**（有入口路径）预热 —— 单文件没有库层 ✓。
        let warm = match (&carrier.served_from_artifact, &carrier.doc.path) {
            (true, Some(entry)) => Some(WarmRequest {
                entry: entry.clone(),
                text: job.text.clone(),
                root: carrier.doc.root.clone(),
                overlay: overlay.clone(),
                options: CompileOptions { prelude: mode },
            }),
            _ => None,
        };
        compile.put_carrier(uri.clone(), carrier);
        (changed.then_some(diagnostics), downstream, warm)
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
    CompileOutcome {
        to_publish,
        warm: published.2,
    }
}

/// `compile_one` 的产物：要发的诊断 + **A5 的后台预热材料**。
#[derive(Default)]
struct CompileOutcome {
    to_publish: Vec<(Url, Vec<Diagnostic>, Option<i32>)>,
    /// `None` = 这次不是产物命中（真编过 ⇒ 检查点已经热了）或不是项目文档。
    warm: Option<WarmRequest>,
}

/// **A5**：后台库层预热的材料 —— 编译时那份输入的**快照**。
///
/// 预热必须喂热**与真编译同一个键**的检查点（`lib_key` 含模块集合/顺序/路径/
/// 源文本/开关）⇒ 这里存的必须是**这一次编译用的**那一份，不能到预热时重算 ✗。
struct WarmRequest {
    entry: std::path::PathBuf,
    text: String,
    root: Option<std::path::PathBuf>,
    overlay: Vec<(std::path::PathBuf, String)>,
    options: CompileOptions,
}

impl Backend {
    // ---- I9 goal 视图协议：结构化 goal 请求（coq-lsp `proof/goals` 模式）----

    /// 组 `soko/goals` 的 wire 数据。`probe` = 是否跑请求期 kernel 探针填
    /// 函数 spine 子洞的期望类型（`nextHole` 只看洞 span，用 `false` 不引入
    /// 内核成本）。
    fn goal_decls(&self, probe: bool, runs: bool) -> Option<(String, Vec<GoalDeclInfo>)> {
        let doc = self.doc.lock().expect("doc lock");
        // LSP 契约：没有报告（尚未编译 / parse 失败）时 `soko/goals` 答空。
        doc.report()?;
        let text = doc.text();
        let decls = sokonanoda_front::query::with_line_index(text, || {
            // 同作用域里再套**折叠缓存**（第 100 轮 ✓）：下面这段映射会为每条开放声明折一次
            // `fold`（实测 ≈0.67ms/13 条 ✗），而作用域内用的是**同一份 doc 的同一张记法表** ✓
            // ⇒ 键只按文本成立 ✓（退出即清 ✓）。
            sokonanoda_front::display::with_fold_cache(|| {
                doc.query()
                    // **候选 B（第 96 轮）**：`runs = false` ⇒ 走 `goals_without_runs` ✓
                    //（只清 `*_runs` ✓；名字/kind/span/文本一字不动 ✓）⇒ 载荷 −72% ✓。
                    .goals_or_without_runs(probe, runs)
                    // 解析失败（`NotParsable`）与"还没有报告"同答空：LSP 的 parse 诊断
                    // 走 `publishDiagnostics`（`Doc::diagnostics` 已经是 parse 优先），
                    // `soko/goals` 的 wire 形状不改（G-17 只动 CLI/MCP 的 ok 信封）。
                    .unwrap_or_default()
                    .into_iter()
                    .map(|decl| query_map::decl_info(decl, text))
                    .collect()
            })
        });
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
            .goal_decls(true, params.runs)
            .map(|(_, decls)| decls)
            .unwrap_or_default();
        Ok(GoalsResponse {
            decls,
            // 回显请求的文档身份：客户端据此丢弃"答的是另一份文档"的过期响应。
            uri: request_uri.to_string(),
            version,
        })
    }

    /// `soko/goalAt`：**光标处那一条**声明的 goal 视图（Lean `$/lean/plainGoal`
    /// 的声明级对应物，设计 `docs/design/persistent-declarations.md` §7.9 ✓）。
    ///
    /// 与 [`Self::goals`] 的关系：形状**逐字段相同**（同一条声明在两条入口上
    /// 必须一模一样 ✓ —— 两份映射就是第二份真相 ✗），只是**只回含光标的那一条** ✓。
    /// 老入口 `soko/goals`（整份声明列表）**一字不动** ✓：它服务的是练习树/Infoview
    /// 的声明列表，那本来就该是整份 ✓。
    ///
    /// 这里与 `goal_decls` 共用同两条加速件（作用域行首索引 + 作用域折叠缓存 ✓）：
    /// 两条入口走**同一份**映射实现，读数才可比 ✓。
    async fn goal_at(&self, params: GoalAtParams) -> Result<GoalAtResponse> {
        let request_uri = params.text_document.uri.clone();
        let version;
        let text;
        let decl;
        {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_request(&request_uri);
            version = docs.version();
            let doc = &*docs;
            // 没有报告（尚未编译 / parse 失败）⇒ 与 `soko/goals` 同答"空"：LSP 的
            // 错误通道是诊断通知（`Doc::diagnostics` 已经 parse 优先），不是这条应答。
            text = doc.text().to_string();
            decl = if doc.report().is_some() {
                let cursor = position_to_offset(&text, params.position);
                sokonanoda_front::query::with_line_index(&text, || {
                    // 与 `goal_decls` 同一个**折叠缓存**作用域（同一份 doc 的同一张
                    // 记法表 ⇒ 键只按文本成立 ✓；退出即清 ✓）。
                    sokonanoda_front::display::with_fold_cache(|| {
                        doc.query()
                            .goal_at(cursor, true)
                            .unwrap_or_default()
                            .map(|decl| query_map::decl_info(decl, &text))
                    })
                })
            } else {
                None
            };
        }
        Ok(GoalAtResponse {
            uri: request_uri.to_string(),
            version,
            decl,
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

    /// 服务器自述：版本 + 进程号。`Sokonanoda: Restart Server (重启服务器)` 用它在重启前后
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
    /// `docs/protocol.md` §`soko/stateAt`). Lean `goalsAt?` semantics: a cursor
    /// **strictly inside** a tactic shows the state **after** it (`goalsAfter`,
    /// Lean's `useAfter := hoverPos > pos`); at the very start it shows the
    /// state **entering** it; otherwise the state after the last tactic that
    /// ended before it. The response carries the document version so clients
    /// drop stale answers.
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
            //
            // **C3（2026-10-08）**：光标停在 `#check`/`#print` 那一行时**正是**"不在
            // 任何声明内" ⇒ 这里仍要把**那一行的命令输出**带上 ✓（否则用户把光标放到
            // `#print myid` 上什么也看不到 ✗ —— 那正是 P5 报的现象）。
            Err(_) => {
                let mut empty = StateAtResponse::empty(request_uri.as_str(), version);
                empty.messages =
                    query_map::state_messages(doc.text(), doc.query().messages_at(cursor));
                Ok(empty)
            }
            Ok(answer) => Ok(query_map::state_answer(
                request_uri.as_str(),
                doc.text(),
                answer,
            )),
        }
    }
}

/// 0-based LSP position → byte offset（与本服务器的 char 计数约定一致）。
/// **G-36 的 trace 包装**（`SOKO_POS_TRACE=1` ✓，纯诊断）：把"输入列 → 输出 offset → 落点字符"
/// 打出来 ✓ —— 用来区分"映射本身错" ✗ 与"映射对了、下游另有偏位" ✗。
pub(crate) fn position_to_offset(text: &str, position: Position) -> usize {
    let out = position_to_offset_impl(text, position);
    if std::env::var("SOKO_POS_TRACE").is_ok() {
        let at = text[out.min(text.len())..]
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "(eof)".to_string());
        eprintln!(
            "[pos] line={} character={} ⇒ offset={} 落点字符={at:?}",
            position.line, position.character, out
        );
    }
    out
}

fn position_to_offset_impl(text: &str, position: Position) -> usize {
    let mut offset = 0usize;
    for (i, line) in text.lines().enumerate() {
        if i == position.line as usize {
            // **G-36 的修（2026-10-03 ✓）**：LSP 的 `character` 是 **UTF-16 码元**偏移 ✓，
            // 不是 `char` 计数 ✗ —— 星平面字符（`𝒫` ✓、emoji ✓）占 **2** 个码元 ✓。
            // 旧实现用 `char_indices().nth(character)` ✗ ⇒ `𝒫` 之后整行偏 1 ✓
            //（实测：`𝒫 A` 的 `A` 在 UTF-16 列 44 ✓，旧实现按第 44 个 `char` 取 ⇒ 落在空格上 ✗）。
            let mut units = 0usize;
            for (idx, ch) in line.char_indices() {
                if units >= position.character as usize {
                    return offset + idx;
                }
                units += ch.len_utf16();
            }
            return offset + line.len();
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
///
/// **光标落在 tactic 里的名字上**（`apply Set.ext` 的 `Set.ext`、`exact h` 的
/// `h`）：goal state 之后再加一条 Markdown 水平线 `---` 与该名字的类型行
/// —— 类型行是**独立的 ```sokonanoda 围栏块**（与 tactic 行 / goal state 同一个
/// [`CODE_LANG`] ⇒ 同样上色），不是行内码（用户 2026-10-10 反馈三 + 实测；
/// 契约 `docs/protocol.md` §Tactic goal-state hover）。
/// 关键字 / 数字 / 括号 / 字符串 / 记法符号上 ⇒ **逐字节不变**（不加、不改顺序）。
fn tactic_goal_hover(
    report: &DocumentReport,
    text: &str,
    offset: usize,
    decls: &[(String, SemanticKind)],
    query: &QueryDoc,
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
    // language as the goal state below, docs/protocol.md §`soko/stateAt`).
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
    // 名字行（用户反馈三）：光标下的名字 ⇒ 分割线 + 它的类型。拿不到干净类型
    // （未知标识符 / 含 `$N` 松散变量）⇒ **不编那一行**（与记法 hover 同一条纪律）。
    //
    // **渲染走同一个围栏**（2026-10-10 用户实测）：这一行以前是**行内码**
    // （`` `Set.ext : …` ``）⇒ VS Code 不给它上色，而同一条 hover 里的 tactic 行
    // 与 goal state 都是 ```sokonanoda 围栏 ⇒ 同一份内容两种观感 ✗。现在它是
    // **独立围栏块**（[`code_block`]，与 tactic 行 / goal state / 声明签名同一个
    // `CODE_LANG`）⇒ 名字与类型都按 `sokonanoda` 语法上色。
    if let Some(name) = tactic_name_at(text, step.span, offset) {
        if let Some(ty) = tactic_name_type(query, &selection.goals, offset, &name) {
            value.push_str(NAME_DIVIDER);
            value.push_str(&code_block(&format!("{name} : {ty}")));
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

/// goal state 与「名字的类型行」之间的**分割线**（Markdown 水平线）。
///
/// 前后各留一个空行：紧跟代码围栏的 `---` 会被 Markdown 当成 **setext 标题下划线**
/// （把上一段变成 `<h2>`），空行隔开才是真的 `<hr>`（VS Code 的 hover 渲染器就是
/// 标准 Markdown）。形式**只在这里定义一次**，契约写在 `docs/protocol.md`。
const NAME_DIVIDER: &str = "\n\n---\n\n";

/// tactic 关键字里**不在** [`sokonanoda_front::semantic::keywords`] 词表里的那些
/// （今天只有 `sorry`）。它们不是"名字"——不给类型行（`#check sorry` 本来也会被
/// 内核按 `elab-hole-misplaced` 拒掉，这里只是不去白问一次）。
const NON_NAME_TACTIC_WORDS: &[&str] = &["sorry"];

/// 光标下的 tactic **名字**：`Ident` token 且不是语言关键字 ⇒ 那个名字
/// （点分名 `Set.ext` 在词法层是**一个** `Ident`，见 `front::token`）。
///
/// 数字 / 括号 / 字符串 / 记法符号（`Sym`）/ 关键字 ⇒ `None`。词法走**语言自己的
/// lexer**（不是文本扫描）——"这是不是一个名字"与 parser 同源。
///
/// `_` 也是 `Ident`，但它是**占位符**不是名字（`#check _` 今天就是
/// `elab-unknown-identifier`）⇒ 显式挡掉，省一次白问内核。
fn tactic_name_at(text: &str, span: sokonanoda_front::Span, offset: usize) -> Option<String> {
    if offset < span.start.offset || offset >= span.end.offset {
        return None;
    }
    let slice = text.get(span.start.offset..span.end.offset)?;
    let tokens = sokonanoda_front::tokenize(slice).ok()?;
    let rel = offset - span.start.offset;
    let token = tokens
        .iter()
        .find(|t| t.span.start.offset <= rel && rel < t.span.end.offset)?;
    let sokonanoda_front::TokenKind::Ident(name) = &token.kind else {
        return None;
    };
    if sokonanoda_front::semantic::keywords().contains(&name.as_str())
        || NON_NAME_TACTIC_WORDS.contains(&name.as_str())
        || name == "_"
    {
        return None;
    }
    Some(name.clone())
}

/// **hover 里问常量签名**的唯一入口（2026-10-10 修一个**假话**缺陷）。
///
/// ⚠ **为什么不用 `judge_type_of_constant`**：它的缓存键只有
/// `(prelude 模式, 名字)`（`judge.rs` 的前提是「常量签名与**谁在用它**无关，
/// 名字唯一且单调增长」），而那张表是**进程级**的、**没有任何失效路径**
/// ⇒ 同一个名字出现在**另一份环境**里（另一个项目、或用户刚把签名改了）会答出
/// **旧签名** ✗。真 LSP 实测（用户可见）：`def myop : Nat := 0` 上 hover 出
/// `` `myop : Nat` ``，把文件改成 `def myop : Bool := Bool.true` 再 hover
/// ⇒ goal state 是**新的**（`⊢ Bool`）而名字那行**仍是 `Nat`** ✗ —— hover 说假话。
///
/// `judge_type_of` 的键**含整段前缀**（`judge_cache_key`）⇒ 环境变了必然重算 ✓；
/// 而它自己的缓存仍在（同一份文档反复 hover ⇒ 命中；实测第二次 **1ms**）⇒
/// **零性能代价**（`judge_type_of_constant` 未命中时走的就是这一条 ✓）。
///
/// 横向排查：`lib.rs` 里**所有 hover 路径**都走这里（记法符号的原始类型 ·
/// 记法目标名 · tactic 里的名字 · 内核内建登记名）。**编译路径**（`elab_notation`
/// 那条）仍用 `judge_type_of_constant`（它的前缀逐条增长 ⇒ 名字键才是那笔
/// O(n²) 的解药）；那条路的**跨环境**风险另立台账（`docs/gaps/ledger.jsonl` G-100）。
fn hover_type_of_constant(
    prefix: &str,
    options: &sokonanoda_front::compile::CompileOptions,
    name: &str,
) -> std::result::Result<String, sokonanoda_front::judge::Judgement> {
    sokonanoda_front::judge::judge_type_of(prefix, options, name)
}

/// 名字的类型（**诚实省略**：拿不到干净类型就不编这一行）。
///
/// 解析顺序 = 语言的名字解析顺序，两级：
///  1. **局部绑元**（`exact h` 的 `h`）：进入该 tactic 的 goal 快照里那个 binder
///     的类型——与上面 goal block **同一份真相**（已折记法）；
///  2. **常量**（`apply Set.ext`）：问内核（`judge_type_of_constant`，与记法
///     hover 同一条路），前缀取**闭包 + 本文件**（目标在被 import 的库里也拿得到）。
///
/// 折记法（`query.fold_display`）与 goal state 同一观感；`$N` 松散变量的文本不可信
/// ⇒ 弃用（`front::display::print_back` 对含 `$` 的输入本来就原样返回）。
fn tactic_name_type(
    query: &QueryDoc,
    goals: &[sokonanoda_front::compile::ByGoalState],
    offset: usize,
    name: &str,
) -> Option<String> {
    // ① 局部假设：点分名不可能是绑元名，跳过（省一次遍历）。
    if !name.contains('.') {
        if let Some(ty) = goals
            .iter()
            .flat_map(|g| g.binders.iter())
            .find(|b| b.name == name)
            .map(|b| b.ty.clone())
        {
            if !ty.is_empty() && !ty.contains('$') {
                return Some(ty);
            }
        }
    }
    // ② 常量：走内核（答不出 / 不干净 ⇒ 不编）。
    let options = CompileOptions {
        prelude: query.mode,
    };
    let prefix = query.judge_prefix_with_entry(offset);
    let ty = hover_type_of_constant(&prefix, &options, name).ok()?;
    if ty.is_empty() || ty.contains('$') {
        return None;
    }
    Some(query.fold_display(&ty))
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
    // **判据在真相层**（审计 #15，2026-09-25 ✓）：这里原来**内联抄了同一份规则** ✗
    //（"多余洞"的包含判定 ✓）⇒ 分叉时会给学生**相反**的下一步指令 ✗。
    // 现在只用 `sokonanoda_front::query` 的两个公开 helper ✓（唯一归属 ✓）。
    let redundant_spans = sokonanoda_front::query::redundant_hole_spans(report);
    let all_holes_redundant = |d: &DeclState| {
        !d.holes.is_empty()
            && d.holes
                .iter()
                .all(|hole| sokonanoda_front::query::hole_is_redundant(hole, &redundant_spans))
    };
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
/// `.sokonanoda` text the client renders (docs/protocol.md §`soko/stateAt`).
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
///
/// `input_hint`（`Some` = 光标下的标识符是输入法表里的**标识符型**表项，如
/// `α`）：**追加**一段「怎么输入」，不抢类型行（设计 D5）。`None` ⇒ 逐字节与
/// 从前相同。
fn hover_markup(res: render::HoverResolved, input_hint: Option<&str>) -> Hover {
    let mut value = code_block(&res.content);
    if let Some(hint) = input_hint {
        value.push_str("\n\n");
        value.push_str(hint);
    }
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range_of(res.range)),
    }
}

/// 光标落在 `#check` / `#print` 那一行时，hover 显示**那条命令自己的输出**。
///
/// **数据来自真相层**（[`sokonanoda_front::query::QueryDoc::messages_at`]）——
/// 与 Infoview 的「命令输出」块（`soko/stateAt.messages`）**同一份**：选择语义 =
/// 按**行**取（贴 Lean 的 `getInteractiveDiagnostics{lineRange?}`），`#check` 的
/// 类型那一半已过记法折叠，`#print` 是内核的声明 pp。⇒ hover 与面板**永不漂移** ✓
/// （适配器只做映射：文本进围栏块 ⇒ 与目标/条件/声明卡片**同一个** `code_block`
/// 上色接口；`start..end` 折成 LSP `Range` 当高亮范围）。
///
/// **为什么必须有这条路**（用户 2026-10-10 实测两条）：
/// * `#check Eq.refl` 以前落回"表达式 hover"，而那条路的文本来自
///   `infer_under_binders` + pp（含未解层元变量时曾 panic ⇒ 静默降级成空文本）
///   ⇒ hover 只剩 `Eq.refl`、**没有类型** ✗；
/// * `#print Set.singleton` 那一行**一个 hover 行都没有**（`walk.rs::print`
///   从不 push `cmd_hovers`）⇒ 完全静默 ✗。
///
/// 拿不到输出（那一行不是命令、或命令没有结果）⇒ `None`，既有 hover 链继续走。
fn command_line_hover(
    query: &sokonanoda_front::query::QueryDoc,
    text: &str,
    cursor: usize,
) -> Option<Hover> {
    let message = query.messages_at(cursor).into_iter().next()?;
    if message.text.trim().is_empty() {
        return None;
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: code_block(&message.text),
        }),
        // 高亮范围 = 那条命令的源范围（`messages_at` 给的就是命令自己的 span）。
        range: Some(query_map::range_of_offsets(
            text,
            message.start,
            message.end,
        )),
    })
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
    // **内建 / prelude 记法**（E10 之后的事实；**2026-10-10 更正**）：`∧`/`=` 的
    // 展开目标是内核 **prelude 名**（`And`/`Eq`），它们**不在任何模块里**，但 E10
    // 起 prelude 里有**登记注释行**（`-- sokonanoda:builtin-notation "∧" => And`）
    // ⇒ `F12` **有**落点（落 `prelude/Prelude.sokonanoda` 那一行 ✓；单文件模式由
    // `notation::builtin_declaration_span` 兜底 ✓）。
    //
    // ⚠ 旧文案「内建记法（内核 prelude）：**没有源码声明**，`F12` 无处可跳」是
    // **T-D20 时代**的话 —— E10 之后它在 `∧ ∨ ↔ ¬ ≠`（以及本轮之后的 `=`）上是
    // **假话**，而且与 `definition` 的返回值**当场自相矛盾** ✗（用户 2026-10-10
    // 反馈：F12 明明落到了 prelude，hover 却说无处可跳）。⇒ 删掉，改成今天的事实 ✓。
    //
    // 判据也从「闭包表里查不到」换成**直接问内建表**（`notation::is_builtin_notation`）：
    // 旧反推会把**没人声明的库记法**（`∈` 没 import 库时）也算成"内建" ✗。
    let builtin = sokonanoda_front::notation::is_builtin_notation(&symbol);
    let from_module = query
        .notation_at(text, offset)
        .is_some_and(|(_, _, module, _)| module.is_some());
    if builtin {
        lines.push(
            "内建记法（内核 prelude）：声明点是 prelude 里那行 `-- sokonanoda:builtin-notation` \
             登记注释，`F12` 落到 `prelude/Prelude.sokonanoda`"
                .to_string(),
        );
    } else if !locally_declared && !from_module {
        // **诚实说明 ≠ 静默**（T-D50 同一条纪律）：表里有这个符号、但这份文本的
        // 闭包里没人声明它（典型：`∈` 没 `import` 声明它的课程库）⇒ `F12` 没有落点，
        // 说清原因，别让学习者以为是坏了 ✓。
        lines.push(
            "这个符号在**本文件与它的 `import` 闭包**里都没有声明\
             （课程库的记法要 `import` 声明它的模块）⇒ `F12` 没有落点"
                .to_string(),
        );
    }
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
        // **§11.17 修法 (a)（2026-10-09）**：目标在**本文件**声明时，前缀必须含**本文件
        // 自己的文本** —— `judge_prefix` 只给**闭包**（不含入口），单文件时是**空串**
        // ⇒ 合成的 `#check <目标>` 里根本没有那条声明 ⇒ 以前这条**整行消失** ✗。
        // 以前"能出那行"靠的是编译期写进函数级 `CACHE` 的**副作用**（键不含前缀），
        // 而就地快路不再写它（§11.14–§11.17）⇒ 这里**不依赖副作用**、把文本给足 ✓。
        let judge_prefix = if locally_declared {
            query.judge_prefix_with_entry(offset)
        } else {
            prefix.clone()
        };
        if let Ok(ty) = hover_type_of_constant(&judge_prefix, &options, &target) {
            // 松散变量（`$N`）的文本不可信——与 `render::hover_type_at` 同一条
            // 纪律：拿不到干净的类型就不编。
            if !ty.is_empty() && !ty.contains('$') {
                // **语言文本走围栏块**（2026-10-10 横向排查）：类型行是
                // `.sokonanoda` 文本 ⇒ 与 tactic 名字行 / 声明签名同一条
                // `code_block` 接口（行内码在 VS Code 里**不上色** ✗）。
                lines.push(code_block(&format!("{target} : {ty}")));
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
                lines.push(code_block(&format!("{expr} : {}", h.text)));
            }
        }
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: lines.join("\n\n"),
        }),
        // **T-D17：range 收窄到符号本身**。以前是 `None`（客户端按"光标词"高亮）
        // ——对 `∈` 这种单字符还行，对 `⁻¹'`/`×ˢ`/`𝒫` 这种多字符或星平面符号
        // 就不准（客户端的分词规则和我们的词法不是一回事）。
        // span 走与 `symbol_at` **同一条**词法查找（`symbol_span_at`），
        // 所以"认得出来"与"给出范围"永远一致。
        range: notation_input::symbol_span_at(text, offset).map(range_of),
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
    let term_text = fold_for_display(text, &render_expr(body));
    // 声明目标 = 声明类型剥掉值已消耗的层数。
    let file = parse(slice).ok()?;
    let ty = match file.commands.first()? {
        Command::Theorem { ty, .. } | Command::Def { ty, .. } => ty.clone(),
        _ => return None,
    };
    let goal_ty = peel_pi_layers(&ty, binders.len())?;
    let goal_text = fold_for_display(text, &render_expr(&goal_ty));

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
                goals.push(fold_for_display(text, &render_expr(domain)));
                cur = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                for binder in binders {
                    goals.push(fold_for_display(
                        text,
                        &render_expr(binder.ty.as_deref().unwrap_or_else(|| body.as_ref())),
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
    let codomain = fold_for_display(text, &render_expr(cur));
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
                // **`import lib.Set` 的模块名是可点链接**（用户 2026-10-08：「import 这一行的
                // 代码增加跳转功能，打开对应的文件」；设计 `docs/design/import-links.md`）。
                // target 在 `document_link` 里直接给 ⇒ **不需要 resolve** ✓（与 inlay hint 同款）。
                document_link_provider: Some(DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
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
        // **必须读用户缓冲区那份（`latest_text`），不能读 `text()`** ——
        // 2026-10-01 用户反馈「输入几行代码后，**整份代码的颜色全乱了**」的真根因：
        //
        // `text()` 是**上一次编译用的**文本，编译装回之前它一直落后于缓冲区
        // （`docs/mod.rs` 的分工注释：`text` 与 `report` 同源）。而语义 token 的
        // 消费者是**客户端**：VS Code 把这份 token 画到**当前**缓冲区上 ⇒ 编辑点
        // 之后**每个 token 都错位**，输入的行越多错得越远 = 满屏颜色错位。
        //
        // 为什么这里可以（也应该）用缓冲区那份：`front::semantic::semantic_tokens`
        // 是**纯源文本函数**（`lex_prefix` + `parse` + `classify`），**不碰 report**
        // ⇒ 与"新文本 + 旧报告"那个不一致无关 ✓。其余 handler（hover/definition/
        // inlay_hint/…）读 `text()` 是**对的**：它们同时读 `report`，两者必须同源。
        //
        // 判据：`crates/lsp/src/tests/tokens.rs::semantic_tokens_follow_the_buffer_not_the_last_compile`
        // （在开头插一行后 `theorem` 必须在第 2 行；答旧文本时它落在第 1 行 ⇒ 必红）。
        let text = {
            let mut docs = self.doc.lock().expect("doc lock");
            docs.focus_request(&params.text_document.uri);
            docs.latest_text().to_string()
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
        // **命令行 hover**（`#check` / `#print`，2026-10-10 用户实测的两条）：
        // 光标落在那一行 ⇒ 直接显示**那条命令自己的输出**。**必须最先试**：
        // 命令行不在任何声明里，它要的答案就是命令输出本身（与 Infoview 的
        // 「命令输出」块同一份真相 `QueryDoc::messages_at` ⇒ 永不漂移 ✓）。
        //
        // 修前两条路的病：`#check Eq.refl` 落回"表达式 hover"，那条路依赖
        // `infer_under_binders` 的推断文本（含未解层元变量时曾 panic ⇒
        // `resolve_hovers` 静默降级成空文本 ⇒ hover 只剩 `Eq.refl`、没有类型 ✗）；
        // `#print` 那一行**一个 hover 行都没有**（`walk.rs::print` 从不 push
        // `cmd_hovers`）⇒ 完全静默 ✗。
        if std::env::var("SOKO_HOVER_TRACE").is_ok() {
            eprintln!("[hover-chain] 尝试 command_line_hover (offset={offset})");
        }
        if let Some(hover) = command_line_hover(doc.query(), doc.text(), offset) {
            return Ok(Some(hover));
        }
        // `by` tactic hover: show the goal state entering the tactic under the
        // cursor (Lean Infoview-style, user request). Before keyword suppression
        // below, because tactic words (intro/exact/…) are keywords.
        if std::env::var("SOKO_HOVER_TRACE").is_ok() {
            eprintln!("[hover-chain] 尝试 tactic_goal_hover (offset={offset})");
        }
        if let Some(hover) = tactic_goal_hover(report, doc.text(), offset, &decls, doc.query()) {
            return Ok(Some(hover));
        }
        // 半截表达式的 goal-state（内核拒绝 + 有可推断的部分应用）。
        // 只在 hover 请求时计算（不在按键路径），judge_infer 有缓存。
        if std::env::var("SOKO_HOVER_TRACE").is_ok() {
            eprintln!("[hover-chain] 尝试 half_expression_goals_hover (offset={offset})");
        }
        if let Some(hover) = half_expression_goals_hover(report, doc.text(), offset, &decls) {
            return Ok(Some(hover));
        }
        // 记法符号（`∧` / 本文件声明的 `⊗` / import 来的 `∈`）：符号 + 展开 +
        // **怎么输入**（用户要求，D5）。必须在下面的关键字闸门**之前**——
        // 已声明的记法符号被 `front::semantic` 归进 `Keyword`，闸门会把它们
        // 一起吞掉（实测：本文件声明的符号 hover 完全静默）。
        if std::env::var("SOKO_HOVER_TRACE").is_ok() {
            eprintln!("[hover-chain] 尝试 notation_symbol_hover (offset={offset})");
        }
        if let Some(hover) = notation_symbol_hover(doc.text(), offset, report, pos, doc.query()) {
            return Ok(Some(hover));
        }
        // **记法声明的目标名**（T-D50 / 缺口 G-37）：`=> Set.image` 里那个名字
        // 在 AST 里不是使用点 ⇒ 以前 hover 完全静默。这里说清"它是谁的记法目标"，
        // 名字解析得出来时再补一行签名（与 `notation_symbol_hover` 同一口径）。
        //
        // **第二跳（2026-10-10）**：注释登记行（`-- sokonanoda:builtin-notation
        // "∧" => And`）里的目标名也走这条 —— `notation_target_at` 现在两种形态
        // 都认 ✓（判据 `notation_input::target_resolution_tests`）。
        // **解析不出签名时必须说明原因**（诚实说明 ≠ 静默 ✗）：典型是
        // `-- sokonanoda:builtin-sugar "{a}" => Set.singleton` —— 目标是**卷 I
        // 课程库**的常量，不在 prelude 文件的闭包里 ⇒ `F12` 也没有落点 ✓。
        if let Some((name, _)) =
            sokonanoda_front::notation_input::notation_target_at(doc.text(), offset)
        {
            let mut lines = vec![format!("`{name}` —— 记法的目标")];
            let options = sokonanoda_front::compile::CompileOptions {
                prelude: doc.query().mode,
            };
            // 前缀取**闭包 + 本文件**（`judge_prefix_with_entry`）：目标在被 import
            // 的库里、或就在本文件里时都拿得到签名 ✓（以前传空串 ⇒ 只有 prelude
            // 名字拿得到，其余静默 ✗）。
            let prefix = doc.query().judge_prefix_with_entry(offset);
            let ty = hover_type_of_constant(&prefix, &options, &name)
                .ok()
                .filter(|ty| !ty.is_empty() && !ty.contains('$'));
            match &ty {
                Some(ty) => lines.push(code_block(&format!("{name} : {ty}"))),
                None => lines.push(
                    "这个名字在**本文件与它的 `import` 闭包**里没有声明 ⇒ 给不出签名，\
                     `F12` 也没有落点（登记行只**登记**目标，不声明它）"
                        .to_string(),
                ),
            }
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: lines.join("\n\n"),
                }),
                range: None,
            }));
        }
        // **内核内建家族的登记名**（E2）：`-- sokonanoda:builtin-rust "Nat / …"` 里的
        // 名字在**源文本里没有声明位置**（`install_prelude` 在 Rust 里手搓 AST 装进
        // 环境）⇒ `F12` 如实没有落点（`prelude_def_span` 返回 `None` ✓，**不许编位置** ✗）。
        // 但 hover **不许静默**（与 T-D50 同一条纪律）：说清它是什么、为什么没落点 ✓。
        if let Some((name, _)) =
            sokonanoda_front::notation_input::builtin_registry_name_at(doc.text(), offset)
        {
            let mut lines = vec![format!("`{name}` —— 内核内建（prelude）")];
            let options = sokonanoda_front::compile::CompileOptions {
                prelude: doc.query().mode,
            };
            if let Ok(ty) = hover_type_of_constant("", &options, &name) {
                if !ty.is_empty() && !ty.contains('$') {
                    lines.push(code_block(&format!("{name} : {ty}")));
                }
            }
            lines.push(
                "它由内核**按名字安装**（不是源文件里声明的）⇒ `prelude/Prelude.sokonanoda` \
                 里没有它的声明位置，`F12` 没有落点（E2 如实登记）"
                    .to_string(),
            );
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: lines.join("\n\n"),
                }),
                range: None,
            }));
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
        if std::env::var("SOKO_HOVER_TRACE").is_ok() {
            eprintln!("[hover-chain] 尝试 bracket_hover (offset={offset})");
        }
        if let Some(res) = bracket_hover(doc.text(), &report.hovers, pos.line, pos.character) {
            return Ok(Some(hover_markup(res, None)));
        }
        // **标识符型表项的「怎么输入」**（设计 `docs/design/notation-input.md` D5，
        // 用户反馈：「`α` 没有快捷输入」）：希腊字母是**标识符**不是记法符号
        // （D4）⇒ 上面那条 `notation_symbol_hover` 不会触发，这一行只能补在这里。
        // **追加**在类型行之后，不抢主线（`α : Prop` 才是学习者要看的）。
        let input_hint = sokonanoda_front::notation_input::input_hint_at(doc.text(), offset);
        if let Some(h) = hover_type_at_offset(&report.hovers, offset) {
            // 学习者需求：显示「表达式 : 类型」——表达式从源码按 span 切片
            //（括号平衡成良构），并返回表达式范围供编辑器高亮。
            return Ok(Some(hover_markup(
                expr_hover(doc.text(), h),
                input_hint.as_deref(),
            )));
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
                return Ok(Some(hover_markup(
                    expr_hover(doc.text(), h),
                    input_hint.as_deref(),
                )));
            }
        }
        if let Some(d) = decl_at(&report.decls, pos.line, pos.character) {
            let signature = match &d.ty_text {
                Some(ty) => format!("{} {} : {}", d.kind.as_str(), decl_name(d), ty),
                None => format!("{} {}", d.kind.as_str(), decl_name(d)),
            };
            // Signature and goal state are `.sokonanoda` text → fenced blocks so
            // the editor highlights them (docs/protocol.md §`soko/stateAt`).
            let mut value = code_block(&signature);
            // **`def` 的值**（T-D52 / 用户第 8 条反馈，2026-10-10 用户要求
            // hover 也带上）：`:=` 之后那个东西 —— 类型看不出 `Set.mem` 的本质，
            // 值才看得出。与 Infoview 的 `decl-val-line` 同一语义（`:=` 标签 +
            // 值块 ⇒ 这里是 `:= <值>` 的**独立围栏块**）。
            //
            // `theorem`/`axiom`/`inductive` 的 `val_text` 是 `None`（证明 / 公设 /
            // 构造子表都不是"定义"）⇒ **一个字节都不加**（回归基线：它们的 hover
            // 与改动前逐字节相同）。空值同样不加（诚实省略，不编一个空块）。
            if let Some(val) = d.val_text.as_deref().filter(|t| !t.is_empty()) {
                value.push_str("\n\n");
                value.push_str(&code_block(&format!(":= {val}")));
            }
            match d.status {
                DeclStatus::Open => {
                    // 光标正落在某个 `sorry` 上：先给这个洞的精确期望类型
                    //（超量应用走查经 def 展开算出，如 `(And.right a (Not a)
                    // x) sorry` 的洞期望 `a`，而不是整个声明类型）。
                    let hole_ty = d
                        .sub_goals
                        .iter()
                        .find(|s| s.span.start.offset <= offset && offset <= s.span.end.offset);
                    // **显示副本**（A0，2026-09-26）：报告里那份
                    //（`sub_goals[].ty`）是**判定输入**（`suggest.rs` 回读它算建议）
                    // ⇒ 这里折的是**给用户看的那一份克隆** ✓（与 wire 同源 ✓）。
                    let hole_ty_display = hole_ty
                        .and_then(|sg| sg.ty.as_deref())
                        .map(|t| doc.query().fold_display(t));
                    match (&d.goal, hole_ty) {
                        (Some(goal), Some(sg)) if sg.ty.is_some() => value.push_str(&format!(
                            "\n此处 `sorry` 的期望类型：\n{}\n\n剩余目标：\n{}",
                            code_block(hole_ty_display.as_deref().unwrap_or_default()),
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

    /// `textDocument/documentLink`：`import lib.Set` 的**模块名**是链接，点开就是
    /// 被导入的那个文件（用户 2026-10-08：「import 这一行的代码增加跳转功能，打开
    /// 对应的文件」；设计 `docs/design/import-links.md`）。
    ///
    /// 三条边界（都是**有意的**）：
    ///  * 单文件 / 没有 `import` ⇒ `Some(vec![])` —— **空数组合法**，不是错误 ✗
    ///    （LSP 语义：没有链接就是没有链接，报错会让编辑器弹面板）；
    ///  * 解析不到的模块（`import-not-found`）⇒ **不给链接** ✗ —— 点开一个不存在的
    ///    文件比"点不动"更糟，而诊断已经在说原因 ✓；
    ///  * 扫描的是 `latest_text()`（**用户缓冲区**那份 ✓，与语义 token 同一条纪律）：
    ///    `import` 是**纯词法**的，打字中也要答得对 ⇒ 走 `front::query::import_lines`
    ///    （唯一实现 = 闭包加载器那份 ✓，不重跑内核）。
    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        let request_uri = params.text_document.uri;
        let mut docs = self.doc.lock().expect("doc lock");
        docs.focus(&request_uri);
        let text = docs.latest_text().to_string();
        let mut links = Vec::new();
        for (module, span) in import_lines(&text) {
            // 模块名 → 绝对路径由真相层回答（闭包模块表）；答不出 ⇒ 不给死链 ✗。
            let Some(path) = docs.query().module_path(&module) else {
                continue;
            };
            let Ok(target) = Url::from_file_path(&path) else {
                continue;
            };
            links.push(DocumentLink {
                // `import` 行只有 ASCII 前导（`import` + 空白）⇒ `range_of`（按 char 计列）
                // 与 LSP 的 UTF-16 口径在这里**必然一致** ✓，不需要 `range_of_in`。
                range: range_of(span),
                target: Some(target),
                tooltip: Some(format!("打开模块 {module}")),
                data: None,
            });
        }
        Ok(Some(links))
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
        // **记法符号**（T-D12/T-D13，用户第 6 条反馈的后半："代码里的 notation
        // 不能跳转"）：`∈` 不是声明名，`definition_at`（走 hover span + 声明表）
        // 答不上来。先试记法：闭包记法表给出**声明它的模块 + 那一行的 span**。
        {
            let text = docs.text().to_string();
            let offset = position_to_offset(&text, pos);
            if let Some((_, _, module, span)) = docs.query().notation_at(&text, offset) {
                // **E10（v0.76.0）**：内建记法（`∧ ∨ ↔ ¬ ≠`）**不属于任何模块**
                //（`module: None`：prelude 不是模块 ✓），但 prelude 里那行
                // `-- sokonanoda:builtin-notation "∧" => And` 给了它们 span
                //（`notation::builtin_directive_span` ✓）⇒ 落点走 **prelude 源**
                //（与 `Or`/`And` 这些 prelude 名字同一条路：`prelude_source_path()`
                // 物化出与喂进编译**同一份字节** ✓）。
                // ⚠ 以前 `let module = module?` 会把内建**直接丢掉** ⇒ 学生对 `∧`
                // 按 F12 **毫无反应** ✗（E10 要修的「回记法」那一跳 ✓）。
                if module.is_none() && span.start.offset != 0 {
                    if let Some(path) = sokonanoda_front::compile::prelude_source_path() {
                        if let Ok(uri) = Url::from_file_path(&path) {
                            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                                uri,
                                range: range_of(span),
                            })));
                        }
                    }
                }
                if let Some(module) = module {
                    if let Some(path) = docs.query().module_path(&module) {
                        if let Ok(uri) = Url::from_file_path(&path) {
                            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                                uri,
                                range: range_of(span),
                            })));
                        }
                    }
                }
            }
        }
        // **内建记法的声明点，不依赖项目模式**（2026-10-10 用户反馈）：
        // 上面那条 `notation_at` 只在**项目模式**下答得上（它查闭包记法表
        // `project.notations`）⇒ **单文件**里内建记法够不着声明点，实测两种坏法：
        //   * `=` ⇒ `definition_at` 也答不上来 ⇒ **无声返回 `null`** ✗（用户原话：
        //     「`"a = b"` 里的等于号没有跳转」—— 有 import / 单文件都一样）；
        //   * `∧` ⇒ 落到下面 `definition_at` 的 `ResolvedTarget::Notation` 分支，
        //     它的 `range` 是**光标处**那个 span ⇒ **原地跳**（F12 视觉上没反应 ✗，
        //     正是 E05/G-37 的教训）。
        // 内建记法的声明点是 prelude 里那行**登记注释**，与项目无关 ⇒ 直接问前端
        // 要（`notation::builtin_declaration_span`，与记法表**同一份 span** ✓）。
        // 非内建符号 ⇒ `None` ⇒ 不抢别的分支 ✓。
        {
            let text = docs.text().to_string();
            let offset = position_to_offset(&text, pos);
            if let Some((symbol, _)) = sokonanoda_front::notation_input::symbol_at(&text, offset) {
                if let Some(span) = sokonanoda_front::notation::builtin_declaration_span(&symbol) {
                    if let Some(path) = sokonanoda_front::compile::prelude_source_path() {
                        if let Ok(uri) = Url::from_file_path(&path) {
                            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                                uri,
                                range: range_of(span),
                            })));
                        }
                    }
                }
            }
        }
        // **记法声明的目标名**（T-D50 / 缺口 G-37）：`infixr:80 " '' " => Set.image`
        // 里 `=>` 后面的名字是**普通引用**，但它在 AST 里不是使用点 ⇒
        // `definition_at` 答不上来。词法认出它之后，走与上面**同一条闭包通道**
        // （`project_definition` 握着整个闭包的名字 → 模块）。
        //
        // **E05（Bug A，2026-09-27）：span 用错了一个** ✗✓ —— 原来把
        // `project_definition` 返回的**真定义 span** 用 `_` 丢掉，`range` 用了
        // **光标处**那个 span（就是目标名自己）⇒ F12 **原地跳**（视觉上等于
        // 没反应 ✗）。真课程库实测：`Set.powerset` 应落 **L81** 实落 **L125**、
        // `Set.compl` 应落 **L79** 实落 **L126** ✓。⇒ 现在用**定义那一处**的 span。
        // 判据：`goto_definition_on_a_notation_target_name_lands_on_the_definition`
        // （断言落点行号 == `def` 那一行，**不许自跳** ✗）。
        {
            let text = docs.text().to_string();
            let offset = position_to_offset(&text, pos);
            if let Some((name, _)) =
                sokonanoda_front::notation_input::notation_target_at(&text, offset)
            {
                if let Some((path, def_span)) = docs.query().project_definition(&name) {
                    if let Ok(uri) = Url::from_file_path(&path) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri,
                            range: range_of(def_span),
                        })));
                    }
                }
                // **prelude 里的目标名**（第二跳，2026-10-10 用户反馈）：
                // `-- sokonanoda:builtin-notation "∧" => And` 的 `And` 住在 **prelude
                // 源**里，而 prelude **不是模块** ⇒ `project_definition` 查不到它
                //（打开 prelude 文件本身时连项目模式都没有 ✗）⇒ 以前 F12 `null` ✗。
                // 走与 A4 同一条**已有真相通道**：`PRELUDE_NAMES` + `prelude_def_span`
                //（真 parser 出的 span ✓，不做文本比对 ✓）⇒ `And` 落 `inductive And`
                // 那一行、`Eq` 落 `axiom Eq` 那一行 ✓。
                // ⚠ **顺序**：`project_definition` 在前（闭包/本文件自己的声明优先 ——
                // 与 A4 的「prelude 让位」同一条纪律 ✓）。
                if sokonanoda_front::compile::PRELUDE_NAMES.contains(&name.as_str()) {
                    if let Some((path, span)) = sokonanoda_front::compile::prelude_source_path()
                        .zip(sokonanoda_front::compile::prelude_def_span(&name))
                    {
                        if let Ok(uri) = Url::from_file_path(&path) {
                            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                                uri,
                                range: range_of(span),
                            })));
                        }
                    }
                }
            }
        }
        let Some(target) = definition_at(&report.hovers, pos.line, pos.character) else {
            // **prelude 名字**（A4，2026-09-26 用户报告第 4 条）：`Or` / `And` /
            // `Iff` / `False` 这些是**内置前奏**（受信任安装），它们的 hover 行按
            // 设计 `resolution: None`（`report.rs`：prelude 名字没有定义位置）
            // ⇒ `definition_at` 答不上来 ⇒ 以前 F12 **什么都不发生** ✗。
            //
            // 现在返回**前奏源文件里的真 span**：源就是内嵌常量本身（与喂进编译的
            // 同一份字节 ✓），落成一个**真实文件**（Lean 4 的 `Init/Prelude.lean`
            // 同款 ✓）。落盘失败 / 那份源里没有这个文字（`Nat`/`Bool` 家族是 Rust
            // AST 手搓的）⇒ **不编造位置**，返回 `None` ✓。
            //
            // ⚠ **必须放在 `definition_at` 之后**（实测教训）：文件/项目**自己声明**
            // 了同名名字时，prelude 会**让位**（族级让位 ✓），此时 `definition_at`
            // 已经答上了 ⇒ 前奏分支抢在前面就是**回归** ✗
            // （`tests::project::definition_jumps_into_the_imported_module` 抓到的
            // 正是它：被 import 的模块自己定义了 `Or`）。
            {
                let text = docs.text().to_string();
                let at_cursor = render::hover_type_at(&report.hovers, pos.line, pos.character)
                    .and_then(|h| {
                        let (s, e) = (h.span.start.offset, h.span.end.offset);
                        text.get(s..e)
                    });
                if let Some(name) = at_cursor {
                    if sokonanoda_front::compile::PRELUDE_NAMES.contains(&name) {
                        if let Some((path, span)) = sokonanoda_front::compile::prelude_source_path()
                            .zip(sokonanoda_front::compile::prelude_def_span(name))
                        {
                            if let Ok(uri) = Url::from_file_path(&path) {
                                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                                    uri,
                                    range: range_of(span),
                                })));
                            }
                        }
                    }
                }
            }
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
            // 记法符号的**跨模块**跳转走 T-D10 的记法分支（在 `definition_at`
            // 之前就返回了）；走到这里的是本文件内声明的记法 ⇒ 没有跨文件目标。
            ResolvedTarget::Notation { .. } => None,
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
        // **G-55（2026-10-07 用户授权"本轮做"）**：记法声明的**目标名**在 AST 里
        // **不是使用点**（没有 hover 行、也没有 `resolution`）⇒ 下面那条
        // 「定义 → 引用」反查永远空手 ⇒ 以前**一律 `null`**（真课程库 11 条实测）。
        // 这里先走**词法**认出它（`notation_target_at`，与 F12 那条分支同一份判据），
        // 再按定案语义把"同一个定义"的位置一次答齐（见
        // `render::notation_target_highlight` 的头注：① 名字自己 · ② 本文件里
        // 展开到这个目标的记法符号的每一处 · ③ 定义那一处 + 点名使用处）。
        {
            let text = docs.text();
            let offset = position_to_offset(text, pos);
            if let Some((name, span)) =
                sokonanoda_front::notation_input::notation_target_at(text, offset)
            {
                // 定义只在本文件里时才进结果：`documentHighlight` 的 range 属于
                // **被请求的这份文档**（跨文件的落点是 `definition` 的事）。
                let definition =
                    docs.query()
                        .project_definition(&name)
                        .and_then(|(path, def_span)| {
                            (docs.query().path.as_deref() == Some(path.as_path()))
                                .then_some(def_span)
                        });
                let ranges =
                    notation_target_highlight(text, &report.hovers, &name, span, definition);
                return Ok(Some(
                    ranges
                        .into_iter()
                        .map(|range| DocumentHighlight {
                            range,
                            kind: Some(DocumentHighlightKind::TEXT),
                        })
                        .collect(),
                ));
            }
        }
        let Some(ranges) = highlight_uses(doc.text(), &report.hovers, pos.line, pos.character)
        else {
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
                docs.text(),
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
                docs.text(),
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
            .custom_method("soko/goalAt", Backend::goal_at)
            .custom_method("soko/nextHole", Backend::next_hole)
            .custom_method("soko/hints", Backend::hints)
            .custom_method("soko/stateAt", Backend::state_at)
            .custom_method("soko/project", Backend::project)
            .custom_method("soko/version", Backend::version)
            .finish();
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}
