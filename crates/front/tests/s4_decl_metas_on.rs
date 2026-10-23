//! **S4 / B2-a（2026-10-23）**：`SOKO_DECL_METAS=1` 的判据 —— 三档都剥不出 binder
//! 类型时造元变量，**声明出口**解出后源级重写 + 重 elaborate
//! （`plans/apply-unification-plan.md` §21；施工图 `/tmp/s4-patch-plan.md` §4）。
//!
//! ⚠ **一个文件一个 `#[test]`**（`scripts/check-test-env-isolation.py`：同文件里
//! `set_var` + 两个 `#[test]` = 红）。`SOKO_DECL_METAS` 走 `OnceLock` **只读一次环境**，
//! 同进程里两个档位会抢同一个 `OnceLock` ⇒ 一个档位一个文件（各自独立进程 ✓）。
//!
//! 夹具（施工图 §4.0，**都是良类型、今天都红**）：
//! * F+ A `def f : (let T := Nat -> Nat; T) := fun x => x`（期望类型是 `let` 绑定）；
//! * F+ B `T0..T10` 别名链 + `def f : T10 := fun x => x`（`b2_whnf_peel` 的 8 步预算不够）。
//!
//! 三条负例（**必须仍然红、同码同句同 span**）：`fun n => fun m => n`、`(fun x y => x) 1`、
//! `#check fun x => x`。

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

fn checked(src: &str) -> (Vec<(String, String)>, bool) {
    let file = parse(src).expect("夹具必须能解析");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());
    let errors = out
        .errors
        .iter()
        .map(|e| (e.code().to_string(), e.message.clone()))
        .collect::<Vec<_>>();
    let checked = report
        .decls
        .iter()
        .any(|d| d.name.as_deref() == Some("f") && d.status == DeclStatus::Checked);
    (errors, checked)
}

/// 开关 ON：两条正例判绿（`decl.checked`）；三条负例仍红且**码/句/span 逐字节不变**。
#[test]
fn the_flag_solves_binder_metas_and_keeps_the_negatives_red() {
    std::env::set_var("SOKO_DECL_METAS", "1");

    // ① F+ A：`let` 绑定的期望类型 ⇒ 元变量解出 `Nat` ⇒ 声明 checked。
    let (errors, is_checked) = checked(F_PLUS_A);
    assert_eq!(errors, vec![], "F+ A 在开关 ON 时必须是绿的：{errors:?}");
    assert!(is_checked, "F+ A 的 `f` 必须 `decl.checked`");

    // ② F+ B：10 层别名链（`b2_whnf_peel` 的 8 步预算不够）⇒ 同样判绿。
    let (errors, is_checked) = checked(F_PLUS_B);
    assert_eq!(errors, vec![], "F+ B 在开关 ON 时必须是绿的：{errors:?}");
    assert!(is_checked, "F+ B 的 `f` 必须 `decl.checked`");

    // ③ 负例：码 / 句 / span 三件都必须与今天**逐字节相同**（施工单 §4.0）。
    for (src, line, column) in [(F_MINUS_1, 1, 36), (F_MINUS_2, 1, 21), (F_MINUS_3, 1, 12)] {
        let file = parse(src).expect("负例必须能解析");
        let (out, _) = compile_all_with(&file, &CompileOptions::default());
        assert_eq!(out.errors.len(), 1, "负例必须恰好一条诊断：{src:?}");
        let e = &out.errors[0];
        assert_eq!(e.code(), "elab-untyped-binder", "负例的码变了：{src:?}");
        assert_eq!(e.kind, ErrorKind::ElabUntypedBinder, "负例的 kind 变了");
        assert_eq!(e.message, MSG, "负例的句子变了：{src:?}");
        assert_eq!(
            (e.span.start.line, e.span.start.column),
            (line, column),
            "负例的 span 变了：{src:?}"
        );
    }
}
