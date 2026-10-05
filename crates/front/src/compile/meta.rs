//! **元参数引擎**（IA-4 **M1**）：元变量存储 + **三值合一** + occurs / type-occurs / 作用域 +
//! **有界待定约束** + 出口 `zonk`。
//!
//! 设计 ⇒ `docs/design/metavar-engine.md` §2（§1.2 是 Lean 4 / Coq 的 17 行机制对照）；
//! M0 的基线与 13 形状能力清单 ⇒ `docs/design/metavar-m0.md`（含两条**硬约束**：S13 ⇒ 必须复用
//! delta 展开兜底；13 形状的 26 个读数 = `sibling` 档的回归臂）。
//!
//! **三条设计红线**（照抄设计 §2，改之前先读）：
//! 1. **元变量不进项**：引擎只在**一次求解内**活着，出口 `zonk` 之后不许留 `\0soko_m*`（本模块的
//!    `discharge` 出口自检 + 调用方 `elab_expr` 遇未知名自然报错 = 双保险）；
//! 2. **三值**（`Yes`/`No`/`Undef`）：`Undef` = **弃权**（不是失败）⇒ 进待定队列，等后续赋值再试；
//! 3. **有界**：深度 / 轮数 / 步数三上限 + `unify_all` 的**停条件 = 待定计数不再严格下降**
//!    （与 Lean `processPostponed` 同款）；超限按「无解」处理，**不新增失败面** ✓。
//!
//! 元变量的**表示**：保留前缀的 fresh 名 `\0soko_m{id}`（`\0` 不可能出现在源标识符里 ⇒ 与
//! `implicit::telescope` 的 `\0soko_p{i}` 同一纪律）⇒ `spine::substitute`/`mentions`/`same_shape`
//! **一个字都不用改**就能在模板里代换元变量。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::span::Span;

/// 元变量名的保留前缀（`\0` 前缀不可能与用户标识符相撞）。
const META_PREFIX: &str = "\u{0}soko_m";

/// 元变量的名字（`\0soko_m{id}`）。
pub(crate) fn meta_name(id: u32) -> String {
    format!("{META_PREFIX}{id}")
}

/// 这个名字是不是元变量？（`zonk` 之后不许再出现 ⇒ 出口自检用它。）
pub(crate) fn is_meta_name(name: &str) -> bool {
    name.starts_with(META_PREFIX)
}

// ───────────────────────── U1：宇宙层的元变量（§2.9）─────────────────────────
//
// **U1 第 1 片（2026-10-05）**：先落**编码**与 **occurs** —— 与元变量同款
// （`\0` 前缀的保留名 ⇒ 与用户标识符**永不撞车** ✓）。
//
// **Lean 4 对照** ✓（本机源码 HEAD `d0493e4c1e` ✓）：
// * Lean 的层是 **`Level.mvar LMVarId`**（`Lean/Level.lean`），由
//   `Meta/LevelDefEq.lean` 的 `isLevelDefEqAux`/`solve` **合一**（含 `solveSelfMax`）；
// * **刚性宇宙约束推迟**到项元变量被赋值（`postponeIsLevelDefEq`）；
// * 赋值处有 **occurs**（`u` 出现在自己的解里 ⇒ 拒）。
//
// ⚠ **偏离 1 条（白纸黑字）**：Lean 用**独立的 `LMVarId` 类型**，我们用**名字编码**
// （`\0soko_u{id}`）—— 与 `MetaId` 同款理由：源级表示**一个字都不用改**
// （`spine::substitute`/`mentions`/`same_shape` 全按名字走），面更小。
// ⚠ **本片零行为变化**：没有任何生产调用方；快路径「`SortKind` 字面相等 ⇒ Ok」原样保留。

/// 层元变量名的保留前缀（与 `META_PREFIX` **不同** ⇒ 两类元变量不会互相误判）。
const LEVEL_PREFIX: &str = "\u{0}soko_u";

/// 层元变量的名字（`\0soko_u{id}`）。
#[allow(dead_code)] // U1 第 2 片（约束存储 + 合一）会用
pub(crate) fn level_name(id: u32) -> String {
    format!("{LEVEL_PREFIX}{id}")
}

/// 这个名字是不是层元变量？
#[allow(dead_code)] // U1 第 2 片会用
pub(crate) fn is_level_name(name: &str) -> bool {
    name.starts_with(LEVEL_PREFIX)
}

/// 层元变量的 id（不是层元变量名 ⇒ `None`）。
#[allow(dead_code)] // U1 第 2 片会用
pub(crate) fn level_id_of(name: &str) -> Option<u32> {
    name.strip_prefix(LEVEL_PREFIX)?.parse().ok()
}

/// **occurs 检查**（对齐 Lean 赋值处的那一条）：`u` 出现在自己的解里 ⇒ **拒**。
///
/// ⚠ **作用域是「层文本」不是「整项」** —— 层的解永远是**层表达式**（`u` / `u+1` /
/// `max u v`），而层只出现在 `Sort` 位 ⇒ 检查**那段文本**就够，**不必**写整棵 AST 的
/// 遍历（源 AST 没有通用子项遍历，写了就是大改）。
///
/// ⚠ **为什么必须有**：没有它，`u := u + 1` 这类赋值会让 `zonk` 的链式迭代
/// **不终止或给错值** ⇒ 这是**正确性**要求，不是优化。
///
/// 判据（词边界）：`u` 命中 · `u1` **不**命中 · `uu` **不**命中 · `max u v` 命中。
#[allow(dead_code)] // U1 第 2 片（赋值）会用
pub(crate) fn level_occurs_in_text(target: u32, text: &str) -> bool {
    let needle = level_name(target);
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '\'';
    let bytes = text.as_bytes();
    let nb = needle.as_bytes();
    let mut i = 0usize;
    while i + nb.len() <= bytes.len() {
        if &bytes[i..i + nb.len()] == nb {
            let before_ok = i == 0 || !is_ident(text[..i].chars().next_back().unwrap_or(' '));
            let after_ok = i + nb.len() == bytes.len()
                || !is_ident(text[i + nb.len()..].chars().next().unwrap_or(' '));
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// 元变量 id（**只在一次求解内有效**，不跨调用、不进项）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MetaId(pub u32);

/// 元变量的**种类**（对齐 Lean 的 `MetavarKind`）：
/// `Natural` = 可被合一赋值；`SyntheticOpaque` = **永不被类型约束填**（洞的语义，M1 只留接缝）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // `SyntheticOpaque` 的用武之地在 M2/B2（洞的语义）
pub(crate) enum MetaKind {
    Natural,
    SyntheticOpaque,
}

/// 三值结果（对齐 Lean 的 `LBool`）：`Yes` 成立 · `No` 不成立 · `Undef` **弃权**（不是失败）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tri {
    Yes,
    No,
    Undef,
}

/// 引擎的硬错误（`unify` 会把它们折成 `Tri::No`；单测直接看它）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum MetaErr {
    /// occurs check：元变量出现在自己的解里
    Occurs,
    /// **type-occurs**：元变量出现在「解里提到的另一个元变量」的**类型**里（Lean 的 `typeOccursCheck`）
    TypeOccurs,
    /// **作用域**：解里提到了该元变量作用域外的名字（与 occurs 分开报 —— 两者是不同 bug）
    Scope,
    /// **sort/kind 不对**（M3）：值的 sort 与元变量声明类型的 sort **确定**不符
    /// （只拒"确定错"的；不知道就放行 ⇒ 不可能产生假拒绝 ✓）
    Kind,
    /// **刚性冲突**（两侧头不同、且都没有可赋值的元变量）
    Clash,
}

/// 一次求解的**结局**（M3 的"三通道"；**不是**新的错误码 —— 用户可见的码仍是既有两条，
/// 通道只决定 **hint 说哪一句**，见 `docs/design/metavar-engine.md` §2.6）：
/// `Unsolved` = 补不出（既有语义）· `Kind` = 值的 sort 确定不对 · `Clash` = 两条约束刚性冲突。
#[derive(Debug)]
pub(crate) enum MetaSolve {
    Solved(Vec<Expr>),
    Unsolved,
    Kind,
    Clash,
}

impl MetaSolve {
    /// 给只关心"成没成"的调用方（`solve_prefix` 的三个调用方里有两个不报错）。
    pub(crate) fn into_option(self) -> Option<Vec<Expr>> {
        match self {
            MetaSolve::Solved(v) => Some(v),
            _ => None,
        }
    }
}

struct MVar {
    /// 声明类型（= 该前导层的**域**，已把更早的元变量代进去）
    ty: Expr,
    /// 赋值（`None` = 未解）
    value: Option<Expr>,
    /// 种类（M1 只有 `Natural` 会真的被赋值；`SyntheticOpaque` 的接缝留给 M2/B2）
    #[allow(dead_code)]
    kind: MetaKind,
    /// 作用域外的名字（= 该位**更晚**的望远镜参数名；赋值时不许提到它们）
    out_of_scope: Vec<String>,
}

/// 单次 `unify` 的递归深度上限。
const MAX_DEPTH: u32 = 64;
/// `unify_all` 的轮数上限。
const MAX_ROUNDS: u32 = 8;
/// 一次求解的总步数上限（保护**失败路径**：学习者写错时的额外成本是常数）。
const DEFAULT_FUEL: u32 = 4096;
/// **撞预算之后允许「加大预算重试」几次** ✓（G-88 真修，2026-10-04 ✓）。
///
/// 为什么要有它 ✗：预算耗尽**不是**「无解」✗ —— 它只说明"**这一步没算完**"✓。
/// 正解 = **加大预算继续算** ✓（这就是「只允许变慢」✓ 的字面实现 ✓），
/// 而不是把「没算完」当成「不成立」✗。
/// 上界是**常数倍**（fuel ≤ `DEFAULT_FUEL × 2^(N+1)` ✓、depth ≤ `MAX_DEPTH × 2^N` ✓）
/// ⇒ 不会挂死 ✓（`unify_all` 自己还有 `MAX_ROUNDS` 那道轮数闸 ✓）。
const MAX_ESCALATIONS: u32 = 6;

/// **预算取值的取证口**（2026-10-04 ✓）：默认 = 上面那两个常量 ✓（**一个不动** ✓），
/// 但**能拧到 1** ✓ —— 判据要能回答「把预算逼到极限，结论会不会变」✓
/// （「预算耗尽只允许变慢」这句话如果是真的，拧到 1 结论必须**逐字节相同** ✓）。
/// ⚠ 这是**取证口** ✗，不是用户配置面 —— 用户面走 `CompileOptions` + `[limits]`（下一笔 ✓）。
/// 命名对齐 Lean ✓（`maxHeartbeats` = fuel ✓ / `maxRecDepth` = depth ✓）。
fn meta_limit(name: &str, default: u32) -> u32 {
    static FUEL: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    static DEPTH: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let slot = if name == "SOKO_LIMIT_MAX_HEARTBEATS" { &FUEL } else { &DEPTH };
    *slot.get_or_init(|| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(default)
            .max(1)
    })
}
/// `zonk` 的链式代换轮数上限（赋值链 `?a := ?b`、`?b := Nat` 这种）。
const MAX_ZONK_ROUNDS: u32 = 8;

/// 求解上下文（**一次 `solve_prefix` 一个**；严格档一次都不构造 ⇒ 默认路径零分配 ✓）。
/// **IA-4 B2（2026-10-05 ✓）**：元变量的**数据**（**无生命周期** ✓）——
/// 它要**跨一次求解调用**活着 ✓（用户 00:05：「允许活过一次求解调用」✓），
/// 所以从 `MetaCtx` 里抽出来 ✗（ctx 带 `unfold` 的生命周期 ✗，数据不带 ✓）。
/// ⚠ 字段全是 **owned** ✓（`MVar` 存 `Expr` ✓、`postponed` 存 `(Expr, Expr)` ✓）⇒ **零生命周期** ✓。
#[derive(Default)]
pub(crate) struct MetaStore {
    mvars: Vec<MVar>,
    /// 待定约束（`Undef` 的叶子自己压进来；`unify_all` 重扫到不动点）
    postponed: Vec<(Expr, Expr)>,
    /// **宇宙约束**（U1 第 2 片 ✓；§4 的 #6 行：「`postponed` 含宇宙约束，**U1 起分表**」✓）。
    ///
    /// ⚠ **为什么分表** ✗：项约束与层约束的**求解时机不同** ✓ —— 层约束要
    /// **推迟到项元变量被赋值之后**（Lean 的 `postponeIsLevelDefEq` ✓，§2.9 ✓），
    /// 而项约束是 `unify_all` 当场重扫 ✓ ⇒ 混在一起会让层约束被**过早**求解 ✗。
    ///
    /// ⚠ **本片只落存储** ✓（零行为变化 ✓ —— 没有生产调用方 ✓，合一与惰性检查是 ③/④ ✓）。
    #[allow(dead_code)] // U1 第 3/4 片（合一 + 惰性检查）才读；本片只有单测用
    pub(crate) univ: Vec<UnivConstraint>,
}

/// **宇宙约束**（U1 第 2 片 ✓；§2.9：「`ULe`/`UEq`/`ULub` **三种够用**」✓）。
///
/// **Lean 4 对照** ✓（本机源码 HEAD `d0493e4c1e` ✓）：Lean 的约束是
/// `LevelDefEq` 里的 `ULevel` 问题（`ULe`/`UEq`/`ULub` 一族 ✓），
/// **刚性约束推迟**到项元变量被赋值（`postponeIsLevelDefEq` ✓）。
///
/// ⚠ **id 都是层元变量的 id** ✓（`level_name(id)` 的那个 id ✓）⇒ 与 `Level` 表示解耦 ✓。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // U1 第 3/4 片（合一 + 惰性检查）会用
pub(crate) enum UnivConstraint {
    /// `lhs ≤ rhs`。
    Le { lhs: u32, rhs: u32 },
    /// `lhs = rhs`。
    Eq { lhs: u32, rhs: u32 },
    /// `out = max(a, b)`。
    Lub { out: u32, a: u32, b: u32 },
}

pub(crate) struct MetaCtx<'a> {
    /// **数据借自 `MetaStore`** ✓（B2：让它可以跨求解活着 ✓ —— 今天调用方一次求解一个 store ✓，
    /// B2 会把它挂到声明级 ✓）。
    store: &'a mut MetaStore,
    fuel: u32,
    depth: u32,
    /// **本次求解的递归深度上限** ✓（撞了就**加倍**，见 `unify_impl` ✓）。
    /// 它是**可变**的 ✗→✓（先前直接用常量 `MAX_DEPTH` ⇒ 撞上就只能判否 ✗）。
    max_depth: u32,
    /// 已经「加大预算重试」过几次 ✓（上界 `MAX_ESCALATIONS` ✓）。
    escalations: u32,
    /// 本次求解**第一个**硬错误（M3 的通道归因；`unify` 把它折成 `Tri::No`，但通道要留住）
    first_err: Option<MetaErr>,
    /// **模板/实参两侧的 delta 展开兜底**（M0 的 S13 硬约束：不接它就会把
    /// 「两条约束 defeq 一致」的形状误判成刚性冲突 ✗）
    unfold: &'a dyn Fn(&Expr) -> Expr,
}

impl<'a> MetaCtx<'a> {
    pub(crate) fn new(unfold: &'a dyn Fn(&Expr) -> Expr, store: &'a mut MetaStore) -> Self {
        MetaCtx {
            store,
            fuel: meta_limit("SOKO_LIMIT_MAX_HEARTBEATS", DEFAULT_FUEL),
            depth: 0,
            max_depth: meta_limit("SOKO_LIMIT_MAX_REC_DEPTH", MAX_DEPTH),
            escalations: 0,
            first_err: None,
            unfold,
        }
    }

    /// 建一个元变量。`out_of_scope` = 该位**更晚**的望远镜参数名（作用域检查用）。
    pub(crate) fn fresh(&mut self, ty: Expr, kind: MetaKind, out_of_scope: Vec<String>) -> MetaId {
        let id = MetaId(self.store.mvars.len() as u32);
        self.store.mvars.push(MVar {
            ty,
            value: None,
            kind,
            out_of_scope,
        });
        id
    }

    /// 元变量在表达式里的形态（`\0soko_m{id}` 这个 `Ident`）。
    pub(crate) fn meta_expr(&self, m: MetaId) -> Expr {
        Expr::Ident {
            name: meta_name(m.0),
            span: Span::default(),
        }
    }

    #[allow(dead_code)] // M2 的调用方（`implicit::solve_prefix` 的接线）会用
    pub(crate) fn value(&self, m: MetaId) -> Option<Expr> {
        self.store.mvars[m.0 as usize].value.clone()
    }

    /// **出口 zonk**：把已赋值的元变量代进 `e`（链式赋值有界迭代）。
    pub(crate) fn zonk(&self, e: &Expr) -> Expr {
        let mut sigma: HashMap<String, Expr> = HashMap::new();
        for (i, m) in self.store.mvars.iter().enumerate() {
            if let Some(v) = &m.value {
                sigma.insert(meta_name(i as u32), v.clone());
            }
        }
        if sigma.is_empty() {
            return e.clone();
        }
        let mut cur = e.clone();
        for _ in 0..MAX_ZONK_ROUNDS {
            let next = crate::spine::substitute(&cur, &sigma);
            if crate::spine::same_shape(&next, &cur) {
                break;
            }
            cur = next;
        }
        cur
    }

    /// 赋值（**occurs + type-occurs + 作用域**三道闸门；M3 起再加 sort/kind 检查）。
    pub(crate) fn assign(&mut self, m: MetaId, v: Expr) -> Result<(), MetaErr> {
        let v = self.zonk(&v);
        if crate::spine::mentions(&meta_name(m.0), &v) {
            return Err(MetaErr::Occurs);
        }
        // type-occurs：解里提到的**未解元变量**，其声明类型里不许出现 `m`（防经类型的环）
        for other in meta_ids_in(&v) {
            if other == m {
                continue;
            }
            if crate::spine::mentions(&meta_name(m.0), &self.store.mvars[other.0 as usize].ty) {
                return Err(MetaErr::TypeOccurs);
            }
        }
        for name in &self.store.mvars[m.0 as usize].out_of_scope {
            if !name.is_empty() && crate::spine::mentions(name, &v) {
                return Err(MetaErr::Scope);
            }
        }
        // **sort/kind 检查**（M3，设计 §2.5）：`?m : T` 要求 `v : T` **恰好**成立
        // （本内核**非累积**）。三值语法近似：**只拒"确定错"的**，`None` = 不知道 ⇒ 放行 ✓。
        if let (Some(k), Some(n)) = (
            sort_of_value(&v),
            sort_of_type(&self.store.mvars[m.0 as usize].ty),
        ) {
            if k != n {
                return Err(MetaErr::Kind);
            }
        }
        self.store.mvars[m.0 as usize].value = Some(v);
        Ok(())
    }

    /// 合一一步：`Yes` 成立 / `No` 不成立 / `Undef` **弃权**（已压进待定队列）。
    pub(crate) fn unify(&mut self, l: &Expr, r: &Expr) -> Tri {
        self.depth = 0;
        self.unify_impl(l, r, true)
    }

    fn unify_impl(&mut self, l: &Expr, r: &Expr, allow_unfold: bool) -> Tri {
        if self.fuel == 0 || self.depth >= self.max_depth {
            // ── **G-88 真修（2026-10-04 ✓）：撞预算 = 加大预算继续算** ─────────────
            // 先前这里是 `return Tri::No;`（原话「预算耗尽 ⇒ 按**无解**处理」✗）——
            // 那是**唯一一处「判不了 ⇒ 当成否」**✗：学习者写长一点的证明，系统说
            // 「解不出来」，而**那不是真的无解** ✗（值守 13:12「这种闸我都不能接受」✓）。
            // 总规矩：**预算耗尽只允许变慢，绝不允许变错** ✓。这里逐字实现它：
            // * 还有升级额度 ⇒ **fuel / max_depth 各翻倍，继续算** ✓（= 变慢 ✓，
            //   答案**不变** ✓ —— 预算够的那次会给出同一个结论 ✓）；
            // * 升级到底还撞 ⇒ **弃权** `Tri::Undef`（= 「判不了 ⇒ 走慢路」✓），
            //   **绝不再返回 `Tri::No`** ✗（= 「判不了 ⇒ 当成否」✗）。
            // 上界是**常数倍**（见 `MAX_ESCALATIONS` ✓）⇒ 不挂死 ✓。
            // **计数出口**（G-91 ✓）：升级与最终弃权**各一个** ✓（整本课程实测
            // **两个都是 0** ✓ ⇒ 这条改动在语料上**零行为变化** ✓，判据见
            // `crates/front/tests/gate_census.rs` ✓）。
            if self.escalations < MAX_ESCALATIONS {
                self.escalations += 1;
                // ⚠ 升级基准取**默认值**（不是"当前值"✗）：这样拧到 1 时，
                // 升满 `MAX_ESCALATIONS` 次之后预算**一定 ≥ 默认** ✓
                // ⇒ 默认预算能判的，拧到 1 也一定判得出来 ✓（只是慢 ✓）。
                self.fuel = self
                    .fuel
                    .max(meta_limit("SOKO_LIMIT_MAX_HEARTBEATS", DEFAULT_FUEL))
                    .saturating_mul(2);
                self.max_depth = self
                    .max_depth
                    .max(meta_limit("SOKO_LIMIT_MAX_REC_DEPTH", MAX_DEPTH))
                    .saturating_mul(2);
                sokonanoda::gates::META_BUDGET_ESCALATED.bump();
            } else {
                sokonanoda::gates::META_BUDGET_EXHAUSTED.bump();
                return Tri::Undef;
            }
        }
        self.fuel -= 1;
        self.depth += 1;
        let out = self.unify_body(l, r, allow_unfold);
        self.depth -= 1;
        out
    }

    fn unify_body(&mut self, l: &Expr, r: &Expr, allow_unfold: bool) -> Tri {
        let l = self.zonk(l);
        let r = self.zonk(r);
        // **G-63 ④ 的修（2026-10-03 ✓）**：源码里的**记法节点**（`a = b` ⇒
        // `Expr::Notation { target: "Eq", lhs, rhs }` ✓）与 elaborate 后的
        // `Eq.{u} α a b` **结构不同** ✗ ⇒ 判成刚性冲突 ✗（决定性 trace 实测：本层**见到**了记法对 ✓
        // 但旧规则判 `false` ✗ —— 因为只比**一层** ✗：模板的尾部实参是 `Quot.mk α (qr α) a` ✓
        // 而源码操作数是 `Quot.mk (qr α) a` ✗，差在**内层的隐式 `α`** ✓）。
        // ⇒ 规则必须是**递归 + 按尾部对齐** ✓（允许 elaborate 侧多出前导隐式 ✓）。
        if notation_matches_app(&l, &r) || notation_matches_app(&r, &l) {
            return Tri::Yes;
        }
        // 快路径：结构相同（忽略 span）⇒ 立刻成立（顺序纪律：便宜检查在前）
        if crate::spine::same_shape(&l, &r) {
            return Tri::Yes;
        }
        if let Some(m) = meta_of(&l) {
            return self.tri_assign(m, r);
        }
        if let Some(m) = meta_of(&r) {
            return self.tri_assign(m, l);
        }
        // 结构分解（**两侧**；今天只做单侧匹配）
        match (&l, &r) {
            (
                Expr::App {
                    fun: f1, arg: a1, ..
                },
                Expr::App {
                    fun: f2, arg: a2, ..
                },
            ) => {
                let h = self.unify_impl(f1, f2, allow_unfold);
                if h != Tri::Yes {
                    return h;
                }
                return self.unify_impl(a1, a2, allow_unfold);
            }
            (
                Expr::Arrow {
                    domain: d1,
                    codomain: c1,
                    ..
                },
                Expr::Arrow {
                    domain: d2,
                    codomain: c2,
                    ..
                },
            ) => {
                let d = self.unify_impl(d1, d2, allow_unfold);
                if d != Tri::Yes {
                    return d;
                }
                return self.unify_impl(c1, c2, allow_unfold);
            }
            // 洞是通配（既有语义：洞不约束）
            (Expr::Hole { .. }, _) | (_, Expr::Hole { .. }) => return Tri::Yes,
            _ => {}
        }
        // delta 展开兜底（**S13 硬约束**）：任一侧展开后形状变了 ⇒ 再试一次（只展开一层）
        if allow_unfold {
            let ul = (self.unfold)(&l);
            if !crate::spine::same_shape(&ul, &l) {
                let t = self.unify_impl(&ul, &r, false);
                if t == Tri::Yes {
                    return Tri::Yes;
                }
            }
            let ur = (self.unfold)(&r);
            if !crate::spine::same_shape(&ur, &r) {
                let t = self.unify_impl(&l, &ur, false);
                if t == Tri::Yes {
                    return Tri::Yes;
                }
            }
        }
        // 刚性冲突 vs 弃权：还有**未解**元变量 ⇒ 弃权（压进待定队列，等后续赋值）
        if has_unassigned_meta(&l) || has_unassigned_meta(&r) {
            self.store.postponed.push((l, r));
            return Tri::Undef;
        }
        // 刚性冲突：**归因**（M3 的三通道之一），调用方据此换一句 message（码不变）
        if self.first_err.is_none() {
            self.first_err = Some(MetaErr::Clash);
        }
        // **G-63 第一刀：纯诊断 ✓（判定路径一字未动 ✗）**。台账 G-63 的 2026-10-02 诊断点名
        // 这是"下一步最便宜"的一步 ✓：先钉死**两条线索到底给出了哪两个落地项** ✓，
        // 再决定修法（候选：冲突前对两侧都做 delta 归一 / 赋值时优先取折叠形态 ✓）。
        // 只在 `SOKO_CLASH_TRACE=1` 下打 stderr ✓ —— 默认零输出、零行为变化 ✓。
        // ── **G-63 的钉死结论（2026-10-03 ✓，代码出处逐条）** ─────────────────────
        // ① 左边那个 `\0soko_p{i}` 是**签名模板的 fresh 形参名** —— 唯一构造点
        //    `implicit.rs:56`（`telescope()` ✓）：把签名每个参数名换成 NUL 前缀的 fresh 名，
        //    理由是"求解器按名字代换，签名名与用户变量同名会**捕获**"（`implicit.rs:40-45` ✓）。
        //    实测 `Quot.sound` 打出 `左= soko_p2` ✓ = 签名 `{A : Sort u} -> {r : A -> A -> Prop}
        //    -> (a : A) -> …` 里 `A` 的 fresh 名 ✓。
        // ② 右边 `a` 是**用户实参**（`Quot.sound a b h` 的 `a` ✓）。
        // ③ 设计上模板名**必须**先绑元变量（`implicit.rs:194-197` 的 ② 步"望远镜名 → 元变量"✓）⇒
        //    比较里该出现**元变量**，**绝不该出现裸 fresh 名** ✗。裸名出现 ⇒ 某个元变量被**赋成了
        //    未代换的模板名**（`assign` 在 `meta.rs:190` ✓，它只 `zonk` 不代换 `sigma` ✗），
        //    或者该 fresh 名压根没建元变量 ✗ ⇒ 于是模板名与实参被当成**两个刚性头** ⇒ `Clash` ✓。
        // ⇒ **归因路径没错**（它如实报了"两条线索给出不同落地项" ✓）；错在**上游**：
        //    `solve_prefix_meta` 的 ②/③ 步（模板代换 / 后续层域 ≟ 实参类型 ✓）。
        // ⇒ **不是"产品侧本来就该接受这一对"** ✗ —— 同一件事（`A`）的两种写法不该判冲突 ✓。
        // 下一步（未做 ✗）：在 ③ 的两条约束里各打一次"这一对是从哪条约束来的"（加 `SOKO_CLASH_TRACE`
        // 的第二行 ✓），确认是"赋成未代换模板名"还是"没建元变量" ✓，再动 `solve_prefix_meta` ✓
        // （反向臂现成：`SOKO_METAVAR=sibling|off` ✓ + `SOKO_CLASH_TRACE=1` 看裸名是否消失 ✓）。
        // **G-63 ④**：上一版归一插在 `unify_impl` 的 match 里**没被走到** ✗ ⇒ 这里把**调用链**打出来 ✓
        // （只在 `SOKO_CLASH_TRACE=1` 下 ✓），钉死是**哪条路**把这一对顶到刚性冲突的 ✓。
        if std::env::var("SOKO_CLASH_TRACE").is_ok() {
            eprintln!(
                "[clash-backtrace] {}",
                std::backtrace::Backtrace::force_capture()
            );
        }
        if std::env::var("SOKO_CLASH_TRACE").is_ok() {
            // **渲染会折叠记法**（`Eq.{u} …` 显示成 `= …` ✓）⇒ 光看渲染分不出"真不同"还是"只差写法" ✗
            // ⇒ 同时打**结构 Debug**（G-63 ④ 的钉死就靠它 ✓）。
            eprintln!(
                "[clash] 左={} ｜ 右={}\n[clash-raw] 左={l:?}\n[clash-raw] 右={r:?}",
                crate::proof::render_expr(&l),
                crate::proof::render_expr(&r)
            );
        }
        Tri::No
    }

    /// 跑到不动点：`Yes` 队列空 / `No` 有硬冲突 / `Undef` 还剩待定。
    /// **停条件 = 待定计数不再严格下降**（与 Lean `processPostponed` 同款）。
    pub(crate) fn unify_all(&mut self) -> Tri {
        for _ in 0..MAX_ROUNDS {
            if self.store.postponed.is_empty() {
                return Tri::Yes;
            }
            let before = self.store.postponed.len();
            let batch = std::mem::take(&mut self.store.postponed);
            for (l, r) in batch {
                // 仍然卡住的叶子会**自己重新压回** `postponed`
                if self.unify(&l, &r) == Tri::No {
                    return Tri::No;
                }
            }
            if self.store.postponed.is_empty() {
                return Tri::Yes;
            }
            if self.store.postponed.len() >= before {
                // **弃权**（正当 ✓：判不了 ⇒ 走慢路 ✓）—— 但仍要有出口 ✗（G-91 ✓）。
                sokonanoda::gates::UNIFY_NO_PROGRESS.bump();
                return Tri::Undef; // 没有净进展 ⇒ 停（不是无界工作队列 ✓）
            }
        }
        // 轮数用光 ⇒ 同样是**弃权** ✓（不是"判否"✗）—— 出口同上 ✓。
        sokonanoda::gates::UNIFY_ROUNDS_EXHAUSTED.bump();
        Tri::Undef
    }

    /// **E19 选择规则的显式化**（设计 §2.7）：未解的位借**声明类型同形**的**已解**兄弟的值
    /// （按 index 升序取第一个）。**这不是推理、是选择** —— 与 `fill_pending_by_shape` 同一判据。
    pub(crate) fn default_unresolved(&mut self) -> Result<(), MetaErr> {
        for i in 0..self.store.mvars.len() {
            if self.store.mvars[i].value.is_some() {
                continue;
            }
            let ti = self.zonk(&self.store.mvars[i].ty.clone());
            let mut picked: Option<Expr> = None;
            for j in 0..self.store.mvars.len() {
                if i == j {
                    continue;
                }
                let Some(v) = self.store.mvars[j].value.clone() else {
                    continue;
                };
                let tj = self.zonk(&self.store.mvars[j].ty.clone());
                if crate::spine::same_shape(&ti, &tj) {
                    picked = Some(v);
                    break;
                }
            }
            if let Some(v) = picked {
                self.assign(MetaId(i as u32), v)?;
            }
        }
        Ok(())
    }

    pub(crate) fn unsolved(&self) -> Vec<MetaId> {
        self.store.mvars
            .iter()
            .enumerate()
            .filter(|(_, m)| m.value.is_none())
            .map(|(i, _)| MetaId(i as u32))
            .collect()
    }

    fn tri_assign(&mut self, m: MetaId, v: Expr) -> Tri {
        match self.assign(m, v) {
            Ok(()) => Tri::Yes,
            Err(e) => {
                if self.first_err.is_none() {
                    self.first_err = Some(e);
                }
                Tri::No
            }
        }
    }

    /// 本次求解的**通道**（M3）：`Kind` / `Clash` / `Unsolved`（都映射到**既有码**）。
    pub(crate) fn channel(&self) -> MetaSolve {
        match &self.first_err {
            Some(MetaErr::Kind) => MetaSolve::Kind,
            Some(_) => MetaSolve::Clash,
            None => MetaSolve::Unsolved,
        }
    }

    /// **出口**：不动点 + defaulting + 全部解出 + zonk。任一不成立 ⇒ 对应的**通道**
    /// （调用方照旧报既有码，只是 hint 说哪一句不同 ✓）。
    pub(crate) fn discharge(&mut self, ids: &[MetaId]) -> MetaSolve {
        if self.unify_all() == Tri::No {
            return self.channel();
        }
        // 中途有过**硬错误**（kind 不对 / 刚性冲突）⇒ 这次求解已经知道是坏的，直接归因
        // （否则会被"还有未解元变量"盖成 `Unsolved`，通道就丢了 ✗）
        if self.first_err.is_some() {
            return self.channel();
        }
        if let Err(e) = self.default_unresolved() {
            return match e {
                MetaErr::Kind => MetaSolve::Kind,
                _ => MetaSolve::Clash,
            };
        }
        if !self.unsolved().is_empty() {
            return MetaSolve::Unsolved;
        }
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let v = self.zonk(&self.meta_expr(*id));
            // 出口自检：**不许留元变量**（设计红线 1）
            if has_unassigned_meta(&v) {
                return MetaSolve::Unsolved;
            }
            out.push(v);
        }
        MetaSolve::Solved(out)
    }

    /// **IA-4 B2 第 2 步（2026-10-05 ✓）**：出口的**放宽版** ✓ —— **不要求全解出** ✗。
    ///
    /// 与 [`Self::discharge`] 的区别**只有一处** ✗：第 4 步那道 `unsolved()` 闸**不开** ✓ ——
    /// 已解的照旧 `zonk` ✓，**未解的连 id 一起返回** ✓（它们的 `postponed` **留在 store 里** ✓，
    /// 等**声明末尾**再 `unify_all` ✓）。
    ///
    /// ⚠ **红线 1 仍然守** ✓：本方法**只允许在 elaborate 期用** ✗ —— 返回的 `Vec<Expr>` 里
    /// **可能含元变量名** ✓（`\0soko_m{id}` ✓），它们**绝不许进内核** ✗（K1 的
    /// `add_declar` 入口硬拒是兜底 ✓，`builder.rs:399` ✓）。
    ///
    /// ⚠ **硬错误照旧失败** ✓（`kind` 不对 / 刚性冲突 ⇒ 通道归因 ✓，不会因为"允许未解"被盖掉 ✗）。
    // ⚠ **本步还没有生产调用方** ✗（B2 第 3 步才把它接进 `Lambda` 臂 ✓）⇒
    // 仓库对 dead code 严格 ✓ ⇒ 显式放行 + 写明理由 ✓（同 `probe_tag` 的处置 ✓）。
    #[allow(dead_code)]
    pub(crate) fn discharge_keep_unsolved(
        &mut self,
        ids: &[MetaId],
    ) -> Result<(Vec<Expr>, Vec<MetaId>), MetaSolve> {
        if self.unify_all() == Tri::No {
            return Err(self.channel());
        }
        if self.first_err.is_some() {
            return Err(self.channel());
        }
        if let Err(e) = self.default_unresolved() {
            return Err(match e {
                MetaErr::Kind => MetaSolve::Kind,
                _ => MetaSolve::Clash,
            });
        }
        let mut out = Vec::with_capacity(ids.len());
        let mut still = Vec::new();
        for id in ids {
            let v = self.zonk(&self.meta_expr(*id));
            if has_unassigned_meta(&v) {
                // **未解** ✓ ⇒ **原样返回元变量名** ✓（不进内核 ✗）+ 记下 id ✓。
                still.push(*id);
            }
            out.push(v);
        }
        Ok((out, still))
    }
}

/// `v : Sort n` 的那个 `n`（**三值**：`None` = 不知道 ⇒ 调用方放行）。
///
/// 只认**语法上是 sort** 的形状（设计 §2.5 的表）：`Prop`=1 · `Type`(`Type 0`)=2 ·
/// `Sort(n)`=n+1 · `Sort(Level(_))`/`App`/`Lambda`/`Ident`/其余 = **不知道**。
/// ⚠ 局部变量那一档（查书写类型）留给后续片：**放行**是保守方向（不会假拒绝）✓。
pub(crate) fn sort_of_value(v: &Expr) -> Option<u64> {
    match v {
        Expr::Sort {
            sort: crate::ast::SortKind::Prop,
            ..
        } => Some(1),
        Expr::Sort {
            sort: crate::ast::SortKind::Type,
            ..
        } => Some(2),
        Expr::Sort {
            sort: crate::ast::SortKind::Sort(n),
            ..
        } => Some(n + 1),
        _ => None,
    }
}

/// 声明类型 `t` 作为 sort 的层级：`Prop`=0 · `Type`=1 · `Sort(n)`=n · 其余 = 不知道。
fn sort_of_type(t: &Expr) -> Option<u64> {
    match t {
        Expr::Sort {
            sort: crate::ast::SortKind::Prop,
            ..
        } => Some(0),
        Expr::Sort {
            sort: crate::ast::SortKind::Type,
            ..
        } => Some(1),
        Expr::Sort {
            sort: crate::ast::SortKind::Sort(n),
            ..
        } => Some(*n),
        _ => None,
    }
}

/// 这个表达式**就是**一个元变量吗？
fn meta_of(e: &Expr) -> Option<MetaId> {
    if let Expr::Ident { name, .. } = e {
        if is_meta_name(name) {
            return name[META_PREFIX.len()..].parse::<u32>().ok().map(MetaId);
        }
    }
    None
}

/// 表达式里出现的**未解**元变量（`zonk` 之后剩下的元变量名都是未解的）。
/// 遍历形状与 [`crate::spine::mentions`] 同一份（**漏一个变体就会漏一次检查** ✗）。
fn meta_ids_in(e: &Expr) -> Vec<MetaId> {
    let mut out = Vec::new();
    collect_meta_ids(e, &mut out);
    out
}

fn collect_meta_ids(e: &Expr, out: &mut Vec<MetaId>) {
    if let Some(m) = meta_of(e) {
        out.push(m);
        return;
    }
    match e {
        Expr::Ident { .. }
        | Expr::UniverseApp { .. }
        | Expr::Sort { .. }
        | Expr::Num { .. }
        | Expr::Hole { .. }
        | Expr::By { .. } => {}
        Expr::App { fun, arg, .. } => {
            collect_meta_ids(fun, out);
            collect_meta_ids(arg, out);
        }
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            for el in elements {
                collect_meta_ids(el, out);
            }
        }
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            for b in binders {
                if let Some(t) = b.ty.as_deref() {
                    collect_meta_ids(t, out);
                }
            }
            collect_meta_ids(body, out);
        }
        Expr::Arrow {
            domain, codomain, ..
        } => {
            collect_meta_ids(domain, out);
            collect_meta_ids(codomain, out);
        }
        Expr::Plus { lhs, rhs, .. } => {
            collect_meta_ids(lhs, out);
            collect_meta_ids(rhs, out);
        }
        Expr::Let {
            binder, val, body, ..
        } => {
            if let Some(t) = binder.ty.as_deref() {
                collect_meta_ids(t, out);
            }
            collect_meta_ids(val, out);
            collect_meta_ids(body, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_meta_ids(scrutinee, out);
            for arm in arms {
                if let Some(g) = arm.guard.as_ref() {
                    collect_meta_ids(g, out);
                }
                collect_meta_ids(&arm.body, out);
            }
        }
        Expr::Notation { lhs, rhs, .. } => {
            if let Some(l) = lhs.as_deref() {
                collect_meta_ids(l, out);
            }
            if let Some(r) = rhs.as_deref() {
                collect_meta_ids(r, out);
            }
        }
    }
}

fn has_unassigned_meta(e: &Expr) -> bool {
    !meta_ids_in(e).is_empty()
}

/// **G-63 ④**：记法节点（源码形态 ✓）与 `target(lhs, rhs)` 应用（elaborate 后 ✓）判同一件事 ✓。
fn notation_matches_app(nota: &Expr, app: &Expr) -> bool {
    let Expr::Notation {
        target, lhs, rhs, ..
    } = nota
    else {
        return false;
    };
    let (Some(l), Some(r)) = (lhs.as_deref(), rhs.as_deref()) else {
        return false;
    };
    let span = nota.span();
    let source = Expr::App {
        fun: Box::new(Expr::App {
            fun: Box::new(Expr::Ident {
                name: target.clone(),
                span,
            }),
            arg: Box::new(l.clone()),
            explicit_spine: false,
            span,
        }),
        arg: Box::new(r.clone()),
        explicit_spine: false,
        span,
    };
    tail_aligned_eq(&source, app)
}

/// 头名（`Ident` / `UniverseApp` 都按名字 ✓）。
fn head_name_of(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// **按尾部对齐的递归相等**（G-63 ④ ✓）：头名相同（`Ident` / `UniverseApp` 都按名字 ✓），
/// 实参**从尾部**逐个递归比较 ✓ —— 允许一侧**多出前导隐式实参**（elaborate 加上的 ✓）。
fn tail_aligned_eq(a: &Expr, b: &Expr) -> bool {
    let (ha, aa) = crate::spine::spine_of(a);
    let (hb, ab) = crate::spine::spine_of(b);
    match (head_name_of(ha), head_name_of(hb)) {
        (Some(x), Some(y)) if x == y => {}
        _ => return false,
    }
    let n = aa.len().min(ab.len());
    (0..n).all(|k| tail_aligned_eq(aa[aa.len() - 1 - k], ab[ab.len() - 1 - k]))
}

#[cfg(test)]
mod tests {
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
    /// `if fuel == 0 || depth >= MAX_DEPTH { return Tri::No }` ⇒ 下面第一条断言
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
        assert_eq!(m.value(a), Some(ident("Nat")), "升级之后 ?a 必须真的解出来 ✓");
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
    /// 「不许挂死」这条**照旧** ✓ —— 升级次数有上界（`MAX_ESCALATIONS` ✓）。
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
            m.escalations <= MAX_ESCALATIONS,
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
        assert!(!is_meta_name(&level_name(3)), "层名**不许**被认成项元变量 ✗");
        assert!(!is_level_name(&meta_name(3)), "项名**不许**被认成层元变量 ✗");
        // ② 往返 ✓。
        assert_eq!(level_id_of(&level_name(7)), Some(7), "id 往返必须一致 ✓");
        assert_eq!(level_id_of("u"), None, "普通层名不是层元变量 ✓");
        // ③ 词边界 ✓。
        let u = level_name(5);
        assert!(level_occurs_in_text(5, &u), "裸名必须命中 ✓");
        assert!(level_occurs_in_text(5, &format!("max {u} v")), "复合文本里必须命中 ✓");
        assert!(level_occurs_in_text(5, &format!("{u}+1")), "层级算术里必须命中 ✓");
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
        assert!(!level_occurs_in_text(6, &u), "id 5 的文本不许被 id 6 命中 ✗");
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
}
