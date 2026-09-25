//! **模块级批量编译**（阶段 C / T-C2）：把模块根下的**全部** `.sokonanoda` 文件
//! 当**一批**编一次，并给出**逐文件**报告。
//!
//! 契约与等价性口径见 `docs/design/module-batch.md`（T-C1）。一句话：
//! `build <dir>` 今天对每个文件各编一遍**完整闭包** ⇒ O(文件数 × 闭包)；
//! 这里改成"取所有文件闭包的**并集**、按路径去重、依赖先、**编一次**"，
//! 共享依赖只 elaborate 一次 —— 省下来的就是这部分。
//!
//! **与既有路径的关系**：单文件/单入口路径一行不改（`import`-free 文件零项目开销）；
//! 本模块是**新增**入口，`build` 是否使用它由开关决定（T-C4，默认关）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::compile::{
    compile_all_units, CompileOptions, CompileOutput, DocumentReport, SourceUnit,
};
use crate::project::{plan_project, ProjectDiagnostic};
use crate::FolFile;

/// 模块根下的**产物目录名**（枚举时跳过它：产物绝不是源文件，见 T-B4 §3.3）。
pub const ARTIFACTS_DIR: &str = ".sokonanoda";

/// 批里的一个单元（= 一个文件 + 它的模块名）。
pub struct PlannedUnit {
    pub path: PathBuf,
    pub name: String,
    pub file: FolFile,
}

/// 模块级计划：根 + **拓扑序**（依赖先、平手按路径）去重后的单元 + 加载期诊断。
pub struct ModulePlan {
    pub root: PathBuf,
    pub units: Vec<PlannedUnit>,
    /// 各文件闭包加载期的诊断（缺依赖/环/解析失败），按路径去重。
    pub diagnostics: Vec<ProjectDiagnostic>,
}

/// 批编结果：**逐文件**报告 + 扁平事件流。
pub struct ModuleBatch {
    pub reports: BTreeMap<PathBuf, DocumentReport>,
    pub output: CompileOutput,
}

/// 模块根下的源文件清单：递归、**排序**、**跳过 `.sokonanoda/`**。
///
/// 为什么必须显式跳过：`build` 今天的 `collect_files`（`crates/cli/src/build.rs:172`）
/// **不跳隐藏目录**，而产物目录（R-3）就在模块根下 ⇒ 少了这一条，产物会被当成源文件
/// 再编一遍（自我吞掉）。扩展名那道防线（产物恒为 `*.json`）是第二道保险。
pub fn module_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = read.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == ARTIFACTS_DIR) {
                continue;
            }
            collect(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == super::MODULE_EXTENSION)
        {
            out.push(path);
        }
    }
}

/// 规划一批：枚举 → 逐文件取**自己的闭包** → 合并去重（**保拓扑序**）→ 定序。
///
/// 为什么逐闭包合并仍然保拓扑序：每个闭包内部本来就是拓扑序（依赖先、入口最后），
/// 而"先出现的先入列"只可能把一个单元**提前**——若 X 排在它的依赖 Y 前面，说明 X 在
/// 更早那趟闭包里先出现，可那一趟里 Y 是 X 的传递依赖、本该更早入列 ⇒ 矛盾。
pub fn plan_module(root: &Path) -> ModulePlan {
    let mut units: Vec<PlannedUnit> = Vec::new();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut diagnostics: Vec<ProjectDiagnostic> = Vec::new();
    for file in module_files(root) {
        let plan = plan_project(&file, None, Some(root));
        for diagnostic in &plan.diagnostics {
            if !diagnostics.iter().any(|known| {
                known.module == diagnostic.module && known.message == diagnostic.message
            }) {
                diagnostics.push(diagnostic.clone());
            }
        }
        for module in plan.modules() {
            if !seen.insert(module.path.clone()) {
                continue;
            }
            units.push(PlannedUnit {
                path: module.path.clone(),
                name: module.name.clone(),
                file: module.file.clone(),
            });
        }
    }
    ModulePlan {
        root: root.to_path_buf(),
        units,
        diagnostics,
    }
}

/// 批编**一次**，把结果按单元切回**逐文件**报告（键是文件路径）。
///
/// 逐单元报告与 `units` 同序（`compile_all_units` 的契约），这里只做"路径 → 报告"的
/// 归位；等价性（与"以该文件为入口单独编"逐项相同）由 T-C3 判。
pub fn compile_module(plan: &ModulePlan, options: &CompileOptions) -> ModuleBatch {
    let units: Vec<SourceUnit<'_>> = plan
        .units
        .iter()
        .map(|unit| SourceUnit {
            name: &unit.name,
            path: Some(&unit.path),
            file: &unit.file,
        })
        .collect();
    let (output, reports) = compile_all_units(&units, options);
    let per_file = plan
        .units
        .iter()
        .map(|unit| unit.path.clone())
        .zip(reports)
        .collect();
    ModuleBatch {
        reports: per_file,
        output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-module-batch-{tag}-{}",
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

    /// **T-C2 判据①**：枚举递归、排序、**跳过 `.sokonanoda/`**（R-3 的红线）。
    #[test]
    fn module_files_are_sorted_and_skip_the_artifacts_directory() {
        let root = tmp("files");
        write(&root, "B.sokonanoda", "axiom Q : Prop\n");
        write(&root, "sub/A.sokonanoda", "axiom P : Prop\n");
        // 产物目录：里面的东西**绝不能**被当成源文件。
        write(&root, ".sokonanoda/compiled/deadbeef.json", "{}");
        write(
            &root,
            ".sokonanoda/README.sokonanoda",
            "axiom Bogus : Prop\n",
        );

        let files = module_files(&root);
        let names: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(names, vec!["B.sokonanoda", "sub/A.sokonanoda"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **T-C2 判据②**：共享依赖**只出现一次**（既是入口又是依赖的文件去重），
    /// 且顺序是拓扑序（依赖先于依赖者）。
    #[test]
    fn plan_module_dedups_shared_dependencies_and_keeps_topological_order() {
        let root = tmp("dedup");
        write(&root, "sokonanoda.toml", "name = \"demo\"\n");
        write(
            &root,
            "Lib.sokonanoda",
            "axiom P : Prop\naxiom proofP : P\n",
        );
        write(
            &root,
            "B.sokonanoda",
            "import Lib\n\ntheorem b : P := proofP\n",
        );
        write(
            &root,
            "C.sokonanoda",
            "import Lib\n\ntheorem c : P := proofP\n",
        );

        let plan = plan_module(&root);
        let order: Vec<String> = plan.units.iter().map(|unit| unit.name.clone()).collect();
        assert_eq!(
            order,
            vec!["Lib", "B", "C"],
            "共享依赖只编一次、且排在依赖者前面：{order:?}"
        );
        assert_eq!(plan.units.len(), 3, "Lib 是 B 与 C 的依赖，但只该出现一次");

        // **批编一次** ⇒ 逐文件报告，键是路径；三个文件都有报告。
        let batch = compile_module(&plan, &CompileOptions::default());
        assert_eq!(batch.reports.len(), 3, "{:?}", batch.reports.keys());
        for unit in &plan.units {
            assert!(
                batch.reports.contains_key(&unit.path),
                "缺 {} 的报告",
                unit.path.display()
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
