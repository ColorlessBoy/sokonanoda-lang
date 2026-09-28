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

use crate::compile::{run_pass_with, split_report, CompileOptions, SourceUnit};
use crate::compile::{CompileOutput, DocumentReport};

/// 跑一次"库层一次 + 各入口各自"的编译会话；每个入口的结果经 `on_entry` 交回。
///
/// `lib_units` = 共享库层的单元（拓扑序）· `entries[i]` = 第 i 个入口**自己的**单元
/// （**不含**依赖 —— 依赖已在环境里）。
pub fn with_project_session<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    mut on_entry: impl FnMut(usize, CompileOutput, Vec<DocumentReport>, &[DocumentReport]) -> R,
) -> Vec<R> {
    let arena = stumpalo::Arena::new();
    let builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    // ① 库层编一次（影子不建：`None` ⇒ 不需要额外的局部 arena，见 `run_pass_with` 的注释）。
    let (lib_pass, mut builder) = run_pass_with(
        builder, None, true, lib_units, options, true, None, None, None,
    );
    // **逐模块报告**（与 `check::run` 同构）：库层那趟的报告按单元切分 ⇒ 接线方
    // 能组装出与今天**逐字节相同**的 `ProjectReport`（缓存内容不变）。
    let lib_reports = split_report(
        lib_pass.report,
        &lib_pass.out.error_cmds,
        &lib_pass.out.warning_cmds,
        lib_units,
    );
    // ② 检查点 = "只有库层"的环境（`DeclarMap: Clone` 由 `snapshot()` 已在用）。
    let checkpoint = builder.hide_declars();
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry_units) in entries.iter().enumerate() {
        // ③ 回到只有库层的状态 ⇒ 入口之间不共享环境。
        builder.restore_declars(checkpoint.clone());
        let (pass, next) = run_pass_with(
            builder,
            None,
            false,
            entry_units,
            options,
            true,
            None,
            None,
            None,
        );
        builder = next;
        let entry_reports = split_report(
            pass.report,
            &pass.out.error_cmds,
            &pass.out.warning_cmds,
            entry_units,
        );
        out.push(on_entry(index, pass.out, entry_reports, &lib_reports));
        // ④ 丢掉这个入口的声明（下一次循环再装回检查点）。
        drop(builder.hide_declars());
    }
    out
}
