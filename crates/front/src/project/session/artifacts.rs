//! **产物层**（2026-10-13 拆出来的子模块 —— `AGENTS.md` 硬规则 5；判据 ⑨
//! （`STRUCT-SRC-LINES`）对 `session.rs` 是**只许减不许增** ✓）。
//!
//! 装在这里的三样东西，正好是**磁盘产物那一层的全部**：
//!
//! 1. **§8.3 的闸门**（[`shared_artifacts_enabled`]）—— **默认关**（红线 ✓）：关着时产物层
//!    用**检查点键**（[`super::lib_key`]，含每个模块的**绝对路径** + `SOKO_*`，T-A06 的语义）
//!    ⇒ 产物只可能由**这个根、这个环境**写出来 ⇒ **外来产物不在候选里** ✓；打开（显式同意）
//!    才改用**内容寻址键**（[`artifact_layer_key`] → `cache::content_key`，**无路径、无环境**✓）
//!    ⇒ 可跨 root / 可下载 ✓，且读侧**还要**过 `artifacts.lock.json`（§8.3 清单）✓。
//! 2. **两把键的拆分**：[`lib_artifact_key_content`]（产物键，§3）vs [`super::lib_key`]
//!    （检查点键）；收口点是 [`artifact_layer_key`] ✓。
//! 3. **产物的读/写/装载**（[`read_lib_artifact`] / [`write_lib_artifact`] /
//!    [`load_lib_checkpoint_in`]）✓。
//!
//! ⚠ **检查点那一侧的键一个字没动**：线程局部 LRU 的身份恒为 `lib_key` ✓
//! （装载出来的检查点也拿 `lib_key` 当身份 —— 见调用方传的 `checkpoint_key` ✓）。

use std::sync::atomic::Ordering;

use sokonanoda::builder::EnvBuilder;
use stumpalo::ArenaRef;

use super::{lib_key, LibCheckpoint, ResumeState, SourceUnit, LEAKED_LIB_ARENAS};
use crate::compile::CompileOptions;

/// **T1-B 批 2**：把一份磁盘产物装成 [`LibCheckpoint`]（`'static` —— 要进线程局部 LRU ✓）。
///
/// ⚠ **只装 arena 不装单元**：产物里没有源文本（它存的是**结果** ✓）⇒ 这条路上
/// **不需要** `leak_lib_units`（省一份泄漏 ✓）。
///
/// 装出来的检查点与"整条趟"那份**逐字段可比**：`key`（产物键 = 库层摘要 ✓）·
/// `shape = None`（整条命中不比形状 —— 键已经把整条源文本折进去了 ✓，与
/// [`LibCheckpoint`] 的既有约定一致 ✓）· `resume = default`（同上 ✓）·
/// `ranges` 由**本次的 units** 现算（它是 units 的纯函数 ✓，不存进产物 ✓）。
pub(super) fn load_lib_checkpoint(
    text: &str,
    key: &str,
    lib_units: &[SourceUnit<'_>],
) -> Option<LibCheckpoint<'static>> {
    let arena: &'static ArenaRef<'static> =
        Box::leak(Box::new(stumpalo::Arena::new())).as_arena_ref();
    LEAKED_LIB_ARENAS.fetch_add(1, Ordering::Relaxed);
    load_lib_checkpoint_in(arena, text, key, lib_units)
}

/// 装载的**内核**：把产物装进**调用方给的 arena**（寿命由调用方负责 ✓）。
///
/// * `'static` arena（LSP 那条路）⇒ 结果进线程局部 LRU ✓；
/// * **栈上** arena（CLI 那条路）⇒ 结果只活本次调用 ⇒ **零泄漏** ✓。
pub(super) fn load_lib_checkpoint_in<'a>(
    arena: &'a ArenaRef<'a>,
    text: &str,
    key: &str,
    lib_units: &[SourceUnit<'_>],
) -> Option<LibCheckpoint<'a>> {
    let (env, tables, facts) = crate::project::artifacts::decode_payload(arena, text).ok()?;
    let builder = EnvBuilder::from_export_file(arena, env);
    Some(LibCheckpoint {
        key: key.to_string(),
        shape: None,
        resume: ResumeState::default(),
        builder,
        tables,
        out: facts.out,
        reports: facts.reports,
        ranges: facts.ranges.iter().map(|(a, b)| *a..*b).collect(),
        n_commands: facts.n_commands,
        prefix_commands: facts.prefix_commands,
        reuses: 0,
        // **产物里不存这两张表**（它们是**文本的纯函数** ✓，存了只是把产物撑大 ✗）⇒
        // 装载时按 `lib_units` 现算**一次** ✓（不是每刀 ✓）。
        lib_display: crate::compile::display_notations(lib_units),
        lib_defs: crate::compile::top_level_def_spans_over(lib_units),
        lib_prefix: facts.lib_prefix,
    })
}

/// **T1-B 批 2**：库层趟跑完 ⇒ 写一份产物。**best-effort**（任何一步失败都静默跳过 ✓）：
/// 产物是**加速件**，写不出来只该"下次还慢"，**绝不该**让本次编译失败或改判 ✗。
pub(super) fn write_lib_artifact(
    root: &std::path::Path,
    key: &str,
    lib: &LibCheckpoint<'_>,
    lib_units: &[SourceUnit<'_>],
    options: &CompileOptions,
) {
    let facts = crate::project::artifacts::LibPassFacts {
        out: lib.out.clone(),
        reports: lib.reports.clone(),
        ranges: crate::compile::unit_ranges(lib_units)
            .into_iter()
            .map(|r| (r.start, r.end))
            .collect(),
        n_commands: lib.n_commands,
        prefix_commands: lib.prefix_commands,
        lib_prefix: lib.lib_prefix.clone(),
    };
    let Ok(text) =
        crate::project::artifacts::encode_payload(&lib.builder.snapshot(), &lib.tables, &facts)
    else {
        return;
    };
    let _ = crate::project::artifacts::write(root, key, &text, options);
}

/// **§8.3 的闸门（默认关 —— 这是红线 ✓）**：`SOKONANODA_SHARED_ARTIFACTS=1` ⇒ 磁盘产物层
/// 改用**内容寻址键**（[`lib_artifact_key_content`]）并接受 `artifacts.lock.json` 清单允许的产物。
///
/// ## 为什么必须与"内容寻址"**同一轮**落（否则就是一个信任口子 ✗）
///
/// 产物 = **直接注入内核的声明**（设计 §8.3）⇒ 一把**不含路径**的键会让"**任何**放进制品目录的
/// 外来产物"都匹配上 ✗。而"内容寻址"**挡不住伪造**：键由**输入**算出，不校验产物**内容**是否真由
/// 这份输入编出来（能挡的只有 §8.2 的损坏/错配 ✓）。⇒ 闸门关着时**根本不用那把键**
/// ⇒ **外来产物不在候选里** ✓✓。
///
/// ## 默认关怎么还能被验证
///
/// 开闸的**唯一**手段就是这条环境变量（`=1` 才算；`0`/空/false/off 一律当关 ✓，免得用户写
/// `=0` 反而打开 ✗）⇒ 判据可以**自己开关它**跑正反两臂 ✓
/// （`crates/cli/tests/module_artifacts_shared.rs`）。
///
/// ⚠ **诚实边界**：设计 §8.3 写的是"**设置项**默认 `false`"。今天落地的**是环境变量**，不是
/// `sokonanoda.toml` 的设置项；清单的**密码学签名**（发布者公钥）**没做** —— 清单只提供
/// "**按摘要钉死**"这一层（清单被篡改 ⇒ 对不上 ⇒ 拒绝 ✓）。
pub fn shared_artifacts_enabled() -> bool {
    super::module_artifacts_enabled() && env_flag_on("SOKONANODA_SHARED_ARTIFACTS")
}

/// 环境变量"**开着**"的判定：只有 `1`/`true`/`yes`/`on`（大小写不敏感、去空白）算开 ✓。
///
/// 为什么不照仓里既有的"**存在即开**"（`var_os(..).is_none()` 那一族）：那些是**逃生门**
/// （默认开、设了就关 ✓），而这条是**默认关的同意开关** —— 「存在即开」会让
/// `SOKONANODA_SHARED_ARTIFACTS=0` **反而打开**信任口子 ✗（用户写 `=0` 的意图是关 ✓）。
fn env_flag_on(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

/// **§3 的产物键**（**无路径、无环境**，只由**内容**决定 —— `docs/design/module-artifacts.md`
/// §3/§8.1）：同一模块在**任何项目、任何机器**上落到**同一个名字** ⇒ 天然可共享 ✓。
///
/// **与 [`super::lib_key`] 差在哪**（这就是"键拆分"的全部内容 ✓）：
/// * **不含 `unit.path`** ⇒ 两个目录下逐字相同的项目**共用同一把产物键** ✓（这正是共享的前提，
///   也正是 `lib_key` **故意不做**的事 —— T-A06 的语义在检查点那一侧**原样保留** ✓）；
/// * **不含 `SOKO_*`**（走 `cache::content_key`）⇒ 下载方的环境不同也算出同一个键 ✓；
/// * **自己一个 tag**（[`LIB_ARTIFACT_KEY_TAG`]）⇒ 与检查点键**永不可能撞名** ✓。
///
/// ⚠ **它默认不被使用**（见 [`shared_artifacts_enabled`]）。
fn lib_artifact_key_content(lib_units: &[SourceUnit<'_>], options: &CompileOptions) -> String {
    let mut text = String::from(LIB_ARTIFACT_KEY_TAG);
    for unit in lib_units {
        push_lib_artifact_key_text(&mut text, unit);
    }
    crate::compile::cache::content_key(&text, options)
}

/// **产物层的键** —— 磁盘读写**唯一**该用的那把（键拆分后的收口点 ✓）。
///
/// * **闸门关（默认 ✓）** ⇒ [`super::lib_key`]：含**模块绝对路径** + `SOKO_*`（T-A06 的语义）
///   ⇒ **任何外来产物默认不被吃** ✓；
/// * **闸门开** ⇒ [`lib_artifact_key_content`]：内容寻址 ⇒ 可跨 root / 可下载 ✓，
///   且读侧**还要**过 §8.3 的清单（`artifacts.lock.json`）⇒ 不是"打开就什么都吃" ✓。
pub(super) fn artifact_layer_key(lib_units: &[SourceUnit<'_>], options: &CompileOptions) -> String {
    if shared_artifacts_enabled() {
        lib_artifact_key_content(lib_units, options)
    } else {
        lib_key(lib_units, options)
    }
}

/// **产物键**文本的头（与 `LIB_KEY_TAG` 并列、**不同** ⇒ 两条链不会撞 ✓）。
const LIB_ARTIFACT_KEY_TAG: &str = "soko.lib-artifact/1\0";

/// 把一个单元**折叠进产物键文本**（[`lib_artifact_key_content`] 的**唯一**实现 ✓）。
///
/// ⚠ 与 `super::push_lib_key_text` **只差一行**（`unit.path`）—— **故意的**：两处并排就自明
/// "产物键凭什么可共享"✓；合成一个有 `include_path: bool` 参数的助手会让那一行**藏起来** ✗
/// （而那正是本环节要钉死的那一行 ✓）。
fn push_lib_artifact_key_text(text: &mut String, unit: &SourceUnit<'_>) {
    text.push_str(unit.name);
    text.push('\0');
    text.push_str(&unit.file.src);
    text.push('\0');
}

/// **读一份磁盘产物** —— 产物层读侧的**唯一**入口（键拆分 + §8.3 闸门在这里收口 ✓）。
///
/// * 闸门关（默认 ✓）⇒ 与 T1-B 落地的行为**逐字节相同**（`artifacts::read` 的 §8.2 三道 ✓）；
/// * 闸门开 ⇒ 先过 §8.2 三道，**再**过 §8.3 的清单 ⇒ 两关都过才用，并**报一行来源**
///   （§8.3「不静默」✓ —— 谁给的、哪个 key ✓）。
///
/// ⚠ **报来源走 stderr**：`--json` 的契约在 **stdout**（`docs/protocol.md`）⇒ 这一行
/// **不进** `--json` 流 ⇒ 逐字节红线不受影响 ✓。
pub(super) fn read_lib_artifact(
    root: &std::path::Path,
    key: &str,
    options: &CompileOptions,
) -> Option<String> {
    if !shared_artifacts_enabled() {
        return crate::project::artifacts::read(root, key, options);
    }
    let text = crate::project::artifacts::read_shared(root, key, options)?;
    eprintln!(
        "artifacts: shared manifest={} key={key}",
        crate::project::artifacts::lock_path(root).display()
    );
    Some(text)
}
