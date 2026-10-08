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

use crate::compile::elab::{
    DefInfo, DefTable, InductiveInfo, InductiveTable, KnownName, KnownTable, MatchCtor, MatchField,
};
use sokonanoda::env::Declar;

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
    /// `inductives` 的 **wire 形**（见 [`InductiveInfoWire`] ✓）
    inductives: Vec<(String, InductiveInfoWire)>,
}

/// **归纳表项的 wire 形** —— 与 [`InductiveInfo`] **逐字段同形，只少一样东西**：
/// `MatchField.ty`（内核 arena **裸指针** ✗ 跨进程无意义、也不能序列化）。
///
/// 它**不是"丢信息"**：那份 `ty` 是"构造子望远镜里第 k 个 binder 的类型"，而构造子
/// 声明**就在已装载的内核环境里** ⇒ 装载时用 [`rehydrate_inductives`] 按
/// **构造子名字**重新取一遍即可（取的是**同一个** `kernel_field_binders`，见
/// `compile::elab` ⇒ 同源、不会漂移 ✓）。
///
/// 其余字段（含 `src_ty`/`index_types` 的**源级 AST**）都照存 ✓ —— 源级 AST 能 serde
/// 是上一轮（§25）打的地基。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MatchFieldWire {
    pub name: String,
    pub style: sokonanoda::expr::BinderStyle,
    pub src_ty: Option<crate::ast::Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MatchCtorWire {
    pub name: String,
    pub canonical: String,
    pub fields: Vec<MatchFieldWire>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InductiveInfoWire {
    pub ctors: Vec<MatchCtorWire>,
    pub recursor: String,
    pub rec_universe_arity: usize,
    pub recursive: bool,
    pub num_params: usize,
    pub param_names: Vec<String>,
    pub num_indices: usize,
    pub index_types: Vec<crate::ast::Expr>,
}

/// [`decode`] 的结果。
///
/// ⚠ `#[allow(dead_code)]`：**消费入口接上之前没人读**（同本文件其余三处）——
/// 库目标看不见 `#[cfg(test)]` 里的读，测试目标看得见 ✓。
#[allow(dead_code)]
pub(crate) struct Decoded {
    pub known: KnownTable,
    pub defs: DefTable,
    pub inductives: Vec<(String, InductiveInfoWire)>,
}

/// 编码两张表 ⇒ 文本（**确定序** ✓）。`known`/`defs` 是 `BTreeMap` 的替代视角，
/// 调用方传 `HashMap` 即可。
#[allow(dead_code)]
pub(crate) fn encode(
    known: &KnownTable,
    defs: &DefTable,
    inductives: &InductiveTable<'_>,
) -> Result<String, String> {
    let mut known_rows: Vec<(String, KnownName)> =
        known.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    known_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut def_rows: Vec<(String, DefInfo)> =
        defs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    def_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut ind_rows: Vec<(String, InductiveInfoWire)> = inductives
        .iter()
        .map(|(k, v)| (k.clone(), wire_of(v)))
        .collect();
    ind_rows.sort_by(|a, b| a.0.cmp(&b.0));
    let file = TablesFile {
        format: TABLES_FORMAT,
        known: known_rows,
        defs: def_rows,
        inductives: ind_rows,
    };
    serde_json::to_string(&file).map_err(|e| format!("前端表序列化失败：{e}"))
}

/// 解码。**不认的 `format` ⇒ `Err`**（调用方当"没有产物"处理 ✓）。
#[allow(dead_code)]
pub(crate) fn decode(text: &str) -> Result<Decoded, String> {
    let file: TablesFile =
        serde_json::from_str(text).map_err(|e| format!("前端表反序列化失败：{e}"))?;
    if file.format != TABLES_FORMAT {
        return Err(format!(
            "前端表格式号不认识（{} ≠ {}）",
            file.format, TABLES_FORMAT
        ));
    }
    Ok(Decoded {
        known: file.known.into_iter().collect(),
        defs: file.defs.into_iter().collect(),
        inductives: file.inductives,
    })
}

/// [`InductiveInfo`] ⇒ wire（**只丢** `MatchField.ty` ✓）。
fn wire_of(v: &InductiveInfo<'_>) -> InductiveInfoWire {
    InductiveInfoWire {
        ctors: v
            .ctors
            .iter()
            .map(|c| MatchCtorWire {
                name: c.name.clone(),
                canonical: c.canonical.clone(),
                fields: c
                    .fields
                    .iter()
                    .map(|f| MatchFieldWire {
                        name: f.name.clone(),
                        style: f.style,
                        src_ty: f.src_ty.clone(),
                    })
                    .collect(),
            })
            .collect(),
        recursor: v.recursor.clone(),
        rec_universe_arity: v.rec_universe_arity,
        recursive: v.recursive,
        num_params: v.num_params,
        param_names: v.param_names.clone(),
        num_indices: v.num_indices,
        index_types: v.index_types.clone(),
    }
}

/// **装载时把 `inductives` 补齐**：按构造子的**规范名**在已装载的内核环境里找到
/// 它的声明，走 [`crate::compile::elab::kernel_field_binders`] 重新取出逐层类型
/// （跳过 `num_params` 层参数），填回 `MatchField.ty` ✓。
///
/// `lookup` 由调用方给（"规范名 ⇒ 声明"）—— 前端**不能**把字符串 intern 进一份
/// 已经建好的 `ExportFile`（那要它的 `Dag`），所以这条口子必须是**回调** ✓。
///
/// **失败面**（一律 `Err` ⇒ 调用方当"没有产物"、回退本地重编 ✓）：
/// * 某个构造子不在环境里；
/// * 望远镜层数**少于**存的字段数（说明产物与环境**不匹配** ✗）；
/// * 逐层 `style` 与产物里存的不一致（同上，漂移的第二个信号 ✓）。
#[allow(dead_code)]
pub(crate) fn rehydrate_inductives<'a>(
    wire: &[(String, InductiveInfoWire)],
    lookup: impl Fn(&str) -> Option<Declar<'a>>,
) -> Result<InductiveTable<'a>, String> {
    let mut out: InductiveTable<'a> = Default::default();
    for (ind_name, info) in wire {
        let mut ctors = Vec::with_capacity(info.ctors.len());
        for c in &info.ctors {
            let declar = lookup(&c.canonical)
                .ok_or_else(|| format!("归纳表要求 `{}` 在环境里", c.canonical))?;
            let mut tys = crate::compile::elab::kernel_field_binders(declar.info().ty)
                .into_iter()
                .skip(info.num_params);
            let mut fields = Vec::with_capacity(c.fields.len());
            for f in &c.fields {
                let (binder_style, ty) = tys.next().ok_or_else(|| {
                    format!(
                        "构造子 `{}` 的望远镜层数不够（存的字段数 {} > 环境里能取到的）",
                        c.canonical,
                        c.fields.len()
                    )
                })?;
                if binder_style != f.style {
                    return Err(format!(
                        "构造子 `{}` 的 binder 风格与产物里存的不一致（环境 ≠ 产物）",
                        c.canonical
                    ));
                }
                fields.push(MatchField {
                    name: f.name.clone(),
                    ty,
                    style: binder_style,
                    src_ty: f.src_ty.clone(),
                });
            }
            ctors.push(MatchCtor {
                name: c.name.clone(),
                canonical: c.canonical.clone(),
                fields,
            });
        }
        out.insert(
            ind_name.clone(),
            InductiveInfo {
                ctors,
                recursor: info.recursor.clone(),
                rec_universe_arity: info.rec_universe_arity,
                recursive: info.recursive,
                num_params: info.num_params,
                param_names: info.param_names.clone(),
                num_indices: info.num_indices,
                index_types: info.index_types.clone(),
            },
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Binder, BinderKind, Expr};
    use crate::span::Span;
    use sokonanoda::builder::EnvBuilder;
    use sokonanoda::env::{ConstructorData, DeclarInfo};
    use sokonanoda::expr::BinderStyle;
    use sokonanoda::util::{Config, ExprPtr};
    use stumpalo::Arena;

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

    fn empty_inductives<'a>() -> InductiveTable<'a> {
        Default::default()
    }

    /// **正路**：编码 → 解码 ⇒ 两张表**逐项相同** ✓（三种 `KnownName` 与带 AST 的
    /// `DefInfo` 都覆盖）。
    #[test]
    fn tables_round_trip_through_the_codec() {
        let known = sample_known();
        let defs = sample_defs();
        let text = encode(&known, &defs, &empty_inductives()).expect("encode");
        let decoded = decode(&text).expect("decode");
        assert_eq!(decoded.known, known, "`known` 往返必须逐项相同");
        assert_eq!(
            decoded.defs, defs,
            "`defs` 往返必须逐项相同（含源级 AST 体）"
        );
        assert!(decoded.inductives.is_empty(), "空归纳表往返仍是空");
    }

    /// **确定序**：同一份表，**插入顺序不同** ⇒ 编码文本**逐字节相同** ✓。
    ///
    /// （反过来会出的事：同一份环境两次编码给出不同文本 ⇒ 缓存/对拍再也比不了 ✗。）
    #[test]
    fn encoding_is_deterministic_regardless_of_insertion_order() {
        let known = sample_known();
        let defs = sample_defs();
        let first = encode(&known, &defs, &empty_inductives()).expect("encode");

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
            encode(&known_rev, &defs_rev, &empty_inductives()).expect("encode rev"),
            first,
            "插入顺序不许影响编码文本（本模块按**键排序**写 ✓）"
        );
    }

    /// **反向验证**：不认的格式号 ⇒ `Err`（调用方据此**当不存在**、静默回退 ✓）；
    /// 另加"文本被截断"这一条（真实下载损坏的样子）。
    #[test]
    fn a_foreign_format_or_truncated_text_is_rejected() {
        let text = encode(&sample_known(), &sample_defs(), &empty_inductives()).expect("encode");
        let bumped = text.replace(&format!("\"format\":{TABLES_FORMAT}"), "\"format\":9999");
        assert_ne!(bumped, text, "补丁没生效 ⇒ 判据会空转 ✗");
        assert!(decode(&bumped).is_err(), "不认的格式号必须被拒 ✓");
        assert!(
            decode(&text[..text.len() / 2]).is_err(),
            "截断的文本必须被拒（不许「猜着用」半个表 ✗）"
        );
    }

    /// 造一份**真的**内核环境：一个构造子 `C : (α : Sort 0) → α → α → C`
    /// —— `num_params = 1`、两个字段 ✓（`MatchField.ty` 就是它望远镜里的第 2、3 层）。
    fn env_with_a_ctor<'a>(arena: &'a Arena) -> (Declar<'a>, Vec<ExprPtr<'a>>) {
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let zero = b.zero();
        let sort0 = b.mk_sort(zero);
        let alpha = b.name_from_str("α");
        let c = b.name_from_str("C");
        let x = b.name_from_str("x");
        let y = b.name_from_str("y");
        let empty = b.alloc_levels_slice(&[]);
        let alpha_ty = b.mk_sort(zero);
        // ⚠ `mk_pi` 是**从里往外**造的：望远镜的**最外层最后建**。
        // 目标顺序（外→里）= `α`（参数）· `x` · `y` · `C`
        // ⇒ 先建 `y` 那一层，再套 `x`，最后套 `α` ✓（建反了 binder 顺序就倒过来，
        //   `MatchField.ty` 会错位 —— 这条判据第一版就是这么红的 ✓）。
        let c_const = b.mk_const(c, empty);
        let p_y = b.mk_pi(y, BinderStyle::Implicit, alpha_ty, c_const);
        let p_x = b.mk_pi(x, BinderStyle::Default, alpha_ty, p_y);
        let ty = b.mk_pi(alpha, BinderStyle::Default, sort0, p_x);
        let declar = Declar::Constructor(ConstructorData {
            info: DeclarInfo {
                name: c,
                uparams: empty,
                ty,
            },
            inductive_name: c,
            ctor_idx: 0,
            num_params: 1,
            num_fields: 2,
        });
        b.add_declar(declar.clone()).expect("ctor must be accepted");
        let binders: Vec<ExprPtr<'a>> = crate::compile::elab::kernel_field_binders(ty)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        (declar, binders)
    }

    /// ⭐ **B 块后半的判据**：`inductives` 的 **wire 往返 + 装载时重解析** ——
    /// 重解析出来的 `MatchField.ty` 必须与**原来那份（同一个 arena 的）指针**
    /// **逐位相同** ✓。
    ///
    /// 为什么这条是 B 的关键：`MatchField.ty` 是**内核裸指针**（不可序列化 ✗），
    /// 所以 wire 里**故意不存它**；能不能"按构造子名字 + 望远镜第 k 层"重新取回
    /// **同一批**类型，就是整个 B 能不能跨进程复用的分水岭 ✓。
    ///
    /// **反向验证**（同用例内）：把 `num_params` 故意写大 1 ⇒ 层数不够 ⇒
    /// **必须 `Err`** ✓（否则会静默给出错的字段类型 ✗）。
    #[test]
    fn inductives_wire_round_trips_and_rehydrates_to_the_same_types() {
        let arena = Arena::new();
        let (declar, binders) = env_with_a_ctor(&arena);
        // 望远镜 = (α : Sort 0)（参数）· (x : α) · (y : α)
        assert_eq!(binders.len(), 3, "夹具形状：1 参数 + 2 字段");

        let mut inductives: InductiveTable<'_> = Default::default();
        inductives.insert(
            "C".to_string(),
            InductiveInfo {
                ctors: vec![MatchCtor {
                    name: "C".to_string(),
                    canonical: "C".to_string(),
                    fields: vec![
                        MatchField {
                            name: "x".to_string(),
                            ty: binders[1],
                            style: BinderStyle::Default,
                            src_ty: None,
                        },
                        MatchField {
                            name: "y".to_string(),
                            ty: binders[2],
                            style: BinderStyle::Implicit,
                            src_ty: None,
                        },
                    ],
                }],
                recursor: "C.rec".to_string(),
                rec_universe_arity: 0,
                recursive: false,
                num_params: 1,
                param_names: vec!["α".to_string()],
                num_indices: 0,
                index_types: vec![],
            },
        );

        let text = encode(&sample_known(), &sample_defs(), &inductives).expect("encode");
        let decoded = decode(&text).expect("decode");
        assert_eq!(decoded.inductives.len(), 1, "归纳表必须整体过一遍 wire");

        // 装载侧只拿得到"规范名 ⇒ 声明"这一条口子（真实调用方也是如此 ✓）。
        let lookup = |name: &str| (name == "C").then(|| declar.clone());
        let back = rehydrate_inductives(&decoded.inductives, lookup).expect("rehydrate");
        let c = &back["C"].ctors[0];
        assert_eq!(c.name, "C");
        assert_eq!(c.canonical, "C");
        assert_eq!(c.fields.len(), 2);
        assert_eq!(
            c.fields[0].ty, binders[1],
            "字段 0 的类型必须**指针同一** ✓"
        );
        assert_eq!(
            c.fields[1].ty, binders[2],
            "字段 1 的类型必须**指针同一** ✓"
        );
        assert_eq!(c.fields[0].style, BinderStyle::Default);
        assert_eq!(c.fields[1].style, BinderStyle::Implicit);
        assert_eq!(back["C"].recursor, "C.rec");
        assert_eq!(back["C"].num_params, 1);

        // **反向验证**：`num_params` 写错（差 1）⇒ 层数不够 ⇒ 必须 `Err` ✓。
        let mut wrong = decoded.inductives.clone();
        wrong[0].1.num_params += 1;
        assert!(
            rehydrate_inductives(&wrong, |name: &str| (name == "C").then(|| declar.clone()))
                .is_err(),
            "`num_params` 写错必须判红（不许静默给出错的字段类型 ✗）"
        );
        // **反向验证（第二条牙）**：规范名不在环境里 ⇒ 必须 `Err` ✓。
        assert!(
            rehydrate_inductives(&decoded.inductives, |_: &str| None).is_err(),
            "构造子不在环境里必须判红 ✓"
        );
    }
}
