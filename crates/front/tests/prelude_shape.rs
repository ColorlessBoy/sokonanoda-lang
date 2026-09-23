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
