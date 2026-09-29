//! 项目层（`import` 闭包）性能哨兵 + 分阶段实测（I16 例行化）。
//!
//! 与 `crates/front/tests/perf.rs`（单文件编译器路径）配套：这里量的是**项目
//! 特有的环节**，每一环都留一行可采集的记录：
//!
//! * `PERF project …`          —— 人类可读（`scripts/perf-report.sh` / CI artifact 收集）
//! * `PERFJSON {…}`            —— 机器可读（`scripts/perf-ledger.sh` 收进
//!   `docs/perf/ledger.jsonl`，供跨版本对比"哪一环退化了"）
//!
//! 阶段划分（对应 `docs/architecture.md` §4.5 的六步）：
//! `plan`（根发现 + 闭包加载：IO/parse）→ `digest`（闭包哈希）→
//! `compile`（拓扑序一次过内核）→ `total`（= 前几项之和，CLI/LSP 每次按键的真实成本）。
//!
//! 阈值原则与单文件层一致：**只抓算法级回归**（O(n²)/意外的重复编译），
//! 缩放比按"线性 × 1.6 余量"，绝对延迟阈值刻意宽松（CI runner 波动 2-3 倍）。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::{compile_plan, compile_project, plan_project};

/// 生成一条 `modules` 长的依赖链项目：`Lib0 ← Lib1 ← … ← Main`。
///
/// 链是**最深**的闭包形状（每次编译都要按拓扑序走完全部模块），也是"一次按键
/// 重编译整个闭包"的最坏情况；每个模块 `decls` 条已证声明。
fn gen_project(tag: &str, modules: usize, decls: usize) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("soko-perf-project-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

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
        std::fs::write(dir.join(format!("Lib{index}.sokonanoda")), text).expect("write module");
    }

    let mut entry = String::new();
    entry.push_str(&format!("import Lib{}\n\n", modules - 1));
    for i in 0..decls {
        entry.push_str(&format!("theorem main_s{i} : P := proofP\n"));
    }
    let entry_path = dir.join("Main.sokonanoda");
    std::fs::write(&entry_path, entry).expect("write entry");
    (dir, entry_path)
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// 一次完整的分阶段测量（plan → digest → compile）。
fn measure_stages(entry: &Path) -> (f64, f64, f64, f64) {
    let options = CompileOptions::default();
    let started = Instant::now();
    let plan = plan_project(entry, None, None);
    let plan_ms = ms(started.elapsed());
    let started = Instant::now();
    let _digest = plan.digest(&options);
    let digest_ms = ms(started.elapsed());
    let started = Instant::now();
    let report = compile_plan(plan, &options);
    let compile_ms = ms(started.elapsed());
    assert!(
        report.diagnostics.is_empty(),
        "generated project must be clean: {:?}",
        report.diagnostics
    );
    (
        plan_ms,
        digest_ms,
        compile_ms,
        plan_ms + digest_ms + compile_ms,
    )
}

/// 同一项目重复 `rounds` 轮分阶段测量，取 `total` 最小的那一轮。
///
/// 为什么需要：这些用例在**同一个测试进程里并行**跑（`cargo test` 默认多线程），
/// 单次采样会被同进程的其他重活抢占——实测**同一份代码**两次记录相差 25%
/// （见 `docs/PERF.md`「噪声地板」）。取最小轮 = "这个阶段最少要多久"，
/// 与 `perf.rs` / LSP perf 的 best-of-N 口径一致。真正要求准确时用
/// `--test-threads=1`（`scripts/perf-ledger.sh` 已带）。
fn measure_best(entry: &Path, rounds: usize) -> (f64, f64, f64, f64) {
    let mut best = measure_stages(entry);
    for _ in 1..rounds {
        let candidate = measure_stages(entry);
        if candidate.3 < best.3 {
            best = candidate;
        }
    }
    best
}

fn perf_json(value: serde_json::Value) {
    println!("PERFJSON {value}");
}

// ── 1. 分阶段实测（教学规模：4 模块 × 20 声明）────────────────────────

#[test]
fn project_closure_stage_costs_are_recorded() {
    // 预热：首跑含冷启动（页缓存/内核首次分配），会污染第一个数据点。
    {
        let (dir, entry) = gen_project("warmup", 2, 2);
        let _ = compile_project(&entry, None, &CompileOptions::default(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
    let (dir, entry) = gen_project("stages", 4, 20);
    let (plan_ms, digest_ms, compile_ms, total_ms) = measure_best(&entry, 3);
    println!(
        "PERF project stages: plan {plan_ms:.1}ms · digest {digest_ms:.2}ms · \
         compile {compile_ms:.1}ms · total {total_ms:.1}ms (4 modules × 20 decls)"
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "front-project",
        "case": "closure_stages",
        "modules": 4,
        "decls_per_module": 20,
        "plan_ms": (plan_ms * 100.0).round() / 100.0,
        "digest_ms": (digest_ms * 1000.0).round() / 1000.0,
        "compile_ms": (compile_ms * 100.0).round() / 100.0,
        "total_ms": (total_ms * 100.0).round() / 100.0,
    }));
    // 绝对上界**只挡"数量级"退化** —— ⚠ **2026-09-29 实测教训**：
    // 这条原本是 `< 2000.0`，在 **CI 的 ubuntu runner 上实测 2038.2ms 判红** ✗，
    // 而**同一 commit 本地 < 2000ms 全绿** ⇒ 那是**假红**（本机 vs CI runner 实测差 **7.5×**，
    // 见 `AGENTS.md` 的 perf-gate 教训）。**门禁假红比没有门禁更糟** ⇒
    // 天花板放宽到**只抓数量级**（20×），把"细粒度回归"交给**机器无关**的判据：
    // ① 下面的 `plan_ms + digest_ms < compile_ms`（**比值**，与机器快慢无关 ✓）；
    // ② `scripts/check-recompile-factor.py`（**测次数**，噪声免疫 ✓，已在 CI 判红）。
    assert!(
        total_ms < 40000.0,
        "a 4×20 teaching project took {total_ms:.1}ms — **order-of-magnitude** regression?"
    );
    // 加载/哈希不该成为成本主体：parse+IO+哈希 < 编译内核的时间。
    assert!(
        plan_ms + digest_ms < compile_ms.max(50.0),
        "plan+digest ({:.1}ms) dominates compile ({compile_ms:.1}ms)",
        plan_ms + digest_ms
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── 2. 缩放：闭包规模翻倍 ⇒ 时间线性（不是 O(n²)）────────────────────

#[test]
fn project_closure_compile_scales_linearly() {
    let options = CompileOptions::default();
    {
        let (dir, entry) = gen_project("scale-warmup", 2, 10);
        let _ = compile_project(&entry, None, &options, None);
        let _ = std::fs::remove_dir_all(&dir);
    }
    let mut times = Vec::new();
    for modules in [4usize, 8, 16] {
        let (dir, entry) = gen_project(&format!("scale-{modules}"), modules, 10);
        let (_, _, compile_ms, _) = measure_best(&entry, 3);
        times.push(compile_ms);
        let _ = std::fs::remove_dir_all(&dir);
    }
    let small = times[0].max(0.01);
    let large = times[2].max(0.01);
    let ratio = large / small; // 4× 规模，线性 = 4，O(n²) = 16
    println!(
        "PERF project compile scaling: 4/8/16 modules → {:?} ms (4× size ratio {ratio:.1}×)",
        times
            .iter()
            .map(|value| (value * 10.0).round() / 10.0)
            .collect::<Vec<_>>()
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "front-project",
        "case": "closure_compile_scaling",
        "modules": [4, 8, 16],
        "decls_per_module": 10,
        "ms": times.iter().map(|v| (v * 100.0).round() / 100.0).collect::<Vec<_>>(),
        "ratio_4x": (ratio * 100.0).round() / 100.0,
    }));
    assert!(
        ratio < 6.4,
        "4× more modules took {ratio:.1}× the time (linear = 4×, O(n²) = 16×)"
    );
}

// ── 3. 按键路径：改一行 ⇒ 重编译整个闭包（项目模式今天的真实成本）──

/// **同机参考量**：全新项目上的**冷编译**耗时（`gen_project` + 一次编译，不带覆盖）。
///
/// 为什么要它（2026-09-28 实测，见下）：本用例原来的判据是**绝对**墙上时钟
/// `worst < 2000.0`，而**绝对毫秒跨机器不可转移** —— 同一份 `crates/` 代码实测：
/// 本机 **44–71ms**、CI run `36362709263` **240ms**、`36373825237` **343ms**、
/// `36378945287` attempt 1 **2431ms**（判红）/ attempt 2 **511ms**（判绿）。
/// 同一 job 同代码两次差 **4.8×**、跨机差 **55×** ⇒ 那个断言其实在**量机器**，不是在量代码。
///
/// 而**比值**是稳的：本机实测 `keystroke_worst / cold_ref = 44.6 / 74.5 = 0.60`。
/// ⇒ 判据改成「**按键重编译 vs 同机冷编译**」的比值，再配一个**很宽的**绝对天花板
/// （防"两边一起变慢"）：真正的量级回归两者都会咬住，机器抖动咬不住。
fn cold_reference_ms(tag: &str, options: &CompileOptions) -> f64 {
    let (dir, entry) = gen_project(tag, 4, 20);
    let started = Instant::now();
    let report = compile_project(&entry, None, options, None);
    let elapsed = ms(started.elapsed());
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let _ = std::fs::remove_dir_all(&dir);
    elapsed
}

#[test]
fn project_keystroke_recompiles_the_closure_within_budget() {
    let options = CompileOptions::default();
    let (dir, entry) = gen_project("keystroke", 4, 20);
    let base = std::fs::read_to_string(&entry).expect("read entry");
    // 模拟"改最后一条声明的名字"（编辑器里的一次按键最终就是一次整文件重编译）。
    let edited = base.replace("main_s19", "main_s19x");
    // 记录两个统计量：`worst`（5 次里最慢，断言用的保守上界）与 `best`（5 次里最快，
    // 跨版本比较用——max 统计量天然抖，同一提交两次记录实测能差 45%）。
    let mut worst = 0.0f64;
    let mut best = f64::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        let report = compile_project(&entry, Some(&edited), &options, None);
        let elapsed = ms(started.elapsed());
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        worst = worst.max(elapsed);
        best = best.min(elapsed);
    }
    // 同机参考量取 3 次里最快的（最接近"这台机器的干净速度"，干扰最小）。
    let cold_ref = (0..3)
        .map(|i| cold_reference_ms(&format!("keystroke-ref{i}"), &options))
        .fold(f64::MAX, f64::min);
    let ratio = worst / cold_ref;
    println!(
        "PERF project keystroke: recompile closure (4 modules × 20 decls) \
         best {best:.1}ms · worst {worst:.1}ms · cold-ref {cold_ref:.1}ms · ratio {ratio:.2}"
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "front-project",
        "case": "keystroke_recompile_closure",
        "modules": 4,
        "decls_per_module": 20,
        "best_ms": (best * 100.0).round() / 100.0,
        "worst_ms": (worst * 100.0).round() / 100.0,
        "cold_ref_ms": (cold_ref * 100.0).round() / 100.0,
        "ratio": (ratio * 100.0).round() / 100.0,
    }));
    // ① **机器无关的量级判据**：按键重编译不该比同机冷编译贵一个量级。
    //    实测比值 **0.60** ⇒ 8× 留 13 倍余量（只咬量级回归，不咬机器抖动）。
    assert!(
        ratio < 8.0,
        "one keystroke cost {worst:.1}ms vs a same-machine cold compile of {cold_ref:.1}ms \
         (ratio {ratio:.2}) — order-of-magnitude regression?"
    );
    // ② **很宽的绝对天花板**：防"重编译与冷编译一起变慢"（比值看不出来那种）。
    //    取 30000ms ⇒ 相对实测（本机 71ms / CI 2431ms 最坏那次）仍有 12× 以上余量；
    //    它挡的是"整条路慢掉一个数量级"，不是"机器今天很吵"。
    assert!(
        worst < 30000.0,
        "one keystroke cost {worst:.1}ms on a 4×20 project — order-of-magnitude regression?"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── 4. 内存覆盖不改变成本量级（LSP 每次通知都走它）──────────────────

#[test]
fn project_compile_with_overlay_costs_the_same_order() {
    use sokonanoda_front::project::compile_project_with_overlay;
    let options = CompileOptions::default();
    let (dir, entry) = gen_project("overlay", 4, 20);
    let overlay: Vec<(PathBuf, String)> = (0..4)
        .map(|index| {
            let path = dir.join(format!("Lib{index}.sokonanoda"));
            let text = std::fs::read_to_string(&path).expect("read module");
            (path, text)
        })
        .collect();

    let mut plain = f64::MAX;
    let mut overlaid = f64::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        let report = compile_project(&entry, None, &options, None);
        plain = plain.min(ms(started.elapsed()));
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);

        let entry_text = std::fs::read_to_string(&entry).expect("read entry");
        let started = Instant::now();
        let report =
            compile_project_with_overlay(&entry, Some(&entry_text), &options, None, &overlay);
        overlaid = overlaid.min(ms(started.elapsed()));
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }
    println!(
        "PERF project overlay: disk {plain:.1}ms vs overlay {overlaid:.1}ms (4 modules × 20 decls)"
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "front-project",
        "case": "overlay_overhead",
        "modules": 4,
        "decls_per_module": 20,
        "disk_ms": (plain * 100.0).round() / 100.0,
        "overlay_ms": (overlaid * 100.0).round() / 100.0,
    }));
    // 覆盖只是"把读盘换成读内存"，不该比读盘慢一个量级（含 canonicalize 的系统调用）。
    assert!(
        overlaid < plain.max(50.0) * 4.0,
        "overlay compile ({overlaid:.1}ms) is far slower than disk ({plain:.1}ms)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── 5. 教学规模的真实按键成本（2/3/5 模块 × 12 声明）──────────────
#[test]
fn project_sizes_teaching_scale_keystroke_cost() {
    let options = CompileOptions::default();
    let mut records = Vec::new();
    for modules in [2usize, 3, 5] {
        let (dir, entry) = gen_project(&format!("teach-{modules}"), modules, 12);
        let base = std::fs::read_to_string(&entry).expect("read entry");
        let edited = base.replace("main_s11", "main_s11x");
        let mut best = f64::MAX;
        for _ in 0..5 {
            let started = Instant::now();
            let report = compile_project(&entry, Some(&edited), &options, None);
            best = best.min(ms(started.elapsed()));
            assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        }
        println!("PERF project teaching scale: {modules} modules x 12 decls keystroke {best:.1}ms");
        records.push((modules, (best * 100.0).round() / 100.0));
        let _ = std::fs::remove_dir_all(&dir);
    }
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "front-project",
        "case": "teaching_scale_keystroke",
        "decls_per_module": 12,
        "modules": records.iter().map(|(m, _)| *m).collect::<Vec<_>>(),
        "ms": records.iter().map(|(_, ms)| *ms).collect::<Vec<_>>(),
    }));
}

// ── 6. 判据前缀成本：`match`/`by` 每次都要把闭包前缀交给内核 ──────────────

/// 入口里对**被导入**归纳类型做 `match`：每次 match 都要合成"闭包前缀 + 本文件
/// 前缀"再问内核（`judge_infer`，见 `docs/architecture.md` §4.5）。这是"课程内容
/// import 化"的真实成本：前缀里现在带着依赖的声明文本。
#[test]
fn judge_prefix_with_imported_declarations_stays_within_budget() {
    let dir = std::env::temp_dir().join(format!("soko-perf-judge-prefix-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("Logic.sokonanoda"),
        "inductive Or (A B : Prop) : Prop\n\
ctor inl (a : A) : Or A B\n\
ctor inr (b : B) : Or A B\n\
end\n",
    )
    .expect("write logic");
    let mut entry = String::from("import Logic\n\n");
    for i in 0..10 {
        entry.push_str(&format!(
            "theorem or_comm_{i} (A : Prop) (B : Prop) (h : Or A B) : Or B A :=\n\
  match h with\n\
  | inl a => inr B A a\n\
  | inr b => inl B A b\n"
        ));
    }
    let entry_path = dir.join("Main.sokonanoda");
    std::fs::write(&entry_path, &entry).expect("write entry");

    let options = CompileOptions::default();
    // 预热（首次执行/首次判据缓存都算冷启动）。
    let _ = compile_project(&entry_path, Some(&entry), &options, None);
    let mut best = f64::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        let plan = plan_project(&entry_path, Some(&entry), None);
        let report = compile_plan(plan, &options);
        best = best.min(ms(started.elapsed()));
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }
    println!(
        "PERF project judge prefix: 10 matches over an imported inductive {best:.1}ms \
         (2 modules, closure prefix in every judge call)"
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "front-project",
        "case": "judge_prefix_with_imports",
        "modules": 2,
        "matches": 10,
        "ms": (best * 100.0).round() / 100.0,
    }));
    // ⚠ 同上的教训：原本 `< 3000.0`，**CI 上实测 3104.4ms 判红** ✗ 而本地全绿 ⇒ 假红。
    // 放宽到**只抓数量级**（20×）——真正的守卫是**机器无关**的那些
    // （`--json` 逐字节对拍 / `check-recompile-factor.py` 的计数 / 阶段 1 的 `passes` 计数）。
    assert!(
        best < 60000.0,
        "10 matches over an imported inductive took {best:.1}ms — **order-of-magnitude** regression?"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
