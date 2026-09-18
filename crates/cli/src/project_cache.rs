//! 项目闭包的编译缓存：**一份键，三条命令共用**（`check` / `build` / `query`）。
//!
//! 键 = `ProjectPlan::digest(options)`（拓扑序上每个模块的名字/源/imports + prelude
//! 模式，见 `docs/design/compile-cache.md` §7）。单文件文件不走这里（各自的文本键）。
//!
//! 为什么单独成模块：`query` 曾经自己重编译整个闭包（热跑 37ms vs `check` 热跑
//! 3ms），而且 `--text` 的中间态**不能**缓存（磁盘上没有对应源码，摘要会张冠李戴）。

use std::path::Path;

use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::{plan_project, ModuleReport, ProjectPlan};

/// 解析项目根 + 加载闭包（IO/parse）并算出**缓存键**。
pub(crate) fn plan(
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
pub(crate) fn load(digest: &str, options: &CompileOptions) -> Option<CachedCompile> {
    cache::load(digest, options)
}

/// 只缓存**干净**的编译产物（有诊断的结果下次仍要重新算：诊断归因依赖具体文件）。
pub(crate) fn store_if_clean(
    digest: &str,
    options: &CompileOptions,
    entry: &ModuleReport,
    is_clean: bool,
) {
    if !is_clean {
        return;
    }
    cache::store(
        digest,
        options,
        &CachedCompile {
            report: entry.report.clone(),
            output: Some(entry.events.clone()),
        },
    );
}
