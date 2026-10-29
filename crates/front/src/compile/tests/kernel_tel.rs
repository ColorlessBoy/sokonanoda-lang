//! **P1-F1 判据**：[`kernel_telescope`] 的形状、全课程库一致性扫描、暖路径（产物装载）。
//!
//! 夹具 = prelude + `courses/set-theory/lib/*.sokonanoda` 全部 17 个模块：走产品的
//! 计划步骤（`plan_project_with_overlay` → `precheck_plan` → `units_for_modules`，
//! 与 `project::warm_library_checkpoint` 同序）+ 库层趟同款的 `run_pass_with` 调用。

use super::elab::KnownName;
use super::kernel_tel::{kernel_telescope, KernelTelescope, ParamKind};
use super::tests::on_compile_stack;
use super::{run_pass_with, CompileOptions, PassTables};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::Config;
use std::path::PathBuf;

const LIB_MODULES: [&str; 17] = [
    "Cardinal",
    "Choice",
    "Demo",
    "Equiv",
    "Exists",
    "Extensionality",
    "Fun",
    "Image",
    "Logic",
    "Order",
    "Ordinal",
    "Prod",
    "Rel",
    "Set",
    "Sum",
    "SUnion",
    "ZF",
];

fn course_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../courses/set-theory")
}

/// prelude + 17 个课程库模块编完之后的 `(builder, tables)` 交给 `f`（arena 只活在本函数里）。
fn with_course_lib<R>(f: impl FnOnce(&mut EnvBuilder<'_>, &PassTables<'_>) -> R) -> R {
    // 合成入口（磁盘上不存在，源码走 `entry_src`）：只为让闭包装进全部 17 个库模块。
    let entry = course_root().join("f1_kernel_tel_probe.sokonanoda");
    let entry_src: String = LIB_MODULES
        .iter()
        .map(|m| format!("import lib.{m}\n"))
        .collect();
    let options = CompileOptions::default();
    let mut plan = crate::project::plan_project_with_overlay(&entry, Some(&entry_src), None, &[]);
    assert!(
        plan.diagnostics.is_empty(),
        "计划期诊断：{:?}",
        plan.diagnostics
    );
    crate::project::precheck_plan(&mut plan, &options);
    let entry_path = plan.entry.clone();
    let units = crate::project::units_for_modules(&plan, |m| m.path != entry_path);
    assert_eq!(units.len(), LIB_MODULES.len(), "库层单元数");
    let arena = stumpalo::Arena::new();
    let builder = EnvBuilder::new(arena.as_arena_ref(), Config::default());
    // 与 `project/session.rs::run_library_pass` 同一组实参（只把 `snapshot_state` 关掉）。
    let (pass, mut builder, tables, _, _) = run_pass_with(
        builder,
        None,
        true,
        PassTables::new(),
        &units,
        &options,
        true,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        0,
        false,
        None,
        None,
        false,
        false,
    );
    assert_eq!(pass.out.errors, vec![], "课程库必须零错误编完");
    f(&mut builder, &tables)
}

fn decl<'k>(tables: &'k PassTables<'_>, name: &str) -> &'k KnownName {
    match tables.known.get(name) {
        Some(entry @ KnownName::Decl { .. }) => entry,
        other => panic!("`{name}` 必须是 known 里的 Decl，实际 {other:?}"),
    }
}

fn kinds(tel: &KernelTelescope<'_>) -> Vec<ParamKind> {
    tel.params.iter().map(|p| p.kind).collect()
}

fn names<'t>(tel: &'t KernelTelescope<'_>) -> Vec<&'t str> {
    tel.params.iter().map(|p| p.name.as_str()).collect()
}

fn explicit_count(tel: &KernelTelescope<'_>) -> usize {
    tel.params
        .iter()
        .filter(|p| p.kind == ParamKind::Explicit)
        .count()
}

/// 只看内核 `binder_style`（把 `implicit_prefix` 当 0）的档 —— 判"前缀覆盖改了几位"。
fn raw_kinds(builder: &mut EnvBuilder<'_>, name: &str) -> Vec<ParamKind> {
    let raw = KnownName::Decl {
        universes: Vec::new(),
        implicit_prefix: 0,
        explicit_arity: 0,
        signature: None,
    };
    kinds(&kernel_telescope(builder, &raw, name).expect("raw telescope"))
}

fn kind_text(kinds: &[ParamKind]) -> String {
    kinds
        .iter()
        .map(|k| match k {
            ParamKind::Explicit => 'E',
            ParamKind::Implicit => 'I',
            ParamKind::InstImplicit => 'S',
        })
        .collect()
}

/// **判据 1（形状）**：五个真实声明的 binder 档 / 个数 / 名字 / 宇宙，照它们在
/// `prelude/*.sokonanoda` 与 `courses/set-theory/lib/` 里的**实际写法**断言。
#[test]
fn kernel_telescope_shapes_of_real_declarations() {
    use ParamKind::{Explicit as E, Implicit as I};
    on_compile_stack(|| {
        with_course_lib(|builder, tables| {
            // lib/Exists：`inductive Exists (A : Type) (p : A → Prop) : Prop` +
            // `ctor intro (w : A) (h : p w)` ⇒ 归纳参数 A、p 在内核里是 `Default`，
            // 由 `implicit_prefix = 2` 覆盖成隐式。
            let entry = decl(tables, "Exists.intro");
            assert_eq!(entry.implicit_prefix(), 2);
            let tel = kernel_telescope(builder, entry, "Exists.intro").expect("Exists.intro");
            assert_eq!(kinds(&tel), vec![I, I, E, E]);
            assert_eq!(names(&tel), vec!["A", "p", "w", "h"]);
            assert_eq!(
                raw_kinds(builder, "Exists.intro"),
                vec![E, E, E, E],
                "构造子的归纳参数在内核类型里是 Default binder"
            );
            assert!(tel.universes.is_empty());
            // 参数类型是**内核**项：`A : Type` 是 `Sort`，`w : A` 是指回 A 的 de Bruijn 变量。
            assert!(matches!(
                *tel.params[0].ty,
                sokonanoda::expr::Expr::Sort { .. }
            ));
            assert!(
                !matches!(*tel.params[2].ty, sokonanoda::expr::Expr::Sort { .. }),
                "w 的类型是 A（变量），不是 Sort：{:?}",
                *tel.params[2].ty
            );

            // lib/Exists：`theorem Exists.elim {A} {p} {Q} (h : Exists A p) (f : (w : A) → p w → Q) : Q`
            // （`f` 的类型是参数类型，不被剥开）。
            let entry = decl(tables, "Exists.elim");
            let tel = kernel_telescope(builder, entry, "Exists.elim").expect("Exists.elim");
            assert_eq!(kinds(&tel), vec![I, I, I, E, E]);
            assert_eq!(names(&tel), vec!["A", "p", "Q", "h", "f"]);
            assert_eq!(raw_kinds(builder, "Exists.elim"), kinds(&tel));

            // prelude/L1：`inductive And (a b : Prop)` + `ctor And.intro (ha : a) (hb : b)`。
            let entry = decl(tables, "And.intro");
            assert_eq!(entry.implicit_prefix(), 2);
            let tel = kernel_telescope(builder, entry, "And.intro").expect("And.intro");
            assert_eq!(kinds(&tel), vec![I, I, E, E]);
            assert_eq!(names(&tel), vec!["a", "b", "ha", "hb"]);
            assert_eq!(raw_kinds(builder, "And.intro"), vec![E, E, E, E]);

            // lib/Set：`namespace Set` 里的 `def mem {α : Type} (a : α) (A : Set α) : Prop`；
            // 结果是 `Prop`（语法上的 `Sort`，不 whnf）。
            let entry = decl(tables, "Set.mem");
            let tel = kernel_telescope(builder, entry, "Set.mem").expect("Set.mem");
            assert_eq!(kinds(&tel), vec![I, E, E]);
            assert_eq!(names(&tel), vec!["α", "a", "A"]);
            assert!(
                matches!(*tel.result, sokonanoda::expr::Expr::Sort { .. }),
                "Set.mem 的结果是 Prop，实际 {:?}",
                *tel.result
            );

            // prelude/Eq：`axiom Eq.subst {u} : {α : Sort u} -> {p : α -> Prop} -> {a : α} ->
            // {b : α} -> @Eq.{u} α a b -> p a -> p b`（箭头形 binder 链；后两位是匿名 binder）。
            let entry = decl(tables, "Eq.subst");
            assert_eq!(entry.implicit_prefix(), 4);
            let tel = kernel_telescope(builder, entry, "Eq.subst").expect("Eq.subst");
            assert_eq!(kinds(&tel), vec![I, I, I, I, E, E]);
            assert_eq!(&names(&tel)[..4], &["α", "p", "a", "b"]);
            assert_eq!(tel.universes, vec!["u".to_string()]);
            assert_eq!(raw_kinds(builder, "Eq.subst"), kinds(&tel));

            // 不在环境里 ⇒ `None`（调用方回落）。
            assert!(kernel_telescope(builder, entry, "No.such.decl").is_none());
            // 非 `Decl` ⇒ `None`。
            let alias = KnownName::Alias {
                canonical: "Exists.intro".to_string(),
            };
            assert!(kernel_telescope(builder, &alias, "Exists.intro").is_none());
        })
    })
}

/// **判据 2（一致性扫描）**：prelude + 17 个库模块里**每一条** `KnownName::Decl`。
///
/// 硬断言（不变式）：内核声明找得到；前 `implicit_prefix` 个参数存在；宇宙参数名与登记一致。
/// **只报不断言**：内核望远镜的显式个数 vs 源级 `explicit_arity`（两者口径本来就不同，见报告）。
#[test]
fn kernel_telescope_consistency_sweep_over_course_lib() {
    on_compile_stack(|| {
        with_course_lib(|builder, tables| {
            let mut keys: Vec<&String> = tables.known.keys().collect();
            keys.sort();
            let (mut decls, mut missing, mut short, mut univ, mut arity) =
                (0usize, Vec::new(), Vec::new(), Vec::new(), Vec::new());
            let mut prefix_overrides = 0usize;
            // 非空扫描的读数：比较真的发生在有参数的声明上。
            let (mut total_explicit, mut total_params) = (0usize, 0usize);
            for name in keys {
                let entry = &tables.known[name];
                let KnownName::Decl {
                    universes,
                    implicit_prefix,
                    explicit_arity,
                    signature,
                } = entry
                else {
                    continue;
                };
                decls += 1;
                let Some(tel) = kernel_telescope(builder, entry, name) else {
                    missing.push(name.clone());
                    continue;
                };
                if tel.params.len() < *implicit_prefix {
                    short.push(format!(
                        "{name}: implicit_prefix={implicit_prefix} > kernel params={}",
                        tel.params.len()
                    ));
                }
                if &tel.universes != universes {
                    univ.push(format!(
                        "{name}: known={universes:?} kernel={:?}",
                        tel.universes
                    ));
                }
                let raw = raw_kinds(builder, name);
                if raw != kinds(&tel) {
                    prefix_overrides += 1;
                    println!(
                        "KERNEL_TEL_OVERRIDE {name}: raw={} kinds={}",
                        kind_text(&raw),
                        kind_text(&kinds(&tel))
                    );
                }
                let got = explicit_count(&tel);
                total_explicit += got;
                total_params += tel.params.len();
                if got != *explicit_arity {
                    arity.push(format!(
                        "{name}: explicit_arity={explicit_arity} kernel_explicit={got} \
                         kernel_params={} implicit_prefix={implicit_prefix} \
                         kinds={} raw={} sig={}",
                        tel.params.len(),
                        kind_text(&kinds(&tel)),
                        kind_text(&raw),
                        signature.as_deref().unwrap_or("<none>"),
                    ));
                }
            }
            println!(
                "KERNEL_TEL_SWEEP decls={decls} missing={} short_prefix={} universe_mismatch={} \
                 prefix_overrides={prefix_overrides} explicit_arity_mismatch={} \
                 total_params={total_params} total_explicit={total_explicit}",
                missing.len(),
                short.len(),
                univ.len(),
                arity.len()
            );
            for line in &arity {
                println!("KERNEL_TEL_ARITY {line}");
            }
            assert!(decls > 0, "扫描必须看到声明");
            assert_eq!(
                missing,
                Vec::<String>::new(),
                "known 里的 Decl 必须都在内核环境里"
            );
            assert_eq!(
                short,
                Vec::<String>::new(),
                "前 implicit_prefix 个参数必须存在"
            );
            assert_eq!(univ, Vec::<String>::new(), "宇宙参数名必须与登记一致");
        })
    })
}

/// **判据 3（暖路径）**：库层产物装载那条路（`project/session/artifacts.rs::load_lib_checkpoint_in`
/// = `decode_payload` + `EnvBuilder::from_export_file`，入口趟再 `clone` + `seal_library_layer`，
/// 见 `project/session.rs::run_entries`）上，`known` 的每条 `Decl` 的内核声明都在 builder 里，
/// 且望远镜与冷路径逐位同形。
#[test]
fn kernel_telescope_survives_the_artifact_warm_path() {
    on_compile_stack(|| {
        with_course_lib(|cold, tables| {
            let facts = crate::project::artifacts::LibPassFacts {
                out: Default::default(),
                reports: Vec::new(),
                ranges: Vec::new(),
                n_commands: 0,
                prefix_commands: 0,
                lib_prefix: String::new(),
            };
            let text = crate::project::artifacts::encode_payload(&cold.snapshot(), tables, &facts)
                .expect("课程库的产物必须写得出来（否则暖路径根本不存在）");
            let arena = stumpalo::Arena::new();
            let (env, warm_tables, _) =
                crate::project::artifacts::decode_payload(arena.as_arena_ref(), &text)
                    .expect("产物必须装得回来");
            let loaded = EnvBuilder::from_export_file(arena.as_arena_ref(), env);
            // 与 `run_entries` 同款：入口趟拿检查点 builder 的克隆并封层。
            let mut warm = loaded.clone();
            warm.seal_library_layer();
            assert_eq!(warm_tables.known, tables.known, "known 表必须原样往返");
            let mut checked = 0usize;
            let mut diffs = Vec::new();
            for (name, entry) in &warm_tables.known {
                if !matches!(entry, KnownName::Decl { .. }) {
                    continue;
                }
                checked += 1;
                let warm_tel = kernel_telescope(&mut warm, entry, name);
                let cold_tel = kernel_telescope(cold, entry, name);
                match (cold_tel, warm_tel) {
                    (Some(c), Some(w)) => {
                        let same = c.universes == w.universes
                            && names(&c) == names(&w)
                            && kinds(&c) == kinds(&w)
                            && std::mem::discriminant(&*c.result)
                                == std::mem::discriminant(&*w.result);
                        if !same {
                            diffs.push(format!("{name}: 冷暖望远镜不同形"));
                        }
                    }
                    (c, w) => {
                        diffs.push(format!("{name}: cold={} warm={}", c.is_some(), w.is_some()))
                    }
                }
            }
            println!("KERNEL_TEL_WARM decls={checked} diffs={}", diffs.len());
            assert!(checked > 0);
            assert_eq!(
                diffs,
                Vec::<String>::new(),
                "暖路径上每条 Decl 都必须查得到且同形"
            );
        })
    })
}
