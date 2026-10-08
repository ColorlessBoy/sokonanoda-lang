//! 命令走查（`run_pass` 的 `parse → elab → check-then-add` 前两段）：把每个
//! `Command` elaborate 成 `PendingOp` / `DeclState` / 错误，**不**调用内核。
//!
//! 批次 3 第三刀：从 `check/mod.rs` 的 `run_pass` 主循环整体切出，每个
//! `Command` 变体一个方法，方法体就是原来的 match arm——只动位置不动语义：
//! 外层局部变量变成 `Walk` 的字段（可变累加器）、`run` 的参数（只读上下文）
//! 或 `CmdCtx` 的字段（每个命令派生一次的前缀/模板/信任位）；arm 里的
//! `continue` 改 `return`（每个 arm 都没有内层循环，语义等价）。
//!
//! 单文件模式下 `prefix_src` 仍是**借用**本文件前缀（`Cow::Borrowed`），
//! 不产生额外分配：A1（无 `import` 的文件逐字节不变）不受影响。

use super::{
    by_step_states, failed_state, lower_value, skipped, CmdHover, KernelFailed, PendingOp,
    TrustPlan,
};
use crate::compile::elab::{
    build_axiom, build_def, build_example, build_theorem, elab_expr, install_inductive_block,
    make_univ_map, params_of_ty, resolve_known, strip_lambdas_n, telescope_arity_of_ty, DefInfo,
    ElabCtx, ElabScope, HoverNode, InductiveTable, KnownName, KnownTable, UnivMap,
};
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::event::CompileOutput;
use crate::compile::goals::{expr_has_hole, open_goal, spine_without_arg, GoalTemplates};
use crate::compile::prelude::CompileOptions;
use crate::compile::report::{ByGoalState, DeclKind, DeclState, GoalBinder};
use crate::compile::scope::{NamespaceScope, OpenEntry};
use crate::compile::units::SourceUnit;
use crate::{Binder, Command, CtorDecl, Expr, IotaRule, OpenFilter, RecDecl, Span};
use sokonanoda::builder::EnvBuilder;

/// **声明头部的根状态**：**只剥源位 λ 链的绑元**（= 声明里写的**具名绑元**），
/// **不是**整条 ∀ 望远镜 ✗。
///
/// 为什么（**2026-10-02 值守第 9 单**，用户实测报的 ✗）：前端 lowering 会给
/// **函数型语句**（`a → b → a` / `∀ x, …`）补 λ ⇒ `split_by_value(val)` 的 λ 链
/// **含语句自身的 Π 层** ⇒ 按它剥 `peel_pi_layers(ty, n)` 会**多剥**（目标的箭头全没、
/// 上下文多出匿名 `_`）；层数不够时 `peel_pi_layers` 返 `None` ⇒ `by_root = None` ⇒
/// 退回 `ty_text` + 空 binders ⇒ 顶显示整句 `∀` ✗。
/// 实测：`theorem (a b : Prop) : a → b → a := by` ⇒ 顶 `[a,b,_,_] ⊢ a` ✗
/// vs 底（声明卡片）`[a,b] ⊢ a → b → a` ✓。
///
/// 正确口径 = **与声明卡片同源**：open 声明在调用点直接用 `open_goal` 的 `info`
/// （顶 ≡ 底 **by construction** ✓）；其余（失败 / 已证）用这里的 λ 前缀版 ✓。
///
/// **题面状态**（G-82 兜底，2026-10-03）：声明**源位 λ 链的具名绑元** + 按它们
/// 剥出的剩余目标 —— 未折记法的**真相文本**（喂 judge 的那一份 ✓）。
///
/// 为什么要有这一条：`open_goal` 分解不了值（记法头、超量应用、def 展开间接调用…）
/// 时，`walk.rs` 原先退回 **空上下文 + 整句声明类型** ✗ —— 有具名绑元的题就变成
/// 「题目参数忽然跑到目标里去了」，整句判红 ✗（用户 2026-10-02 实测报的 G-82）。
/// 走查分解不了**不等于**题面没有上下文：题面状态**永远**是"具名绑元进上下文、
/// 目标只剩剥掉它们之后的命题" ✓。
///
/// 与 [`decl_prefix_state`] **同源**（那个 = 这条 + 折一次记法）⇒ 顶 ≡ 底
/// 在这条兜底路径上也成立 ✓。`val` 必须是**源位**（lowering 之前）的值：lowering
/// 会给函数型语句补 λ，拿 lowering 之后的值数 λ 链会多剥 ✗。
fn decl_root_state(
    ty: &Expr,
    src_val: &Expr,
    display: &crate::display::DisplayNotations,
) -> Option<(String, Vec<GoalBinder>)> {
    let (binders, _) = crate::by::split_by_value(src_val)?;
    let body = crate::proof::peel_pi_layers(ty, binders.len())?;
    // **走唯一显示接口** ✓（`scripts/audit-notation-paths.py` 的棘轮 ✓）：绕过它 ⇒
    // Infoview 顶部目标退回**点形式** ✗ —— 正是 G-81/G-83 那一族的病根 ✓。
    // `fold` 幂等 ✓ ⇒ 下面 `decl_prefix_state` 再折一次不改变结果 ✓（那两处照旧逐字节不变 ✓）。
    Some((
        display.render(&body),
        binders
            .iter()
            .map(|b| GoalBinder {
                name: b.name.clone(),
                ty: b
                    .ty
                    .as_deref()
                    .map(|t| display.render(t))
                    .unwrap_or_default(),
            })
            .collect(),
    ))
}

/// 题面状态的**显示副本**（折一次记法）：`by_root` / hover 直接渲染它 ✓。
fn fold_root_state(
    state: &(String, Vec<GoalBinder>),
    display: &crate::display::DisplayNotations,
) -> ByGoalState {
    let (goal, binders) = state;
    ByGoalState {
        ty: display.fold(goal),
        binders: binders
            .iter()
            .map(|b| GoalBinder {
                name: b.name.clone(),
                ty: display.fold(&b.ty),
            })
            .collect(),
    }
}

/// 同 [`decl_root_state`]，但**折一次记法**（显示副本）—— 失败 / 已证声明用。
fn decl_prefix_state(
    ty: &Expr,
    val: &Expr,
    display: &crate::display::DisplayNotations,
) -> Option<ByGoalState> {
    decl_root_state(ty, val, display)
        .as_ref()
        .map(|state| fold_root_state(state, display))
}
use sokonanoda::env::Declar;
use sokonanoda::util::ExprPtr;
use std::borrow::Cow;

/// 命令走查的**可变累加器**（原 `run_pass` 主循环里被 arm 改写的局部变量）。
pub(super) struct Walk<'arena: 'shadow, 'shadow> {
    /// **显示期的记法表**（线 C）：整趟建一次，给 `ty_text` 与 `by` 步进的
    /// 展示副本共用（`check/mod.rs` 的 `display_notations`）。
    pub(super) display: crate::display::DisplayNotations,
    pub(super) builder: EnvBuilder<'arena>,
    /// **影子环境**（T-K12b）：一份**只读给 judge 用**的环境，内容是"到目前为
    /// 止已经 elaborate 且**已通过内核检查**的前缀"。它从 `self.ops` **惰性重放**
    /// （[`Walk::shadow_env`]），检查序列**逐条镜像** `kernel_phase` 的主路径
    /// （主声明 `try_check_declar`＝`ByName` 形式、归纳块逐成员检查），
    /// 失败的不进环境、记进 [`Walk::shadow_failed`]。
    ///
    /// 为什么不直接用 `builder`：**两者语义不同** ✗ —— `builder` 是**活环境**
    /// （walk **无条件** `add_declar`，到当前命令为止的声明**尚未过内核检查**；
    /// P1-a 的就地判定 `InplaceEnv` 用的正是它 ✓），而 `shadow` 只收**已通过内核检查**
    /// 的前缀（逐条镜像 `kernel_phase` 的检查序列 ⇒ T-D3 的对照数据）⇒ 混用会改判定 ✗。
    ///
    /// ⚠⚠ **旧注释（2026-10-05 更正 ✗→✓）**：原文写「`builder` 最终要被 `kernel_phase`
    /// 的 `finish()` **消费**，而且 walk 阶段**不往里 add** 文件声明 ✗（它只装 prelude +
    /// intern 名字）」—— **两句都不成立** ✗：`kernel_phase` 收的是 `&mut EnvBuilder`
    /// 且只用 `with_env` 借出（**不消费** ✓），walk 也确实往里 add（**9 处**，
    /// 其中 8 处无条件 ✓）。这正是设计 `docs/design/incremental-environment.md` §8 的
    /// 那个误读（**§8.1 已更正 ✓**）⇒ **别再照旧注释推断所有权** ✗。
    pub(super) shadow: Option<EnvBuilder<'shadow>>,
    /// 影子环境已重放到 `ops` 的哪个下标。
    pub(super) shadow_upto: usize,
    /// 影子重放中**内核拒绝**的那些 `ops` 下标（与 `kernel_phase` 的失败表同键：
    /// 都按"命令序"索引 ✓）。
    pub(super) shadow_failed: Vec<usize>,
    /// 同上，附带**内核给的错误原文**（观测用：知道"哪条失败"还不够，要知道
    /// **为什么** —— 例如 "unknown const" ⇒ 缺依赖、"def_eq mismatch" ⇒ 值不对）。
    pub(super) shadow_failed_msg: Vec<(usize, String)>,
    /// 本轮 pass 的**已知失败集**（`run_pass` 的 `skip` 参数）。
    ///
    /// 影子必须**同样跳过**这些命令 ✗：`kernel_phase` 对它们**不重查**、
    /// 因此也不会记进 `failed_cmds` ✓；影子若无条件重查，失败表就会比内核**多**
    /// 几条 ✗（T-K12b 的对照实测正是这样：`unit01` 内核 0 失败、影子 3 失败 ✓）。
    /// 存**克隆**（几条而已 ✓），因为 `skip` 的寿命是 `run` 的 `'src` ✗。
    pub(super) shadow_skip: Option<KernelFailed>,
    pub(super) known: KnownTable,
    pub(super) inductives: InductiveTable<'arena>,
    /// 源级 `def` 表（课程 Lean 化）：跨单元累加，`by` 引擎做一层 delta 展开用。
    pub(super) defs: crate::compile::elab::DefTable,
    pub(super) out: CompileOutput,
    pub(super) ops: Vec<PendingOp<'arena>>,
    pub(super) cmd_hovers: Vec<CmdHover<'arena>>,
    pub(super) decl_states: Vec<DeclState>,
    /// `example` 的内部名计数器（`_example_N`，按出现次序）。
    pub(super) example_idx: usize,
    /// G-05：命名空间栈 + `open` 集合。按源码顺序推进（`namespace`/`end`/`open`
    /// 三条命令的臂），单元切换处 [`NamespaceScope::reset`]——`open` 是**文件**
    /// 作用域，不跨 `import`（设计 N5）；`namespace` 由 parser 校验闭合，所以
    /// 单元边界上栈必然为空。
    pub(super) ns: NamespaceScope,
    /// **跨单元导出表**（第二刀 §N7）：`export Foo` 记在这里，单元切换时重放。
    /// 依赖按拓扑序排在入口之前，所以入口文件在文件头就能用依赖导出的短名。
    pub(super) exports: Vec<OpenEntry>,
    /// **本趟 pass「成功进环境」的声明名**（G-31/G-92 第二刀 ✓，2026-10-07）。
    ///
    /// 逐命令压进 `TRUSTED_PREFIX` 栈项（`run` 里的 `with_trusted_prefix` ✓）
    /// ⇒ judge 的**合成判定文档**读得到"调用方那趟**确实加过**哪些 `theorem`"
    /// ⇒ 内层只 elaborate 类型、按**不透明常量**加（证明体不重跑 ✓）。
    /// 名字 = **声明身份**（不用命令号：两套坐标系实测**不对齐** ✗，见
    /// `judge::EnteredNames` 的注释 ✓）。
    pub(super) entered: crate::judge::EnteredNames,
    /// **调用方那一趟**的名表（`TrustPlan::trusted_entered` ✓）：`Some` 时才允许
    /// 走不透明快路；`None`（**绝大多数 pass** ✓）⇒ **逐字节回到今天** ✓。
    pub(super) trusted_entered: Option<crate::judge::EnteredNames>,
    /// **G-29 第 3 棒**：本趟的 `idx` 相对 **judge 合成文档前缀坐标系**的平移量。
    ///
    /// judge 合成文档的前缀 = `closure_prefixes[unit_idx]` + 本文件前缀 = **整条闭包**
    /// （`session` 的入口趟显式传库层源码 ✓），而**入口趟**只走入口自己的命令 ⇒
    /// 它的 `idx` 是**入口空间**的 ✗ ⇒ `judge::synthesized_trust` 的闸门
    /// `before >= prefix_commands` **恒不成立** ⇒ 每次 `judge_infer` 未命中都回退
    /// `compile_fol_with`（整份前缀从零编 ✗）。平移量 = **库层那一段的命令数** ✓
    /// ⇒ 担保回到闭包空间 ✓。默认 `0` = 同坐标系（老路 / 单文件 / 库层趟 ✓）。
    pub(super) judge_prefix_offset: usize,
    /// **T1-A（2026-10-09）**：**接着哪个模块边界继续编**（`None` = 从头 ✓）。
    /// 见 [`crate::compile::ResumeState`]：`closure_id`/`exports`/`example_idx`
    /// 三样都是**跨单元累加器** ⇒ 逐模块编译时必须显式带上 ✗→✓。
    pub(super) resume: Option<crate::compile::ResumeState>,
    /// **T1-A**：本趟结束时把 [`Walk::resume_out`] 算出来交回调用方。
    pub(super) snapshot_state: bool,
    /// 见 [`Walk::snapshot_state`]：本趟结束（= 模块边界）上的续编状态。
    pub(super) resume_out: Option<crate::compile::ResumeState>,
}

/// 单个命令的派生上下文：每个命令算一次，arm 里按需取用。
struct CmdCtx<'a> {
    idx: usize,
    templates: &'a GoalTemplates,
    /// 合成前缀源码：单文件 = 本文件前缀（借用）；闭包 = 依赖声明 + 本文件前缀。
    prefix_src: Cow<'a, str>,
    /// `[0, before)` 的命令已在上一会话判过：只 elaborate，不判。
    trusted: bool,
    env_before: usize,
    options: &'a CompileOptions,
    skip: Option<&'a KernelFailed>,
    /// G-05：**本单元**用了 `namespace`/`open` ⇒ `by` 引擎的根目标先过一遍
    /// 内核 pp（`docs/design/namespace-open.md` §4.6）。没碰命名空间的文件
    /// 零额外开销、行为逐字不变。
    canonical_goal: bool,
    /// 本单元源码（`open … in …` 的合成前缀要按**源码文本**补一行 open，
    /// 见 [`Walk::open_in`]）。
    src: &'a str,
}

/// 把一个引用重借成**局部寿命**。
///
/// 为什么需要：`CmdCtx` 里的字段是 `&'x T`（`'x` 是 `c` 的寿命参数）。直接拷出来
/// 会让 `ElabCtx<'arena, 'b>` 的 `'b` 被统一到 `'x`，于是 `'arena: 'x` 变成方法
/// 签名上的义务——那是调用方（甚至 `run`）无法证明的。过一次这个恒等函数，寿命
/// 就回到方法体内的局部推断变量，`'arena: 'b` 自然成立。
#[inline]
fn local<T: ?Sized>(r: &T) -> &T {
    r
}

impl<'arena: 'shadow, 'shadow> Walk<'arena, 'shadow> {
    /// **把影子环境推进到"当前已 elaborate 的前缀"**（T-K12b）。
    ///
    /// 惰性：只在第一次（以及每次有新 `ops` 之后）被调用时才重放新增的那几条
    /// ⇒ **不用就零成本** ✓。重放的检查序列**逐条镜像** `kernel_phase`：
    /// 主声明走 `try_check_declar`（`ByName` 形式，同 `kernel_phase.rs:251`），
    /// 归纳块逐成员检查（同 `kernel_phase.rs:315`）；**内核拒绝的不进环境**
    /// （check-then-add 语义 ✓），名字记进 `shadow_failed`。
    pub(super) fn shadow_env(&mut self) -> Option<&mut EnvBuilder<'shadow>> {
        self.shadow.as_ref()?;
        while self.shadow_upto < self.ops.len() {
            // 失败表按 **`cmd`（命令下标）** 记 —— 与 `kernel_phase` 的
            // `failed_cmds: KernelFailed` **同键**，这样两张表能逐条对照 ✓。
            //
            // **已知失败的命令整条跳过**（镜像 `kernel_phase` 的 skip 语义 ✓）：
            // 内核不重查它们 ⇒ 也不记进失败表；影子照做，两张表才对得上 ✓。
            let skipped = match &self.ops[self.shadow_upto] {
                PendingOp::Decl { cmd, .. } | PendingOp::InductiveBlock { cmd, .. } => self
                    .shadow_skip
                    .as_ref()
                    .is_some_and(|s| s.contains_key(cmd)),
                _ => false,
            };
            if skipped {
                self.shadow_upto += 1;
                continue;
            }
            match &self.ops[self.shadow_upto] {
                PendingOp::Decl { declar, cmd, .. } => {
                    let (declar, cmd) = (declar.clone(), *cmd);
                    let _ = self.shadow_check_and_add(&declar, cmd);
                }
                PendingOp::InductiveBlock { declars, cmd, .. } => {
                    // **逐条镜像 `kernel_phase` 的归纳块语义**（`kernel_phase.rs:313-325`）：
                    // 逐成员 check-then-add，**首个失败就 `break`** ✗ —— 后面的成员
                    // **不再进环境** ✓。我原来"失败也继续" ⇒ 环境状态就此分叉 ✗
                    // ⇒ 之后依赖它们的声明会拿到不同的 `decl_idx` ⇒ 假 `def_eq mismatch` ✓
                    // （R0 的根因假设，2026-09-24 第 95 轮）。
                    let (declars, cmd) = (declars.clone(), *cmd);
                    for declar in declars {
                        if !self.shadow_check_and_add(&declar, cmd) {
                            break;
                        }
                    }
                }
                _ => {}
            }
            self.shadow_upto += 1;
        }
        self.shadow.as_mut()
    }

    /// **D-2 的 A 步开关**（2026-09-25 round 260）：让 walk 的 check-then-add
    /// **同时**落到真 `builder`（而不只落到 `self.shadow`）✓。
    ///
    /// 为什么可以这么做（round 259 读传递链确认 ✓）：walk 运行期间**持有** `builder`
    /// —— `run_pass_with` 把 builder **按值**交给 walk、跑完再**交回**调用方 ✓
    /// （`session.rs` 因此能跨入口复用**同一套 DAG** ✓）；`kernel_phase` 收的是
    /// `&mut EnvBuilder` 且只用 `with_env` 借出 ⇒ **不消费** ✓
    /// ⇒ **没有任何结构性障碍** ✓（round 258 曾误判为"要动所有权设计" ✗）。
    /// ⚠ **2026-10-05 更正** ✗→✓：原文这里写「`kernel_phase` 才 `finish()` **消费**」✗
    /// —— 编译路径上**没有** `EnvBuilder::finish()`（`crates/front/src/` 里唯一一处
    /// `.finish()` 是 `judge.rs` 的 `Hasher::finish`；`EnvBuilder::finish` 只出现在
    /// kernel 测试里）⇒ 别再照它推断所有权；真正的形状见设计
    /// `docs/design/incremental-environment.md` §32.3 ✓。
    ///
    /// 为什么**同时**写两边而不是只写真 `builder` ✓：`self.shadow` 是 T-D3 的
    /// **对照实验**（影子与内核阶段的一致性 ✓，`SOKO_SHADOW_STRICT` 可复现 ✓）
    /// ⇒ 只写一边会让那份对照数据消失 ✗。两边都写 ⇒ 实验照旧 ✓、真环境也开始被填 ✓。
    ///
    /// **默认关** ✓（阶段 D 的护栏：先开关后默认 ✓）；要复现/推进 D-2 时打开 ✓。
    /// **T-D8（D-2 的前缀复用）的处置 —— 2026-09-25 round 310 量清 ✓**
    ///
    /// **这个开关默认关** ✓，而且**没有可测收益** ✗ —— 这不是"没做完"✗，是
    /// **前提过期** ✗：计划的措辞写在 **R-3（`.sokonanoda/compiled/`）之前** ✓，
    /// 而 R-3 已经把"热编译"这件事做完了 ✓（小项目冷/热 **0.08s → 0.04s** ✓；
    /// 整门课 `soko course` **压根不缓存** ✓ 且只要 **0.38s** ✓）。
    ///
    /// **但它不是废码** ✓：判据**超额达成** ✓（开关态 failed **100 → 0** ✗✓、
    /// 组合态 **= 基线 11** ✓）、三层回归全过 ✓（kernel `tests/` ✓ · front 736/0 ✓ ·
    /// CLI `--json` **逐字节相同** ✓），而且**它动过的那条热路径**
    /// （`did_open_same_session`，**134ms** ✓）现在由 CI 的 **`perf-gate`** 守着 ✓
    /// —— **收益量不出，但"以后慢下来会被发现"** ✓。
    ///
    /// **回退方式** ✓：删掉本函数 + 三处 `Self::walk_real_add_enabled()` 的 `if` 分支
    /// （`walk.rs` 的 `build_redundant_probes` 调用点 ✓），并把内核的
    /// `hide_declars` / `restore_declars`（`crates/kernel/src/builder.rs` ✓）一并删除 ✓
    /// —— **默认路径不调用它们** ✓ ⇒ 删掉后判定行为**零变化** ✓。
    ///
    /// **复现（判它是否还值得留）** ✓：
    /// `SOKO_WALK_REAL_ADD=1 cargo test -q -p sokonanoda-front --lib` ✓
    /// ⇒ 期望 **736 passed / 0 failed** ✓（关掉时同样 736/0 ✓）。
    fn walk_real_add_enabled() -> bool {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| std::env::var("SOKO_WALK_REAL_ADD").is_ok_and(|v| v != "0"))
    }

    /// **R1c-2b（2026-10-05）**：**声明出口** —— 层元变量收口 ✓
    /// （`ty`/`val` 里的 mvar ⇒ `param` ✓、新名并进 `uparams` ✓）。
    ///
    /// **对齐 Lean** ✓：`Elab/Declaration.lean:118-127` 是「`type ← levelMVarToParam type`
    /// ⇒ `levelParams := sortDeclLevelParams …` ⇒ **然后才** `addDecl decl`」✓。
    ///
    /// ⚠ **为什么必须在 walk（`add_declar` **之前**）** ✗（白纸黑字）：本仓库的声明是
    /// walk **当场** `add_declar`（切片 1b ✓ —— 内核阶段只**读**环境 ✓）⇒ 晚一步的话
    /// 环境里那份还带 mvar ✗ ⇒ 后面的声明引用它时 `all_uparams_defined`
    /// （`infer.rs:103/114`）判拒 ✗。Lean 的形状也正是「先转、后 `addDecl`」✓。
    ///
    /// 返回值第二项 = **新加的宇宙参数名** ✓（调用方并进 `DeclState.universe` ✓）。
    fn discharge_level_mvars(&mut self, decl: Declar<'arena>) -> (Declar<'arena>, Vec<String>) {
        crate::compile::check::level_exit::discharge_declar(&mut self.builder, decl)
    }

    /// 影子环境的一条"检查后加入"（check-then-add，与 `kernel_phase` 同序同语义）。
    /// 检查走 `ExportFile`（`try_check_declar` 是它的方法）⇒ 借 `with_env` 一次；
    /// **内核拒绝的不进环境** ✓，只记下标。
    /// 返回"是否通过"（归纳块要在首个失败处 `break` ✓）。
    fn shadow_check_and_add(&mut self, declar: &Declar<'arena>, cmd: usize) -> bool {
        if self.shadow.is_none() {
            return true; // 影子未启用（session 路径）⇒ 不记录、不建环境
        }
        let declar = declar.clone();
        let result = self
            .shadow
            .as_mut()
            .unwrap()
            .with_env(|env| env.try_check_declar(&declar));
        match result {
            Ok(()) => {
                // **A 步**（round 260）：开关下**同时**落到真 `builder` ✓ ——
                // 真环境由此开始被 walk 填满，B 步（内核阶段跳过已覆盖的 cmd）
                // 才有依据 ✓。`declar` 已被 clone 一次 ⇒ 再 clone 一次给第二个环境 ✓。
                if Self::walk_real_add_enabled() {
                    let _ = self.builder.add_declar(declar.clone());
                }
                let _ = self.shadow.as_mut().unwrap().add_declar(declar);
                true
            }
            Err(e) => {
                self.shadow_failed.push(cmd);
                self.shadow_failed_msg.push((cmd, format!("{e:?}")));
                false
            }
        }
    }

    /// 扁平命令序走查。`flat` 是 `(单元下标, 命令)`，单文件时只有一个单元。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn run<'src>(
        &mut self,
        units: &[SourceUnit<'src>],
        flat: &[(usize, &'src Command)],
        options: &CompileOptions,
        skip: Option<&KernelFailed>,
        trust: Option<&TrustPlan>,
        all_templates: &[GoalTemplates],
        closure_prefixes: &[String],
        mut progress: Option<&mut dyn crate::compile::ProgressSink>,
    ) {
        // 影子重放要**同样跳过**本轮已知失败的命令（见 `shadow_skip` 的注释）。
        self.shadow_skip = skip.cloned();
        // G-05：每个单元是否用了 namespace/open（`by` 引擎的根目标规范化开关，
        // 每单元算一次；没用到的文件零开销）。第二刀：`open … in` 与 `export`
        // 同样会改引用解析，所以一并计入。
        let unit_uses_namespaces: Vec<bool> = units
            .iter()
            .map(|unit| {
                unit.file.commands.iter().any(|command| {
                    matches!(
                        command,
                        Command::Namespace { .. }
                            | Command::End { .. }
                            | Command::Open { .. }
                            | Command::OpenIn { .. }
                            | Command::Export { .. }
                    )
                })
            })
            .collect();
        // **P2 进度粒度**：每单元的命令（声明）总数，供回调报 `k/n`。
        let unit_totals: Vec<usize> = {
            let mut totals = vec![0usize; units.len()];
            for (unit_idx, _) in flat {
                totals[*unit_idx] += 1;
            }
            totals
        };
        let mut unit_seen: Vec<usize> = vec![0usize; units.len()];
        // **增量身份** ✓（值守 2026-10-04 派单 · 用户 13:08「不许降级修」✓）：
        // 判定缓存的键要的是**环境身份** ✓，而 `own_prefix` 是**逐命令增长**的 ✗
        // ⇒ 每命令都重解析整份前缀 = **O(n²)** ✗（unit12 实测 +2.3s ✗，那正是我先前
        // 往里塞尺寸闸的原因 ✗）。这里**边读边累加** ✓：闭包部分每个单元算**一次** ✓；
        // 本文件部分逐命令追加**那一条命令**的贡献 ✓（摊还 O(1) ✓）。
        //
        // ⚠ **身份必须从 AST 直取，不许"切片段再 parse"** ✗→✓（2026-10-04 **实测定位** ✓）：
        // 先前是「按文本切出新增片段 ⇒ `canonical_prefix_id_checked(片段)` ⇒ 不成则退回整体」✗。
        // 片段**用了依赖声明的记法**时（G-04 第二刀：`Aᶜ` / `∈`）**必然解析不过** ✗
        // （继承记法表没跟着走 ✓），整体**同样**不过 ✗ ⇒ 一路退到**原文** ✗
        // —— 实测课程 `unit08` **27 处** ✗（`fallbacks` 读数 ✓），那 27 处正是
        // 「开预置 ⇒ 判据 ① 多出几趟」**剩下的那部分** ✗。
        // 现在：walker 手上有**已经解析好的** `unit.file.commands` ✓ ⇒ 直接问
        // `command_env_id` ✓（**与 `canonical_prefix_id` 同一个函数** ✓ ⇒ 两条路不会分叉 ✓），
        // 一次 `parse` 都不用 ✓。契约：累加出的身份 == `canonical_prefix_id(prefix_src)` ✓
        // （判据 ④；前缀**能解析**时逐位相等 ✓，解析不过时以**这里**为准 ✓ —— 见探针 ✓）。
        let mut unit_env_ids: Vec<String> = vec![String::new(); units.len()];
        // 闭包累加（前面**所有单元**的身份）+ 每单元的快照（每单元只克隆一次 ✓）。
        //
        // **T1-A（2026-10-09）**：`resume` 时从**模块边界**起跑 —— 闭包身份接着
        // 前缀的累加值、导出表与 `_example_N` 计数器一并带上（三样都是跨单元
        // 累加器 ⇒ 不带就是静默错编面，见 `ResumeState` 的字段注释 ✓）。
        let mut closure_acc = self
            .resume
            .as_ref()
            .map(|r| r.closure_id.clone())
            .unwrap_or_default();
        if let Some(resume) = self.resume.take() {
            self.exports = resume.exports;
            self.example_idx = resume.example_idx;
        }
        let mut closure_ids: Vec<Option<String>> = vec![None; units.len()];
        let mut prev_unit: Option<usize> = None;
        // 探针读数：**不可比**的条数（前缀解析不过 ⇒ `expect` 是原文 ⇒ 那次不比 ✓）。
        let mut id_uncomparable = 0usize;
        for (idx, &(unit_idx, command)) in flat.iter().enumerate() {
            let unit = &units[unit_idx];
            // **每条声明一个计时事件**（`SOKO_DECL_PROFILE=1`；默认零开销 ✓）。
            // 为什么要有它：218.8s 只给聚合数答不了"花在哪一步"，更验不了
            // "单条成本是否随序号线性增长"（O(N²) 前缀重跑）——见
            // `docs/perf/course-profile-2026-09-29.md` 与 `scripts/profile-course.sh`。
            let profile_on = decl_profile::enabled();
            let decl_start = if profile_on {
                Some(std::time::Instant::now())
            } else {
                None
            };
            // 每处理**一条命令**回调一次（声明级）：先报"开始处理这一条"，
            // 于是首拍在编译一开始就到、末拍覆盖到最后一条命令 ✓。
            if let Some(sink) = progress.as_deref_mut() {
                sink.tick(crate::compile::ProgressTick {
                    module: unit.name,
                    index: unit_seen[unit_idx],
                    total: unit_totals[unit_idx],
                });
            }
            unit_seen[unit_idx] += 1;
            // G-05 N5：单元（文件）切换处清空作用域——`open` 与 `namespace`
            // 都是文件内的（`import` 不做模块限定，但被导入模块的**全局名**
            // 本来就可见，所以入口里的 `open Set` 对依赖的 `Set.mem` 仍然有效）。
            // 第二刀 §N7：`export` 是**唯一**跨 `import` 的那一半——清空之后
            // 重放导出表（依赖按拓扑序排在入口之前，此时它的导出已经齐了）。
            if idx == 0 || flat[idx - 1].0 != unit_idx {
                self.ns.reset();
                let exports = self.exports.clone();
                for entry in exports {
                    self.ns.open_entry(entry);
                }
                // **单元切换 ⇒ 把上一个单元的身份并进闭包累加** ✓。
                // `flat` 是「先依赖、后入口」且**按单元分组** ✓（见 `check/mod.rs` 的
                // `flat` 构造 ✓）⇒ 走到这里时上一个单元的命令**已经全部走完** ✓
                // ⇒ 它的身份**已经完整** ✓（这正是"每单元只算一次"的实现 ✓）。
                if let Some(done) = prev_unit {
                    closure_acc.push_str(&unit_env_ids[done]);
                }
                prev_unit = Some(unit_idx);
                closure_ids[unit_idx] = Some(closure_acc.clone());
            }
            // **信任判定**：连续前缀（`idx < before`）**或** S6 的脏集模型给出的
            // 额外信任位（`trusted_extra[idx]`，见 `TrustPlan`）。
            let trusted = trust.is_some_and(|t| {
                idx < t.before || t.trusted_extra.get(idx).copied().unwrap_or(false)
            });
            let env_before = self.builder.declaration_count();
            // `match` 的宇宙查询用前缀源码（与 `by` 同一条合成 `#check` 路线）：
            // 闭包模式 = 依赖声明文本 + 本文件到当前命令为止的前缀。
            let own_prefix = unit
                .file
                .src
                .get(..command.span().start.offset)
                .unwrap_or("");
            let prefix_src: Cow<'_, str> = match closure_prefixes.get(unit_idx) {
                Some(deps) if !deps.is_empty() => {
                    // 本文件前缀里的 `import` 行也要去掉：合成文件里它已经不在
                    // 文件开头，留着会让合成文件解析失败（那正是上一次尝试踩的坑）。
                    Cow::Owned(format!(
                        "{deps}{}",
                        crate::project::importless_source(own_prefix)
                    ))
                }
                _ => Cow::Borrowed(own_prefix),
            };
            // **预置增量身份** ✓（值守 2026-10-04 ✓）：闭包部分（每单元一次 ✓）+ 本文件部分
            // （滚动累加 ✓）⇒ `judge` 那边的 `canonical_prefix_cached` **直接命中** ✓，
            // **一次都不用解析** ✓（这就是删掉尺寸闸之后不回归的原因 ✓）。
            // ⚠ **本文件那半必须用 `importless_source(own_prefix)`** ✗→✓（2026-10-04 实测踩到 ✓）：
            // `prefix_src` 是 `deps + importless_source(own_prefix)` ✓ ⇒ 身份必须按**同一份文本**算 ✓。
            // 我先前图省事直接用 `own_prefix`（理由：「`import` 的身份是空串」✓）—— 那**不成立** ✗：
            // `importless_source` 会**重建字符串** ⇒ 剥过之后各命令的**切片文本**与原文不同 ⇒ 身份不同 ✓。
            // 实测后果：证明体两条判据从 `prefix=0` 掉到 **13** ✗。
            {
                // **闭包那半**：每单元一次 ✓（`closure_ids[unit_idx]` 在单元切换处快照 ✓）。
                let closure_id: &str = closure_ids[unit_idx].as_deref().unwrap_or("");
                // **本文件那半**：AST 直取的累加 ✓（**当前命令之前**的环境 ✓）。
                let own_id: &str = &unit_env_ids[unit_idx];
                let seeded = format!("{closure_id}{own_id}");
                // **判据 ④（值守 2026-10-04 ✓）：增量身份必须与重解析**逐位相等** ✓。**
                // `SOKO_PREFIX_ID_CHECK=1` 时逐命令自检 ✓，不等就打印**第一个分歧点** ✓
                // （这正是"错编"的入口 ✓ —— 不等就意味着键与真身份脱钩 ✓）。
                //
                // ⚠ **前缀解析不过时这次不可比** ✓（2026-10-04 实测口径 ✓）：那种前缀
                // （用了**依赖声明的记法** ✓）会让 `canonical_prefix_id` 走**退回原文** ✗
                // ⇒ `expect` 是**原文**、`seeded` 是**真身份** ✓ —— 这时**以 `seeded` 为准** ✓
                // （它来自**已经解析好的** AST ✓）。所以探针只比"**没退回**"的那些 ✓，
                // 并把"不可比"的条数单独报出来 ✓（否则会像上一棒那样把**降级**看成"零分歧" ✗）。
                if std::env::var_os("SOKO_PREFIX_ID_CHECK").is_some() {
                    crate::judge::stats::IDENTITY_PROBED
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let before_fb = crate::judge::stats::prefix_fallbacks().0;
                    let expect = crate::compile::canonical_prefix_id(&prefix_src);
                    let fell_back = crate::judge::stats::prefix_fallbacks().0 != before_fb;
                    if fell_back {
                        id_uncomparable += 1;
                        crate::judge::stats::IDENTITY_UNCOMPARABLE
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    } else if expect != seeded {
                        crate::judge::stats::IDENTITY_MISMATCHES
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let at = expect
                            .chars()
                            .zip(seeded.chars())
                            .position(|(a, b)| a != b)
                            .unwrap_or(expect.chars().count().min(seeded.chars().count()));
                        let win = |s: &str| -> String {
                            s.chars().skip(at.saturating_sub(30)).take(90).collect()
                        };
                        eprintln!(
                            "PREFIX_ID_MISMATCH at char {at}：expect_len={} seeded_len={}\n  \
                             expect…{:?}\n  seeded…{:?}",
                            expect.chars().count(),
                            seeded.chars().count(),
                            win(&expect),
                            win(&seeded)
                        );
                    }
                }
                // **预置增量身份** ✓（值守 2026-10-04 ✓）：闭包部分（每单元一次 ✓）+ 本文件部分
                // （AST 累加 ✓）⇒ `judge` 那边的 `canonical_prefix_cached` **直接命中** ✓，
                // **一次都不用解析** ✓（这就是删掉尺寸闸之后不回归的原因 ✓）。
                //
                // ⚠ **默认开** ✗→✓（2026-10-04 收口 ✓，两处修因都**实测定位**了 ✓）：
                // 先前默认关，理由是「开着 ⇒ 判据 ① 从 `prefix=0` 变 **5**」✗ —— 机制
                // **现已查明**（不是猜的 ✓，两条都是**实测** ✓）：
                // ① `canonical_prefix_id` 用**严格** `parse` ✗ ⇒ 前缀停在未闭合 `namespace`
                //    里时**必然**报 `parse-namespace-unclosed` ✗ ⇒ **退回原文** ✗
                //    （课程 `unit08` 自检 **1505/1540** 条 ✗）⇒ 键退化成**原文哈希** ✗，
                //    与这里累加出的**真身份**对不上 ✗ —— 已改成 `parse_fragment` ✓；
                // ② 剩下那几趟 = 片段用了**依赖声明的记法** ⇒ 片段与整体**都解析不过** ✗
                //    ⇒ 退到原文 ✗（实测 27 处 ✗）—— 已改成**从 AST 直取** ✓，一次 parse 都不用 ✓。
                // 两条都修掉之后：判据 ① 与 ② **同时**成立 ✓（见 `lsp_keystroke_structure` ✓）。
                // 逃生门 `SOKO_NO_SEED=1`（排查用 ✓，**不是**降级结案 ✓）。
                if std::env::var_os("SOKO_NO_SEED").is_none() {
                    crate::judge::seed_canonical_prefix(&prefix_src, &seeded);
                }
                // **用完之后**再把这条命令并进本单元的累加 ✓（身份是"**当前命令之前**"的环境 ✓）。
                let contribution = super::command_env_id(&unit.file.src, command);
                if !contribution.is_empty() {
                    let acc = &mut unit_env_ids[unit_idx];
                    acc.push_str(&contribution);
                    acc.push('\n');
                }
            }
            let c = CmdCtx {
                idx,
                templates: &all_templates[unit_idx],
                prefix_src,
                trusted,
                env_before,
                options,
                skip,
                canonical_goal: unit_uses_namespaces[unit_idx],
                src: &unit.file.src,
            };
            // **§31.2 的只读取证**（`SOKO_JUDGE_ENV_PROBE=1`，零行为变化）：
            // 主编译 pass 每检查完一条命令，就把"**这条命令之前**已核的命令数"
            // 压进 `TRUSTED_PREFIX`，好让 judge 的 `check_synthesized` 能量到
            // "**若这条路接通，能不能担保住**"。
            //
            // ⚠ **为什么量这个**：judge 拿到的 `prefix_commands` 是它**合成文档里
            // 属于前缀的命令数**，而 pass 手上的是**自己这一趟的命令序号 `idx`**
            // —— 两者**是不是同一个数**正是 §31.2 的第 ① 条前提
            //（"judge 的文本前缀与 pass 已核的命令同序同源"）。
            // 相等 ⇒ 路成立 ✓；普遍不等 ⇒ 此路作废 ✗（不许凭推断动手）。
            //
            // ⚠ **压的是 `idx`（这条命令**之前**的数）而不是 `idx + 1`**：
            // judge 合成文档里前缀命令 = `[0, prefix_commands)`，而 `self.command`
            // 正在检查的是第 `idx` 条 ⇒ 已核的是 `[0, idx)` ✓。
            // **接通（§30.5 的刀口，落地见 §31.1）**：`SOKO_JUDGE_ENV_VOUCH=1` ⇒ 主编译 pass
            // 真的压栈担保（judge 的前缀因此不再重查）。
            // ⚠ 与 `probe` 分开两个开关：**取证**（只读、零行为）与**生效**（改行为）
            // 必须能各自单独开，否则"量到的"与"生效的"分不清 ✗。
            // ⚠ **影子档也要压栈**（`vouch_mode() != Off`）：它要"两条路都跑"，
            // 而"被担保那条路"必须先有栈才走得通 ✓。
            let vouch =
                crate::judge::env_probe::vouch_mode() != crate::judge::env_probe::VouchMode::Off;
            // **G-29 第 3 棒**：压栈的 `before` 必须落在 **judge 合成文档前缀的坐标系**里
            // （= 整条闭包的命令序 ✓）。本趟走的是 `units`，`idx` 是**本趟自己**的序号
            // ⇒ 加上调用方给的平移量 `judge_prefix_offset`（库层那一段的命令数 ✓）。
            // 默认 `0` ⇒ 与今天**逐字节相同** ✓（老路/单文件那条路本来就同坐标系 ✓）。
            let judge_before = idx + self.judge_prefix_offset;
            if vouch || crate::judge::env_probe::on() {
                crate::judge::with_trusted_prefix(
                    judge_before,
                    &Default::default(),
                    // **本趟 walk 自己的**「成功进环境」名表 ✓（G-31/G-92 第二刀）：
                    // judge 的合成文档据此把「确实加过」的前缀 `theorem` 装成
                    // 不透明常量 ⇒ 证明体不重跑 ✓。
                    Some(self.entered.clone()),
                    || {
                        self.command(&c, command);
                    },
                );
            } else {
                self.command(&c, command);
            }
            if let Some(start) = decl_start {
                decl_profile::emit(
                    unit.name,
                    decl_profile::decl_name(command),
                    unit_seen[unit_idx] - 1,
                    unit_totals[unit_idx],
                    start.elapsed(),
                );
            }
        }
        // **探针读数**（`SOKO_PREFIX_ID_CHECK=1`）：这一趟 walk 里有多少条前缀
        // **解析不过**（⇒ 判据 ④ 那次不可比 ✓）。它必须能回答"零分歧"是**真等价** ✓
        // 还是**全被跳过** ✗ —— 上一棒正是栽在这里（把**降级**看成"零分歧" ✗）。
        if std::env::var_os("SOKO_PREFIX_ID_CHECK").is_some() {
            eprintln!("PREFIX_ID_CHECK uncomparable={id_uncomparable}");
        }
        // **T1-A**：把**本趟结束 = 模块边界**上的续编状态交回调用方。
        // ⚠ 闭包身份必须**补上最后一个单元**：主循环只在**单元切换处**累加
        // （`idx == 0 || flat[idx-1].0 != unit_idx`）⇒ 循环结束时 `closure_acc`
        // 里**没有**最后一个单元 ✓ —— 而"模块边界"要的正是**含它**的那一份
        // （与主循环里"切到下一个单元时"的读数逐字相同 ✓）。
        if self.snapshot_state {
            let mut closure_id = closure_acc;
            if let Some(last) = prev_unit {
                closure_id.push_str(&unit_env_ids[last]);
            }
            self.resume_out = Some(crate::compile::ResumeState {
                closure_id,
                exports: self.exports.clone(),
                example_idx: self.example_idx,
            });
        }
    }

    /// 一条命令的分发（原 `run` 主循环里的 `match`，逐字搬过来）。
    ///
    /// 抽成方法的**唯一**原因：`open Foo in <命令>` 要把被包住的命令按同一个
    /// 上下文再走一遍（见 [`Walk::open_in`]）；arm 里的 `return` 语义不变
    /// （每个 arm 都没有内层循环，返回后 `run` 继续下一条命令）。
    fn command(&mut self, c: &CmdCtx<'_>, command: &Command) {
        match command {
            // `import` 自身不产生声明：被导入模块的命令由项目层按拓扑序
            // 先送进同一个 EnvBuilder（docs/design/imports-and-projects.md §4.5）。
            Command::Import { .. } => {}
            Command::Def {
                name,
                universe,
                ty,
                val,
                span,
            } => self.def(c, name, universe, ty, val, *span),
            Command::Theorem {
                name,
                universe,
                ty,
                val,
                span,
            } => self.theorem(c, name, universe, ty, val, *span),
            Command::Axiom {
                name,
                universe,
                ty,
                span,
            } => self.axiom(c, name, universe, ty, *span),
            Command::Example { ty, val, span } => self.example(c, ty, val, *span),
            Command::InductiveBlock {
                name,
                params,
                ty,
                constructors,
                recursor,
                iota_rules,
                span,
            } => self.inductive_block(
                c,
                name,
                params,
                ty,
                constructors,
                recursor,
                iota_rules,
                *span,
            ),
            Command::Check { expr, span: _ } => self.check(c, expr),
            Command::Reduce { expr, span: _ } => self.reduce(c, expr),
            Command::Print { name, span } => self.print(c, name, *span),
            // 记法命令**不是声明**（设计 N6）：不 elaborate、不产
            // PendingOp、不进声明表——与 `Command::Import` 同族。
            Command::Notation { .. } => {}
            // G-05：三条作用域命令同样不是声明。声明名加前缀在 parser 里
            // 已经落定（N3），这里只维护**引用解析**用的作用域（N4）：
            // `namespace` 压栈、`end` 弹栈、`open` 进可省略前缀集合。
            // trusted 前缀也要走（否则后半段的解析会丢作用域）。
            Command::Namespace { name, .. } => self.ns.push(name),
            Command::End { .. } => self.ns.pop(),
            // `open scoped <名字>`（第三刀 §12.3）**只**打开记法作用域
            // （副作用在 parser 里已经落定），**不**打开名字前缀——与 Lean
            // 一致（`open scoped Foo` 不会让 `Foo.bar` 能写成 `bar`）。
            Command::Open {
                name,
                scoped: false,
                filter,
                ..
            } => self.ns.open_entry(OpenEntry::new(name, filter.clone())),
            Command::Open { scoped: true, .. } => {}
            // `open Foo in <命令>`（第二刀 §N7）：局部 open。
            Command::OpenIn {
                name,
                filter,
                inner,
                header,
                ..
            } => self.open_in(c, name, filter, inner, *header),
            // `export Foo`（第二刀 §N7）：本文件内与 `open` 逐字相同，额外
            // 记进导出表（跨 `import` 生效）。
            Command::Export { name, filter, .. } => {
                let entry = OpenEntry::new(name, filter.clone());
                self.ns.open_entry(entry.clone());
                if !self.exports.contains(&entry) {
                    self.exports.push(entry);
                }
            }
        }
    }

    /// `open <name> [<子句>] in <命令>`（第二刀 §N7）：把 open 压进作用域、
    /// 走一遍被包住的命令、再撤销。
    ///
    /// 被包住的命令用**同一个 `CmdCtx`**，只把合成前缀补一行 open 的源码文本
    /// （`open Foo hiding a`）——`by` 引擎的根目标规范化（`judge_render_type`）
    /// 与 `judge_terms` 都是"前缀源码 + 合成命令"再走一遍流水线，前缀里没有这
    /// 一行，短名在那里就解析不了（退回源 AST 是安全的，但 `apply` 的文本对齐
    /// 会失准）。补的是**源码原文**，不是重建的文本，所以子句逐字保真。
    fn open_in(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        filter: &OpenFilter,
        inner: &Command,
        header: Span,
    ) {
        let mark = self.ns.opens_mark();
        self.ns.open_entry(OpenEntry::new(name, filter.clone()));
        let header_text = c
            .src
            .get(header.start.offset..header.end.offset)
            .unwrap_or("");
        let inner_ctx = CmdCtx {
            idx: c.idx,
            templates: c.templates,
            prefix_src: Cow::Owned(format!("{}{header_text}\n", c.prefix_src)),
            trusted: c.trusted,
            env_before: c.env_before,
            options: c.options,
            skip: c.skip,
            canonical_goal: c.canonical_goal,
            src: c.src,
        };
        self.command(&inner_ctx, inner);
        self.ns.rollback_opens(mark);
    }

    /// `def name : T := v`：elaborate 成 `PendingOp::Decl`（或开练习）。
    #[allow(clippy::too_many_arguments)]
    fn def(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        universe: &[String],
        ty: &Expr,
        val: &Expr,
        span: Span,
    ) {
        let idx = c.idx;
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            notations: Some(&self.display),
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        // **P1-b 第二刀**：`by` 引擎的就地判定用调用方手里的活环境
        // （开关关着 ⇒ `None` ⇒ 逐字节回到今天 ✓）。先落到**具名变量**再借出去
        // （`Option<InplaceEnv>` 直接传是临时值 ⇒ temporary-dropped ✗）。
        let mut by_env = None;
        crate::judge::inplace_env_for_by(&mut by_env, &mut self.builder, &self.known);
        let lowered = match lower_value(
            ty,
            val,
            universe,
            prefix_src,
            options,
            c.canonical_goal,
            &self.inductives,
            &self.defs,
            &elab_ctx,
            by_env.as_mut(),
        ) {
            Ok(v) => v,
            Err(failure) => {
                self.out.push_error(idx, failure.error.clone());
                {
                    let mut st = failed_state(
                        DeclKind::Definition,
                        Some(name.to_string()),
                        span,
                        failure.error,
                        idx,
                    );
                    // **B1**：失败也要保留**已跑成功的**那些步（P3 显示面）✓
                    st.by_steps = by_step_states(&failure.steps, &self.display);
                    st.by_root = decl_prefix_state(ty, val, &self.display);
                    self.decl_states.push(st);
                }
                return;
            }
        };
        let by_root = decl_prefix_state(ty, val, &self.display);
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1, &self.display);
        // 源级 delta 表：**值完整**的 def 才登记（开练习的值是洞，展开没意义）。
        // `by` 引擎的 `intro`/`apply` 靠它看穿 `A ⊆ B` 这类 def 头。
        if open_goal(ty, val, templates, &mut Vec::new()).is_none() {
            let params = params_of_ty(ty);
            let info = DefInfo {
                universes: universe.to_vec(),
                implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                // **G-72**：剥的层数 = `params` 的长度（= 值位外面那层 lambda 的
                // binder 数）。两者必须**同源**：`params` 只数前导 `Forall`
                // （返回类型里的 `->` 不是参数），否则会多剥 ⇒ 定义体里出现悬空
                // 变量 ⇒ 展开回读报 `unknown identifier …`。
                body: strip_lambdas_n(val, params.len()),
                telescope_arity: telescope_arity_of_ty(ty),
                params,
            };
            self.defs.insert(name.to_string(), info.clone());
            // **短名别名**（R2 实测）：`namespace Set` 里的 def 体是用**短名**
            // 写的（`def powerset … := fun B => subset α B A`），而 delta 展开是
            // 逐层的——第二层拿到的头是短名 `subset`，`defs` 里却只有规范名
            // `Set.subset` ⇒ 展开在第二层断掉（实测：`A ∈ 𝒫 B` 上 `intro` 报
            // 「需要一个函数目标」，而目标明明是集合成员关系）。
            //
            // 只登记**不冲突**的短名（先到先得）：同名短名在两个命名空间里都有
            // 时保持今天的行为（查不到 ⇒ 不展开 ⇒ 响亮报错），绝不猜。
            if let Some(short) = name.rsplit('.').next() {
                if short != name && !short.is_empty() {
                    self.defs.entry(short.to_string()).or_insert(info);
                }
            }
        }
        if trusted {
            // Trusted prefix: keep the environment, skip the kernel.
            // Cached failures keep the name free (check-then-add);
            // open exercises never enter the environment anyway.
            if skip.is_some_and(|s| s.contains_key(&idx))
                || open_goal(ty, val, templates, &mut Vec::new()).is_some()
            {
                return;
            }
            let mut hovers = Vec::new();
            if let Ok(decl) = build_def(
                &mut self.builder,
                name,
                universe,
                ty,
                val,
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                // **R1c-2b（2026-10-05）**：**声明出口** ✓ —— 层元变量收口
                // （对齐 Lean：`levelMVarToParam` 在 `addDecl` **之前** ✓）。
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Definition,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        let mut redundant_spans: Vec<Span> = Vec::new();
        let open_info = open_goal(ty, val, templates, &mut redundant_spans);
        if let Some(info) = open_info {
            // G-01：签名必须先过 elaborate；`Err` 与值位 elaborate 失败同罪。
            let mut signature =
                match open_signature(&mut self.builder, universe, ty, &self.known, &elab_ctx) {
                    Ok(sig) => sig,
                    Err(e) => {
                        self.out.push_error(idx, e.clone());
                        {
                            let mut st = failed_state(
                                DeclKind::Definition,
                                Some(name.to_string()),
                                span,
                                e,
                                idx,
                            );
                            st.by_root = by_root.clone();
                            self.decl_states.push(st);
                        }
                        return;
                    }
                };
            // **签名里的 hover 行**（A3）：开放练习也是声明，它的签名同样要能
            // hover / 跳转。`env_at` 取 `env_before` —— 签名就是在**本声明入环境
            // 之前** elaborate 的（与 `#check` 同一条口径）。
            self.cmd_hovers.push(CmdHover {
                env_at: c.env_before,
                nodes: std::mem::take(&mut signature.hovers),
                cmd: idx,
            });
            self.ops.push(PendingOp::OpenExercise {
                name: Some(name.to_string()),
                kind: DeclKind::Definition,
                // **R1c-2b（2026-10-05）**：签名出口新加的宇宙参数名并进来 ✓
                // （对齐 Lean `setLevelNames (r.newParamNames …)` ✓）——
                // judge 合成声明时要靠它才知道 `u_1`/`u_2`… ✓。
                universe: universe
                    .iter()
                    .cloned()
                    .chain(signature.fresh_params.iter().cloned())
                    .collect(),
                redundant_probes: {
                    {
                        // **T-D8**：开关下**看不见文件声明** ✓（探针仍在真 `builder` 的 DAG 里
                        // elaborate ✓ ⇒ 指针同一性保住 ✓；只是环境里没有文件声明 ✓）。
                        // 用 hide/restore **两个方法**而**不是**闭包 ✗：这里同时借
                        // `&mut self.builder` 与 `&self.known`（**不同字段** ✓）⇒ 闭包会让
                        // `self` 被可变借两次 ✗（round 301 预判 ✓）。
                        let saved = if Self::walk_real_add_enabled() {
                            {
                                Some(self.builder.hide_declars())
                            }
                        } else {
                            {
                                None
                            }
                        };
                        let probes = build_redundant_probes(
                            &mut self.builder,
                            universe,
                            ty,
                            val,
                            &redundant_spans,
                            &self.known,
                            &elab_ctx,
                        );
                        if let Some(s) = saved {
                            {
                                self.builder.restore_declars(s);
                            }
                        }
                        probes
                    }
                },
                env_before: c.env_before,
                declared_ty: Some(signature.declared_ty),
                sig_probe: signature.probe,
                sig_span: signature.span,
                goal: Some(info.goal.clone()),
                binders: info.binders.clone(),
                holes: info.holes,
                sub_goals: info.sub_goals,
                refine_template: info.refine_template,
                by_steps: by_steps.clone(),
                // **与「底」（声明卡片）同源**（2026-10-02 值守第 9 单）：
                // open 声明的根状态就用 `open_goal` 的 `info` ⇒ 顶 ≡ 底 **by construction** ✓
                // （`kernel_phase` 把同一份 `goal`/`binders` 填进 `DeclState` ✓）。
                by_root: Some(ByGoalState {
                    // **折一次**：`info.goal`/绑元类型是**源级渲染**（`And P Q`），而
                    // 这条 `by_root` 还会被 **hover 路径直接渲染**（不经查询层的 fold ✗）
                    // ⇒ 在这里折好 ⇒ wire 与 hover 同一份文本 ✓（折叠幂等 ✓，wire 不变 ✓）。
                    ty: self.display.fold(&info.goal),
                    binders: info
                        .binders
                        .iter()
                        .map(|b| GoalBinder {
                            name: b.name.clone(),
                            ty: self.display.fold(&b.ty),
                        })
                        .collect(),
                }),
                span,
                cmd: idx,
            });
            return;
        }
        let mut hovers = Vec::new();
        match build_def(
            &mut self.builder,
            name,
            universe,
            ty,
            val,
            &self.known,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                let name_owned = name.to_string();
                // **R1c-2b（2026-10-05）**：**声明出口** ✓（同上：`addDecl` 之前 ✓）。
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    {
                        let mut st = failed_state(
                            DeclKind::Definition,
                            Some(name_owned.clone()),
                            span,
                            err,
                            idx,
                        );
                        st.by_root = by_root.clone();
                        self.decl_states.push(st);
                    }
                    return;
                }
                self.known.insert(
                    name_owned.clone(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: Some(name_owned),
                    kind: DeclKind::Definition,
                    declar: decl,
                    by_steps: by_steps.clone(),
                    by_root: by_root.clone(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                {
                    let mut st =
                        failed_state(DeclKind::Definition, Some(name.to_string()), span, e, idx);
                    st.by_root = by_root.clone();
                    self.decl_states.push(st);
                }
            }
        }
    }

    /// `theorem name : T := v`：与 `def` 同形，只差 `DeclKind` 与构造器。
    #[allow(clippy::too_many_arguments)]
    fn theorem(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        universe: &[String],
        ty: &Expr,
        val: &Expr,
        span: Span,
    ) {
        let idx = c.idx;
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            notations: Some(&self.display),
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        // **G-31/G-92 第二刀（2026-10-07 ✓）**：合成判定文档的**前缀定理** ——
        // 调用方那一趟**确实把这个名字加进过环境** ⇒ 它的**证明体对下游零可观测**
        //（`conv.rs::unfold_hint` ⇒ 定理一律 `Opaque`、**永不展开** ✓；
        //  `self.defs` delta 表**只收 `fn def`** ✓）⇒ **只 elaborate 类型**、
        // 按**不透明常量**（`Declar::Axiom`）加进环境 ⇒ 前缀的 `by` **不再重跑** ✓
        //（这正是 `by_calls` 的 Σ(1..N) 那一项 ✓）。
        //
        // ⚠ **只在「调用方确实加过」时触发** ✗→✓：`skip`（失败命令）与 `open_goal`
        //（开放练习）今天在受信分支里**跳过不加** ⇒ 那两类**不**在名表里 ⇒ 走**原路**
        //（逐字保留今天的加/不加与报错行为 ✓ —— 红线 ✓）。
        // ⚠ `known` 那条插入与 `build_theorem` 那条**逐字段相同**（都从**源 `ty`** 算 ✓）。
        if trusted
            && self
                .trusted_entered
                .as_ref()
                .is_some_and(|e| e.borrow().contains(name))
        {
            let mut hovers = Vec::new();
            if let Ok(decl) = build_axiom(
                &mut self.builder,
                name,
                universe,
                ty,
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
                // 内层的**嵌套** judge 也要看得到它（它的前缀是本份合成文档的前段 ✓）。
                self.entered.borrow_mut().insert(name.to_string());
            }
            return;
        }
        // **P1-b 第二刀**：`by` 引擎的就地判定用调用方手里的活环境
        // （开关关着 ⇒ `None` ⇒ 逐字节回到今天 ✓）。先落到**具名变量**再借出去
        // （`Option<InplaceEnv>` 直接传是临时值 ⇒ temporary-dropped ✗）。
        let mut by_env = None;
        crate::judge::inplace_env_for_by(&mut by_env, &mut self.builder, &self.known);
        let lowered = match lower_value(
            ty,
            val,
            universe,
            prefix_src,
            options,
            c.canonical_goal,
            &self.inductives,
            &self.defs,
            &elab_ctx,
            by_env.as_mut(),
        ) {
            Ok(v) => v,
            Err(failure) => {
                self.out.push_error(idx, failure.error.clone());
                {
                    let mut st = failed_state(
                        DeclKind::Theorem,
                        Some(name.to_string()),
                        span,
                        failure.error,
                        idx,
                    );
                    // **B1**：失败也要保留**已跑成功的**那些步（P3 显示面）✓
                    st.by_steps = by_step_states(&failure.steps, &self.display);
                    st.by_root = decl_prefix_state(ty, val, &self.display);
                    self.decl_states.push(st);
                }
                return;
            }
        };
        // **题面状态**（G-82，源位值、未折记法）：`open_goal` 分解不了值时兜底要用它
        // —— 拿 lowering **之后**的值数 λ 链会多剥 ✗（见 `decl_root_state` 的说明）。
        // 算**一次**：`by_root` 就是它的显示副本（折叠幂等 ✓）。
        let src_root = decl_root_state(ty, val, &self.display);
        let by_root = src_root
            .as_ref()
            .map(|state| fold_root_state(state, &self.display));
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1, &self.display);
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx))
                || open_goal(ty, val, templates, &mut Vec::new()).is_some()
            {
                return;
            }
            let mut hovers = Vec::new();
            if let Ok(decl) = build_theorem(
                &mut self.builder,
                name,
                universe,
                ty,
                val,
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                // **R1c-2b（2026-10-05）**：**声明出口** ✓ —— 层元变量收口
                // （对齐 Lean：`levelMVarToParam` 在 `addDecl` **之前** ✓）。
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
                // **受信分支也记账** ✓（G-31/G-92 第二刀）：内层的**嵌套** judge
                // 看到的前缀是本份文档的前段 ⇒ 它也要知道这个名字已经进过环境 ✓。
                self.entered.borrow_mut().insert(name.to_string());
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Theorem,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        // 尾部复用 + fallback（I13-S5b）：open_goal 的 spine 走查
        // 无法分解时（如超量应用、def 展开间接调用），如果值里有
        // 洞 → 生成 **generic open exercise**（整值 = 一个洞，目标 =
        // 声明类型）。学习者看到的是一个可填充的练习而不是报错。
        let mut redundant_spans: Vec<Span> = Vec::new();
        let open_info = open_goal(ty, val, templates, &mut redundant_spans);
        let open_info = match open_info {
            Some(info) => Some(info),
            None if expr_has_hole(val) => {
                // spine 走查无法分解，但值有洞 → generic open exercise。
                //
                // ⚠ **不许**退回「空上下文 + 整句声明类型」✗（G-82，2026-10-03）：
                // 有具名绑元的题会显示成「题目参数忽然跑到目标里去了」、整句判红 ✗
                // —— 走查分解不了**不等于**题面没有上下文 ✓。至少答题面状态
                // （具名绑元 + 剥掉它们之后的命题 ✓，与 `by_root` 同源 ⇒ 顶 ≡ 底 ✓）；
                // 没有 `by` 块（`src_root == None`）时保持原样（那时空上下文本来就是对的 ✓）。
                //
                // **闸类出口**（G-91 乙类 ✓）：走到这里 = **目标分解失败** ⇒ 题面
                // 退回 generic 兜底 ✓ —— **判定不变** ✓（仍是一个可填的练习 ✓），
                // **显示降质** ✗（子洞期望类型不再精确 ✓）⇒ 必须看得见 ✗。
                // ⚠ 只数**整条走查失败**这一处 ✗：`goals.rs` 内部各策略的 `return None`
                // 是正常的不匹配 ✓（那条形状不归它管，另一条会接 ✓），数进去会假读数 ✗。
                sokonanoda::gates::GOAL_DECOMPOSE_FALLBACK.bump();
                let (goal, binders) = src_root
                    .clone()
                    .unwrap_or_else(|| (self.display.render(ty), Vec::new()));
                Some(crate::compile::goals::OpenGoalInfo {
                    goal,
                    binders,
                    holes: vec![val.span()],
                    sub_goals: Vec::new(),
                    refine_template: None,
                })
            }
            _ => None,
        };
        if let Some(info) = open_info {
            // G-01：签名必须先过 elaborate；`Err` 与值位 elaborate 失败同罪。
            let mut signature =
                match open_signature(&mut self.builder, universe, ty, &self.known, &elab_ctx) {
                    Ok(sig) => sig,
                    Err(e) => {
                        self.out.push_error(idx, e.clone());
                        {
                            let mut st = failed_state(
                                DeclKind::Theorem,
                                Some(name.to_string()),
                                span,
                                e,
                                idx,
                            );
                            st.by_root = by_root.clone();
                            self.decl_states.push(st);
                        }
                        return;
                    }
                };
            // **签名里的 hover 行**（A3）：开放练习也是声明，它的签名同样要能
            // hover / 跳转。`env_at` 取 `env_before` —— 签名就是在**本声明入环境
            // 之前** elaborate 的（与 `#check` 同一条口径）。
            self.cmd_hovers.push(CmdHover {
                env_at: c.env_before,
                nodes: std::mem::take(&mut signature.hovers),
                cmd: idx,
            });
            self.ops.push(PendingOp::OpenExercise {
                name: Some(name.to_string()),
                kind: DeclKind::Theorem,
                // **R1c-2b（2026-10-05）**：签名出口新加的宇宙参数名并进来 ✓
                // （对齐 Lean `setLevelNames (r.newParamNames …)` ✓）——
                // judge 合成声明时要靠它才知道 `u_1`/`u_2`… ✓。
                universe: universe
                    .iter()
                    .cloned()
                    .chain(signature.fresh_params.iter().cloned())
                    .collect(),
                redundant_probes: {
                    {
                        // **T-D8**：开关下**看不见文件声明** ✓（探针仍在真 `builder` 的 DAG 里
                        // elaborate ✓ ⇒ 指针同一性保住 ✓；只是环境里没有文件声明 ✓）。
                        // 用 hide/restore **两个方法**而**不是**闭包 ✗：这里同时借
                        // `&mut self.builder` 与 `&self.known`（**不同字段** ✓）⇒ 闭包会让
                        // `self` 被可变借两次 ✗（round 301 预判 ✓）。
                        let saved = if Self::walk_real_add_enabled() {
                            {
                                Some(self.builder.hide_declars())
                            }
                        } else {
                            {
                                None
                            }
                        };
                        let probes = build_redundant_probes(
                            &mut self.builder,
                            universe,
                            ty,
                            val,
                            &redundant_spans,
                            &self.known,
                            &elab_ctx,
                        );
                        if let Some(s) = saved {
                            {
                                self.builder.restore_declars(s);
                            }
                        }
                        probes
                    }
                },
                env_before: c.env_before,
                declared_ty: Some(signature.declared_ty),
                sig_probe: signature.probe,
                sig_span: signature.span,
                goal: Some(info.goal.clone()),
                binders: info.binders.clone(),
                holes: info.holes,
                sub_goals: info.sub_goals,
                refine_template: info.refine_template,
                by_steps: by_steps.clone(),
                // **与「底」（声明卡片）同源**（2026-10-02 值守第 9 单）：
                // open 声明的根状态就用 `open_goal` 的 `info` ⇒ 顶 ≡ 底 **by construction** ✓
                // （`kernel_phase` 把同一份 `goal`/`binders` 填进 `DeclState` ✓）。
                by_root: Some(ByGoalState {
                    // **折一次**：`info.goal`/绑元类型是**源级渲染**（`And P Q`），而
                    // 这条 `by_root` 还会被 **hover 路径直接渲染**（不经查询层的 fold ✗）
                    // ⇒ 在这里折好 ⇒ wire 与 hover 同一份文本 ✓（折叠幂等 ✓，wire 不变 ✓）。
                    ty: self.display.fold(&info.goal),
                    binders: info
                        .binders
                        .iter()
                        .map(|b| GoalBinder {
                            name: b.name.clone(),
                            ty: self.display.fold(&b.ty),
                        })
                        .collect(),
                }),
                span,
                cmd: idx,
            });
            return;
        }
        let mut hovers = Vec::new();
        match build_theorem(
            &mut self.builder,
            name,
            universe,
            ty,
            val,
            &self.known,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                let name_owned = name.to_string();
                // **R1c-2b（2026-10-05）**：**声明出口** ✓（同上：`addDecl` 之前 ✓）。
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    {
                        let mut st = failed_state(
                            DeclKind::Theorem,
                            Some(name_owned.clone()),
                            span,
                            err,
                            idx,
                        );
                        st.by_root = by_root.clone();
                        self.decl_states.push(st);
                    }
                    return;
                }
                self.known.insert(
                    name_owned.clone(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
                // **记账**（G-31/G-92 第二刀 ✓）：这一趟**成功进环境**的名字 ——
                // judge 的合成文档据此把前缀 `theorem` 装成不透明常量（证明体不重跑 ✓）。
                // ⚠ 只记**真的加进去**的（`add_declar` 失败已在上面的 `return` 里排除 ✓）。
                self.entered.borrow_mut().insert(name_owned.clone());
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: Some(name_owned),
                    kind: DeclKind::Theorem,
                    declar: decl,
                    by_steps: by_steps.clone(),
                    by_root: by_root.clone(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                {
                    let mut st =
                        failed_state(DeclKind::Theorem, Some(name.to_string()), span, e, idx);
                    st.by_root = by_root.clone();
                    self.decl_states.push(st);
                }
            }
        }
    }

    /// `axiom name : T`：只 elaborate 类型，没有值。
    #[allow(clippy::too_many_arguments)]
    fn axiom(&mut self, c: &CmdCtx<'_>, name: &str, universe: &[String], ty: &Expr, span: Span) {
        let idx = c.idx;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            notations: Some(&self.display),
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx)) {
                return;
            }
            let mut hovers = Vec::new();
            if let Ok(decl) = build_axiom(
                &mut self.builder,
                name,
                universe,
                ty,
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                // **R1c-2b（2026-10-05）**：**声明出口** ✓ —— 层元变量收口
                // （对齐 Lean：`levelMVarToParam` 在 `addDecl` **之前** ✓）。
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Axiom,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        let mut hovers = Vec::new();
        match build_axiom(
            &mut self.builder,
            name,
            universe,
            ty,
            &self.known,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                let name_owned = name.to_string();
                // **R1c-2b（2026-10-05）**：**声明出口** ✓（同上：`addDecl` 之前 ✓）。
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    self.decl_states.push(failed_state(
                        DeclKind::Axiom,
                        Some(name_owned.clone()),
                        span,
                        err,
                        idx,
                    ));
                    return;
                }
                self.known.insert(
                    name_owned.clone(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::decl_signature(ty)),
                    },
                );
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: Some(name_owned),
                    kind: DeclKind::Axiom,
                    declar: decl,
                    by_steps: Vec::new(),
                    by_root: None,
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Axiom,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
            }
        }
    }

    /// `example : T := v`：没有名字，内部名按出现次序编号（`_example_N`）。
    #[allow(clippy::too_many_arguments)]
    fn example(&mut self, c: &CmdCtx<'_>, ty: &Expr, val: &Expr, span: Span) {
        let idx = c.idx;
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            notations: Some(&self.display),
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        let mut by_env = None;
        crate::judge::inplace_env_for_by(&mut by_env, &mut self.builder, &self.known);
        let lowered = match lower_value(
            ty,
            val,
            // `example` 不能声明宇宙参数 ⇒ 空切片（判定合成声明不需要带宇宙 binder）。
            &[],
            prefix_src,
            options,
            c.canonical_goal,
            &self.inductives,
            &self.defs,
            &elab_ctx,
            by_env.as_mut(),
        ) {
            Ok(v) => v,
            Err(failure) => {
                self.out.push_error(idx, failure.error.clone());
                let mut st = failed_state(DeclKind::Example, None, span, failure.error, idx);
                // **B1**：失败也要保留**已跑成功的**那些步（P3 显示面）✓
                st.by_steps = by_step_states(&failure.steps, &self.display);
                st.by_root = decl_prefix_state(ty, val, &self.display);
                self.decl_states.push(st);
                return;
            }
        };
        // **题面状态**（G-82，源位值、未折记法）：`open_goal` 分解不了值时兜底要用它
        // —— 拿 lowering **之后**的值数 λ 链会多剥 ✗（见 `decl_root_state` 的说明）。
        // 算**一次**：`by_root` 就是它的显示副本（折叠幂等 ✓）。
        let src_root = decl_root_state(ty, val, &self.display);
        let by_root = src_root
            .as_ref()
            .map(|state| fold_root_state(state, &self.display));
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1, &self.display);
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx))
                || open_goal(ty, val, templates, &mut Vec::new()).is_some()
            {
                return;
            }
            self.example_idx += 1;
            let internal_name = format!("_example_{}", self.example_idx);
            let mut hovers = Vec::new();
            if let Ok(decl) = build_example(
                &mut self.builder,
                &internal_name,
                ty,
                val,
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                let (decl, _fresh) = self.discharge_level_mvars(decl);
                let _ = self.builder.add_declar(decl);
            }
            return;
        }
        if let Some(err) = skipped(skip, &mut self.out, idx, DeclKind::Example, None, span) {
            self.decl_states.push(err);
            return;
        }
        // 尾部复用 + fallback（I13-S5b）：open_goal 的 spine 走查
        // 无法分解时（如超量应用、def 展开间接调用），如果值里有
        // 洞 → 生成 **generic open exercise**（整值 = 一个洞，目标 =
        // 声明类型）。学习者看到的是一个可填充的练习而不是报错。
        let mut redundant_spans: Vec<Span> = Vec::new();
        let open_info = open_goal(ty, val, templates, &mut redundant_spans);
        let open_info = match open_info {
            Some(info) => Some(info),
            None if expr_has_hole(val) => {
                // spine 走查无法分解，但值有洞 → generic open exercise。
                //
                // ⚠ **不许**退回「空上下文 + 整句声明类型」✗（G-82，2026-10-03）：
                // 有具名绑元的题会显示成「题目参数忽然跑到目标里去了」、整句判红 ✗
                // —— 走查分解不了**不等于**题面没有上下文 ✓。至少答题面状态
                // （具名绑元 + 剥掉它们之后的命题 ✓，与 `by_root` 同源 ⇒ 顶 ≡ 底 ✓）；
                // 没有 `by` 块（`src_root == None`）时保持原样（那时空上下文本来就是对的 ✓）。
                //
                // **闸类出口**（G-91 乙类 ✓）：同 `Theorem` 那条（**目标分解失败** ⇒
                // generic 兜底 ⇒ 显示降质 ✗、判定不变 ✓）。
                sokonanoda::gates::GOAL_DECOMPOSE_FALLBACK.bump();
                let (goal, binders) = src_root
                    .clone()
                    .unwrap_or_else(|| (self.display.render(ty), Vec::new()));
                Some(crate::compile::goals::OpenGoalInfo {
                    goal,
                    binders,
                    holes: vec![val.span()],
                    sub_goals: Vec::new(),
                    refine_template: None,
                })
            }
            _ => None,
        };
        if let Some(info) = open_info {
            // G-01：`example` 没有宇宙参数，签名同样必须先过 elaborate。
            let mut signature =
                match open_signature(&mut self.builder, &[], ty, &self.known, &elab_ctx) {
                    Ok(sig) => sig,
                    Err(e) => {
                        self.out.push_error(idx, e.clone());
                        let mut st = failed_state(DeclKind::Example, None, span, e, idx);
                        st.by_root = by_root.clone();
                        self.decl_states.push(st);
                        return;
                    }
                };
            // **签名里的 hover 行**（A3）：开放练习也是声明，它的签名同样要能
            // hover / 跳转。`env_at` 取 `env_before` —— 签名就是在**本声明入环境
            // 之前** elaborate 的（与 `#check` 同一条口径）。
            self.cmd_hovers.push(CmdHover {
                env_at: c.env_before,
                nodes: std::mem::take(&mut signature.hovers),
                cmd: idx,
            });
            self.ops.push(PendingOp::OpenExercise {
                name: None,
                kind: DeclKind::Example,
                // **R1c-2b（2026-10-05）**：`example` 没有源级宇宙参数，但签名出口
                // 仍可能新加 `u_1`/`u_2`… ✓（同上面两条 ✓）。
                universe: signature.fresh_params.clone(),
                redundant_probes: {
                    {
                        // **T-D8**：开关下**看不见文件声明** ✓（探针仍在真 `builder` 的 DAG 里
                        // elaborate ✓ ⇒ 指针同一性保住 ✓；只是环境里没有文件声明 ✓）。
                        // 用 hide/restore **两个方法**而**不是**闭包 ✗：这里同时借
                        // `&mut self.builder` 与 `&self.known`（**不同字段** ✓）⇒ 闭包会让
                        // `self` 被可变借两次 ✗（round 301 预判 ✓）。
                        let saved = if Self::walk_real_add_enabled() {
                            {
                                Some(self.builder.hide_declars())
                            }
                        } else {
                            {
                                None
                            }
                        };
                        let probes = build_redundant_probes(
                            &mut self.builder,
                            &[],
                            ty,
                            val,
                            &redundant_spans,
                            &self.known,
                            &elab_ctx,
                        );
                        if let Some(s) = saved {
                            {
                                self.builder.restore_declars(s);
                            }
                        }
                        probes
                    }
                },
                env_before: c.env_before,
                declared_ty: Some(signature.declared_ty),
                sig_probe: signature.probe,
                sig_span: signature.span,
                goal: Some(info.goal.clone()),
                binders: info.binders.clone(),
                holes: info.holes,
                sub_goals: info.sub_goals,
                refine_template: info.refine_template,
                by_steps: by_steps.clone(),
                // **与「底」（声明卡片）同源**（2026-10-02 值守第 9 单）：
                // open 声明的根状态就用 `open_goal` 的 `info` ⇒ 顶 ≡ 底 **by construction** ✓
                // （`kernel_phase` 把同一份 `goal`/`binders` 填进 `DeclState` ✓）。
                by_root: Some(ByGoalState {
                    // **折一次**：`info.goal`/绑元类型是**源级渲染**（`And P Q`），而
                    // 这条 `by_root` 还会被 **hover 路径直接渲染**（不经查询层的 fold ✗）
                    // ⇒ 在这里折好 ⇒ wire 与 hover 同一份文本 ✓（折叠幂等 ✓，wire 不变 ✓）。
                    ty: self.display.fold(&info.goal),
                    binders: info
                        .binders
                        .iter()
                        .map(|b| GoalBinder {
                            name: b.name.clone(),
                            ty: self.display.fold(&b.ty),
                        })
                        .collect(),
                }),
                span,
                cmd: idx,
            });
            return;
        }
        self.example_idx += 1;
        let internal_name = format!("_example_{}", self.example_idx);
        let mut hovers = Vec::new();
        match build_example(
            &mut self.builder,
            &internal_name,
            ty,
            val,
            &self.known,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    let mut st = failed_state(DeclKind::Example, None, span, err, idx);
                    st.by_root = by_root.clone();
                    self.decl_states.push(st);
                    return;
                }
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: None,
                    kind: DeclKind::Example,
                    declar: decl,
                    by_steps: by_steps.clone(),
                    by_root: by_root.clone(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                let mut st = failed_state(DeclKind::Example, None, span, e, idx);
                st.by_root = by_root.clone();
                self.decl_states.push(st);
            }
        }
    }

    /// `inductive … end`：整块进 `InductiveTable` 与 env，作为最小增量单元。
    #[allow(clippy::too_many_arguments)]
    fn inductive_block(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        params: &[Binder],
        ty: &Expr,
        constructors: &[CtorDecl],
        recursor: &Option<RecDecl>,
        iota_rules: &[IotaRule],
        span: Span,
    ) {
        let idx = c.idx;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        if trusted {
            // The whole block is the minimal incremental unit: it was
            // kernel-validated together when first checked.
            if skip.is_some_and(|s| s.contains_key(&idx)) {
                return;
            }
            // 失败已经记在会话缓存里（`skip`），这里不重复报错；env 自己持有
            // 声明副本，返回的 `Declar` 只是给内核阶段用的句柄，丢弃即可。
            let mut hovers = Vec::new();
            let mut built: Vec<Declar<'_>> = Vec::new();
            let _ = install_inductive_block(
                &mut self.builder,
                &mut self.known,
                &mut self.inductives,
                prefix_src,
                options,
                &self.ns,
                name,
                params,
                ty,
                constructors,
                recursor.as_ref(),
                iota_rules,
                &mut hovers,
                &mut built,
            );
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Inductive,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        let mut hovers = Vec::new();
        let mut built: Vec<Declar<'_>> = Vec::new();
        match install_inductive_block(
            &mut self.builder,
            &mut self.known,
            &mut self.inductives,
            prefix_src,
            options,
            &self.ns,
            name,
            params,
            ty,
            constructors,
            recursor.as_ref(),
            iota_rules,
            &mut hovers,
            &mut built,
        ) {
            Ok(()) => {
                self.ops.push(PendingOp::InductiveBlock {
                    name: name.to_string(),
                    declars: built,
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: self.builder.declaration_count(),
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Inductive,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
            }
        }
    }

    /// `#check e`：只 elaborate，求值在内核阶段（`env_before` 快照）。
    #[allow(clippy::too_many_arguments)]
    fn check(&mut self, c: &CmdCtx<'_>, expr: &Expr) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let options = local(c.options);
        let env_before = c.env_before;
        let mut hovers = Vec::new();
        match elab_expr(
            &mut self.builder,
            expr,
            &mut ElabScope::new(),
            &no_universe,
            &self.known,
            &mut hovers,
            None,
            None,
            &ElabCtx {
                notations: Some(&self.display),
                prefix_src,
                options,
                inductives: &self.inductives,
                ns: &self.ns,
                defs: &self.defs,
            },
        ) {
            Ok(e) => {
                // **U2（2026-10-05 ✓）**：裸常量（不带 `.{n}` 的 `Ident`）⇒ 带上**签名源文本** ✓
                // （渲染侧用它 ✓，见 `kernel_phase.rs` 的 `PendingOp::Check` 分支 ✓）。
                // ⚠ 只在**渲染**上生效 ✓ —— 判定路径不动 ✓。
                let sig = match expr {
                    Expr::Ident { name, .. } => self
                        .known
                        .get(name.as_str())
                        .and_then(|info| info.signature())
                        .map(str::to_string),
                    _ => None,
                };
                // **R1c-2b（2026-10-05）**：`#check` 也走出口 ✓ ——
                // **对齐 Lean `BuiltinCommand.lean:435`**：
                // `let e ← Term.levelMVarToParam (← instantiateMVars e)` ✓。
                let (e, _) =
                    crate::compile::check::level_exit::discharge_expr(&mut self.builder, e, &[]);
                self.ops.push(PendingOp::Check {
                    expr: e,
                    env_at: env_before,
                    span: expr.span(),
                    cmd: idx,
                    sig,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_before,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => self.out.push_error(idx, e),
        }
    }

    /// `#reduce e`：同 `#check`，内核阶段做化简。
    #[allow(clippy::too_many_arguments)]
    fn reduce(&mut self, c: &CmdCtx<'_>, expr: &Expr) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let options = local(c.options);
        let env_before = c.env_before;
        let mut hovers = Vec::new();
        match elab_expr(
            &mut self.builder,
            expr,
            &mut ElabScope::new(),
            &no_universe,
            &self.known,
            &mut hovers,
            None,
            None,
            &ElabCtx {
                notations: Some(&self.display),
                prefix_src,
                options,
                inductives: &self.inductives,
                ns: &self.ns,
                defs: &self.defs,
            },
        ) {
            Ok(e) => {
                // **R1c-2b（2026-10-05）**：`#reduce` 同 `#check` ✓
                // （**对齐 Lean `BuiltinCommand.lean:457`** ✓）。
                let (e, _) =
                    crate::compile::check::level_exit::discharge_expr(&mut self.builder, e, &[]);
                self.ops.push(PendingOp::Reduce {
                    expr: e,
                    env_at: env_before,
                    span: expr.span(),
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_before,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => self.out.push_error(idx, e),
        }
    }

    /// `#print name`：只记名字指针，打印在内核阶段。
    #[allow(clippy::too_many_arguments)]
    fn print(&mut self, c: &CmdCtx<'_>, name: &str, span: Span) {
        let idx = c.idx;
        // R2：`#print mk` 与 `#print Wrap.mk` 打印同一条声明（别名解析到规范名）；
        // 歧义/未知走各自稳定的错误码，与 `#check` 同源。G-05：命名空间/open
        // 的候选顺序也走这一条（`#print mem` 在 `namespace Set` 里解析到 `Set.mem`）。
        let canonical = match resolve_known(&self.known, &self.ns, name, span) {
            Ok(canonical) => canonical,
            Err(e) => {
                self.out.push_error(idx, e);
                return;
            }
        };
        let ptr = self.builder.name_from_str(&canonical);
        self.ops.push(PendingOp::Print {
            name: canonical,
            ptr,
            span,
            cmd: idx,
        });
    }
}

/// 开练习的签名检查产物（G-01 / WO-004）。
///
/// 值位是 `sorry` 不再让签名免检：签名必须先 elaborate 成内核类型
/// （`elab_expr` 的 `Err` 由调用方走既有失败通道上报），再由内核阶段用
/// [`Self::probe`] 终审「它是不是一个类型 / 是不是 Prop」。
pub(super) struct OpenSignature<'arena> {
    /// 签名 elaborate 后的内核表达式（只用来渲染 `DeclState.ty_text`）。
    pub(super) declared_ty: ExprPtr<'arena>,
    /// 「签名是不是一个类型」的探针：一条**不入环境**的同签名 axiom。
    /// 内核的 `check_declar_info_v` 先 `ensure_sort_v`，消息族与 checked
    /// 路径同源（`Declar::Axiom` 不需要值，正适合签名这种"没有值"的声明）。
    pub(super) probe: Box<Declar<'arena>>,
    /// 诊断 span：**签名**的 AST 范围（G-01 要求报在签名上；G-15 已修，内核 span 本身精确）。
    pub(super) span: Span,
    /// **R1c-2b（2026-10-05）**：签名出口**新加的宇宙参数名** ✓（`u_1`/`u_2`… ✓）。
    /// 调用方把它并进 `PendingOp::OpenExercise.universe` ⇒ `DeclState.universe`
    /// ⇒ judge 合成声明时才知道这些参数 ✓（对齐 Lean 的
    /// `setLevelNames (r.newParamNames …)` ✓，`TermElabM.lean:984-986` ✓）。
    pub(super) fresh_params: Vec<String>,
    /// **签名里每一个子表达式的 hover 行**（A3 的根因，2026-09-26）。
    ///
    /// 以前这个 Vec 是 `open_signature` 的**局部变量**，elaborate 完就丢 ✗ ⇒
    /// 开放练习的**签名**一条 hover 行都没有 ⇒ 在未解出的练习里 hover / F12 /
    /// 高亮 / 引用**全部失效**（学生最常见的状态就是这个 ✗）。判据：
    /// `report.hovers` 里没有任何 span 落在开放练习的签名区间内（实测
    /// `max end offset = 113`，而签名从 116 起）。
    pub(super) hovers: Vec<HoverNode<'arena>>,
}

/// 开练习的签名检查（G-01 / WO-004）：把签名 elaborate 成内核类型并造终审探针。
///
/// 与 checked 路径用**同一套** elaborate 上下文：宇宙参数在作用域里
/// （原来这里传的是空宇宙表 `UnivMap::new()`，`{u}` 签名的 `ty_text`
/// 因此渲染不出来——签名检查顺带把它对齐）。`Err` = 签名 elaborate 不过，
/// 调用方必须把它当失败上报，**不要**登记开放练习。
fn open_signature<'arena>(
    builder: &mut EnvBuilder<'arena>,
    universe: &[String],
    ty: &Expr,
    known: &KnownTable,
    ctx: &ElabCtx<'arena, '_>,
) -> Result<OpenSignature<'arena>, CompileError> {
    let univ = make_univ_map(builder, universe);
    let mut hovers: Vec<HoverNode<'arena>> = Vec::new();
    let declared_ty = elab_expr(
        builder,
        ty,
        &mut ElabScope::new(),
        &univ,
        known,
        &mut hovers,
        None,
        None,
        ctx,
    )?;
    // 探针重新 elaborate 一次签名：`build_axiom` 是现成的**无值**声明构造器，
    // 复用它的宇宙参数登记（`collect_uparams`），免得在这里重造一遍。
    let probe = build_axiom(
        builder,
        SIG_PROBE_NAME,
        universe,
        ty,
        known,
        &mut Vec::new(),
        ctx,
    )?;
    // **R1c-2b（2026-10-05）**：**签名出口** ✓ —— 两次 elaborate 的层元变量各自收口 ✓。
    // ⚠ **两次必须同名同序** ✗：`used` 两次都只含源级 `universe` ⇒ 都从 `u_1` 起
    // （`level_exit::discharge_expr` 的文档 ✓）。
    let (declared_ty, fresh_params) =
        crate::compile::check::level_exit::discharge_expr(builder, declared_ty, universe);
    let (probe, _) = crate::compile::check::level_exit::discharge_declar(builder, probe);
    Ok(OpenSignature {
        declared_ty,
        probe: Box::new(probe),
        span: ty.span(),
        hovers,
        fresh_params,
    })
}

/// 签名探针的内部名（不入环境，不会与用户名字冲突；对照
/// `_soko_redundant_sorry_N`）。
const SIG_PROBE_NAME: &str = "_soko_signature_probe";

/// 「多余的 `sorry`」的 kernel 探针（`docs/design/redundant-sorry.md` §4）：
/// 对每个候选洞，把那个实参从应用 spine 上删掉、按原声明的类型合成一条
/// **不会进入环境**的声明；pass 2 用 `try_check_declar_at` 终审——过了才说明
/// "删掉这行 sorry 就通过"，也就是"它不是你要证的东西"。
/// 造不出来（elab 失败/形状不认识）就跳过：绝不猜。
///
/// **注意（§8.1）**：探针不入环境 ⇒ 它的名字没有 `decl_idx` ⇒
/// `try_check_declar` 内部的 `EnvLimit::ByName(探针名)` 取 0（空环境），
/// 终审必然 `unknown const`。修法见 §8.3（终审显式传 `EnvLimit::ByIndex(env_before)`）。
#[allow(clippy::too_many_arguments)]
fn build_redundant_probes<'arena>(
    builder: &mut EnvBuilder<'arena>,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    spans: &[Span],
    known: &KnownTable,
    ctx: &ElabCtx<'arena, '_>,
) -> Vec<(Declar<'arena>, Span)> {
    let mut probes = Vec::new();
    for (i, span) in spans.iter().enumerate() {
        let Some(modified) = spine_without_arg(val, *span) else {
            continue;
        };
        let mut hovers: Vec<HoverNode<'arena>> = Vec::new();
        let name = format!("_soko_redundant_sorry_{i}");
        if let Ok(declar) = build_def(
            builder,
            &name,
            universe,
            ty,
            &modified,
            known,
            &mut hovers,
            ctx,
        ) {
            // **R1c-2b（2026-10-05）**：探针**不入环境**，但会被内核终审
            // （`kernel_phase` 的 `try_check_declar_at` ✓）⇒ 它的层元变量同样要收口 ✓
            // （否则 `all_uparams_defined` 判拒 ⇒ 假「不是多余的 sorry」✗）。
            let (declar, _) = crate::compile::check::level_exit::discharge_declar(builder, declar);
            probes.push((declar, *span));
        }
    }
    probes
}

/// **逐声明耗时事件**（`SOKO_DECL_PROFILE=1`）：JSON lines 到 stderr。
///
/// 形状（**常设**，`scripts/profile-course.sh` 消费）：
/// `{"soko":"decl","module":…,"name":…,"index":…,"total":…,"ms":…}`
/// —— `index` 是**该声明在它自己文件里的序号** ⇒ 直接支撑"耗时 vs 序号"的
/// O(N²) 判定（假设 A）✓。默认关：开关没开时 `enabled()` 只读一次 `OnceLock`。
pub(crate) mod decl_profile {
    use std::io::Write;
    use std::sync::OnceLock;
    use std::time::Duration;

    static ON: OnceLock<bool> = OnceLock::new();

    pub(crate) fn enabled() -> bool {
        *ON.get_or_init(|| std::env::var_os("SOKO_DECL_PROFILE").is_some())
    }

    /// **只报超过阈值的声明**（仿 Lean 的 `profiler.threshold`，默认 100ms）。
    ///
    /// 为什么必须有：真课程实测 **256160 条**逐声明事件（`judge_infer` 每未命中一次
    /// 就合成一趟 pass，那一趟也逐条打）⇒ 全打出来没法看 ✗。
    /// 出处：<https://vca-epfl.github.io/wiki/lean-profiling/>（"a threshold of at least
    /// 100ms before the result is shown… adjusted by `trace.profiler.threshold`"）。
    /// 设 `SOKO_DECL_PROFILE_MS=0` ⇒ 全打（要画"耗时 vs 序号"的分布时用）。
    pub(crate) fn threshold_ms() -> f64 {
        static MS: OnceLock<f64> = OnceLock::new();
        *MS.get_or_init(|| {
            std::env::var("SOKO_DECL_PROFILE_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0)
        })
    }

    /// 命令的**声明名**（`import`/`open` 之类没有名字 ⇒ 用它的命令种类当名字）。
    pub(crate) fn decl_name(command: &crate::ast::Command) -> &str {
        match command {
            crate::ast::Command::Def { name, .. }
            | crate::ast::Command::Theorem { name, .. }
            | crate::ast::Command::Axiom { name, .. }
            | crate::ast::Command::InductiveBlock { name, .. } => name,
            crate::ast::Command::Example { .. } => "<example>",
            crate::ast::Command::Import { .. } => "<import>",
            _ => "<cmd>",
        }
    }

    pub(crate) fn emit(module: &str, name: &str, index: usize, total: usize, elapsed: Duration) {
        let ms = elapsed.as_secs_f64() * 1000.0;
        if ms < threshold_ms() {
            return;
        }
        let line = format!(
            "{{\"soko\":\"decl\",\"module\":{module:?},\"name\":{name:?},\"index\":{index},\"total\":{total},\"ms\":{ms:.3}}}"
        );
        let mut err = std::io::stderr().lock();
        let _ = writeln!(err, "{line}");
    }
}
