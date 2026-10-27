//! **B2-P3a**：**B3 守卫**（`SOKO_DECL_MVARS=1`）—— pp 回读的类型里**作用域外的名字**
//! 不许被反解进前导隐式实参。
//!
//! ## 夹具（B3 的最小复现）
//!
//! ```text
//! def u (A : Set Nat) : Set (Set Nat) := Set.image Set.singleton A
//! ```
//!
//! `Set.image {α β} (f : α → β) (A : Set α) : Set β` 的**第一个实参**是
//! `Set.singleton`（一个常量，没有书写类型可查）⇒ `operand_type_expr` 走内核 pp 那条路，
//! 回读到的是**签名原文** `{α : Type} → α → Set α` —— 其中 `α` 是**常量自己的 binder 名**，
//! 回读后是**自由名** ⇒ 求解器把它当"实参类型"反解出 `α := α`，再 elaborate 就报
//! `elab-unknown-identifier` / ``unknown identifier `α` ``（span 还落在**第 1 行**：
//! 裸名是从签名原文漏出来的，见 `b2_shared_metavar_context.rs` 的同类描述）。
//!
//! ## P3a 之后
//!
//! 守卫在 `operand_type_expr` 的 pp 出口上**只否决不发明**：类型里每个 `Ident` 都必须在
//! `scope` 里 / 是 `\0` fresh 名 / 在本层声明表里，否则当作"没有类型信息"（`None`）回退。
//! 于是这一位改由**期望类型**（`Set.image` 的结果 `Set β` ≟ `Set (Set Nat)`）解出，
//! `Set.singleton` 的 `Nat` 再由函数位 `f : Nat → Set Nat` 定出来 ⇒ **本条夹具 now 判绿**
//! （`decl.checked`、0 诊断；内核是唯一判定者）。
//!
//! ## B2-P4a（2026-10-26）之后：这一条**仍是绿的**，而且**不再**靠"载体活到内核"
//!
//! P4a 在声明出口把声明级 store **收口**（`unify_all` + `default_unresolved` + `commit`）
//! 并**重建**声明；载体一条都不许出门（K1 硬不变式），收不掉就**重放闸门关**。本夹具的
//! `u` 在这里**解得出来**（`decl.checked`、0 诊断）—— 闸门关同一份源仍报裸 `α`
//! （B3 守卫是**闸门内**的改进，`operand_type_expr` 的那条 `pp_type_scope_safe` ✓）。
//!
//! ⚠ **不是** P0 收口：P0（`b2_shared_metavar_context.rs`）缺一条约束（期望类型 ≟ 头结果
//! 类型）⇒ 那条仍是闸门关的诊断（= `b2p3_partial_carriers.rs` 钉的"等于闸门关"）。
//!
//! **跑法**：`cargo test -p sokonanoda-front --test b2p3_scope_safe_carrier`

use std::env;

use sokonanoda_front::compile::{compile_all_with, CompileOptions, DeclStatus, DocumentReport};
use sokonanoda_front::parse;

/// B3 的最小复现（自足：`Set`/`Set.singleton`/`Set.image` 的签名与课程库同形）。
const B3: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
def u (A : Set Nat) : Set (Set Nat) := Set.image Set.singleton A
";

fn compile(src: &str) -> (Vec<(String, String)>, DocumentReport) {
    let file = parse(src).expect("夹具必须能解析");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());
    let errors = out
        .errors
        .iter()
        .map(|e| (e.code().to_string(), e.message.clone()))
        .collect();
    (errors, report)
}

fn is_checked(report: &DocumentReport, name: &str) -> bool {
    report
        .decls
        .iter()
        .any(|d| d.name.as_deref() == Some(name) && d.status == DeclStatus::Checked)
}

/// 闸门开（`SOKO_DECL_MVARS=1`）：B3 的**旧症状**（`α` 的 unknown-identifier）必须消失，
/// 且这一条现在真的解得出来（`u` checked、0 诊断 —— 内核判定）。
#[test]
fn b3_const_binder_alpha_no_longer_leaks_into_the_term() {
    // ⚠ 本文件**恰好一个** `#[test]`（独立进程 ⇒ `OnceLock` 不竞态；
    // `scripts/check-test-env-isolation.py` 的判据）。
    env::set_var("SOKO_DECL_MVARS", "1");
    let (errors, report) = compile(B3);

    // ① **旧症状必须消失**：不许再出现"裸 `α`"这条 unknown-identifier（B3 的红）。
    assert!(
        !errors
            .iter()
            .any(|(code, msg)| code == "elab-unknown-identifier" && msg.contains('α')),
        "B3 守卫之后不许再报常量自己的 binder `α`：{errors:?}"
    );

    // ② **具体结果**（P3a 只做作用域 + 守卫，不是 P3b/P4）：这一条现在解得出来。
    //    `Set.image` 的 `β` 来自期望类型 `Set (Set Nat)`、`α` 来自 `A : Set Nat`、
    //    `Set.singleton` 的 `Nat` 来自函数位 `f : Nat → Set Nat` ✓。
    assert_eq!(
        errors,
        vec![],
        "P3a 之后这条必须无诊断（若红了，说明守卫的回退让期望类型路线也断了）"
    );
    assert!(
        is_checked(&report, "u"),
        "P3a 之后 `u` 必须 `decl.checked`（内核判定；不是靠文本比对）"
    );
}
