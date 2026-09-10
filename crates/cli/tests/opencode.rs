//! opencode project-config contract (`opencode.json` + `.opencode/lsp/`): the
//! agent editor must start the LSP through the repo-local launcher, which
//! keeps working when `cargo` is not on opencode's PATH, avoids a first-run
//! build stalling the LSP handshake (spawn failures disable the server for the
//! session), and prefers the binary already shipped by the VS Code extension.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

use common::repo_root;

fn command_text(config: &serde_json::Value) -> Vec<String> {
    config["lsp"]["sokonanoda"]["command"]
        .as_array()
        .expect("lsp.sokonanoda.command is an array")
        .iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

#[test]
fn opencode_lsp_uses_the_repo_local_launcher() {
    let root = repo_root();
    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("opencode.json")).expect("opencode.json"),
    )
    .expect("opencode.json is valid JSON");
    let command = command_text(&config);
    assert!(
        command
            .iter()
            .any(|part| part.contains(".opencode/lsp/sokonanoda-lsp.sh")),
        "opencode.json must launch the repo-local LSP script: {command:?}"
    );
    // Regression guard: the old `cargo run` form breaks for GUI-launched
    // opencode (no cargo on PATH) and on first-run builds.
    assert!(
        !command.iter().any(|part| part == "cargo"),
        "the LSP command must not invoke cargo directly: {command:?}"
    );

    let script = root.join(".opencode/lsp/sokonanoda-lsp.sh");
    assert!(script.exists(), "launcher script exists");
    let body = fs::read_to_string(&script).expect("launcher readable");
    assert!(
        body.contains("target/release") && body.contains("target/debug"),
        "launcher must reuse existing release/debug builds before compiling"
    );
    assert!(
        body.contains("sokonanoda-lang.sokonanoda-"),
        "launcher must look inside installed VS Code extensions"
    );
    assert!(
        body.contains("cargo build"),
        "launcher must be able to build on a fresh clone"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&script)
            .expect("stat launcher")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "launcher must be executable (mode {mode:o})"
        );
    }
}

/// The launcher must pick up the server bundled in an installed VS Code
/// extension without needing `cargo` (the scenario reported by the user).
#[cfg(unix)]
#[test]
fn launcher_falls_back_to_the_vscode_extension_binary_without_cargo() {
    use std::os::unix::fs::PermissionsExt;

    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!(
        "sokonanoda-launcher-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    // A copied launcher sees `$tmp` as the repo root, so the real workspace
    // `target/` builds cannot shadow the extension lookup under test.
    let lsp_dir = tmp.join(".opencode/lsp");
    fs::create_dir_all(&lsp_dir).expect("create temp launcher dir");
    let script = lsp_dir.join("sokonanoda-lsp.sh");
    fs::copy(root.join(".opencode/lsp/sokonanoda-lsp.sh"), &script).expect("copy launcher");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod launcher");

    // Fake extension install: sokonanoda-lang.sokonanoda-9.9.9 with a binary
    // for every target (the launcher picks its host target).
    let home = tmp.join("home");
    for target in [
        "darwin-arm64",
        "darwin-x64",
        "linux-x64",
        "linux-arm64",
        "alpine-x64",
        "alpine-arm64",
        "win32-x64",
        "win32-arm64",
    ] {
        let bin_dir = home
            .join(".vscode/extensions")
            .join("sokonanoda-lang.sokonanoda-9.9.9/bin")
            .join(target);
        fs::create_dir_all(&bin_dir).expect("create fake extension bin dir");
        let fake = bin_dir.join("sokonanoda-lsp");
        fs::write(&fake, "#!/usr/bin/env bash\necho FAKE_EXTENSION_BIN\n").expect("write fake bin");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("chmod fake bin");
    }

    let run = |home: &Path| {
        Command::new("bash")
            .arg(&script)
            .env("HOME", home)
            .env("PATH", "/usr/bin:/bin") // deliberately no ~/.cargo/bin
            .env_remove("SOKONANODA_LSP_BIN")
            .output()
            .expect("run launcher")
    };

    let output = run(&home);
    assert!(
        output.status.success(),
        "launcher must succeed with the extension binary: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "FAKE_EXTENSION_BIN",
        "launcher must exec the VS Code extension's bundled binary first"
    );

    // No extension and no cargo on PATH → a clear, actionable failure.
    let empty_home = tmp.join("empty-home");
    fs::create_dir_all(&empty_home).expect("create empty home");
    let output = run(&empty_home);
    assert!(
        !output.status.success(),
        "launcher must fail when no binary is available at all"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("找不到语言服务器二进制"),
        "failure must be actionable: {stderr}"
    );

    fs::remove_dir_all(&tmp).ok();
}

/// Keep the compile-time import honest on non-unix targets.
#[allow(dead_code)]
fn _path_buf_marker(_: PathBuf) {}
