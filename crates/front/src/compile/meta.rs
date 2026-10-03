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
/// `zonk` 的链式代换轮数上限（赋值链 `?a := ?b`、`?b := Nat` 这种）。
const MAX_ZONK_ROUNDS: u32 = 8;

/// 求解上下文（**一次 `solve_prefix` 一个**；严格档一次都不构造 ⇒ 默认路径零分配 ✓）。
pub(crate) struct MetaCtx<'a> {
    mvars: Vec<MVar>,
    /// 待定约束（`Undef` 的叶子自己压进来；`unify_all` 重扫到不动点）
    postponed: Vec<(Expr, Expr)>,
    fuel: u32,
    depth: u32,
    /// 本次求解**第一个**硬错误（M3 的通道归因；`unify` 把它折成 `Tri::No`，但通道要留住）
    first_err: Option<MetaErr>,
    /// **模板/实参两侧的 delta 展开兜底**（M0 的 S13 硬约束：不接它就会把
    /// 「两条约束 defeq 一致」的形状误判成刚性冲突 ✗）
    unfold: &'a dyn Fn(&Expr) -> Expr,
}

impl<'a> MetaCtx<'a> {
    pub(crate) fn new(unfold: &'a dyn Fn(&Expr) -> Expr) -> Self {
        MetaCtx {
            mvars: Vec::new(),
            postponed: Vec::new(),
            fuel: DEFAULT_FUEL,
            depth: 0,
            first_err: None,
            unfold,
        }
    }

    /// 建一个元变量。`out_of_scope` = 该位**更晚**的望远镜参数名（作用域检查用）。
    pub(crate) fn fresh(&mut self, ty: Expr, kind: MetaKind, out_of_scope: Vec<String>) -> MetaId {
        let id = MetaId(self.mvars.len() as u32);
        self.mvars.push(MVar {
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
        self.mvars[m.0 as usize].value.clone()
    }

    /// **出口 zonk**：把已赋值的元变量代进 `e`（链式赋值有界迭代）。
    pub(crate) fn zonk(&self, e: &Expr) -> Expr {
        let mut sigma: HashMap<String, Expr> = HashMap::new();
        for (i, m) in self.mvars.iter().enumerate() {
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
            if crate::spine::mentions(&meta_name(m.0), &self.mvars[other.0 as usize].ty) {
                return Err(MetaErr::TypeOccurs);
            }
        }
        for name in &self.mvars[m.0 as usize].out_of_scope {
            if !name.is_empty() && crate::spine::mentions(name, &v) {
                return Err(MetaErr::Scope);
            }
        }
        // **sort/kind 检查**（M3，设计 §2.5）：`?m : T` 要求 `v : T` **恰好**成立
        // （本内核**非累积**）。三值语法近似：**只拒"确定错"的**，`None` = 不知道 ⇒ 放行 ✓。
        if let (Some(k), Some(n)) = (
            sort_of_value(&v),
            sort_of_type(&self.mvars[m.0 as usize].ty),
        ) {
            if k != n {
                return Err(MetaErr::Kind);
            }
        }
        self.mvars[m.0 as usize].value = Some(v);
        Ok(())
    }

    /// 合一一步：`Yes` 成立 / `No` 不成立 / `Undef` **弃权**（已压进待定队列）。
    pub(crate) fn unify(&mut self, l: &Expr, r: &Expr) -> Tri {
        self.depth = 0;
        self.unify_impl(l, r, true)
    }

    fn unify_impl(&mut self, l: &Expr, r: &Expr, allow_unfold: bool) -> Tri {
        if self.fuel == 0 || self.depth >= MAX_DEPTH {
            return Tri::No; // 预算耗尽 ⇒ 按「无解」处理（不新增失败面）
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
            self.postponed.push((l, r));
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
            if self.postponed.is_empty() {
                return Tri::Yes;
            }
            let before = self.postponed.len();
            let batch = std::mem::take(&mut self.postponed);
            for (l, r) in batch {
                // 仍然卡住的叶子会**自己重新压回** `postponed`
                if self.unify(&l, &r) == Tri::No {
                    return Tri::No;
                }
            }
            if self.postponed.is_empty() {
                return Tri::Yes;
            }
            if self.postponed.len() >= before {
                return Tri::Undef; // 没有净进展 ⇒ 停（不是无界工作队列 ✓）
            }
        }
        Tri::Undef
    }

    /// **E19 选择规则的显式化**（设计 §2.7）：未解的位借**声明类型同形**的**已解**兄弟的值
    /// （按 index 升序取第一个）。**这不是推理、是选择** —— 与 `fill_pending_by_shape` 同一判据。
    pub(crate) fn default_unresolved(&mut self) -> Result<(), MetaErr> {
        for i in 0..self.mvars.len() {
            if self.mvars[i].value.is_some() {
                continue;
            }
            let ti = self.zonk(&self.mvars[i].ty.clone());
            let mut picked: Option<Expr> = None;
            for j in 0..self.mvars.len() {
                if i == j {
                    continue;
                }
                let Some(v) = self.mvars[j].value.clone() else {
                    continue;
                };
                let tj = self.zonk(&self.mvars[j].ty.clone());
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
        self.mvars
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
    fn ctx_with<'a>(unfold: &'a dyn Fn(&Expr) -> Expr) -> MetaCtx<'a> {
        MetaCtx::new(unfold)
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
        assert_eq!(m.postponed.len(), 1);
        assert_eq!(m.unify_all(), Tri::Undef, "无净进展 ⇒ 停（不挂死）");
    }

    /// 刚性冲突：两个不同头的常量 ⇒ `No`（且不进待定队列）。
    #[test]
    fn rigid_clash_is_no() {
        let mut m = ctx_with(&no_unfold);
        assert_eq!(m.unify(&ident("Nat"), &ident("Bool")), Tri::No);
        assert!(m.postponed.is_empty());
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

    /// 预算：步数耗尽 ⇒ 按「无解」处理（**不许挂死**）。
    #[test]
    fn fuel_bounds_the_work() {
        let mut m = ctx_with(&no_unfold);
        m.fuel = 0;
        assert_eq!(m.unify(&ident("Nat"), &ident("Nat")), Tri::No);
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
}
