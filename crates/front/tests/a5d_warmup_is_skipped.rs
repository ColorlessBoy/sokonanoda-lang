//! **A5d 的可咬守卫**（2026-10-09）：**模块产物这条路开着 ⇒ 投机预热一律不做** ✓✓。
//!
//! ## 为什么断言的是"返回值"，而不是探针里的 trace
//!
//! A5d 的落点在 [`warm_library_checkpoint`] 的**返回值**上（`false` = 没建检查点 ✓）。
//! 判据下移到这里有两个好处：
//! * **进程内**——不必把开关送进 LSP 子进程（逃生门是 env 驱动 ⇒ 在 LSP 测试里
//!   `SOKO_*` 传不进去，实测两次都送不到 ✗，见 `lsp_artifact_warmup.rs` 的如实记）；
//! * **反向验证做得到**——把 A5d 那段早退去掉，这条**当场判红** ✓（见文件末尾的用法）。
//!
//! ## 背景（为什么这条值得守）
//!
//! 预热与产物**做的是同一件事**（把库层供上），而编译被钉在**一条** worker 线程上
//! （`worker_threads(1)`）⇒ 预热会**挡用户那一刀**、还会**挤掉多槽 LRU**。实测（同一探针、
//! 同一构建、只切 `SOKO_NO_LIB_WARMUP` 两档）：第一刀 **333 → 127ms**（2.6×）·
//! 跨入口 **911 → 665ms**（−27%，`modules=8` 复用 0 ⇒ `5` 复用 3）。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::warm_library_checkpoint;

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("soko-a5d-{tag}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// 一个小模块根（有 `import` ⇒ 有库层 ⇒ 预热这条路真的会跑起来 ✓）。
fn fixture(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, String) {
    let root = temp_root(tag);
    std::fs::create_dir_all(&root).expect("mkdir");
    std::fs::write(
        root.join("Lib.sokonanoda"),
        "theorem l1 (P : Prop) (h : P) : P := by\n  exact h\n",
    )
    .expect("write lib");
    let entry = root.join("E.sokonanoda");
    let text = "import Lib\ntheorem e1 (P : Prop) : P → P := fun h => h\n".to_string();
    std::fs::write(&entry, &text).expect("write entry");
    (root, entry, text)
}

#[test]
fn the_speculative_warmup_is_skipped_while_module_artifacts_are_enabled() {
    let (root, entry, text) = fixture("skip");
    let options = CompileOptions::default();

    // **契约**：产物这条路开着（默认）⇒ 预热**直接返回 `false`**（没建检查点 ✓）——
    // 而且**与"盘上有没有产物"无关**（A5c 管"有产物就跳"、A5d 管"这条路开着就跳" ✓）。
    assert!(
        sokonanoda_front::project::session::module_artifacts_enabled(),
        "夹具前提：这条判据只在**产物开着**的档下有意义（逃生门设了就该跳过本用例 ✓）"
    );
    assert!(
        !warm_library_checkpoint(&entry, &text, Some(root.as_path()), &[], &options),
        "**产物在飞 ⇒ 不许做投机预热** ✗（A5d）：`warm_library_checkpoint` 必须返回 `false` \
         —— 否则预热会挡用户那一刀、还会挤掉多槽 LRU（实测第一刀 333→127ms、跨入口 911→665ms）"
    );

    // 反向验证的用法（**做得到**，与 LSP 那条不同 ✓）：把 `warm_library_checkpoint` 里
    // A5d 的早退注释掉再跑本用例 ⇒ 这里必须**判红** ✓。
    let _ = std::fs::remove_dir_all(&root);
}
