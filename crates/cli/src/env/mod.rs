// 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
//! Environment subcommands (`sokonanoda version/doctor/setup/update/grade/gate`)
//! — the binary replacement for the old `scripts/soko.sh`. Design:
//! `docs/design/binary-cli.md`.

mod download;
mod target;

use std::process::ExitCode;

use target::{
    binary_path, binary_ready, cache_dir, expected_marker, offline, read_marker, version,
    vsce_target, TARGET,
};

/// The pinned asset set: `(release pkg, cache binary base)`.
const BINARIES: [(&str, &str); 2] = [
    ("sokonanoda-cli", "sokonanoda"),
    ("sokonanoda-lsp", "sokonanoda-lsp"),
];

fn ready_line() -> String {
    format!(
        "sokonanoda: ready — {} (v{} {}, {})",
        binary_path("sokonanoda").display(),
        version(),
        vsce_target(),
        TARGET
    )
}

/// `version [--json]`: read-only repo/target + cached markers.
pub fn version_cmd(json: bool) -> ExitCode {
    let cli_marker = read_marker("sokonanoda");
    let lsp_marker = read_marker("sokonanoda-lsp");
    let cli_present = binary_path("sokonanoda").is_file();
    let lsp_present = binary_path("sokonanoda-lsp").is_file();
    let expected = expected_marker();
    let cli_match = cli_marker.as_deref() == Some(expected.as_str());
    let lsp_match = lsp_marker.as_deref() == Some(expected.as_str());

    if json {
        let value = serde_json::json!({
            "version": version(),
            "target": vsce_target(),
            "rust_target": TARGET,
            "cache": cache_dir().display().to_string(),
            "cli": {
                "present": cli_present,
                "marker": cli_marker.clone().unwrap_or_default(),
                "match": cli_match,
            },
            "lsp": {
                "present": lsp_present,
                "marker": lsp_marker.clone().unwrap_or_default(),
                "match": lsp_match,
            },
        });
        println!(
            "{}",
            serde_json::to_string(&value).expect("serialize version")
        );
    } else {
        let mark = |ok: bool| if ok { "match" } else { "mismatch" };
        println!("sokonanoda version");
        println!("  bin:   v{} ({}, {})", version(), vsce_target(), TARGET);
        println!("  cache: {}", cache_dir().display());
        println!(
            "  cli:   {} ({})",
            cli_marker.as_deref().unwrap_or("<missing>"),
            mark(cli_match)
        );
        println!(
            "  lsp:   {} ({})",
            lsp_marker.as_deref().unwrap_or("<missing>"),
            mark(lsp_match)
        );
    }
    ExitCode::SUCCESS
}

/// `doctor [--json]`: readiness report; exit 3 when not ready.
pub fn doctor(json: bool) -> ExitCode {
    let cli_ready = binary_ready("sokonanoda");
    let lsp_ready = binary_ready("sokonanoda-lsp");
    let ready = cli_ready && lsp_ready;
    let cli_path = binary_path("sokonanoda");
    let lsp_path = binary_path("sokonanoda-lsp");

    if json {
        let value = serde_json::json!({
            "ready": ready,
            "version": version(),
            "target": vsce_target(),
            "rust_target": TARGET,
            "cache": cache_dir().display().to_string(),
            "offline": offline(),
            "cli": {
                "path": cli_path.display().to_string(),
                "present": cli_path.is_file(),
                "version_match": cli_ready,
            },
            "lsp": {
                "path": lsp_path.display().to_string(),
                "present": lsp_path.is_file(),
                "version_match": lsp_ready,
            },
        });
        println!(
            "{}",
            serde_json::to_string(&value).expect("serialize doctor")
        );
    } else {
        println!("sokonanoda doctor");
        println!("  version:  v{}", version());
        println!("  platform: {} (rust: {})", vsce_target(), TARGET);
        println!(
            "  cache:    {}{}",
            cache_dir().display(),
            if offline() { " (offline)" } else { "" }
        );
        println!(
            "  cli:      {} present={} version_match={}",
            cli_path.display(),
            cli_path.is_file(),
            cli_ready
        );
        println!(
            "  lsp:      {} present={} version_match={}",
            lsp_path.display(),
            lsp_path.is_file(),
            lsp_ready
        );
        if ready {
            println!("  status:   READY");
        } else {
            println!("  status:   NOT READY — run: sokonanoda setup");
        }
    }
    if ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    }
}

/// `setup [--force]`: ensure the pinned CLI + LSP are cached.
pub fn setup(force: bool) -> ExitCode {
    let mut failed = false;
    for (pkg, base) in BINARIES {
        if !force && binary_ready(base) {
            continue;
        }
        if offline() {
            eprintln!(
                "sokonanoda: 离线模式且缓存缺失或不匹配 {base}（需要 v{} {}）；先联网跑 `sokonanoda setup`",
                version(),
                TARGET
            );
            failed = true;
            continue;
        }
        if let Err(e) = download::download_binary(pkg, base) {
            eprintln!("sokonanoda: {e}");
            failed = true;
        }
    }
    if failed {
        return ExitCode::from(3);
    }
    println!("{}", ready_line());
    ExitCode::SUCCESS
}

/// `update`: force-refresh the cache to this binary's version.
pub fn update() -> ExitCode {
    eprintln!("sokonanoda: updating cache to v{}…", version());
    setup(true)
}

/// `grade <file...> [--root <dir>]`: the `--json` batch view for one or more files.
///
/// `root` = `--root <dir>`（显式模块根，`docs/design/imports-and-projects.md` §4.9）。
/// ⚠ **2026-10-12 修**：以前这里**只收位置参数** ⇒ `--root` 被 `main` 解析进 `root`
/// 变量却**没传进来** ⇒ `scripts/soko grade --root <根> <入口>`（`AGENTS.md` 写的用法）
/// **静默按入口目录找模块**（`import lib.Set` 报 `import-not-found`，期望路径是入口
/// 目录而不是 `--root`）✗。判据 `crates/cli/tests/imports.rs::grade_subcommand_honors_root`。
pub fn grade(paths: &[String], root: Option<&str>) -> ExitCode {
    if paths.is_empty() {
        eprintln!("usage: sokonanoda grade <file...> [--root <dir>]");
        return ExitCode::FAILURE;
    }
    let root = root.map(std::path::PathBuf::from);
    let mut ok = true;
    for path in paths {
        match std::fs::read_to_string(path) {
            Ok(src) => {
                if !crate::check::check_source(crate::check::CheckRequest {
                    src: &src,
                    label: path,
                    json: true,
                    bare: false,
                    root: root.clone(),
                    no_project: false,
                }) {
                    ok = false;
                }
            }
            Err(e) => {
                // 2026-10-08（用户「cli 很多命令有问题」）：以前是裸 `error: {e}` ✗ ——
                // 不说是**哪个**文件（批量判卷 / 课程门禁里无法定位 ✓）。
                eprintln!("error: 读不到 {path}：{e}");
                ok = false;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The repo's `[workspace.package] version` from `Cargo.toml` in the cwd, when
/// readable (contributors run `gate` from the repo root).
fn repo_workspace_version() -> Option<String> {
    let text = std::fs::read_to_string("Cargo.toml").ok()?;
    let mut in_workspace_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if in_workspace_package {
            if let Some(rest) = trimmed.strip_prefix("version") {
                let value = rest.trim_start_matches([' ', '=']).trim().trim_matches('"');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// `gate`: contributor CI gate (cargo required) + the playground anchor.
///
/// The anchor compiles `playground.sokonanoda` with **this binary's embedded
/// compiler**. If the binary is a stale download-cache build (e.g. an old cache
/// while the repo has moved on), the anchor would silently gate the repo with
/// old logic — a 0.27.0 cache cannot parse newer syntax and the failure looks
/// like a source bug. Refuse to run when the repo's version disagrees.
pub fn gate(args: &[String]) -> ExitCode {
    let _ = args;
    // `--fast`：**迭代用的内环**——只跑 fmt + clippy + anchor（+ 调用方补的
    // "改动过的 crate 的单测"）。`cargo test --workspace` 里最贵的是**课程规模**
    // 的那几个 suite（实测 LSP 库 129s、`judge_batch` 87s），迭代时每轮跑它们
    // 是纯浪费；提交/推送前跑**完整** gate（不带 `--fast`）。
    let fast = args.iter().any(|a| a == "--fast");
    if let Some(repo) = repo_workspace_version() {
        let binary = env!("CARGO_PKG_VERSION");
        if repo != binary {
            eprintln!(
                "sokonanoda: gate 用的是 v{binary} 二进制内嵌的编译逻辑，而仓库是 v{repo} \
                 —— anchor 结果不可信。请 `sokonanoda update`，或改跑 \
                 `cargo run -q -p sokonanoda-cli --bin sokonanoda -- playground.sokonanoda`。"
            );
            return ExitCode::from(3);
        }
    }
    const STEPS: [&[&str]; 3] = [
        &[
            "fmt",
            "-p",
            "sokonanoda-front",
            "-p",
            "sokonanoda-cli",
            "-p",
            "sokonanoda-lsp",
            "--",
            "--check",
        ],
        &["clippy", "--workspace", "--all-targets"],
        &["test", "--workspace", "--locked"],
    ];
    for args in STEPS.iter().take(if fast { 2 } else { 3 }) {
        eprintln!("+ cargo {}", args.join(" "));
        match std::process::Command::new("cargo").args(*args).status() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!("sokonanoda: gate failed (cargo exit {status})");
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!("sokonanoda: 贡献者门禁需要 Rust/cargo: {e}");
                return ExitCode::from(3);
            }
        }
    }
    if fast {
        eprintln!("+ （--fast：跳过 `cargo test --workspace`——调用方会补改动过的 crate 的单测）");
    }
    let anchor = "playground.sokonanoda";
    match std::fs::read_to_string(anchor) {
        Ok(src) => {
            if crate::check::check_source(crate::check::CheckRequest {
                src: &src,
                label: anchor,
                json: true,
                bare: false,
                root: None,
                no_project: false,
            }) {
                eprintln!("sokonanoda: gate PASS");
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("sokonanoda: 读不到 {anchor}: {e}");
            ExitCode::FAILURE
        }
    }
}
