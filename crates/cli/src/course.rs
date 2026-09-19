//! `sokonanoda course <course.json>`: aggregate the units of the course
//! manifest (the agent-facing material library) into a progress map
//! (docs/design/course-status.md; event contract in docs/protocol.md).
//!
//! Units with `import` go through the **project closure** (WO-007 / G-06): the
//! same closure, module root and cache digest as `grade`/`query check`/`build`.
//! A unit without `import` keeps the single-file pipeline byte for byte.
//!
//! Progress is not an error: open/failed exercises still exit 0 — only an
//! unreadable manifest fails.

use sokonanoda_front::compile::{prelude_mode_from_source, CheckEvent, CompileOptions};
use sokonanoda_front::project::find_manifest;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Default, PartialEq)]
struct UnitCounts {
    checked: usize,
    open: usize,
    failed: usize,
    reduced: usize,
}

pub(crate) fn course(manifest: &str, json: bool) -> ExitCode {
    // 单元路径相对**清单文件所在目录**解析。清单路径先绝对化（失败则退回原串），
    // 单元路径随之绝对化：`course` 因此不依赖 cwd，也不必借道通用模块根发现
    // （G-12 的地盘是 `find_manifest`/`module_root`，这里只是调用方自保）。
    // 错误信息仍用用户给的 `manifest` 原串。
    let manifest_path = std::fs::canonicalize(manifest).unwrap_or_else(|_| PathBuf::from(manifest));
    let raw = match std::fs::read_to_string(&manifest_path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("error: cannot read course manifest {manifest}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) else {
        eprintln!("error: {manifest} is not a course manifest (JSON array expected)");
        return ExitCode::FAILURE;
    };

    let base = manifest_path.parent().unwrap_or(Path::new("."));
    let mut totals = UnitCounts::default();
    let mut units = 0usize;
    for entry in &entries {
        let file = entry["file"].as_str().unwrap_or_default();
        let title = entry["title"].as_str().unwrap_or("（无标题）");
        let unit = entry["unit"].as_u64().unwrap_or(0);
        units += 1;

        let unit_path = base.join(file);
        let counts = std::fs::read_to_string(&unit_path)
            .map_err(|e| format!("cannot read: {e}"))
            .and_then(|src| {
                count_unit(&unit_path, base, &src).map_err(|e| format!("cannot compile: {e}"))
            });
        match counts {
            Ok(counts) => {
                totals.checked += counts.checked;
                totals.open += counts.open;
                totals.failed += counts.failed;
                totals.reduced += counts.reduced;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "type": "course.unit",
                            "file": file,
                            "title": title,
                            "unit": unit,
                            "checked": counts.checked,
                            "open": counts.open,
                            "failed": counts.failed,
                            "reduced": counts.reduced,
                        })
                    );
                } else {
                    println!(
                        "unit {unit} {title} —— {} checked · {} open · {} failed",
                        counts.checked, counts.open, counts.failed
                    );
                }
            }
            Err(message) => {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "type": "course.unit",
                            "file": file,
                            "title": title,
                            "unit": unit,
                            "error": message,
                        })
                    );
                } else {
                    println!("unit {unit} {title} —— 错误：{message}");
                }
            }
        }
    }
    if json {
        println!(
            "{}",
            serde_json::json!({
                "type": "course.summary",
                "units": units,
                "checked": totals.checked,
                "open": totals.open,
                "failed": totals.failed,
            })
        );
    } else {
        println!(
            "共 {} 单元 —— {} checked · {} open · {} failed",
            units, totals.checked, totals.open, totals.failed
        );
    }
    ExitCode::SUCCESS
}

/// One unit's counts.
///
/// * **no `import`**: the single-file pipeline, byte-identical to the pre-WO-007
///   behaviour (`compile_cached`, whose cache key is the source text alone);
/// * **with `import`**: the **project closure** — same module root, same prelude
///   mode and same `ProjectPlan::digest` cache key as `grade`/`query check`/
///   `build` (`crate::project_cache`); the counts come from the entry module
///   only, and `failed` sums the closure because `attach_diagnostics` hangs
///   project-level errors on some module (`failed == 0` ⇔ `grade` exits 0).
fn count_unit(path: &Path, course_dir: &Path, src: &str) -> Result<UnitCounts, String> {
    // Reuse the CLI checker's parse stage so manifest maps never crash on
    // unparseable units; compile_cached needs the parsed file anyway.
    let file = sokonanoda_front::parse(src).map_err(|e| e.message.to_string())?;
    // Prelude choice: a file-level `-- sokonanoda:prelude` directive decides
    // (same as `check`/`query`/`build`).
    let options = CompileOptions {
        prelude: prelude_mode_from_source(src),
    };

    if !file.commands.iter().any(|command| command.is_import()) {
        let (out, _report) = crate::check::compile_cached(&file, src, &options);
        let mut counts = UnitCounts::default();
        tally(&mut counts, &out.events);
        counts.failed = out.errors.len();
        return Ok(counts);
    }

    let root_override = closure_root(path, course_dir);
    let (plan, digest) =
        crate::project_cache::plan(path, Some(src), root_override.as_deref(), &options);
    if let Some(cached) = crate::project_cache::load(&digest, &options) {
        if let Some(output) = cached.output {
            let mut counts = UnitCounts::default();
            tally(&mut counts, &output.events);
            counts.failed = output.errors.len();
            return Ok(counts);
        }
    }
    let project = sokonanoda_front::project::compile_plan(plan, &options);
    let mut counts = UnitCounts {
        failed: project
            .modules
            .iter()
            .map(|module| module.events.errors.len())
            .sum(),
        ..UnitCounts::default()
    };
    if let Some(entry) = project.entry_module() {
        // Counts are the **entry module's** events: a dependency's declarations
        // and exercises are not part of the unit's score.
        tally(&mut counts, &entry.events.events);
        // 只缓存干净的项目（与 `check`/`build`/`query` 同一份摘要键）。
        crate::project_cache::store_if_clean(&digest, &options, entry, project.is_clean());
    }
    Ok(counts)
}

/// 模块根策略（WO-007）：入口自己的最近 `sokonanoda.toml` 优先——位于
/// `course/unit11-project/` 这类子项目里的单元不会被课程根覆盖；找不到清单时
/// 回退到 **`course.json` 所在目录**（"清单目录即默认项目根"）。课程布局天然是
/// `<课程根>/{course.json,lib/,units/}`，只认入口目录会让 `import lib.Set` 去找
/// `<课程根>/units/lib/Set.sokonanoda`。
fn closure_root(entry: &Path, course_dir: &Path) -> Option<PathBuf> {
    let entry_dir = entry
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    match find_manifest(entry_dir) {
        // 交给 `plan_project` 的通用发现（就近的清单优先，含清单的 `src`）。
        Some(_) => None,
        None => Some(course_dir.to_path_buf()),
    }
}

/// 累加一个模块的事件。`example.checked` 归入忽略档（既有的 `course` 口径：
/// unit08 修好后仍是 `checked:15` 而不是 16）。
fn tally(counts: &mut UnitCounts, events: &[CheckEvent]) {
    for event in events {
        match event {
            CheckEvent::DeclarationChecked { .. } => counts.checked += 1,
            CheckEvent::ExerciseOpen { .. } => counts.open += 1,
            CheckEvent::Reduced { .. } => counts.reduced += 1,
            CheckEvent::ExampleChecked
            | CheckEvent::TypeChecked { .. }
            | CheckEvent::Printed { .. } => {}
        }
    }
}
