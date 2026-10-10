//! 项目状态视图：`QueryDoc::project_view()`（设计 `docs/design/project-view.md`）。
//!
//! 只读派生：从**已经编译过的** `ProjectReport` 装配，不重跑内核、不算摘要、
//! 不碰缓存。CLI `query project` 与 LSP `soko/project` 都读这一份。

use super::types::{
    ArtifactWriter, ProjectArtifacts, ProjectCounts, ProjectDiagnosticInfo, ProjectModule,
    ProjectView,
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
    ///
    /// **写者照实统计**（2026-10-10 用户实测的第二个反馈）：谁写的写在**条目文件名**
    /// 里（`project::cache::entry_stem`），这里一次 `read_dir` 就数出来了 ——
    /// 既不看 `meta.json` 那个"最后一次写入"的戳（它会被**任何一次写入**刷成当前
    /// 编译器 ⇒ 一条新条目就能把另外 247 条的历史改写掉 ✗），也不解析条目内容
    /// （整份报告、MB 量级 ⇒ 读不起）。
    ///
    /// 于是三件事同时成立：① 面板显示的版本号是**真的**；② 混合目录（升级后只
    /// 重编了一部分）如实报出分布；③ 与当前编译器不一致 ⇒ `stale = true`，面板
    /// 提示 Rebuild（那些条目的键里带版本 + 构建戳 ⇒ **不可能被命中**）。
    fn artifacts_of(root: &std::path::Path) -> Option<ProjectArtifacts> {
        let dir = root.join(".sokonanoda");
        let read = std::fs::read_dir(dir.join("compiled")).ok()?;
        let (current_compiler, current_stamp) = crate::project::cache::current_writer();
        let mut entries = 0usize;
        let mut bytes = 0u64;
        // 写者 → 条数（`(version, stamp)` 一起进键：同版本的不同构建也算两个写者）。
        let mut by_writer: std::collections::BTreeMap<(String, String), usize> =
            std::collections::BTreeMap::new();
        let mut unrecorded = 0usize;
        let mut stale = false;
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            entries += 1;
            bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            let name = entry.file_name();
            match crate::project::cache::writer_of_entry_file(&name.to_string_lossy()) {
                Some((compiler, stamp)) => {
                    *by_writer
                        .entry((compiler.to_string(), stamp.to_string()))
                        .or_insert(0) += 1;
                    if compiler != current_compiler || stamp != current_stamp {
                        stale = true;
                    }
                }
                // 旧命名（`<key>.json`）⇒ 写者**未记录**：如实说，不猜。
                // 它们同样不可能被这份编译器命中 ⇒ 建议 Rebuild。
                None => {
                    unrecorded += 1;
                    stale = true;
                }
            }
        }
        if unrecorded > 0 {
            stale = true;
        }
        let mut writers: Vec<ArtifactWriter> = by_writer
            .into_iter()
            .map(|((compiler, build_stamp), entries)| ArtifactWriter {
                compiler,
                build_stamp,
                entries,
            })
            .collect();
        // 条数降序；同数按 (版本, 构建戳) 升序 ⇒ 输出稳定、判据可钉。
        writers.sort_by(|a, b| {
            b.entries
                .cmp(&a.entries)
                .then_with(|| (&a.compiler, &a.build_stamp).cmp(&(&b.compiler, &b.build_stamp)))
        });
        let compiler = writers.first().map(|writer| writer.compiler.clone());
        Some(ProjectArtifacts {
            dir: dir.display().to_string(),
            entries,
            bytes,
            compiler,
            writers,
            stale,
            current: current_compiler.to_string(),
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

    /// **T-B6 的真相层判据**（R-3）+ **写者照实统计**（2026-10-10 用户实测）：
    /// 产物快照的语义 —— 计数只认文件事实，写者只认**条目文件名**。
    #[test]
    fn artifacts_snapshot_is_read_only_and_counts_only_entries() {
        let root = tmp("view");
        let (current, stamp) = crate::project::cache::current_writer();
        let named =
            |compiler: &str, stamp: &str, key: &str| format!("{compiler}+{stamp}+{key}.json");

        // ① 目录不存在 ⇒ `None`，而且**绝不创建**（`project_view()` 的只读纪律）。
        assert!(ProjectView::artifacts_of(&root).is_none());
        assert!(
            !root.join(".sokonanoda").exists(),
            "the snapshot must not create the artifacts directory"
        );

        // ② 有产物 ⇒ 计数只算 `*.json`（写了一半的 `*.tmp-*` 不算）；写者来自**文件名**，
        //    **不看** `meta.json` —— 那个字段会被任何一次写入刷成当前编译器（谎的来源）。
        let compiled = root.join(".sokonanoda/compiled");
        std::fs::create_dir_all(&compiled).expect("mkdir compiled");
        std::fs::write(compiled.join(named(current, &stamp, "aa")), b"12345").expect("write aa");
        std::fs::write(compiled.join(named(current, &stamp, "bb")), b"123").expect("write bb");
        std::fs::write(compiled.join("cc.json.tmp-999"), b"ignored").expect("write tmp");
        // meta.json 故意写一个**别的**版本：它不再参与显示（写者以条目为准 ✓）。
        let schema = crate::project::cache::meta_schema();
        std::fs::write(
            root.join(".sokonanoda/meta.json"),
            format!(r#"{{"schema":"{schema}","compiler":"1.2.3"}}"#),
        )
        .expect("write meta");
        let snapshot = ProjectView::artifacts_of(&root).expect("snapshot");
        assert_eq!(snapshot.entries, 2, "only `*.json` entries count");
        assert_eq!(snapshot.bytes, 8);
        assert_eq!(
            snapshot.compiler.as_deref(),
            Some(current),
            "写者必须来自条目文件名（meta.json 那个 1.2.3 是谎话，不许上屏）"
        );
        assert_eq!(snapshot.writers.len(), 1);
        assert_eq!(snapshot.writers[0].entries, 2);
        assert_eq!(snapshot.writers[0].build_stamp, stamp);
        assert!(!snapshot.stale, "当前编译器写的产物不许报 stale");
        assert_eq!(snapshot.current, current);

        // ③ **旧命名**（`<key>.json`，本次改动之前写的产物）⇒ 写者**未记录**：
        //    如实说 `None` + `stale`（它们同样不可能被这份编译器命中 ⇒ 建议 Rebuild），
        //    而**不是**拿 `meta.json` 那个会被刷新的字段冒充 ✗。
        std::fs::write(compiled.join("legacy-key.json"), b"1").expect("write legacy");
        let legacy = ProjectView::artifacts_of(&root).expect("snapshot");
        assert_eq!(legacy.entries, 3);
        assert_eq!(
            legacy.compiler.as_deref(),
            Some(current),
            "多数派仍是当前编译器（2 条 vs 1 条未记录）"
        );
        assert!(
            legacy.stale,
            "有写者未记录的条目 ⇒ stale（键里带版本 + 构建戳 ⇒ 不可能命中）"
        );

        // ④ 目录在、`compiled/` 空 ⇒ `Some` 且 0 条（"清过了"也是有用信息）。
        std::fs::remove_file(compiled.join(named(current, &stamp, "aa"))).unwrap();
        std::fs::remove_file(compiled.join(named(current, &stamp, "bb"))).unwrap();
        std::fs::remove_file(compiled.join("legacy-key.json")).unwrap();
        let empty = ProjectView::artifacts_of(&root).expect("snapshot");
        assert_eq!(empty.entries, 0);
        assert_eq!(empty.compiler, None);
        assert!(empty.writers.is_empty());
        assert!(!empty.stale, "空目录里没有「不是这份编译器写的」产物");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **2026-10-10 用户实测（第二个反馈）**：面板上的「由编译器 0.87.3 写入」是**假的**
    /// —— 用户用 0.87.2 编完 248 条产物，装上 0.87.3、**还没 rebuild**，面板就说 0.87.3
    /// （「失去了这个版本号的意义」）。
    ///
    /// 根因（修前）：写者取自 `meta.json.compiler`，而它被 [cache::update_index]
    /// 在**任何一次写入**时刷成当前编译器 ⇒ **一条新条目就能把另外 247 条的历史
    /// 改写掉** ✗。修法：写者编进**条目文件名**（`<compiler>+<stamp>+<key>.json`）,
    /// 读侧照实统计 ⇒ 后写的条目**只能**声明自己那一条。
    ///
    /// 这条判据钉的正是"一条写入改不了历史" + "版本不一致 ⇒ 建议 Rebuild"。
    /// **反向验证**：让 `artifacts_of` 改回读 `meta.json.compiler` ⇒ 第 ① 条断言当场判红。
    #[test]
    fn one_write_cannot_rewrite_the_other_entries_writer_version() {
        let root = tmp("writer-history");
        let (current, _) = crate::project::cache::current_writer();
        // 用户现场：目录里躺着**旧编译器**写的一批条目。
        let old_compiler = "0.87.2";
        let old_stamp = "0123456789abcdef";
        let compiled = root.join(".sokonanoda/compiled");
        std::fs::create_dir_all(&compiled).expect("mkdir compiled");
        for key in ["k1", "k2", "k3"] {
            std::fs::write(
                compiled.join(format!("{old_compiler}+{old_stamp}+{key}.json")),
                b"{}",
            )
            .expect("write old entry");
        }
        // `meta.json` 此刻自述旧编译器（它由旧进程建的）。
        std::fs::write(
            root.join(".sokonanoda/meta.json"),
            format!(
                r#"{{"schema":"{}","compiler":"{old_compiler}","build_stamp":"{old_stamp}"}}"#,
                crate::project::cache::meta_schema()
            ),
        )
        .expect("write meta");

        // 装上新编译器之后**只编了一个文件**（用户实测的现场：还没 rebuild）。
        let new_stem = crate::project::cache::entry_stem("newkey");
        std::fs::write(compiled.join(format!("{new_stem}.json")), b"{}").expect("write new entry");
        crate::project::cache::update_index(
            &root,
            std::path::Path::new("/tmp/Main.sokonanoda"),
            &new_stem,
        );

        let snapshot = ProjectView::artifacts_of(&root).expect("snapshot");
        assert_eq!(
            snapshot.compiler.as_deref(),
            Some(old_compiler),
            "写者必须是**多数派**（3 条 0.87.2 vs 1 条 {current}）—— 一次新写入不许改写历史：{:?}",
            snapshot.writers
        );
        assert_eq!(snapshot.writers.len(), 2, "混合目录要如实报两个写者");
        assert_eq!(snapshot.writers[0].compiler, old_compiler);
        assert_eq!(snapshot.writers[0].entries, 3);
        assert_eq!(snapshot.writers[1].compiler, current);
        assert_eq!(snapshot.writers[1].entries, 1);
        assert!(
            snapshot.stale,
            "有产物不是当前编译器写的 ⇒ stale（提示 Rebuild；修前这里会静默说「都是 0.87.3」✗）"
        );
        assert_eq!(snapshot.current, current, "提示要能说出当前版本");

        let _ = std::fs::remove_dir_all(&root);
    }
}
