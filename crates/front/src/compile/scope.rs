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
//! `open` 不会泄漏进入口文件（`export` 是例外：它**跨 `import`**，重放在
//! `Walk::run` 的导出表里完成，见设计 §N7）。

use crate::ast::OpenFilter;

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
    /// `open` / `export` 的条目，按出现顺序（先开的先试；去重）。
    opens: Vec<OpenEntry>,
}

/// 一条 `open` / `export`：前缀 + 过滤/改名子句（第二刀 §N7）。
///
/// `export` 与 `open` 共用这一份表示：文件内的解析行为逐字相同，差别只在
/// `Walk` 额外把它记进"跨单元导出表"（设计 §N7）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenEntry {
    pub(crate) prefix: String,
    pub(crate) filter: OpenFilter,
}

impl OpenEntry {
    pub(crate) fn new(prefix: &str, filter: OpenFilter) -> Self {
        Self {
            prefix: prefix.to_string(),
            filter,
        }
    }

    /// 这条条目给引用 `name` 的候选全名（没有 ⇒ `None`）。
    ///
    /// * `renaming a => b`：引用 `b` 的候选是 `Foo.a`（改名后的可见名）；
    /// * 其余情况按 [`OpenFilter::visible_short`] 过滤（`only`/`hiding`/原短名
    ///   被改名占位）。
    pub(crate) fn candidate(&self, name: &str) -> Option<String> {
        if let Some((from, _)) = self.filter.renaming.iter().find(|(_, to)| to == name) {
            return Some(format!("{}.{}", self.prefix, from));
        }
        match self.filter.visible_short(name) {
            Some(short) if short == name => Some(format!("{}.{}", self.prefix, name)),
            _ => None,
        }
    }
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

    /// 带子句的 `open` / `export`：**逐字相同**的条目只记一次（`open Foo`
    /// 两次是幂等的；不同子句各占一个位次，按出现顺序参与解析）。
    pub(crate) fn open_entry(&mut self, entry: OpenEntry) {
        if !self.opens.contains(&entry) {
            self.opens.push(entry);
        }
    }

    /// `open Foo in <cmd>` 的作用域标记：记下当前长度。
    pub(crate) fn opens_mark(&self) -> usize {
        self.opens.len()
    }

    /// 撤销到 [`Self::opens_mark`]：`open Foo in <cmd>` 结束。
    ///
    /// `open_entry` 对**逐字相同**的条目是幂等的（不追加），所以这里 truncate
    /// 天然安全：已经开着的同一个前缀不会被误删（长度没变）。
    pub(crate) fn rollback_opens(&mut self, mark: usize) {
        self.opens.truncate(mark);
    }

    /// 单元（文件）切换处：`open` 是文件作用域，不跨 `import`（设计 N5）。
    /// 调用方随后按需重放**导出表**（`export` 跨 `import`，设计 §N7）。
    pub(crate) fn reset(&mut self) {
        self.stack.clear();
        self.opens.clear();
    }

    /// 引用 `name` 时的候选全名，按解析顺序（设计 N4）：
    ///
    /// 1. 当前命名空间链，从内到外（最长前缀优先）；
    /// 2. 精确 `name`（根命名空间）；
    /// 3. `open` 的条目，按出现顺序（每条最多给一个候选——`renaming` 的可见名
    ///    优先于同名声明）。
    ///
    /// 调用方按顺序取**第一个在 `known` 表里能解析**的候选；都没有就是
    /// 既有的 `elab-unknown-identifier`。
    pub(crate) fn candidates<'a>(&'a self, name: &'a str) -> Vec<String> {
        let mut out = Vec::with_capacity(self.stack.len() + 1 + self.opens.len());
        for prefix in self.stack.iter().rev() {
            out.push(format!("{prefix}.{name}"));
        }
        out.push(name.to_string());
        for entry in &self.opens {
            if let Some(candidate) = entry.candidate(name) {
                out.push(candidate);
            }
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
        scope.open_entry(OpenEntry::new("C", OpenFilter::default()));
        scope.open_entry(OpenEntry::new("D", OpenFilter::default()));
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
        scope.open_entry(OpenEntry::new("C", OpenFilter::default()));
        scope.open_entry(OpenEntry::new("C", OpenFilter::default()));
        assert_eq!(scope.candidates("x"), vec!["x", "C.x"]);
        scope.push("A");
        scope.reset();
        assert_eq!(scope.current(), None);
        assert_eq!(scope.candidates("x"), vec!["x"]);
    }

    #[test]
    fn only_hiding_and_renaming_filter_the_candidates() {
        let mut scope = NamespaceScope::new();
        scope.open_entry(OpenEntry::new(
            "A",
            OpenFilter {
                only: Some(vec!["x".to_string()]),
                ..OpenFilter::default()
            },
        ));
        assert_eq!(scope.candidates("x"), vec!["x", "A.x"]);
        // only 之外的短名一个候选都没有。
        assert_eq!(scope.candidates("y"), vec!["y"]);

        let mut hiding = NamespaceScope::new();
        hiding.open_entry(OpenEntry::new(
            "A",
            OpenFilter {
                hiding: vec!["y".to_string()],
                ..OpenFilter::default()
            },
        ));
        assert_eq!(hiding.candidates("x"), vec!["x", "A.x"]);
        assert_eq!(hiding.candidates("y"), vec!["y"]);

        let mut renaming = NamespaceScope::new();
        renaming.open_entry(OpenEntry::new(
            "A",
            OpenFilter {
                renaming: vec![("x".to_string(), "z".to_string())],
                ..OpenFilter::default()
            },
        ));
        // 新名 `z` ⇒ 候选 `A.x`；原短名 `x` 不再是候选。
        assert_eq!(renaming.candidates("z"), vec!["z", "A.x"]);
        assert_eq!(renaming.candidates("x"), vec!["x"]);
        // 没被改名的短名照旧。
        assert_eq!(renaming.candidates("w"), vec!["w", "A.w"]);
    }

    #[test]
    fn local_open_rolls_back_and_keeps_an_already_open_prefix() {
        let mut scope = NamespaceScope::new();
        scope.open_entry(OpenEntry::new("A", OpenFilter::default()));
        let mark = scope.opens_mark();
        scope.open_entry(OpenEntry::new("B", OpenFilter::default()));
        assert_eq!(scope.candidates("x"), vec!["x", "A.x", "B.x"]);
        scope.rollback_opens(mark);
        assert_eq!(scope.candidates("x"), vec!["x", "A.x"], "B must be gone");
        // 已经开着的 `A` 不会被局部 open 的撤销误删（幂等 ⇒ 长度没变）。
        let mark = scope.opens_mark();
        scope.open_entry(OpenEntry::new("A", OpenFilter::default()));
        scope.rollback_opens(mark);
        assert_eq!(scope.candidates("x"), vec!["x", "A.x"]);
    }
}
