// 引擎身份 = **这份二进制编进去的源**的摘要（CACHE-KEY-NO-ENGINE-IDENTITY，2026-10-14）。
//
// 为什么需要它：编译缓存的键（`compile::cache`）以前只折（版本 · 平台 · 内建 prelude
// 摘要 · `SOKO_BUILD_LABEL`）⇒ **同版本、同平台、只改了 `crates/front`/`crates/kernel`
// 其余源码**时键**逐字相同** ⇒ 跨引擎量同一棵树会**回放**旧引擎的判定 ✗（buildQ 实测：
// 同一夹具同一路径、同一份空缓存起步、两个 `NO_*ARTIFACTS` 逃生门都开 ⇒ 旧引擎
// `exit 1` → 新引擎 `exit 0` → **旧引擎再来一次 `exit 0`**）。
//
// ## 覆盖什么（判据 = M-07 那条：改了它 ⇒ 同一份用户源码的判定/显示会变）
//
// * `crates/front/src`：前端全部（判定/报告/事件的产出方）；
// * `crates/kernel/src`：内核（**判定本体**）；
// * `prelude/`、`reference/`：这两棵树的文件由 `include_str!` **编进二进制**
//   （`prelude/Eq|L1|Quot.sokonanoda`、`reference/{commands,tactics}/*.md`）⇒ 改一行
//   就是另一个引擎（`prelude/` 里另有 `Prelude.sokonanoda` 这个**合并视图**、`reference/`
//   里另有 README —— 它们不是 `include_str!` 的输入，多折一次只多一次失效，
//   换掉"漏一个文件就少一处失效"的口子 ✓）；
// * `crates/front/build.rs`：摘要算法自己的入口。
//
// **不进**的：`crates/cli` 与 `crates/lsp` —— 它们只是把 front 产出的报告/事件
// **原样**存进缓存（`cli/src/check.rs::compile_cached` / `cli/src/build.rs` 三处
// `cache::store*`），改它们不会改变缓存里的**字节** ✓；`crates/*/tests` 不进二进制 ✓。
//
// ## 为什么不是"哈希 `current_exe()`"
//
// 那是 G-27 修过的坑：CLI 与 LSP 是**两个**可执行文件 ⇒ 内容哈希不同 ⇒
// `sokonanoda build` 预热出来的条目编辑器**永远够不着** ✗。身份必须是**源码级**的
// 编译期常量：同一份源码编出来的 CLI/LSP 拿到**同一个**值 ✓。
//
// ## 稳定性
//
// 只折**相对仓库根**的路径 + 文件**内容**（不折绝对路径、不折 mtime、不折文件系统
// 属性）⇒ 同一份源码在任何机器/任何 checkout 上算出同一个摘要 ✓。换行符差异会让摘要
// 不同 —— 与既有的 `builtin_prelude_digest`（同样按字节折 `include_str!` 的源）**同款**，
// 不新增口径 ✓（而且它俩本来就被 OS/ARCH 分开）。
//
// ⚠ **本文件被 `build.rs` 用 `include!` 拼进 build script 的 crate 根**（单一真相：
// 算法只有这一份，`build.rs` 与单测都调它 ⇒ 三者不许漂移）⇒ 顶部只能用**外注**
// （`//`），**不许 `//!`**（内属性在宏展开的位置非法 ✗）。它同时是 `engine_digest.rs`
// 的 `scan` 子模块（**只在测试里编** ⇒ 生产路径不留死代码 ✓）。
//
// ⚠ 身份**常量**（`env!("SOKO_ENGINE_DIGEST")`）与逃生门在 `../engine_digest.rs`，
// **不在**本文件：build script 编译时那个变量还不存在 —— 它是 build script **产出**的 ✓。

use std::path::{Path, PathBuf};

/// 引擎的输入：`(相对仓库根的路径前缀, 绝对路径)`。目录会被递归展开，文件就是它自己。
///
/// `manifest_dir` = `crates/front`（`CARGO_MANIFEST_DIR`）⇒ 仓库根 = 它的上两级。
pub(crate) fn engine_inputs(manifest_dir: &Path) -> Vec<(&'static str, PathBuf)> {
    let repo = manifest_dir.join("..").join("..");
    vec![
        ("crates/front/src", manifest_dir.join("src")),
        ("crates/front/build.rs", manifest_dir.join("build.rs")),
        (
            "crates/kernel/src",
            repo.join("crates").join("kernel").join("src"),
        ),
        ("prelude", repo.join("prelude")),
        ("reference", repo.join("reference")),
    ]
}

/// 引擎输入里的一个文件：`rel` 是**相对仓库根**的路径（进摘要的那一份），
/// `abs` 是读内容用的绝对路径（**不进**摘要 ⇒ 换 checkout 不改摘要 ✓）。
pub(crate) struct EngineFile {
    pub rel: String,
    pub abs: PathBuf,
}

/// 把 [`engine_inputs`] 展开成**排序好**的文件表（按 `rel` 排 ⇒ 与目录枚举顺序无关 ✓）。
pub(crate) fn engine_files(manifest_dir: &Path) -> Vec<EngineFile> {
    let mut out = Vec::new();
    for (prefix, path) in engine_inputs(manifest_dir) {
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => walk(&path, prefix, &mut out),
            Ok(_) => out.push(EngineFile {
                rel: prefix.to_string(),
                abs: path,
            }),
            // 读不到的输入**不静默跳过**：折一个标记名进去（build script 另发
            // `cargo:warning` 出声，单测也会因为"和真树算不出同一个摘要"判红 ✓）。
            Err(_) => out.push(EngineFile {
                rel: prefix.to_string(),
                abs: path,
            }),
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<EngineFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let rel = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if kind.is_dir() {
            walk(&entry.path(), &rel, out);
        } else if kind.is_file() {
            // 符号链接**不跟**（不引入环）。本仓这五棵输入树里没有符号链接；
            // 真要加，得同时想清楚"链到树外"算不算引擎的一部分。
            out.push(EngineFile {
                rel,
                abs: entry.path(),
            });
        }
    }
}

/// 摘要本体（**纯函数**：输入就是 `(rel, 内容)` 表 ⇒ 可单测 ✓）。FNV-1a 64，
/// 与键的其余分量同一族（键本身就是 FNV-1a 64 ⇒ 这里不用更长的哈希也不构成短板 ✓）。
///
/// 编码：先折文件**个数**，再逐个折 `rel` + `\0` + 内容 + `\0` ⇒ 分隔符让
/// "一个文件里带 `\0`"与"两个文件"不许撞车 ✓。
pub(crate) fn engine_digest(contents: &[(String, Vec<u8>)]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in (contents.len() as u64).to_le_bytes() {
        eat(byte);
    }
    for (rel, bytes) in contents {
        for byte in rel.as_bytes() {
            eat(*byte);
        }
        eat(0);
        for byte in bytes {
            eat(*byte);
        }
        eat(0);
    }
    hash
}

/// 读真树并算摘要（`build.rs` 与「接线」单测**共用这一条** ⇒ 不会各算各的 ✓）。
///
/// 返回 `(摘要, 读不到的输入的 rel)`。读不到的文件折 `unreadable:<rel>` 而不是静默跳过：
/// 那样「少看一个文件」会**改变**摘要（可见 ✓），而静默跳过会让摘要变成「少看一棵树」
/// 却没人知道 ✗。第二个返回值给 `build.rs` 发 `cargo:warning` 用。
pub(crate) fn digest_of_tree(manifest_dir: &Path) -> (u64, Vec<String>) {
    let mut contents: Vec<(String, Vec<u8>)> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    for file in engine_files(manifest_dir) {
        match std::fs::read(&file.abs) {
            Ok(bytes) => contents.push((file.rel, bytes)),
            Err(_) => {
                unreadable.push(file.rel.clone());
                contents.push((format!("unreadable:{}", file.rel), Vec::new()));
            }
        }
    }
    (engine_digest(&contents), unreadable)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一棵**合成引擎树**（`crates/front/src` + `crates/kernel/src` + `prelude/` +
    /// `reference/` + `build.rs`）—— 判据不许碰真树（真树是别人的工作区 ✗）。
    fn tmp_engine(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("soko-engine-digest-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let front = root.join("crates/front");
        std::fs::create_dir_all(front.join("src")).expect("front/src");
        std::fs::create_dir_all(root.join("crates/kernel/src")).expect("kernel/src");
        std::fs::create_dir_all(root.join("prelude")).expect("prelude");
        std::fs::create_dir_all(root.join("reference/tactics")).expect("reference");
        std::fs::write(front.join("build.rs"), "fn main() {}\n").expect("build.rs");
        std::fs::write(front.join("src/lib.rs"), "pub fn f() {}\n").expect("lib.rs");
        std::fs::write(root.join("crates/kernel/src/lib.rs"), "pub fn k() {}\n").expect("kernel");
        std::fs::write(root.join("prelude/L1.sokonanoda"), "axiom P : Prop\n").expect("prelude");
        std::fs::write(root.join("reference/tactics/intro.md"), "# intro\n").expect("md");
        front
    }

    /// 摘要那一半（读得到的树 ⇒ 第二个返回值必空 ✓）。
    fn tree(manifest_dir: &Path) -> u64 {
        let (digest, unreadable) = digest_of_tree(manifest_dir);
        assert!(unreadable.is_empty(), "夹具树必须读得到：{unreadable:?}");
        digest
    }

    /// **判据 ①**：摘要必须**对内容敏感** —— 五棵输入树里**任何一棵**改一个字节都得变
    /// （少一棵就是"改了引擎却不失效"的口子 ✗）。
    #[test]
    fn a_byte_in_any_engine_tree_changes_the_digest() {
        let front = tmp_engine("sensitive");
        let base = tree(&front);
        assert_ne!(base, 0, "退化值会把引擎分量抹掉 ✗");
        let edits: [(&str, &str); 5] = [
            ("crates/front/src/lib.rs", "pub fn f() { /* x */ }\n"),
            ("crates/kernel/src/lib.rs", "pub fn k() { /* x */ }\n"),
            ("prelude/L1.sokonanoda", "axiom P : Prop -- x\n"),
            ("reference/tactics/intro.md", "# intro x\n"),
            ("crates/front/build.rs", "fn main() { /* x */ }\n"),
        ];
        for (rel, text) in edits {
            let path = front.join("../..").join(rel);
            let old = std::fs::read_to_string(&path).expect("read");
            std::fs::write(&path, text).expect("write");
            assert_ne!(
                tree(&front),
                base,
                "`{rel}` 改一个字节 ⇒ 摘要必须变（否则「改了引擎却命中旧条目」✗）"
            );
            std::fs::write(&path, old).expect("restore");
            assert_eq!(tree(&front), base, "还原 ⇒ 摘要必须回到原值（纯函数 ✓）");
        }
        let _ = std::fs::remove_dir_all(front.join("../.."));
    }

    /// **判据 ②**：**多一个文件**也得变（只折"已知文件的内容"会漏掉新增模块 ✗），
    /// 且路径本身进摘要（同名不同目录不许撞车 ✓）。
    #[test]
    fn adding_or_renaming_a_file_changes_the_digest() {
        let front = tmp_engine("addfile");
        let base = tree(&front);
        std::fs::write(front.join("src/new_mod.rs"), "pub fn g() {}\n").expect("new");
        let added = tree(&front);
        assert_ne!(added, base, "新增文件必须改变摘要");
        std::fs::rename(front.join("src/new_mod.rs"), front.join("src/other.rs")).expect("rename");
        assert_ne!(
            tree(&front),
            added,
            "改名（内容一字不变）也必须改变摘要（路径进摘要 ✓）"
        );
        let _ = std::fs::remove_dir_all(front.join("../.."));
    }

    /// **判据 ③**：纯函数 + 分隔符编码 —— 同输入同摘要；"一个文件带 `\\0`"与
    /// "两个文件"不许折成同一个值（否则摘要可以被内容构造撞掉 ✓）。
    #[test]
    fn the_digest_is_pure_and_its_encoding_is_unambiguous() {
        let one = vec![("a".to_string(), b"x\0y".to_vec())];
        assert_eq!(engine_digest(&one), engine_digest(&one.clone()));
        let two = vec![
            ("a".to_string(), b"x".to_vec()),
            ("b".to_string(), b"y".to_vec()),
        ];
        assert_ne!(engine_digest(&one), engine_digest(&two));
        let empty: Vec<(String, Vec<u8>)> = Vec::new();
        assert_ne!(engine_digest(&empty), engine_digest(&one));
    }
}
