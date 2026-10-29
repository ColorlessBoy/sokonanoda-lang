//! **K1d**：本会话层元变量的定义等价（Lean `Meta/LevelDefEq.lean` `isLevelDefEqAux`/`solve`，v4.35.0）。
//!
//! 闸：[`MetaContext::has_session_levels`](crate::mvar::MetaContext::has_session_levels) 为假 ⇒
//! 原样 `eq_antisymm`（课程语料/普通导出逐字节不变）。只有**登记过**的层 mvar 可赋值、可看穿；
//! 未登记的（前端 R1c 造的等）照旧是刚性原子。
//!
//! Lean 对照（`LevelDefEq.lean`）：
//! * `isLevelDefEqAux` 先 `instantiateLevelMVars` 再比；`u == v` ⇒ true；
//! * `solve (?u, v)`：`?u` 可赋值且 `!u.occurs v` ⇒ `?u := v`（103-115）；右侧 mvar ⇒ 对调（121）；
//! * `succ u =?= succ v` ⇒ 递归（127-135 的 `decLevel?` 这里只收 `succ` 这一种）；
//! * 其余 ⇒ 落回刚性的 `eq_antisymm`（Lean 走 `normalize` 后再比或 `postpone`；这里不推迟 ⇒ 宁拒）。
use crate::level::Level;
use crate::tc::TypeChecker;
use crate::util::{LevelPtr, LevelsPtr, TcCtx};

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    /// 只看穿**登记过**的层 mvar 的赋值（未登记的有值也不看 ⇒ 与今天同样刚性）。
    pub(crate) fn inst_session_level(&mut self, l: LevelPtr<'t>, fuel: u32) -> LevelPtr<'t> {
        match self.read_level(l) {
            Level::Zero | Level::Param(..) => l,
            Level::Succ(a, _) => {
                let a2 = self.inst_session_level(a, fuel);
                if a2 == a { l } else { self.succ(a2) }
            }
            Level::Max(a, b, _) => {
                let (a2, b2) = (self.inst_session_level(a, fuel), self.inst_session_level(b, fuel));
                if a2 == a && b2 == b { l } else { self.max(a2, b2) }
            }
            Level::IMax(a, b, _) => {
                let (a2, b2) = (self.inst_session_level(a, fuel), self.inst_session_level(b, fuel));
                if a2 == a && b2 == b { l } else { self.imax(a2, b2) }
            }
            Level::MVar(id, _) => match self.mvars.level_value(id) {
                Some(v) if fuel < 256 && self.mvars.is_session_level(id) => self.inst_session_level(v, fuel + 1),
                _ => l,
            },
        }
    }

    /// Lean `solve` + `isLevelDefEqAux` 的子集（见模块注释）。赋值走 `assign_level`（进 checkpoint）。
    fn session_level_def_eq(&mut self, u: LevelPtr<'t>, v: LevelPtr<'t>) -> bool {
        if u == v {
            return true;
        }
        let u = self.inst_session_level(u, 0);
        let v = self.inst_session_level(v, 0);
        if u == v {
            return true;
        }
        match (self.read_level(u), self.read_level(v)) {
            (Level::MVar(id, _), _) if self.mvars.level_assignable(id) => self.session_level_solve(id, u, v),
            (_, Level::MVar(id, _)) if self.mvars.level_assignable(id) => self.session_level_solve(id, v, u),
            (Level::Succ(a, _), Level::Succ(b, _)) => self.session_level_def_eq(a, b),
            _ => self.eq_antisymm(u, v),
        }
    }

    /// `?id =?= v`（`mv` = `?id` 本身）：occurs ⇒ 落回刚性比较（`?u =?= succ ?u` 在那里为假）；否则赋值。
    fn session_level_solve(&mut self, id: u64, mv: LevelPtr<'t>, v: LevelPtr<'t>) -> bool {
        let mut seen = Vec::new();
        self.collect_level_mvars(v, &mut seen);
        if seen.contains(&id) {
            return self.eq_antisymm(mv, v);
        }
        self.mvars.assign_level(id, v);
        true
    }
}

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    /// 造一个层元变量（与 `EnvBuilder::level_mvar` 同哈希）。不登记 ⇒ 刚性；要可赋值另调 `register_level`。
    pub fn mk_level_mvar(&mut self, id: u64) -> LevelPtr<'t> {
        let hash = crate::hash64!(crate::level::MVAR_HASH, id);
        self.alloc_level(Level::MVar(id, hash))
    }
}

impl<'x, 't, 'p: 't> TypeChecker<'x, 't, 'p> {
    /// conv 的层比较入口（`Sort` 层、常量层表）。无登记层 mvar ⇒ 原 `eq_antisymm`。
    pub(crate) fn level_eq(&mut self, u: LevelPtr<'t>, v: LevelPtr<'t>) -> bool {
        if !self.ctx.mvars.has_session_levels() {
            return self.ctx.eq_antisymm(u, v);
        }
        let g = self.ctx.mvars.generation();
        let ok = self.ctx.session_level_def_eq(u, v);
        if self.ctx.mvars.generation() != g {
            // 赋值即失效（与项元变量的 `unify_meta` 同一纪律）。
            self.invalidate_on_assign();
        }
        ok
    }

    /// 常量层表版（长度不同 ⇒ false，同 `eq_antisymm_many`）。
    pub(crate) fn level_eq_many(&mut self, xs: LevelsPtr<'t>, ys: LevelsPtr<'t>) -> bool {
        if !self.ctx.mvars.has_session_levels() {
            return self.ctx.eq_antisymm_many(xs, ys);
        }
        if xs == ys {
            return true;
        }
        let (xs, ys) = (self.ctx.read_levels(xs), self.ctx.read_levels(ys));
        if xs.len() != ys.len() {
            return false;
        }
        // 原子：前几个位赋了值、后面失败 ⇒ 撤销（否则 `heads_match = false` 后走展开分支还留着半截赋值）。
        let cp = self.ctx.mvars.checkpoint();
        let ok = xs.iter().copied().zip(ys.iter().copied()).all(|(x, y)| self.level_eq(x, y));
        if !ok && self.ctx.mvars.restore(cp) {
            self.tc_cache.clear();
        }
        ok
    }
}
