//! **G-76（0.81.0）的 CLI 端到端守卫** —— ℕ 的**递归方程是定义等式**。
//!
//! 为什么判据必须放在**判卷退出码 + 事件计数**上：`Nat.add` 是 prelude 的
//! **自引用占位定义**，归约全在 `crates/kernel/src/eval.rs` 的原生规则里。
//! 「名字在不在、类型对不对」都咬不住它 —— 唯一咬得住的是
//! **`Eq.refl` 能不能证出那两条方程**（那要内核真的把它们判成定义相等）。
//!
//! 三条牙（每条都断言**用户可见的结果**）：
//!   ① `docs/gaps/repro/G76-nat-add-not-unfoldable.sokonanoda` 判绿
//!      （5 条 `decl.checked`、0 条诊断、exit 0）—— 复现件从判红翻成判绿；
//!   ② 同一形状写进**新文件**也判绿（不是"复现件特供"）；
//!   ③ **反面**：`Nat.add n m = m` 判红（exit 1）—— 判定不许退化。
//!
//! 反向验证：把 `nat_red_recursive_equation` 的递归方程删掉 ⇒ ①② 判红。

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
/// `tag` 进临时目录名：cargo 的测试是**同进程多线程**跑的，共用一份路径会互相
/// 覆盖（实测：三条测试同时跑 ⇒ 读到别人的源文本 ✗）。
fn grade_source(tag: &str, src: &str) -> (bool, usize, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("soko-g76-{}-{tag}", std::process::id()));
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

/// ① 缺口复现件必须判绿（这是 G-76 的验收判据本身）。
#[test]
fn g76_repro_is_green() {
    let (ok, checked, diagnostics) = grade_probe("G76-nat-add-not-unfoldable.sokonanoda");
    assert!(
        diagnostics.is_empty(),
        "G-76 复现件不许有诊断：{diagnostics:#?}"
    );
    assert!(ok, "G-76 复现件必须判绿（exit 0）");
    assert_eq!(checked, 5, "G-76 复现件的 5 条声明都要判绿");
}

/// ② 同一形状写在新文件里也判绿 —— 递归方程是**语言**行为，不是复现件特供。
#[test]
fn g76_equations_hold_in_a_fresh_file() {
    let src = "theorem add_succ (n m : Nat) :\n\
               \x20   Nat.add n (Nat.succ m) = Nat.succ (Nat.add n m) :=\n\
               \x20 Eq.refl.{1} Nat (Nat.succ (Nat.add n m))\n\
               theorem add_zero (n : Nat) : Nat.add n Nat.zero = n :=\n\
               \x20 Eq.refl.{1} Nat n\n\
               theorem zero_add (n : Nat) : Nat.add Nat.zero n = n :=\n\
               \x20 Nat.rec (fun (k : Nat) => Nat.add Nat.zero k = k)\n\
               \x20   (Eq.refl.{1} Nat Nat.zero)\n\
               \x20   (fun (k : Nat) (ih : Nat.add Nat.zero k = k) =>\n\
               \x20     congrArg.{1} Nat Nat (Nat.add Nat.zero k) k Nat.succ ih)\n\
               \x20   n\n";
    let (ok, checked, diagnostics) = grade_source("fresh", src);
    assert!(diagnostics.is_empty(), "不许有诊断：{diagnostics:#?}");
    assert!(ok, "递归方程必须判绿");
    assert_eq!(checked, 3, "三条声明都要判绿");
}

/// ③ **反面**：判定不许退化 —— `Nat.add n m = m` 必须仍判红。
#[test]
fn g76_equations_do_not_degenerate() {
    let src = "theorem bad (n m : Nat) : Nat.add n m = m := Eq.refl.{1} Nat m\n";
    let (ok, checked, diagnostics) = grade_source("degenerate", src);
    assert!(!ok, "`Nat.add n m = m` 必须判红");
    assert_eq!(checked, 0, "一条都不许判绿");
    assert!(!diagnostics.is_empty(), "必须给出诊断");
}
