//! Pin the compile-time target triple so the CLI can name its own
//! version-pinned release asset (`<pkg>-<TARGET>.tar.gz`) without guessing
//! the platform at runtime.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").expect("Cargo always sets TARGET for build scripts");
    println!("cargo:rustc-env=SOKONANODA_TARGET={target}");
}
