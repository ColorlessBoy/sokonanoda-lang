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
    // ⭐ **项目模式**（第 87 轮）：设上 `path` 才会去解 `import` 闭包 ✓（LSP 那条路就是这么做的 ✓
    // —— 与 `a3`/`a4a`/`a7` 等既有测试同款 ✓）。不设的话 `import Set` 解不了 ⇒ 量不到真工作量 ✗。
    doc.path = Some(path.clone());
    doc.set_text(&text, 1, None);
    let _ = doc.goals(false).expect("first goals（项目模式）");
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

/// **控制实验：声明数相同、只把类型文本变复杂**（第 88 轮）——
/// 平凡夹具 13 条 = **0.20ms** ✓、真 unit08 13 条 = **2.16ms** ✗ ⇒ 差在**类型文本**而非条数 ✓。
/// 这条夹具把条数固定成 13，只把类型拉长（深 `->` 链 ⇒ 记法/run 更多 ✓）：
/// 若它跳到毫秒级 ⇒ front 的成本**随类型文本走** ✓ ⇒ 下一步只在"**逐条处理类型文本**"的那几处里找
/// （`runs` 已两次实测否掉 ✗ ⇒ 只剩 `display.fold` / `ty_runs` 之外的逐条工作 ✓）。
#[test]
fn goals_cost_scales_with_type_text_not_decl_count() {
    let mut src = String::from("axiom P : Prop\naxiom Q : Prop\n");
    // 13 条（与上面两条夹具同数），但类型是**深链**： run/记法的数量级上去 ✓。
    let long_ty =
        "(P -> Q) -> (Q -> P) -> (P -> Q) -> (Q -> P) -> (P -> Q) -> (Q -> P) -> (P -> Q) -> P";
    for i in 0..13 {
        src.push_str(&format!("theorem long{i} : {long_ty} := by sorry\n"));
    }
    let mut doc = QueryDoc::new();
    doc.set_text(&src, 1, None);
    let _ = doc.goals(false).expect("first goals");
    let mut goals = Vec::new();
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let _ = doc.goals(false).expect("goals");
        goals.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    goals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "PERF goals-cost-longtypes: goals(false) median {:.2}ms (n=5, 13 条深链类型) · \
         对照：13 条平凡 = 0.20ms ✓ / 真 unit08 = 2.16ms ⇒ 若这里 ≈ms 级 ⇒ 成本随**文本**走 ✓",
        goals[2]
    );
}

/// **控制实验 2：同款长类型，改成 `axiom`（无 `goal` ⇒ 不调 `display.fold`）**（第 88 轮）——
/// 上一条长类型夹具用的是 `:= by sorry`（开放 ⇒ 每条都过一遍 `display.fold(goal)` ✓）。
/// 若"长类型 + axiom"掉回 ~0.2ms ⇒ 贵因是**逐条的 `fold`** ✓；若仍 ~1.3ms ⇒ 贵因在**类型文本本身**
/// 的那条路（记法/run ✓，而 `runs` 已两次被否 ⇒ 落在 `notation_symbols`/`decl_kinds` 这类**每文档一次**
/// 的预处理上也有份 ⇒ 那就与"条数×文本"的耦合有关 ✓）。
#[test]
fn goals_cost_without_goals_isolates_the_fold() {
    let mut src = String::from("axiom P : Prop\naxiom Q : Prop\n");
    let long_ty =
        "(P -> Q) -> (Q -> P) -> (P -> Q) -> (Q -> P) -> (P -> Q) -> (Q -> P) -> (P -> Q) -> P";
    for i in 0..13 {
        // `axiom` 有类型、**没有 goal** ⇒ `goal_display` 为 `None` ⇒ 不调 `fold` ✓。
        src.push_str(&format!("axiom long{i} : {long_ty}\n"));
    }
    let mut doc = QueryDoc::new();
    doc.set_text(&src, 1, None);
    let _ = doc.goals(false).expect("first goals");
    let mut goals = Vec::new();
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let _ = doc.goals(false).expect("goals");
        goals.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    goals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "PERF goals-cost-axiomlongtypes: goals(false) median {:.2}ms (n=5, 13 条深链类型但**无 goal**) · \
         对照：同款类型 + `by sorry`（有 goal）= 1.31ms ⇒ 若这里 ≈0.2 ⇒ 贵因 = 逐条 `display.fold` ✓",
        goals[2]
    );
}
