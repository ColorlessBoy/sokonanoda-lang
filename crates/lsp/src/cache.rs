//! Persistent compile cache ("olean"-like).
//!
//! Opening an unchanged `.sokonanoda` document reuses the [`DocumentReport`] the
//! kernel already produced for the same (compiler version, prelude mode, source
//! text), instead of recompiling from scratch — this is what keeps opening many
//! files fast once a workspace fills up (design `docs/design/compile-cache.md`).
//!
//! The kernel stays the single judge: an entry is only written from a report the
//! kernel produced for exactly that content, and the key embeds the compiler
//! version, so a new binary always misses. Nothing is ever *inferred* from the
//! cache — a hit only skips redundant work.
//!
//! - Disable with `SOKONANODA_NO_CACHE=1`.
//! - Relocate with `SOKONANODA_CACHE_DIR=<dir>` (tests use this).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sokonanoda_front::compile::DocumentReport;

/// Bump when the on-disk report schema changes (invalidates old entries).
const CACHE_FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
struct CacheFile {
    format: u32,
    report: DocumentReport,
}

fn cache_root() -> Option<PathBuf> {
    if std::env::var_os("SOKONANODA_NO_CACHE").is_some() {
        return None;
    }
    // Never touch the real cache from `cargo test` (unit/integration tests run
    // `refresh` a lot; a shared cache would both pollute the developer's cache
    // and turn the perf tests into cache hits). The `_in` helpers are tested
    // directly instead.
    if cfg!(test) {
        return None;
    }
    if let Some(dir) = std::env::var_os("SOKONANODA_CACHE_DIR") {
        return Some(PathBuf::from(dir));
    }
    if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches/sokonanoda"))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map(|h| PathBuf::from(h).join("sokonanoda"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|p| p.join("sokonanoda"))
    }
}

fn compiled_dir() -> Option<PathBuf> {
    cache_root().map(|root| root.join("compiled"))
}

/// A cheap build fingerprint: the running executable's mtime (seconds). The
/// compiler version alone is not enough during development — rebuilding with
/// the same version must not reuse a stale report, so the stamp participates in
/// the key. `0` when unavailable (still version-scoped).
pub fn build_stamp() -> u64 {
    std::env::current_exe()
        .and_then(std::fs::metadata)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Stable FNV-1a 64 over `format | version | build stamp | mode | text` (hex).
/// Stable across runs/platforms, unlike `DefaultHasher`.
pub fn key(version: &str, prelude_bare: bool, text: &str) -> String {
    key_with_build(version, prelude_bare, text, build_stamp())
}

/// [`key`] with an explicit build stamp (pure; used by tests).
pub fn key_with_build(version: &str, prelude_bare: bool, text: &str, build: u64) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in CACHE_FORMAT.to_le_bytes() {
        eat(byte);
    }
    for byte in version.as_bytes() {
        eat(*byte);
    }
    for byte in build.to_le_bytes() {
        eat(byte);
    }
    eat(u8::from(prelude_bare));
    for byte in text.as_bytes() {
        eat(*byte);
    }
    format!("{hash:016x}")
}

/// Load the cached report for `key`, or `None` on miss / unreadable entry.
pub fn load(key: &str) -> Option<DocumentReport> {
    load_in(compiled_dir()?.as_path(), key)
}

fn load_in(dir: &Path, key: &str) -> Option<DocumentReport> {
    let bytes = std::fs::read(dir.join(format!("{key}.json"))).ok()?;
    let file: CacheFile = serde_json::from_slice(&bytes).ok()?;
    if file.format != CACHE_FORMAT {
        return None;
    }
    Some(file.report)
}

/// Persist `report` under `key` (best-effort; cache failures never fail a
/// request). Writes to a temp file and renames so a reader never sees a
/// half-written entry.
pub fn store(key: &str, report: &DocumentReport) {
    if let Some(dir) = compiled_dir() {
        store_in(&dir, key, report);
    }
}

fn store_in(dir: &Path, key: &str, report: &DocumentReport) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(&CacheFile {
        format: CACHE_FORMAT,
        report: report.clone(),
    }) else {
        return;
    };
    write_atomic(&dir.join(format!("{key}.json")), &bytes);
}

fn write_atomic(path: &Path, bytes: &[u8]) {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    if std::fs::write(&tmp, bytes).is_err() {
        return;
    }
    // `rename` replaces an existing entry on every supported platform.
    if std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokonanoda_front::compile::CompileOptions;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("soko-cache-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn key_is_stable_and_content_sensitive() {
        let k = |v: &str, bare: bool, t: &str| key_with_build(v, bare, t, 7);
        assert_eq!(k("0.48.0", false, "a"), k("0.48.0", false, "a"));
        assert_ne!(k("0.48.0", false, "a"), k("0.48.0", false, "b"));
        assert_ne!(k("0.48.0", false, "a"), k("0.48.1", false, "a"));
        assert_ne!(k("0.48.0", false, "a"), k("0.48.0", true, "a"));
        assert_ne!(
            key_with_build("0.48.0", false, "a", 7),
            key_with_build("0.48.0", false, "a", 8),
            "a rebuild with the same version must miss"
        );
    }

    #[test]
    fn store_then_load_round_trips_and_is_key_scoped() {
        let dir = tmp_dir("roundtrip");
        let src = "def two : Nat := 2\nexample : Prop := sorry\n";
        let _ = CompileOptions::default();
        let report = sokonanoda_front::compile::check_document(
            &sokonanoda_front::parse(src).expect("parse"),
        );
        let k = key_with_build("0.48.0", false, src, 7);
        store_in(&dir, &k, &report);
        let loaded = load_in(&dir, &k).expect("cache hit");
        assert_eq!(loaded.decls.len(), report.decls.len());
        assert_eq!(loaded.decls[0].status, report.decls[0].status);
        assert!(load_in(&dir, "deadbeef").is_none(), "unknown key misses");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn format_mismatch_is_a_miss() {
        let dir = tmp_dir("format");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("k.json"),
            br#"{"format":999,"report":{"decls":[],"hovers":[],"hover_cmds":[],"errors":[],"checks":[],"warnings":[]}}"#,
        )
        .unwrap();
        assert!(load_in(&dir, "k").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
