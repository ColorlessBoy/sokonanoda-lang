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

/// **C2（2026-10-08）**：与 [`run`] 同，但把**子进程的 cwd** 设成 `cwd` ——
/// 无参 `build`/`rebuild` 的默认根是**当前目录**，判据必须能钉住这一点 ✓。
fn run_in_dir(cache: &Path, args: &[&str], cwd: &Path) -> (i32, Vec<Value>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(args)
        .current_dir(cwd)
        .env("SOKONANODA_CACHE_DIR", cache)
        .env_remove("SOKONANODA_NO_PROJECT_ARTIFACTS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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
    // `soko.artifacts/<条目格式>.r<报告形状版本>`（2026-10-02 值守第 8 单：形状版本进 schema
    // ⇒ 报告形状一变，整个产物目录不认 ✓）。字面量是**故意**的：集成测试盯的是**冻结的
    // 契约串**（front 的 `project::cache::meta_schema()` 是唯一来源 ✓，改它这里必须跟着改 ✓）。
    // ⚠ **r3 → r4**：C3 补口（2026-10-09，`DocumentReport.prints` 在**项目模式**下
    // 从"恒空"变成"真的有"）把 `REPORT_SHAPE` 3 → 4 ⇒ 这里必须同步
    // （C3 的 2 → 3、B1 的 1 → 2 都是同一条规矩 ✓）—— 漏改就是本套件判红 ✗。
    // ⚠ **r4 → r5**：G-102（2026-10-10，`CheckInfo::cmd`/`PrintInfo::cmd` **进序列化**
    // —— 回放后按 `cmd` 去重的增量拼接才认得出同一份报告）把 `REPORT_SHAPE` 4 → 5
    // ⇒ 这里同样必须同步（**实测**：这一版发版预检就是漏了这一步判红 ✗✓）。
    assert_eq!(meta["schema"], "soko.artifacts/2.r5");
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

/// **C2（2026-10-08）**：**无参** `rebuild`（cwd = 模块根）也必须清到模块根产物 ——
/// 否则紧接着的预热**全是 `hit`**，用户看到的是"rebuild 什么都没重编"这个**假动作** ✗
/// （实测复现：`build .` 后无参 `rebuild` 报 `1 hit, 1 compiled` ✗）。
///
/// 根因：`project_roots(&[])` 返回**空集**（只有给了路径才去解模块根），而 `warm`
/// 的默认一直是**当前目录** ⇒ 两条路的默认不一致。修法 = 让 `project_roots` 用
/// **同一个默认**（`PLAN-cli-editor-perf.md` §1 P2 的"残留"）。
///
/// **反向验证**：把 `project_roots` 的默认改回空集 ⇒ 本用例**必须红**
/// （`clean.project == 0` 且 `summary.hit ≥ 1`）—— 已实跑取证 ✓。
#[test]
fn a_no_argument_rebuild_also_clears_the_module_root_artifacts() {
    let root = scratch("rebuild-noargs");
    let cache = scratch("cache-rebuild-noargs");
    project(&root);
    let entry = root.join("Main.sokonanoda");

    // 冷 `build`：写项目产物（判据的前提 —— 有东西可清 ✓）。
    let (code, events) = run(&cache, &["build", "--json", entry.to_str().unwrap()]);
    assert_eq!(code, 0, "冷 build 要成功：{events:?}");
    assert_eq!(
        project_entries(&root).len(),
        1,
        "前提：模块根下必须有 1 条项目产物"
    );
    assert_eq!(summary(&events)["hit"], 0, "前提：第一次是冷编");

    // **无参 `rebuild`**（cwd = 模块根）。
    let (code, events) = run_in_dir(&cache, &["rebuild", "--json"], &root);
    assert_eq!(code, 0, "无参 rebuild 要成功：{events:?}");
    let clean = events
        .iter()
        .find(|event| event["type"] == "build.clean")
        .expect("build.clean");
    assert!(
        clean["project"].as_u64().unwrap_or(0) >= 1,
        "**C2 的正身**：无参 `rebuild` 必须清到模块根产物（`project >= 1`）；\
         现在是 {} ⇒ 只清了全局 ⇒ 紧接着的预热会全是 hit ✗：{clean:?}",
        clean["project"]
    );
    let sum = summary(&events);
    assert_eq!(
        sum["hit"], 0,
        "清干净了 ⇒ 必须**真的重编**（`hit == 0`）；今天这里是 1 ⇒ 假重编 ✗：{sum:?}"
    );
    assert!(
        sum["compiled"].as_u64().unwrap_or(0) >= 1,
        "至少要重编一个文件：{sum:?}"
    );

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

/// **需求 1（2026-10-09 用户实测）的可见层判据**：老产物目录（`meta.json` 的
/// `schema` 停在旧格式）在一次 `build` 之后必须被**升级**，`query project` 才报得出
/// `compiler`。
///
/// **屏幕上的判据**：扩展画的那一行「由编译器 X 写入」只读
/// `soko/project.artifacts.compiler`（`editor/vscode/media/infoview.js`）——它是
/// `null` 时面板就写「由编译器 ? 写入」✗（用户看到的就是这个裸问号）。所以这条
/// **不看 `meta.json` 的字面量**，直接问用户界面问的那条命令（`query project`）✓。
///
/// 现场形状（本地 `course/shared/.sokonanoda/meta.json` 逐字节如此）：`compiler`
/// 已被写侧刷新成当前版本，`schema` 却停在 `soko.artifacts/1` ⇒ 读侧两个消费者都
/// 按 `schema == 当前` 过滤 ⇒ 整目录当不存在。
#[test]
fn a_stale_artifact_schema_is_upgraded_by_the_next_build() {
    let root = scratch("stale-schema");
    let cache = scratch("cache-stale-schema");
    project(&root);
    let entry = root.join("Main.sokonanoda");
    let entry_arg = entry.to_str().unwrap();

    // ① 先有一次正常 build —— 目录与条目都是真的。
    let (code, _) = run(&cache, &["build", "--json", entry_arg]);
    assert_eq!(code, 0);
    let meta_path = root.join(".sokonanoda/meta.json");
    let fresh: Value = serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
    let current_schema = fresh["schema"].clone();
    assert!(
        current_schema.as_str().is_some_and(|s| !s.is_empty()),
        "刚写完的 meta.json 必须有 schema：{fresh}"
    );

    // ② 把 `meta.json` 打回**用户现场那个形状**：schema 旧、compiler 旧。
    //
    //    ⚠ 这一格**不能**用 `query project` 去读"旧 schema ⇒ 不报 compiler"：`query`
    //    自己就是一条会**写产物**的命令（`load_document` 未命中就
    //    `store_if_clean_at`）⇒ 它一边读一边把目录修好了（这本身是修复 ✓，不是判据 ✗）。
    //    "读侧过滤"那半边由 front 的单测钉：
    //    `crates/front/src/query/project.rs::artifacts_snapshot_is_read_only_and_counts_only_entries` ②′
    //    （只读快照，不写盘）。
    let mut stale = fresh.clone();
    stale["schema"] = Value::String("soko.artifacts/1".into());
    stale["compiler"] = Value::String("0.78.0".into());
    std::fs::write(&meta_path, format!("{stale}\n")).unwrap();
    let on_disk: Value = serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
    assert_eq!(
        on_disk["schema"], "soko.artifacts/1",
        "夹具必须真的把目录打回旧 schema：{on_disk}"
    );

    // ③ 再来一次 build ⇒ **写产物那一刻** schema 与版本戳一起升级。
    let (code, events) = run(&cache, &["build", "--json", entry_arg]);
    assert_eq!(code, 0);
    assert_eq!(
        summary(&events)["compiled"],
        1,
        "旧 schema ⇒ 读侧当 miss、重编（宁可重算，绝不误用）：{events:?}"
    );
    let after: Value = serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
    assert_eq!(
        after["schema"], current_schema,
        "写产物必须把 schema 升回**当前**值（= ① 那次写下的；修前它永远停在 1 ✗）：{after}"
    );

    // ④ 用户可见的结果：面板那一行拿得到编译器版本号 ⇒ 不再画「?」。
    let (code, stdout) = run_raw(&cache, &["query", "project", "--file", entry_arg], &[]);
    assert_eq!(code, 0, "{stdout}");
    let answer: Value = serde_json::from_str(&stdout).expect("query JSON");
    assert_eq!(
        answer["data"]["project"]["artifacts"]["compiler"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "升级后面板必须拿到当前编译器版本号（修前这里恒为 null ⇒「?」）：{stdout}"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn every_entry_keeps_its_own_artifact_round_after_round() {
    // **性能回归的判据**（2026-09-25）：产物目录的语义是"**每个入口当前的编译结果**"
    // ⇒ 它天然有界（= 入口文件数），**不该**按条数淘汰 —— 一旦淘汰，像课程门禁那样
    // "反复判同一批文件"的用法就会**互相淘汰刚写下的条目**，命中率崩掉、反复重编 ✗
    // （阶段 B 收尾那次 CI 从 ~15 分钟变成 50+ 分钟就是这个）。
    //
    // 判据：建 34 个入口（> 曾经的 32 条上限），逐个 `build` ⇒
    // ① 34 条产物**一条不少**；② 再跑一轮**全部命中**。
    let root = scratch("retention");
    let cache = scratch("cache-retention");
    std::fs::write(root.join("sokonanoda.toml"), "name = \"many\"\n").unwrap();
    std::fs::write(
        root.join("Lib.sokonanoda"),
        "axiom P : Prop\naxiom proofP : P\n",
    )
    .unwrap();
    let entries: Vec<std::path::PathBuf> = (0..34)
        .map(|i| {
            let path = root.join(format!("U{i}.sokonanoda"));
            std::fs::write(&path, "import Lib\n\ntheorem u : P := proofP\n").unwrap();
            path
        })
        .collect();

    for entry in &entries {
        let (code, events) = run(&cache, &["build", "--json", entry.to_str().unwrap()]);
        assert_eq!(code, 0, "{events:?}");
    }
    let kept = project_entries(&root);
    assert_eq!(
        kept.len(),
        entries.len(),
        "每个入口都该留下自己的产物（不许按条数互相淘汰）"
    );

    let (_, again) = run(&cache, &["build", "--json", entries[0].to_str().unwrap()]);
    assert_eq!(summary(&again)["hit"], 1, "第二轮必须命中：{again:?}");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache);
}

/// **C4（2026-10-08）· 冷开 / 提交链路的结构判据**（规划 §`C4`，P6）。
///
/// 规划给的三条判据里，**①（warm 命中）与 ②（跨进程自足）在本条里量成数字** ✓；
/// ③（版本 bump 的影响）**如实登记为"单测量不了"**（见末尾 ✓）。
///
/// ## 量到的两条（都是实测，不是推测）
///
/// * **① warm 命中**：`build.summary` 的 `hit == files`、`compiled == 0` ✓。
/// * **② 模块根产物**跨**全局缓存**自足：**换一个全新的全局缓存目录**、
///   而模块根产物还在 ⇒ 仍然 `hit == files` ✓（这正是"跨进程/跨机器"那条价值的
///   结构读数 —— 以前只有全局缓存时，换个缓存目录就得整条闭包重编 ✗）。
///
/// ## 顺手钉住的一条 as-built 机制（**负向守卫**）
///
/// `meta.json` 里的 `compiler` / `build_stamp` **只管显示**（Infoview 画「编译器 X」），
/// **不参与命中判定** —— 实测：把两者篡改成 `0.0.0` / 全零之后，下一次 `build`
/// **照样 `hit == 1`** ✓。⇒ 这条守卫防的是"有人以为它是命中键、顺手把它删了/改了语义"
/// ✗（真删了 ⇒ 版本戳不再跟着写产物的编译器走 ⇒ 用户升级后 Infoview 又画旧版本号 ✗，
/// 那条判据在 `crates/front/src/project/cache.rs::the_artifact_stamp_follows_the_writing_compiler` ✓）。
///
/// ## ③ 为什么不在单测里判（**如实登记**）
///
/// 规划③要"记录 **bump 后**首轮打开重编的模块数"。bump = 改 `CARGO_PKG_VERSION`
/// ⇒ 那是**换一份二进制**，单测里做不到（不能在一个进程里伪造 `env!("CARGO_PKG_VERSION")`
/// ✗）。篡改 `meta.json` 的版本戳**模拟不了**它（上面那条负向守卫已经证明：
/// 那个字段不进命中判定 ✓）⇒ 真正的读数是**发版节点**上的：bump 之后第一次
/// `sokonanoda build <课程入口>` 的 `compiled`/`files`。⇒ **留给发版节点量**，
/// 且 **A3（模块级产物）落地后**它应当从"整条闭包"降到"入口模块" ✓ —— 那时这条
/// 注释旁边补上实测数字 ✓。
#[test]
fn module_root_artifacts_are_self_sufficient_across_global_caches() {
    let root = scratch("c4-root");
    project(&root);
    let entry = root.join("Main.sokonanoda");
    let entry_arg = entry.to_str().unwrap();

    // ① 冷开（空缓存 + 空模块根）⇒ 编，不命中。
    let cache_a = scratch("c4-cache-a");
    let (code, cold) = run(&cache_a, &["build", "--json", entry_arg]);
    assert_eq!(code, 0, "{cold:?}");
    assert_eq!(
        summary(&cold)["hit"],
        0,
        "冷开不该命中（夹具前提）：{cold:?}"
    );
    assert_eq!(
        summary(&cold)["files"],
        1,
        "一个入口 ⇒ files = 1（结构读数按**入口**数）：{cold:?}"
    );

    // ② 换一个**全新的全局缓存目录**，模块根产物还在 ⇒ 必须仍然命中 ✓。
    let cache_b = scratch("c4-cache-b");
    let (code, across) = run(&cache_b, &["build", "--json", entry_arg]);
    assert_eq!(code, 0, "{across:?}");
    assert_eq!(
        summary(&across)["hit"],
        summary(&across)["files"],
        "**② 的正身**：模块根产物必须**跨全局缓存自足**（`hit == files`）—— \
         以前只有全局缓存 ⇒ 换个缓存目录就整条闭包重编 ✗：{across:?}"
    );
    assert_eq!(
        summary(&across)["compiled"],
        0,
        "命中就不许再编（`compiled == 0`）：{across:?}"
    );

    // ③ 负向守卫：`meta.json` 的版本戳**不进命中判定**（篡改之后照样命中 ✓）。
    let meta_path = root.join(".sokonanoda/meta.json");
    let raw = std::fs::read_to_string(&meta_path).expect("meta.json readable");
    let mut meta: Value = serde_json::from_str(&raw).expect("meta.json is json");
    assert!(
        meta.get("compiler").and_then(Value::as_str).is_some(),
        "meta.json 必须带 compiler（Infoview 的「编译器 X」就画它）：{raw}"
    );
    meta["compiler"] = Value::String("0.0.0".into());
    meta["build_stamp"] = Value::String("0000000000000000".into());
    std::fs::write(&meta_path, format!("{meta}\n")).expect("write tampered meta");

    let (code, tampered) = run(&cache_b, &["build", "--json", entry_arg]);
    assert_eq!(code, 0, "{tampered:?}");
    assert_eq!(
        summary(&tampered)["hit"],
        summary(&tampered)["files"],
        "**as-built 机制**：版本戳只管显示、**不参与命中** ⇒ 篡改后照样命中 ✓。\
         若这里判红 ⇒ 有人把版本戳接进了命中判定 —— 那会让**每次升级都整库重编**，\
         与 A3 的方向相反 ✗；真要改，先读本测试的文档注释与规划 §`C4`③。{tampered:?}"
    );

    // ④ **夹具自检**（这条断言真的咬得住吗？）：关掉模块根产物这条通道
    // （逃生门 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`）+ 全新全局缓存 ⇒ **必须不命中**
    // ✓ —— 它证明 ② 的 `hit == files` **不是恒真**（实测：`hit: 0, compiled: 1` ✓）。
    // 没有这一臂，"② 命中"可能只是"根本没看产物"或"恒真断言" ✗。
    let cache_c = scratch("c4-cache-c");
    let (code, blind) = run_with_env(
        &cache_c,
        &["build", "--json", entry_arg],
        &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")],
    );
    assert_eq!(code, 0, "{blind:?}");
    assert_ne!(
        summary(&blind)["hit"],
        summary(&blind)["files"],
        "**自检**：关掉模块根产物 ⇒ 全新全局缓存下**不该命中** ⇒ ② 那条断言才有判别力 ✓：{blind:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache_a);
    let _ = std::fs::remove_dir_all(&cache_b);
    let _ = std::fs::remove_dir_all(&cache_c);
}
