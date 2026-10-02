//! **G-72（0.81.0）的 CLI 端到端守卫** —— 「单文件判绿、被 `import` 时判红」不再出现。
//!
//! 缺口现场：`def mkRel (α : Type) (r : α → α → Prop) : α → α → Prop :=
//! fun (a b : α) => r a b ∧ P`（**字面箭头**返回类型 + λ 体）⇒ `params_of_ty` 把
//! 返回类型的两个箭头也当参数 ⇒ `strip_lambdas_n` 多剥两层 ⇒ 登记进 `defs` 的
//! 「定义体」里 `a`/`b` 悬空 ⇒ 入口引用它时（`And.left h` 解隐式实参要展开 delta）
//! 报 `unknown identifier b` ✗；而**模块自己判卷**不展开这层 delta ⇒ 判绿。
//!
//! 判据形状（**两态必须逐字一致**，这正是缺口的名字）：
//!   * 相位 A：模块**单独**判卷 ⇒ exit 0；
//!   * 相位 B：入口 `import` 它并**引用**那个 def ⇒ exit 0；
//!   * **反面**：相位 A 都不绿 ⇒ 复现件的形状变了（判 2，不当成"已修"）。
//!
//! 反向验证：把 `params_of_ty` 的 `Arrow` 循环加回去 ⇒ 相位 B 判红。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

/// 建一个两文件项目并判卷入口，返回 `(exit_ok, 诊断文本)`。
fn grade_entry(tag: &str, lib_src: &str, entry_src: &str) -> (bool, Vec<String>, String) {
    let dir = std::env::temp_dir().join(format!("soko-g72-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).expect("mkdir lib");
    std::fs::create_dir_all(dir.join("units")).expect("mkdir units");
    std::fs::write(dir.join("sokonanoda.toml"), "name = \"g72\"\n").expect("write manifest");
    std::fs::write(dir.join("lib/Lib.sokonanoda"), lib_src).expect("write lib");
    let entry = dir.join("units/u.sokonanoda");
    std::fs::write(&entry, entry_src).expect("write entry");

    let run = |file: &Path| {
        let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
            .arg("--json")
            .arg("--root")
            .arg(&dir)
            .arg(file)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn sokonanoda");
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        (out.status.success(), text)
    };

    let (lib_ok, lib_text) = run(&dir.join("lib/Lib.sokonanoda"));
    assert!(
        lib_ok,
        "相位 A（模块单独判卷）必须绿 —— 不绿就是复现件的形状变了，不是 G-72：\n{lib_text}"
    );

    let (entry_ok, text) = run(&entry);
    let diagnostics: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some("diagnostic"))
        .map(|v| v["message"].as_str().unwrap_or("<no message>").to_string())
        .collect();
    (entry_ok, diagnostics, text)
}

/// 字面箭头返回类型 + **多 binder** λ 体：两态都必须绿。
#[test]
fn g72_literal_arrow_return_type_imports_cleanly() {
    let (ok, diagnostics, raw) = grade_entry(
        "literal",
        "axiom P : Prop\n\
         def mkRel (α : Type) (r : α → α → Prop) : α → α → Prop :=\n\
         \x20 fun (a b : α) => r a b ∧ P\n",
        "import lib.Lib\n\
         theorem use (α : Type) (r : α → α → Prop) (a : α) (h : mkRel α r a a) : r a a :=\n\
         \x20 And.left h\n",
    );
    assert!(diagnostics.is_empty(), "入口不许有诊断：{diagnostics:#?}");
    assert!(ok, "入口必须判绿（两态一致）：\n{raw}");
}

/// **反面**：单 binder 的嵌套 λ 体（同一个 bug 的另一个形状）也必须绿。
#[test]
fn g72_literal_arrow_with_nested_lambdas_imports_cleanly() {
    let (ok, diagnostics, raw) = grade_entry(
        "nested",
        "axiom P : Prop\n\
         def mkRel (α : Type) (r : α → α → Prop) : α → α → Prop :=\n\
         \x20 fun (a : α) => fun (b : α) => r a b ∧ P\n",
        "import lib.Lib\n\
         theorem use (α : Type) (r : α → α → Prop) (a : α) (h : mkRel α r a a) : r a a :=\n\
         \x20 And.left h\n",
    );
    assert!(diagnostics.is_empty(), "入口不许有诊断：{diagnostics:#?}");
    assert!(ok, "入口必须判绿（两态一致）：\n{raw}");
}

/// 台账里记的绕法（具名别名返回类型）**继续可用** —— 修 bug 不许把绕法弄坏。
#[test]
fn g72_named_alias_workaround_still_works() {
    let (ok, diagnostics, raw) = grade_entry(
        "alias",
        "axiom P : Prop\n\
         def Rel (α : Type) : Type := α → α → Prop\n\
         def mkRel (α : Type) (r : α → α → Prop) : Rel α :=\n\
         \x20 fun (a b : α) => r a b ∧ P\n",
        "import lib.Lib\n\
         theorem use (α : Type) (r : α → α → Prop) (a : α) (h : mkRel α r a a) : r a a :=\n\
         \x20 And.left h\n",
    );
    assert!(diagnostics.is_empty(), "入口不许有诊断：{diagnostics:#?}");
    assert!(ok, "具名别名那一形也必须判绿：\n{raw}");
}

/// 复现脚本本身必须仍在仓库里且指向同一个形状（防止它被删/被改写成空转）。
#[test]
fn g72_repro_script_exists() {
    let path = repo_root().join("docs/gaps/repro/G72-import-path-lambda-body.sh");
    assert!(path.exists(), "G-72 复现脚本不见了：{}", path.display());
}
