//! **T1-A 的红线判据：逐模块库层检查点必须与"整条一趟"那条路**报告逐字相同**。
//!
//! ## 为什么必须有它
//!
//! T1-A 把库层趟从"**整条一趟**"改成"**逐模块趟 + 模块边界检查点**"（为了让
//! 两条入口**只共享前几个模块**时能从那个边界续编 ⇒ `a3` 的 3 → 2 ✓）。
//! 而"切成多趟"会动到一批**按整条闭包算**的派生量（建议材料 / 记法表 / 定义 span 表 /
//! judge 前缀 / 命令坐标系 / 跨单元累加器）—— 每一格都靠**现成覆盖入口**补回去
//! （见 `project/session.rs::run_library_from` 的表 ✓）。
//!
//! ⇒ 那句"补回去"**不能只靠读代码相信** ✗：本文件用**两条真实路径**对拍 ——
//! **CLI 老路**（`compile_project` = 整条一趟）vs **LSP 路**（`QueryDoc` =
//! `with_project_session_reusing` = 逐模块）—— 同一份夹具、同一份输入，
//! **逐模块报告逐字相同** ✓（`AGENTS.md` 硬规则 1 的"判定正确性不变"）。
//!
//! ## 两臂各覆盖什么
//!
//! | 臂 | 走的路 | 覆盖 |
//! |---|---|---|
//! | ① `MainA`（冷） | 逐模块**冷建**检查点 | 每个模块边界上的环境/报告 |
//! | ② `MainB`（共享 `Common`） | **前缀续编**（`Common` 命中、`LibB` 续编） | `ResumeState`（`closure_id`/`exports`/`example_idx`）+ 输出拼接 |
//!
//! ⚠ 两条臂都要比，**只比第一条等于没比** ✗：续编那条才是 T1-A 新增的那条路。

use std::path::PathBuf;

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::compile_project;
use sokonanoda_front::project::session::lib_checkpoint_reset;
use sokonanoda_front::query::QueryDoc;

/// `Common ← LibA ← MainA` × `Common ← LibB ← MainB`（与 `a3_*` 同形）。
///
/// `MainA` 与 `MainB` 的库层**只共享前 1 个模块**（`Common`）⇒ 第二条入口走
/// **前缀续编**那条路 ✓。
fn gen_project(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("soko-t1a-parity-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("Common.sokonanoda"), "axiom P : Prop\n").expect("Common");
    std::fs::write(
        dir.join("LibA.sokonanoda"),
        "import Common\n\naxiom a : P\n",
    )
    .expect("LibA");
    std::fs::write(
        dir.join("LibB.sokonanoda"),
        "import Common\n\ntheorem b : P -> P := fun (h : P) => h\naxiom b2 : P\n",
    )
    .expect("LibB");
    let main_a = dir.join("MainA.sokonanoda");
    std::fs::write(&main_a, "import LibA\n\ntheorem ta : P := a\n").expect("MainA");
    let main_b = dir.join("MainB.sokonanoda");
    std::fs::write(&main_b, "import LibB\n\ntheorem tb : P := b2\n").expect("MainB");
    (dir, main_a, main_b)
}

/// 一臂的**可比较快照**：模块名 / 状态 / 逐模块报告（`DocumentReport` 的 JSON）/
/// 编译事件与诊断（`CompileOutput` 的 JSON）。
///
/// 用 JSON 文本比而不是 `PartialEq`：`DocumentReport` 没实现 `PartialEq`
/// （`derive(Debug, Clone, Default, Serialize, Deserialize)`），而它**实现了
/// `Serialize`** ⇒ JSON 逐字节就是"报告逐字相同"的最强可执行形式 ✓。
fn snapshot(report: &sokonanoda_front::project::ProjectReport) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for module in &report.modules {
        out.push(format!(
            "{}:{:?}:{}:{}",
            module.name,
            module.status,
            serde_json::to_string(&module.report).expect("serialize report"),
            serde_json::to_string(&module.events).expect("serialize events"),
        ));
    }
    out.push(format!(
        "diagnostics:{}",
        serde_json::to_string(&report.diagnostics).expect("serialize diagnostics")
    ));
    out
}

#[test]
fn module_checkpoints_are_report_identical_to_the_monolithic_path() {
    lib_checkpoint_reset();
    let (dir, main_a, main_b) = gen_project("both");
    let options = CompileOptions::default();

    // ① **冷开**：两条路径对拍（逐模块冷建 ↔ 整条一趟）。
    let mono_a = compile_project(&main_a, None, &options, Some(dir.as_path()));
    assert!(
        mono_a.diagnostics.is_empty(),
        "夹具自检：`MainA` 必须编过：{:?}",
        mono_a.diagnostics
    );
    let mut doc = QueryDoc::new();
    doc.path = Some(main_a.clone());
    let text_a = std::fs::read_to_string(&main_a).expect("read MainA");
    doc.set_text(&text_a, 1, None);
    let per_a = doc
        .project_report_ref()
        .expect("LSP 路必须产出项目报告（否则量的是错误路径 ✗）")
        .clone();
    assert_eq!(
        snapshot(&mono_a),
        snapshot(&per_a),
        "**T1-A 红线**：冷开那一刀，逐模块库层检查点那条路与'整条一趟'那条路的\
         报告必须**逐字相同**（模块名/状态/`DocumentReport`/`CompileOutput`/诊断）✗\n\
         ⇒ 不等说明某格覆盖入口没接上（建议材料/记法表/定义 span 表/judge 前缀/\
         跨单元累加器，见 `run_library_from` 的表 ✓）。"
    );

    // ② **前缀续编**：`MainB` 与 `MainA` 只共享 `Common` ⇒ 走 `resume_at` 那条路。
    let mono_b = compile_project(&main_b, None, &options, Some(dir.as_path()));
    assert!(
        mono_b.diagnostics.is_empty(),
        "夹具自检：`MainB` 必须编过：{:?}",
        mono_b.diagnostics
    );
    doc.path = Some(main_b.clone());
    let text_b = std::fs::read_to_string(&main_b).expect("read MainB");
    doc.set_text(&text_b, 2, None);
    let per_b = doc
        .project_report_ref()
        .expect("续编之后也要有项目报告")
        .clone();
    assert_eq!(
        snapshot(&mono_b),
        snapshot(&per_b),
        "**T1-A 红线（续编那一臂）**：从 `Common` 的模块边界续编 `LibB` 之后，报告\
         必须与整条一趟**逐字相同** ✗\n\
         ⇒ 不等就是 `ResumeState` 三样（`closure_id`/`exports`/`example_idx`）\
         或输出拼接口径没对齐。"
    );
    // **续编那条路**的判据读数（证明上面那一臂真的走了它，而不是又冷建了一遍）：
    // 冷建会 `Box::leak` 一份新 arena，续编**不会**（游标只活本次调用 ✓）。
    assert_eq!(
        sokonanoda_front::project::session::lib_checkpoint_arenas_leaked(),
        1,
        "**前缀续编不许泄漏新 arena**：`MainB` 应当从 `Common` 的检查点续编\
         （实测泄漏计数 1 = 只有冷开 `MainA` 那一份 ✓）—— 读到 2 说明它**又冷建**了\
         一条库层（那是「没走续编路」，读数 2 就失去判别力 ✗）。"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
