//! kernel 判定的术语匹配（I9 goal 视图深化）。
//!
//! 设计（docs/design/i8-i9.md §2）：不发明第二套判定逻辑——把"候选术语 +
//! 已写 binders"合成一条**完整的声明**（`def _soko_judge_k : <声明类型> :=
//! fun <binders> => <术语>`），交给标准流水线（含 prelude 决策与
//! check-then-add 语义），由完整 kernel 当裁判：
//!
//! - 通过 → [`Judgement::Match`]；
//! - 内核拒绝且带 `def_eq mismatch expected/actual` → [`Judgement::Mismatch`]
//!   （"期望 X / 实际 Y"直接来自内核，呼应 Lean `exact?` 的教训：无效建议
//!   根本不该出现）；
//! - elaborate 失败 → [`Judgement::Error`]（稳定错误码 + 教学提示）。
//!
//! 注意：prelude 决策扫描的是"前缀 + 合成声明"，看不到文档后缀。若用户在
//! 目标声明之后才定义自己的 `Eq`/`Nat`（遮蔽 prelude），判定环境与文档环境
//! 可能有差别——教学文档（练习先于解答）不会出现这种形态。
//!
//! [`judge_value_replace`] 服务失败声明（kernel 拒绝、没有洞）：候选整体
//! 替换 `:=` 之后的值位，同名机制合成判定声明。失败声明的针对性建议
//! （`suggest` 的 Eq 形状 rfl 替换）用它做 kernel 终审。
//!
//! 判定永远走 kernel，不做文本比对（REQUIREMENTS §2.8）。

use crate::compile::{
    check_document_with, compile_fol_with, CheckEvent, CompileError, CompileOptions, DeclStatus,
    DocumentReport,
};
use crate::proof::{parse_expr_text, render_expr};
use crate::span::Pos;
use crate::{tokenize, Binder, BinderKind, Command, Expr, FolFile, Span, Token, TokenKind};

/// 一个开放练习的判定规格：**剩余目标**（与 `DeclState.goal` /
/// `ProofState::goal_text` 同语义）、声明的宇宙参数、已写 binders
/// （名字 + 类型文本；类型必须可解析，`DeclState.binders` 恒有类型——
/// 未写时借用声明层）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenGoalSpec {
    pub universe: Vec<String>,
    pub ty: String,
    pub binders: Vec<GoalBinderSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalBinderSpec {
    pub name: String,
    pub ty: Option<String>,
}

impl OpenGoalSpec {
    /// 从 `#prove` 会话状态构造（REPL 用）。
    pub fn from_goal(universe: Vec<String>, ty: String, binders: Vec<GoalBinderSpec>) -> Self {
        Self {
            universe,
            ty,
            binders,
        }
    }
}

/// 一次 kernel 判定的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// 术语的类型与剩余目标 definitional equal（kernel 判定通过）。
    Match,
    /// kernel 拒绝：期望类型与实际类型（两者都来自内核渲染）。
    Mismatch { expected: String, actual: String },
    /// 术语无法 elaborate（错误码 + 消息，教学提示同诊断管线）。
    Error { code: String, message: String },
}

/// 对开放声明 `open` 逐个判定 `terms` 是否能填进洞里。
/// 返回值与 `terms` 等长、按序对应；每次调用独立跑一遍前缀流水线。
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

/// 判定结果缓存（I13 性能收口）：judge_* 的每次调用都要**重编译整个文档
/// 前缀**（拼合成 `#check` 声明后走完整 compile）——by 块 tactic `apply`/
/// `exact` 让这个 O(前缀) 成本落在每一次按键上。缓存按请求指纹命中，
/// 容量封顶（防内存膨胀）；前缀文本参与指纹，文档任何更早的编辑都会
/// 失效缓存——**保守但正确**。
// 判定缓存容量。**128 是 R2 实测的灾难值**，不是保守值：判定的前缀重编译会
// **递归**触发更早声明的 `by` 块判定（前缀里就有那些 `by`），而 FIFO 128 条
// 一被挤爆，缓存就再也接不住这次递归 ⇒ 成本随声明数**指数**增长
// （实测：单元④ 解答 6 条声明 8.9s、第 7 条 → >60s；整份 >600s 不返回）。
// 课程 Lean 化之前每份文件只有个位数判定，128 够用；tactic 风格之后一份文件
// 轻松上百次判定 ⇒ 把容量提到与"一次判卷的全部判定数"同量级。
const JUDGE_CACHE_CAP: usize = 4096;

/// 判定缓存的值：`Infer` = judge_infer 的类型文本（Ok/Err 都缓存），
/// `Terms` = judge_terms / judge_hole_fill 的结论序列。
#[derive(Clone)]
enum JudgeCacheValue {
    Infer(Result<String, Judgement>),
    Terms(Vec<Judgement>),
}

/// 缓存存储：指纹 → 结论；`Vec` 记录插入序（FIFO 淘汰）。
type JudgeCacheStore = (HashMap<u64, JudgeCacheValue>, Vec<u64>);

fn judge_cache() -> &'static Mutex<JudgeCacheStore> {
    static CACHE: OnceLock<Mutex<JudgeCacheStore>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new((HashMap::new(), Vec::new())))
}

fn judge_cache_key(parts: &[&str]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut h);
        0u8.hash(&mut h); // 分隔符，避免拼接歧义
    }
    h.finish()
}

fn judge_cache_get(key: u64) -> Option<JudgeCacheValue> {
    judge_cache()
        .lock()
        .expect("judge cache")
        .0
        .get(&key)
        .cloned()
}

/// **类型查询**（[`judge_type_of`]）的独立缓存。
///
/// 为什么不与 `judge_infer` 共用：两者是**不同的查询**（一个问"项的类型"、
/// 一个问"项在 binder 语境下的类型"），共用一张 FIFO 会让记法展开的类型查询
/// 把 tactic 判定的条目挤出去（实测：`judge_cache_returns_identical_results_and_
/// stores_entries` 在全量跑里被挤到容量上限而判红）。
fn type_cache() -> &'static Mutex<JudgeCacheStore> {
    static CACHE: OnceLock<Mutex<JudgeCacheStore>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new((HashMap::new(), Vec::new())))
}

fn type_cache_get(key: u64) -> Option<JudgeCacheValue> {
    type_cache()
        .lock()
        .expect("type cache")
        .0
        .get(&key)
        .cloned()
}

fn type_cache_put(key: u64, value: JudgeCacheValue) {
    let mut cache = type_cache().lock().expect("type cache");
    if cache.0.insert(key, value).is_none() {
        cache.1.push(key);
        while cache.1.len() > JUDGE_CACHE_CAP {
            let oldest = cache.1.remove(0);
            cache.0.remove(&oldest);
        }
    }
}

fn judge_cache_put(key: u64, value: JudgeCacheValue) {
    let mut cache = judge_cache().lock().expect("judge cache");
    if cache.0.insert(key, value.clone()).is_none() {
        cache.1.push(key);
        while cache.1.len() > JUDGE_CACHE_CAP {
            let oldest = cache.1.remove(0);
            cache.0.remove(&oldest);
        }
    }
}

#[cfg(test)]
pub(crate) fn judge_cache_len() -> usize {
    judge_cache().lock().expect("judge cache").0.len()
}

/// 某个键**在不在**缓存里（第三刀：容量是 FIFO 的 `JUDGE_CACHE_CAP`，用
/// "长度变大了"当"键进去了"的判据会被别的测试挤爆——直接问键，与容量无关）。
#[cfg(test)]
pub(crate) fn judge_cache_contains(key: u64) -> bool {
    judge_cache()
        .lock()
        .expect("judge cache")
        .0
        .contains_key(&key)
}

fn options_key(options: &CompileOptions) -> String {
    format!("{:?}", options.prelude)
}

pub fn judge_terms(
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    judge_terms_with("", prefix_src, options, open, terms)
}

/// 同 [`judge_terms`]，但把 `extra_prefix`（闭包上下文：被导入模块的声明文本）
/// 拼在文档前缀之前——项目模式下 quick-fix 才看得见导入的名字。
pub fn judge_terms_with(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    let key = judge_cache_key(&[
        extra_prefix,
        prefix_src,
        &options_key(options),
        &format!("{open:?}"),
        &format!("{terms:?}"),
    ]);
    if let Some(JudgeCacheValue::Terms(j)) = judge_cache_get(key) {
        return j;
    }
    let j = judge_terms_uncached(extra_prefix, prefix_src, options, open, terms);
    judge_cache_put(key, JudgeCacheValue::Terms(j.clone()));
    j
}

fn judge_terms_uncached(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    open: &OpenGoalSpec,
    terms: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        terms.len()
    ];
    if terms.is_empty() {
        return judgements;
    }
    // 剩余目标解析失败 → 全部判为解析错误。
    //
    // **前缀先解析**（顺序对调，G-04 第二刀）：前缀的 `FolFile` 里带着本文件
    // 声明过的记法，`render_expr` 打回来的目标文本（`a ∈ A`、`Aᶜ`）必须用同一张
    // 表回读，否则符号会被读成「未声明符号」——这是第一刀就有的边界（`by` 块的
    // 目标文本走 `render_expr` + `parse_expr_text` 往返），第二刀顺手修掉。前缀
    // 本来就为后面的合成声明解析，所以**零额外解析开销**；前缀里没有记法命令时
    // 逐字节等于旧行为。
    let full_prefix = synthesized_prefix(extra_prefix, prefix_src);
    let Ok(prefix_file) = parse_prefix(&full_prefix) else {
        return vec![
            Judgement::Error {
                code: "parse".to_string(),
                message: "前缀源码无法解析".to_string(),
            };
            terms.len()
        ];
    };
    // 记法表按**声明顺序**收，`scoped` 的按「前缀里有没有 `open scoped`」过滤
    // （第三刀 §12.3）：主通道是位置敏感的（声明点之后才生效），回读通道只有
    // 一份前缀，所以取"前缀结束时生效的那些"——`open scoped` 写在用之前是
    // 主通道也要求的写法。
    let mut opened_scopes: Vec<String> = Vec::new();
    let mut notations: Vec<crate::ast::NotationDecl> = Vec::new();
    for command in &prefix_file.commands {
        if let crate::ast::Command::Open {
            name, scoped: true, ..
        } = command
        {
            if !opened_scopes.contains(name) {
                opened_scopes.push(name.clone());
            }
            continue;
        }
        let Some(decl) = command.notation_decl() else {
            continue;
        };
        match &decl.scope {
            Some(scope) if !opened_scopes.contains(scope) => {}
            _ => notations.push(decl),
        }
    }
    let Ok(goal) = crate::proof::parse_expr_text_with(&open.ty, &notations) else {
        return vec![
            Judgement::Error {
                code: "parse".to_string(),
                message: format!("无法解析目标类型 `{}`", open.ty),
            };
            terms.len()
        ];
    };
    // 把已写 binders 折叠回声明类型：`(b1 : T1) -> (b2 : T2) -> 剩余目标`。
    // binder 名字与显隐风格不影响内核检查（只影响打印），统一折成命名箭头。
    let ty = match fold_declared(goal, &open.binders, &notations) {
        Ok(ty) => ty,
        Err(missing) => {
            return vec![
                Judgement::Error {
                    code: "elab-untyped-binder".to_string(),
                    message: format!("binder `{missing}` 缺少类型标注，无法合成判定声明"),
                };
                terms.len()
            ]
        }
    };

    let mut commands = prefix_file.commands;
    let mut failed_parse: Option<usize> = None;
    // The synthesized declarations sit *after* the real prefix in the source:
    // give them a span past `prefix_src` and hand the prefix as the file text so
    // `command.span().start`-based prefix lookup (which `match`'s universe query
    // uses) sees the real declarations again (`Color`, …). Without this, a
    // `by exact match c with …` judgement would fail `elab-match-no-expected-type`.
    let prefix_len = full_prefix.len();
    let after_prefix = Span::new(
        Pos {
            offset: prefix_len,
            line: 0,
            column: 0,
        },
        Pos {
            offset: prefix_len,
            line: 0,
            column: 0,
        },
    );
    for (k, term) in terms.iter().enumerate() {
        let Ok(term_expr) = crate::proof::parse_expr_text_with(term, &notations) else {
            failed_parse = Some(k);
            continue;
        };
        let val = wrap_binders(&open.binders, term_expr, &notations);
        commands.push(Command::Def {
            name: format!("_soko_judge_{k}"),
            universe: open.universe.clone(),
            ty: ty.clone(),
            val,
            span: after_prefix,
        });
    }
    let report = check_document_with(
        &FolFile {
            commands,
            src: full_prefix,
        },
        options,
    );
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if failed_parse == Some(k) {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message: format!("无法解析术语 `{}`", terms[k]),
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

/// **一个项的类型文本**（不合成 lambda、不剥 binder）：合成 `#check <term>`
/// 走完整流水线，取 `TypeChecked` 的 `inferred_type`。
///
/// 用途：记法展开要读**目标常量的签名**（`Set.powerset : (α : Type) → Set α →
/// Set (Set α)`）。[`judge_infer`] 那条路要先合成 `fun (binders) => term` 再逐层
/// 剥 binder，目标**本身是函数**时内核 pp 的多 binder 折叠会让剥离结果错位
/// （第二刀实测：`Set.image` 的签名被剥成
/// `(β : Sort 1) -> … -> (α : Sort 1) (β : Sort 1) -> …`，`''` 的前导参数
/// 永远解不出）。这里直接问，不剥——签名是常量自己的，与调用点的 binder 无关。
pub fn judge_type_of(
    prefix_src: &str,
    options: &CompileOptions,
    term: &str,
) -> Result<String, Judgement> {
    let key = judge_cache_key(&[prefix_src, &options_key(options), "type-of", term]);
    if let Some(JudgeCacheValue::Infer(r)) = type_cache_get(key) {
        return r;
    }
    let r = judge_type_of_uncached(prefix_src, options, term);
    type_cache_put(key, JudgeCacheValue::Infer(r.clone()));
    r
}

/// 记法目标的**签名缓存**（与 [`judge_type_of`] 分开，见
/// `docs/design/course-lean-style.md` §9「判卷成本」）。
///
/// 为什么需要：`elab_notation` 每展开一个符号都要问一次内核「目标常量的类型」，
/// 而 [`judge_type_of`] 的缓存键**含整段前缀**——同一份文件里前缀随每条声明
/// 增长 ⇒ 每条声明的每个记法都命中不了缓存，退化成**全前缀重编译**，总量 O(n²)
/// （实测：课程 Lean 化之后，8 模块闭包从 1.3s 涨到 11.5s，单元解答从秒级涨到
/// 分钟级）。
///
/// 常量的签名与「谁在用它」无关（名字唯一且单调增长），所以这里的键只有
/// （选项, 规范名）。**只缓存成功**：失败照旧走原路（那可能只是"还没声明"）。
pub fn judge_type_of_constant(
    prefix_src: &str,
    options: &CompileOptions,
    name: &str,
) -> Result<String, Judgement> {
    static CACHE: OnceLock<Mutex<HashMap<String, Result<String, Judgement>>>> = OnceLock::new();
    const CAP: usize = 4096;
    let key = format!("{}|{name}", options_key(options));
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return hit;
    }
    let result = judge_type_of(prefix_src, options, name);
    if result.is_ok() {
        if let Ok(mut c) = cache.lock() {
            if c.len() < CAP {
                c.insert(key, result.clone());
            }
        }
    }
    result
}

fn judge_type_of_uncached(
    prefix_src: &str,
    options: &CompileOptions,
    term: &str,
) -> Result<String, Judgement> {
    let mut src = String::from(prefix_src);
    let query_start = src.len();
    src.push_str("#check ");
    src.push_str(term);
    src.push('\n');
    // 片段模式（G-05 §4.1）：前缀可能停在未闭合的 `namespace` 里。
    let Ok(file) = crate::parse_fragment(&src) else {
        return Err(Judgement::Error {
            code: "parse".to_string(),
            message: "无法解析类型查询".to_string(),
        });
    };
    let report = compile_fol_with(&file, options);
    if let Some(err) = query_error(query_start, &report.errors) {
        return Err(err);
    }
    let last_cmd = file.commands.len().checked_sub(1);
    report
        .events
        .iter()
        .zip(report.event_cmds.iter())
        .filter(|(_, cmd)| Some(**cmd) == last_cmd)
        .find_map(|(e, _)| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.clone()),
            _ => None,
        })
        .or_else(|| {
            report.events.iter().rev().find_map(|e| match e {
                CheckEvent::TypeChecked { text, .. } => Some(text.clone()),
                _ => None,
            })
        })
        .ok_or_else(|| {
            prefix_error(&report.errors).unwrap_or(Judgement::Error {
                code: "judge-infer-none".to_string(),
                message: "内核未返回类型".to_string(),
            })
        })
}

/// 推断 `term` 在 `binders` 语境下的**类型文本**（kernel 判定驱动，供
/// `apply` 读取被应用函数的类型）。合成 `<prefix>\n#check fun <binders> =>
/// <term>\n` 走完整流水线，取 `TypeChecked` 事件文本，再剥掉 n 层
/// binder 箭头得 `term` 的类型。
pub fn judge_infer(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    judge_infer_with("", prefix_src, options, binders, term)
}

/// 同 [`judge_infer`]，但把 `extra_prefix`（闭包上下文）拼在文档前缀之前。
pub fn judge_infer_with(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    let key = judge_cache_key(&[
        extra_prefix,
        prefix_src,
        &options_key(options),
        &format!("{binders:?}"),
        term,
    ]);
    if let Some(JudgeCacheValue::Infer(r)) = judge_cache_get(key) {
        return r;
    }
    let r = judge_infer_uncached(extra_prefix, prefix_src, options, binders, term);
    judge_cache_put(key, JudgeCacheValue::Infer(r.clone()));
    r
}

fn judge_infer_uncached(
    extra_prefix: &str,
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    term: &str,
) -> Result<String, Judgement> {
    let mut text = String::from("#check ");
    text.push_str("fun ");
    for b in binders {
        let ty = match &b.ty {
            Some(t) => t.clone(),
            None => {
                return Err(Judgement::Error {
                    code: "elab-untyped-binder".to_string(),
                    message: format!("binder `{}` 缺少类型标注，无法推断", b.name),
                })
            }
        };
        text.push_str(&format!("({} : {}) ", b.name, ty));
    }
    text.push_str("=> ");
    text.push_str(term);
    text.push('\n');
    let mut src = synthesized_prefix(extra_prefix, prefix_src);
    let query_start = src.len();
    src.push_str(&text);
    // 片段模式（G-05 §4.1）：前缀可能停在未闭合的 `namespace` 里，合成的
    // `#check` 必须落在**仍然打开的**那个命名空间内。
    let Ok(file) = crate::parse_fragment(&src) else {
        return Err(Judgement::Error {
            code: "parse".to_string(),
            message: "无法解析推断请求".to_string(),
        });
    };
    let report = compile_fol_with(&file, options);
    if let Some(err) = query_error(query_start, &report.errors) {
        return Err(err);
    }
    // 取**最后一条命令**的 `TypeChecked`：合成的前缀里可能本来就有 `#check`
    // （课程/playground 里很常见），它们的事件排在前面；而我们要的是刚追加的
    // 那条查询。按命令号过滤，不做文本比对。
    let last_cmd = file.commands.len().checked_sub(1);
    let ty = report
        .events
        .iter()
        .zip(report.event_cmds.iter())
        .filter(|(_, cmd)| Some(**cmd) == last_cmd)
        .find_map(|(e, _)| match e {
            CheckEvent::TypeChecked { text, .. } => Some(text.clone()),
            _ => None,
        })
        .or_else(|| {
            // 兜底：`event_cmds` 理论上总是与 `events` 平行；若命令号对不上
            // （例如解析把查询并进了别的命令），退回「最后一条 TypeChecked」。
            report.events.iter().rev().find_map(|e| match e {
                CheckEvent::TypeChecked { text, .. } => Some(text.clone()),
                _ => None,
            })
        })
        .ok_or_else(|| {
            prefix_error(&report.errors).unwrap_or(Judgement::Error {
                code: "judge-infer-none".to_string(),
                message: "内核未返回类型".to_string(),
            })
        })?;
    // 剥掉 `fun (b1:T1) => ... => <codomain>` 的 n 层 binder 箭头。
    // pp 可能把相邻 binder 折叠成 `forall (a b : Prop), ...`（一个 Forall 多
    // binder），所以逐 **单个** binder 剥；余下重渲染成可回读的单箭头链。
    let mut t = ty;
    for _ in 0..binders.len() {
        let Ok(e) = parse_expr_text(&t) else {
            break;
        };
        match peel_one_binder(&e) {
            Some(rest) => t = render_roundtrip(&rest),
            None => break,
        }
    }
    Ok(t)
}

/// 剥掉 `expr` 的第一个 binder（多 binder Forall 去掉首个、单 binder 去 body、
/// Arrow 去 codomain），返回余下结构。
fn peel_one_binder(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() > 1 {
                Some(Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: Span::default(),
                })
            } else {
                Some(body.as_ref().clone())
            }
        }
        Expr::Arrow { codomain, .. } => Some(codomain.as_ref().clone()),
        _ => None,
    }
}

/// 渲染成**可回读**的文本：多 binder Forall 逐名字拆成 `(a : T) -> … ->` 单
/// 箭头链（`proof::render_expr` 会拼成 `(a) (b) ->`，无法再 parse）。
fn render_roundtrip(expr: &Expr) -> String {
    match expr {
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let mut rest = render_roundtrip(body);
            for binder in binders.iter().rev() {
                let ty = binder.ty.as_deref().map(render_expr).unwrap_or_default();
                let (l, r) = match binder.style {
                    BinderKind::Explicit => ("(", ")"),
                    BinderKind::Implicit => ("{", "}"),
                };
                rest = format!("{l}{} : {ty}{r} -> {rest}", binder.name);
            }
            rest
        }
        _ => render_expr(expr),
    }
}

/// 报告里的错误**按归属分拣**：只认落在追加查询那一段里的那条。
///
/// 为什么需要（设计 `docs/design/course-lean-style.md` L2.10）：判定通道是
/// 「文档前缀 + 一条合成查询」拼起来**重编译**。前缀里**任何一条先前失败的
/// 声明**都会让这次重编译报错，而旧代码取 `errors[0]`——那通常是**前缀里的**
/// 错，于是每条 tactic、每个记法都会收到一条与它无关的诊断（实测：一个坏声明
/// 让后面整片文件报 `elab-notation-unknown-target`，改写期极难定位）。
///
/// 判据是**字节偏移**（`span.start.offset` 与查询起点比较），不做文本比对：
/// 解析器给的 span 就是这份拼接文本上的位置。`query_start` = 拼接前
/// `src.len()`（查询文本紧跟在它后面）。
///
/// **前缀里的错不在这里报**——它们在声明通道有自己的 span（G-10/G-15），
/// 而且不该让一条与它们无关的查询失败。只有在查询**自己也没拿到结果**时才用
/// [`prefix_error`] 把它们抬出来解释原因。
fn query_error(query_start: usize, errors: &[CompileError]) -> Option<Judgement> {
    errors
        .iter()
        .find(|e| e.span.start.offset >= query_start)
        .map(|e| Judgement::Error {
            code: e.code().to_string(),
            message: e.message.clone(),
        })
}

/// 查询没拿到结果时的**解释**：前缀里有声明没通过就如实说（比「内核未返回类型」
/// 有信息量），否则 `None`（调用方给既有的兜底码）。
fn prefix_error(errors: &[CompileError]) -> Option<Judgement> {
    let first = errors.first()?;
    Some(Judgement::Error {
        code: "prefix-decl-failed".to_string(),
        message: format!(
            "前面的声明没通过（第 {} 行）：{}——先修它，这条查询才有意义",
            first.span.start.line, first.message
        ),
    })
}

/// 内核 pp 渲染的**类型文本**（G-05，设计 `docs/design/namespace-open.md` §4.6）。
///
/// 合成 `#check fun (x : <ty>) => x` 走完整流水线，取回的文本是
/// `(x : T) -> T`，剥掉那一层 binder 就是 `T` 的**规范文本**（内核自己的
/// pretty printer 产出：命名空间里的短名会渲染成全名 `A.mem`）。
///
/// 为什么需要：`by` 引擎的 `apply` 用**文本**把「被应用函数的 codomain」与
/// 「当前目标」对齐（`unify_spine`），而 codomain 来自内核 pp、目标来自源 AST。
/// 源里写短名时两边文本不同（`mem` vs `A.mem`）——把目标也过一遍内核，
/// 两边就同源了（判定仍在内核，不做文本比对）。
///
/// 失败一律 `None`（调用方退回源 AST：行为与加这条之前逐字相同）。
pub fn judge_render_type(
    prefix_src: &str,
    options: &CompileOptions,
    binders: &[GoalBinderSpec],
    ty: &str,
) -> Option<String> {
    let term = format!("fun (x : {ty}) => x");
    let text = judge_infer(prefix_src, options, binders, &term).ok()?;
    let parsed = parse_expr_text(&text).ok()?;
    let rest = peel_one_binder(&parsed)?;
    Some(render_roundtrip(&rest))
}

/// 把 doc 中 decl_span 命令里的 hole_span 替换为候选 term，改名合成声明
/// 后走完整流水线判定（与 [`judge_terms`] 同语义：合成声明是唯一裁判）。
///
/// 与 [`judge_terms`] 的差别：判定发生在**文档里的真实命令**中。声明名换成
/// `_soko_judge_k`（`example` 声明无名字：把首 token `example` 换成
/// `def _soko_judge_k`；宇宙参数 `{u}` 等保留原样），指定洞的 `sorry` 换成
/// 候选，其余洞保持原样。因此命令里只要还有剩余洞，合成声明就仍是 open
/// 练习，结论如实是 [`Judgement::Error`]——多洞状态的逐洞判定由调用方
/// （`suggest`）改用 [`judge_terms`] 按子洞期望类型完成。
pub fn judge_hole_fill(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    judge_hole_fill_with("", doc_src, options, decl_span, hole_span, candidates)
}

/// 同 [`judge_hole_fill`]，但把 `extra_prefix`（闭包上下文）拼在文档前缀之前。
pub fn judge_hole_fill_with(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let key = judge_cache_key(&[
        extra_prefix,
        doc_src,
        &options_key(options),
        &format!("{decl_span:?}"),
        &format!("{hole_span:?}"),
        &format!("{candidates:?}"),
    ]);
    if let Some(JudgeCacheValue::Terms(j)) = judge_cache_get(key) {
        return j;
    }
    let j = judge_hole_fill_uncached(
        extra_prefix,
        doc_src,
        options,
        decl_span,
        hole_span,
        candidates,
    );
    judge_cache_put(key, JudgeCacheValue::Terms(j.clone()));
    j
}

fn judge_hole_fill_uncached(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    hole_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        candidates.len()
    ];
    if candidates.is_empty() {
        return judgements;
    }
    let all_parse_error = |message: String| -> Vec<Judgement> {
        vec![
            Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            candidates.len()
        ]
    };
    let decl_start = decl_span.start.offset.min(doc_src.len());
    let decl_end = decl_span.end.offset.clamp(decl_start, doc_src.len());
    let slice = &doc_src[decl_start..decl_end];
    let Ok(tokens) = tokenize(slice) else {
        return all_parse_error("声明命令切片无法分词".to_string());
    };
    // 声明名 token 的切片内区间：`example` 换成 `def _soko_judge_k`；
    // def/theorem/axiom 的名字 token 换成 `_soko_judge_k`。名字定位复用
    // `references::decl_name_span`（token 精确，绝不扫描文本）。
    let (name_start, name_end, example_keyword) =
        match decl_name_segment(doc_src, decl_span, decl_start, &tokens) {
            Ok(segment) => segment,
            Err(message) => return all_parse_error(message),
        };
    // 洞的切片内区间：必须确实落在命令里，且切片就是 `sorry`。
    let hole_start = hole_span.start.offset;
    let hole_end = hole_span.end.offset;
    let in_decl = hole_start >= decl_start && hole_end <= decl_end && hole_start < hole_end;
    if !in_decl || &doc_src[hole_start..hole_end] != "sorry" {
        return all_parse_error("洞位置不在该声明的 `sorry` 上".to_string());
    }
    let hole_start = hole_start - decl_start;
    let hole_end = hole_end - decl_start;
    if name_start < hole_end && hole_start < name_end {
        return all_parse_error("声明名与洞重叠，无法合成判定声明".to_string());
    }
    // 逐候选：名字段与洞段两处替换，按偏移拼接出合成命令文本。
    let mut commands: Vec<Command> = Vec::with_capacity(candidates.len());
    let mut failed_parse: Vec<Option<String>> = vec![None; candidates.len()];
    for (k, candidate) in candidates.iter().enumerate() {
        let name = format!("_soko_judge_{k}");
        let name_text = if example_keyword {
            format!("def {name}")
        } else {
            name.clone()
        };
        // 名字段在声明头、洞段在其后；仍按偏移排序，防御性处理乱序输入。
        let segments: [(usize, usize, &str); 2] = if name_start <= hole_start {
            [
                (name_start, name_end, name_text.as_str()),
                (hole_start, hole_end, (*candidate)),
            ]
        } else {
            [
                (hole_start, hole_end, (*candidate)),
                (name_start, name_end, name_text.as_str()),
            ]
        };
        let mut synth = String::with_capacity(slice.len() + name_text.len() + candidate.len());
        let mut cursor = 0usize;
        for (start, end, text) in segments {
            synth.push_str(&slice[cursor..start]);
            synth.push_str(text);
            cursor = end;
        }
        synth.push_str(&slice[cursor..]);
        match crate::parse(&synth) {
            Ok(file) => match file.commands.as_slice() {
                [parsed @ (Command::Def {
                    name: parsed_name, ..
                }
                | Command::Theorem {
                    name: parsed_name, ..
                })] if parsed_name == &name => {
                    commands.push(parsed.clone());
                }
                _ => {
                    failed_parse[k] =
                        Some(format!("合成文本不是声明 `{name}`（候选 `{candidate}`）"));
                }
            },
            Err(err) => {
                failed_parse[k] = Some(format!(
                    "无法解析合成命令（候选 `{candidate}`）：{}",
                    err.message
                ));
            }
        }
    }
    // 前缀命令表 + 合成声明，走与 judge_terms 一致的完整流水线。
    // `extra_prefix`（闭包上下文）拼在文档前缀之前：项目模式下合成声明才能看见
    // 被导入的声明（否则 `And.intro` 之类会报未定义标识符）。
    let full_prefix = synthesized_prefix(extra_prefix, &doc_src[..decl_start]);
    let Ok(mut file) = parse_prefix(&full_prefix) else {
        return all_parse_error("前缀源码无法解析".to_string());
    };
    file.commands.extend(commands);
    let report = check_document_with(&file, options);
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if let Some(message) = failed_parse[k].take() {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

/// 合成判定文件的完整前缀：**闭包上下文**（被导入模块的声明文本）+ 文档前缀。
///
/// 项目模式下 `judge_*` 合成的文件只有文档本身时，内核看不见被导入的名字
/// （`front::suggest` 因此给不出 quick-fix，`match`/`by` 也会报未定义）。
/// `extra_prefix` 为空（单文件）时与今天逐字节相同。
fn synthesized_prefix(extra_prefix: &str, doc_prefix: &str) -> String {
    let mut out = String::with_capacity(extra_prefix.len() + doc_prefix.len());
    out.push_str(extra_prefix);
    out.push_str(doc_prefix);
    out
}

/// 前缀解析：走**片段模式**（G-05，设计 `docs/design/namespace-open.md` §4.1）。
///
/// 前缀是文件的一个片段，`by` 块所在的声明在 `namespace Foo` 里时它必然带着
/// 一个未闭合的 `namespace`——严格入口会报 `parse-namespace-unclosed`，而这里
/// 必须容忍，并且**保持命名空间打开**（拼在后面的合成 `#check`/合成声明要落在
/// 里面，引用才按 N4 解析）。
fn parse_prefix(prefix_src: &str) -> Result<FolFile, ()> {
    crate::parse_fragment(prefix_src).map_err(|_| ())
}

/// 声明名 token 在命令切片内的区间（切片相对偏移）：`example` 返回关键字
/// token 自身（合成时换成 `def _soko_judge_k`）；def/theorem/axiom 返回
/// 名字 token（经 `references::decl_name_span` 定位，绝不扫描文本）。
/// [`judge_hole_fill`] 与 [`judge_value_replace`] 共用。第四个返回值表示
/// 声明是匿名 `example`。
fn decl_name_segment(
    doc_src: &str,
    decl_span: Span,
    decl_start: usize,
    tokens: &[Token],
) -> Result<(usize, usize, bool), String> {
    match tokens.first().map(|t| &t.kind) {
        Some(TokenKind::Ident(kw)) if kw == "example" => {
            let token = &tokens[0];
            Ok((token.span.start.offset, token.span.end.offset, true))
        }
        Some(TokenKind::Ident(kw)) if matches!(kw.as_str(), "def" | "theorem" | "axiom") => {
            let Some(TokenKind::Ident(name)) = tokens.get(1).map(|t| &t.kind) else {
                return Err("声明名缺失，无法合成判定声明".to_string());
            };
            let Some(name_span) = crate::references::decl_name_span(doc_src, decl_span, name)
            else {
                return Err(format!("找不到声明名 `{name}` 的 token"));
            };
            let start = name_span.start.offset.saturating_sub(decl_start);
            let end = name_span.end.offset.saturating_sub(decl_start);
            Ok((start, end, false))
        }
        _ => Err("只有 def/theorem/example 声明可以合成判定".to_string()),
    }
}

/// 把 doc 中 decl_span 命令里 `:=` 之后的**整个值位**替换为候选，改名合成
/// 声明后走完整流水线判定（与 [`judge_hole_fill`] 同语义：合成声明是唯一
/// 裁判）。服务失败声明（kernel 拒绝、没有洞）的针对性建议：候选被 kernel
/// 接受才值得呈现。值位起点由 tokenize 定位（`:=` 后第一个 token），
/// 终点即声明 span 末尾（解析器把 span 收在值的最后一个 token 上）。
///
/// 与 [`judge_hole_fill`] 的差别：没有洞可填——候选**整体替换值位**，
/// 宇宙参数与值位之前的命令头原样保留；`example` 声明仍按首 token 换名。
/// 解析失败一律报 [`Judgement::Error`]，绝不 panic。
pub fn judge_value_replace(
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    judge_value_replace_with("", doc_src, options, decl_span, candidates)
}

/// 同 [`judge_value_replace`]，但把 `extra_prefix`（闭包上下文）拼在文档前缀之前。
pub fn judge_value_replace_with(
    extra_prefix: &str,
    doc_src: &str,
    options: &CompileOptions,
    decl_span: Span,
    candidates: &[&str],
) -> Vec<Judgement> {
    let mut judgements = vec![
        Judgement::Error {
            code: "judge-not-run".to_string(),
            message: "判定未执行".to_string(),
        };
        candidates.len()
    ];
    if candidates.is_empty() {
        return judgements;
    }
    let all_parse_error = |message: String| -> Vec<Judgement> {
        vec![
            Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            candidates.len()
        ]
    };
    let decl_start = decl_span.start.offset.min(doc_src.len());
    let decl_end = decl_span.end.offset.clamp(decl_start, doc_src.len());
    let slice = &doc_src[decl_start..decl_end];
    let Ok(tokens) = tokenize(slice) else {
        return all_parse_error("声明命令切片无法分词".to_string());
    };
    let (name_start, name_end, example_keyword) =
        match decl_name_segment(doc_src, decl_span, decl_start, &tokens) {
            Ok(segment) => segment,
            Err(message) => return all_parse_error(message),
        };
    // 值位：`:=` 后第一个 token 起，到声明 span 末尾。
    let Some(colon_eq) = tokens.iter().position(|t| t.kind == TokenKind::ColonEq) else {
        return all_parse_error("声明没有 `:=` 值位，无法替换判定".to_string());
    };
    let Some(value_tok) = tokens.get(colon_eq + 1) else {
        return all_parse_error("声明没有值位，无法替换判定".to_string());
    };
    if value_tok.kind == TokenKind::Eof {
        return all_parse_error("声明没有值位，无法替换判定".to_string());
    }
    let value_start = value_tok.span.start.offset;
    if name_end > value_start {
        return all_parse_error("声明名与值位重叠，无法合成判定声明".to_string());
    }
    // 逐候选：名字段与值段两处替换，按偏移拼接出合成命令文本（值段延伸到
    // 切片末尾，其后没有剩余文本）。
    let mut commands: Vec<Command> = Vec::with_capacity(candidates.len());
    let mut failed_parse: Vec<Option<String>> = vec![None; candidates.len()];
    for (k, candidate) in candidates.iter().enumerate() {
        let name = format!("_soko_judge_{k}");
        let name_text = if example_keyword {
            format!("def {name}")
        } else {
            name.clone()
        };
        let mut synth = String::with_capacity(slice.len() + name_text.len() + candidate.len());
        synth.push_str(&slice[..name_start]);
        synth.push_str(&name_text);
        synth.push_str(&slice[name_end..value_start]);
        synth.push_str(candidate);
        match crate::parse(&synth) {
            Ok(file) => match file.commands.as_slice() {
                [parsed @ (Command::Def {
                    name: parsed_name, ..
                }
                | Command::Theorem {
                    name: parsed_name, ..
                })] if parsed_name == &name => {
                    commands.push(parsed.clone());
                }
                _ => {
                    failed_parse[k] =
                        Some(format!("合成文本不是声明 `{name}`（候选 `{candidate}`）"));
                }
            },
            Err(err) => {
                failed_parse[k] = Some(format!(
                    "无法解析合成命令（候选 `{candidate}`）：{}",
                    err.message
                ));
            }
        }
    }
    // 前缀命令表 + 合成声明，走与 judge_terms 一致的完整流水线
    // （`extra_prefix` 见 [`judge_hole_fill_with`]）。
    let full_prefix = synthesized_prefix(extra_prefix, &doc_src[..decl_start]);
    let Ok(mut file) = parse_prefix(&full_prefix) else {
        return all_parse_error("前缀源码无法解析".to_string());
    };
    file.commands.extend(commands);
    let report = check_document_with(&file, options);
    for (k, judgement) in judgements.iter_mut().enumerate() {
        if let Some(message) = failed_parse[k].take() {
            *judgement = Judgement::Error {
                code: "parse".to_string(),
                message,
            };
            continue;
        }
        *judgement = judgement_of(&report, k);
    }
    judgements
}

/// 把剩余目标与已写 binders 折叠成完整声明类型：一个 Forall 望远镜
/// `forall (b1 : T1) (b2 : T2), 剩余目标`——与命名箭头的语法语义一致
/// （后一个 binder 的类型可以引用前一个，必须在同一 telescope 内 elaborate）。
/// 返回 `Err(binder_name)` 表示该 binder 缺少类型标注。
fn fold_declared(
    goal: Expr,
    binders: &[GoalBinderSpec],
    notations: &[crate::ast::NotationDecl],
) -> Result<Expr, String> {
    let mut parsed = Vec::with_capacity(binders.len());
    for binder in binders {
        let Some(text) = &binder.ty else {
            return Err(binder.name.clone());
        };
        // binder 的类型也是 `render_expr` 打回来的源码文本：`intro h` 在
        // `a ∈ A -> …` 上引入的 `h` 类型就是 `a ∈ A`（G-04 第二刀）。
        let Ok(domain) = crate::proof::parse_expr_text_with(text, notations) else {
            return Err(binder.name.clone());
        };
        parsed.push(Binder {
            name: binder.name.clone(),
            ty: Some(Box::new(domain)),
            style: BinderKind::Explicit,
            span: Span::default(),
        });
    }
    if parsed.is_empty() {
        return Ok(goal);
    }
    Ok(Expr::Forall {
        binders: parsed,
        body: Box::new(goal),
        span: Span::default(),
    })
}

/// 把术语包上已写 binders：`fun (b1 : T1) => fun (b2 : T2) => term`。
/// 未写类型的 binder 留空，交给声明类型驱动的 binder 推断（I6）。
///
/// **binder 类型文本必须带记法表回读**（课程 Lean 化实测发现，设计
/// `docs/design/course-lean-style.md` X2）：`GoalBinderSpec.ty` 是
/// `render_expr` 打回来的**源码级**文本（`intro h` 在目标 `¬ A -> …` 上
/// 引入的 `h` 类型就是 `¬ A`）。用不带记法表的 `parse_expr_text` 回读时，
/// 数学码点类之外的符号（`¬`U+00AC / `↔`U+2194 / `→`U+2192）会被读成
/// **标识符**，于是 `¬ A` 变成 `App(Ident("¬"), A)`，判卷报
/// `unknown identifier ¬`。`fold_declared`（本文件 `:981`）从一开始就传了
/// `notations`，这里漏了——两处现在同口径。
fn wrap_binders(
    binders: &[GoalBinderSpec],
    term: Expr,
    notations: &[crate::ast::NotationDecl],
) -> Expr {
    let mut term = term;
    for binder in binders.iter().rev() {
        let ty = match &binder.ty {
            Some(text) => crate::proof::parse_expr_text_with(text, notations)
                .ok()
                .map(Box::new),
            None => None,
        };
        term = Expr::Lambda {
            binders: vec![Binder {
                name: binder.name.clone(),
                ty,
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(term),
            span: Span::default(),
        };
    }
    term
}

/// 从合成声明的检查结果提取判定结论。
fn judgement_of(report: &DocumentReport, k: usize) -> Judgement {
    let name = format!("_soko_judge_{k}");
    let Some(state) = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some(name.as_str()))
    else {
        return Judgement::Error {
            code: "judge-missing".to_string(),
            message: format!("合成声明 `{name}` 没有产生状态（内部错误）"),
        };
    };
    match state.status {
        DeclStatus::Checked => Judgement::Match,
        DeclStatus::Open => Judgement::Error {
            code: "elab-hole-misplaced".to_string(),
            message: "洞不在可填写的位置".to_string(),
        },
        DeclStatus::Failed => {
            let Some(err) = &state.error else {
                return Judgement::Error {
                    code: "judge-unknown".to_string(),
                    message: "判定失败但没有错误信息（内部错误）".to_string(),
                };
            };
            match (&err.expected, &err.actual) {
                (Some(expected), Some(actual)) => Judgement::Mismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                },
                _ => Judgement::Error {
                    code: err.code().to_string(),
                    message: err.message.clone(),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{check_document, DeclState, DeclStatus, PreludeMode};
    use crate::parse;

    fn spec(ty: &str, binders: &[(&str, Option<&str>)]) -> OpenGoalSpec {
        OpenGoalSpec {
            universe: Vec::new(),
            ty: ty.to_string(),
            binders: binders
                .iter()
                .map(|(name, ty)| GoalBinderSpec {
                    name: name.to_string(),
                    ty: ty.map(|t| t.to_string()),
                })
                .collect(),
        }
    }

    #[test]
    fn judge_cache_returns_identical_results_and_stores_entries() {
        // 缓存是性能设施（by 块 tactic 的 judge_infer 每键全前缀重编译是
        // funapply 移除的根因）：命中必须返回与直算一致的结果，且条目入库。
        let prefix = "axiom P : Prop\naxiom Q : Prop\n";
        let binders = vec![crate::judge::GoalBinderSpec {
            name: "h".into(),
            ty: Some("P".into()),
        }];
        let options = CompileOptions::default();
        let first = judge_infer(prefix, &options, &binders, "h");
        let before = judge_cache_len();
        let second = judge_infer(prefix, &options, &binders, "h");
        assert_eq!(first, second, "cache must not change results");
        assert!(judge_cache_len() >= before, "the request must be cached");

        // 不同 prelude 模式是指纹的一部分：不串台。
        // 判据是**键在不在**（不是"缓存变长了"）：容量是 FIFO 的
        // `JUDGE_CACHE_CAP`，别的测试把缓存填满时长度不再增长，旧判据会假红
        // （第三刀实测：新增的记法测试把缓存填到上限）。
        let bare = CompileOptions {
            prelude: crate::compile::PreludeMode::Bare,
        };
        let _ = judge_infer(prefix, &bare, &binders, "h");
        // 键的组成与 `judge_infer_with` 逐字一致（`extra_prefix` 是空串）。
        let bare_key = judge_cache_key(&[
            "",
            prefix,
            &options_key(&bare),
            &format!("{binders:?}"),
            "h",
        ]);
        assert!(
            judge_cache_contains(bare_key),
            "different options = different key"
        );
        assert!(judge_cache_len() >= before);
    }

    #[test]
    fn matching_hypothesis_is_a_kernel_match() {
        let prefix = "axiom a : Prop\n";
        // 剩余目标 a，已写 binder h : a ⇒ 折叠出的声明类型是 (h : a) -> a。
        let open = spec("a", &[("h", Some("a"))]);
        let judgements = judge_terms(
            prefix,
            &CompileOptions::default(),
            &open,
            &["h", "fun (x : Prop) => x"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        match &judgements[1] {
            Judgement::Mismatch { expected, actual } => {
                // debug printer 渲染：Prop → Sort(0)，宇宙参数带 .[] 后缀。
                assert!(expected.contains("a"), "expected: {expected}");
                assert!(actual.contains("Sort(0)"), "actual: {actual}");
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn dependent_binder_types_are_judged_correctly() {
        // h : a（依赖前面的 binder a）与剩余目标 a 相同。
        let open = spec("a", &[("a", Some("Prop")), ("h", Some("a"))]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["h"]);
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn binder_without_type_annotation_is_reported() {
        // 声明层总会为未写类型的 binder 借来类型，所以 None 只可能是
        // 防御性输入；判定明确报错而不是静默猜测。
        let open = spec("Prop", &[("x", None)]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["x"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "elab-untyped-binder");
                assert!(message.contains('x'), "message: {message}");
            }
            other => panic!("expected untyped-binder error, got {other:?}"),
        }
    }

    #[test]
    fn defeq_but_differently_written_type_matches() {
        // `Not a` 与 `a -> False` 文本不同但 definitional equal —— 文本比对
        // 会漏掉它，kernel 判定能识别（REQUIREMENTS §2.8 的意义所在）。
        let prefix = "axiom False : Prop\ndef Not : Prop -> Prop := fun (a : Prop) => a -> False\n";
        let open = spec("Not a", &[("a", Some("Prop")), ("h", Some("a -> False"))]);
        let judgements = judge_terms(prefix, &CompileOptions::default(), &open, &["h"]);
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn unknown_term_reports_elab_error() {
        let open = spec("Prop", &[]);
        let judgements = judge_terms("", &CompileOptions::default(), &open, &["nope"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "elab-unknown-identifier");
                assert!(message.contains("nope"));
            }
            other => panic!("expected elab error, got {other:?}"),
        }
    }

    #[test]
    fn failed_prefix_declaration_keeps_its_name_free() {
        // check-then-add：前缀里被 kernel 拒绝的名字不占用。引用它的候选在
        // pass1 能通过（用的是"幽灵"声明类型），pass2 会重新 elaborate 并
        // 得到真正的 unknown-identifier —— 这正是教学想要的判定语义。
        let prefix = "def broken : Prop -> Type := fun (x : Prop) => x\n";
        let open = spec("Prop -> Type", &[]);
        let judgements = judge_terms(prefix, &CompileOptions::default(), &open, &["broken"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => {
                assert_eq!(code, "elab-unknown-identifier");
            }
            other => panic!("expected unknown identifier, got {other:?}"),
        }
    }

    #[test]
    fn bare_mode_judges_without_prelude() {
        let options = CompileOptions {
            prelude: PreludeMode::Bare,
        };
        let open = spec("Prop -> Prop", &[]);
        let judgements = judge_terms("", &options, &open, &["fun (x : Prop) => x"]);
        assert_eq!(judgements[0], Judgement::Match);
        // Bare 模式下 Nat 不存在。
        let open_nat = spec("Nat", &[]);
        let judgements = judge_terms("", &options, &open_nat, &["2"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => {
                assert_eq!(code, "elab-unknown-identifier");
            }
            other => panic!("expected unknown Nat in bare mode, got {other:?}"),
        }
    }

    #[test]
    fn universe_carrying_open_goal_judges() {
        // 带宇宙参数的声明：剩余目标 α，合成声明必须携带同样的 {u}。
        let prefix =
            "def id {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n";
        let open = OpenGoalSpec {
            universe: vec!["u".to_string()],
            ty: "α".to_string(),
            binders: vec![
                GoalBinderSpec {
                    name: "α".to_string(),
                    ty: Some("Sort u".to_string()),
                },
                GoalBinderSpec {
                    name: "a".to_string(),
                    ty: Some("α".to_string()),
                },
            ],
        };
        let judgements = judge_terms(
            prefix,
            &CompileOptions::default(),
            &open,
            &["a", "id.{u} α a"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert_eq!(judgements[1], Judgement::Match);
    }

    // ---- judge_hole_fill：文档真实命令里的逐洞判定 ----

    /// 取文档里第一个 open 练习的 DeclState（span 与洞位都来自完整流水线）。
    fn open_decl(doc: &str) -> DeclState {
        let report = check_document(&parse(doc).expect("parses"));
        report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open exercise")
            .clone()
    }

    const SUB_HOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
         axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
         theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
         fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro a b ha sorry\n";

    #[test]
    fn hole_fill_accepts_hypothesis_in_a_sub_hole() {
        // 剩一个子洞的 spine 状态：填对假设 ⇒ 整个证明被完整 kernel 接受；
        // 填错 ⇒ 内核给出"期望 / 实际"（类型不合的候选 → Mismatch）。
        let d = open_decl(SUB_HOLE_DOC);
        let judgements = judge_hole_fill(
            SUB_HOLE_DOC,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["hb", "ha"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "type-mismatched candidate must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn hole_fill_renames_examples_by_replacing_the_first_token() {
        // `example` 无名字：首 token `example` 换成 `def _soko_judge_k`。
        let doc = "axiom False : Prop\n\
                   example : (h : False) -> False := fun (h : False) => sorry\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["h", "False"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        match &judgements[1] {
            Judgement::Mismatch { expected, actual } => {
                // 内核渲染：`False.[]` 保留原名，Prop 显示为 Sort(0)。
                assert!(expected.contains("False"), "expected: {expected}");
                assert!(actual.contains("Sort(0)"), "actual: {actual}");
            }
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn hole_fill_keeps_universe_params_from_the_source() {
        // def 的宇宙参数 `{u}` 原样保留在合成命令里，Sort u 目标可判定。
        let doc =
            "def idT {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => a\n\
                   theorem t {u} : {α : Sort u} -> (a : α) -> Eq.{u} α a a := \
                   fun {α : Sort u} => fun (a : α) => sorry\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["Eq.refl.{u} α a"],
        );
        assert_eq!(judgements[0], Judgement::Match);
    }

    #[test]
    fn hole_fill_with_remaining_holes_reports_open_honestly() {
        // 其余洞保持 `sorry` ⇒ 合成声明仍是 open 练习：kernel 没能整体裁决，
        // 结论如实为 Error（绝不把"没判过"说成 Match）。
        let doc = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem t : (a : Prop) -> (b : Prop) -> (ha : a) -> (hb : b) -> And a b := \
                   fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro sorry sorry\n";
        let d = open_decl(doc);
        let judgements = judge_hole_fill(
            doc,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["ha", "hb"],
        );
        assert!(
            judgements.iter().all(
                |j| matches!(j, Judgement::Error { code, .. } if code == "elab-hole-misplaced")
            ),
            "remaining holes keep the fill open: {judgements:?}"
        );
    }

    #[test]
    fn hole_fill_reports_parse_failures_as_errors_not_panics() {
        let d = open_decl(SUB_HOLE_DOC);
        let judgements = judge_hole_fill(
            SUB_HOLE_DOC,
            &CompileOptions::default(),
            d.span,
            d.holes[0],
            &["no(", "hb"],
        );
        match &judgements[0] {
            Judgement::Error { code, .. } => assert_eq!(code, "parse"),
            other => panic!("expected parse error, got {other:?}"),
        }
        assert_eq!(judgements[1], Judgement::Match);
    }

    // ---- judge_value_replace：失败声明的值位整体替换判定 ----

    /// 取文档里第一个 failed 声明的 DeclState（span 来自完整流水线）。
    fn failed_decl(doc: &str) -> DeclState {
        let report = check_document(&parse(doc).expect("parses"));
        report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Failed)
            .expect("failed declaration")
            .clone()
    }

    #[test]
    fn value_replace_accepts_a_kernel_verified_rfl_candidate() {
        // 匿名 example：首 token 换名后整值替换。kernel 接受的 rfl 候选
        // Match，两边不同的候选被内核以 Mismatch 拒绝。
        let doc = "example : Eq.{1} Nat 2 2 := 3\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(
            doc,
            &CompileOptions::default(),
            d.span,
            &["Eq.refl.{1} Nat 2", "Eq.refl.{1} Nat 3"],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "3 ≢ 2 must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn value_replace_keeps_universe_params_and_declared_binders() {
        // def 的宇宙参数 `{u}` 与值位之前的命令头原样保留：候选 lambda
        // 引用 Sort u 必须仍可 elaborate，内核才判得出 Match。
        let doc =
            "def idT {u} : {α : Sort u} -> (a : α) -> α := fun {α : Sort u} => fun (a : α) => 1\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(
            doc,
            &CompileOptions::default(),
            d.span,
            &[
                "fun {α : Sort u} => fun (a : α) => a",
                "fun {α : Sort u} => fun (a : α) => 2",
            ],
        );
        assert_eq!(judgements[0], Judgement::Match);
        assert!(
            matches!(judgements[1], Judgement::Mismatch { .. }),
            "Nat ≢ α must be a kernel mismatch, got {:?}",
            judgements[1]
        );
    }

    #[test]
    fn value_replace_reports_unparseable_candidates_as_errors() {
        let doc = "example : Eq.{1} Nat 2 2 := 3\n";
        let d = failed_decl(doc);
        let judgements =
            judge_value_replace(doc, &CompileOptions::default(), d.span, &["no(", "nope"]);
        match &judgements[0] {
            Judgement::Error { code, message } => {
                assert_eq!(code, "parse");
                assert!(message.contains("no("), "message: {message}");
            }
            other => panic!("expected parse error, got {other:?}"),
        }
        match &judgements[1] {
            Judgement::Error { code, .. } => assert_eq!(code, "elab-unknown-identifier"),
            other => panic!("expected elab error, got {other:?}"),
        }
    }

    #[test]
    fn value_replace_reports_declarations_without_a_value_as_errors() {
        // axiom 没有 `:=` 值位：明确报错，绝不 panic。
        let doc = "axiom bad : undefined_name\n";
        let d = failed_decl(doc);
        let judgements = judge_value_replace(doc, &CompileOptions::default(), d.span, &["Prop"]);
        match &judgements[0] {
            Judgement::Error { code, .. } => assert_eq!(code, "parse"),
            other => panic!("expected parse error, got {other:?}"),
        }
    }
}
