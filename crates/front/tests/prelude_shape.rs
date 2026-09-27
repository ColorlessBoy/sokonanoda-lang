//! **K2 复用的守卫**（T-K20 / 设计 `docs/design/closure-incremental.md` §2.1）。
//!
//! 内核预装（`Nat`/`Bool`/`Eq`/L1）是**按整个闭包**决定的 ⇒ 共享一份编译环境
//! 之前必须证明两个闭包的 prelude 形状相同，否则会**静默改变判卷**。
//!
//! 计划里要求的是"**脚本化验证** `courses/set-theory` 每个入口的 prelude 形状
//! 是否相同（调研说'是'，但那是实测结论、不是代码保证）"。这个文件就是那个
//! 保证：课程里**每一个** `.sokonanoda`（lib + units + solutions）的闭包形状
//! 都必须一模一样。**哪天有人给某个单元加一行 `inductive Nat`，这里立刻红。**

use sokonanoda_front::compile::{prelude_shape, PreludeShape};
use sokonanoda_front::project;
use std::path::{Path, PathBuf};

fn course_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../courses/set-theory")
}

/// 课程里全部 `.sokonanoda`（lib / units / solutions / 顶层）。
fn all_sources() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![course_dir()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "sokonanoda") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// 算一个入口的**闭包**形状（走真的闭包加载，不是只看这个文件）。
fn shape_of(entry: &Path) -> Option<PreludeShape> {
    let text = std::fs::read_to_string(entry).ok()?;
    let plan = project::plan_project(entry, Some(&text), None);
    // 闭包模块**已经解析好了**（`LoadedModule.file`）——不必再 parse 一遍。
    let units: Vec<sokonanoda_front::compile::SourceUnit<'_>> = plan
        .modules()
        .iter()
        .map(|m| sokonanoda_front::compile::SourceUnit {
            name: &m.name,
            path: Some(&m.path),
            file: &m.file,
        })
        .collect();
    if units.is_empty() {
        return None;
    }
    Some(prelude_shape(&units))
}

#[test]
fn every_course_entry_has_the_same_prelude_shape() {
    let sources = all_sources();
    assert!(
        sources.len() > 20,
        "课程里应该有几十个 .sokonanoda，实际 {}",
        sources.len()
    );
    let mut shapes: Vec<(PathBuf, PreludeShape)> = Vec::new();
    for path in &sources {
        if let Some(shape) = shape_of(path) {
            shapes.push((path.clone(), shape));
        }
    }
    assert!(!shapes.is_empty(), "至少要能算出一个入口的形状");
    let (first_path, first) = &shapes[0];
    for (path, shape) in &shapes[1..] {
        assert_eq!(
            shape,
            first,
            "prelude 形状不一致：{} vs {}（K2 复用会因此静默改变判卷）",
            first_path.display(),
            path.display()
        );
    }
    // 形状本身也要有内容：课程**不**自带 Nat/Bool（用 prelude 的），
    // 也不该撞 prelude 的名字。
    assert!(!first.explicit_nat, "课程不该自带 inductive Nat");
    assert!(!first.explicit_bool, "课程不该自带 inductive Bool");
    assert!(
        first.shadowed.is_empty(),
        "课程不该撞 prelude 的名字：{:?}",
        first.shadowed
    );
}

/// **T-D11 的判据**：unit01 的记法表里 `∈` 指向 `lib/Set.sokonanoda` 的声明行。
///
/// 以前这张表在加载期算完就丢（`exports` 是局部量）⇒ "跳转"没有数据可用。
#[test]
fn the_entry_notation_table_points_at_the_declaring_module() {
    let entry = course_dir().join("units/unit01-sets-membership.sokonanoda");
    let text = std::fs::read_to_string(&entry).expect("读入口");
    let plan = project::plan_project(&entry, Some(&text), None);
    let report = project::compile_plan(plan, &Default::default());
    let mem = report
        .notations
        .iter()
        .find(|n| n.symbol == "∈")
        .expect("入口可见 `∈`（它声明在被 import 的库里）");
    assert_eq!(mem.target, "Set.mem");
    assert_eq!(
        mem.module.as_deref(),
        Some("lib.Set"),
        "声明它的模块（不是入口）"
    );
    // 声明点的文本**逐字**是那一行——这是"跳转"要落到的位置。
    let lib = std::fs::read_to_string(course_dir().join("lib/Set.sokonanoda")).expect("读库");
    let line = &lib[mem.span.start.offset..mem.span.end.offset];
    assert!(
        line.contains("infix:50") && line.contains("Set.mem"),
        "声明点必须落在 `infix:50 \" ∈ \" => Set.mem` 那一行：{line:?}"
    );
}

/// **E01 的判据（真相层）**：卷 I 的**复合记法**真的声明在库里、且跨 `import` 可见。
///
/// 符号与优先级都是**取证过**的，不是随手挑的数字：
///
/// * `∘` = `infixr:90 " ∘ " => Function.comp` —— Lean 4 core 逐字
///   （`src/Init/Notation.lean:274`：`@[inherit_doc] infixr:90 " ∘ "  => Function.comp`）；
/// * `•` = `infixr:80 " • " => Rel.comp` —— **本课自定**（PLAN v0.74.0 指定用 `•`）。
///   ⚠ 顺带纠正一条**原来写错的引用**：`lib/Rel.sokonanoda` 头部曾写"Mathlib
///   `Relation.comp`，即记法 `r • s`"——**Mathlib 用的是 `∘r`，而且是 `local`**：
///   `Mathlib/Logic/Relation.lean:158` 逐字是
///   `local infixr:80 " ∘r " => Relation.Comp`。本课取 `•` 是为了与 `Function.comp`
///   的 `∘` 区分（同一个符号在同一闭包里只能有一个目标），优先级沿用 Mathlib 那一档 80。
///
/// 这一层守的是**真相**：`report.notations` 是入口闭包的记法表（T-D11）——它红了，
/// 说明库没声明、或声明没随 `import` 传播、或优先级/目标名被改动。
#[test]
fn the_course_libraries_declare_the_composition_notations() {
    use sokonanoda_front::NotationAssoc;

    let entry = course_dir().join("units/unit12-synthesis.sokonanoda");
    let text = std::fs::read_to_string(&entry).expect("读入口");
    let plan = project::plan_project(&entry, Some(&text), None);
    let report = project::compile_plan(plan, &Default::default());

    let find = |sym: &str| {
        report
            .notations
            .iter()
            .find(|n| n.symbol == sym)
            .unwrap_or_else(|| panic!("入口可见 `{sym}`（它声明在被 import 的库里）"))
    };
    let decl_line = |module: &str, span: sokonanoda_front::Span| {
        let src = std::fs::read_to_string(course_dir().join(format!("{module}.sokonanoda")))
            .unwrap_or_else(|e| panic!("读 {module}: {e}"));
        src[span.start.offset..span.end.offset].to_string()
    };

    let comp = find("•");
    assert_eq!(comp.target, "Rel.comp");
    assert_eq!(comp.precedence, Some(80));
    assert_eq!(comp.assoc, NotationAssoc::Infixr);
    assert_eq!(comp.module.as_deref(), Some("lib.Rel"));
    let line = decl_line("lib/Rel", comp.span);
    assert!(
        line.contains("infixr:80") && line.contains("Rel.comp"),
        "声明点必须落在 `infixr:80 \" • \" => Rel.comp` 那一行：{line:?}"
    );

    let fun = find("∘");
    assert_eq!(fun.target, "Function.comp");
    assert_eq!(fun.precedence, Some(90));
    assert_eq!(fun.assoc, NotationAssoc::Infixr);
    assert_eq!(fun.module.as_deref(), Some("lib.Fun"));
    let line = decl_line("lib/Fun", fun.span);
    assert!(
        line.contains("infixr:90") && line.contains("Function.comp"),
        "声明点必须落在 `infixr:90 \" ∘ \" => Function.comp` 那一行：{line:?}"
    );
}

/// **E02 的判据（真相层）**：`×ˢ` 的目标 `Set.prod` 住在**库里**（`lib.Prod`）。
///
/// 为什么这条重要：记法 `×ˢ` 由 `lib/Set` 声明，而它的**目标**原先只活在单元⑤ 的
/// 画布里 ⇒ 任何"只 `import` 库"的闭包写 `s ×ˢ t` 都报
/// `elab-notation-unknown-target`（"记法 `×ˢ` 指向的目标 `Set.prod` 不存在"）。
/// 收进 `lib/Prod` 之后目标在库内 ⇒ 闭包自足（这也是 E07「跨模块目标」的一半）。
#[test]
fn the_set_product_notation_target_lives_in_the_library() {
    use sokonanoda_front::NotationAssoc;

    let entry = course_dir().join("units/unit05-pairs-products.sokonanoda");
    let text = std::fs::read_to_string(&entry).expect("读入口");
    let plan = project::plan_project(&entry, Some(&text), None);
    let report = project::compile_plan(plan, &Default::default());

    let prod = report
        .notations
        .iter()
        .find(|n| n.symbol == "×ˢ")
        .expect("入口可见 `×ˢ`（它声明在被 import 的 lib.Set 里）");
    assert_eq!(prod.target, "Set.prod");
    assert_eq!(prod.precedence, Some(80));
    assert_eq!(prod.assoc, NotationAssoc::Infixr);
    assert_eq!(
        prod.module.as_deref(),
        Some("lib.Set"),
        "记法本身仍声明在 lib.Set（目标搬了、声明点没搬）"
    );

    // 目标由**库**提供：`lib.Prod` 的声明表里有一条 checked 的 `Set.prod`。
    let lib = report
        .module("lib.Prod")
        .expect("闭包里有 lib.Prod（单元⑤ import 了它）");
    let decl = lib
        .report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("Set.prod"))
        .expect("`Set.prod` 必须由 lib.Prod 提供（E02 收进库）");
    assert!(
        matches!(decl.status, sokonanoda_front::compile::DeclStatus::Checked),
        "lib.Prod 里的 `Set.prod` 必须 checked：{:?}",
        decl.status
    );

    // 画布自己**不再**声明它——同名重声明会被判 `import-name-collision`。
    let entry_module = report.entry_module().expect("入口模块报告");
    assert!(
        !entry_module
            .report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("Set.prod")),
        "画布不该再声明 `Set.prod`（副本必须删掉，否则与 lib.Prod 撞名）"
    );
}

/// **E03 的判据（真相层）**：`r ⁻¹`（`Rel.inv`）与 `A ≈ B`（`Set.Equiv`）两条记法。
///
/// 取证（E03）：**两条都是本课自定** —— mathlib4 master 里查不到它们的声明
/// （`Mathlib/Logic/Relation.lean` 只有 `local infixr:80 " ∘r " => Relation.Comp`；
/// `Mathlib/Logic/Equiv/Defs.lean:80` 的 `infixl:25 " ≃ " => Equiv` 是**等价的类型**，
/// 不是集合等势；`Mathlib/SetTheory/Cardinal/Basic.lean` 无 `≈`）。符号按数学书读法取。
#[test]
fn the_course_libraries_declare_the_inverse_and_equinumerous_notations() {
    use sokonanoda_front::NotationAssoc;

    let entry = course_dir().join("units/unit12-synthesis.sokonanoda");
    let text = std::fs::read_to_string(&entry).expect("读入口");
    let plan = project::plan_project(&entry, Some(&text), None);
    let report = project::compile_plan(plan, &Default::default());

    let find = |sym: &str| {
        report
            .notations
            .iter()
            .find(|n| n.symbol == sym)
            .unwrap_or_else(|| panic!("入口可见 `{sym}`（它声明在被 import 的库里）"))
    };
    let decl_line = |module: &str, span: sokonanoda_front::Span| {
        let src = std::fs::read_to_string(course_dir().join(format!("{module}.sokonanoda")))
            .unwrap_or_else(|e| panic!("读 {module}: {e}"));
        src[span.start.offset..span.end.offset].to_string()
    };

    let inv = find("⁻¹");
    assert_eq!(inv.target, "Rel.inv");
    assert_eq!(inv.precedence, Some(100));
    assert_eq!(inv.assoc, NotationAssoc::Postfix);
    assert_eq!(inv.module.as_deref(), Some("lib.Rel"));
    let line = decl_line("lib/Rel", inv.span);
    assert!(
        line.contains("postfix:100") && line.contains("Rel.inv"),
        "声明点必须落在 `postfix:100 \" ⁻¹ \" => Rel.inv` 那一行：{line:?}"
    );

    let equiv = find("≈");
    assert_eq!(equiv.target, "Set.Equiv");
    assert_eq!(equiv.precedence, Some(50));
    assert_eq!(equiv.assoc, NotationAssoc::Infix);
    assert_eq!(equiv.module.as_deref(), Some("lib.Equiv"));
    let line = decl_line("lib/Equiv", equiv.span);
    assert!(
        line.contains("infix:50") && line.contains("Set.Equiv"),
        "声明点必须落在 `infix:50 \" ≈ \" => Set.Equiv` 那一行：{line:?}"
    );

    // ⚠ `⁻¹`（逆关系）与 `⁻¹'`（原像，lib/Set）是**两个符号**：词法按声明最长匹配，
    // 两条必须同时出现在闭包记法表里（单元⑧⑫ 同文件共存）。
    let preimage = find("⁻¹'");
    assert_eq!(preimage.target, "Set.preimage");
    assert_eq!(preimage.module.as_deref(), Some("lib.Set"));
}
