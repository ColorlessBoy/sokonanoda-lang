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
    grade_source_env(tag, src, &[])
}

/// 同上，但给子进程注入环境变量（`SOKO_UNIVERSE_METAVAR` 是**进程级**开关 ✓）。
fn grade_source_env(tag: &str, src: &str, envs: &[(&str, &str)]) -> (bool, usize, Vec<String>) {
    // 返回的第三条 = **诊断码**（`elab-implicit-argument-unsolved` 这类）—— 码在
    // 事件的 `code` 字段里，**不在** `message` 文案里 ✗（实测：按 message 找会找不到 ✓）。
    let dir = std::env::temp_dir().join(format!("soko-g85-{}-{tag}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Probe.sokonanoda");
    std::fs::write(&path, src).expect("write probe");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    for (k, v) in envs {
        cmd.env(k, v);
    }
    // **诊断（2026-10-03 ✓）**：CI 上"开关开"那一支的表现**逐字等于"开关关"** ✗
    // （u1 实测 2/2 = off 期望 ✓；b1 实测 5/2 = off 期望 ✓），而本地全绿 ✓
    // ⇒ 先确定一件事：**开关到底有没有进到子进程** ✓。本串会随断言失败**原样进 panic 文本** ✓，
    // 所以下一轮 CI 直接给出答案 ✓，不必再猜（"OnceLock 跨测试污染"已被源码与串行实验否掉 ✗）。
    let diag = format!(
        "注入={envs:?} · cmd 上 SOKO_*={:?} · 父进程 SOKO_*={:?}",
        cmd.get_envs()
            .filter(|(k, _)| k.to_string_lossy().starts_with("SOKO_"))
            .map(|(k, v)| (
                k.to_string_lossy().into_owned(),
                v.map(|x| x.to_string_lossy().into_owned())
            ))
            .collect::<Vec<_>>(),
        std::env::vars()
            .filter(|(k, _)| k.starts_with("SOKO_"))
            .collect::<Vec<_>>(),
    );
    // 打进 stderr ⇒ cargo test **只在失败时**回显它 ✓ —— CI 红的那两条测试的 panic 文本里就有这一行 ✓。
    eprintln!("· 诊断：{diag}");
    let out = cmd
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

#[test]
fn u1_solves_an_unwritten_universe_level_only_when_the_switch_is_on() {
    let src = "\
axiom Show {u} : {\u{3b1} : Sort u} \u{2192} \u{3b1} \u{2192} Nat \u{2192} Nat\n\
def t1 : Nat := Show 0 5\n\
def t2 : Nat := Show.{1} 0 5\n\
def bad : Nat := Show.{0} 0 5\n";

    // ① 开关关：`Show 0 5`（缺口面）与 `Show.{0} 0 5`（写死错的层级）判红 ✓，
    //    对照 `Show.{1} 0 5` 与 axiom 判过 ⇒ checked=2 · 2 条诊断 ✓。
    let (ok_off, checked_off, diags_off) = grade_source_env("u1-off", src, &[]);
    // G-30 收口（2026-10-05）后层 mvar 恒生成并解出（对齐 Lean `mkFreshLevelMVars`）：
    // `Show 0 5` 的 `u` 由 `α := Nat` 解出 ⇒ 开关**关**时也从红转绿 ⇒ checked=3、仅 `bad` 红。
    assert!(
        !ok_off && checked_off == 3 && diags_off.len() == 1,
        "开关**关**时：`Show 0 5` 已判过（G-30 恒解层）、仅写死错的 `bad` 红 ⇒ ok={ok_off} checked={checked_off} diags={diags_off:#?}"
    );

    // ②③ 开关开：`Show 0 5` 转绿（`u := 1` ✓），只有写死错的层级仍红 ✓。
    let (ok_on, checked_on, diags_on) =
        grade_source_env("u1-on", src, &[("SOKO_UNIVERSE_METAVAR", "1")]);
    assert!(
        checked_on == 3 && diags_on.len() == 1,
        "开关**开**时：只有 `Show.{{0}} 0 5` 该判红 ⇒ checked={checked_on} diags={diags_on:#?}"
    );
    assert!(
        !ok_on,
        "反面仍在 ⇒ 整份文件仍 exit 1（**预期** ✓：说明不是「什么都放行」✗）"
    );
}

/// **B1 片的端到端守卫**（范围 B · 开关 `SOKO_ARG_EXPECTED`，默认关 ✓）——
/// **短写的隐式调用当实参时，期望类型必须送到实参位** ✓（台账 **G-86** ✗）。
///
/// 形状：`h (Or.inl hp)`（`h : ¬ (P ∨ Q)`）—— `Or.inl` 的 `?B` 唯一来源就是实参位的
/// 期望类型（`h` 的域 `P ∨ Q` ✓）；修前落到「按兄弟同形兜底」⇒ `?B := ?A := P` ✗
/// ⇒ 内核 `期望 ((Or P) Q)，实际 ((Or P) P)` ✗。
///
/// 修法**零内核调用、零递归** ✓：头是**局部变量**时它的书写类型就在 `scope.src_tys` ✓
/// ⇒ 直接剥 Π 到实参位（`¬ X` 是 def 头 ⇒ δ 展开一次 ✓）—— 不走 `application_arg_expected`
/// ✗（那条要 `judge_infer` 头 ⇒ 判定再入 ⇒ **栈溢出** exit 134 ✗，见 G-86 notes ✓）。
///
/// 四条牙：① 开关关 ⇒ `t3` 红 ✓（基线 ✓）；② 开关开 ⇒ `t3` 绿 ✓；③ 两个对照
/// （期望位=显式目标 / 前导写全）两态都绿 ✓；④ **反面**：真解不出的 `ignores 3`
/// 开关开时**仍须**红 ✓（不是"什么都放行" ✗）。
///
/// 与 `docs/gaps/repro/B1-arg-expected-solved.sh` **逐字同源** ✓。
#[test]
fn b1_argument_expected_type_reaches_a_short_implicit_call() {
    let src = "\
axiom P : Prop\n\
axiom Q : Prop\n\
\n\
theorem t (hp : P) : P \u{2228} Q := Or.inl hp\n\
theorem t2 (h : \u{ac} (P \u{2228} Q)) (hp : P) : False := h (Or.inl P Q hp)\n\
theorem t3 (h : \u{ac} (P \u{2228} Q)) (hp : P) : False := h (Or.inl hp)\n\
\n\
def ignores {\u{3b1} : Type} (n : Nat) : Nat := n\n\
def uses : Nat := ignores 3\n";

    // ① 开关关：`t3`（缺口面）与 `uses`（真解不出）判红 ✓，其余 5 条判过 ✓。
    let (ok_off, checked_off, diags_off) =
        grade_source_env("b1-off", src, &[("SOKO_ARG_EXPECTED", "0")]);
    assert!(
        !ok_off && checked_off == 5 && diags_off.len() == 2,
        "开关**关**时：`t3` 与 `uses` 判红、其余 5 条判过 ⇒ ok={ok_off} checked={checked_off} diags={diags_off:#?}"
    );

    // ②③④ 开关开：`t3` 转绿（`?B := Q` ✓），只有真解不出的 `uses` 仍红 ✓。
    let (ok_on, checked_on, diags_on) =
        grade_source_env("b1-on", src, &[("SOKO_ARG_EXPECTED", "1")]);
    assert!(
        !ok_on && checked_on == 6 && diags_on.len() == 1,
        "开关**开**时：只有 `uses` 该判红（`t3` 转绿 ✓、两个对照仍绿 ✓）⇒ ok={ok_on} checked={checked_on} diags={diags_on:#?}"
    );
    assert!(
        diags_on
            .iter()
            .any(|d| d.contains("elab-implicit-argument-unsolved")),
        "反面必须是**既有专用码** `elab-implicit-argument-unsolved`（不是「什么都放行」✗）：{diags_on:#?}"
    );
}
