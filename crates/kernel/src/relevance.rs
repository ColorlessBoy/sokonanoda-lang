use crate::tc::TypeChecker;
use crate::util::{LevelPtr, LevelsPtr, NamePtr};
use crate::value::{Spine, Value, S};

/// **签名能记多少个参数**（G-90）：取值走 [`crate::gates::limits::max_tracked`] ✓
/// —— 默认 **128** ✓（G-90 收口：对齐 Lean 4 的 `synthInstance.maxSize = 128` ✓，
/// 见 `gates.rs::limits::max_tracked` 的说明 ✓），上界 = **位掩码宽度 128** ✓
/// （`u128` ⇒ 128 位 ✓；传再大也只到 128 ✓，不许静默截断成错 ✗），
/// 但**能拧到 1** 以证明「丢精度只变慢、不变错」✓（判据见 `docs/gaps/repro/G90-*.sh` ✓）。
#[inline]
pub(crate) fn max_tracked() -> u32 {
    crate::gates::limits::max_tracked()
}

/// **常量签名掩码**（G-90：载体 `u64` → **`u128`** ✓）。
///
/// 每个掩码的第 `i` 位 = 「第 `i` 个参数 / 第 `i` 格结果类型」✓；位宽 = `MAX_TRACKED`
/// = **128** ✓（`u128` 的 128 位 ✓ —— 旧的 `u64` 装不下 64..128 个参数 ✗，那正是 G-90 ✗）。
///
/// ⚠ **结果格的下标 = 参数个数 `n`** ⇒ `n = 128` 时那一格（第 129 格）在掩码外 ✓：
/// 这是**旧行为在 64 上的同款形状** ✓（闸值 = 「能记多少个**参数**」= 128 ✓，
/// 计数出口因此照旧咬得住：**≥128 个参数才触发** ✓，见 `crate::gates::SIG_OVERFLOW` ✓）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sig {
    pub(crate) arity: u8,
    pub(crate) prop_arg: u128,
    pub(crate) arg_known: u128,
    pub(crate) absent_arg: u128,
    pub(crate) prop_result: u128,
    pub(crate) result_known: u128,
}

impl Sig {
    pub(crate) const ALL_RELEVANT: Sig =
        Sig { arity: 0, prop_arg: 0, arg_known: 0, absent_arg: 0, prop_result: 0, result_known: 0 };

    #[inline]
    fn ignorable(&self) -> u128 { (self.prop_arg & self.arg_known) | self.absent_arg }

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
                // **闸类计数出口** ✓（G-90/G-91）：望远镜超过 128 位 ⇒ 签名**丢精度** ✗
                // （`terminal = None` ⇒ 结果那一格判不了 ✓）。丢精度只该**变慢** ✓
                // （少一条捷径 ✓），**绝不许**变错 ✗ —— 但**必须可见** ✗。
                // 闸值 = Lean 的 `synthInstance.maxSize = 128` ✓（G-90 已对齐 ✓）。
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
        let mut prop_arg = 0u128;
        let mut arg_known = 0u128;
        for i in 0..n {
            if let Some(l) = dom[i] {
                arg_known |= 1u128 << i;
                if self.ctx.is_zero(l) {
                    prop_arg |= 1u128 << i;
                }
            }
        }

        let mut prop_result = 0u128;
        let mut result_known = 0u128;
        if let Some(term) = terminal {
            if let Some(sb) = self.level_of_type(depth, term) {
                let mut r = sb;
                if n < max_tracked() as usize {
                    result_known |= 1u128 << n;
                    if self.ctx.is_zero(r) {
                        prop_result |= 1u128 << n;
                    }
                } else {
                    // 结果那一格超出掩码宽度 ⇒ 同样丢精度 ✓（计数出口，见上 ✓）。
                    // `n = max_tracked = 128` 时就是这一格（第 129 格）✓ —— 参数掩码
                    // 本身仍记满 128 位 ✓（见 `Sig` 的说明 ✓）。
                    crate::gates::SIG_OVERFLOW.bump();
                }
                for k in (0..n).rev() {
                    let Some(s) = dom[k] else { break };
                    let im = self.ctx.imax(s, r);
                    r = self.ctx.simplify(im);
                    result_known |= 1u128 << k;
                    if self.ctx.is_zero(r) {
                        prop_result |= 1u128 << k;
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

    fn absent_args(&mut self, name: NamePtr<'t>) -> u128 {
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
        // **「绑定元 `j` 在值体里用到了吗」**（G-90）：`Expr::fv_mask` 仍是 **u64**
        // （表达式表示层的宽度 ⇒ 本条不动它 ✗）⇒ 只在它**精确**的区间走位掩码快路 ✓
        // （值体的 loose bvar 全在 `0..64` 内 ⇒ 掩码是**过近似** ✓，见 `expr.rs` 的
        // `child_mask` / `body_mask` ✓）；其余（`arity > 64` 或 `j ≥ 64`）逐位
        // **精确查询** ✓。两个方向都**只许**「查不出来 ⇒ 不记 absent」✓
        // —— 记错 = 少比一个参数 = **变错** ✗（这正是 G-90 的第三条出口 ✓）。
        let used = body.as_ref().fv_mask();
        let mask_exact = u32::from(body.num_loose_bvars()) <= 64;
        let mut absent = 0u128;
        let mut rest_ty = ty;
        for i in 0..arity {
            let crate::expr::Expr::Pi { body: rest, .. } = self.ctx.read_expr(rest_ty) else { break };
            let j = arity - 1 - i; // 绑定元 `i` 在值体里的 de Bruijn 下标 ✓
            let unused_in_value =
                if mask_exact && j < 64 { (used >> j) & 1 == 0 } else { !self.ctx.has_loose_bvar(body, j as u16) };
            if unused_in_value && crate::expr::ignores_binder(rest) {
                absent |= 1u128 << i;
            }
            rest_ty = rest;
        }
        absent
    }
}
