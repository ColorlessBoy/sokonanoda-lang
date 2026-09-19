//! 语法级警告：按声明名判定的教学提示（不参与内核判定）。
use serde::{Deserialize, Serialize};

use crate::ast::{Command, FolFile};
use crate::references::decl_name_span;
use crate::Span;

/// 内核已经定义、不能再声明的名字（`Prop` / `Sort` / `Type`）：表达式位置
/// 的这些标识符一律指向内核定义的那个，不会去查环境里有没有同名声明，
/// 因此同名顶层声明不可能被任何引用命中。
pub const RESERVED_SORT_NAMES: [&str; 3] = ["Prop", "Sort", "Type"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningKind {
    ReservedDeclarationName,
    /// `import` 的模块里还有未完成的练习：它们对下游不可见（洞不污染环境）。
    ImportHasOpenExercises,
    /// 值位里"多出来"的 `sorry`：前面的项已经完成了证明，它接在一个不是
    /// 函数的项后面，填什么都不可能是良类型的应用。与"真缺口"（洞有期望
    /// 类型）是两件事，见 `docs/design/redundant-sorry.md`。
    RedundantSorry,
    /// `open` / `export` 让一个短名有了**两个候选**（或与根上的同名声明撞车）：
    /// 本语言按候选顺序静默取第一个（设计 N4），所以给一条**警告**（不是错误）
    /// 告诉学习者"这个短名其实指向谁"。见 `docs/design/namespace-open.md` §N8。
    OpenShadowedName,
}

impl WarningKind {
    pub fn code(self) -> &'static str {
        match self {
            WarningKind::ReservedDeclarationName => "reserved-declaration-name",
            WarningKind::ImportHasOpenExercises => "import-has-open-exercises",
            WarningKind::RedundantSorry => "redundant-sorry",
            WarningKind::OpenShadowedName => "open-shadowed-name",
        }
    }

    /// 第一版教学提示：学习者读到 warning 时的下一步（与错误 hint 同风格）。
    pub fn hint(self) -> &'static str {
        match self {
            WarningKind::ReservedDeclarationName => {
                "删掉这一行即可；要写命题或类型，直接用内核已经有的 Prop / Sort / Type。"
            }
            WarningKind::ImportHasOpenExercises => {
                "被导入文件里还有 `sorry`：这些声明对下游不可见（未完成的洞不进入环境），下游看不到它们的名字。"
            }
            WarningKind::RedundantSorry => {
                "删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。"
            }
            WarningKind::OpenShadowedName => {
                "要点名的那一个就写全前缀（`A.x`）；要让短名指向另一个候选，用 `open A hiding x` / `open A (x)` / `open A renaming x => y` 把撞车的名字挡掉。"
            }
        }
    }

    /// 这条 warning 是**内核终审**过的，还是每次 update 由 [`collect_warnings`]
    /// 重算的语法级提示？
    ///
    /// 内核终审过的（[`WarningKind::RedundantSorry`]）必须按命令进会话快照
    /// 跨版本复用——增量编辑时信任前缀不会重跑探针，只有快照记得它；
    /// 语法级的每次重算，不能进快照（否则会重复报）。
    pub fn is_kernel_verified(self) -> bool {
        match self {
            WarningKind::ReservedDeclarationName => false,
            // 项目层警告是语法级重算的（不来自内核终审）。
            WarningKind::ImportHasOpenExercises => false,
            WarningKind::RedundantSorry => true,
            // `open` 遮蔽是纯语法的候选表推断（每次 update 在整文件上重算）。
            WarningKind::OpenShadowedName => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileWarning {
    pub kind: WarningKind,
    pub message: String,
    pub span: Span,
}

impl CompileWarning {
    /// Stable machine code (used by `--json` and the protocol).
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// First teaching hint for this warning.
    pub fn hint(&self) -> &'static str {
        self.kind.hint()
    }
}

/// 顶层声明名撞上内核已定义的名字时产出 warning（纯语法，与内核结果无关）。
/// span 收窄到名字 token；拿不到时退回整条声明的 span。
///
/// 第二刀：追加 `open`/`export` 的**遮蔽**警告（§N8）。
pub fn collect_warnings(file: &FolFile) -> Vec<CompileWarning> {
    let mut warnings = Vec::new();
    // `open … in <声明>` 包住的声明照样是声明（`effective_commands` 展开）。
    for command in crate::ast::effective_commands(file) {
        let (name, span) = match command {
            Command::Def { name, span, .. }
            | Command::Theorem { name, span, .. }
            | Command::Axiom { name, span, .. }
            | Command::InductiveBlock { name, span, .. } => (name, span),
            Command::Example { .. }
            | Command::Check { .. }
            | Command::Reduce { .. }
            | Command::Print { .. }
            | Command::Import { .. }
            // 记法命令不是声明（设计 N6）：没有声明名可查，也就不会有
            // 「占了内核保留名」这类 warning。G-05 的 namespace/end/open 同理
            // （它们是作用域命令，声明名加前缀已经在 parser 里落定）。
            | Command::Notation { .. }
            | Command::Namespace { .. }
            | Command::End { .. }
            | Command::Open { .. }
            | Command::OpenIn { .. }
            | Command::Export { .. } => continue,
        };
        if !RESERVED_SORT_NAMES.contains(&name.as_str()) {
            continue;
        }
        let message = if name == "Prop" {
            "`Prop` 内核已经定义过了，不能再声明一次。Prop 在形式化证明里地位特殊：所有命题都住在 Prop 里，代码里每个 `Prop` 指的都是内核定义的那个，这一行声明出来的名字不会被用到。".to_string()
        } else {
            format!(
                "`{name}` 内核已经定义过了，不能再声明一次；代码里每个 `{name}` 指的都是内核定义的那个，这一行声明出来的名字不会被用到。"
            )
        };
        warnings.push(CompileWarning {
            kind: WarningKind::ReservedDeclarationName,
            message,
            span: decl_name_span(&file.src, *span, name).unwrap_or(*span),
        });
    }
    warnings.append(&mut collect_open_shadow_warnings(file));
    warnings
}

/// `open` / `export` 的**遮蔽**警告（第二刀 §N8）：一条 `open` 让某个短名有了
/// 第二个候选时，本语言按 N4 的顺序**静默取第一个**——这里给一条 warning
/// （不是 error，判定不受影响），把"它其实指向谁"说清楚。
///
/// 判据（纯语法，**只看本文件**）：
///
/// * 两个 `open` 都提供同一个短名 ⇒ 后开的被先开的挡住（候选顺序 ③）；
/// * 短名与**根上的**同名声明撞车 ⇒ 根名赢（候选顺序 ②「精确名」在 `open`
///   之前），这条 `open` 对这个短名等于没写。
///
/// **边界（明说，不假装覆盖）**：语法 pass 只看得见本文件声明的名字——依赖
/// 模块（`import`）声明的候选不在 `known` 表里，所以 `open Set`（`Set` 来自
/// 库文件）不会在这里被判遮蔽。要覆盖它得把 elab 的 `known` 表喂进 warning
/// pass，那是另一刀（设计 §7 逐条记账）。`open … in <命令>` 是**有意为之**的
/// 局部作用域，不参与这条警告（否则"我就想在这一条命令里用 A.x"会变成噪声）。
fn collect_open_shadow_warnings(file: &FolFile) -> Vec<CompileWarning> {
    use std::collections::{BTreeSet, HashMap};

    // 本文件声明的短名，按**前缀**分组（`""` = 根命名空间）。
    let mut declared: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut record = |full: &str| {
        let (prefix, short) = match full.rsplit_once('.') {
            Some((prefix, short)) => (prefix.to_string(), short.to_string()),
            None => (String::new(), full.to_string()),
        };
        declared.entry(prefix).or_default().insert(short);
    };
    for command in crate::ast::effective_commands(file) {
        match command {
            Command::Def { name, .. }
            | Command::Theorem { name, .. }
            | Command::Axiom { name, .. } => record(name),
            Command::InductiveBlock {
                name, constructors, ..
            } => {
                record(name);
                // 构造子按**规范名**登记（R1）：`open Wrap` 让 `mk` 可用，
                // 所以 `mk` 也在遮蔽判据里。
                for ctor in constructors {
                    record(&crate::compile::canonical_ctor_name(name, &ctor.name));
                }
            }
            _ => {}
        }
    }
    let roots: BTreeSet<String> = declared.get("").cloned().unwrap_or_default();

    // 已生效的短名 → 它的候选全名（按出现顺序）。
    let mut visible: Vec<(String, String)> = Vec::new();
    let mut warnings = Vec::new();
    for command in &file.commands {
        let (name, filter, span) = match command {
            Command::Open {
                name,
                scoped: false,
                filter,
                span,
            } => (name, filter, span),
            Command::Export { name, filter, span } => (name, filter, span),
            _ => continue,
        };
        let Some(shorts) = declared.get(name) else {
            continue;
        };
        // (短名, 这条 open 给的候选, 真正胜出的候选)
        let mut shadowed: Vec<(String, String, String)> = Vec::new();
        for short in shorts {
            let Some(visible_short) = filter.visible_short(short) else {
                continue;
            };
            let candidate = format!("{name}.{short}");
            if roots.contains(&visible_short) {
                shadowed.push((visible_short.clone(), candidate, visible_short));
            } else if let Some((_, winner)) = visible
                .iter()
                .find(|(already, _)| *already == visible_short)
            {
                shadowed.push((visible_short, candidate, winner.clone()));
            } else {
                visible.push((visible_short, candidate));
            }
        }
        if shadowed.is_empty() {
            continue;
        }
        let keyword = if matches!(command, Command::Export { .. }) {
            "export"
        } else {
            "open"
        };
        let command_text = format!("{keyword} {name}{}", filter.source_text());
        let items = shadowed
            .iter()
            .take(3)
            .map(|(short, candidate, winner)| {
                if *winner == *short {
                    format!("`{short}` 仍然指向根上的 `{winner}`")
                } else {
                    format!("`{short}` 会解析到 `{winner}`（不是 `{candidate}`）")
                }
            })
            .collect::<Vec<_>>()
            .join("；");
        let more = if shadowed.len() > 3 {
            format!("；另有 {} 个短名同理", shadowed.len() - 3)
        } else {
            String::new()
        };
        let message = format!(
            "`{command_text}` 让同名候选撞车：{items}{more}。本语言按候选顺序取第一个能解析的（当前命名空间链 → 精确名 → `open` 的先后），撞车的短名不会报错、只会静默取第一个。"
        );
        warnings.push(CompileWarning {
            kind: WarningKind::OpenShadowedName,
            message,
            span: decl_name_span(&file.src, *span, name).unwrap_or(*span),
        });
    }
    warnings
}
