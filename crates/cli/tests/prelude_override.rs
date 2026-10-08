//! **E3（2026-10-08）的端到端判据**：prelude 的**运行时覆盖**（`SOKO_PRELUDE_DIR`）。
//!
//! 为什么必须在**子进程**里测：覆盖是**进程级**的（环境变量只读一次、`OnceLock` 缓存 ✓）
//! ⇒ 同一个测试进程里翻不了档 ✗。
//!
//! 三条（对应规划 §2 E3 的判据 ①③）：
//! * **① 行为改变**：把 L1 里 `axiom True.intro : True` 改成 `: False` ⇒ 同一条探针
//!   （`theorem t : True := True.intro`）**判绿 → 判红** —— 覆盖**真的进了安装** ✓；
//!   ⚠ 覆盖**改的是已有族的声明内容**：往覆盖里**新增**名字**不会**被装进来
//!   （`install_l1_prelude` 只装 `L1_FAMILIES` 认得的命令 ✓ —— 这是覆盖的**边界**，
//!   不是 bug；要新增名字得先把它加进族表 ✓）；
//! * **③ 畸形覆盖**：不 panic、**退出码 2**（用法/环境错 ✓）、stderr 有原因 ✓。

use std::process::{Command, Stdio};

fn tmpdir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("soko-e3-cli-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tmpdir");
    dir
}

/// 跑一次 `grade --json`，返回 `(exit_code, stdout, stderr)`。
fn grade(path: &std::path::Path, prelude_dir: Option<&std::path::Path>) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.arg("--json")
        .arg("--no-project")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = prelude_dir {
        cmd.env("SOKO_PRELUDE_DIR", dir);
    }
    let out = cmd.output().expect("spawn sokonanoda");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// 探针：`True.intro` 的**类型**在覆盖版里被改过 ⇒ 它的判绿/判红就是"覆盖生效了没有" ✓。
const PROBE: &str = "theorem e3_probe : True := True.intro\n";

#[test]
fn a_runtime_prelude_override_changes_behaviour() {
    let dir = tmpdir("probe");
    let file = dir.join("Probe.sokonanoda");
    std::fs::write(&file, PROBE).expect("write probe");

    // 不覆盖 ⇒ 内置 `True.intro : True` ⇒ 探针判绿 ✓。
    let (code, out, err) = grade(&file, None);
    assert_eq!(code, 0, "内置 prelude 下探针必须判绿 ✓\nstdout:\n{out}\nstderr:\n{err}");

    // 覆盖 L1：只把 `axiom True.intro : True` 改成 `: False`（其余逐字照抄 ✓）
    // ⇒ 同一条探针**判红**（类型不匹配）—— 覆盖**真的进了安装** ✓。
    let prelude = tmpdir("probe-prelude");
    let builtin_l1 = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("prelude")
            .join("L1.sokonanoda"),
    )
    .expect("read prelude/L1.sokonanoda");
    // ⚠ 翻转必须**装得上**：写成 `False`（后面族才有的名字）= **前向引用** ⇒ 会被
    // E3 的试装拦成"覆盖不生效"（那是判据 ③ 的场景 ✓，不是 ①）。这里用同一族里
    // 已经装好的 `True` 组成 `True -> True` ⇒ 装得上、但探针的类型对不上 ✓。
    let flipped = builtin_l1.replacen(
        "axiom True.intro : True",
        "axiom True.intro : True -> True",
        1,
    );
    assert_ne!(flipped, builtin_l1, "夹具前提：L1 里要有 `axiom True.intro : True`");
    std::fs::write(prelude.join("L1.sokonanoda"), &flipped).expect("write override");

    let (code, out, err) = grade(&file, Some(&prelude));
    assert_eq!(
        code, 1,
        "覆盖把 `True.intro` 的类型改成 `False` ⇒ 探针必须判红 ✓（覆盖没生效才会判绿 ✗）\n         stdout:\n{out}\nstderr:\n{err}"
    );

    // 只覆盖了 L1 ⇒ `Eq` 仍走内置（`Eq.refl` 可用）✓。
    let eq_probe = dir.join("EqProbe.sokonanoda");
    std::fs::write(&eq_probe, "theorem eq_probe : Eq.{1} Prop True True := Eq.refl.{1} Prop True\n")
        .expect("write eq probe");
    let (code, out, err) = grade(&eq_probe, Some(&prelude));
    assert_eq!(
        code, 0,
        "没覆盖的那两份（Eq/Quot）仍用内置 ⇒ `Eq.refl` 必须可用 ✓\nstdout:\n{out}\nstderr:\n{err}"
    );
}

#[test]
fn a_malformed_override_exits_two_without_panicking() {
    let dir = tmpdir("bad");
    let file = dir.join("Probe.sokonanoda");
    std::fs::write(&file, "theorem t : True := True.intro\n").expect("write probe");

    let prelude = tmpdir("bad-prelude");
    std::fs::write(prelude.join("L1.sokonanoda"), "theorem oops : : :\n").expect("write bad");

    let (code, _out, err) = grade(&file, Some(&prelude));
    assert_eq!(code, 2, "畸形覆盖 = 用法/环境错 ⇒ 退出码 2 ✓（不是 panic ✗）\nstderr:\n{err}");
    assert!(
        err.contains("解析失败") && err.contains("L1.sokonanoda"),
        "stderr 要说清是哪一份、为什么 ✗：\n{err}"
    );
    assert!(
        !err.contains("panicked"),
        "**不许** panic（`install_*_prelude` 的 `expect` 不能被覆盖触发 ✗）：\n{err}"
    );
    // 诊断类子命令**不拦**（它们正是用来排查环境的 ✓）。
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("version")
        .env("SOKO_PRELUDE_DIR", &prelude)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn version");
    assert_eq!(
        out.status.code(),
        Some(0),
        "`version` 不该被畸形覆盖拦住（诊断通道要能用 ✓）"
    );
}
