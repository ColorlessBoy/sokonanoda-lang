//! Implementation of the `Level` type representing universes
use crate::util::{LevelPtr, LevelsPtr, NamePtr, TcCtx};

pub(crate) const ZERO_HASH: u64 = 283;
pub(crate) const SUCC_HASH: u64 = 541;
pub(crate) const MAX_HASH: u64 = 1091;
pub(crate) const IMAX_HASH: u64 = 1747;
pub(crate) const PARAM_HASH: u64 = 947;
/// **R1a（2026-10-05）**：层元变量 —— **对齐 Lean `Level.lean:98`**
/// （`mkData (mixHash 2237 <| hash mvarId) 0 true false` ✓ ⇒ 用 Lean 的 **2237** ✓）。
pub(crate) const MVAR_HASH: u64 = 2237;
use Level::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level<'a> {
    Zero,
    Succ(LevelPtr<'a>, u64),
    Max(LevelPtr<'a>, LevelPtr<'a>, u64),
    IMax(LevelPtr<'a>, LevelPtr<'a>, u64),
    Param(NamePtr<'a>, u64),
    /// **层元变量**（R1a ✓）—— **对齐 Lean `Level.lean:94` `| mvar : LMVarId → Level`** ✓。
    ///
    /// ⚠ **加法**：今天**没有任何地方构造它** ✗ ⇒ **零行为变化** ✓。
    /// 构造者是 R1b（常量每宇宙位一个 fresh mvar ✓）；
    /// 消掉它的是 R1c（出口 `levelMVarToParam` ⇒ 转 `Param` ✓）。
    MVar(u64, u64),
}

/// **R2a**：三值合取 ✓ —— **对齐 Lean 的 `<&&>`**（`LBool` 的 `and` ✓：
/// `false` 支配 ✓；有 `undef` 无 `false` ⇒ `undef` ✓；全 `true` ⇒ `true` ✓）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelEq {
    True,
    False,
    Undef,
}

pub(crate) fn and3(a: LevelEq, b: LevelEq) -> LevelEq {
    match (a, b) {
        (LevelEq::False, _) | (_, LevelEq::False) => LevelEq::False,
        (LevelEq::Undef, _) | (_, LevelEq::Undef) => LevelEq::Undef,
        _ => LevelEq::True,
    }
}

impl<'a> Level<'a> {
    fn get_hash(&self) -> u64 {
        match self {
            Zero => ZERO_HASH,
            Succ(.., hash) | Max(.., hash) | IMax(.., hash) | Param(.., hash) | MVar(.., hash) => *hash,
        }
    }
}

impl<'a> std::hash::Hash for Level<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { state.write_u64(self.get_hash()) }
}

impl<'a> crate::util::RawHash for Level<'a> {
    #[inline]
    fn raw_hash(&self) -> u64 { self.get_hash() }
}

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    pub(crate) fn level_succs(&self, mut l: LevelPtr<'t>) -> (LevelPtr<'t>, usize) {
        let mut num_succs = 0usize;
        while let Succ(pred, ..) = self.read_level(l) {
            l = pred;
            num_succs += 1;
        }
        (l, num_succs)
    }

    fn combining(&mut self, l: LevelPtr<'t>, r: LevelPtr<'t>) -> LevelPtr<'t> {
        match self.read_level_pair(l, r) {
            (Zero, _) => r,
            (_, Zero) => l,
            (Succ(l, ..), Succ(r, ..)) => {
                let pred = self.combining(l, r);
                self.succ(pred)
            }
            _ => self.max(l, r),
        }
    }

    pub fn simplify(&mut self, ptr: LevelPtr<'t>) -> LevelPtr<'t> {
        match self.read_level(ptr) {
            Zero | Param(..) => return ptr,
            _ => {}
        }
        if let Some(cached) = self.expr_cache.simplify_cache.get(&ptr).copied() {
            return cached;
        }
        let result = match self.read_level(ptr) {
            Zero | Param(..) => ptr,
            // **R1a**：mvar 已是简单形状 ✓（对齐 Lean：`normalize` 不动 mvar ✓）。
            MVar(..) => ptr,
            Succ(val, ..) => {
                let val = self.simplify(val);
                self.succ(val)
            }
            Max(l, r, ..) => {
                let l = self.simplify(l);
                let r = self.simplify(r);
                self.combining(l, r)
            }
            IMax(l, r, ..) => {
                let l_simp = self.simplify(l);
                let r_simp = self.simplify(r);
                if self.is_zero(l_simp) || self.is_one(l_simp) {
                    r_simp
                } else {
                  match self.read_level(r_simp) {
                      Zero => r_simp,
                      Succ(..) => self.combining(l_simp, r_simp),
                      _ => self.imax(l_simp, r_simp)
                  }
                }
            }
        };
        self.expr_cache.simplify_cache.insert(ptr, result);
        result
    }

    /// returns `true` iff every element in `ls` is a `Param`, and `ls` has no duplicate elements.
    pub(crate) fn no_dupes_all_params(&mut self, ls: LevelsPtr<'t>) -> bool {
        let mut set = crate::util::new_fx_hash_set();
        for l in self.read_levels(ls).iter().copied() {
            match self.read_level(l) {
                Param(..) =>
                    if set.contains(&l) {
                        return false
                    } else {
                        set.insert(l);
                    },
                _ => return false,
            }
        }
        true
    }

    /// Return `uparams [ks |-> vs]` for a list of uparams
    pub fn subst_levels(&mut self, uparams: LevelsPtr<'t>, ks: LevelsPtr<'t>, vs: LevelsPtr<'t>) -> LevelsPtr<'t> {
        let out =
            self.read_levels(uparams).iter().copied().map(|l| self.subst_level(l, ks, vs)).collect::<Vec<_>>();
        self.alloc_levels(&out)
    }

    /// **R2a（2026-10-05）**：层合一的**三值结果** ✓ —— **对齐 Lean 的 `LBool`** ✓
    /// （`Meta/LevelDefEq.lean:90-125` 的 `solve` 返回 `LBool` ✓：
    /// `true` = 合一成功 ✓ · `false` = **确定不等** ✓ · `undef` = **弃权**（推迟 ✓））。
    ///
    /// ⚠ 与 U1 ③ 的 `level_assign`（`Result<(),()>` ✓，弃权 = `Err` ✓）**同构** ✓ ——
    /// 那是**文本级**（源级层名 ✓），这是**内核级**（`LevelPtr` ✓）。
    pub fn level_solve(
        &mut self,
        u: LevelPtr<'t>,
        v: LevelPtr<'t>,
        assign: &mut Vec<(u64, LevelPtr<'t>)>,
    ) -> LevelEq {
        let u = self.resolve_level_mvar(u, assign);
        let v = self.resolve_level_mvar(v, assign);
        match (self.read_level(u), self.read_level(v)) {
            // `u` 是 mvar ⇒ occurs 闸 ✓ ⇒ 赋值 ⇒ true；occurs 命中 ⇒ **undef** ✓（弃权 ✓）。
            (MVar(id, _), _) => {
                let mut seen = Vec::new();
                self.collect_level_mvars(v, &mut seen);
                if seen.contains(&id) {
                    LevelEq::Undef
                } else {
                    assign.push((id, v));
                    LevelEq::True
                }
            }
            // 右侧是 mvar ⇒ **undef** ✓（对齐 Lean：「Let `solve v u` to handle this case」✓）。
            (_, MVar(..)) => LevelEq::Undef,
            (Zero, Zero) => LevelEq::True,
            (Zero, Succ(..)) => LevelEq::False,
            (Zero, Max(a, b, _)) => {
                let x = self.level_solve(self.zero(), a, assign);
                let y = self.level_solve(self.zero(), b, assign);
                and3(x, y)
            }
            (Zero, IMax(_, b, _)) => self.level_solve(self.zero(), b, assign),
            (Succ(a, _), Succ(b, _)) => self.level_solve(a, b, assign),
            (Max(a1, b1, _), Max(a2, b2, _)) | (IMax(a1, b1, _), IMax(a2, b2, _)) => {
                let x = self.level_solve(a1, a2, assign);
                let y = self.level_solve(b1, b2, assign);
                and3(x, y)
            }
            // param：**同名**才等 ✓（对齐 Lean：param 之间按名字 ✓）。
            (Param(n1, _), Param(n2, _)) => {
                if n1 == n2 {
                    LevelEq::True
                } else {
                    LevelEq::False
                }
            }
            // 其余组合（`succ` vs `param` ✓、结构不同 ✓）⇒ **undef** ✓（弃权，不猜 ✗）。
            _ => LevelEq::Undef,
        }
    }

    /// 顺着已有赋值**解析**一个层 ✓（对齐 Lean 的 `instantiate` ✓）。
    pub fn resolve_level_mvar(
        &self,
        level: LevelPtr<'t>,
        assign: &[(u64, LevelPtr<'t>)],
    ) -> LevelPtr<'t> {
        let mut cur = level;
        let mut fuel = 64;
        while let MVar(id, _) = self.read_level(cur) {
            match assign.iter().rev().find(|(i, _)| *i == id) {
                Some((_, v)) if fuel > 0 => {
                    cur = *v;
                    fuel -= 1;
                }
                _ => break,
            }
        }
        cur
    }

    /// **R1c-2a（2026-10-05）**：收集一个层里出现的**层元变量 id**（去重、保序）✓ ——
    /// **对齐 Lean `Level.collectMVars`** ✓（出口要先知道「哪些 id 在这个声明里」✓）。
    /// ⚠ **加法**：本片**不接线** ✗ ⇒ **零行为变化** ✓。
    pub fn collect_level_mvars(&self, level: LevelPtr<'t>, out: &mut Vec<u64>) {
        match self.read_level(level) {
            Zero | Param(..) => {}
            Succ(val, ..) => self.collect_level_mvars(val, out),
            Max(l, r, ..) | IMax(l, r, ..) => {
                self.collect_level_mvars(l, out);
                self.collect_level_mvars(r, out);
            }
            MVar(id, _) => {
                if !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }

    /// **R1c-1（2026-10-05）**：把**层元变量**按映射换成 **`Param`** ✓ ——
    /// **对齐 Lean `MCtx.levelMVarToParam`**（`Meta/LevelDefEq.lean`；
    /// 出口由 `TermElabM.levelMVarToParam`（`:981`）调用 ✓）。
    ///
    /// ⚠ 与 [`Self::subst_level`] **不同**：那个只代 **param** ✓（`instantiateLevelParams` ✓），
    /// 这个只认 **mvar** ✓ —— Lean 里也是**两个函数** ✓。
    /// ⚠ **加法**：本片**不接线** ✗ ⇒ **零行为变化** ✓。
    pub fn level_mvar_to_param(&mut self, level: LevelPtr<'t>, mapping: &[(u64, NamePtr<'t>)]) -> LevelPtr<'t> {
        match self.read_level(level) {
            Zero => self.zero(),
            Succ(val, ..) => {
                let val = self.level_mvar_to_param(val, mapping);
                self.succ(val)
            }
            Max(l, r, ..) => {
                let l = self.level_mvar_to_param(l, mapping);
                let r = self.level_mvar_to_param(r, mapping);
                self.max(l, r)
            }
            IMax(l, r, ..) => {
                let l = self.level_mvar_to_param(l, mapping);
                let r = self.level_mvar_to_param(r, mapping);
                self.imax(l, r)
            }
            // param 不动 ✓（它不是 mvar ✓）。
            Param(..) => level,
            MVar(id, _) => match mapping.iter().find(|(i, _)| *i == id) {
                Some((_, name)) => {
                    let hash = crate::hash64!(PARAM_HASH, *name);
                    self.alloc_level(Level::Param(*name, hash))
                }
                // 映射里没有 ⇒ 原样 ✓（调用方负责保证**出口不留 mvar** ✓）。
                None => level,
            },
        }
    }

    /// Return `uparam [ks |-> vs]`
    pub fn subst_level(&mut self, level: LevelPtr<'t>, ks: LevelsPtr<'t>, vs: LevelsPtr<'t>) -> LevelPtr<'t> {
        match self.read_level(level) {
            Zero => self.zero(),
            Succ(val, ..) => {
                let val = self.subst_level(val, ks, vs);
                self.succ(val)
            }
            Max(l, r, ..) => {
                let l_prime = self.subst_level(l, ks, vs);
                let r_prime = self.subst_level(r, ks, vs);
                self.max(l_prime, r_prime)
            }
            IMax(l, r, ..) => {
                let l_prime = self.subst_level(l, ks, vs);
                let r_prime = self.subst_level(r, ks, vs);
                self.imax(l_prime, r_prime)
            }
            Param(..) => {
                let (ks, vs) = (self.read_levels(ks), self.read_levels(vs));
                for (k, v) in ks.iter().copied().zip(vs.iter().copied()) {
                    if level == k {
                        return v
                    }
                }
                level
            }
            // **R1a**：`subst_level` 只代 **param** ✓ ⇒ mvar **原样** ✓
            //（对齐 Lean：`instantiateLevelParams` 只认 param ✓；mvar 由
            // `levelMVarToParam` 在**出口**转掉 ✓ —— 那是 R1c 的事 ✓）。
            MVar(..) => level,
        }
    }

    /// for some level `l` and list of params `ps`, assert that:\
    /// `forall Param(n) e. l, n e. params`
    pub(crate) fn all_uparams_defined(&self, level: LevelPtr<'t>, params: LevelsPtr<'t>) -> bool {
        match self.read_level(level) {
            Zero => true,
            Succ(val, ..) => self.all_uparams_defined(val, params),
            Max(l, r, ..) | IMax(l, r, ..) =>
                self.all_uparams_defined(l, params) && self.all_uparams_defined(r, params),
            Param(..) => self.read_levels(params).iter().copied().any(|x| x == level),
            // **R1a**：mvar **不是** uparam ⇒ **false** ✓ —— 这正是「出口必须消掉 mvar」的守卫 ✓
            //（对齐 Lean：内核不接受含层元变量的声明 ✓ —— 报错而**不是** panic ✓）。
            MVar(..) => false,
        }
    }

    fn is_any_max(&self, level: LevelPtr<'t>) -> bool { matches!(self.read_level(level), Max(..) | IMax(..)) }

    fn is_param(&self, level: LevelPtr<'t>) -> bool { matches!(self.read_level(level), Param(..)) }

    fn subst_simp(&mut self, level: LevelPtr<'t>, ks: LevelsPtr<'t>, vs: LevelsPtr<'t>) -> LevelPtr<'t> {
        let l = self.subst_level(level, ks, vs);
        self.simplify(l)
    }

    /// Test whether `lhs <= rhs` by checking whether it holds regardless of whether
    /// a parameter `p` is zero or non-zero.
    fn leq_imax_by_cases(&mut self, param: LevelPtr<'t>, lhs: LevelPtr<'t>, rhs: LevelPtr<'t>, diff: isize) -> bool {
        let zero = self.zero();
        let succ_param = self.succ(param);
        let zero_slice = self.alloc_levels_slice(&[zero]);
        let succ_param_slice = self.alloc_levels_slice(&[succ_param]);
        let param_slice = self.alloc_levels_slice(&[param]);

        let lhs_0 = self.subst_simp(lhs, param_slice, zero_slice);
        let rhs_0 = self.subst_simp(rhs, param_slice, zero_slice);
        let lhs_s = self.subst_simp(lhs, param_slice, succ_param_slice);
        let rhs_s = self.subst_simp(rhs, param_slice, succ_param_slice);

        self.leq_core(lhs_0, rhs_0, diff) && self.leq_core(lhs_s, rhs_s, diff)
    }

    // The more positive it is, the more have been applied to the right side compared to the left side.
    fn leq_core(&mut self, l_in: LevelPtr<'t>, r_in: LevelPtr<'t>, diff: isize) -> bool {
        match self.read_level_pair(l_in, r_in) {
            (Zero, _) if diff >= 0 => true,
            (_, Zero) if diff < 0 => false,
            (Param(a, ..), Param(x, ..)) => a == x && diff >= 0,
            (Param(..), Zero) => false,
            (Zero, Param { .. }) => diff >= 0,
            (Succ(s, ..), _) => self.leq_core(s, r_in, diff - 1),
            (_, Succ(s, ..)) => self.leq_core(l_in, s, diff + 1),
            (Max(a, b, ..), _) => self.leq_core(a, r_in, diff) && self.leq_core(b, r_in, diff),
            (Param(..), Max(x, y, ..)) => self.leq_core(l_in, x, diff) || self.leq_core(l_in, y, diff),
            (Zero, Max(x, y, ..)) => self.leq_core(l_in, x, diff) || self.leq_core(l_in, y, diff),
            (IMax(a, b, ..), IMax(x, y, ..)) if (a == x) && (b == y) && diff >= 0 => true,
            (IMax(_, b, _), _) if self.is_param(b) => self.leq_imax_by_cases(b, l_in, r_in, diff),

            (_, IMax(_, y, _)) if self.is_param(y) => self.leq_imax_by_cases(y, l_in, r_in, diff),

            (IMax(a, b, ..), _) if self.is_any_max(b) => match self.read_level(b) {
                IMax(x, y, ..) => {
                    let new_lhs = self.imax(a, y);
                    let new_rhs = self.imax(x, y);
                    let new_max = self.max(new_lhs, new_rhs);
                    self.leq_core(new_max, r_in, diff)
                }
                Max(x, y, ..) => {
                    let new_lhs = self.imax(a, x);
                    let new_rhs = self.imax(a, y);
                    let new_max = self.max(new_lhs, new_rhs);
                    let new_max = self.simplify(new_max);
                    self.leq_core(new_max, r_in, diff)
                }
                _ => panic!(),
            },
            (_, IMax(x, y, ..)) if self.is_any_max(y) => match self.read_level(y) {
                IMax(j, k, ..) => {
                    let new_lhs = self.imax(x, k);
                    let new_rhs = self.imax(j, k);
                    let new_max = self.max(new_lhs, new_rhs);
                    self.leq_core(l_in, new_max, diff)
                }
                Max(j, k, ..) => {
                    let new_lhs = self.imax(x, j);
                    let new_rhs = self.imax(x, k);
                    let new_rhs = self.max(new_lhs, new_rhs);
                    let new_rhs = self.simplify(new_rhs);
                    self.leq_core(l_in, new_rhs, diff)
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    pub fn leq(&mut self, l: LevelPtr<'t>, r: LevelPtr<'t>) -> bool {
        if l == r {
            return true
        }
        let l_prime = self.simplify(l);
        let r_prime = self.simplify(r);
        self.leq_core(l_prime, r_prime, 0)
    }

    pub fn eq_antisymm(&mut self, l: LevelPtr<'t>, r: LevelPtr<'t>) -> bool {
        l == r || (self.leq(l, r) && self.leq(r, l))
    }

    pub fn eq_antisymm_many(&mut self, xs: LevelsPtr<'t>, ys: LevelsPtr<'t>) -> bool {
        if xs == ys {
            return true
        }
        let xs = self.read_levels(xs);
        let ys = self.read_levels(ys);
        if xs.len() != ys.len() {
            return false
        }
        xs.iter().copied().zip(ys.iter().copied()).all(|(x, y)| self.eq_antisymm(x, y))
    }

    /// Does this list of universe parameters already contain `Param(n)` for some `n : Name`
    ///
    /// Used for generating a unique elim universe in the inductive module
    pub(crate) fn contains_param(&self, uparams: LevelsPtr<'t>, candidate: NamePtr<'t>) -> bool {
        self.read_levels(uparams).iter().copied().any(|lptr| match self.read_level(lptr) {
            Param(n, ..) => n == candidate,
            _ => false,
        })
    }
    
    fn is_one(&mut self, l: LevelPtr<'t>) -> bool {
        match self.read_level(l) {
            Level::Succ(pred, _) => self.is_zero(pred),
            _ => false
        }
    }

    /// l <= 0 -> is_zero(l)
    pub fn is_zero(&mut self, level: LevelPtr<'t>) -> bool {
        let zero = self.zero();
        self.leq(level, zero)
    }

    // 1 <= level -> is_nonzero(level)
    pub fn is_nonzero(&mut self, level: LevelPtr<'t>) -> bool {
        let zero = self.zero();
        let one = self.succ(zero);
        self.leq(one, level)
    }
}
