//! 课程共享库子项目（`course/shared/`）的守护 —— I16 的"教学内容 import 化"落地。
//!
//! 背景（用户 2026-09-18：『顺便重构教学内容呢，前后 import 之类的，这个是不是适合
//! 一个外接的子项目』）：单元画布**故意各自自给自足**（学习者打开一个文件就能看到
//! 全部前置声明；golden/镜像/课程树也都按"一单元一文件"钉着），但同几段声明在
//! 多个单元里**逐字重复**（历史上是 And 公理 25 份、Or 块 12 份、显式 Nat 块 8 份
//! ——**2026-09-21 C2.5 之后只剩 `Nat` 块 8 份**：用户拍板删掉入门课的自建 And/Or
//! 骨架，prelude 的真归纳接管，于是那两个规范模块与它们的副本表一并删除）。
//! 复制粘贴的漂移没有任何测试能发现——事件计数只看得见语义变化，改个 binder 风格
//! 或注释位置照样全绿。
//!
//! 做法：把规范文本放进 `course/shared/`（一个**真的子项目**：有 sokonanoda.toml、
//! 可被 import、可单独判卷），再用这个测试守住"每个单元里的副本与规范文本逐字一致"。
//! 画布不 import 它——两份载体，测试负责不让它们漂移。
//!
//! 两个方向都查：
//!   * 表里列的每份文件都必须含规范文本（改了规范文本不更新副本 ⇒ 红）；
//!   * 含规范文本的文件必须都在表里（新单元抄了这段却不登记 ⇒ 红，提示你更新表）。

use std::path::{Path, PathBuf};
use std::process::Command;

/// `course/shared/Nat.sokonanoda` → 8 份副本（2 单元 × 中英 × 画布+解答）。
const NAT_COPIES: &[&str] = &[
    "course/unit6-induction-recursion-1.sokonanoda",
    "course/unit7-induction-recursion-2.sokonanoda",
    "course/en/unit6-induction-recursion-1.sokonanoda",
    "course/en/unit7-induction-recursion-2.sokonanoda",
    "course/solutions/unit6-induction-recursion-1-solution.sokonanoda",
    "course/solutions/unit7-induction-recursion-2-solution.sokonanoda",
    "course/en/solutions/unit6-induction-recursion-1-solution.sokonanoda",
    "course/en/solutions/unit7-induction-recursion-2-solution.sokonanoda",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// 只留代码行：去掉 `--` 行注释与空行（中英镜像的注释语言不同，代码必须一致）。
fn code_only(text: &str) -> String {
    text.lines()
        .map(|line| line.split("--").next().unwrap_or("").trim_end())
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 语料里的全部文件（画布 + 解答，中英各一份 + 单元⑪ 的多文件项目）。
///
/// `unit11-project/` 是 2026-09-21（R3）补进来的：当时它的 `Logic.sokonanoda` 是
/// And 公理块的又一份逐字副本，而在那之前它**不在**扫描面里——抄了那段却不登记
/// 也照样全绿（手工同步它时发现的守卫空洞）。C2.5 删骨架之后它不再含任何规范
/// 文本，但扫描面保持不变（下一条新抄的块照样会被抓到）。
fn corpus_files() -> Vec<String> {
    let root = repo_root().join("course");
    let mut out = Vec::new();
    for dir in [
        "",
        "en",
        "solutions",
        "en/solutions",
        "unit11-project",
        "unit11-project/solutions",
    ] {
        let base = if dir.is_empty() {
            root.clone()
        } else {
            root.join(dir)
        };
        for entry in std::fs::read_dir(&base).expect("read course dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|ext| ext == "sokonanoda") {
                let relative = path
                    .strip_prefix(repo_root())
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(relative);
            }
        }
    }
    out.sort();
    out
}

fn run_sokonanoda(file: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg(file)
        .current_dir(repo_root())
        .output()
        .expect("run sokonanoda")
}

#[test]
fn shared_modules_compile_standalone() {
    // C2.5 之后只剩 `Nat` 一个规范模块（`And`/`Or` 随自建骨架一起退役）。
    let module = "course/shared/Nat.sokonanoda";
    let out = run_sokonanoda(module);
    assert!(
        out.status.success(),
        "{module} must compile on its own: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn shared_demo_compiles_through_the_import_closure() {
    // 这个子项目存在的另一个理由：它是"课程内容 import 化"的可执行样例——
    // 跨模块递归（Nat 的显式归纳块 + Nat.rec）。**C2.5 之后** And/Or 两条演示
    // 改用 prelude 的真归纳（不再 import 那两个模块），跨模块那条线由 Nat 承担。
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["--json", "course/shared/Demo.sokonanoda"])
        .current_dir(repo_root())
        .output()
        .expect("run sokonanoda");
    assert!(
        out.status.success(),
        "Demo must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = String::from_utf8_lossy(&out.stdout);
    for name in ["and_comm_demo", "or_comm_demo", "double"] {
        assert!(
            events.contains(&format!("\"name\":\"{name}\"")),
            "Demo must check {name}: {events}"
        );
    }
    // `#reduce double zero` ⇒ zero、`double 2` ⇒ 4：递归定义真的算出来了。
    assert!(
        events.contains("expr.reduced"),
        "Demo keeps its #reduce self-tests: {events}"
    );
}

#[test]
fn every_unit_copy_matches_the_canonical_module() {
    // C2.5 之后只剩 `Nat` 这一条线（`And`/`Or` 的规范模块与副本表一起退役）。
    let files = corpus_files();
    let module = "course/shared/Nat.sokonanoda";
    let canonical = code_only(&read(module));
    assert!(!canonical.is_empty(), "{module} must not be empty");
    let mut found: Vec<String> = files
        .iter()
        .filter(|file| code_only(&read(file)).contains(&canonical))
        .cloned()
        .collect();
    found.sort();
    let mut expected: Vec<String> = NAT_COPIES.iter().map(|s| s.to_string()).collect();
    expected.sort();

    let missing: Vec<&String> = expected.iter().filter(|f| !found.contains(f)).collect();
    let extra: Vec<&String> = found.iter().filter(|f| !expected.contains(f)).collect();
    assert!(
        missing.is_empty(),
        "{module} 改了但这些副本没跟上（逐字复制的那份必须一起改）：{missing:#?}"
    );
    assert!(
        extra.is_empty(),
        "这些文件也含有 {module} 的规范文本，但没登记进副本表——请更新 \
         crates/cli/tests/course_shared.rs：{extra:#?}"
    );
}

#[test]
fn the_shared_library_replaces_rather_than_duplicates_the_canvas_purpose() {
    // 画布仍然自给自足：每个单元文件里都不得出现 `import`（那是新单元/项目的形态）。
    // 这条不是洁癖——它挡住"有人顺手把画布改成 import 共享库"的回归：
    // 那样学习者打开单元就得追模块，golden 与镜像契约也要整体重钉。
    for file in corpus_files() {
        if !file.contains("/unit") {
            continue;
        }
        // `unit11-project/` 例外：它**就是**多文件项目（入口第一行 `import Logic`），
        // 单元⑪ 教的那件事。它进语料是为了「逐字副本不许漂移」（C2.5 之前是它的
        // And 公理块；现在只剩 `Nat` 块那条线管不到它，但留着无害——它同样要接受
        // 「抄了规范文本就得登记」的检查），与「单元画布自给自足」这条正交。
        if file.starts_with("course/unit11-project/") {
            continue;
        }
        let code = code_only(&read(&file));
        assert!(
            !code
                .lines()
                .any(|line| line.trim_start().starts_with("import ")),
            "{file} 应该是自给自足的单元画布（不该出现 import）"
        );
    }
}
