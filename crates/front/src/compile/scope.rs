//! 命名空间 / `open` 作用域（G-05，设计 `docs/design/namespace-open.md`）。
//!
//! 这个模块是**名字作用域的唯一实现**：parser（声明名加前缀）与 elab
//! （引用解析）都读同一份规则，避免造出第二份名字真相（硬规则 4 的同款禁令）。
//!
//! 两条状态：
//!
//! * **命名空间栈**：累积全前缀，从外到内（`namespace A` + `namespace B`
//!   ⇒ `["A", "A.B"]`）。栈顶 = 当前命名空间。
//! * **open 集合**：`open` 的前缀，按出现顺序（先开的先试）。
//!
//! 作用域边界（设计 N5）：`namespace` 是**词法块**（parser 在文件尾校验闭合，
//! 所以单元边界上栈必然为空）；`open` 是**文件**——闭包编译把多单元拼成一条
//! 扁平命令序，`Walk::run` 在单元切换处 [`NamespaceScope::reset`]，依赖模块的
//! `open` 不会泄漏进入口文件。

/// `parent` 命名空间里的 `name` 展开成什么全前缀。
///
/// `namespace A` + `namespace B` 与一次写成 `namespace A.B` 必须得到同一个
/// `A.B`——parser（声明名前缀）与 elab（栈）都调这一个函数。
pub(crate) fn join_ns(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(parent) if !parent.is_empty() => format!("{parent}.{name}"),
        _ => name.to_string(),
    }
}

/// 解析期/elaborate 期共用的命名空间作用域。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NamespaceScope {
    /// 累积全前缀，从外到内；栈顶是当前命名空间。
    stack: Vec<String>,
    /// `open` 的前缀，按出现顺序（去重）。
    opens: Vec<String>,
}

impl NamespaceScope {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 当前命名空间（最内层全前缀）；空栈 = 根命名空间。
    pub(crate) fn current(&self) -> Option<&str> {
        self.stack.last().map(String::as_str)
    }

    /// `namespace <name>`：把 `name` 接在栈顶后面（[`join_ns`]）。
    pub(crate) fn push(&mut self, name: &str) {
        let full = join_ns(self.current(), name);
        self.stack.push(full);
    }

    /// `end <name>`：弹栈。parser 已校验同名，所以这里只看空栈的防御分支。
    pub(crate) fn pop(&mut self) {
        self.stack.pop();
    }

    /// `open <name>`：加进可省略前缀集合（重复 `open` 只记一次）。
    pub(crate) fn open(&mut self, prefix: &str) {
        if !self.opens.iter().any(|existing| existing == prefix) {
            self.opens.push(prefix.to_string());
        }
    }

    /// 单元（文件）切换处：`open` 是文件作用域，不跨 `import`（设计 N5）。
    pub(crate) fn reset(&mut self) {
        self.stack.clear();
        self.opens.clear();
    }

    /// 引用 `name` 时的候选全名，按解析顺序（设计 N4）：
    ///
    /// 1. 当前命名空间链，从内到外（最长前缀优先）；
    /// 2. 精确 `name`（根命名空间）；
    /// 3. `open` 的前缀，按 `open` 出现顺序。
    ///
    /// 调用方按顺序取**第一个在 `known` 表里能解析**的候选；都没有就是
    /// 既有的 `elab-unknown-identifier`。
    pub(crate) fn candidates<'a>(&'a self, name: &'a str) -> Vec<String> {
        let mut out = Vec::with_capacity(self.stack.len() + 1 + self.opens.len());
        for prefix in self.stack.iter().rev() {
            out.push(format!("{prefix}.{name}"));
        }
        out.push(name.to_string());
        for prefix in &self.opens {
            out.push(format!("{prefix}.{name}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_and_dotted_namespaces_agree() {
        let mut nested = NamespaceScope::new();
        nested.push("A");
        nested.push("B");
        let mut dotted = NamespaceScope::new();
        dotted.push("A.B");
        // 两条路径的**当前命名空间**逐字相同（声明名加前缀读的就是它）；
        // 候选表的差别只在「嵌套那份还知道外层 A」——那是解析顺序 ① 要求的。
        assert_eq!(nested.current(), Some("A.B"));
        assert_eq!(dotted.current(), Some("A.B"));
        assert_eq!(nested.candidates("x")[0], "A.B.x");
        assert_eq!(dotted.candidates("x")[0], "A.B.x");
    }

    #[test]
    fn candidates_follow_the_documented_order() {
        let mut scope = NamespaceScope::new();
        scope.push("A");
        scope.push("B");
        scope.open("C");
        scope.open("D");
        assert_eq!(
            scope.candidates("x"),
            vec!["A.B.x", "A.x", "x", "C.x", "D.x"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn open_is_a_set_and_reset_clears_both_halves() {
        let mut scope = NamespaceScope::new();
        scope.open("C");
        scope.open("C");
        assert_eq!(scope.candidates("x"), vec!["x", "C.x"]);
        scope.push("A");
        scope.reset();
        assert_eq!(scope.current(), None);
        assert_eq!(scope.candidates("x"), vec!["x"]);
    }
}
