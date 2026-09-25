//! `sokonanoda build [--clean] [<path> ...]`: warm (and inspect) the shared
//! persistent compile cache so later `check`/`course`/LSP runs are hits
//! (docs/design/compile-cache.md; event contract in docs/protocol.md).
//!
//! A positional may be a `.sokonanoda` file or a directory (walked
//! recursively, sorted). No positional means the current directory. Exit is 0
//! when at least one file resolved; a path that resolves to nothing is usage
//! (FAILURE). Parse/read failures are counted, not fatal.

use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{compile_all_with, prelude_mode_from_source, CompileOptions};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn build(
    args: &[String],
    json: bool,
    clean: bool,
    root: Option<&str>,
    no_project: bool,
) -> ExitCode {
    if clean {
        // R-3（T-B5）：项目条目现在落在**模块根**的 `.sokonanoda/compiled/`，所以
        // `--clean` 必须**两处都清** —— 只清全局的话 `rebuild`（= `build --clean`）
        // 会命中项目条目 ⇒ 表面"清空了"，实际什么都没重编（用户可见的假动作 ✗）。
        let global = cache::clean();
        let mut project = 0usize;
        for root in project_roots(args) {
            project += sokonanoda_front::project::cache::clean_at(&root);
        }
        let removed = global + project;
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "type": "build.clean",
                    "removed": removed,
                    // additive：老消费者读 `removed`（= 两处之和）语义不变 ✓
                    "global": global,
                    "project": project,
                })
            );
        } else {
            println!("removed {removed} cached file(s) ({global} global, {project} project)");
        }
        return ExitCode::SUCCESS;
    }

    let roots: Vec<PathBuf> = if args.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("usage: sokonanoda build [--json] [--clean] [<file.sokonanoda> | <dir> ...]");
        return ExitCode::FAILURE;
    }

    let mut hit = 0usize;
    let mut compiled = 0usize;
    let mut failed = 0usize;
    for file in &files {
        let status = std::fs::read_to_string(file)
            .map_err(|e| format!("cannot read: {e}"))
            .and_then(|src| build_one(file, &src, root, no_project));
        let status = match status {
            Ok(status) => status,
            Err(message) => {
                eprintln!("error: {}: {message}", file.display());
                "failed"
            }
        };
        match status {
            "hit" => hit += 1,
            "compiled" => compiled += 1,
            _ => failed += 1,
        }
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "type": "build.file",
                    "file": file.display().to_string(),
                    "status": status,
                })
            );
        }
    }

    let total = hit + compiled + failed;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "type": "build.summary",
                "files": total,
                "hit": hit,
                "compiled": compiled,
                "failed": failed,
            })
        );
    } else {
        println!("built {total} file(s) — {hit} hit, {compiled} compiled, {failed} failed");
    }
    ExitCode::SUCCESS
}

/// `--clean` 用：从位置参数解析出**模块根**（项目入口的 `plan.root`）并去重。
///
/// 无参数（`build --clean`）⇒ 返回空 ⇒ 只清全局缓存，并在输出里如实报告
/// `project=0`（设计 §3.7：解不出模块根时不清项目产物，但不假装清了）。
fn project_roots(args: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for arg in args {
        collect_files(Path::new(arg), &mut files);
    }
    let mut roots: Vec<PathBuf> = Vec::new();
    for file in files {
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        if !sokonanoda_front::project::is_project_source(&src) {
            continue;
        }
        let root = sokonanoda_front::project::plan_project(&file, Some(&src), None).root;
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots
}

/// Compile one source through the cache, returning `"hit"`, `"compiled"` or
/// `"failed"`. Options mirror the batch checker (file-directive prelude mode)
/// so a `build` warms exactly the entries `check`/`course` later load.
fn build_one(
    path: &Path,
    src: &str,
    root: Option<&str>,
    no_project: bool,
) -> Result<&'static str, String> {
    let options = CompileOptions {
        prelude: prelude_mode_from_source(src),
    };
    // 有 import ⇒ 项目闭包（v1 不进缓存：闭包键在 P4 落地；现在宁可重编译，
    // 也不拿单文件键去缓存一个依赖别人环境的报告）。
    //
    // 分发用 `is_project_source`：入口**单独 parse 失败**但写了 `import` 时也走
    // 闭包——入口可能用了依赖声明的记法（G-04 第二刀），闭包路径能编。
    if sokonanoda_front::project::is_project_source(src) {
        let root_override = if no_project {
            path.parent().map(Path::to_path_buf)
        } else {
            root.map(PathBuf::from)
        };
        let plan =
            sokonanoda_front::project::plan_project(path, Some(src), root_override.as_deref());
        let digest = plan.digest(&options);
        // R-3（T-B5）：项目条目落**模块根**的 `.sokonanoda/compiled/`
        // （`plan.root` 是发现规则算出来的模块根，这里现成）。
        let artifacts_root = plan.root.clone();
        if let Some(entry) =
            sokonanoda_front::project::cache::load_at(&artifacts_root, &digest, &options)
        {
            if entry.output.is_some() {
                return Ok("hit");
            }
        }
        let project = sokonanoda_front::project::compile_plan(plan, &options);
        let ok = project
            .entry_module()
            .is_none_or(|module| module.events.errors.is_empty())
            && !project.has_errors();
        if ok && project.is_clean() {
            sokonanoda_front::project::cache::store_at(
                &artifacts_root,
                &digest,
                &options,
                &project,
            );
        }
        return Ok(if ok { "compiled" } else { "failed" });
    }
    let parsed = match sokonanoda_front::parse(src) {
        Ok(parsed) => parsed,
        Err(diag) => {
            return Err(format!(
                "{}:{}: error[{}]: {}",
                diag.span.start.line,
                diag.span.start.column,
                diag.stage_code(),
                diag.message
            ));
        }
    };
    if let Some(entry) = cache::load(src, &options) {
        if entry.output.is_some() {
            return Ok("hit");
        }
    }
    let (output, report) = compile_all_with(&parsed, &options);
    cache::store(
        src,
        &options,
        &CachedCompile {
            report,
            output: Some(output),
            // 单文件条目（无项目报告）。
            project: None,
        },
    );
    Ok("compiled")
}

/// Recursively collect `*.sokonanoda` files under `path` (sorted per level).
fn collect_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = match std::fs::read_dir(path) {
            Ok(read) => read.filter_map(|e| e.ok().map(|e| e.path())).collect(),
            Err(e) => {
                eprintln!("error: cannot read directory {}: {e}", path.display());
                return;
            }
        };
        entries.sort();
        for entry in entries {
            collect_files(&entry, out);
        }
    } else if path
        .extension()
        .map(|ext| ext == "sokonanoda")
        .unwrap_or(false)
    {
        out.push(path.to_path_buf());
    }
}
