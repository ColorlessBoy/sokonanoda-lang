//! 隐式实参（IA-1，路线 C：**风格对齐 + 唯一确定**）。
//!
//! 设计 `docs/design/implicit-arguments.md`（§3 是路线 C 的算法，§0 是安全性质）。
//!
//! 一句话：**显式实参按「风格」对齐到显式层、跳过隐式层**；被跳过的层由
//! 「后续显式实参的类型 + 整体期望类型」的头部匹配**唯一确定**；解不出就报
//! 专用错误码（`elab-implicit-argument-unsolved`），**不猜、不搜索、不回溯**。
//!
//! **安全性质**（设计 §0，P1 可独立发布的依据）：签名里没有隐式 binder 时，
//! 本模块的入口在调用方就返回 `None`，行为与今天的 `mk_app` 链**逐字节相同**。
//! `crates/cli/tests/notation.rs` 的
//! `no_course_signature_uses_an_implicit_binder` 把「课程签名今天没有隐式
//! binder」钉死——否则这条安全性质会随课程改写**悄悄失效**（设计 §0 的注）。
//!
//! 内核零改动：`BinderStyle` 在内核里只是 pp 风格，隐式 binder 与显式 binder
//! 在内核里都是**真 Pi**（`crates/kernel` 的注释自述「只被 pp 使用，不改变类型
//! 检查」）⇒ 前端把解出来的隐式实参当**普通实参**喂进 `mk_app` 即可。

use crate::ast::{BinderKind, Expr};

/// 签名的一层：名字 / 域 / 风格。
#[derive(Clone)]
pub(crate) struct Layer {
    pub name: String,
    pub domain: Expr,
    /// 源级风格。**应用路径现在用注册表的 `implicit_prefix` 而不是它**（构造子
    /// 的参数在 kernel 类型里显式、语义上隐式）；保留它是为了让 `telescope` 的
    /// 单测仍能钉住「风格活着从签名文本里出来」。
    #[allow(dead_code)]
    pub style: BinderKind,
}

/// 把 `judge_infer` 给的签名文本拆成「带风格的 Pi 层 + 结果类型」。
///
/// 与 `elab::notation_telescope` 同一份解析（`parse_expr_text` + 逐层剥 Pi），
/// 只多带一个 `style`：内核 pp 把隐式 binder 打成 `{α : Type}`，而
/// `spine::peel_pi` 把风格丢了——路线 C 全靠它。多 binder 折叠
/// （`forall (a b : T), …`）与 `->`（匿名的显式层）都要认。
///
/// **参数名一律换成 fresh 名**（`\0soko_p0`…，NUL 前缀不可能出现在源码标识符里），
/// 并把域/结果里对**外层参数名**的引用一起代换。理由：求解器是**按名字**代换的
/// （[`solve_prefix`] 的 `sigma`），签名参数名一旦与用户变量同名就会**捕获**——
/// `congrArg {α β} … (f : α → β)` 在 `theorem t (α β : Type) (g : β → α) … :=
/// congrArg.{1} g hxy` 里把 `β` 解成用户的 `β` 而不是 `α`（实测判红）。fresh 名
/// 让模板与用户表达式永不撞名。
/// **telescope 解析次数**（A4b 的判据读数 · `#[doc(hidden)]` · 只给判据用）。
///
/// **为什么先建它**（2026-10-08 · 规划 §2 A4b）：telescope 解析**今天没有任何出口** ——
/// 两处（本模块的 [`telescope`] 与 `elab::notation_telescope`）都**每次都
/// `parse_expr_text`**（无 memo）⇒ "要不要做签名级缓存"**没有数据可依** ✗。
/// 计数器先回答"一次按键解析几次"，再谈优化 ✓（判据纪律：先读数、后动手）。
pub static TELESCOPE_PARSES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// 记一次 telescope 解析（**两处调用点共用**；只给判据用）。
#[doc(hidden)]
pub fn note_telescope_parse() {
    TELESCOPE_PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// 见 [`TELESCOPE_PARSES`]。
#[doc(hidden)]
pub fn telescope_parses_total() -> u64 {
    TELESCOPE_PARSES.load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn telescope(signature: &str) -> Option<(Vec<Layer>, Expr)> {
    note_telescope_parse();
    let mut cur = crate::proof::parse_expr_text(signature).ok()?;
    let mut layers: Vec<Layer> = Vec::new();
    let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
    let mut fresh_id = 0usize;
    loop {
        match cur {
            Expr::Forall { binders, body, .. } if !binders.is_empty() => {
                for b in &binders {
                    let domain = crate::spine::substitute(b.ty.as_deref()?, &sigma);
                    let fresh = format!("\u{0}soko_p{fresh_id}");
                    fresh_id += 1;
                    sigma.insert(
                        b.name.clone(),
                        Expr::Ident {
                            name: fresh.clone(),
                            span: b.span,
                        },
                    );
                    layers.push(Layer {
                        name: fresh,
                        domain,
                        style: b.style.clone(),
                    });
                }
                cur = *body;
            }
            Expr::Arrow {
                domain, codomain, ..
            } => {
                layers.push(Layer {
                    name: String::new(),
                    domain: crate::spine::substitute(&domain, &sigma),
                    style: BinderKind::Explicit,
                });
                cur = *codomain;
            }
            other => return Some((layers, crate::spine::substitute(&other, &sigma))),
        }
    }
}

/// 前导隐式层的个数（`0` ⇒ 调用方走今天的老路）。
///
/// 应用路径改用注册表的 `implicit_prefix`（构造子的参数在 pp 里是显式的）；
/// 这个函数只被 `implicit.rs` 的单测用来钉住 `telescope` 的风格解析。
#[allow(dead_code)]
pub(crate) fn leading_implicit(layers: &[Layer]) -> usize {
    layers
        .iter()
        .take_while(|l| l.style == BinderKind::Implicit)
        .count()
}

/// 解出被跳过的前导 `k` 层。
///
/// **路线 ①（由后续显式实参的类型反解）**：`arg_tys[i]` = 第 `k + i` 层那个
/// 实参的**类型**（`None` = 拿不到）。对第 `i` 个待解的隐式层，找**后面第一个**
/// 提到它的层 `j`，把 `layers[j].domain`（已解出的参数先代进去）与
/// `arg_tys[j - k]` 头部匹配：
///   · `Set.mem {α} (a : α) …` ⇒ `α` 出现在第 `k` 层（`a : α`）⇒ 由 `a` 的类型解出；
///   · `Set.image {α β} (f : α → β) …` ⇒ `α`/`β` 都在第 `k` 层 ⇒ 由 `f` 的类型解出；
///   · `picks {α} (n : Nat) (b : α)` 调用 `picks 3 b` ⇒ `α` 在第 `k+1` 层
///     （`b : α`）⇒ 由 `b` 的类型解出（**扫全部显式层**，不只第一层）。
///
/// **路线 ②（由期望类型反解）**：参数只出现在**结果类型**里时（`Or.inl` 的 `B`、
/// `False.elim`/`absurd` 的结论、`Eq.refl` 的 `α`），把已解出的参数代进 `result`
/// 再与 `expected` 头部匹配。与记法路径的 `solve_prefix_args` 同一条机械。
///
/// 两条路都解不出 ⇒ `None`（调用方报专用错误码，**不猜**）。
///
/// 两条路都带**期望类型/实参类型的 delta 展开兜底**：`Or.inl h` 的目标常写成
/// `a ∈ A ∪ B`（`Set.mem … (Set.union …)`，两层 `def`），`And.left h` 的 `h`
/// 常写成 `a ∈ A ∩ B`——不展开到 `Or`/`And` 归纳头就匹配不上。
///
/// **E19 刀2（`SOKO_NOTATION_METAVAR`，2026-10-01 起默认开）**：严格档解不出时
/// ⇒ 再走一遍**待定档**（[`solve_prefix_pending`]）—— 解不出的位先记成待定，走完由
/// [`fill_pending_by_shape`] 与**同形的已解兄弟**合一 ✓。
/// **逃生门** `=0`/`off` ⇒ 只跑严格档（逐字节等于刀0 的行为）✓。
/// ⚠ 严格档**永远先跑**：解得出的形状走的就是刀0 那条路，待定档一次都不进 ⇒
/// 默认路径零开销 ✓（冷 `build` 结构计数逐项等于刀0，见 §9）。
pub(crate) fn solve_prefix(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> Option<Vec<Expr>> {
    if let Some(solved) = solve_prefix_impl(
        layers,
        result,
        k,
        arg_tys,
        &[],
        expected,
        defs,
        is_inductive,
        false,
    ) {
        return Some(solved);
    }
    solve_prefix_outcome(layers, result, k, arg_tys, expected, defs, is_inductive).into_option()
}

/// [`solve_prefix`] 的**带通道**版本（M3）：严格档**永远先跑且不变**，失败后按档位分流；
/// 引擎档把失败归因成三条通道（`Unsolved` / `Kind` / `Clash`）——**用户可见的码仍是既有那条**，
/// 通道只决定调用方的 hint/message 说哪一句 ✓（D6 = 不新增码）。
/// **G-63 修复的入口**（2026-10-03 ✓）：与 [`solve_prefix`] 同义，但多带**显式实参本身** ✓ ——
/// 求解器要用它们把**显式层的 fresh 名**映射掉 ✗（否则模板里留裸 `\0soko_p*` ⇒ 与实际实参
/// 判成刚性冲突 ✗；根因读数见 commit `c3ab1211` ✓）。旧签名那条保留为包装 ✓ ⇒ 既有调用点零改动 ✓。
// G-63 ③ 的包装多带一个实参（8 个）⇒ 越过 clippy 的 7 个阈值 ✓（纯转发，不引入复杂度 ✓）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_prefix_with_args(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    arg_vals: &[&Expr],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> Option<Vec<Expr>> {
    solve_prefix_outcome_with_args(
        layers,
        result,
        k,
        arg_tys,
        arg_vals,
        expected,
        defs,
        is_inductive,
    )
    .into_option()
}

pub(crate) fn solve_prefix_outcome(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> crate::compile::meta::MetaSolve {
    solve_prefix_outcome_with_args(
        layers,
        result,
        k,
        arg_tys,
        &[],
        expected,
        defs,
        is_inductive,
    )
}

/// [`solve_prefix_outcome`] 的**带实参版**（G-63 ✓）：多带**显式实参本身** ✓。
// G-63 ③ 的包装多带一个实参（8 个）⇒ 越过 clippy 的 7 个阈值 ✓（纯转发，不引入复杂度 ✓）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_prefix_outcome_with_args(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    // **G-63 修复**：显式实参**本身** ✓。
    arg_vals: &[&Expr],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> crate::compile::meta::MetaSolve {
    use crate::compile::meta::MetaSolve;
    if let Some(solved) = solve_prefix_impl(
        layers,
        result,
        k,
        arg_tys,
        arg_vals,
        expected,
        defs,
        is_inductive,
        false,
    ) {
        return MetaSolve::Solved(solved);
    }
    if metavar_enabled() {
        // **档位**（IA-4 M2）：`Sibling` = E19 的窄版（默认，逐字节等于今天）；
        // `Engine` = 新引擎（真元变量 + 三值合一 + occurs/作用域 + 有界待定 + 出口 zonk）。
        return if metavar_mode() == MetavarMode::Engine {
            solve_prefix_meta(
                layers,
                result,
                k,
                arg_tys,
                arg_vals,
                expected,
                defs,
                is_inductive,
            )
        } else {
            match solve_prefix_pending(
                layers,
                result,
                k,
                arg_tys,
                arg_vals,
                expected,
                defs,
                is_inductive,
            ) {
                Some(v) => MetaSolve::Solved(v),
                None => MetaSolve::Unsolved,
            }
        };
    }
    MetaSolve::Unsolved
}

/// **IA-4 M2 的引擎档**（`SOKO_METAVAR=engine`）：`solve_prefix` 的**一般路径**（应用 / 裸常量 /
/// 路线③富余实参）走 [`crate::compile::meta::MetaCtx`]。
///
/// 与 [`solve_prefix_pending`] 的**约束同源**（不再一位一位贪心）：
/// ① 每个前导位建一个元变量（`ty` = 该层的域，作用域 = **更晚**的望远镜参数名）；
/// ② 望远镜名 → 元变量（模板代换）；③ 每个**后续**层的域 ≟ 该实参的类型；④ 结果 ≟ 期望类型；
/// ⑤ 不动点 + defaulting + zonk（出口无残留自检在 `discharge` 里）。
///
/// ⚠ `implicit::telescope` 的参数名**本来就是 fresh 名**（`\0soko_p{i}`）⇒ 这里不需要再 freshen
/// （记法路径那条要，因为 `notation_telescope` 用的是签名原文名 —— M1 实测的坑）。
/// **元变量不进项**：解不出就返回 `None`，调用方照旧报既有码 ✓。
// G-63 ③ 多带一个 `arg_vals`（8 个）⇒ 越过 clippy 的 7 个阈值 ✓（参数同族，不引入复杂度 ✓）。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_meta(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    // **G-63 修复**：显式实参**本身**（不是它们的类型 ✗）—— 把显式层的 fresh 名映射掉 ✓。
    arg_vals: &[&Expr],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> crate::compile::meta::MetaSolve {
    let unfold = |e: &Expr| {
        crate::spine::unfold_to_inductive(
            e,
            is_inductive,
            defs,
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
    // ① 元变量（与窄版同一条闸门：前导位必须都有名字）
    let mut ids = Vec::with_capacity(k);
    for i in 0..k {
        if layers[i].name.is_empty() {
            return crate::compile::meta::MetaSolve::Unsolved;
        }
        let out_of_scope: Vec<String> = layers
            .iter()
            .skip(i + 1)
            .map(|l| l.name.clone())
            .filter(|n| !n.is_empty())
            .collect();
        ids.push(meta.fresh(
            layers[i].domain.clone(),
            crate::compile::meta::MetaKind::Natural,
            out_of_scope,
        ));
    }
    // ② 望远镜名 → 元变量
    let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        sigma.insert(layers[i].name.clone(), meta.meta_expr(*id));
    }
    // ②b **G-63 修复**（2026-10-03 ✓）：显式层的 fresh 名也要映射到**实际实参本身** ✓。
    // 不映射 ⇒ 模板里留裸 `\0soko_p*` ✗ ⇒ 与实际实参判成刚性冲突 ✗（根因读数见 commit c3ab1211 ✓：
    // `③ i=1 j=4 模板= soko_m1 soko_p2 soko_p3 实际=qr α a b` ⇒ `[clash] 左= soko_p2 ｜ 右= a` ✗）。
    for x in 0..arg_vals.len().min(layers.len().saturating_sub(k)) {
        let name = layers[k + x].name.clone();
        if !name.is_empty() {
            sigma.insert(name, arg_vals[x].clone());
        }
    }
    // ③ 路线①：后续层的域（已代换）≟ 该实参的类型
    for i in 0..k {
        let name = layers[i].name.clone();
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            let Some(actual) = j
                .checked_sub(k)
                .and_then(|x| arg_tys.get(x))
                .and_then(|t| t.as_ref())
            else {
                continue;
            };
            if !crate::spine::mentions(&name, &layer.domain) {
                continue;
            }
            let template = crate::spine::substitute(&layer.domain, &sigma);
            if std::env::var("SOKO_CLASH_TRACE").is_ok() {
                eprintln!(
                    "[g63] ③ i={i} j={j} 模板={} 实际={}",
                    crate::proof::render_expr(&template),
                    crate::proof::render_expr(actual)
                );
            }
            if meta.unify(&template, actual) == crate::compile::meta::Tri::No {
                return meta.channel();
            }
        }
    }
    // ④ 路线②：结果（已代换）≟ 期望类型
    if let Some(expected) = expected {
        let template = crate::spine::substitute(result, &sigma);
        if std::env::var("SOKO_CLASH_TRACE").is_ok() {
            eprintln!(
                "[g63] ④ 模板={} 期望={}",
                crate::proof::render_expr(&template),
                crate::proof::render_expr(expected)
            );
        }
        if meta.unify(&template, expected) == crate::compile::meta::Tri::No {
            return meta.channel();
        }
    }
    // ⑤ 不动点 + defaulting（E19 的选择规则）+ zonk
    meta.discharge(&ids)
}

/// **E19 的开关**（刀1 记法路径 + 刀2 一般路径**共用同一个开关**）：给求解器引入
/// **待定参数**（`?α`）。
///
/// **2026-10-01 用户拍板：默认开** ✓ —— 今天判红的形状（`∅ ≈ {b}` 一类）变绿是
/// **有意**的判定变化（非课程语料里只有 G-48 那一份变；课程语料一个字不动 ✓），
/// 逐项读数 ⇒ `docs/design/e19-baseline.md` §9。
/// **逃生门** `SOKO_NOTATION_METAVAR=0`（或 `off`）⇒ 回到**严格档**（刀0 的既有
/// 行为：任何一位解不出就报专用错误码，**不猜** ✓）—— 与 `SOKO_JUDGE_INPLACE`
/// 同一口径（默认 `on`、显式 `off` 回退 ✓）。
///
/// ⚠ 开关**关**态仍是"两态反向验证"的那一半（`crates/cli/tests/{notation,implicit}_metavar.rs`
/// 都用它咬"开关其实是假的"）⇒ **别删**。
/// 只读一次环境（求解热路径上）。
/// **求解器的元变量档位**（IA-4 M1 起三态，**M4 起默认 `Engine`**）：
/// `Off` = 严格档（逃生门口径）· `Sibling` = E19 的「同形已解兄弟」窄版（**回归臂**）·
/// `Engine` = 新引擎（`crate::compile::meta`：真元变量 + 三值合一 + 有界待定 + defaulting）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MetavarMode {
    Off,
    Sibling,
    Engine,
}

/// 开关（只读一次环境，求解热路径上）：`SOKO_METAVAR=0|off` / `=sibling` / `=engine`（或 `unify`）；
/// **不设 ⇒ `Engine`**（IA-4 **M4：默认开** —— 两态三指纹逐字节相同是前提 ✓）。
/// **`sibling` 永久保留为回归臂**（E19 那一份实现，判据仍然钉着它）✓。
/// **兼容旧逃生门** `SOKO_NOTATION_METAVAR=0|off` ⇒ `Off`（两个都设时以 `SOKO_METAVAR` 为准）。
/// 未知取值按 `Engine`（**不静默换档** ✗）。
pub(crate) fn metavar_mode() -> MetavarMode {
    static MODE: std::sync::OnceLock<MetavarMode> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("SOKO_METAVAR").ok().as_deref() {
        Some("0") | Some("off") => MetavarMode::Off,
        Some("sibling") | Some("1") | Some("on") => MetavarMode::Sibling,
        Some("engine") | Some("unify") => MetavarMode::Engine,
        Some(_) | None => match std::env::var("SOKO_NOTATION_METAVAR").ok().as_deref() {
            Some("0") | Some("off") => MetavarMode::Off,
            // **M4 默认 = 引擎**（不设开关 ⇒ 引擎；旧逃生门仍把档位关掉 ✓）
            _ => MetavarMode::Engine,
        },
    })
}

/// 「待定档要不要跑」—— `Off` 之外都跑（**语义与 E19 的 `metavar_enabled` 逐字一致** ✓）。
pub(crate) fn metavar_enabled() -> bool {
    metavar_mode() != MetavarMode::Off
}

/// **待定位的合一**（E19 刀1/刀2 **共用一份**）：把每一个还没解出的前导参数 `?i`
/// 与一个**已解出**的兄弟 `?j` 合一 —— 判据是两层的**域同形**
/// （[`crate::spine::same_shape`]，忽略 span）。
///
/// **为什么是这条判据**：`{α β : Type}` 的两个类型参数**域都是 `Type`** ⇒ 它们
/// "同一种东西"；`{b}` 已经把 `β` 定成 `β` 了，那么定不出来的 `α` 只能跟着它
/// （`Set.Equiv ∅ {b}` 读作 `Set.Equiv β β ∅ {b}`）✓。
/// **一个都定不出来就仍然失败**（没有兄弟可依 ⇒ 照旧报"补不出参数"，不猜、
/// 不发明类型 ✓）。
///
/// **IA-4 M2 起它降级为 defaulting 的参考实现**：引擎档把同一条规则实现为
/// [`crate::compile::meta::MetaCtx::default_unresolved`]（判据仍是"声明类型同形"，只是从
/// "拷一个**已解**兄弟的值"一般化成"两两合一、选代表"）⇒ 本函数是 **`Sibling` 档**的那一份，
/// 不再是"待定参数的唯一出口"（默认档逐字节不变 ✓）。
pub(crate) fn fill_pending_by_shape(
    layers: &[(String, Expr)],
    solved: &mut [Option<Expr>],
) -> Option<()> {
    // 域里的**前导参数名**先代成已解值 —— 同形比较要看**代换后**的形状。
    let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
    for (k, value) in solved.iter().enumerate() {
        if let (Some((name, _)), Some(value)) = (layers.get(k), value.as_ref()) {
            sigma.insert(name.clone(), value.clone());
        }
    }
    let domains: Vec<Expr> = layers
        .iter()
        .map(|(_, domain)| super::goals::substitute_names(domain, &sigma, &Default::default()))
        .collect();
    for i in 0..solved.len() {
        if solved[i].is_some() {
            continue;
        }
        let mut picked: Option<Expr> = None;
        for j in 0..solved.len() {
            if j == i {
                continue;
            }
            let Some(value) = solved[j].as_ref() else {
                continue;
            };
            if crate::spine::same_shape(&domains[i], &domains[j]) {
                picked = Some(value.clone());
                break;
            }
        }
        solved[i] = Some(picked?);
    }
    Some(())
}

/// **待定档**（E19 刀2）：某一位解不出时**不立刻失败**，先记成**待定**（`?α`），
/// 等所有位都走完再用 [`fill_pending_by_shape`] 合一 ✓。
///
/// ⚠ **待定值从不进入项**：合一在**实参 elaborate 之前**完成 ⇒ 实参拿到的期望类型
/// 永远是**具体**类型，内核看到的项里没有任何占位符 ✓。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_pending(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    // **G-63 修复**：显式实参**本身**（不是它们的类型 ✗）—— 把显式层的 fresh 名映射掉 ✓。
    arg_vals: &[&Expr],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
) -> Option<Vec<Expr>> {
    solve_prefix_impl(
        layers,
        result,
        k,
        arg_tys,
        arg_vals,
        expected,
        defs,
        is_inductive,
        true,
    )
}

/// 严格档（`allow_pending = false`）= **既有行为，逐字节不变**：任何一位解不出就
/// 整体 `None`（调用方报专用错误码，**不猜**）✓。
#[allow(clippy::too_many_arguments)]
fn solve_prefix_impl(
    layers: &[Layer],
    result: &Expr,
    k: usize,
    arg_tys: &[Option<Expr>],
    // **G-63 修复**：显式实参**本身**（不是它们的类型 ✗）—— 把显式层的 fresh 名映射掉 ✓。
    _arg_vals: &[&Expr],
    expected: Option<&Expr>,
    defs: &crate::compile::elab::DefTable,
    is_inductive: &dyn Fn(&str) -> bool,
    allow_pending: bool,
) -> Option<Vec<Expr>> {
    let unfold = |e: &Expr| {
        crate::spine::unfold_to_inductive(
            e,
            is_inductive,
            defs,
            8,
            None,
            crate::spine::UnfoldAlign::Short,
        )
    };
    // **`Option` 槽**（E19 刀2）：严格档里它**永远全是 `Some`**（解不出就提前
    // `return None`）⇒ 与改动前逐字节同行为 ✓；待定档里 `None` = "这一位待定" ✓。
    let mut solved: Vec<Option<Expr>> = Vec::with_capacity(k);
    for i in 0..k {
        let name = layers[i].name.clone();
        if name.is_empty() {
            return None;
        }
        let mut sigma: std::collections::HashMap<String, Expr> = std::collections::HashMap::new();
        for (j, s) in solved.iter().enumerate() {
            if !layers[j].name.is_empty() {
                if let Some(s) = s {
                    sigma.insert(layers[j].name.clone(), s.clone());
                }
            }
        }
        let mut found: Option<Expr> = None;
        // 路线 ①：后续显式层的域 vs 该层实参的类型。
        for (j, layer) in layers.iter().enumerate().skip(i + 1) {
            // `j < k` 的层也是隐式的（还没解出，也没有实参可问）。
            let Some(actual) = j
                .checked_sub(k)
                .and_then(|x| arg_tys.get(x))
                .and_then(|t| t.as_ref())
            else {
                continue;
            };
            let domain = crate::spine::substitute(&layer.domain, &sigma);
            if !crate::spine::mentions(&name, &domain) {
                continue;
            }
            let mut v = super::elab::unify_extract(&domain, actual, &name);
            if v.is_none() {
                let unfolded = unfold(actual);
                if &unfolded != actual {
                    v = super::elab::unify_extract(&domain, &unfolded, &name);
                }
            }
            // **模板侧也要能展开**（T-N13，2026-09-30）：记法路径的 `solve_prefix_args`
            // 早就有这一条（R5，2026-09-26 实测：`{∅} ∩ {…}` 一族），而**唯一钩子**
            // 这条路线只展开了**实参侧** ⇒ 同一形状在两条路上判得不一样 ✗。
            //
            // 形状：模板是 `Set ?α`（`Set` 是 **def**），实参的类型文本却是**箭头形态**
            // （`Set Two` 的 pp 就是 `Two -> Prop`）⇒ `App(Set, ?α)` 与 `Arrow{…}`
            // 头对不上 ⇒ 解不出 ✗（实测：`f1 '' ({aa} ∩ {bb})` 落在 `=` 的操作数位时，
            // `∩` 的 `α` 报 `elab-implicit-argument-unsolved`）。
            // **只加解、不改既有解**：先按原样试，失败才展开 ✓。
            //
            // ⚠ **闸门：实际项必须是「`Set` 的展开形态」**（`X -> Prop`，陪域是 Sort）——
            // 否则会拿**函数类型**的域去对：`Set.univ` 作为第一个实参时，它的类型是
            // `(α : Type) → Set α`（真 Pi），展开后的模板 `?α -> Prop` 与它按 `peel_pi`
            // 一对，域 `?α` 就吃下 `Type 0` ⇒ `α := Type 0` ⇒ 内核报
            // `def_eq mismatch expected: Sort(1) | actual: Sort(2)` ✗（实测：
            // `Set.univ ⊆ A` 与 `Set.subset Set.univ A` 两条都当场判红）。
            // 判据与 R5 的意图一致：展开只在**两边同形**时才有意义 ✓。
            // ⚠ **剥到底**再判：`α → β → Prop`（lambda 的类型）的**第一层陪域**是
            // `β → Prop`（还是 Pi），只看一层会把合法的形状挡掉 ✗（实测：unit12 的
            // `(fun …) • (fun …)` —— 两个操作数都是 lambda，类型是 `α → β → Prop`）。
            let actual_is_unfolded_set = match crate::spine::peel_pi(actual) {
                None => true,
                Some(_) => {
                    let mut cur = actual.clone();
                    while let Some(pi) = crate::spine::peel_pi(&cur) {
                        cur = pi.body;
                    }
                    matches!(cur, Expr::Sort { .. })
                }
            };
            if v.is_none() && actual_is_unfolded_set {
                let unfolded_domain = unfold(&domain);
                if unfolded_domain != domain {
                    v = super::elab::unify_extract(&unfolded_domain, actual, &name);
                    if v.is_none() {
                        let unfolded = unfold(actual);
                        if &unfolded != actual {
                            v = super::elab::unify_extract(&unfolded_domain, &unfolded, &name);
                        }
                    }
                }
            }
            if let Some(v) = v {
                found = Some(v);
                break;
            }
        }
        // 路线 ②：期望类型 vs 已代换的结果类型。
        if found.is_none() {
            if let Some(expected) = expected {
                let template = crate::spine::substitute(result, &sigma);
                found = super::elab::unify_extract(&template, expected, &name);
                if found.is_none() {
                    let unfolded = unfold(expected);
                    if &unfolded != expected {
                        found = super::elab::unify_extract(&template, &unfolded, &name);
                    }
                }
                // 模板侧展开（与路线 ① 同款，理由见上）。
                if found.is_none() {
                    let unfolded_template = unfold(&template);
                    if unfolded_template != template {
                        found = super::elab::unify_extract(&unfolded_template, expected, &name);
                        if found.is_none() {
                            let unfolded = unfold(expected);
                            if &unfolded != expected {
                                found = super::elab::unify_extract(
                                    &unfolded_template,
                                    &unfolded,
                                    &name,
                                );
                            }
                        }
                    }
                }
            }
        }
        match found {
            Some(value) => solved.push(Some(value)),
            // **待定档**：记成 `None`，等 `fill_pending_by_shape` 合一 ✓。
            None if allow_pending => solved.push(None),
            // **严格档**（既有行为）：一位解不出 ⇒ 整体失败，调用方报专用码 ✓。
            None => return None,
        }
    }
    if allow_pending {
        let pairs: Vec<(String, Expr)> = layers
            .iter()
            .map(|l| (l.name.clone(), l.domain.clone()))
            .collect();
        fill_pending_by_shape(&pairs, &mut solved)?;
    }
    // 还有 `None` ⇒ 整体失败（严格档恒不成立；待定档 = "一个兄弟都借不到"）✓。
    solved.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_defs() -> crate::compile::elab::DefTable {
        crate::compile::elab::DefTable::new()
    }

    /// 风格必须活着从签名文本里出来（`{}` = 隐式、`()`/`->` = 显式）。
    #[test]
    fn telescope_keeps_binder_styles() {
        let (layers, result) = telescope("forall {α : Type 0}, (a : α) -> (A : α -> Prop) -> Prop")
            .expect("telescope");
        assert_eq!(layers.len(), 3);
        // 参数名是 fresh 名（防捕获），域里对 `α` 的引用也换成了 fresh 名。
        assert!(layers[0].name.starts_with('\u{0}'));
        assert!(matches!(&layers[1].domain, Expr::Ident { name, .. } if name == &layers[0].name));
        assert_eq!(layers[0].style, BinderKind::Implicit);
        assert_eq!(layers[1].style, BinderKind::Explicit);
        assert_eq!(layers[2].style, BinderKind::Explicit);
        assert!(
            matches!(result, Expr::Sort { .. }),
            "`Prop` 是 Sort，不是 Ident: {result:?}"
        );
        assert_eq!(leading_implicit(&layers), 1);
    }

    /// `(a b : T)` 的多 binder 折叠要逐层摊开，`->` 算匿名的显式层。
    #[test]
    fn telescope_unfolds_multi_binder_foralls_and_arrows() {
        // `Nat -> Nat` 是**一支箭头**（域 Nat、陪域 Nat），不是两支。
        let (layers, _) = telescope("forall (a b : Nat), Nat -> Nat").expect("telescope");
        assert_eq!(layers.len(), 3);
        assert!(layers[0].name.starts_with('\u{0}'));
        assert!(layers[1].name.starts_with('\u{0}'));
        assert_ne!(layers[0].name, layers[1].name);
        assert_eq!(layers[2].name, "");
        assert_eq!(layers[2].style, BinderKind::Explicit);
        assert_eq!(leading_implicit(&layers), 0);
    }

    /// 中间隐式（`(a) {β} (b)`）：前导隐式是 0 ⇒ 调用方走老路。
    #[test]
    fn leading_implicit_stops_at_the_first_explicit_layer() {
        let (layers, _) =
            telescope("forall (a : Nat) {β : Type 0} (b : β), Prop").expect("telescope");
        assert_eq!(leading_implicit(&layers), 0);
        let (layers, _) =
            telescope("forall {α : Type 0} (a : α) {β : Type 0} (b : β), Prop").expect("telescope");
        assert_eq!(leading_implicit(&layers), 1);
    }

    /// 路线 ①：`Set.mem {α} (a : α) …` 的 `α` 由第一个显式实参的类型解出。
    #[test]
    fn solve_prefix_reads_the_first_explicit_layer() {
        let (layers, _) = telescope("forall {α : Type 0}, (a : α) -> (A : α -> Prop) -> Prop")
            .expect("telescope");
        let actual_ty = crate::proof::parse_expr_text("Nat").expect("parse");
        let result = crate::proof::parse_expr_text("Prop").expect("parse");
        let solved = solve_prefix(
            &layers,
            &result,
            1,
            &[Some(actual_ty)],
            None,
            &empty_defs(),
            &|_| false,
        )
        .expect("solved");
        assert_eq!(solved.len(), 1);
        assert!(matches!(&solved[0], Expr::Ident { name, .. } if name == "Nat"));
    }

    /// 解不出的形状必须返回 `None`（调用方报专用码），**不许猜**。
    #[test]
    fn solve_prefix_scans_later_explicit_layers_too() {
        // `{α}` 只在第 2 层（`b : α`）出现 ⇒ 由**第 2 个**实参的类型解出
        // （`picks 3 b` 的形状）。
        let (layers, _) =
            telescope("forall {α : Type 0}, (a : Nat) -> (b : α) -> Prop").expect("telescope");
        assert_eq!(leading_implicit(&layers), 1);
        let nat = crate::proof::parse_expr_text("Nat").expect("parse");
        let prop = crate::proof::parse_expr_text("Prop").expect("parse");
        let result = crate::proof::parse_expr_text("Prop").expect("parse");
        let solved = solve_prefix(
            &layers,
            &result,
            1,
            &[Some(nat.clone()), Some(prop)],
            None,
            &empty_defs(),
            &|_| false,
        )
        .expect("solved");
        assert!(
            matches!(&solved[0], Expr::Sort { .. }),
            "`Prop` 是 Sort: {solved:?}"
        );
        // 后面没有任何层提到它 ⇒ 解不出（专用错误码，不猜）
        let (layers, result) =
            telescope("forall {α : Type 0}, (a : Nat) -> Nat").expect("telescope");
        assert!(solve_prefix(
            &layers,
            &result,
            1,
            &[Some(nat)],
            None,
            &empty_defs(),
            &|_| false
        )
        .is_none());
    }

    /// 路线 ②：参数只出现在**结果类型**里 ⇒ 由期望类型解出（`Or.inl` 的 `B`）。
    #[test]
    fn solve_prefix_reads_the_expected_type() {
        // `Or.inl {A B} (a : A) : Or A B`：`B` 不在任何后续层的域里。
        let (layers, result) =
            telescope("forall {A : Prop} {B : Prop}, (a : A) -> Or A B").expect("telescope");
        assert_eq!(leading_implicit(&layers), 2);
        let a_ty = crate::proof::parse_expr_text("Nat").expect("parse");
        // `a : Nat`，期望 `Or Nat Bool` ⇒ `A := Nat`（路线①）、`B := Bool`（路线②）
        let expected = crate::proof::parse_expr_text("Or Nat Bool").expect("parse");
        let solved = solve_prefix(
            &layers,
            &result,
            2,
            &[Some(a_ty)],
            Some(&expected),
            &empty_defs(),
            &|_| false,
        )
        .expect("solved");
        assert_eq!(solved.len(), 2);
        assert!(matches!(&solved[0], Expr::Ident { name, .. } if name == "Nat"));
        assert!(matches!(&solved[1], Expr::Ident { name, .. } if name == "Bool"));
        // 没有期望类型 ⇒ **严格档**解不出（不许猜）✓ —— 逃生门
        // `SOKO_NOTATION_METAVAR=0` 走的就是这一档。
        assert!(solve_prefix_impl(
            &layers,
            &result,
            2,
            &[Some(crate::proof::parse_expr_text("Nat").unwrap())],
            &[],
            None,
            &empty_defs(),
            &|_| false,
            false,
        )
        .is_none());
        // **默认开**（2026-10-01 用户拍板；同轮复核这条后果后接受 ✓）：同一条
        // 输入走 [`solve_prefix`] ⇒ 待定档按"**域同形 ⇒ 跟已解兄弟**"把 `B` 解成 `A`
        // 的值 `Nat`（`{A B : Prop}` 两层域同形）。⚠ 这是**有意**的接受面变宽（刀2 的
        // 规则本身）；"严格档不许猜"由上面那条钉死 ✓。
        let guessed = solve_prefix(
            &layers,
            &result,
            2,
            &[Some(crate::proof::parse_expr_text("Nat").unwrap())],
            None,
            &empty_defs(),
            &|_| false,
        )
        .expect("默认态：待定档把 `B` 与同形兄弟 `A` 合一");
        assert!(
            matches!(&guessed[1], Expr::Ident { name, .. } if name == "Nat"),
            "`B` 必须跟同形兄弟 `A := Nat` 的值：{guessed:?}"
        );
    }

    /// **E19 刀2 的待定档**（真值层，直接调 [`solve_prefix_pending`]，不碰环境）：
    /// `{α β : Type}` 两位，`α` 那一位**解不出**（实参类型拿不到），`β` 由第二个
    /// 实参的类型解出 ⇒ `α` 与**同形的已解兄弟** `β` 合一 ✓；
    /// **两个都解不出**（没有兄弟可依）⇒ 仍 `None`（不猜、不发明类型 ✓）；
    /// **严格档**在同样输入上仍是 `None` ⇒ 刀2 不改它的语义 ✓。
    #[test]
    fn pending_solver_unifies_same_shape_siblings_and_never_guesses() {
        let (layers, result) =
            telescope("forall {α : Type 0} {β : Type 0}, (A : Set α) -> (B : Set β) -> Prop")
                .expect("telescope");
        assert_eq!(leading_implicit(&layers), 2);
        let beta_ty = crate::proof::parse_expr_text("Set β").expect("parse");
        // ① 第一位拿不到类型（`∅`），第二位是 `Set β` ⇒ `α := β`（同形合一）。
        let solved = solve_prefix_pending(
            &layers,
            &result,
            2,
            &[None, Some(beta_ty.clone())],
            &[],
            None,
            &empty_defs(),
            &|_| false,
        )
        .expect("pending solved");
        assert_eq!(solved.len(), 2);
        assert!(
            matches!(&solved[0], Expr::Ident { name, .. } if name == "β"),
            "待定位必须跟同形兄弟 `β` 合一：{solved:?}"
        );
        assert!(matches!(&solved[1], Expr::Ident { name, .. } if name == "β"));
        // ② 两位都拿不到 ⇒ 没有兄弟可依 ⇒ `None`（不猜）。
        assert!(solve_prefix_pending(
            &layers,
            &result,
            2,
            &[None, None],
            &[],
            None,
            &empty_defs(),
            &|_| false
        )
        .is_none());
        // ③ **严格档**（`allow_pending = false` —— 逃生门 `SOKO_NOTATION_METAVAR=0`
        //    走的就是它）在同样的输入上仍是 `None` ✓：待定档只在**追加**，不改它。
        assert!(solve_prefix_impl(
            &layers,
            &result,
            2,
            &[None, Some(beta_ty.clone())],
            &[],
            None,
            &empty_defs(),
            &|_| false,
            false,
        )
        .is_none());
        // ④ **默认开**（2026-10-01 用户拍板）：同一条输入走 [`solve_prefix`]（读开关）
        //    ⇒ 待定档接管、解出来了 ✓（③ 与 ④ 一起钉死"默认开 = 严格档 + 待定档"）。
        assert!(solve_prefix(
            &layers,
            &result,
            2,
            &[None, Some(beta_ty)],
            None,
            &empty_defs(),
            &|_| false
        )
        .is_some());
    }
}
