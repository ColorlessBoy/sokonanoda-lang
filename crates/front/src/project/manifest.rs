//! 项目清单 `sokonanoda.toml`：**存在即项目根**，元数据全部可选。
//!
//! 设计：`docs/design/imports-and-projects.md` §4.4（Q1 采用 TOML）。
//!
//! * 发现规则：从入口文件所在目录**向上**找最近的清单；
//! * **硬边界**：走到含 `.git` 的目录就停（绝不到文件系统根、更不到 `$HOME`）——
//!   调研里 clangd/rust-analyzer/pyright/gopls/Agda/lean4 **都一路走到 `/`**，
//!   `$HOME` 里的流浪 marker 会捕获无关文件（§2.4 的"共同坑"）；
//! * 没有清单时的零配置退路在 `compile_project` 里：模块根 = 入口文件目录。

use std::path::{Path, PathBuf};

/// 清单文件名。
pub const MANIFEST_FILE: &str = "sokonanoda.toml";

/// 已加载的清单（字段全部可选：空文件也是合法的项目根标记）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    /// 项目名（元数据，不参与解析）。
    pub name: Option<String>,
    /// 工具链钉版本（类似 `lean-toolchain` 的**意图**）：`"0.57"` 表示
    /// major.minor 必须匹配；不匹配只给 warning（Q6：v1 不阻断）。
    pub requires: Option<String>,
    /// 模块根（相对清单目录），默认 `"."`。
    pub src: Option<String>,
}

/// 清单加载失败（都带教学文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    pub path: PathBuf,
    pub message: String,
    pub hint: String,
}

impl ManifestError {
    fn new(path: &Path, message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            path: path.to_path_buf(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

/// `[derive(Deserialize)]` 的镜像：只认我们声明的键，类型错由 serde 报。
#[derive(serde::Deserialize)]
struct ManifestFile {
    name: Option<String>,
    requires: Option<String>,
    src: Option<String>,
}

/// 从 `start`（一个**目录**）向上找最近的 `sokonanoda.toml`。
///
/// 停止条件（任一命中）：找到清单 / 目录含 `.git` / 到达用户家目录 /
/// 到达文件系统根。返回清单文件的绝对（或调用方传入的相对）路径。
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut current = Some(start);
    while let Some(dir) = current {
        let candidate = dir.join(MANIFEST_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        // 硬边界：`.git` 是这个仓库/工作区的顶，外面不再属于本项目。
        if dir.join(".git").exists() {
            return None;
        }
        if home.as_deref().is_some_and(|home| home == dir) {
            return None;
        }
        current = dir.parent();
    }
    None
}

/// 读取并校验清单。
pub fn load(path: &Path) -> Result<Manifest, ManifestError> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        ManifestError::new(
            path,
            format!("读不到项目清单：{err}"),
            "确认 `sokonanoda.toml` 可读；不想要项目就把这个文件删掉（单文件模式仍然可用）。",
        )
    })?;
    let parsed: ManifestFile = toml::from_str(&text).map_err(|err| {
        ManifestError::new(
            path,
            format!("项目清单不是合法 TOML：{err}"),
            "清单只需要可选的 `name` / `requires` / `src` 三个键，例如：\n  name = \"my-proofs\"\n  src = \"src\"",
        )
    })?;
    if let Some(src) = &parsed.src {
        if src.is_empty() || Path::new(src).is_absolute() || src.split('/').any(|part| part == "..")
        {
            return Err(ManifestError::new(
                path,
                format!("`src = \"{src}\"` 不是合法的模块根"),
                "`src` 必须是清单目录下的相对路径（例如 `src`），不能用绝对路径或 `..`。",
            ));
        }
    }
    Ok(Manifest {
        name: parsed.name,
        requires: parsed.requires,
        src: parsed.src,
    })
}

/// 清单目录 + `src` = 模块根。
pub fn module_root(manifest_path: &Path, manifest: &Manifest) -> PathBuf {
    let dir = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    match &manifest.src {
        Some(src) => dir.join(src),
        None => dir,
    }
}

/// `requires` 与当前二进制的 major.minor 不一致时的提示（v1 只警告）。
pub fn version_warning(manifest: &Manifest) -> Option<String> {
    let requires = manifest.requires.as_deref()?;
    let running = env!("CARGO_PKG_VERSION");
    if major_minor(requires) == major_minor(running) {
        return None;
    }
    Some(format!(
        "项目清单要求 sokonanoda {requires}，当前是 {running}（版本不一致可能带来行为差异）。"
    ))
}

fn major_minor(version: &str) -> Option<(u64, u64)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "soko-front-manifest-test-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn missing_manifest_is_not_an_error() {
        let dir = tmp_dir("none");
        assert_eq!(find_manifest(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_the_nearest_ancestor_manifest_and_stops_at_git() {
        let root = tmp_dir("ancestor");
        std::fs::write(root.join(MANIFEST_FILE), "name = \"outer\"\n").expect("write manifest");
        let nested = root.join("a/b");
        std::fs::create_dir_all(&nested).expect("create nested dirs");
        assert_eq!(find_manifest(&nested), Some(root.join(MANIFEST_FILE)));

        // `.git` 是硬边界：它下面的清单不会被外面的项目认领。
        let repo = root.join("other");
        std::fs::create_dir_all(repo.join(".git")).expect("create .git");
        let deep = repo.join("x");
        std::fs::create_dir_all(&deep).expect("create deep dir");
        assert_eq!(find_manifest(&deep), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_manifest_is_a_valid_project_root() {
        let dir = tmp_dir("empty");
        let path = dir.join(MANIFEST_FILE);
        std::fs::write(&path, "").expect("write empty manifest");
        let manifest = load(&path).expect("empty manifest is legal");
        assert_eq!(manifest, Manifest::default());
        assert_eq!(module_root(&path, &manifest), dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn src_moves_the_module_root_and_must_stay_inside() {
        let dir = tmp_dir("src");
        let path = dir.join(MANIFEST_FILE);
        std::fs::write(&path, "name = \"p\"\nsrc = \"src\"\n").expect("write manifest");
        let manifest = load(&path).expect("valid");
        assert_eq!(manifest.name.as_deref(), Some("p"));
        assert_eq!(module_root(&path, &manifest), dir.join("src"));

        std::fs::write(&path, "src = \"../escape\"\n").expect("write bad src");
        let err = load(&path).expect_err(".. must be rejected");
        assert!(err.message.contains("不是合法的模块根"), "{}", err.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_toml_is_reported_with_a_hint() {
        let dir = tmp_dir("bad");
        let path = dir.join(MANIFEST_FILE);
        std::fs::write(&path, "name = [unterminated\n").expect("write");
        let err = load(&path).expect_err("bad toml");
        assert!(err.message.contains("合法 TOML"), "{}", err.message);
        assert!(err.hint.contains("`src`"), "{}", err.hint);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requires_compares_major_minor_only() {
        let manifest = Manifest {
            requires: Some("0.57".into()),
            ..Manifest::default()
        };
        let warning = version_warning(&manifest).expect("0.57 != 0.56 at this point in history");
        assert!(warning.contains("0.57"), "{warning}");

        let running = env!("CARGO_PKG_VERSION");
        let same = Manifest {
            requires: Some(running.to_string()),
            ..Manifest::default()
        };
        assert_eq!(version_warning(&same), None);
        assert_eq!(version_warning(&Manifest::default()), None);
    }
}
