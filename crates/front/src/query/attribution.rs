//! **P5**：`query check` 的 `failed[]` 归到**声明名**。
//!
//! `check()` 的 `output.errors` 只有 span、没有名字；`FailedDecl::name` 长期恒
//! `None`（2026-10-20 P4 被它误导，把 2 条真实声明失败当成「诊断泄漏」）。
//! 归因走**报告里已有的失败声明状态**（`DocumentReport::decl_states`），
//! 不按文本猜、不重跑判定；找不到对应状态（parse 错误、project 级错误）就保持
//! `None`，与旧行为一致。

use crate::compile::{CompileError, DeclStatus, DocumentReport};

/// 把一条编译错误归到**失败的声明名**；归不上 ⇒ `None`（不是空结果）。
pub(crate) fn failed_decl_name(
    report: Option<&DocumentReport>,
    error: &CompileError,
) -> Option<String> {
    let report = report?;
    let at = error.span.start.offset;
    report
        .decls
        .iter()
        .filter(|st| st.status == DeclStatus::Failed)
        .find(|st| {
            st.span.start.offset == at || (st.span.start.offset <= at && at < st.span.end.offset)
        })
        .and_then(|st| st.name.clone())
}
