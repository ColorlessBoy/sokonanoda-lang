//! opencode project-config contract (`opencode.json` + `.opencode/lsp/`): the
//! agent editor must start the LSP through the repo-local launcher. Code
//! agents are decoupled from the VS Code extension, so the launcher resolves
//! (or downloads, version-pinned) the server on its own — no `cargo` and no
//! VS Code install required.

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

fn unique_tmp(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sokonanoda-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, body).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod script");
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
    // The command should stay a one-liner: `bash -c "exec <root>/launcher"`.
    assert!(
        command.len() == 3 && command[0] == "bash" && command[1] == "-c",
        "keep the opencode LSP command minimal (bash -c exec …): {command:?}"
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
        "launcher must look inside installed VS Code extensions (zero-network reuse)"
    );
    assert!(
        body.contains("releases/download/v${version}/"),
        "launcher must download version-pinned release assets"
    );
    assert!(
        !body.contains("/latest/"),
        "launcher must never download from releases/latest"
    );
    assert!(
        body.contains("SOKONANODA_LSP_OFFLINE"),
        "launcher must expose an offline escape hatch"
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

/// Copy the launcher into `$tmp` so the real workspace `target/` builds cannot
/// shadow the lookup under test; returns the copied script path.
#[cfg(unix)]
fn copied_launcher(tmp: &Path) -> PathBuf {
    let lsp_dir = tmp.join(".opencode/lsp");
    fs::create_dir_all(&lsp_dir).expect("create temp launcher dir");
    let script = lsp_dir.join("sokonanoda-lsp.sh");
    fs::copy(repo_root().join(".opencode/lsp/sokonanoda-lsp.sh"), &script).expect("copy launcher");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod launcher");
    script
}

/// The launcher must pick up the server bundled in an installed VS Code
/// extension without needing `cargo` (zero-network reuse for editor users).
#[cfg(unix)]
#[test]
fn launcher_reuses_the_vscode_extension_binary_without_cargo() {
    let tmp = unique_tmp("ext");
    let script = copied_launcher(&tmp);
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
        write_executable(
            &bin_dir.join("sokonanoda-lsp"),
            "#!/usr/bin/env bash\necho FAKE_EXTENSION_BIN\n",
        );
    }

    let output = Command::new("bash")
        .arg(&script)
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin") // deliberately no ~/.cargo/bin
        .env_remove("SOKONANODA_LSP_BIN")
        .env_remove("SOKONANODA_LSP_OFFLINE")
        .output()
        .expect("run launcher");
    assert!(
        output.status.success(),
        "launcher must succeed with the extension binary: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "FAKE_EXTENSION_BIN",
        "launcher must exec the installed extension's bundled binary"
    );
    fs::remove_dir_all(&tmp).ok();
}

/// Headless / no-VS-Code machinery: the launcher downloads the version-pinned
/// release tarball (checked via a fake `curl`) and runs the extracted binary.
#[cfg(unix)]
#[test]
fn launcher_downloads_the_version_pinned_release_without_vscode_or_cargo() {
    let tmp = unique_tmp("dl");
    let script = copied_launcher(&tmp);
    fs::write(tmp.join("Cargo.toml"), "version = \"9.9.9\"\n").expect("write Cargo.toml");

    // Prepared release asset: a tarball with `sokonanoda-lsp` at its root.
    let payload_dir = tmp.join("payload");
    fs::create_dir_all(&payload_dir).expect("create payload dir");
    write_executable(
        &payload_dir.join("sokonanoda-lsp"),
        "#!/usr/bin/env bash\necho FAKE_DOWNLOAD\n",
    );
    let tarball = tmp.join("asset.tar.gz");
    let tar = Command::new("tar")
        .args(["czf"])
        .arg(&tarball)
        .args(["-C"])
        .arg(&payload_dir)
        .arg("sokonanoda-lsp")
        .output()
        .expect("run tar");
    assert!(tar.status.success(), "tar failed: {tar:?}");

    // Fake curl: records its arguments, streams the prepared tarball.
    let fake_bin = tmp.join("fake-bin");
    fs::create_dir_all(&fake_bin).expect("create fake bin");
    write_executable(
        &fake_bin.join("curl"),
        "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" >> \"$FAKE_CURL_LOG\"\ncat \"$FAKE_TARBALL\"\n",
    );
    let curl_log = tmp.join("curl.log");
    let home = tmp.join("home");

    let output = Command::new("bash")
        .arg(&script)
        .env("HOME", &home)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("FAKE_CURL_LOG", &curl_log)
        .env("FAKE_TARBALL", &tarball)
        .env_remove("SOKONANODA_LSP_BIN")
        .env_remove("SOKONANODA_LSP_OFFLINE")
        .output()
        .expect("run launcher");
    assert!(
        output.status.success(),
        "launcher must succeed after download: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "FAKE_DOWNLOAD",
        "launcher must exec the downloaded binary"
    );

    let log = fs::read_to_string(&curl_log).expect("fake curl log");
    assert!(
        log.contains("releases/download/v9.9.9/sokonanoda-lsp-"),
        "download must be pinned to the repo version: {log}"
    );
    assert!(
        !log.contains("/latest/"),
        "download must never use the latest alias: {log}"
    );
    fs::remove_dir_all(&tmp).ok();
}

/// With no extension, no cache, no network and no cargo, the failure must be
/// actionable instead of silent.
#[cfg(unix)]
#[test]
fn launcher_failure_is_actionable_when_nothing_is_available() {
    let tmp = unique_tmp("empty");
    let script = copied_launcher(&tmp);
    fs::write(tmp.join("Cargo.toml"), "version = \"9.9.9\"\n").expect("write Cargo.toml");
    let home = tmp.join("home");
    fs::create_dir_all(&home).expect("create empty home");

    let output = Command::new("bash")
        .arg(&script)
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin") // no cargo; offline disables the download
        .env("SOKONANODA_LSP_OFFLINE", "1")
        .env_remove("SOKONANODA_LSP_BIN")
        .output()
        .expect("run launcher");
    assert!(
        !output.status.success(),
        "launcher must fail when nothing is available"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("找不到语言服务器二进制"),
        "failure must be actionable: {stderr}"
    );
    fs::remove_dir_all(&tmp).ok();
}
