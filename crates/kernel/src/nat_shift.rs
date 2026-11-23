//! `Nat.shiftLeft` / `Nat.shiftRight` 的**超宽位移**归约。
//!
//! LKA `bugs/nat-shift-huge`：位移量是 `2^64`，`u64` 装不下，`BigUint` 侧没有
//! 可用的宽度。两条出路：被移位的实参能 delta 展开成字面量时照常算
//! （`n >> 2^64 = 0`、`0 << 2^64 = 0`）；展开不出来就让项保持 stuck —— 绝不
//! 展开常量自己的体，那会以同一个巨大的主前提重新进入递归定义
//! （实测 `whnf_head` / `iota_value` 无限互相递归）。

use crate::name::NatRed;
use crate::tc::{NatBinOp, TypeChecker};
use crate::util::{nat_div, nat_gcd, nat_land, nat_lor, nat_mod, nat_sub, nat_xor};
use num_bigint::BigUint;
use num_traits::{One, Pow, Zero};
use crate::value::V;

pub(crate) fn shift_amount(y: &BigUint) -> Option<u64> { u64::try_from(y).ok() }

pub(crate) fn nat_shl(x: BigUint, y: BigUint) -> BigUint {
    match shift_amount(&y) {
        Some(sh) => x << sh,
        // `0 << n = 0` for every `n` (the arena corpus pins `0 << 2^64`); a
        // non-zero base with an unrepresentable shift is reported, not left as a
        // stuck term that would unfold the recursive body 2^64 times.
        None => {
            assert!(x.is_zero(), "Nat.shiftLeft: shift does not fit in a machine word");
            BigUint::zero()
        }
    }
}

pub(crate) fn nat_shr(x: BigUint, y: BigUint) -> BigUint {
    match shift_amount(&y) {
        Some(sh) => x >> sh,
        None => BigUint::zero(),
    }
}


impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    /// 位移量不可表示时：把被移位的实参 delta 展开一层再试一次。
    pub(crate) fn nat_red_wide_shift(&mut self, depth: u32, kind: NatRed, args: &[V<'t>]) -> Option<V<'t>> {
        if !matches!(kind, NatRed::Shl | NatRed::Shr) || args.len() != 2 {
            return None;
        }
        let yn = self.value_to_bignum_at(depth, args[1], true)?;
        if shift_amount(&yn).is_some() {
            return None;
        }
        let base = self.unfold_value_go(depth, args[0], true);
        let xn = self.value_to_bignum_at(depth, base, true)?;
        let op = if matches!(kind, NatRed::Shl) { NatBinOp::Shl } else { NatBinOp::Shr };
        self.do_nat_bin_val(xn, yn, op)
    }

    /// 这个位移量是不是**不可表示**（装不进机器字）⇒ 调用方保持 stuck。
    pub(crate) fn nat_red_wide_shift_stuck(&mut self, depth: u32, kind: Option<NatRed>, args: &[V<'t>]) -> bool {
        if !matches!(kind, Some(NatRed::Shl | NatRed::Shr)) || args.len() != 2 {
            return false;
        }
        match self.value_to_bignum_at(depth, args[1], true) {
            Some(y) => shift_amount(&y).is_none(),
            None => false,
        }
    }
    pub(crate) fn do_nat_bin_val(&mut self, x: BigUint, y: BigUint, op: NatBinOp) -> Option<V<'t>> {
        use NatBinOp::*;
        match op {
            Add => self.mk_natlit_val(x + y),
            Sub => self.mk_natlit_val(nat_sub(x, y)),
            Mul => self.mk_natlit_val(x * y),
            Pow => self.mk_natlit_val(x.pow(y)),
            Div => self.mk_natlit_val(nat_div(x, y)),
            Mod => self.mk_natlit_val(nat_mod(x, y)),
            Gcd => self.mk_natlit_val(nat_gcd(&x, &y)),
            LAnd => self.mk_natlit_val(nat_land(x, y)),
            LOr => self.mk_natlit_val(nat_lor(x, y)),
            XOr => self.mk_natlit_val(nat_xor(&x, &y)),
            Shl => self.mk_natlit_val(nat_shl(x, y)),
            Shr => self.mk_natlit_val(nat_shr(x, y)),
            Beq => self.bool_val(x == y),
            Ble => self.bool_val(x <= y),
        }
    }

}
