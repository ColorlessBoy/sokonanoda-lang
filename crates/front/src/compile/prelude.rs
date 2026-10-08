//! 可信内置声明（prelude）的安装：Nat 骨架、Eq 三件套与可选性配置。
//!
//! prelude 是「受信任的预置」：安装后不再被内核重查（不进 PendingOp）。
//! 教学文件可以在两种模式下编译（用户要求）：
//! * `PreludeMode::Full` —— 安装全部内置基元（Nat/Eq，若未被文件自带声明占用）；
//! * `PreludeMode::Bare` —— 完全不安装任何东西，课程从零构造一切
//!   （例如自带 `inductive Nat` 块或纯逻辑公理文件）。

use super::elab::{
    build_axiom, build_def, install_inductive_block, params_of_ty, strip_lambdas_n, DefInfo,
    DefTable, ElabCtx, InductiveTable, KnownName, KnownTable,
};
use super::scope::NamespaceScope;
use crate::{Binder, BinderKind, Command, CtorDecl, Expr, SortKind, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, DeclarInfo, ReducibilityHint};
use sokonanoda::expr::BinderStyle;
use sokonanoda::util::ExprPtr;
use std::collections::HashSet;

/// Which trusted base declarations a compilation installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreludeMode {
    /// Install the built-in teaching prelude (Nat + Eq, unless the file
    /// declares its own versions).
    #[default]
    Full,
    /// Install nothing: the file must be self-contained. `1 + 1` and `Nat`
    /// only work if the file provides them.
    Bare,
}

/// Options for one compilation session.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptions {
    pub prelude: PreludeMode,
}

/// Read the file-level prelude directive from `--` comment lines:
/// `-- sokonanoda:prelude none` (or `bare`) selects `PreludeMode::Bare`,
/// `-- sokonanoda:prelude full` selects `Full`. The flag stays declarative:
/// it is a comment, so the file remains a plain text canvas.
pub fn prelude_mode_from_source(src: &str) -> PreludeMode {
    explicit_prelude_mode(src).unwrap_or(PreludeMode::Full)
}

/// 文件**显式**写了 `-- sokonanoda:prelude …` 指令时返回它，否则 `None`。
/// 项目闭包里"没写指令"= 继承入口的模式（设计 §4.6），只有**显式冲突**
/// 才是 `import-prelude-conflict`。
pub fn explicit_prelude_mode(src: &str) -> Option<PreludeMode> {
    for line in src.lines() {
        let trimmed = line.trim_start();
        let Some(comment) = trimmed.strip_prefix("--") else {
            continue;
        };
        let comment = comment.trim();
        let Some(rest) = comment.strip_prefix("sokonanoda:prelude") else {
            continue;
        };
        let value = rest.trim();
        return Some(match value {
            "none" | "bare" => PreludeMode::Bare,
            _ => PreludeMode::Full,
        });
    }
    None
}

/// Trusted equality primitives, written in the teaching syntax itself and
/// installed without re-checking (like the Nat prelude). The signatures match
/// official Lean's `Eq`/`Eq.refl`/`Eq.subst`, so a filled exercise file that
/// uses them still checks in real Lean.
/// The trusted prelude's top-level names (Full mode). Completions material:
/// prelude declarations are trusted installs without `DeclState`s, so the
/// goal view / completion layer needs this list to offer them.
///
/// L1 (`prelude/*.sokonanoda` §3.3) added 30 names: the 28
/// declarations of [`PRELUDE_L1_SRC`] plus the two derived recursors
/// (`And.rec`/`Or.rec`, which `install_inductive_block` generates).
///
/// B8（L-03，2026-09-19；0.61.0 扩族）再加 5 条：`Eq.rec`、由它定义的
/// `Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`（Type 层重写；设计
/// `docs/design/eq-type-level-rewriting.md`）。`Eq.mp`/`Eq.mpr`/`cast` 是
/// **宇宙多态**的（层级算术 `u+1` 落地后，签名与 Lean core 逐字对齐）。
pub const PRELUDE_NAMES: &[&str] = &[
    "Nat",
    "Nat.zero",
    "Nat.succ",
    "Nat.rec",
    "Nat.add",
    "Bool",
    "Bool.true",
    "Bool.false",
    "Bool.rec",
    "Eq",
    "Eq.refl",
    "Eq.subst",
    // ---- L1: 真伪 (B1/B2) ----
    "True",
    "True.intro",
    "False",
    "False.rec",
    "False.elim",
    // ---- L1: 且 (B3) ----
    "And",
    "And.intro",
    "And.left",
    "And.right",
    "And.elim",
    "And.rec",
    // ---- L1: 或 (B4) ----
    "Or",
    "Or.inl",
    "Or.inr",
    "Or.elim",
    "Or.rec",
    // ---- L1: 非 (B5) ----
    "Not",
    "Not.intro",
    "Not.elim",
    "absurd",
    // ---- L1: 不相等 (L2.3，`≠` 的目标) ----
    "Ne",
    "Ne.intro",
    // ---- L1: 当且仅当 (B6) ----
    "Iff",
    "Iff.intro",
    "Iff.mp",
    "Iff.mpr",
    "Iff.refl",
    "Iff.symm",
    "Iff.trans",
    // ---- L1: Eq 引理 (B7) ----
    "Eq.symm",
    "Eq.trans",
    "congrArg",
    // ---- L1: Eq 大消去 / Type 层重写 (B8) ----
    "Eq.rec",
    "Eq.ndrec",
    "Eq.mp",
    "Eq.mpr",
    "cast",
    // ---- ST2: 商类型 Quot（v0.77.0，内核内建的声明种类）----
    "Quot",
    "Quot.mk",
    "Quot.lift",
    "Quot.ind",
    "Quot.sound",
    // ---- G-75: 商的反射方向（sound 版本：`r` 是等价关系）----
    "Quot.exact",
    // ---- G-74: 排中律（B10，v0.81.0）----
    "Classical.em",
    "Classical.byContradiction",
];

/// Full 模式下**永不**让位的 prelude 名字（`Nat`/`Bool` 家族）。
///
/// 与 [`PRELUDE_NAMES`] 的分工（设计 §2.3-2）：`PRELUDE_NAMES` 是补全/材料
/// 列表（含 L1），`PRELUDE_NEVER_YIELDS` 只用于 `check_name_collisions` 的
/// 豁免面。L1 名字按**族**合法让位（文件自己声明 ⇒ 整族来自文件），所以它们
/// **不能**进这个列表——否则两个模块各自声明 `True` 就不再报友好的
/// `import-name-collision`，退化成内核裸错。
pub const PRELUDE_NEVER_YIELDS: &[&str] = &[
    "Nat",
    "Nat.zero",
    "Nat.succ",
    "Nat.rec",
    "Nat.add",
    "Bool",
    "Bool.true",
    "Bool.false",
    "Bool.rec",
];

pub const PRELUDE_EQ_SRC: &str = include_str!("../../../../prelude/Eq.sokonanoda");

/// L1 的规范源文本（设计 `prelude/*.sokonanoda` 附录 A）：
/// Lean core 的逻辑与等式骨架，用教学语法逐字写出来，作为**受信任安装**
/// （与 `Nat`/`Bool`/`Eq` 同一条路径：走 `build_def`/`install_inductive_block`
/// 与用户声明同一个 elaborator，但不再被内核重查）。
///
/// 与课程库 `courses/set-theory/lib/Logic.sokonanoda` 的差异只有两处（附录 A）：
/// **`And` 由 axiom 族改成真归纳块**、**`Or` 的构造子由裸名 `inl`/`inr`
/// 改成 `Or.inl`/`Or.inr`**。文件顺序必须满足依赖：
/// B1/B2 → B3 → B4 → B5 → B6 → B7 → B8（`And.elim` 在 `And.left/right` 之后、
/// `Iff` 在 `And` 之后、`Eq.mp`/`Eq.mpr` 在 `Eq.rec` 之后）。
///
/// 三条硬约束下的定形（设计 §1.2）：`{u}` 是唯一宇宙 binder（G-14）；
/// `axiom` 一律柯里化（G-13）；构造子写点号名（G-02 的可行解）；
/// 隐式实参不自动插入，所以签名显式给全参数。
///
/// **B8（`Eq.rec`/`Eq.ndrec`/`Eq.mp`/`Eq.mpr`/`cast`，L-03，2026-09-19）**：
/// Eq prelude 的 `Eq` 是**公理**（不是归纳块），所以内核不会为它派生消去子
/// （`Eq.rec` 实测 `elab-unknown-constant`）；而本语言的 `inductive` 头部今天
/// 不吃宇宙 binder，没法把 `Eq` 立成宇宙多态的归纳块。因此 B8 走**公理**：
/// `axiom Eq.rec {u, v}` 的签名与 Lean core 的 `Eq.rec.{u, v}` 逐字同形
/// （`docs/design/eq-type-level-rewriting.md` §3 有实测与取舍）。
///
/// 其余四条都是 `def`（不新增信任面）：`Eq.ndrec` 是 Lean core 的
/// **非依赖消去子**（motive 不吃证明，Lean 里它是 `abbrev`），
/// `Eq.mp`/`Eq.mpr`/`cast` 是 `Eq.rec` 在 `Sort u` 上的实例——Lean core 里
/// `cast h a` 就是 `Eq.mp h a`（`h.rec a`），三条签名逐字对齐：
/// `{α β : Sort u} (h : @Eq.{u+1} (Sort u) α β)`。**层级算术 `u+1` 落地后**
/// （`docs/design/type-level-syntax.md` §5）它们才是宇宙多态的；此前是
/// Type 0 实例（0.60.0 的残留边界，设计 §4-1 已销账）。
pub const PRELUDE_L1_SRC: &str = include_str!("../../../../prelude/L1.sokonanoda");

/// **ST2（v0.77.0）**：`Quot` 五条的类型**源文本** —— 由 [`install_quot_src`] 交给
/// **前端自己的 elaborator** 建成 `Declar::Quot`（四条）+ `Declar::Axiom`（`Quot.sound`）。
///
/// **为什么写成源文本**（本环节最贵的一课，实测踩了 10+ 轮）：内核
/// `crates/kernel/src/quot.rs::check_quot` 里那些 `mk_var(n)` 的索引**与"按
/// de Bruijn 深度推"的直觉不一致** —— 手搓 `EnvBuilder` 表达式时结构"看起来对"
/// （`#check Quot.{1}` 甚至能渲染成正确形状），但 `def q … := Quot.{1} α r`
/// 判红「期望 `… $0 …`，实际 `… $2 …`」✗。交给前端 elaborator 就没有这个问题
/// （它就是平时建 `axiom`/`def` 类型的那条路）✓，且类型文本是**真的**、与
/// [`prelude_source`] 同源、F12 直接可用 ✓。
///
/// 形状（与内核期望**语义一致**：`{A : Sort u}` 隐式 / 其余显式 / `Quot.lift` 带 `{v}`）：
/// `Quot.sound` 是**唯一**的公理（Lean TPiL §12.4：`Quot`/`Quot.mk`/`Quot.ind`/`Quot.lift`
/// 属逻辑框架，只有 `Quot.sound` 是公理 ✓）。
///
/// 判据不是"文本看起来对不对"，而是**归约**：`Quot.lift`/`Quot.ind` 在 `Quot.mk`
/// 上必须**算得出来**（见 `crates/front/src/compile/tests.rs` 的 ST2 判据）。
pub const QUOT_TYPES_SRC: &str = include_str!("../../../../prelude/Quot.sokonanoda");

/// **E3（2026-10-08）：prelude 的运行时覆盖**（默认**关** ✓）。
///
/// **动机**（用户 P8 的"可修改"那一半）：学生用 release VSIX 时**没有 cargo**
/// ⇒ 只有运行时覆盖才能"**改 prelude 就见效**" ✓（E1 已经把真相搬进
/// `prelude/*.sokonanoda`，但那只对**仓库/贡献者**路径有效）。
///
/// **用法**：`SOKO_PRELUDE_DIR=<目录>` ⇒ 目录里的 `Eq.sokonanoda` /
/// `L1.sokonanoda` / `Quot.sokonanoda` **按名覆盖**（**缺哪个用内置的哪个** ✓）。
///
/// **四条不变量（缺一不可）**：
/// ① **默认关**：不设环境变量 ⇒ 三条源**逐字节等于内置**（自足分发不变 ✓）；
/// ② **覆盖内容哈希进缓存键**（`cache` 的 `prelude_override_state()`）——
///    换了内容却命中旧条目 = 拿旧 prelude 的答案 ✗（红线）；
/// ③ **畸形覆盖不 panic**：解析不过的覆盖**不生效**（该条回落内置）且原因记在
///    [`prelude_override_error`]，由 CLI/LSP 报成诊断或退出码 ✓
///    （`install_*_prelude` 里的 `expect("… parses")` 因此**永远不会**被覆盖触发 ✓）；
/// ④ **受信任安装语义不变**：覆盖的仍然是 prelude，走**同一条**安装路
///    （`install_eq_prelude`/`install_l1_prelude`/`install_quot_src` ✓），`--bare` 不受影响 ✓。
pub struct PreludeOverride {
    pub eq: String,
    pub l1: String,
    pub quot: String,
    /// 覆盖内容的指纹（**0 = 没有覆盖** ⇒ 键与今天逐字相同 ✓）。
    pub hash: u64,
    /// 畸形覆盖的原因（`None` = 干净）；**不 panic**，只记下来 ✓。
    pub error: Option<String>,
}

/// **试装**一份覆盖源（第二道验证 · 见 [`load_prelude_override`] 的 ③）：
/// 在一次性 scratch 环境里按**真装的顺序**走一遍（Nat → Bool → Eq → 该段 ✓），
/// 把 panic 转成 `Err` ✓。
///
/// ⚠ **为什么可以在这里 `catch_unwind`**：scratch 环境**随后整份丢弃** ⇒ 半装坏的
/// 中间态不会进真环境 ✓（这正是内核那条"`quiet_catch` 不可嵌套"纪律要防的东西 ✓）。
fn trial_install(text: &str, file: &str) -> Result<(), String> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let arena = stumpalo::Arena::new();
        let mut builder = EnvBuilder::new(arena.as_arena_ref(), Default::default());
        let mut known = KnownTable::new();
        let mut inductives = InductiveTable::new();
        let mut defs = DefTable::new();
        let empty: HashSet<String> = HashSet::new();
        install_prelude(&mut builder, &mut known, &mut inductives);
        install_bool_prelude(&mut builder, &mut known, &mut inductives);
        match file {
            "Eq.sokonanoda" => {
                // Eq 段：真装走 `install_eq_prelude`，它读的是**生效源** ⇒ 这里临时
                // 换成试装文本（进程级覆盖只读一次，试装期还没定下来 ✓）。
                install_eq_prelude_src(&mut builder, &mut known, text, &empty)
            }
            _ => {
                install_eq_prelude_src(&mut builder, &mut known, PRELUDE_EQ_SRC, &empty);
                install_l1_prelude_src(
                    &mut builder,
                    &mut known,
                    &mut inductives,
                    &mut defs,
                    text,
                    QUOT_TYPES_SRC,
                    &empty,
                )
            }
        }
    }));
    result.map_err(|_| "试装时内核拒绝（前向引用/类型不成立之类）".to_string())
}

/// 读一个目录里的三份覆盖（**纯函数** ⇒ 可单测 ✓）。缺文件 = 用内置 ✓。
pub fn load_prelude_override(dir: &std::path::Path) -> PreludeOverride {
    let mut out = PreludeOverride {
        eq: PRELUDE_EQ_SRC.to_string(),
        l1: PRELUDE_L1_SRC.to_string(),
        quot: QUOT_TYPES_SRC.to_string(),
        hash: 0,
        error: None,
    };
    let mut parts: Vec<String> = Vec::new();
    for (file, slot) in [
        ("Eq.sokonanoda", 0usize),
        ("L1.sokonanoda", 1usize),
        ("Quot.sokonanoda", 2usize),
    ] {
        let path = dir.join(file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // 缺文件 = 用内置 ✓（"只覆盖想改的那一份"）
        };
        // ③ **畸形覆盖不 panic**：两道验证，任何一道不过 ⇒ 该条**不生效** ✓。
        // 第一道：**解析**（语法）。
        if let Err(err) = crate::parse(&text) {
            out.error = Some(format!(
                "prelude 覆盖 `{}` 解析失败（**不生效**，回落内置）：{}",
                path.display(),
                err.message
            ));
            continue;
        }
        // 第二道：**装得上**（elaborate）。只验解析是**不够**的 ✗ —— 解析过但装不上的
        // 覆盖（典型：前向引用，例如把 `True.intro` 的类型写成**后面**族才有的名字）
        // 会在 `install_l1_command` 的 `expect` 上 **panic** ✗（实测踩到）。
        // 做法：在**一次性的 scratch 环境**里试装一遍（顺序与真装一致：Nat/Bool/Eq/L1 ✓），
        // 用 `catch_unwind` 把 panic 变成**可报的错** ✓ —— scratch 环境随后整份丢弃，
        // 所以"半装坏"的中间态**不会**污染真环境 ✓。
        if let Err(err) = trial_install(&text, file) {
            out.error = Some(format!(
                "prelude 覆盖 `{}` 装不上（**不生效**，回落内置）：{err}",
                path.display()
            ));
            continue;
        }
        parts.push(format!("{file}\0{text}"));
        match slot {
            0 => out.eq = text,
            1 => out.l1 = text,
            _ => out.quot = text,
        }
    }
    if !parts.is_empty() {
        out.hash = crate::compile::cache::prelude_override_hash(&parts.join("\0"));
    }
    out
}

// **装载期重入闸**（E3 最贵的一课，实测：CLI 一启动就**死锁** ✗）。
//
// 为什么必须有：覆盖的装载里要**试装**（`trial_install` ⇒ `crate::parse` ⇒ …），
// 而解析路上会读到 `prelude_source`（记法表 ✓）—— 那条路**又**要读覆盖
// ⇒ **重入正在初始化的 `OnceLock`** ⇒ `Once::wait` **永久阻塞** ✗
// （`sample` 栈：`prelude_override_error → OnceLock::initialize → load_prelude_override
// → Once::wait` ✓）。修法：装载期间**一律回落到内置** ✓（试装本来就用显式源 ✓，
// 所以语义不受影响；真装发生在装载**之后** ⇒ 读到的是已发布的覆盖 ✓）。
thread_local! {
    static LOADING_OVERRIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 进程级覆盖（`OnceLock`：环境变量只读一次 ✓；`None` = 没设 ⇒ 零开销 ✓）。
fn prelude_override() -> Option<&'static PreludeOverride> {
    static OVERRIDE: std::sync::OnceLock<Option<PreludeOverride>> = std::sync::OnceLock::new();
    // 装载期（含试装）内的任何一次读取 ⇒ 回落内置，**绝不重入** ✓。
    if LOADING_OVERRIDE.with(std::cell::Cell::get) {
        return None;
    }
    OVERRIDE
        .get_or_init(|| {
            LOADING_OVERRIDE.with(|f| f.set(true));
            let loaded = load_prelude_override_from_env();
            LOADING_OVERRIDE.with(|f| f.set(false));
            loaded
        })
        .as_ref()
}

/// 真的去读环境变量 + 装载（**只在 [`prelude_override`] 的初始化里调用** ✓）。
fn load_prelude_override_from_env() -> Option<PreludeOverride> {
    {
        {
            let dir = std::env::var_os("SOKO_PRELUDE_DIR")?;
            let dir = std::path::PathBuf::from(dir);
            let loaded = load_prelude_override(&dir);
            if let Some(err) = &loaded.error {
                // 可见信号（③）：不 panic，但**必须看得见** ✓。
                eprintln!("[sokonanoda] {err}");
            }
            Some(loaded)
        }
    }
}

/// 畸形覆盖的原因（`None` = 没有覆盖 / 覆盖干净）——CLI 用它定退出码 ✓。
pub fn prelude_override_error() -> Option<&'static str> {
    prelude_override().and_then(|o| o.error.as_deref())
}

/// 覆盖指纹（`0` = 没覆盖）——缓存键用 ✓。
pub fn prelude_override_state() -> u64 {
    prelude_override().map(|o| o.hash).unwrap_or(0)
}

/// **Eq 段**的生效源文本（覆盖优先；下同）。
pub fn prelude_eq_src() -> &'static str {
    match prelude_override() {
        Some(o) => o.eq.as_str(),
        None => PRELUDE_EQ_SRC,
    }
}

/// **L1 段**的生效源文本。
pub fn prelude_l1_src() -> &'static str {
    match prelude_override() {
        Some(o) => o.l1.as_str(),
        None => PRELUDE_L1_SRC,
    }
}

/// **Quot 段**的生效源文本。
pub fn quot_types_src() -> &'static str {
    match prelude_override() {
        Some(o) => o.quot.as_str(),
        None => QUOT_TYPES_SRC,
    }
}

/// **A4（2026-09-26 用户报告第 4 条）**：prelude 的**只读源文本** —— 编辑器要
/// "跳进 prelude"就得有一份能打开的源 ✓。
///
/// 它**不是**新真相：就是内嵌常量本身（`PRELUDE_EQ_SRC` + `PRELUDE_L1_SRC`，
/// 与真正喂进编译的是**同一份字节** ✓）。`OnceLock` 缓存：拼一次。
pub fn prelude_source() -> &'static str {
    static SRC: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    static BUILTIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    // **E3**：装载期（试装的解析路上会走到这里）**必须**用内置视图 —— 既避免重入
    // 正在初始化的覆盖 `OnceLock`（死锁 ✗），也保证这份缓存不会被"装载中的半态"
    // 污染 ✓（两个 `OnceLock` 各管一份：内置的与生效的 ✓）。
    if LOADING_OVERRIDE.with(std::cell::Cell::get) {
        return BUILTIN
            .get_or_init(|| format!("{PRELUDE_EQ_SRC}\n{PRELUDE_L1_SRC}\n{QUOT_TYPES_SRC}"));
    }
    // 走**生效**的三段（没设覆盖时逐字节等于内置 ✓）。
    SRC.get_or_init(|| {
        format!(
            "{}\n{}\n{}",
            prelude_eq_src(),
            prelude_l1_src(),
            quot_types_src()
        )
    })
}

/// prelude 名字 → 它在 [`prelude_source`] 里的**真 span**。
///
/// `None` = 那份源里**没有它的文字**：`Nat`/`Bool` 家族 9 个名字是 **Rust AST
/// 手搓**的（`Span::default()`），**今天确实没有定义位置** ⇒ 返回 `None`，
/// 调用方**不编造位置**（`docs/design/notation-subset.md` §88-105 的先例 ✓）。
///
/// span 来自**真 parser**（`check::top_level_def_spans` ✓，含归纳块的构造子与
/// 消去子）⇒ 与判定同源，**不是文本比对** ✓。整份源 parse 一次并缓存。
pub fn prelude_def_span(name: &str) -> Option<Span> {
    static SPANS: std::sync::OnceLock<std::collections::HashMap<String, Span>> =
        std::sync::OnceLock::new();
    let spans = SPANS.get_or_init(|| {
        crate::parse(prelude_source())
            .map(|file| super::check::top_level_def_spans(&file))
            .unwrap_or_default()
    });
    if let Some(span) = spans.get(name) {
        return Some(*span);
    }
    // **派生名**（`And.rec` / `Or.rec`）：递归子是归纳块**自动派生**的，源里只写了
    // 块头 + 构造子 ⇒ 源里没有它们的文字。退到**所属归纳块**的声明位置 ✓ ——
    // 这是**真话**（递归子就是那个块派生的 ✓），不是编一个位置 ✗。
    // `Nat.rec` 这种连块头都没有源文字的，退不到 ⇒ 仍然 `None` ✓。
    let head = name.rsplit_once('.').map(|(h, _)| h)?;
    spans.get(head).copied()
}

/// [`prelude_source`] 落成**真实文件**的路径（**幂等**：内容一样就不重写）。
///
/// **路线取舍（A4，2026-09-26 ✓）**：Lean 4 的做法就是"工具链里有一份**真源
/// 文件**"（`Init/Prelude.lean` ✓）；而本仓库**只有 `file:` 一种 URI 形态**
/// （零 `TextDocumentContentProvider`、零自定义 scheme）。虚拟文档要同时改
/// LSP 与扩展两侧（含 stub 宿主）✗，真文件只需这一处 + 一个 `file:` 位置 ✓。
/// 落在**平台缓存目录**（`compile::cache::root()` ✓，`SOKONANODA_CACHE_DIR`
/// 可重定向）—— **不落工作区** ✓（不污染用户的树、不进 `build`）。
/// 缓存被禁用（`root()` = `None`）⇒ 返回 `None` ⇒ 调用方**不编造位置** ✓。
///
/// 实测前提：这份源作为**普通文档**编译是**干净**的（35 条声明 / 0 诊断 ✓）
/// ⇒ 在编辑器里打开它不会满屏红 ✓。
pub fn prelude_source_path() -> Option<std::path::PathBuf> {
    let src = prelude_source();
    // ⓪ **仓库里的真源**（**E1，2026-10-08**）：检出仓库时 F12 直接落到
    //    `<仓库>/prelude/Prelude.sokonanoda` —— 那份**就是**三段真源的合并视图
    //    （`include_str!` 的同一个字节），学生打开的是**真文件**、改它**真的会改行为** ✓
    //    （以前落到缓存副本 ⇒ 改了没用 ✗，正是用户 2026-10-06 报的那条）。
    //    ⚠ `CARGO_MANIFEST_DIR` 是**编译期**路径 ⇒ 发布产物（VSIX / 缓存二进制）在
    //    用户机器上**不存在** ⇒ `is_file()` 为假 ⇒ 落到下面两条兜底 ✓（发布形态不变 ✓）。
    let repo_view = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("prelude")
        .join("Prelude.sokonanoda");
    if repo_view.is_file() {
        return Some(repo_view);
    }
    // ① 缓存目录（**首选**：路径稳定 ⇒ 编辑器里的打开文档/书签不会漂）；
    // ② 系统临时目录（**兜底**：缓存被禁用或**不可写**时——例如受限沙箱、
    //    只读 HOME——仍然要能给一个**真实存在**的位置 ✓。临时目录是易失的，
    //    但它们本来就每次重新落盘（幂等 ✓），代价只是路径不如缓存稳定）。
    // 两处都写不进去 ⇒ `None` ⇒ 调用方**不编造位置** ✓。
    let candidates = super::cache::root()
        .map(|d| d.join("prelude").join("Prelude.sokonanoda"))
        .into_iter()
        .chain(std::iter::once(
            std::env::temp_dir()
                .join("sokonanoda-prelude")
                .join("Prelude.sokonanoda"),
        ));
    for path in candidates {
        // 幂等：内容一致就不重写（避免每次 F12 都动一次 mtime ⇒ 编辑器反复重载）。
        if matches!(std::fs::read_to_string(&path), Ok(old) if old == src) {
            return Some(path);
        }
        let written = path
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .ok()
            .flatten()
            .and_then(|()| std::fs::write(&path, src).ok());
        if written.is_some() {
            return Some(path);
        }
    }
    None
}

/// 让位的粒度 = **族**，族之间按依赖做闭包让位（设计 §2.2）。
///
/// 一个族要么整族来自 prelude，要么整族来自文件——避免"prelude 的 `And`
/// + 文件的 `And.left`"这种静默不一致。
///
/// `deps` 里的族一旦被文件占用，本族也必须让位：它们的定义体直接引用被依赖
/// 的名字（`Not.elim` 用 `False.elim`、`Iff.mp` 用 `And.left`、`Eq.symm`
/// 用 `Eq.subst`），否则 prelude 源文本自己就 elaborate 不过。
pub(crate) struct PreludeFamily {
    /// 族名（诊断/测试用；`B7` 额外依赖 Eq prelude，见 [`L1_FAMILIES`]）。
    pub name: &'static str,
    /// 本族的全部顶层名字（含派生递归子，用于触发判定）。
    pub names: &'static [&'static str],
    /// 被让位时本族也必须让位的族名。
    pub deps: &'static [&'static str],
}

/// L1 的族表（设计 §2.2 的 B1–B7 + L-03 的 B8 + L2.3 的 B9）。`B7`/`B8`/`B9` 依赖 **Eq prelude**：
/// `Eq` 被占用时 `install_eq_prelude` 整体不装，`Eq.symm`/`Eq.trans`/`congrArg`
/// 与 `Eq.rec`/`Eq.mp`/`Eq.mpr` 的定义体引用的 `Eq.subst`/`Eq` 就不存在，
/// 所以它们必须一起让位。
pub(crate) const L1_FAMILIES: &[PreludeFamily] = &[
    PreludeFamily {
        name: "B1",
        names: &["True", "True.intro"],
        deps: &[],
    },
    PreludeFamily {
        name: "B2",
        names: &["False", "False.rec", "False.elim"],
        deps: &[],
    },
    PreludeFamily {
        name: "B3",
        names: &[
            "And",
            "And.intro",
            "And.left",
            "And.right",
            "And.elim",
            "And.rec",
        ],
        deps: &[],
    },
    PreludeFamily {
        name: "B4",
        names: &["Or", "Or.inl", "Or.inr", "Or.elim", "Or.rec"],
        deps: &[],
    },
    PreludeFamily {
        name: "B5",
        names: &["Not", "Not.intro", "Not.elim", "absurd"],
        deps: &["B2"],
    },
    PreludeFamily {
        name: "B6",
        names: &[
            "Iff",
            "Iff.intro",
            "Iff.mp",
            "Iff.mpr",
            "Iff.refl",
            "Iff.symm",
            "Iff.trans",
        ],
        deps: &["B3"],
    },
    PreludeFamily {
        name: "B7",
        names: &["Eq.symm", "Eq.trans", "congrArg"],
        deps: &["EQ"],
    },
    PreludeFamily {
        name: "B8",
        names: &["Eq.rec", "Eq.ndrec", "Eq.mp", "Eq.mpr", "cast"],
        deps: &["EQ"],
    },
    // B9（L2.3，2026-09-19）：`≠` 的目标常量。定义体同时用 `Eq.{u}` 与 `False`
    // ⇒ 依赖 **EQ 与 B2**（与 B5 的 `Not` 依赖 B2 同一个理由：定义体引用的族
    // 一旦让位，本族也必须让位，否则报「unknown identifier `False`」）。
    PreludeFamily {
        name: "B9",
        names: &["Ne", "Ne.intro"],
        deps: &["B2", "EQ"],
    },
    // B10（G-74，0.81.0）：排中律。用 `Or`（B4）与 `Not`（B5），而 `Not` 依赖 B2
    // ⇒ 两条都写进 deps（依赖一旦让位本族也必须让位）。
    PreludeFamily {
        name: "B10",
        names: &["Classical.em", "Classical.byContradiction"],
        deps: &["B2", "B4", "B5"],
    },
];

/// 一个族名（`B1`…`B9`）是否必须让位：它自己或它的依赖被 `taken` 命中。
fn family_yields(family: &PreludeFamily, taken: &HashSet<String>) -> bool {
    family.names.iter().any(|name| taken.contains(*name))
        || family.deps.iter().any(|dep| {
            dep_is_taken(dep, taken)
                || L1_FAMILIES
                    .iter()
                    .find(|f| f.name == *dep)
                    .is_some_and(|f| family_yields(f, taken))
        })
}

/// `EQ` 不是 L1 族，它是 [`PRELUDE_EQ_SRC`] 的族名（`install_eq_prelude`
/// 的 all-or-nothing 口径）。`taken` 命中三名之一 ⇒ B7 一起让位。
fn dep_is_taken(dep: &str, taken: &HashSet<String>) -> bool {
    if dep == "EQ" {
        return ["Eq", "Eq.refl", "Eq.subst"]
            .iter()
            .any(|name| taken.contains(*name));
    }
    false
}

thread_local! {
    /// 重入闸（as-built，设计 §1.2 之外的**新发现**）：装 L1 的 `And` 归纳块时，
    /// `install_inductive_block` 要用 `large_elim_test_mirror` 问内核「字段类型
    /// 是不是 Prop」（`field_sort_via_kernel` → `judge_infer` → **内层
    /// `compile_fol_with`**）。内层编译又会装一遍 L1 的 `And` ⇒ 无限递归
    /// （实测：`stack overflow, SIGABRT`）。
    ///
    /// 计数 > 0 时 `install_l1_prelude` 直接返回：内层编译的环境里没有 L1。
    /// 这是安全的，因为 L1 安装期间的 kernel 探针只问「`Prop` 参数是不是
    /// `Prop`」（`field_sort_via_kernel` 的 binder 全部来自归纳块自己的参数，
    /// 类型是内建的 `Prop`），不需要任何 L1 名字。用户代码触发的内层编译
    /// （depth 0）照常拿完整 L1。
    static L1_INSTALL_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// 一个名字属于哪个 L1 族（`None` = 不是 L1 名字）。建议材料层用它与
/// [`family_yields_by_name`] 复现同一条让位规则（`goals.rs`）。
pub(crate) fn l1_family_of(name: &str) -> Option<&'static PreludeFamily> {
    L1_FAMILIES
        .iter()
        .find(|family| family.names.contains(&name))
}

/// 名字 `name` 所属的族是否因 `taken` 而让位。`name` 不是 L1 名字时返回
/// `false`（调用方自行判定归属）。
pub(crate) fn family_yields_by_name(name: &str, taken: &HashSet<String>) -> bool {
    l1_family_of(name).is_some_and(|family| family_yields(family, taken))
}

/// 重入闸：见 [`L1_INSTALL_DEPTH`]。
///
/// 装 L1：按族顺序逐条 install，命中让位的族整体跳过。
///
/// 安装走**与用户声明同一条 elaborator**（`build_axiom`/`build_def`/
/// `install_inductive_block`），不是只走 `build_axiom`——设计 §4.1 的
/// 主要风险点。
pub(crate) fn install_l1_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    taken: &HashSet<String>,
) {
    // **E3**：真装走**生效源**（有覆盖就是覆盖 ✓）；试装（`trial_install`）走 `_src` ✓。
    install_l1_prelude_src(
        builder,
        known,
        inductives,
        defs,
        prelude_l1_src(),
        quot_types_src(),
        taken,
    );
}

/// [`install_l1_prelude`] 的**显式源**版本（E3 的试装用它 ✓）。
fn install_l1_prelude_src<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    src: &str,
    quot_src: &str,
    taken: &HashSet<String>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    if L1_INSTALL_DEPTH.with(|d| d.get()) > 0 {
        return; // 见 `L1_INSTALL_DEPTH`：内层编译不再装 L1
    }
    let Ok(file) = crate::parse(src) else {
        panic!("L1 prelude source parses");
    };
    let options = CompileOptions::default();
    L1_INSTALL_DEPTH.with(|d| d.set(d.get() + 1));
    for family in L1_FAMILIES {
        if family_yields(family, taken) {
            continue;
        }
        for command in &file.commands {
            if !command_belongs_to(command, family) {
                continue;
            }
            install_l1_command(builder, known, inductives, defs, &options, command);
        }
    }
    L1_INSTALL_DEPTH.with(|d| d.set(d.get() - 1));
    // ST2：商类型（**不是** L1 的源级命令，见 `install_quot`）。
    install_quot_src(builder, known, quot_src, taken);
}

/// **本条命令之前**的 prelude 源文本（不含本条 ✓）—— 慢路重跑要用它。
///
/// * `PRELUDE_L1_SRC` 是 `&'static str` ⇒ 切出来的子串也是 `'static` ✓（借用不成问题）；
/// * 切点用**命令的 span 起点**（一定落在字符边界上 ✓；万一越界/非边界 ⇒ 退回 `""`，
///   退化的只是"这一条判定不参与比对"，**不会 panic** ✓）；
/// * 空串是合法输入 ✓（第一条命令之前就是空 ✓）—— 与改动前的行为逐字相同 ✓。
fn prelude_prefix_before(command: &Command) -> &'static str {
    let start = command.span().start.offset;
    prelude_l1_src().get(..start).unwrap_or("")
}

/// 这条命令是不是本族的（按顶层名字判定；`ctor`/`rec` 归它们的归纳块）。
fn command_belongs_to(command: &Command, family: &PreludeFamily) -> bool {
    match command {
        Command::Axiom { name, .. } | Command::Def { name, .. } => {
            family.names.contains(&name.as_str())
        }
        Command::InductiveBlock { name, .. } => family.names.contains(&name.as_str()),
        _ => false,
    }
}

/// 装一条 L1 命令。axiom 走 `build_axiom`，def 走 `build_def`，归纳块走
/// `install_inductive_block`（构造子与派生递归子由它一起登记）。
fn install_l1_command<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
    defs: &mut DefTable,
    options: &CompileOptions,
    command: &Command,
) {
    // `prefix_src` = **本条命令之前**的 prelude 源文本（**E4 第二步，2026-10-08**）。
    //
    // 以前这里是 `""`，理由（原文）："L1 的签名不依赖文件前缀 …… 安装期间的 kernel
    // 探针只需内建的 `Prop`（`L1_INSTALL_DEPTH` 挡掉内层重入 ⇒ 内层环境里没有 L1
    // 名字也不影响）"。**判定路径本身不受影响** ✓ —— 变的是**影子档**：它要把就地路
    // 与慢路**逐字比**，而慢路（`judge_infer`）是"重跑前缀"⇒ 前缀为空时它手里没有
    // `And`/`Or` ⇒ 结构性 `None` ⇒ 只能**排除不比** ✗（`elab.rs` 那条
    // `prelude_install_active()` 早退，E4 要撤的就是它）。
    //
    // E1 之后 prelude 是**普通源文本**（`PRELUDE_L1_SRC` 是 `&'static str` ✓）⇒
    // 把"本条之前"那一段交出去，慢路就**真的能重跑**这段 prelude ✓ ⇒ 安装期的判定
    // **也能比**了 ✓。G-05 不变：prelude 永远在根命名空间、没有 `open` ✓。
    let ns = NamespaceScope::new();
    let empty_defs: DefTable = DefTable::new();
    let ctx = ElabCtx {
        notations: None,
        prefix_src: prelude_prefix_before(command),
        options,
        inductives,
        ns: &ns,
        defs: &empty_defs,
    };
    let mut hovers = Vec::new();
    match command {
        Command::Axiom {
            name, universe, ty, ..
        } => {
            let decl = build_axiom(builder, name, universe, ty, known, &mut hovers, &ctx)
                .expect("L1 prelude axiom elaborates");
            builder
                .add_declar(decl)
                .expect("duplicate L1 prelude axiom");
            known.insert(
                name.clone(),
                KnownName::Decl {
                    universes: universe.clone(),
                    implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                    explicit_arity: crate::compile::elab::explicit_arity(ty),
                    signature: Some(crate::proof::decl_signature(ty)),
                },
            );
        }
        Command::Def {
            name,
            universe,
            ty,
            val,
            ..
        } => {
            let decl = build_def(builder, name, universe, ty, val, known, &mut hovers, &ctx)
                .expect("L1 prelude definition elaborates");
            builder
                .add_declar(decl)
                .expect("duplicate L1 prelude definition");
            known.insert(
                name.clone(),
                KnownName::Decl {
                    universes: universe.clone(),
                    implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                    explicit_arity: crate::compile::elab::explicit_arity(ty),
                    signature: Some(crate::proof::decl_signature(ty)),
                },
            );
            // 源级 delta 表：`by` 引擎靠它看穿 `Not`/`Iff` 这类 **def** 头
            // （`intro x` 在 `¬ A` 目标上、`apply h` 在 `h : A ⊆ B` 上都要它）。
            defs.insert(name.clone(), {
                let params = params_of_ty(ty);
                DefInfo {
                    universes: universe.clone(),
                    implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                    // 与 walk.rs 同一口径（G-72）：剥的层数 = 声明参数个数。
                    body: strip_lambdas_n(val, params.len()),
                    telescope_arity: crate::compile::elab::telescope_arity_of_ty(ty),
                    params,
                }
            });
        }
        Command::InductiveBlock {
            name,
            params,
            ty,
            constructors,
            recursor,
            iota_rules,
            ..
        } => {
            let mut built = Vec::new();
            install_inductive_block(
                builder,
                known,
                inductives,
                "",
                options,
                &ns,
                name,
                params,
                ty,
                constructors,
                recursor.as_ref(),
                iota_rules,
                &mut hovers,
                &mut built,
            )
            .expect("L1 prelude inductive block installs");
        }
        _ => panic!("L1 prelude must only contain axiom/def/inductive commands"),
    }
}

/// **ST2（v0.77.0）**：装 `Quot` 族 —— 四条 `Declar::Quot` + 两条公理
/// （`Quot.sound` 与 **G-75 的 `Quot.exact`**，后者见下）。
///
/// ⚠ **G-75 的取舍（0.81.0，与台账原文不同）**：台账写「Lean 4 core 有
/// `Quot.exact : Quot.mk r a = Quot.mk r b → r a b`（`Quotient.exact` 由它得到）」
/// ——**事实相反**。实查 Lean 工具链源码（`~/.elan/toolchains/*/src/lean/`）：
/// * `Init/Core.lean` 的 `namespace Quot` **没有** `exact`（只有 `sound`/`liftBeta`/
///   `indBeta`/`inductionOn`/`exists_rep`/`indep`/`indepCoherent`/`liftIndepPr1`）；
/// * 有的是 `Quotient.exact`（`Init/Core.lean:2223`），**对 `Setoid`**，是**定理**。
///
/// **一般形式是假的**：`Quot r` 的相等是 `r` 的**等价闭包**（`Init/Prelude.lean`
/// 的原话："The relation `r` is not required to be an equivalence relation; the
/// resulting quotient type's equality extends `r` to an equivalence"）。取
/// `r a b := (a = 0 ∧ b = 1) ∨ (a = 1 ∧ b = 2)`：`Quot.sound` 两次 + 传递 ⇒
/// `mk 0 = mk 2` ⇒ 一般形式的 `exact` 给出可证为假的 `r 0 2` ⇒ **不一致** ✗。
/// 所以装的是 **sound 版本**（= `Quotient.exact` 的语义）：要求 `r` 是等价关系，
/// 三条证明显式交进来（就是 Lean `Setoid` 的字段在这门语言里的展开写法）✓。
///
/// **为什么不"证明它"**：Mathlib 的 `Quotient.exact` 走 `Quot.lift` 到 `Prop` +
/// **`propext`**。本 prelude 若装 `propext`，课程 `lib/Extensionality.sokonanoda`
/// 自己声明的那条就会被 prelude 影子化 ⇒ 课程各入口的 **prelude 形状不再一致**，
/// 被 `crates/front/tests/prelude_shape.rs::every_course_entry_has_the_same_prelude_shape`
/// 实测咬住（K2 环境复用会因此静默改变判卷）✗。所以这里**只把"信"限定在
/// `Quot.exact` 这一条**（它说的是**真话**：`r` 已是等价关系时，闭包 = `r`），
/// 不引入一条通用的 `propext` ✓。
///
/// 为什么必须是 `Declar::Quot`：内核按**声明种类**认商（`Declar::Quot` ⇒
/// `RigidHead::QuotConst` ⇒ `Quot.lift`/`Quot.ind` 的 iota 归约，见
/// `crates/kernel/src/eval.rs` 的 `fire_quot` 与 `conv.rs` 的刚性头）。写成
/// `Declar::Axiom` 名字对、**归约不发生** ⇒ `Quot.lift f h (Quot.mk r a)` 卡住 ✗
/// （实测：装成 `Axiom` 时 `Eq.refl` 证不出 `Quot.lift … (Quot.mk …) = f a`）。
///
/// 类型**从 [`QUOT_TYPES_SRC`] 源文本 elaborate**（见那个常量的注释：手搓
/// `EnvBuilder` 表达式会撞内核索引约定）。
///
/// ⚠ 这条路径是**受信任安装**（与 Nat/Bool/Eq/L1 同一条：不进 `PendingOp`、内核
/// 不重查）⇒ `check_quot` 不会跑；它跑不了还有第二个原因 —— `check_quot` 的前置
/// `check_eq` 要求 `Eq` 是**归纳块**（`env.get_inductive("Eq")` + 一个构造子
/// `Eq.refl`），而本语言 prelude 的 `Eq` 是**公理**（[`PRELUDE_EQ_SRC`]）⇒ 真走
/// 内核那条路会 panic（`cannot add Quot; improperly formed Eq type`）。
/// 判据因此放在**归约**上 ✓。
///
/// 让位口径与 L1 族一致：文件自己声明 `Quot` 族任一名字 ⇒ 整族不装（`taken`）。
/// **ST2 的 Quot 安装**（`Quot` 五条 + re-kind 成 `Declar::Quot`/`Axiom`）。
///
/// ⚠ **试装必须走显式源**：试装发生在覆盖的 `OnceLock` **正在初始化**的时候，
/// 里面任何一次 `*_src()` 读取都会**重入** `OnceLock::get_or_init` ⇒ **死锁** ✗
/// （实测：e2e 挂住 10 分钟）⇒ 所以试装一路只用**显式传入**的源 ✓，
/// 真装（`install_l1_prelude`）传的是生效源 ✓。
fn install_quot_src<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    src: &str,
    taken: &HashSet<String>,
) {
    const QUOT_NAMES: [&str; 6] = [
        "Quot",
        "Quot.mk",
        "Quot.lift",
        "Quot.ind",
        "Quot.sound",
        "Quot.exact",
    ];
    if QUOT_NAMES.iter().any(|name| taken.contains(*name)) {
        return;
    }
    // prelude 安装期间关闭隐式实参插入（与 Eq/L1 同一个守卫）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    let Ok(file) = crate::parse(src) else {
        panic!("Quot type source parses");
    };
    let options = CompileOptions::default();
    let ns = NamespaceScope::new();
    let empty_inductives: InductiveTable<'_> = InductiveTable::new();
    let empty_defs: DefTable = DefTable::new();
    let ctx = ElabCtx {
        notations: None,
        prefix_src: "",
        options: &options,
        inductives: &empty_inductives,
        ns: &ns,
        defs: &empty_defs,
    };
    let mut hovers = Vec::new();
    // **`Eq` 可能不在 `known` 里**（`-- sokonanoda:prelude none` 的 Bare 模式、
    // 或文件自己声明了 `Eq` 三件套 ⇒ Eq 族让位）—— 但 `Quot.lift`/`Quot.sound`
    // 的类型**点名引用 `Eq`** ⇒ 先确认它在，否则整族不装（与 L1 的让位口径一致：
    // 依赖缺了就不装，绝不装一半 ✗）。
    if !known.contains_key("Eq") {
        return;
    }
    // 装前几条时要把 `Quot`/`Quot.mk` 放进作用域（后几条的类型点名引用它们）
    // ⇒ 在**局部副本**上做，不污染调用方的 `known`（那由本函数末尾统一登记）。
    let mut scope_known = known.clone();
    for command in &file.commands {
        let Command::Axiom {
            name, universe, ty, ..
        } = command
        else {
            panic!("QUOT_TYPES_SRC must only contain axiom commands");
        };
        let decl = build_axiom(builder, name, universe, ty, &scope_known, &mut hovers, &ctx)
            .expect("Quot type elaborates");
        // **只改声明种类**：`Quot.sound`/`Quot.exact` 保持公理，其余四条变 `Declar::Quot`。
        let decl = match decl {
            Declar::Axiom { info } if name != "Quot.sound" && name != "Quot.exact" => {
                Declar::Quot { info }
            }
            other => other,
        };
        builder
            .add_declar(decl)
            .expect("duplicate Quot prelude declaration");
        // 立刻登记（**两份都写**）：`scope_known` 给本函数后面几条的类型解析用
        // （`Quot.lift`/`Quot.sound` 点名引用 `Eq`/`Quot`/`Quot.mk`），
        // `known` 给**调用方**用（后续 `#check`/应用路径都要能解析到这五个名字）。
        let entry = KnownName::Decl {
            universes: universe.clone(),
            implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
            explicit_arity: crate::compile::elab::explicit_arity(ty),
            signature: Some(crate::proof::decl_signature(ty)),
        };
        scope_known.insert(name.clone(), entry.clone());
        known.insert(name.clone(), entry);
    }
}

/// Install the Eq prelude, skipping the whole block when the file declares
/// any of the names itself (all-or-nothing, mirroring the explicit `Nat`
/// block behavior: the file then owns equality entirely).
/// `known` gains the universe-parameter arity of each installed axiom.
pub(crate) fn install_eq_prelude(
    builder: &mut EnvBuilder<'_>,
    known: &mut KnownTable,
    taken: &std::collections::HashSet<String>,
) {
    // **E3**：同 `install_l1_prelude`（真装 = 生效源 ✓）。
    install_eq_prelude_src(builder, known, prelude_eq_src(), taken);
}

/// [`install_eq_prelude`] 的**显式源**版本（E3 的试装用它 ✓）。
fn install_eq_prelude_src(
    builder: &mut EnvBuilder<'_>,
    known: &mut KnownTable,
    src: &str,
    taken: &std::collections::HashSet<String>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    const EQ_NAMES: [&str; 3] = ["Eq", "Eq.refl", "Eq.subst"];
    if EQ_NAMES.iter().any(|name| taken.contains(*name)) {
        return;
    }
    // **E3**：走**生效**源（覆盖已在装载时验证过 ⇒ 这个 `expect` **永远不会**被覆盖触发 ✓）。
    let file = crate::parse(src).expect("Eq prelude source parses");
    let empty: InductiveTable<'_> = InductiveTable::new();
    let options = CompileOptions::default();
    let ns = NamespaceScope::new();
    let empty_defs: DefTable = DefTable::new();
    let ctx = ElabCtx {
        notations: None,
        prefix_src: "",
        options: &options,
        inductives: &empty,
        ns: &ns,
        defs: &empty_defs,
    };
    for command in &file.commands {
        let Command::Axiom {
            name, universe, ty, ..
        } = command
        else {
            panic!("Eq prelude must only contain axioms");
        };
        if taken.contains(name) {
            continue;
        }
        let mut hovers = Vec::new();
        let decl = build_axiom(builder, name, universe, ty, known, &mut hovers, &ctx)
            .expect("Eq prelude axiom elaborates");
        builder.add_declar(decl).expect("duplicate prelude axiom");
        // **G-30 第二堵墙（2026-10-06）**：Eq 族登记**真实的**前导隐式层数 ✓
        // （原来是硬编码 `0` ✗）。
        //
        // **Lean 4 对照**（`Lean/Elab/App.lean:752-775` `ElabAppArgs.main` +
        // `:712-717` `processImplicitArg`）：`Eq {α : Sort u} (a b : α)` 的
        // `{α}` 是 implicit binder ⇒ 写法 `Eq a b`（**只给显式实参**）由
        // `addImplicitArg` 补一个 fresh mvar，再与实参类型/期望类型合一
        // ⇒ `?α := typeof a` ✓。旧的 `implicit_prefix: 0` 让**应用路径**整条
        // 不触发 ✗ ⇒ `Eq a b` 被逐位当旧写法装成 `Eq.{?} a b`（`a` 顶到 `α`
        // 位上 ✗）。这条偏差是 2026-09 为"旧写法 `Eq.{u} α a b` 与短写在形状上
        // 分不开"打的补丁 ✓；那个歧义现在由 **G-42 的两条可判定闸门**兜住 ✓
        // （`args.len() > explicit_arity` = 写全参数的旧写法 ✓；
        // `fits_old_style` = 逐位贴合层域的旧写法 ✓）⇒ 补丁可以撤 ✓。
        //
        // 实测读数（第二堵墙复现件 `docs/gaps/repro/
        // G30-second-wall-iff-intro-implicit-alpha.sokonanoda`）：求解器本来就
        // 把 `Iff.intro` 的两位解成 `Eq (Set.singleton a) …` / `Eq a b` ✓，
        // 坏的是**回读后重 elaborate** 那一步 ✗ —— pp 文本
        // （`infer_type_text`）**省略隐式实参** ✓，回读成源级 `Eq …` 后必须靠
        // 应用路径把 `α` 补回来 ✓。
        known.insert(
            name.clone(),
            KnownName::Decl {
                universes: universe.clone(),
                implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                explicit_arity: crate::compile::elab::explicit_arity(ty),
                signature: Some(crate::proof::decl_signature(ty)),
            },
        );
    }
}

/// Trusted built-in base declarations. These are never re-checked by the
/// kernel. `Nat` is installed as a trusted source-style inductive block
/// (`Nat.zero`/`Nat.succ` constructors + a derived `Nat.rec`) so that `match`
/// on the prelude `Nat` lowers to a real `Nat.rec` and the recursor has a
/// proper iota rule set. `Nat.add` stays the native self-referential
/// definition: the kernel's native Nat reduction is enabled by the matching
/// declaration names.
pub(crate) fn install_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    let span = Span::default();
    let nat_sort = Expr::Sort {
        sort: SortKind::Type,
        span,
    };
    let nat_ident = Expr::Ident {
        name: "Nat".to_string(),
        span,
    };
    let constructors = vec![
        CtorDecl {
            name: "Nat.zero".to_string(),
            binders: Vec::new(),
            result: nat_ident.clone(),
            span,
        },
        CtorDecl {
            name: "Nat.succ".to_string(),
            binders: vec![Binder {
                name: "n".to_string(),
                ty: Some(Box::new(nat_ident.clone())),
                style: BinderKind::Explicit,
                span,
            }],
            result: nat_ident,
            span,
        },
    ];
    let mut hovers = Vec::new();
    let mut built = Vec::new();
    // G-05：prelude 永远在根命名空间、没有 `open`。
    let ns = NamespaceScope::new();
    install_inductive_block(
        builder,
        known,
        inductives,
        "",
        &CompileOptions::default(),
        &ns,
        "Nat",
        &[],
        &nat_sort,
        &constructors,
        None,
        &[],
        &mut hovers,
        &mut built,
    )
    .expect("built-in Nat block installs");

    let anon = builder.anonymous();
    let empty = builder.alloc_levels_slice(&[]);
    let nat = builder.name_from_str("Nat");
    let nat_type = builder.mk_const(nat, empty);

    let inner_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, nat_type);
    let add_arrow = builder.mk_pi(anon, BinderStyle::Default, nat_type, inner_arrow);
    let add_name = builder.name_from_str("Nat.add");
    let add_levels = builder.alloc_levels_slice(&[]);
    let add_self = builder.mk_const(add_name, add_levels);
    add_definition(builder, "Nat.add", add_arrow, add_self);
    known.insert(
        "Nat.add".to_string(),
        KnownName::Decl {
            universes: Vec::new(),
            implicit_prefix: 0,
            explicit_arity: 2,
            signature: None,
        },
    );
}

/// Trusted built-in `Bool`, installed exactly like the `Nat` block: a
/// source-style inductive with constructors `Bool.true`/`Bool.false` and a
/// derived `Bool.rec`, registered in `known` and the `match` `InductiveTable`.
/// Non-recursive, so the recursor is the plain two-branch eliminator and the
/// kernel needs no change. The names must stay `Bool`/`Bool.true`/`Bool.false`
/// to match the kernel's name cache (frozen; `docs/architecture.md` §5.4).
pub(crate) fn install_bool_prelude<'a>(
    builder: &mut EnvBuilder<'a>,
    known: &mut KnownTable,
    inductives: &mut InductiveTable<'a>,
) {
    // prelude 安装期间关闭隐式实参插入（见 `elab::PreludeInstallGuard` 的注释）。
    let _implicit_guard = crate::compile::elab::PreludeInstallGuard::enter();
    let span = Span::default();
    let bool_sort = Expr::Sort {
        sort: SortKind::Type,
        span,
    };
    let bool_ident = Expr::Ident {
        name: "Bool".to_string(),
        span,
    };
    let constructors = vec![
        CtorDecl {
            name: "Bool.true".to_string(),
            binders: Vec::new(),
            result: bool_ident.clone(),
            span,
        },
        CtorDecl {
            name: "Bool.false".to_string(),
            binders: Vec::new(),
            result: bool_ident,
            span,
        },
    ];
    let mut hovers = Vec::new();
    let mut built = Vec::new();
    // G-05：prelude 永远在根命名空间、没有 `open`。
    let ns = NamespaceScope::new();
    install_inductive_block(
        builder,
        known,
        inductives,
        "",
        &CompileOptions::default(),
        &ns,
        "Bool",
        &[],
        &bool_sort,
        &constructors,
        None,
        &[],
        &mut hovers,
        &mut built,
    )
    .expect("built-in Bool block installs");
}

fn add_definition<'a>(builder: &mut EnvBuilder<'a>, name: &str, ty: ExprPtr<'a>, val: ExprPtr<'a>) {
    let name = builder.name_from_str(name);
    let info = DeclarInfo {
        name,
        uparams: builder.alloc_levels_slice(&[]),
        ty,
    };
    builder
        .add_declar(Declar::Definition {
            info,
            val,
            hint: ReducibilityHint::Regular(0),
        })
        .expect("duplicate builtin definition");
}

#[cfg(test)]
mod e3_tests {
    use super::*;

    /// 每个用例一个**独立目录**（避免并行测试互相看见对方写的覆盖 ✓）。
    fn tmpdir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("soko-e3-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    /// **判据 ④ 的装载层那一半**：空目录 ⇒ 三条源**逐字节等于内置**、指纹 **0** ✓
    /// ⇒ 缓存键与"没有 E3 时"逐字相同（自足分发不变 ✓）。
    #[test]
    fn an_absent_override_is_byte_identical_to_the_builtin() {
        let o = load_prelude_override(&tmpdir("empty"));
        assert_eq!(o.eq, PRELUDE_EQ_SRC);
        assert_eq!(o.l1, PRELUDE_L1_SRC);
        assert_eq!(o.quot, QUOT_TYPES_SRC);
        assert_eq!(o.hash, 0, "没覆盖 ⇒ 指纹 0 ⇒ 键与今天逐字相同 ✓");
        assert!(o.error.is_none());
    }

    /// **判据 ① 的装载层那一半 + ②**：只覆盖 `L1.sokonanoda` ⇒ 只有 L1 变 ✓；
    /// 覆盖**留下指纹** ✓；换内容 ⇒ **指纹必变**（⇒ 缓存键必变 ⇒ 必 miss ✓）。
    #[test]
    fn an_override_replaces_only_the_named_file_and_fingerprints_it() {
        let dir = tmpdir("one");
        let extra = format!("{PRELUDE_L1_SRC}\naxiom E3Probe : Prop\n");
        std::fs::write(dir.join("L1.sokonanoda"), &extra).expect("write");
        let o = load_prelude_override(&dir);
        assert_eq!(o.l1, extra, "被覆盖的那一条用覆盖内容");
        assert_eq!(o.eq, PRELUDE_EQ_SRC, "没覆盖的仍用内置 ✓");
        assert_eq!(o.quot, QUOT_TYPES_SRC);
        assert_ne!(o.hash, 0, "覆盖必须留下指纹（进缓存键 ✓）");
        assert!(o.error.is_none());

        std::fs::write(
            dir.join("L1.sokonanoda"),
            format!("{extra}\naxiom E3Probe2 : Prop\n"),
        )
        .expect("write");
        assert_ne!(
            load_prelude_override(&dir).hash,
            o.hash,
            "换了覆盖内容 ⇒ 指纹必须变（否则会命中旧 prelude 的条目 ✗）"
        );
    }

    /// **判据 ③ 的强一半**：**解析过但装不上**（前向引用）也**不许 panic** ✗ ——
    /// 只验解析是**不够**的（实测：把 `True.intro` 的类型写成后面族才有的 `False`
    /// ⇒ `install_l1_command` 的 `expect` 直接 panic ✗）⇒ E3 用**试装**（scratch 环境 +
    /// `catch_unwind`）把它变成可报的错 ✓。
    #[test]
    fn an_override_that_parses_but_does_not_install_is_rejected_not_panicking() {
        let dir = tmpdir("forward-ref");
        let flipped =
            PRELUDE_L1_SRC.replacen("axiom True.intro : True", "axiom True.intro : False", 1);
        assert_ne!(flipped, PRELUDE_L1_SRC, "夹具前提：L1 里有那条 axiom");
        std::fs::write(dir.join("L1.sokonanoda"), &flipped).expect("write");
        let o = load_prelude_override(&dir);
        assert_eq!(o.l1, PRELUDE_L1_SRC, "装不上 ⇒ 该条不生效 ✓");
        assert_eq!(o.hash, 0);
        let err = o.error.expect("必须记下原因 ✓");
        assert!(err.contains("装不上"), "{err}");
    }

    /// **判据 ③**：畸形覆盖 ⇒ **不 panic**、该条**不生效**（回落内置）、原因**记下来**
    /// （CLI 据此报退出码 2 ✓；`install_*_prelude` 的 `expect` 因此永远不会被覆盖触发 ✓）。
    #[test]
    fn a_malformed_override_does_not_panic_and_falls_back() {
        let dir = tmpdir("bad");
        std::fs::write(dir.join("L1.sokonanoda"), "theorem oops : : :\n").expect("write");
        let o = load_prelude_override(&dir);
        assert_eq!(o.l1, PRELUDE_L1_SRC, "解析不过 ⇒ 该条不生效 ✓");
        assert_eq!(o.hash, 0, "没生效 ⇒ 不留指纹 ✓");
        let err = o.error.expect("必须记下原因（否则用户看不见 ✗）");
        assert!(err.contains("解析失败"), "{err}");
        assert!(err.contains("L1.sokonanoda"), "原因里要有文件名：{err}");
    }
}
