//! **T1-C 的守卫**（2026-10-09）：**键优先、mtime 只兜底** —— 两条方向都要咬得住。
//!
//! ## 契约（写死在这里，判据在下面）
//!
//! * **mtime 变、内容不变 ⇒ 不许 miss** ✓（键是**内容**的纯函数 ⇒ 摸一下文件不该让产物失效）
//! * **内容变、mtime 不变 ⇒ 必须 miss** ✓（否则"改完再把时间戳改回去"就能读到**旧环境** ✗✗
//!   —— 那是**静默错判**，比慢得多严重）
//!
//! ## 为什么这条必须有
//!
//! 产物/缓存这两层都是**内容寻址**的（键 = 库层闭包的 Merkle 键 ✓）—— 而历史上
//! `compiled/` 那条用过 mtime 做"快路" ⇒ 与"键优先"是**两条互相矛盾的失效规则** ✗。
//! 只写契约不写守卫 = 下一棒改回 mtime 快路时没人拦 ✗。
//!
//! ## 怎么观测（不数计数器，看**文件本身**）
//!
//! * **命中** ⇒ 产物**不被重写** ⇒ 它的 mtime **不动** ✓；
//! * **未命中** ⇒ 重新落盘 ⇒ mtime **变** ✓，且（内容变了时）**多出一对** ✓。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::project::{artifacts, compile_plan_with_artifacts, plan_project};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("soko-t1c-{tag}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// 产物目录里 `.bin` 的 `(文件名, mtime)` 表（判据只看这两样 ✓）。
fn payload_mtimes(root: &std::path::Path) -> Vec<(String, std::time::SystemTime)> {
    let mut out: Vec<(String, std::time::SystemTime)> = std::fs::read_dir(artifacts::dir(root))
        .map(|d| {
            d.flatten()
                .filter(|e| e.file_name().to_string_lossy().ends_with(".bin"))
                .map(|e| {
                    let m = e
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    (e.file_name().to_string_lossy().to_string(), m)
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn compile_once(root: &std::path::Path) {
    let entry = root.join("E.sokonanoda");
    let plan = plan_project(&entry, None, Some(root));
    let _ = compile_plan_with_artifacts(plan, &CompileOptions::default());
}

#[test]
fn the_key_wins_over_mtime_in_both_directions() {
    let root = temp_root("contract");
    std::fs::create_dir_all(&root).expect("mkdir");
    let lib = root.join("Lib.sokonanoda");
    std::fs::write(&lib, "theorem l1 (P : Prop) (h : P) : P := by\n  exact h\n")
        .expect("write lib");
    std::fs::write(
        root.join("E.sokonanoda"),
        "import Lib\ntheorem e1 (P : Prop) : P → P := fun h => h\n",
    )
    .expect("write entry");

    // ① 冷编一次 ⇒ 产物落地 ✓（夹具自检：必须真的写了一对 ✓）。
    compile_once(&root);
    let first = payload_mtimes(&root);
    assert_eq!(
        first.len(),
        1,
        "夹具自检：冷编一次该落**一对**产物（实得 {} 对：{:?}）",
        first.len(),
        first
    );

    // ② **mtime 变、内容不变 ⇒ 不许 miss**（键是内容的纯函数 ✓）。
    //    把 Lib 的 mtime 往后推 2 秒（内容一字不动 ✓）—— 用 `filetime` 不可用 ⇒
    //    用"重写同样的内容"也能改 mtime ✓（内容相同 ⇒ 键相同 ✓）。
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let same = std::fs::read_to_string(&lib).expect("read lib");
    std::fs::write(&lib, &same).expect("rewrite same content");
    compile_once(&root);
    let second = payload_mtimes(&root);
    assert_eq!(
        second, first,
        "**mtime 变、内容不变 ⇒ 不许 miss** ✗：产物被重写了（或键变了）—— \
         键必须是**内容**的纯函数，摸一下文件不该让产物失效 ✓"
    );

    // ③ **内容变、mtime 不变 ⇒ 必须 miss**（否则能把时间戳改回去读旧环境 ✗✗）。
    //    改内容，然后把 mtime **恢复**成第一份产物的时间（尽力而为 ⇒ 用 `set_modified` ✓）。
    std::fs::write(
        &lib,
        "theorem l1 (P : Prop) (h : P) : P := by\n  exact h\ntheorem l2 (Q : Prop) (h : Q) : Q := by\n  exact h\n",
    )
    .expect("write new content");
    if let Ok(f) = std::fs::File::options().write(true).open(&lib) {
        let _ = f.set_modified(first[0].1);
    }
    compile_once(&root);
    let third = payload_mtimes(&root);
    assert_eq!(
        third.len(),
        2,
        "**内容变、mtime 不变 ⇒ 必须 miss** ✗：应当**多出一对**新产物（实得 {} 对：{:?}）—— \
         若这里仍是 1 对，说明失效看的是 mtime 而不是键 ⇒ 改完内容再把时间戳改回去就能读到\
         **旧环境**（静默错判 ✗✗）",
        third.len(),
        third
    );
    assert!(
        third.iter().any(|(name, _)| *name == first[0].0),
        "旧产物不该被删（新键 ⇒ 新的一对并存 ✓）"
    );

    let _ = std::fs::remove_dir_all(&root);
}
