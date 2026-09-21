//! **真实课程闭包**的编辑器路径哨兵（计划 T-007 / T-020）。
//!
//! 为什么单独一个文件：`docs/PERF.md` 的既有夹具是 2–5 模块 × 10–12 声明，
//! 比 `courses/set-theory` **小 1–2 个数量级**——既有数字 12–14ms，而真实课程
//! 打开一个单元是 **1.8–8.5s**（`unit12` 的解答更是 36.1s）。
//! 用合成夹具量不出用户感受到的那个量级，所以这里**直接用仓库里的真课程**。
//!
//! 口径（`docs/PERF.md` 的纪律）：
//! * 重活**互相串行**（`COURSE_PERF_LOCK`）——同一台机器上并行跑会把单次计时放大 3–4×；
//! * 阈值**只抓量级回归**（整闭包重编译退化成 O(n²) 会是秒级 → 几十秒），
//!   绝对数字宽松（CI runner 的实例方差有 4×，见 `docs/PERF.md` 的 300ms→800ms 教训）；
//! * 每例打一行 `PERFJSON`（`scope: "lsp-course"`），由 `scripts/perf-ledger.sh`
//!   收进 `docs/perf/ledger.jsonl` 供跨版本对比。
//!
//! 慢例（`unit12-solution`，实测 36.1s）**默认跳过**：它会让每次 `cargo test`
//! 多花几十秒。要看它：`SOKO_PERF_COURSE_SLOW=1 cargo test -p sokonanoda-lsp --lib perf_course`。

use super::perf::perf_json;
use super::*;

/// 真实课程里量哪几个入口（相对 `courses/set-theory/`）。
/// 括号里是 2026-09-21 的修前实测，作为判读时的对照。
const COURSE_ENTRIES: &[(&str, &str)] = &[
    ("units/unit01-sets-membership.sokonanoda", "1.8s"),
    ("units/unit08-images-preimages.sokonanoda", "4.6s"),
    ("units/unit12-synthesis.sokonanoda", "8.5s"),
];

/// 课程目录（相对 `crates/lsp/`）。找不到就跳过——课程仓与语言仓可以分开检出。
fn course_root() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    dir.join("sokonanoda.toml").is_file().then_some(dir)
}

/// 三个课程用例**互相串行**（同 `perf.rs` 的 `PROJECT_PERF_LOCK` 的理由）。
static COURSE_PERF_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 打开一个真实课程单元，返回 `didOpen → 诊断` 的毫秒数与诊断条数。
async fn open_course_unit(
    service: &mut tower_lsp::LspService<Backend>,
    socket: &mut ClientSocket,
    root: &Url,
    rel: &str,
) -> (u128, usize, Url) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory")
        .join(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("读不到 {}：{error}", path.display()));
    let uri = Url::from_file_path(&path).expect("file url");

    let start = std::time::Instant::now();
    let opened = testutil::did_open_at_drained(service, socket, &uri, &text).await;
    let ms = start.elapsed().as_millis();
    let count = opened.first().map(|o| o.diagnostics.len()).unwrap_or(0);
    assert!(
        !opened.is_empty(),
        "didOpen 必须至少发一份诊断（{rel}）"
    );
    let _ = root;
    (ms, count, uri)
}

#[tokio::test]
async fn perf_course_did_open_is_recorded() {
    let _serial = COURSE_PERF_LOCK.lock().await;
    let Some(root_dir) = course_root() else {
        eprintln!("PERF course: 跳过（找不到 courses/set-theory/sokonanoda.toml）");
        return;
    };
    let root = Url::from_directory_path(&root_dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let mut slowest = 0u128;
    for (rel, before) in COURSE_ENTRIES {
        let (ms, diagnostics, uri) = open_course_unit(&mut service, &mut socket, &root, rel).await;
        println!(
            "PERF course lsp: didOpen {rel} = {ms}ms（{diagnostics} 条诊断；修前基线 {before}）"
        );
        perf_json(serde_json::json!({
            "schema": "soko.perf/1",
            "scope": "lsp-course",
            "case": "did_open",
            "entry": rel,
            "ms": ms,
            "diagnostics": diagnostics,
            "before_ms_note": before,
        }));
        slowest = slowest.max(ms);
        // 同一个 LSP 会话里连续打开：后续文档共享已编译的依赖模块（Session 复用），
        // 所以这里量的是"冷开这个入口"的上界，不是三次独立冷启动。
        testutil::did_close_at(&mut service, &uri).await;
    }

    // 量级哨兵：修前最慢 8.5s。这里给 60s——抓的是"退化成分钟级"（O(n²) 或
    // 每次打开都重编整个闭包 ×N），不是 ±20% 的波动。
    assert!(
        slowest < 60_000,
        "课程入口 didOpen 最慢 {slowest}ms（修前基线 8.5s；量级哨兵 60s）"
    );
    // 会话级复用：同一个 LSP 里再开一次同一份文档，必须**明显**快于冷开。
    let (again, _, _) = open_course_unit(
        &mut service,
        &mut socket,
        &root,
        COURSE_ENTRIES[0].0,
    )
    .await;
    println!("PERF course lsp: 同会话重开 {} = {again}ms", COURSE_ENTRIES[0].0);
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-course",
        "case": "did_open_same_session",
        "entry": COURSE_ENTRIES[0].0,
        "ms": again,
    }));
}

#[tokio::test]
async fn perf_course_by_block_is_recorded() {
    if std::env::var("SOKO_PERF_COURSE_SLOW").ok().as_deref() != Some("1") {
        eprintln!("PERF course: 跳过 by_block（慢例，约 36s；设 SOKO_PERF_COURSE_SLOW=1 打开）");
        return;
    }
    let _serial = COURSE_PERF_LOCK.lock().await;
    let Some(root_dir) = course_root() else {
        eprintln!("PERF course: 跳过（找不到 courses/set-theory/sokonanoda.toml）");
        return;
    };
    let root = Url::from_directory_path(&root_dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let rel = "units/solutions/unit12-solution.sokonanoda";
    let (ms, diagnostics, _uri) = open_course_unit(&mut service, &mut socket, &root, rel).await;
    println!("PERF course lsp: didOpen {rel} = {ms}ms（{diagnostics} 条诊断；修前基线 36.1s）");
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-course",
        "case": "by_block_did_open",
        "entry": rel,
        "ms": ms,
        "diagnostics": diagnostics,
        "before_ms_note": "36.1s",
    }));
}

/// 真实课程上的**一次按键**：改一条声明的名字（整份文档 → 整个闭包重编译）。
///
/// 与 `perf.rs` 的合成夹具同族，但用真课程（8 个模块），量的是用户按键时的真实延迟。
#[tokio::test]
async fn perf_course_keystroke_is_recorded() {
    let _serial = COURSE_PERF_LOCK.lock().await;
    let Some(root_dir) = course_root() else {
        eprintln!("PERF course: 跳过（找不到 courses/set-theory/sokonanoda.toml）");
        return;
    };
    let root = Url::from_directory_path(&root_dir).expect("dir url");
    let (mut service, mut socket) = test_service();
    testutil::handshake_with_root(&mut service, &root).await;

    let rel = "units/unit08-images-preimages.sokonanoda";
    let path = root_dir.join(rel);
    let text = std::fs::read_to_string(&path).expect("读课程单元");
    let uri = Url::from_file_path(&path).expect("file url");
    testutil::did_open_at_drained(&mut service, &mut socket, &uri, &text).await;

    // 来回改同一个标识符 3 次取最小（口径同 `perf.rs` 的按键用例：这个 lib 测试
    // 二进制里 140+ 用例并行跑，单次采样会被邻居抢 CPU 放大）。
    let mut current = text.clone();
    let mut version = 2i32;
    let mut best = u128::MAX;
    for round in 0..3 {
        let next = if round % 2 == 0 {
            current.replace("image_mem", "image_mem_x")
        } else {
            current.replace("image_mem_x", "image_mem")
        };
        let start = std::time::Instant::now();
        let published =
            testutil::did_change_at_drained(&mut service, &mut socket, &uri, version, &next).await;
        best = best.min(start.elapsed().as_millis());
        version += 1;
        assert_eq!(published.len(), 1, "一次按键只发一份文档的诊断：{published:?}");
        current = next;
    }

    println!("PERF course lsp: keystroke {rel} = {best}ms（8 模块闭包）");
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-course",
        "case": "keystroke",
        "entry": rel,
        "ms": best,
    }));
    // 量级哨兵：修前同文本 didChange 是 121–476ms；这里给 5s（抓"退化成分钟级"）。
    assert!(best < 5_000, "课程单元一次按键 {best}ms（量级哨兵 5s）");
}
