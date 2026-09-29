//! **wide 档（P1-b 第一档）的反向判据**：那两个新接线点（`guarded_binder_type` /
//! `solve_prefix_args`）也必须"依赖真变 ⇒ **重算**"，不能把上一趟的结论捞回来。
//!
//! **为什么单独一个文件**：`SOKO_JUDGE_INPLACE*` 只读一次环境 ⇒ 一个进程只能验一个档位；
//! 集成测试各自独立进程 ⇒ 天然隔离 ✓（同 `judge_inplace.rs` / `judge_inplace_on.rs` 的理由）。
//!
//! **夹具怎么踩到 wide 点**（附九 §5 的 recipe）：走**两段式 binder 记法**
//! `∃ x ∈ s, p x` —— binder 不写类型，类型由 **guard `x ∈ s` 反解**
//! ⇒ 正好落在 `binder_notation_operand` → `guarded_binder_type` 那条路上 ✓。
//! ⚠ 一段式（`∃ (x : U), p x`）**踩不到**：写了标注就早退，不进 guard 反解 ✓。
//!
//! 判据（缺一不算）：
//! 1. **判据不空转**：wide 开后 `INPLACE_USED` **必须增长**（否则夹具没踩到接线点）；
//! 2. **反向判据**：换依赖里一个声明的**参数顺序** ⇒ 结论**必须变**。
use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::inplace_report;
use sokonanoda_front::parse;

/// 入口（同一份）：两段式 binder 记法 ⇒ binder 类型靠 guard 反解。
///
/// ⚠ **记法声明必须在入口文件里也写一遍**：本语言的记法是**按文件**在**解析期**生效的
/// （`parse` 不吃 import 的继承；CLI 的多单元路径才带继承表）—— 实测：只写在 `Dep` 里
/// ⇒ 入口在解析期就报 `NotationUnknownSymbol: ∃` ✗。
const ENTRY: &str = "\
import Dep
infix:50 \" ∈ \" => mem
binder_notation \"∃\" => Exists
theorem t (s : Set U) (p : U -> Prop) : ∃ x ∈ s, p x := by sorry
";

/// 依赖的两版：只差 `mem` 的**参数顺序**。
///
/// * `DEP_OK`：`mem : a -> Set a -> Prop` ⇒ guard `x ∈ s` 反解出 `x : U`
///   ⇒ `p x` 成立 ✓；
/// * `DEP_SWAPPED`：`mem : Set a -> a -> Prop` ⇒ 同一个 guard 反解出 `x : Set U`
///   ⇒ `p x` 类型不符 ⇒ **必须报错** ✓（结论可观测地不同）。
const DEP_HEAD: &str = "\
axiom U : Type
axiom Set : Type -> Type
";
const MEM_OK: &str = "axiom mem : {a : Type} -> a -> Set a -> Prop\n";
const MEM_SWAPPED: &str = "axiom mem : {a : Type} -> Set a -> a -> Prop\n";
const DEP_TAIL: &str = "\
inductive Exists (A : Type) (q : A -> Prop) : Prop
  ctor intro (w : A) (hw : q w) : Exists A q
end
";

/// 拼出依赖整份源码。
fn dep(mem: &str) -> String {
    format!("{DEP_HEAD}{mem}{DEP_TAIL}")
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

#[test]
fn wide_inplace_recomputes_when_the_dependency_changes() {
    std::env::set_var("SOKO_JUDGE_INPLACE", "on");
    std::env::set_var("SOKO_JUDGE_INPLACE_WIDE", "1");

    // ① 判据不空转：wide 开后就地路径**必须**被走到（`used` 增长）。
    let before = inplace_report().0;
    let ok = compile(&dep(MEM_OK), ENTRY);
    let mid = inplace_report().0;
    assert!(
        mid > before,
        "wide 档一次都没走到（used {} → {}）⇒ 判据空转：夹具没踩到 `guarded_binder_type` \
         （两段式 binder 记法 `∃ x ∈ s, p x`）",
        before,
        mid
    );

    // ② 反向判据：换依赖里 `mem` 的参数顺序 ⇒ 结论必须变。
    let swapped = compile(&dep(MEM_SWAPPED), ENTRY);
    assert_ne!(
        ok, swapped,
        "换了依赖（`mem` 参数顺序）之后结论没变 ⇒ 判定没有跟着前缀/依赖走 ✗"
    );
}
