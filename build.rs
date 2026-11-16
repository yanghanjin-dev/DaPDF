use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "build/icon_resource.rs"]
mod icon_resource;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_dir = manifest_dir.join("vendor").join("lib");
    let bin_dir = manifest_dir.join("vendor").join("bin");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let object = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("dapdf-icon.obj");
        icon_resource::compile(
            &manifest_dir.join("assets/branding/dapdf.ico"),
            &object,
            &env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
        );
        println!("cargo:rustc-link-arg={}", object.display());
    }
    println!("cargo:rerun-if-changed=assets/branding/dapdf.ico");
    println!("cargo:rerun-if-changed=build/icon_resource.rs");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=pdfium.dll");

    if let Ok(out_dir) = env::var("OUT_DIR") {
        let out_path = PathBuf::from(out_dir);
        if let Some(target_dir) = out_path.ancestors().nth(3) {
            let dll_src = bin_dir.join("pdfium.dll");
            let dll_dst = target_dir.join("pdfium.dll");
            if dll_src.exists() {
                let _ = fs::copy(&dll_src, &dll_dst);
            }
        }
    }

    println!("cargo:rerun-if-changed=vendor/lib/pdfium.dll.lib");
    println!("cargo:rerun-if-changed=vendor/bin/pdfium.dll");
}
