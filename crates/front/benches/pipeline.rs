//! Criterion benchmarks for the sokonanoda front-end pipeline
//! (`docs/design-kernel-taxonomy.md` §0.5/§3, item M).
//!
//! Motivation: kernel performance is a product property (REQUIREMENTS §3);
//! these benchmarks pin the timing of three paths so that later kernel or
//! incremental (I8) changes cannot silently regress them. They run locally via
//! `cargo bench -p sokonanoda-front`; CI does not run them.
//!
//! Black-box: only the public front-end API is exercised (`parse`,
//! `compile::check_document_with`, `session::Session`). The corpora are
//! inlined teaching documents (never read from disk) and are validated once
//! before the first benchmark runs: if any of them fails to check, the bench
//! run panics instead of reporting misleading timings.

use std::sync::OnceLock;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use sokonanoda_front::compile::{check_document_with, CompileOptions};
use sokonanoda_front::parse;
use sokonanoda_front::session::Session;

/// A synthetic teaching document: ~30 kernel-checked declarations mixing
/// `def`/`theorem`/`axiom`, Prop-level proof terms, native Nat arithmetic and
/// a 39-digit big-number `#reduce` tail (the same corpus shape as
/// `perf_smoke_native_and_iota_reduce`).
const SYNTHETIC_DOC: &str = r#"-- Sokonanoda bench corpus: a synthetic teaching document.

def idp : Prop -> Prop := fun (x : Prop) => x

def konst {u, v} : {α : Sort u} -> {β : Sort v} -> (a : α) -> (b : β) -> α
  := fun {α : Sort u} => fun {β : Sort v} => fun (a : α) => fun (b : β) => a

def comp {u, v, w} : {α : Sort u} -> {β : Sort v} -> {δ : Sort w} -> (f : β -> δ) -> (g : α -> β) -> (x : α) -> δ
  := fun {α : Sort u} => fun {β : Sort v} => fun {δ : Sort w} => fun (f : β -> δ) => fun (g : α -> β) => fun (x : α) => f (g x)

def flip {u, v, w} : {α : Sort u} -> {β : Sort v} -> {φ : Sort w} -> (f : α -> β -> φ) -> (b : β) -> (a : α) -> φ
  := fun {α : Sort u} => fun {β : Sort v} => fun {φ : Sort w} => fun (f : α -> β -> φ) => fun (b : β) => fun (a : α) => f a b

def two : Nat := 1 + 1
def four : Nat := two + two
def six : Nat := four + two
def ten : Nat := six + four
def add1 : Nat -> Nat := fun n => n + 1

#check add1
#reduce ten + ten
#reduce Nat.add two six

theorem refl_two : Eq.{1} Nat 2 2 := Eq.refl.{1} Nat 2

theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a
  := fun (a : Nat) => fun (b : Nat) => fun (h : Eq.{1} Nat a b) =>
       Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)

axiom True : Prop
axiom True.intro : True

theorem truly : True := True.intro

axiom False : Prop

axiom And : Prop -> Prop -> Prop
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b
axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a
axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b

theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a
  := fun (a : Prop) => fun (b : Prop) => fun (h : And a b) =>
       And.intro b a (And.right a b h) (And.left a b h)

theorem and_self : (a : Prop) -> (h : a) -> And a a
  := fun (a : Prop) => fun (h : a) => And.intro a a h h

axiom Or : Prop -> Prop -> Prop
axiom Or.inl : (a : Prop) -> (b : Prop) -> a -> Or a b
axiom Or.inr : (a : Prop) -> (b : Prop) -> b -> Or a b

theorem or_left : (a : Prop) -> (b : Prop) -> a -> Or a b
  := fun (a : Prop) => fun (b : Prop) => fun (h : a) => Or.inl a b h

axiom Iff : Prop -> Prop -> Prop
axiom Iff.intro : (a : Prop) -> (b : Prop) -> (a -> b) -> (b -> a) -> Iff a b

theorem iff_refl : (a : Prop) -> Iff a a
  := fun (a : Prop) => Iff.intro a a (fun (x : a) => x) (fun (x : a) => x)

theorem k_dep : ∀ (P : Prop), P -> P := fun (P : Prop) (hp : P) => hp

#reduce 1234567890123456789012345678901234567890 + 1
"#;

/// An explicit `inductive Nat` block with iota rules and a deep `add two two`
/// reduction, inlined from `examples/py-nat.sokonanoda` (never read from disk
/// so the bench stays self-contained).
const IOTA_DOC: &str = r#"-- Ported from py_nanobruijn nat.fol, with explicit iota rules.

inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} :
  (motive : (n : Nat) -> Sort u) ->
  (mz : motive zero) ->
  (ms : (n : Nat) -> motive n -> motive (succ n)) ->
  (n : Nat) -> motive n
iota zero :=
  fun (motive : (n : Nat) -> Sort u) =>
  fun (mz : motive zero) =>
  fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz
iota succ :=
  fun (motive : (n : Nat) -> Sort u) =>
  fun (mz : motive zero) =>
  fun (ms : (n : Nat) -> motive n -> motive (succ n)) =>
  fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)
end

def one : Nat := succ zero
def two : Nat := succ one
def three : Nat := succ two
def four : Nat := succ three

def add : Nat -> Nat -> Nat :=
  fun (m : Nat) => fun (n : Nat) =>
    Nat.rec.{1} (fun (x : Nat) => Nat) n
      (fun (k : Nat) => fun (ih : Nat) => succ ih) m

#reduce add two two
"#;

/// A five-declaration document for the incremental suffix-recheck benchmark;
/// editing the 4th declaration must kernel-recheck exactly the 4th and 5th
/// commands (`kernel_checks == 2`, anchored by `session_rechecks_only_the_
/// affected_suffix`).
const FIVE_DOC: &str = "\
def one : Nat := 1
def two : Nat := 2
def three : Nat := 3
def four : Nat := 4
def five : Nat := 5
";

/// Panics on first call if any bench corpus does not parse or check cleanly
/// (or if the incremental path drifts from its tested semantics), so a broken
/// corpus fails loudly instead of benchmarking nonsense.
fn validate_corpus() {
    static VALIDATED: OnceLock<()> = OnceLock::new();
    if VALIDATED.set(()).is_err() {
        return;
    }
    for (name, src) in [("synthetic", SYNTHETIC_DOC), ("iota", IOTA_DOC)] {
        let file = parse(src)
            .unwrap_or_else(|diag| panic!("bench corpus `{name}` does not parse: {diag:?}"));
        let report = check_document_with(&file, &CompileOptions::default());
        assert!(
            report.errors.is_empty(),
            "bench corpus `{name}` must compile without errors: {:?}",
            report.errors
        );
    }
    let mut session = Session::new(CompileOptions::default());
    let first = session.update(FIVE_DOC, 1);
    assert!(
        first.report.errors.is_empty(),
        "five-decl corpus must compile without errors: {:?}",
        first.report.errors
    );
    let edited = FIVE_DOC.replace("def four : Nat := 4", "def four : Nat := 40");
    assert_ne!(edited, FIVE_DOC);
    let second = session.update(&edited, 2);
    assert_eq!(
        second.recompiled_from,
        Some(3),
        "the edit must land on the 4th command"
    );
    assert_eq!(
        second.stats.kernel_checks, 2,
        "the suffix recheck must stay the kernel-recheck path"
    );
}

fn bench_native_bigint_reduce(c: &mut Criterion) {
    validate_corpus();
    c.bench_function("native_bigint_reduce", |b| {
        b.iter_batched(
            || parse(SYNTHETIC_DOC).expect("synthetic doc parses"),
            |file| check_document_with(&file, &CompileOptions::default()),
            BatchSize::SmallInput,
        )
    });
}

fn bench_iota_deep_reduce(c: &mut Criterion) {
    validate_corpus();
    c.bench_function("iota_deep_reduce", |b| {
        b.iter_batched(
            || parse(IOTA_DOC).expect("iota doc parses"),
            |file| check_document_with(&file, &CompileOptions::default()),
            BatchSize::SmallInput,
        )
    });
}

fn bench_session_suffix_recheck(c: &mut Criterion) {
    validate_corpus();
    c.bench_function("session_suffix_recheck", |b| {
        b.iter_batched_ref(
            || {
                let mut session = Session::new(CompileOptions::default());
                let first = session.update(FIVE_DOC, 1);
                assert!(
                    first.report.errors.is_empty(),
                    "five-decl corpus must compile without errors: {:?}",
                    first.report.errors
                );
                session
            },
            |session| {
                session.update(
                    &FIVE_DOC.replace("def four : Nat := 4", "def four : Nat := 40"),
                    2,
                )
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_native_bigint_reduce,
    bench_iota_deep_reduce,
    bench_session_suffix_recheck
);
criterion_main!(benches);
