//! AST → 内核表达式的 elaborate、声明构建（build_*）与 hover 记录。

use super::check::quiet_catch;
use super::error::{CompileError, ErrorKind};
use super::prelude::CompileOptions;
use super::report::ResolvedTarget;
use super::scope::NamespaceScope;
use crate::ast::{MatchArm, Pattern};
use crate::judge::{judge_infer, GoalBinderSpec};
use crate::proof::render_expr;
use crate::{Binder, BinderKind, CtorDecl, Expr, IotaRule, NotationAssoc, RecDecl, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{
    ConstructorData, Declar, DeclarInfo, RecRule, RecursorData, ReducibilityHint,
};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::{ExprPtr, LevelPtr, LevelsPtr, NamePtr};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(crate) type UnivMap<'a> = HashMap<String, LevelPtr<'a>>;

/// One constructor field's elaborated type, for building a `match` minor.
#[derive(Debug, Clone)]
pub(crate) struct MatchField<'a> {
    /// The constructor's declared field name (used to rename it to the user's
    /// `match` binder when a later field type references it).
    pub name: String,
    pub ty: ExprPtr<'a>,
    pub style: BinderStyle,
    /// Source type (for binder hover / judge-inference scope).
    pub src_ty: Option<Expr>,
}

/// One constructor of a source-declared inductive, in declaration order.
#[derive(Debug, Clone)]
pub(crate) struct MatchCtor<'a> {
    /// The name as written in the source (`prod_mk`) — `match` arms match on
    /// this spelling (R3, source-level).
    pub name: String,
    /// The installed kernel name (`Prod.prod_mk`, R1) — recursor rules and
    /// hover/goto use it.
    pub canonical: String,
    pub fields: Vec<MatchField<'a>>,
}

/// What a name in the `known` table resolves to.
///
/// `Decl` is a real declaration (or a prelude name): its canonical spelling is
/// the key itself. `Alias` is the **subset extension** (R2): a bare constructor
/// name that is unique in the closure resolves to its canonical name — it must
/// never become a second kernel constant. `Ambiguous` is a bare name two
/// constructors claim (G-02's `mk`): it does not resolve at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KnownName {
    Decl {
        universes: Vec<String>,
        /// **IA-1 的签名表**（设计 `docs/design/implicit-arguments.md` §3.2）：
        /// 声明类型望远镜的**前导隐式 binder 个数**（`0` = 今天的行为）。
        /// 纯源级 AST 走查（[`leading_implicit_prefix`]），**零内核调用**——
        /// 插入路径靠它做**免费闸门**：为 0 就一行都不跑。
        implicit_prefix: usize,
        /// 签名的**显式实参层数**（[`explicit_arity`]）。`try_implicit_application`
        /// 用它做**第二个免费闸门**：实参个数 > 显式层数 ⇒ 调用点是"写全参数"
        /// 的旧写法 ⇒ 直接走老路，**不做判定**。
        explicit_arity: usize,
        /// 声明的**源级类型文本**（`render_expr(ty)`）。应用路径的望远镜从它
        /// 解析，而不是从 `judge_infer` 拿 pp 文本——两处原因：
        /// ① **不递归**：`judge_infer` 会重编译前缀（含 prelude 安装），而
        ///    prelude 自身的部分应用会再次触发插入 ⇒ 无限递归（实测栈溢出）；
        /// ② **看得见前导参数**：pp 会省略嵌套常量的隐式实参
        ///    （`Eq.symm` 的 `h : Eq a b` 里 `α` 不见了 ⇒ 反解不出来），
        ///    源文本写的是 `Eq.{u} α a b` ⇒ 解得出来。
        /// `None` = 没有源文本（运行时构造的 `Nat.add`）⇒ 不做插入。
        signature: Option<String>,
    },
    Alias {
        canonical: String,
    },
    Ambiguous {
        candidates: Vec<String>,
    },
}

impl KnownName {
    /// The universes a real declaration was installed with (aliases carry the
    /// canonical declaration's own arity, so they elab the same way).
    pub(crate) fn universes(&self) -> &[String] {
        match self {
            KnownName::Decl { universes, .. } => universes,
            KnownName::Alias { .. } | KnownName::Ambiguous { .. } => &[],
        }
    }

    /// IA-1：这个声明的**前导隐式 binder 个数**（非声明 = 0）。
    pub(crate) fn implicit_prefix(&self) -> usize {
        match self {
            KnownName::Decl {
                implicit_prefix, ..
            } => *implicit_prefix,
            KnownName::Alias { .. } | KnownName::Ambiguous { .. } => 0,
        }
    }

    /// 这个声明的**源级类型文本**（非声明 = `None`）。
    pub(crate) fn signature(&self) -> Option<&str> {
        match self {
            KnownName::Decl { signature, .. } => signature.as_deref(),
            KnownName::Alias { .. } | KnownName::Ambiguous { .. } => None,
        }
    }

    /// 这个声明的**显式实参层数**（非声明 = 0）。曾是应用路径的免费闸门；
    /// 现在签名直接存在表里，「实参 == 全部层数 ⇒ 一次装完」分支取代了它，
    /// 保留字段供诊断/后续使用。
    #[allow(dead_code)]
    pub(crate) fn explicit_arity(&self) -> usize {
        match self {
            KnownName::Decl { explicit_arity, .. } => *explicit_arity,
            KnownName::Alias { .. } | KnownName::Ambiguous { .. } => 0,
        }
    }
}

/// 声明类型望远镜的**前导隐式 binder 个数**：`{a} {b} (c : T) -> …` ⇒ `2`。
///
/// 纯源级 AST 走查，**零内核调用**（IA-1 的签名表；设计 §3.2 的落点）。
/// `peel_pi` 把 `BinderKind` 丢了，所以这里自己走 `Expr::Forall`。
pub(crate) fn leading_implicit_prefix(ty: &Expr) -> usize {
    let mut n = 0;
    let mut cur = ty;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } => {
                let mut all_implicit = true;
                for b in binders {
                    if b.style == BinderKind::Implicit {
                        n += 1;
                    } else {
                        all_implicit = false;
                        break;
                    }
                }
                if !all_implicit {
                    return n;
                }
                cur = body;
            }
            _ => return n,
        }
    }
}

thread_local! {
    /// prelude 安装期间 > 0：此时**禁止**隐式实参插入（见 [`PreludeInstallGuard`]）。
    static PRELUDE_INSTALL_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// RAII：prelude 安装期间关闭隐式实参插入。
///
/// **为什么必须有**：插入路径要先问内核「这个头的签名是什么」（`judge_infer`），
/// 而 `judge_infer` 会**重新编译整个前缀** —— 也就**重新安装 prelude**。
/// prelude 自己的签名里就有对 prelude 名字的**部分应用**（`Eq.refl` 的类型
/// `… -> Eq.{u} α a a` 里的 `Eq.{u} α`），于是：装 prelude → 部分应用 →
/// 问内核 → 重装 prelude → … **无限递归**（实测栈溢出）。
/// prelude 源文本一律**写全实参**，所以安装期间退回老路是**语义无损**的。
pub(crate) struct PreludeInstallGuard;

impl PreludeInstallGuard {
    pub(crate) fn enter() -> Self {
        PRELUDE_INSTALL_DEPTH.with(|c| c.set(c.get() + 1));
        PreludeInstallGuard
    }
}

impl Drop for PreludeInstallGuard {
    fn drop(&mut self) {
        PRELUDE_INSTALL_DEPTH.with(|c| c.set(c.get().saturating_sub(1)));
    }
}

/// prelude 安装（含 `judge_infer` 触发的**嵌套**安装）期间为真。
pub(crate) fn prelude_install_active() -> bool {
    PRELUDE_INSTALL_DEPTH.with(|c| c.get() > 0)
}

/// 签名 Pi 望远镜的**总层数**（`forall` 的每个 binder 算一层、`->` 算一层）。
pub(crate) fn pi_arity(ty: &Expr) -> usize {
    let mut n = 0;
    let mut cur = ty;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } => {
                n += binders.len();
                cur = body;
            }
            Expr::Arrow { codomain, .. } => {
                n += 1;
                cur = codomain;
            }
            _ => return n,
        }
    }
}

/// 签名的**显式实参层数** = 总层数 − 前导隐式层数。
pub(crate) fn explicit_arity(ty: &Expr) -> usize {
    pi_arity(ty).saturating_sub(leading_implicit_prefix(ty))
}

/// The name table threaded through elaboration: source spelling → resolution.
/// Closure-level and flat (module scoping is G-05), matching the flat
/// `check_name_collisions` contract.
pub(crate) type KnownTable = HashMap<String, KnownName>;

/// R1: the canonical (installed) constructor name. A ctor whose source name
/// already carries a dot is kept verbatim — that protects the prelude
/// (`Nat.zero`/`Bool.true`) and any explicit dotted spelling.
pub(crate) fn canonical_ctor_name(ind: &str, ctor: &str) -> String {
    if ctor.contains('.') {
        ctor.to_string()
    } else {
        format!("{ind}.{ctor}")
    }
}

/// R2: register one bare-name alias. The first constructor to claim a bare
/// name owns it; a second one turns it `Ambiguous` (never silently wins). Real
/// declarations are never overwritten by an alias.
pub(crate) fn insert_ctor_alias(known: &mut KnownTable, source_name: &str, canonical: &str) {
    if source_name.contains('.') {
        return; // already the canonical spelling: nothing to alias
    }
    match known.get(source_name) {
        Some(KnownName::Decl { .. }) => {} // a real declaration wins (R2)
        Some(KnownName::Alias { canonical: first }) if first != canonical => {
            let mut candidates = vec![first.clone()];
            candidates.push(canonical.to_string());
            known.insert(source_name.to_string(), KnownName::Ambiguous { candidates });
        }
        Some(KnownName::Ambiguous { .. }) | Some(KnownName::Alias { .. }) => {}
        None => {
            known.insert(
                source_name.to_string(),
                KnownName::Alias {
                    canonical: canonical.to_string(),
                },
            );
        }
    }
}

/// Resolve one `Expr::Ident` spelling to its canonical kernel name.
///
/// G-05：候选按 [`NamespaceScope::candidates`] 的顺序（当前命名空间链从内到外
/// → 精确名 → `open` 的前缀，按 open 顺序）逐个查 `known`，**第一个命中即止**。
///
/// * a real declaration resolves to itself;
/// * a **unique** bare constructor alias resolves to its canonical name (R2);
/// * an ambiguous bare alias is an error naming both candidates;
/// * an unknown name is the ordinary `elab-unknown-identifier`.
pub(crate) fn resolve_known(
    known: &KnownTable,
    ns: &NamespaceScope,
    name: &str,
    span: Span,
) -> Result<String, CompileError> {
    let Some(candidate) = ns
        .candidates(name)
        .into_iter()
        .find(|candidate| known.contains_key(candidate))
    else {
        return Err(CompileError::elab(
            ErrorKind::ElabUnknownIdentifier,
            format!("unknown identifier `{name}`"),
            span,
        ));
    };
    resolve_candidate(known, &candidate, span)
}

/// 候选名在 `known` 里的解析（`Decl` 自身 / 别名 → 规范名 / 歧义报错）。
fn resolve_candidate(
    known: &KnownTable,
    candidate: &str,
    span: Span,
) -> Result<String, CompileError> {
    match &known[candidate] {
        KnownName::Decl { .. } => Ok(candidate.to_string()),
        KnownName::Alias { canonical } => Ok(canonical.clone()),
        KnownName::Ambiguous { candidates } => Err(CompileError::elab(
            ErrorKind::ElabAmbiguousCtorAlias,
            format!(
                "构造子名 `{candidate}` 有歧义：{} 都声明了它；请写全前缀名",
                candidates
                    .iter()
                    .map(|c| format!("`{c}`"))
                    .collect::<Vec<_>>()
                    .join(" 与 ")
            ),
            span,
        )),
    }
}

/// Same as [`resolve_known`], with the constant-flavoured unknown code
/// (`#check Nat.add.{1}` / `Foo.{u}` style uses).
pub(crate) fn resolve_known_constant(
    known: &KnownTable,
    ns: &NamespaceScope,
    name: &str,
    span: Span,
) -> Result<String, CompileError> {
    // G-05：候选顺序与 `resolve_known` 同一条（`UniverseApp` 与 `Ident` 共用
    // 一套命名空间解析）；只是「都没有」时的码换成常量味。
    match ns
        .candidates(name)
        .into_iter()
        .find(|candidate| known.contains_key(candidate))
    {
        Some(candidate) => resolve_candidate(known, &candidate, span),
        None => Err(CompileError::elab(
            ErrorKind::ElabUnknownConstant,
            format!("unknown constant `{name}`"),
            span,
        )),
    }
}

/// Source-declared inductive metadata that `match` lowering reads (kernel frozen).
#[derive(Debug, Clone)]
pub(crate) struct InductiveInfo<'a> {
    pub ctors: Vec<MatchCtor<'a>>,
    pub recursor: String,
    pub rec_universe_arity: usize,
    pub recursive: bool,
    /// Non-indexed parameter count (`num_params=0` for `Nat`/prelude).
    pub num_params: usize,
    /// Parameter names in declaration order (for `match` param substitution).
    pub param_names: Vec<String>,
    /// Index count (`num_indices=0` for `Nat`/`Bool`/non-indexed inductives).
    pub num_indices: usize,
    /// Index binder source types in declaration order (for `match` motives).
    pub index_types: Vec<Expr>,
}

/// 一条**源级 `def`**：参数名 + 定义体。
///
/// `by` 引擎用它做**一层 delta 展开**——`intro`/`apply` 必须看得穿
/// 「目标/假设的头是个 def」的情形，而本语言**没有内核 whnf 的公开入口**
/// （内核冻结，只有 `check_declar`/`assert_def_eq`/`is_proposition`）。
/// 课程里最典型的形状：`A ⊆ B`（`Set.subset` 是 `def ... := ∀ x, A x → B x`）
/// ——没有这一层，`intro x` 在 `A ⊆ B` 目标上直接报「需要一个函数目标」。
///
/// **只展开一层**：展开后仍要看穿就再来一次（课程里没有更深的嵌套）。
/// 源级展开足够——`Set.subset`/`Not`/`Iff` 都是一层 def。
#[derive(Debug, Clone)]
pub(crate) struct DefInfo {
    /// 声明的参数名（按顺序）——展开时与实参位置对齐做代换。
    pub params: Vec<String>,
    /// 声明的**宇宙参数名**（`def Ne {u} …` 里的 `u`）。
    ///
    /// 为什么 delta 展开需要它：定义体里会出现**宇宙变量**（`Ne` 的体是
    /// `Eq.{u} α a b -> False`），而展开只代换**项**参数——宇宙参数没人管，
    /// 展开出来的 AST 就留着悬空的 `.{u}`。那段 AST 一旦被回读
    /// （render → parse → elab）就报 `unknown universe level u`（实测：
    /// `{a} ≠ ∅` 的证明卡在这儿，而且报错点离根因很远）。
    pub universes: Vec<String>,
    /// 声明的**前导隐式 binder 个数**（与 `KnownName::Decl::implicit_prefix` 同源：
    /// `leading_implicit_prefix(ty)`）。
    ///
    /// **为什么 delta 展开/归一化需要它**（G-69）：源级 AST 里的实参是**写出来的**
    /// 那些（前导隐式实参没写），而 `params` 含隐式 ⇒ 光有 `params.len()` 分不清
    /// 「这个应用写全了没有」✗。pp 又会**省掉**「函数位是裸常量、类型是隐式 Pi」
    /// 的那个实参 ⇒ `Notation(∩, [A, B])` 的 pp 形态只有 2 个实参（忠实形态是
    /// `操作数 + implicit_prefix` 个）⇒ 回读会读成 `α := A` ✗。
    pub implicit_prefix: usize,
    pub body: Expr,
    /// **完整项望远镜层数**（`params_of_ty` 的前导 `Forall` **加上**返回类型里的
    /// `->`）。`by.rs::def_shape` 的「写出来的实参够不够」判据要它 —— 与
    /// `params`（**声明参数**，见 G-72）是两个口径，别混。
    pub telescope_arity: usize,
}

/// 一条 `def` 声明的**参数名**（按顺序）——从它的类型里剥 **Pi 前导链**取 binder 名。
/// 零参 def 返回空表。`by` 引擎的 delta 展开与 prelude 登记共用它。
///
/// **G-72（0.81.0）：只数前导 `Forall`，返回类型里的 `->` 不算参数。**
/// `def mkRel (α : Type) (r : α → α → Prop) : α → α → Prop := fun (a b : α) => …`
/// 的类型是 `(α : Type) → (r : …) → α → α → Prop` —— 后两个箭头是**返回类型**，
/// 不是参数 ✗。老口径把 `Arrow` 也数进来 ⇒ `params` 比值位外面那层 lambda 的
/// binder **多 2** ⇒ `strip_lambdas_n` **多剥两层**（把 `fun (a b : α) =>` 也剥掉）
/// ⇒ 登记进 `defs` 的“定义体”里 `a`/`b` 悬空 ⇒ 展开回读报
/// **`unknown identifier b`**（实测：**单文件判绿、被 `import` 时判红**，因为
/// 「模块自己判卷」不展开这层 delta，而入口引用它时（`And.left h` 解隐式实参）
/// 才展开 ⇒ 同一个模块两种结果 ✗）。
///
/// 边界：返回类型**以 `∀` 开头**时（`def f (α) : ∀ (β : Type), β → β := fun β => …`）
/// 前导链会连着返回类型的那个 `Forall` —— 那不是误判：那种写法的**值位**也必须
/// 从 `fun (β : Type) =>` 开始，两者按位置对齐 ✓（lambda 层数 = 前导 `Forall` 数）。
/// 零参 `def f : Nat → Nat := fun x => x` ⇒ 前导链为空 ⇒ 一层都不剥 ✓
/// （老口径剥 2 层，把 `fun x =>` 也吃掉 ✗）。
/// 需要**完整望远镜**（含返回类型的箭头）的地方（`by.rs::def_shape` 的
/// 「写出来的实参够不够」判据）另算，不能拿这里的结果当总数。
pub(crate) fn params_of_ty(ty: &Expr) -> Vec<String> {
    let mut params = Vec::new();
    let mut cur = ty;
    while let Expr::Forall { binders, body, .. } = cur {
        for b in binders {
            params.push(b.name.clone());
        }
        cur = body;
    }
    params
}

/// 一条类型的**完整项望远镜层数**（前导 `Forall` **加上**返回类型里的 `->`）——
/// `by.rs::def_shape` 的「写出来的实参够不够」判据要它（pp 形态会丢前导隐式实参，
/// 判据问的是「这个常量一共吃几个实参」）。**与 [`params_of_ty`] 是两个口径，
/// 别混**（G-72）。
pub(crate) fn telescope_arity_of_ty(ty: &Expr) -> usize {
    let mut n = 0;
    let mut cur = ty;
    while let Expr::Forall { binders, body, .. } = cur {
        n += binders.len();
        cur = body;
    }
    while let Expr::Arrow { codomain, .. } = cur {
        n += 1;
        cur = codomain;
    }
    n
}

/// 剥掉 `def` 值位外面的 lambda 层，露出**定义体**。
///
/// `def subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x`
/// 的值位在 AST 里是 `fun (α : Type) (A : Set α) (B : Set α) => forall (x : α), …`
/// ——参数被 lambda 包着（实测踩过：不剥就会拿整个 lambda 当"定义体"，
/// `peel_pi` 当然剥不出 Pi，delta 展开静默失败）。参数名由
/// [`params_of_ty`] 从**类型**里取，两者按位置对齐——`n` 就取它的长度。
/// **只剥 `n` 层** lambda（按值位自己的 binder 个数算，一层
/// `Lambda` 可能有多个 binder）。
///
/// **为什么必须限量**：`def` 的值位是 `fun <参数表> => <定义体>`，而定义体
/// **自己**可能也是 lambda——`def Set.union (α) (A B : Set α) : Set α :=
/// fun (x : α) => Or (A x) (B x)`。全剥会把里面的 `fun (x)` 也吃掉，于是
/// `(A ∪ B) x` 的展开变成 `Or (A x) (B x) x`（多贴一个实参，实测：`cases` 在
/// `x ∈ A ∪ B` 上报「头 `Set.union` 不在归纳表里」）。
///
/// `n` 由 `params_of_ty(ty).len()` 给——类型与值位的 binder 按位置对齐。
pub(crate) fn strip_lambdas_n(val: &Expr, n: usize) -> Expr {
    let mut cur = val;
    let mut left = n;
    while left > 0 {
        let Expr::Lambda { binders, body, .. } = cur else {
            break;
        };
        if binders.len() > left {
            // 一层里 binder 比还要剥的多：只剥前 `left` 个。
            let mut out = Expr::Lambda {
                binders: binders[left..].to_vec(),
                body: body.clone(),
                span: crate::Span::default(),
            };
            // 剩下的 binder 仍在，返回带剩余 binder 的 lambda。
            out = match out {
                Expr::Lambda { binders, body, .. } if binders.is_empty() => *body,
                other => other,
            };
            return out;
        }
        left -= binders.len();
        cur = body;
    }
    cur.clone()
}

/// 源级 `def` 表（名字 → 参数名 + 定义体）。跨单元累加，与 `InductiveTable`
/// 同一条命：闭包里依赖按拓扑序排在入口之前，所以入口看得见库里的 def。
pub(crate) type DefTable = HashMap<String, DefInfo>;

/// Forward-accumulated registry of the file's own `inductive` blocks, keyed by
/// inductive name. A `match` may only eliminate an inductive already declared
/// earlier in the file (design §4).
pub(crate) type InductiveTable<'a> = HashMap<String, InductiveInfo<'a>>;

/// Read-only context threaded through elaboration: source prefix + compile
/// options (for the [`judge_infer`] universe query that `match` needs) and the
/// inductive registry.
pub(crate) struct ElabCtx<'a, 'b> {
    pub prefix_src: &'b str,
    pub options: &'b CompileOptions,
    pub inductives: &'b InductiveTable<'a>,
    /// G-05：当前命名空间栈 + `open` 集合（引用解析用，只读）。
    pub ns: &'b NamespaceScope,
    /// **显示用的记法表**（T-U11 ✓）：**只许消息路径用** ✓（诊断文本给学习者看 ⇒ 要折记法 ✓）；
    /// ⚠ **绝不许**用于**判定/解析**路径 ✗（折了会改判定 = 内核红线 ✓），
    /// 也**绝不许**在这层**重建** arity ✗（= 第五套实现 ✓，守卫会抓 ✓）。
    /// `None` = "这里不是显示上下文" ✓（沿用 `prelude.rs` 的既有约定 ✓）。
    pub notations: Option<&'b crate::display::DisplayNotations>,
    /// 已声明的 `def` 体表（只读）。隐式实参的**期望类型 delta 展开**要用它：
    /// `Or.inl h` 的目标常写成 `a ∈ A ∪ B`（`Set.mem … (Set.union …)`，两层
    /// `def`），不展开到 `Or …` 就头部匹配不上。没有表的地方传空表（不影响）。
    pub defs: &'b DefTable,
}

pub(crate) struct ElabScope<'a> {
    names: Vec<String>,
    tys: Vec<ExprPtr<'a>>,
    /// Parallel to `names`: the binder's source type when it was written
    /// explicitly (used to synthesize `judge_infer` binder specs for `match`).
    src_tys: Vec<Option<Expr>>,
    /// Parallel to `names`: each binder's own source span, so a name use can
    /// record where its binder is defined.
    spans: Vec<Span>,
}

impl<'a> ElabScope<'a> {
    pub(crate) fn new() -> Self {
        Self {
            names: Vec::new(),
            tys: Vec::new(),
            src_tys: Vec::new(),
            spans: Vec::new(),
        }
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    fn truncate(&mut self, len: usize) {
        self.names.truncate(len);
        self.tys.truncate(len);
        self.src_tys.truncate(len);
        self.spans.truncate(len);
    }
    fn push(&mut self, name: String, ty: ExprPtr<'a>, src_ty: Option<Expr>, span: Span) {
        self.names.push(name);
        self.tys.push(ty);
        self.src_tys.push(src_ty);
        self.spans.push(span);
    }
    /// 局部变量的**书写类型**（源级 AST）。隐式实参反解优先用它，而不是
    /// `infer_type_text` 的内核 pp 文本：pp 会**丢掉嵌套常量的隐式实参**
    /// （`Eq.{1} Nat 1 1` pp 成 `Eq 1 1`，回读成 `@Eq 1 1` ⇒ `α := 1`），
    /// 于是任何「隐式命题里含 `=`」的短写法（`And.intro h1 h2`、`And.left h`…）
    /// 都被解错、判红。书写类型是 `1 = 1`（记法节点），`unify_extract` 认得。
    fn source_type_of(&self, name: &str) -> Option<Expr> {
        self.names
            .iter()
            .rposition(|candidate| candidate == name)
            .and_then(|i| self.src_tys[i].clone())
    }

    /// Binder specs for [`judge_infer`]: named binders with a written source
    /// type, in scope order. Anonymous/untyped binders are dropped (nothing can
    /// reference them by name).
    fn judge_binders(&self) -> Vec<GoalBinderSpec> {
        self.names
            .iter()
            .zip(self.src_tys.iter())
            .filter(|(name, _)| !name.is_empty())
            .filter_map(|(name, src)| {
                src.as_ref().map(|ty| GoalBinderSpec {
                    name: name.clone(),
                    ty: Some(render_expr(ty)),
                })
            })
            .collect()
    }
    /// [`judge_binders`] 的**源 AST 版**（P1-a 就地判定用）：同样的筛选
    /// （有名字 + **有书写类型**）、同样的顺序，但给的是**源 `Expr`** 而不是渲染文本。
    ///
    /// 为什么需要它：就地路径要**直接造项**（binder 源类型 + 调用点的源 `Expr`），
    /// 不能走 render→回读 —— 回读要么不认识前缀里声明的记法、要么得把 ~46 KB
    /// 前缀整个解析一遍（见 [`infer_type_text_inplace`] 的两条实测死路）。
    fn judge_binder_srcs(&self) -> Vec<(String, Expr)> {
        self.names
            .iter()
            .zip(self.src_tys.iter())
            .filter(|(name, _)| !name.is_empty())
            .filter_map(|(name, src)| src.as_ref().map(|ty| (name.clone(), ty.clone())))
            .collect()
    }

    /// Like [`judge_binders`], but keeps only the binders `expr` (transitively)
    /// depends on, in scope order.
    ///
    /// `judge_infer` peels its answer one Pi layer at a time by re-rendering and
    /// re-parsing each intermediate type, and `render_expr` does not parenthesise
    /// a `forall` that sits in an arrow's domain. A binder whose written type is
    /// itself a function (`hs : (k : Nat) -> P k -> P (succ k)`) therefore
    /// corrupts that round trip and the query returns the wrong sub-term — even
    /// when the sort being asked about never mentions it. Passing only the needed
    /// binders keeps those unrelated function-typed binders out of the telescope.
    fn judge_binders_for(&self, expr: &Expr) -> Vec<GoalBinderSpec> {
        let n = self.len();
        let mut needed = vec![false; n];
        for (i, name) in self.names.iter().enumerate() {
            if !name.is_empty() && mentions_ident(expr, name) {
                needed[i] = true;
            }
        }
        // A binder's written type may only mention earlier binders, so a single
        // right-to-left pass closes the dependency set.
        for i in (0..n).rev() {
            if !needed[i] {
                continue;
            }
            if let Some(ty) = self.src_tys[i].as_ref() {
                for (j, name) in self.names.iter().enumerate().take(i) {
                    if !needed[j] && !name.is_empty() && mentions_ident(ty, name) {
                        needed[j] = true;
                    }
                }
            }
        }
        (0..n)
            .filter(|&i| needed[i])
            .filter_map(|i| {
                self.src_tys[i].as_ref().map(|ty| GoalBinderSpec {
                    name: self.names[i].clone(),
                    ty: Some(render_expr(ty)),
                })
            })
            .collect()
    }
    /// The written source type of the innermost binder named `name`.
    fn src_ty(&self, name: &str) -> Option<&Expr> {
        let pos = self.names.iter().rposition(|candidate| candidate == name)?;
        self.src_tys[pos].as_ref()
    }
}

/// One recorded sub-expression during elaboration, with the binder scope it
/// lives under (outermost first). Used to answer editor hovers.
pub(crate) struct HoverNode<'a> {
    pub(crate) span: Span,
    pub(crate) expr: ExprPtr<'a>,
    pub(crate) scope_names: Vec<String>,
    pub(crate) scope_tys: Vec<ExprPtr<'a>>,
    /// When this node is an ident use point: where the name is defined.
    /// Top-level targets carry a placeholder span here and are backfilled
    /// from the file's name → def-span map in `run_pass`.
    pub(crate) resolution: Option<ResolvedTarget>,
    /// This node is a lambda/forall **binder declaration** (`name : ty`):
    /// the hover should render the declaration itself (not `expr : type`).
    pub(crate) binder: bool,
}

pub(crate) fn record_hover<'a>(
    hovers: &mut Vec<HoverNode<'a>>,
    scope: &ElabScope<'a>,
    span: Span,
    expr: ExprPtr<'a>,
    resolution: Option<ResolvedTarget>,
) {
    hovers.push(HoverNode {
        span,
        expr,
        scope_names: scope.names.clone(),
        scope_tys: scope.tys.clone(),
        resolution,
        binder: false,
    });
}

/// Record a binder-declaration hover row (`name : ty`), with the scope as it
/// was **before** this binder was pushed (the type is elaborated in that scope).
pub(crate) fn record_binder_hover<'a>(
    hovers: &mut Vec<HoverNode<'a>>,
    scope: &ElabScope<'a>,
    span: Span,
    ty: ExprPtr<'a>,
) {
    hovers.push(HoverNode {
        span,
        expr: ty,
        scope_names: scope.names.clone(),
        scope_tys: scope.tys.clone(),
        resolution: None,
        binder: true,
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn install_inductive_block<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    table: &mut InductiveTable<'a>,
    prefix_src: &str,
    options: &CompileOptions,
    ns: &NamespaceScope,
    name: &str,
    params: &[Binder],
    ty: &Expr,
    constructors: &[CtorDecl],
    recursor: Option<&RecDecl>,
    iota_rules: &[crate::IotaRule],
    hovers: &mut Vec<HoverNode<'a>>,
    built: &mut Vec<Declar<'a>>,
) -> Result<(), CompileError> {
    let num_params = u16::try_from(params.len()).map_err(|_| {
        CompileError::elab(
            ErrorKind::ElabTooManyBinders,
            "too many inductive parameters",
            ty.span(),
        )
    })?;
    // 带索引归纳：`ty` 的 Pi 望远镜（本编译器把参数也一并记在这里，见
    // `derive_recursor` 的索引命名与 `recursor_telescope`；`num_indices` 的
    // 口径以既有行为为准，勿与内核的 `local_indices` 混为一谈）。
    let index_binders = result_chain_binders(ty);
    let num_indices = u16::try_from(index_binders.len()).map_err(|_| {
        CompileError::elab(
            ErrorKind::ElabTooManyBinders,
            "too many inductive indices",
            ty.span(),
        )
    })?;
    // K 目标标志由块形状唯一决定，**显式 rec 与派生 rec 必须给同一个值**：
    // 内核断言 `rd.is_k == st.k_target`（`kernel/src/inductive.rs:662`）。
    let is_k = is_k_target(ty, constructors);
    let empty: UnivMap = UnivMap::new();
    // 归纳声明自身内部出现 `match` 的情形按「本块尚未登记」处理（递归类型本就
    // 不在 v1 支持内）。这里借用既有登记表，插入在本函数末尾进行。
    let empty_defs: DefTable = DefTable::new();
    let elab_ctx = ElabCtx {
        notations: None,
        prefix_src,
        options,
        inductives: table,
        ns,
        defs: &empty_defs,
    };
    // 归纳类型 = `forall params, ty`：params 是内核 Pi 望远镜最外层（顺序与
    // 声明的 binder 风格一致），ty 在它们的作用域内 elaborate。
    // `ind_ty_src` 供 `derive_recursor` 判「块是不是 Prop / 有没有索引」——
    // 那是**源级**问题，必须用参数未剥离的源类型。
    let ind_ty_src = if params.is_empty() {
        ty.clone()
    } else {
        Expr::Forall {
            binders: params.to_vec(),
            body: Box::new(ty.clone()),
            span: ty.span(),
        }
    };
    let kernel_ind_ty = elab_expr(
        builder,
        &ind_ty_src,
        &mut ElabScope::new(),
        &empty,
        known,
        hovers,
        None,
        None,
        &elab_ctx,
    )?;
    let ind_name = builder.name_from_str(name);
    // R1：安装名（规范名）= 内核里的构造子名。源名 `c.name` 仍用于源级匹配
    // （`iota` 规则、`match` 分支），别名（R2）只在解析层。
    let ctor_canonical: Vec<String> = constructors
        .iter()
        .map(|c| canonical_ctor_name(name, &c.name))
        .collect();
    let ctor_names: Vec<NamePtr<'a>> = ctor_canonical
        .iter()
        .map(|canonical| builder.name_from_str(canonical))
        .collect();
    // 内核按「构造子 binder 类型里是否提到归纳名」自算 is_recursive 并断言
    // 一致（inductive.rs::end_block）——这里从源码 AST 做同规则镜像，非递归
    // 块（Bool/Unit/Empty）才能通过声明检查。
    let is_recursive = constructors.iter().any(|ctor| {
        ctor.binders
            .iter()
            .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
            || result_telescope_mentions(&ctor.result, name)
    });
    let no_uparams = builder.alloc_levels_slice(&[]);
    builder.begin_inductive_block();
    let ind_declar = builder
        .add_inductive(
            DeclarInfo {
                name: ind_name,
                uparams: no_uparams,
                ty: kernel_ind_ty,
            },
            is_recursive,
            num_params,
            num_indices,
            Arc::from([ind_name]),
            Arc::from(ctor_names.clone()),
        )
        .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, Span::default()))?;
    built.push(ind_declar);
    known.insert(
        name.to_string(),
        KnownName::Decl {
            universes: Vec::new(),
            implicit_prefix: 0,
            explicit_arity: pi_arity(&ind_ty_src),
            signature: Some(crate::proof::decl_signature(&ind_ty_src)),
        },
    );

    let mut match_ctors: Vec<MatchCtor<'a>> = Vec::with_capacity(constructors.len());
    // 每个构造子已 elaborate 的**内核** Pi 望远镜（`params ++ fields`）——派生
    // recursor 的 large-elimination 判据要读它（见 `kernel_large_elim_test`）。
    let mut kernel_ctor_tys: Vec<ExprPtr<'a>> = Vec::with_capacity(constructors.len());
    for (idx, ctor) in constructors.iter().enumerate() {
        // ctor 类型 = `forall (params ++ fields), result`：参数先于字段，且必须
        // 与归纳声明的参数逐位同形（内核 check_ctor 会 def_eq 断言）。
        let mut ctor_binders: Vec<Binder> = params.to_vec();
        ctor_binders.extend(ctor.binders.iter().cloned());
        let ctor_ty = Expr::Forall {
            binders: ctor_binders,
            body: Box::new(ctor.result.clone()),
            span: ctor.span,
        };
        let ctor_src_arity = pi_arity(&ctor_ty);
        let ctor_src_sig = crate::proof::decl_signature(&ctor_ty);
        let ctor_ty = elab_expr(
            builder,
            &ctor_ty,
            &mut ElabScope::new(),
            &empty,
            known,
            hovers,
            None,
            None,
            &elab_ctx,
        )?;
        // 字段元数据：类型用已 elaborate 的内核 Pi 望远镜（与 recursor 的
        // minor 形状逐位一致），源码 binder 提供 hover / judge 用的源类型。
        // 内核望远镜前 `num_params` 层是参数，字段从其后的位置开始。
        let src_fields = ctor_field_binders(ctor);
        let kernel_fields = kernel_field_binders(ctor_ty);
        let kernel_field_tys = kernel_fields.into_iter().skip(params.len());
        let fields = src_fields
            .iter()
            .zip(kernel_field_tys)
            .map(|(src, (style, kernel_ty))| MatchField {
                name: src.name.clone(),
                ty: kernel_ty,
                style,
                src_ty: src.ty.as_deref().cloned(),
            })
            .collect();
        match_ctors.push(MatchCtor {
            name: ctor.name.clone(),
            canonical: ctor_canonical[idx].clone(),
            fields,
        });
        kernel_ctor_tys.push(ctor_ty);
        let ctor_name = ctor_names[idx];
        let no_uparams = builder.alloc_levels_slice(&[]);
        // 内核把构造子类型整体当 Pi 望远镜数字段（result 箭头链的 domain
        // 也是字段），num_fields = 望远镜 − 参数（check_declared_metadata）。
        // `ctor_field_binders` 只含字段，故无需再减参数。
        let num_fields = u16::try_from(ctor_field_binders(ctor).len()).map_err(|_| {
            CompileError::elab(
                ErrorKind::ElabTooManyCtorFields,
                "too many constructor fields",
                ctor.span,
            )
        })?;
        let ctor_declar = Declar::Constructor(ConstructorData {
            info: DeclarInfo {
                name: ctor_name,
                uparams: no_uparams,
                ty: ctor_ty,
            },
            inductive_name: ind_name,
            ctor_idx: idx as u16,
            num_params,
            num_fields,
        });
        builder
            .add_declar(ctor_declar.clone())
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, ctor.span))?;
        built.push(ctor_declar);
        // R1：规范名进解析表；R2：裸名作为**别名键**（唯一才可解析）。
        known.insert(
            ctor_canonical[idx].clone(),
            KnownName::Decl {
                universes: Vec::new(),
                // Lean：归纳**参数在构造子类型里是隐式的**（`And.intro {a b} …`）
                // ⇒ 构造子的前导隐式层数 = 参数个数。写全参数的调用点走老路
                // （`layers.len() < k + args.len()` ⇒ `try_implicit_application`
                // 返回 `None`），所以既有语料逐字节不变。
                implicit_prefix: params.len(),
                explicit_arity: ctor_src_arity.saturating_sub(params.len()),
                signature: Some(ctor_src_sig),
            },
        );
        insert_ctor_alias(known, &ctor.name, &ctor_canonical[idx]);
    }

    // 显式 rec 优先：源里有 rec 时零行为变化；无 rec 时自动派生等价的
    // RecDecl + iota 规则（py-nat 手写版同构），再走同一条 elab 路径。
    //
    // 派生**推迟到这里**（ctor 类型 elaborate 之后，策略 A）：recursor 要不要
    // 额外宇宙参数，由内核 `large_elim_test` 的镜像判据决定，而它要读每个构造子
    // 已 elaborate 的**内核**字段类型与结果实参（G-03 / WO-006，设计
    // docs/design/prop-large-elim-mirror.md §3）。
    let owned_rec;
    let owned_rules;
    let (recursor, iota_rules): (&RecDecl, &[crate::IotaRule]) = match recursor {
        Some(rec) => (rec, iota_rules),
        None => {
            // `ty` 是**源级结果排序**（`Prop` / `A -> Prop`），参数不在其中：
            // `derive_recursor` 的索引望远镜与 `is_prop_block_ty` 都以此为口径。
            let block_is_prop = is_prop_block_ty(ty);
            let wants_u = large_elim_test_mirror(
                &elab_ctx,
                builder,
                params,
                name,
                constructors,
                &kernel_ctor_tys,
                block_is_prop,
            );
            let (rec, rules) =
                derive_recursor(name, params, ty, constructors, &ctor_canonical, wants_u);
            owned_rec = rec;
            owned_rules = rules;
            (&owned_rec, &owned_rules)
        }
    };

    let rec_name_text = recursor.name.clone();
    let rec_universe_arity = recursor.universe.len();
    {
        let rec = recursor;
        let univ = make_univ_map(builder, &rec.universe);
        let rec_ty = elab_expr(
            builder,
            &rec.ty,
            &mut ElabScope::new(),
            &univ,
            known,
            hovers,
            None,
            None,
            &elab_ctx,
        )?;
        let rec_name = builder.name_from_str(&rec.name);
        let known_rec_universes = rec.universe.clone();
        known.insert(
            rec.name.clone(),
            KnownName::Decl {
                universes: known_rec_universes.clone(),
                implicit_prefix: 0,
                explicit_arity: pi_arity(&rec.ty),
                signature: Some(crate::proof::decl_signature(&rec.ty)),
            },
        );

        let mut rules = Vec::with_capacity(iota_rules.len());
        for rule in iota_rules {
            // R3：显式 `iota` 规则按**源名**匹配（`iota zero :=` 里的 `zero`）；
            // 派生规则带的是规范名，两种拼写都命中。迁移轮把 `ctor zero` 改写成
            // `ctor Nat.zero` 时，同文件的 `iota` 也必须跟着写点名前缀。
            let ctor_idx = constructors
                .iter()
                .enumerate()
                .position(|(i, c)| c.name == rule.ctor_name || ctor_canonical[i] == rule.ctor_name)
                .ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabUnknownCtorForIota,
                        format!(
                            "iota rule refers to unknown constructor `{}`",
                            rule.ctor_name
                        ),
                        rule.span,
                    )
                })?;
            let ctor_name = ctor_names[ctor_idx];
            let val = elab_expr(
                builder,
                &rule.val,
                &mut ElabScope::new(),
                &univ,
                known,
                hovers,
                None,
                None,
                &elab_ctx,
            )?;
            rules.push(RecRule {
                ctor_name,
                // 与 num_fields 同规则：按整条 Pi 望远镜计（含 result 链）。
                ctor_telescope_size_wo_params: ctor_field_binders(&constructors[ctor_idx]).len()
                    as u16,
                val,
            });
        }
        let info = DeclarInfo {
            name: rec_name,
            uparams: collect_uparams(builder, &univ, &known_rec_universes),
            ty: rec_ty,
        };
        let rec_declar = Declar::Recursor(RecursorData {
            info,
            all_inductives: Arc::from([ind_name]),
            num_params,
            num_indices,
            num_motives: 1,
            num_minors: constructors.len() as u16,
            rec_rules: Arc::from(rules),
            is_k,
        });
        builder
            .add_declar(rec_declar.clone())
            .map_err(|e| CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, rec.span))?;
        built.push(rec_declar);
    }
    builder.end_inductive_block();
    table.insert(
        name.to_string(),
        InductiveInfo {
            ctors: match_ctors,
            recursor: rec_name_text,
            rec_universe_arity,
            recursive: is_recursive,
            num_params: params.len(),
            param_names: params.iter().map(|b| b.name.clone()).collect(),
            num_indices: index_binders.len(),
            index_types: index_binders
                .iter()
                .map(|b| {
                    b.ty.as_deref()
                        .cloned()
                        .unwrap_or(Expr::Hole { span: b.span })
                })
                .collect(),
        },
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_def<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty_kernel = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let val_kernel = elab_expr(
        builder,
        val,
        &mut scope,
        &univ,
        known,
        hovers,
        Some(ty_kernel),
        Some(ty),
        ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Definition {
        info: DeclarInfo {
            name,
            uparams,
            ty: ty_kernel,
        },
        val: val_kernel,
        hint: ReducibilityHint::Regular(0),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_theorem<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty_kernel = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let val_kernel = elab_expr(
        builder,
        val,
        &mut scope,
        &univ,
        known,
        hovers,
        Some(ty_kernel),
        Some(ty),
        ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Theorem {
        info: DeclarInfo {
            name,
            uparams,
            ty: ty_kernel,
        },
        val: val_kernel,
    })
}

pub(crate) fn build_example<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    ty: &Expr,
    val: &Expr,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, &[]);
    let ty_kernel = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let val_kernel = elab_expr(
        builder,
        val,
        &mut scope,
        &univ,
        known,
        hovers,
        Some(ty_kernel),
        Some(ty),
        ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = builder.alloc_levels_slice(&[]);
    Ok(Declar::Definition {
        info: DeclarInfo {
            name,
            uparams,
            ty: ty_kernel,
        },
        val: val_kernel,
        hint: ReducibilityHint::Regular(0),
    })
}

pub(crate) fn build_axiom<'a>(
    builder: &mut EnvBuilder<'a>,
    name: &str,
    universe: &[String],
    ty: &Expr,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Declar<'a>, CompileError> {
    let mut scope = ElabScope::new();
    let univ = make_univ_map(builder, universe);
    let ty = elab_expr(
        builder, ty, &mut scope, &univ, known, hovers, None, None, ctx,
    )?;
    let name = builder.name_from_str(name);
    let uparams = collect_uparams(builder, &univ, universe);
    Ok(Declar::Axiom {
        info: DeclarInfo { name, uparams, ty },
    })
}

pub(crate) fn make_univ_map<'a>(builder: &mut EnvBuilder<'a>, universe: &[String]) -> UnivMap<'a> {
    universe
        .iter()
        .map(|name| {
            let ptr = builder.name_from_str(name);
            let level = builder.level_param(ptr);
            (name.clone(), level)
        })
        .collect()
}

pub(crate) fn collect_uparams<'a>(
    builder: &mut EnvBuilder<'a>,
    univ: &UnivMap<'a>,
    universe: &[String],
) -> sokonanoda::util::LevelsPtr<'a> {
    let levels: Vec<LevelPtr<'a>> = universe.iter().map(|name| univ[name]).collect();
    builder.alloc_levels_slice(&levels)
}

/// 层级**文本** → 内核层级（`EnvBuilder` 的公开 API：`zero`/`succ`/
/// `level_param`，硬规则 1 内核零改动）。
///
/// 文本语法（parser 的 `parse_level_text`，设计 `docs/design/type-level-syntax.md` §5）：
/// `3`、`u`、`u+1`、`u+1+1`——首段是数字或已声明的宇宙参数，其余每段必须是
/// 数字（各加一次 `succ`）。`u+v`/`max` 不在语法面内，落到 `unknown universe
/// level` 诊断。
pub(crate) fn level_ptr<'a>(
    builder: &mut EnvBuilder<'a>,
    level: &str,
    univ: &UnivMap<'a>,
    span: Span,
) -> Result<LevelPtr<'a>, CompileError> {
    let unknown = |level: &str| {
        CompileError::elab(
            ErrorKind::ElabUnknownUniverseLevel,
            format!("unknown universe level `{level}`"),
            span,
        )
    };
    let mut parts = level.split('+');
    let head = parts.next().unwrap_or(level);
    let mut out = if let Ok(n) = head.parse::<u64>() {
        let mut out = builder.zero();
        for _ in 0..n {
            out = builder.succ(out);
        }
        out
    } else if let Some(level) = univ.get(head).copied() {
        level
    } else {
        return Err(unknown(level));
    };
    for part in parts {
        let Ok(n) = part.parse::<u64>() else {
            return Err(unknown(level));
        };
        for _ in 0..n {
            out = builder.succ(out);
        }
    }
    Ok(out)
}

pub(crate) fn kernel_binder_style(kind: &BinderKind) -> BinderStyle {
    match kind {
        BinderKind::Explicit => BinderStyle::Default,
        BinderKind::Implicit => BinderStyle::Implicit,
    }
}

/// 记法展开（G-04 / WO-011，设计 N4）：把记号节点降级成 `App` 形状。
///
/// 目标 telescope 比操作数多出来的**前导参数**（`Set.mem (α : Type) …` 的
/// `α`）按固定顺序补全：
///
/// 1. 由**操作数**解出：第 2 个参数的类型就是裸变量 `α` ⇒ `α := typeof(a)`；
/// 2. 无操作数时由**期望类型**解出：`Set.empty : (α) → Set α` 对上期望
///    `Set α₀` ⇒ `α := α₀`。
///
/// 解不出 ⇒ `elab-notation-argument-unsolved`（hint 教点名写法）。
/// 目标名不存在 ⇒ `elab-notation-unknown-target`。
///
/// **补全只发生在这条路径上**：点名写法（`Set.mem a A`，省 `α`）继续被内核
/// 拒绝（设计 N4.3 的护城河）。
#[allow(clippy::too_many_arguments)]
fn elab_notation<'a>(
    builder: &mut EnvBuilder<'a>,
    symbol: &str,
    target: &str,
    operands: &[&Expr],
    span: Span,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    fallback_prefix_args: Option<&[Expr]>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    let canonical = resolve_known(known, ctx.ns, target, span).map_err(|_| {
        CompileError::elab(
            ErrorKind::ElabNotationUnknownTarget,
            format!("记法 `{symbol}` 指向的目标 `{target}` 不存在：检查记法命令里的名字（要写点名，例如 Set.mem）"),
            span,
        )
    })?;
    // 目标自身的签名（`forall (α : Type 0), α -> Set α -> Prop`）由内核 pp
    // 渲染（不做文本比对）。**用 `judge_type_of` 而不是 `judge_infer`**：后者
    // 要先合成 `fun (binders) => target` 再逐层剥 binder，目标本身是函数时
    // （`Set.image`）多 binder 折叠会让剥离结果错位（第二刀实测），而签名是
    // 常量自己的、与调用点的 binder 无关。
    // 用**常量签名缓存**（键不含前缀）：记法在每个使用点都要问一次签名，而
    // `judge_type_of` 的缓存键含整段前缀 ⇒ 逐声明退化成正前缀重编译（O(n²)）。
    let signature = crate::judge::judge_type_of_constant(ctx.prefix_src, ctx.options, &canonical)
        .map_err(|j| {
        CompileError::elab(
            ErrorKind::ElabNotationUnknownTarget,
            format!(
                "读不到记法 `{symbol}` 的目标 `{target}` 的类型：{}",
                judgement_message(&j)
            ),
            span,
        )
    })?;
    // 前导参数：先走既有的裸变量匹配；解不出时用调用方给的**回退**
    // （第三刀 §12.4 的集合字面量：`{∅}` 的元素类型只能从期望类型解，
    // 既有路径的结构化匹配在这里够不着——回退只加解、不改既有解）。
    let uparams = known
        .get(&canonical)
        .map(|entry| entry.universes().to_vec())
        .unwrap_or_default();
    // **隐式档：走唯一钩子**（T-N14 的核心，2026-09-30）。目标签名有前导隐式
    // binder ⇒ 把 `symbol(op1 … opn)` 还原成**源级应用脊** `target op1 … opn`，
    // 整条交给 `elab_expr` —— 前缀由 `try_implicit_application` 解出，记法路径
    // **不再有第二套补参机械** ✗（那套只做"后续 binder 的域里提到这个裸变量"
    // 的结构化匹配，没有唯一钩子的三条判据 ⇒ 实测把 `Set.univ ∩ A` 的 `α` 解成
    // `Type 0`、`Sort(2)` 撞 `Sort(1)` ✗ —— 就是 S1 地图的**模式 B**）。
    //
    // **免费闸门**：`implicit_prefix == 0` 的记法（内建的 `= ∧ ∨ ↔ ¬ ∃` 与今天
    // 全部课程库记法）**根本走不到这里** ⇒ 下面那段**逐字节不变** ✓。
    if known
        .get(&canonical)
        .map(|entry| entry.implicit_prefix())
        .unwrap_or(0)
        > 0
    {
        let level_texts = notation_level_texts(builder, &uparams, operands, ctx, scope, known);
        return elab_notation_implicit(
            builder,
            symbol,
            target,
            &canonical,
            &level_texts,
            operands,
            span,
            scope,
            univ,
            known,
            hovers,
            expected_src,
            ctx,
        );
    }
    let prefix_args = match notation_prefix_args(
        &signature,
        operands,
        expected_src,
        ctx,
        scope,
        Some(&mut InplaceEnv {
            builder: &mut *builder,
            known,
        }),
    )? {
        Some(args) => args,
        None => match fallback_prefix_args {
            Some(args) => args.to_vec(),
            None => {
                return Err(notation_argument_unsolved(symbol, target, span));
            }
        },
    };
    // 操作数的**期望类型**：`∅ ⊆ A` 里的 `∅` 自己也是零元记法，只有拿到
    // 「这里是 `Set α`」才知道补什么（设计 N4.2 ② 在嵌套位置上的同一规则）。
    // 期望类型文本由内核 pp 给出（`judge_infer` 读目标签名），再按前导参数
    // 的实例代换。
    let operand_expected = notation_operand_expected(&signature, &prefix_args, operands.len());
    // 源到源拼出完整应用，再交给**既有** elaborate 路径——类型错、`@`、
    // 宇宙参数等语义一字不改地复用。
    //
    // **宇宙参数**（L2.4b / L2.3，设计 SP1）：`Eq`/`Ne` 各带一个 `u`，
    // 而 `Eq.{1} (Set α) A B` 与 `Eq.{0} A B`（`A B : Prop`）是**两个不同的
    // 常量应用**——点名路径靠源里的 `.{1}` 写死，记法路径必须自己解。
    // 解不出时保持既有行为（全 0，与不写 `.{u}` 的点名写法同判）。
    //
    // ⚠ **位置**：这一段在 k==0 路径里必须留在**解前导参数之后**（与 B3-④ 之前
    // 逐字一致）；隐式档在上面的分支里另算一份。
    let level_texts = notation_level_texts(builder, &uparams, operands, ctx, scope, known);
    let levels = if level_texts.is_empty() {
        builder.alloc_levels_slice(&[])
    } else {
        let mut resolved = Vec::with_capacity(level_texts.len());
        for text in &level_texts {
            // **解不出就退回 0**（= 这条记法在引入宇宙求解之前的旧行为），
            // 不报错。为什么不能报错：层级文本来自**内核 pp**，而 pp 会打出
            // **宇宙变量名**（`Eq.{u}`）——那个 `u` 在我们这层作用域里根本不存在，
            // 于是 `level_ptr` 报 `unknown universe level u`，把一条本来能过的
            // 声明判红（实测：`{a} ≠ ∅` 的证明）。退回 0 至少不比从前差。
            match level_ptr(builder, text, univ, span) {
                Ok(level) => resolved.push(level),
                Err(_) => resolved.push(builder.zero()),
            }
        }
        builder.alloc_levels_slice(&resolved)
    };
    let const_name = builder.name_from_str(&canonical);
    let mut app = builder.mk_const(const_name, levels);
    for arg in &prefix_args {
        let arg = elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx)?;
        app = builder.mk_app(app, arg);
    }
    for (i, operand) in operands.iter().enumerate() {
        let expected_src = operand_expected.get(i).and_then(|t| t.as_ref());
        let operand = elab_expr(
            builder,
            operand,
            scope,
            univ,
            known,
            hovers,
            None,
            expected_src,
            ctx,
        )?;
        app = builder.mk_app(app, operand);
    }
    Ok(app)
}

/// 记法目标的**宇宙层级文本**（`Eq.{u}` 那一档）：目标只带 1 个宇宙参数时由
/// **操作数**解（`a : Set α` ⇒ `Set α : Type 0` ⇒ `u = 1`），其余位默认 `"0"`；
/// 目标没有宇宙参数 ⇒ 空表（调用方据此走"零层级"那条）。
///
/// 两条路（k==0 的老路与隐式档）共用这一份；解不出时**保持既有行为**（全 0，
/// 与不写 `.{u}` 的点名写法同判），见调用点那段注释。
fn notation_level_texts<'a>(
    builder: &mut EnvBuilder<'a>,
    uparams: &[String],
    operands: &[&Expr],
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    known: &KnownTable,
) -> Vec<String> {
    if uparams.is_empty() {
        return Vec::new();
    }
    let mut texts: Vec<String> = uparams.iter().map(|_| "0".to_string()).collect();
    if uparams.len() == 1 {
        if let Some(text) = universe_level_text_of_operands(
            operands,
            ctx,
            scope,
            Some(&mut InplaceEnv {
                builder: &mut *builder,
                known,
            }),
        ) {
            texts[0] = text;
        }
    }
    texts
}

/// 记法展开的**隐式档**（T-N14 / B3-④，2026-09-30）：目标签名带**前导隐式
/// binder** 时，把 `symbol(op1 … opn)` 还原成**源级应用脊** `target op1 … opn`，
/// 整条交给 `Expr::App` 臂 —— 前缀由**唯一钩子** `try_implicit_application` 解出。
///
/// **为什么必须借道唯一钩子**：记法路径原来那套（`notation_prefix_args`）只做
/// 「后续 binder 的域里提到这个裸变量」的**结构化匹配**，没有唯一钩子的三条
/// 判据（旧写法贴合 / 富余实参落到结果 / 期望类型逐位代换）⇒ 实测把
/// `Set.univ ∩ A` 的 `α` 解成 `Type 0`（`Sort(2)` 撞 `Sort(1)` ✗，S1 地图的
/// **模式 B**）。借道之后两条路**共用一份机械**，判据不会分叉 ✓。
///
/// **免费闸门仍在**：`implicit_prefix == 0` 的记法（内建的 `= ∧ ∨ ↔ ¬ ∃` 与
/// 今天全部课程库记法）**根本走不到这里** ⇒ 逐字节不变 ✓。
#[allow(clippy::too_many_arguments)]
fn elab_notation_implicit<'a>(
    builder: &mut EnvBuilder<'a>,
    symbol: &str,
    target: &str,
    canonical: &str,
    level_texts: &[String],
    operands: &[&Expr],
    span: Span,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    // **零操作数 + 无期望类型** ⇒ 前缀**无从解出**（`#check ∅` 就是这一档：
    // 裸常量没有期望类型时唯一钩子**故意**不动它 —— 裸常量当函数值用是合法的）。
    // 记法这层知道用户写的是**一个应该完整的表达式**，所以在这里报既有诊断
    // （与 k==0 路径同一个码、同一条教学 hint ✓）。
    if operands.is_empty() && expected_src.is_none() {
        return Err(notation_argument_unsolved(symbol, target, span));
    }
    let mut app: Expr = if level_texts.is_empty() {
        Expr::Ident {
            name: canonical.to_string(),
            span,
        }
    } else {
        Expr::UniverseApp {
            name: canonical.to_string(),
            levels: level_texts.to_vec(),
            span,
        }
    };
    for operand in operands {
        app = Expr::App {
            fun: Box::new(app),
            arg: Box::new((*operand).clone()),
            explicit_spine: false,
            span,
        };
    }
    elab_expr(
        builder,
        &app,
        scope,
        univ,
        known,
        hovers,
        None,
        expected_src,
        ctx,
    )
    .map_err(|e| {
        // 唯一钩子报的是**应用路径**的码（`elab-implicit-argument-unsolved`）。
        // 用户写的是**记法**，所以换回记法那条码与 hint —— 同一处缺口，两种
        // 写法给同一种教学引导 ✓（`docs/protocol.md` 的码是给工具用的，
        // 换码是**有意**的：记法的诊断要教"写出点名形式"）。
        if e.kind == ErrorKind::ElabImplicitArgumentUnsolved {
            notation_argument_unsolved(symbol, target, span)
        } else {
            e
        }
    })
}

/// 记法展开补不出前导参数时的**唯一**诊断（两条路共用一个码与一条 hint）。
fn notation_argument_unsolved(symbol: &str, target: &str, span: Span) -> CompileError {
    CompileError::elab(
        ErrorKind::ElabNotationArgumentUnsolved,
        format!(
            "记法 `{symbol}` 展开成 `{target}` 时补不出前面的类型参数：请写出点名形式（例如 {target} α …）"
        ),
        span,
    )
}

/// 这个实参**必须**拿到期望类型才能 elaborate 吗？
///
/// 判据是「**补不出前导类型参数**的形状」：
/// - **零元记法**（`∅` → `Set.empty`）：没有操作数，只能从期望类型解；
/// - **集合字面量**（`{a}` / `{a, b}` → `Set.singleton` / `Set.pair`）：元素类型
///   只能从期望类型解（设计 `docs/design/notation-subset.md` §12.4）。
///
/// 其余形状（点名、应用、lambda…）不需要，于是**零开销**——这是这条推广不拖慢
/// 编译的关键。
/// 开关 `SOKO_ARG_EXPECTED`（**默认关** ✓）：关着时本片**一次都不进** ⇒ 逐字节不变 ✓。
/// **探针身份**（值守 2026-10-04 拍板 ✓ —— 见 `AGENTS.md`「探针读数必须带构建身份」✓）。
///
/// **为什么**：探针读数一旦跨**不同构建**比较就会得出**反向结论** ✗（第 5 轮实测：
/// 四出口探针散在不同构建里 ⇒ 不可比却当可比 ⇒ 白烧一轮 ✓）。
/// 所以每条读数**行首自带身份** ✓ ⇒ 两份日志**并排就自明不可比** ✓（不靠人记 ✓）。
///
/// **取法（同一处取 ✓）**：**运行中二进制**的 mtime 秒 + 字节数 ✓ —— 零依赖、零内核调用 ✓，
/// 且**改了任何前端代码重编都会变** ✓（这正是"同一份构建"的判据 ✓）。
/// ⚠ `#[allow(dead_code)]` **是故意的** ✓：它是**探针工具**（平时没有调用点 ✓，
/// 只在"要打探针"时被临时接上 ✓）⇒ 不能因为"当前没人用"就删掉 ✗。
/// 它由下面 `probe_identity_is_stable_and_self_describing` 那条单测**钉住** ✓
/// （守卫咬得住 ✓：模板一变、身份一空，测试就红 ✗）。
#[allow(dead_code)]
pub(crate) fn probe_tag() -> &'static str {
    static TAG: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TAG.get_or_init(|| {
        let meta = std::env::current_exe()
            .ok()
            .and_then(|p| std::fs::metadata(p).ok());
        match meta {
            Some(m) => {
                let secs = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                format!("build={secs} bytes={}", m.len())
            }
            None => "build=unknown".to_string(),
        }
    })
    .as_str()
}

/// **探针行模板（统一 ✓）**：`[<构建身份>] <标签> <正文>` ✓。
///
/// ⚠ **纪律**（`AGENTS.md` ✓）：① 探针**必须打在判据生效点之后** ✗（打反了会得出**反向结论** ✗）；
/// ② **跑完必还原** ✓（`grep` = 0 ✓）；③ 不同构建的读数**只能各自单独看** ✗。
#[allow(dead_code)]
pub(crate) fn probe_line(tag: &str, body: &str) -> String {
    format!("[{}] {} {}", probe_tag(), tag, body)
}

pub(crate) fn arg_expected_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        matches!(
            std::env::var("SOKO_ARG_EXPECTED").ok().as_deref(),
            Some("1") | Some("on")
        )
    })
}

/// **头是局部变量**时的实参期望类型（B1 片 · G-86）—— **零内核调用、零递归** ✓。
///
/// 为什么必须走这条而不是 [`application_arg_expected`] ✗：后者要 `judge_infer` **头**的类型，
/// 而判定**再入** elaborate ⇒ 对"每个应用实参都算期望类型"的形状**栈溢出**（实测 exit 134 ✗）。
/// 局部变量的**书写类型**就在 `scope.src_tys` 里 ✓ ⇒ 直接剥 Π 到实参位 ✓，不问内核 ✓。
///
/// 形状：`h : ¬ (P ∨ Q)` 写成 `h (Or.inl hp)` ⇒ 实参的期望类型 = `h` 的域 `P ∨ Q` ✓
/// （`¬ X` 是 **def 头** ⇒ δ 展开一次 ✓）；`Or.inl` 的 `?B` 由此定下 ✓（台账 G-86 ✗）。
/// **B1 片的开关闸门** ✓（`SOKO_ARG_EXPECTED=1|on` ⇒ 开；默认关 ✓）。
fn b1_local_expected(fun: &Expr, scope: &ElabScope<'_>, defs: &DefTable) -> Option<Expr> {
    if !arg_expected_enabled() {
        return None;
    }
    local_arg_expected(fun, scope, defs)
}

/// **头是局部变量**时的实参期望类型（B1 片 · G-30 第 2 轮 ✓）—— **零内核调用、零递归** ✓。
///
/// ## 第 2 轮补的三处（**实测出来的** ✓，见台账 G-30）
///
/// ① **头可以是"局部名的部分应用"** ✗：`ext {a} {b} (fun …)` 展开成
///    `App(App(App(Ident ext, {a}), {b}), (fun …))` ✓ ⇒ 给**第二、第三个**实参算期望类型时，
///    `fun` 是 `App(Ident ext, …)` 而**不是** `Ident` ✗ —— 原实现只认 `Ident` ✗
///    ⇒ **只有第一个实参**拿得到期望类型 ✗。
/// ② **多名一组** ✗：`∀ {A B : Set α}, …` 是**两个名字一组** ✓（`binders.len() == 2` ✓），
///    而原实现只认 `len() == 1` ✗ ⇒ 直接落空 ✗。
/// ③ **保守边界** ✓：要剥到的那个域的**前面**若引用了更早的 binder（`A`/`B` ✓），
///    不代换就**答不对** ✗ ⇒ **宁可不答** ✗（返回 `None` ✓ = 今天的行为 ✓），
///    绝不返回一个含**不在作用域的名字**的"期望类型" ✗（那会让下游 elaborat 到错项 ✗）。
fn local_arg_expected(fun: &Expr, scope: &ElabScope<'_>, defs: &DefTable) -> Option<Expr> {
    // **① 头可以是部分应用** ✓：数一下已经吃掉几个实参 ✓。
    let (name, applied) = match fun {
        Expr::Ident { name, .. } => (name.as_str(), 0usize),
        other => {
            let (head, args) = crate::spine::spine_of(other);
            match head {
                Expr::Ident { name, .. } => (name.as_str(), args.len()),
                _ => return None,
            }
        }
    };
    // 最近的同名 binder（作用域是栈 ✓）
    let idx = scope.names.iter().rposition(|n| n == name)?;
    let mut cur = scope.src_tys.get(idx)?.as_ref()?.clone();
    // 跳到**第 `applied` 个 Π 层**：它的域正是本实参的期望类型 ✓。
    let mut seen = 0usize;
    for _ in 0..8 {
        match &cur {
            Expr::Arrow { domain, codomain, .. } => {
                if seen == applied {
                    return Some(domain.as_ref().clone());
                }
                seen += 1;
                cur = codomain.as_ref().clone();
            }
            Expr::Forall { binders, body, .. } => {
                for b in binders {
                    if seen == applied {
                        // **③ 保守**：本层的域若提到**更早的** binder ⇒ 不代换就答不对 ✗ ⇒ 不答 ✓。
                        let ty = b.ty.as_ref()?;
                        if mentions_any(ty, scope, &binders[..]) {
                            return None;
                        }
                        return Some(ty.as_ref().clone());
                    }
                    seen += 1;
                }
                cur = body.as_ref().clone();
            }
            _ => match crate::spine::unfold_one(&cur, defs, None) {
                Some(next) if next != cur => cur = next,
                _ => return None,
            },
        }
    }
    None
}

/// `e` 里有没有引用**更早的 binder**（保守判据 ✓ —— 只看**裸名** ✓，命中就放弃 ✓）。
fn mentions_any(e: &Expr, scope: &ElabScope<'_>, earlier: &[crate::ast::Binder]) -> bool {
    let mut names: Vec<&str> = earlier.iter().map(|b| b.name.as_str()).collect();
    names.retain(|n| !n.is_empty());
    if names.is_empty() {
        return false;
    }
    // 作用域里**同名**的那些**不是**这一组的 ⇒ 也算"更早的" ✓（保守方向安全 ✓）。
    let _ = scope;
    fn walk(e: &Expr, names: &[&str]) -> bool {
        match e {
            Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => names.contains(&name.as_str()),
            Expr::App { fun, arg, .. } => walk(fun, names) || walk(arg, names),
            Expr::Forall { binders, body, .. } | Expr::Lambda { binders, body, .. } => {
                binders.iter().any(|b| b.ty.as_deref().is_some_and(|t| walk(t, names)))
                    || walk(body, names)
            }
            Expr::Arrow { domain, codomain, .. } => walk(domain, names) || walk(codomain, names),
            Expr::Plus { lhs, rhs, .. } => walk(lhs, names) || walk(rhs, names),
            Expr::Let { val, body, .. } => walk(val, names) || walk(body, names),
            _ => false,
        }
    }
    walk(e, &names)
}

fn needs_expected_type(expr: &Expr) -> bool {
    match expr {
        Expr::SetLiteral { .. } => true,
        // 零元记法：`lhs`/`rhs` 都没有才是（`prefix` 记法有 `rhs`、`postfix` 有 `lhs`，
        // 它们能从前缀/后缀操作数解出参数）。
        Expr::Notation { lhs, rhs, .. } => lhs.is_none() && rhs.is_none(),
        _ => false,
    }
}

/// 应用**最后一个实参**的期望类型：取头的签名望远镜，按位置取第 k 层，并把前面
/// 已经写出的实参代进去（`Eq.symm.{1} (Set α) A ∅ h` ⇒ `∅` 的期望类型是
/// `Set α` 而不是形参名 `α`）。
///
/// 头是任意表达式（`f x ∅` 里的 `f x`）：类型文本由 `judge_infer` 给，签名用
/// [`notation_telescope`] 剥（与记法路径**同一份**机械，两条路的判据不会分叉）。
/// 读不出签名 ⇒ `None`（退回「无期望类型」的既有行为，绝不比今天差）。
fn application_arg_expected<'a>(
    expr: &Expr,
    scope: &ElabScope<'a>,
    ctx: &ElabCtx<'a, '_>,
    // **G-31**：有活环境就**就地**推头的类型（`infer_type_text` 优先就地、失败回落）
    // —— 头的类型以前每次都靠 `judge_infer` 合成一份 `#check` **整份重编前缀**，
    // 而这条调用点在**每个带期望类型的应用**上都会走 ⇒ 它是 `prefix_runs` 的大头之一。
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<Expr> {
    let (head, args) = crate::spine::spine_of(expr);
    if args.is_empty() {
        return None;
    }
    let index = args.len() - 1;
    // ⚠ 慢路文本**必须逐字不变**：`infer_type_text` 内部用的是
    // `&render_expr(operand)`，与这里原来的 `render_expr(head)` **同一份** ✓
    // ⇒ 回落时与今天逐字节相同（缓存键也因此同源 ✓）。
    let ty_text = infer_type_text(ctx, scope, head, env)?;
    let (layers, _) = notation_telescope(&ty_text)?;
    let (_, domain) = layers.get(index)?;
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (k, (name, _)) in layers.iter().take(index).enumerate() {
        if !name.is_empty() {
            sigma.insert(name.clone(), args[k].clone());
        }
    }
    Some(crate::spine::substitute(domain, &sigma))
}

/// 记法操作数里**第一个能定出宇宙层级**的那个，返回层级文本。
///
/// `a = b`（内建记法 → `Eq`）：`a : T` ⇒ `T : Sort u` ⇒ `u`。
/// 例：`a : Set α` ⇒ `Set α : Type 0` ⇒ `u = 1`；`A : Prop` ⇒ `Prop` ⇒ `u = 0`。
/// 逐个操作数试（`Eq`/`Ne` 的类型参数在第一位；换个记法可能在别处）。
///
/// 判据全部问内核（`judge_infer`），**不做文本猜测**：`infer_type_text` 拿
/// 操作数的类型文本，再对那份文本问一次它的类型（= sort）。
fn universe_level_text_of_operands<'a>(
    operands: &[&Expr],
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    // **P1-a**：`Some` ⇒ 第一问（操作数的类型文本）就地答；第二问吃的是**文本**，
    // 天生不适合就地路径（附二 B 表 `1396` 那行）⇒ 保持源码重跑 ✓。
    mut env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<String> {
    for operand in operands {
        let ty_text = infer_type_text(ctx, scope, operand, InplaceEnv::reborrow(&mut env))?;
        // **G-31（第二处，实测未命中大头：unit08 全程 100/226 次）**：第二问要的是
        // **`ty_text` 的类型**（= 它的宇宙）。它以前每次都 `judge_infer` 合成一份
        // `#check`、**整份重编前缀**；这里先试**就地** —— 把类型文本回读成 `#check`
        // 的 AST，再走 `infer_type_text_inplace`（**只就地、失败即 `Err`**）。
        //
        // ⚠ 回落**必须问原来那句**（`ty_text` 本身），**不能**用 `infer_type_text`
        // 的内置回落：那个回落到 `render_expr(operand)` —— 而这里 operand 是**回读**
        // 出来的 AST，重渲染的文本未必与 `ty_text` 逐字相同 ⇒ 问的就不是同一个问题了
        // ✗（缓存键也会分叉）。所以就地失败时**原样**调 `judge_infer(… &ty_text)` ✓。
        let inplace = env.as_deref_mut().and_then(|e| {
            let file = crate::parse_fragment(&format!("#check {ty_text}\n")).ok()?;
            let expr = match file.commands.first()? {
                crate::ast::Command::Check { expr, .. } => expr.clone(),
                _ => return None,
            };
            infer_type_text_inplace(
                e,
                ctx,
                &scope.judge_binder_srcs(),
                &expr,
                scope.judge_binders().len(),
            )
            .ok()
        });
        let sort_text = match inplace {
            Some(text) => text,
            None => match judge_infer(
                ctx.prefix_src,
                ctx.options,
                &scope.judge_binders(),
                &ty_text,
            ) {
                Ok(text) => text,
                Err(_) => continue,
            },
        };
        if let Some(level) = level_text_of_sort(&sort_text) {
            return Some(level);
        }
    }
    None
}

/// 内核 pp 的 sort 文本 → 宇宙层级文本。
///
/// | pp 文本 | 含义 | 层级 |
/// |---|---|---|
/// | `Prop` | `Prop : Sort 0` | `0` |
/// | `Type n` | `Type n : Sort (n+1)` | `n+1` |
/// | `Sort n` | 自身 | `n` |
///
/// `Type u`（层级变量）⇒ `u+1`——`level_ptr` 认得这种文本（与 `Sort (u+1)`
/// 同一条路）。解析不出 ⇒ `None`（调用方退回全 0，与不写 `.{u}` 的点名写法同判）。
pub(crate) fn level_text_of_sort(text: &str) -> Option<String> {
    let text = text.trim();
    if text == "Prop" {
        return Some("0".to_string());
    }
    let rest = text
        .strip_prefix("Type")
        .map(|rest| (rest, 1u64))
        .or_else(|| text.strip_prefix("Sort").map(|rest| (rest, 0u64)))?;
    let (rest, offset) = rest;
    let rest = rest.trim();
    if rest.is_empty() {
        // 裸 `Type` = `Type 0` ⇒ 层级 1；裸 `Sort` 不该出现（内核总写数字）。
        return Some(offset.to_string());
    }
    if let Ok(n) = rest.parse::<u64>() {
        return Some((n + offset).to_string());
    }
    // `Type u` / `Type (u+1)` / `Sort u`：交给 `level_ptr` 的层级算术。
    let inner = rest
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(')'))
        .unwrap_or(rest);
    if offset == 0 {
        Some(inner.to_string())
    } else {
        Some(format!("{inner}+{offset}"))
    }
}

/// 两段式 binder 的 guard 形状判据：`guard` 是**以 binder 名为左操作数**的
/// 记号表达式（`x ∈ s`）。
fn guard_is_binder(guard: &Expr, binder_name: &str) -> bool {
    matches!(
        guard,
        Expr::Notation { lhs: Some(lhs), .. }
            if matches!(lhs.as_ref(), Expr::Ident { name, .. } if name == binder_name)
    )
}

/// `∀ x ∈ s, p` 的 body 是 `Arrow (x ∈ s) p` ⇒ 取出 guard（第三刀 §12.1）。
fn split_arrow_guard<'a>(body: &'a Expr, binder_name: &str) -> Option<&'a Expr> {
    match body {
        Expr::Arrow { domain, .. } if guard_is_binder(domain, binder_name) => Some(domain),
        _ => None,
    }
}

/// `∃ x ∈ s, p` 的 body 是 `And (x ∈ s) p` ⇒ 取出 guard（第三刀 §12.1）。
fn split_and_guard<'a>(body: &'a Expr, binder_name: &str) -> Option<&'a Expr> {
    let Expr::App { fun, .. } = body else {
        return None;
    };
    let Expr::App {
        fun: head,
        arg: guard,
        ..
    } = fun.as_ref()
    else {
        return None;
    };
    let is_and = matches!(head.as_ref(), Expr::Ident { name, .. } if name == "And");
    (is_and && guard_is_binder(guard, binder_name)).then_some(guard)
}

/// **诊断消息里的表达式文本**（T-U11 第一次真迁移 ✓ 2026-09-25）：
/// 走唯一接口 `DisplayNotations::render` ✓（= `fold(render_expr(e))` ✓）；
/// **没有表 ⇒ 原样** ✓（那说明这里不是显示上下文 ✓，沿用 `prelude.rs` 的约定 ✓）。
/// ⚠ 只给**消息**用 ✓ —— 判定/解析路径一律走 `render_expr` ✗（内核红线 ✓）。
fn render_msg(ctx: &ElabCtx<'_, '_>, expr: &Expr) -> String {
    match ctx.notations {
        Some(n) => n.render(expr),
        None => crate::proof::render_expr(expr),
    }
}

/// binder 记法的操作数（`fun (x : A) => …`）在 binder 没写类型时**由 guard
/// 反解**出类型并填进注解（第三刀 §12.1）。不需要动 ⇒ `None`（走原路径）；
/// guard 在、但解不出 ⇒ `elab-binder-notation-unsolved`。
fn binder_notation_operand<'a>(
    symbol: &str,
    operand: Option<&Expr>,
    scope: &ElabScope<'a>,
    known: &KnownTable,
    ctx: &ElabCtx<'a, '_>,
    // **P1-b**：`Some` ⇒ 就地判定（调用方手里有活环境）；`None` ⇒ 今天的老路。
    mut env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Result<Option<Expr>, CompileError> {
    let Some(Expr::Lambda {
        binders,
        body,
        span,
    }) = operand
    else {
        return Ok(None);
    };
    let [binder] = binders.as_slice() else {
        return Ok(None);
    };
    if binder.ty.is_some() {
        return Ok(None);
    }
    let ty = match split_and_guard(body, &binder.name) {
        // 两段式：由 guard 的关系（`∈`）反解；解不出是**专用诊断**（不猜）。
        Some(guard) => {
            let wide = if crate::judge::inplace_wide() {
                InplaceEnv::reborrow(&mut env)
            } else {
                None
            };
            let Some(ty) = guarded_binder_type(guard, &binder.name, scope, known, ctx, wide) else {
                return Err(CompileError::elab(
                    ErrorKind::ElabBinderNotationUnsolved,
                    format!(
                        "binder 记法 `{symbol}` 里 `{}` 的类型从 guard `{}` 反解不出来：给 binder 补类型标注（例如 {symbol} ({} : α) ∈ s, p），或改用点名写法",
                        binder.name,
                        render_msg(ctx, guard),
                        binder.name
                    ),
                    binder.span,
                ));
            };
            ty
        }
        // 一段式 `∃ x, p`：binder **必须自己带类型标注**——记法不引入元变量
        // 与一般合一（第一刀 N4.2 的边界），裸 `∃ x, p` 的 x 类型没有来源。
        // 与语言里既有的 `∀ x, p` 同规则（那边报 `elab-untyped-binder`）。
        None => {
            return Err(CompileError::elab(
                ErrorKind::ElabBinderNotationUnsolved,
                format!(
                    "binder 记法 `{symbol}` 里 `{}` 没有类型：一段式要写标注（{symbol} ({} : α), p），两段式靠 guard（{symbol} {} ∈ s, p）",
                    binder.name, binder.name, binder.name
                ),
                binder.span,
            ));
        }
    };
    let mut binder = binder.clone();
    binder.ty = Some(Box::new(ty));
    Ok(Some(Expr::Lambda {
        binders: vec![binder],
        body: body.clone(),
        span: *span,
    }))
}

/// 集合字面量的**前导参数回退解**（第三刀 §12.4）：期望类型 `Set α₀` ⇒
/// `Set.singleton`/`Set.pair` 的 `α := α₀`。元素自己的类型解不出时（`{∅}`）
/// 只能走这条路——它是"期望类型传播"的又一处具体形态，不是新语义。
fn set_literal_prefix_args(
    target: &str,
    expected_src: Option<&Expr>,
    span: Span,
    known: &KnownTable,
    ctx: &ElabCtx<'_, '_>,
) -> Option<Vec<Expr>> {
    let expected = expected_src?;
    let canonical = resolve_known(known, ctx.ns, target, span).ok()?;
    let text = render_expr(&Expr::Ident {
        name: canonical,
        span,
    });
    let signature = crate::judge::judge_type_of(ctx.prefix_src, ctx.options, &text).ok()?;
    let (layers, result) = notation_telescope(&signature)?;
    let param = layers.first()?.0.clone();
    if param.is_empty() {
        return None;
    }
    let found = unify_extract(&result, expected, &param)?;
    Some(vec![found])
}

/// **两段式 binder 的变量类型反解**（第三刀 §12.1）：`∀ x ∈ s, p` /
/// `∃ x ∈ s, p` 里 x 的类型由 guard 的关系（`∈`）反解——
///
/// 1. 读 guard 目标（`Set.mem`）的签名，拆出 Pi 望远镜；
/// 2. 用 guard 的**其它**操作数（容器 `s`）解前导类型参数（跳过 binder 自己：
///    它还没有类型，问不出类型文本）；
/// 3. binder 那一层的**域**就是 x 的类型（`Set.mem` 的第 2 个参数域是 `α`
///    ⇒ `x : α₀`）。
///
/// 解不出 ⇒ `None`（调用方报专用诊断，绝不猜）。
fn guarded_binder_type<'a>(
    guard: &Expr,
    binder_name: &str,
    scope: &ElabScope<'a>,
    known: &KnownTable,
    ctx: &ElabCtx<'a, '_>,
    // **P1-b**：`Some` ⇒ 就地判定（调用方手里有活环境）；`None` ⇒ 今天的老路。
    mut env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<Expr> {
    let Expr::Notation {
        target,
        lhs,
        rhs,
        span,
        ..
    } = guard
    else {
        return None;
    };
    let operands: Vec<&Expr> = [lhs.as_deref(), rhs.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    if operands.len() != 2 {
        return None;
    }
    let Expr::Ident { name, .. } = operands[0] else {
        return None;
    };
    if name != binder_name {
        return None;
    }
    let canonical = resolve_known(known, ctx.ns, target, *span).ok()?;
    let text = render_expr(&Expr::Ident {
        name: canonical,
        span: *span,
    });
    let signature = crate::judge::judge_type_of(ctx.prefix_src, ctx.options, &text).ok()?;
    let (layers, _) = notation_telescope(&signature)?;
    let missing = layers.len().checked_sub(operands.len())?;
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for i in 0..missing {
        let param = layers[i].0.clone();
        if param.is_empty() {
            return None;
        }
        let mut solved: Option<Expr> = None;
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            // 操作数位 `k = j - missing`；`k == 0` 是 binder 自己（跳过）。
            let Some(k) = j.checked_sub(missing) else {
                continue;
            };
            if k == 0 || !mentions_ident(&layer.1, &param) {
                continue;
            }
            let Some(operand) = operands.get(k) else {
                continue;
            };
            let wide = if crate::judge::inplace_wide() {
                InplaceEnv::reborrow(&mut env)
            } else {
                None
            };
            let Some(actual) = operand_type_expr(ctx, scope, operand, wide) else {
                continue;
            };
            if std::env::var_os("SOKO_SOLVE_STEP").is_some() {
                eprintln!(
                    "SOLVE-STEP param={param:?} layer_domain={:?} operand_ty={:?}",
                    format!("{:?}", layer.1)
                        .chars()
                        .take(110)
                        .collect::<String>(),
                    format!("{:?}", actual)
                        .chars()
                        .take(110)
                        .collect::<String>()
                );
            }
            if let Some(found) = unify_extract(&layer.1, &actual, &param) {
                solved = Some(found);
                break;
            }
        }
        sigma.insert(param, solved?);
    }
    let (_, domain) = layers.get(missing)?;
    Some(super::goals::substitute_names(
        domain,
        &sigma,
        &HashMap::new(),
    ))
}

/// **记法重载的候选选择**（第三刀 §12.2）。
///
/// 单候选 ⇒ 原样返回（零开销；展开路径逐字节等于第二刀）。多候选 ⇒ 按
/// **期望类型**筛：
///
/// 1. 读每个候选的签名（`judge_type_of`，内核 pp），取 telescope 的**结果
///    类型**，与期望类型做**头部匹配**（裸变量模板算通配——v1 不做一般合一）；
/// 2. 恰好一个 ⇒ 选它；
/// 3. 一个都不匹配 ⇒ `elab-notation-no-candidate`（列出每个候选的结果类型）；
/// 4. 还剩 ≥2 个（或压根没有期望类型却有多候选）⇒ `elab-notation-ambiguous`。
///
/// 判据只用**内核给的签名文本**（不比对源文本）：候选的取舍是"这个目标的
/// 结果类型能不能长成期望的样子"，与操作数类型对不对是**两件事**——后者仍由
/// 既有展开路径（`elab-notation-argument-unsolved`）与内核终审。
#[allow(clippy::too_many_arguments)]
fn choose_notation_target<'a>(
    candidates: &[&'a str],
    symbol: &str,
    span: Span,
    known: &KnownTable,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'_, '_>,
) -> Result<&'a str, CompileError> {
    let Some((first, rest)) = candidates.split_first() else {
        return Err(CompileError::elab(
            ErrorKind::ElabNotationUnknownTarget,
            format!("记法 `{symbol}` 没有候选目标"),
            span,
        ));
    };
    if rest.is_empty() {
        return Ok(first);
    }
    let mut results: Vec<(&'a str, Option<Expr>)> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let result = resolve_known(known, ctx.ns, candidate, span)
            .ok()
            .and_then(|canonical| {
                let text = render_expr(&Expr::Ident {
                    name: canonical,
                    span,
                });
                crate::judge::judge_type_of(ctx.prefix_src, ctx.options, &text).ok()
            })
            .and_then(|signature| notation_telescope(&signature).map(|(_, result)| result));
        results.push((candidate, result));
    }
    let viable: Vec<&'a str> = match expected_src {
        Some(expected) => results
            .iter()
            .filter(|(_, result)| {
                result
                    .as_ref()
                    .is_some_and(|result| notation_result_matches(result, expected))
            })
            .map(|(candidate, _)| *candidate)
            .collect(),
        None => candidates.to_vec(),
    };
    match viable.len() {
        1 => Ok(viable[0]),
        0 => Err(CompileError::elab(
            ErrorKind::ElabNotationNoCandidate,
            format!(
                "记法 `{symbol}` 的候选目标（{}）没有一个能对上这里的期望类型：{}",
                candidate_list(candidates),
                describe_candidate_results(ctx, &results)
            ),
            span,
        )),
        _ => Err(CompileError::elab(
            ErrorKind::ElabNotationAmbiguous,
            format!(
                "记法 `{symbol}` 有多个候选目标都能用（{}），{}看不出该选哪个",
                viable.join("、"),
                match expected_src {
                    Some(_) => "期望类型",
                    None => "这里没有期望类型，",
                }
            ),
            span,
        )),
    }
}

/// `Set.mem`、`List.mem` 这样的人话候选清单。
fn candidate_list(candidates: &[&str]) -> String {
    candidates
        .iter()
        .map(|candidate| format!("`{candidate}`"))
        .collect::<Vec<_>>()
        .join("、")
}

/// `Set.mem → Prop；Set.singleton → Set α` 这样的候选结果类型清单（读不到
/// 签名的候选标 `?`）。
/// **候选结果的消息文本**（T-U11 ✓）：走 `render_msg` ✓ ⇒ 有记法表就折 ✓
/// （这串文本是**给学习者看的诊断** ✓ —— 用户硬规则：用户可见文本必须折 ✓）。
fn describe_candidate_results(ctx: &ElabCtx<'_, '_>, results: &[(&str, Option<Expr>)]) -> String {
    results
        .iter()
        .map(|(candidate, result)| match result {
            Some(result) => format!("`{candidate}` 的结果类型是 `{}`", render_msg(ctx, result)),
            None => format!("`{candidate}` 的签名读不到"),
        })
        .collect::<Vec<_>>()
        .join("；")
}

/// 候选的**结果类型**与期望类型的头部匹配（v1 只做一层：头同名 + 实参个数
/// 相同；模板是裸变量 `α` 时算通配）。与 `unify_extract` 同族的"不做一般
/// 合一"取舍——宁可判**歧义**（报专用码 + 教点名写法），也不猜。
fn notation_result_matches(template: &Expr, expected: &Expr) -> bool {
    if let Expr::Ident { .. } = template {
        return true;
    }
    if let (
        Expr::Arrow {
            domain: template_domain,
            codomain: template_codomain,
            ..
        },
        Expr::Arrow {
            domain: expected_domain,
            codomain: expected_codomain,
            ..
        },
    ) = (template, expected)
    {
        return notation_result_matches(template_domain, expected_domain)
            && notation_result_matches(template_codomain, expected_codomain);
    }
    let (template_head, template_args) = crate::spine::spine_of(template);
    let (expected_head, expected_args) = crate::spine::spine_of(expected);
    if template_args.len() != expected_args.len() {
        return false;
    }
    match (template_head, expected_head) {
        (Expr::Ident { name: t, .. }, Expr::Ident { name: e, .. }) => {
            t == e
                && template_args
                    .iter()
                    .zip(expected_args.iter())
                    .all(|(t, e)| notation_result_matches(t, e))
        }
        _ => false,
    }
}

fn judgement_message(j: &crate::judge::Judgement) -> String {
    match j {
        crate::judge::Judgement::Error { message, .. } => message.clone(),
        crate::judge::Judgement::Mismatch { expected, actual } => {
            format!("期望 `{expected}`，实际是 `{actual}`")
        }
        crate::judge::Judgement::Match => "类型推断没有给出类型".to_string(),
    }
}

/// 补出目标 telescope 的**前导参数**（设计 N4.2 的裸变量匹配）。
///
/// 返回 `None` ⇒ 补不出（调用方报 `elab-notation-argument-unsolved`）。
/// 返回 `Some(vec![])` ⇒ 不需要补（目标参数个数正好等于操作数个数）。
///
/// 两条求解路径，都是同一个**头部匹配 + 从实参位提取裸变量**：
///
/// ① **由操作数解出**：找第一个 `j > i` 且域里提到参数名 `n` 的 binder，
///    把它的域与「第 j 个 binder 对应的那个操作数的类型」头部匹配
///    （`Set.mem` 的第 2 个参数域是裸变量 `α` ⇒ `α := typeof(a)`；
///    `Set.subset` 的第 2 个参数域是 `Set α`、`typeof(A) = Set α₀`
///    ⇒ `α := α₀`）。
/// ② **由期望类型解出**（零操作数时唯一的路）：把已解出的参数代进 telescope
///    剩余部分，与期望类型头部匹配（`Set.empty : (α) → Set α` 对上
///    `Set α₀` ⇒ `α := α₀`）。
fn notation_prefix_args<'a>(
    signature: &str,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    // **P1-b**：`Some` ⇒ 就地判定（调用方手里有活环境）；`None` ⇒ 今天的老路。
    mut env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Result<Option<Vec<Expr>>, CompileError> {
    let Some((layers, result)) = notation_telescope(signature) else {
        return Ok(None);
    };
    // 操作数对齐到**最后** `operands.len()` 个参数；多出来的前导参数要补。
    let Some(max_missing) = layers.len().checked_sub(operands.len()) else {
        // 操作数比参数还多：交给既有应用路径报错（elab/kernel 的既有诊断）。
        return Ok(Some(Vec::new()));
    };
    if max_missing == 0 {
        return Ok(Some(Vec::new()));
    }
    // **从最大候选往下试**（第二刀）：`peel_pi` 把**结果类型里的箭头**也算成
    // 参数层——`compl : (α : Type) → (α -> Prop) → α -> Prop` 数出 3 层，而操作数
    // 只有 1 个 ⇒ 最大的候选把参数位对到结果的箭头上，症状是"要补的位没有名字"
    // （`peel_pi` 对箭头给的名字是空串）。所以逐个往下试，取**第一个能完整解出**
    // 的候选；"前导参数必须有名字"这条不变量正好把多出来的结果层筛掉。
    for missing in (1..=max_missing).rev() {
        if let Some(solved) = solve_prefix_args(
            &layers,
            &result,
            missing,
            operands,
            expected_src,
            ctx,
            scope,
            InplaceEnv::reborrow(&mut env),
        ) {
            return Ok(Some(solved));
        }
        // **E19 刀1：待定参数（`SOKO_NOTATION_METAVAR`，2026-10-01 起默认开）** ——
        // 只放宽**最大候选**这一读（操作数对齐到**最后** `operands.len()` 层，
        // 也就是语义上正确的那一读）。更小的候选是"错位读法"（实测 `∅ ≈ {b}`
        // 会掉到 `missing=1`：把 `{b}` 对到 `A : Set α` 上，解出 `α := β`，
        // 再让 `∅` 拿到期望类型 `Type` ⇒ 报"补不出参数" ✗）⇒ **不在那里放宽** ✓。
        if crate::compile::implicit::metavar_enabled() && missing == max_missing {
            // **档位**（IA-4 M1）：`Sibling` = E19 的窄版（默认，逐字节等于今天）；
            // `Engine` = 新引擎（`crate::compile::meta`：真元变量 + 三值合一 + occurs/作用域 +
            // 有界待定 + 出口 zonk）。**两条都只放宽"最大候选"这一读** ✓。
            let pending = if crate::compile::implicit::metavar_mode()
                == crate::compile::implicit::MetavarMode::Engine
            {
                solve_prefix_args_meta(
                    &layers,
                    &result,
                    missing,
                    operands,
                    expected_src,
                    ctx,
                    scope,
                    InplaceEnv::reborrow(&mut env),
                )
            } else {
                solve_prefix_args_pending(
                    &layers,
                    &result,
                    missing,
                    operands,
                    expected_src,
                    ctx,
                    scope,
                    InplaceEnv::reborrow(&mut env),
                )
            };
            if let Some(solved) = pending {
                return Ok(Some(solved));
            }
        }
    }
    Ok(None)
}

/// `signature` 的 Pi 望远镜：`(名字, 域)` 逐层 + 余下的结果类型。解析不出 ⇒ `None`。
fn notation_telescope(signature: &str) -> Option<(Vec<(String, Expr)>, Expr)> {
    let sig = crate::proof::parse_expr_text(signature).ok()?;
    let mut layers: Vec<(String, Expr)> = Vec::new();
    let mut result = sig;
    while let Some(pi) = crate::spine::peel_pi(&result) {
        layers.push((pi.name, pi.domain));
        result = pi.body;
    }
    Some((layers, result))
}

/// 固定 `missing` 时解前导参数；解不出（或某一位**没有名字**）⇒ `None`。
///
/// **严格档**（`allow_pending = false`）：既有行为，逐字节不变 —— 任何一位解不出
/// 就整体 `None`（调用方报专用错误码，**不猜**）✓。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_args<'a>(
    layers: &[(String, Expr)],
    result: &Expr,
    missing: usize,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<Vec<Expr>> {
    solve_prefix_args_impl(
        layers,
        result,
        missing,
        operands,
        expected_src,
        ctx,
        scope,
        env,
        false,
    )
}

/// **E19 刀1 的待定参数档**（开关 [`crate::compile::implicit::metavar_enabled`]，
/// 与刀2 的一般路径**共用同一个开关**，**默认开**；逃生门 `SOKO_NOTATION_METAVAR=0`）：
/// 某一位解不出时**不立刻失败**，先记成
/// **待定**（`?α`），等所有位都走完再用
/// [`crate::compile::implicit::fill_pending_by_shape`] 把待定位与**同形的已解兄弟**合一 ✓。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_args_pending<'a>(
    layers: &[(String, Expr)],
    result: &Expr,
    missing: usize,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<Vec<Expr>> {
    solve_prefix_args_impl(
        layers,
        result,
        missing,
        operands,
        expected_src,
        ctx,
        scope,
        env,
        true,
    )
}

/// **IA-4 M1 的引擎档**（开关 `SOKO_METAVAR=engine`）：把记法前导参数的求解换成
/// [`crate::compile::meta::MetaCtx`] 上的**真元变量 + 三值合一**。
///
/// 与 [`solve_prefix_args_pending`] 的**约束同源**（不再一位一位贪心）：
/// ① 每个前导位建一个元变量（作用域 = **更晚**的望远镜参数名）；② 望远镜名 → 元变量（模板代换）；
/// ③ **每个操作数只问一次**类型（窄版是在 `(i,j)` 双重循环里反复问）；④ 每个后续层的域 ≟ 对应操作数
/// 的类型；⑤ 结果 ≟ 期望类型；⑥ 不动点 + defaulting + zonk（出口无残留自检在 `discharge` 里）。
///
/// **delta 展开兜底由引擎内部做**（M0 的 S13 硬约束：不接它就会把「两条约束 defeq 一致」误判成
/// 刚性冲突 ✗）。**元变量不进项**：`discharge` 解不出就返回 `None`，调用方照旧报既有码 ✓。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_args_meta<'a>(
    layers: &[(String, Expr)],
    result: &Expr,
    missing: usize,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    mut env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<Vec<Expr>> {
    // 与窄版同一条闸门：前导位**必须都有名字**（`peel_pi` 对结果里的箭头给空名）。
    if layers.iter().take(missing).any(|l| l.0.is_empty()) {
        return None;
    }
    let unfold = |e: &Expr| {
        crate::spine::unfold_to_inductive(
            e,
            &|n| ctx.inductives.get(n).is_some(),
            ctx.defs,
            8,
            None,
            crate::spine::UnfoldAlign::Short,
        )
    };
    // **IA-4 B2 第 1 步（2026-10-05 ✓）**：store 从 `MetaCtx` 里抽出来了 ✓ ——
    // 今天仍**一次求解一个** ✓（行为逐字节不变 ✓）；B2 的下一步会把它挂到**声明级** ✓
    // （用户 00:05：「允许活过一次求解调用」✓）。
    let mut store = crate::compile::meta::MetaStore::default();
    let mut meta = crate::compile::meta::MetaCtx::new(&unfold, &mut store);
    // ⓪ **望远镜名先换成 fresh 名**（与 `implicit::telescope` 的防捕获纪律对齐）：
    // `notation_telescope` 用的是**签名原文名**（`α`/`β`/`A`/`B`）⇒ 拿它当"作用域外"判据会与
    // **用户变量撞名**（实测：`theorem t (α β : Type) (b : β) : ¬ (∅ ≈ {b})` 里把 `β` 误判成
    // 越界 ⇒ 引擎档把 G-48 判红 ✗）。fresh 名（`\0soko_mp{i}`）让模板、作用域判据与用户名字**永不撞车**。
    let mut ren: HashMap<String, Expr> = HashMap::new();
    let mut tl: Vec<(String, Expr)> = Vec::with_capacity(layers.len());
    for (i, (name, dom)) in layers.iter().enumerate() {
        let dom = crate::spine::substitute(dom, &ren);
        let fresh = format!("\u{0}soko_mp{i}");
        let kept = if name.is_empty() {
            String::new()
        } else {
            ren.insert(
                name.clone(),
                Expr::Ident {
                    name: fresh.clone(),
                    span: Span::default(),
                },
            );
            fresh
        };
        tl.push((kept, dom));
    }
    let result = crate::spine::substitute(result, &ren);
    // ① 元变量（作用域 = 更晚的望远镜参数名 ⇒ 赋值时不许提到它们）
    let mut ids = Vec::with_capacity(missing);
    for i in 0..missing {
        let out_of_scope: Vec<String> = tl
            .iter()
            .skip(i + 1)
            .map(|l| l.0.clone())
            .filter(|n| !n.is_empty())
            .collect();
        ids.push(meta.fresh(
            tl[i].1.clone(),
            crate::compile::meta::MetaKind::Natural,
            out_of_scope,
        ));
    }
    // ② 望远镜名 → 元变量
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        sigma.insert(tl[i].0.clone(), meta.meta_expr(*id));
    }
    // ③ 操作数类型（每个操作数只问一次）
    let mut arg_tys: Vec<Option<Expr>> = Vec::with_capacity(operands.len());
    for operand in operands {
        let wide = if crate::judge::inplace_wide() {
            InplaceEnv::reborrow(&mut env)
        } else {
            None
        };
        arg_tys.push(operand_type_expr(ctx, scope, operand, wide));
    }
    // ④ 约束①：后续层的域（已代换）≟ 该操作数的类型
    for i in 0..missing {
        let name = tl[i].0.clone();
        for (j, layer) in tl.iter().enumerate().skip(i + 1) {
            let Some(actual) = j
                .checked_sub(missing)
                .and_then(|x| arg_tys.get(x))
                .and_then(|t| t.as_ref())
            else {
                continue;
            };
            if !crate::spine::mentions(&name, &layer.1) {
                continue;
            }
            let template = crate::spine::substitute(&layer.1, &sigma);
            if meta.unify(&template, actual) == crate::compile::meta::Tri::No {
                return None;
            }
        }
    }
    // ⑤ 约束②：结果（已代换）≟ 期望类型
    if let Some(expected) = expected_src {
        let template = crate::spine::substitute(&result, &sigma);
        if meta.unify(&template, expected) == crate::compile::meta::Tri::No {
            return None;
        }
    }
    // ⑥ 不动点 + defaulting（E19 的选择规则）+ zonk（M3：出口带通道，本路径暂只用"成没成"）
    let out = meta.discharge(&ids).into_option();
    if std::env::var_os("SOKO_META_DEBUG").is_some() {
        eprintln!(
            "[meta] missing={missing} ids={} solved={:?} unsolved={:?}",
            ids.len(),
            out.as_ref().map(|v| v.len()),
            meta.unsolved().len()
        );
        for (i, id) in ids.iter().enumerate() {
            // ⚠ 这里**不许**用 `render_expr`：那是绕过唯一记法接口的路径
            // （`scripts/audit-notation-paths.py` 会判"新增绕过" ✗）⇒ 打 `Expr` 的 Debug 即可。
            eprintln!("[meta]   #{i} value={:?}", meta.value(*id));
        }
    }
    out
}

/// 固定 `missing` 时解前导参数；解不出（或某一位**没有名字**）⇒ `None`。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_args_impl<'a>(
    layers: &[(String, Expr)],
    result: &Expr,
    missing: usize,
    operands: &[&Expr],
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    mut env: Option<&mut InplaceEnv<'_, 'a>>,
    allow_pending: bool,
) -> Option<Vec<Expr>> {
    // 只在参数是**显式**形态时补：隐式 binder（`{α : Type}`）在 v1 的展开里
    // 不插实参（语言不插入隐式实参，设计 §1 第 3 条）。
    //
    // **`Option` 槽**（E19 刀1）：严格档里它**永远全是 `Some`**（解不出就提前
    // `return None`）⇒ 与改动前的 `Vec<Expr>` 逐字节同行为 ✓；待定档里
    // `None` = "这一位待定" ✓。
    // **路线③（G-42 / G-62 ✓，2026-10-03）**：实参个数**多于显式层数**时 ✓，末尾多出来的那些
    // 属于"**把结果函数又应用了一次**"（`Set.univ x` ✓ / `some Nat` ✓），**不是**在填前导隐式 ✗。
    // ⚠ 安全边界：`surplus == 0` 时下面那个 `filter` 是**恒真**的 ✓ ⇒ 今天能过的程序
    // **逐字节不变** ✗（这条是硬约束 ✓）。
    let explicit_layers = layers.len().saturating_sub(missing);
    let surplus = operands.len().saturating_sub(explicit_layers);
    let usable = operands.len().saturating_sub(surplus);
    let mut solved: Vec<Option<Expr>> = Vec::with_capacity(missing);
    for i in 0..missing {
        let name = layers[i].0.clone();
        if name.is_empty() {
            return None;
        }
        // ① 由操作数解出：第一个提到 `name` 的**后续** binder 的域。
        let mut arg: Option<Expr> = None;
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            // 操作数对齐到**最后** `operands.len()` 层；`j < missing` 的层是
            // 还没解出的前导参数，没有操作数可问（第二刀实测：`Set.image`
            // 有 2 个前导参数，`j - missing` 在 j=1 时会下溢）。
            let Some(operand) = j
                .checked_sub(missing)
                .filter(|k| *k < usable) // ← 路线③：只在**前 `usable` 个**实参里找 ✓
                .and_then(|k| operands.get(k))
            else {
                continue;
            };
            if !mentions_ident(&layer.1, &name) {
                continue;
            }
            let wide = if crate::judge::inplace_wide() {
                InplaceEnv::reborrow(&mut env)
            } else {
                None
            };
            let Some(actual) = operand_type_expr(ctx, scope, operand, wide) else {
                continue;
            };
            if std::env::var_os("SOKO_OUT_DEBUG").is_some() && name.starts_with('\u{0}') {
                eprintln!(
                    "OUT-DEBUG name={name} layer={:?} actual={:?}",
                    format!("{:?}", layer.1)
                        .chars()
                        .take(90)
                        .collect::<String>(),
                    format!("{:?}", actual)
                        .chars()
                        .take(130)
                        .collect::<String>()
                );
            }
            if let Some(found) = unify_extract(&layer.1, &actual, &name) {
                arg = Some(found);
                break;
            }
            // **delta 展开兜底**（R5，2026-09-26）：`implicit::solve_prefix` 早就有
            // 这一条（它展开的是**实参**侧），记法这条路线**两边都可能要展开**：
            //   · 实参侧：`Set.mem α a (Set.union α A B)`（def 头）⇒ 展开到 `Or`；
            //   · **模板侧**：`Set.inter` 的 α 层域是 `Set α`（`Set` 是 **def**），
            //     而操作数的类型文本是**箭头形态**（`Set (Set Nat)` 的 pp 就是
            //     `Set Nat -> Prop`）⇒ `App(Set, α)` 与 `Arrow{…}` 头对不上 ⇒
            //     不展开就报 `elab-notation-argument-unsolved`（实测：`{∅} ∩ {…}`
            //     这一族——unit12 的 `soko:notation-ok: R5` 标记正是它）。
            // **只加解、不改既有解**：先按原样试，失败才展开。
            let unfold = |e: &Expr| {
                crate::spine::unfold_to_inductive(
                    e,
                    &|n| ctx.inductives.get(n).is_some(),
                    ctx.defs,
                    8,
                    None,
                    crate::spine::UnfoldAlign::Short,
                )
            };
            let unfolded_actual = unfold(&actual);
            if unfolded_actual != actual {
                if let Some(found) = unify_extract(&layer.1, &unfolded_actual, &name) {
                    arg = Some(found);
                    break;
                }
            }
            let unfolded_template = unfold(&layer.1);
            if unfolded_template != layer.1 {
                if let Some(found) = unify_extract(&unfolded_template, &actual, &name) {
                    arg = Some(found);
                    break;
                }
                if let Some(found) = unify_extract(&unfolded_template, &unfolded_actual, &name) {
                    arg = Some(found);
                    break;
                }
            }
        }
        // ② 由期望类型解出：把已解出的参数代入 telescope 剩余部分。
        //
        // **delta 展开兜底（R5，2026-09-26）**：`∅`（零元记法）只能走这条路，
        // 而它的模板是 `Set α`（`Set` 是 **def**），期望类型却常常是**箭头形态**
        // ——`Set (Set Nat)` 的 pp 就是 `Set Nat -> Prop`，`Set Nat` 的是
        // `Nat -> Prop` ⇒ `App(Set, α)` 与 `Arrow{…}` **头对不上** ⇒ 不展开就报
        // 「记法 `∅` 展开成 `Set.empty` 时补不出前面的类型参数」✗
        // （实测：`{∅} ∩ {(Set.univ Nat)}` 这一族 —— unit12 的
        // `soko:notation-ok: R5` 标记正是它）。
        // **展开的是模板侧**（`Set α` ⇒ `Arrow{α, Prop}`）：把期望侧展开成
        // `Set` 的形状是做不到的（没有"反向折叠"），而模板展开后两边同形 ✓。
        if arg.is_none() {
            if let Some(expected) = expected_src {
                let rest = substitute_prefix_params(layers, &solved, i, result);
                arg = unify_extract(&rest, expected, &name);
                let unfold = |e: &Expr| {
                    crate::spine::unfold_to_inductive(
                        e,
                        &|n| ctx.inductives.get(n).is_some(),
                        ctx.defs,
                        8,
                        None,
                        crate::spine::UnfoldAlign::Short,
                    )
                };
                if arg.is_none() {
                    let unfolded = unfold(&rest);
                    if unfolded != rest {
                        arg = unify_extract(&unfolded, expected, &name);
                        if arg.is_none() {
                            let unfolded_expected = unfold(expected);
                            if unfolded_expected != *expected {
                                arg = unify_extract(&unfolded, &unfolded_expected, &name);
                            }
                        }
                    }
                }
            }
        }
        match arg {
            Some(value) => solved.push(Some(value)),
            // **待定档**：记成 `None`，等 `fill_pending_by_shape` 合一 ✓。
            None if allow_pending => solved.push(None),
            // **严格档**（既有行为）：一位解不出 ⇒ 整体失败，调用方报专用码 ✓。
            None => return None,
        }
    }
    if allow_pending {
        crate::compile::implicit::fill_pending_by_shape(layers, &mut solved)?;
    }
    // 还有 `None` ⇒ 整体失败（严格档恒不成立；待定档 = "一个兄弟都借不到"）✓。
    solved.into_iter().collect()
}

/// 目标 telescope 里**操作数位**的期望类型（源级 AST），按已解出的前导参数
/// 代换：`Set.mem : (α) → (a : α) → (A : Set α) → Prop`、`α := Nat`
/// ⇒ `[Nat, Set Nat]`。解析不出时该位为 `None`（操作数照旧无期望类型地
/// elaborate，与今天的行为一致——绝不比既有路径差）。
fn notation_operand_expected(
    signature: &str,
    prefix_args: &[Expr],
    operand_count: usize,
) -> Vec<Option<Expr>> {
    let mut out: Vec<Option<Expr>> = vec![None; operand_count];
    let Some((layers, _)) = notation_telescope(signature) else {
        return out;
    };
    // 与 `notation_prefix_args` 同一条对齐规则：`missing` 取**已解出的前导参数
    // 个数**（它经过候选搜索，是权威值；再自己算一遍会把结果类型的箭头又算进去）。
    let missing = prefix_args.len();
    if layers.len() < operand_count {
        return out;
    }
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (k, arg) in prefix_args.iter().enumerate() {
        if let Some((name, _)) = layers.get(k) {
            sigma.insert(name.clone(), arg.clone());
        }
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let Some((_, domain)) = layers.get(missing + i) else {
            continue;
        };
        *slot = Some(super::goals::substitute_names(
            domain,
            &sigma,
            &HashMap::new(),
        ));
    }
    out
}

/// 把 `solved` 里已经解出的前导参数代入 telescope 的第 `i+1` 个 binder 起
/// 的剩余部分（**不含**正在求解的第 `i` 层：那一层正是要被消掉的），得到「结果类型」模板：`(α : Type) → Set α` 代入 `α := α₀`
/// ⇒ `Set α₀`。
fn substitute_prefix_params(
    layers: &[(String, Expr)],
    solved: &[Option<Expr>],
    i: usize,
    result: &Expr,
) -> Expr {
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    // 待定位（`None`）**没有值可代** ⇒ 跳过 ✓（严格档里不存在待定位）。
    for (k, arg) in solved.iter().enumerate() {
        if let Some(arg) = arg {
            sigma.insert(layers[k].0.clone(), arg.clone());
        }
    }
    let mut rest = result.clone();
    for (name, domain) in layers[(i + 1).min(layers.len())..].iter().rev() {
        rest = Expr::Forall {
            binders: vec![Binder {
                name: name.clone(),
                ty: Some(Box::new(super::goals::substitute_names(
                    domain,
                    &sigma,
                    &HashMap::new(),
                ))),
                style: BinderKind::Explicit,
                span: Span::default(),
            }],
            body: Box::new(rest),
            span: Span::default(),
        };
    }
    super::goals::substitute_names(&rest, &sigma, &HashMap::new())
}

/// **就地判定那一刻手里的活环境**（P1-a 第一步，2026-09-29）。
///
/// **为什么需要它**：`judge_infer` 只吃 `prefix_src: &str` ⇒ 每次未命中都要把
/// **整段前缀**合成文件、从零重跑一趟 pass（解析 + elaborate + 内核检查）。
/// 实测（冷缓存 · 1 job · 全课）：`JUDGE_PREFIX runs=3759 / bytes=174213583`
/// —— 而只有 **488 个不同前缀** ⇒ **7.7× 纯重复**。其中
/// **`infer_type_text` 一个判定点就占 2697 趟（72%）**，
/// 而它的调用方（`try_implicit_application` / `elab_notation`）**手里本来就有**
/// 当前的 `EnvBuilder` 与 `KnownTable` ✓。
///
/// **只装"读环境 + 写项"需要的那两样**：`builder`（同一个 `dag` ⇒
/// 指针同一性保住 ✓，这正是 §17 否掉"重建 builder"那条红线）、`known`（名字解析表）。
///
/// ⚠ **`None` = 走今天的老路**（源码重跑）⇒ 回退零成本、语义零变化 ✓。
/// 就地路径**答不出**的原因（诊断用；`On` 档一律当"答不出"⇒回退源码重跑）。
///
/// 直方图由 `SOKO_JUDGE_INPLACE=shadow` 打印（`JUDGE_INPLACE_WHY`）——
/// 它是"这条路为什么不够快"的**唯一**直接证据（本步就是靠它定位到
/// `Parse=77822` 那次分叉的 ✓）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InplaceFail {
    /// 在活环境上 elaborate **某个 binder 的源类型**失败 / 内核 panic。
    ElabBinder,
    /// 在活环境上 elaborate **项本身**失败 / 内核 panic。
    ElabOperand,
    /// 求类型那一步被内核拒绝（panic）。
    Kernel,
}

pub(crate) struct InplaceEnv<'e, 'a> {
    pub builder: &'e mut EnvBuilder<'a>,
    pub known: &'e KnownTable,
}

impl<'e, 'a> InplaceEnv<'e, 'a> {
    /// 在循环里把 `Option<&mut Self>` **重借**一次（它不是 `Copy`）。
    ///
    /// 为什么单开一个方法：clippy 的 `option_as_ref_deref` 建议 `.as_deref_mut()`，
    /// 但那要求 `Self: DerefMut` —— 这里要的是**整个 `InplaceEnv`** 的可变重借，
    /// 不是它内部某个字段的 ⇒ 只有这一处的 `#[allow]`，理由写在这里。
    #[allow(clippy::option_as_ref_deref)]
    pub(crate) fn reborrow<'x>(env: &'x mut Option<&mut Self>) -> Option<&'x mut Self> {
        env.as_mut().map(|e| &mut **e)
    }
}

/// **就地求类型文本**：在**活环境**上 elaborate，不再合成整段前缀。
///
/// 三步，每一步都刻意与"源码重跑那条路"**逐字对齐**：
/// 1. **造项**：**直接用源 AST**（`scope` 里各 binder 的源类型 + 调用点的
///    `operand`）造出与慢路同形的 `fun (b1:T1) … => term`；
/// 2. **elaborate**：形态对齐 `Walk::check`（`#check` 命令的处理器）——
///    **空 `UnivMap` + scratch hovers** ✓；
/// 3. **求类型**：`ExportFile::infer_type_text_at` —— 与 `kernel_phase` 里
///    `PendingOp::Check` 的 `with_tc(ByIndex(env_at)) + infer_closed_type + pp_expr`
///    **同一条内核调用** ✓；binder 剥离复用 `judge::peel_binders` ✓。
///
/// ## ⚠ 第 ① 步为什么**必须**避开 render→回读（两条都是实测踩到的死路）
///
/// * **只用 `proof::parse_expr_text` 解析查询文本** ⇒ 那是**内核 pp 文本**的回读
///   入口，**不认识前缀里声明的源级记法**（`∈` / `ᶜ` / `''` / `⁻¹'`）⇒
///   `unit12-solution` 单文件实测 **77822 次 `Parse` 失败**（占全部分叉的 88%）✗；
/// * **改成"接上整段前缀再 `parse_fragment`"**（慢路就是这么解析的）⇒ 解析**对**了，
///   但每一问都要解析 **≈46 KB 前缀**；而本路径对**每一次** `infer_type_text`
///   都生效（含十几万次缓存命中的调用，慢路那边它们是**不花前缀钱**的）
///   ⇒ 实测 **300 s 都跑不完** ✗✗。
///
/// ⇒ 只有"**不解析**"配得上这条路径：成本只随**项**大小走，**不随前缀走** ✓。
///
/// ⚠ **失败就返回 `Err(reason)`**（= "这条环境答不了"）⇒ 调用方**必须**回退到源码
/// 重跑，与 `EnvProvider::infer_type_text` 的 `None` 约定一致 ✓。
/// **`judge_render_type` 的就地实现**（P1-b 第二刀，2026-09-30）。
///
/// 与 [`infer_type_text_inplace`] **同形**（照抄那三步，含两个实测坑），
/// 差别在**收尾**：慢路是把内核 pp 文本交给 `judge::peel_binders` **在文本上剥**，
/// 而就地路**在项层面剥完再 pp**（`infer_type_text_at_peeled`）✓。
///
/// ## ⚠ 为什么必须在项层面剥（附十的根因）
///
/// `judge::peel_binders` 的第一步是 `parse_expr_text`，它**不认前缀里声明的源级
/// 记法**，而且失败时是 `break`（**静默**原样返回）⇒ 就地路拿到的是**未剥过**的
/// 原始类型文本（`… -> Set.mem α h (Set.empty α) -> …` 里就有 `Set.empty` 这类
/// 点名），**一层都剥不掉** ⇒ 交出去一个多层的函数类型 ⇒ 判定分叉
/// （实测：全课程 38 个文件从 `compiled` 变 `failed`）。
/// ⇒ 在**项层面**沿 `Pi.body` 走指针（零解析、零记法风险），**剥完再 pp** ✓。
///
/// ## 三条走不通的死路（别再试）
///
/// ① 手搓 `mk_lambda(nm, Default, ty, ty)`（体错传成 `ty`）⇒ 推断成 `T -> T` ✗；
/// ② 手搓 `mk_lambda(nm, Default, ty, mk_var(0))` ⇒ `eval: loose bvar` **每趟 panic**
///    （`infer_type_text_at` 走 `infer_closed_type`，那个 `mk_var(0)` 在它自己的
///    局部上下文里没有条目）；
/// ③ 把声明 binder 先 `push` 进 `ElabScope` 再 elaborate `ty` ⇒ 产物是**开项**
///    ⇒ 还是 `loose bvar` ✗（最隐蔽：scope 看着"更完整"，其实正是病根）。
///
/// **唯一走通的**：把查询写成**一整条源级 λ**（`fun (b1:T1) … (bn:Tn)
/// (__soko_render : ty) => __soko_render`），交给 `elab_expr` 的 `Expr::Lambda`
/// 分支 —— de Bruijn 的推入/抬升全由它负责 ⇒ 产物必然是**闭项** ✓。
/// 这也正是慢路在跑的东西（那边是同一形状的**文本**）⇒ 两条路同源 ✓。
pub(crate) fn inplace_render_type<'a>(
    env: &mut InplaceEnv<'_, 'a>,
    ctx: &ElabCtx<'a, '_>,
    binders: &[crate::ast::Binder],
    ty: &Expr,
) -> Option<String> {
    // 每个"放弃"都记一笔原因 —— 否则"就地没生效"只能靠猜 ✗
    // （P1-a 就是靠这组原因才发现"夹具没踩到接线点"的）。
    macro_rules! give_up {
        ($why:expr) => {{
            crate::judge::stats::note_by_reason($why);
            return None;
        }};
    }

    const RENDER: &str = "__soko_render";
    let mut telescope: Vec<crate::ast::Binder> = binders.to_vec();
    telescope.push(crate::ast::Binder {
        name: RENDER.to_string(),
        ty: Some(Box::new(ty.clone())),
        style: crate::ast::BinderKind::Explicit,
        span: ty.span(),
    });
    let query = Expr::Lambda {
        binders: telescope,
        body: Box::new(Expr::Ident {
            name: RENDER.to_string(),
            span: ty.span(),
        }),
        span: ty.span(),
    };
    // **空 `UnivMap` + scratch hovers**：与 `#check` 的处理器（`Walk::check`）
    // 逐字一致（`#check` 没有宇宙参数可解 ⇒ 读到宇宙变量的查询在两条路上一样失败 ✓）。
    let no_universe: UnivMap<'_> = UnivMap::new();
    let mut scratch_hovers: Vec<HoverNode<'a>> = Vec::new();
    let mut sc = ElabScope::new();
    let term = match quiet_catch(|| {
        elab_expr(
            env.builder,
            &query,
            &mut sc,
            &no_universe,
            env.known,
            &mut scratch_hovers,
            None,
            None,
            ctx,
        )
    }) {
        Ok(Ok(ptr)) => ptr,
        Ok(Err(_)) => give_up!("query-elab"),
        Err(_) => give_up!("query-panic"),
    };
    // **项层面剥掉整条望远镜**（`n` 个声明 binder + 1 个 `__soko_render`），
    // 再 pp ⇒ 交出去的形态与"慢路先 pp 再文本剥 `n+1` 层"**同构** ✓。
    let limit = sokonanoda::env::EnvLimit::ByIndex(env.builder.declaration_count());
    let text = env.builder.with_env(|ef| {
        // ⚠ **必须与 `kernel_phase` 的 `#check` 同档**（`proofs = true`）：
        // 默认 `proofs = false` 时 pp 会对**每个子项**调 `is_proof`（用**空局部
        // 上下文**推类型）⇒ binder 内的 `Eq n m` 直接 `loose bvar in infer` panic ✗
        // （P1-a 实测踩到 7 次，不是理论风险）。
        ef.config.pp_options.proofs = true;
        // ⚠ **`explicit` 也必须与 `#check` 出口同款**（G-71 ✗→✓，2026-10-04）：
        // `kernel_phase.rs:463-471` 在 `#check` 里把 `pp_options.explicit` 设成
        // `judge::explicit_pp_active()`、出完立刻还原 ✓；就地路**漏了这一行** ✗
        // ⇒ 同一问在两条路上打出**不同的文本**（`@Exists A q` vs `Exists A q` ✓）
        // ⇒ 下游按文本做的判断会**分叉**（实测 shadow `shadow_diff=855,317` ✗✗）。
        ef.config.pp_options.explicit = crate::judge::explicit_pp_active();
        // ⚠ **剥 `n` 层，不是 `n+1`** —— 必须与慢路**对称**：
        //   * 慢路：`judge_infer` 剥掉 `n` 层声明 binder（`judge_infer_uncached`
        //     结尾），**留下 `ty -> ty`**，再由 `judge_render_type_finish` 的
        //     `peel_one_binder` 剥掉 `__soko_render` 那一层 ⇒ 得到 `ty` ✓；
        //   * 就地路：**同样剥 `n` 层**（留下 `ty -> ty`），交给**同一个**收尾 ✓。
        // 第一版写成 `n + 1`（自己把 `__soko_render` 也剥了）⇒ 收尾再剥一层就
        // **多剥** ⇒ `peel_one_binder` 对非 Pi 返回 `None` ⇒ 就地路径**全部答不出**
        // ✗ —— 而 `on` 档的 `--json` 居然是**逐字节相同**的（因为那些答案下游
        // 会被 `is_rereadable`/`keep_if_lossless` 丢掉）⇒ **影子档才抓得住**
        // （实测：`shadow_diff=37508 / shadow_same=0`，`inplace=None`）。
        // `scope` = 那 `n` 个声明 binder 的名字（**外层在前**）⇒ 剥完 `n` 层后
        // 剩下的松散变量印回**真名**（与慢路逐字同形）✓。
        let scope: Vec<String> = binders.iter().map(|b| b.name.clone()).collect();
        quiet_catch(|| {
            ef.infer_type_text_at_peeled(limit, term, binders.len(), &scope, |t| t.to_string())
        })
    });
    match text {
        Ok(text) => {
            crate::judge::stats::INPLACE_BY_USED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(text)
        }
        Err(why) => {
            // 把内核的原话记下来（只写 "kernel" 查不出任何东西 ✗）
            crate::judge::stats::note_by_reason(&format!(
                "kernel({})",
                why.chars().take(60).collect::<String>()
            ));
            None
        }
    }
}

pub(crate) fn infer_type_text_inplace<'a>(
    env: &mut InplaceEnv<'_, 'a>,
    ctx: &ElabCtx<'a, '_>,
    // **只要 `(名字, 源类型)` 这一对**（原来传 `&ElabScope`，但它对这个函数**只有**
    // `judge_binder_srcs()` 一个用途）—— 换成裸数据之后，**没有 `ElabScope` 的调用链
    // 也能就地**（`by.rs` 的 `apply_tactic` 就是那种：它有 `env`、有 `context_binders`
    // 给的源类型，但拿不到 `ElabScope` ⇒ 以前只能整份重编前缀 ✗）。
    binder_srcs: &[(String, Expr)],
    operand: &Expr,
    binder_count: usize,
) -> Result<String, InplaceFail> {
    debug_assert_eq!(
        binder_srcs.len(),
        binder_count,
        "binder 筛选必须与 `judge_binders()` 完全一致（剥层数靠它）"
    );
    // ① 在活环境上 elaborate。空宇宙表：**与 `#check` 的处理器逐字一致**
    //    （`#check` 没有宇宙参数可解 ⇒ 读到宇宙变量的查询在两条路上**一样失败** ✓）。
    let no_universe: UnivMap<'_> = UnivMap::new();
    let mut scratch_hovers: Vec<HoverNode<'a>> = Vec::new();
    // 逐 binder：**先 elaborate 它的源类型、再推进作用域**（依赖顺序与源一致 ✓）。
    // ⚠ **内核以 panic 报拒绝**（架构 §8 gotcha 0）⇒ 每步都包 `quiet_catch`。
    let mut sc = ElabScope::new();
    let mut tys: Vec<ExprPtr<'a>> = Vec::with_capacity(binder_srcs.len());
    for (name, src_ty) in binder_srcs {
        let ty = quiet_catch(|| {
            elab_expr(
                env.builder,
                src_ty,
                &mut sc,
                &no_universe,
                env.known,
                &mut scratch_hovers,
                None,
                None,
                ctx,
            )
        })
        .map_err(|_| InplaceFail::ElabBinder)?
        .map_err(|_| InplaceFail::ElabBinder)?;
        tys.push(ty);
        sc.push(name.clone(), ty, Some(src_ty.clone()), operand.span());
    }
    // ② 项本身（`fun` 的体）。
    let body = quiet_catch(|| {
        elab_expr(
            env.builder,
            operand,
            &mut sc,
            &no_universe,
            env.known,
            &mut scratch_hovers,
            None,
            None,
            ctx,
        )
    });
    let body = match body {
        Ok(Ok(ptr)) => ptr,
        // **诊断**（`SOKO_INPLACE_WHY=1`，默认零成本 ✓）：实测就地失败 **100% 在这一步** ✗
        // （`used=303 / fallback=695`，原因全是 `on-elab-operand` ✓），而
        // `.map_err(|_| …)` 把 elaborator 的**原话丢了** ✗ ⇒ "为什么答不出"只能靠猜。
        // 记原话（空白换成 `_`：直方图按空白切词 ✗）。
        Ok(Err(err)) => {
            if crate::judge::inplace_why_enabled() {
                crate::judge::stats::note_by_reason(&format!(
                    "op({})",
                    err.message
                        .chars()
                        .take(70)
                        .collect::<String>()
                        .replace(char::is_whitespace, "_")
                ));
            }
            return Err(InplaceFail::ElabOperand);
        }
        Err(_) => {
            if crate::judge::inplace_why_enabled() {
                crate::judge::stats::note_by_reason("op(panic)");
            }
            return Err(InplaceFail::ElabOperand);
        }
    };
    // ③ 从内往外包 λ —— 与 `elab_expr` 的 `Expr::Lambda` 分支同序同形 ✓。
    let mut term = body;
    for ((name, _), ty) in binder_srcs.iter().zip(tys).rev() {
        let nm = env.builder.name_from_str(name);
        term = env.builder.mk_lambda(nm, BinderStyle::Default, ty, term);
    }
    // ④ 就地求类型文本：**只看"到此刻为止已落地"的声明**（= 前缀）——
    //    与合成文件里 `#check` 的 `env_at` 同一口径（`ByIndex`）✓。
    let limit = sokonanoda::env::EnvLimit::ByIndex(env.builder.declaration_count());
    let ty = env.builder.with_env(|ef| {
        // ⚠ **必须与 `kernel_phase` 的 `#check` 同档**（`kernel_phase.rs:199`：
        // `env.config.pp_options.proofs = true`）。默认 `proofs = false` 时，
        // pp 会对**每一个子项**调 `is_proof`（用**空局部上下文**推它的类型）
        // ⇒ 打印到 binder 内的 `Eq n m` 这类子项就 `loose bvar in infer` panic ✗
        // （本步实测踩到 **7 次**，不是理论风险）。`#check` 那条路正是靠这一行
        // 躲开的 —— 少了它，两条路的**文本与成败都会分叉** ✓。
        // `with_env` 的 `config` 是**克隆**进来的、不回写 ✓ ⇒ 不会污染主环境。
        ef.config.pp_options.proofs = true;
        // ⚠ **`explicit` 同样必须与 `#check` 出口同款**（G-71 ✗→✓，2026-10-04）：
        // `kernel_phase.rs:463-471` 在 `#check` 里把 `pp_options.explicit` 设成
        // `judge::explicit_pp_active()`、出完立刻还原 ✓；就地路**漏了这一行** ✗
        // ⇒ 同一问在两条路上打出**不同的文本**（`@Exists A q` vs `Exists A q` ✓）
        // ⇒ 下游按文本做的判断会**分叉**（实测 shadow `shadow_diff=855,317` ✗✗）。
        ef.config.pp_options.explicit = crate::judge::explicit_pp_active();
        quiet_catch(|| ef.infer_type_text_at(limit, term, |t| t.to_string()))
    });
    let ty = ty.map_err(|_| InplaceFail::Kernel)?;
    Ok(crate::judge::peel_binders(ty, binder_count))
}

/// 问内核要 `operand` 在**当前 binder 上下文**里的类型文本（与 `apply`
/// 读被应用函数类型同一条路：`judge_infer`，不做文本比对）。
///
/// **P1-a 第一步（2026-09-29）**：多收一个 [`InplaceEnv`] ——
/// `Some` 且开关允许时就地答（**不再合成整段前缀**）；
/// `None` / 开关 `off` / 就地答不出 ⇒ **逐字节回退到今天的行为** ✓。
fn infer_type_text<'a>(
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    operand: &Expr,
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<String> {
    let binders = scope.judge_binders();
    // ⚠ 这一行**保持原样**（`&render_expr(operand)` 内联）：它是记法路径守卫
    // （`scripts/audit-notation-paths.py`）棘轮基线里的一条 —— 改成 `let term = …`
    // 会让指纹对不上而被判"**新增** 1 处绕过唯一记法接口" ✗（实测踩到）。
    // 就地路**本来就不用**这份渲染文本（它直接用 `operand` 的源 AST）✓。
    // ⚠ **判定的查询文本**（**不是显示路径** ⇒ 这里用 `render_expr` 是对的：
    // 记法折叠那条接口是给**显示**用的，判定路径用了会改判定 ✗）。
    // 它同时喂两处：源码重跑（慢路）与 `judge` 缓存键（键必须与慢路**同源**，
    // 否则就地路径与慢路各自命中不同的缓存槽）。
    // 记法路径守卫（`scripts/audit-notation-paths.py`）把它算作**同族调用点**
    // —— 与基线里 `judge_infer(… &render_expr(val))` 那条同类 ✓。
    let term = render_expr(operand);
    let slow = || judge_infer(ctx.prefix_src, ctx.options, &binders, &term).ok();
    match (crate::judge::inplace_mode(), env) {
        (crate::judge::InplaceMode::On, Some(env)) => {
            // ① **命中先走今天那条快路**（一次哈希即可）—— 就地只做未命中那一段 ✓
            //    （理由见 `judge::judge_infer_lookup` 的注释：把廉价命中换成
            //    ~1 ms/次 的就地路径是**负优化**，实测过 ✗）。
            if let Some(hit) =
                crate::judge::judge_infer_lookup("", ctx.prefix_src, ctx.options, &binders, &term)
            {
                return hit.ok();
            }
            // ② 未命中 ⇒ 就地答（**不编译前缀**）；答出来了就写回同一张缓存 ✓。
            match infer_type_text_inplace(
                env,
                ctx,
                &scope.judge_binder_srcs(),
                operand,
                binders.len(),
            ) {
                Ok(text) => {
                    crate::judge::stats::INPLACE_USED
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let r = Ok(text.clone());
                    crate::judge::judge_infer_store(
                        "",
                        ctx.prefix_src,
                        ctx.options,
                        &binders,
                        &term,
                        &r,
                    );
                    Some(text)
                }
                // 就地答不出 ⇒ **直接答 `None`**（2026-10-04 · G-29 第一刀 ✓）。
                //
                // **为什么现在敢**（判据，不是感觉 ✓）：`explicit` 那一行补上之后，
                // shadow 档在 unit08 冷跑上实测
                //   infer 路 `shadow_same=5,975,543 · shadow_diff=0` ✓
                //   by   路 `shadow_same=9,512     · shadow_diff=0` ✓
                // （修前分别是 `855,317` / `4,664` 条分叉 ✗）。
                // shadow 的判据约定是：`(slow=None, inplace=Err) ⇒ same` ✓，而
                // **`(slow=Some, inplace=Err) ⇒ 分叉`** ✗ ⇒ **0 分叉正是**
                // 「就地失败 ⇒ 慢路也失败」这条蕴含的证明 ✓✓
                // ⇒ 失败时答 `None` 与跑慢路**等价** ✓，而后者要**重跑整份前缀**
                // 去发现同一个失败 ✗（实测一次按键 **28 趟 ≈ 1.8s** ✗ = 编辑慢的大头 ✓）。
                //
                // ⚠ **红线口径**：`SOKO_JUDGE_INPLACE=off` vs `on`（+`WIDE=0`）的全课程
                // `build --json` 必须**逐字节相同** ✓ —— 那是这条改动的验收 ✓
                // （⚠ `WIDE=1` 那条 wide 路**另有**分歧 ✗，与本刀无关 ✓，见台账 ✓）。
                Err(why) => {
                    crate::judge::stats::INPLACE_FALLBACK
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // **诊断**（`SOKO_INPLACE_WHY=1`，默认零成本 ✓）：`On` 档的失败
                    // 以前**只计数、不记因** ✗（原因只在 shadow 档且两条分叉时才记 ✗）
                    // ⇒ "就地路为什么答不出"只能靠猜 —— 实测 `used=303 / fallback=695`
                    // （**69.6% 答不出** ✗，原因 100% 是 `on-elab-operand` ✓：
                    // 记法 `∅` 展开成 `Set.empty` 时补不出前导类型参数 ✗）。
                    if crate::judge::inplace_why_enabled() {
                        crate::judge::stats::note_by_reason(match why {
                            InplaceFail::ElabBinder => "on-elab-binder",
                            InplaceFail::ElabOperand => "on-elab-operand",
                            InplaceFail::Kernel => "on-kernel",
                        });
                    }
                    // 失败 ⇒ **直接答 `None`**（不跑慢路 ⇒ 不再重跑整份前缀 ✓）。
                    //
                    // **依据**：就地路与慢路现在**文本逐字节一致** ✓（shadow `diff=0` ✓），
                    // 且「全课程 `off` vs `on WIDE=0 BY=0` 的 `build --json` 逐字节相同」✓
                    // —— 后者是这条改动的验收口径 ✓（读数见提交信息 ✓）。
                    None
                }
            }
        }
        // 影子档：**两条都跑**、比对文本；**返回源码重跑那份** ⇒ 判定逐字节不变 ✓。
        (crate::judge::InplaceMode::Shadow, Some(env)) => {
            // 命中 ⇒ 两条路**都轮不到**（`On` 档同样直接返回缓存）⇒ 记一笔 same 即可。
            // 不在这里跑就地：那是 15 万次 × ~1 ms 的账 ✗（真正要比的是**未命中**那批）。
            if let Some(hit) =
                crate::judge::judge_infer_lookup("", ctx.prefix_src, ctx.options, &binders, &term)
            {
                crate::judge::stats::INPLACE_SHADOW_SAME
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return hit.ok();
            }
            let old = slow();
            let new = infer_type_text_inplace(
                env,
                ctx,
                &scope.judge_binder_srcs(),
                operand,
                binders.len(),
            );
            let same = match (&old, &new) {
                (Some(a), Ok(b)) => a == b,
                (None, Err(_)) => true,
                _ => false,
            };
            if same {
                crate::judge::stats::INPLACE_SHADOW_SAME
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else {
                crate::judge::stats::INPLACE_SHADOW_DIFF
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Err(why) = &new {
                    if let Ok(mut reasons) = crate::judge::stats::INPLACE_FAIL_REASONS.lock() {
                        reasons.push_str(&format!("{why:?} "));
                    }
                    // 同一份原因也**落文件**（`atexit` 在 LSP 上不跑，见该函数的注释）。
                    crate::judge::stats::note_inplace_fail(&format!("{why:?}"));
                }
                eprintln!(
                    "JUDGE_INPLACE_MISMATCH prefix={} binders={} term={} slow={:?} inplace={:?} \
                     why={:?}",
                    ctx.prefix_src.len(),
                    binders.len(),
                    format!("{operand:?}").chars().take(60).collect::<String>(),
                    old.as_deref()
                        .map(|s| s.chars().take(120).collect::<String>()),
                    new.as_ref()
                        .ok()
                        .map(|s| s.chars().take(120).collect::<String>()),
                    new.as_ref().err(),
                );
            }
            old
        }
        _ => slow(),
    }
}

/// `operand` 的类型（**源级 AST**）：局部变量**优先取书写类型**，零内核调用。
///
/// 为什么必须有这条快路（T-K22，G-34）：`infer_type_text` 走 `judge_infer`，
/// 而 `judge_infer` 的缓存键**含整段前缀**；前缀随每条声明增长 ⇒ 同一批查询
/// （`α` / `f` / `A` / `y` …）每次都换一个键，未命中就**全前缀重编译一趟 pass**，
/// 命中也要哈希整段前缀。实测 `unit12-solution`：`judge_infer` **126,105 次调用
/// / 363 次未命中**，占掉整次判卷的绝大部分（G-34）。而这些查询问的几乎全是
/// **局部变量**，答案就在 `scope` 里。
///
/// 与 `implicit.rs` 那条路同一个判据（见下面 `arg_tys` 的注释）：书写类型不仅
/// **零内核调用**，还比内核 pp 文本**更准**——pp 会丢掉嵌套常量的隐式实参
/// （`Eq.{1} Nat 1 1` pp 成 `Eq 1 1`）。
///
/// 拿不到书写类型（隐式插入的 binder、复合项）⇒ 原路 `infer_type_text`，
/// 逐字节不变。
/// 类型表达式的**头**（G-42 的贴合比较用 ✓）。
///
/// `Sort` 家族按**层级**归一 —— `Prop` = `Sort 0`、`Type` = `Sort 1` ✓
/// （Lean：`Type u = Sort (u+1)`；parser 的注释也这么写 ✓）。**这正是"`Type` 与
/// `Sort 1` 同义而异形"那个坑的解** ✓：它们在 AST 里是**两个变体**
/// （`SortKind::Type` / `SortKind::Sort(1)`）✗，按层级归一就一致了 ✓。
/// 应用取**函数位置**的头（`Set α` ⇒ `Set` ✓）。
///
/// **返回 `None` = "认不出" ⇒ 调用方按"不贴合"处理** ✓（宁可不猜 ✗）。
/// ⚠ 这里**刻意不用 `render_expr`** ✗ —— 那是**显示**路径（唯一接口是
/// `DisplayNotations` ✓，`scripts/audit-notation-paths.py` 会抓 ✓），而本函数只是
/// "试哪种读法"的启发式 ✓；也**不能**直接比 `Expr` ✗ —— `PartialEq` 含 `span` ✓。
fn type_head(e: &Expr) -> Option<String> {
    match e {
        Expr::Sort { sort, .. } => Some(match sort {
            SortKind::Prop => "0".to_string(),
            SortKind::Type => "1".to_string(),
            SortKind::Sort(n) => n.to_string(),
            SortKind::Level(_) => return None,
        }),
        Expr::Ident { name, .. } => Some(name.clone()),
        Expr::App { fun, .. } => type_head(fun),
        Expr::Arrow { .. } => Some("->".to_string()),
        Expr::Forall { .. } => Some("forall".to_string()),
        _ => None,
    }
}

/// **写出来的实参是否逐位贴合对应层的域**（G-42 第二半，2026-09-26）。
///
/// 把"把隐式位也逐位写出来"（`some Nat`、`And.intro a b ha hb`）与"短写"
/// （`Set.mem a A`）分开 —— 光看**个数**分不开（两种读法的个数可以一样 ✗）。
///
/// 做法：从左到右比 `arg` 的类型与 `layers[i].domain`（**边比边代入**已认下的实参
/// ⇒ 后面的层能引用前面的 ✓），两侧都先 δ 展开头部（`Set α` 是 def ⇒ 与 `α -> Prop`
/// 要能比上 ✓）。**有一位不贴合就整体否掉** ⇒ 交回短写 ✓（宁可不猜 ✗）。
///
/// ⚠ **两个坑都踩过（实测）**：
/// 1. **不能比 `Expr` 结构** —— `Expr` 的 `PartialEq` **含 `span`** ✗
///    （两边都是 `Sort { sort: Type, span: … }`、只差 offset 就判不等 ✓）；
/// 2. **也不能直接比 pp 文本** —— `Type` 与 `Sort 1` 同义而异形 ✗
///    ⇒ 要按**层级**归一（见 [`type_head`] ✓）。
///
/// 这里要的只是"两种写法是不是同一个类型"的**启发式**（决定试哪种读法 ✓，
/// **不是判定** ✗ —— 判定永远走 kernel ✓），所以比归一的 pp 文本是合适的 ✓。
fn args_fit_layers_in_order<'a>(
    layers: &[crate::compile::implicit::Layer],
    args: &[&Expr],
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    // **P1-a**：`Some` ⇒ 就地判定（调用方手里有活环境）；`None` ⇒ 今天的老路。
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> bool {
    let mut env = env;
    if args.is_empty() || args.len() > layers.len() {
        return false;
    }
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (i, a) in args.iter().enumerate() {
        // `env` 按 `&mut` 逐次借用（`Option<&mut _>` 不是 `Copy`）——**每轮只借一次**。
        let domain = crate::spine::substitute(&layers[i].domain, &sigma);
        if !type_head_fits_layer(&domain, a, ctx, scope, InplaceEnv::reborrow(&mut env)) {
            return false;
        }
        if !layers[i].name.is_empty() {
            sigma.insert(layers[i].name.clone(), (*a).clone());
        }
    }
    true
}

/// 一位实参是否贴合该层的域（[`args_fit_layers_in_order`] 的逐位判据，抽出来给
/// 「开头是不是旧写法」单独用 —— T-N13）。
///
/// ⚠ **既不比 `Expr` 结构、也不比 pp 文本**：
/// * 比结构 ✗ —— `Expr` 的 `PartialEq` **含 `span`**（只差 offset 就判不等 ✓）；
/// * 比 pp ✗ —— 那是**显示**路径（唯一接口是 `DisplayNotations` ✓，记法守卫会抓 ✓），
///   而且 `Type` 与 `Sort 1` 同义而异形 ✓。
///   ⇒ 比**类型表达式的头**（`Sort` 家族按层级归一 ✓、应用取函数位置的头 ✓）。
fn type_head_fits_layer<'a>(
    domain: &Expr,
    a: &Expr,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> bool {
    let is_inductive = |n: &str| ctx.inductives.contains_key(n);
    let unfold = |e: &Expr| {
        let mut cur = e.clone();
        for _ in 0..4 {
            let next = crate::spine::unfold_to_inductive(
                &cur,
                &is_inductive,
                ctx.defs,
                4,
                None,
                crate::spine::UnfoldAlign::Short,
            );
            if next == cur {
                break;
            }
            cur = next;
        }
        cur
    };
    let Some(actual) = operand_type_expr(ctx, scope, a, env) else {
        return false;
    };
    let Some(d) = type_head(&unfold(domain)) else {
        return false;
    };
    type_head(&unfold(&actual)).as_deref() == Some(d.as_str())
}

/// 操作数的**类型表达式**（源级 AST），供隐式前缀求解当模板/实参用。
///
/// **书写类型优先**（零内核调用）：`Ident` 取作用域里的**源级**类型；lambda 取
/// **源级 binder 注解**拼出来的箭头类型（见 [`lambda_source_type`]）。两者都
/// **绕开内核 pp** —— pp 会丢隐式实参（实测：`Set.image α β f A` 打成
/// `Set.image β f A`），而这份文本会被**回读成项**再 elaborate ⇒ 静默换成另一个
/// 解（`α := β`）✗。剩下的形状才问内核（pp 文本 → `parse_expr_text`）。
fn operand_type_expr<'a>(
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    operand: &Expr,
    env: Option<&mut InplaceEnv<'_, 'a>>,
) -> Option<Expr> {
    if let Expr::Ident { name, .. } = operand {
        if let Some(src) = scope.source_type_of(name) {
            return Some(src);
        }
    }
    if let Some(src) = lambda_source_type(operand) {
        return Some(src);
    }
    infer_type_text(ctx, scope, operand, env)
        .and_then(|text| crate::proof::parse_expr_text(&text).ok())
}

/// `fun (h : P) => h` 的**书写类型** `P -> P`，纯源级走查（**零内核调用**）。
///
/// 只认**一条**形状：所有 binder 都有注解，且体是某个 binder 的**裸名引用**
/// （`fun (h : P) => h` —— 课程证明项里压倒性的多数，`Iff.intro`/`And.intro`
/// 的实参全是它）。体的类型就是**那个 binder 的注解**，于是整条类型 =
/// `T₀ → T₁ → … → Tₙ₋₁ → T_idx` ✓ —— 不需要问内核，也就不会被 pp 的隐式实参
/// 丢失污染 ✓。
///
/// 其余形状（`fun (x : α) => f x` 之类）返回 `None` ⇒ 调用方照旧问内核，
/// **不比今天差** ✓。
fn lambda_source_type(operand: &Expr) -> Option<Expr> {
    let Expr::Lambda { binders, body, .. } = operand else {
        return None;
    };
    if binders.is_empty() {
        return None;
    }
    let mut tys: Vec<Expr> = Vec::with_capacity(binders.len());
    for binder in binders {
        tys.push(binder.ty.as_deref()?.clone());
    }
    let Expr::Ident { name, .. } = body.as_ref() else {
        return None;
    };
    let idx = binders.iter().position(|b| &b.name == name)?;
    let mut ty = tys[idx].clone();
    for domain in tys.iter().rev() {
        ty = Expr::Arrow {
            domain: Box::new(domain.clone()),
            codomain: Box::new(ty),
            span: Span::default(),
        };
    }
    Some(ty)
}

/// **IA-1 的唯一钩子**（设计 `docs/design/implicit-arguments.md` §3.2）：`expr`
/// 是一条应用脊，头如果是签名带**前导隐式 binder** 的常量/局部名，就按路线 C
/// 把那些隐式实参解出来插进去，返回装好的整条脊。
///
/// 返回 `None` ⇒ 调用方走**今天的老路**（签名里没有隐式 binder 时逐字节不变，
/// 这是 P1 可独立发布的安全性质，设计 §0）。
///
/// 算法（设计 §3.1）：实参按**风格**对齐到显式层；被跳过的前导隐式层由**第一个
/// 显式实参的类型**头部匹配唯一确定（[`implicit::solve_prefix`]）；解不出报
/// `elab-implicit-argument-unsolved`，**不猜**。
/// **B3-①（缺口 G-40）**：**裸常量**（零实参）的前导隐式实参插入。
///
/// 为什么必须是**独立一条**：`try_implicit_application` 只挂在 `Expr::App` 臂上
/// （设计 §3.2 的"唯一钩子"），而 `∅` 展开成的是**光秃秃的 `Set.empty`** ——
/// 它根本不是 `App` ✗ ⇒ 钩子永远够不着 ⇒ 词项停在 `{α : Type} → Set α` 那个 Pi
/// 上（`rfl` 判不出来，实测见缺口 G-40 的复现件）。
///
/// **只在期望类型真的给了、且签名确实有前导隐式 binder 时**才动；**解不出就原样
/// 返回**（不报错、不猜）—— 裸常量当函数值用是合法的 ✓。
#[allow(clippy::too_many_arguments)]
fn try_bare_implicit_constant<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &KnownTable,
    canonical: &str,
    bare: ExprPtr<'a>,
    expected_src: Option<&Expr>,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    hovers: &mut Vec<HoverNode<'a>>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    let Some(expected) = expected_src else {
        return Ok(bare);
    };
    // 与那条唯一钩子同款的两道闸门（`@` 那条不适用：裸常量没有脊）。
    if prelude_install_active() {
        return Ok(bare);
    }
    let Some(declared) = known.get(canonical) else {
        return Ok(bare);
    };
    let k = declared.implicit_prefix();
    if k == 0 {
        return Ok(bare);
    }
    let Some(ty_text) = declared.signature() else {
        return Ok(bare);
    };
    let Some((layers, result)) = crate::compile::implicit::telescope(ty_text) else {
        return Ok(bare);
    };
    if k > layers.len() {
        return Ok(bare);
    }
    let is_inductive = |n: &str| ctx.inductives.contains_key(n);
    let Some(solved) = crate::compile::implicit::solve_prefix(
        &layers,
        &result,
        k,
        &[],
        Some(expected),
        ctx.defs,
        &is_inductive,
    ) else {
        return Ok(bare);
    };
    let mut out = bare;
    for value in &solved {
        let term = elab_expr(builder, value, scope, univ, known, hovers, None, None, ctx)?;
        out = builder.mk_app(out, term);
    }
    Ok(out)
}

/// 把 `base` 沿 Π 展开 `surplus` 层 —— 路线③ 的「富余实参落到**结果**上」。
///
/// `Set α` 是 **def** ⇒ 不 δ 展开就看不到 `α -> Prop` 那一层 ✗。
/// **展不动就少给几层**（调用方按 `len() == surplus` 判成功 ✓，绝不假装展开了 ✗）。
fn surplus_layers(
    base: &Expr,
    surplus: usize,
    defs: &DefTable,
) -> Vec<crate::compile::implicit::Layer> {
    let mut tail: Vec<crate::compile::implicit::Layer> = Vec::with_capacity(surplus);
    let mut cur = base.clone();
    for i in 0..surplus {
        let mut pi = cur.clone();
        for _ in 0..8 {
            if matches!(pi, Expr::Arrow { .. } | Expr::Forall { .. }) {
                break;
            }
            match crate::spine::unfold_one(&pi, defs, None) {
                Some(next) if next != pi => pi = next,
                _ => break,
            }
        }
        let (domain, codomain) = match &pi {
            Expr::Arrow {
                domain, codomain, ..
            } => (domain.as_ref().clone(), codomain.as_ref().clone()),
            Expr::Forall { binders, body, .. } if binders.len() == 1 && binders[0].ty.is_some() => {
                (
                    binders[0]
                        .ty
                        .as_ref()
                        .expect("checked above")
                        .as_ref()
                        .clone(),
                    body.as_ref().clone(),
                )
            }
            _ => {
                tail.clear();
                break;
            }
        };
        tail.push(crate::compile::implicit::Layer {
            name: format!("\0soko_r3_{i}"),
            domain,
            style: BinderKind::Explicit,
        });
        cur = codomain;
    }
    tail
}

#[allow(clippy::too_many_arguments)]
fn try_implicit_application<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    explicit_spine: bool,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<Option<ExprPtr<'a>>, CompileError> {
    // Lean 的 `@`：整条脊的实参是**逐位显式**的 ⇒ 不插隐式实参（设计 §7 第 5 条）。
    if explicit_spine {
        return Ok(None);
    }
    // prelude 安装期间（含 `judge_infer` 触发的嵌套安装）**一律不插**：插入要先
    // `judge_infer`，而它会重装 prelude ⇒ 无限递归（见 [`PreludeInstallGuard`]）。
    if prelude_install_active() {
        return Ok(None);
    }
    let (head, args) = crate::spine::spine_of(expr);
    if args.is_empty() {
        return Ok(None);
    }
    // **免费闸门**（IA-1 的签名表：零内核调用）：头的签名没有前导隐式 binder ⇒
    // 一行都不跑。这一条兜住两个东西——安全性质（今天所有签名都是 0 ⇒ 逐字节
    // 不变）与成本（否则**每个**应用都要 `judge_infer`，而判定会**递归**重编译
    // 前缀 ⇒ 栈溢出，实测）。
    let raw_head = match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.as_str(),
        // **零元记法在函数位**（`(∅) x` —— by 引擎把目标 `∅ ⊆ A` 展开成
        // `∀ x, (∅) x → A x` 之后就是这个形状）：记法节点自己**没有实参**，
        // spine 上的实参就是它的**富余实参** ⇒ 走与 `Set.univ x` 同一条路线 ③
        // （`α` 从富余实参的类型解出）✓。
        //
        // ⚠ **非零元记法不在这里认** ✗：它的操作数在记法节点**内部**
        // （`(A ∩ B) x` 的 `A`/`B` 不在 spine 里）⇒ 拿 spine 实参当实参表会错位。
        // 那条形状由 `spine::unfold_one` 的「记法在函数位」分支管 ✓。
        Expr::Notation {
            target,
            lhs: None,
            rhs: None,
            ..
        } => target.as_str(),
        _ => return Ok(None),
    };
    // ⚠ **已知缺口 G-42**（2026-09-26 实测）：这里**应该**用**解析后的规范名**
    // 查签名表 —— `namespace Set` 里写 `subset B A` 时 AST 上是**裸名** `subset`，
    // 而签名表按**规范名** `Set.subset` 建 ⇒ 拿裸名查**永远查不到** ⇒ 隐式插入
    // 整条不触发 ✗（5 行最小复现见 `docs/gaps/repro/G42-*.sh`；**去掉 namespace
    // 就好**，这正是它躲过所有既有测试的原因 ✓）。
    //
    // **为什么还没改**：显然的改法（先 `resolve_known`）**修好 G-42 却回归**
    // `#check some Nat`（`Nat -> Option Nat` ⇒ `Option Type 0` ✗，撞红既有
    // `parameterized_option_checks_and_derives_recursor`）。两者是**同一个洞**：
    // 下面 `:2260` 那条"实参个数 > 显式层数 ⇒ 当成旧写法（隐式位也逐位写了）"的
    // 判据，**分不清**"隐式位也写了"（`Eq.symm α a b h`）与"把结果函数又应用了
    // 一次"（`Set.univ x`、`some Nat`）—— 一旦钩子真的被触发，这个歧义就暴露 ✓。
    // ⇒ 修 G-42 必须**同时**给求解器加"富余实参应用到结果类型"那一档（路线③），
    // 判据是**两条一起绿**：本文件的 Option 测试 + G-42 的复现件 ✓。
    // ⚠ **已知缺口 G-42**（2026-09-26 实测）：这里**应该**用**解析后的规范名**
    // 查签名表 —— `namespace Set` 里写 `subset B A` 时 AST 上是**裸名** `subset`，
    // 而签名表按**规范名** `Set.subset` 建 ⇒ 拿裸名查**永远查不到** ⇒ 隐式插入
    // 整条不触发 ✗（5 行最小复现见 `docs/gaps/repro/G42-*.sh`；**去掉 namespace
    // 就好**，这正是它躲过所有既有测试的原因 ✓）。
    //
    // **为什么还没改**：显然的改法（先 `resolve_known`）**修好 G-42 却回归**
    // `#check some Nat`（`Nat -> Option Nat` ⇒ `Option Type 0` ✗，撞红既有
    // `parameterized_option_checks_and_derives_recursor`）。两者是**同一个洞**：
    // 下面 `:2260` 那条"实参个数 > 显式层数 ⇒ 当成旧写法（隐式位也逐位写了）"的
    // 判据，**分不清**"隐式位也写了"（`Eq.symm α a b h`）与"把结果函数又应用了
    // 一次"（`Set.univ x`、`some Nat`）—— 一旦钩子真的被触发，这个歧义就暴露 ✓。
    // ⇒ 修 G-42 必须**同时**给求解器加"富余实参应用到结果类型"那一档（路线③），
    // 判据是**两条一起绿**：本文件的 Option 测试 + G-42 的复现件 ✓。
    // **用解析后的规范名查签名表**（G-42，2026-09-26）：`namespace Foo` 里写
    // `subset B A` 时 AST 上是**裸名** `subset`，而签名表按**规范名** `Foo.subset`
    // 建 ⇒ 拿裸名查永远查不到 ⇒ 隐式插入整条不触发 ✗。解析不出来的名字（真未定义）
    // ⇒ 交回老路，让它照旧报 `unknown identifier` ✓。
    let head_name = match resolve_known(known, ctx.ns, raw_head, head.span()) {
        Ok(canonical) => canonical,
        Err(_) => raw_head.to_string(),
    };
    let head_name = head_name.as_str();
    // **头的 elaborate 形态**：头是**零元记法节点**时（`(∅) x`），它的展开就是
    // 目标常量本身 ⇒ 用 `Ident(target)` 代替节点去 elaborate ✓ —— 直接
    // `elab_expr(记法节点)` 会走记法路径，而那里**没有期望类型**（它在函数位）
    // ⇒ 报「补不出前导类型参数」✗（实测）。其余形状零开销（借用原节点 ✓）。
    let head_src: std::borrow::Cow<'_, Expr> = match head {
        Expr::Notation { target, span, .. } => std::borrow::Cow::Owned(Expr::Ident {
            name: target.clone(),
            span: *span,
        }),
        other => std::borrow::Cow::Borrowed(other),
    };

    let declared = match known.get(head_name) {
        Some(k) if k.implicit_prefix() > 0 => k,
        _ => return Ok(None),
    };
    // 头的**源级**签名文本（注册表自带；见 `KnownName::Decl::signature`）。
    // 不再用 `judge_infer` 拿 pp 文本：那会重编译前缀（含 prelude 安装）⇒
    // prelude 自身的部分应用会再次触发插入、无限递归（实测栈溢出）；而且 pp 会
    // 抹掉嵌套常量的隐式实参（`Eq.symm` 的 `h : Eq a b` 里 `α` 不见了），
    // 前导参数反解不出来。
    let Some(ty_text) = declared.signature() else {
        return Ok(None);
    };
    let Some((layers, result)) = crate::compile::implicit::telescope(ty_text) else {
        return Ok(None);
    };
    // **`k` 取注册表里的前导隐式层数，不取 pp 文本的风格**：构造子的参数在
    // kernel 类型里是**显式** binder（Lean 也是），但语义上它们是隐式的
    // （`And.intro h1 h2`）——只有注册表知道这件事。pp 的 `{}`/`()` 与注册表的
    // 数值对同一签名必须一致（非构造子时二者相等）。
    let k = declared.implicit_prefix();
    if k == 0 || k > layers.len() {
        return Ok(None);
    }
    // **路线③（B3，2026-09-26；缺口 G-41 / G-42 的共同前置）**：
    // 签名里**只有隐式 binder** 的常量被应用时（`Set.univ x`），富余实参落到
    // **结果类型**上 —— 正确读法是 `@Set.univ ?α x`，而 `?α` 要从**富余实参自己
    // 的类型**解出来（`x : ?α`）✓。
    //
    // 为什么必须排在"旧写法"**之前**：那条分支的判据是"实参个数 > 显式层数"，
    // 它**分不清**「隐式位也逐位写了」（`And.intro a b ha hb`）与「把**结果函数**
    // 又应用了一次」（`Set.univ x`）✗。全隐式参数的常量最惨：`explicit_arity == 0`
    // ⇒ **任何**应用都落进旧写法 ⇒ `x` 被装到 `layers[0]`（域 `Type`）上 ⇒
    // `Set.univ x : Set x` ✗（S1 只读侦察的 REPL 直证：
    // `#check fun (α : Type) (x : α) => Set.univ x` ⇒ `forall (α : Type 0) (x : α), Set x`）。
    //
    // 实现上**不改求解器**：把结果类型沿 Π 展开 `surplus` 层，**接在望远镜后面**
    // 当"虚拟层"⇒ 直接复用同一条 `solve_prefix` ✓（`arg_tys` 本来就是按
    // `layers[k..]` 对齐的，接长一层就自动对齐到富余实参 ✓）。
    // **解不出 / 结果展不成 Π**（`And.intro a b ha hb` 的 `And a b` 是归纳类型 ✗）
    // ⇒ 静默退回下面的旧写法 ⇒ **既有行为不变** ✓。
    let explicit_layers = layers.len().saturating_sub(k);
    // **先算"旧写法是否贴合"**（G-42 的规则 ✓）—— 它决定路线③ 让不让位 ✓：
    // `And.intro a b ha hb` 逐位贴合 ⇒ 旧写法 ✓（不许被"富余实参落到结果上"抢走 ✗）；
    // `Set.image f A y` 不贴合（`f` 是函数、不是 `Type` ✗）⇒ 让给路线③ ✓。
    // ⚠ 必须在块**外**算：下面两个分支都要用它 ✓。
    let fits_old_style = args.len() <= layers.len()
        && args_fit_layers_in_order(
            &layers,
            &args,
            ctx,
            scope,
            Some(&mut InplaceEnv {
                builder: &mut *builder,
                known,
            }),
        );
    // **第一个实参是否落在第 0 层上**（T-N13，2026-09-30）：旧写法的**开头**就是
    // `layers[0]`（`Eq.trans.{1} (Set Two) X …` 的 `Set Two` 正好落在 `{α}` 层上 ✓）；
    // 短写的开头落在**第一个显式层**（`Set.image f A y` 的 `f` ✗ 不是 `Type`）。
    //
    // 为什么要单独一条：`fits_old_style` 是**逐位**判据，**中间**有一位不贴合就整体
    // 否掉——而旧写法里"某一位是零元隐式常量"（`Eq.trans.{1} (Set Two) X (f1 '' (∅)) (∅) h1 h2`
    // 的第 4 位 `∅`）拿到的类型是它自己的 **Pi**（`{α} → Set α`，它要等期望类型才补
    // 隐式实参）⇒ 必然"不贴合" ✗ ⇒ 路线③ 抢走 ⇒ 组装错位 ⇒ 内核报
    // `期望 (Set.[] Two.[])，实际是 Pi (α : Sort(1)), (Set.[] $0)` ✗（实测：
    // unit08 的 `image_inter_singletons_empty`）。
    // 开头贴合 ⇒ 这就是旧写法，路线③ **必须让位** ✓（它只该管"实参从显式层开始"的形状）。
    let starts_old_style = args.first().is_some_and(|a| {
        type_head_fits_layer(
            &layers[0].domain,
            a,
            ctx,
            scope,
            Some(&mut InplaceEnv {
                builder: &mut *builder,
                known,
            }),
        )
    });
    {
        // **闸门 = 「实参比显式层多」且「旧写法不贴合」** ✓。
        //
        // 演进过程（都实测过，别退回去 ✗）：
        // * 第一版放宽到"任意富余实参"⇒ **当场打红 prelude**
        //   （`l1_prelude_is_available_in_full_mode`：`期望 Pi (_ : Pi (_ : $1), False), False`
        //   实际 `Sort(0)` ✗）—— 因为"实参个数 > 显式层数"**分不清**
        //   「隐式位也逐位写了」与「把返回值继续应用」✗；
        // * 于是收紧到 `explicit_layers == 0`（G-41 那一族 ✓）—— 但那只覆盖**全隐式**，
        //   `Set.image f A y`（`{α β}` 两个隐式 + 两个显式，实参 3 个 ✗）漏在外面 ✗
        //   ⇒ `lib/Image` / `unit08` / `unit12` 都卡在
        //   `def_eq mismatch expected: Sort(1) | actual: Pi ( : $4), $4` ✗；
        // * 现在用 **G-42 的"贴合"判据**把那个歧义**真的判掉**了 ✓（`args_fit_layers_in_order`
        //   —— 逐位比对应层的**类型头** ✓）⇒ 可以安全地放宽到"实参比显式层多" ✓：
        //   贴合 ⇒ 旧写法 ✓；不贴合 ⇒ 富余实参落到**结果**上 ✓。
        if !fits_old_style && !starts_old_style && !args.is_empty() && args.len() > explicit_layers
        {
            let surplus = args.len() - explicit_layers;
            let mut tail = surplus_layers(&result, surplus, ctx.defs);
            // **第二趟（G-85，2026-10-03）**：结果类型是**变量**时（`And.right` 的
            // `b`）第一趟展不动 ✗ —— 但把**前导隐式参数解出来**之后它就是函数了 ✓
            // （`And.right h x`：`b := ∀ (x : Prop), Q x`）。原料与短写同一条
            // `solve_prefix`（显式实参的类型 ↔ `layers[k..]`）✓。
            //
            // ⚠ 只在**第一趟失败**时才算 ⇒ 既有形状**零额外开销、逐字节不变** ✓
            //（`operand_type_expr` 可能要问内核，绝不能无条件提前算 ✗）。
            //
            // ⚠⚠ **还必须 `args.len() <= k`**（本轮实测踩过 ✗）：旧写法把实参按
            // `layers[0..]` 逐位装 ⇒ 只有实参个数 **> k** 时才够得着显式层 ✓；
            // `args.len() <= k` ⇒ 旧写法**结构上不可能**到达显式层 ⇒ 读法唯一 ✓✓。
            // 不设这一条会抢走 `Eq.refl.{2} Type A` / `Eq.mp.{1} A A h` 这类
            // **宇宙显式给出**的调用（k 只数得到 `{α}` 那一层）⇒ 实测把
            // `docs/gaps/repro/L03-eq-type-level.sokonanoda` 从 exit 0 打成 exit 1 ✗
            // （`期望 Sort(1)，实际是 Sort(2)`）—— 那正是"守卫太宽"的典型面孔 ✓。
            if tail.len() != surplus && args.len() <= k {
                let arg_tys_prefix: Vec<Option<Expr>> = args
                    .iter()
                    .take(explicit_layers)
                    .map(|a| {
                        operand_type_expr(
                            ctx,
                            scope,
                            a,
                            Some(&mut InplaceEnv {
                                builder: &mut *builder,
                                known,
                            }),
                        )
                    })
                    .collect();
                let is_inductive = |n: &str| ctx.inductives.contains_key(n);
                if let Some(solved_prefix) = crate::compile::implicit::solve_prefix_with_args(
                    &layers,
                    &result,
                    k,
                    &arg_tys_prefix,
                    &args,
                    expected_src,
                    ctx.defs,
                    &is_inductive,
                ) {
                    let mut sigma: HashMap<String, Expr> = HashMap::new();
                    for (j, value) in solved_prefix.iter().enumerate() {
                        if !layers[j].name.is_empty() {
                            sigma.insert(layers[j].name.clone(), value.clone());
                        }
                    }
                    let inst = crate::spine::substitute(&result, &sigma);
                    if inst != result {
                        let tail2 = surplus_layers(&inst, surplus, ctx.defs);
                        if tail2.len() == surplus {
                            tail = tail2;
                        }
                    }
                }
            }
            if tail.len() == surplus {
                let mut extended = layers.clone();
                extended.extend(tail);
                let mut arg_tys: Vec<Option<Expr>> = Vec::with_capacity(args.len());
                for a in &args {
                    arg_tys.push(operand_type_expr(
                        ctx,
                        scope,
                        a,
                        Some(&mut InplaceEnv {
                            builder: &mut *builder,
                            known,
                        }),
                    ));
                }
                let is_inductive = |n: &str| ctx.inductives.contains_key(n);
                if let Some(solved) = crate::compile::implicit::solve_prefix_with_args(
                    &extended,
                    &result,
                    k,
                    &arg_tys,
                    &args,
                    expected_src,
                    ctx.defs,
                    &is_inductive,
                ) {
                    let mut out = elab_expr(
                        builder, &head_src, scope, univ, known, hovers, None, None, ctx,
                    )?;
                    for (i, value) in solved.iter().enumerate() {
                        // 与主路径同款：解出来的隐式实参也要吃**该层的域**当期望类型 ✓。
                        let expected_src = crate::spine::substitute(&extended[i].domain, &{
                            let mut sig: HashMap<String, Expr> = HashMap::new();
                            for (j, s) in solved.iter().take(i).enumerate() {
                                if !extended[j].name.is_empty() {
                                    sig.insert(extended[j].name.clone(), s.clone());
                                }
                            }
                            sig
                        });
                        let term = elab_expr(
                            builder,
                            value,
                            scope,
                            univ,
                            known,
                            hovers,
                            None,
                            Some(&expected_src),
                            ctx,
                        )?;
                        out = builder.mk_app(out, term);
                        // 隐式实参算完之后，**写出来的那些实参**照旧逐位接上 ✓
                        // （它们的期望类型就是对应层的域 ✓，与旧写法同一口径）。
                        if i + 1 == k {
                            let mut sigma: HashMap<String, Expr> = HashMap::new();
                            for (j, s) in solved.iter().enumerate() {
                                if !extended[j].name.is_empty() {
                                    sigma.insert(extended[j].name.clone(), s.clone());
                                }
                            }
                            for (j, a) in args.iter().enumerate() {
                                let expected_src =
                                    crate::spine::substitute(&extended[k + j].domain, &sigma);
                                let t = elab_expr(
                                    builder,
                                    a,
                                    scope,
                                    univ,
                                    known,
                                    hovers,
                                    None,
                                    Some(&expected_src),
                                    ctx,
                                )?;
                                out = builder.mk_app(out, t);
                                if !extended[k + j].name.is_empty() {
                                    sigma.insert(extended[k + j].name.clone(), (*a).clone());
                                }
                            }
                        }
                    }
                    return Ok(Some(out));
                }
            }
        }
    }
    // **旧写法（把隐式位也逐位写出来）**：`And.intro a b ha hb`、
    // `Eq.symm α a b h`、`cast.{1} α β h`、`Iff.mpr A B h`。判据 = 实参个数 >
    // 显式层数（Lean 短写法只会给显式层的实参）。满足就**在这里一次装完**，
    // 按 `layers[0..args.len()]` 逐位对齐，绝不递归到前缀——前缀
    // （`cast.{1} α β`、`And.right a`）会被误判成"隐式短写"而报错。
    // **第二半（G-42）**：钩子真被触发之后，这条闸门里的歧义就暴露了 —— `some Nat`
    // 会被**短写**抢走（`a := Nat`、`A` 从 `Nat` 的类型解 ⇒ `Option Type 0` ✗），
    // 而正确读法是**旧写法**（`A := Nat` ⇒ `Nat -> Option Nat` ✓）。光看**个数**
    // 分不开（两种读法都是 1 个实参 ✗）⇒ 多一条**可判定**的入口：写出来的实参
    // **逐位贴合**对应层的域 ⇒ 这就是"把隐式位也逐位写出来" ✓。
    if args.len() > declared.explicit_arity() || fits_old_style {
        if args.len() > layers.len() {
            return Ok(None);
        }
        let mut out = elab_expr(
            builder, &head_src, scope, univ, known, hovers, None, None, ctx,
        )?;
        let mut sigma: HashMap<String, Expr> = HashMap::new();
        for (i, a) in args.iter().enumerate() {
            let expected_src = crate::spine::substitute(&layers[i].domain, &sigma);
            let t = elab_expr(
                builder,
                a,
                scope,
                univ,
                known,
                hovers,
                None,
                Some(&expected_src),
                ctx,
            )?;
            out = builder.mk_app(out, t);
            if !layers[i].name.is_empty() {
                sigma.insert(layers[i].name.clone(), (*a).clone());
            }
        }
        return Ok(Some(out));
    }
    if layers.len() < k + args.len() {
        return Ok(None);
    }
    let span = expr.span();
    let head_term = elab_expr(
        builder, &head_src, scope, univ, known, hovers, None, None, ctx,
    )?;
    // 每个实参的**类型**（路线 ① 的原料：`arg_tys[i]` 对应第 `k + i` 层）。
    // 局部变量**优先取书写类型**（零内核调用，且不会像 pp 那样丢隐式实参）。
    //
    // **B3-③（模式 B/D，2026-09-30）**：这一段必须在**解出隐式前缀之前**、且
    // 实参**还没有**被 elaborate —— 实参的期望类型要等 `solved` 出来才能代换，
    // 所以组装放在求解之后（见下面 `sigma` 那一段）。以前这里先把**第一个**显式
    // 实参 `elab_expr(…, None)` 装好，于是"实参本身是**零元隐式常量**"的形状
    // （`Set.inter Set.univ A`、`Set.union Set.empty A`）拿不到期望类型 ⇒
    // `Set.univ` 停在 Pi 上 ⇒ 内核报
    // `期望 (Set.[] $1)，实际是 Pi (α : Sort(1)), (Set.[] $0)` ✗（实测）。
    let mut arg_tys: Vec<Option<Expr>> = Vec::with_capacity(args.len());
    for a in &args {
        arg_tys.push(operand_type_expr(
            ctx,
            scope,
            a,
            Some(&mut InplaceEnv {
                builder: &mut *builder,
                known,
            }),
        ));
    }
    // 期望类型/实参类型的 delta 展开要用 `defs` + `is_inductive`（`a ∈ A ∪ B`
    // 是 `Set.mem … (Set.union …)`，展开到 `Or …` 才能反解 `Or.inl` 的另一个析取项）。
    let is_inductive = |n: &str| ctx.inductives.contains_key(n);
    // **M3 的三通道**（**同一个码**，只有 message 说哪一句不同 ✓；D6 = 不新增码）：
    // `Unsolved` = 补不出（既有文案，逐字不变）· `Kind` = 值的 sort 确定不对 · `Clash` = 两条约束刚性冲突。
    let solved = match crate::compile::implicit::solve_prefix_outcome_with_args(
        &layers,
        &result,
        k,
        &arg_tys,
        &args,
        expected_src,
        ctx.defs,
        &is_inductive,
    ) {
        crate::compile::meta::MetaSolve::Solved(v) => v,
        channel => {
            let why = match channel {
                crate::compile::meta::MetaSolve::Kind => {
                    "**种类（sort/kind）不对**：反解出来的东西是一个**类型**，而这个位置要的是**项**（或反过来）——\
                     常见于把 `Nat`/`Type` 这类**类型**写在了要元素的位置。"
                }
                crate::compile::meta::MetaSolve::Clash => {
                    "**两条线索互相矛盾**：后续实参的类型与期望类型对同一个参数给出了**不一致**的要求。"
                }
                _ => "（本子集按「后续显式实参的类型 + 期望类型」反解）",
            };
            return Err(CompileError::elab(
                ErrorKind::ElabImplicitArgumentUnsolved,
                format!(
                    "`{}` 的签名 `{}` 里有 {} 个**隐式**参数，但补不出来{}。把参数写全，例如 `{} …` 逐位写下来",
                    render_msg(ctx, head),
                    ty_text,
                    k,
                    why,
                    render_msg(ctx, head)
                ),
                span,
            ));
        }
    };
    // 组装：先插隐式实参，再逐个装显式实参（**每个**都给「代入后」的期望类型）。
    //
    // **第一个实参与其余一视同仁**（B3-③，2026-09-30）：`solved` 已经出来了，
    // `layers[k].domain` 里的隐式参数可以代换 ⇒ 第一位也能拿到期望类型 ——
    // 这正是"实参本身是零元隐式常量"（`Set.inter Set.univ A`）能补出来的唯一
    // 条件 ✓。以前第一位不给，是因为它被提前 elaborate 了（见上面的注释）。
    let mut out = head_term;
    // **U1 片（开关默认关 ✓）**：常量**没写**宇宙实参时（头不是 `UniverseApp` ✓），
    // 按**字面约束**把未写的层级解出来再重建头常量 ✓（`Quot.lift α β f h` 一族 —— 台账 G-63 ✗）。
    //
    // ⚠ 两个要点：① 头**已经**照旧 `elab_expr` 过（hovers/定义跳转等副作用不能少 ✗），
    // 解出来才**替换**；② 开关关着 / 解不出 ⇒ 原样 ✓（`solve_universes` 自己会拒 ✓，
    // 关着时**一次都不进** ✓ ⇒ 逐字节不变 ✓）。
    // ⚠ 开关**先判**（关着时这里连 `pairs` 都不建 ⇒ 既有形状**零额外开销** ✓）
    // **U2-a（开关默认关 ✓）**：头写了 `.{n}` 时也进来 —— 但只做**检查** ✓（**显式优先** ✓）：
    // 解出来的字面与写出来的字面**冲突** ⇒ 报专用码 `ElabUniverseLevelConflict` ✗（不再一路落到内核 ✓）；
    // 相等 / 有一边不是字面 ⇒ **保留写出来的** ✓（一个字节都不改 ✓）。
    let written_levels: Option<&[String]> = match head {
        Expr::UniverseApp { levels, .. } => Some(levels.as_slice()),
        _ => None,
    };
    if crate::compile::level::universe_metavar_enabled()
        && (matches!(head, Expr::Ident { .. }) || written_levels.is_some())
    {
        // 模板侧必须**先用解出的项参数代换** ✓：`Box {u} : {α : Sort u} → α → Sort u` 的
        // `result` 是 `Sort u`，与期望类型 `Nat → Sort 1` 形状不同 ✗ ⇒ 代换后才同形 ✓。
        let mut lvl_sigma: HashMap<String, Expr> = HashMap::new();
        for (j, value) in solved.iter().enumerate() {
            if let Some(layer) = layers.get(j) {
                if !layer.name.is_empty() {
                    lvl_sigma.insert(layer.name.clone(), value.clone());
                }
            }
        }
        let subst = |e: &Expr| crate::spine::substitute(e, &lvl_sigma);
        let tpl_layers: Vec<Expr> = layers.iter().map(|l| subst(&l.domain)).collect();
        let tpl_result = subst(&result);
        // (模板, 实际) 的**三路**原料：
        //   ① 显式实参位：层域 ↔ 实参类型 ✓
        //   ② 结果 ↔ 期望类型 ✓
        //   ③ **解出来的隐式项参数自己的类型** ✓✓ —— `Show {u} : {α : Sort u} → α → Nat`
        //      写成 `Show 0` 时 `u` 只出现在 `{α : Sort u}` 那一层（**没有实参可问** ✗），
        //      但 `α` 已被解成 `Nat` ✓ ⇒ `Nat : Sort 1` ⇒ `u := 1` ✓✓
        //      （本轮实测：缺这一路 ⇒ `Show 0` 两态都判红 ✗，`期望 Sort(0)，实际是 Sort(1)`）。
        let mut tpl: Vec<Expr> = Vec::new();
        let mut act: Vec<Expr> = Vec::new();
        for (j, ty) in arg_tys.iter().enumerate() {
            if let (Some(ty), Some(layer)) = (ty.as_ref(), tpl_layers.get(k + j)) {
                tpl.push(layer.clone());
                act.push(ty.clone());
            }
        }
        if let Some(expected) = expected_src {
            tpl.push(tpl_result.clone());
            act.push(expected.clone());
        }
        for (j, value) in solved.iter().enumerate() {
            if let Some(layer) = tpl_layers.get(j) {
                if let Some(ty) = operand_type_expr(
                    ctx,
                    scope,
                    value,
                    Some(&mut InplaceEnv {
                        builder: &mut *builder,
                        known,
                    }),
                ) {
                    tpl.push(layer.clone());
                    act.push(ty);
                }
            }
        }
        let pairs: Vec<(&Expr, &Expr)> = tpl.iter().zip(act.iter()).collect();
        if crate::compile::level::trace_enabled() {
            eprintln!(
                "[u1] head={} univs={:?} k={} pairs={} solved_terms={}",
                head_name,
                declared.universes(),
                k,
                pairs.len(),
                solved.len()
            );
        }
        let solved_levels = crate::compile::level::solve_universes(declared.universes(), &pairs);
        if crate::compile::level::trace_enabled() {
            eprintln!("[u1]   solved_levels={:?}", solved_levels);
        }
        if let Some(written) = written_levels {
            // **显式优先** ✓：只比**字面 vs 字面** ✓（解出的是"下界/未定"⇒ 不判冲突 ✗）。
            if let Some(solved) = solved_levels.as_ref() {
                // **U2-b：约束攒着、收尾才查** ✓ —— 写出来的与解出来的都当**约束**推进
                // `LevelBatch` ✓，`check()` 一次性判 ✓（惰性只是时机，不是放水 ✓：
                // `lazy_and_eager_verdicts_are_identical` 单测钉死等价 ✓）。
                let mut batch = crate::compile::level::LevelBatch::new();
                for (i, (s, w)) in solved.iter().zip(written.iter()).enumerate() {
                    let (Ok(sn), Ok(wn)) = (s.parse::<u64>(), w.parse::<u64>()) else {
                        continue; // 有一边不是字面 ⇒ 本片不判 ✓（保守 ✓）
                    };
                    // 同一个位置的两条要求：解出来的 = `i` 号参数，写出来的也是 `i` 号 ✓
                    let key = format!("pos{i}");
                    batch.push(&key, sn);
                    batch.push(&key, wn);
                }
                if let crate::compile::level::LevelVerdict::Conflict { first, second, .. } =
                    batch.check()
                {
                    return Err(CompileError::elab(
                        ErrorKind::ElabUniverseLevelConflict,
                        format!(
                            "`{}` 的宇宙实参与实参类型要求的层级对不上：写的是 `{}`，而类型要求 `{}` \
                             —— 把它改成要求的那一个（或整段省掉，让引擎自己解 ✓）",
                            render_msg(ctx, head),
                            second,
                            first
                        ),
                        span,
                    ));
                }
            }
            // ⚠ **不早退** ✗：下面的组装（插隐式实参 + 装显式实参）还得跑 ✓ ——
            // 早退过一次，`Show.{1} 0` 被跳过组装 ⇒ 内核 `kernel-rejected` ✗（实测）。
        }
        // **显式优先** ✓：头写了 `.{n}` 时**不替换** head（写出来的就是权威 ✓），只走检查 ✓。
        if written_levels.is_none() {
            if let Some(levels) = solved_levels {
                let mut ptrs: Vec<LevelPtr<'a>> = Vec::with_capacity(levels.len());
                for text in &levels {
                    let mut lv = builder.zero();
                    for _ in 0..text.parse::<u64>().unwrap_or(0) {
                        lv = builder.succ(lv);
                    }
                    ptrs.push(lv);
                }
                let levels_ptr = builder.alloc_levels_slice(&ptrs);
                let head_ptr = builder.name_from_str(head_name);
                out = builder.mk_const(head_ptr, levels_ptr);
            }
        }
    }
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    for (j, s) in solved.iter().enumerate() {
        // **解出来的隐式实参也要吃期望类型**（T-N13，2026-09-30）：解出的值可能来自
        // **内核 pp 文本**（`operand_type_expr` 的第三条路），而 pp 会丢掉第一个隐式
        // 实参（`Set.empty Two` 打成 `Set.empty`）⇒ 不给期望类型，`Set.empty` 就停在
        // 自己的 Pi 上 ⇒ 内核报 `期望 (Set.[] Two.[])，实际是 Pi (α : Sort(1)), (Set.[] $0)` ✗
        // （实测：`congrArg.{1} (fun (X : Set Two) => f1 '' X) inter_singletons_empty`
        // —— 解出的 `b` 是 pp 回读的裸 `Set.empty`）。
        // 期望类型就是**该层的域**（已解出的参数先代进去）✓ —— 与写出来的实参同一口径 ✓。
        let expected_src = crate::spine::substitute(&layers[j].domain, &sigma);
        if !layers[j].name.is_empty() {
            sigma.insert(layers[j].name.clone(), s.clone());
        }
        let t = elab_expr(
            builder,
            s,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected_src),
            ctx,
        )?;
        out = builder.mk_app(out, t);
    }
    for (i, a) in args.iter().enumerate() {
        let li = k + i;
        let expected_src = crate::spine::substitute(&layers[li].domain, &sigma);
        let t = elab_expr(
            builder,
            a,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected_src),
            ctx,
        )?;
        out = builder.mk_app(out, t);
        if !layers[li].name.is_empty() {
            sigma.insert(layers[li].name.clone(), (*a).clone());
        }
    }
    Ok(Some(out))
}

/// 头部匹配 + 提取裸变量：`template` 是 `name` 本身 ⇒ 取 `actual`；两者是
/// 同头、同实参个数的应用链且某个实参位恰好是裸变量 `name` ⇒ 取 `actual`
/// 对应位的实参。其余形状返回 `None`（v1 不做一般合一，设计 N4.2）。
///
/// **`->` 也算一个二元头**（第二刀 §10.4 的实测增量）：`Set.image` 的签名是
/// `(α β : Type) → (f : α → β) → Set α → Set β`，`α`/`β` 只出现在 `f` 的
/// **箭头域/陪域**里；只认应用链的话 `''`/`⁻¹'` 的前导参数一个都补不出来
/// （实测 `elab-notation-argument-unsolved`）。所以这里把 `α → β` 与
/// `α₀ → β₀` 按域/陪域两个位置对齐——仍然是"裸变量匹配"，不引入元变量。
pub(crate) fn unify_extract(template: &Expr, actual: &Expr, name: &str) -> Option<Expr> {
    if let Expr::Ident { name: n, .. } = template {
        if n == name {
            return Some(actual.clone());
        }
    }
    // `->` 也算一个二元头（第二刀 §10.4 的实测增量）……而且 **`forall` 与 `->`
    // 必须等价**：内核 pp 把 lambda 的类型打成 `forall (n : Nat), Eq n n`，而
    // 签名里的 `A -> Prop` 解析成 `Expr::Arrow`——只认 Arrow 的话
    // `∃ (n : Nat), …` 的前导参数解不出来（第三刀实测）。`peel_pi` 是两者
    // 的唯一共用剥层（内核 pp 的多 binder 折叠也由它处理）。
    if let (Some(template_pi), Some(actual_pi)) = (
        crate::spine::peel_pi(template),
        crate::spine::peel_pi(actual),
    ) {
        if let Some(found) = unify_extract(&template_pi.domain, &actual_pi.domain, name)
            .or_else(|| unify_extract(&template_pi.body, &actual_pi.body, name))
        {
            return Some(found);
        }
    }
    // **模板是望远镜、实际不是**：把模板剥到**结果**再试。
    //
    // `solve_prefix_args` 的路线② 传进来的 `rest` 是「从第 i 层起的整个
    // 望远镜」（`(w : α) -> ( : p w) -> Exists α p`），而要解的参数常常只出现
    // 在**结果**里（`Exists α p` 的 `p`）——不剥到底就永远匹配不上
    // （L2.7 实测：`⟨a, ⟨b, h⟩⟩` 卡在这儿）。
    if crate::spine::peel_pi(template).is_some() {
        let mut cur = template.clone();
        while let Some(pi) = crate::spine::peel_pi(&cur) {
            cur = pi.body;
        }
        if let Some(found) = unify_extract(&cur, actual, name) {
            return Some(found);
        }
    }
    let (head_name, template_args, _) = head_and_args_notation(template)?;
    let (actual_head_name, actual_args, actual_head) = head_and_args_notation(actual)?;
    if head_name != actual_head_name {
        return None;
    }
    // **名字就是模板的头**（`p w` 里的 `p`，L2.7 实测）：要解的实参就是实际的
    // 头本身。`Exists.intro` 的第 2 个参数 `p` 正是这个形状——它只以 `p w`
    // 出现在后一层（`h : p w`）的域里，而"从 `p w` 反解 `p`"合法：`p w` 的
    // **头就是这个项**（不是它的某个实参）。旧路径只找「实参位上的名字」，
    // 于是 `⟨w, hw⟩` 在 `∃ (x : α), p x` 上报"补不出前面的类型参数"。
    if head_name == name && !template_args.is_empty() {
        return Some(actual_head);
    }
    // 实参**右对齐**，但只在「实际比模板少」时放行：记法只写操作数，前导类型
    // 参数由 elab 补——期望类型常常是 **binder 记法**（`∃ (x : α), p x` 是
    // `Exists α (fun (x : α) => p x)` 的记法形态，只有 1 个操作数，而模板有
    // 2 个实参）。实际比模板**多**仍然 `None`（那是另一种形状，不在这里猜）。
    if actual_args.len() > template_args.len() {
        return None;
    }
    let n = actual_args.len();
    let t_off = template_args.len() - n;
    let a_off = actual_args.len() - n;
    for k in 0..n {
        if let Expr::Ident { name: tname, .. } = &template_args[t_off + k] {
            if tname == name {
                return Some(actual_args[a_off + k].clone());
            }
        }
    }
    // **嵌套位置**（T-N13，2026-09-30）：模板该位是**含变量的复合式**时递归下去。
    //
    // 病根：迁移后的 pp 会丢掉**第一个隐式实参**（`Set.univ α` 打成 `Set.univ`），
    // 于是 `𝒫 (Set.univ α)` 的期望类型只剩 `Set (Set α)` 这一条线索——`α` 出现在
    // **嵌套的实参位**（`Set (Set ?α)` 的第二层），而 v1 只认「实参位恰好是裸变量」
    // ⇒ 路线② 永远匹配不上 ⇒ `Set.powerset` 报「补不出前面的类型参数」✗（实测：
    // unit11 的 `𝒫 (Set.univ α)`；`exact` 里对目标做一遍 pp 回读就是这个形状）。
    //
    // **仍然不是一般合一**：单侧结构匹配 + 只解一个变量；同一个变量在多个位置
    // 出现时要求**取值一致**（不一致 ⇒ `None`，不猜、不搜索 ✓）。既有那条
    // 「裸变量位取第一个命中」**原样保留**（先跑，命中即返回）⇒ 老行为逐字节不变 ✓。
    let mut found: Option<Expr> = None;
    for k in 0..n {
        let template_arg = &template_args[t_off + k];
        if !crate::spine::mentions(name, template_arg) {
            continue;
        }
        if let Some(v) = unify_extract(template_arg, &actual_args[a_off + k], name) {
            match &found {
                None => found = Some(v),
                Some(prev) if *prev == v => {}
                Some(_) => return None,
            }
        }
    }
    found
}

/// 头名 + 实参 + **头的表达式**：**记法节点按「目标名 + 操作数」算**。
///
/// 为什么需要（L2.7 实测）：期望类型常常是 binder 记法 `∃ (x : α), p x`，
/// 而 [`crate::spine::spine_of`] 只看得到记法节点本身（头不是 `Ident`）⇒
/// `unify_extract` 解不出 `p`，`⟨a, ⟨b, h⟩⟩` 这种嵌套匿名构造子就报
/// 「补不出前面的类型参数」。判据与 [`crate::spine::head_and_args`] 同一份。
fn head_and_args_notation(expr: &Expr) -> Option<(String, Vec<Expr>, Expr)> {
    if let Expr::Notation {
        target,
        assoc,
        lhs,
        rhs,
        ..
    } = expr
    {
        let mut args: Vec<Expr> = Vec::new();
        // **binder 记法**（`∃ (x : α), p x`）的**应用形态**是
        // `Exists α (fun (x : α) => p x)`：记法只写一个操作数（那个 lambda），
        // 而常量的第一个参数（域 `α`）在应用里也要占位。不补这一位，
        // `⟨a, ⟨b, h⟩⟩` 这种嵌套匿名构造子的前置参数就右对齐错位
        // （模板 2 个实参、实际 1 个）。
        if *assoc == crate::ast::NotationAssoc::Binder {
            if let Some(Expr::Lambda { binders, .. }) = rhs.as_deref() {
                if let Some(ty) = binders.first().and_then(|b| b.ty.as_deref()) {
                    args.push(ty.clone());
                }
            }
        }
        if let Some(l) = lhs {
            args.push((**l).clone());
        }
        if let Some(r) = rhs {
            args.push((**r).clone());
        }
        let head = Expr::Ident {
            name: target.clone(),
            span: expr.span(),
        };
        return Some((target.clone(), args, head));
    }
    let (head, args) = crate::spine::spine_of(expr);
    match head {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some((
            name.clone(),
            args.into_iter().cloned().collect(),
            head.clone(),
        )),
        _ => None,
    }
}

/// Peel one Pi layer off the expected type: returns the binder style, the
/// binder type and the remaining body. Used to infer untyped lambda binders
/// from the declared type of the surrounding declaration.
fn peel_expected<'a>(
    expected: Option<ExprPtr<'a>>,
) -> Option<(BinderStyle, ExprPtr<'a>, ExprPtr<'a>)> {
    match expected {
        Some(e) => match &*e {
            sokonanoda::expr::Expr::Pi {
                binder_style,
                binder_type,
                body,
                ..
            } => Some((*binder_style, *binder_type, *body)),
            _ => None,
        },
        None => None,
    }
}

/// Advance past one expected Pi layer without taking its binder (the binder
/// carries an explicit type annotation, so its kernel type comes from the
/// annotation instead).
fn drop_expected_layer(expected: Option<ExprPtr<'_>>) -> Option<ExprPtr<'_>> {
    match expected {
        Some(e) => match &*e {
            sokonanoda::expr::Expr::Pi { body, .. } => Some(*body),
            _ => None,
        },
        None => None,
    }
}

/// Source-level mirror of [`peel_expected`]: the binder style, the written
/// domain and the remaining source type.
fn peel_expected_src(expected: Option<&Expr>) -> Option<(BinderStyle, Expr, Expr)> {
    match expected? {
        Expr::Arrow {
            domain, codomain, ..
        } => Some((
            BinderStyle::Default,
            domain.as_ref().clone(),
            codomain.as_ref().clone(),
        )),
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            let binder = &binders[0];
            let domain = binder.ty.as_deref()?.clone();
            let rest = if binders.len() > 1 {
                Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: binder.span,
                }
            } else {
                body.as_ref().clone()
            };
            Some((kernel_binder_style(&binder.style), domain, rest))
        }
        _ => None,
    }
}

/// Source-level mirror of [`drop_expected_layer`].
fn drop_expected_src_layer(expected: Option<&Expr>) -> Option<Expr> {
    match expected? {
        Expr::Arrow { codomain, .. } => Some(codomain.as_ref().clone()),
        Expr::Forall { binders, body, .. } if !binders.is_empty() => {
            if binders.len() > 1 {
                Some(Expr::Forall {
                    binders: binders[1..].to_vec(),
                    body: body.clone(),
                    span: binders[0].span,
                })
            } else {
                Some(body.as_ref().clone())
            }
        }
        _ => None,
    }
}

/// `⟨a, b⟩` 的目标构造子：由**期望类型**的头决定（L2.7，路线 C）。
///
/// 认三类（顺序即优先级）：
/// 1. **prelude 里以 def 形态存在的单构造子类型**（`Iff` ⇒ `Iff.intro`）——
///    它在归纳表里查不到（是 def），但构造子名是固定的；
/// 2. **归纳表里的单构造子归纳**（`And` / `Exists` / `Prod` / 课程自定义）⇒
///    取它的构造子名。多构造子 ⇒ 报错（`⟨…⟩` 说不清是哪一个）；
/// 3. 其它头（`Or`、函数、`Prop`…）⇒ 报错，hint 说清为什么。
///
/// **不做合一**：头名直接来自期望类型的**源 AST**（记法节点用它的 target），
/// 与记法展开的"前导参数补全"同一条路线。
fn anon_ctor_target(
    expected: &Expr,
    ctx: &ElabCtx<'_, '_>,
    span: Span,
) -> Result<String, CompileError> {
    let head = crate::spine::head_and_args(expected).map(|(name, _)| name.to_string());
    let Some(head) = head else {
        return Err(CompileError::elab(
            ErrorKind::ElabAnonCtorNoExpectedType,
            format!(
                "`⟨…⟩` 的期望类型 `{}` 不是「头 + 参数」形状，看不出该用哪个构造子：改用点名构造子",
                render_msg(ctx, expected)
            ),
            span,
        ));
    };
    if let Some(ctor) = builtin_constructor_of(&head) {
        return Ok(ctor.to_string());
    }
    if let Some(info) = ctx.inductives.get(&head) {
        return match info.ctors.as_slice() {
            // 用**规范名**（`Exists.intro`），不是源里写的裸名（`intro`）：
            // 记法展开要的是安装进内核的那个名字（与 `match` 的 recursor 规则
            // 同一个字段）。
            [only] => Ok(only.canonical.clone()),
            [] => Err(CompileError::elab(
                ErrorKind::ElabAnonCtorNoExpectedType,
                format!("`{head}` 没有构造子，`⟨…⟩` 用不了"),
                span,
            )),
            many => Err(CompileError::elab(
                ErrorKind::ElabAnonCtorNoExpectedType,
                format!(
                    "`{head}` 有 {} 个构造子（{}），`⟨…⟩` 说不清用哪一个：写点名构造子",
                    many.len(),
                    many.iter()
                        .map(|c| c.canonical.as_str())
                        .collect::<Vec<_>>()
                        .join(" / ")
                ),
                span,
            )),
        };
    }
    Err(CompileError::elab(
        ErrorKind::ElabAnonCtorNoExpectedType,
        format!(
            "`⟨…⟩` 的期望类型头是 `{head}`——它既不是单构造子归纳，也没有固定的构造子：`⟨…⟩` 只能用在 `And` / `Exists` / `Prod` / `Iff` 这类「只有一个构造子」的类型上"
        ),
        span,
    ))
}

/// prelude 里**以 `def` 形态存在**的「单构造子类型」→ 它的构造子式引理。
///
/// `Iff` 展开成 `And (A -> B) (B -> A)`（`compile/prelude.rs` 的 L1 源码），
/// 所以它不在归纳表里，但 `Iff.intro` 的签名就是它的构造子。`by` 引擎的
/// `constructor`（L3.2）与 `⟨…⟩`（L2.7）共用这一份表——两处各写一份必然分叉。
pub(crate) fn builtin_constructor_of(head: &str) -> Option<&'static str> {
    match head {
        "Iff" => Some("Iff.intro"),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn elab_expr<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected: Option<ExprPtr<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
) -> Result<ExprPtr<'a>, CompileError> {
    // Lambda-headed application with untyped binders: infer the binder types
    // from the arguments' types (non-dependent case, I6). Source-to-source
    // rewrite, then the normal path elaborates the annotated lambda.
    if let Some(rewritten) = annotate_application_lambda(expr, ctx, scope) {
        return elab_expr(
            builder,
            &rewritten,
            scope,
            univ,
            known,
            hovers,
            expected,
            expected_src,
            ctx,
        );
    }
    match expr {
        Expr::Sort {
            sort: SortKind::Prop,
            span,
        } => {
            let z = builder.zero();
            let out = builder.mk_sort(z);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Type,
            span,
        } => {
            let z = builder.zero();
            let ty = builder.succ(z);
            let out = builder.mk_sort(ty);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Sort(n),
            span,
        } => {
            let mut level = builder.zero();
            for _ in 0..*n {
                level = builder.succ(level);
            }
            let out = builder.mk_sort(level);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Sort {
            sort: SortKind::Level(name),
            span,
        } => {
            // `Sort u`（纯名字）与 `Sort (u+1)`（层级算术）走同一条：名字先在
            // 本声明的宇宙参数里找；带 `+` 的层级文本交给 `level_ptr`。
            let level = match univ.get(name).copied() {
                Some(level) => level,
                None if name.contains('+') => level_ptr(builder, name, univ, *span)?,
                None => {
                    return Err(CompileError::elab(
                        ErrorKind::ElabUnknownUniverseLevel,
                        format!("universe variable `{name}` is not declared in this declaration"),
                        *span,
                    ));
                }
            };
            let out = builder.mk_sort(level);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Ident { name, span } => {
            let bound = scope.names.iter().rposition(|candidate| candidate == name);
            let (out, resolution) = match bound {
                Some(pos) => {
                    let idx = u16::try_from(scope.names.len() - 1 - pos).map_err(|_| {
                        CompileError::elab(
                            ErrorKind::ElabTooManyBinders,
                            "too many nested binders for kernel index",
                            *span,
                        )
                    })?;
                    (
                        builder.mk_var(idx),
                        Some(ResolvedTarget::Binder(scope.spans[pos])),
                    )
                }
                None => {
                    // R1/R2：裸名别名解析到**规范名**——只往 `known` 里加一个
                    // 裸名键会造出第二个内核常量（`mk` ≠ `P1.mk`）。
                    let canonical = resolve_known(known, ctx.ns, name, *span)?;
                    // The defining command's span is backfilled in `run_pass`
                    // (placeholder survives until then; prelude names resolve
                    // to no source definition and drop the record there).
                    let target = ResolvedTarget::Declaration {
                        name: canonical.clone(),
                        span: Span::default(),
                    };
                    let params = known[&canonical].universes();
                    let levels: Vec<LevelPtr<'a>> = params.iter().map(|_| builder.zero()).collect();
                    let levels = builder.alloc_levels_slice(&levels);
                    let name = builder.name_from_str(&canonical);
                    let bare = builder.mk_const(name, levels);
                    // **B3-①（缺口 G-40）**：签名带**前导隐式 binder** 的常量
                    // **裸着写**（零实参）时也要能补出来 —— 零元记法 `∅`
                    // （= 裸 `Set.empty`）正是这一档：以前它只是那个 **Pi**
                    // （`{α : Type} → Set α`）⇒ `rfl` 判不出来 ✗、课程库也就
                    // 没法把前导类型参数改成隐式 ✗。
                    //
                    // 走与 `try_implicit_application` **同一条**求解器
                    // （`implicit::solve_prefix` 的路线 ②：期望类型 vs 结果类型）。
                    // 三条安全性质：
                    //   · **免费闸门**：`implicit_prefix == 0` 一行不跑 ⇒ 老路
                    //     **逐字节不变** ✓；
                    //   · **没有期望类型就不动**（`expected_src` 为 `None` ⇒ 原样
                    //     返回 ✓）—— 裸常量当**函数值**用（`Set.empty` 本身）
                    //     时不会被误插 ✓；
                    //   · **解不出也原样返回**（不报错、不猜 ✗）—— 与 App 臂那条
                    //     不同：那里报 `elab-implicit-argument-unsolved` 是对的
                    //     （用户在写应用），这里"裸常量"本身就可能是想要的词项 ✓。
                    let bare = try_bare_implicit_constant(
                        builder,
                        known,
                        &canonical,
                        bare,
                        expected_src,
                        scope,
                        univ,
                        hovers,
                        ctx,
                    )?;
                    (bare, Some(target))
                }
            };
            record_hover(hovers, scope, *span, out, resolution);
            Ok(out)
        }
        Expr::UniverseApp { name, levels, span } => {
            let canonical = resolve_known_constant(known, ctx.ns, name, *span)?;
            let params = known[&canonical].universes();
            if params.len() != levels.len() {
                return Err(CompileError::elab(
                    ErrorKind::ElabUniverseArity,
                    format!(
                        "constant `{name}` expects {} universe argument(s), got {}",
                        params.len(),
                        levels.len()
                    ),
                    *span,
                ));
            }
            let mut resolved = Vec::with_capacity(levels.len());
            for level in levels {
                resolved.push(level_ptr(builder, level, univ, *span)?);
            }
            let levels = builder.alloc_levels_slice(&resolved);
            let name = builder.name_from_str(&canonical);
            let out = builder.mk_const(name, levels);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Num { value, span } => {
            let n: num_bigint::BigUint = value.parse().map_err(|_| {
                CompileError::elab(
                    ErrorKind::ElabInvalidNatLiteral,
                    format!("invalid natural literal `{value}`"),
                    *span,
                )
            })?;
            let ptr = builder.alloc_bignum(n).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabNatLiteralDisabled,
                    "Nat literals are disabled",
                    *span,
                )
            })?;
            let out = builder.mk_nat_lit(ptr).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabNatLiteralDisabled,
                    "Nat literals are disabled",
                    *span,
                )
            })?;
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Hole { span } => Err(CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "`sorry` is only allowed as the value of an open exercise",
            *span,
        )),
        Expr::App {
            fun,
            arg,
            explicit_spine,
            span,
        } => {
            // **IA-1 的唯一钩子**（设计 `docs/design/implicit-arguments.md` §3.2）：
            // 头如果是签名带**前导隐式 binder** 的常量/局部名，就把那些隐式实参
            // 解出来插进去。返回 `None` ⇒ 走下面的老路（签名没有隐式 binder 时
            // **逐字节不变**，这是 P1 可独立发布的安全性质，设计 §0）。
            if let Some(out) = try_implicit_application(
                builder,
                expr,
                *explicit_spine,
                scope,
                univ,
                known,
                hovers,
                expected_src,
                ctx,
            )? {
                // ⚠ **这条路会 early-return** ✗ ⇒ G-93 那条补层**必须也挂在这里** ✓
                // （实测：`myax α a` 走的就是这条 ⇒ 只挂下面那处 ⇒ 它永远补不到 ✗）。
                let out = infer_const_universes(builder, known, ctx, scope, expr, out)?;
                record_hover(hovers, scope, *span, out, None);
                return Ok(out);
            }
            // 实参的**期望类型**（设计 N4.2 ① 从记法操作数**推广到应用实参**）：
            // `subset_antisymm α A ∅ h …` 里 `∅` 是零元记法，拿不到期望类型就补不出
            // `α`（报 `elab-notation-argument-unsolved`）——课程 **227 处**
            // `Set.empty α` 全是这个形状，也正是 Lean 化改写最大的绊脚石。
            //
            // **只在实参真的需要时才算**（`needs_expected_type`）：取头的签名要问一次
            // 内核（有缓存），普通实参零开销、逐字节不变。
            // TODO(G-21)：这条推广（「应用实参也吃期望类型」）能修掉课程 227 处
            // `Set.empty α`，但在 `Eq.subst.{1} (Set α) … ` 这种「显式实参写在
            // 隐式位上」的调用里会把 `∅` 解成 `Set.empty A`（把上一个实参当成了
            // 类型）——**这就是它被关着的唯一原因**（判卷器那条线已修完，与本条
            // 无关）。**G-21 不在 `docs/gaps/ledger.jsonl` 里**：改动被 `if false`
            // 关着，出货二进制上复现不出，台账的 repro 契约套不上；记录在
            // `docs/design/course-lean-style.md` §9「另一条被 park 的改动」。
            // 重开属于 IA-2（R2.5）：先修「显式实参写在隐式位上」的实参→形参对齐。
            let arg_expected = if needs_expected_type(arg) {
                // `InplaceEnv` 的两个字段这里都现成（`builder` / `known`）——
                // 与 `elab_expr` 内 `Some(&mut InplaceEnv { .. })` 那处同款 ✓。
                // **B1 片（G-30 第 2 轮实测 ✓）**：⚠ **必须能回落到局部路** ✗ ——
                // 原先这里是 `if … else if arg_expected_enabled() { … }` ✗ ⇒
                // 「**需要期望类型**的实参」（正是集合字面量 `{a}` 那一类 ✓）**永远走不到**
                // 局部那条 ✗ ⇒ 头是**局部假设**（`ext : ∀ {A B : Set α}, …` ✓）时
                // 常量路答不出（它查 `known` ✗）⇒ 期望类型**丢了** ✗ ⇒ G-30 判红 ✓。
                application_arg_expected(expr, scope, ctx, Some(&mut InplaceEnv { builder, known }))
                    .or_else(|| b1_local_expected(fun, scope, ctx.defs))
            } else {
                // **B1 片**：头是**局部变量** ⇒ 用书写类型剥到实参位 ✓
                // （零内核调用 ⇒ 不会像 `application_arg_expected` 那样递归栈溢出 ✗）。
                b1_local_expected(fun, scope, ctx.defs)
            };
            let fun = elab_expr(builder, fun, scope, univ, known, hovers, None, None, ctx)?;
            let arg = elab_expr(
                builder,
                arg,
                scope,
                univ,
                known,
                hovers,
                None,
                arg_expected.as_ref(),
                ctx,
            )?;
            let out = builder.mk_app(fun, arg);
            // **G-58/G-59（0.81.0）**：裸 `Foo.rec`（没写 `.{u}`）的消去层级默认成
            // `0`（= Prop 动机）⇒「`Prop` 入、`Type` 出」与「`Type` 值归纳块消去到
            // `Type`」都写不出来。这里用**期望类型的宇宙**把它补回来。
            let out =
                infer_recursor_universes(builder, known, ctx, scope, expr, out, expected_src)?;
            // **G-93 真修（第 21 棒 ✓）**：裸常量的宇宙层**从签名与实参类型解出来** ✓
            // （递归子那条补的是**结果**层，这条补的是**参数**层 —— 两回事 ✓）。
            let out = infer_const_universes(builder, known, ctx, scope, expr, out)?;
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Lambda {
            binders,
            body,
            span,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            let mut styles = Vec::with_capacity(binders.len());
            let mut rest = expected;
            let mut rest_src = expected_src.cloned();
            for binder in binders {
                let (ty, style, src_ty) = match &binder.ty {
                    Some(ty) => {
                        let t =
                            elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx)?;
                        // The annotation wins, but the expected telescope
                        // still loses one layer so later untyped binders
                        // stay aligned with the declared type.
                        rest = drop_expected_layer(rest);
                        rest_src = drop_expected_src_layer(rest_src.as_ref());
                        (
                            t,
                            kernel_binder_style(&binder.style),
                            Some(ty.as_ref().clone()),
                        )
                    }
                    None => match peel_expected(rest) {
                        Some((style, binder_ty, body)) => {
                            let src_layer = peel_expected_src(rest_src.as_ref());
                            rest = Some(body);
                            rest_src = src_layer.as_ref().map(|(_, _, body)| body.clone());
                            let src_ty = src_layer.map(|(_, domain, _)| domain);
                            (binder_ty, style, src_ty)
                        }
                        None => {
                            return Err(CompileError::elab(
                                ErrorKind::ElabUntypedBinder,
                                "cannot infer the type of this binder: the declared type does not \
                                 provide a matching position (write it explicitly, e.g. fun (x : Nat) => x)",
                                binder.span,
                            ));
                        }
                    },
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(style);
                record_binder_hover(hovers, scope, binder.span, ty);
                scope.push(binder.name.clone(), ty, src_ty, binder.span);
            }
            let mut body_expr = elab_expr(
                builder,
                body,
                scope,
                univ,
                known,
                hovers,
                rest,
                rest_src.as_ref(),
                ctx,
            )?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_lambda(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr, None);
            Ok(body_expr)
        }
        Expr::Forall {
            binders,
            body,
            span,
        } => {
            let base = scope.len();
            let mut names = Vec::with_capacity(binders.len());
            let mut tys = Vec::with_capacity(binders.len());
            let mut styles = Vec::with_capacity(binders.len());
            for binder in binders {
                let ty = match &binder.ty {
                    Some(ty) => {
                        elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx)?
                    }
                    // 无注解的 Pi binder：**只有**两段式 binder 的 guard
                    // （`∀ x ∈ s, p`）能反解出 x 的类型（第三刀 §12.1）。
                    // 其余无注解 binder 走**逐字不变**的 `elab-untyped-binder`。
                    None => match split_arrow_guard(body, &binder.name).and_then(|guard| {
                        guarded_binder_type(
                            guard,
                            &binder.name,
                            scope,
                            known,
                            ctx,
                            Some(&mut InplaceEnv {
                                builder: &mut *builder,
                                known,
                            }),
                        )
                    }) {
                        Some(source_ty) => elab_expr(
                            builder, &source_ty, scope, univ, known, hovers, None, None, ctx,
                        )?,
                        None => {
                            return Err(CompileError::elab(
                                ErrorKind::ElabUntypedBinder,
                                "types must be written explicitly on Pi binders",
                                binder.span,
                            ));
                        }
                    },
                };
                let name = builder.name_from_str(&binder.name);
                tys.push(ty);
                names.push(name);
                styles.push(kernel_binder_style(&binder.style));
                record_binder_hover(hovers, scope, binder.span, ty);
                scope.push(
                    binder.name.clone(),
                    ty,
                    binder.ty.as_deref().cloned(),
                    binder.span,
                );
            }
            let mut body_expr =
                elab_expr(builder, body, scope, univ, known, hovers, None, None, ctx)?;
            scope.truncate(base);
            for ((name, ty), style) in names.into_iter().zip(tys).zip(styles).rev() {
                body_expr = builder.mk_pi(name, style, ty, body_expr);
            }
            record_hover(hovers, scope, *span, body_expr, None);
            Ok(body_expr)
        }
        Expr::Arrow {
            domain,
            codomain,
            span,
        } => {
            let domain_src = domain.as_ref().clone();
            let domain = elab_expr(builder, domain, scope, univ, known, hovers, None, None, ctx)?;
            // `A -> B` desugars to a Pi with an anonymous binder, so free
            // variables in the codomain live one binder deeper.
            scope.push(String::new(), domain, Some(domain_src), Span::default());
            let codomain = elab_expr(
                builder, codomain, scope, univ, known, hovers, None, None, ctx,
            )?;
            scope.truncate(scope.len() - 1);
            let anon = builder.anonymous();
            let out = builder.mk_pi(anon, BinderStyle::Default, domain, codomain);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Plus { lhs, rhs, span } => {
            // `+` is sugar for `Nat.add`; in bare mode (no Nat prelude) the
            // constant must not dangle — report a proper unknown identifier.
            if !known.contains_key("Nat.add") {
                return Err(CompileError::elab(
                    ErrorKind::ElabUnknownIdentifier,
                    "`+` needs Nat.add, which is not defined (install the prelude or define Nat yourself)",
                    *span,
                ));
            }
            let add = builder.name_from_str("Nat.add");
            let levels = builder.alloc_levels_slice(&[]);
            let add_const = builder.mk_const(add, levels);
            let lhs = elab_expr(builder, lhs, scope, univ, known, hovers, None, None, ctx)?;
            let rhs = elab_expr(builder, rhs, scope, univ, known, hovers, None, None, ctx)?;
            let applied = builder.mk_app(add_const, lhs);
            let out = builder.mk_app(applied, rhs);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        // 记号节点（G-04 / WO-011，设计 N4）：源到源降级成 `App` 形状。
        // 目标 telescope 比操作数多出来的**前导参数**由操作数类型 / 期望类型
        // 解出（裸变量匹配）；补全只发生在这条路径上——点名写法省参数**仍然
        // 被内核拒绝**（设计 N4.3 的护城河）。
        Expr::Notation {
            symbol,
            target,
            assoc,
            lhs,
            rhs,
            alternatives,
            span,
            // T-D14 的 `symbol_span`：**符号自己**的 span（不是整段节点）。
            // T-D15 用它当 `ResolvedTarget::Notation` 的 span ⇒ 记法符号从
            // "没有名字可解析"变成**一等目标**。
            symbol_span,
        } => {
            // binder 记法（第三刀 §12.1）：操作数是 `fun (x : A) => p`（两段式
            // 时 body 是 `And (x ∈ s) p`）。binder 没写类型时**先由 guard 反解**
            // 出类型、填进源级 AST 的 binder 注解，之后走与前缀记法**逐字相同**
            // 的展开路径（`notation_operand_expected` 给出 `A -> Prop`）。
            let annotated = if *assoc == NotationAssoc::Binder {
                binder_notation_operand(
                    symbol,
                    rhs.as_deref(),
                    scope,
                    known,
                    ctx,
                    Some(&mut InplaceEnv {
                        builder: &mut *builder,
                        known,
                    }),
                )?
            } else {
                None
            };
            let rhs = annotated.as_ref().or(rhs.as_deref());
            let operands: Vec<&Expr> = [lhs.as_deref(), rhs].into_iter().flatten().collect();
            // 记法重载（第三刀 §12.2）：`target` 是主候选，`alternatives` 是其余。
            let mut candidates: Vec<&str> = Vec::with_capacity(1 + alternatives.len());
            candidates.push(target.as_str());
            candidates.extend(alternatives.iter().map(String::as_str));
            let chosen =
                choose_notation_target(&candidates, symbol, *span, known, expected_src, ctx)?;
            let out = elab_notation(
                builder,
                symbol,
                chosen,
                &operands,
                *span,
                scope,
                univ,
                known,
                hovers,
                expected_src,
                None,
                ctx,
            )?;
            // **T-D15**：记法符号是一等目标——`resolution` 指向**使用处那个符号
            // 自己**（T-D14 的 `symbol_span`）。以前这里是 `None`，于是"光标在
            // 符号上"会掉进两个回退里、误命中外层 binder（T-D30 的守卫是权宜之计）。
            record_hover(
                hovers,
                scope,
                *span,
                out,
                Some(ResolvedTarget::Notation {
                    symbol: symbol.clone(),
                    span: *symbol_span,
                    module: None,
                }),
            );
            Ok(out)
        }
        // **集合字面量**（第三刀 §12.4）：新语法，内建糖——展开成点名形式
        // `Set.singleton α a` / `Set.pair α a b`（与 `+` → `Nat.add` 同族）。
        // 复用记法展开的前导参数补全与操作数期望类型传播：`α` 由元素类型或
        // 期望类型解出（`{∅}` 走期望类型那条路）。
        Expr::SetLiteral { elements, span } => {
            let (symbol, target) = if elements.len() == 1 {
                ("{a}", "Set.singleton")
            } else {
                ("{a, b}", "Set.pair")
            };
            let Ok(canonical) = resolve_known(known, ctx.ns, target, *span) else {
                return Err(CompileError::elab(
                    ErrorKind::ElabSetLiteralUnknownTarget,
                    format!(
                        "集合字面量 `{symbol}` 展开成点名形式 `{target}`，但这个文件里没有 `{target}`：先 `import` 提供它的库（卷 I 的 `lib/Set`），或改用点名写法"
                    ),
                    *span,
                ));
            };
            let operands: Vec<&Expr> = elements.iter().collect();
            // 元素类型 `α` 的**回退解**：期望类型 `Set α₀` ⇒ `α := α₀`。
            // `{∅}` 这类"元素自己的类型也要从期望类型解"的嵌套只有这条路
            // （元素类型的裸变量匹配结构上够不着，见 `set_literal_prefix_args`）。
            let fallback = set_literal_prefix_args(target, expected_src, *span, known, ctx);
            let out = elab_notation(
                builder,
                symbol,
                target,
                &operands,
                *span,
                scope,
                univ,
                known,
                hovers,
                expected_src,
                fallback.as_deref(),
                ctx,
            )?;
            // **A3**（用户 2026-09-26 报告第 3 条）：`{a}` 要能跳转到它展开成的
            // `Set.singleton`。为什么以前不行：`∈` 那类**记法符号**走
            // `notation_at`（查记法表 ✓），而 `{a}` 是**内建语法**、不在记法表里
            // ⇒ 那条分支够不着；`definition_at` 又只读 hover 的 `resolution`，
            // 这里以前记的是 `None` ⇒ F12 直接 `null`。
            // 记法与点名的 `ResolvedTarget::Declaration` 同形：真实位置由既有
            // 回填（`kernel_phase` 的 `top_level_def_spans_over`）给，跨文件
            // 跳转走 `project_definition` ⇒ 与普通名字**同一条通道**。
            record_hover(
                hovers,
                scope,
                *span,
                out,
                Some(ResolvedTarget::Declaration {
                    name: canonical,
                    span: Span::default(),
                }),
            );
            Ok(out)
        }
        // **匿名构造子**（课程 Lean 化 L2.7）：`⟨a, b⟩`。
        //
        // 用哪个构造子由**期望类型**决定（路线 C：不做合一、不引入元变量）。
        // 认三类头：内建记法的**目标名**（`A ∧ B` 的记法节点 target = `And`）、
        // 归纳表里的**单构造子**归纳（`And`/`Exists`/`Prod`/课程自定义）、以及
        // prelude 里以 **def** 形态存在的 `Iff`（构造子 `Iff.intro`）。
        //
        // 展开**复用记法路径**（`elab_notation`）：前导参数补全（`⟨w, hw⟩` 在
        // `∃ (x : α), p x` 上要解出 `α` 与 `p`）与操作数期望类型传播都是既有
        // 机械，`⟨a, ⟨b, c⟩⟩` 的嵌套因此天然可用（内层的期望类型由外层
        // 操作数位给出）。
        Expr::AnonCtor { elements, span } => {
            let Some(expected) = expected_src else {
                return Err(CompileError::elab(
                    ErrorKind::ElabAnonCtorNoExpectedType,
                    "`⟨…⟩` 用哪个构造子由**期望类型**决定，这里读不到期望类型：把它写进有类型标注的位置（`have h : T := ⟨…⟩`、`exact ⟨…⟩` 的目标、声明类型），或改用点名构造子（`And.intro` / `Exists.intro` / `Prod.mk`）",
                    *span,
                ));
            };
            let target = anon_ctor_target(expected, ctx, *span)?;
            let operands: Vec<&Expr> = elements.iter().collect();
            let out = elab_notation(
                builder,
                "⟨…⟩",
                &target,
                &operands,
                *span,
                scope,
                univ,
                known,
                hovers,
                Some(expected),
                None,
                ctx,
            )?;
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        Expr::Let {
            binder,
            val,
            body,
            span,
        } => {
            let base = scope.len();
            // 1) binder 类型在「未引入 x」的外层 scope 里 elaborate；无注解时
            //    问内核推断值 `v` 的类型（`judge_infer`，复用有界缓存）。
            let mut inferred_src: Option<Expr> = None;
            let ty = match binder.ty.as_deref() {
                Some(src_ty) => {
                    elab_expr(builder, src_ty, scope, univ, known, hovers, None, None, ctx)?
                }
                None => {
                    let binders = scope.judge_binders();
                    let text =
                        judge_infer(ctx.prefix_src, ctx.options, &binders, &render_expr(val))
                            .map_err(|_| {
                                CompileError::elab(
                            ErrorKind::ElabLetTypeQueryFailed,
                            "无法推断 `let` 绑定的类型；请补上类型标注，例如 `let x : Nat := 1; x`",
                            binder.span,
                        )
                            })?;
                    let parsed = crate::proof::parse_expr_text(&text).map_err(|_| {
                        CompileError::elab(
                            ErrorKind::ElabLetTypeQueryFailed,
                            "无法解析推断出的 `let` 绑定类型",
                            binder.span,
                        )
                    })?;
                    let kernel = elab_expr(
                        builder, &parsed, scope, univ, known, hovers, None, None, ctx,
                    )?;
                    inferred_src = Some(parsed);
                    kernel
                }
            };
            let ty_src = inferred_src.as_ref().or(binder.ty.as_deref());
            // 2) binder 声明行 hover（`x : T`），scope 仍是外层。
            record_binder_hover(hovers, scope, binder.span, ty);
            // 3) 值在期望类型 T 下 elaborate（未注解的 lambda binder 可借此推断）。
            let val = elab_expr(
                builder,
                val,
                scope,
                univ,
                known,
                hovers,
                Some(ty),
                ty_src,
                ctx,
            )?;
            // 4) 引入 x，body 在扩展 scope + 外层 expected 下 elaborate。
            scope.push(binder.name.clone(), ty, ty_src.cloned(), binder.span);
            let body = elab_expr(
                builder,
                body,
                scope,
                univ,
                known,
                hovers,
                expected,
                expected_src,
                ctx,
            )?;
            scope.truncate(base);
            // 5) 拼内核 Let 并落 hover（`nondep` 保守取 false，见设计 §3.3）。
            let name = builder.name_from_str(&binder.name);
            let out = builder.mk_let(name, ty, val, body, false);
            record_hover(hovers, scope, *span, out, None);
            Ok(out)
        }
        // `match`：降低为 `<Ind>.rec.{level} (fun (_ : Ind) => R) minor… scrutinee`
        // （design `docs/design/match.md` §5）。判定交给完整内核。
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let (Some(_expected_kernel), Some(expected_src)) = (expected, expected_src) else {
                return Err(CompileError::elab(
                    ErrorKind::ElabMatchNoExpectedType,
                    "`match` 的结果类型必须已知：请把它放在有类型标注的位置（声明类型 / \
                     let / fun 的 binder 注解），或让外层 match 提供结果类型",
                    *span,
                ));
            };
            // 1) elaborate scrutinee (no expected), then find its inductive head.
            let scrutinee_kernel = elab_expr(
                builder, scrutinee, scope, univ, known, hovers, None, None, ctx,
            )?;
            let (ind_name, written_args) =
                infer_inductive_with_params(ctx, scope, scrutinee).ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabMatchNotInductive,
                        "`match` 的被匹配项不是已知的归纳类型（本文件用 inductive 声明，或 prelude 的 Nat/Bool）",
                        scrutinee.span(),
                    )
                })?;
            let info = ctx.inductives.get(&ind_name).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabMatchNotInductive,
                    format!(
                        "`match` 的被匹配项类型 `{ind_name}` 不是已知的归纳类型（本文件用 inductive 声明，或 prelude 的 Nat/Bool）"
                    ),
                    scrutinee.span(),
                )
            })?;
            // 参数化归纳的 params：scrutinee 必须是书写源类型为 `Ind p1 … pn`
            // 的局部变量；取前 num_params 个源实参。
            let param_args_src: Vec<Expr> = if info.num_params == 0 {
                Vec::new()
            } else {
                let args = written_args.as_ref().ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：被匹配项必须是一个书写类型为 `{ind_name} …` 的局部变量"
                        ),
                        scrutinee.span(),
                    )
                })?;
                if args.len() < info.num_params {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：被匹配项的书写类型需要显式给出 {} 个参数",
                            info.num_params
                        ),
                        scrutinee.span(),
                    ));
                }
                args.iter().take(info.num_params).cloned().collect()
            };
            // 参数实参在进入构造子字段作用域之前 elaborate。
            let param_kernel: Vec<ExprPtr<'a>> = param_args_src
                .iter()
                .map(|arg| elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx))
                .collect::<Result<Vec<_>, _>>()?;
            // 带索引归纳：scrutinee 书写类型里参数之后是索引实参（如
            // `v : Vec A n` 的 `n`）；motive/recursor 应用都要用它们。
            let index_args_src: Vec<Expr> = if info.num_indices == 0 {
                Vec::new()
            } else {
                let args = written_args.as_ref().ok_or_else(|| {
                    CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个带索引归纳形状 `{ind_name}`：被匹配项必须是一个书写类型为 `{ind_name} <参数> <索引>` 的局部变量"
                        ),
                        scrutinee.span(),
                    )
                })?;
                if args.len() < info.num_params + info.num_indices {
                    return Err(CompileError::elab(
                        ErrorKind::ElabMatchParameterizedUnsupported,
                        format!(
                            "`match` 暂不支持这个带索引归纳形状 `{ind_name}`：被匹配项的书写类型需要显式给出 {} 个参数 + {} 个索引",
                            info.num_params, info.num_indices
                        ),
                        scrutinee.span(),
                    ));
                }
                args.iter()
                    .skip(info.num_params)
                    .take(info.num_indices)
                    .cloned()
                    .collect()
            };
            let index_kernel: Vec<ExprPtr<'a>> = index_args_src
                .iter()
                .map(|arg| elab_expr(builder, arg, scope, univ, known, hovers, None, None, ctx))
                .collect::<Result<Vec<_>, _>>()?;
            // 递归归纳：字段源类型必须可知（用于定位递归字段并插 IH）。
            if info.recursive
                && info
                    .ctors
                    .iter()
                    .flat_map(|c| c.fields.iter())
                    .any(|f| f.src_ty.is_none())
            {
                return Err(CompileError::elab(
                    ErrorKind::ElabMatchRecursiveUnsupported,
                    format!(
                        "`match` 暂不支持这个递归归纳形状 `{ind_name}`：构造子字段缺少源类型，无法定位归纳假设"
                    ),
                    scrutinee.span(),
                ));
            }
            // 2) 模式编译器：canonical 化成「每构造子恰好一条 arm」，处理
            //    通配/绑定/嵌套构造子/Nat 字面量/守卫（docs/design/match-patterns.md §4）。
            let top_vars = vec![ColVar {
                expr: (**scrutinee).clone(),
                ind: Some(info),
                ind_name: Some(ind_name.clone()),
                subst: info
                    .param_names
                    .iter()
                    .cloned()
                    .zip(param_args_src.iter().cloned())
                    .collect(),
            }];
            let top_rows: Vec<PatternRow> = arms
                .iter()
                .map(|arm| PatternRow {
                    pats: vec![arm.pattern.clone()],
                    guard: arm.guard.clone(),
                    body: arm.body.clone(),
                })
                .collect();
            let compiled = compile_pattern_body(ctx, &top_vars, &top_rows, *span)?;
            let Expr::Match {
                arms: canon_arms, ..
            } = compiled
            else {
                // 所有模式都不可反驳（`| _ => …` / `| y => …`）：直接用替换后的 body。
                return elab_expr(
                    builder,
                    &compiled,
                    scope,
                    univ,
                    known,
                    hovers,
                    expected,
                    Some(expected_src),
                    ctx,
                );
            };
            // 依赖 motive 触发（v1，design docs/design/match-dependent-motive.md §1）：
            // scrutinee 是裸局部变量 `x`，且 `x` 在结果类型 R 中出现。否则保持
            // 常量 motive（完全兼容既有行为）。
            let dependent_var: Option<String> = match &**scrutinee {
                Expr::Ident { name, .. }
                    if scope.names.iter().any(|n| n == name)
                        && mentions_ident(expected_src, name) =>
                {
                    Some(name.clone())
                }
                _ => None,
            };
            // 3) level：judge_infer(R) 的类型文本映射宇宙（design §5 step 3）。
            let level = infer_expected_level(ctx, scope, expected_src).ok_or_else(|| {
                CompileError::elab(
                    ErrorKind::ElabMatchNoExpectedType,
                    "无法确定 `match` 结果类型所在的宇宙层级（v1 只支持内核能推断出 Sort 的结果类型）",
                    *span,
                )
            })?;
            // 4) motive：依赖时为 `fun (t : Ind params) => R[x := t]`，否则
            //    `fun (_ : Ind params) => R`（v1 非依赖，见设计 §2）。
            // motive 域的书写源类型 = `Ind params`（从 scrutinee 的书写参数）。
            let ind_ty_src = param_args_src.iter().fold(
                Expr::Ident {
                    name: ind_name.clone(),
                    span: *span,
                },
                |acc, p| Expr::App {
                    fun: Box::new(acc),
                    arg: Box::new(p.clone()),
                    explicit_spine: false,
                    span: *span,
                },
            );
            let (motive_name, body_src) = if let Some(x) = &dependent_var {
                // 新鲜 motive binder 名 `t`：避让作用域内全部名字（包括 x）。
                let motive_name = {
                    let mut candidate = String::from("t");
                    let mut k = 1;
                    while scope.names.iter().any(|n| n == &candidate) {
                        k += 1;
                        candidate = format!("t{k}");
                    }
                    candidate
                };
                let mut map = HashMap::new();
                map.insert(
                    x.clone(),
                    Expr::Ident {
                        name: motive_name.clone(),
                        span: *span,
                    },
                );
                let body_src = super::goals::substitute_names(expected_src, &map, &HashMap::new());
                (motive_name, body_src)
            } else {
                (String::new(), expected_src.clone())
            };
            // body 必须在 motive binder 的作用域里 elaborate：`mk_lambda` 不做
            // de Bruijn shift，body 的索引须相对扩展后的上下文。
            let outer = scope.len();
            // 带索引归纳：motive 先绑索引再绑 major（`Ind params i1…ik`），与内核
            // `mk_motive_dep` 一致。索引名新鲜、类型代入参数实参。
            let param_map: HashMap<String, Expr> = info
                .param_names
                .iter()
                .cloned()
                .zip(param_args_src.iter().cloned())
                .collect();
            let mut index_binder_srcs: Vec<(String, Expr)> = Vec::new();
            for (k, ty) in info.index_types.iter().enumerate() {
                let name = {
                    let mut candidate = format!("__soko_i{k}");
                    while scope.names.iter().any(|n| n == &candidate) {
                        candidate.push('_');
                    }
                    candidate
                };
                let ty = super::goals::substitute_names(ty, &param_map, &HashMap::new());
                let ty_kernel =
                    elab_expr(builder, &ty, scope, univ, known, hovers, None, None, ctx)?;
                scope.push(name.clone(), ty_kernel, Some(ty.clone()), *span);
                index_binder_srcs.push((name, ty));
            }
            // major 的书写源类型 = `Ind <参数实参> <索引名…>`。
            let ind_ty_src =
                index_binder_srcs
                    .iter()
                    .fold(ind_ty_src, |acc, (name, _)| Expr::App {
                        fun: Box::new(acc),
                        arg: Box::new(Expr::Ident {
                            name: name.clone(),
                            span: *span,
                        }),
                        explicit_spine: false,
                        span: *span,
                    });
            let ind_kernel = elab_expr(
                builder,
                &ind_ty_src,
                scope,
                univ,
                known,
                hovers,
                None,
                None,
                ctx,
            )?;
            scope.push(motive_name.clone(), ind_kernel, Some(ind_ty_src), *span);
            let motive_body = elab_expr(
                builder, &body_src, scope, univ, known, hovers, None, None, ctx,
            )?;
            scope.truncate(outer);
            let motive_name_ptr = if motive_name.is_empty() {
                builder.anonymous()
            } else {
                builder.name_from_str(&motive_name)
            };
            let mut motive = builder.mk_lambda(
                motive_name_ptr,
                BinderStyle::Default,
                ind_kernel,
                motive_body,
            );
            for (name, ty) in index_binder_srcs.iter().rev() {
                let ty_kernel =
                    elab_expr(builder, ty, scope, univ, known, hovers, None, None, ctx)?;
                let name_ptr = builder.name_from_str(name);
                motive = builder.mk_lambda(name_ptr, BinderStyle::Default, ty_kernel, motive);
            }
            // 5) minors：按构造子声明序重排；**递归字段后插入归纳假设 IH**
            //    （类型 = motive 结果 R，v1 非依赖 motive；design §5 / Phase 2）。
            let base = scope.len();
            let mut minors = Vec::with_capacity(info.ctors.len());
            for (ctor, canon) in info.ctors.iter().zip(canon_arms.iter()) {
                let arm = canon;
                // canonical arm 的参数就是该构造子的字段绑定（顺序与 fields 对齐）。
                let binders: Vec<Binder> = match &arm.pattern {
                    Pattern::Ident { args, .. } => args
                        .iter()
                        .map(|a| Binder {
                            name: match a {
                                Pattern::Ident { name, .. } => name.clone(),
                                _ => String::new(),
                            },
                            ty: None,
                            style: BinderKind::Explicit,
                            span: a.span(),
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let mut minor_binders: Vec<(String, BinderStyle, ExprPtr<'a>, Option<Expr>)> =
                    Vec::new();
                // 字段原名 → 用户模式里的绑定名：后面的字段类型可能引用前面的
                // 字段（`v : Vec A n` 引用 `n`），elaborate 前必须改名。
                let mut field_rename: HashMap<String, Expr> = HashMap::new();
                let param_map: HashMap<String, Expr> = info
                    .param_names
                    .iter()
                    .cloned()
                    .zip(param_args_src.iter().cloned())
                    .collect();
                for (field, binder) in ctor.fields.iter().zip(binders.iter()) {
                    // 参数化归纳：把字段源类型里的参数名代换成 scrutinee 的
                    // 书写实参后再 elaborate，得到该构造子在其实例下的字段类型。
                    let parameterized = info.num_params > 0;
                    let renamed_src = field.src_ty.as_ref().map(|src| {
                        let mut map = param_map.clone();
                        map.extend(field_rename.clone());
                        super::goals::substitute_names(src, &map, &HashMap::new())
                    });
                    let (field_ty, field_src_ty): (ExprPtr<'a>, Option<Expr>) = if !parameterized {
                        (field.ty, renamed_src)
                    } else {
                        let src = renamed_src.ok_or_else(|| {
                            CompileError::elab(
                                ErrorKind::ElabMatchParameterizedUnsupported,
                                format!(
                                    "`match` 暂不支持这个参数化归纳形状 `{ind_name}`：构造子 `{}` 的字段缺少源类型",
                                    ctor.name
                                ),
                                binder.span,
                            )
                        })?;
                        let k =
                            elab_expr(builder, &src, scope, univ, known, hovers, None, None, ctx)?;
                        (k, Some(src))
                    };
                    if !field.name.is_empty() && field.name != binder.name {
                        field_rename.insert(
                            field.name.clone(),
                            Expr::Ident {
                                name: binder.name.clone(),
                                span: binder.span,
                            },
                        );
                    }
                    record_binder_hover(hovers, scope, binder.span, field_ty);
                    scope.push(
                        binder.name.clone(),
                        field_ty,
                        field_src_ty.clone(),
                        binder.span,
                    );
                    minor_binders.push((
                        binder.name.clone(),
                        field.style,
                        field_ty,
                        field_src_ty.clone(),
                    ));
                    let recursive_field = info.recursive
                        && field_src_ty
                            .as_ref()
                            .is_some_and(|t| mentions_ident(t, &ind_name));
                    if recursive_field {
                        // 归纳假设名避开既有绑定（`ih`、`ih2`、…），供 branch 引用。
                        let ih_name = {
                            let used = |name: &str| {
                                scope.names.iter().any(|n| n == name)
                                    || minor_binders.iter().any(|(n, ..)| n == name)
                            };
                            let mut candidate = String::from("ih");
                            let mut k = 1;
                            while used(&candidate) {
                                k += 1;
                                candidate = format!("ih{k}");
                            }
                            candidate
                        };
                        // 依赖 motive：IH 类型 = motive <field> = R[x := field]；
                        // 否则保持常量 R（Phase 2 非依赖）。须在当前 minor 作用域
                        // 里 elaborate（索引相对已推入的字段/IH binder）。
                        let ih_src = if let Some(x) = &dependent_var {
                            let mut map = HashMap::new();
                            map.insert(
                                x.clone(),
                                Expr::Ident {
                                    name: binder.name.clone(),
                                    span: binder.span,
                                },
                            );
                            super::goals::substitute_names(expected_src, &map, &HashMap::new())
                        } else {
                            expected_src.clone()
                        };
                        let ih_kernel = elab_expr(
                            builder, &ih_src, scope, univ, known, hovers, None, None, ctx,
                        )?;
                        record_binder_hover(hovers, scope, binder.span, ih_kernel);
                        scope.push(
                            ih_name.clone(),
                            ih_kernel,
                            Some(ih_src.clone()),
                            binder.span,
                        );
                        minor_binders.push((
                            ih_name,
                            BinderStyle::Default,
                            ih_kernel,
                            Some(ih_src),
                        ));
                    }
                }
                // 依赖 motive：分支期望类型 = R[x := C params v…]（把 scrutinee
                // 变量替换成该分支的构造子项）；否则保持常量 R。同样在当前 minor
                // 作用域里 elaborate（branch body 就在这个作用域下）。
                let branch_expected_src = if let Some(x) = &dependent_var {
                    let mut term = Expr::Ident {
                        name: ctor.name.clone(),
                        span: arm.span,
                    };
                    for p in &param_args_src {
                        term = Expr::App {
                            fun: Box::new(term),
                            arg: Box::new(p.clone()),
                            explicit_spine: false,
                            span: arm.span,
                        };
                    }
                    for b in &binders {
                        term = Expr::App {
                            fun: Box::new(term),
                            arg: Box::new(Expr::Ident {
                                name: b.name.clone(),
                                span: b.span,
                            }),
                            explicit_spine: false,
                            span: arm.span,
                        };
                    }
                    let mut map = HashMap::new();
                    map.insert(x.clone(), term);
                    super::goals::substitute_names(expected_src, &map, &HashMap::new())
                } else {
                    expected_src.clone()
                };
                let branch_expected = elab_expr(
                    builder,
                    &branch_expected_src,
                    scope,
                    univ,
                    known,
                    hovers,
                    None,
                    None,
                    ctx,
                )?;
                let mut body = elab_expr(
                    builder,
                    &arm.body,
                    scope,
                    univ,
                    known,
                    hovers,
                    Some(branch_expected),
                    Some(&branch_expected_src),
                    ctx,
                )?;
                scope.truncate(base);
                for (name, style, ty, _src) in minor_binders.iter().rev() {
                    let nm = builder.name_from_str(name);
                    body = builder.mk_lambda(nm, *style, *ty, body);
                }
                minors.push(body);
            }
            // 6) `<Ind>.rec.{level} params motive minor_1 … minor_n scrutinee`.
            let rec_ptr = builder.name_from_str(&info.recursor);
            let rec_const = if info.rec_universe_arity == 0 {
                // Prop 小消去推导出的 recursor 没有宇宙参数（如多构造子 Prop 枚举）。
                let levels = builder.alloc_levels_slice(&[]);
                builder.mk_const(rec_ptr, levels)
            } else {
                let lvl = level_from_u64(builder, level);
                let levels = builder.alloc_levels_slice(&[lvl]);
                builder.mk_const(rec_ptr, levels)
            };
            let mut app = rec_const;
            for param in &param_kernel {
                app = builder.mk_app(app, *param);
            }
            app = builder.mk_app(app, motive);
            for minor in minors {
                app = builder.mk_app(app, minor);
            }
            for index in &index_kernel {
                app = builder.mk_app(app, *index);
            }
            app = builder.mk_app(app, scrutinee_kernel);
            record_hover(hovers, scope, *span, app, None);
            Ok(app)
        }
        // `by` 块应在 elab 前由引擎降级为 lambda AST；到不了这里。
        Expr::By { span, .. } => Err(CompileError::elab(
            ErrorKind::ElabHoleMisplaced,
            "internal: `by` block reached elaboration without being lowered",
            *span,
        )),
    }
}

/// Peel a constructor's elaborated kernel type into its field binder types
/// (dependencies resolved by de Bruijn), in declaration order.
fn kernel_field_binders<'a>(mut ty: ExprPtr<'a>) -> Vec<(BinderStyle, ExprPtr<'a>)> {
    let mut out = Vec::new();
    loop {
        match &*ty {
            sokonanoda::expr::Expr::Pi {
                binder_style,
                binder_type,
                body,
                ..
            } => {
                out.push((*binder_style, *binder_type));
                ty = *body;
            }
            _ => return out,
        }
    }
}

/// The head identifier of a type expression (`Color`, `Color A`, `@{…}`), used
/// to map a scrutinee/expected type onto the inductive registry.
fn head_ident(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => Some(name.clone()),
        Expr::App { fun, .. } => head_ident(fun),
        _ => None,
    }
}

/// Flatten a source type application `C p1 … pn` into its head and arguments
/// (owned clones, for `match` parameter instantiation). Non-spine heads yield
/// `None`.
///
/// **记法也算 spine**（0.62.0，R3 实测补）：`h : B ∨ C` 的书写类型是记法节点，
/// 但它的**源像**就是 `target` 那条 spine（`Or B C`）——记法声明本身就写明了
/// 目标点名。不认这一层，`cases h`（参数化归纳要读书写类型的参数）会拒绝
/// Lean 风格里的常见写法「`have h : A ∨ B := …` 之后 `cases h`」，逼学习者
/// 把局部假设的类型写成点名形式。操作数按**源序**收集：中缀 `[lhs, rhs]`、
/// 前缀 `[rhs]`（`¬ A`）、后缀 `[lhs]`、零元 `[]`（`∅`）。
fn src_spine(expr: &Expr) -> Option<(String, Vec<Expr>)> {
    match expr {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => {
            Some((name.clone(), Vec::new()))
        }
        Expr::App { fun, arg, .. } => {
            let (head, mut args) = src_spine(fun)?;
            args.push(arg.as_ref().clone());
            Some((head, args))
        }
        Expr::Notation {
            target,
            assoc,
            lhs,
            rhs,
            ..
        } => {
            let mut args = Vec::new();
            // **binder 记法**（`∃ (x : α), p x`）的应用形态是
            // `Exists α (fun (x : α) => p x)`：记法只写那个 lambda，而常量的
            // 第一个参数（域 `α`）在应用里也要占位。漏掉这一位，参数化归纳
            // （`Exists`）就只拿到 1 个实参 ⇒ `match` 报「书写类型需要显式给出
            // 2 个参数」（2026-09-21 实测：`lib/Image` 的 `Set.image` 定义体改用
            // `∃` 之后，单元⑧ 的 `cases hy` 整类打红）。
            // 这条与 `spine::spine_with_notation` 的 binder 分支**必须同款**
            // ——两个函数都声称「记法节点的源像 = target(操作数…)」，不一致就会
            // 一个认得出、另一个代错位。
            if *assoc == crate::ast::NotationAssoc::Binder {
                if let Some(Expr::Lambda { binders, .. }) = rhs.as_deref() {
                    if let Some(ty) = binders.first().and_then(|b| b.ty.as_deref()) {
                        args.push(ty.clone());
                    }
                }
            }
            if let Some(lhs) = lhs {
                args.push(lhs.as_ref().clone());
            }
            if let Some(rhs) = rhs {
                args.push(rhs.as_ref().clone());
            }
            Some((target.clone(), args))
        }
        _ => None,
    }
}

// ---- 应用位置的 lambda binder 类型推断（I6，非依赖）----

/// Rewrite a lambda-headed application whose leading binders lack annotations
/// by inferring those binder types from the argument types (kernel-backed
/// `judge_infer`, design `docs/design/elaborator-let-match.md`). Handles the
/// curried case `(fun x y => …) a b`. Returns `None` when the shape or the
/// query does not apply — the normal path then reports the untyped binder.
fn annotate_application_lambda(expr: &Expr, ctx: &ElabCtx, scope: &ElabScope) -> Option<Expr> {
    // Flatten the spine `f a1 … an` (leftmost non-App is the head).
    let mut args: Vec<&Expr> = Vec::new();
    let mut head = expr;
    while let Expr::App { fun, arg, .. } = head {
        args.push(arg);
        head = fun;
    }
    if args.is_empty() {
        return None;
    }
    args.reverse();
    let Expr::Lambda {
        binders,
        body,
        span,
    } = head
    else {
        return None;
    };
    let untyped: Vec<usize> = binders
        .iter()
        .enumerate()
        .filter(|(_, b)| b.ty.is_none())
        .map(|(i, _)| i)
        .collect();
    if untyped.is_empty() || args.len() < untyped.len() {
        return None;
    }
    let judge = scope.judge_binders();
    let mut new_binders = binders.clone();
    for (n, &i) in untyped.iter().enumerate() {
        let text = judge_infer(ctx.prefix_src, ctx.options, &judge, &render_expr(args[n])).ok()?;
        let ty = crate::proof::parse_expr_text(&text).ok()?;
        new_binders[i].ty = Some(Box::new(ty));
    }
    let mut rebuilt = Expr::Lambda {
        binders: new_binders,
        body: body.clone(),
        span: *span,
    };
    for arg in &args {
        rebuilt = Expr::App {
            fun: Box::new(rebuilt),
            arg: Box::new((*arg).clone()),
            explicit_spine: false,
            span: expr.span(),
        };
    }
    Some(rebuilt)
}

// ---- 模式编译器（docs/design/match-patterns.md §4）----
//
// 把用户写的 `| <pattern> [if <guard>] => body`（可能含通配、绑定、嵌套构造子、
// Nat 字面量）**源到源** canonical 化成「每构造子恰好一条 arm、参数全是绑定」
// 的 `Expr::Match` 树；嵌套匹配生成在该 arm 的 body 里，交回本模块既有的
// lowering 逐层处理。好处：不手搓 de Bruijn，守卫复用 prelude `Bool.rec`。

/// 待匹配的一列（顶层是用户写的 scrutinee；嵌套是字段 binder 名）。
#[derive(Clone)]
struct ColVar<'c, 'a> {
    expr: Expr,
    /// 该列类型的归纳元数据（`None` = 参数/未知 → 只能绑定或通配）。
    ind: Option<&'c InductiveInfo<'a>>,
    ind_name: Option<String>,
    /// 该列类型的参数实例（参数名 → 书写实参），用于把字段类型 `A` 代换成
    /// `Option Nat` 里的 `Nat`（否则嵌套模式看不到内层归纳）。
    subst: HashMap<String, Expr>,
}

/// 编译器的一行：模式串（长度 = 列数）+ body + 守卫。
#[derive(Clone)]
struct PatternRow {
    pats: Vec<Pattern>,
    guard: Option<Expr>,
    body: Expr,
}

/// 模式在某一列的解析结果。
enum Resolved {
    Ctor { ci: usize, args: Vec<Pattern> },
    Bind(String),
    Wild,
}

fn bad_arm(message: &str, span: Span) -> CompileError {
    CompileError::elab(ErrorKind::ElabMatchBadArm, message.to_string(), span)
}

fn ident_expr(name: &str, span: Span) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span,
    }
}

/// 错误消息里列出的构造子拼写：源名与规范名都给（R1 之后源名已不是内核名，
/// 学员按提示写哪一个都能过 —— R3）。
fn ctor_names_text_from(info: &InductiveInfo) -> String {
    info.ctors
        .iter()
        .map(|c| {
            if c.name == c.canonical {
                format!("`{}`", c.name)
            } else {
                format!("`{}`（或 `{}`）", c.canonical, c.name)
            }
        })
        .collect::<Vec<_>>()
        .join("、")
}

/// 构造子下标：源名（`succ`，R3 的源级写法）、规范名（`Nat.succ`，R1）与
/// 该归纳内唯一的裸后缀都命中。
fn ctor_index(info: &InductiveInfo, name: &str) -> Option<usize> {
    if let Some(i) = info.ctors.iter().position(|c| c.name == name) {
        return Some(i);
    }
    if let Some(i) = info.ctors.iter().position(|c| c.canonical == name) {
        return Some(i);
    }
    if name.contains('.') {
        return None;
    }
    let hits: Vec<usize> = info
        .ctors
        .iter()
        .enumerate()
        .filter(|(_, c)| c.canonical.rsplit('.').next() == Some(name))
        .map(|(i, _)| i)
        .collect();
    if hits.len() == 1 {
        Some(hits[0])
    } else {
        None
    }
}

/// Nat 形状：零构造子（0 字段）+ succ 构造子（1 字段，类型回到自身）。
fn nat_shape(info: &InductiveInfo, name: &str) -> Option<(usize, usize)> {
    let zero = info.ctors.iter().position(|c| c.fields.is_empty())?;
    let succ = info.ctors.iter().position(|c| {
        c.fields.len() == 1
            && c.fields[0].src_ty.as_ref().and_then(head_ident).as_deref() == Some(name)
    })?;
    Some((zero, succ))
}

fn resolve_pattern(pat: &Pattern, col: &ColVar) -> Result<Resolved, CompileError> {
    match pat {
        Pattern::Wild { .. } => Ok(Resolved::Wild),
        Pattern::Num { value, span } => {
            let info = col.ind.ok_or_else(|| {
                bad_arm(
                    "数字字面量模式只能用在 Nat 上：这一列不是已知的归纳类型",
                    *span,
                )
            })?;
            let name = col.ind_name.as_deref().unwrap_or("");
            let (zero, succ) = nat_shape(info, name).ok_or_else(|| {
                bad_arm(
                    "数字字面量模式只能用在 Nat 上（0 元零构造子 + 一元 succ 构造子）",
                    *span,
                )
            })?;
            let n: u64 = value.parse().unwrap_or(0);
            if n == 0 {
                Ok(Resolved::Ctor {
                    ci: zero,
                    args: Vec::new(),
                })
            } else {
                Ok(Resolved::Ctor {
                    ci: succ,
                    args: vec![Pattern::Num {
                        value: (n - 1).to_string(),
                        span: *span,
                    }],
                })
            }
        }
        Pattern::Ident { name, args, span } => {
            if let Some(info) = col.ind {
                if let Some(ci) = ctor_index(info, name) {
                    let want = info.ctors[ci].fields.len();
                    if args.len() != want {
                        return Err(bad_arm(
                            &format!(
                                "构造子 `{}` 有 {} 个字段，但这一支写了 {} 个子模式；请写满字段",
                                info.ctors[ci].canonical,
                                want,
                                args.len()
                            ),
                            *span,
                        ));
                    }
                    return Ok(Resolved::Ctor {
                        ci,
                        args: args.clone(),
                    });
                }
            }
            if args.is_empty() {
                return Ok(Resolved::Bind(name.clone()));
            }
            let ty = col.ind_name.clone().unwrap_or_default();
            let available = col.ind.map(ctor_names_text_from).unwrap_or_default();
            Err(bad_arm(
                &format!("`{ty}` 没有构造子 `{name}`；可用的是：{available}"),
                *span,
            ))
        }
    }
}

/// 守卫链：`| p if g1 => b1 | …` 在「模式都已匹配」后按顺序判定，第一个为真
/// 的 body 胜；为假落到下一行；最后一行若仍有守卫 → 没有兜底。
fn guard_chain(rows: &[PatternRow], span: Span) -> Result<Expr, CompileError> {
    let Some(row) = rows.first() else {
        return Err(CompileError::elab(
            ErrorKind::ElabMatchNonExhaustive,
            "`match` 的守卫为假时没有兜底分支：请在后面补一条不加守卫的分支".to_string(),
            span,
        ));
    };
    match &row.guard {
        None => Ok(row.body.clone()),
        Some(g) => {
            let fallback = guard_chain(&rows[1..], span)?;
            let gspan = g.span();
            Ok(Expr::Match {
                scrutinee: Box::new(g.clone()),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Ident {
                            name: "Bool.true".to_string(),
                            args: Vec::new(),
                            span: gspan,
                        },
                        guard: None,
                        body: row.body.clone(),
                        span: gspan,
                    },
                    MatchArm {
                        pattern: Pattern::Ident {
                            name: "Bool.false".to_string(),
                            args: Vec::new(),
                            span: gspan,
                        },
                        guard: None,
                        body: fallback,
                        span: gspan,
                    },
                ],
                span,
            })
        }
    }
}

/// 子模式的列变量：字段的书写类型给出嵌套归纳。
fn field_col<'c, 'a>(
    name: &str,
    src_ty: Option<&Expr>,
    parent_subst: &HashMap<String, Expr>,
    ctx: &'c ElabCtx<'a, '_>,
) -> ColVar<'c, 'a> {
    // 参数化归纳：字段类型里的参数名先代入（`some (a : A)` 在 `Option Nat`
    // 下 → `Nat`），嵌套模式才能解析内层构造子。
    let substituted =
        src_ty.map(|t| super::goals::substitute_names(t, parent_subst, &HashMap::new()));
    let spine = substituted.as_ref().and_then(src_spine);
    let head = spine.as_ref().map(|(h, _)| h.clone());
    let args = spine.map(|(_, a)| a).unwrap_or_default();
    let ind = head.as_deref().and_then(|h| ctx.inductives.get(h));
    let subst = match ind {
        Some(info) => info
            .param_names
            .iter()
            .cloned()
            .zip(args)
            .collect::<HashMap<String, Expr>>(),
        None => HashMap::new(),
    };
    ColVar {
        expr: ident_expr(name, Span::default()),
        ind,
        ind_name: head,
        subst,
    }
}

/// 编译一层的 `match`（`vars` 都已在作用域里），返回一个 body 表达式
/// （可能是生成出来的嵌套 `Expr::Match`，也可能直接就是叶子 body）。
fn compile_pattern_body<'c, 'a>(
    ctx: &'c ElabCtx<'a, '_>,
    vars: &[ColVar<'c, 'a>],
    rows: &[PatternRow],
    span: Span,
) -> Result<Expr, CompileError> {
    if rows.is_empty() {
        return Err(CompileError::elab(
            ErrorKind::ElabMatchNonExhaustive,
            "`match` 的分支不完整：有些取值没有对应分支".to_string(),
            span,
        ));
    }
    let mut resolved: Vec<Vec<Resolved>> = Vec::with_capacity(rows.len());
    let mut all_irrefutable = true;
    for row in rows {
        let mut this = Vec::with_capacity(vars.len());
        for (j, pat) in row.pats.iter().enumerate() {
            let res = resolve_pattern(pat, &vars[j])?;
            if !matches!(res, Resolved::Bind(_) | Resolved::Wild) {
                all_irrefutable = false;
            }
            this.push(res);
        }
        resolved.push(this);
    }
    if all_irrefutable {
        // 这一层所有模式都不可反驳：顺序 + 守卫决定，无需再造 match。
        return guard_chain(rows, span);
    }
    // 选第一处含可反驳模式的列。
    let col = (0..vars.len())
        .find(|&j| {
            resolved
                .iter()
                .any(|r| matches!(r[j], Resolved::Ctor { .. }))
        })
        .expect("at least one refutable pattern exists");
    let info = vars[col]
        .ind
        .ok_or_else(|| bad_arm("无法确定被匹配类型的归纳信息", span))?;
    let mut arms: Vec<MatchArm> = Vec::with_capacity(info.ctors.len());
    for (ci, ctor) in info.ctors.iter().enumerate() {
        let k = ctor.fields.len();
        // 每个字段的嵌套归纳（用于判定某个子模式是不是构造子）。
        let field_inds: Vec<Option<&InductiveInfo<'a>>> = ctor
            .fields
            .iter()
            .map(|f| f.src_ty.as_ref().and_then(head_ident))
            .map(|head| head.and_then(|h| ctx.inductives.get(&h)))
            .collect();
        // 子模式是「绑定」吗（无子模式，且名字不是该字段类型的构造子）。
        let is_bind_arg = |j: usize, name: &str| -> bool {
            !name.contains('.') && field_inds[j].is_none_or(|ind| ctor_index(ind, name).is_none())
        };
        // 1) 字段名：某行在该字段是「绑定」时优先沿用它的名字（canonical 输入
        //    因此保持名字不变 → 编译幂等）；否则用新鲜名。
        let mut field_names: Vec<String> = Vec::with_capacity(k);
        for j in 0..k {
            let mut chosen: Option<String> = None;
            for (ri, _) in rows.iter().enumerate() {
                if let Resolved::Ctor { ci: rci, args } = &resolved[ri][col] {
                    if *rci == ci {
                        if let Pattern::Ident { name, args: a, .. } = &args[j] {
                            if a.is_empty() && is_bind_arg(j, name) && !field_names.contains(name) {
                                chosen = Some(name.clone());
                                break;
                            }
                        }
                    }
                }
            }
            field_names.push(chosen.unwrap_or_else(|| format!("__soko_m{col}_{ci}_{j}")));
        }
        // 2) 逐行特化（去掉 col，换成该构造子的 k 个子模式）。
        let mut sub_rows: Vec<PatternRow> = Vec::new();
        for (ri, row) in rows.iter().enumerate() {
            let (args, bound_column) = match &resolved[ri][col] {
                Resolved::Ctor { ci: rci, args } if *rci == ci => (args.clone(), None),
                Resolved::Ctor { .. } => continue,
                Resolved::Wild => (
                    vec![
                        Pattern::Wild {
                            span: row.pats[col].span(),
                        };
                        k
                    ],
                    None,
                ),
                Resolved::Bind(name) => (
                    vec![
                        Pattern::Wild {
                            span: row.pats[col].span(),
                        };
                        k
                    ],
                    Some(name.clone()),
                ),
            };
            let mut map: HashMap<String, Expr> = HashMap::new();
            for (j, arg) in args.iter().enumerate() {
                if let Pattern::Ident {
                    name, args: sub, ..
                } = arg
                {
                    if sub.is_empty() && is_bind_arg(j, name) && field_names[j] != *name {
                        map.insert(name.clone(), ident_expr(&field_names[j], arg.span()));
                    }
                }
            }
            if let Some(name) = bound_column {
                map.insert(name, vars[col].expr.clone());
            }
            let mut pats = row.pats.clone();
            pats.splice(col..=col, args.iter().cloned());
            let body = super::goals::substitute_names(&row.body, &map, &HashMap::new());
            let guard = row
                .guard
                .as_ref()
                .map(|g| super::goals::substitute_names(g, &map, &HashMap::new()));
            sub_rows.push(PatternRow { pats, guard, body });
        }
        // 3) 新列：去掉 col、在 col 处插入 k 个字段列。
        let mut next_vars: Vec<ColVar<'c, 'a>> = Vec::with_capacity(vars.len() - 1 + k);
        for (j, v) in vars.iter().enumerate() {
            if j != col {
                next_vars.push(v.clone());
            }
        }
        let field_vars: Vec<ColVar<'c, 'a>> = (0..k)
            .map(|j| {
                field_col(
                    &field_names[j],
                    ctor.fields[j].src_ty.as_ref(),
                    &vars[col].subst,
                    ctx,
                )
            })
            .collect();
        next_vars.splice(col..col, field_vars);
        let body = compile_pattern_body(ctx, &next_vars, &sub_rows, span)?;
        arms.push(MatchArm {
            pattern: Pattern::Ident {
                name: ctor.name.clone(),
                args: field_names
                    .iter()
                    .map(|n| Pattern::Ident {
                        name: n.clone(),
                        args: Vec::new(),
                        span,
                    })
                    .collect(),
                span,
            },
            guard: None,
            body,
            span,
        });
    }
    Ok(Expr::Match {
        scrutinee: Box::new(vars[col].expr.clone()),
        arms,
        span,
    })
}

/// The inductive a `match` scrutinee eliminates, plus (when the scrutinee is a
/// local variable with a written source type) the source arguments of that
/// type application — `Option Nat` yields `["Nat"]`. The argument list is
/// `None` when the head came from the `judge_infer` fallback.
fn infer_inductive_with_params(
    ctx: &ElabCtx,
    scope: &ElabScope,
    scrutinee: &Expr,
) -> Option<(String, Option<Vec<Expr>>)> {
    if let Expr::Ident { name, .. } = scrutinee {
        if let Some(ty) = scope.src_ty(name) {
            if let Some((head, args)) = src_spine(ty) {
                return Some((head, Some(args)));
            }
        }
    }
    let binders = scope.judge_binders();
    let text = judge_infer(
        ctx.prefix_src,
        ctx.options,
        &binders,
        &render_expr(scrutinee),
    )
    .ok()?;
    let ty = crate::proof::parse_expr_text(&text).ok()?;
    let head = head_ident(&ty)?;
    Some((head, None))
}

/// Map the kernel-rendered sort of `R` to the recursor's universe level:
/// `Prop`→0, `Type`→1, `Sort n`→n (design §5).
fn sort_text_level(text: &str) -> Option<u64> {
    match text.trim() {
        "Prop" => Some(0),
        "Type" => Some(1),
        other => {
            if let Some(rest) = other.strip_prefix("Sort ") {
                rest.trim().parse::<u64>().ok()
            } else if let Some(rest) = other.strip_prefix("Type ") {
                rest.trim().parse::<u64>().ok().map(|n| n + 1)
            } else {
                None
            }
        }
    }
}

/// The recursor universe level for the expected result type `R`: the sort of
/// `R` as inferred by the kernel (`judge_infer` reuses the 128-entry cache).
fn infer_expected_level(ctx: &ElabCtx, scope: &ElabScope, expected_src: &Expr) -> Option<u64> {
    let term = render_expr(expected_src);
    let binders = scope.judge_binders_for(expected_src);
    let text = if binders.is_empty() {
        // `R` is closed w.r.t. the local context: add one dummy `Prop` binder so
        // `judge_infer`'s `fun … => R` wrapper still has a layer to peel.
        let dummy = vec![GoalBinderSpec {
            name: "_soko_expected_level".to_string(),
            ty: Some("Prop".to_string()),
        }];
        judge_infer(ctx.prefix_src, ctx.options, &dummy, &term).ok()?
    } else {
        judge_infer(ctx.prefix_src, ctx.options, &binders, &term).ok()?
    };
    sort_text_level(&text)
}

fn level_from_u64<'a>(builder: &mut EnvBuilder<'a>, n: u64) -> LevelPtr<'a> {
    let mut level = builder.zero();
    for _ in 0..n {
        level = builder.succ(level);
    }
    level
}

/// **G-58/G-59（0.81.0）**：裸 `Foo.rec`（**没写 `.{u}`**）的消去层级，按
/// **整个应用的期望类型的宇宙**补回来。
///
/// 为什么这是对的：递归子的结果类型就是 `motive major`，而 `motive : Ind → Sort u`
/// ⇒ 期望类型必须与 `motive major` **定义相等** ⇒ `u` 就是期望类型所在的宇宙
/// （`universe_level_text_of_operands` 问的正是这个）。Lean 4 里 `u` 由 elaboration
/// 统一求出来，所以 `And.rec A B (fun _ => Type) …` 与 `MyBox.rec (fun _ => Type) …`
/// 都是普通写法。
///
/// **四条闸门**（缺一条就原样返回 ⇒ 绝不比今天差）：
/// * 头必须是**源级裸 `Ident` 且 `levels: None`** —— 用户写了 `.{u}` 就听用户的 ✗；
/// * 头必须是某个归纳块的**递归子**，且它有消去层级（`rec_universe_arity > 0`）；
/// * 这一层必须**有期望类型**（`#check` 之类没有 ⇒ 保持默认，逐字节不变）；
/// * 推出来的层级必须**不是 0**（0 就是默认值 ⇒ 不用动，逐字节不变）。
fn infer_recursor_universes<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &KnownTable,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    src: &Expr,
    out: ExprPtr<'a>,
    expected_src: Option<&Expr>,
) -> Result<ExprPtr<'a>, CompileError> {
    let Some(expected) = expected_src else {
        return Ok(out);
    };
    // prelude 安装期间**不推断**：那一段的慢路（`judge_infer` 合成 `#check`、整份重编）
    // 看不见正在安装的 L1 名字（`And a b` 还没进环境）⇒ 就地路与慢路会**分叉**
    // （实测被 `judge_inplace` 的 shadow 判据咬住），而且 prelude 自己的 `.rec`
    // 用法全是 `Prop` 层级、不需要这条推断 ✓。
    if prelude_install_active() {
        return Ok(out);
    }
    let (head, _args) = crate::spine::spine_of(src);
    // 只有**裸 `Ident`**（不带 `.{...}`）才补：`Foo.rec.{2}` 是 `UniverseApp`，
    // 用户已经说了算 ⇒ 原样返回。
    let Expr::Ident { name, .. } = head else {
        return Ok(out);
    };
    let Some(info) = ctx.inductives.values().find(|i| i.recursor == *name) else {
        return Ok(out);
    };
    let arity = info.rec_universe_arity;
    if arity == 0 {
        return Ok(out);
    }
    let level_text = {
        let mut env = InplaceEnv {
            builder: &mut *builder,
            known,
        };
        infer_type_text(ctx, scope, expected, Some(&mut env))
    };
    // `u` = **期望类型所在的宇宙**（`R : Sort u`）。与 `match` 的动机层级用的是
    // **同一个**口径（`infer_expected_level` ⇒ `sort_text_level`）。
    let Some(u) = level_text
        .as_deref()
        .and_then(level_text_of_sort)
        .and_then(|t| t.trim().parse::<u64>().ok())
    else {
        return Ok(out);
    };
    if u == 0 {
        return Ok(out);
    }
    // 递归子的宇宙参数表是 `[消去层级, …归纳块自己的宇宙参数]`（内核 `mk_elim_level`），
    // 后面的照旧默认 0（与今天一致）。
    let mut levels: Vec<LevelPtr<'a>> = Vec::with_capacity(arity);
    levels.push(level_from_u64(builder, u));
    for _ in 1..arity {
        levels.push(builder.zero());
    }
    let levels = builder.alloc_levels_slice(&levels);
    Ok(relabel_app_head(builder, out, levels))
}

/// 把一条已 elaborate 的**应用脊**的头常量换成另一个宇宙层级表（G-58/G-59 用）。
/// 头不是 `Const` ⇒ 原样返回。
/// **G-93 真修**：裸常量（**没写 `.{n}`**）的宇宙层，**从签名与实参类型解出来** ✓。
///
/// ## 病根（第 18/19 棒实测定位 ✓）
///
/// `Expr::Ident` 那条路把常量的**每一个**宇宙位都写成 `0` ✗
/// （`elab.rs:4525` 的 `params.iter().map(|_| builder.zero())` —— 全文件**唯一**一处 ✓）
/// ⇒ `Eq.refl α a`（`α : Type`）被当成 `Eq.refl.{0}` ✗
/// ⇒ 内核「**期望 `Sort(0)`，实际是 `Sort(1)`**」✗。
/// 前置库的 `Eq` 一族（`Eq`/`Eq.refl`/`Eq.subst`/`Ne`/`Eq.symm`/`congrArg`/`Quot`… ✓）
/// 全是 `{u}` 多态的 ✓ ⇒ **课程里最常见的证明项**全中招 ✗。
///
/// ## ⚠ 第 20 棒试过「位置式」修法，**被判据否掉** ✗ —— 这条是它的修正版 ✓
///
/// 位置式 = 「取**首个书写实参**的类型 ⇒ 就当那个宇宙位」✗ —— 症状面确实转绿 ✓
/// 但 `courses/set-theory/lib/Order.sokonanoda` 第 474 行 `Acc.intro` 当场
/// `compiled → failed` ✗（**首个实参未必对应宇宙位所在的形参** ✗）。
/// ⇒ **本版加了签名闸门** ✓：**层 0 的域必须恰好是 `Sort <该常量自己的某个宇宙参数>`** ✓
/// —— 签名导向 ✓，不是按位置猜 ✗。`Acc` 是**单态**的（`(α : Type)` ✓，宇宙位 0 个 ✓）
/// ⇒ 连闸门都进不来 ✓。
///
/// ## 为什么从**实参**读、而不是从**期望类型**读（与递归子那条的区别 ✓）
///
/// `u` 是**参数**层（`{α : Sort u}` 里 `α` 的层 ✓），**不是结果层** ✗ ——
/// 递归子那条要的消去层级才是结果层 ✓（`infer_recursor_universes` ✓）。
/// ⚠ **记法那条路早就在这么做** ✓（`universe_level_text_of_operands` ✓，
/// 实测 `: a = a := sorry` 判绿 ✓ 而指向式 `: Eq α a a := sorry` 判红 ✗）
/// ⇒ 这条只是把**同一个口径**铺到普通常量应用上 ✓，不是新发明 ✓。
///
/// ## 安全性质（照 `infer_recursor_universes` ✓）
///
/// * **免费闸门**：头不是**裸 `Ident`**（写了 `.{n}` ⇒ 用户说了算 ✓）⇒ 原样返回 ✓；
/// * **签名闸门** ✓：宇宙位**恰好 1 个** ✗（多宇宙位的对齐是另一件事，见 G-63 ✗）·
///   签名文本拿得到 ✓ · 层 0 的域**恰好**是 `Sort <自己的宇宙参数>` ✓ ——
///   三者缺一 ⇒ 原样返回 ✓（**这是第 20 棒那版缺的那道闸** ✗）；
/// * **算不出 / 算成 0 ⇒ 原样返回** ✓（`Prop` 那档默认值**碰巧是对的** ✓，别去动它 ✗）；
/// * prelude 安装期间**不推断** ✓（同 `infer_recursor_universes` 的理由：那一段的
///   慢路看不见正在安装的名字 ⇒ 就地路与慢路会分叉 ✗）。
///
/// 逃生门：`SOKO_NO_CONST_LEVELS=1`（排查用 ✓，**不是**降级结案 ✗）。
fn infer_const_universes<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &KnownTable,
    ctx: &ElabCtx<'a, '_>,
    scope: &ElabScope<'a>,
    src: &Expr,
    out: ExprPtr<'a>,
) -> Result<ExprPtr<'a>, CompileError> {
    if std::env::var_os("SOKO_NO_CONST_LEVELS").is_some() {
        return Ok(out);
    }
    if prelude_install_active() {
        return Ok(out);
    }
    let (head, args) = crate::spine::spine_of(src);
    // 只有**裸 `Ident`**（不带 `.{...}`）才补 ✓ —— 写了 `.{n}` 是 `UniverseApp`，
    // 用户已经说了算 ⇒ 原样返回 ✓（这条就是"显式写法行为不变"的守卫 ✓）。
    let Expr::Ident { name, .. } = head else {
        return Ok(out);
    };
    let Some(info) = known.get(name.as_str()) else {
        return Ok(out);
    };
    // **签名闸门①**：宇宙位**恰好 1 个** ✓（多位的层对齐见 G-63 ✗，不在这里猜 ✗）。
    if info.universes().len() != 1 {
        return Ok(out);
    }
    let Some(sig) = info.signature() else {
        return Ok(out);
    };
    let Some((layers, _result)) = crate::compile::implicit::telescope(sig) else {
        return Ok(out);
    };
    // **签名闸门②** ✓：层 0 的域必须**恰好**是 `Sort <该常量自己的宇宙参数>` ✓。
    // 这一条就是第 20 棒缺的那道闸 ✗ —— 位置式在 `Acc.intro` 上翻车正是因为
    // 它**不看签名** ✗。签名文本是**源级**的 ✓ ⇒ 里面的 `u` **没被默认掉** ✓
    // （`KnownName::Decl::signature` 的文档原话：源文本写的是 `Eq.{u} α a b` ✓）。
    let Some(layer0) = layers.first() else {
        return Ok(out);
    };
    let dom0 = render_expr(&layer0.domain);
    let Some(param) = dom0.trim().strip_prefix("Sort ").map(str::trim) else {
        return Ok(out);
    };
    if !info.universes().iter().any(|u| u == param) {
        return Ok(out);
    }
    let Some(first) = args.first() else {
        return Ok(out);
    };
    // 首个**书写**实参 ↔ 首个形参 ✓（`Eq α a a` 的 `α` 填的正是隐式 `{α : Sort u}` ✓）。
    let level_text = {
        let mut env = InplaceEnv {
            builder: &mut *builder,
            known,
        };
        infer_type_text(ctx, scope, first, Some(&mut env))
    };
    // 类型文本 ⇒ 宇宙层级 ✓（`Prop`⇒`0` · `Type n`⇒`n+1` · `Sort n`⇒`n` ✓）。
    let Some(level) = level_text.as_deref().and_then(level_text_of_sort) else {
        return Ok(out);
    };
    let Ok(n) = level.trim().parse::<u64>() else {
        return Ok(out);
    };
    // `0` 就是今天的默认值 ✓ —— 那档**碰巧是对的**（`Prop` ✓），别去动它 ✗。
    if n == 0 {
        return Ok(out);
    }
    let levels = [level_from_u64(builder, n)];
    let levels = builder.alloc_levels_slice(&levels);
    Ok(relabel_app_head(builder, out, levels))
}

fn relabel_app_head<'a>(
    builder: &mut EnvBuilder<'a>,
    e: ExprPtr<'a>,
    levels: LevelsPtr<'a>,
) -> ExprPtr<'a> {
    match &*e {
        sokonanoda::expr::Expr::App { fun, arg, .. } => {
            let fun = relabel_app_head(builder, *fun, levels);
            builder.mk_app(fun, *arg)
        }
        sokonanoda::expr::Expr::Const { name, .. } => builder.mk_const(*name, levels),
        _ => e,
    }
}

/// Whether any sub-expression of `e` uses the identifier `name` (mirror of
/// the kernel's own `is_recursive` scan over constructor binder types, which
/// checks binder types for a mention of an inductive name of the block).
fn mentions_ident(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Ident { name: n, .. } => n == name,
        Expr::UniverseApp { name: n, .. } => n == name,
        Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } => false,
        Expr::App { fun, arg, .. } => mentions_ident(fun, name) || mentions_ident(arg, name),
        Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
            binders
                .iter()
                .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
                || mentions_ident(body, name)
        }
        Expr::Arrow {
            domain, codomain, ..
        } => mentions_ident(domain, name) || mentions_ident(codomain, name),
        Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
            elements.iter().any(|element| mentions_ident(element, name))
        }
        Expr::Plus { lhs, rhs, .. } => mentions_ident(lhs, name) || mentions_ident(rhs, name),
        Expr::Let {
            binder, val, body, ..
        } => {
            binder
                .ty
                .as_deref()
                .is_some_and(|ty| mentions_ident(ty, name))
                || mentions_ident(val, name)
                || mentions_ident(body, name)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            mentions_ident(scrutinee, name)
                || arms.iter().any(|arm| {
                    // 模式本身不含表达式（v1）；但守卫与 body 是表达式。
                    arm.guard.as_ref().is_some_and(|g| mentions_ident(g, name))
                        || mentions_ident(&arm.body, name)
                })
        }
        Expr::By { .. } => false, // by 块在 elab 前已被引擎降级为普通表达式
        // 记号节点（G-04 / WO-011）：符号与目标名不是标识符，只走操作数。
        Expr::Notation { lhs, rhs, .. } => {
            lhs.as_deref().is_some_and(|e| mentions_ident(e, name))
                || rhs.as_deref().is_some_and(|e| mentions_ident(e, name))
        }
    }
}

/// Walk the constructor's result as a Pi telescope (every arrow domain is a
/// binder type, the final codomain is not) and report whether any binder type
/// mentions `name` — the kernel scans the elaborated ctor type the same way.
fn result_telescope_mentions(result: &Expr, name: &str) -> bool {
    let mut current = result;
    loop {
        match current {
            Expr::Arrow {
                domain, codomain, ..
            } => {
                if mentions_ident(domain, name) {
                    return true;
                }
                current = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                if binders
                    .iter()
                    .any(|b| b.ty.as_deref().is_some_and(|ty| mentions_ident(ty, name)))
                {
                    return true;
                }
                current = body;
            }
            _ => return false,
        }
    }
}

// ---------------------------------------------------------------------------
// 无显式 rec 的归纳块：recursor 自动派生
//
// 与 py-nat 的手写 rec 同构（内核按同形状重建规则并 def_eq 比对）：
//   rec <Ind>.rec {u} :
//     (motive : (x : Ind) -> Sort u) ->
//     (m<i> : forall (<字段望远镜> <ih…>), motive (<c_i> <字段>…)) …
//     (target : Ind) -> motive target
//   iota <c_i> := fun (motive) => fun (m_0) => … =>
//     fun (<字段望远镜>) => m_i <字段…> [<递归字段后的自调用>]
// ---------------------------------------------------------------------------

/// One constructor's derived view: its (hygiene-renamed) field telescope and,
/// per recursive field in declaration order, the field name plus the binder
/// telescope of the self-call (the Pi domains of the field type).
struct DerivedCtor {
    fields: Vec<Binder>,
    /// `(field name, telescope, field index arguments)` for each recursive field.
    rec_args: Vec<(String, Vec<Binder>, Vec<Expr>)>,
    /// The ctor's result index arguments (`Vec A (Nat.succ n)` → `[Nat.succ n]`).
    ctor_indices: Vec<Expr>,
}

/// All fields of a constructor in declaration order: the explicit binders
/// followed by the domains of the result's arrow chain — the parser puts
/// `ctor base : (b : Bad) -> Bad`'s field in the result, and the kernel
/// counts the whole elaborated Pi telescope (`pi_telescope_size`).
fn ctor_field_binders(ctor: &CtorDecl) -> Vec<Binder> {
    let mut out: Vec<Binder> = ctor.binders.to_vec();
    out.extend(result_chain_binders(&ctor.result));
    out
}

/// The binder telescope of a (possibly arrow-chained) type: Forall binders
/// are collected verbatim, `A -> B` contributes one anonymous binder for `A`.
/// Head + arguments of the *codomain* of a possibly-arrow/forall type — a
/// recursive field's index arguments live under its telescope
/// (`(x : Nat) -> Vec A x` → `Vec A x`).
fn spine_of_codomain(ty: &Expr) -> Option<(String, Vec<Expr>)> {
    let mut cur = ty;
    loop {
        match cur {
            Expr::Arrow { codomain, .. } => cur = codomain,
            Expr::Forall { body, .. } => cur = body,
            _ => return src_spine(cur),
        }
    }
}

fn result_chain_binders(result: &Expr) -> Vec<Binder> {
    chain_binders_after(result, 0)
}

/// The k-th (0-based) Pi binders of a possibly arrow/forall-chained type: the
/// first `skip` are dropped. `A -> B` contributes one anonymous binder for `A`.
fn chain_binders_after(result: &Expr, skip: usize) -> Vec<Binder> {
    let mut out = Vec::new();
    let mut current = result;
    loop {
        match current {
            Expr::Arrow {
                domain, codomain, ..
            } => {
                out.push(Binder {
                    name: String::new(),
                    ty: Some(Box::new(domain.as_ref().clone())),
                    style: BinderKind::Explicit,
                    span: current.span(),
                });
                current = codomain;
            }
            Expr::Forall { binders, body, .. } => {
                out.extend(binders.iter().cloned());
                current = body;
            }
            _ => break,
        }
    }
    out.split_off(skip.min(out.len()))
}

/// A name that no already-chosen binder uses (identifiers may shadow, so the
/// derived telescopes must avoid every name they will reference).
fn fresh_name(base: &str, taken: &mut HashSet<String>) -> String {
    let mut candidate = base.to_string();
    while taken.contains(&candidate) {
        candidate.push('_');
    }
    taken.insert(candidate.clone());
    candidate
}

/// `Prop`/`Sort 0` written as the block's declared sort. The kernel then only
/// Whether this block's recursor is a **K target** (`RecursorData.is_k`) —
/// the front-side mirror of the kernel's `init_k_target`
/// (`crates/kernel/src/inductive.rs:1268-1276`). The kernel asserts
/// `rd.is_k == st.k_target` (`inductive.rs:662`), so a wrong flag makes
/// every derived recursor for such a block get rejected
/// (`recursor declares the wrong k-reduction flag`).
///
/// The kernel's predicate is exactly: the block lives in `Prop` (`is_zero`),
/// it is neither mutual nor nested (exactly one inductive in the block), and
/// `pi_telescope_size(only_ctor.ty) == local_params.len()`. The ctor's kernel
/// type is assembled below as `forall (params ++ fields), result`, so that
/// equality means **the single constructor has no fields of its own** — the
/// result's arrow chain counts as fields too. Mirror it literally:
/// `ctor_field_binders` is the front's side of that same telescope.
///
/// Do **not** approximate this predicate. Two shapes that "look singleton-ish"
/// are *not* K targets and get rejected if flagged: a ctor whose field count
/// merely equals the parameter count (`Both (A B : Prop)` / `mk (a : A)
/// (b : B)`), and — in the other direction — an *indexed* family with a single
/// field-less ctor (`Q : Nat -> Prop` / `q : Q 0`) which **is** a K target.
fn is_k_target(ty: &Expr, constructors: &[CtorDecl]) -> bool {
    let [only_ctor] = constructors else {
        return false;
    };
    is_prop_block_ty(ty) && ctor_field_binders(only_ctor).is_empty()
}

/// allows large elimination when the block is empty or has a single ctor with
/// exclusively Prop-typed fields; a multi-ctor Prop block therefore gets a
/// small-elimination recursor (no universe parameter, motive into `Prop`).
fn is_prop_block_ty(ty: &Expr) -> bool {
    // 带索引归纳的 `ty` 是索引望远镜（`Nat -> … -> Sort`）：先剥到最终 Sort。
    let mut cur = ty;
    loop {
        match cur {
            Expr::Arrow { codomain, .. } => cur = codomain,
            Expr::Forall { body, .. } => cur = body,
            Expr::Sort {
                sort: SortKind::Prop,
                ..
            }
            | Expr::Sort {
                sort: SortKind::Sort(0),
                ..
            } => return true,
            _ => return false,
        }
    }
}

fn e_ident(name: &str, span: Span) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span,
    }
}

fn e_app(fun: Expr, arg: Expr, span: Span) -> Expr {
    Expr::App {
        fun: Box::new(fun),
        arg: Box::new(arg),
        explicit_spine: false,
        span,
    }
}

fn e_forall(binders: Vec<Binder>, body: Expr, span: Span) -> Expr {
    Expr::Forall {
        binders,
        body: Box::new(body),
        span,
    }
}

fn e_lambda(binders: Vec<Binder>, body: Expr, span: Span) -> Expr {
    Expr::Lambda {
        binders,
        body: Box::new(body),
        span,
    }
}

fn e_universe_app(name: &str, levels: &[String], span: Span) -> Expr {
    Expr::UniverseApp {
        name: name.to_string(),
        levels: levels.to_vec(),
        span,
    }
}

/// The front's mirror of the kernel's `large_elim_test`
/// (`crates/kernel/src/inductive.rs:1201-1223`): does this block's recursor
/// carry an extra universe parameter?
///
/// The kernel *computes* that answer and then asserts the front's derived
/// recursor agrees (`assert_nonnested_recursors_def_eq` → `subst_expr_levels`
/// compares `rec_uparams`). A source-level approximation ("is the field type
/// spelled `Prop`?") therefore becomes a hard rejection wherever the two
/// disagree — the G-03 bug: `inductive Bar (A : Type) : Prop` +
/// `ctor mk (a : A) : Bar A` got a `Sort u` motive from the front while the
/// kernel wanted `Prop`.
///
/// Mirror the kernel literally:
///
/// * a block whose result sort is not `Prop` (`is_nonzero`) eliminates large;
/// * an **empty** Prop block (`[] => true`) eliminates large;
/// * a Prop block with **more than one** constructor does not (`_ => false`);
/// * a single-constructor Prop block asks [`large_elim_test_aux_mirror`].
///
/// Only the last case can disagree with the rule this replaced
/// (`is_prop_block_ty(ty) && constructors.len() > 1`), so the semantic work is
/// confined to it.
fn large_elim_test_mirror<'a>(
    ctx: &ElabCtx,
    builder: &mut EnvBuilder<'a>,
    params: &[Binder],
    name: &str,
    constructors: &[CtorDecl],
    kernel_ctor_tys: &[ExprPtr<'a>],
    block_is_prop: bool,
) -> bool {
    if !block_is_prop {
        // `is_nonzero`: the block lives in `Type <n>` and eliminates large.
        return true;
    }
    debug_assert_eq!(constructors.len(), kernel_ctor_tys.len());
    match (kernel_ctor_tys, constructors) {
        // An empty Prop block eliminates large (`[] => true`).
        ([], _) => true,
        // Exactly one constructor: the kernel's `large_elim_test_aux`.
        ([only], [ctor]) => large_elim_test_aux_mirror(ctx, builder, params, name, ctor, only),
        // More than one constructor: no large elimination.
        _ => false,
    }
}

/// The front's mirror of the kernel's `large_elim_test_aux`
/// (`crates/kernel/src/inductive.rs:1164-1199`) for one constructor.
///
/// The kernel walks the constructor's Pi telescope, skips the first
/// `num_params` binders, and records the de Bruijn *level* of every remaining
/// domain whose sort is not `Prop` (`is_prop_type`). It then asks whether each
/// of those fields, taken as a variable, occurs among the arguments of the
/// constructor's result type (`ind params ++ indices`). A non-`Prop` field that
/// is not one of the inductive's own arguments means the block only eliminates
/// into `Prop`.
///
/// Two kernel facts this mirror must not "improve" on:
///
/// * the subset test is **syntactic** (`unfold_apps` + pointer equality), so a
///   field must *be* the result's own argument: `PA A (ident A a)` does not
///   count as `a` even though `ident A a` reduces to it;
/// * "is this domain a `Prop`" is the kernel's `is_prop_type`, i.e. the sort of
///   the domain is `Sort 0`. Impredicativity is included, so `P -> Q` and
///   `forall (x : Nat), P` are `Prop`-typed (P10/P11) while `A` is not (P1).
fn large_elim_test_aux_mirror<'a>(
    ctx: &ElabCtx,
    builder: &mut EnvBuilder<'a>,
    params: &[Binder],
    name: &str,
    ctor: &CtorDecl,
    ctor_ty: &ExprPtr<'a>,
) -> bool {
    let (domains, result) = peel_pi_telescope(*ctor_ty);
    let num_params = params.len();
    let depth = u16::try_from(domains.len()).expect("constructor telescope exceeds u16");
    // 源级字段（显式 binder ++ 结果箭头链）与内核的 Pi 望远镜逐位同序。
    let src_fields = ctor_field_binders(ctor);
    debug_assert_eq!(domains.len(), num_params + src_fields.len());
    // 内核的 `is_prop_type` 需要「参数 + 前序字段」这个 binder 语境。
    let mut scope = ElabScope::new();
    for (binder, domain) in params.iter().zip(domains.iter()) {
        scope.push(
            binder.name.clone(),
            *domain,
            binder.ty.as_deref().cloned(),
            binder.span,
        );
    }
    let mut non_prop: Vec<ExprPtr<'a>> = Vec::new();
    for (offset, (src, domain)) in src_fields
        .iter()
        .zip(domains[num_params..].iter())
        .enumerate()
    {
        let level = u16::try_from(num_params + offset).expect("telescope level exceeds u16");
        if !field_type_is_prop(ctx, &scope, name, src) {
            non_prop.push(builder.mk_var(depth - 1 - level));
        }
        scope.push(
            src.name.clone(),
            *domain,
            src.ty.as_deref().cloned(),
            src.span,
        );
    }
    let (_, args) = unfold_apps(result);
    non_prop.iter().all(|field| args.contains(field))
}

/// The kernel's `is_prop_type` for one constructor field: does the field's own
/// type live in `Sort 0`?
///
/// * A reference to the block's own inductive is a proposition: the caller only
///   reaches this for a `Prop` block, and the inductive is not yet in the
///   `judge_infer` prefix (it is being defined right now), so this case cannot
///   go to the kernel. A recursive field such as `h : Bar A` is a proof, not
///   data, and must not be mistaken for one.
/// * Everything else is semantic — a `Prop` parameter, `P -> Q`, a `forall`
///   ending in a proposition (impredicativity: `imax(_, 0) == 0`), a **named**
///   `Prop` definition — and is asked of the kernel. A syntactic "does the
///   source say `Prop`" test gets P10/P11/P13/P14 wrong and would trade this
///   assertion for another (`left:0/right:1`).
fn field_type_is_prop(ctx: &ElabCtx, scope: &ElabScope, ind_name: &str, field: &Binder) -> bool {
    let Some(src_ty) = field.ty.as_deref() else {
        // 无类型标注的字段：elaborate 阶段已报 `elab-untyped-binder`，这里按
        // 非 Prop 保守处理，不吞掉内核本该给出的诊断。
        return false;
    };
    if type_is_prop_by_source(src_ty, ind_name) {
        return true;
    }
    field_sort_via_kernel(ctx, scope, src_ty)
}

/// **G-56（0.81.0）**：源码层的**充分**判据 —— 这个字段类型一定是 `Prop` 吗？
///
/// 为什么要单独判一档：`field_sort_via_kernel` 靠合成一份 `#check` 问内核，
/// 而那份合成源**看不见正在声明的块自己**（块还没进环境）⇒ 字段类型里一旦提到
/// 本块（`h : ∀ (y : α), r y x → Acc α r y`），查询**解不出来**、保守答 false ✗
/// ⇒ 前端派生的 `Acc.rec` 宇宙参数个数与内核算出的不一致，撞
/// `assert_nonnested_recursors_def_eq`（实测：期望 `motive … Sort(0)`、
/// 实际 `motive … Sort(u)`）。这是带索引归纳（`Acc`）的**第三道门**。
///
/// 两条规则都只覆盖**内核一定会答 Prop** 的形状：
/// * **蕴涵/全称链的尾件**：`∀ x, … → B` 的宇宙是 `imax(_, level(B))` ⇒
///   尾件是命题就一定是命题（impredicativity，`imax(_, 0) = 0`）；
/// * 尾件是 `Prop` 本身、或**本块自己的名字**（块是 Prop 块 —— 这是调用方
///   `large_elim_test_mirror` 的前置条件）⇒ `Acc … : Prop` ✓。
///
/// 其余一切照旧交给内核（`A`、`P -> Q`、具名 `Prop` 定义 …）—— 这里**不做**
/// 通用宇宙推断，只补「本块自己」这一档 ✗。
fn type_is_prop_by_source(src_ty: &Expr, ind_name: &str) -> bool {
    match src_ty {
        Expr::Arrow { codomain, .. } => type_is_prop_by_source(codomain, ind_name),
        Expr::Forall { body, .. } => type_is_prop_by_source(body, ind_name),
        Expr::Ident { name, .. } => name == "Prop",
        // 尾件是本块自己的名字（`Acc α r y`）也算 —— `head_ident` 判的是**应用头**。
        other => head_ident(other).as_deref() == Some(ind_name),
    }
}

/// Ask the kernel for the sort of `src_ty` and report whether it is `Prop`.
///
/// `judge_infer` synthesizes `#check fun <binders> => <src_ty>` and compiles it
/// with the real kernel, so this is a kernel verdict rather than a text test —
/// the same oracle `match` uses for its motive level
/// ([`infer_expected_level`]). It correctly answers `Prop` for a `Prop`
/// parameter, for `P -> Q`, and for a **named** `Prop` definition
/// (`def Named : Prop := …`), which a syntactic "does it say `Prop`" check
/// would get wrong and thereby trade this assertion for another.
///
/// A failed query yields `false` (non-`Prop`), keeping the kernel's own
/// diagnostic for the block instead of masking it.
fn field_sort_via_kernel(ctx: &ElabCtx, scope: &ElabScope, src_ty: &Expr) -> bool {
    let term = render_expr(src_ty);
    let mut binders = scope.judge_binders_for(src_ty);
    if binders.is_empty() {
        // 与 `infer_expected_level` 同法：`judge_infer` 要剥掉一层 binder 才能
        // 把答案读成「term 的类型」。
        binders.push(GoalBinderSpec {
            name: "_soko_field_sort".to_string(),
            ty: Some("Prop".to_string()),
        });
    }
    let Ok(text) = judge_infer(ctx.prefix_src, ctx.options, &binders, &term) else {
        return false;
    };
    sort_text_level(&text) == Some(0)
}

/// Peel a Pi telescope into `(domains, body)`, in declaration order.
fn peel_pi_telescope<'a>(mut ty: ExprPtr<'a>) -> (Vec<ExprPtr<'a>>, ExprPtr<'a>) {
    let mut domains = Vec::new();
    loop {
        match &*ty {
            sokonanoda::expr::Expr::Pi {
                binder_type, body, ..
            } => {
                domains.push(*binder_type);
                ty = *body;
            }
            _ => return (domains, ty),
        }
    }
}

/// `f a₀ … aₙ` → `(f, [a₀, …, aₙ])` over elaborated expressions. The kernel's
/// subset test uses its own `TcCtx::unfold_apps`; expressions are hash-consed in
/// the arena, so pointer equality is structural equality here just as there.
fn unfold_apps<'a>(mut e: ExprPtr<'a>) -> (ExprPtr<'a>, Vec<ExprPtr<'a>>) {
    let mut args = Vec::new();
    while let sokonanoda::expr::Expr::App { fun, arg, .. } = *e {
        args.push(arg);
        e = fun;
    }
    args.reverse();
    (e, args)
}

/// Synthesize the recursor declaration and one iota rule per constructor for
/// a block written without `rec`. Every binder name is picked fresh against
/// the names the synthesized terms must reference (inductive, constructors,
/// source fields), so no derived binder can shadow a reference.
///
/// `large_elim` is the **kernel's own** large-elimination verdict for this block
/// ([`large_elim_test_mirror`], a literal mirror of
/// `kernel/src/inductive.rs::large_elim_test`). The kernel asserts that the
/// derived recursor carries exactly the universe parameters it computed
/// (`assert_nonnested_recursors_def_eq` → `subst_expr_levels`), so this flag —
/// not a source-level approximation — decides the recursor's shape.
fn derive_recursor(
    name: &str,
    params: &[Binder],
    ty: &Expr,
    constructors: &[CtorDecl],
    ctor_canonical: &[String],
    large_elim: bool,
) -> (RecDecl, Vec<IotaRule>) {
    let ty_span = ty.span();
    let small_elim = !large_elim;
    let universe: Vec<String> = if small_elim {
        Vec::new()
    } else {
        vec!["u".to_string()]
    };
    let motive_sort = |span: Span| {
        if small_elim {
            Expr::Sort {
                sort: SortKind::Prop,
                span,
            }
        } else {
            Expr::Sort {
                sort: SortKind::Level("u".to_string()),
                span,
            }
        }
    };
    let mut taken: HashSet<String> = HashSet::new();
    taken.insert(name.to_string());
    for ctor in constructors {
        taken.insert(ctor.name.clone());
    }
    // 参数名纳入卫生集合：派生的字段/motive/minor 名不得遮蔽参数引用。
    for param in params {
        if !param.name.is_empty() {
            taken.insert(param.name.clone());
        }
    }
    for ctor in constructors {
        for field in ctor_field_binders(ctor) {
            if !field.name.is_empty() {
                taken.insert(field.name);
            }
        }
    }
    // 索引望远镜（`ty` 在 params 之外）：给每个索引一个新鲜名字，供 motive/
    // recursor 引用（内核的 motive = `forall indices, Ind params indices -> Sort`）。
    let index_binders: Vec<Binder> = result_chain_binders(ty)
        .into_iter()
        .map(|b| {
            let base = if b.name.is_empty() { "i" } else { &b.name };
            Binder {
                name: fresh_name(base, &mut taken),
                ty: b.ty,
                style: b.style,
                span: b.span,
            }
        })
        .collect();
    let index_names: Vec<String> = index_binders.iter().map(|b| b.name.clone()).collect();

    // `Ind p1 … pn i1 … ik`（无参数/索引时就是裸 `Ind`）。
    let ind_applied = |span: Span| {
        let mut e = params.iter().fold(e_ident(name, span), |acc, p| {
            e_app(acc, e_ident(&p.name, span), span)
        });
        for index in &index_names {
            e = e_app(e, e_ident(index, span), span);
        }
        e
    };

    let motive = fresh_name("motive", &mut taken);
    let minors: Vec<String> = (0..constructors.len())
        .map(|i| fresh_name(&format!("m{i}"), &mut taken))
        .collect();
    let target = fresh_name("target", &mut taken);

    let motive_x = fresh_name("x", &mut taken);
    // motive : forall (indices…), (x : Ind params indices) -> Sort
    let motive_ty = e_forall(
        index_binders.clone(),
        e_forall(
            vec![Binder {
                name: motive_x,
                ty: Some(Box::new(ind_applied(ty_span))),
                style: BinderKind::Explicit,
                span: ty_span,
            }],
            motive_sort(ty_span),
            ty_span,
        ),
        ty_span,
    );

    // 派生字段名（避让参数/motive 等），并把 ctor 字段名 → 派生名的替换同时作用
    // 到字段类型与构造子结果的索引实参上（索引可能引用字段，如 `Vec A n`）。
    let derived: Vec<DerivedCtor> = constructors
        .iter()
        .map(|ctor| {
            let mut rename: HashMap<String, Expr> = HashMap::new();
            let mut fields = Vec::new();
            for binder in ctor_field_binders(ctor) {
                let base = if binder.name.is_empty() {
                    "x"
                } else {
                    &binder.name
                };
                let field_name = fresh_name(base, &mut taken);
                if !binder.name.is_empty() && binder.name != field_name {
                    rename.insert(binder.name.clone(), e_ident(&field_name, binder.span));
                }
                let ty = binder.ty.map(|t| {
                    Box::new(super::goals::substitute_names(&t, &rename, &HashMap::new()))
                });
                fields.push(Binder {
                    name: field_name,
                    ty,
                    style: binder.style,
                    span: binder.span,
                });
            }
            // A ctor's result may be written in the arrow chain
            // (`ctor ps (n : Nat) : P n -> P (Nat.succ n)`); the indices live in
            // the *codomain*, so read them through `spine_of_codomain`. Reading
            // `ctor.result` with `src_spine` returned `None` for `Arrow`/`Forall`
            // and silently dropped the minor's index arguments, which the kernel
            // then rejected (docs/design/agent-query-channel.md H6-C / TODO A).
            let ctor_indices: Vec<Expr> = spine_of_codomain(&ctor.result)
                .filter(|(head, _)| head.as_str() == name)
                .map(|(_, args)| {
                    args.into_iter()
                        .skip(params.len())
                        .map(|a| super::goals::substitute_names(&a, &rename, &HashMap::new()))
                        .collect()
                })
                .unwrap_or_default();
            let rec_args = fields
                .iter()
                .filter(|field| {
                    field
                        .ty
                        .as_deref()
                        .is_some_and(|ty| mentions_ident(ty, name))
                })
                .map(|field| {
                    let field_ty = field.ty.as_deref().expect("field has a type");
                    let raw = result_chain_binders(field_ty);
                    let mut telescope = Vec::with_capacity(raw.len());
                    for binder in raw {
                        let base = if binder.name.is_empty() {
                            "x"
                        } else {
                            &binder.name
                        };
                        let binder_name = fresh_name(base, &mut taken);
                        telescope.push(Binder {
                            name: binder_name,
                            ty: binder.ty,
                            style: binder.style,
                            span: binder.span,
                        });
                    }
                    let index_args: Vec<Expr> = spine_of_codomain(field_ty)
                        .map(|(_, args)| args.into_iter().skip(params.len()).collect())
                        .unwrap_or_default();
                    (field.name.clone(), telescope, index_args)
                })
                .collect();
            DerivedCtor {
                fields,
                rec_args,
                ctor_indices,
            }
        })
        .collect();

    // 每个构造子的 minor 前提：forall (字段… ih…), motive <ctor 索引实参> (C 字段…)。
    let minor_types: Vec<Expr> = constructors
        .iter()
        .enumerate()
        .zip(&derived)
        .map(|((i, ctor), d)| {
            let mut binders = d.fields.clone();
            for (field_name, telescope, field_indices) in &d.rec_args {
                let field_app = telescope
                    .iter()
                    .fold(e_ident(field_name, ctor.span), |acc, binder| {
                        e_app(acc, e_ident(&binder.name, ctor.span), ctor.span)
                    });
                // IH : motive <field 索引实参> <field 应用>
                let mut ih_body = e_ident(&motive, ctor.span);
                for index in field_indices {
                    ih_body = e_app(ih_body, index.clone(), ctor.span);
                }
                ih_body = e_app(ih_body, field_app, ctor.span);
                let ih_ty = if telescope.is_empty() {
                    ih_body
                } else {
                    e_forall(telescope.clone(), ih_body, ctor.span)
                };
                let ih = fresh_name("ih", &mut taken);
                binders.push(Binder {
                    name: ih,
                    ty: Some(Box::new(ih_ty)),
                    style: BinderKind::Explicit,
                    span: ctor.span,
                });
            }
            // R1：minor 里生成的 `C params fields` 项必须用**规范名**——
            // 内核按名字重建比对 recursor 的 minor 与 iota 规则。
            let canonical = &ctor_canonical[i];
            let mut c_app = params.iter().fold(e_ident(canonical, ctor.span), |acc, p| {
                e_app(acc, e_ident(&p.name, ctor.span), ctor.span)
            });
            c_app = d.fields.iter().fold(c_app, |acc, field| {
                e_app(acc, e_ident(&field.name, ctor.span), ctor.span)
            });
            let mut body = e_ident(&motive, ctor.span);
            for index in &d.ctor_indices {
                body = e_app(body, index.clone(), ctor.span);
            }
            let body = e_app(body, c_app, ctor.span);
            e_forall(binders, body, ctor.span)
        })
        .collect();

    // 递归子望远镜：params → motive → minors → 索引 → 目标
    // （内核 `major_idx = params + motives + minors + indices`）。
    let mut rec_binders: Vec<Binder> = params.to_vec();
    rec_binders.push(Binder {
        name: motive.clone(),
        ty: Some(Box::new(motive_ty.clone())),
        style: BinderKind::Explicit,
        span: ty_span,
    });
    for ((ctor, minor_name), minor_ty) in constructors.iter().zip(&minors).zip(&minor_types) {
        rec_binders.push(Binder {
            name: minor_name.clone(),
            ty: Some(Box::new(minor_ty.clone())),
            style: BinderKind::Explicit,
            span: ctor.span,
        });
    }
    rec_binders.extend(index_binders.clone());
    rec_binders.push(Binder {
        name: target.clone(),
        ty: Some(Box::new(ind_applied(ty_span))),
        style: BinderKind::Explicit,
        span: ty_span,
    });
    let mut rec_body = e_ident(&motive, ty_span);
    for index in &index_names {
        rec_body = e_app(rec_body, e_ident(index, ty_span), ty_span);
    }
    rec_body = e_app(rec_body, e_ident(&target, ty_span), ty_span);
    let rec_ty = e_forall(rec_binders, rec_body, ty_span);
    let rec = RecDecl {
        name: format!("{name}.rec"),
        universe: universe.clone(),
        ty: rec_ty,
        span: constructors.last().map(|ctor| ctor.span).unwrap_or(ty_span),
    };

    // 每构造子一条规则：telescope = (params, motive, 全部 minors, 本构造子字段)，
    // 返回 m_i <字段…>，递归字段后面追加自调用（携带该字段的索引实参）。
    let rules = constructors
        .iter()
        .enumerate()
        .map(|(i, ctor)| {
            let d = &derived[i];
            let mut binders =
                Vec::with_capacity(params.len() + constructors.len() + d.fields.len() + 1);
            binders.extend(params.iter().cloned());
            binders.push(Binder {
                name: motive.clone(),
                ty: Some(Box::new(motive_ty.clone())),
                style: BinderKind::Explicit,
                span: ty_span,
            });
            for (minor_name, minor_ty) in minors.iter().zip(&minor_types) {
                binders.push(Binder {
                    name: minor_name.clone(),
                    ty: Some(Box::new(minor_ty.clone())),
                    style: BinderKind::Explicit,
                    span: ty_span,
                });
            }
            binders.extend(d.fields.iter().cloned());
            let mut body = e_ident(&minors[i], ctor.span);
            for field in &d.fields {
                body = e_app(body, e_ident(&field.name, ctor.span), ctor.span);
            }
            for (field_name, telescope, field_indices) in &d.rec_args {
                let mut call = e_universe_app(&format!("{name}.rec"), &universe, ctor.span);
                for param in params {
                    call = e_app(call, e_ident(&param.name, ctor.span), ctor.span);
                }
                call = e_app(call, e_ident(&motive, ctor.span), ctor.span);
                for minor_name in &minors {
                    call = e_app(call, e_ident(minor_name, ctor.span), ctor.span);
                }
                for index in field_indices {
                    call = e_app(call, index.clone(), ctor.span);
                }
                let field_app = telescope
                    .iter()
                    .fold(e_ident(field_name, ctor.span), |acc, binder| {
                        e_app(acc, e_ident(&binder.name, ctor.span), ctor.span)
                    });
                call = e_app(call, field_app, ctor.span);
                let self_call = if telescope.is_empty() {
                    call
                } else {
                    e_lambda(telescope.clone(), call, ctor.span)
                };
                body = e_app(body, self_call, ctor.span);
            }
            IotaRule {
                // 内核断言 `rule.ctor_name == ctor.name`（inductive.rs:1590），
                // 而 ctor.name 已是安装名（规范名）——这里必须同步。
                ctor_name: ctor_canonical[i].clone(),
                val: e_lambda(binders, body, ctor.span),
                span: ctor.span,
            }
        })
        .collect();

    (rec, rules)
}
