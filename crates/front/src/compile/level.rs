//! **U1 的核心**（IA-4 §2.9）：**宇宙层**的元变量 + 约束存储 + occurs + **字面快路径**。
//!
//! 为什么单独一层（设计 `docs/design/metavar-engine.md` §2.9/§4 的 U1 片）：
//! 今天**没写**宇宙实参的常量一律取 `0`（`elab.rs` 的裸常量路径 `builder.zero()` ✗）——
//! `Quot.lift α β f h` 这种写法因此常与内核签名对不上（台账 **G-63** ✗）。要"解出"
//! 未写的层级，就需要一个**层级侧**的求解器（项侧那条 `implicit::solve_prefix` 管的是
//! 类型参数，管不到 `Level` ✗）。
//!
//! **本片的边界（刻意窄 ✓）**：
//! * 只做**字面**约束（`u := 0/1/2…`）—— 这是 §2.9 的"**快路径**：`SortKind` 字面相等 ⇒ Ok"
//!   那一条 ✓；`u+1` / `max u v` 这类**层级算术**不在本片 ✗（要等 U2 的惰性批量检查 ✓）。
//! * 解不出**就一个都不写**（调用方保持原行为 ✓）—— **绝不猜** ✗（与项侧"两条路都解不出
//!   ⇒ `None`"同一纪律 ✓）。
//! * 开关 `SOKO_UNIVERSE_METAVAR`（**默认关** ✓）：关着时本模块**一次都不进** ⇒ 逐字节不变 ✓
//!   （与 M1–M3 的"先在开关下证明两态等价、再默认开"同一条路 ✓）。
//!
//! 约束三种（§2.9）：`UEq`（相等，本片唯一会**赋值**的）/ `ULe`（`≤`，只**检查**）/
//! `ULub`（最小上界，本片**只存**）。赋值处做 **occurs check**（`u` 出现在自己的解里 ⇒ 拒 ✗）。

use crate::ast::{Expr, SortKind};
use std::collections::HashMap;

/// 层级元变量的**编码**（与项侧待定参数同款：不可打印的前缀 ✓）：
/// `\0soko_u{id}` —— 用户源码里写不出来，`Level(String)` 里一眼认得出 ✓。
#[allow(dead_code)]
pub(crate) const LEVEL_MVAR_PREFIX: &str = "\0soko_u";

/// 这个 `Level(name)` 是不是本模块造的**生成式**元变量 ✓（U2 的惰性检查要用 ✗ ——
/// 本片解的是**声明**的宇宙参数名，用不到这条 ✓）。
#[allow(dead_code)]
pub(crate) fn is_level_mvar(name: &str) -> bool {
    name.starts_with(LEVEL_MVAR_PREFIX)
}

/// 层级**是不是字面**（`0` / `1` / …）—— 快路径只吃字面 ✓（`u` / `u+1` 一律不算 ✗）。
pub(crate) fn literal_level(text: &str) -> Option<u64> {
    text.parse::<u64>().ok()
}

/// 层级约束（§2.9 的三种，够用 ✓）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LevelConstraint {
    /// `lhs = rhs`（本片**唯一**会产生的：每个解出来的字面都记一条 ✓）
    Eq(String, String),
    /// `lhs ≤ rhs` —— **U2 才产生/才检查** ✗（本片只留形状 ✓）
    #[allow(dead_code)]
    Le(String, String),
    /// `lhs = max(rhs, third)` —— 同上，U2 ✗
    #[allow(dead_code)]
    Lub(String, String, String),
}

/// 层级约束存储 + 赋值（含 **occurs check** ✓）。
///
/// 纪律：`assign` **只在两边都是字面或已知解**时才写 ✓；写不进去就**原样返回 `false`**
/// （调用方据此**放弃整条求解**，不部分写 ✗ —— 部分写会造出"半解"的项 ✗）。
#[derive(Debug, Default)]
pub(crate) struct LevelStore {
    /// 已赋值的层级元变量 ⇒ 字面
    assigned: HashMap<String, u64>,
    /// 约束流水（本片只记不查 ✓ —— U2 才做惰性批量检查 ✗）
    constraints: Vec<LevelConstraint>,
}

impl LevelStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 约束流水（本片只记不查 ✓ —— U2 的惰性批量检查才是判的地方 ✗）。
    #[allow(dead_code)]
    pub(crate) fn constraints(&self) -> &[LevelConstraint] {
        &self.constraints
    }

    pub(crate) fn get(&self, mvar: &str) -> Option<u64> {
        self.assigned.get(mvar).copied()
    }

    /// `mvar := value`。**occurs check**：`mvar` 出现在 `value` 里（这里 `value` 只可能是
    /// 字面或另一个元变量）⇒ 拒 ✗；与已赋值冲突 ⇒ 拒 ✗（**不覆盖** ✗）。
    pub(crate) fn assign(&mut self, mvar: &str, value: &str) -> bool {
        // ⚠ 入参是**声明的宇宙参数名**（`u` / `v` —— 源名 ✓），**不是**生成式元变量的
        // `\0soko_u{id}` 编码 ✗（那是 U2 的形状 ✓）。这里只拒空名 ✓。
        if mvar.is_empty() {
            return false;
        }
        if value == mvar {
            return false; // occurs：自己解自己 ✗
        }
        let resolved = match literal_level(value) {
            Some(n) => n,
            None if is_level_mvar(value) => match self.assigned.get(value) {
                Some(n) => *n,
                None => return false, // 解还是个未定的元变量 ⇒ 本片不写 ✗
            },
            None => return false, // `u+1` / `max u v` 等层级算术 ⇒ 本片不写 ✗
        };
        match self.assigned.get(mvar) {
            Some(prev) => *prev == resolved,
            None => {
                self.assigned.insert(mvar.to_string(), resolved);
                true
            }
        }
    }

    /// 记一条约束（**不判**✓ —— U2 的惰性批量检查才是判的地方 ✗）。
    pub(crate) fn push(&mut self, c: LevelConstraint) {
        self.constraints.push(c);
    }

    /// 把已解出的层级写成 `u ⇒ "n"`（调用方据此重建常量 ✓）。
    pub(crate) fn solved(&self, params: &[String]) -> Option<Vec<String>> {
        params
            .iter()
            .map(|p| self.assigned.get(p).map(|n| n.to_string()))
            .collect()
    }
}

/// 开关 `SOKO_UNIVERSE_METAVAR`（**默认关** ✓）：关着时本模块**一次都不进** ⇒ 逐字节不变 ✓
/// （与 M1–M3「先在开关下证明两态等价、再默认开」同一条路 ✓）。
pub(crate) fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SOKO_U1_TRACE").is_ok())
}

pub(crate) fn universe_metavar_enabled() -> bool {
    // **G-30 收口（2026-10-05）作废**：裸常量层 mvar 恒生成并解出（对齐 Lean
    // `mkFreshLevelMVars` + `levelMVarToParam`），U1 的「未写层级按字面约束求解」
    // 已被 `infer_const_universes` 完整覆盖 —— 开关再开会产生与恒解**冲突**
    // 的第二套解（实测 `elab-universe-level-conflict` ✗）。自创机制作废（用户
    // 2026-10-05 纪律：完全对齐 Lean 4，dsh 自创的东西全作废）。恒 false，
    // `solve_universes` 永不进入，开关开/关行为一致（都走 G-30 恒解）。
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| false)
}

/// 给一条常量签名的**未写宇宙参数**求解：`pairs` = (模板, 实际) 对（层域 ↔ 实参类型、
/// 结果 ↔ 期望类型 ✓）。
///
/// **纪律**：每个参数**逐个**找（一个字面匹配就够 ✓）；任何一个**解不出 ⇒ 整条 `None`** ✗
/// （绝不部分写 ✗ —— 半解的常量层级会造出另一个错项 ✗）。开关关着 / 没有宇宙参数
/// ⇒ `None`（调用方走既有路径 ✓，零开销 ✓）。
pub(crate) fn solve_universes(params: &[String], pairs: &[(&Expr, &Expr)]) -> Option<Vec<String>> {
    if params.is_empty() || !universe_metavar_enabled() {
        return None;
    }
    let mut store = LevelStore::new();
    for param in params {
        let mut found = false;
        for (template, actual) in pairs {
            // 每次尝试用**独立**的探针 store ✓ —— `match_levels` 走一半失败时可能已经
            // 写了几个位 ✗，不许污染正式 store ✗。
            let mut probe = LevelStore::new();
            if match_levels(template, actual, param, &mut probe) {
                if let Some(n) = probe.get(param) {
                    found = store.assign(param, &n.to_string());
                    if found {
                        // 记一条**相等**约束 ✓（U2 的惰性检查要读这份流水 ✗）
                        store.push(LevelConstraint::Eq(param.clone(), n.to_string()));
                    }
                    break;
                }
            }
        }
        if !found {
            return None;
        }
    }
    debug_assert_eq!(
        store.constraints().len(),
        params.len(),
        "每个解出的层级都要留一条约束 ✓"
    );
    store.solved(params)
}

/// 在两个**类型表达式**里对齐着找 `param` 的解（**浅层、字面、保守** ✓）。
///
/// 认识的形状（其余一律**放弃** ✗ —— 不猜）：
/// * `Sort(Level(x))` ↔ `Sort(Level(y))`：`x == param` 且 `y` 是字面 ⇒ 记 `param := y` ✓；
/// * `Sort(Type)` ↔ `Sort(Type)` / `Sort(Prop)` ↔ `Sort(Prop)`：字面相等 ⇒ 继续 ✓；
/// * `UniverseApp { name, levels }` 同名同元数 ⇒ **逐位**递归（`level_text` 那条 ✓）；
/// * `App` 同名同元数 / `Arrow` / `Forall`（单 binder）⇒ 逐位递归 ✓；
/// * 其余（含 `Ident` 头不同、`def` 头、`Hole`…）⇒ 放弃 ✗。
///
/// 返回 `false` = **放弃整条求解**（调用方保持原行为 ✓）。
pub(crate) fn match_levels(
    template: &Expr,
    actual: &Expr,
    param: &str,
    store: &mut LevelStore,
) -> bool {
    match (template, actual) {
        (
            Expr::Sort {
                sort: SortKind::Level(t),
                ..
            },
            Expr::Sort {
                sort: SortKind::Level(a),
                ..
            },
        ) => match_level_text(t, a, param, store),
        // `Sort u` ↔ **源级字面形态**：`Type`（= `Sort 1` ✓）/ `Prop`（= `Sort 0` ✓）/
        // `Sort n`（数字 ✓）。实测（本轮）：`Show {u} : {α : Sort u} → α → Nat` 写成
        // `Show 0` 时，`α` 的类型在源级是 **`Type`**（不是 `Sort(Level("1"))` ✗）
        // ⇒ 只认 `Level` 对 `Level` 会**一个解都拿不到** ✗（接线在跑、就是解不出 ✗）。
        (
            Expr::Sort {
                sort: SortKind::Level(t),
                ..
            },
            Expr::Sort { sort: a, .. },
        ) if t.as_str() == param => sort_literal(a, store, param),
        (Expr::Sort { sort: t, .. }, Expr::Sort { sort: a, .. }) => t == a,
        (
            Expr::UniverseApp {
                name: tn,
                levels: tl,
                ..
            },
            Expr::UniverseApp {
                name: an,
                levels: al,
                ..
            },
        ) if tn == an && tl.len() == al.len() => tl
            .iter()
            .zip(al.iter())
            .all(|(t, a)| match_level_text(t, a, param, store)),
        (
            Expr::App {
                fun: tf, arg: ta, ..
            },
            Expr::App {
                fun: af, arg: aa, ..
            },
        ) => match_levels(tf, af, param, store) && match_levels(ta, aa, param, store),
        (
            Expr::Arrow {
                domain: td,
                codomain: tc,
                ..
            },
            Expr::Arrow {
                domain: ad,
                codomain: ac,
                ..
            },
        ) => match_levels(td, ad, param, store) && match_levels(tc, ac, param, store),
        _ => false,
    }
}

/// **源级字面 sort** 的层级数：`Prop` = 0 ✓ · `Type` = 1 ✓ · `Sort n` = n ✓；其余（含
/// `Sort u` 这种带变量的）⇒ 放弃 ✗（**绝不猜** ✓）。
fn sort_literal(sort: &SortKind, store: &mut LevelStore, param: &str) -> bool {
    let n = match sort {
        SortKind::Prop => 0,
        SortKind::Type => 1,
        SortKind::Sort(n) => *n,
        SortKind::Level(_) => return false,
    };
    store.assign(param, &n.to_string())
}

/// 层级**文本**之间的对齐：`param` ↔ 字面 ⇒ 赋值 ✓；两边同字面 ⇒ Ok ✓；其余放弃 ✗。
fn match_level_text(template: &str, actual: &str, param: &str, store: &mut LevelStore) -> bool {
    if template == param {
        return store.assign(param, actual);
    }
    if let (Some(t), Some(a)) = (literal_level(template), literal_level(actual)) {
        return t == a;
    }
    false
}

// ───────────────────────── U2-b：**惰性批量检查** ─────────────────────────
//
// 设计 `docs/design/metavar-engine.md` §2.9 / 表 8 第 8 行（对齐 Coq）：**约束攒着**，
// **声明收尾才查** —— 不逐次合一都查图 ✓。这里先把"攒 + 收尾查"这件事做成一个**纯**件 ✓
// （不带内核调用、不碰项 ✓），它的**等价性**用单测钉死 ✓：
// 「边推边查」（eager ✓）与「推完一次查」（lazy ✓）对**同一组约束**必须给出**同一个判定** ✓
// —— 即"**惰性只是时机，不是放水**" ✓。

/// 一条**已解出**的宇宙约束（`param := literal` ✓）—— 只记不判 ✓。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LevelBinding {
    pub(crate) param: String,
    pub(crate) literal: u64,
}

/// 批量检查的判定 ✓。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LevelVerdict {
    /// 全部自洽 ✓
    Ok,
    /// **刚性冲突**：同一个参数被要求成两个不同的字面 ✓ —— 走**宇宙专属报错通道** ✗
    /// （**不许**并进 `elab-implicit-argument-unsolved` / `elab-notation-argument-unsolved` ✓）。
    Conflict {
        param: String,
        first: u64,
        second: u64,
    },
}

/// **约束攒着、收尾才查** ✓（U2-b）。
#[derive(Debug, Default)]
pub(crate) struct LevelBatch {
    bindings: Vec<LevelBinding>,
}

impl LevelBatch {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 记一条约束 —— **不判** ✓（惰性 ✓）。
    pub(crate) fn push(&mut self, param: &str, literal: u64) {
        self.bindings.push(LevelBinding {
            param: param.to_string(),
            literal,
        });
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.bindings.len()
    }

    /// **收尾统一查** ✓（O(n²) 对 n=宇宙参数个数，最大也就是个位数 ✓）。
    pub(crate) fn check(&self) -> LevelVerdict {
        for (i, a) in self.bindings.iter().enumerate() {
            for b in self.bindings.iter().skip(i + 1) {
                if a.param == b.param && a.literal != b.literal {
                    return LevelVerdict::Conflict {
                        param: a.param.clone(),
                        first: a.literal,
                        second: b.literal,
                    };
                }
            }
        }
        LevelVerdict::Ok
    }

    /// **边推边查**的对照实现 ✓ —— 只为单测证明"惰性 ≡ 急切"（可见输出不因时机而变 ✓）。
    #[cfg(test)]
    pub(crate) fn check_eagerly(&self) -> LevelVerdict {
        let mut acc = LevelBatch::new();
        for b in &self.bindings {
            acc.push(&b.param, b.literal);
            if let LevelVerdict::Conflict { .. } = acc.check() {
                return acc.check();
            }
        }
        LevelVerdict::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;

    fn sort_level(text: &str) -> Expr {
        Expr::Sort {
            sort: SortKind::Level(text.to_string()),
            span: Span::default(),
        }
    }

    fn sort_type() -> Expr {
        Expr::Sort {
            sort: SortKind::Type,
            span: Span::default(),
        }
    }

    #[test]
    fn mvar_encoding_is_unprintable_and_recognised() {
        let name = format!("{LEVEL_MVAR_PREFIX}7");
        assert!(is_level_mvar(&name));
        assert!(!is_level_mvar("u"));
        assert!(!is_level_mvar("0"));
    }

    #[test]
    fn literal_fast_path_only_accepts_digits() {
        assert_eq!(literal_level("0"), Some(0));
        assert_eq!(literal_level("12"), Some(12));
        assert_eq!(literal_level("u"), None);
        assert_eq!(literal_level("u+1"), None);
        assert_eq!(literal_level("max u v"), None);
    }

    #[test]
    fn assign_writes_literals_and_refuses_arithmetic() {
        let mut s = LevelStore::new();
        assert!(s.assign("\0soko_u0", "1"));
        assert_eq!(s.get("\0soko_u0"), Some(1));
        // 层级算术不在本片 ⇒ 不写 ✓
        assert!(!s.assign("\0soko_u1", "u+1"));
        assert_eq!(s.get("\0soko_u1"), None);
        // 空名 ⇒ 拒 ✓
        assert!(!s.assign("", "1"));
    }

    #[test]
    fn assign_is_idempotent_and_refuses_conflicts() {
        let mut s = LevelStore::new();
        assert!(s.assign("\0soko_u0", "1"));
        assert!(s.assign("\0soko_u0", "1"), "同一个值重复赋值 ⇒ Ok ✓");
        assert!(!s.assign("\0soko_u0", "2"), "冲突 ⇒ 拒（不覆盖 ✗）");
        assert_eq!(s.get("\0soko_u0"), Some(1));
    }

    #[test]
    fn occurs_check_rejects_self_solution() {
        let mut s = LevelStore::new();
        assert!(!s.assign("\0soko_u0", "\0soko_u0"), "自己解自己 ⇒ 拒 ✗");
        // 解本身是**未定**的元变量 ⇒ 本片不写 ✗（要等 U2 的约束求解）
        assert!(!s.assign("\0soko_u0", "\0soko_u1"));
    }

    #[test]
    fn assign_through_a_solved_mvar_resolves() {
        let mut s = LevelStore::new();
        assert!(s.assign("\0soko_u1", "2"));
        assert!(s.assign("\0soko_u0", "\0soko_u1"), "解已定 ⇒ 跟着定下来 ✓");
        assert_eq!(s.get("\0soko_u0"), Some(2));
    }

    #[test]
    fn solved_requires_every_param() {
        let mut s = LevelStore::new();
        let params = vec!["\0soko_u0".to_string(), "\0soko_u1".to_string()];
        assert!(s.assign("\0soko_u0", "1"));
        assert_eq!(
            s.solved(&params),
            None,
            "有一个没解出 ⇒ 整条放弃 ✗（不部分写 ✓）"
        );
        assert!(s.assign("\0soko_u1", "3"));
        assert_eq!(
            s.solved(&params),
            Some(vec!["1".to_string(), "3".to_string()])
        );
    }

    #[test]
    fn solve_universes_is_inert_while_the_switch_is_off() {
        // 默认关 ⇒ 即便形状完全可解也**一个都不写** ✓（逐字节不变的前提 ✓）
        let mvar = "\0soko_u0";
        let params = vec![mvar.to_string()];
        let t = sort_level(mvar);
        let a = sort_level("2");
        if !universe_metavar_enabled() {
            assert_eq!(solve_universes(&params, &[(&t, &a)]), None);
        }
    }

    #[test]
    fn solve_universes_needs_every_param() {
        // 无宇宙参数 ⇒ 直接 None（零开销 ✓）
        assert_eq!(solve_universes(&[], &[]), None);
    }

    // ── U2-b：惰性批量检查 ────────────────────────────────────────────────
    #[test]
    fn batch_defers_the_verdict_until_check() {
        let mut b = LevelBatch::new();
        b.push("u", 1);
        b.push("u", 2); // 冲突 ✓ —— 但 push **不判** ✓
        assert_eq!(b.len(), 2);
        assert_eq!(
            b.check(),
            LevelVerdict::Conflict {
                param: "u".to_string(),
                first: 1,
                second: 2
            }
        );
    }

    #[test]
    fn batch_is_ok_when_every_param_agrees() {
        let mut b = LevelBatch::new();
        b.push("u", 1);
        b.push("v", 2);
        b.push("u", 1);
        assert_eq!(b.check(), LevelVerdict::Ok);
    }

    #[test]
    fn batch_ignores_conflicts_across_different_params() {
        let mut b = LevelBatch::new();
        b.push("u", 1);
        b.push("v", 2);
        assert_eq!(b.check(), LevelVerdict::Ok, "不同参数各要各的 ⇒ 不是冲突 ✓");
    }

    /// **判据 2 的核心** ✓：**惰性只是时机，不是放水** —— 同一组约束，
    /// 「推完一次查」（lazy ✓）与「边推边查」（eager ✓）必须给出**同一个判定** ✓。
    #[test]
    fn lazy_and_eager_verdicts_are_identical() {
        let cases: Vec<Vec<(&str, u64)>> = vec![
            vec![],
            vec![("u", 1)],
            vec![("u", 1), ("u", 1)],
            vec![("u", 1), ("u", 2)],
            vec![("u", 1), ("v", 1), ("u", 2)],
            vec![("u", 3), ("v", 3), ("w", 3)],
            vec![("u", 0), ("u", 1), ("u", 0)],
        ];
        for case in cases {
            let mut b = LevelBatch::new();
            for (p, l) in &case {
                b.push(p, *l);
            }
            assert_eq!(
                b.check(),
                b.check_eagerly(),
                "lazy 与 eager 判定必须一致（同一组约束 ✓）：{case:?}"
            );
        }
    }

    #[test]
    fn batch_reports_the_first_conflicting_pair_deterministically() {
        let mut b = LevelBatch::new();
        b.push("v", 1);
        b.push("u", 2);
        b.push("u", 3);
        assert_eq!(
            b.check(),
            LevelVerdict::Conflict {
                param: "u".to_string(),
                first: 2,
                second: 3
            },
            "按**推入顺序**报第一对 ✓（输出稳定 ✓，与 `--json` 对拍有关 ✓）"
        );
    }

    #[test]
    fn batch_len_counts_every_binding() {
        let mut b = LevelBatch::new();
        assert_eq!(b.len(), 0);
        b.push("u", 1);
        b.push("u", 1);
        assert_eq!(b.len(), 2, "去重发生在 check ✓，不在 push ✓（惰性 ✓）");
    }

    #[test]
    fn batch_check_is_repeatable() {
        let mut b = LevelBatch::new();
        b.push("u", 1);
        b.push("u", 2);
        let first = b.check();
        assert_eq!(first, b.check(), "检查是纯的 ⇒ 可重复 ✓");
    }

    #[test]
    fn batch_empty_is_ok() {
        assert_eq!(LevelBatch::new().check(), LevelVerdict::Ok);
    }

    #[test]
    fn match_reads_a_level_from_a_sort_pair() {
        // **声明**的宇宙参数名（源名 ✓）也能解 —— 解的就是它 ✓
        let mut s = LevelStore::new();
        assert!(match_levels(
            &sort_level("u"),
            &sort_level("2"),
            "u",
            &mut s
        ));
        assert_eq!(s.get("u"), Some(2));
        let mut s = LevelStore::new();
        let mvar = "\0soko_u0";
        assert!(match_levels(
            &sort_level(mvar),
            &sort_level("2"),
            mvar,
            &mut s
        ));
        assert_eq!(s.get(mvar), Some(2));
    }

    #[test]
    fn match_reads_levels_from_source_level_literals() {
        // `Sort u` ↔ `Type` ⇒ `u := 1` ✓；`Sort u` ↔ `Prop` ⇒ `u := 0` ✓
        let mvar = "\0soko_u0";
        let mut s = LevelStore::new();
        assert!(match_levels(&sort_level(mvar), &sort_type(), mvar, &mut s));
        assert_eq!(s.get(mvar), Some(1));
        let mut s = LevelStore::new();
        let prop = Expr::Sort {
            sort: SortKind::Prop,
            span: Span::default(),
        };
        assert!(match_levels(&sort_level(mvar), &prop, mvar, &mut s));
        assert_eq!(s.get(mvar), Some(0));
        // `Sort u` ↔ `Sort 3`（数字形态）⇒ `u := 3` ✓
        let mut s = LevelStore::new();
        let three = Expr::Sort {
            sort: SortKind::Sort(3),
            span: Span::default(),
        };
        assert!(match_levels(&sort_level(mvar), &three, mvar, &mut s));
        assert_eq!(s.get(mvar), Some(3));
    }

    #[test]
    fn match_refuses_unknown_shapes() {
        // `Sort Type` vs `Sort Type` ⇒ 字面相等 ⇒ Ok ✓（但**不写**任何东西 ✓）
        let mut s = LevelStore::new();
        assert!(match_levels(
            &sort_type(),
            &sort_type(),
            "\0soko_u0",
            &mut s
        ));
        assert!(s.constraints().is_empty());
        // 头不同 ⇒ 放弃 ✗（绝不猜）
        let mut s = LevelStore::new();
        let a = Expr::Ident {
            name: "A".into(),
            span: Span::default(),
        };
        let b = Expr::Ident {
            name: "B".into(),
            span: Span::default(),
        };
        assert!(!match_levels(&a, &b, "\0soko_u0", &mut s));
    }

    #[test]
    fn match_walks_application_spines_and_level_arguments() {
        // `F.{u} x` ↔ `F.{2} x` ⇒ `u := 2` ✓（宇宙实参位也要认 ✓）
        let mut s = LevelStore::new();
        let mvar = "\0soko_u0";
        let t = Expr::App {
            fun: Box::new(Expr::UniverseApp {
                name: "F".into(),
                levels: vec![mvar.to_string()],
                span: Span::default(),
            }),
            arg: Box::new(sort_type()),
            explicit_spine: false,
            span: Span::default(),
        };
        let a = Expr::App {
            fun: Box::new(Expr::UniverseApp {
                name: "F".into(),
                levels: vec!["2".to_string()],
                span: Span::default(),
            }),
            arg: Box::new(sort_type()),
            explicit_spine: false,
            span: Span::default(),
        };
        assert!(match_levels(&t, &a, mvar, &mut s));
        assert_eq!(s.get(mvar), Some(2));
    }
}
