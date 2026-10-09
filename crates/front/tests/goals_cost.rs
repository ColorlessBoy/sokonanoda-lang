//! **goal 口径的**分层读数**（第 86 轮 · 平行线）**：`soko/goals` 的 5.9ms（真子进程读数 ✓）
//! 到底花在哪一层？本文件在 **front 层**把 `QueryDoc` 的公开入口分开计时 ⇒
//! **先量再动** ✓（第 85 轮的教训：读码猜定位 = 白做一轮 ✗）。
//!
//! 读数**不是判据**（墙钟不许当门禁 ✓）；判据在别处（同一文档连问两次逐字节相同 ✓）。
//! 夹具是**合成项目**（与 `keystroke_structure.rs` 同纪律：不依赖课程目录 ✓）。

use sokonanoda_front::query::QueryDoc;

fn fixture() -> QueryDoc {
    // 十几条声明 + 末尾一条带 `sorry` 的开放练习 ⇒ 形状与 unit08 同量级 ✓。
    let mut src = String::from("axiom P : Prop\naxiom Q : Prop\n");
    for i in 0..12 {
        src.push_str(&format!("theorem t{i} (h : P) : P := h\n"));
    }
    src.push_str("theorem open_one (h : P) : Q := by sorry\n");
    let mut doc = QueryDoc::new();
    doc.set_text(&src, 1, None);
    doc
}

#[test]
fn goals_cost_is_split_by_layer() {
    let doc = fixture();
    // 先跑一次把报告落定（量的是**稳态** ✓）。
    let _ = doc.goals(false).expect("first goals");
    let mut check = Vec::new();
    let mut goals = Vec::new();
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let _ = doc.check();
        check.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = std::time::Instant::now();
        let _ = doc.goals(false);
        goals.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    check.sort_by(|a, b| a.partial_cmp(b).unwrap());
    goals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "PERF goals-cost: check median {:.2}ms · goals(false) median {:.2}ms (n=5) · \
         （对照：真子进程 goals=5.9ms / 诊断臂 74.9ms ⇒ 本读数说明 front 这两层各占多少 ✓）",
        check[2], goals[2]
    );
}

/// **同一条读数、换成真课程文本**：上面那条用的是**平凡类型**（`P`/`Q`）⇒ 只能当**下界** ✗。
/// 这一条读 `courses/set-theory/units/I.3/unit08-*.sokonanoda` 的真文本（找不到就跳过 ✓，
/// 与按键探针同纪律 ✓）⇒ 才能判定 5.9ms 的大头到底在 **front** 还是在 **LSP 层** ✓。
#[test]
fn goals_cost_on_the_real_course_unit() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    let mut doc = QueryDoc::new();
    doc.set_text(&text, 1, None);
    // ⚠ unit08 有 `import Set` ⇒ **单文件** `QueryDoc`（没设 `path`/`root`）解不了闭包 ✗ ⇒
    // 这里**如实跳过**（不当失败 ✓）：要量它得先把 `path`/`root` 摆成项目模式 ✓
    // （LSP 那条路就是这么做的 ✓）—— 记为下一刀的备选 ✓。
    let Ok(first) = doc.goals(false) else {
        eprintln!("跳过：单文件 QueryDoc 解不了 unit08 的 `import` ⇒ 需设 path/root（项目模式）");
        return;
    };
    let _ = first;
    let mut goals = Vec::new();
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let _ = doc.goals(false);
        goals.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    goals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "PERF goals-cost-course: goals(false) median {:.2}ms (n=5) · 对照：同文本真子进程 = 5.9ms \
         ⇒ 若这里 ≈5.9 ⇒ 大头在 **front** ✓；若这里 ≪5.9 ⇒ 大头在 **LSP 层** ✓（第 86 轮的判定点 ✓）",
        goals[2]
    );
}
