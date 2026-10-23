//! `Value` 的小构造器（从 `value.rs` 拆出，判据 ⑨ 的行数预算）。
use crate::util::{BigUintPtr, ExprPtr, LevelPtr, StringPtr};
use std::cell::{Cell, OnceCell};
use crate::value::{mk_rigid, E, RigidHead, S, Value, V};
use bumpalo::Bump;

pub fn mk_sort<'a>(arena: &'a Bump, level: LevelPtr<'a>) -> V<'a> {
    arena.alloc(Value::Sort { level, key: Cell::new(0) })
}

pub fn mk_natlit<'a>(arena: &'a Bump, ptr: BigUintPtr<'a>) -> V<'a> {
    arena.alloc(Value::NatLit { ptr, key: Cell::new(0) })
}
pub fn mk_strlit<'a>(arena: &'a Bump, ptr: StringPtr<'a>) -> V<'a> {
    arena.alloc(Value::StrLit { ptr, key: Cell::new(0) })
}
pub fn mk_bvar_with_empty<'a>(arena: &'a Bump, level: u32, ty: V<'a>, empty: S<'a>) -> V<'a> {
    mk_rigid(arena, RigidHead::BVar(level, ty), empty)
}
pub fn mk_rigid_head_with_empty<'a>(arena: &'a Bump, head: RigidHead<'a>, empty: S<'a>) -> V<'a> {
    mk_rigid(arena, head, empty)
}
pub fn mk_thunk<'a>(arena: &'a Bump, env: E<'a>, expr: ExprPtr<'a>) -> V<'a> {
    arena.alloc(Value::Thunk { env, expr, forced: OnceCell::new(), key: Cell::new(0) })
}

