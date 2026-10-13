//! **产物的读侧**（2026-10-13 从 `artifacts.rs` 拆出来 —— `AGENTS.md` 硬规则 5；
//! 判据 ⑨（`STRUCT-SRC-LINES`）对 `artifacts.rs` 是**只许减不许增** ✓）。
//!
//! 两件事，**缺一不可**：
//! * **§8.2 完整性三道**（[`read`] / [`read_pair`]）：全过才用，任一条不过 ⇒ **当不存在**；
//! * **§8.3 的清单闸门**（[`read_shared`] / [`lock_allows`]）：**内容寻址挡不住伪造**
//!   （键由**输入**算出，不校验产物**内容**是否真由这份输入编出来）⇒ 跨 root / 下载来的
//!   产物**必须**另有信任来源 —— `artifacts.lock.json`（**钉在模块根**，要进仓库 ✓）。
//!
//! ⚠ **默认关**：`read_shared` 只在 §8.3 的闸门打开时才被调用（`project::session`），
//! 闸门关着时走的是[检查点键](crate::project::session)那条路 ⇒ 外来产物**不在候选里** ✓。
//!
//! 公开路径不变：`artifacts.rs` 用 `pub use` re-export（硬规则 5 ✓）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    digest_hex, key_is_safe, meta_path, payload_path, prelude_name, target, ArtifactMeta,
    ARTIFACT_FORMAT,
};
use crate::compile::CompileOptions;

/// 读一份产物 —— **三道全过**才返回 `Some`，否则 `None`（**当不存在** ✓ §8.2）。
///
/// | 道 | 比什么 | 挡掉什么 |
/// |---|---|---|
/// | ① | `meta.json` 的 `format` / `front_version` / `build_stamp` / `target` / `prelude` 与**本机逐项相同** | 跨版本、跨平台、跨 prelude 档、**跨构建**复用 ✗ |
/// | ② | 载荷字节数 == `meta.bytes` **且** 摘要 == `meta.digest` | **截断/损坏的下载**、磁盘坏了、改了一个字节 ✗ |
/// | ③ | 调用方给的 `key` **就是文件名**（第③道是"键可重算"的落点：`key` 由**磁盘上的源文本**按 §3 算出 ⇒ 源文本变一个字 ⇒ 键变 ⇒ 旧产物再也匹配不上 ✓） | 错配（拿 A 的产物当 B 的） ✗ |
///
/// ⚠ **§8.3 的安全模型必须读懂再用**：三道**挡不住伪造** —— 一个手工构造的
/// `<key>.bin` + `meta.json`（`digest` 自洽）会被接受，而它的内容可能是一批
/// **不成立的声明** ⇒ **错编**。所以：① 默认**关**；② 只从固定发布者按 tag 锁定拉；
/// ③ 用了就**报一行来源**；④ 课程仓可钉 `artifacts.lock.json`。本函数**不负责**这些。
pub fn read(root: &Path, key: &str, options: &CompileOptions) -> Option<String> {
    let (_meta, bytes) = read_pair(root, key, options)?;
    // ③ 键即文件名（上面两处路径都由 `key` 构造 ⇒ 天然成立；非法形状在函数入口已拦 ✓）。
    String::from_utf8(bytes).ok()
}

/// **§8.2 三道**的落点（[`read`] 与 [`read_shared`] **共用**，别复制一份 ✗）：
/// 过三道 ⇒ 交出 `(凭据, 载荷字节)`；任一不过 ⇒ `None`（调用方一律当"没有产物" ✓）。
fn read_pair(root: &Path, key: &str, options: &CompileOptions) -> Option<(ArtifactMeta, Vec<u8>)> {
    if !key_is_safe(key) {
        return None;
    }
    // ① 凭据必须**先**读出来并逐项相符。
    let meta_text = std::fs::read_to_string(meta_path(root, key)).ok()?;
    let meta: ArtifactMeta = serde_json::from_str(&meta_text).ok()?;
    if meta.format != ARTIFACT_FORMAT
        || meta.front_version != env!("CARGO_PKG_VERSION")
        || meta.build_stamp != crate::compile::cache::build_stamp()
        || meta.target != target()
        || meta.prelude != prelude_name(options)
    {
        return None;
    }
    // ② 载荷：字节数与摘要都要对。
    let bytes = std::fs::read(payload_path(root, key)).ok()?;
    if bytes.len() != meta.bytes || digest_hex(&bytes) != meta.digest {
        return None;
    }
    Some((meta, bytes))
}

// ───────────────────────────────────────────────────────────────────────────
// **§8.3 的清单**（`artifacts.lock.json`）—— 内容寻址产物的**信任来源**
// ───────────────────────────────────────────────────────────────────────────

/// 清单文件名（**钉在模块根**，与版本钉文件同一条"按版本钉"的纪律 ✓ —— 设计 §8.3 末条）。
pub const LOCK_FILE: &str = "artifacts.lock.json";

/// `<模块根>/artifacts.lock.json`。
///
/// ⚠ 它**不在** `.sokonanoda/` 里，而在**模块根**：它是要**进仓库**的那份清单
/// （设计 §8.3「课程仓可把'允许的产物清单'钉进仓库」✓）—— `.sokonanoda/` 整个是自忽略的 ✗。
pub fn lock_path(root: &Path) -> PathBuf {
    root.join(LOCK_FILE)
}

/// 清单的**形状**（设计 §8.3：「清单里记每个 `key` 的产物摘要」✓）。
///
/// 形状（本版 `version = 1`）：
///
/// ```json
/// {
///   "version": 1,
///   "artifacts": [
///     { "key": "<内容寻址键>", "digest": "<载荷摘要>", "bytes": 80773 }
///   ]
/// }
/// ```
///
/// * `key` = **产物文件名**（§3 的内容寻址键 ✓）；
/// * `digest`/`bytes` = **载荷**的摘要与字节数 ⇒ **与 `meta.json` 再对一次账**
///   （清单被篡改 ⇒ 对不上 ⇒ 拒绝 ✓；载荷被篡改 ⇒ §8.2 第②道先拦 ✓）。
///
/// ⚠ **没做**：密码学签名（发布者公钥）。今天这层提供的是"**按摘要钉死**"——
/// 篡改清单里的摘要 ⇒ 拒绝 ✓，但**伪造整份清单**（连同自洽的摘要）挡不住 ✗。
/// 这是设计 §8.3 第 3 条里"**签名**"那一半的**未实现部分**，如实记在 VERDICT 的边界段 ✓。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLock {
    /// 清单格式号（不认的号 ⇒ 整份当**空** ⇒ 什么都不允许 ✓，与产物的 `format` 同一条纪律）。
    pub version: u32,
    /// 允许的产物（**空表 = 什么都不允许** ✓ —— 默认关的第二种表达 ✓）。
    #[serde(default)]
    pub artifacts: Vec<ArtifactLockEntry>,
}

/// 清单里的一条。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLockEntry {
    /// 产物文件名（§3 的内容寻址键）。
    pub key: String,
    /// 载荷摘要（`meta.json` 的 `digest` 必须**逐字节**等于它 ✓）。
    pub digest: String,
    /// 载荷字节数。
    pub bytes: usize,
}

/// 清单格式号。
pub const LOCK_FORMAT: u32 = 1;

/// **§8.3 的闸门**：载荷是不是**清单允许**的那一份。
///
/// 只做一件事、且**只信两条**：`key` 在清单里、且清单的 `digest`/`bytes` 与**凭据**逐项相同
/// （凭据本身已经过 §8.2 第①②道 ⇒ 它描述的**就是**载荷 ✓）⇒ 篡改清单 ⇒ 在此拦下 ✓。
///
/// 读不到 / 解析不了 / 格式号不认 ⇒ **一律不允许**（fail-closed ✓，与"回退是默认"同源）。
pub fn lock_allows(root: &Path, key: &str, meta: &ArtifactMeta) -> bool {
    let Ok(text) = std::fs::read_to_string(lock_path(root)) else {
        return false;
    };
    let Ok(lock) = serde_json::from_str::<ArtifactLock>(&text) else {
        return false;
    };
    if lock.version != LOCK_FORMAT {
        return false;
    }
    lock.artifacts
        .iter()
        .any(|entry| entry.key == key && entry.digest == meta.digest && entry.bytes == meta.bytes)
}

/// **共享档（§8.3 闸门开）下的读**：§8.2 三道 **+** §8.3 清单。
///
/// 两道闸门**都要**过才交出载荷 —— 缺任何一道，外来产物就默认被吃了 ✗
/// （这正是"默认关 + 清单"必须与内容寻址**同一轮**落的原因 ✓）。
pub fn read_shared(root: &Path, key: &str, options: &CompileOptions) -> Option<String> {
    let (meta, bytes) = read_pair(root, key, options)?;
    if !lock_allows(root, key, &meta) {
        return None;
    }
    String::from_utf8(bytes).ok()
}
