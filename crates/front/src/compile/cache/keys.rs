//! **缓存键的计算**（2026-10-13 从 `cache.rs` 拆出来 —— `AGENTS.md` 硬规则 5
//! 「文件接近 ~500 行即拆分」，且判据 ⑨（`STRUCT-SRC-LINES`）对 `cache.rs` 是
//! **只许减不许增** ✓）。
//!
//! 这里只有**两把键**与它们**共用的输入**：
//! * [`key`] / [`key_with_build`] —— **检查点键**：`SOKO_*` 开关档位进键（b1 那条 CI
//!   判红换来的语义 ✓）+ 报告形状进键；
//! * [`content_key`] / [`content_key_with_build`] —— **§3 的内容寻址键**
//!   （`docs/design/module-artifacts.md` §3/§8.1）：`SOKO_*` **不进键** ⇒ 同一份源文本
//!   在**任何机器、任何环境**上算出**同一个键** ⇒ 产物可共享/可下载 ✓。
//!
//! 两条链共用 [`key_parts`]（**单一真相** ✓）—— 区别只有**折不折 `flags_state`** 一处 ✓。
//! 公开路径不变：`cache.rs` 用 `pub use` re-export（硬规则 5 ✓）。

use super::{build_stamp, CACHE_FORMAT};
use crate::compile::prelude::{CompileOptions, PreludeMode};
use crate::compile::Dialect;

/// 判定开关档位在缓存键里的状态：全部 `SOKO_*` 环境变量 + **方言**折成一份 FNV 哈希。
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
///
/// ⚠ **方言必须由实参进来，不许只靠环境**（S2，`docs/design/dialect-config.md` §3.3）：
/// 今天 `SOKO_STRICT_AT` 恰好也是 `SOKO_*` ⇒ 顺手被折了；但 S3+ 的来源是 CLI 旗标 /
/// `sokonanoda.toml` / 编辑器设置，**不经过环境** ⇒ 只认环境的键会让两档在同一个
/// `<模块根>/.sokonanoda/` 里**互相命中**（宽松档算出来的"接受"被严格档回放 ✗）。
/// ⇒ 方言取自 [`CompileOptions::dialect`]（[`dialect_pair`]，**与来源无关** ✓），
/// 守卫 = `crates/front/tests/dialect_cache_keys.rs`（环境变量那半是假绿：它本来就在键里）。
pub fn flags_state(dialect: Dialect) -> u64 {
    flags_hash(
        std::env::vars_os()
            .filter_map(|(k, v)| {
                // 名字做 ASCII 前缀判断 ⇒ 非 UTF-8 的名字不可能带 `SOKO_`，跳过即可。
                let k = k.into_string().ok()?;
                k.starts_with("SOKO_")
                    .then(|| (k, v.to_string_lossy().into_owned()))
            })
            .chain(std::iter::once(dialect_pair(dialect))),
    )
}

/// 方言折进 [`flags_state`] 的**合成键对**：名字不与任何真实环境变量重名
/// （`SOKO_*` 的过滤在上面 ⇒ 它只能从这里来 ✓），两态各一个取值 ⇒ 互斥 ✓。
///
/// 用穷尽 `match`（不是 `if dialect.strict()`）：将来加第三态时**编译器**会逼着这里
/// 表态，而不是把它悄悄折成宽松档的键（那正是本段要防的串味 ✗）。
fn dialect_pair(dialect: Dialect) -> (String, String) {
    let value = match dialect {
        Dialect::Strict => "strict",
        Dialect::Loose => "loose",
    };
    ("dialect".to_string(), value.to_string())
}

/// [`flags_state`] 的纯函数部分：**排序后**逐个折（分隔符 `0` 不会出现在 UTF-8 里）。
pub(crate) fn flags_hash<I: IntoIterator<Item = (String, String)>>(vars: I) -> u64 {
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
/// prelude mode, **metavariable mode**, **`SOKO_*` flags**, **dialect**, source text).
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
        flags_state(options.dialect),
        // **E3**：覆盖指纹（`0` = 没覆盖 ⇒ 键与今天逐字相同 ✓）。
        crate::compile::prelude::prelude_override_state(),
        src,
    )
}

/// 元变量档位在缓存键里的**状态字节**：`Sibling`（默认档，今天的行为）**保持 0**
/// ⇒ 老缓存继续可用 ✓；另两档各占一个字节 ⇒ 三档互不污染 ✓。
///
/// **HO3b（2026-10-26）**：再折两**位**（`0b0100` = `SOKO_META_PATTERN` 开、
/// `0b1000` = `SOKO_META_APPROX` 开）——**默认 0 ⇒ 键逐字节不变** ✓。
/// 为什么必须进键：这两档会**改判定面**（近似档给更一般的解 / 更少的拒绝）⇒
/// 少了它们，先跑那一档的检出会被另一档原样回放（正是 b1 那类缓存污染 ✗）。
/// 为什么挂在 `metavar_state` 而不是 `flags_state`：内容寻址键**不折 `SOKO_*`**
/// （`content_key_with_build` 那一侧传 `0`）⇒ 只有挂在这里，共享产物才既**可共享**
/// （默认档恒 0 ✓）又**不跨档串味** ✓。
fn metavar_state() -> u8 {
    pattern_bits(
        crate::compile::meta::pattern::enabled(),
        crate::compile::meta::pattern::approx_enabled(),
    ) | match crate::compile::implicit::metavar_mode() {
        crate::compile::implicit::MetavarMode::Sibling => 0,
        crate::compile::implicit::MetavarMode::Engine => 1,
        crate::compile::implicit::MetavarMode::Off => 2,
    }
}

/// [`metavar_state`] 的**纯函数**部分（两档开关 ⇒ 两位；**默认 0**）。
///
/// 单列出来是为了**可判据**：环境变量在进程内只读一次、单测改不了它
/// （`scripts/check-test-env-isolation.py` 的纪律）⇒ 这条纯函数把"位怎么算"
/// 变成能断言的东西，运行期的档位由 `.json` 三指纹（`crates/cli/tests/meta_pattern.rs`）钉。
fn pattern_bits(pattern: bool, approx: bool) -> u8 {
    (u8::from(pattern) << 2) | (u8::from(approx) << 3)
}

// 参数多是故意的：缓存键必须显式列尽每一个影响判定的维度（格式、形状、版本、
// build、prelude 形态、元变量开关、SOKO_* 开关、源码），漏一个就是 b1 那类缓存污染。
#[allow(clippy::too_many_arguments)]
pub(crate) fn key_parts(
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

/// **§3 的内容寻址键**（`docs/design/module-artifacts.md` §3/§8.1）：与 [`key`] **同一把尺子**
/// （同一个 [`key_parts`] ✓），但**不折 `SOKO_*` 开关**。
///
/// **为什么必须另有一条**（而不是把 `flags_state` 从 [`key`] 里删掉）：检查点键要守 T-A06
/// ——「**内容相同不代表位置相同**」＋「诊断开关换挡必须 miss」（[`flags_state`] 的注释里
/// 那条 b1 CI 判红就是它换来的 ✓）。而**可共享产物**要的正好相反：同一份源文本在**任何
/// 机器、任何环境**上都必须算出**同一个键**，否则下载来的产物永远匹配不上 ✗。
/// ⇒ 两个用途、两条键，**都从这一条链出来**（单一真相 ✓）。
///
/// ⚠ **保留**在内容键里的维度一个不少：格式 · 报告形状 · 版本 · build stamp · prelude 模式 ·
/// **元变量档** · **prelude 覆盖指纹** · 源文本。后两个虽然从环境来，但**默认档下恒为
/// 常数**（`0`）⇒ 进键**不损可共享性**（下载方默认档算出同一个键 ✓），却堵住
/// 「`SOKO_METAVAR` / `SOKO_PRELUDE_DIR` 换挡后吃掉旧产物」✗。
pub fn content_key(src: &str, options: &CompileOptions) -> String {
    content_key_with_build(src, options, build_stamp())
}

/// [`content_key`] 的纯函数形（显式 build stamp；给测试用 ✓）。
pub fn content_key_with_build(src: &str, options: &CompileOptions, build: u64) -> String {
    key_parts(
        CACHE_FORMAT,
        crate::compile::REPORT_SHAPE,
        env!("CARGO_PKG_VERSION"),
        build,
        options.prelude == PreludeMode::Bare,
        metavar_state(),
        // ⚠ **`0` = 「不折 `SOKO_*`」**（§3 的「无环境」）—— **不是**「环境为空」的哈希
        // （[`flags_hash`] 对空集返回的是 `0xcbf2_9ce4_8422_2325` ✓）。用字面量而不把
        // `key_parts` 的实参换成 `Option`，是为了**不动那个签名**（它的每个实参都有既有
        // 判据钉着，改签名会牵动十几处测试调用点 ⇒ 白涨行数，而判据 ⑨ 只许减不许增 ✗）。
        // 两条链会不会因此撞名？**功能上不会**：源文本的 tag 先把它们分开
        // （`soko.lib-artifact/1` vs `soko.lib-checkpoint/1`，见 `project::session`）✓。
        //
        // ⚠ **方言也不在这条键里**（S2，`docs/design/dialect-config.md` §3.3 只点名
        // 检查点键 + 判定缓存键）：默认那条产物层路走 [`key`]（方言已进键 ✓），只有
        // 显式打开 `SOKONANODA_SHARED_ARTIFACTS=1` 的**实验性**共享通道才用这条键 ——
        // 它要的是「任何环境算出同一个键」，方言该不该进去留到 S3 之后单独定 ✓。
        0,
        crate::compile::prelude::prelude_override_state(),
        src,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **§3 的内容寻址键**（`docs/design/module-artifacts.md` §3/§8.1，2026-10-13 键拆分）：
    /// 与检查点键**同一把尺子**（`key_parts`）但**不折 `SOKO_*`**。
    ///
    /// 判据三条 —— 每条都咬住"它凭什么可共享"：
    /// ① 纯函数（同输入同键 ✓）；
    /// ② **与检查点键不同**：`flags_state` 传 `None` vs `Some(..)` ⇒ 两条链**永不撞名** ✓
    ///    （哪怕哪天两条路的源文本一模一样 ✓）；
    /// ③ 敏感性**一个不少**：源文本 / prelude 档 / build stamp / 版本 / 格式 / 形状 任一变 ⇒ 键变 ✓
    ///    （少了哪一条，共享产物就会跨版本/跨档被吃 ✗）。
    ///
    /// **反向验证**（实测）：把 `content_key_with_build` 里的 `None` 改回 `Some(flags_state())`
    /// ⇒ ② **当场判红** ✓；把 `key_parts` 里 `if let Some(..)` 那一段整段删掉
    /// ⇒ ② **仍然判红**（两条链算出来逐字相同）✓ —— 这条断言**同时**咬住那两个改动 ✓。
    /// **HO3b 的判据**：pattern / 近似档各占**一位**，默认档是 **0**（⇒ 键逐字节不变）。
    ///
    /// 反向验证：把 `pattern_bits` 的两个位移改成同一个位 ⇒ 第二、三条当场红（两档会撞键）。
    #[test]
    fn pattern_and_approx_own_distinct_key_bits() {
        assert_eq!(
            pattern_bits(false, false),
            0,
            "默认档必须是 0（老键照旧命中）"
        );
        assert_eq!(pattern_bits(true, false), 0b0100);
        assert_eq!(pattern_bits(false, true), 0b1000);
        assert_eq!(pattern_bits(true, true), 0b1100);
        // 与元变量三档（0/1/2）**不重叠** ⇒ 任意组合都互不撞键。
        for mode in 0u8..3 {
            for bits in [0b0100u8, 0b1000, 0b1100] {
                assert_eq!(bits & mode, 0, "新两位不许压到档位字节上");
            }
        }
    }

    #[test]
    fn the_content_key_is_flag_free_and_still_sensitive() {
        let full = CompileOptions::default();
        let bare = CompileOptions::new(PreludeMode::Bare);
        // ① 纯函数。
        assert_eq!(
            content_key_with_build("a", &full, 7),
            content_key_with_build("a", &full, 7)
        );
        // ② **与检查点键永不撞名**（`None` 那一段的守卫 ✓）。
        assert_ne!(
            content_key_with_build("a", &full, 7),
            key_with_build("a", &full, 7),
            "内容寻址键与检查点键必须是两条链（否则 T-A06 的语义会被悄悄改掉 ✗）"
        );
        // ③ 敏感性一个不少。
        assert_ne!(
            content_key_with_build("a", &full, 7),
            content_key_with_build("b", &full, 7),
            "源文本必须进键"
        );
        assert_ne!(
            content_key_with_build("a", &full, 7),
            content_key_with_build("a", &bare, 7),
            "prelude 模式必须进键"
        );
        assert_ne!(
            content_key_with_build("a", &full, 7),
            content_key_with_build("a", &full, 8),
            "build stamp 必须进键（否则跨构建会吃旧产物 ✗）"
        );
    }
}
