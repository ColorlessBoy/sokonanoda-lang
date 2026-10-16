//! **产物的磁盘布局与凭据维度**（2026-10-16 从 `artifacts.rs` 拆出来 ——
//! `AGENTS.md` 硬规则 5；判据 ⑨（`STRUCT-SRC-LINES`）对 `artifacts.rs` 是
//! **只许减不许增** ✓）。
//!
//! 两块，都是"**产物长什么样**"这一件事的两面：
//! * **布局**（设计 `docs/design/module-artifacts.md` §8.1「存哪、叫什么」）：
//!   [`dir`] / [`payload_path`] / [`meta_path`] / [`exists`]，外加
//!   [`ensure_self_ignore`]（让 `<模块根>/.sokonanoda/` 在 `git status` 里看不见 ✓）
//!   与 [`key_is_safe`]（`key` 的形状闸门 —— 挡目录穿越 ✓）；
//! * **凭据维度**（§8.2 第①道要比的那几项）：[`ArtifactMeta`] 与它的生产者
//!   [`target`] / [`prelude_name`] / [`digest_hex`]（第②道的一半）· [`ARTIFACT_FORMAT`]；
//! * [`MAX_ARTIFACT_PAIRS`]：布局的**上界**（超了按 mtime 淘汰，见 `artifacts.rs` 的 `prune`）。
//!
//! 公开路径不变：`artifacts.rs` 用 `pub use` re-export（硬规则 5 ✓）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::compile::CompileOptions;
use crate::compile::PreludeMode;

/// 产物**格式**版本。与 `compiled/` 的 `CACHE_FORMAT` **分开计数**：两者的
/// 载荷形状无关（一个是 `{report, output}`，一个是内核环境文本）⇒ 一个变不该
/// 让另一个失效 ✓。
pub const ARTIFACT_FORMAT: u32 = 1;

/// **产物对数的上界**（T1-B 批 3 收尾，2026-10-09）。
///
/// **为什么必须有**：`compiled/` 那条**天然有界**（"一份产物对应一个入口文件" ✓）；
/// 产物这条**没有** —— 键 = **库层闭包的 Merkle 键** ⇒ 用户每改一次库层就多一个新键 ✓
/// ⇒ 老产物**再也没人读**却一直占着盘 ✗（设计 §8.1 只说了"文件名即内容寻址"，没定上界 ✗）。
///
/// **怎么淘汰**：按 **mtime 最旧**先走（够用、判据便宜 ✓）；一次写之后最多扫一遍目录 ✓。
/// 64 对 ≈ 真课程量级下"几十 MB"（单份 ~1MB 量级 ✓）—— 取的是**别无限长**，
/// 不是"省盘" ✓（要更省就调小，或将来换成"按最近命中"的 LRU ✓）。
pub const MAX_ARTIFACT_PAIRS: usize = 64;

/// `<模块根>/.sokonanoda/artifacts/`。
pub fn dir(root: &Path) -> PathBuf {
    crate::project::cache::artifacts_dir(root).join("artifacts")
}

/// **那一对产物在不在**（T4-B/A5c 用 · 2026-10-09）：`payload` 与 `meta` **都要在** ✓。
///
/// ⚠ 只作**提示**用（"要不要做一次投机预热" ✓）—— 真正的判据仍是 [`read`] 的完整性三道 ✓
/// （这里说在、`read` 判坏了 ⇒ 走回退 ✓，不会静默用坏数据 ✓）。
pub fn exists(root: &Path, key: &str) -> bool {
    payload_path(root, key).is_file() && meta_path(root, key).is_file()
}
/// `<key>.bin`（载荷 = NDJSON 文本，`ExportFile::to_ndjson` 写出来的那份 ✓）。
pub fn payload_path(root: &Path, key: &str) -> PathBuf {
    dir(root).join(format!("{key}.bin"))
}

/// `<key>.meta.json`（**使用前**的完整性凭据 ✓）。
pub fn meta_path(root: &Path, key: &str) -> PathBuf {
    dir(root).join(format!("{key}.meta.json"))
}

/// 产物的**完整性凭据**。字段就是 §8.2 第①道要比的维度 —— **逐项相同才用** ✓。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMeta {
    /// = [`ARTIFACT_FORMAT`]
    pub format: u32,
    /// 前端版本（`CARGO_PKG_VERSION`）
    pub front_version: String,
    /// 编译期构建身份（`compile::cache::build_stamp`，**跨进程稳定** ✓）
    pub build_stamp: u64,
    /// `os/arch`
    pub target: String,
    /// prelude 模式（`"full"` / `"bare"`）
    pub prelude: String,
    /// 载荷字节数（第②道的一半）
    pub bytes: usize,
    /// 载荷的内容摘要（第②道的另一半）
    pub digest: String,
}

/// 本机的 `os/arch`（与 `compiled/` 的 `platform` 同口径 ✓）。
pub(crate) fn target() -> String {
    format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub(crate) fn prelude_name(options: &CompileOptions) -> &'static str {
    match options.prelude {
        PreludeMode::Full => "full",
        PreludeMode::Bare => "bare",
    }
}

/// FNV-1a 64 hex（与 `ProjectPlan::digest` / `compile::cache` 同一条哈希函数 ✓）。
pub(crate) fn digest_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// `key` 只允许由 `cache::key` / `module_keys` 产生的那类字符组成。
///
/// **为什么必须挡**：`key` 将来可能来自**下载来的清单**（§8.3）⇒ 它一旦含 `/` 或
/// `..`，[`payload_path`] 就变成**目录穿越**（写/读到模块根之外）✗。
/// 这里是一条**低成本、判据明确**的边界（不合法 ⇒ 与"没有产物"同处置 ✓）。
pub(crate) fn key_is_safe(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// 让 `<模块根>/.sokonanoda/` **自忽略**（内容恰好一行 `*`，与 `compiled/` 同一手法 ✓）。
///
/// **为什么产物自己也要做这件事**：`compiled/` 的 `.gitignore` 是**缓存写路径**建的
/// （[`crate::project::cache`] 的 `ensure_layout`）；一个只用产物、还没写过缓存条目的
/// 模块根，若没人放这个文件，`git status` 里就会冒出 `?? .sokonanoda/` ✗ ——
/// 那是"用户仓库被工具污染"，不是小噪音。内容与缓存侧**逐字相同**（实测 `*` 一行
/// 才完全看不见；`*` + `!.gitignore` 反而会漏 `?? .sokonanoda/` ✗）。
pub(super) fn ensure_self_ignore(root: &Path) {
    let base = crate::project::cache::artifacts_dir(root);
    let ignore = base.join(".gitignore");
    if !ignore.exists() {
        let _ = std::fs::write(&ignore, "*\n");
    }
}
