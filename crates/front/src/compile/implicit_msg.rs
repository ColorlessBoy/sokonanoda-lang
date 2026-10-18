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

/// **`cases` 的被消去项类型**（a15）：类型文本读不出来时**把内层真错交出来** ✓ ——
/// 旧文案「`cases` 读不到 `h` 的类型」把真错（假设类型里 `Set.univ α` 的严格档拒绝）
/// **吞掉**了 ✗（a12 的「吞诊断」族：用户看不见该改哪 ✗，实测
/// `unit04-extensionality-identities:53` —— `h : x ∈ A ∪ Set.univ α` 里的
/// `Set.univ α` 才是真凶，而文案只报"读不到 h 的类型"）。
/// 判定**一个字没动**：只换文案（`Err` 还是 `Err` ✓）。
pub(crate) fn cases_scrutinee_type(
    name: &str,
    r: &Result<String, crate::judge::Judgement>,
    span: crate::Span,
) -> Result<Expr, crate::compile::CompileError> {
    let fail = |inner: Option<&str>| {
        crate::compile::CompileError::elab(
            crate::compile::ErrorKind::ElabTacticFailed,
            match inner {
                Some(m) => format!("`cases` 读 `{name}` 的类型时失败：{m}"),
                None => format!("`cases` 读不到 `{name}` 的类型"),
            },
            span,
        )
    };
    match r {
        Ok(text) => crate::proof::parse_expr_text(text).map_err(|_| fail(None)),
        Err(crate::judge::Judgement::Error { message, .. }) => Err(fail(Some(message))),
        Err(_) => Err(fail(None)),
    }
}

/// **严格档开关**（**默认开** ✓；`SOKO_STRICT_AT=0|off|false` ⇒ 关）。
///
/// 严格档 = **正常形式（没写 `@`）不接受位置隐式实参** ✓，与 Lean 4
/// `Lean/Elab/App.lean:706` `processImplicitArg` 的
/// `if (← read).explicit then processExplicitArg argName else addImplicitArg argName`
/// **同构**（`:709` = `@` 让隐式 binder 也吃一个位置实参 ✓；`:711` = 正常形式给隐式
/// binder 插元变量、**不碰实参表** ✓）。
///
/// ⭐ **2026-10-18 翻默认**（`SUB-implicit-args-strict-default` ✓）—— 用户 10-16 那条
/// 需求的**最后一步**：此前默认关的**唯一**理由是**语料没迁完**（一翻默认整语料
/// 从"接受"变"拒绝" ✗），而迁移已按棒做完（a1 诊断带真实行号 · a4 构造子糖只标 `@` ·
/// a5 引擎装配/回读的全显式脊 + `courses/set-theory/lib/**` · a6 其余语料按诊断
/// 「出路 ①」逐处加 `@`）⇒ **默认开** ✓。逐文件账见 `.cache/ux/fix-a6/VERDICT.md`。
///
/// ⚠⚠ **逃生门必须留** ✗（`SOKO_STRICT_AT=0|off|false` ⇒ 关）：遇到没迁完的语料
/// （第三方文件、旧课程仓）时，**一条环境变量就能退回旧行为**，不必回滚版本 ✓。
/// **极性**：未设、或取值不认识 ⇒ **开**（默认即严格档）；只有上面三个字面量表示"关" ✓
/// —— `SOKO_STRICT_AT=1|on|true` 仍照旧表示"开"（旧脚本/CI 一行不用改 ✓）。
///
/// **a5（2026-10-17）· 第二处消费者**：`elab::operand_type_expr` 的 **pp 档**。那份文本是
/// **内核 pp** 打的，回读成项后是「正常形式写满实参」的应用（实测
/// `Eq (fst A B (P.mk A B a b)) a`）⇒ 严格档一律拒 ✗（**诊断落在 `line 1` = 文本坐标**，
/// 正是"这份文本不是用户写的"的指纹 ✓）。全显式 pp 打的是 `@Eq α x a` ⇒ 回读即 `@` 形态
/// ⇒ 逐位对齐 ✓；默认档恒 `false` ⇒ **逐字节不变** ✓。同族：`expr::e_app_at`（引擎装配的
/// 全显式脊要自己标 `@` ✓）。
pub(crate) fn strict_at_enabled() -> bool {
    !matches!(
        std::env::var("SOKO_STRICT_AT").ok().as_deref(),
        Some("0") | Some("off") | Some("false")
    )
}

/// **a15**：引擎自己产出、**马上要回读**的文本（目标/子目标的规范形态）要不要走
/// **全显式 pp**（`@f a b` 形态）。
///
/// 为什么必须（实测最小复现）：目标由 β 归约得到时（`Reflexive A (fun (a b : A) => a = b)`
/// 走 `intro` ⇒ 目标成为 `Eq A a a`），引擎把目标 pp 成「正常形式写满实参」的文本、
/// 再回读 ⇒ 严格档一律拒 ✗ —— **用户写对了也判红**，且诊断给的出路（`@a = a`）
/// 在源文件里**根本不存在** ⇒ 不可执行 ✗。
/// 对齐 Lean 4：那边 `pp.explicit`（`PrettyPrinter/Delaborator/Options.lean:154`）
/// 打的就是 `@Eq α a b`；而 Lean 的 goal 始终是 `MVarId`+`Expr`，**从不回读自己的
/// pp 文本** —— 我们有意偏离（`by` 引擎的目标是文本）⇒ 偏离的代价由这条补上
/// （a5 已对 `operand_type_expr` 的 pp 档这么做 ✓）。
pub(crate) fn engine_text_pp(on: bool) -> bool {
    on || strict_at_enabled()
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
    // **a15**：`@` 只加在**头名**上 ✗ —— 以前加在**整条应用的渲染文本**上，
    // 而渲染是记法感知的：`Eq.{1} A a a` 渲染成 `a = a` ⇒ 出路印成 `@a = a`
    // （**在源文件里根本不存在** ✗，a13 点名的那条）。剥不掉（渲染形状不一致）
    // ⇒ 退回只给 `@<头> …`（同 `unsolved_message`：**绝不印一条错的出路** ✗）。
    let escape = match whole_text.strip_prefix(head_text) {
        Some(rest) => format!("@{} {}", head_text, rest.trim_start()),
        None => format!("@{} …", head_text),
    };
    format!(
        "`{}` 的签名 `{}` 里，{}。\
         正常形式（没写 `@`）的**位置实参只按显式形参逐位绑定**，隐式形参由推断补\
         —— 这与 Lean 4 一致；你写了 {} 个位置实参，多出来的落不到任何形参上。\n\
         出路 ①（**全显式**：连隐式位也逐位写下来）⇒ 在函数名前加 `@`：`{}`\n\
         出路 ②（**让推断补**：删掉喂给隐式位的那些实参，只留显式形参对应的）\
         ⇒ 按显式形参顺序只写 {} 个实参。",
        head_text,
        ty_text,
        shape_sentence(names.as_ref(), k, total_layers),
        n_args,
        escape,
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
///
/// ⚠⚠ **a15：旧「出路 ②」已删** ✗ —— 它写的是「给整个表达式一个类型标注
/// （`def … : T := 这里`）或给相关实参加类型标注」，而**本子集没有 `(e : T)` 这条
/// 语法**（实测 `(5 : Nat)` / `by exact (5 : Nat)` / `(g (α := Nat) 5 : Nat)` **一律
/// `unexpected-token`** ✗）；`by` 位里也没有"整个表达式的类型标注"这种写法 ⇒
/// 那条出路**不可执行** ✗。按用户 2026-10-17 硬令（「给用户的出路必须是可执行的」）
/// **去掉它**，只留能照抄的那一条 ✓（`a1` 的教训：给用户的出路必须是可执行的 ✗）。
pub(crate) fn unsolved_message(
    head_text: &str,
    whole_text: &str,
    ty_text: &str,
    k: usize,
    why: &str,
) -> String {
    let names = source_layer_names(ty_text);
    let implicit_list: Vec<String> = (0..k)
        .map(|i| format!("`{}`", name_of(names.as_ref(), i)))
        .collect();
    // 头文本与整条应用的原文都从**同一份渲染**取 ⇒ 前缀能逐字剥掉 ✓；
    // 剥不掉（渲染形状不一致）⇒ 退回只给 `@<头> …`（**绝不印一条错的出路** ✗）。
    let rest = whole_text.strip_prefix(head_text);
    let placeholders: Vec<String> = (0..k)
        .map(|i| format!("<{}>", name_of(names.as_ref(), i)))
        .collect();
    let escape = match rest {
        Some(rest) => format!(
            "@{} {} {}",
            head_text,
            placeholders.join(" "),
            rest.trim_start()
        ),
        None => format!("@{} {} …", head_text, placeholders.join(" ")),
    };
    format!(
        "`{}` 的签名 `{}` 里，前 {} 个形参是**隐式**的（按位置依次是 {}）——\
         这里**至少有一个没解出来**{}。\n\
         出路（**全显式**：连隐式位也逐位写下来）⇒ 在函数名前加 `@`，\
         并在**最前面**把这 {} 个隐式实参按顺序补上（`<…>` 是占位符）：`{}`",
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

/// **命名实参点名失败**（用户 2026-10-16 第 ② 条）：`f (α := Nat)` 里的 `α`
/// 不是 `f` 的形参名。
///
/// **Lean 4 对照**：`Lean/Elab/App.lean:39 throwInvalidNamedArg` —— 一个码 +
/// **列出可用形参名** + 一条 suggestion（code action）。我们照同一条契约：
/// 把**全部**源级形参名按位置印出来，并给**可复制**的改正形态 ✓。
pub(crate) fn unknown_named_arg_message(
    name: &str,
    names: &[(String, BinderKind)],
    ty_text: &str,
) -> String {
    let all: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(i, (n, _))| {
            let shown = if n.is_empty() {
                format!("第 {} 位（匿名）", i + 1)
            } else {
                format!("`{n}`")
            };
            shown
        })
        .collect();
    format!(
        "命名实参 `({} := …)` 里的 `{}` **不是**这个函数的形参名。\
         它的签名是 `{}`，按位置可用的形参名依次是：{}。\n\
         改法：把名字改成上面列出的某一个（例如 `({} := …)`）；\
         或者改用 `@` + **位置**实参（隐式位也要逐位写下来）。",
        name,
        name,
        ty_text,
        if all.is_empty() {
            "（无）".to_string()
        } else {
            all.join("、")
        },
        names
            .iter()
            .find(|(n, _)| !n.is_empty())
            .map(|(n, _)| n.as_str())
            .unwrap_or("α"),
    )
}

/// **同一个形参给了两次**（`f (α := Nat) (α := Bool)`）。
///
/// **Lean 4 对照**：那边 `findBinderName?` 只取**第一个**命中并把条目从
/// `namedArgs` 里删掉（`App.lean:1073`）⇒ 第二个**留在表里** ⇒ 走到
/// `:238 throwInvalidNamedArg`（"unused named arguments"）⇒ 同样是**报错** ✓。
pub(crate) fn duplicate_named_arg_message(name: &str, ty_text: &str) -> String {
    format!(
        "形参 `{name}` 用命名实参给了**两次**（签名 `{ty_text}`）。\
         一个形参只能给一次 —— 删掉其中一个（Lean 4 同样报错，只是它的措辞是 \
         `unused named arguments`）。"
    )
}

/// **命名实参在位、但**剩下的隐式前缀仍然补不出来。
///
/// 与 [`unsolved_message`] 同一个码（`elab-implicit-argument-unsolved`，D6 不新增 ✓），
/// 只把出路换成**命名实参**那条（这才是"指令明确"：用户刚用过的语法就是出路 ✓）。
pub(crate) fn named_arg_unsolved_message(ty_text: &str, k: usize) -> String {
    let names = source_layer_names(ty_text);
    let implicit_list: Vec<String> = (0..k)
        .map(|i| format!("`{}`", name_of(names.as_ref(), i)))
        .collect();
    format!(
        "`{}` 的签名 `{}` 里，前 {} 个形参是**隐式**的（按位置依次是 {}）——\
         你用命名实参给了其中一部分，**剩下那些补不出来**（本子集只按后续位置实参的\
         类型反解，不做搜索）。\n\
         出路 ①（**按名字给全**）⇒ 把没给的那几个也用命名实参写上，例如 `({} := …)`。\n\
         出路 ②（**全显式**）⇒ 在函数名前加 `@`，把这 {} 个隐式实参按**位置**顺序写全。",
        "这个函数",
        ty_text,
        k,
        if implicit_list.is_empty() {
            "（无）".to_string()
        } else {
            implicit_list.join("、")
        },
        name_of(names.as_ref(), 0),
        k,
    )
}

/// **命名实参落在了钩子认不出的头上**（不是已知常量、是局部假设、或与富余实参组合）。
///
/// **Lean 4 对照**：那边任何 `forall` 都带 binder 名，所以局部假设也能按名绑定
/// （`App.lean:753` 的 `fType.bindingName!`）—— **我们还没有那条路**（如实 ✗，
/// 见 `named_arg` 模块头的「与 Lean 的差异」）。这条文案就是那个边界的**明说**：
/// 给出两条**现在就能用**的出路 ✓。
pub(crate) fn misplaced_named_arg_message(name: &str) -> String {
    format!(
        "命名实参 `({name} := …)` 只能按**已知常量自己的形参名**绑定；这里的函数头不是\
         已知常量（常见：它是 `intro`/`have` 进来的**局部假设**，或者位置实参比形参还多）。\n\
         出路 ①（**位置实参 + `@`**）⇒ 在函数名前加 `@`，把隐式位也逐位写下来。\n\
         出路 ②（**让推断补**）⇒ 删掉命名实参，只按显式形参的顺序写位置实参。"
    )
}
