//! **G-85（0.81.0）的 CLI 端到端守卫** —— **省掉前导隐式实参 + 结果再收一个实参**时，
//! 实参不许被按位置装错 ✗。
//!
//! 形状（`And.right : {a b : Prop} → And a b → b`，前导隐式 k=2 · 显式 arity m=1）：
//!
//! | n | 写法 | 修前 | 读法 |
//! |---|---|---|---|
//! | 1 = m | `And.right h` | ✓ | 短写（隐式前缀补出来） |
//! | **2 = m+1** | **`And.right h x`** | **✗ 红** | 短写 + 富余实参落到**结果**上 |
//! | 3 = k+m | `And.right p (∀…) h x` | ✓ | 旧写法（隐式位逐位写出） |
//! | 4 = k+m+1 | `And.right p (∀…) h x y` | ✓ | 旧写法 + 富余实参 |
//!
//! 病根：路线③（富余实参落到结果上）靠**展开结果类型**造虚拟层，而 `And.right` 的结果
//! 是**变量** `b` ✗ ⇒ 展不动 ⇒ 落到「旧写法」分支（判据 = 实参个数 > 显式层数）⇒
//! **按位置**把 `h` 装进 `a : Prop` ⇒ `期望 Sort(0)，实际是 And …` ✗。
//! 修法：路线③ 加**第二趟** —— 先按显式实参的类型解出前导隐式参数（与短写同一条
//! `solve_prefix`），代进结果再展 ✓。
//!
//! 三条牙（每条都断言**用户可见的结果**）：
//!   ① 四条形状**全绿**（exit 0 · `decl.checked == 4` · 0 诊断）—— 缺口那一形翻绿 ✓；
//!   ② **三个对照照旧绿** ⇒ 修法没把「旧写法」抢走 ✗（路线③ 演进史上**实测踩过**：
//!      放宽判据当场把 prelude 打红 ✓）；
//!   ③ **反面**：真的解不出的隐式实参**仍须**判红并报专用码 ✓ —— 修的是"能解的"，
//!      不许把"解不出"也放过去 ✗。
//!
//! 反向验证：撤掉第二趟 ⇒ ① 的 `n2` 当场判红（本轮实测：修复前 n2 红、n1/n3/n4 绿 ✓）。

use std::process::{Command, Stdio};

/// 判一份源文本，返回 `(exit_ok, decl_checked, diagnostics)`。
///
/// `tag` 进临时目录名：cargo 的测试是**同进程多线程**跑的，共用一份路径会互相覆盖。
fn grade_source(tag: &str, src: &str) -> (bool, usize, Vec<String>) {
    // 返回的第三条 = **诊断码**（`elab-implicit-argument-unsolved` 这类）—— 码在
    // 事件的 `code` 字段里，**不在** `message` 文案里 ✗（实测：按 message 找会找不到 ✓）。
    let dir = std::env::temp_dir().join(format!("soko-g85-{}-{tag}", std::process::id()));
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
                value["code"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        value["message"].as_str().unwrap_or("<no code>").to_string()
                    }),
            ),
            _ => {}
        }
    }
    (out.status.success(), checked, diagnostics)
}

/// 四条形状（缺口那一形 + 三个对照）—— 与 `docs/gaps/repro/G85-*.sh` **逐字同源** ✓。
const SHAPES: &str = "\
theorem n1 (p q : Prop) (h : p \u{2227} q) : q := And.right h

theorem n2 (p : Prop) (Q : Prop \u{2192} Prop) (h : p \u{2227} (\u{2200} (x : Prop), Q x)) (x : Prop) : Q x :=
  And.right h x

theorem n3 (p : Prop) (Q : Prop \u{2192} Prop) (h : p \u{2227} (\u{2200} (x : Prop), Q x)) (x : Prop) : Q x :=
  And.right p (\u{2200} (y : Prop), Q y) h x

theorem n4 (p : Prop) (Q : Prop \u{2192} Prop \u{2192} Prop)
    (h : p \u{2227} (\u{2200} (x : Prop), \u{2200} (y : Prop), Q x y)) (x y : Prop) : Q x y :=
  And.right p (\u{2200} (u : Prop), \u{2200} (v : Prop), Q u v) h x y
";

/// ① + ②：缺口那一形翻绿，且三个对照照旧绿。
#[test]
fn g85_overapplied_with_dropped_implicits_is_accepted() {
    let (ok, checked, diagnostics) = grade_source("shapes", SHAPES);
    assert!(
        diagnostics.is_empty(),
        "四条形状都不许有诊断（`n2` 修前是「期望 Sort(0)，实际是 And …」✗）：{diagnostics:#?}"
    );
    assert!(ok, "四条形状必须判绿（exit 0）：{diagnostics:#?}");
    assert_eq!(checked, 4, "四条声明都要判过（`decl.checked`）");
}

/// ②（单独一条，便于定位）：**旧写法**（隐式位逐位写出）不许被抢走 ✗。
///
/// 为什么单列：路线③ 的判据一放宽就抢旧写法 —— **实测踩过**（prelude 当场打红：
/// `l1_prelude_is_available_in_full_mode` 报 `期望 Pi …, 实际 Sort(0)`）。
#[test]
fn g85_does_not_steal_the_old_style() {
    let src = "\
theorem old_style (p : Prop) (Q : Prop \u{2192} Prop) (h : p \u{2227} (\u{2200} (x : Prop), Q x))
    (x : Prop) : Q x :=
  And.right p (\u{2200} (y : Prop), Q y) h x
";
    let (ok, checked, diagnostics) = grade_source("old-style", src);
    assert!(
        ok && checked == 1 && diagnostics.is_empty(),
        "旧写法（前导隐式位逐位写出）必须照旧判绿：ok={ok} checked={checked} {diagnostics:#?}"
    );
}

/// ③ 反面：真的解不出的隐式实参**仍须**判红 + 报专用码 ✓。
#[test]
fn g85_still_rejects_an_unsolvable_implicit_argument() {
    let src = "\
def ignores {\u{3b1} : Type} (n : Nat) : Nat := n

def uses : Nat := ignores 3
";
    let (ok, _checked, diagnostics) = grade_source("unsolvable", src);
    assert!(!ok, "解不出的隐式实参必须判红：{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.contains("elab-implicit-argument-unsolved")),
        "必须报 elab-implicit-argument-unsolved（既有专用码不许消失）：{diagnostics:#?}"
    );
}
