//! opencode project-config + onboarding contract: the single environment
//! entrypoint is `scripts/soko.sh` (docs/design-onboarding.md); the opencode
//! layer must stay a thin, namespaced, cargo-free wrapper around it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

use common::repo_root;

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
    fs::write(path, body).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod script");
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

/// Copy `scripts/soko.sh` + the LSP shim + a fake `Cargo.toml` into `$tmp` so
/// the real workspace `target/` builds cannot shadow the lookup under test.
#[cfg(unix)]
fn copy_script_env(tmp: &Path, version: &str) {
    write_executable(
        &tmp.join("scripts/soko.sh"),
        &fs::read_to_string(repo_root().join("scripts/soko.sh")).expect("read soko.sh"),
    );
    write_executable(
        &tmp.join(".opencode/lsp/sokonanoda-lsp.sh"),
        &fs::read_to_string(repo_root().join(".opencode/lsp/sokonanoda-lsp.sh"))
            .expect("read launcher"),
    );
    fs::write(tmp.join("Cargo.toml"), format!("version = \"{version}\"\n")).expect("Cargo.toml");
}

/// Regression guard: the project `opencode.json` must keep wiring the
/// sokonanoda LSP (the block was accidentally deleted once, which made
/// opencode log "all LSPs are disabled").
#[test]
fn opencode_json_wires_the_sokonanoda_lsp() {
    let root = repo_root();
    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("opencode.json")).expect("opencode.json"),
    )
    .expect("opencode.json is valid JSON");
    let lsp = &config["lsp"]["sokonanoda"];
    assert!(
        lsp.is_object(),
        "opencode.json must configure lsp.sokonanoda"
    );
    let command = lsp["command"].as_array().expect("command array");
    assert!(
        command.iter().any(|v| v
            .as_str()
            .is_some_and(|s| s.contains(".opencode/lsp/sokonanoda-lsp.sh"))),
        "lsp.sokonanoda.command must run the repo launcher shim: {command:?}"
    );
    assert!(
        lsp["extensions"]
            .as_array()
            .expect("extensions array")
            .iter()
            .any(|v| v.as_str() == Some(".sokonanoda")),
        "lsp.sokonanoda.extensions must include .sokonanoda"
    );
    assert!(
        config["skills"]["paths"].is_array(),
        "skills.paths must stay wired"
    );
}

#[test]
fn opencode_layer_is_namespaced_thin_and_cargo_free() {
    let root = repo_root();

    // Commands live under the `sokonanoda/` namespace (invoked /sokonanoda/…);
    // the old flat names must be gone.
    for name in ["setup", "doctor", "check", "gate", "round"] {
        let path = root.join(format!(".opencode/command/sokonanoda/{name}.md"));
        assert!(path.exists(), "missing namespaced command {name}");
        let body = fs::read_to_string(&path).expect("command readable");
        assert!(
            !body.contains("cargo run") && !body.contains("cargo build"),
            "{name} must stay cargo-free (user/agent path)"
        );
    }
    for dead in ["check.md", "setup.md", "gate.md", "round.md"] {
        assert!(
            !root.join(".opencode/command").join(dead).exists(),
            "flat command {dead} must be removed (namespaced now)"
        );
    }
    for name in ["setup", "doctor", "check"] {
        let body = fs::read_to_string(root.join(format!(".opencode/command/sokonanoda/{name}.md")))
            .expect("command readable");
        assert!(
            body.contains("scripts/soko.sh"),
            "{name} must call the single entrypoint scripts/soko.sh"
        );
        // opencode may be opened in a repo subdirectory: commands must resolve
        // the repo root instead of assuming cwd.
        assert!(
            body.contains("git rev-parse --show-toplevel"),
            "{name} must resolve the repo root (cwd-independent)"
        );
    }
    let gate_body = fs::read_to_string(root.join(".opencode/command/sokonanoda/gate.md"))
        .expect("gate readable");
    assert!(
        gate_body.contains("git rev-parse --show-toplevel") && gate_body.contains("soko.sh"),
        "gate must also be cwd-independent"
    );
    // The LSP launcher is only a shim over `soko.sh lsp`.
    let launcher = fs::read_to_string(root.join(".opencode/lsp/sokonanoda-lsp.sh"))
        .expect("launcher readable");
    assert!(
        launcher.contains("scripts/soko.sh") && launcher.contains("lsp"),
        "launcher must delegate to scripts/soko.sh lsp"
    );

    // The startup plugin provisions binaries and injects the cache into PATH.
    let plugin =
        fs::read_to_string(root.join(".opencode/plugin/sokonanoda.ts")).expect("plugin readable");
    assert!(
        plugin.contains("scripts/soko.sh")
            && plugin.contains("shell.env")
            && plugin.contains("findRepoRoot"),
        "plugin must find the repo root, run the setup script, and inject PATH via shell.env"
    );

    // The entrypoint itself must exist and be runnable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(root.join("scripts/soko.sh"))
            .expect("stat soko.sh")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "scripts/soko.sh must be executable");
    }
}

/// `doctor --json` is the machine-readable readiness contract: exit 3 when the
/// cache is empty, exit 0 once both binaries with matching version markers are
/// present.
#[cfg(unix)]
#[test]
fn soko_doctor_reports_readiness_with_exit_codes() {
    let root = repo_root();
    let tmp = unique_tmp("doctor");
    let cache = tmp.join("cache");
    fs::create_dir_all(&cache).expect("create cache");

    let run = || {
        Command::new("bash")
            .arg(root.join("scripts/soko.sh"))
            .arg("doctor")
            .arg("--json")
            .env("SOKONANODA_CACHE_DIR", &cache)
            .env("SOKONANODA_OFFLINE", "1")
            .output()
            .expect("run doctor")
    };

    let output = run();
    assert_eq!(
        output.status.code(),
        Some(3),
        "empty cache must be NOT READY (exit 3)"
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor emits JSON");
    assert_eq!(json["ready"], false);
    let version = json["version"].as_str().expect("version").to_string();
    let target = json["target"].as_str().expect("target").to_string();

    // Fake binaries + version markers make the environment ready (offline).
    write_executable(&cache.join("sokonanoda"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(
        &cache.join("sokonanoda-lsp"),
        "#!/usr/bin/env bash\nexit 0\n",
    );
    fs::write(
        cache.join("sokonanoda.version"),
        format!("{version} {target}\n"),
    )
    .expect("cli marker");
    fs::write(
        cache.join("sokonanoda-lsp.version"),
        format!("{version} {target}\n"),
    )
    .expect("lsp marker");

    let output = run();
    assert_eq!(
        output.status.code(),
        Some(0),
        "matching markers must be READY (exit 0): {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor emits JSON");
    assert_eq!(json["ready"], true);
    assert_eq!(json["cli"]["version_match"], true);
    assert_eq!(json["lsp"]["version_match"], true);

    fs::remove_dir_all(&tmp).ok();
}

/// `setup` in offline mode must fail with exit 3 and an actionable message
/// instead of hanging or silently doing nothing.
#[cfg(unix)]
#[test]
fn soko_setup_offline_is_actionable() {
    let root = repo_root();
    let tmp = unique_tmp("setup-offline");
    let cache = tmp.join("cache");
    fs::create_dir_all(&cache).expect("create cache");

    let output = Command::new("bash")
        .arg(root.join("scripts/soko.sh"))
        .arg("setup")
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_OFFLINE", "1")
        .output()
        .expect("run setup");
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("离线模式"),
        "offline failure must be actionable: {stderr}"
    );
    fs::remove_dir_all(&tmp).ok();
}

/// `grade` must exec the cached CLI with `--json` and the given files.
#[cfg(unix)]
#[test]
fn soko_grade_runs_the_cached_cli() {
    let root = repo_root();
    let tmp = unique_tmp("grade");
    let cache = tmp.join("cache");
    write_executable(
        &cache.join("sokonanoda"),
        "#!/usr/bin/env bash\necho \"ARGS:$*\"\n",
    );
    // Marker must match the repo version + host target.
    let doctor = Command::new("bash")
        .arg(root.join("scripts/soko.sh"))
        .arg("doctor")
        .arg("--json")
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_OFFLINE", "1")
        .output()
        .expect("run doctor");
    let json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor emits JSON");
    let version = json["version"].as_str().expect("version").to_string();
    let target = json["target"].as_str().expect("target").to_string();
    fs::write(
        cache.join("sokonanoda.version"),
        format!("{version} {target}\n"),
    )
    .expect("cli marker");

    let output = Command::new("bash")
        .arg(root.join("scripts/soko.sh"))
        .arg("grade")
        .arg("playground.sokonanoda")
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_OFFLINE", "1")
        .output()
        .expect("run grade");
    assert!(output.status.success(), "grade must succeed: {output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "ARGS:--json playground.sokonanoda"
    );
    fs::remove_dir_all(&tmp).ok();
}

/// The editor resolution chain must find the server bundled by an installed
/// VS Code extension without `cargo` (zero-network reuse).
#[cfg(unix)]
#[test]
fn launcher_reuses_the_vscode_extension_binary_without_cargo() {
    let tmp = unique_tmp("ext");
    copy_script_env(&tmp, "9.9.9");
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
        write_executable(
            &home
                .join(".vscode/extensions")
                .join("sokonanoda-lang.sokonanoda-9.9.9/bin")
                .join(target)
                .join("sokonanoda-lsp"),
            "#!/usr/bin/env bash\necho FAKE_EXTENSION_BIN\n",
        );
    }

    let output = Command::new("bash")
        .arg(tmp.join(".opencode/lsp/sokonanoda-lsp.sh"))
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin") // deliberately no ~/.cargo/bin
        .env_remove("SOKONANODA_LSP_BIN")
        .env_remove("SOKONANODA_OFFLINE")
        .output()
        .expect("run launcher");
    assert!(
        output.status.success(),
        "launcher must succeed with the extension binary: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "FAKE_EXTENSION_BIN"
    );
    fs::remove_dir_all(&tmp).ok();
}

/// Headless / no-VS-Code machinery: the launcher downloads the version-pinned
/// release tarball (checked via a fake `curl`) and runs the extracted binary.
#[cfg(unix)]
#[test]
fn launcher_downloads_the_version_pinned_release_without_vscode_or_cargo() {
    let tmp = unique_tmp("dl");
    copy_script_env(&tmp, "9.9.9");

    let payload_dir = tmp.join("payload");
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

    let fake_bin = tmp.join("fake-bin");
    write_executable(
        &fake_bin.join("curl"),
        "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" >> \"$FAKE_CURL_LOG\"\ncat \"$FAKE_TARBALL\"\n",
    );
    let curl_log = tmp.join("curl.log");
    let home = tmp.join("home");

    let output = Command::new("bash")
        .arg(tmp.join(".opencode/lsp/sokonanoda-lsp.sh"))
        .env("HOME", &home)
        .env("SOKONANODA_CACHE_DIR", home.join("cache"))
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("FAKE_CURL_LOG", &curl_log)
        .env("FAKE_TARBALL", &tarball)
        .env_remove("SOKONANODA_LSP_BIN")
        .env_remove("SOKONANODA_OFFLINE")
        .output()
        .expect("run launcher");
    assert!(
        output.status.success(),
        "launcher must succeed after download: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "FAKE_DOWNLOAD"
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
/// actionable (exit 3) instead of silent.
#[cfg(unix)]
#[test]
fn launcher_failure_is_actionable_when_nothing_is_available() {
    let tmp = unique_tmp("empty");
    copy_script_env(&tmp, "9.9.9");
    let home = tmp.join("home");
    fs::create_dir_all(&home).expect("create home");

    let output = Command::new("bash")
        .arg(tmp.join(".opencode/lsp/sokonanoda-lsp.sh"))
        .env("HOME", &home)
        .env("SOKONANODA_CACHE_DIR", home.join("cache"))
        .env("PATH", "/usr/bin:/bin") // no cargo; offline disables the download
        .env("SOKONANODA_OFFLINE", "1")
        .env_remove("SOKONANODA_LSP_BIN")
        .output()
        .expect("run launcher");
    assert_eq!(
        output.status.code(),
        Some(3),
        "environment failure must use exit code 3"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("找不到语言服务器二进制"),
        "failure must be actionable: {stderr}"
    );
    fs::remove_dir_all(&tmp).ok();
}
