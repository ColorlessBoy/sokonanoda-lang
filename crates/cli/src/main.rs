//! CLI entry: argument parsing, dispatch to file-check / repl / lsp / course.

mod build;
mod check;
mod course;
mod env;
mod help;
mod json_report;
mod project_cache;
mod query;
mod repl;
mod watch;

use check::check_source;
use help::print_help;
use repl::repl;
use std::io::{IsTerminal, Read};
use std::process::ExitCode;
use watch::{watch_doc, watch_workspace};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json = false;
    let mut bare = false;
    let mut force = false;
    let mut clean = false;
    let mut doc: Option<String> = None;
    let mut workspace: Option<String> = None;
    let mut root: Option<String> = None;
    let mut no_project = false;
    let mut all = false;
    let mut positionals: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--bare" => bare = true,
            "--force" => force = true,
            "--clean" => clean = true,
            "--doc" => match args.get(i + 1) {
                Some(path) => {
                    doc = Some(path.clone());
                    i += 1;
                }
                None => {
                    eprintln!("error: --doc requires a path");
                    return ExitCode::FAILURE;
                }
            },
            "--root" => match args.get(i + 1) {
                Some(path) => {
                    root = Some(path.clone());
                    i += 1;
                }
                None => {
                    eprintln!("error: --root requires a directory");
                    return ExitCode::FAILURE;
                }
            },
            "--no-project" => no_project = true,
            // 多清单聚合（设计 `docs/design/course-manifest-v2.md` §4.5）：
            // 只对 `course` 有意义，下面有专门的形状检查。
            "--all" => all = true,
            "--workspace" => match args.get(i + 1) {
                Some(root) => {
                    workspace = Some(root.clone());
                    i += 1;
                }
                None => {
                    eprintln!("error: --workspace requires a root directory");
                    return ExitCode::FAILURE;
                }
            },
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("sokonanoda {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            _ => positionals.push(args[i].clone()),
        }
        i += 1;
    }
    if (doc.is_some() || workspace.is_some())
        && positionals.first().map(String::as_str) != Some("watch")
    {
        eprintln!("error: --doc/--workspace are only valid with `sokonanoda watch`");
        return ExitCode::FAILURE;
    }
    if all && positionals.first().map(String::as_str) != Some("course") {
        eprintln!("error: --all is only valid with `sokonanoda course`");
        return ExitCode::FAILURE;
    }
    match positionals.first().map(String::as_str) {
        // Environment subcommands (binary replacement for scripts/soko.sh).
        Some("version") => env::version_cmd(json),
        Some("doctor") => env::doctor(json),
        Some("setup") => env::setup(force),
        Some("update") => env::update(),
        Some("grade") => env::grade(&positionals[1..]),
        Some("gate") => env::gate(&args),
        // 内核真相查询（agent/MCP 的"提问式"通道；单 JSON 对象）。
        // 设计：docs/design/agent-query-channel.md §5。
        Some("query") if !json => query::run(&positionals[1..], root.as_deref()),
        Some("query") => {
            eprintln!("error: query 自带 JSON 输出，不要加 --json");
            ExitCode::FAILURE
        }
        Some("build") => build::build(&positionals[1..], json, clean, root.as_deref(), no_project),
        // **`clean` / `rebuild`**（用户 2026-10-08：「rebuild 和 clean 没有实现」✗）：
        // 与扩展的三条命令**一一对应**（Build / Rebuild / Clean Cache ✓）——
        // `clean` ≡ `build --clean`（只清不编）· `rebuild` = 先清两处再预热 ✓。
        // `build --clean` 一字不动地保留（向后兼容 ✓，文档与脚本都还在用它 ✓）。
        Some("clean") => build::clean(&positionals[1..], json),
        Some("rebuild") => build::rebuild(&positionals[1..], json, root.as_deref(), no_project),
        Some("repl") if !json => repl(),
        Some("repl") => {
            eprintln!("error: --json is only supported for batch checking, not the repl");
            ExitCode::FAILURE
        }
        // 单二进制分发（gleam 模式）：编辑器可用 `sokonanoda lsp` 拉起服务器。
        // stdout 是 LSP 协议帧；任何提示只进 stderr。
        Some("lsp") if !json => {
            if std::io::stdin().is_terminal() {
                eprintln!("sokonanoda lsp: starting the language server on stdio (editors spawn this; you probably want `sokonanoda repl`)");
            }
            sokonanoda_lsp::run();
            ExitCode::SUCCESS
        }
        Some("lsp") => {
            eprintln!("error: --json is only supported for batch checking, not lsp");
            ExitCode::FAILURE
        }
        Some("watch") => match (doc.as_deref(), workspace.as_deref()) {
            (Some(_), Some(_)) => {
                eprintln!("error: --doc and --workspace are mutually exclusive");
                ExitCode::FAILURE
            }
            (Some(path), None) => watch_doc(path),
            (None, Some(root)) => watch_workspace(root),
            (None, None) => match positionals.get(1) {
                Some(path) => watch_doc(path),
                None => {
                    eprintln!(
                        "usage: sokonanoda watch <file.sokonanoda> | --doc <file> | --workspace <root>"
                    );
                    ExitCode::FAILURE
                }
            },
        },
        Some("course") => course::course(&positionals[1..], all, json),
        None => check_path_or_stdin(None, json, bare, root, no_project),
        // **未知子命令**（用户 2026-10-08：「cli 很多命令有问题」✗）：以前**任何**没匹配上的
        // 位置参数都被当成**文件路径** ⇒ `sokonanoda rebuild` 报的是
        // `error: No such file or directory (os error 2)`（裸 OS 错误、不说是命令 ✗、
        // 退出码还是 1 = "有拒绝" ✗）。现在按**形状**分流 ✓：
        //  * 磁盘上**存在** ⇒ 照旧当文件检查 ✓（`sokonanoda file.sokonanoda` 是文档写法 ✓）；
        //  * 不存在、且长得像**命令词**（无扩展名、无路径分隔符）⇒ 明说"未知子命令" +
        //    指路 `--help`，退出码 **2 = 用法错误** ✓（`docs/protocol.md` 的退出码契约 ✓）。
        Some(p) if looks_like_an_unknown_command(p) => {
            eprintln!("error: 未知子命令 `{p}`");
            eprintln!(
                "提示：`sokonanoda --help` 列出全部子命令；要检查一个文件请写 `sokonanoda <file.sokonanoda>`"
            );
            ExitCode::from(2)
        }
        Some(p) => check_path_or_stdin(Some(p), json, bare, root, no_project),
    }
}

/// 「长得像子命令、但磁盘上没有这个东西」⇒ 报**未知子命令**而不是裸 OS 错误 ✓。
///
/// 判据（三条都要满足）：① 不以 `-` 开头（那是选项）· ② 磁盘上不存在 ✓ ·
/// ③ 没有路径分隔符、也没有 `.sokonanoda` 后缀（否则它**是**一个路径，只是不存在 ✓ ——
/// 那种情况该报"读不到 <path>"，见 [`check_path_or_stdin`] ✓）。
fn looks_like_an_unknown_command(token: &str) -> bool {
    !token.starts_with('-')
        && !token.ends_with(".sokonanoda")
        && !token.contains('/')
        && !token.contains('\\')
        && !std::path::Path::new(token).exists()
}

fn check_path_or_stdin(
    arg: Option<&str>,
    json: bool,
    bare: bool,
    root: Option<String>,
    no_project: bool,
) -> ExitCode {
    let mut src = String::new();
    let read_result = match arg {
        None | Some("-") => std::io::stdin().read_to_string(&mut src),
        Some(path) => std::fs::read_to_string(path).map(|s| {
            src = s;
            src.len()
        }),
    };
    if let Err(e) = read_result {
        // 2026-10-08（用户「cli 很多命令有问题」）：以前是裸 `error: {e}` ✗ ——
        // 连**哪个**文件都看不出来（脚本/批量里无法定位 ✓）。现在指名道姓 ✓，
        // 且**目录**给一条出路提示 ✓（`sokonanoda courses` 以前只说 `Is a directory` ✗）。
        match arg {
            Some(path) => {
                eprintln!("error: 读不到 {path}：{e}");
                if std::path::Path::new(path).is_dir() {
                    eprintln!(
                        "提示：`{path}` 是目录 —— 要编译整个目录请用 `sokonanoda build {path}`"
                    );
                }
            }
            _ => eprintln!("error: {e}"),
        }
        return ExitCode::FAILURE;
    }
    let label = match arg {
        None | Some("-") => "<stdin>".to_string(),
        Some(path) => path.to_string(),
    };
    if check_source(check::CheckRequest {
        src: &src,
        label: &label,
        json,
        bare,
        root: root.map(std::path::PathBuf::from),
        no_project,
    }) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
