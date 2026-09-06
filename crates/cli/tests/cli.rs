use std::io::Write;
use std::process::{Command, Stdio};

fn run(input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

#[test]
fn cli_checks_a_valid_file_via_stdin() {
    let out = run("def id : Prop → Prop := fun (x : Prop) => x\n\
         #check id\n");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration id"));
    assert!(stdout.contains("#check : Prop → Prop"));
}

#[test]
fn cli_rejects_a_bad_declaration() {
    let out = run("def bad : Prop → Type := fun (x : Prop) => x\n");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("error:"));
}

#[test]
fn cli_reports_parse_errors_with_positions() {
    let out = run("def broken : Prop :=\n");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("2:1: error:"), "stderr: {stderr}");
}

#[test]
fn cli_checks_nat_and_reduces_addition() {
    let out = run(
        "def two : Nat := 1 + 1\n\
         #reduce 1 + 2\n",
    );
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("checked declaration two"));
    assert!(stdout.contains("#reduce => 3"), "stdout: {stdout}");
}
