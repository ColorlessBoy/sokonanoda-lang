//! **模块产物的磁盘存放 + 完整性三道**（T1-B 批 2 的前半，2026-10-09）。
//!
//! 设计 = `docs/design/module-artifacts.md` **§8.1–§8.4**。本模块只管**存/取**：
//! * 存哪、叫什么（§8.1）：`<模块根>/.sokonanoda/artifacts/<key>.bin` 与
//!   `<key>.meta.json`；**文件名即内容寻址**（`key` = 每模块 Merkle 键，
//!   见 [`crate::project::ProjectPlan::module_keys`] ✓）。
//! * 完整性三道（§8.2）：**全过才用**，任一条不过 ⇒ **当不存在**（调用方照常本地重编 ✓）。
//! * 安全模型（§8.3）如实写在 [`read`] 的文档里：**加载产物 = 把声明直接注入内核**
//!   ⇒ 内容寻址**挡不住伪造**（键由**输入**算出，不校验产物内容真的由这份输入编出）。
//!   ⇒ 本模块只做 §8.2 那三道（损坏 / 错配），**默认开关与来源策略在调用方**。
//! * 离线降级（§8.4）：本模块**从不联网**，也没有"失败要报错"的形态 —— 一切失败
//!   都是"当不存在" ⇒ 天然的静默回退 ✓。
//!
//! ## 命名（避免与既有函数混）
//!
//! [`crate::project::cache::artifacts_dir`] 指的是 **`<模块根>/.sokonanoda/`** 这一层
//! （它下面还有 `compiled/`）。本模块的 [`dir`] 指的是**再下一层** `artifacts/`
//! —— 与设计文档 §8.1 的路径逐字对应 ✓。
//!
//! ## 这一版**不接线**
//!
//! 本模块只提供存/取与判据；**写入口与消费入口**（session 里"每模块编完写一份"
//! 与"下次进程读回来接着编"）是批 2 的后半 —— 后者还差**前端表**（`known`/
//! `inductives`/`defs`）的来路（要么序列化，要么从内核环境重建；见 PLAN §23）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::compile::CompileOptions;
use crate::compile::PreludeMode;

/// 产物**格式**版本。与 `compiled/` 的 `CACHE_FORMAT` **分开计数**：两者的
/// 载荷形状无关（一个是 `{report, output}`，一个是内核环境文本）⇒ 一个变不该
/// 让另一个失效 ✓。
pub const ARTIFACT_FORMAT: u32 = 1;

/// `<模块根>/.sokonanoda/artifacts/`。
pub fn dir(root: &Path) -> PathBuf {
    crate::project::cache::artifacts_dir(root).join("artifacts")
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
fn target() -> String {
    format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn prelude_name(options: &CompileOptions) -> &'static str {
    match options.prelude {
        PreludeMode::Full => "full",
        PreludeMode::Bare => "bare",
    }
}

/// FNV-1a 64 hex（与 `ProjectPlan::digest` / `compile::cache` 同一条哈希函数 ✓）。
fn digest_hex(bytes: &[u8]) -> String {
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
fn key_is_safe(key: &str) -> bool {
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
fn ensure_self_ignore(root: &Path) {
    let base = crate::project::cache::artifacts_dir(root);
    let ignore = base.join(".gitignore");
    if !ignore.exists() {
        let _ = std::fs::write(&ignore, "*\n");
    }
}

/// 写一份产物：**先载荷、后凭据**（顺序是判据的一部分 ✓ —— 崩在中间只会留下
/// 一份没有凭据的载荷 ⇒ 下次读是 miss，**绝不会**读到半份产物 ✓）。
pub fn write(
    root: &Path,
    key: &str,
    payload: &str,
    options: &CompileOptions,
) -> std::io::Result<PathBuf> {
    if !key_is_safe(key) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("拒绝写入：产物键形状不合法（{key:?}）"),
        ));
    }
    std::fs::create_dir_all(dir(root))?;
    ensure_self_ignore(root);
    let bin = payload_path(root, key);
    std::fs::write(&bin, payload.as_bytes())?;
    let meta = ArtifactMeta {
        format: ARTIFACT_FORMAT,
        front_version: env!("CARGO_PKG_VERSION").to_string(),
        build_stamp: crate::compile::cache::build_stamp(),
        target: target(),
        prelude: prelude_name(options).to_string(),
        bytes: payload.len(),
        digest: digest_hex(payload.as_bytes()),
    };
    let json = serde_json::to_string(&meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(meta_path(root, key), format!("{json}\n"))?;
    Ok(bin)
}

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
    // ③ 键即文件名（上面两处路径都由 `key` 构造 ⇒ 天然成立；非法形状在函数入口已拦 ✓）。
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个用例一个**独立**临时根（pid + 纳秒 + 标签 ⇒ 并发跑也不撞 ✓）。
    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "soko-artifacts-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    fn options() -> CompileOptions {
        CompileOptions::default()
    }

    const KEY: &str = "closure-0123456789abcdef";

    /// **正路**：写一份、读回来**逐字节相同** ✓。
    #[test]
    fn artifact_round_trips_through_the_disk() {
        let root = temp_root("round-trip");
        let payload =
            "{\"meta\":{}}\n{\"axiom\":1,\"levelParams\":[],\"type\":0,\"isUnsafe\":false}\n";
        write(&root, KEY, payload, &options()).expect("write");
        assert_eq!(
            read(&root, KEY, &options()).as_deref(),
            Some(payload),
            "写进去的载荷必须原样读回来 ✓"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **② 损坏** ⇒ 当不存在 ✓（**反向验证**：这条就是"第②道有牙"的证明）。
    #[test]
    fn a_corrupted_payload_is_treated_as_absent() {
        let root = temp_root("corrupt");
        write(&root, KEY, "payload-A", &options()).expect("write");
        assert!(read(&root, KEY, &options()).is_some(), "改之前必须读得到");
        // 改**一个字节**（保持长度不变 ⇒ 只有摘要能发现它 ✓）。
        std::fs::write(payload_path(&root, KEY), b"payload-B").expect("corrupt");
        assert_eq!(
            read(&root, KEY, &options()),
            None,
            "载荷变了而凭据没变 ⇒ **必须**当不存在（否则会把坏产物当好的用 ✗）"
        );
        // 截断（长度也变 ⇒ 第②道的另一半）。
        std::fs::write(payload_path(&root, KEY), b"payl").expect("truncate");
        assert_eq!(read(&root, KEY, &options()), None, "截断必须当不存在");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **① 版本 / 构建身份 / prelude 档不符** ⇒ 当不存在 ✓。
    #[test]
    fn stale_identity_is_treated_as_absent() {
        let root = temp_root("stale");
        write(&root, KEY, "payload", &options()).expect("write");
        let good = std::fs::read_to_string(meta_path(&root, KEY)).expect("meta");
        // 逐项改一个字段（每次都要能读回来 ⇒ 恢复原样再改下一个）。
        for (label, patched) in [
            (
                "format",
                good.replace(&format!("\"format\":{ARTIFACT_FORMAT}"), "\"format\":9999"),
            ),
            (
                "front_version",
                good.replace(
                    &format!("\"front_version\":\"{}\"", env!("CARGO_PKG_VERSION")),
                    "\"front_version\":\"0.0.0\"",
                ),
            ),
            (
                "build_stamp",
                good.replace("\"build_stamp\":", "\"build_stamp\":0,"),
            ),
            (
                "target",
                good.replacen(
                    &format!("\"target\":\"{}\"", target()),
                    "\"target\":\"x/y\"",
                    1,
                ),
            ),
            (
                "prelude",
                good.replacen(
                    &format!("\"prelude\":\"{}\"", prelude_name(&options())),
                    "\"prelude\":\"bare\"",
                    1,
                ),
            ),
        ] {
            assert_ne!(patched, good, "补丁 `{label}` 没生效 ⇒ 判据会空转 ✗");
            std::fs::write(meta_path(&root, KEY), &patched).expect("patch meta");
            assert_eq!(
                read(&root, KEY, &options()),
                None,
                "凭据里的 `{label}` 不符本机 ⇒ **必须**当不存在"
            );
        }
        std::fs::write(meta_path(&root, KEY), &good).expect("restore");
        assert!(
            read(&root, KEY, &options()).is_some(),
            "还原之后必须又读得到 ✓"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **③ 键不匹配** ⇒ miss；且**没有凭据**（只有载荷）也 ⇒ miss。
    #[test]
    fn a_foreign_key_and_a_meta_less_payload_are_both_misses() {
        let root = temp_root("foreign");
        write(&root, KEY, "payload", &options()).expect("write");
        assert_eq!(
            read(&root, "closure-fedcba9876543210", &options()),
            None,
            "换一条键 ⇒ 读不到（键即文件名 ⇒ 拿 A 的产物当 B 的用会被挡 ✓）"
        );
        // 只有载荷、没有凭据（模拟"写了一半就崩"）：必须是 miss，**不许**猜着用。
        let bare = temp_root("meta-less");
        std::fs::create_dir_all(dir(&bare)).expect("mkdir");
        std::fs::write(payload_path(&bare, KEY), b"payload").expect("payload");
        assert_eq!(
            read(&bare, KEY, &options()),
            None,
            "没有 `meta.json` ⇒ 完整性无从校验 ⇒ 必须当不存在 ✓"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&bare);
    }

    /// **产物目录必须自忽略**（否则用户的 `git status` 会冒出 `?? .sokonanoda/` ✗）。
    #[test]
    fn writing_an_artifact_keeps_the_module_root_clean_for_git() {
        let root = temp_root("self-ignore");
        write(&root, KEY, "payload", &options()).expect("write");
        let ignore = crate::project::cache::artifacts_dir(&root).join(".gitignore");
        assert_eq!(
            std::fs::read_to_string(&ignore).ok().as_deref(),
            Some("*\n"),
            "产物写路径必须顺手让 `.sokonanoda/` 自忽略（内容与 `compiled/` 侧逐字相同 ✓）"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **键形状守卫**（§8.3 的目录穿越面）：非法键**不写、不读** ✓。
    #[test]
    fn a_malformed_key_is_rejected_before_touching_the_disk() {
        let root = temp_root("malformed");
        for bad in ["../escape", "a/b", "", "x".repeat(129).as_str()] {
            assert!(
                write(&root, bad, "payload", &options()).is_err(),
                "非法键 `{bad:?}` 必须**拒绝写入**（否则就是目录穿越 ✗）"
            );
            assert_eq!(read(&root, bad, &options()), None, "非法键必须读不到");
        }
        assert!(
            !dir(&root).exists() || std::fs::read_dir(dir(&root)).unwrap().next().is_none(),
            "非法键不许在磁盘上留下任何东西 ✓"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
