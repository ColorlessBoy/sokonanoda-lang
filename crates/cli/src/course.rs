//! `sokonanoda course <course.json>`: aggregate the units of the course
//! manifest (the agent-facing material library) into a progress map
//! (docs/design-course-status.md; event contract in docs/protocol.md).
//!
//! Progress is not an error: open/failed exercises still exit 0 — only an
//! unreadable manifest fails.

use sokonanoda_front::compile::{compile_fol_with, CheckEvent, CompileOptions};
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
    let manifest_path = PathBuf::from(manifest);
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
            .and_then(|src| count_unit(&src).map_err(|e| format!("cannot compile: {e}")));
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

/// One unit's counts, from a single full compile (same pipeline as the
/// course golden tests: full prelude, events counted, errors = failures).
fn count_unit(src: &str) -> Result<UnitCounts, String> {
    // Reuse the CLI checker's parse stage so manifest maps never crash on
    // unparseable units; compile_fol needs the parsed file anyway.
    let file = sokonanoda_front::parse(src).map_err(|e| e.message.to_string())?;
    let out = compile_fol_with(&file, &CompileOptions::default());
    let mut counts = UnitCounts {
        failed: out.errors.len(),
        ..UnitCounts::default()
    };
    for event in &out.events {
        match event {
            CheckEvent::DeclarationChecked { .. } => counts.checked += 1,
            CheckEvent::ExerciseOpen { .. } => counts.open += 1,
            CheckEvent::Reduced { .. } => counts.reduced += 1,
            CheckEvent::ExampleChecked
            | CheckEvent::TypeChecked { .. }
            | CheckEvent::Printed { .. } => {}
        }
    }
    Ok(counts)
}
