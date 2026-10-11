//! **参考文档落点的唯一一份机械**（五档解析）—— tactic 与命令**共用**。
//!
//! 设计：`docs/design/tactic-docs.md` §4.9 D10（tactic 侧，原文）+ 设计
//! `docs/design/command-docs.md`（命令侧）。**为什么抽出来**：命令页与 tactic 页的
//! 落点规则**逐条相同**（仓库真源 → 插件目录 → 缓存物化 → 临时物化 → `None`），
//! 各写一份必然在"加一档 / 改子目录名 / 改物化策略"时**静默分叉** ✗ ⇒ 只留一份，
//! [`crate::tactics::doc_path`] 与 [`crate::commands::doc_path`] 都转发到这里 ✓。
//!
//! ⚠ **① 是编译期路径**（`env!("CARGO_MANIFEST_DIR")` 烘的是**编译机**的路径）⇒
//! 只在开发树 / CI 上存在；发布产物（VSIX / 缓存二进制）在用户机器上 `is_file()`
//! 为假 ⇒ 天然落到 ②（插件目录）或 ③（物化）✓ —— 与 `prelude_source_path`
//! （`compile/prelude.rs`）同一门纪律。
//!
//! ③④ 与 prelude 同款：**幂等**（读出来与编译期内嵌的 `markdown` 一致就不重写，
//! 免得每次 F12 动一次 mtime 让编辑器反复重载）、写不进就试下一档、两档都写不进
//! ⇒ `None`（**绝不编造位置** ✗）。
//!
//! ⚠ 两个**具名**插件目录键（不是一条通用前缀）：tactic 与命令各自一个
//! （`SOKONANODA_DOCS_DIR` / `SOKONANODA_COMMAND_DOCS_DIR`），扩展侧只写**新增的那一个**
//! 键、不 `...process.env`（与 `SOKONANODA_CACHE_DIR` 同一条纪律）。键名 ↔ 目录名的
//! 对账判据在 `crates/cli/tests/extension.rs`（接缝守卫：LSP 读的键必须与扩展写的键同名）。

/// 五档解析一份**随插件分发**的参考文档的落点。
///
/// * `subdir` —— `reference/` 下的子目录名（`tactics` / `commands`）；
/// * `file_stem` —— 文件名（不含 `.md`）；
/// * `markdown` —— 编译期内嵌的正文（③④ 档用它判"是不是同一份"，也用它物化）；
/// * `plugin_env` —— ② 档读的环境变量名（tactic 与命令各一个键）。
///
/// 每一档都过 `is_file()`：目录在而文件缺 ⇒ 继续往下走（不做死指针 ✗）。
pub(crate) fn doc_path(
    subdir: &str,
    file_stem: &str,
    markdown: &str,
    plugin_env: &str,
) -> Option<std::path::PathBuf> {
    let file_name = format!("{file_stem}.md");
    // ① 仓库真源（开发树首选：改文档立即生效，不必重打包）。
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("reference")
        .join(subdir)
        .join(&file_name);
    if repo.is_file() {
        return Some(repo);
    }
    // ② 插件目录（安装形态：扩展把 `plugin_env` 指到 `<extension>/docs/<subdir>`）。
    if let Some(dir) = std::env::var_os(plugin_env) {
        let plugin = std::path::PathBuf::from(dir).join(&file_name);
        if plugin.is_file() {
            return Some(plugin);
        }
    }
    // ③ 缓存物化 → ④ 临时物化（照 `prelude_source_path` 的三段兜底，少一段仓库）。
    let candidates = crate::compile::cache::root()
        .map(|root| root.join("reference").join(subdir).join(&file_name))
        .into_iter()
        .chain(std::iter::once(
            std::env::temp_dir()
                .join("sokonanoda-reference")
                .join(subdir)
                .join(&file_name),
        ));
    for path in candidates {
        if matches!(std::fs::read_to_string(&path), Ok(old) if old == markdown) {
            return Some(path);
        }
        let written = path
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .ok()
            .flatten()
            .and_then(|()| std::fs::write(&path, markdown).ok());
        if written.is_some() {
            return Some(path);
        }
    }
    // ⑤ 五档都不成立 ⇒ 没有落点（编辑器答"没有定义"，比给死指针好 ✓）。
    None
}
