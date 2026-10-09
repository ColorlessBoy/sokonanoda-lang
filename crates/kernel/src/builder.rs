//! In-memory declaration building for front-ends.
//!
//! The parser-style NDJSON path builds an [`ExportFile`] at the very end.
//! `EnvBuilder` gives the Sokonanoda front-end the same ability: construct
//! names, levels, expressions and declarations against one arena, then call
//! [`EnvBuilder::finish`] to obtain the environment.

use crate::env::InductiveData;
use crate::env::{Declar, DeclarInfo, DeclarMap, NotationMap};
use crate::expr::{
    BinderStyle, Expr, MetaKind, APP_HASH, CONST_HASH, LAMBDA_HASH, LET_HASH, META_HASH,
    NAT_LIT_HASH, PI_HASH, PROJ_HASH, SORT_HASH,
    STRING_LIT_HASH, VAR_HASH,
};
use crate::level::{Level, PARAM_HASH};
use crate::name::{Name, NUM_HASH, STR_HASH};
use crate::util::{
    BigUintPtr, Config, CowStr, Dag, ExportFile, ExprPtr, LevelPtr, LevelsPtr,
    NamePtr, StringPtr,
};
use num_bigint::BigUint;
use std::sync::Arc;
use stumpalo::ArenaRef;

/// Accumulates declarations in dependency order and turns them into a kernel
/// [`ExportFile`] once all declarations for one compilation unit are ready.
///
/// **`Clone`（2026-10-08 · G-29 / 设计 §33）**：库层级检查点（"只有库层"的环境）
/// 要**跨调用**留着，而接着编入口必须拿一份**可变的副本**（跑完入口那趟之后
/// 检查点本身要原封不动）。克隆是**浅**的：`dag` 的 intern 表与 `declars` 的键值
/// 都是**指针的拷贝** ⇒ 同一份 arena、同一批 `ExprPtr`/`NamePtr` 地址
/// ⇒ 内核那些**按指针比较**的地方（`conv.rs` 的 `NatLit`、`eval.rs` 用地址做
/// 内容哈希、`NameInterner` 比 `StringPtr` 地址）**逐字节不变** ✓
/// —— 与 `hide_declars`/`restore_declars`（`DeclarMap: Clone`）同一个不变式，
/// 见本文件的
/// **T2-A：命令边界的环境快照**（`Clone` = **O(1)**）。
///
/// ## 为什么**不含** `Dag`（intern 表）—— 单调 intern 引理
///
/// intern 表**只增不删**、节点在 arena 内**不可变** ⇒ `intern(v)` 一旦给出某指针，
/// 之后恒给**同一个**指针；表在时刻 `t` 的状态恒是其后任意状态的**子集**。
/// ⇒ 拿"更新的表"重放旧命令，得到的指针与首次运行**逐字节相同** ✓。
/// 所以一趟只需要**一份**活表（判据⑤：**不许**第二份 intern 表），
/// 快照**不必**捕获它 —— `Dag` 分项从 O(#interned) 直接归 **0**
/// （不是"少复制一点"，是**不复制** ✓）。
///
/// ## 装回（[`EnvBuilder::restore_env`]）
///
/// 三张表整体装回（`declars` 连同它的 `seal` 状态 ✓）；`Dag` **一个字节都不动** ✓。
/// ⚠ 命令边界上 `block_in_progress` 恒为 `None`（归纳块在一条命令内闭合，
/// 见 `walk.rs` 的 `inductive_block`）⇒ 不进快照 ✓。
///
/// 详见 `docs/design/persistent-declarations.md` §2.2/§2.3。
pub struct EnvSnapshot<'a> {
    declars: DeclarMap<'a>,
    notations: NotationMap<'a>,
    mutual_block_sizes: crate::util::CowMap<NamePtr<'a>, (usize, usize)>,
}

/// 手写（不是 derive）⇒ 克隆成本进 [`crate::util::clone_stats`] ✓：
/// 三张表**全是共享根** ⇒ **0 条目**被复制（判据①的目标值 ✓）。
/// 「共享」不是口头承诺：[`EnvSnapshot::shares_tables_with`] 用 `Arc::ptr_eq` 直接判 ✓。
impl<'a> Clone for EnvSnapshot<'a> {
    fn clone(&self) -> Self {
        crate::util::clone_stats::record(0, 0, 0);
        Self {
            declars: self.declars.clone(),
            notations: self.notations.clone(),
            mutual_block_sizes: self.mutual_block_sizes.clone(),
        }
    }
}

impl<'a> EnvSnapshot<'a> {
    /// **判据①的结构面（测试用）**：这份快照与那个 builder 是否**共享同一批表**
    /// —— 共享 ⇒ 上面的 `record(0,0,0)` 是**事实**而不是声明 ✓；
    /// 谁把共享换成复制，这条当场判红 ✓。
    #[allow(dead_code)]
    pub(crate) fn shares_tables_with(&self, b: &EnvBuilder<'a>) -> bool {
        self.declars.shares_layers_with(&b.declars)
            && self.notations.shares_with(&b.notations)
            && self.mutual_block_sizes.shares_with(&b.mutual_block_sizes)
    }
}

/// `a_reused_environment_is_pointer_identical_and_a_rebuilt_one_is_not` ✓。
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
    mutual_block_sizes: crate::util::CowMap<NamePtr<'a>, (usize, usize)>,
}

/// **手写 `Clone`（不是 derive）** —— T2-A 的**结构计数**在唯一的克隆出口记账 ✓：
/// 「在命令边界取一份环境」要复制多少**表条目**（判据①，见
/// [`crate::util::clone_stats`]，读数 = `declars` + intern 表 + 另两张表）。
///
/// ⚠ **记账是纯读**（各表的 `len()`）⇒ 克隆出来的环境与 derive 版**逐字段相同**，
/// 判定行为零变化 ✓（硬规则 1 的红线不受影响 ✓）。
/// **先建先红**：T2-A 落地前，这个读数 = O(#decls + #interned)，判据①当场判红 ✓。
impl<'a> Clone for EnvBuilder<'a> {
    fn clone(&self) -> Self {
        // **T2-A 之后**：`declars`（分层 `Arc` 共享）与另两张表（`CowMap`）的克隆都
        // 只提升引用计数 ⇒ **0 条目** ✓；仍然逐条复制的只剩 intern 表（`Dag`）。
        // ⚠ **命令边界不走这条** —— 那条走 [`EnvSnapshot`]（不含 `Dag`，依据是它上面的
        // **单调 intern 引理**）✓。
        crate::util::clone_stats::record(0, self.dag.table_entries(), 0);
        Self {
            arena: self.arena,
            dag: self.dag.clone(),
            anon: self.anon,
            zero: self.zero,
            declars: self.declars.clone(),
            notations: self.notations.clone(),
            config: self.config.clone(),
            block_in_progress: self.block_in_progress.clone(),
            mutual_block_sizes: self.mutual_block_sizes.clone(),
        }
    }
}

impl<'a> EnvBuilder<'a> {
    /// **T2-A**：把当前的**环境三张表**取成一份快照 —— **克隆成本 O(1)** ✓
    /// （`declars` 分层共享、另两张表是 `CowMap`；`Dag` **不进快照**，
    /// 依据见 [`EnvSnapshot`] 的单调 intern 引理 ✓）。
    ///
    /// 这是 T2-B「入口趟命令级环境快照」按命令边界要取的那一份 ✓。
    pub fn env_snapshot(&self) -> EnvSnapshot<'a> {
        // 取快照 = 三张表各提升一次引用计数 ⇒ **0 条目**被复制 ✓（判据①在**这个**
        // 调用点记账 —— 它才是"命令边界取一份环境"那一下）。
        crate::util::clone_stats::record(0, 0, 0);
        EnvSnapshot {
            declars: self.declars.clone(),
            notations: self.notations.clone(),
            mutual_block_sizes: self.mutual_block_sizes.clone(),
        }
    }

    /// **T2-A**：把一份快照装回（[`Self::env_snapshot`] 的逆）—— `Dag` **不动** ✓
    /// ⇒ 快照前后的 intern 指针**同一** ✓（判据⑤：只有一份 intern 表）。
    ///
    /// ⚠ 装回会**丢弃**当前三张表（连同当前 `seal` 状态）⇒ 只在"重新从某一命令边界
    /// 往后编"那条路上用 ✓（正是 T2-B 的形状）。
    pub fn restore_env(&mut self, snap: &EnvSnapshot<'a>) {
        self.declars = snap.declars.clone();
        self.notations = snap.notations.clone();
        self.mutual_block_sizes = snap.mutual_block_sizes.clone();
    }

    /// **T2-A**：库层搭完 ⇒ **封层**（对齐 Lean `SMap.switch`）。
    ///
    /// 入口趟**必须**在插入任何入口声明**之前**调它：封层后插入走 `local`
    /// （只复制本地层 ✓）；不封层则共享的库层会被 `Arc::make_mut` **整份复制** ✗
    /// ⇒ 判据①当场退回 O(#decls)（反向验证见设计档 §4）。
    pub fn seal_library_layer(&mut self) { self.declars.seal(); }

    /// 库层是否已封层（= 插入走本地层、库层只读共享）。
    pub fn library_layer_sealed(&self) -> bool { self.declars.is_sealed() }

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

    /// **T-D8**：给**合成探针**一个"**看不见文件声明**"的环境 ✓ ——
    /// 只把 `declars` **暂时挪走** ✓、**`dag` 不动** ✓ ⇒ **指针同一性保住** ✓
    ///（与 `with_env` 那套"搬来搬去"**同一手法** ✓，见 `:99`/`:110` ✓）。
    ///
    /// 为什么需要它（T-D8 的 C1 ✓）：`build_redundant_probes` 造的探针是**合成声明** ✓，
    /// 它要在**真 `builder` 的 DAG** 里 elaborate（否则 `def_eq` 因指针不同而**假失败** ✗，
    /// 见 `snapshot` 的 ⚠ 与 round 285/294 ✓），**但**它不该看见文件里的声明 ✓
    ///（A 步把声明提前加进真 `builder` 之后 ✓，探针看到它们 ⇒ 冗余判定翻转 ✗）。
    ///
    /// **默认路径不调用它** ✓ ⇒ 判定行为**零变化** ✓（硬规则 1 的红线不受影响 ✓）。
    /// **T-D8**：把 `declars` **挪走并返回** ✓（调用方随后用 `restore_declars` 还回 ✓）。
    /// 为什么拆成两个方法而**不是**闭包 ✗：调用点要同时给出 `&mut self.builder` 与
    /// `&self.known` ✓（两个**不同字段** ✓，靠**字段级借用拆分**才合法 ✓）——
    /// 闭包形式会让 `self` 被**可变借两次** ✗ ⇒ **必然借用错** ✗（round 301 预判 ✓）。
    pub fn hide_declars(&mut self) -> DeclarMap<'a> {
        std::mem::take(&mut self.declars)
    }

    /// 与 [`Self::hide_declars`] 配对 ✓：把声明表还回去 ✓。
    pub fn restore_declars(&mut self, saved: DeclarMap<'a>) {
        self.declars = saved;
    }

    /// **同时**借出"只读环境"与"可变 builder"（judge 增量路径的最小切口，2026-09-29）。
    ///
    /// **为什么需要它**（实测链条）：
    /// * 前端 `judge_infer` 要回答"这个项什么类型"，但它今天只拿到 `prefix_src: &str`
    ///   ⇒ 只能把**整段前缀**合成文件、走 `check_document_with` **从零重跑一趟 pass** ✗。
    ///   缓存键含**整段前缀哈希** ⇒ 前缀随声明序号线性变长 ⇒ 后段全 miss ⇒ **O(N²)**。
    ///   实测：judge 占墙钟 **≈88%**（219.3s → 跳掉后 **26.8s**）、合成 pass **253513** 次
    ///   = 自身声明事件的 **95.8×**、最贵单条 **4680ms**。
    /// * 要就地查表，就得**同时**"读当前环境"（`Env`）与"写新项"（builder 的 `mk_*`）。
    ///   而 `elab_expr` 已经 `&mut EnvBuilder` ⇒ 再借一次是冲突 ✗（§12）。
    /// * **关键观察**：`Env` 只需要 `&declars` + `&notations`，而 `mk_*` 只需要 `dag`
    ///   —— 它们是**不同字段** ⇒ **Rust 的分离字段借用允许同时借** ✓。
    ///
    /// **本方法提供的**：闭包拿到 `(&Env, &mut EnvBuilder)` ——
    /// `Env` 的 `cutoff` 用 `EnvLimit::PpUnlimited`（看得到**已落地**的全部声明；
    /// 要"只看前缀"由调用方按需用 `Env::new` + `EnvLimit::ByIndex` 自建）。
    ///
    /// **语义**：**纯新增**，没有任何既有调用点改用它 ⇒ 既有行为零变化 ✓。
    /// **指针同一性保住** ✓：`Env` 借的是**同一个** `declars`/`notations`，
    /// `EnvBuilder` 还是**同一个** `dag` ⇒ `NatLit` 的指针比较不受影响
    /// （这正是 §17 否掉 B″ 的那条红线）。
    /// ⚠ **实现用 `mem::take`（与 `with_env` 同一手法）**：Rust 不允许在同一表达式里
    /// `&self.declars` 与 `&mut self` 共存（**即使字段不同** —— 因为 `f(&env, self)`
    /// 里 `self` 是**整体**可变借）✗。所以把两张表**挪出去**、借它们、调用完再还回来：
    /// 期间 `self` 的 `declars`/`notations` 是**空的** ⇒ **回调里不许依赖它们**
    /// （`mk_*` 只碰 `dag`/`arena`，满足 ✓）。
    pub fn with_env_scope<R>(
        &mut self,
        f: impl FnOnce(&crate::env::Env<'_, 'a>, &mut EnvBuilder<'a>) -> R,
    ) -> R {
        let declars = std::mem::take(&mut self.declars);
        let notations = std::mem::take(&mut self.notations);
        let env = crate::env::Env::new(&declars, &notations, crate::env::EnvLimit::PpUnlimited);
        let out = f(&env, self);
        self.declars = declars;
        self.notations = notations;
        out
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
            declars: DeclarMap::new(),
            notations: NotationMap::default(),
            config,
            block_in_progress: None,
            mutual_block_sizes: crate::util::CowMap::default(),
        }
    }

    /// **T1-B 批 1 的装载口**（2026-10-09）：从一份**已建好的内核环境**
    /// （[`ExportFile`]）**继续编**（装载之后照常 `add_declar` / `finish`）。
    ///
    /// ## 用途（设计 `docs/design/module-artifacts.md` §2.4 T1-B 第 2 件）
    ///
    /// 磁盘模块产物（`.olean` 等价物）的消费者要"**对着已 elaborate 的依赖环境**
    /// 编自己那一层" —— 那要求内核能把一份环境的**全部表**原样接过来。
    /// `builder.rs` 原有七个方法（`new` / `snapshot` / `hide_declars` /
    /// `restore_declars` / `with_env` / `with_env_scope` / `finish`）**都不够**：
    /// `new` 从空开始、`snapshot`/`with_env` 只借只读副本、`finish` 是反方向 ✗。
    ///
    /// ## 两条硬约束（都由调用方负责 · 本函数只做"搬家"）
    ///
    /// 1. **`arena` 必须是 `file` 那份表所在的同一个 arena**（或寿命 ⊇ 它的）：
    ///    表里全是 arena 内裸地址（`ExprPtr`/`NamePtr`/`LevelPtr`）⇒ 换 arena
    ///    就是**悬空指针** ✗。T1-B 的形状是"**装载进本趟 pass 自己的 arena**"
    ///    （设计 §2.4 第 3 件）⇒ 这条不是可选优化，是**设计约束** ✓。
    /// 2. **`file` 必须在声明边界上**（`block_in_progress` 丢弃为 `None`）：
    ///    归纳块编到一半的 `ExportFile` **不是**一个合法环境（内核检查是整块做的）
    ///    ⇒ 装载后 `begin_inductive_block` 的记账从头来 ✓。
    ///
    /// **指针同一性**：本函数**只搬字段、不重建任何东西**（`Dag` 的 intern 表也
    /// 是整份搬走 ✓）⇒ 与 `hide_declars`/`restore_declars`、`snapshot` 同一条
    /// 不变式 —— **同一份表、同一批地址** ✓（判据见本文件
    /// `from_export_file_carries_the_intern_tables_not_a_rebuilt_dag`）。
    pub fn from_export_file(arena: &'a ArenaRef<'a>, file: ExportFile<'a>) -> Self {
        Self {
            arena,
            dag: file.dag,
            anon: file.anon,
            zero: file.zero,
            declars: file.declars,
            notations: file.notations,
            config: file.config,
            // 归纳块记账**不跨 `finish`**：装载点必须是声明边界 ✓（见文档）。
            block_in_progress: None,
            mutual_block_sizes: file.mutual_block_sizes,
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

    /// **R1b（2026-10-05）**：构造一个**层元变量** ✓ —— **对齐 Lean `mkFreshLevelMVar`** 的产物形状 ✓
    /// （`Lean/Level.lean:94` `| mvar : LMVarId → Level` ✓；hash 用 Lean 的 **2237** ✓）。
    ///
    /// ⚠ **加法**：本片**不接线** ✗ ⇒ **零行为变化** ✓（R1c 才在出口把它转成 `param` ✓）。
    pub fn level_mvar(&mut self, id: u64) -> LevelPtr<'a> {
        let hash = crate::hash64!(crate::level::MVAR_HASH, id);
        self.alloc_level(Level::MVar(id, hash))
    }

    pub fn level_param(&mut self, name: NamePtr<'a>) -> LevelPtr<'a> {
        let hash = crate::hash64!(PARAM_HASH, name);
        self.alloc_level(Level::Param(name, hash))
    }

    /// **R1c-2b（2026-10-05）**：`max` / `imax` 的构造器 ✓ ——
    /// 与 `TcCtx::max`/`TcCtx::imax`（`util.rs:967/971` ✓）**同一形状** ✓。
    ///
    /// ⚠ **为什么需要** ✗：出口重写（把层 mvar 换成 `param` ✓）要**重建**含 mvar 的
    /// 复合层 ✓ —— 而 `TcCtx` 的分配落在 `with_ctx` 的**作用域 arena** 上 ✗
    /// （指针出不了那个作用域 ✓）⇒ 声明的 `ty`/`val` 必须在 `EnvBuilder`（**持久 DAG**）上重建 ✓。
    /// **加法、零行为变化** ✓（今天没有调用方 ✓）。
    pub fn level_max(&mut self, l: LevelPtr<'a>, r: LevelPtr<'a>) -> LevelPtr<'a> {
        let hash = crate::hash64!(crate::level::MAX_HASH, l, r);
        self.alloc_level(Level::Max(l, r, hash))
    }

    pub fn level_imax(&mut self, l: LevelPtr<'a>, r: LevelPtr<'a>) -> LevelPtr<'a> {
        let hash = crate::hash64!(crate::level::IMAX_HASH, l, r);
        self.alloc_level(Level::IMax(l, r, hash))
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

    /// **IA-4 K1（D8 = (i)）**：造一个**占位符** ✓。
    ///
    /// ⚠ **本片（K1）没有任何调用方** ✗ —— 它是给 **B2「元变量进项」** 准备的载体 ✓；
    /// 一旦有东西造出它，**含占位符的声明就必须被硬拒** ✓（见 `add_decl` 那条路 ✓）。
    pub fn mk_meta(&mut self, id: u32) -> ExprPtr<'a> {
        self.mk_meta_with_kind(id, MetaKind::Natural)
    }

    /// **IA-4 B2（2026-10-05 ✓）**：带**种类**的占位符 ✓（设计 §4 第 2 行 ✓）。
    ///
    /// ⚠ **`hash` 必须含 `kind`** ✗ —— 否则 `Natural` 与 `SyntheticOpaque` 的**同 id**
    /// 占位符会**撞哈希** ✗（结构共享/去重会把两者混为一谈 ✗）。
    pub fn mk_meta_with_kind(&mut self, id: u32, kind: MetaKind) -> ExprPtr<'a> {
        let salt = match kind {
            MetaKind::Natural => 0u64,
            MetaKind::SyntheticOpaque => 1u64,
        };
        let hash = crate::hash64!(META_HASH, id as u64, salt);
        self.alloc_expr(Expr::Meta { id, kind, hash })
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
        // **IA-4 K1 硬不变式①（设计 §2.11 ✓）**：**含占位符的声明一律拒绝** ✗。
        //
        // 为什么在**入口**拒（而不是在各判定点拒）：占位符（`Expr::Meta` ✓）是给
        // **B2「元变量进项」** 准备的 ✓ —— 它**只允许活在 elaborate 期** ✓，
        // **绝不许**进环境 ✗。入口这一条把"判定层见不到它"变成**结构性保证** ✓，
        // 于是 `conv`/`infer`/`eval` 里那些 `panic!` 臂**不可达** ✓。
        //
        // 错误消息**固定前缀** ✓（前端据此映射错误码 ✓ —— 契约见 `docs/protocol.md`）。
        if self.declar_contains_meta(&d) {
            return Err(format!(
                "declaration contains a metavariable placeholder: {}",
                self.name_to_string(name)
            ));
        }
        if self.declars.contains_key(&name) {
            return Err(format!("duplicate declaration {}", self.name_to_string(name)));
        }
        let idx = self.declars.len();
        name.as_ref().set_decl_idx(u32::try_from(idx).expect("more than u32::MAX declarations in one environment"));
        self.declars.insert(name, d);
        Ok(())
    }

    /// **IA-4 K1 硬不变式①**：这个项里有没有**占位符** ✓（`Expr::Meta`）。
    ///
    /// ⚠ **必须走遍所有子项** ✗ —— 漏一处就等于留一条"含元变量的声明能进环境"的缝 ✗
    /// （设计 §2.11 的硬不变式是**一律拒绝** ✓，不是"大多数拒绝" ✗）。
    pub(crate) fn contains_meta(&self, e: ExprPtr<'a>) -> bool {
        // `ExprPtr` 自己就能取到 `&Expr` ✓（`util::read_expr` 也是 `*p.as_ref()` ✓）。
        match e.as_ref() {
            Expr::Meta { .. } => true,
            Expr::App { fun, arg, .. } => self.contains_meta(*fun) || self.contains_meta(*arg),
            Expr::Pi { binder_type, body, .. } | Expr::Lambda { binder_type, body, .. } => {
                self.contains_meta(*binder_type) || self.contains_meta(*body)
            }
            Expr::Let { data, .. } => {
                self.contains_meta(data.binder_type)
                    || self.contains_meta(data.val)
                    || self.contains_meta(data.body)
            }
            Expr::Proj { structure, .. } => self.contains_meta(*structure),
            Expr::Var { .. }
            | Expr::Sort { .. }
            | Expr::Const { .. }
            | Expr::StringLit { .. }
            | Expr::NatLit { .. } => false,
        }
    }

    /// **IA-4 K1 硬不变式①**：这一条声明里有没有占位符 ✓。
    ///
    /// 覆盖面（**逐类走遍** ✓）：每个变体的 `info().ty` ✓ · `Theorem`/`Definition`/`Opaque`
    /// 的 `val` ✓ · **递归子的 `rec_rules[].val`** ✓（构造子的类型在它自己的 `info.ty` 里 ✓）。
    fn declar_contains_meta(&self, d: &Declar<'a>) -> bool {
        if self.contains_meta(d.info().ty) {
            return true;
        }
        if let Some(val) = d.value() {
            if self.contains_meta(val) {
                return true;
            }
        }
        if let Declar::Recursor(rec) = d {
            if rec.rec_rules.iter().any(|r| self.contains_meta(r.val)) {
                return true;
            }
        }
        false
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

    /// **R1c-2b（2026-10-05）**：`NamePtr` ⇒ 字符串 ✓（原来是私有 ✓）。
    ///
    /// ⚠ **为什么前端需要它** ✗：出口（(c)）要按声明**已有的宇宙参数名**建 `UnivMap`
    /// 才能把「表说已解出」的层文本（`u+1` 之类 ✓）解析回内核层 ✓ ——
    /// 判重/解析都要名字的**文本** ✓。**加法、零行为变化** ✓。
    pub fn name_to_string(&self, name: NamePtr<'a>) -> String {
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

    /// 造 `n` 条 `axiom a<i> : Sort 0`：名字走 intern ⇒ `Dag` 也一起长 ✓
    /// （否则只量到 `declars` 那一块，读不出总成本 ✗）。
    fn build_axioms<'a>(arena: &'a Arena, n: usize) -> EnvBuilder<'a> {
        let mut b = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        for i in 0..n {
            add_axiom(&mut b, &format!("a{i}"));
        }
        b
    }

    /// 加一条 `axiom <name> : Sort 0`（与既有用例同形 ✓）。
    fn add_axiom(b: &mut EnvBuilder<'_>, name: &str) {
        let name = b.name_from_str(name);
        let zero = b.zero();
        let ty = b.mk_sort(zero);
        let uparams = b.alloc_levels_slice(&[]);
        b.add_declar(Declar::Axiom {
            info: DeclarInfo { name, uparams, ty },
        })
        .expect("a `Sort 0` axiom must be accepted");
    }

    /// 克隆一份**命令边界快照** ⇒ `(总条目数, declars 桶, dag 桶, 其余桶)`。
    fn snapshot_clone_cost(b: &EnvBuilder<'_>) -> (u64, u64, u64, u64) {
        use crate::util::clone_stats;
        clone_stats::reset();
        // 两种形态都量：**取**快照（命令边界那一下）与**克隆**快照（T2-B 要留住它）。
        let snap = b.env_snapshot();
        let snap2 = snap.clone();
        let last = clone_stats::last();
        let (clones, declars, dag, tables) = clone_stats::take();
        assert_eq!(clones, 2, "取 + 克隆快照各记一笔账 ✗");
        drop((snap, snap2));
        (last, declars, dag, tables)
    }

    #[test]
    fn name_cache_discovers_nat() {
        let arena = Arena::new();
        let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let _ = builder.name_from_str("Nat");
        let env = builder.finish();
        assert!(env.name_cache.nat.is_some(), "Nat was not discovered by name cache");
    }

    /// **G-29 的指针同一性判据（两向 ✓ · 2026-10-07）** —— 「复用来的环境与被复用的
    /// 环境是**同一份**」。
    ///
    /// 为什么需要它：G-29 的真修法是**跨调用**复用库层的内核环境（设计
    /// `docs/design/incremental-environment.md` §33）。复用的**唯一合法性依据**是
    /// 环境**逐字段同一** —— 不是"内容相等"，是**同一份**。内核多处按**指针**比较
    /// （`conv.rs` 的 `NatLit` 指针相等、`eval.rs` 用地址做内容哈希、`NameInterner`
    /// 比 `StringPtr` 地址）⇒ 若复用路径交出来的是一份**重建**的环境，指针会变
    /// ⇒ **本该判过的 `def_eq` 假失败** ✗（与 `snapshot()` 的 ⚠ 同族）。
    ///
    /// **两向**：① 复用路径（同 arena · `hide_declars` → `restore_declars`，即
    /// `project/session.rs` 库层复用那条）⇒ **同一**；② 回退路径（**另一份 arena**
    /// 里重建同一个声明）⇒ **不同** ⇒ 那时**不许**把上一份环境的结论带过去，必须走
    /// 整份重编（= 今天的行为 ✓）。
    ///
    /// 判据只用**指针**（`ExprPtr` 的 `PartialEq` 比的是地址位 `util.rs:224`）⇒
    /// 机器无关、与墙钟无关 ✓。
    #[test]
    fn a_reused_environment_is_pointer_identical_and_a_rebuilt_one_is_not() {
        /// 在同一份 arena 里造一个 `witness : Sort 0` 公理。
        fn witness<'a>(arena: &'a Arena) -> (EnvBuilder<'a>, NamePtr<'a>) {
            let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
            let name = builder.name_from_str("witness");
            let zero = builder.zero();
            let ty = builder.mk_sort(zero);
            let uparams = builder.alloc_levels_slice(&[]);
            let declar = Declar::Axiom {
                info: DeclarInfo { name, uparams, ty },
            };
            builder
                .add_declar(declar)
                .expect("a `Sort 0` axiom must be accepted");
            (builder, name)
        }

        let arena = Arena::new();
        let (mut builder, name) = witness(&arena);
        let before = builder.declars.get(&name).cloned().expect("declared");

        // ① 复用路径：检查点搬出去、再搬回来（库层检查点就是这条）。
        let checkpoint = builder.hide_declars();
        builder.restore_declars(checkpoint);
        let after = builder.declars.get(&name).cloned().expect("restored");
        assert_eq!(
            before, after,
            "复用路径必须交出**同一份**环境（`ExprPtr` 的 `PartialEq` 比地址位 ✓）—— \
             不等 ⇒ 中间重建了一份 ⇒ 指针同一性已破 ✗"
        );
        // 只读快照走的是同一条同一性：`snapshot()` 克隆的是**指针**，不是项。
        let snap = builder.snapshot();
        assert_eq!(
            snap.declars.get(&name).cloned().expect("in snapshot"),
            before,
            "`snapshot()` 必须与原环境**指针同一**（它只克隆表、不重建项 ✓）"
        );

        // ② 回退路径：**另一份 arena** 里重建同一个声明 ⇒ 指针**不同**。
        let other = Arena::new();
        let (rebuilt_builder, rebuilt_name) = witness(&other);
        let rebuilt = rebuilt_builder
            .declars
            .get(&rebuilt_name)
            .cloned()
            .expect("declared");
        assert_ne!(
            before, rebuilt,
            "另一份 arena 里重建出来的环境**不是**同一份（地址不同）⇒ 那种环境**不许**\
             当作复用结果交出去，必须走回退（整份重编）✓"
        );
    }

    /// **T1-B 批 1 的判据**（2026-10-09）：[`EnvBuilder::from_export_file`] 装载出来的
    /// 环境必须是**同一份**（指针同一）—— 既不是"内容相等"，也不是"重建一份"。
    ///
    /// ## 为什么这条判据必须存在（不是形式主义）
    ///
    /// 磁盘产物（T1-B）的消费者要"对着**已 elaborate 的依赖环境**编自己那一层"。
    /// 内核多处按**指针**比较（`conv.rs` 的 `NatLit`、`eval.rs` 用地址做内容哈希、
    /// `NameInterner` 比 `StringPtr` 地址）⇒ 若装载路径交出来的是一份**重建**的表，
    /// 同一个名字/字面量会出现**第二个节点** ⇒ 本该判过的 `def_eq` 假失败 ✗
    /// （与本文件 `a_reused_environment_is_pointer_identical_and_a_rebuilt_one_is_not`
    /// 同一条不变式，只是那条测 `hide/restore`，本条测**跨 `finish` 的装载** ✓）。
    ///
    /// ## 反向验证（同一用例里两条）
    ///
    /// ⑤ 空环境里同一条声明必须**判不过**（否则 ④ 的 `Ok` 是空转 ✗）；
    /// ⑥ **重建**一份 `EnvBuilder`（新 `Dag`）之后同一个名字会被 intern 成
    /// **第二个** `NamePtr` ⇒ 判据②当场能分辨"搬过来"与"重建一份" ✓。
    #[test]
    fn from_export_file_carries_the_intern_tables_not_a_rebuilt_dag() {
        let arena = Arena::new();
        let mut builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        // 造一条 `axiom w : Sort 0`（= Prop），就是"已 elaborate 的库层"的最小形态。
        let w = builder.name_from_str("w");
        let zero = builder.zero();
        let ty = builder.mk_sort(zero);
        let uparams = builder.alloc_levels_slice(&[]);
        builder
            .add_declar(Declar::Axiom {
                info: DeclarInfo { name: w, uparams, ty },
            })
            .expect("a `Sort 0` axiom must be accepted");
        let w_declar = builder.declars.get(&w).cloned().expect("declared");
        // `finish()` = 产物（磁盘上是它的序列化形式；本轮先证"装载"这一半 ✓）。
        let file = builder.finish();

        let mut loaded = EnvBuilder::from_export_file(arena.as_arena_ref(), file);
        // ① 声明整份搬过来，且与产物里那份**指针同一** ✓。
        assert_eq!(loaded.declaration_count(), 1, "装载必须把声明整份搬过来");
        assert_eq!(
            loaded.declars.get(&w).cloned().expect("w in loaded"),
            w_declar,
            "装载只搬字段、不重建任何东西 ⇒ 必须与产物里那份**指针同一** ✗"
        );
        // ② **intern 表也搬过来了**（不是新 `Dag`）：同一个名字再 intern ⇒ 同一个指针。
        let w_again = loaded.name_from_str("w");
        assert_eq!(
            w_again, w,
            "装载换了新的 `Dag` ⇒ 同一个名字会造出**第二个** `Name` 节点 ✗ \
             （指针同一性已破 ⇒ 下游 `def_eq` 会假失败）"
        );
        // ③ 装载后能照常 `add_declar`：造一条引用已装载 `w` 的 `u : w`。
        let u = loaded.name_from_str("u");
        let levels = loaded.alloc_levels_slice(&[]);
        let u_ty = loaded.mk_const(w, levels);
        let u_uparams = loaded.alloc_levels_slice(&[]);
        let u_declar = Declar::Axiom {
            info: DeclarInfo { name: u, uparams: u_uparams, ty: u_ty },
        };
        loaded
            .add_declar(u_declar.clone())
            .expect("loading must not break `add_declar`");
        assert_eq!(loaded.declaration_count(), 2);

        // ④ **内核真判**：`u : w` 在**装载来的**环境里判得过（`w` 在 idx 0 ⇒ `ByIndex(1)`）。
        let env_loaded = loaded.finish();
        assert!(
            env_loaded
                .try_check_declar_at(&u_declar, crate::env::EnvLimit::ByIndex(1))
                .is_ok(),
            "装载后 `u : w` 必须引用得到已装载的 `w`（内核判过 ✓）"
        );
        // ⑤ **反向验证**：同一份声明在**空环境**里**判不过** ⇒ ④ 不是空转 ✓。
        let env_empty = EnvBuilder::new(arena.as_arena_ref(), Config::default()).finish();
        assert!(
            env_empty
                .try_check_declar_at(&u_declar, crate::env::EnvLimit::ByIndex(0))
                .is_err(),
            "反向验证：空环境里 `u : w` 必须被判拒（`w` 不在环境里）—— \
             它若也过 ⇒ ④ 的 `Ok` 是空转 ✗"
        );
        // ⑥ **反向验证（指针侧）**：**重建**一份 builder（新 `Dag`）⇒ 同一个名字是
        //    **另一个** `NamePtr` ⇒ 判据②真的分得开"搬过来"与"重建一份" ✓。
        let mut rebuilt = EnvBuilder::new(arena.as_arena_ref(), Config::default());
        let w_rebuilt = rebuilt.name_from_str("w");
        assert_ne!(
            w_rebuilt, w,
            "重建的 `Dag` 必须为同一个名字造出**第二个**节点 ⇒ 判据②有牙 ✓"
        );
    }

    /// **T2-A 判据①**：「在**命令边界**取一份环境」（[`EnvSnapshot`]）要复制多少条目？
    ///
    /// ## 判据
    ///
    /// **与 #decls 无关**：`EnvSnapshot::clone` 复制的条目数在 64 条与 512 条声明上必须
    /// **相同**、且是一个**小常数**（对齐 Lean `SMap`：库层平坦表经共享、本地层持久化
    /// ⇒ 克隆只动根指针，`src/Lean/Data/SMap.lean:17-27`）。
    ///
    /// ## 为什么是"两臂相等"而不是"一个绝对数"
    ///
    /// 绝对数依赖实现细节 ⇒ 会变成"改一下就调阈值"的假判据 ✗。
    /// **两臂相等**咬住的才是真东西：**伸缩性**（O(#decls) ⇒ O(1)）✓。
    /// `AGENTS.md` 判据纪律②：结构计数、机器无关 ✓。
    ///
    /// ⚠ **先建先红**（2026-10-09 已实测 ✓）：T2-A 之前，同一把尺子量
    /// `EnvBuilder::clone` 读出 **64 条 = 196 条目 / 512 条 = 1540 条目**（`declars` 512 +
    /// `Dag` 1028）⇒ 本条**判红** ✓；分层落地后翻绿 ✓（**不许**放宽它）。
    #[test]
    fn env_snapshot_clone_cost_is_constant_in_declaration_count() {
        let arena = Arena::new();
        let small = build_axioms(&arena, 64);
        let big = build_axioms(&arena, 512);

        let (small_total, small_d, small_g, small_t) = snapshot_clone_cost(&small);
        let (big_total, big_d, big_g, big_t) = snapshot_clone_cost(&big);

        assert_eq!(
            small_total, big_total,
            "命令边界快照的克隆成本**随 #decls 增长** ✗（64 条 = {small_total} 条目              [declars {small_d} · dag {small_g} · 其余 {small_t}]，             512 条 = {big_total} 条目 [declars {big_d} · dag {big_g} · 其余 {big_t}]）⇒              命令级快照在数据结构上是 O(N²)（判据①要的是 O(1)）"
        );
        assert!(
            big_total <= 8,
            "快照克隆复制的条目数是 {big_total}（512 条声明）—— 判据①要求它是一个**小常数**             （O(1)：只动共享根指针）✗ [declars {big_d} · dag {big_g} · 其余 {big_t}]"
        );
        // **结构面**：`record(0,0,0)` 必须是**事实**，不是声明 ⇒ 快照与 builder
        // **共享同一批表**（`Arc::ptr_eq`）✓。谁把共享换成复制，这条当场判红 ✓。
        assert!(
            big.env_snapshot().shares_tables_with(&big),
            "快照与 builder 必须**共享同一批表**（`Arc::ptr_eq`）—— 不共享 ⇒              上面那个 0 是假的 ✗"
        );
    }

    /// **T2-A 判据①的诚实面**：封层之后，快照的代价只是**推迟**到"下一次插入"那一下
    /// （`Arc::make_mut` 的写时复制）。这条咬住的是：那次复制**只许与本地层
    /// （入口文件自己的声明数）成比例**，**绝不许**与库层 (#base) 成比例 ✗。
    ///
    /// 为什么必须单独有它：判据① 读到的 0 有两种来源 —— ①**真共享** ✓；
    /// ②**把成本藏到别处** ✗。这条排掉第二种 ✓。
    #[test]
    fn sealing_keeps_the_library_layer_out_of_the_copy_on_write() {
        use crate::util::clone_stats;

        let arena = Arena::new();
        // 库层：512 条；封层；再取一份快照 —— 此后每次插入都会命中 COW。
        let mut b = build_axioms(&arena, 512);
        b.seal_library_layer();
        assert!(b.library_layer_sealed());
        assert_eq!(b.declaration_count(), 512, "夹具前提：库层 512 条");
        let _snap = b.env_snapshot();

        // 入口层：加 3 条（每条都跟在"快照共享"之后 ⇒ 第一次插入触发一次 COW）。
        clone_stats::reset();
        for i in 0..3 {
            add_axiom(&mut b, &format!("entry{i}"));
        }
        let (copies, entries) = clone_stats::take_cow();
        assert_eq!(copies, 1, "封层后**只该**复制一次本地层（第一次插入），实测 {copies} 次 ✗");
        assert!(
            entries < 512,
            "写时复制搬了 {entries} 条 —— 它与**库层**（512 条）同量级 ⇒              共享没生效（封层没做 / 插入漏进了 `base`）✗"
        );

        // **反向验证**：不封层 ⇒ 插入命中共享的 `base` ⇒ 复制量 = 整条库层 ✓。
        let mut unsealed = build_axioms(&arena, 512);
        let _snap = unsealed.env_snapshot();
        clone_stats::reset();
        add_axiom(&mut unsealed, "leak");
        let (copies2, entries2) = clone_stats::take_cow();
        assert_eq!(copies2, 1, "不封层也要复制一次（且是整条库层）");
        assert!(
            entries2 >= 512,
            "反向验证失败：不封层时那一次复制本该搬**整条库层**（≥512 条），实测 {entries2} ⇒              上一条的 `< 512` 咬不住东西 ✗"
        );
    }

    /// **T2-A 判据⑤ + 单调 intern 引理**：快照**不含** `Dag` 的合法性依据 ——
    /// 「拿更新的 intern 表重放旧命令，指针与首次运行**逐字节相同**」。
    ///
    /// 两向：
    /// ① 快照**之前** intern 的名字，装回之后仍**同一**指针 ✓；
    /// ② 快照**之后**才 intern 的名字也**同一**（表是超集、不重建）✓。
    /// ⇒ 一趟只有**一份** intern 表（判据⑤），快照不必捕获 `Dag` ✓。
    #[test]
    fn snapshot_restore_keeps_the_single_intern_table_pointer_identical() {
        let arena = Arena::new();
        let mut b = build_axioms(&arena, 8);
        b.seal_library_layer();
        let before = b.name_from_str("stable");
        let snap = b.env_snapshot();
        let after_snapshot = b.name_from_str("later");
        b.restore_env(&snap);
        // 一次 `restore_env` 不许换表 ⇒ `Dag` 之外什么都没动 ✓。
        assert_eq!(
            b.name_from_str("stable"),
            before,
            "装回快照后重新 intern 同一个名字给出了**另一个** `NamePtr` ⇒              `restore_env` 动了 intern 表（本该一个字节都不动）✗"
        );
        assert_eq!(
            b.name_from_str("later"),
            after_snapshot,
            "快照**之后**才 intern 的名字在装回后变了指针 ⇒ 表被回滚了 ✗             （单调 intern 引理要求：表只增不删，快照是它的子集 ✓）"
        );
    }

}
