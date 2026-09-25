//! 项目层（`import` 闭包）**CLI 端到端**性能哨兵 + 实测留档（I16 例行化）。
//!
//! 这里量的是用户/agent 真实的一条命令：进程启动 + 项目闭包编译（或缓存命中）。
//! 与 `crates/front/tests/perf_project.rs`（纯前端口径）互补：那个量的是环节成本，
//! 这个量的是**从命令行到结果**的成本（含 spawn 与磁盘缓存）。
//!
//! 每例打印 `PERF project cli …`（人读）与 `PERFJSON {…}`（`scripts/perf-ledger.sh`
//! 采集进 `docs/perf/ledger.jsonl`）。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-perf-project-{tag}-{}",
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

/// `Lib0 ← Lib1 ← … ← Main`，每个模块 `decls` 条已证声明（教学规模）。
fn gen_project(dir: &Path, modules: usize, decls: usize) {
    for index in 0..modules {
        let mut text = String::new();
        if index > 0 {
            text.push_str(&format!("import Lib{}\n\n", index - 1));
        }
        if index == 0 {
            text.push_str("axiom P : Prop\naxiom proofP : P\n");
        }
        for i in 0..decls {
            text.push_str(&format!("theorem lib{index}_s{i} : P := proofP\n"));
        }
        write(dir, &format!("Lib{index}.sokonanoda"), &text);
    }
    let mut entry = format!("import Lib{}\n\n", modules - 1);
    for i in 0..decls {
        entry.push_str(&format!("theorem main_s{i} : P := proofP\n"));
    }
    write(dir, "Main.sokonanoda", &entry);
}

/// 打掉"新构建二进制的第一次 spawn"代价。
///
/// macOS 上刚构建出来的二进制第一次执行要付代码签名校验/页缓存代价——本机实测
/// **~425ms**（`--version` 也要），与编译毫无关系。不预热就会把这份一次性成本
/// 记进"冷跑"，基线直接失真（第一次写这个测试时就踩了：cold 527ms vs 真实 17ms）。
fn warm_up(dir: &Path, cache: &Path) {
    let _ = run_timed(dir, cache, &["--version"]);
}

/// 固定缓存目录（`SOKONANODA_CACHE_DIR`），观察冷/热与失效必须复用同一份。
fn run_timed(dir: &Path, cache: &Path, args: &[&str]) -> (Duration, std::process::Output) {
    let started = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .current_dir(dir)
        .env("SOKONANODA_CACHE_DIR", cache)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda");
    (started.elapsed(), out)
}

fn ms(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

fn perf_json(value: serde_json::Value) {
    println!("PERFJSON {value}");
}

#[test]
fn project_cli_cold_and_warm_costs_are_recorded() {
    let dir = tmp_dir("cold-warm");
    let cache = dir.join(".cache");
    gen_project(&dir, 3, 12);
    warm_up(&dir, &cache); // 先付掉"首次执行"的一次性成本

    // 冷跑：整条闭包过内核。
    let (cold, out) = run_timed(&dir, &cache, &["Main.sokonanoda"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // 热跑：命中闭包摘要缓存，跳过内核。
    let (warm, out) = run_timed(&dir, &cache, &["Main.sokonanoda"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // 再热一次取最小值，削掉调度噪声（记录的是"最好的热跑"）。
    let (warm_best, _) = run_timed(&dir, &cache, &["Main.sokonanoda"]);
    let warm_best = warm.min(warm_best);

    println!(
        "PERF project cli: cold {:.1}ms · warm {:.1}ms (3 modules × 12 decls)",
        ms(cold),
        ms(warm_best)
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "cli-project",
        "case": "cold_warm_check",
        "modules": 3,
        "decls_per_module": 12,
        "cold_ms": ms(cold),
        "warm_ms": ms(warm_best),
    }));
    assert!(
        warm_best < cold,
        "the cache must beat a cold closure compile (cold {:.1}ms, warm {:.1}ms)",
        ms(cold),
        ms(warm_best)
    );
    assert!(
        ms(warm_best) < 300.0,
        "warm run took {:.1}ms",
        ms(warm_best)
    );

    // 依赖改动 ⇒ 摘要变 ⇒ 必 miss（成本回到冷跑量级），且结果正确。
    let dep = dir.join("Lib0.sokonanoda");
    let mut text = std::fs::read_to_string(&dep).expect("read dep");
    text.push_str("theorem extra : P := proofP\n");
    std::fs::write(&dep, text).expect("write dep");
    let (after_edit, out) = run_timed(&dir, &cache, &["Main.sokonanoda"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "cli-project",
        "case": "dependency_edit_cache_miss",
        "modules": 3,
        "decls_per_module": 12,
        "after_dependency_edit_ms": ms(after_edit),
    }));
    println!(
        "PERF project cli: after a dependency edit {:.1}ms (cache must miss)",
        ms(after_edit)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_cli_query_and_build_costs_are_recorded() {
    let dir = tmp_dir("query-build");
    let cache = dir.join(".cache");
    gen_project(&dir, 3, 12);
    warm_up(&dir, &cache); // 同上：不预热就不是"冷跑"而是"首次执行 + 冷跑"

    // `query check`：一个 JSON 对象（agent 的常规入口）。
    let (query_cold, out) = run_timed(
        &dir,
        &cache,
        &["query", "check", "--file", "Main.sokonanoda"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("query prints one JSON object");
    assert_eq!(body["ok"], serde_json::json!(true), "{body}");

    // `build`：暖缓存（按 DAG 顺序编一遍）。
    let (build, out) = run_timed(&dir, &cache, &["build", "."]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // build 之后 `query` 必命中。
    let (query_warm, out) = run_timed(
        &dir,
        &cache,
        &["query", "check", "--file", "Main.sokonanoda"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    println!(
        "PERF project cli: query cold {:.1}ms · build {:.1}ms · query warm {:.1}ms",
        ms(query_cold),
        ms(build),
        ms(query_warm)
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "cli-project",
        "case": "query_and_build",
        "modules": 3,
        "decls_per_module": 12,
        "query_cold_ms": ms(query_cold),
        "build_ms": ms(build),
        "query_warm_ms": ms(query_warm),
    }));
    assert!(
        ms(query_warm) < ms(query_cold),
        "query after build must hit the cache (cold {:.1}ms, warm {:.1}ms)",
        ms(query_cold),
        ms(query_warm)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **T-B6 的数字**（R-3）：同一模块连跑两次 `build`，第二次必须**显著更快**，
/// 并把产物目录的事实一起记进台账（`scripts/perf-ledger.sh` 收 `PERFJSON`）。
///
/// 为什么用 `build` 而不是 `check`：`build` 是用户/agent 明确"预热"的那条命令
/// （编辑器 `alt+b` 也走它），R-3 的承诺就是"第二次调用命中、不再重复计算"。
/// 哨兵同时断言**产物落在模块根**（否则这条数字量的就不是 R-3 的路径了）。
#[test]
fn project_build_hit_is_far_cheaper_than_a_cold_build() {
    let dir = tmp_dir("build-cold-warm");
    let cache = dir.join(".cache");
    gen_project(&dir, 3, 12);
    warm_up(&dir, &cache);
    // 冷跑前清干净（本机可能残留上一次的产物）：`tmp_dir` 已经重建过目录，
    // 这里再显式删一次 `.sokonanoda`，让"冷"是真的冷。
    let _ = std::fs::remove_dir_all(dir.join(".sokonanoda"));

    let (cold, out) = run_timed(&dir, &cache, &["build", "Main.sokonanoda"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // **产物必须落在模块根**（R-3 的可见结果）——否则这条数字与 R-3 无关。
    let compiled = dir.join(".sokonanoda/compiled");
    assert!(
        compiled.is_dir(),
        "R-3: project artifacts must live in <module root>/.sokonanoda/"
    );
    let (warm, out) = run_timed(&dir, &cache, &["build", "Main.sokonanoda"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (warm_again, _) = run_timed(&dir, &cache, &["build", "Main.sokonanoda"]);
    let warm_best = warm.min(warm_again);
    let artifacts = std::fs::read_dir(&compiled)
        .map(|read| {
            read.flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
                .count()
        })
        .unwrap_or(0);
    assert!(artifacts >= 1, "the artifacts directory must hold entries");

    println!(
        "PERF project cli build: cold {:.1}ms · warm {:.1}ms ({artifacts} artifact(s) in <root>/.sokonanoda)",
        ms(cold),
        ms(warm_best)
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "cli-project",
        "case": "build_cold_warm_artifacts",
        "modules": 3,
        "decls_per_module": 12,
        "cold_ms": ms(cold),
        "warm_ms": ms(warm_best),
        "artifacts": artifacts,
    }));
    assert!(
        ms(warm_best) * 2.0 < ms(cold),
        "the second `build` must be far cheaper than a cold one (cold {:.1}ms, warm {:.1}ms)",
        ms(cold),
        ms(warm_best)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
