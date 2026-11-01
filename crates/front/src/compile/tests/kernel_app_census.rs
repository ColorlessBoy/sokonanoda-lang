//! **P2-12 S0/S0b 判据**：内核回落后老路结局计数（`kernel_app_census`）。线程局部读数；闸门强制开、strict。

use super::kernel_app::TEST_OVERRIDE;
use super::kernel_app_census::{
    arm, pending_notation, TEST_FORCE_ERR, TEST_SETTLED, TEST_SETTLED_ROUTED,
};
use super::kernel_app_lambda::TEST_NO_TYPED_LAMBDA;
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;
use crate::Span;

/// `(errors, [(结局, 原因)])`；`typed = false` ⇒ 关掉 P2-10b（让带注解 λ 实参回落）。
fn run(src: &str, on: bool, typed: bool) -> (Vec<String>, Vec<(&'static str, &'static str)>) {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(on)));
        // **P2-12 S6 翻默认**：strict 档默认不回落 ⇒ 本文件量的是**回落路径**的普查 ⇒ 显式回到之前的档。
        super::kernel_app_fallback::TEST_NO_FALLBACK.with(|c| c.set(Some(false)));
        TEST_NO_TYPED_LAMBDA.with(|c| c.set(!typed));
        TEST_SETTLED.with(|v| v.borrow_mut().clear());
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (errors, TEST_SETTLED.with(|v| v.borrow().clone()))
    })
}

fn n(v: &[(&str, &str)], tag: &str, why: Option<&str>) -> usize {
    v.iter()
        .filter(|(t, w)| *t == tag && why.is_none_or(|x| x == *w))
        .count()
}

const C1: &str = "axiom g : {A : Type} -> Nat -> Prop\ndef t : Prop := g 0\n";

/// `Eq.subst h hp`：高阶 motive 头挡在门外（`unsupported`）⇒ 回落、老路接下（接受）。
const RESCUED: &str =
    "theorem s2 (P : Nat -> Prop) (a b : Nat) (h : a = b) (hp : P a) : P b := Eq.subst h hp\n";

#[test]
fn c1_unsolved_fallback_is_counted_as_old_err() {
    let (errors, v) = run(C1, true, true);
    assert!(!errors.is_empty(), "c1 本来就拒绝");
    assert!(
        n(&v, "err", Some("unsolved")) >= 1,
        "c1 ⇒ old_err_unsolved ≥ 1：{v:?}"
    );
    // **G3（2026-11-01）重钉计数**（**不是**放宽断言 ✗）：修前 `old_ok = 0` —— c1 的老路在
    // 载体的 `` unknown identifier `\u{0}soko_m*` `` 上早退（`pp_shape::elab_engine_value`
    // 摘掉了元变量表 ⇒ `decl_meta_carrier` 够不着载体）。修后载体翻成 `Expr::Meta`
    // ⇒ 内层应用**走得到末尾** ⇒ 普查里必须有 `ok`。**判卷一个字没变**（上面
    // `!errors.is_empty()` + `err ≥ 1` 仍然成立）⇒ 这一条钉的是"普查**如实记账**"，
    // 不再是"老路必死"。**旧读数 `0` 已随 G3 作废，这里是新的精确读数**（不是 `≥`：钉死个数，
    // 多了少了都要重新解释）。
    assert_eq!(
        n(&v, "ok", None),
        2,
        "G3 后 c1 的老路内层会走到末尾 ⇒ 恰 2 条 ok 结算：{v:?}"
    );
    // **G3 的正面判据**（方向翻转的那一条）：c1 的诊断里**不许**再出现内核内部名。
    // 修前这里是 `unknown identifier `\u{0}soko_m*``（内部名 + span 0:0）⇒ 这条当场红。
    for e in &errors {
        for bad in ["soko_m", "soko_p", "\u{0}", "\\0"] {
            assert!(
                !e.contains(bad),
                "G3：c1 的诊断里不许出现内核名 {bad}（修前的形状就是它）：{e}"
            );
        }
    }
    // 闸门关 ⇒ 一条都不记、判卷相同。
    let (errors_off, v_off) = run(C1, false, true);
    assert_eq!(errors, errors_off);
    assert!(v_off.is_empty(), "{v_off:?}");
}

#[test]
fn rescued_fallback_is_counted_as_old_ok() {
    let (errors, v) = run(RESCUED, true, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(n(&v, "ok", None) >= 1, "老路接下 ⇒ old_ok ≥ 1：{v:?}");
}

fn routed() -> Vec<(&'static str, String)> {
    TEST_SETTLED_ROUTED.with(|v| v.borrow().clone())
}

/// **S0b 判据（机制层）**：记法路由的守卫 `Drop` 默认记 `err`、[`super::kernel_app_census::Settle::ok`]
/// 兑现成 `ok`、没压过原因（`arm(false)`）时什么都不记 —— 三条都要咬。
#[test]
fn notation_arm_records_ok_and_err() {
    TEST_SETTLED_ROUTED.with(|v| v.borrow_mut().clear());
    let span = Span::default();
    // 老路走到末尾 ⇒ ok
    pending_notation("unsolved", "Set.Le", span);
    arm(true).ok();
    // 老路早退 / `?` / 显式 Err ⇒ err
    pending_notation("unsolved", "Set.Le", span);
    drop(arm(true));
    // 没压过原因 ⇒ 不结算（不能把外层那一帧记成自己的）
    drop(arm(false));
    assert_eq!(
        routed(),
        vec![
            ("ok", "notation_unsolved".to_string()),
            ("err", "notation_unsolved".to_string()),
        ],
        "记法路由只按自己的出口结算"
    );
}

/// **反向验证**：把结算强制成 `err` ⇒ 上面那条 `ok` 判据必须判红（守卫咬得住）。
#[test]
fn notation_arm_ok_is_reverse_verified() {
    TEST_SETTLED_ROUTED.with(|v| v.borrow_mut().clear());
    TEST_FORCE_ERR.with(|c| c.set(true));
    pending_notation("unsolved", "Set.Le", Span::default());
    arm(true).ok();
    TEST_FORCE_ERR.with(|c| c.set(false));
    assert_eq!(
        routed(),
        vec![("err", "notation_unsolved".to_string())],
        "强制 err ⇒ ok 不再成立"
    );
}

/// **S0b 判据（对账）**：真文件上 `fallback_unsolved` 必须等于 `probe_cut` + 各 `fallback_old_*_unsolved`
/// —— S0b 之前 unit19-solution 差 68、unit19-cardinal-le 差 31（记法路由没结算）。见 CLI 集成判据
/// `crates/cli/tests/strict_fallback_census.rs`（单元层拿不到课程库，`Set.Le` 记法不在 prelude）。
#[test]
fn unit_layer_cannot_cover_notation_route_without_course_lib() {
    // 反面对照：prelude 的记法（`≠` 等）目标都带前导隐式或没有前导参数 ⇒ 走不到记法路由的 `missing > 0` 分支。
    // 记法路由的端到端判据只能落在 CLI 集成测试 ⇒ 这条只钉"为什么这里没有它"，防止有人把 CLI 那条删了。
    let src = "theorem s4 (a b : Nat) (h : a = b) : a = b := h\n";
    let (errors, v) = run(src, true, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        v.iter().all(|(_, w)| !w.starts_with("notation_")),
        "prelude 记法不该走 `pending_notation`：{v:?}"
    );
}
