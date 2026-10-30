//! **P2-6 判据**：探测重做只认**本帧名下**的切（`kernel_app_probe::with_redo`）。
//!
//! 形状 = unit12 `flawed_equalities_refuted` 的操作数 `(fun (_ : Set Nat) => ∅) '' ({∅} ∩ {(@Set.univ Nat)})`：
//! `∩` 第一趟探测切掉 `{∅}`、随后按域解出（`∩` 自己 `Solved`）；`''` 第一位是 λ（体 `∅` 无期望 ⇒ 推迟，不是切）、
//! 无期望时 `Unsolved ["β"]`（域还含元变量那一类，本刀不修）。P2-5 把 `∩` 名下的切算进 `''` ⇒ `''` 白重做一遍
//! （重做后照样 `Unsolved`）。P2-6 ⇒ `''` 不重做。

use super::kernel_app::TEST_OVERRIDE;
use super::kernel_app_probe::{TEST_REDOS, TEST_TRANSITIVE_CUTS};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

const PRELUDE: &str = "def Set (α : Type) : Type := α -> Prop\n\
     def Set.empty {α : Type} : Set α := fun (x : α) => False\n\
     notation \"∅\" => Set.empty\n\
     def Set.univ {α : Type} : Set α := fun (x : α) => True\n\
     def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a\n\
     def Set.inter {α : Type} (A B : Set α) : Set α := fun (x : α) => And (A x) (B x)\n\
     infixl:70 \" ∩ \" => Set.inter\n\
     def Set.image {α β : Type} (f : α -> β) (A : Set α) : Set β := fun (y : β) => True\n\
     infixr:80 \" '' \" => Set.image\n";

/// `≠` 的右操作数定下 `α = Set (Set Nat)` ⇒ 外层解得出。第一趟里 `''` 是被 `≠` 探测的节点：它内部 `∩` 切掉 `{∅}`
/// 又自己解出；`''` 本身 `Unsolved` ⇒ 被 `≠` 切掉、推迟、按域解出 ⇒ `≠` `Solved`。
/// P2-5：`''` 把 `∩` 名下的切算成自己的 ⇒ 被切之前先白重做一遍。P2-6：0 次重做。
const NESTED_SRC: &str = "theorem p26 : (fun (_ : Set Nat) => ∅) '' ({∅} ∩ {(@Set.univ Nat)}) ≠ \
     @Set.univ (Set Nat) := sorry\n";

/// c2 本体（P2-5 的回归形）：`≠` 第一趟**自己**切掉了 `''` ⇒ 它的重做必须保留（它让判卷与 base 相同）。
const C2_SRC: &str = "theorem c2p26 : (fun (_ : Set Nat) => ∅) '' ({∅} ∩ {(@Set.univ Nat)}) ≠ \
     ((fun (_ : Set Nat) => ∅) '' {∅}) ∩ ((fun (_ : Set Nat) => ∅) '' {(@Set.univ Nat)}) := sorry\n";

/// `(errors, 重做次数)`：闸门强制开、strict；`transitive = true` ⇒ P2-5 的"子树里切过就重做"。
fn run(body: &str, transitive: bool) -> (Vec<String>, u64) {
    let src = format!("{PRELUDE}{body}");
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_TRANSITIVE_CUTS.with(|c| c.set(transitive));
        TEST_REDOS.with(|c| c.set(0));
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let errors = out.errors.iter().map(|e| format!("{e:?}")).collect();
        (errors, TEST_REDOS.with(|c| c.get()))
    })
}

#[test]
fn cut_resolved_by_inner_frame_does_not_redo_outer() {
    let (errors, redos) = run(NESTED_SRC, false);
    assert_eq!(errors, Vec::<String>::new());
    assert_eq!(
        redos, 0,
        "`∩` 名下的切已被 `∩` 消化 ⇒ `''` 不重做（`≠` 自己解出 ⇒ 也不重做）"
    );
}

#[test]
fn transitive_cuts_redo_outer_reverse_check() {
    // 反向验证：切并给外层（P2-5）⇒ `''` 重做；判卷不变。
    let (errors, redos) = run(NESTED_SRC, true);
    assert_eq!(errors, Vec::<String>::new());
    assert!(
        redos > 0,
        "P2-5 口径必须重做（否则本组测试咬不住）：redos={redos}"
    );
}

#[test]
fn own_cut_still_redoes_c2() {
    // `≠` 自己切掉的 `''` 让它解不出 ⇒ 重做保留、c2 照旧接受；且比 P2-5 口径少做。
    let (errors, redos) = run(C2_SRC, false);
    assert_eq!(errors, Vec::<String>::new(), "c2 必须照旧接受");
    assert!(redos > 0, "`≠` 名下的切 ⇒ 必须重做");
    let (errors_t, redos_t) = run(C2_SRC, true);
    assert_eq!(errors_t, Vec::<String>::new());
    assert!(redos < redos_t, "本帧口径只减不增：{redos} vs {redos_t}");
}
