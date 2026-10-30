//! **P2-5**：内核应用路径第一趟的**探测模式**（Lean `Elab/App.lean:433-443` + `TermElabM.lean` 1449-1452 /
//! 1663-1679：推迟一个实参很便宜，发现"它得推迟"这件事本身不做昂贵的工作）。
//!
//! [`super::kernel_app`] 的 `run` 第一趟无期望地 elaborate 实参（"试一下，不行就推迟"）。这段时间
//! [`ProbeGuard::enter`] 记下"正在探测的是**这个实参节点**"（builder 地址 + span + scope 深度）。那个节点自己
//! 是应用 / 记法、内核尝试解不出（见 [`cuts`]）⇒ **不**回落老路（老路要 `judge_infer` = 整份前缀重编），改报
//! 一条便宜的 elaborate 错误 ⇒ 外层把这一位推迟，第二趟带着域再 elaborate（不在探测里，老路照常可走）；
//! 第二趟也不行 ⇒ 外层照今天回落老路。⇒ 只省掉老路**投机**做的那一份，工作只减不增。
//!
//! 只认**正好是那个实参节点**（同 span、同深度、同 builder）：实参内部更深的子项（λ 体、老路里的分支、
//! `judge_infer` 嵌套前缀重编用的另一个 builder）都看不见探测 ⇒ 不会有吞掉探测错误、改走别的分支的地方。
//! 实参本身又是走内核的应用 ⇒ 它自己的第一趟再 `enter`，链式往下。
//!
//! 守卫是 RAII：每个出口（含 `?` 早退、panic 展开）都还原进入前的状态。闸门关 ⇒ 调用方不进本模块。

use super::elab::ElabScope;
use super::error::{CompileError, ErrorKind};
use super::kernel_app_miss::MissGuard;
use crate::Span;
use sokonanoda::builder::EnvBuilder;
use std::cell::Cell;

type Owner = (usize, Span, usize);

thread_local! {
    /// `Some((builder 地址, 实参 span, scope 深度))` ⇒ 正在第一趟里探测这个实参节点。
    static OWNER: Cell<Option<Owner>> = const { Cell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// 测试用：`true` ⇒ 探测永不生效（反向验证：P2-5 之前的行为）。
    pub(crate) static TEST_NO_PROBE: Cell<bool> = const { Cell::new(false) };
}

fn owner(builder: &EnvBuilder<'_>, span: Span, scope: &ElabScope<'_>) -> Owner {
    (
        builder as *const EnvBuilder<'_> as *const () as usize,
        span,
        scope.len(),
    )
}

/// 探测状态的存续期（drop ⇒ 还原成进入前的值）。
#[must_use]
pub(crate) struct ProbeGuard(Option<Owner>);

impl ProbeGuard {
    /// 第一趟 elaborate 实参 `span`（在 `scope` 当前深度下）：探测它。
    pub(crate) fn enter(builder: &EnvBuilder<'_>, span: Span, scope: &ElabScope<'_>) -> Self {
        let me = owner(builder, span, scope);
        ProbeGuard(OWNER.with(|o| o.replace(Some(me))))
    }
}

impl Drop for ProbeGuard {
    fn drop(&mut self) {
        OWNER.with(|o| o.set(self.0));
    }
}

/// 探测中哪些回落原因**不**走老路：内核真跑过 `elab_app`、只是无期望时解不出（unit12 实测 12/12 的
/// `unsolved` 未命中都是这一类、都在外层第一趟里）。`shape` / 宇宙多态头 / `argmismatch` 等照旧走老路。
fn cuts(why: &str) -> bool {
    why == "unsolved"
}

/// 内核路径回落（`why` = 已计数的原因）：这个节点正被外层第一趟探测、且原因可切 ⇒ 便宜的 elaborate 错误
/// （作用域 / hover 已由 `kernel_app::finish` 照回落出口撤回；外层第一趟接住它、推迟这一位）；
/// 否则 ⇒ 老路照走，返回归因用的 [`MissGuard`]（调用方持有到返回，同今天）。
pub(crate) fn fallback(
    builder: &EnvBuilder<'_>,
    why: &'static str,
    span: Span,
    scope: &ElabScope<'_>,
) -> Result<MissGuard, CompileError> {
    #[cfg(test)]
    let off = TEST_NO_PROBE.with(|c| c.get());
    #[cfg(not(test))]
    let off = false;
    let probing = !off && OWNER.with(|o| o.get()) == Some(owner(builder, span, scope));
    if probing && cuts(why) {
        return Err(CompileError::elab(
            ErrorKind::ElabImplicitArgumentUnsolved,
            "内核应用路径：无期望类型时解不出（推迟到按域 elaborate）",
            span,
        ));
    }
    Ok(MissGuard::push(why))
}
