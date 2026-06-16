// build.rs — pass our linker script to rust-lld and pin re-run conditions.
use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ld_script = PathBuf::from(&manifest_dir).join("linker.ld");

    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");

    println!("cargo:rustc-link-arg-bins=-T{}", ld_script.display());
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-zmax-page-size=0x1000");
}
