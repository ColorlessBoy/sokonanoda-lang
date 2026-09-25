//! `sokonanoda course <course.json> [<course.json> ...] [--all]`: aggregate the
//! units of one or more course manifests (the agent-facing material library)
//! into a progress map (docs/design/course-status.md; event contract in
//! docs/protocol.md).
//!
//! The manifest comes in two shapes and **both are read** (ledger G-07,
//! design `docs/design/course-manifest-v2.md`): the v1 flat array
//! (`course/course.json`) and the structured v2 object (`soko.course/2`,
//! `courses/set-theory/course.json`). Parsing lives in [`manifest`]; v1 stays
//! the reference behaviour — its events gain no new keys (additive-only).
//!
//! **Several manifests at once** (design §4.5, the §7 "不做" item): each
//! positional is a manifest file or a directory holding `course.json`, and
//! `--all` walks a directory recursively for every `course.json`. When more
//! than one manifest is aggregated each `course.unit` gains `manifest` (the
//! path as constructed from the arguments) and `course.summary` gains
//! `manifests`; a **single-manifest** run keeps the frozen unit shape and only
//! gains the `manifests: 1` count. A manifest that cannot be read or parsed
//! fails the whole run **before any event** — a batch never prints half a map.
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
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// 课程清单的约定文件名：目录参数与 `--all` 发现都用它。
const MANIFEST_FILE: &str = "course.json";

/// `--all` 递归时**不进入**的目录名（构建产物/依赖树）。隐藏目录
/// （`.` 开头）同样跳过——它们不是课程材料。
const SKIPPED_DIRS: &[&str] = &["target", "node_modules"];

#[derive(Debug, Default, PartialEq)]
struct UnitCounts {
    checked: usize,
    open: usize,
    failed: usize,
    reduced: usize,
}

/// 一份读完的清单：`display` 是**按调用实参拼出来的**路径（事件报它，可复现），
/// `base` 是单元相对路径的解析目录。
struct Loaded {
    display: String,
    base: PathBuf,
    course: Manifest,
}

pub(crate) fn course(paths: &[String], all: bool, json: bool) -> ExitCode {
    if paths.is_empty() && !all {
        eprintln!("usage: sokonanoda course <course.json> [<course.json> ...] [--all]");
        return ExitCode::FAILURE;
    }
    let sources = match discover_sources(paths, all) {
        Ok(sources) => sources,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    // 先全部读完再发事件：一批里有一份读不了/不是清单 ⇒ 整体失败且**零事件**
    // （不报半张表；与单清单时的行为一致）。
    let mut loaded: Vec<Loaded> = Vec::new();
    for (display, path) in sources {
        // 单元路径相对**清单文件所在目录**解析。清单路径先绝对化（失败则退回原串），
        // 单元路径随之绝对化：`course` 因此不依赖 cwd，也不必借道通用模块根发现
        // （G-12 的地盘是 `find_manifest`/`module_root`，这里只是调用方自保）。
        let manifest_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let raw = match std::fs::read_to_string(&manifest_path) {
            Ok(raw) => raw,
            Err(e) => {
                eprintln!("error: cannot read course manifest {display}: {e}");
                return ExitCode::FAILURE;
            }
        };
        let course = match manifest::parse(&raw) {
            Ok(course) => course,
            Err(reason) => {
                eprintln!(
                    "error: {display} is not a course manifest \
                     (v1 flat JSON array or a `{}` object expected): {reason}",
                    manifest::SCHEMA_V2
                );
                return ExitCode::FAILURE;
            }
        };
        let base = manifest_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        loaded.push(Loaded {
            display,
            base,
            course,
        });
    }

    let multi = loaded.len() > 1;
    let mut totals = UnitCounts::default();
    let mut units = 0usize;
    let mut volumes = 0usize;
    let mut chapters = 0usize;
    for course in &loaded {
        volumes += course.course.volumes;
        chapters += course.course.chapters;
        if multi && !json {
            println!("── {} ──", course.display);
        }
        for entry in &course.course.units {
            let file = entry.file.as_str();
            let title = entry.title.as_str();
            let unit = entry.unit;
            units += 1;

            let unit_path = course.base.join(file);
            let counts = std::fs::read_to_string(&unit_path)
                .map_err(|e| format!("cannot read: {e}"))
                .and_then(|src| {
                    count_unit(&unit_path, &course.base, &src)
                        .map_err(|e| format!("cannot compile: {e}"))
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
                        add_manifest_context(&mut event, course, multi);
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
                        add_manifest_context(&mut event, course, multi);
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
                "volumes": volumes,
                "chapters": chapters,
                // Multi-manifest aggregation (design §4.5): the number of
                // manifests this summary aggregates — `1` for the classic
                // single-manifest invocation.
                "manifests": loaded.len(),
            })
        );
    } else {
        let manifest_prefix = if multi {
            format!("{} 份清单 · ", loaded.len())
        } else {
            String::new()
        };
        println!(
            "共 {manifest_prefix}{units} 单元{} —— {} checked · {} open · {} failed",
            structure_suffix(volumes, chapters),
            totals.checked,
            totals.open,
            totals.failed
        );
    }
    ExitCode::SUCCESS
}

/// Expand the positional arguments into an ordered, de-duplicated list of
/// `(display, path)` manifest sources (design §4.5):
///
/// * a **file** is that manifest (a missing path stays a file — the read below
///   reports it, which is the pre-aggregation behaviour);
/// * a **directory** is `<dir>/course.json`;
/// * with `--all`, a directory is walked **recursively** for every
///   `course.json` (sorted by path; hidden directories, `target/` and
///   `node_modules/` skipped). A root with no manifest is an error — an empty
///   report would look like a green course.
///
/// The same manifest given twice (file + directory, or two spellings of one
/// path) is reported **once**; the first spelling wins.
fn discover_sources(paths: &[String], all: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let requested: Vec<String> = if paths.is_empty() {
        vec![".".to_string()]
    } else {
        paths.to_vec()
    };
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for raw in &requested {
        let path = PathBuf::from(raw);
        if path.is_dir() {
            if all {
                let found = walk_manifests(&path);
                if found.is_empty() {
                    return Err(format!(
                        "--all found no {MANIFEST_FILE} under {raw} \
                         (a course directory holds one; an empty root is an error, not an empty map)"
                    ));
                }
                for found_path in found {
                    let display = found_path.to_string_lossy().into_owned();
                    push_source(&mut out, &mut seen, display, found_path);
                }
            } else {
                let manifest = path.join(MANIFEST_FILE);
                if !manifest.is_file() {
                    return Err(format!(
                        "no {MANIFEST_FILE} in directory {raw} \
                         (give a manifest file, or use --all to search recursively)"
                    ));
                }
                let display = manifest.to_string_lossy().into_owned();
                push_source(&mut out, &mut seen, display, manifest);
            }
        } else {
            push_source(&mut out, &mut seen, raw.clone(), path);
        }
    }
    Ok(out)
}

fn push_source(
    out: &mut Vec<(String, PathBuf)>,
    seen: &mut HashSet<PathBuf>,
    display: String,
    path: PathBuf,
) {
    // 去重按**规范化路径**（同一份清单的两种拼写只算一次）；路径还不存在时
    // 退回原串——缺失的清单由读取那一步报错，不在这里静默吞掉。
    let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    if seen.insert(key) {
        out.push((display, path));
    }
}

/// Every `course.json` under `root`, recursively, in sorted order.
///
/// Directory symlinks are not followed (`symlink_metadata`), so a symlink loop
/// cannot hang the walk; hidden directories, `target/` and `node_modules/` are
/// skipped because they never hold course material.
fn walk_manifests(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if name.starts_with('.') || SKIPPED_DIRS.contains(&name) {
                    continue;
                }
                stack.push(path);
            } else if metadata.is_file()
                && path.file_name().and_then(|name| name.to_str()) == Some(MANIFEST_FILE)
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Add the aggregation context to a `course.unit` event — **only when more than
/// one manifest is aggregated**: with a single manifest the unit's manifest is
/// unambiguous, and the frozen event shape stays byte-for-byte (design §4.5).
fn add_manifest_context(event: &mut serde_json::Value, course: &Loaded, multi: bool) {
    if multi {
        event["manifest"] = serde_json::json!(course.display);
    }
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

/// Human totals line: `（1 卷 4 章）` for v2, nothing for v1.
fn structure_suffix(volumes: usize, chapters: usize) -> String {
    if volumes == 0 && chapters == 0 {
        return String::new();
    }
    format!("（{volumes} 卷 {chapters} 章）")
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
        crate::project_cache::plan(path, Some(src), root_override.as_deref(), &[], &options);
    // R-3（T-B5）：模块根产物目录优先，全局缓存兜底。
    let artifacts_root = plan.root.clone();
    if let Some(cached) = crate::project_cache::load_at(&artifacts_root, &digest, &options) {
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
        crate::project_cache::store_if_clean_at(&artifacts_root, &digest, &options, &project);
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
