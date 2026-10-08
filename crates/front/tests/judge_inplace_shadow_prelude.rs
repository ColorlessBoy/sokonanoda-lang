//! **E4 的判据读数**（2026-10-08）：影子档里那条「prelude 安装期**不比**」的排除分支。
//!
//! ## 这条排除是什么
//!
//! `elab.rs` 的影子档（`InplaceMode::Shadow`）里有一段：`prelude_install_active()` 时
//! **直接返回慢路结果、不比对**（2026-10-06 加的）。理由写在代码注释里：prelude 安装
//! **进行中**做的判定（`And.rec`/`Or.rec`/`Iff.intro`/`Or.elim`）在慢路那边合成重跑时
//! **无法重跑 prelude 本身**（`prefix_src` 为空）⇒ 环境里没有 `And` ⇒
//! `unknown identifier` ⇒ 结构性 `None`（**不是实现 bug**）。
//!
//! ## 为什么它是 E4 的目标
//!
//! `docs/design/course-stdlib.md` §7 要撤掉它：prelude 变成**普通源文本**（E1 ✓）之后，
//! 合成前缀**可以**把 prelude 拼进去 ⇒ 那段安装期的判定**也能比**了 ⇒ 排除不再需要 ✓。
//! 判据就是本文件读的那个数：**撤掉之后必须是 0** ✓。
//!
//! ## E4 第二步已落地（2026-10-08）—— 断言已按约定翻转 ✓
//!
//! 第一步留的出口是"撤掉分支之后**必须**把断言从 `> 0` 改成 `== 0`" ✓ —— 本轮做到：
//! 安装期现在把**本条命令之前的 prelude 源文本**当 `prefix_src` 交出去
//! （`prelude.rs::prelude_prefix_before` ✓）⇒ 慢路**真的能重跑**那段 prelude ⇒
//! 两条路可以逐字比 ⇒ 排除分支删除 ✓。
//!
//! 本文件现在同时钉三件事（缺一不算 ✓）：
//! 1. **排除不再命中**：读数 `== 0` ✓（判据本体）；
//! 2. **影子档真的跑了**（否则 1 是空转 ✗）：`same > 0` ✓；
//! 3. **受信任安装期的两条路逐字相同**：`diff == 0` ✓ —— 出 diff 就是**真分歧**
//!    （不是噪声 ✗）⇒ 停下来定性，**不许**放宽 ✗。
//!
//! ## 为什么是独立测试文件
//!
//! 读数是**进程级**计数器 ⇒ 独立进程 = 天然隔离（同 `g29_closure_recompile.rs` /
//! `judge_inplace_by.rs` 的纪律 ✓）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_report, inplace_shadow_prelude_excluded};
use sokonanoda_front::parse;

/// 一份**必须走 prelude** 的夹具：`And` 的构造/投影都在 prelude 里。
const FIXTURE: &str =
    "theorem t (A B : Prop) (h : A) (k : B) : A \u{2227} B := by\n  exact \u{27e8}h, k\u{27e9}\n";

#[test]
fn the_prelude_install_exclusion_is_gone_and_the_shadow_agrees() {
    // 影子档：两条路都跑、比对文本（`SOKO_JUDGE_INPLACE=shadow`）。
    std::env::set_var("SOKO_JUDGE_INPLACE", "shadow");
    let (_, _, same_before, diff_before) = inplace_report();
    let before = inplace_shadow_prelude_excluded();
    // 单文件（无 import）：prelude 安装 + 本文件的判定都要发生 ✓。
    let file = parse(FIXTURE).expect("夹具 parse");
    let units = [SourceUnit::single("Main", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    let after = inplace_shadow_prelude_excluded();
    let (_, _, same_after, diff_after) = inplace_report();

    // 夹具自检：这份夹具必须**编得过**（否则量的是错误路径 ✗）。
    assert!(
        out.errors.is_empty(),
        "夹具必须编得过（否则量的是错误路径）：{:?}",
        out.errors
    );
    let hits = after - before;
    assert_eq!(
        hits, 0,
        "**E4 的判据本体**：prelude 安装期的排除分支**不许再命中**（实测 {hits} 次）—— \
         它已经被第二步撤掉了 ✓。若这里非 0 ⇒ 排除又回来了（或 `prelude_prefix_before` \
         没接上）✗。"
    );
    // ② 影子档**真的跑了**（否则 ① 是空转 ✗）：本夹具必须让两条路都比过。
    assert!(
        same_after > same_before,
        "影子档必须**真的比过**（`shadow_same` 要增长：{} → {}）—— 否则 ① 的 0 是空转 ✗",
        same_before,
        same_after
    );
    // ③ **受信任安装期**的两条路必须逐字相同（判定红线）。
    assert_eq!(
        diff_after, diff_before,
        "**判定红线**：prelude 安装期两条路的文本必须**逐字相同**（`shadow_diff` {} → {}）—— \
         出 diff 就是**真分歧**（不是噪声 ✗）⇒ 停下来定性，不许放宽 ✗。",
        diff_before, diff_after
    );
}
