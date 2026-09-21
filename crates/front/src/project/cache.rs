//! 项目闭包的编译缓存：**一份键，三条命令共用**（`check` / `build` / `query`）。
//!
//! 键 = `ProjectPlan::digest(options)`（拓扑序上每个模块的名字/源/imports + prelude
//! 模式，见 `docs/design/compile-cache.md` §7）。单文件不走这里（各自的文本键）。
//!
//! **为什么在 `front` 而不是 CLI**（T-A01）：LSP 也需要这份缓存
//! （`crates/lsp/src/lib.rs` 的 `Doc::set_text` 对有 `import` 的文档要读它），
//! 而 LSP 依赖不到 CLI crate。搬到 `front` 之后 CLI 与 LSP 共用同一份实现——
//! 这正是 `docs/design/compile-cache.md` §1「LSP 与 CLI 共用同一份」那句话
//! 本来该有的样子（对项目文件，它曾经并不成立）。

use std::path::Path;

use crate::compile::cache::{self, CachedCompile};
use crate::compile::CompileOptions;
use crate::project::{plan_project, ProjectPlan, ProjectReport};

/// 解析项目根 + 加载闭包（IO/parse）并算出**缓存键**。
pub fn plan(
    entry: &Path,
    src: Option<&str>,
    root_override: Option<&Path>,
    options: &CompileOptions,
) -> (ProjectPlan, String) {
    let plan = plan_project(entry, src, root_override);
    let digest = plan.digest(options);
    (plan, digest)
}

/// 摘要命中？（`None` = 未命中或缓存不可用——调用方照常编译）
pub fn load(digest: &str, options: &CompileOptions) -> Option<CachedCompile> {
    cache::load(digest, options)
}

/// 写一条项目缓存条目（**整份** `ProjectReport`，T-A03）。
///
/// `report`/`output` 仍然是**入口模块**的（CLI `--json` 与 LSP 的 `report()`
/// 读的就是它，形状与单文件条目一致）；`project` 是整份模块表 + 归因，
/// LSP 的跨文件能力（definition/references/rename/`soko/project`）靠它。
pub fn store(digest: &str, options: &CompileOptions, project: &ProjectReport) {
    let Some(entry) = project.entry_module() else {
        return;
    };
    cache::store(
        digest,
        options,
        &CachedCompile {
            report: entry.report.clone(),
            output: Some(entry.events.clone()),
            project: Some(project.clone()),
        },
    );
}

/// 只缓存**干净**的编译产物（有诊断的结果下次仍要重新算：诊断归因依赖具体文件）。
///
/// **注意（G-24）**：`is_clean()` 会被 `requires` 版本漂移一票否决
/// （`requires_warning`），于是整个项目永不入缓存——`courses/set-theory` 的
/// `requires = "0.61"` 就是这个现场。T-A05 会改这条判据（漂移是可回放的确定性
/// 事实，不该关掉缓存）；在那之前，这个函数对带漂移的课程是**恒不写**的。
pub fn store_if_clean(
    digest: &str,
    options: &CompileOptions,
    project: &ProjectReport,
    is_clean: bool,
) {
    if !is_clean {
        return;
    }
    store(digest, options, project);
}
