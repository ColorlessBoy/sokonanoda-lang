//! Persistent compile cache ("olean"-like).
//!
//! Re-opening an unchanged `.sokonanoda` document reuses the report the kernel
//! already produced for the same (compiler version, build, prelude mode, source
//! text) instead of recompiling from scratch (design
//! `docs/design/compile-cache.md`).
//!
//! The kernel stays the single judge: an entry only holds a report the kernel
//! produced for exactly that content, and the key embeds the compiler version +
//! build stamp, so a new binary always misses. Nothing is ever *inferred* from
//! the cache — a hit only skips redundant work.
//!
//! - Disable with `SOKONANODA_NO_CACHE=1`.
//! - Relocate with `SOKONANODA_CACHE_DIR=<dir>` (tests use this).

use super::event::CompileOutput;
use super::prelude::{CompileOptions, PreludeMode};
use super::report::DocumentReport;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Bump when the on-disk entry schema changes (invalidates old entries).
/// 条目格式版本。**改动键的构成或条目 schema 时必须 bump**——旧条目一律作废。
///
/// `3`（2026-09-21 / T-A02）：`build_stamp` 从"可执行文件 mtime"换成编译期常量。
pub const CACHE_FORMAT: u32 = 3;

/// One cached compile: the document report (for the LSP) and, when the
/// producer computed it, the CLI event output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCompile {
    pub report: DocumentReport,
    pub output: Option<CompileOutput>,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    format: u32,
    report: DocumentReport,
    output: Option<CompileOutput>,
}

/// Cache root (None when disabled): honors `SOKONANODA_CACHE_DIR` (use as-is),
/// `SOKONANODA_NO_CACHE` (disable), else the platform cache dir: macOS
/// `~/Library/Caches/sokonanoda`, Windows `%LOCALAPPDATA%\sokonanoda`, else
/// `$XDG_CACHE_HOME|~/.cache` then `/sokonanoda`.
pub fn root() -> Option<PathBuf> {
    if std::env::var_os("SOKONANODA_NO_CACHE").is_some() {
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
    root().map(|root| root.join("compiled"))
}

/// 构建指纹：**编译期常量**，不是文件系统属性（T-A02 / G-27）。
///
/// 以前这里是 `current_exe()` 的 **mtime（秒级）**。两个问题：
///
/// 1. **它不是"这份二进制是什么"，而是"这个文件什么时候被写到磁盘上的"**——
///    `cp` / `tar -x` / 下载 / 安装都会改它，内容一个字节没变也照样变。
/// 2. **CLI 与 LSP 是两个不同的可执行文件** ⇒ `sokonanoda build` 预热出来的
///    条目，能否被编辑器里的 LSP 命中，取决于两个二进制的 mtime 是否落在
///    **同一秒**。本机实测四组里**三组不同秒**（release 差 619s、debug 差一天、
///    下载缓存差 2s；只有扩展自己 staged 的那对同秒，那是巧合）。
///    机械复现：`docs/gaps/repro/G27-cache-key-folds-binary-mtime.sh`。
///
/// 换成编译期常量之后，它表达的才是"这份二进制是什么"：
/// `CARGO_PKG_VERSION`（调用方已单独折进键）+ profile（debug/release 不串台）
/// + 目标三元组（跨平台不串台）。开发期想区分"同版本号的不同构建"时，
/// 用**显式环境变量** `SOKO_BUILD_LABEL`（`option_env!` 在编译期取值），
/// 而不是靠文件系统的副作用。
pub fn build_stamp() -> u64 {
    // FNV-1a 64 over the compile-time identity string. 纯函数、跨进程稳定。
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    // `PROFILE` / `TARGET` 是 **build script** 的环境变量，库 crate 里取不到
    // （`env!` 直接编译失败）。用同样是编译期常量的替代：
    // `cfg!(debug_assertions)`（debug / release 不串台）+
    // `std::env::consts::{OS, ARCH}`（跨平台不串台）。
    eat(if cfg!(debug_assertions) { b'd' } else { b'r' });
    for byte in std::env::consts::OS.bytes() {
        eat(byte);
    }
    eat(b'/');
    for byte in std::env::consts::ARCH.bytes() {
        eat(byte);
    }
    if let Some(label) = option_env!("SOKO_BUILD_LABEL") {
        eat(0);
        for byte in label.bytes() {
            eat(byte);
        }
    }
    hash
}

/// Stable FNV-1a 64 hex over (CACHE_FORMAT, CARGO_PKG_VERSION, build stamp,
/// prelude mode, source text). Pure + deterministic.
pub fn key(src: &str, options: &CompileOptions) -> String {
    key_with_build(src, options, build_stamp())
}

/// [`key`] with an explicit build stamp (pure; used by tests).
pub fn key_with_build(src: &str, options: &CompileOptions, build: u64) -> String {
    key_parts(
        CACHE_FORMAT,
        env!("CARGO_PKG_VERSION"),
        build,
        options.prelude == PreludeMode::Bare,
        src,
    )
}

fn key_parts(format: u32, version: &str, build: u64, prelude_bare: bool, src: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in format.to_le_bytes() {
        eat(byte);
    }
    for byte in version.as_bytes() {
        eat(*byte);
    }
    for byte in build.to_le_bytes() {
        eat(byte);
    }
    eat(u8::from(prelude_bare));
    for byte in src.as_bytes() {
        eat(*byte);
    }
    format!("{hash:016x}")
}

/// Load the entry for `(src, options)`, or `None` on miss/corruption/disabled.
pub fn load(src: &str, options: &CompileOptions) -> Option<CachedCompile> {
    load_in(&compiled_dir()?, &key(src, options))
}

fn load_in(dir: &Path, key: &str) -> Option<CachedCompile> {
    let bytes = std::fs::read(dir.join(format!("{key}.json"))).ok()?;
    let file: CacheFile = serde_json::from_slice(&bytes).ok()?;
    if file.format != CACHE_FORMAT {
        return None;
    }
    Some(CachedCompile {
        report: file.report,
        output: file.output,
    })
}

/// Best-effort persist (atomic: temp file + rename). Never fails the caller.
pub fn store(src: &str, options: &CompileOptions, entry: &CachedCompile) {
    if let Some(dir) = compiled_dir() {
        store_in(&dir, &key(src, options), entry);
    }
}

fn store_in(dir: &Path, key: &str, entry: &CachedCompile) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(&CacheFile {
        format: CACHE_FORMAT,
        report: entry.report.clone(),
        output: entry.output.clone(),
    }) else {
        return;
    };
    let path = dir.join(format!("{key}.json"));
    let tmp = dir.join(format!("{key}.json.tmp-{}", std::process::id()));
    if std::fs::write(&tmp, &bytes).is_err() {
        return;
    }
    // `rename` replaces an existing entry on every supported platform.
    if std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Remove all cache files; returns how many entries were removed.
pub fn clean() -> usize {
    compiled_dir().map(|dir| clean_in(&dir)).unwrap_or(0)
}

fn clean_in(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_json = path.extension().map(|ext| ext == "json").unwrap_or(false);
        if std::fs::remove_file(&path).is_ok() && is_json {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "soko-front-cache-test-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn entry_for(src: &str) -> CachedCompile {
        let file = crate::parse(src).expect("parse");
        let options = CompileOptions::default();
        let (output, report) = super::super::check::compile_all_with(&file, &options);
        CachedCompile {
            report,
            output: Some(output),
        }
    }

    #[test]
    fn key_is_deterministic_and_sensitive() {
        let full = CompileOptions::default();
        let bare = CompileOptions {
            prelude: PreludeMode::Bare,
        };
        assert_eq!(key_with_build("a", &full, 7), key_with_build("a", &full, 7));
        assert_ne!(key_with_build("a", &full, 7), key_with_build("b", &full, 7));
        assert_ne!(key_with_build("a", &full, 7), key_with_build("a", &bare, 7));
        assert_ne!(key_with_build("a", &full, 7), key_with_build("a", &full, 8));
        assert_ne!(
            key_parts(CACHE_FORMAT, "0.1.0", 7, false, "a"),
            key_parts(CACHE_FORMAT, "0.2.0", 7, false, "a"),
            "a version bump must miss"
        );
        assert_ne!(
            key_parts(CACHE_FORMAT, "0.1.0", 7, false, "a"),
            key_parts(CACHE_FORMAT + 1, "0.1.0", 7, false, "a"),
            "a schema bump must miss"
        );
    }

    #[test]
    fn store_then_load_round_trips_and_is_key_scoped() {
        let dir = tmp_dir("roundtrip");
        let src = "def two : Nat := 2\nexample : Prop := sorry\n";
        let entry = entry_for(src);
        assert!(
            entry
                .output
                .as_ref()
                .is_some_and(|out| !out.events.is_empty()),
            "the combined entry point must carry CLI events"
        );
        let k = key_parts(CACHE_FORMAT, "0.48.0", 7, false, src);
        store_in(&dir, &k, &entry);
        let loaded = load_in(&dir, &k).expect("cache hit");
        assert_eq!(loaded.report.decls.len(), entry.report.decls.len());
        assert_eq!(loaded.report.decls[0].status, entry.report.decls[0].status);
        assert_eq!(
            loaded.output.map(|o| o.events),
            entry.output.map(|o| o.events)
        );
        let other = key_parts(CACHE_FORMAT, "0.48.0", 7, false, "def x : Nat := 1\n");
        assert!(load_in(&dir, &other).is_none(), "different source misses");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn format_mismatch_is_a_miss() {
        let dir = tmp_dir("format");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("k.json"),
            br#"{"format":999,"report":{"decls":[],"hovers":[],"hover_cmds":[],"errors":[],"checks":[],"warnings":[]},"output":null}"#,
        )
        .unwrap();
        assert!(load_in(&dir, "k").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_removes_entries() {
        let dir = tmp_dir("clean");
        let entry = CachedCompile {
            report: DocumentReport::default(),
            output: None,
        };
        store_in(&dir, "a", &entry);
        store_in(&dir, "b", &entry);
        assert_eq!(clean_in(&dir), 2);
        assert!(load_in(&dir, "a").is_none());
        assert_eq!(clean_in(&dir), 0, "a second clean finds nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
