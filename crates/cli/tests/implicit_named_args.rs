//! **命名实参**（`g (α := Nat) 5`）的 CLI 端到端守卫 —— 用户 2026-10-16 第 ② 条
//! （队列 `SUB-implicit-named-args`）。
//!
//! 用户原话（逐字）：「真不如**对齐 lean**：**只有 `@` 形式接收隐式参数，正常形式
//! 不接受隐式参数**，并且**支持显式地指定参数名字**。」
//!
//! **Lean 4 对照**（本机源码 `~/Documents/lean/lean4/src/Lean/Elab/App.lean` ✓）：
//! * `:753` 主循环**先**问 `findBinderName? s.namedArgs binderName`（`:87` = 按 binder
//!   名线性查找）—— 命中就用它、且**不消耗位置实参**，**无论 binder 是什么风格**
//!   （那条 `match binfo` 在 `| none =>` 分支里 ⇒ **命名优先于风格** ✓）；
//! * `:1073` `getNextArg?` 同款（先命名、后位置；位置只在 `isExplicit` 时才吃）；
//! * `:238`/`:39` 名字对不上时抛 `throwInvalidNamedArg`：**一个码 + 列出可用形参名**。
//! * 语法侧 `Lean/Parser/Term.lean:808` `namedArgument` 是 `argument`（`:816`）的候选
//!   ⇒ 命名实参**只出现在应用实参位** ✓（我们同款）。
//!
//! ## 五条牙（每条都断言**用户可见的结果**）
//!
//!   ① **用户原案**：`g (α := Nat) 5` 改前 `unexpected-token`（语法就不过 ✗）⇒ 改后
//!      **必须 checked** ✓；
//!   ② **正对照**：`g 5` / `@g Nat 5` / `h (fun …) 5` / `@h Nat Nat (fun …) 5`
//!      **照旧 checked** ✓（「正确的不许被误伤」✗✗）；`h Nat Nat … 5`（`b2`）在**默认档**
//!      照旧接受 ✓（默认档零行为变更 ✓ —— 严格档是另一条子任务的开关）；
//!   ③ **任意顺序**：`h (β := Nat) (α := Nat) …` ⇒ checked ✓（按**名字**绑定，不看书写位置 ✓）；
//!   ④ **判红 + 文案可用**：名字写错 ⇒ `elab-invalid-named-argument`，文案里**列出真实形参名**
//!      —— 而且**把列出的名字抠出来粘回去必须判绿** ✓（「指令要明确」的唯一硬判据 ✓）；
//!   ⑤ **同名给两次** ⇒ 判红 ✓（Lean 4 也报错，措辞是 `unused named arguments`）。

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

/// 用户原案（逐字）。
const REPRO: &str = "\
def g {α : Type} (x : α) : α := x

def a1 : Nat := g 5
def a2 : Nat := g (α := Nat) 5
def a3 : Nat := @g Nat 5
";

/// ① **用户第 ② 条本体**：`g (α := Nat) 5` 必须 checked（改前是 `unexpected-token` ✗）。
#[test]
fn named_argument_binds_by_name() {
    let (ok, checked, diags) = grade("bind", REPRO, &[]);
    assert!(
        ok,
        "`g (α := Nat) 5` 必须被接受：checked={checked:?} diags={diags:?}"
    );
    for name in ["g", "a1", "a2", "a3"] {
        assert!(
            checked.iter().any(|c| c == name),
            "`{name}` 应当 checked；实得 {checked:?} diags={diags:?}"
        );
    }
}

/// ② **正对照**：正确的不许被误伤 ✗✗ —— 正常形式（隐式靠推断）与 `@` 写法都照旧 ✓；
/// 而**位置喂隐式**（`b2`）在默认档（= 严格档）被拒 ✓、`SOKO_STRICT_AT=0` 下退回旧行为 ✓。
///
/// **2026-10-18 翻默认**：本条原先把 `b2` 当"默认档正对照"（那时严格档默认关 ✗）——
/// 现在默认即严格档 ⇒ 正对照收窄到 `b1`/`b3`，`b2` 的两档读数分别由
/// `crates/cli/tests/implicit_at_alignment.rs` 与这里下面那一半咬住 ✓。
#[test]
fn positive_controls_are_untouched() {
    let src = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b1 : Nat := h (fun (n : Nat) => n) 5
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5
";
    let (ok, checked, diags) = grade("pos", src, &[]);
    assert!(
        !ok,
        "默认档下 `b2` 该判红（其余三条不许被误伤）：diags={diags:?}"
    );
    for name in ["h", "b1", "b3"] {
        assert!(
            checked.iter().any(|c| c == name),
            "`{name}` 应当 checked（**正对照**）；实得 {checked:?} diags={diags:?}"
        );
    }
    assert!(
        !checked.iter().any(|c| c == "b2"),
        "`b2`（位置喂隐式）默认档必须被拒 ✗；实得 {checked:?}"
    );
    // 逃生门：`SOKO_STRICT_AT=0` ⇒ **四条全绿**（退回旧行为 ✓）。
    let (ok_off, checked_off, diags_off) = grade("pos-off", src, &[("SOKO_STRICT_AT", "0")]);
    assert!(
        ok_off && checked_off.iter().any(|c| c == "b2"),
        "`SOKO_STRICT_AT=0` 必须退回旧行为（`b2` 重新 checked）：{checked_off:?} diags={diags_off:?}"
    );
}

/// ③ **按名字绑定，与书写顺序无关**（Lean `App.lean:753` 走的是 binder 顺序 ✓）。
#[test]
fn named_arguments_bind_in_any_order() {
    let src = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def c1 : Nat := h (β := Nat) (fun (n : Nat) => n) 5
def c2 : Nat := h (α := Nat) (fun (n : Nat) => n) 5
def c3 : Nat := h (β := Nat) (α := Nat) (fun (n : Nat) => n) 5
";
    let (ok, checked, diags) = grade("order", src, &[]);
    assert!(
        ok,
        "命名实参必须与顺序无关：checked={checked:?} diags={diags:?}"
    );
    for name in ["c1", "c2", "c3"] {
        assert!(
            checked.iter().any(|c| c == name),
            "`{name}` 应当 checked；实得 {checked:?} diags={diags:?}"
        );
    }
}

/// ④ **名字写错 ⇒ 判红 + 文案列出真实形参名**（Lean `throwInvalidNamedArg` 同款 ✓）。
#[test]
fn unknown_name_is_rejected_with_the_real_names_listed() {
    let src = "\
def g {α : Type} (x : α) : α := x

def bad : Nat := g (β := Nat) 5
";
    let (ok, _, diags) = grade("unknown", src, &[]);
    assert!(!ok, "名字对不上必须判红；diags={diags:?}");
    let diag = diags
        .iter()
        .find(|d| d.starts_with("elab-invalid-named-argument"))
        .unwrap_or_else(|| panic!("要 `elab-invalid-named-argument`；实得 {diags:?}"));
    for want in ["`α`", "`x`"] {
        assert!(
            diag.contains(want),
            "文案必须列出真实形参名 {want}（Lean 同款「列出可用名」）✗：{diag}"
        );
    }
}

/// ④ 的第二半 —— **照文案做就判绿**（「指令明确」的唯一硬判据 ✓）。
///
/// 为什么必须有 ✗：光断言"文案里有 `α`"会放过一条**写错的**出路 ✗（imp1 那轮就实测
/// 抓到过一条 `@ignores 3`）。这里把文案里**列出的第一个形参名抠出来**粘回去重判 ✓。
#[test]
fn the_name_listed_in_the_message_actually_works() {
    let bad = "\
def g {α : Type} (x : α) : α := x

def bad : Nat := g (β := Nat) 5
";
    let (_, _, diags) = grade("escape", bad, &[]);
    let diag = diags
        .iter()
        .find(|d| d.starts_with("elab-invalid-named-argument"))
        .unwrap_or_else(|| panic!("要 `elab-invalid-named-argument`；实得 {diags:?}"));
    // 文案形如「按位置可用的形参名依次是：`α`、`x`。」⇒ 抠第一个反引号里的名字。
    let after = diag
        .split("形参名依次是：")
        .nth(1)
        .unwrap_or_else(|| panic!("文案里要有一段列出可用形参名 ✗：{diag}"));
    let name = after
        .split('`')
        .nth(1)
        .unwrap_or_else(|| panic!("列出的名字要用反引号包起来 ✗：{after}"));
    let good =
        format!("def g {{α : Type}} (x : α) : α := x\n\ndef fixed : Nat := g ({name} := Nat) 5\n");
    let (ok, checked, diags) = grade("escape2", &good, &[]);
    assert!(
        ok && checked.iter().any(|c| c == "fixed"),
        "**照文案改必须判绿** ✗：抠出来的名字 `{name}` ⇒ checked={checked:?} diags={diags:?}"
    );
}

/// ⑤ **同一个形参给两次 ⇒ 判红**（Lean 4：第二个留在表里 ⇒ `throwInvalidNamedArg` ✓）。
#[test]
fn duplicate_name_is_rejected() {
    let src = "\
def g {α : Type} (x : α) : α := x

def bad : Nat := g (α := Nat) (α := Bool) 5
";
    let (ok, _, diags) = grade("dup", src, &[]);
    assert!(!ok, "同名给两次必须判红；diags={diags:?}");
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("elab-invalid-named-argument")),
        "要 `elab-invalid-named-argument`；实得 {diags:?}"
    );
}
