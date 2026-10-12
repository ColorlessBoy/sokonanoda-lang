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
//! ⚠ **内建 prelude 是"编译结果的一部分"，但不走 [`key_parts`]**（M-07，2026-10-12）：
//! `prelude/*.sokonanoda` 由 `include_str!` **编进二进制** ⇒ 改一行源就是**另一个构建**，
//! 折进 [`build_stamp`]（内部走 `builtin_prelude_digest`）——见那里的事故与理由。
//! 于是"只改 prelude"不可能与"什么都没改"共用一条键 ✓。
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
/// `6`（2026-10-08 / **E3**）：**prelude 运行时覆盖的指纹进键** ——
/// `SOKO_PRELUDE_DIR` 换了内容而键不变 ⇒ 会命中**旧 prelude** 编出来的条目 ✗
/// （"拿旧答案"的红线）⇒ 按本常量自己的规矩 bump ✓。
/// `7`（2026-10-12 / **M-07**）：**内建 prelude 的摘要进 `build_stamp`**（键里的
/// build 分量不再只是 profile/OS/ARCH）。⚠ 写这一段是为了记下**为什么不动本常量**
/// ✗：内建 prelude 是 `include_str!` 编进二进制的，摘要折进 `build_stamp` 之后
/// **每一次"改了 prelude"的构建都自带一条新键**（旧条目够不着 ✓），而条目 schema
/// 与键里显式列出的各分量**一个都没变** ⇒ 按上面这条规矩**不 bump**（bump 只会
/// 让下一次改 prelude 的构建也顺手整库作废 —— 对一个**已经必然 miss** 的改动是
/// 纯多付的代价 ✗）。判据：`a_different_builtin_prelude_must_change_the_key`
/// ＋ `.cache/ux/fix-a5/VERDICT.md` 的两条行为读数（改 prelude ⇒ 只重建 ⇒ 行为变 ✓）。
/// `7`（2026-10-13 / **G-112**）：**profile 不再进 `build_stamp`**（`key_parts` 里
/// `build` 那一段的**构成**变了 ⇒ 按本常量的规矩 bump ✓）。同一版本号的 release /
/// debug 两个构建以前各有各的戳 ⇒ 同一个模块根下**互不命中、还互相删条目**
/// （用户现场：`compiled/` 里 247 条 release 戳 + 1 条 debug 戳，面板据此**恒**报
/// `stale: true` + 建议 Rebuild，照做也修不好 ✗）。判据：
/// `docs/gaps/repro/G112-two-builds-of-one-version-cannot-share-the-artifact-dir.sh`
/// （0 = 缺口仍在 / 1 = 已修）＋逃生门 `SOKO_CACHE_KEY_KEEP_PROFILE=1`（回到旧行为，
/// 复现件必须重新判红 ✓）。
pub const CACHE_FORMAT: u32 = 7;

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
/// （调用方已单独折进键）、目标平台（`consts::OS` + `consts::ARCH`，跨平台不串台）、
/// **内建 prelude 的摘要**（M-07，见下）。开发期想区分"同版本号的不同构建"时，
/// 用**显式环境变量** `SOKO_BUILD_LABEL`（`option_env!` 在编译期取值），
/// 而不是靠文件系统的副作用。
///
/// ## **G-112（2026-10-13）**：**profile（debug/release）不再是构建身份**
///
/// 这里以前还折了一个 profile 字节（`cfg!(debug_assertions)` ⇒ `d`/`r`），理由是
/// "debug/release 不串台"。实测的代价（用户 2026-10-11 现场）：**同一个版本号的两个
/// 构建**共用一个模块根 `.sokonanoda/`，而戳进了键（[`key_parts`]）**又进了条目文件名**
/// （`project::cache::entry_stem`）⇒ 两边**永远互不命中**，且 `update_index` 的
/// "一个入口一条条目"语义让它们**轮流删掉对方**刚写的条目；面板据此恒报
/// `stale: true` + 建议 Rebuild（用户照做也修不好：换个二进制编一次又 stale ✗）。
///
/// ⇒ 把 profile 从戳里去掉：**它不改变"这份二进制算什么"** —— 判定只看
/// （版本 · 平台 · 内建 prelude · 档位 · 用户源码），而 `cfg!(debug_assertions)`
/// 在本仓**没有任何一处**参与判定（`git grep debug_assertions crates/{front,kernel,cli,lsp}/src`
/// 只命中本文件这一行 ✓）。于是同版本的 release / debug 共用同一份条目 ✓
/// （用户要的正是这个），而"不同版本 / 不同平台 / 改了 prelude / 显式 `SOKO_BUILD_LABEL`"
/// 这些**真正**的构建差异照旧进戳 ✓。
///
/// 逃生门（只给反向验证用）：`SOKO_CACHE_KEY_KEEP_PROFILE=1` ⇒ profile 重新进戳
/// （= 旧行为）⇒ G-112 的复现件必须重新判红 ✓。
///
/// ## **M-07（2026-10-12）**：内建 prelude 的摘要也是构建身份
///
/// 事故（本批实测）：把 `prelude/L1.sokonanoda` 的 `And.left` 从 `theorem` 改回
/// `def`、**只重建不清缓存** ⇒ 旧条目被复用、`#print And.left` 继续答 `theorem` ✗。
/// 根因就是这里：prelude 是 `include_str!` **编进二进制**的
/// （[`super::prelude::PRELUDE_EQ_SRC`] / `PRELUDE_L1_SRC` / `QUOT_TYPES_SRC` +
/// 两张名字表），而本函数当时只折 profile/OS/ARCH ⇒
/// **"同版本、同档位、只动 prelude"= 键完全不变** ✗ ——
/// 正是"源码没变而行为变了却命中旧答案"那条红线，只是变的是**内建源**不是用户源。
///
/// ⇒ 折进 [`builtin_prelude_digest`]（**编译期常量**算出来的纯函数值 ✓）。
/// 于是"改一行 prelude"= **另一个构建** ⇒ 全部缓存键（含模块层产物键，
/// `project::ProjectPlan::module_keys` 走的也是 [`key`]）一起变 ⇒ 必 miss ✓。
///
/// **为什么不在这里读 [`super::prelude::prelude_source`]**：那份可以带
/// `SOKO_PRELUDE_DIR` 的**运行时**覆盖（E3）——运行时状态进 `build_stamp` 会破坏
/// "纯函数、跨进程稳定"这条前提（同一个进程内先问一次键、再读一次覆盖，值就变了 ✗），
/// 而且覆盖本来就有自己的键分量（[`super::prelude::prelude_override_state`] ✓）。
/// 这里只要**内建**那三段的字节 + 两张名字表。
pub fn build_stamp() -> u64 {
    stamp_with_prelude_digest(builtin_prelude_digest())
}

/// [`build_stamp`] 的**纯函数**那一半（可单测 ✓）：`(OS, ARCH, prelude 摘要[, label][, profile])`。
///
/// 单独拆出来是为了让"prelude 摘要确实折进去了"有**可执行的判据** ——
/// 传两个不同的摘要必须得到两个不同的 stamp（否则就是在自证空转 ✗）。
fn stamp_with_prelude_digest(prelude_digest: u64) -> u64 {
    stamp_of(prelude_digest, keep_profile_in_stamp())
}

/// **逃生门（只给反向验证用）**：`SOKO_CACHE_KEY_KEEP_PROFILE=1` ⇒ profile 重新进
/// `build_stamp`（= 2026-10-13 之前的行为）⇒ 同一版本的两个构建又互不命中
/// （G-112 的复现件必须重新判红 ✓）。
///
/// 极性说明：**新**行为是"profile **不**进键"，所以逃生门说的是"**保留** profile"。
/// ⚠ 它自己也以 `SOKO_` 开头 ⇒ 被 [`flags_state`] 折进键 ⇒ 开着它跑出来的条目与
/// 关着它跑出来的**永不互命中**（不会互相污染 ✓）。
fn keep_profile_in_stamp() -> bool {
    matches!(
        std::env::var("SOKO_CACHE_KEY_KEEP_PROFILE").as_deref(),
        Ok("1")
    )
}

/// [`stamp_with_prelude_digest`] 的**纯函数**内核（`keep_profile` 是显式形参 ⇒
/// 两个 profile 都能在**同一个**测试进程里判 ✓，见
/// `the_build_stamp_no_longer_folds_the_profile`）。
fn stamp_of(prelude_digest: u64, keep_profile: bool) -> u64 {
    // FNV-1a 64 over the compile-time identity string. 纯函数、跨进程稳定。
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    // `PROFILE` / `TARGET` 是 **build script** 的环境变量，库 crate 里取不到
    // （`env!` 直接编译失败）。用同样是编译期常量的替代：
    // `std::env::consts::{OS, ARCH}`（跨平台不串台）。
    //
    // **G-112**：profile（`cfg!(debug_assertions)`）**不进戳** —— 它不改变"这份
    // 二进制算什么"（判定路径里没有一处读它 ✓），却让同版本的两个构建互相擦条目 ✗。
    // 逃生门开着时才回到旧行为（见 [`keep_profile_in_stamp`]）。
    if keep_profile {
        eat(if cfg!(debug_assertions) { b'd' } else { b'r' });
    }
    for byte in std::env::consts::OS.bytes() {
        eat(byte);
    }
    eat(b'/');
    for byte in std::env::consts::ARCH.bytes() {
        eat(byte);
    }
    // **M-07**：内建 prelude 的摘要（排在 label 之前 —— label 是可选尾巴，
    // 加它的人要的是"再区分一层"，不该改变下面这个必需分量的位置 ✓）。
    eat(b'/');
    for byte in prelude_digest.to_le_bytes() {
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

/// **M-07**：**内建** prelude 的摘要（FNV-1a 64）—— 编进这份二进制的那些
/// `include_str!` 源 + 两张名字表。
///
/// 覆盖哪些东西的判据是**"改了它 ⇒ 同一份用户源码的判定/显示会变"**：
/// * `PRELUDE_EQ_SRC` / `PRELUDE_L1_SRC` / `QUOT_TYPES_SRC`：真装进内核的源 ✓
///   （`And.left` 是 `theorem` 还是 `def` 就是这里的一行）；
/// * `PRELUDE_NAMES` / `PRELUDE_NEVER_YIELDS`：补全/悬停的**材料**表（LSP 的
///   `PRELUDE_NAMES` 回退直接把它当"这里有定义"讲给用户 ✓）—— 漂了同样是
///   "改了行为却答旧答案"。
///
/// ⚠ **顺序与分隔符是契约**：名字表用 `\0` 逐个隔开（`["ab","c"]` 与 `["a","bc"]`
/// 不许撞车），三段源之间用 `\0`，末尾不再补 —— 与
/// [`super::prelude::prelude_source`] 的合并视图**不要求逐字相同**（那份是给
/// 编辑器 F12 用的），这里只要求**稳定 + 敏感** ✓。
fn builtin_prelude_digest() -> u64 {
    use super::prelude::{
        PRELUDE_EQ_SRC, PRELUDE_L1_SRC, PRELUDE_NAMES, PRELUDE_NEVER_YIELDS, QUOT_TYPES_SRC,
    };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for text in [PRELUDE_EQ_SRC, PRELUDE_L1_SRC, QUOT_TYPES_SRC] {
        for byte in text.bytes() {
            eat(byte);
        }
        eat(0);
    }
    for names in [PRELUDE_NAMES, PRELUDE_NEVER_YIELDS] {
        for name in names {
            for byte in name.bytes() {
                eat(byte);
            }
            eat(0);
        }
        // 两张表之间再加一个 `\0`（空表与"少一张表"不许撞车 ✓）。
        eat(0);
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

/// **E3**：覆盖内容的指纹（FNV-1a 64；**只给缓存键用** ✓）。
pub fn prelude_override_hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
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
        // **E3**：覆盖指纹（`0` = 没覆盖 ⇒ 键与今天逐字相同 ✓）。
        super::prelude::prelude_override_state(),
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
    prelude_override: u64,
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
    // **E3**：prelude 覆盖的指纹进键（`0` = 没覆盖 ⇒ 与今天逐字相同 ✓）。
    for byte in prelude_override.to_le_bytes() {
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
    // **回放闸门**（2026-10-10 · G-102 的入口侧）：条目里的报告可能带着
    // "同一条命令两份结果"（历史产物、或将来某个新生产者的口子）。这里
    // **进场前收干净**，下游（LSP 的 `set_cached_entry`、CLI 的 `query`/`check`/
    // `build`）拿到的都是单份 ⇒ 回放不可能再把重复装回内存、再靠拼接 +1 ✗。
    //
    // ⚠ 只清 `checks`/`prints`/`decls`（**来源身份**重复），不动 `hovers`
    // （同一条命令的多个使用处是合法的）——见 `DocumentReport::drop_replayed_duplicates`。
    let mut report = file.report;
    report.drop_replayed_duplicates();
    let project = file.project.map(|mut project| {
        project.sanitize_replayed_outputs();
        project
    });
    Some(CachedCompile {
        report,
        output: file.output,
        project,
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
    /// **M-07 判据 ②（键那一半）**：**内建 prelude 的摘要必须折进键** ⇒ 同一份源码、
    /// 同 profile/OS/ARCH、**只换 prelude** ⇒ 键**必须不同** ✓。
    ///
    /// **为什么必须有这条**（2026-10-12 实测咬过一次）：改一行
    /// `prelude/L1.sokonanoda`、只重建不清缓存 ⇒ 旧条目被复用、`#print And.left`
    /// 继续答 `theorem` ✗ —— 正是"修好了但用户看到旧的"那条红线。
    /// 键 = `…, build, …`，而 `build = stamp_with_prelude_digest(内建 prelude 摘要)`
    /// ⇒ 这条测试把两半都咬住：只改**摘要**（build 分量）就得换键；只改**用户源码**
    /// 也得换键（下面第二段，反过来证明"换键"不是无条件成立 ✗）。
    ///
    /// 反向验证（**咬得住**）：把 `key_with_build` 里的 `build` 那一段从
    /// `key_parts` 删掉 ⇒ 第一段当场判红 ✓。
    #[test]
    fn a_different_builtin_prelude_must_change_the_key() {
        let src = "import lib.Set\n\n#print And.left\n";
        let options = CompileOptions::default();
        let old_prelude = stamp_with_prelude_digest(0x1111_2222_3333_4444);
        let new_prelude = stamp_with_prelude_digest(0x1111_2222_3333_4445); // 只差 1 bit
        assert_ne!(
            old_prelude, new_prelude,
            "prelude 摘要必须真的参与 stamp（否则这一条在自证空转 ✗）"
        );
        // 同 profile/OS/ARCH、同 prelude 档位、同源码：**只** prelude 摘要不同。
        assert_ne!(
            key_with_build(src, &options, old_prelude),
            key_with_build(src, &options, new_prelude),
            "换了内建 prelude ⇒ 缓存键必须不同（否则旧条目会答旧行为 ✗）"
        );
        // 反过来：同一份 prelude 摘要 ⇒ 键逐字相同（纯函数、跨进程稳定 ✓）。
        assert_eq!(
            key_with_build(src, &options, old_prelude),
            key_with_build(src, &options, old_prelude)
        );
    }

    /// **G-112 判据（纯函数那一半）**：`build_stamp` **不再折 profile**，但仍对
    /// （OS/ARCH · 内建 prelude 摘要 · `SOKO_BUILD_LABEL`）敏感。
    ///
    /// **为什么这条测试是"可判据"的**：一个测试进程只能是**一个** profile
    /// （`cfg!(debug_assertions)` 是编译期常量）⇒ "release 与 debug 同戳"这件事
    /// 在单测里**测不到**；能测的是**机制**：`keep_profile = true/false` 必须给出
    /// **两个不同的戳**（否则逃生门是空转 ✗），而 `false`（= 默认路径）那一份
    /// 对 prelude 摘要仍然敏感（M-07 不许被这次改动弄丢 ✓）。
    ///
    /// 端到端判据（两个真构建）：
    /// `docs/gaps/repro/G112-two-builds-of-one-version-cannot-share-the-artifact-dir.sh`
    /// ⇒ 0 = 缺口仍在 / 1 = 已修。
    #[test]
    fn the_build_stamp_no_longer_folds_the_profile() {
        let digest = 0x1111_2222_3333_4444;
        assert_ne!(
            stamp_of(digest, true),
            stamp_of(digest, false),
            "逃生门（keep_profile）必须真的改变戳 —— 否则它挡不住任何东西 ✗"
        );
        assert_eq!(
            stamp_of(digest, false),
            stamp_of(digest, false),
            "默认那一份必须是纯函数（两个 profile 的构建会算出同一个戳 ✓）"
        );
        assert_ne!(
            stamp_of(digest, false),
            stamp_of(digest ^ 1, false),
            "去掉 profile 之后，prelude 摘要**仍然**必须参与戳（M-07 的红线 ✓）"
        );
        assert_eq!(
            stamp_of(digest, false),
            stamp_with_prelude_digest(digest),
            "默认路径（没设 SOKO_CACHE_KEY_KEEP_PROFILE）走的必须是不带 profile 那一份"
        );
    }

    /// **M-07**：`builtin_prelude_digest` 的几个**可判据**的性质 ——
    /// 非退化（不是 0、也不是纯 FNV offset）、且**对内容敏感**。
    ///
    /// ⚠ 这里**不**复算"真源的 FNV 应该等于多少"（那与实现同源、等于自证 ✓）；
    /// "改了真源 ⇒ 摘要必变"由端到端判据钉住（改 `prelude/L1.sokonanoda` 一行 ⇒
    /// 重建 ⇒ 键变 ⇒ 行为变，见 `.cache/ux/fix-a5/VERDICT.md`）。
    /// 这里钉的是**纯函数那半边**：分隔符/顺序不许把不同输入折成同一份。
    #[test]
    fn the_builtin_prelude_digest_is_content_sensitive() {
        let digest = builtin_prelude_digest();
        assert_ne!(digest, 0, "退化值会把 prelude 分量抹掉 ✗");
        assert_ne!(digest, 0xcbf2_9ce4_8422_2325, "不能等于 FNV offset basis");
        assert_eq!(digest, builtin_prelude_digest(), "纯函数 ⇒ 两次相同 ✓");
        // 名字表必须**分隔**着折（`["ab","c"]` 与 `["a","bc"]` 不许撞车）——
        // 这条用同一函数的两张真表间接证明不了，直接按同一条公式验编码方式。
        let join = |names: &[&str]| {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            let mut eat = |byte: u8| {
                h ^= u64::from(byte);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            };
            for name in names {
                for byte in name.bytes() {
                    eat(byte);
                }
                eat(0);
            }
            h
        };
        assert_ne!(
            join(&["ab", "c"]),
            join(&["a", "bc"]),
            "名字表必须带分隔符（否则两张不同的表会折成同一个摘要 ✗）"
        );
    }

    /// **E3 判据 ② 的键那一半**：prelude 覆盖的指纹**必须进键** ⇒
    /// 换了覆盖内容 ⇒ 键不同 ⇒ 缓存**必 miss** ✓（否则会拿旧 prelude 编出来的条目 ✗）。
    #[test]
    fn the_prelude_override_fingerprint_enters_the_key() {
        let base = key_parts(6, 3, "0.0.0", 7, false, 0, 0, 0, "def x : Prop := Prop\n");
        let other = key_parts(
            6,
            3,
            "0.0.0",
            7,
            false,
            0,
            0,
            0xdead_beef,
            "def x : Prop := Prop\n",
        );
        assert_ne!(base, other, "覆盖指纹必须进键 ✓");
        // 没覆盖（0）与**今天**的键逐字相同 —— 用同一批参数算两次自证纯函数 ✓。
        assert_eq!(
            base,
            key_parts(6, 3, "0.0.0", 7, false, 0, 0, 0, "def x : Prop := Prop\n")
        );
    }

    use super::*;
    use crate::Span;

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
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, 0, "a"),
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.2.0", 7, false, 0, 0, 0, "a"),
            "a version bump must miss"
        );
        // **IA-4 M1**：元变量档位必须分开（否则同一个缓存目录里先跑的那一档污染后面所有档 ✗）
        for (x, y) in [(0u8, 1u8), (0, 2), (1, 2)] {
            assert_ne!(
                key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, x, 0, 0, "a"),
                key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, y, 0, 0, "a"),
                "不同元变量档位必须是不同的键（state {x} vs {y}）"
            );
        }
        assert_ne!(
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, 0, "a"),
            key_parts(
                CACHE_FORMAT + 1,
                REPORT_SHAPE,
                "0.1.0",
                7,
                false,
                0,
                0,
                0,
                "a"
            ),
            "a schema bump must miss"
        );
        // **报告形状版本也进键**（值守第 8 单）：形状一变，键必须变 ✓
        // ——否则"源码没变 + 二进制变了"会命中旧条目、静默给旧答案 ✗。
        assert_ne!(
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 0, 0, "a"),
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.1.0", 7, false, 0, 1, 0, "a"),
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
        let k = key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.48.0", 7, false, 0, 0, 0, src);
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
            key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.48.0", 7, false, 0, 0, 0, src),
            key_parts(
                CACHE_FORMAT,
                REPORT_SHAPE + 1,
                "0.48.0",
                7,
                false,
                0,
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
        let k = key_parts(CACHE_FORMAT, REPORT_SHAPE, "0.48.0", 7, false, 0, 0, 0, src);
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

    /// **回放闸门**（2026-10-10 · G-102 的入口侧）：条目里"同一条命令多份结果"
    /// 必须在**离开缓存之前**收干净。
    ///
    /// 现场物证就是磁盘上这种形状（用户机器上 `compiled/*.json` 里同一条
    /// `#check` 躺着 3 份、逐字节相同）：回放 ⇒ 内存里 3 份 ⇒ 下一次编辑拼接
    /// 再 +1 ⇒ **每编辑一次多一份**，一路到 19 份。这条判据钉的是"回放这一侧"
    /// ——**撤掉 `load_in` 里那句 `drop_replayed_duplicates()` 当场判红** ✓。
    ///
    /// 顺带钉住"干净条目进场后逐字段不变"（闸门对正常条目是恒等变换 ⇒
    /// 回放语义零变化 ✓）。
    #[test]
    fn a_replayed_entry_is_sanitised_before_it_leaves_the_cache() {
        let dir = tmp_dir("replay-sanitise");
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let clean = CachedCompile {
            report: DocumentReport {
                checks: vec![crate::compile::CheckInfo {
                    span: Span::default(),
                    text: "lib_id : Nat -> Nat".to_string(),
                    cmd: 3,
                }],
                prints: vec![crate::compile::PrintInfo {
                    span: Span::default(),
                    name: "lib_id".to_string(),
                    text: "def lib_id : Nat -> Nat := fun (n : Nat) => n".to_string(),
                    cmd: 4,
                }],
                ..DocumentReport::default()
            },
            output: None,
            project: None,
        };
        store_in(&dir, "dup", &clean);

        // 干净的条目：读回来**逐字段相同**（闸门是恒等变换）。
        let same = load_in(&dir, "dup").expect("条目必须读得回来");
        assert_eq!(
            serde_json::to_value(&same.report).expect("serialize"),
            serde_json::to_value(&clean.report).expect("serialize"),
            "没有重复的条目进场后必须一个字节都不变"
        );

        // 往磁盘上注入重复 = 伪造"历史产物/坏产物"（正是用户机器上那份的形状）。
        let path = dir.join("dup.json");
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read entry")).expect("entry json");
        for _ in 0..2 {
            let clone = json["report"]["checks"][0].clone();
            json["report"]["checks"]
                .as_array_mut()
                .expect("checks array")
                .push(clone);
            let clone = json["report"]["prints"][0].clone();
            json["report"]["prints"]
                .as_array_mut()
                .expect("prints array")
                .push(clone);
        }
        std::fs::write(&path, serde_json::to_vec(&json).expect("serialize")).expect("write entry");
        // 前提：磁盘上**确实**是 3 份（否则这条判据会空转 ✗）。
        let raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read entry")).expect("entry json");
        assert_eq!(
            raw["report"]["checks"].as_array().map(Vec::len),
            Some(3),
            "注入失败：磁盘上必须真的有三份 `#check` 结果"
        );

        let loaded = load_in(&dir, "dup").expect("坏条目也要读得回来");
        assert_eq!(
            loaded.report.checks.len(),
            1,
            "回放前必须收成一份（修前 = 3 份 ⇒ 回放后每编辑一次 +1）: {:?}",
            loaded.report.checks
        );
        assert_eq!(loaded.report.prints.len(), 1, "`#print` 同一条纪律");
        assert_eq!(loaded.report.checks[0].cmd, 3, "留下的是原结论，不是新猜的");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **同类横向排查**（`AGENTS.md` 验证纪律 0(b)）：「整份报告被追加一遍」这类
    /// 坏条目不会只重复 `#check`/`#print` —— 声明卡片同样会重复。这里把同一道
    /// 闸门对 `decls` 也钉住（键 = `(cmd, span)`；inductive 块的一条命令多个构造子
    /// ⇒ span 必须一起进键，别把合法内容当重复删 ✗）。
    #[test]
    fn the_replay_gate_keeps_distinct_declarations_and_drops_copies() {
        let mut report = DocumentReport {
            decls: vec![
                decl(1, 10, 20, Some("t1")),
                // 同一条命令的**另一个构造子**（span 不同）⇒ 必须留下。
                decl(1, 30, 40, Some("t1.mk")),
                decl(1, 10, 20, Some("t1")),
            ],
            ..DocumentReport::default()
        };
        report.drop_replayed_duplicates();
        let kept: Vec<(usize, usize)> = report
            .decls
            .iter()
            .map(|d| (d.cmd, d.span.start.offset))
            .collect();
        assert_eq!(
            kept,
            vec![(1, 10), (1, 30)],
            "同一个 (cmd, span) 的复制品要删、同命令不同位置的声明要留"
        );
    }

    fn decl(cmd: usize, start: usize, end: usize, name: Option<&str>) -> crate::compile::DeclState {
        let mut span = Span::default();
        span.start.offset = start;
        span.end.offset = end;
        crate::compile::DeclState {
            kind: crate::compile::DeclKind::Theorem,
            name: name.map(str::to_string),
            span,
            status: crate::compile::DeclStatus::Checked,
            error: None,
            goal: None,
            by_root: None,
            binders: Vec::new(),
            cmd,
            universe: Vec::new(),
            holes: Vec::new(),
            hole_modes: Vec::new(),
            sub_goals: Vec::new(),
            ty_text: None,
            val_text: None,
            refine_template: None,
            hints: Vec::new(),
            by_steps: Vec::new(),
        }
    }
}
