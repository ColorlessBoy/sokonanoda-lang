//! **P1-b 第二刀（`by` 路径）的判据** —— `judge_render_type` 的就地兄弟。
//!
//! 这一档与前面几档**不同**：`by` 引擎的判定**决定后续 tactic 步进**
//! ⇒ 两条路一旦分叉，症状是"步进不同"（比 `elab` 路径难定位得多）。
//! ⇒ 附九明写：**影子档是这一档的必需品，不是可选项** ✓。
//!
//! **为什么单独一个文件**：`SOKO_JUDGE_INPLACE*` 只读一次环境 ⇒ 一个进程只能验
//! 一个档位；集成测试各自独立进程 ⇒ 天然隔离 ✓（同 `judge_inplace.rs` 的理由）。
//!
//! 判据（缺一不算）：
//! 1. **判据不空转**：`by` 就地路径**必须被走到**（`INPLACE_BY used` 增长）
//!    —— 否则一个永远走不到的实现也能让"逐字节相同"变绿 ✗；
//! 2. **影子档**：两条路的文本**逐字节相同**（`shadow_diff == 0` 且 `same > 0`）
//!    —— 附十的教训：`on` 档的 `--json` 逐字节相同**不足以**证明两条路一致
//!    （第一版多剥一层时 `on` 照样全绿，是影子档把 `diff=37508` 抓出来的）；
//! 3. **反向判据**：换依赖里一个声明的类型 ⇒ 结论**必须变**
//!    —— 证明是**重算**而不是捞旧结论。
//!
//! ⚠ **夹具必须真的走到 `canonical_goal_type`**：它只在
//! `canonical_goal == true`（文件用了 `namespace`/`open`）**且**目标是个 `by` 块时
//! 才被调用（`run_by_inner` 里那句 `if canonical_goal`）。⇒ 夹具自带
//! `namespace` + 一个 `by` 证明 ✓（实测：不带 namespace 时 `used=0`，当场判红）。
use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_by_report, inplace_by_shadow};
use sokonanoda_front::parse;

/// 依赖：`Set` + `mem`（两版只差 `mem` 的**参数类型**）。
const DEP_HEAD: &str = "\
axiom U : Type
axiom Set : Type -> Type
";
const MEM_OK: &str = "axiom mem : {a : Type} -> a -> Set a -> Prop\n";
/// 换成"第二个参数是 `a`、第三个是 `Set a`"⇒ 同一个目标的意义变了。
const MEM_SWAPPED: &str = "axiom mem : {a : Type} -> Set a -> a -> Prop\n";

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

/// **反向判据**：换依赖里一个声明的类型 ⇒ 结论**必须变**（证明是重算）。
#[test]
fn by_inplace_recomputes_when_the_dependency_changes() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
    std::env::set_var("SOKO_JUDGE_INPLACE_BY", "1");

    let before = inplace_by_report().0;
    let ok = compile(&dep(MEM_OK), ENTRY);
    let mid = inplace_by_report().0;
    assert!(
        mid > before,
        "`by` 就地路径一次都没走到（used {} → {}）⇒ 判据空转",
        before,
        mid
    );

    let swapped = compile(&dep(MEM_SWAPPED), ENTRY);
    assert_ne!(
        ok, swapped,
        "换了依赖（`mem` 的参数类型）之后结论没变 ⇒ 判定没有跟着依赖走 ✗"
    );
}
