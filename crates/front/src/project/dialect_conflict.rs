//! **闭包内方言文件指令的一致性**（S4，设计 `docs/design/dialect-config.md` §2 第 4 档 / §4）。
//!
//! 方言（严格隐式参数读法）是**整个编译单元**的属性 —— 与 prelude 同形：一个闭包只有
//! 一种读法。入口文件的 `-- sokonanoda:dialect …` 参与五档优先级链
//! （[`crate::compile::DialectSources`]），**依赖模块**的指令只有两种命运：与它一致
//! （继承 ✓），或者成为一条 `import-dialect-conflict`（默认**不通融** ✗）。
//!
//! **判据只比较"源文件里写下的指令"**（用户 2026-10-20 的措辞：「闭包里两个模块的
//! 文件指令不一致」），**不读 `CompileOptions`**。为什么这样更稳：同一个闭包在 CLI
//! （S4 已接全部五档）与编辑器（文件/清单/设置那一半在 S5）里会解析出不同的
//! `options.dialect` —— 拿指令去比 `options` 就会出现「CLI 绿、编辑器红」的假红 ✗。
//! prelude 的检查 (`check_prelude_conflicts`) 比的是 `options.prelude`，但 LSP 已经
//! 从入口读过 prelude 指令（`crates/lsp/src/lib.rs:1088`）⇒ 那边没有这条缝 ✓。
//!
//! 参照值：**入口**的指令（写了的话）—— 它正是优先级链第 4 档在场的那个值；入口没写
//! ⇒ 拓扑序里**第一个**写了指令的模块（`Closure::modules` 本身就是拓扑序 ⇒ 确定性 ✓）。
//!
//! **为什么在单独一个文件**：`project/mod.rs` 在判据 ⑨ 的存量基线里（**只许减不许增**），
//! 这段逻辑 (~60 行) 放进去会当场判红；同族先例 = `error_hint.rs` / `compile/dialect.rs`。
//!
//! 边界（设计 §6 第 1 条）：粒度**止于文件指令** —— 不按声明、不按语句切换读法 ✓。

use crate::compile::{explicit_dialect, Dialect};

use super::graph::Closure;
use super::report::{ProjectDiagnostic, ProjectKind};

/// 闭包里出现两种方言指令 ⇒ 每个"少数派"模块的每条 `import` 边上报一条诊断
/// （形状照 `check_prelude_conflicts`：诊断挂在**导入方**的 `import` 行上）。
pub(crate) fn check_conflicts(closure: &Closure, diagnostics: &mut Vec<ProjectDiagnostic>) {
    let entry_directive = closure
        .module_index(&closure.entry)
        .and_then(|index| explicit_dialect(&closure.modules[index].file.src));
    let mut reference = entry_directive.map(|dialect| (closure.entry.clone(), dialect));
    if reference.is_none() {
        for index in closure.compilable() {
            let module = &closure.modules[index];
            if let Some(dialect) = explicit_dialect(&module.file.src) {
                reference = Some((module.name.clone(), dialect));
                break;
            }
        }
    }
    let Some((reference_module, reference_dialect)) = reference else {
        return; // 整个闭包都没写指令 ⇒ 不表态，一条诊断都不发（0.95.0 的行为）
    };
    let mut conflicts: Vec<(String, Dialect)> = Vec::new();
    for index in closure.compilable() {
        let module = &closure.modules[index];
        if module.name == reference_module {
            continue;
        }
        if let Some(dialect) = explicit_dialect(&module.file.src) {
            if dialect != reference_dialect {
                conflicts.push((module.name.clone(), dialect));
            }
        }
    }
    for (module_name, wrote) in conflicts {
        for module in &closure.modules {
            for (dep, span) in &module.imports {
                if *dep != module_name {
                    continue;
                }
                diagnostics.push(ProjectDiagnostic {
                    kind: ProjectKind::DialectConflict,
                    message: format!(
                        "模块 `{module_name}` 的方言指令是 `-- sokonanoda:dialect {}`，而 `{reference_module}` 写的是 `{}`：方言是整个编译单元的属性，闭包里只能有一种读法",
                        wrote.keyword(),
                        reference_dialect.keyword()
                    ),
                    module: module.name.clone(),
                    path: Some(module.path.clone()),
                    span: Some(*span),
                });
            }
        }
    }
}
