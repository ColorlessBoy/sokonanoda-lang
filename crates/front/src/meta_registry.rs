//! **S3**：judge 合成文档期间的元变量类型登记（线程局部）。
//!
//! 为什么是线程局部：`elab_expr` 的 `Ident` 臂拿不到 `OpenGoalSpec`（参数链太长），
//! 而 judge 的合成/检查是**单线程一段**（`judge_pairs_uncached` 内），退出即清空。
//! 只登记 **`?mN` 的类型文本**（源级），elaborate 时再回读。
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static TYPES: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
}

/// 在这段执行里让 `?mN` 的类型可见（退出时清空，避免跨判定泄漏）。
pub(crate) fn with_entries<T>(entries: &[(u32, String)], f: impl FnOnce() -> T) -> T {
    TYPES.with(|m| {
        let mut m = m.borrow_mut();
        m.clear();
        for (id, ty) in entries {
            m.insert(*id, ty.clone());
        }
    });
    let out = f();
    TYPES.with(|m| m.borrow_mut().clear());
    out
}

pub(crate) fn lookup(id: u32) -> Option<String> {
    TYPES.with(|m| m.borrow().get(&id).cloned())
}
