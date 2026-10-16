//! **载荷的合体形**（A = 内核环境文本 + B = 前端表文本 + C = 库层那趟）——
//! 2026-10-16 从 `artifacts.rs` 拆出来（`AGENTS.md` 硬规则 5；判据 ⑨
//! （`STRUCT-SRC-LINES`）对 `artifacts.rs` 是**只许减不许增** ✓）。
//!
//! 格式：一行定长头 `<magic> <A 的字节数> <B 的字节数>\n`，后面接 A、B、C 三段 ✓
//! （**不包 JSON**：A 是几十万字节的文本，包一层要整体转义 ⇒ 体积 ×1.2 ✗）。
//! 设计 = `docs/design/module-artifacts.md` §8.1/§8.3 与
//! `docs/design/declaration-incremental.md`（A/B 两段的来路）。
//!
//! 公开路径不变：`artifacts.rs` 用 `pub use` / `pub(crate) use` re-export（硬规则 5 ✓）。

use serde::{Deserialize, Serialize};

/// **C 块：库层那趟的产物**（T1-B 批 2 的第三块）。
///
/// 为什么**必须**存它：消费入口要"**跳过库层 walk**、只走入口自己的命令"，而
/// `run_entries` 把**库层输出与入口输出合并**成入口那份 `CompileOutput`
/// （`on_entry` 收的就是合并结果）⇒ 少了库层这半，`--json` 就**不是逐字节相同** ✗。
///
/// 字段名与 [`crate::project::session::LibCheckpoint`] 一一对应（除 `builder`/`tables`
/// —— 那两块是 A/B ✓）。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LibPassFacts {
    /// 库层那趟的合并输出（`kernel_checks` 已搬好 ✓）
    pub out: crate::compile::CompileOutput,
    /// 库层逐模块报告（**并集顺序** ✓）
    pub reports: Vec<crate::compile::DocumentReport>,
    /// 并集顺序下每个库模块的命令区间 —— `std::ops::Range` 没有 serde 实现
    /// ⇒ 落成 `(start, end)` 二元组 ✓
    pub ranges: Vec<(usize, usize)>,
    /// 库层那趟的命令数（合并输出的偏移量 ✓）
    pub n_commands: usize,
    /// 库层那趟去掉 `import` 行之后的命令数（入口趟的 `judge_prefix_offset` ✓）
    pub prefix_commands: usize,
    /// 库层全部单元拼接之后的**闭包前缀**（A4a ✓）
    pub lib_prefix: String,
}

/// 载荷头部（`<magic> <A 的字节数> <B 的字节数>\n` 然后接 A、B、C 三段）。
///
/// **为什么用一行定长头而不是 JSON 包一层**：A 是几十万字节的文本，
/// 包进 JSON 要整体转义（体积 ×1.2、还要多一次分配 ✗）；一行头 + 两段拼接
/// 既确定又可流式校验 ✓。**格式号在头里** ⇒ 不认的号直接当"没有产物" ✓。
pub const PAYLOAD_MAGIC: &str = "soko.module-artifact/1";

/// `A（[`ExportFile`]） + B（[`PassTables`]）` ⇒ 一份载荷文本。
///
/// 任一段写不出来（A 有本版 writer 不支持的节点 / B 是空的且 …）都返回 `Err`
/// ⇒ 调用方**不写产物**（"失败当不存在"，同设计 §8.1 ✓）。
#[allow(dead_code)]
pub(crate) fn encode_payload(
    env: &sokonanoda::util::ExportFile<'_>,
    tables: &crate::compile::PassTables<'_>,
    facts: &LibPassFacts,
) -> Result<String, String> {
    let a = env.to_ndjson()?;
    let b = crate::project::tables::encode(&tables.known, &tables.defs, &tables.inductives)?;
    let c = serde_json::to_string(facts).map_err(|e| format!("库层产物（C 块）序列化失败：{e}"))?;
    Ok(format!(
        "{PAYLOAD_MAGIC} {} {}\n{a}{b}{c}",
        a.len(),
        b.len()
    ))
}

/// 载荷文本 ⇒ `(A, B)`，**装进调用方给的 arena**（T1-B 的硬约束：
/// "装载必须发生在本趟 pass 自己的 arena 里" ⇒ 这条口子必须收 arena，不能自建 ✓）。
///
/// ⚠ **信任模型（设计 §8.3）**：这里按"**自己人写出来的产物**"解析 ⇒ 传的
/// [`Config`] **放行一切公理**（产物里的 `axiom` 是本地声明的回放，不是外来的）。
/// 真要去信任**下载来**的产物，是 §8.3 的开关 + 清单签名那件事，**不是**本函数的事。
#[allow(dead_code)]
pub(crate) fn decode_payload<'a>(
    arena: &'a stumpalo::ArenaRef<'a>,
    text: &str,
) -> Result<
    (
        sokonanoda::util::ExportFile<'a>,
        crate::compile::PassTables<'a>,
        LibPassFacts,
    ),
    String,
> {
    let (a_text, b_text, c_text) = split_payload(text)?;
    let config = sokonanoda::util::Config {
        unsafe_permit_all_axioms: true,
        unpermitted_axiom_hard_error: false,
        ..sokonanoda::util::Config::default()
    };
    let (env, _skipped) = sokonanoda::parser::parse_export_mapped(arena, a_text.as_bytes(), config)
        .map_err(|e| format!("产物里的内核环境解析失败：{e}"))?;
    let decoded = crate::project::tables::decode(b_text)?;
    let by_name = declaration_index(&env);
    let inductives = crate::project::tables::rehydrate_inductives(&decoded.inductives, |n| {
        by_name.get(n).cloned()
    })?;
    let facts: LibPassFacts =
        serde_json::from_str(c_text).map_err(|e| format!("库层产物（C 块）反序列化失败：{e}"))?;
    Ok((
        env,
        crate::compile::PassTables {
            known: decoded.known,
            inductives,
            defs: decoded.defs,
        },
        facts,
    ))
}

/// 拆头 + 按长度切两段。**任何不自洽都 `Err`**（截断、长度超界、格式号不认识 ✓）。
fn split_payload(text: &str) -> Result<(&str, &str, &str), String> {
    let nl = text
        .find('\n')
        .ok_or_else(|| "产物载荷没有头部".to_string())?;
    let header = &text[..nl];
    let rest = &text[nl + 1..];
    let mut parts = header.split(' ');
    let magic = parts.next().unwrap_or("");
    if magic != PAYLOAD_MAGIC {
        return Err(format!("产物载荷格式号不认识（{magic:?}）"));
    }
    let a_len: usize = parts
        .next()
        .ok_or_else(|| "产物头部缺 A 的长度".to_string())?
        .parse()
        .map_err(|_| "产物头部里 A 的长度不是数字".to_string())?;
    let b_len: usize = parts
        .next()
        .ok_or_else(|| "产物头部缺 B 的长度".to_string())?
        .parse()
        .map_err(|_| "产物头部里 B 的长度不是数字".to_string())?;
    if a_len.saturating_add(b_len) > rest.len() {
        return Err(format!(
            "产物头部声称 A+B 有 {} 字节，实际只剩 {}（截断 ✗）",
            a_len.saturating_add(b_len),
            rest.len()
        ));
    }
    Ok((
        &rest[..a_len],
        &rest[a_len..a_len + b_len],
        &rest[a_len + b_len..],
    ))
}

/// `规范名 → 声明` 表（`rehydrate_inductives` 的口子）。
///
/// **为什么要在 `with_ctx` 里自己渲染名字**：内核没有公开的"`NamePtr` ⇒ 字符串"
/// 自由函数（`pretty_printer` 的那个是私有的、且要 `&mut self`）⇒ 这里用
/// [`sokonanoda::util::TcCtx::read_name`]/`read_string` 递归拼（10 行、与
/// `pretty_printer::name_to_string` **同一套语义**：点分隔、`Anon` 为空 ✓）。
fn declaration_index<'a>(
    env: &sokonanoda::util::ExportFile<'a>,
) -> std::collections::HashMap<String, sokonanoda::env::Declar<'a>> {
    env.with_ctx(|ctx, _cache, _bump| {
        let mut out = std::collections::HashMap::new();
        for (name, declar) in env.declars.iter() {
            out.insert(render_name(ctx, *name), declar.clone());
        }
        out
    })
}

fn render_name(ctx: &sokonanoda::util::TcCtx<'_, '_>, n: sokonanoda::util::NamePtr<'_>) -> String {
    match ctx.read_name(n) {
        sokonanoda::name::Name::Anon => String::new(),
        sokonanoda::name::Name::Str(pfx, sfx, _) => {
            let mut out = render_name(ctx, pfx);
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(ctx.read_string(sfx).as_ref());
            out
        }
        sokonanoda::name::Name::Num(pfx, sfx, _) => {
            let mut out = render_name(ctx, pfx);
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(&sfx.to_string());
            out
        }
    }
}
