//! **SUB-keyword-decl-name 的反向验证判据**（2026-10-12）：
//! `SOKO_ALLOW_KEYWORD_DECL_NAME=1` ⇒ 关键字声明名的闸门整条放开，
//! `def match : Nat := 0` 的**旧行为原样回来**。
//!
//! **为什么这条判据在子进程里、不在 `crates/front` 的单测里**：逃生门读的是
//! `std::env`，而进程环境是**全局**的 —— 在 libtest 里 `set_var` 会与同进程
//! 并行跑的 `parser::tests::keyword_decl_name_is_a_dedicated_parse_error`
//! 抢跑（实测过：同一份代码第一条命令偶发判红、再跑两次又全绿 = 环境竞态 flake ✗）。
//! 这里给子进程**自己的**环境 ⇒ 判定确定、与别的测试零耦合。
//!
//! 判据（三条一起，缺一条就可能是"没真放开"或"放太开"✗）：
//! 1. **闸门默认关**：不设变量 ⇒ `def match` 判红 + 专用码 `parse-keyword-decl-name`；
//! 2. **设了变量 ⇒ 旧行为回来**：`def match : Nat := 0` 单条 exit 0 + `decl.checked`
//!    （这正是改前的读数）；
//! 3. **引用时那条误导诊断也原样回来**：`unexpected-token` + span 落在
//!    **第 3 行**（不是声明那一行）—— 证明放开的是**整条闸**，不是换了个报法。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-kw-hatch-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// 跑一次 `--json`，`hatch` 决定要不要设逃生门。返回 `(exit_ok, stdout)`。
fn grade(src: &str, hatch: bool, cache: &Path) -> (bool, String) {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-kw-hatch-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, src).expect("write canvas");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.args(["--json", path.to_str().expect("utf-8 path")])
        .env("SOKONANODA_CACHE_DIR", cache)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if hatch {
        cmd.env("SOKO_ALLOW_KEYWORD_DECL_NAME", "1");
    } else {
        cmd.env_remove("SOKO_ALLOW_KEYWORD_DECL_NAME");
    }
    let out = cmd.output().expect("run sokonanoda");
    let _ = std::fs::remove_file(&path);
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn the_keyword_decl_name_gate_lifts_with_the_escape_hatch() {
    const SINGLE: &str = "def match : Nat := 0\n";
    const WITH_REF: &str = "def match : Nat := 0\ndef g : Nat := match\n";

    // ① 闸门默认关：单条 `def match` 判红 + 专用码。
    let (ok, out) = grade(SINGLE, false, &cache_dir("single-off"));
    assert!(!ok, "闸门开着时 `def match` 必须判红：{out}");
    assert!(
        out.contains("parse-keyword-decl-name"),
        "必须是专用码，不是通用 unexpected-token：{out}"
    );

    // ② 逃生门开 ⇒ **旧行为原样回来**：单条 exit 0 + `decl.checked`。
    let (ok, out) = grade(SINGLE, true, &cache_dir("single-on"));
    assert!(
        ok,
        "逃生门开着时 `def match` 必须回到旧行为（exit 0）：{out}"
    );
    assert!(
        out.contains("decl.checked"),
        "旧行为会发 decl.checked：{out}"
    );
    assert!(
        !out.contains("parse-keyword-decl-name"),
        "逃生门开着时不许再报新码：{out}"
    );

    // ③ 引用时那条"别处的误导诊断"也原样回来（span 落在第 3 行）。
    let (ok, out) = grade(WITH_REF, true, &cache_dir("ref-on"));
    assert!(!ok, "引用裸关键字名仍然失败（这就是要修的现象）：{out}");
    assert!(out.contains("unexpected-token"), "{out}");
    assert!(
        out.contains("\"line\":3") || out.contains("\"line\": 3"),
        "旧行为的 span 落在第 3 行（文件尾），实际：{out}"
    );
    assert!(
        out.contains("expected an expression, found 输入末尾"),
        "旧行为就是这条误导消息：{out}"
    );
}
