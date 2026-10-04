use crate::tc::TypeChecker;
use crate::util::{LevelPtr, LevelsPtr, NamePtr};
use crate::value::{Spine, Value, S};

/// **签名能记多少个参数**（G-90）：取值走 [`crate::gates::limits::max_tracked`] ✓
/// —— 默认 64 不变 ✓（`u64` 掩码的硬上界 ✓），但**能拧到 1** 以证明
/// 「丢精度只变慢、不变错」✓（判据见 `docs/gaps/repro/G90-*.sh` ✓）。
#[inline]
pub(crate) fn max_tracked() -> u32 {
    crate::gates::limits::max_tracked()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sig {
    pub(crate) arity: u8,
    pub(crate) prop_arg: u64,
    pub(crate) arg_known: u64,
    pub(crate) absent_arg: u64,
    pub(crate) prop_result: u64,
    pub(crate) result_known: u64,
}

impl Sig {
    pub(crate) const ALL_RELEVANT: Sig =
        Sig { arity: 0, prop_arg: 0, arg_known: 0, absent_arg: 0, prop_result: 0, result_known: 0 };

    #[inline]
    fn ignorable(&self) -> u64 { (self.prop_arg & self.arg_known) | self.absent_arg }

    #[inline]
    pub(crate) fn masks_any_arg(&self) -> bool { self.ignorable() != 0 }

    #[inline]
    pub(crate) fn arg_is_ignorable(&self, idx: u32) -> bool {
        idx < max_tracked() && (self.ignorable() >> idx) & 1 == 1
    }

    #[inline]
    pub(crate) fn result_is_not_proof(&self, k: u32) -> bool {
        k < max_tracked() && (self.result_known >> k) & 1 == 1 && (self.prop_result >> k) & 1 == 0
    }
}

pub(crate) fn app_prefix_len(spine: S<'_>) -> u32 {
    if !spine.has_proj() {
        return spine.len();
    }
    let mut limit = spine.len();
    let mut cur = spine;
    while let Spine::Snoc { prev, elim, .. } = cur {
        if !elim.is_app() {
            limit = prev.len();
        }
        cur = prev;
    }
    limit
}

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    pub(crate) fn sig_of(&mut self, name: NamePtr<'t>, levels: LevelsPtr<'t>) -> Sig {
        if self.env.has_temp_ext() {
            return Sig::ALL_RELEVANT;
        }
        if let Some(s) = self.ctx.sig_cache.get(&(name, levels)) {
            return *s;
        }
        if !self.ctx.sig_computing.insert((name, levels)) {
            return Sig::ALL_RELEVANT;
        }
        let s = self.sig_compute(name, levels);
        self.ctx.sig_computing.remove(&(name, levels));
        self.ctx.sig_cache.insert((name, levels), s);
        s
    }

    fn sig_compute(&mut self, name: NamePtr<'t>, levels: LevelsPtr<'t>) -> Sig {
        let mut dom: Vec<Option<LevelPtr<'t>>> = Vec::new();
        let mut cur = self.const_head_type(name, levels);
        let mut depth = 0u32;
        let terminal = loop {
            let cur_f = self.force_all(depth, cur);
            let Value::Pi { domain, body, .. } = cur_f else { break Some(cur_f) };
            if dom.len() >= max_tracked() as usize {
                // **闸类计数出口** ✓（G-90/G-91）：望远镜超过 64 位 ⇒ 签名**丢精度** ✗
                // （`terminal = None` ⇒ 结果那一格判不了 ✓）。丢精度只该**变慢** ✓
                // （少一条捷径 ✓），**绝不许**变错 ✗ —— 但**必须可见** ✗。
                // 终点 = 对齐 Lean 的 `synthInstance.maxSize = 128` ✓。
                crate::gates::SIG_OVERFLOW.bump();
                break None;
            }
            let d = *domain;
            dom.push(self.level_of_type(depth, d));
            let fresh = self.mk_bvar_hc(depth, d);
            cur = self.apply_closure(depth + 1, body, fresh, Some(d));
            depth += 1;
        };

        let n = dom.len();
        let mut prop_arg = 0u64;
        let mut arg_known = 0u64;
        for i in 0..n {
            if let Some(l) = dom[i] {
                arg_known |= 1u64 << i;
                if self.ctx.is_zero(l) {
                    prop_arg |= 1u64 << i;
                }
            }
        }

        let mut prop_result = 0u64;
        let mut result_known = 0u64;
        if let Some(term) = terminal {
            if let Some(sb) = self.level_of_type(depth, term) {
                let mut r = sb;
                if n < max_tracked() as usize {
                    result_known |= 1u64 << n;
                    if self.ctx.is_zero(r) {
                        prop_result |= 1u64 << n;
                    }
                } else {
                    // 结果那一格超出掩码宽度 ⇒ 同样丢精度 ✓（计数出口，见上 ✓）。
                    crate::gates::SIG_OVERFLOW.bump();
                }
                for k in (0..n).rev() {
                    let Some(s) = dom[k] else { break };
                    let im = self.ctx.imax(s, r);
                    r = self.ctx.simplify(im);
                    result_known |= 1u64 << k;
                    if self.ctx.is_zero(r) {
                        prop_result |= 1u64 << k;
                    }
                }
            }
        }

        Sig {
            arity: u8::try_from(n).expect("telescope arity exceeds the tracked bound"),
            prop_arg,
            arg_known,
            absent_arg: self.absent_args(name),
            prop_result,
            result_known,
        }
    }

    fn absent_args(&mut self, name: NamePtr<'t>) -> u64 {
        let Some((_, val)) = self.env.get_declar_val(&name) else { return 0 };
        let Some(decl) = self.env.get_declar(&name) else { return 0 };
        let ty = decl.info().ty;
        let mut body = val;
        let mut arity = 0u32;
        while let crate::expr::Expr::Lambda { body: inner, .. } = self.ctx.read_expr(body) {
            if arity == max_tracked() {
                break;
            }
            body = inner;
            arity += 1;
        }
        if arity == 0 || u32::from(body.num_loose_bvars()) > max_tracked() {
            return 0;
        }
        let used = body.as_ref().fv_mask();
        let mut absent = 0u64;
        let mut rest_ty = ty;
        for i in 0..arity {
            let crate::expr::Expr::Pi { body: rest, .. } = self.ctx.read_expr(rest_ty) else { break };
            let unused_in_value = (used >> (arity - 1 - i)) & 1 == 0;
            if unused_in_value && crate::expr::ignores_binder(rest) {
                absent |= 1u64 << i;
            }
            rest_ty = rest;
        }
        absent
    }
}
