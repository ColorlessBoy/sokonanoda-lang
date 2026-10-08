//! **T4-A：CLI / LSP 分路的契约守卫**（规划 `docs/notes/PLAN-align-lean4.md` §5）。
//!
//! ## 契约（一句话）
//!
//! > **跨进程增量归产物（磁盘模块产物），跨按键增量归检查点（线程局部环境）；
//! > CLI = 冷路 + 并行，LSP = 热路 + 单 worker。**
//!
//! 分路**不是设计，是 `ArenaRef: !Send` 逼出来的结果**（`project/session.rs` 的文件头）：
//! 内核环境借 `&'a ArenaRef<'a>`，而它是 `Cell<*mut u8>` + 裸指针 ⇒ 带环境的检查点
//! **只能线程局部**（`LIB_CHECKPOINTS`）⇒ 用它的那条路（LSP）必须把编译钉在一条线程上。
//!
//! ⇒ 于是 CLI 与 LSP 各自只有一条合法路径：
//!
//! | 谁 | 走哪条 | 不该发生什么 |
//! |---|---|---|
//! | CLI（`check`/`course`/`build` 的默认臂） | `compile_plan*` → **栈上 arena**，进程即弃 | **不许**泄漏任何库层检查点（短命进程里它只有代价、没有收益） |
//! | LSP（`QueryDoc`） | `with_project_session_reusing` → **线程局部检查点 LRU** | —— |
//!
//! ## 这条守卫为什么必须有牙
//!
//! 「CLI 不许依赖线程局部检查点」这句话，如果只是文档里的一句声明，下一轮把
//! `compile_plan` 顺手接到 `reusing` 上（"让它也复用一个"）**没人会红** ✗。
//! ⇒ 这里把它变成**两向判据**：
//!
//! ① **正向**：CLI 那条路（`compile_project`）跑完一份真项目，
//!    `lib_checkpoint_arenas_leaked()` **必须恒 0** ✓；
//! ② **反向验证**（守卫的判别力所在）：**同一条判据打到 LSP 那条路**
//!    （`QueryDoc` 的项目编译）上**必须 ≥ 1** —— 若两臂读数一样，
//!    这条守卫就分不清"没泄漏"和"计数器根本没接上"，等于空转 ✗
//!    （`AGENTS.md`：「咬不住的守卫等于没有」）。
//!
//! 夹具/计数器都是**进程级**的 ⇒ 整个契约放在**一个测试函数**里（文件 = 独立测试
//! crate，天然隔离 ✓）；`lib_checkpoint_reset()` 先把两臂放到同一个起点。

use std::path::PathBuf;

use sokonanoda_front::project::session::{lib_checkpoint_arenas_leaked, lib_checkpoint_reset};
use sokonanoda_front::project::{compile_project, plan_project};
use sokonanoda_front::query::QueryDoc;

/// `Common ← LibA ← MainA`（与 `a3_cross_entry_module_reuse.rs` 同形：**有 `import`**
/// 才走项目路；单文件不走 session ⇒ 量不到检查点）。
fn gen_project(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("soko-split-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("Common.sokonanoda"), "axiom P : Prop\n").expect("Common");
    std::fs::write(
        dir.join("LibA.sokonanoda"),
        "import Common\n\naxiom a : P\n",
    )
    .expect("LibA");
    let entry = dir.join("MainA.sokonanoda");
    std::fs::write(&entry, "import LibA\n\ntheorem ta : P := a\n").expect("MainA");
    (dir, entry)
}

#[test]
fn cli_path_leaks_no_checkpoint_but_lsp_path_does() {
    lib_checkpoint_reset();

    // ① **CLI 臂**：`compile_project` = `compile_plan*` → 栈上 arena，跑完即弃。
    let (dir, entry) = gen_project("cli");
    let options = sokonanoda_front::compile::CompileOptions::default();
    let report = compile_project(&entry, None, &options, Some(dir.as_path()));
    assert!(
        report.diagnostics.is_empty(),
        "夹具自检：CLI 臂必须编过：{:?}",
        report.diagnostics
    );
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        0,
        "**T4-A 契约破了**：CLI 那条路（`compile_plan*`）泄漏了库层检查点 \
         ⇒ 它被接到了 `session::reusing` 上。CLI 是**短命进程**：线程局部检查点只活在 \
         `LIB_CHECKPOINTS` 里、下一个子命令又是新进程 ⇒ **只有代价没有收益** ✗。\n\
         （若这是有意为之，请连同 T4-B0 的「先量后做」读数一起改本文件与规划 §5 ✓。）"
    );

    // ② **反向验证（LSP 臂）**：同一条判据打到 `QueryDoc` 上**必须**看到泄漏
    //    —— 否则上面那个 0 只是"计数器没接上"的假绿 ✗。
    let mut doc = QueryDoc::new();
    let text = std::fs::read_to_string(&entry).expect("read entry");
    doc.path = Some(entry.clone());
    doc.set_text(&text, 1, None);
    assert!(
        doc.report.is_some(),
        "反向验证臂必须真的走完一次项目编译（否则量的是错误路径 ✗）"
    );
    assert!(
        lib_checkpoint_arenas_leaked() >= 1,
        "**反向验证失败**：LSP 那条路（`QueryDoc` 的项目编译）应当走 \
         `with_project_session_reusing` 并泄漏 ≥1 份库层检查点（内存上限 8 份，\
         `MAX_LEAKED_LIB_ARENAS`）—— 读到 0 ⇒ 要么它被接到别处去了，要么计数器没接上 \
         ⇒ ① 的那个 0 **没有判别力** ✗。"
    );

    // ③ 契约的**第三半**：分路只该影响"哪条路跑"，不该影响判卷 ——
    //    CLI 臂的报告与"逐入口老路"（`compile_plan`）逐条同形（同一次调用两次 ⇒
    //    逐字节相同）。这条挡住"把 CLI 顺手接到 reusing 上"之后**判定分叉**那一类。
    let plan = plan_project(&entry, None, Some(dir.as_path()));
    let again = sokonanoda_front::project::compile_plan(plan, &options);
    assert_eq!(
        again.diagnostics.len(),
        report.diagnostics.len(),
        "分路不该改变诊断条数"
    );
    assert_eq!(
        again.modules.len(),
        report.modules.len(),
        "分路不该改变模块表"
    );
    let statuses = |r: &sokonanoda_front::project::ProjectReport| -> Vec<String> {
        r.modules
            .iter()
            .map(|m| format!("{}:{:?}", m.name, m.status))
            .collect()
    };
    assert_eq!(
        statuses(&again),
        statuses(&report),
        "两臂的模块状态必须一致（分路只该影响'哪条路跑'，不该影响判定）"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
