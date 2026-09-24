//! In-memory declaration building for front-ends.
//!
//! The parser-style NDJSON path builds an [`ExportFile`] at the very end.
//! `EnvBuilder` gives the Sokonanoda front-end the same ability: construct
//! names, levels, expressions and declarations against one arena, then call
//! [`EnvBuilder::finish`] to obtain the environment.

use crate::env::InductiveData;
use crate::env::{Declar, DeclarInfo, DeclarMap, NotationMap};
use crate::expr::{
    BinderStyle, Expr, APP_HASH, CONST_HASH, LAMBDA_HASH, LET_HASH, NAT_LIT_HASH, PI_HASH, PROJ_HASH, SORT_HASH,
    STRING_LIT_HASH, VAR_HASH,
};
use crate::level::{Level, PARAM_HASH};
use crate::name::{Name, NUM_HASH, STR_HASH};
use crate::util::{
    new_fx_hash_map, new_fx_index_map, BigUintPtr, Config, CowStr, Dag, ExportFile, ExprPtr, LevelPtr, LevelsPtr,
    NamePtr, StringPtr,
};
use num_bigint::BigUint;
use std::sync::Arc;
use stumpalo::ArenaRef;

/// Accumulates declarations in dependency order and turns them into a kernel
/// [`ExportFile`] once all declarations for one compilation unit are ready.
pub struct EnvBuilder<'a> {
    arena: &'a ArenaRef<'a>,
    dag: Dag<'a>,
    anon: NamePtr<'a>,
    zero: LevelPtr<'a>,
    declars: DeclarMap<'a>,
    notations: NotationMap<'a>,
    config: Config,
    /// In-progress inductive block: (start index, member inductive names).
    block_in_progress: Option<(usize, Vec<NamePtr<'a>>)>,
    /// (start, size) per inductive-block head name, mirroring the NDJSON
    /// parser's bookkeeping; the kernel's inductive/recursor checkers need it
    /// to find block boundaries.
    mutual_block_sizes: rustc_hash::FxHashMap<NamePtr<'a>, (usize, usize)>,
}

impl<'a> EnvBuilder<'a> {
    /// **把 builder 的字段临时装进一个 `ExportFile<'a>` 交给回调，回调结束后装回**
    /// （T-K12 / K1-b；`ExportFile` 结构体一字不改）。
    ///
    /// 为什么需要它（见 `docs/design/by-prefix-reuse.md` §2.2）：front 的 judge 要判
    /// 一条**合成声明**，而检查器必须看见与 builder **完全同一份** intern 表 ——
    /// `NameNode::decl_idx` 是挂在**被 intern 的 NameNode** 上的全局槽位，换一份表
    /// 就会**静默取到别人的声明**（不报错）。这里让检查器直接用 builder 的活表，
    /// 指针恒等式天然成立。
    ///
    /// 而且它**从不 `add_declar`** ⇒ 合成声明不进环境，`decl_idx` 与环境都不被污染。
    ///
    /// `ExportFile`/`TcCtx`/`eval`/`conv`/`infer` **零改动** ⇒ 不碰
    /// `ExportFile: Sync` 与 `ArenaRef: !Send/!Sync` 那对矛盾（K1-c 被否的理由）。
    ///
    /// ⚠ `f` 里**不要再嵌套** `with_env`（builder 此刻是空壳）；也别在 `f` 里碰
    /// `self` —— 借用检查器会挡住，这正是我们想要的。
    /// **取一份只读快照**（T-K13）：把 builder 的字段**克隆**进一个 `ExportFile`。
    ///
    /// 与 [`EnvBuilder::with_env`] 的关键区别：`with_env` 是**借出再装回**
    /// （回调期间 builder 是空壳 ✗），而 `snapshot` 是**复制**（builder 原封不动 ✓）。
    /// 检查器拿快照**只读**用 ⇒ **不写任何 `decl_idx` 槽位** ✓
    /// ⇒ 天然绕开"往独立环境里 `add_declar` 会改写共享槽位"那堵墙
    /// （T-K12c 的死因：跳过一条声明就整体错位 ⇒ 假 `def_eq mismatch` ✓，
    /// 见 `docs/design/vscode-editor-feedback-plan.md` 的 T-K12c）。
    ///
    /// ⚠ **快照必须在合成声明建好之后取**（规格里的陷阱 ✓）：内核多处依赖
    /// "同一字面量/名字 ⇒ 同一指针"（`conv.rs` 的 `NatLit` 指针相等、
    /// `eval.rs` 用 `ptr.get_hash()`＝**地址**做内容哈希、`NameInterner::get`
    /// 比 `StringPtr` 的**地址**）⇒ 快照若早于合成声明里新出现的字面量，
    /// 检查器会为同一个值造出第二个指针 ⇒ **本该判过的 def_eq 假失败** ✗。
    ///
    /// 成本：一次 O(表大小) memcpy（数 MB × 每个 by-site = 几十 ms，可忽略 ✓）。
    pub fn snapshot(&self) -> ExportFile<'a> {
        ExportFile {
            dag: self.dag.clone(),
            anon: self.anon,
            zero: self.zero,
            declars: self.declars.clone(),
            notations: self.notations.clone(),
            // `name_cache` 是**派生**缓存（builder 不持有它）⇒ 现造一个。
            name_cache: self.dag.mk_name_cache(self.anon),
            config: self.config.clone(),
            mutual_block_sizes: self.mutual_block_sizes.clone(),
        }
    }

    pub fn with_env<R>(&mut self, f: impl FnOnce(&mut ExportFile<'a>) -> R) -> R {
        // 占位 dag：回调期间 builder 不可用 ⇒ 占位不会被读到（`new_local` 很便宜）。
        let placeholder = Dag::new_local(&self.config);
        let dag = std::mem::replace(&mut self.dag, placeholder);
        let anon = self.anon;
        let name_cache = dag.mk_name_cache(anon);
        let mut env = ExportFile {
            dag,
            anon,
            zero: self.zero,
            declars: std::mem::take(&mut self.declars),
            notations: std::mem::take(&mut self.notations),
            name_cache,
            config: self.config.clone(),
            mutual_block_sizes: std::mem::take(&mut self.mutual_block_sizes),
        };
        let out = f(&mut env);
        // 装回：`dag`/`declars`/`notations`/`mutual_block_sizes` 都是"搬来搬去"，
        // 没有克隆 ⇒ 指针恒等式与 intern 表**逐字节不变**。`name_cache` 是派生
        // 缓存，丢弃即可（builder 本来就不持有它）。
        self.dag = env.dag;
        self.declars = env.declars;
        self.notations = env.notations;
        self.mutual_block_sizes = env.mutual_block_sizes;
        out
    }

    pub fn new(arena: &'a ArenaRef<'a>, config: Config) -> Self {
        let mut dag = Dag::new_local(&config);
        let anon = NamePtr::global(dag.names.intern(arena, Name::Anon));
        let zero = LevelPtr::global(dag.levels.intern(arena, Level::Zero));
        Self {
            arena,
            dag,
            anon,
            zero,
            declars: new_fx_index_map(),
            notations: new_fx_hash_map(),
            config,
            block_in_progress: None,
            mutual_block_sizes: new_fx_hash_map(),
        }
    }

    // -- inductive blocks ----------------------------------------------------

    /// Mark the start of an `inductive ... end` block so kernel checkers can
    /// locate its boundaries (mirrors the NDJSON parser's bookkeeping).
    pub fn begin_inductive_block(&mut self) {
        self.block_in_progress = Some((self.declars.len(), Vec::new()));
    }

    /// Close the inductive block opened by [`begin_inductive_block`].
    pub fn end_inductive_block(&mut self) {
        let Some((start, names)) = self.block_in_progress.take() else {
            return;
        };
        let size = self.declars.len() - start;
        for name in names {
            self.mutual_block_sizes.insert(name, (start, size));
        }
    }

    // -- names / strings ----------------------------------------------------

    pub fn anonymous(&self) -> NamePtr<'a> {
        self.anon
    }

    pub fn alloc_string(&mut self, s: &str) -> StringPtr<'a> {
        let cow: CowStr<'a> = CowStr::Owned(s.to_owned());
        StringPtr::global(self.dag.strings.intern(self.arena, cow))
    }

    pub fn name_from_str(&mut self, s: &str) -> NamePtr<'a> {
        let mut out = self.anon;
        for segment in s.split('.') {
            if let Ok(n) = segment.parse::<u64>() {
                out = self.name_num(out, n);
            } else {
                out = self.name_str(out, segment);
            }
        }
        out
    }

    fn name_str(&mut self, pfx: NamePtr<'a>, sfx: &str) -> NamePtr<'a> {
        let sfx = self.alloc_string(sfx);
        let hash = crate::hash64!(STR_HASH, pfx, sfx);
        let name = Name::Str(pfx, sfx, hash);
        NamePtr::global(self.dag.names.intern(self.arena, name))
    }

    fn name_num(&mut self, pfx: NamePtr<'a>, n: u64) -> NamePtr<'a> {
        let hash = crate::hash64!(NUM_HASH, pfx, n);
        let name = Name::Num(pfx, n, hash);
        NamePtr::global(self.dag.names.intern(self.arena, name))
    }

    // -- levels -------------------------------------------------------------

    pub fn zero(&self) -> LevelPtr<'a> {
        self.zero
    }

    pub fn succ(&mut self, level: LevelPtr<'a>) -> LevelPtr<'a> {
        let hash = crate::hash64!(crate::level::SUCC_HASH, level);
        self.alloc_level(Level::Succ(level, hash))
    }

    pub fn level_param(&mut self, name: NamePtr<'a>) -> LevelPtr<'a> {
        let hash = crate::hash64!(PARAM_HASH, name);
        self.alloc_level(Level::Param(name, hash))
    }

    pub fn alloc_levels_slice(&mut self, levels: &[LevelPtr<'a>]) -> LevelsPtr<'a> {
        LevelsPtr::global(self.dag.uparams.intern(self.arena, levels))
    }

    fn alloc_level(&mut self, level: Level<'a>) -> LevelPtr<'a> {
        LevelPtr::global(self.dag.levels.intern(self.arena, level))
    }

    // -- expressions --------------------------------------------------------

    pub fn mk_var(&mut self, dbj_idx: u16) -> ExprPtr<'a> {
        let hash = crate::hash64!(VAR_HASH, dbj_idx);
        self.alloc_expr(Expr::Var { dbj_idx, hash })
    }

    pub fn mk_sort(&mut self, level: LevelPtr<'a>) -> ExprPtr<'a> {
        let hash = crate::hash64!(SORT_HASH, level);
        self.alloc_expr(Expr::Sort { level, hash })
    }

    pub fn mk_const(&mut self, name: NamePtr<'a>, levels: LevelsPtr<'a>) -> ExprPtr<'a> {
        let hash = crate::hash64!(CONST_HASH, name, levels);
        self.alloc_expr(Expr::Const { name, levels, hash })
    }

    pub fn mk_app(&mut self, fun: ExprPtr<'a>, arg: ExprPtr<'a>) -> ExprPtr<'a> {
        let hash = crate::hash64!(APP_HASH, fun, arg);
        let fv_mask = crate::expr::child_mask(fun) | crate::expr::child_mask(arg);
        self.alloc_expr(Expr::App { fun, arg, fv_mask, hash })
    }

    pub fn mk_lambda(
        &mut self,
        binder_name: NamePtr<'a>,
        binder_style: BinderStyle,
        binder_type: ExprPtr<'a>,
        body: ExprPtr<'a>,
    ) -> ExprPtr<'a> {
        let hash = crate::hash64!(LAMBDA_HASH, binder_name, binder_style, binder_type, body);
        let fv_mask = crate::expr::child_mask(binder_type) | crate::expr::body_mask(body);
        self.alloc_expr(Expr::Lambda { binder_name, binder_style, binder_type, body, fv_mask, hash })
    }

    pub fn mk_pi(
        &mut self,
        binder_name: NamePtr<'a>,
        binder_style: BinderStyle,
        binder_type: ExprPtr<'a>,
        body: ExprPtr<'a>,
    ) -> ExprPtr<'a> {
        let hash = crate::hash64!(PI_HASH, binder_name, binder_style, binder_type, body);
        let fv_mask = crate::expr::child_mask(binder_type) | crate::expr::body_mask(body);
        self.alloc_expr(Expr::Pi { binder_name, binder_style, binder_type, body, fv_mask, hash })
    }

    pub fn mk_let(
        &mut self,
        binder_name: NamePtr<'a>,
        binder_type: ExprPtr<'a>,
        val: ExprPtr<'a>,
        body: ExprPtr<'a>,
        nondep: bool,
    ) -> ExprPtr<'a> {
        let hash = crate::hash64!(LET_HASH, binder_name, binder_type, val, body, nondep);
        let fv_mask =
            crate::expr::child_mask(binder_type) | crate::expr::child_mask(val) | crate::expr::body_mask(body);
        let data = self.arena.alloc(crate::expr::LetData { binder_name, binder_type, val, body, nondep });
        self.alloc_expr(Expr::Let { data, fv_mask, hash })
    }

    pub fn mk_proj(&mut self, ty_name: NamePtr<'a>, idx: u16, structure: ExprPtr<'a>) -> ExprPtr<'a> {
        let hash = crate::hash64!(PROJ_HASH, ty_name, idx, structure);
        let fv_mask = crate::expr::child_mask(structure);
        self.alloc_expr(Expr::Proj { ty_name, idx, structure, fv_mask, hash })
    }

    pub fn mk_nat_lit(&mut self, ptr: BigUintPtr<'a>) -> Option<ExprPtr<'a>> {
        if !self.config.nat_extension {
            return None;
        }
        let hash = crate::hash64!(NAT_LIT_HASH, ptr);
        Some(self.alloc_expr(Expr::NatLit { ptr, hash }))
    }

    pub fn alloc_bignum(&mut self, n: BigUint) -> Option<BigUintPtr<'a>> {
        let local = self.dag.bignums.as_mut()?;
        Some(BigUintPtr::global(local.intern(self.arena, n)))
    }

    pub fn mk_string_lit(&mut self, ptr: StringPtr<'a>) -> Option<ExprPtr<'a>> {
        if !self.config.string_extension {
            return None;
        }
        let hash = crate::hash64!(STRING_LIT_HASH, ptr);
        Some(self.alloc_expr(Expr::StringLit { ptr, hash }))
    }

    fn alloc_expr(&mut self, e: Expr<'a>) -> ExprPtr<'a> {
        ExprPtr::local(self.dag.exprs.intern(self.arena, e))
    }

    // -- declarations -------------------------------------------------------

    pub fn declaration_count(&self) -> usize {
        self.declars.len()
    }

    pub fn add_declar(&mut self, d: Declar<'a>) -> Result<(), String> {
        let name = d.info().name;
        if self.declars.contains_key(&name) {
            return Err(format!("duplicate declaration {}", self.name_to_string(name)));
        }
        let idx = self.declars.len();
        name.as_ref().set_decl_idx(u32::try_from(idx).expect("more than u32::MAX declarations in one environment"));
        self.declars.insert(name, d);
        Ok(())
    }

    /// Add a trusted inductive type. Kernel-side validation of inductive
    /// blocks happens when full export-style declarations are checked; this is
    /// the entry point used by the built-in teaching prelude. Returns the
    /// built declaration so front-ends can queue it for kernel checking.
    #[allow(clippy::too_many_arguments)]
    pub fn add_inductive(
        &mut self,
        info: DeclarInfo<'a>,
        is_recursive: bool,
        num_params: u16,
        num_indices: u16,
        all_ind_names: Arc<[NamePtr<'a>]>,
        all_ctor_names: Arc<[NamePtr<'a>]>,
    ) -> Result<Declar<'a>, String> {
        let declar = Declar::Inductive(InductiveData {
            info,
            is_recursive,
            is_nested: false,
            num_params,
            num_indices,
            all_ind_names,
            all_ctor_names,
        });
        if let Some((_, names)) = &mut self.block_in_progress {
            names.push(declar.info().name);
        }
        self.add_declar(declar.clone())?;
        Ok(declar)
    }

    fn name_to_string(&self, name: NamePtr<'a>) -> String {
        match name.as_ref().kind {
            Name::Anon => String::new(),
            Name::Str(pfx, sfx, _) => {
                let prefix = self.name_to_string(pfx);
                if prefix.is_empty() {
                    sfx.as_ref().to_string()
                } else {
                    format!("{prefix}.{}", sfx.as_ref())
                }
            }
            Name::Num(pfx, n, _) => {
                let prefix = self.name_to_string(pfx);
                if prefix.is_empty() {
                    n.to_string()
                } else {
                    format!("{prefix}.{n}")
                }
            }
        }
    }

    pub fn finish(self) -> ExportFile<'a> {
        let anon = self.anon;
        let name_cache = self.dag.mk_name_cache(anon);
        ExportFile {
            dag: self.dag,
            anon,
            zero: self.zero,
            declars: self.declars,
            notations: self.notations,
            name_cache,
            config: self.config,
            mutual_block_sizes: self.mutual_block_sizes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stumpalo::Arena;

    #[test]
    fn name_cache_discovers_nat() {
        let arena = Arena::new();
        let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let _ = builder.name_from_str("Nat");
        let env = builder.finish();
        assert!(env.name_cache.nat.is_some(), "Nat was not discovered by name cache");
    }
}
