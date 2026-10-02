//! **G-74（0.81.0）的 CLI 端到端守卫** —— 排中律 `Classical.em` 可用。
//!
//! 判据形状（都断言**用户可见的结果**：判卷退出码 + `decl.checked` 计数 + 诊断）：
//!   * `docs/gaps/repro/G74-no-classical-logic.sokonanoda` 判绿（6 条 checked、0 诊断）；
//!   * **反面**：`theorem bad (P : Prop) : P` 判红 —— 公理只补排中律，不许把逻辑
//!     弄成平凡的 ✗；
//!   * **反面**：`theorem bad_false : False` 判红（同上）。
//!
//! 反向验证：把 B10 族从 `L1_FAMILIES` 里删掉 ⇒ 第一条判红。

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
fn grade_source(tag: &str, src: &str) -> (bool, usize, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("soko-classical-{}-{tag}", std::process::id()));
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

#[test]
fn g74_repro_is_green() {
    let path = repo_root()
        .join("docs/gaps/repro")
        .join("G74-no-classical-logic.sokonanoda");
    assert!(path.exists(), "复现件不存在：{}", path.display());
    let src = std::fs::read_to_string(&path).expect("read probe");
    let (ok, checked, diagnostics) = grade_source("repro", &src);
    assert!(
        diagnostics.is_empty(),
        "G-74 复现件不许有诊断：{diagnostics:#?}"
    );
    assert!(ok, "G-74 复现件必须判绿（exit 0）");
    assert_eq!(checked, 6, "G-74 复现件的 6 条声明都要判绿");
}

/// **反面**：公理只补排中律 —— 任意命题与 `False` 都**不许**变得可证。
#[test]
fn classical_does_not_make_logic_trivial() {
    for (tag, src) in [
        ("any-p", "theorem bad (P : Prop) : P := Classical.em P\n"),
        ("false", "theorem bad_false : False := Classical.em False\n"),
    ] {
        let (ok, checked, diagnostics) = grade_source(tag, src);
        assert!(!ok, "`{tag}` 必须判红");
        assert_eq!(checked, 0, "`{tag}`：一条都不许判绿");
        assert!(!diagnostics.is_empty(), "`{tag}`：必须给出诊断");
    }
}
