//! 可信内置声明（prelude）的安装：Nat 骨架、Eq 三件套与可选性配置。
//!
//! prelude 是「受信任的预置」：安装后不再被内核重查（不进 PendingOp）。
//! 教学文件可以在两种模式下编译（用户要求）：
//! * `PreludeMode::Full` —— 安装全部内置基元（Nat/Eq，若未被文件自带声明占用）；
//! * `PreludeMode::Bare` —— 完全不安装任何东西，课程从零构造一切
//!   （例如自带 `inductive Nat` 块或纯逻辑公理文件）。

use super::elab::{build_axiom, install_inductive_block, ElabCtx, InductiveTable};
use crate::{Binder, BinderKind, Command, CtorDecl, Expr, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::ExprPtr;
use std::collections::HashMap;

/// Which trusted base declarations a compilation installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreludeMode {
    /// Install the built-in teaching prelude (Nat + Eq, unless the file
    /// declares its own versions).
    #[default]
    Full,
    /// Install nothing: the file must be self-contained. `1 + 1` and `Nat`
    /// only work if the file provides them.
    Bare,
}

/// Options for one compilation session.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptions {
    pub prelude: PreludeMode,
}

/// Read the file-level prelude directive from `--` comment lines:
/// `-- sokonanoda:prelude none` (or `bare`) selects `PreludeMode::Bare`,
/// `-- sokonanoda:prelude full` selects `Full`. The flag stays declarative:
/// it is a comment, so the file remains a plain text canvas.
pub fn prelude_mode_from_source(src: &str) -> PreludeMode {
    explicit_prelude_mode(src).unwrap_or(PreludeMode::Full)
}

/// 文件**显式**写了 `-- sokonanoda:prelude …` 指令时返回它，否则 `None`。
/// 项目闭包里"没写指令"= 继承入口的模式（设计 §4.6），只有**显式冲突**
/// 才是 `import-prelude-conflict`。
pub fn explicit_prelude_mode(src: &str) -> Option<PreludeMode> {
    for line in src.lines() {
        let trimmed = line.trim_start();
        let Some(comment) = trimmed.strip_prefix("--") else {
            continue;
        };
        let comment = comment.trim();
        let Some(rest) = comment.strip_prefix("sokonanoda:prelude") else {
            continue;
        };
        let value = rest.trim();
        return Some(match value {
            "none" | "bare" => PreludeMode::Bare,
            _ => PreludeMode::Full,
        });
    }
    None
}

/// Trusted equality primitives, written in the teaching syntax itself and
/// installed without re-checking (like the Nat prelude). The signatures match
/// official Lean's `Eq`/`Eq.refl`/`Eq.subst`, so a filled exercise file that
/// uses them still checks in real Lean.
/// The trusted prelude's top-level names (Full mode). Completions material:
/// prelude declarations are trusted installs without `DeclState`s, so the
/// goal view / completion layer needs this list to offer them.
pub const PRELUDE_NAMES: &[&str] = &[
    "Nat",
    "Nat.zero",
    "Nat.succ",
    "Nat.rec",
    "Nat.add",
    "Bool",
    "Bool.true",
    "Bool.false",
    "Bool.rec",
    "Eq",
    "Eq.refl",
    "Eq.subst",
];

pub(crate) const PRELUDE_EQ_SRC: &str = "\
axiom Eq {u} : {α : Sort u} -> α -> α -> Prop
axiom Eq.refl {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a
axiom Eq.subst {u} : {α : Sort u} -> {p : α -> Prop} -> {a : α} -> {b : α} -> Eq.{u} α a b -> p a -> p b
";

/// Install the Eq prelude, skipping the whole block when the file declares
/// any of the names itself (all-or-nothing, mirroring the explicit `Nat`
/// block behavior: the file then owns equality entirely).
/// `known` gains the universe-parameter arity of each installed axiom.
pub(crate) fn install_eq_prelude(
    builder: &mut EnvBuilder<'_>,
    known: &mut HashMap<String, Vec<String>>,
    taken: &std::collections::HashSet<String>,
) {
    const EQ_NAMES: [&str; 3] = ["Eq", "Eq.refl", "Eq.subst"];
    if EQ_NAMES.iter().any(|name| taken.contains(*name)) {
        return;
    }
    let file = crate::parse(PRELUDE_EQ_SRC).expect("Eq prelude source parses");
    let empty: InductiveTable<'_> = InductiveTable::new();
    let options = CompileOptions::default();
    let ctx = ElabCtx {
        prefix_src: "",
        options: &options,
        inductives: &empty,
    };
    for command in &file.commands {
        let Command::Axiom {
            name, universe, ty, ..
        } = command
        else {
            panic!("Eq prelude must only contain axioms");
        };
        if taken.contains(name) {
            continue;
        }
        let mut hovers = Vec::new();
        let decl = build_axiom(builder, name, universe, ty, known, &mut hovers, &ctx)
            .expect("Eq prelude axiom elaborates");
        builder.add_declar(decl).expect("duplicate prelude axiom");
        known.insert(name.clone(), universe.clone());
    }
}

/// Trusted built-in base declarations. These are never re-checked by the
/// kernel. `Nat` is installed as a trusted source-style inductive block
/// (`Nat.zero`/`Nat.succ` constructors + a derived `Nat.rec`) so that `match`
/// on the prelude `Nat` lowers to a real `Nat.rec` and the recursor has a
/// proper iota rule set. `Nat.add` stays the native self-referential
/// definition: the kernel's native Nat reduction is enabled by the matching
/// declaration names.
pub(crate) fn install_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut HashMap<String, Vec<String>>,
    inductives: &mut InductiveTable<'a>,
) {
    let span = Span::default();
    let nat_sort = Expr::Sort {
        sort: SortKind::Type,
        span,
    };
    let nat_ident = Expr::Ident {
        name: "Nat".to_string(),
        span,
    };
    let constructors = vec![
        CtorDecl {
            name: "Nat.zero".to_string(),
            binders: Vec::new(),
            result: nat_ident.clone(),
            span,
        },
        CtorDecl {
            name: "Nat.succ".to_string(),
            binders: vec![Binder {
                name: "n".to_string(),
                ty: Some(Box::new(nat_ident.clone())),
                style: BinderKind::Explicit,
                span,
            }],
            result: nat_ident,
            span,
        },
    ];
    let mut hovers = Vec::new();
    let mut built = Vec::new();
    install_inductive_block(
        builder,
        known,
        inductives,
        "",
        &CompileOptions::default(),
        "Nat",
        &[],
        &nat_sort,
        &constructors,
        None,
        &[],
        &mut hovers,
        &mut built,
    )
    .expect("built-in Nat block installs");

    let anon = builder.anonymous();
    let empty = builder.alloc_levels_slice(&[]);
    let nat = builder.name_from_str("Nat");
    let nat_type = builder.mk_const(nat, empty);

    let inner_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let add_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, inner_arrow);
    let add_name = builder.name_from_str("Nat.add");
    let add_levels = builder.alloc_levels_slice(&[]);
    let add_self = builder.mk_const(add_name, add_levels);
    add_definition(builder, "Nat.add", add_arrow, add_self);
    known.insert("Nat.add".to_string(), Vec::new());
}

/// Trusted built-in `Bool`, installed exactly like the `Nat` block: a
/// source-style inductive with constructors `Bool.true`/`Bool.false` and a
/// derived `Bool.rec`, registered in `known` and the `match` `InductiveTable`.
/// Non-recursive, so the recursor is the plain two-branch eliminator and the
/// kernel needs no change. The names must stay `Bool`/`Bool.true`/`Bool.false`
/// to match the kernel's name cache (frozen; `docs/architecture.md` §5.4).
pub(crate) fn install_bool_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut HashMap<String, Vec<String>>,
    inductives: &mut InductiveTable<'a>,
) {
    let span = Span::default();
    let bool_sort = Expr::Sort {
        sort: SortKind::Type,
        span,
    };
    let bool_ident = Expr::Ident {
        name: "Bool".to_string(),
        span,
    };
    let constructors = vec![
        CtorDecl {
            name: "Bool.true".to_string(),
            binders: Vec::new(),
            result: bool_ident.clone(),
            span,
        },
        CtorDecl {
            name: "Bool.false".to_string(),
            binders: Vec::new(),
            result: bool_ident,
            span,
        },
    ];
    let mut hovers = Vec::new();
    let mut built = Vec::new();
    install_inductive_block(
        builder,
        known,
        inductives,
        "",
        &CompileOptions::default(),
        "Bool",
        &[],
        &bool_sort,
        &constructors,
        None,
        &[],
        &mut hovers,
        &mut built,
    )
    .expect("built-in Bool block installs");
}

fn add_definition<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Definition {
            info,
            val,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("duplicate builtin definition");
}
