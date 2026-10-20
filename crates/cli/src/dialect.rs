//! **方言旗标 S3**（设计 `docs/design/dialect-config.md` §2/§3.2）：`--dialect <strict|loose>`。
//!
//! **优先级（本段只有两档）**：`--dialect` > `SOKO_STRICT_AT` 环境 > 默认 `loose`。
//!
//! ⚠ **环境这一档不在这里读**（设计 §3.2「只解析一次」）：未给旗标时一律走
//! [`CompileOptions::new`]，让环境解析仍留在 `front::compile::dialect` 的 `Default`
//! 里那**唯一一处**；本模块只做"旗标有没有给"的合并 ✓。这样 `--dialect strict`
//! 与 `SOKO_STRICT_AT=1` 走的是**同一个值**（后者由 `new` 解析出来），判定逐字节相同 ✓。
//!
//! 旗标解析（argv → [`Dialect`]）在 `main`；本模块是 CLI 侧**唯一**的合并点。

use sokonanoda_front::compile::{CompileOptions, Dialect, PreludeMode};

/// 解析 `--dialect` 的值：只收 `strict` / `loose`，**大小写不敏感**
/// （`STRICT` / `Strict` / `loose` 都收 —— 学习者的终端里大小写不该变成陷阱）。
///
/// 别的值 ⇒ `Err(人话 + 出路)`，调用方按**用法错误退出码 2** 报出去
/// （与 `query`/未知子命令的用法错误同一档 ✓）。
pub(crate) fn parse(value: &str) -> Result<Dialect, String> {
    match value.to_ascii_lowercase().as_str() {
        "strict" => Ok(Dialect::Strict),
        "loose" => Ok(Dialect::Loose),
        _ => Err(format!(
            "error: --dialect 只接受 `strict` 或 `loose`（大小写不敏感），收到 `{value}`\n\
             提示：严格隐式参数用 `--dialect strict`，旧读法用 `--dialect loose`；\
             不传旗标则沿用 `SOKO_STRICT_AT`（1/on/true = strict），默认 loose"
        )),
    }
}

/// 把 `--dialect`（`None` = 没给旗标）合并进编译选项 —— **CLI 侧唯一的合并点**。
///
/// * `Some(dialect)` ⇒ [`CompileOptions::with_dialect`]：**完全不读环境**
///   （所以能覆盖 `SOKO_STRICT_AT=0`，也不会让旗标绕过 S2 的缓存键 ——
///   解析后的 `Dialect` 照旧进 `CompileOptions`，键里折的就是它 ✓）；
/// * `None` ⇒ [`CompileOptions::new`]：环境解析仍只在 front 那一处，**逐字节等于旧行为** ✓
///   （这里**不许**自己 `std::env::var("SOKO_STRICT_AT")` ✗）。
pub(crate) fn resolve_dialect(prelude: PreludeMode, flag: Option<Dialect>) -> CompileOptions {
    match flag {
        Some(dialect) => CompileOptions::with_dialect(prelude, dialect),
        None => CompileOptions::new(prelude),
    }
}
