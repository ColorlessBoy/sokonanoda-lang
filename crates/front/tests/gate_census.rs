//! **闸类普查的判据**（G-88 / G-89 / G-90 / G-91，2026-10-04 值守派单 ✓）。
//!
//! ## 值守的总规矩（用户原话「这种闸我都不能接受」✓）
//!
//! > **预算 / 尺寸 / 规模耗尽，只允许「变慢」，绝不允许「变错 / 变差」。**
//! > 凡「**超过某个数字就换一条路**」的分支，**必须带一个计数出口**；否则不许进仓。
//! > **先加计数、跑一遍课程报真实触发次数**：**0 也留闸 + 断言**，**> 0 必真修**。
//!
//! 界线 ✓：`判不了 ⇒ 走慢路` **可以** ✓；`判不了 ⇒ 当成否` **不可以** ✗。
//!
//! ## 这条判据钉什么
//!
//! ① **甲类四个闸必须是 0**（它们全是「判不了 ⇒ 换路 / 判否」那类 ✗）：
//!    `probe_exhausted`（`PROBE_CAP`）· `sig_overflow` / `sig_arity_clamped`
//!    （`MAX_TRACKED`）· `meta_budget_exhausted`（`fuel`/`MAX_DEPTH` ⇒ **`Tri::No`** ✗）·
//!    `unify_rounds_exhausted`（`MAX_ROUNDS` ⇒ `Tri::Undef` ✓ 正当，但仍要看得见 ✓）。
//! ② **计数器机制本身不是空转** ✗ —— 证据有两条（都在下面「实测 / 反向验证」里 ✓）：
//!    整本课程上 `unify_no_progress = 26` ✓（那条路径确实在写 ✓）；
//!    **反向验证**：把 `MAX_DEPTH` 临时改成 `1` ⇒ `meta_budget_exhausted` **0 → 59** ⇒ 判红 ✓。
//!
//! ## 实测（2026-10-04 · 整本课程 `build courses/set-theory` · release）
//!
//! ```text
//! fallbacks=0 identity_parses=0 identity_evictions=0
//! probe_exhausted=0 sig_overflow=0 sig_arity_clamped=0
//! unify_rounds_exhausted=0 unify_no_progress=26 meta_budget_exhausted=0
//! ```
//!
//! ⇒ 四个甲类闸**一次都没触发** ✓（现在没炸 ✓，但闸还在 ⇒ **留闸 + 计数 + 断言** ✓）。
//! ⚠ 别把这条读成「闸没问题」✗ —— 它只说明**当前语料**碰不到 ✓；
//! 数值对齐 Lean 与可配置化是**下一笔**（G-88 的两步走 ✓）。
//!
//! **反向验证**（2026-10-04 实测 ✓）：`meta.rs` 的 `MAX_DEPTH` 临时改成 `1` ⇒
//! 同一条命令量到 `meta_budget_exhausted=59` ⇒ 本用例**判红** ✓ —— 证明这四行
//! `assert_eq!(…, 0)` **咬得住** ✓（不是「全被跳过」那种假绿 ✗）。
//!
//! ## 为什么放集成测试
//!
//! 计数器是**进程级**的 ✓ ⇒ 放 lib 测试会被并行用例串味 ✗（同 `identity_probe.rs` ✓）。

use sokonanoda::gates;

/// 跑一遍「合成夹具 + 真课程 `unit08`」，返回六个闸的**差量**。
fn census() -> [u64; 6] {
    gates::reset();
    let before = gates::report().map(|(_, n)| n);

    // 合成夹具（两模块闭包：`namespace` + 继承记法 ✓）——与 `identity_probe.rs` 同形 ✓。
    let dir = std::env::temp_dir().join(format!("soko-gate-census-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录");
    std::fs::write(
        dir.join("Dep.sokonanoda"),
        "namespace Lib\n\ndef twice (α : Type) (f : α → α) (a : α) : α := f (f a)\n\ninfix:50 \" ⊕ \" => Lib.twice\n\ndef dep_val (α : Type) (f : α → α) (a : α) : α := f ⊕ a\n\nend Lib\n",
    )
    .expect("写依赖");
    let entry = dir.join("Entry.sokonanoda");
    std::fs::write(
        &entry,
        "import Dep\n\ndef entry_uses_notation (α : Type) (f : α → α) (a : α) : α := f ⊕ a\n\ndef entry_uses_dep (α : Type) (f : α → α) (a : α) : α := Lib.twice α f a\n",
    )
    .expect("写入口");
    let options = sokonanoda_front::compile::CompileOptions::default();
    let _ = sokonanoda_front::project::compile_project(&entry, None, &options, Some(&dir));
    let _ = std::fs::remove_dir_all(&dir);

    // 真课程（可选：课程仓可分开检出 ✓）—— 闸要在**真实规模**上普查 ✓。
    let course = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    let course_entry = course.join("units/I.3/unit08-images-preimages.sokonanoda");
    if course.join("sokonanoda.toml").is_file() && course_entry.is_file() {
        let _ = sokonanoda_front::project::compile_project(
            &course_entry,
            None,
            &options,
            Some(&course),
        );
    }

    let after = gates::report().map(|(_, n)| n);
    let mut out = [0u64; 6];
    for i in 0..6 {
        out[i] = after[i] - before[i];
    }
    out
}

/// **甲类闸必须 0** ✓（`unify_no_progress` 是阳性对照，见文件头 ✓）。
#[test]
fn gate_census_reports_the_real_trigger_counts() {
    let [probe, sig_overflow, sig_clamped, rounds, no_progress, meta_budget] = census();
    println!(
        "PERF gate-census: probe_exhausted={probe} sig_overflow={sig_overflow} \
         sig_arity_clamped={sig_clamped} unify_rounds_exhausted={rounds} \
         unify_no_progress={no_progress} meta_budget_exhausted={meta_budget}"
    );

    // ① **机制自证**：读得到的数必须真的是刚跑出来的 ✓（`reset()` 之后从 0 起算 ✓）。
    // ⚠ **它不是阳性对照** ✗ —— 阳性对照在文件头的「反向验证」里 ✓（`MAX_DEPTH=1`
    // ⇒ `meta_budget_exhausted` 0 → **59** ⇒ 判红 ✓，2026-10-04 实测 ✓）。
    // 这条只保证「读的是差量、不是别人攒下来的数」✓。
    assert!(
        [probe, sig_overflow, sig_clamped, rounds, meta_budget].iter().all(|n| *n < 1000),
        "读数不像差量（`gates::reset()` 之后应当从小数起算 ✗）：{probe} {sig_overflow} \
         {sig_clamped} {rounds} {meta_budget}"
    );
    let _ = no_progress; // 正当的弃权（`Tri::Undef` ✓），见文件头 ✓ —— 这里不判它 ✓。

    // ② **`fuel` / `MAX_DEPTH` 耗尽 ⇒ `Tri::No`** ✗（G-88 本体：判不了 ⇒ **当成否** ✗）。
    assert_eq!(
        meta_budget, 0,
        "**G-88**：`unify_impl` 的预算耗尽触发了 {meta_budget} 次 ✗ —— 那是\
         「**判不了 ⇒ 当成否**」✗（学习者会看到「解不出来」，而那不是真的无解 ✗）。\
         按值守规矩：**> 0 必真修** ✓（① 可配置化 · ② 放宽到 Lean 的数值 ✓）"
    );
    // ③ **`PROBE_CAP` 耗尽**（G-89）。
    assert_eq!(
        probe, 0,
        "**G-89**：相等性探查预算耗尽 {probe} 次 ✗ —— Lean 4 **没有此物** ✓ ⇒ 终点是\
         **去掉**（不许换数字留着 ✗）；去掉前先看清触发次数 ✓（这个数就是它 ✓）"
    );
    // ④ **`MAX_TRACKED` 丢精度**（G-90）：两处落点都要 0。
    assert_eq!(
        sig_overflow, 0,
        "**G-90**：签名丢精度（望远镜超过 64 位）触发了 {sig_overflow} 次 ✗ —— \
         相关性捷径**静默失效** ✗。终点 = 对齐 Lean 的 `synthInstance.maxSize = 128` ✓"
    );
    assert_eq!(
        sig_clamped, 0,
        "**G-90**：`conv.rs` 的 `k >= MAX_TRACKED` 截断触发了 {sig_clamped} 次 ✗（同上 ✓）"
    );
    // ⑤ **`MAX_ROUNDS` 用光** ⇒ `Tri::Undef` ✓（正当的弃权 ✓，但仍要看得见 ✓）。
    assert_eq!(
        rounds, 0,
        "`unify_all` 的轮数上限用光 {rounds} 次 —— 返回 `Tri::Undef`（**弃权** ✓，\
         不是「判否」✗）⇒ 正当 ✓，但既然 > 0 就要在台账里说明它为什么正当 ✓"
    );
}
