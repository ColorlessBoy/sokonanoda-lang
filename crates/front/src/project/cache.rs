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
/// **判据只有一处**（审计 #1，2026-09-25 ✓）：`clean` **不再是形参** ✗ ——
/// 那个 bool 正是分叉的接缝 ✓：调用者各算各的，`crates/cli/src/query.rs` 就漏算了
/// 依赖模块的错误 ⇒ **`query check` 会为不干净的项目写缓存** ✗ ⇒ 之后所有
/// `grade`/`check` **静默丢掉依赖模块的警告** ✓（实测：冷跑 1 条 → 跑一次
/// `query check` → 再跑 **0 条** ✗）。现在由函数自己对 `&ProjectReport` 现取
/// `project.is_clean()` ✓ ⇒ 谁调用都不会算错 ✓。
///
/// **注意（G-24 的历史，已由 T-A05 解决 ✓）**：旧的过期注释说 `is_clean()` 会被
/// `requires` 漂移一票否决 ⇒ 课程永不入缓存 ✗ —— **已不成立** ✓：
/// `report.rs::is_clean` 明确写着 `requires_warning` **不算**不干净 ✓。
pub fn store_if_clean(digest: &str, options: &CompileOptions, project: &ProjectReport) {
    if !project.is_clean() {
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
/// 产物目录的**条目结构**版本。`2`（2026-10-02 / 值守第 8 单）：`schema` 里带上
/// `report::REPORT_SHAPE` —— 报告形状一变，整个产物目录**不认** ✓（以前 digest 只由
/// **源码**算 ⇒ 源码没变而二进制变了时，旧产物被命中、新字段走 `#[serde(default)]`
/// ⇒ 静默回放旧报告 ✗）。
const ARTIFACTS_FORMAT: u32 = 2;

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

/// 当前 `meta.json` 的 schema 串（**单一来源** ✓）：`soko.artifacts/<条目格式>.r<报告形状>`。
/// 三个消费者都用它：`meta_ok`（决定产物目录认不认）、`query::project`（决定要不要
/// 把里面的 `compiler` 版本显示给用户 —— schema 不符 ⇒ **不显示** ✗）、
/// 以及 [`update_index`]（**写侧把它升级成当前值** ✓，见那里的长注释）。
///
/// ⚠ **"免得报旧二进制版本号"这条顾虑已由写侧解决**（2026-10-09 需求 1）：`update_index`
/// 与 `compiler`/`build_stamp` **同一次**把 schema 刷成当前值 ⇒ 读侧这道过滤从此只兜底
/// "这份 `meta.json` 不是本进程写的、也没被本进程刷新过"（例如只读目录、逃生门、
/// 或旧二进制写的目录还没被覆盖过）。
pub fn meta_schema() -> String {
    format!(
        "soko.artifacts/{ARTIFACTS_FORMAT}.r{}",
        crate::compile::REPORT_SHAPE
    )
}

/// `meta.json` 可读且 schema 相符？（缺失也算不符——半成品目录不认）
///
/// 比对走 [`meta_schema`]（**单一来源** ✓ —— 这里以前把同一个 `format!` 又写了一遍，
/// 2026-10-09 收敛掉：写侧现在也要用这个串，两处各写一份迟早漂移）。
fn meta_ok(root: &Path) -> bool {
    let Ok(bytes) = std::fs::read(artifacts_dir(root).join("meta.json")) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|meta| meta.get("schema")?.as_str().map(str::to_string))
        .is_some_and(|schema| schema == meta_schema())
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 当前编译器的**版本戳** `(compiler, build_stamp)`。
///
/// ⚠ 它必须**跟着"最后写产物的那次编译"走**，不能只在建目录时写一次 ✗：
/// 2026-09-29 用户实测 —— 升级到 0.78.1 后 Infoview 顶上写 `server 0.78.1`、
/// 项目区块却写 `编译器 0.78.0`（`meta.json` 是 0.78.0 那次建的）⇒ 两个版本号
/// **无法调和**。
///
/// 刷新它**不影响缓存正确性**：作废靠的是**键**里的版本 + stamp
/// （`compile::cache::key_parts`），`meta.json` 这两个字段按设计
/// 「只作诊断与提示」（`docs/design/project-artifacts.md` §3.2）✓。
fn current_stamp() -> (&'static str, String) {
    (
        env!("CARGO_PKG_VERSION"),
        format!("{:016x}", compiled::build_stamp()),
    )
}

/// **条目文件名的写者标记**：`<compiler>+<stamp16>+<key>`（不含 `.json`）。
///
/// ## 为什么把写者写进**文件名**（2026-10-10 用户实测的第二个反馈）
///
/// 用户现场：「`产物：248 条 · 1.2 GiB · 由编译器 0.87.3 写入` 这个版本号是假的
/// —— 我用 0.87.2 编完，装上 0.87.3 还没 rebuild，面板就说 0.87.3 了」。
/// 根因是**目录级**的 `meta.json.compiler` 会被**任何一次写入**刷成"当前编译器"
/// （[`update_index`]）⇒ 一条新条目就能把另外 247 条的历史改写掉，这个版本号
/// 从此失去意义 ✗。
///
/// ⇒ 写者只认**条目自己**：写的时候把 `(compiler, build_stamp)` 编进文件名，
/// 读侧一次 `read_dir` 就能如实统计"这些产物分别是哪个编译器写的"（零额外 IO、
/// 不解析条目内容——条目是整份报告，MB 量级），而且**不会被后来的写入改写** ✓。
/// `ls` 一眼也能看出写者（人/agent 都读得到）。
///
/// **分隔符选 `+`**：版本号按仓规只能是纯 `x.y.z`（带后缀会被 `scripts/soko`
/// 按 G-16 拒绝运行）⇒ 版本里不可能出现 `+`，于是"从左边切两刀"永远无歧义 ✓
/// （用 `.` 会与版本号里的点撞车 ✗）。
///
/// **旧命名 `<key>.json` 怎么办**：不认（当 miss、重编 ⇒ 与升级后的键本来就
/// 命中不了同一件事），读侧把它算成"写者未记录"⇒ 面板如实说"未记录 · 建议
/// Rebuild"，**不再**拿 `meta.json` 那个会被刷新的字段冒充 ✓。
pub fn entry_stem(key: &str) -> String {
    let (compiler, stamp) = current_stamp();
    format!("{compiler}+{stamp}+{key}")
}

/// 从条目文件名（含或不含 `.json`）解出写者 `(compiler, build_stamp)`。
///
/// `None` = 旧命名（`<key>.json`）或形状不认识 ⇒ **写者未记录**（读侧如实说，
/// 不猜）。判据见 `query::project::tests`。
pub fn writer_of_entry_file(name: &str) -> Option<(&str, &str)> {
    let stem = name.strip_suffix(".json").unwrap_or(name);
    let mut parts = stem.split('+');
    let compiler = parts.next()?;
    let stamp = parts.next()?;
    let key = parts.next()?;
    // 三段都要在，且 key 段必须恰好一段（多于三段 = 不是我们的命名）。
    if compiler.is_empty() || stamp.is_empty() || key.is_empty() || parts.next().is_some() {
        return None;
    }
    Some((compiler, stamp))
}

/// 当前编译器身份（`current_stamp()` 的公开只读视图，读侧比"产物是不是这份
/// 编译器写的"要用它）。
pub fn current_writer() -> (&'static str, String) {
    current_stamp()
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
        let (compiler, build_stamp) = current_stamp();
        let payload = serde_json::json!({
            "schema": meta_schema(),
            "compiler": compiler,
            "build_stamp": build_stamp,
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
        let key = compiled::key(digest, options);
        if let Some(entry) = compiled::load_in(&compiled_at(root), &entry_stem(&key)) {
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
    let stem = entry_stem(&key);
    compiled::store_in(
        &compiled_at(root),
        &stem,
        &CachedCompile {
            report: entry.report.clone(),
            output: Some(entry.events.clone()),
            project: Some(project.clone()),
        },
    );
    // `entry.path` 就是这个入口**文件**的路径（模块根下的某个 `.sokonanoda`）。
    // 索引里存**完整文件名**（不是裸 key）：它同时是"替换时要删掉的那一份"的
    // 句柄，也是读侧统计写者的依据 ✓。
    update_index(root, &entry.path, &stem);
}

/// [`store_at`] 的"只缓存干净项目"版本（判据与全局那条完全一致）。
pub fn store_if_clean_at(
    root: &Path,
    digest: &str,
    options: &CompileOptions,
    project: &ProjectReport,
) {
    // 判据与全局那条**同一处**（审计 #1 ✓）：不再收 `is_clean` 形参 ✗。
    if !project.is_clean() {
        return;
    }
    store_at(root, digest, options, project);
}

/// 清掉某个模块根的产物**条目**（保留 `.gitignore` 与 `meta.json`）**与模块产物**
/// （`.sokonanoda/artifacts/` 里的全部内容，保留它的 `.gitignore`）；返回删除条数。
pub fn clean_at(root: &Path) -> usize {
    // **T1-B 批 3**：模块产物（`.sokonanoda/artifacts/`）**一起清** —— 不清它，
    // `rebuild` 会命中旧产物 ⇒ 与 R-3 同形的"清空了却什么都没重编"假动作会重演 ✗。
    let removed =
        compiled::clean_in(&compiled_at(root)) + crate::project::artifacts::clean_in(root);
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

/// 写完之后更新**索引**、`written_unix`、版本戳与 schema（best-effort；`meta.json`
/// 只有几百字节）。
///
/// **读侧的 schema 过滤没变**（[`meta_ok`] / `artifacts_of` 仍要求 `== meta_schema()`）；
/// 变的是**写侧**：写产物时把 schema 一并升到当前值 ⇒ 老目录不会永远卡在
/// "整目录当不存在"（2026-10-09 需求 1 的根因与理由见下面那段注释 ✓）。
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
pub(crate) fn update_index(root: &Path, entry_path: &Path, key: &str) {
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
    // **版本戳跟着"最后写产物的那次编译"走** ✓（见 [`current_stamp`]）——
    // 不刷新的话，升级后 `meta.json` 里永远留着建目录那天的版本号，
    // 而它会被 Infoview 画成「编译器 X」⇒ 与顶上那行 `服务器 Y` 打架 ✗。
    let (compiler, build_stamp) = current_stamp();
    object.insert("compiler".into(), serde_json::json!(compiler));
    object.insert("build_stamp".into(), serde_json::json!(build_stamp));
    // **schema 也必须升级**（2026-10-09 需求 1，用户实测）。
    //
    // 症状：面板顶上「编译器 ?」——根因是 `query::project::artifacts_of` 与
    // [`meta_ok`] 都按 `schema == meta_schema()` 过滤，而老目录的 `schema` 停在旧格式
    // （本地实测 `course/shared/.sokonanoda/meta.json` = `soko.artifacts/1`，而当前是
    // `soko.artifacts/2.r3`）⇒ 整个产物目录**当不存在**：`compiler` 一律 `None` ⇒
    // Infoview 画成「由编译器 ? 写入」✗；顺带每次编译都白读不到自己的产物。
    //
    // 为什么升级它**不是**"把陈旧数据画上屏"（[`meta_schema`] 那条老顾虑已不成立 ✓）：
    // 走到这一行时，`compiled/` 里刚写进一条**当前编译器**产出的新条目，`compiler` /
    // `build_stamp` 也在同一次刷新成当前值 ⇒ schema 描述的正是这份目录**现在的**写者。
    // 陈旧条目不会因此被误用：条目的磁盘键里带版本 + `build_stamp`（`key_parts`），
    // 每条还带 `CACHE_FORMAT` 校验（第二道保险）。
    //
    // 已经是当前值就一个字节都不动（幂等；也保住"文件没变化就不重写"的既有语义 ✓）。
    let schema = meta_schema();
    if object.get("schema").and_then(|value| value.as_str()) != Some(schema.as_str()) {
        object.insert("schema".into(), serde_json::json!(schema));
    }
    let _ = std::fs::write(&path, format!("{meta}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("soko-project-cache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// **③ 的真相层判据**（2026-09-29 用户实测「同一 Infoview 版本号不一致」）。
    ///
    /// `meta.json` 的版本戳必须**跟着写产物的编译器走**。修前它只在**建目录**时写
    /// 一次 ⇒ 用户升级到 0.78.1 之后，目录里永远留着 `0.78.0`，而 Infoview 会把它
    /// 画成「编译器 0.78.0」⇒ 与顶上 `服务器 0.78.1` **打架** ✗。
    ///
    /// 这条判据**咬得住那个 bug**：把 `update_index` 里那两行 `object.insert` 删掉
    /// ⇒ `compiler` 停在 `9.9.9` ⇒ 当场判红 ✓（反向验证已做）。
    #[test]
    fn the_artifact_stamp_follows_the_writing_compiler() {
        let root = tmp_root("stamp");
        assert!(ensure_layout(&root), "产物目录要建得起来");
        // 伪造一份「**旧编译器**建的目录」—— 就是用户现场那个形状。
        let meta = artifacts_dir(&root).join("meta.json");
        let schema = format!(
            "soko.artifacts/{ARTIFACTS_FORMAT}.r{}",
            crate::compile::REPORT_SHAPE
        );
        let stale = serde_json::json!({
            "schema": schema,
            "compiler": "9.9.9",
            "build_stamp": "0000000000000000",
            "platform": "test/test",
            "created_unix": 1,
            "written_unix": 1,
        });
        std::fs::write(&meta, format!("{stale}\n")).expect("write stale meta");

        let entry = root.join("Entry.sokonanoda");
        update_index(&root, &entry, "abc123");

        let raw = std::fs::read_to_string(&meta).expect("meta readable");
        let after: serde_json::Value = serde_json::from_str(&raw).expect("meta is json");
        let (compiler, build_stamp) = current_stamp();
        assert_eq!(
            after.get("compiler").and_then(|v| v.as_str()),
            Some(compiler),
            "版本戳必须刷新成**当前**编译器（修前这里停在 9.9.9）：{raw}"
        );
        assert_eq!(
            after.get("build_stamp").and_then(|v| v.as_str()),
            Some(build_stamp.as_str()),
            "build_stamp 同理：{raw}"
        );
        // **schema 已经是当前值**（这份目录是新编译器建的）⇒ 一个字节都不动
        //（幂等 ✓；升级那条在下面 `the_artifact_schema_is_upgraded_...` 里钉）。
        assert_eq!(
            after.get("schema").and_then(|v| v.as_str()),
            Some(schema.as_str()),
            "schema 已是最新时不许动：{raw}"
        );
        // 索引照常写入（这条是既有行为，顺带钉住"刷新没把别的字段挤掉"）。
        assert_eq!(
            after
                .get("entries")
                .and_then(|entries| entries.get(entry.display().to_string()))
                .and_then(|v| v.as_str()),
            Some("abc123"),
            "索引照常写入：{raw}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **需求 1（2026-10-09 用户实测）**：老产物目录的 `meta.json` 停在**旧 schema**
    /// ⇒ Infoview 显示「由编译器 ? 写入」✗。
    ///
    /// 现场形状（本地 `course/shared/.sokonanoda/meta.json` 逐字节如此）：`compiler`
    /// 已经被 [`update_index`] 刷新成当前版本（`0.87.0`），`schema` 却还停在
    /// `soko.artifacts/1`（当前是 `soko.artifacts/2.r5`）。两个读侧消费者都按
    /// `schema == meta_schema()` 过滤 ⇒ 整目录当不存在 ⇒ `compiler: null` ⇒ 面板
    /// 画成「由编译器 ? 写入」✗。
    ///
    /// 判据（**咬得住**）：把 `update_index` 里那段 schema 升级删掉 ⇒ 这条当场判红 ✓；
    /// 顺带钉住"升级后读侧真的认这个目录"（[`meta_ok`] 由 false → true）——
    /// 否则"升级了却还是读不到"会是第二个静默降级通道 ✗。
    #[test]
    fn the_artifact_schema_is_upgraded_when_the_directory_is_stale() {
        let root = tmp_root("schema-upgrade");
        assert!(ensure_layout(&root), "产物目录要建得起来");
        let meta = artifacts_dir(&root).join("meta.json");
        // 用户现场那份（schema 旧、compiler 新）。
        let stale = serde_json::json!({
            "schema": "soko.artifacts/1",
            "compiler": "0.87.0",
            "build_stamp": "0000000000000000",
            "platform": "test/test",
            "created_unix": 1,
            "written_unix": 1,
            "entries": {},
        });
        std::fs::write(&meta, format!("{stale}\n")).expect("write stale meta");
        assert!(
            !meta_ok(&root),
            "读侧纪律不变：旧 schema 的目录在升级前当不存在（宁可重算，绝不误用）"
        );

        let entry = root.join("Entry.sokonanoda");
        update_index(&root, &entry, "abc123");

        let raw = std::fs::read_to_string(&meta).expect("meta readable");
        let after: serde_json::Value = serde_json::from_str(&raw).expect("meta is json");
        assert_eq!(
            after.get("schema").and_then(|v| v.as_str()),
            Some(meta_schema().as_str()),
            "写产物必须把 schema 升到**当前**值（否则 `artifacts_of` 不报 compiler \
             ⇒ 面板「由编译器 ? 写入」✗）：{raw}"
        );
        let (compiler, build_stamp) = current_stamp();
        assert_eq!(
            after.get("compiler").and_then(|v| v.as_str()),
            Some(compiler),
            "schema 与版本戳**同一次**刷新（不许只升一个）：{raw}"
        );
        assert_eq!(
            after.get("build_stamp").and_then(|v| v.as_str()),
            Some(build_stamp.as_str()),
            "{raw}"
        );
        assert!(
            meta_ok(&root),
            "升级后读侧必须认这个目录（否则整个产物目录白写）：{raw}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
