//! ST2（v0.77.0）的**商类型守卫** —— 把「源语言里真的能用 `Quot`」钉到**归约**上。
//!
//! 为什么判据必须放在**归约**上（而不是"名字在不在"）：
//!   `Quot`/`Quot.mk`/`Quot.lift`/`Quot.ind` 只有在被登记成内核的 **`Declar::Quot`**
//!   时才拿到 `RigidHead::QuotConst` ⇒ `crates/kernel/src/eval.rs` 的 `fire_quot`
//!   才会做 iota 归约。装成普通 `Declar::Axiom`：**名字在、类型对、归约死** ✗
//!   ⇒ `Quot.lift f h (Quot.mk r a)` 卡住，`Eq.refl` 证不出 `… = f a`。
//!   ⇒ 「`Quot.lift` 在 `Quot.mk` 上算得出来」这一条**同时**证明了：
//!   ① 四条被装上了 ② 它们的类型与内核期望一致（否则内核不会认那四个名字）
//!   ③ 声明种类是对的。**它是 ST2 唯一咬得住的那颗牙** ✓。
//!   反向验证：撤掉 `install_quot` 的 re-kind（全装成 `Axiom`）⇒ 本文件判红 ✓。
//!
//! 另半条：**让位口径**（文件自带 `Quot` ⇒ 整族不装）—— 判据是断言
//! `Quot.lift` 报 `elab-unknown-identifier`（不是"悄悄用上了 prelude 的"）。

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

/// 判卷一个复现件，返回 `(exit_ok, decl_checked, diagnostics)`。
fn grade_probe(file: &str) -> (bool, usize, Vec<(String, String)>) {
    let path = repo_root().join("docs/gaps/repro").join(file);
    assert!(path.exists(), "复现件不存在：{}", path.display());
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("--json")
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
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("{file}: 不是合法 JSON 行：{e}\n{line}"));
        match value.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "decl.checked" => checked += 1,
            "diagnostic" => diagnostics.push((
                value["code"].as_str().unwrap_or("<no code>").to_string(),
                value["message"]
                    .as_str()
                    .unwrap_or("<no message>")
                    .to_string(),
            )),
            _ => {}
        }
    }
    (out.status.success(), checked, diagnostics)
}

/// **核心判据**：`Quot` 可用，且 `Quot.lift`/`Quot.ind` 在 `Quot.mk` 上**算得出来**。
///
/// 4 条 `decl.checked` / 0 诊断 / exit 0。第 ③④ 条的证明项是 `Eq.refl` ——
/// 只有归约真的发生才判得过。
#[test]
fn st2_quot_reduces_on_quot_mk() {
    let (ok, checked, diags) = grade_probe("ST2-quot-reduces.sokonanoda");
    assert!(
        ok,
        "ST2：`Quot` 探针必须判卷通过（`Quot.lift`/`Quot.ind` 要能算）：{diags:#?}"
    );
    assert_eq!(
        checked, 4,
        "ST2：应有 4 条 checked，实际 {checked}：{diags:#?}"
    );
    assert!(diags.is_empty(), "ST2：不该有诊断：{diags:#?}");
}

/// **让位口径**：文件自己声明 `Quot` ⇒ 整族不装（不许装一半）。
///
/// 判据形状：断言 `Quot.lift` 报 `elab-unknown-identifier`，且**一条 checked 都没有
/// 来自 prelude**（文件自己那条 `axiom Quot` 会 checked —— 所以只钉 checked 数
/// 与诊断原文，不钉 exit）。
#[test]
fn st2_file_declaring_quot_takes_over_the_family() {
    let (_ok, checked, diags) = grade_probe("ST2-quot-family-yields.sokonanoda");
    assert_eq!(
        checked, 1,
        "ST2：只应有文件自己那条 `axiom Quot` checked（prelude 整族让位）：{diags:#?}"
    );
    assert!(
        diags
            .iter()
            .any(|(code, msg)| code == "elab-unknown-identifier"
                && msg.contains("unknown identifier `Quot.lift`")),
        "ST2：文件自带 `Quot` ⇒ `Quot.lift` 必须是未知标识符（不许装一半）：{diags:#?}"
    );
}

/// **G-75（0.81.0）**：`docs/gaps/repro/G75-no-quot-exact.sokonanoda` 必须判绿
/// （3 条 `decl.checked`、0 条诊断、exit 0）—— 复现件从判红翻成判绿。
///
/// ⚠ 顺带钉住**修前**的一处失真：那份复现件顶部原来写着 `import lib.Rel`，
/// 而 `docs/gaps/repro/` 下**没有** `lib/`（模块根 = 入口目录）⇒ 判卷在 `import`
/// 阶段就报 `import-not-found` ⇒ 缺口**从来没被真正判红过**（红的原因不对）。
/// 这条判据现在要求整份文件判绿 ⇒ 再有人加回那种 import，这里立刻判红 ✓。
#[test]
fn g75_quot_exact_repro_is_green() {
    let (ok, checked, diagnostics) = grade_probe("G75-no-quot-exact.sokonanoda");
    assert!(
        diagnostics.is_empty(),
        "G-75 复现件不许有诊断：{diagnostics:#?}"
    );
    assert!(ok, "G-75 复现件必须判绿（exit 0）");
    assert_eq!(checked, 3, "G-75 复现件的 3 条声明都要判绿");
}
