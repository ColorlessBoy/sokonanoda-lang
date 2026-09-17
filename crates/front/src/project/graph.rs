//! 闭包加载：从入口出发按 `import` 建图，产出拓扑序与项目级诊断。
//!
//! 规则（设计 `docs/design/imports-and-projects.md` §4.5）：
//!
//! * 深度优先、后序 = **依赖在前、入口在最后**（声明入 `EnvBuilder` 的顺序）；
//! * 重复 import 只算一次；成环报 `import-cycle` 并把环打印出来；
//! * 某个模块自己失败（找不到 / 解析失败）⇒ 它的**下游一律不编译**，
//!   只在下游的 `import` 行上报一条 `import-dependency-failed`
//!   （不阻断会级联出几十条"未定义标识符"，掩盖真正的第一因）。

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::module_name::ModuleName;
use super::report::{ProjectDiagnostic, ProjectKind};
use super::resolve::{resolve_module, Lookup};
use crate::compile::ErrorKind;
use crate::{parse, FolFile, Span};

/// 一个已解析的模块。
pub struct LoadedModule {
    pub name: String,
    pub path: PathBuf,
    pub file: FolFile,
    /// `import` 边（模块名 + 该 `import` 命令的 span），书写顺序、按名字去重。
    pub imports: Vec<(String, Span)>,
    /// 该模块自身有没有失败（找不到 / 解析失败 / 环）。
    pub failed: bool,
    /// 被依赖拖累（上游失败），因此不编译。
    pub blocked: bool,
}

/// 闭包加载的结果。
pub struct Closure {
    /// 拓扑序（依赖在前、入口在最后）。
    pub modules: Vec<LoadedModule>,
    pub diagnostics: Vec<ProjectDiagnostic>,
    /// 入口模块名。
    pub entry: String,
}

impl Closure {
    /// 可以编译的拓扑序模块下标（没有被阻断的）。
    pub fn compilable(&self) -> Vec<usize> {
        self.modules
            .iter()
            .enumerate()
            .filter(|(_, module)| !module.failed && !module.blocked)
            .map(|(index, _)| index)
            .collect()
    }

    pub fn module_index(&self, name: &str) -> Option<usize> {
        self.modules.iter().position(|module| module.name == name)
    }
}

/// 从 `entry` 出发加载整个闭包。
///
/// `entry_path` 用于推导入口模块名（`--text` 时可传一个约定的占位路径）；
/// `entry_src` 是入口的源文本（`None` = 从磁盘读）。
pub fn load_closure(root: &Path, entry_path: &Path, entry_src: Option<&str>) -> Closure {
    let entry_name = module_name_of_path(root, entry_path);
    let mut modules: Vec<LoadedModule> = Vec::new();
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let mut diagnostics: Vec<ProjectDiagnostic> = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    let mut visiting: HashSet<String> = HashSet::new();
    visit(
        root,
        &entry_name,
        entry_path,
        entry_src,
        &mut modules,
        &mut by_name,
        &mut diagnostics,
        &mut stack,
        &mut visiting,
    );

    let mut closure = Closure {
        modules,
        diagnostics,
        entry: entry_name,
    };
    let mut extra: Vec<ProjectDiagnostic> = Vec::new();
    propagate_blocked(&mut closure, &mut extra);
    closure.diagnostics.append(&mut extra);
    closure
}

/// 阻断传播（可重复调用）：上游失败 ⇒ 下游不编译，并在下游的 `import` 行
/// 报一条 `import-dependency-failed`。拓扑序保证"上游先被判定"。
pub fn propagate_blocked(closure: &mut Closure, diagnostics: &mut Vec<ProjectDiagnostic>) {
    let mut broken: BTreeSet<String> = closure
        .modules
        .iter()
        .filter(|module| module.failed)
        .map(|module| module.name.clone())
        .collect();
    for index in 0..closure.modules.len() {
        if closure.modules[index].failed {
            continue;
        }
        let deps: Vec<(String, Span)> = closure.modules[index].imports.clone();
        let path = closure.modules[index].path.clone();
        let name = closure.modules[index].name.clone();
        for (dep, span) in deps {
            if broken.contains(&dep) && !closure.modules[index].blocked {
                closure.modules[index].blocked = true;
                broken.insert(name.clone());
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::Error(ErrorKind::ImportDependencyFailed),
                    message: format!(
                        "`{dep}` 没有编译成功，所以这里先不编译——先修好它的第一条错误"
                    ),
                    module: name.clone(),
                    path: Some(path.clone()),
                    span: Some(span),
                });
            }
        }
    }
}

/// 入口模块名：模块根下的相对路径去掉扩展名、`/` 换成 `.`；
/// 入口不在模块根下（`--text` / 根外文件）时退回文件主名。
pub fn module_name_of_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).ok();
    let stem_path = relative.unwrap_or(path);
    let without_ext = stem_path.with_extension("");
    let text = without_ext.to_string_lossy().replace('\\', "/");
    let text = text.trim_start_matches("./").trim_end_matches('/');
    if relative.is_none() {
        // 根外/占位路径：只用文件主名（不做点分展开，避免把绝对路径当模块名）。
        return stem_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Main".to_string());
    }
    if text.is_empty() || text.contains("..") {
        return "Main".to_string();
    }
    text.replace('/', ".")
}

/// 一次 `visit` 的结果：调用方据此生成**带正确 span** 的环诊断。
enum VisitOutcome {
    /// 已加载（或本次加载完成）。
    Loaded,
    /// 这条边回到栈上的某个模块——环。
    Cycle(Vec<String>),
}

#[allow(clippy::too_many_arguments)]
fn visit(
    root: &Path,
    name: &str,
    path: &Path,
    src: Option<&str>,
    modules: &mut Vec<LoadedModule>,
    by_name: &mut HashMap<String, usize>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
    stack: &mut Vec<String>,
    visiting: &mut HashSet<String>,
) -> VisitOutcome {
    if by_name.contains_key(name) {
        return VisitOutcome::Loaded; // 重复 import 只算一次
    }
    if visiting.contains(name) {
        let from = stack.iter().position(|item| item == name).unwrap_or(0);
        let mut cycle = stack[from..].to_vec();
        cycle.push(name.to_string());
        return VisitOutcome::Cycle(cycle);
    }

    // 读源文本（入口可以用调用方给的中间态文本）。
    let text = match src {
        Some(text) => text.to_string(),
        None => match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::Error(ErrorKind::ImportNotFound),
                    message: format!("读不到模块文件 {}：{err}", path.display()),
                    module: name.to_string(),
                    path: Some(path.to_path_buf()),
                    span: None,
                });
                push_module(
                    modules,
                    by_name,
                    name,
                    path,
                    FolFile {
                        commands: Vec::new(),
                        src: String::new(),
                    },
                    Vec::new(),
                    true,
                );
                return VisitOutcome::Loaded;
            }
        },
    };

    let file = match parse(&text) {
        Ok(file) => file,
        Err(diagnostic) => {
            diagnostics.push(ProjectDiagnostic {
                kind: ProjectKind::Error(ErrorKind::ImportModuleInvalid),
                message: format!("{}（{name}）", diagnostic.message),
                module: name.to_string(),
                path: Some(path.to_path_buf()),
                span: Some(diagnostic.span),
            });
            push_module(
                modules,
                by_name,
                name,
                path,
                FolFile {
                    commands: Vec::new(),
                    src: text,
                },
                Vec::new(),
                true,
            );
            return VisitOutcome::Loaded;
        }
    };

    // 收集 import 边（书写顺序、按名字去重）。
    let mut imports: Vec<(String, Span)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for command in &file.commands {
        let Some(raw) = command.import_module() else {
            continue;
        };
        if seen.insert(raw.to_string()) {
            imports.push((raw.to_string(), command.span()));
        }
    }

    stack.push(name.to_string());
    visiting.insert(name.to_string());
    let mut module_failed = false;
    for (dep, span) in &imports {
        // 模块名非法（解析期已经拦过，这里防御性处理）。
        let Ok(dep_name) = ModuleName::parse(dep) else {
            diagnostics.push(ProjectDiagnostic {
                kind: ProjectKind::Error(ErrorKind::ImportNotFound),
                message: format!("`{dep}` 不是合法的模块名"),
                module: name.to_string(),
                path: Some(path.to_path_buf()),
                span: Some(*span),
            });
            module_failed = true;
            continue;
        };
        if dep_name.as_str() == name {
            diagnostics.push(ProjectDiagnostic {
                kind: ProjectKind::Error(ErrorKind::ImportCycle),
                message: format!("import 成环：{name} → {name}"),
                module: name.to_string(),
                path: Some(path.to_path_buf()),
                span: Some(*span),
            });
            module_failed = true;
            continue;
        }
        match resolve_module(root, &dep_name) {
            Lookup::Found(found) => {
                match visit(
                    root,
                    &dep_name.as_str(),
                    &found,
                    None,
                    modules,
                    by_name,
                    diagnostics,
                    stack,
                    visiting,
                ) {
                    VisitOutcome::Loaded => {}
                    VisitOutcome::Cycle(cycle) => {
                        diagnostics.push(ProjectDiagnostic {
                            kind: ProjectKind::Error(ErrorKind::ImportCycle),
                            message: format!("import 成环：{}", cycle.join(" → ")),
                            module: name.to_string(),
                            path: Some(path.to_path_buf()),
                            span: Some(*span),
                        });
                        module_failed = true;
                    }
                }
            }
            Lookup::Missing(missing) => {
                let mut message = format!(
                    "找不到模块 `{}`（期望 {}）",
                    missing.module,
                    missing.expected.display()
                );
                if let Some(case_hit) = &missing.case_mismatch {
                    message.push_str(&format!(
                        "；磁盘上是 `{}`——模块名大小写必须完全一致",
                        case_hit
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    ));
                }
                if !missing.dash_candidates.is_empty() {
                    let names: Vec<String> = missing
                        .dash_candidates
                        .iter()
                        .filter_map(|candidate| {
                            candidate
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                        })
                        .collect();
                    message.push_str(&format!(
                        "；文件名的 `-` 不是模块名字符，你是不是想 import {}？",
                        names.join(" / ")
                    ));
                }
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::Error(ErrorKind::ImportNotFound),
                    message,
                    module: name.to_string(),
                    path: Some(path.to_path_buf()),
                    span: Some(*span),
                });
                module_failed = true;
            }
        }
    }
    stack.pop();
    visiting.remove(name);

    // **后序登记**：依赖已经在 `modules` 里，入口最后（拓扑序）。
    push_module(modules, by_name, name, path, file, imports, module_failed);
    VisitOutcome::Loaded
}

#[allow(clippy::too_many_arguments)]
fn push_module(
    modules: &mut Vec<LoadedModule>,
    by_name: &mut HashMap<String, usize>,
    name: &str,
    path: &Path,
    file: FolFile,
    imports: Vec<(String, Span)>,
    failed: bool,
) {
    let index = modules.len();
    modules.push(LoadedModule {
        name: name.to_string(),
        path: path.to_path_buf(),
        file,
        imports,
        failed,
        blocked: false,
    });
    by_name.insert(name.to_string(), index);
}
