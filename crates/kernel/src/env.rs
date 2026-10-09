use crate::util::{ExprPtr, FxIndexMap, LevelsPtr, NamePtr};
use std::collections::HashSet;
use std::sync::Arc;
use serde::Deserialize;

/// Reducibility hints accompany definitions; used to determine how
/// to unfold expressions in order to most efficiently proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, serde::Serialize)]
pub enum ReducibilityHint {
    #[serde(rename = "opaque")]
    Opaque,
    #[serde(rename = "regular")]
    Regular(u16),
    #[serde(rename = "abbrev")]
    Abbrev,
}

impl ReducibilityHint {
    /// Check whether `self` is "less than" `other` in terms of reducibility; during
    /// delta reduction in equality checking, we want to unfold the greater of the two
    /// definitions to try and bring the two closer.
    pub(crate) fn is_lt(&self, other: &Self) -> bool {
        use ReducibilityHint::*;
        match (self, other) {
            (_, Opaque) => false,
            (Abbrev, _) => false,
            (Opaque, _) => true,
            (_, Abbrev) => true,
            (Regular(h1), Regular(h2)) => h1 < h2,
        }
    }
}

/// Convenience declaration for the elements common across all kinds
/// of declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclarInfo<'a> {
    pub name: NamePtr<'a>,
    pub uparams: LevelsPtr<'a>,
    pub ty: ExprPtr<'a>,
}

/// Computation rules for iota-reduction (pattern matching).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecRule<'a> {
    pub ctor_name: NamePtr<'a>,
    /// the constructor's telescope size minus the params (but including indices).
    /// So a constructor with 2 params, 1 index, and 4 args is (1 + 4) = 5.
    pub ctor_telescope_size_wo_params: u16,
    pub val: ExprPtr<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declar<'a> {
    Axiom { info: DeclarInfo<'a> },
    Quot { info: DeclarInfo<'a> },
    Theorem { info: DeclarInfo<'a>, val: ExprPtr<'a> },
    Definition { info: DeclarInfo<'a>, val: ExprPtr<'a>, hint: ReducibilityHint },
    Opaque { info: DeclarInfo<'a>, val: ExprPtr<'a> },
    Inductive(InductiveData<'a>),
    Constructor(ConstructorData<'a>),
    Recursor(RecursorData<'a>),
}

/// This structure is what's taken from the export file; it contains enough
/// information to begin the process of checking an inductive declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InductiveData<'a> {
    pub(crate) info: DeclarInfo<'a>,
    /// `true` when recursive (that is, the inductive type appears as an argument in a constructor).
    pub(crate) is_recursive: bool,
    /// `true` when this typs is a nested inductive
    #[allow(dead_code)]
    pub(crate) is_nested: bool,
    /// All inductive types in a mutual block must have the same parameters, though this
    /// does not exactly hold for nested inductives.
    pub(crate) num_params: u16,
    pub(crate) num_indices: u16,
    /// The names of this type, and any other inductive types in a `mutual..end`
    /// block. No nested inductive info is conveyed here.
    pub(crate) all_ind_names: Arc<[NamePtr<'a>]>,
    /// The constructor names for THIS type only. No constructors
    /// from other elements in a mutual block, nothing from any nested
    /// construction.
    pub(crate) all_ctor_names: Arc<[NamePtr<'a>]>,
}

impl<'a> InductiveData<'a> {
    pub fn aux_data_ck(&self, other: &Self) -> bool {
        self.info.name == other.info.name
            && self.num_params == other.num_params
            && self.num_indices == other.num_indices
            && self.is_nested == other.is_nested
            && self.all_ctor_names.iter().collect::<HashSet<_>>()
                == other.all_ctor_names.iter().collect::<HashSet<_>>()
            && if other.is_nested {
                self.all_ind_names
                    .iter()
                    .collect::<HashSet<_>>()
                    .is_subset(&other.all_ind_names.iter().collect::<HashSet<_>>())
            } else {
                self.all_ind_names.iter().collect::<HashSet<_>>() == other.all_ind_names.iter().collect::<HashSet<_>>()
            }
    }
}

/// `inductive_name` is the name of the type this constructs. e.g. `Prod` for `Prod.mk`
///
/// `ctor_idx` is 0-based; e.g. `List.nil (ctor_idx := 0)`, `List.cons (ctor_idx := 1)`
///
/// num_params is the number of parameters in the inductive specification, not including ctor args;
/// num_fields is the number of ctor args, not including parameters.
///
/// `Prod.mk (A B) (a b) ;;
/// (num_params := 2) (num_fields := 2)`
///
/// `HAppend.mk {α : Type u} → {β : Type v} → {γ : outParam (Type w)} → (α → β → γ) → HAppend α β γ
/// (num_params := 3) (num_fields := 1)`
///
/// `Syntax.node (num_params := 0) (num_fields := 3)`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorData<'a> {
    pub info: DeclarInfo<'a>,
    pub inductive_name: NamePtr<'a>,
    pub ctor_idx: u16,
    /// The number of parameters, not including ctor args
    pub num_params: u16,
    /// The number of ctor args, not including parameters.
    pub num_fields: u16,
}

impl<'a> ConstructorData<'a> {
    pub fn aux_data_ck(&self, other: &Self) -> bool {
        self.info.name == other.info.name
            && self.inductive_name == other.inductive_name
            && self.ctor_idx == other.ctor_idx
            && self.num_params == other.num_params
            && self.num_fields == other.num_fields
    }
}

/// Information received from the export file regarding a recursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursorData<'a> {
    pub info: DeclarInfo<'a>,
    pub all_inductives: Arc<[NamePtr<'a>]>,
    pub num_params: u16,
    pub num_indices: u16,
    pub num_motives: u16,
    pub num_minors: u16,
    pub rec_rules: Arc<[RecRule<'a>]>,
    pub is_k: bool,
}

impl<'a> RecursorData<'a> {
    /// Compute the index in the recursor's type (in the telescope) where the major premise is located. 
    pub fn major_idx(&self) -> usize {
        (self.num_params + self.num_motives + self.num_minors + self.num_indices) as usize
    }

    pub fn aux_data_ck(&self, other: &Self) -> bool {
        self.info.name == other.info.name
            && self.num_params == other.num_params
            && self.num_indices == other.num_indices
            && self.num_motives == other.num_motives
            && self.num_minors == other.num_minors
            && self.is_k == other.is_k
            && self.all_inductives.iter().collect::<HashSet<_>>()
                == other.all_inductives.iter().collect::<HashSet<_>>()
    }
}

impl<'a> Declar<'a> {
    pub fn info(&self) -> &DeclarInfo<'a> {
        use Declar::*;
        match self {
            Axiom { info, .. }
            | Quot { info, .. }
            | Theorem { info, .. }
            | Definition { info, .. }
            | Inductive(InductiveData { info, .. })
            | Constructor(ConstructorData { info, .. })
            | Recursor(RecursorData { info, .. })
            | Opaque { info, .. } => info,
        }
    }

    /// 声明的**值**（`:=` 之后那个东西）——只有 `Definition`/`Opaque` 有。
    ///
    /// **为什么加它**（计划 T-D52，用户第 8 条反馈）：声明栏只显示类型时，
    /// `Set.mem` 的 `forall (α : Type 0), α -> Set α -> Prop` **看不出它的本质**；
    /// 用户要的是 `:= fun (α : Type 0) (a : α) (A : Set α) => A a` 那一行。
    /// 值一直在内核手里（`Declar::Definition { info, val, hint }`），只是
    /// `info()` 没暴露 ⇒ 补一个**纯读访问器**。
    ///
    /// **判定红线**：这是只读访问器，不改变任何判定路径。
    pub fn value(&self) -> Option<ExprPtr<'a>> {
        match self {
            Declar::Definition { val, .. } | Declar::Opaque { val, .. } => Some(*val),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notation<'a> {
    Prefix { name: NamePtr<'a>, priority: usize, oper: Arc<str> },
    Infix { name: NamePtr<'a>, priority: usize, oper: Arc<str> },
    Postfix { name: NamePtr<'a>, priority: usize, oper: Arc<str> },
}

impl<'a> Notation<'a> {
    pub fn new_prefix(name: NamePtr<'a>, priority: usize, oper: Arc<str>) -> Self {
        Notation::Prefix { name, priority, oper }
    }

    pub fn new_infix(name: NamePtr<'a>, priority: usize, oper: Arc<str>) -> Self {
        Notation::Infix { name, priority, oper }
    }

    pub fn new_postfix(name: NamePtr<'a>, priority: usize, oper: Arc<str>) -> Self {
        Notation::Postfix { name, priority, oper }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EnvLimit<'a> {
    Empty,
    ByIndex(usize),
    ByName(NamePtr<'a>),
    PpUnlimited
}

/// A Lean environment, which consists of a set of declarations that my have a temporary
/// extension, and some notation items. The temporary extensions are used to acommodate
/// the specialization process needed for checking nested inductives.
///
/// When a tyep checker looks up a declaration in the environment, it will check the temporary
/// extension first if there is one, then fall back to the persistent map.
pub struct Env<'x, 'a: 'x> {
    /// ⚠ **借用生命周期是 `'x`（不是 `'a`）** —— 2026-09-29 放宽，为
    /// `EnvBuilder::with_env_scope` 让路：那里两张表是**局部 `mem::take` 出来的**
    /// ⇒ 只能借 `'x`（短），而表**内部**的指针仍是 `'a`（长）✓。
    /// 这是**纯泛化**（原来 `'a` 能过的，现在 `'x` 都能过）⇒ 既有调用点零变化 ✓。
    declars: &'x DeclarMap<'a>,
    /// Used for checking nested inductives.
    ///
    /// ⚠ **这是"临时扩展"，不是环境主表** ⇒ 保持**平坦** `FxIndexMap`：
    /// 它的生命周期只有一次归纳块检查、不参与命令级快照（T2-A 只动主表 ✓）。
    temp_declars: Option<&'x FxIndexMap<NamePtr<'a>, Declar<'a>>>,
    #[allow(dead_code)]
    pub(crate) notation: &'x NotationMap<'a>,
    /// `cutoff` is used to mark the end of what should be the "visible" environment.
    /// This allows us to make the complete environment at parse time, and then control visibility
    /// between threads by only making a particular slice of that environment available to a thread.
    cutoff: usize,
}

/// **声明表（T2-A：分层持久化 / COW）** —— 对齐 Lean `SMap`
/// （`~/Documents/lean/lean4/src/Lean/Data/SMap.lean:28-33`）。
///
/// ## 形状（为什么是两层）
///
/// * **`base`（库层 = Lean 的 `map₁`）**：平坦 `FxIndexMap`。库层趟里**独占**
///   （`stage1 == true`）⇒ 走破坏性写、O(1) 摊还插入 ✓；[`DeclarMap::seal`] 之后
///   **只读共享** ⇒ 克隆只提升引用计数 ✓。
/// * **`local`（本地层 = Lean 的 `map₂` 位）**：入口趟新增的声明；`Arc` + COW
///   ⇒ 克隆 O(1)，复制只发生在**封层后第一次插入**、且**只复制本地层**
///   （不是库层 ✗ —— 那正是 O(#decls) 的来源）。
///
/// Lean 的原话（同文件 `:17-25`）：导入条目**远多于**本地条目、HashMap 比 PHashMap 快、
/// 读导入文件时**独占** ⇒ 走破坏性写。我们的分界（库层 / 入口）与它同构 ✓。
///
/// ⚠ **刻意偏离（写在设计里）**：Lean 的 `map₂` 是**真 PHashMap**（HAMT）；
/// 我们的 `local` 用 `Arc` + COW（插入摊还 O(1)，但快照后第一次插入付一次
/// O(#local)）。`#local` = **入口文件自己的声明数**（课程 ≤ ~120）⇒ 量级已拿到；
/// 换 HAMT 的边际收益 ≤ 常数倍，却要新造一整个持久化**索引**结构（含位置索引与
/// 保序迭代两个额外要求）。边界见 `docs/design/persistent-declarations.md` §1。
///
/// ## 两条不变式（破一条就是静默错判 ✗）
///
/// 1. **`decl_idx` = 位置**：新声明的槽位 = `len()` = `base.len() + local.len()`；
///    **封层后 `base` 永不改变** ⇒ 既有 `decl_idx` 槽位逐字节不变 ✓。
/// 2. **`seal()` 之后不许再写 `base`**：否则共享的 `Arc` 被 `make_mut` 整份复制
///    ⇒ 克隆成本当场退回 O(#decls) ✗（反向验证见设计文档 §4）。
#[derive(Clone)]
pub struct DeclarMap<'a> {
    base: Arc<FxIndexMap<NamePtr<'a>, Declar<'a>>>,
    local: Arc<FxIndexMap<NamePtr<'a>, Declar<'a>>>,
    /// `true` ⇒ 插入进 `base`（库层趟）；`false` ⇒ 插入 `local`（入口趟）。
    /// 对齐 Lean `SMap.stage₁`（`SMap.lean:29`）。
    stage1: bool,
}

impl<'a> Default for DeclarMap<'a> {
    fn default() -> Self { Self::new() }
}

impl<'a> DeclarMap<'a> {
    pub fn new() -> Self {
        Self {
            base: Arc::new(crate::util::new_fx_index_map()),
            local: Arc::new(crate::util::new_fx_index_map()),
            stage1: true,
        }
    }

    /// **库层搭完 ⇒ 封层**（对齐 Lean `SMap.switch`，`SMap.lean:96-98`）：
    /// 之后的插入一律进 `local` ⇒ `base` 变成**只读共享**的那一份 ✓。
    ///
    /// 幂等 ✓（已经封层再调 = 什么也不做）。
    pub fn seal(&mut self) { self.stage1 = false; }

    /// 是否已封层（只读共享）。
    pub fn is_sealed(&self) -> bool { !self.stage1 }

    /// 库层的条目数（库层/入口层的分界；位置语义用）。
    pub fn base_len(&self) -> usize { self.base.len() }

    #[inline]
    pub fn len(&self) -> usize { self.base.len() + self.local.len() }

    #[inline]
    pub fn is_empty(&self) -> bool { self.base.is_empty() && self.local.is_empty() }

    /// 按名字查：**本地层先、库层后**（对齐 Lean `SMap.find?` 的 `map₂` 先查 ✓）。
    ///
    /// ⚠ `stage1 == true` 时 `local` 必空（[`Self::insert`] 只写 `base`、
    /// [`Self::seal`] 单调）⇒ 两层的先后在这一态下无差别 ✓。
    #[inline]
    pub fn get(&self, k: &NamePtr<'a>) -> Option<&Declar<'a>> {
        self.local.get(k).or_else(|| self.base.get(k))
    }

    #[inline]
    pub fn contains_key(&self, k: &NamePtr<'a>) -> bool { self.get(k).is_some() }

    /// **按位置取**（`decl_idx` 就是位置 ✓）：`inductive.rs` 按**下标区间**扫归纳块、
    /// `tc.rs` 批量检查按 `get_index(i)` 走 ⇒ 这条 API 必须留 ✓。
    #[inline]
    pub fn get_index(&self, i: usize) -> Option<(&NamePtr<'a>, &Declar<'a>)> {
        if i < self.base.len() {
            self.base.get_index(i)
        } else {
            self.local.get_index(i - self.base.len())
        }
    }

    /// 按名字取**位置**（`tc.rs:143` 的 recursor 序号）。
    #[inline]
    pub fn get_index_of(&self, k: &NamePtr<'a>) -> Option<usize> {
        if let Some(i) = self.local.get_index_of(k) {
            Some(self.base.len() + i)
        } else {
            self.base.get_index_of(k)
        }
    }

    /// 插入（**取位置 = `len()`，与既有 `set_decl_idx` 的调用点一致 ✓**）。
    #[inline]
    pub fn insert(&mut self, k: NamePtr<'a>, v: Declar<'a>) -> Option<Declar<'a>> {
        // ⚠ **共享时 `make_mut` 会整份复制** ⇒ 记进 COW 桶（判据①的 0 不许藏成本 ✗）。
        let layer = if self.stage1 { &mut self.base } else { &mut self.local };
        if Arc::get_mut(layer).is_none() {
            crate::util::clone_stats::record_cow(layer.len());
        }
        Arc::make_mut(layer).insert(k, v)
    }

    /// **克隆成本的判据形态（测试用）**：两张表是否**共享同一批底层 `FxIndexMap`**
    /// —— 共享 ⇒ 克隆只是引用计数、**不复制条目** ✓（判据①的结构面）。
    #[allow(dead_code)]
    pub(crate) fn shares_layers_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.base, &other.base) && Arc::ptr_eq(&self.local, &other.local)
    }

    /// 保序迭代（库层在前、本地层在后 = **插入序** ✓）。
    pub fn iter(&self) -> impl Iterator<Item = (&NamePtr<'a>, &Declar<'a>)> {
        self.base.iter().chain(self.local.iter())
    }

    pub fn keys(&self) -> impl Iterator<Item = &NamePtr<'a>> {
        self.base.keys().chain(self.local.keys())
    }

    pub fn values(&self) -> impl Iterator<Item = &Declar<'a>> {
        self.base.values().chain(self.local.values())
    }
}

impl<'a> std::ops::Index<usize> for DeclarMap<'a> {
    type Output = Declar<'a>;
    #[inline]
    fn index(&self, i: usize) -> &Self::Output {
        self.get_index(i)
            .map(|(_, d)| d)
            .expect("declaration index out of range")
    }
}

pub(crate) type NotationMap<'a> = crate::util::CowMap<NamePtr<'a>, Notation<'a>>;

impl<'x, 'a: 'x> Env<'x, 'a> {
    /// Create a new environment (without any temporary extension)
    pub fn new(declars: &'x DeclarMap<'a>, notation: &'x NotationMap<'a>, limit: EnvLimit<'a>) -> Self {
        Self::new_w_temp_ext(declars, None, notation, limit)
    }

    /// Create a new environment that includes some temporary extension; the temporary
    /// extension is used for checking nested inductives.
    pub fn new_w_temp_ext(
        declars: &'x DeclarMap<'a>,
        temp_declars: Option<&'x FxIndexMap<NamePtr<'a>, Declar<'a>>>,
        notation: &'x NotationMap<'a>,
        limit: EnvLimit<'a>
    ) -> Self {
        let cutoff = match limit {
            EnvLimit::Empty => 0,
            EnvLimit::ByIndex(idx) => idx,
            EnvLimit::PpUnlimited => declars.len(),
            EnvLimit::ByName(n) => match n.as_ref().decl_idx() {
                crate::name::NO_DECL => 0,
                idx => idx as usize,
            },
        };
        Self { declars, cutoff, temp_declars, notation }
    }

    /// Retrieve a declaration by first checking the contents of any temporary extension,
    /// then checking the persistent environment.
    pub fn get_declar(&self, n: &NamePtr<'a>) -> Option<&Declar<'a>> {
        self.temp_declars.as_ref().and_then(|ext| ext.get(n)).or_else(|| self.get_old_declar(n))
    }

    pub fn has_temp_ext(&self) -> bool { self.temp_declars.is_some() }

    /// Get a declaration, only looking in the temporary extension.
    pub fn get_temp_declar(&self, n: &NamePtr<'a>) -> Option<&Declar<'a>> {
        self.temp_declars.as_ref().and_then(|ext| ext.get(n))
    }

    /// Get a declaration, bypassing the temporary extension, only searching in
    /// the persistent set of declarations.
    ///
    /// ⚠⚠ **必须保持"按下标取"的既有语义**（T2-A 第一版曾改成"按名字查"⇒ **判红** ✗）：
    /// `NameNode::decl_idx` 是挂在**被 intern 的节点**上的槽位 ⇒ **跨 builder** 查询时
    /// 那个下标可能指向**另一份环境**的位置 —— 既有守卫
    /// `crates/kernel/tests/memory_api.rs::
    /// cross_builder_name_lookup_is_silently_positional_without_with_env`（K1-b）
    /// **正是钉住这条语义的** ✓。T2-A 落地时把它改成 `declars.get(n)`（"按名字查、
    /// 更正确"）⇒ 那条守卫当场判红（`const_head_type: unknown const`）✗
    /// ⇒ **已改回按下标取** ✓（分层结构不影响位置语义：`get_index(i)` = 库层/本地层
    /// 拼起来的第 i 个 ✓，与旧扁平表逐位相同）。
    pub fn get_old_declar(&self, n: &NamePtr<'a>) -> Option<&Declar<'a>> {
        let idx = n.as_ref().decl_idx() as usize;
        if idx < self.cutoff {
            self.declars.get_index(idx).map(|(_, d)| d)
        } else {
            None
        }
    }

    pub fn get_inductive(&self, n: &NamePtr<'a>) -> Option<&InductiveData<'a>> {
        match self.get_declar(n) {
            Some(Declar::Inductive(i)) => Some(i),
            _ => None,
        }
    }

    pub fn get_recursor(&self, n: &NamePtr<'a>) -> Option<&RecursorData<'a>> {
        match self.get_declar(n) {
            Some(Declar::Recursor(r)) => Some(r),
            _ => None,
        }
    }

    #[inline]
    pub fn get_constructor(&self, n: &NamePtr<'a>) -> Option<&ConstructorData<'a>> {
        match self.get_declar(n) {
            Some(Declar::Constructor(c)) => Some(c),
            _ => None,
        }
    }

    /// Returns `true` iff the inductive type declaration associated with `n` has the
    /// characteristics required of a structure. The requirements to be a structure are
    /// (1) the inductive declaration is not recursive, (2) the declaration has only one
    /// constructor, and (3) the type is declared with no indices.
    pub(crate) fn can_be_struct(&self, n: &NamePtr<'a>) -> bool { self.get_structure(n, false).is_some() }

    pub(crate) fn get_structure(&self, n: &NamePtr<'a>, rec_ok: bool) -> Option<&InductiveData<'a>> {
        match self.get_inductive(n) {
            Some(i @ InductiveData { is_recursive, num_indices, all_ctor_names, .. })
                if (all_ctor_names.len() == 1) && (*num_indices == 0) && (rec_ok || !is_recursive) =>
                Some(i),
            _ => None,
        }
    }

    /// Get the value of a declaration, if that declaration has an associated value (only
    /// definitions and theorems have values). Also returns the declaration's universe parameters.
    pub fn get_declar_val(&self, n: &NamePtr<'a>) -> Option<(LevelsPtr<'a>, ExprPtr<'a>)> {
        match self.get_declar(n)? {
            Declar::Definition { info, val, .. } | Declar::Theorem { info, val, .. } => Some((info.uparams, *val)),
            _ => None,
        }
    }
}
