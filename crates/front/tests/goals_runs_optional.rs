//! **`goals_without_runs` 的判据 + 载荷读数（第 95 轮 · 平行线）** —— 设计
//! `docs/design/goals-payload-slimming.md` §3 候选 B 的配套判据 ✓。
//!
//! 判据（**等价性**）：剥掉着色数据后，其余字段与全量版**逐字段相同** ✓
//! （名字/kind/status/span/ty/value/goal ✓），而 `ty_runs`/`value_runs`/`goal_runs` **全空** ✓。
//! ⚠ **反向前提必须放在循环外**（第 94 轮那次就是把它放进逐条循环 ⇒ 在"本来就没有 runs 的
//! 声明"上判红 ✗）：这里先算**全局**总 runs 数 > 0 ✓ —— "咬不住的守卫等于没有" ✓。

use sokonanoda_front::query::QueryDoc;

fn real_unit08() -> Option<QueryDoc> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda");
    let text = std::fs::read_to_string(&path).ok()?;
    let mut doc = QueryDoc::new();
    doc.path = Some(path.clone()); // 项目模式（unit08 有 `import Set` ✓）
    doc.set_text(&text, 1, None);
    Some(doc)
}

#[test]
fn stripping_runs_changes_nothing_else_and_cuts_the_payload() {
    let Some(doc) = real_unit08() else {
        eprintln!("跳过：读不到课程单元");
        return;
    };
    let full = doc.goals(false).expect("goals");
    let light = doc.goals_without_runs(false).expect("goals_without_runs");

    // ① **反向前提（全局，不在循环里 ✗）**：全量版**确实**带着 runs，否则本判据是空转 ✓。
    let runs_full: usize = full
        .iter()
        .map(|d| d.ty_runs.len() + d.value_runs.len() + d.goal_runs.len())
        .sum();
    assert!(
        runs_full > 0,
        "夹具前提：全量版总 runs 数必须 > 0（否则判据空转 ✗）"
    );

    // ② **等价性**：条数与"除 runs 外"的每个字段逐字段相同 ✓。
    assert_eq!(full.len(), light.len(), "条数必须相同");
    for (a, b) in full.iter().zip(light.iter()) {
        assert_eq!(a.name, b.name, "name");
        assert_eq!(a.kind, b.kind, "kind");
        assert_eq!(a.status, b.status, "status");
        assert_eq!((a.start, a.end), (b.start, b.end), "span");
        assert_eq!(a.ty, b.ty, "ty");
        assert_eq!(a.value, b.value, "value");
        assert_eq!(a.goal, b.goal, "goal");
        assert!(b.ty_runs.is_empty(), "`ty_runs` 必须为空");
        assert!(b.value_runs.is_empty(), "`value_runs` 必须为空");
        assert!(b.goal_runs.is_empty(), "`goal_runs` 必须为空");
    }

    // ③ **读数**：两版 JSON 体积差 = 这一刀省下的载荷 ✓。
    let s_full = serde_json::to_string(&full).expect("ser").len();
    let s_light = serde_json::to_string(&light).expect("ser").len();
    println!(
        "PERF goals-runs-optional: 全量 decls≈{}B（runs 共 {} 条）⇒ 剥掉后≈{}B · 省 **{}B（{:.0}%）** ✓",
        s_full,
        runs_full,
        s_light,
        s_full - s_light,
        100.0 * (s_full - s_light) as f64 / s_full as f64
    );
}
