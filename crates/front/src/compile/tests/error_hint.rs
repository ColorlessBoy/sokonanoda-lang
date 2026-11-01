//! **P2-12 S5 ④**：`ErrorKind::hint()` 文案表的**可执行性**判据 —— 死出路不许回来。
//!
//! ## 为什么有这个文件
//!
//! 2026-11-01 的 S3 读数（`reports/2026-11-01-p212-s3-readings.md` §6.2）与发版面清单
//! （`reports/2026-11-01-strict-to-release.md` §2bis）同时抓到一条**用户可见的真缺陷**：
//! `ElabImplicitArgumentUnsolved` 的 hint 还写着「出路 ② 给整个表达式或相关实参加类型标注」，
//! 而本子集**没有 `(e : T)` 这条语法**（实测 `(5 : Nat)` / `by exact (5 : Nat)` 一律
//! `unexpected-token`，见 `implicit_msg::unsolved_message` 的 a15 注释）。那条出路照抄就报错 ⇒
//! 违反用户 2026-10-17 的硬令「给用户的出路必须是可执行的」（`REQUIREMENTS.md` 的 a1 教训）。
//!
//! 本文件是那条缺陷的**守卫**：hint 里出现本子集做不到的写法 ⇒ 判红。
//!
//! ## 三条牙
//!
//! 1. `implicit_unsolved_hint_only_teaches_executable_escapes`：被点名那一支的 hint 里
//!    ① 旧「出路 ②」的措辞与 `(e : T)` / `（e : T）` 形状**一条都没有**；
//!    ② a15 的 `@a = a` 形状（把 `@` 加在**渲染后的整条应用**上 ⇒ 源里根本不存在）不许出现：
//!    任何带 `@` 的代码片段都不许含 `=`；
//!    ③ 反而**必须**有两条能照抄的出路（`@` 全显式 + 删掉多余位置实参），且必须写明
//!    「`@` 不替你补」—— 否则用户会写成 `@g 0`（实测：`3` 被装到 `{α}` 位上 ✗）。
//! 2. `no_hint_teaches_expression_type_ascription`：**所有**讲解「类型标注」的 hint 都不许用
//!    `(e : T)` 那种**表达式类型标注**的形状（反面：它们教的是 binder / `let` / `have` /
//!    声明类型上的标注 —— 那些本子集**有**）。
//! 3. 反向验证（读数里做过）：把 `error_hint.rs` 那一支换回旧文本 ⇒ 第 1 条判红。
//!
//! ⚠ **为什么只挑这几个 kind**：`ErrorKind` 没有 `ALL`，全表遍历要往 `error.rs`（`docs-lint` ⑨ 的
//! 存量冻结清单里）加一张表 ⇒ 不值当。挑的是「hint 里真的在教类型标注写法」的那几支 ——
//! 死出路只可能长在这里（其余 hint 与标注无关，写不出这个病）。

use super::{error_hint, ErrorKind};

/// 旧「出路 ②」的措辞碎片（**逐字**来自删掉的那一版）：任何一条回来都判红。
///
/// 分开列三个碎片而不是整句：改写措辞时只要还在教"给表达式/实参加类型标注"，总有一片命中。
const DEAD_END_PHRASES: [&str; 3] = [
    "给整个表达式或相关实参加类型标注",
    "相关实参加类型标注",
    "类型标注，让推断有线索",
];

/// 本子集**没有**的语法形状（`(e : T)` 的表达式类型标注）—— 全角括号也要挡。
const DEAD_END_SHAPES: [&str; 2] = ["(e : T)", "（e : T）"];

/// hint 里**在教「类型标注」怎么写**的那几支（死出路只可能长在这里）。
///
/// 每一支教的都是**本子集真有**的标注位置：binder（`fun (x : Nat) => x`）· `let`
/// （`let x : Nat := 1`）· `have`（`have h : A ∧ B := …`）· 声明类型（`def … : T := …`）·
/// binder 记法的类型位（`∀ (x : α) ∈ s, p`）。
const ANNOTATION_HINTS: [ErrorKind; 6] = [
    ErrorKind::ElabUntypedBinder,
    ErrorKind::ElabMatchNoExpectedType,
    ErrorKind::ElabLetTypeQueryFailed,
    ErrorKind::ElabNotationAmbiguous,
    ErrorKind::ElabBinderNotationUnsolved,
    ErrorKind::ElabAnonCtorNoExpectedType,
];

/// ① 被点名那一支：**只教能照抄的出路**。
#[test]
fn implicit_unsolved_hint_only_teaches_executable_escapes() {
    let h = error_hint::hint(ErrorKind::ElabImplicitArgumentUnsolved);

    // (a) 死出路的措辞与形状，一条都不许回来。
    for dead in DEAD_END_PHRASES {
        assert!(
            !h.contains(dead),
            "hint 里出现了**不可执行**的旧出路 ②（`{dead}`）—— 本子集没有 `(e : T)` 语法，\
             照抄会 `unexpected-token` ✗：{h}"
        );
    }
    for shape in DEAD_END_SHAPES {
        assert!(
            !h.contains(shape),
            "hint 里出现了表达式类型标注的形状 `{shape}`（本子集没有这条语法）✗：{h}"
        );
    }
    // (b) a15 的 `@a = a` 形状：`@` 只能加在**头名**上。把 `@` 加在渲染后的整条应用上
    //     （`a = a` 这种记法渲染）⇒ 源里根本不存在那个写法 ✗。
    for chunk in h.split('`') {
        assert!(
            !(chunk.contains('@') && chunk.contains('=')),
            "hint 里的代码片段 `{chunk}` 把 `@` 加到了含 `=` 的整条应用上 —— 那是 a15 的死形状\
             （`@` 只加在**函数名**前）✗：{h}"
        );
    }

    // (c) 两条**能照抄**的出路都必须在：
    //     ① `@` 全显式（连隐式位一起写）；② 删掉喂给隐式位的实参。
    assert!(
        h.contains("`@"),
        "hint 必须给出 `@` 这条确切出路（并在最前面补隐式实参）✗：{h}"
    );
    assert!(
        h.contains("删掉"),
        "hint 必须给出第二条**可执行**的出路（删掉喂给隐式位的实参，让推断去补）✗：{h}"
    );
    // (d) 必须说清 `@` 的语义：它只关掉自动补，**不会替你把隐式补出来** ——
    //     不说清用户就会写成 `@g 0`（实测：`3` 被装到 `{α}` 位上 ⇒ `期望 Sort(1)` ✗）。
    assert!(
        h.contains("不会替你补") || h.contains("只关掉"),
        "hint 必须写明 `@` 只关掉自动补、不会替你补隐式实参（否则用户写 `@g 0` 照错）✗：{h}"
    );
}

/// ② 横向：讲解「类型标注」的 hint 里，**没有**任何一条教 `(e : T)` 这种表达式类型标注。
///
/// 反面（正面清单）：它们教的是 binder / `let` / `have` / 声明类型上的标注 —— 那些本子集**有**。
#[test]
fn no_hint_teaches_expression_type_ascription() {
    for kind in ANNOTATION_HINTS {
        let h = error_hint::hint(kind);
        for shape in DEAD_END_SHAPES {
            assert!(
                !h.contains(shape),
                "`{kind:?}` 的 hint 教了表达式类型标注 `{shape}` —— 本子集没有这条语法 ✗：{h}"
            );
        }
        // 每条都要真的给**一个位置**（一个 `:` 或一个关键字形态），否则"补类型标注"是一句空话。
        assert!(
            h.contains(':') || h.contains("注解"),
            "`{kind:?}` 的 hint 只说「补类型标注」却没给可照抄的位置/写法 ✗：{h}"
        );
    }
}
