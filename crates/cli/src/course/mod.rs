//! `sokonanoda course <course.json>`: aggregate the units of the course
//! manifest (the agent-facing material library) into a progress map
//! (docs/design/course-status.md; event contract in docs/protocol.md).
//!
//! The manifest comes in two shapes and **both are read** (ledger G-07,
//! design `docs/design/course-manifest-v2.md`): the v1 flat array
//! (`course/course.json`) and the structured v2 object (`soko.course/2`,
//! `courses/set-theory/course.json`). Parsing lives in [`manifest`]; v1 stays
//! the reference behaviour — its events gain no new keys (additive-only).
//!
//! Units with `import` go through the **project closure** (WO-007 / G-06): the
//! same closure, module root and cache digest as `grade`/`query check`/`build`.
//! A unit without `import` keeps the single-file pipeline byte for byte.
//!
//! Progress is not an error: open/failed exercises still exit 0 — only an
//! unreadable manifest fails.

mod manifest;

use manifest::Manifest;
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
    let course = match manifest::parse(&raw) {
        Ok(course) => course,
        Err(reason) => {
            eprintln!(
                "error: {manifest} is not a course manifest \
                 (v1 flat JSON array or a `{}` object expected): {reason}",
                manifest::SCHEMA_V2
            );
            return ExitCode::FAILURE;
        }
    };

    let base = manifest_path.parent().unwrap_or(Path::new("."));
    let mut totals = UnitCounts::default();
    let mut units = 0usize;
    for entry in &course.units {
        let file = entry.file.as_str();
        let title = entry.title.as_str();
        let unit = entry.unit;
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
                    let mut event = serde_json::json!({
                        "type": "course.unit",
                        "file": file,
                        "title": title,
                        "unit": unit,
                        "checked": counts.checked,
                        "open": counts.open,
                        "failed": counts.failed,
                        "reduced": counts.reduced,
                    });
                    add_v2_context(&mut event, entry);
                    println!("{event}");
                } else {
                    println!(
                        "unit {unit}{} {title} —— {} checked · {} open · {} failed",
                        chapter_suffix(entry),
                        counts.checked,
                        counts.open,
                        counts.failed
                    );
                }
            }
            Err(message) => {
                if json {
                    let mut event = serde_json::json!({
                        "type": "course.unit",
                        "file": file,
                        "title": title,
                        "unit": unit,
                        "error": message,
                    });
                    add_v2_context(&mut event, entry);
                    println!("{event}");
                } else {
                    println!(
                        "unit {unit}{} {title} —— 错误：{message}",
                        chapter_suffix(entry)
                    );
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
                // v2 additions (ledger G-07): always present, `0` for a v1
                // flat manifest — a count is a number, not a presence flag.
                "volumes": course.volumes,
                "chapters": course.chapters,
            })
        );
    } else {
        println!(
            "共 {} 单元{} —— {} checked · {} open · {} failed",
            units,
            structure_suffix(&course),
            totals.checked,
            totals.open,
            totals.failed
        );
    }
    ExitCode::SUCCESS
}

/// Add the v2 context to a `course.unit` event — **only when the manifest
/// actually carries it** (ledger G-07: additive-only; a v1 unit gains no key).
fn add_v2_context(event: &mut serde_json::Value, entry: &manifest::UnitEntry) {
    let Some(volume) = &entry.volume else {
        return;
    };
    event["volume"] = serde_json::json!({"id": volume.id, "title": volume.title});
    if let Some(chapter) = &entry.chapter {
        event["chapter"] = serde_json::json!({
            "id": chapter.id,
            "title": chapter.title,
            "tags": chapter.tags,
        });
        // Flat copy for consumers that filter by tag without descending
        // (design `course-manifest-v2.md` §4.1).
        event["tags"] = serde_json::json!(chapter.tags);
    }
}

/// Human view: `（卷 I 集合论 / I.1 集合、子集与集合运算）` for a v2 unit,
/// nothing at all for a v1 one.
fn chapter_suffix(entry: &manifest::UnitEntry) -> String {
    match (&entry.volume, &entry.chapter) {
        (Some(volume), Some(chapter)) => {
            format!("（{} / {} {}）", volume.title, chapter.id, chapter.title)
        }
        (Some(volume), None) => format!("（{}）", volume.title),
        _ => String::new(),
    }
}

/// Human totals line: `· 1 卷 4 章` for v2, nothing for v1.
fn structure_suffix(manifest: &Manifest) -> String {
    if manifest.volumes == 0 && manifest.chapters == 0 {
        return String::new();
    }
    format!("（{} 卷 {} 章）", manifest.volumes, manifest.chapters)
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
    // Prelude choice: a file-level `-- sokonanoda:prelude` directive decides
    // (same as `check`/`query`/`build`).
    let options = CompileOptions {
        prelude: prelude_mode_from_source(src),
    };

    // 分发用 `is_project_source`（G-04 第二刀）：入口**单独 parse 失败**但写了
    // `import` 时也走闭包——它可能用了依赖声明的记法，闭包路径能编。
    if !sokonanoda_front::project::is_project_source(src) {
        // Reuse the CLI checker's parse stage so manifest maps never crash on
        // unparseable units; compile_cached needs the parsed file anyway.
        let file = sokonanoda_front::parse(src).map_err(|e| e.message.to_string())?;
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
