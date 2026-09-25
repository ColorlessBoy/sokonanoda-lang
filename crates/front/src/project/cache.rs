//! 项目闭包的编译缓存：**一份键，三条命令共用**（`check` / `build` / `query`）。
//!
//! 键 = `ProjectPlan::digest(options)`（拓扑序上每个模块的名字/源/imports + prelude
//! 模式，见 `docs/design/compile-cache.md` §7）。单文件不走这里（各自的文本键）。
//!
//! **为什么在 `front` 而不是 CLI**（T-A01）：LSP 也需要这份缓存
//! （`crates/lsp/src/lib.rs` 的 `Doc::set_text` 对有 `import` 的文档要读它），
//! 而 LSP 依赖不到 CLI crate。搬到 `front` 之后 CLI 与 LSP 共用同一份实现——
//! 这正是 `docs/design/compile-cache.md` §1「LSP 与 CLI 共用同一份」那句话
//! 本来该有的样子（对项目文件，它曾经并不成立）。

use std::path::Path;

use crate::compile::cache::{self, CachedCompile};
use crate::compile::CompileOptions;
use crate::project::{plan_project_with_overlay, ProjectPlan, ProjectReport};

/// 解析项目根 + 加载闭包（IO/parse）并算出**缓存键**。
///
/// `overlay` 是**打开文档的内存文本**（编辑器才有；CLI 传 `&[]`）。
/// 它必须参与摘要：依赖的未落盘编辑会改变这份文档的闭包结果，不折进键里
/// 就会**错命中**——回放出一份按旧依赖算出来的报告。
pub fn plan(
    entry: &Path,
    src: Option<&str>,
    root_override: Option<&Path>,
    overlay: &[(std::path::PathBuf, String)],
    options: &CompileOptions,
) -> (ProjectPlan, String) {
    let plan = plan_project_with_overlay(entry, src, root_override, overlay);
    let digest = plan.digest(options);
    (plan, digest)
}

/// 摘要命中？（`None` = 未命中或缓存不可用——调用方照常编译）
pub fn load(digest: &str, options: &CompileOptions) -> Option<CachedCompile> {
    cache::load(digest, options)
}

/// 写一条项目缓存条目（**整份** `ProjectReport`，T-A03）。
///
/// `report`/`output` 仍然是**入口模块**的（CLI `--json` 与 LSP 的 `report()`
/// 读的就是它，形状与单文件条目一致）；`project` 是整份模块表 + 归因，
/// LSP 的跨文件能力（definition/references/rename/`soko/project`）靠它。
pub fn store(digest: &str, options: &CompileOptions, project: &ProjectReport) {
    let Some(entry) = project.entry_module() else {
        return;
    };
    cache::store(
        digest,
        options,
        &CachedCompile {
            report: entry.report.clone(),
            output: Some(entry.events.clone()),
            project: Some(project.clone()),
        },
    );
}

/// 只缓存**干净**的编译产物（有诊断的结果下次仍要重新算：诊断归因依赖具体文件）。
///
/// **注意（G-24）**：`is_clean()` 会被 `requires` 版本漂移一票否决
/// （`requires_warning`），于是整个项目永不入缓存——`courses/set-theory` 的
/// `requires = "0.61"` 就是这个现场。T-A05 会改这条判据（漂移是可回放的确定性
/// 事实，不该关掉缓存）；在那之前，这个函数对带漂移的课程是**恒不写**的。
pub fn store_if_clean(
    digest: &str,
    options: &CompileOptions,
    project: &ProjectReport,
    is_clean: bool,
) {
    if !is_clean {
        return;
    }
    store(digest, options, project);
}

// ── 模块根下的产物目录（R-3 / T-B5）────────────────────────────────────────
//
// 契约见 `docs/design/project-artifacts.md`。一句话：**项目条目落模块根、
// 单文件条目留全局缓存**——项目条目的键里含入口绝对路径（T-A06），天生按位置
// 隔离，放模块根不串台也不丢跨项目复用；单文件条目的键只含内容，留全局缓存才
// 能跨项目共享。
//
// 三条纪律（都是既有缓存纪律的延续，不是新发明）：
//   * **只省重复劳动，永不推断**：条目仍是内核算过的结果，读不到/损坏/格式不符
//     ⇒ 照常重编；
//   * **best-effort**：目录不可写 ⇒ 退回全局缓存，绝不让请求失败；
//   * **同一个条目格式**：`<key>.json` 与全局缓存逐字节同格式（复用
//     `compile::cache` 的三个目录原语），所以"拷贝即迁移"。

use crate::compile::cache as compiled;
use std::path::PathBuf;

/// 模块根下的产物目录名（用户 R-3 点名的名字）。
pub const ARTIFACTS_DIR: &str = ".sokonanoda";
/// `meta.json` 的 schema。**不符 ⇒ 整个目录当不存在**（宁可重算，绝不误用）。
/// 注意条目本身还带 `CACHE_FORMAT` 校验（第二道保险）。
const ARTIFACTS_FORMAT: u32 = 1;

/// `<模块根>/.sokonanoda`。
pub fn artifacts_dir(root: &Path) -> PathBuf {
    root.join(ARTIFACTS_DIR)
}

/// 产物条目的目录（`<模块根>/.sokonanoda/compiled/`）。
pub fn compiled_at(root: &Path) -> PathBuf {
    artifacts_dir(root).join("compiled")
}

/// 逃生门：`SOKONANODA_NO_PROJECT_ARTIFACTS=1` ⇒ 完全退回今天的行为（只写全局）。
fn enabled() -> bool {
    std::env::var_os("SOKONANODA_NO_PROJECT_ARTIFACTS").is_none()
}

/// `meta.json` 可读且 schema 相符？（缺失也算不符——半成品目录不认）
fn meta_ok(root: &Path) -> bool {
    let Ok(bytes) = std::fs::read(artifacts_dir(root).join("meta.json")) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|meta| meta.get("schema")?.as_str().map(str::to_string))
        .is_some_and(|schema| schema == format!("soko.artifacts/{ARTIFACTS_FORMAT}"))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 建目录 + `.gitignore`（一行 `*`，自忽略）+ `meta.json`。
///
/// `.gitignore` 的内容**恰好一行 `*`**：实测 `git status --porcelain` 完全看不见
/// 这个目录（变体 `*` + `!.gitignore` 反而会漏 `?? .sokonanoda/` ✗）。
/// 不去改用户仓库根的 `.gitignore`（只在自己目录里放一个）。
fn ensure_layout(root: &Path) -> bool {
    let dir = artifacts_dir(root);
    if std::fs::create_dir_all(compiled_at(root)).is_err() {
        return false;
    }
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        let _ = std::fs::write(&ignore, "*\n");
    }
    let meta = dir.join("meta.json");
    if !meta.exists() {
        let payload = serde_json::json!({
            "schema": format!("soko.artifacts/{ARTIFACTS_FORMAT}"),
            "compiler": env!("CARGO_PKG_VERSION"),
            "build_stamp": format!("{:016x}", compiled::build_stamp()),
            "platform": format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
            "created_unix": now_unix(),
            "written_unix": now_unix(),
        });
        if std::fs::write(&meta, format!("{payload}\n")).is_err() {
            return false;
        }
    }
    true
}

/// 读一条**项目**条目：**模块根产物目录 → 全局缓存**（升级平滑：升级前写进全局的
/// 条目仍然命中）。
pub fn load_at(root: &Path, digest: &str, options: &CompileOptions) -> Option<CachedCompile> {
    if enabled() && meta_ok(root) {
        if let Some(entry) = compiled::load_in(&compiled_at(root), &compiled::key(digest, options))
        {
            return Some(entry);
        }
    }
    // **全局兜底那条必须自己校验模块根**（T-B4 取证 B 实测的反例）：项目摘要里
    // 只有**入口绝对路径**、**没有模块根**（`project/mod.rs:156-190`），所以
    // "同一入口 + 两个不同 `--root`"会算出**同一个摘要** ⇒ `build --root rootA`
    // 之后 `build --root rootB` 直接 hit，随后 `query project --root rootB` 报出
    // **rootA** 的根与模块路径（definition/references 会跳到别的根下的文件）✗。
    // 项目目录那条路天然安全（目录就是根），全局这条要按 `ProjectReport::root` 把关。
    // 校验不过 ⇒ 当 miss（宁可重编，绝不回放错位置的报告）。
    let entry = load(digest, options)?;
    if let Some(project) = &entry.project {
        if !same_root(&project.root, root) {
            return None;
        }
    }
    Some(entry)
}

/// 两个模块根是不是同一个地方。
///
/// 快路径是文本比较（今天两侧都来自 `plan.root`，所以本来就相等——`plan.root`
/// 是规范化过的，实测 `--root /var/...` 得到的 root 是 `/private/var/...`）。
/// 慢路径留给"某个调用方传了非 `plan.root` 的写法"：`canonicalize` 之后比
/// （只在文本不等时才付这两次 syscall）。**判不出来就当不同** ⇒ 当 miss 重编
/// （宁可重算，绝不回放错位置的报告）。
fn same_root(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// 写一条**项目**条目到模块根产物目录。
///
/// 逃生门开着、或目录不可写 ⇒ 退回**全局缓存**（与今天逐字节相同的行为），
/// 绝不让调用方失败（既有缓存的 best-effort 纪律）。
pub fn store_at(root: &Path, digest: &str, options: &CompileOptions, project: &ProjectReport) {
    if !enabled() || !ensure_layout(root) {
        store(digest, options, project);
        return;
    }
    let Some(entry) = project.entry_module() else {
        return;
    };
    let key = compiled::key(digest, options);
    compiled::store_in(
        &compiled_at(root),
        &key,
        &CachedCompile {
            report: entry.report.clone(),
            output: Some(entry.events.clone()),
            project: Some(project.clone()),
        },
    );
    // `entry.path` 就是这个入口**文件**的路径（模块根下的某个 `.sokonanoda`）。
    update_index(root, &entry.path, &key);
}

/// [`store_at`] 的"只缓存干净项目"版本（判据与全局那条完全一致）。
pub fn store_if_clean_at(
    root: &Path,
    digest: &str,
    options: &CompileOptions,
    project: &ProjectReport,
    is_clean: bool,
) {
    if !is_clean {
        return;
    }
    store_at(root, digest, options, project);
}

/// 清掉某个模块根的产物**条目**（保留 `.gitignore` 与 `meta.json`）；返回删除条数。
pub fn clean_at(root: &Path) -> usize {
    let removed = compiled::clean_in(&compiled_at(root));
    // 条目都删了 ⇒ 索引里的旧键一并清掉（否则它会一直指向不存在的文件）。
    let path = artifacts_dir(root).join("meta.json");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(mut meta) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(object) = meta.as_object_mut() {
                object.insert("entries".into(), serde_json::json!({}));
                let _ = std::fs::write(&path, format!("{meta}\n"));
            }
        }
    }
    removed
}

/// 写完之后更新**索引**与 `written_unix`（best-effort；`meta.json` 只有几百字节）。
///
/// 索引 = `入口路径 → 条目的磁盘键`，它承担两件事：
/// ① **替换**：同一个入口的新结果把**它的**旧条目删掉（而不是让别的入口被淘汰）；
/// ② **有界**：一份产物对应一个入口文件 ⇒ 目录大小天然有界（= 入口文件数）。
///
/// 为什么不用"条数上限 + mtime 淘汰"（早先的做法，**实测踩到** ✗）：课程门禁反复判
/// `courses/set-theory` 的 ~35 个文件，而上限取 32 ⇒ 每一轮都在**互相淘汰刚写下的
/// 条目**，命中率崩掉、反复重编（CI 的 `test` job 从 ~15 分钟变成 50+ 分钟）。
/// 判据：`crates/cli/tests/artifacts.rs::every_entry_keeps_its_own_artifact_round_after_round`
/// （34 个入口 ⇒ 34 条产物、第二轮全命中；条数上限版本实测 **32 ≠ 34** ✗）。
fn update_index(root: &Path, entry_path: &Path, key: &str) {
    let path = artifacts_dir(root).join("meta.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let Ok(mut meta) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return;
    };
    let Some(object) = meta.as_object_mut() else {
        return;
    };
    let index = object
        .entry("entries")
        .or_insert_with(|| serde_json::json!({}));
    let Some(entries) = index.as_object_mut() else {
        return;
    };
    let entry = entry_path.display().to_string();
    if let Some(previous) = entries.get(&entry).and_then(|value| value.as_str()) {
        if previous != key {
            let _ = std::fs::remove_file(compiled_at(root).join(format!("{previous}.json")));
        }
    }
    entries.insert(entry, serde_json::json!(key));
    object.insert("written_unix".into(), serde_json::json!(now_unix()));
    let _ = std::fs::write(&path, format!("{meta}\n"));
}
