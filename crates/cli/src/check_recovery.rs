//! **单文件 parse 失败时的恢复式编译**（G-117，2026-10-15 用户「有问题就修」）。
//!
//! 本模块只有一个函数，但它承担一条**构造性**保证：`grade --json` 与
//! `query check` 是**同一份编译的两个视图**（`crates/cli/tests/query.rs` 头注释：
//! "they are two views of the same compile — a divergence would mean a second source
//! of truth"）⇒ 同一份文本**只许有一个编译者** ✓。
//!
//! * `query check` 那条路：`QueryDoc::set_text`（`crates/front/src/query/mod.rs:523`）
//!   → `Session::update`（`crates/front/src/session.rs:140`）。N2 起它就带**恢复式
//!   解析**：一条没写完的声明不再让整份文件的报告消失。
//! * `grade` 那条路（本模块之前）：`check_source` 的 `Err(diag)` 分支**直接印诊断
//!   就返回** ⇒ 零 `decl.checked`、零声明 —— 两个消费者各走一条路 ✗。
//!
//! 修法是**让第二条路调用同一个 `Session::update`**（不是"再写一遍恢复逻辑" ✗）：
//! 两个视图的一致性因此是**构造出来的**，不是对出来的 ✓。
//!
//! 拆成独立文件（而不是塞进 `check.rs`）：`AGENTS.md` 硬规则 5 + `docs-lint`
//! 判据 ⑨ 的方向是"新代码进新文件"，`check.rs` 只留分发。
//!
//! **改前/改后**（夹具 `a` 好 / `broken` 坏 / `c` 好：
//! `docs/gaps/repro/G117-grade-json-vs-query-check-divergence.sh`）：
//!
//! | 通道 | 改前 | 改后 |
//! |---|---|---|
//! | `query check` | `decl_checked=2` · `failed=[unexpected-token]` | 不变 ✓ |
//! | `grade --json` | 1 条 diagnostic · **0 条 `decl.checked`** ✗ | 2 条 `decl.checked` + 1 条 diagnostic ✓ |
//!
//! ⚠ **退出码不变**：恢复出来的报告里一定至少有一条诊断（触发恢复的那条）
//! ⇒ [`compile_recovering`] 的 `errors` 永不为空 ⇒ `grade` 仍 exit 1 ✓
//! （判据：「不许为了让两视图一致就把错吞掉」）。

use sokonanoda_front::compile::{CompileError, CompileOptions, CompileOutput};
use sokonanoda_front::session::Session;
use sokonanoda_front::Diagnostic;

/// parse 失败的**单文件**回退：走恢复式编译，返回与 `check_source` 同一口径的产物。
///
/// 与 `QueryDoc::set_text` 的单文件分支**逐条对齐**：同一个 `Session::update`、
/// 同一个版本号（1）、同一份 `events` / `report.errors` / `report.warnings` 装配
/// ⇒ `report_json` / `report_output` 印出来的就是 `query check` 读到的那份编译 ✓。
///
/// ⚠ **不传 `PreludeMode::Full` 硬编码**：`options` 是调用方（`--bare` 标志或
/// `-- sokonanoda:prelude none` 指令）算出来的**这份文件该用的**模式 —— 回退分支
/// 用另一套模式就又造了第二个真相源 ✗。
pub(crate) fn compile_recovering(
    src: &str,
    options: CompileOptions,
    diag: &Diagnostic,
) -> CompileOutput {
    let mut session = Session::new(options);
    let update = session.update(src, 1);
    let mut output = CompileOutput {
        events: update.events,
        errors: update.report.errors,
        warnings: update.report.warnings,
        ..Default::default()
    };
    if output.errors.is_empty() {
        // 一条都救不回来 ⇒ **老契约逐字节不变**（空产物 + 那一条 parse 诊断）。
        // 什么时候会到这里：词法失败 / 整份都是坏的 / 唯一那条命令就是坏的 /
        // 恢复被闸门关掉（`SOKO_NO_PARSE_RECOVERY=1`，或源码里有 `import` 行 ——
        // `Session::update` 的 `parse_recovery_enabled`，与 `query check` 同一道闸门 ✓）。
        //
        // `CompileError::from_parse` 让码 / 提示 / span / stage 与 `Diagnostic`
        // **同源**（`crates/front/src/compile/error.rs:475`）⇒ 这一行 `--json` 与
        // 改前**逐字节相同** ✓（改前的形态见本模块头那张表）。
        output.errors.push(CompileError::from_parse(diag));
    }
    output
}
