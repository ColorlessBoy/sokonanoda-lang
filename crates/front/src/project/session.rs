//! **切片 1b**：一次 `build <dir>` 内**共享一套 DAG** —— 共享库层只编一次，入口各自复用。
//!
//! 为什么是这个形状（设计 `docs/design/module-artifacts.md` §9）：front **不能命名** kernel 的
//! `pub(crate)` 别名 `DeclarMap` ⇒ 检查点**不能进具名字段** ⇒ 把"库层 + 各入口"的循环整个放进
//! 本函数，检查点只做**局部变量**（类型推断即可）。
//!
//! 语义：库层（各 `lib/*`）编一次 ⇒ `hide_declars()` 留检查点 ⇒ 每个入口 `restore_declars`
//! 回到"只有库层"⇒ **只走该入口自己的命令** ⇒ 编完 `hide_declars()` 丢掉入口声明
//! ⇒ **单元之间从不共处一个环境**（09-25 那次假"重复声明"的结构性根因因此消失）。

use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::Config;

use crate::compile::{run_pass_with, split_report, CompileOptions, PassTables, SourceUnit};
use crate::compile::{CompileOutput, DocumentReport};

/// 跑一次"库层一次 + 各入口各自"的编译会话；每个入口的结果经 `on_entry` 交回。
///
/// `lib_units` = 共享库层的单元（拓扑序）· `entries[i]` = 第 i 个入口**自己的**单元
/// （**不含**依赖 —— 依赖已在环境里）。
pub fn with_project_session<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    mut on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        // **并集顺序**下每个库模块的命令区间（修法 A：按各入口自己的闭包顺序拼接 + 重编号）。
        &[std::ops::Range<usize>],
        // 该入口在**合并输出**里的命令区间（`lib_n..lib_n + 入口那趟命令数`）。
        std::ops::Range<usize>,
    ) -> R,
) -> Vec<R> {
    let arena = stumpalo::Arena::new();
    let builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    // ① 库层编一次（影子不建：`None` ⇒ 不需要额外的局部 arena，见 `run_pass_with` 的注释）。
    // **切片 1b**：prelude 登记表**跨趟复用**（库层趟装好、入口趟接着用）——
    // 它们不在 `declars` 里，检查点救不了 ✗（2026-09-29 定位）。
    let mut tables = PassTables::new();
    let (lib_pass, mut builder, lib_tables) = run_pass_with(
        builder, None, true, tables, lib_units, options, true, None, None, None,
    );
    tables = lib_tables;
    // **逐模块报告**（与 `check::run` 同构）：库层那趟的报告按单元切分 ⇒ 接线方
    // 能组装出与今天**逐字节相同**的 `ProjectReport`（缓存内容不变）。
    let lib_n = lib_pass.n_commands;
    let lib_ranges = crate::compile::unit_ranges(lib_units);
    let lib_reports = split_report(
        lib_pass.report,
        &lib_pass.out.error_cmds,
        &lib_pass.out.warning_cmds,
        lib_units,
    );
    // ② 检查点 = "只有库层"的环境（`DeclarMap: Clone` 由 `snapshot()` 已在用）。
    let lib_out = lib_pass.out;
    let checkpoint = builder.hide_declars();
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry_units) in entries.iter().enumerate() {
        // ③ 回到只有库层的状态 ⇒ 入口之间不共享环境。
        builder.restore_declars(checkpoint.clone());
        let (pass, next, next_tables) = run_pass_with(
            builder,
            None,
            false,
            tables,
            entry_units,
            options,
            true,
            None,
            None,
            None,
        );
        builder = next;
        tables = next_tables;
        let entry_range = lib_n..lib_n + pass.n_commands;
        let entry_reports = split_report(
            pass.report,
            &pass.out.error_cmds,
            &pass.out.warning_cmds,
            entry_units,
        );
        // **闭包级扁平输出**（库层在前、入口在后，命令号整体偏移 `lib_n`）⇒ 接线方
        // 能按 `unit_ranges(闭包 units)` 正确切分事件（与今天逐字节等价的前提）。
        let mut merged = lib_out.clone();
        merged.events.extend(pass.out.events.iter().cloned());
        merged
            .event_cmds
            .extend(pass.out.event_cmds.iter().map(|c| c + lib_n));
        merged.errors.extend(pass.out.errors.iter().cloned());
        merged
            .error_cmds
            .extend(pass.out.error_cmds.iter().map(|c| c + lib_n));
        merged.warnings.extend(pass.out.warnings.iter().cloned());
        merged
            .warning_cmds
            .extend(pass.out.warning_cmds.iter().map(|c| c + lib_n));
        merged.stats.kernel_checks += pass.out.stats.kernel_checks;
        out.push(on_entry(
            index,
            merged,
            entry_reports,
            &lib_reports,
            &lib_ranges,
            entry_range,
        ));
        // ④ 丢掉这个入口的声明（下一次循环再装回检查点）。
        drop(builder.hide_declars());
    }
    out
}
