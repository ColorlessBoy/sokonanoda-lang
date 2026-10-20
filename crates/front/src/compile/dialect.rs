//! **方言（严格隐式参数读法）**：一个值、一处解析、[`CompileOptions`] 的构造器。
//!
//! 设计 `docs/design/dialect-config.md` §3（**S1**，2026-10-20）：**一个值、只解析一次** ——
//! 来源是环境变量 `SOKO_STRICT_AT=1|on|true`（S1）、CLI 旗标（S3）、文件指令与项目清单
//! （S4），之后全链路只读 `options.dialect`，**热路径不再读环境**
//! （每次调用读一次曾是设计 §1 第 1 条的坑 ✓）。
//!
//! **五处来源、一条优先级链**（设计 §2，细粒度优先 ——
//! [`DialectSources::resolve`] 是**唯一**的合并点）：
//! `--dialect`(1) > `SOKO_STRICT_AT`(2) > 编辑器(3，S5) > 文件指令(4) > 清单(5) > 默认 `loose`。
//!
//! **为什么本文件单独存在**：[`CompileOptions`] 的 `Default`（20+ 行文档 + 环境解析）
//! 原先长在 `prelude.rs` 里，而那是判据 ⑨ 的**冻结清单**（`scripts/docs-lint.py`，
//! **只许减不许增** ✗）⇒ 搬到这里，`prelude.rs` 只留结构体 + `use` ✓。
//! 同族先例：`error_hint.rs`（从 `error.rs` 拆出来腾预算，2026-10-16）。
//! S4 的**文件指令解析器**也落在这里（`prelude.rs:41` 的 `explicit_prelude_mode`
//! 是形状先例，但那边**零余量**，一行都不加）。

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
/// **一个值、只解析一次**（S1）：来源在 [`DialectSources::resolve`] 里按 §2 逐级覆盖
/// **一次** ⇒ 之后全链路只读 `options.dialect` ✓，热路径**不再**读环境。
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

    /// **源级关键字**（`strict` / `loose`）：CLI 旗标、文件指令、诊断文案共用这一个
    /// 拼写来源（两档的写法不会各处漂移 ✓）。
    ///
    /// 用穷尽 `match`（不是 `if strict`）：将来加第三态时编译器会逼着这里表态 ✗
    /// —— 同 `cache/keys.rs::dialect_pair` 的写法。
    pub fn keyword(self) -> &'static str {
        match self {
            Dialect::Strict => "strict",
            Dialect::Loose => "loose",
        }
    }

    /// **源级关键字 → 方言**（大小写不敏感）：CLI 旗标（S3）、文件指令（S4）与
    /// 编辑器的 `initializationOptions`（S5）**共用这一个解析口径**，三处不会各记一套。
    ///
    /// `None` = 不认识 ⇒ 调用方按"**不表态**"处理（落到下一档；绝不替用户猜）。
    pub fn parse_keyword(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case(Self::Strict.keyword()) {
            return Some(Self::Strict);
        }
        if value.eq_ignore_ascii_case(Self::Loose.keyword()) {
            return Some(Self::Loose);
        }
        None
    }
}

/// 文件指令的**键**（S4，设计 §2 第 4 档）：`-- sokonanoda:dialect <strict|loose>`。
pub const DIALECT_DIRECTIVE: &str = "sokonanoda:dialect";

/// 读**文件指令** `-- sokonanoda:dialect strict|loose`（S4，设计 `dialect-config.md` §2 第 4 档）。
///
/// **形状照抄** `prelude.rs:54` 的 `explicit_prelude_mode`：逐行找 `--` 前缀、
/// 取 `sokonanoda:` 后面的值、**第一次出现即定**（后面的重复不覆盖 —— 与 prelude 同形，
/// 于是"文件里写两条指令"在两处行为一致）。
///
/// 返回值是**表态**：
/// * `strict` / `loose`（**大小写不敏感**，与 CLI 旗标 `--dialect` 同口径）⇒ `Some(..)`；
/// * 没写 / 值不认识 ⇒ `None`（**不表态**，绝不替用户选宽松 —— 写错一个字就静默
///   变回默认值是最坏的配置面 ✗；残留风险见 S4 汇报的「遗留风险」）。
///
/// 为什么"不认识就不表态"而 prelude 是"不认识就当 Full"：prelude 的 `Full` 恰好是
/// 默认值，两者在**没写指令**时不可区分；方言这边 `loose` 是默认值，但用户手写
/// `-- sokonanoda:dialect strcit` 时把**清单里的 strict** 压成 loose 更坏 ✗。
pub fn explicit_dialect(src: &str) -> Option<Dialect> {
    for line in src.lines() {
        let trimmed = line.trim_start();
        let Some(comment) = trimmed.strip_prefix("--") else {
            continue;
        };
        let comment = comment.trim();
        let Some(rest) = comment.strip_prefix(DIALECT_DIRECTIVE) else {
            continue;
        };
        // 与 CLI 旗标 / 编辑器设置**同一个关键字表**（[`Dialect::parse_keyword`]）。
        return Dialect::parse_keyword(rest.trim());
    }
    None
}

/// **环境变量这一档的表态**（`SOKO_STRICT_AT`，设计 §2 第 2 档）。
///
/// * `1|on|true` ⇒ `Some(Strict)`；`0|off|false` ⇒ `Some(Loose)`（**显式关**也是表态：
///   CI 与逃生门靠它盖住文件指令 ✓）；
/// * 未设 / 取值不认识 ⇒ `None`（不表态，交给下一档）。
///
/// ⚠ 与 S1 的 `Default` **极性逐字相同**：`Default` 走
/// `dialect_from_env().unwrap_or_default()` ⇒ "只有 1/on/true 才是严格档、
/// 其余一律宽松"这条旧性质**一个字节都没变** ✓。
pub fn dialect_from_env() -> Option<Dialect> {
    match std::env::var("SOKO_STRICT_AT").ok().as_deref() {
        Some("1") | Some("on") | Some("true") => Some(Dialect::Strict),
        Some("0") | Some("off") | Some("false") => Some(Dialect::Loose),
        _ => None,
    }
}

/// **五处来源**（设计 `docs/design/dialect-config.md` §2/§3.2）：每一格是**一档的表态**，
/// `None` = 这一档没有意见。**合并只发生在 [`Self::resolve`] 一处** —— 谁要加来源
/// （S5 的编辑器设置）就往这里填一格，**不许**在调用方另写 if 链 ✗。
///
/// 字段顺序 = 优先级顺序（高的在前），`resolve` 用 `Option::or` 逐级落地 ⇒
/// "细粒度优先"是**结构上**成立的，不是靠人记 ✓。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DialectSources {
    /// 1（最高）：CLI `--dialect strict|loose`（S3）。
    pub cli: Option<Dialect>,
    /// 2：`SOKO_STRICT_AT`（CI、脚本、逃生门）。
    pub env: Option<Dialect>,
    /// 3：**编辑器设置**（S5：`initializationOptions` / `didChangeConfiguration`）。
    /// S4 **恒 `None`** —— 位置留在这里，接线是 S5 的事 ✓。
    pub editor: Option<Dialect>,
    /// 4：文件指令 `-- sokonanoda:dialect …`（S4，[`explicit_dialect`]）。
    pub file_directive: Option<Dialect>,
    /// 5：项目清单 `[dialect] strict_implicit = true|false`（S4，
    /// [`crate::project::manifest::Manifest::dialect`]）。
    pub manifest: Option<Dialect>,
}

impl DialectSources {
    /// 逐级覆盖：`cli > env > editor > file_directive > manifest > 默认 loose`（设计 §2）。
    ///
    /// **默认仍是 `loose`**（与 0.95.0 逐字节一致 ✓）：五档全 `None` ⇒
    /// [`Dialect::default`] ⇒ [`Dialect::Loose`]。
    pub fn resolve(self) -> Dialect {
        self.cli
            .or(self.env)
            .or(self.editor)
            .or(self.file_directive)
            .or(self.manifest)
            .unwrap_or_default()
    }
}

impl CompileOptions {
    /// **默认档**的选项：`prelude` 照给，`dialect` 读一次环境（= [`Self::default`]）。
    ///
    /// **28 处字面量都走这条**（`CompileOptions { prelude: X }` 的多行写法会撑爆
    /// 判据 ⑨ 的冻结行数 ⇒ 一行调用 ✓）。相对旧写法**判定行为逐字节不变**：
    /// 档位仍然只由 `SOKO_STRICT_AT` 决定 ✓ —— S4 的来源**不经过这里**
    /// （CLI 侧一律走 `CompileOptions::with_dialect`，见 `crates/cli/src/dialect.rs`）。
    pub fn new(prelude: PreludeMode) -> Self {
        Self {
            prelude,
            ..Self::default()
        }
    }

    /// **显式档位**：不读环境（S3 的 CLI 旗标 / S4 的文件指令与清单 / S5 的编辑器设置
    /// 都走这条 ✓）。
    pub fn with_dialect(prelude: PreludeMode, dialect: Dialect) -> Self {
        Self { prelude, dialect }
    }
}

impl Default for CompileOptions {
    /// ⭐ **S1：唯一读严格档环境变量的地方**（设计 §3.2「只解析一次」）。
    ///
    /// 极性与其前身（`implicit_msg::strict_at_enabled()` 的旧实现）**逐字相同**：
    /// 只有 `1|on|true` ⇒ [`Dialect::Strict`]；未设 / 其它取值 ⇒ [`Dialect::Loose`] ✓
    /// （细节见 [`dialect_from_env`]）。编译热路径（`elab`/`by`/`goals`/`suggest`）
    /// **一律不读环境** ✗——每次调用读一次曾是设计 §1 的坑；值进 `options.dialect`
    /// 之后只在这一处解析 ✓（S2 进缓存键）。
    fn default() -> Self {
        Self {
            prelude: PreludeMode::default(),
            dialect: dialect_from_env().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文件指令：注释行、`strict`/`loose`、大小写不敏感 —— 与 `--dialect` 旗标同口径。
    #[test]
    fn a_file_directive_is_read_from_the_leading_comments() {
        assert_eq!(
            explicit_dialect("-- sokonanoda:dialect strict\ndef x : Nat := 1\n"),
            Some(Dialect::Strict)
        );
        assert_eq!(
            explicit_dialect("-- 课程说明\n-- sokonanoda:dialect loose\n"),
            Some(Dialect::Loose)
        );
        // 大小写不敏感（与 CLI 旗标同口径：`STRICT` / `Loose` 都收）。
        assert_eq!(
            explicit_dialect("-- sokonanoda:dialect STRICT\n"),
            Some(Dialect::Strict)
        );
        // 指令可以写在文件中间（与 prelude 的扫描形状一致：逐行找 `--` 前缀）。
        assert_eq!(
            explicit_dialect("def x : Nat := 1\n  -- sokonanoda:dialect strict\n"),
            Some(Dialect::Strict)
        );
        // 代码行里的字样不算指令（必须是 `--` 注释）。
        assert_eq!(
            explicit_dialect("def s : Prop := True -- sokonanoda:dialect strict\n"),
            None
        );
        assert_eq!(explicit_dialect("def x : Nat := 1\n"), None);
    }

    /// 第一次出现即定（与 `explicit_prelude_mode` 同形）；值不认识 ⇒ **不表态**
    /// （绝不把用户写错的指令静默折成 loose ⇒ 那会压掉清单里的 strict ✗）。
    #[test]
    fn only_the_first_directive_counts_and_unknown_values_do_not_speak() {
        assert_eq!(
            explicit_dialect("-- sokonanoda:dialect loose\n-- sokonanoda:dialect strict\n"),
            Some(Dialect::Loose)
        );
        assert_eq!(explicit_dialect("-- sokonanoda:dialect strcit\n"), None);
        assert_eq!(explicit_dialect("-- sokonanoda:dialect\n"), None);
    }

    /// **五档优先级**（设计 §2）：每一档都真的能盖住它下面的所有档；编辑器那一档
    /// 虽然 S4 恒 `None`，位置**已经钉在 env 与文件指令之间**（S5 接上时不用改这里）。
    #[test]
    fn the_five_sources_follow_the_documented_priority() {
        let all = |dialect: Dialect| DialectSources {
            cli: Some(dialect),
            env: Some(dialect),
            editor: Some(dialect),
            file_directive: Some(dialect),
            manifest: Some(dialect),
        };
        // 每一档都能盖住下面全部：把该档及其下面设成 Strict、上面设成 Loose。
        let strict = Dialect::Strict;
        let loose = Dialect::Loose;
        let ladder = [
            // cli 最高。
            (
                DialectSources {
                    cli: Some(loose),
                    ..all(strict)
                },
                loose,
            ),
            // env 盖住 editor/file/manifest。
            (
                DialectSources {
                    cli: None,
                    env: Some(loose),
                    ..all(strict)
                },
                loose,
            ),
            // editor 盖住 file/manifest（S5 的位置）。
            (
                DialectSources {
                    cli: None,
                    env: None,
                    editor: Some(loose),
                    ..all(strict)
                },
                loose,
            ),
            // 文件指令盖住清单。
            (
                DialectSources {
                    cli: None,
                    env: None,
                    editor: None,
                    file_directive: Some(loose),
                    manifest: Some(strict),
                },
                loose,
            ),
            // 清单最低，但**比默认高**。
            (
                DialectSources {
                    manifest: Some(strict),
                    ..DialectSources::default()
                },
                strict,
            ),
            // 谁都不表态 ⇒ 默认 loose（0.95.0 的行为，设计 §2 最后一行）。
            (DialectSources::default(), loose),
        ];
        for (sources, expected) in ladder {
            assert_eq!(sources.resolve(), expected, "sources = {sources:?}");
        }
    }
}
