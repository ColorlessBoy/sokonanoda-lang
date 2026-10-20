//! **S3 端到端守卫**（设计 `docs/design/dialect-config.md` §2/§3.2/§4）：
//! `--dialect <strict|loose>` 是**输入面**的搬家 —— 同一个值从环境变量搬到 argv，
//! 判定必须**逐字节不变**。
//!
//! 四条牙（每条都断言用户可见的结果）：
//!   ① `--dialect strict` ≡ `SOKO_STRICT_AT=1`（stdout 逐字节 + 退出码）；
//!   ② `--dialect loose` ≡ 不设任何环境变量；
//!   ③ 旗标**覆盖**环境：`SOKO_STRICT_AT=0 --dialect strict` ⇒ 严格档；
//!   ④ 非法值 ⇒ **退出码 2** + 可读的用法错误（不是 panic / 不是 1）。
//!
//! 夹具取 `implicit_at_alignment.rs` 的 `REPRO` 形状：`b2` 是**唯一**能区分两档的
//! 那一条（loose ⇒ checked，strict ⇒ `elab-implicit-argument-unsolved`）—— 所以下面
//! 先断言"两档真的不同"（牙有咬合力），再断言"旗标与环境逐字节相同"。

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const REPRO: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b1 : Nat := h (fun (n : Nat) => n) 5
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
def b3 : Nat := @h Nat Nat (fun (n : Nat) => n) 5
";

/// 每个用例一份独立夹具（cargo 测试同进程多线程，共用路径会互相覆盖）。
fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("soko-s3-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    let path = dir.join("Probe.sokonanoda");
    std::fs::write(&path, REPRO).expect("write fixture");
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

#[test]
fn dialect_strict_flag_is_byte_identical_to_the_environment() {
    let path = fixture("strict-eq-env");
    let path = path.to_str().expect("utf-8 path");
    let env_strict = run(&["grade", "--json", path], &[("SOKO_STRICT_AT", "1")]);
    let flag_strict = run(&["grade", "--json", "--dialect", "strict", path], &[]);
    let env_off = run(&["grade", "--json", path], &[("SOKO_STRICT_AT", "0")]);

    // 夹具**必须有咬合力**：严格档真的拒了 `b2`，旧读法真的放行（否则逐字节相同
    // 可能只是"两边都坏成一样"✗）。
    assert_ne!(
        env_strict.1, env_off.1,
        "夹具无法区分两档：strict 与 off 的 --json 输出相同（判据没有咬合力）"
    );
    assert_eq!(env_strict.0, 1, "严格档应因 `b2` 被拒而 exit 1");
    assert_eq!(env_off.0, 0, "旧读法下四个声明应全绿");

    assert_eq!(
        flag_strict.0, env_strict.0,
        "--dialect strict 与 SOKO_STRICT_AT=1 的退出码必须相同（stderr={}）",
        flag_strict.2
    );
    assert_eq!(
        flag_strict.1, env_strict.1,
        "--dialect strict 与 SOKO_STRICT_AT=1 的 --json 输出必须**逐字节**相同"
    );

    // 大小写不敏感（文档承诺）：`STRICT` 与 `strict` 同一档。
    let flag_upper = run(&["grade", "--json", "--dialect", "STRICT", path], &[]);
    assert_eq!(
        flag_upper.1, env_strict.1,
        "--dialect 的值大小写不敏感：`STRICT` 必须等同 `strict`"
    );
}

#[test]
fn dialect_loose_flag_is_byte_identical_to_the_default() {
    let path = fixture("loose-eq-default");
    let path = path.to_str().expect("utf-8 path");
    let no_env = run(&["grade", "--json", path], &[]);
    let flag_loose = run(&["grade", "--json", "--dialect", "loose", path], &[]);
    assert_eq!(
        flag_loose.0, no_env.0,
        "--dialect loose 与不设环境的退出码必须相同（stderr={}）",
        flag_loose.2
    );
    assert_eq!(
        flag_loose.1, no_env.1,
        "--dialect loose 与不设任何环境变量的 --json 输出必须**逐字节**相同"
    );
}

#[test]
fn dialect_strict_flag_overrides_the_environment() {
    let path = fixture("strict-overrides-env");
    let path = path.to_str().expect("utf-8 path");
    let env_strict = run(&["grade", "--json", path], &[("SOKO_STRICT_AT", "1")]);
    let flag_over_env_off = run(
        &["grade", "--json", "--dialect", "strict", path],
        &[("SOKO_STRICT_AT", "0")],
    );
    let env_off = run(&["grade", "--json", path], &[("SOKO_STRICT_AT", "0")]);

    // 反向验证：环境=0 时**不**给旗标 ⇒ 旧读法（否则下面的相同可能只是"环境没生效"）。
    assert_ne!(
        env_off.1, env_strict.1,
        "SOKO_STRICT_AT=0 必须真的回到旧读法（否则覆盖判据无意义）"
    );
    assert_eq!(
        flag_over_env_off.0, env_strict.0,
        "--dialect strict 必须覆盖 SOKO_STRICT_AT=0（退出码）"
    );
    assert_eq!(
        flag_over_env_off.1, env_strict.1,
        "--dialect strict 必须覆盖 SOKO_STRICT_AT=0：stderr={}",
        flag_over_env_off.2
    );
}

#[test]
fn unknown_dialect_value_is_a_usage_error_with_a_way_out() {
    let path = fixture("bogus");
    let path = path.to_str().expect("utf-8 path");
    let bogus = run(&["grade", "--dialect", "bogus", path], &[]);
    assert_eq!(
        bogus.0, 2,
        "非法 --dialect ⇒ 用法错误退出码 2（实测 {}，stderr={}）",
        bogus.0, bogus.2
    );
    for needle in ["--dialect", "strict", "loose", "提示"] {
        assert!(
            bogus.2.contains(needle),
            "用法错误必须说清出路（缺 `{needle}`）：{}",
            bogus.2
        );
    }

    // 缺值同样是用法错误（不是"把子命令/文件当成值"或 panic）。
    let missing = run(&["grade", "--dialect"], &[]);
    assert_eq!(missing.0, 2, "缺值也必须是退出码 2：stderr={}", missing.2);
    assert!(
        missing.2.contains("需要一个值"),
        "缺值要说人话：{}",
        missing.2
    );
}

/// `query` 通道也必须认旗标（S3 交付里点名 `query`）：同一份夹具，严格档
/// `query check` 的 `failed` 必须 ≥1 且与 `SOKO_STRICT_AT=1` **逐字节相同**。
#[test]
fn query_check_honors_the_dialect_flag() {
    let path = fixture("query");
    let path = path.to_str().expect("utf-8 path");
    let env_strict = run(
        &["query", "check", "--file", path],
        &[("SOKO_STRICT_AT", "1")],
    );
    let flag_strict = run(
        &["query", "check", "--file", path, "--dialect", "strict"],
        &[],
    );
    let flag_loose = run(
        &["query", "check", "--file", path, "--dialect", "loose"],
        &[],
    );
    assert_eq!(
        flag_strict.1, env_strict.1,
        "`query check --dialect strict` 与 SOKO_STRICT_AT=1 必须逐字节相同（stderr={}）",
        flag_strict.2
    );
    assert_ne!(
        flag_strict.1, flag_loose.1,
        "query 通道的旗标必须真的改变判定（否则只是被静默忽略）"
    );
    let strict: Value = serde_json::from_slice(&flag_strict.1).expect("query returns JSON");
    assert_eq!(
        strict["data"]["failed"].as_array().map(Vec::len),
        Some(1),
        "严格档下 `b2` 必须恰好计入 1 条 failed（实际输出：{strict}）"
    );
}
