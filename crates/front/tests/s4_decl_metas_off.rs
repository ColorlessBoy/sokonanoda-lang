//! **S4 / B2-a（2026-10-23）**：`SOKO_DECL_METAS` **默认关**的判据 —— 开关一个字都不许
//! 改变今天的行为（`plans/apply-unification-plan.md` §21）。
//!
//! ⚠ **一个文件一个 `#[test]`**（`scripts/check-test-env-isolation.py`）。本文件**不设**
//! 环境变量 ⇒ 走默认档；反向验证（`SOKO_DECL_METAS=1` 下本文件必须**红**）由施工单单独跑，
//! 不写进文件 ✓（与 `soko_defer_off.rs` 同款纪律）。
//!
//! 夹具同 `s4_decl_metas_on.rs`：两条**良类型**正例（今天红）+ 三条负例。
//! 默认档的判据：正例**仍然红**（码/句/span 与今天逐字节相同）⇒ 判定面一个字节都没动。

use sokonanoda_front::compile::{compile_all_with, CompileOptions, DeclStatus, ErrorKind};
use sokonanoda_front::parse;

/// 今天那条诊断的**句子**（逐字；`docs/protocol.md` 与既有测试都钉它）。
const MSG: &str = "cannot infer the type of this binder: the declared type does not provide a matching position (write it explicitly, e.g. fun (x : Nat) => x)";

const F_PLUS_A: &str = "def f : (let T := Nat -> Nat; T) := fun x => x\n";

const F_PLUS_B: &str = "\
def T0 : Type := Nat -> Nat
def T1 : Type := T0
def T2 : Type := T1
def T3 : Type := T2
def T4 : Type := T3
def T5 : Type := T4
def T6 : Type := T5
def T7 : Type := T6
def T8 : Type := T7
def T9 : Type := T8
def T10 : Type := T9
def f : T10 := fun x => x
";

const F_MINUS_1: &str = "def f : Nat -> Nat := fun n => fun m => n\n";
const F_MINUS_2: &str = "def f : Nat := (fun x y => x) 1\n";
const F_MINUS_3: &str = "#check fun x => x\n";

/// 默认档：每条夹具**恰好一条** `elab-untyped-binder`，位置就是今天那个 binder。
#[test]
fn without_the_flag_binder_metas_stay_rejected() {
    for (src, line, column) in [
        (F_PLUS_A, 1, 41),
        (F_PLUS_B, 12, 20),
        (F_MINUS_1, 1, 36),
        (F_MINUS_2, 1, 21),
        (F_MINUS_3, 1, 12),
    ] {
        let file = parse(src).expect("夹具必须能解析");
        let (out, report) = compile_all_with(&file, &CompileOptions::default());
        assert_eq!(out.errors.len(), 1, "默认档必须恰好一条诊断：{src:?}");
        let e = &out.errors[0];
        assert_eq!(e.code(), "elab-untyped-binder", "默认档的码变了：{src:?}");
        assert_eq!(e.kind, ErrorKind::ElabUntypedBinder, "默认档的 kind 变了");
        assert_eq!(e.message, MSG, "默认档的句子变了：{src:?}");
        assert_eq!(
            (e.span.start.line, e.span.start.column),
            (line, column),
            "默认档的 span 变了：{src:?}"
        );
        // 有 `f` 的夹具：声明必须是 `Failed`（不是 checked、不是 open）。
        if src != F_MINUS_3 {
            let d = report
                .decls
                .iter()
                .find(|d| d.name.as_deref() == Some("f"))
                .expect("声明 f");
            assert_eq!(
                d.status,
                DeclStatus::Failed,
                "默认档声明必须 Failed：{src:?}"
            );
        }
    }
}
