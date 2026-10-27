//! `meta.rs` 的用例（原样搬出 —— `docs-lint` 判据 ⑨：`meta.rs` 在存量基线上
//! **只许减不许增**，而 `tests/` 文件按判据自己的口径**排除在 ⑨ 之外**
//! （「很多小用例聚在一个文件是本仓**有意**的形态」）⇒ 搬出是那条棘轮认可的拆法 ✓。
//!
//! `use super::*` 仍然指 `crate::compile::meta`（本文件是它的**子模块**）⇒
//! 私有项的可见性与搬出前**逐字相同** ✓。
use super::*;

fn ident(name: &str) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span: Span::default(),
    }
}

fn app(f: Expr, a: Expr) -> Expr {
    Expr::App {
        fun: Box::new(f),
        arg: Box::new(a),
        explicit_spine: false,
        span: Span::default(),
    }
}

fn arrow(d: Expr, c: Expr) -> Expr {
    Expr::Arrow {
        domain: Box::new(d),
        codomain: Box::new(c),
        span: Span::default(),
    }
}

/// 没有 delta 兜底的 ctx（单测里显式给展开函数的那条另测）。
/// ⚠ **B2 起 store 由调用方持有** ✓（`MetaCtx` 借用它 ✓ ⇒ 生命周期在调用方的栈上 ✓）。
fn ctx_with<'a>(unfold: &'a dyn Fn(&Expr) -> Expr) -> MetaCtx<'a> {
    // ⚠ **单测专用** ✓：store 借自一个 `Box::leak`（`'static` ✓ ⇒ 能coerce 到 `'a` ✓）
    // ⇒ **18 个调用点一个字都不用改** ✓（每次调用泄漏一个空 store ✓ —— 只发生在测试里 ✓）。
    let store: &'a mut MetaStore = Box::leak(Box::new(MetaStore::default()));
    MetaCtx::new(unfold, store)
}

fn no_unfold(e: &Expr) -> Expr {
    e.clone()
}

/// `Type`（= `Sort 1`）这个**类型**——注意它是 `Expr::Sort`，不是 `Ident`（实测踩过）。
fn ty_sort() -> Expr {
    Expr::Sort {
        sort: crate::ast::SortKind::Type,
        span: Span::default(),
    }
}

/// 期望**解出**（M3 起 `discharge` 返回通道枚举）。
fn solved(m: &mut MetaCtx, ids: &[MetaId]) -> Vec<Expr> {
    match m.discharge(ids) {
        MetaSolve::Solved(v) => v,
        other => panic!("期望解出，实际 {other:?}"),
    }
}

/// 期望**失败**，返回通道。
fn failed(m: &mut MetaCtx, ids: &[MetaId]) -> MetaSolve {
    match m.discharge(ids) {
        MetaSolve::Solved(v) => panic!("期望失败，实际解出 {v:?}"),
        other => other,
    }
}

/// 结构分解：`Set ?α ≟ Set Nat` ⇒ `α := Nat`；`?α → ?β ≟ Nat → Bool` ⇒ 两个都解出。
#[test]
fn decomposes_app_and_arrow_on_both_sides() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let set_a = app(ident("Set"), m.meta_expr(a));
    assert_eq!(m.unify(&set_a, &app(ident("Set"), ident("Nat"))), Tri::Yes);
    assert_eq!(m.value(a), Some(ident("Nat")));

    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let c = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let tpl = arrow(m.meta_expr(b), m.meta_expr(c));
    let act = arrow(ident("Nat"), ident("Bool"));
    assert_eq!(m.unify(&tpl, &act), Tri::Yes);
    assert_eq!(m.value(b), Some(ident("Nat")));
    assert_eq!(m.value(c), Some(ident("Bool")));
}

/// occurs check：`?α ≟ Set ?α` 必须被拒（否则就是无限类型）。
#[test]
fn occurs_check_bites() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let cyclic = app(ident("Set"), m.meta_expr(a));
    assert_eq!(m.unify(&m.meta_expr(a).clone(), &cyclic), Tri::No);
    assert_eq!(m.assign(a, cyclic), Err(MetaErr::Occurs));
    assert_eq!(m.value(a), None);
}

/// **type-occurs**：解里提到的另一个元变量，其**声明类型**里不许出现自己。
#[test]
fn type_occurs_check_bites() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    // b 的声明类型是 `Set ?a` ⇒ 把 `?b` 赋给 `?a` 会经类型成环
    let b = m.fresh(app(ident("Set"), m.meta_expr(a)), MetaKind::Natural, vec![]);
    assert_eq!(m.assign(a, m.meta_expr(b)), Err(MetaErr::TypeOccurs));
}

/// **作用域**与 occurs **分开报**（不同 bug、不同消息）。
#[test]
fn scope_check_is_separate_from_occurs() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(
        ident("Type"),
        MetaKind::Natural,
        vec!["\u{0}soko_p1".to_string()],
    );
    // 值里提到**更晚**的望远镜参数 ⇒ Scope（不是 Occurs）
    assert_eq!(
        m.assign(a, app(ident("Set"), ident("\u{0}soko_p1"))),
        Err(MetaErr::Scope)
    );
}

/// 元变量 vs 元变量：**直接赋值**（`?α := ?β`，链式由 `zonk` 追到底）—— 与 Lean 同判，
/// 不是"两个都不知道所以弃权"。
#[test]
fn meta_vs_meta_assigns_a_chain() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let (ea, eb) = (m.meta_expr(a), m.meta_expr(b));
    assert_eq!(m.unify(&ea, &eb), Tri::Yes);
    assert_eq!(m.value(a), Some(eb.clone()), "α 的值就是 ?β（链式）");
    m.assign(b, ident("Nat")).expect("β := Nat");
    assert_eq!(solved(&mut m, &[a, b]), vec![ident("Nat"), ident("Nat")]);
}

/// **G-88 的判据（2026-10-04 ✓）：预算耗尽 ⇒ 「加大预算重试」✓，绝不「当成否」✗。**
///
/// **反向验证**（把修复撤掉 ⇒ 本用例判红 ✓）：旧实现在 `unify_impl` 里
/// `if fuel == 0 || depth >= 深度上限 { return Tri::No }` ⇒ 下面第一条断言
/// （`!= Tri::No`）当场判红 ✗ —— 那正是「学习者写长一点的证明 ⇒ 系统说解不出来」✗。
///
/// 三条一起断言（缺一条就会被"碰巧"骗过去 ✓）：
/// ① 预算**故意调到 1** ⇒ 结论**仍然不是 `No`** ✗（判不了 ≠ 判否 ✓）；
/// ② 该成立的约束**仍然判 `Yes`** ✓（升级把活干完了 ⇒ **答案不变、只是变慢** ✓）；
/// ③ 该冲突的约束**仍然判 `No`** ✓（升级不能把"真冲突"洗成"不知道"✗）。
#[test]
fn exhausted_budget_escalates_instead_of_judging_false() {
    let before_escalated = sokonanoda::gates::META_BUDGET_ESCALATED.get();
    let before_exhausted = sokonanoda::gates::META_BUDGET_EXHAUSTED.get();

    // ① + ②：`?a` 与 `Nat`（同一形状族、可赋值）⇒ 预算再小也必须判 **Yes** ✓。
    let mut m = ctx_with(&no_unfold);
    m.fuel = 1;
    m.max_depth = 1;
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    // ⚠ **必须真的需要不止一步** ✗：`?a ≟ Nat` 一步就完了 ⇒ 撞不到预算 ⇒
    // 判据**空转** ✗（第一版就是这么假绿的 ✓）。用 `Set ?a ≟ Set Nat` ✓
    // —— 它要下钻一层才碰到元变量 ✓。
    let ea = app(ident("Set"), m.meta_expr(a));
    let verdict = m.unify(&ea, &app(ident("Set"), ident("Nat")));
    assert_ne!(
        verdict,
        Tri::No,
        "**G-88**：预算耗尽被判成「不成立」✗ —— 那是「判不了 ⇒ 当成否」✗ \
             （正确行为 = 加大预算重试 ✓，见 `unify_impl`）"
    );
    assert_eq!(
        verdict,
        Tri::Yes,
        "预算**故意调到 1** ⇒ 升级必须把这条该成立的约束算完 ✓（答案不变、只是变慢 ✓）"
    );
    assert_eq!(
        m.value(a),
        Some(ident("Nat")),
        "升级之后 ?a 必须真的解出来 ✓"
    );
    assert!(
        sokonanoda::gates::META_BUDGET_ESCALATED.get() > before_escalated,
        "升级计数没动 ⇒ 这条用例根本没撞过预算（守卫空转 ✗）"
    );

    // ③：**刚性冲突**（两个不同的刚性常量、没有可赋值的元变量）⇒ 仍必须是 `No` ✓。
    let mut m2 = ctx_with(&no_unfold);
    m2.fuel = 1;
    m2.max_depth = 1;
    assert_eq!(
        m2.unify(&ident("Nat"), &ident("Bool")),
        Tri::No,
        "刚性冲突在升级之后必须**仍然**判否 ✓（升级不许把真冲突洗成「不知道」✗）"
    );
    let _ = before_exhausted;
}

/// **G-88 判据①（2026-10-07 ✓）：旋钮真的接到求解器上** ——
/// `MetaCtx::new` 的两个预算**取自登记表** ✓（`gates::limits` ✓），
/// 不是这个文件里的本地常量 ✗（本地常量**已经删掉** ✓ ⇒ 这条断言不可能被"本地默认值"糊过去 ✓）。
#[test]
fn budget_knobs_are_wired_from_the_registry() {
    let m = ctx_with(&no_unfold);
    assert_eq!(
        m.fuel,
        sokonanoda::gates::limits::max_heartbeats(),
        "fuel 必须取自 SOKO_LIMIT_MAX_HEARTBEATS 那一项 ✓"
    );
    assert_eq!(
        m.max_depth,
        sokonanoda::gates::limits::max_rec_depth(),
        "max_depth 必须取自 SOKO_LIMIT_MAX_REC_DEPTH 那一项 ✓"
    );
    assert_eq!(m.escalations, 0, "新 ctx 的升级计数从 0 起 ✓");
}

/// **G-88 判据②（2026-10-07 ✓）：默认值 = Lean 的数值** ✓ ——
/// `synthInstance.maxHeartbeats` = **20000** ✓ · `maxRecDepth` = **512** ✓
/// （出处逐行写在 `crates/kernel/src/gates.rs` 的常量文档里 ✓）。
///
/// ⚠ 断言**常量**、不读环境 ✗：`SOKO_LIMIT_*` 是**进程级**的（`OnceLock` 只读一次 ✓），
/// 读环境会让这条判据被外部开关污染 ✗（见 `scripts/check-test-env-isolation.py` ✓）。
#[test]
fn budget_defaults_are_the_lean_numbers() {
    assert_eq!(
        sokonanoda::gates::limits::DEFAULT_MAX_HEARTBEATS,
        20000,
        "Lean `synthInstance.maxHeartbeats`（Meta/SynthInstance.lean:20）= 20000 ✓"
    );
    assert_eq!(
        sokonanoda::gates::limits::DEFAULT_MAX_REC_DEPTH,
        512,
        "Lean `maxRecDepth`（Util/RecDepth.lean:15 ⇒ Init/Prelude.lean:4760）= 512 ✓"
    );
    assert_eq!(
        sokonanoda::gates::limits::DEFAULT_MAX_ESCALATIONS,
        6,
        "Lean 无对应物（耗尽即报错）⇒ 保持 6 ✓"
    );
}

/// **G-88 判据①的另一半（2026-10-07 ✓）：升级额度也用光 ⇒ 弃权 ✓，绝不判否** ✗。
///
/// 这一档正是 `SOKO_LIMIT_MAX_ESCALATIONS=0` 的语义 ✓（一撞预算就弃权 ✓）。
/// 与上面那条**成对**：那条钉"还有额度 ⇒ 算完 ✓"，这条钉"没额度 ⇒ 弃权 ✓"。
#[test]
fn exhausted_escalations_abstain_instead_of_judging_false() {
    let before = sokonanoda::gates::META_BUDGET_EXHAUSTED.get();
    let mut m = ctx_with(&no_unfold);
    // 预算拧到极小 + 升级额度**预先用光** ✓。
    m.fuel = 1;
    m.max_depth = 1;
    m.escalations = sokonanoda::gates::limits::max_escalations();
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    // ⚠ 同上面那条：必须真的**需要不止一步**（`Set ?a ≟ Set Nat` 要下钻一层 ✓），
    // 否则撞不到预算 ⇒ 判据空转 ✗。
    let ea = app(ident("Set"), m.meta_expr(a));
    let verdict = m.unify(&ea, &app(ident("Set"), ident("Nat")));
    assert_ne!(
        verdict,
        Tri::No,
        "**G-88**：升级额度用光仍撞预算 ⇒ 必须**弃权** ✓，绝不许判否 ✗"
    );
    assert_eq!(
        verdict,
        Tri::Undef,
        "弃权 = `Tri::Undef` ✓（= 「判不了 ⇒ 走慢路」✓）"
    );
    assert!(
        sokonanoda::gates::META_BUDGET_EXHAUSTED.get() > before,
        "弃权计数没动 ⇒ 这条用例根本没走到弃权路径（守卫空转 ✗）"
    );
}

/// 待定约束：形状对不上 + 有**埋着的**未解元变量 ⇒ 弃权；`unify_all` **无净进展就停**
/// （不是无界工作队列 ⇒ 不会挂死 ✓）。
#[test]
fn postponed_stops_without_progress() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let (ea, eb) = (m.meta_expr(a), m.meta_expr(b));
    // `Set ?α ≟ (?β → Prop)`：(App, Arrow) 不分解、没有展开兜底 ⇒ 弃权
    let l = app(ident("Set"), ea);
    let r = arrow(eb, ident("Prop"));
    assert_eq!(m.unify(&l, &r), Tri::Undef);
    assert_eq!(m.store.postponed.len(), 1);
    assert_eq!(m.unify_all(), Tri::Undef, "无净进展 ⇒ 停（不挂死）");
}

/// 刚性冲突：两个不同头的常量 ⇒ `No`（且不进待定队列）。
#[test]
fn rigid_clash_is_no() {
    let mut m = ctx_with(&no_unfold);
    assert_eq!(m.unify(&ident("Nat"), &ident("Bool")), Tri::No);
    assert!(m.store.postponed.is_empty());
}

/// **S13 硬约束**：两条约束**语法不同形但 defeq 一致** ⇒ delta 兜底必须救回来
/// （不接它就会把今天绿的形状误判成冲突 ✗ —— M0 §2.1 第 1 条）。
#[test]
fn delta_fallback_keeps_defeq_agreement() {
    // `Set Nat` ↝ `Nat → Prop`（模拟 `Set` 是 def 的展开）
    let unfold = |e: &Expr| -> Expr {
        if let Expr::App { fun, arg, .. } = e {
            if let Expr::Ident { name, .. } = fun.as_ref() {
                if name == "Set" {
                    return arrow((**arg).clone(), ident("Prop"));
                }
            }
        }
        e.clone()
    };
    let mut m = ctx_with(&unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    m.assign(a, ident("Nat")).expect("α := Nat");
    // 第二条约束：`?α → Prop ≟ Set Nat`（左 Arrow、右 App ⇒ 必须靠展开）
    let tpl = arrow(m.meta_expr(a), ident("Prop"));
    assert_eq!(m.unify(&tpl, &app(ident("Set"), ident("Nat"))), Tri::Yes);
}

/// defaulting = E19 的规则：同形已解兄弟 ⇒ 借它的值；**一个兄弟都没有 ⇒ 仍失败**（不猜）。
#[test]
fn default_unresolved_follows_the_e19_rule() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    assert!(
        matches!(failed(&mut m, &[a, b]), MetaSolve::Unsolved),
        "两位都空 ⇒ 不许猜"
    );
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    m.assign(b, ident("Nat")).expect("β := Nat");
    let out = solved(&mut m, &[a, b]);
    assert_eq!(out, vec![ident("Nat"), ident("Nat")]);
}

/// **B2-P4a**：`SyntheticOpaque` 是**保留种类**（洞的语义）⇒ defaulting **不许**替它选值。
///
/// 反向验证：把 `default_unresolved` 里那条 `kind == SyntheticOpaque` 的 `continue` 删掉
/// ⇒ 本条**当场红**（`?hole` 会被借成 `Nat` ⇒ 出口从 `Unsolved` 变成 `Solved` ✗）。
///
/// ⚠ 为什么这条必须在 defaulting 这一层（而不是出口）：内核对 `SyntheticOpaque` 的赋值
/// **一律拒**（`mvar.rs:183`，Lean 同款）⇒ 前端替它猜值 = 与内核的 kind 纪律互相矛盾，
/// 而且会把"洞"报成"已解出"（用户可见的错）。
#[test]
fn default_unresolved_skips_synthetic_opaque() {
    let mut m = ctx_with(&no_unfold);
    let nat = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let hole = m.fresh(ident("Type"), MetaKind::SyntheticOpaque, vec![]);
    m.assign(nat, ident("Nat")).expect("α := Nat");
    // 同形兄弟在（`α := Nat`）⇒ 若没有那条 skip，`?hole` 会被借成 `Nat`。
    let _ = m.default_unresolved();
    assert_eq!(
        m.unsolved(),
        vec![hole],
        "`SyntheticOpaque` 必须留在未解（defaulting 只借 `Natural` 位）"
    );
    assert_eq!(m.value(hole), None, "洞不许被默认成具体项");
}

/// 出口 zonk：解出来的值里**不许留** `\0soko_m*`（设计红线 1）。
#[test]
fn zonk_leaves_no_residue() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    m.assign(b, ident("Nat")).expect("β := Nat");
    m.assign(a, m.meta_expr(b)).expect("α := ?β（链式）");
    let out = solved(&mut m, &[a, b]);
    assert_eq!(out, vec![ident("Nat"), ident("Nat")]);
    assert!(!has_unassigned_meta(&out[0]));
}

/// 预算：**耗尽不许改变结论** ✓（G-88 真修 ✓），**也不许挂死** ✓。
///
/// ⚠ **这条用例原先钉的正是那个缺陷** ✗：旧断言是
/// `assert_eq!(m.unify(&Nat, &Nat), Tri::No)` —— 即「`fuel = 0` ⇒ 两个**一模一样**的
/// 常量被判**不相等**」✗✗。它是 G-88「判不了 ⇒ 当成否」**最干净的现场** ✓：
/// 结论与预算无关，而旧实现让它有关 ✗。
///
/// 现在（2026-10-04 ✓）：撞预算 ⇒ **加大预算重试** ✓ ⇒ 结论回到 `Yes` ✓
/// （**答案对** ✓，只是多花步数 ✓ = 「只允许变慢」✓）；
/// 「不许挂死」这条**照旧** ✓ —— 升级次数有上界（`limits::max_escalations()` ✓）。
#[test]
fn fuel_bounds_the_work() {
    let mut m = ctx_with(&no_unfold);
    m.fuel = 0;
    assert_eq!(
        m.unify(&ident("Nat"), &ident("Nat")),
        Tri::Yes,
        "**G-88**：`Nat ≟ Nat` 在 `fuel = 0` 下被判「不相等」✗ —— \
             预算耗尽**不许改变结论** ✓（旧实现正是这样 ✗，而本用例原先把它当**期望**钉住了 ✗）"
    );
    assert!(
        m.escalations <= sokonanoda::gates::limits::max_escalations(),
        "升级次数必须有上界 ✓（没上界就不叫「只变慢」了 ✗）：{}",
        m.escalations
    );
}

/// **M3 的 sort/kind 表**（三值：`None` = 不知道 ⇒ 放行）。
#[test]
fn sort_of_value_is_three_valued() {
    let prop = Expr::Sort {
        sort: crate::ast::SortKind::Prop,
        span: Span::default(),
    };
    let ty = Expr::Sort {
        sort: crate::ast::SortKind::Type,
        span: Span::default(),
    };
    let s3 = Expr::Sort {
        sort: crate::ast::SortKind::Sort(3),
        span: Span::default(),
    };
    let lvl = Expr::Sort {
        sort: crate::ast::SortKind::Level("u".to_string()),
        span: Span::default(),
    };
    assert_eq!(sort_of_value(&prop), Some(1), "`Prop : Type 0`");
    assert_eq!(sort_of_value(&ty), Some(2), "`Type 0 : Type 1`");
    assert_eq!(sort_of_value(&s3), Some(4), "`Sort 3 : Sort 4`");
    assert_eq!(sort_of_value(&lvl), None, "`Level(u)` ⇒ 不知道");
    assert_eq!(
        sort_of_value(&ident("Nat")),
        None,
        "常量 ⇒ 不知道（保守放行）"
    );
}

/// **M3 的闸门**：`?α : Type`（= `Sort 1`）拿到 `Type 0`（`Sort 2`）⇒ **拒**（否则就是 kind 错）。
#[test]
fn sort_check_rejects_a_kind() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ty_sort(), MetaKind::Natural, vec![]);
    let kind = ty_sort();
    assert_eq!(m.assign(a, kind.clone()), Err(MetaErr::Kind));
    assert_eq!(m.value(a), None, "拒了就不许写进去");
    // 走 `unify` 的路径也一样，且**通道**归因到 `Kind`
    let ea = m.meta_expr(a);
    assert_eq!(m.unify(&ea, &kind), Tri::No);
    assert!(matches!(failed(&mut m, &[a]), MetaSolve::Kind));
}

/// 保守方向：**不知道就不拒**（`Nat` 是常量，语法上看不出 sort ⇒ 放行 ⇒ 不可能假拒绝）。
#[test]
fn sort_check_passes_when_unknown() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ty_sort(), MetaKind::Natural, vec![]);
    assert_eq!(m.assign(a, ident("Nat")), Ok(()));
}

/// **M3 的三通道**：刚性冲突归因到 `Clash`（与 `Kind`、`Unsolved` 分开）。
#[test]
fn channel_separates_clash_from_unsolved() {
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    assert_eq!(m.unify(&ident("Nat"), &ident("Bool")), Tri::No);
    assert!(matches!(failed(&mut m, &[a]), MetaSolve::Clash));
    let mut m = ctx_with(&no_unfold);
    let b = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    assert!(matches!(failed(&mut m, &[b]), MetaSolve::Unsolved));
}

/// **IA-4 B2 第 2 步（2026-10-05 ✓）**：出口的**放宽版** ✓ ——
/// 判据两条，**都必须能咬住** ✗（`AGENTS.md`：咬不住的守卫等于没有 ✓）：
/// ① 未解的元变量**连 id 一起返回** ✓（严格版对同一输入**必须**报 `Unsolved` ✗ —— 对照组 ✓）；
/// ② **已解**的照旧 `zonk` 成值 ✓（放宽不等于不 zonk ✗）。
///
/// ⚠ **反向验证** ✓：把 `discharge_keep_unsolved` 里的 `still.push` 去掉 ⇒ 第 ① 条**判红** ✗。
#[test]
fn discharge_keep_unsolved_returns_live_metavariables() {
    // ① **未解** ⇒ 连 id 一起返回 ✓；同输入的**严格版**报 `Unsolved` ✗（对照 ✓）。
    let mut m = ctx_with(&no_unfold);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    let (out, still) = m
        .discharge_keep_unsolved(&[a])
        .expect("放宽版**不许**因为「还有未解」就失败 ✗");
    assert_eq!(still, vec![a], "未解的元变量必须**连 id 一起返回** ✓");
    assert!(
        has_unassigned_meta(&out[0]),
        "未解的那一位必须**原样**是元变量名 ✓（B2 靠它把期望类型带出这次求解 ✓）"
    );
    let mut m2 = ctx_with(&no_unfold);
    let b = m2.fresh(ident("Type"), MetaKind::Natural, vec![]);
    assert!(
        matches!(failed(&mut m2, &[b]), MetaSolve::Unsolved),
        "**严格版**对同一输入必须报 `Unsolved` ✗（否则这条对照是空转 ✗）"
    );
    // ② **已解** ⇒ zonk 成值 ✓、`still` 为空 ✓。
    let mut m3 = ctx_with(&no_unfold);
    let c = m3.fresh(ident("Type"), MetaKind::Natural, vec![]);
    m3.assign(c, ident("Nat")).expect("赋一个无元变量的值 ✓");
    let (out3, still3) = m3.discharge_keep_unsolved(&[c]).expect("已解 ⇒ 必成功 ✓");
    assert!(still3.is_empty(), "已解的不该出现在 `still` 里 ✓");
    assert!(!has_unassigned_meta(&out3[0]), "已解的必须被 zonk 成值 ✓");
}

/// **U1 第 1 片（2026-10-05）**：层元变量的**编码**与 **occurs** ✓。
///
/// 判据四条，**都要能咬** ✗（`AGENTS.md`：咬不住的守卫等于没有 ✓）：
/// ① 编码与**项**元变量**不互判** ✓（两个前缀不同 ✓ —— 混了会让 `zonk` 把层名当项名 ✗）；
/// ② `level_id_of` **往返**一致 ✓；
/// ③ occurs 的**词边界** ✓：`u` 命中 ✓、`u1`/`uu` **不**命中 ✗（没有边界 ⇒ `u` 会误伤 `u1` ✗）；
/// ④ occurs 在**复合层文本**里也命中 ✓（`max u v` / `u+1` ✓）。
///
/// ⚠ **反向验证**：把 `level_occurs_in_text` 的 `before_ok`/`after_ok` 去掉（无条件命中）
/// ⇒ 第 ③ 条**判红** ✗。
#[test]
fn level_meta_encoding_and_occurs_bite() {
    // ① 两类元变量**不互判** ✓。
    assert!(is_level_name(&level_name(3)), "层名必须被认成层名 ✓");
    assert!(
        !is_meta_name(&level_name(3)),
        "层名**不许**被认成项元变量 ✗"
    );
    assert!(
        !is_level_name(&meta_name(3)),
        "项名**不许**被认成层元变量 ✗"
    );
    // ② 往返 ✓。
    assert_eq!(level_id_of(&level_name(7)), Some(7), "id 往返必须一致 ✓");
    assert_eq!(level_id_of("u"), None, "普通层名不是层元变量 ✓");
    // ③ 词边界 ✓。
    let u = level_name(5);
    assert!(level_occurs_in_text(5, &u), "裸名必须命中 ✓");
    assert!(
        level_occurs_in_text(5, &format!("max {u} v")),
        "复合文本里必须命中 ✓"
    );
    assert!(
        level_occurs_in_text(5, &format!("{u}+1")),
        "层级算术里必须命中 ✓"
    );
    assert!(
        !level_occurs_in_text(5, &format!("{u}1")),
        "`u1` **不许**命中 ✗（没有词边界就会误伤 ✓）"
    );
    assert!(
        !level_occurs_in_text(5, &format!("x{u}")),
        "`xu` **不许**命中 ✗"
    );
    assert!(!level_occurs_in_text(5, "v"), "别的层名不命中 ✓");
    // ④ 不同的 id 互不命中 ✓。
    assert!(
        !level_occurs_in_text(6, &u),
        "id 5 的文本不许被 id 6 命中 ✗"
    );
}

/// **U1 第 2 片（2026-10-05）**：宇宙约束的**存储**（§2.9 的 `ULe`/`UEq`/`ULub` 三种）✓。
///
/// 判据三条，**都要能咬** ✗：
/// ① 三种约束**互不相等** ✓（`Le{1,2}` ≠ `Eq{1,2}` ≠ `Lub{0,1,2}` ✗ ——
///    混了会让惰性检查按错的规则判 ✗）；
/// ② `MetaStore` 的 `univ` **默认空** ✓（零行为变化的前提 ✓）；
/// ③ **与项约束分表** ✓：压一条宇宙约束**不改变** `postponed` 的长度 ✗
///    （§4 的 #6 行要求分表 ✓；混在一起会让层约束被 `unify_all` **过早**求解 ✗）。
///
/// ⚠ **反向验证**：把 `univ` 与 `postponed` 合成一个 `Vec` ⇒ 第 ③ 条**判红** ✗。
#[test]
fn univ_constraints_are_stored_separately() {
    // ① 三种约束互不相等 ✓。
    let le = UnivConstraint::Le { lhs: 1, rhs: 2 };
    let eq = UnivConstraint::Eq { lhs: 1, rhs: 2 };
    let lub = UnivConstraint::Lub { out: 0, a: 1, b: 2 };
    assert_ne!(le, eq, "`Le` 与 `Eq` 不许相等 ✗");
    assert_ne!(le, lub, "`Le` 与 `Lub` 不许相等 ✗");
    assert_ne!(eq, lub, "`Eq` 与 `Lub` 不许相等 ✗");
    // ② 默认空 ✓。
    let store = MetaStore::default();
    assert!(store.univ.is_empty(), "`univ` 默认必须是空 ✓");
    assert!(store.postponed.is_empty(), "`postponed` 默认必须是空 ✓");
    // ③ 分表 ✓：宇宙约束**不进** `postponed` ✓。
    let mut store = MetaStore::default();
    store.univ.push(le);
    store.univ.push(eq);
    store.univ.push(lub);
    assert_eq!(store.univ.len(), 3, "三条都该在 `univ` 里 ✓");
    assert!(
        store.postponed.is_empty(),
        "宇宙约束**不许**混进 `postponed` ✗（§4 #6 要求分表 ✓）"
    );
}

/// **U1 第 3 片（2026-10-05）**：层元变量的**赋值**与 **zonk**（对齐 Lean `solve` 的核心档）✓。
///
/// 判据四条，**都要能咬** ✗：
/// ① **occurs 命中 ⇒ 拒** ✓（`?u := ?u + 1` 必须被挡 ✗ —— 没有这条 `zonk` 会不终止/给错值 ✓）；
/// ② 正常赋值 ⇒ `Ok` ✓，且 `level_zonk_text` **代进去** ✓；
/// ③ **链式**赋值 ✓（`?u := ?v` 且 `?v := 0` ⇒ zonk 到 `0` ✓）；
/// ④ **未赋值的保持原样** ✓（zonk 不许把没赋值的层名抹掉 ✗）。
///
/// ⚠ **反向验证**：把 `level_assign` 的 occurs 闸去掉 ⇒ 第 ① 条**判红** ✗。
#[test]
fn level_assign_and_zonk_bite() {
    let mut vals: Vec<Option<String>> = Vec::new();
    let u = level_name(0);
    let v = level_name(1);
    // ① occurs 命中 ⇒ 拒 ✓。
    assert!(
        level_assign(&mut vals, 0, &format!("{u}+1")).is_err(),
        "`?u := ?u+1` 必须被拒 ✗（occurs 闸 ✓）"
    );
    assert!(
        vals.is_empty() || vals[0].is_none(),
        "被拒的赋值**不许**写进表 ✗"
    );
    // ② 正常赋值 + zonk ✓。
    assert!(level_assign(&mut vals, 0, "2").is_ok(), "普通赋值该成功 ✓");
    assert_eq!(level_zonk_text(&vals, &u), "2", "赋过的层名必须被代进去 ✓");
    // ③ 链式 ✓。
    assert!(
        level_assign(&mut vals, 1, &u).is_ok(),
        "`?v := ?u` 该成功 ✓"
    );
    assert_eq!(
        level_zonk_text(&vals, &v),
        "2",
        "链式赋值必须 zonk 到底 ✓（`?v := ?u` 且 `?u := 2` ⇒ `2` ✓）"
    );
    // ④ 未赋值的保持原样 ✓。
    let w = level_name(9);
    assert_eq!(
        level_zonk_text(&vals, &w),
        w,
        "没赋值的层名**不许**被抹掉 ✗"
    );
}

/// **U1 第 4 片（2026-10-05）**：宇宙约束的**惰性批量检查**（§2.9「对齐 Coq」✓）。
///
/// 判据五条，**都要能咬** ✗：
/// ① 两侧都有值且**成立** ⇒ `Ok` ✓（`Eq{0,1}` 都 = 2 ✓；`Le{0,1}` 1 ≤ 2 ✓）；
/// ② 两侧都有值且**不成立** ⇒ **`Clash`** ✗（`Eq` 2 vs 3 ✓；`Le` 3 vs 2 ✓）；
/// ③ 任一侧**没赋值** ⇒ **`Pending`** ✓ —— ⚠ **不是 Clash** ✗
///    （报成失败会**假红** ✗ —— 这正是「惰性」的意义 ✓）；
/// ④ `Lub` ⇒ **`Pending`** ✓（它要**算 max** ✗，本片弃权 ✓）；
/// ⑤ 空约束表 ⇒ `Ok` ✓。
///
/// ⚠ **反向验证**：把「没赋值 ⇒ `Pending`」改成「⇒ `Clash`」⇒ 第 ③ 条**判红** ✗。
#[test]
fn univ_check_is_lazy_and_bites() {
    let mut vals: Vec<Option<String>> = vec![Some("2".to_string()), Some("2".to_string())];
    // ① 成立 ⇒ Ok ✓。
    assert!(
        univ_check(&[UnivConstraint::Eq { lhs: 0, rhs: 1 }], &vals).is_ok(),
        "`?0 = ?1` 且都 = 2 ⇒ 必须 Ok ✓"
    );
    assert!(
        univ_check(&[UnivConstraint::Le { lhs: 0, rhs: 1 }], &vals).is_ok(),
        "`?0 ≤ ?1` 且 2 ≤ 2 ⇒ 必须 Ok ✓"
    );
    // ② 不成立 ⇒ Clash ✗。
    vals[1] = Some("3".to_string());
    assert_eq!(
        univ_check(&[UnivConstraint::Eq { lhs: 0, rhs: 1 }], &vals),
        Err(UnivUnsolved::Clash),
        "`?0 = ?1` 但 2 ≠ 3 ⇒ 必须 Clash ✗"
    );
    vals[0] = Some("3".to_string());
    vals[1] = Some("2".to_string());
    assert_eq!(
        univ_check(&[UnivConstraint::Le { lhs: 0, rhs: 1 }], &vals),
        Err(UnivUnsolved::Clash),
        "`?0 ≤ ?1` 但 3 ≤ 2 ⇒ 必须 Clash ✗"
    );
    // ③ 没赋值 ⇒ **Pending**（**不是** Clash ✗）。
    let half: Vec<Option<String>> = vec![Some("2".to_string()), None];
    assert_eq!(
        univ_check(&[UnivConstraint::Eq { lhs: 0, rhs: 1 }], &half),
        Err(UnivUnsolved::Pending),
        "有一侧没赋值 ⇒ 必须 Pending ✓（报成 Clash 就是**假红** ✗）"
    );
    // ④ Lub ⇒ Pending ✓。
    assert_eq!(
        univ_check(&[UnivConstraint::Lub { out: 0, a: 1, b: 2 }], &vals),
        Err(UnivUnsolved::Pending),
        "`Lub` 要**算 max** ⇒ 本片弃权 ⇒ Pending ✓"
    );
    // ⑤ 空表 ⇒ Ok ✓。
    assert!(univ_check(&[], &vals).is_ok(), "没有约束 ⇒ 必须 Ok ✓");
}

// ── **delta 兜底的可达性 + 探索期错误不许粘**（2026-10-15 · `Eq.subst` 的谓词位）──
//
// 用户原案 `apply Eq.subst (Eq.symm hx) h` 的 ④ 是 `?p ?b ≟ Set.mem x A`：
// 结构分解先比 `A ≟ Set.mem x` 判 No（**留下 `Clash`** ✗），而 `Set.mem x A`
// 一层 delta 就是 `A x` ⇒ 兜底本该判 `Yes` ✓。两条缺一不可：
//   ① 结构分解判否**不提前收摊** ⇒ 兜底进得去；
//   ② `Yes` ⇒ **回滚**探索期的 `first_err` ⇒ 出口不按 `Clash` 归因。
// 两条各自都有反向臂：撤任一条 ⇒ 下面两个用例**都**判红 ✓。

/// 一层 delta：`mem x A` ⇒ `A x`（`mem` 的定义体就是 `A a` —— 取的是**内层**
/// 实参 `x`，不是外层那个集合 `A` ✗）。
fn unfold_mem(e: &Expr) -> Expr {
    if let Expr::App { fun, arg: _set, .. } = e {
        if let Expr::App {
            fun: head,
            arg: elem,
            ..
        } = &**fun
        {
            if matches!(&**head, Expr::Ident { name, .. } if name == "mem") {
                return app(ident("A"), (**elem).clone());
            }
        }
    }
    e.clone()
}

/// **① 兜底可达**：结构分解判否 ⇒ delta 兜底必须接管 ⇒ `A x ≟ mem x A` 判 `Yes`。
/// 改前：结构分支当场 `return No` ⇒ 兜底一步都进不去 ⇒ 判红 ✓。
#[test]
fn a_failed_structural_comparison_falls_back_to_delta_unfolding() {
    let mut m = ctx_with(&unfold_mem);
    let l = app(ident("A"), ident("x"));
    let r = app(app(ident("mem"), ident("x")), ident("A"));
    assert_eq!(
        m.unify(&l, &r),
        Tri::Yes,
        "`A x` 与 `mem x A` 只差一层 delta ⇒ 必须判 Yes ✓"
    );
}

/// **② 探索期的错误不许粘**：同一对判 `Yes` 之后，出口 `discharge` **必须**
/// `Solved` —— 结构分解那次留下的 `Clash` 不许把结论顶掉。
/// 改前：`first_err = Some(Clash)` 粘住 ⇒ `discharge` 第二道闸归因 `Clash` ⇒ 判红 ✓。
#[test]
fn a_rescued_comparison_does_not_keep_the_speculative_clash() {
    let mut m = ctx_with(&unfold_mem);
    let a = m.fresh(ident("Type"), MetaKind::Natural, vec![]);
    m.assign(a, ident("Nat")).expect("α := Nat");
    let l = app(ident("A"), ident("x"));
    let r = app(app(ident("mem"), ident("x")), ident("A"));
    assert_eq!(m.unify(&l, &r), Tri::Yes);
    assert!(
        matches!(m.channel(), MetaSolve::Unsolved),
        "通道不许是 Clash —— 那一对**已经合一**了 ✓（实际 {:?}）",
        m.channel()
    );
    assert_eq!(solved(&mut m, &[a]), vec![ident("Nat")]);
}

/// **B0（2026-10-23）**：记法节点与点名前缀**同形**（`x ∈ A` ≟ `Set.mem x A`）。
/// 以前结构路径按"同形"比 ⇒ 记法节点判 No（现场：`intro x` 后目标 `x ∈ s` 与
/// 类型里的 `Set.mem x s` 对不上）⇒ 兜底摊开后判 Yes。
#[test]
fn notation_unifies_with_its_target_spine() {
    let mut m = ctx_with(&no_unfold);
    let x = ident("x");
    let a = ident("A");
    let notation = Expr::Notation {
        symbol: "∈".to_string(),
        symbol_span: Span::default(),
        target: "Set.mem".to_string(),
        assoc: crate::ast::NotationAssoc::Infix,
        lhs: Some(Box::new(x.clone())),
        rhs: Some(Box::new(a.clone())),
        alternatives: Vec::new(),
        span: Span::default(),
    };
    let spine = app(app(ident("Set.mem"), x), a);
    assert_eq!(m.unify(&notation, &spine), Tri::Yes, "记法应与其目标脊同形");
    // 反向也认（合一没有方向偏好）
    assert_eq!(m.unify(&spine, &notation), Tri::Yes);
    // 不同实参 ⇒ 仍然 No（摊开不能变成"什么都能过"）
    let b = ident("B");
    let other = app(app(ident("Set.mem"), ident("x")), b);
    assert_eq!(m.unify(&notation, &other), Tri::No);
}

// ─────────── B2-P1（2026-10-25）：声明级 store 的**机制**判据（不依赖 P2）───────────

use crate::compile::implicit::{no_alt_type, solve_prefix_meta_alt, Layer};

/// 一次 `{α : Type} (a : α) -> Prop` 求解（`?α` 由实参类型 `Nat` 解出）。
///
/// 直接调**引擎**那一份（`solve_prefix_meta_alt`），绕开 `solve_prefix_outcome_*` 的
/// 「先试严格/待定档」—— 否则引擎根本轮不到，store 也就不会长（判据会假绿 ✗）。
fn solve_with_decl_store(store: &DeclMVarStore, layers: &[Layer], result: &Expr) -> Vec<Expr> {
    let arg_tys = [Some(crate::proof::parse_expr_text("Nat").expect("parse"))];
    let alt = no_alt_type;
    match solve_prefix_meta_alt(
        layers,
        result,
        1,
        &arg_tys,
        &alt,
        None,
        &[],
        None,
        &crate::compile::elab::DefTable::new(),
        &|_n: &str| false,
        Some(store),
    ) {
        MetaSolve::Solved(v) => v,
        other => panic!("期望解出，实际 {other:?}"),
    }
}

/// **B2-P1 的判据**：同一条声明里的**两次**求解共用**一个** store ——
/// 第二次的 id **接着**第一次往后长（`ids == [0, 1]`，不是又回到 0）。
///
/// 反向验证（施工图）：把 `park_decl_store(Some)` 换成 `StoreGuard::detached()`
/// ⇒ 两次都从 0 起、`ids` 变成 `[0]` ⇒ 本条当场红 ✓。
#[test]
fn one_declaration_shares_one_store_across_two_solves() {
    // **闸门 ON 的档位由显式构造给**（不走环境 ⇒ 无全局竞态 ✓）；`store` 就是 `walk` 的
    // 声明处理器经 `new_meta_table()` 拿到的那个句柄 ✓。
    let decl = DeclMetaCtx::with_flag(true);
    let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
    assert!(
        DeclMetaCtx::with_flag(false).store.is_none(),
        "闸门关 ⇒ 不造声明级 store"
    );
    let layers = vec![
        Layer {
            name: "α".to_string(),
            domain: ty_sort(),
            style: crate::ast::BinderKind::Implicit,
        },
        Layer {
            name: "a".to_string(),
            domain: ident("α"),
            style: crate::ast::BinderKind::Explicit,
        },
    ];
    let result = Expr::Sort {
        sort: crate::ast::SortKind::Prop,
        span: Span::default(),
    };
    let first = solve_with_decl_store(store, &layers, &result);
    assert_eq!(store.len(), 1, "第一次求解必须留下 1 个元变量");
    let second = solve_with_decl_store(store, &layers, &result);
    assert_eq!(first, second, "同一个形状两次求解必须同解");
    assert_eq!(
        store.len(),
        2,
        "第二次必须**接着**第一次往后长（不是新建 store ✗）"
    );
    assert_eq!(
        store.ids(),
        vec![0, 1],
        "id 必须连续：第一次 id 0、第二次 id 1 ⇒ 共用了一个 store ✓"
    );
}

/// **R2**：同一条声明里 **引擎 id（`\0soko_m{id}`，从 0 往上）** 与
/// **S4 id（`fresh_decl_meta_id`，从 `DECL_META_ID_BASE` 往上）** 互不撞名；
/// **重入求解的 scratch store**（另一块）也不撞。
///
/// 反向验证（施工图）：把 `DECL_META_ID_BASE` 改回 0 ⇒ S4 的 id 落进引擎那一侧 ⇒ 本条红 ✓。
#[test]
fn engine_and_s4_meta_ids_never_collide_in_one_declaration() {
    let decl = DeclMVarStore::new();
    // 外层求解（声明级 store：base 0）。
    let mut guard = decl.park();
    let outer = MetaCtx::new(&no_unfold, guard.store_mut())
        .fresh(ty_sort(), MetaKind::Natural, vec![])
        .0;
    // 重入求解（槽已被外层占着 ⇒ scratch，从 DECL_META_ID_BASE - SCRATCH_BLOCK 起）。
    let inner = {
        let mut scratch = decl.park();
        let mut ctx = MetaCtx::new(&no_unfold, scratch.store_mut());
        ctx.fresh(ty_sort(), MetaKind::Natural, vec![]).0
    };
    // S4 那一侧（进程级原子；另一个线程也可能在分配 ⇒ 只断言**命名空间**，不断言具体值）。
    let s4 = fresh_decl_meta_id();
    guard.commit();

    assert!(
        outer < DECL_META_ID_BASE,
        "引擎 id 必须在 [0, DECL_META_ID_BASE)：{outer}"
    );
    assert!(
        (DECL_META_ID_BASE - SCRATCH_BLOCK..DECL_META_ID_BASE).contains(&inner),
        "scratch（重入）的 id 必须落在自己的块里：{inner}"
    );
    assert!(
        s4 >= DECL_META_ID_BASE,
        "S4 的 id 必须从 DECL_META_ID_BASE 起：{s4}"
    );
    assert_ne!(outer, inner, "重入求解与外层求解不许共用 id ✗");
    assert_ne!(outer, s4, "S4 与引擎不许共用 id ✗");
    assert_ne!(inner, s4, "S4 与重入 scratch 不许共用 id ✗");
}

// ─────── B2-P2（2026-10-25）：`\0soko_m{id}` ⇒ 内核 `Expr::Meta` 的**载体**判据 ───────
//
// 这三条**不读环境变量** ⇒ 无全局竞态 ✓（闸门档位一律由 `DeclMetaCtx::with_flag` 显式给）。

use crate::compile::dialect::Dialect;
use crate::compile::elab::{
    elab_expr, DefTable, ElabCtx, ElabScope, InductiveTable, KnownName, KnownTable, UnivMap,
};
use crate::compile::prelude::{CompileOptions, PreludeMode};
use crate::compile::scope::NamespaceScope;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::ExprPtr;

/// P2 的 elab 脚手架：**空作用域 / 空已知表**（元变量名只走 store，不需要任何常量）。
fn elab_with_decl<'a>(
    builder: &mut EnvBuilder<'a>,
    decl: Option<&DeclMetaCtx>,
    expr: &Expr,
) -> Result<ExprPtr<'a>, crate::compile::error::CompileError> {
    // ⚠ **显式两档**（不走 `Default` —— 那个会读 `SOKO_STRICT_AT` ⇒ 测试不许依赖环境 ✓）。
    let options = CompileOptions::with_dialect(PreludeMode::Full, Dialect::Loose);
    let inductives: InductiveTable<'a> = InductiveTable::new();
    let ns = NamespaceScope::default();
    let defs = DefTable::new();
    let ctx = ElabCtx {
        prefix_src: "",
        options: &options,
        inductives: &inductives,
        ns: &ns,
        notations: None,
        defs: &defs,
        metas: decl,
    };
    let mut scope: ElabScope<'a> = ElabScope::new();
    let univ: UnivMap<'a> = UnivMap::new();
    let known = KnownTable::new();
    let mut hovers = Vec::new();
    elab_expr(
        builder,
        expr,
        &mut scope,
        &univ,
        &known,
        &mut hovers,
        None,
        None,
        &ctx,
    )
}

/// 闸门 **ON** 的声明级 store + **一个**元变量（`ty`/`kind` 由调用方给）。
/// **P3a**：走 `guard.meta_ctx`（自动接**类型日志** ⇒ 停车期间可查 ✓）。
fn decl_with_one_mvar(ty: Expr, kind: MetaKind) -> (DeclMetaCtx, u32) {
    let decl = DeclMetaCtx::with_flag(true);
    let id = {
        let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
        let mut guard = store.park();
        let id = guard.meta_ctx(&no_unfold).fresh(ty, kind, vec![]).0;
        guard.commit();
        id
    };
    (decl, id)
}

/// **B2-P2 判据①**：`\0soko_m{id}` 必须 elaborate 成**内核** `Expr::Meta{id, kind, ty}` ——
/// `id`/`kind`/`ty` 三者都**来自 store**（不是默认 `Natural`、不是发明一个类型）。
///
/// 反向验证（施工图）：把 arm 里的 `kernel_meta_kind(kind)` 换成 `MetaKind::Natural`
/// （或直接用 `mk_meta`）⇒ `kind` 断言当场红 ✓；把 ty 换成 `builder.mk_sort(...)` ⇒ 类型断言红 ✓。
#[test]
fn meta_carrier_elaborates_to_kernel_meta_with_stored_kind_and_type() {
    let (decl, id) = decl_with_one_mvar(ty_sort(), MetaKind::SyntheticOpaque);
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), sokonanoda::util::Config::default());
    let out = elab_with_decl(&mut builder, Some(&decl), &ident(&meta_name(id)))
        .expect("store 里有这个 id ⇒ 载体必须命中");
    match &*out {
        sokonanoda::expr::Expr::Meta {
            id: got, kind, ty, ..
        } => {
            assert_eq!(*got, id, "核侧的 id 必须与名字里的绝对 id 相同");
            assert_eq!(
                *kind,
                sokonanoda::expr::MetaKind::SyntheticOpaque,
                "kind 必须**来自 store**（SyntheticOpaque）—— `mk_meta` 会给默认 `Natural` ✗"
            );
            assert!(
                matches!(&**ty, sokonanoda::expr::Expr::Sort { .. }),
                "ty 必须是 store 里 `Type` 的 elaborate 结果（`Sort`），实际 {ty:?}"
            );
        }
        other => panic!("必须是内核 `Expr::Meta` ✗，实际 {other:?}"),
    }
}

/// **B2-P2 判据②a**：**元变量作类型**（外层的 `ty` 就是内层的名字）⇒ 必须**递归展开且
/// 终止**，且内层仍是**同一个 id** 的内核 `Expr::Meta` ✓。
#[test]
fn meta_typed_meta_expands_through_the_store_and_terminates() {
    let decl = DeclMetaCtx::with_flag(true);
    let (outer_id, inner_id) = {
        let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
        let mut guard = store.park();
        let inner = guard
            .meta_ctx(&no_unfold)
            .fresh(ty_sort(), MetaKind::Natural, vec![])
            .0;
        // 外层的**类型**就是内层的元变量名（mvar-typed-mvar）。
        let outer = guard
            .meta_ctx(&no_unfold)
            .fresh(ident(&meta_name(inner)), MetaKind::Natural, vec![])
            .0;
        guard.commit();
        (outer, inner)
    };
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), sokonanoda::util::Config::default());
    let out = elab_with_decl(&mut builder, Some(&decl), &ident(&meta_name(outer_id)))
        .expect("两层元变量都必须从 store 展开（且必须终止）");
    match &*out {
        sokonanoda::expr::Expr::Meta { id, ty, .. } => {
            assert_eq!(*id, outer_id, "外层 id 必须对得上");
            match &**ty {
                sokonanoda::expr::Expr::Meta {
                    id: got,
                    ty: inner_ty,
                    ..
                } => {
                    assert_eq!(
                        *got, inner_id,
                        "嵌套的元变量必须展开成**同一个 id** 的 `Expr::Meta` ✓"
                    );
                    assert!(
                        matches!(&**inner_ty, sokonanoda::expr::Expr::Sort { .. }),
                        "内层类型必须来自 store 的 `Type`，实际 {inner_ty:?}"
                    );
                }
                other => panic!(
                    "外层 Meta 的类型必须是**内层 Meta**（mvar-typed-mvar）✗，实际 {other:?}"
                ),
            }
        }
        other => panic!("外层必须是内核 `Expr::Meta` ✗，实际 {other:?}"),
    }
}

/// **B2-P2 判据②b**：**自引用**（`m` 的类型就是 `m`）⇒ 必须**终止并拒**（今天的
/// unknown-identifier 路），**不许**无限递归、也不许发明类型 ✓。
#[test]
fn self_referential_meta_type_is_rejected_instead_of_recursing() {
    let decl = DeclMetaCtx::with_flag(true);
    // 白盒：`fresh` 要求**一次**给 ty，而 ty 里要写"即将分配的那个 id" ⇒ 先算 id 再推 store。
    let id = {
        let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
        let mut slot = store.slot.borrow_mut();
        let inner = slot.store.as_mut().expect("没有外层求解 ⇒ 槽里有 store");
        let id = inner.base + inner.mvars.len() as u32;
        inner.mvars.push(MVar {
            lctx: Default::default(),
            ty: ident(&meta_name(id)), // **自引用**
            value: None,
            kind: MetaKind::Natural,
            out_of_scope: Vec::new(),
        });
        id
    };
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), sokonanoda::util::Config::default());
    let err = elab_with_decl(&mut builder, Some(&decl), &ident(&meta_name(id)))
        .expect_err("自引用必须被拒（不是栈溢出、也不是发明类型）");
    assert_eq!(
        err.code(),
        "elab-unknown-identifier",
        "环路守卫必须回落到**今天**的 unknown-identifier；实际 {} / {}",
        err.code(),
        err.message
    );
}

/// **B2-P3a 判据①（P2 判据③ 的改写）**：闸门关 ⇒ 回落**今天**的 unknown-identifier；
/// 闸门开时 **store 被外层求解 park 走**也**照样查得到**（读**类型日志**、不读槽 ✓ ——
/// 这是 P3a 的正向保证：停车期间的载体可查）。
#[test]
fn carrier_reads_the_type_log_while_the_store_is_parked() {
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), sokonanoda::util::Config::default());

    // ① **闸门关**：`walk` 仍传 `Some(ctx)`，但 ctx 里没有 store（P1 的档位）。
    let off = DeclMetaCtx::with_flag(false);
    assert!(off.store.is_none(), "闸门关 ⇒ 不造声明级 store");
    let err = elab_with_decl(&mut builder, Some(&off), &ident(&meta_name(0)))
        .expect_err("闸门关 ⇒ 必须回到今天的 unknown-identifier");
    assert_eq!(
        err.code(),
        "elab-unknown-identifier",
        "闸门关：{} / {}",
        err.code(),
        err.message
    );

    // ② **store 正被外层求解独占**（槽是 `None`）：日志仍在 ⇒ carrier 必须命中 ✓。
    let decl = DeclMetaCtx::with_flag(true);
    let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
    let mut guard = store.park();
    let id = guard
        .meta_ctx(&no_unfold)
        .fresh(ty_sort(), MetaKind::SyntheticOpaque, vec![])
        .0;
    assert!(
        decl.lookup_meta(id).is_some(),
        "parked（槽为 None）期间类型日志必须可查 ✓"
    );
    let out = elab_with_decl(&mut builder, Some(&decl), &ident(&meta_name(id)))
        .expect("停车期间同一个 id 也必须可见（P3a）");
    assert!(
        matches!(&*out, sokonanoda::expr::Expr::Meta { id: got, .. } if *got == id),
        "停车期间必须造出 `Expr::Meta{{id: {id}}}`"
    );
    // ③ 放回之后同样可见（日志随 store 一起提交 ✓）。
    guard.commit();
    drop(guard);
    assert!(decl.lookup_meta(id).is_some(), "提交后仍然可见");
}

/// **B2-P3a 判据②**：类型日志**只由 `fresh` 记账**（槽里有、日志没有 ⇒ 看不见 ✓）、
/// 且**失败求解（未 `commit`）会把日志截回去**（不留可查的幽灵 id ✓）。
#[test]
fn type_log_ignores_the_slot_and_rolls_back_with_the_store() {
    // ① 直接往槽里的 store 推一条 MVar（绕过 `fresh` ⇒ 日志没有它）⇒ 查不到 ✓。
    let decl = DeclMetaCtx::with_flag(true);
    let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
    let mut guard = store.park();
    guard.store_mut().mvars.push(MVar {
        lctx: Default::default(),
        ty: ty_sort(),
        value: None,
        kind: MetaKind::Natural,
        out_of_scope: Vec::new(),
    });
    guard.commit();
    drop(guard);
    assert!(
        decl.lookup_meta(0).is_none(),
        "槽里有、日志没有 ⇒ 不许看得见（P3a 读日志、不读槽 ✓）"
    );

    // ② 一次**失败**求解（不 commit）造的 id ⇒ 回滚后查不到 ✓。
    let decl = DeclMetaCtx::with_flag(true);
    let store = decl.store.as_ref().expect("闸门开 ⇒ 声明级 store");
    let mut guard = store.park();
    let id = guard
        .meta_ctx(&no_unfold)
        .fresh(ty_sort(), MetaKind::Natural, vec![])
        .0;
    assert!(decl.lookup_meta(id).is_some(), "求解中必须可查");
    drop(guard); // 不 commit ⇒ 回滚
    assert!(
        decl.lookup_meta(id).is_none(),
        "失败求解的 id 不许留在日志里 ✓"
    );
}

/// **B2-P3a 判据③**：**实例化的 `ty` 里没有裸 fresh 名** —— `ty_i` 必须用
/// `sigma_{<i}` 把更早的前缀名换成**载体**（`\0soko_m*`）✓（作用域安全；环由
/// `MetaExpandGuard` 兜 ✓）。
#[test]
fn instantiated_telescope_types_carry_no_raw_fresh_names() {
    let mut meta = ctx_with(&no_unfold);
    let tel = [
        ("\0soko_p0".to_string(), ty_sort()),
        (
            "\0soko_mp1".to_string(),
            app(ident("Set"), ident("\0soko_p0")),
        ),
        (
            "\0soko_p2".to_string(),
            arrow(ident("\0soko_mp1"), ident("\0soko_p0")),
        ),
    ];
    let tel: Vec<(&str, &Expr)> = tel.iter().map(|(n, d)| (n.as_str(), d)).collect();
    let (ids, sigma) = crate::compile::meta::fresh_telescope_metas(&mut meta, &tel, 3);
    assert_eq!(ids.len(), 3);
    assert_eq!(sigma.get("\0soko_p0"), Some(&meta.meta_expr(ids[0])));
    // `ty_i` 里的**更早**名字必须已经换成载体；裸 fresh 名一个都不许留 ✓。
    for (i, (_, dom)) in tel.iter().enumerate() {
        let ty = &meta.store.mvars[i].ty;
        for (raw, _) in &tel {
            assert!(
                !crate::spine::mentions(raw, ty),
                "ty_{i} 里不许有裸 fresh 名 `{raw}`：{ty:?}（原文 {dom:?}）"
            );
        }
    }
    let carrier = |i: usize| Expr::Ident {
        name: meta_name(ids[i].0),
        span: Span::default(),
    };
    assert_eq!(
        meta.store.mvars[1].ty,
        app(ident("Set"), carrier(0)),
        "ty_1 = `Set m0`（同一趟接上 ✓）"
    );
    assert_eq!(
        meta.store.mvars[2].ty,
        arrow(carrier(1), carrier(0)),
        "ty_2 = `m1 → m0`（两个名字都换成载体 ✓）"
    );
}

/// **B2-P3a 判据④（B3 守卫）**：pp 回读的类型里**作用域外**的名字 ⇒ 拒（`false`）；
/// 作用域里的名字 / `\0` fresh 名 ⇒ 放行 ✓（**只否决、不发明**）。
#[test]
fn pp_type_scope_guard_rejects_free_names_and_accepts_scoped_ones() {
    // **B3 的反向面**：pp 文本里的常量名（**axiom/构造子**只有 `known` 认得）必须放行。
    let mut known = KnownTable::new();
    known.insert(
        "Person".to_string(),
        KnownName::Decl {
            universes: Vec::new(),
            implicit_prefix: 0,
            explicit_arity: 0,
            signature: None,
        },
    );
    let arena = stumpalo::Arena::new();
    let mut builder = EnvBuilder::new(arena.as_arena_ref(), sokonanoda::util::Config::default());
    let mut scope: ElabScope<'_> = ElabScope::new();
    let prop = builder.mk_sort(builder.zero());
    scope.push("A".to_string(), prop, None, Span::default());

    let names = scope.local_names();
    assert!(
        crate::compile::elab::pp_type_scope_safe(names, &known, &ident("A")),
        "作用域里的名字必须放行"
    );
    assert!(
        crate::compile::elab::pp_type_scope_safe(names, &known, &ident("Person")),
        "`known` 里的声明（axiom 也只在这张表里）必须放行"
    );
    assert!(
        crate::compile::elab::pp_type_scope_safe(names, &known, &ident("\0soko_p0")),
        "`\\0` fresh 名必须放行"
    );
    assert!(
        !crate::compile::elab::pp_type_scope_safe(names, &known, &ident("α")),
        "B3 的自由名（常量自己的 binder）必须拒 ✗"
    );
    assert!(
        !crate::compile::elab::pp_type_scope_safe(
            names,
            &known,
            &arrow(ident("α"), app(ident("Set"), ident("α")))
        ),
        "嵌套的自由名也要咬住（pp 的 `α → Set α`）✗"
    );
}
