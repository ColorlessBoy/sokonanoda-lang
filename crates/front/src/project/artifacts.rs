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
    // **原子落盘**（T1-B 批 3）：先写**临时名**再 `rename` —— `rename` 在同一目录里是
    // 原子的 ⇒ 并发读者**要么看到旧的那份、要么看到完整的新那份**，不会看到半份 ✓。
    // ⚠ 即使这里被中断（临时文件残留），完整性三道也会把它当**不存在** ✓
    // （临时名不是 `<key>.bin` ⇒ 根本读不到 ✓）；[`clean_in`] 会把残留一并清掉 ✓。
    let tmp = dir(root).join(format!("{key}.bin.tmp-{}", std::process::id()));
    std::fs::write(&tmp, payload.as_bytes())?;
    std::fs::rename(&tmp, &bin)?;
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

// ───────────────────────────────────────────────────────────────────────────
// **载荷的合体形**（A = 内核环境文本 + B = 前端表文本）—— T1-B 批 2 的接线前件
// ───────────────────────────────────────────────────────────────────────────

/// **C 块：库层那趟的产物**（T1-B 批 2 的第三块）。
///
/// 为什么**必须**存它：消费入口要"**跳过库层 walk**、只走入口自己的命令"，而
/// `run_entries` 把**库层输出与入口输出合并**成入口那份 `CompileOutput`
/// （`on_entry` 收的就是合并结果）⇒ 少了库层这半，`--json` 就**不是逐字节相同** ✗。
///
/// 字段名与 [`crate::project::session::LibCheckpoint`] 一一对应（除 `builder`/`tables`
/// —— 那两块是 A/B ✓）。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LibPassFacts {
    /// 库层那趟的合并输出（`kernel_checks` 已搬好 ✓）
    pub out: crate::compile::CompileOutput,
    /// 库层逐模块报告（**并集顺序** ✓）
    pub reports: Vec<crate::compile::DocumentReport>,
    /// 并集顺序下每个库模块的命令区间 —— `std::ops::Range` 没有 serde 实现
    /// ⇒ 落成 `(start, end)` 二元组 ✓
    pub ranges: Vec<(usize, usize)>,
    /// 库层那趟的命令数（合并输出的偏移量 ✓）
    pub n_commands: usize,
    /// 库层那趟去掉 `import` 行之后的命令数（入口趟的 `judge_prefix_offset` ✓）
    pub prefix_commands: usize,
    /// 库层全部单元拼接之后的**闭包前缀**（A4a ✓）
    pub lib_prefix: String,
}

/// 载荷头部（`<magic> <A 的字节数> <B 的字节数>\n` 然后接 A、B、C 三段）。
///
/// **为什么用一行定长头而不是 JSON 包一层**：A 是几十万字节的文本，
/// 包进 JSON 要整体转义（体积 ×1.2、还要多一次分配 ✗）；一行头 + 两段拼接
/// 既确定又可流式校验 ✓。**格式号在头里** ⇒ 不认的号直接当"没有产物" ✓。
pub const PAYLOAD_MAGIC: &str = "soko.module-artifact/1";

/// `A（[`ExportFile`]） + B（[`PassTables`]）` ⇒ 一份载荷文本。
///
/// 任一段写不出来（A 有本版 writer 不支持的节点 / B 是空的且 …）都返回 `Err`
/// ⇒ 调用方**不写产物**（"失败当不存在"，同设计 §8.1 ✓）。
#[allow(dead_code)]
pub(crate) fn encode_payload(
    env: &sokonanoda::util::ExportFile<'_>,
    tables: &crate::compile::PassTables<'_>,
    facts: &LibPassFacts,
) -> Result<String, String> {
    let a = env.to_ndjson()?;
    let b = crate::project::tables::encode(&tables.known, &tables.defs, &tables.inductives)?;
    let c = serde_json::to_string(facts).map_err(|e| format!("库层产物（C 块）序列化失败：{e}"))?;
    Ok(format!(
        "{PAYLOAD_MAGIC} {} {}\n{a}{b}{c}",
        a.len(),
        b.len()
    ))
}

/// 载荷文本 ⇒ `(A, B)`，**装进调用方给的 arena**（T1-B 的硬约束：
/// "装载必须发生在本趟 pass 自己的 arena 里" ⇒ 这条口子必须收 arena，不能自建 ✓）。
///
/// ⚠ **信任模型（设计 §8.3）**：这里按"**自己人写出来的产物**"解析 ⇒ 传的
/// [`Config`] **放行一切公理**（产物里的 `axiom` 是本地声明的回放，不是外来的）。
/// 真要去信任**下载来**的产物，是 §8.3 的开关 + 清单签名那件事，**不是**本函数的事。
#[allow(dead_code)]
pub(crate) fn decode_payload<'a>(
    arena: &'a stumpalo::ArenaRef<'a>,
    text: &str,
) -> Result<
    (
        sokonanoda::util::ExportFile<'a>,
        crate::compile::PassTables<'a>,
        LibPassFacts,
    ),
    String,
> {
    let (a_text, b_text, c_text) = split_payload(text)?;
    let config = sokonanoda::util::Config {
        unsafe_permit_all_axioms: true,
        unpermitted_axiom_hard_error: false,
        ..sokonanoda::util::Config::default()
    };
    let (env, _skipped) = sokonanoda::parser::parse_export_mapped(arena, a_text.as_bytes(), config)
        .map_err(|e| format!("产物里的内核环境解析失败：{e}"))?;
    let decoded = crate::project::tables::decode(b_text)?;
    let by_name = declaration_index(&env);
    let inductives = crate::project::tables::rehydrate_inductives(&decoded.inductives, |n| {
        by_name.get(n).cloned()
    })?;
    let facts: LibPassFacts =
        serde_json::from_str(c_text).map_err(|e| format!("库层产物（C 块）反序列化失败：{e}"))?;
    Ok((
        env,
        crate::compile::PassTables {
            known: decoded.known,
            inductives,
            defs: decoded.defs,
        },
        facts,
    ))
}

/// 拆头 + 按长度切两段。**任何不自洽都 `Err`**（截断、长度超界、格式号不认识 ✓）。
fn split_payload(text: &str) -> Result<(&str, &str, &str), String> {
    let nl = text
        .find('\n')
        .ok_or_else(|| "产物载荷没有头部".to_string())?;
    let header = &text[..nl];
    let rest = &text[nl + 1..];
    let mut parts = header.split(' ');
    let magic = parts.next().unwrap_or("");
    if magic != PAYLOAD_MAGIC {
        return Err(format!("产物载荷格式号不认识（{magic:?}）"));
    }
    let a_len: usize = parts
        .next()
        .ok_or_else(|| "产物头部缺 A 的长度".to_string())?
        .parse()
        .map_err(|_| "产物头部里 A 的长度不是数字".to_string())?;
    let b_len: usize = parts
        .next()
        .ok_or_else(|| "产物头部缺 B 的长度".to_string())?
        .parse()
        .map_err(|_| "产物头部里 B 的长度不是数字".to_string())?;
    if a_len.saturating_add(b_len) > rest.len() {
        return Err(format!(
            "产物头部声称 A+B 有 {} 字节，实际只剩 {}（截断 ✗）",
            a_len.saturating_add(b_len),
            rest.len()
        ));
    }
    Ok((
        &rest[..a_len],
        &rest[a_len..a_len + b_len],
        &rest[a_len + b_len..],
    ))
}

/// `规范名 → 声明` 表（`rehydrate_inductives` 的口子）。
///
/// **为什么要在 `with_ctx` 里自己渲染名字**：内核没有公开的"`NamePtr` ⇒ 字符串"
/// 自由函数（`pretty_printer` 的那个是私有的、且要 `&mut self`）⇒ 这里用
/// [`sokonanoda::util::TcCtx::read_name`]/`read_string` 递归拼（10 行、与
/// `pretty_printer::name_to_string` **同一套语义**：点分隔、`Anon` 为空 ✓）。
fn declaration_index<'a>(
    env: &sokonanoda::util::ExportFile<'a>,
) -> std::collections::HashMap<String, sokonanoda::env::Declar<'a>> {
    env.with_ctx(|ctx, _cache, _bump| {
        let mut out = std::collections::HashMap::new();
        for (name, declar) in env.declars.iter() {
            out.insert(render_name(ctx, *name), declar.clone());
        }
        out
    })
}

fn render_name(ctx: &sokonanoda::util::TcCtx<'_, '_>, n: sokonanoda::util::NamePtr<'_>) -> String {
    match ctx.read_name(n) {
        sokonanoda::name::Name::Anon => String::new(),
        sokonanoda::name::Name::Str(pfx, sfx, _) => {
            let mut out = render_name(ctx, pfx);
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(ctx.read_string(sfx).as_ref());
            out
        }
        sokonanoda::name::Name::Num(pfx, sfx, _) => {
            let mut out = render_name(ctx, pfx);
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(&sfx.to_string());
            out
        }
    }
}

/// **清掉这个模块根下的模块产物**（保留 `.gitignore`）；返回删除条数。
///
/// **为什么 `clean` 必须带上它**（T1-B 批 3）：`compiled/` 那条已经有过一次同形的
/// 教训（R-3：只清全局 ⇒ `rebuild` 命中项目条目 ⇒ "清空了却什么都没重编"的**假动作** ✗）。
/// 产物是**第三个**存放点 ⇒ 不清它，同一个假动作会在这一层重演 ✗。
///
/// 连 `*.tmp-*` 一起清（原子写的残留 ✓）—— 它们是**垃圾**，留着只会让目录越来越大。
pub fn clean_in(root: &Path) -> usize {
    let d = dir(root);
    let Ok(entries) = std::fs::read_dir(&d) else {
        return 0;
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".gitignore" {
            continue;
        }
        if std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
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
    /// ⭐ **批 2 的合体判据**：`A（内核环境）+ B（前端表）` 写成一份载荷、过磁盘、
    /// **在另一份 arena**（= 另一个进程的等价物）装回来 ⇒ ① 环境**过完整内核检查** ✓、
    /// ② 前端表**逐项相同** ✓。**反向验证**：把载荷截断 ⇒ `Err` ✓。
    ///
    /// 为什么这条是批 2 的核心：跨进程复用的全部难点就是"**指针不许跨进程**"——
    /// A 走结构文本、B 走 wire（只留名字式引用）⇒ 两边都必须在**新 arena** 里重建 ✓。
    #[test]
    fn payload_round_trips_across_arenas() {
        use crate::compile::elab::KnownName;
        use crate::compile::PassTables;
        use sokonanoda::builder::EnvBuilder;
        use sokonanoda::env::Declar;
        use sokonanoda::util::Config;
        use stumpalo::Arena;

        let root = temp_root("payload");

        // ① A：一份**真的**内核环境（两条公理）。
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let p = b.name_from_str("P");
        let zero = b.zero();
        let prop = b.mk_sort(zero);
        let empty = b.alloc_levels_slice(&[]);
        b.add_declar(Declar::Axiom {
            info: sokonanoda::env::DeclarInfo {
                name: p,
                uparams: empty,
                ty: prop,
            },
        })
        .expect("axiom P");
        let a = b.name_from_str("a");
        let p_const = b.mk_const(p, empty);
        b.add_declar(Declar::Axiom {
            info: sokonanoda::env::DeclarInfo {
                name: a,
                uparams: empty,
                ty: p_const,
            },
        })
        .expect("axiom a");
        let env = b.finish();

        // ② B：一小张前端表（`known` 里放一条真声明；`defs`/`inductives` 空）。
        let mut tables = PassTables {
            known: Default::default(),
            inductives: Default::default(),
            defs: Default::default(),
        };
        tables.known.insert(
            "P".to_string(),
            KnownName::Decl {
                universes: vec![],
                implicit_prefix: 0,
                explicit_arity: 0,
                signature: Some("Prop".to_string()),
            },
        );

        // ③ C 块：库层那趟的产物（这里用最小形状 —— 判据要证的是"三段都能原样过 wire"）。
        let facts = LibPassFacts {
            out: crate::compile::CompileOutput::default(),
            reports: vec![crate::compile::DocumentReport::default()],
            ranges: vec![(0, 2), (2, 5)],
            n_commands: 5,
            prefix_commands: 3,
            lib_prefix: "(lib prefix)\n".to_string(),
        };

        // ④ 写 → 磁盘（过完整性三道）→ 读
        let payload = encode_payload(&env, &tables, &facts).expect("encode payload");
        write(&root, KEY, &payload, &options()).expect("write");
        let text = read(&root, KEY, &options()).expect("read（完整性三道全过 ⇒ 才给 Some）");

        // ④ 换一份 arena 装回来
        let arena2 = Arena::new();
        let (env2, tables2, facts2) =
            decode_payload(arena2.as_arena_ref(), &text).expect("decode payload");
        assert_eq!(
            env2.declars.len(),
            env.declars.len(),
            "环境必须一条声明都不少"
        );
        env2.check_all_declars();
        assert_eq!(tables2.known, tables.known, "前端表必须逐项相同");
        // C 块也要原样回来（`DocumentReport` 没有 `PartialEq` ⇒ 比计数 + 其余字段 ✓）。
        assert_eq!(facts2.out, facts.out, "库层输出必须逐项相同");
        assert_eq!(facts2.reports.len(), facts.reports.len(), "逐模块报告条数");
        assert_eq!(facts2.ranges, facts.ranges, "库模块命令区间");
        assert_eq!(facts2.n_commands, facts.n_commands);
        assert_eq!(facts2.prefix_commands, facts.prefix_commands);
        assert_eq!(facts2.lib_prefix, facts.lib_prefix);

        // ⑤ **反向验证**：截断载荷 ⇒ `Err`（不许「猜着用」✗）。
        let arena3 = Arena::new();
        assert!(
            decode_payload(arena3.as_arena_ref(), &text[..text.len() - 5]).is_err(),
            "截断的载荷必须被拒 ✓"
        );
        // 另一条：把头部的长度改大 ⇒ 也要被拒（长度自洽性检查）✓。
        let bumped = text.replacen(
            &format!("{PAYLOAD_MAGIC} "),
            &format!("{PAYLOAD_MAGIC} 999999999 "),
            1,
        );
        assert_ne!(bumped, text, "补丁没生效 ⇒ 判据会空转 ✗");
        let arena4 = Arena::new();
        assert!(
            decode_payload(arena4.as_arena_ref(), &bumped).is_err(),
            "头部声称的长度超过实际 ⇒ 必须被拒 ✓"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **T1-B 批 3 · 原子落盘**：写完**不留**临时文件；残留的 `.tmp-*` **不是**产物 ✓。
    #[test]
    fn writing_is_atomic_and_leaves_no_temp_files() {
        let root = temp_root("atomic");
        write(&root, KEY, "payload", &options()).expect("write");
        let leftovers: Vec<String> = std::fs::read_dir(dir(&root))
            .expect("dir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "原子写不许留临时文件（实得 {leftovers:?}）—— 留了就是垃圾越积越多 ✗"
        );
        // 手造一份残留：它**不是** `<key>.bin` ⇒ 任何键都读不到它 ✓（命名即边界 ✓）。
        std::fs::write(dir(&root).join("stray.bin.tmp-1"), b"junk").expect("stray");
        assert_eq!(read(&root, "stray.bin.tmp-1", &options()), None);
        assert_eq!(read(&root, KEY, &options()).as_deref(), Some("payload"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **T1-B 批 3 · `clean`**：`artifacts::clean_in` 清产物（含 `.tmp-*` 残留）、
    /// **保留**自忽略文件；`cache::clean_at`（`sokonanoda clean` 的项目那一半）
    /// **必须把两处都清** —— 否则 `rebuild` 命中旧产物 ⇒ 与 R-3 同形的
    /// "清空了却什么都没重编"**假动作**会在产物这一层重演 ✗。
    #[test]
    fn clean_removes_artifacts_and_keeps_the_self_ignore() {
        let root = temp_root("clean");
        write(&root, KEY, "payload", &options()).expect("write");
        std::fs::write(dir(&root).join("stray.bin.tmp-1"), b"junk").expect("stray");
        // 再造一份 `compiled/` 条目（`clean_at` 的另一半 ✓）。
        let compiled = crate::project::cache::compiled_at(&root);
        std::fs::create_dir_all(&compiled).expect("mkdir");
        std::fs::write(compiled.join("entry.bin"), b"x").expect("entry");

        let n = crate::project::cache::clean_at(&root);
        assert!(
            n >= 3,
            "`clean_at` 必须把 `compiled/` 的条目 + 产物的 `.bin`/`.meta.json`/残留都清掉（实得 {n}）"
        );
        assert_eq!(
            read(&root, KEY, &options()),
            None,
            "清完之后必须读不到产物 ✓（否则 `rebuild` 会命中旧产物 = 假动作 ✗）"
        );
        assert!(
            crate::project::cache::artifacts_dir(&root)
                .join(".gitignore")
                .exists(),
            "自忽略文件必须留着（清了它，`git status` 会冒出 `?? .sokonanoda/` ✗）"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
