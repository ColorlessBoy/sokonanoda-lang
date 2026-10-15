//! **`#check <裸 def 名>` 的声明卡片**（2026-10-15 用户新需求，`REQUIREMENTS.md` §9.22）。
//!
//! 从 `kernel_phase.rs` 切出来（判据 ⑨ 的源码行数红线：`kernel_phase.rs` 已在基线上，
//! **只许减不许增** ✗ ⇒ 新代码进**新文件**，`AGENTS.md` 硬规则 5 的正解 ✓）。
//!
//! ## 用户原话（2026-10-15 15:22）
//!
//! > 我想让 `#check Set.subset` 也支持针对 def 的优化，例如 `#check Set.subset` 显示
//! > `def Set.subset : ∀ {α : Type}, Set α → Set α → Prop`
//! > `:= fun {α : Type 0} (A B : Set α) => ∀ (x : α), x ∈ A → x ∈ B`
//!
//! **两件缺一不可** ✗：① **带定义体**（`:= …`）＋ ② **记法渲染**（`∀`/`→`/`∈`，
//! 不是 `forall`/`->`/`Set.mem`）。
//!
//! ## 形状：**与 Infoview 的声明卡片逐字节同形**（不是第二套渲染 ✗）
//!
//! * 类型那一半 = `display.fold`（线 C 的**唯一**转化接口 ✓）
//!   ＋ [`crate::lean4_universe::lean4_universe_text`]（`Type 0` → `Type`，与
//!   `lsp::render::lean4_universe_text` **同一份实现** ✓）；
//! * 定义体那一半 = `display.fold`（**不过** universe 规则 —— 与既有卡片的
//!   `value` 一半同款 ✓，见 LSP 的 `declaration_card`）。
//!
//! ⇒ `#check Set.subset` 印出来的就是**用户 hover 时已经看到的那张卡片**，
//! 命令行与编辑器终于同一个形状 ✓（2026-10-10 用户实测第三条的"收口统一"）。
//!
//! ## 边界（一条都不许含糊 ✗）
//!
//! * **只给 `Declar::Definition`**：`theorem`/`opaque`/`axiom`/`inductive`/
//!   构造子/递归子 ⇒ `None`（用户 2026-10-15 的判据：`#check <theorem>` 仍只打签名 ✓）。
//! * **拿不到就 `None`**：名字不在环境里 / pp 失败 ⇒ 不产卡片、显示退回
//!   `<源文本>: <类型>`（**诚实省略，不编** ✗）。
//! * **判定路径一个字都不动** ✗：`CheckEvent::TypeChecked.text` 仍是**类型**
//!   （`judge::judge_type_of` ⇒ 记法目标签名读的是它 ✓）；卡片只进 `decl` 字段。
//! * `pp` 档照 `PendingOp::Check` 的既有纪律套（`set_option pp.*` + G-71 的内部档），
//!   调用一结束立刻还原 ⇒ 不带 `set_option` 的文件**逐字节不变** ✓。
//! * 记法**只在记法表里有对应符号时**才用（`fold` 自己的契约）；没有的仍走点名形式 ✓。

use super::{apply_pp_flags, PpFlags};
use crate::display::DisplayNotations;
use sokonanoda::env::Declar;
use sokonanoda::util::{ExportFile, NamePtr};

/// `#check <裸 def 名>` 的卡片文本；不是 `def` / 拿不到 ⇒ `None`（见模块文档）。
pub(super) fn check_decl_card<'t>(
    env: &mut ExportFile<'t>,
    card: Option<&(String, NamePtr<'t>)>,
    display: &DisplayNotations,
    pp: PpFlags,
) -> Option<String> {
    let (name, ptr) = card?;
    let ptr = *ptr;
    // **只有 `def`**：`Declar::Definition` 才带 `val` 且**就是**"定义"
    // （`Theorem`/`Opaque` 也有 `val`，但它们的 `val` 是证明/不透明体 ⇒ 不给 ✗）。
    let (ty, val) = match env.declars.get(&ptr) {
        Some(Declar::Definition { info, val, .. }) => (info.ty, *val),
        _ => return None,
    };
    let saved_pp = env.config.pp_options.clone();
    apply_pp_flags(
        &mut env.config.pp_options,
        pp,
        crate::judge::explicit_pp_active(),
    );
    // `pp_expr` 走**与 `DeclState.ty_text`/`val_text` 同一条**内核 pp
    // （`pp_expr` ⇒ `render(width)`），不是第二条真相 ✓。
    let ty_text = env.with_pp(|pp| pp.pp_expr(ty));
    let val_text = env.with_pp(|pp| pp.pp_expr(val));
    env.config.pp_options = saved_pp;
    let ty_text = crate::lean4_universe::lean4_universe_text(&display.fold(&ty_text));
    let val_text = display.fold(&val_text);
    Some(format!("def {name} : {ty_text} := {val_text}"))
}

/// **`#check <裸标识符>` 的卡片目标**：规范名 + 内核指针（`None` = 不给卡片）。
///
/// 走 [`resolve_known`]（与 `#print` **同一条**口径：命名空间候选顺序 + 别名 +
/// 歧义 ✓）；**解不出不报错** ✗ —— `#check` 的错误通道归 `elab_expr`（它有自己的
/// 稳定错误码 ✓），这里只"没有卡片" ⇒ 显示逐字节回到今天 ✓。
pub(crate) fn card_target<'a>(
    known: &crate::compile::elab::KnownTable,
    ns: &crate::compile::scope::NamespaceScope,
    builder: &mut sokonanoda::builder::EnvBuilder<'a>,
    expr: &crate::Expr,
) -> Option<(String, NamePtr<'a>)> {
    match expr {
        crate::Expr::Ident { name, span } => {
            crate::compile::elab::resolve_known(known, ns, name, *span, &[])
                .ok()
                .map(|canonical| {
                    let ptr = builder.name_from_str(&canonical);
                    (canonical, ptr)
                })
        }
        _ => None,
    }
}
