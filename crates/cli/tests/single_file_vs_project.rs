//! 「单文件 vs 项目」的**自动区分**契约（用户 2026-09-18 提问：
//! 「单文件和项目文件编译器能自动区分吗？比如单文件不会找项目配置文件，
//! 能像脚本一样直接跑」）。
//!
//! 规则（实现见 `crates/cli/src/check.rs` 的 `command.is_import()` 分支、
//! `crates/front/src/query/mod.rs`、`crates/lsp/src/lib.rs`）：
//!
//! 1. **触发条件是 `import`，不是清单**：文件里一个 `import` 都没有 ⇒ 走单文件
//!    流水线，**从不发现、从不读 `sokonanoda.toml`**（所以同目录/祖先目录里的清单
//!    哪怕坏掉也与它无关；`--root` / `--no-project` 对它也是空操作）。
//! 2. 有 `import` ⇒ 从**入口文件所在目录**向上找最近清单（止于 `.git`/HOME），
//!    找不到就零配置（模块根 = 入口目录）。清单坏了报 `manifest-invalid`，但仍以
//!    入口目录为根继续编译（错误可见、工作不中断）。
//! 3. **依赖模块自己的清单永远不参与**：模块根只由入口决定；`import Sub.Lib`
//!    直接按名字解析到 `Sub/Lib.sokonanoda`，`Sub/sokonanoda.toml` 是无关文件。
//! 4. stdin 与文件等价（逐字节一致的输出），所以"像脚本一样直接跑"对小文件成立；
//!    只有 stdin **带 `import`** 时才需要一个模块根（`--root`），并给出明确 hint。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-autodetect-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, text).expect("write file");
}

fn run(dir: &Path, args: &[&str], stdin: Option<&str>) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .current_dir(dir)
        .env("SOKONANODA_CACHE_DIR", dir.join(".cache"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    if let Some(input) = stdin {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }
    child.wait_with_output().expect("wait")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// 一个自给自足的合法单文件（没有任何 `import`）。
const SINGLE: &str = "def id : (p : Prop) -> p -> p := fun (p : Prop) => fun (h : p) => h\n\
axiom P : Prop\n\
theorem t : P -> P := fun (h : P) => id P h\n";

/// 坏到解析器会报错的清单文本。
const BROKEN_MANIFEST: &str = "this is not = valid TOML [[[\n";

#[test]
fn a_single_file_never_reads_a_manifest() {
    let dir = tmp_dir("single-manifest");
    write(&dir, "Single.sokonanoda", SINGLE);
    // 同目录（以及祖先目录）放着坏清单：单文件必须完全不理会它。
    write(&dir, "sokonanoda.toml", BROKEN_MANIFEST);

    let out = run(&dir, &["Single.sokonanoda"], None);
    assert!(
        out.status.success(),
        "a file without `import` must not touch the manifest: {}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("manifest"),
        "no manifest diagnostics may appear for a single file: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("checked declaration t"),
        "{}",
        stdout(&out)
    );

    // `--root` / `--no-project` 是项目模式的开关，对单文件是空操作。
    for flag in ["--root", "--no-project"] {
        let args: Vec<&str> = if flag == "--root" {
            vec!["--root", "/definitely/not/here", "Single.sokonanoda"]
        } else {
            vec![flag, "Single.sokonanoda"]
        };
        let out = run(&dir, &args, None);
        assert!(
            out.status.success(),
            "`{flag}` must be a no-op for a single file: {}",
            stderr(&out)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_same_manifest_does_bite_once_the_file_has_an_import() {
    let dir = tmp_dir("project-manifest");
    write(&dir, "Single.sokonanoda", SINGLE);
    write(
        &dir,
        "Main.sokonanoda",
        "import Single\n\ntheorem u : P -> P := fun (h : P) => id P h\n",
    );
    write(&dir, "sokonanoda.toml", BROKEN_MANIFEST);

    // 单文件：照跑不误。
    let single = run(&dir, &["Single.sokonanoda"], None);
    assert!(single.status.success(), "{}", stderr(&single));

    // 同一个目录里的项目入口：清单被读到，坏清单是**可见的错误**（退出码 1），
    // 但仍以入口目录为根把闭包编完（错误不吞掉工作）。
    let project = run(&dir, &["Main.sokonanoda"], None);
    assert!(
        !project.status.success(),
        "broken manifest must be reported"
    );
    assert!(
        stderr(&project).contains("manifest-invalid"),
        "{}",
        stderr(&project)
    );
    assert!(
        stdout(&project).contains("checked declaration u"),
        "the closure still compiles with the entry directory as root: {}",
        stdout(&project)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_dependency_module_never_contributes_its_own_manifest() {
    let dir = tmp_dir("dep-manifest");
    // 入口在根（根有**合法**清单），依赖在子目录、子目录里有**坏**清单：
    // 依赖的清单与模块解析无关（`Sub.Lib` → `Sub/Lib.sokonanoda`）。
    write(&dir, "sokonanoda.toml", "name = \"root\"\n");
    write(&dir, "Sub/sokonanoda.toml", BROKEN_MANIFEST);
    write(&dir, "Sub/Lib.sokonanoda", "axiom Q : Prop\n");
    write(
        &dir,
        "Deep.sokonanoda",
        "import Sub.Lib\n\ntheorem v : Q -> Q := fun (h : Q) => h\n",
    );

    let out = run(&dir, &["Deep.sokonanoda"], None);
    assert!(
        out.status.success(),
        "a dependency's own manifest must be ignored: {}",
        stderr(&out)
    );
    assert!(!stderr(&out).contains("manifest"), "{}", stderr(&out));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zero_config_imports_still_work_and_stdin_matches_a_file() {
    let dir = tmp_dir("zero-config");
    write(&dir, "Single.sokonanoda", SINGLE);
    write(
        &dir,
        "Main.sokonanoda",
        "import Single\n\ntheorem u : P -> P := fun (h : P) => id P h\n",
    );
    // 没有清单：模块根 = 入口目录，直接跑。
    let out = run(&dir, &["Main.sokonanoda"], None);
    assert!(out.status.success(), "{}", stderr(&out));

    // stdin 与文件等价（"像脚本一样直接跑"）：输出逐字节一致。
    let from_file = run(&dir, &["--json", "Single.sokonanoda"], None);
    let from_stdin = run(&dir, &["--json", "-"], Some(SINGLE));
    assert!(from_file.status.success() && from_stdin.status.success());
    assert_eq!(
        stdout(&from_file),
        stdout(&from_stdin),
        "stdin must be indistinguishable from the file"
    );

    // 只有 stdin **带 import** 时才需要模块根，并且要给得出下一步（hint 在 JSON 里）。
    let needs_root = run(&dir, &["--json", "-"], Some("import Single\n"));
    assert!(!needs_root.status.success());
    assert!(
        stdout(&needs_root).contains("import-not-found") && stdout(&needs_root).contains("--root"),
        "stdin + import must explain how to supply a module root: {}",
        stdout(&needs_root)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
