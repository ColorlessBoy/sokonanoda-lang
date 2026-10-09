//! Shared fixtures for the CLI integration suites (protocol / skill
//! conformance tests guard the same machine contract from two sides).
//! Each test binary compiles its own copy; not every binary uses every item.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The closed event vocabulary of `--json` (docs/protocol.md): the batch-run
/// stream plus the `sokonanoda course` progress map (docs/protocol.md
/// "Course map"). Protocol tests assert the emitted stream stays inside it;
/// skill tests assert the agent-facing skill files only advertise these names.
pub const EVENT_VOCABULARY: [&str; 10] = [
    "decl.checked",
    "example.checked",
    "expr.typed",
    "expr.reduced",
    "decl.printed",
    "exercise.open",
    "diagnostic",
    "warning",
    "course.unit",
    "course.summary",
];

/// The closed delta-event vocabulary of `sokonanoda watch` (docs/protocol.md,
/// watch stream section): per-version state transitions from front::session.
/// `file.changed` is the deprecated alias of the canonical `file.didChange`
/// opener; it is accepted for one minor cycle but never emitted together.
pub const WATCH_VOCABULARY: [&str; 8] = [
    "file.didChange",
    "file.changed",
    "decl.checked",
    "decl.failed",
    "exercise.opened",
    "exercise.solved",
    "exercise.failed",
    "diagnostic",
];

/// The first line of the watch stream: the service handshake (not a delta
/// event), mirroring LSP `soko/version`.
pub const WATCH_HANDSHAKE: &str = "service.hello";

/// Custom LSP requests exposed to clients (goal view + hint ladder +
/// per-tactic cursor state + project closure, docs/protocol.md).
///
/// `soko/project` was missing from this list (E22, 2026-09-27): the server has
/// registered it since 0.58.0 (`crates/lsp/src/lib.rs` 的
/// `custom_method("soko/project", …)`) and `docs/protocol.md` §`soko/project`
/// documents it, but the skill vocabulary guard did not know the name — so any
/// skill that mentioned it was rejected as "not part of the protocol contract"
/// even though it is. Keeping this list in sync is the point of the guard.
pub const LSP_CUSTOM_METHODS: [&str; 7] = [
    "soko/goals",
    // `soko/goalAt`（2026-10-09）：光标处**那一条**声明的 goal 视图 —— Lean
    // `$/lean/plainGoal` 的声明级对应物（`docs/protocol.md` §`soko/goalAt`）。
    // 加新方法时**这份清单与 `docs/protocol.md` 必须同一轮改** ✓ —— 这条守卫
    // 就是为此存在的（加它时正是它判红拦下的 ✓）。
    "soko/goalAt",
    "soko/nextHole",
    "soko/hints",
    "soko/stateAt",
    "soko/project",
    "soko/version",
];

/// Repository root (this crate lives at `<root>/crates/cli`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}
