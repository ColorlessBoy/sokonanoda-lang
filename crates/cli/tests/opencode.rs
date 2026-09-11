//! Environment subcommands + opencode project-config contract: the environment
//! entrypoint is the `sokonanoda` binary itself (`version`/`doctor`/`setup`/
//! `update`/`lsp`/`gate`, docs/design/binary-cli.md). The opencode layer must
//! stay a thin, namespaced, cargo-free wrapper around it.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

mod common;

use common::repo_root;

/// The CLI binary under test (Cargo provides its path to integration tests).
const SOKO: &str = env!("CARGO_BIN_EXE_sokonanoda");

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

/// `sokonanoda <args>` with an explicit cache dir.
fn run(cache: &Path, args: &[&str]) -> Output {
    Command::new(SOKO)
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache)
        .output()
        .expect("run sokonanoda")
}

fn version_target(cache: &Path) -> (String, String) {
    let out = run(cache, &["version", "--json"]);
    assert!(out.status.success(), "version must succeed");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("version emits JSON");
    (
        json["version"].as_str().expect("version").to_string(),
        json["target"].as_str().expect("target").to_string(),
    )
}

/// A minimal one-shot-at-a-time HTTP server that serves `bytes` for every
/// request and records the request lines. Runs detached (thread leaks at test
/// exit, which is fine).
fn serve_bytes(bytes: Vec<u8>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let log = Arc::new(Mutex::new(Vec::new()));
    let log2 = Arc::clone(&log);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n])
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            log2.lock().expect("log lock").push(request);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/gzip\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&bytes);
            let _ = stream.flush();
        }
    });
    (base, log)
}

/// Regression guard for the base wiring: the plugin itself must provide the
/// LSP (native binary, cross-platform, no bash) and version-check the cache.
#[test]
fn opencode_lsp_is_wired_via_the_plugin() {
    let root = repo_root();
    let plugin =
        fs::read_to_string(root.join(".opencode/plugin/sokonanoda.ts")).expect("plugin readable");
    for needle in [
        "config:",
        "cfg.lsp.sokonanoda",
        "sokonanoda-lsp",
        "extensions: [\".sokonanoda\"]",
        "fetch(",
        "markerMatches",
        "findRepoRoot",
        "shell.env",
    ] {
        assert!(plugin.contains(needle), "plugin must contain `{needle}`");
    }
    assert!(
        !plugin.contains("\"bash\""),
        "the plugin must not need bash (it runs on opencode's bundled runtime)"
    );

    // The shim (non-opencode harnesses) resolves the binary and runs `lsp`.
    let launcher = fs::read_to_string(root.join(".opencode/lsp/sokonanoda-lsp.sh"))
        .expect("launcher readable");
    assert!(
        launcher.contains("sokonanoda") && launcher.contains("lsp"),
        "the fallback launcher must resolve the binary and run `sokonanoda lsp`"
    );

    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("opencode.json")).expect("opencode.json"),
    )
    .expect("opencode.json is valid JSON");
    assert!(
        config["skills"]["paths"].is_array(),
        "skills.paths must stay wired"
    );
    assert!(
        config["lsp"].is_null(),
        "opencode.json must not hardcode an lsp block (the plugin wires the binary)"
    );
}

#[test]
fn opencode_layer_is_namespaced_thin_and_cargo_free() {
    let root = repo_root();

    assert!(
        !root.join("scripts/soko.sh").exists(),
        "scripts/soko.sh must be gone (the binary is the entrypoint)"
    );

    // Commands live under the `sokonanoda/` namespace and call the binary.
    for name in [
        "setup", "update", "version", "doctor", "check", "gate", "round",
    ] {
        let path = root.join(format!(".opencode/command/sokonanoda/{name}.md"));
        assert!(path.exists(), "missing namespaced command {name}");
        let body = fs::read_to_string(&path).expect("command readable");
        assert!(
            !body.contains("cargo run") && !body.contains("cargo build"),
            "{name} must stay cargo-free (user/agent path)"
        );
        assert!(
            !body.contains("soko.sh"),
            "{name} must not reference the removed shell script"
        );
    }
    for dead in ["check.md", "setup.md", "gate.md", "round.md"] {
        assert!(
            !root.join(".opencode/command").join(dead).exists(),
            "flat command {dead} must be removed (namespaced now)"
        );
    }
    for name in ["setup", "update", "version", "doctor", "check"] {
        let body = fs::read_to_string(root.join(format!(".opencode/command/sokonanoda/{name}.md")))
            .expect("command readable");
        assert!(
            body.contains("sokonanoda"),
            "{name} must call the sokonanoda binary"
        );
    }
    // The grading command resolves the repo root (cwd-independent file path).
    let check_body = fs::read_to_string(root.join(".opencode/command/sokonanoda/check.md"))
        .expect("check readable");
    assert!(
        check_body.contains("git rev-parse --show-toplevel"),
        "check must resolve the repo root (cwd-independent)"
    );

    // The startup plugin provisions binaries, version-checks the cache, and
    // injects the cache into PATH.
    let plugin =
        fs::read_to_string(root.join(".opencode/plugin/sokonanoda.ts")).expect("plugin readable");
    assert!(
        plugin.contains("shell.env")
            && plugin.contains("findRepoRoot")
            && plugin.contains("markerMatches"),
        "plugin must find the repo root, version-check the cache, and inject PATH"
    );
}

/// `version --json` is the read-only status contract: it reports the binary
/// version + target and the cached binaries' markers, flagging stale caches.
#[cfg(unix)]
#[test]
fn version_reports_repo_and_cached_markers() {
    let tmp = unique_tmp("version");
    let cache = tmp.join("cache");
    fs::create_dir_all(&cache).expect("create cache");

    let out = run(&cache, &["version", "--json"]);
    assert!(out.status.success(), "version must always succeed");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("version JSON");
    assert_eq!(json["cli"]["match"], false, "empty cache cannot match");
    assert_eq!(json["lsp"]["present"], false);
    let (version, target) = version_target(&cache);

    // A present-but-stale binary must be flagged, not reported as ready.
    write_executable(&cache.join("sokonanoda"), "#!/usr/bin/env bash\nexit 0\n");
    fs::write(cache.join("sokonanoda.version"), "0.0.1 nowhere\n").expect("stale marker");
    let out = run(&cache, &["version", "--json"]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("version JSON");
    assert_eq!(json["cli"]["present"], true);
    assert_eq!(json["cli"]["marker"], "0.0.1 nowhere");
    assert_eq!(json["cli"]["match"], false, "stale marker must not match");

    fs::write(
        cache.join("sokonanoda.version"),
        format!("{version} {target}\n"),
    )
    .expect("fresh marker");
    let out = run(&cache, &["version", "--json"]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("version JSON");
    assert_eq!(json["cli"]["match"], true);

    fs::remove_dir_all(&tmp).ok();
}

/// `doctor --json`: exit 3 when the cache is empty, exit 0 once both binaries
/// carry matching version markers.
#[cfg(unix)]
#[test]
fn doctor_reports_readiness_with_exit_codes() {
    let tmp = unique_tmp("doctor");
    let cache = tmp.join("cache");
    fs::create_dir_all(&cache).expect("create cache");

    let out = run(&cache, &["doctor", "--json"]);
    assert_eq!(out.status.code(), Some(3), "empty cache must be NOT READY");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("doctor JSON");
    assert_eq!(json["ready"], false);
    let (version, target) = version_target(&cache);

    write_executable(&cache.join("sokonanoda"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(
        &cache.join("sokonanoda-lsp"),
        "#!/usr/bin/env bash\nexit 0\n",
    );
    for base in ["sokonanoda", "sokonanoda-lsp"] {
        fs::write(
            cache.join(format!("{base}.version")),
            format!("{version} {target}\n"),
        )
        .expect("marker");
    }
    let out = run(&cache, &["doctor", "--json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "matching markers must be READY: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("doctor JSON");
    assert_eq!(json["ready"], true);
    assert_eq!(json["cli"]["version_match"], true);
    assert_eq!(json["lsp"]["version_match"], true);

    fs::remove_dir_all(&tmp).ok();
}

/// `setup` offline must fail with exit 3 and an actionable message.
#[cfg(unix)]
#[test]
fn setup_offline_is_actionable() {
    let tmp = unique_tmp("setup-offline");
    let cache = tmp.join("cache");
    fs::create_dir_all(&cache).expect("create cache");

    let out = Command::new(SOKO)
        .arg("setup")
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_OFFLINE", "1")
        .output()
        .expect("run setup");
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("离线模式"),
        "offline failure must be actionable: {stderr}"
    );
    fs::remove_dir_all(&tmp).ok();
}

/// `update` re-fetches the version-pinned tarballs through the embedded
/// downloader (local HTTP server), even when the cache already matches.
#[cfg(unix)]
#[test]
fn update_downloads_version_pinned_assets() {
    let tmp = unique_tmp("update");
    let cache = tmp.join("cache");
    fs::create_dir_all(&cache).expect("cache");

    // Payload tarball carrying both binaries (the downloader extracts only the
    // expected name, so one archive serves both requests).
    let payload = tmp.join("payload");
    write_executable(
        &payload.join("sokonanoda"),
        "#!/usr/bin/env bash\necho CLI\n",
    );
    write_executable(
        &payload.join("sokonanoda-lsp"),
        "#!/usr/bin/env bash\necho LSP\n",
    );
    let tarball = tmp.join("asset.tar.gz");
    let tar = Command::new("tar")
        .args(["czf"])
        .arg(&tarball)
        .args(["-C"])
        .arg(&payload)
        .args(["sokonanoda", "sokonanoda-lsp"])
        .output()
        .expect("run tar");
    assert!(tar.status.success(), "tar failed: {tar:?}");
    let (base, log) = serve_bytes(fs::read(&tarball).expect("read tarball"));

    // A cache that already matches the version: `setup` would skip it.
    write_executable(&cache.join("sokonanoda"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(
        &cache.join("sokonanoda-lsp"),
        "#!/usr/bin/env bash\nexit 0\n",
    );
    let (version, target) = version_target(&cache);
    for base_name in ["sokonanoda", "sokonanoda-lsp"] {
        fs::write(
            cache.join(format!("{base_name}.version")),
            format!("{version} {target}\n"),
        )
        .expect("marker");
    }

    let out = Command::new(SOKO)
        .arg("update")
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_RELEASE_BASE", &base)
        .output()
        .expect("run update");
    assert!(
        out.status.success(),
        "update must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let requests = log.lock().expect("log lock").join("\n");
    assert!(
        requests.contains("/sokonanoda-cli-") && requests.contains("/sokonanoda-lsp-"),
        "update must fetch both pinned tarballs (CLI + LSP): {requests}"
    );
    assert!(
        requests.contains(&format!("/v{version}/")),
        "download URL must be pinned to v{version}: {requests}"
    );
    assert!(
        !requests.contains("/latest/"),
        "download must never use the latest alias: {requests}"
    );
    // The fake payload replaced the pre-existing cache contents.
    let cli = fs::read_to_string(cache.join("sokonanoda")).expect("cli readable");
    assert!(cli.contains("echo CLI"), "CLI must be refreshed: {cli}");

    fs::remove_dir_all(&tmp).ok();
}

/// The non-opencode shim resolves `SOKONANODA_BIN` and execs `sokonanoda lsp`.
#[cfg(unix)]
#[test]
fn launcher_resolves_the_binary_and_runs_lsp() {
    let tmp = unique_tmp("shim");
    fs::create_dir_all(&tmp).expect("tmp");
    let fake = tmp.join("fake-sokonanoda");
    write_executable(&fake, "#!/usr/bin/env bash\necho ARGS:$*\n");

    let out = Command::new("bash")
        .arg(repo_root().join(".opencode/lsp/sokonanoda-lsp.sh"))
        .env("SOKONANODA_BIN", &fake)
        .output()
        .expect("run shim");
    assert!(
        out.status.success(),
        "shim must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ARGS:lsp");
    fs::remove_dir_all(&tmp).ok();
}

/// With no binary anywhere, the shim's failure must be actionable (exit 3).
#[cfg(unix)]
#[test]
fn launcher_failure_is_actionable_when_nothing_is_available() {
    let tmp = unique_tmp("empty");
    let shim_dir = tmp.join(".opencode/lsp");
    fs::create_dir_all(&shim_dir).expect("shim dir");
    let shim = shim_dir.join("sokonanoda-lsp.sh");
    fs::copy(repo_root().join(".opencode/lsp/sokonanoda-lsp.sh"), &shim).expect("copy shim");

    let out = Command::new("bash")
        .arg(&shim)
        .env("PATH", "/usr/bin:/bin") // no sokonanoda on PATH
        .env_remove("SOKONANODA_BIN")
        .output()
        .expect("run shim");
    assert_eq!(
        out.status.code(),
        Some(3),
        "environment failure must use exit code 3"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("找不到 sokonanoda 二进制"),
        "failure must be actionable: {stderr}"
    );
    fs::remove_dir_all(&tmp).ok();
}
