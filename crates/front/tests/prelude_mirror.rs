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

/// **E10 的判据（闭包记法表这一层）**：内建记法必须在**闭包记法表**里、
/// 且带 **prelude 指令行**的 span ✓ —— 否则 `Query::notation_at`（只查
/// `project.notations`）认不出 `∧`，学生文件里按 F12 **毫无反应** ✗。
///
/// ⚠ 这条是**接缝判据**：`notation.rs` 里"有声明点"（另一条判据 ✓）**不等于**
/// 查询层拿得到 —— 实测踩过：只在 `notation.rs` 里给 span，`notation_at` 仍然
/// 找不到 `∧`（它走的是 `project.notations` ✗）✓。
#[test]
fn builtin_notations_reach_the_closure_notation_table() {
    use sokonanoda_front::compile::CompileOptions;
    use sokonanoda_front::project;
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-builtin-table-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("sokonanoda.toml"),
        "entry = \"Canvas.sokonanoda\"\n",
    )
    .expect("write manifest");
    let entry = dir.join("Canvas.sokonanoda");
    std::fs::write(&entry, "theorem t (a b : Prop) (h : a ∧ b) : a ∧ b := h\n")
        .expect("write entry");
    let report = project::compile_project(&entry, None, &CompileOptions::default(), None);
    let table = report.notations;
    let prelude = prelude_source();
    let mut checked = 0;
    for decl in sokonanoda_front::notation::builtin_notation_decls_for_test() {
        // `=` **故意**没有指令行（最长匹配会把 `=>` 吃坏 ✗）⇒ 它不在这条判据里 ✓。
        if decl.symbol == "=" {
            continue;
        }
        let found = table
            .iter()
            .find(|it| it.symbol == decl.symbol)
            .unwrap_or_else(|| {
                panic!(
                    "闭包记法表里没有内建 `{}` ✗（`notation_at` 会找不到它）",
                    decl.symbol
                )
            });
        let at = &prelude[found.span.start.offset..found.span.end.offset];
        assert!(
            at.starts_with("-- sokonanoda:builtin-notation"),
            "闭包表里的内建 `{}` 必须带 prelude 指令行的 span ✗（实际圈到：{at:?}）",
            decl.symbol
        );
        checked += 1;
    }
    assert!(checked >= 5, "至少 5 条（实测 {checked}）");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **E10 的接缝判据（词法那一段）**：`notation_at` 的第一步是
/// `notation_input::symbol_at_with_sources(text, offset, &闭包符号名)` —— 它对
/// **内建**符号（`∧`）必须也认得出 ✓（内建不在闭包符号表里，走的是
/// `symbol_at` 的"本文件声明 + 内建"那条路 ✓）。
///
/// ⚠ 这条单独钉住，是因为"闭包表里有 `∧`"（上一条 ✓）**不等于**"光标处的 `∧`
/// 能被认出来" ✗ —— 两步都通，`notation_at` 才答得上 ✓。
#[test]
fn the_lexer_recognises_a_builtin_notation_symbol_at_the_cursor() {
    use sokonanoda_front::notation_input::symbol_at_with_sources;
    let src = "theorem t (a b : Prop) (h : a ∧ b) : a ∧ b := h\n";
    let offset = src.find('∧').expect("the symbol");
    let found = symbol_at_with_sources(src, offset, &[]);
    assert!(
        found.is_some(),
        "内建记法 `∧` 必须被认得出（否则 `notation_at` 第一步就断了 ✗）"
    );
    let (symbol, target) = found.expect("checked");
    assert_eq!(symbol, "∧");
    assert_eq!(target.as_deref(), Some("And"), "内建的目标名来自内建表 ✓");
}

/// **E11 的判据**：内建糖（`{a}` / `{a, b}` / `⟨a, b⟩`）在 prelude 里有**登记行**，
/// 且与 `elab.rs` 里的**硬编码目标逐字一致** ✓。
///
/// 判红（2026-09-28 实测）：全仓 `grep -rn "builtin-sugar"` **只有注释里提过这个名字** ✗
/// ⇒ 登记区**不存在**；而 `elab.rs` 里 `{a}`/`{a, b}` 的目标是硬编码的
/// （**L1330**：`Set.singleton` / `Set.pair` ✓）⇒ 学生查不到、也没人能证明两边一致 ✗。
///
/// ⚠ `⟨a, b⟩` **没有单一目标**（目标构造子由**期望类型**的头决定，`elab.rs` **L2836**
/// 的"路线 C" ✓）⇒ 本判据只要求它**如实登记**这件事 ✓，**不许**断言某个具体目标 ✗。
///
/// **反向验证**：把登记行里的 `Set.singleton` 改成别的名字 ⇒ 判红 ✓。
#[test]
fn builtin_sugar_registry_matches_the_elaborator() {
    let prelude = prelude_source();
    let elab = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/compile/elab.rs"),
    )
    .expect("read elab.rs");

    // 有单一目标的两条：登记的目标名必须**同时**出现在 elab.rs 里（逐字 ✓）。
    for (sugar, target) in [("{a}", "Set.singleton"), ("{a, b}", "Set.pair")] {
        let want = format!("-- sokonanoda:builtin-sugar \"{sugar}\" => {target}");
        assert!(
            prelude.lines().any(|line| line == want),
            "prelude 里必须有逐字登记行：{want:?} ✗（E11：登记区）"
        );
        assert!(
            elab.contains(target),
            "登记的目标 `{target}` 必须在 `elab.rs` 里真的存在（逐字一致 ✗）：{sugar}"
        );
    }
    // 无单一目标的那条：**如实**登记"由期望类型决定"，不许编目标 ✗。
    assert!(
        prelude
            .lines()
            .any(|line| line.starts_with("-- sokonanoda:builtin-sugar \"⟨a, b⟩\" => 期望类型决定")),
        "`⟨a, b⟩` 必须**如实**登记「期望类型决定」（`elab.rs` L2836 的路线 C ✓）—— 不许编一个目标名 ✗"
    );
}
