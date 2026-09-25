//! 项目状态视图：`QueryDoc::project_view()`（设计 `docs/design/project-view.md`）。
//!
//! 只读派生：从**已经编译过的** `ProjectReport` 装配，不重跑内核、不算摘要、
//! 不碰缓存。CLI `query project` 与 LSP `soko/project` 都读这一份。

use super::types::{
    ProjectArtifacts, ProjectCounts, ProjectDiagnosticInfo, ProjectModule, ProjectView,
};
use super::QueryDoc;
use crate::project::{ModuleStatus, ProjectReport};

/// 视图里的路径一律**绝对**（编辑器要拿它开文件、agent 要拿它拼命令）。
///
/// `canonicalize` 顺手统一了两种写法（macOS 上 `/var` 与 `/private/var` 是同一个
/// 目录）——于是 CLI 与 LSP 对同一个文件给出逐字相同的答案。文件不存在时
/// （缺失模块 / `--text` 的占位路径）退回原样。
fn absolute(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

impl QueryDoc {
    /// 项目视图：文本里有 `import` 且项目编译跑过 ⇒ `Some`。
    ///
    /// `None` **不是**错误：单文件（无 `import`）/ 定位不到入口（stdin）都是合法
    /// 状态，原因见 [`Self::project_view_reason`]（设计 §2 第 2 条）。
    pub fn project_view(&self) -> Option<ProjectView> {
        self.project_report().map(ProjectView::from_report)
    }

    /// 为什么 [`Self::project_view`] 是 `None`（机器码，`docs/protocol.md`）。
    ///
    /// **读 `set_text` 时算好的缓存**（T-A24）：这个问题每次诊断事件都会被问一次
    /// （VS Code 的 `soko/project`），以前每次重新 parse 整份文本。
    /// 兜底：还没 `set_text` 过（`project_reason` 是 `None`）时现算一次。
    pub fn project_view_reason(&self) -> &'static str {
        self.project_reason
            .unwrap_or_else(|| Self::compute_project_reason(&self.project, &self.text))
    }

    fn project_report(&self) -> Option<&ProjectReport> {
        self.project.as_ref()
    }
}

impl ProjectView {
    /// 模块根下产物目录的**只读快照**（R-3）。
    ///
    /// `compiled/` 不存在 ⇒ `None`（**不创建**——这是 `project_view()` 的纪律）；
    /// 存在但空 ⇒ `Some` 且 `entries = 0`（"清过了"也是有用的信息）。
    fn artifacts_of(root: &std::path::Path) -> Option<ProjectArtifacts> {
        let dir = root.join(".sokonanoda");
        let read = std::fs::read_dir(dir.join("compiled")).ok()?;
        let mut entries = 0usize;
        let mut bytes = 0u64;
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                entries += 1;
                bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            }
        }
        let compiler = std::fs::read(dir.join("meta.json"))
            .ok()
            .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok())
            .and_then(|meta| meta.get("compiler")?.as_str().map(str::to_string));
        Some(ProjectArtifacts {
            dir: dir.display().to_string(),
            entries,
            bytes,
            compiler,
        })
    }

    /// 从项目报告装配视图。入口 = 拓扑序最后一个模块（`compile_plan` 的契约）。
    fn from_report(report: &ProjectReport) -> Self {
        let entry = report
            .modules
            .last()
            .map(|module| module.name.clone())
            .unwrap_or_default();
        let mut counts = ProjectCounts {
            modules: report.modules.len(),
            ..ProjectCounts::default()
        };
        let modules: Vec<ProjectModule> = report
            .modules
            .iter()
            .map(|module| {
                let errors = module.report.errors.len();
                let warnings = module.report.warnings.len();
                let open_exercises = module.open_exercises();
                counts.decls += module.report.decls.len();
                counts.errors += errors;
                counts.warnings += warnings;
                counts.open_exercises += open_exercises;
                match module.status {
                    ModuleStatus::Compiled => counts.compiled += 1,
                    ModuleStatus::LoadFailed => counts.failed += 1,
                    ModuleStatus::Blocked => counts.blocked += 1,
                }
                ProjectModule {
                    name: module.name.clone(),
                    path: absolute(&module.path),
                    status: module.status.code().to_string(),
                    entry: module.name == entry,
                    imports: module.imports.clone(),
                    decls: module.report.decls.len(),
                    errors,
                    warnings,
                    open_exercises,
                    // 状态的一句话解释取该模块的**第一条**项目诊断
                    // （`import-not-found` / `import-cycle` / `import-dependency-failed`）；
                    // 编译成功的模块即使有诊断也不在这里重复（报告里已有）。
                    message: match module.status {
                        ModuleStatus::Compiled => None,
                        _ => report
                            .diagnostics
                            .iter()
                            .find(|diag| diag.module == module.name)
                            .map(|diag| diag.message.clone()),
                    },
                }
            })
            .collect();
        Self {
            entry,
            root: absolute(&report.root),
            manifest: report.manifest.as_ref().map(|path| absolute(path)),
            requires_warning: report.requires_warning.clone(),
            modules,
            diagnostics: report
                .diagnostics
                .iter()
                .map(|diag| ProjectDiagnosticInfo {
                    code: diag.code().to_string(),
                    message: diag.message.clone(),
                    module: diag.module.clone(),
                    severity: if diag.kind.is_error() {
                        "error"
                    } else {
                        "warning"
                    }
                    .to_string(),
                    start: diag.span.map(|span| span.start.offset).unwrap_or(0),
                    end: diag.span.map(|span| span.end.offset).unwrap_or(0),
                })
                .collect(),
            counts,
            artifacts: Self::artifacts_of(std::path::Path::new(&report.root)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectView;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-project-artifacts-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// **T-B6 的真相层判据**（R-3）：产物目录快照的三条语义。
    #[test]
    fn artifacts_snapshot_is_read_only_and_counts_only_entries() {
        let root = tmp("view");

        // ① 目录不存在 ⇒ `None`，而且**绝不创建**（`project_view()` 的只读纪律）。
        assert!(ProjectView::artifacts_of(&root).is_none());
        assert!(
            !root.join(".sokonanoda").exists(),
            "the snapshot must not create the artifacts directory"
        );

        // ② 有产物 + meta ⇒ 计数只算 `*.json`（写了一半的 `*.tmp-*` 不算），
        //    字节数只累计条目，`compiler` 来自 `meta.json`。
        let compiled = root.join(".sokonanoda/compiled");
        std::fs::create_dir_all(&compiled).expect("mkdir compiled");
        std::fs::write(compiled.join("aa.json"), b"12345").expect("write aa");
        std::fs::write(compiled.join("bb.json"), b"123").expect("write bb");
        std::fs::write(compiled.join("cc.json.tmp-999"), b"ignored").expect("write tmp");
        std::fs::write(
            root.join(".sokonanoda/meta.json"),
            br#"{"schema":"soko.artifacts/1","compiler":"1.2.3"}"#,
        )
        .expect("write meta");
        let snapshot = ProjectView::artifacts_of(&root).expect("snapshot");
        assert_eq!(snapshot.entries, 2, "only `*.json` entries count");
        assert_eq!(snapshot.bytes, 8);
        assert_eq!(snapshot.compiler.as_deref(), Some("1.2.3"));
        assert!(snapshot.dir.ends_with(".sokonanoda"));

        // ③ 目录在、`compiled/` 空 ⇒ `Some` 且 0 条（"清过了"也是有用信息）。
        std::fs::remove_file(compiled.join("aa.json")).unwrap();
        std::fs::remove_file(compiled.join("bb.json")).unwrap();
        let empty = ProjectView::artifacts_of(&root).expect("snapshot");
        assert_eq!(empty.entries, 0);

        let _ = std::fs::remove_dir_all(&root);
    }
}
