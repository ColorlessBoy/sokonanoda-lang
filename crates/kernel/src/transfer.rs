//! **F2a**：项跨会话搬运（tc 会话 arena ⇒ `EnvBuilder`），**不经文本**。
//!
//! 每次 `ExportFile::with_ctx` 都新建 arena；会话里造的 `ExprPtr<'t>`（赋值、instantiate 结果）
//! 随闭包一起死。今天前端只能 pp 成文本再 parse 回 builder（`meta_exit.rs`）——REQUIREMENTS §9.39
//! 要消掉的正是这条路。这里给一条结构化的路：
//!
//! * [`export_expr`]：任意 arena 的项 ⇒ [`OwnedExpr`]（无生命周期；按节点指针记忆 ⇒ DAG 共享保留）；
//! * [`EnvBuilder::import_owned`]：[`OwnedExpr`] ⇒ builder 项，**走 builder 自己的 `mk_*`/名字/层构造器**
//!   ⇒ hash 与 intern 指针与原生构造的项一致（同结构 ⇒ 同指针）；
//! * [`ExportFile::solve_metas`]：一次 tc 会话里跑一组 `is_def_eq` 方程，带回全部赋值（owned）。
//!
//! 纯加法：没有既有判定路径调用本模块 ⇒ 零行为变化。
use crate::builder::EnvBuilder;
use crate::env::EnvLimit;
use crate::expr::{BinderStyle, Expr, MetaKind};
use crate::level::Level;
use crate::name::Name;
use crate::util::{ExportFile, ExprPtr, FxHashMap, LevelPtr, NamePtr, TcCtx};
use num_bigint::BigUint;

/// 名字的一段（根在前）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameSeg {
    Str(String),
    Num(u64),
}

/// 结构化的名字：`Anon` = 空向量；`A.b.3` = `[Str A, Str b, Num 3]`。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OwnedName(pub Vec<NameSeg>);

/// 结构化的层（含 `Param` 与 `MVar`）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnedLevel {
    Zero,
    Succ(Box<OwnedLevel>),
    Max(Box<OwnedLevel>, Box<OwnedLevel>),
    IMax(Box<OwnedLevel>, Box<OwnedLevel>),
    Param(OwnedName),
    MVar(u64),
}

/// 一个节点；子节点是同一张 [`OwnedExpr::nodes`] 表里的下标，且**总小于**自己的下标（后序）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedNode {
    Var(u16),
    Sort(OwnedLevel),
    Const(OwnedName, Vec<OwnedLevel>),
    App(u32, u32),
    Pi { name: OwnedName, style: BinderStyle, ty: u32, body: u32 },
    Lambda { name: OwnedName, style: BinderStyle, ty: u32, body: u32 },
    Let { name: OwnedName, ty: u32, val: u32, body: u32, nondep: bool },
    Proj { ty_name: OwnedName, idx: u16, structure: u32 },
    StringLit(String),
    NatLit(BigUint),
    Meta { id: u32, kind: MetaKind, ty: u32 },
}

/// 无 arena 的项：节点表（共享子项只出现一次）+ 根下标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedExpr {
    pub nodes: Vec<OwnedNode>,
    pub root: u32,
}

impl OwnedExpr {
    /// 不同节点的个数（= DAG 大小，不是树大小）。
    pub fn node_count(&self) -> usize { self.nodes.len() }
}

/// 一个元变量种子：`(id, 类型, 种类, 创建处 binder 深度)`。
#[derive(Debug, Clone, Copy)]
pub struct MetaSeed<'p> {
    pub id: u32,
    pub ty: ExprPtr<'p>,
    pub kind: MetaKind,
    pub depth: u32,
}

/// [`ExportFile::solve_metas`] 的结果。
///
/// `exprs` 是会话里**全部**被赋值的项元变量（按 id 升序）的**原始**赋值 —— 可能提到别的已赋值元变量
/// （赋值链）⇒ 导入后把**整张表**交给 [`EnvBuilder::instantiate_metas`]。`unassigned` = 种子里没被赋值的 id
/// （种子顺序）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Solution {
    pub exprs: Vec<(u32, OwnedExpr)>,
    pub levels: Vec<(u64, OwnedLevel)>,
    pub unassigned: Vec<u32>,
}

// -- export -----------------------------------------------------------------

fn export_name(n: NamePtr<'_>) -> OwnedName {
    let mut segs = Vec::new();
    let mut cur = n.as_ref().kind;
    loop {
        match cur {
            Name::Anon => break,
            Name::Str(pfx, s, _) => {
                segs.push(NameSeg::Str(s.as_ref().to_string()));
                cur = pfx.as_ref().kind;
            }
            Name::Num(pfx, k, _) => {
                segs.push(NameSeg::Num(k));
                cur = pfx.as_ref().kind;
            }
        }
    }
    segs.reverse();
    OwnedName(segs)
}

/// 层 ⇒ [`OwnedLevel`]（层很小，不做共享）。
pub fn export_level(l: LevelPtr<'_>) -> OwnedLevel {
    match *l.as_ref() {
        Level::Zero => OwnedLevel::Zero,
        Level::Succ(a, _) => OwnedLevel::Succ(Box::new(export_level(a))),
        Level::Max(a, b, _) => OwnedLevel::Max(Box::new(export_level(a)), Box::new(export_level(b))),
        Level::IMax(a, b, _) => OwnedLevel::IMax(Box::new(export_level(a)), Box::new(export_level(b))),
        Level::Param(n, _) => OwnedLevel::Param(export_name(n)),
        Level::MVar(id, _) => OwnedLevel::MVar(id),
    }
}

struct Exporter<'e> {
    nodes: Vec<OwnedNode>,
    memo: FxHashMap<ExprPtr<'e>, u32>,
}

impl<'e> Exporter<'e> {
    fn push(&mut self, n: OwnedNode) -> u32 {
        let i = u32::try_from(self.nodes.len()).expect("more than u32::MAX nodes");
        self.nodes.push(n);
        i
    }

    fn go(&mut self, e: ExprPtr<'e>) -> u32 {
        if let Some(i) = self.memo.get(&e) {
            return *i;
        }
        let node = match *e.as_ref() {
            Expr::Var { dbj_idx, .. } => OwnedNode::Var(dbj_idx),
            Expr::Sort { level, .. } => OwnedNode::Sort(export_level(level)),
            Expr::Const { name, levels, .. } =>
                OwnedNode::Const(export_name(name), levels.iter().map(|l| export_level(*l)).collect()),
            Expr::App { fun, arg, .. } => {
                let (f, a) = (self.go(fun), self.go(arg));
                OwnedNode::App(f, a)
            }
            Expr::Pi { binder_name, binder_style, binder_type, body, .. } => {
                let (ty, body) = (self.go(binder_type), self.go(body));
                OwnedNode::Pi { name: export_name(binder_name), style: binder_style, ty, body }
            }
            Expr::Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let (ty, body) = (self.go(binder_type), self.go(body));
                OwnedNode::Lambda { name: export_name(binder_name), style: binder_style, ty, body }
            }
            Expr::Let { data, .. } => {
                let (ty, val, body) = (self.go(data.binder_type), self.go(data.val), self.go(data.body));
                OwnedNode::Let { name: export_name(data.binder_name), ty, val, body, nondep: data.nondep }
            }
            Expr::Proj { ty_name, idx, structure, .. } => {
                let structure = self.go(structure);
                OwnedNode::Proj { ty_name: export_name(ty_name), idx, structure }
            }
            Expr::StringLit { ptr, .. } => OwnedNode::StringLit(ptr.as_ref().to_string()),
            Expr::NatLit { ptr, .. } => OwnedNode::NatLit(ptr.as_ref().clone()),
            Expr::Meta { id, kind, ty, .. } => {
                let ty = self.go(ty);
                OwnedNode::Meta { id, kind, ty }
            }
        };
        let i = self.push(node);
        self.memo.insert(e, i);
        i
    }
}

/// 任意 arena 的项 ⇒ [`OwnedExpr`]（按节点指针记忆 ⇒ 共享子项只导出一次）。
pub fn export_expr(e: ExprPtr<'_>) -> OwnedExpr {
    let mut ex = Exporter { nodes: Vec::new(), memo: FxHashMap::default() };
    let root = ex.go(e);
    OwnedExpr { nodes: ex.nodes, root }
}

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    /// **F2a**：会话里的项 ⇒ [`OwnedExpr`]（可活过本会话）。
    pub fn export_owned(&self, e: ExprPtr<'t>) -> OwnedExpr { export_expr(e) }
}

// -- import -----------------------------------------------------------------

impl<'a> EnvBuilder<'a> {
    fn import_name(&mut self, n: &OwnedName) -> NamePtr<'a> {
        let mut out = self.anonymous();
        for seg in &n.0 {
            out = match seg {
                NameSeg::Str(s) => self.name_str(out, s),
                NameSeg::Num(k) => self.name_num(out, *k),
            };
        }
        out
    }

    /// **F2a**：[`OwnedLevel`] ⇒ builder 层（intern ⇒ 与原生构造同指针）。
    pub fn import_level(&mut self, l: &OwnedLevel) -> LevelPtr<'a> {
        match l {
            OwnedLevel::Zero => self.zero(),
            OwnedLevel::Succ(a) => {
                let a = self.import_level(a);
                self.succ(a)
            }
            OwnedLevel::Max(a, b) => {
                let (a, b) = (self.import_level(a), self.import_level(b));
                self.level_max(a, b)
            }
            OwnedLevel::IMax(a, b) => {
                let (a, b) = (self.import_level(a), self.import_level(b));
                self.level_imax(a, b)
            }
            OwnedLevel::Param(n) => {
                let n = self.import_name(n);
                self.level_param(n)
            }
            OwnedLevel::MVar(id) => self.level_mvar(*id),
        }
    }

    /// **F2a**：[`OwnedExpr`] ⇒ builder 项。走 `mk_*` ⇒ hash/intern 与原生构造一致（同结构 ⇒ 同指针）。
    ///
    /// 按节点表顺序一遍（子节点下标总小于父节点）⇒ 每个共享节点只建一次，无递归。
    /// `None` ⇔ 表不合法（子下标越界）或 builder 的配置关掉了 Nat/String 字面量扩展。
    pub fn import_owned(&mut self, o: &OwnedExpr) -> Option<ExprPtr<'a>> {
        let mut out: Vec<ExprPtr<'a>> = Vec::with_capacity(o.nodes.len());
        for n in &o.nodes {
            let get = |i: u32| out.get(i as usize).copied();
            let p = match n {
                OwnedNode::Var(i) => self.mk_var(*i),
                OwnedNode::Sort(l) => {
                    let l = self.import_level(l);
                    self.mk_sort(l)
                }
                OwnedNode::Const(name, ls) => {
                    let name = self.import_name(name);
                    let ls: Vec<LevelPtr<'a>> = ls.iter().map(|l| self.import_level(l)).collect();
                    let ls = self.alloc_levels_slice(&ls);
                    self.mk_const(name, ls)
                }
                OwnedNode::App(f, a) => {
                    let (f, a) = (get(*f)?, get(*a)?);
                    self.mk_app(f, a)
                }
                OwnedNode::Pi { name, style, ty, body } => {
                    let (ty, body) = (get(*ty)?, get(*body)?);
                    let name = self.import_name(name);
                    self.mk_pi(name, *style, ty, body)
                }
                OwnedNode::Lambda { name, style, ty, body } => {
                    let (ty, body) = (get(*ty)?, get(*body)?);
                    let name = self.import_name(name);
                    self.mk_lambda(name, *style, ty, body)
                }
                OwnedNode::Let { name, ty, val, body, nondep } => {
                    let (ty, val, body) = (get(*ty)?, get(*val)?, get(*body)?);
                    let name = self.import_name(name);
                    self.mk_let(name, ty, val, body, *nondep)
                }
                OwnedNode::Proj { ty_name, idx, structure } => {
                    let s = get(*structure)?;
                    let ty_name = self.import_name(ty_name);
                    self.mk_proj(ty_name, *idx, s)
                }
                OwnedNode::StringLit(s) => {
                    let s = self.alloc_string(s);
                    self.mk_string_lit(s)?
                }
                OwnedNode::NatLit(n) => {
                    let n = self.alloc_bignum(n.clone())?;
                    self.mk_nat_lit(n)?
                }
                OwnedNode::Meta { id, kind, ty } => {
                    let ty = get(*ty)?;
                    self.mk_meta_with_kind(*id, *kind, ty)
                }
            };
            out.push(p);
        }
        out.get(o.root as usize).copied()
    }

    /// **F2a**：把 [`Solution`] 的赋值导入 builder ⇒ 正好是 [`Self::instantiate_metas`] 要的两张表。
    #[allow(clippy::type_complexity)]
    pub fn import_solution(
        &mut self,
        s: &Solution,
    ) -> Option<(std::collections::HashMap<u32, ExprPtr<'a>>, std::collections::HashMap<u64, LevelPtr<'a>>)> {
        let mut exprs = std::collections::HashMap::new();
        for (id, o) in &s.exprs {
            exprs.insert(*id, self.import_owned(o)?);
        }
        let levels = s.levels.iter().map(|(id, l)| (*id, self.import_level(l))).collect();
        Some((exprs, levels))
    }
}

// -- solve ------------------------------------------------------------------

impl<'p> ExportFile<'p> {
    /// **F2a**：在**一次** tc 会话里登记 `seeds`、按顺序跑 `eqs` 的 `is_def_eq`，带回赋值（owned）。
    ///
    /// * 任一方程 `false` ⇒ `None`（`is_def_eq` 已回滚那一条的赋值；整个会话随即丢弃）；
    /// * 全部成立 ⇒ `Some(Solution)`：被赋值的项/层元变量 + 仍未赋值的种子 id；
    /// * 内部任何 panic ⇒ `None`（`catch_unwind` + 本线程静音；不外泄）。
    pub fn solve_metas(
        &self,
        limit: EnvLimit<'p>,
        seeds: &[MetaSeed<'p>],
        eqs: &[(ExprPtr<'p>, ExprPtr<'p>)],
    ) -> Option<Solution> {
        let _q = crate::quiet::quiet();
        let run = std::panic::AssertUnwindSafe(|| {
            self.with_tc(limit, |tc| {
                for s in seeds {
                    tc.ctx.mvars.register_at(s.id, s.ty, s.depth);
                    tc.ctx.mvars.register_kind(s.id, s.kind);
                }
                for (x, y) in eqs {
                    if !tc.is_def_eq(*x, *y) {
                        return None;
                    }
                }
                let mut exprs: Vec<(u32, OwnedExpr)> =
                    tc.ctx.mvars.assignments().map(|(id, v)| (id, export_expr(v))).collect();
                exprs.sort_by_key(|(id, _)| *id);
                let mut levels: Vec<(u64, OwnedLevel)> =
                    tc.ctx.mvars.level_assignments().map(|(id, l)| (id, export_level(l))).collect();
                levels.sort_by_key(|(id, _)| *id);
                let unassigned = seeds.iter().map(|s| s.id).filter(|id| tc.ctx.mvars.value(*id).is_none()).collect();
                Some(Solution { exprs, levels, unassigned })
            })
        });
        std::panic::catch_unwind(run).ok().flatten()
    }
}
