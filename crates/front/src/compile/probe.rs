//! **请求期 kernel 探针的入口与 panic 兜底**（自 `goals.rs` 拆出，2026-11-04）。
//!
//! 拆出理由：`docs-lint` ⑨ —— `goals.rs` 在软线之上、**只许减不许增**；而本单
//! （issue #7 第 3 条）要给请求期探针加一层 `catch_unwind`。
//!
//! 正文仍在 [`crate::compile::goals::probe_sub_goal_types_inner`]（它要用 `goals.rs`
//! 的 `ProbeEnv` / `GoalTemplates` / `goal_under_binders`）。

use super::goals::probe_sub_goal_types_inner;
use crate::compile::CompileOptions;
use crate::span::Span;

/// 请求期 kernel 探针入口（方案 A，front-only，内核零改动）：
/// 重解析文档、定位开放练习声明，再用 [`judge_infer`] 为**函数 spine 子洞**
/// 重算期望类型。返回 `(洞起点 offset, 类型文本)`；调用方按 span 覆盖
/// report 中为 `None` 的 `sub_goals[i].ty`。
///
/// 纪律（设计稿见 `git log --all -- docs/design/spine-meta-a.md`）：只在 LSP 的
/// `soko/goals` / inlay / hover 请求路径调用，绝不进 didChange / keystroke
/// 路径；`open_goal` 的廉价 AST 走查仍是「是否 Open + 洞 span」的唯一来源。
/// 算不出（深层嵌套、无类型 binder 等）就返回空——不倒退、永不比 B′ 差。
pub fn probe_sub_goal_types(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
) -> Vec<(usize, String)> {
    probe_sub_goal_types_with("", doc_src, options, decl_span)
}

/// 同 [`probe_sub_goal_types`]，但把 `extra_prefix`（闭包上下文）交给探针：
/// 项目模式下子洞期望类型才会用**闭包环境**算（否则被导入的名字看不见）。
///
/// **内核交互一律包 `catch_unwind`**（`docs/architecture.md` §8 gotcha 0）。这是
/// **请求期**探针（`soko/goals` / inlay / hover），**不在**编译路径上 ⇒ 没有
/// `quiet_catch` 兜住；而内核用 panic 报拒绝。用户 2026-11-04（issue #7 第 3 条）
/// 的描述是「写错一条 tactic 之后 lsp server 直接死掉，只能 Reload Window」——
/// 请求期一次 panic 就足以让**那个请求永远不回答**（tower-lsp 的 handler 被 panic
/// 之后连接仍在，客户端等的是一个不会来的响应）⇒ 编辑器"像死了一样"。
/// 探针本来就是**尽力而为**（"算不出就返回空——不倒退、永不比 B′ 差"）⇒ panic 也
/// 按同一语义降级 ✓（`catch_unwind` 默认仍会把 panic 打到 stderr，便于下次取证）。
pub fn probe_sub_goal_types_with(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
) -> Vec<(usize, String)> {
    catch_probe_panic(|| probe_sub_goal_types_inner(extra_prefix, doc_src, options, decl_span))
}

/// 请求期探针的 **panic 兜底**（唯一实现；判据 `tests.rs::the_request_probe_swallows_a_kernel_panic`）。
///
/// 抽成函数是为了**能被单测直接咬住**：删掉 `catch_unwind` ⇒ 那条测试当场 panic ✗。
pub(crate) fn catch_probe_panic<R: Default>(f: impl FnOnce() -> R) -> R {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_default()
}
