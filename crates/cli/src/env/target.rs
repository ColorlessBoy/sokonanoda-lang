//! Host target + cache layout, compatible with the release VSIX / opencode
//! plugin: markers are `<version> <vsce-target>` in `<name>.version` files.

use std::path::PathBuf;

/// Compile-time target triple (set by `build.rs`).
pub const TARGET: &str = env!("SOKONANODA_TARGET");

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// VS Code / vsce target name for this binary's triple (marker compatibility).
pub fn vsce_target() -> &'static str {
    match TARGET {
        "aarch64-apple-darwin" => "darwin-arm64",
        "x86_64-apple-darwin" => "darwin-x64",
        "x86_64-unknown-linux-gnu" => "linux-x64",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        "x86_64-unknown-linux-musl" => "alpine-x64",
        "aarch64-unknown-linux-musl" => "alpine-arm64",
        "x86_64-pc-windows-msvc" => "win32-x64",
        "aarch64-pc-windows-msvc" => "win32-arm64",
        _ => "unknown",
    }
}

/// Platform binary file name (`sokonanoda` / `sokonanoda.exe`).
pub fn binary_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Cache directory (`SOKONANODA_CACHE_DIR` or the shared default).
pub fn cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SOKONANODA_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local")
        .join("share")
        .join("sokonanoda")
        .join("bin")
}

pub fn binary_path(base: &str) -> PathBuf {
    cache_dir().join(binary_name(base))
}

pub fn marker_path(base: &str) -> PathBuf {
    cache_dir().join(format!("{base}.version"))
}

/// The marker value this binary expects: `<version> <vsce-target>`.
pub fn expected_marker() -> String {
    format!("{} {}", version(), vsce_target())
}

pub fn read_marker(base: &str) -> Option<String> {
    std::fs::read_to_string(marker_path(base))
        .ok()
        .map(|s| s.trim().to_string())
}

/// True when the binary is present and its marker matches this version+target.
pub fn binary_ready(base: &str) -> bool {
    binary_path(base).is_file() && read_marker(base).as_deref() == Some(expected_marker().as_str())
}

/// Download opt-out (`SOKONANODA_OFFLINE=1`).
pub fn offline() -> bool {
    std::env::var_os("SOKONANODA_OFFLINE").is_some()
        || std::env::var_os("SOKONANODA_LSP_OFFLINE").is_some()
}

/// Release download base (overridable for tests / self-hosting).
pub fn release_base() -> String {
    std::env::var("SOKONANODA_RELEASE_BASE").unwrap_or_else(|_| {
        "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download".to_string()
    })
}
