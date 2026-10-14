//! **引擎身份**（CACHE-KEY-NO-ENGINE-IDENTITY，2026-10-14）：这份二进制**编进去的源**
//! 的摘要 —— 折进 `compile::cache::build_stamp` ⇒ 折进**全局缓存键**。
//!
//! 事故、覆盖范围（为什么是 `crates/front/src` + `crates/kernel/src` + `prelude/` +
//! `reference/` + `build.rs`）、为什么**不是**哈希 `current_exe()`、以及稳定性口径，
//! 全部写在 `scan` 的头注释里（那是算法与输入清单所在的地方，**单一真相** ✓）。
//!
//! 这里只有两样东西：**编译期注入的常量**与**折进键的那一份**（含逃生门）。
//! 算法与"哪几个目录算引擎"在 `scan.rs`：它被 `crates/front/build.rs` 用 `include!`
//! 拼进 build script，所以**不能**读 `env!("SOKO_ENGINE_DIGEST")`（那正是 build script
//! 要产出的东西 ⇒ 会编译失败 ✗）—— 这就是两个文件的分界。

/// **引擎身份**：这份二进制**编进去的源**的摘要（16 位十六进制），由 `crates/front/build.rs`
/// 在编译期算好、经 `cargo:rustc-env` 注入。
///
/// `env!`（**不是** `option_env!`）：build script 没产出它 ⇒ **编译失败** ⇒ 不存在
/// "悄悄没有身份"的降级 ✗（那正是本缺口最怕的形状：键里少一个必需分量而没人知道）。
pub(crate) const ENGINE_DIGEST: &str = env!("SOKO_ENGINE_DIGEST");

/// 折进键的那一份身份：`None` = 逃生门 `SOKO_CACHE_KEY_NO_ENGINE_ID=1`（= 旧公式）。
///
/// **极性**：新行为是"身份**进**戳"，所以逃生门说的是"**不要**身份"。它自己也以
/// `SOKO_` 开头 ⇒ 被 `compile::cache::flags_state` 折进键 ⇒ 开着它跑出来的条目与关着
/// 它的**永不互命中** ✓（不会互相污染）。
///
/// **为什么留它**（与 `SOKO_CACHE_SHAPE=off` / `SOKO_CACHE_KEY_KEEP_PROFILE=1` 同款）：
/// 用户立的回测铁律要求"撤掉修复 ⇒ 判据必须判红"。本修复的"撤掉"= 让键**不再**带引擎
/// 身份 ⇒ 只剩这一种表达方式 ✓：开着它，三步夹具（旧引擎 → 新引擎 → 旧引擎）**必须**
/// 回到 `1 ⇒ 0 ⇒ 0` ✓（`.cache/ux/fix-cacheR/VERDICT.md`）。生产路径不该设它。
pub(crate) fn engine_identity() -> Option<&'static str> {
    if matches!(
        std::env::var("SOKO_CACHE_KEY_NO_ENGINE_ID").as_deref(),
        Ok("1")
    ) {
        return None;
    }
    Some(ENGINE_DIGEST)
}

/// 摘要算法与输入清单（`build.rs` 与单测共用同一份源码 ⇒ 不会各算各的 ✓）。
/// **只在测试里编**：生产路径只需要上面那个常量 ✓。
#[cfg(test)]
mod scan;

#[cfg(test)]
mod tests {
    /// **接线判据**（本缺口的守卫）：`build.rs` 注入的 `SOKO_ENGINE_DIGEST` 必须**就是**
    /// 当前源码树算出来的摘要。
    ///
    /// 咬的是什么：**cargo 没有重跑 build script**（`rerun-if-changed` 漏了目录、或漏了
    /// 包外的 `crates/kernel/src`）⇒ 二进制带着**旧**身份 ⇒ "改了引擎却命中旧条目" ✗。
    /// 这条测试跑在**编译产物**里、却把**真树**重算一遍 ⇒ 两边不一致立刻红 ✓。
    /// （算法与输入清单只有 `scan` 一份 ⇒ 不是自证空转 ✓。）
    #[test]
    fn the_injected_engine_digest_is_the_digest_of_this_tree() {
        let (digest, unreadable) =
            super::scan::digest_of_tree(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
        assert!(
            unreadable.is_empty(),
            "引擎输入必须都读得到：{unreadable:?}"
        );
        assert_eq!(
            format!("{digest:016x}"),
            super::ENGINE_DIGEST,
            "注入的引擎身份与真树算出来的不一致 ⇒ build script 没重跑（引擎改了却命中旧条目 ✗）"
        );
        assert_eq!(super::ENGINE_DIGEST.len(), 16, "身份必须是 16 位十六进制");
        assert!(
            super::ENGINE_DIGEST.bytes().all(|b| b.is_ascii_hexdigit()),
            "身份必须是十六进制：{}",
            super::ENGINE_DIGEST
        );
        assert_ne!(
            super::ENGINE_DIGEST,
            "0000000000000000",
            "退化值会把身份分量抹掉 ✗"
        );
    }

    /// **默认必须有身份**（逃生门没设时）—— 这条挡的是"常量被接成空 / 逃生门默认开着" ✗。
    #[test]
    fn the_engine_identity_is_present_by_default() {
        assert!(
            super::engine_identity().is_some(),
            "默认（没设 SOKO_CACHE_KEY_NO_ENGINE_ID）**必须**有引擎身份"
        );
    }
}
