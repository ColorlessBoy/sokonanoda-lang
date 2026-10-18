//! **IA-5（`SUB-implicit-args-align-lean4`，用户 2026-10-16 08:5x 亲报）的 CLI 端到端守卫**
//! —— **只有 `@` 形式接收隐式参数，正常形式不接受隐式参数**（对齐 Lean 4）
//! + **报错要完善、指令要明确**（报错里给的出路**必须照抄就能用**）。
//!
//! 用户原话（逐字）：「关键是 sokonanoda **不需要 at 符号就可以输入隐式参数**，也很难受，
//! 真不如**对齐 lean**：**只有 `@` 形式接收隐式参数，正常形式不接受隐式参数**，
//! 并且**支持显式地指定参数名字**。」+「**报错信息要完善，指令要明确**」。
//!
//! 用户实测的三条（本文件逐条咬住）：
//!
//! ```text
//! def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a
//! def b1 : Nat := h (fun (n : Nat) => n) 5            ⇒ checked ✓（正确，不许被误伤）
//! def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5    ⇒ 改前 checked ✗ ⇒ 严格档**必须判红** ✓
//! def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5   ⇒ checked ✓（正确，不许被误伤）
//! ```
//!
//! **Lean 4 对照**（`~/Documents/lean/lean4/src/Lean/Elab/App.lean`，本机源码 ✓）：
//! `:706` `processImplicitArg` = `if (← read).explicit then processExplicitArg else addImplicitArg`
//! —— `@`（`:709`）让隐式 binder 也吃一个位置实参；正常形式（`:711`）给隐式 binder
//! 插元变量、**不碰实参表**；`getNextArg?`（`:1069`）的注释说得很直白：隐式实参
//! 「**不是用命名实参给的**」就拿不到 ⇒ **命名实参是正常形式下指定隐式的通道** ✓。
//!
//! ## 本轮的档位（**2026-10-18 翻默认** ✓）
//!
//! **严格档 = 默认档**（不设任何环境变量 ⇒ 正常形式不接受位置隐式实参 ✓）；
//! **逃生门 `SOKO_STRICT_AT=0|off|false` ⇒ 关**（退回旧行为，给没迁完的语料用 ✓）。
//! 翻默认的前提是**语料迁移做完**（a1–a6 六棒）—— 逐文件账见
//! `.cache/ux/fix-a6/VERDICT.md` ✓。
//!
//! 因此本文件有**两条互补的牙**：
//!   * `default_mode_is_strict`：**不设开关 ⇒ 严格档** ✓（用户判据：`g Nat 5` 被拒 ✓）；
//!   * `strict_at_off_restores_the_old_semantics`：`SOKO_STRICT_AT=0` ⇒ **退回旧行为** ✓
//!     （逃生门**真的能退**，不是摆设 ✗）。
//!
//! ## 四条牙（每条都断言**用户可见的结果**）
//!
//!   ① **正对照**：`b1`（正常形式、隐式靠推断）与 `b3`（`@` 全显式）**照旧 checked** ✓
//!      —— 「正确的不许被误伤」是硬要求 ✗✗；
//!   ② **判红**：`b2` 必须被拒 ✓，且诊断码可归属 ✓；
//!   ③ **报错可执行**：诊断文案必须（a）**点名**隐式形参名 + （b）给出**可复制**的出路
//!      —— 而且那条出路**粘回去必须判绿** ✓（用户第 ③ 条「指令要明确」的唯一硬判据 ✓：
//!      光断言"文案里有 `@`"会放过一条**写错的**出路 ✗ —— 本轮就实测抓到一条：
//!      `@ignores 3` 把 `3` 装到 `{α}` 位上 ✗，已改成 `@ignores <α> 3` ✓）；
//!   ④ **反向验证**：默认（= 严格档）⇒ `b2` 判红 · `SOKO_STRICT_AT=0` ⇒ `b2` 判绿 ✓
//!      （咬得住 ✓，不是恒真 ✗）。

use std::process::{Command, Stdio};

/// 判一份源文本，返回 `(exit_ok, checked_names, diagnostics)`；诊断串 = `码\u{1}文案`。
///
/// `tag` 进临时目录名：cargo 的测试是**同进程多线程**跑的，共用一份路径会互相覆盖。
fn grade(tag: &str, src: &str, envs: &[(&str, &str)]) -> (bool, Vec<String>, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("soko-ia5-{}-{tag}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Probe.sokonanoda");
    std::fs::write(&path, src).expect("write probe");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    // **测试环境隔离**：`Command` 默认**继承**父进程环境 ⇒ 开发机/CI 只要外部设过
    // `SOKO_STRICT_AT`，「默认档」那一条量的就不是默认值而是环境值 ✗（判据随环境漂移
    // = 假红/假绿 ✓）。先摘掉，再注入本用例显式要的那一档 ✓。
    cmd.env_remove("SOKO_STRICT_AT");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd
        .arg("--json")
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut checked = Vec::new();
    let mut diags = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // 事件是**一行一个 JSON**；本用例只认 `type` 与 `name` / `message`
        // （按整串 substring 找字段 ⇒ 不引 serde 依赖，测试二进制更小 ✓）。
        if line.contains("\"type\":\"decl.checked\"") {
            if let Some(name) = json_str_field(line, "name") {
                checked.push(name);
            }
        } else if line.contains("\"type\":\"diagnostic\"") {
            // ⚠ **码在 `code` 字段里，不在 `message` 文案里** ✗（实测：按文案找找不到 ✓）
            // ⇒ 拼成 `码\u{1}文案` 一起交出去 ✓（`\u{1}` 不可能出现在两者里 ✓）。
            let code = json_str_field(line, "code").unwrap_or_default();
            let msg = json_str_field(line, "message").unwrap_or_default();
            diags.push(format!("{code}\u{1}{msg}"));
        }
    }
    (out.status.success(), checked, diags)
}

/// 从一行事件 JSON 里取一个**字符串**字段（够用就好：这些字段都是普通 JSON 串）。
fn json_str_field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let start = line.find(&pat)? + pat.len();
    let rest = &line[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => return Some(out),
            },
            other => out.push(other),
        }
    }
    Some(out)
}

/// 用户原案（逐字，含 `b1`/`b2`/`b3` 三个形状）。
const REPRO: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b1 : Nat := h (fun (n : Nat) => n) 5
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5
";

/// ④ **反向验证的第一半 + 用户判据**：**不设开关 ⇒ 严格档** ——
/// `b2`（正常形式喂位置隐式实参）**必须判红**，`b1`/`b3` 照旧绿 ✓。
///
/// 为什么必须有这一条 ✗：这是**用户 2026-10-16 要的语义**（「只有 `@` 形式接收隐式
/// 参数，正常形式不接受隐式参数」）—— 翻默认之后它必须**不带任何环境变量**就成立 ✓。
#[test]
fn default_mode_is_strict() {
    let (ok, checked, diags) = grade("default", REPRO, &[]);
    assert!(!ok, "默认（= 严格档）下 `b2` 必须判红：{diags:?}");
    for name in ["h", "b1", "b3"] {
        assert!(
            checked.iter().any(|c| c == name),
            "默认档下 `{name}` 必须照旧 checked：{checked:?}"
        );
    }
    assert!(
        !checked.iter().any(|c| c == "b2"),
        "`b2`（正常形式喂位置隐式实参）默认档必须被拒 ✗：{checked:?}"
    );
}

/// ④ **反向验证的第二半 + 逃生门**：`SOKO_STRICT_AT=0` ⇒ **退回旧行为**（四个声明全绿 ✓）。
///
/// 为什么必须有这一条 ✗：逃生门是"遇到没迁完的语料能一条环境变量退回去"的**唯一**保证 ✓
/// —— 只断言"开关存在"会放过一条**接错极性**的逃生门 ✗（`=0` 必须**真的**退回旧行为）。
#[test]
fn strict_at_off_restores_the_old_semantics() {
    let (ok, checked, diags) = grade("off", REPRO, &[("SOKO_STRICT_AT", "0")]);
    assert!(
        ok,
        "`SOKO_STRICT_AT=0` 必须退回旧行为（四个声明全绿）：checked={checked:?} diags={diags:?}"
    );
    for name in ["h", "b1", "b2", "b3"] {
        assert!(
            checked.iter().any(|c| c == name),
            "`SOKO_STRICT_AT=0` 下 `{name}` 必须照旧 checked：{checked:?}"
        );
    }
}

/// ① + ②（严格档）：正对照照旧绿、`b2` 判红。
#[test]
fn strict_at_rejects_positional_implicit_arguments() {
    let (ok, checked, diags) = grade("strict", REPRO, &[("SOKO_STRICT_AT", "1")]);
    assert!(!ok, "严格档下 `b2` 必须判红（实测 checked={checked:?}）");
    // ① **正对照**：`h`（声明本身）、`b1`（隐式靠推断）、`b3`（`@` 全显式）都要在 ✓。
    for name in ["h", "b1", "b3"] {
        assert!(
            checked.iter().any(|c| c == name),
            "`{name}` 必须照旧 checked（正确的不许被误伤 ✗）：{checked:?}"
        );
    }
    // ② `b2` **不在** checked 里（不是"报了个警告"那种放过 ✓）。
    assert!(
        !checked.iter().any(|c| c == "b2"),
        "`b2`（正常形式喂位置隐式实参）必须被拒 ✗：{checked:?}"
    );
    assert_eq!(diags.len(), 1, "恰好一条诊断：{diags:?}");
    assert!(
        diags[0].starts_with("elab-implicit-argument-unsolved\u{1}"),
        "诊断码可归属（码在 `code` 字段里 ✗）：{}",
        diags[0]
    );
}

/// ③ 报错必须**可执行**（严格档那条）：(a) 点名隐式形参 · (b) 给出可复制的 `@` 出路
/// —— 且那条出路**粘回去判绿** ✓。
#[test]
fn the_strict_error_names_the_implicit_parameters_and_its_escape_actually_works() {
    let (_, _, diags) = grade("msg", REPRO, &[("SOKO_STRICT_AT", "1")]);
    assert_eq!(diags.len(), 1, "恰好一条诊断：{diags:?}");
    let msg = &diags[0];
    // (a) **点名**两个隐式形参（源级名 `α` / `β`，不是 fresh 名 `\0soko_p2` ✗）。
    for name in ["α", "β"] {
        assert!(msg.contains(name), "文案必须点名隐式形参 `{name}`：{msg}");
    }
    assert!(
        !msg.contains("soko_p"),
        "文案不许印内部的 fresh 形参名 ✗：{msg}"
    );
    // (c) 不许只说"把参数写全"而**不说用哪个语法** ✗（用户点名的反例 ✓）。
    assert!(
        !msg.contains("把参数写全"),
        "文案不许只说「把参数写全」而不给语法 ✗：{msg}"
    );
    assert!(msg.contains('@'), "必须给出 `@` 这条确切出路：{msg}");
    // (b) **出路必须真的能用**：把 `@h Nat Nat (fun (n : Nat) => n) 5` 抠出来重判 ⇒ 必须绿 ✓。
    let escape = msg
        .split('`')
        .find(|s| s.starts_with("@h "))
        .unwrap_or_else(|| panic!("文案里找不到可复制的 `@h …` 出路：{msg}"))
        .to_string();
    let probe = format!(
        "def h {{α : Type}} {{β : Type}} (f : α → β) (a : α) : β := f a\n\ndef esc : Nat := {escape}\n"
    );
    let (ok, checked, diags) = grade("escape", &probe, &[]);
    assert!(
        ok && checked.iter().any(|c| c == "esc"),
        "文案给出的出路 `{escape}` 必须**照抄就判绿** ✓（实测 ok={ok} checked={checked:?} diags={diags:?}）"
    );
}

/// ③ 报错必须**可执行**（**默认档**那条 —— 用户 2026-10-15 亲见的就是它）：
/// 「`…` 里有 N 个**隐式**参数，但**补不出来**。**把参数写全**，例如 `f …` 逐位写下来」
/// ⇒ 用户点名两个问题：**没说用哪个语法** ✗ · **没点名是哪一个参数** ✗。
///
/// 本条的牙：文案必须点名 `α` + 给 `@ignores <α> 3`（**占位符**），且把占位符换成
/// `Nat` 之后**必须判绿** ✓ —— 「照做就对」是「指令明确」的唯一硬判据 ✓。
#[test]
fn the_unsolved_error_names_the_parameter_and_its_escape_actually_works() {
    // 形状取自 `crates/cli/tests/implicit_application.rs::g85_still_rejects_an_unsolvable_implicit_argument`
    // （**既有**的默认档形状 ✓，不是本用例新造的 ✓）。
    let src = "\
def ignores {α : Type} (n : Nat) : Nat := n

def uses : Nat := ignores 3
";
    let (ok, _checked, diags) = grade("unsolved", src, &[]);
    assert!(!ok, "解不出的隐式实参必须判红：{diags:?}");
    let msg = diags
        .iter()
        .find(|d| d.starts_with("elab-implicit-argument-unsolved\u{1}"))
        .unwrap_or_else(|| panic!("必须报 elab-implicit-argument-unsolved：{diags:?}"));
    let msg = msg.split('\u{1}').nth(1).unwrap_or(msg).to_string();
    let msg = &msg;
    // (a) **点名**参数（源级名 ✓）+ 位置 ✓。
    assert!(msg.contains("`α`"), "文案必须点名隐式形参 `α`：{msg}");
    assert!(msg.contains("前 1 个"), "文案必须给位置：{msg}");
    assert!(
        !msg.contains("soko_p"),
        "文案不许印内部的 fresh 形参名 ✗：{msg}"
    );
    // (c) 旧文案那句"把参数写全"（没说语法 ✗）必须**消失** ✓。
    assert!(
        !msg.contains("把参数写全"),
        "文案不许只说「把参数写全」而不给语法 ✗：{msg}"
    );
    // (b) **出路必须真的能用**：文案给 `@ignores <α> 3` ⇒ 把 `<α>` 换成 `Nat` 后必须判绿 ✓。
    let escape = msg
        .split('`')
        .find(|s| s.starts_with("@ignores ") && s.contains('<'))
        .unwrap_or_else(|| panic!("文案里找不到带占位符的 `@ignores <…> …` 出路：{msg}"))
        .to_string();
    let filled = escape.replace("<α>", "Nat");
    assert_ne!(filled, escape, "占位符必须能被填上：{escape}");
    let probe =
        format!("def ignores {{α : Type}} (n : Nat) : Nat := n\n\ndef uses : Nat := {filled}\n");
    let (ok, checked, diags) = grade("unsolved-escape", &probe, &[]);
    assert!(
        ok && checked.iter().any(|c| c == "uses"),
        "文案给出的出路 `{escape}`（填成 `{filled}`）必须**照抄就判绿** ✓\
         （实测 ok={ok} checked={checked:?} diags={diags:?}）"
    );
}

/// **正对照的第二半**：`@` 与正常形式在**能推断**时判得一样 ✓ —— 两条一起 ⇒ 「`@` 没被误伤」✗。
#[test]
fn at_sign_still_works_in_both_modes() {
    let src = "\
def g {α : Type} (x : α) : α := x

def ok1 : Nat := g 5
def ok2 : Nat := @g Nat 5
";
    for (tag, envs) in [
        ("at-default", Vec::new()),
        ("at-strict-on", vec![("SOKO_STRICT_AT", "1")]),
        ("at-strict-off", vec![("SOKO_STRICT_AT", "0")]),
    ] {
        let (ok, checked, diags) = grade(tag, src, &envs);
        assert!(ok, "[{tag}] 两条都必须绿（实测 diags={diags:?}）");
        for name in ["ok1", "ok2"] {
            assert!(
                checked.iter().any(|c| c == name),
                "[{tag}] `{name}` 必须绿：{checked:?}"
            );
        }
    }
}
