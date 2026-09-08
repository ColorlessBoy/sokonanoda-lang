//! Every checked-in lesson file must stay valid: parse, elaborate and pass the
//! complete kernel. Adding a lesson is a change to the curriculum, so it must
//! come with a working `.sokonanoda` file (and ideally a golden event test).

use std::process::{Command, Stdio};

fn run_file(path: &str) -> std::process::Output {
    let child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    child.wait_with_output().expect("wait")
}

#[test]
fn every_example_lesson_is_a_valid_sokonanoda_file() {
    let examples = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let mut files: Vec<_> = std::fs::read_dir(examples)
        .expect("read examples dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sokonanoda")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    assert!(!files.is_empty(), "no .sokonanoda files under {examples}");

    for file in files {
        let path = file.path();
        let name = file.file_name().to_string_lossy().into_owned();
        let out = run_file(path.to_str().expect("utf-8 path"));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "lesson {name} failed:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("error["),
            "lesson {name} must not print error lines on stderr:\nstderr:\n{stderr}"
        );

        if name == "fol-basics.sokonanoda" {
            assert!(
                stdout.contains("checked declaration absurd"),
                "fol-basics should check theorem absurd:\n{stdout}"
            );
        }
        if name == "py-fol-core.sokonanoda" {
            let checked = stdout
                .lines()
                .filter(|l| l.starts_with("checked declaration "))
                .count();
            assert!(
                checked >= 20,
                "py-fol-core should check at least 20 declarations, found {checked}:\n{stdout}"
            );
        }

        if name == "lesson-01.sokonanoda" || name == "lesson-02.sokonanoda" {
            assert!(
                stdout.contains("exercise open (fill the sorry)"),
                "lesson {name} should end with an open exercise:\n{stdout}"
            );
        }
        if name == "py-nat.sokonanoda" {
            assert!(
                stdout.contains("add two two => succ (succ (succ (succ zero)))"),
                "nat lesson should reduce add two two:\n{stdout}"
            );
        }
    }
}
