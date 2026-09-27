//! **E09 的守卫**：仓库里的 `prelude/Prelude.sokonanoda` 必须与**编译期真相**
//! （`sokonanoda_front::compile::prelude_source()`）**逐字节相等** ✓。
//!
//! 为什么要有这份"镜子"（用户 2026-09-27 的 v0.76.0 计划 §E09）：prelude 一直是
//! `crates/front/src/compile/prelude.rs` 里的 **Rust 字符串常量**
//! （`PRELUDE_EQ_SRC` / `PRELUDE_L1_SRC`），`prelude_source_path()` 只在 F12 时
//! **物化到临时目录** ⇒ **学生根本看不到**那份源 ✗（想看 `And`/`Or`/`Eq` 是怎么声明的
//! 只能去读 Rust 源码 ✗）。
//!
//! ⚠ **单一真相仍然留在编译期常量** ✗✓（计划原文）—— 镜子是**产物**，
//! **绝不**改成运行时读文件（会碰 `PreludeMode::Bare` 那条红线 ✗）。
//! 所以守卫的方向是「**常量 ⇒ 文件**」：常量动了、镜子没跟着动 ⇒ 这里判红 ✓。
//!
//! 重新生成镜子（**唯一**正路，别手抄 ✗ —— 手抄必然漂移）：
//!
//! ```text
//! SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror
//! ```
//!
//! **反向验证**（改这里之前先跑）：把镜子文件改一个字符、或改常量里的一行 ⇒
//! `the_repo_mirror_is_byte_identical_to_the_compiled_prelude` 必须判红 ✓。

use sokonanoda_front::compile::prelude_source;
use std::path::{Path, PathBuf};

/// 镜子文件的路径（仓库根下的 `prelude/Prelude.sokonanoda`）。
fn mirror_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("prelude")
        .join("Prelude.sokonanoda")
}

#[test]
fn the_repo_mirror_is_byte_identical_to_the_compiled_prelude() {
    let path = mirror_path();
    let truth = prelude_source();

    // 生成模式：只在显式要求时写盘 ✓（普通 `cargo test` **绝不**改仓库 ✗）。
    if std::env::var("SOKO_WRITE_PRELUDE").as_deref() == Ok("1") {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create prelude/");
        }
        std::fs::write(&path, truth).expect("write the mirror");
        eprintln!("wrote {} ({} bytes)", path.display(), truth.len());
        return;
    }

    let Ok(on_disk) = std::fs::read_to_string(&path) else {
        panic!(
            "仓库里必须有 prelude 的镜子文件（学生要能读到它 ✗）：{}\n\
             ⇒ 生成：SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror",
            path.display()
        );
    };
    if on_disk != truth {
        // 报**第一处**差异（行号 + 两边原文）—— 只报"不相等"会让人无从下手 ✗。
        let mut line = 1usize;
        for (a, b) in on_disk.lines().zip(truth.lines()) {
            if a != b {
                panic!(
                    "镜子文件与编译期真相**漂移**了 ✗（第 {line} 行）：\n\
                     文件：{a:?}\n真相：{b:?}\n\
                     ⇒ 重新生成：SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror"
                );
            }
            line += 1;
        }
        panic!(
            "镜子文件与编译期真相**长度**不同 ✗：文件 {} 字节 / 真相 {} 字节（前 {} 行相同）\n\
             ⇒ 重新生成：SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror",
            on_disk.len(),
            truth.len(),
            line - 1
        );
    }
}
