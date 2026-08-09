// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repository = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate must remain under crates/");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let zig_source = repository.join("src/interface/ffi/src/accelerator.zig");
    let zig_abi = repository.join("src/interface/generated/abi/enaction_accelerator.zig");
    let library = output.join("libenaction_accelerator.a");

    println!("cargo:rerun-if-changed={}", zig_source.display());
    println!("cargo:rerun-if-changed={}", zig_abi.display());
    println!(
        "cargo:rerun-if-changed={}",
        repository
            .join("src/interface/generated/abi/enaction_accelerator.rs")
            .display()
    );

    let status = Command::new("zig")
        .current_dir(repository)
        .env("ZIG_GLOBAL_CACHE_DIR", output.join("zig-global-cache"))
        .env("ZIG_LOCAL_CACHE_DIR", output.join("zig-local-cache"))
        .args([
            "build-lib",
            "-OReleaseSafe",
            "-static",
            "-fPIC",
            "-fcompiler-rt",
            "--dep",
            "abi",
            &format!("-Mroot={}", zig_source.display()),
            &format!("-Mabi={}", zig_abi.display()),
            &format!("-femit-bin={}", library.display()),
        ])
        .status()
        .expect("failed to launch the pinned Zig toolchain");
    assert!(
        status.success(),
        "pure-Zig accelerator library failed to build"
    );

    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=enaction_accelerator");
}
