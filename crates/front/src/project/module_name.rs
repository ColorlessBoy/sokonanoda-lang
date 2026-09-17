//! 模块名与「模块名 ↔ 文件路径」规则（`import Foo.Bar` 的语义基础）。
//!
//! 规则与官方 Lean 同款：import 名 = **模块根**之下的相对路径、去掉
//! `.sokonanoda` 扩展名、路径分隔符换成 `.`；反过来 `Foo.Bar` 对应
//! `<root>/Foo/Bar.sokonanoda`。由此推出教学上最关键的两条结论：
//!
//! * 每个分量必须是**合法标识符** ⇒ 文件名里的 `-` 不是模块名字符
//!   （`unit1-propositions-proofs.sokonanoda` 不能被 `import`）；
//! * 分量不能为空、不能以数字开头（`Foo.`、`Foo..Bar`、`1Foo` 都非法）。
//!
//! 词法层的标识符集合由 `crate::token` 定义，这里**复用同一份谓词**（只额外
//! 禁用 `.`——它在模块名里是分隔符），避免"两套标识符规则"漂移。

use std::path::PathBuf;

use crate::token::{is_ident_continue, is_ident_start};

/// 模块源文件扩展名（不含点）。
pub const MODULE_EXTENSION: &str = "sokonanoda";

/// `-` 的教学提示。词法层（`crate::token` 在 `import` 行遇到 `-` 时）与
/// 模块名校验共用同一句话，避免"两处说法不一致"。
pub const DASH_HINT: &str =
    "`-` 不是模块名字符（官方 Lean 同样如此）：文件名里的横线不能出现在 import 名里。要么改文件名，要么建一个合法名字的模块文件。";

/// 一个已校验的模块名：至少一个分量，逐分量保证非空且是合法标识符。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleName {
    components: Vec<String>,
}

/// 模块名不合法的原因（都带中文教学文案，见 [`ModuleNameError::message`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleNameError {
    /// `import` 后面没有名字。
    Empty,
    /// 空分量：`Foo.`、`Foo..Bar`、`.Foo`。
    EmptyComponent,
    /// 分量以非标识符起始字符开头（典型：数字）。
    BadStart { component: String, ch: char },
    /// 分量含非标识符字符（典型：`-`）。
    BadChar { component: String, ch: char },
}

impl ModuleNameError {
    /// 面向用户的错误正文（进诊断 `message`）。
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "`import` 后面要写模块名".to_string(),
            Self::EmptyComponent => {
                "模块名里有空的分量（`.` 不能开头、结尾或连续出现）".to_string()
            }
            Self::BadStart { component, ch } => {
                format!("模块名分量 `{component}` 不能以 `{ch}` 开头")
            }
            Self::BadChar { component, ch } => {
                format!("模块名分量 `{component}` 里有非法字符 `{ch}`")
            }
        }
    }

    /// 教学提示（进诊断 `hint`）。
    pub fn hint(&self) -> &'static str {
        match self {
            Self::Empty | Self::EmptyComponent => {
                "模块名是**点分**的标识符，例如 `import Lesson.Logic` 对应 `Lesson/Logic.sokonanoda`。"
            }
            Self::BadStart { .. } => {
                "模块名的每一段都必须像标识符一样开头（字母或 `_`），不能以数字开头。"
            }
            Self::BadChar { ch, .. } if *ch == '-' => DASH_HINT,
            Self::BadChar { .. } => {
                "模块名的每一段只能是字母、数字、`_`、`'`、`!`、`?`，用 `.` 分隔——与官方 Lean 的标识符规则一致。"
            }
        }
    }
}

impl ModuleName {
    /// 解析并校验一个点分模块名。
    pub fn parse(raw: &str) -> Result<Self, ModuleNameError> {
        if raw.is_empty() {
            return Err(ModuleNameError::Empty);
        }
        let mut components = Vec::new();
        for part in raw.split('.') {
            if part.is_empty() {
                return Err(ModuleNameError::EmptyComponent);
            }
            let mut chars = part.chars();
            let first = chars.next().expect("non-empty component");
            if !is_ident_start(first) {
                return Err(ModuleNameError::BadStart {
                    component: part.to_string(),
                    ch: first,
                });
            }
            for ch in chars {
                if !is_component_continue(ch) {
                    return Err(ModuleNameError::BadChar {
                        component: part.to_string(),
                        ch,
                    });
                }
            }
            components.push(part.to_string());
        }
        Ok(Self { components })
    }

    /// 规范化文本（分量用 `.` 连接；与原始写法逐字相同）。
    pub fn as_str(&self) -> String {
        self.components.join(".")
    }

    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// 最后一个分量（用于 `import Foo.Bar` ↔ `Foo/Bar.sokonanoda` 的组织）。
    pub fn last_component(&self) -> &str {
        self.components
            .last()
            .expect("ModuleName always has a component")
    }

    /// 相对模块根的路径：`Foo.Bar` → `Foo/Bar.sokonanoda`。
    pub fn relative_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        for component in &self.components {
            path.push(component);
        }
        path.set_extension(MODULE_EXTENSION);
        path
    }

    /// 可移植性说明（官方 Lean 的 portability lint 对应物）：某些名字在部分
    /// 操作系统上不可用。返回 `Some(说明)` 供上层发 warning，**不是错误**。
    pub fn portability_note(&self) -> Option<String> {
        for component in &self.components {
            let upper = component.to_ascii_uppercase();
            let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || is_reserved_numbered(&upper, "COM")
                || is_reserved_numbered(&upper, "LPT");
            if reserved {
                return Some(format!(
                    "模块名分量 `{component}` 在部分操作系统上是保留设备名，换一个名字更安全。"
                ));
            }
            if let Some(ch) = component
                .chars()
                .find(|c| matches!(c, '<' | '>' | '"' | '|' | '?' | '*' | '!'))
            {
                return Some(format!(
                    "模块名分量 `{component}` 里的 `{ch}` 在部分操作系统上不能出现在文件名里。"
                ));
            }
        }
        None
    }
}

/// 分量续字符 = 词法层标识符续字符去掉 `.`（`.` 在模块名里是分隔符）。
fn is_component_continue(c: char) -> bool {
    c != '.' && is_ident_continue(c)
}

fn is_reserved_numbered(upper: &str, prefix: &str) -> bool {
    upper
        .strip_prefix(prefix)
        .is_some_and(|rest| rest.len() == 1 && matches!(rest.as_bytes()[0], b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_round_trips_a_dotted_name() {
        let name = ModuleName::parse("Lesson.Logic").expect("valid");
        assert_eq!(name.as_str(), "Lesson.Logic");
        assert_eq!(name.components(), ["Lesson", "Logic"]);
        assert_eq!(
            name.relative_path(),
            PathBuf::from("Lesson/Logic.sokonanoda")
        );
        assert_eq!(name.last_component(), "Logic");
    }

    #[test]
    fn single_component_maps_to_a_sibling_file() {
        let name = ModuleName::parse("Bar").expect("valid");
        assert_eq!(name.relative_path(), PathBuf::from("Bar.sokonanoda"));
    }

    #[test]
    fn rejects_empty_names_and_empty_components() {
        assert_eq!(ModuleName::parse(""), Err(ModuleNameError::Empty));
        assert_eq!(
            ModuleName::parse("Foo."),
            Err(ModuleNameError::EmptyComponent)
        );
        assert_eq!(
            ModuleName::parse("Foo..Bar"),
            Err(ModuleNameError::EmptyComponent)
        );
        assert_eq!(
            ModuleName::parse(".Foo"),
            Err(ModuleNameError::EmptyComponent)
        );
    }

    #[test]
    fn rejects_digits_at_the_start_of_a_component() {
        assert_eq!(
            ModuleName::parse("Unit1.Proofs"),
            Ok(ModuleName {
                components: vec!["Unit1".into(), "Proofs".into()]
            }),
            "digits are fine after the first character"
        );
        assert_eq!(
            ModuleName::parse("1Foo"),
            Err(ModuleNameError::BadStart {
                component: "1Foo".into(),
                ch: '1'
            })
        );
    }

    #[test]
    fn rejects_dash_with_a_teaching_hint() {
        let err = ModuleName::parse("unit1-propositions").expect_err("dash is not a module char");
        assert_eq!(
            err,
            ModuleNameError::BadChar {
                component: "unit1-propositions".into(),
                ch: '-'
            }
        );
        assert!(
            err.hint().contains("`-` 不是模块名字符"),
            "the dash case gets its own hint: {}",
            err.hint()
        );
    }

    #[test]
    fn accepts_the_same_identifier_charset_as_the_lexer() {
        // 词法层允许 `'!?` 续接；模块名沿用同一集合（但 `.` 是分隔符）。
        for raw in [
            "Foo'",
            "Foo!",
            "Foo?",
            "Foo_bar",
            "Foo2",
            "_Foo",
            "逻辑.基础",
        ] {
            assert!(
                ModuleName::parse(raw).is_ok(),
                "{raw} should be a legal module name"
            );
        }
    }

    #[test]
    fn portability_note_flags_reserved_and_forbidden_names() {
        assert!(ModuleName::parse("Logic")
            .expect("valid")
            .portability_note()
            .is_none());
        assert!(ModuleName::parse("Con")
            .expect("valid")
            .portability_note()
            .is_some_and(|note| note.contains("保留设备名")));
        assert!(ModuleName::parse("Com1")
            .expect("valid")
            .portability_note()
            .is_some_and(|note| note.contains("保留设备名")));
        assert!(ModuleName::parse("Foo!")
            .expect("valid")
            .portability_note()
            .is_some_and(|note| note.contains("`!`")));
    }
}
