//! 项目闭包缓存的 **CLI 侧薄封装**（T-A01 起实现在 `sokonanoda_front::project::cache`）。
//!
//! 为什么留这一层：CLI 的三条命令（`check` / `build` / `query`）与 `course`
//! 都通过它调用，公开形状不变 ⇒ 搬迁那一刀只动位置、不动语义（配二进制对拍）。
//!
//! **为什么实现搬到了 `front`**：LSP 也要读这份缓存（含 `import` 的文档），
//! 而 LSP 依赖不到 CLI crate。见 `docs/design/compile-cache.md` §1。

pub(crate) use sokonanoda_front::project::cache::{load, plan, store_if_clean};
