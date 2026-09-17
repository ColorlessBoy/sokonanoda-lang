//! 在**一个模块根**下把模块名解析成文件路径，并为「找不到」准备教学线索。
//!
//! 本期只做**查找**（纯文件系统、不读清单）：模块根由上层决定（`sokonanoda.toml`
//! 所在目录，或没有清单时的入口文件目录，见 `docs/design/imports-and-projects.md`
//! §4.4）。查找失败时收集两类最常踩的线索：
//!
//! * **大小写不一致**：macOS/Windows 的文件系统大小写不敏感，
//!   `import bar` 能骗过磁盘却骗不过 Linux CI；
//! * **文件名里有 `-`**：`unit1-propositions-proofs.sokonanoda` 的横线不是模块名
//!   字符，学生多半会写 `import unit1-propositions-proofs`——
//!   词法层会先报错，但 `import unit1` 这类写法也需要"你是想说 … 吗"的提示。

use std::fs;
use std::path::{Path, PathBuf};

use super::module_name::{ModuleName, MODULE_EXTENSION};

/// 查找结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// 期望路径存在且是文件。
    Found(PathBuf),
    /// 找不到：带上"试过哪条路径"与教学线索。
    Missing(MissingModule),
}

/// 找不到模块时的上下文（渲染进 `import-not-found` 诊断）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingModule {
    pub module: String,
    /// 期望的绝对/相对路径（`<root>/Foo/Bar.sokonanoda`）。
    pub expected: PathBuf,
    /// 大小写不一致但确实存在的同名文件。
    pub case_mismatch: Option<PathBuf>,
    /// 同目录下"以 `<末分量>-` 开头"的候选文件（最多 3 个）。
    pub dash_candidates: Vec<PathBuf>,
}

/// 在 `root` 下解析 `name`。
pub fn resolve_module(root: &Path, name: &ModuleName) -> Lookup {
    if let Some(found) = exact_lookup(root, name) {
        return Lookup::Found(found);
    }
    Lookup::Missing(MissingModule {
        module: name.as_str(),
        case_mismatch: case_insensitive_lookup(root, name),
        dash_candidates: dash_candidates(root, name),
        expected: root.join(name.relative_path()),
    })
}

/// **逐分量、按字节精确**的查找。
///
/// 刻意不用 `path.is_file()`：macOS/Windows 的文件系统大小写不敏感，
/// `is_file()` 会让 `import logic` 在本地"碰巧"命中 `Logic.sokonanoda`，
/// 到了 Linux CI 才炸——解析结果必须逐平台一致，所以这里对着目录项逐字比较
/// （大小写不一致只作为**提示**，见 [`MissingModule::case_mismatch`]）。
fn exact_lookup(root: &Path, name: &ModuleName) -> Option<PathBuf> {
    let mut current = root.to_path_buf();
    let components = name.components();
    for (index, component) in components.iter().enumerate() {
        let last = index + 1 == components.len();
        let entries = fs::read_dir(&current).ok()?;
        let mut hit: Option<PathBuf> = None;
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let matches = if last {
                file_name.strip_suffix(&format!(".{MODULE_EXTENSION}")) == Some(component.as_str())
                    && entry.path().is_file()
            } else {
                file_name == component.as_str() && entry.path().is_dir()
            };
            if matches {
                hit = Some(entry.path());
                break;
            }
        }
        current = hit?;
    }
    Some(current)
}

/// 逐分量做大小写不敏感匹配；命中且与期望路径不同才返回。
fn case_insensitive_lookup(root: &Path, name: &ModuleName) -> Option<PathBuf> {
    let mut current = root.to_path_buf();
    for (index, component) in name.components().iter().enumerate() {
        let last = index + 1 == name.components().len();
        let entries = fs::read_dir(&current).ok()?;
        let mut hit: Option<PathBuf> = None;
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let matches = if last {
                file_name
                    .strip_suffix(&format!(".{MODULE_EXTENSION}"))
                    .is_some_and(|stem| stem.eq_ignore_ascii_case(component))
            } else {
                file_name.eq_ignore_ascii_case(component)
            };
            if matches {
                hit = Some(entry.path());
                break;
            }
        }
        current = hit?;
    }
    let expected = root.join(name.relative_path());
    (current != expected).then_some(current)
}

/// 同目录下以 `<末分量>-` 开头的模块文件（说明"文件名的横线不是模块名字符"）。
fn dash_candidates(root: &Path, name: &ModuleName) -> Vec<PathBuf> {
    let expected = root.join(name.relative_path());
    let Some(dir) = expected.parent() else {
        return Vec::new();
    };
    let prefix = format!("{}-", name.last_component());
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == MODULE_EXTENSION)
                && path
                    .file_stem()
                    .is_some_and(|stem| stem.to_string_lossy().starts_with(&prefix))
        })
        .collect();
    found.sort();
    found.truncate(3);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "soko-front-resolve-test-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn write(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, "def x : Nat := 1\n").expect("write module");
    }

    #[test]
    fn finds_nested_and_sibling_modules() {
        let root = tmp_dir("found");
        write(&root.join("Bar.sokonanoda"));
        write(&root.join("Lesson/Logic.sokonanoda"));

        let sibling = ModuleName::parse("Bar").expect("valid");
        assert_eq!(
            resolve_module(&root, &sibling),
            Lookup::Found(root.join("Bar.sokonanoda"))
        );
        let nested = ModuleName::parse("Lesson.Logic").expect("valid");
        assert_eq!(
            resolve_module(&root, &nested),
            Lookup::Found(root.join("Lesson/Logic.sokonanoda"))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_module_reports_expected_path_and_case_mismatch() {
        let root = tmp_dir("case");
        write(&root.join("Logic.sokonanoda"));

        let name = ModuleName::parse("logic").expect("valid");
        let Lookup::Missing(missing) = resolve_module(&root, &name) else {
            panic!("`logic` must not resolve: the file on disk is `Logic.sokonanoda`");
        };
        assert_eq!(missing.expected, root.join("logic.sokonanoda"));
        assert_eq!(missing.case_mismatch, Some(root.join("Logic.sokonanoda")));
        assert!(missing.dash_candidates.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_module_suggests_dashed_files_in_the_same_directory() {
        let root = tmp_dir("dash");
        write(&root.join("unit1-propositions-proofs.sokonanoda"));

        let name = ModuleName::parse("unit1").expect("valid");
        let Lookup::Missing(missing) = resolve_module(&root, &name) else {
            panic!("`unit1` must not resolve to a dashed file");
        };
        assert_eq!(
            missing.dash_candidates,
            vec![root.join("unit1-propositions-proofs.sokonanoda")]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn exact_case_hit_does_not_report_a_mismatch() {
        let root = tmp_dir("exact");
        write(&root.join("Logic.sokonanoda"));
        let name = ModuleName::parse("Logic").expect("valid");
        assert!(matches!(resolve_module(&root, &name), Lookup::Found(_)));
        let _ = fs::remove_dir_all(&root);
    }
}
