//! **HO3b 的进程级判据**（`docs/design/metavar-pattern.md` §4 HO3/§5 回退条件）：
//! `SOKO_META_PATTERN` / `SOKO_META_APPROX` 是**新增解**，**不许**动既有判定面。
//!
//! 三条判据（7 个基线文件 × 4 臂）：
//! 1. **默认档 = 逃生门档逐字节**：不设开关 ≡ `SOKO_META_PATTERN=0`（stdout + stderr
//!    逐字节）—— 这是 HO3b 的硬门（关着时与今天**逐字节相同**）；
//! 2. **判定面不变**：两个新档（`pattern` / `pattern+approx`）的**退出码**与**除
//!    `decl.printed` 之外的每一条事件**逐字节相同**；
//! 3. **`decl.printed` 只许变文本、不许变别的**：事件条数与名字相同 —— pattern 档会
//!    给出**更一般**的解（实测：`unit02` 的 `Eq.subst` motive 从 `Set.mem x` 变成
//!    `fun A' => Set.mem x A'`，设计 §1「必须取最一般解」正是它），判定结论一个字不变。
//!
//! ⚠ **每个 (文件, 臂) 独立缓存目录** + 关掉项目产物：缓存的键里已折了两档的位
//! （`cache::metavar_state`），这里仍按最保守的口径跑，且不往仓库里写产物 ✓。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

mod common;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-meta-pattern-cache-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// 七份基线文件（相对仓库根；与 HO3a 的 `/tmp/ho3-base` 同一批）。
const FILES: &[&str] = &[
    "examples/fol-basics.sokonanoda",
    "examples/lesson-01.sokonanoda",
    "examples/lesson-02.sokonanoda",
    "playground.sokonanoda",
    "examples/py-fol-core.sokonanoda",
    "examples/py-nat.sokonanoda",
    "courses/set-theory/units/I.1/unit02-subsets-empty.sokonanoda",
];

/// 一个臂：`(label, 环境覆盖)`；`default` = 什么都不设。
const ARMS: &[(&str, &[(&str, &str)])] = &[
    ("default", &[]),
    ("off", &[("SOKO_META_PATTERN", "0")]),
    ("pattern", &[("SOKO_META_PATTERN", "1")]),
    (
        "approx",
        &[("SOKO_META_PATTERN", "1"), ("SOKO_META_APPROX", "1")],
    ),
];

/// 跑一次 `--json`：`(stdout 原文, stderr 原文, 退出码)`。
fn run(path: &Path, envs: &[(&str, &str)]) -> (String, String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.args(["--json", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir())
        // 不往仓库的模块根写产物（判定与它无关，只为 hermetic）。
        .env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")
        .env_remove("SOKO_METAVAR")
        .env_remove("SOKO_NOTATION_METAVAR")
        .env_remove("SOKO_META_PATTERN")
        .env_remove("SOKO_META_APPROX");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run --json");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// 事件流里**除 `decl.printed` 之外**的那些行（逐字节；顺序即输出顺序）。
fn verdict_lines(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|line| !line.contains("\"type\":\"decl.printed\""))
        .collect()
}

/// `decl.printed` 那一族：`(name, type)` 列表（文本可以变，名字与条数不许变）。
fn printed_names(stdout: &str) -> Vec<(String, String)> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|e| e["type"] == "decl.printed")
        .map(|e| {
            (
                e["name"].as_str().unwrap_or_default().to_string(),
                e["type"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// **判据 1/2/3**（一个 `#[test]`：7 文件 × 4 臂 = 28 次真进程）。
#[test]
fn meta_pattern_arms_keep_the_seven_baseline_files_identical() {
    let root = common::repo_root();
    let mut total = 0usize;
    for file in FILES {
        let path = root.join(file);
        assert!(path.exists(), "基线文件不存在：{}", path.display());
        let mut got = Vec::new();
        for (label, envs) in ARMS {
            got.push((*label, run(&path, envs)));
        }
        let (_, (default_out, default_err, default_code)) = &got[0];
        // 防"空 == 空"的假绿：默认档必须真的事件流（末尾再核总条数）。
        assert!(
            !default_out.is_empty(),
            "{file}：默认档没有任何事件输出 ⇒ 判据会退化"
        );
        total += default_out.lines().count();
        for (label, (out, err, code)) in got.iter().skip(1) {
            // 判据 1：逃生门档**逐字节**等于默认档（stdout + stderr）。
            if *label == "off" {
                assert_eq!(
                    (out, err, code),
                    (default_out, default_err, default_code),
                    "{file}：`{label}` 臂必须与默认档**逐字节**相同（HO3b 的硬门）"
                );
                continue;
            }
            // 判据 2：判定面（退出码 + 除 `decl.printed` 之外的事件）**逐字节**相同。
            assert_eq!(code, default_code, "{file}：`{label}` 臂的退出码变了");
            assert_eq!(
                verdict_lines(out),
                verdict_lines(default_out),
                "{file}：`{label}` 臂除 `decl.printed` 之外的判定事件变了 ✗ \
                 （近似档只许**新增解**，不许改既有判定面）"
            );
            // 判据 3：`decl.printed` 只许变文本（名字与条数相同）。
            assert_eq!(
                printed_names(out),
                printed_names(default_out),
                "{file}：`{label}` 臂的 `decl.printed` 条数/名字变了"
            );
        }
    }
    assert!(
        total >= 20,
        "七个基线文件合计只有 {total} 条事件 ⇒ 判据退化成「空 == 空」"
    );
}
