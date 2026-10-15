//! **判据的输入面**：把「用户会输入什么」变成判据的常规输入（生成式夹具）。
//!
//! ## 为什么有这个模块（`AGENT-DOES-NOT-PRODUCE-USER-INPUTS`，2026-10-15，high）
//!
//! 用户 2026-10-15 17:4x 的结构性诊断（`SUB-produce-the-users-inputs`）：
//!
//! > 我每次稍微**做两道题**，就发现一堆问题，但是 Agent 好像不太会找问题。
//!
//! 主因**不是**"仪表看不见"（那条已被实测否掉：CLI 明明能报 `entry-parse-failed` ✓）
//! —— 是 **Agent 从不喂那个输入** ✗：它跑判据时喂的是 `courses/**` 的**成品文件**
//! （252 份课程文件、123 份带 `sorry` ⇒ 目标总是关的），而用户手上是**过程态**
//! （写一半的证明、删掉一行的中间状态）。`SUB-parse-error-kills-whole-file` 那条
//! bug 要求"**目标未关的 `by` 块**"，而那个输入**此前从来没有被生成过** ✗ ——
//! 修它时补的两条用例（`crates/front/src/tests/parser_boundary.rs`）是**事后手写的
//! 单个实例**，不是"这一类输入"的生成器 ⇒ 同类换个 tactic（`cases` 分支 binder）
//! 就没人管 ✗；而**本模块第一次跑起来就抓到了一条修复没盖住的活 bug**
//! （见 [`Family::Negative`] 与 `crates/front/src/prefix_recovery.rs`）。
//!
//! ⇒ 本模块把用户那 10 条特征里**可机械化的**部分做成生成器：
//!
//! | 家族 | 用户的说法 | 生成什么 |
//! |------|-----------|---------|
//! | [`Family::ProcessState`] | ②"输入是过程态，不是成品态" | 目标未关的 `by` 块 + **后面还接着别的命令**（4 处参数表 × 3 种后续命令 × 有无空行 = 24 份） |
//! | [`Family::Differential`] | ③"做差分测试" | 同一输入的变体对：带/不带 `sorry`、**删掉一行**、换个收尾 tactic |
//! | [`Family::Negative`] | ④"做阴性观察" | 一条坏命令夹在好命令中间 ⇒ **该有的还在不在** |
//! | [`Family::Boundary`] | ⑩"给可复算的最小现场" | 未关块在文件尾（对照）/ 后面跟两条 / 后续命令被缩进 |
//!
//! ## 用法（**判据侧**，不是产品侧）
//!
//! * 真相层 —— `crates/front/tests/process_state_fixtures.rs`（解析结构）
//! * 入口层 —— `crates/cli/tests/process_state_entry.rs`（`query check` 的 counts）
//!
//! ⚠ **这里是"输入"，不是"期望"**：`expect_*` 只记**该有的在不在**（阴性断言要用的
//! 那几个数），**不是**"应该没有诊断" ✗ —— 后者正是被诊断点名的"只做阳性断言"病根。
//!
//! ⚠ **本模块不进判定路径**：它只产出源文本，不碰 lexer/parser/judge ✓。

/// 夹具家族 —— 逐条对应用户报告的输入特征（见模块文档的表）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    /// ②过程态：目标未关的 `by` 块**后面还接着别的命令**。
    ProcessState,
    /// ③差分：同一输入的变体（带/不带 `sorry`、删掉一行、换收尾 tactic）。
    Differential,
    /// ④阴性观察：一条坏命令夹在好命令中间 ⇒ "该有的还在不在"。
    Negative,
    /// ⑩边界变体：未关块在文件尾 / 后面跟两条 / 后续命令被缩进。
    Boundary,
}

impl Family {
    /// 家族名（判据的输出里用它定位是哪一类输入红了）。
    pub fn name(self) -> &'static str {
        match self {
            Family::ProcessState => "过程态",
            Family::Differential => "差分",
            Family::Negative => "阴性观察",
            Family::Boundary => "边界",
        }
    }
}

/// **阴性断言**：这份输入里"**该有的**"是什么。
///
/// * `declarations` —— **真相层**：解析出的顶层命令**条数**（精确；一条都不许丢）；
/// * `judged` / `open` / `examples` —— **入口层**：`decl_checked` / `exercise_open` /
///   `example_checked` 的**下界**（"该有的在" ✓）。
///
/// ⚠ 入口层用**下界**而不是精确值是有意的：恢复式解析会把"出错那条命令的前半截"
/// 也救回来（`graph/recovery.rs` 自己写着这条 ✓）⇒ 精确值会钉住一个**实现细节**；
/// 而"下界"钉住的正是用户看得见的那件事 —— **声明列表回来了没有** ✓。
#[derive(Clone, Copy, Debug)]
pub struct Expect {
    pub declarations: usize,
    pub judged: usize,
    pub open: usize,
    pub examples: usize,
}

/// 一份生成出来的用户输入。
#[derive(Clone, Debug)]
pub struct UserInput {
    /// 稳定 id（判据失败时按它定位；**不含**任何机器相关的路径/时间）。
    pub id: String,
    pub family: Family,
    /// 生成的 `.sokonanoda` 源文本。
    pub source: String,
    /// 为什么"用户会输入这个"（一句话，引用户原话或实测）。
    pub why: &'static str,
    pub expect: Expect,
}

// ─────────────────────────── 生成器的零件 ───────────────────────────

/// **未关目标**的收尾 tactic —— 恰好覆盖修复时动过的**四处参数表**
/// （`SUB-parse-error-kills-whole-file` 的 commit message：`intro` / `unfold` /
/// `unfold … at` / `cases` 分支 binder）。
///
/// ⚠ 修那次只补了 `intro` 与 `unfold … at` **两条**用例（`parser_boundary.rs`）
/// ⇒ 另两处**没人管** ✗ —— 这正是"修实例不修类"；本生成器把四处**全**覆盖 ✓。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Terminator {
    /// `intro h` —— 名字串。
    Intro,
    /// `unfold myid` —— ident 串（没有 `at`）。
    Unfold,
    /// `unfold myid at h` —— `at` 后面的假设名串。
    UnfoldAt,
    /// `cases h with | …` —— 分支 binder 串。
    Cases,
}

/// `unfold` 家族的辅助定义名（`unfold` 要一个可展开的 `def`；用局部变量名会
/// 让声明**语义**失败、把入口层读数搅浑 ⇒ 这里用真 def，读数才可归属 ✓）。
///
/// **公开**是给判据当锚点用的：`crates/front/tests/process_state_fixtures.rs`
/// 用它断言"`def` 的名字与 `unfold` 的目标**是同一个**"（否则夹具会静默变成
/// 一条语义失败的声明，入口层读数就不再可归属 ✗）。
pub const HELPER: &str = "myid";

impl Terminator {
    pub fn name(self) -> &'static str {
        match self {
            Terminator::Intro => "intro",
            Terminator::Unfold => "unfold",
            Terminator::UnfoldAt => "unfold-at",
            Terminator::Cases => "cases",
        }
    }

    /// 这条未关块**前面**要不要垫一条辅助 `def`（`unfold` 家族要）。
    fn helper(self) -> String {
        match self {
            Terminator::Unfold | Terminator::UnfoldAt => {
                format!("def {HELPER} (p : Prop) : Prop := p\n\n")
            }
            Terminator::Intro | Terminator::Cases => String::new(),
        }
    }

    /// 这份夹具在真相层的**顶层命令条数**（辅助 def + 未关的 `a`）。
    fn decls(self) -> usize {
        match self {
            Terminator::Unfold | Terminator::UnfoldAt => 2,
            Terminator::Intro | Terminator::Cases => 1,
        }
    }

    /// 未关块所在声明的**头**（缩进两格的 tactic 体见 [`Terminator::body`]）。
    fn head(self) -> String {
        match self {
            Terminator::Intro => "theorem a (p : Prop) : p → p := by\n".to_string(),
            Terminator::Unfold => {
                format!("theorem a (p : Prop) : {HELPER} p → p := by\n")
            }
            Terminator::UnfoldAt => {
                format!("theorem a (p : Prop) (h : {HELPER} p) : p := by\n")
            }
            Terminator::Cases => "theorem a (p : Prop) : p ∨ p → p := by\n".to_string(),
        }
    }

    /// 未关块的 tactic 体（**最后一步不关目标** ✓）。
    fn body(self) -> String {
        match self {
            Terminator::Intro => "  intro h\n".to_string(),
            Terminator::Unfold => format!("  unfold {HELPER}\n"),
            Terminator::UnfoldAt => format!("  unfold {HELPER} at h\n"),
            Terminator::Cases => {
                "  intro h\n  cases h with\n    | inl hb => exact hb\n".to_string()
            }
        }
    }
}

/// 未关块**后面**接着的那条命令。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Follower {
    Theorem,
    Example,
    Def,
}

impl Follower {
    pub fn name(self) -> &'static str {
        match self {
            Follower::Theorem => "theorem",
            Follower::Example => "example",
            Follower::Def => "def",
        }
    }

    /// `tag` 只给**具名**的那些用（`example` 没有名字 ⇒ 传了也不出现 ✓）。
    fn source(self, tag: &str) -> String {
        match self {
            Follower::Theorem => {
                format!("theorem {tag} (q : Prop) : q → q := by\n  intro h\n  exact h\n")
            }
            Follower::Example => {
                "example (q : Prop) : q → q := by\n  intro h\n  exact h\n".to_string()
            }
            Follower::Def => format!("def {tag} : Prop → Prop := fun q => q\n"),
        }
    }

    /// 入口层的下界：这条后续命令**必须**被判定出来 ✓（"该有的在"）——
    /// 返回它是 `example`（`example_checked`）还是普通声明（`decl_checked`）。
    fn expect(self) -> (usize, usize) {
        match self {
            Follower::Theorem | Follower::Def => (1, 0),
            Follower::Example => (0, 1),
        }
    }
}

/// 未关块与后续命令之间**有没有空行**（用户的最小现场里两者都有 ✓）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Separator {
    Blank,
    Tight,
}

impl Separator {
    fn text(self) -> &'static str {
        match self {
            Separator::Blank => "\n",
            Separator::Tight => "",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Separator::Blank => "blank",
            Separator::Tight => "tight",
        }
    }
}

const TERMINATORS: [Terminator; 4] = [
    Terminator::Intro,
    Terminator::Unfold,
    Terminator::UnfoldAt,
    Terminator::Cases,
];
const FOLLOWERS: [Follower; 3] = [Follower::Theorem, Follower::Example, Follower::Def];
const SEPARATORS: [Separator; 2] = [Separator::Blank, Separator::Tight];

// ─────────────────────────── 四个家族 ───────────────────────────

/// ②过程态：**目标未关的 `by` 块 + 后面还接着命令** ——
/// `Terminator` × `Follower` × `Separator` 的**全组合**（4 × 3 × 2 = 24 份）。
///
/// 这是 `SUB-parse-error-kills-whole-file` 那一类；**改前**只有两条手写实例
/// （`intro` / `unfold … at`）⇒ 本函数把**四处参数表**都变成常规输入 ✓。
pub fn process_state_inputs() -> Vec<UserInput> {
    let mut out = Vec::new();
    for term in TERMINATORS {
        for fol in FOLLOWERS {
            for sep in SEPARATORS {
                let (_checked, examples) = fol.expect();
                let src = format!(
                    "{}{}{}{}",
                    term.helper(),
                    term.head(),
                    term.body(),
                    format_args!("{}{}", sep.text(), fol.source("b"))
                );
                out.push(UserInput {
                    id: format!(
                        "process-state/{}/{}/{}",
                        term.name(),
                        fol.name(),
                        sep.name()
                    ),
                    family: Family::ProcessState,
                    source: src,
                    why: "用户②「输入是过程态，不是成品态」：写一半的证明（目标未关）后面还接着别的命令",
                    // 辅助 def（若有）+ 未关的 `a` + 后续命令：**每一条都该被判定** ✓
                    expect: Expect {
                        declarations: term.decls() + 1,
                        judged: term.decls() + 1,
                        open: 1,
                        examples,
                    },
                });
            }
        }
    }
    out
}

/// ③差分：同一输入的**变体对** —— 判据要断言"变体之间只差该差的那一点"。
///
/// 三对：
/// 1. `sorry` 有/无（用户原话："`apply` **带 `sorry` / 不带 `sorry`** 两版对比"）；
/// 2. **删掉一行**（收尾 tactic 去掉 ⇒ 目标未关）；
/// 3. 收尾 tactic 换成另一处参数表（`intro` ↔ `cases`）——
///    修复时**只补了 `intro` 那一条**用例 ⇒ 这一对正是"类"的覆盖 ✓。
pub fn differential_pairs() -> Vec<(UserInput, UserInput)> {
    let tail = "theorem b (q : Prop) : q → q := by\n  intro h\n  exact h\n";
    let pair = |id: &str, source: String, why: &'static str, expect: Expect| UserInput {
        id: id.to_string(),
        family: Family::Differential,
        source,
        why,
        expect,
    };
    let two_open1 = Expect {
        declarations: 2,
        judged: 2,
        open: 1,
        examples: 0,
    };

    let p1 = pair(
        "differential/sorry/with",
        format!("theorem a (p : Prop) : p → p := by\n  intro h\n  sorry\n\n{tail}"),
        "用户③「做差分测试」：`apply` 带 `sorry` / 不带 `sorry` 两版对比",
        two_open1,
    );
    let p2 = pair(
        "differential/sorry/without",
        format!("theorem a (p : Prop) : p → p := by\n  intro h\n\n{tail}"),
        "同上，**不带 `sorry`** 的那一版（目标未关）",
        two_open1,
    );

    let p3 = pair(
        "differential/delete-line/full",
        format!("theorem a (p : Prop) : p → p := by\n  intro h\n  exact h\n\n{tail}"),
        "用户③「**删掉那行**再看」—— 删之前的完整版",
        Expect {
            declarations: 2,
            judged: 2,
            open: 0,
            examples: 0,
        },
    );
    let p4 = pair(
        "differential/delete-line/deleted",
        format!("theorem a (p : Prop) : p → p := by\n  intro h\n\n{tail}"),
        "用户③「删掉那行」—— 用户 2026-10-15 17:3x 真的删了 `apply Eq.subst …` 那一行",
        two_open1,
    );

    let p5 = pair(
        "differential/terminator/intro",
        format!("theorem a (p : Prop) : p → p := by\n  intro h\n\n{tail}"),
        "同一形状换个**收尾 tactic**（`intro`）—— 修复只补了这条的用例",
        two_open1,
    );
    let p6 = pair(
        "differential/terminator/cases",
        format!(
            "theorem a (p : Prop) : p ∨ p → p := by\n  intro h\n  cases h with\n    | inl hb => exact hb\n\n{tail}"
        ),
        "同一形状换成 `cases` 分支 binder —— **修复时没人管**的那一处参数表",
        two_open1,
    );

    vec![(p1, p2), (p3, p4), (p5, p6)]
}

/// ④阴性观察：**一条坏命令夹在好命令中间** ⇒ 判据要断言"**该有的还在**"。
///
/// 用户的读法不是"报错了"，是「前两行的 **goal 都没了**」「声明列表**不回来**」✓
/// ⇒ 判据必须断言**视图里有内容**（`a` 与 **`c`** 两条都要被判定），
/// **不能**只断言"没有诊断" ✗。
///
/// ⚠ **这一族第一次跑起来就是红的**（2026-10-15 实测）：`a` 之前的声明照常判定，
/// 但**出错那条命令之后**的 `c` 全部报 `elab-tactic-failed … 前缀源码无法解析` ✗
/// ⇒ 修复只盖住了"出错之前的那些"。根因与修法见
/// `crates/front/src/prefix_recovery.rs`（同族 G-86 的第二次现场）。
pub fn negative_observation_inputs() -> Vec<UserInput> {
    let good_before = "theorem a (p : Prop) : p → p := by\n  intro h\n  exact h\n";
    let good_after = "theorem c (r : Prop) : r → r := by\n  intro h\n  exact h\n";
    // 坏命令的三种形状：缺类型 / 多余的括号 / 坏 tactic。
    [
        (
            "missing-type",
            "theorem broken (p : Prop) : := by\n  intro h\n",
        ),
        (
            "stray-parens",
            "theorem broken (p : Prop) : p → p := by\n  intro h\n  )))\n",
        ),
        (
            "bad-tactic",
            "theorem broken (p : Prop) : p → p := by\n  intro h\n  exact\n",
        ),
    ]
    .into_iter()
    .map(|(tag, broken)| UserInput {
        id: format!("negative/{tag}"),
        family: Family::Negative,
        source: format!("{good_before}\n{broken}\n{good_after}"),
        why: "用户④「做阴性观察」：注意到**少了什么**（`声明列表不回来`），不是\"报错了\"",
        // 三条顶层声明；**该有的** = 两条干净定理（`a` 与 **`c`**）都被判定 ✓
        // （出错的那条 `broken` 不算 —— 它本来就该报诊断；恢复式解析还可能把它
        //  的前半截救回来当开放练习，所以入口层用**下界** ✓）。
        expect: Expect {
            declarations: 3,
            judged: 2,
            open: 0,
            examples: 0,
        },
    })
    .collect()
}

/// ⑩边界变体：未关块在**文件尾**（对照：改前也解析正常 ✓）/ 后面跟**两条** /
/// 后续命令**被缩进**（`colGt` 边界另一侧）。
pub fn boundary_inputs() -> Vec<UserInput> {
    let tail = "theorem b (q : Prop) : q → q := by\n  intro h\n  exact h\n";
    let mk = |id: &str, source: String, why: &'static str, expect: Expect| UserInput {
        id: id.to_string(),
        family: Family::Boundary,
        source,
        why,
        expect,
    };
    vec![
        mk(
            "boundary/unclosed-at-eof",
            "theorem a (p : Prop) : p → p := by\n  intro h\n".to_string(),
            "用户⑩最小现场的**对照**：同一个未关块放在文件末尾（后面没有声明）",
            Expect {
                declarations: 1,
                judged: 1,
                open: 1,
                examples: 0,
            },
        ),
        mk(
            "boundary/two-followers",
            format!(
                "theorem a (p : Prop) : p → p := by\n  intro h\n\n{tail}\
                 theorem c (r : Prop) : r → r := by\n  intro h\n  exact h\n"
            ),
            "边界变体：未关块后面跟着**两条**命令（不只一条）",
            Expect {
                declarations: 3,
                judged: 3,
                open: 1,
                examples: 0,
            },
        ),
        mk(
            "boundary/unclosed-then-indented",
            format!("theorem a (p : Prop) : p → p := by\n  intro h\n\n  {tail}"),
            "边界变体：未关块后面那条命令**被缩进了**（`colGt` 的边界另一侧）",
            Expect {
                declarations: 2,
                judged: 2,
                open: 1,
                examples: 0,
            },
        ),
    ]
}

/// 全部夹具（四个家族）—— 判据的**常规输入**入口。
pub fn all() -> Vec<UserInput> {
    let mut out = process_state_inputs();
    out.extend(negative_observation_inputs());
    out.extend(boundary_inputs());
    for (a, b) in differential_pairs() {
        out.push(a);
        out.push(b);
    }
    out
}
