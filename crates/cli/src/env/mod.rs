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

/// `grade <file...>`: the `--json` batch view for one or more files.
pub fn grade(paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        eprintln!("usage: sokonanoda grade <file...>");
        return ExitCode::FAILURE;
    }
    let mut ok = true;
    for path in paths {
        match std::fs::read_to_string(path) {
            Ok(src) => {
                if !crate::check::check_source(&src, path, true, false) {
                    ok = false;
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
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

/// `gate`: contributor CI gate (cargo required) + the playground anchor.
pub fn gate() -> ExitCode {
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
    for args in STEPS {
        eprintln!("+ cargo {}", args.join(" "));
        match std::process::Command::new("cargo").args(args).status() {
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
    let anchor = "playground.sokonanoda";
    match std::fs::read_to_string(anchor) {
        Ok(src) => {
            if crate::check::check_source(&src, anchor, true, false) {
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
