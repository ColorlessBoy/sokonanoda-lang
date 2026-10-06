//! Persistent compile cache ("olean"-like).
//!
//! Re-opening an unchanged `.sokonanoda` document reuses the report the kernel
//! already produced for the same (compiler version, build, prelude mode,
//! **metavariable mode**, source text) instead of recompiling from scratch
//! (design `docs/design/compile-cache.md`).
//!
//! ⚠ **凡是会改变编译结果的环境开关都必须进键**（IA-4 M1 实测的教训）：键里少了
//! `SOKO_METAVAR`/`SOKO_NOTATION_METAVAR` 时，同一个缓存目录里**先跑的那一档会污染后面所有档**
//! —— 逃生门 `SOKO_NOTATION_METAVAR=0` 会被静默忽略（release `v0.79.0` 实测复现 ✗）。
//! 新增任何这类开关时，**同轮**把它加进 [`key_parts`]。
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
use crate::project::ProjectReport;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Bump when the on-disk entry schema changes (invalidates old entries).
/// 条目格式版本。**改动键的构成或条目 schema 时必须 bump**——旧条目一律作废。
///
/// `3`（2026-09-21 / T-A02）：`build_stamp` 从"可执行文件 mtime"换成编译期常量。
/// `4`（2026-10-02 / 值守第 8 单）：**报告形状版本 `REPORT_SHAPE` 进键 + 进条目**
/// —— 以前源码没变而二进制变了时会命中旧条目，新字段走 `#[serde(default)]` ⇒
/// **静默给旧答案** ✗（实测：G-78 的修复被整库陈旧缓存挡掉）。现在形状一变整库不命中 ✓。
/// `5`（2026-10-06）：**判定开关档位 `flags_state()` 进键** —— 同一份源码在
/// 不同 `SOKO_*` 档位下的判定互不污染 ✓（b1/u1 跨档污染的根因）。按本常量自己的
/// 规矩「改动键的构成 ⇒ bump」：旧键**够不着**是事实，bump 让这件事**写在条目里**
/// ✓（本地那份 0.81.0 的陈旧条目也一并作废，免得下一轮读到「假中性」✗）。
pub const CACHE_FORMAT: u32 = 5;

/// One cached compile: the document report (for the LSP) and, when the
/// producer computed it, the CLI event output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCompile {
    /// 入口模块的报告（单文件条目就是这份文件的报告）。
    pub report: DocumentReport,
    /// 生产者算出来的事件流（CLI `--json` 回放用；LSP 不需要）。
    pub output: Option<CompileOutput>,
    /// **整份项目报告**（T-A03）：模块表 + 归因。`None` = 单文件条目。
    ///
    /// 为什么必须整份存：LSP 的跨文件能力（`definition` / `references` /
    /// `rename` / `soko/project` 的模块表 / 扇出判定）读的都是
    /// `project_modules()`，而它来自 `ProjectReport`。只存入口报告的话，
    /// 命中缓存的文档会"能显示、不能跳转"——设计 §4.8 写的本来就是"按模块存"，
    /// 实现曾经是"一闭包一条、只存入口"，这里是把它对齐。
    pub project: Option<ProjectReport>,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    format: u32,
    /// **报告形状版本**（`report::REPORT_SHAPE`）—— 与 `format` 分开：`format` 管
    /// "条目结构"，它管"**报告结构**"。**必填**（没有 `#[serde(default)]` ✗）：
    /// 老条目缺这个字段 ⇒ 反序列化直接失败 ⇒ 当 miss 重编 ✓（不许静默读旧答案 ✗）。
    shape: u32,
    report: DocumentReport,
    output: Option<CompileOutput>,
    /// `CACHE_FORMAT` 3 起：项目条目带整份 `ProjectReport`。
    /// ⚠ **必填，故意不给 `#[serde(default)]`**（2026-10-02 值守第 8 单）：老条目
    /// 缺字段 ⇒ 反序列化失败 ⇒ miss 重编 ✓；给默认值就等于"安静地回放旧形状" ✗。
    project: Option<ProjectReport>,
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
/// 换成编译期常量之后，它表达的才是"这份二进制是什么"：`CARGO_PKG_VERSION`
/// （调用方已单独折进键）、profile（debug/release 不串台）、目标平台
/// （`consts::OS` + `consts::ARCH`，跨平台不串台）。开发期想区分"同版本号的
/// 不同构建"时，用**显式环境变量** `SOKO_BUILD_LABEL`（`option_env!` 在编译期
/// 取值），而不是靠文件系统的副作用。
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

/// 判定开关档位在缓存键里的状态：全部 `SOKO_*` 环境变量折成一份 FNV 哈希。
///
/// **2026-10-06 实测修复（CI test 组判红根因）**：缓存键原先只折了
/// `metavar_state`（MetavarMode 三档），**漏掉** `SOKO_ARG_EXPECTED` /
/// `SOKO_UNIVERSE_METAVAR` / `SOKO_JUDGE_INPLACE` / `SOKO_NO_CONST_LEVELS`
/// 等判定开关 ⇒ 同一 src 在开关 A 下跑出的判定被缓存，切到开关 B 仍读旧判定
/// （b1 测试：`b1-off` 的 t3 红被缓存，`b1-on` 复用时 t3 仍红 ✗）。开关换挡
/// 后必须 miss。诊断开关（`SOKO_TRACE_*` 等）一起折：诊断场景本就不该命中
/// 缓存（要跑真实路径）。
///
/// ⚠ **键必须顺序无关**（2026-10-06 复核补）：`std::env::vars()` 的**枚举顺序**
/// 是未指定的 —— 同一组 `SOKO_*` 值、只换环境块的顺序，哈希就不同 ✗（实测：
/// `env -i A=1 B=2` 与 `env -i B=2 A=1` 在同一缓存目录里留下**两个**条目）。
/// 那会让 `sokonanoda build` 预热出来的条目被编辑器（**另一个进程**、另一份
/// 环境顺序）够不着 ✗ —— 正是 G-27 修过的那类「CLI ↔ LSP 复用不了」。
/// ⇒ **先按名字排序再折**（守卫 `flags_hash_ignores_environment_order`）。
///
/// 非 UTF-8 的**值**走 `to_string_lossy`：`vars()` 遇到非 Unicode 会 **panic** ✗，
/// 而这是编译热路径 —— 不许新增崩溃路径。
pub fn flags_state() -> u64 {
    flags_hash(std::env::vars_os().filter_map(|(k, v)| {
        // 名字做 ASCII 前缀判断 ⇒ 非 UTF-8 的名字不可能带 `SOKO_`，跳过即可。
        let k = k.into_string().ok()?;
        k.starts_with("SOKO_")
            .then(|| (k, v.to_string_lossy().into_owned()))
    }))
}

/// [`flags_state`] 的纯函数部分：**排序后**逐个折（分隔符 `0` 不会出现在 UTF-8 里）。
fn flags_hash<I: IntoIterator<Item = (String, String)>>(vars: I) -> u64 {
    let mut pairs: Vec<(String, String)> = vars.into_iter().collect();
    pairs.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for (k, v) in pairs {
        for byte in k.bytes() {
            eat(byte);
        }
        eat(0);
        for byte in v.bytes() {
            eat(byte);
        }
        eat(0);
    }
    hash
}

pub fn key(src: &str, options: &CompileOptions) -> String {
    key_with_build(src, options, build_stamp())
}

/// Stable FNV-1a 64 hex over (CACHE_FORMAT, CARGO_PKG_VERSION, build stamp,
/// prelude mode, **metavariable mode**, **`SOKO_*` flags**, source text).
/// Pure + deterministic.
///
/// [`key`] with an explicit build stamp (pure; used by tests).
pub fn key_with_build(src: &str, options: &CompileOptions, build: u64) -> String {
    key_parts(
        CACHE_FORMAT,
        crate::compile::REPORT_SHAPE,
        env!("CARGO_PKG_VERSION"),
        build,
        options.prelude == PreludeMode::Bare,
        metavar_state(),
        flags_state(),
        src,
    )
}

/// 元变量档位在缓存键里的**状态字节**：`Sibling`（默认档，今天的行为）**保持 0**
/// ⇒ 老缓存继续可用 ✓；另两档各占一个字节 ⇒ 三档互不污染 ✓。
fn metavar_state() -> u8 {
    match super::implicit::metavar_mode() {
        super::implicit::MetavarMode::Sibling => 0,
        super::implicit::MetavarMode::Engine => 1,
        super::implicit::MetavarMode::Off => 2,
    }
}

// 参数多是故意的：缓存键必须显式列尽每一个影响判定的维度（格式、形状、版本、
// build、prelude 形态、元变量开关、SOKO_* 开关、源码），漏一个就是 b1 那类缓存污染。
#[allow(clippy::too_many_arguments)]
fn key_parts(
    format: u32,
    shape: u32,
    version: &str,
    build: u64,
    prelude_bare: bool,
    metavar_state: u8,
    flags_state: u64,
    src: &str,
) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in format.to_le_bytes() {
        eat(byte);
    }
    // **报告形状版本进键**（值守第 8 单）：形状一变，键就变 ⇒ 整库不命中 ✓
    // ——不再依赖"记得 bump CACHE_FORMAT"这种自觉 ✗。
    for byte in shape.to_le_bytes() {
        eat(byte);
    }
    for byte in version.as_bytes() {
        eat(*byte);
    }
    for byte in build.to_le_bytes() {
        eat(byte);
    }
    eat(u8::from(prelude_bare));
    eat(metavar_state);
    for byte in flags_state.to_le_bytes() {
        eat(byte);
    }
    for byte in src.as_bytes() {
        eat(*byte);
    }
    format!("{hash:016x}")
}

/// **逃生门（只给反向验证用）**：`SOKO_CACHE_SHAPE=off` ⇒ 关掉"报告形状版本"这一道
/// 校验 ⇒ 旧形状的条目会被当成**命中**、把旧答案静默回放出来 ✗。
///
/// 为什么留它：用户立的回测铁律要求「撤掉修复 ⇒ 判据必须判红」，而这条修复的"撤掉"
/// = 让形状校验不再拦 ⇒ 只剩这一种表达方式 ✓（见
/// `docs/gaps/repro/G79-cache-shape-version.sh` 的 `expect-red` 段）。生产路径不该设它。
fn shape_check_disabled() -> bool {
    matches!(std::env::var("SOKO_CACHE_SHAPE").as_deref(), Ok("off"))
}

/// Load the entry for `(src, options)`, or `None` on miss/corruption/disabled.
pub fn load(src: &str, options: &CompileOptions) -> Option<CachedCompile> {
    load_in(&compiled_dir()?, &key(src, options))
}

/// 目录化的三个原语（`load_in`/`store_in`/`clean_in`）：**同一个条目格式**，
/// 换一个根目录就是"另一个缓存"。模块根下的 `.sokonanoda/compiled/` 就是靠
/// 这三个复用它（`crate::project::cache`）——不造第二套格式、不造第二个键。
pub(crate) fn load_in(dir: &Path, key: &str) -> Option<CachedCompile> {
    let bytes = std::fs::read(dir.join(format!("{key}.json"))).ok()?;
    let file: CacheFile = serde_json::from_slice(&bytes).ok()?;
    if file.format != CACHE_FORMAT {
        return None;
    }
    // **形状不符 ⇒ miss**（逃生门 `SOKO_CACHE_SHAPE=off` 只给反向验证用 ✗：
    // 关掉之后旧形状条目会被当成命中 ⇒ 正是"修好了但用户看不到"的复现）。
    if file.shape != crate::compile::REPORT_SHAPE && !shape_check_disabled() {
        return None;
    }
    Some(CachedCompile {
        report: file.report,
        output: file.output,
        project: file.project,
    })
}

/// Best-effort persist (atomic: temp file + rename). Never fails the caller.
pub fn store(src: &str, options: &CompileOptions, entry: &CachedCompile) {
    if let Some(dir) = compiled_dir() {
        store_in(&dir, &key(src, options), entry);
    }
}

pub(crate) fn store_in(dir: &Path, key: &str, entry: &CachedCompile) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(&CacheFile {
        format: CACHE_FORMAT,
        shape: crate::compile::REPORT_SHAPE,
        report: entry.report.clone(),
        output: entry.output.clone(),
        project: entry.project.clone(),
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
///
/// **不受 `SOKONANODA_NO_CACHE` 影响**（实测踩到）：`compiled_dir()` 走 `root()`，
/// 而 `NO_CACHE` 让 `root()` 返回 `None` ⇒ 用户"关掉缓存"之后 `--clean` 恒
/// `removed 0`，已有条目**再也清不掉**（只能手动删目录）✗。清理的语义是"把这堆
/// 文件删掉"，与"这次要不要写缓存"无关 ⇒ 这里只认 `SOKONANODA_CACHE_DIR` 与平台默认。
pub fn clean() -> usize {
    clean_dir().map(|dir| clean_in(&dir)).unwrap_or(0)
}

/// `compiled/` 的目录，**故意不看 `NO_CACHE`**（只给清理用）。
fn clean_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SOKONANODA_CACHE_DIR") {
        return Some(PathBuf::from(dir).join("compiled"));
    }
    root_for_clean().map(|root| root.join("compiled"))
}

/// 平台默认缓存根（与 `root()` 的最后一支相同，但忽略 `NO_CACHE`）。
fn root_for_clean() -> Option<PathBuf> {
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

pub(crate) fn clean_in(dir: &Path) -> usize {
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
            project: None,
        }
    }

    use crate::compile::REPORT_SHAPE;

    /// **2026-10-06 复核补的守卫**：开关哈希必须**顺序无关** —— 否则同一个缓存
    /// 目录里，两个环境块顺序不同的进程（`sokonanoda build` 预热 ↔ 编辑器 LSP）
    /// 会永远互相 miss ✗（实测：`env -i A=1 B=2` 与 `env -i B=2 A=1` 留下两条）。
    /// 反向验证：把 `flags_hash` 里的 `pairs.sort()` 撤掉 ⇒ 本测试当场判红 ✓。
    #[test]
    fn flags_hash_ignores_environment_order() {
        let ab = vec![
            ("SOKO_A".to_string(), "1".to_string()),
            ("SOKO_B".to_string(), "2".to_string()),
        ];
        let ba = vec![
            ("SOKO_B".to_string(), "2".to_string()),
            ("SOKO_A".to_string(), "1".to_string()),
        ];
        assert_eq!(flags_hash(ab.clone()), flags_hash(ba), "顺序不许改变键");
        assert_ne!(
            flags_hash(ab.clone()),
            flags_hash(vec![("SOKO_A".to_string(), "1".to_string())]),
            "少一个开关必须 miss"
        );
        assert_ne!(
            flags_hash(ab.clone()),
            flags_hash(vec![
                ("SOKO_A".to_string(), "1".to_string()),
                ("SOKO_B".to_string(), "3".to_string()),
            ]),
            "同一个开关换值必须 miss"
        );
        // 分隔符编码不能把 (A=1, B=2) 与 (A="1\0B", B="2") 混起来。
        assert_ne!(
            flags_hash(ab),
            flags_hash(vec![
                ("SOKO_A".to_string(), "1\u{0}SOKO_B".to_string()),
                ("SOKO_B".to_string(), "2".to_string()),
            ])
        );
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
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, "a"),
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.2.0", 7, false, 0, 0, "a"),
            "a version bump must miss"
        );
        // **IA-4 M1**：元变量档位必须分开（否则同一个缓存目录里先跑的那一档污染后面所有档 ✗）
        for (x, y) in [(0u8, 1u8), (0, 2), (1, 2)] {
            assert_ne!(
                key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, x, 0, "a"),
                key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, y, 0, "a"),
                "不同元变量档位必须是不同的键（state {x} vs {y}）"
            );
        }
        assert_ne!(
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, "a"),
            key_parts(CACHE_FORMAT + 1, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, "a"),
            "a schema bump must miss"
        );
        // **报告形状版本也进键**（值守第 8 单）：形状一变，键必须变 ✓
        // ——否则"源码没变 + 二进制变了"会命中旧条目、静默给旧答案 ✗。
        assert_ne!(
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, "a"),
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 1, "a"),
            "flags 状态变化必须 miss（b1 缓存污染根因的守护）"
        );
    }

    /// **值守第 8 单**：老缓存（形状不对）必须 **miss** —— 不许静默回放旧报告 ✗。
    ///
    /// 为什么单独立一条：这正是用户实测那条通道 ——「源码没变 ⇒ digest 没变 ⇒ 命中 ⇒
    /// 新字段走 `#[serde(default)]` ⇒ 静默给旧答案」⇒ 用户看到的是**修好之前**的样子 ✗。
    /// 判据 = ① 当前形状读得回来 ✓ ② 形状不对 ⇒ `None` ✓ ③ 缺字段（老条目）⇒ `None` ✓。
    #[test]
    fn a_stale_shape_entry_is_a_miss_not_a_silent_old_answer() {
        let dir = tmp_dir("shape");
        let src = "def two : Nat := 2\nexample : Prop := sorry\n";
        let entry = entry_for(src);
        let k = key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.48.0", 7, false, 0, 0, src);
        store_in(&dir, &k, &entry);
        assert!(load_in(&dir, &k).is_some(), "当前形状必须读得回来 ✓");

        let path = dir.join(format!("{k}.json"));
        let raw = std::fs::read(&path).expect("entry file");
        let mut v: serde_json::Value = serde_json::from_slice(&raw).expect("entry json");

        // ② 形状版本不对 ⇒ miss
        v["shape"] = serde_json::json!(0);
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(
            load_in(&dir, &k).is_none(),
            "形状不对的条目必须 miss（不许静默给旧答案）✗"
        );

        // ③ **老条目的真身**：整个 `shape` 字段都没有（写它的二进制还不认识形状版本）
        v.as_object_mut().unwrap().remove("shape");
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(
            load_in(&dir, &k).is_none(),
            "缺 `shape` 字段的老条目必须 miss（`CacheFile.shape` 故意没有 serde 默认值）✗"
        );

        // ④ 键也必须随形状变（形状一变 ⇒ 整库换键 ⇒ 老条目够都够不着）
        assert_ne!(
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.48.0", 7, false, 0, 0, src),
            key_parts(
                CACHE_FORMAT,
                REPORT_SHAPE + 1,
                "0.48.0",
                7,
                false,
                0,
                0,
                src
            ),
        );
        let _ = std::fs::remove_dir_all(&dir);
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
        let k = key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.48.0", 7, false, 0, 0, src);
        store_in(&dir, &k, &entry);
        let loaded = load_in(&dir, &k).expect("cache hit");
        assert_eq!(loaded.report.decls.len(), entry.report.decls.len());
        assert_eq!(loaded.report.decls[0].status, entry.report.decls[0].status);
        assert_eq!(
            loaded.output.map(|o| o.events),
            entry.output.map(|o| o.events)
        );
        let other = key_parts(
            CACHE_FORMAT,
            REPORT_SHAPE,
            "0.48.0",
            7,
            false,
            0,
            0,
            "def x : Nat := 1\n",
        );
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
            project: None,
        };
        store_in(&dir, "a", &entry);
        store_in(&dir, "b", &entry);
        assert_eq!(clean_in(&dir), 2);
        assert!(load_in(&dir, "a").is_none());
        assert_eq!(clean_in(&dir), 0, "a second clean finds nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **T-A03**：项目条目带**整份** `ProjectReport`，往返之后逐字段相等。
    ///
    /// 为什么必须整份：LSP 的跨文件能力（definition/references/rename/
    /// `soko/project` 的模块表/扇出判定）读的都是模块表；只存入口报告的话，
    /// 命中缓存的文档会"能显示、不能跳转"（设计 §4.8 写的本来就是"按模块存"）。
    ///
    /// 用 `store_in`/`load_in`（**带目录参数**）而不是 env 版：同一轮测试里
    /// 并行跑，改 `SOKONANODA_CACHE_DIR` 会互相打架（仓库既有纪律）。
    #[test]
    fn a_project_entry_round_trips_the_whole_report() {
        let dir = tmp_dir("project-roundtrip");
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(
            dir.join("SetLib.sokonanoda"),
            "def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n",
        )
        .expect("write lib");
        let entry = dir.join("Canvas.sokonanoda");
        std::fs::write(
            &entry,
            "import SetLib\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n",
        )
        .expect("write entry");

        let options = CompileOptions::default();
        let plan = crate::project::plan_project(&entry, None, None);
        let digest = plan.digest(&options);
        let project = crate::project::compile_plan(plan, &options);

        let cache_dir = dir.join("entries");
        crate::project::cache::store(&digest, &options, &project);
        // `project::cache::store` 走 env 版；这里改用同一个键的 `store_in` 复核
        // 序列化本身（`store` 的实现就是 `cache::store`，形状完全一致）。
        let entry_module = project.entry_module().expect("entry module").clone();
        let packed = CachedCompile {
            report: entry_module.report.clone(),
            output: Some(entry_module.events.clone()),
            project: Some(project.clone()),
        };
        store_in(&cache_dir, &digest, &packed);
        let loaded = load_in(&cache_dir, &digest).expect("条目必须读得回来");
        let round_tripped = loaded.project.expect("项目条目必须带整份 ProjectReport");

        // 逐字段相等：用 serde_json 的规范形态比（`ProjectReport` 没有 PartialEq）。
        assert_eq!(
            serde_json::to_value(&project).expect("serialize"),
            serde_json::to_value(&round_tripped).expect("serialize"),
            "整份 ProjectReport 必须逐字段往返相等"
        );
        assert!(
            round_tripped.modules.len() >= 2,
            "模块表必须完整（入口 + 依赖），实际 = {}",
            round_tripped.modules.len()
        );
        assert_eq!(
            round_tripped.entry_module().map(|m| m.name.as_str()),
            project.entry_module().map(|m| m.name.as_str()),
            "入口模块必须还是那一个"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 单文件条目的**形状没变**（T-A03 只动项目条目）：`project` 是 `None`。
    #[test]
    fn a_single_file_entry_carries_no_project_report() {
        let dir = tmp_dir("single-shape");
        let packed = CachedCompile {
            report: DocumentReport::default(),
            output: None,
            project: None,
        };
        store_in(&dir, "single", &packed);
        let loaded = load_in(&dir, "single").expect("条目必须读得回来");
        assert!(loaded.project.is_none(), "单文件条目不带项目报告");
        assert!(loaded.output.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
