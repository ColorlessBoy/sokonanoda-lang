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
//! ① **甲类闸必须是 0**（它们全是「判不了 ⇒ 换路 / 判否」那类 ✗）：
//!    `sig_overflow` / `sig_arity_clamped`（`MAX_TRACKED`）·
//!    `meta_budget_exhausted`（`fuel`/`MAX_DEPTH` ⇒ **`Tri::No`** ✗）·
//!    `unify_rounds_exhausted`（`MAX_ROUNDS` ⇒ `Tri::Undef` ✓ 正当，但仍要看得见 ✓）。
//!    ⚠ **G-89 收口**（2026-10-07 ✓）：原来的第一项（`PROBE_CAP` 的耗尽计数 ✓）
//!    **已随闸一起删除** ✓ —— Lean 4 没有那道闸 ✓（`Meta/ExprDefEq.lean` 只有
//!    `withIncRecDepth` 那个**全局**递归深度 ✓）⇒ 探查**跑到底** ✓、「耗尽」不存在 ✓
//!    ⇒ 留着就是**空转出口** ✗（永远读 0，而 0 的含义会从「没触发」变成「不存在」✗）。
//!    它的守卫因此**换了形态**（不再是「这个计数必须 0」✗，而是「闸与出口都不许回来」✓）：
//!    `kernel/src/tests/probe.rs`（源码级 ✓）+ `compile::tests::probe_*`（行为 ✓）
//!    + `docs/gaps/repro/G89-probe-cap-exhausted.sh` ✓。
//!
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
//! ⇒ 甲类闸**一次都没触发** ✓（现在没炸 ✓，但闸还在 ⇒ **留闸 + 计数 + 断言** ✓）。
//! ⚠ 别把这条读成「闸没问题」✗ —— 它只说明**当前语料**碰不到 ✓；
//! 数值对齐 Lean 与可配置化是**下一笔**（G-88 的两步走 ✓）。
//! ⚠ 上面那行 `probe_exhausted=0` 是**当时的读数** ✓（现已无此出口 ✗，见 ① 的说明 ✓）。
//!
//! **反向验证**（2026-10-04 实测 ✓）：`meta.rs` 的 `MAX_DEPTH` 临时改成 `1` ⇒
//! 同一条命令量到 `meta_budget_exhausted=59` ⇒ 本用例**判红** ✓ —— 证明那几行
//! `assert_eq!(…, 0)` **咬得住** ✓（不是「全被跳过」那种假绿 ✗）。
//!
//! ## 乙类 4 处（G-91 第二笔 · 2026-10-07 ✓）—— **追加在末尾**（`report()` 7 → 11 ✓；
//! G-89 收口后 **6 + 4 = 10** ✓）
//!
//! 整本课程 `build --json courses/set-theory`（**串行** `SOKONANODA_BUILD_JOBS=1` ✓ ·
//! 构建身份 `d7b00cc42f245faa` ✓）实测：
//!
//! ```text
//! judge_cache_evicted=14056  const_sig_cache_full=0
//! goal_decompose_fallback=0  skeleton_layers_clamped=0
//! ```
//!
//! * **①`judge_cache_evicted=14056`**（`JUDGE_CACHE_CAP=4096` 的 FIFO 淘汰 ✓）——
//!   **这是本笔的发现** ✓：容量**小于整门课的工作集** ⇒ 缓存**部分失效**（天天在发生，
//!   而此前**没有任何读数** ✗）。它**只变慢** ✓：**反向验证**把 `JUDGE_CACHE_CAP`
//!   临时改成 `1` ⇒ `judge_cache_evicted=4678`（单文件）而同一份 `--json`
//!   **0 行不同** ✓（剔 `build.tick`/`build.progress` ✓）⇒ 淘汰**不可能**改判定 ✓。
//!   ⇒ **故意不判 0** ✗（断言 0 会变成假守卫 ✗）；调大容量是**另一笔**（本笔不动 ✗）。
//! * **②`const_sig_cache_full=0`** ✓（常量签名缓存没饱和 ⇒ 判 0 ✓）。
//! * **③`goal_decompose_fallback=0`** ✓ / **④`skeleton_layers_clamped=0`** ✓
//!   —— 这两条属「**变差**」✗（题面精度 / 提示精度）⇒ 判 0 并钉住 ✓。
//!
//! **反向验证（乙类 ✓，2026-10-07 实测）**：
//! ① 删掉 `suggest.rs` 的 `SKELETON_LAYERS_CLAMPED.bump()` ⇒ 复现件 `G91-…sh`
//!    **exit 0**（判红 ✓）+ 常驻断言判红（`（0 → 0）` ✓）；
//! ② 临时探针（跑完即删 ✗）：**3 层**望远镜 ⇒ 差量 **0** ✓、**4 层** ⇒ **1** ✓；
//!    再把 `SKELETON_MAX_LAYERS` 临时改成 `4` ⇒ 4 层也 **0** ✓
//!    ⇒ 它咬的是「**截断**」，不是「走到了这个函数」✓（恰好 3 层是**正常终止** ✗）；
//! ③ 删掉 `walk.rs` 的 `GOAL_DECOMPOSE_FALLBACK.bump()` ⇒ `compile::tests::sorry_in_
//!    argument_position_…` 判红 ✓（见那里的注释 ✓）。
//!
//! ## 为什么放集成测试
//!
//! 计数器是**进程级**的 ✓ ⇒ 放 lib 测试会被并行用例串味 ✗（同 `identity_probe.rs` ✓）。

use sokonanoda::gates;

/// 跑一遍「合成夹具 + 真课程 `unit08`」，返回**十个**闸的**差量** ✓
/// （G-89 收口前是 11 个 —— 探查耗尽那一项已随闸删除 ✓，见文件头 ① ✓）。
fn census() -> [u64; 10] {
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
    let mut out = [0u64; 10];
    for i in 0..10 {
        out[i] = after[i] - before[i];
    }
    out
}

/// **甲类闸必须 0** ✓（`unify_no_progress` 是阳性对照，见文件头 ✓）。
#[test]
fn gate_census_reports_the_real_trigger_counts() {
    let [sig_overflow, sig_clamped, rounds, no_progress, meta_budget, meta_escalated, cache_evicted, const_sig_full, goal_fallback, skeleton_clamped] =
        census();
    println!(
        "PERF gate-census: sig_overflow={sig_overflow} \
         sig_arity_clamped={sig_clamped} unify_rounds_exhausted={rounds} \
         unify_no_progress={no_progress} meta_budget_exhausted={meta_budget} \
         meta_budget_escalated={meta_escalated} judge_cache_evicted={cache_evicted} \
         const_sig_cache_full={const_sig_full} goal_decompose_fallback={goal_fallback} \
         skeleton_layers_clamped={skeleton_clamped}"
    );

    // ① **机制自证**：读得到的数必须真的是刚跑出来的 ✓（`reset()` 之后从 0 起算 ✓）。
    // ⚠ **它不是阳性对照** ✗ —— 阳性对照在文件头的「反向验证」里 ✓（`MAX_DEPTH=1`
    // ⇒ `meta_budget_exhausted` 0 → **59** ⇒ 判红 ✓，2026-10-04 实测 ✓）。
    // 这条只保证「读的是差量、不是别人攒下来的数」✓。
    assert!(
        [sig_overflow, sig_clamped, rounds, meta_budget]
            .iter()
            .all(|n| *n < 1000),
        "读数不像差量（`gates::reset()` 之后应当从小数起算 ✗）：{sig_overflow} \
         {sig_clamped} {rounds} {meta_budget}"
    );
    // **升级（`meta_budget_escalated`）不是缺陷** ✓：撞预算 ⇒ **加大预算重试** ✓
    // ⇒ 只变慢、答案不变 ✓（G-88 真修 ✓，判据 `meta::tests::exhausted_budget_…` ✓）。
    // 这里同样不判它 ✓ —— 但**要读出来**（`> 0` 说明那条路真的在跑 ✓）。
    let _ = (no_progress, meta_escalated);
    // **乙类 ①：判定缓存被挤掉**（`JUDGE_CACHE_CAP = 4096`）—— **故意不判 0** ✗：
    // 它**只变慢** ✓（纯记忆化，重算结论逐字节相同 ✓），**整门课实测 14056 次** ✓
    // （2026-10-07，见文件头）⇒ 断言 0 会变成**假守卫** ✗（那条路本来就允许在跑 ✓）。
    // ⚠ 但它**要读出来** ✓ —— 14056 这个数本身就是这一笔的**发现** ✓：
    // 容量小于整门课的工作集 ⇒ 缓存**部分失效**（下一笔可评估调大 ✓，本笔不动 ✗）。
    let _ = cache_evicted;
    // **乙类 ②③④**：这三条在**本夹具 + unit08** 上是 0 ✓ —— 与整门课实测一致 ✓
    // （`const_sig_cache_full=0` · `goal_decompose_fallback=0` · `skeleton_layers_clamped=0` ✓）。
    // ② 属"只变慢" ✓（缓存饱和）；③④ 属"变差" ✗（显示 / 提示降质）⇒ 后者必须钉住 ✓。
    assert_eq!(
        const_sig_full, 0,
        "**G-91 乙类②**：常量签名缓存（`CAP = 4096`）满 ⇒ **静默停止写入** ✗ —— \
         它只变慢 ✓（结论不变 ✓），但表满了之后**一个字节都不再长** ⇒ 必须看得见 ✗"
    );
    assert_eq!(
        goal_fallback, 0,
        "**G-91 乙类③**：目标分解失败 ⇒ generic 兜底 {goal_fallback} 次 ✗ —— \
         判定不变 ✓，但**题面精度下降** ✗（子洞期望类型不再精确）⇒ 要么修走查、\
         要么在台账里说明它为什么可以接受 ✓"
    );
    assert_eq!(
        skeleton_clamped, 0,
        "**G-91 乙类④**：重启骨架被 `SKELETON_MAX_LAYERS = 3` 截断 {skeleton_clamped} 次 ✗ \
         —— 只掉提示精度 ✓（判定一字不动 ✓），但**没有出口就看不见** ✗"
    );

    // ② **`fuel` / `MAX_DEPTH` 耗尽 ⇒ `Tri::No`** ✗（G-88 本体：判不了 ⇒ **当成否** ✗）。
    assert_eq!(
        meta_budget, 0,
        "**G-88**：预算**升级到底仍弃权**触发了 {meta_budget} 次 ✗ —— 这已经不是\
         「判不了 ⇒ 当成否」✗（那一半**已真修** ✓：撞预算先**加大预算重试** ✓），\
         但它意味着**这条语料真的算不动** ✗ ⇒ 要么调大默认预算、要么查为什么算不完 ✓"
    );
    // ③ **`PROBE_CAP` 耗尽**（G-89）—— **2026-10-07 收口：闸与出口都删了** ✓
    //    ⇒ 这里**没有**这个槽位了 ✓（不是「删断言让它变绿」✗：那个计数已无写入点，
    //    留着就是**永远读 0 的空转出口** ✗）。守卫换了形态 ✓：
    //    `kernel/src/tests/probe.rs`（源码级：闸与出口都不许回来 ✓）+
    //    `compile::tests::probe_*`（行为：该判相等的仍判相等 ✓ / 刚性不等仍判不等 ✓ /
    //    以前会耗尽旧预算的夹具现在仍判相等 ✓）。
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
