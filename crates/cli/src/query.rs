//! `sokonanoda query <op>`：内核真相的 CLI 出口（harness 中立、零配置）。
//!
//! 设计与契约：`docs/design/agent-query-channel.md` §5、`docs/protocol.md`。
//! 与 `sokonanoda --json <file>` 的关系：**同一份判卷，两种视图**——`--json` 是
//! 全量事件流（既有消费者不变），`query` 是"摘要 + 可寻址"的单 JSON 对象，
//! 供 agent/脚本一次解析。两者都由 `front::query` 同一实现产出。
//!
//! 退出码：`0` 成功（含开放的 `sorry` 练习——那是合法状态）、`1` 有拒绝
//! （内核拒绝的声明**或**源文本解析失败）、`2` 用法错误、`3` 环境/输入不可用。
//! **判据永远是 JSON 内容**，不是退出码。

use std::io::Read;
use std::process::ExitCode;

use serde_json::{json, Value};
use sokonanoda_front::query::{offset_of_line_col, QueryDoc, QueryError};

/// 协议标识：字段只增不改，改名视为破坏性变更（需 minor bump + 文档 + 测试）。
const SCHEMA: &str = "soko.query/1";

/// 一次 `query` 调用的参数。
struct Args {
    op: String,
    file: Option<String>,
    text: Option<String>,
    line: Option<usize>,
    col: Option<usize>,
    offset: Option<usize>,
    probe: bool,
    direction: Option<String>,
    expr: Option<String>,
    compact: bool,
    /// `--root <dir>`：项目闭包的模块根（`--text` 里有 `import` 时必需）。
    root: Option<String>,
}

/// 用法错误（退出码 2）。
struct Usage(String);

fn parse_args(argv: &[String]) -> Result<Args, Usage> {
    let mut args = Args {
        op: String::new(),
        file: None,
        text: None,
        line: None,
        col: None,
        offset: None,
        probe: false,
        direction: None,
        expr: None,
        compact: false,
        root: None,
    };
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        if !arg.starts_with("--") {
            if args.op.is_empty() {
                args.op = arg.to_string();
                i += 1;
                continue;
            }
            return Err(Usage(format!("意外的位置参数 `{arg}`")));
        }
        let mut value = |what: &str| -> Result<String, Usage> {
            let v = argv
                .get(i + 1)
                .ok_or_else(|| Usage(format!("{what} 需要一个值")))?;
            i += 1;
            Ok(v.clone())
        };
        match arg {
            "--file" => args.file = Some(value("--file")?),
            "--root" => args.root = Some(value("--root")?),
            "--text" => args.text = Some(value("--text")?),
            "--line" => {
                args.line = Some(
                    value("--line")?
                        .parse()
                        .map_err(|_| Usage("--line 需要一个数字".to_string()))?,
                )
            }
            "--col" => {
                args.col = Some(
                    value("--col")?
                        .parse()
                        .map_err(|_| Usage("--col 需要一个数字".to_string()))?,
                )
            }
            "--offset" => {
                args.offset = Some(
                    value("--offset")?
                        .parse()
                        .map_err(|_| Usage("--offset 需要一个字节偏移".to_string()))?,
                )
            }
            "--direction" => args.direction = Some(value("--direction")?),
            "--expr" => args.expr = Some(value("--expr")?),
            "--probe" => args.probe = true,
            "--compact" => args.compact = true,
            other => return Err(Usage(format!("未知参数 `{other}`"))),
        }
        i += 1;
    }
    if args.op.is_empty() {
        return Err(Usage(
            "用法：sokonanoda query <check|state|goals|holes|hints|reduce|project> [选项]"
                .to_string(),
        ));
    }
    Ok(args)
}

/// 读输入源：`--file` / `--text` / stdin（`-`）。
fn load_source(args: &Args) -> Result<String, String> {
    match (&args.file, &args.text) {
        (Some(_), Some(_)) => Err("--file 与 --text 只能给一个".to_string()),
        (Some(path), None) => {
            if path == "-" {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .map_err(|e| format!("读取 stdin 失败：{e}"))?;
                Ok(buf)
            } else {
                std::fs::read_to_string(path).map_err(|e| format!("读取 {path} 失败：{e}"))
            }
        }
        (None, Some(text)) => Ok(text.clone()),
        (None, None) => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("读取 stdin 失败：{e}"))?;
            if buf.is_empty() {
                return Err("需要 --file <path>、--text <src> 或从 stdin 读入源码".to_string());
            }
            Ok(buf)
        }
    }
}

/// 把 `--offset` / `--line --col` 解析成字节 offset。
/// 装配查询文档：**项目文件优先走闭包缓存**（与 `check`/`build` 同一份摘要键）。
///
/// 之前 `query` 每次都重编译整个闭包（热跑 37ms，而 `check` 热跑 3ms）；现在
/// 命中就直接用缓存里的入口报告与事件，未命中则编译一次并把同一份键写回缓存，
/// 下次 `check`/`build`/`query` 都能用。`--text`（磁盘上没有对应源码）不缓存。
fn load_document(doc: &mut QueryDoc, src: &str) {
    let options = sokonanoda_front::compile::CompileOptions {
        prelude: sokonanoda_front::compile::prelude_mode_from_source(src),
    };
    let project_entry = doc.path.clone().filter(|_| has_imports(src));
    let Some(entry) = project_entry else {
        doc.set_text(src, 1, None);
        return;
    };
    let (plan, digest) =
        crate::project_cache::plan(&entry, Some(src), doc.root.as_deref(), &[], &options);
    if let Some(cached) = crate::project_cache::load(&digest, &options) {
        if let Some(output) = cached.output {
            doc.set_cached_entry(src, 1, cached.report, output, cached.project);
            return;
        }
    }
    doc.set_text(src, 1, None);
    let _ = plan; // 计划已在上面算过摘要；编译走 QueryDoc 自己的路径
    if let Some(report) = doc.project_report_ref() {
        let output = doc.compiled_output().clone();
        let entry_ok = report
            .entry_module()
            .is_none_or(|module| module.report.errors.is_empty());
        let clean = output.errors.is_empty() && entry_ok;
        crate::project_cache::store_if_clean(&digest, &options, report, clean);
    }
}

/// 文本要不要走项目闭包（触发项目路径的唯一条件）。
///
/// 分发用真相层的 `is_project_source`：**入口单独 parse 失败时也看它有没有
/// `import` 行**——入口用了依赖声明的记法时（G-04 第二刀），单文件 parse 必然
/// 报 `notation-unknown-symbol`，而闭包路径能编。
fn has_imports(src: &str) -> bool {
    sokonanoda_front::project::is_project_source(src)
}

fn cursor_of(args: &Args, doc: &QueryDoc) -> Result<usize, Value> {
    if let Some(offset) = args.offset {
        return Ok(offset);
    }
    match (args.line, args.col) {
        (Some(line), Some(col)) => offset_of_line_col(&doc.text, line, col).ok_or_else(|| {
            error_envelope(
                &args.op,
                QueryError::PositionOutOfRange,
                doc.version,
                Some(format!("{line}:{col} 不在源文本范围内")),
            )
        }),
        (Some(_), None) | (None, Some(_)) => Err(error_envelope(
            &args.op,
            QueryError::PositionOutOfRange,
            doc.version,
            Some("--line 与 --col 必须成对给出".to_string()),
        )),
        (None, None) => Err(error_envelope(
            &args.op,
            QueryError::PositionOutOfRange,
            doc.version,
            Some("该 op 需要 --offset 或 --line/--col".to_string()),
        )),
    }
}

/// 错误信封：`ok:false` + 机器可判的 `error.code`。
fn error_envelope(op: &str, err: QueryError, version: u64, message: Option<String>) -> Value {
    json!({
        "schema": SCHEMA,
        "op": op,
        "version": version,
        "ok": false,
        "error": {
            "code": err.code(),
            "message": message.unwrap_or_else(|| err.message().to_string()),
        },
    })
}

fn ok_envelope(op: &str, version: u64, data: Value) -> Value {
    json!({
        "schema": SCHEMA,
        "op": op,
        "version": version,
        "ok": true,
        "data": data,
    })
}

/// 报告类 op（`goals`/`holes`）"问不出来"的统一出口：`ok:false` + 退出码 1。
///
/// 今天只有一种情形走到这里：源文本解析失败（`QueryError::NotParsable`，G-17）。
/// 空数组是**答案**（"这份画布没有声明 / 没有洞"），解析失败不是——所以它绝不
/// 能答空数组 + `ok:true`。退出码 1 与 `check` 同义："这份文件被拒了"，写成
/// `query goals … && 继续` 的脚本必须在坏文件上停下。
fn unable_to_answer(err: QueryError, doc: &QueryDoc, args: &Args) -> ExitCode {
    let envelope = error_envelope(&args.op, err, doc.version, None);
    print(envelope, args, 1)
}

/// 入口：解析 → 执行 → 打印单 JSON 对象 → 退出码。
pub(crate) fn run(argv: &[String], root: Option<&str>) -> ExitCode {
    let args = match parse_args(argv) {
        Ok(args) => args,
        Err(Usage(message)) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };
    // 先校验 op 名：未知 op 要报"未知 op"，而不是在下游报"缺输入源"。
    if !matches!(
        args.op.as_str(),
        "check" | "state" | "goals" | "holes" | "hints" | "reduce" | "project"
    ) {
        eprintln!(
            "error: 未知的 query op `{}`（支持 check/state/goals/holes/hints/reduce/project）",
            args.op
        );
        return ExitCode::from(2);
    }
    let src = match load_source(&args) {
        Ok(src) => src,
        Err(message) => {
            eprintln!("error: {message}");
            // 输入不可用 = 环境/参数边界；用 2（用法）最贴近"命令没跑起来"。
            return ExitCode::from(2);
        }
    };
    let mut doc = QueryDoc::new();
    // 入口路径 / `--root`：只有带 `import` 的文档才会用到（项目闭包编译）。
    doc.path = args.file.as_ref().map(std::path::PathBuf::from);
    doc.root = args.root.as_deref().or(root).map(std::path::PathBuf::from);
    load_document(&mut doc, &src);

    let (payload, exit) = match args.op.as_str() {
        "check" => {
            let summary = doc.check();
            let failed = summary.failed.len();
            (
                serde_json::to_value(&summary).unwrap_or(Value::Null),
                if failed == 0 { 0u8 } else { 1u8 },
            )
        }
        "state" => {
            let cursor = match cursor_of(&args, &doc) {
                Ok(cursor) => cursor,
                Err(envelope) => return print(envelope, &args, 0),
            };
            match doc.state_at(cursor) {
                Ok(answer) => (serde_json::to_value(&answer).unwrap_or(Value::Null), 0),
                Err(err) => {
                    // `ok:false` 是一个**答案**（问不出来且说明了原因）→ 退出 0；
                    // 退出码只区分"有没有答案"与"文件是否被内核拒"（见模块头）。
                    let envelope = error_envelope(&args.op, err, doc.version, None);
                    return print(envelope, &args, 0);
                }
            }
        }
        "goals" => {
            // 探针默认关闭（每次请求都要跑内核，见 `spine-meta-a.md`）；
            // `holes` 因为**需要**期望类型，内部总是开。
            match doc.goals(args.probe) {
                Ok(decls) => (serde_json::to_value(decls).unwrap_or(Value::Null), 0),
                Err(err) => return unable_to_answer(err, &doc, &args),
            }
        }
        "holes" => {
            let holes = match doc.holes() {
                Ok(holes) => holes,
                Err(err) => return unable_to_answer(err, &doc, &args),
            };
            let next = match (args.offset, args.direction.as_deref()) {
                (Some(from), Some(dir)) => {
                    let forward = match dir {
                        "next" => true,
                        "prev" => false,
                        other => {
                            eprintln!("error: --direction 只能是 next 或 prev（收到 {other}）");
                            return ExitCode::from(2);
                        }
                    };
                    match doc.next_hole(from, forward) {
                        Ok(hole) => serde_json::to_value(hole).unwrap_or(Value::Null),
                        Err(err) => return unable_to_answer(err, &doc, &args),
                    }
                }
                _ => Value::Null,
            };
            (
                json!({ "holes": serde_json::to_value(&holes).unwrap_or(Value::Null), "navigated": next }),
                0,
            )
        }
        "hints" => {
            let cursor = match cursor_of(&args, &doc) {
                Ok(cursor) => cursor,
                Err(envelope) => return print(envelope, &args, 0),
            };
            let hints = doc.hints_at(cursor);
            (json!({ "hints": hints }), 0)
        }
        // 项目状态视图：只读派生（不重跑内核）。单文件不是错误——
        // `project: null` + `reason` 是**合法答案**，退出码仍是 0。
        "project" => {
            let view = doc.project_view();
            let reason = if view.is_none() {
                Value::String(doc.project_view_reason().to_string())
            } else {
                Value::Null
            };
            (
                json!({
                    "project": serde_json::to_value(&view).unwrap_or(Value::Null),
                    "reason": reason,
                }),
                0,
            )
        }
        "reduce" => {
            let Some(expr) = args.expr.as_deref() else {
                eprintln!("error: reduce 需要 --expr <表达式>");
                return ExitCode::from(2);
            };
            match doc.reduce(expr) {
                Some(answer) => (serde_json::to_value(&answer).unwrap_or(Value::Null), 0),
                None => (
                    json!({ "error": "该表达式无法求值（可能引用了不存在的名字）" }),
                    1,
                ),
            }
        }
        // 入口已校验 op，这里不可达；保留分支是为了穷尽匹配。
        _ => unreachable!("op validated before dispatch"),
    };

    print(ok_envelope(&args.op, doc.version, payload), &args, exit)
}

fn print(envelope: Value, args: &Args, exit: u8) -> ExitCode {
    let text = if args.compact {
        serde_json::to_string(&envelope)
    } else {
        serde_json::to_string_pretty(&envelope)
    }
    .unwrap_or_else(|_| "{}".to_string());
    println!("{text}");
    ExitCode::from(exit)
}
