//! **NDJSON writer**（T1-B 批 1 的 writer 半边，2026-10-09）。
//!
//! 把一份 [`ExportFile`]（内核内存环境）写成 **Lean 导出格式**的文本 —— 与
//! `crate::parser`（读侧）**同一个格式**（serde 的 `Serialize`/`Deserialize` 是
//! 同一套派生 ⇒ 形状**构造性一致** ✓，不是"照文档手抄" ✗）。
//!
//! ## 为什么需要它（设计 `docs/design/module-artifacts.md` §2.4 T1-B 第 1 件）
//!
//! 磁盘模块产物要能被**另一个进程**读回来 ⇒ 必须把 arena 里的**指针图**变成
//! **结构文本**（arena 是 bump 分配器，裸地址跨进程无意义 ✗）。读侧
//! （`parser.rs`）早就有；本模块补写侧。
//!
//! ## 格式（从读侧反推 · 仓库里没有 ndjson 样例）
//!
//! 每行一个 JSON 对象（`ExportJsonObject`）：
//! * **名字**（`in` 下标）：`{"str":{"pre":<父名下标>,"str":"…"},"in":k}`
//!   （`num` 同理）。⚠ 下标 **0 恒为 `Anon`**（读侧预置 ✓）⇒ 本模块**不写 0 号名**。
//! * **层级**（`il` 下标）：`succ`/`max`/`imax`/`param`。⚠ 下标 **0 恒为 `Zero`** ✓。
//! * **项**（`ie` 下标）：`sort`/`bvar`/`const`/`app`/`forallE`/`lam`/`letE`/`proj`/
//!   `natVal`/`strVal`。下标从 **0** 起。
//! * **声明**（无下标）：`axiom`/`thm`/`def`/`opaque`/`quot`/`inductive`。
//! * 首行必须是 `{"meta":{…}}`（读侧的 semver 闸门读它 ✓）。
//!
//! **顺序**：所有**名字**行 → 所有**层级**行 → 所有**项**行 → 所有**声明**行。
//! 因为引用是下标、而读侧按行**增量**填表（`get_name_ptr(idx)` 要求 idx 已填 ✓）
//! ⇒ 被引用者必须先出行 ⇒ 本模块按**依赖序**（名先父后子 · 层与项先子后父）分配下标 ✓。
//!
//! ## 覆盖范围（增量落地）
//!
//! 本版覆盖 **非归纳**声明（`axiom`/`thm`/`def`/`opaque`/`quot`）与它们用到的
//! 名字/层级/项（`sort`/`bvar`/`const`/`app`/`forallE`/`lam`/`letE`/`proj`/
//! `natVal`/`strVal`）。**归纳块（`inductive`/`ctor`/`recursor`）下一版** ——
//! 遇到就返回 `Err`（调用方**静默回退**到"本地重编"，不产生半个产物 ✓）。

use crate::env::{ConstructorData, Declar, DeclarInfo, InductiveData, RecursorData};
use crate::expr::Expr;
use crate::level::Level;
use crate::name::Name;
use crate::parser::{
    BackRef, Constructor, DefinitionSafety, ExportJsonObject, ExportJsonVal, ExporterMeta, FileMeta,
    FormatMeta, IndInfo, LeanMeta, QuotKind, Recursor, RecursorRule,
};
use crate::util::{ExportFile, ExprPtr, FxHashMap, LevelPtr, LevelsPtr, NamePtr};
use std::borrow::Cow;

/// 导出格式版本（读侧 `check_semver` 的窗口 = `[3.1.0, 3.2.0)` ✓）。
const EXPORT_FORMAT_VERSION: &str = "3.1.0";

/// 把一份环境写成 NDJSON 文本。**纯函数、不改环境** ✓。
pub(crate) fn write_export_file(file: &ExportFile<'_>) -> Result<String, String> {
    let mut w = Writer::new();
    w.collect(file)?;
    Ok(w.finish())
}

struct Writer<'a> {
    names: FxHashMap<NamePtr<'a>, u32>,
    levels: FxHashMap<LevelPtr<'a>, u32>,
    exprs: FxHashMap<ExprPtr<'a>, u32>,
    name_lines: Vec<String>,
    level_lines: Vec<String>,
    expr_lines: Vec<String>,
    decl_lines: Vec<String>,
    /// 下一个可用名字下标 —— **从 1 起**（0 = `Anon`，读侧预置 ✓）。
    next_name: u32,
    /// 下一个可用层级下标 —— **从 1 起**（0 = `Zero`，读侧预置 ✓）。
    next_level: u32,
    next_expr: u32,
}

fn line(val: ExportJsonVal<'_>, i: Option<BackRef>) -> Result<String, String> {
    serde_json::to_string(&ExportJsonObject { val, i }).map_err(|e| format!("ndjson 序列化失败：{e}"))
}

impl<'a> Writer<'a> {
    fn new() -> Self {
        Self {
            names: FxHashMap::default(),
            levels: FxHashMap::default(),
            exprs: FxHashMap::default(),
            name_lines: Vec::new(),
            level_lines: Vec::new(),
            expr_lines: Vec::new(),
            decl_lines: Vec::new(),
            next_name: 1,
            next_level: 1,
            next_expr: 0,
        }
    }

    /// 名字下标（**先父后子**：`pre` 必须先出行 ✓）。`Anon` ⇒ **0**（读侧预置，不写）。
    fn name(&mut self, p: NamePtr<'a>) -> Result<u32, String> {
        if let Some(&i) = self.names.get(&p) {
            return Ok(i);
        }
        let idx = match p.as_ref().kind {
            Name::Anon => 0,
            Name::Str(pre, s, _) => {
                let pre_i = self.name(pre)?;
                let text: String = (**s.as_ref()).to_string();
                let i = self.next_name;
                self.next_name += 1;
                self.name_lines.push(line(
                    ExportJsonVal::NameStr {
                        pre: pre_i,
                        str: Cow::Owned(text),
                    },
                    Some(BackRef::In(i)),
                )?);
                i
            }
            Name::Num(pre, n, _) => {
                let pre_i = self.name(pre)?;
                let i = self.next_name;
                self.next_name += 1;
                self.name_lines.push(line(
                    ExportJsonVal::NameNum {
                        pre: pre_i,
                        i: u32::try_from(n).map_err(|_| "名字的数值后缀超出 u32".to_string())?,
                    },
                    Some(BackRef::In(i)),
                )?);
                i
            }
        };
        self.names.insert(p, idx);
        Ok(idx)
    }

    /// 层级下标（**先子后父**）。`Zero` ⇒ **0**（读侧预置，不写）。
    fn level(&mut self, p: LevelPtr<'a>) -> Result<u32, String> {
        if let Some(&i) = self.levels.get(&p) {
            return Ok(i);
        }
        let idx = match p.as_ref() {
            Level::Zero => 0,
            Level::Succ(l, _) => {
                let li = self.level(*l)?;
                let i = self.next_level;
                self.next_level += 1;
                self.level_lines
                    .push(line(ExportJsonVal::LevelSucc(li), Some(BackRef::Il(i)))?);
                i
            }
            Level::Max(l, r, _) => {
                let (li, ri) = (self.level(*l)?, self.level(*r)?);
                let i = self.next_level;
                self.next_level += 1;
                self.level_lines
                    .push(line(ExportJsonVal::LevelMax([li, ri]), Some(BackRef::Il(i)))?);
                i
            }
            Level::IMax(l, r, _) => {
                let (li, ri) = (self.level(*l)?, self.level(*r)?);
                let i = self.next_level;
                self.next_level += 1;
                self.level_lines
                    .push(line(ExportJsonVal::LevelIMax([li, ri]), Some(BackRef::Il(i)))?);
                i
            }
            Level::Param(n, _) => {
                let ni = self.name(*n)?;
                let i = self.next_level;
                self.next_level += 1;
                self.level_lines
                    .push(line(ExportJsonVal::LevelParam(ni), Some(BackRef::Il(i)))?);
                i
            }
            // 层元变量**不许进环境**（`level_exit` 在 `add_declar` 之前消掉它 ✓）
            // ⇒ 出现就是"产物不该有它"⇒ 静默回退（不写半个产物）✓。
            Level::MVar(..) => return Err("层级里出现层元变量（不该进环境）".to_string()),
        };
        self.levels.insert(p, idx);
        Ok(idx)
    }

    fn levels(&mut self, ps: LevelsPtr<'a>) -> Result<Vec<u32>, String> {
        ps.iter().map(|p| self.level(*p)).collect()
    }

    /// 项下标（**先子后父**）。
    fn expr(&mut self, p: ExprPtr<'a>) -> Result<u32, String> {
        if let Some(&i) = self.exprs.get(&p) {
            return Ok(i);
        }
        let val = match p.as_ref() {
            Expr::Var { dbj_idx, .. } => ExportJsonVal::ExprBVar(*dbj_idx),
            Expr::Sort { level, .. } => ExportJsonVal::ExprSort(self.level(*level)?),
            Expr::Const { name, levels, .. } => ExportJsonVal::ExprConst {
                name: self.name(*name)?,
                levels: self.levels(*levels)?,
            },
            Expr::App { fun, arg, .. } => ExportJsonVal::ExprApp {
                fun: self.expr(*fun)?,
                arg: self.expr(*arg)?,
            },
            Expr::Pi {
                binder_name,
                binder_style,
                binder_type,
                body,
                ..
            } => ExportJsonVal::ExprPi {
                binder_name: self.name(*binder_name)?,
                binder_type: self.expr(*binder_type)?,
                body: self.expr(*body)?,
                binder_info: *binder_style,
            },
            Expr::Lambda {
                binder_name,
                binder_style,
                binder_type,
                body,
                ..
            } => ExportJsonVal::ExprLambda {
                binder_name: self.name(*binder_name)?,
                binder_type: self.expr(*binder_type)?,
                body: self.expr(*body)?,
                binder_info: *binder_style,
            },
            Expr::Let { data, .. } => ExportJsonVal::ExprLet {
                name: self.name(data.binder_name)?,
                ty: self.expr(data.binder_type)?,
                value: self.expr(data.val)?,
                body: self.expr(data.body)?,
                nondep: data.nondep,
            },
            Expr::Proj {
                ty_name,
                idx,
                structure,
                ..
            } => ExportJsonVal::ExprProj {
                type_name: self.name(*ty_name)?,
                idx: usize::from(*idx),
                structure: self.expr(*structure)?,
            },
            Expr::NatLit { ptr, .. } => {
                ExportJsonVal::NatLit(ptr.as_ref().clone())
            }
            Expr::StringLit { ptr, .. } => {
                ExportJsonVal::StrLit(Cow::Owned((**ptr.as_ref()).to_string()))
            }
            // 占位符（`Expr::Meta`）**不许进环境**（`add_declar` 入口硬拒 ✓）⇒ 同理回退。
            Expr::Meta { .. } => return Err("项里出现占位符（不该进环境）".to_string()),
        };
        let i = self.next_expr;
        self.next_expr += 1;
        self.expr_lines.push(line(
            val,
            Some(BackRef::Ie(i)),
        )?);
        self.exprs.insert(p, i);
        Ok(i)
    }

    /// `levelParams` = **名字下标**列表（读侧 `get_uparams_ptr` 把它们变成 `Level::Param` ✓）。
    fn uparams(&mut self, info: &DeclarInfo<'a>) -> Result<Vec<u32>, String> {
        let mut out = Vec::new();
        for p in info.uparams.iter() {
            match p.as_ref() {
                Level::Param(n, _) => out.push(self.name(*n)?),
                _ => return Err("`levelParams` 里出现了不是 `Param` 的层级".to_string()),
            }
        }
        Ok(out)
    }

    fn collect(&mut self, file: &ExportFile<'a>) -> Result<(), String> {
        // 归纳块在 `declars` 里是**连续**的一段（ind… · ctor… · rec…），区间记账在
        // `mutual_block_sizes`（`begin/end_inductive_block` 填的，前端每个块都调 ✓）。
        let all: Vec<&Declar<'a>> = file.declars.values().collect();
        let mut handled = vec![false; all.len()];
        for i in 0..all.len() {
            if handled[i] {
                continue;
            }
            match all[i] {
                Declar::Inductive(ind) => {
                    let (start, size) = *file
                        .mutual_block_sizes
                        .get(&ind.info.name)
                        .ok_or_else(|| "归纳块缺少 `mutual_block_sizes` 记账".to_string())?;
                    if start != i || size == 0 || start + size > all.len() {
                        return Err("归纳块的记账区间不自洽".to_string());
                    }
                    for k in start..start + size {
                        handled[k] = true;
                    }
                    self.inductive_block(&all[start..start + size])?;
                }
                other => self.single_declar(other)?,
            }
        }
        Ok(())
    }

    /// 一个**归纳块** ⇒ **一行** `inductive`（读侧 `Inductive { … }` 就是这个形状 ✓）。
    fn inductive_block(&mut self, slice: &[&Declar<'a>]) -> Result<(), String> {
        let mut ind_vals = Vec::new();
        let mut ctor_vals = Vec::new();
        let mut rec_vals = Vec::new();
        for d in slice {
            match d {
                Declar::Inductive(x) => ind_vals.push(self.ind_info(x)?),
                Declar::Constructor(x) => ctor_vals.push(self.ctor_info(x)?),
                Declar::Recursor(x) => rec_vals.push(self.rec_info(x)?),
                _ => return Err("归纳块里混进了别的声明".to_string()),
            }
        }
        if ind_vals.is_empty() {
            return Err("归纳块里没有归纳类型".to_string());
        }
        self.decl_lines.push(line(
            ExportJsonVal::Inductive {
                ind_vals,
                ctor_vals,
                rec_vals,
            },
            None,
        )?);
        Ok(())
    }

    fn ind_info(&mut self, x: &InductiveData<'a>) -> Result<IndInfo, String> {
        let all = x
            .all_ind_names
            .iter()
            .map(|n| self.name(*n))
            .collect::<Result<Vec<u32>, String>>()?;
        let ctors = x
            .all_ctor_names
            .iter()
            .map(|n| self.name(*n))
            .collect::<Result<Vec<u32>, String>>()?;
        Ok(IndInfo {
            name: self.name(x.info.name)?,
            uparams: self.uparams(&x.info)?,
            ty: self.expr(x.info.ty)?,
            all,
            ctors,
            is_rec: x.is_recursive,
            // ⚠ 读侧**忽略**这一位（`IndInfo { .. , .. }` 里是 `..` ✓）⇒ 写 `false`
            // 不会丢语义；但**必须写一个确定值**，否则往返不幂等 ✗。
            is_reflexive: false,
            num_indices: x.num_indices,
            num_nested: u16::from(x.is_nested),
            num_params: x.num_params,
            is_unsafe: false,
        })
    }

    fn ctor_info(&mut self, x: &ConstructorData<'a>) -> Result<Constructor, String> {
        Ok(Constructor {
            name: self.name(x.info.name)?,
            uparams: self.uparams(&x.info)?,
            ty: self.expr(x.info.ty)?,
            is_unsafe: false,
            cidx: x.ctor_idx,
            num_params: x.num_params,
            num_fields: x.num_fields,
            induct: self.name(x.inductive_name)?,
        })
    }

    fn rec_info(&mut self, x: &RecursorData<'a>) -> Result<Recursor, String> {
        let mut rules = Vec::new();
        for r in x.rec_rules.iter() {
            rules.push(RecursorRule {
                ctor: self.name(r.ctor_name)?,
                nfields: r.ctor_telescope_size_wo_params,
                rhs: self.expr(r.val)?,
            });
        }
        let all = x
            .all_inductives
            .iter()
            .map(|n| self.name(*n))
            .collect::<Result<Vec<u32>, String>>()?;
        Ok(Recursor {
            name: self.name(x.info.name)?,
            uparams: self.uparams(&x.info)?,
            ty: self.expr(x.info.ty)?,
            is_unsafe: false,
            num_params: x.num_params,
            num_indices: x.num_indices,
            num_motives: x.num_motives,
            num_minors: x.num_minors,
            rules,
            all,
            k: x.is_k,
        })
    }

    /// 非归纳声明 ⇒ 一行。
    fn single_declar(&mut self, declar: &Declar<'a>) -> Result<(), String> {
        {
            match declar {
                Declar::Axiom { info } => {
                    let (n, us, ty) =
                        (self.name(info.name)?, self.uparams(info)?, self.expr(info.ty)?);
                    self.decl_lines.push(line(
                        ExportJsonVal::Axiom {
                            name: n,
                            uparams: us,
                            ty,
                            is_unsafe: false,
                        },
                        None,
                    )?);
                }
                Declar::Theorem { info, val } => {
                    let (n, us, ty, v) = (
                        self.name(info.name)?,
                        self.uparams(info)?,
                        self.expr(info.ty)?,
                        self.expr(*val)?,
                    );
                    self.decl_lines.push(line(
                        ExportJsonVal::Thm {
                            name: n,
                            uparams: us,
                            ty,
                            value: v,
                        },
                        None,
                    )?);
                }
                Declar::Definition { info, val, hint } => {
                    let (n, us, ty, v) = (
                        self.name(info.name)?,
                        self.uparams(info)?,
                        self.expr(info.ty)?,
                        self.expr(*val)?,
                    );
                    self.decl_lines.push(line(
                        ExportJsonVal::Defn {
                            name: n,
                            uparams: us,
                            ty,
                            value: v,
                            hint: *hint,
                            safety: DefinitionSafety::Safe,
                        },
                        None,
                    )?);
                }
                Declar::Opaque { info, val } => {
                    let (n, us, ty, v) = (
                        self.name(info.name)?,
                        self.uparams(info)?,
                        self.expr(info.ty)?,
                        self.expr(*val)?,
                    );
                    self.decl_lines.push(line(
                        ExportJsonVal::Opaque {
                            name: n,
                            uparams: us,
                            ty,
                            value: v,
                            is_unsafe: false,
                        },
                        None,
                    )?);
                }
                Declar::Quot { info } => {
                    let (n, us, ty) =
                        (self.name(info.name)?, self.uparams(info)?, self.expr(info.ty)?);
                    self.decl_lines.push(line(
                        ExportJsonVal::Quot {
                            name: n,
                            uparams: us,
                            ty,
                            kind: QuotKind::Ty,
                        },
                        None,
                    )?);
                }
                // 归纳块走 `inductive_block`（上面）⇒ 到这里说明记账坏了。
                Declar::Inductive(_) | Declar::Constructor(_) | Declar::Recursor(_) => {
                    return Err("归纳块没走成块路径（`mutual_block_sizes` 记账缺失）".to_string())
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> String {
        let meta = ExportJsonVal::Metadata(FileMeta {
            lean: LeanMeta {
                version: Cow::Borrowed("4.0.0"),
                githash: Cow::Borrowed("sokonanoda"),
            },
            exporter: ExporterMeta {
                name: Cow::Borrowed("sokonanoda"),
                version: Cow::Borrowed(env!("CARGO_PKG_VERSION")),
            },
            format: FormatMeta {
                version: Cow::Borrowed(EXPORT_FORMAT_VERSION),
            },
        });
        let mut out = String::new();
        out.push_str(&line(meta, None).expect("meta 一定能序列化"));
        out.push('\n');
        for l in self
            .name_lines
            .iter()
            .chain(self.level_lines.iter())
            .chain(self.expr_lines.iter())
            .chain(self.decl_lines.iter())
        {
            out.push_str(l);
            out.push('\n');
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::EnvBuilder;
    use crate::env::{DeclarInfo, ReducibilityHint};
    use crate::expr::BinderStyle;
    use crate::util::Config;
    use stumpalo::Arena;

    fn axiom<'a>(b: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>) -> NamePtr<'a> {
        let n = b.name_from_str(name);
        let us = b.alloc_levels_slice(&[]);
        b.add_declar(Declar::Axiom {
            info: DeclarInfo { name: n, uparams: us, ty },
        })
        .expect("axiom must be accepted");
        n
    }

    /// **T1-B 批 1 的往返判据**（2026-10-09）：`ExportFile → 文本 → Parser → ExportFile'`
    /// 之后，**再写一遍**必须与第一遍**逐字节相同**（幂等 ✓），且 `ExportFile'` 真的
    /// **过得了内核**（`check_all_declars` ✓）。
    ///
    /// ## 为什么判据是"文本幂等"而不是"指针同一"
    ///
    /// 读侧在**另一份 arena** 里重建 ⇒ `ExprPtr` 地址**必然不同** ✗ ⇒ 跨 arena 比指针
    /// 没有意义。真正要证的是"**结构**一字不差"：写一遍、读回来、再写一遍 —— 两份文本
    /// 相同就说明往返没有丢信息、也没有多造节点 ✓（`--json` 层面的判据在 front 侧，
    /// 见 T1-B 批 2；本判据是**内核层**的第一道 ✓）。
    ///
    /// **反向验证**：把"写第二遍"换成"写一个空环境" ⇒ 文本必不同 ✓（有牙 ——
    /// 见同文件 `round_trip_guard_bites_on_a_different_environment`）。
    #[test]
    fn export_file_round_trips_through_ndjson() {
        let arena = Arena::new();
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        // `axiom P : Prop`
        let zero = b.zero();
        let prop = b.mk_sort(zero);
        let p = axiom(&mut b, "P", prop);
        // `axiom a : P`
        let empty = b.alloc_levels_slice(&[]);
        let p_const = b.mk_const(p, empty);
        let a = axiom(&mut b, "a", p_const);
        // `theorem t : P := a`（⚠ 值位是 `Const a`，不是 `Const P` —— 写错会以
        // `def_eq mismatch expected: P | actual: Sort(0)` 的形式在本判据的
        // `check_all_declars` 那一步露头 ✓）
        let a_const = b.mk_const(a, empty);
        let t = b.name_from_str("t");
        let us = b.alloc_levels_slice(&[]);
        b.add_declar(Declar::Theorem {
            info: DeclarInfo { name: t, uparams: us, ty: p_const },
            val: a_const,
        })
        .expect("theorem must be accepted");
        // `def id (x : P) : P := x` —— 覆盖 `Pi`/`Lambda`/`Var`/`Const` 四条写路径。
        let idn = b.name_from_str("id");
        let x = b.name_from_str("x");
        let pi = b.mk_pi(x, BinderStyle::Default, p_const, p_const);
        let var = b.mk_var(0);
        let lam = b.mk_lambda(x, BinderStyle::Default, p_const, var);
        let us2 = b.alloc_levels_slice(&[]);
        b.add_declar(Declar::Definition {
            info: DeclarInfo { name: idn, uparams: us2, ty: pi },
            val: lam,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("definition must be accepted");

        let file = b.finish();
        let text = file.to_ndjson().expect("writer 必须能写这份环境");

        // ── 读回来（另一份 arena）──
        let arena2 = Arena::new();
        let config = Config {
            unsafe_permit_all_axioms: true,
            unpermitted_axiom_hard_error: false,
            ..Config::default()
        };
        let (file2, skipped) =
            crate::parser::parse_export_mapped(arena2.as_arena_ref(), text.as_bytes(), config)
                .expect("写出来的文本必须能被读侧解析");
        assert!(skipped.is_empty(), "不该有被跳过的声明：{skipped:?}");
        assert_eq!(
            file2.declars.len(),
            file.declars.len(),
            "往返必须一个声明都不少、也不多"
        );

        // ── ① 结构幂等：再写一遍，逐字节相同 ──
        let text2 = file2.to_ndjson().expect("读回来的环境也要能写");
        assert_eq!(text, text2, "往返必须**逐字节**幂等（结构一字不差）");

        // ── ② 读回来的环境真的过得了内核 ──
        file2.check_all_declars();
    }

    /// **反向验证**：判据①的牙 —— 换一份**不同**的环境写出来 ⇒ 文本**必不同** ✓。
    #[test]
    fn round_trip_guard_bites_on_a_different_environment() {
        let arena = Arena::new();
        let mut b1 = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let zero = b1.zero();
        let prop = b1.mk_sort(zero);
        axiom(&mut b1, "P", prop);
        let text1 = b1.finish().to_ndjson().expect("write 1");

        let arena2 = Arena::new();
        let mut b2 = EnvBuilder::new(arena2.as_arena_ref(), Config::default());
        let zero2 = b2.zero();
        let prop2 = b2.mk_sort(zero2);
        axiom(&mut b2, "Q", prop2);
        let text2 = b2.finish().to_ndjson().expect("write 2");
        assert_ne!(text1, text2, "不同环境必须写出**不同**文本 ⇒ 判据①有牙 ✓");
    }
}
