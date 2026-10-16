//! `ErrorKind::hint()` 的**文案表**（从 `compile/error.rs` 拆出来，2026-10-16）。
//!
//! **为什么单独一个文件**：`error.rs` 在 `scripts/docs-lint.py` 判据 ⑨ 的**存量冻结**
//! 清单里（**只许减不许增** ✗）。本轮要给它加 `ElabInvalidNamedArgument`（用户
//! 2026-10-16 第 ② 条「支持显式地指定参数名字」的错误码）⇒ 按 `prefix_recovery.rs`
//! 的先例，把这一整段搬出来，`error.rs` 那边只留一行转发 ✓。

use super::error::ErrorKind;

/// 见 [`ErrorKind::hint`]（实现搬走了，这里只留转发 ✓）。
pub(crate) fn hint(kind: ErrorKind) -> &'static str {
    use ErrorKind::*;
    match kind {
            ElabUnknownIdentifier => {
                "这个名字还没有被定义。检查拼写，或确认它出现在你前面的某个声明里（练习要在解决之后才能被后面的代码引用）。若它在该命名空间里，检查前缀或加 open。"
            }
            ElabUnknownConstant => {
                "这里引用了一个不存在的常量。如果它带宇宙参数，请先定义它。"
            }
            ElabUniverseLevelConflict => {
                "这个常量的宇宙层级写错了：写出来的 `.{n}` 与实参类型要求的层级对不上。把它改成提示里的那个数，或者整段 `.{…}` 省掉让引擎自己解。"
            }
            ElabUnknownUniverseLevel => {
                "这个宇宙层级变量没有在当前声明里声明。用 {u} 声明它，例如 def id {u} : ...。"
            }
            ElabUniverseArity => {
                "宇宙参数个数不对。这个常量声明了几个宇宙参数，就要给几个，例如 id.{u, v}。"
            }
            ElabUntypedBinder => {
                "这个 binder 缺少类型标注。教学版本要求写全类型，例如 fun (x : Nat) => x；let 的绑定也要写类型，例如 let x : Nat := 1; x。"
            }
            ElabHoleMisplaced => {
                "sorry 只能出现在声明的值（答案区）位置，例如 example : T := sorry。"
            }
            ElabDuplicateDeclaration => {
                "这个名字已经定义过了。Lean 里每个名字只能声明一次；换一个名字，或删掉前面的声明。"
            }
            ElabTooManyBinders => {
                "嵌套的 binder 太多，超出了内核能表示的深度。把大表达式拆成几个小定义。"
            }
            ElabNatLiteralDisabled => {
                "数字字面量没有被启用。这个版本默认打开 Nat 扩展，如遇到此错误请联系工具作者。"
            }
            ElabInvalidNatLiteral => {
                "这不是一个合法的自然数字面量。"
            }
            ElabTooManyCtorFields => {
                "构造子的字段太多了。"
            }
            ElabUnknownCtorForIota => {
                "iota 规则引用了一个不存在的构造子。检查构造子名字是否与 ctor 声明一致。"
            }
            ElabAmbiguousCtorAlias => {
                "这个裸构造子名被两个类型各声明了一次，无法判断是哪一个。写全前缀名（例如 `P1.mk`），或给其中一个构造子换个名字。"
            }
            ElabTacticFailed => {
                "`by` 块里的 tactic 失败了：请检查当前目标与已引入的假设。"
            }
            ElabApplyNeedsATerm => {
                "`funapply` 后面要跟一个证明或函数，例如 `funapply h`；要应用的项复杂时可以用括号界定范围，例如 `funapply (f a)`。"
            }
            ElabApplyNotApplicable => {
                "`funapply h` 要求 `h` 的结论正好是当前目标（`h : … -> 目标`）。看看 `h` 类型的最后一段是不是当前目标；不是就换一个前提，或直接写答案。"
            }
            ElabMatchBadArm => {
                "match 的模式要写对：构造子名与字段数要对应（可以嵌套，如 some (succ k)）；不确定的名字按变量绑定处理，带子模式的未知名才报错。"
            }
            ElabMatchNotInductive => {
                "match 的被匹配项必须是已知的归纳类型：本文件用 inductive 声明的类型，或 prelude 内建的 Nat/Bool（分支写 Nat.zero/Nat.succ 或 Bool.true/Bool.false）。"
            }
            ElabMatchNoExpectedType => {
                "match 的结果类型必须已知：把它放在有类型标注的位置（声明类型、let/fun 的 binder 注解），或由外层 match 提供。"
            }
            ElabMatchRecursiveUnsupported => {
                "match 暂不支持递归归纳类型（v1 只做没有归纳假设的非递归分情况）；递归定义请直接用消去子 .rec。"
            }
            ElabMatchNonExhaustive => {
                "match 要覆盖该归纳类型的每一个构造子（含每个字段位置）；带守卫的 arm 还要在后面补一条不带守卫的兜底。"
            }
            ElabMatchParameterizedUnsupported => {
                "参数化归纳的 match 目前只支持：被匹配项是一个局部变量，且它的类型写成 `T 参数…`（显式给出归纳的全部参数）。换成一个这样标注的变量再 match。"
            }
            ElabLetTypeQueryFailed => {
                "无法从值推断出 `let` 绑定的类型；补上类型标注即可，例如 `let x : Nat := 1; x`。"
            }
            ElabNotationUnknownTarget => {
                "记法命令指向的目标名不存在。检查 infix/notation 行里 `=>` 后面的名字拼写（要写点名，例如 Set.mem），并确认它已经声明过。"
            }
            ElabNotationArgumentUnsolved => {
                "这个记法展开时补不出前面的类型参数（本子集只按操作数的类型补，不做一般推断）。改用点名写法把参数写全，例如 Set.mem α a A；或在两边都是已知类型的上下文里使用记法。"
            }
            ElabImplicitArgumentUnsolved => {
                "这个声明的签名里有**隐式**参数（`{α : Type}` 这种），而那些参数补不出来（本子集只按后续显式实参的类型反解，不做搜索）。出路：① 在函数名前加 `@`，把隐式位也逐位写下来（`@f α a`）；② 给整个表达式或相关实参加类型标注，让推断有线索。⚠ 直接写 `f α a`（不加 `@`）**不接受** —— 位置实参只按显式形参逐位绑定（与 Lean 4 一致）。"
            }
            ElabNotationAmbiguous => {
                "同一个符号声明了多条记法（重载），这里从期望类型看不出该用哪一条。写出点名形式（例如 Set.mem α a A）就消歧了；或者把这个表达式放到一个带类型标注的位置（例如 def … : T := 这里），让期望类型能定下来。"
            }
            ElabNotationNoCandidate => {
                "同一个符号的几条记法候选，结果类型都对不上这里的期望类型。对照错误里列出的候选结果类型，检查是不是用错了符号，或者改用点名形式。"
            }
            ElabBinderNotationUnsolved => {
                "binder 记法里的变量类型解不出：`∀ x ∈ s, p` / `∃ x ∈ s, p` 的 x 类型是从 `∈` 两边反解的。给 binder 补上类型标注（例如 ∀ (x : α) ∈ s, p），或改用点名写法（forall (x : α), Set.mem α x s -> p）。"
            }
            ElabSetLiteralUnknownTarget => {
                "集合字面量 `{a}` / `{a, b}` 展开成点名形式 Set.singleton / Set.pair，但这个文件里没有它们。先 `import` 提供它们的库（卷 I 的 lib/Set），或改用点名写法。"
            }
            ElabInvalidNamedArgument => {
                "命名实参 `(名字 := 值)` 只能按**函数自己的形参名**绑定，而且那个函数必须是一个**已知常量**（导入的或本文件前面声明过的）。三种常见原因：① 名字拼错了 —— 照错误里列出的可用形参名改；② 同一个形参给了两次 —— 删掉一个；③ 函数是一个**局部假设**（`have`/`intro` 进来的），本子集还不能按名字绑定它 —— 用 `@` 加位置实参，或把那个假设写成带名字的 `have h : … := …` 再应用。"
            }
            ElabAnonCtorNoExpectedType => {
                "`⟨a, b⟩` 是匿名构造子：用哪个构造子（And.intro / Exists.intro / Prod.mk / Iff.intro）由**期望类型**决定，而这里读不到期望类型。把它写进有类型标注的位置（`have h : A ∧ B := ⟨…⟩`、`theorem … : T := ⟨…⟩`、`exact ⟨…⟩`），或改用点名构造子。"
            }
            KernelExpectedSort => {
                "这里需要写一个类型（如 Prop、Type、Nat），但你写成了一个普通的项。两种常见原因：\
                 ① **点名调用漏了前导类型参数**——库里 `Set.mem α a A` 的参数是有序的，少写最前面的 \
                 `α` 就会把 `a` 顶到类型位上（`Set.mem a A` 是错的）；用记法 `a ∈ A` 可以让编译器\
                 自动补 `α`。② 冒号或 binder 后面跟的是个值而不是类型。"
            }
            KernelExpectedPi => {
                "你把一个不是函数的值当函数用了，或者参数给多了。检查这个位置的东西的类型是不是 … -> … 形状。"
            }
            KernelTheoremNotProp => {
                "theorem 的类型必须是命题（Prop 里的东西）。想定义普通值请用 def。"
            }
            KernelPropNotCumulative => {
                "这里需要 Type（数据），但你给的是 Prop（命题）：本语言没有累积性，Prop 不是 Type 的子集——官方 Lean 4 同样没有（它要用 PLift 显式把命题抬进 Type），所以这不是内核缺陷。把陈述改成 Prop（例如等势用 Set.Equiv … : Prop 这样的命题版），或者交一个真正的 Type 值（如 Nat）。"
            }
            KernelNonPositive => {
                "递归引用出现在了负位置：构造子参数里 T 出现在箭头左边（如 T → Nat）。递归引用只能写在返回类型一侧。"
            }
            KernelCtorResultMismatch => {
                "构造子的返回类型必须是本 inductive 的完整应用——参数和索引都要补齐，比如 T A n 而不是只写 T。"
            }
            KernelCtorArgInvalidApp => {
                "构造子参数里的递归引用 T … 不是本 inductive 的合法应用：参数/索引的个数或取值不对。"
            }
            KernelCtorArgNotType => {
                "构造子参数的类型本身必须是一个类型，这里写成了一个项。"
            }
            KernelCtorArgTooLarge => {
                "构造子参数的类型所在的宇宙太大，装不进这个 inductive。把 inductive 声明成更大的 Type，或把该参数类型改成 Prop 里的命题。"
            }
            KernelRecRuleMismatch => {
                "显式消去子（rec/iota）与内核推导出的规则不一致：iota 规则必须按构造子声明顺序一条不落地给出，每条规则的值也要与 motive、minor 和构造子参数的形状完全匹配。"
            }
            KernelRejected => {
                "内核判定不成立：类型不匹配或证明项不完整。先对比期望类型与你的值的形状；最常见的错误是两边结构不同（例如期望 a -> a，却写成了返回 Nat 的项）。"
            }
            KernelInternal => {
                "内核内部错误（这不是你的代码问题）。请把这段代码反馈给工具作者。"
            }
            ImportNotFound => {
                "找不到这个模块。`import Foo.Bar` 对应模块根下的 `Foo/Bar.sokonanoda`；检查名字拼写与文件位置（名字里不能有 `-`）。"
            }
            ImportCycle => {
                "import 成环了：A 依赖 B、B 又（直接或间接）依赖 A。把公共部分抽到一个更底层的模块里。"
            }
            ImportDependencyFailed => {
                "被导入的模块自己还有错误，所以这里先不编译——修好那个文件的第一条错误再看这里。"
            }
            ImportNameCollision => {
                "同一个名字在两个模块里各声明了一次。改掉其中一个，或把它移到一个公共模块里只声明一次。"
            }
            ImportPreludeConflict => {
                "prelude（内置基元）是整个编译单元的属性：入口文件的设置与依赖模块冲突了。统一成一种（要么都用内置 prelude，要么都在入口声明 `-- sokonanoda:prelude none`）。"
            }
            ManifestInvalid => {
                "`sokonanoda.toml` 读不了或不是合法 TOML。它只需要可选的 `name` / `requires` / `src` 三个键；不想要项目就把文件删掉（单文件模式仍然可用）。"
            }
            ImportModuleInvalid => {
                "被导入的文件本身没解析成功：先打开它、修好那里的语法错误，再回来看这里（下游不会替你猜）。"
            }
            EntryParseFailed => {
                "这个文件自己没解析成功：按上面那条诊断的位置改**本文件**，与它 import 的文件无关（那些文件没有被牵连）。"
            }
    }
}
