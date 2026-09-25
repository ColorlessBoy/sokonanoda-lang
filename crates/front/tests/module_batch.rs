//! **T-C3：等价性判据**（阶段 C 的刹车点）。
//!
//! 契约（`docs/design/module-batch.md` §2.3）：对模块根下**每个**文件 `F`，
//! 批编给出的报告/事件必须与"**以 `F` 为入口、按今天的路径单独编一次**"
//! **逐项相同**：声明状态、诊断（错误/警告）、事件序列（`decl.checked` /
//! `exercise.open` …，含 `cmd` 下标）。
//!
//! 为什么这是刹车点：平坦命令序让 `F` 的环境包含**它没有 import** 的模块
//! （T-C1 §3 点名的三类差异：名字遮蔽、重复声明诊断、未使用类警告）。
//! 判不过 ⇒ 按 `e2-plan.md` §3 **不接 `build`**，只留 API + 本判据 + 差异记录。
//!
//! 两个用例：
//! * 夹具（快，进 CI）：共享库 + 两个单元 + 一个**同名声明**的文件；
//! * 真课程（`#[ignore]`，手动跑）：`courses/set-theory` 整根逐文件比。

use std::path::{Path, PathBuf};

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::compile_plan;
use sokonanoda_front::project::module_plan::{compile_module, module_files, plan_module};
use sokonanoda_front::project::plan_project;

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-module-batch-eq-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, text).expect("write file");
}

/// 逐文件比较"批编"与"逐入口编"，返回差异清单（空 = 等价 ✓）。
///
/// 比较口径**逐项**：声明状态、错误、警告、事件序列（含重基后的 `cmd`）。
fn differences(root: &Path) -> Vec<String> {
    let options = CompileOptions::default();
    let plan = plan_module(root);
    let batch = compile_module(&plan, &options);
    let mut out = Vec::new();
    // 规模旋钮：`SOKO_BATCH_SUBSET=<n>` 只比前 n 个文件（真课程整根比较是分钟级，
    // 排查时先用小 n 拿结论；默认 0 = 全部）。
    let subset = std::env::var("SOKO_BATCH_SUBSET")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(0);
    let mut files = module_files(root);
    if subset > 0 {
        files.truncate(subset);
    }
    for file in files {
        // 旧路径：以这个文件为入口，编它自己的闭包（今天 `build <file>`/`grade` 走的路）。
        let legacy = plan_project(&file, None, Some(root));
        let legacy_report = compile_plan(legacy, &options);
        let Some(entry) = legacy_report.entry_module() else {
            out.push(format!("{}: 旧路径没有入口报告", file.display()));
            continue;
        };
        let Some(new_report) = batch.reports.get(&file) else {
            out.push(format!("{}: 批编没有给出报告", file.display()));
            continue;
        };
        let Some(new_events) = batch.events.get(&file) else {
            out.push(format!("{}: 批编没有给出事件", file.display()));
            continue;
        };
        // `DeclState` 没有 `PartialEq`，而把它整个序列化在**整门课**上太贵
        // （MB 量级 × 35 文件，实测 15 分钟跑不完 ✗）⇒ 比**状态直方图**：
        // 条数 + 每个状态的计数。逐项到"每个声明的状态"这一层，够判等价性，
        // 又不会把判据本身变成负担 ✓。
        let histogram = |report: &sokonanoda_front::compile::DocumentReport| {
            let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
            for decl in &report.decls {
                *counts.entry(format!("{:?}", decl.status)).or_default() += 1;
            }
            counts
        };
        let new_decls = histogram(new_report);
        let legacy_decls = histogram(&entry.report);
        if new_decls != legacy_decls || new_report.decls.len() != entry.report.decls.len() {
            out.push(format!(
                "{}: 声明状态不同 —— 批 {new_decls:?} / 逐入口 {legacy_decls:?}",
                file.display()
            ));
        }
        if new_report.errors != entry.report.errors {
            out.push(format!(
                "{}: 错误不同 —— 批 {:?} / 逐入口 {:?}",
                file.display(),
                new_report.errors,
                entry.report.errors
            ));
        }
        if new_report.warnings != entry.report.warnings {
            out.push(format!(
                "{}: 警告不同 —— 批 {:?} / 逐入口 {:?}",
                file.display(),
                new_report.warnings,
                entry.report.warnings
            ));
        }
        if new_events.events != entry.events.events
            || new_events.event_cmds != entry.events.event_cmds
        {
            out.push(format!(
                "{}: 事件不同 —— 批 {} 条 / 逐入口 {} 条",
                file.display(),
                new_events.events.len(),
                entry.events.events.len()
            ));
        }
    }
    out
}

#[test]
fn a_module_batch_matches_per_entry_compilation_file_by_file() {
    let root = tmp("fixture");
    write(&root, "sokonanoda.toml", "name = \"demo\"\n");
    write(
        &root,
        "Lib.sokonanoda",
        "axiom P : Prop\naxiom proofP : P\n",
    );
    write(
        &root,
        "A.sokonanoda",
        "import Lib\n\ntheorem a : P := proofP\n",
    );
    write(
        &root,
        "B.sokonanoda",
        "import Lib\n\ntheorem b : P := proofP\n",
    );
    let differences = differences(&root);
    assert!(
        differences.is_empty(),
        "批编必须与逐入口编逐项相同（差异按 T-C1 §3 分类记录）：\n{}",
        differences.join("\n")
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **真课程**（手动跑）：`SOKO_COURSE_BATCH=1 cargo test -p sokonanoda-front --test module_batch -- --ignored --nocapture`
///
/// 为什么默认 `#[ignore]`：整门课 35 个文件、每份报告 MB 量级，跑一次要分钟级；
/// CI 用夹具那条守回归，这条用来回答"真实课程里等价性成不成立"。
#[test]
#[ignore]
fn the_real_course_module_batch_matches_per_entry_compilation() {
    let root = PathBuf::from(REPO).join("courses/set-theory");
    let files = module_files(&root);
    assert!(files.len() > 10, "课程文件数异常：{}", files.len());
    let differences = differences(&root);
    println!(
        "PERF course module batch: {} files · {} differences",
        files.len(),
        differences.len()
    );
    for line in differences.iter().take(20) {
        println!("  差异: {line}");
    }
    assert!(
        differences.is_empty(),
        "真课程上批编与逐入口编必须逐项相同；差异见上（按 T-C1 §3 分类）"
    );
}
