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
//! ## 今天它是什么（**断言当前行为**）
//!
//! **> 0** ✓ —— prelude 安装期的判定全走排除。⚠ 这不是"想要的终态"，而是**起点读数**：
//! E4 第二步（把 prelude 拼进合成前缀 + 撤掉分支）落地时，**必须把下面的断言从
//! `> 0` 改成 `== 0`** ✓ —— 那是本文件唯一的正确出口，**不许**用放宽它的办法变绿 ✗。
//!
//! ## 为什么是独立测试文件
//!
//! 读数是**进程级**计数器 ⇒ 独立进程 = 天然隔离（同 `g29_closure_recompile.rs` /
//! `judge_inplace_by.rs` 的纪律 ✓）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::inplace_shadow_prelude_excluded;
use sokonanoda_front::parse;

/// 一份**必须走 prelude** 的夹具：`And` 的构造/投影都在 prelude 里。
const FIXTURE: &str = "theorem t (A B : Prop) (h : A) (k : B) : A \u{2227} B := by\n  exact \u{27e8}h, k\u{27e9}\n";

#[test]
fn the_prelude_install_exclusion_is_still_hit_today() {
    // 影子档：两条路都跑、比对文本（`SOKO_JUDGE_INPLACE=shadow`）。
    std::env::set_var("SOKO_JUDGE_INPLACE", "shadow");
    let before = inplace_shadow_prelude_excluded();
    // 单文件（无 import）：prelude 安装 + 本文件的判定都要发生 ✓。
    let file = parse(FIXTURE).expect("夹具 parse");
    let units = [SourceUnit::single("Main", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    let after = inplace_shadow_prelude_excluded();

    // 夹具自检：这份夹具必须**编得过**（否则量的是错误路径 ✗）。
    assert!(
        out.errors.is_empty(),
        "夹具必须编得过（否则量的是错误路径）：{:?}",
        out.errors
    );
    let hits = after - before;
    assert!(
        hits > 0,
        "**E4 的起点读数**：prelude 安装期的排除分支今天必须是**被命中**的（实测 {hits} 次）—— \
         若这里已经是 0 ⇒ 说明排除已经被撤掉了（好事 ✓）：那时请把本断言改成 `== 0` 并把 \
         文件头的「断言当前行为」改成「E4 已落地」✓。若它变成 0 而 E4 并没做 ⇒ \
         说明影子档根本没跑到 prelude 安装期 ⇒ 这条读数**空转** ✗（先查 `SOKO_JUDGE_INPLACE` \
         有没有生效）。"
    );
}
