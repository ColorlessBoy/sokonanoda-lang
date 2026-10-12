//! `textDocument/completion` 的候选表装配（**唯一实现**）。
//!
//! ## 为什么单独一个模块（长尾 ④，2026-10-12）
//!
//! 补全以前是 `Backend::completion` 里一段内联代码，只给四样东西：**光标处已有
//! token 的 hover 行**里的本域 binder、语言关键字、宇宙、prelude 名，加**本文
//! 件自己**的声明。于是课程现场有三条缺口（`scripts/ux-probes/lsp-probe.mjs`）：
//!
//! * **C2**：`import lib.Set` 之后 `Set.mem` / `Set.singleton` / `Set.subset_def`
//!   **一条都不在候选里** —— 库名对补全不可见；
//! * **C1**：binder 只在**光标落在已存在的 token 里面**时出现（那要靠
//!   `report.hovers` 里某个 span 真包含光标），**行末**（真正打字处）一个都不给；
//! * **C4**：prelude 名只有 `detail: "prelude"`，**没有签名** ⇒「要写几个实参」无解；
//! * **C5**：`\in` 这类输入法缩写不在服务面（VS Code 有 JS 改写器，别的客户端没有）。
//!
//! ## 库名的来源（**边界**：不许把课程 `lib/**` 硬编码进 LSP）
//!
//! 唯一来源是**已解析的项目闭包**——`QueryDoc::project_modules()`
//! （`crates/front/src/project/**` 那套，`soko/project` 请求读的也是它）。
//! 每个依赖模块按**模块名最后一段**加前缀：模块 `lib.Set` 的 `mem` ⇒ 候选
//! `Set.mem`。规则有两条现成的旁证：① 课程库用 `namespace Set` 包起来，声明名
//! 本来就是 `Set.mem`（`project_definition` 按全名精确匹配，那条跨文件跳转用例
//! 正是拿 `Set.singleton` 咬住的）；② 入口模块（拓扑序最后一项）已经由
//! 「本文件自己的声明」那一支覆盖，**不重复**。
//!
//! 硬编码课程库内容会随课程漂移 ✗；读闭包则**课程改一行、补全跟着变** ✓。
//!
//! ## 位置相关性（C3 的一半）
//!
//! 关键字 / 宇宙 / prelude 名这三份与光标位置无关（它们本来就全域可见），
//! 但**本域 binder**、**库名**、**`\…` 输入法**都随位置变（C1/C5 的判据就是
//! 位置）。`sortText` 按"越本地越靠前"分档，客户端不再需要自己猜排序。

use sokonanoda_front::compile::{DeclKind, DeclState, DeclStatus, PreludeMode};
use sokonanoda_front::query::QueryDoc;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, MarkupContent,
    MarkupKind, Position, Range, TextEdit,
};

/// `sortText` 分档（字典序即优先级）：越本地越靠前。
///
/// 分档的理由是**用户先看什么**：光标处的 binder > 本文件声明 > 库名 > 宇宙 >
/// prelude > 关键字。同档内保持插入顺序（稳定）。
mod rank {
    pub(super) const BINDER: &str = "0";
    pub(super) const CURRENT_DECL: &str = "1";
    pub(super) const CLOSURE_DECL: &str = "2";
    pub(super) const SORT: &str = "3";
    pub(super) const PRELUDE: &str = "4";
    pub(super) const KEYWORD: &str = "5";
    pub(super) const INPUT: &str = "6";
}

/// 代码块围栏语言（与 `lib.rs::code_block` 同口径；文档弹窗里的高亮）。
const CODE_LANG: &str = "sokonanoda";

fn code_block(text: &str) -> String {
    format!("```{CODE_LANG}\n{}\n```", text.trim_end_matches('\n'))
}

fn doc_block(value: String) -> Option<Documentation> {
    Some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }))
}

/// **逃生门**（只给反向验证用）：`SOKO_COMPLETION_CLOSURE=0` 关掉"库名进候选"。
///
/// 为什么留它：这条修复的判据（`scripts/ux-probes/lsp-probe.mjs` 的
/// `C2-completion-imported-names`）必须能**重新判红** —— 撤掉修复 ⇒ 红，恢复 ⇒
/// 绿。生产路径永不设它（默认 `true`）。
pub(crate) fn closure_enabled() -> bool {
    !matches!(
        std::env::var("SOKO_COMPLETION_CLOSURE").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

/// 闭包里**依赖模块**的声明（入口模块的那份由调用方按本文件处理，不在这里）。
///
/// `modules` 是拓扑序、**入口在最后**（`project_modules()` 的既有契约）⇒
/// 这里取 `len - 1` 之前的所有模块，与 `judge_prefix` / `notation_symbols`
/// 的口径逐字相同（同一份列表，不另立一份）。
pub(crate) fn push_closure_decls(
    items: &mut Vec<CompletionItem>,
    modules: &[sokonanoda_front::project::ModuleReport],
) {
    let Some((entry, deps)) = modules.split_last() else {
        return;
    };
    let _ = entry; // 入口自己由 `push_document_decls` 覆盖
                   // 模块根 = 前缀的基准（`<root>/lib/Set.sokonanoda` ⇒ `lib.Set`）。
    let root = project_root(modules);
    for module in deps {
        if module.status != sokonanoda_front::project::ModuleStatus::Compiled {
            continue; // 被阻断 / 失败模块的报告是空的或不可信
        }
        for decl in &module.report.decls {
            let Some(name) = &decl.name else {
                continue;
            };
            if name.starts_with('_') {
                continue; // 内部名（`_example_N`）
            }
            let Some(label) = qualified_label(&module.name, &module.path, root.as_deref(), name)
            else {
                continue;
            };
            items.push(decl_item(
                &label,
                decl,
                rank::CLOSURE_DECL,
                Some(module.name.as_str()),
            ));
        }
    }
}

/// 模块根（`lib.Set` 前缀的基准）。
///
/// 从**入口那条报告**拿（`ModuleReport.path` 是模块文件，模块名是相对模块根算
/// 的 ⇒ 根 = 入口路径去掉模块名对应的那几段）。LSP 侧拿不到 `ProjectReport.root`
/// 那条路（`project_modules()` 只给模块表），所以这里按 `path` 反推：入口
/// `name` 的分量数就是要去掉的段数（含文件名本身）。
fn project_root(modules: &[sokonanoda_front::project::ModuleReport]) -> Option<std::path::PathBuf> {
    let entry = modules.last()?;
    let components = sokonanoda_front::project::ModuleName::parse(&entry.name)
        .map(|name| name.components().len())
        .unwrap_or_else(|_| entry.name.split('.').count());
    let mut root = entry.path.clone();
    for _ in 0..components {
        root.pop();
    }
    Some(root)
}

/// 闭包里一条声明的**规范名**（跨文件引用用的那个名字）。
///
/// 两条规则，缺一不可（C2 的第一版就栽在第二条上：`namespace Set` 让声明名
/// **本来就是** `Set.mem`，再加前缀就成了 `Set.Set.mem` ✗）：
///
/// 1. **带点的名字已经是规范名**（`Set.mem` —— `namespace Set` 里的 `def mem`；
///    `Eq.subst` 这种本来就在命名空间里的库名同理）⇒ 原样用，不再加前缀；
/// 2. 不带点的**裸名**（模块里没有 `namespace`）⇒ 前缀 = 模块**从模块根算起**的
///    名字（`<root>/lib/Set.sokonanoda` ⇒ `lib.Set`，与 `import lib.Set` 同一串），
///    规则与官方 Lean 同款：`import` 名 = 模块根之下的相对路径去扩展名、斜杠换点。
///    取**最后一段**会在 `Foo/Bar.sokonanoda`（import 名 `Foo.Bar`）这类深路径上
///    给出 `Bar.x` ✗ —— 那不是任何地方认的名字。
fn qualified_label(
    module_name: &str,
    module_path: &std::path::Path,
    root: Option<&std::path::Path>,
    decl: &str,
) -> Option<String> {
    if decl.contains('.') {
        return Some(decl.to_string()); // 已是规范名（命名空间里的声明）
    }
    // ① 从路径推**模块名**（与 `import` 逐字同源）。
    if let Some(relative) = root.and_then(|root| module_path.strip_prefix(root).ok()) {
        let with_dots = relative
            .to_string_lossy()
            .replace(['/', '\\'], ".")
            .trim_end_matches(&format!(".{}", sokonanoda_front::project::MODULE_EXTENSION))
            .to_string();
        // 推出来的必须真的是个模块名（分量合法）才用——否则退回模块名的最后一段。
        if sokonanoda_front::project::ModuleName::parse(&with_dots).is_ok() {
            return Some(format!("{with_dots}.{decl}"));
        }
    }
    // ② 兜底：模块名的最后一段（`lib.Set` ⇒ `Set`）。
    let fallback = module_name.rsplit('.').next().unwrap_or(module_name);
    (!fallback.is_empty()).then(|| format!("{fallback}.{decl}"))
}

/// 本文件自己的声明。
pub(crate) fn push_document_decls_with(items: &mut Vec<CompletionItem>, decls: &[DeclState]) {
    for decl in decls {
        let Some(name) = &decl.name else {
            continue;
        };
        if name.starts_with('_') {
            continue; // internal names (_example_N)
        }
        items.push(decl_item(name, decl, rank::CURRENT_DECL, None));
    }
}

/// 一条声明的候选（本文件与闭包两条支路**共用**，形状不许分叉）。
fn decl_item(
    label: &str,
    decl: &DeclState,
    rank: &str,
    module_name: Option<&str>,
) -> CompletionItem {
    let kind = match decl.kind {
        DeclKind::Inductive => CompletionItemKind::STRUCT,
        DeclKind::Axiom => CompletionItemKind::CONSTANT,
        _ => CompletionItemKind::FUNCTION,
    };
    // `detail`：本文件的声明说「kind · 状态」；库名**带上模块名**说明来历
    // （否则用户不知道这条为什么在这——"它从哪来"是补全最常被问的事）。
    let detail = match module_name {
        Some(module) => format!(
            "{} · 库 {} · {}",
            decl.kind.as_str(),
            module,
            status_word(decl.status)
        ),
        None => format!("{} · {}", decl.kind.as_str(), status_word(decl.status)),
    };
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: Some(detail),
        // Signature as a `sokonanoda` fence so the docs popup is highlighted
        // like every other surface (§7).
        documentation: decl.ty_text.as_ref().map(|ty| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: code_block(&format!("{} {} : {ty}", decl.kind.as_str(), label)),
            })
        }),
        sort_text: Some(format!("{rank}{label}")),
        ..Default::default()
    }
}

/// 与 `render::status_label` 同一口径（那一个是 `pub(crate)` 且不在这里的
/// 依赖面里；两处若分叉，`detail` 会与灯泡/大纲说两套话）。
fn status_word(status: DeclStatus) -> &'static str {
    match status {
        DeclStatus::Open => "exercise: open",
        DeclStatus::Checked => "solved ✓",
        DeclStatus::Failed => "failed",
    }
}

/// **prelude 候选的名字 + 签名**（C4）—— 活跃文档的闭包环境一次算齐。
///
/// ## 数据源（只有一条）
///
/// `judge_print_of` 合成 `#print <名字>`、走完整流水线取内核 `pp_declar` 的文本
/// ——与 hover 的声明卡片、用户在源里真写 `#print` 得到的是**同一份内核渲染**，
/// 不是第二条真相、不是客户端拼字符串 ✓。prelude 声明是**受信安装**（没有
/// `DeclState`），报告里查不到它们，所以只能问内核。
///
/// ## 为什么必须缓存，而且必须缓存"整份列表"
///
/// `judge_print_of` 是**一次完整判定**（真编 —— `judge` 自己的 `type_cache` 在
/// 冷的时候救不了）。实测（debug 构建，2026-10-12）：一条 ≈ **270ms** ⇒
/// 56 条 = **15s**，而补全在打字时**每个字母问一次** ✗。两条一起用：
///
/// * [`PRELUDE_SIGS`]：**进程级** `(闭包前缀, 模式) → [(名字, 签名)]` 缓存。
///   键里带前缀与模式（**不是**文档文本）：签名只取决于"哪些库在环境里 + 哪个
///   prelude 模式"，同一门课里改一行入口**不改变任何签名** ⇒ 键越稳，重算越少；
///   而依赖变了一点都不含糊地整份重算（`judge.rs` 那条"无失效路径缓存"的教训）。
/// * 调用方（`Backend::completion`）一次请求取一次整份列表。
///
/// 为什么不"只算客户端前缀命中的那几条"：客户端的前缀过滤发生在**它自己的
/// UI 里**（补全响应的过滤是客户端的事），服务端并不知道用户打了什么 ⇒ 只算
/// 一部分就会让"没算的那几条"在候选里没有签名（静默降级 ✗，`AGENTS.md`
/// 验证纪律第 4 条）。
pub(crate) fn prelude_signatures(
    query: &QueryDoc,
    mode: PreludeMode,
) -> Vec<(String, Option<String>)> {
    // **闭包前缀 = 签名的全部上下文**：它把"哪些库在环境里"完整编码进来
    //（拼进去的正是各依赖模块的源码）。入口自己那份文本**不进键** —— 签名全部
    // 来自闭包 + prelude 模式，改一行入口不改变任何签名 ⇒ 键越稳，重算越少。
    let prefix = query.judge_prefix(0);
    // 缓存键里 `PreludeMode` 折成 `u8`（那份枚举没有 `Hash`，而它是内核公共面，
    // 不为一个 LSP 缓存去加 derive）。
    let key = (prefix, matches!(mode, PreludeMode::Bare) as u8);
    if let Some(cached) = PRELUDE_SIGS.with(|cell| cell.borrow().get(&key).cloned()) {
        return cached;
    }
    let options = sokonanoda_front::compile::CompileOptions { prelude: mode };
    // **一趟**判定取回全部签名（`judge_print_all`）：一条一次的话是 56 趟，
    // 实测 15–20s ✗ —— 补全在打字时每键一次，那个量级不可接受。
    let names: Vec<&str> = sokonanoda_front::compile::PRELUDE_NAMES.to_vec();
    let printed = sokonanoda_front::judge::judge_print_all(&key.0, &options, &names);
    let computed: Vec<(String, Option<String>)> = names
        .iter()
        .zip(printed)
        .map(|(name, result)| {
            let signature = result
                .and_then(|printed| printed.ok())
                .and_then(|text| signature_of(&text, query));
            ((*name).to_string(), signature)
        })
        .collect();
    PRELUDE_SIGS.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.len() >= PRELUDE_SIGS_MAX {
            cache.clear(); // 有界（同一编辑器会话里前缀不会多到几十份）
        }
        cache.insert(key, computed.clone());
    });
    computed
}

/// 进程级签名缓存的上界（条目数；一条 = 一份闭包前缀的全部 prelude 签名）。
const PRELUDE_SIGS_MAX: usize = 8;

/// 一份闭包前缀（+ 模式）下的**全部** prelude 候选：`[(名字, 签名)]`。
type PreludeCandidates = Vec<(String, Option<String>)>;

/// 缓存键：`(闭包前缀, prelude 模式折成 u8)`。
type PreludeCacheKey = (String, u8);

thread_local! {
    /// 见 [`prelude_signatures`]。**线程局部**：LSP 的请求都在同一 tokio 线程池
    /// 上跑，没必要为它上 `Mutex`；多一份副本也只是多算一次，不会答错。
    static PRELUDE_SIGS: std::cell::RefCell<
        std::collections::HashMap<PreludeCacheKey, PreludeCandidates>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 内核 `#print` 的输出 → **签名**（丢掉 `:= <值>` 那一段）。
///
/// 顶层第一个 ` := ` —— 内核 pp 的**签名**里不会出现裸 ` := `（绑定器是 `=>`、
/// 类型是 `->`/`→`），`:=` 只出现在"声明的值"这一处（`declaration_card` 同款
/// 切分）。补全只要签名，值块丢掉 ✓。
fn signature_of(printed: &str, query: &QueryDoc) -> Option<String> {
    let printed = printed.trim();
    if printed.is_empty() {
        return None;
    }
    let signature = match printed.find(" := ") {
        Some(at) => &printed[..at],
        None => printed,
    };
    Some(query.fold_display(signature))
}

/// prelude 候选：签名（有就给）进 `detail`，完整卡片进 `documentation`（C4）。
pub(crate) fn push_prelude(items: &mut Vec<CompletionItem>, name: &str, signature: Option<&str>) {
    items.push(CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(match signature {
            Some(sig) => format!("prelude · {sig}"),
            None => "prelude".to_string(),
        }),
        documentation: signature.map(|sig| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: code_block(sig),
            })
        }),
        sort_text: Some(format!("{}{name}", rank::PRELUDE)),
        ..Default::default()
    });
}

pub(crate) fn push_keyword(items: &mut Vec<CompletionItem>, keyword: &str) {
    let doc = sokonanoda_front::tactics::doc(keyword);
    items.push(CompletionItem {
        label: keyword.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: doc.map(|d| d.summary.to_string()),
        documentation: doc
            .and_then(|d| doc_block(format!("{}\n\n完整文档：`F12` 跳转", d.summary))),
        sort_text: Some(format!("{}{keyword}", rank::KEYWORD)),
        ..Default::default()
    });
}

pub(crate) fn push_sort(items: &mut Vec<CompletionItem>, sort: &str) {
    items.push(CompletionItem {
        label: sort.to_string(),
        kind: Some(CompletionItemKind::STRUCT),
        detail: Some("宇宙".to_string()),
        sort_text: Some(format!("{}{sort}", rank::SORT)),
        ..Default::default()
    });
}

pub(crate) fn push_binder(items: &mut Vec<CompletionItem>, name: &str) {
    items.push(CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::VARIABLE),
        detail: Some("本域 binder".to_string()),
        sort_text: Some(format!("{}{name}", rank::BINDER)),
        ..Default::default()
    });
}

/// `\…` 输入法候选（C5）。
///
/// 真相源是 `front::notation_input::TABLE`（与 hover 的「输入：`\in`」、
/// VS Code 的 `abbreviation-rewriter.js` 镜像**同一份表**，契约测试钉住两边
/// 逐条相等）——这里不另建一张缩写表 ✗。
///
/// 两个决定：
/// * **符号集 = 本文件 + 闭包声明过的记法**（`QueryDoc::notation_symbols`），
///   不是全部 75 个 —— 打 `\and` 能变出 `∧`，但那个文件里根本写不了 `∧`，
///   给了就是骗人（设计 D3 的同一条纪律）。
/// * **带 `textEdit`**：把光标前的 `\in` 整段换成 `∈`。只给一个文案候选等于
///   让别的客户端自己去实现改写器 ✗（VS Code 有，vim 没有 —— 这正是 C5 的现场）。
///   替换范围由调用方按**词法**算好（本文件里只有这一处知道"用户打到哪里了"）。
pub(crate) fn push_input_method(
    items: &mut Vec<CompletionItem>,
    symbol: &str,
    range: Option<Range>,
) {
    let Some(entry) = sokonanoda_front::notation_input::input_for(symbol) else {
        return;
    };
    if !entry.supported {
        return; // 语言今天还没有这个符号 ⇒ hover 同样不承诺可输入
    }
    let mut abbreviations = vec![entry.abbreviation];
    abbreviations.extend(entry.aliases.iter().copied());
    for abbreviation in abbreviations {
        items.push(CompletionItem {
            // 候选文本就是用户**打出来的东西**（`\in`）——accept 之后由
            // `textEdit` 落成 `∈`，与 VS Code 改写器的手感一致。
            label: format!("\\{abbreviation}"),
            // **过滤用不带 leader 的缩写**：用户打 `\i` 时 `filterText` 里
            // 没有反斜杠，客户端才匹配得上（否则必须逐字打对才出现）。
            filter_text: Some(abbreviation.to_string()),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some(format!("输入法 · {symbol}")),
            documentation: sokonanoda_front::notation_input::input_hint(symbol).and_then(doc_block),
            text_edit: range.map(|range| {
                CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: symbol.to_string(),
                })
            }),
            sort_text: Some(format!("{}{abbreviation}", rank::INPUT)),
            ..Default::default()
        });
    }
}

/// 光标处**正在打的那个 `\…` 片段**的替换范围（`None` = 光标前不是 `\…`）。
///
/// 判据是**词法**的、只看光标左侧：从光标往前吃 `[A-Za-z]`，再要求紧邻一个
/// `\`，且反斜杠**不是**标识符的一部分（`a\in` 这种写法在源里只可能是输入法
/// 片段——`\` 不是标识符字符）。这样 `\in`、`\i`、`\` 都能命中，光标在
/// `(a ∈ A)` 之后则一个都不命中。
pub(crate) fn backslash_fragment_at(text: &str, position: Position) -> Option<Range> {
    let offset = crate::position_to_offset(text, position);
    let bytes = text.as_bytes();
    if offset > bytes.len() {
        return None;
    }
    let mut start = offset;
    while start > 0 && bytes[start - 1].is_ascii_alphabetic() {
        start -= 1;
    }
    if start == 0 || bytes[start - 1] != b'\\' {
        return None;
    }
    start -= 1;
    // 片段必须是同一行里的（跨行 `\` 不可能是输入法）。
    if text[start..offset].contains('\n') {
        return None;
    }
    // 区间换算走 wire 的**唯一入口**（`query_map::range_of_offsets`）—— 别的
    // 地方不再有第二份 offset → Position 实现。
    Some(crate::query_map::range_of_offsets(text, start, offset))
}
