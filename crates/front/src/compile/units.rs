//! 闭包级（多单元）编译的装配件：单元描述、命令区间、报告切分。
//!
//! 与 `check/` 的分工：`check/mod.rs` + `check/walk.rs` + `check/kernel_phase.rs`
//! 是**单文件流水线**（parse → elab → check-then-add → events/report），
//! 这里负责把多个单元拼成"扁平命令序"、
//! 再把结果按单元切回去（`docs/architecture.md` §4.5 的第 3–5 步）。
//! 单文件编译也走同一条路径（`units.len() == 1`，逐字节等价）。

use super::check::run;
use super::{CompileOptions, CompileOutput, DocumentReport};
use crate::FolFile;

pub struct SourceUnit<'a> {
    /// 模块名（报告/诊断归因；单文件编译时可用文件标签）。
    pub name: &'a str,
    /// 源文件路径（stdin / 未落盘文本为 `None`）。
    pub path: Option<&'a std::path::Path>,
    pub file: &'a FolFile,
}

impl<'a> SourceUnit<'a> {
    /// 单文件编译（今天 CLI/LSP 的默认路径）的便捷构造。
    pub fn single(name: &'a str, file: &'a FolFile) -> Self {
        Self {
            name,
            path: None,
            file,
        }
    }
}

/// 每个单元在"扁平命令序"里的下标区间 `[start, end)`。
pub fn unit_ranges(units: &[SourceUnit<'_>]) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::with_capacity(units.len());
    let mut start = 0;
    for unit in units {
        let end = start + unit.file.commands.len();
        ranges.push(start..end);
        start = end;
    }
    ranges
}

/// 把"扁平命令序"的报告按单元切开，并把 `cmd` 下标**重基**到模块内
/// （单文件编译时是恒等变换）。入口文件的报告因此可以直接交给既有的
/// 单文档消费者（Session / LSP / `query`）。
///
/// 归因一律走**命令下标**（`decl.cmd` / `hover_cmds` / `error_cmds` /
/// `check.cmd`），不用 span：不同文件的 offset 不在同一个坐标空间里，
/// 用 span 猜文件会在"入口第 1 行"和"依赖第 1 行"之间张冠李戴。
pub fn split_report(
    flat: DocumentReport,
    error_cmds: &[usize],
    warning_cmds: &[usize],
    units: &[SourceUnit<'_>],
) -> Vec<DocumentReport> {
    let ranges = unit_ranges(units);
    let fallback = units.len().saturating_sub(1); // 归不到任何命令时算入口的
    let unit_of_cmd = |cmd: usize| ranges.iter().position(|range| range.contains(&cmd));
    let mut reports: Vec<DocumentReport> =
        units.iter().map(|_| DocumentReport::default()).collect();
    let DocumentReport {
        decls,
        hovers,
        hover_cmds,
        errors,
        checks,
        warnings,
    } = flat;
    for mut decl in decls {
        let unit = unit_of_cmd(decl.cmd).unwrap_or(fallback);
        decl.cmd -= ranges[unit].start;
        reports[unit].decls.push(decl);
    }
    for (hover, cmd) in hovers.into_iter().zip(hover_cmds) {
        let unit = unit_of_cmd(cmd).unwrap_or(fallback);
        reports[unit].hovers.push(hover);
        reports[unit].hover_cmds.push(cmd - ranges[unit].start);
    }
    debug_assert_eq!(
        errors.len(),
        error_cmds.len(),
        "every error must carry its command index (CompileOutput::push_error)"
    );
    for (position, error) in errors.into_iter().enumerate() {
        let cmd = error_cmds.get(position).copied().unwrap_or(usize::MAX);
        let unit = unit_of_cmd(cmd).unwrap_or(fallback);
        reports[unit].errors.push(error);
    }
    for check in checks {
        let unit = unit_of_cmd(check.cmd).unwrap_or(fallback);
        reports[unit].checks.push(check);
    }
    // 警告同法按命令归因：语法级的被 `kernel_phase` 钉在所属单元的首条命令上，
    // 内核终审过的（`redundant-sorry`）带真实命令下标——两者都只依赖命令区间，
    // 不猜 offset（不同文件的 offset 不在同一个坐标空间里）。
    debug_assert_eq!(
        warnings.len(),
        warning_cmds.len(),
        "every warning must carry its command index (CompileOutput::push_warning)"
    );
    for (position, warning) in warnings.into_iter().enumerate() {
        let cmd = warning_cmds.get(position).copied().unwrap_or(usize::MAX);
        let unit = unit_of_cmd(cmd).unwrap_or(fallback);
        reports[unit].warnings.push(warning);
    }
    reports
}

/// 编译进度的一拍（**声明级**，P2「进度粒度」）。
///
/// `module` = 正在处理的单元名（单文件模式是空串）；`index`/`total` = 该单元内的
/// 命令（声明）序号（**从 0 起**）与总数。**为什么要有它**：`build`/`rebuild` 的
/// 最小进度粒度以前是**文件** ⇒ 大文件时 UI 长时间不动像卡死（实测冷编
/// `courses/set-theory` 的**最长无输出间隔 39.0s**，`unit08-solution`）✗。
///
/// **零成本纪律**：不传回调（`None`）⇒ 热路径上每条命令只多一次 `Option` 分支；
/// 既有入口 `compile_all_units` 的行为逐字节不变。
pub struct ProgressTick<'a> {
    pub module: &'a str,
    pub index: usize,
    pub total: usize,
}

/// 进度接收端（P2）。**为什么是 trait 而不是 `&mut dyn FnMut(ProgressTick<'_>)`**：
/// 后者在函数签名里会把 `ProgressTick` 的生命周期钉成**某一个**具体生命周期，
/// 于是同一个 sink 传不了两趟 pass（`run` 的 pass 1 / pass 2）✗；trait 的方法签名
/// 天然是 higher-ranked（`for<'x> fn(&mut self, ProgressTick<'x>)`）⇒ 可以反复重借 ✓。
pub trait ProgressSink {
    fn tick(&mut self, tick: ProgressTick<'_>);
}

impl<F> ProgressSink for F
where
    F: for<'x> FnMut(ProgressTick<'x>),
{
    fn tick(&mut self, tick: ProgressTick<'_>) {
        self(tick);
    }
}

/// 项目闭包编译的唯一入口：`units` 按拓扑序排列、**入口在最后**。
/// 返回扁平事件流 + 与 `units` 同序（且 `cmd` 已重基）的逐模块报告。
/// 单文件编译走同一条路径（一个单元），行为与 `compile_all_with` 逐字节一致。
pub fn compile_all_units(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
) -> (CompileOutput, Vec<DocumentReport>) {
    run(units, options, true, None)
}

/// 同 [`compile_all_units`]，但每处理一条命令回调一次（**声明级进度**，P2）。
pub fn compile_all_units_with_progress(
    units: &[SourceUnit<'_>],
    options: &CompileOptions,
    progress: Option<&mut dyn ProgressSink>,
) -> (CompileOutput, Vec<DocumentReport>) {
    run(units, options, true, progress)
}
