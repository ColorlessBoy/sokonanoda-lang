//! **IA-5（`SUB-implicit-args-align-lean4`，用户 2026-10-16 08:5x 亲报）的错误文案**。
//!
//! 用户原话（逐字）：「**隐式参数的自动推导能力要强**；当然用户可能没考虑周到，以为能自动
//! 推导出来，这时**报错信息要完善，指令要明确**。否则就像我现在的体验一样，**一会要手动
//! 输入隐式参数，一会又不要**。」
//!
//! **为什么单独一个文件** ✗：`elab.rs`（8915 行）与 `implicit.rs`（1244 行）都在
//! `scripts/docs-lint.py` 判据 ⑨ 的**存量冻结**清单里（**只许减不许增** ✗）⇒
//! 文案长在那边会让 `docs-lint` 当场判红 ✓（实测踩过 ✓）。
//!
//! **用户第 ③ 条的三样**（判据在 `crates/cli/tests/implicit_at_alignment.rs` ✓）：
//! **(a) 点名哪个参数**（**源级**名 + 在签名里的位置 —— 不许印 `\0soko_p2` 这种 fresh 名 ✗）·
//! **(b) 给出确切出路**（`@` 那条**必须照抄就能判绿** ✓ —— 测试会把出路抠出来重判 ✓）·
//! **(c) 不说"把参数写全"**（旧文案就是这句：**没说用哪个语法** ✗ —— 用户点名的反例 ✓）。
//!
//! **Lean 4 对照**（`~/Documents/lean/lean4/src/Lean/Elab/App.lean`，本机源码 ✓）：
//! 那里这两条分别报 `function expected`（`Meta/InferType.lean:76`）与
//! `Application type mismatch`（`Meta/Check.lean:254`）。我们沿用**既有码**
//! `ElabImplicitArgumentUnsolved`（**不新增码**，设计 D6 ✓），只把文案做完善 ✓。

use crate::ast::{BinderKind, Expr};

/// **严格档开关**（`SOKO_STRICT_AT=1|on|true` ⇒ 开）。
///
/// 严格档 = **正常形式（没写 `@`）不接受位置隐式实参** ✓，与 Lean 4
/// `Lean/Elab/App.lean:706` `processImplicitArg` 的
/// `if (← read).explicit then processExplicitArg argName else addImplicitArg argName`
/// **同构**（`:709` = `@` 让隐式 binder 也吃一个位置实参 ✓；`:711` = 正常形式给隐式
/// binder 插元变量、**不碰实参表** ✓）。
///
/// ⚠⚠ **默认关** ✗ —— **不是"没做"** ✓，是**语料迁移还没做完**：本轮实测
/// **269/294** 个 `.sokonanoda` 会从"接受"变"拒绝"，其中 **19 个在
/// `docs/gaps/repro/**`**，而那个目录**不在本棒的写者范围内** ⇒ 一翻默认
/// `scripts/gap.py check` 当场判红 ✗。账见 `.cache/ux/fix-imp1/VERDICT.md`；
/// 翻默认 + 语料迁移 = 子任务 `SUB-implicit-args-strict-default` ✓。
pub(crate) fn strict_at_enabled() -> bool {
    matches!(
        std::env::var("SOKO_STRICT_AT").ok().as_deref(),
        Some("1") | Some("on") | Some("true")
    )
}

/// 签名文本里**源级**的形参名与风格，按位置返回。
///
/// `implicit::telescope` 把参数名一律换成 fresh 名（防捕获 ✓）⇒ 报错文案里印出来是
/// `\0soko_p2` 这种**没法读**的东西 ✗（实测 ✓）。用户第 ③ 条要求「**点名哪个参数**
/// 没解出来（名字 + 在签名里的位置）」⇒ 文案必须用**原文名** ✓。
///
/// ⚠ **只在报错路径上调用** ✓ ⇒ 快乐路径**零开销、逐字节不变** ✓。
pub(crate) fn source_layer_names(signature: &str) -> Option<Vec<(String, BinderKind)>> {
    let mut cur = crate::proof::parse_expr_text(signature).ok()?;
    let mut out: Vec<(String, BinderKind)> = Vec::new();
    loop {
        match cur {
            Expr::Forall { binders, body, .. } if !binders.is_empty() => {
                for b in &binders {
                    out.push((b.name.clone(), b.style.clone()));
                }
                cur = *body;
            }
            Expr::Arrow { codomain, .. } => {
                out.push((String::new(), BinderKind::Explicit));
                cur = *codomain;
            }
            _ => return Some(out),
        }
    }
}

/// 第 `i` 层的**可读名**：源级名优先；匿名 `->` 层或取不到源名 ⇒ 用位置占位
/// （**绝不印 fresh 名** ✗）。
fn name_of(names: Option<&Vec<(String, BinderKind)>>, i: usize) -> String {
    match names.and_then(|v| v.get(i)).map(|(n, _)| n.clone()) {
        Some(n) if !n.is_empty() => n,
        _ => format!("第 {} 位", i + 1),
    }
}

/// 「有 N 个**隐式**形参（按位置依次是 …），后面的显式形参依次是 …」这句公共前缀。
///
/// ⚠ **隐式/显式的分界取注册表的 `k`，不取 `ty_text` 的风格** ✗：构造子的 kernel
/// 类型里参数是**显式** binder（Lean 也是），只有注册表知道它们**语义上隐式**
/// （`And.intro` 的 `{a b : Prop}` ✓）—— 取风格会把 `And.intro` 的隐式位印成"显式"，
/// 与 `k` 的说法**自相矛盾** ✗（实测踩过 ✓）。
fn shape_sentence(names: Option<&Vec<(String, BinderKind)>>, k: usize, total: usize) -> String {
    let implicit_list: Vec<String> = (0..k).map(|i| format!("`{}`", name_of(names, i))).collect();
    let explicit_list: Vec<String> = (k..total)
        .map(|i| format!("`{}`", name_of(names, i)))
        .collect();
    format!(
        "前 {} 个形参是**隐式**的（按位置依次是 {}），后面的显式形参依次是 {}",
        k,
        if implicit_list.is_empty() {
            "（无）".to_string()
        } else {
            implicit_list.join("、")
        },
        if explicit_list.is_empty() {
            "（无）".to_string()
        } else {
            explicit_list.join("、")
        },
    )
}

/// **严格档的拒绝文案**：正常形式写了位置隐式实参（实参比显式层多，且富余的落不到结果上）。
///
/// **Lean 4 对照**：那里报 `function expected`（`Meta/InferType.lean:76`）/
/// `Application type mismatch`（`Meta/Check.lean:254`）。
pub(crate) fn strict_rejection_message(
    head_text: &str,
    whole_text: &str,
    ty_text: &str,
    k: usize,
    total_layers: usize,
    explicit_layers: usize,
    n_args: usize,
) -> String {
    let names = source_layer_names(ty_text);
    format!(
        "`{}` 的签名 `{}` 里，{}。\
         正常形式（没写 `@`）的**位置实参只按显式形参逐位绑定**，隐式形参由推断补\
         —— 这与 Lean 4 一致；你写了 {} 个位置实参，多出来的落不到任何形参上。\n\
         出路 ①（**全显式**：连隐式位也逐位写下来）⇒ 在函数名前加 `@`：`@{}`\n\
         出路 ②（**让推断补**：删掉喂给隐式位的那些实参，只留显式形参对应的）\
         ⇒ 按显式形参顺序只写 {} 个实参。",
        head_text,
        ty_text,
        shape_sentence(names.as_ref(), k, total_layers),
        n_args,
        whole_text,
        explicit_layers,
    )
}

/// **「补不出来」文案**（默认档也会走到 —— 用户 2026-10-15 亲见的就是它 ✓）。
///
/// 旧文案（逐字）：「`…` 的签名 `…` 里有 N 个**隐式**参数，但补不出来…。**把参数写全**，
/// 例如 `f …` 逐位写下来」⇒ 用户点名的两个问题：**没说用哪个语法** ✗ ·
/// **没点名是哪一个参数** ✗。本函数把两样都补上 ✓。
///
/// ⚠⚠ **出路① 必须真的能用** ✗：`@` 只关掉"插隐式实参"，它**不会**把隐式实参
/// **补出来** ⇒ `@ignores 3` 会把 `3` 装到 `{α}` 位上 ✗（实测：报 `期望 Sort(1)` ✗）
/// ⇒ 出路里必须**同时**让用户在最前面补上这 `k` 个隐式实参 ✓（用 `<α>` 这种占位符
/// 点名位置 ✓）。测试会把占位符填上再判一遍 ✓（**照做就对** = 「指令明确」的唯一硬判据 ✓）。
pub(crate) fn unsolved_message(
    head_text: &str,
    whole_text: &str,
    ty_text: &str,
    k: usize,
    why: &str,
) -> String {
    let names = source_layer_names(ty_text);
    let implicit_list: Vec<String> = (0..k).map(|i| format!("`{}`", name_of(names.as_ref(), i))).collect();
    // 头文本与整条应用的原文都从**同一份渲染**取 ⇒ 前缀能逐字剥掉 ✓；
    // 剥不掉（渲染形状不一致）⇒ 退回只给 `@<头> …`（**绝不印一条错的出路** ✗）。
    let rest = whole_text.strip_prefix(head_text);
    let placeholders: Vec<String> = (0..k).map(|i| format!("<{}>", name_of(names.as_ref(), i))).collect();
    let escape = match rest {
        Some(rest) => format!("@{} {} {}", head_text, placeholders.join(" "), rest.trim_start()),
        None => format!("@{} {} …", head_text, placeholders.join(" ")),
    };
    format!(
        "`{}` 的签名 `{}` 里，前 {} 个形参是**隐式**的（按位置依次是 {}）——\
         这里**至少有一个没解出来**{}。\n\
         出路 ①（**全显式**：连隐式位也逐位写下来）⇒ 在函数名前加 `@`，\
         并在**最前面**把这 {} 个隐式实参按顺序补上（`<…>` 是占位符）：`{}`\n\
         出路 ②（**给推断线索**：把这几位的类型从上下文里定下来）⇒ \
         给整个表达式一个类型标注（`def … : T := 这里`）或给相关实参加类型标注。",
        head_text,
        ty_text,
        k,
        if implicit_list.is_empty() {
            "（无）".to_string()
        } else {
            implicit_list.join("、")
        },
        why,
        k,
        escape,
    )
}
