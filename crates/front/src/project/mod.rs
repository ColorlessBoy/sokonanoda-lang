//! 项目层入口：清单发现 → 闭包加载 → 一次编译 → 逐模块报告。
//!
//! 设计：`docs/design/imports-and-projects.md`（ROADMAP I16）。要点：
//!
//! * **零配置退路**：没有清单时模块根 = 入口文件所在目录（刻意偏离真实
//!   `lean`：官方搜索路径里没有文件自己的目录，cwd 只影响模块名——见 §2.1）；
//! * **一次编译**：所有模块按拓扑序进**同一个 arena / 同一个 `EnvBuilder`**，
//!   内核一行不改；
//! * 入口文件的报告可以直接交给既有的单文档消费者（`cmd` 已重基）。

pub mod cache;
pub mod graph;
pub mod manifest;
pub mod module_name;
pub mod module_plan;
pub mod report;
pub mod resolve;
pub mod session;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::compile::{
    explicit_prelude_mode, unit_ranges, CompileOptions, CompileOutput, DeclStatus, DocumentReport,
    ErrorKind, PreludeMode, SourceUnit, WarningKind, PRELUDE_NEVER_YIELDS,
};

pub use graph::{load_closure, Closure, LoadedModule};

/// 去掉 `import` 代码行（注释行原样保留）。
///
/// 用途：把多个模块的源码**首尾相接**成合成文件（`judge_*` 的闭包前缀、LSP 的
/// 判据前缀）。`import` 语义上必须排在文件最前，直接拼接会让第二个模块的
/// `import` 出现在文件中间——合成文件连解析都过不去。只过滤**代码行**
/// （`--` 注释里的 "import" 字样保留：它不影响语义，也不该被误伤）。
pub(crate) fn importless_source(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim_start();
        if is_import_line(trimmed) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// 这一行（已 `trim_start`）是不是一条 `import` **代码行**。
fn is_import_line(trimmed: &str) -> bool {
    trimmed.starts_with("import ") || trimmed == "import"
}

/// 源码里有没有 `import` 代码行——**只看分发**，不看语义。
///
/// 用途只有一个：入口**单独 parse 失败**时判断它该不该走项目闭包。入口用了
/// 依赖声明的记法时（G-04 第二刀：`import lib.Set` + `Aᶜ`），单文件 parse 必然
/// 报 `notation-unknown-symbol`，但**闭包路径能编**——所以分发不能只看
/// `parse` 成功与否。**判定永不使用它**（判定只认 kernel）。
pub fn source_has_import_line(source: &str) -> bool {
    source.lines().any(|line| is_import_line(line.trim_start()))
}

/// 这份源码要不要走**项目闭包**：先 `parse`（唯一权威）；parse 失败时**只有一种
/// 情况**才改判——错误是 `notation-unknown-symbol` 且源码里写了 `import`。
///
/// 为什么只放这一种（G-04 第二刀）：**只有"用了依赖声明的记法"这一种解析失败
/// 可能被 import 治好**。别的解析错误（`import` 没置顶、模块名有横线、括号不配对…）
/// 加载多少依赖都还是错，必须原样报给用户——放宽成"有 import 行就走闭包"会让
/// 那些诊断退化成 `import-module-invalid`（实测：`project_features.rs` 的三条
/// 用例当场变红）。
///
/// 记法符号在**未声明**时都收敛到同一条码（这是第二刀的词法设计：`∈`/`''` 走
/// `Sym`，`𝒫`/`ᶜ`/`⁻¹'`/`×ˢ` 是标识符字符、单独 parse 本来就过），所以判据只需
/// 这一条。**只用于分发**，不参与任何判定。
pub fn is_project_source(source: &str) -> bool {
    match crate::parse(source) {
        Ok(file) => file.commands.iter().any(|command| command.is_import()),
        Err(diag) => diag.code() == "notation-unknown-symbol" && source_has_import_line(source),
    }
}

pub use manifest::{find_manifest, Manifest, MANIFEST_FILE};
pub use module_name::{ModuleName, ModuleNameError, DASH_HINT, MODULE_EXTENSION};
pub use report::{ModuleReport, ModuleStatus, ProjectDiagnostic, ProjectKind, ProjectReport};
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

/// 带**内存覆盖**的闭包编译（LSP 跨文件失效用）：`overlay` 里的路径不读盘，
/// 直接用给定文本——编辑器未保存的依赖编辑因此对入口可见。
pub fn compile_project_with_overlay(
    entry_path: &Path,
    entry_src: Option<&str>,
    options: &CompileOptions,
    root_override: Option<&Path>,
    overlay: &[(PathBuf, String)],
) -> ProjectReport {
    compile_plan(
        plan_project_with_overlay(entry_path, entry_src, root_override, overlay),
        options,
    )
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
        mix(b"soko.project-iface/2");
        // **入口路径也要进摘要**（T-A06 顺带发现）。
        //
        // 报告里的 `entry`/`root`/`manifest` 与每个模块的 `path` 都是**绝对路径**。
        // 只按"模块名 + 源文本 + import 边"算键的话，两个**内容逐字相同但在不同
        // 目录**的项目会共用一个键 ⇒ 第二个回放到的是第一个的路径：
        // `query project` 报错的模块根、LSP 的 definition/references **跳到别的
        // 目录的文件**。内容相同不代表位置相同。
        mix(digest_path(&self.entry).as_bytes());
        mix(b"\0");
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

    /// **模块级 Merkle 键**（per-module 产物的键；设计 `docs/design/module-artifacts.md` §3）。
    ///
    /// 与 [`ProjectPlan::digest`] **同一个哈希函数、同一条 Merkle 链**，差别只在**粒度**：
    /// `digest` 把**整条闭包**折成一条键 ⇒ 42 个入口各存一份、共享依赖各编一遍（**G-68** ✗）；
    /// 这里给**每个模块**一条键 ⇒ 依赖没变时它的键**逐字节不变** ⇒ 产物可被所有下游复用 ✓。
    ///
    /// `key(M) = cache::key("soko.module-artifact/1\0" + 路径(M) + 源文本(M) + [name(D), key(D)]…)`
    /// —— 复用 `compile::cache::key`，于是**版本 / build stamp / prelude 模式**仍是单一真相，
    /// 且"依赖变 ⇒ `key(D)` 变 ⇒ 所有下游 key 变"是**构造性**成立的（这是**不许错编**的底线）。
    /// 路径进键的理由与 T-A06 相同：**内容相同不代表位置相同**。
    /// 返回**拓扑序**的 `(模块名, 键)`（依赖在前、入口在最后）。
    pub fn module_keys(&self, options: &CompileOptions) -> Vec<(String, String)> {
        let mut keys: Vec<(String, String)> = Vec::with_capacity(self.closure.modules.len());
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for module in &self.closure.modules {
            let mut text = String::from("soko.module-artifact/1\0");
            text.push_str(&digest_path(&module.path));
            text.push('\0');
            text.push_str(&module.file.src);
            text.push('\0');
            for (dep, _) in &module.imports {
                text.push_str(dep);
                text.push('\0');
                if let Some(&idx) = seen.get(dep.as_str()) {
                    text.push_str(&keys[idx].1);
                }
                text.push('\0');
            }
            let key = crate::compile::cache::key(&text, options);
            seen.insert(module.name.as_str(), keys.len());
            keys.push((module.name.clone(), key));
        }
        keys
    }
}

/// 解析项目根、加载闭包（不编译）。
pub fn plan_project(
    entry_path: &Path,
    entry_src: Option<&str>,
    root_override: Option<&Path>,
) -> ProjectPlan {
    plan_project_with_overlay(entry_path, entry_src, root_override, &[])
}

/// **词法**绝对化：`cwd` 只在这里用一次，此后不参与任何判定（G-12；设计 §4.4）。
///
/// * 相对路径 ⇒ `current_dir().join(path)`：尾部原样保留，**不**解析 `..`、不碰
///   符号链接——`canonicalize` 是**视图层**的职责（`query/project.rs` 的
///   `absolute()`），而且 stdin + `--root` 会合成一个磁盘上不存在的入口路径，
///   `canonicalize` 必然失败并回落成相对路径，反而制造"绝对 entry + 相对 root"；
/// * 空路径 ⇒ `cwd`（`--root ''`、裸文件名的空 `parent()` 都不是合法模块根，
///   语义上等于 cwd）⇒ 返回值**永不**为空；
/// * 已经绝对 ⇒ 原样。
fn absolute_lexical(path: &Path) -> PathBuf {
    let cwd = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if path.as_os_str().is_empty() {
        return cwd();
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    cwd().join(path)
}

/// 摘要里用的路径形态：**只去掉 `.` 组件**，别的原样。
///
/// 为什么需要它：`grade Main.sokonanoda` 与 `build .`（它收集到的是
/// `./Main.sokonanoda`）指的是同一个文件，而 `absolute_lexical` 按 G-12 的纪律
/// **故意不解析** `..`/`.`（"尾部原样保留"）⇒ 两个拼写给出两个字符串。
/// 摘要若直接用原串，同一次编译经两条命令就会 miss（实测：加了入口路径之后
/// `a_project_cache_hits_and_a_dependency_change_invalidates_it` 立刻红）。
///
/// **不**做更多：不解析 `..`、不碰符号链接、不 `canonicalize`——那会违反 G-12
/// （路径在词法绝对化之后就不再被解释，模块名靠 `strip_prefix(root)` 稳定）。
fn digest_path(path: &Path) -> String {
    let mut out = PathBuf::new();
    for component in path.components() {
        if matches!(component, std::path::Component::CurDir) {
            continue;
        }
        out.push(component.as_os_str());
    }
    out.to_string_lossy().into_owned()
}

/// 解析项目根、加载闭包（不编译）——`overlay` 提供打开文档的内存文本。
pub fn plan_project_with_overlay(
    entry_path: &Path,
    entry_src: Option<&str>,
    root_override: Option<&Path>,
    overlay: &[(PathBuf, String)],
) -> ProjectPlan {
    // G-12：入口路径与 `root_override` **一起**词法绝对化——只绝对化一个，
    // `module_name_of_path` 的 `strip_prefix(root)` 就失配，模块名退化成裸
    // `file_stem`（`units.u` → `u`），`ProjectPlan::digest`（缓存键）跟着漂。
    let entry_path = absolute_lexical(entry_path);
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
        Some(root) => (absolute_lexical(root), None, None),
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
    //
    // **G-29 诊断**（`SOKO_PLAN_TRACE=1`）：这一步是**每次按键**都跑的（LSP 的
    // `project_compile_incremental` 第一件事就是它 ✓）⇒ 它值多少毫秒、解析了几个模块，
    // 直接决定"编辑延迟"的地板 ✓。判据用**结构计数**（解析的模块数），墙钟只做参考。
    let plan_started = std::time::Instant::now();
    let closure = graph::load_closure_with_overlay(&root, &entry_path, entry_src, overlay);
    if std::env::var_os("SOKO_PLAN_TRACE").is_some() {
        eprintln!(
            "PLAN_TRACE load={}ms modules={}",
            plan_started.elapsed().as_millis(),
            closure.modules.len()
        );
    }

    ProjectPlan {
        entry: entry_path,
        root,
        manifest: manifest_path,
        requires_warning,
        diagnostics,
        closure,
    }
}

/// 执行计划：闭包级检查 → 一次编译 → 逐模块报告 → 挂诊断。
pub fn compile_plan(plan: ProjectPlan, options: &CompileOptions) -> ProjectReport {
    compile_plan_with_progress(plan, options, None)
}

/// 同 [`compile_plan`]，但每处理一条命令回调一次（**声明级进度**，P2）——
/// `build`/`rebuild` 用它把"文件级"进度细化到"声明级"（`docs/protocol.md` 的 `build.decl`）。
/// **切片 1b 的接线助手**：从 plan 的闭包里取"满足 `keep` 的模块"的单元
/// （**拓扑序**，与 `compile_plan` 同序 —— 依赖必须先于被依赖，否则编译会失败）。
///
/// 用途：把"共享库层"与"各入口自己"分开交给 [`crate::project::session::with_project_session`]
/// （库层只编一次，入口各自复用同一套 DAG）。
pub fn units_for_modules<'a>(
    plan: &'a ProjectPlan,
    keep: impl Fn(&crate::project::graph::LoadedModule) -> bool,
) -> Vec<crate::compile::SourceUnit<'a>> {
    plan.closure
        .compilable()
        .iter()
        .map(|&index| &plan.closure.modules[index])
        .filter(|module| keep(module))
        .map(|module| crate::compile::SourceUnit {
            name: &module.name,
            path: Some(module.path.as_path()),
            file: &module.file,
        })
        .collect()
}

/// **切片 1b**：把「编」与「组装」分开 —— session 可用**预算结果**替换「编」
/// （库层只编一次、各入口复用同一套 DAG）。组装段**逐字**搬自原 `compile_plan_with_progress`。
pub struct PlanCompiled<'a> {
    pub compilable: Vec<usize>,
    pub flat_out: CompileOutput,
    pub reports: Vec<DocumentReport>,
    pub closure: &'a crate::project::graph::Closure,
    pub diagnostics: Vec<ProjectDiagnostic>,
    pub entry_path: std::path::PathBuf,
    pub root: std::path::PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub requires_warning: Option<String>,
}

/// **切片 1 接线的前置步骤**：跑那两步**闭包级检查**并把诊断**并进 plan**。
///
/// `compile_plan_with_progress` 在**编之前**跑 `check_name_collisions` 与
/// `check_prelude_conflicts`（`:538`/`:539`），两者都要 **`&mut Closure` / `options`**。
/// 切片 1 的接线方（CLI）**够不着**私有 `closure` ⇒ 由本函数代跑 ✓。
///
/// **必须在跑 session 之前调用**（检查可能把模块标成 blocked，
/// 那会改变"编哪些模块"）⇒ 顺序不能反 ✗。
/// **不跑 = 丢诊断 = `--json` 会变** ✗。
pub fn precheck_plan(plan: &mut ProjectPlan, options: &CompileOptions) {
    check_name_collisions(&mut plan.closure, &mut plan.diagnostics);
    check_prelude_conflicts(&plan.closure, options, &mut plan.diagnostics);
}

/// **切片 1（G-68）的接线口**：把"一次 session 的结果"组装成该入口的 `ProjectReport`。
///
/// **为什么需要**：`build <dir>` 今天对**每个入口**走 `compile_plan_with_progress`
/// ⇒ 共享 `lib/*` 被**每个入口各编一遍** ✗（这正是 4.14× 与"分片无效"的同一个根，
/// 见 `docs/design/incremental-environment.md` §21）。切片 1 的做法是
/// **一次 `with_project_session` 覆盖全部入口**（库层只编一次），
/// 而 session 的回调交的是**片段**（`flat_out` + `reports`）⇒ 需要在这里组装。
///
/// **为什么不直接把 `PlanCompiled` 交给 CLI**：它的 `closure` 字段要 `&Closure`，
/// 而 `ProjectPlan::closure` 是**私有的**（§24）⇒ CLI 组不出来。
/// 本函数在前端内部 ⇒ **够得着** ✓，且 CLI 不必知道 `PlanCompiled` 的细节 ✓。
///
/// **语义**：与 `compile_plan_with_progress` 的组装段**逐字同构**
/// （都是 `assemble_report(PlanCompiled { … })`）⇒ `--json` 逐字节不变的前提 ✓。
///
/// ⚠ **接线方必须先调 [`precheck_plan`]**（那两步闭包级检查要 `&mut Closure`，
/// 本函数够不着）—— **不调 = 丢诊断 = `--json` 会变** ✗。
/// **切片 1 接线用**：把 session 交的两份报告**按该入口自己的闭包顺序**拼成一份。
///
/// **为什么需要**：`assemble_report` 要的 `reports` 是**整个闭包（`lib/*` + 入口）
/// 逐模块**的报告（长度 = `compilable.len()`，下标 = `compilable` 里的槽位）。
/// 而 session 回调交的是**两份**：`lib_reports`（**并集顺序**，与 `lib_units` 对齐）
/// 与 `entry_reports`（只有入口）。
///
/// ⚠ **必须按该入口自己的闭包顺序**重排 —— 用并集顺序会让 `--json` 对不上 ✗
/// （切片 1b 实测：`passes` 4126→**5404**、`judge_ms` 148.4→**162.9** ⇒ 更慢）。
///
/// **参数**：`closure_units` = 该入口闭包的单元（**拓扑序、入口在最后**）；
/// `lib_units` = session 收到的库层（**并集顺序**，与 `lib_reports` 一一对应）；
/// `entry_reports` = 该入口自己的报告（session 交的那份）。
///
/// **返回**：与 `closure_units` 等长、逐槽位对齐的报告序列。
pub fn merge_session_reports(
    closure_units: &[crate::compile::SourceUnit<'_>],
    lib_units: &[crate::compile::SourceUnit<'_>],
    lib_reports: &[crate::compile::DocumentReport],
    entry_reports: Vec<crate::compile::DocumentReport>,
) -> Vec<crate::compile::DocumentReport> {
    let lib_slot: std::collections::HashMap<&str, usize> = lib_units
        .iter()
        .enumerate()
        .map(|(slot, unit)| (unit.name, slot))
        .collect();
    // 入口那份报告按**名字**对到闭包里的入口单元（闭包最后一个）。
    let entry_name = closure_units.last().map(|unit| unit.name);
    let mut entry_reports = entry_reports.into_iter();
    closure_units
        .iter()
        .map(|unit| {
            if Some(unit.name) == entry_name {
                entry_reports.next().unwrap_or_default()
            } else {
                lib_slot
                    .get(unit.name)
                    .and_then(|&slot| lib_reports.get(slot))
                    .cloned()
                    .unwrap_or_else(crate::compile::DocumentReport::default)
            }
        })
        .collect()
}

/// **S2 步 3 的接线口**：闭包编译走「库层一趟 + 入口一趟（可带**信任前缀**）」，
/// 组装成与 [`compile_plan_with_progress`] **同形**的 `ProjectReport`。
///
/// **为什么不复用 [`compile_plan_with_progress`]**：那一条把**所有单元**塞进**一趟**
/// `run`（`compile_all_units_with_progress`）⇒ 入口拿不到信任前缀（库层命令排在它
/// 前面，一旦信任整段都被跳过，而它们的报告没人补）✗。这一条用
/// [`crate::project::session::with_project_session_trusted`]：库层一趟、入口各自一趟
/// ⇒ `EntryTrust.plan.before` 落在**入口自己的命令空间**里（设计
/// `docs/design/declaration-incremental.md` §4.2）。
///
/// ⚠ **入口那一趟的报告只覆盖「新查的那一段」** —— 被信任的前缀不进报告。
/// `splice` 就是补它的：调用方把缓存的前缀拼回一份**完整**的入口报告。
/// 传 `None` ⇒ 用 session 交的那份（**只有新查段**，前缀会缺声明）。
///
/// 入口被阻断（没有可编的入口单元）⇒ 退回 [`compile_plan_with_progress`]
/// （那才是今天的行为，不猜）。
pub(crate) fn compile_plan_incremental(
    mut plan: ProjectPlan,
    options: &CompileOptions,
    entry_trust: Option<crate::project::session::EntryTrust>,
    splice: Option<&dyn Fn(crate::compile::DocumentReport) -> crate::compile::DocumentReport>,
) -> ProjectReport {
    // **只给测量用的逃生门**（`SOKO_NO_ENTRY_TRUST=1`）：把信任前缀**当成没有**
    // ⇒ 这一次编译走的就是"优化前"那条路（入口整份重查）。用途：让设计
    // `docs/design/declaration-incremental.md` §5.2.1 的**前/后对比**能在
    // **同一份二进制、同一份夹具、同一操作序列**上量出来 —— 跨机墙钟不可转移，
    // 同进程自比才是判据（`AGENTS.md` 判据纪律②）。**默认关** ⇒ 生产零改动 ✓。
    let entry_trust = if std::env::var_os("SOKO_NO_ENTRY_TRUST").is_some() {
        None
    } else {
        entry_trust
    };
    // ⚠ **必须在跑 session 之前**（检查可能把模块标成 blocked，那会改变"编哪些模块"）。
    precheck_plan(&mut plan, options);
    let entry_path = plan.entry.clone();
    let lib_units = units_for_modules(&plan, |m| m.path != entry_path);
    let entry_units = units_for_modules(&plan, |m| m.path == entry_path);
    if entry_units.is_empty() {
        return compile_plan_with_progress(plan, options, None);
    }
    let closure_units = units_for_modules(&plan, |_| true);
    let entries = vec![entry_units];
    let trust = vec![entry_trust];
    let mut reports_out: Vec<ProjectReport> = crate::project::session::with_project_session_trusted(
        &lib_units,
        &entries,
        options,
        &trust,
        |_i, merged, fresh_entry, lib_reports, _lib_ranges, _entry_range| {
            let mut fresh = fresh_entry.into_iter().next().unwrap_or_default();
            let full = match splice {
                Some(f) => f(fresh),
                None => std::mem::take(&mut fresh),
            };
            let reports =
                merge_session_reports(&closure_units, &lib_units, lib_reports, vec![full]);
            assemble_from_session(&plan, merged, reports)
        },
    );
    reports_out.pop().expect("一个入口必须回调一次")
}

pub fn assemble_from_session(
    plan: &ProjectPlan,
    flat_out: crate::compile::CompileOutput,
    reports: Vec<crate::compile::DocumentReport>,
) -> ProjectReport {
    let mut diagnostics = plan.diagnostics.clone();
    diagnostics.extend(plan.closure.diagnostics.iter().cloned());
    assemble_report(PlanCompiled {
        compilable: plan.closure.compilable(),
        flat_out,
        reports,
        closure: &plan.closure,
        diagnostics,
        entry_path: plan.entry.clone(),
        root: plan.root.clone(),
        manifest_path: plan.manifest.clone(),
        requires_warning: plan.requires_warning.clone(),
    })
}

pub fn assemble_report(c: PlanCompiled<'_>) -> ProjectReport {
    let PlanCompiled {
        compilable,
        flat_out,
        reports,
        closure,
        mut diagnostics,
        entry_path,
        root,
        manifest_path,
        requires_warning,
    } = c;
    // `units` 借用 `closure` ⇒ 不能与它同处一个结构体（自引用 ✗）；这里**重建**（与编段同构）。
    let units: Vec<SourceUnit<'_>> = compilable
        .iter()
        .map(|&index| SourceUnit {
            name: &closure.modules[index].name,
            path: Some(closure.modules[index].path.as_path()),
            file: &closure.modules[index].file,
        })
        .collect();
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
                    source: module.file.src.clone(),
                    report: reports[slot].clone(),
                    events,
                    status: ModuleStatus::Compiled,
                });
            }
            None => modules.push(ModuleReport {
                name: module.name.clone(),
                path: module.path.clone(),
                imports,
                source: module.file.src.clone(),
                report: DocumentReport::default(),
                events: CompileOutput::default(),
                // 自己加载失败（缺失/解析/环）vs 被上游拖住——报告都是空的，
                // 状态必须分开，否则项目树会把"根因"说成"受害者"。
                status: if module.failed {
                    ModuleStatus::LoadFailed
                } else {
                    ModuleStatus::Blocked
                },
            }),
        }
    }

    // 6) 开放练习提示：被导入模块里的 `sorry` 对下游不可见，值得在 import 行上说一句。
    collect_open_exercise_warnings(closure, &modules, &mut diagnostics);

    // 7) 依赖的**编译失败**同样阻断下游（check-then-add 的跨模块版本）：
    //    一个非入口模块的报告里有错误 ⇒ 所有（传递）import 它的模块不保留
    //    编译结果，只在 import 行上留一条 `import-dependency-failed`。
    //    v1 是"全编译后再丢弃"：闭包共用一次 arena/builder，教学规模下这点浪费
    //    可以接受；等 P7 有 decl 级产物时再做真正的早停。
    let result_blocked = block_on_compile_failures(closure, &modules, &mut diagnostics);
    for module in &mut modules {
        if result_blocked.contains(&module.name) {
            module.report = DocumentReport::default();
            module.events = CompileOutput::default();
            module.status = ModuleStatus::Blocked;
        }
    }

    let mut project = ProjectReport {
        entry: entry_path,
        root,
        manifest: manifest_path,
        modules,
        diagnostics,
        requires_warning,
        // 入口可见的记法表（T-D11）：加载期算好的那份，直接搬到报告层。
        notations: closure.notations.clone(),
        // **§5.1 的验收读数**：组装段把逐模块事件重建了 ⇒ `flat_out.stats` 会丢
        // ⇒ 这里**显式**搬过来（实测不搬恒 0，LSP/`QueryDoc` 那条路读不到）。
        kernel_checks: flat_out.stats.kernel_checks,
    };
    project.attach_diagnostics();
    project
}

pub fn compile_plan_with_progress(
    mut plan: ProjectPlan,
    options: &CompileOptions,
    progress: Option<&mut dyn crate::compile::ProgressSink>,
) -> ProjectReport {
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

    let (flat_out, reports) =
        crate::compile::compile_all_units_with_progress(&units, options, progress);
    assemble_report(PlanCompiled {
        compilable,
        flat_out,
        reports,
        closure: &closure,
        diagnostics,
        entry_path,
        root,
        manifest_path,
        requires_warning,
    })
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
            // `Nat`/`Bool` 家族由 prelude 自己占，**永不**让位（另有专门的
            // 冲突检查 `check_prelude_conflicts`）。L1 名字**不在**这里：
            // 它们按族合法让位（`PRELUDE_NEVER_YIELDS` 的文档，设计 §2.3-2），
            // 所以两个模块各自声明 `True` 仍要报友好的 `import-name-collision`。
            if PRELUDE_NEVER_YIELDS.contains(&declared.as_str()) {
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
