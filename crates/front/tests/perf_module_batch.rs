//! **模块级批编 vs 逐入口编**的量本（阶段 C / T-C5 的前哨）。
//!
//! 为什么要有它：`build <dir>` 的基线是 O(文件数 × 闭包)（`courses/set-theory`
//! 实测 146.07s / 35 文件）。批编把"共享依赖只 elaborate 一次"，但**平坦环境会变大**
//! ——后编的单元要在一个更长的前缀上工作，理论上可能超线性 ✗。这条哨兵用**生成的项目**
//! （可控规模、零仓库依赖）量两件事：
//! * `per_entry_ms`：逐文件各编一次自己的闭包（今天的路径）的总和；
//! * `batch_ms`：`plan_module` + `compile_module` 一次批编的总和。
//!
//! 打印 `PERF module batch …` 与 `PERFJSON`（`scripts/perf-ledger.sh` 收 `^PERF`）。

use std::path::{Path, PathBuf};
use std::time::Instant;

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::compile_plan;
use sokonanoda_front::project::module_plan::{compile_module, module_files, plan_module};
use sokonanoda_front::project::plan_project;

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-perf-module-batch-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// 一个"共享库 + N 个单元"的项目：每个单元都 `import Lib`，单元之间互不依赖
/// （课程的真实形状：`lib/*` 被所有 unit 共享）。
fn gen_project(dir: &Path, units: usize, decls: usize) {
    let mut lib = String::from("axiom P : Prop\naxiom proofP : P\n");
    for i in 0..decls {
        lib.push_str(&format!("theorem lib_s{i} : P := proofP\n"));
    }
    std::fs::write(dir.join("sokonanoda.toml"), "name = \"batch\"\n").unwrap();
    std::fs::write(dir.join("Lib.sokonanoda"), lib).unwrap();
    for unit in 0..units {
        let mut text = String::from("import Lib\n\n");
        for i in 0..decls {
            text.push_str(&format!("theorem u{unit}_s{i} : P := proofP\n"));
        }
        std::fs::write(dir.join(format!("U{unit}.sokonanoda")), text).unwrap();
    }
}

fn ms(started: Instant) -> f64 {
    (started.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

#[test]
fn module_batch_is_not_slower_than_per_entry_compilation() {
    let dir = tmp("scaling");
    let units = 8;
    let decls = 12;
    gen_project(&dir, units, decls);
    let options = CompileOptions::default();

    // ① 逐入口：每个文件各编一次自己的闭包（今天 `build <dir>` 做的事）。
    let started = Instant::now();
    let files = module_files(&dir);
    let mut per_entry_files = 0usize;
    for file in &files {
        let plan = plan_project(file, None, Some(&dir));
        let report = compile_plan(plan, &options);
        if report.entry_module().is_some() {
            per_entry_files += 1;
        }
    }
    let per_entry_ms = ms(started);

    // ② 批编：一次。
    let started = Instant::now();
    let plan = plan_module(&dir);
    let batch = compile_module(&plan, &options);
    let batch_ms = ms(started);

    println!(
        "PERF module batch: {} files ({} units × {} decls) · per-entry {per_entry_ms:.1}ms · batch {batch_ms:.1}ms · ratio {:.2}×",
        per_entry_files,
        units,
        decls,
        per_entry_ms / batch_ms.max(0.001)
    );
    println!(
        "PERFJSON {}",
        serde_json::json!({
            "schema": "soko.perf/1",
            "scope": "front-module-batch",
            "case": "batch_vs_per_entry",
            "units": units,
            "decls_per_unit": decls,
            "files": per_entry_files,
            "per_entry_ms": per_entry_ms,
            "batch_ms": batch_ms,
        })
    );
    assert_eq!(batch.reports.len(), files.len(), "每个文件都要有报告");
    // 不判"必须更快"（那是 T-C5 的结论，要真课程数字），但**不许出现灾难性退化**：
    // 批编比逐入口慢 1.5 倍以上就说明平坦环境的前缀代价失控 ✗，要停下来看。
    assert!(
        batch_ms <= per_entry_ms * 1.5,
        "批编不该显著慢于逐入口（per-entry {per_entry_ms:.1}ms vs batch {batch_ms:.1}ms）"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
