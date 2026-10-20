//! **S4 端到端守卫**（设计 `docs/design/dialect-config.md` §2/§4）：方言的两个新来源
//! —— **项目清单**（`sokonanoda.toml` 的 `[dialect] strict_implicit`）与**文件指令**
//! （`-- sokonanoda:dialect strict|loose`）—— 以及**五档优先级链**：
//!
//! `--dialect`(1) > `SOKO_STRICT_AT`(2) > 编辑器(3，S5) > 文件指令(4) > 清单(5) > 默认 `loose`。
//!
//! 五条牙（每条都断言**用户可见的结果**：退出码 + `--json` 字节）：
//!   ① 清单 strict ⇒ 严格档，且与 `SOKO_STRICT_AT=1` 的 `--json` **逐字节相同**；
//!   ② 文件指令 strict ⇒ 严格档（同样与环境的严格档逐字节相同）；
//!   ③ 清单 loose + 指令 strict ⇒ **指令赢**（反过来也成立：清单 strict + 指令 loose）；
//!   ④ 旗标 / 环境**盖住**清单（`--dialect loose` 盖 `strict_implicit = true`）；
//!   ⑤ 谁都不表态 ⇒ 默认 `loose`（0.95.0 的行为，逐字节不变）。
//!
//! 另加两条**闭包**牙：两个模块指令不一致 ⇒ `import-dialect-conflict`（默认不通融）；
//! 一致 ⇒ 一条都不报。夹具取 `implicit_at_alignment.rs` 的 `REPRO` 形状：`b2` 是
//! **唯一**能区分两档的声明（loose ⇒ checked，strict ⇒ `elab-implicit-argument-unsolved`）
//! ⇒ 每条断言之前都能先证明"两档真的不同"（判据有咬合力）。

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const REPRO: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b1 : Nat := h (fun (n : Nat) => n) 5
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5
";

const STRICT_CODE: &str = "elab-implicit-argument-unsolved";
const CONFLICT_CODE: &str = "import-dialect-conflict";

/// 每个用例一份独立目录（cargo 测试同进程多线程，共用路径会互相覆盖）。
fn fixture_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("soko-s4-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    dir
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write fixture");
    path
}

/// 跑一次 CLI，返回 `(exit code, stdout 原始字节, stderr)`。
///
/// **测试环境隔离**：先摘掉继承来的 `SOKO_STRICT_AT`（否则"默认档"量到的是开发机
/// 的环境值，判据随环境漂移）；`SOKO_KERNEL_ARG_PP` 同族（见
/// `implicit_at_alignment.rs` 的注释）。
fn run(args: &[&str], envs: &[(&str, &str)]) -> (i32, Vec<u8>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.env_remove("SOKO_STRICT_AT");
    cmd.env_remove("SOKO_KERNEL_ARG_PP");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let out: Output = cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda");
    (
        out.status.code().expect("exit code"),
        out.stdout,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn grade(json: bool, path: &Path, extra: &[&str], envs: &[(&str, &str)]) -> (i32, Vec<u8>, String) {
    let path = path.to_str().expect("utf-8 path");
    let mut args: Vec<&str> = vec!["grade"];
    if json {
        args.push("--json");
    }
    args.extend_from_slice(extra);
    args.push(path);
    run(&args, envs)
}

fn occurrences(bytes: &[u8], needle: &str) -> usize {
    String::from_utf8_lossy(bytes).matches(needle).count()
}

/// 清单 `[dialect] strict_implicit = true` ⇒ 严格档，且与 `SOKO_STRICT_AT=1` **逐字节相同**。
#[test]
fn the_manifest_turns_on_the_strict_dialect() {
    let dir = fixture_dir("manifest");
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = true\n",
    );
    let probe = write(&dir, "Probe.sokonanoda", REPRO);

    let manifest_only = grade(true, &probe, &[], &[]);
    let env_strict = grade(true, &probe, &[], &[("SOKO_STRICT_AT", "1")]);
    let flag_loose = grade(true, &probe, &["--dialect", "loose"], &[]);

    // 夹具必须有咬合力 + 清单真的进了严格档。
    assert_eq!(
        manifest_only.0, 1,
        "清单 strict 应拒 `b2`（stderr={}）",
        manifest_only.2
    );
    assert_eq!(occurrences(&manifest_only.1, STRICT_CODE), 1);
    assert_eq!(flag_loose.0, 0, "旧读法下四个声明应全绿");
    assert_ne!(
        manifest_only.1, flag_loose.1,
        "夹具无法区分两档：清单 strict 与 loose 的 --json 相同（判据没有咬合力）"
    );
    assert_eq!(
        manifest_only.1, env_strict.1,
        "清单 strict 与 SOKO_STRICT_AT=1 的 --json 必须**逐字节**相同"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// 文件指令 `-- sokonanoda:dialect strict` ⇒ 严格档（同样与环境的严格档逐字节相同）。
#[test]
fn a_file_directive_turns_on_the_strict_dialect() {
    let dir = fixture_dir("directive");
    let with_directive = format!("-- sokonanoda:dialect strict\n{REPRO}");
    let probe = write(&dir, "Probe.sokonanoda", &with_directive);

    let directive_only = grade(true, &probe, &[], &[]);
    let env_strict = grade(true, &probe, &[], &[("SOKO_STRICT_AT", "1")]);
    let flag_loose = grade(true, &probe, &["--dialect", "loose"], &[]);

    assert_eq!(
        directive_only.0, 1,
        "指令 strict 应拒 `b2`（stderr={}）",
        directive_only.2
    );
    assert_eq!(occurrences(&directive_only.1, STRICT_CODE), 1);
    assert_eq!(
        directive_only.1, env_strict.1,
        "指令 strict 与环境 strict 必须逐字节相同"
    );
    assert_ne!(
        directive_only.1, flag_loose.1,
        "旗标 loose 必须真的盖住文件指令"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **细粒度优先**：清单（第 5 档）与文件指令（第 4 档）冲突时**指令赢**，两个方向都成立。
#[test]
fn the_file_directive_wins_over_the_manifest() {
    let dir = fixture_dir("directive-vs-manifest");
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = false\n",
    );
    let probe = write(
        &dir,
        "Probe.sokonanoda",
        &format!("-- sokonanoda:dialect strict\n{REPRO}"),
    );
    let env_strict = grade(true, &probe, &[], &[("SOKO_STRICT_AT", "1")]);
    let from_directive = grade(true, &probe, &[], &[]);
    assert_eq!(
        from_directive.0, 1,
        "指令 strict 必须盖住清单 loose（stderr={}）",
        from_directive.2
    );
    assert_eq!(
        from_directive.1, env_strict.1,
        "清单 loose + 指令 strict 的结果必须与严格档逐字节相同"
    );

    // 反向：清单 strict + 指令 loose ⇒ 指令赢（宽松）。
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = true\n",
    );
    let probe = write(
        &dir,
        "Probe.sokonanoda",
        &format!("-- sokonanoda:dialect loose\n{REPRO}"),
    );
    let from_directive = grade(true, &probe, &[], &[]);
    assert_eq!(
        from_directive.0, 0,
        "指令 loose 必须盖住清单 strict（stderr={}）",
        from_directive.2
    );
    assert_eq!(occurrences(&from_directive.1, STRICT_CODE), 0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// 旗标（第 1 档）与环境（第 2 档）**盖住清单**；反过来不行。
#[test]
fn the_flag_and_the_environment_override_the_manifest() {
    let dir = fixture_dir("flag-over-manifest");
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = true\n",
    );
    let probe = write(&dir, "Probe.sokonanoda", REPRO);
    let env_strict = grade(true, &probe, &[], &[("SOKO_STRICT_AT", "1")]);

    // `--dialect loose` 必须盖住清单的 strict ⇒ 与"不设任何来源"的旧读法逐字节相同。
    let flag_loose = grade(true, &probe, &["--dialect", "loose"], &[]);
    assert_eq!(
        flag_loose.0, 0,
        "旗标 loose 必须盖住清单 strict（stderr={}）",
        flag_loose.2
    );
    assert_eq!(occurrences(&flag_loose.1, STRICT_CODE), 0);

    // 环境 `SOKO_STRICT_AT=0` 同样盖住清单 strict。
    let env_loose = grade(true, &probe, &[], &[("SOKO_STRICT_AT", "0")]);
    assert_eq!(
        env_loose.0, 0,
        "SOKO_STRICT_AT=0 必须盖住清单 strict（stderr={}）",
        env_loose.2
    );
    assert_eq!(env_loose.1, flag_loose.1);

    // 反向：清单 loose 时环境仍能开严格档（第 2 档 > 第 5 档）。
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = false\n",
    );
    let env_strict_over_loose = grade(true, &probe, &[], &[("SOKO_STRICT_AT", "1")]);
    assert_eq!(env_strict_over_loose.0, 1, "环境 strict 必须盖住清单 loose");
    assert_eq!(env_strict_over_loose.1, env_strict.1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// **默认不许变**：清单里没有 `[dialect]`、文件里没有指令、环境没设 ⇒ 旧读法
/// （与显式 `--dialect loose` 逐字节相同）。
#[test]
fn no_source_keeps_the_default_loose() {
    let dir = fixture_dir("default");
    // 清单存在但**不表态**：它仍然是一个项目根，而方言保持默认。
    write(&dir, "sokonanoda.toml", "name = \"p\"\n");
    let probe = write(&dir, "Probe.sokonanoda", REPRO);
    let default = grade(true, &probe, &[], &[]);
    let flag_loose = grade(true, &probe, &["--dialect", "loose"], &[]);
    assert_eq!(
        default.0, 0,
        "不设任何来源 ⇒ 旧读法（stderr={}）",
        default.2
    );
    assert_eq!(
        default.1, flag_loose.1,
        "不设任何来源的 --json 必须与显式 `--dialect loose` 逐字节相同"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 闭包内两个模块的方言指令**不一致** ⇒ `import-dialect-conflict`（默认不通融）；
/// 一致 ⇒ 一条都不报。
#[test]
fn conflicting_closures_get_the_dialect_conflict_diagnostic() {
    let dir = fixture_dir("closure");
    write(
        &dir,
        "Lib.sokonanoda",
        "-- sokonanoda:dialect loose\ndef lib : Nat := 2\n",
    );
    let main = write(
        &dir,
        "Main.sokonanoda",
        "-- sokonanoda:dialect strict\nimport Lib\n\ndef x : Nat := lib\n",
    );

    let conflict = grade(true, &main, &[], &[]);
    assert_eq!(conflict.0, 1, "指令不一致必须判红（stderr={}）", conflict.2);
    assert!(
        occurrences(&conflict.1, CONFLICT_CODE) >= 1,
        "`--json` 里必须有 {CONFLICT_CODE}：{}",
        String::from_utf8_lossy(&conflict.1)
    );
    let line = String::from_utf8_lossy(&conflict.1)
        .lines()
        .find(|line| line.contains(CONFLICT_CODE))
        .expect("diagnostic line")
        .to_string();
    let diagnostic: Value = serde_json::from_str(&line).expect("diagnostic is JSON");
    assert_eq!(diagnostic["stage"], "import");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|m| m.contains("Lib")),
        "诊断要点名出问题的模块：{diagnostic}"
    );

    // 一致 ⇒ 不报（而且这份闭包本身是绿的）。
    write(
        &dir,
        "Lib.sokonanoda",
        "-- sokonanoda:dialect strict\ndef lib : Nat := 2\n",
    );
    let consistent = grade(true, &main, &[], &[]);
    assert_eq!(occurrences(&consistent.1, CONFLICT_CODE), 0, "一致时不许报");
    assert_eq!(
        consistent.0, 0,
        "一致时闭包应判绿（stderr={}）",
        consistent.2
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `query` 通道也认这两个来源（S4 的接线点与 `grade` 同一份合并函数）：
/// 同一份夹具在文件指令 strict 下 `failed` 恰好 1 条（`b2`）。
#[test]
fn query_check_honors_the_file_directive_and_the_manifest() {
    let dir = fixture_dir("query");
    let probe = write(
        &dir,
        "Probe.sokonanoda",
        &format!("-- sokonanoda:dialect strict\n{REPRO}"),
    );
    let probe = probe.to_str().expect("utf-8 path");
    let with_directive = run(&["query", "check", "--file", probe], &[]);
    let env_strict = run(
        &["query", "check", "--file", probe],
        &[("SOKO_STRICT_AT", "1")],
    );
    assert_eq!(
        with_directive.1, env_strict.1,
        "`query check` 的文件指令必须与 SOKO_STRICT_AT=1 逐字节相同（stderr={}）",
        with_directive.2
    );
    let strict: Value = serde_json::from_slice(&with_directive.1).expect("query returns JSON");
    assert_eq!(
        strict["data"]["failed"].as_array().map(Vec::len),
        Some(1),
        "文件指令 strict 下 `b2` 必须恰好计入 1 条 failed（实际：{strict}）"
    );

    // 清单这一档在 `query` 里同样生效。
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = true\n",
    );
    let plain = write(&dir, "Plain.sokonanoda", REPRO);
    let plain = plain.to_str().expect("utf-8 path");
    let from_manifest = run(&["query", "check", "--file", plain], &[]);
    let strict: Value = serde_json::from_slice(&from_manifest.1).expect("query returns JSON");
    assert_eq!(
        strict["data"]["failed"].as_array().map(Vec::len),
        Some(1),
        "清单 strict 下 `b2` 必须恰好计入 1 条 failed（实际：{strict}）"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **`--no-project` 必须把清单那一档关掉，但**文件指令照算**（2026-10-20 主会话验收时发现的洞）。
///
/// 起因：`--no-project` 从 0.6x 起就写作「忽略 `sokonanoda.toml`」（`AGENTS.md` 的 CLI 表），
/// 但它此前只影响**模块根规划**；`[dialect]` 让清单第一次有了**判定**语义 ⇒ 这条洞才显形：
/// `env::grade` 把 `no_project` 硬编码成 `false`，旗标被静默吞掉。
#[test]
fn no_project_ignores_the_manifest_but_keeps_the_file_directive() {
    let dir = fixture_dir("no-project");
    write(
        &dir,
        "sokonanoda.toml",
        "[dialect]\nstrict_implicit = true\n",
    );
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).expect("sub");
    let probe = write(&sub, "Probe.sokonanoda", REPRO);
    let with_directive = write(
        &sub,
        "Directive.sokonanoda",
        &format!("-- sokonanoda:dialect strict\n{REPRO}"),
    );

    let inherited = grade(true, &probe, &[], &[]);
    let no_project = grade(true, &probe, &["--no-project"], &[]);
    let directive = grade(true, &with_directive, &["--no-project"], &[]);

    assert_eq!(inherited.0, 1, "默认应吃清单 ⇒ 严格档");
    assert_eq!(
        no_project.0, 0,
        "`--no-project` 必须忽略清单（否则清单在用户明确拒绝项目时仍在生效 ✗）：stderr={}",
        no_project.2
    );
    assert_eq!(
        directive.0, 1,
        "`--no-project` 只关**清单**那一档 —— 文件指令是文件自己的，照算 ✓"
    );
    assert_eq!(occurrences(&directive.1, STRICT_CODE), 1);

    let _ = std::fs::remove_dir_all(&dir);
}
