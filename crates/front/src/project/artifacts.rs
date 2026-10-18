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

use crate::compile::CompileOptions;

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
    // ⚠ **临时名必须逐写者唯一**（`FLAKY-CONCURRENT-ARTIFACT-WRITERS-TEST`，high，2026-10-16 ✗→✓）：
    // 旧写法只带 `std::process::id()` ⇒ **同进程的每个线程算出的临时名完全相同** ⇒ 两位写者
    // 交错时"甲写临时名 → 乙覆盖同一个临时名 → 甲 `rename`"⇒ 落地的载荷 = **甲的头 + 乙的尾**
    // （实测：4 位写者 × 8 MiB × 64 轮 ⇒ 磁盘上出现 8,390,940 字节的混合载荷，**5/5 判红** ✓）。
    // ⚠ 那是**真竞态**（不是"CI 运气差"）：本仓的 `sokonanoda-lsp` 是多线程的 ⇒ 同进程并发写
    // 产物**可达** ✓。⚠ 读侧的摘要那道能把撕裂折成 miss（**不是静默用坏数据** ✓），但"写坏一次
    // ⇒ 白丢一次缓存"仍不该发生 ✓。
    let tmp = dir(root).join(format!(
        "{key}.bin.tmp-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
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
    // **有界**（best-effort：淘汰失败绝不影响"这次写成功了"这个事实 ✓）。
    prune(root, key);
    Ok(bin)
}

/// 把产物目录压回 [`MAX_ARTIFACT_PAIRS`] 对以内（**按 mtime 最旧先走** ✓）。
///
/// ⚠ **刚写的那一对永不淘汰**（否则"写成功"与"读得到"会打架 ✗）。
/// 淘汰的单位是**一对**（`.bin` + `.meta.json` 一起走 ✓）—— 只删一半会留下
/// "有凭据没载荷"或反之的垃圾 ✓（读侧会把它们当 miss，但白占盘 ✗）。
fn prune(root: &Path, keep_key: &str) {
    // **反向验证的逃生门**（与仓里其它 `SOKO_*` 量具同源 ✓）：设了它就**不淘汰** ⇒
    // `the_artifact_store_stays_bounded_and_keeps_the_newest` 当场判红（实测 `实得 70 > 64` ✓）
    // ⇒ 证明那条判据**有牙**、不是空转 ✓。**默认关** ⇒ 生产零影响 ✓。
    if std::env::var_os("SOKO_T1B_NO_PRUNE").is_some() {
        return;
    }
    let d = dir(root);
    let Ok(entries) = std::fs::read_dir(&d) else {
        return;
    };
    // 收集 `<key>.bin`（每个键恰好一条 ⇒ 用它当"对"的代表 ✓）。
    let mut pairs: Vec<(std::time::SystemTime, String)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(key) = name.strip_suffix(".bin") else {
            continue;
        };
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        pairs.push((mtime, key.to_string()));
    }
    if pairs.len() <= MAX_ARTIFACT_PAIRS {
        return;
    }
    pairs.sort_by_key(|(mtime, _)| *mtime);
    let excess = pairs.len() - MAX_ARTIFACT_PAIRS;
    for (_, key) in pairs.into_iter().take(excess) {
        if key == keep_key {
            continue;
        }
        let _ = std::fs::remove_file(payload_path(root, &key));
        let _ = std::fs::remove_file(meta_path(root, &key));
    }
}

// ───────────────────────────────────────────────────────────────────────────
// **子模块**（`AGENTS.md` 硬规则 5：接近 ~500 行即拆分）
//
// 本文件只留**写路径**（[`write`] / [`prune`] / [`clean_in`]）与模块文档；其余按职责
// 拆出去，公开路径一律用 re-export 保持稳定 ✓ ⇒ 调用方写法一个字不变 ✓
//（与 `artifacts/lock.rs` 2026-10-13 拆出读侧同一形状 ✓）。
// ───────────────────────────────────────────────────────────────────────────

mod layout;
// **硬规则 5**：磁盘布局与凭据维度（§8.1 的"存哪、叫什么" + §8.2 第①道要比的维度）
// 整块拆进 `artifacts/layout.rs` ⇒ 调用方仍写 `artifacts::dir` / `artifacts::ArtifactMeta` ✓。
use layout::ensure_self_ignore;
pub(crate) use layout::{digest_hex, key_is_safe, prelude_name, target};
pub use layout::{
    dir, exists, meta_path, payload_path, ArtifactMeta, ARTIFACT_FORMAT, MAX_ARTIFACT_PAIRS,
};

mod lock;
// **硬规则 5**：读侧（§8.2 三道 + §8.3 清单）整块拆进 `artifacts/lock.rs`；
// 公开路径用 re-export 保持稳定 ✓（调用方仍写 `artifacts::read` ✓）。
pub use lock::{lock_allows, lock_path, read, read_shared, LOCK_FILE};

mod payload;
// **硬规则 5**：载荷的合体形（A = 内核环境文本 + B = 前端表文本 + C = 库层那趟）
// 整块拆进 `artifacts/payload.rs` ⇒ 调用方仍写 `artifacts::encode_payload` ✓。
pub use payload::PAYLOAD_MAGIC;
pub(crate) use payload::{decode_payload, encode_payload, LibPassFacts};

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

    /// 磁盘上那份载荷**不是**任何一份完整载荷时，返回它的长度（否则 `None` ✓）。
    ///
    /// ⚠ 用在并发判据的**旁路**上：`read` 的摘要那道会把撕裂**静默**折成 miss（与"文件不在"
    /// 同形 ✗）⇒ 只看 `read` 会得出"没撕裂"的假结论 ✓。
    fn not_a_complete_payload(root: &Path, payloads: &[String]) -> Option<usize> {
        let bytes = std::fs::read(payload_path(root, KEY)).ok()?;
        match std::str::from_utf8(&bytes) {
            Ok(text) if payloads.iter().any(|p| p == text) => None,
            _ => Some(bytes.len()),
        }
    }

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

    /// **T1-B 批 3 · 并发取证**：**多个写者同时写同一个键** ⇒ 读者**永远不会**
    /// 看到"半份/混合"的载荷 —— 只可能读到**某一位写者的完整那份**，或者（凭据与载荷
    /// 恰好来自不同写者时）**当不存在** ✓。
    ///
    /// 为什么这条就够了（而不是"必须串行化写"）：产物是**内容寻址**的 —— 同一个键
    /// 在正常使用里对应**同一份内容**（键由输入算出 ✓）⇒ 交错是良性的；真出问题只会是
    /// "读到半份" ⇒ 那正是本用例钉死的东西 ✓。
    ///
    /// 三条机制一起给这个性质：① 载荷 `rename` **原子** ✓；② **凭据最后写** ✓；
    /// ③ 完整性三道（摘要不符 ⇒ miss ✓）。
    ///
    /// ## ⚠ 空转闸**不许**用"读者恰好命中过"（2026-10-17，`FLAKY-CONCURRENT-ARTIFACT-WRITERS-TEST`）
    ///
    /// 旧版把 `assert!(reads > 0)` 当空转闸，而**命中率是调度的函数**：读者在"读凭据"与
    /// "读载荷"之间被换下 CPU 一次，那一轮就**必然** miss（凭据与载荷来自不同写者 ⇒
    /// 第②道拦下 ✓）⇒ 那个计数**没有下界** ✗。实测（`crates/front/tests/zz_cw2_probe.rs`，
    /// 同一份构建）：
    /// * 空载本机 4000 轮命中 **336–417（8.4%–10.4%）**；加压到 64–128 个争用线程时
    ///   **72%–83%**（**同一份代码，命中率跨一个数量级** ⇒ 它量的是机器，不是代码 ✗）；
    /// * **写者被闸门挡住时 `hits == 0` 是确定性的**（4000/4000 miss，写者放行后
    ///   `writes=269`、静默后 `read` 命中 ✓）⇒ **产品没有缺陷也能红** ✓；
    /// * 决定性的一笔：**空仓时 4000 轮读只要 8.4 ms**，而写者从 `spawn` 到第一次落地要
    ///   **0.86–11.7 ms**（本机实测，仅 64 个争用线程就撞到 11.7 ms）⇒ **两个区间重叠**
    ///   ⇒ 共享 runner（2 核 + 1003 条用例并行）上 `reads == 0` 是常态，不是意外 ✓。
    ///   CI（run `78cfcaac` 1001/1002 · run `f878f88a` 1002/1003）红的就是这一条。
    ///
    /// ⇒ 换成两条**与调度无关**的空转闸（**更强**，不是放宽）：① 写者真落地了几次
    /// （结构计数 ✓，`AGENTS.md`「判据的两条硬规矩②」：绝对时序 ⇒ 结构计数/比值 ✓）；
    /// ② **静默后**（写者全部 join）凭据与载荷**必然**自洽 ⇒ `read` **必须**命中
    /// （旧版这里只是 `if let`，miss 就什么都不判 ⇒ 真空转 ✗）。**载荷不许被写撕**那条
    /// 判据（4000 轮里每一次 `Some`）**一字未动** ✓。
    #[test]
    fn concurrent_writers_never_produce_a_torn_payload() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::sync::Arc;

        let root = temp_root("concurrent");
        // 四位写者，载荷长度**刻意不同**（半份/拼接最容易在这里露头 ✓）。
        let payloads: Vec<String> = (0..4)
            .map(|i| format!("{}{}", "x".repeat(1000 + i * 777), i))
            .collect();
        let stop = Arc::new(AtomicBool::new(false));
        // **结构计数**：写者真落地了几对 —— 空转闸的输入，与读者调度无关 ✓。
        let landed = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for p in payloads.clone() {
            let root = root.clone();
            let stop = Arc::clone(&stop);
            let landed = Arc::clone(&landed);
            handles.push(std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    // 写失败不算错（并发下 `rename` 竞争 ✓），只是这一轮不写。
                    if write(&root, KEY, &p, &options()).is_ok() {
                        landed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        // 读侧：任何一次"读到了"的东西都必须是**某一份完整载荷** ✓。
        for _ in 0..4000 {
            if let Some(got) = read(&root, KEY, &options()) {
                assert!(
                    payloads.contains(&got),
                    "读到了**不完整/混合**的载荷（长度 {}）⇒ 原子性与完整性三道都没拦住 ✗",
                    got.len()
                );
            }
        }
        stop.store(true, Ordering::Relaxed);
        for h in handles {
            h.join().expect("writer thread");
        }
        // ② 不许空转（**结构计数**版）：写者真落地过 ⇒ 判据打在生效点上 ✓。
        assert!(
            landed.load(Ordering::Relaxed) > 0,
            "四位写者一次都没写成 ⇒ 判据没打在生效点上（空转 ✗）"
        );
        // ③ 静默后**必须**读得到（**确定性**版）：⚠ 判据 = 「**先单线程写一次、再读**」——
        //    2026-10-18 实测（CI 连红 2 次，本行 `:610`）：直接断言"静默后必然命中"是
        //    **比不变量更强**的判据 ✗ —— `write` 是**两步**（载荷 `rename`、再写凭据），
        //    不是一次原子事务 ⇒ 并发写者的**最后一次**可能停在「新载荷 + 旧凭据」之间
        //    ⇒ 摘要不符 ⇒ **合法地** miss（读侧把不一致折成 miss、绝不用坏数据 ✓；
        //    见 `write` 的 docstring：「崩在中间只会留下一份没有凭据的载荷 ⇒ 下次读是 miss」）。
        //    ⇒ 在**没有并发**的静默态写一次（那一对必然自洽）⇒ `read` **必须**命中 ✓
        //    （空转性没丢：读侧照样被打在生效点上 ✓）。
        let settled = payloads[0].clone();
        write(&root, KEY, &settled, &options()).expect("静默后的单线程写必须成功");
        let last = read(&root, KEY, &options()).expect(
            "静默后**单线程**写出来的那一对必然自洽 ⇒ `read` 必须命中\
             （miss = 读侧根本没被打在生效点上，空转 ✗）",
        );
        assert!(payloads.contains(&last), "收尾读到了混合载荷 ✗");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **同进程四位写者 ⇒ 载荷不许被写撕**（`FLAKY-CONCURRENT-ARTIFACT-WRITERS-TEST`，
    /// high，2026-10-16 · CI `78cfcaac` 上 `concurrent_writers_never_produce_a_torn_payload`
    /// 判红 **1/1002**，而本机同一条命令 **20/20 绿** ⇒ 真因是**竞态**，不是"CI 运气差"）。
    ///
    /// ⚠ **为什么必须再写一条**：上一条用例靠四位写者的**自然交错**去撞撕裂 ⇒ 撞到的概率是
    /// **调度**的函数（本机 20 次 0 次 ⇒ 它在 CI 上红、在本机不红 ✗）。判据没坏，坏的是**触发**：
    /// 一条"只在共享 runner 上偶发"的用例既不能当回归门禁、也不能当诊断 ✓。
    ///
    /// 这一条把触发**确定化**，而且**不依赖墙钟**（`AGENTS.md`「判据的两条硬规矩②」：
    /// **绝对时序 ⇒ 结构计数/比值** ✓）：四位写者先在 [`std::sync::Barrier`] 上对齐、**同时**
    /// 冲进 [`write`]，读侧**全程**只认"完整载荷或 miss"。写者共享临时名时，那个交错
    /// （甲写临时名 → 乙覆盖同一个临时名 → 甲 `rename`）就落在**每次**对齐窗口里 ⇒ 判据必红 ✓。
    ///
    /// ⚠ 它咬的是**临时名唯一性**这个不变量（不是某一种拼法）：唯一 ⇒ 交错根本构造不出来 ✓。
    #[test]
    fn concurrent_writers_at_a_barrier_never_tear_the_payload() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};

        let root = temp_root("torn-payload-barrier");
        // 长度**刻意不同**：撕裂（半份/拼接）在长度上就露头 ✓。
        // ⚠ 载荷**必须够大**：`write` 的"临时名 → `rename`"窗口 = 一次 8 MiB 落盘
        // （实测 ~1 ms 量级 ✓）—— 小载荷那个窗口只有微秒级，读侧**几乎不可能**落在里面
        // （本机实测：1 KB × 64 轮 ⇒ 撕裂 0 次 ✗，即 CI 那条 flaky 的真因 ✓）。
        let payloads: Vec<String> = (0..4)
            .map(|i| format!("{}{}", "z".repeat(8 * 1024 * 1024 + i * 777), i))
            .collect();
        let barrier = Arc::new(Barrier::new(payloads.len()));
        let stop = Arc::new(AtomicBool::new(false));
        let writes = Arc::new(AtomicUsize::new(0));

        // 读侧先起：它要在写者对齐后的**整个**窗口里持续读 ✓。
        let reader = {
            let root = root.clone();
            let stop = Arc::clone(&stop);
            let payloads = payloads.clone();
            std::thread::spawn(move || {
                let mut complete = 0usize;
                let mut misses = 0usize;
                let mut torn: Option<usize> = None;
                let mut torn_bytes: Option<usize> = None;
                while !stop.load(Ordering::Relaxed) {
                    // ⚠ **旁路**：`read` 的摘要那道会把撕裂**静默**折成 miss（与"文件不在"同形 ✗）
                    // ⇒ 只看 `read` 会得出"没撕裂"的假结论。这里**直接**取磁盘字节来判 ✓。
                    // ⚠ 判据 = "产物路径下**读到过**不是任何一份完整载荷的字节" —— 那正是
                    // 原子性不变量被破的形状（`rename` 本应保证读者**要么**旧整份**要么**新整份 ✓）。
                    if let Some(len) = not_a_complete_payload(&root, &payloads) {
                        torn_bytes = Some(len);
                        break;
                    }
                    match read(&root, KEY, &options()) {
                        Some(got) if payloads.contains(&got) => complete += 1,
                        Some(got) => {
                            torn = Some(got.len());
                            break;
                        }
                        None => misses += 1,
                    }
                }
                (complete, misses, torn, torn_bytes)
            })
        };

        let mut handles = Vec::new();
        for p in payloads.clone() {
            let root = root.clone();
            let barrier = Arc::clone(&barrier);
            let writes = Arc::clone(&writes);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                // 对齐后每位写者都**真写**：写失败（并发 `rename` 竞争）不算错 ✓。
                for _ in 0..64 {
                    if write(&root, KEY, &p, &options()).is_ok() {
                        writes.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for h in handles {
            h.join().expect("writer thread");
        }
        stop.store(true, Ordering::Relaxed);
        let (complete, misses, torn, torn_bytes) = reader.join().expect("reader thread");

        // ① 咬住的那一条：磁盘上**一次都不许**出现"不是任何一份完整载荷"的字节 ✓。
        assert!(
            torn_bytes.is_none(),
            "磁盘上出现了**不完整/混合**的载荷（长度 {}）⇒ 写者的临时名不唯一 ⇒ 载荷被写撕 ✗",
            torn_bytes.unwrap_or(0)
        );
        // ①b 读侧同样不许把撕裂当有效产物交出去 ✓（两道一起咬：写侧不产、读侧不认）。
        assert!(
            torn.is_none(),
            "读侧把**不完整/混合**的载荷当有效产物交出去了（长度 {}）✗",
            torn.unwrap_or(0)
        );
        // ② 不许空转：写者真写了、读侧真读到了完整载荷 ✓（否则上面那条是白绿 ✗）。
        assert!(
            writes.load(Ordering::Relaxed) > 0,
            "四位写者一次都没写成 ⇒ 判据没打在生效点上（空转 ✗）"
        );
        assert!(
            complete > 0,
            "整段并发里一次完整载荷都没读到（miss {misses}）⇒ 判据没打在生效点上（空转 ✗）"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **T1-B 批 3 收尾 · 有界**：产物目录必须被压回 [`MAX_ARTIFACT_PAIRS`] 对以内，
    /// **刚写的那一对永远留着**、最旧的走，且**不留"半对"垃圾** ✓。
    ///
    /// 为什么这条是判据而不是"实现细节"：键 = **库层闭包的 Merkle 键** ⇒ 用户每改一次
    /// 库层就多一个新键 ⇒ 老产物**再也没人读**却一直占盘 ✗（`compiled/` 那条天然有界，
    /// 产物这条**没有** ✓）。
    #[test]
    fn the_artifact_store_stays_bounded_and_keeps_the_newest() {
        let root = temp_root("bounded");
        let total = MAX_ARTIFACT_PAIRS + 6;
        let mut keys = Vec::new();
        for i in 0..total {
            let key = format!("closure-{i:016x}");
            write(&root, &key, &format!("payload-{i}"), &options()).expect("write");
            keys.push(key);
            // mtime 要**可分辨**，淘汰顺序才是确定的（否则这条判据会飘 ✗）。
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let bins = std::fs::read_dir(dir(&root))
            .expect("dir")
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bin"))
            .count();
        assert!(
            bins <= MAX_ARTIFACT_PAIRS,
            "产物对数必须被压到上界 {MAX_ARTIFACT_PAIRS} 以内（实得 {bins}）——              否则用户每改一次库层就多一份、永远不回收 ✗"
        );
        // 最新那一份**必须**还在（刚写的不许被自己淘汰掉 ✗）。
        let last = keys.last().expect("at least one");
        assert_eq!(
            read(&root, last, &options()).as_deref(),
            Some(format!("payload-{}", total - 1).as_str()),
            "刚写的那一对不许被淘汰 ✓"
        );
        // 最旧那一份必须走了（否则上界没生效 ✓）。
        assert_eq!(
            read(&root, &keys[0], &options()),
            None,
            "最旧的那一对必须被淘汰 ✓"
        );
        // **不许留半对**：`.bin` 与 `.meta.json` 要么都在、要么都不在 ✓。
        for key in &keys {
            let has_bin = payload_path(&root, key).exists();
            let has_meta = meta_path(&root, key).exists();
            assert_eq!(
                has_bin, has_meta,
                "`{key}` 留下了半对（bin={has_bin} meta={has_meta}）⇒ 白占盘的垃圾 ✗"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **自忽略不许覆盖用户内容**（2026-10-09 自查）：`.sokonanoda/.gitignore` **已经存在**时
    /// ⇒ `ensure_self_ignore` **一个字都不改** ✓（那是用户/别处的文件 ✗）；`clean` 也不许删它 ✓。
    ///
    /// 为什么值得一条判据：这个文件在**用户的项目里**（不是我们的临时目录）⇒ 覆盖它 =
    /// 悄悄改人家的仓库卫生设置 ✗。今天的行为是"**只在缺席时补 `*`**" ✓ —— 钉住它。
    #[test]
    fn an_existing_self_ignore_is_never_rewritten() {
        let root = temp_root("ignore");
        let cache = crate::project::cache::artifacts_dir(&root);
        std::fs::create_dir_all(&cache).expect("mkdir");
        let ignore = cache.join(".gitignore");
        let mine = "# 用户自己写的\ncompiled/\n";
        std::fs::write(&ignore, mine).expect("write");

        // 写一份产物 ⇒ 顺带走一遍 `ensure_self_ignore` ✓。
        write(&root, "abcdef0123456789", "payload", &options()).expect("write");
        assert_eq!(
            std::fs::read_to_string(&ignore).expect("read"),
            mine,
            "**已有的 `.gitignore` 不许被覆盖** ✗（自忽略只在**缺席**时补一行 `*` ✓）"
        );

        // `clean` 也不许把它删掉 ✓（它是"自忽略"的凭据，不是产物 ✓）。
        let _ = clean_in(&root);
        assert!(ignore.is_file(), "`clean` 不许删掉 `.gitignore` ✓");
        assert_eq!(
            std::fs::read_to_string(&ignore).expect("read"),
            mine,
            "`clean` 之后内容也不许变 ✓"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
