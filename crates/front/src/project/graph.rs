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
use crate::ast::NotationDecl;
use crate::compile::ErrorKind;
use crate::parser::{parse_with_inherited, scan_import_lines};
use crate::FolFile;
use crate::Span;

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
    /// **入口可见的记法表**（T-D11）：符号 → 声明点（`span`）+ **声明它的模块名**
    /// （`module`）。继承来的那份保留它原来的模块名——那才是它的声明点。
    ///
    /// 以前这张表在加载期算完就丢（`exports` 是局部量）⇒ "跳转"没有数据可用。
    pub notations: Vec<NotationDecl>,
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
/// 内存覆盖：`(路径, 文本)`。命中时**不读盘**——编辑器里未保存的缓冲区就是
/// 编译器该看到的那份文本（LSP 跨文件失效的前提，I16 P5）。
///
/// 路径按 `canonicalize` 比较：同一个文件在 macOS 上可能是 `/var/...` 与
/// `/private/var/...` 两种写法，直接比字符串会漏命中。
pub type Overlay = Vec<(PathBuf, String)>;

pub fn load_closure(root: &Path, entry_path: &Path, entry_src: Option<&str>) -> Closure {
    load_closure_with_overlay(root, entry_path, entry_src, &[])
}

/// 带内存覆盖的闭包加载（LSP 用；`overlay` 为空时与 [`load_closure`] 等价）。
pub fn load_closure_with_overlay(
    root: &Path,
    entry_path: &Path,
    entry_src: Option<&str>,
    overlay: &[(PathBuf, String)],
) -> Closure {
    let entry_name = module_name_of_path(root, entry_path);
    let mut modules: Vec<LoadedModule> = Vec::new();
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let mut diagnostics: Vec<ProjectDiagnostic> = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    let mut visiting: HashSet<String> = HashSet::new();
    // 每个模块**导出**的记法表（自己声明的 + 从自己的 import 传递来的）：
    // 解析一个模块之前，把它各依赖的导出表并起来当继承表（G-04 第二刀 §10.3）。
    let mut exports: HashMap<String, Vec<NotationDecl>> = HashMap::new();
    visit(
        root,
        &entry_name,
        entry_path,
        entry_src,
        overlay,
        &mut modules,
        &mut by_name,
        &mut diagnostics,
        &mut stack,
        &mut visiting,
        &mut exports,
    );

    let notations = exports.remove(&entry_name).unwrap_or_default();
    let mut closure = Closure {
        modules,
        diagnostics,
        entry: entry_name,
        notations,
    };
    let mut extra: Vec<ProjectDiagnostic> = Vec::new();
    propagate_blocked(&mut closure, &mut extra);
    closure.diagnostics.append(&mut extra);
    closure
}

/// 把一个模块**自己**声明的记法并进继承表（同符号覆盖 = 遮蔽）。
fn absorb_notations(file: &FolFile, inherited: &mut Vec<NotationDecl>, module: &str) {
    for command in &file.commands {
        let Some(mut decl) = command.notation_decl() else {
            continue;
        };
        // **声明它的模块名**（T-D10）：这条命令自己不知道自己在哪个模块里，
        // 加载层知道。跨 `import` 传播时这个字段跟着走（继承来的那份保留
        // 它**原来的**模块名——那才是它的声明点）。
        decl.module = Some(module.to_string());
        match inherited.iter_mut().find(|it| it.symbol == decl.symbol) {
            Some(slot) => *slot = decl,
            None => inherited.push(decl),
        }
    }
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
    overlay: &[(PathBuf, String)],
    modules: &mut Vec<LoadedModule>,
    by_name: &mut HashMap<String, usize>,
    diagnostics: &mut Vec<ProjectDiagnostic>,
    stack: &mut Vec<String>,
    visiting: &mut HashSet<String>,
    exports: &mut HashMap<String, Vec<NotationDecl>>,
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

    // 读源文本（入口可以用调用方给的中间态文本；其它模块先看内存覆盖，再读盘）。
    let text = match src {
        Some(text) => text.to_string(),
        None => match read_module_text(path, overlay) {
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

    // **先收 import 边，再解析自己**（G-04 第二刀 §10.3 的顺序对调）。
    //
    // 为什么要对调：入口可能用**依赖声明的记法**（`import lib.Set` + `Aᶜ`），
    // 单独解析必然报 `notation-unknown-symbol`；而"解析失败 ⇒ 收不到 import 边
    // ⇒ 依赖根本不被加载 ⇒ 记法永远传不过来"是个死锁（实测闭包里只剩入口一个
    // 模块）。所以这里先用**词法级**扫描（`scan_import_lines`：`import` 必须是
    // 完整的一行，与 `Lexer::on_import_line` 同款判据）拿到边、先访问依赖、拿到
    // 它们的记法表之后再解析自己——解析成功时 `Command::Import` 的边是**权威**
    // 版本（用它替换扫描版）。
    let mut imports: Vec<(String, Span)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (dep, span) in scan_import_lines(&text) {
        if seen.insert(dep.clone()) {
            imports.push((dep, span));
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
                    overlay,
                    modules,
                    by_name,
                    diagnostics,
                    stack,
                    visiting,
                    exports,
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

    // 依赖都访问完了 ⇒ 它们的导出记法表可用。并起来当**继承表**解析自己。
    let mut inherited: Vec<NotationDecl> = Vec::new();
    for (dep, _) in &imports {
        if let Some(table) = exports.get(dep) {
            for decl in table {
                match inherited.iter_mut().find(|it| it.symbol == decl.symbol) {
                    Some(slot) => *slot = decl.clone(),
                    None => inherited.push(decl.clone()),
                }
            }
        }
    }
    let (file, parsed) = match parse_with_inherited(&text, &inherited) {
        Ok(file) => (file, true),
        Err(diagnostic) => {
            diagnostics.push(ProjectDiagnostic {
                kind: ProjectKind::Error(ErrorKind::ImportModuleInvalid),
                message: format!("{}（{name}）", diagnostic.message),
                module: name.to_string(),
                path: Some(path.to_path_buf()),
                span: Some(diagnostic.span),
            });
            (
                FolFile {
                    commands: Vec::new(),
                    src: text.clone(),
                },
                false,
            )
        }
    };
    if parsed {
        // 解析成功时 `Command::Import` 是权威边（书写顺序、按名字去重）。
        let mut from_ast: Vec<(String, Span)> = Vec::new();
        let mut ast_seen: HashSet<String> = HashSet::new();
        for command in &file.commands {
            let Some(raw) = command.import_module() else {
                continue;
            };
            if ast_seen.insert(raw.to_string()) {
                from_ast.push((raw.to_string(), command.span()));
            }
        }
        imports = from_ast;
    }
    // 本模块**导出**的记法 = 继承来的 + 自己声明的（同符号自己覆盖）。
    let mut table = inherited;
    absorb_notations(&file, &mut table, name);
    exports.insert(name.to_string(), table);

    // **后序登记**：依赖已经在 `modules` 里，入口最后（拓扑序）。
    push_module(
        modules,
        by_name,
        name,
        path,
        file,
        imports,
        module_failed || !parsed,
    );
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

/// 模块源文本：内存覆盖优先（不读盘），否则读盘。
fn read_module_text(path: &Path, overlay: &[(PathBuf, String)]) -> std::io::Result<String> {
    if overlay.is_empty() {
        return std::fs::read_to_string(path);
    }
    let key = canonical(path);
    overlay
        .iter()
        .find(|(candidate, _)| canonical(candidate) == key)
        .map(|(_, text)| Ok(text.clone()))
        .unwrap_or_else(|| std::fs::read_to_string(path))
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
