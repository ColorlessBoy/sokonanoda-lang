//! Batch checking: parse and compile a source, then report events or diagnostics.
//!
//! 有 `import` 的文件走**项目闭包**（`front::project::compile_project`）；
//! 没有 `import` 的文件走原来的单文件路径，输出逐字节不变（设计 A1）。

use std::path::{Path, PathBuf};

use crate::json_report::{print_json_line, report_json, span_json};
use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{
    compile_all_with, prelude_mode_from_source, CheckEvent, CompileOptions, CompileOutput,
    DocumentReport, PreludeMode,
};
use sokonanoda_front::project::{compile_project, ProjectReport};
use sokonanoda_front::{parse, FolFile};

/// 批检查的请求（`--root` / `--no-project` 只对有 `import` 的文件生效）。
pub(crate) struct CheckRequest<'a> {
    pub src: &'a str,
    pub label: &'a str,
    pub json: bool,
    pub bare: bool,
    /// `--root <dir>`：显式模块根。
    pub root: Option<PathBuf>,
    /// `--no-project`：不读 `sokonanoda.toml`（模块根 = 入口文件目录）。
    pub no_project: bool,
}

pub(crate) fn check_source(request: CheckRequest<'_>) -> bool {
    let CheckRequest {
        src,
        label,
        json,
        bare,
        root,
        no_project,
    } = request;
    let file = match parse(src) {
        Ok(file) => file,
        Err(diag) => {
            if json {
                print_json_line(&serde_json::json!({
                    "type": "diagnostic",
                    "stage": "parse",
                    "code": diag.code(),
                    "message": diag.message,
                    "hint": diag.hint(),
                    "span": span_json(diag.span),
                }));
            } else {
                eprintln!(
                    "{}:{}:{}: error[{}]: {}",
                    label,
                    diag.span.start.line,
                    diag.span.start.column,
                    diag.stage_code(),
                    diag.message
                );
            }
            return false;
        }
    };
    // Prelude choice: an explicit `--bare` flag wins; otherwise a file-level
    // `-- sokonanoda:prelude none` comment directive decides; default Full.
    let prelude = if bare {
        PreludeMode::Bare
    } else {
        prelude_mode_from_source(src)
    };
    let options = CompileOptions { prelude };

    // 有 import ⇒ 项目闭包；没有 ⇒ 单文件（今天的行为，逐字节不变）。
    if file.commands.iter().any(|command| command.is_import()) {
        let Some(entry) = entry_path(label, root.as_deref()) else {
            emit_stdin_without_root(json, label);
            return false;
        };
        let root_override = if no_project {
            entry
                .parent()
                .map(Path::to_path_buf)
                .or_else(|| Some(PathBuf::from(".")))
        } else {
            root.clone()
        };
        let project = compile_project(&entry, Some(src), &options, root_override.as_deref());
        return report_project(&project, src, json);
    }

    let (output, _report) = compile_cached(&file, src, &options);
    if json {
        report_json(&output, src);
    } else {
        report_output(&output, src, 0);
    }
    output.errors.is_empty()
}

/// 入口路径：命令行给的路径，或 `--root` 下的占位入口（stdin / `--text`）。
fn entry_path(label: &str, root: Option<&Path>) -> Option<PathBuf> {
    if label != "<stdin>" && label != "-" {
        return Some(PathBuf::from(label));
    }
    root.map(|root| root.join("Main.sokonanoda"))
}

/// stdin 里有 `import` 却没有 `--root`：没有位置就没有模块根（设计 §4.12）。
fn emit_stdin_without_root(json: bool, label: &str) {
    let message = "标准输入里的 `import` 需要一个模块根";
    let hint =
        "用 `--root <dir>` 指定模块根，或把文件落到磁盘（`sokonanoda path/to/Main.sokonanoda`）。";
    if json {
        print_json_line(&serde_json::json!({
            "type": "diagnostic",
            "stage": "import",
            "code": "import-not-found",
            "message": message,
            "hint": hint,
        }));
    } else {
        eprintln!("{label}:1:1: error[import-not-found]: {message}");
        eprintln!("  hint: {hint}");
    }
}

/// 渲染一次项目闭包编译：入口照旧（事件/诊断），**依赖模块**的诊断带文件路径。
fn report_project(project: &ProjectReport, entry_src: &str, json: bool) -> bool {
    let entry = project.entry_module();
    if let Some(entry) = entry {
        if json {
            report_json(&entry.events, entry_src);
        } else {
            report_output(&entry.events, entry_src, 0);
        }
    }
    for module in project.modules.iter().rev().skip(1).rev() {
        report_module_diagnostics(module, json);
    }
    if !json {
        if let Some(note) = &project.requires_warning {
            eprintln!("warning[manifest-version]: {note}");
        }
    }
    let entry_ok = entry.is_none_or(|module| module.events.errors.is_empty());
    entry_ok && !project.has_errors()
}

/// 依赖模块的错误/警告：人类视图带 `path:line:col`，JSON 视图带 `file`/`module`。
fn report_module_diagnostics(module: &sokonanoda_front::project::ModuleReport, json: bool) {
    let path = module
        .path
        .display()
        .to_string()
        .trim_start_matches("./")
        .to_string();
    for warning in &module.report.warnings {
        if json {
            print_json_line(&serde_json::json!({
                "type": "warning",
                "file": path,
                "module": module.name,
                "code": warning.code(),
                "message": warning.message,
                "hint": warning.hint(),
                "span": span_json(warning.span),
            }));
        } else {
            eprintln!(
                "{}:{}:{}: warning[{}]: {}",
                path,
                warning.span.start.line,
                warning.span.start.column,
                warning.code(),
                warning.message
            );
        }
    }
    for error in &module.report.errors {
        if json {
            print_json_line(&serde_json::json!({
                "type": "diagnostic",
                "file": path,
                "module": module.name,
                "stage": error.stage().code(),
                "code": error.code(),
                "message": error.message,
                "hint": error.hint(),
                "span": span_json(error.span),
            }));
        } else {
            eprintln!(
                "{}:{}:{}: error[{}]: {}",
                path,
                error.span.start.line,
                error.span.start.column,
                error.code(),
                error.message
            );
        }
    }
}

/// Compile `file` (parsed from `src`) through the shared persistent cache,
/// returning the same `(CompileOutput, DocumentReport)` a cold
/// `compile_all_with` would. A hit only ever replays a report the kernel
/// produced for the exact same `(version, build, prelude mode, source)`, so
/// event ordering and error/warning handling are unchanged.
pub(crate) fn compile_cached(
    file: &FolFile,
    src: &str,
    options: &CompileOptions,
) -> (CompileOutput, DocumentReport) {
    if let Some(entry) = cache::load(src, options) {
        if let Some(output) = entry.output {
            return (output, entry.report);
        }
    }
    let (output, report) = compile_all_with(file, options);
    cache::store(
        src,
        options,
        &CachedCompile {
            report: report.clone(),
            output: Some(output.clone()),
        },
    );
    (output, report)
}

pub(crate) fn report_output(output: &CompileOutput, src: &str, seen_events: usize) {
    for event in output.events.iter().skip(seen_events) {
        match event {
            CheckEvent::DeclarationChecked { name } => println!("checked declaration {name}"),
            CheckEvent::ExampleChecked => println!("checked example"),
            CheckEvent::TypeChecked { text, span } => {
                println!("{}: {text}", expr_text(src, *span));
            }
            CheckEvent::Reduced { text, span } => {
                println!("{} => {text}", expr_text(src, *span));
            }
            CheckEvent::Printed { name, text } => println!("#print {name} :\n{text}"),
            CheckEvent::ExerciseOpen { .. } => println!("exercise open (fill the sorry)"),
        }
    }
    for warning in &output.warnings {
        eprintln!(
            "{}:{}: warning[{}]: {}",
            warning.span.start.line,
            warning.span.start.column,
            warning.code(),
            warning.message
        );
    }
    for err in &output.errors {
        eprintln!(
            "{}:{}: error[{}]: {}",
            err.span.start.line,
            err.span.start.column,
            err.code(),
            err.message
        );
    }
}

pub(crate) fn expr_text(src: &str, span: sokonanoda_front::Span) -> &str {
    if span.end.offset <= src.len() {
        &src[span.start.offset..span.end.offset]
    } else {
        "<expr>"
    }
}
