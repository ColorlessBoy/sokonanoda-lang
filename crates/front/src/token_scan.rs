//! 词法级预扫描（从 `token.rs` 拆出：判据 ⑨ 的行数预算）：
//! `import` 边、记法符号、记法声明。
use crate::span::{Pos, Span};
use crate::token::{is_valid_notation_symbol, NOTATION_COMMANDS};
use crate::token::{is_ident_continue, is_ident_start};

/// **词法级 `import` 扫描**（G-04 第二刀 §10.3）：每一条 `import <模块名>` 的
/// `(模块名, span)`，按书写顺序、按名字去重。
///
/// 判据与 [`Lexer::on_import_line`] **同款**：`import` 必须在行首（允许前导
/// 空白）、后面跟空白，名字是紧随其后的标识符（`.` 是续接字符 ⇒ `lib.Set`
/// 是一个名字）；行尾的 `--` 注释不算内容。
///
/// 用途只有一个：闭包加载器要在**解析一个模块之前**拿到它的 `import` 边
/// （`project/graph.rs`），因为被导入模块的记法必须先就位——否则入口用了库
/// 记法时会死在 `notation-unknown-symbol` 上，而"解析失败 ⇒ 收不到边 ⇒ 依赖
/// 不加载"是个死锁。**判定永不使用它**（判定只认 kernel）。
pub(crate) fn scan_import_lines(src: &str) -> Vec<(String, Span)> {
    let mut out: Vec<(String, Span)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut line_start = 0usize;
    for (index, line) in src.lines().enumerate() {
        let this_line = line_start;
        line_start += line.len() + 1; // `\n`；最后一行没有也只会多算 1，不再使用
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("import") else {
            continue;
        };
        // `importFoo` 不算（`import` 后面必须是空白或行尾）。
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let indent = line.len() - trimmed.len();
        let name_start = indent + "import".len() + (rest.len() - rest.trim_start().len());
        let name: String = line[name_start..]
            .chars()
            .take_while(|c| is_ident_continue(*c))
            .collect();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        // 列按 **UTF-16 code unit**（LSP 口径 ✓）——走唯一口径
        // [`crate::query::column_width`]（T-D31：这里原来按 `char` 数 ✗）。
        let width = |s: &str| s.chars().map(crate::query::column_width).sum::<usize>();
        let column = width(&line[..name_start]) + 1;
        let start = Pos {
            offset: this_line + name_start,
            line: index + 1,
            column,
        };
        let end = Pos {
            offset: start.offset + name.len(),
            line: index + 1,
            column: column + width(&name),
        };
        out.push((name, Span::new(start, end)));
    }
    out
}

/// **预扫描**（第二刀，设计 §10.2）：源码里所有记法命令声明的符号文本。
///
/// 一趟字符扫描，**跳过 `--` 行注释与字符串字面量**；状态机只有两态：
/// 「刚读完一条记法命令关键字」（含可选的 `:N`）⇒ 紧跟着的字符串字面量就是
/// 符号。它**不解析**文件（不认识命令、不检查目标名）——只回答"哪些字符序列
/// 在本文件里是记法符号"，交给 `tokenize_with_symbols` 做最长匹配。
///
/// 预扫描看得见**整份源码**的声明（包括声明点之前的），所以「声明之前使用」
/// 得到的是 parser 的 `notation-unknown-symbol`（N5 的专用诊断），不是
/// `unknown identifier`——与第一刀 `∈` 的行为一致。
pub(crate) fn scan_notation_symbols(src: &str) -> Vec<String> {
    scan_notation_decls(src)
        .into_iter()
        .map(|(symbol, _)| symbol)
        .collect()
}

/// 本文件声明的记法：**符号 + 展开目标**（词法级扫描，**不依赖 parse 成功**）。
///
/// [`scan_notation_symbols`] 是它的投影（只要符号）。为什么需要目标：hover 要
/// 告诉学习者「这个符号展开成什么」（设计 `docs/design/notation-input.md` §4），
/// 而使用库记法的文件**单文件 parse 必然失败**（记法随 `import` 传播），
/// 所以这条信息不能靠 parse 拿——只能靠词法扫描。
///
/// 目标是 `=>` 之后那个点分标识符（`=> Set.mem`）；扫描不到就是 `None`
/// （半成品记法命令——那是 parse 的错误，不由这里报告）。
pub(crate) fn scan_notation_decls(src: &str) -> Vec<(String, Option<String>)> {
    let chars: Vec<char> = src.chars().collect();
    let mut decls: Vec<(String, Option<String>)> = Vec::new();
    let mut expecting_symbol = false;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // `--` 行注释（`->` 不是注释：这里要求两个 `-`）。
        if c == '-' && chars.get(i + 1) == Some(&'-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' && chars[i] != '\n' {
                text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                i += 1;
            }
            if expecting_symbol {
                let symbol = text.trim().to_string();
                // 与 parser 的合法性判据**同一份**：被拒的符号不进符号表
                // （否则 `"in"` 会把 `infix` 拆成 `in` + `fix`）。
                if is_valid_notation_symbol(&symbol) && !decls.iter().any(|(s, _)| *s == symbol) {
                    let target = scan_notation_target(&chars, &mut i);
                    decls.push((symbol, target));
                }
            }
            expecting_symbol = false;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if is_ident_start(c) {
            let mut text = String::new();
            while i < chars.len() && is_ident_continue(chars[i]) {
                text.push(chars[i]);
                i += 1;
            }
            expecting_symbol = NOTATION_COMMANDS.contains(&text.as_str());
            continue;
        }
        // `infix:50 " … "`：`:` 与数字不改变"下一个字符串是符号"的状态。
        if c == ':' && expecting_symbol {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            continue;
        }
        expecting_symbol = false;
        i += 1;
    }
    decls
}

/// 从记法命令的符号串之后找 `=> <目标名>`；找到就把 `i` 停在目标名之后。
/// 找不到（或半成品）返回 `None`，**不移动 `i`**——后续扫描照旧。
fn scan_notation_target(chars: &[char], i: &mut usize) -> Option<String> {
    let saved = *i;
    let mut j = *i;
    let skip_ws = |j: &mut usize| {
        while *j < chars.len() && chars[*j].is_whitespace() {
            *j += 1;
        }
    };
    // `scoped Foo` 前缀：`=>` 之前可能还有 `scoped <作用域名>`。
    loop {
        skip_ws(&mut j);
        if chars.get(j) == Some(&'=') && chars.get(j + 1) == Some(&'>') {
            j += 2;
            break;
        }
        // `scoped` 的作用域名（点分标识符）或别的修饰词：跳过一段标识符。
        if chars.get(j).copied().is_some_and(is_ident_start) {
            while j < chars.len() && (is_ident_continue(chars[j]) || chars[j] == '.') {
                j += 1;
            }
            continue;
        }
        *i = saved;
        return None;
    }
    skip_ws(&mut j);
    let start = j;
    while j < chars.len() && (is_ident_continue(chars[j]) || chars[j] == '.') {
        j += 1;
    }
    if j == start {
        *i = saved;
        return None;
    }
    let target: String = chars[start..j].iter().collect();
    *i = j;
    Some(target)
}

