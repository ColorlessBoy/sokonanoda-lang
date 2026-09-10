//! M0 acceptance: drive the complete kernel from memory without any export
//! file, config file or external tool.

use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{ConstructorData, Declar, DeclarInfo, EnvLimit, RecRule, RecursorData, ReducibilityHint};
use sokonanoda::expr::{BinderStyle, Expr};
use sokonanoda::util::{Config, ExportFile};
use stumpalo::Arena;

/// 回归测试（conv 快路径 soundness 修复）：
/// `(A : Sort 1) -> A` 不可居住，`fun (A : Sort 1) => A` 的类型是
/// `(A : Sort 1) -> Sort 1`，与声明类型的依赖 codomain `$0` 不同，
/// 内核必须拒绝。修复前 conv 的 Pi/Pi body-expr 快路径把"同一 interned
/// `Var 0` 体"的 eval 闭包与 infer 闭包误判为相等，导致该不可居住类型
/// 被接受（官方 Lean 拒绝）。
#[test]
fn dependent_codomain_is_not_inhabited_by_identity_lambda() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let one = b.succ(b.zero());
    let sort1 = b.mk_sort(one);
    let name = b.name_from_str("A");
    let var0 = b.mk_var(0);
    // declared: Pi (A : Sort 1), Var 0   （依赖 codomain，不可居住）
    let ty = b.mk_pi(name, BinderStyle::Default, sort1, var0);
    // val: Lambda (A : Sort 1), Var 0   （类型是 Pi (A : Sort 1), Sort 1）
    let val = b.mk_lambda(name, BinderStyle::Default, sort1, var0);
    let declar = Declar::Definition {
        info: DeclarInfo {
            name: b.name_from_str("t7"),
            uparams: b.alloc_levels_slice(&[]),
            ty,
        },
        val,
        hint: ReducibilityHint::Regular(0),
    };
    b.add_declar(declar.clone()).expect("add declar");
    let env = b.finish();
    let result = env.try_check_declar(&declar);
    assert!(
        result.is_err(),
        "the uninhabited dependent codomain must be rejected"
    );
}

/// 回归测试（对照）：非依赖的身份函数仍然通过——
/// `(A : Sort 1) -> Sort 1 := fun (A : Sort 1) => A` 合法。
#[test]
fn identity_over_sort_still_checks() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let one = b.succ(b.zero());
    let sort1 = b.mk_sort(one);
    let name = b.name_from_str("A");
    let var0 = b.mk_var(0);
    let codomain = b.mk_sort(one);
    let ty = b.mk_pi(name, BinderStyle::Default, sort1, codomain);
    let val = b.mk_lambda(name, BinderStyle::Default, sort1, var0);
    let declar = Declar::Definition {
        info: DeclarInfo {
            name: b.name_from_str("id0"),
            uparams: b.alloc_levels_slice(&[]),
            ty,
        },
        val,
        hint: ReducibilityHint::Regular(0),
    };
    b.add_declar(declar.clone()).expect("add declar");
    let env = b.finish();
    let result = env.try_check_declar(&declar);
    assert!(result.is_ok(), "non-dependent identity must check: {result:?}");
}

/// 回归测试（显示层，2026-09-10）：pp 在打印含**开项**（松散变量）的类型
/// 时不得 panic。`is_implicit_fun` 曾开空 context 推断隐式参数风格，
/// 遇到 Var 头应用（`p a`）或依赖实参展开（`Eq A a a`）会对松散变量
/// panic（`infer: loose bvar` / `eval: loose bvar`），导致 hover / `#check`
/// 的类型文本被吞成空。修复：开项直接返回 `false`（按显式打印）。
#[test]
fn pp_of_dependent_applications_with_loose_bvars_does_not_panic() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let empty = b.alloc_levels_slice(&[]);
    let zero = b.zero();
    let prop = b.mk_sort(zero);
    let one = b.succ(zero);
    let sort1 = b.mk_sort(one);
    let a_name = b.name_from_str("A");
    let p_name = b.name_from_str("p");
    let a_lower = b.name_from_str("a");
    let anon = b.anonymous();

    // Eq : (A : Sort 1) -> A -> A -> Prop
    let var1 = b.mk_var(1);
    let b2 = b.mk_pi(anon, BinderStyle::Default, var1, prop);
    let var0 = b.mk_var(0);
    let b1 = b.mk_pi(anon, BinderStyle::Default, var0, b2);
    let eq_ty = b.mk_pi(a_name, BinderStyle::Default, sort1, b1);
    let eq_name = b.name_from_str("Eq");
    b.add_declar(Declar::Axiom {
        info: DeclarInfo {
            name: eq_name,
            uparams: empty,
            ty: eq_ty,
        },
    })
    .expect("add Eq");

    // refl : (A : Sort 1) -> (a : A) -> Eq A a a
    // （`Eq A a a` 的 `Eq A` 展开需要 eval 松散 A：Const 头依赖应用 panic 站点）
    let eq_const = b.mk_const(eq_name, empty);
    let refl_a = b.mk_var(1);
    let eq_a = b.mk_app(eq_const, refl_a);
    let refl_arg = b.mk_var(0);
    let eq_a_a = b.mk_app(eq_a, refl_arg);
    let refl_arg2 = b.mk_var(0);
    let refl_body = b.mk_app(eq_a_a, refl_arg2);
    let refl_dom = b.mk_var(0);
    let refl_inner = b.mk_pi(a_lower, BinderStyle::Default, refl_dom, refl_body);
    let refl_ty = b.mk_pi(a_name, BinderStyle::Default, sort1, refl_inner);
    let refl_name = b.name_from_str("refl");
    b.add_declar(Declar::Axiom {
        info: DeclarInfo {
            name: refl_name,
            uparams: empty,
            ty: refl_ty,
        },
    })
    .expect("add refl");

    // subst_like : (A : Sort 1) -> (p : A -> Prop) -> (a : A) -> p a
    // （`p a` 是 Var 头应用：`is_implicit_fun(p)` 的 loose bvar in infer 站点）
    let p_dom = b.mk_var(0);
    let p_ty = b.mk_pi(anon, BinderStyle::Default, p_dom, prop);
    let p_var = b.mk_var(1);
    let a_var = b.mk_var(0);
    let subst_body = b.mk_app(p_var, a_var);
    let subst_a_dom = b.mk_var(1);
    let subst_a = b.mk_pi(a_lower, BinderStyle::Default, subst_a_dom, subst_body);
    let subst_p = b.mk_pi(p_name, BinderStyle::Default, p_ty, subst_a);
    let subst_ty = b.mk_pi(a_name, BinderStyle::Default, sort1, subst_p);
    let subst_name = b.name_from_str("subst_like");
    b.add_declar(Declar::Axiom {
        info: DeclarInfo {
            name: subst_name,
            uparams: empty,
            ty: subst_ty,
        },
    })
    .expect("add subst_like");

    let mut env = b.finish();
    // 与教学前端一致（front 设置 proofs=true，否则 `is_proof` 也会在开项上
    // 走空 context 推断——那是另一条路径，见 compile/check.rs）。
    env.config.pp_options.proofs = true;
    env.with_tc(EnvLimit::PpUnlimited, |tc| {
        for (label, name, needle) in [
            ("refl", refl_name, "Eq A a a"),
            ("subst_like", subst_name, "p a"),
        ] {
            let c = tc.ctx.mk_const(name, empty);
            let ty = tc.infer_closed_type(c);
            let printed = tc.with_pp(|pp| pp.pp_expr(ty));
            assert!(
                printed.contains(needle),
                "pp of {label} must contain `{needle}`, got: {printed}"
            );
        }
    });
}

#[test]
fn empty_env_infers_and_reduces_a_closed_lambda() {
    let arena = Arena::new();
    let env = ExportFile::empty(arena.as_arena_ref(), Config::default());

    env.with_tc(EnvLimit::Empty, |tc| {
        // (fun x : Sort 0 => x)
        let prop = tc.ctx.mk_sort(tc.ctx.zero());
        let var = tc.ctx.mk_var(0);
        let lam = tc.ctx.mk_lambda(tc.ctx.anonymous(), BinderStyle::Default, prop, var);

        let ty = tc.infer_closed_type(lam);
        match tc.ctx.read_expr(ty) {
            Expr::Pi { binder_type, body, .. } => {
                assert!(matches!(tc.ctx.read_expr(binder_type), Expr::Sort { .. }));
                assert!(
                    matches!(tc.ctx.read_expr(body), Expr::Sort { .. }),
                    "the inferred codomain of an identity over Sort 0 is Sort 0"
                );
            }
            other => panic!("expected Pi type, got {other:?}"),
        }

        let printed = tc.with_pp(|pp| pp.pp_expr(ty));
        assert_eq!(printed, "Prop -> Prop", "unexpected pretty output: {printed}");

        let reduced = tc.reduce_closed(lam);
        assert!(
            matches!(tc.ctx.read_expr(reduced), Expr::Lambda { .. }),
            "identity lambda should already be in normal form"
        );
    });
}

// ---- 内核冷路径分诊（iota/消去子规则一致性）回归 ----
//
// 显式声明的消去子（rec/iota）由内核与自己的重构规则逐条比对；此前
// 顺序/个数不一致时落进裸 `assert_eq!`，学习者会看到
// "assertion `left == right` failed" 加 interned 指针地址。
// 现在两个站点改成稳定 panic 消息（front 分类器 → `kernel-rec-rule-mismatch`）。

/// `iota` 规则的摆放方式。
enum RuleArrangement {
    /// 按构造子声明顺序：z、s（合法，对照）。
    InOrder,
    /// 写反：s 在前（应被拒绝）。
    Swapped,
    /// 少写一条：只有 z 的规则（应被拒绝）。
    MissingSucc,
}

/// 用内存 API 搭一个与教学前端完全同构的显式归纳块
/// `inductive MyNat : Type` + 两个构造子 + 显式消去子 + iota 规则。
fn build_my_nat_block<'a>(b: &mut EnvBuilder<'a>, arrangement: RuleArrangement) -> Declar<'a> {
    let anon = b.anonymous();
    let empty_levels = b.alloc_levels_slice(&[]);
    let u_name = b.name_from_str("u");
    let u = b.level_param(u_name);
    let u_levels = b.alloc_levels_slice(std::slice::from_ref(&u));

    let nat = b.name_from_str("MyNat");
    let z = b.name_from_str("z");
    let s = b.name_from_str("s");
    let rec_name = b.name_from_str("MyNat.rec");
    let motive = b.name_from_str("motive");
    let mz_name = b.name_from_str("mz");
    let ms_name = b.name_from_str("ms");
    let n_name = b.name_from_str("n");

    let nat_const = b.mk_const(nat, empty_levels);
    let z_const = b.mk_const(z, empty_levels);
    let s_const = b.mk_const(s, empty_levels);
    let sort_u = b.mk_sort(u);

    // 消去子类型：
    // (motive : (n : MyNat) -> Sort u) -> (mz : motive z) ->
    // (ms : (n : MyNat) -> motive n -> motive (s n)) -> (n : MyNat) -> motive n
    let motive_ty = b.mk_pi(n_name, BinderStyle::Default, nat_const, sort_u);
    let motive_n = b.mk_var(0);
    let mz_ty = b.mk_app(motive_n, z_const);
    // ms : (n : MyNat) -> motive n -> motive (s n)
    // （在此深度：进入 n 后 motive = Var 2；进入匿名假设后 motive = Var 3、n = Var 1）
    let var2 = b.mk_var(2);
    let var0 = b.mk_var(0);
    let motive_n_ms = b.mk_app(var2, var0);
    let var3 = b.mk_var(3);
    let var1 = b.mk_var(1);
    let succ_n1 = b.mk_app(s_const, var1);
    let motive_succ_n = b.mk_app(var3, succ_n1);
    let ms_body = b.mk_pi(anon, BinderStyle::Default, motive_n_ms, motive_succ_n);
    let ms_ty = b.mk_pi(n_name, BinderStyle::Default, nat_const, ms_body);
    // body：motive n（motive = Var 3, n = Var 0）
    let n0 = b.mk_var(0);
    let rec_body = b.mk_app(var3, n0);
    let n_binder = b.mk_pi(n_name, BinderStyle::Default, nat_const, rec_body);
    let mz_binder = b.mk_pi(ms_name, BinderStyle::Default, ms_ty, n_binder);
    let rec_mz = b.mk_pi(mz_name, BinderStyle::Default, mz_ty, mz_binder);
    let rec_ty = b.mk_pi(motive, BinderStyle::Default, motive_ty, rec_mz);

    let mz0 = b.mk_var(1);
    let z_ms = b.mk_lambda(ms_name, BinderStyle::Default, ms_ty, mz0);
    let z_mz = b.mk_lambda(mz_name, BinderStyle::Default, mz_ty, z_ms);
    let z_rule_val = b.mk_lambda(motive, BinderStyle::Default, motive_ty, z_mz);
    let rec_head = b.mk_const(rec_name, u_levels);
    let rm = b.mk_var(3);
    let rec_a1 = b.mk_app(rec_head, rm);
    let mz1 = b.mk_var(2);
    let rec_a2 = b.mk_app(rec_a1, mz1);
    let ms1 = b.mk_var(1);
    let rec_a3 = b.mk_app(rec_a2, ms1);
    let n00 = b.mk_var(0);
    let rec_app = b.mk_app(rec_a3, n00);
    // `ms n (MyNat.rec.{u} motive mz ms n)`（ms = Var 1, n = Var 0）
    let ms_n = b.mk_app(ms1, n00);
    let s_rule_body = b.mk_app(ms_n, rec_app);
    let s_n = b.mk_lambda(n_name, BinderStyle::Default, nat_const, s_rule_body);
    let s_ms = b.mk_lambda(ms_name, BinderStyle::Default, ms_ty, s_n);
    let s_mz = b.mk_lambda(mz_name, BinderStyle::Default, mz_ty, s_ms);
    let s_rule_val = b.mk_lambda(motive, BinderStyle::Default, motive_ty, s_mz);
    let z_rule = RecRule { ctor_name: z, ctor_telescope_size_wo_params: 0, val: z_rule_val };
    let s_rule = RecRule { ctor_name: s, ctor_telescope_size_wo_params: 1, val: s_rule_val };
    let rules: Vec<RecRule<'a>> = match arrangement {
        RuleArrangement::InOrder => vec![z_rule, s_rule],
        RuleArrangement::Swapped => vec![s_rule, z_rule],
        RuleArrangement::MissingSucc => vec![z_rule],
    };

    b.begin_inductive_block();
    let zero_level = b.zero();
    let one = b.succ(zero_level);
    let ind_ty = b.mk_sort(one);
    let s_ctor_ty = b.mk_pi(n_name, BinderStyle::Default, nat_const, nat_const);
    let ind = b
        .add_inductive(DeclarInfo { name: nat, uparams: empty_levels, ty: ind_ty }, true, 0, 0, std::sync::Arc::from([nat]), std::sync::Arc::from([z, s]))
        .expect("add inductive");
    b.add_declar(Declar::Constructor(ConstructorData {
        info: DeclarInfo { name: z, uparams: empty_levels, ty: nat_const },
        inductive_name: nat,
        ctor_idx: 0,
        num_params: 0,
        num_fields: 0,
    }))
    .expect("add ctor z");
    b.add_declar(Declar::Constructor(ConstructorData {
        info: DeclarInfo { name: s, uparams: empty_levels, ty: s_ctor_ty },
        inductive_name: nat,
        ctor_idx: 1,
        num_params: 0,
        num_fields: 1,
    }))
    .expect("add ctor s");
    b.add_declar(Declar::Recursor(RecursorData {
        info: DeclarInfo { name: rec_name, uparams: u_levels, ty: rec_ty },
        all_inductives: std::sync::Arc::from([nat]),
        num_params: 0,
        num_indices: 0,
        num_motives: 1,
        num_minors: 2,
        rec_rules: std::sync::Arc::from(rules),
        is_k: false,
    }))
    .expect("add rec");
    b.end_inductive_block();
    ind
}

/// 对照：按构造子声明顺序给出全部 iota 规则的块必须通过完整内核检查。
#[test]
fn accepts_iota_rules_in_constructor_order() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let ind = build_my_nat_block(&mut b, RuleArrangement::InOrder);
    let env = b.finish();
    // 与前端一致：逐条 try_check_declar（ind → ctor z → ctor s → rec）。
    for (i, d) in collect_block_declars(&env, &ind).iter().enumerate() {
        if let Err(e) = env.try_check_declar(d) {
            panic!("declar #{i} failed: {e}");
        }
    }
}

/// 收集归纳块里的全部声明（与前端 `built` 顺序一致）。
fn collect_block_declars<'a>(env: &ExportFile<'a>, ind: &Declar<'a>) -> Vec<Declar<'a>> {
    let name = ind.info().name;
    let (start, size) = *env.mutual_block_sizes.get(&name).expect("block boundaries");
    (start..start + size)
        .map(|idx| env.declars.get_index(idx).expect("declar in block").1.clone())
        .collect()
}

#[test]
fn rejects_iota_rules_out_of_constructor_order() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let ind = build_my_nat_block(&mut b, RuleArrangement::Swapped);
    let env = b.finish();
    let all = collect_block_declars(&env, &ind);
    let err = env.try_check_declar(&all[0]).expect_err("swapped iota rules must be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("iota rule is not listed in constructor declaration order"),
        "unexpected message: {msg}"
    );
}

#[test]
fn rejects_iota_rule_count_short_of_constructors() {
    let arena = Arena::new();
    let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    let ind = build_my_nat_block(&mut b, RuleArrangement::MissingSucc);
    let env = b.finish();
    let all = collect_block_declars(&env, &ind);
    let err = env
        .try_check_declar(&all[0])
        .expect_err("an incomplete iota rule set must be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("iota rule count does not match the constructor count"),
        "unexpected message: {msg}"
    );
}
