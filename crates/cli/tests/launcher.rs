//! Behavioral contract for `scripts/soko` — the harness-neutral launcher.
//!
//! `dsh.rs` asserts the launcher's *shape* by reading it; this file **runs**
//! it, because the property that matters here is behavioral:
//!
//! > a forced refresh that did not happen must never look like success.
//!
//! The setup is hermetic — no network, no real cache, no reliance on what
//! happens to sit in `target/`:
//!
//! * `SOKONANODA_CACHE_DIR` → a fresh empty directory (nothing to reuse);
//! * `SOKONANODA_OFFLINE=1` → the download step cannot succeed;
//! * `SOKONANODA_BIN` / `SOKONANODA_LSP_BIN` → binaries that exist, i.e. a
//!   *usable* fallback. That is exactly the shape that used to swallow the
//!   failure: `update` resolved the fallback, reported it, and exited 0 while
//!   the cache stayed untouched (2026-09-17 — a stale message would have been
//!   the first hint, and there was none).
//!
//! The second half of the contract (G-11 + G-16, WO-001) is the **version
//! source chain**: a course repository has no `Cargo.toml`, so the pinned
//! version has to come from `SOKONANODA_VERSION` → `<repo>/sokonanoda-version.txt`
//! → `<repo>/sokonanoda.toml`'s `requires` → `<repo>/Cargo.toml`. And with no
//! expected version at all, the launcher must **never** exec a cached binary —
//! rewriting the probe's `cache(STALE …)` into `cache(unknown repo version)`
//! used to walk straight past the stale guard and run an old binary (G-16).

mod common;

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

use common::repo_root;

/// The launcher *is* a Node program, so this contract can only be exercised
/// where Node exists. CI has it; a contributor image without it must not fail
/// the suite — but a silent skip would hide the only behavioral pin on the
/// launcher, so the skip is printed, and under CI it is a hard failure.
fn require_node(what: &str) -> bool {
    let present = matches!(
        Command::new("node").arg("--version").output(),
        Ok(out) if out.status.success()
    );
    if present {
        return true;
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "{what}: `node` is not on PATH, yet this is the only test that runs the launcher. \
         CI must exercise it (the launcher is a Node program — REQUIREMENTS §2 rule 9 keeps Rust \
         off the *user* path, not Node off the *test* path)"
    );
    eprintln!("skip {what}: `node` is not on PATH (scripts/soko is a Node program)");
    false
}

fn unique_tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp cache dir");
    dir
}

/// Run `scripts/soko <args>` against an empty cache, offline, with a usable
/// fallback binary — the masked-failure shape.
fn launcher(cache: &Path, args: &[&str]) -> Output {
    let bin = env!("CARGO_BIN_EXE_sokonanoda");
    Command::new("node")
        .arg(repo_root().join("scripts/soko"))
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache)
        .env("SOKONANODA_OFFLINE", "1")
        .env("SOKONANODA_BIN", bin)
        .env("SOKONANODA_LSP_BIN", bin)
        .output()
        .expect("run scripts/soko")
}

#[test]
fn update_never_reports_success_when_the_cache_was_not_refreshed() {
    let what = "update_never_reports_success_when_the_cache_was_not_refreshed";
    if !require_node(what) {
        return;
    }
    let cache = unique_tmp("soko-update");
    let out = launcher(&cache, &["update"]);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    assert_eq!(
        out.status.code(),
        Some(3),
        "`update` means \"force-refresh the cache\". When the download cannot happen it must exit 3 \
         even though a fallback binary exists — otherwise a broken network, an unwritable cache \
         directory or a full disk all look like success.\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    assert!(
        stderr.contains("NOT refreshed"),
        "stderr must say the cache was NOT refreshed (merely failing is not enough — the operator \
         has to learn that the thing they asked for did not happen):\n{stderr}"
    );
    assert!(
        stderr.contains("SOKONANODA_OFFLINE"),
        "stderr must carry the download failure reason, not just the verdict:\n{stderr}"
    );
    assert!(
        stdout.contains("[override]"),
        "stdout must still report which binary is actually being used, so the operator knows what \
         they are falling back to:\n{stdout}"
    );
    let _ = std::fs::remove_dir_all(&cache);
}

/// Nothing resolvable at all: no override, no repo build, no cache, no network.
/// This is precisely the case the launcher's own "could not provide matching
/// binaries … Next: allow network access" message was written for — and
/// precisely the case where a `TypeError` used to kill the process first, so
/// that message was unreachable. Hermetic: a copy of the launcher inside a fake
/// repository (no `Cargo.toml` → no repo version and no repo build).
#[test]
fn an_unresolvable_environment_fails_with_the_actionable_message_not_a_crash() {
    let what = "an_unresolvable_environment_fails_with_the_actionable_message_not_a_crash";
    if !require_node(what) {
        return;
    }
    let root = unique_tmp("soko-unresolvable");
    let scripts = root.join("scripts");
    std::fs::create_dir_all(&scripts).expect("create fake repo");
    std::fs::copy(repo_root().join("scripts/soko"), scripts.join("soko")).expect("copy launcher");
    let cache = root.join("cache");

    let out = Command::new("node")
        .arg(scripts.join("soko"))
        .arg("setup")
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKONANODA_OFFLINE", "1")
        .env_remove("SOKONANODA_BIN")
        .env_remove("SOKONANODA_LSP_BIN")
        .output()
        .expect("run scripts/soko");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    assert_eq!(
        out.status.code(),
        Some(3),
        "an unresolvable environment is a diagnosed condition (exit 3), not a crash:\n\
         --- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    assert!(
        stderr.contains("could not provide matching binaries"),
        "the operator must get the actionable message (target / offline / download reason / Next), \
         which means the report must survive an unresolved binary:\n{stderr}"
    );
    assert!(
        !stderr.contains("TypeError"),
        "a stack trace is not a diagnosis:\n{stderr}"
    );
    assert!(
        stderr.contains("SOKONANODA_OFFLINE"),
        "the message must name the concrete blocker:\n{stderr}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn setup_may_resolve_a_fallback_without_failing() {
    let what = "setup_may_resolve_a_fallback_without_failing";
    if !require_node(what) {
        return;
    }
    let cache = unique_tmp("soko-setup");
    let out = launcher(&cache, &["setup"]);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    assert_eq!(
        out.status.code(),
        Some(0),
        "`setup` promises readiness, not a refreshed cache: a usable fallback is a legitimate \
         outcome and must stay quiet (only `update`, which promises the refresh itself, is loud).\n\
         --- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    let _ = std::fs::remove_dir_all(&cache);
}

// ── the version source chain (G-11) and the refuse-exec guard (G-16) ─────────

/// The version the test binary reports — i.e. what a correct marker has to
/// carry. Read from the binary instead of hard-coding it: the launcher pins
/// against `<repo>/Cargo.toml`, and both must agree for a healthy repo.
fn env_version() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("--version")
        .output()
        .expect("run sokonanoda --version");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let version = stdout
        .split_whitespace()
        .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .unwrap_or_else(|| panic!("--version must report x.y.z: {stdout}"))
        .to_string();
    assert!(
        version.split('.').count() == 3,
        "the launcher's marker contract needs a full x.y.z version, got {version:?}"
    );
    version
}

/// The vsce target this host is pinned for (the other half of the marker).
fn marker_target() -> &'static str {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "darwin-arm64"
        } else {
            "darwin-x64"
        }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "win32-arm64"
        } else {
            "win32-x64"
        }
    } else if cfg!(target_arch = "aarch64") {
        "linux-arm64"
    } else {
        "linux-x64"
    }
}

/// A fake course repository: launcher vendored into `scripts/` (exactly what
/// `scripts/new-course-repo.sh` does) and **no `Cargo.toml`** — the shape that
/// has no version source today. Hermetic: its cache is a fresh directory, it is
/// offline, and no override env var is set unless the test asks for one.
struct FakeRepo {
    root: PathBuf,
    cache: PathBuf,
    script: PathBuf,
}

impl FakeRepo {
    fn new(tag: &str) -> FakeRepo {
        let root = unique_tmp(tag);
        let scripts = root.join("scripts");
        std::fs::create_dir_all(&scripts).expect("create fake repo");
        std::fs::copy(repo_root().join("scripts/soko"), scripts.join("soko"))
            .expect("copy launcher");
        let cache = root.join("cache");
        std::fs::create_dir_all(&cache).expect("create fake cache");
        FakeRepo {
            root: root.clone(),
            cache,
            script: scripts.join("soko"),
        }
    }

    fn write(&self, name: &str, contents: &str) -> &Self {
        std::fs::write(self.root.join(name), contents).expect("write repo file");
        self
    }

    /// A cached binary plus its marker: `<version> <target>` is the contract the
    /// CLI's own `setup` writes, so this is exactly what a real cache looks like.
    fn cached(&self, base: &str, marker: Option<&str>) -> &Self {
        let name = if cfg!(windows) {
            format!("{base}.exe")
        } else {
            base.to_string()
        };
        std::fs::write(self.cache.join(&name), "#!/bin/sh\necho should-not-run\n")
            .expect("write fake binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(
                self.cache.join(&name),
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("chmod fake binary");
        }
        if let Some(marker) = marker {
            std::fs::write(
                self.cache.join(format!("{base}.version")),
                format!("{marker}\n"),
            )
            .expect("write marker");
        }
        self
    }

    /// A binary that records that it ran — used to prove the guard did *not*
    /// exec the cache. It prints the sentinel and exits 0.
    fn cached_sentinel(&self, base: &str, marker: Option<&str>) -> &Self {
        let name = if cfg!(windows) {
            format!("{base}.exe")
        } else {
            base.to_string()
        };
        std::fs::write(
            self.cache.join(&name),
            "#!/bin/sh\necho SENTINEL-EXECUTED\n",
        )
        .expect("write sentinel binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(
                self.cache.join(&name),
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("chmod sentinel binary");
        }
        if let Some(marker) = marker {
            std::fs::write(
                self.cache.join(format!("{base}.version")),
                format!("{marker}\n"),
            )
            .expect("write marker");
        }
        self
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_env(args, &[])
    }

    fn run_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = Command::new("node");
        command
            .arg(&self.script)
            .args(args)
            .env("SOKONANODA_CACHE_DIR", &self.cache)
            .env("SOKONANODA_OFFLINE", "1")
            // The ambient environment must not decide the outcome: this test
            // host may well have a real cache/pin exported.
            .env_remove("SOKONANODA_BIN")
            .env_remove("SOKONANODA_LSP_BIN")
            .env_remove("SOKONANODA_VERSION")
            .env_remove("SOKONANODA_RELEASE_BASE")
            .env_remove("SOKONANODA_REPO")
            .env_remove("SOKONANODA_HOME");
        for (key, value) in env {
            if value.is_empty() {
                command.env_remove(key);
            } else {
                command.env(key, value);
            }
        }
        command.output().expect("run scripts/soko")
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn json_of(out: &Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("expected a JSON report, got {stdout:?} ({err})"))
}

fn text(out: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// 1. A course repo that pins the version in `sokonanoda-version.txt` — the file
///    `scripts/new-course-repo.sh` already writes — resolves that version, names
///    the source, and is ready when the cache carries the matching marker.
#[test]
fn a_course_repo_pins_its_version_in_sokonanoda_version_txt() {
    let what = "a_course_repo_pins_its_version_in_sokonanoda_version_txt";
    if !require_node(what) {
        return;
    }
    let version = env_version();
    let repo = FakeRepo::new("soko-version-txt");
    repo.write("sokonanoda-version.txt", &format!("{version}\n"));
    repo.cached(
        "sokonanoda",
        Some(&format!("{version} {}", marker_target())),
    );
    repo.cached_sentinel(
        "sokonanoda-lsp",
        Some(&format!("{version} {}", marker_target())),
    );

    let out = repo.run(&["version", "--json"]);
    let json = json_of(&out);
    assert_eq!(
        json["version"], version,
        "without a Cargo.toml the pinned version must come from sokonanoda-version.txt:\n{json:#}"
    );
    assert_eq!(
        json["version_source"], "version.txt",
        "`version --json` must name which source won:\n{json:#}"
    );
    assert_eq!(
        json["cli"]["source"], "cache",
        "a marker matching <version> <target> is a usable cache entry:\n{json:#}"
    );

    let out = repo.run(&["doctor", "--json"]);
    let (stdout, stderr) = text(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a course repo pinned by its own version file is ready:\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    let json = json_of(&out);
    assert_eq!(
        json["version"], version,
        "doctor must report the pinned version:\n{json:#}"
    );
    repo.cleanup();
}

/// 2. The cache is a *different* version than the pin: the forwarded command
///    must refuse to run it, and it must say which file to edit — not leak the
///    old binary's ENOENT (`No such file or directory (os error 2)` was the
///    observable symptom of the silent stale exec).
#[test]
fn a_stale_cache_is_refused_with_the_pinned_version_and_the_file_to_edit() {
    let what = "a_stale_cache_is_refused_with_the_pinned_version_and_the_file_to_edit";
    if !require_node(what) {
        return;
    }
    let repo = FakeRepo::new("soko-stale-cache");
    repo.write("sokonanoda-version.txt", "0.42.0\n");
    repo.cached_sentinel("sokonanoda", Some("0.42.0 nowhere-else"));
    repo.cached_sentinel("sokonanoda-lsp", Some("0.42.0 nowhere-else"));

    let out = repo.run(&["query", "check", "--file", "u.sokonanoda", "--compact"]);
    let (stdout, stderr) = text(&out);
    assert_eq!(
        out.status.code(),
        Some(3),
        "a cache whose marker is not the pinned version must never be exec'd:\n\
         --- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    assert!(
        !stdout.contains("SENTINEL-EXECUTED"),
        "the stale binary really ran:\n{stdout}"
    );
    assert!(
        stderr.contains("0.42.0") && stderr.contains("sokonanoda-version.txt"),
        "the refusal must name the expected version and the file that pins it:\n{stderr}"
    );
    assert!(
        !stderr.contains("os error 2"),
        "the operator must get a diagnosis, not the stale binary's raw ENOENT:\n{stderr}"
    );
    repo.cleanup();
}

/// 3. **G-16**: no version source at all + a cache. The launcher cannot tell
///    whether the cache is current, so it must refuse (today it rewrote the
///    probe's `cache(STALE …)` into `cache(unknown repo version)` and exec'd it).
#[test]
fn without_any_version_source_the_launcher_refuses_to_exec_the_cache() {
    let what = "without_any_version_source_the_launcher_refuses_to_exec_the_cache";
    if !require_node(what) {
        return;
    }
    let repo = FakeRepo::new("soko-no-version-source");
    repo.cached_sentinel("sokonanoda", Some("0.58.0 darwin-arm64"));
    repo.cached_sentinel("sokonanoda-lsp", Some("0.58.0 darwin-arm64"));

    let out = repo.run(&["grade", "u.sokonanoda"]);
    let (stdout, stderr) = text(&out);
    assert_eq!(
        out.status.code(),
        Some(3),
        "with no expected version there is no such thing as a matching cache entry:\n\
         --- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    assert!(
        !stdout.contains("SENTINEL-EXECUTED"),
        "the unverifiable cached binary really ran:\n{stdout}"
    );
    assert!(
        stderr.contains("sokonanoda-version.txt") && stderr.contains("Cargo.toml"),
        "the refusal must name the files it looked for (that is what makes it actionable):\n{stderr}"
    );
    assert!(
        stderr.contains("SOKONANODA_VERSION"),
        "the refusal must name the other pin (env var) the operator can use:\n{stderr}"
    );
    repo.cleanup();
}

/// 4. The precedence of the chain: `SOKONANODA_VERSION` → `sokonanoda-version.txt`
///    → `sokonanoda.toml`'s `requires` → `Cargo.toml`, and a disagreement between
///    two present sources is a named-file error rather than a silent winner.
#[test]
fn the_version_source_chain_has_a_stable_precedence_and_reports_disagreement() {
    let what = "the_version_source_chain_has_a_stable_precedence_and_reports_disagreement";
    if !require_node(what) {
        return;
    }
    let version = env_version();
    let (major, minor) = {
        let mut parts = version.split('.');
        (
            parts.next().expect("major").to_string(),
            parts.next().expect("minor").to_string(),
        )
    };

    // (a) with no disagreement, `requires = "x.y.z"` alone is a download anchor.
    let repo = FakeRepo::new("soko-chain-requires");
    repo.write(
        "sokonanoda.toml",
        &format!("name = \"course\"\nrequires = \"{version}\"\n"),
    );
    repo.cached(
        "sokonanoda",
        Some(&format!("{version} {}", marker_target())),
    );
    repo.cached(
        "sokonanoda-lsp",
        Some(&format!("{version} {}", marker_target())),
    );
    let json = json_of(&repo.run(&["version", "--json"]));
    assert_eq!(
        json["version"], version,
        "a full `requires` is an anchor:\n{json:#}"
    );
    assert_eq!(json["version_source"], "manifest.requires", "{json:#}");
    repo.cleanup();

    // (b) the env var wins over every file, and it accepts the release-tag shape
    //     (`v` prefix) that `scripts/install.sh` already uses. The manifest's
    //     `requires` stays a constraint (it may name a different patch).
    let repo = FakeRepo::new("soko-chain-env");
    repo.write("sokonanoda-version.txt", &format!("{version}\n"));
    repo.write(
        "sokonanoda.toml",
        &format!("name = \"course\"\nrequires = \"{major}.{minor}.0\"\n"),
    );
    repo.write(
        "Cargo.toml",
        &format!("[package]\nversion = \"{version}\"\n"),
    );
    let json = json_of(&repo.run_env(
        &["version", "--json"],
        &[("SOKONANODA_VERSION", &format!("v{version}"))],
    ));
    assert_eq!(
        json["version"], version,
        "the env pin wins over the files (and tolerates a leading `v`):\n{json:#}"
    );
    assert_eq!(json["version_source"], "env", "{json:#}");
    repo.cleanup();

    // (e) the env pin must lose to a *file* that pins another release line —
    //     picking either one silently is how two machines end up on different
    //     toolchains while both think they are pinned.
    let repo = FakeRepo::new("soko-chain-env-conflict");
    repo.write("sokonanoda-version.txt", "0.42.0\n");
    repo.write(
        "Cargo.toml",
        &format!("[package]\nversion = \"{version}\"\n"),
    );
    let out = repo.run_env(
        &["version", "--json"],
        &[("SOKONANODA_VERSION", &format!("v{version}"))],
    );
    let json = json_of(&out);
    assert_eq!(
        json["version"],
        serde_json::Value::Null,
        "env vs file disagreement must be reported, not resolved:\n{json:#}"
    );
    assert!(
        json["version_error"]
            .as_str()
            .unwrap_or_default()
            .contains("sokonanoda-version.txt"),
        "the conflict must name the file the operator has to reconcile:\n{json:#}"
    );
    repo.cleanup();

    // (c) `requires = "0.58"` is a *constraint*, not a pin: with nothing cached
    //     and nothing downloadable it must explain that, and it must never turn
    //     into a `v0.58` download URL (or the historical literal `v?`).
    let repo = FakeRepo::new("soko-chain-constraint");
    repo.write(
        "sokonanoda.toml",
        &format!("name = \"course\"\nrequires = \"{major}.{minor}\"\n"),
    );
    let json = json_of(&repo.run(&["version", "--json"]));
    assert_eq!(
        json["version"],
        serde_json::Value::Null,
        "a major.minor requirement is not a download anchor:\n{json:#}"
    );
    assert_eq!(
        json["version_constraint"],
        format!("{major}.{minor}"),
        "{json:#}"
    );
    let out = repo.run(&["grade", "u.sokonanoda"]);
    let (stdout, stderr) = text(&out);
    assert_eq!(
        out.status.code(),
        Some(3),
        "constraint-only cannot resolve a binary:\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    assert!(
        stderr.contains("sokonanoda-version.txt") && stderr.contains(&format!("{major}.{minor}")),
        "the message must explain that `requires` is a constraint and how to pin:\n{stderr}"
    );
    assert!(
        !stderr.contains("/v?/"),
        "an unresolved version must never reach a download URL:\n{stderr}"
    );
    repo.cleanup();

    // (d) two present sources that disagree: name the files, do not pick one.
    let repo = FakeRepo::new("soko-chain-conflict");
    repo.write("sokonanoda-version.txt", "0.42.0\n");
    repo.write(
        "Cargo.toml",
        &format!("[package]\nversion = \"{version}\"\n"),
    );
    let out = repo.run(&["version", "--json"]);
    let (stdout, stderr) = text(&out);
    let json = json_of(&out);
    assert_eq!(
        json["version"],
        serde_json::Value::Null,
        "two disagreeing pins must not produce a silent winner:\n{json:#}\n{stdout}{stderr}"
    );
    let err = json["version_error"].as_str().unwrap_or_default();
    assert!(
        err.contains("sokonanoda-version.txt") && err.contains("Cargo.toml"),
        "the conflict must name both files (that is the whole point of the check):\n{json:#}"
    );
    repo.cleanup();
}

/// A one-shot-at-a-time HTTP server that serves `bytes` for every request and
/// records the request lines (the same shape `opencode.rs` uses to exercise the
/// CLI's own downloader).
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

/// A tar.gz holding both fake binaries (built with the host `tar`, like
/// `opencode.rs` does for the CLI's own downloader).
fn cli_and_lsp_tarball(dir: &Path) -> PathBuf {
    let payload = dir.join("payload");
    std::fs::create_dir_all(&payload).expect("create payload dir");
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    for (name, sentinel) in [("sokonanoda", "CLI"), ("sokonanoda-lsp", "LSP")] {
        let file = payload.join(format!("{name}{suffix}"));
        std::fs::write(&file, format!("#!/bin/sh\necho {sentinel}\nexit 0\n"))
            .expect("write payload binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
                .expect("chmod payload");
        }
    }
    let tarball = dir.join("asset.tar.gz");
    let status = Command::new("tar")
        .arg("czf")
        .arg(&tarball)
        .arg("-C")
        .arg(&payload)
        .args(["sokonanoda", &format!("sokonanoda-lsp{suffix}")])
        .status()
        .expect("run tar");
    assert!(status.success(), "tar failed");
    tarball
}

/// 5. The download URL carries the version the *course repo* pinned, never
///    `latest` — the whole point of a pin. `SOKONANODA_RELEASE_BASE` exists in
///    the CLI already (`crates/cli/src/env/target.rs`), so the launcher uses the
///    same switch and this can be exercised without the network.
#[cfg(unix)]
#[test]
fn setup_downloads_the_version_pinned_by_the_course_repo() {
    let what = "setup_downloads_the_version_pinned_by_the_course_repo";
    if !require_node(what) {
        return;
    }
    let repo = FakeRepo::new("soko-pinned-download");
    repo.write("sokonanoda-version.txt", "0.42.0\n");
    let tarball = cli_and_lsp_tarball(&repo.root);
    let (base, log) = serve_bytes(std::fs::read(&tarball).expect("read tarball"));

    let out = repo.run_env(
        &["setup"],
        &[
            ("SOKONANODA_RELEASE_BASE", &base),
            ("SOKONANODA_OFFLINE", ""),
        ],
    );
    let (stdout, stderr) = text(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "setup must fetch both pinned binaries:\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}"
    );
    let requests = log.lock().expect("log lock").join("\n");
    assert!(
        requests.contains("/v0.42.0/sokonanoda-cli-")
            && requests.contains("/v0.42.0/sokonanoda-lsp-"),
        "both downloads must use the version pinned by sokonanoda-version.txt: {requests}"
    );
    assert!(
        !requests.contains("/latest/"),
        "a pinned launcher must never address the mutable latest release: {requests}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.cache.join("sokonanoda.version"))
            .expect("cli marker")
            .trim(),
        format!("0.42.0 {}", marker_target()),
        "the marker written after the download must be <version> <target>"
    );
    repo.cleanup();
}
