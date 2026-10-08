//! **前端表的可序列化部分**（`known` + `defs`）—— T1-B 批 2 的 **B 块前半**
//! （2026-10-09）。
//!
//! ## 为什么 B 必须自己存（而不是从内核环境反推）
//!
//! 见 PLAN `### 24.` 的结论：`KnownName::Decl` 的 `signature` 是**源级类型文本**
//! （`proof::decl_signature`），`implicit_prefix`/`explicit_arity`/`telescope_arity`
//! 是**源级 AST 走查**（`leading_implicit_prefix`/`explicit_arity`/…）——
//! 三者**只吃源级 AST**（读 `walk.rs` 的登记点可核实 ✓），**不是**内核环境的纯函数。
//! 从内核环境反推等于再造一套实现 ⇒ 一旦边界情形分叉，隐式实参插入就变
//! ⇒ **判定/`--json` 分叉** ✗。⇒ 存下来。
//!
//! ## 两条格式纪律
//!
//! 1. **确定序**：两张表都是 `HashMap`，直接 serde 出来的**顺序不定** ⇒ 同一份表
//!    两次编码可能给出**不同文本** ✗（缓存/对拍都要逐字节 ⇒ 这是硬要求）。
//!    本模块一律按**键排序**成 `Vec<(String, …)>` 再写 ✓。
//! 2. **带格式号**：编码出来的对象里有 `format`；不认的号 ⇒ **当不存在**（调用方
//!    静默回退"本地重编"，与产物侧的 §8.2 同一条纪律 ✓）。
//!
//! ## 这一版**不含** `inductives`
//!
//! `InductiveInfo` 里有 **内核裸指针**（`MatchField.ty: ExprPtr<'a>`）⇒ 它的
//! 序列化必须**按名字**在已装载的内核环境里重解析（不能按地址 ✗）—— 那是
//! B 块的后半，与"消费入口"同批做 ✓。

use serde::{Deserialize, Serialize};

use crate::compile::elab::{DefInfo, DefTable, KnownName, KnownTable};

/// 表的**格式**版本（载荷形状变了就 +1 ⇒ 老产物当不存在 ✓）。
pub const TABLES_FORMAT: u32 = 1;

/// ⚠ `#[allow(dead_code)]` 三处（本文件）：**消费入口接上之前没人调** ——
/// 按 PLAN `### 24.` 的顺序纪律，**先别只写入口**（只写不读是纯开销），
/// 所以 B 的编解码**先定型、先带判据**，等消费入口同批接线时立刻可用 ✓。
/// 三个判据都在本文件的 `#[cfg(test)]` 里跑（测试目标看得见，库目标看不见 ⇒ 需要这条 ✓）。
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct TablesFile {
    format: u32,
    /// **有序**的 `(名字, 表项)`（见模块文档第 1 条纪律 ✓）
    known: Vec<(String, KnownName)>,
    defs: Vec<(String, DefInfo)>,
}

/// 编码两张表 ⇒ 文本（**确定序** ✓）。`known`/`defs` 是 `BTreeMap` 的替代视角，
/// 调用方传 `HashMap` 即可。
#[allow(dead_code)]
pub(crate) fn encode(known: &KnownTable, defs: &DefTable) -> Result<String, String> {
    let mut known_rows: Vec<(String, KnownName)> =
        known.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    known_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut def_rows: Vec<(String, DefInfo)> =
        defs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    def_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let file = TablesFile {
        format: TABLES_FORMAT,
        known: known_rows,
        defs: def_rows,
    };
    serde_json::to_string(&file).map_err(|e| format!("前端表序列化失败：{e}"))
}

/// 解码。**不认的 `format` ⇒ `Err`**（调用方当"没有产物"处理 ✓）。
#[allow(dead_code)]
pub(crate) fn decode(text: &str) -> Result<(KnownTable, DefTable), String> {
    let file: TablesFile =
        serde_json::from_str(text).map_err(|e| format!("前端表反序列化失败：{e}"))?;
    if file.format != TABLES_FORMAT {
        return Err(format!(
            "前端表格式号不认识（{} ≠ {}）",
            file.format, TABLES_FORMAT
        ));
    }
    Ok((
        file.known.into_iter().collect(),
        file.defs.into_iter().collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Binder, BinderKind, Expr};
    use crate::span::Span;

    fn sample_known() -> KnownTable {
        let mut known: KnownTable = Default::default();
        known.insert(
            "Set.subset".to_string(),
            KnownName::Decl {
                universes: vec!["u".to_string(), "v".to_string()],
                implicit_prefix: 2,
                explicit_arity: 2,
                signature: Some("(A : Set α) → (B : Set α) → Prop".to_string()),
            },
        );
        // `Alias` 与 `Ambiguous` 也要覆盖（G-02 的 `mk` 就是 Ambiguous ✓）。
        known.insert(
            "subset".to_string(),
            KnownName::Alias {
                canonical: "Set.subset".to_string(),
            },
        );
        known.insert(
            "mk".to_string(),
            KnownName::Ambiguous {
                candidates: vec!["Prod.mk".to_string(), "Set.mk".to_string()],
            },
        );
        known
    }

    fn sample_defs() -> DefTable {
        let mut defs: DefTable = Default::default();
        defs.insert(
            "Set.subset".to_string(),
            DefInfo {
                params: vec!["A".to_string(), "B".to_string()],
                universes: vec!["u".to_string()],
                implicit_prefix: 1,
                // 体是一段**源级 AST**（序列化的关键：AST 已能 serde ✓）。
                body: Expr::Forall {
                    binders: vec![Binder {
                        name: "x".to_string(),
                        ty: Some(Box::new(Expr::Ident {
                            name: "α".to_string(),
                            span: Span::default(),
                        })),
                        style: BinderKind::Explicit,
                        span: Span::default(),
                    }],
                    body: Box::new(Expr::Ident {
                        name: "Prop".to_string(),
                        span: Span::default(),
                    }),
                    span: Span::default(),
                },
                telescope_arity: 3,
            },
        );
        defs
    }

    /// **正路**：编码 → 解码 ⇒ 两张表**逐项相同** ✓（三种 `KnownName` 与带 AST 的
    /// `DefInfo` 都覆盖）。
    #[test]
    fn tables_round_trip_through_the_codec() {
        let known = sample_known();
        let defs = sample_defs();
        let text = encode(&known, &defs).expect("encode");
        let (known2, defs2) = decode(&text).expect("decode");
        assert_eq!(known2, known, "`known` 往返必须逐项相同");
        assert_eq!(defs2, defs, "`defs` 往返必须逐项相同（含源级 AST 体）");
    }

    /// **确定序**：同一份表，**插入顺序不同** ⇒ 编码文本**逐字节相同** ✓。
    ///
    /// （反过来会出的事：同一份环境两次编码给出不同文本 ⇒ 缓存/对拍再也比不了 ✗。）
    #[test]
    fn encoding_is_deterministic_regardless_of_insertion_order() {
        let known = sample_known();
        let defs = sample_defs();
        let first = encode(&known, &defs).expect("encode");

        // 反序重建两张表（内容相同、插入顺序相反）。
        let mut known_rev: KnownTable = Default::default();
        let mut pairs: Vec<_> = known.iter().collect();
        pairs.reverse();
        for (k, v) in pairs {
            known_rev.insert(k.clone(), v.clone());
        }
        let mut defs_rev: DefTable = Default::default();
        for (k, v) in defs.iter() {
            defs_rev.insert(k.clone(), v.clone());
        }
        assert_eq!(
            encode(&known_rev, &defs_rev).expect("encode rev"),
            first,
            "插入顺序不许影响编码文本（本模块按**键排序**写 ✓）"
        );
    }

    /// **反向验证**：不认的格式号 ⇒ `Err`（调用方据此**当不存在**、静默回退 ✓）；
    /// 另加"文本被截断"这一条（真实下载损坏的样子）。
    #[test]
    fn a_foreign_format_or_truncated_text_is_rejected() {
        let text = encode(&sample_known(), &sample_defs()).expect("encode");
        let bumped = text.replace(&format!("\"format\":{TABLES_FORMAT}"), "\"format\":9999");
        assert_ne!(bumped, text, "补丁没生效 ⇒ 判据会空转 ✗");
        assert!(decode(&bumped).is_err(), "不认的格式号必须被拒 ✓");
        assert!(
            decode(&text[..text.len() / 2]).is_err(),
            "截断的文本必须被拒（不许「猜着用」半个表 ✗）"
        );
    }
}
