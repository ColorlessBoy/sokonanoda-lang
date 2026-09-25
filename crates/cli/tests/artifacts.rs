//! **T-B5 的判据**：模块根下的 `.sokonanoda/` 产物目录（R-3 前半）。
//!
//! 契约见 `docs/design/project-artifacts.md`。判据全部走**真进程**（spawn CLI）+
//! 隔离的 `SOKONANODA_CACHE_DIR`：缓存行为是"跨进程的磁盘事实"，
//! 只有真跑一遍才算验过（沿用既有纪律，见 `crates/lsp/src/lib.rs` 的
//! `cfg!(test)` 注释与 `docs/gaps/repro/G25-…`）。
//!
//! **屏幕上会多什么**：项目文件第一次 `build`/`grade` 之后，模块根下出现
//! `.sokonanoda/`（`compiled/<key>.json` + `.gitignore` + `meta.json`）——
//! vscode 与 code agent 都在这里取编译后的数据。单文件（无 `import`）不产生它。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-artifacts-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
}

/// 一个最小可编译的项目：`sokonanoda.toml` + 依赖 + 入口（入口必须走闭包路径）。
fn project(root: &Path) {
    std::fs::write(root.join("sokonanoda.toml"), "name = \"demo\"\n").unwrap();
    std::fs::write(root.join("Lib.sokonanoda"), "def lib : Nat := 1\n").unwrap();
    std::fs::write(
        root.join("Main.sokonanoda"),
        "import Lib\n\ndef main : Nat := lib\n",
    )
    .unwrap();
}

fn run(cache: &Path, args: &[&str]) -> (i32, Vec<Value>) {
    run_with_env(cache, args, &[])
}

fn run_with_env(cache: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, Vec<Value>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache)
        // 这些用例测的**就是**项目产物（R-3）⇒ 必须自己控制这个开关：
        // 继承来的 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（例如本地为了绕开受限环境
        // 的文件策略而给 gate 开的逃生门）会把被测功能关掉，让判据变成假红 ✗。
        // 显式 `env_remove` ⇒ 下面的 `env` 参数仍可把它设回来（逃生门那条用例）。
        .env_remove("SOKONANODA_NO_PROJECT_ARTIFACTS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let out = command
        .spawn()
        .expect("spawn sokonanoda")
        .wait_with_output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let events = stdout
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("bad JSON ({e}): {line}")))
        .collect();
    (out.status.code().unwrap_or(-1), events)
}

/// 跑到结束并拿**整段 stdout**（`query` 的输出是**美化过的多行 JSON**，
/// 不能像 `build --json` 那样逐行解析）。
fn run_raw(cache: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(args)
        .env("SOKONANODA_CACHE_DIR", cache)
        // 这些用例测的**就是**项目产物（R-3）⇒ 必须自己控制这个开关：
        // 继承来的 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（例如本地为了绕开受限环境
        // 的文件策略而给 gate 开的逃生门）会把被测功能关掉，让判据变成假红 ✗。
        // 显式 `env_remove` ⇒ 下面的 `env` 参数仍可把它设回来（逃生门那条用例）。
        .env_remove("SOKONANODA_NO_PROJECT_ARTIFACTS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let out = command
        .spawn()
        .expect("spawn sokonanoda")
        .wait_with_output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn summary(events: &[Value]) -> &Value {
    events
        .iter()
        .find(|event| event["type"] == "build.summary")
        .expect("build.summary")
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = read
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    out.sort();
    out
}

fn cache_entries(cache: &Path) -> Vec<PathBuf> {
    entries(&cache.join("compiled"))
}

fn project_entries(root: &Path) -> Vec<PathBuf> {
    entries(&root.join(".sokonanoda/compiled"))
}

#[test]
fn a_project_build_writes_its_artifact_into_the_module_root() {
    let root = scratch("root");
    let cache = scratch("cache");
    project(&root);
    let entry = root.join("Main.sokonanoda");

    let (code, events) = run(&cache, &["build", "--json", entry.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert_eq!(summary(&events)["compiled"], 1);

    // ① 产物落在**模块根**（用户 R-3 的原话："`sokonanoda.toml` 所在的根目录下"）。
    let written = project_entries(&root);
    assert_eq!(written.len(), 1, "模块根下应恰好一条产物：{written:?}");
    // ② 同一个条目格式（与全局缓存逐字节同格式）⇒ 它是一份 `CachedCompile`。
    let payload: Value = serde_json::from_slice(&std::fs::read(&written[0]).unwrap()).unwrap();
    assert!(payload.get("report").is_some(), "条目要有 report");
    assert!(payload.get("project").is_some(), "项目条目要有整份报告");
    // ③ `.gitignore` 自忽略（一行 `*`）⇒ 不脏用户的工作区。
    assert_eq!(
        std::fs::read_to_string(root.join(".sokonanoda/.gitignore")).unwrap(),
        "*\n"
    );
    // ④ `meta.json` 自述 schema/版本（人能读，机器能判）。
    let meta: Value =
        serde_json::from_slice(&std::fs::read(root.join(".sokonanoda/meta.json")).unwrap())
            .unwrap();
    assert_eq!(meta["schema"], "soko.artifacts/1");
    assert!(meta["compiler"].as_str().is_some_and(|v| !v.is_empty()));
    // ⑤ 项目条目**不再**落全局缓存（分工：项目条目只认模块根）。
    assert!(
        cache_entries(&cache).is_empty(),
        "项目条目不该再写全局缓存：{:?}",
        cache_entries(&cache)
    );

    // ⑥ 第二次调用**命中**（用户要的"避免重复计算"）。
    let (_, again) = run(&cache, &["build", "--json", entry.to_str().unwrap()]);
    assert_eq!(summary(&again)["hit"], 1, "第二次必须命中：{again:?}");
    assert_eq!(summary(&again)["compiled"], 0);
    assert_eq!(project_entries(&root).len(), 1, "命中不该再写一条");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_single_file_build_leaves_no_artifacts_directory() {
    let root = scratch("single");
    let cache = scratch("cache-single");
    let file = root.join("Solo.sokonanoda");
    // 无 `import` ⇒ 单文件路径（条目只含内容，留全局缓存才谈得上跨项目共享）。
    std::fs::write(&file, "theorem t : True := trivial\n").unwrap();

    let (code, _) = run(&cache, &["build", "--json", file.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(
        !root.join(".sokonanoda").exists(),
        "单文件不该产生模块根产物目录"
    );
    assert_eq!(cache_entries(&cache).len(), 1, "单文件条目仍在全局缓存");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn the_escape_hatch_restores_the_old_single_store_behaviour() {
    let root = scratch("escape");
    let cache = scratch("cache-escape");
    project(&root);
    let entry = root.join("Main.sokonanoda");

    let (code, events) = run_with_env(
        &cache,
        &["build", "--json", entry.to_str().unwrap()],
        &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")],
    );
    assert_eq!(code, 0);
    assert_eq!(summary(&events)["compiled"], 1);
    assert!(
        !root.join(".sokonanoda").exists(),
        "逃生门开着时**不许**建产物目录（行为要与今天逐字节相同）"
    );
    assert_eq!(cache_entries(&cache).len(), 1, "退回全局缓存");
    let (_, again) = run_with_env(
        &cache,
        &["build", "--json", entry.to_str().unwrap()],
        &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")],
    );
    assert_eq!(summary(&again)["hit"], 1, "逃生门下也要能命中");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn clean_empties_both_stores_and_keeps_the_artifacts_metadata() {
    let root = scratch("clean");
    let cache = scratch("cache-clean");
    project(&root);
    let entry = root.join("Main.sokonanoda");
    run(&cache, &["build", "--json", entry.to_str().unwrap()]);
    // 再让**单文件**条目进全局缓存 ⇒ `--clean` 必须两处都清。
    let solo = root.join("Solo.sokonanoda");
    std::fs::write(&solo, "theorem t : True := trivial\n").unwrap();
    run(&cache, &["build", "--json", solo.to_str().unwrap()]);
    assert_eq!(project_entries(&root).len(), 1);
    assert_eq!(cache_entries(&cache).len(), 1);

    let (code, events) = run(
        &cache,
        &["build", "--clean", "--json", entry.to_str().unwrap()],
    );
    assert_eq!(code, 0);
    let clean = events
        .iter()
        .find(|event| event["type"] == "build.clean")
        .expect("build.clean");
    assert_eq!(clean["project"], 1, "项目产物要清：{clean:?}");
    assert_eq!(clean["global"], 1, "全局缓存也要清：{clean:?}");
    assert_eq!(
        clean["removed"], 2,
        "`removed` = 两处之和（老消费者语义不变）"
    );
    assert!(project_entries(&root).is_empty());
    assert!(cache_entries(&cache).is_empty());
    // 元数据不是条目 ⇒ 不被 `--clean` 删（否则每次都重建，还丢 created 时间）。
    assert!(root.join(".sokonanoda/.gitignore").exists());
    assert!(root.join(".sokonanoda/meta.json").exists());

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_project_entry_is_never_replayed_across_module_roots() {
    // 取证 B 实测的反例：项目摘要里只有**入口绝对路径**、**没有模块根**，所以
    // "同一入口 + 两个内容逐字相同的根"会算出同一个摘要 ⇒ 没有守卫时
    // `build --root rootB` 会直接命中 rootA 的条目，`query project` 报出 rootA 的
    // 根与模块路径（definition/references 会跳到别的根下的文件）✗。
    let base = scratch("cross");
    let cache = scratch("cache-cross");
    let entry_dir = base.join("entry");
    let root_a = base.join("rootA");
    let root_b = base.join("rootB");
    for dir in [&entry_dir, &root_a, &root_b] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(root_a.join("Lib.sokonanoda"), "def lib : Nat := 1\n").unwrap();
    std::fs::write(root_b.join("Lib.sokonanoda"), "def lib : Nat := 1\n").unwrap();
    let entry = entry_dir.join("Main.sokonanoda");
    std::fs::write(&entry, "import Lib\n\ndef main : Nat := lib\n").unwrap();

    let args_a = [
        "build",
        "--json",
        "--root",
        root_a.to_str().unwrap(),
        entry.to_str().unwrap(),
    ];
    let args_b = [
        "build",
        "--json",
        "--root",
        root_b.to_str().unwrap(),
        entry.to_str().unwrap(),
    ];

    let (_, first) = run(&cache, &args_a);
    assert_eq!(summary(&first)["compiled"], 1, "rootA 首次编译");
    // **判据**：rootB 必须**重新编译**（不许跨根回放）。
    let (_, across) = run(&cache, &args_b);
    assert_eq!(
        summary(&across)["hit"],
        0,
        "同一入口 + 不同模块根不许命中（回放会把别的根的文件当成自己的）：{across:?}"
    );
    assert_eq!(summary(&across)["compiled"], 1);
    // 各自的产物落在各自根下；再跑各自那次都要命中。
    assert_eq!(project_entries(&root_a).len(), 1);
    assert_eq!(project_entries(&root_b).len(), 1);
    let (_, hit_a) = run(&cache, &args_a);
    assert_eq!(summary(&hit_a)["hit"], 1, "同根第二次要命中");

    let _ = std::fs::remove_dir_all(&base);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn query_project_lists_the_artifacts_of_its_module_root() {
    // **T-B6 的可见层判据**：产物目录"有什么"要能**查**（`--json` 能列），
    // 而不是让人去 `ls` 一个隐藏目录 —— 这正是用户说的"vscode 和 code agent
    // 都应该在这里取编译后的数据"。
    let root = scratch("query");
    let cache = scratch("cache-query");
    project(&root);
    let entry = root.join("Main.sokonanoda");
    run(&cache, &["build", "--json", entry.to_str().unwrap()]);

    // 注意：`query` 自带 JSON 输出、**不接受 `--json`**（加了会 exit 1），
    // 而且是**美化过的多行 JSON** ⇒ 整段解析。
    let (code, stdout) = run_raw(
        &cache,
        &["query", "project", "--file", entry.to_str().unwrap()],
        &[],
    );
    assert_eq!(code, 0, "{stdout}");
    let answer: Value = serde_json::from_str(&stdout).expect("query prints one JSON object");
    let view = &answer["data"]["project"];
    let artifacts = view
        .get("artifacts")
        .unwrap_or_else(|| panic!("project view must carry `artifacts` (R-3): {view}"));
    assert!(!artifacts.is_null(), "{view}");
    assert!(
        artifacts["entries"].as_u64().unwrap_or(0) >= 1,
        "{artifacts}"
    );
    assert!(artifacts["bytes"].as_u64().unwrap_or(0) > 0, "{artifacts}");
    assert_eq!(
        artifacts["compiler"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "{artifacts}"
    );
    assert!(
        artifacts["dir"]
            .as_str()
            .unwrap_or_default()
            .ends_with(".sokonanoda"),
        "{artifacts}"
    );

    // 逃生门 ⇒ 没有产物目录 ⇒ 字段是 `null`（如实报告，不编一个假的）。
    let escape_root = scratch("query-escape");
    let escape_cache = scratch("cache-query-escape");
    project(&escape_root);
    let escape_entry = escape_root.join("Main.sokonanoda");
    let (escape_code, escape_stdout) = run_raw(
        &escape_cache,
        &["query", "project", "--file", escape_entry.to_str().unwrap()],
        &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")],
    );
    assert_eq!(escape_code, 0, "{escape_stdout}");
    let escape_answer: Value = serde_json::from_str(&escape_stdout).expect("query JSON");
    assert!(
        escape_answer["data"]["project"]["artifacts"].is_null(),
        "the escape hatch must report `null`, not a fabricated snapshot: {escape_stdout}"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
    let _ = std::fs::remove_dir_all(&escape_root);
    let _ = std::fs::remove_dir_all(&escape_cache);
}
