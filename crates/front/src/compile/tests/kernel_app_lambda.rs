//! **P2-10b 判据**：带注解 binder 的 λ 实参、体是内核路径的应用 ⇒ 推迟、体对着域的 codomain elaborate
//! （`lib/Exists.sokonanoda:110` `Exists.imp` 的形状）。反向：`TEST_NO_TYPED_LAMBDA` 关掉 ⇒ 体回落老路。

use super::kernel_app::{TEST_FALLBACK, TEST_OVERRIDE, TEST_SOLVED};
use super::kernel_app_lambda::{TEST_BARE, TEST_NO_TYPED_LAMBDA};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

/// `(errors, solved, fallback)`：闸门强制开、strict；`typed = false` ⇒ P2-10b 之前的行为。
pub(super) fn run_typed_lambda(src: &str, typed: bool) -> (Vec<String>, u64, u64) {
    let (errors, solved, fallback, _) = run_counted(src, typed);
    (errors, solved, fallback)
}

/// 同 [`run_typed_lambda`]，另返回本线程 `elab_bare` 次数（推迟的带注解 λ 退回老办法的次数）。
fn run_counted(src: &str, typed: bool) -> (Vec<String>, u64, u64, u64) {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_BARE.with(|c| c.set(0));
        TEST_NO_TYPED_LAMBDA.with(|c| c.set(!typed));
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (
            errors,
            TEST_SOLVED.with(|c| c.get()),
            TEST_FALLBACK.with(|c| c.get()),
            TEST_BARE.with(|c| c.get()),
        )
    })
}

/// 闸门关的判卷（基准）。
fn run_off(src: &str) -> Vec<String> {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(false)));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        out.errors.iter().map(|e| format!("{e:?}")).collect()
    })
}

/// `Exists.imp` 的最小形：`Ex.elim h (fun (w : A) => fun (hw : p w) => Ex.intro w (f w hw))`。
/// 内层 `Ex.intro` 无期望时 `q` 只能从 `f w hw : q w` 反推（非模式）⇒ 解不出；对着 `Ex.elim` 域的 codomain
/// `Ex A q`（`b` 已由外层期望定）就解出。
const EXISTS_IMP_SRC: &str = "\
inductive Ex (A : Type) (q : A -> Prop) : Prop
  ctor intro (w : A) (hw : q w) : Ex A q
end
axiom Ex.elim : {A : Type} -> {q : A -> Prop} -> {b : Prop} -> Ex A q -> ((a : A) -> q a -> b) -> b
theorem Ex.imp (A : Type) (p q : A -> Prop) (h : Ex A p) (f : (w : A) -> p w -> q w) : Ex A q :=
  Ex.elim h (fun (w : A) => fun (hw : p w) => Ex.intro w (f w hw))
";

#[test]
fn typed_lambda_body_solved_against_domain_codomain() {
    let errors_off = run_off(EXISTS_IMP_SRC);
    assert_eq!(errors_off, Vec::<String>::new());
    let (errors, solved, fallback) = run_typed_lambda(EXISTS_IMP_SRC, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同");
    assert_eq!(fallback, 0, "λ 体对着域的 codomain 解出（solved={solved}）");
    assert!(
        solved >= 2,
        "外层 Ex.elim 与体里的 Ex.intro 都由内核解出（solved={solved}）"
    );
}

#[test]
fn typed_lambda_body_falls_back_without_p210b() {
    // 反向验证：关掉 P2-10b ⇒ 带注解 λ 第一趟无期望 elaborate ⇒ 体里的 `Ex.intro` 回落老路（判卷不变）。
    let (errors, _, fallback) = run_typed_lambda(EXISTS_IMP_SRC, false);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        fallback > 0,
        "关掉 P2-10b ⇒ 必须回落（fallback={fallback}）"
    );
}

/// 域一直卡在元变量上（`b` 无外层期望、只能从 λ 体来）⇒ 推迟位补回老办法（`fill_late`）⇒ 判卷不变、不死循环。
const BLOCKED_SRC: &str = "\
inductive Ex (A : Type) (q : A -> Prop) : Prop
  ctor intro (w : A) (hw : q w) : Ex A q
end
axiom Ex.elim : {A : Type} -> {q : A -> Prop} -> {b : Prop} -> Ex A q -> ((a : A) -> q a -> b) -> b
axiom P : Prop
axiom hp : P
axiom K : {X : Prop} -> X -> P
theorem blk (A : Type) (p : A -> Prop) (h : Ex A p) : P :=
  K (Ex.elim h (fun (w : A) => fun (hw : p w) => K hp))
";

#[test]
fn blocked_typed_lambda_is_filled_late_and_judgement_unchanged() {
    let errors_off = run_off(BLOCKED_SRC);
    assert_eq!(errors_off, Vec::<String>::new());
    // 瞄准 P2-10b 自己的退回路径（`fill_late` / `elab_ready` 的 Err 臂 ⇒ `elab_bare`）：作用域深度若漏了恢复
    // ⇒ 老路在多出的 binder 下 elaborate ⇒ `loose bvar`。必须真走过退回（`bare > 0`）且零诊断。
    let (errors, _, _, bare) = run_counted(BLOCKED_SRC, true);
    assert_eq!(errors, errors_off, "判卷与闸门关逐字相同（无 loose bvar）");
    assert!(bare > 0, "域卡在元变量上 ⇒ 必须退回老办法（bare={bare}）");
    let (errors_np, _, _, bare_np) = run_counted(BLOCKED_SRC, false);
    assert_eq!(errors_np, errors_off);
    assert_eq!(bare_np, 0, "关掉 P2-10b ⇒ 退回路径一行不跑");
}

/// `kernel_app.rs` 测试里的守卫 `failed_arg_elaboration_does_not_leak_binders_into_scope` 的源：P2-10b 开时
/// 这条带注解 λ 推迟、对着域解出 ⇒ 零回落（守卫自己关掉 P2-10b 跑，仍测回落路径的作用域纪律）。
#[test]
fn leak_guard_source_is_solved_with_p210b() {
    let src = "\
inductive Ex (A : Type) (q : A -> Prop) : Prop
  ctor intro (w : A) (hw : q w) : Ex A q
end
axiom Ex.elim2 : {A : Type} -> {q : A -> Prop} -> {b : Prop} -> Ex A q -> ((a : A) -> q a -> b) -> b
theorem r1 {B : Type} (r : B -> Prop) (s : B -> Prop) (h : Ex B (fun (x : B) => And (r x) (s x))) : Ex B (fun (x : B) => And (s x) (r x)) :=
  Ex.elim2 h (fun (b : B) (hb : And (r b) (s b)) => Ex.intro b (And.intro (And.right hb) (And.left hb)))
";
    let (errors, solved, fallback) = run_typed_lambda(src, true);
    assert_eq!(errors, Vec::<String>::new());
    assert_eq!(fallback, 0, "P2-10b 开 ⇒ 不回落（solved={solved}）");
}
