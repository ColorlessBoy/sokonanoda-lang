//! **方言（严格隐式参数读法）**：一个值、一处解析、[`CompileOptions`] 的构造器。
//!
//! 设计 `docs/design/dialect-config.md` §3（**S1**，2026-10-20）：**一个值、只解析一次** ——
//! 今天唯一来源是环境变量 `SOKO_STRICT_AT=1|on|true`（S2 进缓存键、S3+ 加 CLI 旗标 /
//! 清单 / 编辑器设置），之后全链路只读 `options.dialect`，**热路径不再读环境**
//! （每次调用读一次曾是设计 §1 第 1 条的坑 ✓）。
//!
//! **为什么本文件单独存在**：[`CompileOptions`] 的 `Default`（20+ 行文档 + 环境解析）
//! 原先长在 `prelude.rs` 里，而那是判据 ⑨ 的**冻结清单**（`scripts/docs-lint.py`，
//! **只许减不许增** ✗）⇒ 搬到这里，`prelude.rs` 只留结构体 + `use` ✓。
//! 同族先例：`error_hint.rs`（从 `error.rs` 拆出来腾预算，2026-10-16）。

use super::prelude::{CompileOptions, PreludeMode};

/// **方言**（S1，设计 `docs/design/dialect-config.md` §3.1）：严格隐式参数读法的**两态**开关。
///
/// 严格档 = **正常形式（没写 `@`）不接受位置隐式实参** ✓，与 Lean 4
/// `Lean/Elab/App.lean:706` `processImplicitArg` 的
/// `if (← read).explicit then processExplicitArg argName else addImplicitArg argName`
/// **同构**（`:709` = `@` 让隐式 binder 也吃一个位置实参 ✓；`:711` = 正常形式给隐式
/// binder 插元变量、**不碰实参表** ✓）。
///
/// ⚠ **两态 enum 而不是 `bool`**（设计 §3.1）：将来要加第三态「严格但允许省略前导隐式」
/// 时不用再改签名 ✓。
///
/// **一个值、只解析一次**（S1）：来源（今天只有环境变量）在
/// `impl Default for CompileOptions` 里解析**一次** ⇒ 之后全链路只读
/// `options.dialect` ✓，热路径**不再**读环境（每次调用读一次是设计 §1 第 1 条的坑）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    /// 严格档：正常形式不收位置隐式实参（= Lean 4 的读法 ✓）。
    Strict,
    /// 旧读法（**默认** ✓，与 0.95.0 一致；翻默认是 `strict-mode-v2` 第 4 棒的事）。
    #[default]
    Loose,
}

impl Dialect {
    /// 是否严格档（**值层读取器**；调用点仍走 [`super::implicit_msg::strict_at_enabled`] ✓）。
    pub fn strict(self) -> bool {
        matches!(self, Dialect::Strict)
    }
}

impl CompileOptions {
    /// **默认档**的选项：`prelude` 照给，`dialect` 读一次环境（= [`Self::default`]）。
    ///
    /// **28 处字面量都走这条**（`CompileOptions { prelude: X }` 的多行写法会撑爆
    /// 判据 ⑨ 的冻结行数 ⇒ 一行调用 ✓）。相对旧写法**判定行为逐字节不变**：
    /// 档位仍然只由 `SOKO_STRICT_AT` 决定 ✓。
    pub fn new(prelude: PreludeMode) -> Self {
        Self {
            prelude,
            ..Self::default()
        }
    }

    /// **显式档位**：不读环境（S3+ 的 CLI 旗标 / 清单 / 编辑器设置走这条 ✓）。
    pub fn with_dialect(prelude: PreludeMode, dialect: Dialect) -> Self {
        Self { prelude, dialect }
    }
}

impl Default for CompileOptions {
    /// ⭐ **S1：唯一读严格档环境变量的地方**（设计 §3.2「只解析一次」）。
    ///
    /// 极性与其前身（`implicit_msg::strict_at_enabled()` 的旧实现）**逐字相同**：
    /// 只有 `1|on|true` ⇒ [`Dialect::Strict`]；未设 / 其它取值 ⇒ [`Dialect::Loose`] ✓。
    /// 编译热路径（`elab`/`by`/`goals`/`suggest`）**一律不读环境** ✗——每次调用读一次
    /// 曾是设计 §1 的坑；值进 `options.dialect` 之后只在这一处解析 ✓（S2 再加进缓存键）。
    fn default() -> Self {
        let dialect = match std::env::var("SOKO_STRICT_AT").ok().as_deref() {
            Some("1") | Some("on") | Some("true") => Dialect::Strict,
            _ => Dialect::Loose,
        };
        Self {
            prelude: PreludeMode::default(),
            dialect,
        }
    }
}
