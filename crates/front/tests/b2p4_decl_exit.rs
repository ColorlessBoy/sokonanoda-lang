//! **B2-P4a**：**两条闸门同时开**（`SOKO_DECL_METAS=1` + `SOKO_DECL_MVARS=1`）时，声明出口的
//! 顺序仍然是「**先收口**（store）→ **再** S4 的 `finish`（binder 元变量）」—— 后者在**收口
//! 之后/之外**跑，它的重 elaborate 因此看得见已收口的 store ✓。
//!
//! 判据（三条，都绑"用户可见结果"）：
//! 1. **S4 的正例仍绿**：`def f : (let T := Nat -> Nat; T) := fun x => x` ⇒ 0 诊断 +
//!    `f` `decl.checked`（binder 类型元变量被解出、源级重写、重 elaborate 成功）；
//! 2. **S4 的负例同码同句**：`def f : Nat -> Nat := fun n => fun m => n` ⇒
//!    `elab-untyped-binder` + 那条**逐字**句子（出口没有把 S4 的错改形）；
//! 3. **与"只开 S4"的档位逐字相同**（store 的接入不许改 S4 的判定面）。
//!
//! ⚠ 一个文件一个 `#[test]`（`scripts/check-test-env-isolation.py`：两个闸门都走 `OnceLock`，
//! 同文件两个测试会抢同一个档位）。

use std::env;

use sokonanoda_front::compile::{compile_all_with, CompileOptions, DeclStatus};
use sokonanoda_front::parse;

/// 今天那条诊断的**句子**（逐字；`docs/protocol.md` 与 `s4_decl_metas_on.rs` 同一条常量）。
const MSG: &str = "cannot infer the type of this binder: the declared type does not provide a matching position (write it explicitly, e.g. fun (x : Nat) => x)";

/// S4 的正例（期望类型是 `let` 绑定 ⇒ `peel_expected` 三档都剥不出 ⇒ 造 binder 元变量）。
const PLUS: &str = "def f : (let T := Nat -> Nat; T) := fun x => x\n";

/// S4 的负例（`y` 的类型真的推不出 ⇒ 必须报**今天那条**错）。
const MINUS: &str = "def f : Nat -> Nat := fun n => fun m => n\n";

#[allow(clippy::type_complexity)]
fn compile(src: &str) -> (Vec<(String, String)>, Vec<(Option<String>, DeclStatus)>) {
    let file = parse(src).expect("夹具必须能解析");
    let (out, report) = compile_all_with(&file, &CompileOptions::default());
    (
        out.errors
            .iter()
            .map(|e| (e.code().to_string(), e.message.clone()))
            .collect(),
        report
            .decls
            .iter()
            .map(|d| (d.name.clone(), d.status))
            .collect(),
    )
}

#[test]
fn decl_exit_runs_s4_after_the_store_close() {
    // ⚠ 本文件**恰好一个** `#[test]`（独立进程 ⇒ 两个 `OnceLock` 都不竞态）。
    env::set_var("SOKO_DECL_METAS", "1");
    env::set_var("SOKO_DECL_MVARS", "1");

    // ① S4 的正例：收口之后 S4 的 `finish` 照样把 binder 元变量解出来 ⇒ 全绿。
    let (errors, decls) = compile(PLUS);
    assert_eq!(
        errors,
        vec![],
        "两条闸门同开时 S4 的正例必须仍然 0 诊断（`finish` 在收口之后跑）：{errors:?}"
    );
    assert!(
        decls
            .iter()
            .any(|(n, s)| n.as_deref() == Some("f") && *s == DeclStatus::Checked),
        "`f` 必须 `decl.checked`（内核判定，不是文本比对）：{decls:?}"
    );

    // ② S4 的负例：码 + 句**逐字不变**（出口不许改形）。
    let (errors, decls) = compile(MINUS);
    assert_eq!(errors.len(), 1, "负例必须恰好一条诊断：{errors:?}");
    assert_eq!(
        errors[0].0, "elab-untyped-binder",
        "码必须逐字不变：{errors:?}"
    );
    assert_eq!(errors[0].1, MSG, "句必须逐字不变");
    assert!(
        decls
            .iter()
            .any(|(n, s)| n.as_deref() == Some("f") && *s != DeclStatus::Checked),
        "负例的 `f` 不许 `decl.checked`：{decls:?}"
    );
}
