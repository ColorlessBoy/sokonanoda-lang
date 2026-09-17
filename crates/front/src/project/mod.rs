//! 项目层入口：清单发现 → 闭包加载 → 一次编译 → 逐模块报告。
//!
//! 设计：`docs/design/imports-and-projects.md`（ROADMAP I16）。要点：
//!
//! * **零配置退路**：没有清单时模块根 = 入口文件所在目录（刻意偏离真实
//!   `lean`：官方搜索路径里没有文件自己的目录，cwd 只影响模块名——见 §2.1）；
//! * **一次编译**：所有模块按拓扑序进**同一个 arena / 同一个 `EnvBuilder`**，
//!   内核一行不改；
//! * 入口文件的报告可以直接交给既有的单文档消费者（`cmd` 已重基）。

pub mod graph;
pub mod manifest;
pub mod module_name;
pub mod report;
pub mod resolve;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::compile::{
    compile_all_units, explicit_prelude_mode, unit_ranges, CompileOptions, CompileOutput,
    DeclStatus, DocumentReport, ErrorKind, PreludeMode, SourceUnit, WarningKind, PRELUDE_NAMES,
};

pub use graph::{load_closure, Closure, LoadedModule};
pub use manifest::{find_manifest, Manifest, MANIFEST_FILE};
pub use module_name::{ModuleName, ModuleNameError, DASH_HINT, MODULE_EXTENSION};
pub use report::{ModuleReport, ProjectDiagnostic, ProjectKind, ProjectReport};
pub use resolve::{resolve_module, Lookup, MissingModule};

/// Full 模式下由 prelude 安装、因而**不能被普通声明占用**的名字
/// （`Eq` 一族例外：它整体让位，见 `install_eq_prelude`）。
const PRELUDE_OWNED: &[&str] = &[
    "Nat",
    "Nat.zero",
    "Nat.succ",
    "Nat.rec",
    "Nat.add",
    "Bool",
    "Bool.true",
    "Bool.false",
    "Bool.rec",
];

/// 编译一个项目闭包。
///
/// * `entry_path`：入口文件路径（`--text` 时给一个约定的占位路径）；
/// * `entry_src`：入口的中间态文本（`None` = 从磁盘读）；
/// * `root_override`：`--root` 显式指定的模块根（跳过清单发现）。
pub fn compile_project(
    entry_path: &Path,
    entry_src: Option<&str>,
    options: &CompileOptions,
    root_override: Option<&Path>,
) -> ProjectReport {
    compile_plan(plan_project(entry_path, entry_src, root_override), options)
}

/// 一次项目编译的**计划**：根、清单、闭包（都只做了读取与解析）。
///
/// 拆出来是为了缓存：闭包哈希必须在**编译之前**算出来（`plan.digest()`），
/// 命中就整个跳过内核；而计划本身只做 IO 与 parse，反复算也不贵。
pub struct ProjectPlan {
    pub entry: PathBuf,
    pub root: PathBuf,
    pub manifest: Option<PathBuf>,
    pub requires_warning: Option<String>,
    /// 加载期诊断（找不到/环/语法错误），编译期诊断由 `compile_plan` 追加。
    pub diagnostics: Vec<ProjectDiagnostic>,
    closure: Closure,
}

impl ProjectPlan {
    pub fn modules(&self) -> &[LoadedModule] {
        &self.closure.modules
    }

    pub fn entry(&self) -> &str {
        &self.closure.entry
    }

    /// 闭包摘要：**所有模块的源文本按拓扑序** + import 边 + prelude 模式。
    /// 依赖变了 ⇒ 摘要变 ⇒ 入口的缓存键变（设计 §4.8 的 Merkle 链）。
    /// 单文件（无 import）不走这条路：它用既有的"源文本"键。
    pub fn digest(&self, options: &CompileOptions) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100_0000_01b3);
            }
        };
        mix(b"soko.project-iface/1");
        mix(&[match options.prelude {
            PreludeMode::Full => 1,
            PreludeMode::Bare => 2,
        }]);
        for module in &self.closure.modules {
            mix(module.name.as_bytes());
            mix(b"\0");
            mix(module.file.src.as_bytes());
            mix(b"\0");
            for (dep, _) in &module.imports {
                mix(dep.as_bytes());
                mix(b",");
            }
            mix(b"\n");
        }
        format!("closure-{hash:016x}")
    }
}

/// 解析项目根、加载闭包（不编译）。
pub fn plan_project(
    entry_path: &Path,
    entry_src: Option<&str>,
    root_override: Option<&Path>,
) -> ProjectPlan {
    let entry_dir = entry_path
        .parent()
        .map(Path::to_path_buf)
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    let entry_label = entry_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Main".to_string());
    let mut diagnostics: Vec<ProjectDiagnostic> = Vec::new();

    // 1) 项目根：显式覆盖 > 最近祖先清单 > 入口文件目录（零配置退路）。
    let (root, manifest_path, requires_warning) = match root_override {
        Some(root) => (root.to_path_buf(), None, None),
        None => match find_manifest(&entry_dir) {
            Some(path) => match manifest::load(&path) {
                Ok(manifest) => {
                    let root = manifest::module_root(&path, &manifest);
                    (root, Some(path), manifest::version_warning(&manifest))
                }
                Err(err) => {
                    diagnostics.push(ProjectDiagnostic {
                        kind: ProjectKind::Error(ErrorKind::ManifestInvalid),
                        message: format!("{}：{}", err.path.display(), err.message),
                        module: entry_label.clone(),
                        path: Some(err.path.clone()),
                        span: None,
                    });
                    (entry_dir.clone(), Some(path), None)
                }
            },
            None => (entry_dir.clone(), None, None),
        },
    };

    // 2) 闭包加载（解析 + 环 + 找不到 + 阻断传播）。
    let closure = load_closure(&root, entry_path, entry_src);

    ProjectPlan {
        entry: entry_path.to_path_buf(),
        root,
        manifest: manifest_path,
        requires_warning,
        diagnostics,
        closure,
    }
}

/// 执行计划：闭包级检查 → 一次编译 → 逐模块报告 → 挂诊断。
pub fn compile_plan(mut plan: ProjectPlan, options: &CompileOptions) -> ProjectReport {
    let entry_path = plan.entry.clone();
    let root = plan.root.clone();
    let manifest_path = plan.manifest.clone();
    let requires_warning = plan.requires_warning.clone();
    let mut diagnostics = std::mem::take(&mut plan.diagnostics);
    let mut closure = plan.closure;
    diagnostics.append(&mut closure.diagnostics);

    // 3) 闭包级检查：重名 + prelude 冲突（都在入内核之前拦下，避免内核文案）。
    check_name_collisions(&mut closure, &mut diagnostics);
    check_prelude_conflicts(&closure, options, &mut diagnostics);

    // 4) 只编译未被阻断的模块（拓扑序，入口在最后）。
    let compilable = closure.compilable();
    let units: Vec<SourceUnit<'_>> = compilable
        .iter()
        .map(|&index| SourceUnit {
            name: &closure.modules[index].name,
            path: Some(closure.modules[index].path.as_path()),
            file: &closure.modules[index].file,
        })
        .collect();
    let (flat_out, reports) = compile_all_units(&units, options);

    // 5) 逐模块事件（扁平事件按单元区间切分，`cmd` 重基到模块内）。
    //    被阻断的模块不编译，但**仍然出现在报告里**（入口永远在最后，
    //    否则入口的阻塞诊断无处安放）。
    let ranges = unit_ranges(&units);
    let mut compiled: HashMap<usize, usize> = compilable
        .iter()
        .enumerate()
        .map(|(slot, &index)| (index, slot))
        .collect();
    let mut modules: Vec<ModuleReport> = Vec::with_capacity(closure.modules.len());
    for (index, module) in closure.modules.iter().enumerate() {
        let imports: Vec<String> = module
            .imports
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        match compiled.remove(&index) {
            Some(slot) => {
                let range = ranges[slot].clone();
                let mut events = CompileOutput::default();
                for (position, event) in flat_out.events.iter().enumerate() {
                    let cmd = flat_out.event_cmds[position];
                    if range.contains(&cmd) {
                        events.events.push(event.clone());
                        events.event_cmds.push(cmd - range.start);
                    }
                }
                events.errors = reports[slot].errors.clone();
                events.warnings = reports[slot].warnings.clone();
                modules.push(ModuleReport {
                    name: module.name.clone(),
                    path: module.path.clone(),
                    imports,
                    report: reports[slot].clone(),
                    events,
                });
            }
            None => modules.push(ModuleReport {
                name: module.name.clone(),
                path: module.path.clone(),
                imports,
                report: DocumentReport::default(),
                events: CompileOutput::default(),
            }),
        }
    }

    // 6) 开放练习提示：被导入模块里的 `sorry` 对下游不可见，值得在 import 行上说一句。
    collect_open_exercise_warnings(&closure, &modules, &mut diagnostics);

    // 7) 依赖的**编译失败**同样阻断下游（check-then-add 的跨模块版本）：
    //    一个非入口模块的报告里有错误 ⇒ 所有（传递）import 它的模块不保留
    //    编译结果，只在 import 行上留一条 `import-dependency-failed`。
    //    v1 是"全编译后再丢弃"：闭包共用一次 arena/builder，教学规模下这点浪费
    //    可以接受；等 P7 有 decl 级产物时再做真正的早停。
    let result_blocked = block_on_compile_failures(&closure, &modules, &mut diagnostics);
    for module in &mut modules {
        if result_blocked.contains(&module.name) {
            module.report = DocumentReport::default();
            module.events = CompileOutput::default();
        }
    }

    let mut project = ProjectReport {
        entry: entry_path,
        root,
        manifest: manifest_path,
        modules,
        diagnostics,
        requires_warning,
    };
    project.attach_diagnostics();
    project
}

/// 顶层名字在闭包里必须唯一：第二个声明它的模块报 `import-name-collision`。
fn check_name_collisions(closure: &mut Closure, diagnostics: &mut Vec<ProjectDiagnostic>) {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut newly_failed: Vec<usize> = Vec::new();
    for index in closure.compilable() {
        let module = &closure.modules[index];
        let name = module.name.clone();
        let path = module.path.clone();
        for (declared, span) in crate::compile::top_level_def_spans(&module.file) {
            // prelude 的名字由 prelude 自己占（另有专门的冲突检查）。
            if PRELUDE_NAMES.contains(&declared.as_str()) {
                continue;
            }
            match seen.get(&declared) {
                Some(first) if *first != name => {
                    diagnostics.push(ProjectDiagnostic {
                        kind: ProjectKind::Error(ErrorKind::ImportNameCollision),
                        message: format!("`{declared}` 在 `{first}` 与 `{name}` 里各声明了一次"),
                        module: name.clone(),
                        path: Some(path.clone()),
                        span: Some(span),
                    });
                    newly_failed.push(index);
                    break;
                }
                Some(_) => {}
                None => {
                    seen.insert(declared, name.clone());
                }
            }
        }
    }
    if !newly_failed.is_empty() {
        for index in newly_failed {
            closure.modules[index].failed = true;
        }
        graph::propagate_blocked(closure, diagnostics);
    }
}

/// Full 模式下依赖模块不得占用 prelude 自己的名字（`Eq` 一族除外），
/// 也不得与入口的 prelude 模式不一致。
fn check_prelude_conflicts(
    closure: &Closure,
    options: &CompileOptions,
    diagnostics: &mut Vec<ProjectDiagnostic>,
) {
    let mut conflicts: Vec<(String, String)> = Vec::new(); // (module, 说明)
    for index in closure.compilable() {
        let module = &closure.modules[index];
        if module.name == closure.entry {
            continue; // 入口保持今天单文件语义（内核行为不变）
        }
        // 只有**显式**写了指令且与入口不一致才算冲突；没写 = 继承入口的模式。
        if explicit_prelude_mode(&module.file.src).is_some_and(|mode| mode != options.prelude) {
            conflicts.push((
                module.name.clone(),
                "它的 `-- sokonanoda:prelude` 指令与入口不一致".to_string(),
            ));
            continue;
        }
        if options.prelude != PreludeMode::Full {
            continue;
        }
        for (declared, _) in crate::compile::top_level_def_spans(&module.file) {
            if PRELUDE_OWNED.contains(&declared.as_str())
                && !owns_inductive(&module.file, &declared)
            {
                conflicts.push((module.name.clone(), format!("它声明了 `{declared}`")));
                break;
            }
        }
    }
    for (module_name, what) in conflicts {
        for module in &closure.modules {
            for (dep, span) in &module.imports {
                if *dep != module_name {
                    continue;
                }
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::Error(ErrorKind::ImportPreludeConflict),
                    message: format!(
                        "模块 `{module_name}` 与内置 prelude 冲突：{what}（prelude 是整个编译单元的属性）"
                    ),
                    module: module.name.clone(),
                    path: Some(module.path.clone()),
                    span: Some(*span),
                });
            }
        }
    }
}

/// `inductive Nat`/`inductive Bool` 及其成员不算"占用 prelude 名字"
/// （闭包级让位规则本来就是为这种共享 prelude 模块设计的）。
fn owns_inductive(file: &crate::FolFile, name: &str) -> bool {
    let root = name.split('.').next().unwrap_or(name);
    file.commands.iter().any(
        |command| matches!(command, crate::Command::InductiveBlock { name, .. } if name == root),
    )
}

/// 依赖模块的编译失败（elab/kernel 错误）⇒ 下游不保留结果，并逐条
/// `import` 边报一条 `import-dependency-failed`。返回被阻断的模块名集合。
fn block_on_compile_failures(
    closure: &Closure,
    modules: &[ModuleReport],
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> HashSet<String> {
    // "坏"模块 = 非入口、且报告里有错误的模块（警告不算）。
    let broken: HashSet<String> = modules
        .iter()
        .filter(|module| module.name != closure.entry && !module.report.errors.is_empty())
        .map(|module| module.name.clone())
        .collect();
    if broken.is_empty() {
        return broken;
    }
    let mut blocked: HashSet<String> = HashSet::new();
    // 拓扑序保证"上游先被判定"。
    for module in &closure.modules {
        if broken.contains(&module.name) || blocked.contains(&module.name) {
            continue;
        }
        let mut hit = false;
        for (dep, span) in &module.imports {
            if broken.contains(dep) || blocked.contains(dep) {
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::Error(ErrorKind::ImportDependencyFailed),
                    message: format!(
                        "`{dep}` 没有编译成功，所以这里先不编译——先修好它的第一条错误"
                    ),
                    module: module.name.clone(),
                    path: Some(module.path.clone()),
                    span: Some(*span),
                });
                hit = true;
            }
        }
        if hit {
            blocked.insert(module.name.clone());
        }
    }
    blocked
}

/// 被导入模块里还有 `sorry` ⇒ 在下游的 `import` 行给一条 warning。
fn collect_open_exercise_warnings(
    closure: &Closure,
    modules: &[ModuleReport],
    diagnostics: &mut Vec<ProjectDiagnostic>,
) {
    let open: HashSet<&str> = modules
        .iter()
        .filter(|module| {
            module
                .report
                .decls
                .iter()
                .any(|decl| decl.status == DeclStatus::Open)
        })
        .map(|module| module.name.as_str())
        .collect();
    for module in &closure.modules {
        for (dep, span) in &module.imports {
            if open.contains(dep.as_str()) {
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::Warning(WarningKind::ImportHasOpenExercises),
                    message: format!("`{dep}` 里还有未完成的练习（`sorry`）：它们对下游不可见"),
                    module: module.name.clone(),
                    path: Some(module.path.clone()),
                    span: Some(*span),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
