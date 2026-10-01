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

/// **S2 步 2**：某个入口那一趟的**信任前缀**（I8 的 `TrustPlan` + 已缓存失败）。
///
/// 为什么需要：闭包编译今天对入口走的是**整份重查**（`check/mod.rs:608` 原文
/// "闭包编译不使用 TrustPlan（v1）"）⇒ 改一个 `theorem` 要把**前面所有**声明
/// 重查一遍（实测 unit08 改一行 **2189ms**，且与改动位置无关 —— 见
/// `docs/design/declaration-incremental.md` §1.2）。
///
/// `plan.before` 与 `failures` 的键都在**该入口自己的命令流**坐标系里
/// （入口趟 `run_pass_with` 只看到 `entry_units`，库层命令不在它的下标空间里）✓。
///
/// ⚠ **本结构只声明"这段前缀的文本没变过、上一轮查过"** —— 文本没变 ⇒ 前缀
/// 语义不变，是既有 I8 不变式（单文件路径已用了很久）。**被信任的那段不会
/// 出现在返回的报告里**（状态由调用方的会话缓存补）⇒ 调用方必须自己拼回去，
/// 否则报告会缺声明（设计 §4.2）。
pub(crate) struct EntryTrust {
    pub plan: crate::compile::TrustPlan,
    pub failures: crate::compile::KernelFailed,
}

/// 跑一次"库层一次 + 各入口各自"的编译会话；每个入口的结果经 `on_entry` 交回。
///
/// `lib_units` = 共享库层的单元（拓扑序）· `entries[i]` = 第 i 个入口**自己的**单元
/// （**不含**依赖 —— 依赖已在环境里）。
pub fn with_project_session<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    on_entry: impl FnMut(
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
    with_project_session_trusted(lib_units, entries, options, &[], on_entry)
}

/// 同 [`with_project_session`]，但**每个入口可以带一份信任前缀**（S2 步 2）。
///
/// `entry_trust[i]` = 第 i 个入口的 [`EntryTrust`]；`None`/缺省 ⇒ 那一趟与今天
/// **逐字节相同**（整份重查）⇒ 既有调用方（CLI `build`）行为零变化 ✓。
pub(crate) fn with_project_session_trusted<R>(
    lib_units: &[SourceUnit<'_>],
    entries: &[Vec<SourceUnit<'_>>],
    options: &CompileOptions,
    entry_trust: &[Option<EntryTrust>],
    mut on_entry: impl FnMut(
        usize,
        CompileOutput,
        Vec<DocumentReport>,
        &[DocumentReport],
        &[std::ops::Range<usize>],
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
        builder, None, true, tables, lib_units, options, true, None, None, None, None, None, None,
    );
    tables = lib_tables;
    // **逐模块报告**（与 `check::run` 同构）：库层那趟的报告按单元切分 ⇒ 接线方
    // 能组装出与今天**逐字节相同**的 `ProjectReport`（缓存内容不变）。
    let lib_n = lib_pass.n_commands;
    // 读在 `lib_pass.report` 被搬走**之前**（`split_report` 会吃掉它）。
    let lib_checks = lib_pass.kernel_checks();
    let lib_ranges = crate::compile::unit_ranges(lib_units);
    let lib_reports = split_report(
        lib_pass.report,
        &lib_pass.out.error_cmds,
        &lib_pass.out.warning_cmds,
        lib_units,
    );
    // ② 检查点 = "只有库层"的环境（`DeclarMap: Clone` 由 `snapshot()` 已在用）。
    let lib_out = {
        let mut out = lib_pass.out;
        out.stats.kernel_checks = lib_checks;
        out
    };
    let checkpoint = builder.hide_declars();
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry_units) in entries.iter().enumerate() {
        // ③ 回到只有库层的状态 ⇒ 入口之间不共享环境。
        builder.restore_declars(checkpoint.clone());
        // **切片 1 路乙**：入口趟必须拿到"**该入口闭包**"的闭包前缀与记法表 ——
        // 否则入口里的 `judge_infer` **看不到库层声明**（它只吃源码字符串，
        // `judge.rs:949`）⇒ 实测这是三次接线失败的同一个根因
        //（`docs/design/incremental-environment.md` §29）。
        // 该入口闭包 = `lib_units`（本 session 的库层）+ 该入口自己的单元。
        let entry_closure: Vec<SourceUnit<'_>> = lib_units
            .iter()
            .chain(entry_units.iter())
            .map(|u| SourceUnit {
                name: u.name,
                path: u.path,
                file: u.file,
            })
            .collect();
        // ⚠ **取"最后一格"是必须的**（2026-09-29 实测踩到）：入口趟传给
        // `run_pass_with` 的 `units` 是 **`entry_units`（长度 1）** ⇒ walk 里
        // `unit_idx` **恒为 0** ⇒ 它要的是"**入口那一格**"的前缀
        // = `entry_prefixes` 的**最后一格**（前几格属于库单元，第一格还是**空串**）。
        // 直接把整个 `entry_prefixes` 传进去 ⇒ `get(0)` = 空串 ⇒ 走 `_ =>` 分支
        // ⇒ **入口没有库层前缀** ✗ ⇒ 实测报「前缀源码无法解析」+ 一串记法解析失败
        // （`≠`/`{a,b}`/`=`/`∈` 全都"读不到目标类型"）。
        let entry_prefixes_all = crate::compile::closure_prefixes_for(&entry_closure);
        let entry_prefixes: Vec<String> = entry_prefixes_all
            .last()
            .map(|last| vec![last.clone()])
            .unwrap_or_default();
        let entry_display = crate::compile::display_notations(&entry_closure);
        // **跨模块 hover 回填**（切片 1b 的入口趟）：`resolution` 要指向**库层**声明
        // 的真实 span，而入口趟的 `units` 只有入口 ⇒ 不传这张表的话，入口里
        // `Point`（来自 `import Lib`）的 hover `resolution` 会退化成 `None`
        // ⇒ F12/高亮在跨模块名字上失效 ✗（实测：与会话外整份编译的报告因此不同）。
        let entry_defs = crate::compile::top_level_def_spans_over(&entry_closure);
        // **S2 步 2**：该入口这一趟的信任前缀（缺省 = 整份重查，与今天逐字节相同）。
        let trusted = entry_trust.get(index).and_then(|slot| slot.as_ref());
        let (pass, next, next_tables) = run_pass_with(
            builder,
            None,
            false,
            tables,
            entry_units,
            options,
            true,
            trusted.map(|t| &t.failures),
            trusted.map(|t| &t.plan),
            None,
            Some(&entry_prefixes),
            Some(&entry_display),
            Some(&entry_defs),
        );
        builder = next;
        tables = next_tables;
        let entry_range = lib_n..lib_n + pass.n_commands;
        // 读在 `pass.report` 被搬走**之前**（`split_report` 会吃掉它）。
        let entry_checks = pass.kernel_checks();
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
        // **S2 步 2 顺带修的一个漏**：`run` 会把 `pass.checks` 搬进
        // `out.stats.kernel_checks`，而这条路以前**只加了没赋值的那个 0** ⇒
        // 合并输出里的 `kernel_checks` 恒为 0（判据读不到"少查了多少"）。
        merged.stats.kernel_checks += entry_checks;
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
