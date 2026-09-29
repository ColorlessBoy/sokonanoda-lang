//! **P1-b 第二刀（`by` 路径）的影子档判据** —— `judge_render_type` 的就地兄弟。
//!
//! ⚠ **与反向判据分成两个文件是必须的，不是洁癖** ✗：`SOKO_JUDGE_INPLACE*` 走
//! `OnceLock` **只读一次环境**，而**同一个集成测试文件里的多个 `#[test]` 共享一个
//! 进程**（`cargo test` 默认多线程跑它们）⇒ 两个测试 `set_var` 抢同一个 `OnceLock`，
//! **谁先跑到谁定档**。实测：合并成一个文件时**约 2/3 的运行判红**（另一条拿到
//! `shadow` 档就看不到 `used` 增长）—— 而 `gate` 恰好抽中红的那次 ✗。
//! ⇒ **一个档位一个文件**（各自独立进程 ⇒ 天然隔离 ✓）。
//!
//! 判据（缺一不算）：
//! 1. **判据不空转**：`by` 就地路径**必须被走到**（`INPLACE_BY used` 增长）；
//! 2. **影子档**：两条路的文本**逐字节相同**（`shadow_diff == 0` 且 `same > 0`）
//!    —— 附十的教训：`on` 档的 `--json` 逐字节相同**不足以**证明两条路一致
//!    （第一版多剥一层时 `on` 照样全绿，是影子档把 `diff=37508` 抓出来的）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_by_report, inplace_by_shadow};
use sokonanoda_front::parse;

/// 依赖：`Set` + `mem`（两版只差 `mem` 的**参数类型**）。
const DEP_HEAD: &str = "\
axiom U : Type
axiom Set : Type -> Type
";
const MEM_OK: &str = "axiom mem : {a : Type} -> a -> Set a -> Prop\n";

/// 入口：**必须带 `namespace`**（否则 `canonical_goal == false`
/// ⇒ `canonical_goal_type` 根本不被调用 ⇒ 判据空转 ✗）+ 一个 `by` 块。
const ENTRY: &str = "\
import Dep
infix:50 \" ∈ \" => mem
namespace N
theorem t (A : Set U) (a : U) : a ∈ A := by
  sorry
end N
";

fn dep(mem: &str) -> String {
    format!("{DEP_HEAD}{mem}")
}

fn compile(dep: &str, entry: &str) -> String {
    let dep = parse(dep).expect("dep parses");
    let entry = parse(entry).expect("entry parses");
    let units = [
        SourceUnit::single("Dep", &dep),
        SourceUnit::single("Entry", &entry),
    ];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    format!("errors={:?}\nevents={:?}", out.errors, out.events)
}

/// **影子档**：两条路都跑、比对文本，**返回慢路那一份**（行为零变化）。
#[test]
fn by_inplace_shadow_agrees_on_every_query() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
    std::env::set_var("SOKO_JUDGE_INPLACE_BY", "shadow");

    let _ = compile(&dep(MEM_OK), ENTRY);

    let (used, _fallback) = inplace_by_report();
    let (same, diff) = inplace_by_shadow();
    assert!(
        used > 0,
        "`by` 就地路径一次都没走到（used={used}）⇒ **判据空转**：夹具没踩到 \
         `canonical_goal_type`（它只在 `canonical_goal == true` 且目标是 `by` 块时被调用）"
    );
    assert!(same > 0, "影子档一次都没比对上（same={same}）⇒ 判据空转");
    assert_eq!(
        diff, 0,
        "两条路的文本**必须逐字节相同**（same={same} diff={diff}）—— \
         `by` 的判定决定后续 tactic 步进，分叉会以\"步进不同\"出现 ✗"
    );
}
