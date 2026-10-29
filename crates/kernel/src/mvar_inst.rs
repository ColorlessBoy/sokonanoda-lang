//! **K1b**：`instantiate_mvars`（Lean `instantiateMVars`，`MetavarContext.lean:614/1502`）
//! + 未赋值元变量清单（Lean `getMVars` / `Level.collectMVars`）。
//!
//! 只读 `TcCtx.mvars`（项赋值 + 层赋值），分配落在当前会话 arena（`'t`）。
//! 与 Lean 同：顺着赋值链展开；`?p a ..` 且 `?p` 赋成 λ ⇒ **head-beta**（`visitApp … headBeta`）；
//! 未赋值的 `?m` 原样保留（它的类型不展开，Lean 同）。今天**没有判定路径调用它** ⇒ 零行为变化。
use crate::expr::Expr::*;
use crate::level::Level;
use crate::util::{ExprPtr, FxHashMap, LevelPtr, TcCtx};

/// 赋值链/层赋值的展开上限（防外部 `assign_level` 造的环；`unify_meta` 自己有 occurs）。
const INST_FUEL: u32 = 256;

/// [`TcCtx::unassigned_mvars`] 的结果：项元变量 id 与层元变量 id（去重、首次出现序）。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UnassignedMVars {
    pub exprs: Vec<u32>,
    pub levels: Vec<u64>,
}

impl UnassignedMVars {
    pub fn is_empty(&self) -> bool { self.exprs.is_empty() && self.levels.is_empty() }
}

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    /// 把 `e` 里**已赋值**的项/层元变量换成赋值（递归、顺链），并对 `?p a` 做 head-beta。
    pub fn instantiate_mvars(&mut self, e: ExprPtr<'t>) -> ExprPtr<'t> {
        let mut cache = FxHashMap::default();
        self.inst_mvars_go(e, &mut cache, 0)
    }

    /// 层的 `instantiate`：已赋值的 `Level::MVar` 换成赋值（顺链）。
    pub fn instantiate_level_mvars(&mut self, l: LevelPtr<'t>) -> LevelPtr<'t> { self.inst_level_go(l, 0) }

    fn inst_level_go(&mut self, l: LevelPtr<'t>, fuel: u32) -> LevelPtr<'t> {
        match self.read_level(l) {
            Level::Zero | Level::Param(..) => l,
            Level::Succ(a, _) => {
                let a2 = self.inst_level_go(a, fuel);
                if a2 == a { l } else { self.succ(a2) }
            }
            Level::Max(a, b, _) => {
                let (a2, b2) = (self.inst_level_go(a, fuel), self.inst_level_go(b, fuel));
                if a2 == a && b2 == b { l } else { self.max(a2, b2) }
            }
            Level::IMax(a, b, _) => {
                let (a2, b2) = (self.inst_level_go(a, fuel), self.inst_level_go(b, fuel));
                if a2 == a && b2 == b { l } else { self.imax(a2, b2) }
            }
            Level::MVar(id, _) => match self.mvars.level_value(id) {
                Some(v) if fuel < INST_FUEL => self.inst_level_go(v, fuel + 1),
                _ => l,
            },
        }
    }

    fn inst_mvars_go(
        &mut self,
        e: ExprPtr<'t>,
        cache: &mut FxHashMap<ExprPtr<'t>, ExprPtr<'t>>,
        fuel: u32,
    ) -> ExprPtr<'t> {
        if let Some(r) = cache.get(&e) {
            return *r;
        }
        let r = match self.read_expr(e) {
            Var { .. } | StringLit { .. } | NatLit { .. } => e,
            Meta { id, .. } => match self.mvars.value(id) {
                Some(v) if fuel < INST_FUEL => self.inst_mvars_go(v, cache, fuel + 1),
                _ => e,
            },
            Sort { level, .. } => {
                let l2 = self.inst_level_go(level, 0);
                if l2 == level { e } else { self.mk_sort(l2) }
            }
            Const { name, levels, .. } => {
                let ls = self.read_levels(levels);
                let ls2: Vec<LevelPtr<'t>> = ls.iter().map(|l| self.inst_level_go(*l, 0)).collect();
                if ls2.iter().zip(ls.iter()).all(|(a, b)| a == b) {
                    e
                } else {
                    let ls2 = self.alloc_levels(&ls2);
                    self.mk_const(name, ls2)
                }
            }
            App { .. } => {
                let mut args = Vec::new();
                let mut head = e;
                while let App { fun, arg, .. } = self.read_expr(head) {
                    args.push(arg);
                    head = fun;
                }
                args.reverse();
                let head_was_meta = matches!(self.read_expr(head), Meta { .. });
                let mut f = self.inst_mvars_go(head, cache, fuel);
                let args: Vec<ExprPtr<'t>> = args.into_iter().map(|a| self.inst_mvars_go(a, cache, fuel)).collect();
                let mut rest = args.as_slice();
                // **head-beta**（只在头是被换掉的元变量时，Lean `visitApp … headBeta` 同）。
                // **K1d**：实参可以是**开放**的（`?m x` 在 binder 下）⇒ `inst_open`（代入时按所在
                // binder 深度 lift；实参全闭 ⇒ 就是 `inst`）。以前用 `inst` ⇒ `?m #0`、`?m := λx y. x`
                // 得 `λy. #0`（被 `y` 捕获）✗，应为 `λy. #1`（K1d-T6）。
                // **一次代入全部**：剥 n 层 λ、`inst_open(b, args[..n])`。逐个代入是错的：`inst` 不 lower
                // 超出代入表的 loose var（`unwrap_or(e)`）⇒ 第一个开放实参进了剩下的 λ 之后，它的 var
                // 在下一步代入里不会降回来（K1d 把 `elab_app` 换到这里时 T2 咬出来的）。赋值是闭的 ⇒
                // `b` 只含 xs ⇒ 一次代入正确。
                if head_was_meta && f != head {
                    let (mut b, mut n) = (f, 0usize);
                    while n < rest.len() {
                        let Lambda { body, .. } = self.read_expr(b) else { break };
                        b = body;
                        n += 1;
                    }
                    if n > 0 {
                        f = self.inst_open(b, &rest[..n]);
                        rest = &rest[n..];
                    }
                }
                let mut acc = f;
                for a in rest {
                    acc = self.mk_app(acc, *a);
                }
                acc
            }
            Lambda { binder_name, binder_style, binder_type, body, .. } => {
                let t2 = self.inst_mvars_go(binder_type, cache, fuel);
                let b2 = self.inst_mvars_go(body, cache, fuel);
                if t2 == binder_type && b2 == body { e } else { self.mk_lambda(binder_name, binder_style, t2, b2) }
            }
            Pi { binder_name, binder_style, binder_type, body, .. } => {
                let t2 = self.inst_mvars_go(binder_type, cache, fuel);
                let b2 = self.inst_mvars_go(body, cache, fuel);
                if t2 == binder_type && b2 == body { e } else { self.mk_pi(binder_name, binder_style, t2, b2) }
            }
            Let { data, .. } => {
                let crate::expr::LetData { binder_name, binder_type, val, body, nondep } = *data;
                let t2 = self.inst_mvars_go(binder_type, cache, fuel);
                let v2 = self.inst_mvars_go(val, cache, fuel);
                let b2 = self.inst_mvars_go(body, cache, fuel);
                if t2 == binder_type && v2 == val && b2 == body {
                    e
                } else {
                    self.mk_let(binder_name, t2, v2, b2, nondep)
                }
            }
            Proj { ty_name, idx, structure, .. } => {
                let s2 = self.inst_mvars_go(structure, cache, fuel);
                if s2 == structure { e } else { self.mk_proj(ty_name, idx, s2) }
            }
        };
        cache.insert(e, r);
        r
    }

    /// `e` 里（顺着赋值看穿后）**仍未赋值**的项/层元变量。元变量的类型不扫（Lean `getMVars` 同）。
    pub fn unassigned_mvars(&self, e: ExprPtr<'t>) -> UnassignedMVars {
        let mut out = UnassignedMVars::default();
        let mut seen = crate::util::FxHashSet::default();
        self.unassigned_go(e, &mut out, &mut seen, 0);
        out
    }

    fn unassigned_level_go(&self, l: LevelPtr<'t>, out: &mut UnassignedMVars, fuel: u32) {
        match self.read_level(l) {
            Level::Zero | Level::Param(..) => {}
            Level::Succ(a, _) => self.unassigned_level_go(a, out, fuel),
            Level::Max(a, b, _) | Level::IMax(a, b, _) => {
                self.unassigned_level_go(a, out, fuel);
                self.unassigned_level_go(b, out, fuel);
            }
            Level::MVar(id, _) => match self.mvars.level_value(id) {
                Some(v) if fuel < INST_FUEL => self.unassigned_level_go(v, out, fuel + 1),
                _ => {
                    if !out.levels.contains(&id) {
                        out.levels.push(id);
                    }
                }
            },
        }
    }

    fn unassigned_go(
        &self,
        e: ExprPtr<'t>,
        out: &mut UnassignedMVars,
        seen: &mut crate::util::FxHashSet<ExprPtr<'t>>,
        fuel: u32,
    ) {
        if !seen.insert(e) {
            return;
        }
        match self.read_expr(e) {
            Var { .. } | StringLit { .. } | NatLit { .. } => {}
            Meta { id, .. } => match self.mvars.value(id) {
                Some(v) if fuel < INST_FUEL => self.unassigned_go(v, out, seen, fuel + 1),
                _ => {
                    if !out.exprs.contains(&id) {
                        out.exprs.push(id);
                    }
                }
            },
            Sort { level, .. } => self.unassigned_level_go(level, out, 0),
            Const { levels, .. } => {
                for l in self.read_levels(levels) {
                    self.unassigned_level_go(*l, out, 0);
                }
            }
            App { fun, arg, .. } => {
                self.unassigned_go(fun, out, seen, fuel);
                self.unassigned_go(arg, out, seen, fuel);
            }
            Lambda { binder_type, body, .. } | Pi { binder_type, body, .. } => {
                self.unassigned_go(binder_type, out, seen, fuel);
                self.unassigned_go(body, out, seen, fuel);
            }
            Let { data, .. } => {
                self.unassigned_go(data.binder_type, out, seen, fuel);
                self.unassigned_go(data.val, out, seen, fuel);
                self.unassigned_go(data.body, out, seen, fuel);
            }
            Proj { structure, .. } => self.unassigned_go(structure, out, seen, fuel),
        }
    }
}
