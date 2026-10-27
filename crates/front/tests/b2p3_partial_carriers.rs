//! **B2-P4b**：声明出口（P4a 的收口 + 重建）**加上**应用结点的「结果类型 ≟ 期望类型」
//! 方程之后，P0 形状（`b2_shared_metavar_context.rs`）在闸门开（`SOKO_DECL_MVARS=1`）下
//! **与对照组同判**：0 诊断 + `t` `decl.checked`。
//!
//! ## 形状的变化（本文件钉的就是这条）
//!
//! | 档位 | P3b 之后 | P4a 之后 | **P4b 之后（本文件）** |
//! |---|---|---|---|
//! | 闸门开 | `kernel-rejected`（载体进项 ⇒ walk 静默跳过 ⇒ 假 `unknown const`） | `elab-unknown-identifier`（= 闸门关 ✓） | **0 诊断 + `t` checked** ✓ |
//!
//! ## 为什么 P4a 只做到"等于闸门关"、P4b 才全绿
//!
//! P0 形状缺一条方程：内层应用的结果类型 `Set ?β` 与外层的期望类型 `Set ?α` 之间
//! **没有约束进 store** —— 贪心求解器先跑（它的解只活在那棵源级项里）⇒ 引擎的路线 ④
//! 一步都不进 ⇒ 收口时两个元变量都悬着，`default_unresolved`（E19，**选择**不是推理）
//! 只能借同形兄弟。**P4b 补两条**：① 贪心出口也把 `结果 ≟ 期望` 记进声明级 store
//! （`p4b::app_result_equation`）② 组装顺序改成「先实参、后隐式实参 + 装前 zonk」
//! （`p4b::assemble_app`）⇒ 载体**根本不进项** ⇒ 第一趟就是干净的声明。
//!
//! **跑法**：`cargo test -p sokonanoda-front --test b2p3_partial_carriers`
//! （档位由环境变量决定 ⇒ 本文件**恰好一个** `#[test]`，见
//! `scripts/check-test-env-isolation.py`）。

use std::env;

use sokonanoda_front::compile::{compile_all_with, CompileOptions, DeclStatus};
use sokonanoda_front::parse;

/// P0 形状（与 `b2_shared_metavar_context.rs` 的 `ELIDED` **同一份**源）。
const P0: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.image {α β : Type} (f : α → β) (A : Set α) : Set β := fun (_ : β) => True
def Set.Equiv {α β : Type} (A : Set α) (B : Set β) : Prop := True

def t (A : Set Nat) (B : Set (Set Nat)) : Prop := Set.Equiv (Set.image Set.singleton A) B
";

/// P4b：闸门开 ⇒ 0 诊断 + `t` checked（且**绝不**再出现 `kernel-rejected`）。
#[test]
fn p0_shape_after_the_declaration_exit_matches_gate_off() {
    // ⚠ 本文件**恰好一个** `#[test]`（独立进程 ⇒ `OnceLock` 不竞态）。
    env::set_var("SOKO_DECL_MVARS", "1");
    let file = parse(P0).expect("夹具必须能解析");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());
    let errors: Vec<(&str, &str)> = out
        .errors
        .iter()
        .map(|e| (e.code(), e.message.as_str()))
        .collect();

    // ① **P3b 的回归必须消失**：载体**绝不**再走到内核（那条会变成后面全 `unknown const`）。
    assert!(
        !errors.iter().any(|(code, _)| *code == "kernel-rejected"),
        "P4a/P4b 之后不许再有 kernel-rejected（载体必须死在声明出口）：{errors:?}"
    );
    // ② **全绿**：那条方程（内层结果 ≟ 外层期望）把 `?α` 定下来 ⇒ 0 诊断。
    assert_eq!(
        errors,
        vec![],
        "P4b 之后 P0 形状必须 0 诊断（内层 `Set.image … : Set ?β` ≟ 外层期望 `Set ?α`）：{errors:?}"
    );
    // ③ **真判定**（不是文本比对）：`t` 必须 `decl.checked`。
    assert!(
        report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("t") && d.status == DeclStatus::Checked),
        "`t` 必须 `decl.checked`：{:?}",
        report
            .decls
            .iter()
            .map(|d| (d.name.clone(), d.status))
            .collect::<Vec<_>>()
    );
}
