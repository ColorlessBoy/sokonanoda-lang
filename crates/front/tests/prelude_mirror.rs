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

use sokonanoda_front::compile::{prelude_def_span, prelude_source};
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

/// **prelude 源的干净性守卫（2026-10-10）**：把 prelude 的**生效源**
/// （`prelude_source()` —— 编辑器 F12 打开的那一份）当**普通文档**判一遍，
/// 断言 **0 诊断 + 声明计数** ✓。
///
/// **为什么必须有这条**（这才是它存在的理由）：prelude 是「**受信任的预置**」——
/// 安装后**不再被内核重查**（`crates/front/src/compile/prelude.rs:3` 的设计 ✓）。
/// ⇒ **体的类型错误只能靠"判 prelude 源"这条守卫抓** ✗✓：它不会在任何学生文件里
/// 冒出来，而 `install_l1_command` 的 `.expect(...)` **只保证 elaborate 成功、
/// 不保证内核接受** ✗。实测（2026-10-10）：`Classical.byContradiction` 的 `Or.elim`
/// **漏了动机位 `c`** ⇒ `Classical.em p` 被塞进 `f` 的位置 ⇒ 这份源 43 条声明里
/// 1 条判红，而**三层测试全绿**（没人判过这份源本身 ✗）。
///
/// **通道与 `scripts/soko grade` 同源**（不许自造一条文本比对的路 ✗）：无 `import`
/// 的普通文档走 `parse` → `prelude_mode_from_source` → `compile_all_with`
/// （= `crates/cli/src/check.rs:29` 的 `check_source` ✓）；计数读的就是
/// `query check` 用的那两份（`output.events` / `output.errors`，`query/mod.rs` ✓）。
///
/// **反向验证**（硬要求，2026-10-10 实测）：把 L1 里那行改回错体
/// （`Or.elim p (Not p) (Classical.em p) (fun …) (fun …)`）⇒ 本用例判红 ✓
/// （`cargo test -p sokonanoda-front --test prelude_mirror \
/// the_prelude_source_grades_clean_as_an_ordinary_document` ⇒ `FAILED`，exit 101）。
/// 判红时的报错原文（逐字；下面按注释宽度折行，实际是两行）：
///
/// ```text
/// prelude 源作为普通文档必须**0 诊断** ✗（`scripts/soko grade` 会 exit 1、编辑器里打开它就是满屏红）—— 它是**受信任安装**的源，体的类型错误不会在任何学生文件里冒出来，只有这条守卫抓得住：
///   L34:1 [kernel-rejected] 类型不匹配：期望 `Sort(0)`，实际是 `((Or.[] 第 1 个绑元（p）) (Not.[] 第 1 个绑元（p）))`
/// ```
///
/// ⚠ **计数只许多不许少**：`checked >= 43` 是**下界**（2026-10-10 实测 43）——
/// 加声明不用改这条 ✓，**删**声明（或把它写成判红/`sorry`）会判红 ✓。
#[test]
fn the_prelude_source_grades_clean_as_an_ordinary_document() {
    use sokonanoda_front::compile::{
        compile_all_with, prelude_mode_from_source, CheckEvent, CompileOptions, DeclStatus,
    };
    let src = prelude_source();
    let file = sokonanoda_front::parse(src)
        .expect("prelude 源必须能 parse（否则学生 F12 打开它就是满屏红 ✗）");
    let options = CompileOptions {
        prelude: prelude_mode_from_source(src),
    };
    let (output, report) = compile_all_with(&file, &options);

    let count = |status: DeclStatus| report.decls.iter().filter(|d| d.status == status).count();
    let checked = count(DeclStatus::Checked);
    let failed = count(DeclStatus::Failed);
    let open = count(DeclStatus::Open);
    let events_checked = output
        .events
        .iter()
        .filter(|e| matches!(e, CheckEvent::DeclarationChecked { .. }))
        .count();

    // ① 0 诊断（与 `scripts/soko grade --json` / `query check` 的 `failed` 同源）。
    assert!(
        output.errors.is_empty() && report.errors.is_empty(),
        "prelude 源作为普通文档必须**0 诊断** ✗（`scripts/soko grade` 会 exit 1、\
         编辑器里打开它就是满屏红）—— 它是**受信任安装**的源，体的类型错误不会在\
         任何学生文件里冒出来，只有这条守卫抓得住：\n{}",
        output
            .errors
            .iter()
            .map(|e| format!(
                "  L{}:{} [{}] {}",
                e.span.start.line,
                e.span.start.column,
                e.kind.code(),
                e.message
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // ② 声明计数：每条都 Checked（0 failed / 0 open），且不少于 43。
    assert_eq!(
        failed, 0,
        "prelude 源里不许有判红的声明 ✗（{checked} checked / {failed} failed / {open} open）"
    );
    assert_eq!(
        open, 0,
        "prelude 源里不许有 open 练习 ✗（{checked} checked / {failed} failed / {open} open）"
    );
    assert_eq!(
        checked,
        report.decls.len(),
        "prelude 源里每条声明都必须 Checked ✗（{} 条里只有 {checked} 条）",
        report.decls.len()
    );
    assert_eq!(
        events_checked, checked,
        "事件计数（`query check` 读的那份）与报告计数必须一致 ✗"
    );
    assert!(
        checked >= 43,
        "prelude 的声明计数只许多不许少（2026-10-10 实测 43，今天 {checked}）—— \
         变少说明有声明被删掉、或被写成了判红/`sorry` ✗"
    );
}

/// **E1（2026-10-08）的方向翻转**：`prelude/*.sokonanoda` **三段真源**才是真相，
/// 合并视图 `prelude/Prelude.sokonanoda` 是**由它们生成**的 ✓。
///
/// 判据（两条，缺一不算）：
/// ① 三段文件按 `"\n"` 拼接 == 视图（**逐字节**）—— 上面那条镜子判据管"视图 ==
///    `prelude_source()`"，这条管"三段 ⇒ 视图"，合起来 = 三段 ⇒ 编译期真相 ✓；
/// ② **Rust 里不许再有 prelude 源文本**（哨兵行在 `crates/**/*.rs` 里 0 命中）——
///    否则"改文件行为不变"的老毛病会悄悄回来 ✗（那正是用户 2026-10-06 报的：
///    仓库里有一份"镜子"，改它**行为一个字不变** ✗）。
///
/// ⚠ **三段文件不许带/丢行尾换行**：`include_str!` 逐字节取文件 ⇒ 多一个 `\n`
/// 就改 `prelude_source()` 的字节 ⇒ 全课程 `--json` 会漂 ✗（判据③）。
#[test]
fn the_three_source_files_are_the_truth_and_rust_holds_no_prelude_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let read = |name: &str| {
        let path = root.join("prelude").join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("必须有真源文件 {}：{e}", path.display()))
    };
    let joined = format!(
        "{}\n{}\n{}",
        read("Eq.sokonanoda"),
        read("L1.sokonanoda"),
        read("Quot.sokonanoda")
    );
    assert_eq!(
        joined,
        prelude_source(),
        "三段真源拼接必须与编译期真相**逐字节**相等（`include_str!` 直取它们 ⇒ \
         不相等说明有文件多了/少了字节，例如行尾换行 ✗）"
    );
    assert_eq!(
        joined,
        std::fs::read_to_string(root.join("prelude/Prelude.sokonanoda")).expect("视图"),
        "合并视图必须等于三段拼接（E1 之后**方向是文件 ⇒ 视图** ✓）"
    );

    // ② 哨兵行（三条各自唯一 —— 今天它们在 `crates/**/*.rs` 里各 1 命中，就是那三个常量）
    //
    // ⚠ 哨兵**用 `concat!` 拼**：否则这段字符串本身就在本文件里出现 ⇒ 判据自己
    // 咬自己 ✗（实测踩到）。拼出来 = 源文本里永远不会有整条哨兵 ✓ 覆盖不缩水 ✓。
    let sentinels = [
        concat!("axiom Classical.em", " : (p : Prop)"),
        concat!("def Or.elim", " {a b c : Prop}"),
        concat!("axiom Quot.mk", " {u} : {A : Sort u}"),
    ];
    let mut rust_files: Vec<PathBuf> = Vec::new();
    collect_rs(&root.join("crates"), &mut rust_files);
    assert!(
        rust_files.len() > 50,
        "哨兵扫描的前提：必须扫到 `crates/**/*.rs`（扫到 {} 个 ⇒ 路径写错了 ✗）",
        rust_files.len()
    );
    for sentinel in sentinels {
        for path in &rust_files {
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            assert!(
                !text.contains(sentinel),
                "**Rust 里不许再有 prelude 源文本** ✗：{} 里出现了 {sentinel:?}\n\
                 ⇒ prelude 的真相是 `prelude/*.sokonanoda`（`include_str!` 直取）；\
                 把源文本搬回 Rust = 回到「改文件行为不变」的老毛病 ✗",
                path.display()
            );
        }
    }
}

/// 递归收集 `*.rs`（标准库够用；不引依赖 ✓）。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
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
///
/// **G-60（0.83.0）增量**：集合建构式两条也登记在同一区（设计 §19）。它们的展开
/// 目标**不在 `elab.rs` 而在 `parser.rs`**（`parse_set_builder` 是 parser 期脱糖）
/// ⇒ 本判据按**每条登记指向哪个文件**核对（逐字 ✓），不是一律查 `elab.rs`：
/// `{x ∈ A | P x}` → `Set.sep`（`parser.rs`，**卷 I 的库常量**）；`{x : α | P x}`
/// **没有目标常量**（脱糖成函数）⇒ 只要求**如实登记** ✓。`{x | P x}` **故意不登记**
/// （它写不出来）⇒ 断言它**不在**登记区（防有人偷偷把它做成能写的）。
/// **E2 的判据（2026-10-08）**：`Nat`/`Bool` 两族 9 个名字是**内核内建**
/// （Rust 里手搓 AST + 内核按名字给算术规约）⇒ prelude 源里**没有**它们的声明
/// ⇒ `prelude_def_span` 对它们**都**返回 `None`。这条把"边界"钉成**可查的事实** ✓：
/// ① 登记行逐字在 prelude 里（学生看得到"为什么 F12 不跳"）；
/// ② 9 个名字的 `prelude_def_span` **确实**是 `None`（登记**不许说谎** ✗）；
/// ③ **反向**：哪天有人把它们源化了 ⇒ ② 判红 ⇒ 必须同轮更新登记行 ✓。
///
/// **为什么是"登记"而不是"源化"**（规划 §2 E2 的两半，本轮选了后一半）：spike 发现
/// `Nat.add` 今天是 `Declar::Definition`（值自指 + `ReducibilityHint::Regular`）且
/// 规约走内核内建（`eval.rs` 的 G-76 递归方程，**按名字**）⇒ 源级 `axiom` 会换掉内核
/// 声明形态，过不了「与手搓版全课程 `--json` 逐字节相同」✗（`Nat`/`Bool` 又是最先装的）。
#[test]
fn builtin_rust_registry_is_honest() {
    let prelude = prelude_source();
    for line in [
        "-- sokonanoda:builtin-rust \"Nat / Nat.zero / Nat.succ / Nat.rec / Nat.add\"",
        "-- sokonanoda:builtin-rust \"Bool / Bool.true / Bool.false / Bool.rec\"",
    ] {
        assert!(
            prelude.lines().any(|l| l == line),
            "prelude 里必须有逐字登记行：{line:?} ✗（E2：内建家族登记区）"
        );
    }
    for name in [
        "Nat",
        "Nat.zero",
        "Nat.succ",
        "Nat.rec",
        "Nat.add",
        "Bool",
        "Bool.true",
        "Bool.false",
        "Bool.rec",
    ] {
        assert!(
            prelude_def_span(name).is_none(),
            "`{name}` 登记成「内核内建、无源位置」⇒ `prelude_def_span` 必须是 `None` ✗ \
             （真源化了就同轮改登记行 ✓）"
        );
    }
}

#[test]
fn builtin_sugar_registry_matches_the_elaborator() {
    let prelude = prelude_source();
    let read = |rel: &str| {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_else(|err| panic!("read {rel}: {err}"))
    };
    let elab = read("src/compile/elab.rs");
    let parser = read("src/parser.rs");

    // 有单一目标的：登记的目标名必须**同时**出现在**它真正的展开落点**那个文件里
    // （逐字 ✓）—— `{a}`/`{a, b}` 在 elab 期展开（`elab.rs`），
    // `{x ∈ A | P x}` 在 parser 期脱糖（`parser.rs`）。
    for (sugar, target, src) in [
        ("{a}", "Set.singleton", &elab),
        ("{a, b}", "Set.pair", &elab),
        ("{x ∈ A | P x}", "Set.sep", &parser),
    ] {
        let want = format!("-- sokonanoda:builtin-sugar \"{sugar}\" => {target}");
        assert!(
            prelude.lines().any(|line| line == want),
            "prelude 里必须有逐字登记行：{want:?} ✗（E11：登记区）"
        );
        assert!(
            src.contains(target),
            "登记的目标 `{target}` 必须在它的展开落点里真的存在（逐字一致 ✗）：{sugar}"
        );
    }
    // 无单一目标的两条：**如实**登记，不许编目标 ✗。
    assert!(
        prelude
            .lines()
            .any(|line| line.starts_with("-- sokonanoda:builtin-sugar \"⟨a, b⟩\" => 期望类型决定")),
        "`⟨a, b⟩` 必须**如实**登记「期望类型决定」（`elab.rs` L2836 的路线 C ✓）—— 不许编一个目标名 ✗"
    );
    assert!(
        prelude.lines().any(|line| line
            == "-- sokonanoda:builtin-sugar \"{x : α | P x}\" => 函数（fun (x : α) => P x），无目标常量"),
        "`{{x : α | P x}}` 必须**如实**登记「脱糖成函数、无目标常量」（设计 §19）—— 不许编一个目标名 ✗"
    );
    // `{x | P x}` **写不出来**（没有元变量）⇒ 不许有它的登记行（有 = 有人把它做成了能写的 ✗）。
    assert!(
        !prelude
            .lines()
            .any(|line| line.starts_with("-- sokonanoda:builtin-sugar \"{x | P x}\"")),
        "`{{x | P x}}` 故意不登记（没有元变量 ⇒ 类型没有来源，诊断 `set-builder-shape` 指路）\
         —— 出现登记行说明边界被打破了 ✗"
    );
}
