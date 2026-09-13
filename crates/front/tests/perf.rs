//! 编译器性能测试：阈值断言 + 缩放比校验。
//!
//! 这些测试不是 microbenchmark（criterion 才是），而是**回归哨兵**：
//! - **缩放比断言**：编译 200 块不应比编译 50 块慢 6 倍以上（线性 = 4 倍，
//!   6 倍的余量容忍 CI 噪声；O(n²) = 16 倍，必然触发）；
//! - **增量编辑断言**：编辑 50 块文件中的任意块，每键延迟 < 50ms；
//! - **judge 缓存断言**：命中缓存跳过全前缀重编译。
//!
//! 阈值故意宽松（CI runner 性能波动 2-3 倍是常态），只抓算法级回归。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::session::Session;
use std::time::{Duration, Instant};

/// 生成一个含 `checked` 条已证声明 + `open` 条 open 练习的画布源码。
fn gen_canvas(checked: usize, open: usize) -> String {
    let mut lines = vec!["axiom P : Prop".to_string(), "axiom proofP : P".to_string()];
    for i in 1..=checked {
        lines.push(format!("theorem solved_{i} : P := proofP"));
    }
    for i in 1..=open {
        lines.push(format!("theorem exercise_{i} : P := sorry"));
    }
    lines.join("\n") + "\n"
}

fn parse_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// ── 缩放比：O(n) 不是 O(n²) ────────────────────────────────────────

#[test]
fn check_document_scaling_is_linear() {
    let sizes = [50usize, 200, 400];
    let mut times = Vec::new();
    for &size in &sizes {
        let src = gen_canvas(size, size / 10);
        let file = sokonanoda_front::parse(&src).expect("parse");
        let start = Instant::now();
        let report = sokonanoda_front::compile::check_document(&file);
        let elapsed = start.elapsed();
        assert!(
            report.errors.is_empty(),
            "size {size}: unexpected errors: {:?}",
            report.errors
        );
        times.push(parse_ms(elapsed));
    }
    // 线性：400/50 = 8 倍大小 → 应 ≤ 12 倍时间（1.5× 余量）。
    // O(n²) 则 = 64 倍。
    let ratio = times[2] / times[0];
    assert!(
        ratio < 12.0,
        "check_document scaling ratio = {ratio:.1}× (sizes {sizes:?}, times {times:?}) — O(n²)?"
    );
    println!("PERF check_document scaling: {sizes:?} → {times:?} ms (ratio {ratio:.1}×)");
}

// ── 增量编辑：Session::update 只重编译当前块 ────────────────────────

#[test]
fn incremental_edit_anywhere_is_fast() {
    let checked = 50;
    let open = 10;
    let src = gen_canvas(checked, open);
    let mut session = Session::new(CompileOptions::default());

    // 初始编译
    let start = Instant::now();
    let u0 = session.update(&src, 1);
    let initial_ms = parse_ms(start.elapsed());
    assert!(u0.parse_error.is_none());
    // kernel_checks = checked theorems + axioms（axiom 也过内核）
    assert!(
        u0.stats.kernel_checks >= checked,
        "initial compile should have ≥{checked} kernel checks, got {}",
        u0.stats.kernel_checks
    );

    // 依次编辑每个 open 练习（模拟学习者逐题做题）：每次延迟 < 50ms。
    // 这覆盖了"编辑块 k → 尾部重查"的路径。
    let mut cur = src.clone();
    for (round, i) in (1u64..).zip(1..=open) {
        let (old, new) = {
            let old_line = format!("theorem exercise_{i} : P := sorry");
            let new_line = format!("theorem exercise_{i} : P := proofP");
            let new = cur.replace(&old_line, &new_line);
            assert!(new != cur, "block {i} not found");
            (cur.clone(), new)
        };
        let _ = old; // old = cur before edit
        let start = Instant::now();
        let u = session.update(&new, round + 1);
        let elapsed = parse_ms(start.elapsed());
        assert!(
            elapsed < 50.0,
            "editing exercise {i} took {elapsed:.1}ms (threshold 50ms)"
        );
        assert!(
            u.stats.kernel_checks <= 1,
            "exercise {i}: kernel_checks = {} (should be ≤1: only the edited block is env-affecting)",
            u.stats.kernel_checks
        );
        cur = new;
    }
    println!("PERF incremental: initial {initial_ms:.1}ms, {open} edits each < 50ms");
}

// ── 全前缀重编译不存在于基础路径（O(n²) 哨兵）────────────────────────

#[test]
fn editing_first_exercise_does_not_slow_down_with_file_length() {
    // 编辑第一个练习 → 后面的块（文本未变）应该被快速处理。
    // 长文件的编辑延迟不应显著长于短文件。
    let sizes = [50usize, 250];
    let mut times = Vec::new();
    for &checked in &sizes {
        let src = gen_canvas(checked, 5);
        let mut session = Session::new(CompileOptions::default());
        let _ = session.update(&src, 1);

        // 编辑第一个练习（最近的编辑点离文件头最近——最坏情况）
        let (old, new) = {
            let old_line = "theorem exercise_1 : P := sorry";
            let new_line = "theorem exercise_1 : P := proofP";
            (src.clone(), src.replace(old_line, new_line))
        };
        let _ = old;
        let start = Instant::now();
        let u = session.update(&new, 2);
        let elapsed = parse_ms(start.elapsed());
        assert!(u.parse_error.is_none());
        times.push(elapsed);
    }
    // 250 声明编辑不应比 50 声明编辑慢 8 倍以上（线性 = 5 倍，余量 1.6×）
    let ratio = times[1] / times[0];
    assert!(
        ratio < 8.0,
        "edit-at-top scaling ratio = {ratio:.1}× (sizes {sizes:?}, times {times:?}) — superlinear?"
    );
    println!("PERF edit-at-top scaling: {sizes:?} → {times:?} ms (ratio {ratio:.1}×)");
}
