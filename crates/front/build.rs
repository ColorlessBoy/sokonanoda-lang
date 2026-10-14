//! 把「这份引擎是什么」在**编译期**算成一个摘要，注入 `SOKO_ENGINE_DIGEST`
//! —— `compile::cache` 把它折进 `build_stamp` ⇒ 折进**全局缓存键**
//! （CACHE-KEY-NO-ENGINE-IDENTITY，2026-10-14；事故与覆盖范围见
//! `src/engine_digest.rs` 的头注释）。
//!
//! 为什么用 build script 而不是 `env!`/`option_env!`：源码摘要**算不出来**——
//! 没有任何一个 cargo 提供的编译期常量表达"这几个目录的内容"。`crates/cli/build.rs`
//! （注入 `SOKONANODA_TARGET`）是同一个模式 ✓。
//!
//! ⚠ **`rerun-if-changed` 必须把目录也声明一遍**：只声明"上次见到的那些文件"
//! 会漏掉**新增**的文件（引擎多了一个模块 ⇒ 摘要不变 ✗）；目录扫描能覆盖它 ✓。
//! ⚠ 声明的是 `crates/kernel/src`（**包外**的路径）—— 内核改了必须让本 build script
//! 重跑，否则摘要会停在旧值（"改了内核却命中旧条目" ✗）。
include!("src/engine_digest/scan.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo 一定给 build script 这个变量");
    let manifest = Path::new(&manifest);

    let files = engine_files(manifest);
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.abs.display());
    }
    for (_, path) in engine_inputs(manifest) {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let (digest, unreadable) = digest_of_tree(manifest);
    for rel in &unreadable {
        // 出声，不静默：读不到的输入会折成 `unreadable:<rel>` ⇒ 与真树算出来的摘要
        // 不同 ⇒ `cache.rs` 那条「接线」单测当场判红 ✓。
        println!("cargo:warning=引擎源码读不到（摘要按 unreadable 折）：{rel}");
    }
    println!("cargo:rustc-env=SOKO_ENGINE_DIGEST={digest:016x}");
}
