//! **方言的 LSP 侧接线**（S5）：`initializationOptions` + `workspace/didChangeConfiguration`
//! 热切换 —— 设计 `docs/design/dialect-config.md` §2 第 3 档 / §3.4。
//!
//! 这一份文件装三件事，`lib.rs` 只留**三处**接线（它有 3860 行硬上限，
//! `actions.rs` 一行都不能加；判据 `scripts/src-lines-baseline.json`）：
//!
//! 1. **值从哪来**：`initialize` 的 `initializationOptions`（规范形态
//!    `{"dialect": "strict"}`，也认 `{"sokonanoda": {"dialect": …}}` 这层 VS Code
//!    设置包裹）与 `workspace/didChangeConfiguration` 的 `settings`；
//! 2. **优先级**：**只往 [`DialectSources`] 的 `editor` 那一格填值** —— 合并仍是
//!    front 的唯一实现 `cli > env > editor > file_directive > manifest > 默认`（设计 §2）。
//!    LSP 进程没有 `--dialect` 旗标（第 1 档），文件指令/清单（第 4/5 档）**本段不接线**
//!    （S4 只在 CLI 侧接，S5 不动判定来源面）⇒ 实际是 `env > 编辑器 > 默认 loose`，
//!    而"**没有**初始化选项时"这条链与今天**逐字节相同**（编辑器那格是 `None`）；
//! 3. **热切换**（§3.4）：生效值一变就 ① 记下新值 ② 作废本会话的编译载体
//!    （[`Compiler::invalidate_dialect_caches`]）③ 用**同一份文本**重新调度所有
//!    已打开文档 —— **不重启服务器、不重发 `initialize`**。
//!
//! ## 为什么值要落进 [`QueryDoc::dialect`]，而不是每个构造点现场读设置
//!
//! [`Doc::set_text`](crate::Doc) 经 [`options_for`] 把解析结果写进文档；之后所有
//! `CompileOptions` 构造点（hover 的常量类型、补全的 prelude 签名、code action、
//! A5 的 warm 请求）都走 [`query_options`] 从**同一份文档**取 —— 否则会出现
//! 「诊断严格、补全宽松」这种半档状态（S5 交付第 4 条）。
//!
//! 方言一变就**整份换新** `QueryDoc`（增量会话 + 入口信任前缀 + 显示表一起作废）：
//! `EntryCache.deps`（`front/src/query/mod.rs` 的 `dependency_fingerprint`）只折
//! **prelude 模式**、没有折方言，跨档复用会让旧前缀的内核结论被当成信任前缀 ✗
//! （同族事故：prelude 模式当初漏进指纹，见那里的注释）。

use std::sync::Mutex;

use serde_json::Value;
/// crate 内其它模块取方言类型用（`lib.rs` 的 `Doc::set_text` 签名里要它）。
pub(crate) use sokonanoda_front::compile::Dialect;
use sokonanoda_front::compile::{dialect_from_env, CompileOptions, DialectSources, PreludeMode};
use sokonanoda_front::query::QueryDoc;
use sokonanoda_front::session::Session;

use crate::{Backend, Compiler};

/// 一次参数解析的结论：**三态**，因为"没说"与"说了个听不懂的"处置不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parsed {
    /// 参数里**没有**方言这一项（给了设置对象、但没有该键）⇒ 来源清空。
    Absent,
    /// 提到了、但值不认识 ⇒ **不动**（一个拼写错误不许静默翻档：用户写
    /// `"strcit"` 时把上一档的 strict 悄悄换成 loose 是最坏的配置面 ✗ ——
    /// 与 S4 文件指令"不认识就不表态"同一条理由）。
    Invalid,
    /// 明确表态。
    Dialect(Dialect),
}

/// 编辑器那**一格**（设计 §2 第 3 档）：`None` = 编辑器没表态。
///
/// **每个 [`Backend`](crate::Backend) 一份**（挂在 [`Compiler`] 上，随它进
/// `Arc`），不是进程级静态量：LSP 单元测试在同一进程里并行开多个服务器，
/// 全局量会让一个用例的 strict 漏进另一个用例的 loose（既有 248 条用例的纪律）。
#[derive(Debug, Default)]
pub(crate) struct DialectConfig {
    /// 编辑器最近一次**有效**表态。`didChangeConfiguration` 的缺省 ⇒ 清空。
    editor: Mutex<Option<Dialect>>,
}

impl DialectConfig {
    /// 编辑器现在的表态（`None` = 不表态）。
    pub(crate) fn editor(&self) -> Option<Dialect> {
        *self.editor.lock().expect("dialect lock")
    }

    fn set(&self, value: Option<Dialect>) {
        *self.editor.lock().expect("dialect lock") = value;
    }

    /// 这一次生效的方言。**唯一**合并点仍是 front 的 [`DialectSources::resolve`]：
    /// 这里只填 `env` 与 `editor` 两格（`cli`/文件指令/清单不在 LSP 进程里）。
    pub(crate) fn effective(&self) -> Dialect {
        sources(self.editor()).resolve()
    }

    /// `initialize`：只认 `initializationOptions` 里的**明确表态**；缺省/不认识
    /// ⇒ 不表态（落到环境变量、再落到默认 `loose` —— 与今天逐字节相同）。
    pub(crate) fn adopt_initialization(&self, value: &Option<Value>) {
        if let Some(value) = value {
            if let Parsed::Dialect(dialect) = parse(value) {
                self.set(Some(dialect));
            }
        }
    }

    /// `workspace/didChangeConfiguration`：返回**生效值是否真的变了**
    /// （变了才作废缓存 + 重编；值没变 ⇒ 一次编译都不白跑）。
    ///
    /// 三态处置见 [`Parsed`]：明确表态 ⇒ 覆盖；**没提** ⇒ 清空编辑器这一格
    /// （用户删掉设置 ⇒ 回到下一档）；提到了但听不懂 ⇒ **保持原值**。
    pub(crate) fn update_from_settings(&self, settings: &Value) -> bool {
        let before = self.effective();
        match parse(settings) {
            Parsed::Dialect(dialect) => self.set(Some(dialect)),
            Parsed::Absent => self.set(None),
            Parsed::Invalid => return false,
        }
        self.effective() != before
    }
}

/// `initialize` 的接线（`lib.rs` 只留一行）：`self.compile.dialect.…` 这种链会超过
/// rustfmt 的 `chain_width` 被折成三行，而 `lib.rs` 的行数预算是**纪律**（不在这里
/// 摊销，就得删别处的注释 ✗）。
pub(crate) fn adopt_initialization(config: &DialectConfig, value: &Option<Value>) {
    config.adopt_initialization(value);
}

/// 五档来源里**LSP 侧**的两格（`env` 与 `editor`）。合并只发生在 `resolve` ✓。
fn sources(editor: Option<Dialect>) -> DialectSources {
    DialectSources {
        // LSP 进程没有 `--dialect` 旗标（那是 CLI 的入口，S3）。
        cli: None,
        env: dialect_from_env(),
        editor,
        // 文件指令（S4）与清单（S4）在 LSP 侧**本段不接线**：没有初始化选项时
        // 行为必须与今天逐字节相同（设计 §4 S5 的可回退点 = 只认 `initialize`）。
        ..DialectSources::default()
    }
}

/// 解析 `initializationOptions` / `didChangeConfiguration.settings` 里的方言。
///
/// 接受四种写法（**值的大小写不敏感**，与 CLI 旗标共用
/// [`Dialect::parse_keyword`]）：
///
/// * `{"dialect": "strict"}` —— 规范形态（`initialize` 的建议形态）；
/// * `{"dialect": "loose"}`；
/// * `{"sokonanoda": {"dialect": "strict"}}` —— VS Code `workspace` 设置的包裹层
///   （`package.json` 的 `configuration.title` 就是 `sokonanoda`）；
/// * `{"dialect": {"strictImplicitArgs": true}}` —— 设计 §2 点名的设置名
///   `sokonanoda.dialect.strictImplicitArgs` 的布尔形态（S6 可直接推这一份）。
///
/// 非对象 / 空对象 / 未知值 ⇒ 不表态（[`Parsed::Absent`] / [`Parsed::Invalid`]）。
fn parse(value: &Value) -> Parsed {
    if !value.is_object() {
        // `null`（客户端没给设置）与字符串/数组都**不算**"删除了设置" ⇒ 不动。
        return Parsed::Invalid;
    }
    let node = value
        .get("sokonanoda")
        .filter(|inner| inner.is_object())
        .unwrap_or(value);
    let Some(dialect) = node.get("dialect") else {
        return Parsed::Absent;
    };
    match dialect {
        Value::String(text) => {
            Dialect::parse_keyword(text).map_or(Parsed::Invalid, Parsed::Dialect)
        }
        // 布尔形态：`true` = strict、`false` = loose（与 `SOKO_STRICT_AT` 的极性
        // 同向：严格是 opt-in）。
        Value::Bool(strict) => Parsed::Dialect(if *strict {
            Dialect::Strict
        } else {
            Dialect::Loose
        }),
        Value::Object(table) => match table.get("strictImplicitArgs") {
            Some(Value::Bool(strict)) => Parsed::Dialect(if *strict {
                Dialect::Strict
            } else {
                Dialect::Loose
            }),
            _ => Parsed::Invalid,
        },
        _ => Parsed::Invalid,
    }
}

/// **读懂方言之后**的选项（hover / 补全 / code action / warm 请求共用）。
///
/// 值来自**这份文档上一次编译时解析的那一格**（[`QueryDoc::dialect`]，由
/// [`options_for`] 写、`adopt_view` 装回）—— 一处解析、全链路只读 ✓。
/// 从没编过（`None`）时退回环境/默认，与 `CompileOptions::new` 同值。
pub(crate) fn query_options(query: &QueryDoc) -> CompileOptions {
    let dialect = query.dialect.unwrap_or_else(|| sources(None).resolve());
    CompileOptions::with_dialect(query.mode, dialect)
}

/// `Doc::set_text` 的入口：**解析一次 + 落进文档**，返回 `(options, 方言是否变了)`。
///
/// 方言变了（或这份文档第一次编）⇒ **整份换新** `QueryDoc`：增量会话、
/// `EntryCache`（信任前缀）与显示表都带着上一档的结论，跨档复用就是静默错判 ✗。
/// 换新之后 `text`/`path`/`overlay` 由 `set_text` 的后续步骤重新写入。
pub(crate) fn options_for(
    doc: &mut QueryDoc,
    editor: Option<Dialect>,
    mode: PreludeMode,
) -> (CompileOptions, bool) {
    let options = CompileOptions::with_dialect(mode, sources(editor).resolve());
    if doc.dialect == Some(options.dialect) {
        return (options, false);
    }
    *doc = QueryDoc::new();
    doc.mode = mode;
    doc.dialect = Some(options.dialect);
    doc.session = Session::new(options);
    (options, true)
}

impl Compiler {
    /// 方言热切换：作废本会话的**编译载体**。
    ///
    /// 载体里存着增量会话、`EntryCache`（信任前缀）与显示表 —— 它们的键**不含
    /// 方言**（见本文件抬头）。只清 `carriers`、**不动** `pending`/`inflight`：
    /// 正在飞的编译由它自己收尾（它装回的结果会被接下来调度的同文本重编覆盖），
    /// 清 `inflight` 会让同一份文档起两个任务、共用一条增量会话 ✗。
    ///
    /// 磁盘缓存（`compile::cache` / `project_cache`）**不用**在这里删：S2 已把
    /// 方言折进键（`keys.rs::flags_state` + `judge::options_key`），下一趟自然 miss。
    fn invalidate_dialect_caches(&self) {
        self.carriers.lock().expect("carriers lock").clear();
    }
}

impl Backend {
    /// `workspace/didChangeConfiguration` 的**全部**逻辑（`lib.rs` 只留一行调用）。
    ///
    /// 顺序固定：① 重新解析（**只认通知参数**，不发 `workspace/configuration`
    /// —— 那会在消息循环里等一个可能永远不来的应答）② 生效值变了才 ③ 作废载体
    /// ④ 用**缓冲区最新文本**重新调度每一份已打开文档（不是磁盘旧文本：
    /// 用户可能正在编辑）。
    pub(crate) async fn apply_dialect_settings(&self, settings: &Value) {
        if !self.compile.dialect.update_from_settings(settings) {
            return;
        }
        let dialect = self.compile.dialect.effective();
        self.compile.invalidate_dialect_caches();
        // 一次性取快照（锁内不做任何编译）：把 handlers 读的那一份也立刻切到新档，
        // 这样"诊断还在重编、`soko/stateAt` 已经按新档补判"不会出现半档 ✗。
        let open: Vec<_> = {
            let mut docs = self.doc.lock().expect("doc lock");
            let order = docs.order.clone();
            order
                .into_iter()
                .filter_map(|uri| {
                    let doc = docs.map.get_mut(&uri)?;
                    doc.doc.dialect = Some(dialect);
                    Some((uri, doc.latest_text().to_string(), Some(doc.version())))
                })
                .collect()
        };
        for (uri, text, version) in open {
            // `always = true` 在 `schedule_refresh` 里：诊断没变也要重发一轮 ——
            // 客户端据此清掉旧档的波浪线 ✓（翻转往往**只有消失**，没有新诊断）。
            self.schedule_refresh(uri, text, version);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 规范形态 + 大小写不敏感 + VS Code 包裹层 + 布尔形态（S6 的设置名）。
    #[test]
    fn the_editor_shapes_are_parsed_with_the_cli_keywords() {
        assert_eq!(
            parse(&serde_json::json!({"dialect": "strict"})),
            Parsed::Dialect(Dialect::Strict)
        );
        assert_eq!(
            parse(&serde_json::json!({"dialect": "LOOSE"})),
            Parsed::Dialect(Dialect::Loose)
        );
        assert_eq!(
            parse(&serde_json::json!({"sokonanoda": {"dialect": "Strict"}})),
            Parsed::Dialect(Dialect::Strict)
        );
        assert_eq!(
            parse(&serde_json::json!({"sokonanoda": {"dialect": {"strictImplicitArgs": false}}})),
            Parsed::Dialect(Dialect::Loose)
        );
        assert_eq!(
            parse(&serde_json::json!({"dialect": true})),
            Parsed::Dialect(Dialect::Strict)
        );
    }

    /// **缺省 / 不认识**都不表态（`Absent` 与 `Invalid` 的区别见 `DialectConfig`）：
    /// 一个拼错的设置值不许静默翻档。
    #[test]
    fn missing_and_unknown_values_do_not_speak() {
        assert_eq!(parse(&serde_json::json!({})), Parsed::Absent);
        assert_eq!(
            parse(&serde_json::json!({"sokonanoda": {"infoview": {}}})),
            Parsed::Absent
        );
        assert_eq!(
            parse(&serde_json::json!({"dialect": "strcit"})),
            Parsed::Invalid
        );
        assert_eq!(
            parse(&serde_json::json!({"dialect": {"strict": true}})),
            Parsed::Invalid
        );
        // 客户端没给设置（`settings: null`）⇒ 不算"删掉了设置"。
        assert_eq!(parse(&Value::Null), Parsed::Invalid);
    }

    /// `options_for` 把值**落进文档**：第一次（或换档）返回 `changed = true`，
    /// 同一个值再来一次返回 `false`（`Doc::set_text` 的文本短路靠它）。
    /// 不 assert 具体档位 —— 本机环境里可能有 `SOKO_STRICT_AT`（判据不该随环境漂移）。
    #[test]
    fn the_dialect_is_written_into_the_document_once() {
        let mut doc = QueryDoc::new();
        let (first, changed) = options_for(&mut doc, Some(Dialect::Strict), PreludeMode::Full);
        assert!(changed, "第一次必须认作变化（否则会被文本短路吞掉）");
        assert_eq!(doc.dialect, Some(first.dialect));
        let (again, changed) = options_for(&mut doc, Some(Dialect::Strict), PreludeMode::Full);
        assert!(!changed, "同一个值不该反复作废会话");
        assert_eq!(again.dialect, first.dialect);
        let (flipped, changed) = options_for(&mut doc, Some(Dialect::Loose), PreludeMode::Full);
        assert!(changed);
        assert_eq!(doc.dialect, Some(flipped.dialect));
        assert_eq!(doc.mode, PreludeMode::Full, "整份换新之后模式不许丢");
    }
}
