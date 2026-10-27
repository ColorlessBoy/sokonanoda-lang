//! **HO3b · FO 近似**（`SOKO_META_APPROX` 那一档；设计 `docs/design/metavar-pattern.md`
//! §3.2.3）—— Lean `processAssignmentFOApprox`（`Meta/ExprDefEq.lean:1054`）的对应物。
//!
//! 为什么单独一个文件：`pattern.rs` 是**新增文件**，`docs-lint` 判据 ⑨ 的软线是 **500 行**
//! （`AGENTS.md` 硬规则 5）⇒ 近似档整块搬出来（模块结构，不是搬家）。
//!
//! ⚠ 本文件只有 [`MetaCtx`] 的**一个 impl 块**：规则本体（严格/quasi + 全部闸门 +
//! 内核检查 + 单写）仍在 `pattern.rs`，FO 只是**它的调用者**（前缀赋值走同一条规则）。

use crate::ast::Expr;
use crate::span::Span;

use crate::compile::meta::MetaErr;

use super::{as_pattern, meta_name, MetaCtx, MetaId, MetaKind, Pattern, Tri};

impl<'a> MetaCtx<'a> {
    /// **HO3b · FO 近似**（设计 §3.2.3；Lean `processAssignmentFOApprox` `ExprDefEq.lean:1054`）：
    /// `?m a₁…aₙ ≟ f b₁…bₖ`（`k < n`）⇒ 从右往左 `a_{i+j} ≟ b_j`（`i = n − k`），
    /// 再 `?m a₁…aᵢ := f`（走**同一条** pattern 赋值 ⇒ 全部闸门 + 内核检查都在）。
    ///
    /// **为什么是一档近似**：只抽象**前缀** ⇒ 解比严格的更**不一般**（`?m a b ≟ f b`
    /// 解成 `fun a => f`、`?m a b` 只剩 `f b`）；Lean 正是这么做的（`approxDefEq` 的 `foApprox`）。
    ///
    /// ⚠ **状态**：两条 `unify` 会写**别的**元变量与待定队列 ⇒ 非 `Yes` 一律
    /// [`Self::restore`]（`values` + `postponed` + `first_err`）⇒ **判定中性** ✓。
    pub(super) fn fo_assign(&mut self, l: &Expr, r: &Expr) -> Tri {
        let l = self.zonk(l);
        let r = self.zonk(r);
        if self.lctx.is_empty() {
            return Tri::Undef;
        }
        let lctx = self.lctx.clone();
        let (p, t) = match as_pattern(&l, &lctx) {
            Some(p) => (p, r),
            None => match as_pattern(&r, &lctx) {
                Some(p) => (p, l),
                None => return Tri::Undef,
            },
        };
        self.fo_assign_one(p, &t)
    }

    /// 已识别出 `?m a₁…aₙ ≟ t`：FO 的形状闸 + 逐位 `unify` + 前缀赋值。
    fn fo_assign_one(&mut self, p: Pattern, t: &Expr) -> Tri {
        let (head, args) = crate::spine::spine_of(t);
        let n = p.args.len();
        // 形状：`t` 必须是**应用**，且至少留**一个**实参给前缀（`i ≥ 1`）；
        // `k = n`（i = 0）是裸 `?m ≟ f` 那一类 ⇒ 归 `tri_assign` / 严格档。
        if args.is_empty() || args.len() >= n {
            return Tri::Undef;
        }
        // ① 三条赋值前闸（与 `pattern_assign_one` 同源：未赋值 / 非洞 / 无反向待定约束）。
        if !self.assignable(p.meta) || self.unsettled(p.meta) {
            return Tri::Undef;
        }
        // ⑧ 预算（FO 每次至少两次 `unify`，先看一次）。
        if self.fuel == 0 {
            return Tri::Undef;
        }
        let i = n - args.len();
        // ② 快照（设计 §3.2.5：失败 restore ⇒ 判定中性）。
        let values: Vec<Option<Expr>> = self.store.mvars.iter().map(|m| m.value.clone()).collect();
        let postponed = self.store.postponed.clone();
        let saved_err = self.first_err.clone();
        // ③ 从右往左逐位（Lean 的顺序：先 `args.back ≟ a.back`）。
        for (j, b) in args.iter().enumerate().rev() {
            let arg = Expr::Ident {
                name: p.args[i + j].clone(),
                span: Span::default(),
            };
            if self.unify(&arg, b) != Tri::Yes {
                self.restore(values, postponed, saved_err);
                return Tri::Undef;
            }
        }
        // ④ 前缀赋值：`?m a₁…aᵢ := f`（**同一条**规则 ⇒ 闸门/内核检查/单写全在）。
        let prefix = Pattern {
            meta: p.meta,
            args: p.args[..i].to_vec(),
        };
        match self.pattern_assign_one(prefix, head) {
            Tri::Yes => Tri::Yes,
            _ => {
                self.restore(values, postponed, saved_err);
                Tri::Undef
            }
        }
    }

    /// **赋值前**的三条闸（FO 与 `pattern_assign_one` 的 ① 同一条判据）：
    /// 未赋值、不是 `SyntheticOpaque`、且在本 store 里。
    fn assignable(&self, m: MetaId) -> bool {
        let Some(idx) = m.0.checked_sub(self.store.base) else {
            return false;
        };
        let Some(mv) = self.store.mvars.get(idx as usize) else {
            return false;
        };
        mv.value.is_none() && mv.kind != MetaKind::SyntheticOpaque
    }

    /// 待定约束闸（设计 §3.2(iii)；与 ⑥ 同一判据）：有把它往**别的方向**拉的约束 ⇒ `true`。
    fn unsettled(&self, m: MetaId) -> bool {
        let name = meta_name(m.0);
        self.store
            .postponed
            .iter()
            .any(|(l, r)| crate::spine::mentions(&name, l) || crate::spine::mentions(&name, r))
    }

    /// 把状态退回到快照（`values` + `postponed` + `first_err`）——FO 用。
    fn restore(
        &mut self,
        values: Vec<Option<Expr>>,
        postponed: Vec<(Expr, Expr)>,
        saved_err: Option<MetaErr>,
    ) {
        for (m, v) in self.store.mvars.iter_mut().zip(values) {
            m.value = v;
        }
        self.store.postponed = postponed;
        self.first_err = saved_err;
    }
}

/// 自由名（不含类型内部 binder）：B3 守卫用（外层 `pp_type_scope_safe` 经 pattern 重导出）。
pub(crate) fn free_idents(e: &Expr) -> Vec<String> {
    let mut bound = Vec::new();
    let mut used = Vec::new();
    let mut binders = Vec::new();
    super::free_names(e, &mut bound, &mut used, &mut binders);
    used
}
