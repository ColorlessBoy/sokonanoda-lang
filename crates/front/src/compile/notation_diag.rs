//! 记法路径的**诊断保真**（a12，2026-10-18）：记法自己解不出前导类型参数时，
//! **先逐位问一遍操作数** —— 真错常常在操作数身上（把隐式位写成位置实参 ⇒ 严格档
//! 拒绝），而记法文案会把它顶掉、把用户指到错地方 ✗。
//!
//! **两处实测**（都是「记法替操作数背锅」）：
//! * a10：`Quot.lift.{1, 1} α r β f h (Quot.mk.{1} α r a) = f a` —— 操作数自己的
//!   严格档拒绝**带着它自己的 span** 上来，被 `elab_notation_implicit` 换成了
//!   「记法 `=` 补不出前面的类型参数」✗（`=` 走**隐式档**）；
//! * a12：`∃ (x : α), Eq.{1} β (f x) y` —— 操作数的错**连 span 都到不了**记法这层
//!   （它被前缀求解的**类型查询**吞掉了）⇒ 只按 span 判别会漏，必须**真去 elaborate** ✓；
//!   而且 `∃` 走的是**另一条**路：`Exists` 的 binder 是**显式**的 ⇒ `implicit_prefix == 0`
//!   ⇒ `elab.rs` 的 k==0 档（`notation_prefix_args` 只回 `None`，连 `Err` 都没有 ✗）
//!   ⇒ **两个放弃点都要挂**本模块 ✓（`elab_notation` 的 k==0 档与 `elab_notation_implicit`）。
//!
//! **为什么单独一个文件**：`elab.rs` 受 `docs-lint` 判据 ⑨ 冻结（**只许减不许增** ✗）
//! ⇒ 这段机械长在那边会当场判红 ✓（与 `implicit_msg` 同源，2026-10-16）。
//!
//! **判据一条没放宽**：本模块只挑**哪条诊断**报出来，不改变任何接受/拒绝 ——
//! 调用方两条路都是 `Err` ✓（同 a10 的先例：放行后必然 `Ok(None)` ⇒ 交回既有逐层路径）。

use super::elab::{elab_expr, ElabCtx, ElabScope, HoverNode, KnownTable, UnivMap};
use super::error::{CompileError, ErrorKind};
use crate::Expr;
use sokonanoda::builder::EnvBuilder;

/// 逐位 elaborate 操作数；谁报**严格档拒绝**（`ElabImplicitArgumentUnsolved`）
/// 就把**它的**诊断交出去 ✓；其余错类保持原样（免得误报 ✗）⇒ `None`。
///
/// **只在记法已经失败之后调用**（错误路径）⇒ 成功路径零开销 ✓；返回的第一条
/// 就交出去（不再往下试），所以最坏代价 = 操作数个数的 elaborate ✓。
pub(crate) fn operand_rejection<'a>(
    builder: &mut EnvBuilder<'a>,
    operands: &[&Expr],
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    ctx: &ElabCtx<'a, '_>,
) -> Option<CompileError> {
    for operand in operands {
        // 探针**不写 hovers**：这里已在错误路径上（声明必红 ⇒ 悬停节点没有消费者），
        // 而调用方可能正在**试探**一条路 ⇒ 污染它的悬停输出 ✗。
        let mut sink: Vec<HoverNode<'a>> = Vec::new();
        if let Err(e) = elab_expr(
            builder, operand, scope, univ, known, &mut sink, None, None, ctx,
        ) {
            if e.kind == ErrorKind::ElabImplicitArgumentUnsolved {
                return Some(e);
            }
        }
    }
    None
}
