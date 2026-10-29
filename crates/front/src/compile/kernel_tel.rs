//! **P1-F1：按需内核 telescope**（`reports/2026-10-29-strict-replan.md` 的 P0 telescope 段 + F1 行）。
//!
//! 严格档的应用 elaborate（P1-F2，对照 Lean `Lean/Elab/App.lean` 的 `elabAppArgs`）要每个被调者的
//! **参数望远镜**：名字、binder 档、**内核**类型。来源只许是**内核声明类型**
//! （`builder.with_env(|ef| ef.declars.get(..))`，先例 `check/decl_card.rs`），不许是源级签名文本。
//!
//! 规则：
//! * 走**语法上**的整条 Pi 链，**不做 whnf**（`Set α` 这种结果类型由 F2 自己 whnf）；
//! * 前 `implicit_prefix` 个参数一律 `Implicit`，不管 `binder_style`（构造子的归纳参数在内核类型里是
//!   `Default`，语义上是隐式 —— `KnownName::Decl::implicit_prefix` 就是这么登记的）；其余按 `binder_style`；
//! * 声明不在环境里 / 不是 `Decl` ⇒ `None`（调用方回落今天的路径）。

use super::elab::KnownName;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::expr::{BinderStyle, Expr};
use sokonanoda::level::Level;
use sokonanoda::util::ExprPtr;

/// 应用 elaborate 关心的 binder 档（`StrictImplicit` 归入 `Implicit`：本仓不区分严格隐式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamKind {
    Explicit,
    Implicit,
    InstImplicit,
}

/// 望远镜的一个参数：binder 名、档、**内核**类型（可能引用前面的参数 —— de Bruijn 未实例化）。
// 未来的消费者：P1-F2 的内核级 `elab_app`（`ty` 今天只有单测读）。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct KernelParam<'a> {
    pub(crate) name: String,
    pub(crate) kind: ParamKind,
    pub(crate) ty: ExprPtr<'a>,
}

/// 一个声明的内核望远镜：宇宙参数名 + 参数 + 剥完 Pi 之后的结果类型。
// 未来的消费者：P1-F2 的内核级 `elab_app`（字段今天只有单测读）。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct KernelTelescope<'a> {
    pub(crate) universes: Vec<String>,
    pub(crate) params: Vec<KernelParam<'a>>,
    pub(crate) result: ExprPtr<'a>,
}

/// `canonical` 的内核望远镜（`entry` 是它在 `known` 里的登记；非 `Decl` / 环境里没有 ⇒ `None`）。
// 未来的消费者：P1-F2 的内核级 `elab_app`（今天只有单测调用 ⇒ 零行为变化）。
#[allow(dead_code)]
pub(crate) fn kernel_telescope<'a>(
    builder: &mut EnvBuilder<'a>,
    entry: &KnownName,
    canonical: &str,
) -> Option<KernelTelescope<'a>> {
    let KnownName::Decl {
        implicit_prefix, ..
    } = entry
    else {
        return None;
    };
    let ptr = builder.name_from_str(canonical);
    // `with_env` 期间 dag 被挪走 ⇒ 只把 `Copy` 的 `DeclarInfo` 带出来，走查在外面做。
    let info = builder.with_env(|ef| ef.declars.get(&ptr).map(|d| *d.info()))?;
    let universes = info
        .uparams
        .iter()
        .map(|l| match **l {
            Level::Param(name, _) => builder.name_to_string(name),
            // 声明的 uparams 恒为 `Param`；万一不是，照实给出调试文本而不是编一个名字。
            other => format!("{other:?}"),
        })
        .collect();
    let mut params = Vec::new();
    let mut cur = info.ty;
    while let Expr::Pi {
        binder_name,
        binder_style,
        binder_type,
        body,
        ..
    } = *cur
    {
        let kind = if params.len() < *implicit_prefix {
            ParamKind::Implicit
        } else {
            match binder_style {
                BinderStyle::Default => ParamKind::Explicit,
                BinderStyle::Implicit | BinderStyle::StrictImplicit => ParamKind::Implicit,
                BinderStyle::InstanceImplicit => ParamKind::InstImplicit,
            }
        };
        params.push(KernelParam {
            name: builder.name_to_string(binder_name),
            kind,
            ty: binder_type,
        });
        cur = body;
    }
    Some(KernelTelescope {
        universes,
        params,
        result: cur,
    })
}
