//! 跨文件引用与改名（I16 P5 余项）：项目闭包里的一个名字在**所有模块**中的出现点。
//!
//! 跨文件的身份是**名字**而不是 `ResolvedTarget`：`ResolvedTarget::Declaration`
//! 里的 span 是"声明所在那份文本"的坐标，两份文件里数值相同也不代表同一个声明。
//! 闭包里顶层名字唯一由 front 的 `import-name-collision` 保证（重名直接报错、
//! 不编译），所以按名字跨模块收集是安全的。
//!
//! 复用的是 LSP 单文件路径同一套真相：定义名 token 来自
//! `front::references::decl_name_span`，使用点来自每个模块 `report.hovers` 里
//! `resolution` 指向该声明的行——注释/字符串里同名的文本不会被碰到。

use sokonanoda_front::references::decl_name_span;
use sokonanoda_front::Span;
use tower_lsp::lsp_types::*;

/// 闭包里一个模块在 LSP 侧的视图。
///
/// `text` 是**编译时文本**（打开的文档用内存里的最新文本，未打开的用
/// `ModuleReport::source`），所以 span 与 `report` 永远自洽。
pub(crate) struct ModuleView<'a> {
    pub uri: Url,
    pub text: &'a str,
    /// 打开的文档带版本（改名的 WorkspaceEdit 要它）；未打开为 `None`
    /// （LSP 的 `OptionalVersionedTextDocumentIdentifier` 用 null 表示"不校验版本"）。
    pub version: Option<i32>,
    pub report: &'a sokonanoda_front::compile::DocumentReport,
}

impl ModuleView<'_> {
    /// 该模块有没有**声明**这个名字（顶层声明名）。
    fn declares(&self, name: &str) -> bool {
        self.report
            .decls
            .iter()
            .any(|decl| decl.name.as_deref() == Some(name))
    }

    /// 该模块里这个名字的**定义名 token** span（只有声明它的模块才有）。
    fn decl_name(&self, name: &str) -> Option<Span> {
        self.report
            .decls
            .iter()
            .find(|decl| decl.name.as_deref() == Some(name))
            .and_then(|decl| decl_name_span(self.text, decl.span, name))
    }

    /// 该模块里这个名字的全部使用点（含嵌套/遮蔽交给 front 的解析结果）。
    fn uses(&self, name: &str) -> Vec<Span> {
        self.report
            .hovers
            .iter()
            .filter(|hover| {
                matches!(
                    hover.resolution.as_ref(),
                    Some(sokonanoda_front::compile::ResolvedTarget::Declaration { name: target, .. })
                        if target == name
                )
            })
            .map(|hover| hover.span)
            .collect()
    }
}

/// 闭包里有没有别的模块已经声明了 `name`（改名冲突预检：LSP 层先拦，
/// 否则改完整个项目会变成 `import-name-collision` 而编译不了）。
pub(crate) fn declared_elsewhere(modules: &[ModuleView<'_>], name: &str, from: &Url) -> bool {
    modules
        .iter()
        .any(|module| &module.uri != from && module.declares(name))
}

/// `textDocument/references`（项目版）：闭包内该名字的**所有**出现点。
///
/// 顺序：`include_declaration` 时定义排在**最前**（与单文件路径一致），
/// 随后按模块的拓扑序（依赖在前、入口最后）与模块内 offset 升序。
pub(crate) fn references(
    modules: &[ModuleView<'_>],
    name: &str,
    include_declaration: bool,
) -> Vec<Location> {
    let mut out: Vec<Location> = Vec::new();
    if include_declaration {
        for module in modules {
            if let Some(span) = module.decl_name(name) {
                out.push(Location {
                    uri: module.uri.clone(),
                    range: range_of(span),
                });
                break;
            }
        }
    }
    for module in modules {
        let mut spans = module.uses(name);
        spans.sort_by_key(|span| span.start.offset);
        spans.dedup_by_key(|span| span.start.offset);
        out.extend(spans.into_iter().map(|span| Location {
            uri: module.uri.clone(),
            range: range_of(span),
        }));
    }
    out
}

/// `textDocument/rename`（项目版）：每个受影响模块一份 `TextDocumentEdit`。
///
/// 只有**真的**有编辑的模块进列表（打开的模块带版本、未打开的版本为 null）。
/// 名字没有出现在任何模块 → 空列表（调用方据 `declares` 判定"改不到"）。
pub(crate) fn rename_edits(
    modules: &[ModuleView<'_>],
    name: &str,
    new_name: &str,
) -> Vec<TextDocumentEdit> {
    let mut edits: Vec<TextDocumentEdit> = Vec::new();
    for module in modules {
        let mut spans: Vec<Span> = module.uses(name);
        if let Some(decl) = module.decl_name(name) {
            spans.push(decl);
        }
        if spans.is_empty() {
            continue;
        }
        spans.sort_by_key(|span| span.start.offset);
        spans.dedup_by_key(|span| span.start.offset);
        edits.push(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: module.uri.clone(),
                version: module.version,
            },
            edits: spans
                .into_iter()
                .map(|span| {
                    OneOf::Left(TextEdit {
                        range: range_of(span),
                        new_text: new_name.to_string(),
                    })
                })
                .collect(),
        });
    }
    edits
}

/// front（1-based 行列）span → LSP range（0-based）。
fn range_of(span: Span) -> Range {
    Range {
        start: Position {
            line: span.start.line.saturating_sub(1) as u32,
            character: span.start.column.saturating_sub(1) as u32,
        },
        end: Position {
            line: span.end.line.saturating_sub(1) as u32,
            character: span.end.column.saturating_sub(1) as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokonanoda_front::compile::{CompileOptions, PreludeMode};
    use sokonanoda_front::project::{compile_project, ProjectReport};

    const LOGIC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : forall (a b : Prop), a -> b -> And a b\n";

    const CANVAS: &str = "import Logic\n\n\
theorem and_intro_demo (a b : Prop) (h : a) (k : b) : And a b := And.intro a b h k\n";

    /// 真闭包（临时目录里两文件、走完整 project 流水线）——跨文件的名字必须
    /// 由**编译出来的报告**回答，手搓报告测不出真语义。
    ///
    /// 目录名必须带**每个测试自己的 tag**：同进程里两个测试并行跑，共用 `pid`
    /// 目录时一个的 `remove_dir_all` 会把另一个的文件删掉（实测 25 次里红 3 次
    /// ——这是测试自己的 race，不是被测代码的）。
    fn closure(tag: &str) -> (ProjectReport, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "soko-lsp-project-refs-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("Logic.sokonanoda"), LOGIC).expect("write");
        let canvas = dir.join("Canvas.sokonanoda");
        std::fs::write(&canvas, CANVAS).expect("write");
        let report = compile_project(
            &canvas,
            None,
            &CompileOptions {
                prelude: PreludeMode::Full,
            },
            None,
        );
        (report, canvas)
    }

    fn views<'a>(report: &'a ProjectReport) -> Vec<ModuleView<'a>> {
        report
            .modules
            .iter()
            .map(|module| ModuleView {
                uri: Url::from_file_path(&module.path).expect("file url"),
                text: &module.source,
                version: Some(7),
                report: &module.report,
            })
            .collect()
    }

    #[test]
    fn references_span_modules_in_closure_order() {
        let (report, dir) = closure("references");
        let modules = views(&report);
        assert_eq!(modules.len(), 2, "Logic 在前、入口在后");
        assert!(modules[0].uri.as_str().ends_with("Logic.sokonanoda"));

        // 定义在 Logic、使用在 Canvas：include_declaration 时定义排最前。
        let with_decl = references(&modules, "And.intro", true);
        assert_eq!(with_decl.len(), 2, "{with_decl:?}");
        assert!(with_decl[0].uri.as_str().ends_with("Logic.sokonanoda"));
        assert!(with_decl[1].uri.as_str().ends_with("Canvas.sokonanoda"));
        let uses_only = references(&modules, "And.intro", false);
        assert_eq!(uses_only.len(), 1, "{uses_only:?}");
        assert!(uses_only[0].uri.as_str().ends_with("Canvas.sokonanoda"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_edits_cover_every_module_that_uses_the_name() {
        let (report, dir) = closure("rename");
        let modules = views(&report);
        let edits = rename_edits(&modules, "And.intro", "And.mk");
        assert_eq!(edits.len(), 2, "{edits:?}");
        assert_eq!(edits[0].edits.len(), 1, "Logic 只声明一次");
        assert_eq!(edits[1].edits.len(), 1, "Canvas 只使用一次");
        assert_eq!(
            edits[1].text_document.version,
            Some(7),
            "打开的文档要带版本（原子应用）"
        );
        // 没有任何出现点的名字不产生（可能为空的）编辑块。
        assert!(rename_edits(&modules, "And.left", "And.l").is_empty());

        let canvas_uri = &modules[1].uri;
        let logic_uri = &modules[0].uri;
        assert!(declared_elsewhere(&modules, "And", canvas_uri));
        assert!(!declared_elsewhere(&modules, "And", logic_uri));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
