//! **G-58/G-59（0.81.0）的 CLI 端到端守卫** —— 大消去（large elimination）可用。
//!
//! 判据形状（都断言**用户可见的结果**：判卷退出码 + `decl.checked` 计数 + 诊断）：
//!   * `G56-large-elim-into-type.sokonanoda`（G-58 的复现件）判绿 / 4 条 checked / 0 诊断；
//!   * `G56-type-valued-large-elim.sokonanoda`（G-59 的复现件）判绿 / 4 条 checked / 0 诊断；
//!   * **反面**：显式 `.{1}` 配 `Type` 的 motive 仍判红（推断不许覆盖用户写的层级）。
//!
//! 反向验证：把 `elab.rs::infer_recursor_universes` 的调用点注释掉 ⇒ 前两条判红。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 仓库根（`crates/cli` → 上两级）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

/// 判一份源文本，返回 `(exit_ok, decl_checked, diagnostics)`。
///
/// `tag` 进临时目录名（cargo 的测试同进程多线程 ⇒ 共用路径会互相覆盖）。
fn grade_source(tag: &str, src: &str) -> (bool, usize, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("soko-largeelim-{}-{tag}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Probe.sokonanoda");
    std::fs::write(&path, src).expect("write probe");
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("--json")
        .arg("--no-project")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sokonanoda");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut checked = 0usize;
    let mut diagnostics = Vec::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("不是合法 JSON 行：{e}\n{line}"));
        match value.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "decl.checked" => checked += 1,
            "diagnostic" => diagnostics.push(
                value["message"]
                    .as_str()
                    .unwrap_or("<no message>")
                    .to_string(),
            ),
            _ => {}
        }
    }
    (out.status.success(), checked, diagnostics)
}

/// 判卷 `docs/gaps/repro/<file>`。
fn grade_probe(file: &str) -> (bool, usize, Vec<String>) {
    let path = repo_root().join("docs/gaps/repro").join(file);
    assert!(path.exists(), "复现件不存在：{}", path.display());
    let src = std::fs::read_to_string(&path).expect("read probe");
    grade_source(file, &src)
}

#[test]
fn g58_repro_is_green() {
    let (ok, checked, diagnostics) = grade_probe("G56-large-elim-into-type.sokonanoda");
    assert!(
        diagnostics.is_empty(),
        "G-58 复现件不许有诊断：{diagnostics:#?}"
    );
    assert!(ok, "G-58 复现件必须判绿（exit 0）");
    assert_eq!(checked, 4, "G-58 复现件的 4 条声明都要判绿");
}

#[test]
fn g59_repro_is_green() {
    let (ok, checked, diagnostics) = grade_probe("G56-type-valued-large-elim.sokonanoda");
    assert!(
        diagnostics.is_empty(),
        "G-59 复现件不许有诊断：{diagnostics:#?}"
    );
    assert!(ok, "G-59 复现件必须判绿（exit 0）");
    assert_eq!(checked, 4, "G-59 复现件的 4 条声明都要判绿");
}

/// **反面**：显式 `.{1}` 配 `Type` 的 motive 仍判红 —— 推断只补**裸** `Foo.rec`，
/// 用户写了 `.{n}` 就完全听用户的。
#[test]
fn explicit_level_still_wins() {
    let src = "inductive MyBox : Type\n\
               ctor MyBox.mk (n : Nat) : MyBox\n\
               end\n\
               def boxElim1 (h : MyBox) : Type :=\n\
               \x20 MyBox.rec.{1} (fun (_ : MyBox) => Type) (fun (n : Nat) => Nat) h\n";
    let (ok, checked, diagnostics) = grade_source("explicit", src);
    assert!(!ok, "显式 `.{{1}}` 配 `Type` 必须判红");
    assert_eq!(checked, 1, "只有 `MyBox` 本身判绿");
    assert!(!diagnostics.is_empty(), "必须给出诊断");
}
