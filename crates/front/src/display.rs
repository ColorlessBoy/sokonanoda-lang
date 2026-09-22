//! **显示专用文本**：线 C（goal / 类型行用记法）的编译期护栏。
//!
//! 设计：`docs/design/notation-aware-printing.md` §3.5；消费者审计见同文 §2。
//!
//! **为什么需要它**：线 C 的 print-back 把"给人看"的文本重写成记法形式
//! （`Set.mem α a A` → `a ∈ A`），而**同一批字段里有几个同时是 judge 的输入**
//! （`DeclState.goal` / `binders[].ty` / `sub_goals[].ty` —— `suggest.rs` 的
//! `open_spec` 与 `goals.rs` 的 `binder_spec` 会拿它们去合成判定规格，再由
//! `judge_terms_uncached` 走 `parse_expr_text_with` 回读）。把两者混起来是
//! **静默改坏判卷**：学习者看到"判过了"，或者本该过的被判红。
//!
//! ⇒ 用**类型**把"给人看"与"回读"分开：
//!
//! * 没有 `Deref<Target = str>`、没有 `as_str()`、没有 `Into<String>`；
//! * 唯一的读法是 [`DisplayText::as_display_str`]。
//!
//! 于是下面两条**编译不过**——这比注释与 review 可靠：

/// **护栏 1/2：回读路径编译不过**——`parse_expr_text` 要 `&str`，而 `DisplayText`
/// 没有 `Deref<Target = str>`，所以这行过不了类型检查。
///
/// **这条对 `Deref` 敏感**：谁给 `DisplayText` 加上 `Deref`（或让它可以被当成
/// `&str` 用），这条 doctest 立刻从"编译失败"变成"编译通过" ⇒ **红**。
///
/// ```compile_fail
/// use sokonanoda_front::display::DisplayText;
/// use sokonanoda_front::proof::parse_expr_text;
///
/// let shown = DisplayText::new("Set.mem α a A");
/// let _ = parse_expr_text(&shown); // 编译错误：expected `&str`, found `&DisplayText`
/// ```
///
/// **护栏 2/2：没有 `as_str()` 这种"顺手拿回 `&str`"的口子**。
///
/// **这条对"加个访问器"敏感**：`as_str()` 一旦存在，取到 `&str` 之后就能喂给
/// 任何回读入口 ⇒ 护栏形同虚设。
///
/// ```compile_fail
/// use sokonanoda_front::display::DisplayText;
///
/// let shown = DisplayText::new("a ∈ A");
/// let _: &str = shown.as_str(); // 编译错误：no method named `as_str`
/// ```
///
/// 对照（**必须编译过**）：显示出口的读法就一条。
///
/// ```
/// use sokonanoda_front::display::DisplayText;
///
/// let shown = DisplayText::new("a ∈ A");
/// assert_eq!(shown.as_display_str(), "a ∈ A");
/// assert_eq!(shown.to_string(), "a ∈ A"); // `Display` 是给 `format!`/wire 用的
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayText(String);

impl DisplayText {
    /// 包一段**只给人看**的文本。
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// 显示出口唯一的读法（`format!` / JSON wire / fenced code block 都用它）。
    pub fn as_display_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DisplayText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for DisplayText {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<&str> for DisplayText {
    fn from(text: &str) -> Self {
        Self(text.to_string())
    }
}
