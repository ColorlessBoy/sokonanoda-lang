//! **方言的 CLI 侧合并点**（设计 `docs/design/dialect-config.md` §2/§3.2）：
//! `--dialect`(S3) · `SOKO_STRICT_AT` · 编辑器(S5) · 文件指令(S4) · 项目清单(S4) · 默认 `loose`。
//!
//! **优先级（细粒度优先，设计 §2）**：
//! `--dialect` > `SOKO_STRICT_AT` > 编辑器(S5) > 文件指令 > 清单 > 默认 `loose`。
//!
//! ⚠ **合并只发生在 [`resolve_dialect`] 一处**：五档的**值**从这里进
//! [`DialectSources`]（front），`Option::or` 逐级落地 —— 谁要加来源（S5 的编辑器设置）
//! 只填那一格，**不许**在调用点另写 if 链 ✗。环境这一档也**不在这里读**第二次：
//! `dialect_from_env` 是唯一解析器，`CompileOptions::default` 与这里共用它 ✓
//! （S1 的「只解析一次」合同：解析出来的值进 `options.dialect`，之后全链路只读它）。
//!
//! 旗标解析（argv → [`Dialect`]）在 `main`；本模块是 CLI 侧**唯一**的合并点。

use std::path::Path;

use sokonanoda_front::compile::{
    dialect_from_env, explicit_dialect, prelude_mode_from_source, CompileOptions, Dialect,
    DialectSources, PreludeMode,
};
use sokonanoda_front::project::manifest;

/// 解析 `--dialect` 的值：只收 `strict` / `loose`，**大小写不敏感**
/// （`STRICT` / `Strict` / `loose` 都收 —— 学习者的终端里大小写不该变成陷阱）。
///
/// 与文件指令 `-- sokonanoda:dialect <值>` **同一套关键字**（[`Dialect::keyword`]）：
/// 旗标与指令的写法不会各记一套 ✓。
///
/// 别的值 ⇒ `Err(人话 + 出路)`，调用方按**用法错误退出码 2** 报出去
/// （与 `query`/未知子命令的用法错误同一档 ✓）。
pub(crate) fn parse(value: &str) -> Result<Dialect, String> {
    if value.eq_ignore_ascii_case(Dialect::Strict.keyword()) {
        return Ok(Dialect::Strict);
    }
    if value.eq_ignore_ascii_case(Dialect::Loose.keyword()) {
        return Ok(Dialect::Loose);
    }
    Err(format!(
        "error: --dialect 只接受 `strict` 或 `loose`（大小写不敏感），收到 `{value}`\n\
         提示：严格隐式参数用 `--dialect strict`，旧读法用 `--dialect loose`；\
         不传旗标则依次看 SOKO_STRICT_AT（1/on/true = strict）、文件里的 \
         `-- sokonanoda:dialect` 指令与 sokonanoda.toml 的 [dialect]，默认 loose"
    ))
}

/// 一次解析要的全部**来源**（设计 §3.2 的 `resolve_dialect(cli, env, editor, file, manifest)`）。
///
/// * `src`：入口**源文本** —— 文件指令从它读（与 prelude 读指令的位置相同）；
/// * `entry`：入口**路径**（stdin / `--text` 为 `None`）—— 清单从它向上发现；
/// * `root_override`：`--root` / `--no-project` 给的模块根（`Some` ⇒ 跳过清单，
///   与 `plan_project` 的发现规则**逐字一致** ✓）；
/// * `flag`：`--dialect`（`None` = 没给旗标 ⇒ 下一档）。
pub(crate) fn resolve_sources(
    src: &str,
    entry: Option<&Path>,
    root_override: Option<&Path>,
    flag: Option<Dialect>,
) -> Dialect {
    DialectSources {
        cli: flag,
        env: dialect_from_env(),
        // S5 的编辑器设置：位置在这里（第三档），本段恒 `None` ✓。
        editor: None,
        file_directive: explicit_dialect(src),
        manifest: entry.and_then(|path| manifest::dialect_at(path, root_override)),
    }
    .resolve()
}

/// CLI 侧**唯一**的合并点：五档来源 ⇒ [`CompileOptions`]。
pub(crate) fn resolve_dialect(
    prelude: PreludeMode,
    src: &str,
    entry: Option<&Path>,
    root_override: Option<&Path>,
    flag: Option<Dialect>,
) -> CompileOptions {
    CompileOptions::with_dialect(prelude, resolve_sources(src, entry, root_override, flag))
}

/// 便利包装（入口**一定是路径**的调用点：`build` / `course`）：
/// prelude 也从同一份源文本的指令算（与旧行为逐字相同 ✓）。
pub(crate) fn options(
    src: &str,
    entry: &Path,
    root_override: Option<&Path>,
    flag: Option<Dialect>,
) -> CompileOptions {
    resolve_dialect(
        prelude_mode_from_source(src),
        src,
        Some(entry),
        root_override,
        flag,
    )
}
