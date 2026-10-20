// 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
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

use crate::compile::Dialect;

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
    /// **方言**（S4，设计 `docs/design/dialect-config.md` §2 第 5 档）：
    /// `[dialect] strict_implicit = true|false` ⇒ 严格档 / 旧读法。
    ///
    /// `None` = 清单**不表态**（交给上面的档位，最后落到默认 `loose`）。注意
    /// `false` 是**显式的旧读法**、不是"没写" —— 它会盖住默认值（虽然今天两者
    /// 同值，但优先级链里它是第 5 档的"我表过态了" ✓）。
    pub dialect: Option<Dialect>,
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
    dialect: Option<ManifestDialect>,
}

/// `[dialect]` 表（S4，设计 `docs/design/dialect-config.md` §2）：键名照设计，
/// 将来的同类开关进同一个表，避免顶层键无限增殖 ✓。
///
/// ⚠ `deny_unknown_fields` 是**故意的**：配置面最坏的失败是"配了却不生效"——
/// 设计 §4 写死了那句「先让用户能配、配了却不生效，比不能配更坏 ✗」。`strict_implcit`
/// 这种拼写错误若被静默忽略，用户会以为严格档开了 ⇒ 这里让它当场变成一条
/// `manifest-invalid`（serde 的报错会点名 unknown field + 正确键名 ✓）。
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDialect {
    /// 严格隐式参数读法：`true` = 严格档，`false` = 旧读法。
    strict_implicit: Option<bool>,
}

/// 从 `start`（一个**目录**）向上找最近的 `sokonanoda.toml`。
///
/// 停止条件（任一命中）：找到清单 / 目录含 `.git` / 到达用户家目录 /
/// 到达文件系统根。返回清单文件的绝对（或调用方传入的相对）路径。
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut current = Some(start);
    while let Some(dir) = current {
        // G-12 护栏：空目录分量既不是起点也不是"上溯的一站"。
        // `Path::new("").join("sokonanoda.toml")` = `"sokonanoda.toml"`，语义上等于
        // "去问 CWD 要清单"——那是本层不该有的隐状态（入口绝对化之后这里到不了，
        // 护栏防的是别处再传空路径进来）。
        if dir.as_os_str().is_empty() {
            return None;
        }
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
            "清单只需要可选的 `name` / `requires` / `src` 与 `[dialect]` 表，例如：\n  name = \"my-proofs\"\n  src = \"src\"\n  [dialect]\n  strict_implicit = true",
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
    let dialect = parsed
        .dialect
        .and_then(|table| table.strict_implicit)
        .map(|strict| {
            if strict {
                Dialect::Strict
            } else {
                Dialect::Loose
            }
        });
    Ok(Manifest {
        name: parsed.name,
        requires: parsed.requires,
        src: parsed.src,
        dialect,
    })
}

/// **清单里的方言**（S4，设计 `docs/design/dialect-config.md` §2 第 5 档）：
/// `[dialect] strict_implicit` 的**表态**；没清单 / 没写该键 ⇒ `None`（不表态）。
///
/// **发现规则与 `plan_project` 完全一致**（`root_override` 给了就跳过清单：
/// `--root` / `--no-project` 的语义在两条路上必须是同一个 ✓）。
///
/// 清单坏掉 ⇒ `None`：**报错仍由 `plan_project` 的 `manifest-invalid` 负责**
/// ——这里只读一个值，绝不因为配置面让一个本来能编译的文件判坏（诊断不重复、
/// 退出码不因读配置而变 ✓）。
pub fn dialect_at(entry_path: &Path, root_override: Option<&Path>) -> Option<Dialect> {
    if root_override.is_some() {
        return None;
    }
    // **与 `plan_project` 同一套绝对化**（G-12）：相对入口 `units/u.sokonanoda`
    // 若从裸路径向上找，`find_manifest` 会在空目录分量处停下（G-12 的护栏）⇒
    // **CWD 里的清单会被漏掉**，而真正编译时又找得到 ⇒ 同一个文件两条答案 ✗。
    let entry_path = super::absolute_lexical(entry_path);
    let dir = entry_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let path = find_manifest(dir)?;
    load(&path).ok()?.dialect
}

/// 清单目录 + `src` = 模块根。
///
/// 清单路径没有目录分量（`Path::new("sokonanoda.toml").parent()` = `Some("")`）时
/// 目录当 `.`——模块根**永不为空**（G-12：空 root 会让 `resolve_module` 的
/// `read_dir("")` ENOENT、让 `query project` 把 root 渲染成 `''`）。
pub fn module_root(manifest_path: &Path, manifest: &Manifest) -> PathBuf {
    let dir = match manifest_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
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

    /// G-12 病根：`Path::new("").join("sokonanoda.toml")` = `"sokonanoda.toml"`，
    /// 语义上等于"去问 CWD 要清单"。空目录分量既不能当上溯的起点，也不能当一站。
    ///
    /// 这里直接钉住 `find_manifest` 的返回：**空路径必须在看 CWD 之前就返回 `None`**
    /// （CWD 里恰好有清单时，缺护栏的实现在这里就会红）。形状①的端到端现场
    /// （相对入口 + 祖先清单）由 `crates/cli/tests/imports.rs` 的用例钉死——单元
    /// 测试不能安全地改进程 CWD。
    #[test]
    fn an_empty_directory_component_is_not_an_ancestor_step() {
        let cwd_manifest = std::env::current_dir().expect("cwd").join(MANIFEST_FILE);
        assert_eq!(
            find_manifest(Path::new("")),
            None,
            "空路径不许命中 CWD 里的 {cwd_manifest:?}"
        );
        // 裸文件名与单层相对目录的 `parent()` 就是空分量：同样到此为止，不跨过去。
        assert_eq!(
            find_manifest(Path::new("units").parent().expect("parent")),
            None
        );
        assert_eq!(
            find_manifest(Path::new("u.sokonanoda").parent().expect("parent")),
            None
        );
    }

    /// 清单路径没有目录分量时（`module_root("sokonanoda.toml")`）模块根必须是 `.`，
    /// **永不**是空 `PathBuf`：空 root 会让 `resolve_module` 的 `read_dir("")` ENOENT
    /// （G-12），也让 `query project` 的 `canonicalize("")` 失败、把 root 渲染成 `''`。
    #[test]
    fn a_manifest_without_a_directory_component_roots_at_dot() {
        assert_eq!(
            module_root(Path::new(MANIFEST_FILE), &Manifest::default()),
            Path::new(".")
        );
        // `src` 也照样挂在 `.` 上（相对路径保持相对，绝对化是 `plan_project` 的事）。
        let with_src = Manifest {
            src: Some("src".to_string()),
            ..Manifest::default()
        };
        assert_eq!(
            module_root(Path::new(MANIFEST_FILE), &with_src),
            Path::new(".").join("src")
        );
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

    /// **S4 清单档**（设计 `docs/design/dialect-config.md` §2 第 5 档）：`[dialect]
    /// strict_implicit` 的三态 —— `true` / `false` / **没写（不表态）** 必须可区分，
    /// 前者进优先级链、后者交给下一档。
    #[test]
    fn the_dialect_table_is_optional_present_and_explicitly_loose() {
        let dir = tmp_dir("dialect");
        let path = dir.join(MANIFEST_FILE);

        std::fs::write(&path, "[dialect]\nstrict_implicit = true\n").expect("write");
        assert_eq!(load(&path).expect("valid").dialect, Some(Dialect::Strict));

        std::fs::write(&path, "[dialect]\nstrict_implicit = false\n").expect("write");
        assert_eq!(
            load(&path).expect("valid").dialect,
            Some(Dialect::Loose),
            "`false` 是**显式旧读法**，不是 `None`（第 5 档表过态了）"
        );

        // 表在、键不在 ⇒ 不表态（与"整个表没有"同义）。
        std::fs::write(&path, "[dialect]\n").expect("write");
        assert_eq!(load(&path).expect("valid").dialect, None);
        std::fs::write(&path, "name = \"p\"\n").expect("write");
        assert_eq!(load(&path).expect("valid").dialect, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **配置面不许"配了却不生效"**（设计 §4 的原话）：拼错的键必须当场报错、
    /// 并且报错里要看得见**正确键名**（serde 的 unknown-field 报错带候选 ✓）。
    #[test]
    fn a_typo_in_the_dialect_table_is_an_error_not_a_silent_default() {
        let dir = tmp_dir("dialect-typo");
        let path = dir.join(MANIFEST_FILE);
        std::fs::write(&path, "[dialect]\nstrict_implcit = true\n").expect("write");
        let err = load(&path).expect_err("a typo must not be ignored");
        assert!(
            err.message.contains("合法 TOML") && err.message.contains("strict_implicit"),
            "报错要点名正确的键：{}",
            err.message
        );
        assert!(
            err.hint.contains("strict_implicit"),
            "出路要给一份能照抄的清单：{}",
            err.hint
        );

        // 类型错（`strict_implicit = "yes"`）同样走"人话 + 出路"，不是 panic。
        std::fs::write(&path, "[dialect]\nstrict_implicit = \"yes\"\n").expect("write");
        let err = load(&path).expect_err("a wrong type must be rejected");
        assert!(err.message.contains("合法 TOML"), "{}", err.message);
        assert!(
            err.message.contains("boolean") || err.message.contains("bool"),
            "类型错要说清期望什么：{}",
            err.message
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **发现规则与 `plan_project` 同源**：最近的清单、`--root`/`--no-project`
    /// （= `root_override` 给了值）一律跳过清单；坏清单不在这里报错（那是
    /// `plan_project` 的 `manifest-invalid`）。
    #[test]
    fn dialect_at_uses_the_same_manifest_discovery_as_the_planner() {
        let dir = tmp_dir("dialect-at");
        std::fs::write(
            dir.join(MANIFEST_FILE),
            "[dialect]\nstrict_implicit = true\n",
        )
        .expect("write manifest");
        let entry = dir.join("Main.sokonanoda");
        std::fs::write(&entry, "def one : Nat := 1\n").expect("write entry");

        assert_eq!(dialect_at(&entry, None), Some(Dialect::Strict));
        assert_eq!(
            dialect_at(&entry, Some(&dir)),
            None,
            "`--root` / `--no-project` 跳过清单（否则两条路的根语义会打架）"
        );

        // 坏清单 ⇒ 不表态（诊断归 `plan_project`）。
        std::fs::write(dir.join(MANIFEST_FILE), "src = \"../escape\"\n").expect("write bad");
        assert_eq!(dialect_at(&entry, None), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requires_compares_major_minor_only() {
        // 不要写死版本：写死的 "0.57" 在 0.57.0 发版当天自己变成"匹配"。
        // 这里从运行版本推出一个必然不同的 minor。
        let running = env!("CARGO_PKG_VERSION");
        let mut parts = running.split('.');
        let major: u64 = parts.next().expect("major").parse().expect("numeric major");
        let minor: u64 = parts.next().expect("minor").parse().expect("numeric minor");
        let other = format!("{major}.{}", minor + 1);

        let manifest = Manifest {
            requires: Some(other.clone()),
            ..Manifest::default()
        };
        let warning = version_warning(&manifest)
            .unwrap_or_else(|| panic!("requires {other} must warn against {running}"));
        assert!(warning.contains(&other), "{warning}");

        // 只有 major.minor 参与比较：补一个补丁号仍然算匹配。
        let same_minor = Manifest {
            requires: Some(format!("{major}.{minor}")),
            ..Manifest::default()
        };
        let same = Manifest {
            requires: Some(running.to_string()),
            ..Manifest::default()
        };
        assert_eq!(version_warning(&same_minor), None);
        assert_eq!(version_warning(&same), None);
        assert_eq!(version_warning(&Manifest::default()), None);
    }
}
