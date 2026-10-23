//! **IA-4 K1 的硬不变式**（设计 `docs/design/metavar-engine.md` §2.11 ✓）。
//!
//! D8 = **(i) 内核占位符** ✓：内核 `Expr` 加 `Meta` 构造子 ✓，**判定层硬拒**含它的声明 ✓，
//! 前端协议**不变** ✓。本片（K1）**没有任何东西构造占位符** ✗ ⇒ **零行为变化** ✓；
//! 它给的是**结构性保证**：等 B2 真把元变量送进来时，"判定层见不到它"已经由入口挡住了 ✓。
//!
//! 本文件钉**两条**（设计 §2.11 的"共同硬不变式" ✓）：
//! ① **含占位符的声明一律拒绝** ✓（固定错误前缀 ✓，前端据此映射错误码 ✓）；
//! ② **走查必须走遍所有子项** ✗ —— 只查顶层就等于留缝 ✗（本文件专门钉**深处**那一例 ✓）。
use crate::builder::EnvBuilder;
use crate::env::{Declar, DeclarInfo};
use stumpalo::Arena;

/// 造一个最小环境 + 几个项 ✓。返回 `(arena, builder)` 的持有者 ✓。
struct Fixture<'a> {
    _arena: Box<Arena>,
    builder: EnvBuilder<'a>,
    prop: crate::util::ExprPtr<'a>,
    meta: crate::util::ExprPtr<'a>,
}

fn fixture<'a>(arena: &'a Arena) -> (EnvBuilder<'a>, crate::util::ExprPtr<'a>, crate::util::ExprPtr<'a>) {
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Default::default());
    let prop = builder.mk_sort(builder.zero());
    let meta = builder.mk_meta(7);
    (builder, prop, meta)
}

/// **硬不变式①**：类型位带占位符 ⇒ `add_declar` **必须拒绝** ✓，且消息有**固定前缀** ✓。
#[test]
fn a_declaration_whose_type_contains_a_placeholder_is_rejected() {
    let arena = Arena::new();
    let (mut builder, _prop, meta) = fixture(&arena);
    let name = builder.name_from_str("bad_ty");
    let d = Declar::Axiom {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: meta,
        },
    };
    let err = builder
        .add_declar(d)
        .expect_err("含占位符的声明**必须**被拒 ✗（K1 硬不变式①）");
    assert!(
        err.starts_with("declaration contains a metavariable placeholder"),
        "错误消息必须有**固定前缀** ✓（前端据此映射错误码 ✓）：{err:?}"
    );
}

/// **硬不变式①（值位）**：`Definition` 的值里带占位符 ⇒ 同样拒绝 ✓。
#[test]
fn a_definition_whose_value_contains_a_placeholder_is_rejected() {
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena);
    let name = builder.name_from_str("bad_val");
    let d = Declar::Definition {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: prop,
        },
        val: meta,
        hint: crate::env::ReducibilityHint::Regular(0),
    };
    let err = builder
        .add_declar(d)
        .expect_err("值位含占位符**也必须**被拒 ✗");
    assert!(err.starts_with("declaration contains a metavariable placeholder"));
}

/// ⚠ **硬不变式②（本文件最要紧的一条）**：占位符**埋在深处**也必须被找到 ✗。
///
/// 为什么专门钉它：只查顶层的实现会**放过**这一例 ⇒ 那就是"含元变量的声明能进环境"的缝 ✗
/// （设计 §2.11 要求的是**一律拒绝** ✓，不是"大多数拒绝" ✗）。
#[test]
fn a_placeholder_nested_deep_inside_the_type_is_still_rejected() {
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena);
    // `(Prop → ?m7) → Prop`：占位符在**两层里面** ✓。
    let anon = builder.name_from_str("_");
    let inner = builder.mk_pi(anon, crate::expr::BinderStyle::Default, prop, meta);
    let outer = builder.mk_pi(anon, crate::expr::BinderStyle::Default, inner, prop);
    assert!(
        builder.contains_meta(outer),
        "走查**必须**走遍所有子项 ✗ —— 只查顶层就会放过这一例 ✗"
    );
    let name = builder.name_from_str("deep");
    let d = Declar::Axiom {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: outer,
        },
    };
    assert!(builder.add_declar(d).is_err(), "深处带占位符 ⇒ 一样拒 ✓");
}

/// **对照组** ✓：**没有**占位符的声明**照常通过** ✓ ——
/// 否则这条守卫就是"一律拒绝"的空转 ✗（`AGENTS.md`：咬不住的守卫等于没有 ✓）。
#[test]
fn a_placeholder_free_declaration_is_still_accepted() {
    let arena = Arena::new();
    let (mut builder, prop, _meta) = fixture(&arena);
    let name = builder.name_from_str("ok_ax");
    let d = Declar::Axiom {
        info: DeclarInfo {
            name,
            uparams: builder.alloc_levels_slice(&[]),
            ty: prop,
        },
    };
    builder
        .add_declar(d)
        .expect("没有占位符的声明**必须**照常通过 ✓（否则守卫是空转 ✗）");
}

/// **IA-4 B2 第一片（2026-10-05 ✓）**：元变量的**种类** ✓ ——
/// 判据两条，**都必须能咬住** ✗（`AGENTS.md`：咬不住的守卫等于没有 ✓）：
/// ① **哈希必须含 `kind`** ✗ —— 否则 `Natural` 与 `SyntheticOpaque` 的**同 id**
///    占位符会**撞哈希** ✗（结构共享/去重会把两者混为一谈 ✗）；
/// ② `is_synthetic_opaque` 要**只认** `SyntheticOpaque` ✓（对齐 Lean
///    `MetavarKind.isSyntheticOpaque`，`MetavarContext.lean:282` ✓）。
///
/// ⚠ **反向验证** ✓：把 `mk_meta_with_kind` 里的 `salt` 去掉 ⇒ 第 ① 条**必须判红** ✗
/// （实测过 ✓ —— 见 commit message ✓）。
#[test]
fn meta_kind_is_part_of_identity_and_predicate_is_exact() {
    let arena = Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), Default::default());
    let natural = builder.mk_meta_with_kind(7, crate::expr::MetaKind::Natural);
    let opaque = builder.mk_meta_with_kind(7, crate::expr::MetaKind::SyntheticOpaque);
    let legacy = builder.mk_meta(7);
    // ① **同 id、不同 kind ⇒ `hash` 必须不同** ✓。
    // ⚠ **不许用 `ExprPtr` 的相等性判** ✗ —— 它比的是 **arena 地址**（`util.rs:224` ✓），
    //   与 `hash` 无关 ⇒ **咬不住** ✗（实测：去掉 `salt` 后它**照样通过** ✓ —— 反向验证抓到的 ✓）。
    let hash_of = |e: crate::util::ExprPtr<'_>| match e.as_ref() {
        crate::expr::Expr::Meta { hash, .. } => *hash,
        other => panic!("这里只该出现占位符 ✗，实际是 {other:?}"),
    };
    assert_ne!(
        hash_of(natural),
        hash_of(opaque),
        "**同 id 不同 kind 的占位符不许同 `hash`** ✗（`hash` 必须含 `kind` ✓）"
    );
    // ② 谓词**只认** `SyntheticOpaque` ✓。
    assert!(!crate::expr::MetaKind::Natural.is_synthetic_opaque());
    assert!(crate::expr::MetaKind::SyntheticOpaque.is_synthetic_opaque());
    // ③ `mk_meta`（K1 的旧入口 ✓）**仍造 `Natural`** ✓ ⇒ 本片零行为变化 ✓。
    assert_eq!(
        hash_of(legacy),
        hash_of(natural),
        "`mk_meta` 必须与 `mk_meta_with_kind(_, Natural)` **同 `hash`** ✓（零行为变化 ✓）"
    );
}

/// **SPIKE（2026-10-23）**：内核能否在"有元变量上下文"时**推断元变量的类型**、并**把它解出来**。
///
/// 这是 P4 的生死判据：by 引擎每一步都要交给内核判（`judge_terms`），
/// 而目标/项里出现 `Expr::Meta` 时，内核必须能：
/// ① `infer` 出类型（从上下文查）；
/// ② `def_eq`/`conv` 把元变量**解出来**（赋值），而不是 panic。
#[test]
fn spike_kernel_can_infer_and_solve_a_meta() {
    use crate::env::EnvLimit;
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena);
    // 一条良类型的项：`p : Prop`（存在量化它，避免依赖任何库）。
    let p_name = builder.name_from_str("p");
    let ty_prop = prop; // `p` 的类型就是 `Prop`
    let levels = builder.alloc_levels_slice(&[]);
    builder
        .add_declar(crate::env::Declar::Axiom {
            info: DeclarInfo { name: p_name, uparams: levels, ty: ty_prop },
        })
        .expect("axiom p : Prop 应被接受");
    let p = builder.mk_const(p_name, levels);

    let (ty_text, refl, solved) = builder.with_env(|ef| {
        ef.with_tc(EnvLimit::PpUnlimited, |tc| {
            // ① 登记：?m7 : Prop
            tc.ctx.mvars.register(7, ty_prop);
            let ty = tc.infer_closed_type(meta);
            let text = tc.with_pp(|pp| pp.pp_expr(ty));
            // ② 反射（不应 panic）
            let refl = tc.def_eq_core(meta, meta);
            // ③ 解出来：?m7 =?= p ⇒ 赋值
            let solved = tc.def_eq_core(meta, p);
            (text, refl, solved)
        })
    });
    eprintln!("[spike] infer(?m7) = {ty_text:?} · refl={refl} · solved={solved}");
    assert!(!ty_text.is_empty(), "infer 必须给出类型");
    assert!(refl, "?m7 ≟ ?m7 必须判等");
    assert!(solved, "?m7 ≟ p 必须能解出来（赋值）");
}

/// **S1（2026-10-23）**：元变量赋值的健全性 —— occurs / `SyntheticOpaque` / 赋值可见 / 缓存不毒化。
#[test]
fn s1_occurs_kind_and_assignment_are_sound() {
    use crate::env::{Declar, EnvLimit};
    use crate::expr::{BinderStyle, MetaKind as KernelKind};
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena); // ?m7 : (待登记)
    let levels = builder.alloc_levels_slice(&[]);
    // p : Prop
    let p_name = builder.name_from_str("p");
    builder
        .add_declar(Declar::Axiom {
            info: DeclarInfo { name: p_name, uparams: levels, ty: prop },
        })
        .unwrap();
    let p = builder.mk_const(p_name, levels);
    // f : Prop -> Prop
    let anon = builder.anonymous();
    let f_ty = builder.mk_pi(anon, BinderStyle::Default, prop, prop);
    let f_name = builder.name_from_str("f");
    builder
        .add_declar(Declar::Axiom {
            info: DeclarInfo { name: f_name, uparams: levels, ty: f_ty },
        })
        .unwrap();
    let f = builder.mk_const(f_name, levels);
    let f_meta = builder.mk_app(f, meta); // f ?m7（含元变量）
    let meta8 = builder.mk_meta(8);

    let (occurs, solved, kind_refused, cache_ok, infer_nested) = builder.with_env(|ef| {
        ef.with_tc(EnvLimit::PpUnlimited, |tc| {
            tc.ctx.mvars.register(7, prop);
            // ① occurs：?m7 ≟ f ?m7 必须拒绝（不赋值）
            let occurs = tc.def_eq_core(meta, f_meta);
            let unassigned = tc.ctx.mvars.value(7).is_none();
            // ② 正常赋值：?m7 ≟ p ⇒ true，且 eval 之后能看到
            let solved = tc.def_eq_core(meta, p);
            // ③ 缓存不毒化：再问一次仍 true
            let cache_ok = tc.def_eq_core(meta, p);
            // ④ SyntheticOpaque 拒绝
            tc.ctx.mvars.register(8, prop);
            tc.ctx.mvars.register_kind(8, KernelKind::SyntheticOpaque);
            let kind_refused = !tc.def_eq_core(meta8, p) && tc.ctx.mvars.value(8).is_none();
            // ⑤ 元变量在类型/项位置都能 infer（不 panic）
            let ty = tc.infer_closed_type(f_meta);
            let ty_text = tc.with_pp(|pp| pp.pp_expr(ty));
            let infer_nested = !ty_text.is_empty();
            (occurs, solved, kind_refused, cache_ok, infer_nested && unassigned)
        })
    });
    eprintln!(
        "[s1] occurs(应当 false)={occurs} · solved={solved} · kind_refused={kind_refused} · \
         cache_ok={cache_ok} · nested_infer={infer_nested}"
    );
    assert!(!occurs, "occurs：?m ≟ f ?m 不许赋值");
    assert!(solved, "?m ≟ p 必须能解出来");
    assert!(cache_ok, "赋值后重复判定必须仍为 true（缓存不毒化）");
    assert!(kind_refused, "SyntheticOpaque 不许被赋值");
    assert!(infer_nested, "含元变量的项必须能 infer（不 panic）");
}

/// **S2 第一步（2026-10-23）**：带种子检查 + 赋值可**渲染成文本**（跨检查的载体）。
/// 同时钉死"每次检查一个新上下文"（未种子的下一次看不见上一次的赋值）。
#[test]
fn s2_seeded_check_and_collectable_assignment() {
    use crate::env::EnvLimit;
    use crate::expr::MetaKind as KernelKind;
    let arena = Arena::new();
    let (mut builder, prop, meta) = fixture(&arena);
    let levels = builder.alloc_levels_slice(&[]);
    let p_name = builder.name_from_str("p");
    builder
        .add_declar(Declar::Axiom {
            info: DeclarInfo { name: p_name, uparams: levels, ty: prop },
        })
        .unwrap();
    let p = builder.mk_const(p_name, levels);
    let seeds = [(7u32, prop, KernelKind::Natural)];

    // 检查 A：种子 ?m7 : Prop，赋值成功，赋值渲染成文本
    let (solved, collected) = builder.with_env(|ef| {
        ef.with_tc_seeded(EnvLimit::PpUnlimited, &seeds, |tc| {
            let solved = tc.def_eq_core(meta, p);
            let text = tc
                .ctx
                .mvars
                .value(7)
                .map(|t| tc.with_pp(|pp| pp.pp_expr(t)));
            (solved, text)
        })
    });
    // 检查 B：新上下文 ⇒ 上一次的赋值看不见（这正是 S2 要解决的；用 S1 的 API 钉住现状）
    let isolated = builder.with_env(|ef| {
        ef.with_tc_seeded(EnvLimit::PpUnlimited, &seeds, |tc| tc.ctx.mvars.value(7).is_none())
    });
    eprintln!("[s2] solved={solved} · collected={collected:?} · next_check_isolated={isolated}");
    assert!(solved, "种子后 ?m7 ≟ p 必须解出");
    assert!(collected.is_some(), "赋值必须能渲染成文本（跨检查的载体）");
    assert!(isolated, "未回灌的下一次检查必须看不见上一次赋值（S2 的动机）");
}
