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
use crate::project::ProjectReport;
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
    report: DocumentReport,
    output: Option<CompileOutput>,
    /// `CACHE_FORMAT` 3 起：项目条目带整份 `ProjectReport`。
    /// `#[serde(default)]` 让**旧的单文件条目**仍然读得进来（那时没有这个字段）。
    #[serde(default)]
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
        project: file.project,
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
            project: None,
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
