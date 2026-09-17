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

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
