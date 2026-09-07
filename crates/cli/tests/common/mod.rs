//! Shared fixtures for the CLI integration suites (protocol / skill
//! conformance tests guard the same machine contract from two sides).
//! Each test binary compiles its own copy; not every binary uses every item.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The closed event vocabulary of `--json` (docs/protocol.md).
/// Protocol tests assert the emitted stream stays inside it; skill tests
/// assert the agent-facing skill files only advertise these names.
pub const EVENT_VOCABULARY: [&str; 7] = [
    "decl.checked",
    "example.checked",
    "expr.typed",
    "expr.reduced",
    "decl.printed",
    "exercise.open",
    "diagnostic",
];

/// The closed delta-event vocabulary of `sokonanoda watch` (docs/protocol.md,
/// watch stream section): per-version state transitions from front::session.
pub const WATCH_VOCABULARY: [&str; 7] = [
    "file.changed",
    "decl.checked",
    "decl.failed",
    "exercise.opened",
    "exercise.solved",
    "exercise.failed",
    "diagnostic",
];

/// Custom LSP requests exposed to clients (goal view + hint ladder,
/// docs/protocol.md).
pub const LSP_CUSTOM_METHODS: [&str; 3] = ["soko/goals", "soko/nextHole", "soko/hints"];

/// Repository root (this crate lives at `<root>/crates/cli`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}
